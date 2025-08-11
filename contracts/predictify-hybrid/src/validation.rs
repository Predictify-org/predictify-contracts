#![allow(unused_variables)]

extern crate alloc;

use crate::{
    config,
    errors::Error,
    types::{Market, OracleConfig, OracleProvider},
};
// use alloc::string::ToString; // Removed to fix Display/ToString trait errors
use soroban_sdk::{contracttype, vec, Address, Env, IntoVal, Map, String, Symbol, Vec};

// ===== VALIDATION ERROR TYPES =====

/// Comprehensive validation error types for prediction market operations.
///
/// This enum defines all possible validation failures that can occur within
/// the Predictify Hybrid smart contract. Each error type corresponds to a
/// specific validation domain and provides detailed error categorization
/// for debugging and user feedback.
///
/// # Error Categories
///
/// **Input Validation Errors:**
/// - `InvalidInput`: General input validation failures
/// - `InvalidAddress`: Address format or permission errors
/// - `InvalidString`: String length, format, or content errors
/// - `InvalidNumber`: Numeric range or format errors
/// - `InvalidTimestamp`: Time-related validation errors
/// - `InvalidDuration`: Duration range or format errors
///
/// **Market Validation Errors:**
/// - `InvalidMarket`: Market state or configuration errors
/// - `InvalidOutcome`: Market outcome validation errors
/// - `InvalidStake`: Stake amount or permission errors
/// - `InvalidThreshold`: Threshold value errors
///
/// **System Validation Errors:**
/// - `InvalidOracle`: Oracle configuration or data errors
/// - `InvalidFee`: Fee structure or amount errors
/// - `InvalidVote`: Voting permission or state errors
/// - `InvalidDispute`: Dispute creation or state errors
/// - `InvalidConfig`: Configuration parameter errors
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Address, String};
/// # use predictify_hybrid::validation::{ValidationError, InputValidator};
/// # let env = Env::default();
///
/// // Handle address validation error
/// let user_address = Address::generate(&env);
/// match InputValidator::validate_address(&env, &user_address) {
///     Ok(()) => println!("Address is valid"),
///     Err(ValidationError::InvalidAddress) => {
///         println!("Address validation failed");
///         // Handle address error
///     }
///     Err(other_error) => {
///         println!("Unexpected validation error: {:?}", other_error);
///     }
/// }
///
/// // Handle string validation error
/// let market_question = String::from_str(&env, ""); // Empty string
/// match InputValidator::validate_string(&env, &market_question, 10, 500) {
///     Ok(()) => println!("Question is valid"),
///     Err(ValidationError::InvalidString) => {
///         println!("Question validation failed - too short or too long");
///         // Handle string length error
///     }
///     Err(other_error) => {
///         println!("Unexpected validation error: {:?}", other_error);
///     }
/// }
///
/// // Handle number validation error
/// let stake_amount = -1000i128; // Negative stake
/// match InputValidator::validate_positive_number(&stake_amount) {
///     Ok(()) => println!("Stake amount is valid"),
///     Err(ValidationError::InvalidNumber) => {
///         println!("Stake must be positive");
///         // Handle negative number error
///     }
///     Err(other_error) => {
///         println!("Unexpected validation error: {:?}", other_error);
///     }
/// }
/// ```
///
/// # Error Conversion
///
/// Convert validation errors to contract errors:
/// ```rust
/// # use predictify_hybrid::validation::ValidationError;
/// # use predictify_hybrid::errors::Error;
///
/// // Convert validation error to contract error
/// let validation_error = ValidationError::InvalidStake;
/// let contract_error = validation_error.to_contract_error();
///
/// match contract_error {
///     Error::InsufficientStake => {
///         println!("Converted to insufficient stake error");
///         // Handle insufficient stake
///     }
///     _ => {
///         println!("Unexpected contract error conversion");
///     }
/// }
///
/// // Handle multiple validation errors
/// let validation_errors = vec![
///     ValidationError::InvalidAddress,
///     ValidationError::InvalidString,
///     ValidationError::InvalidNumber,
/// ];
///
/// for error in validation_errors {
///     let contract_error = error.to_contract_error();
///     println!("Validation error {:?} -> Contract error {:?}", error, contract_error);
/// }
/// ```
///
/// # Error Handling Patterns
///
/// Common error handling patterns:
/// ```rust
/// # use soroban_sdk::{Env, Address, String};
/// # use predictify_hybrid::validation::{ValidationError, MarketValidator};
/// # use predictify_hybrid::types::{OracleConfig, OracleProvider};
/// # let env = Env::default();
///
/// // Pattern 1: Early return on validation error
/// fn validate_market_creation_params(
///     env: &Env,
///     admin: &Address,
///     question: &String,
/// ) -> Result<(), ValidationError> {
///     // Validate admin address
///     if let Err(e) = InputValidator::validate_address(env, admin) {
///         return Err(e);
///     }
///     
///     // Validate question string
///     if let Err(e) = InputValidator::validate_string(env, question, 10, 500) {
///         return Err(e);
///     }
///     
///     Ok(())
/// }
///
/// // Pattern 2: Collect all validation errors
/// fn validate_all_market_params(
///     env: &Env,
///     admin: &Address,
///     question: &String,
///     duration: &u32,
/// ) -> Vec<ValidationError> {
///     let mut errors = Vec::new();
///     
///     if InputValidator::validate_address(env, admin).is_err() {
///         errors.push(ValidationError::InvalidAddress);
///     }
///     
///     if InputValidator::validate_string(env, question, 10, 500).is_err() {
///         errors.push(ValidationError::InvalidString);
///     }
///     
///     if InputValidator::validate_duration(duration).is_err() {
///         errors.push(ValidationError::InvalidDuration);
///     }
///     
///     errors
/// }
///
/// // Pattern 3: Convert and propagate errors
/// fn create_market_with_validation(
///     env: &Env,
///     admin: &Address,
///     question: &String,
/// ) -> Result<(), Error> {
///     match validate_market_creation_params(env, admin, question) {
///         Ok(()) => {
///             // Proceed with market creation
///             println!("All validations passed, creating market");
///             Ok(())
///         }
///         Err(validation_error) => {
///             // Convert validation error to contract error
///             Err(validation_error.to_contract_error())
///         }
///     }
/// }
/// ```
///
/// # Integration Points
///
/// ValidationError integrates with:
/// - **Contract Error System**: Convert to contract errors for user feedback
/// - **Input Validation**: Categorize input validation failures
/// - **Market Validation**: Handle market-specific validation errors
/// - **Oracle Validation**: Manage oracle configuration errors
/// - **Fee Validation**: Handle fee structure validation errors
/// - **Event System**: Log validation errors for debugging
/// - **Admin System**: Validate administrative operations
///
/// # Error Recovery
///
/// Strategies for error recovery:
/// - **Input Sanitization**: Clean and retry with sanitized input
/// - **Default Values**: Use safe defaults for optional parameters
/// - **User Guidance**: Provide specific error messages for correction
/// - **Graceful Degradation**: Continue with reduced functionality
/// - **Retry Logic**: Implement retry mechanisms for transient errors
#[contracttype]
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    InvalidInput,
    InvalidMarket,
    InvalidOracle,
    InvalidFee,
    InvalidVote,
    InvalidDispute,
    InvalidAddress,
    InvalidString,
    InvalidNumber,
    InvalidTimestamp,
    InvalidDuration,
    InvalidOutcome,
    InvalidStake,
    InvalidThreshold,
    InvalidConfig,
    StringTooLong,
    StringTooShort,
    NumberOutOfRange,
    InvalidAddressFormat,
    TimestampOutOfBounds,
    ArrayTooLarge,
    ArrayTooSmall,
    InvalidQuestionFormat,
    InvalidOutcomeFormat,
}

impl ValidationError {
    /// Convert validation error to contract error
    pub fn to_contract_error(&self) -> Error {
        match self {
            ValidationError::InvalidInput => Error::InvalidInput,
            ValidationError::InvalidMarket => Error::MarketNotFound,
            ValidationError::InvalidOracle => Error::InvalidOracleConfig,
            ValidationError::InvalidFee => Error::InvalidFeeConfig,
            ValidationError::InvalidVote => Error::AlreadyVoted,
            ValidationError::InvalidDispute => Error::AlreadyDisputed,
            ValidationError::InvalidAddress => Error::Unauthorized,
            ValidationError::InvalidString => Error::InvalidQuestion,
            ValidationError::InvalidNumber => Error::InvalidThreshold,
            ValidationError::InvalidTimestamp => Error::InvalidDuration,
            ValidationError::InvalidDuration => Error::InvalidDuration,
            ValidationError::InvalidOutcome => Error::InvalidOutcome,
            ValidationError::InvalidStake => Error::InsufficientStake,
            ValidationError::InvalidThreshold => Error::InvalidThreshold,
            ValidationError::InvalidConfig => Error::InvalidOracleConfig,
            ValidationError::StringTooLong => Error::InvalidQuestion,
            ValidationError::StringTooShort => Error::InvalidQuestion,
            ValidationError::NumberOutOfRange => Error::InvalidThreshold,
            ValidationError::InvalidAddressFormat => Error::Unauthorized,
            ValidationError::TimestampOutOfBounds => Error::InvalidDuration,
            ValidationError::ArrayTooLarge => Error::InvalidOutcomes,
            ValidationError::ArrayTooSmall => Error::InvalidOutcomes,
            ValidationError::InvalidQuestionFormat => Error::InvalidQuestion,
            ValidationError::InvalidOutcomeFormat => Error::InvalidOutcome,
        }
    }
}

// ===== VALIDATION RESULT TYPES =====

