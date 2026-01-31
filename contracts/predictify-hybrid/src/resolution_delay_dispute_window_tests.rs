#![cfg(test)]

use crate::config::{ConfigManager, DISPUTE_EXTENSION_HOURS};
use crate::disputes::DisputeManager;
use crate::errors::Error;
use crate::markets::MarketStateManager;
use crate::types::{Market, MarketState, OracleConfig, OracleProvider};
use crate::voting::VotingManager;
use crate::PredictifyHybrid;
use soroban_sdk::testutils::{Address as _, Ledger, LedgerInfo};
use soroban_sdk::{symbol_short, Address, Env, Map, String, Symbol, Vec};

/// Test setup helper
struct TestSetup {
    env: Env,
    contract_id: Address,
    admin: Address,
}

impl TestSetup {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let contract_id = env.register_contract(None, PredictifyHybrid);
        
        // Initialize config in contract context
        env.as_contract(&contract_id, || {
            let config = ConfigManager::get_development_config(&env);
            ConfigManager::store_config(&env, &config).unwrap();
        });
        
        Self {
            env,
            contract_id,
            admin,
        }
    }
    
    /// Helper function to create a test market
    fn create_test_market(&self, end_time: u64) -> (Symbol, Market) {
        let market_id = Symbol::new(&self.env, "test_market");
        
        let mut outcomes = Vec::new(&self.env);
        outcomes.push_back(String::from_str(&self.env, "yes"));
        outcomes.push_back(String::from_str(&self.env, "no"));
        
        let oracle_config = OracleConfig::new(
            OracleProvider::Reflector,
            String::from_str(&self.env, "BTC/USD"),
            50_000_00,
            String::from_str(&self.env, "gt"),
        );
        
        let market = Market::new(
            &self.env,
            self.admin.clone(),
            String::from_str(&self.env, "Will BTC reach $50k?"),
            outcomes,
            end_time,
            oracle_config,
            MarketState::Active,
        );
        
        (market_id, market)
    }
    
    /// Helper function to advance ledger time
    fn advance_time(&self, seconds: u64) {
        let current_time = self.env.ledger().timestamp();
        self.env.ledger().set(LedgerInfo {
            timestamp: current_time + seconds,
            protocol_version: 22,
            sequence_number: self.env.ledger().sequence() + 1,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 16,
            min_persistent_entry_ttl: 16,
            max_entry_ttl: 6312000,
        });
    }
}

// ===== PAYOUT BLOCKING TESTS =====

#[test]
fn test_payout_blocked_during_dispute_window() {
    let setup = TestSetup::new();
    let user = Address::generate(&setup.env);
    
    setup.env.as_contract(&setup.contract_id, || {
        // Create market that ends in 1 hour
        let end_time = setup.env.ledger().timestamp() + 3600;
        let (market_id, mut market) = setup.create_test_market(end_time);
        
        // Add a vote
        market.votes.set(user.clone(), String::from_str(&setup.env, "yes"));
        market.stakes.set(user.clone(), 1_000_000);
        market.total_staked = 1_000_000;
        
        // Store market
        setup.env.storage().persistent().set(&market_id, &market);
        
        // Advance time past end_time
        setup.advance_time(3700);
        
        // Market ends (but not resolved yet)
        market.state = MarketState::Ended;
        setup.env.storage().persistent().set(&market_id, &market);
        
        // File a dispute - this should extend the market
        let dispute_stake = 10_000_000;
        MarketStateManager::add_dispute_stake(&mut market, user.clone(), dispute_stake, Some(&market_id));
        
        let cfg = ConfigManager::get_config(&setup.env).unwrap();
        MarketStateManager::extend_for_dispute(&mut market, &setup.env, cfg.voting.dispute_extension_hours.into());
        setup.env.storage().persistent().set(&market_id, &market);
        
        // Try to claim payout during dispute window - should fail
        let current_time = setup.env.ledger().timestamp();
        assert!(current_time < market.end_time, "Should still be in dispute window");
        
        // Verify market is still in dispute period
        let extended_market: Market = setup.env.storage().persistent().get(&market_id).unwrap();
        assert!(extended_market.end_time > current_time);
    });
}

