//! Admin module for the Disputes contract.
//!
//! Provides administrative action tracking with a configurable cooldown
//! period to prevent rapid abuse of critical dispute operations. The
//! cooldown window applies between successive admin actions on disputes.
//!
//! # Cooldown
//!
//! The default cooldown is 1 hour (3600 seconds). After an admin performs
//! a critical dispute action, subsequent actions are blocked until the
//! cooldown window expires.

use soroban_sdk::{contracttype, panic_with_error, Address, Env};

/// Number of seconds that must elapse between critical admin actions.
pub const ADMIN_ACTION_COOLDOWN_SECS: u64 = 3600;

/// Storage key for the last critical admin action timestamp.
#[contracttype]
enum AdminDataKey {
    /// Admin address for the disputes contract.
    Admin,
    /// Timestamp of the last critical admin action (ledger time).
    LastCriticalAdminAction,
    /// Cooldown period in seconds (default: 3600).
    CooldownPeriod,
}

/// Error codes for the disputes admin module.
#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum AdminCooldownError {
    /// Caller is not the registered admin.
    Unauthorized = 100,
    /// The cooldown period has not yet elapsed since the last admin action.
    AdminCooldownActive = 543,
    /// No admin has been set; the contract is uninitialized.
    AdminNotSet = 419,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Record a critical admin action at the current ledger timestamp.
///
/// Must be called **after** a successful admin operation to update the
/// cooldown clock.
pub fn record_admin_action(env: &Env) {
    let now = env.ledger().timestamp();
    env.storage()
        .persistent()
        .set(&AdminDataKey::LastCriticalAdminAction, &now);
}

/// Check whether the cooldown period has elapsed since the last record.
///
/// Returns `Ok(())` if enough time has passed (or no action was ever
/// recorded).  Returns `Err(AdminCooldownActive)` if the cooldown is
/// still active.
pub fn validate_admin_cooldown(env: &Env) -> Result<(), AdminCooldownError> {
    let key = AdminDataKey::LastCriticalAdminAction;
    let cooldown: u64 = env
        .storage()
        .persistent()
        .get(&AdminDataKey::CooldownPeriod)
        .unwrap_or(ADMIN_ACTION_COOLDOWN_SECS);

    let last_action: Option<u64> = env.storage().persistent().get(&key);
    if let Some(last_ts) = last_action {
        let now = env.ledger().timestamp();
        let elapsed = now.saturating_sub(last_ts);
        if elapsed < cooldown {
            return Err(AdminCooldownError::AdminCooldownActive);
        }
    }
    Ok(())
}

/// Set or update the admin address for the disputes contract.
pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().persistent().set(&AdminDataKey::Admin, admin);
}

/// Return the stored admin address, or `None` if not yet set.
pub fn get_admin(env: &Env) -> Option<Address> {
    env.storage().persistent().get(&AdminDataKey::Admin)
}

/// Require that `caller` is the registered admin.
///
/// Panics with `AdminCooldownError::Unauthorized` if the caller does not
/// match the stored admin address.
pub fn require_admin(env: &Env, caller: &Address) {
    let stored: Address = env
        .storage()
        .persistent()
        .get(&AdminDataKey::Admin)
        .unwrap_or_else(|| {
            panic_with_error!(env, AdminCooldownError::AdminNotSet);
        });
    if caller != &stored {
        panic_with_error!(env, AdminCooldownError::Unauthorized);
    }
}

/// Require that the caller is admin **and** the cooldown has elapsed.
///
/// This is a convenience wrapper for operations that must be both
/// admin-gated and cooldown-gated.
pub fn require_admin_with_cooldown(env: &Env, caller: &Address) -> Result<(), AdminCooldownError> {
    require_admin(env, caller);
    validate_admin_cooldown(env)
}

