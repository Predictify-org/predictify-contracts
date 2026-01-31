//! Resolution Delay and Dispute Window Tests
//!
//! Comprehensive test coverage for the resolution delay (dispute window) feature.
//! These tests ensure the dispute window mechanism works correctly across all scenarios.
//!
//! # Test Categories
//!
//! 1. **Window Configuration Tests** - Setting global and per-market windows
//! 2. **Resolution Workflow Tests** - Proposal and window opening
//! 3. **Dispute During Window Tests** - Filing disputes
//! 4. **Finalization Tests** - Closing window and enabling payouts
//! 5. **Edge Case Tests** - Boundary conditions and error handling
//! 6. **Integration Tests** - Full lifecycle tests

#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    Address, Env, Map, String, Symbol, Vec,
};

use crate::errors::Error;
use crate::markets::MarketStateManager;
use crate::resolution_delay::{
    ResolutionDelayManager, DEFAULT_DISPUTE_WINDOW_HOURS, MAX_DISPUTE_WINDOW_HOURS,
    MIN_DISPUTE_WINDOW_HOURS,
};
use crate::types::{Market, MarketState, OracleConfig, ResolutionDelayConfig};

// ===== TEST HELPERS =====

/// Creates a test environment with admin set up
fn setup_test_env() -> (Env, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);

    // Set up admin in storage with contract context
    env.storage()
        .persistent()
        .set(&Symbol::new(&env, "Admin"), &admin);

    (env, admin)
}

/// Creates a test market that has ended
fn create_ended_market(env: &Env, market_id: &Symbol, admin: &Address) -> Market {
    let outcomes = Vec::from_array(
        env,
        [
            String::from_str(env, "yes"),
            String::from_str(env, "no"),
        ],
    );

    let oracle_config = OracleConfig {
        provider: crate::types::OracleProvider::Pyth,
        feed_id: String::from_str(env, "TEST/USD"),
        threshold: 100,
        comparison: String::from_str(env, "gt"),
    };

    // Set market end time in the past
    let current_time = env.ledger().timestamp();
    let end_time = current_time.saturating_sub(3600); // 1 hour ago

    let mut market = Market::new(
        env,
        admin.clone(),
        String::from_str(env, "Test market?"),
        outcomes,
        end_time,
        oracle_config,
        MarketState::Ended,
    );

    // Set oracle result so resolution can be proposed
    market.oracle_result = Some(String::from_str(env, "yes"));

    // Store market
    env.storage().persistent().set(market_id, &market);

    market
}

/// Advances the ledger timestamp by the given number of seconds
fn advance_time(env: &Env, seconds: u64) {
    let current_time = env.ledger().timestamp();
    env.ledger().set(LedgerInfo {
        timestamp: current_time + seconds,
        protocol_version: env.ledger().protocol_version(),
        sequence_number: env.ledger().sequence(),
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 1,
        min_persistent_entry_ttl: 1,
        max_entry_ttl: 10000000,
    });
}

// ===== WINDOW CONFIGURATION TESTS =====

mod window_configuration_tests {
    use super::*;

    #[test]
    fn test_set_global_dispute_window() {
        let (env, admin) = setup_test_env();

        // Set global window to 72 hours
        let result = ResolutionDelayManager::set_global_dispute_window(&env, &admin, 72);
        assert!(result.is_ok());

        // Verify config
        let config = ResolutionDelayManager::get_global_config(&env);
        assert_eq!(config.dispute_window_hours, 72);
        assert_eq!(config.min_dispute_stake, 10_000_000);
        assert!(config.auto_finalize_enabled);
    }

    #[test]
    fn test_set_global_window_minimum() {
        let (env, admin) = setup_test_env();

        // Set to minimum (1 hour)
        let result =
            ResolutionDelayManager::set_global_dispute_window(&env, &admin, MIN_DISPUTE_WINDOW_HOURS);
        assert!(result.is_ok());

        let config = ResolutionDelayManager::get_global_config(&env);
        assert_eq!(config.dispute_window_hours, MIN_DISPUTE_WINDOW_HOURS);
    }

    #[test]
    fn test_set_global_window_maximum() {
        let (env, admin) = setup_test_env();

        // Set to maximum (168 hours = 1 week)
        let result =
            ResolutionDelayManager::set_global_dispute_window(&env, &admin, MAX_DISPUTE_WINDOW_HOURS);
        assert!(result.is_ok());

        let config = ResolutionDelayManager::get_global_config(&env);
        assert_eq!(config.dispute_window_hours, MAX_DISPUTE_WINDOW_HOURS);
    }

