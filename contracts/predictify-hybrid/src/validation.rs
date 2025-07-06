#![allow(unused_variables)]

use soroban_sdk::{
    contracttype, symbol_short, vec, Address, Env, Map, String, Symbol, Vec,
};
use crate::{
    errors::Error,
    types::{Market, OracleConfig, OracleProvider},
    config,
    alloc::string::ToString,
};

// ===== VALIDATION ERROR TYPES =====

/// Validation error types for different validation failures
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
        }
    }
}

// ===== VALIDATION RESULT TYPES =====

/// Validation result with detailed information
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

// ===== INPUT VALIDATION =====

/// Input validation utilities
pub struct InputValidator;

impl InputValidator {
    /// Validate address format and structure
    pub fn validate_address(env: &Env, address: &Address) -> Result<(), ValidationError> {
        // Address validation is handled by Soroban SDK
        // Additional validation can be added here if needed
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
            return Err(ValidationError::InvalidString);
        }
        
        if length > max_length {
            return Err(ValidationError::InvalidString);
        }
        
        if value.is_empty() {
            return Err(ValidationError::InvalidString);
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
            return Err(ValidationError::InvalidNumber);
        }
        
        if *value > *max {
            return Err(ValidationError::InvalidNumber);
        }
        
        Ok(())
    }

    /// Validate positive number
    pub fn validate_positive_number(value: &i128) -> Result<(), ValidationError> {
        if *value <= 0 {
            return Err(ValidationError::InvalidNumber);
        }
        
        Ok(())
    }

    /// Validate timestamp (must be in the future)
    pub fn validate_future_timestamp(env: &Env, timestamp: &u64) -> Result<(), ValidationError> {
        let current_time = env.ledger().timestamp();
        
        if *timestamp <= current_time {
            return Err(ValidationError::InvalidTimestamp);
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

/// Market validation utilities
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
        if let Err(_) = InputValidator::validate_address(env, admin) {
            result.add_error();
        }
        
        // Validate question
        if let Err(_) = InputValidator::validate_string(env, question, 1, 500) {
            result.add_error();
        }
        
        // Validate outcomes
        if let Err(_) = Self::validate_outcomes(env, outcomes) {
            result.add_error();
        }
        
        // Validate duration
        if let Err(_) = InputValidator::validate_duration(duration_days) {
            result.add_error();
        }
        
        // Validate oracle config
        if let Err(_) = OracleValidator::validate_oracle_config(env, oracle_config) {
            result.add_error();
        }
        
        result
    }

    /// Validate market outcomes
    pub fn validate_outcomes(env: &Env, outcomes: &Vec<String>) -> Result<(), ValidationError> {
        if outcomes.len() < config::MIN_MARKET_OUTCOMES {
            return Err(ValidationError::InvalidOutcome);
        }
        
        if outcomes.len() > config::MAX_MARKET_OUTCOMES {
            return Err(ValidationError::InvalidOutcome);
        }
        
        // Validate each outcome
        for outcome in outcomes.iter() {
            if let Err(_) = InputValidator::validate_string(env, &outcome, 1, 100) {
                return Err(ValidationError::InvalidOutcome);
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
        if market.question.to_string().is_empty() {
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
        if market.question.to_string().is_empty() {
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
        if market.question.to_string().is_empty() {
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
    /// Validate oracle configuration
    pub fn validate_oracle_config(
        env: &Env,
        oracle_config: &OracleConfig,
    ) -> Result<(), ValidationError> {
        // Validate feed ID
        if let Err(_) = InputValidator::validate_string(env, &oracle_config.feed_id, 1, 50) {
            return Err(ValidationError::InvalidOracle);
        }
        
        // Validate threshold
        if let Err(_) = InputValidator::validate_positive_number(&oracle_config.threshold) {
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
        if oracle_result.to_string().is_empty() {
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
    /// Validate fee amount
    pub fn validate_fee_amount(amount: &i128) -> Result<(), ValidationError> {
        if let Err(_) = InputValidator::validate_positive_number(amount) {
            return Err(ValidationError::InvalidFee);
        }
        
        if *amount < config::MIN_FEE_AMOUNT {
            return Err(ValidationError::InvalidFee);
        }
        
        if *amount > config::MAX_FEE_AMOUNT {
            return Err(ValidationError::InvalidFee);
        }
        
        Ok(())
    }

    /// Validate fee percentage
    pub fn validate_fee_percentage(percentage: &i128) -> Result<(), ValidationError> {
        if let Err(_) = InputValidator::validate_positive_number(percentage) {
            return Err(ValidationError::InvalidFee);
        }
        
        if *percentage > 100 {
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

/// Vote validation utilities
pub struct VoteValidator;

impl VoteValidator {
    /// Validate vote parameters
    pub fn validate_vote(
        env: &Env,
        user: &Address,
        market_id: &Symbol,
        outcome: &String,
        stake_amount: &i128,
        market: &Market,
    ) -> Result<(), ValidationError> {
        // Validate user address
        if let Err(_) = InputValidator::validate_address(env, user) {
            return Err(ValidationError::InvalidVote);
        }
        
        // Validate market for voting
        if let Err(_) = MarketValidator::validate_market_for_voting(env, market, market_id) {
            return Err(ValidationError::InvalidVote);
        }
        
        // Validate outcome
        if let Err(_) = Self::validate_outcome(env, outcome, &market.outcomes) {
            return Err(ValidationError::InvalidVote);
        }
        
        // Validate stake amount
        if let Err(_) = Self::validate_stake_amount(stake_amount) {
            return Err(ValidationError::InvalidVote);
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
        if outcome.to_string().is_empty() {
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

/// Dispute validation utilities
pub struct DisputeValidator;

impl DisputeValidator {
    /// Validate dispute creation
    pub fn validate_dispute_creation(
        env: &Env,
        user: &Address,
        market_id: &Symbol,
        dispute_stake: &i128,
        market: &Market,
    ) -> Result<(), ValidationError> {
        // Validate user address
        if let Err(_) = InputValidator::validate_address(env, user) {
            return Err(ValidationError::InvalidDispute);
        }
        
        // Validate market exists and is resolved
        if market.question.to_string().is_empty() {
            return Err(ValidationError::InvalidMarket);
        }
        
        if market.winning_outcome.is_none() {
            return Err(ValidationError::InvalidMarket);
        }
        
        // Validate dispute stake
        if let Err(_) = Self::validate_dispute_stake(dispute_stake) {
            return Err(ValidationError::InvalidDispute);
        }
        
        // Check if user has already disputed
        if market.dispute_stakes.contains_key(user.clone()) {
            return Err(ValidationError::InvalidDispute);
        }
        
        Ok(())
    }

    /// Validate dispute stake amount
    pub fn validate_dispute_stake(stake_amount: &i128) -> Result<(), ValidationError> {
        if let Err(_) = InputValidator::validate_positive_number(stake_amount) {
            return Err(ValidationError::InvalidStake);
        }
        
        if *stake_amount < config::MIN_DISPUTE_STAKE {
            return Err(ValidationError::InvalidStake);
        }
        
        Ok(())
    }
}

// ===== CONFIGURATION VALIDATION =====

/// Configuration validation utilities
pub struct ConfigValidator;

impl ConfigValidator {
    /// Validate contract configuration
    pub fn validate_contract_config(
        env: &Env,
        admin: &Address,
        token_id: &Address,
    ) -> Result<(), ValidationError> {
        // Validate admin address
        if let Err(_) = InputValidator::validate_address(env, admin) {
            return Err(ValidationError::InvalidConfig);
        }
        
        // Validate token address
        if let Err(_) = InputValidator::validate_address(env, token_id) {
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

/// Comprehensive validation utilities
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
            env, admin, question, outcomes, duration_days, oracle_config
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
        if let Err(_) = InputValidator::validate_address(env, admin) {
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
        if market.question.to_string().is_empty() {
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

/// Validation testing utilities
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
            Address::from_str(env, "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF"),
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

/// Validation error handling utilities
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

/// Validation documentation utilities
pub struct ValidationDocumentation;

impl ValidationDocumentation {
    /// Get validation system overview
    pub fn get_validation_overview(env: &Env) -> String {
        String::from_str(env, "Comprehensive validation system for Predictify Hybrid contract")
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
            String::from_str(env, "Voting requires valid user, market, outcome, and stake amount")
        );
        
        rules.set(
            String::from_str(env, "oracle"),
            String::from_str(env, "Oracle config requires valid provider, feed_id, threshold, and comparison operator")
        );
        
        rules.set(
            String::from_str(env, "fees"),
            String::from_str(env, "Fees must be within configured min/max ranges and percentages")
        );
        
        rules
    }

    /// Get validation error codes
    pub fn get_validation_error_codes(env: &Env) -> Map<String, String> {
        let mut codes = Map::new(env);
        
        codes.set(
            String::from_str(env, "InvalidInput"),
            String::from_str(env, "General input validation error")
        );
        
        codes.set(
            String::from_str(env, "InvalidMarket"),
            String::from_str(env, "Market-specific validation error")
        );
        
        codes.set(
            String::from_str(env, "InvalidOracle"),
            String::from_str(env, "Oracle-specific validation error")
        );
        
        codes.set(
            String::from_str(env, "InvalidFee"),
            String::from_str(env, "Fee-specific validation error")
        );
        
        codes
    }
} 