//! # Test Suite Status
//!
//! Some tests are temporarily disabled while we maintain a simplified version of the contract.
//! The following feature tests are currently disabled:
//!
//! - Fee Management Tests: Will be re-enabled when fee collection is implemented
//! - Event Tests: Will be re-enabled when event emission is implemented
//! - Configuration Tests: Will be re-enabled when dynamic configuration is implemented
//! - Utility Tests: Will be re-enabled when utility functions are implemented
//! - Validation Tests: Will be re-enabled when advanced validation is implemented
//! - Oracle Tests: Will be re-enabled when oracle integration is implemented
//!
//! Only core functionality tests are currently active:
//! - Market Creation
//! - Basic Voting
//! - Simple Market Resolution

#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    token::{self, StellarAssetClient},
    vec, String, Symbol,
};

// Test setup structures
struct TokenTest {
    token_id: Address,
    env: Env,
}

impl TokenTest {
    fn setup() -> Self {
        let env = Env::default();
        env.mock_all_auths();
        let token_admin = Address::generate(&env);
        let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_address = token_contract.address();

        Self {
            token_id: token_address,
            env,
        }
    }
}

struct PredictifyTest {
    env: Env,
    contract_id: Address,
    token_test: TokenTest,
    admin: Address,
    user: Address,
    market_id: Symbol,
}

impl PredictifyTest {
    fn setup() -> Self {
        let token_test = TokenTest::setup();
        let env = token_test.env.clone();

        // Setup admin and user
        let admin = Address::generate(&env);
        let user = Address::generate(&env);

        // Initialize contract
        let contract_id = env.register(PredictifyHybrid, ());
        let client = PredictifyHybridClient::new(&env, &contract_id);
        client.initialize(&admin);

        // Set token for staking
        env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .set(&Symbol::new(&env, "TokenID"), &token_test.token_id);
        });

        // Fund admin and user with tokens
        let stellar_client = StellarAssetClient::new(&env, &token_test.token_id);
        env.mock_all_auths();
        stellar_client.mint(&admin, &1000_0000000); // Mint 1000 XLM to admin
        stellar_client.mint(&user, &1000_0000000); // Mint 1000 XLM to user

        // Create market ID
        let market_id = Symbol::new(&env, "market");

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

        // Create market outcomes
        let outcomes = vec![
            &self.env,
            String::from_str(&self.env, "yes"),
            String::from_str(&self.env, "no"),
        ];

        // Create market
        self.env.mock_all_auths();
        client.create_market(
            &self.admin,
            &String::from_str(&self.env, "Will BTC go above $25,000 by December 31?"),
            &outcomes,
            &30,
            &OracleConfig {
                provider: OracleProvider::Reflector,
                feed_id: String::from_str(&self.env, "BTC"),
                threshold: 2500000,
                comparison: String::from_str(&self.env, "gt"),
            },
        );
    }
}

