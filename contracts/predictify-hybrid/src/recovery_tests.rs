//! Recovery subsystem tests — issue #984.
//!
//! Verifies the recovery-related contract entrypoints:
//!
//! - `validate_market_state_integrity` — pure read, returns `bool`.
//! - `recover_market_state` — admin-only state reconstruction.
//! - `partial_refund_mechanism` — admin-only, refund selected users.
//! - `get_recovery_status` — pure read, returns a status string.
//!
//! The `capabilities()` bitmap specifically advertises RECOVERY (bit 17) so
//! that clients can discover the subsystem before calling these functions.
//! The RECOVERY bit itself is tested in `capability_bitmap_tests.rs`.

#![cfg(test)]

use crate::{capabilities::capability, test::PredictifyTest, PredictifyHybridClient};
use soroban_sdk::Vec;

// ── helpers ──────────────────────────────────────────────────────────────────

/// Set up a contract + an open market, returning both.
fn setup_with_market() -> (PredictifyTest, soroban_sdk::Symbol) {
    let test = PredictifyTest::setup();
    let mkt_id = test.create_test_market();
    (test, mkt_id)
}

// ── capabilities() advertises RECOVERY ──────────────────────────────────────

/// The capabilities bitmap must have RECOVERY set so clients can detect the
/// subsystem is available before calling any recovery entrypoints.
#[test]
fn test_capabilities_advertises_recovery() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    let caps = client.capabilities();

    assert!(
        caps & capability::RECOVERY != 0,
        "RECOVERY bit must be advertised; caps = {:#018x}",
        caps
    );
}

/// capabilities() does not alter contract state when called before any
/// recovery operation.
#[test]
fn test_capabilities_is_pure_before_recovery_ops() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    let events_before = test.env.events().all().len();

    let caps = client.capabilities();
    assert!(caps & capability::RECOVERY != 0);

    assert_eq!(
        test.env.events().all().len(),
        events_before,
        "capabilities() must emit no events"
    );
}

// ── validate_market_state_integrity ─────────────────────────────────────────

/// A freshly created, well-formed market must pass integrity validation.
#[test]
fn test_integrity_valid_for_new_market() {
    let (test, mkt_id) = setup_with_market();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    test.env.mock_all_auths();
    let ok = client.validate_market_state_integrity(&mkt_id);

    assert!(
        ok,
        "validate_market_state_integrity must return true for a healthy market"
    );
}

/// validate_market_state_integrity is a pure read — it must not emit events.
#[test]
fn test_integrity_check_emits_no_events() {
    let (test, mkt_id) = setup_with_market();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    let events_before = test.env.events().all().len();

    test.env.mock_all_auths();
    let _ = client.validate_market_state_integrity(&mkt_id);

    assert_eq!(
        test.env.events().all().len(),
        events_before,
        "validate_market_state_integrity must not emit events"
    );
}

// ── recover_market_state ─────────────────────────────────────────────────────

/// Recovering a healthy market should return false (no reconstruction needed).
#[test]
fn test_recover_healthy_market_returns_false() {
    let (test, mkt_id) = setup_with_market();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    test.env.mock_all_auths();
    let recovered = client.recover_market_state(&test.admin, &mkt_id);

    assert!(
        !recovered,
        "recover_market_state must return false when no reconstruction is needed"
    );
}

// ── partial_refund_mechanism ─────────────────────────────────────────────────

/// Calling partial_refund with an empty user list must return 0.
#[test]
fn test_partial_refund_empty_users_returns_zero() {
    let (test, mkt_id) = setup_with_market();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    let empty_users: Vec<soroban_sdk::Address> = Vec::new(&test.env);

    test.env.mock_all_auths();
    let refunded = client.partial_refund_mechanism(&test.admin, &mkt_id, &empty_users);

    assert_eq!(refunded, 0, "empty user list must produce zero refund total");
}

// ── get_recovery_status ──────────────────────────────────────────────────────

/// get_recovery_status must return a non-empty string for any market
/// (typically "unknown" for markets with no prior recovery activity).
#[test]
fn test_recovery_status_returns_string() {
    let (test, mkt_id) = setup_with_market();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    test.env.mock_all_auths();
    let status = client.get_recovery_status(&mkt_id);

    assert!(
        !status.is_empty(),
        "get_recovery_status must return a non-empty string"
    );
}

/// get_recovery_status is a pure read — it must not emit events.
#[test]
fn test_recovery_status_emits_no_events() {
    let (test, mkt_id) = setup_with_market();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    let events_before = test.env.events().all().len();

    test.env.mock_all_auths();
    let _ = client.get_recovery_status(&mkt_id);

    assert_eq!(
        test.env.events().all().len(),
        events_before,
        "get_recovery_status must not emit events"
    );
}

/// The status string is stable across repeated reads on an unmodified market.
#[test]
fn test_recovery_status_is_stable() {
    let (test, mkt_id) = setup_with_market();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    test.env.mock_all_auths();
    let s1 = client.get_recovery_status(&mkt_id);
    let s2 = client.get_recovery_status(&mkt_id);

    assert_eq!(s1, s2, "get_recovery_status must be deterministic");
}

// ── end-to-end: integrity → recovery → status ───────────────────────────────

/// Full end-to-end: verify that the workflow of checking integrity, attempting
/// recovery, and reading back the status all behave consistently on a healthy
/// market.
#[test]
fn test_recovery_workflow_on_healthy_market() {
    let (test, mkt_id) = setup_with_market();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    test.env.mock_all_auths();

    // 1. Integrity should be valid.
    assert!(client.validate_market_state_integrity(&mkt_id));

    // 2. Recovery should be skipped (no reconstruction needed).
    let recovered = client.recover_market_state(&test.admin, &mkt_id);
    assert!(!recovered);

    // 3. Partial refund with no users should be zero.
    let empty_users: Vec<soroban_sdk::Address> = Vec::new(&test.env);
    let total = client.partial_refund_mechanism(&test.admin, &mkt_id, &empty_users);
    assert_eq!(total, 0);

    // 4. Status must be a non-empty string.
    let status = client.get_recovery_status(&mkt_id);
    assert!(!status.is_empty());

    // 5. The RECOVERY bit must still be set in capabilities.
    let caps = client.capabilities();
    assert!(caps & capability::RECOVERY != 0);
}
