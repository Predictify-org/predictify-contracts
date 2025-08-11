#![allow(dead_code)]

use soroban_sdk::{contracttype, token, vec, Address, Env, Map, String, Symbol, Vec};

use crate::errors::Error;
use crate::types::*;
// Oracle imports removed - not currently used

/// Market management system for Predictify Hybrid contract
///
/// This module provides a comprehensive market management system with:
/// - Market creation and configuration functions
/// - Market state management and validation
/// - Market analytics and statistics
/// - Market helper utilities and testing functions
/// - Market resolution and dispute handling

// ===== MARKET CREATION =====

/// Market creation utilities for the Predictify prediction market platform.
///
/// This struct provides methods to create different types of prediction markets
/// with various oracle configurations. All market creation functions validate
/// input parameters and handle fee processing automatically.
pub struct MarketCreator;

impl MarketCreator {
    /// Create a new market with full configuration

    /// Creates a new prediction market with comprehensive configuration options.
    ///
    /// This is the primary market creation function that supports all oracle types
    /// and validates all input parameters before creating the market. The function
    /// automatically generates a unique market ID, processes creation fees, and
    /// stores the market in persistent storage.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `admin` - Address of the market administrator (must have sufficient balance for fees)
    /// * `question` - The prediction question (1-500 characters, cannot be empty)
    /// * `outcomes` - Vector of possible outcomes (minimum 2, maximum 10 outcomes)
    /// * `duration_days` - Market duration in days (1-365 days)
    /// * `oracle_config` - Oracle configuration specifying data source and resolution criteria
    ///
    /// # Returns
    ///
    /// * `Ok(Symbol)` - Unique market identifier for the created market
    /// * `Err(Error)` - Creation failed due to validation or processing errors
    ///
    /// # Errors
    ///
    /// * `Error::InvalidQuestion` - Question is empty or exceeds character limits
    /// * `Error::InvalidOutcomes` - Less than 2 outcomes or empty outcome strings
    /// * `Error::InvalidDuration` - Duration is 0 or exceeds 365 days
    /// * `Error::InsufficientBalance` - Admin lacks funds for creation fee
    /// * `Error::InvalidOracleConfig` - Oracle configuration is malformed
    ///
    /// # Example
    ///
    /// ```rust
    /// use soroban_sdk::{Env, Address, String, vec};
    /// use crate::markets::MarketCreator;
    /// use crate::types::{OracleConfig, OracleProvider};
    ///
    /// let env = Env::default();
    /// let admin = Address::generate(&env);
    /// let question = String::from_str(&env, "Will Bitcoin reach $100,000 by end of 2024?");
    /// let outcomes = vec![
    ///     &env,
    ///     String::from_str(&env, "Yes"),
    ///     String::from_str(&env, "No")
    /// ];
    /// let oracle_config = OracleConfig::new(
    ///     OracleProvider::Pyth,
    ///     String::from_str(&env, "BTC/USD"),
    ///     100_000_00, // $100,000 with 2 decimal places
    ///     String::from_str(&env, "gte")
    /// );
    ///
    /// let market_id = MarketCreator::create_market(
    ///     &env,
    ///     admin,
    ///     question,
    ///     outcomes,
    ///     90, // 90 days duration
    ///     oracle_config
    /// ).expect("Market creation should succeed");
    /// ```

    pub fn create_market(
        env: &Env,
        admin: Address,
        question: String,
        outcomes: Vec<String>,
        duration_days: u32,
        oracle_config: OracleConfig,
    ) -> Result<Symbol, Error> {
        // Validate market parameters
        MarketValidator::validate_market_params(env, &question, &outcomes, duration_days)?;

        // Validate oracle configuration
        MarketValidator::validate_oracle_config(env, &oracle_config)?;

        // Generate unique market ID
        let market_id = MarketUtils::generate_market_id(env);

        // Calculate end time
        let end_time = MarketUtils::calculate_end_time(env, duration_days);

        // Create market instance
        let market = Market::new(
            env,
            admin.clone(),
            question,
            outcomes,
            end_time,
            oracle_config,
            MarketState::Active,
        );

        // Process market creation fee
        MarketUtils::process_creation_fee(env, &admin)?;

        // Store market
        env.storage().persistent().set(&market_id, &market);

        Ok(market_id)
    }


    /// Create a market with Reflector oracle

    /// Creates a prediction market using Reflector oracle as the data source.
    ///
    /// Reflector oracle provides real-time price feeds for various cryptocurrency
    /// and traditional assets. This convenience method automatically configures
    /// the oracle settings and delegates to the main create_market function.
    ///
    /// # Parameters
    ///
    /// * `_env` - The Soroban environment for blockchain operations
    /// * `admin` - Address of the market administrator
    /// * `question` - The prediction question (1-500 characters)
    /// * `outcomes` - Vector of possible outcomes (minimum 2, maximum 10)
    /// * `duration_days` - Market duration in days (1-365 days)
    /// * `asset_symbol` - Reflector asset symbol (e.g., "BTC", "ETH", "XLM")
    /// * `threshold` - Price threshold for comparison (in asset's base units)
    /// * `comparison` - Comparison operator ("gt", "gte", "lt", "lte", "eq")
    ///
    /// # Returns
    ///
    /// * `Ok(Symbol)` - Unique market identifier for the created market
    /// * `Err(Error)` - Creation failed due to validation or processing errors
    ///
    /// # Errors
    ///
    /// Same as `create_market`, plus:
    /// * `Error::InvalidOracleConfig` - Invalid asset symbol or comparison operator
    ///
    /// # Example
    ///
    /// ```rust
    /// use soroban_sdk::{Env, Address, String, vec};
    /// use crate::markets::MarketCreator;
    ///
    /// let env = Env::default();
    /// let admin = Address::generate(&env);
    /// let question = String::from_str(&env, "Will XLM price exceed $1.00 this month?");
    /// let outcomes = vec![
    ///     &env,
    ///     String::from_str(&env, "Yes"),
    ///     String::from_str(&env, "No")
    /// ];
    ///
    /// let market_id = MarketCreator::create_reflector_market(
    ///     &env,
    ///     admin,
    ///     question,
    ///     outcomes,
    ///     30, // 30 days
    ///     String::from_str(&env, "XLM"),
    ///     1_00, // $1.00 with 2 decimal places
    ///     String::from_str(&env, "gt")
    /// ).expect("Reflector market creation should succeed");
    /// ```

    pub fn create_reflector_market(
        _env: &Env,
        admin: Address,
        question: String,
        outcomes: Vec<String>,
        duration_days: u32,
        asset_symbol: String,
        threshold: i128,
        comparison: String,
    ) -> Result<Symbol, Error> {
        let oracle_config = OracleConfig {
            provider: OracleProvider::Reflector,
            feed_id: asset_symbol,
            threshold,
            comparison,
        };

        Self::create_market(
            _env,
            admin,
            question,
            outcomes,
            duration_days,
            oracle_config,
        )
    }

    /// Creates a prediction market using Pyth Network oracle as the data source.
    ///
    /// Pyth Network provides high-frequency, high-fidelity market data from
    /// professional trading firms and exchanges. This method configures a market
    /// to use Pyth's price feeds for automated resolution.
    ///
    /// # Parameters
    ///
    /// * `_env` - The Soroban environment for blockchain operations
    /// * `admin` - Address of the market administrator
    /// * `question` - The prediction question (1-500 characters)
    /// * `outcomes` - Vector of possible outcomes (minimum 2, maximum 10)
    /// * `duration_days` - Market duration in days (1-365 days)
    /// * `feed_id` - Pyth price feed identifier (e.g., "BTC/USD", "ETH/USD")
    /// * `threshold` - Price threshold for comparison (in feed's base units)
    /// * `comparison` - Comparison operator ("gt", "gte", "lt", "lte", "eq")
    ///
    /// # Returns
    ///
    /// * `Ok(Symbol)` - Unique market identifier for the created market
    /// * `Err(Error)` - Creation failed due to validation or processing errors
    ///
    /// # Errors
    ///
    /// Same as `create_market`, plus:
    /// * `Error::InvalidOracleConfig` - Invalid feed ID or comparison operator
    ///
    /// # Example
    ///
    /// ```rust
    /// use soroban_sdk::{Env, Address, String, vec};
    /// use crate::markets::MarketCreator;
    ///
    /// let env = Env::default();
    /// let admin = Address::generate(&env);
    /// let question = String::from_str(&env, "Will ETH break $5,000 before year end?");
    /// let outcomes = vec![
    ///     &env,
    ///     String::from_str(&env, "Yes"),
    ///     String::from_str(&env, "No")
    /// ];
    ///
    /// let market_id = MarketCreator::create_pyth_market(
    ///     &env,
    ///     admin,
    ///     question,
    ///     outcomes,
    ///     60, // 60 days
    ///     String::from_str(&env, "ETH/USD"),
    ///     5_000_00, // $5,000 with 2 decimal places
    ///     String::from_str(&env, "gte")
    /// ).expect("Pyth market creation should succeed");
    /// ```
    pub fn create_pyth_market(
        _env: &Env,
        admin: Address,
        question: String,
        outcomes: Vec<String>,
        duration_days: u32,
        feed_id: String,
        threshold: i128,
        comparison: String,
    ) -> Result<Symbol, Error> {
        let oracle_config = OracleConfig {
            provider: OracleProvider::Pyth,
            feed_id,
            threshold,
            comparison,
        };

        Self::create_market(
            _env,
            admin,
            question,
            outcomes,
            duration_days,
            oracle_config,
        )
    }

    /// Creates a prediction market using Reflector oracle for specific asset types.
    ///
    /// This is a specialized version of `create_reflector_market` that provides
    /// additional validation and configuration for specific asset classes. It's
    /// particularly useful for markets focused on specific cryptocurrency or
    /// commodity price predictions.
    ///
    /// # Parameters
    ///
    /// * `_env` - The Soroban environment for blockchain operations
    /// * `admin` - Address of the market administrator
    /// * `question` - The prediction question (1-500 characters)
    /// * `outcomes` - Vector of possible outcomes (minimum 2, maximum 10)
    /// * `duration_days` - Market duration in days (1-365 days)
    /// * `asset_symbol` - Specific asset symbol (e.g., "BTC", "ETH", "GOLD")
    /// * `threshold` - Price threshold for comparison (in asset's base units)
    /// * `comparison` - Comparison operator ("gt", "gte", "lt", "lte", "eq")
    ///
    /// # Returns
    ///
    /// * `Ok(Symbol)` - Unique market identifier for the created market
    /// * `Err(Error)` - Creation failed due to validation or processing errors
    ///
    /// # Errors
    ///
    /// Same as `create_reflector_market`
    ///
    /// # Example
    ///
    /// ```rust
    /// use soroban_sdk::{Env, Address, String, vec};
    /// use crate::markets::MarketCreator;
    ///
    /// let env = Env::default();
    /// let admin = Address::generate(&env);
    /// let question = String::from_str(&env, "Will Bitcoin dominance exceed 50% this quarter?");
    /// let outcomes = vec![
    ///     &env,
    ///     String::from_str(&env, "Yes"),
    ///     String::from_str(&env, "No")
    /// ];
    ///
    /// let market_id = MarketCreator::create_reflector_asset_market(
    ///     &env,
    ///     admin,
    ///     question,
    ///     outcomes,
    ///     90, // 90 days
    ///     String::from_str(&env, "BTC"),
    ///     50_00, // 50% with 2 decimal places
    ///     String::from_str(&env, "gt")
    /// ).expect("Asset market creation should succeed");
    /// ```
    pub fn create_reflector_asset_market(
        _env: &Env,
        admin: Address,
        question: String,
        outcomes: Vec<String>,
        duration_days: u32,
        asset_symbol: String,
        threshold: i128,
        comparison: String,
    ) -> Result<Symbol, Error> {
        Self::create_reflector_market(
            _env,
            admin,
            question,
            outcomes,
            duration_days,
            asset_symbol,
            threshold,
            comparison,
        )
    }
}

