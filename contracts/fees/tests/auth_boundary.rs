#![cfg(test)]

use fees::{ContractError, FeeConfig, FeesContract, FeesContractClient};
use soroban_sdk::{testutils::Address as _, Address, Env};

// ============================================================
// Test Helpers
// ============================================================

/// Registers the contract and initializes it with the given admin.
/// The auth snapshot from initialization is drained so it does not
/// interfere with subsequent assertions on `env.auths()`.
///
/// Note: `mock_all_auths()` is sticky on this `Env`. All
/// `require_auth()` calls will pass for the rest of the test.
/// Tests that need to verify `require_auth()` rejection (as opposed
/// to admin authorization) must use a fresh `Env`.
fn register_and_initialize<'a>(env: &Env, admin: &Address) -> FeesContractClient<'a> {
    let contract_id = env.register(FeesContract, ());
    let client = FeesContractClient::new(env, &contract_id);
    env.mock_all_auths();
    client.initialize(admin);
    // Drain auth snapshot from initialization
    let _ = env.auths();
    client
}

fn default_fee_config() -> FeeConfig {
    FeeConfig {
        platform_fee_percentage: 500, // 5.00%
        creation_fee: 100,
        min_fee_amount: 10,
        max_fee_amount: 1_000_000,
        collection_threshold: 50_000,
        fees_enabled: true,
    }
}

// ============================================================
// Auth Boundary Tests — initialize
// ============================================================

/// `initialize` is the only entrypoint that can test low-level
/// `require_auth()` failure directly. All other state-changing
/// entrypoints need the contract already initialized, which
/// requires `mock_all_auths()` to be active. Their tests instead
/// verify admin authorization via the `assert_is_admin` guard.
#[test]
fn test_initialize_requires_auth() {
    let env = Env::default();
    // Do NOT mock_all_auths — require_auth should fail as a host error
    let contract_id = env.register(FeesContract, ());
    let client = FeesContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    let result = client.try_initialize(&admin);
    assert!(result.is_err(), "initialize should require auth");
}

#[test]
fn test_initialize_succeeds_with_auth() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(FeesContract, ());
    let client = FeesContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    assert_eq!(auths[0].0, admin);
}

#[test]
fn test_initialize_cannot_reinitialize() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let client = register_and_initialize(&env, &admin);

    let result = client.try_initialize(&admin);
    match result {
        Err(Ok(ContractError::InvalidState)) => {} // Expected
        other => panic!(
            "Re-initialization should fail with InvalidState, got: {:?}",
            other
        ),
    }
}

// ============================================================
// Auth Boundary Tests — update_fee_config
// ============================================================

#[test]
fn test_update_fee_config_rejects_non_admin() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let client = register_and_initialize(&env, &admin);
    let non_admin = Address::generate(&env);

    let config = default_fee_config();
    let result = client.try_update_fee_config(&non_admin, &config);
    match result {
        Err(Ok(ContractError::Unauthorized)) => {} // Expected
        other => panic!("Non-admin should get Unauthorized, got: {:?}", other),
    }
}

#[test]
fn test_update_fee_config_succeeds_with_admin_auth() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let client = register_and_initialize(&env, &admin);

    let config = default_fee_config();
    client.update_fee_config(&admin, &config);

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    assert_eq!(auths[0].0, admin);
}

#[test]
fn test_update_fee_config_rejects_invalid_percentage() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let client = register_and_initialize(&env, &admin);

    let mut config = default_fee_config();
    config.platform_fee_percentage = 20_000; // > 10_000

    let result = client.try_update_fee_config(&admin, &config);
    match result {
        Err(Ok(ContractError::FeePercentageTooHigh)) => {} // Expected
        other => panic!("Should reject excessive fee percentage, got: {:?}", other),
    }
}

#[test]
fn test_update_fee_config_rejects_negative_values() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let client = register_and_initialize(&env, &admin);

    let mut config = default_fee_config();
    config.creation_fee = -1;

    let result = client.try_update_fee_config(&admin, &config);
    match result {
        Err(Ok(ContractError::InvalidInput)) => {} // Expected
        other => panic!("Should reject negative creation fee, got: {:?}", other),
    }
}

