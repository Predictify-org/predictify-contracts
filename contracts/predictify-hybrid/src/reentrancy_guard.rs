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
//! 3. **Cross-function reentrancy**: even if the inbound call hits a
//!    different public entrypoint, shared persistent state (e.g. fee vault,
//!    market totals) can be observed at an inconsistent intermediate value.
//! 4. **Panic safety**: a Soroban host function may panic. A panic ordered
//!    between a state mutation and its matching external call leaves the
//!    on-ledger state inconsistent unless the writes are sequenced after
//!    the call (Checks-Effects-Interactions).
//!
//! ## Design
//!
//! The guard stores a single boolean in **persistent** storage:
//!
//! - Chosen over instance/temporary storage so the lock survives the
//!   sub-call boundary and is observable to forensic tooling.
//! - The single key keeps the per-call overhead at one ledger read plus at
//!   most two writes; finer-grained per-market locks were considered and
//!   rejected as they would not prevent cross-function reentrancy.
//!
//! ## Recommended call pattern
//!
//! Prefer [`ReentrancyGuard::with_external_call`] over the manual
//! [`ReentrancyGuard::before_external_call`] /
//! [`ReentrancyGuard::after_external_call`] pair. The closure form
//! guarantees the lock is cleared on every return path, including error
//! returns, which would otherwise leave the contract wedged for all
//! subsequent callers.
//!
//! ```ignore
//! use crate::reentrancy_guard::ReentrancyGuard;
//!
//! ReentrancyGuard::with_external_call(env, || {
//!     // Effects: update internal state first.
//!     vault::debit(env, amount)?;
//!
//!     // Interactions: external call last.
//!     token_client.transfer(&env.current_contract_address(), &user, &amount);
//!     Ok(())
//! })?;
//! ```
//!
//! ## Non-goals
//!
//! - **Per-market or per-user locking**: the guard is a single global flag.
//!   Finer locks would not prevent cross-function reentrancy and would
//!   cost extra ledger writes per protected call.
//! - **Cross-transaction protection**: the lock is scoped to a single
//!   top-level invocation. Sequencing concerns across transactions are
//!   handled by higher-level state machines (`MarketState`, `ClaimInfo`).
//! - **Replacing Checks-Effects-Interactions**: the guard is an additional
//!   defensive layer, not a substitute for ordering state writes before
//!   external calls.

use soroban_sdk::{contracterror, symbol_short, Env};

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
    /// lock is already held — indicating a (possibly malicious) re-entry
    /// from a downstream contract during an in-flight external call.
    ReentrancyGuardActive = 1,
    /// Returned by [`ReentrancyGuard::validate_external_call_success`]
    /// when a caller-supplied success flag is `false`. Provided so that
    /// every external-call site can surface a consistent failure code.
    ExternalCallFailed = 2,
}

/// Global cross-function reentrancy guard.
///
/// `ReentrancyGuard` is a zero-sized type that exposes associated functions
/// over a single boolean stored in persistent ledger storage. Holding the
/// lock indicates that an external (cross-contract) call is currently
/// in-flight and that *no* public entrypoint of this contract may make
/// further state changes until the call returns.
///
/// See the [module-level documentation](self) for the threat model,
/// recommended call pattern, and Soroban-vs-EVM rationale.
pub struct ReentrancyGuard;

impl ReentrancyGuard {
    /// Persistent storage key for the reentrancy lock.
    ///
    /// The key is intentionally short (`reent_lk`) to minimise per-call
    /// storage cost. Persistent storage is used so the flag survives the
    /// sub-call boundary and remains visible to off-chain forensic tools
    /// if a transaction aborts mid-flow.
    fn key() -> soroban_sdk::Symbol {
        symbol_short!("reent_lk")
    }

    /// Returns `true` if the reentrancy lock is currently held.
    ///
    /// This is a pure read — it does not mutate storage. Use it for
    /// diagnostics or in tests; production call sites should prefer
    /// [`Self::check_reentrancy_state`] or [`Self::with_external_call`].
    pub fn is_locked(env: &Env) -> bool {
        env.storage()
            .persistent()
            .get::<soroban_sdk::Symbol, bool>(&Self::key())
            .unwrap_or(false)
    }

    /// Asserts that no external call is currently in-flight.
    ///
    /// Returns [`GuardError::ReentrancyGuardActive`] if the lock is held,
    /// otherwise `Ok(())`. Intended for entrypoints that perform sensitive
    /// reads or state changes but do not themselves make outbound calls,
    /// so they cannot be safely interleaved with one in progress.
    pub fn check_reentrancy_state(env: &Env) -> Result<(), GuardError> {
        if Self::is_locked(env) {
            return Err(GuardError::ReentrancyGuardActive);
        }
        Ok(())
    }