// ===== MARKET VALIDATION =====

/// Market validation utilities for ensuring data integrity and business rules.
///
/// This struct provides comprehensive validation functions for market creation,
/// voting operations, and state transitions. All validation functions follow
/// strict business rules to maintain platform integrity and user experience.
pub struct MarketValidator;

impl MarketValidator {
    /// Validates all parameters required for market creation.
    ///
    /// This function performs comprehensive validation of market creation parameters
    /// to ensure they meet platform requirements and business rules. It checks
    /// question validity, outcome constraints, and duration limits.
    ///
    /// # Parameters
    ///
    /// * `_env` - The Soroban environment for blockchain operations
    /// * `question` - The prediction question to validate
    /// * `outcomes` - Vector of possible outcomes to validate
    /// * `duration_days` - Market duration in days to validate
    ///
    /// # Returns
    ///
    /// * `Ok(())` - All parameters are valid
    /// * `Err(Error)` - One or more parameters failed validation
    ///
    /// # Errors
    ///
    /// * `Error::InvalidQuestion` - Question is empty or exceeds 500 characters
    /// * `Error::InvalidOutcomes` - Less than 2 outcomes, more than 10 outcomes, or empty outcome strings
    /// * `Error::InvalidDuration` - Duration is 0 or exceeds 365 days
    ///
    /// # Validation Rules
    ///
    /// * Question: Must be non-empty and between 1-500 characters
    /// * Outcomes: Must have 2-10 unique, non-empty outcome strings
    /// * Duration: Must be between 1-365 days inclusive
    ///
    /// # Example
    ///
    /// ```rust
    /// use soroban_sdk::{Env, String, vec};
    /// use crate::markets::MarketValidator;
    ///
    /// let env = Env::default();
    /// let question = String::from_str(&env, "Will it rain tomorrow?");
    /// let outcomes = vec![
    ///     &env,
    ///     String::from_str(&env, "Yes"),
    ///     String::from_str(&env, "No")
    /// ];
    ///
    /// // Valid parameters
    /// assert!(MarketValidator::validate_market_params(
    ///     &env,
    ///     &question,
    ///     &outcomes,
    ///     7 // 1 week duration
    /// ).is_ok());
    ///
    /// // Invalid duration
    /// assert!(MarketValidator::validate_market_params(
    ///     &env,
    ///     &question,
    ///     &outcomes,
    ///     400 // Too long
    /// ).is_err());
    /// ```
    pub fn validate_market_params(
        _env: &Env,
        question: &String,
        outcomes: &Vec<String>,
        duration_days: u32,
    ) -> Result<(), Error> {
        // Validate question is not empty
        if question.is_empty() {
            return Err(Error::InvalidQuestion);
        }

        // Validate outcomes
        if outcomes.len() < 2 {
            return Err(Error::InvalidOutcomes);
        }

        for outcome in outcomes.iter() {
            if outcome.is_empty() {
                return Err(Error::InvalidOutcomes);
            }
        }

        // Validate duration
        if duration_days == 0 || duration_days > 365 {
            return Err(Error::InvalidDuration);
        }

        Ok(())
    }

    /// Validates oracle configuration for market creation.
    ///
    /// This function ensures that the oracle configuration is properly formatted
    /// and contains valid parameters for the specified oracle provider. It delegates
    /// to the oracle configuration's internal validation method.
    ///
    /// # Parameters
    ///
    /// * `_env` - The Soroban environment for blockchain operations
    /// * `oracle_config` - Oracle configuration to validate
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Oracle configuration is valid
    /// * `Err(Error)` - Oracle configuration is invalid
    ///
    /// # Errors
    ///
    /// * `Error::InvalidOracleConfig` - Invalid provider, feed ID, or comparison operator
    /// * `Error::InvalidThreshold` - Threshold value is out of acceptable range
    ///
    /// # Example
    ///
    /// ```rust
    /// use soroban_sdk::{Env, String};
    /// use crate::markets::MarketValidator;
    /// use crate::types::{OracleConfig, OracleProvider};
    ///
    /// let env = Env::default();
    /// let oracle_config = OracleConfig::new(
    ///     OracleProvider::Pyth,
    ///     String::from_str(&env, "BTC/USD"),
    ///     50_000_00, // $50,000
    ///     String::from_str(&env, "gt")
    /// );
    ///
    /// assert!(MarketValidator::validate_oracle_config(&env, &oracle_config).is_ok());
    /// ```
    pub fn validate_oracle_config(_env: &Env, oracle_config: &OracleConfig) -> Result<(), Error> {
        oracle_config.validate(_env)
    }

    /// Validates that a market is in the correct state to accept votes.
    ///
    /// This function checks if a market is still active and accepting votes.
    /// It verifies that the market hasn't expired and hasn't been resolved yet.
    ///
    /// # Parameters
    ///
    /// * `_env` - The Soroban environment for blockchain operations
    /// * `market` - Market to validate for voting eligibility
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Market is eligible for voting
    /// * `Err(Error)` - Market cannot accept votes
    ///
    /// # Errors
    ///
    /// * `Error::MarketClosed` - Market has expired (current time >= end_time)
    /// * `Error::MarketAlreadyResolved` - Market has already been resolved
    ///
    /// # Example
    ///
    /// ```rust
    /// use soroban_sdk::Env;
    /// use crate::markets::{MarketValidator, MarketStateManager};
    /// use crate::types::Symbol;
    ///
    /// let env = Env::default();
    /// let market_id = Symbol::new(&env, "test_market");
    /// let market = MarketStateManager::get_market(&env, &market_id)?;
    ///
    /// // Check if market accepts votes
    /// match MarketValidator::validate_market_for_voting(&env, &market) {
    ///     Ok(()) => println!("Market is open for voting"),
    ///     Err(e) => println!("Cannot vote: {:?}", e),
    /// }
    /// ```
    pub fn validate_market_for_voting(_env: &Env, market: &Market) -> Result<(), Error> {
        let current_time = _env.ledger().timestamp();

        if current_time >= market.end_time {
            return Err(Error::MarketClosed);
        }

        if market.oracle_result.is_some() {
            return Err(Error::MarketAlreadyResolved);
        }

        Ok(())
    }

    /// Validates that a market is ready for resolution.
    ///
    /// This function checks if a market has expired and has oracle data available
    /// for resolution. It ensures that the market is in the correct state to
    /// determine the winning outcome.
    ///
    /// # Parameters
    ///
    /// * `_env` - The Soroban environment for blockchain operations
    /// * `market` - Market to validate for resolution eligibility
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Market is ready for resolution
    /// * `Err(Error)` - Market cannot be resolved yet
    ///
    /// # Errors
    ///
    /// * `Error::MarketClosed` - Market hasn't expired yet (current time < end_time)
    /// * `Error::OracleUnavailable` - Oracle result is not available yet
    ///
    /// # Example
    ///
    /// ```rust
    /// use soroban_sdk::Env;
    /// use crate::markets::{MarketValidator, MarketStateManager};
    /// use crate::types::Symbol;
    ///
    /// let env = Env::default();
    /// let market_id = Symbol::new(&env, "expired_market");
    /// let market = MarketStateManager::get_market(&env, &market_id)?;
    ///
    /// // Check if market can be resolved
    /// match MarketValidator::validate_market_for_resolution(&env, &market) {
    ///     Ok(()) => println!("Market is ready for resolution"),
    ///     Err(e) => println!("Cannot resolve yet: {:?}", e),
    /// }
    /// ```
    pub fn validate_market_for_resolution(_env: &Env, market: &Market) -> Result<(), Error> {
        let current_time = _env.ledger().timestamp();

        if current_time < market.end_time {
            return Err(Error::MarketClosed);
        }

        if market.oracle_result.is_none() {
            return Err(Error::OracleUnavailable);
        }

        Ok(())
    }

    /// Validate outcome for a market
    pub fn validate_outcome(
        _env: &Env,
        outcome: &String,
        market_outcomes: &Vec<String>,
    ) -> Result<(), Error> {
        for valid_outcome in market_outcomes.iter() {
            if *outcome == valid_outcome {
                return Ok(());
            }
        }

        Err(Error::InvalidOutcome)
    }

    /// Validates that a stake amount meets minimum requirements and is positive.
    ///
    /// This function ensures that users provide adequate stake amounts for voting
    /// or disputing. It checks both minimum stake requirements and basic validity
    /// (positive values).
    ///
    /// # Parameters
    ///
    /// * `stake` - The stake amount to validate (in token base units)
    /// * `min_stake` - The minimum required stake amount (in token base units)
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Stake amount is valid
    /// * `Err(Error)` - Stake amount is invalid
    ///
    /// # Errors
    ///
    /// * `Error::InsufficientStake` - Stake is below the minimum required amount
    /// * `Error::InvalidState` - Stake is zero or negative
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::markets::MarketValidator;
    ///
    /// let min_stake = 1_000_000; // 0.1 XLM (assuming 7 decimal places)
    ///
    /// // Valid stake
    /// assert!(MarketValidator::validate_stake(5_000_000, min_stake).is_ok());
    ///
    /// // Insufficient stake
    /// assert!(MarketValidator::validate_stake(500_000, min_stake).is_err());
    ///
    /// // Invalid stake (negative)
    /// assert!(MarketValidator::validate_stake(-1_000_000, min_stake).is_err());
    ///
    /// // Invalid stake (zero)
    /// assert!(MarketValidator::validate_stake(0, min_stake).is_err());
    /// ```
    pub fn validate_stake(stake: i128, min_stake: i128) -> Result<(), Error> {
        if stake < min_stake {
            return Err(Error::InsufficientStake);
        }

        if stake <= 0 {
            return Err(Error::InvalidState);
        }

        Ok(())
    }
}

// ===== MARKET STATE MANAGEMENT =====

/// Market state management utilities for persistent storage operations.
///
/// This struct provides comprehensive functions for managing market state,
/// including storage operations, vote management, dispute handling, and
/// state transitions. All functions maintain data consistency and emit
/// appropriate events for state changes.
pub struct MarketStateManager;

impl MarketStateManager {
    /// Retrieves a market from persistent storage by its unique identifier.
    ///
    /// This function fetches market data from the blockchain's persistent storage.
    /// It's the primary method for accessing market information and is used
    /// throughout the system for market operations.
    ///
    /// # Parameters
    ///
    /// * `_env` - The Soroban environment for blockchain operations
    /// * `market_id` - Unique symbol identifier for the market
    ///
    /// # Returns
    ///
    /// * `Ok(Market)` - The market data if found
    /// * `Err(Error)` - Market not found or storage error
    ///
    /// # Errors
    ///
    /// * `Error::MarketNotFound` - No market exists with the specified ID
    ///
    /// # Example
    ///
    /// ```rust
    /// use soroban_sdk::{Env, Symbol};
    /// use crate::markets::MarketStateManager;
    ///
    /// let env = Env::default();
    /// let market_id = Symbol::new(&env, "market_123");
    ///
    /// match MarketStateManager::get_market(&env, &market_id) {
    ///     Ok(market) => {
    ///         println!("Found market: {}", market.question);
    ///         println!("Market state: {:?}", market.state);
    ///     },
    ///     Err(e) => println!("Market not found: {:?}", e),
    /// }
    /// ```
    pub fn get_market(_env: &Env, market_id: &Symbol) -> Result<Market, Error> {
        _env.storage()
            .persistent()
            .get(market_id)
            .ok_or(Error::MarketNotFound)
    }

