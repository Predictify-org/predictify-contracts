#![cfg(test)]

use crate::errors::Error;
use crate::types::{EventVisibility, MarketState, OracleConfig, OracleProvider};
use crate::{PredictifyHybrid, PredictifyHybridClient};
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{
    symbol_short, token::StellarAssetClient, vec, Address, Env, String, Symbol, Vec,
};

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
        provider: OracleProvider::reflector(),
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

/// Test that create_event validates minimum outcomes (parity with create_market)
#[test]
#[should_panic(expected = "Error(Contract, #301)")]
fn test_create_event_invalid_outcomes_too_few() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);

    let description = String::from_str(&setup.env, "Single outcome event?");
    let outcomes = vec![&setup.env, String::from_str(&setup.env, "Yes")]; // Only 1 outcome - invalid
    let end_time = setup.env.ledger().timestamp() + 3600;
    let oracle_config = OracleConfig {
        provider: OracleProvider::reflector(),
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

/// Test that create_event validates empty description (parity with create_market)
#[test]
#[should_panic(expected = "Error(Contract, #300)")]
fn test_create_event_invalid_empty_description() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);

    let description = String::from_str(&setup.env, ""); // Empty description - invalid
    let outcomes = vec![
        &setup.env,
        String::from_str(&setup.env, "Yes"),
        String::from_str(&setup.env, "No"),
    ];
    let end_time = setup.env.ledger().timestamp() + 3600;
    let oracle_config = OracleConfig {
        provider: OracleProvider::reflector(),
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

/// Test that create_event validates end_time is in the future (parity with create_market)
#[test]
#[should_panic(expected = "Error(Contract, #302)")]
fn test_create_event_invalid_end_time_past() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);

    let description = String::from_str(&setup.env, "Past event?");
    let outcomes = vec![
        &setup.env,
        String::from_str(&setup.env, "Yes"),
        String::from_str(&setup.env, "No"),
    ];
    let end_time = setup.env.ledger().timestamp() - 3600; // Past time - invalid
    let oracle_config = OracleConfig {
        provider: OracleProvider::reflector(),
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

/// Test that create_event validates end_time equals current time (boundary condition)
#[test]
#[should_panic(expected = "Error(Contract, #302)")]
fn test_create_event_invalid_end_time_current() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);

    let description = String::from_str(&setup.env, "Current time event?");
    let outcomes = vec![
        &setup.env,
        String::from_str(&setup.env, "Yes"),
        String::from_str(&setup.env, "No"),
    ];
    let end_time = setup.env.ledger().timestamp(); // Current time - invalid (must be > current)
    let oracle_config = OracleConfig {
        provider: OracleProvider::reflector(),
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

/// Test that create_event validates unauthorized caller
#[test]
#[should_panic(expected = "Error(Contract, #100)")]
fn test_create_event_unauthorized_caller() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);

    let unauthorized_user = Address::generate(&setup.env);
    let description = String::from_str(&setup.env, "Unauthorized event?");
    let outcomes = vec![
        &setup.env,
        String::from_str(&setup.env, "Yes"),
        String::from_str(&setup.env, "No"),
    ];
    let end_time = setup.env.ledger().timestamp() + 3600;
    let oracle_config = OracleConfig {
        provider: OracleProvider::reflector(),
        oracle_address: Address::generate(&setup.env),
        feed_id: String::from_str(&setup.env, "BTC/USD"),
        threshold: 50000,
        comparison: String::from_str(&setup.env, "gt"),
    };

    client.create_event(
        &unauthorized_user,
        &description,
        &outcomes,
        &end_time,
        &oracle_config,
        &None,
        &0,
    );
}

/// Test that create_event with fallback oracle validates both configs
#[test]
fn test_create_event_with_fallback_oracle() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);

    let description = String::from_str(&setup.env, "Event with fallback oracle");
    let outcomes = vec![
        &setup.env,
        String::from_str(&setup.env, "Yes"),
        String::from_str(&setup.env, "No"),
    ];
    let end_time = setup.env.ledger().timestamp() + 3600;
    let oracle_config = OracleConfig {
        provider: OracleProvider::reflector(),
        oracle_address: Address::generate(&setup.env),
        feed_id: String::from_str(&setup.env, "BTC/USD"),
        threshold: 50000,
        comparison: String::from_str(&setup.env, "gt"),
    };
    let fallback_oracle_config = OracleConfig {
        provider: OracleProvider::pyth(),
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
        &Some(fallback_oracle_config.clone()),
        &0,
    );

    let event = client.get_event(&event_id).unwrap();
    assert!(event.has_fallback);
    assert_eq!(event.fallback_oracle_config.provider, fallback_oracle_config.provider);
}

/// Test that create_event with resolution timeout validates the timeout
#[test]
fn test_create_event_with_resolution_timeout() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);

    let description = String::from_str(&setup.env, "Event with resolution timeout");
    let outcomes = vec![
        &setup.env,
        String::from_str(&setup.env, "Yes"),
        String::from_str(&setup.env, "No"),
    ];
    let end_time = setup.env.ledger().timestamp() + 3600;
    let oracle_config = OracleConfig {
        provider: OracleProvider::reflector(),
        oracle_address: Address::generate(&setup.env),
        feed_id: String::from_str(&setup.env, "BTC/USD"),
        threshold: 50000,
        comparison: String::from_str(&setup.env, "gt"),
    };
    let resolution_timeout = 86400; // 1 day

    let event_id = client.create_event(
        &setup.admin,
        &description,
        &outcomes,
        &end_time,
        &oracle_config,
        &None,
        &resolution_timeout,
    );

    let event = client.get_event(&event_id).unwrap();
    assert_eq!(event.resolution_timeout, resolution_timeout);
}

/// Test that create_event with multiple outcomes works correctly
#[test]
fn test_create_event_multiple_outcomes() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);

    let description = String::from_str(&setup.env, "Multi-outcome event");
    let outcomes = vec![
        &setup.env,
        String::from_str(&setup.env, "Option A"),
        String::from_str(&setup.env, "Option B"),
        String::from_str(&setup.env, "Option C"),
        String::from_str(&setup.env, "Option D"),
    ];
    let end_time = setup.env.ledger().timestamp() + 3600;
    let oracle_config = OracleConfig {
        provider: OracleProvider::reflector(),
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

    let event = client.get_event(&event_id).unwrap();
    assert_eq!(event.outcomes.len(), 4);
    assert_eq!(event.status, MarketState::Active);
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
        provider: OracleProvider::reflector(),
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
        provider: OracleProvider::reflector(),
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
        provider: OracleProvider::reflector(),
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
        provider: OracleProvider::reflector(),
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
        provider: OracleProvider::reflector(),
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
        );
    }
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
        provider: OracleProvider::reflector(),
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
        provider: OracleProvider::reflector(),
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

    let stored = client.get_event(&event_id).unwrap();
    assert_eq!(stored.description, description);
    assert_eq!(stored.end_time, end_time);
    assert_eq!(stored.outcomes.len(), outcomes.len());
    assert_eq!(stored.id, event_id);
}
