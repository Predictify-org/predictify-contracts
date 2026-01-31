#![cfg(test)]

use crate::errors::Error;
use crate::types::{MarketState, OracleConfig, OracleProvider};
use crate::{PredictifyHybrid, PredictifyHybridClient};
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{vec, Address, Env, String, Symbol, Vec};

// Test helper structure
struct TestSetup {
    env: Env,
    contract_id: Address,
    admin: Address,
}

impl TestSetup {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        // Set a non-zero timestamp to avoid overflow in tests
        env.ledger().with_mut(|li| {
            li.timestamp = 10000;
        });

        let admin = Address::generate(&env);
        let contract_id = env.register(PredictifyHybrid, ());

        // Initialize the contract
        let client = PredictifyHybridClient::new(&env, &contract_id);
        client.initialize(&admin, &None);

        Self {
            env,
            contract_id,
            admin,
        }
    }
}

#[test]
fn test_create_event_success() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);

    let description = String::from_str(&setup.env, "Will prediction markets be the future?");
    let outcomes = vec![
        &setup.env,
        String::from_str(&setup.env, "Yes"),
        String::from_str(&setup.env, "No"),
    ];
    let end_time = setup.env.ledger().timestamp() + 3600; // 1 hour from now
    let oracle_config = OracleConfig {
        provider: OracleProvider::Reflector,
        oracle_address: Address::generate(&setup.env),
        feed_id: String::from_str(&setup.env, "BTC/USD"),
        threshold: 50000,
        comparison: String::from_str(&setup.env, "gt"),
    };

    let event_id = client.create_event(
        &setup.admin,
        &description,
        &outcomes,
        &end_time,
        &oracle_config,
        &None,
        &0,
    );

    // Verify event details using the new get_event method
    let event = client.get_event(&event_id).unwrap();
    assert_eq!(event.description, description);
    assert_eq!(event.end_time, end_time);
    assert_eq!(event.outcomes.len(), outcomes.len());
}

#[test]
fn test_create_market_success() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);

    let description = String::from_str(&setup.env, "Will this market be created?");
    let outcomes = vec![
        &setup.env,
        String::from_str(&setup.env, "Yes"),
        String::from_str(&setup.env, "No"),
    ];
    let duration_days = 30;
    let oracle_config = OracleConfig {
        provider: OracleProvider::Reflector,
        oracle_address: Address::generate(&setup.env),
        feed_id: String::from_str(&setup.env, "BTC/USD"),
        threshold: 50000,
        comparison: String::from_str(&setup.env, "gt"),
    };

    let market_id = client.create_market(
        &setup.admin,
        &description,
        &outcomes,
        &duration_days,
        &oracle_config,
        &None,
        &0,
    );

    assert!(client.get_market(&market_id).is_some());
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #100)")] // Error::Unauthorized = 100
fn test_create_event_unauthorized() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);

    let non_admin = Address::generate(&setup.env);
    let description = String::from_str(&setup.env, "Test event?");
    let outcomes = vec![
        &setup.env,
        String::from_str(&setup.env, "Yes"),
        String::from_str(&setup.env, "No"),
    ];
    let end_time = setup.env.ledger().timestamp() + 3600;
    let oracle_config = OracleConfig {
        provider: OracleProvider::Reflector,
        oracle_address: Address::generate(&setup.env),
        feed_id: String::from_str(&setup.env, "BTC/USD"),
        threshold: 50000,
        comparison: String::from_str(&setup.env, "gt"),
    };

    client.create_event(
        &non_admin,
        &description,
        &outcomes,
        &end_time,
        &oracle_config,
        &None,
        &0,
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #302)")] // Error::InvalidDuration = 302
fn test_create_event_invalid_end_time() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);

    let description = String::from_str(&setup.env, "Test event?");
    let outcomes = vec![
        &setup.env,
        String::from_str(&setup.env, "Yes"),
        String::from_str(&setup.env, "No"),
    ];
    let end_time = setup.env.ledger().timestamp() - 3600; // Past time
    let oracle_config = OracleConfig {
        provider: OracleProvider::Reflector,
        oracle_address: Address::generate(&setup.env),
        feed_id: String::from_str(&setup.env, "BTC/USD"),
        threshold: 50000,
        comparison: String::from_str(&setup.env, "gt"),
    };

    client.create_event(
        &setup.admin,
        &description,
        &outcomes,
        &end_time,
        &oracle_config,
        &None,
        &0,
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #301)")] // Error::InvalidDuration = 302
fn test_create_event_empty_outcomes() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);

    let description = String::from_str(&setup.env, "Test event?");
    let outcomes = Vec::new(&setup.env);
    let end_time = setup.env.ledger().timestamp() - 3600; // Past time
    let oracle_config = OracleConfig {
        provider: OracleProvider::Reflector,
        oracle_address: Address::generate(&setup.env),
        feed_id: String::from_str(&setup.env, "BTC/USD"),
        threshold: 50000,
        comparison: String::from_str(&setup.env, "gt"),
    };

    client.create_event(
        &setup.admin,
        &description,
        &outcomes,
        &end_time,
        &oracle_config,
        &None,
        &0,
    );
}
