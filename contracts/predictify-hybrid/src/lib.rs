#![no_std]
extern crate alloc;
use soroban_sdk::{
    contract, contractimpl, panic_with_error, vec, Address, Env, Map, String, Symbol, Vec, symbol_short,
};
use alloc::string::ToString;

// ===== MODULE ORGANIZATION =====
// Predictify Hybrid Contract - Organized Module Structure
// 
// This contract provides a comprehensive prediction market system with:
// - Oracle integration for automated market resolution
// - Community voting and consensus mechanisms
// - Dispute resolution and escalation systems
// - Fee management and analytics
// - Admin controls and configuration management
// - Event logging and monitoring
// - Validation and security systems

// ===== MODULE DECLARATIONS =====

/// Error handling and management module
pub mod errors;
use errors::Error;

/// Core data types and structures module
pub mod types;
use types::*;

/// Oracle integration and management module
pub mod oracles;

/// Market creation and state management module
pub mod markets;
use markets::{MarketCreator, MarketStateManager};

/// Voting system and consensus module
pub mod voting;
use voting::VotingManager;

/// Dispute resolution and escalation module
pub mod disputes;
use disputes::DisputeManager;

/// Market resolution and analytics module
pub mod resolution;
use resolution::{OracleResolutionManager, MarketResolutionManager};

/// Fee calculation and management module
pub mod fees;
use fees::FeeManager;

/// Configuration management module
pub mod config;
use config::{ConfigManager, ConfigUtils, ConfigValidator, ContractConfig, Environment};

/// Utility functions and helpers module
pub mod utils;
use utils::{TimeUtils, StringUtils, NumericUtils, ValidationUtils, CommonUtils};

/// Event logging and monitoring module
pub mod events;
use events::{EventLogger, EventHelpers, EventTestingUtils, EventDocumentation};

/// Admin controls and functions module
pub mod admin;
use admin::{AdminInitializer, AdminFunctions};

/// Market extensions and modifications module
pub mod extensions;
use extensions::{ExtensionManager, ExtensionUtils, ExtensionValidator};

/// Input validation and security module
pub mod validation;
use validation::{
    ValidationResult, InputValidator, 
    MarketValidator as ValidationMarketValidator, 
    OracleValidator as ValidationOracleValidator,
    FeeValidator as ValidationFeeValidator, 
    VoteValidator as ValidationVoteValidator, 
    DisputeValidator as ValidationDisputeValidator, 
    ComprehensiveValidator, ValidationDocumentation,
};

#[contract]
pub struct PredictifyHybrid;

#[contractimpl]
impl PredictifyHybrid {
    pub fn initialize(env: Env, admin: Address) {
        match AdminInitializer::initialize(&env, &admin) {
            Ok(_) => (), // Success
            Err(e) => panic_with_error!(env, e),
        }
    }