    #[test]
    fn test_invalid_window_zero_hours() {
        let (env, admin) = setup_test_env();

        // 0 hours should fail
        let result = ResolutionDelayManager::set_global_dispute_window(&env, &admin, 0);
        assert_eq!(result, Err(Error::InvalidTimeoutHours));
    }

    #[test]
    fn test_invalid_window_exceeds_maximum() {
        let (env, admin) = setup_test_env();

        // Exceeds maximum (200 > 168)
        let result = ResolutionDelayManager::set_global_dispute_window(&env, &admin, 200);
        assert_eq!(result, Err(Error::InvalidTimeoutHours));
    }

    #[test]
    fn test_unauthorized_set_global_window() {
        let (env, admin) = setup_test_env();
        let non_admin = Address::generate(&env);

        // Non-admin should fail
        let result = ResolutionDelayManager::set_global_dispute_window(&env, &non_admin, 48);
        assert_eq!(result, Err(Error::Unauthorized));

        // Admin should succeed
        let result = ResolutionDelayManager::set_global_dispute_window(&env, &admin, 48);
        assert!(result.is_ok());
    }

    #[test]
    fn test_default_config() {
        let env = Env::default();
        let config = ResolutionDelayManager::get_global_config(&env);

        assert_eq!(config.dispute_window_hours, DEFAULT_DISPUTE_WINDOW_HOURS);
        assert_eq!(config.min_dispute_stake, 10_000_000);
        assert!(config.auto_finalize_enabled);
    }

    #[test]
    fn test_set_market_dispute_window() {
        let (env, admin) = setup_test_env();
        let market_id = Symbol::new(&env, "test_market");

        // Create a market first
        create_ended_market(&env, &market_id, &admin);

        // Set per-market window to 24 hours
        let result =
            ResolutionDelayManager::set_market_dispute_window(&env, &admin, &market_id, 24);
        assert!(result.is_ok());

        // Get config for this market - should use market-specific setting
        let config = ResolutionDelayManager::get_dispute_window_config(&env, &market_id);
        assert_eq!(config.dispute_window_hours, 24);
    }

    #[test]
    fn test_market_window_zero_uses_global() {
        let (env, admin) = setup_test_env();
        let market_id = Symbol::new(&env, "test_market");

        // Set global to 72 hours
        ResolutionDelayManager::set_global_dispute_window(&env, &admin, 72).unwrap();

        // Create market
        create_ended_market(&env, &market_id, &admin);

        // Set market to 0 (use global)
        ResolutionDelayManager::set_market_dispute_window(&env, &admin, &market_id, 0).unwrap();

        // Should use global config
        let config = ResolutionDelayManager::get_dispute_window_config(&env, &market_id);
        assert_eq!(config.dispute_window_hours, 72);
    }
}

// ===== RESOLUTION WORKFLOW TESTS =====

mod resolution_workflow_tests {
    use super::*;

    #[test]
    fn test_propose_resolution_opens_window() {
        let (env, admin) = setup_test_env();
        let market_id = Symbol::new(&env, "test_market");

        // Create ended market
        create_ended_market(&env, &market_id, &admin);

        // Propose resolution
        let outcome = String::from_str(&env, "yes");
        let source = String::from_str(&env, "Oracle");
        let result = ResolutionDelayManager::propose_resolution(&env, &market_id, outcome, source);
        assert!(result.is_ok());

        // Window should now be open
        assert!(ResolutionDelayManager::is_dispute_window_open(&env, &market_id));
    }

    #[test]
    fn test_window_timing() {
        let (env, admin) = setup_test_env();
        let market_id = Symbol::new(&env, "test_market");

        // Set 24 hour window
        ResolutionDelayManager::set_global_dispute_window(&env, &admin, 24).unwrap();

        // Create and propose
        create_ended_market(&env, &market_id, &admin);
        ResolutionDelayManager::propose_resolution(
            &env,
            &market_id,
            String::from_str(&env, "yes"),
            String::from_str(&env, "Oracle"),
        )
        .unwrap();

        // Window open initially
        assert!(ResolutionDelayManager::is_dispute_window_open(&env, &market_id));

        // Remaining time should be close to 24 hours
        let remaining = ResolutionDelayManager::get_window_remaining_time(&env, &market_id);
        assert!(remaining > 23 * 60 * 60);
        assert!(remaining <= 24 * 60 * 60);

        // Advance 12 hours
        advance_time(&env, 12 * 60 * 60);
        assert!(ResolutionDelayManager::is_dispute_window_open(&env, &market_id));

        let remaining = ResolutionDelayManager::get_window_remaining_time(&env, &market_id);
        assert!(remaining > 11 * 60 * 60);
        assert!(remaining <= 12 * 60 * 60);

        // Advance to exactly 24 hours - window should be closed
        advance_time(&env, 12 * 60 * 60);
        assert!(!ResolutionDelayManager::is_dispute_window_open(&env, &market_id));
        assert_eq!(
            ResolutionDelayManager::get_window_remaining_time(&env, &market_id),
            0
        );
    }

