#![no_std]

use soroban_sdk::{
    contract, contractimpl, panic_with_error, symbol_short, Address, Env, Map, String, Symbol, Vec,
};

// ===== MODULE DECLARATIONS =====
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
mod utils;
mod validation;
mod voting;

// Re-export commonly used items
pub use errors::Error;
pub use types::*;

// Basic imports (only import what we're sure exists)
use admin::AdminInitializer;

// Predictify Hybrid Contract
// 
// This contract provides a comprehensive prediction market system with:
// - Oracle integration for automated market resolution
// - Community voting and consensus mechanisms
// - Dispute resolution and escalation systems
// - Fee management and analytics
// - Admin controls and configuration management

#[contract]
pub struct PredictifyHybrid;

const PERCENTAGE_DENOMINATOR: i128 = 100;
const FEE_PERCENTAGE: i128 = 2; // 2% fee for the platform

#[contractimpl]
impl PredictifyHybrid {
    /// Initialize the contract with an admin
    pub fn initialize(env: Env, admin: Address) {
        match AdminInitializer::initialize(&env, &admin) {
            Ok(_) => (), // Success
            Err(e) => panic_with_error!(env, e),
        }
    }

    /// Create a new prediction market
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
                panic_with_error!(env, Error::Unauthorized);
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

        // Validate oracle configuration
        if let Err(e) = oracle_config.validate(&env) {
            panic_with_error!(env, e);
        }

        // Generate a unique market ID
        let counter_key = Symbol::new(&env, "MarketCounter");
        let counter: u32 = env.storage().persistent().get(&counter_key).unwrap_or(0);
        let new_counter = counter + 1;
        env.storage().persistent().set(&counter_key, &new_counter);

        // Create market ID using symbol short notation
        let market_id = symbol_short!("market");

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

    /// Allow users to vote on a market outcome by staking tokens
    pub fn vote(env: Env, user: Address, market_id: Symbol, outcome: String, stake: i128) {
        user.require_auth();

        // Validate stake amount
        if stake <= 0 {
            panic_with_error!(env, Error::InsufficientStake);
        }

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

        // Check market state
        if market.state != MarketState::Active {
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
        market.votes.set(user.clone(), outcome.clone());
        market.stakes.set(user.clone(), stake);
        market.total_staked += stake;

        env.storage().persistent().set(&market_id, &market);
    }

    /// Allow users to claim winnings from resolved markets
    pub fn claim_winnings(env: Env, user: Address, market_id: Symbol) -> i128 {
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

        let payout = if &user_outcome == winning_outcome {
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
                (user_share * total_pool) / winning_total
            } else {
                0
            }
        } else {
            0 // User didn't win
        };

        // Mark as claimed
        market.claimed.set(user.clone(), true);
        env.storage().persistent().set(&market_id, &market);

        payout
    }

    /// Get market information
    pub fn get_market(env: Env, market_id: Symbol) -> Option<Market> {
        env.storage().persistent().get(&market_id)
    }

    /// Manually resolve a market (admin only)
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

        // Check if market is already resolved
        if market.winning_outcome.is_some() {
            panic_with_error!(env, Error::MarketAlreadyResolved);
        }

        // Validate winning outcome
        let outcome_exists = market.outcomes.iter().any(|o| o == winning_outcome);
        if !outcome_exists {
            panic_with_error!(env, Error::InvalidOutcome);
        }

        // Set winning outcome and update state
        market.winning_outcome = Some(winning_outcome.clone());
        market.state = MarketState::Resolved;
        env.storage().persistent().set(&market_id, &market);
    }
    
    /// Get market vote information for a user
    pub fn get_user_vote(env: Env, market_id: Symbol, user: Address) -> Option<(String, i128)> {
        let market: Market = env.storage().persistent().get(&market_id)?;
        
        let outcome = market.votes.get(user.clone())?;
        let stake = market.stakes.get(user).unwrap_or(0);
        
        Some((outcome, stake))
    }
    
    /// Get market statistics
    pub fn get_market_stats(env: Env, market_id: Symbol) -> Option<(i128, u32, bool)> {
        let market: Market = env.storage().persistent().get(&market_id)?;
        
        let total_staked = market.total_staked;
        let total_voters = market.votes.len();
        let is_resolved = market.winning_outcome.is_some();
        
        Some((total_staked, total_voters, is_resolved))
    }
    
    /// Check if market has ended but not resolved
    pub fn needs_resolution(env: Env, market_id: Symbol) -> bool {
        let market: Market = match env.storage().persistent().get(&market_id) {
            Some(m) => m,
            None => return false,
        };
        
        let current_time = env.ledger().timestamp();
        current_time >= market.end_time && market.winning_outcome.is_none()
    }
    
    /// Get all outcomes for a market
    pub fn get_market_outcomes(env: Env, market_id: Symbol) -> Option<Vec<String>> {
        let market: Market = env.storage().persistent().get(&market_id)?;
        Some(market.outcomes)
    }
    
    /// Check if user has already voted
    pub fn has_user_voted(env: Env, market_id: Symbol, user: Address) -> bool {
        let market: Market = match env.storage().persistent().get(&market_id) {
            Some(m) => m,
            None => return false,
        };
        
        market.votes.get(user).is_some()
    }
    
    /// Get market end time
    pub fn get_market_end_time(env: Env, market_id: Symbol) -> Option<u64> {
        let market: Market = env.storage().persistent().get(&market_id)?;
        Some(market.end_time)
    }
    
    /// Get market state
    pub fn get_market_state(env: Env, market_id: Symbol) -> Option<MarketState> {
        let market: Market = env.storage().persistent().get(&market_id)?;
        Some(market.state)
    }
    
    /// Get total number of markets created
    pub fn get_total_markets(env: Env) -> u32 {
        let counter_key = Symbol::new(&env, "MarketCounter");
        env.storage().persistent().get(&counter_key).unwrap_or(0)
    }
}

#[cfg(test)]
mod test;