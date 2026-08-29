//! Focused tests for voting snapshot stability across repeated reads.
//!
//! The community-consensus "snapshot" is computed (not persisted) by
//! [`crate::markets::MarketAnalytics::calculate_community_consensus`] and consumed
//! by [`crate::markets::MarketUtils::determine_winning_outcomes`]. These tests pin
//! the two critical determinism invariants:
//!
//! 1. Repeated reads of the consensus yield an identical value for the same vote
//!    census (stable snapshot).
//! 2. Tie-breaking does not depend on Soroban `Map` iteration order; ties resolve
//!    against the market's canonical, application-ordered `outcomes` list.

#![cfg(test)]

use soroban_sdk::testutils::Address as _;
use soroban_sdk::{vec, Address, Env, Map, String, Vec};

use crate::markets::{MarketAnalytics, MarketUtils};
use crate::types::{Market, MarketState, OracleConfig, OracleProvider};

/// Builds a Market with the given outcomes and no votes.
fn make_market(env: &Env, outcomes: Vec<String>) -> Market {
    Market::new(
        env,
        Address::generate(env),
        String::from_str(env, "Will test outcomes be deterministically selected?"),
        outcomes,
        env.ledger().timestamp() + 86400,
        OracleConfig::new(
            OracleProvider::pyth(),
            Address::from_str(
                env,
                "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
            ),
            String::from_str(env, "TEST/USD"),
            2_500_000,
            String::from_str(env, "gt"),
        ),
        None,
        86400,
        MarketState::Active,
    )
}

fn outcomes(env: &Env, names: &[&str]) -> Vec<String> {
    let mut out = Vec::new(env);
    for name in names {
        out.push_back(String::from_str(env, name));
    }
    out
}

/// Casts one vote of `stake` for each `(outcome, stake)` entry under a fresh
/// address. Outcomes are the canonical winners candidates; votes are keyed by
/// generated addresses (the contract logic keys votes by Address).
fn cast_votes(env: &Env, market: &mut Market, entries: &[(&str, i128)]) {
    for (outcome, stake) in entries {
        let address = Address::generate(env);
        market
            .votes
            .set(address.clone(), String::from_str(env, outcome));
        market.stakes.set(address, (*stake).into());
    }
}

// ── Repeated-read stability ─────────────────────────────────────

#[test]
fn test_consensus_is_stable_across_repeated_reads() {
    let env = Env::default();
    let mut market = make_market(&env, outcomes(&env, &["a", "b", "c"]));
    cast_votes(
        &env,
        &mut market,
        &[("a", 100), ("a", 100), ("b", 100), ("c", 100)],
    );

    let first = MarketAnalytics::calculate_community_consensus(&market);
    for _ in 0..10 {
        let repeated = MarketAnalytics::calculate_community_consensus(&market);
        assert_eq!(repeated.outcome, first.outcome, "consensus outcome drifted");
        assert_eq!(repeated.votes, first.votes, "consensus vote count drifted");
        assert_eq!(
            repeated.total_votes, first.total_votes,
            "consensus total drifted"
        );
        assert_eq!(
            repeated.percentage, first.percentage,
            "consensus percentage drifted"
        );
    }

    // Clear majority: "a" wins with 2/4 votes = 50%.
    assert_eq!(first.outcome, String::from_str(&env, "a"));
    assert_eq!(first.votes, 2);
    assert_eq!(first.total_votes, 4);
    assert_eq!(first.percentage, 50);
}

// ── Tie-breaking determinism ────────────────────────────────────

#[test]
fn test_tie_breaks_deterministically_by_canonical_outcome_order() {
    let env = Env::default();
    // Exact three-way structure: "a" and "b" tie for the lead (2 each), "c" has 1.
    let mut market = make_market(&env, outcomes(&env, &["a", "b", "c"]));
    cast_votes(
        &env,
        &mut market,
        &[("a", 100), ("a", 100), ("b", 100), ("b", 100), ("c", 100)],
    );

    // "a" is first-listed among the tied leaders, so it must win the tie.
    let consensus = MarketAnalytics::calculate_community_consensus(&market);
    assert_eq!(consensus.outcome, String::from_str(&env, "a"));
    assert_eq!(consensus.votes, 2);
    assert_eq!(consensus.total_votes, 5);
    assert_eq!(consensus.percentage, 40);

    // Repeated reads never flip the chosen outcome.
    for _ in 0..10 {
        let repeated = MarketAnalytics::calculate_community_consensus(&market);
        assert_eq!(repeated.outcome, String::from_str(&env, "a"));
    }
}

