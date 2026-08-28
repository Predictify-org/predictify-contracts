#![cfg(test)]

use crate::err::Error;
use crate::types::{DeviationBounds, OracleConfig, OracleProvider};
use crate::validation::DeviationValidator;
use soroban_sdk::{Address, Env, String};

// ===== DEVIATION CALCULATION TESTS =====

#[test]
fn test_calculate_deviation_equal_prices() {
    // When prices are equal, deviation should be 0
    let result = DeviationValidator::calculate_deviation_bps(1000, 1000);
    assert_eq!(result, Ok(0));
}

#[test]
fn test_calculate_deviation_5_percent() {
    // Price1: 1000, Price2: 950 -> 50/950 * 10000 ≈ 526 bps (5.26%)
    let result = DeviationValidator::calculate_deviation_bps(1000, 950);
    assert!(result.is_ok());
    let deviation = result.unwrap();
    // Should be around 526-527 bps
    assert!(deviation >= 520 && deviation <= 530);
}

#[test]
fn test_calculate_deviation_1_bps() {
    // Price1: 10000, Price2: 9999 -> 1/9999 * 10000 ≈ 1 bps
    let result = DeviationValidator::calculate_deviation_bps(10000, 9999);
    assert!(result.is_ok());
    let deviation = result.unwrap();
    // Should be 1 bps (rounded)
    assert_eq!(deviation, 1);
}

#[test]
fn test_calculate_deviation_50_percent() {
    // Price1: 200, Price2: 100 -> 100/100 * 10000 = 10000 bps (100%)
    let result = DeviationValidator::calculate_deviation_bps(200, 100);
    assert!(result.is_ok());
    let deviation = result.unwrap();
    // 100% deviation should be capped at 10000 bps
    assert_eq!(deviation, 10000);
}

#[test]
fn test_calculate_deviation_large_difference() {
    // Price1: 1000000, Price2: 1 -> very large deviation, should cap at 10000
    let result = DeviationValidator::calculate_deviation_bps(1000000, 1);
    assert!(result.is_ok());
    let deviation = result.unwrap();
    // Should be capped at 10000 (100%)
    assert_eq!(deviation, 10000);
}

// ===== DEVIATION VALIDATION TESTS =====

#[test]
fn test_validate_bounds_valid_0_percent() {
    let bounds = DeviationBounds {
        max_deviation_bps: 0,
        enforce_fallback_on_deviation: false,
    };
    assert!(bounds.is_valid());
    let result = DeviationValidator::validate_bounds(&bounds);
    assert!(result.is_ok());
}

#[test]
fn test_validate_bounds_valid_100_percent() {
    let bounds = DeviationBounds {
        max_deviation_bps: 10000,
        enforce_fallback_on_deviation: true,
    };
    assert!(bounds.is_valid());
    let result = DeviationValidator::validate_bounds(&bounds);
    assert!(result.is_ok());
}

#[test]
fn test_validate_bounds_valid_5_percent() {
    let bounds = DeviationBounds {
        max_deviation_bps: 500,
        enforce_fallback_on_deviation: true,
    };
    assert!(bounds.is_valid());
    let result = DeviationValidator::validate_bounds(&bounds);
    assert!(result.is_ok());
}

#[test]
fn test_validate_bounds_invalid_exceeds_max() {
    let bounds = DeviationBounds {
        max_deviation_bps: 10001,
        enforce_fallback_on_deviation: false,
    };
    assert!(!bounds.is_valid());
    let result = DeviationValidator::validate_bounds(&bounds);
    assert!(result.is_err());
    assert_eq!(result, Err(Error::InvalidDeviationBounds));
}

#[test]
fn test_validate_bounds_invalid_way_over_limit() {
    let bounds = DeviationBounds {
        max_deviation_bps: 100000,
        enforce_fallback_on_deviation: false,
    };
    assert!(!bounds.is_valid());
    let result = DeviationValidator::validate_bounds(&bounds);
    assert!(result.is_err());
}

// ===== DEVIATION CHECKING TESTS =====

#[test]
fn test_check_deviation_within_bounds() {
    let bounds = DeviationBounds {
        max_deviation_bps: 1000, // 10%
        enforce_fallback_on_deviation: true,
    };
    
    // Deviation: 50/1000 * 10000 = 500 bps (5%) < 1000 bps
    let result = DeviationValidator::check_deviation_exceeds_bounds(1000, 950, &bounds);
    assert_eq!(result, Ok(false));
}

