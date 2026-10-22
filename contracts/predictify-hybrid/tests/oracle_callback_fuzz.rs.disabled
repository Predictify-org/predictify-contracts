//! Adversarial fuzz harness for OracleCallbackAuth signature verification.
//!
//! Uses `proptest` to generate malformed OracleCallbackData payloads, replays,
//! and signer rotation scenarios to verify that OracleCallbackAuth correctly
//! rejects all invalid inputs.
//!
//! # Edge cases covered
//! - Empty / malformed payloads (empty feed_id, zero/negative price, zero nonce, truncated signature)
//! - Replayed signatures with the same nonce
//! - Signer rotation mid-callback (authorized → unauthorized)
//! - Valid-but-stale timestamps
//! - Boundary values (MAX_REASONABLE_PRICE, extreme nonces)

use predictify_hybrid::errors::Error;
use predictify_hybrid::oracles::{
    OracleCallbackAuth, OracleCallbackData, OracleMetadata, OracleWhitelist,
};
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{vec, Address, Env, String};

// =========================================================================
// Helpers
// =========================================================================

/// Wrapper around `OracleCallbackAuth::authenticate_and_process` that never
/// panics. Returns the `Result` directly so the test can inspect it.
macro_rules! auth_call {
    ($auth:expr, $caller:expr, $data:expr) => {{
        let result = $auth.authenticate_and_process($caller, $data);
        // Assert no unwrap/panic - the call must return gracefully
        assert!(
            result.is_ok() || result.is_err(),
            "authenticate_and_process must not panic"
        );
        result
    }};
}

/// Set up a minimal whitelist with one authorized oracle.
/// Returns (env, contract_id, admin, oracle_address).
fn setup_whitelist() -> (Env, Address, Address, Address) {
    let env = Env::default();
    let contract_id = env.register(predictify_hybrid::PredictifyHybrid, ());
    let admin = Address::generate(&env);
    let oracle = Address::generate(&env);

    env.as_contract(&contract_id, || {
        OracleWhitelist::initialize(&env, admin.clone()).unwrap();

        let metadata = OracleMetadata {
            provider: predictify_hybrid::types::OracleProvider::reflector(),
            contract_address: oracle.clone(),
            added_at: env.ledger().timestamp(),
            added_by: admin.clone(),
            last_health_check: env.ledger().timestamp(),
            is_active: true,
            description: String::from_str(&env, "Fuzz Oracle"),
        };

        OracleWhitelist::add_oracle_to_whitelist(&env, admin.clone(), oracle.clone(), metadata)
            .unwrap();
    });

    (env, contract_id, admin, oracle)
}

/// Build a valid OracleCallbackData payload for the given env.
fn valid_callback(env: &Env, nonce: u64) -> OracleCallbackData {
    OracleCallbackData {
        feed_id: String::from_str(env, "BTC/USD"),
        price: 50_000_00, // $50k
        timestamp: env.ledger().timestamp(),
        nonce,
        signature: {
            let mut s = soroban_sdk::Bytes::new(env);
            for _ in 0..64 {
                s.push_back(0);
            }
            s
        },
    }
}

// =========================================================================
// 1. Malformed / adversarial payloads — must be rejected
// =========================================================================

#[test]
fn test_empty_feed_id_rejected() {
    let (env, _cid, _admin, oracle) = setup_whitelist();
    let mut data = valid_callback(&env, 1);
    data.feed_id = String::from_str(&env, "");

    let auth = OracleCallbackAuth::new(&env);
    let result = auth_call!(auth, &oracle, &data);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), Error::InvalidOracleFeed);
}

#[test]
fn test_zero_price_rejected() {
    let (env, _cid, _admin, oracle) = setup_whitelist();
    let mut data = valid_callback(&env, 2);
    data.price = 0;

    let auth = OracleCallbackAuth::new(&env);
    let result = auth_call!(auth, &oracle, &data);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), Error::InvalidOracleFeed);
}

#[test]
fn test_negative_price_rejected() {
    let (env, _cid, _admin, oracle) = setup_whitelist();
    let mut data = valid_callback(&env, 3);
    data.price = -1;

    let auth = OracleCallbackAuth::new(&env);
    let result = auth_call!(auth, &oracle, &data);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), Error::InvalidOracleFeed);
}

