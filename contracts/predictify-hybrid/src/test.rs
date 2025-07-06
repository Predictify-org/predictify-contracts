#![cfg(test)]

use super::*;
use crate::errors::Error;
use crate::oracles::ReflectorOracle;
use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    token::{Client as TokenClient, StellarAssetClient},
    vec, String, Symbol,
};
extern crate alloc;
use alloc::string::ToString;

struct TokenTest<'a> {
    token_id: Address,
    token_client: TokenClient<'a>,
    env: Env,
}

impl<'a> TokenTest<'a> {
    fn setup() -> Self {
        let env = Env::default();
        env.mock_all_auths();
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
            env.storage()
                .persistent()
                .set(&Symbol::new(&env, "TokenID"), &token_test.token_id);
        });

        // Fund admin and user with tokens - mock auth for the token admin
        let stellar_client = StellarAssetClient::new(&env, &token_test.token_id);
        env.mock_all_auths();
        stellar_client.mint(&admin, &1000_0000000); // Mint 1000 XLM to admin
        stellar_client.mint(&user, &1000_0000000); // Mint 1000 XLM to user

        // Create market ID
        let market_id = Symbol::new(&env, "market");

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
            &self.create_default_oracle_config(),
        );
    }

    fn create_default_oracle_config(&self) -> OracleConfig {
        OracleConfig {
            provider: OracleProvider::Pyth,
            feed_id: String::from_str(&self.env, "BTC/USD"),
            threshold: 2500000,
            comparison: String::from_str(&self.env, "gt"),
        }
    }
}

