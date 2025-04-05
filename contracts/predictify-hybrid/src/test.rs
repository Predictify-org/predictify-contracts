#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token::{Client as TokenClient, StellarAssetClient},
    vec, Symbol,
};
use soroban_sdk::testutils::LedgerInfo;
use core::string::ToString;

struct TokenTest<'a> {
    token_id: Address,
    token_client: TokenClient<'a>,
    env: Env,
}

impl<'a> TokenTest<'a> {
    fn setup() -> Self {
        let env = Env::default();
        let token_admin = Address::generate(&env);
        let token_id = env.register_stellar_asset_contract(token_admin.clone());
        let token_client = TokenClient::new(&env, &token_id);
        
        Self {
            token_id,
            token_client,
            env,
        }
    }
}

struct PredictifyTest<'a> {
    env: Env,
    contract_id: Address,
    token_test: TokenTest<'a>,
    admin: Address,
    user: Address,
    market_id: Symbol,
    pyth_contract: Address,
}

impl<'a> PredictifyTest<'a> {
    fn setup() -> Self {
        let token_test = TokenTest::setup();
        let env = token_test.env.clone();
        
        // Setup admin and user
        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        
        // Initialize contract
        let contract_id = env.register_contract(None, PredictifyHybrid);
        let client = PredictifyHybridClient::new(&env, &contract_id);
        client.initialize(&admin);
        
        // Set token for staking
        env.as_contract(&contract_id, || {
            env.storage().persistent().set(
                &Symbol::new(&env, "TokenID"), 
                &token_test.token_id
            );
        });
        
        // Fund user with tokens - mock auth for the token admin
        let stellar_client = StellarAssetClient::new(&env, &token_test.token_id);
        env.mock_all_auths();
        stellar_client.mint(&user, &1000);
        
        // Create market ID
        let market_id = Symbol::new(&env, "test_market");
        
        // Create a mock Pyth oracle contract
        let pyth_contract = Address::generate(&env);
        
        Self {
            env,
            contract_id,
            token_test,
            admin,
            user,
            market_id,
            pyth_contract,
        }
    }
    
    fn create_test_market(&self) {
        let client = PredictifyHybridClient::new(&self.env, &self.contract_id);
        
        // Create oracle config for Pyth
        let oracle_config = OracleConfig {
            provider: OracleProvider::Pyth,
            feed_id: String::from_str(&self.env, "BTC/USD"),
            threshold: 25_000_00, // $25,000
            comparison: String::from_str(&self.env, "gt"),
        };
        
        // Create market outcomes
        let outcomes = vec![
            &self.env,
            String::from_str(&self.env, "yes"),
            String::from_str(&self.env, "no"),
        ];
        
        // Create market question
        let question = String::from_str(&self.env, "Will BTC go above $25,000 by December 31?");
        
        // Set end time to 30 days from now
        let current_time = self.env.ledger().timestamp();
        let end_time = current_time + 30 * 24 * 60 * 60; // 30 days in seconds
        
        // Create market
        self.env.mock_all_auths();
        client.create_market(
            &self.admin,
            &self.market_id,
            &question,
            &outcomes,
            &end_time,
            &oracle_config,
        );
    }
}

#[test]
fn test_successful_vote() {
    // Setup test environment
    let test = PredictifyTest::setup();
    test.create_test_market();
    
    // Create contract client
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);
    
    // Check initial balance
    let user_balance_before = test.token_test.token_client.balance(&test.user);
    let contract_balance_before = test.token_test.token_client.balance(&test.contract_id);
    
    // Set staking amount
    let stake_amount: i128 = 100;
    
    // Vote on the market
    test.env.mock_all_auths();
    client.vote(
        &test.user,
        &test.market_id,
        &String::from_str(&test.env, "yes"),
        &stake_amount,
    );
    
    // Verify token transfer
    let user_balance_after = test.token_test.token_client.balance(&test.user);
    let contract_balance_after = test.token_test.token_client.balance(&test.contract_id);
    
    assert_eq!(user_balance_before - stake_amount, user_balance_after);
    assert_eq!(contract_balance_before + stake_amount, contract_balance_after);
    
    // Verify vote was recorded
    let market = test.env.as_contract(&test.contract_id, || {
        test.env.storage().persistent().get::<Symbol, Market>(&test.market_id).unwrap()
    });
    
    assert_eq!(market.votes.get(test.user.clone()).unwrap(), String::from_str(&test.env, "yes"));
    assert_eq!(market.total_staked, stake_amount);
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn test_vote_on_closed_market() {
    // Setup test environment
    let test = PredictifyTest::setup();
    test.create_test_market();
    
    // Get market to find out its end time
    let market = test.env.as_contract(&test.contract_id, || {
        test.env.storage().persistent().get::<Symbol, Market>(&test.market_id).unwrap()
    });
    
    // Advance ledger past the end time
    test.env.ledger().set_timestamp(market.end_time + 1);
    
    // Attempt to vote on the closed market (should fail)
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);
    test.env.mock_all_auths();
    client.vote(
        &test.user,
        &test.market_id,
        &String::from_str(&test.env, "yes"),
        &100,
    );
}

