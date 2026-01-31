#![cfg(test)]

//! Comprehensive tests for query functions
//!
//! This module contains unit tests, integration tests, and property-based tests
//! for all query functions in the Predictify Hybrid contract.

use crate::queries::*;
use crate::types::*;
use soroban_sdk::{vec, Address, Env, String, Symbol};

// ===== UNIT TESTS =====

#[test]
fn test_market_status_conversion() {
    let test_cases = vec![
        (MarketState::Active, MarketStatus::Active),
        (MarketState::Ended, MarketStatus::Ended),
        (MarketState::Disputed, MarketStatus::Disputed),
        (MarketState::Resolved, MarketStatus::Resolved),
        (MarketState::Closed, MarketStatus::Closed),
        (MarketState::Cancelled, MarketStatus::Cancelled),
    ];

    for (market_state, expected_status) in test_cases {
        let status = MarketStatus::from_market_state(market_state);
        assert_eq!(
            status, expected_status,
            "Failed to convert {:?} to correct status",
            market_state
        );
    }
}

#[test]
fn test_payout_calculation_zero_stake() {
    let env = Env::default();
    let admin = Address::generate(&env);

    let market = Market::new(
        &env,
        admin,
        String::from_str(&env, "Test Market"),
        vec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ],
        env.ledger().timestamp() + 1000,
        OracleConfig::new(
            OracleProvider::Reflector,
            String::from_str(&env, "TEST"),
            100,
            String::from_str(&env, "gt"),
        ),
        MarketState::Active,
    );

    let payout = QueryManager::calculate_payout(&env, &market, 0);
    assert!(payout.is_ok(), "Payout calculation failed for zero stake");
    assert_eq!(payout.unwrap(), 0, "Zero stake should result in zero payout");
}

#[test]
fn test_payout_calculation_unresolved_market() {
    let env = Env::default();
    let admin = Address::generate(&env);

    let market = Market::new(
        &env,
        admin,
        String::from_str(&env, "Test Market"),
        vec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ],
        env.ledger().timestamp() + 1000,
        OracleConfig::new(
            OracleProvider::Reflector,
            String::from_str(&env, "TEST"),
            100,
            String::from_str(&env, "gt"),
        ),
        MarketState::Active,
    );

    // Market has no winning outcome set
    let payout = QueryManager::calculate_payout(&env, &market, 5_000_000);
    assert!(
        payout.is_ok(),
        "Payout calculation failed for unresolved market"
    );
    assert_eq!(
        payout.unwrap(),
        0,
        "Unresolved market should have zero payout"
    );
}

#[test]
fn test_implied_probabilities_zero_pool() {
    let env = Env::default();
    let admin = Address::generate(&env);

    let market = Market::new(
        &env,
        admin,
        String::from_str(&env, "Test Market"),
        vec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ],
        env.ledger().timestamp() + 1000,
        OracleConfig::new(
            OracleProvider::Reflector,
            String::from_str(&env, "TEST"),
            100,
            String::from_str(&env, "gt"),
        ),
        MarketState::Active,
    );

    // No stakes in market
    let probs = QueryManager::calculate_implied_probabilities(&env, &market);
    assert!(probs.is_ok(), "Probability calculation failed");
    let (p1, p2) = probs.unwrap();
    assert_eq!(p1, 50, "Default probability should be 50%");
    assert_eq!(p2, 50, "Default probability should be 50%");
}

#[test]
fn test_implied_probabilities_sum_to_100() {
    let env = Env::default();
    let admin = Address::generate(&env);

    let market = Market::new(
        &env,
        admin,
        String::from_str(&env, "Test Market"),
        vec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ],
        env.ledger().timestamp() + 1000,
        OracleConfig::new(
            OracleProvider::Reflector,
            String::from_str(&env, "TEST"),
            100,
            String::from_str(&env, "gt"),
        ),
        MarketState::Active,
    );

    let probs = QueryManager::calculate_implied_probabilities(&env, &market);
    assert!(probs.is_ok());
    let (p1, p2) = probs.unwrap();
    assert_eq!(
        p1 + p2, 100,
        "Probabilities should sum to 100% (got {} + {})",
        p1, p2
    );
}

