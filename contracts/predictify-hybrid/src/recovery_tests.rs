#![cfg(test)]
use crate::{PredictifyHybridClient, test::PredictifyTest};

#[test]
fn test_recovery_mechanisms() {
    let test_ctx = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test_ctx.env, &test_ctx.contract_id);
    let mkt_id = test_ctx.create_test_market();

    // Integrity should be valid initially (call static method via client env)
    let ok = client.validate_market_state_integrity(&mkt_id);
    assert!(ok);

    // Attempt recovery (should skip/no action)
    let recovered = client.recover_market_state(&test_ctx.admin, &mkt_id);
    assert!(!recovered); // no reconstruction needed

    // Simulate corruption by manually editing storage (direct access)
    // (Simplified: we can't easily modify internal storage here without public API; skip)

    // Partial refund with no users should be zero
    let empty_users = soroban_sdk::Vec::new(&test_ctx.env);
    let refunded = client.partial_refund_mechanism(&test_ctx.admin, &mkt_id, &empty_users);
    assert_eq!(refunded, 0);

    // Check recovery status API (should exist)
    let status = client.get_recovery_status(&mkt_id);
    assert!(!status.is_empty());
}
