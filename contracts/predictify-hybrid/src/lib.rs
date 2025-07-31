#![no_std]

extern crate alloc;
extern crate wee_alloc;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

// Module declarations - all modules enabled
mod admin;
mod config;
mod disputes;
mod errors;
mod events;
mod extensions;
mod fees;
mod markets;
mod oracles;
mod resolution;
mod types;
mod upgrade;
mod utils;
mod validation;
mod voting;

// Re-export commonly used items
pub use errors::Error;
pub use types::*;
use admin::AdminInitializer;

use soroban_sdk::{
    contract, contractimpl, panic_with_error, Address, Env, Map, String, Symbol, Vec,
};

#[contract]
pub struct PredictifyHybrid;

const PERCENTAGE_DENOMINATOR: i128 = 100;
const FEE_PERCENTAGE: i128 = 2; // 2% fee for the platform

#[contractimpl]
impl PredictifyHybrid {
    pub fn initialize(env: Env, admin: Address) {
        match AdminInitializer::initialize(&env, &admin) {
            Ok(_) => (), // Success
            Err(e) => panic_with_error!(env, e),
        }
    }

    // Create a market
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


        if admin != stored_admin {
            panic_with_error!(env, Error::Unauthorized);

        }

        // Validate inputs
        if outcomes.len() < 2 {
            panic_with_error!(env, Error::InvalidOutcomes);
        }

        if question.len() == 0 {
            panic_with_error!(env, Error::InvalidQuestion);
        }

        // Generate a unique market ID
        let counter_key = Symbol::new(&env, "MarketCounter");
        let counter: u32 = env.storage().persistent().get(&counter_key).unwrap_or(0);
        let new_counter = counter + 1;
        env.storage().persistent().set(&counter_key, &new_counter);

        let market_id = Symbol::new(&env, "market");

        // Calculate end time
        let seconds_per_day: u64 = 24 * 60 * 60;
        let duration_seconds: u64 = (duration_days as u64) * seconds_per_day;
        let end_time: u64 = env.ledger().timestamp() + duration_seconds;


        // Create a new market
        let market = Market {
            admin: admin.clone(),
            question,
            outcomes,
            end_time,
            oracle_config,
            oracle_result: None,
            votes: Map::new(&env),
            total_staked: 0,
            dispute_stakes: Map::new(&env),
            stakes: Map::new(&env),
            claimed: Map::new(&env),
            winning_outcome: None,
            fee_collected: false,
            state: MarketState::Active,
            total_extension_days: 0,
            max_extension_days: 30,
            extension_history: Vec::new(&env),
        };

        // Store the market
        env.storage().persistent().set(&market_id, &market);

