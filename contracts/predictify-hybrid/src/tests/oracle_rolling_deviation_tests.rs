#![cfg(test)]

//! Tests for the rolling-median oracle deviation outlier rejection feature.
//!
//! Covers:
//! - `OracleDeviationHistory` ring buffer (empty, single, odd/even median, FIFO eviction, pop_last, MAD)
//! - `validate_oracle_data` with rolling-median enabled (first price accepted, outlier rejected, valid price passes)
//! - Config validation for `max_deviation_z_multiple`

use crate::oracles::{OracleDeviationHistory, OracleValidationConfigManager};
use crate::types::{GlobalOracleValidationConfig, EventOracleValidationConfig, OraclePriceData, OracleProvider};
use soroban_sdk::{Env, Symbol, String, IntoVal, Val};

// ============================================================================
// OracleDeviationHistory unit tests
// ============================================================================

#[test]
fn test_deviation_history_empty_median_is_none() {
    let env = Env::default();
    let history = OracleDeviationHistory::new(&env, 10);
    assert!(history.is_empty());
    assert_eq!(history.len(), 0);
    assert_eq!(history.rolling_median(), None);
}

#[test]
fn test_deviation_history_single_price() {
    let env = Env::default();
    let mut history = OracleDeviationHistory::new(&env, 10);
    history.push(100);
    assert_eq!(history.len(), 1);
    assert_eq!(history.rolling_median(), Some(100));
}

#[test]
fn test_deviation_history_odd_median() {
    let env = Env::default();
    let mut history = OracleDeviationHistory::new(&env, 10);
    history.push(300);
    history.push(100);
    history.push(200);
    // Sorted: [100, 200, 300], median index = 3/2 = 1 => 200
    assert_eq!(history.rolling_median(), Some(200));
}

#[test]
fn test_deviation_history_even_median_lower_middle() {
    let env = Env::default();
    let mut history = OracleDeviationHistory::new(&env, 10);
    history.push(400);
    history.push(100);
    history.push(300);
    history.push(200);
    // Sorted: [100, 200, 300, 400], median index = 4/2 = 2 => 300
    assert_eq!(history.rolling_median(), Some(300));
}

#[test]
fn test_deviation_history_fifo_eviction() {
    let env = Env::default();
    let mut history = OracleDeviationHistory::new(&env, 3);
    history.push(10);
    history.push(20);
    history.push(30);
    assert_eq!(history.len(), 3);
    assert_eq!(history.rolling_median(), Some(20)); // [10, 20, 30]

    history.push(40); // 10 is evicted, now [20, 30, 40]
    assert_eq!(history.len(), 3);
    assert_eq!(history.rolling_median(), Some(30)); // [20, 30, 40]

    history.push(50); // 20 is evicted, now [30, 40, 50]
    assert_eq!(history.rolling_median(), Some(40)); // [30, 40, 50]
}

#[test]
fn test_deviation_history_pop_last() {
    let env = Env::default();
    let mut history = OracleDeviationHistory::new(&env, 5);
    history.push(10);
    history.push(20);
    history.push(30);
    assert_eq!(history.len(), 3);

    history.pop_last();
    assert_eq!(history.len(), 2);
    assert_eq!(history.rolling_median(), Some(10)); // [10, 20], median = 10

    // Pop to empty
    history.pop_last();
    history.pop_last();
    assert!(history.is_empty());
    assert_eq!(history.rolling_median(), None);

    // Pop on empty is a no-op
    history.pop_last();
    assert!(history.is_empty());
}

#[test]
fn test_deviation_history_mad() {
    let env = Env::default();
    let mut history = OracleDeviationHistory::new(&env, 10);
    // MAD requires at least 2 entries
    history.push(100);
    assert_eq!(history.mad(), None);

    // [100, 110] -> median=100, deviations=[0,10] -> MAD=0
    history.push(110);
    assert_eq!(history.mad(), Some(0));
    // Actually [100, 110]: sorted deviations [0,10], mid=1 -> 10
    // Wait: deviations = [|100-100|=0, |110-100|=10], sorted = [0, 10], mid = 2/2 = 1 => 10
    assert_eq!(history.mad(), Some(10));

    // [100, 110, 200] -> median=110, deviations=[10,0,90], sorted=[0,10,90], mid=1 => 10
    history.push(200);
    assert_eq!(history.mad(), Some(10));
}