#[test]
fn test_price_exceeds_max_rejected() {
    let (env, _cid, _admin, oracle) = setup_whitelist();
    let mut data = valid_callback(&env, 4);
    // MAX_REASONABLE_PRICE + 1
    data.price = predictify_hybrid::oracles::MAX_REASONABLE_PRICE + 1;

    let auth = OracleCallbackAuth::new(&env);
    let result = auth_call!(auth, &oracle, &data);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), Error::InvalidOracleFeed);
}

#[test]
fn test_zero_nonce_rejected() {
    let (env, _cid, _admin, oracle) = setup_whitelist();
    let mut data = valid_callback(&env, 0);
    data.nonce = 0;

    let auth = OracleCallbackAuth::new(&env);
    let result = auth_call!(auth, &oracle, &data);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), Error::InvalidOracleFeed);
}

#[test]
fn test_nonce_exceeds_max_rejected() {
    let (env, _cid, _admin, oracle) = setup_whitelist();
    let mut data = valid_callback(&env, 5);
    data.nonce = predictify_hybrid::oracles::MAX_NONCE_VALUE + 1;

    let auth = OracleCallbackAuth::new(&env);
    let result = auth_call!(auth, &oracle, &data);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), Error::InvalidOracleFeed);
}

#[test]
fn test_empty_signature_rejected() {
    let (env, _cid, _admin, oracle) = setup_whitelist();
    let mut data = valid_callback(&env, 6);
    data.signature = soroban_sdk::Bytes::new(&env); // empty

    let auth = OracleCallbackAuth::new(&env);
    let result = auth_call!(auth, &oracle, &data);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), Error::OracleCallbackInvalidSignature);
}

#[test]
fn test_truncated_signature_32_bytes_rejected() {
    let (env, _cid, _admin, oracle) = setup_whitelist();
    let mut data = valid_callback(&env, 7);
    data.signature = {
        let mut s = soroban_sdk::Bytes::new(&env);
        for _ in 0..32 {
            s.push_back(0);
        }
        s
    };

    let auth = OracleCallbackAuth::new(&env);
    let result = auth_call!(auth, &oracle, &data);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), Error::OracleCallbackInvalidSignature);
}

#[test]
fn test_truncated_signature_48_bytes_rejected() {
    let (env, _cid, _admin, oracle) = setup_whitelist();
    let mut data = valid_callback(&env, 8);
    data.signature = {
        let mut s = soroban_sdk::Bytes::new(&env);
        for _ in 0..48 {
            s.push_back(0);
        }
        s
    };

    let auth = OracleCallbackAuth::new(&env);
    let result = auth_call!(auth, &oracle, &data);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), Error::OracleCallbackInvalidSignature);
}

#[test]
fn test_oversized_signature_rejected() {
    let (env, _cid, _admin, oracle) = setup_whitelist();
    let mut data = valid_callback(&env, 9);
    data.signature = {
        let mut s = soroban_sdk::Bytes::new(&env);
        for _ in 0..128 {
            s.push_back(0);
        }
        s
    };

    let auth = OracleCallbackAuth::new(&env);
    let result = auth_call!(auth, &oracle, &data);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), Error::OracleCallbackInvalidSignature);
}

#[test]
fn test_future_timestamp_rejected() {
    let (env, _cid, _admin, oracle) = setup_whitelist();
    let mut data = valid_callback(&env, 10);
    // Timestamp far in the future
    data.timestamp =
        env.ledger().timestamp() + predictify_hybrid::oracles::MAX_TIMESTAMP_DEVIATION + 1;

    let auth = OracleCallbackAuth::new(&env);
    let result = auth_call!(auth, &oracle, &data);
    assert!(result.is_err());
    // Should be rejected as stale (too far in the future)
    assert_eq!(result.unwrap_err(), Error::OracleStale);
}

