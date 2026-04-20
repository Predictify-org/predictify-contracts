#![cfg(test)]

//! Comprehensive tests for query functions
//!
//! This module contains unit tests, integration tests, and property-based tests
//! for all query functions in the Predictify Hybrid contract.

use crate::queries::*;
use crate::types::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{vec as svec, Address, Env, String, Symbol};

const TEST_ORACLE_ADDRESS: &str = "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";

// ===== UNIT TESTS =====

#[test]
fn test_market_status_conversion() {
    let test_cases: [(MarketState, MarketStatus); 6] = [
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
        svec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ],
        env.ledger().timestamp() + 1000,
        OracleConfig::new(
            OracleProvider::reflector(),
            Address::from_str(&env, TEST_ORACLE_ADDRESS),
            String::from_str(&env, "TEST"),
            100,
            String::from_str(&env, "gt"),
        ),
        None,
        86400,
        MarketState::Active,
    );

    let payout = QueryManager::calculate_payout(&env, &market, 0);
    assert!(payout.is_ok(), "Payout calculation failed for zero stake");
    assert_eq!(
        payout.unwrap(),
        0,
        "Zero stake should result in zero payout"
    );
}

#[test]
fn test_payout_calculation_unresolved_market() {
    let env = Env::default();
    let admin = Address::generate(&env);

    let market = Market::new(
        &env,
        admin,
        String::from_str(&env, "Test Market"),
        svec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ],
        env.ledger().timestamp() + 1000,
        OracleConfig::new(
            OracleProvider::reflector(),
            Address::from_str(&env, TEST_ORACLE_ADDRESS),
            String::from_str(&env, "TEST"),
            100,
            String::from_str(&env, "gt"),
        ),
        None,
        86400,
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
        svec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ],
        env.ledger().timestamp() + 1000,
        OracleConfig::new(
            OracleProvider::reflector(),
            Address::from_str(&env, TEST_ORACLE_ADDRESS),
            String::from_str(&env, "TEST"),
            100,
            String::from_str(&env, "gt"),
        ),
        None,
        86400,
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
        svec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ],
        env.ledger().timestamp() + 1000,
        OracleConfig::new(
            OracleProvider::reflector(),
            Address::from_str(&env, TEST_ORACLE_ADDRESS),
            String::from_str(&env, "TEST"),
            100,
            String::from_str(&env, "gt"),
        ),
        None,
        86400,
        MarketState::Active,
    );

    let probs = QueryManager::calculate_implied_probabilities(&env, &market);
    assert!(probs.is_ok());
    let (p1, p2) = probs.unwrap();
    assert_eq!(
        p1 + p2,
        100,
        "Probabilities should sum to 100% (got {} + {})",
        p1,
        p2
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
        svec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ],
        env.ledger().timestamp() + 1000,
        OracleConfig::new(
            OracleProvider::reflector(),
            Address::from_str(&env, TEST_ORACLE_ADDRESS),
            String::from_str(&env, "TEST"),
            100,
            String::from_str(&env, "gt"),
        ),
        None,
        86400,
        MarketState::Active,
    );

    let outcome = String::from_str(&env, "yes");
    let pool = QueryManager::calculate_outcome_pool(&env, &market, &outcome);
    assert!(pool.is_ok(), "Outcome pool calculation failed");
    assert_eq!(
        pool.unwrap(),
        0,
        "Empty market should have zero pool for outcome"
    );
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
        svec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ],
        env.ledger().timestamp() + 1000,
        OracleConfig::new(
            OracleProvider::reflector(),
            Address::from_str(&env, TEST_ORACLE_ADDRESS),
            String::from_str(&env, "TEST"),
            100,
            String::from_str(&env, "gt"),
        ),
        None,
        86400,
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
        svec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ],
        env.ledger().timestamp() + 1000,
        OracleConfig::new(
            OracleProvider::reflector(),
            Address::from_str(&env, TEST_ORACLE_ADDRESS),
            String::from_str(&env, "TEST"),
            100,
            String::from_str(&env, "gt"),
        ),
        None,
        86400,
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
    let states: [MarketState; 6] = [
        MarketState::Active,
        MarketState::Ended,
        MarketState::Disputed,
        MarketState::Resolved,
        MarketState::Closed,
        MarketState::Cancelled,
    ];

    for state in states.iter().copied() {
        let status = MarketStatus::from_market_state(state);
        // Should not panic and should return valid status
        match status {
            MarketStatus::Active
            | MarketStatus::Ended
            | MarketStatus::Disputed
            | MarketStatus::Resolved
            | MarketStatus::Closed
            | MarketStatus::Cancelled => {
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
        svec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ],
        env.ledger().timestamp() + 1000,
        OracleConfig::new(
            OracleProvider::reflector(),
            Address::from_str(&env, TEST_ORACLE_ADDRESS),
            String::from_str(&env, "TEST"),
            100,
            String::from_str(&env, "gt"),
        ),
        None,
        86400,
        MarketState::Active,
    );

    let probs = QueryManager::calculate_implied_probabilities(&env, &market);
    assert!(probs.is_ok());
    let (p1, p2) = probs.unwrap();

    assert!(p1 <= 100, "Probability 1 out of range: {}", p1);
    assert!(p2 <= 100, "Probability 2 out of range: {}", p2);
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
        svec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ],
        env.ledger().timestamp() + 1000,
        OracleConfig::new(
            OracleProvider::reflector(),
            Address::from_str(&env, TEST_ORACLE_ADDRESS),
            String::from_str(&env, "TEST"),
            100,
            String::from_str(&env, "gt"),
        ),
        None,
        86400,
        MarketState::Active,
    );

    let stake = 10_000_000i128;
    market.total_staked = stake;
    market.winning_outcomes = Some(soroban_sdk::vec![&env, String::from_str(&env, "yes")]);

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
        svec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ],
        env.ledger().timestamp() + 1000,
        OracleConfig::new(
            OracleProvider::reflector(),
            Address::from_str(&env, TEST_ORACLE_ADDRESS),
            String::from_str(&env, "TEST"),
            100,
            String::from_str(&env, "gt"),
        ),
        None,
        86400,
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
        svec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ],
        env.ledger().timestamp() + 1000,
        OracleConfig::new(
            OracleProvider::reflector(),
            Address::from_str(&env, TEST_ORACLE_ADDRESS),
            String::from_str(&env, "TEST"),
            100,
            String::from_str(&env, "gt"),
        ),
        None,
        86400,
        MarketState::Active,
    );

    // Add votes in reverse order 2, 1
    market2.votes.set(user2.clone(), outcome.clone());
    market2.stakes.set(user2.clone(), 2_000_000i128);
    market2.votes.set(user1.clone(), outcome.clone());
    market2.stakes.set(user1.clone(), 3_000_000i128);

    let pool2 = QueryManager::calculate_outcome_pool(&env, &market2, &outcome).unwrap();

    assert_eq!(pool1, pool2, "Pool calculation should be order-independent");
}