/// Set a custom cooldown period (in seconds).  Only callable by admin.
pub fn set_cooldown_period(env: &Env, caller: &Address, cooldown_secs: u64) {
    require_admin(env, caller);
    env.storage()
        .persistent()
        .set(&AdminDataKey::CooldownPeriod, &cooldown_secs);
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::Env;

    /// Helper: set up env with mock auth, register admin.
    fn setup() -> (Env, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        set_admin(&env, &admin);
        (env, admin)
    }

    #[test]
    fn test_require_admin_passes_for_valid_admin() {
        let (env, admin) = setup();
        // Must not panic.
        require_admin(&env, &admin);
    }

    #[test]
    #[should_panic(expected = "HostError")]
    fn test_require_admin_panics_for_impostor() {
        let (env, _admin) = setup();
        let impostor = Address::generate(&env);
        require_admin(&env, &impostor);
    }

    #[test]
    #[should_panic(expected = "HostError")]
    fn test_require_admin_panics_when_admin_not_set() {
        let env = Env::default();
        env.mock_all_auths();
        let caller = Address::generate(&env);
        require_admin(&env, &caller);
    }

    #[test]
    fn test_cooldown_allows_first_action() {
        let (env, _admin) = setup();
        assert!(validate_admin_cooldown(&env).is_ok());
    }

    #[test]
    fn test_cooldown_rejects_immediate_repeat() {
        let (env, admin) = setup();
        require_admin(&env, &admin);
        record_admin_action(&env);

        let result = validate_admin_cooldown(&env);
        assert_eq!(result, Err(AdminCooldownError::AdminCooldownActive));
    }

    #[test]
    fn test_cooldown_allows_action_after_window_elapses() {
        let (env, admin) = setup();
        require_admin(&env, &admin);
        record_admin_action(&env);

        // Advance ledger past the 1-hour cooldown.
        let now = env.ledger().timestamp();
        env.ledger()
            .set_timestamp(now + ADMIN_ACTION_COOLDOWN_SECS + 1);

        assert!(validate_admin_cooldown(&env).is_ok());
    }

    #[test]
    fn test_record_admin_action_updates_timestamp() {
        let (env, admin) = setup();
        require_admin(&env, &admin);
        record_admin_action(&env);

        let now = env.ledger().timestamp();
        let key = AdminDataKey::LastCriticalAdminAction;
        let stored: u64 = env.storage().persistent().get(&key).unwrap();
        assert_eq!(stored, now);
    }

    #[test]
    fn test_set_cooldown_period_changes_window() {
        let (env, admin) = setup();
        let short_cooldown: u64 = 10;

        set_cooldown_period(&env, &admin, short_cooldown);
        require_admin(&env, &admin);
        record_admin_action(&env);

        // Still within 10-second cooldown.
        assert_eq!(
            validate_admin_cooldown(&env),
            Err(AdminCooldownError::AdminCooldownActive)
        );

        // Advance past the short cooldown.
        let now = env.ledger().timestamp();
        env.ledger().set_timestamp(now + short_cooldown + 1);
        assert!(validate_admin_cooldown(&env).is_ok());
    }

    #[test]
    fn test_full_lifecycle_sequence() {
        let (env, admin) = setup();

        // 1. First action succeeds.
        require_admin(&env, &admin);
        assert!(validate_admin_cooldown(&env).is_ok());
        record_admin_action(&env);

        // 2. Immediate repeat blocked.
        assert_eq!(
            validate_admin_cooldown(&env),
            Err(AdminCooldownError::AdminCooldownActive)
        );

        // 3. After cooldown elapses, second action succeeds.
        let now = env.ledger().timestamp();
        env.ledger()
            .set_timestamp(now + ADMIN_ACTION_COOLDOWN_SECS + 1);
        assert!(validate_admin_cooldown(&env).is_ok());
        record_admin_action(&env);

        // 4. Immediately after second action, blocked again.
        assert_eq!(
            validate_admin_cooldown(&env),
            Err(AdminCooldownError::AdminCooldownActive)
        );
    }
}