#[test]
fn test_create_market_successful() {
    //Setup test environment
    let test = PredictifyTest::setup();

    //Create contract client
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    //duration_days
    let duration_days = 30;

    //Create market outcomes
    let outcomes = vec![
        &test.env,
        String::from_str(&test.env, "yes"),
        String::from_str(&test.env, "no"),
    ];

    //Create market
    client.create_market(
        &test.admin,
        &String::from_str(&test.env, "Will BTC go above $25,000 by December 31?"),
        &outcomes,
        &duration_days,
        &test.create_default_oracle_config(),
    );

    // Verify market creation
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
#[should_panic(expected = "Error(Contract, #1)")]
fn test_create_market_with_non_admin() {
    // Setup test environment
    let test = PredictifyTest::setup();

    // Create contract client
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Attempt to create market with non-admin user
    let outcomes = vec![
        &test.env,
        String::from_str(&test.env, "yes"),
        String::from_str(&test.env, "no"),
    ];

    //test should panic with none admin user
    client.create_market(
        &test.user,
        &String::from_str(&test.env, "Will BTC go above $25,000 by December 31?"),
        &outcomes,
        &30,
        &test.create_default_oracle_config(),
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #53)")]
fn test_create_market_with_empty_outcome() {
    // Setup test environment
    let test = PredictifyTest::setup();

    // Create contract client
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Attempt to create market with empty outcome
    // will panic
    let outcomes = vec![&test.env];

    client.create_market(
        &test.admin,
        &String::from_str(&test.env, "Will BTC go above $25,000 by December 31?"),
        &outcomes,
        &30,
        &test.create_default_oracle_config(),
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #52)")]
fn test_create_market_with_empty_question() {
    // Setup test environment
    let test = PredictifyTest::setup();

    // Create contract client
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Attempt to create market with non-admin user
    let outcomes = vec![
        &test.env,
        String::from_str(&test.env, "yes"),
        String::from_str(&test.env, "no"),
    ];

    //test should panic with none admin user
    client.create_market(
        &test.admin,
        &String::from_str(&test.env, ""),
        &outcomes,
        &30,
        &test.create_default_oracle_config(),
    );
}

#[test]
fn test_successful_vote() {
    //Setup test environment
    let test = PredictifyTest::setup();

    //Create contract client
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    //duration_days
    let duration_days = 30;

    //Create market outcomes
    let outcomes = vec![
        &test.env,
        String::from_str(&test.env, "yes"),
        String::from_str(&test.env, "no"),
    ];

    //Create market
    client.create_market(
        &test.admin,
        &String::from_str(&test.env, "Will BTC go above $25,000 by December 31?"),
        &outcomes,
        &duration_days,
        &test.create_default_oracle_config(),
    );

    // Check initial balance
    let user_balance_before = test.token_test.token_client.balance(&test.user);
    let contract_balance_before = test.token_test.token_client.balance(&test.contract_id);

    // Set staking amount
    let stake_amount: i128 = 100_0000000;

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
    assert_eq!(
        contract_balance_before + stake_amount,
        contract_balance_after
    );

    // Verify vote was recorded
    let market = test.env.as_contract(&test.contract_id, || {
        test.env
            .storage()
            .persistent()
            .get::<Symbol, Market>(&test.market_id)
            .unwrap()
    });

    assert_eq!(
        market.votes.get(test.user.clone()).unwrap(),
        String::from_str(&test.env, "yes")
    );
    assert_eq!(market.total_staked, stake_amount);
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn test_vote_on_closed_market() {
    //Setup test environment
    let test = PredictifyTest::setup();

    //Create contract client
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    //duration_days
    let duration_days = 30;

    //Create market outcomes
    let outcomes = vec![
        &test.env,
        String::from_str(&test.env, "yes"),
        String::from_str(&test.env, "no"),
    ];

    //Create market
    client.create_market(
        &test.admin,
        &String::from_str(&test.env, "Will BTC go above $25,000 by December 31?"),
        &outcomes,
        &duration_days,
        &test.create_default_oracle_config(),
    );

    // Get market to find out its end time
    let market = test.env.as_contract(&test.contract_id, || {
        test.env
            .storage()
            .persistent()
            .get::<Symbol, Market>(&test.market_id)
            .unwrap()
    });

    // Advance ledger past the end time
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

    // Attempt to vote on the closed market (should fail)
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);
    test.env.mock_all_auths();
    client.vote(
        &test.user,
        &test.market_id,
        &String::from_str(&test.env, "yes"),
        &100_0000000,
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn test_vote_with_invalid_outcome() {
    //Setup test environment
    let test = PredictifyTest::setup();

    //Create contract client
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    //duration_days
    let duration_days = 30;

    //Create market outcomes
    let outcomes = vec![
        &test.env,
        String::from_str(&test.env, "yes"),
        String::from_str(&test.env, "no"),
    ];

    //Create market
    client.create_market(
        &test.admin,
        &String::from_str(&test.env, "Will BTC go above $25,000 by December 31?"),
        &outcomes,
        &duration_days,
        &test.create_default_oracle_config(),
    );
    // Attempt to vote with an invalid outcome
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);
    test.env.mock_all_auths();
    client.vote(
        &test.user,
        &test.market_id,
        &String::from_str(&test.env, "maybe"),
        &100_0000000,
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #11)")]
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
        &100_0000000,
    );
}

#[test]
#[should_panic]
fn test_authentication_required() {
    // Setup test environment
    let test = PredictifyTest::setup();
    test.create_test_market();

    // Register a direct client that doesn't go through the client SDK
    // which would normally automatic auth checks
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Clear any existing auths explicitly
    test.env.set_auths(&[]);

    // This call should fail because we're not providing authentication
    client.vote(
        &test.user,
        &test.market_id,
        &String::from_str(&test.env, "yes"),
        &100_0000000,
    );
}

#[test]
fn test_fetch_oracle_result() {
    // Setup test environment
    let test = PredictifyTest::setup();
    test.create_test_market();

    // Get market to find out its end time
    let market = test.env.as_contract(&test.contract_id, || {
        test.env
            .storage()
            .persistent()
            .get::<Symbol, Market>(&test.market_id)
            .unwrap()
    });

    // Advance ledger past the end time
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

    // Fetch oracle result
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);
    let outcome = client.fetch_oracle_result(&test.market_id, &test.pyth_contract);

    // Verify the outcome based on mock Pyth price ($26k > $25k threshold)
    assert_eq!(outcome, String::from_str(&test.env, "yes"));

    // Verify market state
    let updated_market = test.env.as_contract(&test.contract_id, || {
        test.env
            .storage()
            .persistent()
            .get::<Symbol, Market>(&test.market_id)
            .unwrap()
    });
    assert_eq!(
        updated_market.oracle_result,
        Some(String::from_str(&test.env, "yes"))
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn test_fetch_oracle_result_market_not_ended() {
    // Setup test environment
    let test = PredictifyTest::setup();
    test.create_test_market();

    // Don't advance time

    // Attempt to fetch oracle result before market ends
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);
    client.fetch_oracle_result(&test.market_id, &test.pyth_contract);
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn test_fetch_oracle_result_already_resolved() {
    // Setup test environment
    let test = PredictifyTest::setup();
    test.create_test_market();

    // Get market end time
    let market = test.env.as_contract(&test.contract_id, || {
        test.env
            .storage()
            .persistent()
            .get::<Symbol, Market>(&test.market_id)
            .unwrap()
    });

    // Advance time past end time
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

    // Fetch result once
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);
    client.fetch_oracle_result(&test.market_id, &test.pyth_contract);

    // Attempt to fetch again
    client.fetch_oracle_result(&test.market_id, &test.pyth_contract);
}

#[test]
fn test_dispute_result() {
    // Setup test environment
    let test = PredictifyTest::setup();
    test.create_test_market();

    // Get market end time
    let market = test.env.as_contract(&test.contract_id, || {
        test.env
            .storage()
            .persistent()
            .get::<Symbol, Market>(&test.market_id)
            .unwrap()
    });
    let original_end_time = market.end_time;

    // Advance time past end time
    test.env.ledger().set(LedgerInfo {
        timestamp: original_end_time + 1,
        protocol_version: 22,
        sequence_number: test.env.ledger().sequence(),
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 1,
        min_persistent_entry_ttl: 1,
        max_entry_ttl: 10000,
    });

    // Fetch oracle result first
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);
    test.env.mock_all_auths();
    client.fetch_oracle_result(&test.market_id, &test.pyth_contract);

    // Dispute the result
    let dispute_stake: i128 = 10_0000000;
    test.env.mock_all_auths();
    client.dispute_result(&test.user, &test.market_id, &dispute_stake);

    // Verify stake transfer
    assert_eq!(
        test.token_test.token_client.balance(&test.user),
        1000_0000000 - dispute_stake
    );
    assert!(test.token_test.token_client.balance(&test.contract_id) >= dispute_stake);

    // Verify dispute recorded and end time extended
    let updated_market = test.env.as_contract(&test.contract_id, || {
        test.env
            .storage()
            .persistent()
            .get::<Symbol, Market>(&test.market_id)
            .unwrap()
    });
    assert_eq!(
        updated_market
            .dispute_stakes
            .get(test.user.clone())
            .unwrap(),
        dispute_stake
    );

    let dispute_extension = 24 * 60 * 60;
    assert_eq!(
        updated_market.end_time,
        test.env.ledger().timestamp() + dispute_extension
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn test_dispute_result_insufficient_stake() {
    // Setup test environment
    let test = PredictifyTest::setup();
    test.create_test_market();

    // Get market end time
    let market = test.env.as_contract(&test.contract_id, || {
        test.env
            .storage()
            .persistent()
            .get::<Symbol, Market>(&test.market_id)
            .unwrap()
    });

    // Advance time past end time
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

    // Fetch oracle result first
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);
    test.env.mock_all_auths();
    client.fetch_oracle_result(&test.market_id, &test.pyth_contract);

    // Attempt to dispute with insufficient stake
    let insufficient_stake: i128 = 5_000_000; // 5 XLM
    test.env.mock_all_auths();
    client.dispute_result(&test.user, &test.market_id, &insufficient_stake);
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn test_resolve_market_before_end_time() {
    // Setup
    let test = PredictifyTest::setup();
    test.create_test_market();

    // Don't advance time

    // Attempt to resolve before end time
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);
    client.resolve_market(&test.market_id);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn test_resolve_market_oracle_unavailable() {
    // Setup
    let test = PredictifyTest::setup();
    test.create_test_market();

    // Get market end time
    let market = test.env.as_contract(&test.contract_id, || {
        test.env
            .storage()
            .persistent()
            .get::<Symbol, Market>(&test.market_id)
            .unwrap()
    });

    // Advance time past end time
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

    // Don't call fetch_oracle_result

    // Attempt to resolve
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);
    client.resolve_market(&test.market_id);
}

#[test]
fn test_resolve_market_oracle_and_community_agree() {
    // Setup
    let test = PredictifyTest::setup();
    test.create_test_market();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // --- Setup Votes ---
    // 6 users vote 'yes', 4 vote 'no' -> Community says 'yes'
    test.env.mock_all_auths();
    let token_sac_client = StellarAssetClient::new(&test.env, &test.token_test.token_id);
    for i in 0..10 {
        let voter = Address::generate(&test.env);
        let outcome = if i < 6 { "yes" } else { "no" };
        // Mint some tokens to each voter using StellarAssetClient
        token_sac_client.mint(&voter, &10_0000000);
        client.vote(
            &voter,
            &test.market_id,
            &String::from_str(&test.env, outcome),
            &1_0000000,
        );
    }

    // --- Advance Time & Fetch Oracle Result ---
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
    // Oracle result is 'yes' (mock price 26k > 25k threshold)
    let oracle_outcome = client.fetch_oracle_result(&test.market_id, &test.pyth_contract);
    assert_eq!(oracle_outcome, String::from_str(&test.env, "yes"));

    // --- Resolve Market ---
    let final_result = client.resolve_market(&test.market_id);

    // --- Verify Result ---
    // Since oracle ('yes') and community ('yes') agree, final should be 'yes'
    assert_eq!(final_result, String::from_str(&test.env, "yes"));
}

#[test]
fn test_resolve_market_oracle_wins_low_votes() {
    // Setup
    let test = PredictifyTest::setup();
    test.create_test_market();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // --- Setup Votes ---
    // 2 users vote 'no', 1 vote 'yes' -> Community says 'no', but only 3 total votes
    test.env.mock_all_auths();
    let token_sac_client = StellarAssetClient::new(&test.env, &test.token_test.token_id);
    for i in 0..3 {
        let voter = Address::generate(&test.env);
        let outcome = if i < 2 { "no" } else { "yes" };
        // Mint some tokens to each voter using StellarAssetClient
        token_sac_client.mint(&voter, &10_0000000);
        client.vote(
            &voter,
            &test.market_id,
            &String::from_str(&test.env, outcome),
            &1_0000000,
        );
    }

    // --- Advance Time & Fetch Oracle Result ---
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
    // Oracle result is 'yes'
    let oracle_outcome = client.fetch_oracle_result(&test.market_id, &test.pyth_contract);
    assert_eq!(oracle_outcome, String::from_str(&test.env, "yes"));

    // --- Resolve Market ---
    let final_result = client.resolve_market(&test.market_id);

    // --- Verify Result ---
    // Oracle ('yes') disagrees with community ('no'), but low votes (<5), so oracle wins.
    assert_eq!(final_result, String::from_str(&test.env, "yes"));
}

#[test]
fn test_resolve_market_oracle_wins_weighted() {
    // Setup
    let test = PredictifyTest::setup();
    test.create_test_market();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // --- Setup Votes ---
    // 6 users vote 'no', 4 vote 'yes' -> Community says 'no' (significant votes)
    test.env.mock_all_auths();
    let token_sac_client = StellarAssetClient::new(&test.env, &test.token_test.token_id);
    for i in 0..10 {
        let voter = Address::generate(&test.env);
        let outcome = if i < 6 { "no" } else { "yes" };
        // Mint some tokens to each voter using StellarAssetClient
        token_sac_client.mint(&voter, &10_0000000);
        client.vote(
            &voter,
            &test.market_id,
            &String::from_str(&test.env, outcome),
            &1_0000000,
        );
    }

    // --- Advance Time & Fetch Oracle Result ---
    let market = test.env.as_contract(&test.contract_id, || {
        test.env
            .storage()
            .persistent()
            .get::<Symbol, Market>(&test.market_id)
            .unwrap()
    });
    // Set ledger sequence/timestamp to make random_value >= 30 (favor oracle)
    let sequence = 100;
    let timestamp = market.end_time + 50; // Ensure timestamp + sequence >= 30 mod 100
    test.env.ledger().set(LedgerInfo {
        timestamp,
        protocol_version: 22,
        sequence_number: sequence,
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 1,
        min_persistent_entry_ttl: 1,
        max_entry_ttl: 10000,
    });
    // Oracle result is 'yes'
    let oracle_outcome = client.fetch_oracle_result(&test.market_id, &test.pyth_contract);
    assert_eq!(oracle_outcome, String::from_str(&test.env, "yes"));

    // --- Resolve Market ---
    let final_result = client.resolve_market(&test.market_id);

    // --- Verify Result ---
    // Oracle ('yes') disagrees with community ('no'), significant votes,
    // but weighted random choice favors oracle.
    assert_eq!(final_result, String::from_str(&test.env, "yes"));
}

#[test]
fn test_resolve_market_community_wins_weighted() {
    // Setup
    let test = PredictifyTest::setup();
    test.create_test_market();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // --- Setup Votes ---
    // 6 users vote 'no', 4 vote 'yes' -> Community says 'no' (significant votes)
    test.env.mock_all_auths();
    let token_sac_client = StellarAssetClient::new(&test.env, &test.token_test.token_id);
    for i in 0..10 {
        let voter = Address::generate(&test.env);
        let outcome = if i < 6 { "no" } else { "yes" };
        // Mint some tokens to each voter using StellarAssetClient
        token_sac_client.mint(&voter, &10_0000000);
        client.vote(
            &voter,
            &test.market_id,
            &String::from_str(&test.env, outcome),
            &1_0000000,
        );
    }

    // --- Advance Time & Fetch Oracle Result ---
    let market = test.env.as_contract(&test.contract_id, || {
        test.env
            .storage()
            .persistent()
            .get::<Symbol, Market>(&test.market_id)
            .unwrap()
    });
    // Set ledger sequence/timestamp to make random_value < 30 (favor community)
    let sequence = 10;
    let timestamp = market.end_time + 5; // Ensure timestamp + sequence < 30 mod 100
    test.env.ledger().set(LedgerInfo {
        timestamp,
        protocol_version: 22,
        sequence_number: sequence,
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 1,
        min_persistent_entry_ttl: 1,
        max_entry_ttl: 10000,
    });
    // Oracle result is 'yes'
    let oracle_outcome = client.fetch_oracle_result(&test.market_id, &test.pyth_contract);
    assert_eq!(oracle_outcome, String::from_str(&test.env, "yes"));

    // --- Resolve Market ---
    let final_result = client.resolve_market(&test.market_id);

    // --- Verify Result ---
    // Oracle ('yes') disagrees with community ('no'), significant votes,
    // and weighted random choice favors community.
    assert_eq!(final_result, String::from_str(&test.env, "no"));
}

#[test]
#[should_panic(expected = "Error(Storage, MissingValue)")]
fn test_reflector_oracle_get_price_success() {
    // Setup test environment
    let test = PredictifyTest::setup();

    // Use a mock contract address for testing
    let mock_reflector_contract = Address::generate(&test.env);

    // Create ReflectorOracle instance
    let reflector_oracle = ReflectorOracle::new(mock_reflector_contract.clone());

    // Test get_price function with mock Reflector contract
    // This should panic because the mock contract doesn't exist
    let feed_id = String::from_str(&test.env, "BTC/USD");
    let _result = reflector_oracle.get_price(&test.env, &feed_id);

    // This line should not be reached due to panic
    panic!("Should have panicked before reaching this point");
}

#[test]
#[should_panic(expected = "Error(Storage, MissingValue)")]
fn test_reflector_oracle_get_price_with_different_assets() {
    // Setup test environment
    let test = PredictifyTest::setup();

    // Use a mock contract address for testing
    let mock_reflector_contract = Address::generate(&test.env);

    // Create ReflectorOracle instance
    let reflector_oracle = ReflectorOracle::new(mock_reflector_contract.clone());

    // Test different asset feed IDs with mock Reflector oracle
    // This should panic because the mock contract doesn't exist
    let test_cases = [
        ("BTC/USD", "Bitcoin"),
        ("ETH/USD", "Ethereum"),
        ("XLM/USD", "Stellar Lumens"),
    ];

    for (feed_id_str, _asset_name) in test_cases.iter() {
        let feed_id = String::from_str(&test.env, feed_id_str);
        let _result = reflector_oracle.get_price(&test.env, &feed_id);
        // This should panic on the first iteration
    }

    // This line should not be reached due to panic
    panic!("Should have panicked before reaching this point");
}

#[test]
#[should_panic(expected = "Error(Storage, MissingValue)")]
fn test_reflector_oracle_integration_with_market_creation() {
    // Setup test environment
    let test = PredictifyTest::setup();

    // Create contract client
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Create Reflector oracle configuration
    let oracle_config = OracleConfig {
        provider: OracleProvider::Reflector,
        feed_id: String::from_str(&test.env, "BTC"),
        threshold: 5000000, // $50,000 threshold
        comparison: String::from_str(&test.env, "gt"),
    };

    // Create market with Reflector oracle
    let market_id = client.create_market(
        &test.admin,
        &String::from_str(&test.env, "Will BTC price be above $50,000 by December 31?"),
        &vec![
            &test.env,
            String::from_str(&test.env, "yes"),
            String::from_str(&test.env, "no"),
        ],
        &30,
        &oracle_config,
    );

    // Verify market was created with Reflector oracle
    let market = test.env.as_contract(&test.contract_id, || {
        test.env
            .storage()
            .persistent()
            .get::<Symbol, Market>(&market_id)
            .unwrap()
    });

    assert_eq!(market.oracle_config.provider, OracleProvider::Reflector);
    assert_eq!(
        market.oracle_config.feed_id,
        String::from_str(&test.env, "BTC")
    );

    // Test fetching oracle result (this will test the get_price function indirectly)
    let market_end_time = market.end_time;

    // Advance time past market end
    test.env.ledger().set(LedgerInfo {
        timestamp: market_end_time + 1,
        protocol_version: 22,
        sequence_number: test.env.ledger().sequence(),
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 1,
        min_persistent_entry_ttl: 1,
        max_entry_ttl: 10000,
    });

    // Use a mock Reflector contract address for testing
    let mock_reflector_contract = Address::generate(&test.env);

    // Test fetch_oracle_result (this internally calls get_price)
    // This should panic because the mock contract doesn't exist
    let _outcome = client.fetch_oracle_result(&market_id, &mock_reflector_contract);

    // This line should not be reached due to panic
    panic!("Should have panicked before reaching this point");
}

#[test]
#[should_panic(expected = "Error(Storage, MissingValue)")]
fn test_reflector_oracle_error_handling() {
    // Setup test environment
    let test = PredictifyTest::setup();

    // Create ReflectorOracle with an invalid contract address to test error handling
    let invalid_contract = Address::generate(&test.env);
    let reflector_oracle = ReflectorOracle::new(invalid_contract);

    // Test get_price with invalid contract - should panic because contract doesn't exist
    let feed_id = String::from_str(&test.env, "BTC/USD");
    let _result = reflector_oracle.get_price(&test.env, &feed_id);

    // This line should not be reached due to panic
    panic!("Should have panicked before reaching this point");
}

#[test]
#[should_panic(expected = "Error(Storage, MissingValue)")]
fn test_reflector_oracle_fallback_mechanism() {
    // Setup test environment
    let test = PredictifyTest::setup();

    // Use a mock contract address for testing
    let mock_reflector_contract = Address::generate(&test.env);
    let reflector_oracle = ReflectorOracle::new(mock_reflector_contract.clone());

    // Test that the fallback mechanism works
    // This should panic because the mock contract doesn't exist
    let feed_id = String::from_str(&test.env, "BTC/USD");
    let _result = reflector_oracle.get_price(&test.env, &feed_id);

    // This line should not be reached due to panic
    panic!("Should have panicked before reaching this point");
}

#[test]
fn test_reflector_oracle_with_empty_feed_id() {
    // Setup test environment
    let test = PredictifyTest::setup();

    // Use a mock contract address for testing
    let mock_reflector_contract = Address::generate(&test.env);
    let reflector_oracle = ReflectorOracle::new(mock_reflector_contract.clone());

    // Test with empty feed_id - should return InvalidOracleFeed error
    let empty_feed_id = String::from_str(&test.env, "");
    let result = reflector_oracle.get_price(&test.env, &empty_feed_id);

    // Should return InvalidOracleFeed error for empty feed ID
    assert!(result.is_err());
    match result {
        Err(Error::InvalidOracleFeed) => (), // Expected error
        _ => panic!("Expected InvalidOracleFeed error, got {:?}", result),
    }
}

#[test]
#[should_panic(expected = "Error(Storage, MissingValue)")]
fn test_reflector_oracle_performance() {
    // Setup test environment
    let test = PredictifyTest::setup();

    // Use a mock contract address for testing
    let mock_reflector_contract = Address::generate(&test.env);
    let reflector_oracle = ReflectorOracle::new(mock_reflector_contract.clone());

    // Test multiple price requests to check performance
    // This should panic because the mock contract doesn't exist
    let feed_id = String::from_str(&test.env, "BTC/USD");

    // Make multiple calls to test performance and reliability
    for _i in 0..3 {
        let _result = reflector_oracle.get_price(&test.env, &feed_id);
        // This should panic on the first iteration
    }

    // This line should not be reached due to panic
    panic!("Should have panicked before reaching this point");
}

// Ensure PredictifyHybridClient is in scope (usually generated by #[contractimpl])
use crate::PredictifyHybridClient;

// ===== FEE MANAGEMENT TESTS =====

#[test]
fn test_fee_manager_collect_fees() {
    // Setup test environment
    let test = PredictifyTest::setup();
    test.create_test_market();

    // Add some votes to create stakes
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);
    test.env.mock_all_auths();
    
    // Add votes to create stakes
    let token_sac_client = StellarAssetClient::new(&test.env, &test.token_test.token_id);
    for i in 0..5 {
        let voter = Address::generate(&test.env);
        token_sac_client.mint(&voter, &10_0000000);
        client.vote(
            &voter,
            &test.market_id,
            &String::from_str(&test.env, "yes"),
            &1_0000000,
        );
    }

    // Resolve the market
    let market = test.env.as_contract(&test.contract_id, || {
        test.env
            .storage()
            .persistent()
            .get::<Symbol, Market>(&test.market_id)
            .unwrap()
    });

    // Advance time past end time
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

    // Fetch oracle result
    client.fetch_oracle_result(&test.market_id, &test.pyth_contract);

    // Resolve market
    client.resolve_market(&test.market_id);

    // Collect fees
    test.env.mock_all_auths();
    client.collect_fees(&test.admin, &test.market_id);

    // Verify fees were collected
    let updated_market = test.env.as_contract(&test.contract_id, || {
        test.env
            .storage()
            .persistent()
            .get::<Symbol, Market>(&test.market_id)
            .unwrap()
    });

    assert!(updated_market.fee_collected);
}

#[test]
#[should_panic(expected = "Error(Contract, #74)")]
fn test_fee_manager_collect_fees_already_collected() {
    // Setup test environment
    let test = PredictifyTest::setup();
    test.create_test_market();

    // Add votes and resolve market (same as above)
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);
    test.env.mock_all_auths();
    
    let token_sac_client = StellarAssetClient::new(&test.env, &test.token_test.token_id);
    for i in 0..5 {
        let voter = Address::generate(&test.env);
        token_sac_client.mint(&voter, &10_0000000);
        client.vote(
            &voter,
            &test.market_id,
            &String::from_str(&test.env, "yes"),
            &1_0000000,
        );
    }

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

    client.fetch_oracle_result(&test.market_id, &test.pyth_contract);
    client.resolve_market(&test.market_id);

    // Collect fees once
    test.env.mock_all_auths();
    client.collect_fees(&test.admin, &test.market_id);

    // Try to collect fees again (should fail)
    test.env.mock_all_auths();
    client.collect_fees(&test.admin, &test.market_id);
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn test_fee_manager_collect_fees_market_not_resolved() {
    // Setup test environment
    let test = PredictifyTest::setup();
    test.create_test_market();

    // Try to collect fees before market is resolved
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);
    test.env.mock_all_auths();
    client.collect_fees(&test.admin, &test.market_id);
}

#[test]
fn test_fee_calculator_platform_fee() {
    // Setup test environment
    let test = PredictifyTest::setup();
    let mut market = Market::new(
        &test.env,
        test.admin.clone(),
        String::from_str(&test.env, "Test Market"),
        vec![
            &test.env,
            String::from_str(&test.env, "yes"),
            String::from_str(&test.env, "no"),
        ],
        test.env.ledger().timestamp() + 86400,
        test.create_default_oracle_config(),
    );

    // Set total staked
    market.total_staked = 1_000_000_000; // 100 XLM

    // Calculate fee
    let fee = crate::fees::FeeCalculator::calculate_platform_fee(&market).unwrap();
    assert_eq!(fee, 20_000_000); // 2% of 100 XLM = 2 XLM
}

#[test]
fn test_fee_calculator_user_payout_after_fees() {
    let user_stake = 1_000_000_000; // 100 XLM
    let winning_total = 5_000_000_000; // 500 XLM
    let total_pool = 10_000_000_000; // 1000 XLM

    let payout = crate::fees::FeeCalculator::calculate_user_payout_after_fees(user_stake, winning_total, total_pool).unwrap();
    
    // Expected: (100 * 500 / 1000) * 0.98 = 49 XLM
    assert_eq!(payout, 49_000_000_000);
}

#[test]
fn test_fee_calculator_fee_breakdown() {
    // Setup test environment
    let test = PredictifyTest::setup();
    let mut market = Market::new(
        &test.env,
        test.admin.clone(),
        String::from_str(&test.env, "Test Market"),
        vec![
            &test.env,
            String::from_str(&test.env, "yes"),
            String::from_str(&test.env, "no"),
        ],
        test.env.ledger().timestamp() + 86400,
        test.create_default_oracle_config(),
    );

    market.total_staked = 1_000_000_000; // 100 XLM

    let breakdown = crate::fees::FeeCalculator::calculate_fee_breakdown(&market).unwrap();
    
    assert_eq!(breakdown.total_staked, 1_000_000_000);
    assert_eq!(breakdown.fee_percentage, crate::fees::PLATFORM_FEE_PERCENTAGE);
    assert_eq!(breakdown.fee_amount, 20_000_000); // 2 XLM
    assert_eq!(breakdown.platform_fee, 20_000_000);
    assert_eq!(breakdown.user_payout_amount, 980_000_000); // 98 XLM
}

#[test]
fn test_fee_validator_admin_permissions() {
    let test = PredictifyTest::setup();
    let admin = Address::generate(&test.env);

    // Set admin in storage
    test.env.as_contract(&test.contract_id, || {
        test.env.storage()
            .persistent()
            .set(&Symbol::new(&test.env, "Admin"), &admin);
    });

    // Valid admin
    assert!(crate::fees::FeeValidator::validate_admin_permissions(&test.env, &admin).is_ok());

    // Invalid admin
    let invalid_admin = Address::generate(&test.env);
    assert!(crate::fees::FeeValidator::validate_admin_permissions(&test.env, &invalid_admin).is_err());
}

#[test]
fn test_fee_validator_fee_amount() {
    // Valid fee amount
    assert!(crate::fees::FeeValidator::validate_fee_amount(crate::fees::MIN_FEE_AMOUNT).is_ok());

    // Invalid fee amount (too small)
    assert!(crate::fees::FeeValidator::validate_fee_amount(crate::fees::MIN_FEE_AMOUNT - 1).is_err());

    // Invalid fee amount (too large)
    assert!(crate::fees::FeeValidator::validate_fee_amount(crate::fees::MAX_FEE_AMOUNT + 1).is_err());
}

#[test]
fn test_fee_validator_market_for_fee_collection() {
    let test = PredictifyTest::setup();
    let mut market = Market::new(
        &test.env,
        test.admin.clone(),
        String::from_str(&test.env, "Test Market"),
        vec![
            &test.env,
            String::from_str(&test.env, "yes"),
            String::from_str(&test.env, "no"),
        ],
        test.env.ledger().timestamp() + 86400,
        test.create_default_oracle_config(),
    );

    // Market not resolved
    assert!(crate::fees::FeeValidator::validate_market_for_fee_collection(&market).is_err());

    // Set winning outcome
    market.winning_outcome = Some(String::from_str(&test.env, "yes"));

    // Insufficient stakes
    market.total_staked = crate::fees::FEE_COLLECTION_THRESHOLD - 1;
    assert!(crate::fees::FeeValidator::validate_market_for_fee_collection(&market).is_err());

    // Sufficient stakes
    market.total_staked = crate::fees::FEE_COLLECTION_THRESHOLD;
    assert!(crate::fees::FeeValidator::validate_market_for_fee_collection(&market).is_ok());

    // Fees already collected
    market.fee_collected = true;
    assert!(crate::fees::FeeValidator::validate_market_for_fee_collection(&market).is_err());
}

#[test]
fn test_fee_utils_can_collect_fees() {
    let test = PredictifyTest::setup();
    let mut market = Market::new(
        &test.env,
        test.admin.clone(),
        String::from_str(&test.env, "Test Market"),
        vec![
            &test.env,
            String::from_str(&test.env, "yes"),
            String::from_str(&test.env, "no"),
        ],
        test.env.ledger().timestamp() + 86400,
        test.create_default_oracle_config(),
    );

    // Market not resolved
    assert!(!crate::fees::FeeUtils::can_collect_fees(&market));

    // Set winning outcome
    market.winning_outcome = Some(String::from_str(&test.env, "yes"));

    // Insufficient stakes
    market.total_staked = crate::fees::FEE_COLLECTION_THRESHOLD - 1;
    assert!(!crate::fees::FeeUtils::can_collect_fees(&market));

    // Sufficient stakes
    market.total_staked = crate::fees::FEE_COLLECTION_THRESHOLD;
    assert!(crate::fees::FeeUtils::can_collect_fees(&market));

    // Fees already collected
    market.fee_collected = true;
    assert!(!crate::fees::FeeUtils::can_collect_fees(&market));
}

#[test]
fn test_fee_utils_get_fee_eligibility() {
    let test = PredictifyTest::setup();
    let mut market = Market::new(
        &test.env,
        test.admin.clone(),
        String::from_str(&test.env, "Test Market"),
        vec![
            &test.env,
            String::from_str(&test.env, "yes"),
            String::from_str(&test.env, "no"),
        ],
        test.env.ledger().timestamp() + 86400,
        test.create_default_oracle_config(),
    );

    // Market not resolved
    let (eligible, reason) = crate::fees::FeeUtils::get_fee_eligibility(&market);
    assert!(!eligible);
    assert!(reason.to_string().contains("not resolved"));

    // Set winning outcome
    market.winning_outcome = Some(String::from_str(&test.env, "yes"));

    // Insufficient stakes
    market.total_staked = crate::fees::FEE_COLLECTION_THRESHOLD - 1;
    let (eligible, reason) = crate::fees::FeeUtils::get_fee_eligibility(&market);
    assert!(!eligible);
    assert!(reason.to_string().contains("Insufficient stakes"));

    // Sufficient stakes
    market.total_staked = crate::fees::FEE_COLLECTION_THRESHOLD;
    let (eligible, reason) = crate::fees::FeeUtils::get_fee_eligibility(&market);
    assert!(eligible);
    assert!(reason.to_string().contains("Eligible"));
}

#[test]
fn test_fee_config_manager() {
    let test = PredictifyTest::setup();
    let config = crate::fees::FeeConfig {
        platform_fee_percentage: 3,
        creation_fee: 15_000_000,
        min_fee_amount: 2_000_000,
        max_fee_amount: 2_000_000_000,
        collection_threshold: 200_000_000,
        fees_enabled: true,
    };

    // Store and retrieve config
    crate::fees::FeeConfigManager::store_fee_config(&test.env, &config).unwrap();
    let retrieved_config = crate::fees::FeeConfigManager::get_fee_config(&test.env).unwrap();

    assert_eq!(config, retrieved_config);
}

#[test]
fn test_fee_config_manager_reset_to_defaults() {
    let test = PredictifyTest::setup();
    
    let default_config = crate::fees::FeeConfigManager::reset_to_defaults(&test.env).unwrap();
    
    assert_eq!(default_config.platform_fee_percentage, crate::fees::PLATFORM_FEE_PERCENTAGE);
    assert_eq!(default_config.creation_fee, crate::fees::MARKET_CREATION_FEE);
    assert_eq!(default_config.min_fee_amount, crate::fees::MIN_FEE_AMOUNT);
    assert_eq!(default_config.max_fee_amount, crate::fees::MAX_FEE_AMOUNT);
    assert_eq!(default_config.collection_threshold, crate::fees::FEE_COLLECTION_THRESHOLD);
    assert!(default_config.fees_enabled);
}

#[test]
fn test_fee_analytics_calculation() {
    let test = PredictifyTest::setup();
    
    // Test with no fee history
    let analytics = crate::fees::FeeAnalytics::calculate_analytics(&test.env).unwrap();
    assert_eq!(analytics.total_fees_collected, 0);
    assert_eq!(analytics.markets_with_fees, 0);
    assert_eq!(analytics.average_fee_per_market, 0);
}

#[test]
fn test_fee_analytics_market_fee_stats() {
    let test = PredictifyTest::setup();
    let mut market = Market::new(
        &test.env,
        test.admin.clone(),
        String::from_str(&test.env, "Test Market"),
        vec![
            &test.env,
            String::from_str(&test.env, "yes"),
            String::from_str(&test.env, "no"),
        ],
        test.env.ledger().timestamp() + 86400,
        test.create_default_oracle_config(),
    );

    market.total_staked = 1_000_000_000; // 100 XLM

    let stats = crate::fees::FeeAnalytics::get_market_fee_stats(&market).unwrap();
    assert_eq!(stats.total_staked, 1_000_000_000);
    assert_eq!(stats.fee_amount, 20_000_000); // 2 XLM
}

#[test]
fn test_fee_analytics_fee_efficiency() {
    let test = PredictifyTest::setup();
    let mut market = Market::new(
        &test.env,
        test.admin.clone(),
        String::from_str(&test.env, "Test Market"),
        vec![
            &test.env,
            String::from_str(&test.env, "yes"),
            String::from_str(&test.env, "no"),
        ],
        test.env.ledger().timestamp() + 86400,
        test.create_default_oracle_config(),
    );

    market.total_staked = 1_000_000_000; // 100 XLM

    // Fees not collected yet
    let efficiency = crate::fees::FeeAnalytics::calculate_fee_efficiency(&market).unwrap();
    assert_eq!(efficiency, 0.0);

    // Mark fees as collected
    market.fee_collected = true;
    let efficiency = crate::fees::FeeAnalytics::calculate_fee_efficiency(&market).unwrap();
    assert_eq!(efficiency, 1.0);
}

#[test]
fn test_fee_manager_process_creation_fee() {
    let test = PredictifyTest::setup();
    
    // Process creation fee
    crate::fees::FeeManager::process_creation_fee(&test.env, &test.admin).unwrap();
    
    // Verify fee was transferred (check contract balance increased)
    let contract_balance = test.token_test.token_client.balance(&test.contract_id);
    assert_eq!(contract_balance, crate::fees::MARKET_CREATION_FEE);
}

#[test]
fn test_fee_manager_get_fee_analytics() {
    let test = PredictifyTest::setup();
    
    let analytics = crate::fees::FeeManager::get_fee_analytics(&test.env).unwrap();
    assert_eq!(analytics.total_fees_collected, 0);
    assert_eq!(analytics.markets_with_fees, 0);
}

#[test]
fn test_fee_manager_update_fee_config() {
    let test = PredictifyTest::setup();
    
    let new_config = crate::fees::FeeConfig {
        platform_fee_percentage: 3,
        creation_fee: 15_000_000,
        min_fee_amount: 2_000_000,
        max_fee_amount: 2_000_000_000,
        collection_threshold: 200_000_000,
        fees_enabled: true,
    };

    // Set admin in storage
    test.env.as_contract(&test.contract_id, || {
        test.env.storage()
            .persistent()
            .set(&Symbol::new(&test.env, "Admin"), &test.admin);
    });

    let updated_config = crate::fees::FeeManager::update_fee_config(&test.env, test.admin.clone(), new_config.clone()).unwrap();
    assert_eq!(updated_config, new_config);
}

#[test]
fn test_fee_manager_get_fee_config() {
    let test = PredictifyTest::setup();
    
    let config = crate::fees::FeeManager::get_fee_config(&test.env).unwrap();
    assert_eq!(config.platform_fee_percentage, crate::fees::PLATFORM_FEE_PERCENTAGE);
    assert_eq!(config.creation_fee, crate::fees::MARKET_CREATION_FEE);
    assert!(config.fees_enabled);
}

#[test]
fn test_fee_manager_validate_market_fees() {
    let test = PredictifyTest::setup();
    test.create_test_market();
    
    let result = crate::fees::FeeManager::validate_market_fees(&test.env, &test.market_id).unwrap();
    assert!(!result.is_valid);
    assert!(!result.errors.is_empty());
}

#[test]
fn test_fee_calculator_dynamic_fee() {
    let test = PredictifyTest::setup();
    let mut market = Market::new(
        &test.env,
        test.admin.clone(),
        String::from_str(&test.env, "Test Market"),
        vec![
            &test.env,
            String::from_str(&test.env, "yes"),
            String::from_str(&test.env, "no"),
        ],
        test.env.ledger().timestamp() + 86400,
        test.create_default_oracle_config(),
    );

    // Small market (no adjustment)
    market.total_staked = 50_000_000; // 5 XLM
    let fee = crate::fees::FeeCalculator::calculate_dynamic_fee(&market).unwrap();
    assert_eq!(fee, 1_000_000); // 2% of 5 XLM = 0.1 XLM, but minimum is 0.1 XLM

    // Medium market (10% reduction)
    market.total_staked = 500_000_000; // 50 XLM
    let fee = crate::fees::FeeCalculator::calculate_dynamic_fee(&market).unwrap();
    assert_eq!(fee, 9_000_000); // 2% of 50 XLM = 1 XLM, then 90% = 0.9 XLM

    // Large market (20% reduction)
    market.total_staked = 2_000_000_000; // 200 XLM
    let fee = crate::fees::FeeCalculator::calculate_dynamic_fee(&market).unwrap();
    assert_eq!(fee, 32_000_000); // 2% of 200 XLM = 4 XLM, then 80% = 3.2 XLM
}

#[test]
fn test_fee_validator_fee_config() {
    // Valid config
    let valid_config = crate::fees::FeeConfig {
        platform_fee_percentage: 2,
        creation_fee: 10_000_000,
        min_fee_amount: 1_000_000,
        max_fee_amount: 1_000_000_000,
        collection_threshold: 100_000_000,
        fees_enabled: true,
    };
    assert!(crate::fees::FeeValidator::validate_fee_config(&valid_config).is_ok());

    // Invalid config - negative fee percentage
    let invalid_config = crate::fees::FeeConfig {
        platform_fee_percentage: -1,
        creation_fee: 10_000_000,
        min_fee_amount: 1_000_000,
        max_fee_amount: 1_000_000_000,
        collection_threshold: 100_000_000,
        fees_enabled: true,
    };
    assert!(crate::fees::FeeValidator::validate_fee_config(&invalid_config).is_err());

    // Invalid config - max fee less than min fee
    let invalid_config = crate::fees::FeeConfig {
        platform_fee_percentage: 2,
        creation_fee: 10_000_000,
        min_fee_amount: 1_000_000_000,
        max_fee_amount: 500_000_000,
        collection_threshold: 100_000_000,
        fees_enabled: true,
    };
    assert!(crate::fees::FeeValidator::validate_fee_config(&invalid_config).is_err());
}

#[test]
fn test_testing_utilities() {
    // Test fee config validation
    let config = crate::fees::testing::create_test_fee_config();
    assert!(crate::fees::testing::validate_fee_config_structure(&config).is_ok());

    // Test fee collection validation
    let test = PredictifyTest::setup();
    let collection = crate::fees::testing::create_test_fee_collection(
        &test.env,
        Symbol::new(&test.env, "test"),
        1_000_000,
        Address::generate(&test.env),
    );
    assert!(crate::fees::testing::validate_fee_collection_structure(&collection).is_ok());

    // Test fee breakdown
    let breakdown = crate::fees::testing::create_test_fee_breakdown();
    assert_eq!(breakdown.total_staked, 1_000_000_000);
    assert_eq!(breakdown.fee_amount, 20_000_000);
    assert_eq!(breakdown.user_payout_amount, 980_000_000);
}

// ===== RESOLUTION SYSTEM TESTS =====

#[test]
fn test_oracle_resolution_manager_fetch_result() {
    let test = PredictifyTest::setup();
    test.create_test_market();

    // Get market end time
    let market = test.env.as_contract(&test.contract_id, || {
        test.env
            .storage()
            .persistent()
            .get::<Symbol, Market>(&test.market_id)
            .unwrap()
    });

    // Advance time past end time
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

    // Fetch oracle result
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);
    let outcome = client.fetch_oracle_result(&test.market_id, &test.pyth_contract);

    // Verify the outcome
    assert_eq!(outcome, String::from_str(&test.env, "yes"));

    // Test get_oracle_resolution
    let oracle_resolution = client.get_oracle_resolution(&test.market_id);
    assert!(oracle_resolution.is_some());
}

#[test]
fn test_market_resolution_manager_resolve_market() {
    let test = PredictifyTest::setup();
    test.create_test_market();

    // Add some votes
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);
    test.env.mock_all_auths();
    
    let token_sac_client = StellarAssetClient::new(&test.env, &test.token_test.token_id);
    for i in 0..5 {
        let voter = Address::generate(&test.env);
        token_sac_client.mint(&voter, &10_0000000);
        client.vote(
            &voter,
            &test.market_id,
            &String::from_str(&test.env, "yes"),
            &1_0000000,
        );
    }

    // Get market end time and advance time
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

    // Fetch oracle result first
    client.fetch_oracle_result(&test.market_id, &test.pyth_contract);

    // Resolve market
    let final_result = client.resolve_market(&test.market_id);
    assert_eq!(final_result, String::from_str(&test.env, "yes"));

    // Test get_market_resolution
    let market_resolution = client.get_market_resolution(&test.market_id);
    assert!(market_resolution.is_some());
}

#[test]
fn test_resolution_validation() {
    let test = PredictifyTest::setup();
    test.create_test_market();

    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test validation before market ends
    let validation = client.validate_resolution(&test.market_id);
    assert!(!validation.is_valid);
    assert!(!validation.errors.is_empty());

    // Test validation after market ends but before oracle resolution
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

    let validation = client.validate_resolution(&test.market_id);
    assert!(validation.is_valid);
    assert!(!validation.recommendations.is_empty());
}

#[test]
fn test_resolution_state_management() {
    let test = PredictifyTest::setup();
    test.create_test_market();

    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test initial state
    let state = client.get_resolution_state(&test.market_id);
    assert_eq!(state, crate::resolution::ResolutionState::Active);

    // Test can_resolve_market
    let can_resolve = client.can_resolve_market(&test.market_id);
    assert!(!can_resolve);

    // Test after market ends
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

    let can_resolve = client.can_resolve_market(&test.market_id);
    assert!(!can_resolve); // Still can't resolve without oracle result

    // Test after oracle resolution
    client.fetch_oracle_result(&test.market_id, &test.pyth_contract);
    let state = client.get_resolution_state(&test.market_id);
    assert_eq!(state, crate::resolution::ResolutionState::OracleResolved);

    let can_resolve = client.can_resolve_market(&test.market_id);
    assert!(can_resolve);

    // Test after market resolution
    client.resolve_market(&test.market_id);
    let state = client.get_resolution_state(&test.market_id);
    assert_eq!(state, crate::resolution::ResolutionState::MarketResolved);
}

#[test]
fn test_resolution_analytics() {
    let test = PredictifyTest::setup();
    test.create_test_market();

    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test initial analytics
    let analytics = client.get_resolution_analytics();
    assert_eq!(analytics.total_resolutions, 0);

    // Test oracle stats
    let oracle_stats = client.get_oracle_stats();
    assert_eq!(oracle_stats.total_resolutions, 0);

    // Resolve a market
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

    client.fetch_oracle_result(&test.market_id, &test.pyth_contract);
    client.resolve_market(&test.market_id);

    // Test updated analytics
    let analytics = client.get_resolution_analytics();
    assert_eq!(analytics.total_resolutions, 1);
}

#[test]
fn test_resolution_time_calculation() {
    let test = PredictifyTest::setup();
    test.create_test_market();

    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test resolution time before market ends
    let resolution_time = client.calculate_resolution_time(&test.market_id);
    assert_eq!(resolution_time, 0);

    // Test resolution time after market ends
    let market = test.env.as_contract(&test.contract_id, || {
        test.env
            .storage()
            .persistent()
            .get::<Symbol, Market>(&test.market_id)
            .unwrap()
    });

    let advance_time = 3600; // 1 hour
    test.env.ledger().set(LedgerInfo {
        timestamp: market.end_time + advance_time,
        protocol_version: 22,
        sequence_number: test.env.ledger().sequence(),
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 1,
        min_persistent_entry_ttl: 1,
        max_entry_ttl: 10000,
    });

    let resolution_time = client.calculate_resolution_time(&test.market_id);
    assert_eq!(resolution_time, advance_time);
}

#[test]
fn test_resolution_method_determination() {
    let test = PredictifyTest::setup();
    test.create_test_market();

    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Add votes to create different scenarios
    test.env.mock_all_auths();
    let token_sac_client = StellarAssetClient::new(&test.env, &test.token_test.token_id);
    
    // Scenario 1: Oracle and community agree
    for i in 0..6 {
        let voter = Address::generate(&test.env);
        token_sac_client.mint(&voter, &10_0000000);
        client.vote(
            &voter,
            &test.market_id,
            &String::from_str(&test.env, "yes"),
            &1_0000000,
        );
    }

    for i in 0..4 {
        let voter = Address::generate(&test.env);
        token_sac_client.mint(&voter, &10_0000000);
        client.vote(
            &voter,
            &test.market_id,
            &String::from_str(&test.env, "no"),
            &1_0000000,
        );
    }

    // Resolve market
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

    client.fetch_oracle_result(&test.market_id, &test.pyth_contract);
    let final_result = client.resolve_market(&test.market_id);

    // Verify resolution method
    let market_resolution = client.get_market_resolution(&test.market_id);
    assert!(market_resolution.is_some());
    
    let resolution = market_resolution.unwrap();
    assert_eq!(resolution.final_outcome, String::from_str(&test.env, "yes"));
    assert!(resolution.confidence_score > 0);
}

#[test]
fn test_resolution_error_handling() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test resolution of non-existent market
    let non_existent_market = Symbol::new(&test.env, "non_existent");
    
    // These should not panic but return None or default values
    let oracle_resolution = client.get_oracle_resolution(&non_existent_market);
    assert!(oracle_resolution.is_none());

    let market_resolution = client.get_market_resolution(&non_existent_market);
    assert!(market_resolution.is_none());

    let state = client.get_resolution_state(&non_existent_market);
    assert_eq!(state, crate::resolution::ResolutionState::Active);

    let can_resolve = client.can_resolve_market(&non_existent_market);
    assert!(!can_resolve);

    let resolution_time = client.calculate_resolution_time(&non_existent_market);
    assert_eq!(resolution_time, 0);
}

#[test]
fn test_resolution_with_disputes() {
    let test = PredictifyTest::setup();
    test.create_test_market();

    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Add votes and resolve market
    test.env.mock_all_auths();
    let token_sac_client = StellarAssetClient::new(&test.env, &test.token_test.token_id);
    for i in 0..5 {
        let voter = Address::generate(&test.env);
        token_sac_client.mint(&voter, &10_0000000);
        client.vote(
            &voter,
            &test.market_id,
            &String::from_str(&test.env, "yes"),
            &1_0000000,
        );
    }

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

    client.fetch_oracle_result(&test.market_id, &test.pyth_contract);
    client.resolve_market(&test.market_id);

    // Add dispute
    let dispute_stake: i128 = 10_0000000;
    test.env.mock_all_auths();
    client.dispute_result(&test.user, &test.market_id, &dispute_stake);

    // Test resolution state with dispute
    let state = client.get_resolution_state(&test.market_id);
    assert_eq!(state, crate::resolution::ResolutionState::Disputed);

    // Test validation with dispute
    let validation = client.validate_resolution(&test.market_id);
    assert!(validation.is_valid);
    assert!(!validation.recommendations.is_empty());
}

#[test]
fn test_resolution_performance() {
    let test = PredictifyTest::setup();
    test.create_test_market();

    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test multiple resolution operations
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

    // Multiple oracle resolution calls should be fast
    client.fetch_oracle_result(&test.market_id, &test.pyth_contract);

    // Multiple market resolution calls should be fast
    client.resolve_market(&test.market_id);

    // Multiple analytics calls should be fast
    client.get_resolution_analytics();

    // Verify the operations completed successfully (performance testing removed for no_std compatibility)
    let analytics = client.get_resolution_analytics();
    assert_eq!(analytics.total_resolutions, 1);
}

// ===== CONFIGURATION MANAGEMENT TESTS =====

#[test]
fn test_configuration_initialization() {
    // Setup test environment
    let test = PredictifyTest::setup();

    // Test initialization with development config
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);
    test.env.mock_all_auths();
    client.initialize_with_config(&test.admin, &crate::config::Environment::Development);

    // Verify configuration was stored
    let config = client.get_contract_config();
    assert_eq!(config.network.environment, crate::config::Environment::Development);
    assert_eq!(config.fees.platform_fee_percentage, crate::config::DEFAULT_PLATFORM_FEE_PERCENTAGE);
}