    /// Updates market data in persistent storage.
    ///
    /// This function saves the current market state to persistent storage,
    /// overwriting any existing data. It's used after making changes to
    /// market state, votes, or other market properties.
    ///
    /// # Parameters
    ///
    /// * `_env` - The Soroban environment for blockchain operations
    /// * `market_id` - Unique symbol identifier for the market
    /// * `market` - Updated market data to store
    ///
    /// # Example
    ///
    /// ```rust
    /// use soroban_sdk::{Env, Symbol};
    /// use crate::markets::MarketStateManager;
    ///
    /// let env = Env::default();
    /// let market_id = Symbol::new(&env, "market_123");
    /// let mut market = MarketStateManager::get_market(&env, &market_id)?;
    ///
    /// // Modify market data
    /// market.total_staked += 1_000_000;
    ///
    /// // Save changes
    /// MarketStateManager::update_market(&env, &market_id, &market);
    /// ```
    pub fn update_market(_env: &Env, market_id: &Symbol, market: &Market) {
        _env.storage().persistent().set(market_id, market);
    }

    /// Removes a market from persistent storage after proper closure.
    ///
    /// This function safely removes a market from storage, ensuring it's
    /// properly closed first. It handles state transitions and emits
    /// appropriate events before deletion.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `market_id` - Unique symbol identifier for the market to remove
    ///
    /// # State Transitions
    ///
    /// If the market is not already closed, it will be transitioned to
    /// `MarketState::Closed` before removal.
    ///
    /// # Example
    ///
    /// ```rust
    /// use soroban_sdk::{Env, Symbol};
    /// use crate::markets::MarketStateManager;
    ///
    /// let env = Env::default();
    /// let market_id = Symbol::new(&env, "expired_market");
    ///
    /// // Remove market (will close it first if needed)
    /// MarketStateManager::remove_market(&env, &market_id);
    ///
    /// // Verify removal
    /// assert!(MarketStateManager::get_market(&env, &market_id).is_err());
    /// ```
    pub fn remove_market(env: &Env, market_id: &Symbol) {
        let mut market = match Self::get_market(env, market_id) {
            Ok(m) => m,
            Err(_) => return,
        };
        if market.state != MarketState::Closed {
            MarketStateLogic::validate_state_transition(market.state, MarketState::Closed).unwrap();
            let old_state = market.state;
            market.state = MarketState::Closed;
            MarketStateLogic::emit_state_change_event(env, market_id, old_state, market.state);
            Self::update_market(env, market_id, &market);
        }
        env.storage().persistent().remove(market_id);
    }

    /// Adds a user's vote to a market with the specified stake amount.
    ///
    /// This function records a user's vote for a specific outcome and their
    /// associated stake. It updates the market's vote mappings, stake tracking,
    /// and total staked amount. The function validates market state before
    /// allowing the vote.
    ///
    /// # Parameters
    ///
    /// * `market` - Mutable reference to the market to add the vote to
    /// * `user` - Address of the user placing the vote
    /// * `outcome` - The outcome the user is voting for
    /// * `stake` - Amount staked on this vote (in token base units)
    /// * `_market_id` - Optional market ID for event emission (currently unused)
    ///
    /// # State Requirements
    ///
    /// * Market must be in `Active` state
    /// * Market must not have expired
    /// * User must not have already voted (will overwrite existing vote)
    ///
    /// # Side Effects
    ///
    /// * Updates `market.votes` mapping with user's choice
    /// * Updates `market.stakes` mapping with user's stake
    /// * Increments `market.total_staked` by the stake amount
    ///
    /// # Example
    ///
    /// ```rust
    /// use soroban_sdk::{Env, Address, String, Symbol};
    /// use crate::markets::MarketStateManager;
    ///
    /// let env = Env::default();
    /// let user = Address::generate(&env);
    /// let market_id = Symbol::new(&env, "active_market");
    /// let mut market = MarketStateManager::get_market(&env, &market_id)?;
    ///
    /// let outcome = String::from_str(&env, "Yes");
    /// let stake = 5_000_000; // 0.5 XLM
    ///
    /// MarketStateManager::add_vote(
    ///     &mut market,
    ///     user,
    ///     outcome,
    ///     stake,
    ///     Some(&market_id)
    /// );
    ///
    /// // Save updated market
    /// MarketStateManager::update_market(&env, &market_id, &market);
    /// ```
    pub fn add_vote(
        market: &mut Market,
        user: Address,
        outcome: String,
        stake: i128,
        _market_id: Option<&Symbol>,
    ) {
        MarketStateLogic::check_function_access_for_state("vote", market.state).unwrap();
        market.votes.set(user.clone(), outcome);
        market.stakes.set(user.clone(), stake);
        market.total_staked += stake;
        // No state change for voting
    }


    /// Add dispute stake to market

    /// Adds a user's dispute stake to challenge the market's oracle result.
    ///
    /// This function allows users to stake tokens to dispute the oracle's
    /// resolution of a market. When dispute stakes are added, the market
    /// may transition from `Ended` to `Disputed` state, triggering additional
    /// resolution mechanisms.
    ///
    /// # Parameters
    ///
    /// * `market` - Mutable reference to the market being disputed
    /// * `user` - Address of the user adding dispute stake
    /// * `stake` - Amount staked for the dispute (in token base units)
    /// * `market_id` - Optional market ID for event emission
    ///
    /// # State Requirements
    ///
    /// * Market must be in `Ended` state to initiate dispute
    /// * Market must have an oracle result to dispute
    ///
    /// # State Transitions
    ///
    /// * `Ended` → `Disputed` when first dispute stake is added
    ///
    /// # Side Effects
    ///
    /// * Updates `market.dispute_stakes` mapping (accumulates existing stakes)
    /// * May transition market state to `Disputed`
    /// * Emits state change event if transition occurs
    ///
    /// # Example
    ///
    /// ```rust
    /// use soroban_sdk::{Env, Address, Symbol};
    /// use crate::markets::MarketStateManager;
    /// use crate::types::MarketState;
    ///
    /// let env = Env::default();
    /// let disputer = Address::generate(&env);
    /// let market_id = Symbol::new(&env, "ended_market");
    /// let mut market = MarketStateManager::get_market(&env, &market_id)?;
    ///
    /// // Ensure market is in Ended state
    /// assert_eq!(market.state, MarketState::Ended);
    ///
    /// let dispute_stake = 10_000_000; // 1.0 XLM
    ///
    /// MarketStateManager::add_dispute_stake(
    ///     &mut market,
    ///     disputer,
    ///     dispute_stake,
    ///     Some(&market_id)
    /// );
    ///
    /// // Market should now be in Disputed state
    /// assert_eq!(market.state, MarketState::Disputed);
    ///
    /// MarketStateManager::update_market(&env, &market_id, &market);
    /// ```

    pub fn add_dispute_stake(
        market: &mut Market,
        user: Address,
        stake: i128,
        market_id: Option<&Symbol>,
    ) {
        MarketStateLogic::check_function_access_for_state("dispute", market.state).unwrap();
        let existing_stake = market.dispute_stakes.get(user.clone()).unwrap_or(0);
        market.dispute_stakes.set(user, existing_stake + stake);
        // State transition: Ended -> Disputed
        if market.state == MarketState::Ended {
            MarketStateLogic::validate_state_transition(market.state, MarketState::Disputed)
                .unwrap();
            let old_state = market.state;
            market.state = MarketState::Disputed;
            let env = &market.votes.env();
            let owned_event_id = market_id
                .cloned()
                .unwrap_or_else(|| Symbol::new(env, "unknown_market_id"));
            MarketStateLogic::emit_state_change_event(
                env,
                &owned_event_id,
                old_state,
                market.state,
            );
        }
    }

    /// Marks a user as having claimed their winnings from a resolved market.
    ///
    /// This function updates the market's claimed status for a specific user,
    /// preventing double-claiming of rewards. It's called after successful
    /// payout distribution to winning participants.
    ///
    /// # Parameters
    ///
    /// * `market` - Mutable reference to the market
    /// * `user` - Address of the user who has claimed their winnings
    /// * `_market_id` - Optional market ID for event emission (currently unused)
    ///
    /// # State Requirements
    ///
    /// * Market must be in `Resolved` state
    /// * User must have been a winning participant
    /// * User must not have already claimed
    ///
    /// # Side Effects
    ///
    /// * Updates `market.claimed` mapping to mark user as claimed
    ///
    /// # Example
    ///
    /// ```rust
    /// use soroban_sdk::{Env, Address, Symbol};
    /// use crate::markets::MarketStateManager;
    /// use crate::types::MarketState;
    ///
    /// let env = Env::default();
    /// let winner = Address::generate(&env);
    /// let market_id = Symbol::new(&env, "resolved_market");
    /// let mut market = MarketStateManager::get_market(&env, &market_id)?;
    ///
    /// // Ensure market is resolved
    /// assert_eq!(market.state, MarketState::Resolved);
    ///
    /// // Check if user hasn't claimed yet
    /// assert!(!market.claimed.get(winner.clone()).unwrap_or(false));
    ///
    /// // Process payout (external logic)
    /// // ...
    ///
    /// // Mark as claimed
    /// MarketStateManager::mark_claimed(&mut market, winner.clone(), Some(&market_id));
    ///
    /// // Verify claim status
    /// assert!(market.claimed.get(winner).unwrap_or(false));
    ///
    /// MarketStateManager::update_market(&env, &market_id, &market);
    /// ```
    pub fn mark_claimed(market: &mut Market, user: Address, _market_id: Option<&Symbol>) {
        MarketStateLogic::check_function_access_for_state("claim", market.state).unwrap();
        market.claimed.set(user, true);
    }

    /// Sets the oracle result for a market that has reached its end time.
    ///
    /// This function stores the oracle's resolution data for the market.
    /// The oracle result is used in combination with community consensus
    /// to determine the final market outcome using the hybrid resolution algorithm.
    ///
    /// # Parameters
    ///
    /// * `market` - Mutable reference to the market
    /// * `result` - Oracle result string (typically matches one of the market outcomes)
    ///
    /// # Side Effects
    ///
    /// * Sets `market.oracle_result` to the provided result
    /// * Enables the market for resolution processing
    ///
    /// # Example
    ///
    /// ```rust
    /// use soroban_sdk::{Env, String, Symbol};
    /// use crate::markets::MarketStateManager;
    ///
    /// let env = Env::default();
    /// let market_id = Symbol::new(&env, "ended_market");
    /// let mut market = MarketStateManager::get_market(&env, &market_id)?;
    ///
    /// let oracle_result = String::from_str(&env, "Yes");
    /// MarketStateManager::set_oracle_result(&mut market, oracle_result);
    ///
    /// // Oracle result is now available for resolution
    /// assert!(market.oracle_result.is_some());
    ///
    /// MarketStateManager::update_market(&env, &market_id, &market);
    /// ```
    pub fn set_oracle_result(market: &mut Market, result: String) {
        market.oracle_result = Some(result);
    }