// ============================================================
// Auth Boundary Tests — set_platform_fee
// ============================================================

#[test]
fn test_set_platform_fee_rejects_non_admin() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let client = register_and_initialize(&env, &admin);
    let non_admin = Address::generate(&env);

    let result = client.try_set_platform_fee(&non_admin, &300);
    match result {
        Err(Ok(ContractError::Unauthorized)) => {} // Expected
        other => panic!("Non-admin should get Unauthorized, got: {:?}", other),
    }
}

#[test]
fn test_set_platform_fee_succeeds_with_admin_auth() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let client = register_and_initialize(&env, &admin);

    client.set_platform_fee(&admin, &300);

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    assert_eq!(auths[0].0, admin);
}

#[test]
fn test_set_platform_fee_rejects_excessive() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let client = register_and_initialize(&env, &admin);

    let result = client.try_set_platform_fee(&admin, &11_000);
    match result {
        Err(Ok(ContractError::FeePercentageTooHigh)) => {} // Expected
        other => panic!("Should reject fee > 100%, got: {:?}", other),
    }
}

// ============================================================
// Auth Boundary Tests — collect_fees
// ============================================================

#[test]
fn test_collect_fees_rejects_non_admin() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let client = register_and_initialize(&env, &admin);
    let non_admin = Address::generate(&env);

    let result = client.try_collect_fees(&non_admin);
    match result {
        Err(Ok(ContractError::Unauthorized)) => {} // Expected
        other => panic!("Non-admin should get Unauthorized, got: {:?}", other),
    }
}

#[test]
fn test_collect_fees_succeeds_with_admin_auth() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let client = register_and_initialize(&env, &admin);
    let user = Address::generate(&env);

    // Record enough fees to meet the default collection threshold (100M stroops)
    client.record_fee(&user, &150_000_000);
    // Drain auth from record_fee
    let _ = env.auths();

    client.collect_fees(&admin);

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    assert_eq!(auths[0].0, admin);
}

#[test]
fn test_collect_fees_fails_below_threshold() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let client = register_and_initialize(&env, &admin);
    let user = Address::generate(&env);

    // Record a tiny amount below the default threshold (100M stroops)
    client.record_fee(&user, &100);

    let result = client.try_collect_fees(&admin);
    match result {
        Err(Ok(ContractError::BelowCollectionThreshold)) => {} // Expected
        other => panic!("Collection below threshold should fail, got: {:?}", other),
    }
}

// ============================================================
// Auth Boundary Tests — pause_fees
// ============================================================

#[test]
fn test_pause_fees_rejects_non_admin() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let client = register_and_initialize(&env, &admin);
    let non_admin = Address::generate(&env);

    let result = client.try_pause_fees(&non_admin);
    match result {
        Err(Ok(ContractError::Unauthorized)) => {} // Expected
        other => panic!("Non-admin should get Unauthorized, got: {:?}", other),
    }
}

#[test]
fn test_pause_fees_succeeds_with_admin_auth() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let client = register_and_initialize(&env, &admin);

    client.pause_fees(&admin);

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    assert_eq!(auths[0].0, admin);
    assert!(client.is_paused());
}

#[test]
fn test_pause_fees_prevents_record_fee() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let client = register_and_initialize(&env, &admin);
    let user = Address::generate(&env);

    client.pause_fees(&admin);

    let result = client.try_record_fee(&user, &1000);
    match result {
        Err(Ok(ContractError::FeesPaused)) => {} // Expected
        other => panic!(
            "Record fee should fail when fees are paused, got: {:?}",
            other
        ),
    }
}

// ============================================================
// Auth Boundary Tests — unpause_fees
// ============================================================

#[test]
fn test_unpause_fees_rejects_non_admin() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let client = register_and_initialize(&env, &admin);
    let non_admin = Address::generate(&env);

    client.pause_fees(&admin);
    let _ = env.auths(); // drain pause auth

    let result = client.try_unpause_fees(&non_admin);
    match result {
        Err(Ok(ContractError::Unauthorized)) => {} // Expected
        other => panic!("Non-admin should get Unauthorized, got: {:?}", other),
    }
}

