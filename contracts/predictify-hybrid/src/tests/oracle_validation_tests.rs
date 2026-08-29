//! Validation Tests for Oracle Configuration
//!
//! This module contains tests that verify the strict validation logic for
//! oracle configurations, including provider support and provider-specific
//! feed ID constraints.

use super::*;
use crate::markets::{MarketPauseManager, MarketStateManager};
use crate::oracles::OracleValidationConfigManager;
use crate::types::MarketPauseInfo;
use soroban_sdk::{Env, String, Address, Symbol, Vec, Map, IntoVal, vec};
use soroban_sdk::testutils::{Address as _, Ledger};

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

// ============================================================================
// Auto-pause on oracle validation failure tests
// ============================================================================

/// Helper: set up contract environment with a stored market and admin.
fn setup_auto_pause_env(env: &Env, contract_id: &Address) -> Symbol {
    let admin = Address::generate(env);
    let market_id = Symbol::new(env, "auto_pause_market");

    env.as_contract(contract_id, || {
        env.storage().persistent().set(&Symbol::new(env, "Admin"), &admin);

        let market = Market {
            admin: admin.clone(),
            question: String::from_str(env, "Test?"),
            outcomes: Vec::from_array(env, [String::from_str(env, "yes"), String::from_str(env, "no")]),
            end_time: env.ledger().timestamp() + 3600,
            oracle_config: OracleConfig::new(
                OracleProvider::reflector(),
                Address::generate(env),
                String::from_str(env, "BTC/USD"),
                50_000_00,
                String::from_str(env, "gt"),
            ),
            metadata_commitment: BytesN::from_array(env, &[0u8; 32]),
            has_fallback: false,
            fallback_oracle_config: OracleConfig::none_sentinel(env),
            resolution_timeout: 86400,
            oracle_result: None,
            votes: Map::new(env),
            stakes: Map::new(env),
            claimed: Map::new(env),
            total_staked: 0,
            dispute_stakes: Map::new(env),
            winning_outcomes: None,
            fee_collected: false,
            state: MarketState::Active,
            total_extension_days: 0,
            max_extension_days: 30,
            extension_history: Vec::new(env),
            category: None,
            tags: Vec::new(env),
            min_pool_size: None,
            bet_deadline: 0,
            dispute_window_seconds: 86400,
            winnings_swept: false,
            timelock_config: crate::timelock::MarketTimelockConfig::default(),
            dispute_stake_floor: None,
            max_participants: None,
        };
        env.storage().persistent().set(&market_id, &market);
    });

    market_id
}

#[test]
fn test_auto_pause_config_validation_accepts_valid() {
    let env = Env::default();
    let config = GlobalOracleValidationConfig {
        max_staleness_secs: 60,
        max_confidence_bps: 500,
        max_deviation_bps: None,
        max_deviation_z_multiple: None,
        history_size: None,
        auto_pause_duration_secs: Some(3600),
    };
    assert!(OracleValidationConfigManager::set_global_config(&env, &config).is_ok());
}

#[test]
fn test_auto_pause_config_validation_rejects_zero() {
    let env = Env::default();
    let config = GlobalOracleValidationConfig {
        max_staleness_secs: 60,
        max_confidence_bps: 500,
        max_deviation_bps: None,
        max_deviation_z_multiple: None,
        history_size: None,
        auto_pause_duration_secs: Some(0),
    };
    let result = OracleValidationConfigManager::set_global_config(&env, &config);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), Error::InvalidInput);
}

#[test]
fn test_auto_pause_config_validation_rejects_too_large() {
    let env = Env::default();
    let config = GlobalOracleValidationConfig {
        max_staleness_secs: 60,
        max_confidence_bps: 500,
        max_deviation_bps: None,
        max_deviation_z_multiple: None,
        history_size: None,
        auto_pause_duration_secs: Some(604_801),
    };
    let result = OracleValidationConfigManager::set_global_config(&env, &config);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), Error::InvalidInput);
}

#[test]
fn test_auto_pause_staleness_triggers_pause() {
    let env = Env::default();
    let contract_id = env.register_contract(None, crate::PredictifyHybrid);
    let market_id = setup_auto_pause_env(&env, &contract_id);

    env.as_contract(&contract_id, || {
        env.ledger().with_mut(|li| li.timestamp = 100);

        let config = GlobalOracleValidationConfig {
            max_staleness_secs: 10,
            max_confidence_bps: 500,
            max_deviation_bps: None,
            max_deviation_z_multiple: None,
            history_size: None,
            auto_pause_duration_secs: Some(3600),
        };
        OracleValidationConfigManager::set_global_config(&env, &config).unwrap();

        let data = OraclePriceData {
            price: 100_00,
            publish_time: env.ledger().timestamp().saturating_sub(20),
            confidence: None,
            exponent: 0,
        };

        let result = OracleValidationConfigManager::validate_oracle_data(
            &env, &market_id, &OracleProvider::reflector(),
            &String::from_str(&env, "BTC/USD"), &data,
        );
        assert_eq!(result.unwrap_err(), Error::OracleStale);
        assert!(MarketPauseManager::is_market_paused(&env, &market_id).unwrap());
    });
}