#[test]
fn test_deviation_history_capacity_zero_defaults_to_one() {
    let env = Env::default();
    let mut history = OracleDeviationHistory::new(&env, 0);
    assert_eq!(history.capacity, 1);
    history.push(100);
    history.push(200); // 100 gets evicted since capacity=1
    assert_eq!(history.len(), 1);
    assert_eq!(history.rolling_median(), Some(200));
}

#[test]
fn test_deviation_history_deterministic_same_input() {
    let env = Env::default();
    let prices = [500, 300, 400, 200, 100];

    let mut h1 = OracleDeviationHistory::new(&env, 10);
    let mut h2 = OracleDeviationHistory::new(&env, 10);
    for &p in &prices {
        h1.push(p);
        h2.push(p);
    }
    assert_eq!(h1.rolling_median(), h2.rolling_median());
}

// ============================================================================
// validate_oracle_data integration tests
// ============================================================================

fn make_price_data(env: &Env, price: i128, publish_time: u64) -> OraclePriceData {
    OraclePriceData {
        price,
        publish_time,
        confidence: None,
        exponent: 0,
    }
}

#[test]
fn test_rolling_median_first_price_accepted() {
    let env = Env::default();
    let contract_id = env.register(crate::PredictifyHybrid, ());
    let market_id = Symbol::new(&env, "test_market");
    let feed_id = String::from_str(&env, "BTC/USD");
    let provider = OracleProvider::reflector();

    env.as_contract(&contract_id, || {
        // Set config with rolling-median enabled
        let config = GlobalOracleValidationConfig {
            max_staleness_secs: 3600,
            max_confidence_bps: 1000,
            max_deviation_bps: None,
            max_deviation_z_multiple: Some(500), // 5% deviation allowed
            history_size: Some(10),
        };
        OracleValidationConfigManager::set_global_config(&env, &config).unwrap();

        // First price should always be accepted
        let data = make_price_data(&env, 50_000_00, env.ledger().timestamp());
        let result = OracleValidationConfigManager::validate_oracle_data(
            &env, &market_id, &provider, &feed_id, &data,
        );
        assert!(result.is_ok(), "First price should be accepted");
    });
}

#[test]
fn test_rolling_median_similar_price_accepted() {
    let env = Env::default();
    let contract_id = env.register(crate::PredictifyHybrid, ());
    let market_id = Symbol::new(&env, "test_market");
    let feed_id = String::from_str(&env, "BTC/USD");
    let provider = OracleProvider::reflector();

    env.as_contract(&contract_id, || {
        let config = GlobalOracleValidationConfig {
            max_staleness_secs: 3600,
            max_confidence_bps: 1000,
            max_deviation_bps: None,
            max_deviation_z_multiple: Some(500), // 5% = 500 bps
            history_size: Some(10),
        };
        OracleValidationConfigManager::set_global_config(&env, &config).unwrap();

        let t = env.ledger().timestamp();

        // First price
        let data1 = make_price_data(&env, 50_000_00, t);
        OracleValidationConfigManager::validate_oracle_data(
            &env, &market_id, &provider, &feed_id, &data1,
        ).unwrap();

        // Second price within 5% should be accepted
        let data2 = make_price_data(&env, 51_000_00, t); // 2% above
        let result = OracleValidationConfigManager::validate_oracle_data(
            &env, &market_id, &provider, &feed_id, &data2,
        );
        assert!(result.is_ok(), "Price within 5% should be accepted");
    });
}