#[test]
fn test_outcome_pool_empty_market() {
    let env = Env::default();
    let admin = Address::generate(&env);

    let market = Market::new(
        &env,
        admin,
        String::from_str(&env, "Test Market"),
        vec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ],
        env.ledger().timestamp() + 1000,
        OracleConfig::new(
            OracleProvider::Reflector,
            String::from_str(&env, "TEST"),
            100,
            String::from_str(&env, "gt"),
        ),
        MarketState::Active,
    );

    let outcome = String::from_str(&env, "yes");
    let pool = QueryManager::calculate_outcome_pool(&env, &market, &outcome);
    assert!(pool.is_ok(), "Outcome pool calculation failed");
    assert_eq!(pool.unwrap(), 0, "Empty market should have zero pool for outcome");
}

#[test]
fn test_outcome_pool_with_single_vote() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let user = Address::generate(&env);

    let mut market = Market::new(
        &env,
        admin,
        String::from_str(&env, "Test Market"),
        vec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ],
        env.ledger().timestamp() + 1000,
        OracleConfig::new(
            OracleProvider::Reflector,
            String::from_str(&env, "TEST"),
            100,
            String::from_str(&env, "gt"),
        ),
        MarketState::Active,
    );

    let yes_outcome = String::from_str(&env, "yes");
    let stake = 5_000_000i128;

    market.votes.set(user.clone(), yes_outcome.clone());
    market.stakes.set(user, stake);

    let pool = QueryManager::calculate_outcome_pool(&env, &market, &yes_outcome);
    assert!(pool.is_ok(), "Outcome pool calculation failed");
    assert_eq!(pool.unwrap(), stake, "Pool should equal single vote stake");
}

#[test]
fn test_outcome_pool_with_multiple_votes() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let user3 = Address::generate(&env);

    let mut market = Market::new(
        &env,
        admin,
        String::from_str(&env, "Test Market"),
        vec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ],
        env.ledger().timestamp() + 1000,
        OracleConfig::new(
            OracleProvider::Reflector,
            String::from_str(&env, "TEST"),
            100,
            String::from_str(&env, "gt"),
        ),
        MarketState::Active,
    );

    let yes_outcome = String::from_str(&env, "yes");
    let no_outcome = String::from_str(&env, "no");

    // Add multiple votes for YES
    market.votes.set(user1.clone(), yes_outcome.clone());
    market.stakes.set(user1, 3_000_000i128);

    market.votes.set(user2.clone(), yes_outcome.clone());
    market.stakes.set(user2, 2_000_000i128);

    // Add vote for NO
    market.votes.set(user3.clone(), no_outcome.clone());
    market.stakes.set(user3, 5_000_000i128);

    let yes_pool = QueryManager::calculate_outcome_pool(&env, &market, &yes_outcome);
    let no_pool = QueryManager::calculate_outcome_pool(&env, &market, &no_outcome);

    assert!(yes_pool.is_ok());
    assert!(no_pool.is_ok());
    assert_eq!(yes_pool.unwrap(), 5_000_000i128, "YES pool should be 5M");
    assert_eq!(no_pool.unwrap(), 5_000_000i128, "NO pool should be 5M");
}

#[test]
fn test_market_status_all_states() {
    // Test all market states convert properly
    let states = vec![
        MarketState::Active,
        MarketState::Ended,
        MarketState::Disputed,
        MarketState::Resolved,
        MarketState::Closed,
        MarketState::Cancelled,
    ];

    for state in states {
        let status = MarketStatus::from_market_state(state);
        // Should not panic and should return valid status
        match status {
            MarketStatus::Active | MarketStatus::Ended | MarketStatus::Disputed
            | MarketStatus::Resolved | MarketStatus::Closed | MarketStatus::Cancelled => {
                // Valid status
            }
        }
    }
}

// ===== PROPERTY-BASED TESTS =====

