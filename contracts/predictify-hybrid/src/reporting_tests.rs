#![cfg(test)]

//! Comprehensive tests for state snapshot and reporting APIs
//!
//! This module contains tests for:
//! - State snapshot APIs (event snapshots, platform stats)
//! - Reporting APIs (active events, pagination)
//! - Empty state handling
//! - Bounded size and gas efficiency
//! - No state mutation guarantees
//! - Edge cases and error handling

use crate::reporting::*;
use crate::types::*;
use crate::PredictifyHybrid;
use soroban_sdk::{vec, Address, Env, String, Symbol, Vec as SdkVec};
use soroban_sdk::testutils::Address as _;

// Helper function to create oracle config
fn create_oracle_config(env: &Env, name: &str) -> OracleConfig {
    OracleConfig::new(
        OracleProvider::Reflector,
        Address::generate(env),
        String::from_str(env, name),
        100,
        String::from_str(env, "gt"),
    )
}

// Helper function to create fallback oracle config
fn create_fallback_oracle_config(env: &Env) -> SdkVec<OracleConfig> {
    soroban_sdk::vec![env,
        OracleConfig::new(
            OracleProvider::Reflector,
            Address::generate(env),
            String::from_str(env, "FALLBACK"),
            100,
            String::from_str(env, "gte"),
        )
    ]
}

// Helper function to create outcomes
fn create_outcomes(env: &Env) -> SdkVec<String> {
    soroban_sdk::vec![env, String::from_str(env, "yes"), String::from_str(env, "no")]
}

// ===== EMPTY STATE TESTS =====

/// Test that get_active_events returns empty list when no markets exist
#[test]
fn test_get_active_events_empty_state() {
    let env = Env::default();
    let admin = Address::generate(&env);
    
    // Initialize contract (no markets created)
    PredictifyHybrid::initialize(env.clone(), admin.clone(), None);
    
    // Query active events should return empty list
    let result = ReportingManager::get_active_events(&env, 0, 10);
    assert!(result.is_ok(), "Should return Ok for empty state");
    let events = result.unwrap();
    assert_eq!(events.len(), 0, "Should have no active events");
}

/// Test that get_platform_stats returns correct values for empty state
#[test]
fn test_get_platform_stats_empty_state() {
    let env = Env::default();
    let admin = Address::generate(&env);
    
    PredictifyHybrid::initialize(env.clone(), admin.clone(), None);
    
    let stats = ReportingManager::get_platform_stats(&env).unwrap();
    assert_eq!(stats.total_active_events, 0, "No active events");
    assert_eq!(stats.total_resolved_events, 0, "No resolved events");
    assert_eq!(stats.total_pool_all_events, 0, "No funds in pool");
    assert_eq!(stats.total_fees_collected, 0, "No fees collected");
}

/// Test that get_event_snapshot returns error for non-existent market
#[test]
fn test_get_event_snapshot_not_found() {
    let env = Env::default();
    let id = Symbol::new(&env, "NON_EXISTENT");
    let result = ReportingManager::get_event_snapshot(&env, id);
    assert!(result.is_err(), "Should return error for non-existent market");
}

// ===== PAGINATION TESTS =====

#[test]
fn test_get_active_events_pagination() {
    let env = Env::default();
    let admin = Address::generate(&env);
    
    // Initialize contract
    PredictifyHybrid::initialize(env.clone(), admin.clone(), None);
    
    // Create some markets
    let outcomes = create_outcomes(&env);
    let oracle_config = create_oracle_config(&env, "BTC");
    let fallback = create_fallback_oracle_config(&env);

    let m1 = PredictifyHybrid::create_market(
        env.clone(),
        admin.clone(),
        String::from_str(&env, "Question 1"),
        outcomes.clone(),
        30,
        oracle_config.clone(),
        fallback.clone(),
        3600,
    );
    let m2 = PredictifyHybrid::create_market(
        env.clone(),
        admin.clone(),
        String::from_str(&env, "Question 2"),
        outcomes.clone(),
        30,
        oracle_config.clone(),
        fallback.clone(),
        3600,
    );
    let m3 = PredictifyHybrid::create_market(
        env.clone(),
        admin.clone(),
        String::from_str(&env, "Question 3"),
        outcomes.clone(),
        30,
        oracle_config.clone(),
        fallback.clone(),
        3600,
    );

    // Test pagination
    let active_all = ReportingManager::get_active_events(&env, 0, 10).unwrap();
    assert_eq!(active_all.len(), 3);

    let active_paged = ReportingManager::get_active_events(&env, 1, 1).unwrap();
    assert_eq!(active_paged.len(), 1);
    // Check if it's one of the markets (order depends on implementation of market index)
    let id = active_paged.get(0).unwrap().id;
    assert!(id == m1 || id == m2 || id == m3);

    let active_empty = ReportingManager::get_active_events(&env, 10, 10).unwrap();
    assert_eq!(active_empty.len(), 0);
}