    #[test]
    fn test_propose_resolution_for_active_market_fails() {
        let (env, admin) = setup_test_env();
        let market_id = Symbol::new(&env, "test_market");

        // Create a market that hasn't ended
        let outcomes = Vec::from_array(
            &env,
            [
                String::from_str(&env, "yes"),
                String::from_str(&env, "no"),
            ],
        );

        let oracle_config = OracleConfig {
            provider: crate::types::OracleProvider::Pyth,
            feed_id: String::from_str(&env, "TEST/USD"),
            threshold: 100,
            comparison: String::from_str(&env, "gt"),
        };

        let current_time = env.ledger().timestamp();
        let end_time = current_time + 3600; // 1 hour in the future

        let market = Market::new(
            &env,
            admin.clone(),
            String::from_str(&env, "Test market?"),
            outcomes,
            end_time,
            oracle_config,
            MarketState::Active,
        );

        env.storage().persistent().set(&market_id, &market);

        // Propose should fail - market hasn't ended
        let result = ResolutionDelayManager::propose_resolution(
            &env,
            &market_id,
            String::from_str(&env, "yes"),
            String::from_str(&env, "Oracle"),
        );
        assert_eq!(result, Err(Error::MarketClosed));
    }

    #[test]
    fn test_get_dispute_window_status() {
        let (env, admin) = setup_test_env();
        let market_id = Symbol::new(&env, "test_market");

        // Before proposal - no window
        let (is_open, remaining, disputes) =
            ResolutionDelayManager::get_dispute_window_status(&env, &market_id);
        assert!(!is_open);
        assert_eq!(remaining, 0);
        assert_eq!(disputes, 0);

        // Create and propose
        create_ended_market(&env, &market_id, &admin);
        ResolutionDelayManager::propose_resolution(
            &env,
            &market_id,
            String::from_str(&env, "yes"),
            String::from_str(&env, "Oracle"),
        )
        .unwrap();

        // After proposal
        let (is_open, remaining, disputes) =
            ResolutionDelayManager::get_dispute_window_status(&env, &market_id);
        assert!(is_open);
        assert!(remaining > 0);
        assert_eq!(disputes, 0);
    }
}

// ===== DISPUTE DURING WINDOW TESTS =====

mod dispute_during_window_tests {
    use super::*;

    #[test]
    fn test_validate_dispute_allowed_during_window() {
        let (env, admin) = setup_test_env();
        let market_id = Symbol::new(&env, "test_market");

        create_ended_market(&env, &market_id, &admin);
        ResolutionDelayManager::propose_resolution(
            &env,
            &market_id,
            String::from_str(&env, "yes"),
            String::from_str(&env, "Oracle"),
        )
        .unwrap();

        // Dispute should be allowed during window
        let result = ResolutionDelayManager::validate_dispute_allowed(&env, &market_id);
        assert!(result.is_ok());
    }

    #[test]
    fn test_dispute_not_allowed_before_proposal() {
        let (env, admin) = setup_test_env();
        let market_id = Symbol::new(&env, "test_market");

        create_ended_market(&env, &market_id, &admin);

        // No proposal yet
        let result = ResolutionDelayManager::validate_dispute_allowed(&env, &market_id);
        assert_eq!(result, Err(Error::MarketNotResolved));
    }

    #[test]
    fn test_dispute_not_allowed_after_window_closes() {
        let (env, admin) = setup_test_env();
        let market_id = Symbol::new(&env, "test_market");

        // Set short window
        ResolutionDelayManager::set_global_dispute_window(&env, &admin, 1).unwrap();

        create_ended_market(&env, &market_id, &admin);
        ResolutionDelayManager::propose_resolution(
            &env,
            &market_id,
            String::from_str(&env, "yes"),
            String::from_str(&env, "Oracle"),
        )
        .unwrap();

        // Advance past window
        advance_time(&env, 2 * 60 * 60); // 2 hours

        // Dispute should not be allowed
        let result = ResolutionDelayManager::validate_dispute_allowed(&env, &market_id);
        assert_eq!(result, Err(Error::DisputeVotingNotAllowed));
    }