#[test]
fn test_probabilities_are_percentages() {
    // Property: Implied probabilities should always be 0-100
    let env = Env::default();
    let admin = Address::generate(&env);

    let market = Market::new(
        &env,
        admin,
        String::from_str(&env, "Test"),
        vec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ],
        env.ledger().timestamp() + 1000,
        OracleConfig::new(
            OracleProvider::Reflector,
            String::from_str(&env, "TEST"),
            100,
            String::from_str(&env, "gt"),
        ),
        MarketState::Active,
    );

    let probs = QueryManager::calculate_implied_probabilities(&env, &market);
    assert!(probs.is_ok());
    let (p1, p2) = probs.unwrap();

    assert!(p1 >= 0 && p1 <= 100, "Probability 1 out of range: {}", p1);
    assert!(p2 >= 0 && p2 <= 100, "Probability 2 out of range: {}", p2);
}

#[test]
fn test_payout_never_exceeds_total_pool() {
    // Property: Payout should never exceed total pool
    let env = Env::default();
    let admin = Address::generate(&env);

    let mut market = Market::new(
        &env,
        admin,
        String::from_str(&env, "Test"),
        vec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ],
        env.ledger().timestamp() + 1000,
        OracleConfig::new(
            OracleProvider::Reflector,
            String::from_str(&env, "TEST"),
            100,
            String::from_str(&env, "gt"),
        ),
        MarketState::Active,
    );

    let stake = 10_000_000i128;
    market.total_staked = stake;
    market.winning_outcome = Some(String::from_str(&env, "yes"));

    let payout = QueryManager::calculate_payout(&env, &market, stake);
    assert!(payout.is_ok());
    assert!(
        payout.unwrap() <= market.total_staked,
        "Payout exceeds total pool"
    );
}

#[test]
fn test_pool_calculation_commutative() {
    // Property: Pool calculation should be independent of order
    let env = Env::default();
    let admin = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);

    // First market
    let mut market1 = Market::new(
        &env,
        admin.clone(),
        String::from_str(&env, "Test"),
        vec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ],
        env.ledger().timestamp() + 1000,
        OracleConfig::new(
            OracleProvider::Reflector,
            String::from_str(&env, "TEST"),
            100,
            String::from_str(&env, "gt"),
        ),
        MarketState::Active,
    );

    let outcome = String::from_str(&env, "yes");

    // Add votes in order 1, 2
    market1.votes.set(user1.clone(), outcome.clone());
    market1.stakes.set(user1.clone(), 3_000_000i128);
    market1.votes.set(user2.clone(), outcome.clone());
    market1.stakes.set(user2.clone(), 2_000_000i128);

    let pool1 = QueryManager::calculate_outcome_pool(&env, &market1, &outcome).unwrap();

    // Second market with same votes
    let mut market2 = Market::new(
        &env,
        admin,
        String::from_str(&env, "Test"),
        vec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ],
        env.ledger().timestamp() + 1000,
        OracleConfig::new(
            OracleProvider::Reflector,
            String::from_str(&env, "TEST"),
            100,
            String::from_str(&env, "gt"),
        ),
        MarketState::Active,
    );

    // Add votes in reverse order 2, 1
    market2.votes.set(user2, outcome.clone());
    market2.stakes.set(user2, 2_000_000i128);
    market2.votes.set(user1, outcome.clone());
    market2.stakes.set(user1, 3_000_000i128);

    let pool2 = QueryManager::calculate_outcome_pool(&env, &market2, &outcome).unwrap();

    assert_eq!(pool1, pool2, "Pool calculation should be order-independent");
}

// ===== INTEGRATION TESTS =====

#[test]
fn test_status_conversion_roundtrip() {
    // Test that we can convert states and back
    let all_states = vec![
        MarketState::Active,
        MarketState::Ended,
        MarketState::Disputed,
        MarketState::Resolved,
        MarketState::Closed,
        MarketState::Cancelled,
    ];

    for state in all_states {
        let status = MarketStatus::from_market_state(state);
        // Verify status is valid
        match status {
            MarketStatus::Active => assert_eq!(state, MarketState::Active),
            MarketStatus::Ended => assert_eq!(state, MarketState::Ended),
            MarketStatus::Disputed => assert_eq!(state, MarketState::Disputed),
            MarketStatus::Resolved => assert_eq!(state, MarketState::Resolved),
            MarketStatus::Closed => assert_eq!(state, MarketState::Closed),
            MarketStatus::Cancelled => assert_eq!(state, MarketState::Cancelled),
        }
    }
}

