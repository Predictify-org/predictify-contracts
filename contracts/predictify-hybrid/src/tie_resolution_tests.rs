
//! Tie-resolution regression tests for `resolve_market_with_ties`.
//!
//! These tests lock the documented payout specification for equal-stake
//! multi-winner markets.  Every scenario is self-contained: it creates a
//! fresh market, places bets (synced to votes/stakes), advances the ledger past
//! the market end-time
//! and dispute window, resolves with ties, then asserts payout correctness.
//!
//! # Payout formula (from PAYOUT_SPECIFICATION.md)
//!
//! ```text
//! user_share  = user_stake * (10_000 - fee_bps) / 10_000
//! payout      = user_share * total_pool / winning_total
//! ```
//!
//! Where `winning_total` is the sum of stakes on **all** winning outcomes.
//! For a perfect two-way tie with equal stakes the formula reduces to:
//!
//! ```text
//! payout ≈ user_stake * (1 - fee) * total_pool / (total_pool / 2)
//!        = user_stake * (1 - fee) * 2
//! ```
//!
//! # Acceptance criteria verified
//!
//! - Two-way tie payouts are proportional to individual stakes.
//! - Three-way tie payouts are proportional to individual stakes.
//! - Sum of all payouts never exceeds `total_pool` minus fees (no dust leak).
//! - Single-winner path is unaffected by the tie code-path.
//! - Rounding dust (odd-stroop pools) never causes an over-payment.
//! - Recorded payouts match [`PayoutData`] / `MarketUtils::calculate_payout`.

use crate::errors::Error;
use crate::markets::{MarketAnalytics, MarketUtils, WinningStats};
use crate::types::{Market, MarketState, OracleConfig, OracleProvider};
use crate::voting::PayoutData;
use crate::{PredictifyHybrid, PredictifyHybridClient};
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{vec, Address, Env, String, Symbol};

// ---------------------------------------------------------------------------
// Test harness
// ---------------------------------------------------------------------------

/// Shared setup for every tie-resolution test.
///
/// Mirrors the pattern used in `voting_tests.rs` so the two suites stay
/// consistent.  The contract is initialised with the default 2 % platform fee
/// (200 basis points) stored under the `"platform_fee"` key, which is what
/// `distribute_payouts` reads.
struct TieSetup {
    env: Env,
    contract_id: Address,
    admin: Address,
    token_id: Address,
}

impl TieSetup {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let contract_id = env.register(PredictifyHybrid, ());

        // Register a Stellar asset so the token client works.
        let token_contract =
            env.register_stellar_asset_contract_v2(Address::generate(&env));
        let token_id = token_contract.address();

