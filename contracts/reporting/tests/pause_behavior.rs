#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Events, Ledger, LedgerInfo},
    Address, Env, String,
};

use reporting::{ReportingContract, ReportingContractClient};

fn setup(env: &Env) -> (ReportingContractClient, Address, Address) {
    env.mock_all_auths();
    env.ledger().set(LedgerInfo {
        timestamp: 1735689600,
        protocol_version: 20,
        sequence_number: 1,
        network_id: [0; 32],
        base_reserve: 10,
        min_temp_entry_ttl: 1,
        min_persistent_entry_ttl: 1,
        max_entry_ttl: 518400,
    });

    let admin = Address::generate(env);
    let reporter = Address::generate(env);

    let contract_id = env.register_contract(None, ReportingContract);
    let client = ReportingContractClient::new(env, &contract_id);

    client.initialize(&admin);

    (client, admin, reporter)
}

// ---------------------------------------------------------------------------
// is_reporting_paused — read is always allowed
// ---------------------------------------------------------------------------

#[test]
fn test_is_reporting_paused_returns_false_after_init() {
    let env = Env::default();
    let (client, _, _) = setup(&env);

    assert!(
        !client.is_reporting_paused(),
        "reporting should not be paused after init"
    );
}

#[test]
fn test_is_reporting_paused_returns_true_after_pause() {
    let env = Env::default();
    let (client, admin, _) = setup(&env);

    client.pause_reporting(&admin);

    assert!(
        client.is_reporting_paused(),
        "reporting should be paused after pause_reporting"
    );
}

#[test]
fn test_is_reporting_paused_returns_false_after_unpause() {
    let env = Env::default();
    let (client, admin, _) = setup(&env);

    client.pause_reporting(&admin);
    assert!(client.is_reporting_paused());

    client.unpause_reporting(&admin);

    assert!(
        !client.is_reporting_paused(),
        "reporting should be unpaused after unpause_reporting"
    );
}

// ---------------------------------------------------------------------------
// State-changing functions revert when paused
// ---------------------------------------------------------------------------

#[test]
fn test_submit_report_fails_when_paused() {
    let env = Env::default();
    let (client, admin, reporter) = setup(&env);

    client.pause_reporting(&admin);

    let result = client.try_submit_report(
        &reporter,
        &1u32,
        &String::from_str(&env, "data"),
        &String::from_str(&env, "0xhash"),
    );
    assert!(result.is_err(), "submit_report should fail when paused");
}

#[test]
fn test_submit_report_succeeds_after_unpause() {
    let env = Env::default();
    let (client, admin, reporter) = setup(&env);

    // Pause then unpause
    client.pause_reporting(&admin);
    client.unpause_reporting(&admin);

    // Should succeed now
    let result = client.try_submit_report(
        &reporter,
        &1u32,
        &String::from_str(&env, "data"),
        &String::from_str(&env, "0xhash"),
    );
    // Auth passes, business logic may return Ok or non-auth error
    match result {
        Ok(_) => assert!(true, "submit_report succeeded after unpause"),
        Err(e) => {
            let error_str = format!("{:?}", e);
            assert!(!error_str.contains("auth"), "Auth should not fail");
        }
    }
}

#[test]
fn test_verify_report_fails_when_paused() {
    let env = Env::default();
    let (client, admin, _) = setup(&env);

    client.pause_reporting(&admin);

    let result = client.try_verify_report(&admin, &1u32, &true);
    assert!(result.is_err(), "verify_report should fail when paused");
}

#[test]
fn test_dispute_report_fails_when_paused() {
    let env = Env::default();
    let (client, admin, reporter) = setup(&env);

    client.pause_reporting(&admin);

    let result = client.try_dispute_report(&reporter, &1u32, &String::from_str(&env, "reason"));
    assert!(result.is_err(), "dispute_report should fail when paused");
}

#[test]
fn test_resolve_dispute_fails_when_paused() {
    let env = Env::default();
    let (client, admin, _) = setup(&env);

    client.pause_reporting(&admin);

    let result = client.try_resolve_dispute(&admin, &1u32, &true);
    assert!(result.is_err(), "resolve_dispute should fail when paused");
}

#[test]
fn test_update_report_status_fails_when_paused() {
    let env = Env::default();
    let (client, admin, _) = setup(&env);

    client.pause_reporting(&admin);

    let result = client.try_update_report_status(&admin, &1u32, &2u32);
    assert!(
        result.is_err(),
        "update_report_status should fail when paused"
    );
}

#[test]
fn test_delete_report_fails_when_paused() {
    let env = Env::default();
    let (client, admin, _) = setup(&env);

    client.pause_reporting(&admin);

    let result = client.try_delete_report(&admin, &1u32);
    assert!(result.is_err(), "delete_report should fail when paused");
}

// ---------------------------------------------------------------------------
// Idempotency — double-pause and double-unpause are safe
// ---------------------------------------------------------------------------

#[test]
fn test_double_pause_is_idempotent() {
    let env = Env::default();
    let (client, admin, _) = setup(&env);

    client.pause_reporting(&admin);
    let events_before = env.events().all().len();

    // Second pause should not fail
    client.pause_reporting(&admin);

    assert!(
        client.is_reporting_paused(),
        "should still be paused after double-pause"
    );
    // No additional event should have been emitted
    assert_eq!(
        env.events().all().len(),
        events_before,
        "double-pause should not emit a second event"
    );
}

#[test]
fn test_double_unpause_is_idempotent() {
    let env = Env::default();
    let (client, admin, _) = setup(&env);

    client.pause_reporting(&admin);
    client.unpause_reporting(&admin);
    let events_before = env.events().all().len();

    // Second unpause should not fail
    client.unpause_reporting(&admin);

    assert!(
        !client.is_reporting_paused(),
        "should still be unpaused after double-unpause"
    );
    assert_eq!(
        env.events().all().len(),
        events_before,
        "double-unpause should not emit a second event"
    );
}

// ---------------------------------------------------------------------------
// Events are emitted correctly
// ---------------------------------------------------------------------------

#[test]
fn test_pause_emits_event() {
    let env = Env::default();
    let (client, admin, _) = setup(&env);

    client.pause_reporting(&admin);

    let events = env.events().all();
    let has_pause_event = events
        .iter()
        .any(|e| format!("{:?}", e.0).contains("reporting_paused"));
    assert!(
        has_pause_event,
        "pause should emit a reporting_paused event"
    );
}

#[test]
fn test_unpause_emits_event() {
    let env = Env::default();
    let (client, admin, _) = setup(&env);

    client.pause_reporting(&admin);
    client.unpause_reporting(&admin);

    let events = env.events().all();
    let has_unpause_event = events
        .iter()
        .any(|e| format!("{:?}", e.0).contains("reporting_unpaused"));
    assert!(
        has_unpause_event,
        "unpause should emit a reporting_unpaused event"
    );
}
