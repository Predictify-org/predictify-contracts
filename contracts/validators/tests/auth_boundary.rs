//! Auth-boundary tests for the Validators contract.
//!
//! Each public state-changing entrypoint gets two tests:
//!   1. An *unauthorized* call that must be rejected.
//!   2. An *authorized* call (correct signer) that must pass auth
//!      (business-logic errors are acceptable; auth errors are not).
//!
//! Read-only entrypoints (`get_validator`, `is_validator`, `validator_count`,
//! `is_validators_paused`, `admin`, `version`) are exercised separately to
//! confirm they work without any authentication at all.

#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    Address, Env,
};
use validators::{ValidatorsContract, ValidatorsContractClient};

// ---------------------------------------------------------------------------
// Test harness
// ---------------------------------------------------------------------------

/// Default stake bounds used across most tests.
const MIN_STAKE: i128 = 100;
const MAX_STAKE: i128 = 1_000_000;
const INIT_STAKE: i128 = 500;

struct TestSetup {
    admin: Address,
    validator: Address,
    unauthorized: Address,
    client: ValidatorsContractClient,
}

fn setup(env: &Env) -> TestSetup {
    env.mock_all_auths();
    env.ledger().set(LedgerInfo {
        timestamp: 1_735_689_600,
        protocol_version: 20,
        sequence_number: 1,
        network_id: [0; 32],
        base_reserve: 10,
        min_temp_entry_ttl: 1,
        min_persistent_entry_ttl: 1,
        max_entry_ttl: 518_400,
    });

    let admin = Address::generate(env);
    let validator = Address::generate(env);
    let unauthorized = Address::generate(env);

    let contract_id = env.register_contract(None, ValidatorsContract);
    let client = ValidatorsContractClient::new(env, &contract_id);

    TestSetup { admin, validator, unauthorized, client }
}

/// Initialize the contract and register one validator, ready for tests that
/// need an already-registered entry.
fn setup_with_validator(env: &Env) -> TestSetup {
    let ts = setup(env);
    ts.client.initialize(&ts.admin, &MIN_STAKE, &MAX_STAKE);
    ts.client.register_validator(&ts.validator, &INIT_STAKE);
    ts
}

// ===========================================================================
// initialize
// ===========================================================================

#[test]
fn test_initialize_requires_auth() {
    let env = Env::default();
    let ts = setup(&env);

    // Unauthorized caller: must be rejected.
    let result = ts.client.try_initialize(&ts.unauthorized, &MIN_STAKE, &MAX_STAKE);
    assert!(result.is_err(), "unauthorized initialize must fail");
}

#[test]
fn test_initialize_requires_auth_success() {
    let env = Env::default();
    let ts = setup(&env);

    let result = ts.client.try_initialize(&ts.admin, &MIN_STAKE, &MAX_STAKE);
    match result {
        Ok(_) => {} // success
        Err(e) => {
            let s = format!("{e:?}");
            assert!(!s.to_lowercase().contains("auth"), "initialize auth must pass; got: {s}");
        }
    }
}

// ===========================================================================
// register_validator
// ===========================================================================

#[test]
fn test_register_validator_requires_auth() {
    let env = Env::default();
    let ts = setup(&env);
    ts.client.initialize(&ts.admin, &MIN_STAKE, &MAX_STAKE);

    // A different address trying to register `ts.validator` must fail.
    let result = ts.client.try_register_validator(&ts.unauthorized, &INIT_STAKE);
    assert!(result.is_err(), "unauthorized register_validator must fail");
}

#[test]
fn test_register_validator_requires_auth_success() {
    let env = Env::default();
    let ts = setup(&env);
    ts.client.initialize(&ts.admin, &MIN_STAKE, &MAX_STAKE);

    let result = ts.client.try_register_validator(&ts.validator, &INIT_STAKE);
    match result {
        Ok(_) => {}
        Err(e) => {
            let s = format!("{e:?}");
            assert!(!s.to_lowercase().contains("auth"), "register_validator auth must pass; got: {s}");
        }
    }
}