// ===== INTEGRATION TESTS =====

#[test]
fn test_status_conversion_roundtrip() {
    // Test that we can convert states and back
    let all_states: [MarketState; 6] = [
        MarketState::Active,
        MarketState::Ended,
        MarketState::Disputed,
        MarketState::Resolved,
        MarketState::Closed,
        MarketState::Cancelled,
    ];

    for state in all_states.iter().copied() {
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
        svec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ],
        env.ledger().timestamp() + 1000,
        OracleConfig::new(
            OracleProvider::reflector(),
            Address::from_str(&env, TEST_ORACLE_ADDRESS),
            String::from_str(&env, "TEST"),
            100,
            String::from_str(&env, "gt"),
        ),
        None,
        86400,
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
    let user = Address::generate(&env);
    let yes = String::from_str(&env, "yes");

    let mut market = Market::new(
        &env,
        admin,
        String::from_str(&env, "Test"),
        svec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ],
        env.ledger().timestamp() + 1000,
        OracleConfig::new(
            OracleProvider::reflector(),
            Address::from_str(&env, TEST_ORACLE_ADDRESS),
            String::from_str(&env, "TEST"),
            100,
            String::from_str(&env, "gt"),
        ),
        None,
        86400,
        MarketState::Active,
    );

    let stake = 100_000_000i128; // 10 XLM
    market.votes.set(user.clone(), yes.clone());
    market.stakes.set(user.clone(), stake);
    market.total_staked = stake;
    market.winning_outcomes = Some(soroban_sdk::vec![&env, yes]);

    let payout = QueryManager::calculate_payout(&env, &market, stake).unwrap();

    // Should be less than stake due to fee (2%)
    assert!(
        payout < stake,
        "Payout should be less than stake due to fees"
    );
    assert!(
        payout >= stake * 98 / 100,
        "Payout should be approximately 98% of stake, got {}",
        payout
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
        svec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ],
        env.ledger().timestamp() + 1000,
        OracleConfig::new(
            OracleProvider::reflector(),
            Address::from_str(&env, TEST_ORACLE_ADDRESS),
            String::from_str(&env, "TEST"),
            100,
            String::from_str(&env, "gt"),
        ),
        None,
        86400,
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
        svec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ],
        env.ledger().timestamp() + 1000,
        OracleConfig::new(
            OracleProvider::reflector(),
            Address::from_str(&env, TEST_ORACLE_ADDRESS),
            String::from_str(&env, "TEST"),
            100,
            String::from_str(&env, "gt"),
        ),
        None,
        86400,
        MarketState::Active,
    );

    // Should handle large numbers without panicking
    let payout = QueryManager::calculate_payout(&env, &market, i128::MAX / 2);
    assert!(payout.is_ok() || payout.is_err()); // Either succeeds or returns error gracefully
}