    /// Acquires the reentrancy lock prior to an external call.
    ///
    /// Returns [`GuardError::ReentrancyGuardActive`] if the lock is already
    /// held — this is the signal that a downstream contract has re-entered
    /// the protocol during an in-flight call.
    ///
    /// **Caller obligation**: every successful call to
    /// `before_external_call` must be paired with [`Self::after_external_call`]
    /// on every return path, including error paths and panics. Failing to
    /// release the lock leaves the contract wedged for all subsequent
    /// callers. Prefer [`Self::with_external_call`] which enforces this
    /// invariant by construction.
    pub fn before_external_call(env: &Env) -> Result<(), GuardError> {
        if Self::is_locked(env) {
            return Err(GuardError::ReentrancyGuardActive);
        }
        env.storage().persistent().set(&Self::key(), &true);
        Ok(())
    }

    /// Releases the reentrancy lock after an external call completes.
    ///
    /// Idempotent: calling it on an already-released lock is a no-op write
    /// of `false`, not an error. This makes it safe to call from cleanup
    /// paths that may run more than once (e.g. nested error handlers).
    pub fn after_external_call(env: &Env) {
        env.storage().persistent().set(&Self::key(), &false);
    }

    /// Runs `f` with the reentrancy lock held, releasing it on every
    /// return path.
    ///
    /// This is the recommended way to protect a section that performs an
    /// external call. It guarantees:
    ///
    /// - The lock is acquired before `f` runs (returns
    ///   [`GuardError::ReentrancyGuardActive`] if it cannot be acquired).
    /// - The lock is released after `f` returns, regardless of whether
    ///   `f` returned `Ok` or `Err`.
    /// - The original return value of `f` is propagated unchanged.
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
    ///
    /// ReentrancyGuard::with_external_call(env, || {
    ///     vault::debit(env, amount)?;
    ///     token_client.transfer(&env.current_contract_address(), &user, &amount);
    ///     Ok::<_, crate::Error>(())
    /// })?;
    /// ```
    pub fn with_external_call<T, E, F>(env: &Env, f: F) -> Result<T, E>
    where
        F: FnOnce() -> Result<T, E>,
        E: From<GuardError>,
    {
        Self::before_external_call(env).map_err(E::from)?;
        let result = f();
        Self::after_external_call(env);
        result
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
    use soroban_sdk::Env;

    fn with_contract<F: FnOnce()>(env: &Env, f: F) {
        let addr = env.register_contract(None, PredictifyHybrid);
        env.as_contract(&addr, || {
            f();
        });
    }

    #[test]
    fn lock_cycle_sets_and_clears_flag() {
        let env = Env::default();
        with_contract(&env, || {
            assert!(!ReentrancyGuard::is_locked(&env));

            assert!(ReentrancyGuard::before_external_call(&env).is_ok());
            assert!(ReentrancyGuard::is_locked(&env));

            ReentrancyGuard::after_external_call(&env);
            assert!(!ReentrancyGuard::is_locked(&env));
        });
    }

    #[test]
    fn check_reentrancy_state_blocks_when_locked() {
        let env = Env::default();
        with_contract(&env, || {
            assert!(ReentrancyGuard::check_reentrancy_state(&env).is_ok());

            assert!(ReentrancyGuard::before_external_call(&env).is_ok());
            let err = ReentrancyGuard::check_reentrancy_state(&env).unwrap_err();
            assert_eq!(err, GuardError::ReentrancyGuardActive);

            ReentrancyGuard::after_external_call(&env);
            assert!(ReentrancyGuard::check_reentrancy_state(&env).is_ok());
        });
    }

    /// `before_external_call` must reject a second acquisition while the
    /// lock is already held — this is the core reentrancy detection.
    #[test]
    fn before_external_call_rejects_reentry() {
        let env = Env::default();
        with_contract(&env, || {
            assert!(ReentrancyGuard::before_external_call(&env).is_ok());
            let err = ReentrancyGuard::before_external_call(&env).unwrap_err();
            assert_eq!(err, GuardError::ReentrancyGuardActive);
            // Lock must remain held after a failed reentry attempt.
            assert!(ReentrancyGuard::is_locked(&env));

            ReentrancyGuard::after_external_call(&env);
        });
    }

    /// `after_external_call` must be idempotent so cleanup paths that run
    /// more than once do not regress to an error.
    #[test]
    fn after_external_call_is_idempotent() {
        let env = Env::default();
        with_contract(&env, || {
            assert!(ReentrancyGuard::before_external_call(&env).is_ok());
            ReentrancyGuard::after_external_call(&env);
            ReentrancyGuard::after_external_call(&env);
            assert!(!ReentrancyGuard::is_locked(&env));

            // And we can re-acquire afterwards.
            assert!(ReentrancyGuard::before_external_call(&env).is_ok());
            ReentrancyGuard::after_external_call(&env);
        });
    }

    /// Calling `after_external_call` without ever acquiring should also be
    /// safe — it must not panic and must leave the lock cleared.
    #[test]
    fn after_external_call_without_acquire_is_safe() {
        let env = Env::default();
        with_contract(&env, || {
            ReentrancyGuard::after_external_call(&env);
            assert!(!ReentrancyGuard::is_locked(&env));
        });
    }

    #[test]
    fn validate_external_call_success_branches() {
        let env = Env::default();
        with_contract(&env, || {
            assert!(ReentrancyGuard::validate_external_call_success(&env, true).is_ok());
            let err =
                ReentrancyGuard::validate_external_call_success(&env, false).unwrap_err();
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

    /// `with_external_call` must hold the lock while the closure runs and
    /// release it on the success path.
    #[test]
    fn with_external_call_locks_during_closure_and_releases() {
        let env = Env::default();
        with_contract(&env, || {
            let mut observed_locked = false;
            let result: Result<i32, GuardError> =
                ReentrancyGuard::with_external_call(&env, || {
                    observed_locked = ReentrancyGuard::is_locked(&env);
                    Ok(42)
                });
            assert_eq!(result, Ok(42));
            assert!(observed_locked, "closure should observe the lock as held");
            assert!(
                !ReentrancyGuard::is_locked(&env),
                "lock must be released after with_external_call returns Ok"
            );
        });
    }

    /// `with_external_call` must release the lock even when the closure
    /// returns `Err`. This is the central correctness property — a
    /// failed external call must not wedge the contract.
    #[test]
    fn with_external_call_releases_on_error() {
        let env = Env::default();
        with_contract(&env, || {
            let result: Result<(), GuardError> =
                ReentrancyGuard::with_external_call(&env, || {
                    Err(GuardError::ExternalCallFailed)
                });
            assert_eq!(result, Err(GuardError::ExternalCallFailed));
            assert!(
                !ReentrancyGuard::is_locked(&env),
                "lock must be released after with_external_call returns Err"
            );
        });
    }

    /// Nested `with_external_call` invocations must surface
    /// `ReentrancyGuardActive` from the inner attempt and leave the
    /// outer lock intact for the duration of the outer closure.
    #[test]
    fn with_external_call_rejects_nested_invocation() {
        let env = Env::default();
        with_contract(&env, || {
            let outer: Result<(), GuardError> =
                ReentrancyGuard::with_external_call(&env, || {
                    let inner: Result<(), GuardError> =
                        ReentrancyGuard::with_external_call(&env, || Ok(()));
                    assert_eq!(inner, Err(GuardError::ReentrancyGuardActive));
                    // Outer lock must still be held while we're inside it.
                    assert!(ReentrancyGuard::is_locked(&env));
                    Ok(())
                });
            assert!(outer.is_ok());
            assert!(!ReentrancyGuard::is_locked(&env));
        });
    }

    /// After a failed acquire attempt, a normal acquire/release cycle
    /// must still work — the failure path must not corrupt the flag.
    #[test]
    fn lock_recoverable_after_rejected_reentry() {
        let env = Env::default();
        with_contract(&env, || {
            assert!(ReentrancyGuard::before_external_call(&env).is_ok());
            assert!(ReentrancyGuard::before_external_call(&env).is_err());
            ReentrancyGuard::after_external_call(&env);

            // Subsequent normal use still works.
            assert!(ReentrancyGuard::before_external_call(&env).is_ok());
            assert!(ReentrancyGuard::is_locked(&env));
            ReentrancyGuard::after_external_call(&env);
            assert!(!ReentrancyGuard::is_locked(&env));
        });
    }

    /// Sanity check that `GuardError` carries the documented discriminants
    /// — these values are part of the public ABI surfaced through
    /// `#[contracterror]`.
    #[test]
    fn guard_error_discriminants_are_stable() {
        assert_eq!(GuardError::ReentrancyGuardActive as u32, 1);
        assert_eq!(GuardError::ExternalCallFailed as u32, 2);
    }
}