        // Wire the token and circuit-breaker before initialising the contract.
        env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .set(&Symbol::new(&env, "TokenID"), &token_id);
            crate::circuit_breaker::CircuitBreaker::initialize(&env).unwrap();
        });

        PredictifyHybridClient::new(&env, &contract_id).initialize(&admin, &None, &None);

        // `initialize` stores DEFAULT_PLATFORM_FEE_PERCENTAGE (200 bps = 2 %)
        // under "platform_fee".  `distribute_payouts` reads that key directly,
        // so no extra setup is needed.

        Self { env, contract_id, admin, token_id }
    }

    /// Create and fund a fresh user with 100 000 XLM worth of stroops.
    fn user(&self) -> Address {
        let u = Address::generate(&self.env);
        soroban_sdk::token::StellarAssetClient::new(&self.env, &self.token_id)
            .mint(&u, &100_000_000_000i128);
        u
    }

    /// Create a market with the supplied `outcomes` and a 1-day duration.
    /// The dispute window is set to 0 so tests can claim immediately after
    /// resolution without advancing the ledger a second time.
    fn create_market(&self, outcomes: soroban_sdk::Vec<String>) -> Symbol {
        PredictifyHybridClient::new(&self.env, &self.contract_id).create_market(
            &self.admin,
            &String::from_str(&self.env, "Tie regression market"),
            &outcomes,
            &1u32, // 1-day duration
            &OracleConfig::new(
                OracleProvider::reflector(),
                Address::from_str(
                    &self.env,
                    "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
                ),
                String::from_str(&self.env, "BTC/USD"),
                5_000_000,
                String::from_str(&self.env, "gt"),
            ),
            &None,
            &86400u64,
            &None,
            &None,
            // dispute_window_seconds = 0 so claims are unblocked immediately
            &Some(0u64),
        )
    }

    /// Advance the ledger past the market end-time (and any dispute window).
    fn advance_past_end(&self) {
        // 1 day + 1 second is enough to pass a 1-day market.
        self.env.ledger().with_mut(|li| li.timestamp += 86_401);
    }

    /// Lock stake on an outcome via `place_bet` (same path `test.rs` uses for
    /// tie-resolution scenarios).
    fn stake_on(&self, user: &Address, market_id: &Symbol, outcome: &str, amount: i128) {
        PredictifyHybridClient::new(&self.env, &self.contract_id).place_bet(
            user,
            market_id,
            &String::from_str(&self.env, outcome),
            &amount,
         &None,);
    }

    /// Resolve with ties via the admin endpoint.
    fn resolve_with_ties(&self, market_id: &Symbol, winning: soroban_sdk::Vec<String>) {
        PredictifyHybridClient::new(&self.env, &self.contract_id)
            .resolve_market_with_ties(&self.admin, market_id, &winning);
    }

    /// Read the payout recorded in `market.claimed` for `user`.
    /// Returns 0 if the user has no entry.
    fn recorded_payout(&self, market_id: &Symbol, user: &Address) -> i128 {
        self.env.as_contract(&self.contract_id, || {
            let market: Market = self
                .env
                .storage()
                .persistent()
                .get(market_id)
                .unwrap();
            market
                .claimed
                .get(user.clone())
                .map(|info| info.get_payout())
                .unwrap_or(0)
        })
    }

    /// Platform fee in basis points (matches default `initialize` value).
    const FEE_BPS: i128 = 200;

    /// Compute the expected payout using the documented formula.
    fn expected_payout(user_stake: i128, winning_total: i128, total_pool: i128) -> i128 {
        let fee_denom: i128 = 10_000;
        let user_share = user_stake * (fee_denom - Self::FEE_BPS) / fee_denom;
        user_share * total_pool / winning_total
    }

    /// Upper bound on distributable winnings: total pool minus platform fee.
    fn max_distributable(total_pool: i128) -> i128 {
        total_pool * (10_000 - Self::FEE_BPS) / 10_000
    }

    /// Build a [`PayoutData`] snapshot and verify it against the recorded payout.
    fn assert_payout_data_matches_spec(
        user_stake: i128,
        winning_total: i128,
        total_pool: i128,
        actual_payout: i128,
    ) {
        let expected = MarketUtils::calculate_payout(user_stake, winning_total, total_pool, 2)
            .expect("calculate_payout must succeed for positive winning_total");

        let data = PayoutData {
            user_stake,
            winning_total,
            total_pool,
            fee_percentage: 2,
            payout_amount: expected,
        };

        assert_eq!(
            actual_payout, data.payout_amount,
            "recorded payout must match PayoutData spec snapshot"
        );
    }
}

// ---------------------------------------------------------------------------
// 1. Two-way tie — equal stakes
// ---------------------------------------------------------------------------

