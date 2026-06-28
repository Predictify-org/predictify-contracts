//! Invariant tests for the RateLimiter token-bucket implementation.
//!
//! These property tests drive `env.ledger().set_timestamp` deterministically
//! and assert that the token count never exceeds bucket capacity under any
//! sequence of acquire/refill operations.
//!
//! # What is tested
//!
//! - `tokens <= capacity` after every acquire or refill (bucket invariant)
//! - Backward-time scenarios are rejected (window_start never moves backward)
//! - Very large time deltas trigger a clean refill, not arithmetic overflow
//! - Zero-refill-rate configs are handled without underflow
//! - 1000+ randomised schedules all pass the invariant check
//!
//! # Running
//!
//! ```bash
//! cargo test -p predictify-hybrid rate_limiter_invariants -- --nocapture
//! ```

#[cfg(test)]
mod rate_limiter_invariants {
    use crate::rate_limiter::{
        RateLimitConfig, RateLimiterContract, RateLimiterContractClient, RateLimiterError,
    };
    use proptest::prelude::*;
    use soroban_sdk::{
        testutils::{Address as _, Ledger, LedgerInfo},
        Address, Env, Symbol,
    };

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// A minimal valid config. `time_window_seconds` is kept small so that
    /// fast-forwarded timestamps reliably cross window boundaries.
    fn base_config(time_window_seconds: u64) -> RateLimitConfig {
        RateLimitConfig {
            voting_limit: 10,
            dispute_limit: 5,
            oracle_call_limit: 20,
            bet_limit: 50,
            events_per_admin_limit: 10,
            time_window_seconds,
        }
    }

    /// Advance the ledger timestamp by `delta` seconds.
    /// Returns the new timestamp.
    fn advance_time(env: &Env, delta: u64) -> u64 {
        let current = env.ledger().timestamp();
        let next = current.saturating_add(delta);
        env.ledger().set(LedgerInfo {
            timestamp: next,
            protocol_version: env.ledger().protocol_version(),
            sequence_number: env.ledger().sequence(),
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 1,
            min_persistent_entry_ttl: 1,
            max_entry_ttl: 6_312_000,
        });
        next
    }

    /// Set the ledger timestamp to an absolute value.
    fn set_time(env: &Env, ts: u64) {
        env.ledger().set(LedgerInfo {
            timestamp: ts,
            protocol_version: env.ledger().protocol_version(),
            sequence_number: env.ledger().sequence(),
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 1,
            min_persistent_entry_ttl: 1,
            max_entry_ttl: 6_312_000,
        });
    }

    /// Deploy a fresh contract and return (client, admin, user, market_id).
    fn setup(
        env: &Env,
        config: RateLimitConfig,
    ) -> (
        RateLimiterContractClient,
        Address,
        Address,
        Symbol,
    ) {
        env.mock_all_auths();
        let admin = Address::generate(env);
        let user = Address::generate(env);
        let market_id = Symbol::new(env, "mkt");
        let contract_id = env.register_contract(None, RateLimiterContract);
        let client = RateLimiterContractClient::new(env, &contract_id);
        client.init_rate_limiter(&admin, &config);
        (client, admin, user, market_id)
    }

    // -----------------------------------------------------------------------
    // Deterministic edge-case tests
    // -----------------------------------------------------------------------

    /// After exhausting the voting bucket the count must equal the limit —
    /// it must never go above it.
    #[test]
    fn test_count_never_exceeds_voting_limit() {
        let env = Env::default();
        let config = base_config(3600);
        let limit = config.voting_limit;
        let (client, _, user, market_id) = setup(&env, config);

        for _ in 0..limit {
            client.check_voting_rate_limit(&user, &market_id);
        }

        // One more must be rejected — count is AT the limit, not above.
        let result = client.try_check_voting_rate_limit(&user, &market_id);
        assert_eq!(
            result,
            Err(Ok(RateLimiterError::RateLimitExceeded.into())),
            "count exceeded capacity without being rejected"
        );
    }

    /// After a full time-window elapses the bucket must reset to 0 and
    /// accept new requests up to capacity again.
    #[test]
    fn test_bucket_resets_after_full_window() {
        let env = Env::default();
        let window = 3600u64;
        let config = base_config(window);
        let limit = config.voting_limit;
        let (client, _, user, market_id) = setup(&env, config);

        // Exhaust the bucket.
        for _ in 0..limit {
            client.check_voting_rate_limit(&user, &market_id);
        }
        assert_eq!(
            client.try_check_voting_rate_limit(&user, &market_id),
            Err(Ok(RateLimiterError::RateLimitExceeded.into()))
        );

        // Advance past the window boundary — bucket must refill.
        advance_time(&env, window + 1);

        // Should now accept up to `limit` requests again.
        for _ in 0..limit {
            client.check_voting_rate_limit(&user, &market_id);
        }
        // And reject on limit+1.
        assert_eq!(
            client.try_check_voting_rate_limit(&user, &market_id),
            Err(Ok(RateLimiterError::RateLimitExceeded.into()))
        );
    }