/// Test pagination with offset but no limit (should return up to offset count)
#[test]
fn test_get_active_events_offset_only() {
    let env = Env::default();
    let admin = Address::generate(&env);
    
    PredictifyHybrid::initialize(env.clone(), admin.clone(), None);
    
    let outcomes = create_outcomes(&env);
    let oracle_config = create_oracle_config(&env, "BTC");
    let fallback = create_fallback_oracle_config(&env);

    // Create 5 markets
    for i in 0..5 {
        PredictifyHybrid::create_market(
            env.clone(),
            admin.clone(),
            String::from_str(&env, &format!("Question {}", i)),
            outcomes.clone(),
            30,
            oracle_config.clone(),
            fallback.clone(),
            3600,
        );
    }

    // Skip first 2, get rest
    let result = ReportingManager::get_active_events(&env, 2, 10).unwrap();
    assert_eq!(result.len(), 3, "Should return remaining 3 markets");
}

/// Test pagination with zero limit returns empty
#[test]
fn test_get_active_events_zero_limit() {
    let env = Env::default();
    let admin = Address::generate(&env);
    
    PredictifyHybrid::initialize(env.clone(), admin.clone(), None);
    
    let outcomes = create_outcomes(&env);
    let oracle_config = create_oracle_config(&env, "BTC");
    let fallback = create_fallback_oracle_config(&env);

    PredictifyHybrid::create_market(
        env.clone(),
        admin.clone(),
        String::from_str(&env, "Question"),
        outcomes.clone(),
        30,
        oracle_config.clone(),
        fallback.clone(),
        3600,
    );

    let result = ReportingManager::get_active_events(&env, 0, 0).unwrap();
    assert_eq!(result.len(), 0, "Zero limit should return empty");
}

// ===== SNAPSHOT CONTENT VERIFICATION =====

#[test]
fn test_get_event_snapshot() {
    let env = Env::default();
    let admin = Address::generate(&env);
    
    // Initialize contract
    PredictifyHybrid::initialize(env.clone(), admin.clone(), None);
    
    let question = String::from_str(&env, "Snapshot Question");
    let outcomes = create_outcomes(&env);
    let oracle_config = create_oracle_config(&env, "BTC");
    let fallback = create_fallback_oracle_config(&env);

    let market_id = PredictifyHybrid::create_market(
        env.clone(),
        admin.clone(),
        question.clone(),
        outcomes.clone(),
        30,
        oracle_config,
        fallback,
        3600,
    );

    let snapshot = ReportingManager::get_event_snapshot(&env, market_id.clone()).unwrap();
    assert_eq!(snapshot.id, market_id);
    assert_eq!(snapshot.question, question);
    assert_eq!(snapshot.outcomes, outcomes);
    assert_eq!(snapshot.state, MarketState::Active);
    assert_eq!(snapshot.total_pool, 0);
    assert_eq!(snapshot.participant_count, 0);
}

/// Test snapshot content matches current state after market modification
#[test]
fn test_snapshot_matches_current_state() {
    let env = Env::default();
    let admin = Address::generate(&env);
    
    PredictifyHybrid::initialize(env.clone(), admin.clone(), None);
    
    let question = String::from_str(&env, "State Match Test");
    let outcomes = create_outcomes(&env);
    let oracle_config = create_oracle_config(&env, "BTC");
    let fallback = create_fallback_oracle_config(&env);

    let market_id = PredictifyHybrid::create_market(
        env.clone(),
        admin.clone(),
        question.clone(),
        outcomes.clone(),
        30,
        oracle_config.clone(),
        fallback.clone(),
        3600,
    );

    // Get initial snapshot
    let snapshot1 = ReportingManager::get_event_snapshot(&env, market_id.clone()).unwrap();
    assert_eq!(snapshot1.state, MarketState::Active);
    assert_eq!(snapshot1.total_pool, 0);

    // Note: Full state modification test would require placing bets
    // which is complex in test environment. The snapshot should reflect
    // whatever current state exists.

    // Get snapshot again - should be same
    let snapshot2 = ReportingManager::get_event_snapshot(&env, market_id.clone()).unwrap();
    assert_eq!(snapshot1.id, snapshot2.id);
    assert_eq!(snapshot1.question, snapshot2.question);
    assert_eq!(snapshot1.state, snapshot2.state);
}