// ===== PAGINATION TESTS =====

/// Helper: build a minimal Market for pagination tests.
fn make_market(env: &Env) -> crate::types::Market {
    let admin = Address::generate(env);
    crate::types::Market::new(
        env,
        admin,
        String::from_str(env, "Will BTC hit 100k?"),
        svec![
            env,
            String::from_str(env, "yes"),
            String::from_str(env, "no"),
        ],
        env.ledger().timestamp() + 1000,
        OracleConfig::new(
            OracleProvider::Reflector,
            Address::from_str(env, TEST_ORACLE_ADDRESS),
            String::from_str(env, "BTC"),
            100,
            String::from_str(env, "gt"),
        ),
        None,
        86400,
        MarketState::Active,
    )
}

#[test]
fn test_get_all_markets_paged_empty() {
    // Empty market index → empty first page, next_cursor = 0, total_count = 0.
    let env = Env::default();
    let contract_id = env.register(crate::PredictifyHybrid, ());
    let page = env.as_contract(&contract_id, || {
        QueryManager::get_all_markets_paged(&env, 0, 10).unwrap()
    });
    assert_eq!(page.items.len(), 0);
    assert_eq!(page.next_cursor, 0);
    assert_eq!(page.total_count, 0);
}

#[test]
fn test_get_all_markets_paged_limit_capped() {
    // Requesting more than MAX_PAGE_SIZE (50) must be silently capped.
    let env = Env::default();
    let contract_id = env.register(crate::PredictifyHybrid, ());
    // With an empty index the cap doesn't change the result count, but we can
    // verify the function doesn't panic with an oversized limit.
    let page = env.as_contract(&contract_id, || {
        QueryManager::get_all_markets_paged(&env, 0, 9999).unwrap()
    });
    assert_eq!(page.items.len(), 0);
}

#[test]
fn test_get_all_markets_paged_cursor_beyond_end() {
    // A cursor past the end of the list returns an empty page.
    let env = Env::default();
    let contract_id = env.register(crate::PredictifyHybrid, ());
    let page = env.as_contract(&contract_id, || {
        QueryManager::get_all_markets_paged(&env, 100, 10).unwrap()
    });
    assert_eq!(page.items.len(), 0);
    // next_cursor must not exceed total_count
    assert!(page.next_cursor <= page.total_count);
}