/// Two users each stake the same amount on different outcomes.
/// Both outcomes are declared winners.  Each user should receive a payout
/// equal to their proportional share of the pool after the 2 % fee.
///
/// Pool = 200, winning_total = 200, user_stake = 100
/// expected = 100 * 9800 / 10000 * 200 / 200 = 98
#[test]
fn test_two_way_tie_equal_stakes_payout_correct() {
    let s = TieSetup::new();

    let outcomes = vec![
        &s.env,
        String::from_str(&s.env, "Alpha"),
        String::from_str(&s.env, "Beta"),
    ];
    let mid = s.create_market(outcomes);

    let u1 = s.user();
    let u2 = s.user();
    let stake: i128 = 100_000_000; // 10 XLM in stroops

    s.stake_on(&u1, &mid, "Alpha", stake);
    s.stake_on(&u2, &mid, "Beta", stake);

    s.advance_past_end();

    let winning = vec![
        &s.env,
        String::from_str(&s.env, "Alpha"),
        String::from_str(&s.env, "Beta"),
    ];
    s.resolve_with_ties(&mid, winning);

    let total_pool = stake * 2;
    let winning_total = stake * 2; // both outcomes win
    let expected = TieSetup::expected_payout(stake, winning_total, total_pool);

    let p1 = s.recorded_payout(&mid, &u1);
    let p2 = s.recorded_payout(&mid, &u2);

    assert_eq!(p1, expected, "u1 payout mismatch in two-way equal-stake tie");
    assert_eq!(p2, expected, "u2 payout mismatch in two-way equal-stake tie");

    TieSetup::assert_payout_data_matches_spec(stake, winning_total, total_pool, p1);
    TieSetup::assert_payout_data_matches_spec(stake, winning_total, total_pool, p2);

    let cap = TieSetup::max_distributable(total_pool);
    assert!(
        p1 + p2 <= cap,
        "payouts ({}) exceed pool minus fees ({}) — dust leak detected",
        p1 + p2,
        cap
    );
}

// ---------------------------------------------------------------------------
// 2. Two-way tie — unequal stakes
// ---------------------------------------------------------------------------

/// u1 stakes 3× more than u2 on different outcomes, both declared winners.
/// Payouts must be proportional: p1 / p2 ≈ 3.
#[test]
fn test_two_way_tie_unequal_stakes_proportional_payout() {
    let s = TieSetup::new();

    let outcomes = vec![
        &s.env,
        String::from_str(&s.env, "Alpha"),
        String::from_str(&s.env, "Beta"),
    ];
    let mid = s.create_market(outcomes);

    let u1 = s.user();
    let u2 = s.user();
    let stake1: i128 = 300_000_000; // 30 XLM
    let stake2: i128 = 100_000_000; // 10 XLM

    s.stake_on(&u1, &mid, "Alpha", stake1);
    s.stake_on(&u2, &mid, "Beta", stake2);

    s.advance_past_end();

    let winning = vec![
        &s.env,
        String::from_str(&s.env, "Alpha"),
        String::from_str(&s.env, "Beta"),
    ];
    s.resolve_with_ties(&mid, winning);

    let total_pool = stake1 + stake2;
    let winning_total = stake1 + stake2;

    let expected1 = TieSetup::expected_payout(stake1, winning_total, total_pool);
    let expected2 = TieSetup::expected_payout(stake2, winning_total, total_pool);

    let p1 = s.recorded_payout(&mid, &u1);
    let p2 = s.recorded_payout(&mid, &u2);

    assert_eq!(p1, expected1, "u1 payout mismatch");
    assert_eq!(p2, expected2, "u2 payout mismatch");

    // Proportionality: p1 should be ~3× p2 (within 1 stroop of rounding).
    let ratio = (p1 * 100) / p2;
    assert!(
        ratio >= 295 && ratio <= 305,
        "expected ~3:1 payout ratio, got {ratio} (p1={p1}, p2={p2})"
    );

    // No dust leak: sum must stay within pool minus platform fee.
    let cap = TieSetup::max_distributable(total_pool);
    assert!(
        p1 + p2 <= cap,
        "payouts ({}) exceed pool minus fees ({})",
        p1 + p2,
        cap
    );
}

// ---------------------------------------------------------------------------
// 3. Three-way tie — equal stakes
// ---------------------------------------------------------------------------

