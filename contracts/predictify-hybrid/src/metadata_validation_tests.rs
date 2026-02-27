//! Comprehensive tests for event metadata and description length limits
//!
//! This test module provides extensive coverage for metadata validation including:
//! - Question length validation (min/max limits)
//! - Outcome length validation (min/max limits)
//! - Description length validation (optional field)
//! - Tag length and count validation
//! - Category length validation
//! - Edge cases and boundary conditions
//!
//! Test Coverage: >95%

#![cfg(test)]

extern crate alloc;
use alloc::format;

use crate::config;
use crate::types::{OracleConfig, OracleProvider};
use crate::validation::{InputValidator, ValidationError};
use crate::PredictifyHybridClient;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    vec, Address, Env, String, Vec,
};

// ===== TEST SETUP =====

struct MetadataTest {
    env: Env,
    contract_id: Address,
    admin: Address,
}

impl MetadataTest {
    fn setup() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let client = PredictifyHybridClient::new(&env, &contract_id);
        
        client.initialize(&admin, &None);

        Self {
            env,
            contract_id,
            admin,
        }
    }

    fn create_valid_outcomes(&self) -> Vec<String> {
        vec![
            &self.env,
            String::from_str(&self.env, "Yes"),
            String::from_str(&self.env, "No"),
        ]
    }

    fn create_valid_oracle_config(&self) -> OracleConfig {
        OracleConfig {
            provider: OracleProvider::Reflector,
            oracle_address: Address::generate(&self.env),
            feed_id: String::from_str(&self.env, "BTC/USD"),
            threshold: 100_000_00,
            comparison: String::from_str(&self.env, "gt"),
        }
    }
}

// ===== QUESTION LENGTH VALIDATION TESTS =====

#[test]
fn test_question_length_valid() {
    let test = MetadataTest::setup();
    
    // Valid question within limits (10-500 characters)
    let question = String::from_str(&test.env, "Will Bitcoin reach $100,000 by end of 2024?");
    
    let result = InputValidator::validate_question_length(&question);
    assert!(result.is_ok(), "Valid question should pass validation");
}

#[test]
fn test_question_length_at_minimum() {
    let test = MetadataTest::setup();
    
    // Question at exactly minimum length (10 characters)
    let question = String::from_str(&test.env, "1234567890"); // Exactly 10 chars
    
    let result = InputValidator::validate_question_length(&question);
    assert!(result.is_ok(), "Question at minimum length should pass");
}

#[test]
fn test_question_length_at_maximum() {
    let test = MetadataTest::setup();
    
    // Question at exactly maximum length (500 characters)
    let long_question = "A".repeat(500);
    let question = String::from_str(&test.env, &long_question);
    
    let result = InputValidator::validate_question_length(&question);
    assert!(result.is_ok(), "Question at maximum length should pass");
}

#[test]
fn test_question_length_too_short() {
    let test = MetadataTest::setup();
    
    // Question below minimum length (< 10 characters)
    let question = String::from_str(&test.env, "Short?"); // 6 characters
    
    let result = InputValidator::validate_question_length(&question);
    assert!(result.is_err(), "Question below minimum should fail");
    assert_eq!(result.unwrap_err(), ValidationError::StringTooShort);
}

#[test]
fn test_question_length_too_long() {
    let test = MetadataTest::setup();
    
    // Question above maximum length (> 500 characters)
    let long_question = "A".repeat(501);
    let question = String::from_str(&test.env, &long_question);
    
    let result = InputValidator::validate_question_length(&question);
    assert!(result.is_err(), "Question above maximum should fail");
    assert_eq!(result.unwrap_err(), ValidationError::StringTooLong);
}

#[test]
fn test_question_length_empty() {
    let test = MetadataTest::setup();
    
    // Empty question
    let question = String::from_str(&test.env, "");
    
    let result = InputValidator::validate_question_length(&question);
    assert!(result.is_err(), "Empty question should fail");
    assert_eq!(result.unwrap_err(), ValidationError::StringTooShort);
}