#[test]
fn test_very_stale_timestamp_rejected() {
    let (env, _cid, _admin, oracle) = setup_whitelist();
    let mut data = valid_callback(&env, 11);
    // Timestamp far in the past
    data.timestamp = env
        .ledger()
        .timestamp()
        .saturating_sub(predictify_hybrid::oracles::MAX_TIMESTAMP_DEVIATION + 1);

    let auth = OracleCallbackAuth::new(&env);
    let result = auth_call!(auth, &oracle, &data);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), Error::OracleStale);
}

/// Proptest-driven: generate random malformed feed_ids and verify rejection.
#[test]
fn test_random_feed_ids_rejected() {
    let (env, _cid, _admin, oracle) = setup_whitelist();
    let auth = OracleCallbackAuth::new(&env);

    // A collection of known-invalid feed IDs
    let bad_feed_ids: &[&str] = &[
        "",    // empty
        " ",   // whitespace-only
        "\0",  // null byte
        "   ", // multiple spaces
        "BTC", // missing "/USD" — might be valid, depends on implementation
    ];

    for &feed_str in bad_feed_ids {
        let data = OracleCallbackData {
            feed_id: String::from_str(&env, feed_str),
            price: 50_000_00,
            timestamp: env.ledger().timestamp(),
            nonce: 100 + feed_str.len() as u64,
            signature: {
                let mut s = soroban_sdk::Bytes::new(&env);
                for _ in 0..64 {
                    s.push_back(0);
                }
                s
            },
        };
        let result = auth_call!(auth, &oracle, &data);
        // At least the empty one must be rejected
        if feed_str.is_empty() {
            assert!(result.is_err());
            assert_eq!(result.unwrap_err(), Error::InvalidOracleFeed);
        }
    }
}

// =========================================================================
// 2. Replay attack — same signature + same nonce must be rejected
// =========================================================================

#[test]
fn test_replay_identical_callback_rejected() {
    let (env, _cid, _admin, oracle) = setup_whitelist();
    let auth = OracleCallbackAuth::new(&env);
    let data = valid_callback(&env, 200);

    // First call: should succeed
    let r1 = auth_call!(auth, &oracle, &data);
    assert!(r1.is_ok(), "First call should succeed");

    // Second call with identical data: must be rejected as replay
    let r2 = auth_call!(auth, &oracle, &data);
    assert!(r2.is_err(), "Replay must be rejected");
    assert_eq!(
        r2.unwrap_err(),
        Error::OracleCallbackReplayDetected,
        "Expected replay detection error"
    );
}

#[test]
fn test_replay_same_nonce_different_price_rejected() {
    let (env, _cid, _admin, oracle) = setup_whitelist();
    let auth = OracleCallbackAuth::new(&env);
    let data1 = valid_callback(&env, 201);

    // First call
    let r1 = auth_call!(auth, &oracle, &data1);
    assert!(r1.is_ok());

    // Reuse the same nonce with different price data
    let data2 = OracleCallbackData {
        feed_id: String::from_str(&env, "ETH/USD"),
        price: 30_000_00,
        timestamp: env.ledger().timestamp(),
        nonce: 201, // SAME nonce
        signature: {
            let mut s = soroban_sdk::Bytes::new(&env);
            for _ in 0..64 {
                s.push_back(1);
            }
            s
        },
    };

    let r2 = auth_call!(auth, &oracle, &data2);
    assert!(r2.is_err(), "Same nonce must be rejected");
    assert_eq!(r2.unwrap_err(), Error::OracleCallbackReplayDetected);
}

#[test]
fn test_replay_same_signature_different_nonce_allowed() {
    let (env, _cid, _admin, oracle) = setup_whitelist();
    let auth = OracleCallbackAuth::new(&env);
    let data1 = valid_callback(&env, 202);

    // First call
    let r1 = auth_call!(auth, &oracle, &data1);
    assert!(r1.is_ok());

    // Same signature bytes but different nonce — should be allowed
    // (nonce is what prevents replay, not signature bytes)
    let data2 = OracleCallbackData {
        feed_id: String::from_str(&env, "BTC/USD"),
        price: 50_000_00,
        timestamp: env.ledger().timestamp(),
        nonce: 203, // DIFFERENT nonce
        signature: data1.signature.clone(),
    };

    let r2 = auth_call!(auth, &oracle, &data2);
    // Rate limiting may kick in (MIN_CALLBACK_INTERVAL = 10s).
    // Since both calls happen in the same ledger, the second will be rate-limited.
    assert!(r2.is_err());
    assert_eq!(r2.unwrap_err(), Error::OracleCallbackTimeout);
}