// Core functionality tests
#[test]
fn test_create_market_successful() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);
    let duration_days = 30;
    let outcomes = vec![
        &test.env,
        String::from_str(&test.env, "yes"),
        String::from_str(&test.env, "no"),
    ];

    client.create_market(
        &test.admin,
        &String::from_str(&test.env, "Will BTC go above $25,000 by December 31?"),
        &outcomes,
        &duration_days,
        &OracleConfig {
            provider: OracleProvider::Reflector,
            feed_id: String::from_str(&test.env, "BTC"),
            threshold: 2500000,
            comparison: String::from_str(&test.env, "gt"),
        },
    );

    let market = test.env.as_contract(&test.contract_id, || {
        test.env
            .storage()
            .persistent()
            .get::<Symbol, Market>(&test.market_id)
            .unwrap()
    });

    assert_eq!(
        market.question,
        String::from_str(&test.env, "Will BTC go above $25,000 by December 31?")
    );
    assert_eq!(market.outcomes.len(), 2);
    assert_eq!(
        market.end_time,
        test.env.ledger().timestamp() + 30 * 24 * 60 * 60
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #100)")]  // Unauthorized = 100
fn test_create_market_with_non_admin() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);
    let outcomes = vec![
        &test.env,
        String::from_str(&test.env, "yes"),
        String::from_str(&test.env, "no"),
    ];

    client.create_market(
        &test.user,
        &String::from_str(&test.env, "Will BTC go above $25,000 by December 31?"),
        &outcomes,
        &30,
        &OracleConfig {
            provider: OracleProvider::Reflector,
            feed_id: String::from_str(&test.env, "BTC"),
            threshold: 2500000,
            comparison: String::from_str(&test.env, "gt"),
        },
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #301)")]  // InvalidOutcomes = 301
fn test_create_market_with_empty_outcome() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);
    let outcomes = vec![&test.env];

    client.create_market(
        &test.admin,
        &String::from_str(&test.env, "Will BTC go above $25,000 by December 31?"),
        &outcomes,
        &30,
        &OracleConfig {
            provider: OracleProvider::Reflector,
            feed_id: String::from_str(&test.env, "BTC"),
            threshold: 2500000,
            comparison: String::from_str(&test.env, "gt"),
        },
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #300)")]  // InvalidQuestion = 300
fn test_create_market_with_empty_question() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);
    let outcomes = vec![
        &test.env,
        String::from_str(&test.env, "yes"),
        String::from_str(&test.env, "no"),
    ];

    client.create_market(
        &test.admin,
        &String::from_str(&test.env, ""),
        &outcomes,
        &30,
        &OracleConfig {
            provider: OracleProvider::Reflector,
            feed_id: String::from_str(&test.env, "BTC"),
            threshold: 2500000,
            comparison: String::from_str(&test.env, "gt"),
        },
    );
}

#[test]
fn test_successful_vote() {
    let test = PredictifyTest::setup();
    test.create_test_market();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    test.env.mock_all_auths();
    client.vote(
        &test.user,
        &test.market_id,
        &String::from_str(&test.env, "yes"),
        &1_0000000,
    );

    let market = test.env.as_contract(&test.contract_id, || {
        test.env
            .storage()
            .persistent()
            .get::<Symbol, Market>(&test.market_id)
            .unwrap()
    });

    assert!(market.votes.contains_key(test.user.clone()));
    assert_eq!(market.total_staked, 1_0000000);
}

#[test]
#[should_panic(expected = "Error(Contract, #102)")]  // MarketClosed = 102
fn test_vote_on_closed_market() {
    let test = PredictifyTest::setup();
    test.create_test_market();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Get market end time and advance past it
    let market = test.env.as_contract(&test.contract_id, || {
        test.env
            .storage()
            .persistent()
            .get::<Symbol, Market>(&test.market_id)
            .unwrap()
    });

    test.env.ledger().set(LedgerInfo {
        timestamp: market.end_time + 1,
        protocol_version: 22,
        sequence_number: test.env.ledger().sequence(),
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 1,
        min_persistent_entry_ttl: 1,
        max_entry_ttl: 10000,
    });

    test.env.mock_all_auths();
    client.vote(
        &test.user,
        &test.market_id,
        &String::from_str(&test.env, "yes"),
        &1_0000000,
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #108)")]  // InvalidOutcome = 108
fn test_vote_with_invalid_outcome() {
    let test = PredictifyTest::setup();
    test.create_test_market();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    test.env.mock_all_auths();
    client.vote(
        &test.user,
        &test.market_id,
        &String::from_str(&test.env, "invalid"),
        &1_0000000,
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #101)")]  // MarketNotFound = 101
fn test_vote_on_nonexistent_market() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    let nonexistent_market = Symbol::new(&test.env, "nonexistent");
    test.env.mock_all_auths();
    client.vote(
        &test.user,
        &nonexistent_market,
        &String::from_str(&test.env, "yes"),
        &1_0000000,
    );
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]  // SDK authentication error
fn test_authentication_required() {
    let test = PredictifyTest::setup();
    test.create_test_market();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Clear any existing auths explicitly
    test.env.set_auths(&[]);

    // This call should fail because we're not providing authentication
    client.vote(
        &test.user,
        &test.market_id,
        &String::from_str(&test.env, "yes"),
        &1_0000000,
    );
}