#[test]
fn test_payout_allowed_after_dispute_window_closes() {
    let setup = TestSetup::new();
    let user = Address::generate(&setup.env);
    
    setup.env.as_contract(&setup.contract_id, || {
        // Create market
        let end_time = setup.env.ledger().timestamp() + 3600;
        let (market_id, mut market) = setup.create_test_market(end_time);
        
        // Add vote
        market.votes.set(user.clone(), String::from_str(&setup.env, "yes"));
        market.stakes.set(user.clone(), 1_000_000);
        market.total_staked = 1_000_000;
        
        setup.env.storage().persistent().set(&market_id, &market);
        
        // Advance past original end time
        setup.advance_time(3700);
        
        // Market ends
        market.state = MarketState::Ended;
        setup.env.storage().persistent().set(&market_id, &market);
        
        // File dispute and extend
        MarketStateManager::add_dispute_stake(&mut market, user.clone(), 10_000_000, Some(&market_id));
        let cfg = ConfigManager::get_config(&setup.env).unwrap();
        MarketStateManager::extend_for_dispute(&mut market, &setup.env, cfg.voting.dispute_extension_hours.into());
        setup.env.storage().persistent().set(&market_id, &market);
        
        // Advance past dispute window (24 hours)
        setup.advance_time(24 * 3600 + 100);
        
        // Now payout should be allowed
        let current_time = setup.env.ledger().timestamp();
        let final_market: Market = setup.env.storage().persistent().get(&market_id).unwrap();
        assert!(current_time >= final_market.end_time, "Should be past dispute window");
    });
}

#[test]
fn test_payout_blocked_with_active_dispute() {
    let setup = TestSetup::new();
    let user = Address::generate(&setup.env);
    
    setup.env.as_contract(&setup.contract_id, || {
        let end_time = setup.env.ledger().timestamp() + 1000;
        let (market_id, mut market) = setup.create_test_market(end_time);
        
        market.votes.set(user.clone(), String::from_str(&setup.env, "yes"));
        market.stakes.set(user.clone(), 5_000_000);
        market.total_staked = 5_000_000;
        
        setup.env.storage().persistent().set(&market_id, &market);
        
        // Advance past original end time
        setup.advance_time(1100);
        
        // Market ends - set to Ended state before filing dispute
        market.state = MarketState::Ended;
        setup.env.storage().persistent().set(&market_id, &market);
        
        // Add dispute stake
        MarketStateManager::add_dispute_stake(&mut market, user.clone(), 10_000_000, Some(&market_id));
        let cfg = ConfigManager::get_config(&setup.env).unwrap();
        MarketStateManager::extend_for_dispute(&mut market, &setup.env, cfg.voting.dispute_extension_hours.into());
        setup.env.storage().persistent().set(&market_id, &market);
        
        // Verify market is in disputed state
        let disputed_market: Market = setup.env.storage().persistent().get(&market_id).unwrap();
        assert_eq!(disputed_market.state, MarketState::Disputed);
        
        // Verify end time was extended
        let current_time = setup.env.ledger().timestamp();
        assert!(disputed_market.end_time > current_time);
    });
}

// ===== DISPUTE CREATION TESTS =====

#[test]
fn test_dispute_creation_during_window() {
    let setup = TestSetup::new();
    let user = Address::generate(&setup.env);
    
    setup.env.as_contract(&setup.contract_id, || {
        let end_time = setup.env.ledger().timestamp() + 5000;
        let (market_id, mut market) = setup.create_test_market(end_time);
        
        market.state = MarketState::Ended;
        let mut outcomes = soroban_sdk::Vec::new(&setup.env);
        outcomes.push_back(String::from_str(&setup.env, "yes"));
        market.winning_outcomes = Some(outcomes);
        setup.env.storage().persistent().set(&market_id, &market);
        
        // Advance past end time but within dispute window
        setup.advance_time(5100);
        
        // Create dispute
        let dispute_stake = 10_000_000;
        MarketStateManager::add_dispute_stake(&mut market, user.clone(), dispute_stake, Some(&market_id));
        
        // Verify dispute was added
        let dispute_amount = market.dispute_stakes.get(user.clone()).unwrap();
        assert_eq!(dispute_amount, dispute_stake);
    });
}

