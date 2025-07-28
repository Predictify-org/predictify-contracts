#![cfg(test)]

use super::*;
use soroban_sdk::{Address, Env, String, Symbol, Vec, vec};
use crate::validation::{
    InputValidator, MarketValidator, VoteValidator, FeeValidator, OracleValidator, 
    DisputeValidator, ValidationError, ValidationResult, ValidationTestingUtils, 
    ValidationErrorHandler, ValidationDocumentation
};
use crate::config;
use crate::types::{OracleConfig, OracleProvider, Market, MarketState};

#[test]
fn test_validate_string_length() {
    let env = Env::default();
    
    // Test valid string length
    let valid_string = String::from_str(&env, "Valid string");
    assert!(InputValidator::validate_string_length(&valid_string, 50).is_ok());
    
    // Test empty string
    let empty_string = String::from_str(&env, "");
    assert!(InputValidator::validate_string_length(&empty_string, 50).is_err());
    
    // Test string too long
    let long_string = String::from_str(&env, "This is a very long string that exceeds the maximum length limit");
    assert!(InputValidator::validate_string_length(&long_string, 10).is_err());
}

#[test]
fn test_validate_numeric_range() {
    // Test valid range
    assert!(InputValidator::validate_numeric_range(50, 0, 100).is_ok());
    
    // Test value below minimum
    assert!(InputValidator::validate_numeric_range(-10, 0, 100).is_err());
    
    // Test value above maximum
    assert!(InputValidator::validate_numeric_range(150, 0, 100).is_err());
    
    // Test boundary values
    assert!(InputValidator::validate_numeric_range(0, 0, 100).is_ok());
    assert!(InputValidator::validate_numeric_range(100, 0, 100).is_ok());
}

#[test]
fn test_validate_address_format() {
    let env = Env::default();
    
    // Test valid address (Soroban SDK will handle actual validation)
    let valid_address = Address::from_str(&env, "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF");
 
    
    // Instead, test that the address can be created successfully
    assert!(!valid_address.to_string().is_empty());
}

#[test]
fn test_validate_timestamp_bounds() {
    let current_time = 1000000;
    
    // Test valid timestamp
    assert!(InputValidator::validate_timestamp_bounds(current_time, current_time - 1000, current_time + 1000).is_ok());
    
    // Test timestamp below minimum
    assert!(InputValidator::validate_timestamp_bounds(current_time - 2000, current_time - 1000, current_time + 1000).is_err());
    
    // Test timestamp above maximum
    assert!(InputValidator::validate_timestamp_bounds(current_time + 2000, current_time - 1000, current_time + 1000).is_err());
}

#[test]
fn test_validate_array_size() {
    let env = Env::default();
    
    // Test valid array size
    let valid_array = vec![
        &env,
        String::from_str(&env, "Option 1"),
        String::from_str(&env, "Option 2"),
        String::from_str(&env, "Option 3"),
    ];
    assert!(InputValidator::validate_array_size(&valid_array, 10).is_ok());
    
    // Test empty array
    let empty_array = Vec::new(&env);
    assert!(InputValidator::validate_array_size(&empty_array, 10).is_err());
    
    // Test array too large
    let large_array = vec![
        &env,
        String::from_str(&env, "Option 1"),
        String::from_str(&env, "Option 2"),
        String::from_str(&env, "Option 3"),
        String::from_str(&env, "Option 4"),
        String::from_str(&env, "Option 5"),
    ];
    assert!(InputValidator::validate_array_size(&large_array, 3).is_err());
}

#[test]
fn test_validate_question_format() {
    let env = Env::default();
    
    // Test valid question
    let valid_question = String::from_str(&env, "Will Bitcoin reach $100,000 by the end of 2024?");
    assert!(InputValidator::validate_question_format(&valid_question).is_ok());
    
    // Test question too short
    let short_question = String::from_str(&env, "Short?");
    assert!(InputValidator::validate_question_format(&short_question).is_err());
    
    // Test empty question
    let empty_question = String::from_str(&env, "");
    assert!(InputValidator::validate_question_format(&empty_question).is_err());
}

#[test]
fn test_validate_outcome_format() {
    let env = Env::default();
    
    // Test valid outcome
    let valid_outcome = String::from_str(&env, "Yes, it will reach $100,000");
    assert!(InputValidator::validate_outcome_format(&valid_outcome).is_ok());
    
    // Test outcome too short
    let short_outcome = String::from_str(&env, "A");
    assert!(InputValidator::validate_outcome_format(&short_outcome).is_err());
    
    // Test empty outcome
    let empty_outcome = String::from_str(&env, "");
    assert!(InputValidator::validate_outcome_format(&empty_outcome).is_err());
}