#[test]
#[should_panic(expected = "Invalid outcome")]
fn test_vote_with_invalid_outcome() {
    // Setup test environment
    let test = PredictifyTest::setup();
    test.create_test_market();
    
    // Attempt to vote with an invalid outcome
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);
    test.env.mock_all_auths();
    client.vote(
        &test.user,
        &test.market_id,
        &String::from_str(&test.env, "maybe"), // Invalid outcome
        &100,
    );
}

#[test]
#[should_panic(expected = "Market not found")]
fn test_vote_on_nonexistent_market() {
    // Setup test environment
    let test = PredictifyTest::setup();
    // Don't create a market
    
    // Attempt to vote on a non-existent market
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);
    test.env.mock_all_auths();
    client.vote(
        &test.user,
        &Symbol::new(&test.env, "nonexistent_market"),
        &String::from_str(&test.env, "yes"),
        &100,
    );
}

#[test]
#[should_panic(expected = "Unauthorized")]
fn test_authentication_required() {
    // Setup test environment
    let test = PredictifyTest::setup();
    test.create_test_market();
    
    // Register a direct client that doesn't go through the client SDK
    // which would normally automatic auth checks
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);
    
    // Clear any existing auths
    test.env.set_auths(&[]);
    
    // This call should fail because we're not providing authentication
    client.vote(
        &test.user,
        &test.market_id,
        &String::from_str(&test.env, "yes"),
        &100,
    );
}

#[test]
fn test_fetch_oracle_result() {
    // Setup test environment
    let test = PredictifyTest::setup();
    test.create_test_market();
    
    // Create contract client
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);
    
    // Get market to find out its end time
    let market = test.env.as_contract(&test.contract_id, || {
        test.env.storage().persistent().get::<Symbol, Market>(&test.market_id).unwrap()
    });
    
    // Advance ledger past the end time
    test.env.ledger().set_timestamp(market.end_time + 1);
    
    // Fetch oracle result
    let result = client.fetch_oracle_result(
        &test.market_id,
        &test.pyth_contract,
    );
    
    // Verify the result is "yes" (since our mock returns 26,000 which is > 25,000)
    assert_eq!(result, String::from_str(&test.env, "yes"));
    
    // Verify the result was stored in the market
    let market_after = test.env.as_contract(&test.contract_id, || {
        test.env.storage().persistent().get::<Symbol, Market>(&test.market_id).unwrap()
    });
    
    // The oracle result should be set to "yes"
    assert_eq!(market_after.oracle_result, Some(String::from_str(&test.env, "yes")));
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn test_fetch_oracle_result_market_not_ended() {
    // Setup test environment
    let test = PredictifyTest::setup();
    test.create_test_market();
    
    // Create contract client
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);
    
    // Try to fetch oracle result before market ends (should fail with MarketClosed error)
    client.fetch_oracle_result(
        &test.market_id,
        &test.pyth_contract,
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn test_fetch_oracle_result_already_resolved() {
    // Setup test environment
    let test = PredictifyTest::setup();
    test.create_test_market();
    
    // Create contract client
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);
    
    // Get market info
    let market = test.env.as_contract(&test.contract_id, || {
        test.env.storage().persistent().get::<Symbol, Market>(&test.market_id).unwrap()
    });
    
    // Advance ledger past the end time
    test.env.ledger().set_timestamp(market.end_time + 1);
    
    // Fetch oracle result first time
    let _ = client.fetch_oracle_result(
        &test.market_id,
        &test.pyth_contract,
    );
    
    // Try to fetch oracle result again (should fail with MarketAlreadyResolved error)
    client.fetch_oracle_result(
        &test.market_id,
        &test.pyth_contract,
    );
}