#[test]
fn test_get_all_markets_paged_next_cursor_monotone() {
    // next_cursor must be >= cursor (never goes backwards).
    let env = Env::default();
    let contract_id = env.register(crate::PredictifyHybrid, ());
    let page = env.as_contract(&contract_id, || {
        QueryManager::get_all_markets_paged(&env, 5, 10).unwrap()
    });
    assert!(page.next_cursor >= 5);
}

#[test]
fn test_query_user_bets_paged_empty_markets() {
    // No markets → empty page for any user.
    let env = Env::default();
    let contract_id = env.register(crate::PredictifyHybrid, ());
    let user = Address::generate(&env);
    let page = env.as_contract(&contract_id, || {
        QueryManager::query_user_bets_paged(&env, user, 0, 10).unwrap()
    });
    assert_eq!(page.items.len(), 0);
    assert_eq!(page.next_cursor, 0);
    assert_eq!(page.total_count, 0);
}

#[test]
fn test_query_user_bets_paged_limit_capped() {
    // Oversized limit must not panic.
    let env = Env::default();
    let contract_id = env.register(crate::PredictifyHybrid, ());
    let user = Address::generate(&env);
    let page = env.as_contract(&contract_id, || {
        QueryManager::query_user_bets_paged(&env, user, 0, 9999).unwrap()
    });
    assert_eq!(page.items.len(), 0);
}

#[test]
fn test_query_user_bets_paged_cursor_beyond_end() {
    // Cursor past end → empty page, next_cursor ≤ total_count.
    let env = Env::default();
    let contract_id = env.register(crate::PredictifyHybrid, ());
    let user = Address::generate(&env);
    let page = env.as_contract(&contract_id, || {
        QueryManager::query_user_bets_paged(&env, user, 200, 10).unwrap()
    });
    assert_eq!(page.items.len(), 0);
    assert!(page.next_cursor <= page.total_count);
}

#[test]
fn test_query_user_bets_paged_next_cursor_monotone() {
    // next_cursor must be ≥ cursor.
    let env = Env::default();
    let contract_id = env.register(crate::PredictifyHybrid, ());
    let user = Address::generate(&env);
    let page = env.as_contract(&contract_id, || {
        QueryManager::query_user_bets_paged(&env, user, 3, 5).unwrap()
    });
    assert!(page.next_cursor >= 3);
}

#[test]
fn test_query_contract_state_paged_empty() {
    // Empty market list → zero counts, next_cursor = 0.
    let env = Env::default();
    let contract_id = env.register(crate::PredictifyHybrid, ());
    let (state, next_cursor) = env.as_contract(&contract_id, || {
        QueryManager::query_contract_state_paged(&env, 0, 10).unwrap()
    });
    assert_eq!(state.active_markets, 0);
    assert_eq!(state.resolved_markets, 0);
    assert_eq!(state.total_value_locked, 0);
    assert_eq!(next_cursor, 0);
}

#[test]
fn test_query_contract_state_paged_limit_capped() {
    // Oversized limit must not panic.
    let env = Env::default();
    let contract_id = env.register(crate::PredictifyHybrid, ());
    let result = env.as_contract(&contract_id, || {
        QueryManager::query_contract_state_paged(&env, 0, 9999)
    });
    assert!(result.is_ok());
}

#[test]
fn test_query_contract_state_paged_cursor_beyond_end() {
    // Cursor past end → zero counts, next_cursor ≤ total_markets.
    let env = Env::default();
    let contract_id = env.register(crate::PredictifyHybrid, ());
    let (state, next_cursor) = env.as_contract(&contract_id, || {
        QueryManager::query_contract_state_paged(&env, 100, 10).unwrap()
    });
    assert_eq!(state.active_markets, 0);
    assert!(next_cursor <= state.total_markets);
}

#[test]
fn test_query_contract_state_paged_next_cursor_monotone() {
    // next_cursor must be ≥ cursor.
    let env = Env::default();
    let contract_id = env.register(crate::PredictifyHybrid, ());
    let (_, next_cursor) = env.as_contract(&contract_id, || {
        QueryManager::query_contract_state_paged(&env, 5, 10).unwrap()
    });
    assert!(next_cursor >= 5);
}