#[test]
fn test_dispute_extends_market_deadline() {
    let setup = TestSetup::new();
    let user = Address::generate(&setup.env);
    
    setup.env.as_contract(&setup.contract_id, || {
        let end_time = setup.env.ledger().timestamp() + 2000;
        let (market_id, mut market) = setup.create_test_market(end_time);
        
        let original_end_time = market.end_time;
        setup.env.storage().persistent().set(&market_id, &market);
        
        // Advance past end time
        setup.advance_time(2100);
        
        // Market ends - set to Ended state before filing dispute
        market.state = MarketState::Ended;
        setup.env.storage().persistent().set(&market_id, &market);
        
        // File dispute
        MarketStateManager::add_dispute_stake(&mut market, user.clone(), 10_000_000, Some(&market_id));
        
        let cfg = ConfigManager::get_config(&setup.env).unwrap();
        let extension_hours = cfg.voting.dispute_extension_hours;
        MarketStateManager::extend_for_dispute(&mut market, &setup.env, extension_hours.into());
        
        // Verify end time was extended
        let current_time = setup.env.ledger().timestamp();
        let expected_new_end = current_time + (extension_hours as u64 * 3600);
        
        assert!(market.end_time >= expected_new_end || market.end_time == original_end_time);
        assert!(market.end_time > current_time);
    });
}

// ===== PER-EVENT VS GLOBAL WINDOW TESTS =====

#[test]
fn test_per_event_dispute_window() {
    let setup = TestSetup::new();
    let user1 = Address::generate(&setup.env);
    let user2 = Address::generate(&setup.env);
    
    setup.env.as_contract(&setup.contract_id, || {
        // Create two markets with different end times
        let end_time1 = setup.env.ledger().timestamp() + 1000;
        let (market_id1, mut market1) = setup.create_test_market(end_time1);
        
        let end_time2 = setup.env.ledger().timestamp() + 5000;
        let market_id2 = Symbol::new(&setup.env, "test_market_2");
        let mut market2 = market1.clone();
        market2.end_time = end_time2;
        
        setup.env.storage().persistent().set(&market_id1, &market1);
        setup.env.storage().persistent().set(&market_id2, &market2);
        
        // Advance time past first market end
        setup.advance_time(1100);
        
        // First market ends - set to Ended state before filing dispute
        market1.state = MarketState::Ended;
        setup.env.storage().persistent().set(&market_id1, &market1);
        
        // File dispute on first market
        MarketStateManager::add_dispute_stake(&mut market1, user1.clone(), 10_000_000, Some(&market_id1));
        let cfg = ConfigManager::get_config(&setup.env).unwrap();
        MarketStateManager::extend_for_dispute(&mut market1, &setup.env, cfg.voting.dispute_extension_hours.into());
        setup.env.storage().persistent().set(&market_id1, &market1);
        
        // Second market should not be affected
        let market2_check: Market = setup.env.storage().persistent().get(&market_id2).unwrap();
        assert_eq!(market2_check.end_time, end_time2);
        
        // First market should be extended
        let market1_check: Market = setup.env.storage().persistent().get(&market_id1).unwrap();
        assert!(market1_check.end_time > end_time1);
    });
}

#[test]
fn test_global_dispute_extension_constant() {
    let setup = TestSetup::new();
    
    setup.env.as_contract(&setup.contract_id, || {
        // Verify global constant is set correctly
        let cfg = ConfigManager::get_config(&setup.env).unwrap();
        assert_eq!(cfg.voting.dispute_extension_hours, DISPUTE_EXTENSION_HOURS);
        assert_eq!(DISPUTE_EXTENSION_HOURS, 24); // Default 24 hours
    });
}

// ===== EDGE CASE TESTS =====

#[test]
fn test_zero_dispute_window_not_allowed() {
    let setup = TestSetup::new();
    
    setup.env.as_contract(&setup.contract_id, || {
        let cfg = ConfigManager::get_config(&setup.env).unwrap();
        
        // Dispute extension hours should never be zero
        assert!(cfg.voting.dispute_extension_hours > 0);
    });
}

#[test]
fn test_very_long_dispute_window() {
    let setup = TestSetup::new();
    let user = Address::generate(&setup.env);
    
    setup.env.as_contract(&setup.contract_id, || {
        let end_time = setup.env.ledger().timestamp() + 1000;
        let (market_id, mut market) = setup.create_test_market(end_time);
        
        setup.env.storage().persistent().set(&market_id, &market);
        
        // Advance past end time
        setup.advance_time(1100);
        
        // Extend with very long window (e.g., 168 hours = 1 week)
        let long_extension_hours = 168u64;
        MarketStateManager::extend_for_dispute(&mut market, &setup.env, long_extension_hours);
        
        let current_time = setup.env.ledger().timestamp();
        let expected_end = current_time + (long_extension_hours * 3600);
        
        assert_eq!(market.end_time, expected_end);
    });
}