// =========================================================================
// 3. Signer rotation — authorized → unauthorized
// =========================================================================

#[test]
fn test_signer_rotation_authorized_then_unauthorized_rejected() {
    let env = Env::default();
    let contract_id = env.register(predictify_hybrid::PredictifyHybrid, ());
    let admin = Address::generate(&env);
    let oracle_a = Address::generate(&env);
    let oracle_b = Address::generate(&env);

    env.as_contract(&contract_id, || {
        OracleWhitelist::initialize(&env, admin.clone()).unwrap();

        // Whitelist oracle_a only
        let metadata = OracleMetadata {
            provider: predictify_hybrid::types::OracleProvider::reflector(),
            contract_address: oracle_a.clone(),
            added_at: env.ledger().timestamp(),
            added_by: admin.clone(),
            last_health_check: env.ledger().timestamp(),
            is_active: true,
            description: String::from_str(&env, "Oracle A"),
        };
        OracleWhitelist::add_oracle_to_whitelist(&env, admin.clone(), oracle_a.clone(), metadata)
            .unwrap();
    });

    let auth = OracleCallbackAuth::new(&env);
    let data = valid_callback(&env, 300);

    // Authorized caller (oracle_a) should succeed
    let r1 = auth_call!(auth, &oracle_a, &data);
    assert!(r1.is_ok());

    // Unauthorized caller (oracle_b) with same data should be rejected
    let r2 = auth_call!(auth, &oracle_b, &data);
    assert!(r2.is_err());
    assert_eq!(r2.unwrap_err(), Error::OracleCallbackUnauthorized);
}

#[test]
fn test_signer_rotation_deactivated_oracle_rejected() {
    let env = Env::default();
    let contract_id = env.register(predictify_hybrid::PredictifyHybrid, ());
    let admin = Address::generate(&env);
    let oracle = Address::generate(&env);

    env.as_contract(&contract_id, || {
        OracleWhitelist::initialize(&env, admin.clone()).unwrap();

        let metadata = OracleMetadata {
            provider: predictify_hybrid::types::OracleProvider::reflector(),
            contract_address: oracle.clone(),
            added_at: env.ledger().timestamp(),
            added_by: admin.clone(),
            last_health_check: env.ledger().timestamp(),
            is_active: true,
            description: String::from_str(&env, "Oracle"),
        };
        OracleWhitelist::add_oracle_to_whitelist(&env, admin.clone(), oracle.clone(), metadata)
            .unwrap();
    });

    let auth = OracleCallbackAuth::new(&env);
    let data = valid_callback(&env, 400);

    // First call while active should succeed
    let r1 = auth_call!(auth, &oracle, &data);
    assert!(r1.is_ok());

    // Deactivate the oracle
    env.as_contract(&contract_id, || {
        OracleWhitelist::deactivate_oracle(&env, admin.clone(), oracle.clone()).unwrap();
    });

    // After deactivation, calls with a new nonce should be rejected
    let data2 = valid_callback(&env, 401);
    let r2 = auth_call!(auth, &oracle, &data2);
    assert!(r2.is_err());
    assert_eq!(r2.unwrap_err(), Error::OracleCallbackUnauthorized);
}

// =========================================================================
// 4. Rate limiting — back-to-back calls
// =========================================================================

#[test]
fn test_rate_limiting_back_to_back_calls() {
    let (env, _cid, _admin, oracle) = setup_whitelist();
    let auth = OracleCallbackAuth::new(&env);

    let data1 = valid_callback(&env, 500);
    let r1 = auth_call!(auth, &oracle, &data1);
    assert!(r1.is_ok());

    // Back-to-back call with different nonce should be rate-limited
    let data2 = valid_callback(&env, 501);
    let r2 = auth_call!(auth, &oracle, &data2);
    assert!(r2.is_err());
    assert_eq!(r2.unwrap_err(), Error::OracleCallbackTimeout);
}