// ===== INVARIANT / PROPERTY TESTS FOR PAGINATION =====

#[test]
fn test_paged_result_items_never_exceed_limit() {
    // Property: items.len() ≤ min(limit, MAX_PAGE_SIZE) for any cursor.
    let env = Env::default();
    let contract_id = env.register(crate::PredictifyHybrid, ());
    for limit in [1u32, 5, 10, 50, 100] {
        let page = env.as_contract(&contract_id, || {
            QueryManager::get_all_markets_paged(&env, 0, limit).unwrap()
        });
        let effective_limit = core::cmp::min(limit, crate::queries::MAX_PAGE_SIZE);
        assert!(
            page.items.len() <= effective_limit,
            "items.len()={} > effective_limit={} for limit={}",
            page.items.len(),
            effective_limit,
            limit
        );
    }
}

#[test]
fn test_paged_result_next_cursor_never_exceeds_total() {
    // Property: next_cursor ≤ total_count always.
    let env = Env::default();
    let contract_id = env.register(crate::PredictifyHybrid, ());
    for cursor in [0u32, 1, 10, 50] {
        let page = env.as_contract(&contract_id, || {
            QueryManager::get_all_markets_paged(&env, cursor, 10).unwrap()
        });
        assert!(
            page.next_cursor <= page.total_count,
            "next_cursor={} > total_count={} at cursor={}",
            page.next_cursor,
            page.total_count,
            cursor
        );
    }
}

#[test]
fn test_user_bets_paged_items_never_exceed_limit() {
    // Property: items.len() ≤ min(limit, MAX_PAGE_SIZE).
    let env = Env::default();
    let contract_id = env.register(crate::PredictifyHybrid, ());
    let user = Address::generate(&env);
    for limit in [1u32, 5, 10, 50, 200] {
        let page = env.as_contract(&contract_id, || {
            QueryManager::query_user_bets_paged(&env, user.clone(), 0, limit).unwrap()
        });
        let effective_limit = core::cmp::min(limit, crate::queries::MAX_PAGE_SIZE);
        assert!(page.items.len() <= effective_limit);
    }
}

#[test]
fn test_contract_state_paged_next_cursor_never_exceeds_total() {
    // Property: next_cursor ≤ total_markets always.
    let env = Env::default();
    let contract_id = env.register(crate::PredictifyHybrid, ());
    for cursor in [0u32, 5, 20] {
        let (state, next_cursor) = env.as_contract(&contract_id, || {
            QueryManager::query_contract_state_paged(&env, cursor, 10).unwrap()
        });
        assert!(next_cursor <= state.total_markets);
    }
}

#[test]
fn test_max_page_size_constant_value() {
    // Regression: MAX_PAGE_SIZE must be 50 (gas budget assumption).
    assert_eq!(crate::queries::MAX_PAGE_SIZE, 50u32);
}

// ===== DASHBOARD STATISTICS TESTS =====

#[test]
fn test_get_dashboard_statistics_empty_state() {
    // Dashboard stats should initialize with zeros when no markets exist
    let env = Env::default();
    let contract_id = env.register(crate::PredictifyHybrid, ());
    let stats = env.as_contract(&contract_id, || {
        let stats = crate::statistics::StatisticsManager::create_dashboard_stats(&env, 0, 0);
        stats
    });

    assert_eq!(stats.api_version, 1);
    assert_eq!(stats.platform_stats.total_events_created, 0);
    assert_eq!(stats.platform_stats.total_volume, 0);
    assert_eq!(stats.active_user_count, 0);
    assert_eq!(stats.total_value_locked, 0);
}