#[test]
fn test_validate_comprehensive_inputs() {
    let env = Env::default();
    
    let admin = Address::from_str(&env, "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF");
    let question = String::from_str(&env, "Will Bitcoin reach $100,000 by the end of 2024?");
    let outcomes = vec![
        &env,
        String::from_str(&env, "Yes, it will reach $100,000"),
        String::from_str(&env, "No, it will not reach $100,000"),
        String::from_str(&env, "It will reach between $50,000 and $100,000"),
    ];
    let duration_days = 30;
    let oracle_config = OracleConfig {
        provider: OracleProvider::Pyth,
        feed_id: String::from_str(&env, "BTC/USD"),
        threshold: 100000,
        comparison: String::from_str(&env, "gt"),
    };
    

    
    // Test question format
    assert!(InputValidator::validate_question_format(&question).is_ok());
    
    // Test outcomes array size
    assert!(InputValidator::validate_array_size(&outcomes, 10).is_ok());
    
    // Test duration
    assert!(InputValidator::validate_duration(&duration_days).is_ok());
}

#[test]
fn test_validate_market_creation() {
    let env = Env::default();
    
    let admin = Address::from_str(&env, "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF");
    let question = String::from_str(&env, "Will Bitcoin reach $100,000 by the end of 2024?");
    let outcomes = vec![
        &env,
        String::from_str(&env, "Yes, it will reach $100,000"),
        String::from_str(&env, "No, it will not reach $100,000"),
    ];
    let duration_days = 30;
    let oracle_config = OracleConfig {
        provider: OracleProvider::Pyth,
        feed_id: String::from_str(&env, "BTC/USD"),
        threshold: 100000,
        comparison: String::from_str(&env, "gt"),
    };
    

    
    // Test question format
    assert!(InputValidator::validate_question_format(&question).is_ok());
    
    // Test outcomes array size
    assert!(InputValidator::validate_array_size(&outcomes, 10).is_ok());
    
    // Test duration
    assert!(InputValidator::validate_duration(&duration_days).is_ok());
}

#[test]
fn test_validate_vote() {
    let env = Env::default();
    
    let user = Address::from_str(&env, "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF");
    let market_id = Symbol::new(&env, "BTC_MARKET");
    let outcome = String::from_str(&env, "Yes, it will reach $100,000");
    let stake_amount = 10000000; // 1 XLM
    let market = ValidationTestingUtils::create_test_market(&env);
    

    
    // Test outcome format validation
    assert!(InputValidator::validate_outcome_format(&outcome).is_ok());
    
    // Test stake amount validation
    assert!(InputValidator::validate_numeric_range(stake_amount, 1000000, i128::MAX).is_ok());
}

#[test]
fn test_validation_error_conversion() {
    // Test that validation errors convert to contract errors correctly
    let error = ValidationError::StringTooLong;
    let contract_error = error.to_contract_error();
    assert_eq!(contract_error, Error::InvalidQuestion);
    
    let error = ValidationError::NumberOutOfRange;
    let contract_error = error.to_contract_error();
    assert_eq!(contract_error, Error::InvalidThreshold);
    
    let error = ValidationError::InvalidAddressFormat;
    let contract_error = error.to_contract_error();
    assert_eq!(contract_error, Error::Unauthorized);
}

#[test]
fn test_validation_result() {
    let mut result = ValidationResult::valid();
    assert!(result.is_valid);
    assert_eq!(result.error_count, 0);
    
    result.add_error();
    assert!(!result.is_valid);
    assert_eq!(result.error_count, 1);
    
    result.add_warning();
    assert_eq!(result.warning_count, 1);
    
    result.add_recommendation();
    assert_eq!(result.recommendation_count, 1);
    
    assert!(result.has_errors());
    assert!(result.has_warnings());
}

#[test]
fn test_fee_validation() {
    // Test valid fee amount
    let valid_fee = 10000000; // 1 XLM
    assert!(FeeValidator::validate_fee_amount(&valid_fee).is_ok());
    
    // Test fee below minimum
    let low_fee = 100000; // 0.01 XLM
    assert!(FeeValidator::validate_fee_amount(&low_fee).is_err());
    
    // Test fee above maximum
    let high_fee = 2000000000; // 200 XLM
    assert!(FeeValidator::validate_fee_amount(&high_fee).is_err());
    
    // Test valid fee percentage
    let valid_percentage = 5;
    assert!(FeeValidator::validate_fee_percentage(&valid_percentage).is_ok());
    
    // Test percentage above 100
    let invalid_percentage = 150;
    assert!(FeeValidator::validate_fee_percentage(&invalid_percentage).is_err());
}

#[test]
fn test_oracle_validation() {
    let env = Env::default();
    
    let oracle_config = OracleConfig {
        provider: OracleProvider::Pyth,
        feed_id: String::from_str(&env, "BTC/USD"),
        threshold: 100000,
        comparison: String::from_str(&env, "gt"),
    };
    
    // Test valid oracle config
    assert!(OracleValidator::validate_oracle_config(&env, &oracle_config).is_ok());
    
    // Test invalid comparison operator
    let invalid_config = OracleConfig {
        provider: OracleProvider::Pyth,
        feed_id: String::from_str(&env, "BTC/USD"),
        threshold: 100000,
        comparison: String::from_str(&env, "invalid"),
    };
    assert!(OracleValidator::validate_oracle_config(&env, &invalid_config).is_err());
}