#[test]
fn test_tie_break_is_independent_of_vote_insertion_order() {
    let env = Env::default();

    // Same final census, different insertion order into the underlying Map.
    let insert_orderings: [&[(&str, i128)]; 2] = [
        &[("a", 100), ("b", 100), ("a", 100), ("b", 100)],
        &[("b", 100), ("a", 100), ("b", 100), ("a", 100)],
    ];

    let mut outcomes_seen: Vec<String> = Vec::new(&env);
    for entries in insert_orderings.iter() {
        let mut market = make_market(&env, outcomes(&env, &["a", "b"]));
        cast_votes(&env, &mut market, entries);
        let consensus = MarketAnalytics::calculate_community_consensus(&market);
        outcomes_seen.push_back(consensus.outcome.clone());
    }

    // Both insertion orders must resolve to the same deterministic winner ("a").
    assert_eq!(outcomes_seen.len(), 2);
    assert_eq!(outcomes_seen.get(0).unwrap(), String::from_str(&env, "a"));
    assert_eq!(outcomes_seen.get(1).unwrap(), String::from_str(&env, "a"));
}

// ── Consistency between consensus and winning-outcome selection ─

#[test]
fn test_consensus_and_winning_outcomes_agree_on_ties() {
    let env = Env::default();
    let mut market = make_market(&env, outcomes(&env, &["a", "b", "c"]));
    cast_votes(
        &env,
        &mut market,
        &[("a", 100), ("b", 100), ("a", 50), ("b", 50)],
    );

    let consensus = MarketAnalytics::calculate_community_consensus(&market);
    let oracle = String::from_str(&env, "b");

    // With an exact vote tie between "a" and "b", the leader is "a" (canonical
    // order), but stakes are tied (150 each). The stake-based tie-break yields
    // BOTH as winners. Regardless of exact selection, repeated resolution must be
    // identical.
    let first = MarketUtils::determine_winning_outcomes(&env, &market, &oracle, &consensus, 0);
    assert_eq!(first.len(), 2, "vote-tied + stake-tied outcomes both win");
    for _ in 0..10 {
        let repeated =
            MarketUtils::determine_winning_outcomes(&env, &market, &oracle, &consensus, 0);
        assert_eq!(repeated.len(), first.len(), "winning outcome count drifted");
        for i in 0..first.len() {
            assert_eq!(
                repeated.get(i).unwrap(),
                first.get(i).unwrap(),
                "winning outcome set/order drifted"
            );
        }
    }
}

// ── Boundary cases ──────────────────────────────────────────────

#[test]
fn test_no_votes_yields_empty_deterministic_consensus() {
    let env = Env::default();
    let market = make_market(&env, outcomes(&env, &["yes", "no"]));

    let first = MarketAnalytics::calculate_community_consensus(&market);
    assert_eq!(first.total_votes, 0);
    assert_eq!(first.votes, 0);
    assert_eq!(first.percentage, 0);
    // No votes => no consensus outcome.
    assert!(first.outcome.is_empty());

    let second = MarketAnalytics::calculate_community_consensus(&market);
    assert_eq!(first.outcome, second.outcome);
    assert_eq!(first.votes, second.votes);
}

#[test]
fn test_single_outcome_unanimous_vote() {
    let env = Env::default();
    let mut market = make_market(&env, outcomes(&env, &["only"]));
    cast_votes(
        &env,
        &mut market,
        &[("only", 100), ("only", 100), ("only", 100)],
    );

    let consensus = MarketAnalytics::calculate_community_consensus(&market);
    assert_eq!(consensus.outcome, String::from_str(&env, "only"));
    assert_eq!(consensus.votes, 3);
    assert_eq!(consensus.total_votes, 3);
    assert_eq!(consensus.percentage, 100);

    // Regression: unanimous vote is stable across reads.
    let oracle = String::from_str(&env, "only");
    let first = MarketUtils::determine_winning_outcomes(&env, &market, &oracle, &consensus, 0);
    let second = MarketUtils::determine_winning_outcomes(&env, &market, &oracle, &consensus, 0);
    assert_eq!(first.len(), second.len());
    assert_eq!(first.len(), 1);
    assert_eq!(first.get(0).unwrap(), String::from_str(&env, "only"));
    assert_eq!(second.get(0).unwrap(), String::from_str(&env, "only"));
}

// ── Regression: clear majority behavior is unchanged ────────────

#[test]
fn test_clear_majority_still_selects_leader() {
    let env = Env::default();
    let mut market = make_market(&env, outcomes(&env, &["a", "b"]));
    cast_votes(
        &env,
        &mut market,
        &[("a", 100), ("a", 100), ("a", 100), ("b", 100)],
    );

    let consensus = MarketAnalytics::calculate_community_consensus(&market);
    assert_eq!(consensus.outcome, String::from_str(&env, "a"));
    assert_eq!(consensus.votes, 3);
    assert_eq!(consensus.total_votes, 4);
    assert_eq!(consensus.percentage, 75);
}