#[test]
fn test_auto_pause_none_does_not_pause() {
    let env = Env::default();
    let contract_id = env.register_contract(None, crate::PredictifyHybrid);
    let market_id = setup_auto_pause_env(&env, &contract_id);

    env.as_contract(&contract_id, || {
        env.ledger().with_mut(|li| li.timestamp = 100);

        let config = GlobalOracleValidationConfig {
            max_staleness_secs: 10,
            max_confidence_bps: 500,
            max_deviation_bps: None,
            max_deviation_z_multiple: None,
            history_size: None,
            auto_pause_duration_secs: None,
        };
        OracleValidationConfigManager::set_global_config(&env, &config).unwrap();

        let data = OraclePriceData {
            price: 100_00,
            publish_time: env.ledger().timestamp().saturating_sub(20),
            confidence: None,
            exponent: 0,
        };

        let result = OracleValidationConfigManager::validate_oracle_data(
            &env, &market_id, &OracleProvider::reflector(),
            &String::from_str(&env, "BTC/USD"), &data,
        );
        assert_eq!(result.unwrap_err(), Error::OracleStale);
        assert!(!MarketPauseManager::is_market_paused(&env, &market_id).unwrap());
    });
}

#[test]
fn test_auto_pause_already_paused_is_noop() {
    let env = Env::default();
    let contract_id = env.register_contract(None, crate::PredictifyHybrid);
    let market_id = setup_auto_pause_env(&env, &contract_id);

    env.as_contract(&contract_id, || {
        let admin: Address = env.storage().persistent().get(&Symbol::new(&env, "Admin")).unwrap();
        MarketPauseManager::pause_market(&env, admin, &market_id, 1).unwrap();
        assert!(MarketPauseManager::is_market_paused(&env, &market_id).unwrap());

        // Second auto-pause should be a no-op
        MarketPauseManager::auto_pause_market(&env, &market_id, 3600).unwrap();
        assert!(MarketPauseManager::is_market_paused(&env, &market_id).unwrap());
    });
}

#[test]
fn test_auto_pause_config_per_event_override() {
    let env = Env::default();
    let contract_id = env.register_contract(None, crate::PredictifyHybrid);
    let market_id = setup_auto_pause_env(&env, &contract_id);

    env.as_contract(&contract_id, || {
        env.ledger().with_mut(|li| li.timestamp = 100);

        let global = GlobalOracleValidationConfig {
            max_staleness_secs: 60,
            max_confidence_bps: 500,
            max_deviation_bps: None,
            max_deviation_z_multiple: None,
            history_size: None,
            auto_pause_duration_secs: None,
        };
        OracleValidationConfigManager::set_global_config(&env, &global).unwrap();

        let event_cfg = EventOracleValidationConfig {
            max_staleness_secs: 5,
            max_confidence_bps: 500,
            max_deviation_bps: None,
            max_deviation_z_multiple: None,
            history_size: None,
            auto_pause_duration_secs: Some(7200),
        };
        OracleValidationConfigManager::set_event_config(&env, &market_id, &event_cfg).unwrap();

        let data = OraclePriceData {
            price: 100_00,
            publish_time: env.ledger().timestamp().saturating_sub(10),
            confidence: None,
            exponent: 0,
        };

        let result = OracleValidationConfigManager::validate_oracle_data(
            &env, &market_id, &OracleProvider::reflector(),
            &String::from_str(&env, "BTC/USD"), &data,
        );
        assert_eq!(result.unwrap_err(), Error::OracleStale);
        assert!(MarketPauseManager::is_market_paused(&env, &market_id).unwrap());
    });
}

#[test]
fn test_auto_pause_deviation_spike_triggers_pause() {
    let env = Env::default();
    let contract_id = env.register_contract(None, crate::PredictifyHybrid);
    let market_id = setup_auto_pause_env(&env, &contract_id);

    env.as_contract(&contract_id, || {
        let config = GlobalOracleValidationConfig {
            max_staleness_secs: 60,
            max_confidence_bps: 500,
            max_deviation_bps: Some(500),
            max_deviation_z_multiple: None,
            history_size: None,
            auto_pause_duration_secs: Some(3600),
        };
        OracleValidationConfigManager::set_global_config(&env, &config).unwrap();

        let first = OraclePriceData {
            price: 100_000_00,
            publish_time: env.ledger().timestamp(),
            confidence: None,
            exponent: 0,
        };
        OracleValidationConfigManager::validate_oracle_data(
            &env, &market_id, &OracleProvider::reflector(),
            &String::from_str(&env, "BTC"), &first,
        ).unwrap();

        let spike = OraclePriceData {
            price: 200_000_00,
            publish_time: env.ledger().timestamp(),
            confidence: None,
            exponent: 0,
        };
        let result = OracleValidationConfigManager::validate_oracle_data(
            &env, &market_id, &OracleProvider::reflector(),
            &String::from_str(&env, "BTC"), &spike,
        );
        assert_eq!(result.unwrap_err(), Error::OracleNoConsensus);
        assert!(MarketPauseManager::is_market_paused(&env, &market_id).unwrap());
    });
}