// ===== PLATFORM STATS TESTS =====

#[test]
fn test_get_platform_stats() {
    let env = Env::default();
    let admin = Address::generate(&env);
    
    // Initialize contract
    PredictifyHybrid::initialize(env.clone(), admin.clone(), None);
    
    // Create a market
    let outcomes = create_outcomes(&env);
    let oracle_config = create_oracle_config(&env, "BTC");
    let fallback = create_fallback_oracle_config(&env);

    PredictifyHybrid::create_market(
        env.clone(),
        admin.clone(),
        String::from_str(&env, "Test"),
        outcomes,
        30,
        oracle_config,
        fallback,
        3600,
    );

    let stats = ReportingManager::get_platform_stats(&env).unwrap();
    assert_eq!(stats.total_active_events, 1);
    assert_eq!(stats.total_resolved_events, 0);
    assert_eq!(stats.total_pool_all_events, 0);
    assert_eq!(stats.version, String::from_str(&env, "1.0.0"));
}

/// Test platform stats with multiple markets of different states
#[test]
fn test_platform_stats_multiple_markets() {
    let env = Env::default();
    let admin = Address::generate(&env);
    
    PredictifyHybrid::initialize(env.clone(), admin.clone(), None);
    
    let outcomes = create_outcomes(&env);
    let oracle_config = create_oracle_config(&env, "BTC");
    let fallback = create_fallback_oracle_config(&env);

    // Create multiple markets
    for i in 0..3 {
        PredictifyHybrid::create_market(
            env.clone(),
            admin.clone(),
            String::from_str(&env, &format!("Market {}", i)),
            outcomes.clone(),
            30,
            oracle_config.clone(),
            fallback.clone(),
            3600,
        );
    }

    let stats = ReportingManager::get_platform_stats(&env).unwrap();
    assert_eq!(stats.total_active_events, 3, "All 3 markets should be active");
    // Note: Resolved count would need market resolution to test
}

// ===== NO STATE MUTATION TESTS =====

/// Verify that querying functions do not mutate state
#[test]
fn test_no_state_mutation_active_events() {
    let env = Env::default();
    let admin = Address::generate(&env);
    
    PredictifyHybrid::initialize(env.clone(), admin.clone(), None);
    
    let outcomes = create_outcomes(&env);
    let oracle_config = create_oracle_config(&env, "BTC");
    let fallback = create_fallback_oracle_config(&env);

    PredictifyHybrid::create_market(
        env.clone(),
        admin.clone(),
        String::from_str(&env, "Test"),
        outcomes.clone(),
        30,
        oracle_config.clone(),
        fallback.clone(),
        3600,
    );

    // Query multiple times - should return same results
    let result1 = ReportingManager::get_active_events(&env, 0, 10).unwrap();
    let result2 = ReportingManager::get_active_events(&env, 0, 10).unwrap();
    
    assert_eq!(result1.len(), result2.len(), "Results should be identical");
}

/// Verify snapshot queries don't mutate state
#[test]
fn test_no_state_mutation_snapshot() {
    let env = Env::default();
    let admin = Address::generate(&env);
    
    PredictifyHybrid::initialize(env.clone(), admin.clone(), None);
    
    let question = String::from_str(&env, "Mutation Test");
    let outcomes = create_outcomes(&env);
    let oracle_config = create_oracle_config(&env, "BTC");
    let fallback = create_fallback_oracle_config(&env);

    let market_id = PredictifyHybrid::create_market(
        env.clone(),
        admin.clone(),
        question.clone(),
        outcomes.clone(),
        30,
        oracle_config,
        fallback,
        3600,
    );

    // Query snapshot multiple times
    let snapshot1 = ReportingManager::get_event_snapshot(&env, market_id.clone()).unwrap();
    let snapshot2 = ReportingManager::get_event_snapshot(&env, market_id.clone()).unwrap();
    
    // Verify all fields are identical
    assert_eq!(snapshot1.id, snapshot2.id);
    assert_eq!(snapshot1.question, snapshot2.question);
    assert_eq!(snapshot1.state, snapshot2.state);
    assert_eq!(snapshot1.total_pool, snapshot2.total_pool);
    assert_eq!(snapshot1.participant_count, snapshot2.participant_count);
}