/// Comprehensive validation result with detailed information and metrics.
///
/// This structure provides detailed information about validation operations,
/// including success/failure status, error counts, warnings, and recommendations.
/// It enables comprehensive validation reporting and helps developers understand
/// validation outcomes in detail.
///
/// # Core Fields
///
/// **Status Information:**
/// - `is_valid`: Overall validation success status
/// - `error_count`: Number of validation errors encountered
/// - `warning_count`: Number of validation warnings generated
/// - `recommendation_count`: Number of improvement recommendations
///
/// # Example Usage
///
/// ```rust
/// # use predictify_hybrid::validation::ValidationResult;
///
/// // Create a valid result
/// let mut result = ValidationResult::valid();
/// assert!(result.is_valid);
/// assert_eq!(result.error_count, 0);
/// assert_eq!(result.warning_count, 0);
///
/// // Add warnings and recommendations
/// result.add_warning();
/// result.add_recommendation();
///
/// assert!(result.is_valid); // Still valid with warnings
/// assert_eq!(result.warning_count, 1);
/// assert_eq!(result.recommendation_count, 1);
///
/// // Add an error
/// result.add_error();
/// assert!(!result.is_valid); // Now invalid
/// assert_eq!(result.error_count, 1);
/// assert!(result.has_errors());
/// assert!(result.has_warnings());
/// ```
///
/// # Validation Workflow
///
/// Typical validation workflow using ValidationResult:
/// ```rust
/// # use soroban_sdk::{Env, Address, String, Vec};
/// # use predictify_hybrid::validation::{ValidationResult, InputValidator};
/// # let env = Env::default();
///
/// // Start with valid result
/// let mut validation_result = ValidationResult::valid();
///
/// // Validate multiple parameters
/// let admin = Address::generate(&env);
/// let question = String::from_str(&env, "Will BTC reach $100k?");
/// let duration = 30u32;
///
/// // Validate admin address
/// if InputValidator::validate_address(&env, &admin).is_err() {
///     validation_result.add_error();
///     println!("Admin address validation failed");
/// }
///
/// // Validate question length
/// if InputValidator::validate_string(&env, &question, 10, 500).is_err() {
///     validation_result.add_error();
///     println!("Question validation failed");
/// } else if question.len() < 20 {
///     validation_result.add_warning();
///     println!("Question is quite short, consider adding more detail");
/// }
///
/// // Validate duration
/// if InputValidator::validate_duration(&duration).is_err() {
///     validation_result.add_error();
///     println!("Duration validation failed");
/// } else if duration < 7 {
///     validation_result.add_recommendation();
///     println!("Consider longer duration for better market participation");
/// }
///
/// // Check final result
/// if validation_result.is_valid {
///     println!("✓ All validations passed");
///     if validation_result.has_warnings() {
///         println!("⚠ {} warnings generated", validation_result.warning_count);
///     }
///     if validation_result.recommendation_count > 0 {
///         println!("💡 {} recommendations available", validation_result.recommendation_count);
///     }
/// } else {
///     println!("✗ Validation failed with {} errors", validation_result.error_count);
/// }
/// ```
///
/// # Batch Validation
///
/// Handle multiple validation operations:
/// ```rust
/// # use soroban_sdk::{Env, Address, String, Vec};
/// # use predictify_hybrid::validation::{ValidationResult, InputValidator};
/// # let env = Env::default();
///
/// // Validate multiple market parameters
/// fn validate_market_batch(
///     env: &Env,
///     admins: &Vec<Address>,
///     questions: &Vec<String>,
///     durations: &Vec<u32>,
/// ) -> ValidationResult {
///     let mut result = ValidationResult::valid();
///     
///     // Validate all admins
///     for (i, admin) in admins.iter().enumerate() {
///         if InputValidator::validate_address(env, admin).is_err() {
///             result.add_error();
///             println!("Admin {} validation failed", i + 1);
///         }
///     }
///     
///     // Validate all questions
///     for (i, question) in questions.iter().enumerate() {
///         if InputValidator::validate_string(env, question, 10, 500).is_err() {
///             result.add_error();
///             println!("Question {} validation failed", i + 1);
///         } else if question.len() < 20 {
///             result.add_warning();
///             println!("Question {} is quite short", i + 1);
///         }
///     }
///     
///     // Validate all durations
///     for (i, duration) in durations.iter().enumerate() {
///         if InputValidator::validate_duration(duration).is_err() {
///             result.add_error();
///             println!("Duration {} validation failed", i + 1);
///         } else if *duration < 7 {
///             result.add_recommendation();
///             println!("Duration {} could be longer", i + 1);
///         }
///     }
///     
///     result
/// }
///
/// // Use batch validation
/// let admins = vec![
///     Address::generate(&env),
///     Address::generate(&env),
/// ];
/// let questions = vec![
///     String::from_str(&env, "Will BTC reach $100k?"),
///     String::from_str(&env, "Will ETH reach $5k?"),
/// ];
/// let durations = vec![30u32, 60u32];
///
/// let batch_result = validate_market_batch(&env, &admins, &questions, &durations);
///
/// println!("Batch validation completed:");
/// println!("  Valid: {}", batch_result.is_valid);
/// println!("  Errors: {}", batch_result.error_count);
/// println!("  Warnings: {}", batch_result.warning_count);
/// println!("  Recommendations: {}", batch_result.recommendation_count);
/// ```
///
/// # Validation Reporting
///
/// Generate detailed validation reports:
/// ```rust
/// # use predictify_hybrid::validation::ValidationResult;
///
/// // Generate validation summary
/// fn generate_validation_report(result: &ValidationResult) -> String {
///     let mut report = String::new();
///     
///     if result.is_valid {
///         report.push_str("✓ VALIDATION PASSED\n");
///     } else {
///         report.push_str("✗ VALIDATION FAILED\n");
///     }
///     
///     if result.error_count > 0 {
///         report.push_str(&format!("Errors: {}\n", result.error_count));
///     }
///     
///     if result.warning_count > 0 {
///         report.push_str(&format!("Warnings: {}\n", result.warning_count));
///     }
///     
///     if result.recommendation_count > 0 {
///         report.push_str(&format!("Recommendations: {}\n", result.recommendation_count));
///     }
///     
///     report
/// }
///
/// // Example usage
/// let mut result = ValidationResult::valid();
/// result.add_warning();
/// result.add_recommendation();
///
/// let report = generate_validation_report(&result);
/// println!("{}", report);
/// // Output:
/// // ✓ VALIDATION PASSED
/// // Warnings: 1
/// // Recommendations: 1
/// ```
///
/// # Integration Points
///
/// ValidationResult integrates with:
/// - **Input Validation**: Aggregate input validation results
/// - **Market Validation**: Collect market validation outcomes
/// - **Oracle Validation**: Track oracle validation status
/// - **Fee Validation**: Monitor fee validation results
/// - **Admin System**: Validate administrative operations
/// - **Event System**: Log validation outcomes
/// - **User Interface**: Provide detailed validation feedback
///
/// # Best Practices
///
/// Recommendations for using ValidationResult:
/// - **Comprehensive Checking**: Always check both errors and warnings
/// - **User Feedback**: Provide specific feedback based on validation results
/// - **Logging**: Log validation results for debugging and monitoring
/// - **Recovery**: Implement recovery strategies for validation failures
/// - **Performance**: Use batch validation for multiple operations
/// - **Documentation**: Document validation rules and expected outcomes
#[contracttype]
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub error_count: u32,
    pub warning_count: u32,
    pub recommendation_count: u32,
}

impl ValidationResult {
    /// Create a valid validation result
    pub fn valid() -> Self {
        Self {
            is_valid: true,
            error_count: 0,
            warning_count: 0,
            recommendation_count: 0,
        }
    }

    /// Create an invalid validation result
    pub fn invalid() -> Self {
        Self {
            is_valid: false,
            error_count: 1,
            warning_count: 0,
            recommendation_count: 0,
        }
    }

    /// Add an error to the validation result
    pub fn add_error(&mut self) {
        self.is_valid = false;
        self.error_count += 1;
    }

    /// Add a warning to the validation result
    pub fn add_warning(&mut self) {
        self.warning_count += 1;
    }

    /// Add a recommendation to the validation result
    pub fn add_recommendation(&mut self) {
        self.recommendation_count += 1;
    }

    /// Check if validation result has errors
    pub fn has_errors(&self) -> bool {
        self.error_count > 0
    }

    /// Check if validation result has warnings
    pub fn has_warnings(&self) -> bool {
        self.warning_count > 0
    }
}

// ===== COMPREHENSIVE INPUT VALIDATION =====

