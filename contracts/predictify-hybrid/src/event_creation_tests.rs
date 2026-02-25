#![cfg(test)]

use crate::errors::Error;
use crate::types::{EventVisibility, MarketState, OracleConfig, OracleProvider};
use crate::{PredictifyHybrid, PredictifyHybridClient};
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{symbol_short, token::StellarAssetClient, vec, Address, Env, String, Symbol, Vec};

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
        let token_admin = Address::generate(&env);
        let token_contract = env.register_stellar_asset_contract_v2(token_admin);
        let token_address = token_contract.address();

        // Initialize the contract
        let client = PredictifyHybridClient::new(&env, &contract_id);
        client.initialize(&admin, &None);

        // Configure token used for creation fee collection and fund admin balance.
        env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .set(&Symbol::new(&env, "TokenID"), &token_address);
        });
        let token_client = StellarAssetClient::new(&env, &token_address);
        env.mock_all_auths();
        token_client.mint(&admin, &1_000_0000000);

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
        &EventVisibility::Public,
    );

    // Verify event details using the new get_event method
    let event = client.get_event(&event_id).unwrap();
    assert_eq!(event.description, description);
    assert_eq!(event.end_time, end_time);
    assert_eq!(event.outcomes.len(), outcomes.len());

    // Verify that a creation fee was recorded
    setup.env.as_contract(&setup.contract_id, || {
        let key = symbol_short!("creat_fee");
        let total: i128 = setup
            .env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(0);
        assert_eq!(total, crate::fees::MARKET_CREATION_FEE);
    });
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #400)")] // Error::InvalidState = 400
fn test_create_event_without_token_configuration_fails() {
    let env = Env::default();
    env.mock_all_auths();

    env.ledger().with_mut(|li| {
        li.timestamp = 10000;
    });

    let admin = Address::generate(&env);
    let contract_id = env.register(PredictifyHybrid, ());
    let client = PredictifyHybridClient::new(&env, &contract_id);
    client.initialize(&admin, &None);

    // Intentionally do NOT configure TokenID so creation fee processing fails
    let description = String::from_str(&env, "Fee test event");
    let outcomes = vec![
        &env,
        String::from_str(&env, "Yes"),
        String::from_str(&env, "No"),
    ];
    let end_time = env.ledger().timestamp() + 3600;
    let oracle_config = OracleConfig {
        provider: OracleProvider::Reflector,
        oracle_address: Address::generate(&env),
        feed_id: String::from_str(&env, "BTC/USD"),
        threshold: 50000,
        comparison: String::from_str(&env, "gt"),
    };

    client.create_event(
        &admin,
        &description,
        &outcomes,
        &end_time,
        &oracle_config,
        &None,
        &0,
        &EventVisibility::Public,
    );
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
        &None,
        &None,
        &None,
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
        &EventVisibility::Public,
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
        &EventVisibility::Public,
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
        &EventVisibility::Public,
    );
}

#[test]

#[should_panic(expected = "HostError: Error(Contract, #401)")] // Error::InvalidInput = 401
fn test_create_event_limit_enforced() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);

    let description = String::from_str(&setup.env, "Test event");
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

    // The default limit is 20. Creating 21 events should panic on the 21st.
    for _ in 0..21 {
        client.create_market(
            &setup.admin,
            &description,
            &outcomes,
            &1, // duration_days
            &oracle_config,
            &None,
            &0,
            &None,
            &None,
            &None,
        );
    }
}

#[test]
fn test_decrement_on_cancel_frees_slot() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);

    let description = String::from_str(&setup.env, "Test event");
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

    // Create 20 events, reaching the limit.
    let mut last_event_id: Option<Symbol> = None;
    for _ in 0..20 {
        last_event_id = Some(client.create_market(
            &setup.admin,
            &description,
            &outcomes,
            &1, // duration_days
            &oracle_config,
            &None,
            &0,
            &None,
            &None,
            &None,
        ));
    }

    // Cancel the last event created
    setup.env.mock_all_auths();
    if let Some(event_id) = last_event_id {
        client.cancel_event(
            &setup.admin,
            &event_id,
            &Some(String::from_str(&setup.env, "Cancellation")),
        );
    }

    // Creating another event should now succeed, as one slot was freed.
    client.create_market(
        &setup.admin,
        &description,
        &outcomes,
        &1, // duration_days
        &oracle_config,
        &None,
        &0,
        &None,
        &None,
        &None,
    );
}

#[test]
fn test_event_id_unique() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);

    let description = String::from_str(&setup.env, "Will this be a unique event A?");
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

    let event_id_1 = client.create_event(
        &setup.admin,
        &description,
        &outcomes,
        &end_time,
        &oracle_config,
        &None,
        &0,
        &EventVisibility::Public,
    );
    let desc_b = String::from_str(&setup.env, "Will this be a unique event B?");
    let event_id_2 = client.create_event(
        &setup.admin,
        &desc_b,
        &outcomes,
        &end_time,
        &oracle_config,
        &None,
        &0,
        &EventVisibility::Public,
    );

    assert_ne!(event_id_1, event_id_2, "Event IDs must be unique");
}

#[test]
fn test_event_storage_consistency() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);

    let description = String::from_str(&setup.env, "Stored event?");
    let outcomes = vec![
        &setup.env,
        String::from_str(&setup.env, "Yes"),
        String::from_str(&setup.env, "No"),
    ];
    let end_time = setup.env.ledger().timestamp() + 7200;
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
        &EventVisibility::Public,
    );

    let stored = client.get_event(&event_id).unwrap();
    assert_eq!(stored.description, description);
    assert_eq!(stored.end_time, end_time);
    assert_eq!(stored.outcomes.len(), outcomes.len());
    assert_eq!(stored.id, event_id);
}