    #[test]
    fn test_record_dispute() {
        let (env, admin) = setup_test_env();
        let market_id = Symbol::new(&env, "test_market");

        create_ended_market(&env, &market_id, &admin);
        ResolutionDelayManager::propose_resolution(
            &env,
            &market_id,
            String::from_str(&env, "yes"),
            String::from_str(&env, "Oracle"),
        )
        .unwrap();

        // Record a dispute
        let result = ResolutionDelayManager::record_dispute(&env, &market_id);
        assert!(result.is_ok());

        // Dispute count should be 1
        let (_, _, disputes) = ResolutionDelayManager::get_dispute_window_status(&env, &market_id);
        assert_eq!(disputes, 1);

        // Record another
        ResolutionDelayManager::record_dispute(&env, &market_id).unwrap();

        let (_, _, disputes) = ResolutionDelayManager::get_dispute_window_status(&env, &market_id);
        assert_eq!(disputes, 2);
    }

    #[test]
    fn test_record_dispute_outside_window_fails() {
        let (env, admin) = setup_test_env();
        let market_id = Symbol::new(&env, "test_market");

        // Set 1 hour window
        ResolutionDelayManager::set_global_dispute_window(&env, &admin, 1).unwrap();

        create_ended_market(&env, &market_id, &admin);
        ResolutionDelayManager::propose_resolution(
            &env,
            &market_id,
            String::from_str(&env, "yes"),
            String::from_str(&env, "Oracle"),
        )
        .unwrap();

        // Advance past window
        advance_time(&env, 2 * 60 * 60);

        // Recording should fail
        let result = ResolutionDelayManager::record_dispute(&env, &market_id);
        assert_eq!(result, Err(Error::DisputeVotingNotAllowed));
    }
}

// ===== FINALIZATION TESTS =====

mod finalization_tests {
    use super::*;