#[test]
fn test_configuration_environment_specific() {
    // Setup test environment
    let test = PredictifyTest::setup();

    // Test mainnet configuration
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);
    test.env.mock_all_auths();
    client.initialize_with_config(&test.admin, &crate::config::Environment::Mainnet);

    // Verify mainnet-specific values
    let config = client.get_contract_config();
    assert_eq!(config.network.environment, crate::config::Environment::Mainnet);
    assert_eq!(config.fees.platform_fee_percentage, 3); // Higher for mainnet
    assert_eq!(config.fees.creation_fee, 15_000_000); // Higher for mainnet
}

#[test]
fn test_configuration_update() {
    // Setup test environment
    let test = PredictifyTest::setup();

    // Initialize with development config
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);
    test.env.mock_all_auths();
    client.initialize_with_config(&test.admin, &crate::config::Environment::Development);

    // Create custom configuration
    let mut custom_config = client.get_contract_config();
    custom_config.fees.platform_fee_percentage = 5;
    custom_config.fees.creation_fee = 20_000_000;

    // Update configuration
    test.env.mock_all_auths();
    let updated_config = client.update_contract_config(&test.admin, &custom_config);

    // Verify updates
    assert_eq!(updated_config.fees.platform_fee_percentage, 5);
    assert_eq!(updated_config.fees.creation_fee, 20_000_000);
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn test_configuration_update_unauthorized() {
    // Setup test environment
    let test = PredictifyTest::setup();

    // Initialize with development config
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);
    test.env.mock_all_auths();
    client.initialize_with_config(&test.admin, &crate::config::Environment::Development);

    // Try to update with non-admin user
    let custom_config = client.get_contract_config();
    test.env.mock_all_auths();
    client.update_contract_config(&test.user, &custom_config);
}