#[test]
fn test_check_deviation_exactly_at_bounds() {
    let bounds = DeviationBounds {
        max_deviation_bps: 500, // 5%
        enforce_fallback_on_deviation: true,
    };
    
    // Deviation: 50/1000 * 10000 = 500 bps (5%) == 500 bps
    // Should NOT exceed (only exceed if > bounds)
    let result = DeviationValidator::check_deviation_exceeds_bounds(1000, 950, &bounds);
    assert_eq!(result, Ok(false));
}

#[test]
fn test_check_deviation_exceeds_bounds() {
    let bounds = DeviationBounds {
        max_deviation_bps: 400, // 4%
        enforce_fallback_on_deviation: true,
    };
    
    // Deviation: 50/1000 * 10000 = 500 bps (5%) > 400 bps
    let result = DeviationValidator::check_deviation_exceeds_bounds(1000, 950, &bounds);
    assert_eq!(result, Ok(true));
}

#[test]
fn test_check_deviation_one_bps_over_bound() {
    let bounds = DeviationBounds {
        max_deviation_bps: 499,
        enforce_fallback_on_deviation: true,
    };
    
    // Deviation: 50/1000 * 10000 = 500 bps > 499 bps (just over)
    let result = DeviationValidator::check_deviation_exceeds_bounds(1000, 950, &bounds);
    assert_eq!(result, Ok(true));
}

// ===== ERROR CONDITION TESTS =====

#[test]
fn test_check_deviation_zero_primary_price() {
    let bounds = DeviationBounds {
        max_deviation_bps: 500,
        enforce_fallback_on_deviation: true,
    };
    
    let result = DeviationValidator::check_deviation_exceeds_bounds(0, 1000, &bounds);
    assert_eq!(result, Err(Error::InvalidOraclePrice));
}

#[test]
fn test_check_deviation_zero_fallback_price() {
    let bounds = DeviationBounds {
        max_deviation_bps: 500,
        enforce_fallback_on_deviation: true,
    };
    
    let result = DeviationValidator::check_deviation_exceeds_bounds(1000, 0, &bounds);
    assert_eq!(result, Err(Error::InvalidOraclePrice));
}

#[test]
fn test_check_deviation_negative_primary_price() {
    let bounds = DeviationBounds {
        max_deviation_bps: 500,
        enforce_fallback_on_deviation: true,
    };
    
    let result = DeviationValidator::check_deviation_exceeds_bounds(-1000, 1000, &bounds);
    assert_eq!(result, Err(Error::InvalidOraclePrice));
}

#[test]
fn test_check_deviation_negative_fallback_price() {
    let bounds = DeviationBounds {
        max_deviation_bps: 500,
        enforce_fallback_on_deviation: true,
    };
    
    let result = DeviationValidator::check_deviation_exceeds_bounds(1000, -1000, &bounds);
    assert_eq!(result, Err(Error::InvalidOraclePrice));
}

#[test]
fn test_calculate_deviation_both_zero() {
    let result = DeviationValidator::calculate_deviation_bps(0, 0);
    assert_eq!(result, Err(Error::InvalidOraclePrice));
}

// ===== BOUNDARY TESTS =====

#[test]
fn test_calculate_deviation_minimum_valid_prices() {
    // Minimum valid price is 1
    let result = DeviationValidator::calculate_deviation_bps(1, 1);
    assert_eq!(result, Ok(0));
}

#[test]
fn test_calculate_deviation_i128_large_prices() {
    // Test with very large i128 values
    let price1: i128 = 9_223_372_036_854_775_000; // Large i128
    let price2: i128 = 9_223_372_036_854_774_500; // Slightly different
    
    let result = DeviationValidator::calculate_deviation_bps(price1, price2);
    assert!(result.is_ok());
    // Deviation should be very small
    let deviation = result.unwrap();
    assert!(deviation <= 1);
}

#[test]
fn test_calculate_deviation_asymmetric() {
    // Should give same result regardless of order
    let result1 = DeviationValidator::calculate_deviation_bps(1000, 900);
    let result2 = DeviationValidator::calculate_deviation_bps(900, 1000);
    
    assert_eq!(result1, result2);
}

// ===== ORACLE CONFIG TESTS =====

