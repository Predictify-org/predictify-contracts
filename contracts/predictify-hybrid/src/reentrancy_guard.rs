//! # Reentrancy and Cross-Call Safety Guard
//!
//! Soroban-specific reentrancy protection for the Predictify Hybrid contract.
//! See [`docs/security/SECURITY_CONSIDERATIONS.md`] (section
//! "Reentrancy and Cross-Call State Consistency") for the full threat model
//! and integrator-facing notes that this module implements.
//!
//! ## Why a guard at all on Soroban?
//!
//! Soroban differs meaningfully from the EVM execution model:
//!
//! - There is no "fallback function" that runs on every value transfer. A
//!   Stellar Asset Contract (SAC) token's `transfer` cannot execute caller-
//!   supplied code on the recipient.
//! - Cross-contract calls are explicit (`env.invoke_contract(...)`) and the
//!   host accounts for storage modifications atomically per top-level call.
//!
//! The classic EVM pull-payment reentrancy exploit therefore does **not**
//! apply when the only outbound call is to a trusted SAC token. The guard
//! is still required because:
//!
//! 1. **Custom token contracts**: Predictify Hybrid allows non-SAC token
//!    contracts that satisfy the `token::Client` interface (see
//!    [`crate::tokens`]). Any such contract is third-party code and may
//!    re-enter Predictify during `transfer`, `mint`, or `burn`.
//! 2. **Oracle contracts**: oracle calls (see [`crate::oracles`]) cross a
//!    trust boundary into upgradeable third-party code.
//! 3. **Same-entrypoint reentrancy**: a malicious downstream contract may
//!    re-enter the *same* public entrypoint while an external call is
//!    in-flight, observing shared persistent state at an inconsistent
//!    intermediate value.
//! 4. **Panic safety**: a Soroban host function may panic. A panic ordered
//!    between a state mutation and its matching external call leaves the
//!    on-ledger state inconsistent unless the writes are sequenced after
//!    the call (Checks-Effects-Interactions).
//!
//! ## Design
//!
//! The guard stores a `Map<Symbol, bool>` in **persistent** storage, keyed
//! by entrypoint scope name:
//!
//! - Each public entrypoint (or helper scope) acquires its own lock slot,
//!   so a SAC transfer nested under `place_bet` does not false-positive
//!   when a sibling scope such as `lock_fn` performs the outbound call.
//! - Re-entry into the **same** scope while it is held is rejected with
//!   [`GuardError::ReentrancyGuardActive`].
//! - Persistent storage keeps locks observable across sub-call boundaries
//!   for forensic tooling.
//!
//! ## Recommended call pattern
//!
//! Prefer [`ReentrancyGuard::with_guard`] (or [`ReentrancyGuard::with_external_call`]
//! when a legacy single-scope helper suffices) over the manual
//! [`ReentrancyGuard::before_external_call`] /
//! [`ReentrancyGuard::after_external_call`] pair. The closure form
//! guarantees the lock is cleared on every return path, including error
//! returns, which would otherwise leave the contract wedged for all
//! subsequent callers.
//!
//! ```ignore
//! use crate::reentrancy_guard::ReentrancyGuard;
//! use soroban_sdk::symbol_short;
//!
//! let scope = symbol_short!("place_bet");
//! ReentrancyGuard::with_guard(env, &scope, || {
//!     // Effects: update internal state first.
//!     vault::debit(env, amount)?;
//!
//!     // Interactions: external call last (may use a distinct scope).
//!     token_client.transfer(&env.current_contract_address(), &user, &amount);
//!     Ok(())
//! })?;
//! ```
//!
//! ## Non-goals
//!
//! - **Cross-transaction protection**: locks are scoped to a single
//!   top-level invocation. Sequencing concerns across transactions are
//!   handled by higher-level state machines (`MarketState`, `ClaimInfo`).
//! - **Replacing Checks-Effects-Interactions**: the guard is an additional
//!   defensive layer, not a substitute for ordering state writes before
//!   external calls.

use soroban_sdk::{contracterror, symbol_short, Env, Map, Symbol};

/// Errors surfaced by the reentrancy guard.
///
/// These are deliberately narrow and module-local so callers can map them
/// onto the contract-wide [`crate::Error`] without coupling the guard to
/// the rest of the error taxonomy.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum GuardError {
    /// Returned when [`ReentrancyGuard::before_external_call`] or
    /// [`ReentrancyGuard::check_reentrancy_state`] is invoked while the
    /// lock is already held for that entrypoint scope — indicating a
    /// (possibly malicious) re-entry from a downstream contract during an
    /// in-flight external call.
    ReentrancyGuardActive = 1,
    /// Returned by [`ReentrancyGuard::validate_external_call_success`]
    /// when a caller-supplied success flag is `false`. Provided so that
    /// every external-call site can surface a consistent failure code.
    ExternalCallFailed = 2,
}