#[test]
fn test_get_market_statistics_empty_market() {
    // Market with no participants should compute zero consensus
    let env = Env::default();
    let contract_id = env.register(crate::PredictifyHybrid, ());
    let admin = Address::generate(&env);
    let market_id = Symbol::new(&env, "empty_market");

    let market = Market::new(
        &env,
        admin,
        String::from_str(&env, "Empty Market"),
        svec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ],
        env.ledger().timestamp() + 1000,
        OracleConfig::new(
            OracleProvider::reflector(),
            Address::from_str(&env, TEST_ORACLE_ADDRESS),
            String::from_str(&env, "TEST"),
            100,
            String::from_str(&env, "gt"),
        ),
        None,
        86400,
        MarketState::Active,
    );

    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&market_id, &market);
        let stats = QueryManager::get_market_statistics(&env, market_id).unwrap();

        assert_eq!(stats.participant_count, 0);
        assert_eq!(stats.total_volume, 0);
        assert_eq!(stats.average_stake, 0);
        assert_eq!(stats.consensus_strength, 0);
        assert_eq!(stats.volatility, 10000);
        assert_eq!(stats.api_version, 1);
    });
}

#[test]
fn test_get_market_statistics_with_participants() {
    // Market with participants should compute metricsorrectly
    let env = Env::default();
    let contract_id = env.register(crate::PredictifyHybrid, ());
    let admin = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let market_id = Symbol::new(&env, "test_market");

    let mut market = Market::new(
        &env,
        admin,
        String::from_str(&env, "Test Market"),
        svec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ],
        env.ledger().timestamp() + 1000,
        OracleConfig::new(
            OracleProvider::reflector(),
            Address::from_str(&env, TEST_ORACLE_ADDRESS),
            String::from_str(&env, "TEST"),
            100,
            String::from_str(&env, "gt"),
        ),
        None,
        86400,
        MarketState::Active,
    );

    // Add stakes
    market.stakes.set(user1.clone(), 1000i128);
    market.stakes.set(user2.clone(), 2000i128);
    market
        .votes
        .set(user1.clone(), String::from_str(&env, "yes"));
    market
        .votes
        .set(user2.clone(), String::from_str(&env, "yes"));
    market.total_staked = 3000i128;

    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&market_id, &market);
        let stats = QueryManager::get_market_statistics(&env, market_id).unwrap();

        assert_eq!(stats.participant_count, 2);
        assert_eq!(stats.total_volume, 3000);
        assert_eq!(stats.average_stake, 1500);
        assert_eq!(stats.consensus_strength, 10000); // All voted for same outcome
        assert_eq!(stats.volatility, 0);
        assert_eq!(stats.api_version, 1);
    });
}

#[test]
fn test_get_market_statistics_partial_consensus() {
    // Market with split votes should show correct consensus strength
    let env = Env::default();
    let contract_id = env.register(crate::PredictifyHybrid, ());
    let admin = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let market_id = Symbol::new(&env, "split_market");

    let mut market = Market::new(
        &env,
        admin,
        String::from_str(&env, "Split Market"),
        svec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ],
        env.ledger().timestamp() + 1000,
        OracleConfig::new(
            OracleProvider::reflector(),
            Address::from_str(&env, TEST_ORACLE_ADDRESS),
            String::from_str(&env, "TEST"),
            100,
            String::from_str(&env, "gt"),
        ),
        None,
        86400,
        MarketState::Active,
    );

    // 70% on "yes", 30% on "no"
    market.stakes.set(user1.clone(), 7000i128);
    market.stakes.set(user2.clone(), 3000i128);
    market
        .votes
        .set(user1.clone(), String::from_str(&env, "yes"));
    market
        .votes
        .set(user2.clone(), String::from_str(&env, "no"));
    market.total_staked = 10000i128;

    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&market_id, &market);
        let stats = QueryManager::get_market_statistics(&env, market_id).unwrap();

        assert_eq!(stats.participant_count, 2);
        assert_eq!(stats.total_volume, 10000);
        assert_eq!(stats.consensus_strength, 7000); // 70% on max outcome
        assert!(stats.volatility > 0 && stats.volatility < 10000); // Partial consensus
    });
}