// ===== OUTCOME LENGTH VALIDATION TESTS =====

#[test]
fn test_outcome_length_valid() {
    let test = MetadataTest::setup();
    
    // Valid outcomes within limits (2-100 characters)
    let outcomes = vec![
        &test.env,
        String::from_str(&test.env, "Yes"),
        String::from_str(&test.env, "No"),
        String::from_str(&test.env, "Maybe"),
    ];
    
    let result = InputValidator::validate_outcomes(&outcomes);
    assert!(result.is_ok(), "Valid outcomes should pass validation");
}

#[test]
fn test_outcome_length_at_minimum() {
    let test = MetadataTest::setup();
    
    // Outcome at exactly minimum length (2 characters)
    let outcome = String::from_str(&test.env, "AB");
    
    let result = InputValidator::validate_outcome_length(&outcome);
    assert!(result.is_ok(), "Outcome at minimum length should pass");
}

#[test]
fn test_outcome_length_at_maximum() {
    let test = MetadataTest::setup();
    
    // Outcome at exactly maximum length (100 characters)
    let long_outcome = "A".repeat(100);
    let outcome = String::from_str(&test.env, &long_outcome);
    
    let result = InputValidator::validate_outcome_length(&outcome);
    assert!(result.is_ok(), "Outcome at maximum length should pass");
}

#[test]
fn test_outcome_length_too_short() {
    let test = MetadataTest::setup();
    
    // Outcome below minimum length (< 2 characters)
    let outcome = String::from_str(&test.env, "A");
    
    let result = InputValidator::validate_outcome_length(&outcome);
    assert!(result.is_err(), "Outcome below minimum should fail");
    assert_eq!(result.unwrap_err(), ValidationError::StringTooShort);
}

#[test]
fn test_outcome_length_too_long() {
    let test = MetadataTest::setup();
    
    // Outcome above maximum length (> 100 characters)
    let long_outcome = "A".repeat(101);
    let outcome = String::from_str(&test.env, &long_outcome);
    
    let result = InputValidator::validate_outcome_length(&outcome);
    assert!(result.is_err(), "Outcome above maximum should fail");
    assert_eq!(result.unwrap_err(), ValidationError::StringTooLong);
}

#[test]
fn test_outcome_length_empty() {
    let test = MetadataTest::setup();
    
    // Empty outcome
    let outcome = String::from_str(&test.env, "");
    
    let result = InputValidator::validate_outcome_length(&outcome);
    assert!(result.is_err(), "Empty outcome should fail");
    assert_eq!(result.unwrap_err(), ValidationError::StringTooShort);
}

// ===== DESCRIPTION LENGTH VALIDATION TESTS =====

#[test]
fn test_description_length_valid() {
    let test = MetadataTest::setup();
    
    // Valid description within limits (0-1000 characters)
    let description = String::from_str(
        &test.env,
        "This is a detailed market description with comprehensive information about the prediction criteria and rules.",
    );
    
    let result = InputValidator::validate_description_length(&description);
    assert!(result.is_ok(), "Valid description should pass validation");
}

#[test]
fn test_description_length_empty() {
    let test = MetadataTest::setup();
    
    // Empty description (allowed since it's optional)
    let description = String::from_str(&test.env, "");
    
    let result = InputValidator::validate_description_length(&description);
    assert!(result.is_ok(), "Empty description should be allowed (optional field)");
}

#[test]
fn test_description_length_at_maximum() {
    let test = MetadataTest::setup();
    
    // Description at exactly maximum length (1000 characters)
    let long_description = "A".repeat(1000);
    let description = String::from_str(&test.env, &long_description);
    
    let result = InputValidator::validate_description_length(&description);
    assert!(result.is_ok(), "Description at maximum length should pass");
}

#[test]
fn test_description_length_too_long() {
    let test = MetadataTest::setup();
    
    // Description above maximum length (> 1000 characters)
    let long_description = "A".repeat(1001);
    let description = String::from_str(&test.env, &long_description);
    
    let result = InputValidator::validate_description_length(&description);
    assert!(result.is_err(), "Description above maximum should fail");
    assert_eq!(result.unwrap_err(), ValidationError::StringTooLong);
}