/// Legacy default scope used by [`ReentrancyGuard::with_external_call`].
pub fn default_external_call_scope() -> Symbol {
    symbol_short!("ext_call")
}

/// Per-entrypoint cross-call reentrancy guard.
///
/// `ReentrancyGuard` is a zero-sized type that exposes associated functions
/// over a map of entrypoint scopes stored in persistent ledger storage.
/// Holding a scope's lock indicates that an external (cross-contract) call
/// is currently in-flight for that scope and that the same scope must not
/// be re-entered until the call returns.
///
/// See the [module-level documentation](self) for the threat model,
/// recommended call pattern, and Soroban-vs-EVM rationale.
pub struct ReentrancyGuard;

impl ReentrancyGuard {
    /// Persistent storage key for the reentrancy lock map.
    fn key() -> Symbol {
        symbol_short!("reent_lk")
    }

    fn load_locks(env: &Env) -> Map<Symbol, bool> {
        match env
            .storage()
            .persistent()
            .get::<Symbol, Map<Symbol, bool>>(&Self::key())
        {
            Some(map) => map,
            None => Map::new(env),
        }
    }

    fn save_locks(env: &Env, map: &Map<Symbol, bool>) {
        env.storage().persistent().set(&Self::key(), map);
    }

    fn set_scope_locked(env: &Env, name: &Symbol, locked: bool) {
        let mut map = Self::load_locks(env);
        map.set(name.clone(), locked);
        Self::save_locks(env, &map);
    }

    /// Returns `true` if the given entrypoint scope lock is currently held.
    ///
    /// This is a pure read — it does not mutate storage. Use it for
    /// diagnostics or in tests; production call sites should prefer
    /// [`Self::check_reentrancy_state`] or [`Self::with_guard`].
    pub fn is_locked(env: &Env, name: &Symbol) -> bool {
        Self::load_locks(env).get(name.clone()).unwrap_or(false)
    }

    /// Asserts that no external call is currently in-flight for `name`.
    ///
    /// Returns [`GuardError::ReentrancyGuardActive`] if the scope lock is
    /// held, otherwise `Ok(())`. Intended for entrypoints that perform
    /// sensitive reads or state changes but do not themselves make outbound
    /// calls, so they cannot be safely interleaved with one in progress.
    pub fn check_reentrancy_state(env: &Env, name: &Symbol) -> Result<(), GuardError> {
        if Self::is_locked(env, name) {
            return Err(GuardError::ReentrancyGuardActive);
        }
        Ok(())
    }

    /// Acquires the reentrancy lock for `name` prior to an external call.
    ///
    /// Returns [`GuardError::ReentrancyGuardActive`] if the scope lock is
    /// already held — this is the signal that a downstream contract has
    /// re-entered the protocol during an in-flight call on the same scope.
    ///
    /// **Caller obligation**: every successful call to
    /// `before_external_call` must be paired with [`Self::after_external_call`]
    /// on every return path, including error paths and panics. Failing to
    /// release the lock leaves the contract wedged for all subsequent
    /// callers. Prefer [`Self::with_guard`] which enforces this
    /// invariant by construction.
    pub fn before_external_call(env: &Env, name: &Symbol) -> Result<(), GuardError> {
        if Self::is_locked(env, name) {
            return Err(GuardError::ReentrancyGuardActive);
        }
        Self::set_scope_locked(env, name, true);
        Ok(())
    }

    /// Releases the reentrancy lock for `name` after an external call completes.
    ///
    /// Idempotent: calling it on an already-released lock is a no-op write
    /// of `false`, not an error. This makes it safe to call from cleanup
    /// paths that may run more than once (e.g. nested error handlers).
    pub fn after_external_call(env: &Env, name: &Symbol) {
        Self::set_scope_locked(env, name, false);
    }