    /// Sets the winning outcome for a market and transitions it to resolved state.
    ///
    /// This function finalizes the market resolution by setting the winning outcome
    /// and transitioning the market state from `Ended` or `Disputed` to `Resolved`.
    /// This enables users to claim their winnings.
    ///
    /// # Parameters
    ///
    /// * `market` - Mutable reference to the market
    /// * `outcome` - The winning outcome string
    /// * `market_id` - Optional market ID for event emission
    ///
    /// # State Requirements
    ///
    /// * Market must be in `Ended` or `Disputed` state
    ///
    /// # State Transitions
    ///
    /// * `Ended` → `Resolved`
    /// * `Disputed` → `Resolved`
    ///
    /// # Side Effects
    ///
    /// * Sets `market.winning_outcome` to the specified outcome
    /// * Transitions market state to `Resolved`
    /// * Emits state change event
    ///
    /// # Example
    ///
    /// ```rust
    /// use soroban_sdk::{Env, String, Symbol};
    /// use crate::markets::MarketStateManager;
    /// use crate::types::MarketState;
    ///
    /// let env = Env::default();
    /// let market_id = Symbol::new(&env, "ended_market");
    /// let mut market = MarketStateManager::get_market(&env, &market_id)?;
    ///
    /// // Market should be in Ended state
    /// assert_eq!(market.state, MarketState::Ended);
    ///
    /// let winning_outcome = String::from_str(&env, "Yes");
    /// MarketStateManager::set_winning_outcome(
    ///     &mut market,
    ///     winning_outcome,
    ///     Some(&market_id)
    /// );
    ///
    /// // Market should now be resolved
    /// assert_eq!(market.state, MarketState::Resolved);
    /// assert!(market.winning_outcome.is_some());
    ///
    /// MarketStateManager::update_market(&env, &market_id, &market);
    /// ```
    pub fn set_winning_outcome(market: &mut Market, outcome: String, market_id: Option<&Symbol>) {
        MarketStateLogic::check_function_access_for_state("resolve", market.state).unwrap();
        let old_state = market.state;
        market.winning_outcome = Some(outcome);
        // State transition: Ended/Disputed -> Resolved
        if market.state == MarketState::Ended || market.state == MarketState::Disputed {
            MarketStateLogic::validate_state_transition(market.state, MarketState::Resolved)
                .unwrap();
            market.state = MarketState::Resolved;
            let env = &market.votes.env();
            let owned_event_id = market_id
                .cloned()
                .unwrap_or_else(|| Symbol::new(env, "unknown_market_id"));
            MarketStateLogic::emit_state_change_event(
                env,
                &owned_event_id,
                old_state,
                market.state,
            );
        }
    }

    /// Marks platform fees as collected and transitions market to closed state.
    ///
    /// This function is called after platform fees have been successfully collected
    /// from a resolved market. It transitions the market from `Resolved` to `Closed`
    /// state, indicating the market lifecycle is complete.
    ///
    /// # Parameters
    ///
    /// * `market` - Mutable reference to the market
    /// * `market_id` - Optional market ID for event emission
    ///
    /// # State Requirements
    ///
    /// * Market must be in `Resolved` state
    ///
    /// # State Transitions
    ///
    /// * `Resolved` → `Closed`
    ///
    /// # Side Effects
    ///
    /// * Sets `market.fee_collected` to true
    /// * Transitions market state to `Closed`
    /// * Emits state change event
    ///
    /// # Example
    ///
    /// ```rust
    /// use soroban_sdk::{Env, Symbol};
    /// use crate::markets::MarketStateManager;
    /// use crate::types::MarketState;
    ///
    /// let env = Env::default();
    /// let market_id = Symbol::new(&env, "resolved_market");
    /// let mut market = MarketStateManager::get_market(&env, &market_id)?;
    ///
    /// // Market should be resolved
    /// assert_eq!(market.state, MarketState::Resolved);
    ///
    /// // Collect platform fees (external logic)
    /// // ...
    ///
    /// // Mark fees as collected
    /// MarketStateManager::mark_fees_collected(&mut market, Some(&market_id));
    ///
    /// // Market should now be closed
    /// assert_eq!(market.state, MarketState::Closed);
    /// assert!(market.fee_collected);
    ///
    /// MarketStateManager::update_market(&env, &market_id, &market);
    /// ```
    pub fn mark_fees_collected(market: &mut Market, market_id: Option<&Symbol>) {
        MarketStateLogic::check_function_access_for_state("close", market.state).unwrap();
        let old_state = market.state;
        // State transition: Resolved -> Closed
        if market.state == MarketState::Resolved {
            MarketStateLogic::validate_state_transition(market.state, MarketState::Closed).unwrap();
            market.state = MarketState::Closed;
            let env = &market.votes.env();
            let owned_event_id = market_id
                .cloned()
                .unwrap_or_else(|| Symbol::new(env, "unknown_market_id"));
            MarketStateLogic::emit_state_change_event(
                env,
                &owned_event_id,
                old_state,
                market.state,
            );
        }
        market.fee_collected = true;
    }

    /// Extends the market end time to allow for dispute resolution.
    ///
    /// This function extends the market's end time when disputes are raised,
    /// providing additional time for dispute resolution processes. The extension
    /// only applies if it would result in a longer end time than currently set.
    ///
    /// # Parameters
    ///
    /// * `market` - Mutable reference to the market
    /// * `_env` - The Soroban environment for blockchain operations
    /// * `extension_hours` - Number of hours to extend the market (minimum extension)
    ///
    /// # Logic
    ///
    /// The market end time is extended only if the new time (current time + extension)
    /// would be later than the current end time. This prevents shortening the market
    /// duration accidentally.
    ///
    /// # Side Effects
    ///
    /// * May update `market.end_time` to a later timestamp
    ///
    /// # Example
    ///
    /// ```rust
    /// use soroban_sdk::{Env, Symbol};
    /// use crate::markets::MarketStateManager;
    ///
    /// let env = Env::default();
    /// let market_id = Symbol::new(&env, "disputed_market");
    /// let mut market = MarketStateManager::get_market(&env, &market_id)?;
    ///
    /// let original_end_time = market.end_time;
    ///
    /// // Extend market by 24 hours for dispute resolution
    /// MarketStateManager::extend_for_dispute(&mut market, &env, 24);
    ///
    /// // End time should be extended if needed
    /// let current_time = env.ledger().timestamp();
    /// let expected_extension = current_time + (24 * 60 * 60);
    ///
    /// if original_end_time < expected_extension {
    ///     assert_eq!(market.end_time, expected_extension);
    /// } else {
    ///     assert_eq!(market.end_time, original_end_time);
    /// }
    ///
    /// MarketStateManager::update_market(&env, &market_id, &market);
    /// ```
    pub fn extend_for_dispute(market: &mut Market, _env: &Env, extension_hours: u64) {
        let current_time = _env.ledger().timestamp();
        let extension_seconds = extension_hours * 60 * 60;

        if market.end_time < current_time + extension_seconds {
            market.end_time = current_time + extension_seconds;
        }
    }
}

// ===== MARKET ANALYTICS =====

/// Market analytics and statistics utilities for data analysis and insights.
///
/// This struct provides comprehensive analytics functions for extracting
/// meaningful statistics from market data, including participation metrics,
/// outcome distributions, and consensus analysis. These functions are essential
/// for market monitoring, user interfaces, and decision-making processes.
pub struct MarketAnalytics;

impl MarketAnalytics {
    /// Calculates comprehensive statistics for a market.
    ///
    /// This function analyzes market participation data to generate detailed
    /// statistics including vote counts, stake amounts, and outcome distribution.
    /// The statistics are useful for market monitoring and user interfaces.
    ///
    /// # Parameters
    ///
    /// * `market` - Reference to the market to analyze
    ///
    /// # Returns
    ///
    /// * `MarketStats` - Comprehensive statistics structure containing:
    ///   - Total number of votes cast
    ///   - Total amount staked across all participants
    ///   - Total dispute stakes (if any)
    ///   - Distribution of votes across different outcomes
    ///
    /// # Example
    ///
    /// ```rust
    /// use soroban_sdk::{Env, Symbol};
    /// use crate::markets::{MarketAnalytics, MarketStateManager};
    ///
    /// let env = Env::default();
    /// let market_id = Symbol::new(&env, "active_market");
    /// let market = MarketStateManager::get_market(&env, &market_id)?;
    ///
    /// let stats = MarketAnalytics::get_market_stats(&market);
    ///
    /// println!("Total votes: {}", stats.total_votes);
    /// println!("Total staked: {} stroops", stats.total_staked);
    /// println!("Dispute stakes: {} stroops", stats.total_dispute_stakes);
    ///
    /// // Analyze outcome distribution
    /// for (outcome, count) in stats.outcome_distribution.iter() {
    ///     println!("Outcome '{}': {} votes", outcome, count);
    /// }
    /// ```
    pub fn get_market_stats(market: &Market) -> MarketStats {
        let total_votes = market.votes.len() as u32;
        let total_staked = market.total_staked;
        let total_dispute_stakes = market.total_dispute_stakes();

        // Calculate outcome distribution
        let mut outcome_stats = Map::new(&market.votes.env());
        for (_, outcome) in market.votes.iter() {
            let count = outcome_stats.get(outcome.clone()).unwrap_or(0);
            outcome_stats.set(outcome.clone(), count + 1);
        }

        MarketStats {
            total_votes,
            total_staked,
            total_dispute_stakes,
            outcome_distribution: outcome_stats,
        }
    }

    /// Calculates detailed statistics for the winning outcome of a resolved market.
    ///
    /// This function analyzes the winning side of a market to determine payout
    /// distributions and winner statistics. It's essential for calculating
    /// individual payouts and understanding market resolution outcomes.
    ///
    /// # Parameters
    ///
    /// * `market` - Reference to the resolved market
    /// * `winning_outcome` - The outcome that won the market
    ///
    /// # Returns
    ///
    /// * `WinningStats` - Statistics for the winning outcome including:
    ///   - The winning outcome string
    ///   - Total stake amount on the winning outcome
    ///   - Number of users who voted for the winning outcome
    ///   - Total pool size for payout calculations
    ///
    /// # Example
    ///
    /// ```rust
    /// use soroban_sdk::{Env, String, Symbol};
    /// use crate::markets::{MarketAnalytics, MarketStateManager};
    ///
    /// let env = Env::default();
    /// let market_id = Symbol::new(&env, "resolved_market");
    /// let market = MarketStateManager::get_market(&env, &market_id)?;
    ///
    /// let winning_outcome = String::from_str(&env, "Yes");
    /// let winning_stats = MarketAnalytics::calculate_winning_stats(&market, &winning_outcome);
    ///
    /// println!("Winning outcome: {}", winning_stats.winning_outcome);
    /// println!("Winners: {} users", winning_stats.winning_voters);
    /// println!("Winning total: {} stroops", winning_stats.winning_total);
    /// println!("Total pool: {} stroops", winning_stats.total_pool);
    ///
    /// // Calculate payout ratio
    /// let payout_ratio = winning_stats.total_pool as f64 / winning_stats.winning_total as f64;
    /// println!("Payout multiplier: {:.2}x", payout_ratio);
    /// ```
    pub fn calculate_winning_stats(market: &Market, winning_outcome: &String) -> WinningStats {
        let mut winning_total = 0;
        let mut winning_voters = 0;

        for (user, outcome) in market.votes.iter() {
            if &outcome == winning_outcome {
                winning_total += market.stakes.get(user.clone()).unwrap_or(0);
                winning_voters += 1;
            }
        }

        WinningStats {
            winning_outcome: winning_outcome.clone(),
            winning_total,
            winning_voters,
            total_pool: market.total_staked,
        }
    }

    /// Retrieves comprehensive participation statistics for a specific user in a market.
    ///
    /// This function analyzes a user's involvement in a market, including their
    /// voting status, stake amounts, dispute participation, and claim status.
    /// It's useful for user interfaces and determining user eligibility for various actions.
    ///
    /// # Parameters
    ///
    /// * `market` - Reference to the market to analyze
    /// * `user` - Address of the user to get statistics for
    ///
    /// # Returns
    ///
    /// * `UserStats` - User-specific statistics including:
    ///   - Whether the user has voted
    ///   - Amount staked by the user
    ///   - Amount staked in disputes by the user
    ///   - Whether the user has claimed their winnings
    ///   - The outcome the user voted for (if any)
    ///
    /// # Example
    ///
    /// ```rust
    /// use soroban_sdk::{Env, Address, Symbol};
    /// use crate::markets::{MarketAnalytics, MarketStateManager};
    ///
    /// let env = Env::default();
    /// let user = Address::generate(&env);
    /// let market_id = Symbol::new(&env, "active_market");
    /// let market = MarketStateManager::get_market(&env, &market_id)?;
    ///
    /// let user_stats = MarketAnalytics::get_user_stats(&market, &user);
    ///
    /// if user_stats.has_voted {
    ///     println!("User voted for: {:?}", user_stats.voted_outcome);
    ///     println!("Stake amount: {} stroops", user_stats.stake);
    /// } else {
    ///     println!("User has not voted yet");
    /// }
    ///
    /// if user_stats.dispute_stake > 0 {
    ///     println!("User disputed with: {} stroops", user_stats.dispute_stake);
    /// }
    ///
    /// if user_stats.has_claimed {
    ///     println!("User has already claimed winnings");
    /// }
    /// ```
    pub fn get_user_stats(market: &Market, user: &Address) -> UserStats {
        let has_voted = market.votes.contains_key(user.clone());
        let stake = market.stakes.get(user.clone()).unwrap_or(0);
        let dispute_stake = market.dispute_stakes.get(user.clone()).unwrap_or(0);
        let has_claimed = market.claimed.get(user.clone()).unwrap_or(false);
        let voted_outcome = market.votes.get(user.clone());

        UserStats {
            has_voted,
            stake,
            dispute_stake,
            has_claimed,
            voted_outcome,
        }
    }

