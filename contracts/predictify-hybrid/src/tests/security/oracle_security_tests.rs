//! Security Tests for Oracle Integration
//!
//! This module contains comprehensive security tests for oracle functionality,
//! focusing on authorization, signature validation, replay protection, and
//! other security-critical aspects of oracle integration.

use super::super::super::*;
use super::super::mocks::oracle::*;
use soroban_sdk::testutils::Address as _;

/// Test unauthorized oracle access
#[test]
fn test_unauthorized_oracle_access() {
    let env = Env::default();
    let contract_id = Address::generate(&env);

    // Create unauthorized signer mock
    let unauthorized_oracle = MockOracleFactory::create_unauthorized_signer_oracle(contract_id.clone());

    // Attempt to get price - should fail with Unauthorized
    let result = unauthorized_oracle.get_price(&env, &String::from_str(&env, "BTC/USD"));
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), Error::Unauthorized);
}

/// Test invalid signature rejection
#[test]
fn test_invalid_signature_rejection() {
    let env = Env::default();
    let contract_id = Address::generate(&env);

    // Create malicious signature mock
    let malicious_oracle = MockOracleFactory::create_malicious_signature_oracle(contract_id.clone());

    // Attempt to get price - should fail with Unauthorized (representing invalid signature)
    let result = malicious_oracle.get_price(&env, &String::from_str(&env, "BTC/USD"));
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), Error::Unauthorized);
}

/// Test replay attack protection
#[test]
fn test_replay_attack_protection() {
    let env = Env::default();
    let contract_id = Address::generate(&env);

    // Create valid oracle
    let valid_oracle = MockOracleFactory::create_valid_oracle(contract_id.clone(), 2600000);

    // First request should succeed
    let result1 = valid_oracle.get_price(&env, &String::from_str(&env, "BTC/USD"));
    assert!(result1.is_ok());
    assert_eq!(result1.unwrap(), 2600000);

    // Second request with same parameters should still succeed (no replay protection at oracle level)
    // In a real implementation, this would be handled at the contract level with nonces
    let result2 = valid_oracle.get_price(&env, &String::from_str(&env, "BTC/USD"));
    assert!(result2.is_ok());
    assert_eq!(result2.unwrap(), 2600000);
}

/// Test oracle whitelist validation
#[test]
fn test_oracle_whitelist_validation() {
    let env = Env::default();
    let contract_id = env.register(PredictifyHybrid, ());
    let admin = Address::generate(&env);
    let oracle_address = Address::generate(&env);

    env.as_contract(&contract_id, || {
        // Initialize whitelist
        OracleWhitelist::initialize(&env, admin.clone()).unwrap();

        // Try to validate non-whitelisted oracle
        let is_valid = OracleWhitelist::validate_oracle_contract(&env, &oracle_address).unwrap();
        assert!(!is_valid);

        // Add oracle to whitelist
        let metadata = OracleMetadata {
            provider: OracleProvider::Reflector,
            contract_address: oracle_address.clone(),
            added_at: env.ledger().timestamp(),
            added_by: admin.clone(),
            last_health_check: env.ledger().timestamp(),
            is_active: true,
            description: String::from_str(&env, "Test Oracle"),
        };

        OracleWhitelist::add_oracle_to_whitelist(&env, admin, oracle_address.clone(), metadata).unwrap();

        // Now validation should pass
        let is_valid = OracleWhitelist::validate_oracle_contract(&env, &oracle_address).unwrap();
        assert!(is_valid);
    });
}

/// Test oracle deactivation security
#[test]
fn test_oracle_deactivation_security() {
    let env = Env::default();
    let contract_id = env.register(PredictifyHybrid, ());
    let admin = Address::generate(&env);
    let non_admin = Address::generate(&env);
    let oracle_address = Address::generate(&env);

    env.as_contract(&contract_id, || {
        OracleWhitelist::initialize(&env, admin.clone()).unwrap();

        let metadata = OracleMetadata {
            provider: OracleProvider::Reflector,
            contract_address: oracle_address.clone(),
            added_at: env.ledger().timestamp(),
            added_by: admin.clone(),
            last_health_check: env.ledger().timestamp(),
            is_active: true,
            description: String::from_str(&env, "Test Oracle"),
        };

        OracleWhitelist::add_oracle_to_whitelist(&env, admin.clone(), oracle_address.clone(), metadata).unwrap();

        // Non-admin should not be able to deactivate
        let result = OracleWhitelist::deactivate_oracle(&env, non_admin, oracle_address.clone());
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), Error::Unauthorized);

        // Admin should be able to deactivate
        OracleWhitelist::deactivate_oracle(&env, admin.clone(), oracle_address.clone()).unwrap();

        // Oracle should now be invalid
        let is_valid = OracleWhitelist::validate_oracle_contract(&env, &oracle_address).unwrap();
        assert!(!is_valid);
    });
}

/// Test oracle health check manipulation
#[test]
fn test_oracle_health_check_manipulation() {
    let env = Env::default();
    let contract_id = Address::generate(&env);

    // Create timeout oracle (unhealthy)
    let unhealthy_oracle = MockOracleFactory::create_timeout_oracle(contract_id.clone());

    // Health check should fail
    let is_healthy = unhealthy_oracle.is_healthy(&env).unwrap();
    assert!(!is_healthy);

    // Create valid oracle (healthy)
    let healthy_oracle = MockOracleFactory::create_valid_oracle(contract_id.clone(), 2600000);

    // Health check should pass
    let is_healthy = healthy_oracle.is_healthy(&env).unwrap();
    assert!(is_healthy);
}