    /// Runs `f` with the entrypoint scope lock held, releasing it on every
    /// return path.
    ///
    /// This is the recommended way to protect a section that performs an
    /// external call or must not be re-entered on the same scope. It
    /// guarantees:
    ///
    /// - The lock for `name` is acquired before `f` runs (returns
    ///   [`GuardError::ReentrancyGuardActive`] if it cannot be acquired).
    /// - The lock is released after `f` returns, regardless of whether
    ///   `f` returned `Ok` or `Err`.
    /// - The original return value of `f` is propagated unchanged.
    ///
    /// Different entrypoint scopes may be locked concurrently; only
    /// re-entry into the **same** `name` is rejected.
    ///
    /// The closure's error type must be convertible from [`GuardError`]
    /// so that the acquire-failure path can be reported through the
    /// caller's own error taxonomy (typically [`crate::Error`]).
    ///
    /// # Panic safety
    ///
    /// If `f` panics, Soroban aborts the entire host invocation and rolls
    /// back the persistent-storage write that acquired the lock. The next
    /// top-level invocation therefore observes the lock as cleared. No
    /// special unwinding logic is required.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use crate::reentrancy_guard::ReentrancyGuard;
    /// use soroban_sdk::symbol_short;
    ///
    /// let scope = symbol_short!("place_bet");
    /// ReentrancyGuard::with_guard(env, &scope, || {
    ///     vault::debit(env, amount)?;
    ///     token_client.transfer(&env.current_contract_address(), &user, &amount);
    ///     Ok::<_, crate::Error>(())
    /// })?;
    /// ```
    pub fn with_guard<T, E, F>(env: &Env, name: &Symbol, f: F) -> Result<T, E>
    where
        F: FnOnce() -> Result<T, E>,
        E: From<GuardError>,
    {
        Self::before_external_call(env, name).map_err(E::from)?;
        let result = f();
        Self::after_external_call(env, name);
        result
    }

    /// Runs `f` with the legacy default external-call scope held.
    ///
    /// Prefer [`Self::with_guard`] with an explicit entrypoint symbol for
    /// new call sites so nested flows do not false-positive across scopes.
    pub fn with_external_call<T, E, F>(env: &Env, f: F) -> Result<T, E>
    where
        F: FnOnce() -> Result<T, E>,
        E: From<GuardError>,
    {
        let scope = default_external_call_scope();
        Self::with_guard(env, &scope, f)
    }

    /// Standardises validation of an external call's success flag.
    ///
    /// Returns [`GuardError::ExternalCallFailed`] when `ok` is `false`.
    /// Centralising this conversion ensures every call site reports
    /// the same error variant for downstream telemetry.
    pub fn validate_external_call_success(_env: &Env, ok: bool) -> Result<(), GuardError> {
        if ok {
            Ok(())
        } else {
            Err(GuardError::ExternalCallFailed)
        }
    }