/// Three users each stake the same amount on three different outcomes.
/// All three outcomes are declared winners.  Each user should receive the
/// same payout (pool / 3 after fee, since winning_total == total_pool).
#[test]
fn test_three_way_tie_equal_stakes_payout_correct() {
    let s = TieSetup::new();

    let outcomes = vec![
        &s.env,
        String::from_str(&s.env, "Aa"),
        String::from_str(&s.env, "Bb"),
        String::from_str(&s.env, "Cc"),
    ];
    let mid = s.create_market(outcomes);

    let u1 = s.user();
    let u2 = s.user();
    let u3 = s.user();
    let stake: i128 = 100_000_000; // 10 XLM each

    s.stake_on(&u1, &mid, "Aa", stake);
    s.stake_on(&u2, &mid, "Bb", stake);
    s.stake_on(&u3, &mid, "Cc", stake);

    s.advance_past_end();

    let winning = vec![
        &s.env,
        String::from_str(&s.env, "Aa"),
        String::from_str(&s.env, "Bb"),
        String::from_str(&s.env, "Cc"),
    ];
    s.resolve_with_ties(&mid, winning);

    let total_pool = stake * 3;
    let winning_total = stake * 3;
    let expected = TieSetup::expected_payout(stake, winning_total, total_pool);

    let p1 = s.recorded_payout(&mid, &u1);
    let p2 = s.recorded_payout(&mid, &u2);
    let p3 = s.recorded_payout(&mid, &u3);

    assert_eq!(p1, expected, "u1 payout mismatch in three-way tie");
    assert_eq!(p2, expected, "u2 payout mismatch in three-way tie");
    assert_eq!(p3, expected, "u3 payout mismatch in three-way tie");

    TieSetup::assert_payout_data_matches_spec(stake, winning_total, total_pool, p1);

    let cap = TieSetup::max_distributable(total_pool);
    assert!(
        p1 + p2 + p3 <= cap,
        "payouts ({}) exceed pool minus fees ({})",
        p1 + p2 + p3,
        cap
    );
}

// ---------------------------------------------------------------------------
// 4. Three-way tie — mixed stakes, two winners one loser
// ---------------------------------------------------------------------------

/// Three users stake on three outcomes; only two outcomes are declared
/// winners (partial tie).  The loser's stake is absorbed into the pool
/// and split between the two winners proportionally.
#[test]
fn test_three_outcome_partial_tie_two_winners_one_loser() {
    let s = TieSetup::new();

    let outcomes = vec![
        &s.env,
        String::from_str(&s.env, "Aa"),
        String::from_str(&s.env, "Bb"),
        String::from_str(&s.env, "Cc"),
    ];
    let mid = s.create_market(outcomes);

    let winner1 = s.user();
    let winner2 = s.user();
    let loser = s.user();

    let stake_w1: i128 = 200_000_000; // 20 XLM
    let stake_w2: i128 = 100_000_000; // 10 XLM
    let stake_l: i128 = 150_000_000;  // 15 XLM (lost)

    s.stake_on(&winner1, &mid, "Aa", stake_w1);
    s.stake_on(&winner2, &mid, "Bb", stake_w2);
    s.stake_on(&loser,   &mid, "Cc", stake_l);

    s.advance_past_end();

    // Only Aa and Bb win — Cc loses.
    let winning = vec![
        &s.env,
        String::from_str(&s.env, "Aa"),
        String::from_str(&s.env, "Bb"),
    ];
    s.resolve_with_ties(&mid, winning);

    let total_pool = stake_w1 + stake_w2 + stake_l;
    let winning_total = stake_w1 + stake_w2;

    let exp_w1 = TieSetup::expected_payout(stake_w1, winning_total, total_pool);
    let exp_w2 = TieSetup::expected_payout(stake_w2, winning_total, total_pool);

    let p_w1 = s.recorded_payout(&mid, &winner1);
    let p_w2 = s.recorded_payout(&mid, &winner2);
    let p_l  = s.recorded_payout(&mid, &loser);

    assert_eq!(p_w1, exp_w1, "winner1 payout mismatch");
    assert_eq!(p_w2, exp_w2, "winner2 payout mismatch");
    assert_eq!(p_l, 0, "loser must receive zero payout");

    // Winners should receive more than their original stake (they absorbed the loser).
    assert!(p_w1 > stake_w1, "winner1 should profit from loser's stake");
    assert!(p_w2 > stake_w2, "winner2 should profit from loser's stake");

    // No dust leak.
    let cap = TieSetup::max_distributable(total_pool);
    assert!(
        p_w1 + p_w2 <= cap,
        "payouts ({}) exceed pool minus fees ({})",
        p_w1 + p_w2,
        cap
    );
}

