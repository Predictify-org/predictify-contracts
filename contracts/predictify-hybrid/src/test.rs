#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token::{Client as TokenClient, StellarAssetClient},
    vec, Symbol,
};

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
        
        Self {
            env,
            contract_id,
            token_test,
            admin,
            user,
            market_id,
        }
    }
    
    fn create_test_market(&self) {
        let client = PredictifyHybridClient::new(&self.env, &self.contract_id);
        
        // Create oracle config
        let oracle_config = OracleConfig {
            provider: OracleProvider::BandProtocol,
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
