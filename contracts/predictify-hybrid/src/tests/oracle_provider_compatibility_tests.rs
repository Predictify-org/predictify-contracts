use crate::errors::Error;
use soroban_sdk::{contracttype, Address, Env, String};
use crate::types::{OracleConfig, OracleProvider};

/// Comprehensive tests for oracle provider forward compatibility.
///
/// This test suite validates that the new string-based OracleProvider implementation
/// maintains backward compatibility while enabling forward compatibility for future
/// oracle provider additions.
#[contracttest]
fn test_oracle_provider_creation() {
    let env = Env::default();
    
    // Test standard provider creation
    let reflector = OracleProvider::reflector();
    assert_eq!(reflector.as_str(), "reflector");
    assert!(reflector.is_supported());
    assert!(reflector.is_known());
    
    let pyth = OracleProvider::pyth();
    assert_eq!(pyth.as_str(), "pyth");
    assert!(!pyth.is_supported());
    assert!(pyth.is_known());
    
    let band_protocol = OracleProvider::band_protocol();
    assert_eq!(band_protocol.as_str(), "band_protocol");
    assert!(!band_protocol.is_supported());
    assert!(band_protocol.is_known());
    
    let dia = OracleProvider::dia();
    assert_eq!(dia.as_str(), "dia");
    assert!(!dia.is_supported());
    assert!(dia.is_known());
}

#[contracttest]
fn test_oracle_provider_from_string() {
    let env = Env::default();
    
    // Test known providers
    let reflector = OracleProvider::from_str(String::from_str(&env, "reflector"));
    assert_eq!(reflector.as_str(), "reflector");
    assert!(reflector.is_supported());
    assert!(reflector.is_known());
    
    // Test unknown providers (forward compatibility)
    let future_provider = OracleProvider::from_str(String::from_str(&env, "chainlink"));
    assert_eq!(future_provider.as_str(), "chainlink");
    assert!(!future_provider.is_supported());
    assert!(!future_provider.is_known());
    
    let custom_provider = OracleProvider::from_str(String::from_str(&env, "custom_oracle_v2"));
    assert_eq!(custom_provider.as_str(), "custom_oracle_v2");
    assert!(!custom_provider.is_supported());
    assert!(!custom_provider.is_known());
}

#[contracttest]
fn test_oracle_provider_names() {
    let env = Env::default();
    
    // Test known provider names
    let reflector = OracleProvider::reflector();
    assert_eq!(reflector.name(), String::from_str(&env, "Reflector"));
    
    let pyth = OracleProvider::pyth();
    assert_eq!(pyth.name(), String::from_str(&env, "Pyth Network"));
    
    let band_protocol = OracleProvider::band_protocol();
    assert_eq!(band_protocol.name(), String::from_str(&env, "Band Protocol"));
    
    let dia = OracleProvider::dia();
    assert_eq!(dia.name(), String::from_str(&env, "DIA"));
    
    // Test unknown provider name formatting
    let unknown = OracleProvider::from_str(String::from_str(&env, "new_oracle"));
    let expected_name = String::from_str(&env, "Unknown Provider (new_oracle)");
    assert_eq!(unknown.name(), expected_name);
}

#[contracttest]
fn test_oracle_provider_validation() {
    let env = Env::default();
    
    // Test supported provider validation
    let reflector = OracleProvider::reflector();
    assert!(reflector.validate_for_market(&env).is_ok());
    
    // Test known but unsupported providers
    let pyth = OracleProvider::pyth();
    assert!(pyth.validate_for_market(&env).is_err());
    assert!(matches!(pyth.validate_for_market(&env), Err(Error::InvalidOracleConfig)));
    
    let band_protocol = OracleProvider::band_protocol();
    assert!(band_protocol.validate_for_market(&env).is_err());
    
    let dia = OracleProvider::dia();
    assert!(dia.validate_for_market(&env).is_err());
    
    // Test unknown providers
    let unknown = OracleProvider::from_str(String::from_str(&env, "unknown_provider"));
    assert!(unknown.validate_for_market(&env).is_err());
}

#[contracttest]
fn test_oracle_provider_equality() {
    let env = Env::default();
    
    // Test equality for known providers
    let reflector1 = OracleProvider::reflector();
    let reflector2 = OracleProvider::reflector();
    assert_eq!(reflector1, reflector2);
    
    let pyth1 = OracleProvider::pyth();
    let pyth2 = OracleProvider::from_str(String::from_str(&env, "pyth"));
    assert_eq!(pyth1, pyth2);
    
    // Test inequality
    let reflector = OracleProvider::reflector();
    let pyth = OracleProvider::pyth();
    assert_ne!(reflector, pyth);
    
    // Test unknown provider equality
    let unknown1 = OracleProvider::from_str(String::from_str(&env, "custom_oracle"));
    let unknown2 = OracleProvider::from_str(String::from_str(&env, "custom_oracle"));
    assert_eq!(unknown1, unknown2);
    
    let unknown3 = OracleProvider::from_str(String::from_str(&env, "different_oracle"));
    assert_ne!(unknown1, unknown3);
}