    /// Calculates the community consensus for a market based on voting patterns.
    ///
    /// This function analyzes all votes in a market to determine which outcome
    /// has the strongest community support. It calculates both absolute vote counts
    /// and percentage consensus, which is crucial for the hybrid resolution algorithm.
    ///
    /// # Parameters
    ///
    /// * `market` - Reference to the market to analyze
    ///
    /// # Returns
    ///
    /// * `CommunityConsensus` - Consensus analysis including:
    ///   - The outcome with the most votes
    ///   - Number of votes for the leading outcome
    ///   - Total number of votes cast
    ///   - Percentage of votes for the leading outcome
    ///
    /// # Algorithm
    ///
    /// The function counts votes for each outcome and identifies the outcome
    /// with the highest vote count. The consensus percentage is calculated as
    /// (leading_votes / total_votes) * 100.
    ///
    /// # Example
    ///
    /// ```rust
    /// use soroban_sdk::{Env, Symbol};
    /// use crate::markets::{MarketAnalytics, MarketStateManager};
    ///
    /// let env = Env::default();
    /// let market_id = Symbol::new(&env, "active_market");
    /// let market = MarketStateManager::get_market(&env, &market_id)?;
    ///
    /// let consensus = MarketAnalytics::calculate_community_consensus(&market);
    ///
    /// println!("Community consensus: {}", consensus.outcome);
    /// println!("Leading votes: {} out of {}", consensus.votes, consensus.total_votes);
    /// println!("Consensus strength: {}%", consensus.percentage);
    ///
    /// // Check if consensus is strong enough for hybrid resolution
    /// if consensus.percentage > 50 && consensus.total_votes >= 5 {
    ///     println!("Strong community consensus detected");
    /// }
    /// ```
    pub fn calculate_community_consensus(market: &Market) -> CommunityConsensus {
        let mut vote_counts: Map<String, u32> = Map::new(&market.votes.env());

        for (_, outcome) in market.votes.iter() {
            let count = vote_counts.get(outcome.clone()).unwrap_or(0);
            vote_counts.set(outcome.clone(), count + 1);
        }

        let mut consensus_outcome = String::from_str(&market.votes.env(), "");
        let mut max_votes = 0;
        let mut total_votes = 0;

        for (outcome, count) in vote_counts.iter() {
            total_votes += count;
            if count > max_votes {
                max_votes = count;
                consensus_outcome = outcome.clone();
            }
        }

        let consensus_percentage = if total_votes > 0 {
            (max_votes * 100) / total_votes
        } else {
            0
        };

        CommunityConsensus {
            outcome: consensus_outcome,
            votes: max_votes,
            total_votes,
            percentage: consensus_percentage,
        }
    }

    /// Calculates basic analytics for a market (placeholder implementation).
    ///
    /// This function provides a placeholder for basic market analytics calculation.
    /// In a production implementation, this would calculate comprehensive market
    /// metrics such as volatility, participation trends, and prediction accuracy.
    ///
    /// # Parameters
    ///
    /// * `_market` - Reference to the market to analyze (currently unused)
    ///
    /// # Returns
    ///
    /// * `MarketAnalytics` - Empty analytics struct (placeholder)
    ///
    /// # Note
    ///
    /// This is a placeholder implementation. In a production system, this function
    /// would calculate metrics such as:
    /// - Market volatility over time
    /// - Participation rate trends
    /// - Prediction accuracy scores
    /// - Stake distribution patterns
    /// - Time-series analysis of voting behavior
    ///
    /// # Example
    ///
    /// ```rust
    /// use soroban_sdk::{Env, Symbol};
    /// use crate::markets::{MarketAnalytics, MarketStateManager};
    ///
    /// let env = Env::default();
    /// let market_id = Symbol::new(&env, "market_123");
    /// let market = MarketStateManager::get_market(&env, &market_id)?;
    ///
    /// // Currently returns placeholder analytics
    /// let analytics = MarketAnalytics::calculate_basic_analytics(&market);
    ///
    /// // In future versions, this would provide detailed insights
    /// // println!("Market volatility: {}", analytics.volatility);
    /// // println!("Participation trend: {:?}", analytics.trend);
    /// ```
    pub fn calculate_basic_analytics(_market: &Market) -> MarketAnalytics {
        // This is a placeholder implementation
        // In a real implementation, you would calculate comprehensive analytics
        MarketAnalytics
    }
}

// ===== MARKET UTILITIES =====

/// General market utilities for common operations and calculations.
///
/// This struct provides essential utility functions used throughout the market
/// system, including ID generation, time calculations, fee processing, token
/// operations, payout calculations, and hybrid resolution algorithms.
pub struct MarketUtils;

impl MarketUtils {
    /// Generates a unique identifier for a new market.
    ///
    /// This function creates a unique market ID by incrementing a persistent
    /// counter stored in the contract's storage. Each call generates a new
    /// unique identifier to ensure no market ID collisions occur.
    ///
    /// # Parameters
    ///
    /// * `_env` - The Soroban environment for blockchain operations
    ///
    /// # Returns
    ///
    /// * `Symbol` - Unique market identifier
    ///
    /// # Storage Impact
    ///
    /// Updates the persistent "MarketCounter" key with the next counter value.
    ///
    /// # Example
    ///
    /// ```rust
    /// use soroban_sdk::Env;
    /// use crate::markets::MarketUtils;
    ///
    /// let env = Env::default();
    ///
    /// // Generate unique market IDs
    /// let market_id_1 = MarketUtils::generate_market_id(&env);
    /// let market_id_2 = MarketUtils::generate_market_id(&env);
    ///
    /// // IDs are unique
    /// assert_ne!(market_id_1, market_id_2);
    ///
    /// println!("Created market: {}", market_id_1);
    /// ```
    pub fn generate_market_id(_env: &Env) -> Symbol {
        let counter_key = Symbol::new(_env, "MarketCounter");
        let counter: u32 = _env.storage().persistent().get(&counter_key).unwrap_or(0);
        let new_counter = counter + 1;
        _env.storage().persistent().set(&counter_key, &new_counter);

        Symbol::new(_env, "market")
    }

    /// Calculates the end timestamp for a market based on duration in days.
    ///
    /// This function determines when a market should end by adding the specified
    /// duration to the current blockchain timestamp. The calculation uses precise
    /// time arithmetic to ensure accurate market scheduling.
    ///
    /// # Parameters
    ///
    /// * `_env` - The Soroban environment for blockchain operations
    /// * `duration_days` - Market duration in days (1-365)
    ///
    /// # Returns
    ///
    /// * `u64` - Unix timestamp when the market should end
    ///
    /// # Time Calculation
    ///
    /// End time = Current timestamp + (duration_days × 24 × 60 × 60 seconds)
    ///
    /// # Example
    ///
    /// ```rust
    /// use soroban_sdk::Env;
    /// use crate::markets::MarketUtils;
    ///
    /// let env = Env::default();
    /// let current_time = env.ledger().timestamp();
    ///
    /// // Calculate end time for 30-day market
    /// let end_time = MarketUtils::calculate_end_time(&env, 30);
    ///
    /// // Verify calculation
    /// let expected_duration = 30 * 24 * 60 * 60; // 30 days in seconds
    /// assert_eq!(end_time, current_time + expected_duration);
    ///
    /// println!("Market ends at timestamp: {}", end_time);
    /// ```
    pub fn calculate_end_time(_env: &Env, duration_days: u32) -> u64 {
        let seconds_per_day: u64 = 24 * 60 * 60;
        let duration_seconds: u64 = (duration_days as u64) * seconds_per_day;
        _env.ledger().timestamp() + duration_seconds
    }

    /// Processes the market creation fee by delegating to the fees module.
    ///
    /// This function handles the collection of market creation fees from the
    /// market administrator. It's a convenience wrapper that delegates to the
    /// dedicated fees module for consistent fee processing.
    ///
    /// # Parameters
    ///
    /// * `_env` - The Soroban environment for blockchain operations
    /// * `admin` - Address of the market administrator who pays the fee
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Fee processed successfully
    /// * `Err(Error)` - Fee processing failed
    ///
    /// # Errors
    ///
    /// * `Error::InsufficientBalance` - Admin lacks funds for the creation fee
    /// * `Error::InvalidState` - Contract is not properly configured
    ///
    /// # Deprecation Notice
    ///
    /// This function is a wrapper around `FeeManager::process_creation_fee`.
    /// Direct use of the fees module is recommended for new implementations.
    ///
    /// # Example
    ///
    /// ```rust
    /// use soroban_sdk::{Env, Address};
    /// use crate::markets::MarketUtils;
    ///
    /// let env = Env::default();
    /// let admin = Address::generate(&env);
    ///
    /// // Process creation fee
    /// match MarketUtils::process_creation_fee(&env, &admin) {
    ///     Ok(()) => println!("Creation fee processed successfully"),
    ///     Err(e) => println!("Fee processing failed: {:?}", e),
    /// }
    /// ```
    pub fn process_creation_fee(_env: &Env, admin: &Address) -> Result<(), Error> {
        // Delegate to the fees module
        crate::fees::FeeManager::process_creation_fee(_env, admin)
    }

    /// Retrieves the token client for market-related token operations.
    ///
    /// This function creates a token client instance for the contract's configured
    /// token. The token client is used for transferring stakes, processing fees,
    /// and distributing payouts throughout the market lifecycle.
    ///
    /// # Parameters
    ///
    /// * `_env` - The Soroban environment for blockchain operations
    ///
    /// # Returns
    ///
    /// * `Ok(token::Client)` - Configured token client for operations
    /// * `Err(Error)` - Token configuration is invalid or missing
    ///
    /// # Errors
    ///
    /// * `Error::InvalidState` - Token ID is not configured in contract storage
    ///
    /// # Storage Dependency
    ///
    /// Requires the "TokenID" key to be set in persistent storage during
    /// contract initialization.
    ///
    /// # Example
    ///
    /// ```rust
    /// use soroban_sdk::Env;
    /// use crate::markets::MarketUtils;
    ///
    /// let env = Env::default();
    ///
    /// // Get token client for operations
    /// match MarketUtils::get_token_client(&env) {
    ///     Ok(token_client) => {
    ///         println!("Token client ready for operations");
    ///         // Use token_client for transfers, balance checks, etc.
    ///     },
    ///     Err(e) => println!("Token client unavailable: {:?}", e),
    /// }
    /// ```
    pub fn get_token_client(_env: &Env) -> Result<token::Client, Error> {
        let token_id: Address = _env
            .storage()
            .persistent()
            .get(&Symbol::new(_env, "TokenID"))
            .ok_or(Error::InvalidState)?;

        Ok(token::Client::new(_env, &token_id))
    }