    #[test]
    fn test_finalize_after_window_closes() {
        let (env, admin) = setup_test_env();
        let market_id = Symbol::new(&env, "test_market");

        // Set 1 hour window
        ResolutionDelayManager::set_global_dispute_window(&env, &admin, 1).unwrap();

        create_ended_market(&env, &market_id, &admin);
        ResolutionDelayManager::propose_resolution(
            &env,
            &market_id,
            String::from_str(&env, "yes"),
            String::from_str(&env, "Oracle"),
        )
        .unwrap();

        // Advance past window
        advance_time(&env, 2 * 60 * 60);

        // Finalize should succeed
        let result = ResolutionDelayManager::finalize_resolution(&env, &market_id);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), String::from_str(&env, "yes"));
    }

    #[test]
    fn test_finalize_during_window_fails() {
        let (env, admin) = setup_test_env();
        let market_id = Symbol::new(&env, "test_market");

        create_ended_market(&env, &market_id, &admin);
        ResolutionDelayManager::propose_resolution(
            &env,
            &market_id,
            String::from_str(&env, "yes"),
            String::from_str(&env, "Oracle"),
        )
        .unwrap();

        // Don't advance time - window still open

        // Finalize should fail
        let result = ResolutionDelayManager::finalize_resolution(&env, &market_id);
        assert_eq!(result, Err(Error::DisputeTimeoutNotExpired));
    }

    #[test]
    fn test_finalize_without_proposal_fails() {
        let (env, admin) = setup_test_env();
        let market_id = Symbol::new(&env, "test_market");

        create_ended_market(&env, &market_id, &admin);

        // No proposal yet
        let result = ResolutionDelayManager::finalize_resolution(&env, &market_id);
        assert_eq!(result, Err(Error::MarketNotResolved));
    }

    #[test]
    fn test_finalize_already_finalized_fails() {
        let (env, admin) = setup_test_env();
        let market_id = Symbol::new(&env, "test_market");

        // Set 1 hour window
        ResolutionDelayManager::set_global_dispute_window(&env, &admin, 1).unwrap();

        create_ended_market(&env, &market_id, &admin);
        ResolutionDelayManager::propose_resolution(
            &env,
            &market_id,
            String::from_str(&env, "yes"),
            String::from_str(&env, "Oracle"),
        )
        .unwrap();

        // Advance and finalize
        advance_time(&env, 2 * 60 * 60);
        ResolutionDelayManager::finalize_resolution(&env, &market_id).unwrap();

        // Second finalize should fail
        let result = ResolutionDelayManager::finalize_resolution(&env, &market_id);
        assert_eq!(result, Err(Error::MarketAlreadyResolved));
    }

    #[test]
    fn test_is_resolution_finalized() {
        let (env, admin) = setup_test_env();
        let market_id = Symbol::new(&env, "test_market");

        // Set 1 hour window
        ResolutionDelayManager::set_global_dispute_window(&env, &admin, 1).unwrap();

        create_ended_market(&env, &market_id, &admin);

        // Not finalized before proposal
        assert!(!ResolutionDelayManager::is_resolution_finalized(&env, &market_id));

        ResolutionDelayManager::propose_resolution(
            &env,
            &market_id,
            String::from_str(&env, "yes"),
            String::from_str(&env, "Oracle"),
        )
        .unwrap();

        // Not finalized after proposal
        assert!(!ResolutionDelayManager::is_resolution_finalized(&env, &market_id));

        // Advance and finalize
        advance_time(&env, 2 * 60 * 60);
        ResolutionDelayManager::finalize_resolution(&env, &market_id).unwrap();

        // Now finalized
        assert!(ResolutionDelayManager::is_resolution_finalized(&env, &market_id));
    }

    #[test]
    fn test_can_finalize() {
        let (env, admin) = setup_test_env();
        let market_id = Symbol::new(&env, "test_market");

        // Set 1 hour window
        ResolutionDelayManager::set_global_dispute_window(&env, &admin, 1).unwrap();

        create_ended_market(&env, &market_id, &admin);

        // Before proposal
        let (can, reason) = ResolutionDelayManager::can_finalize(&env, &market_id);
        assert!(!can);
        assert_eq!(reason, String::from_str(&env, "Resolution not proposed"));

        ResolutionDelayManager::propose_resolution(
            &env,
            &market_id,
            String::from_str(&env, "yes"),
            String::from_str(&env, "Oracle"),
        )
        .unwrap();

        // During window
        let (can, reason) = ResolutionDelayManager::can_finalize(&env, &market_id);
        assert!(!can);
        assert_eq!(reason, String::from_str(&env, "Window still open"));

        // After window
        advance_time(&env, 2 * 60 * 60);
        let (can, reason) = ResolutionDelayManager::can_finalize(&env, &market_id);
        assert!(can);
        assert_eq!(reason, String::from_str(&env, "Ready to finalize"));
    }

    #[test]
    fn test_force_finalize() {
        let (env, admin) = setup_test_env();
        let market_id = Symbol::new(&env, "test_market");

        create_ended_market(&env, &market_id, &admin);

        // Force finalize without proposal
        let result = ResolutionDelayManager::force_finalize(
            &env,
            &admin,
            &market_id,
            String::from_str(&env, "no"), // Override outcome
        );
        assert!(result.is_ok());

        // Should be finalized
        assert!(ResolutionDelayManager::is_resolution_finalized(&env, &market_id));
    }

    #[test]
    fn test_force_finalize_unauthorized() {
        let (env, admin) = setup_test_env();
        let non_admin = Address::generate(&env);
        let market_id = Symbol::new(&env, "test_market");

        create_ended_market(&env, &market_id, &admin);

        // Non-admin should fail
        let result = ResolutionDelayManager::force_finalize(
            &env,
            &non_admin,
            &market_id,
            String::from_str(&env, "no"),
        );
        assert_eq!(result, Err(Error::Unauthorized));
    }
}

// ===== EDGE CASE TESTS =====

mod edge_case_tests {
    use super::*;

    #[test]
    fn test_market_resolution_fields() {
        let (env, admin) = setup_test_env();
        let market_id = Symbol::new(&env, "test_market");

        // Create ended market
        create_ended_market(&env, &market_id, &admin);
        
        // Propose resolution with 48-hour window
        ResolutionDelayManager::set_global_dispute_window(&env, &admin, 48).unwrap();
        ResolutionDelayManager::propose_resolution(
            &env,
            &market_id,
            String::from_str(&env, "yes"),
            String::from_str(&env, "Oracle"),
        ).unwrap();

        // Check market fields
        let market: Market = env.storage().persistent().get(&market_id).unwrap();
        assert_eq!(market.resolution_proposed_outcome, Some(String::from_str(&env, "yes")));
        assert!(!market.resolution_is_finalized);
        assert_eq!(market.resolution_dispute_count, 0);

        // Window should be open
        let current_time = env.ledger().timestamp();
        assert!(market.is_dispute_window_open(current_time));

        // Window end time should be 48 hours from now
        assert_eq!(market.resolution_window_end_time, current_time + (48 * 60 * 60));
    }