/// Comprehensive input validation utilities
/// Comprehensive input validation utilities for prediction market operations.
///
/// This utility class provides essential input validation operations for prediction markets,
/// including address validation, string validation, numeric validation, timestamp validation,
/// and duration validation. All validation functions return detailed error information
/// and are optimized for blockchain execution.
///
/// # Core Functionality
///
/// **Address Validation:**
/// - Validate address format and structure
/// - Check address permissions and accessibility
/// - Handle Soroban SDK address constraints
///
/// **String Validation:**
/// - Validate string length within specified bounds
/// - Check string content for validity
/// - Handle Unicode and special character constraints
///
/// **Numeric Validation:**
/// - Validate number ranges and bounds
/// - Check for positive numbers
/// - Handle integer overflow and underflow
///
/// **Timestamp Validation:**
/// - Validate future timestamps for market deadlines
/// - Check timestamp format and range
/// - Handle blockchain time constraints
///
/// **Duration Validation:**
/// - Validate duration ranges for markets
/// - Check minimum and maximum duration limits
/// - Handle duration format and conversion
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Address, String};
/// # use predictify_hybrid::validation::{InputValidator, ValidationError};
/// # let env = Env::default();
///
/// // Validate address
/// let user_address = Address::generate(&env);
/// match InputValidator::validate_address(&env, &user_address) {
///     Ok(()) => println!("Address is valid"),
///     Err(ValidationError::InvalidAddress) => {
///         println!("Invalid address format");
///     }
///     Err(e) => println!("Validation error: {:?}", e),
/// }
///
/// // Validate string length
/// let market_question = String::from_str(&env, "Will Bitcoin reach $100,000?");
/// match InputValidator::validate_string(&env, &market_question, 10, 500) {
///     Ok(()) => println!("Question length is valid"),
///     Err(ValidationError::InvalidString) => {
///         println!("Question is too short or too long");
///     }
///     Err(e) => println!("Validation error: {:?}", e),
/// }
///
/// // Validate positive number
/// let stake_amount = 1000000i128; // 1 XLM in stroops
/// match InputValidator::validate_positive_number(&stake_amount) {
///     Ok(()) => println!("Stake amount is positive"),
///     Err(ValidationError::InvalidNumber) => {
///         println!("Stake amount must be positive");
///     }
///     Err(e) => println!("Validation error: {:?}", e),
/// }
///
/// // Validate number range
/// let threshold = 50000000i128; // $50 threshold
/// let min_threshold = 1000000i128; // $1 minimum
/// let max_threshold = 1000000000i128; // $1000 maximum
/// match InputValidator::validate_number_range(&threshold, &min_threshold, &max_threshold) {
///     Ok(()) => println!("Threshold is within valid range"),
///     Err(ValidationError::InvalidNumber) => {
///         println!("Threshold is outside valid range");
///     }
///     Err(e) => println!("Validation error: {:?}", e),
/// }
///
/// // Validate future timestamp
/// let market_deadline = env.ledger().timestamp() + 86400; // 1 day from now
/// match InputValidator::validate_future_timestamp(&env, &market_deadline) {
///     Ok(()) => println!("Deadline is in the future"),
///     Err(ValidationError::InvalidTimestamp) => {
///         println!("Deadline must be in the future");
///     }
///     Err(e) => println!("Validation error: {:?}", e),
/// }
///
/// // Validate duration
/// let market_duration = 30u32; // 30 days
/// match InputValidator::validate_duration(&market_duration) {
///     Ok(()) => println!("Duration is valid"),
///     Err(ValidationError::InvalidDuration) => {
///         println!("Duration is outside valid range");
///     }
///     Err(e) => println!("Validation error: {:?}", e),
/// }
/// ```
///
/// # Address Validation
///
/// Validate addresses for various use cases:
/// ```rust
/// # use soroban_sdk::{Env, Address};
/// # use predictify_hybrid::validation::{InputValidator, ValidationError};
/// # let env = Env::default();
///
/// // Validate market admin address
/// let admin_address = Address::generate(&env);
/// match InputValidator::validate_address(&env, &admin_address) {
///     Ok(()) => {
///         println!("Admin address is valid: {}", admin_address);
///         // Proceed with admin operations
///     }
///     Err(ValidationError::InvalidAddress) => {
///         println!("Invalid admin address format");
///         // Handle invalid address
///     }
///     Err(e) => {
///         println!("Unexpected validation error: {:?}", e);
///     }
/// }
///
/// // Validate multiple participant addresses
/// let participants = vec![
///     Address::generate(&env),
///     Address::generate(&env),
///     Address::generate(&env),
/// ];
///
/// let mut valid_participants = Vec::new();
/// let mut invalid_count = 0;
///
/// for (i, participant) in participants.iter().enumerate() {
///     match InputValidator::validate_address(&env, participant) {
///         Ok(()) => {
///             valid_participants.push(participant.clone());
///             println!("Participant {} is valid", i + 1);
///         }
///         Err(_) => {
///             invalid_count += 1;
///             println!("Participant {} is invalid", i + 1);
///         }
///     }
/// }
///
/// println!("Valid participants: {}", valid_participants.len());
/// println!("Invalid participants: {}", invalid_count);
///
/// // Validate oracle provider address
/// let oracle_address = Address::generate(&env);
/// if InputValidator::validate_address(&env, &oracle_address).is_ok() {
///     println!("Oracle address is valid for price feeds");
/// } else {
///     println!("Oracle address validation failed");
/// }
/// ```
///
/// # String Validation
///
/// Validate strings with length and content constraints:
/// ```rust
/// # use soroban_sdk::{Env, String};
/// # use predictify_hybrid::validation::{InputValidator, ValidationError};
/// # let env = Env::default();
///
/// // Validate market questions
/// let questions = vec![
///     String::from_str(&env, "Will Bitcoin reach $100,000 by the end of 2024?"),
///     String::from_str(&env, "Short?"), // Too short
///     String::from_str(&env, &"A".repeat(600)), // Too long
///     String::from_str(&env, "Will Ethereum reach $5,000 this year?"),
/// ];
///
/// for (i, question) in questions.iter().enumerate() {
///     match InputValidator::validate_string(&env, question, 10, 500) {
///         Ok(()) => {
///             println!("Question {}: ✓ Valid (length: {})", i + 1, question.len());
///         }
///         Err(ValidationError::InvalidString) => {
///             println!("Question {}: ✗ Invalid length ({})", i + 1, question.len());
///         }
///         Err(e) => {
///             println!("Question {}: ✗ Validation error: {:?}", i + 1, e);
///         }
///     }
/// }
///
/// // Validate market descriptions
/// let descriptions = vec![
///     String::from_str(&env, "Detailed market description with comprehensive information about the prediction criteria."),
///     String::from_str(&env, ""), // Empty description
///     String::from_str(&env, "OK"), // Too short
/// ];
///
/// for (i, description) in descriptions.iter().enumerate() {
///     match InputValidator::validate_string(&env, description, 20, 1000) {
///         Ok(()) => {
///             println!("Description {}: ✓ Valid", i + 1);
///         }
///         Err(ValidationError::InvalidString) => {
///             println!("Description {}: ✗ Length must be 20-1000 characters", i + 1);
///         }
///         Err(e) => {
///             println!("Description {}: ✗ Error: {:?}", i + 1, e);
///         }
///     }
/// }
///
/// // Validate oracle feed IDs
/// let feed_ids = vec![
///     String::from_str(&env, "BTC/USD"),
///     String::from_str(&env, "ETH/USD"),
///     String::from_str(&env, "XLM/USD"),
///     String::from_str(&env, "A"), // Too short
/// ];
///
/// for (i, feed_id) in feed_ids.iter().enumerate() {
///     match InputValidator::validate_string(&env, feed_id, 3, 50) {
///         Ok(()) => {
///             println!("Feed ID {}: ✓ Valid ({})", i + 1, feed_id);
///         }
///         Err(ValidationError::InvalidString) => {
///             println!("Feed ID {}: ✗ Invalid format", i + 1);
///         }
///         Err(e) => {
///             println!("Feed ID {}: ✗ Error: {:?}", i + 1, e);
///         }
///     }
/// }
/// ```
///
/// # Numeric Validation
///
/// Validate numbers with range and positivity constraints:
/// ```rust
/// # use predictify_hybrid::validation::{InputValidator, ValidationError};
///
/// // Validate stake amounts
/// let stake_amounts = vec![
///     1000000i128,   // 1 XLM - valid
///     -500000i128,   // Negative - invalid
///     0i128,         // Zero - invalid for stakes
///     100000000i128, // 100 XLM - valid
/// ];
///
/// for (i, stake) in stake_amounts.iter().enumerate() {
///     match InputValidator::validate_positive_number(stake) {
///         Ok(()) => {
///             println!("Stake {}: ✓ Valid ({} stroops)", i + 1, stake);
///         }
///         Err(ValidationError::InvalidNumber) => {
///             println!("Stake {}: ✗ Must be positive ({} stroops)", i + 1, stake);
///         }
///         Err(e) => {
///             println!("Stake {}: ✗ Error: {:?}", i + 1, e);
///         }
///     }
/// }
///
/// // Validate threshold ranges
/// let thresholds = vec![
///     (50000000i128, 1000000i128, 100000000i128),   // $50, valid range $1-$100
///     (500000i128, 1000000i128, 100000000i128),     // $0.50, below minimum
///     (150000000i128, 1000000i128, 100000000i128),  // $150, above maximum
///     (25000000i128, 1000000i128, 100000000i128),   // $25, valid
/// ];
///
/// for (i, (threshold, min, max)) in thresholds.iter().enumerate() {
///     match InputValidator::validate_number_range(threshold, min, max) {
///         Ok(()) => {
///             println!("Threshold {}: ✓ Valid (${:.2})", i + 1, *threshold as f64 / 1_000_000.0);
///         }
///         Err(ValidationError::InvalidNumber) => {
///             println!("Threshold {}: ✗ Outside range ${:.2}-${:.2} (${:.2})",
///                 i + 1,
///                 *min as f64 / 1_000_000.0,
///                 *max as f64 / 1_000_000.0,
///                 *threshold as f64 / 1_000_000.0
///             );
///         }
///         Err(e) => {
///             println!("Threshold {}: ✗ Error: {:?}", i + 1, e);
///         }
///     }
/// }
///
/// // Validate fee percentages
/// let fee_percentages = vec![
///     250i128,   // 2.5% - valid
///     -100i128,  // Negative - invalid
///     0i128,     // 0% - valid
///     10000i128, // 100% - might be too high
///     15000i128, // 150% - definitely too high
/// ];
///
/// let min_fee = 0i128;
/// let max_fee = 1000i128; // 10% maximum
///
/// for (i, fee) in fee_percentages.iter().enumerate() {
///     match InputValidator::validate_number_range(fee, &min_fee, &max_fee) {
///         Ok(()) => {
///             println!("Fee {}: ✓ Valid ({:.2}%)", i + 1, *fee as f64 / 100.0);
///         }
///         Err(ValidationError::InvalidNumber) => {
///             println!("Fee {}: ✗ Must be 0-10% ({:.2}%)", i + 1, *fee as f64 / 100.0);
///         }
///         Err(e) => {
///             println!("Fee {}: ✗ Error: {:?}", i + 1, e);
///         }
///     }
/// }
/// ```
///
/// # Timestamp and Duration Validation
///
/// Validate timestamps and durations for market operations:
/// ```rust
/// # use soroban_sdk::{Env};
/// # use predictify_hybrid::validation::{InputValidator, ValidationError};
/// # let env = Env::default();
///
/// // Get current timestamp
/// let current_time = env.ledger().timestamp();
///
/// // Validate future timestamps
/// let timestamps = vec![
///     current_time - 3600,    // 1 hour ago - invalid
///     current_time,           // Now - invalid
///     current_time + 3600,    // 1 hour from now - valid
///     current_time + 86400,   // 1 day from now - valid
///     current_time + 2592000, // 30 days from now - valid
/// ];
///
/// for (i, timestamp) in timestamps.iter().enumerate() {
///     match InputValidator::validate_future_timestamp(&env, timestamp) {
///         Ok(()) => {
///             let hours_from_now = (*timestamp as i64 - current_time as i64) / 3600;
///             println!("Timestamp {}: ✓ Valid ({} hours from now)", i + 1, hours_from_now);
///         }
///         Err(ValidationError::InvalidTimestamp) => {
///             let hours_from_now = (*timestamp as i64 - current_time as i64) / 3600;
///             println!("Timestamp {}: ✗ Must be in future ({} hours from now)", i + 1, hours_from_now);
///         }
///         Err(e) => {
///             println!("Timestamp {}: ✗ Error: {:?}", i + 1, e);
///         }
///     }
/// }
///
/// // Validate market durations
/// let durations = vec![
///     0u32,   // 0 days - invalid
///     1u32,   // 1 day - valid
///     7u32,   // 1 week - valid
///     30u32,  // 1 month - valid
///     90u32,  // 3 months - valid
///     365u32, // 1 year - valid
///     400u32, // Over 1 year - might be invalid
/// ];
///
/// for (i, duration) in durations.iter().enumerate() {
///     match InputValidator::validate_duration(duration) {
///         Ok(()) => {
///             println!("Duration {}: ✓ Valid ({} days)", i + 1, duration);
///         }
///         Err(ValidationError::InvalidDuration) => {
///             println!("Duration {}: ✗ Invalid ({} days)", i + 1, duration);
///         }
///         Err(e) => {
///             println!("Duration {}: ✗ Error: {:?}", i + 1, e);
///         }
///     }
/// }
///
/// // Convert durations to timestamps and validate
/// let base_time = current_time;
/// for (i, duration) in durations.iter().enumerate() {
///     let deadline = base_time + (*duration as u64 * 86400); // Convert days to seconds
///     
///     match InputValidator::validate_future_timestamp(&env, &deadline) {
///         Ok(()) => {
///             println!("Deadline for duration {}: ✓ Valid", i + 1);
///         }
///         Err(_) => {
///             println!("Deadline for duration {}: ✗ Invalid", i + 1);
///         }
///     }
/// }
/// ```
///
/// # Integration Points
///
/// InputValidator integrates with:
/// - **Market Creation**: Validate all market creation parameters
/// - **User Input**: Validate user-provided data before processing
/// - **Oracle Configuration**: Validate oracle setup parameters
/// - **Fee Configuration**: Validate fee amounts and percentages
/// - **Admin Operations**: Validate administrative parameters
/// - **Voting System**: Validate voting parameters and constraints
/// - **Dispute System**: Validate dispute creation parameters
///
/// # Validation Rules
///
/// Current validation rules and constraints:
/// - **Addresses**: Must be valid Soroban SDK addresses
/// - **Strings**: Length constraints vary by use case (10-500 chars typical)
/// - **Numbers**: Must be positive for stakes, within ranges for thresholds
/// - **Timestamps**: Must be in the future for deadlines
/// - **Durations**: Typically 1-365 days for market duration
///
/// # Performance Considerations
///
/// Input validation is optimized for blockchain execution:
/// - **Fast Validation**: Simple checks with minimal computation
/// - **Early Exit**: Return immediately on first validation failure
/// - **Minimal Memory**: Avoid unnecessary allocations
/// - **Gas Efficient**: Low computational overhead
/// - **Deterministic**: Consistent results for same inputs
pub struct InputValidator;

impl InputValidator {
    /// Validate string length with specific limits
    pub fn validate_string_length(input: &String, max_length: u32) -> Result<(), ValidationError> {
        let length = input.len() as u32;

        if length == 0 {
            return Err(ValidationError::StringTooShort);
        }

        if length > max_length {
            return Err(ValidationError::StringTooLong);
        }

        Ok(())
    }

    /// Validate numeric range for all parameters
    pub fn validate_numeric_range(
        value: i128,
        min: i128,
        max: i128,
    ) -> Result<(), ValidationError> {
        if value < min {
            return Err(ValidationError::NumberOutOfRange);
        }

        if value > max {
            return Err(ValidationError::NumberOutOfRange);
        }

        Ok(())
    }

    /// Validate address format and validity
    pub fn validate_address_format(address: &Address) -> Result<(), ValidationError> {
        //this is called, Soroban host performs the necessary
        // authentication, manages replay prevention and enforces the user's
        // authorization policies.
        address.require_auth();

        Ok(())
    }

    pub fn validate_address(address: &Address, env: &Env) -> Result<(), ValidationError> {
        address.require_auth_for_args(vec![env, address.into_val(env)]);
        Ok(())
    }

    /// Validate timestamp bounds
    pub fn validate_timestamp_bounds(
        timestamp: u64,
        min: u64,
        max: u64,
    ) -> Result<(), ValidationError> {
        if timestamp < min {
            return Err(ValidationError::TimestampOutOfBounds);
        }

        if timestamp > max {
            return Err(ValidationError::TimestampOutOfBounds);
        }

        Ok(())
    }

    /// Validate array size limits
    pub fn validate_array_size(array: &Vec<String>, max_size: u32) -> Result<(), ValidationError> {
        let size = array.len() as u32;

        if size == 0 {
            return Err(ValidationError::ArrayTooSmall);
        }

        if size > max_size {
            return Err(ValidationError::ArrayTooLarge);
        }

        Ok(())
    }