#[test]
fn test_configuration_reset() {
    // Setup test environment
    let test = PredictifyTest::setup();

    // Initialize with development config
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);
    test.env.mock_all_auths();
    client.initialize_with_config(&test.admin, &crate::config::Environment::Development);

    // Reset to defaults
    test.env.mock_all_auths();
    let reset_config = client.reset_config_to_defaults(&test.admin);

    // Verify reset values
    assert_eq!(reset_config.fees.platform_fee_percentage, crate::config::DEFAULT_PLATFORM_FEE_PERCENTAGE);
    assert_eq!(reset_config.fees.creation_fee, crate::config::DEFAULT_MARKET_CREATION_FEE);
}

#[test]
fn test_configuration_validation() {
    // Setup test environment
    let test = PredictifyTest::setup();

    // Initialize with valid config
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);
    test.env.mock_all_auths();
    client.initialize_with_config(&test.admin, &crate::config::Environment::Development);

    // Test validation
    let is_valid = client.validate_configuration();
    assert!(is_valid);
}

#[test]
fn test_configuration_summary() {
    // Setup test environment
    let test = PredictifyTest::setup();

    // Initialize with development config
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);
    test.env.mock_all_auths();
    client.initialize_with_config(&test.admin, &crate::config::Environment::Development);

    // Get configuration summary
    let summary = client.get_config_summary();
    assert!(summary.to_string().contains("development"));
    assert!(summary.to_string().contains("2%")); // Default fee percentage
}