        market_id
    }


    // Allows users to vote on a market outcome by staking tokens
    pub fn vote(env: Env, user: Address, market_id: Symbol, outcome: String, stake: i128) {
        user.require_auth();

        let mut market: Market = env
            .storage()
            .persistent()
            .get(&market_id)
            .unwrap_or_else(|| {
                panic_with_error!(env, Error::MarketNotFound);
            });


        // Check if the market is still active
        if env.ledger().timestamp() >= market.end_time {
            panic_with_error!(env, Error::MarketClosed);
        }

        // Validate outcome
        let outcome_exists = market.outcomes.iter().any(|o| o == outcome);
        if !outcome_exists {
            panic_with_error!(env, Error::InvalidOutcome);
        }

        // Check if user already voted
        if market.votes.get(user.clone()).is_some() {
            panic_with_error!(env, Error::AlreadyVoted);
        }

        // Store the vote and stake
        market.votes.set(user.clone(), outcome);
        market.stakes.set(user.clone(), stake);
        market.total_staked += stake;

        env.storage().persistent().set(&market_id, &market);
    }

    // Claim winnings
    pub fn claim_winnings(env: Env, user: Address, market_id: Symbol) {
        user.require_auth();

        let mut market: Market = env
            .storage()
            .persistent()
            .get(&market_id)
            .unwrap_or_else(|| {
                panic_with_error!(env, Error::MarketNotFound);
            });

        // Check if user has claimed already
        if market.claimed.get(user.clone()).unwrap_or(false) {
            panic_with_error!(env, Error::AlreadyClaimed);
        }

        // Check if market is resolved
        let winning_outcome = match &market.winning_outcome {
            Some(outcome) => outcome,
            None => panic_with_error!(env, Error::MarketNotResolved),
        };

        // Get user's vote
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


            if winning_total > 0 {
                let user_share = (user_stake * (PERCENTAGE_DENOMINATOR - FEE_PERCENTAGE))
                    / PERCENTAGE_DENOMINATOR;
                let total_pool = market.total_staked;
                let _payout = (user_share * total_pool) / winning_total;

                // In a real implementation, transfer tokens here
                // For now, we just mark as claimed
            }
        }

        // Mark as claimed
        market.claimed.set(user.clone(), true);
        env.storage().persistent().set(&market_id, &market);
    }

    // Get market information
    pub fn get_market(env: Env, market_id: Symbol) -> Option<Market> {
        env.storage().persistent().get(&market_id)
    }

    // Manually resolve a market (admin only)
    pub fn resolve_market_manual(env: Env, admin: Address, market_id: Symbol, winning_outcome: String) {
        admin.require_auth();


        // Verify admin
        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, "Admin"))
            .unwrap_or_else(|| {
                panic_with_error!(env, Error::Unauthorized);
            });

        if admin != stored_admin {
            panic_with_error!(env, Error::Unauthorized);

        }


        let mut market: Market = env
            .storage()
            .persistent()
            .get(&market_id)
            .unwrap_or_else(|| {
                panic_with_error!(env, Error::MarketNotFound);
            });

        // Check if market has ended
        if env.ledger().timestamp() < market.end_time {
            panic_with_error!(env, Error::MarketClosed);
        }


        // Validate winning outcome
        let outcome_exists = market.outcomes.iter().any(|o| o == winning_outcome);
        if !outcome_exists {
            panic_with_error!(env, Error::InvalidOutcome);
        }



        // Set winning outcome
        market.winning_outcome = Some(winning_outcome);
        env.storage().persistent().set(&market_id, &market);
    }
    
    /// Fetch oracle result for a market
    pub fn fetch_oracle_result(
        env: Env,
        market_id: Symbol,
        oracle_contract: Address,
    ) -> Result<String, Error> {
        // Get the market from storage
        let market = env.storage().persistent().get::<Symbol, Market>(&market_id)
            .ok_or(Error::MarketNotFound)?;
        
        // Validate market state
        if market.oracle_result.is_some() {
            return Err(Error::MarketAlreadyResolved);
        }
        

        // Check if market has ended
        let current_time = env.ledger().timestamp();
        if current_time < market.end_time {
            return Err(Error::MarketClosed);

        }
        
        // Get oracle result using the resolution module
        let oracle_resolution = resolution::OracleResolutionManager::fetch_oracle_result(&env, &market_id, &oracle_contract)?;
        
        Ok(oracle_resolution.oracle_result)
    }
    
    /// Resolve a market automatically using oracle and community consensus
    pub fn resolve_market(env: Env, market_id: Symbol) -> Result<(), Error> {
        // Use the resolution module to resolve the market
        let _resolution = resolution::MarketResolutionManager::resolve_market(&env, &market_id)?;
        Ok(())
    }
    
    /// Get resolution analytics
    pub fn get_resolution_analytics(env: Env) -> Result<resolution::ResolutionAnalytics, Error> {
        resolution::MarketResolutionAnalytics::calculate_resolution_analytics(&env)
    }
    
    /// Get market analytics
    pub fn get_market_analytics(env: Env, market_id: Symbol) -> Result<markets::MarketStats, Error> {
        let market = env.storage().persistent().get::<Symbol, Market>(&market_id)
            .ok_or(Error::MarketNotFound)?;
        
        // Calculate market statistics
        let stats = markets::MarketAnalytics::get_market_stats(&market);
        
        Ok(stats)
    }
}

mod test;