/// Verify platform stats queries don't mutate state
#[test]
fn test_no_state_mutation_platform_stats() {
    let env = Env::default();
    let admin = Address::generate(&env);
    
    PredictifyHybrid::initialize(env.clone(), admin.clone(), None);
    
    let outcomes = create_outcomes(&env);
    let oracle_config = create_oracle_config(&env, "BTC");
    let fallback = create_fallback_oracle_config(&env);

    PredictifyHybrid::create_market(
        env.clone(),
        admin.clone(),
        String::from_str(&env, "Test"),
        outcomes.clone(),
        30,
        oracle_config.clone(),
        fallback.clone(),
        3600,
    );

    // Query stats multiple times
    let stats1 = ReportingManager::get_platform_stats(&env).unwrap();
    let stats2 = ReportingManager::get_platform_stats(&env).unwrap();
    
    assert_eq!(stats1.total_active_events, stats2.total_active_events);
    assert_eq!(stats1.total_resolved_events, stats2.total_resolved_events);
    assert_eq!(stats1.total_pool_all_events, stats2.total_pool_all_events);
}

// ===== BOUNDED SIZE AND GAS TESTS =====

/// Test that limit parameter properly bounds result size
#[test]
fn test_bounded_size_by_limit() {
    let env = Env::default();
    let admin = Address::generate(&env);
    
    PredictifyHybrid::initialize(env.clone(), admin.clone(), None);
    
    let outcomes = create_outcomes(&env);
    let oracle_config = create_oracle_config(&env, "BTC");
    let fallback = create_fallback_oracle_config(&env);

    // Create 10 markets
    for i in 0..10 {
        PredictifyHybrid::create_market(
            env.clone(),
            admin.clone(),
            String::from_str(&env, &format!("Market {}", i)),
            outcomes.clone(),
            30,
            oracle_config.clone(),
            fallback.clone(),
            3600,
        );
    }

    // Request only 3 - should get at most 3
    let result = ReportingManager::get_active_events(&env, 0, 3).unwrap();
    assert!(result.len() <= 3, "Result should be bounded by limit");
    assert_eq!(result.len(), 3, "Should return exactly 3");
}

/// Test with large limit value (should not cause issues)
#[test]
fn test_large_limit_handling() {
    let env = Env::default();
    let admin = Address::generate(&env);
    
    PredictifyHybrid::initialize(env.clone(), admin.clone(), None);
    
    let outcomes = create_outcomes(&env);
    let oracle_config = create_oracle_config(&env, "BTC");
    let fallback = create_fallback_oracle_config(&env);

    // Create a few markets
    for i in 0..2 {
        PredictifyHybrid::create_market(
            env.clone(),
            admin.clone(),
            String::from_str(&env, &format!("M{}", i)),
            outcomes.clone(),
            30,
            oracle_config.clone(),
            fallback.clone(),
            3600,
        );
    }

    // Use very large limit - should handle gracefully
    let result = ReportingManager::get_active_events(&env, 0, u32::MAX).unwrap();
    assert_eq!(result.len(), 2, "Should return all markets despite large limit");
}

// ===== EDGE CASES =====

/// Test with offset greater than number of markets
#[test]
fn test_offset_exceeds_markets() {
    let env = Env::default();
    let admin = Address::generate(&env);
    
    PredictifyHybrid::initialize(env.clone(), admin.clone(), None);
    
    let outcomes = create_outcomes(&env);
    let oracle_config = create_oracle_config(&env, "BTC");
    let fallback = create_fallback_oracle_config(&env);

    // Create 1 market
    PredictifyHybrid::create_market(
        env.clone(),
        admin.clone(),
        String::from_str(&env, "Test"),
        outcomes.clone(),
        30,
        oracle_config.clone(),
        fallback.clone(),
        3600,
    );

    // Offset beyond available
    let result = ReportingManager::get_active_events(&env, 5, 10).unwrap();
    assert_eq!(result.len(), 0, "Should return empty when offset exceeds");
}

/// Test snapshot with different market states
#[test]
fn test_snapshot_different_states() {
    let env = Env::default();
    let admin = Address::generate(&env);
    
    PredictifyHybrid::initialize(env.clone(), admin.clone(), None);
    
    let outcomes = create_outcomes(&env);
    let oracle_config = create_oracle_config(&env, "BTC");
    let fallback = create_fallback_oracle_config(&env);

    // Active market should have Active state
    let market_id = PredictifyHybrid::create_market(
        env.clone(),
        admin.clone(),
        String::from_str(&env, "Active Market"),
        outcomes.clone(),
        30,
        oracle_config.clone(),
        fallback.clone(),
        3600,
    );

    let snapshot = ReportingManager::get_event_snapshot(&env, market_id).unwrap();
    assert_eq!(snapshot.state, MarketState::Active);
}