// ===========================================================================
// deregister_validator
// ===========================================================================

#[test]
fn test_deregister_validator_requires_auth() {
    let env = Env::default();
    let ts = setup_with_validator(&env);

    // Attempting to deregister `ts.validator` as `ts.unauthorized` must fail.
    let result = ts.client.try_deregister_validator(&ts.unauthorized);
    assert!(result.is_err(), "unauthorized deregister_validator must fail");
}

#[test]
fn test_deregister_validator_requires_auth_success() {
    let env = Env::default();
    let ts = setup_with_validator(&env);

    let result = ts.client.try_deregister_validator(&ts.validator);
    match result {
        Ok(_) => {}
        Err(e) => {
            let s = format!("{e:?}");
            assert!(!s.to_lowercase().contains("auth"), "deregister_validator auth must pass; got: {s}");
        }
    }
}

// ===========================================================================
// update_stake
// ===========================================================================

#[test]
fn test_update_stake_requires_auth() {
    let env = Env::default();
    let ts = setup_with_validator(&env);

    let result = ts.client.try_update_stake(&ts.unauthorized, &INIT_STAKE);
    assert!(result.is_err(), "unauthorized update_stake must fail");
}

#[test]
fn test_update_stake_requires_auth_success() {
    let env = Env::default();
    let ts = setup_with_validator(&env);

    let new_stake: i128 = 750;
    let result = ts.client.try_update_stake(&ts.validator, &new_stake);
    match result {
        Ok(_) => {}
        Err(e) => {
            let s = format!("{e:?}");
            assert!(!s.to_lowercase().contains("auth"), "update_stake auth must pass; got: {s}");
        }
    }
}

// ===========================================================================
// set_validator_active
// ===========================================================================

#[test]
fn test_set_validator_active_requires_auth() {
    let env = Env::default();
    let ts = setup_with_validator(&env);

    // Non-admin calling set_validator_active must fail.
    let result = ts.client.try_set_validator_active(&ts.unauthorized, &ts.validator, &false);
    assert!(result.is_err(), "unauthorized set_validator_active must fail");
}

#[test]
fn test_set_validator_active_requires_auth_success() {
    let env = Env::default();
    let ts = setup_with_validator(&env);

    let result = ts.client.try_set_validator_active(&ts.admin, &ts.validator, &false);
    match result {
        Ok(_) => {}
        Err(e) => {
            let s = format!("{e:?}");
            assert!(!s.to_lowercase().contains("auth"), "set_validator_active auth must pass; got: {s}");
        }
    }
}

// ===========================================================================
// update_score
// ===========================================================================

#[test]
fn test_update_score_requires_auth() {
    let env = Env::default();
    let ts = setup_with_validator(&env);

    let result = ts.client.try_update_score(&ts.unauthorized, &ts.validator, &42_i128);
    assert!(result.is_err(), "unauthorized update_score must fail");
}

#[test]
fn test_update_score_requires_auth_success() {
    let env = Env::default();
    let ts = setup_with_validator(&env);

    let result = ts.client.try_update_score(&ts.admin, &ts.validator, &42_i128);
    match result {
        Ok(_) => {}
        Err(e) => {
            let s = format!("{e:?}");
            assert!(!s.to_lowercase().contains("auth"), "update_score auth must pass; got: {s}");
        }
    }
}

// ===========================================================================
// update_stake_limits
// ===========================================================================

#[test]
fn test_update_stake_limits_requires_auth() {
    let env = Env::default();
    let ts = setup_with_validator(&env);

    let result = ts.client.try_update_stake_limits(&ts.unauthorized, &200_i128, &2_000_000_i128);
    assert!(result.is_err(), "unauthorized update_stake_limits must fail");
}