    /// Executes caller-supplied restoration logic after an external call
    /// has failed.
    ///
    /// In most cases, ordering state writes *after* the external call (the
    /// Checks-Effects-Interactions pattern) is preferable. This helper is
    /// provided for the rare scenarios where provisional state must be
    /// written first — e.g. an idempotency marker that needs to be
    /// rolled back if the downstream call rejects the request.
    ///
    /// The helper is a thin wrapper rather than a no-op so call sites have
    /// a single recognisable pattern that auditors can grep for.
    pub fn restore_state_on_failure<F: FnOnce()>(_env: &Env, restore_fn: F) {
        restore_fn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PredictifyHybrid;
    use crate::PredictifyHybridClient;
    use soroban_sdk::{
        testutils::Address as _,
        token::StellarAssetClient,
        vec, Address, Env, String, Symbol,
    };

    fn with_contract<F: FnOnce()>(env: &Env, f: F) {
        let addr = env.register_contract(None, PredictifyHybrid);
        env.as_contract(&addr, || {
            f();
        });
    }

    fn test_scope() -> Symbol {
        symbol_short!("test_ep")
    }

    fn alt_scope() -> Symbol {
        symbol_short!("alt_ep")
    }

    #[test]
    fn lock_cycle_sets_and_clears_flag() {
        let env = Env::default();
        let scope = test_scope();
        with_contract(&env, || {
            assert!(!ReentrancyGuard::is_locked(&env, &scope));

            assert!(ReentrancyGuard::before_external_call(&env, &scope).is_ok());
            assert!(ReentrancyGuard::is_locked(&env, &scope));

            ReentrancyGuard::after_external_call(&env, &scope);
            assert!(!ReentrancyGuard::is_locked(&env, &scope));
        });
    }

    #[test]
    fn check_reentrancy_state_blocks_when_locked() {
        let env = Env::default();
        let scope = test_scope();
        with_contract(&env, || {
            assert!(ReentrancyGuard::check_reentrancy_state(&env, &scope).is_ok());

            assert!(ReentrancyGuard::before_external_call(&env, &scope).is_ok());
            let err = ReentrancyGuard::check_reentrancy_state(&env, &scope).unwrap_err();
            assert_eq!(err, GuardError::ReentrancyGuardActive);

            ReentrancyGuard::after_external_call(&env, &scope);
            assert!(ReentrancyGuard::check_reentrancy_state(&env, &scope).is_ok());
        });
    }

    /// Same-entrypoint reentry must be rejected.
    #[test]
    fn before_external_call_rejects_same_scope_reentry() {
        let env = Env::default();
        let scope = test_scope();
        with_contract(&env, || {
            assert!(ReentrancyGuard::before_external_call(&env, &scope).is_ok());
            let err = ReentrancyGuard::before_external_call(&env, &scope).unwrap_err();
            assert_eq!(err, GuardError::ReentrancyGuardActive);
            assert!(ReentrancyGuard::is_locked(&env, &scope));

            ReentrancyGuard::after_external_call(&env, &scope);
        });
    }

    /// Different entrypoint scopes may be locked concurrently.
    #[test]
    fn different_entrypoint_scopes_do_not_block_each_other() {
        let env = Env::default();
        let scope_a = test_scope();
        let scope_b = alt_scope();
        with_contract(&env, || {
            let outer: Result<(), GuardError> =
                ReentrancyGuard::with_guard(&env, &scope_a, || {
                    assert!(ReentrancyGuard::is_locked(&env, &scope_a));
                    assert!(!ReentrancyGuard::is_locked(&env, &scope_b));

                    let inner: Result<(), GuardError> =
                        ReentrancyGuard::with_guard(&env, &scope_b, || {
                            assert!(ReentrancyGuard::is_locked(&env, &scope_a));
                            assert!(ReentrancyGuard::is_locked(&env, &scope_b));
                            Ok(())
                        });
                    assert!(inner.is_ok());

                    assert!(ReentrancyGuard::is_locked(&env, &scope_a));
                    assert!(!ReentrancyGuard::is_locked(&env, &scope_b));
                    Ok(())
                });
            assert!(outer.is_ok());
            assert!(!ReentrancyGuard::is_locked(&env, &scope_a));
            assert!(!ReentrancyGuard::is_locked(&env, &scope_b));
        });
    }

    #[test]
    fn after_external_call_is_idempotent() {
        let env = Env::default();
        let scope = test_scope();
        with_contract(&env, || {
            assert!(ReentrancyGuard::before_external_call(&env, &scope).is_ok());
            ReentrancyGuard::after_external_call(&env, &scope);
            ReentrancyGuard::after_external_call(&env, &scope);
            assert!(!ReentrancyGuard::is_locked(&env, &scope));

            assert!(ReentrancyGuard::before_external_call(&env, &scope).is_ok());
            ReentrancyGuard::after_external_call(&env, &scope);
        });
    }

    #[test]
    fn after_external_call_without_acquire_is_safe() {
        let env = Env::default();
        let scope = test_scope();
        with_contract(&env, || {
            ReentrancyGuard::after_external_call(&env, &scope);
            assert!(!ReentrancyGuard::is_locked(&env, &scope));
        });
    }

    #[test]
    fn validate_external_call_success_branches() {
        let env = Env::default();
        with_contract(&env, || {
            assert!(ReentrancyGuard::validate_external_call_success(&env, true).is_ok());
            let err = ReentrancyGuard::validate_external_call_success(&env, false).unwrap_err();
            assert_eq!(err, GuardError::ExternalCallFailed);
        });
    }

    #[test]
    fn restore_state_on_failure_invokes_closure() {
        let env = Env::default();
        with_contract(&env, || {
            let mut ran = false;
            ReentrancyGuard::restore_state_on_failure(&env, || {
                ran = true;
            });
            assert!(ran);
        });
    }

    #[test]
    fn with_guard_locks_during_closure_and_releases() {
        let env = Env::default();
        let scope = test_scope();
        with_contract(&env, || {
            let mut observed_locked = false;
            let result: Result<i32, GuardError> =
                ReentrancyGuard::with_guard(&env, &scope, || {
                    observed_locked = ReentrancyGuard::is_locked(&env, &scope);
                    Ok(42)
                });
            assert_eq!(result, Ok(42));
            assert!(observed_locked, "closure should observe the lock as held");
            assert!(
                !ReentrancyGuard::is_locked(&env, &scope),
                "lock must be released after with_guard returns Ok"
            );
        });
    }

    #[test]
    fn with_guard_releases_on_error() {
        let env = Env::default();
        let scope = test_scope();
        with_contract(&env, || {
            let result: Result<(), GuardError> = ReentrancyGuard::with_guard(&env, &scope, || {
                Err(GuardError::ExternalCallFailed)
            });
            assert_eq!(result, Err(GuardError::ExternalCallFailed));
            assert!(
                !ReentrancyGuard::is_locked(&env, &scope),
                "lock must be released after with_guard returns Err"
            );
        });
    }

    /// Nested `with_guard` on the **same** scope must surface
    /// `ReentrancyGuardActive` and leave the outer lock intact.
    #[test]
    fn with_guard_rejects_nested_same_scope_invocation() {
        let env = Env::default();
        let scope = test_scope();
        with_contract(&env, || {
            let outer: Result<(), GuardError> = ReentrancyGuard::with_guard(&env, &scope, || {
                let inner: Result<(), GuardError> =
                    ReentrancyGuard::with_guard(&env, &scope, || Ok(()));
                assert_eq!(inner, Err(GuardError::ReentrancyGuardActive));
                assert!(ReentrancyGuard::is_locked(&env, &scope));
                Ok(())
            });
            assert!(outer.is_ok());
            assert!(!ReentrancyGuard::is_locked(&env, &scope));
        });
    }

    #[test]
    fn with_external_call_rejects_nested_same_default_scope() {
        let env = Env::default();
        let scope = default_external_call_scope();
        with_contract(&env, || {
            let outer: Result<(), GuardError> = ReentrancyGuard::with_external_call(&env, || {
                let inner: Result<(), GuardError> =
                    ReentrancyGuard::with_external_call(&env, || Ok(()));
                assert_eq!(inner, Err(GuardError::ReentrancyGuardActive));
                assert!(ReentrancyGuard::is_locked(&env, &scope));
                Ok(())
            });
            assert!(outer.is_ok());
            assert!(!ReentrancyGuard::is_locked(&env, &scope));
        });
    }

    #[test]
    fn lock_recoverable_after_rejected_reentry() {
        let env = Env::default();
        let scope = test_scope();
        with_contract(&env, || {
            assert!(ReentrancyGuard::before_external_call(&env, &scope).is_ok());
            assert!(ReentrancyGuard::before_external_call(&env, &scope).is_err());
            ReentrancyGuard::after_external_call(&env, &scope);

            assert!(ReentrancyGuard::before_external_call(&env, &scope).is_ok());
            assert!(ReentrancyGuard::is_locked(&env, &scope));
            ReentrancyGuard::after_external_call(&env, &scope);
            assert!(!ReentrancyGuard::is_locked(&env, &scope));
        });
    }

    #[test]
    fn guard_error_discriminants_are_stable() {
        assert_eq!(GuardError::ReentrancyGuardActive as u32, 1);
        assert_eq!(GuardError::ExternalCallFailed as u32, 2);
    }

    /// `place_bet` holds the `place_bet` scope while `lock_funds` uses a
    /// distinct scope for the SAC transfer — no false positive reentrancy.
    #[test]
    fn place_bet_sac_transfer_cross_scope_no_false_positive() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let contract_id = env.register(PredictifyHybrid, ());
        let client = PredictifyHybridClient::new(&env, &contract_id);
        client.initialize(&admin, &None, &None);

        let token_admin = Address::generate(&env);
        let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_id = token_contract.address();

        env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .set(&Symbol::new(&env, "TokenID"), &token_id);
        });

        let stellar_client = StellarAssetClient::new(&env, &token_id);
        stellar_client.mint(&user, &1000_0000000);

        let token_client = soroban_sdk::token::Client::new(&env, &token_id);
        token_client.approve(&user, &contract_id, &i128::MAX, &1_000_000);

        let outcomes = vec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ];
        let market_id = client.create_market(
            &admin,
            &String::from_str(&env, "Will BTC reach $100k?"),
            &outcomes,
            &30,
            &crate::types::OracleConfig {
                provider: crate::types::OracleProvider::reflector(),
                oracle_address: Address::generate(&env),
                feed_id: String::from_str(&env, "BTC/USD"),
                threshold: 10_000_000,
                comparison: String::from_str(&env, "gt"),
            },
            &None,
            &3600u64,
            &None,
            &None,
            &None,
        );

        let bet = client.place_bet(
            &user,
            &market_id,
            &String::from_str(&env, "yes"),
            &1_000_000,
            &250,
        );

        assert_eq!(bet.amount, 1_000_000);
        assert_eq!(bet.outcome, String::from_str(&env, "yes"));

        env.as_contract(&contract_id, || {
            assert!(!ReentrancyGuard::is_locked(
                &env,
                &symbol_short!("place_bet")
            ));
            assert!(!ReentrancyGuard::is_locked(&env, &symbol_short!("lock_fn")));
        });
    }
}