    /// Validate question format specifically
    pub fn validate_question_format(question: &String) -> Result<(), ValidationError> {
        // Check string length
        if let Err(_) = Self::validate_string_length(question, config::MAX_QUESTION_LENGTH) {
            return Err(ValidationError::InvalidQuestionFormat);
        }

        // Check for empty or whitespace-only questions
        if question.is_empty() {
            return Err(ValidationError::InvalidQuestionFormat);
        }

        // Check for minimum meaningful length (at least 10 characters)
        if question.len() < 10 {
            return Err(ValidationError::InvalidQuestionFormat);
        }
        Ok(())
    }

    /// Validate outcome format specifically
    pub fn validate_outcome_format(outcome: &String) -> Result<(), ValidationError> {
        // Check string length
        if let Err(_) = Self::validate_string_length(outcome, config::MAX_OUTCOME_LENGTH) {
            return Err(ValidationError::InvalidOutcomeFormat);
        }

        // Check for empty outcomes
        if outcome.is_empty() {
            return Err(ValidationError::InvalidOutcomeFormat);
        }

        // Check for minimum meaningful length (at least 2 characters)
        if outcome.len() < 2 {
            return Err(ValidationError::InvalidOutcomeFormat);
        }

        Ok(())
    }

    /// Validate string length and content
    pub fn validate_string(
        env: &Env,
        value: &String,
        min_length: u32,
        max_length: u32,
    ) -> Result<(), ValidationError> {
        let length = value.len() as u32;

        if length < min_length {
            return Err(ValidationError::StringTooShort);
        }

        if length > max_length {
            return Err(ValidationError::StringTooLong);
        }

        if value.is_empty() {
            return Err(ValidationError::StringTooShort);
        }

        Ok(())
    }

    /// Validate number range
    pub fn validate_number_range(
        value: &i128,
        min: &i128,
        max: &i128,
    ) -> Result<(), ValidationError> {
        if *value < *min {
            return Err(ValidationError::NumberOutOfRange);
        }

        if *value > *max {
            return Err(ValidationError::NumberOutOfRange);
        }

        Ok(())
    }

    /// Validate positive number
    pub fn validate_positive_number(value: &i128) -> Result<(), ValidationError> {
        if *value <= 0 {
            return Err(ValidationError::NumberOutOfRange);
        }

        Ok(())
    }

    /// Validate timestamp (must be in the future)
    pub fn validate_future_timestamp(env: &Env, timestamp: &u64) -> Result<(), ValidationError> {
        let current_time = env.ledger().timestamp();

        if *timestamp <= current_time {
            return Err(ValidationError::TimestampOutOfBounds);
        }

        Ok(())
    }

    /// Validate duration range
    pub fn validate_duration(duration_days: &u32) -> Result<(), ValidationError> {
        if *duration_days < config::MIN_MARKET_DURATION_DAYS {
            return Err(ValidationError::InvalidDuration);
        }

        if *duration_days > config::MAX_MARKET_DURATION_DAYS {
            return Err(ValidationError::InvalidDuration);
        }

        Ok(())
    }
}