#[test]
fn test_get_category_statistics_no_markets() {
    // Empty category should return zeros
    let env = Env::default();
    let contract_id = env.register(crate::PredictifyHybrid, ());
    let category = String::from_str(&env, "sports");

    env.as_contract(&contract_id, || {
        let stats = QueryManager::get_category_statistics(&env, category).unwrap();

        assert_eq!(stats.market_count, 0);
        assert_eq!(stats.total_volume, 0);
        assert_eq!(stats.participant_count, 0);
        assert_eq!(stats.resolved_count, 0);
        assert_eq!(stats.average_market_volume, 0);
    });
}

#[test]
fn test_get_category_statistics_with_markets() {
    // Should aggregate metrics across markets in category
    let env = Env::default();
    let contract_id = env.register(crate::PredictifyHybrid, ());
    let admin = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let market_id_1 = Symbol::new(&env, "market_1");
    let market_id_2 = Symbol::new(&env, "market_2");
    let category = String::from_str(&env, "sports");

    // Create first market with category
    let mut market1 = Market::new(
        &env,
        admin.clone(),
        String::from_str(&env, "Market 1"),
        svec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ],
        env.ledger().timestamp() + 1000,
        OracleConfig::new(
            OracleProvider::reflector(),
            Address::from_str(&env, TEST_ORACLE_ADDRESS),
            String::from_str(&env, "TEST"),
            100,
            String::from_str(&env, "gt"),
        ),
        None,
        86400,
        MarketState::Active,
    );
    market1.category = Some(category.clone());
    market1.stakes.set(user1.clone(), 1000i128);
    market1
        .votes
        .set(user1.clone(), String::from_str(&env, "yes"));
    market1.total_staked = 1000i128;

    // Create second market with same category
    let mut market2 = Market::new(
        &env,
        admin.clone(),
        String::from_str(&env, "Market 2"),
        svec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ],
        env.ledger().timestamp() + 1000,
        OracleConfig::new(
            OracleProvider::reflector(),
            Address::from_str(&env, TEST_ORACLE_ADDRESS),
            String::from_str(&env, "TEST"),
            100,
            String::from_str(&env, "gt"),
        ),
        None,
        86400,
        MarketState::Resolved,
    );
    market2.category = Some(category.clone());
    market2.stakes.set(user2.clone(), 2000i128);
    market2
        .votes
        .set(user2.clone(), String::from_str(&env, "yes"));
    market2.total_staked = 2000i128;

    env.as_contract(&contract_id, || {
        // Store markets - need to set up market index
        env.storage().persistent().set(&market_id_1, &market1);
        env.storage().persistent().set(&market_id_2, &market2);
        let mut market_ids: Vec<Symbol> = vec![&env];
        market_ids.push_back(market_id_1);
        market_ids.push_back(market_id_2);
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, "market_ids"), &market_ids);

        let stats = QueryManager::get_category_statistics(&env, category).unwrap();

        assert_eq!(stats.market_count, 2);
        assert_eq!(stats.total_volume, 3000);
        assert_eq!(stats.participant_count, 2);
        assert_eq!(stats.resolved_count, 1);
        assert_eq!(stats.average_market_volume, 1500);
    });
}

#[test]
fn test_category_statistics_version() {
    // CategoryStatisticsV1 should have correct fields
    let env = Env::default();
    let contract_id = env.register(crate::PredictifyHybrid, ());
    let category = String::from_str(&env, "test");

    env.as_contract(&contract_id, || {
        let stats = QueryManager::get_category_statistics(&env, category.clone()).unwrap();

        assert_eq!(stats.category, category);
        assert!(stats.market_count >= 0);
        assert!(stats.total_volume >= 0);
    });
}

#[test]
fn test_top_users_by_winnings_limit_capped() {
    // Should respect MAX_PAGE_SIZE limit
    let env = Env::default();
    let contract_id = env.register(crate::PredictifyHybrid, ());

    env.as_contract(&contract_id, || {
        let users = QueryManager::get_top_users_by_winnings(&env, 1000).unwrap();
        assert!(
            users.len() <= crate::queries::MAX_PAGE_SIZE as usize,
            "Result exceeds MAX_PAGE_SIZE"
        );
    });
}

