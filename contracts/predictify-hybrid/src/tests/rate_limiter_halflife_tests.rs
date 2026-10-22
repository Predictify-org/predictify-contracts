#![cfg(test)]

//! Half-life rate-limiter tests (`rate_limiter_halflife`).
//!
//! These tests exercise the `RefillMode::HalfLife` token-bucket implementation.
//!
//! # What is tested
//!
//! - Basic capacity enforcement: a full bucket blocks the next action
//! - Monotone recovery: available tokens increase (never decrease) as time passes
//! - Asymptotic saturation: after many half-lives the bucket is fully available
//! - Saturation is exact — no overflow past capacity
//! - Half-life zero is rejected by the validator
//! - Half-life > time_window is rejected by the validator
//! - Multiple users / markets have independent buckets
//! - HalfLife and Linear configs are interchangeable at the config level
//! - Edge case: 31+ half-lives saturate the bucket via shift-saturation
//! - Edge case: time doesn't move (elapsed=0) — no phantom refill
//!
//! # Running
//!
//! ```bash
//! cargo test -p predictify-hybrid halflife -- --nocapture
//! ```

use crate::rate_limiter::{
    RateLimitConfig, RateLimiterContract, RateLimiterContractClient, RateLimiterError, RefillMode,
};
use soroban_sdk::{testutils::Address as _, Address, Env, Symbol};

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Construct a HalfLife config with small values for fast-forwarding in tests.
///
/// - `capacity` is used as `voting_limit`; other limits are set large so they
///   never interfere with voting-focused tests.
/// - `half_life_secs` is the half-life period.
/// - `time_window_seconds` is set to `half_life_secs * 32` so the HalfLife
///   validation rule (half_life ≤ time_window) always passes.
fn halflife_config(capacity: u32, half_life_secs: u64) -> RateLimitConfig {
    // time_window must be >= half_life for validation to pass.
    let time_window = half_life_secs * 32;
    // Clamp to the allowed max (2 592 000 s = 30 days) if needed.
    let time_window = if time_window < 60 { 3600 } else { time_window };
    let time_window = time_window.min(2_592_000);
    RateLimitConfig {
        voting_limit: capacity,
        dispute_limit: 1000,
        oracle_call_limit: 1000,
        bet_limit: 0,
        events_per_admin_limit: 0,
        time_window_seconds: time_window,
        refill_mode: RefillMode::HalfLife(half_life_secs),
    }
}

/// Deploy the standalone `RateLimiterContract` and return a client pre-seeded
/// with the given config.
fn deploy(env: &Env, config: RateLimitConfig) -> RateLimiterContractClient {
    env.mock_all_auths();
    let contract_id = env.register_contract(None, RateLimiterContract);
    let client = RateLimiterContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.init_rate_limiter(&admin, &config);
    client
}

/// Advance the ledger timestamp by `delta` seconds.
fn advance(env: &Env, delta: u64) {
    let ts = env.ledger().timestamp();
    env.ledger().with_mut(|li| li.timestamp = ts.saturating_add(delta));
}

/// Set the ledger timestamp to an absolute value.
fn set_time(env: &Env, ts: u64) {
    env.ledger().with_mut(|li| li.timestamp = ts);
}

// ─────────────────────────────────────────────────────────────────────────────
// Basic enforcement
// ─────────────────────────────────────────────────────────────────────────────

/// A bucket with capacity 3 should allow 3 actions then block the 4th.
#[test]
fn halflife_bucket_blocks_after_capacity_exhausted() {
    let env = Env::default();
    env.mock_all_auths();
    set_time(&env, 1_000);

    // capacity=3, half_life=300 s
    let client = deploy(&env, halflife_config(3, 300));
    let user = Address::generate(&env);
    let mkt = Symbol::new(&env, "market");

    // 3 actions succeed
    for i in 0..3 {
        assert!(
            client.try_check_voting_rate_limit(&user, &mkt).is_ok(),
            "action {} should succeed",
            i + 1
        );
    }

    // 4th action must fail
    let result = client.try_check_voting_rate_limit(&user, &mkt);
    assert_eq!(
        result,
        Err(Ok(RateLimiterError::RateLimitExceeded)),
        "4th action should be blocked"
    );
}