// ---------------------------------------------------------------------------
// 5. Single-winner path unaffected
// ---------------------------------------------------------------------------

/// Resolving via `resolve_market_with_ties` with a single winning outcome
/// must behave identically to `resolve_market_manual`: the sole winner
/// receives the full pool minus the platform fee.
#[test]
fn test_single_winner_via_ties_endpoint_correct() {
    let s = TieSetup::new();

    let outcomes = vec![
        &s.env,
        String::from_str(&s.env, "Yes"),
        String::from_str(&s.env, "No"),
    ];
    let mid = s.create_market(outcomes);

    let winner = s.user();
    let loser  = s.user();
    let stake_w: i128 = 300_000_000; // 30 XLM
    let stake_l: i128 = 100_000_000; // 10 XLM

    s.stake_on(&winner, &mid, "Yes", stake_w);
    s.stake_on(&loser,  &mid, "No",  stake_l);

    s.advance_past_end();

    // Single winner passed through the ties endpoint.
    let winning = vec![&s.env, String::from_str(&s.env, "Yes")];
    s.resolve_with_ties(&mid, winning);

    let total_pool = stake_w + stake_l;
    let winning_total = stake_w;
    let expected = TieSetup::expected_payout(stake_w, winning_total, total_pool);

    let p_winner = s.recorded_payout(&mid, &winner);
    let p_loser  = s.recorded_payout(&mid, &loser);

    assert_eq!(p_winner, expected, "single-winner payout mismatch");
    assert_eq!(p_loser, 0, "loser must receive zero");

    TieSetup::assert_payout_data_matches_spec(stake_w, winning_total, total_pool, p_winner);

    assert!(p_winner > stake_w, "winner should profit");

    assert!(
        p_winner <= TieSetup::max_distributable(total_pool),
        "winner payout ({}) exceeds pool minus fees",
        p_winner
    );
}

/// Single-winner payout via `resolve_market_with_ties` must match
/// `resolve_market_manual` for identical stake layout.
#[test]
fn test_single_winner_ties_matches_manual_resolve_payout() {
    let stake_w: i128 = 250_000_000;
    let stake_l: i128 = 75_000_000;
    let total_pool = stake_w + stake_l;
    let winning_total = stake_w;
    let expected = TieSetup::expected_payout(stake_w, winning_total, total_pool);

    // Market resolved through the ties endpoint (single outcome).
    let ties = TieSetup::new();
    let mid_ties = ties.create_market(vec![
        &ties.env,
        String::from_str(&ties.env, "Yes"),
        String::from_str(&ties.env, "No"),
    ]);
    let w_ties = ties.user();
    let l_ties = ties.user();
    ties.stake_on(&w_ties, &mid_ties, "Yes", stake_w);
    ties.stake_on(&l_ties, &mid_ties, "No", stake_l);
    ties.advance_past_end();
    ties.resolve_with_ties(
        &mid_ties,
        vec![&ties.env, String::from_str(&ties.env, "Yes")],
    );
    let p_ties = ties.recorded_payout(&mid_ties, &w_ties);

    // Market resolved through manual single-outcome endpoint.
    let manual = TieSetup::new();
    let mid_manual = manual.create_market(vec![
        &manual.env,
        String::from_str(&manual.env, "Yes"),
        String::from_str(&manual.env, "No"),
    ]);
    let w_manual = manual.user();
    let l_manual = manual.user();
    manual.stake_on(&w_manual, &mid_manual, "Yes", stake_w);
    manual.stake_on(&l_manual, &mid_manual, "No", stake_l);
    manual.advance_past_end();
    PredictifyHybridClient::new(&manual.env, &manual.contract_id).resolve_market_manual(
        &manual.admin,
        &mid_manual,
        &String::from_str(&manual.env, "Yes"),
    );
    let p_manual = manual.recorded_payout(&mid_manual, &w_manual);

    assert_eq!(p_ties, expected, "ties endpoint payout mismatch");
    assert_eq!(p_manual, expected, "manual resolve payout mismatch");
    assert_eq!(
        p_ties, p_manual,
        "single-winner ties payout must equal manual resolve payout"
    );
}