    /// Calculates the payout amount for a winning user based on their stake and pool distribution.
    ///
    /// This function implements the payout algorithm for prediction markets,
    /// distributing the total pool among winning participants proportionally
    /// to their stakes, minus platform fees.
    ///
    /// # Parameters
    ///
    /// * `user_stake` - Amount the user staked on the winning outcome
    /// * `winning_total` - Total amount staked on the winning outcome by all users
    /// * `total_pool` - Total amount staked across all outcomes
    /// * `fee_percentage` - Platform fee percentage (e.g., 2 for 2%)
    ///
    /// # Returns
    ///
    /// * `Ok(i128)` - Calculated payout amount for the user
    /// * `Err(Error)` - Calculation failed due to invalid parameters
    ///
    /// # Errors
    ///
    /// * `Error::NothingToClaim` - No winning stakes exist (winning_total is 0)
    ///
    /// # Payout Formula
    ///
    /// ```text
    /// user_share = user_stake * (100 - fee_percentage) / 100
    /// payout = user_share * total_pool / winning_total
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::markets::MarketUtils;
    ///
    /// // User staked 1000 tokens on winning outcome
    /// // Total winning stakes: 5000 tokens
    /// // Total pool: 10000 tokens
    /// // Platform fee: 2%
    /// let payout = MarketUtils::calculate_payout(1000, 5000, 10000, 2)?;
    ///
    /// // Expected: (1000 * 98 / 100) * 10000 / 5000 = 1960 tokens
    /// assert_eq!(payout, 1960);
    ///
    /// println!("User payout: {} tokens", payout);
    /// ```
    pub fn calculate_payout(
        user_stake: i128,
        winning_total: i128,
        total_pool: i128,
        fee_percentage: i128,
    ) -> Result<i128, Error> {
        if winning_total == 0 {
            return Err(Error::NothingToClaim);
        }

        let user_share = (user_stake * (100 - fee_percentage)) / 100;
        let payout = (user_share * total_pool) / winning_total;

        Ok(payout)
    }

    /// Determines the final market result using the hybrid oracle-community algorithm.
    ///
    /// This function implements Predictify's core hybrid resolution mechanism,
    /// combining oracle data with community consensus to determine the final
    /// market outcome. The algorithm provides resilience against oracle failures
    /// and incorporates community wisdom.
    ///
    /// # Parameters
    ///
    /// * `_env` - The Soroban environment for blockchain operations
    /// * `oracle_result` - The outcome determined by the oracle
    /// * `community_consensus` - Community voting consensus data
    ///
    /// # Returns
    ///
    /// * `String` - The final determined outcome for the market
    ///
    /// # Algorithm Logic
    ///
    /// 1. **Agreement**: If oracle and community agree, use that outcome
    /// 2. **Strong Consensus**: If community has >50% consensus with ≥5 votes:
    ///    - 70% weight to oracle result
    ///    - 30% weight to community result
    ///    - Use pseudo-random selection based on blockchain data
    /// 3. **Weak Consensus**: Default to oracle result
    ///
    /// # Randomness Source
    ///
    /// Uses blockchain timestamp and sequence number for pseudo-random selection
    /// when applying the 70-30 weighting mechanism.
    ///
    /// # Example
    ///
    /// ```rust
    /// use soroban_sdk::{Env, String};
    /// use crate::markets::MarketUtils;
    /// use crate::types::CommunityConsensus;
    ///
    /// let env = Env::default();
    /// let oracle_result = String::from_str(&env, "Yes");
    /// let community_consensus = CommunityConsensus {
    ///     outcome: String::from_str(&env, "No"),
    ///     votes: 8,
    ///     total_votes: 10,
    ///     percentage: 80, // Strong community consensus
    /// };
    ///
    /// let final_result = MarketUtils::determine_final_result(
    ///     &env,
    ///     &oracle_result,
    ///     &community_consensus
    /// );
    ///
    /// // Result will be either "Yes" (70% chance) or "No" (30% chance)
    /// println!("Final market result: {}", final_result);
    /// ```
    pub fn determine_final_result(
        _env: &Env,
        oracle_result: &String,
        community_consensus: &CommunityConsensus,
    ) -> String {
        if oracle_result == &community_consensus.outcome {
            // If both agree, use that outcome
            oracle_result.clone()
        } else {
            // If they disagree, check if community consensus is strong
            if community_consensus.percentage > 50 && community_consensus.total_votes >= 5 {
                // Apply 70-30 weighting using pseudo-random selection
                let timestamp = _env.ledger().timestamp();
                let sequence = _env.ledger().sequence();
                let combined = timestamp as u128 + sequence as u128;
                let random_value = (combined % 100) as u32;

                if random_value < 30 {
                    // 30% chance to choose community result
                    community_consensus.outcome.clone()
                } else {
                    // 70% chance to choose oracle result
                    oracle_result.clone()
                }
            } else {
                // Not enough community consensus, use oracle result
                oracle_result.clone()
            }
        }
    }
}

// ===== MARKET STATISTICS TYPES =====

/// Comprehensive market statistics for analysis and monitoring.
///
/// This structure contains aggregated data about market participation,
/// including vote counts, stake amounts, and outcome distribution.
/// It's used by analytics functions and user interfaces to display
/// market health and participation metrics.
///
/// # Fields
///
/// * `total_votes` - Total number of votes cast in the market
/// * `total_staked` - Total amount staked across all participants (in token base units)
/// * `total_dispute_stakes` - Total amount staked in disputes (in token base units)
/// * `outcome_distribution` - Map of outcomes to their respective vote counts
///
/// # Example Usage
///
/// ```rust
/// use crate::markets::{MarketAnalytics, MarketStateManager};
/// use soroban_sdk::{Env, Symbol};
///
/// let env = Env::default();
/// let market_id = Symbol::new(&env, "market_123");
/// let market = MarketStateManager::get_market(&env, &market_id)?;
/// let stats = MarketAnalytics::get_market_stats(&market);
///
/// println!("Market participation: {} votes", stats.total_votes);
/// println!("Total value locked: {} stroops", stats.total_staked);
/// ```
#[contracttype]
#[derive(Clone, Debug)]
pub struct MarketStats {
    pub total_votes: u32,
    pub total_staked: i128,
    pub total_dispute_stakes: i128,
    pub outcome_distribution: Map<String, u32>,
}

/// Statistics for the winning outcome of a resolved market.
///
/// This structure contains detailed information about the winning side
/// of a prediction market, including stake distribution and participant
/// counts. It's essential for calculating individual payouts and
/// understanding market resolution outcomes.
///
/// # Fields
///
/// * `winning_outcome` - The outcome that won the market
/// * `winning_total` - Total amount staked on the winning outcome (in token base units)
/// * `winning_voters` - Number of participants who voted for the winning outcome
/// * `total_pool` - Total amount staked across all outcomes (in token base units)
///
/// # Payout Calculations
///
/// This structure provides the data needed for payout calculations:
/// - Individual payout = (user_stake / winning_total) × total_pool × (1 - fee_rate)
/// - Payout multiplier = total_pool / winning_total
///
/// # Example Usage
///
/// ```rust
/// use crate::markets::MarketAnalytics;
/// use soroban_sdk::{Env, String, Symbol};
///
/// let env = Env::default();
/// let market_id = Symbol::new(&env, "resolved_market");
/// let market = MarketStateManager::get_market(&env, &market_id)?;
/// let winning_outcome = String::from_str(&env, "Yes");
///
/// let winning_stats = MarketAnalytics::calculate_winning_stats(&market, &winning_outcome);
/// let payout_multiplier = winning_stats.total_pool as f64 / winning_stats.winning_total as f64;
///
/// println!("Winners: {} participants", winning_stats.winning_voters);
/// println!("Payout multiplier: {:.2}x", payout_multiplier);
/// ```
#[derive(Clone, Debug)]
pub struct WinningStats {
    pub winning_outcome: String,
    pub winning_total: i128,
    pub winning_voters: u32,
    pub total_pool: i128,
}

/// Individual user participation statistics for a specific market.
///
/// This structure tracks a user's complete involvement in a market,
/// including voting status, stake amounts, dispute participation,
/// and claim status. It's used for user interfaces and determining
/// user eligibility for various market operations.
///
/// # Fields
///
/// * `has_voted` - Whether the user has cast a vote in this market
/// * `stake` - Amount the user staked on their chosen outcome (in token base units)
/// * `dispute_stake` - Amount the user staked in disputes (in token base units)
/// * `has_claimed` - Whether the user has claimed their winnings (if applicable)
/// * `voted_outcome` - The outcome the user voted for (None if hasn't voted)
///
/// # Use Cases
///
/// - **UI Display**: Show user's current position and eligibility
/// - **Access Control**: Determine if user can perform specific actions
/// - **Payout Calculation**: Calculate individual winnings
/// - **Analytics**: Track user engagement patterns
///
/// # Example Usage
///
/// ```rust
/// use crate::markets::MarketAnalytics;
/// use soroban_sdk::{Env, Address, Symbol};
///
/// let env = Env::default();
/// let user = Address::generate(&env);
/// let market_id = Symbol::new(&env, "market_123");
/// let market = MarketStateManager::get_market(&env, &market_id)?;
///
/// let user_stats = MarketAnalytics::get_user_stats(&market, &user);
///
/// if user_stats.has_voted {
///     println!("User voted for: {:?}", user_stats.voted_outcome);
///     println!("Stake: {} stroops", user_stats.stake);
/// }
///
/// if !user_stats.has_claimed && market.winning_outcome.is_some() {
///     println!("User may be eligible to claim winnings");
/// }
/// ```
#[derive(Clone, Debug)]
pub struct UserStats {
    pub has_voted: bool,
    pub stake: i128,
    pub dispute_stake: i128,
    pub has_claimed: bool,
    pub voted_outcome: Option<String>,
}

/// Community consensus analysis for hybrid market resolution.
///
/// This structure represents the collective opinion of market participants,
/// showing which outcome has the strongest community support. It's a crucial
/// component of Predictify's hybrid resolution algorithm that combines
/// oracle data with community wisdom.
///
/// # Fields
///
/// * `outcome` - The outcome with the highest community support
/// * `votes` - Number of votes for the leading outcome
/// * `total_votes` - Total number of votes cast in the market
/// * `percentage` - Percentage of votes for the leading outcome (0-100)
///
/// # Consensus Strength
///
/// The consensus is considered "strong" when:
/// - `percentage` > 50% (majority support)
/// - `total_votes` >= 5 (minimum participation threshold)
///
/// Strong consensus influences the hybrid resolution algorithm by providing
/// a 30% weight against the oracle's 70% weight when they disagree.
///
/// # Example Usage
///
/// ```rust
/// use crate::markets::{MarketAnalytics, MarketUtils};
/// use soroban_sdk::{Env, String, Symbol};
///
/// let env = Env::default();
/// let market_id = Symbol::new(&env, "market_123");
/// let market = MarketStateManager::get_market(&env, &market_id)?;
///
/// let consensus = MarketAnalytics::calculate_community_consensus(&market);
/// let oracle_result = String::from_str(&env, "No");
///
/// // Check consensus strength
/// if consensus.percentage > 50 && consensus.total_votes >= 5 {
///     println!("Strong community consensus: {} ({}%)", consensus.outcome, consensus.percentage);
///     
///     // Apply hybrid resolution
///     let final_result = MarketUtils::determine_final_result(&env, &oracle_result, &consensus);
///     println!("Final result: {}", final_result);
/// } else {
///     println!("Weak consensus, defaulting to oracle result");
/// }
/// ```
#[derive(Clone, Debug)]
#[contracttype]
pub struct CommunityConsensus {
    pub outcome: String,
    pub votes: u32,
    pub total_votes: u32,
    pub percentage: u32,
}

// ===== MARKET TESTING UTILITIES =====

/// Market testing utilities for development, testing, and debugging.
///
/// This struct provides helper functions specifically designed for testing
/// market functionality. These functions create test data, simulate market
/// operations, and provide utilities for unit tests and integration testing.
///
/// **Note**: These functions are intended for testing environments only.
pub struct MarketTestHelpers;