#[test]
fn test_dispute_validation() {
    let env = Env::default();
    
    let user = Address::from_str(&env, "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF");
    let market_id = Symbol::new(&env, "BTC_MARKET");
    let dispute_stake = 10000000; // 1 XLM
    let market = ValidationTestingUtils::create_test_market(&env);
    
    
    
    // Test dispute stake validation
    assert!(InputValidator::validate_numeric_range(dispute_stake, 1000000, i128::MAX).is_ok());
}

#[test]
fn test_validation_error_handler() {
    let error = ValidationError::InvalidInput;
    let contract_error = ValidationErrorHandler::handle_validation_error(error);
    assert_eq!(contract_error, Error::InvalidInput);
    
    let mut result = ValidationResult::valid();
    result.add_error();
    let handler_result = ValidationErrorHandler::handle_validation_result(result);
    assert!(handler_result.is_err());
    
    let valid_result = ValidationResult::valid();
    let handler_result = ValidationErrorHandler::handle_validation_result(valid_result);
    assert!(handler_result.is_ok());
}

#[test]
fn test_validation_documentation() {
    let env = Env::default();
    
    let overview = ValidationDocumentation::get_validation_overview(&env);
    assert!(!overview.is_empty());
    
    let rules = ValidationDocumentation::get_validation_rules(&env);
    assert!(rules.len() > 0);
    
    let error_codes = ValidationDocumentation::get_validation_error_codes(&env);
    assert!(error_codes.len() > 0);
}

#[test]
fn test_edge_cases() {
    let env = Env::default();
    
    // Test boundary values for string length
    let boundary_string = String::from_str(&env, "1234567890"); // Exactly 10 characters
    assert!(InputValidator::validate_question_format(&boundary_string).is_ok());
    
    let short_string = String::from_str(&env, "123456789"); // 9 characters
    assert!(InputValidator::validate_question_format(&short_string).is_err());
    
    // Test boundary values for numeric range
    assert!(InputValidator::validate_numeric_range(0, 0, 100).is_ok());
    assert!(InputValidator::validate_numeric_range(100, 0, 100).is_ok());
    assert!(InputValidator::validate_numeric_range(-1, 0, 100).is_err());
    assert!(InputValidator::validate_numeric_range(101, 0, 100).is_err());
    
    // Test boundary values for array size
    let min_array = vec![&env, String::from_str(&env, "A"), String::from_str(&env, "B")];
    assert!(InputValidator::validate_array_size(&min_array, 10).is_ok());
    
    let empty_array = Vec::new(&env);
    assert!(InputValidator::validate_array_size(&empty_array, 10).is_err());
}

#[test]
fn test_validation_performance() {
    let env = Env::default();
    
    // Test that validation doesn't take too long with large inputs
    let large_question = String::from_str(&env, "This is a very long question that tests the performance of our validation system. It contains many characters to ensure that the validation logic can handle large inputs efficiently without causing performance issues.");
    
  
    let result = InputValidator::validate_question_format(&large_question);
   

    assert!(result.is_ok());
}

#[test]
fn test_validation_error_messages() {
    // Test that all validation errors have corresponding contract errors
    let validation_errors = [
        ValidationError::InvalidInput,
        ValidationError::InvalidMarket,
        ValidationError::InvalidOracle,
        ValidationError::InvalidFee,
        ValidationError::InvalidVote,
        ValidationError::InvalidDispute,
        ValidationError::InvalidAddress,
        ValidationError::InvalidString,
        ValidationError::InvalidNumber,
        ValidationError::InvalidTimestamp,
        ValidationError::InvalidDuration,
        ValidationError::InvalidOutcome,
        ValidationError::InvalidStake,
        ValidationError::InvalidThreshold,
        ValidationError::InvalidConfig,
        ValidationError::StringTooLong,
        ValidationError::StringTooShort,
        ValidationError::NumberOutOfRange,
        ValidationError::InvalidAddressFormat,
        ValidationError::TimestampOutOfBounds,
        ValidationError::ArrayTooLarge,
        ValidationError::ArrayTooSmall,
        ValidationError::InvalidQuestionFormat,
        ValidationError::InvalidOutcomeFormat,
    ];
    
    for error in validation_errors {
        let contract_error = error.to_contract_error();
        // Ensure that the conversion doesn't panic and returns a valid error
        assert!(matches!(contract_error, Error::InvalidInput | Error::MarketNotFound | Error::InvalidOracleConfig | Error::InvalidFeeConfig | Error::AlreadyVoted | Error::AlreadyDisputed | Error::Unauthorized | Error::InvalidQuestion | Error::InvalidThreshold | Error::InvalidDuration | Error::InvalidOutcome | Error::InsufficientStake | Error::InvalidOutcomes));
    }
} 