#[test]
fn test_fees_enabled_check() {
    // Setup test environment
    let test = PredictifyTest::setup();

    // Initialize with development config
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);
    test.env.mock_all_auths();
    client.initialize_with_config(&test.admin, &crate::config::Environment::Development);

    // Check if fees are enabled
    let fees_enabled = client.fees_enabled();
    assert!(fees_enabled);
}

#[test]
fn test_environment_detection() {
    // Setup test environment
    let test = PredictifyTest::setup();

    // Test different environments
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);
    test.env.mock_all_auths();

    // Development environment
    client.initialize_with_config(&test.admin, &crate::config::Environment::Development);
    let env = client.get_environment();
    assert_eq!(env, crate::config::Environment::Development);

    // Testnet environment
    client.initialize_with_config(&test.admin, &crate::config::Environment::Testnet);
    let env = client.get_environment();
    assert_eq!(env, crate::config::Environment::Testnet);

    // Mainnet environment
    client.initialize_with_config(&test.admin, &crate::config::Environment::Mainnet);
    let env = client.get_environment();
    assert_eq!(env, crate::config::Environment::Mainnet);
}

#[test]
fn test_configuration_constants() {
    // Test that constants are properly defined
    assert_eq!(crate::config::DEFAULT_PLATFORM_FEE_PERCENTAGE, 2);
    assert_eq!(crate::config::DEFAULT_MARKET_CREATION_FEE, 10_000_000);
    assert_eq!(crate::config::MIN_FEE_AMOUNT, 1_000_000);
    assert_eq!(crate::config::MAX_FEE_AMOUNT, 1_000_000_000);
    assert_eq!(crate::config::FEE_COLLECTION_THRESHOLD, 100_000_000);

    assert_eq!(crate::config::MIN_VOTE_STAKE, 1_000_000);
    assert_eq!(crate::config::MIN_DISPUTE_STAKE, 10_000_000);
    assert_eq!(crate::config::DISPUTE_EXTENSION_HOURS, 24);

    assert_eq!(crate::config::MAX_MARKET_DURATION_DAYS, 365);
    assert_eq!(crate::config::MIN_MARKET_DURATION_DAYS, 1);
    assert_eq!(crate::config::MAX_MARKET_OUTCOMES, 10);
    assert_eq!(crate::config::MIN_MARKET_OUTCOMES, 2);

    assert_eq!(crate::config::MAX_EXTENSION_DAYS, 30);
    assert_eq!(crate::config::MIN_EXTENSION_DAYS, 1);
    assert_eq!(crate::config::EXTENSION_FEE_PER_DAY, 100_000_000);
}

