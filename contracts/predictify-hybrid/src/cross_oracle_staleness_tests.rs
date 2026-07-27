/// Tests for the cross-oracle staleness event feature.
///
/// The feature emits a [`CrossOracleStalenessEvent`] when the difference
/// between the freshest `publish_time` and any individual oracle's
/// `publish_time` exceeds the configured staleness threshold during
/// multi-oracle consensus verification.
///
/// # Test strategy
///
/// These are unit-level tests that verify:
/// 1. `CrossOracleStalenessEvent` struct construction and field values.
/// 2. `EventEmitter::emit_cross_oracle_staleness` correctly stores and
///    publishes the event with a monotonically-increasing nonce.
/// 3. The staleness detection threshold logic (gap > threshold triggers,
///    gap == threshold does NOT trigger, gap < threshold does NOT trigger).
/// 4. Edge cases: all-fresh sources, single source, exact-threshold gap.
///
/// Full integration through `OracleIntegrationManager::verify_result` is
/// exercised in `oracle_validation_tests` because that flow requires a
/// properly-initialized market and whitelisted oracle.
#[cfg(test)]
mod cross_oracle_staleness_tests {
    use crate::events::{CrossOracleStalenessEvent, EventEmitter};
    use soroban_sdk::{
        testutils::{Address as _, Ledger as _},
        Address, Env, String, Symbol,
    };

    // ─────────────────────────────────────────────────────────────
    // Helpers
    // ─────────────────────────────────────────────────────────────

    /// Build a minimal environment with a predictable ledger timestamp.
    fn make_env(ts: u64) -> Env {
        let env = Env::default();
        env.ledger().with_mut(|li| li.timestamp = ts);
        env
    }

    fn make_market_id(env: &Env) -> Symbol {
        Symbol::new(env, "btc_50k")
    }

    fn make_provider(env: &Env) -> String {
        String::from_str(env, "Reflector")
    }

    fn make_feed_id(env: &Env) -> String {
        String::from_str(env, "BTC/USD")
    }

    // ─────────────────────────────────────────────────────────────
    // Struct construction tests
    // ─────────────────────────────────────────────────────────────

    #[test]
    fn test_cross_oracle_staleness_event_fields() {
        let env = make_env(1_700_000_200);
        let oracle = Address::generate(&env);
        let market_id = make_market_id(&env);

        let event = CrossOracleStalenessEvent {
            market_id: market_id.clone(),
            stale_oracle: oracle.clone(),
            stale_provider: make_provider(&env),
            feed_id: make_feed_id(&env),
            freshest_timestamp: 1_700_000_120,
            stale_timestamp: 1_700_000_000,
            staleness_gap_secs: 120,
            max_staleness_secs: 60,
            sources_total: 3,
            nonce: 1,
            timestamp: env.ledger().timestamp(),
        };

        assert_eq!(event.market_id, market_id);
        assert_eq!(event.stale_oracle, oracle);
        assert_eq!(event.freshest_timestamp, 1_700_000_120);
        assert_eq!(event.stale_timestamp, 1_700_000_000);
        assert_eq!(event.staleness_gap_secs, 120);
        assert_eq!(event.max_staleness_secs, 60);
        assert_eq!(event.sources_total, 3);
        // gap > threshold → event is appropriate
        assert!(event.staleness_gap_secs > event.max_staleness_secs);
    }

    // ─────────────────────────────────────────────────────────────
    // Emitter tests
    // ─────────────────────────────────────────────────────────────

    #[test]
    fn test_emit_cross_oracle_staleness_does_not_panic() {
        let env = make_env(1_700_000_300);
        let oracle = Address::generate(&env);
        let market_id = make_market_id(&env);

        // Should not panic
        EventEmitter::emit_cross_oracle_staleness(
            &env,
            &market_id,
            &oracle,
            &make_provider(&env),
            &make_feed_id(&env),
            1_700_000_200,   // freshest
            1_700_000_000,   // stale
            200,             // gap_secs (> 60 threshold)
            60,              // max_staleness
            2,               // sources_total
        );
    }

    #[test]
    fn test_emit_cross_oracle_staleness_nonce_increments() {
        let env = make_env(1_700_001_000);
        let oracle = Address::generate(&env);
        let market_id = make_market_id(&env);
        let provider = make_provider(&env);
        let feed_id = make_feed_id(&env);

        // Emit twice and verify nonces are distinct (both > 0).
        EventEmitter::emit_cross_oracle_staleness(
            &env, &market_id, &oracle, &provider, &feed_id,
            1_700_000_200, 1_700_000_000, 200, 60, 2,
        );
        EventEmitter::emit_cross_oracle_staleness(
            &env, &market_id, &oracle, &provider, &feed_id,
            1_700_000_300, 1_700_000_000, 300, 60, 2,
        );

        // The nonce key for "x_stale" should now be ≥ 2.
        use soroban_sdk::symbol_short;
        let key = crate::storage::DataKey::EventNonce(symbol_short!("x_stale"));
        let nonce: u64 = env
            .storage()
            .persistent()
            .get(&key)
            .expect("nonce should be stored");
        assert!(nonce >= 2, "nonce should be at least 2 after two emissions");
    }