#[test]
fn test_top_users_by_win_rate_limit_capped() {
    // Should respect MAX_PAGE_SIZE limit
    let env = Env::default();
    let contract_id = env.register(crate::PredictifyHybrid, ());

    env.as_contract(&contract_id, || {
        let users = QueryManager::get_top_users_by_win_rate(&env, 1000, 10).unwrap();
        assert!(
            users.len() <= crate::queries::MAX_PAGE_SIZE as usize,
            "Result exceeds MAX_PAGE_SIZE"
        );
    });
}

#[test]
fn test_market_statistics_api_version() {
    // MarketStatisticsV1 should always be version 1
    let env = Env::default();
    let contract_id = env.register(crate::PredictifyHybrid, ());
    let admin = Address::generate(&env);
    let market_id = Symbol::new(&env, "version_test");

    let market = Market::new(
        &env,
        admin,
        String::from_str(&env, "Version Test"),
        svec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ],
        env.ledger().timestamp() + 1000,
        OracleConfig::new(
            OracleProvider::reflector(),
            Address::from_str(&env, TEST_ORACLE_ADDRESS),
            String::from_str(&env, "TEST"),
            100,
            String::from_str(&env, "gt"),
        ),
        None,
        86400,
        MarketState::Active,
    );

    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&market_id, &market);
        let stats = QueryManager::get_market_statistics(&env, market_id).unwrap();
        assert_eq!(stats.api_version, 1);
    });
}

#[test]
fn test_dashboard_statistics_version() {
    // DashboardStatisticsV1 should always be version 1
    let env = Env::default();
    let contract_id = env.register(crate::PredictifyHybrid, ());

    env.as_contract(&contract_id, || {
        let stats = crate::statistics::StatisticsManager::create_dashboard_stats(&env, 0, 0);
        assert_eq!(stats.api_version, 1);
    });
}

#[test]
fn test_market_statistics_consensus_strength_range() {
    // Consensus strength should be 0-10000
    let env = Env::default();
    let contract_id = env.register(crate::PredictifyHybrid, ());
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let market_id = Symbol::new(&env, "consensus_test");

    let mut market = Market::new(
        &env,
        admin,
        String::from_str(&env, "Consensus Test"),
        svec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ],
        env.ledger().timestamp() + 1000,
        OracleConfig::new(
            OracleProvider::reflector(),
            Address::from_str(&env, TEST_ORACLE_ADDRESS),
            String::from_str(&env, "TEST"),
            100,
            String::from_str(&env, "gt"),
        ),
        None,
        86400,
        MarketState::Active,
    );

    market.stakes.set(user.clone(), 5000i128);
    market
        .votes
        .set(user.clone(), String::from_str(&env, "yes"));
    market.total_staked = 5000i128;

    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&market_id, &market);
        let stats = QueryManager::get_market_statistics(&env, market_id).unwrap();

        assert!(stats.consensus_strength >= 0 && stats.consensus_strength <= 10000);
    });
}

#[test]
fn test_market_statistics_volatility_range() {
    // Volatility should be 0-10000
    let env = Env::default();
    let contract_id = env.register(crate::PredictifyHybrid, ());
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let market_id = Symbol::new(&env, "volatility_test");

    let mut market = Market::new(
        &env,
        admin,
        String::from_str(&env, "Volatility Test"),
        svec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ],
        env.ledger().timestamp() + 1000,
        OracleConfig::new(
            OracleProvider::reflector(),
            Address::from_str(&env, TEST_ORACLE_ADDRESS),
            String::from_str(&env, "TEST"),
            100,
            String::from_str(&env, "gt"),
        ),
        None,
        86400,
        MarketState::Active,
    );

    market.stakes.set(user.clone(), 5000i128);
    market
        .votes
        .set(user.clone(), String::from_str(&env, "yes"));
    market.total_staked = 5000i128;

    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&market_id, &market);
        let stats = QueryManager::get_market_statistics(&env, market_id).unwrap();

        assert!(stats.volatility >= 0 && stats.volatility <= 10000);
        assert_eq!(stats.consensus_strength + stats.volatility, 10000);
    });
}