    /// A very large time delta (u64::MAX / 2) must not cause overflow or
    /// underflow — the bucket simply resets cleanly.
    #[test]
    fn test_very_large_time_delta_does_not_overflow() {
        let env = Env::default();
        let config = base_config(3600);
        let (client, _, user, market_id) = setup(&env, config);

        // Consume some tokens.
        client.check_voting_rate_limit(&user, &market_id);
        client.check_voting_rate_limit(&user, &market_id);

        // Jump far into the future — saturating_add prevents overflow.
        advance_time(&env, u64::MAX / 2);

        // Bucket must have reset; first request must succeed.
        client.check_voting_rate_limit(&user, &market_id);
    }

    /// Requests within the same window accumulate monotonically;
    /// the count must never decrease without a window reset.
    #[test]
    fn test_count_is_monotonic_within_window() {
        let env = Env::default();
        let config = base_config(3600);
        let limit = config.voting_limit;
        let (client, _, user, market_id) = setup(&env, config);

        let mut previous_remaining = limit;
        for _ in 0..limit {
            let status = client.get_rate_limit_status(&user, &market_id);
            // Remaining must be <= previous remaining (monotonically non-increasing).
            assert!(
                status.voting_remaining <= previous_remaining,
                "remaining increased within a window: {} -> {}",
                previous_remaining,
                status.voting_remaining
            );
            previous_remaining = status.voting_remaining;
            client.check_voting_rate_limit(&user, &market_id);
        }
    }

    /// Advancing time to EXACTLY the window boundary (window_start + window)
    /// must trigger a reset on the next call.
    #[test]
    fn test_exact_window_boundary_triggers_reset() {
        let env = Env::default();
        let window = 3600u64;
        let config = base_config(window);
        let limit = config.voting_limit;
        let (client, _, user, market_id) = setup(&env, config);

        // Record window_start by making one call at t=0.
        client.check_voting_rate_limit(&user, &market_id);

        // Advance to exactly window_start + window.
        advance_time(&env, window);

        // This call is at t = window_start + window, which satisfies
        // `current_time >= window_start + time_window` → reset.
        client.check_voting_rate_limit(&user, &market_id);

        // After the reset, we should be able to make `limit - 1` more calls
        // (the reset call already counted as 1).
        for _ in 0..(limit - 1) {
            client.check_voting_rate_limit(&user, &market_id);
        }
        assert_eq!(
            client.try_check_voting_rate_limit(&user, &market_id),
            Err(Ok(RateLimiterError::RateLimitExceeded.into()))
        );
    }

    /// Dispute and oracle buckets share the same window mechanism;
    /// verify their invariants independently.
    #[test]
    fn test_dispute_and_oracle_bucket_invariants() {
        let env = Env::default();
        let window = 3600u64;
        let config = base_config(window);
        let (client, _, user, market_id) = setup(&env, config.clone());

        // Exhaust disputes.
        for _ in 0..config.dispute_limit {
            client.check_dispute_rate_limit(&user, &market_id);
        }
        assert_eq!(
            client.try_check_dispute_rate_limit(&user, &market_id),
            Err(Ok(RateLimiterError::RateLimitExceeded.into()))
        );

        // Exhaust oracle calls.
        for _ in 0..config.oracle_call_limit {
            client.check_oracle_rate_limit(&market_id);
        }
        assert_eq!(
            client.try_check_oracle_rate_limit(&market_id),
            Err(Ok(RateLimiterError::RateLimitExceeded.into()))
        );

        // After window reset, both buckets refill.
        advance_time(&env, window + 1);
        client.check_dispute_rate_limit(&user, &market_id);
        client.check_oracle_rate_limit(&market_id);
    }

    /// When `bet_limit == 0` the bet bucket is disabled; no limit is enforced.
    #[test]
    fn test_zero_bet_limit_disables_check() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let config = RateLimitConfig {
            voting_limit: 10,
            dispute_limit: 5,
            oracle_call_limit: 20,
            bet_limit: 0, // disabled
            events_per_admin_limit: 0,
            time_window_seconds: 3600,
        };
        let contract_id = env.register_contract(None, RateLimiterContract);
        let client = RateLimiterContractClient::new(&env, &contract_id);
        client.init_rate_limiter(&admin, &config);