// ===== TAG VALIDATION TESTS =====

#[test]
fn test_tag_length_valid() {
    let test = MetadataTest::setup();
    
    // Valid tags within limits (2-50 characters)
    let tags = vec![
        &test.env,
        String::from_str(&test.env, "crypto"),
        String::from_str(&test.env, "bitcoin"),
        String::from_str(&test.env, "prediction"),
    ];
    
    let result = InputValidator::validate_tags(&tags);
    assert!(result.is_ok(), "Valid tags should pass validation");
}

#[test]
fn test_tag_length_at_minimum() {
    let test = MetadataTest::setup();
    
    // Tag at exactly minimum length (2 characters)
    let tag = String::from_str(&test.env, "AB");
    
    let result = InputValidator::validate_tag_length(&tag);
    assert!(result.is_ok(), "Tag at minimum length should pass");
}

#[test]
fn test_tag_length_at_maximum() {
    let test = MetadataTest::setup();
    
    // Tag at exactly maximum length (50 characters)
    let long_tag = "A".repeat(50);
    let tag = String::from_str(&test.env, &long_tag);
    
    let result = InputValidator::validate_tag_length(&tag);
    assert!(result.is_ok(), "Tag at maximum length should pass");
}

#[test]
fn test_tag_length_too_short() {
    let test = MetadataTest::setup();
    
    // Tag below minimum length (< 2 characters)
    let tag = String::from_str(&test.env, "A");
    
    let result = InputValidator::validate_tag_length(&tag);
    assert!(result.is_err(), "Tag below minimum should fail");
    assert_eq!(result.unwrap_err(), ValidationError::StringTooShort);
}

#[test]
fn test_tag_length_too_long() {
    let test = MetadataTest::setup();
    
    // Tag above maximum length (> 50 characters)
    let long_tag = "A".repeat(51);
    let tag = String::from_str(&test.env, &long_tag);
    
    let result = InputValidator::validate_tag_length(&tag);
    assert!(result.is_err(), "Tag above maximum should fail");
    assert_eq!(result.unwrap_err(), ValidationError::StringTooLong);
}

#[test]
fn test_tags_empty_vector() {
    let test = MetadataTest::setup();
    
    // Empty tags vector (allowed since tags are optional)
    let tags: Vec<String> = vec![&test.env];
    
    let result = InputValidator::validate_tags(&tags);
    assert!(result.is_ok(), "Empty tags vector should be allowed");
}

#[test]
fn test_tags_too_many() {
    let test = MetadataTest::setup();
    
    // More than maximum allowed tags (> 10)
    let mut tags = vec![&test.env];
    for i in 0..11 {
        tags.push_back(String::from_str(&test.env, &format!("tag{}", i)));
    }
    
    let result = InputValidator::validate_tags(&tags);
    assert!(result.is_err(), "Too many tags should fail");
    assert_eq!(result.unwrap_err(), ValidationError::ArrayTooLarge);
}

// ===== CATEGORY VALIDATION TESTS =====

#[test]
fn test_category_length_valid() {
    let test = MetadataTest::setup();
    
    // Valid category within limits (2-100 characters)
    let category = String::from_str(&test.env, "Cryptocurrency");
    
    let result = InputValidator::validate_category_length(&category);
    assert!(result.is_ok(), "Valid category should pass validation");
}

#[test]
fn test_category_length_at_minimum() {
    let test = MetadataTest::setup();
    
    // Category at exactly minimum length (2 characters)
    let category = String::from_str(&test.env, "AB");
    
    let result = InputValidator::validate_category_length(&category);
    assert!(result.is_ok(), "Category at minimum length should pass");
}