impl MarketTestHelpers {
    /// Creates a standardized test market configuration for testing purposes.
    ///
    /// This function generates a pre-configured market setup with realistic
    /// parameters suitable for testing various market scenarios. It provides
    /// consistent test data across different test cases.
    ///
    /// # Parameters
    ///
    /// * `_env` - The Soroban environment for blockchain operations
    ///
    /// # Returns
    ///
    /// * `MarketCreationParams` - Pre-configured market parameters including:
    ///   - Test admin address
    ///   - Sample prediction question about BTC price
    ///   - Binary outcomes ("yes", "no")
    ///   - 30-day duration
    ///   - Pyth oracle configuration for BTC/USD
    ///   - Standard creation fee (1 XLM)
    ///
    /// # Test Configuration Details
    ///
    /// - **Question**: "Will BTC go above $25,000 by December 31?"
    /// - **Outcomes**: ["yes", "no"]
    /// - **Duration**: 30 days
    /// - **Oracle**: Pyth BTC/USD feed with $25,000 threshold
    /// - **Fee**: 1,000,000 stroops (1 XLM)
    ///
    /// # Example
    ///
    /// ```rust
    /// use soroban_sdk::Env;
    /// use crate::markets::MarketTestHelpers;
    ///
    /// let env = Env::default();
    /// let test_config = MarketTestHelpers::create_test_market_config(&env);
    ///
    /// println!("Test question: {}", test_config.question);
    /// println!("Duration: {} days", test_config.duration_days);
    /// println!("Oracle provider: {:?}", test_config.oracle_config.provider);
    ///
    /// // Use config for testing market creation
    /// // let market_id = MarketCreator::create_market(...);
    /// ```
    pub fn create_test_market_config(_env: &Env) -> MarketCreationParams {
        MarketCreationParams::new(
            Address::from_str(
                _env,
                "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            ),
            String::from_str(_env, "Will BTC go above $25,000 by December 31?"),
            vec![
                _env,
                String::from_str(_env, "yes"),
                String::from_str(_env, "no"),
            ],
            30,
            OracleConfig::new(
                OracleProvider::Pyth,
                String::from_str(_env, "BTC/USD"),
                25_000_00,
                String::from_str(_env, "gt"),
            ),
            1_000_000, // Creation fee: 1 XLM
        )
    }

    /// Creates a complete test market using the standard test configuration.
    ///
    /// This convenience function combines test configuration generation with
    /// actual market creation, providing a one-step solution for creating
    /// test markets in testing environments.
    ///
    /// # Parameters
    ///
    /// * `_env` - The Soroban environment for blockchain operations
    ///
    /// # Returns
    ///
    /// * `Ok(Symbol)` - Unique identifier of the created test market
    /// * `Err(Error)` - Market creation failed
    ///
    /// # Errors
    ///
    /// Same as `MarketCreator::create_market`, including:
    /// * `Error::InsufficientBalance` - Test admin lacks creation fee funds
    /// * `Error::InvalidOracleConfig` - Oracle configuration issues
    ///
    /// # Prerequisites
    ///
    /// - Contract must be properly initialized
    /// - Token configuration must be set
    /// - Test admin must have sufficient balance for creation fee
    ///
    /// # Example
    ///
    /// ```rust
    /// use soroban_sdk::Env;
    /// use crate::markets::{MarketTestHelpers, MarketStateManager};
    ///
    /// let env = Env::default();
    ///
    /// // Create a test market
    /// let market_id = MarketTestHelpers::create_test_market(&env)
    ///     .expect("Test market creation should succeed");
    ///
    /// // Verify market was created
    /// let market = MarketStateManager::get_market(&env, &market_id)
    ///     .expect("Market should exist");
    ///
    /// println!("Created test market: {}", market_id);
    /// println!("Market question: {}", market.question);
    /// ```
    pub fn create_test_market(_env: &Env) -> Result<Symbol, Error> {
        let config = Self::create_test_market_config(_env);

        MarketCreator::create_market(
            _env,
            config.admin,
            config.question,
            config.outcomes,
            config.duration_days,
            config.oracle_config,
        )
    }

    /// Adds a test vote to an existing market with comprehensive validation.
    ///
    /// This function simulates a complete voting process including validation,
    /// token transfer, and market state updates. It's designed for testing
    /// voting scenarios and market participation flows.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `market_id` - Unique identifier of the target market
    /// * `user` - Address of the user placing the vote
    /// * `outcome` - The outcome the user is voting for
    /// * `stake` - Amount to stake on this vote (in token base units)
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Vote added successfully
    /// * `Err(Error)` - Vote addition failed
    ///
    /// # Errors
    ///
    /// * `Error::MarketNotFound` - Market doesn't exist
    /// * `Error::MarketClosed` - Market has expired or is not accepting votes
    /// * `Error::InvalidOutcome` - Outcome is not valid for this market
    /// * `Error::InsufficientStake` - Stake is below minimum (0.1 XLM)
    /// * `Error::InsufficientBalance` - User lacks sufficient token balance
    ///
    /// # Process Flow
    ///
    /// 1. Validates market exists and is accepting votes
    /// 2. Validates outcome is valid for the market
    /// 3. Validates stake meets minimum requirements
    /// 4. Transfers tokens from user to contract
    /// 5. Updates market with user's vote and stake
    /// 6. Saves updated market state
    ///
    /// # Example
    ///
    /// ```rust
    /// use soroban_sdk::{Env, Address, String, Symbol};
    /// use crate::markets::MarketTestHelpers;
    ///
    /// let env = Env::default();
    /// let market_id = Symbol::new(&env, "test_market");
    /// let user = Address::generate(&env);
    /// let outcome = String::from_str(&env, "yes");
    /// let stake = 5_000_000; // 0.5 XLM
    ///
    /// // Add test vote
    /// match MarketTestHelpers::add_test_vote(&env, &market_id, user, outcome, stake) {
    ///     Ok(()) => println!("Test vote added successfully"),
    ///     Err(e) => println!("Vote failed: {:?}", e),
    /// }
    /// ```
    pub fn add_test_vote(
        env: &Env,
        market_id: &Symbol,
        user: Address,
        outcome: String,
        stake: i128,
    ) -> Result<(), Error> {
        let mut market = MarketStateManager::get_market(env, market_id)?;

        MarketValidator::validate_market_for_voting(env, &market)?;
        MarketValidator::validate_outcome(env, &outcome, &market.outcomes)?;
        MarketValidator::validate_stake(stake, 1_000_000)?; // 0.1 XLM minimum

        // Transfer stake
        let token_client = MarketUtils::get_token_client(env)?;
        token_client.transfer(&user, &env.current_contract_address(), &stake);

        // Add vote
        MarketStateManager::add_vote(&mut market, user, outcome, stake, None);
        MarketStateManager::update_market(env, market_id, &market);

        Ok(())
    }

    /// Simulates the complete market resolution process for testing purposes.
    ///
    /// This function provides a comprehensive simulation of market resolution,
    /// including oracle result processing, community consensus calculation,
    /// hybrid algorithm application, and final outcome determination. It's
    /// essential for testing resolution logic and payout calculations.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `market_id` - Unique identifier of the market to resolve
    /// * `oracle_result` - Simulated oracle result for the market
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - Final determined outcome after hybrid resolution
    /// * `Err(Error)` - Resolution simulation failed
    ///
    /// # Errors
    ///
    /// * `Error::MarketNotFound` - Market doesn't exist
    /// * `Error::MarketClosed` - Market hasn't expired yet or is in wrong state
    /// * `Error::OracleUnavailable` - Oracle result processing failed
    ///
    /// # Resolution Process
    ///
    /// 1. Validates market is ready for resolution
    /// 2. Sets the provided oracle result
    /// 3. Calculates community consensus from votes
    /// 4. Applies hybrid resolution algorithm
    /// 5. Sets winning outcome and updates market state
    /// 6. Returns the final determined outcome
    ///
    /// # State Changes
    ///
    /// - Market state transitions to `Resolved`
    /// - `oracle_result` is set
    /// - `winning_outcome` is determined and set
    ///
    /// # Example
    ///
    /// ```rust
    /// use soroban_sdk::{Env, String, Symbol};
    /// use crate::markets::MarketTestHelpers;
    ///
    /// let env = Env::default();
    /// let market_id = Symbol::new(&env, "expired_market");
    /// let oracle_result = String::from_str(&env, "yes");
    ///
    /// // Simulate resolution
    /// match MarketTestHelpers::simulate_market_resolution(&env, &market_id, oracle_result) {
    ///     Ok(final_outcome) => {
    ///         println!("Market resolved with outcome: {}", final_outcome);
    ///         // Proceed with payout calculations
    ///     },
    ///     Err(e) => println!("Resolution failed: {:?}", e),
    /// }
    /// ```
    pub fn simulate_market_resolution(
        env: &Env,
        market_id: &Symbol,
        oracle_result: String,
    ) -> Result<String, Error> {
        let mut market = MarketStateManager::get_market(env, market_id)?;

        MarketValidator::validate_market_for_resolution(env, &market)?;

        // Set oracle result
        MarketStateManager::set_oracle_result(&mut market, oracle_result.clone());

        // Calculate community consensus
        let community_consensus = MarketAnalytics::calculate_community_consensus(&market);

        // Determine final result
        let final_result =
            MarketUtils::determine_final_result(env, &oracle_result, &community_consensus);

        // Set winning outcome
        MarketStateManager::set_winning_outcome(&mut market, final_result.clone(), None);
        MarketStateManager::update_market(env, market_id, &market);

        Ok(final_result)
    }
}

// ===== MARKET STATE LOGIC =====

/// Market state logic and transition management utilities.
///
/// This struct provides comprehensive functions for managing market state
/// transitions, validating state-dependent operations, and ensuring proper
/// market lifecycle management. It enforces business rules and maintains
/// data consistency throughout market operations.
pub struct MarketStateLogic;

impl MarketStateLogic {
    /// Validates that a market state transition is allowed by business rules.
    ///
    /// This function enforces the market state machine by validating that
    /// transitions between states follow the defined business logic. It prevents
    /// invalid state changes that could compromise market integrity.
    ///
    /// # Parameters
    ///
    /// * `from` - Current market state
    /// * `to` - Target market state
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Transition is valid and allowed
    /// * `Err(Error)` - Transition is not allowed
    ///
    /// # Errors
    ///
    /// * `Error::InvalidState` - The requested state transition is not allowed
    ///
    /// # Valid State Transitions
    ///
    /// * `Active` → `Ended`, `Cancelled`, `Closed`, `Disputed`
    /// * `Ended` → `Resolved`, `Disputed`, `Closed`, `Cancelled`
    /// * `Disputed` → `Resolved`, `Closed`, `Cancelled`
    /// * `Resolved` → `Closed`
    /// * `Closed` → (no transitions allowed)
    /// * `Cancelled` → (no transitions allowed)
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::markets::MarketStateLogic;
    /// use crate::types::MarketState;
    ///
    /// // Valid transition
    /// assert!(MarketStateLogic::validate_state_transition(
    ///     MarketState::Active,
    ///     MarketState::Ended
    /// ).is_ok());
    ///
    /// // Invalid transition
    /// assert!(MarketStateLogic::validate_state_transition(
    ///     MarketState::Closed,
    ///     MarketState::Active
    /// ).is_err());
    /// ```
    pub fn validate_state_transition(from: MarketState, to: MarketState) -> Result<(), Error> {
        use MarketState::*;
        let allowed = match from {
            Active => matches!(to, Ended | Cancelled | Closed | Disputed),
            Ended => matches!(to, Resolved | Disputed | Closed | Cancelled),
            Disputed => matches!(to, Resolved | Closed | Cancelled),
            Resolved => matches!(to, Closed),
            Closed => false,
            Cancelled => false,
        };
        if allowed {
            Ok(())
        } else {
            Err(Error::InvalidState)
        }
    }


