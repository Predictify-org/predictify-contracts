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

#[contract]
pub struct PredictifyHybrid;

const PERCENTAGE_DENOMINATOR: i128 = 100;

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
            admin,
            question,
            outcomes,
            duration_days,
            oracle_config,
        ) {
            Ok(market_id) => market_id,
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
        match VotingManager::collect_fees(&env, admin, market_id) {
            Ok(_) => (), // Success
            Err(e) => panic_with_error!(env, e),
        }
    }

    // Finalize market after disputes
    pub fn finalize_market(env: Env, admin: Address, market_id: Symbol, outcome: String) {
        admin.require_auth();

        // Verify admin
        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, "Admin"))
            .expect("Admin not set");

        // Use error helper for admin validation
        errors::helpers::require_admin(&env, &admin, &stored_admin);

        let mut market: Market = env
            .storage()
            .persistent()
            .get(&market_id)
            .expect("Market not found");

        // Use error helper for outcome validation
        errors::helpers::require_valid_outcome(&env, &outcome, &market.outcomes);

        // Set final outcome
        market.winning_outcome = Some(outcome);
        env.storage().persistent().set(&market_id, &market);
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
        // Get the market from storage
        let mut market: Market = env
            .storage()
            .persistent()
            .get(&market_id)
            .unwrap_or_else(|| {
                panic!("Market not found");
            });

        // Check if the market has already been resolved
        if market.oracle_result.is_some() {
            panic_with_error!(env, Error::MarketAlreadyResolved);
        }

        // Check if the market ended (we can only fetch oracle result after market ends)
        let current_time = env.ledger().timestamp();
        if current_time < market.end_time {
            panic_with_error!(env, Error::MarketClosed);
        }

        // Get the price from the appropriate oracle using the factory pattern
        let oracle = match OracleFactory::create_oracle(
            market.oracle_config.provider.clone(),
            oracle_contract,
        ) {
            Ok(oracle) => oracle,
            Err(e) => panic_with_error!(env, e),
        };

        let price = match oracle.get_price(&env, &market.oracle_config.feed_id) {
            Ok(p) => p,
            Err(e) => panic_with_error!(env, e),
        };

        // Determine the outcome based on the price and threshold using OracleUtils
        let outcome = match OracleUtils::determine_outcome(
            price,
            market.oracle_config.threshold,
            &market.oracle_config.comparison,
            &env,
        ) {
            Ok(result) => result,
            Err(e) => panic_with_error!(env, e),
        };

        // Store the result in the market
        market.oracle_result = Some(outcome.clone());

        // Update the market in storage
        env.storage().persistent().set(&market_id, &market);

        // Return the outcome
        outcome
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
        // Get the market from storage
        let mut market = match MarketStateManager::get_market(&env, &market_id) {
            Ok(market) => market,
            Err(e) => panic_with_error!(env, e),
        };

        // Validate market for resolution
        if let Err(e) = MarketValidator::validate_market_for_resolution(&env, &market) {
            panic_with_error!(env, e);
        }

        // Retrieve the oracle result
        let oracle_result = match &market.oracle_result {
            Some(result) => result.clone(),
            None => panic_with_error!(env, Error::OracleUnavailable),
        };

        // Calculate community consensus
        let community_consensus = MarketAnalytics::calculate_community_consensus(&market);

        // Determine final result using hybrid algorithm
        let final_result =
            MarketUtils::determine_final_result(&env, &oracle_result, &community_consensus);

        // Set winning outcome
        MarketStateManager::set_winning_outcome(&mut market, final_result.clone());

        // Update the market in storage
        MarketStateManager::update_market(&env, &market_id, &market);

        // Return the final result
        final_result
    }

    // Resolve a dispute and determine final market outcome
    pub fn resolve_dispute(env: Env, admin: Address, market_id: Symbol) -> String {
        match DisputeManager::resolve_dispute(&env, market_id, admin) {
            Ok(resolution) => resolution.final_outcome,
            Err(e) => panic_with_error!(env, e),
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
}
mod test;
