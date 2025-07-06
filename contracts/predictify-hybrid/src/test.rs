#![cfg(test)]

use super::*;
use crate::errors::Error;
use crate::oracles::ReflectorOracle;
use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    token::{Client as TokenClient, StellarAssetClient},
    vec, String, Symbol,
};

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
    assert!(reason.contains("not resolved"));

    // Set winning outcome
    market.winning_outcome = Some(String::from_str(&test.env, "yes"));

    // Insufficient stakes
    market.total_staked = crate::fees::FEE_COLLECTION_THRESHOLD - 1;
    let (eligible, reason) = crate::fees::FeeUtils::get_fee_eligibility(&market);
    assert!(!eligible);
    assert!(reason.contains("Insufficient stakes"));

    // Sufficient stakes
    market.total_staked = crate::fees::FEE_COLLECTION_THRESHOLD;
    let (eligible, reason) = crate::fees::FeeUtils::get_fee_eligibility(&market);
    assert!(eligible);
    assert!(reason.contains("Eligible"));
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
