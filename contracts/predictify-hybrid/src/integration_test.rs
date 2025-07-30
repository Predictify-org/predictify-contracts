#![cfg(test)]
use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    token::StellarAssetClient,
    vec, String, Symbol,
};
use alloc::format;

/// Simplified Integration Test Suite for Predictify Hybrid Contract
/// 
/// This module provides basic integration tests covering:
/// - Market creation and voting
/// - Basic market lifecycle
/// - Error scenarios

// ===== INTEGRATION TEST STRUCTURES =====

/// Integration Test Suite
pub struct IntegrationTestSuite {
    pub env: Env,
    pub contract_id: Address,
    pub token_id: Address,
    pub admin: Address,
    pub users: Vec<Address>,
    pub market_ids: Vec<Symbol>,
}

impl IntegrationTestSuite {
    pub fn setup(num_users: usize) -> Self {
        let env = Env::default();
        env.mock_all_auths();

        // Setup token
        let token_admin = Address::generate(&env);
        let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_id = token_contract.address();

        // Setup admin and users
        let admin = Address::generate(&env);
        let mut users = Vec::new(&env);
        for _ in 0..num_users {
            users.push_back(Address::generate(&env));
        }

        // Initialize contract
        let contract_id = env.register(PredictifyHybrid, ());
        let client = PredictifyHybridClient::new(&env, &contract_id);
        client.initialize(&admin);

        // Set token for staking
        env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .set(&Symbol::new(&env, "TokenID"), &token_id);
        });

        // Fund all users with tokens
        let stellar_client = StellarAssetClient::new(&env, &token_id);
        env.mock_all_auths();
        stellar_client.mint(&admin, &10000_0000000); // 10,000 XLM to admin
        for user in users.iter() {
            stellar_client.mint(&user, &1000_0000000); // 1,000 XLM to each user
        }

        let market_ids = Vec::new(&env);

        Self {
            env,
            contract_id,
            token_id,
            admin,
            users,
            market_ids,
        }
    }

    pub fn create_market(&mut self, question: &str, outcomes: Vec<String>, duration_days: u32) -> Symbol {
        let client = PredictifyHybridClient::new(&self.env, &self.contract_id);
        
        self.env.mock_all_auths();
        let market_id = client.create_market(
            &self.admin,
            &String::from_str(&self.env, question),
            &outcomes,
            &duration_days,
            &OracleConfig {
                provider: OracleProvider::Reflector,
                feed_id: String::from_str(&self.env, "BTC"),
                threshold: 2500000,
                comparison: String::from_str(&self.env, "gt"),
            },
        );

        self.market_ids.push_back(market_id.clone());
        market_id
    }

    pub fn vote_on_market(&self, user: &Address, market_id: &Symbol, outcome: &str, stake: i128) {
        let client = PredictifyHybridClient::new(&self.env, &self.contract_id);
        self.env.mock_all_auths();
        client.vote(
            user,
            market_id,
            &String::from_str(&self.env, outcome),
            &stake,
        );
    }

    pub fn advance_time(&self, days: u32) {
        let current_ledger = self.env.ledger();
        let new_timestamp = current_ledger.timestamp() + (days as u64 * 24 * 60 * 60);
        
        self.env.ledger().set(LedgerInfo {
            timestamp: new_timestamp,
            protocol_version: current_ledger.protocol_version(),
            sequence_number: current_ledger.sequence(),
            network_id: current_ledger.network_id().into(),
            base_reserve: 10,
            min_temp_entry_ttl: 1,
            min_persistent_entry_ttl: 1,
            max_entry_ttl: 10000,
        });
    }

    pub fn get_market(&self, market_id: &Symbol) -> Market {
        self.env.as_contract(&self.contract_id, || {
            self.env
                .storage()
                .persistent()
                .get::<Symbol, Market>(market_id)
                .unwrap()
        })
    }

    pub fn resolve_market(&self, market_id: &Symbol) -> Result<(), Error> {
        let client = PredictifyHybridClient::new(&self.env, &self.contract_id);
        self.env.mock_all_auths();
        
        // Get the market to determine the correct outcome to use
        let market = self.get_market(market_id);
        let winning_outcome = market.outcomes.get(0).unwrap().clone(); // Use first outcome as default
        
        // Use manual resolution instead of automatic oracle resolution
        client.resolve_market_manual(&self.admin, market_id, &winning_outcome);
        Ok(())
    }

    pub fn get_user(&self, index: usize) -> Address {
        self.users.get(index as u32).unwrap().clone()
    }
}

// ===== INTEGRATION TESTS =====

