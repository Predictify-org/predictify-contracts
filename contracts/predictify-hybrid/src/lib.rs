#![no_std]



// Module declarations
mod errors;
mod types;
mod markets;
mod voting;
mod oracles;
mod disputes;
// Temporarily disabled advanced modules until they are fully implemented
// mod config;
// mod events;
// mod extensions;
// mod fees;
// mod resolution;
// mod utils;
// mod validation;


// Re-export commonly used items
pub use errors::Error;
pub use types::*;

use soroban_sdk::{
    contract, contractimpl, panic_with_error, token, Address, Env,
    Map, String, Symbol, Vec,
};

// Import oracle implementations from the oracles module
use oracles::{OracleFactory, OracleInterface};

// Import from disputes module
use disputes::DisputeManager;

// Import from other modules (simplified for now)
// use config::{ConfigManager, ConfigValidator, ConfigUtils, ContractConfig, Environment};
// use events::{EventLogger, EventDocumentation, EventTestingUtils, EventHelpers};
// use extensions::{ExtensionManager, ExtensionValidator, ExtensionUtils};
// use types::ExtensionStats;
// use fees::{FeeManager};
// use markets::{MarketStateManager};
// use resolution::{MarketResolutionManager};
// use utils::{TimeUtils, NumericUtils, StringUtils, CommonUtils, ValidationUtils};
// use validation::{ComprehensiveValidator, InputValidator, ValidationVoteValidator, ValidationMarketValidator, ValidationOracleValidator, ValidationFeeValidator, ValidationDisputeValidator, ValidationDocumentation, ValidationResult};
// use voting::{VotingManager};


#[contract]
pub struct PredictifyHybrid;


const PERCENTAGE_DENOMINATOR: i128 = 100;
const FEE_PERCENTAGE: i128 = 2; // 2% fee for the platform