    #[test]
    fn test_market_dispute_window_timing() {
        let (env, admin) = setup_test_env();
        let market_id = Symbol::new(&env, "test_market");

        // Set 24-hour window
        ResolutionDelayManager::set_global_dispute_window(&env, &admin, 24).unwrap();
        create_ended_market(&env, &market_id, &admin);
        ResolutionDelayManager::propose_resolution(
            &env,
            &market_id,
            String::from_str(&env, "yes"),
            String::from_str(&env, "Oracle"),
        ).unwrap();

        let market: Market = env.storage().persistent().get(&market_id).unwrap();
        let start_time = env.ledger().timestamp();

        // Exactly at window end - should be closed
        let time_24h = start_time + (24 * 60 * 60);
        assert!(!market.is_dispute_window_open(time_24h));

        // One second before end - should be open
        let time_just_before = start_time + (24 * 60 * 60) - 1;
        assert!(market.is_dispute_window_open(time_just_before));
    }

    #[test]
    fn test_market_not_found() {
        let (env, _admin) = setup_test_env();
        let market_id = Symbol::new(&env, "nonexistent");

        // All operations should handle missing market gracefully
        assert!(!ResolutionDelayManager::is_dispute_window_open(&env, &market_id));
        assert_eq!(
            ResolutionDelayManager::get_window_remaining_time(&env, &market_id),
            0
        );
        assert!(!ResolutionDelayManager::is_resolution_finalized(&env, &market_id));

        let (is_open, remaining, disputes) =
            ResolutionDelayManager::get_dispute_window_status(&env, &market_id);
        assert!(!is_open);
        assert_eq!(remaining, 0);
        assert_eq!(disputes, 0);
    }

    #[test]
    fn test_very_short_window() {
        let (env, admin) = setup_test_env();
        let market_id = Symbol::new(&env, "test_market");

        // Set minimum window (1 hour)
        ResolutionDelayManager::set_global_dispute_window(&env, &admin, 1).unwrap();

        create_ended_market(&env, &market_id, &admin);
        ResolutionDelayManager::propose_resolution(
            &env,
            &market_id,
            String::from_str(&env, "yes"),
            String::from_str(&env, "Oracle"),
        )
        .unwrap();

        // Window should be open
        assert!(ResolutionDelayManager::is_dispute_window_open(&env, &market_id));

        // Advance 59 minutes - still open
        advance_time(&env, 59 * 60);
        assert!(ResolutionDelayManager::is_dispute_window_open(&env, &market_id));

        // Advance 2 more minutes - now closed
        advance_time(&env, 2 * 60);
        assert!(!ResolutionDelayManager::is_dispute_window_open(&env, &market_id));
    }

    #[test]
    fn test_very_long_window() {
        let (env, admin) = setup_test_env();
        let market_id = Symbol::new(&env, "test_market");

        // Set maximum window (168 hours = 1 week)
        ResolutionDelayManager::set_global_dispute_window(&env, &admin, 168).unwrap();

        create_ended_market(&env, &market_id, &admin);
        ResolutionDelayManager::propose_resolution(
            &env,
            &market_id,
            String::from_str(&env, "yes"),
            String::from_str(&env, "Oracle"),
        )
        .unwrap();

        // Advance 6 days - still open
        advance_time(&env, 6 * 24 * 60 * 60);
        assert!(ResolutionDelayManager::is_dispute_window_open(&env, &market_id));

        // Advance 2 more days - now closed
        advance_time(&env, 2 * 24 * 60 * 60);
        assert!(!ResolutionDelayManager::is_dispute_window_open(&env, &market_id));
    }

    #[test]
    fn test_dispute_not_allowed_after_finalization() {
        let (env, admin) = setup_test_env();
        let market_id = Symbol::new(&env, "test_market");

        // Set 1 hour window
        ResolutionDelayManager::set_global_dispute_window(&env, &admin, 1).unwrap();

        create_ended_market(&env, &market_id, &admin);
        ResolutionDelayManager::propose_resolution(
            &env,
            &market_id,
            String::from_str(&env, "yes"),
            String::from_str(&env, "Oracle"),
        )
        .unwrap();

        // Advance and finalize
        advance_time(&env, 2 * 60 * 60);
        ResolutionDelayManager::finalize_resolution(&env, &market_id).unwrap();

        // Dispute should not be allowed after finalization
        let result = ResolutionDelayManager::validate_dispute_allowed(&env, &market_id);
        assert_eq!(result, Err(Error::MarketAlreadyResolved));
    }