// ---------------------------------------------------------------------------
// 6. Rounding dust — odd-stroop pool
// ---------------------------------------------------------------------------

/// Use a pool size that does not divide evenly to verify that integer
/// truncation never causes the sum of payouts to exceed the pool.
#[test]
fn test_rounding_dust_never_exceeds_pool() {
    let s = TieSetup::new();

    let outcomes = vec![
        &s.env,
        String::from_str(&s.env, "Xx"),
        String::from_str(&s.env, "Yy"),
    ];
    let mid = s.create_market(outcomes);

    let u1 = s.user();
    let u2 = s.user();

    // Minimum allowed bet is 1_000_000 stroops (0.1 XLM).
    // Use the smallest equal stakes to maximise rounding effect.
    let stake: i128 = 1_000_001; // odd number to force non-integer split

    s.stake_on(&u1, &mid, "Xx", stake);
    s.stake_on(&u2, &mid, "Yy", stake);

    s.advance_past_end();

    let winning = vec![
        &s.env,
        String::from_str(&s.env, "Xx"),
        String::from_str(&s.env, "Yy"),
    ];
    s.resolve_with_ties(&mid, winning);

    let total_pool = stake * 2;
    let p1 = s.recorded_payout(&mid, &u1);
    let p2 = s.recorded_payout(&mid, &u2);

    let cap = TieSetup::max_distributable(total_pool);
    assert!(
        p1 + p2 <= cap,
        "dust leak: payouts ({}) exceed pool minus fees ({})",
        p1 + p2,
        cap
    );

    // Both payouts must be non-negative.
    assert!(p1 >= 0, "negative payout for u1");
    assert!(p2 >= 0, "negative payout for u2");
}

/// Near-tie: winning sides differ by 1 stroop; payouts stay proportional and
/// within pool minus fees (integer rounding must not leak dust).
#[test]
fn test_near_tie_one_stroop_difference_proportional() {
    let s = TieSetup::new();

    let outcomes = vec![
        &s.env,
        String::from_str(&s.env, "Alpha"),
        String::from_str(&s.env, "Beta"),
    ];
    let mid = s.create_market(outcomes);

    let u1 = s.user();
    let u2 = s.user();
    let stake1: i128 = 100_000_001;
    let stake2: i128 = 100_000_000;

    s.stake_on(&u1, &mid, "Alpha", stake1);
    s.stake_on(&u2, &mid, "Beta", stake2);

    s.advance_past_end();

    let winning = vec![
        &s.env,
        String::from_str(&s.env, "Alpha"),
        String::from_str(&s.env, "Beta"),
    ];
    s.resolve_with_ties(&mid, winning);

    let total_pool = stake1 + stake2;
    let winning_total = total_pool;

    let p1 = s.recorded_payout(&mid, &u1);
    let p2 = s.recorded_payout(&mid, &u2);

    TieSetup::assert_payout_data_matches_spec(stake1, winning_total, total_pool, p1);
    TieSetup::assert_payout_data_matches_spec(stake2, winning_total, total_pool, p2);

    assert!(p1 >= p2, "larger stake must receive >= payout in near-tie");
    assert!(
        p1 + p2 <= TieSetup::max_distributable(total_pool),
        "near-tie payouts must not exceed pool minus fees"
    );
}

// ---------------------------------------------------------------------------
// 7. Asymmetric rounding — large pool, small winning side
// ---------------------------------------------------------------------------