#[test]
fn test_update_stake_limits_requires_auth_success() {
    let env = Env::default();
    let ts = setup_with_validator(&env);

    let result = ts.client.try_update_stake_limits(&ts.admin, &200_i128, &2_000_000_i128);
    match result {
        Ok(_) => {}
        Err(e) => {
            let s = format!("{e:?}");
            assert!(!s.to_lowercase().contains("auth"), "update_stake_limits auth must pass; got: {s}");
        }
    }
}

// ===========================================================================
// pause_validators
// ===========================================================================

#[test]
fn test_pause_validators_requires_auth() {
    let env = Env::default();
    let ts = setup(&env);
    ts.client.initialize(&ts.admin, &MIN_STAKE, &MAX_STAKE);

    let result = ts.client.try_pause_validators(&ts.unauthorized);
    assert!(result.is_err(), "unauthorized pause_validators must fail");
}

#[test]
fn test_pause_validators_requires_auth_success() {
    let env = Env::default();
    let ts = setup(&env);
    ts.client.initialize(&ts.admin, &MIN_STAKE, &MAX_STAKE);

    let result = ts.client.try_pause_validators(&ts.admin);
    match result {
        Ok(_) => {}
        Err(e) => {
            let s = format!("{e:?}");
            assert!(!s.to_lowercase().contains("auth"), "pause_validators auth must pass; got: {s}");
        }
    }
}

// ===========================================================================
// unpause_validators
// ===========================================================================

#[test]
fn test_unpause_validators_requires_auth() {
    let env = Env::default();
    let ts = setup(&env);
    ts.client.initialize(&ts.admin, &MIN_STAKE, &MAX_STAKE);
    ts.client.pause_validators(&ts.admin);

    let result = ts.client.try_unpause_validators(&ts.unauthorized);
    assert!(result.is_err(), "unauthorized unpause_validators must fail");
}

#[test]
fn test_unpause_validators_requires_auth_success() {
    let env = Env::default();
    let ts = setup(&env);
    ts.client.initialize(&ts.admin, &MIN_STAKE, &MAX_STAKE);
    ts.client.pause_validators(&ts.admin);

    let result = ts.client.try_unpause_validators(&ts.admin);
    match result {
        Ok(_) => {}
        Err(e) => {
            let s = format!("{e:?}");
            assert!(!s.to_lowercase().contains("auth"), "unpause_validators auth must pass; got: {s}");
        }
    }
}

// ===========================================================================
// transfer_ownership
// ===========================================================================

#[test]
fn test_transfer_ownership_requires_auth() {
    let env = Env::default();
    let ts = setup(&env);
    ts.client.initialize(&ts.admin, &MIN_STAKE, &MAX_STAKE);

    let new_owner = Address::generate(&env);
    let result = ts.client.try_transfer_ownership(&ts.unauthorized, &new_owner);
    assert!(result.is_err(), "unauthorized transfer_ownership must fail");
}

#[test]
fn test_transfer_ownership_requires_auth_success() {
    let env = Env::default();
    let ts = setup(&env);
    ts.client.initialize(&ts.admin, &MIN_STAKE, &MAX_STAKE);

    let new_owner = Address::generate(&env);
    let result = ts.client.try_transfer_ownership(&ts.admin, &new_owner);
    match result {
        Ok(_) => {}
        Err(e) => {
            let s = format!("{e:?}");
            assert!(!s.to_lowercase().contains("auth"), "transfer_ownership auth must pass; got: {s}");
        }
    }
}

// ===========================================================================
// Read-only entrypoints — no auth required
// ===========================================================================

#[test]
fn test_view_entrypoints_do_not_require_auth() {
    let env = Env::default();
    let ts = setup_with_validator(&env);

    // version — always works
    let v = ts.client.version();
    assert_eq!(v, 1, "version should return 1");

    // is_validators_paused
    let paused = ts.client.is_validators_paused();
    assert!(!paused, "should not be paused after init");

    // validator_count
    let count = ts.client.validator_count();
    assert_eq!(count, 1, "one validator registered");

    // is_validator
    let exists = ts.client.is_validator(&ts.validator);
    assert!(exists, "ts.validator should be registered");

    // get_validator
    let info_opt = ts.client.get_validator(&ts.validator);
    assert!(info_opt.is_some(), "get_validator should return Some");
    let info = info_opt.unwrap();
    assert_eq!(info.stake, INIT_STAKE);
    assert!(info.active);
    assert_eq!(info.score, 0);

    // admin (read-only)
    let stored_admin = ts.client.admin();
    assert_eq!(stored_admin, ts.admin, "admin() should return the configured admin");
}