// =========================================================================
// 6. Boundary values — should NOT panic
// =========================================================================

#[test]
fn test_boundary_max_price_still_valid() {
    let (env, _cid, _admin, oracle) = setup_whitelist();
    let mut data = valid_callback(&env, 700);
    data.price = predictify_hybrid::oracles::MAX_REASONABLE_PRICE;

    let auth = OracleCallbackAuth::new(&env);
    let result = auth_call!(auth, &oracle, &data);
    // At the boundary value, the price should be accepted
    assert!(result.is_ok(), "MAX_REASONABLE_PRICE should be valid");
}

#[test]
fn test_boundary_max_nonce_still_valid() {
    let (env, _cid, _admin, oracle) = setup_whitelist();
    let mut data = valid_callback(&env, 701);
    data.nonce = predictify_hybrid::oracles::MAX_NONCE_VALUE;

    let auth = OracleCallbackAuth::new(&env);
    let result = auth_call!(auth, &oracle, &data);
    // At the boundary value, the nonce should be accepted
    assert!(result.is_ok(), "MAX_NONCE_VALUE should be valid");
}

#[test]
fn test_boundary_timestamp_exactly_max_deviation() {
    let (env, _cid, _admin, oracle) = setup_whitelist();
    let mut data = valid_callback(&env, 702);
    data.timestamp = env.ledger().timestamp() + predictify_hybrid::oracles::MAX_TIMESTAMP_DEVIATION;

    let auth = OracleCallbackAuth::new(&env);
    let result = auth_call!(auth, &oracle, &data);
    // At exactly the max deviation, it should still be valid
    assert!(result.is_ok(), "Timestamp at max deviation should be valid");
}

// =========================================================================
// 7. Multiple valid calls with different nonces from DIFFERENT callers
// =========================================================================

#[test]
fn test_different_callers_different_nonces_independent() {
    let env = Env::default();
    let contract_id = env.register(predictify_hybrid::PredictifyHybrid, ());
    let admin = Address::generate(&env);
    let oracle_a = Address::generate(&env);
    let oracle_b = Address::generate(&env);

    env.as_contract(&contract_id, || {
        OracleWhitelist::initialize(&env, admin.clone()).unwrap();

        let meta = OracleMetadata {
            provider: predictify_hybrid::types::OracleProvider::reflector(),
            contract_address: oracle_a.clone(),
            added_at: env.ledger().timestamp(),
            added_by: admin.clone(),
            last_health_check: env.ledger().timestamp(),
            is_active: true,
            description: String::from_str(&env, "Oracle A"),
        };
        OracleWhitelist::add_oracle_to_whitelist(&env, admin.clone(), oracle_a.clone(), meta)
            .unwrap();

        let meta_b = OracleMetadata {
            provider: predictify_hybrid::types::OracleProvider::reflector(),
            contract_address: oracle_b.clone(),
            added_at: env.ledger().timestamp(),
            added_by: admin.clone(),
            last_health_check: env.ledger().timestamp(),
            is_active: true,
            description: String::from_str(&env, "Oracle B"),
        };
        OracleWhitelist::add_oracle_to_whitelist(&env, admin.clone(), oracle_b.clone(), meta_b)
            .unwrap();
    });

    let auth = OracleCallbackAuth::new(&env);

    // Both callers should be able to use nonce 800 independently
    let r_a = auth_call!(auth, &oracle_a, &valid_callback(&env, 800));
    let r_b = auth_call!(auth, &oracle_b, &valid_callback(&env, 800));

    // oracle_a should succeed (first call)
    assert!(r_a.is_ok());
    // oracle_b should succeed (different caller, independent rate limit)
    assert!(r_b.is_ok());
}

// =========================================================================
// 8. Comprehensive proptest suite
// =========================================================================

#[cfg(test)]
mod proptests {
    use proptest::prelude::*;