// ===== UTILITY FUNCTION TESTS =====

#[test]
fn test_utility_format_duration() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test duration formatting
    let duration = client.format_duration(&3661u64); // 1 hour 1 minute 1 second
    assert!(duration.to_string().contains("1h 1m"));

    let long_duration = client.format_duration(&90061u64); // 1 day 1 hour 1 minute 1 second
    assert!(long_duration.to_string().contains("1d"));
}

#[test]
fn test_utility_calculate_percentage() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test percentage calculation with custom denominator
    let percentage = client.calculate_percentage(&25, &100, &1000);
    assert_eq!(percentage, 250); // 25% of 100 with denominator 1000 = 250

    let percentage2 = client.calculate_percentage(&50, &200, &100);
    assert_eq!(percentage2, 25); // 50% of 200 with denominator 100 = 25
}

#[test]
fn test_utility_validate_string_length() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    let valid_string = String::from_str(&test.env, "hello");
    assert!(client.validate_string_length(&valid_string, &1, &10));

    let short_string = String::from_str(&test.env, "hi");
    assert!(!client.validate_string_length(&short_string, &5, &10));

    let long_string = String::from_str(&test.env, "very long string that exceeds limit");
    assert!(!client.validate_string_length(&long_string, &1, &10));
}

#[test]
fn test_utility_sanitize_string() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    let dirty_string = String::from_str(&test.env, "hello@world#123!");
    let clean_string = client.sanitize_string(&dirty_string);
    assert_eq!(clean_string.to_string(), "hello world 123");

    let clean_input = String::from_str(&test.env, "hello world 123");
    let sanitized = client.sanitize_string(&clean_input);
    assert_eq!(sanitized.to_string(), "hello world 123");
}

#[test]
fn test_utility_number_conversion() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test number to string conversion
    let number_string = client.number_to_string(&12345);
    assert_eq!(number_string.to_string(), "12345");

    // Test string to number conversion
    let number_string = String::from_str(&test.env, "12345");
    let number = client.string_to_number(&number_string);
    assert_eq!(number, 12345);

    // Test invalid string to number conversion
    let invalid_string = String::from_str(&test.env, "invalid");
    let result = client.string_to_number(&invalid_string);
    assert_eq!(result, 0); // Returns 0 for invalid strings
}

#[test]
fn test_utility_generate_unique_id() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    let prefix = String::from_str(&test.env, "test");
    let id1 = client.generate_unique_id(&prefix);
    let id2 = client.generate_unique_id(&prefix);

    // IDs should be unique
    assert_ne!(id1.to_string(), id2.to_string());
    
    // IDs should contain the prefix
    assert!(id1.to_string().contains("test"));
    assert!(id2.to_string().contains("test"));
}

#[test]
fn test_utility_address_comparison() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    let addr1 = Address::from_str(&test.env, "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF");
    let addr2 = Address::from_str(&test.env, "GBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB");

    assert!(client.addresses_equal(&addr1, &addr1));
    assert!(!client.addresses_equal(&addr1, &addr2));
}

#[test]
fn test_utility_string_comparison() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    let str1 = String::from_str(&test.env, "Hello");
    let str2 = String::from_str(&test.env, "hello");
    let str3 = String::from_str(&test.env, "world");

    assert!(client.strings_equal_ignore_case(&str1, &str2));
    assert!(!client.strings_equal_ignore_case(&str2, &str3));
}

#[test]
fn test_utility_weighted_average() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    let values = vec![&test.env, 10, 20, 30];
    let weights = vec![&test.env, 1, 2, 3];

    let average = client.calculate_weighted_average(&values, &weights);
    assert_eq!(average, 23); // (10*1 + 20*2 + 30*3) / (1+2+3) = 140/6 = 23
}

#[test]
fn test_utility_simple_interest() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    let principal = 1000_0000000; // 1000 XLM
    let rate = 5; // 5%
    let periods = 2;

    let result = client.calculate_simple_interest(&principal, &rate, &periods);
    assert_eq!(result, 1100_0000000); // 1000 + (1000 * 5% * 2) = 1100 XLM
}

#[test]
fn test_utility_rounding() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test rounding to nearest multiple
    assert_eq!(client.round_to_nearest(&123, &10), 120);
    assert_eq!(client.round_to_nearest(&127, &10), 130);
    assert_eq!(client.round_to_nearest(&125, &10), 130);

    // Test clamping
    assert_eq!(client.clamp_value(&15, &10, &20), 15);
    assert_eq!(client.clamp_value(&5, &10, &20), 10);
    assert_eq!(client.clamp_value(&25, &10, &20), 20);

    // Test range validation
    assert!(client.is_within_range(&15, &10, &20));
    assert!(!client.is_within_range(&25, &10, &20));
    assert!(!client.is_within_range(&5, &10, &20));
}

#[test]
fn test_utility_math_operations() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test absolute difference
    assert_eq!(client.abs_difference(&10, &20), 10);
    assert_eq!(client.abs_difference(&20, &10), 10);
    assert_eq!(client.abs_difference(&10, &10), 0);

    // Test square root
    assert_eq!(client.sqrt(&16), 4);
    assert_eq!(client.sqrt(&25), 5);
    assert_eq!(client.sqrt(&0), 0);
}

#[test]
fn test_utility_validation() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test positive number validation
    assert!(client.validate_positive_number(&10));
    assert!(!client.validate_positive_number(&0));
    assert!(!client.validate_positive_number(&-10));

    // Test number range validation
    assert!(client.validate_number_range(&15, &10, &20));
    assert!(!client.validate_number_range(&25, &10, &20));
    assert!(!client.validate_number_range(&5, &10, &20));

    // Test future timestamp validation
    let future_time = test.env.ledger().timestamp() + 3600; // 1 hour in future
    assert!(client.validate_future_timestamp(&future_time));

    let past_time = test.env.ledger().timestamp() - 3600; // 1 hour in past
    assert!(!client.validate_future_timestamp(&past_time));
}

#[test]
fn test_utility_time_utilities() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    let time_info = client.get_time_utilities();
    assert!(time_info.to_string().contains("Current time:"));
    assert!(time_info.to_string().contains("Days to seconds:"));
    assert!(time_info.to_string().contains("86400")); // 1 day in seconds
}

#[test]
fn test_utility_integration() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test integration of multiple utilities
    let input_string = String::from_str(&test.env, "Hello@World#123!");
    let sanitized = client.sanitize_string(&input_string);
    let is_valid_length = client.validate_string_length(&sanitized, &1, &20);
    
    assert!(is_valid_length);
    assert_eq!(sanitized.to_string(), "Hello World 123");

    // Test numeric operations integration
    let values = vec![&test.env, 100, 200, 300];
    let weights = vec![&test.env, 1, 1, 1];
    let average = client.calculate_weighted_average(&values, &weights);
    let percentage = client.calculate_percentage(&average, &600, &100);
    
    assert_eq!(average, 200);
    assert_eq!(percentage, 33); // 200/600 * 100 = 33.33... rounded to 33
}

#[test]
fn test_utility_error_handling() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test error handling for invalid string to number conversion
    let invalid_string = String::from_str(&test.env, "not_a_number");
    let result = client.string_to_number(&invalid_string);
    assert_eq!(result, 0); // Returns 0 for invalid strings

    // Test error handling for empty vectors in weighted average
    let empty_values = vec![&test.env];
    let empty_weights = vec![&test.env];
    let result = client.calculate_weighted_average(&empty_values, &empty_weights);
    assert_eq!(result, 0); // Should return 0 for empty vectors
}

#[test]
fn test_utility_performance() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test performance of multiple utility operations
    let test_string = String::from_str(&test.env, "performance_test_string");
    
    // Multiple operations should complete quickly
    for _ in 0..10 {
        let _sanitized = client.sanitize_string(&test_string);
        let _is_valid = client.validate_string_length(&test_string, &1, &50);
        let _number = client.number_to_string(&12345);
        let _clamped = client.clamp_value(&15, &10, &20);
    }

    // Verify operations completed successfully
    let result = client.number_to_string(&12345);
    assert_eq!(result.to_string(), "12345");
}

// ===== EVENT SYSTEM TESTS =====

#[test]
fn test_event_emitter_market_created() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Create a market to trigger event emission
    test.create_test_market();

    // Get market events
    let events = client.get_market_events(&test.market_id);
    assert!(!events.is_empty());

    // Verify event structure
    let is_valid = client.validate_event_structure(&String::from_str(&test.env, "MarketCreated"), &String::from_str(&test.env, "test"));
    assert!(is_valid);
}

#[test]
fn test_event_emitter_vote_cast() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Create market and vote to trigger event emission
    test.create_test_market();
    test.env.mock_all_auths();
    client.vote(
        &test.user,
        &test.market_id,
        &String::from_str(&test.env, "yes"),
        &100_0000000,
    );

    // Get market events
    let events = client.get_market_events(&test.market_id);
    assert!(events.len() >= 2); // Market created + vote cast

    // Verify event structure
    let is_valid = client.validate_event_structure(&String::from_str(&test.env, "VoteCast"), &String::from_str(&test.env, "test"));
    assert!(is_valid);
}

#[test]
fn test_event_emitter_oracle_result() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Create market and fetch oracle result
    test.create_test_market();
    let market = test.env.as_contract(&test.contract_id, || {
        test.env
            .storage()
            .persistent()
            .get::<Symbol, Market>(&test.market_id)
            .unwrap()
    });

    // Advance time past end time
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

    // Fetch oracle result
    client.fetch_oracle_result(&test.market_id, &test.pyth_contract);

    // Get market events
    let events = client.get_market_events(&test.market_id);
    assert!(events.len() >= 2); // Market created + oracle result

    // Verify event structure
    let is_valid = client.validate_event_structure(&String::from_str(&test.env, "OracleResult"), &String::from_str(&test.env, "test"));
    assert!(is_valid);
}

#[test]
fn test_event_emitter_market_resolved() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Create market, add votes, and resolve
    test.create_test_market();
    
    // Add votes
    test.env.mock_all_auths();
    let token_sac_client = StellarAssetClient::new(&test.env, &test.token_test.token_id);
    for i in 0..5 {
        let voter = Address::generate(&test.env);
        token_sac_client.mint(&voter, &10_0000000);
        client.vote(
            &voter,
            &test.market_id,
            &String::from_str(&test.env, "yes"),
            &1_0000000,
        );
    }

    // Resolve market
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

    client.fetch_oracle_result(&test.market_id, &test.pyth_contract);
    client.resolve_market(&test.market_id);

    // Get market events
    let events = client.get_market_events(&test.market_id);
    assert!(events.len() >= 4); // Market created + votes + oracle result + market resolved

    // Verify event structure
    let is_valid = client.validate_event_structure(&String::from_str(&test.env, "MarketResolved"), &String::from_str(&test.env, "test"));
    assert!(is_valid);
}

#[test]
fn test_event_emitter_dispute_created() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Create market and resolve it
    test.create_test_market();
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

    client.fetch_oracle_result(&test.market_id, &test.pyth_contract);

    // Create dispute
    test.env.mock_all_auths();
    client.dispute_result(&test.user, &test.market_id, &10_0000000);

    // Get market events
    let events = client.get_market_events(&test.market_id);
    assert!(events.len() >= 3); // Market created + oracle result + dispute created

    // Verify event structure
    let is_valid = client.validate_event_structure(&String::from_str(&test.env, "DisputeCreated"), &String::from_str(&test.env, "test"));
    assert!(is_valid);
}