// ===== MARKET VALIDATION =====
/// Comprehensive market validation utilities for prediction market operations.
///
/// This utility class provides specialized validation operations for prediction markets,
/// including market creation validation, voting validation, oracle validation,
/// fee validation, and market state validation. All validation functions return
/// detailed ValidationResult structures with comprehensive error reporting.
///
/// # Core Functionality
///
/// **Market Creation Validation:**
/// - Validate all market creation parameters
/// - Check admin permissions and address validity
/// - Validate market questions and outcomes
/// - Verify duration and oracle configuration
///
/// **Voting Validation:**
/// - Validate user voting permissions
/// - Check vote outcomes and stake amounts
/// - Verify market state for voting eligibility
/// - Handle duplicate vote prevention
///
/// **Oracle Validation:**
/// - Validate oracle configuration parameters
/// - Check oracle provider settings
/// - Verify feed IDs and threshold values
/// - Handle oracle resolution validation
///
/// **Fee Validation:**
/// - Validate fee collection parameters
/// - Check market eligibility for fee collection
/// - Verify fee amounts and percentages
/// - Handle fee distribution validation
///
/// **Market State Validation:**
/// - Validate market state transitions
/// - Check market lifecycle constraints
/// - Verify resolution and dispute states
/// - Handle market extension validation
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Address, String, Vec, Symbol};
/// # use predictify_hybrid::validation::{MarketValidator, ValidationResult};
/// # use predictify_hybrid::types::{Market, OracleConfig, OracleProvider, MarketState};
/// # let env = Env::default();
///
/// // Validate market creation
/// let admin = Address::generate(&env);
/// let question = String::from_str(&env, "Will Bitcoin reach $100,000 by year end?");
/// let outcomes = vec![
///     &env,
///     String::from_str(&env, "yes"),
///     String::from_str(&env, "no"),
/// ];
/// let duration = 90u32; // 90 days
/// let oracle_config = OracleConfig {
///     provider: OracleProvider::Reflector,
///     feed_id: String::from_str(&env, "BTC/USD"),
///     threshold: 100000000000i128, // $100k
///     comparison: String::from_str(&env, "gte"),
/// };
///
/// let creation_result = MarketValidator::validate_market_creation(
///     &env,
///     &admin,
///     &question,
///     &outcomes,
///     &duration,
///     &oracle_config,
/// );
///
/// if creation_result.is_valid {
///     println!("✓ Market creation parameters are valid");
///     if creation_result.has_warnings() {
///         println!("⚠ {} warnings generated", creation_result.warning_count);
///     }
/// } else {
///     println!("✗ Market creation validation failed with {} errors", creation_result.error_count);
/// }
///
/// // Validate voting parameters
/// let voter = Address::generate(&env);
/// let market_id = Symbol::new(&env, "BTC_100K");
/// let outcome = String::from_str(&env, "yes");
/// let stake = 5000000i128; // 5 XLM
///
/// let voting_result = MarketValidator::validate_voting(
///     &env,
///     &voter,
///     &market_id,
///     &outcome,
///     &stake,
/// );
///
/// if voting_result.is_valid {
///     println!("✓ Voting parameters are valid");
/// } else {
///     println!("✗ Voting validation failed");
/// }
/// ```
///
/// # Market Creation Validation
///
/// Comprehensive validation for market creation:
/// ```rust
/// # use soroban_sdk::{Env, Address, String, Vec};
/// # use predictify_hybrid::validation::{MarketValidator, ValidationResult};
/// # use predictify_hybrid::types::{OracleConfig, OracleProvider};
/// # let env = Env::default();
///
/// // Test various market creation scenarios
/// let test_scenarios = vec![
///     // Scenario 1: Valid market
///     (
///         Address::generate(&env),
///         String::from_str(&env, "Will Bitcoin reach $100,000 by December 31, 2024?"),
///         vec![
///             &env,
///             String::from_str(&env, "yes"),
///             String::from_str(&env, "no"),
///         ],
///         90u32,
///         OracleConfig {
///             provider: OracleProvider::Reflector,
///             feed_id: String::from_str(&env, "BTC/USD"),
///             threshold: 100000000000i128,
///             comparison: String::from_str(&env, "gte"),
///         },
///         "Valid market with proper parameters"
///     ),
///     // Scenario 2: Question too short
///     (
///         Address::generate(&env),
///         String::from_str(&env, "BTC?"), // Too short
///         vec![
///             &env,
///             String::from_str(&env, "yes"),
///             String::from_str(&env, "no"),
///         ],
///         30u32,
///         OracleConfig {
///             provider: OracleProvider::Reflector,
///             feed_id: String::from_str(&env, "BTC/USD"),
///             threshold: 100000000000i128,
///             comparison: String::from_str(&env, "gte"),
///         },
///         "Market with question too short"
///     ),
///     // Scenario 3: Duration too short
///     (
///         Address::generate(&env),
///         String::from_str(&env, "Will Ethereum reach $5,000 this quarter?"),
///         vec![
///             &env,
///             String::from_str(&env, "yes"),
///             String::from_str(&env, "no"),
///         ],
///         0u32, // Invalid duration
///         OracleConfig {
///             provider: OracleProvider::Reflector,
///             feed_id: String::from_str(&env, "ETH/USD"),
///             threshold: 5000000000i128,
///             comparison: String::from_str(&env, "gte"),
///         },
///         "Market with invalid duration"
///     ),
/// ];
///
/// for (i, (admin, question, outcomes, duration, oracle_config, description)) in test_scenarios.iter().enumerate() {
///     println!("\n=== Test Scenario {}: {} ===", i + 1, description);
///     
///     let result = MarketValidator::validate_market_creation(
///         &env,
///         admin,
///         question,
///         outcomes,
///         duration,
///         oracle_config,
///     );
///     
///     if result.is_valid {
///         println!("✓ Validation passed");
///         if result.has_warnings() {
///             println!("  ⚠ {} warnings", result.warning_count);
///         }
///         if result.recommendation_count > 0 {
///             println!("  💡 {} recommendations", result.recommendation_count);
///         }
///     } else {
///         println!("✗ Validation failed");
///         println!("  Errors: {}", result.error_count);
///         if result.has_warnings() {
///             println!("  Warnings: {}", result.warning_count);
///         }
///     }
/// }
/// ```
///
/// # Voting Validation
///
/// Validate voting operations and constraints:
/// ```rust
/// # use soroban_sdk::{Env, Address, String, Symbol};
/// # use predictify_hybrid::validation::{MarketValidator, ValidationResult};
/// # let env = Env::default();
///
/// // Test voting scenarios
/// let market_id = Symbol::new(&env, "BTC_100K");
/// let valid_outcomes = vec!["yes", "no"];
///
/// let voting_scenarios = vec![
///     // Scenario 1: Valid vote
///     (
///         Address::generate(&env),
///         String::from_str(&env, "yes"),
///         5000000i128, // 5 XLM stake
///         "Valid vote with proper stake"
///     ),
///     // Scenario 2: Invalid outcome
///     (
///         Address::generate(&env),
///         String::from_str(&env, "maybe"), // Invalid outcome
///         1000000i128,
///         "Vote with invalid outcome"
///     ),
///     // Scenario 3: Zero stake
///     (
///         Address::generate(&env),
///         String::from_str(&env, "no"),
///         0i128, // Zero stake
///         "Vote with zero stake"
///     ),
///     // Scenario 4: Negative stake
///     (
///         Address::generate(&env),
///         String::from_str(&env, "yes"),
///         -1000000i128, // Negative stake
///         "Vote with negative stake"
///     ),
/// ];
///
/// for (i, (voter, outcome, stake, description)) in voting_scenarios.iter().enumerate() {
///     println!("\n=== Voting Scenario {}: {} ===", i + 1, description);
///     
///     let result = MarketValidator::validate_voting(
///         &env,
///         voter,
///         &market_id,
///         outcome,
///         stake,
///     );
///     
///     if result.is_valid {
///         println!("✓ Voting validation passed");
///         println!("  Voter: {}", voter);
///         println!("  Outcome: {}", outcome);
///         println!("  Stake: {} stroops", stake);
///     } else {
///         println!("✗ Voting validation failed");
///         println!("  Errors: {}", result.error_count);
///     }
/// }
/// ```
///
/// # Oracle Validation
///
/// Validate oracle configurations and parameters:
/// ```rust
/// # use soroban_sdk::{Env, String};
/// # use predictify_hybrid::validation::{MarketValidator, ValidationResult};
/// # use predictify_hybrid::types::{OracleConfig, OracleProvider};
/// # let env = Env::default();
///
/// // Test oracle configurations
/// let oracle_scenarios = vec![
///     // Scenario 1: Valid Reflector oracle
///     (
///         OracleConfig {
///             provider: OracleProvider::Reflector,
///             feed_id: String::from_str(&env, "BTC/USD"),
///             threshold: 100000000000i128, // $100k
///             comparison: String::from_str(&env, "gte"),
///         },
///         "Valid Reflector oracle configuration"
///     ),
///     // Scenario 2: Valid Pyth oracle
///     (
///         OracleConfig {
///             provider: OracleProvider::Pyth,
///             feed_id: String::from_str(&env, "ETH/USD"),
///             threshold: 5000000000i128, // $5k
///             comparison: String::from_str(&env, "gte"),
///         },
///         "Valid Pyth oracle configuration"
///     ),
///     // Scenario 3: Invalid threshold (negative)
///     (
///         OracleConfig {
///             provider: OracleProvider::Reflector,
///             feed_id: String::from_str(&env, "XLM/USD"),
///             threshold: -1000000i128, // Negative threshold
///             comparison: String::from_str(&env, "gte"),
///         },
///         "Oracle with negative threshold"
///     ),
///     // Scenario 4: Invalid feed ID (too short)
///     (
///         OracleConfig {
///             provider: OracleProvider::Reflector,
///             feed_id: String::from_str(&env, "B"), // Too short
///             threshold: 50000000000i128,
///             comparison: String::from_str(&env, "gte"),
///         },
///         "Oracle with invalid feed ID"
///     ),
/// ];
///
/// for (i, (oracle_config, description)) in oracle_scenarios.iter().enumerate() {
///     println!("\n=== Oracle Scenario {}: {} ===", i + 1, description);
///     
///     let result = MarketValidator::validate_oracle_config(&env, oracle_config);
///     
///     if result.is_valid {
///         println!("✓ Oracle validation passed");
///         println!("  Provider: {:?}", oracle_config.provider);
///         println!("  Feed ID: {}", oracle_config.feed_id);
///         println!("  Threshold: {}", oracle_config.threshold);
///         println!("  Comparison: {}", oracle_config.comparison);
///     } else {
///         println!("✗ Oracle validation failed");
///         println!("  Errors: {}", result.error_count);
///         if result.has_warnings() {
///             println!("  Warnings: {}", result.warning_count);
///         }
///     }
/// }
/// ```
///
/// # Fee Validation
///
/// Validate fee collection and market eligibility:
/// ```rust
/// # use soroban_sdk::{Env, Address, String, Symbol};
/// # use predictify_hybrid::validation::{MarketValidator, ValidationResult};
/// # use predictify_hybrid::types::{Market, MarketState, OracleConfig, OracleProvider};
/// # let env = Env::default();
///
/// // Create test markets with different states
/// let test_markets = vec![
///     // Market 1: Active market (should not collect fees yet)
///     (
///         Market {
///             admin: Address::generate(&env),
///             question: String::from_str(&env, "Will BTC reach $100k?"),
///             outcomes: vec![
///                 &env,
///                 String::from_str(&env, "yes"),
///                 String::from_str(&env, "no"),
///             ],
///             deadline: env.ledger().timestamp() + 86400,
///             oracle_config: OracleConfig {
///                 provider: OracleProvider::Reflector,
///                 feed_id: String::from_str(&env, "BTC/USD"),
///                 threshold: 100000000000i128,
///                 comparison: String::from_str(&env, "gte"),
///             },
///             state: MarketState::Active,
///         },
///         Symbol::new(&env, "BTC_100K"),
///         "Active market - fees not yet collectible"
///     ),
///     // Market 2: Resolved market (should allow fee collection)
///     (
///         Market {
///             admin: Address::generate(&env),
///             question: String::from_str(&env, "Will ETH reach $5k?"),
///             outcomes: vec![
///                 &env,
///                 String::from_str(&env, "yes"),
///                 String::from_str(&env, "no"),
///             ],
///             deadline: env.ledger().timestamp() - 86400, // Past deadline
///             oracle_config: OracleConfig {
///                 provider: OracleProvider::Reflector,
///                 feed_id: String::from_str(&env, "ETH/USD"),
///                 threshold: 5000000000i128,
///                 comparison: String::from_str(&env, "gte"),
///             },
///             state: MarketState::Resolved,
///         },
///         Symbol::new(&env, "ETH_5K"),
///         "Resolved market - fees collectible"
///     ),
/// ];
///
/// for (i, (market, market_id, description)) in test_markets.iter().enumerate() {
///     println!("\n=== Fee Collection Scenario {}: {} ===", i + 1, description);
///     
///     let result = MarketValidator::validate_market_for_fee_collection(
///         &env,
///         market,
///         market_id,
///     );
///     
///     if result.is_valid {
///         println!("✓ Fee collection validation passed");
///         println!("  Market state: {:?}", market.state);
///         println!("  Market ID: {:?}", market_id);
///     } else {
///         println!("✗ Fee collection validation failed");
///         println!("  Errors: {}", result.error_count);
///         println!("  Market state: {:?}", market.state);
///     }
/// }
/// ```
///
/// # Batch Market Validation
///
/// Validate multiple markets efficiently:
/// ```rust
/// # use soroban_sdk::{Env, Address, String, Vec, Symbol};
/// # use predictify_hybrid::validation::{MarketValidator, ValidationResult};
/// # use predictify_hybrid::types::{OracleConfig, OracleProvider};
/// # let env = Env::default();
///
/// // Batch validate multiple market creation requests
/// fn validate_market_batch(
///     env: &Env,
///     market_requests: &Vec<(
///         Address,
///         String,
///         Vec<String>,
///         u32,
///         OracleConfig,
///     )>,
/// ) -> Vec<ValidationResult> {
///     let mut results = Vec::new();
///     
///     for (admin, question, outcomes, duration, oracle_config) in market_requests {
///         let result = MarketValidator::validate_market_creation(
///             env,
///             admin,
///             question,
///             outcomes,
///             duration,
///             oracle_config,
///         );
///         results.push(result);
///     }
///     
///     results
/// }
///
/// // Create batch of market requests
/// let market_requests = vec![
///     (
///         Address::generate(&env),
///         String::from_str(&env, "Will Bitcoin reach $100,000?"),
///         vec![
///             String::from_str(&env, "yes"),
///             String::from_str(&env, "no"),
///         ],
///         90u32,
///         OracleConfig {
///             provider: OracleProvider::Reflector,
///             feed_id: String::from_str(&env, "BTC/USD"),
///             threshold: 100000000000i128,
///             comparison: String::from_str(&env, "gte"),
///         },
///     ),
///     (
///         Address::generate(&env),
///         String::from_str(&env, "Will Ethereum reach $5,000?"),
///         vec![
///             String::from_str(&env, "yes"),
///             String::from_str(&env, "no"),
///         ],
///         60u32,
///         OracleConfig {
///             provider: OracleProvider::Reflector,
///             feed_id: String::from_str(&env, "ETH/USD"),
///             threshold: 5000000000i128,
///             comparison: String::from_str(&env, "gte"),
///         },
///     ),
/// ];
///
/// let batch_results = validate_market_batch(&env, &market_requests);
///
/// println!("Batch validation completed:");
/// for (i, result) in batch_results.iter().enumerate() {
///     if result.is_valid {
///         println!("  Market {}: ✓ Valid", i + 1);
///     } else {
///         println!("  Market {}: ✗ Invalid ({} errors)", i + 1, result.error_count);
///     }
/// }
///
/// let valid_count = batch_results.iter().filter(|r| r.is_valid).count();
/// let total_count = batch_results.len();
/// println!("Summary: {}/{} markets passed validation", valid_count, total_count);
/// ```
///
/// # Integration Points
///
/// MarketValidator integrates with:
/// - **Market Creation System**: Validate all market creation parameters
/// - **Voting System**: Validate voting operations and constraints
/// - **Oracle System**: Validate oracle configurations and data
/// - **Fee System**: Validate fee collection eligibility
/// - **Admin System**: Validate administrative operations
/// - **Event System**: Log validation outcomes for monitoring
/// - **User Interface**: Provide detailed validation feedback
///
/// # Validation Rules
///
/// Current market validation rules:
/// - **Market Questions**: 10-500 characters, descriptive content
/// - **Market Outcomes**: At least 2 outcomes, valid strings
/// - **Market Duration**: 1-365 days typical range
/// - **Oracle Configuration**: Valid provider, feed ID, threshold
/// - **Voting Stakes**: Positive amounts, valid outcomes
/// - **Fee Collection**: Only for resolved or expired markets
///
/// # Performance Considerations
///
/// Market validation is optimized for blockchain execution:
/// - **Comprehensive Checks**: Validate all parameters in single call
/// - **Detailed Results**: Provide specific error and warning information
/// - **Batch Processing**: Support multiple market validation
/// - **Gas Efficient**: Minimize computational overhead
/// - **Early Exit**: Stop on critical errors when appropriate
pub struct MarketValidator;

impl MarketValidator {
    /// Validate market creation parameters
    pub fn validate_market_creation(
        env: &Env,
        admin: &Address,
        question: &String,
        outcomes: &Vec<String>,
        duration_days: &u32,
        oracle_config: &OracleConfig,
    ) -> ValidationResult {
        let mut result = ValidationResult::valid();

        // Validate admin address
        if let Err(_) = InputValidator::validate_address_format(admin) {
            result.add_error();
        }

        // Validate question format
        if let Err(_) = InputValidator::validate_question_format(question) {
            result.add_error();
        }

        // Validate outcomes array size
        if let Err(_) = InputValidator::validate_array_size(outcomes, config::MAX_MARKET_OUTCOMES) {
            result.add_error();
        }

        // Validate each outcome format
        for outcome in outcomes.iter() {
            if let Err(_) = InputValidator::validate_outcome_format(&outcome) {
                result.add_error();
            }
        }

        // Validate duration
        if let Err(_) = InputValidator::validate_duration(duration_days) {
            result.add_error();
        }

        // Validate oracle config
        if let Err(_) = OracleValidator::validate_oracle_config(env, oracle_config) {
            result.add_error();
        }

        // Add recommendations for optimization
        if result.is_valid {
            if question.len() < 50 {
                result.add_recommendation(); // Suggest longer questions for better clarity
            }
            if outcomes.len() < 3 {
                result.add_recommendation(); // Suggest more outcomes for better market dynamics
            }
        }

        result
    }

    /// Validate market outcomes with comprehensive validation
    pub fn validate_outcomes(env: &Env, outcomes: &Vec<String>) -> Result<(), ValidationError> {
        // Validate array size
        if let Err(_) = InputValidator::validate_array_size(outcomes, config::MAX_MARKET_OUTCOMES) {
            return Err(ValidationError::ArrayTooSmall);
        }

        if (outcomes.len() as u32) < config::MIN_MARKET_OUTCOMES {
            return Err(ValidationError::ArrayTooSmall);
        }

        // Validate each outcome format
        for outcome in outcomes.iter() {
            if let Err(_) = InputValidator::validate_outcome_format(&outcome) {
                return Err(ValidationError::InvalidOutcomeFormat);
            }
        }

        // Check for duplicate outcomes
        let mut seen = Vec::new(env);
        for outcome in outcomes.iter() {
            if seen.contains(&outcome) {
                return Err(ValidationError::InvalidOutcome);
            }
            seen.push_back(outcome.clone());
        }

        Ok(())
    }

    /// Validate market state for voting
    pub fn validate_market_for_voting(
        env: &Env,
        market: &Market,
        market_id: &Symbol,
    ) -> Result<(), ValidationError> {
        // Check if market exists
        if market.question.is_empty() {
            return Err(ValidationError::InvalidMarket);
        }

        // Check if market is still active
        let current_time = env.ledger().timestamp();
        if current_time >= market.end_time {
            return Err(ValidationError::InvalidMarket);
        }

        // Check if market is already resolved
        if market.winning_outcome.is_some() {
            return Err(ValidationError::InvalidMarket);
        }

        Ok(())
    }

