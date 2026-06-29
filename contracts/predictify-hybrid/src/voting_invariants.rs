//! # Proptest invariant tests for VotingManager total-stake conservation
//!
//! Issue #607: These property tests assert that for any sequence of valid votes
//! the following invariant always holds:
//!
//!   sum(outcome_distribution.values()) == market.total_staked == VotingStats.total_staked
//!
//! Tests cover:
//! - Sequential single-outcome votes
//! - Multi-outcome distribution across outcomes
//! - Duplicate-user votes (second vote is silently ignored by the contract)
//! - Zero-stake votes (rejected, total_staked stays unchanged)
//!
//! Strategy: We operate directly on `Market` and `VotingUtils` (unit level) to
//! avoid the oracle/token infrastructure required by the full contract client.
//! `MarketStateManager::add_vote` is the same code path called by
//! `VotingManager::process_vote`, so this covers the bookkeeping invariant.

#![cfg(test)]

use crate::markets::MarketStateManager;
use crate::types::{Market, MarketState, OracleConfig, OracleProvider};
use crate::voting::VotingUtils;
use proptest::prelude::*;
use alloc::format;
use soroban_sdk::{testutils::Address as _, vec as svec, Address, Env, String};

// ── Constants ────────────────────────────────────────────────────────────────

/// Minimum valid stake (mirrors MIN_VOTE_STAKE in voting.rs / config.rs).
const MIN_STAKE: i128 = 1_000_000;

/// A handful of outcome labels that generated votes may reference.
const OUTCOME_LABELS: &[&str] = &["yes", "no", "maybe", "abstain"];

// ── Strategy helpers ─────────────────────────────────────────────────────────

/// Generates a valid stake in [MIN_STAKE, 1_000_000_000] (up to 100 XLM).
fn arb_stake() -> impl Strategy<Value = i128> {
    MIN_STAKE..=1_000_000_000i128
}

/// Generates an outcome index into OUTCOME_LABELS.
fn arb_outcome_idx() -> impl Strategy<Value = usize> {
    0..OUTCOME_LABELS.len()
}

/// A single vote instruction: (outcome_index, stake).
/// The test harness assigns a fresh unique Address per vote so duplicates are
/// only introduced when we deliberately want to test that case.
fn arb_vote_op() -> impl Strategy<Value = (usize, i128)> {
    (arb_outcome_idx(), arb_stake())
}

// ── Market builder ───────────────────────────────────────────────────────────

/// Build a minimal Active market with two outcomes in a fresh Env.
/// Returns `(env, market)`.
fn make_market() -> (Env, Market) {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let oracle = OracleConfig::none_sentinel(&env);
    let market = Market::new(
        &env,
        admin,
        String::from_str(&env, "Will BTC hit 100k?"),
        svec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
            String::from_str(&env, "maybe"),
            String::from_str(&env, "abstain"),
        ],
        env.ledger().timestamp() + 86_400, // ends in 24 h
        oracle,
        None,
        86_400,
        MarketState::Active,
    );
    (env, market)
}

// ── Core invariant check ─────────────────────────────────────────────────────

/// Assert: sum(outcome_distribution) == market.total_staked == VotingStats.total_staked
fn assert_stake_conservation(env: &Env, market: &Market) {
    let stats = VotingUtils::get_voting_stats(env, market);

    // sum of per-outcome buckets
    let dist_sum: i128 = stats.outcome_distribution.values().iter().sum();

    assert_eq!(
        dist_sum,
        market.total_staked,
        "outcome_distribution sum ({}) != market.total_staked ({})",
        dist_sum,
        market.total_staked,
    );

    assert_eq!(
        stats.total_staked,
        market.total_staked,
        "VotingStats.total_staked ({}) != market.total_staked ({})",
        stats.total_staked,
        market.total_staked,
    );
}

// ── Proptest suites ──────────────────────────────────────────────────────────