#[test]
fn test_dispute_result() {
    // Setup the environment and contracts
    let env = Env::default();
    let market_admin = Address::generate(&env);
    let user = Address::generate(&env);
    
    // Setup token contract for stake transfers
    let token_admin = Address::generate(&env);
    let token = create_token_contract(&env, &token_admin);
    
    // Create contract instance
    let contract = create_contract_instance(&env, &market_admin);
    
    // Register token ID for staking
    contract.set_token_contract(&token.address);
    
    // Mint tokens to user for testing
    token.mint(&user, &1000_0000000); // 1000 XLM
    
    // Create a market
    let market_id = Symbol::new(&env, "TEST_MARKET");
    let now = 12345000;
    let end_time = now + 3600; // Market ends in 1 hour
    
    // Set the current time to before market end
    env.ledger().set(LedgerInfo {
        timestamp: now,
        protocol_version: 20,
        sequence_number: 10,
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 10,
        min_persistent_entry_ttl: 10,
        max_entry_ttl: 3110400,
    });
    
    // Create market with initial conditions
    contract.create_market(
        &market_admin,
        &market_id,
        &"Test Market".to_string(&env),
        &end_time,
        &vec![&env, "YES".to_string(&env), "NO".to_string(&env)]
    );
    
    // Test 1: Should fail - Cannot dispute before market ends
    let result = std::panic::catch_unwind(|| {
        contract.dispute_result(&user, &market_id, &20_0000000);
    });
    assert!(result.is_err(), "Should not allow disputes before market end time");
    
    // Fast forward time to after market end
    env.ledger().set(LedgerInfo {
        timestamp: end_time + 60, // 1 minute after market end
        protocol_version: 20,
        sequence_number: 11,
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 10,
        min_persistent_entry_ttl: 10,
        max_entry_ttl: 3110400,
    });
    
    // Test 2: Should fail - Insufficient stake
    let result = std::panic::catch_unwind(|| {
        contract.dispute_result(&user, &market_id, &5_0000000); // 5 XLM (less than minimum)
    });
    assert!(result.is_err(), "Should not allow disputes with insufficient stake");
    
    // Authorize the user for token transfer
    env.mock_all_auths();
    
    // Test 3: Successful dispute with sufficient stake
    let stake_amount = 20_0000000; // 20 XLM
    let user_balance_before = token.balance(&user);
    let contract_balance_before = token.balance(&contract.address);
    
    // Record market end time before dispute
    let market_before = contract.get_market(&market_id);
    let original_end_time = market_before.end_time;
    
    // Submit dispute
    contract.dispute_result(&user, &market_id, &stake_amount);
    
    // Verify token transfer
    let user_balance_after = token.balance(&user);
    let contract_balance_after = token.balance(&contract.address);
    assert_eq!(user_balance_before - stake_amount, user_balance_after, "Incorrect user balance after dispute");
    assert_eq!(contract_balance_before + stake_amount, contract_balance_after, "Incorrect contract balance after dispute");
    
    // Verify market state
    let market_after = contract.get_market(&market_id);
    
    // Verify dispute stake recorded
    let user_stake = market_after.dispute_stakes.get(user.clone()).unwrap();
    assert_eq!(user_stake, stake_amount, "Dispute stake not recorded correctly");
    
    // Test 4: Verify market extension by 24 hours
    let expected_new_end_time = (end_time + 60) + (24 * 60 * 60);
    assert_eq!(market_after.end_time, expected_new_end_time, "Market end time not extended by 24 hours");
    
    // Test 5: Add additional stake for the same user
    let additional_stake = 30_0000000; // 30 XLM
    contract.dispute_result(&user, &market_id, &additional_stake);
    
    // Verify combined stake
    let market_after_additional_stake = contract.get_market(&market_id);
    let combined_stake = market_after_additional_stake.dispute_stakes.get(user.clone()).unwrap();
    assert_eq!(combined_stake, stake_amount + additional_stake, "Combined dispute stake not recorded correctly");
}

// Helper function to create a token contract
fn create_token_contract(env: &Env, admin: &Address) -> token::Client {
    // Deploy the token contract
    let token_contract = token::Client::new(env, &env.register_contract(None, token::Token {}));
    
    // Initialize the token contract
    token_contract.initialize(
        admin,
        &"Test Token".to_string(env),
        &"TEST".to_string(env),
        &7,
    );
    
    token_contract
}

// Helper function to create the prediction market contract
fn create_contract_instance(env: &Env, admin: &Address) -> Client {
    // Deploy the prediction market contract
    let contract_id = env.register_contract(None, PredictionMarket {});
    let client = Client::new(env, &contract_id);
    
    // Initialize the contract
    client.initialize(admin);
    
    client
}