#[contracttest]
fn test_oracle_config_compatibility() {
    let env = Env::default();
    let oracle_address = Address::generate(&env);
    
    // Test creating oracle config with new provider
    let provider = OracleProvider::reflector();
    let config = OracleConfig::new(
        provider.clone(),
        oracle_address.clone(),
        String::from_str(&env, "BTC/USD"),
        50_000_00, // $50,000 in cents
        String::from_str(&env, "gt"),
    );
    
    // Validate config works with new provider
    assert!(config.validate(&env).is_ok());
    assert_eq!(config.provider.as_str(), "reflector");
    
    // Test sentinel config
    let sentinel = OracleConfig::none_sentinel(&env);
    assert!(sentinel.is_none_sentinel());
    assert_eq!(sentinel.provider.as_str(), "reflector");
    assert!(sentinel.feed_id.is_empty());
    assert_eq!(sentinel.threshold, 0);
    assert!(sentinel.comparison.is_empty());
}

#[contracttest]
fn test_forward_compatibility_scenario() {
    let env = Env::default();
    
    // Simulate a market created with a future oracle provider
    // This would happen when a market is created with a newer contract version
    // and then read by an older version
    let future_provider = OracleProvider::from_str(String::from_str(&env, "chainlink"));
    
    // The older contract should be able to read the provider
    assert_eq!(future_provider.as_str(), "chainlink");
    assert!(!future_provider.is_known()); // Not known in this version
    assert!(!future_provider.is_supported()); // Not supported
    
    // But it should provide sensible defaults
    let name = future_provider.name();
    assert!(name.to_string(&env).contains("Unknown Provider"));
    
    // And validation should fail safely
    assert!(future_provider.validate_for_market(&env).is_err());
}

#[contracttest]
fn test_serialization_roundtrip() {
    let env = Env::default();
    
    // Test that providers can be serialized and deserialized correctly
    let original = OracleProvider::reflector();
    
    // In Soroban, contracttype ensures proper serialization
    // We test equality after "serialization" by creating identical instances
    let deserialized = OracleProvider::from_str(String::from_str(&env, "reflector"));
    assert_eq!(original, deserialized);
    
    // Test with unknown provider
    let unknown_original = OracleProvider::from_str(String::from_str(&env, "future_oracle"));
    let unknown_deserialized = OracleProvider::from_str(String::from_str(&env, "future_oracle"));
    assert_eq!(unknown_original, unknown_deserialized);
}

#[contracttest]
fn test_oracle_config_validation_with_new_provider() {
    let env = Env::default();
    let oracle_address = Address::generate(&env);
    
    // Test that oracle config validation works with new provider system
    let valid_config = OracleConfig::new(
        OracleProvider::reflector(),
        oracle_address,
        String::from_str(&env, "BTC/USD"),
        50_000_00,
        String::from_str(&env, "gt"),
    );
    assert!(valid_config.validate(&env).is_ok());
    
    // Test with unsupported provider
    let unsupported_config = OracleConfig::new(
        OracleProvider::pyth(),
        oracle_address,
        String::from_str(&env, "ETH/USD"),
        2_000_00,
        String::from_str(&env, "lt"),
    );
    assert!(unsupported_config.validate(&env).is_err());
    
    // Test with unknown provider
    let unknown_config = OracleConfig::new(
        OracleProvider::from_str(String::from_str(&env, "unknown_oracle")),
        oracle_address,
        String::from_str(&env, "XLM/USD"),
        100,
        String::from_str(&env, "eq"),
    );
    assert!(unknown_config.validate(&env).is_err());
}

#[contracttest]
fn test_provider_string_formats() {
    let env = Env::default();
    
    // Test that provider IDs follow expected format
    let providers = vec![
        (OracleProvider::reflector(), "reflector"),
        (OracleProvider::pyth(), "pyth"),
        (OracleProvider::band_protocol(), "band_protocol"),
        (OracleProvider::dia(), "dia"),
    ];
    
    for (provider, expected_id) in providers {
        assert_eq!(provider.as_str(), expected_id);
        assert_eq!(provider.as_str(), expected_id.to_string());
    }
    
    // Test custom provider formats
    let custom_cases = vec![
        ("chainlink", "chainlink"),
        ("uniswap_oracle", "uniswap_oracle"),
        ("custom_provider_v2", "custom_provider_v2"),
        ("UPPERCASE_PROVIDER", "UPPERCASE_PROVIDER"),
        ("provider-with-dashes", "provider-with-dashes"),
    ];
    
    for (input, expected) in custom_cases {
        let provider = OracleProvider::from_str(String::from_str(&env, input));
        assert_eq!(provider.as_str(), expected);
    }
}

#[contracttest]
fn test_migration_compatibility() {
    let env = Env::default();
    
    // This test simulates migration from the old enum-based system
    // In practice, this would be handled by a migration function
    
    // Simulate old enum variants (as strings for testing)
    let old_variants = vec![
        ("Reflector", "reflector"),
        ("Pyth", "pyth"),
        ("BandProtocol", "band_protocol"),
        ("DIA", "dia"),
    ];
    
    for (old_enum_name, expected_string_id) in old_variants {
        // Simulate migration logic
        let new_provider = match old_enum_name {
            "Reflector" => OracleProvider::reflector(),
            "Pyth" => OracleProvider::pyth(),
            "BandProtocol" => OracleProvider::band_protocol(),
            "DIA" => OracleProvider::dia(),
            _ => OracleProvider::from_str(String::from_str(&env, "unknown")),
        };
        
        assert_eq!(new_provider.as_str(), expected_string_id);
    }
}
