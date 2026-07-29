#![cfg(test)]

//! Tests for the error migration script sketch.
//!
//! Validates that `migrate_error_data` correctly guards preconditions,
//! applies version bumps, and rejects invalid inputs.

use migrate::{ContractError, MigrateContract, MigrateContractClient};
use soroban_sdk::{testutils::Address as _, Address, Env};

/// Deploy and initialise a fresh migrate contract.
struct Fixture {
    #[allow(dead_code)]
    env: Env,
    client: MigrateContractClient<'static>,
    admin: Address,
}

impl Fixture {
    fn new(initial_version: u32) -> Self {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(MigrateContract, ());
        let admin = Address::generate(&env);
        let client = MigrateContractClient::new(&env, &contract_id);
        client.initialize(&admin, &initial_version);
        Self {
            env,
            client,
            admin,
        }
    }
}

// ---------------------------------------------------------------------------
// Happy path
// ---------------------------------------------------------------------------

#[test]
fn successful_migration_bumps_version() {
    let f = Fixture::new(1);

    let result = f.client.try_migrate_error_data(&f.admin, &1, &2);
    assert_eq!(result, Ok(Ok(())), "migration from 1→2 should succeed");

    assert_eq!(
        f.client.current_version(),
        2,
        "stored version must be 2 after migration"
    );
}

#[test]
fn migration_can_skip_multiple_versions() {
    let f = Fixture::new(1);

    assert_eq!(f.client.try_migrate_error_data(&f.admin, &1, &5), Ok(Ok(())));
    assert_eq!(f.client.current_version(), 5);
}

#[test]
fn migrate_from_non_default_start() {
    // Start at version 2 (the maximum allowed by CURRENT_VERSION) and
    // migrate forward — demonstrating non-default starting points work.
    let f = Fixture::new(2);

    assert_eq!(f.client.try_migrate_error_data(&f.admin, &2, &3), Ok(Ok(())));
    assert_eq!(f.client.current_version(), 3);
}

// ---------------------------------------------------------------------------
// Guard: version compare-and-set
// ---------------------------------------------------------------------------

#[test]
fn rejects_expected_version_mismatch() {
    // Start at version 1; pass expected=2 (wrong) — should get VersionMismatch.
    let f = Fixture::new(1);

    assert_eq!(
        f.client.try_migrate_error_data(&f.admin, &2, &4),
        Err(Ok(ContractError::VersionMismatch)),
        "should reject expected=2 when stored=1"
    );
    assert_eq!(
        f.client.current_version(),
        1,
        "state must remain unchanged on error"
    );
}

#[test]
fn rejects_target_equal_to_current() {
    let f = Fixture::new(2);

    assert_eq!(
        f.client.try_migrate_error_data(&f.admin, &2, &2),
        Err(Ok(ContractError::InvalidTargetVersion)),
        "target==current is not an upgrade"
    );
    assert_eq!(f.client.current_version(), 2);
}

#[test]
fn rejects_target_lower_than_current() {
    let f = Fixture::new(2);

    assert_eq!(
        f.client.try_migrate_error_data(&f.admin, &2, &1),
        Err(Ok(ContractError::InvalidTargetVersion)),
        "downgrade must be rejected"
    );
    assert_eq!(f.client.current_version(), 2);
}

// ---------------------------------------------------------------------------
// Guard: not-initialised
// ---------------------------------------------------------------------------

#[test]
fn rejects_uninitialised_contract() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(MigrateContract, ());
    let admin = Address::generate(&env);
    let client = MigrateContractClient::new(&env, &contract_id);

    assert_eq!(
        client.try_migrate_error_data(&admin, &1, &2),
        Err(Ok(ContractError::NotInitialized)),
        "uninitialised contract must return NotInitialized"
    );
}

// ---------------------------------------------------------------------------
// Guard: caller authentication
// ---------------------------------------------------------------------------

#[test]
fn rejects_non_admin_caller() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(MigrateContract, ());
    let admin = Address::generate(&env);
    let stranger = Address::generate(&env);
    let client = MigrateContractClient::new(&env, &contract_id);

    client.initialize(&admin, &1);

    assert_eq!(
        client.try_migrate_error_data(&stranger, &1, &2),
        Err(Ok(ContractError::Unauthorized)),
        "non-admin caller must be rejected"
    );
    assert_eq!(client.current_version(), 1, "state must remain unchanged");
}

// ---------------------------------------------------------------------------
// Integrity: state unchanged on failure
// ---------------------------------------------------------------------------

#[test]
fn state_not_changed_on_version_mismatch() {
    let f = Fixture::new(1);

    let _ = f.client.try_migrate_error_data(&f.admin, &2, &3);
    assert_eq!(f.client.current_version(), 1);
}

#[test]
fn state_not_changed_on_invalid_target() {
    let f = Fixture::new(2);

    let _ = f.client.try_migrate_error_data(&f.admin, &2, &2);
    assert_eq!(f.client.current_version(), 2);

    let _ = f.client.try_migrate_error_data(&f.admin, &2, &1);
    assert_eq!(f.client.current_version(), 2);
}