/// After one half-life half the consumed tokens are returned,
/// so a bucket that was blocked can accept new actions again.
#[test]
fn halflife_refill_after_one_half_life() {
    let env = Env::default();
    env.mock_all_auths();
    set_time(&env, 0);

    // capacity=4, half_life=100 s
    let client = deploy(&env, halflife_config(4, 100));
    let user = Address::generate(&env);
    let mkt = Symbol::new(&env, "market");

    // Exhaust the bucket (4 actions)
    for _ in 0..4 {
        client.check_voting_rate_limit(&user, &mkt);
    }

    // Should be blocked
    assert_eq!(
        client.try_check_voting_rate_limit(&user, &mkt),
        Err(Ok(RateLimiterError::RateLimitExceeded)),
        "bucket should be exhausted"
    );

    // Advance exactly one half-life — used tokens halve from 4 → 2.
    // Available = 4 - 2 = 2 → should unblock.
    advance(&env, 100);

    assert!(
        client.try_check_voting_rate_limit(&user, &mkt).is_ok(),
        "should accept after one half-life"
    );
}

/// After many half-lives the bucket should be (effectively) fully available.
#[test]
fn halflife_bucket_fully_saturates_after_many_half_lives() {
    let env = Env::default();
    env.mock_all_auths();
    set_time(&env, 0);

    // capacity=8, half_life=60 s
    let client = deploy(&env, halflife_config(8, 60));
    let user = Address::generate(&env);
    let mkt = Symbol::new(&env, "market");

    // Exhaust the bucket
    for _ in 0..8 {
        client.check_voting_rate_limit(&user, &mkt);
    }

    // After 32 half-lives (32 × 60 = 1920 s) any remaining used tokens
    // will be shifted to 0 (u32::MAX >> 32 saturates to 0).
    advance(&env, 60 * 32);

    // All 8 slots should be available again
    for i in 0..8 {
        assert!(
            client.try_check_voting_rate_limit(&user, &mkt).is_ok(),
            "slot {} should be available after full saturation",
            i + 1
        );
    }
}

