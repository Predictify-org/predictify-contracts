use soroban_sdk::{contracterror, symbol_short, Env};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum GuardError {
    ReentrancyGuardActive = 1,
    ExternalCallFailed = 2,
}

/// Global cross-function reentrancy guard.
///
/// This guard prevents reentry across all public entrypoints while an external
/// call (e.g., token transfer, oracle invocation) is in-flight. The lock is
/// stored in persistent storage using a single boolean flag.
pub struct ReentrancyGuard;

impl ReentrancyGuard {
    fn key() -> soroban_sdk::Symbol {
        // Persistent storage key for the reentrancy lock
        symbol_short!("reent_lk")
    }

    /// Returns true if the reentrancy lock is currently active.
    pub fn is_locked(env: &Env) -> bool {
        env.storage()
            .persistent()
            .get::<soroban_sdk::Symbol, bool>(&Self::key())
            .unwrap_or(false)
    }

    /// Checks current reentrancy state. Returns an error if locked.
    pub fn check_reentrancy_state(env: &Env) -> Result<(), GuardError> {
        if Self::is_locked(env) {
            return Err(GuardError::ReentrancyGuardActive);
        }
        Ok(())
    }

    /// Sets the reentrancy lock before making an external call.
    ///
    /// If the lock is already set, returns `Error::ReentrancyGuardActive`.
    pub fn before_external_call(env: &Env) -> Result<(), GuardError> {
        if Self::is_locked(env) {
            return Err(GuardError::ReentrancyGuardActive);
        }
        env.storage().persistent().set(&Self::key(), &true);
        Ok(())
    }

    /// Clears the reentrancy lock after the external call completes.
    pub fn after_external_call(env: &Env) {
        env.storage().persistent().set(&Self::key(), &false);
    }

    /// Validates that an external call succeeded.
    ///
    /// This helper standardizes call-site validation and returns a specific
    /// `ExternalCallFailed` error when `ok` is false.
    pub fn validate_external_call_success(_env: &Env, ok: bool) -> Result<(), GuardError> {
        if ok {
            Ok(())
        } else {
            Err(GuardError::ExternalCallFailed)
        }
    }
    ///
    /// This is a light abstraction to execute caller-provided restoration logic
    /// for any provisional state touched before an external call. In most cases,
    /// prefer ordering state writes after the external call succeeds, but this is
    /// provided for scenarios where temporary state must be set.
    pub fn restore_state_on_failure<F: FnOnce()>(_env: &Env, restore_fn: F) {
        restore_fn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PredictifyHybrid;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{Address, Env};

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
            // Initially unlocked
            assert!(!ReentrancyGuard::is_locked(&env));

            // Lock
            assert!(ReentrancyGuard::before_external_call(&env).is_ok());
            assert!(ReentrancyGuard::is_locked(&env));

            // Unlock
            ReentrancyGuard::after_external_call(&env);
            assert!(!ReentrancyGuard::is_locked(&env));
        });
    }

    #[test]
    fn check_reentrancy_state_blocks_when_locked() {
        let env = Env::default();
        with_contract(&env, || {
            // Unlocked state allows operations
            assert!(ReentrancyGuard::check_reentrancy_state(&env).is_ok());

            // Lock and verify it blocks
            assert!(ReentrancyGuard::before_external_call(&env).is_ok());
            let err = ReentrancyGuard::check_reentrancy_state(&env).unwrap_err();
            assert_eq!(err, GuardError::ReentrancyGuardActive);

            // Unlock and verify allowed again
            ReentrancyGuard::after_external_call(&env);
            assert!(ReentrancyGuard::check_reentrancy_state(&env).is_ok());
        });
    }
}