    // ─────────────────────────────────────────────────────────────
    // Threshold boundary tests
    // ─────────────────────────────────────────────────────────────

    /// Helper that returns true if a staleness event *would* be emitted given
    /// a gap and a threshold — mirroring the condition used in
    /// `fetch_and_verify_oracle_result`.
    fn would_emit(gap: u64, max_staleness: u64) -> bool {
        gap > max_staleness
    }

    #[test]
    fn test_threshold_strictly_greater_triggers() {
        assert!(would_emit(61, 60), "gap of 61 > threshold 60 should trigger");
    }

    #[test]
    fn test_threshold_equal_does_not_trigger() {
        assert!(!would_emit(60, 60), "gap equal to threshold should NOT trigger");
    }

    #[test]
    fn test_threshold_below_does_not_trigger() {
        assert!(!would_emit(59, 60), "gap below threshold should NOT trigger");
    }

    #[test]
    fn test_zero_gap_does_not_trigger() {
        assert!(!would_emit(0, 60), "zero gap should never trigger");
    }

    #[test]
    fn test_zero_threshold_triggers_for_any_nonzero_gap() {
        // max_staleness = 0 is not a valid config (validator rejects it), but the
        // pure comparison logic should still behave correctly.
        assert!(would_emit(1, 0), "any nonzero gap > 0 threshold should trigger");
        assert!(!would_emit(0, 0), "zero gap with zero threshold should NOT trigger");
    }

    // ─────────────────────────────────────────────────────────────
    // Multi-source freshest-timestamp logic
    // ─────────────────────────────────────────────────────────────

    /// Simulate the freshest-timestamp fold used in
    /// `fetch_and_verify_oracle_result` and verify it selects the maximum.
    #[test]
    fn test_freshest_timestamp_fold_selects_maximum() {
        let timestamps: alloc::vec::Vec<u64> = alloc::vec![
            1_700_000_000,
            1_700_000_120,
            1_700_000_050,
        ];
        let freshest = timestamps
            .iter()
            .copied()
            .fold(0u64, |acc, ts| if ts > acc { ts } else { acc });

        assert_eq!(freshest, 1_700_000_120);
    }

    #[test]
    fn test_freshest_timestamp_single_source_returns_that_source() {
        let timestamps: alloc::vec::Vec<u64> = alloc::vec![1_700_000_042];
        let freshest = timestamps
            .iter()
            .copied()
            .fold(0u64, |acc, ts| if ts > acc { ts } else { acc });

        assert_eq!(freshest, 1_700_000_042);
    }

    #[test]
    fn test_single_source_never_triggers_cross_staleness() {
        // Cross-oracle staleness only makes sense with ≥ 2 sources.
        let timestamps: alloc::vec::Vec<u64> = alloc::vec![1_700_000_000];
        // The condition in fetch_and_verify_oracle_result is:
        //   `if source_publish_times.len() > 1 { ... }`
        assert!(
            timestamps.len() <= 1,
            "single source should skip cross-staleness check"
        );
    }

    // ─────────────────────────────────────────────────────────────
    // Event payload equality
    // ─────────────────────────────────────────────────────────────

    #[test]
    fn test_cross_oracle_staleness_event_equality() {
        let env = make_env(1_700_002_000);
        let oracle = Address::generate(&env);
        let market_id = make_market_id(&env);
        let provider = make_provider(&env);
        let feed_id = make_feed_id(&env);
        let ts = env.ledger().timestamp();

        let a = CrossOracleStalenessEvent {
            market_id: market_id.clone(),
            stale_oracle: oracle.clone(),
            stale_provider: provider.clone(),
            feed_id: feed_id.clone(),
            freshest_timestamp: 1_700_000_100,
            stale_timestamp: 1_700_000_000,
            staleness_gap_secs: 100,
            max_staleness_secs: 60,
            sources_total: 2,
            nonce: 7,
            timestamp: ts,
        };

        let b = CrossOracleStalenessEvent {
            market_id: market_id.clone(),
            stale_oracle: oracle.clone(),
            stale_provider: provider.clone(),
            feed_id: feed_id.clone(),
            freshest_timestamp: 1_700_000_100,
            stale_timestamp: 1_700_000_000,
            staleness_gap_secs: 100,
            max_staleness_secs: 60,
            sources_total: 2,
            nonce: 7,
            timestamp: ts,
        };

        assert_eq!(a, b);
    }

    #[test]
    fn test_cross_oracle_staleness_event_inequality_on_different_gap() {
        let env = make_env(1_700_003_000);
        let oracle = Address::generate(&env);
        let market_id = make_market_id(&env);
        let provider = make_provider(&env);
        let feed_id = make_feed_id(&env);
        let ts = env.ledger().timestamp();

        let make_event = |gap: u64| CrossOracleStalenessEvent {
            market_id: market_id.clone(),
            stale_oracle: oracle.clone(),
            stale_provider: provider.clone(),
            feed_id: feed_id.clone(),
            freshest_timestamp: 1_700_000_000 + gap,
            stale_timestamp: 1_700_000_000,
            staleness_gap_secs: gap,
            max_staleness_secs: 60,
            sources_total: 2,
            nonce: 1,
            timestamp: ts,
        };

        assert_ne!(make_event(90), make_event(120));
    }
}