#[test]
fn test_unpause_fees_succeeds_with_admin_auth() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let client = register_and_initialize(&env, &admin);

    client.pause_fees(&admin);
    let _ = env.auths(); // drain pause auth

    client.unpause_fees(&admin);

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    assert_eq!(auths[0].0, admin);
    assert!(!client.is_paused());
}

// ============================================================
// Auth Boundary Tests — transfer_admin
// ============================================================

#[test]
fn test_transfer_admin_rejects_non_admin() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let client = register_and_initialize(&env, &admin);
    let non_admin = Address::generate(&env);
    let new_admin = Address::generate(&env);

    let result = client.try_transfer_admin(&non_admin, &new_admin);
    match result {
        Err(Ok(ContractError::Unauthorized)) => {} // Expected
        other => panic!("Non-admin should get Unauthorized, got: {:?}", other),
    }
}

#[test]
fn test_transfer_admin_succeeds_with_admin_auth() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let client = register_and_initialize(&env, &admin);
    let new_admin = Address::generate(&env);

    client.transfer_admin(&admin, &new_admin);

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    assert_eq!(auths[0].0, admin);
    assert_eq!(client.get_admin(), new_admin);
}

#[test]
fn test_transfer_admin_rejects_same_address() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let client = register_and_initialize(&env, &admin);

    let result = client.try_transfer_admin(&admin, &admin);
    match result {
        Err(Ok(ContractError::InvalidInput)) => {} // Expected
        other => panic!("Should reject transferring to same admin, got: {:?}", other),
    }
}

#[test]
fn test_transfer_admin_new_admin_can_operate() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let client = register_and_initialize(&env, &admin);
    let new_admin = Address::generate(&env);

    client.transfer_admin(&admin, &new_admin);

    // Old admin should no longer be able to operate
    let result = client.try_pause_fees(&admin);
    match result {
        Err(Ok(ContractError::Unauthorized)) => {} // Expected
        other => panic!("Old admin should get Unauthorized, got: {:?}", other),
    }

    // New admin should be able to operate
    client.pause_fees(&new_admin);
    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    assert_eq!(auths[0].0, new_admin);
}

// ============================================================
// Auth Boundary Tests — record_fee
// ============================================================

/// `record_fee` uses `payer.require_auth()` instead of admin
/// authorization. Any address can record a fee for itself.
/// Because `mock_all_auths` must be active after initialization,
/// we verify the authorization via `env.auths()` — the payer
/// address must appear in the auth snapshot.
#[test]
fn test_record_fee_requires_payer_auth() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let client = register_and_initialize(&env, &admin);
    let payer = Address::generate(&env);

    client.record_fee(&payer, &1000);

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    assert_eq!(
        auths[0].0, payer,
        "record_fee should authenticate the payer, not the admin"
    );
}

#[test]
fn test_record_fee_rejects_zero_or_negative() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let client = register_and_initialize(&env, &admin);
    let user = Address::generate(&env);

    // Zero amount should fail
    let result = client.try_record_fee(&user, &0);
    match result {
        Err(Ok(ContractError::InvalidInput)) => {} // Expected
        other => panic!("Zero fee should be rejected, got: {:?}", other),
    }

    // Negative amount should fail
    let result = client.try_record_fee(&user, &-100);
    match result {
        Err(Ok(ContractError::InvalidInput)) => {} // Expected
        other => panic!("Negative fee should be rejected, got: {:?}", other),
    }
}

// ============================================================
// Read-Only Entrypoints — No Auth Required
// ============================================================