    /// Validate market state for resolution
    pub fn validate_market_for_resolution(
        env: &Env,
        market: &Market,
        market_id: &Symbol,
    ) -> Result<(), ValidationError> {
        // Check if market exists
        if market.question.is_empty() {
            return Err(ValidationError::InvalidMarket);
        }

        // Check if market has ended
        let current_time = env.ledger().timestamp();
        if current_time < market.end_time {
            return Err(ValidationError::InvalidMarket);
        }

        // Check if market is already resolved
        if market.winning_outcome.is_some() {
            return Err(ValidationError::InvalidMarket);
        }

        // Check if oracle result is available
        if market.oracle_result.is_none() {
            return Err(ValidationError::InvalidMarket);
        }

        Ok(())
    }

    /// Validate market for fee collection
    pub fn validate_market_for_fee_collection(
        env: &Env,
        market: &Market,
        market_id: &Symbol,
    ) -> Result<(), ValidationError> {
        // Check if market exists
        if market.question.is_empty() {
            return Err(ValidationError::InvalidMarket);
        }

        // Check if market is resolved
        if market.winning_outcome.is_none() {
            return Err(ValidationError::InvalidMarket);
        }

        // Check if fees are already collected
        if market.fee_collected {
            return Err(ValidationError::InvalidFee);
        }

        // Check if there are sufficient stakes
        if market.total_staked < config::FEE_COLLECTION_THRESHOLD {
            return Err(ValidationError::InvalidFee);
        }

        Ok(())
    }
}

// ===== ORACLE VALIDATION =====

/// Oracle validation utilities
pub struct OracleValidator;

impl OracleValidator {
    /// Validate oracle configuration with comprehensive validation
    pub fn validate_oracle_config(
        env: &Env,
        oracle_config: &OracleConfig,
    ) -> Result<(), ValidationError> {
        // Validate feed ID string length
        if let Err(_) = InputValidator::validate_string_length(&oracle_config.feed_id, 50) {
            return Err(ValidationError::InvalidOracle);
        }

        // Validate threshold with numeric range
        if let Err(_) =
            InputValidator::validate_numeric_range(oracle_config.threshold, 1, i128::MAX)
        {
            return Err(ValidationError::InvalidOracle);
        }

        // Validate comparison operator
        if let Err(_) = Self::validate_comparison_operator(env, &oracle_config.comparison) {
            return Err(ValidationError::InvalidOracle);
        }

        Ok(())
    }

    /// Validate comparison operator
    pub fn validate_comparison_operator(
        env: &Env,
        comparison: &String,
    ) -> Result<(), ValidationError> {
        let valid_operators = vec![
            env,
            String::from_str(env, "gt"),
            String::from_str(env, "gte"),
            String::from_str(env, "lt"),
            String::from_str(env, "lte"),
            String::from_str(env, "eq"),
            String::from_str(env, "ne"),
        ];

        if !valid_operators.contains(comparison) {
            return Err(ValidationError::InvalidOracle);
        }

        Ok(())
    }

    /// Validate oracle provider
    pub fn validate_oracle_provider(provider: &OracleProvider) -> Result<(), ValidationError> {
        match provider {
            OracleProvider::BandProtocol => Ok(()),
            OracleProvider::DIA => Ok(()),
            OracleProvider::Reflector => Ok(()),
            OracleProvider::Pyth => Ok(()),
        }
    }

    /// Validate oracle result
    pub fn validate_oracle_result(
        env: &Env,
        oracle_result: &String,
        market_outcomes: &Vec<String>,
    ) -> Result<(), ValidationError> {
        // Check if oracle result is empty
        if oracle_result.is_empty() {
            return Err(ValidationError::InvalidOracle);
        }

        // Check if oracle result matches one of the market outcomes
        if !market_outcomes.contains(oracle_result) {
            return Err(ValidationError::InvalidOracle);
        }

        Ok(())
    }
}

// ===== FEE VALIDATION =====

/// Fee validation utilities
pub struct FeeValidator;

impl FeeValidator {
    /// Validate fee amount with comprehensive validation
    pub fn validate_fee_amount(amount: &i128) -> Result<(), ValidationError> {
        if let Err(_) = InputValidator::validate_numeric_range(
            *amount,
            config::MIN_FEE_AMOUNT,
            config::MAX_FEE_AMOUNT,
        ) {
            return Err(ValidationError::InvalidFee);
        }

        Ok(())
    }

    /// Validate fee percentage with comprehensive validation
    pub fn validate_fee_percentage(percentage: &i128) -> Result<(), ValidationError> {
        if let Err(_) = InputValidator::validate_numeric_range(*percentage, 0, 100) {
            return Err(ValidationError::InvalidFee);
        }

        Ok(())
    }

    /// Validate fee configuration
    pub fn validate_fee_config(
        env: &Env,
        platform_fee_percentage: &i128,
        creation_fee: &i128,
        min_fee_amount: &i128,
        max_fee_amount: &i128,
        collection_threshold: &i128,
    ) -> ValidationResult {
        let mut result = ValidationResult::valid();

        // Validate platform fee percentage
        if let Err(_) = Self::validate_fee_percentage(platform_fee_percentage) {
            result.add_error();
        }

        // Validate creation fee
        if let Err(_) = Self::validate_fee_amount(creation_fee) {
            result.add_error();
        }

        // Validate min fee amount
        if let Err(_) = Self::validate_fee_amount(min_fee_amount) {
            result.add_error();
        }

        // Validate max fee amount
        if let Err(_) = Self::validate_fee_amount(max_fee_amount) {
            result.add_error();
        }

        // Validate collection threshold
        if let Err(_) = InputValidator::validate_positive_number(collection_threshold) {
            result.add_error();
        }

        // Validate min <= max
        if *min_fee_amount > *max_fee_amount {
            result.add_error();
        }

        result
    }
}

// ===== VOTE VALIDATION =====

/// Comprehensive vote validation utilities for prediction market voting operations.
///
/// This utility class provides specialized validation for voting operations in prediction markets,
/// including user permission validation, outcome validation, stake amount validation,
/// and market state validation for voting eligibility. All validation functions ensure
/// voting integrity and prevent invalid or duplicate votes.
///
/// # Core Functionality
///
/// **User Validation:**
/// - Validate user addresses and permissions
/// - Check voting eligibility and restrictions
/// - Handle duplicate vote prevention
///
/// **Outcome Validation:**
/// - Validate vote outcomes against market options
/// - Check outcome format and validity
/// - Handle case-sensitive outcome matching
///
/// **Stake Validation:**
/// - Validate stake amounts for voting
/// - Check minimum and maximum stake limits
/// - Handle stake amount formatting
///
/// **Market State Validation:**
/// - Verify market is open for voting
/// - Check market deadlines and expiration
/// - Handle market state transitions
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Address, String, Vec, Symbol};
/// # use predictify_hybrid::validation::{VoteValidator, ValidationError};
/// # let env = Env::default();
///
/// // Validate user voting eligibility
/// let voter = Address::generate(&env);
/// let market_id = Symbol::new(&env, "BTC_100K");
///
/// match VoteValidator::validate_user(&env, &voter, &market_id) {
///     Ok(()) => println!("User is eligible to vote"),
///     Err(ValidationError::InvalidAddress) => {
///         println!("Invalid user address");
///     }
///     Err(ValidationError::InvalidVote) => {
///         println!("User has already voted or is not eligible");
///     }
///     Err(e) => println!("Validation error: {:?}", e),
/// }
///
/// // Validate vote outcome
/// let vote_outcome = String::from_str(&env, "yes");
/// let market_outcomes = vec![
///     String::from_str(&env, "yes"),
///     String::from_str(&env, "no"),
/// ];
///
/// match VoteValidator::validate_outcome(&env, &vote_outcome, &market_outcomes) {
///     Ok(()) => println!("Vote outcome is valid"),
///     Err(ValidationError::InvalidOutcome) => {
///         println!("Invalid vote outcome - must be 'yes' or 'no'");
///     }
///     Err(e) => println!("Validation error: {:?}", e),
/// }
///
/// // Validate stake amount
/// let stake_amount = 5000000i128; // 5 XLM
///
/// match VoteValidator::validate_stake_amount(&stake_amount) {
///     Ok(()) => println!("Stake amount is valid"),
///     Err(ValidationError::InvalidStake) => {
///         println!("Invalid stake amount - must be positive");
///     }
///     Err(e) => println!("Validation error: {:?}", e),
/// }
/// ```
///
/// # Integration Points
///
/// VoteValidator integrates with:
/// - **Voting System**: Validate all voting operations
/// - **Market System**: Check market state and eligibility
/// - **User System**: Validate user permissions and addresses
/// - **Stake System**: Validate stake amounts and limits
/// - **Event System**: Log voting validation outcomes
pub struct VoteValidator;

impl VoteValidator {
    /// Validate vote parameters with comprehensive validation
    pub fn validate_vote(
        env: &Env,
        user: &Address,
        market_id: &Symbol,
        outcome: &String,
        stake_amount: &i128,
        market: &Market,
    ) -> Result<(), ValidationError> {
        // Validate user address format
        if let Err(_) = InputValidator::validate_address_format(user) {
            return Err(ValidationError::InvalidAddressFormat);
        }

        // Validate market for voting
        if let Err(_) = MarketValidator::validate_market_for_voting(env, market, market_id) {
            return Err(ValidationError::InvalidVote);
        }

        // Validate outcome format
        if let Err(_) = InputValidator::validate_outcome_format(outcome) {
            return Err(ValidationError::InvalidOutcomeFormat);
        }

        // Validate outcome against market outcomes
        if let Err(_) = Self::validate_outcome(env, outcome, &market.outcomes) {
            return Err(ValidationError::InvalidVote);
        }

        // Validate stake amount with numeric range
        if let Err(_) =
            InputValidator::validate_numeric_range(*stake_amount, config::MIN_VOTE_STAKE, i128::MAX)
        {
            return Err(ValidationError::InvalidStake);
        }

        // Check if user has already voted
        if market.votes.contains_key(user.clone()) {
            return Err(ValidationError::InvalidVote);
        }

        Ok(())
    }

    /// Validate outcome against market outcomes
    pub fn validate_outcome(
        env: &Env,
        outcome: &String,
        market_outcomes: &Vec<String>,
    ) -> Result<(), ValidationError> {
        if outcome.is_empty() {
            return Err(ValidationError::InvalidOutcome);
        }

        if !market_outcomes.contains(outcome) {
            return Err(ValidationError::InvalidOutcome);
        }

        Ok(())
    }

    /// Validate stake amount
    pub fn validate_stake_amount(stake_amount: &i128) -> Result<(), ValidationError> {
        if let Err(_) = InputValidator::validate_positive_number(stake_amount) {
            return Err(ValidationError::InvalidStake);
        }

        if *stake_amount < config::MIN_VOTE_STAKE {
            return Err(ValidationError::InvalidStake);
        }

        Ok(())
    }
}

// ===== DISPUTE VALIDATION =====