/// One winner with a tiny stake vs. a large loser pool.
/// Verifies the winner receives a large multiplied payout and the sum
/// still does not exceed the total pool.
#[test]
fn test_large_pool_small_winning_stake_no_overflow() {
    let s = TieSetup::new();

    let outcomes = vec![
        &s.env,
        String::from_str(&s.env, "Rare"),
        String::from_str(&s.env, "Common"),
    ];
    let mid = s.create_market(outcomes);

    let winner = s.user();
    let loser  = s.user();

    let stake_w: i128 = 1_000_000;       // 0.1 XLM (minimum)
    let stake_l: i128 = 90_000_000_000;  // 9 000 XLM

    s.stake_on(&winner, &mid, "Rare",   stake_w);
    s.stake_on(&loser,  &mid, "Common", stake_l);

    s.advance_past_end();

    let winning = vec![&s.env, String::from_str(&s.env, "Rare")];
    s.resolve_with_ties(&mid, winning);

    let total_pool = stake_w + stake_l;
    let p_winner = s.recorded_payout(&mid, &winner);

    // Winner should receive nearly the entire pool (minus 2 % fee).
    let expected = TieSetup::expected_payout(stake_w, stake_w, total_pool);
    assert_eq!(p_winner, expected, "large-pool single-winner payout mismatch");

    assert!(
        p_winner <= TieSetup::max_distributable(total_pool),
        "winner payout ({}) exceeds pool minus fees",
        p_winner
    );
}

// ---------------------------------------------------------------------------
// 8. calculate_winning_stats reflects tie correctly
// ---------------------------------------------------------------------------

/// Unit-test `MarketAnalytics::calculate_winning_stats` directly against a
/// crafted `Market` to verify it sums stakes for the requested outcome only.
#[test]
fn test_calculate_winning_stats_two_way_tie_each_outcome() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(PredictifyHybrid, ());
    let token_id =
        env.register_stellar_asset_contract_v2(Address::generate(&env)).address();
    env.as_contract(&contract_id, || {
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, "TokenID"), &token_id);
        crate::circuit_breaker::CircuitBreaker::initialize(&env).unwrap();
    });

    let admin = Address::generate(&env);
    PredictifyHybridClient::new(&env, &contract_id).initialize(&admin, &None, &None);

    // Build a market in memory (no storage write needed for this unit test).
    let mut market = Market::new(
        &env,
        admin.clone(),
        String::from_str(&env, "Stats test?"),
        vec![
            &env,
            String::from_str(&env, "Alpha"),
            String::from_str(&env, "Beta"),
        ],
        env.ledger().timestamp() + 86400,
        OracleConfig::new(
            OracleProvider::reflector(),
            Address::generate(&env),
            String::from_str(&env, "BTC/USD"),
            5_000_000,
            String::from_str(&env, "gt"),
        ),
        None,
        0,
        MarketState::Active,
    );

    let u1 = Address::generate(&env);
    let u2 = Address::generate(&env);
    let u3 = Address::generate(&env);

    // u1 and u3 vote Alpha; u2 votes Beta.
    market.add_vote(u1.clone(), String::from_str(&env, "Alpha"), 300_000_000);
    market.add_vote(u2.clone(), String::from_str(&env, "Beta"),  200_000_000);
    market.add_vote(u3.clone(), String::from_str(&env, "Alpha"), 100_000_000);

    env.as_contract(&contract_id, || {
        let stats_alpha: WinningStats =
            MarketAnalytics::calculate_winning_stats(&market, &String::from_str(&env, "Alpha"));
        assert_eq!(stats_alpha.winning_total, 400_000_000, "Alpha winning_total");
        assert_eq!(stats_alpha.winning_voters, 2, "Alpha winning_voters");
        assert_eq!(stats_alpha.total_pool, market.total_staked, "Alpha total_pool");

        let stats_beta: WinningStats =
            MarketAnalytics::calculate_winning_stats(&market, &String::from_str(&env, "Beta"));
        assert_eq!(stats_beta.winning_total, 200_000_000, "Beta winning_total");
        assert_eq!(stats_beta.winning_voters, 1, "Beta winning_voters");
    });
}

