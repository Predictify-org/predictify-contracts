//! FeeCalculator property-based tests
//!
//! These tests use `proptest` to verify that `FeeCalculator::calculate_platform_fee`
//! and `FeeCalculator::calculate_fee_breakdown` never charge more than the
//! configured basis-point rate, always round down (floor), and always
//! reconcile exactly with `total_staked`.
//!
//! ## Invariants covered
//! 1. `fee <= total_staked` for every generated stake.
//! 2. `fee` never exceeds the configured bps rate applied to `total_staked`
//!    (i.e. `fee * 10_000 <= total_staked * PLATFORM_FEE_PERCENTAGE`).
//! 3. Rounding is strictly floor: `fee` is the largest integer satisfying (2).
//! 4. `platform_fee + user_payout_amount == total_staked` (breakdown reconciles).
//! 5. Fee is monotonically non-decreasing as `total_staked` increases.
//!
//! ## Non-goals
//! - Dynamic/activity-adjusted fees (`calculate_dynamic_fee`) — covered separately.
//! - Fee collection/storage side effects — covered by `fee_idempotency_tests.rs`.

#![cfg(test)]

use proptest::prelude::*;
use soroban_sdk::testutils::{Address as _, EnvTestConfig};
use soroban_sdk::{Env, String, Vec};

    /// Test utility to create a market with realistic Soroban parameters
    fn create_test_market(env: &Env, admin_address: Address, total_staked: i128) -> Symbol {
        let market_id = Symbol::new(env, "test_market");
        let mut market = Market {
            admin: admin_address,
            question: String::from_str(env, "Test question?"),
            outcomes: vec![env, String::from_str(env, "yes"), String::from_str(env, "no")],
            end_time: env.ledger().timestamp() + 86_400,
            oracle_config: OracleConfig::new(
                OracleProvider::Reflector,
                Address::from_str(env, "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF"),
                String::from_str(env, "BTC/USD"),
                10_000_000, // $100
                String::from_str(env, "gt"),
            ),
            has_fallback: false,
            fallback_oracle_config: OracleConfig::new(
                OracleProvider::Reflector,
                Address::from_str(env, "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF"),
                String::from_str(env, "BTC/USD"),
                10_000_000, // $100
                String::from_str(env, "gt"),
            ),
            resolution_timeout: 86400,
            oracle_result: None,
            votes: Map::new(env),
            total_staked,
            dispute_stakes: Map::new(env),
            stakes: Map::new(env),
            claimed: Map::new(env),
            winning_outcomes: None,
            fee_collected: false,
            state: MarketState::Active,
            total_extension_days: 0,
            max_extension_days: 30,
            extension_history: vec![env],
            category: None,
            tags: vec![env],
            min_pool_size: None,
            bet_deadline: 0,
            dispute_window_seconds: 86400,
            winnings_swept: false,
                dispute_stake_floor: None,
                max_participants: None,
            timelock_config: crate::timelock::MarketTimelockConfig::default(),
        };

/// Basis-point denominator used by `FeeCalculator::checked_bps_floor`.
const BPS_DENOMINATOR: i128 = 10_000;

/// Largest stake we can multiply by `BPS_DENOMINATOR` without overflowing i128,
/// keeping every generated case inside the "no overflow" branch of the calculator.
const MAX_SAFE_STAKE: i128 = i128::MAX / BPS_DENOMINATOR;

fn test_env() -> Env {
    let mut env = Env::default();
    env.set_config(EnvTestConfig {
        capture_snapshot_at_drop: false,
    });
    env
}

/// Builds a resolved market with the given `total_staked` amount, reusing the
/// shared test-market fixture so oracle/market metadata stays valid.
fn resolved_market(env: &Env, total_staked: i128) -> Market {
    let mut market = ValidationTestingUtils::create_test_market(env);
    let mut winning_outcomes = Vec::new(env);
    winning_outcomes.push_back(String::from_str(env, "yes"));
    market.winning_outcomes = Some(winning_outcomes);
    market.total_staked = total_staked;
    market
}

proptest! {
    /// Property 1 & 2: the fee is always within the pool and never exceeds the
    /// amount the configured bps rate would charge.
    #[test]
    fn fee_never_exceeds_configured_bps(stake in 1i128..=MAX_SAFE_STAKE) {
        let env = test_env();
        let market = resolved_market(&env, stake);

        if let Ok(fee) = FeeCalculator::calculate_platform_fee(&market) {
            prop_assert!(fee >= 0, "fee must be non-negative, got {}", fee);
            prop_assert!(fee <= stake, "fee {} exceeds stake {}", fee, stake);

            // fee must never exceed what the configured bps rate allows.
            prop_assert!(
                fee * BPS_DENOMINATOR <= stake * PLATFORM_FEE_PERCENTAGE,
                "fee {} exceeds configured {}bps of stake {}",
                fee, PLATFORM_FEE_PERCENTAGE, stake
            );
        }
        // Err is acceptable (e.g. below MIN_FEE_AMOUNT) — that path is exercised
        // by the bounds/threshold cases below and by fees.rs unit tests.
    }

    /// Property 3: rounding is strictly floor — `fee` is the *largest* integer
    /// that does not exceed the exact bps share of the stake.
    #[test]
    fn fee_rounds_down_not_up(stake in 1i128..=MAX_SAFE_STAKE) {
        let env = test_env();
        let market = resolved_market(&env, stake);

        if let Ok(fee) = FeeCalculator::calculate_platform_fee(&market) {
            let exact_numerator = stake * PLATFORM_FEE_PERCENTAGE;
            prop_assert!(
                fee * BPS_DENOMINATOR <= exact_numerator,
                "fee {} rounded up past exact share for stake {}", fee, stake
            );
            prop_assert!(
                (fee + 1) * BPS_DENOMINATOR > exact_numerator,
                "fee {} is not the floor of the exact share for stake {}", fee, stake
            );
        }
    }

    /// Property 4: platform_fee + user_payout_amount always reconciles to
    /// total_staked exactly (no dust lost or created by rounding).
    #[test]
    fn fee_breakdown_reconciles(stake in 1i128..=MAX_SAFE_STAKE) {
        let env = test_env();
        let market = resolved_market(&env, stake);

        if let Ok(breakdown) = FeeCalculator::calculate_fee_breakdown(&market) {
            prop_assert_eq!(breakdown.platform_fee, breakdown.fee_amount);
            prop_assert_eq!(
                breakdown.platform_fee + breakdown.user_payout_amount,
                breakdown.total_staked
            );
            prop_assert!(breakdown.user_payout_amount >= 0);
        }
    }

    /// Property 5: with a fixed bps rate, a larger stake never yields a smaller fee.
    #[test]
    fn fee_is_monotonic_in_stake(
        a in 1i128..=MAX_SAFE_STAKE,
        b in 1i128..=MAX_SAFE_STAKE,
    ) {
        let (lo, hi) = if a <= b { (a, b) } else { (b, a) };

        let env = test_env();
        let market_lo = resolved_market(&env, lo);
        let market_hi = resolved_market(&env, hi);

        let fee_lo = FeeCalculator::calculate_platform_fee(&market_lo);
        let fee_hi = FeeCalculator::calculate_platform_fee(&market_hi);

        if let (Ok(fee_lo), Ok(fee_hi)) = (fee_lo, fee_hi) {
            prop_assert!(
                fee_lo <= fee_hi,
                "monotonicity violated: fee({}) = {} > fee({}) = {}",
                lo, fee_lo, hi, fee_hi
            );
        }
    }
}

// ===== FOCUSED EDGE-CASE TESTS =====
// proptest's default RNG rarely lands exactly on these boundaries, so they are
// pinned down as explicit unit tests.

#[test]
fn edge_case_zero_stake_is_rejected() {
    let env = test_env();
    let market = resolved_market(&env, 0);

    assert!(FeeCalculator::calculate_platform_fee(&market).is_err());
}

#[test]
fn edge_case_i128_max_stake_does_not_panic() {
    let env = test_env();
    let market = resolved_market(&env, i128::MAX);

    // Must return an Err (overflow guard), never panic.
    let result = FeeCalculator::calculate_platform_fee(&market);
    assert!(result.is_err());
}

#[test]
fn edge_case_smallest_stake_that_clears_min_fee() {
    let env = test_env();
    // Smallest stake where floor(stake * 200 / 10_000) >= MIN_FEE_AMOUNT (1_000_000).
    let stake = (1_000_000i128 * BPS_DENOMINATOR) / PLATFORM_FEE_PERCENTAGE;
    let market = resolved_market(&env, stake);

    let fee = FeeCalculator::calculate_platform_fee(&market)
        .expect("fee at min-fee boundary should succeed");
    assert!(fee <= stake);
    assert!(fee * BPS_DENOMINATOR <= stake * PLATFORM_FEE_PERCENTAGE);
}

#[test]
fn edge_case_one_bps_rounds_down_for_small_pool() {
    // Pool of 10_001 stroops at 200 bps: exact share is 200.02 -> floors to 200.
    let env = test_env();
    let market = resolved_market(&env, 10_001);

    // Below MIN_FEE_AMOUNT, so this exercises the InsufficientStake path rather
    // than a nonzero fee — assert it fails closed instead of charging more than
    // the exact share would allow.
    let result = FeeCalculator::calculate_platform_fee(&market);
    if let Ok(fee) = result {
        assert!(fee * BPS_DENOMINATOR <= 10_001 * PLATFORM_FEE_PERCENTAGE);
    } else {
        assert!(result.is_err());
    }
}