#[test]
fn test_complete_market_lifecycle() {
    let mut test_suite = IntegrationTestSuite::setup(5);
    
    // Step 1: Create a market
    let market_id = test_suite.create_market(
        "Will BTC reach $30,000 by end of year?",
        vec![
            &test_suite.env,
            String::from_str(&test_suite.env, "yes"),
            String::from_str(&test_suite.env, "no"),
        ],
        30,
    );

    // Step 2: Multiple users vote
    test_suite.vote_on_market(&test_suite.get_user(0), &market_id, "yes", 100_0000000); // 100 XLM
    test_suite.vote_on_market(&test_suite.get_user(1), &market_id, "yes", 50_0000000);  // 50 XLM
    test_suite.vote_on_market(&test_suite.get_user(2), &market_id, "no", 75_0000000);   // 75 XLM
    test_suite.vote_on_market(&test_suite.get_user(3), &market_id, "yes", 25_0000000);  // 25 XLM
    test_suite.vote_on_market(&test_suite.get_user(4), &market_id, "no", 60_0000000);   // 60 XLM

    // Step 3: Verify market state
    let market = test_suite.get_market(&market_id);
    assert_eq!(market.total_staked, 310_0000000); // 310 XLM total
    assert_eq!(market.state, MarketState::Active);
    assert_eq!(market.votes.len(), 5);

    // Step 4: Advance time to market end
    test_suite.advance_time(31); // Past 30-day duration

    // Step 5: Verify market has ended
    let market = test_suite.get_market(&market_id);
    assert!(market.has_ended(test_suite.env.ledger().timestamp()));

    // Step 6: Resolve market
    let resolution_result = test_suite.resolve_market(&market_id);
    assert!(resolution_result.is_ok());

    // Step 7: Verify market is resolved
    let market = test_suite.get_market(&market_id);
    assert_eq!(market.state, MarketState::Resolved);
    assert!(market.winning_outcome.is_some());
}

#[test]
fn test_multi_user_market_scenarios() {
    let mut test_suite = IntegrationTestSuite::setup(10);
    
    // Create multiple markets
    let market_1 = test_suite.create_market(
        "Market 1: BTC price prediction",
        vec![
            &test_suite.env,
            String::from_str(&test_suite.env, "above_30k"),
            String::from_str(&test_suite.env, "below_30k"),
        ],
        30,
    );

    let market_2 = test_suite.create_market(
        "Market 2: ETH price prediction",
        vec![
            &test_suite.env,
            String::from_str(&test_suite.env, "above_2k"),
            String::from_str(&test_suite.env, "below_2k"),
        ],
        45,
    );

    // Users vote on multiple markets
    for i in 0..10 {
        let user = test_suite.get_user(i);
        
        // Vote on market 1
        let outcome_1 = if i % 2 == 0 { "above_30k" } else { "below_30k" };
        test_suite.vote_on_market(&user, &market_1, outcome_1, ((i + 1) * 10) as i128 * 1_0000000);

        // Vote on market 2
        let outcome_2 = if i % 3 == 0 { "above_2k" } else { "below_2k" };
        test_suite.vote_on_market(&user, &market_2, outcome_2, ((i + 1) * 5) as i128 * 1_0000000);
    }

    // Verify all markets have votes
    let market_1_data = test_suite.get_market(&market_1);
    let market_2_data = test_suite.get_market(&market_2);

    assert_eq!(market_1_data.votes.len(), 10);
    assert_eq!(market_2_data.votes.len(), 10);

    // Advance time and resolve markets
    test_suite.advance_time(31);
    test_suite.resolve_market(&market_1).unwrap();

    test_suite.advance_time(15); // Total 46 days
    test_suite.resolve_market(&market_2).unwrap();

    // Verify all markets are resolved
    let final_market_1 = test_suite.get_market(&market_1);
    let final_market_2 = test_suite.get_market(&market_2);

    assert_eq!(final_market_1.state, MarketState::Resolved);
    assert_eq!(final_market_2.state, MarketState::Resolved);
}

#[test]
#[should_panic(expected = "Error(Contract, #101)")] // MarketNotFound
fn test_error_scenario_integration() {
    let mut test_suite = IntegrationTestSuite::setup(2);
    
    // Test 1: Try to vote on non-existent market
    let client = PredictifyHybridClient::new(&test_suite.env, &test_suite.contract_id);
    let non_existent_market = Symbol::new(&test_suite.env, "non_existent");
    
    test_suite.env.mock_all_auths();
    client.vote(
        &test_suite.get_user(0),
        &non_existent_market,
        &String::from_str(&test_suite.env, "yes"),
        &10_0000000,
    );
    // This should panic with MarketNotFound error
}

#[test]
fn test_stress_test_multiple_markets() {
    let mut test_suite = IntegrationTestSuite::setup(20);
    
    // Create 5 markets simultaneously
    let mut market_ids = Vec::new(&test_suite.env);
    for i in 0..5 {
        let market_id = test_suite.create_market(
            &format!("Stress test market {}", i),
            vec![
                &test_suite.env,
                String::from_str(&test_suite.env, "outcome_a"),
                String::from_str(&test_suite.env, "outcome_b"),
            ],
            30 + (i as u32), // Different durations
        );
        market_ids.push_back(market_id);
    }

    // All users vote on all markets
    for user_index in 0..20 {
        let user = test_suite.get_user(user_index);
        for (market_index, market_id) in market_ids.iter().enumerate() {
            let outcome = if (user_index + market_index) % 2 == 0 { "outcome_a" } else { "outcome_b" };
            let stake = ((user_index + market_index + 1) * 5) as i128 * 1_0000000;
            
                         test_suite.vote_on_market(&user, &market_id, outcome, stake);
        }
    }

    // Verify all markets have votes
    for market_id in market_ids.iter() {
        let market = test_suite.get_market(&market_id);
        assert_eq!(market.votes.len(), 20);
        assert!(market.total_staked > 0);
    }

    // Advance time and resolve all markets
    test_suite.advance_time(40); // Past all market durations

    for market_id in market_ids.iter() {
        let resolution_result = test_suite.resolve_market(&market_id);
        assert!(resolution_result.is_ok());
    }

    // Verify all markets are resolved
    for market_id in market_ids.iter() {
        let market = test_suite.get_market(&market_id);
        assert_eq!(market.state, MarketState::Resolved);
    }
}