#[test]
fn test_oracle_config_with_deviation_bounds() {
    let env = Env::default();
    let provider = OracleProvider::reflector();
    let address = Address::generate(&env);
    let feed_id = String::from_str(&env, "BTC/USD");
    let comparison = String::from_str(&env, "gt");
    
    let bounds = DeviationBounds {
        max_deviation_bps: 500,
        enforce_fallback_on_deviation: true,
    };
    
    let config = OracleConfig::with_deviation_bounds(
        provider,
        address,
        feed_id,
        50_000_00,
        comparison,
        Some(bounds),
    );
    
    assert!(config.deviation_bounds.is_some());
    if let Some(bounds) = config.deviation_bounds {
        assert_eq!(bounds.max_deviation_bps, 500);
        assert_eq!(bounds.enforce_fallback_on_deviation, true);
    }
}

#[test]
fn test_oracle_config_without_deviation_bounds() {
    let env = Env::default();
    let provider = OracleProvider::reflector();
    let address = Address::generate(&env);
    let feed_id = String::from_str(&env, "BTC/USD");
    let comparison = String::from_str(&env, "gt");
    
    // Using the standard new() constructor (backward compatible)
    let config = OracleConfig::new(
        provider,
        address,
        feed_id,
        50_000_00,
        comparison,
    );
    
    assert!(config.deviation_bounds.is_none());
}

// ===== INTEGRATION TESTS =====

#[test]
fn test_deviation_bounds_creation_and_validation() {
    // Test the complete workflow: create bounds, validate, check deviation
    let bounds = DeviationBounds {
        max_deviation_bps: 1000,
        enforce_fallback_on_deviation: true,
    };
    
    // Validate bounds
    assert!(DeviationValidator::validate_bounds(&bounds).is_ok());
    
    // Check deviation within bounds
    assert_eq!(
        DeviationValidator::check_deviation_exceeds_bounds(10000, 9900, &bounds),
        Ok(false)
    );
    
    // Check deviation exceeding bounds
    assert_eq!(
        DeviationValidator::check_deviation_exceeds_bounds(10000, 8800, &bounds),
        Ok(true)
    );
}

#[test]
fn test_get_actual_deviation() {
    // Test the get_actual_deviation helper
    let result = DeviationValidator::get_actual_deviation(1000, 950);
    assert!(result.is_ok());
    
    let deviation = result.unwrap();
    assert!(deviation >= 520 && deviation <= 530); // Around 5.26%
}

#[test]
fn test_get_actual_deviation_zero_prices() {
    // Invalid prices should error
    let result = DeviationValidator::get_actual_deviation(0, 1000);
    assert_eq!(result, Err(Error::InvalidOraclePrice));
}

// ===== ENFORCEMENT TESTS =====

#[test]
fn test_enforcement_flag_true() {
    let bounds = DeviationBounds {
        max_deviation_bps: 500,
        enforce_fallback_on_deviation: true,
    };
    
    // With enforcement enabled, exceeding deviation should trigger fallback
    assert_eq!(
        DeviationValidator::check_deviation_exceeds_bounds(1000, 900, &bounds),
        Ok(true) // 1000 bps deviation > 500 bps bound
    );
}

#[test]
fn test_enforcement_flag_false() {
    let bounds = DeviationBounds {
        max_deviation_bps: 500,
        enforce_fallback_on_deviation: false,
    };
    
    // With enforcement disabled, even though deviation exceeds bounds,
    // the check_deviation_exceeds_bounds still returns true
    // (the enforcement flag is used by the caller to decide what to do)
    assert_eq!(
        DeviationValidator::check_deviation_exceeds_bounds(1000, 900, &bounds),
        Ok(true)
    );
}

// ===== DETERMINISM TESTS =====

#[test]
fn test_deviation_calculation_deterministic() {
    // Same inputs should always produce same output
    for _ in 0..10 {
        let result1 = DeviationValidator::calculate_deviation_bps(5000, 4750);
        let result2 = DeviationValidator::calculate_deviation_bps(5000, 4750);
        assert_eq!(result1, result2);
    }
}

#[test]
fn test_deviation_check_deterministic() {
    let bounds = DeviationBounds {
        max_deviation_bps: 1000,
        enforce_fallback_on_deviation: true,
    };
    
    // Same inputs should always produce same output
    for _ in 0..10 {
        let result1 = DeviationValidator::check_deviation_exceeds_bounds(10000, 9500, &bounds);
        let result2 = DeviationValidator::check_deviation_exceeds_bounds(10000, 9500, &bounds);
        assert_eq!(result1, result2);
    }
}