    #[test]
    fn test_config_validation() {
        let valid_config = ResolutionDelayConfig {
            dispute_window_hours: 48,
            min_dispute_stake: 10_000_000,
            auto_finalize_enabled: true,
        };
        assert!(valid_config.validate().is_ok());

        let zero_hours = ResolutionDelayConfig {
            dispute_window_hours: 0,
            min_dispute_stake: 10_000_000,
            auto_finalize_enabled: true,
        };
        assert_eq!(zero_hours.validate(), Err(Error::InvalidTimeoutHours));

        let too_many_hours = ResolutionDelayConfig {
            dispute_window_hours: 200,
            min_dispute_stake: 10_000_000,
            auto_finalize_enabled: true,
        };
        assert_eq!(too_many_hours.validate(), Err(Error::InvalidTimeoutHours));
    }

    #[test]
    fn test_market_finalization() {
        let (env, admin) = setup_test_env();
        let market_id = Symbol::new(&env, "test_market");

        // Set 1-hour window
        ResolutionDelayManager::set_global_dispute_window(&env, &admin, 1).unwrap();
        create_ended_market(&env, &market_id, &admin);
        ResolutionDelayManager::propose_resolution(
            &env,
            &market_id,
            String::from_str(&env, "yes"),
            String::from_str(&env, "Oracle"),
        ).unwrap();

        let market: Market = env.storage().persistent().get(&market_id).unwrap();
        assert!(!market.resolution_is_finalized);

        // Advance past window
        advance_time(&env, 2 * 60 * 60);

        // Finalize
        ResolutionDelayManager::finalize_resolution(&env, &market_id).unwrap();

        let market: Market = env.storage().persistent().get(&market_id).unwrap();
        assert!(market.resolution_is_finalized);

        // Once finalized, window should not be "open"
        let current_time = env.ledger().timestamp();
        assert!(!market.is_dispute_window_open(current_time));
    }
}

// ===== INTEGRATION TESTS =====

mod integration_tests {
    use super::*;

    #[test]
    fn test_full_lifecycle_no_disputes() {
        let (env, admin) = setup_test_env();
        let market_id = Symbol::new(&env, "test_market");

        // 1. Set global dispute window
        ResolutionDelayManager::set_global_dispute_window(&env, &admin, 24).unwrap();

        // 2. Create market that has ended
        create_ended_market(&env, &market_id, &admin);

        // 3. Propose resolution
        let outcome = String::from_str(&env, "yes");
        ResolutionDelayManager::propose_resolution(
            &env,
            &market_id,
            outcome.clone(),
            String::from_str(&env, "Oracle"),
        )
        .unwrap();

        // 4. Verify window is open
        assert!(ResolutionDelayManager::is_dispute_window_open(&env, &market_id));
        assert!(!ResolutionDelayManager::is_resolution_finalized(&env, &market_id));

        // 5. Cannot finalize during window
        assert_eq!(
            ResolutionDelayManager::finalize_resolution(&env, &market_id),
            Err(Error::DisputeTimeoutNotExpired)
        );

        // 6. Wait for window to close
        advance_time(&env, 25 * 60 * 60); // 25 hours

        // 7. Window closed, can finalize
        assert!(!ResolutionDelayManager::is_dispute_window_open(&env, &market_id));
        let (can, _) = ResolutionDelayManager::can_finalize(&env, &market_id);
        assert!(can);

        // 8. Finalize
        let final_outcome = ResolutionDelayManager::finalize_resolution(&env, &market_id).unwrap();
        assert_eq!(final_outcome, outcome);

        // 9. Verify finalized
        assert!(ResolutionDelayManager::is_resolution_finalized(&env, &market_id));

        // 10. Verify market state updated
        let market: Market = env.storage().persistent().get(&market_id).unwrap();
        assert_eq!(market.winning_outcome, Some(outcome));
        assert_eq!(market.state, MarketState::Resolved);
    }