/// Comprehensive dispute validation utilities for prediction market dispute operations.
///
/// This utility class provides specialized validation for dispute operations in prediction markets,
/// including dispute creation validation, user permission validation, stake validation,
/// and market state validation for dispute eligibility. All validation functions ensure
/// dispute integrity and prevent invalid or duplicate disputes.
///
/// # Core Functionality
///
/// **Dispute Creation Validation:**
/// - Validate dispute creation parameters
/// - Check user permissions for dispute initiation
/// - Verify market state allows disputes
/// - Handle dispute timing constraints
///
/// **Stake Validation:**
/// - Validate dispute stake amounts
/// - Check minimum dispute stake requirements
/// - Handle stake amount formatting and limits
///
/// **Market State Validation:**
/// - Verify market is eligible for disputes
/// - Check market resolution status
/// - Handle dispute window timing
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Address, Symbol};
/// # use predictify_hybrid::validation::{DisputeValidator, ValidationError};
/// # use predictify_hybrid::types::{Market, MarketState, OracleConfig, OracleProvider};
/// # let env = Env::default();
///
/// // Validate dispute creation
/// let user = Address::generate(&env);
/// let market_id = Symbol::new(&env, "BTC_100K");
/// let dispute_stake = 10000000i128; // 10 XLM
/// let market = Market {
///     admin: Address::generate(&env),
///     question: String::from_str(&env, "Will BTC reach $100k?"),
///     outcomes: vec![&env, String::from_str(&env, "yes"), String::from_str(&env, "no")],
///     deadline: env.ledger().timestamp() - 3600, // Past deadline
///     oracle_config: OracleConfig {
///         provider: OracleProvider::Reflector,
///         feed_id: String::from_str(&env, "BTC/USD"),
///         threshold: 100000000000i128,
///         comparison: String::from_str(&env, "gte"),
///     },
///     state: MarketState::Resolved,
/// };
///
/// match DisputeValidator::validate_dispute_creation(&env, &user, &market_id, &dispute_stake, &market) {
///     Ok(()) => println!("Dispute creation is valid"),
///     Err(ValidationError::InvalidDispute) => {
///         println!("Dispute creation failed - market not eligible or user not authorized");
///     }
///     Err(ValidationError::InvalidStake) => {
///         println!("Invalid dispute stake amount");
///     }
///     Err(e) => println!("Validation error: {:?}", e),
/// }
///
/// // Validate dispute stake
/// let stake_amounts = vec![1000000i128, 10000000i128, -5000000i128, 0i128];
///
/// for (i, stake) in stake_amounts.iter().enumerate() {
///     match DisputeValidator::validate_dispute_stake(stake) {
///         Ok(()) => println!("Dispute stake {}: Valid ({} stroops)", i + 1, stake),
///         Err(ValidationError::InvalidStake) => {
///             println!("Dispute stake {}: Invalid - must be positive ({} stroops)", i + 1, stake);
///         }
///         Err(e) => println!("Dispute stake {}: Error {:?}", i + 1, e),
///     }
/// }
/// ```
///
/// # Integration Points
///
/// DisputeValidator integrates with:
/// - **Dispute System**: Validate all dispute operations
/// - **Market System**: Check market state and dispute eligibility
/// - **User System**: Validate user permissions and addresses
/// - **Stake System**: Validate dispute stake amounts
/// - **Resolution System**: Check resolution status for disputes
pub struct DisputeValidator;

impl DisputeValidator {
    /// Validate dispute creation with comprehensive validation
    pub fn validate_dispute_creation(
        env: &Env,
        user: &Address,
        market_id: &Symbol,
        dispute_stake: &i128,
        market: &Market,
    ) -> Result<(), ValidationError> {
        // Validate user address format
        if let Err(_) = InputValidator::validate_address_format(user) {
            return Err(ValidationError::InvalidAddressFormat);
        }

        // Validate market exists and is resolved
        if market.question.is_empty() {
            return Err(ValidationError::InvalidMarket);
        }

        if market.winning_outcome.is_none() {
            return Err(ValidationError::InvalidMarket);
        }

        // Validate dispute stake with numeric range
        if let Err(_) = InputValidator::validate_numeric_range(
            *dispute_stake,
            config::MIN_DISPUTE_STAKE,
            i128::MAX,
        ) {
            return Err(ValidationError::InvalidStake);
        }

        // Check if user has already disputed
        if market.dispute_stakes.contains_key(user.clone()) {
            return Err(ValidationError::InvalidDispute);
        }

        Ok(())
    }

    /// Validate dispute stake amount with comprehensive validation
    pub fn validate_dispute_stake(stake_amount: &i128) -> Result<(), ValidationError> {
        InputValidator::validate_numeric_range(*stake_amount, config::MIN_DISPUTE_STAKE, i128::MAX)
    }
}

// ===== CONFIGURATION VALIDATION =====

/// Comprehensive configuration validation utilities for contract setup and management.
///
/// This utility class provides specialized validation for contract configuration operations,
/// including contract initialization validation, environment configuration validation,
/// admin setup validation, and token configuration validation. All validation functions
/// ensure proper contract setup and configuration integrity.
///
/// # Core Functionality
///
/// **Contract Configuration:**
/// - Validate contract initialization parameters
/// - Check admin address validity and permissions
/// - Verify token configuration and addresses
/// - Handle contract setup constraints
///
/// **Environment Configuration:**
/// - Validate environment-specific settings
/// - Check network configuration parameters
/// - Verify deployment environment constraints
/// - Handle environment-specific validation rules
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Address};
/// # use predictify_hybrid::validation::{ConfigValidator, ValidationError};
/// # use predictify_hybrid::config;
/// # let env = Env::default();
///
/// // Validate contract configuration
/// let admin = Address::generate(&env);
/// let token_id = Address::generate(&env);
///
/// match ConfigValidator::validate_contract_config(&env, &admin, &token_id) {
///     Ok(()) => println!("Contract configuration is valid"),
///     Err(ValidationError::InvalidConfig) => {
///         println!("Invalid contract configuration");
///     }
///     Err(ValidationError::InvalidAddress) => {
///         println!("Invalid admin or token address");
///     }
///     Err(e) => println!("Validation error: {:?}", e),
/// }
///
/// // Validate environment configuration
/// let environment = config::Environment::Testnet;
///
/// match ConfigValidator::validate_environment_config(&env, &environment) {
///     Ok(()) => println!("Environment configuration is valid"),
///     Err(ValidationError::InvalidConfig) => {
///         println!("Invalid environment configuration");
///     }
///     Err(e) => println!("Validation error: {:?}", e),
/// }
/// ```
///
/// # Integration Points
///
/// ConfigValidator integrates with:
/// - **Contract Initialization**: Validate setup parameters
/// - **Admin System**: Validate admin configuration
/// - **Token System**: Validate token configuration
/// - **Environment System**: Validate deployment settings
pub struct ConfigValidator;

impl ConfigValidator {
    /// Validate contract configuration
    pub fn validate_contract_config(
        env: &Env,
        admin: &Address,
        token_id: &Address,
    ) -> Result<(), ValidationError> {
        // Validate admin address
        if let Err(_) = InputValidator::validate_address(admin, env) {
            return Err(ValidationError::InvalidConfig);
        }

        // Validate token address
        if let Err(_) = InputValidator::validate_address(token_id, env) {
            return Err(ValidationError::InvalidConfig);
        }

        Ok(())
    }

    /// Validate environment configuration
    pub fn validate_environment_config(
        env: &Env,
        environment: &config::Environment,
    ) -> Result<(), ValidationError> {
        match environment {
            config::Environment::Development => Ok(()),
            config::Environment::Testnet => Ok(()),
            config::Environment::Mainnet => Ok(()),
            config::Environment::Custom => Ok(()),
        }
    }
}

// ===== COMPREHENSIVE VALIDATION =====

/// Comprehensive validation utilities for complete market operation validation.
///
/// This utility class provides end-to-end validation for complex market operations,
/// combining multiple validation types into comprehensive validation workflows.
/// It orchestrates validation across all system components to ensure complete
/// operation integrity and provides detailed validation reporting.
///
/// # Core Functionality
///
/// **Complete Market Creation:**
/// - Validate all market creation parameters comprehensively
/// - Combine input, market, and oracle validation
/// - Provide detailed validation results with warnings and recommendations
/// - Handle complex validation scenarios
///
/// **Input Validation:**
/// - Validate all user inputs comprehensively
/// - Check multiple input parameters simultaneously
/// - Provide consolidated validation results
/// - Handle input validation dependencies
///
/// **Market State Validation:**
/// - Validate complete market state comprehensively
/// - Check market lifecycle and state transitions
/// - Verify market integrity across all components
/// - Handle complex market validation scenarios
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Address, String, Vec, Symbol};
/// # use predictify_hybrid::validation::{ComprehensiveValidator, ValidationResult};
/// # use predictify_hybrid::types::{Market, OracleConfig, OracleProvider, MarketState};
/// # let env = Env::default();
///
/// // Comprehensive market creation validation
/// let admin = Address::generate(&env);
/// let question = String::from_str(&env, "Will Bitcoin reach $100,000 by year end?");
/// let outcomes = vec![
///     String::from_str(&env, "yes"),
///     String::from_str(&env, "no"),
/// ];
/// let duration = 90u32;
/// let oracle_config = OracleConfig {
///     provider: OracleProvider::Reflector,
///     feed_id: String::from_str(&env, "BTC/USD"),
///     threshold: 100000000000i128,
///     comparison: String::from_str(&env, "gte"),
/// };
///
/// let result = ComprehensiveValidator::validate_complete_market_creation(
///     &env,
///     &admin,
///     &question,
///     &outcomes,
///     &duration,
///     &oracle_config,
/// );
///
/// if result.is_valid {
///     println!("✓ Complete market creation validation passed");
///     if result.has_warnings() {
///         println!("⚠ {} warnings generated", result.warning_count);
///     }
///     if result.recommendation_count > 0 {
///         println!("💡 {} recommendations available", result.recommendation_count);
///     }
/// } else {
///     println!("✗ Market creation validation failed with {} errors", result.error_count);
/// }
///
/// // Comprehensive input validation
/// let input_result = ComprehensiveValidator::validate_inputs(
///     &env,
///     &admin,
///     &question,
///     &outcomes,
///     &duration,
/// );
///
/// println!("Input validation: {}", if input_result.is_valid { "✓ Valid" } else { "✗ Invalid" });
///
/// // Comprehensive market state validation
/// let market = Market {
///     admin: admin.clone(),
///     question: question.clone(),
///     outcomes: outcomes.clone(),
///     deadline: env.ledger().timestamp() + 86400 * duration as u64,
///     oracle_config: oracle_config.clone(),
///     state: MarketState::Active,
/// };
/// let market_id = Symbol::new(&env, "BTC_100K");
///
/// let state_result = ComprehensiveValidator::validate_market_state(
///     &env,
///     &market,
///     &market_id,
/// );
///
/// println!("Market state validation: {}", if state_result.is_valid { "✓ Valid" } else { "✗ Invalid" });
/// ```
///
/// # Integration Points
///
/// ComprehensiveValidator integrates with:
/// - **All Validation Systems**: Orchestrates comprehensive validation
/// - **Market System**: Validates complete market operations
/// - **User Interface**: Provides detailed validation feedback
/// - **Admin System**: Validates administrative operations
/// - **Event System**: Logs comprehensive validation outcomes
pub struct ComprehensiveValidator;

impl ComprehensiveValidator {
    /// Validate complete market creation with all parameters
    pub fn validate_complete_market_creation(
        env: &Env,
        admin: &Address,
        question: &String,
        outcomes: &Vec<String>,
        duration_days: &u32,
        oracle_config: &OracleConfig,
    ) -> ValidationResult {
        let mut result = ValidationResult::valid();

        // Input validation
        let input_result = Self::validate_inputs(env, admin, question, outcomes, duration_days);
        if !input_result.is_valid {
            result.add_error();
        }

        // Market validation
        let market_result = MarketValidator::validate_market_creation(
            env,
            admin,
            question,
            outcomes,
            duration_days,
            oracle_config,
        );
        if !market_result.is_valid {
            result.add_error();
        }

        // Oracle validation
        if let Err(_) = OracleValidator::validate_oracle_config(env, oracle_config) {
            result.add_error();
        }

        // Add recommendations
        if result.is_valid {
            result.add_recommendation();
            result.add_recommendation();
        }

        result
    }