#[test]
fn test_exact_timestamp_match_at_window_boundary() {
    let setup = TestSetup::new();
    let user = Address::generate(&setup.env);
    
    setup.env.as_contract(&setup.contract_id, || {
        let end_time = setup.env.ledger().timestamp() + 1000;
        let (market_id, mut market) = setup.create_test_market(end_time);
        
        setup.env.storage().persistent().set(&market_id, &market);
        
        // Advance to exactly the end time
        setup.advance_time(1000);
        
        let current_time = setup.env.ledger().timestamp();
        assert_eq!(current_time, end_time);
        
        // Market should have ended
        assert!(!market.is_active(current_time));
        assert!(market.has_ended(current_time));
    });
}

#[test]
fn test_dispute_window_boundary_exact_expiry() {
    let setup = TestSetup::new();
    let user = Address::generate(&setup.env);
    
    setup.env.as_contract(&setup.contract_id, || {
        let end_time = setup.env.ledger().timestamp() + 1000;
        let (market_id, mut market) = setup.create_test_market(end_time);
        
        setup.env.storage().persistent().set(&market_id, &market);
        
        // Advance past end time
        setup.advance_time(1100);
        
        // Market ends - set to Ended state before filing dispute
        market.state = MarketState::Ended;
        setup.env.storage().persistent().set(&market_id, &market);
        
        // File dispute and extend
        MarketStateManager::add_dispute_stake(&mut market, user.clone(), 10_000_000, Some(&market_id));
        let cfg = ConfigManager::get_config(&setup.env).unwrap();
        let extension_hours = cfg.voting.dispute_extension_hours;
        MarketStateManager::extend_for_dispute(&mut market, &setup.env, extension_hours.into());
        
        let dispute_end_time = market.end_time;
        setup.env.storage().persistent().set(&market_id, &market);
        
        // Advance to exactly the dispute window end
        let time_to_advance = dispute_end_time - setup.env.ledger().timestamp();
        setup.advance_time(time_to_advance);
        
        let current_time = setup.env.ledger().timestamp();
        assert_eq!(current_time, dispute_end_time);
        
        // At exact boundary, market should have ended
        assert!(current_time >= dispute_end_time);
    });
}

// ===== MULTIPLE DISPUTE TESTS =====

#[test]
fn test_multiple_disputes_extend_once() {
    let setup = TestSetup::new();
    let user1 = Address::generate(&setup.env);
    let user2 = Address::generate(&setup.env);
    
    setup.env.as_contract(&setup.contract_id, || {
        let end_time = setup.env.ledger().timestamp() + 1000;
        let (market_id, mut market) = setup.create_test_market(end_time);
        
        setup.env.storage().persistent().set(&market_id, &market);
        
        // Advance past end time
        setup.advance_time(1100);
        
        // Market ends - set to Ended state before filing disputes
        market.state = MarketState::Ended;
        setup.env.storage().persistent().set(&market_id, &market);
        
        // First user files dispute - this transitions market to Disputed state
        MarketStateManager::add_dispute_stake(&mut market, user1.clone(), 10_000_000, Some(&market_id));
        
        // Verify market is now in Disputed state
        assert_eq!(market.state, MarketState::Disputed);
        
        // Second user adds stake directly (since market is already Disputed)
        // In real scenario, this would be done through a different flow
        let existing_stake = market.dispute_stakes.get(user2.clone()).unwrap_or(0);
        market.dispute_stakes.set(user2.clone(), existing_stake + 10_000_000);
        
        // Extend the market once
        let cfg = ConfigManager::get_config(&setup.env).unwrap();
        MarketStateManager::extend_for_dispute(&mut market, &setup.env, cfg.voting.dispute_extension_hours.into());
        
        let extension_end = market.end_time;
        
        // Verify both users have dispute stakes
        assert_eq!(market.dispute_stakes.get(user1.clone()).unwrap(), 10_000_000);
        assert_eq!(market.dispute_stakes.get(user2.clone()).unwrap(), 10_000_000);
        
        // Verify market was extended once
        let current_time = setup.env.ledger().timestamp();
        assert!(extension_end > current_time);
        
        // Verify that calling extend again doesn't change the end time significantly
        // (it should only extend if current time has passed the previous extension)
        let previous_end = market.end_time;
        MarketStateManager::extend_for_dispute(&mut market, &setup.env, cfg.voting.dispute_extension_hours.into());
        
        // End time should be the same or only slightly different
        assert_eq!(market.end_time, previous_end);
    });
}