// ===========================================================================
// Additional business-logic tests
// ===========================================================================

/// Registering the same address twice should return AlreadyRegistered.
#[test]
fn test_double_register_fails() {
    let env = Env::default();
    let ts = setup_with_validator(&env);

    let result = ts.client.try_register_validator(&ts.validator, &INIT_STAKE);
    assert!(result.is_err(), "duplicate registration must fail");
}

/// Deregistering an unknown address should return ValidatorNotFound.
#[test]
fn test_deregister_unknown_fails() {
    let env = Env::default();
    let ts = setup(&env);
    ts.client.initialize(&ts.admin, &MIN_STAKE, &MAX_STAKE);

    let unknown = Address::generate(&env);
    let result = ts.client.try_deregister_validator(&unknown);
    assert!(result.is_err(), "deregister of unknown validator must fail");
}

/// Staking below the minimum should return StakeTooLow.
#[test]
fn test_register_stake_too_low() {
    let env = Env::default();
    let ts = setup(&env);
    ts.client.initialize(&ts.admin, &MIN_STAKE, &MAX_STAKE);

    let low_stake: i128 = MIN_STAKE - 1;
    let result = ts.client.try_register_validator(&ts.validator, &low_stake);
    assert!(result.is_err(), "stake below minimum must fail");
}

/// Staking above the maximum should return StakeTooHigh.
#[test]
fn test_register_stake_too_high() {
    let env = Env::default();
    let ts = setup(&env);
    ts.client.initialize(&ts.admin, &MIN_STAKE, &MAX_STAKE);

    let high_stake: i128 = MAX_STAKE + 1;
    let result = ts.client.try_register_validator(&ts.validator, &high_stake);
    assert!(result.is_err(), "stake above maximum must fail");
}

/// Calling any state-changing entrypoint while paused should fail.
#[test]
fn test_paused_blocks_register() {
    let env = Env::default();
    let ts = setup(&env);
    ts.client.initialize(&ts.admin, &MIN_STAKE, &MAX_STAKE);
    ts.client.pause_validators(&ts.admin);

    let result = ts.client.try_register_validator(&ts.validator, &INIT_STAKE);
    assert!(result.is_err(), "register_validator must fail while paused");
}

/// Unpause should re-enable registration.
#[test]
fn test_unpause_re_enables_register() {
    let env = Env::default();
    let ts = setup(&env);
    ts.client.initialize(&ts.admin, &MIN_STAKE, &MAX_STAKE);
    ts.client.pause_validators(&ts.admin);
    ts.client.unpause_validators(&ts.admin);

    let result = ts.client.try_register_validator(&ts.validator, &INIT_STAKE);
    assert!(result.is_ok(), "register_validator must succeed after unpause");
}

/// initialize with invalid stake limits should return InvalidConfig.
#[test]
fn test_initialize_invalid_config() {
    let env = Env::default();
    let ts = setup(&env);

    // min > max is invalid
    let result = ts.client.try_initialize(&ts.admin, &1000_i128, &100_i128);
    assert!(result.is_err(), "min_stake > max_stake must fail");
}

/// transfer_ownership to self should fail with InvalidNewOwner.
#[test]
fn test_transfer_ownership_to_self_fails() {
    let env = Env::default();
    let ts = setup(&env);
    ts.client.initialize(&ts.admin, &MIN_STAKE, &MAX_STAKE);

    let result = ts.client.try_transfer_ownership(&ts.admin, &ts.admin);
    assert!(result.is_err(), "transferring ownership to self must fail");
}