    /// Validate all inputs comprehensively
    pub fn validate_inputs(
        env: &Env,
        admin: &Address,
        question: &String,
        outcomes: &Vec<String>,
        duration_days: &u32,
    ) -> ValidationResult {
        let mut result = ValidationResult::valid();

        // Validate admin
        if let Err(_) = InputValidator::validate_address(admin, env) {
            result.add_error();
        }

        // Validate question
        if let Err(_) = InputValidator::validate_string(env, question, 1, 500) {
            result.add_error();
        }

        // Validate outcomes
        if let Err(_) = MarketValidator::validate_outcomes(env, outcomes) {
            result.add_error();
        }

        // Validate duration
        if let Err(_) = InputValidator::validate_duration(duration_days) {
            result.add_error();
        }

        result
    }

    /// Validate market state comprehensively
    pub fn validate_market_state(
        env: &Env,
        market: &Market,
        market_id: &Symbol,
    ) -> ValidationResult {
        let mut result = ValidationResult::valid();

        // Basic market validation
        if market.question.is_empty() {
            result.add_error();
            return result;
        }

        // Check market timing
        let current_time = env.ledger().timestamp();
        if current_time >= market.end_time {
            result.add_warning();
        }

        // Check market resolution
        if market.winning_outcome.is_some() {
            result.add_warning();
        }

        // Check oracle result
        if market.oracle_result.is_some() {
            result.add_warning();
        }

        // Check fee collection
        if market.fee_collected {
            result.add_warning();
        }

        // Add recommendations
        if market.total_staked < config::FEE_COLLECTION_THRESHOLD {
            result.add_recommendation();
        }

        result
    }
}

// ===== VALIDATION TESTING UTILITIES =====

/// Comprehensive validation testing utilities for development and testing.
///
/// This utility class provides essential testing support for validation operations,
/// including test data generation, validation result creation, error simulation,
/// and testing infrastructure support. All functions are designed to facilitate
/// comprehensive testing of validation functionality.
///
/// # Core Functionality
///
/// **Test Data Generation:**
/// - Create test validation results for various scenarios
/// - Generate test validation errors for error handling
/// - Create test markets and oracle configurations
/// - Generate realistic test data for validation testing
///
/// **Validation Testing:**
/// - Validate test data structure integrity
/// - Create test scenarios for validation functions
/// - Support unit and integration testing
/// - Handle validation testing workflows
///
/// **Mock Data Creation:**
/// - Create mock markets for validation testing
/// - Generate mock oracle configurations
/// - Create test addresses and parameters
/// - Support comprehensive validation testing
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env};
/// # use predictify_hybrid::validation::{ValidationTestingUtils, ValidationResult, ValidationError};
/// # use predictify_hybrid::types::{Market, OracleConfig};
/// # let env = Env::default();
///
/// // Create test validation result
/// let test_result = ValidationTestingUtils::create_test_validation_result(&env);
/// println!("Test result valid: {}", test_result.is_valid);
/// println!("Test result errors: {}", test_result.error_count);
///
/// // Create test validation error
/// let test_error = ValidationTestingUtils::create_test_validation_error(&env);
/// println!("Test error: {:?}", test_error);
///
/// // Create test market
/// let test_market = ValidationTestingUtils::create_test_market(&env);
/// println!("Test market question: {}", test_market.question);
/// println!("Test market outcomes: {}", test_market.outcomes.len());
///
/// // Create test oracle config
/// let test_oracle = ValidationTestingUtils::create_test_oracle_config(&env);
/// println!("Test oracle provider: {:?}", test_oracle.provider);
/// println!("Test oracle feed: {}", test_oracle.feed_id);
///
/// // Validate test data structure
/// let validation_result = ValidationTestingUtils::validate_test_data_structure(&test_result);
/// match validation_result {
///     Ok(()) => println!("✓ Test data structure is valid"),
///     Err(e) => println!("✗ Test data validation failed: {:?}", e),
/// }
/// ```
///
/// # Integration Points
///
/// ValidationTestingUtils integrates with:
/// - **Unit Testing**: Provide test data for validation functions
/// - **Integration Testing**: Support end-to-end validation testing
/// - **Development Tools**: Generate test scenarios for development
/// - **Quality Assurance**: Support comprehensive validation testing
pub struct ValidationTestingUtils;

impl ValidationTestingUtils {
    /// Create test validation result
    pub fn create_test_validation_result(env: &Env) -> ValidationResult {
        let mut result = ValidationResult::valid();
        result.add_warning();
        result.add_recommendation();
        result
    }

    /// Create test validation error
    pub fn create_test_validation_error() -> ValidationError {
        ValidationError::InvalidInput
    }

    /// Validate test data structure
    pub fn validate_test_data_structure<T>(_data: &T) -> Result<(), ValidationError> {
        // This is a placeholder for testing validation
        Ok(())
    }

    /// Create test market for validation
    pub fn create_test_market(env: &Env) -> Market {
        Market::new(
            env,
            Address::from_str(
                env,
                "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
            ),
            String::from_str(env, "Test Market"),
            vec![
                env,
                String::from_str(env, "yes"),
                String::from_str(env, "no"),
            ],
            env.ledger().timestamp() + 86400,
            OracleConfig {
                provider: OracleProvider::Pyth,
                feed_id: String::from_str(env, "BTC/USD"),
                threshold: 2500000,
                comparison: String::from_str(env, "gt"),
            },
            crate::types::MarketState::Active,
        )
    }

    /// Create test oracle config for validation
    pub fn create_test_oracle_config(env: &Env) -> OracleConfig {
        OracleConfig {
            provider: OracleProvider::Pyth,
            feed_id: String::from_str(env, "BTC/USD"),
            threshold: 2500000,
            comparison: String::from_str(env, "gt"),
        }
    }
}

// ===== VALIDATION ERROR HANDLING =====

/// Comprehensive validation error handling utilities for error management.
///
/// This utility class provides specialized error handling for validation operations,
/// including error conversion, error logging, error recovery, and error reporting.
/// All functions ensure proper error handling and provide detailed error information
/// for debugging and user feedback.
///
/// # Core Functionality
///
/// **Error Conversion:**
/// - Convert validation errors to contract errors
/// - Handle error type mapping and conversion
/// - Provide consistent error handling across systems
/// - Support error propagation and handling
///
/// **Error Processing:**
/// - Handle validation results and extract errors
/// - Process validation outcomes and error states
/// - Support error aggregation and reporting
/// - Handle complex error scenarios
///
/// **Error Logging:**
/// - Log validation errors for debugging
/// - Record validation warnings and recommendations
/// - Support error monitoring and analysis
/// - Handle error reporting and notification
///
/// # Example Usage
///
/// ```rust
/// # use predictify_hybrid::validation::{ValidationErrorHandler, ValidationError, ValidationResult};
/// # use predictify_hybrid::errors::Error;
///
/// // Handle validation error conversion
/// let validation_error = ValidationError::InvalidStake;
/// let contract_error = ValidationErrorHandler::handle_validation_error(validation_error);
///
/// match contract_error {
///     Error::InsufficientStake => {
///         println!("Converted to insufficient stake error");
///     }
///     _ => {
///         println!("Unexpected error conversion");
///     }
/// }
///
/// // Handle validation result
/// let mut result = ValidationResult::valid();
/// result.add_error();
///
/// match ValidationErrorHandler::handle_validation_result(result) {
///     Ok(()) => println!("Validation passed"),
///     Err(e) => println!("Validation failed: {:?}", e),
/// }
/// ```
///
/// # Integration Points
///
/// ValidationErrorHandler integrates with:
/// - **Error System**: Convert and handle validation errors
/// - **Logging System**: Log validation errors and outcomes
/// - **User Interface**: Provide error feedback and messages
/// - **Monitoring System**: Track validation error patterns
pub struct ValidationErrorHandler;

impl ValidationErrorHandler {
    /// Handle validation error and convert to contract error
    pub fn handle_validation_error(error: ValidationError) -> Error {
        error.to_contract_error()
    }

    /// Handle validation result and return first error if any
    pub fn handle_validation_result(result: ValidationResult) -> Result<(), Error> {
        if result.has_errors() {
            return Err(Error::InvalidInput);
        }
        Ok(())
    }

    /// Log validation warnings and recommendations
    pub fn log_validation_info(env: &Env, result: &ValidationResult) {
        // Log warnings and recommendations
        // In a real implementation, this would log to the event system
    }
}

// ===== VALIDATION DOCUMENTATION =====

/// Comprehensive validation documentation utilities for system documentation.
///
/// This utility class provides documentation and information about the validation system,
/// including validation rules, error codes, system overview, and usage guidelines.
/// All functions provide detailed information for developers and users of the
/// validation system.
///
/// # Core Functionality
///
/// **System Documentation:**
/// - Provide validation system overview and architecture
/// - Document validation rules and constraints
/// - Explain validation workflows and processes
/// - Support developer documentation needs
///
/// **Error Documentation:**
/// - Document validation error codes and meanings
/// - Provide error handling guidelines
/// - Explain error recovery strategies
/// - Support error troubleshooting
///
/// **Usage Documentation:**
/// - Provide validation usage examples
/// - Document best practices and patterns
/// - Explain integration guidelines
/// - Support developer onboarding
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Map, String};
/// # use predictify_hybrid::validation::ValidationDocumentation;
/// # let env = Env::default();
///
/// // Get validation system overview
/// let overview = ValidationDocumentation::get_validation_overview(&env);
/// println!("Validation system: {}", overview);
///
/// // Get validation rules
/// let rules = ValidationDocumentation::get_validation_rules(&env);
/// for (rule_name, rule_description) in rules.iter() {
///     println!("Rule {}: {}", rule_name, rule_description);
/// }
///
/// // Get error codes
/// let error_codes = ValidationDocumentation::get_validation_error_codes(&env);
/// for (error_code, error_description) in error_codes.iter() {
///     println!("Error {}: {}", error_code, error_description);
/// }
/// ```
///
/// # Integration Points
///
/// ValidationDocumentation integrates with:
/// - **Documentation System**: Provide validation documentation
/// - **Developer Tools**: Support development and debugging
/// - **User Interface**: Provide user-facing documentation
/// - **Help System**: Support user guidance and assistance
pub struct ValidationDocumentation;

impl ValidationDocumentation {
    /// Get validation system overview
    pub fn get_validation_overview(env: &Env) -> String {
        String::from_str(
            env,
            "Comprehensive validation system for Predictify Hybrid contract",
        )
    }

    /// Get validation rules documentation
    pub fn get_validation_rules(env: &Env) -> Map<String, String> {
        let mut rules = Map::new(env);

        rules.set(
            String::from_str(env, "market_creation"),
            String::from_str(env, "Market creation requires valid admin, question, outcomes, duration, and oracle config")
        );

        rules.set(
            String::from_str(env, "voting"),
            String::from_str(
                env,
                "Voting requires valid user, market, outcome, and stake amount",
            ),
        );

        rules.set(
            String::from_str(env, "oracle"),
            String::from_str(env, "Oracle config requires valid provider, feed_id, threshold, and comparison operator")
        );

        rules.set(
            String::from_str(env, "fees"),
            String::from_str(
                env,
                "Fees must be within configured min/max ranges and percentages",
            ),
        );

        rules
    }

    /// Get validation error codes
    pub fn get_validation_error_codes(env: &Env) -> Map<String, String> {
        let mut codes = Map::new(env);

        codes.set(
            String::from_str(env, "InvalidInput"),
            String::from_str(env, "General input validation error"),
        );

        codes.set(
            String::from_str(env, "InvalidMarket"),
            String::from_str(env, "Market-specific validation error"),
        );

        codes.set(
            String::from_str(env, "InvalidOracle"),
            String::from_str(env, "Oracle-specific validation error"),
        );

        codes.set(
            String::from_str(env, "InvalidFee"),
            String::from_str(env, "Fee-specific validation error"),
        );

        codes
    }
}