#[test]
fn test_rolling_median_outlier_rejected() {
    let env = Env::default();
    let contract_id = env.register(crate::PredictifyHybrid, ());
    let market_id = Symbol::new(&env, "test_market");
    let feed_id = String::from_str(&env, "BTC/USD");
    let provider = OracleProvider::reflector();

    env.as_contract(&contract_id, || {
        let config = GlobalOracleValidationConfig {
            max_staleness_secs: 3600,
            max_confidence_bps: 1000,
            max_deviation_bps: None,
            max_deviation_z_multiple: Some(500), // 5% = 500 bps
            history_size: Some(10),
        };
        OracleValidationConfigManager::set_global_config(&env, &config).unwrap();

        let t = env.ledger().timestamp();

        // First price (establish baseline)
        let data1 = make_price_data(&env, 50_000_00, t);
        OracleValidationConfigManager::validate_oracle_data(
            &env, &market_id, &provider, &feed_id, &data1,
        ).unwrap();

        // Second price far outside 5% should be rejected
        let data2 = make_price_data(&env, 60_000_00, t); // 20% above
        let result = OracleValidationConfigManager::validate_oracle_data(
            &env, &market_id, &provider, &feed_id, &data2,
        );
        assert!(result.is_err(), "Price 20% above should be rejected");
        assert_eq!(result.unwrap_err(), crate::Error::OracleQuoteOutlier);
    });
}

#[test]
fn test_rolling_median_outlier_not_persisted() {
    let env = Env::default();
    let contract_id = env.register(crate::PredictifyHybrid, ());
    let market_id = Symbol::new(&env, "test_market");
    let feed_id = String::from_str(&env, "BTC/USD");
    let provider = OracleProvider::reflector();

    env.as_contract(&contract_id, || {
        let config = GlobalOracleValidationConfig {
            max_staleness_secs: 3600,
            max_confidence_bps: 1000,
            max_deviation_bps: None,
            max_deviation_z_multiple: Some(500),
            history_size: Some(10),
        };
        OracleValidationConfigManager::set_global_config(&env, &config).unwrap();

        let t = env.ledger().timestamp();

        // Establish baseline with two prices
        OracleValidationConfigManager::validate_oracle_data(
            &env, &market_id, &provider, &feed_id,
            &make_price_data(&env, 50_000_00, t),
        ).unwrap();

        OracleValidationConfigManager::validate_oracle_data(
            &env, &market_id, &provider, &feed_id,
            &make_price_data(&env, 51_000_00, t),
        ).unwrap();

        // Now submit an outlier - should be rejected
        let result = OracleValidationConfigManager::validate_oracle_data(
            &env, &market_id, &provider, &feed_id,
            &make_price_data(&env, 60_000_00, t),
        );
        assert!(result.is_err());

        // The outlier should NOT be in the history
        let history = OracleValidationConfigManager::get_deviation_history(&env, &market_id);
        assert!(history.is_some());
        let history = history.unwrap();
        assert_eq!(history.len(), 2, "Outlier should not be stored");
        assert_eq!(history.rolling_median(), Some(50_000_00), "Median should still be ~50k");
    });
}

#[test]
fn test_rolling_median_multiple_stable_passes() {
    let env = Env::default();
    let contract_id = env.register(crate::PredictifyHybrid, ());
    let market_id = Symbol::new(&env, "stable_market");
    let feed_id = String::from_str(&env, "ETH/USD");
    let provider = OracleProvider::reflector();

    env.as_contract(&contract_id, || {
        let config = GlobalOracleValidationConfig {
            max_staleness_secs: 3600,
            max_confidence_bps: 1000,
            max_deviation_bps: None,
            max_deviation_z_multiple: Some(1000), // 10% allowed
            history_size: Some(5),
        };
        OracleValidationConfigManager::set_global_config(&env, &config).unwrap();

        let t = env.ledger().timestamp();
        // Submit 5 stable prices with small variations
        let prices = [2_000_00, 2_010_00, 1_990_00, 2_020_00, 2_005_00];
        for &price in &prices {
            let result = OracleValidationConfigManager::validate_oracle_data(
                &env, &market_id, &provider, &feed_id,
                &make_price_data(&env, price, t),
            );
            assert!(result.is_ok(), "Stable price {} should be accepted", price);
        }

        // Check history has all 5 prices
        let history = OracleValidationConfigManager::get_deviation_history(&env, &market_id);
        assert!(history.is_some());
        assert_eq!(history.unwrap().len(), 5);
    });
}

