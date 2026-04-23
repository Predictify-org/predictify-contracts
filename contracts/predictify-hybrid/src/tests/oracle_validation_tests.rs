//! Validation Tests for Oracle Configuration
//!
//! This module contains tests that verify the strict validation logic for
//! oracle configurations, including provider support and provider-specific
//! feed ID constraints.

use super::super::*;
use soroban_sdk::{Env, String, Address, vec};

#[test]
fn test_oracle_provider_validation() {
    let env = Env::default();

    // Reflector is supported on Stellar
    let reflector = OracleProvider::reflector();
    assert!(reflector.validate_for_market(&env).is_ok());

    // Pyth is known but not yet supported on Stellar
    let pyth = OracleProvider::pyth();
    let pyth_result = pyth.validate_for_market(&env);
    assert!(pyth_result.is_err());
    assert_eq!(pyth_result.unwrap_err(), Error::InvalidOracleConfig);

    // Band Protocol is not supported on Stellar
    let band = OracleProvider::band_protocol();
    let band_result = band.validate_for_market(&env);
    assert!(band_result.is_err());
    assert_eq!(band_result.unwrap_err(), Error::InvalidOracleConfig);
}

#[test]
fn test_oracle_config_impossible_combinations() {
    let env = Env::default();
    let oracle_address = Address::generate(&env);

    // 1. Reflector with Pyth-like hex feed ID (impossible)
    let reflector_invalid = OracleConfig {
        provider: OracleProvider::reflector(),
        oracle_address: oracle_address.clone(),
        feed_id: String::from_str(&env, "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef12345678"),
        threshold: 100,
        comparison: String::from_str(&env, "gt"),
    };
    let result = reflector_invalid.validate(&env);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), Error::InvalidOracleConfig);

    // 2. Pyth with short feed ID (impossible)
    let pyth_invalid = OracleConfig {
        provider: OracleProvider::pyth(),
        oracle_address: oracle_address.clone(),
        feed_id: String::from_str(&env, "BTC/USD"),
        threshold: 100,
        comparison: String::from_str(&env, "gt"),
    };
    let result = pyth_invalid.validate(&env);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), Error::InvalidOracleConfig);

    // 3. Band with long feed ID (impossible)
    let band_invalid = OracleConfig {
        provider: OracleProvider::band_protocol(),
        oracle_address: oracle_address.clone(),
        feed_id: String::from_str(&env, "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef12345678"),
        threshold: 100,
        comparison: String::from_str(&env, "gt"),
    };
    let result = band_invalid.validate(&env);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), Error::InvalidOracleConfig);
}

#[test]
fn test_oracle_factory_stellar_compatibility() {
    let env = Env::default();
    let oracle_address = Address::generate(&env);

    // Valid Reflector config
    let reflector_valid = OracleConfig {
        provider: OracleProvider::reflector(),
        oracle_address: oracle_address.clone(),
        feed_id: String::from_str(&env, "BTC/USD"),
        threshold: 100,
        comparison: String::from_str(&env, "gt"),
    };
    assert!(crate::oracles::OracleFactory::validate_stellar_compatibility(&reflector_valid).is_ok());

    // Invalid Reflector (long ID)
    let reflector_invalid = OracleConfig {
        provider: OracleProvider::reflector(),
        oracle_address: oracle_address.clone(),
        feed_id: String::from_str(&env, "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef12345678"),
        threshold: 100,
        comparison: String::from_str(&env, "gt"),
    };
    assert!(crate::oracles::OracleFactory::validate_stellar_compatibility(&reflector_invalid).is_err());

    // Pyth with valid ID (passes compatibility check but would fail validate_for_market)
    let pyth_valid_id = OracleConfig {
        provider: OracleProvider::pyth(),
        oracle_address: oracle_address.clone(),
        feed_id: String::from_str(&env, "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef12345678"),
        threshold: 100,
        comparison: String::from_str(&env, "gt"),
    };
    assert!(crate::oracles::OracleFactory::validate_stellar_compatibility(&pyth_valid_id).is_ok());

    // Band Protocol (rejected by compatibility check)
    let band_config = OracleConfig {
        provider: OracleProvider::band_protocol(),
        oracle_address: oracle_address.clone(),
        feed_id: String::from_str(&env, "BTC/USD"),
        threshold: 100,
        comparison: String::from_str(&env, "gt"),
    };
    assert!(crate::oracles::OracleFactory::validate_stellar_compatibility(&band_config).is_err());
}