#[contractimpl]
impl PredictifyHybrid {
    pub fn initialize(env: Env, admin: Address) {
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, "Admin"), &admin);
    }

    // Create a market (we need to add this function for the vote function to work with)
    pub fn create_market(
        env: Env,
        admin: Address,
        question: String,
        outcomes: Vec<String>,
        duration_days: u32,
        oracle_config: OracleConfig, // Add oracle config parameter
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

        if admin != stored_admin {
            panic_with_error!(env, Error::Unauthorized);
        }


        // Validate inputs
        if outcomes.len() < 2 {
            panic!("At least two outcomes are required");

        }

        if question.len() == 0 {
            panic!("Question cannot be empty");
        }

        // Generate a unique market ID using timestamp and a counter
        let counter_key = Symbol::new(&env, "MarketCounter");
        let counter: u32 = env.storage().persistent().get(&counter_key).unwrap_or(0);
        let new_counter = counter + 1;
        env.storage().persistent().set(&counter_key, &new_counter);

        // Create a unique market ID using the counter
        let market_id = Symbol::new(&env, "market");

        // Calculate end time based on duration_days (convert days to seconds)
        let seconds_per_day: u64 = 24 * 60 * 60; // 24 hours * 60 minutes * 60 seconds
        let duration_seconds: u64 = (duration_days as u64) * seconds_per_day;
        let end_time: u64 = env.ledger().timestamp() + duration_seconds;

        // Create a new market
        let market = Market {
            admin: admin.clone(),
            question,
            outcomes,
            end_time,
            oracle_config, // Use the provided oracle config
            oracle_result: None,
            votes: Map::new(&env),
            total_staked: 0,
            dispute_stakes: Map::new(&env),
            stakes: Map::new(&env),
            claimed: Map::new(&env),
            winning_outcome: None,
            fee_collected: false, // Initialize fee collection state
        };

        // Deduct 1 XLM fee from the admin
        let fee_amount: i128 = 10_000_000; // 1 XLM = 10,000,000 stroops

        // Get a token client for the native asset
        // In a real implementation, you would use the actual token contract ID
        let token_id: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, "TokenID"))
            .unwrap_or_else(|| {
                panic!("Token ID not set");
            });
        let token_client = token::Client::new(&env, &token_id);

        // Transfer the fee from admin to the contract
        token_client.transfer(&admin, &env.current_contract_address(), &fee_amount);

        // Store the market
        env.storage().persistent().set(&market_id, &market);

        // Return the market ID
        market_id
    }

    // NEW: Distribute winnings to users
    pub fn claim_winnings(env: Env, user: Address, market_id: Symbol) {
        user.require_auth();

        let mut market: Market = env
            .storage()
            .persistent()
            .get(&market_id)
            .expect("Market not found");

        // Check if user has claimed already
        if market.claimed.get(user.clone()).unwrap_or(false) {
            panic_with_error!(env, Error::AlreadyClaimed);
        }

        // Check if market is resolved
        let winning_outcome = match &market.winning_outcome {
            Some(outcome) => outcome,
            None => panic_with_error!(env, Error::MarketNotResolved),
        };

        // Get user's vote and stake
        let user_outcome = market
            .votes
            .get(user.clone())
            .unwrap_or_else(|| panic_with_error!(env, Error::NothingToClaim));

        let user_stake = market.stakes.get(user.clone()).unwrap_or(0);

        // Calculate payout if user won
        if &user_outcome == winning_outcome {
            // Calculate total winning stakes
            let mut winning_total = 0;
            for (voter, outcome) in market.votes.iter() {
                if &outcome == winning_outcome {
                    winning_total += market.stakes.get(voter.clone()).unwrap_or(0);
                }
            }

            // Calculate user's share (minus fee percentage)
            let user_share =
                (user_stake * (PERCENTAGE_DENOMINATOR - FEE_PERCENTAGE)) / PERCENTAGE_DENOMINATOR;
            let total_pool = market.total_staked;

            // Ensure winning_total is non-zero
            if winning_total == 0 {
                panic_with_error!(env, Error::NothingToClaim);
            }
            let payout = (user_share * total_pool) / winning_total;

            // Get token client
            let token_id = env
                .storage()
                .persistent()
                .get(&Symbol::new(&env, "TokenID"))
                .expect("Token contract not set");

            let token_client = token::Client::new(&env, &token_id);

            // Transfer winnings to user
            token_client.transfer(&env.current_contract_address(), &user, &payout);
        }

        // Mark as claimed
        market.claimed.set(user.clone(), true);
        env.storage().persistent().set(&market_id, &market);
    }

    // NEW: Collect platform fees
    pub fn collect_fees(env: Env, admin: Address, market_id: Symbol) {

        admin.require_auth();

        let market: Market = env
            .storage()
            .persistent()
            .get(&market_id)
            .expect("Market not found");

        // Verify admin
        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, "Admin"))
            .expect("Admin not set");

        if admin != stored_admin {
            panic_with_error!(env, Error::Unauthorized);
        }

        // Check if fees already collected
        if market.fee_collected {
            panic_with_error!(env, Error::AlreadyClaimed);

        }

        // Calculate 2% fee
        let fee = (market.total_staked * 2) / 100;

        // Get token client
        let token_id = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, "TokenID"))
            .expect("Token contract not set");

        let token_client = token::Client::new(&env, &token_id);

        // Transfer fee to admin
        token_client.transfer(&env.current_contract_address(), &admin, &fee);

        // Update market state
        let mut market = market;
        market.fee_collected = true;
        env.storage().persistent().set(&market_id, &market);
    }

    // // Get fee analytics (temporarily disabled)
    // pub fn get_fee_analytics(env: Env) -> fees::FeeAnalytics {
    //     match FeeManager::get_fee_analytics(&env) {
    //         Ok(analytics) => analytics,
    //         Err(e) => panic_with_error!(env, e),
    //     }
    // }

    // // Get current fee configuration (temporarily disabled)
    // pub fn get_fee_config(env: Env) -> fees::FeeConfig {
    //     match FeeManager::get_fee_config(&env) {
    //         Ok(config) => config,
    //         Err(e) => panic_with_error!(env, e),
    //     }
    // }


    // // Finalize market after disputes (temporarily disabled)
    // pub fn finalize_market(env: Env, admin: Address, market_id: Symbol, outcome: String) {
    //     match resolution::MarketResolutionManager::finalize_market(&env, &admin, &market_id, &outcome) {
    //         Ok(_) => (), // Success
    //         Err(e) => panic_with_error!(env, e),
    //     }
    // }

    // Allows users to vote on a market outcome by staking tokens
    pub fn vote(env: Env, user: Address, market_id: Symbol, outcome: String, stake: i128) {
        // Require authentication from the user
        user.require_auth();

        // Get the market from storage
        let mut market: Market = env
            .storage()
            .persistent()
            .get(&market_id)
            .unwrap_or_else(|| {
                panic!("Market not found");
            });

        // Check if the market is still active
        if env.ledger().timestamp() >= market.end_time {
            panic_with_error!(env, Error::MarketClosed);
        }

        // Validate that the chosen outcome is valid
        let outcome_exists = market.outcomes.iter().any(|o| o == outcome);
        if !outcome_exists {
            panic!("Invalid outcome");
        }

        // Define the token contract to use for staking
        let token_id = env
            .storage()
            .persistent()
            .get::<Symbol, Address>(&Symbol::new(&env, "TokenID"))
            .unwrap_or_else(|| {
                panic!("Token contract not set");
            });

        // Create a client for the token contract
        let token_client = token::Client::new(&env, &token_id);

        // Transfer the staked amount from the user to this contract
        token_client.transfer(&user, &env.current_contract_address(), &stake);

        // Store the vote in the market
        market.votes.set(user.clone(), outcome);

        // Store the user's stake
        market.stakes.set(user.clone(), stake);

        // Update the total staked amount
        market.total_staked += stake;

        // Update the market in storage
        env.storage().persistent().set(&market_id, &market);
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

        // Get the price from the appropriate oracle based on provider
        let price = match market.oracle_config.provider {
            OracleProvider::Pyth => {
                let oracle = OracleFactory::create_pyth_oracle(oracle_contract);
                match oracle.get_price(&env, &market.oracle_config.feed_id) {
                    Ok(p) => p,
                    Err(e) => panic_with_error!(env, e),
                }
            }
            OracleProvider::Reflector => {
                let oracle = OracleFactory::create_reflector_oracle(oracle_contract);
                match oracle.get_price(&env, &market.oracle_config.feed_id) {
                    Ok(p) => p,
                    Err(e) => panic_with_error!(env, e),
                }
            }
            OracleProvider::BandProtocol | OracleProvider::DIA => {
                panic_with_error!(env, Error::InvalidOracleConfig);
            }
        };

        // Determine the outcome based on the price and threshold
        let outcome = if market.oracle_config.comparison == String::from_str(&env, "gt") {
            if price > market.oracle_config.threshold {
                String::from_str(&env, "yes")
            } else {
                String::from_str(&env, "no")
            }
        } else if market.oracle_config.comparison == String::from_str(&env, "lt") {
            if price < market.oracle_config.threshold {
                String::from_str(&env, "yes")
            } else {
                String::from_str(&env, "no")
            }
        } else if market.oracle_config.comparison == String::from_str(&env, "eq") {
            if price == market.oracle_config.threshold {
                String::from_str(&env, "yes")
            } else {
                String::from_str(&env, "no")
            }
        } else {
            panic_with_error!(env, Error::InvalidOracleConfig);
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
        // Require authentication from the user
        user.require_auth();

        // Get the market from storage
        let mut market: Market = env
            .storage()
            .persistent()
            .get(&market_id)
            .unwrap_or_else(|| {
                panic!("Market not found");
            });

        // Ensure disputes are only possible after the market ends
        let current_time = env.ledger().timestamp();
        if current_time < market.end_time {
            panic!("Cannot dispute before market ends");
        }

        // Require a minimum stake (10 XLM) to raise a dispute
        let min_stake: i128 = 10_0000000; // 10 XLM (in stroops, 1 XLM = 10^7 stroops)
        if stake < min_stake {
            panic_with_error!(env, Error::InsufficientStake);
        }

        // Define the token contract to use for staking
        let token_id = env
            .storage()
            .persistent()
            .get::<Symbol, Address>(&Symbol::new(&env, "TokenID"))
            .unwrap_or_else(|| {
                panic!("Token contract not set");
            });

        // Create a client for the token contract
        let token_client = token::Client::new(&env, &token_id);

        // Transfer the stake from the user to the contract
        token_client.transfer(&user, &env.current_contract_address(), &stake);

        // Store the dispute stake in the market
        if let Some(existing_stake) = market.dispute_stakes.get(user.clone()) {
            market
                .dispute_stakes
                .set(user.clone(), existing_stake + stake);
        } else {
            market.dispute_stakes.set(user.clone(), stake);
        }

        // Extend the market end time by 24 hours during a dispute (if not already extended)
        let dispute_extension = 24 * 60 * 60; // 24 hours in seconds
        if market.end_time < current_time + dispute_extension {
            market.end_time = current_time + dispute_extension;
        }

        // Update the market in storage
        env.storage().persistent().set(&market_id, &market);
    }

    // Resolves a market by combining oracle results and community votes
    pub fn resolve_market(env: Env, market_id: Symbol) -> String {
        // Get the market from storage
        let mut market: Market = env
            .storage()
            .persistent()
            .get(&market_id)
            .unwrap_or_else(|| {
                panic!("Market not found");
            });

        // Check if the market end time has passed
        let current_time = env.ledger().timestamp();
        if current_time < market.end_time {
            panic_with_error!(env, Error::MarketClosed);
        }

        // Retrieve the oracle result (or fail if unavailable)
        let oracle_result = match &market.oracle_result {
            Some(result) => result.clone(),
            None => panic_with_error!(env, Error::OracleUnavailable),
        };

        // Count community votes for each outcome
        let mut vote_counts: Map<String, u32> = Map::new(&env);
        for (_, outcome) in market.votes.iter() {
            let count = vote_counts.get(outcome.clone()).unwrap_or(0);
            vote_counts.set(outcome.clone(), count + 1);
        }

        // Find the community consensus (outcome with most votes)
        let mut community_result = oracle_result.clone(); // Default to oracle result if no votes
        let mut max_votes = 0;

        for (outcome, count) in vote_counts.iter() {
            if count > max_votes {
                max_votes = count;
                community_result = outcome.clone();
            }
        }

        // Calculate the final result with weights: 70% oracle, 30% community
        let final_result = if oracle_result == community_result {
            // If both agree, use that outcome
            oracle_result
        } else {
            // If they disagree, check if community votes are significant
            let total_votes: u32 = vote_counts
                .values()
                .into_iter()
                .fold(0, |acc, count| acc + count);

            if total_votes == 0 {
                // No community votes, use oracle result
                oracle_result
            } else {
                // Use integer-based calculation to determine if community consensus is strong
                // Check if the winning vote has more than 50% of total votes
                if max_votes * 100 > total_votes * 50 && total_votes >= 5 {
                    // Apply 70-30 weighting using integer arithmetic
                    // We'll use a scale of 0-100 for percentage calculation

                    // Generate a pseudo-random number by combining timestamp and ledger sequence
                    let timestamp = env.ledger().timestamp();
                    let sequence = env.ledger().sequence();
                    let combined = timestamp as u128 + sequence as u128;
                    let random_value = (combined % 100) as u32;

                    // If random_value is less than 30 (representing 30% weight),
                    // choose community result
                    if random_value < 30 {
                        community_result
                    } else {
                        oracle_result
                    }
                } else {
                    // Not enough community consensus, use oracle result
                    oracle_result
                }
            }
        };

        // Calculate winning outcome
        market.winning_outcome = Some(final_result.clone());

        // Calculate total for winning outcome
        let mut _winning_total = 0;
        for (user, outcome) in market.votes.iter() {
            if outcome == final_result {
                _winning_total += market.stakes.get(user.clone()).unwrap_or(0);
            }
        }

        // Record the final result in the market
        market.oracle_result = Some(final_result.clone());

        // Update the market in storage
        env.storage().persistent().set(&market_id, &market);

        // Return the final result
        final_result
    }

    // // Resolution functionality temporarily disabled
    // pub fn get_market_resolution(env: Env, market_id: Symbol) -> Option<resolution::MarketResolution> {
    //     // Implementation pending
    //     None
    // }

    // // Get resolution analytics (temporarily disabled)
    // pub fn get_resolution_analytics(env: Env) -> resolution::ResolutionAnalytics {
    //     // Implementation pending
    // }

    // // Get resolution state (temporarily disabled)
    // pub fn get_resolution_state(env: Env, market_id: Symbol) -> resolution::ResolutionState {
    //     // Implementation pending
    // }

    // // Check if market can be resolved (temporarily disabled)
    // pub fn can_resolve_market(env: Env, market_id: Symbol) -> bool {
    //     // Implementation pending
    //     false
    // }

    // // Calculate resolution time (temporarily disabled)
    // pub fn calculate_resolution_time(env: Env, market_id: Symbol) -> u64 {
    //     // Implementation pending
    //     0
    // }

    // // Advanced features temporarily disabled
    // pub fn get_dispute_stats(env: Env, market_id: Symbol) -> disputes::DisputeStats {
    //     // Implementation pending
    // }

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

        if admin != stored_admin {
            panic_with_error!(env, Error::Unauthorized);
        }

        // Remove market from storage
        env.storage().persistent().remove(&market_id);
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

        // Create Reflector oracle configuration
        let oracle_config = OracleConfig {
            provider: OracleProvider::Reflector,
            feed_id: asset_symbol, // Use asset symbol as feed_id
            threshold,
            comparison,
        };

        // Call the main create_market function
        Self::create_market(env, admin, question, outcomes, duration_days, oracle_config)

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

        // Create Pyth oracle configuration
        let oracle_config = OracleConfig {
            provider: OracleProvider::Pyth,
            feed_id,
            threshold,
            comparison,
        };

        // Call the main create_market function
        Self::create_market(env, admin, question, outcomes, duration_days, oracle_config)

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

        // Create Reflector oracle configuration
        let oracle_config = OracleConfig {
            provider: OracleProvider::Reflector,
            feed_id: asset_symbol, // Use asset symbol as feed_id
            threshold,
            comparison,
        };

        // Call the main create_market function
        Self::create_market(env, admin, question, outcomes, duration_days, oracle_config)

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

    // ===== UTILITY-BASED METHODS =====

    /// Format duration in human-readable format
    pub fn format_duration(seconds: u64) -> String {
        TimeUtils::format_duration(seconds)
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
    pub fn sanitize_string(s: String) -> String {
        StringUtils::sanitize_string(&s)
    }

    /// Convert number to string
    pub fn number_to_string(value: i128) -> String {
        let env = Env::default();
        NumericUtils::i128_to_string(&env, &value)
    }

    /// Convert string to number
    pub fn string_to_number(s: String) -> i128 {
        NumericUtils::string_to_i128(&s)
    }

    /// Generate unique ID
    pub fn generate_unique_id(prefix: String) -> String {
        let env = Env::default();
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
    pub fn validate_future_timestamp(timestamp: u64) -> bool {
        ValidationUtils::validate_future_timestamp(&timestamp)
    }

    /// Get time utilities information
    pub fn get_time_utilities() -> String {
        let env = Env::default();
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
    pub fn validate_event_structure(env: Env, event_type: String, event_data: String) -> bool {
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
        EventDocumentation::get_event_type_docs()
    }

    /// Get event usage examples
    pub fn get_event_usage_examples(env: Env) -> Map<String, String> {
        EventDocumentation::get_usage_examples()
    }

    /// Get event system overview
    pub fn get_event_system_overview() -> String {
        EventDocumentation::get_overview()
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
    pub fn format_event_timestamp(timestamp: u64) -> String {
        EventHelpers::format_timestamp(timestamp)
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
        
        if let Err(error) = ValidationOracleValidator::validate_oracle_config(&env, &oracle_config) {
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
