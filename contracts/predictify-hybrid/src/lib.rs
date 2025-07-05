#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, panic_with_error, token, Address, Env,
    Map, String, Symbol, Vec, symbol_short, vec, IntoVal,
};

// Error management module
pub mod errors;
use errors::Error;

// Types module
pub mod types;
use types::*;

// Oracle management module
pub mod oracles;
use oracles::{OracleInterface, OracleFactory, OracleUtils, OracleInstance};

// Market management module
pub mod markets;
use markets::{MarketCreator, MarketValidator, MarketStateManager, MarketAnalytics, MarketUtils};

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
        match MarketCreator::create_market(&env, admin, question, outcomes, duration_days, oracle_config) {
            Ok(market_id) => market_id,
            Err(e) => panic_with_error!(env, e),
        }
    }

    // Distribute winnings to users
    pub fn claim_winnings(env: Env, user: Address, market_id: Symbol) {
        user.require_auth();

        let mut market = match MarketStateManager::get_market(&env, &market_id) {
            Ok(market) => market,
            Err(e) => panic_with_error!(env, e),
        };

        // Check if user has already claimed
        let claimed = market.claimed.get(user.clone()).unwrap_or(false);
        if claimed {
            panic_with_error!(env, Error::AlreadyClaimed);
        }

        // Check if market is resolved
        if market.winning_outcome.is_none() {
            panic_with_error!(env, Error::MarketNotResolved);
        }

        // Get winning outcome
        let winning_outcome = market.winning_outcome.as_ref().unwrap();

        // Get user's vote and stake
        let user_outcome = market
            .votes
            .get(user.clone())
            .unwrap_or_else(|| panic_with_error!(env, Error::NothingToClaim));

        let user_stake = market.stakes.get(user.clone()).unwrap_or(0);

        // Calculate payout if user won
        if &user_outcome == winning_outcome {
            // Calculate winning statistics
            let winning_stats = MarketAnalytics::calculate_winning_stats(&market, winning_outcome);
            
            // Calculate payout
            let payout = match MarketUtils::calculate_payout(
                user_stake,
                winning_stats.winning_total,
                winning_stats.total_pool,
                FEE_PERCENTAGE,
            ) {
                Ok(payout) => payout,
                Err(e) => panic_with_error!(env, e),
            };

            // Get token client and transfer winnings
            let token_client = match MarketUtils::get_token_client(&env) {
                Ok(client) => client,
                Err(e) => panic_with_error!(env, e),
            };
            token_client.transfer(&env.current_contract_address(), &user, &payout);
        }

        // Mark as claimed
        MarketStateManager::mark_claimed(&mut market, user);
        MarketStateManager::update_market(&env, &market_id, &market);
    }

    // Collect platform fees
    pub fn collect_fees(env: Env, admin: Address, market_id: Symbol) {
        admin.require_auth();

        let mut market = match MarketStateManager::get_market(&env, &market_id) {
            Ok(market) => market,
            Err(e) => panic_with_error!(env, e),
        };

        // Verify admin
        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, "Admin"))
            .expect("Admin not set");

        // Use error helper for admin validation
        errors::helpers::require_admin(&env, &admin, &stored_admin);

        // Check if fees already collected
        if market.fee_collected {
            panic_with_error!(env, Error::FeeAlreadyCollected);
        }

        // Calculate 2% fee
        let fee = (market.total_staked * 2) / 100;

        // Get token client and transfer fee
        let token_client = match MarketUtils::get_token_client(&env) {
            Ok(client) => client,
            Err(e) => panic_with_error!(env, e),
        };
        token_client.transfer(&env.current_contract_address(), &admin, &fee);

        // Update market state
        MarketStateManager::mark_fees_collected(&mut market);
        MarketStateManager::update_market(&env, &market_id, &market);
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
        // Require authentication from the user
        user.require_auth();

        // Get the market from storage
        let mut market = match MarketStateManager::get_market(&env, &market_id) {
            Ok(market) => market,
            Err(e) => panic_with_error!(env, e),
        };

        // Validate market state for voting
        if let Err(e) = MarketValidator::validate_market_for_voting(&env, &market) {
            panic_with_error!(env, e);
        }

        // Validate outcome
        if let Err(e) = MarketValidator::validate_outcome(&env, &outcome, &market.outcomes) {
            panic_with_error!(env, e);
        }

        // Validate stake
        if let Err(e) = MarketValidator::validate_stake(stake, 1_000_000) { // 0.1 XLM minimum
            panic_with_error!(env, e);
        }

        // Get token client and transfer stake
        let token_client = match MarketUtils::get_token_client(&env) {
            Ok(client) => client,
            Err(e) => panic_with_error!(env, e),
        };
        token_client.transfer(&user, &env.current_contract_address(), &stake);

        // Add vote using market state manager
        MarketStateManager::add_vote(&mut market, user, outcome, stake);
        MarketStateManager::update_market(&env, &market_id, &market);
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
        let oracle = match OracleFactory::create_oracle(market.oracle_config.provider.clone(), oracle_contract) {
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
        // Require authentication from the user
        user.require_auth();

        // Get the market from storage
        let mut market = match MarketStateManager::get_market(&env, &market_id) {
            Ok(market) => market,
            Err(e) => panic_with_error!(env, e),
        };

        // Ensure disputes are only possible after the market ends
        let current_time = env.ledger().timestamp();
        if current_time < market.end_time {
            panic_with_error!(env, Error::MarketClosed);
        }

        // Validate stake
        let min_stake: i128 = 10_000_000; // 10 XLM minimum
        if let Err(e) = MarketValidator::validate_stake(stake, min_stake) {
            panic_with_error!(env, e);
        }

        // Get token client and transfer stake
        let token_client = match MarketUtils::get_token_client(&env) {
            Ok(client) => client,
            Err(e) => panic_with_error!(env, e),
        };
        token_client.transfer(&user, &env.current_contract_address(), &stake);

        // Add dispute stake
        MarketStateManager::add_dispute_stake(&mut market, user, stake);

        // Extend market end time for disputes
        MarketStateManager::extend_for_dispute(&mut market, &env, 24);

        // Update the market in storage
        MarketStateManager::update_market(&env, &market_id, &market);
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
        let final_result = MarketUtils::determine_final_result(&env, &oracle_result, &community_consensus);

        // Set winning outcome
        MarketStateManager::set_winning_outcome(&mut market, final_result.clone());

        // Update the market in storage
        MarketStateManager::update_market(&env, &market_id, &market);

        // Return the final result
        final_result
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
        match MarketCreator::create_reflector_market(&env, admin, question, outcomes, duration_days, asset_symbol, threshold, comparison) {
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
        match MarketCreator::create_pyth_market(&env, admin, question, outcomes, duration_days, feed_id, threshold, comparison) {
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
        asset_symbol: String,  // e.g., "BTC", "ETH", "XLM"
        threshold: i128,
        comparison: String,
    ) -> Symbol {
        match MarketCreator::create_reflector_asset_market(&env, admin, question, outcomes, duration_days, asset_symbol, threshold, comparison) {
            Ok(market_id) => market_id,
            Err(e) => panic_with_error!(env, e),
        }
    }
}
mod test;
