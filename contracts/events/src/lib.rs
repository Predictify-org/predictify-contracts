#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, Address, Env,
    Symbol, Val, Vec,
};

// ===========================================================================
// Contract error
// ===========================================================================

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum EventsError {
    /// The caller is not authorized to perform this action.
    Unauthorized = 1,
    /// The callee contract call failed.
    CalleeFailed = 2,
    /// The provided value overflowed its target type.
    Overflow = 3,
    /// The requested resource was not found.
    NotFound = 4,
}

// ===========================================================================
// Contract
// ===========================================================================

/// A lightweight Soroban contract for emitting structured events and
/// invoking cross-contract calls with rollback-safe semantics.
///
/// This contract is designed for the GrantFox FWC26 campaign and
/// serves as the reference implementation for testing cross-contract
/// failure modes (reverts, panics) on event-emitting entrypoints.
///
/// # Auth
///
/// Every state-changing entrypoint requires `require_auth` on the
/// caller address. Read-only query functions are public.
///
/// # Math
///
/// All arithmetic uses overflow-safe methods
/// (`saturating_add`, `saturating_sub`, `checked_mul`, etc.) and
/// the contract never uses `unwrap()` or `expect()` in production
/// paths.
#[contract]
pub struct EventsContract;

// ===========================================================================
// Storage keys
// ===========================================================================

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// Per-topic replay nonce for event deduplication.
    EventNonce(Symbol),
    /// Flag marking whether the contract has been initialized.
    Initialized,
    /// The contract admin address.
    Admin,
    /// A simple counter tracking the total number of successful
    /// cross-contract calls made by this contract.
    CallCount,
}

// ===========================================================================
// Contract implementation
// ===========================================================================

#[contractimpl]
impl EventsContract {
    /// Initialize the contract with an admin address.
    ///
    /// # Panics
    ///
    /// This function traps if it is called more than once (the contract
    /// can only be initialized once).
    pub fn initialize(env: Env, admin: Address) {
        admin.require_auth();
        if env
            .storage()
            .persistent()
            .has(&DataKey::Initialized)
        {
            panic_with_error!(&env, EventsError::Unauthorized);
        }
        env.storage().persistent().set(&DataKey::Initialized, &true);
        env.storage().persistent().set(&DataKey::Admin, &admin);
    }

    /// Emit a structured event and then invoke a target contract's
    /// function in the same invocation frame.
    ///
    /// If the callee reverts or panics, this entire frame rolls back,
    /// including the emission. See the [xcontract test suite] for the
    /// full catalogue of failure-mode guarantees.
    ///
    /// [xcontract test suite]: xcontract
    ///
    /// # Auth
    ///
    /// Requires `admin` to authenticate.
    ///
    /// # Errors
    ///
    /// Returns [`EventsError::Unauthorized`] if the caller is not the admin.
    /// Returns [`EventsError::CalleeFailed`] if the callee call fails and the
    /// error is propagated.
    pub fn emit_and_call(
        env: Env,
        caller: Address,
        callee: Address,
        func: Symbol,
        topic: Symbol,
        market_id: Symbol,
    ) -> Result<(), EventsError> {
        caller.require_auth();
        let admin: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Admin)
            .ok_or(EventsError::NotFound)?;
        if caller != admin {
            return Err(EventsError::Unauthorized);
        }

        Self::emit_event(&env, &topic, &market_id, &caller);
        let result: Result<(), soroban_sdk::Error> = env.invoke_contract(&callee, &func, Vec::<Val>::new(&env));
        match result {
            Ok(()) => {
                env.storage()
                    .persistent()
                    .set(&DataKey::CallCount, &Self::get_call_count(&env).saturating_add(1));
                Ok(())
            }
            Err(_) => Err(EventsError::CalleeFailed),
        }
    }

    /// Invoke a target contract first and only emit the event if the
    /// callee succeeds.
    ///
    /// This ordering mirrors the pattern used by `GovernanceManager::execute_proposal`
    /// where the cross-contract call is made before the emission event.
    ///
    /// # Auth
    ///
    /// Requires `admin` to authenticate.
    pub fn call_and_emit(
        env: Env,
        caller: Address,
        callee: Address,
        func: Symbol,
        topic: Symbol,
        market_id: Symbol,
    ) -> Result<(), EventsError> {
        caller.require_auth();
        let admin: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Admin)
            .ok_or(EventsError::NotFound)?;
        if caller != admin {
            return Err(EventsError::Unauthorized);
        }

        let result: Result<(), soroban_sdk::Error> = env.invoke_contract(&callee, &func, Vec::<Val>::new(&env));
        result.map_err(|_| EventsError::CalleeFailed)?;

        Self::emit_event(&env, &topic, &market_id, &caller);
        env.storage()
            .persistent()
            .set(&DataKey::CallCount, &Self::get_call_count(&env).saturating_add(1));
        Ok(())
    }

    /// Emit a structured event and then attempt a cross-contract call via
    /// `try_invoke_contract`, allowing the caller to recover from callee
    /// failures.
    ///
    /// When the callee fails, the caller's own emission (published before the
    /// call) survives because its invocation frame succeeds; only the callee's
    /// frame is discarded.
    ///
    /// # Auth
    ///
    /// Requires `admin` to authenticate.
    pub fn emit_then_try_call(
        env: Env,
        caller: Address,
        callee: Address,
        func: Symbol,
        topic: Symbol,
        market_id: Symbol,
    ) -> Result<bool, EventsError> {
        caller.require_auth();
        let admin: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Admin)
            .ok_or(EventsError::NotFound)?;
        if caller != admin {
            return Err(EventsError::Unauthorized);
        }

        Self::emit_event(&env, &topic, &market_id, &caller);
        let failed = env
            .try_invoke_contract::<(), EventsError>(&callee, &func, Vec::<Val>::new(&env))
            .is_err();
        if failed {
            env.storage()
                .persistent()
                .set(&DataKey::CallCount, &Self::get_call_count(&env).saturating_add(1));
        }
        Ok(failed)
    }

    /// Query the number of successful cross-contract calls made by this
    /// contract.
    pub fn get_call_count(env: &Env) -> u64 {
        env.storage()
            .persistent()
            .get(&DataKey::CallCount)
            .unwrap_or(0)
    }

    /// Query the replay nonce for a given topic.
    ///
    /// Returns `None` when no emission on that topic has ever been committed.
    pub fn get_nonce(env: &Env, topic: Symbol) -> Option<u64> {
        env.storage().persistent().get(&DataKey::EventNonce(topic))
    }
}

// ===========================================================================
// Internal helpers
// ===========================================================================

impl EventsContract {
    /// Emit an event under `topic` and advance the per-topic nonce.
    fn emit_event(env: &Env, topic: &Symbol, market_id: &Symbol, admin: &Address) {
        let nonce = env
            .storage()
            .persistent()
            .get::<DataKey, u64>(&DataKey::EventNonce(topic.clone()))
            .unwrap_or(0);
        let next_nonce = nonce.saturating_add(1);
        env.storage()
            .persistent()
            .set(&DataKey::EventNonce(topic.clone()), &next_nonce);
        env.events().publish(
            (topic, market_id),
            EventPayload {
                admin: admin.clone(),
                nonce,
                timestamp: env.ledger().timestamp(),
            },
        );
    }
}

// ===========================================================================
// Event payload
// ===========================================================================

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct EventPayload {
    pub admin: Address,
    pub nonce: u64,
    pub timestamp: u64,
}