#[test]
fn test_read_only_entrypoints_no_auth_required() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let client = register_and_initialize(&env, &admin);

    // All read-only functions should succeed without additional auth
    assert_eq!(client.version(), 7);

    let config = client.get_fee_config();
    assert_eq!(config.platform_fee_percentage, 200); // default

    let stored_admin = client.get_admin();
    assert_eq!(stored_admin, admin);

    assert_eq!(client.get_collected_fees(), 0);
    assert!(!client.is_paused());

    let schedule = client.get_withdrawal_schedule();
    assert_eq!(schedule.status, fees::FeeWithdrawalStatus::Ready);
}

#[test]
fn test_get_admin_fails_when_not_initialized() {
    let env = Env::default();
    let contract_id = env.register(FeesContract, ());
    let client = FeesContractClient::new(&env, &contract_id);

    let result = client.try_get_admin();
    match result {
        Err(Ok(ContractError::AdminNotSet)) => {} // Expected
        other => panic!(
            "get_admin should fail with AdminNotSet when uninitialized, got: {:?}",
            other
        ),
    }
}

// ============================================================
// Comprehensive Authorization Coverage Verification
// ============================================================

#[test]
fn test_all_state_changing_entrypoints_require_auth() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let client = register_and_initialize(&env, &admin);
    let non_admin = Address::generate(&env);
    let new_addr = Address::generate(&env);
    let config = default_fee_config();

    // Each admin-restricted entrypoint invoked with a non-admin caller
    // should return ContractError::Unauthorized.

    let r = client.try_update_fee_config(&non_admin, &config);
    assert!(
        matches!(r, Err(Ok(ContractError::Unauthorized))),
        "update_fee_config should reject non-admin"
    );

    let r = client.try_set_platform_fee(&non_admin, &300);
    assert!(
        matches!(r, Err(Ok(ContractError::Unauthorized))),
        "set_platform_fee should reject non-admin"
    );

    let r = client.try_collect_fees(&non_admin);
    assert!(
        matches!(r, Err(Ok(ContractError::Unauthorized))),
        "collect_fees should reject non-admin"
    );

    let r = client.try_pause_fees(&non_admin);
    assert!(
        matches!(r, Err(Ok(ContractError::Unauthorized))),
        "pause_fees should reject non-admin"
    );

    let r = client.try_unpause_fees(&non_admin);
    assert!(
        matches!(r, Err(Ok(ContractError::Unauthorized))),
        "unpause_fees should reject non-admin"
    );

    let r = client.try_transfer_admin(&non_admin, &new_addr);
    assert!(
        matches!(r, Err(Ok(ContractError::Unauthorized))),
        "transfer_admin should reject non-admin"
    );

    // record_fee uses payer auth, not admin auth — verify via auth snapshot
    let payer = Address::generate(&env);
    client.record_fee(&payer, &1000);
    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    assert_eq!(auths[0].0, payer, "record_fee should auth the payer");
}

#[test]
fn test_all_state_changing_entrypoints_succeed_with_proper_auth() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let client = register_and_initialize(&env, &admin);
    let user = Address::generate(&env);

    let config = default_fee_config();

    assert!(
        client.try_update_fee_config(&admin, &config).is_ok(),
        "Admin should invoke update_fee_config"
    );
    assert!(
        client.try_set_platform_fee(&admin, &300).is_ok(),
        "Admin should invoke set_platform_fee"
    );
    assert!(
        client.try_pause_fees(&admin).is_ok(),
        "Admin should invoke pause_fees"
    );
    assert!(
        client.try_unpause_fees(&admin).is_ok(),
        "Admin should invoke unpause_fees"
    );

    // record_fee uses payer auth
    assert!(
        client.try_record_fee(&user, &200_000_000).is_ok(),
        "User should invoke record_fee"
    );

    // collect_fees needs fees above default threshold (100M stroops)
    assert!(
        client.try_collect_fees(&admin).is_ok(),
        "Admin should invoke collect_fees"
    );

    let another_admin = Address::generate(&env);
    assert!(
        client.try_transfer_admin(&admin, &another_admin).is_ok(),
        "Admin should invoke transfer_admin"
    );
}

#[test]
fn test_get_admin_after_initialize() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register(FeesContract, ());
    let client = FeesContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    let stored_admin = client.get_admin();
    assert_eq!(stored_admin, admin);
}