/// Verify that available token count never exceeds capacity (no overflow).
#[test]
fn halflife_no_overflow_past_capacity() {
    let env = Env::default();
    env.mock_all_auths();
    set_time(&env, 0);

    let capacity = 5u32;
    // half_life=60 s
    let client = deploy(&env, halflife_config(capacity, 60));
    let user = Address::generate(&env);
    let mkt = Symbol::new(&env, "market");

    // Use one action
    client.check_voting_rate_limit(&user, &mkt);

    // Advance far into the future
    advance(&env, 60 * 100);

    // Use one more — should not panic, must succeed
    assert!(client.try_check_voting_rate_limit(&user, &mkt).is_ok());

    // Check status: remaining must be ≤ capacity
    let status = client.get_rate_limit_status(&user, &mkt);
    assert!(
        status.voting_remaining <= capacity,
        "remaining {} must not exceed capacity {}",
        status.voting_remaining,
        capacity
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Monotone recovery
// ─────────────────────────────────────────────────────────────────────────────

/// `available` must be monotone non-decreasing as time passes.
#[test]
fn halflife_available_monotone_non_decreasing() {
    let env = Env::default();
    env.mock_all_auths();
    set_time(&env, 0);

    let capacity = 10u32;
    let half_life = 100u64;
    let client = deploy(&env, halflife_config(capacity, half_life));
    let user = Address::generate(&env);
    let mkt = Symbol::new(&env, "market");

    // Exhaust the bucket
    for _ in 0..capacity {
        client.check_voting_rate_limit(&user, &mkt);
    }

    let mut prev_remaining = 0u32;
    // Sample at increasing time offsets
    for &elapsed in &[0u64, 25, 50, 75, 100, 150, 200, 300, 500, 1000, 3200] {
        set_time(&env, elapsed);
        let status = client.get_rate_limit_status(&user, &mkt);
        assert!(
            status.voting_remaining >= prev_remaining,
            "remaining decreased from {} to {} at elapsed={}",
            prev_remaining,
            status.voting_remaining,
            elapsed
        );
        prev_remaining = status.voting_remaining;
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Independence: different users / markets
// ─────────────────────────────────────────────────────────────────────────────

/// Two different users should have completely independent buckets.
#[test]
fn halflife_different_users_are_independent() {
    let env = Env::default();
    env.mock_all_auths();
    set_time(&env, 0);

    let client = deploy(&env, halflife_config(2, 60));
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let mkt = Symbol::new(&env, "market");

    // Exhaust user1's bucket
    for _ in 0..2 {
        client.check_voting_rate_limit(&user1, &mkt);
    }
    assert_eq!(
        client.try_check_voting_rate_limit(&user1, &mkt),
        Err(Ok(RateLimiterError::RateLimitExceeded))
    );

    // user2 is untouched — must still be free
    assert!(
        client.try_check_voting_rate_limit(&user2, &mkt).is_ok(),
        "user2 bucket must be independent"
    );
}

/// Two different markets should have completely independent buckets.
#[test]
fn halflife_different_markets_are_independent() {
    let env = Env::default();
    env.mock_all_auths();
    set_time(&env, 0);

    let client = deploy(&env, halflife_config(2, 60));
    let user = Address::generate(&env);
    let mkt1 = Symbol::new(&env, "mktA");
    let mkt2 = Symbol::new(&env, "mktB");

    // Exhaust mkt1
    for _ in 0..2 {
        client.check_voting_rate_limit(&user, &mkt1);
    }
    assert_eq!(
        client.try_check_voting_rate_limit(&user, &mkt1),
        Err(Ok(RateLimiterError::RateLimitExceeded))
    );

    // mkt2 bucket is independent
    assert!(
        client.try_check_voting_rate_limit(&user, &mkt2).is_ok(),
        "mkt2 bucket must be independent"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Validation
// ─────────────────────────────────────────────────────────────────────────────

/// `half_life_seconds = 0` must be rejected.
#[test]
fn halflife_zero_half_life_rejected_by_validator() {
    let env = Env::default();
    let config = RateLimitConfig {
        voting_limit: 10,
        dispute_limit: 5,
        oracle_call_limit: 20,
        bet_limit: 0,
        events_per_admin_limit: 0,
        time_window_seconds: 3600,
        refill_mode: RefillMode::HalfLife(0), // invalid
    };
    let result = RateLimiterContract::validate_rate_limit_config(env.clone(), config);
    assert_eq!(result, Err(RateLimiterError::InvalidHalfLife));
}

/// `half_life_seconds > time_window_seconds` must be rejected.
#[test]
fn halflife_greater_than_time_window_rejected_by_validator() {
    let env = Env::default();
    let config = RateLimitConfig {
        voting_limit: 10,
        dispute_limit: 5,
        oracle_call_limit: 20,
        bet_limit: 0,
        events_per_admin_limit: 0,
        time_window_seconds: 3600,
        refill_mode: RefillMode::HalfLife(7200), // > time_window
    };
    let result = RateLimiterContract::validate_rate_limit_config(env.clone(), config);
    assert_eq!(result, Err(RateLimiterError::InvalidHalfLife));
}

/// A valid HalfLife config must pass validation.
#[test]
fn halflife_valid_config_passes_validation() {
    let env = Env::default();
    let config = halflife_config(10, 60);
    let result = RateLimiterContract::validate_rate_limit_config(env.clone(), config);
    assert!(result.is_ok(), "valid HalfLife config should pass");
}

// ─────────────────────────────────────────────────────────────────────────────
// Edge-cases
// ─────────────────────────────────────────────────────────────────────────────

/// When time does not advance (elapsed = 0) no phantom refill occurs.
#[test]
fn halflife_no_refill_when_time_static() {
    let env = Env::default();
    env.mock_all_auths();
    set_time(&env, 1_000);

    let client = deploy(&env, halflife_config(2, 60));
    let user = Address::generate(&env);
    let mkt = Symbol::new(&env, "market");

    // Fill the bucket
    client.check_voting_rate_limit(&user, &mkt);
    client.check_voting_rate_limit(&user, &mkt);

    // Same timestamp — no time has elapsed
    assert_eq!(
        client.try_check_voting_rate_limit(&user, &mkt),
        Err(Ok(RateLimiterError::RateLimitExceeded)),
        "no refill should occur when time hasn't advanced"
    );
}

/// After 31+ half-lives the right-shift saturates to 0 used tokens (full bucket).
#[test]
fn halflife_shift_saturation_at_31_or_more_half_lives() {
    let env = Env::default();
    env.mock_all_auths();
    set_time(&env, 0);

    // capacity=4, half_life=60 s  →  time_window=60*32=1920 s
    let client = deploy(&env, halflife_config(4, 60));
    let user = Address::generate(&env);
    let mkt = Symbol::new(&env, "market");

    // Exhaust bucket
    for _ in 0..4 {
        client.check_voting_rate_limit(&user, &mkt);
    }

    // Advance 32 half-lives
    advance(&env, 60 * 32);

    let status = client.get_rate_limit_status(&user, &mkt);
    assert_eq!(
        status.voting_remaining, 4,
        "after 32+ half-lives all 4 tokens should be available"
    );
}

/// A HalfLife bucket with capacity=1 blocks immediately and recovers after one half-life.
#[test]
fn halflife_capacity_one_blocks_and_recovers() {
    let env = Env::default();
    env.mock_all_auths();
    set_time(&env, 0);

    let client = deploy(&env, halflife_config(1, 120));
    let user = Address::generate(&env);
    let mkt = Symbol::new(&env, "market");

    // First action succeeds
    assert!(client.try_check_voting_rate_limit(&user, &mkt).is_ok());

    // Immediately blocked
    assert_eq!(
        client.try_check_voting_rate_limit(&user, &mkt),
        Err(Ok(RateLimiterError::RateLimitExceeded))
    );

    // After one half-life (120 s): used = 1 >> 1 = 0 → available = 1
    advance(&env, 120);
    assert!(
        client.try_check_voting_rate_limit(&user, &mkt).is_ok(),
        "should recover after one half-life"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Interoperability: HalfLife and Linear modes coexist at config level
// ─────────────────────────────────────────────────────────────────────────────

/// `update_rate_limits` can switch from Linear to HalfLife.
#[test]
fn halflife_can_switch_from_linear_to_halflife_via_update() {
    let env = Env::default();
    env.mock_all_auths();
    set_time(&env, 0);

    // Start with Linear config
    let linear_config = RateLimitConfig {
        voting_limit: 5,
        dispute_limit: 5,
        oracle_call_limit: 100,
        bet_limit: 0,
        events_per_admin_limit: 0,
        time_window_seconds: 3600,
        refill_mode: RefillMode::Linear,
    };
    let client = deploy(&env, linear_config);

    // Switch to HalfLife via update
    let hl_config = halflife_config(5, 60);
    let admin = Address::generate(&env);
    client.update_rate_limits(&admin, &hl_config);

    // HalfLife now active — exhaust and verify decay behaviour
    let user = Address::generate(&env);
    let mkt = Symbol::new(&env, "market");
    for _ in 0..5 {
        client.check_voting_rate_limit(&user, &mkt);
    }
    assert_eq!(
        client.try_check_voting_rate_limit(&user, &mkt),
        Err(Ok(RateLimiterError::RateLimitExceeded))
    );

    // After one half-life some tokens should be back
    advance(&env, 60);
    assert!(
        client.try_check_voting_rate_limit(&user, &mkt).is_ok(),
        "HalfLife recovery should work after config switch"
    );
}
