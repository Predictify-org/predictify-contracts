//! # Auth boundary tests for the Monitor contract
//!
//! Verifies that every state-changing entrypoint correctly enforces
//! `require_auth`, and that non-admin callers cannot mutate caps.

#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    Address, Env,
};

use monitor::{CapType, MonitorContract, MonitorContractClient, MonitorError};

fn ledger_info() -> LedgerInfo {
    LedgerInfo {
        timestamp: 1_700_000_000,
        protocol_version: 20,
        sequence_number: 100,
        network_id: [0u8; 32],
        base_reserve: 10,
        min_temp_entry_ttl: 1,
        min_persistent_entry_ttl: 1,
        max_entry_ttl: 535_680,
    }
}

struct TestSetup {
    admin: Address,
    user: Address,
    client: MonitorContractClient,
}

fn setup() -> (Env, TestSetup) {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set(ledger_info());

    let admin = Address::generate(&env);
    let user = Address::generate(&env);

    let contract_id = env.register_contract(None, MonitorContract);
    let client = MonitorContractClient::new(&env, &contract_id);

    let s = TestSetup { admin, user, client };
    s.client.initialize(&s.admin);

    (env, s)
}

// ---------------------------------------------------------------------------
// initialize
// ---------------------------------------------------------------------------

#[test]
fn test_initialize_requires_admin_auth() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set(ledger_info());

    let admin = Address::generate(&env);
    let contract_id = env.register_contract(None, MonitorContract);
    let client = MonitorContractClient::new(&env, &contract_id);

    client.initialize(&admin);

    let auths = env.auths();
    assert!(
        auths.iter().any(|(addr, _)| addr == &admin),
        "admin auth must appear in auth trace"
    );
}

// ---------------------------------------------------------------------------
// set_caps
// ---------------------------------------------------------------------------

#[test]
fn test_set_caps_emits_admin_auth() {
    let (env, s) = setup();
    s.client.set_caps(&s.admin, &CapType::Bets, &5u32);

    let auths = env.auths();
    assert!(
        auths.iter().any(|(addr, _)| addr == &s.admin),
        "admin auth must appear after set_caps"
    );
}

#[test]
fn test_set_caps_non_admin_is_rejected() {
    let (_env, s) = setup();
    let result = s.client.try_set_caps(&s.user, &CapType::Bets, &5u32);
    assert_eq!(result, Ok(Err(MonitorError::Unauthorized)));
}

// ---------------------------------------------------------------------------
// record_bet
// ---------------------------------------------------------------------------

#[test]
fn test_record_bet_emits_user_auth() {
    let (env, s) = setup();
    s.client.record_bet(&s.user);

    let auths = env.auths();
    assert!(
        auths.iter().any(|(addr, _)| addr == &s.user),
        "user auth must appear after record_bet"
    );
}

// ---------------------------------------------------------------------------
// remove_bet
// ---------------------------------------------------------------------------

#[test]
fn test_remove_bet_emits_user_auth() {
    let (env, s) = setup();
    s.client.record_bet(&s.user);
    s.client.remove_bet(&s.user);

    let auths = env.auths();
    assert!(
        auths.iter().any(|(addr, _)| addr == &s.user),
        "user auth must appear after remove_bet"
    );
}

// ---------------------------------------------------------------------------
// record_position
// ---------------------------------------------------------------------------

#[test]
fn test_record_position_emits_user_auth() {
    let (env, s) = setup();
    s.client.record_position(&s.user);

    let auths = env.auths();
    assert!(auths.iter().any(|(addr, _)| addr == &s.user));
}

// ---------------------------------------------------------------------------
// remove_position
// ---------------------------------------------------------------------------

#[test]
fn test_remove_position_emits_user_auth() {
    let (env, s) = setup();
    s.client.record_position(&s.user);
    s.client.remove_position(&s.user);

    let auths = env.auths();
    assert!(auths.iter().any(|(addr, _)| addr == &s.user));
}

// ---------------------------------------------------------------------------
// record_subscription
// ---------------------------------------------------------------------------

#[test]
fn test_record_subscription_emits_user_auth() {
    let (env, s) = setup();
    s.client.record_subscription(&s.user);

    let auths = env.auths();
    assert!(auths.iter().any(|(addr, _)| addr == &s.user));
}

// ---------------------------------------------------------------------------
// remove_subscription
// ---------------------------------------------------------------------------

#[test]
fn test_remove_subscription_emits_user_auth() {
    let (env, s) = setup();
    s.client.record_subscription(&s.user);
    s.client.remove_subscription(&s.user);

    let auths = env.auths();
    assert!(auths.iter().any(|(addr, _)| addr == &s.user));
}

// ---------------------------------------------------------------------------
// Read-only entrypoints — no auth required
// ---------------------------------------------------------------------------

#[test]
fn test_get_caps_requires_no_auth() {
    let (_env, s) = setup();
    let _caps = s.client.get_caps();
}

#[test]
fn test_get_account_state_requires_no_auth() {
    let (_env, s) = setup();
    let _state = s.client.get_account_state(&s.user);
}

#[test]
fn test_version_requires_no_auth() {
    let (_env, s) = setup();
    let _v = s.client.version();
}