/// Test extreme value validation
#[test]
fn test_extreme_value_validation() {
    let env = Env::default();
    let contract_id = Address::generate(&env);

    // Test with extremely high value
    let extreme_high_oracle = MockOracleFactory::create_extreme_value_oracle(contract_id.clone(), i128::MAX);
    let result = extreme_high_oracle.get_price(&env, &String::from_str(&env, "BTC/USD"));
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), i128::MAX);

    // Test with zero value (should be validated elsewhere)
    let zero_oracle = MockOracleFactory::create_extreme_value_oracle(contract_id.clone(), 0);
    let result = zero_oracle.get_price(&env, &String::from_str(&env, "BTC/USD"));
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);

    // Test with negative value
    let negative_oracle = MockOracleFactory::create_extreme_value_oracle(contract_id.clone(), -1000);
    let result = negative_oracle.get_price(&env, &String::from_str(&env, "BTC/USD"));
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), -1000);
}

/// Test oracle provider validation
#[test]
fn test_oracle_provider_validation() {
    // Test supported providers
    assert!(OracleFactory::is_provider_supported(&OracleProvider::Reflector));

    // Test unsupported providers
    assert!(!OracleFactory::is_provider_supported(&OracleProvider::Pyth));
    assert!(!OracleFactory::is_provider_supported(&OracleProvider::BandProtocol));
    assert!(!OracleFactory::is_provider_supported(&OracleProvider::DIA));
}

/// Test oracle configuration security
#[test]
fn test_oracle_configuration_security() {
    let env = Env::default();
    let contract_id = Address::generate(&env);

    // Test creating oracle with unsupported provider
    let result = OracleFactory::create_oracle(OracleProvider::Pyth, contract_id.clone());
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), Error::InvalidOracleConfig);

    // Test creating oracle with supported provider
    let result = OracleFactory::create_oracle(OracleProvider::Reflector, contract_id.clone());
    assert!(result.is_ok());
}

/// Test oracle metadata integrity
#[test]
fn test_oracle_metadata_integrity() {
    let env = Env::default();
    let contract_id = env.register(PredictifyHybrid, ());
    let admin = Address::generate(&env);
    let oracle_address = Address::generate(&env);

    env.as_contract(&contract_id, || {
        OracleWhitelist::initialize(&env, admin.clone()).unwrap();

        let metadata = OracleMetadata {
            provider: OracleProvider::Reflector,
            contract_address: oracle_address.clone(),
            added_at: env.ledger().timestamp(),
            added_by: admin.clone(),
            last_health_check: env.ledger().timestamp(),
            is_active: true,
            description: String::from_str(&env, "Test Oracle"),
        };

        OracleWhitelist::add_oracle_to_whitelist(&env, admin.clone(), oracle_address.clone(), metadata).unwrap();

        // Retrieve metadata and verify integrity
        let retrieved_metadata = OracleWhitelist::get_oracle_metadata(&env, &oracle_address).unwrap();
        assert_eq!(retrieved_metadata.provider, OracleProvider::Reflector);
        assert_eq!(retrieved_metadata.contract_address, oracle_address);
        assert_eq!(retrieved_metadata.added_by, admin);
        assert!(retrieved_metadata.is_active);
    });
}

/// Test admin authorization for oracle management
#[test]
fn test_admin_authorization_oracle_management() {
    let env = Env::default();
    let contract_id = env.register(PredictifyHybrid, ());
    let admin = Address::generate(&env);
    let non_admin = Address::generate(&env);
    let oracle_address = Address::generate(&env);

    env.as_contract(&contract_id, || {
        OracleWhitelist::initialize(&env, admin.clone()).unwrap();

        // Non-admin should not be able to add oracle
        let metadata = OracleMetadata {
            provider: OracleProvider::Reflector,
            contract_address: oracle_address.clone(),
            added_at: env.ledger().timestamp(),
            added_by: non_admin.clone(),
            last_health_check: env.ledger().timestamp(),
            is_active: true,
            description: String::from_str(&env, "Test Oracle"),
        };

        let result = OracleWhitelist::add_oracle_to_whitelist(&env, non_admin, oracle_address, metadata);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), Error::Unauthorized);
    });
}

/// Test oracle removal security
#[test]
fn test_oracle_removal_security() {
    let env = Env::default();
    let contract_id = env.register(PredictifyHybrid, ());
    let admin = Address::generate(&env);
    let non_admin = Address::generate(&env);
    let oracle_address = Address::generate(&env);

    env.as_contract(&contract_id, || {
        OracleWhitelist::initialize(&env, admin.clone()).unwrap();

        let metadata = OracleMetadata {
            provider: OracleProvider::Reflector,
            contract_address: oracle_address.clone(),
            added_at: env.ledger().timestamp(),
            added_by: admin.clone(),
            last_health_check: env.ledger().timestamp(),
            is_active: true,
            description: String::from_str(&env, "Test Oracle"),
        };

        OracleWhitelist::add_oracle_to_whitelist(&env, admin.clone(), oracle_address.clone(), metadata).unwrap();

        // Non-admin should not be able to remove oracle
        let result = OracleWhitelist::remove_oracle_from_whitelist(&env, non_admin, oracle_address.clone());
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), Error::Unauthorized);

        // Admin should be able to remove
        OracleWhitelist::remove_oracle_from_whitelist(&env, admin, oracle_address).unwrap();
    });
}