    /// Check if a function is allowed in the given state

    /// Validates that a specific function can be executed in the given market state.
    ///
    /// This function enforces access control based on market state, ensuring
    /// that operations are only performed when appropriate. It prevents actions
    /// like voting on closed markets or claiming from unresolved markets.
    ///
    /// # Parameters
    ///
    /// * `function` - Name of the function to validate ("vote", "dispute", "resolve", "claim", "close")
    /// * `state` - Current market state to check against
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Function is allowed in the current state
    /// * `Err(Error)` - Function is not allowed in the current state
    ///
    /// # Errors
    ///
    /// * `Error::MarketClosed` - Function is not allowed in the current market state
    ///
    /// # Function Access Rules
    ///
    /// * **vote**: Only allowed in `Active` state
    /// * **dispute**: Only allowed in `Ended` state
    /// * **resolve**: Allowed in `Ended` or `Disputed` states
    /// * **claim**: Only allowed in `Resolved` state
    /// * **close**: Allowed in `Resolved`, `Cancelled`, or `Closed` states
    /// * **other**: All other functions are allowed by default
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::markets::MarketStateLogic;
    /// use crate::types::MarketState;
    ///
    /// // Check if voting is allowed
    /// match MarketStateLogic::check_function_access_for_state("vote", MarketState::Active) {
    ///     Ok(()) => println!("Voting is allowed"),
    ///     Err(_) => println!("Voting is not allowed"),
    /// }
    ///
    /// // Check if claiming is allowed
    /// assert!(MarketStateLogic::check_function_access_for_state(
    ///     "claim",
    ///     MarketState::Resolved
    /// ).is_ok());
    /// ```

    pub fn check_function_access_for_state(
        function: &str,
        state: MarketState,
    ) -> Result<(), Error> {
        use MarketState::*;
        let allowed = match function {
            "vote" => matches!(state, Active),
            "dispute" => matches!(state, Ended),
            "resolve" => matches!(state, Ended | Disputed),
            "claim" => matches!(state, Resolved),
            "close" => matches!(state, Resolved | Cancelled | Closed),
            _ => true, // By default allow
        };
        if allowed {
            Ok(())
        } else {
            Err(Error::MarketClosed)
        }
    }

    /// Emits a blockchain event when a market state changes.
    ///
    /// This function publishes state change events to the blockchain event system,
    /// enabling external systems and user interfaces to track market lifecycle
    /// changes in real-time. Events are essential for monitoring and analytics.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `market_id` - Unique identifier of the market that changed state
    /// * `old_state` - Previous market state
    /// * `new_state` - New market state after transition
    ///
    /// # Event Structure
    ///
    /// The event is published with:
    /// - **Topic**: `("market_state_change", market_id)`
    /// - **Data**: `(old_state, new_state)`
    ///
    /// # Example
    ///
    /// ```rust
    /// use soroban_sdk::{Env, Symbol};
    /// use crate::markets::MarketStateLogic;
    /// use crate::types::MarketState;
    ///
    /// let env = Env::default();
    /// let market_id = Symbol::new(&env, "market_123");
    ///
    /// // Emit state change event
    /// MarketStateLogic::emit_state_change_event(
    ///     &env,
    ///     &market_id,
    ///     MarketState::Active,
    ///     MarketState::Ended
    /// );
    ///
    /// // External systems can now detect this state change
    /// ```
    pub fn emit_state_change_event(
        env: &Env,
        market_id: &Symbol,
        old_state: MarketState,
        new_state: MarketState,
    ) {
        env.events()
            .publish(("market_state_change", market_id), (old_state, new_state));
    }

    /// Validates that a market's state is consistent with its internal data.
    ///
    /// This function performs comprehensive consistency checks to ensure that
    /// the market's state matches its data properties. It helps detect and
    /// prevent data corruption or invalid state combinations.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `market` - Market to validate for state consistency
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Market state is consistent with its data
    /// * `Err(Error)` - Market state is inconsistent
    ///
    /// # Errors
    ///
    /// * `Error::InvalidState` - Market state doesn't match its data properties
    ///
    /// # Consistency Rules
    ///
    /// * **Active**: Must not be expired, must not have winning outcome
    /// * **Ended**: Must be expired, must not have winning outcome
    /// * **Disputed**: Must have dispute stakes
    /// * **Resolved**: Must have winning outcome set
    /// * **Closed/Cancelled**: No specific data requirements
    ///
    /// # Example
    ///
    /// ```rust
    /// use soroban_sdk::{Env, Symbol};
    /// use crate::markets::{MarketStateLogic, MarketStateManager};
    ///
    /// let env = Env::default();
    /// let market_id = Symbol::new(&env, "market_123");
    /// let market = MarketStateManager::get_market(&env, &market_id)?;
    ///
    /// // Validate state consistency
    /// match MarketStateLogic::validate_market_state_consistency(&env, &market) {
    ///     Ok(()) => println!("Market state is consistent"),
    ///     Err(e) => println!("State inconsistency detected: {:?}", e),
    /// }
    /// ```
    pub fn validate_market_state_consistency(env: &Env, market: &Market) -> Result<(), Error> {
        use MarketState::*;
        let now = env.ledger().timestamp();
        match market.state {
            Active => {
                if market.end_time <= now {
                    return Err(Error::InvalidState);
                }
                if market.winning_outcome.is_some() {
                    return Err(Error::InvalidState);
                }
            }
            Ended => {
                if market.end_time > now {
                    return Err(Error::InvalidState);
                }
                if market.winning_outcome.is_some() {
                    return Err(Error::InvalidState);
                }
            }
            Disputed => {
                if market.dispute_stakes.is_empty() {
                    return Err(Error::InvalidState);
                }
            }
            Resolved => {
                if market.winning_outcome.is_none() {
                    return Err(Error::InvalidState);
                }
            }
            Closed | Cancelled => {}
        }
        Ok(())
    }

    /// Retrieves the current state of a market by its identifier.
    ///
    /// This convenience function fetches a market from storage and returns
    /// its current state. It's useful for quick state checks without loading
    /// the entire market data structure.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `market_id` - Unique identifier of the market
    ///
    /// # Returns
    ///
    /// * `Ok(MarketState)` - Current state of the market
    /// * `Err(Error)` - Market not found or access error
    ///
    /// # Errors
    ///
    /// * `Error::MarketNotFound` - No market exists with the specified ID
    ///
    /// # Example
    ///
    /// ```rust
    /// use soroban_sdk::{Env, Symbol};
    /// use crate::markets::MarketStateLogic;
    /// use crate::types::MarketState;
    ///
    /// let env = Env::default();
    /// let market_id = Symbol::new(&env, "market_123");
    ///
    /// match MarketStateLogic::get_market_state(&env, &market_id) {
    ///     Ok(state) => {
    ///         match state {
    ///             MarketState::Active => println!("Market is accepting votes"),
    ///             MarketState::Ended => println!("Market has ended, awaiting resolution"),
    ///             MarketState::Resolved => println!("Market is resolved, users can claim"),
    ///             _ => println!("Market state: {:?}", state),
    ///         }
    ///     },
    ///     Err(e) => println!("Could not get market state: {:?}", e),
    /// }
    /// ```
    pub fn get_market_state(env: &Env, market_id: &Symbol) -> Result<MarketState, Error> {
        let market = MarketStateManager::get_market(env, market_id)?;
        Ok(market.state)
    }

    /// Checks if a market can transition to a specific target state.
    ///
    /// This function determines whether a state transition is possible for
    /// a given market by checking the current state against the target state
    /// using the state transition validation rules.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `market_id` - Unique identifier of the market
    /// * `target_state` - Desired state to transition to
    ///
    /// # Returns
    ///
    /// * `Ok(bool)` - `true` if transition is allowed, `false` if not
    /// * `Err(Error)` - Market not found or access error
    ///
    /// # Errors
    ///
    /// * `Error::MarketNotFound` - No market exists with the specified ID
    ///
    /// # Example
    ///
    /// ```rust
    /// use soroban_sdk::{Env, Symbol};
    /// use crate::markets::MarketStateLogic;
    /// use crate::types::MarketState;
    ///
    /// let env = Env::default();
    /// let market_id = Symbol::new(&env, "market_123");
    ///
    /// // Check if market can be resolved
    /// match MarketStateLogic::can_transition_to_state(&env, &market_id, MarketState::Resolved) {
    ///     Ok(true) => println!("Market can be resolved"),
    ///     Ok(false) => println!("Market cannot be resolved yet"),
    ///     Err(e) => println!("Error checking transition: {:?}", e),
    /// }
    ///
    /// // Check if market can be closed
    /// let can_close = MarketStateLogic::can_transition_to_state(
    ///     &env,
    ///     &market_id,
    ///     MarketState::Closed
    /// )?;
    ///
    /// if can_close {
    ///     println!("Market is ready to be closed");
    /// }
    /// ```
    pub fn can_transition_to_state(
        env: &Env,
        market_id: &Symbol,
        target_state: MarketState,
    ) -> Result<bool, Error> {
        let market = MarketStateManager::get_market(env, market_id)?;
        Ok(MarketStateLogic::validate_state_transition(market.state, target_state).is_ok())
    }
}

// ===== MODULE TESTS =====

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn test_market_validation() {
        let env = Env::default();

        // Test valid market params
        let valid_question = String::from_str(&env, "Test question?");
        let valid_outcomes = vec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ];

        assert!(MarketValidator::validate_market_params(
            &env,
            &valid_question,
            &valid_outcomes,
            30
        )
        .is_ok());

        // Test invalid question
        let invalid_question = String::from_str(&env, "");
        assert!(MarketValidator::validate_market_params(
            &env,
            &invalid_question,
            &valid_outcomes,
            30
        )
        .is_err());

        // Test invalid outcomes
        let invalid_outcomes = vec![&env, String::from_str(&env, "yes")];
        assert!(MarketValidator::validate_market_params(
            &env,
            &valid_question,
            &invalid_outcomes,
            30
        )
        .is_err());

        // Test invalid duration
        assert!(
            MarketValidator::validate_market_params(&env, &valid_question, &valid_outcomes, 0)
                .is_err()
        );
        assert!(MarketValidator::validate_market_params(
            &env,
            &valid_question,
            &valid_outcomes,
            400
        )
        .is_err());
    }

    #[test]
    fn test_market_utils() {
        let env = Env::default();

        // Test end time calculation
        let current_time = env.ledger().timestamp();
        let end_time = MarketUtils::calculate_end_time(&env, 30);
        assert_eq!(end_time, current_time + 30 * 24 * 60 * 60);

        // Test payout calculation
        let payout = MarketUtils::calculate_payout(1000, 5000, 10000, 2).unwrap();
        assert_eq!(payout, 1960); // (1000 * 98 / 100) * 10000 / 5000

        // Test payout with zero winning total
        assert!(MarketUtils::calculate_payout(1000, 0, 10000, 2).is_err());
    }

    #[test]
    fn test_market_analytics() {
        let env = Env::default();

        // Create a test market
        let market = Market::new(
            &env,
            Address::generate(&env),
            String::from_str(&env, "Test?"),
            vec![
                &env,
                String::from_str(&env, "yes"),
                String::from_str(&env, "no"),
            ],
            env.ledger().timestamp() + 86400,
            OracleConfig::new(
                OracleProvider::Pyth,
                String::from_str(&env, "BTC/USD"),
                25_000_00,
                String::from_str(&env, "gt"),
            ),
            MarketState::Active,
        );

        // Test market stats
        let stats = MarketAnalytics::get_market_stats(&market);
        assert_eq!(stats.total_votes, 0);
        assert_eq!(stats.total_staked, 0);

        // Test community consensus with no votes
        let consensus = MarketAnalytics::calculate_community_consensus(&market);
        assert_eq!(consensus.total_votes, 0);
        assert_eq!(consensus.percentage, 0);
    }
}