    // Create a market using the markets module
    pub fn create_market(
        env: Env,
        admin: Address,
        question: String,
        outcomes: Vec<String>,
        duration_days: u32,
        oracle_config: OracleConfig,
    ) -> Symbol {
        // Authenticate that the caller is the admin
        admin.require_auth();

        // Verify the caller is an admin
        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, "Admin"))
            .unwrap_or_else(|| {
                panic!("Admin not set");
            });

        // Use error helper for admin validation
        let _ = errors::helpers::require_admin(&env, &admin, &stored_admin);

        // Use the markets module to create the market
        match MarketCreator::create_market(
            &env,
            admin.clone(),
            question,
            outcomes,
            duration_days,
            oracle_config,
        ) {
            Ok(market_id) => {
                // Process creation fee using the fee management system
                match FeeManager::process_creation_fee(&env, &admin) {
                    Ok(_) => market_id,
                    Err(e) => panic_with_error!(env, e),
                }
            }
            Err(e) => panic_with_error!(env, e),
        }
    }

    // Distribute winnings to users
    pub fn claim_winnings(env: Env, user: Address, market_id: Symbol) {
        match VotingManager::process_claim(&env, user, market_id) {
            Ok(_) => (), // Success
            Err(e) => panic_with_error!(env, e),
        }
    }

    // Collect platform fees
    pub fn collect_fees(env: Env, admin: Address, market_id: Symbol) {
        match FeeManager::collect_fees(&env, admin, market_id) {
            Ok(_) => (), // Success
            Err(e) => panic_with_error!(env, e),
        }
    }

    // Get fee analytics
    pub fn get_fee_analytics(env: Env) -> fees::FeeAnalytics {
        match FeeManager::get_fee_analytics(&env) {
            Ok(analytics) => analytics,
            Err(e) => panic_with_error!(env, e),
        }
    }

    // Update fee configuration (admin only)
    pub fn update_fee_config(env: Env, admin: Address, new_config: fees::FeeConfig) -> fees::FeeConfig {
        match AdminFunctions::update_fee_config(&env, &admin, &new_config) {
            Ok(config) => config,
            Err(e) => panic_with_error!(env, e),
        }
    }

    // Get current fee configuration
    pub fn get_fee_config(env: Env) -> fees::FeeConfig {
        match FeeManager::get_fee_config(&env) {
            Ok(config) => config,
            Err(e) => panic_with_error!(env, e),
        }
    }

    // Validate market fees
    pub fn validate_market_fees(env: Env, market_id: Symbol) -> fees::FeeValidationResult {
        match FeeManager::validate_market_fees(&env, &market_id) {
            Ok(result) => result,
            Err(e) => panic_with_error!(env, e),
        }
    }

    // Finalize market after disputes
    pub fn finalize_market(env: Env, admin: Address, market_id: Symbol, outcome: String) {
        match AdminFunctions::finalize_market(&env, &admin, &market_id, &outcome) {
            Ok(_) => (), // Success
            Err(e) => panic_with_error!(env, e),
        }
    }

    // Allows users to vote on a market outcome by staking tokens
    pub fn vote(env: Env, user: Address, market_id: Symbol, outcome: String, stake: i128) {
        match VotingManager::process_vote(&env, user, market_id, outcome, stake) {
            Ok(_) => (), // Success
            Err(e) => panic_with_error!(env, e),
        }
    }

    // Fetch oracle result to determine market outcome
    pub fn fetch_oracle_result(env: Env, market_id: Symbol, oracle_contract: Address) -> String {
        match resolution::OracleResolutionManager::fetch_oracle_result(&env, &market_id, &oracle_contract) {
            Ok(resolution) => resolution.oracle_result,
            Err(e) => panic_with_error!(env, e),
        }
    }

    // Allows users to dispute the market result by staking tokens
    pub fn dispute_result(env: Env, user: Address, market_id: Symbol, stake: i128) {
        match DisputeManager::process_dispute(&env, user, market_id, stake, None) {
            Ok(_) => (), // Success
            Err(e) => panic_with_error!(env, e),
        }
    }

    // Resolves a market by combining oracle results and community votes
    pub fn resolve_market(env: Env, market_id: Symbol) -> String {
        match resolution::MarketResolutionManager::resolve_market(&env, &market_id) {
            Ok(resolution) => resolution.final_outcome,
            Err(e) => panic_with_error!(env, e),
        }
    }

    // Resolve a dispute and determine final market outcome
    pub fn resolve_dispute(env: Env, admin: Address, market_id: Symbol) -> String {
        match DisputeManager::resolve_dispute(&env, market_id, admin) {
            Ok(resolution) => resolution.final_outcome,
            Err(e) => panic_with_error!(env, e),
        }
    }

    // ===== RESOLUTION SYSTEM METHODS =====

    // Get oracle resolution for a market
    pub fn get_oracle_resolution(env: Env, market_id: Symbol) -> Option<resolution::OracleResolution> {
        match OracleResolutionManager::get_oracle_resolution(&env, &market_id) {
            Ok(resolution) => resolution,
            Err(_) => None,
        }
    }

    // Get market resolution for a market
    pub fn get_market_resolution(env: Env, market_id: Symbol) -> Option<resolution::MarketResolution> {
        match MarketResolutionManager::get_market_resolution(&env, &market_id) {
            Ok(resolution) => resolution,
            Err(_) => None,
        }
    }

    // Get resolution analytics
    pub fn get_resolution_analytics(env: Env) -> resolution::ResolutionAnalytics {
        match resolution::MarketResolutionAnalytics::calculate_resolution_analytics(&env) {
            Ok(analytics) => analytics,
            Err(_) => resolution::ResolutionAnalytics::default(),
        }
    }

    // Get oracle statistics
    pub fn get_oracle_stats(env: Env) -> resolution::OracleStats {
        match resolution::OracleResolutionAnalytics::get_oracle_stats(&env) {
            Ok(stats) => stats,
            Err(_) => resolution::OracleStats::default(),
        }
    }

    // Validate resolution for a market
    pub fn validate_resolution(env: Env, market_id: Symbol) -> resolution::ResolutionValidation {
        let mut validation = resolution::ResolutionValidation {
            is_valid: true,
            errors: vec![&env],
            warnings: vec![&env],
            recommendations: vec![&env],
        };

        // Get market
        let market = match MarketStateManager::get_market(&env, &market_id) {
            Ok(market) => market,
            Err(_) => {
                validation.is_valid = false;
                validation.errors.push_back(String::from_str(&env, "Market not found"));
                return validation;
            }
        };

        // Check resolution state
        let state = resolution::ResolutionUtils::get_resolution_state(&env, &market);
        let (eligible, reason) = resolution::ResolutionUtils::get_resolution_eligibility(&env, &market);

        if !eligible {
            validation.is_valid = false;
            validation.errors.push_back(reason);
        }

        // Add recommendations based on state
        match state {
            resolution::ResolutionState::Active => {
                validation.recommendations.push_back(String::from_str(&env, "Market is active, wait for end time"));
            }
            resolution::ResolutionState::OracleResolved => {
                validation.recommendations.push_back(String::from_str(&env, "Oracle resolved, ready for market resolution"));
            }
            resolution::ResolutionState::MarketResolved => {
                validation.recommendations.push_back(String::from_str(&env, "Market already resolved"));
            }
            resolution::ResolutionState::Disputed => {
                validation.recommendations.push_back(String::from_str(&env, "Resolution disputed, consider admin override"));
            }
            resolution::ResolutionState::Finalized => {
                validation.recommendations.push_back(String::from_str(&env, "Resolution finalized"));
            }
        }

        validation
    }

    // Get resolution state for a market
    pub fn get_resolution_state(env: Env, market_id: Symbol) -> resolution::ResolutionState {
        match MarketStateManager::get_market(&env, &market_id) {
            Ok(market) => resolution::ResolutionUtils::get_resolution_state(&env, &market),
            Err(_) => resolution::ResolutionState::Active,
        }
    }

    // Check if market can be resolved
    pub fn can_resolve_market(env: Env, market_id: Symbol) -> bool {
        match MarketStateManager::get_market(&env, &market_id) {
            Ok(market) => resolution::ResolutionUtils::can_resolve_market(&env, &market),
            Err(_) => false,
        }
    }

    // Calculate resolution time for a market
    pub fn calculate_resolution_time(env: Env, market_id: Symbol) -> u64 {
        match MarketStateManager::get_market(&env, &market_id) {
            Ok(market) => {
                let current_time = env.ledger().timestamp();
                TimeUtils::time_difference(current_time, market.end_time)
            },
            Err(_) => 0,
        }
    }

    // Get dispute statistics for a market
    pub fn get_dispute_stats(env: Env, market_id: Symbol) -> disputes::DisputeStats {
        match DisputeManager::get_dispute_stats(&env, market_id) {
            Ok(stats) => stats,
            Err(e) => panic_with_error!(env, e),
        }
    }

    // Get all disputes for a market
    pub fn get_market_disputes(env: Env, market_id: Symbol) -> Vec<disputes::Dispute> {
        match DisputeManager::get_market_disputes(&env, market_id) {
            Ok(disputes) => disputes,
            Err(e) => panic_with_error!(env, e),
        }
    }

    // Check if user has disputed a market
    pub fn has_user_disputed(env: Env, market_id: Symbol, user: Address) -> bool {
        match DisputeManager::has_user_disputed(&env, market_id, user) {
            Ok(has_disputed) => has_disputed,
            Err(_) => false,
        }
    }

    // Get user's dispute stake for a market
    pub fn get_user_dispute_stake(env: Env, market_id: Symbol, user: Address) -> i128 {
        match DisputeManager::get_user_dispute_stake(&env, market_id, user) {
            Ok(stake) => stake,
            Err(_) => 0,
        }
    }

    // Clean up market storage
    pub fn close_market(env: Env, admin: Address, market_id: Symbol) {
        match AdminFunctions::close_market(&env, &admin, &market_id) {
            Ok(_) => (), // Success
            Err(e) => panic_with_error!(env, e),
        }
    }

    // Helper function to create a market with Reflector oracle
    pub fn create_reflector_market(
        env: Env,
        admin: Address,
        question: String,
        outcomes: Vec<String>,
        duration_days: u32,
        asset_symbol: String,
        threshold: i128,
        comparison: String,
    ) -> Symbol {
        match MarketCreator::create_reflector_market(
            &env,
            admin,
            question,
            outcomes,
            duration_days,
            asset_symbol,
            threshold,
            comparison,
        ) {
            Ok(market_id) => market_id,
            Err(e) => panic_with_error!(env, e),
        }
    }

    // Helper function to create a market with Pyth oracle
    pub fn create_pyth_market(
        env: Env,
        admin: Address,
        question: String,
        outcomes: Vec<String>,
        duration_days: u32,
        feed_id: String,
        threshold: i128,
        comparison: String,
    ) -> Symbol {
        match MarketCreator::create_pyth_market(
            &env,
            admin,
            question,
            outcomes,
            duration_days,
            feed_id,
            threshold,
            comparison,
        ) {
            Ok(market_id) => market_id,
            Err(e) => panic_with_error!(env, e),
        }
    }

    // Helper function to create a market with Reflector oracle for specific assets
    pub fn create_reflector_asset_market(
        env: Env,
        admin: Address,
        question: String,
        outcomes: Vec<String>,
        duration_days: u32,
        asset_symbol: String, // e.g., "BTC", "ETH", "XLM"
        threshold: i128,
        comparison: String,
    ) -> Symbol {
        match MarketCreator::create_reflector_asset_market(
            &env,
            admin,
            question,
            outcomes,
            duration_days,
            asset_symbol,
            threshold,
            comparison,
        ) {
            Ok(market_id) => market_id,
            Err(e) => panic_with_error!(env, e),
        }
    }

    // ===== MARKET EXTENSION FUNCTIONS =====

    /// Extend market duration with validation and fee handling
    pub fn extend_market_duration(
        env: Env,
        admin: Address,
        market_id: Symbol,
        additional_days: u32,
        reason: String,
    ) {
        match AdminFunctions::extend_market_duration(&env, &admin, &market_id, additional_days, &reason) {
            Ok(_) => (), // Success
            Err(e) => panic_with_error!(env, e),
        }
    }

    /// Validate extension conditions for a market
    pub fn validate_extension_conditions(
        env: Env,
        market_id: Symbol,
        additional_days: u32,
    ) -> bool {
        match ExtensionValidator::validate_extension_conditions(&env, &market_id, additional_days) {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    /// Check extension limits for a market
    pub fn check_extension_limits(env: Env, market_id: Symbol, additional_days: u32) -> bool {
        match ExtensionValidator::check_extension_limits(&env, &market_id, additional_days) {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    /// Emit extension event for monitoring
    pub fn emit_extension_event(env: Env, market_id: Symbol, additional_days: u32, admin: Address) {
        ExtensionUtils::emit_extension_event(&env, &market_id, additional_days, &admin);
    }

    /// Get market extension history
    pub fn get_market_extension_history(
        env: Env,
        market_id: Symbol,
    ) -> Vec<types::MarketExtension> {
        match ExtensionManager::get_market_extension_history(&env, market_id) {
            Ok(history) => history,
            Err(_) => vec![&env],
        }
    }

    /// Check if admin can extend market
    pub fn can_extend_market(env: Env, market_id: Symbol, admin: Address) -> bool {
        match ExtensionManager::can_extend_market(&env, market_id, admin) {
            Ok(can_extend) => can_extend,
            Err(_) => false,
        }
    }

    /// Handle extension fees
    pub fn handle_extension_fees(env: Env, market_id: Symbol, additional_days: u32) -> i128 {
        match ExtensionUtils::handle_extension_fees(&env, &market_id, additional_days) {
            Ok(fee_amount) => fee_amount,
            Err(_) => 0,
        }
    }

    /// Get extension statistics for a market
    pub fn get_extension_stats(env: Env, market_id: Symbol) -> ExtensionStats {
        match ExtensionManager::get_extension_stats(&env, market_id) {
            Ok(stats) => stats,
            Err(_) => ExtensionStats {
                total_extensions: 0,
                total_extension_days: 0,
                max_extension_days: 30,
                can_extend: false,
                extension_fee_per_day: 100_000_000,
            },
        }
    }

    /// Calculate extension fee for given days
    pub fn calculate_extension_fee(additional_days: u32) -> i128 {
        // Use numeric utilities for fee calculation
        let base_fee = 100_000_000; // 10 XLM base fee
        let fee_per_day = 10_000_000; // 1 XLM per day
        NumericUtils::clamp(
            &(base_fee + (fee_per_day * additional_days as i128)),
            &100_000_000, // Minimum fee
            &1_000_000_000 // Maximum fee
        )
    }

    // ===== DISPUTE RESOLUTION FUNCTIONS =====

    /// Vote on a dispute
    pub fn vote_on_dispute(
        env: Env,
        user: Address,
        market_id: Symbol,
        dispute_id: Symbol,
        vote: bool,
        stake: i128,
        reason: Option<String>,
    ) {
        user.require_auth();

        match DisputeManager::vote_on_dispute(&env, user, market_id, dispute_id, vote, stake, reason) {
            Ok(_) => (), // Success
            Err(e) => panic_with_error!(env, e),
        }
    }

    /// Calculate dispute outcome based on voting
    pub fn calculate_dispute_outcome(env: Env, dispute_id: Symbol) -> bool {
        match DisputeManager::calculate_dispute_outcome(&env, dispute_id) {
            Ok(outcome) => outcome,
            Err(_) => false,
        }
    }

    /// Distribute dispute fees to winners
    pub fn distribute_dispute_fees(env: Env, dispute_id: Symbol) -> disputes::DisputeFeeDistribution {
        match DisputeManager::distribute_dispute_fees(&env, dispute_id) {
            Ok(distribution) => distribution,
            Err(_) => disputes::DisputeFeeDistribution {
                dispute_id: symbol_short!("error"),
                total_fees: 0,
                winner_stake: 0,
                loser_stake: 0,
                winner_addresses: vec![&env],
                distribution_timestamp: 0,
                fees_distributed: false,
            },
        }
    }

    /// Escalate a dispute
    pub fn escalate_dispute(
        env: Env,
        user: Address,
        dispute_id: Symbol,
        reason: String,
    ) -> disputes::DisputeEscalation {
        user.require_auth();

        match DisputeManager::escalate_dispute(&env, user, dispute_id, reason) {
            Ok(escalation) => escalation,
            Err(_) => {
                let default_address = env.storage()
                    .persistent()
                    .get(&Symbol::new(&env, "Admin"))
                    .unwrap_or_else(|| panic!("Admin not set"));
                disputes::DisputeEscalation {
                    dispute_id: symbol_short!("error"),
                    escalated_by: default_address,
                    escalation_reason: String::from_str(&env, "Error"),
                    escalation_timestamp: 0,
                    escalation_level: 0,
                    requires_admin_review: false,
                }
            },
        }
    }

    /// Get dispute votes
    pub fn get_dispute_votes(env: Env, dispute_id: Symbol) -> Vec<disputes::DisputeVote> {
        match DisputeManager::get_dispute_votes(&env, &dispute_id) {
            Ok(votes) => votes,
            Err(_) => vec![&env],
        }
    }

    /// Validate dispute resolution conditions
    pub fn validate_dispute_resolution(env: Env, dispute_id: Symbol) -> bool {
        match DisputeManager::validate_dispute_resolution_conditions(&env, dispute_id) {
            Ok(valid) => valid,
            Err(_) => false,
        }
    }

    // ===== DYNAMIC THRESHOLD FUNCTIONS =====

    /// Calculate dynamic dispute threshold for a market
    pub fn calculate_dispute_threshold(env: Env, market_id: Symbol) -> voting::DisputeThreshold {
        match VotingManager::calculate_dispute_threshold(&env, market_id) {
            Ok(threshold) => threshold,
            Err(_) => voting::DisputeThreshold {
                market_id: symbol_short!("error"),
                base_threshold: 10_000_000,
                adjusted_threshold: 10_000_000,
                market_size_factor: 0,
                activity_factor: 0,
                complexity_factor: 0,
                timestamp: 0,
            },
        }
    }

    /// Adjust threshold by market size
    pub fn adjust_threshold_by_market_size(env: Env, market_id: Symbol, base_threshold: i128) -> i128 {
        match voting::ThresholdUtils::adjust_threshold_by_market_size(&env, &market_id, base_threshold) {
            Ok(adjustment) => adjustment,
            Err(_) => 0,
        }
    }

    /// Modify threshold by activity level
    pub fn modify_threshold_by_activity(env: Env, market_id: Symbol, activity_level: u32) -> i128 {
        match voting::ThresholdUtils::modify_threshold_by_activity(&env, &market_id, activity_level) {
            Ok(adjustment) => adjustment,
            Err(_) => 0,
        }
    }

    /// Validate dispute threshold
    pub fn validate_dispute_threshold(threshold: i128, market_id: Symbol) -> bool {
        match voting::ThresholdUtils::validate_dispute_threshold(threshold, &market_id) {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    /// Get threshold adjustment factors
    pub fn get_threshold_adjustment_factors(env: Env, market_id: Symbol) -> voting::ThresholdAdjustmentFactors {
        match voting::ThresholdUtils::get_threshold_adjustment_factors(&env, &market_id) {
            Ok(factors) => factors,
            Err(_) => voting::ThresholdAdjustmentFactors {
                market_size_factor: 0,
                activity_factor: 0,
                complexity_factor: 0,
                total_adjustment: 0,
            },
        }
    }

    /// Update dispute thresholds (admin only)
    pub fn update_dispute_thresholds(
        env: Env,
        admin: Address,
        market_id: Symbol,
        new_threshold: i128,
        reason: String,
    ) -> voting::DisputeThreshold {
        admin.require_auth();

        match VotingManager::update_dispute_thresholds(&env, admin, market_id, new_threshold, reason) {
            Ok(threshold) => threshold,
            Err(_) => voting::DisputeThreshold {
                market_id: symbol_short!("error"),
                base_threshold: 10_000_000,
                adjusted_threshold: 10_000_000,
                market_size_factor: 0,
                activity_factor: 0,
                complexity_factor: 0,
                timestamp: 0,
            },
        }
    }

    /// Get threshold history for a market
    pub fn get_threshold_history(env: Env, market_id: Symbol) -> Vec<voting::ThresholdHistoryEntry> {
        match VotingManager::get_threshold_history(&env, market_id) {
            Ok(history) => history,
            Err(_) => vec![&env],
        }
    }

    // ===== CONFIGURATION MANAGEMENT METHODS =====

    /// Initialize contract with configuration
    pub fn initialize_with_config(env: Env, admin: Address, environment: Environment) {
        match AdminInitializer::initialize_with_config(&env, &admin, &environment) {
            Ok(_) => (), // Success
            Err(e) => panic_with_error!(env, e),
        }
    }

    /// Get current contract configuration
    pub fn get_contract_config(env: Env) -> ContractConfig {
        match ConfigManager::get_config(&env) {
            Ok(config) => config,
            Err(_) => ConfigManager::get_development_config(&env), // Return default if not found
        }
    }

    /// Update contract configuration (admin only)
    pub fn update_contract_config(env: Env, admin: Address, new_config: ContractConfig) -> Result<(), Error> {
        match AdminFunctions::update_contract_config(&env, &admin, &new_config) {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    /// Reset configuration to defaults
    pub fn reset_config_to_defaults(env: Env, admin: Address) -> ContractConfig {
        match AdminFunctions::reset_config_to_defaults(&env, &admin) {
            Ok(config) => config,
            Err(e) => panic_with_error!(env, e),
        }
    }

    /// Get configuration summary
    pub fn get_config_summary(env: Env) -> String {
        let config = match ConfigManager::get_config(&env) {
            Ok(config) => config,
            Err(_) => ConfigManager::get_development_config(&env),
        };
        ConfigUtils::get_config_summary(&config)
    }

    /// Check if fees are enabled
    pub fn fees_enabled(env: Env) -> bool {
        let config = match ConfigManager::get_config(&env) {
            Ok(config) => config,
            Err(_) => ConfigManager::get_development_config(&env),
        };
        ConfigUtils::fees_enabled(&config)
    }

    /// Get environment type
    pub fn get_environment(env: Env) -> Environment {
        let config = match ConfigManager::get_config(&env) {
            Ok(config) => config,
            Err(_) => ConfigManager::get_development_config(&env),
        };
        config.network.environment
    }

    /// Validate configuration
    pub fn validate_configuration(env: Env) -> bool {
        let config = match ConfigManager::get_config(&env) {
            Ok(config) => config,
            Err(_) => return false,
        };
        ConfigValidator::validate_contract_config(&config).is_ok()
    }

    // ===== UTILITY-BASED METHODS =====

    /// Format duration in human-readable format
    pub fn format_duration(env: Env, seconds: u64) -> String {
        TimeUtils::format_duration(&env, seconds)
    }

    /// Calculate percentage with custom denominator
    pub fn calculate_percentage(percentage: i128, value: i128, denominator: i128) -> i128 {
        NumericUtils::calculate_percentage(&percentage, &value, &denominator)
    }

    /// Validate string length
    pub fn validate_string_length(s: String, min_length: u32, max_length: u32) -> bool {
        StringUtils::validate_string_length(&s, min_length, max_length).is_ok()
    }

    /// Sanitize string
    pub fn sanitize_string(env: Env, s: String) -> String {
        StringUtils::sanitize_string(&env, &s)
    }

    /// Convert number to string
    pub fn number_to_string(env: Env, value: i128) -> String {
        NumericUtils::i128_to_string(&env, &value)
    }

    /// Convert string to number
    pub fn string_to_number(s: String) -> i128 {
        NumericUtils::string_to_i128(&s)
    }

    /// Generate unique ID
    pub fn generate_unique_id(env: Env, prefix: String) -> String {
        CommonUtils::generate_unique_id(&env, &prefix)
    }

    /// Compare addresses for equality
    pub fn addresses_equal(a: Address, b: Address) -> bool {
        CommonUtils::addresses_equal(&a, &b)
    }

    /// Compare strings ignoring case
    pub fn strings_equal_ignore_case(a: String, b: String) -> bool {
        CommonUtils::strings_equal_ignore_case(&a, &b)
    }

    /// Calculate weighted average
    pub fn calculate_weighted_average(values: Vec<i128>, weights: Vec<i128>) -> i128 {
        CommonUtils::calculate_weighted_average(&values, &weights)
    }

    /// Calculate simple interest
    pub fn calculate_simple_interest(principal: i128, rate: i128, periods: i128) -> i128 {
        CommonUtils::calculate_simple_interest(&principal, &rate, &periods)
    }

    /// Round to nearest multiple
    pub fn round_to_nearest(value: i128, multiple: i128) -> i128 {
        NumericUtils::round_to_nearest(&value, &multiple)
    }

    /// Clamp value between min and max
    pub fn clamp_value(value: i128, min: i128, max: i128) -> i128 {
        NumericUtils::clamp(&value, &min, &max)
    }

    /// Check if value is within range
    pub fn is_within_range(value: i128, min: i128, max: i128) -> bool {
        NumericUtils::is_within_range(&value, &min, &max)
    }

    /// Calculate absolute difference
    pub fn abs_difference(a: i128, b: i128) -> i128 {
        NumericUtils::abs_difference(&a, &b)
    }

    /// Calculate square root
    pub fn sqrt(value: i128) -> i128 {
        NumericUtils::sqrt(&value)
    }

    /// Validate positive number
    pub fn validate_positive_number(value: i128) -> bool {
        ValidationUtils::validate_positive_number(&value)
    }

    /// Validate number range
    pub fn validate_number_range(value: i128, min: i128, max: i128) -> bool {
        ValidationUtils::validate_number_range(&value, &min, &max)
    }

    /// Validate future timestamp
    pub fn validate_future_timestamp(env: Env, timestamp: u64) -> bool {
        ValidationUtils::validate_future_timestamp(&env, &timestamp)
    }

    /// Get time utilities information
    pub fn get_time_utilities(env: Env) -> String {
        let current_time = env.ledger().timestamp();
        let mut s = alloc::string::String::new();
        s.push_str("Current time: ");
        s.push_str(&current_time.to_string());
        s.push_str(", Days to seconds: 86400");
        String::from_str(&env, &s)
    }

    // ===== EVENT-BASED METHODS =====

    /// Get market events
    pub fn get_market_events(env: Env, market_id: Symbol) -> Vec<events::MarketEventSummary> {
        EventLogger::get_market_events(&env, &market_id)
    }

    /// Get recent events
    pub fn get_recent_events(env: Env, limit: u32) -> Vec<events::EventSummary> {
        EventLogger::get_recent_events(&env, limit)
    }

    /// Get error events
    pub fn get_error_events(env: Env) -> Vec<events::ErrorLoggedEvent> {
        EventLogger::get_error_events(&env)
    }

    /// Get performance metrics
    pub fn get_performance_metrics(env: Env) -> Vec<events::PerformanceMetricEvent> {
        EventLogger::get_performance_metrics(&env)
    }

    /// Clear old events
    pub fn clear_old_events(env: Env, older_than_timestamp: u64) {
        EventLogger::clear_old_events(&env, older_than_timestamp);
    }

    /// Validate event structure
    pub fn validate_event_structure(_env: Env, event_type: String, _event_data: String) -> bool {
        match event_type.to_string().as_str() {
            "MarketCreated" => {
                // In a real implementation, you would deserialize and validate
                true
            }
            "VoteCast" => true,
            "OracleResult" => true,
            "MarketResolved" => true,
            "DisputeCreated" => true,
            "DisputeResolved" => true,
            "FeeCollected" => true,
            "ExtensionRequested" => true,
            "ConfigUpdated" => true,
            "ErrorLogged" => true,
            "PerformanceMetric" => true,
            _ => false,
        }
    }

    /// Get event documentation
    pub fn get_event_documentation(env: Env) -> Map<String, String> {
        EventDocumentation::get_event_type_docs(&env)
    }

    /// Get event usage examples
    pub fn get_event_usage_examples(env: Env) -> Map<String, String> {
        EventDocumentation::get_usage_examples(&env)
    }

    /// Get event system overview
    pub fn get_event_system_overview(env: Env) -> String {
        EventDocumentation::get_overview(&env)
    }

    /// Create test event
    pub fn create_test_event(env: Env, event_type: String) -> bool {
        EventTestingUtils::simulate_event_emission(&env, &event_type)
    }

    /// Validate test event structure
    pub fn validate_test_event(env: Env, event_type: String) -> bool {
        match event_type.to_string().as_str() {
            "MarketCreated" => {
                let test_event = EventTestingUtils::create_test_market_created_event(
                    &env,
                    &Symbol::new(&env, "test"),
                    &Address::from_str(&env, "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF"),
                );
                EventTestingUtils::validate_test_event_structure(&test_event).is_ok()
            }
            "VoteCast" => {
                let test_event = EventTestingUtils::create_test_vote_cast_event(
                    &env,
                    &Symbol::new(&env, "test"),
                    &Address::from_str(&env, "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF"),
                );
                EventTestingUtils::validate_test_event_structure(&test_event).is_ok()
            }
            "OracleResult" => {
                let test_event = EventTestingUtils::create_test_oracle_result_event(
                    &env,
                    &Symbol::new(&env, "test"),
                );
                EventTestingUtils::validate_test_event_structure(&test_event).is_ok()
            }
            "MarketResolved" => {
                let test_event = EventTestingUtils::create_test_market_resolved_event(
                    &env,
                    &Symbol::new(&env, "test"),
                );
                EventTestingUtils::validate_test_event_structure(&test_event).is_ok()
            }
            "DisputeCreated" => {
                let test_event = EventTestingUtils::create_test_dispute_created_event(
                    &env,
                    &Symbol::new(&env, "test"),
                    &Address::from_str(&env, "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF"),
                );
                EventTestingUtils::validate_test_event_structure(&test_event).is_ok()
            }
            "FeeCollected" => {
                let test_event = EventTestingUtils::create_test_fee_collected_event(
                    &env,
                    &Symbol::new(&env, "test"),
                    &Address::from_str(&env, "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF"),
                );
                EventTestingUtils::validate_test_event_structure(&test_event).is_ok()
            }
            "ErrorLogged" => {
                let test_event = EventTestingUtils::create_test_error_logged_event(&env);
                EventTestingUtils::validate_test_event_structure(&test_event).is_ok()
            }
            "PerformanceMetric" => {
                let test_event = EventTestingUtils::create_test_performance_metric_event(&env);
                EventTestingUtils::validate_test_event_structure(&test_event).is_ok()
            }
            _ => false,
        }
    }

    /// Get event age in seconds
    pub fn get_event_age(env: Env, event_timestamp: u64) -> u64 {
        let current_timestamp = env.ledger().timestamp();
        EventHelpers::get_event_age(current_timestamp, event_timestamp)
    }

    /// Check if event is recent
    pub fn is_recent_event(env: Env, event_timestamp: u64, recent_threshold: u64) -> bool {
        let current_timestamp = env.ledger().timestamp();
        EventHelpers::is_recent_event(event_timestamp, current_timestamp, recent_threshold)
    }

    /// Format event timestamp
    pub fn format_event_timestamp(env: Env, timestamp: u64) -> String {
        EventHelpers::format_timestamp(&env, timestamp)
    }

    /// Create event context
    pub fn create_event_context(env: Env, context_parts: Vec<String>) -> String {
        EventHelpers::create_event_context(&env, &context_parts)
    }

    /// Validate event timestamp
    pub fn validate_event_timestamp(timestamp: u64) -> bool {
        EventHelpers::is_valid_timestamp(timestamp)
    }

    // ===== VALIDATION METHODS =====

    /// Validate input parameters for market creation
    pub fn validate_market_creation_inputs(
        env: Env,
        admin: Address,
        question: String,
        outcomes: Vec<String>,
        duration_days: u32,
        oracle_config: OracleConfig,
    ) -> ValidationResult {
        ComprehensiveValidator::validate_complete_market_creation(
            &env, &admin, &question, &outcomes, &duration_days, &oracle_config
        )
    }

    /// Validate market state
    pub fn validate_market_state(env: Env, market_id: Symbol) -> ValidationResult {
        if let Some(market) = env.storage().persistent().get::<Symbol, Market>(&market_id) {
            ComprehensiveValidator::validate_market_state(&env, &market, &market_id)
        } else {
            ValidationResult::invalid()
        }
    }

    /// Validate vote parameters
    pub fn validate_vote_inputs(
        env: Env,
        user: Address,
        market_id: Symbol,
        outcome: String,
        stake_amount: i128,
    ) -> ValidationResult {
        let mut result = ValidationResult::valid();
        
        // Validate user address
        if let Err(_error) = InputValidator::validate_address(&env, &user) {
            result.add_error();
        }
        
        // Validate outcome string
        if let Err(_error) = InputValidator::validate_string(&env, &outcome, 1, 100) {
            result.add_error();
        }
        
        // Validate stake amount
        if let Err(_error) = ValidationVoteValidator::validate_stake_amount(&stake_amount) {
            result.add_error();
        }
        
        // Validate market exists and is valid for voting
        if let Some(market) = env.storage().persistent().get::<Symbol, Market>(&market_id) {
            if let Err(_error) = ValidationMarketValidator::validate_market_for_voting(&env, &market, &market_id) {
                result.add_error();
            }
            
            // Validate outcome against market outcomes
            if let Err(_error) = ValidationVoteValidator::validate_outcome(&env, &outcome, &market.outcomes) {
                result.add_error();
            }
        } else {
            result.add_error();
        }
        
        result
    }

    /// Validate oracle configuration
    pub fn validate_oracle_config(env: Env, oracle_config: OracleConfig) -> ValidationResult {
        let mut result = ValidationResult::valid();
        
        if let Err(_error) = ValidationOracleValidator::validate_oracle_config(&env, &oracle_config) {
            result.add_error();
        }
        
        result
    }

    /// Validate fee configuration
    pub fn validate_fee_config(
        env: Env,
        platform_fee_percentage: i128,
        creation_fee: i128,
        min_fee_amount: i128,
        max_fee_amount: i128,
        collection_threshold: i128,
    ) -> ValidationResult {
        ValidationFeeValidator::validate_fee_config(
            &env, &platform_fee_percentage, &creation_fee, &min_fee_amount, &max_fee_amount, &collection_threshold
        )
    }

    /// Validate dispute creation
    pub fn validate_dispute_creation(
        env: Env,
        user: Address,
        market_id: Symbol,
        dispute_stake: i128,
    ) -> ValidationResult {
        let mut result = ValidationResult::valid();
        
        // Validate user address
        if let Err(_error) = InputValidator::validate_address(&env, &user) {
            result.add_error();
        }
        
        // Validate dispute stake
        if let Err(_error) = ValidationDisputeValidator::validate_dispute_stake(&dispute_stake) {
            result.add_error();
        }
        
        // Validate market exists and is resolved
        if let Some(market) = env.storage().persistent().get::<Symbol, Market>(&market_id) {
            if let Err(_error) = ValidationMarketValidator::validate_market_for_fee_collection(&env, &market, &market_id) {
                result.add_error();
            }
        } else {
            result.add_error();
        }
        
        result
    }

    /// Get validation rules documentation
    pub fn get_validation_rules(env: Env) -> Map<String, String> {
        ValidationDocumentation::get_validation_rules(&env)
    }

    /// Get validation error codes
    pub fn get_validation_error_codes(env: Env) -> Map<String, String> {
        ValidationDocumentation::get_validation_error_codes(&env)
    }

    /// Get validation system overview
    pub fn get_validation_overview(env: Env) -> String {
        ValidationDocumentation::get_validation_overview(&env)
    }

    /// Test validation utilities
    pub fn test_validation_utilities(env: Env) -> ValidationResult {
        validation::ValidationTestingUtils::create_test_validation_result(&env)
    }
}
mod test;