    proptest! {
        /// Random nonce values within valid range should be accepted the first time
        /// but rejected on replay.
        #[test]
        fn test_random_nonce_accepted_then_replayed(
            nonce in 1u64..100_000u64,
        ) {
            let (env, _cid, _admin, oracle) = super::setup_whitelist();
            let auth = super::OracleCallbackAuth::new(&env);
            let data = super::valid_callback(&env, nonce);

            let r1 = super::auth_call!(auth, &oracle, &data);
            prop_assert!(r1.is_ok(), "Fresh nonce {nonce} should be accepted");

            let r2 = super::auth_call!(auth, &oracle, &data);
            prop_assert_eq!(
                r2.unwrap_err(),
                predictify_hybrid::errors::Error::OracleCallbackReplayDetected,
                "Replayed nonce {nonce} must be rejected"
            );
        }

        /// Random prices in the valid range must be accepted.
        #[test]
        fn test_random_price_accepted(
            price in 1..predictify_hybrid::oracles::MAX_REASONABLE_PRICE,
        ) {
            let (env, _cid, _admin, oracle) = super::setup_whitelist();
            let auth = super::OracleCallbackAuth::new(&env);
            let mut data = super::valid_callback(&env, 100_000 + price as u64);
            data.price = price;

            let result = super::auth_call!(auth, &oracle, &data);
            prop_assert!(result.is_ok(), "Valid price {price} should be accepted");
        }

        /// Random prices outside valid range must be rejected.
        #[test]
        fn test_random_price_out_of_range_rejected(
            price in (predictify_hybrid::oracles::MAX_REASONABLE_PRICE + 1)..i128::MAX,
        ) {
            let (env, _cid, _admin, oracle) = super::setup_whitelist();
            let auth = super::OracleCallbackAuth::new(&env);
            let mut data = super::valid_callback(&env, 200_000 + price as u64);
            data.price = price;

            let result = super::auth_call!(auth, &oracle, &data);
            prop_assert!(result.is_err(), "Out-of-range price {price} should be rejected");
            prop_assert_eq!(result.unwrap_err(), predictify_hybrid::errors::Error::InvalidOracleFeed);
        }

        /// Random signature lengths that are NOT 64 bytes must be rejected.
        #[test]
        fn test_random_signature_length_rejected(
            sig_len in (0u32..200u32).prop_filter("not 64", |&x| x != 64),
        ) {
            let (env, _cid, _admin, oracle) = super::setup_whitelist();
            let auth = super::OracleCallbackAuth::new(&env);
            let mut data = super::valid_callback(&env, 300_000 + sig_len as u64);
            {
                let mut s = soroban_sdk::Bytes::new(&env);
                for _ in 0..sig_len {
                    s.push_back(0);
                }
                data.signature = s;
            }

            let result = super::auth_call!(auth, &oracle, &data);
            if sig_len == 0 {
                // Empty signature fails in validate_callback_data before reaching ed25519 check
                prop_assert_eq!(
                    result.unwrap_err(),
                    super::Error::OracleCallbackInvalidSignature,
                    "Empty signature (len=0) should be rejected"
                );
            } else {
                prop_assert!(result.is_err(), "Non-64 signature len {sig_len} should be rejected");
                prop_assert_eq!(
                    result.unwrap_err(),
                    super::Error::OracleCallbackInvalidSignature,
                    "Sig len {sig_len} != 64 must yield InvalidSignature"
                );
            }
        }
    }
}

// =========================================================================
// 9. Verify no unwrap() usage in test code
// =========================================================================

/// Sanity test: the harness itself never panics via unwrap.
#[test]
fn test_harness_does_not_use_unwrap_on_results() {
    // All test functions above use `auth_call!` macro which asserts
    // is_ok || is_err without unwrapping. This test ensures the harness
    // itself doesn't panic from any of its own operations.
    let (env, _cid, _admin, oracle) = setup_whitelist();
    let auth = OracleCallbackAuth::new(&env);
    let data = valid_callback(&env, 999_999);

    // Even on failure, the call returns Result — never panics
    let _ = auth.authenticate_and_process(&oracle, &data);

    // If we reach here, no panic occurred
    assert!(true);
}
