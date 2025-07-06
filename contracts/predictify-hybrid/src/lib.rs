#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, panic_with_error, symbol_short, token, vec, Address, Env,
    IntoVal, Map, String, Symbol, Vec,
};

// Error management module
pub mod errors;
use errors::Error;

// Types module
pub mod types;
use types::*;

// Oracle management module
pub mod oracles;
use oracles::{OracleFactory, OracleInstance, OracleInterface, OracleUtils};

// Market management module
pub mod markets;
use markets::{MarketAnalytics, MarketCreator, MarketStateManager, MarketUtils, MarketValidator};

// Voting management module
pub mod voting;
use voting::{VotingAnalytics, VotingManager, VotingUtils, VotingValidator};

// Dispute management module
pub mod disputes;
use disputes::{DisputeAnalytics, DisputeManager, DisputeUtils, DisputeValidator};

// Extension management module
pub mod extensions;
use extensions::{ExtensionManager, ExtensionUtils, ExtensionValidator};
use types::ExtensionStats;

// Fee management module
pub mod fees;
use fees::{FeeManager, FeeCalculator, FeeValidator, FeeUtils, FeeTracker, FeeConfigManager};
use resolution::{OracleResolutionManager, MarketResolutionManager, MarketResolutionAnalytics, OracleResolutionAnalytics, ResolutionUtils};

// Configuration management module
pub mod config;
use config::{ConfigManager, ConfigValidator, ConfigUtils, ContractConfig, Environment};

// Utility functions module
pub mod utils;
use utils::{TimeUtils, StringUtils, NumericUtils, ValidationUtils, ConversionUtils, CommonUtils, TestingUtils};

pub mod resolution;

#[contract]
pub struct PredictifyHybrid;

#[contractimpl]
impl PredictifyHybrid {
    pub fn initialize(env: Env, admin: Address) {
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, "Admin"), &admin);
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
        errors::helpers::require_admin(&env, &admin, &stored_admin);

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
        match FeeManager::update_fee_config(&env, admin, new_config) {
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
        match resolution::MarketResolutionManager::finalize_market(&env, &admin, &market_id, &outcome) {
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
            Ok(market) => resolution::ResolutionUtils::calculate_resolution_time(&env, &market),
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
        admin.require_auth();

        // Verify admin
        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, "Admin"))
            .expect("Admin not set");

        // Use error helper for admin validation
        errors::helpers::require_admin(&env, &admin, &stored_admin);

        // Remove market from storage
        MarketStateManager::remove_market(&env, &market_id);
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
        admin.require_auth();

        // Verify admin
        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, "Admin"))
            .expect("Admin not set");

        // Use error helper for admin validation
        errors::helpers::require_admin(&env, &admin, &stored_admin);

        match ExtensionManager::extend_market_duration(
            &env,
            admin,
            market_id,
            additional_days,
            reason,
        ) {
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
        ExtensionManager::calculate_extension_fee(additional_days)
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
        match DisputeManager::get_dispute_votes(&env, dispute_id) {
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
        // Set admin
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, "Admin"), &admin);

        // Initialize configuration based on environment
        let config = match environment {
            Environment::Development => ConfigManager::get_development_config(&env),
            Environment::Testnet => ConfigManager::get_testnet_config(&env),
            Environment::Mainnet => ConfigManager::get_mainnet_config(&env),
            Environment::Custom => ConfigManager::get_development_config(&env), // Default to development for custom
        };

        // Store configuration
        match ConfigManager::store_config(&env, &config) {
            Ok(_) => (),
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
    pub fn update_contract_config(env: Env, admin: Address, new_config: ContractConfig) -> ContractConfig {
        // Verify admin permissions
        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, "Admin"))
            .unwrap_or_else(|| panic!("Admin not set"));

        errors::helpers::require_admin(&env, &admin, &stored_admin);

        // Validate new configuration
        match ConfigValidator::validate_contract_config(&new_config) {
            Ok(_) => (),
            Err(e) => panic_with_error!(env, e),
        }

        // Store updated configuration
        match ConfigManager::update_config(&env, &new_config) {
            Ok(_) => new_config,
            Err(e) => panic_with_error!(env, e),
        }
    }

    /// Reset configuration to defaults
    pub fn reset_config_to_defaults(env: Env, admin: Address) -> ContractConfig {
        // Verify admin permissions
        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, "Admin"))
            .unwrap_or_else(|| panic!("Admin not set"));

        errors::helpers::require_admin(&env, &admin, &stored_admin);

        // Reset to defaults
        match ConfigManager::reset_to_defaults(&env) {
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
}
mod test;