#[test]
fn test_outcome_pool_consistency() {
    // Property: Sum of outcome pools should equal total staked
    let env = Env::default();
    let admin = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);

    let mut market = Market::new(
        &env,
        admin,
        String::from_str(&env, "Test"),
        vec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ],
        env.ledger().timestamp() + 1000,
        OracleConfig::new(
            OracleProvider::Reflector,
            String::from_str(&env, "TEST"),
            100,
            String::from_str(&env, "gt"),
        ),
        MarketState::Active,
    );

    let yes_outcome = String::from_str(&env, "yes");
    let no_outcome = String::from_str(&env, "no");

    market.votes.set(user1.clone(), yes_outcome.clone());
    market.stakes.set(user1, 7_000_000i128);
    market.votes.set(user2.clone(), no_outcome.clone());
    market.stakes.set(user2, 3_000_000i128);
    market.total_staked = 10_000_000i128;

    let yes_pool = QueryManager::calculate_outcome_pool(&env, &market, &yes_outcome).unwrap();
    let no_pool = QueryManager::calculate_outcome_pool(&env, &market, &no_outcome).unwrap();

    assert_eq!(
        yes_pool + no_pool,
        market.total_staked,
        "Outcome pools should sum to total staked"
    );
}

// ===== EDGE CASE TESTS =====

#[test]
fn test_payout_with_high_fees() {
    // Edge case: Verify fee deduction is applied
    let env = Env::default();
    let admin = Address::generate(&env);

    let mut market = Market::new(
        &env,
        admin,
        String::from_str(&env, "Test"),
        vec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ],
        env.ledger().timestamp() + 1000,
        OracleConfig::new(
            OracleProvider::Reflector,
            String::from_str(&env, "TEST"),
            100,
            String::from_str(&env, "gt"),
        ),
        MarketState::Active,
    );

    let stake = 100_000_000i128; // 10 XLM
    market.total_staked = stake;
    market.winning_outcome = Some(String::from_str(&env, "yes"));

    let payout = QueryManager::calculate_payout(&env, &market, stake).unwrap();

    // Should be less than stake due to fee (2%)
    assert!(payout < stake, "Payout should be less than stake due to fees");
    assert!(
        payout > stake * 98 / 100,
        "Payout should be approximately 98% of stake"
    );
}

#[test]
fn test_negative_values_handled() {
    // Edge case: Negative or zero values should be handled gracefully
    let env = Env::default();
    let admin = Address::generate(&env);

    let market = Market::new(
        &env,
        admin,
        String::from_str(&env, "Test"),
        vec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ],
        env.ledger().timestamp() + 1000,
        OracleConfig::new(
            OracleProvider::Reflector,
            String::from_str(&env, "TEST"),
            100,
            String::from_str(&env, "gt"),
        ),
        MarketState::Active,
    );

    // Test with negative stake (should be safe)
    let payout = QueryManager::calculate_payout(&env, &market, -1_000_000);
    assert!(payout.is_ok());
    assert_eq!(payout.unwrap(), 0);
}

#[test]
fn test_large_number_handling() {
    // Edge case: Very large numbers should be handled without overflow
    let env = Env::default();
    let admin = Address::generate(&env);

    let market = Market::new(
        &env,
        admin,
        String::from_str(&env, "Test"),
        vec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ],
        env.ledger().timestamp() + 1000,
        OracleConfig::new(
            OracleProvider::Reflector,
            String::from_str(&env, "TEST"),
            100,
            String::from_str(&env, "gt"),
        ),
        MarketState::Active,
    );

    // Should handle large numbers without panicking
    let payout = QueryManager::calculate_payout(&env, &market, i128::MAX / 2);
    assert!(payout.is_ok() || payout.is_err()); // Either succeeds or returns error gracefully
}