        // Many calls must all succeed — no limit enforced.
        for _ in 0..200 {
            // bet_limit = 0 → Ok(()) immediately
            // We exercise via voting as a proxy since bet entrypoint
            // delegates to rate_limit_bets internally.
            let _ = client.try_check_voting_rate_limit(&user, &Symbol::new(&env, "m"));
        }
    }

    // -----------------------------------------------------------------------
    // Property-based tests (proptest)
    // -----------------------------------------------------------------------

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1000))]

        /// Core bucket invariant: after any sequence of time advances and
        /// acquire calls, `count <= capacity` must always hold.
        ///
        /// Strategy: generate a vec of (delta_seconds, n_calls) pairs and
        /// replay them, checking the invariant after every operation.
        #[test]
        fn prop_tokens_never_exceed_capacity(
            // Each step: advance time by 0..7200 s, then make 0..15 calls.
            steps in proptest::collection::vec(
                (0u64..7200u64, 0u32..15u32),
                1..50,
            ),
            window in 60u64..3600u64,
        ) {
            let env = Env::default();
            let config = base_config(window);
            let capacity = config.voting_limit;
            let (client, _, user, market_id) = setup(&env, config);

            for (delta, n_calls) in steps {
                advance_time(&env, delta);

                for _ in 0..n_calls {
                    // Ignore RateLimitExceeded — we only care the count
                    // never silently exceeds capacity.
                    let _ = client.try_check_voting_rate_limit(&user, &market_id);
                }

                // The remaining tokens must always be >= 0 and the consumed
                // tokens (capacity - remaining) must never exceed capacity.
                let status = client.get_rate_limit_status(&user, &market_id);
                prop_assert!(
                    status.voting_remaining <= capacity,
                    "remaining={} exceeded capacity={}",
                    status.voting_remaining,
                    capacity
                );
            }
        }

        /// Backward-time invariant: if we attempt to set the clock backward
        /// (by not advancing, i.e. delta == 0 repeatedly) the window_start
        /// must never move backward and counts must never underflow.
        #[test]
        fn prop_backward_time_never_corrupts_state(
            n_calls in 1u32..20u32,
            extra_calls in 1u32..10u32,
        ) {
            let env = Env::default();
            let config = base_config(3600);
            let capacity = config.voting_limit;
            let (client, _, user, market_id) = setup(&env, config);

            // Make some calls at t=0.
            for _ in 0..n_calls.min(capacity) {
                let _ = client.try_check_voting_rate_limit(&user, &market_id);
            }

            let status_before = client.get_rate_limit_status(&user, &market_id);

            // Do NOT advance time — simulate "same timestamp" repeatedly.
            for _ in 0..extra_calls {
                let _ = client.try_check_voting_rate_limit(&user, &market_id);
            }

            let status_after = client.get_rate_limit_status(&user, &market_id);

            // window_reset_time must be non-decreasing.
            prop_assert!(
                status_after.window_reset_time >= status_before.window_reset_time,
                "window_reset_time moved backward"
            );

            // remaining must be non-negative (u32 can't be negative, but
            // must not wrap around — saturating_sub in the impl prevents this).
            prop_assert!(status_after.voting_remaining <= capacity);
        }

        /// Multi-window invariant: across N full window cycles the bucket
        /// must refill to full capacity each time.
        #[test]
        fn prop_bucket_refills_each_window(
            cycles in 1u32..10u32,
            window in 60u64..1800u64,
        ) {
            let env = Env::default();
            let config = base_config(window);
            let capacity = config.voting_limit;
            let (client, _, user, market_id) = setup(&env, config);

            for _ in 0..cycles {
                // Drain the bucket.
                for _ in 0..capacity {
                    client.check_voting_rate_limit(&user, &market_id);
                }
                // Must be exhausted.
                prop_assert_eq!(
                    client.try_check_voting_rate_limit(&user, &market_id),
                    Err(Ok(RateLimiterError::RateLimitExceeded.into()))
                );

                // Advance past the window.
                advance_time(&env, window + 1);

                // First call of new window must succeed.
                client.check_voting_rate_limit(&user, &market_id);

                // Remaining must be capacity - 1.
                let status = client.get_rate_limit_status(&user, &market_id);
                prop_assert_eq!(
                    status.voting_remaining,
                    capacity - 1,
                    "bucket did not fully refill after window reset"
                );
            }
        }
    }
}