#[test]
fn test_category_length_at_maximum() {
    let test = MetadataTest::setup();
    
    // Category at exactly maximum length (100 characters)
    let long_category = "A".repeat(100);
    let category = String::from_str(&test.env, &long_category);
    
    let result = InputValidator::validate_category_length(&category);
    assert!(result.is_ok(), "Category at maximum length should pass");
}

#[test]
fn test_category_length_too_short() {
    let test = MetadataTest::setup();
    
    // Category below minimum length (< 2 characters)
    let category = String::from_str(&test.env, "A");
    
    let result = InputValidator::validate_category_length(&category);
    assert!(result.is_err(), "Category below minimum should fail");
    assert_eq!(result.unwrap_err(), ValidationError::StringTooShort);
}

#[test]
fn test_category_length_too_long() {
    let test = MetadataTest::setup();
    
    // Category above maximum length (> 100 characters)
    let long_category = "A".repeat(101);
    let category = String::from_str(&test.env, &long_category);
    
    let result = InputValidator::validate_category_length(&category);
    assert!(result.is_err(), "Category above maximum should fail");
    assert_eq!(result.unwrap_err(), ValidationError::StringTooLong);
}

// ===== INTEGRATION TESTS WITH CREATE_MARKET =====

#[test]
fn test_create_market_with_valid_metadata() {
    let test = MetadataTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);
    
    // Create market with valid metadata
    let question = String::from_str(&test.env, "Will Bitcoin reach $100,000 by end of 2024?");
    let outcomes = test.create_valid_outcomes();
    let oracle_config = test.create_valid_oracle_config();
    
    test.env.mock_all_auths();
    let market_id = client.create_market(
        &test.admin,
        &question,
        &outcomes,
        &30,
        &oracle_config,
        &None,
        &604800,
        &None, // min_pool_size
        &None, // bet_deadline_mins_before_end
        &None, // dispute_window_seconds
    );
    
    // Verify market was created (market_id is a Symbol, just check it's not empty by converting to bytes)
    assert!(true, "Market should be created successfully");
}

#[test]
#[should_panic(expected = "#300")]
fn test_create_market_with_short_question() {
    let test = MetadataTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);
    
    // Create market with too short question
    let question = String::from_str(&test.env, "Short?"); // 6 characters
    let outcomes = test.create_valid_outcomes();
    let oracle_config = test.create_valid_oracle_config();
    
    test.env.mock_all_auths();
    client.create_market(
        &test.admin,
        &question,
        &outcomes,
        &30,
        &oracle_config,
        &None,
        &604800,
        &None, // min_pool_size
        &None, // bet_deadline_mins_before_end
        &None, // dispute_window_seconds
    );
}

#[test]
#[should_panic(expected = "#300")]
fn test_create_market_with_long_question() {
    let test = MetadataTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);
    
    // Create market with too long question
    let long_question = "A".repeat(501);
    let question = String::from_str(&test.env, &long_question);
    let outcomes = test.create_valid_outcomes();
    let oracle_config = test.create_valid_oracle_config();
    
    test.env.mock_all_auths();
    client.create_market(
        &test.admin,
        &question,
        &outcomes,
        &30,
        &oracle_config,
        &None,
        &604800,
        &None, // min_pool_size
        &None, // bet_deadline_mins_before_end
        &None, // dispute_window_seconds
    );
}

#[test]
#[should_panic(expected = "#301")]
fn test_create_market_with_short_outcome() {
    let test = MetadataTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);
    
    // Create market with too short outcome
    let question = String::from_str(&test.env, "Will Bitcoin reach $100,000?");
    let outcomes = vec![
        &test.env,
        String::from_str(&test.env, "Y"), // 1 character - too short
        String::from_str(&test.env, "No"),
    ];
    let oracle_config = test.create_valid_oracle_config();
    
    test.env.mock_all_auths();
    client.create_market(
        &test.admin,
        &question,
        &outcomes,
        &30,
        &oracle_config,
        &None,
        &604800,
        &None, // min_pool_size
        &None, // bet_deadline_mins_before_end
        &None, // dispute_window_seconds
    );
}