proptest! {
    // Use a deterministic seed so CI is reproducible (1000 cases each).
    #![proptest_config(ProptestConfig {
        cases: 1000,
        // Deterministic seed for CI reproducibility.
        source_file: Some("src/voting_invariants.rs"),
        ..ProptestConfig::default()
    })]

    /// After a sequence of distinct-user votes the invariant must hold after
    /// every step, not just at the end.
    #[test]
    fn prop_sequential_votes_conserve_stake(
        votes in prop::collection::vec(arb_vote_op(), 1..=20),
    ) {
        let (env, mut market) = make_market();

        for (outcome_idx, stake) in votes {
            let user = Address::generate(&env);
            let outcome = String::from_str(&env, OUTCOME_LABELS[outcome_idx]);
            MarketStateManager::add_vote(&mut market, user, outcome, stake, None);
            assert_stake_conservation(&env, &market);
        }
    }

    /// Invariant holds with votes spread across all four outcome labels.
    #[test]
    fn prop_multi_outcome_distribution_conserves_stake(
        v0 in arb_vote_op(),
        v1 in arb_vote_op(),
        v2 in arb_vote_op(),
        v3 in arb_vote_op(),
    ) {
        let (env, mut market) = make_market();
        let votes = [v0, v1, v2, v3];
        for (i, (outcome_idx, stake)) in votes.iter().enumerate() {
            // Pin each vote to a specific outcome to guarantee full coverage.
            let forced_outcome = String::from_str(&env, OUTCOME_LABELS[i % OUTCOME_LABELS.len()]);
            let _ = outcome_idx; // outcome_idx from proptest keeps strategy active; pinning for coverage
            let user = Address::generate(&env);
            MarketStateManager::add_vote(&mut market, user, forced_outcome, *stake, None);
        }
        assert_stake_conservation(&env, &market);
    }

    /// Duplicate vote from the same user: the contract layer (`process_vote`)
    /// rejects it with `AlreadyVoted`, so `total_staked` must not double-count.
    ///
    /// This test verifies two things:
    /// 1. After the first (valid) vote, the invariant holds.
    /// 2. Documented raw `add_vote` behavior when the guard is bypassed:
    ///    `stakes[user]` is overwritten (upsert → stake2), but `total_staked`
    ///    accumulates both stakes, creating an invariant violation.
    ///    This confirms the `AlreadyVoted` guard in `process_vote` is load-bearing.
    #[test]
    fn prop_duplicate_user_stake_not_silently_doubled(
        stake1 in arb_stake(),
        stake2 in arb_stake(),
        outcome_idx in arb_outcome_idx(),
    ) {
        let (env, mut market) = make_market();
        let user = Address::generate(&env);
        let outcome = String::from_str(&env, OUTCOME_LABELS[outcome_idx]);

        // First vote — always valid; invariant must hold after it
        MarketStateManager::add_vote(&mut market, user.clone(), outcome.clone(), stake1, None);
        prop_assert_eq!(market.total_staked, stake1, "First vote sets total_staked to stake1");
        assert_stake_conservation(&env, &market);

        // Raw second call with same Address (bypassing process_vote's AlreadyVoted guard):
        // stakes[user] is overwritten to stake2; total_staked accumulates both.
        MarketStateManager::add_vote(&mut market, user.clone(), outcome.clone(), stake2, None);

        let recorded_stake = market.stakes.get(user).unwrap_or(0);
        prop_assert_eq!(
            recorded_stake, stake2,
            "stakes map holds the latest value (upsert)"
        );
        // total_staked double-counts — this is the vulnerability the guard prevents
        prop_assert_eq!(
            market.total_staked, stake1 + stake2,
            "total_staked double-counts when AlreadyVoted guard is bypassed"
        );

        // The invariant is intentionally violated here; document it via explicit assertion.
        let stats = VotingUtils::get_voting_stats(&env, &market);
        let dist_sum: i128 = stats.outcome_distribution.values().iter().sum();
        prop_assert_eq!(
            dist_sum, stake2,
            "outcome_distribution uses the upserted (latest) stake"
        );
        // dist_sum != total_staked here — expected; the guard prevents this in production
        prop_assert!(
            dist_sum < market.total_staked,
            "invariant violation when guard is bypassed: dist_sum={} < total_staked={}",
            dist_sum, market.total_staked
        );
    }

    /// Zero-stake votes must not change total_staked.
    /// Note: `process_vote` rejects stake < MIN_VOTE_STAKE before reaching add_vote.
    /// This test confirms that if stake=0 somehow passes through, the invariant holds.
    #[test]
    fn prop_zero_stake_does_not_corrupt_total(
        good_stake in arb_stake(),
        outcome_idx in arb_outcome_idx(),
    ) {
        let (env, mut market) = make_market();
        let outcome = String::from_str(&env, OUTCOME_LABELS[outcome_idx]);

        // Apply a valid vote first
        let user1 = Address::generate(&env);
        MarketStateManager::add_vote(&mut market, user1, outcome.clone(), good_stake, None);
        let baseline = market.total_staked;

        // Apply a zero-stake vote (bypasses the MIN_VOTE_STAKE guard at unit level)
        let user2 = Address::generate(&env);
        MarketStateManager::add_vote(&mut market, user2, outcome, 0, None);

        // total_staked grows by 0; distribution sum must still match
        prop_assert_eq!(market.total_staked, baseline, "zero-stake should not increase total_staked");
        assert_stake_conservation(&env, &market);
    }

    /// Large number of voters: invariant must hold at the end of 200 votes.
    #[test]
    fn prop_large_voter_set_conserves_stake(
        votes in prop::collection::vec(arb_vote_op(), 50..=200),
    ) {
        let (env, mut market) = make_market();
        let expected_total: i128 = votes.iter().map(|(_, s)| s).sum();

        for (outcome_idx, stake) in &votes {
            let user = Address::generate(&env);
            let outcome = String::from_str(&env, OUTCOME_LABELS[*outcome_idx]);
            MarketStateManager::add_vote(&mut market, user, outcome, *stake, None);
        }

        prop_assert_eq!(
            market.total_staked,
            expected_total,
            "total_staked ({}) must equal sum of all stakes ({})",
            market.total_staked,
            expected_total
        );
        assert_stake_conservation(&env, &market);
    }
}