    #[test]
    fn test_lifecycle_with_disputes() {
        let (env, admin) = setup_test_env();
        let market_id = Symbol::new(&env, "test_market");

        // Setup
        ResolutionDelayManager::set_global_dispute_window(&env, &admin, 24).unwrap();
        create_ended_market(&env, &market_id, &admin);
        ResolutionDelayManager::propose_resolution(
            &env,
            &market_id,
            String::from_str(&env, "yes"),
            String::from_str(&env, "Oracle"),
        )
        .unwrap();

        // Record some disputes during window
        ResolutionDelayManager::record_dispute(&env, &market_id).unwrap();
        ResolutionDelayManager::record_dispute(&env, &market_id).unwrap();

        // Verify dispute count
        let (is_open, _, disputes) =
            ResolutionDelayManager::get_dispute_window_status(&env, &market_id);
        assert!(is_open);
        assert_eq!(disputes, 2);

        // Advance past window
        advance_time(&env, 25 * 60 * 60);

        // Window closed
        let (is_open, _, disputes) =
            ResolutionDelayManager::get_dispute_window_status(&env, &market_id);
        assert!(!is_open);
        assert_eq!(disputes, 2);

        // Can finalize (disputes were recorded but for this test we assume they're resolved)
        let result = ResolutionDelayManager::finalize_resolution(&env, &market_id);
        assert!(result.is_ok());
    }

    #[test]
    fn test_per_market_override_config() {
        let (env, admin) = setup_test_env();
        let market_id_1 = Symbol::new(&env, "market_1");
        let market_id_2 = Symbol::new(&env, "market_2");

        // Set global to 48 hours
        ResolutionDelayManager::set_global_dispute_window(&env, &admin, 48).unwrap();

        // Create markets
        create_ended_market(&env, &market_id_1, &admin);
        create_ended_market(&env, &market_id_2, &admin);

        // Set market_1 override to 24 hours
        ResolutionDelayManager::set_market_dispute_window(&env, &admin, &market_id_1, 24).unwrap();

        // market_2 uses global (set to 0)
        ResolutionDelayManager::set_market_dispute_window(&env, &admin, &market_id_2, 0).unwrap();

        // Verify configs
        let config_1 = ResolutionDelayManager::get_dispute_window_config(&env, &market_id_1);
        assert_eq!(config_1.dispute_window_hours, 24);

        let config_2 = ResolutionDelayManager::get_dispute_window_config(&env, &market_id_2);
        assert_eq!(config_2.dispute_window_hours, 48);

        // Propose for both
        ResolutionDelayManager::propose_resolution(
            &env,
            &market_id_1,
            String::from_str(&env, "yes"),
            String::from_str(&env, "Oracle"),
        )
        .unwrap();
        ResolutionDelayManager::propose_resolution(
            &env,
            &market_id_2,
            String::from_str(&env, "yes"),
            String::from_str(&env, "Oracle"),
        )
        .unwrap();

        // After 25 hours, market_1 should be closable, market_2 still open
        advance_time(&env, 25 * 60 * 60);

        assert!(!ResolutionDelayManager::is_dispute_window_open(&env, &market_id_1));
        assert!(ResolutionDelayManager::is_dispute_window_open(&env, &market_id_2));

        // After 49 hours total, both should be closable
        advance_time(&env, 24 * 60 * 60);

        assert!(!ResolutionDelayManager::is_dispute_window_open(&env, &market_id_1));
        assert!(!ResolutionDelayManager::is_dispute_window_open(&env, &market_id_2));
    }

    #[test]
    fn test_market_state_transitions() {
        let (env, admin) = setup_test_env();
        let market_id = Symbol::new(&env, "test_market");

        ResolutionDelayManager::set_global_dispute_window(&env, &admin, 1).unwrap();
        create_ended_market(&env, &market_id, &admin);

        // Initial state: Ended
        let market: Market = env.storage().persistent().get(&market_id).unwrap();
        assert_eq!(market.state, MarketState::Ended);

        // After proposal, resolution window is set but state doesn't change yet
        ResolutionDelayManager::propose_resolution(
            &env,
            &market_id,
            String::from_str(&env, "yes"),
            String::from_str(&env, "Oracle"),
        )
        .unwrap();

        let market: Market = env.storage().persistent().get(&market_id).unwrap();
        assert!(market.is_resolution_proposed());

        // After finalization, state becomes Resolved
        advance_time(&env, 2 * 60 * 60);
        ResolutionDelayManager::finalize_resolution(&env, &market_id).unwrap();

        let market: Market = env.storage().persistent().get(&market_id).unwrap();
        assert_eq!(market.state, MarketState::Resolved);
        assert!(market.is_resolution_finalized());
    }
}