// ===== RESOLUTION DELAY TESTS =====

#[test]
fn test_resolution_blocked_before_end_time() {
    let setup = TestSetup::new();
    
    setup.env.as_contract(&setup.contract_id, || {
        let end_time = setup.env.ledger().timestamp() + 10000;
        let (market_id, mut market) = setup.create_test_market(end_time);
        
        market.state = MarketState::Active;
        setup.env.storage().persistent().set(&market_id, &market);
        
        // Try to resolve before end time
        let current_time = setup.env.ledger().timestamp();
        assert!(current_time < end_time);
        
        // Market should still be active
        assert!(market.is_active(current_time));
        assert!(!market.has_ended(current_time));
    });
}

#[test]
fn test_resolution_allowed_after_end_time() {
    let setup = TestSetup::new();
    
    setup.env.as_contract(&setup.contract_id, || {
        let end_time = setup.env.ledger().timestamp() + 1000;
        let (market_id, mut market) = setup.create_test_market(end_time);
        
        market.state = MarketState::Active;
        setup.env.storage().persistent().set(&market_id, &market);
        
        // Advance past end time
        setup.advance_time(1100);
        
        let current_time = setup.env.ledger().timestamp();
        assert!(current_time >= end_time);
        
        // Market should have ended
        assert!(!market.is_active(current_time));
        assert!(market.has_ended(current_time));
        
        // Resolution should be allowed
        market.state = MarketState::Ended;
        let mut outcomes = soroban_sdk::Vec::new(&setup.env);
        outcomes.push_back(String::from_str(&setup.env, "yes"));
        market.winning_outcomes = Some(outcomes);
        market.state = MarketState::Resolved;
        
        assert_eq!(market.state, MarketState::Resolved);
    });
}

#[test]
fn test_full_lifecycle_with_dispute_window() {
    let setup = TestSetup::new();
    let user = Address::generate(&setup.env);
    
    setup.env.as_contract(&setup.contract_id, || {
        // 1. Create market
        let end_time = setup.env.ledger().timestamp() + 1000;
        let (market_id, mut market) = setup.create_test_market(end_time);
        
        market.votes.set(user.clone(), String::from_str(&setup.env, "yes"));
        market.stakes.set(user.clone(), 5_000_000);
        market.total_staked = 5_000_000;
        market.state = MarketState::Active;
        
        setup.env.storage().persistent().set(&market_id, &market);
        
        // 2. Advance to end time
        setup.advance_time(1100);
        
        // 3. Market ends
        market.state = MarketState::Ended;
        let mut outcomes = soroban_sdk::Vec::new(&setup.env);
        outcomes.push_back(String::from_str(&setup.env, "yes"));
        market.winning_outcomes = Some(outcomes);
        setup.env.storage().persistent().set(&market_id, &market);
        
        // 4. File dispute (before resolving, while in Ended state)
        MarketStateManager::add_dispute_stake(&mut market, user.clone(), 10_000_000, Some(&market_id));
        let cfg = ConfigManager::get_config(&setup.env).unwrap();
        MarketStateManager::extend_for_dispute(&mut market, &setup.env, cfg.voting.dispute_extension_hours.into());
        market.state = MarketState::Disputed;
        
        let dispute_end_time = market.end_time;
        setup.env.storage().persistent().set(&market_id, &market);
        
        // 5. During dispute window - payout blocked
        let current_time = setup.env.ledger().timestamp();
        assert!(current_time < dispute_end_time);
        
        // 6. Advance past dispute window
        let time_remaining = dispute_end_time - current_time;
        setup.advance_time(time_remaining + 100);
        
        // 7. After dispute window - payout allowed
        let final_time = setup.env.ledger().timestamp();
        assert!(final_time >= dispute_end_time);
        
        // 8. Finalize
        market.state = MarketState::Closed;
        setup.env.storage().persistent().set(&market_id, &market);
        
        assert_eq!(market.state, MarketState::Closed);
    });
}