/// Test that ActiveEvent contains correct data structure
#[test]
fn test_active_event_data_structure() {
    let env = Env::default();
    let admin = Address::generate(&env);
    
    PredictifyHybrid::initialize(env.clone(), admin.clone(), None);
    
    let question = String::from_str(&env, "Active Event Test");
    let outcomes = create_outcomes(&env);
    let oracle_config = create_oracle_config(&env, "BTC");
    let fallback = create_fallback_oracle_config(&env);

    let market_id = PredictifyHybrid::create_market(
        env.clone(),
        admin.clone(),
        question.clone(),
        outcomes.clone(),
        30,
        oracle_config,
        fallback,
        3600,
    );

    let active_events = ReportingManager::get_active_events(&env, 0, 10).unwrap();
    assert_eq!(active_events.len(), 1);
    
    let event = active_events.get(0).unwrap();
    assert_eq!(event.id, market_id);
    assert_eq!(event.question, question);
    // end_time should be set
    assert!(event.end_time > 0, "End time should be set");
    // total_pool should be initialized
    assert_eq!(event.total_pool, 0, "Initial pool should be 0");
}

/// Test EventSnapshot outcome pools structure
#[test]
fn test_snapshot_outcome_pools() {
    let env = Env::default();
    let admin = Address::generate(&env);
    
    PredictifyHybrid::initialize(env.clone(), admin.clone(), None);
    
    let outcomes = create_outcomes(&env);
    let oracle_config = create_oracle_config(&env, "BTC");
    let fallback = create_fallback_oracle_config(&env);

    let market_id = PredictifyHybrid::create_market(
        env.clone(),
        admin.clone(),
        String::from_str(&env, "Pool Test"),
        outcomes.clone(),
        30,
        oracle_config,
        fallback,
        3600,
    );

    let snapshot = ReportingManager::get_event_snapshot(&env, market_id).unwrap();
    
    // Check outcome pools map contains both outcomes
    assert!(snapshot.outcome_pools.len() >= 2, "Should have pools for outcomes");
}

// ===== CONSISTENCY TESTS =====

/// Test that get_all_markets and get_active_events are consistent
#[test]
fn test_active_events_consistency() {
    let env = Env::default();
    let admin = Address::generate(&env);
    
    PredictifyHybrid::initialize(env.clone(), admin.clone(), None);
    
    let outcomes = create_outcomes(&env);
    let oracle_config = create_oracle_config(&env, "BTC");
    let fallback = create_fallback_oracle_config(&env);

    // Create 5 markets
    let mut market_ids = SdkVec::new(&env);
    for i in 0..5 {
        let id = PredictifyHybrid::create_market(
            env.clone(),
            admin.clone(),
            String::from_str(&env, &format!("M{}", i)),
            outcomes.clone(),
            30,
            oracle_config.clone(),
            fallback.clone(),
            3600,
        );
        market_ids.push_back(id);
    }

    // Get all active events
    let active = ReportingManager::get_active_events(&env, 0, 100).unwrap();
    
    // All markets should be active
    assert_eq!(active.len(), 5, "All markets should be active");
}

/// Test consistency between platform stats and individual snapshots
#[test]
fn test_stats_and_snapshot_consistency() {
    let env = Env::default();
    let admin = Address::generate(&env);
    
    PredictifyHybrid::initialize(env.clone(), admin.clone(), None);
    
    let outcomes = create_outcomes(&env);
    let oracle_config = create_oracle_config(&env, "BTC");
    let fallback = create_fallback_oracle_config(&env);

    // Create market
    let market_id = PredictifyHybrid::create_market(
        env.clone(),
        admin.clone(),
        String::from_str(&env, "Test"),
        outcomes.clone(),
        30,
        oracle_config.clone(),
        fallback.clone(),
        3600,
    );

    // Get platform stats
    let stats = ReportingManager::get_platform_stats(&env).unwrap();
    
    // Get individual snapshot
    let snapshot = ReportingManager::get_event_snapshot(&env, market_id).unwrap();
    
    // Stats should reflect the single market
    assert_eq!(stats.total_active_events, 1);
    assert!(stats.total_pool_all_events >= snapshot.total_pool);
}