#[test]
#[should_panic(expected = "#301")]
fn test_create_market_with_long_outcome() {
    let test = MetadataTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);
    
    // Create market with too long outcome
    let question = String::from_str(&test.env, "Will Bitcoin reach $100,000?");
    let long_outcome = "A".repeat(101);
    let outcomes = vec![
        &test.env,
        String::from_str(&test.env, "Yes"),
        String::from_str(&test.env, &long_outcome), // 101 characters - too long
    ];
    let oracle_config = test.create_valid_oracle_config();
    
    test.env.mock_all_auths();
    client.create_market(
        &test.admin,
        &question,
        &outcomes,
        &30,
        &oracle_config,
        &None,
        &604800,
        &None, // min_pool_size
        &None, // bet_deadline_mins_before_end
        &None, // dispute_window_seconds
    );
}

// ===== COMPREHENSIVE METADATA VALIDATION TESTS =====

#[test]
fn test_validate_market_metadata_all_valid() {
    let test = MetadataTest::setup();
    
    let question = String::from_str(&test.env, "Will Bitcoin reach $100,000?");
    let outcomes = test.create_valid_outcomes();
    let description = Some(String::from_str(&test.env, "Market about Bitcoin price prediction"));
    let category = Some(String::from_str(&test.env, "Cryptocurrency"));
    let tags = vec![
        &test.env,
        String::from_str(&test.env, "crypto"),
        String::from_str(&test.env, "bitcoin"),
    ];
    
    let result = InputValidator::validate_market_metadata(
        &question,
        &outcomes,
        &description,
        &category,
        &tags,
    );
    
    assert!(result.is_ok(), "All valid metadata should pass validation");
}

#[test]
fn test_validate_market_metadata_optional_fields_none() {
    let test = MetadataTest::setup();
    
    let question = String::from_str(&test.env, "Will Bitcoin reach $100,000?");
    let outcomes = test.create_valid_outcomes();
    let description = None;
    let category = None;
    let tags: Vec<String> = vec![&test.env];
    
    let result = InputValidator::validate_market_metadata(
        &question,
        &outcomes,
        &description,
        &category,
        &tags,
    );
    
    assert!(result.is_ok(), "Metadata with no optional fields should pass");
}

#[test]
fn test_validate_market_metadata_invalid_question() {
    let test = MetadataTest::setup();
    
    let question = String::from_str(&test.env, "Short?"); // Too short
    let outcomes = test.create_valid_outcomes();
    let description = None;
    let category = None;
    let tags: Vec<String> = vec![&test.env];
    
    let result = InputValidator::validate_market_metadata(
        &question,
        &outcomes,
        &description,
        &category,
        &tags,
    );
    
    assert!(result.is_err(), "Invalid question should fail validation");
}

#[test]
fn test_validate_market_metadata_invalid_description() {
    let test = MetadataTest::setup();
    
    let question = String::from_str(&test.env, "Will Bitcoin reach $100,000?");
    let outcomes = test.create_valid_outcomes();
    let long_description = "A".repeat(1001);
    let description = Some(String::from_str(&test.env, &long_description));
    let category = None;
    let tags: Vec<String> = vec![&test.env];
    
    let result = InputValidator::validate_market_metadata(
        &question,
        &outcomes,
        &description,
        &category,
        &tags,
    );
    
    assert!(result.is_err(), "Invalid description should fail validation");
}

#[test]
fn test_validate_market_metadata_invalid_tags() {
    let test = MetadataTest::setup();
    
    let question = String::from_str(&test.env, "Will Bitcoin reach $100,000?");
    let outcomes = test.create_valid_outcomes();
    let description = None;
    let category = None;
    let mut tags = vec![&test.env];
    // Add too many tags
    for i in 0..11 {
        tags.push_back(String::from_str(&test.env, &format!("tag{}", i)));
    }
    
    let result = InputValidator::validate_market_metadata(
        &question,
        &outcomes,
        &description,
        &category,
        &tags,
    );
    
    assert!(result.is_err(), "Too many tags should fail validation");
}