// ---------------------------------------------------------------------------
// 9. calculate_payout formula matches spec
// ---------------------------------------------------------------------------

/// Directly verify `MarketUtils::calculate_payout` against [`PayoutData`]
/// examples from PAYOUT_SPECIFICATION.md §10.2.
#[test]
fn test_calculate_payout_spec_examples() {
    let data1 = PayoutData {
        user_stake: 1_000,
        winning_total: 5_000,
        total_pool: 10_000,
        fee_percentage: 2,
        payout_amount: 1_960,
    };
    let p1 = MarketUtils::calculate_payout(
        data1.user_stake,
        data1.winning_total,
        data1.total_pool,
        data1.fee_percentage,
    )
    .unwrap();
    assert_eq!(p1, data1.payout_amount, "spec example 1 mismatch");

    let data2 = PayoutData {
        user_stake: 2_000,
        winning_total: 7_000,
        total_pool: 10_000,
        fee_percentage: 2,
        payout_amount: 2_800,
    };
    let p2 = MarketUtils::calculate_payout(
        data2.user_stake,
        data2.winning_total,
        data2.total_pool,
        data2.fee_percentage,
    )
    .unwrap();
    assert_eq!(p2, data2.payout_amount, "spec example 2 mismatch");

    // Edge: winning_total == 0 must return NothingToClaim.
    let err = MarketUtils::calculate_payout(1_000, 0, 10_000, 2);
    assert_eq!(err, Err(Error::NothingToClaim), "zero winning_total must error");
}

// ---------------------------------------------------------------------------
// Pool-minus-fees invariant across three-way tie stake distributions
// ---------------------------------------------------------------------------

/// For three-way ties with varying stake ratios, verify proportional payouts
/// and that the sum never exceeds pool minus fees.
#[test]
fn test_three_way_tie_sum_never_exceeds_pool_minus_fees() {
    // Stake distributions to test: (stake_a, stake_b, stake_c)
    let distributions: &[(i128, i128, i128)] = &[
        (100_000_000, 100_000_000, 100_000_000), // equal
        (300_000_000, 200_000_000, 100_000_000), // 3:2:1
        (500_000_000, 300_000_000, 200_000_000), // 5:3:2
    ];

    for &(sa, sb, sc) in distributions {
        let s = TieSetup::new();

        let outcomes = vec![
            &s.env,
            String::from_str(&s.env, "Aa"),
            String::from_str(&s.env, "Bb"),
            String::from_str(&s.env, "Cc"),
        ];
        let mid = s.create_market(outcomes);

        let ua = s.user();
        let ub = s.user();
        let uc = s.user();

        s.stake_on(&ua, &mid, "Aa", sa);
        s.stake_on(&ub, &mid, "Bb", sb);
        s.stake_on(&uc, &mid, "Cc", sc);

        s.advance_past_end();

        let winning = vec![
            &s.env,
            String::from_str(&s.env, "Aa"),
            String::from_str(&s.env, "Bb"),
            String::from_str(&s.env, "Cc"),
        ];
        s.resolve_with_ties(&mid, winning);

        let total_pool = sa + sb + sc;
        let winning_total = total_pool;
        let cap = TieSetup::max_distributable(total_pool);
        let pa = s.recorded_payout(&mid, &ua);
        let pb = s.recorded_payout(&mid, &ub);
        let pc = s.recorded_payout(&mid, &uc);
        let sum = pa + pb + pc;

        TieSetup::assert_payout_data_matches_spec(sa, winning_total, total_pool, pa);
        TieSetup::assert_payout_data_matches_spec(sb, winning_total, total_pool, pb);
        TieSetup::assert_payout_data_matches_spec(sc, winning_total, total_pool, pc);

        assert!(
            sum <= cap,
            "distribution ({sa},{sb},{sc}): payouts {sum} exceed pool minus fees {cap}"
        );

        // Proportionality: payouts track stake ordering.
        if sa >= sb && sb >= sc {
            assert!(pa >= pb && pb >= pc, "payouts must follow stake order ({sa},{sb},{sc})");
        }
    }
}