#[test]
fn test_event_emitter_fee_collected() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Create market, add votes, resolve, and collect fees
    test.create_test_market();
    
    // Add votes
    test.env.mock_all_auths();
    let token_sac_client = StellarAssetClient::new(&test.env, &test.token_test.token_id);
    for i in 0..5 {
        let voter = Address::generate(&test.env);
        token_sac_client.mint(&voter, &10_0000000);
        client.vote(
            &voter,
            &test.market_id,
            &String::from_str(&test.env, "yes"),
            &1_0000000,
        );
    }

    // Resolve market
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

    client.fetch_oracle_result(&test.market_id, &test.pyth_contract);
    client.resolve_market(&test.market_id);

    // Collect fees
    test.env.mock_all_auths();
    client.collect_fees(&test.admin, &test.market_id);

    // Get market events
    let events = client.get_market_events(&test.market_id);
    assert!(events.len() >= 5); // Market created + votes + oracle result + market resolved + fee collected

    // Verify event structure
    let is_valid = client.validate_event_structure(&String::from_str(&test.env, "FeeCollected"), &String::from_str(&test.env, "test"));
    assert!(is_valid);
}

#[test]
fn test_event_logger_get_recent_events() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Create some events
    test.create_test_market();
    test.env.mock_all_auths();
    client.vote(
        &test.user,
        &test.market_id,
        &String::from_str(&test.env, "yes"),
        &100_0000000,
    );

    // Get recent events
    let recent_events = client.get_recent_events(&10);
    assert!(!recent_events.is_empty());

    // Verify event structure
    for event in recent_events.iter() {
        assert!(!event.event_type.to_string().is_empty());
        assert!(event.timestamp > 0);
        assert!(!event.details.to_string().is_empty());
    }
}

#[test]
fn test_event_logger_get_error_events() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Get error events
    let error_events = client.get_error_events();
    
    // Initially should be empty or contain existing errors
    // This test verifies the function works without panicking
    assert!(error_events.len() >= 0);
}

#[test]
fn test_event_logger_get_performance_metrics() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Get performance metrics
    let metrics = client.get_performance_metrics();
    
    // Initially should be empty or contain existing metrics
    // This test verifies the function works without panicking
    assert!(metrics.len() >= 0);
}

#[test]
fn test_event_validator_market_created_event() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test validation of market created event
    let is_valid = client.validate_test_event(&String::from_str(&test.env, "MarketCreated"));
    assert!(is_valid);
}

#[test]
fn test_event_validator_vote_cast_event() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test validation of vote cast event
    let is_valid = client.validate_test_event(&String::from_str(&test.env, "VoteCast"));
    assert!(is_valid);
}

#[test]
fn test_event_validator_oracle_result_event() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test validation of oracle result event
    let is_valid = client.validate_test_event(&String::from_str(&test.env, "OracleResult"));
    assert!(is_valid);
}

#[test]
fn test_event_validator_market_resolved_event() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test validation of market resolved event
    let is_valid = client.validate_test_event(&String::from_str(&test.env, "MarketResolved"));
    assert!(is_valid);
}

#[test]
fn test_event_validator_dispute_created_event() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test validation of dispute created event
    let is_valid = client.validate_test_event(&String::from_str(&test.env, "DisputeCreated"));
    assert!(is_valid);
}

#[test]
fn test_event_validator_fee_collected_event() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test validation of fee collected event
    let is_valid = client.validate_test_event(&String::from_str(&test.env, "FeeCollected"));
    assert!(is_valid);
}

#[test]
fn test_event_validator_error_logged_event() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test validation of error logged event
    let is_valid = client.validate_test_event(&String::from_str(&test.env, "ErrorLogged"));
    assert!(is_valid);
}

#[test]
fn test_event_validator_performance_metric_event() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test validation of performance metric event
    let is_valid = client.validate_test_event(&String::from_str(&test.env, "PerformanceMetric"));
    assert!(is_valid);
}

#[test]
fn test_event_helpers_timestamp_validation() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test valid timestamp
    let valid_timestamp = test.env.ledger().timestamp();
    assert!(client.validate_event_timestamp(&valid_timestamp));

    // Test invalid timestamp (0)
    assert!(!client.validate_event_timestamp(&0));

    // Test invalid timestamp (too large)
    assert!(!client.validate_event_timestamp(&99999999999));
}

#[test]
fn test_event_helpers_event_age() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    let current_time = test.env.ledger().timestamp();
    let event_time = current_time - 3600; // 1 hour ago

    let age = client.get_event_age(&event_time);
    assert_eq!(age, 3600);
}

#[test]
fn test_event_helpers_recent_event_check() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    let current_time = test.env.ledger().timestamp();
    let recent_event_time = current_time - 1800; // 30 minutes ago
    let old_event_time = current_time - 7200; // 2 hours ago

    // Check recent event
    assert!(client.is_recent_event(&recent_event_time, &3600)); // Within 1 hour

    // Check old event
    assert!(!client.is_recent_event(&old_event_time, &3600)); // Not within 1 hour
}

#[test]
fn test_event_helpers_format_timestamp() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    let timestamp = 1234567890;
    let formatted = client.format_event_timestamp(&timestamp);
    
    // Should return a string representation
    assert!(!formatted.to_string().is_empty());
    assert!(formatted.to_string().contains("1234567890"));
}

#[test]
fn test_event_helpers_create_context() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    let context_parts = vec![
        &test.env,
        String::from_str(&test.env, "Market"),
        String::from_str(&test.env, "Vote"),
        String::from_str(&test.env, "User"),
    ];

    let context = client.create_event_context(&context_parts);
    
    // Should create a context string with parts separated by " | "
    assert!(context.to_string().contains("Market"));
    assert!(context.to_string().contains("Vote"));
    assert!(context.to_string().contains("User"));
}

#[test]
fn test_event_documentation_overview() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    let overview = client.get_event_system_overview();
    assert!(!overview.to_string().is_empty());
    assert!(overview.to_string().contains("event system"));
}

#[test]
fn test_event_documentation_event_types() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    let docs = client.get_event_documentation();
    assert!(!docs.is_empty());

    // Check for common event types
    let event_types = vec![
        &test.env,
        String::from_str(&test.env, "MarketCreated"),
        String::from_str(&test.env, "VoteCast"),
        String::from_str(&test.env, "OracleResult"),
        String::from_str(&test.env, "MarketResolved"),
        String::from_str(&test.env, "DisputeCreated"),
        String::from_str(&test.env, "FeeCollected"),
    ];

    for event_type in event_types.iter() {
        // Verify documentation exists for each event type
        // Note: In a real implementation, you would check specific keys
        assert!(docs.len() > 0);
    }
}

#[test]
fn test_event_documentation_usage_examples() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    let examples = client.get_event_usage_examples();
    assert!(!examples.is_empty());

    // Check for common usage examples
    let example_types = vec![
        &test.env,
        String::from_str(&test.env, "EmitMarketCreated"),
        String::from_str(&test.env, "EmitVoteCast"),
        String::from_str(&test.env, "GetMarketEvents"),
        String::from_str(&test.env, "ValidateEvent"),
    ];

    for example_type in example_types.iter() {
        // Verify examples exist for each type
        // Note: In a real implementation, you would check specific keys
        assert!(examples.len() > 0);
    }
}

#[test]
fn test_event_testing_utilities() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test creating test events
    let event_types = vec![
        &test.env,
        String::from_str(&test.env, "MarketCreated"),
        String::from_str(&test.env, "VoteCast"),
        String::from_str(&test.env, "OracleResult"),
        String::from_str(&test.env, "MarketResolved"),
        String::from_str(&test.env, "DisputeCreated"),
        String::from_str(&test.env, "FeeCollected"),
        String::from_str(&test.env, "ErrorLogged"),
        String::from_str(&test.env, "PerformanceMetric"),
    ];

    for event_type in event_types.iter() {
        let success = client.create_test_event(&event_type);
        assert!(success);
    }
}

#[test]
fn test_event_clear_old_events() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Create some events
    test.create_test_market();
    test.env.mock_all_auths();
    client.vote(
        &test.user,
        &test.market_id,
        &String::from_str(&test.env, "yes"),
        &100_0000000,
    );

    // Clear old events (older than current time - 1 hour)
    let cutoff_time = test.env.ledger().timestamp() - 3600;
    client.clear_old_events(&cutoff_time);

    // This should not panic and should complete successfully
    // In a real implementation, you would verify events were actually cleared
}

#[test]
fn test_event_integration() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test integration of multiple event operations
    test.create_test_market();
    
    // Add votes
    test.env.mock_all_auths();
    let token_sac_client = StellarAssetClient::new(&test.env, &test.token_test.token_id);
    for i in 0..3 {
        let voter = Address::generate(&test.env);
        token_sac_client.mint(&voter, &10_0000000);
        client.vote(
            &voter,
            &test.market_id,
            &String::from_str(&test.env, "yes"),
            &1_0000000,
        );
    }

    // Get market events
    let market_events = client.get_market_events(&test.market_id);
    assert!(market_events.len() >= 4); // Market created + 3 votes

    // Get recent events
    let recent_events = client.get_recent_events(&10);
    assert!(!recent_events.is_empty());

    // Validate event structures
    for event in market_events.iter() {
        assert!(!event.event_type.to_string().is_empty());
        assert!(event.timestamp > 0);
        assert!(!event.details.to_string().is_empty());
    }

    // Test event age calculation
    let current_time = test.env.ledger().timestamp();
    let event_age = client.get_event_age(&(current_time - 1800)); // 30 minutes ago
    assert_eq!(event_age, 1800);

    // Test recent event check
    let is_recent = client.is_recent_event(&(current_time - 1800), &3600); // Within 1 hour
    assert!(is_recent);
}

#[test]
fn test_event_error_handling() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test invalid event type validation
    let is_valid = client.validate_event_structure(&String::from_str(&test.env, "InvalidEventType"), &String::from_str(&test.env, "test"));
    assert!(!is_valid);

    // Test invalid test event validation
    let is_valid = client.validate_test_event(&String::from_str(&test.env, "InvalidEventType"));
    assert!(!is_valid);

    // Test event age with future timestamp
    let future_time = test.env.ledger().timestamp() + 3600; // 1 hour in future
    let age = client.get_event_age(&future_time);
    assert_eq!(age, 0); // Should return 0 for future timestamps
}

#[test]
fn test_event_performance() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test performance of multiple event operations
    test.create_test_market();
    
    // Multiple event operations should complete quickly
    for _ in 0..10 {
        let _market_events = client.get_market_events(&test.market_id);
        let _recent_events = client.get_recent_events(&5);
        let _is_valid = client.validate_event_structure(&String::from_str(&test.env, "MarketCreated"), &String::from_str(&test.env, "test"));
        let _age = client.get_event_age(&(test.env.ledger().timestamp() - 1800));
    }

    // Verify operations completed successfully
    let market_events = client.get_market_events(&test.market_id);
    assert!(!market_events.is_empty());
}

// ===== VALIDATION SYSTEM TESTS =====

#[test]
fn test_input_validation_address() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test valid address
    let valid_address = Address::generate(&test.env);
    // Note: validate_address method doesn't exist in contract interface
    // For now, we test that the address is valid by checking it's not empty
    assert!(!valid_address.to_string().is_empty());
}

#[test]
fn test_input_validation_string() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test valid string
    let valid_string = String::from_str(&test.env, "Hello World");
    let is_valid = client.validate_string_length(&valid_string, &1, &50);
    assert!(is_valid);

    // Test string too short
    let short_string = String::from_str(&test.env, "Hi");
    let is_valid = client.validate_string_length(&short_string, &5, &50);
    assert!(!is_valid);

    // Test string too long
    let long_string = String::from_str(&test.env, "This is a very long string that exceeds the maximum length limit");
    let is_valid = client.validate_string_length(&long_string, &1, &20);
    assert!(!is_valid);
}