#[test]
fn test_config_validation_rejects_zero_z_multiple() {
    let env = Env::default();
    let config = GlobalOracleValidationConfig {
        max_staleness_secs: 60,
        max_confidence_bps: 500,
        max_deviation_bps: None,
        max_deviation_z_multiple: Some(0), // Invalid
        history_size: Some(10),
    };
    let result = OracleValidationConfigManager::set_global_config(&env, &config);
    assert!(result.is_err(), "z_multiple of 0 should be rejected");
}

#[test]
fn test_config_validation_rejects_too_high_z_multiple() {
    let env = Env::default();
    let config = GlobalOracleValidationConfig {
        max_staleness_secs: 60,
        max_confidence_bps: 500,
        max_deviation_bps: None,
        max_deviation_z_multiple: Some(10_001), // > 100% invalid
        history_size: Some(10),
    };
    let result = OracleValidationConfigManager::set_global_config(&env, &config);
    assert!(result.is_err(), "z_multiple > 10_000 should be rejected");
}

#[test]
fn test_rolling_median_clear_history() {
    let env = Env::default();
    let contract_id = env.register(crate::PredictifyHybrid, ());
    let market_id = Symbol::new(&env, "clear_test");
    let feed_id = String::from_str(&env, "XLM/USD");
    let provider = OracleProvider::reflector();

    env.as_contract(&contract_id, || {
        let config = GlobalOracleValidationConfig {
            max_staleness_secs: 3600,
            max_confidence_bps: 1000,
            max_deviation_bps: None,
            max_deviation_z_multiple: Some(500),
            history_size: Some(10),
        };
        OracleValidationConfigManager::set_global_config(&env, &config).unwrap();

        let t = env.ledger().timestamp();

        // Submit some prices
        OracleValidationConfigManager::validate_oracle_data(
            &env, &market_id, &provider, &feed_id,
            &make_price_data(&env, 100, t),
        ).unwrap();

        assert!(OracleValidationConfigManager::get_deviation_history(&env, &market_id).is_some());

        // Clear history
        OracleValidationConfigManager::clear_deviation_history(&env, &market_id);
        assert!(OracleValidationConfigManager::get_deviation_history(&env, &market_id).is_none());
    });
}

#[test]
fn test_legacy_deviation_still_works_when_rolling_disabled() {
    let env = Env::default();
    let contract_id = env.register(crate::PredictifyHybrid, ());
    let market_id = Symbol::new(&env, "legacy_market");
    let feed_id = String::from_str(&env, "BTC/USD");
    let provider = OracleProvider::reflector();

    env.as_contract(&contract_id, || {
        // Only set legacy deviation (not rolling median)
        let config = GlobalOracleValidationConfig {
            max_staleness_secs: 3600,
            max_confidence_bps: 1000,
            max_deviation_bps: Some(500), // Legacy: 5% allowed
            max_deviation_z_multiple: None,
            history_size: None,
        };
        OracleValidationConfigManager::set_global_config(&env, &config).unwrap();

        let t = env.ledger().timestamp();

        // First price
        OracleValidationConfigManager::validate_oracle_data(
            &env, &market_id, &provider, &feed_id,
            &make_price_data(&env, 50_000_00, t),
        ).unwrap();

        // Within 5%: accepted
        let result = OracleValidationConfigManager::validate_oracle_data(
            &env, &market_id, &provider, &feed_id,
            &make_price_data(&env, 52_000_00, t), // 4% above
        );
        assert!(result.is_ok(), "Legacy: price within 5% should be accepted");

        // Outside 5%: rejected
        let result = OracleValidationConfigManager::validate_oracle_data(
            &env, &market_id, &provider, &feed_id,
            &make_price_data(&env, 55_000_00, t), // 10% above
        );
        assert!(result.is_err(), "Legacy: price outside 5% should be rejected");
        assert_eq!(result.unwrap_err(), crate::Error::OracleNoConsensus);
    });
}