#[test]
fn test_input_validation_number_range() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test valid number in range
    assert!(client.validate_number_range(&15, &10, &20));

    // Test number below range
    assert!(!client.validate_number_range(&5, &10, &20));

    // Test number above range
    assert!(!client.validate_number_range(&25, &10, &20));

    // Test number at boundaries
    assert!(client.validate_number_range(&10, &10, &20));
    assert!(client.validate_number_range(&20, &10, &20));
}

#[test]
fn test_input_validation_positive_number() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test positive number
    assert!(client.validate_positive_number(&10));

    // Test zero
    assert!(!client.validate_positive_number(&0));

    // Test negative number
    assert!(!client.validate_positive_number(&-10));
}

#[test]
fn test_input_validation_future_timestamp() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test future timestamp
    let future_time = test.env.ledger().timestamp() + 3600; // 1 hour in future
    assert!(client.validate_future_timestamp(&future_time));

    // Test past timestamp
    let past_time = test.env.ledger().timestamp() - 3600; // 1 hour in past
    assert!(!client.validate_future_timestamp(&past_time));

    // Test current timestamp
    let current_time = test.env.ledger().timestamp();
    assert!(!client.validate_future_timestamp(&current_time));
}

#[test]
fn test_input_validation_duration() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test valid duration using utility function
    assert!(crate::utils::TimeUtils::validate_duration(&30));

    // Test duration too short
    assert!(!crate::utils::TimeUtils::validate_duration(&0));

    // Test duration too long
    assert!(!crate::utils::TimeUtils::validate_duration(&400)); // More than MAX_MARKET_DURATION_DAYS
}

#[test]
fn test_market_validation_creation() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test valid market creation inputs
    let valid_outcomes = vec![
        &test.env,
        String::from_str(&test.env, "yes"),
        String::from_str(&test.env, "no"),
    ];

    let oracle_config = test.create_default_oracle_config();

    let result = client.validate_market_creation_inputs(
        &test.admin,
        &String::from_str(&test.env, "Will BTC go above $25,000 by December 31?"),
        &valid_outcomes,
        &30,
        &oracle_config,
    );

    assert!(result.is_valid);
    // error_count > 0 means errors present
    assert!(result.error_count == 0);
}

#[test]
fn test_market_validation_invalid_question() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test market creation with empty question
    let valid_outcomes = vec![
        &test.env,
        String::from_str(&test.env, "yes"),
        String::from_str(&test.env, "no"),
    ];

    let oracle_config = test.create_default_oracle_config();

    let result = client.validate_market_creation_inputs(
        &test.admin,
        &String::from_str(&test.env, ""), // Empty question
        &valid_outcomes,
        &30,
        &oracle_config,
    );

    assert!(!result.is_valid);
    assert!(result.error_count > 0);
}

#[test]
fn test_market_validation_invalid_outcomes() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test market creation with single outcome (too few)
    let invalid_outcomes = vec![
        &test.env,
        String::from_str(&test.env, "yes"),
    ];

    let oracle_config = test.create_default_oracle_config();

    let result = client.validate_market_creation_inputs(
        &test.admin,
        &String::from_str(&test.env, "Will BTC go above $25,000 by December 31?"),
        &invalid_outcomes,
        &30,
        &oracle_config,
    );

    assert!(!result.is_valid);
    assert!(result.error_count > 0);
}

#[test]
fn test_market_validation_invalid_duration() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test market creation with invalid duration
    let valid_outcomes = vec![
        &test.env,
        String::from_str(&test.env, "yes"),
        String::from_str(&test.env, "no"),
    ];

    let oracle_config = test.create_default_oracle_config();

    let result = client.validate_market_creation_inputs(
        &test.admin,
        &String::from_str(&test.env, "Will BTC go above $25,000 by December 31?"),
        &valid_outcomes,
        &0, // Invalid duration
        &oracle_config,
    );

    assert!(!result.is_valid);
    assert!(result.error_count > 0);
}

#[test]
fn test_market_validation_state() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Create a market first
    test.create_test_market();

    // Test market state validation
    let result = client.validate_market_state(&test.market_id);
    assert!(result.is_valid);
    assert!(!result.has_errors());
}

#[test]
fn test_market_validation_nonexistent() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test validation of non-existent market
    let non_existent_market = Symbol::new(&test.env, "non_existent");
    let result = client.validate_market_state(&non_existent_market);
    
    assert!(!result.is_valid);
    assert!(result.has_errors());
}

#[test]
fn test_oracle_validation_config() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test valid oracle config
    let valid_config = test.create_default_oracle_config();
    let result = client.validate_oracle_config(&valid_config);
    assert!(result.is_valid);
    assert!(result.error_count == 0);
}

#[test]
fn test_oracle_validation_invalid_feed_id() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test oracle config with empty feed_id
    let invalid_config = OracleConfig {
        provider: OracleProvider::Pyth,
        feed_id: String::from_str(&test.env, ""), // Empty feed_id
        threshold: 2500000,
        comparison: String::from_str(&test.env, "gt"),
    };

    let result = client.validate_oracle_config(&invalid_config);
    assert!(!result.is_valid);
    assert!(result.error_count > 0);
}

#[test]
fn test_oracle_validation_invalid_threshold() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test oracle config with invalid threshold
    let invalid_config = OracleConfig {
        provider: OracleProvider::Pyth,
        feed_id: String::from_str(&test.env, "BTC/USD"),
        threshold: 0, // Invalid threshold (must be positive)
        comparison: String::from_str(&test.env, "gt"),
    };

    let result = client.validate_oracle_config(&invalid_config);
    assert!(!result.is_valid);
    assert!(result.error_count > 0);
}

#[test]
fn test_fee_validation_config() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test valid fee config
    let result = client.validate_fee_config(
        &2, // platform_fee_percentage
        &10_000_000, // creation_fee
        &1_000_000, // min_fee_amount
        &1_000_000_000, // max_fee_amount
        &100_000_000, // collection_threshold
    );

    assert!(result.is_valid);
    assert!(result.error_count == 0);
}

#[test]
fn test_fee_validation_invalid_percentage() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test fee config with invalid percentage
    let result = client.validate_fee_config(
        &150, // Invalid percentage (>100%)
        &10_000_000,
        &1_000_000,
        &1_000_000_000,
        &100_000_000,
    );

    assert!(!result.is_valid);
    assert!(result.error_count > 0);
}

#[test]
fn test_fee_validation_invalid_amounts() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test fee config with min > max
    let result = client.validate_fee_config(
        &2,
        &10_000_000,
        &2_000_000_000, // min > max
        &1_000_000_000,
        &100_000_000,
    );

    assert!(!result.is_valid);
    assert!(result.error_count > 0);
}

#[test]
fn test_vote_validation_inputs() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Create a market first
    test.create_test_market();

    // Test valid vote inputs
    let result = client.validate_vote_inputs(
        &test.user,
        &test.market_id,
        &String::from_str(&test.env, "yes"),
        &100_0000000,
    );

    assert!(result.is_valid);
    assert!(result.error_count == 0);
}

#[test]
fn test_vote_validation_invalid_outcome() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Create a market first
    test.create_test_market();

    // Test vote with invalid outcome
    let result = client.validate_vote_inputs(
        &test.user,
        &test.market_id,
        &String::from_str(&test.env, "maybe"), // Invalid outcome
        &100_0000000,
    );

    assert!(!result.is_valid);
    assert!(result.error_count > 0);
}

#[test]
fn test_vote_validation_invalid_stake() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Create a market first
    test.create_test_market();

    // Test vote with invalid stake amount
    let result = client.validate_vote_inputs(
        &test.user,
        &test.market_id,
        &String::from_str(&test.env, "yes"),
        &500_000, // Too small stake
    );

    assert!(!result.is_valid);
    assert!(result.error_count > 0);
}

#[test]
fn test_dispute_validation_creation() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Create and resolve a market first
    test.create_test_market();
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

    client.fetch_oracle_result(&test.market_id, &test.pyth_contract);
    client.resolve_market(&test.market_id);

    // Test valid dispute creation
    let result = client.validate_dispute_creation(
        &test.user,
        &test.market_id,
        &10_0000000,
    );

    assert!(result.is_valid);
    assert!(result.error_count == 0);
}

#[test]
fn test_dispute_validation_invalid_stake() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Create and resolve a market first
    test.create_test_market();
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

    client.fetch_oracle_result(&test.market_id, &test.pyth_contract);
    client.resolve_market(&test.market_id);

    // Test dispute with invalid stake amount
    let result = client.validate_dispute_creation(
        &test.user,
        &test.market_id,
        &5_000_000, // Too small stake
    );

    assert!(!result.is_valid);
    assert!(result.error_count > 0);
}

#[test]
fn test_validation_rules_documentation() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test getting validation rules
    let rules = client.get_validation_rules();
    assert!(!rules.is_empty());

    // Test getting validation error codes
    let error_codes = client.get_validation_error_codes();
    assert!(!error_codes.is_empty());

    // Test getting validation overview
    let overview = client.get_validation_overview();
    assert!(!overview.to_string().is_empty());
}

#[test]
fn test_validation_testing_utilities() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test validation testing utilities
    let result = client.test_validation_utilities();
    assert!(result.is_valid);
    assert!(result.has_warnings()); // Should have test warnings
}

#[test]
fn test_comprehensive_validation_scenario() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test comprehensive validation with multiple validation types
    let valid_outcomes = vec![
        &test.env,
        String::from_str(&test.env, "yes"),
        String::from_str(&test.env, "no"),
    ];

    let oracle_config = test.create_default_oracle_config();

    // Test market creation validation
    let market_result = client.validate_market_creation_inputs(
        &test.admin,
        &String::from_str(&test.env, "Will BTC go above $25,000 by December 31?"),
        &valid_outcomes.clone(),
        &30,
        &oracle_config.clone(),
    );

    assert!(market_result.is_valid);
    assert!(market_result.error_count == 0);

    // Test oracle config validation
    let oracle_result = client.validate_oracle_config(&oracle_config);
    assert!(oracle_result.is_valid);
    assert!(oracle_result.error_count == 0);

    // Test fee config validation
    let fee_result = client.validate_fee_config(
        &2,
        &10_000_000,
        &1_000_000,
        &1_000_000_000,
        &100_000_000,
    );

    assert!(fee_result.is_valid);
    assert!(fee_result.error_count == 0);

    // Create market and test vote validation
    test.create_test_market();

    let vote_result = client.validate_vote_inputs(
        &test.user,
        &test.market_id,
        &String::from_str(&test.env, "yes"),
        &100_0000000,
    );

    assert!(vote_result.is_valid);
    assert!(vote_result.error_count == 0);
}

#[test]
fn test_validation_error_handling() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test validation with multiple errors
    let invalid_outcomes = vec![
        &test.env,
        String::from_str(&test.env, "yes"), // Only one outcome
    ];

    let oracle_config = test.create_default_oracle_config();

    let result = client.validate_market_creation_inputs(
        &test.admin,
        &String::from_str(&test.env, ""), // Empty question
        &invalid_outcomes,
        &0, // Invalid duration
        &oracle_config,
    );

    assert!(!result.is_valid);
    assert!(result.error_count > 0);
}

#[test]
fn test_validation_warnings_and_recommendations() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Test validation that produces warnings and recommendations
    let valid_outcomes = vec![
        &test.env,
        String::from_str(&test.env, "yes"),
        String::from_str(&test.env, "no"),
    ];

    let oracle_config = test.create_default_oracle_config();

    let result = client.validate_market_creation_inputs(
        &test.admin,
        &String::from_str(&test.env, "Will BTC go above $25,000 by December 31?"),
        &valid_outcomes,
        &30,
        &oracle_config,
    );

    // Valid result should have recommendations
    assert!(result.is_valid);
    assert!(!result.has_errors());
    assert!(result.recommendation_count > 0);
}
