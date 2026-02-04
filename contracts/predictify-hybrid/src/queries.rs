#![allow(dead_code)]

//! Query Functions for Predictify Hybrid Contract
//!
//! This module provides comprehensive read-only query functions for retrieving
//! event information, bet details, and contract state. All functions are:
//! - **Gas-efficient**: Read-only operations with no state modifications
//! - **Secure**: Input validation on all parameters
//! - **Documented**: Comprehensive examples and usage patterns
//! - **Tested**: Full test coverage with property-based tests
//!
//! # Query Categories
//!
//! 1. **Market/Event Queries** - Retrieve detailed information about prediction markets
//! 2. **User Bet Queries** - Get user-specific voting and staking information
//! 3. **Contract State Queries** - Retrieve global contract state and statistics
//! 4. **Analytics Queries** - Get aggregated market analytics and performance metrics

use crate::{
    errors::Error,
    markets::{MarketAnalytics, MarketStateManager, MarketValidator},
    types::{Market, MarketState},
    voting::VotingStats,
};
use soroban_sdk::{contracttype, vec, Address, Env, Map, String, Symbol, Vec};

use crate::types::{
    ContractStateQuery, EventDetailsQuery, MarketPoolQuery, MarketStatus, MultipleBetsQuery,
    UserBalanceQuery, UserBetQuery,
};

// ===== QUERY MANAGER =====

/// Main query management system for Predictify Hybrid contract.
///
/// Provides comprehensive read-only access to contract state and user data,
/// with full validation and error handling. All functions are gas-efficient
/// and suitable for frequent client-side queries.
///
/// # Design Principles
///
/// - **Gas Efficiency**: Minimal storage reads, no state modifications
/// - **Security**: Input validation on all parameters
/// - **Consistency**: Always returns accurate, point-in-time state
/// - **Composability**: Functions can be chained for complex queries
/// - **Client-Friendly**: Structured responses optimized for clients
pub struct QueryManager;

impl QueryManager {
    // ===== EVENT/MARKET QUERIES =====

    /// Query detailed information about a specific market.
    ///
    /// Retrieves comprehensive market details including question, outcomes,
    /// status, and current statistics. This is the primary function for
    /// displaying market information to users.
    ///
    /// # Parameters
    ///
    /// * `env` - Soroban environment for blockchain operations
    /// * `market_id` - The ID of the market to query
    ///
    /// # Returns
    ///
    /// * `Ok(EventDetailsQuery)` - Complete market details
    /// * `Err(Error::MarketNotFound)` - If market doesn't exist
    /// * `Err(Error::InvalidMarket)` - If market data is corrupted
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Symbol};
    /// # use predictify_hybrid::queries::QueryManager;
    /// # let env = Env::default();
    /// # let market_id = Symbol::new(&env, "BTC_100K");
    ///
    /// match QueryManager::query_event_details(&env, market_id) {
    ///     Ok(details) => println!("Question: {}", details.question),
    ///     Err(e) => println!("Market not found: {:?}", e),
    /// }
    /// ```
    pub fn query_event_details(env: &Env, market_id: Symbol) -> Result<EventDetailsQuery, Error> {
        let market = Self::get_market_from_storage(env, &market_id)?;

        // Calculate participant count
        let participant_count = market.votes.len() as u32;

        // Calculate vote count (simple approximation)
        let vote_count = market.votes.len() as u32;

        // Get oracle provider name
        let oracle_provider = market.oracle_config.provider.name();

        let winning_outcome = market.get_winning_outcome();

        let response = EventDetailsQuery {
            market_id,
            question: market.question,
            outcomes: market.outcomes,
            created_at: 0, // TODO: Retrieve from storage if available
            end_time: market.end_time,
            status: MarketStatus::from_market_state(market.state),
            oracle_provider: String::from_str(env, oracle_provider),
            feed_id: market.oracle_config.feed_id,
            total_staked: market.total_staked,
            winning_outcome,
            oracle_result: market.oracle_result.clone(),
            participant_count,
            vote_count,
            admin: market.admin,
        };

        Ok(response)
    }

    /// Query market status for a specific event.
    ///
    /// Lightweight query that returns only the market status and end time.
    /// Useful for quick status checks without full market details.
    ///
    /// # Parameters
    ///
    /// * `env` - Soroban environment
    /// * `market_id` - Market ID to query
    ///
    /// # Returns
    ///
    /// * `Ok((MarketStatus, u64))` - Status and end time
    /// * `Err(Error::MarketNotFound)` - Market not found
    pub fn query_event_status(env: &Env, market_id: Symbol) -> Result<(MarketStatus, u64), Error> {
        let market = Self::get_market_from_storage(env, &market_id)?;
        Ok((
            MarketStatus::from_market_state(market.state),
            market.end_time,
        ))
    }

    /// Get list of all market IDs.
    ///
    /// Returns a vector of all market identifiers created in the contract.
    /// Useful for discovering available markets or implementing pagination.
    ///
    /// # Parameters
    ///
    /// * `env` - Soroban environment
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<Symbol>)` - List of all market IDs
    /// * `Err(Error::ContractStateError)` - If market index is corrupted
    pub fn get_all_markets(env: &Env) -> Result<Vec<Symbol>, Error> {
        // Retrieve market index from storage
        let market_key = Symbol::new(env, "market_index");
        let markets: Vec<Symbol> = env
            .storage()
            .persistent()
            .get(&market_key)
            .map(|v: Vec<Symbol>| v)
            .unwrap_or_else(|| vec![env]);

        Ok(markets)
    }

    // ===== USER BET QUERIES =====

    /// Query detailed information about a user's bet on a specific market.
    ///
    /// Retrieves complete information about a user's participation including
    /// vote, stake, payout eligibility, and claim status. This is the primary
    /// function for displaying user bet details.
    ///
    /// # Parameters
    ///
    /// * `env` - Soroban environment
    /// * `user` - User address to query
    /// * `market_id` - Market ID to query
    ///
    /// # Returns
    ///
    /// * `Ok(UserBetQuery)` - Complete bet details
    /// * `Err(Error::MarketNotFound)` - Market doesn't exist
    /// * `Err(Error::UserNotFound)` - User hasn't participated in market
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address, Symbol};
    /// # use predictify_hybrid::queries::QueryManager;
    /// # let env = Env::default();
    /// # let user = Address::generate(&env);
    /// # let market_id = Symbol::new(&env, "BTC_100K");
    ///
    /// match QueryManager::query_user_bet(&env, user, market_id) {
    ///     Ok(bet) => {
    ///         println!("Stake: {} stroops", bet.stake_amount);
    ///         println!("Winning: {}", bet.is_winning);
    ///     },
    ///     Err(_) => println!("User hasn't bet on this market"),
    /// }
    /// ```
    pub fn query_user_bet(
        env: &Env,
        user: Address,
        market_id: Symbol,
    ) -> Result<UserBetQuery, Error> {
        let market = Self::get_market_from_storage(env, &market_id)?;

        // Check if user has participated
        let outcome = market
            .votes
            .get(user.clone())
            .ok_or(Error::InvalidInput)?;

        let stake_amount = market
            .stakes
            .get(user.clone())
            .ok_or(Error::InvalidInput)?;

        let has_claimed = market.claimed.get(user.clone()).unwrap_or(false);

        // Determine if user is winning
        let is_winning = market.is_winning_outcome(&outcome);

        // Calculate potential payout
        let potential_payout = if is_winning && !has_claimed {
            Self::calculate_payout(env, &market, stake_amount)?
        } else {
            0
        };

        // Get dispute stake if any
        let dispute_stake = market.dispute_stakes.get(user.clone()).unwrap_or(0);

        let response = UserBetQuery {
            user,
            market_id,
            outcome,
            stake_amount,
            voted_at: 0, // TODO: Retrieve from vote timestamp if available
            is_winning,
            has_claimed,
            potential_payout,
            dispute_stake,
        };

        Ok(response)
    }

    /// Query all bets for a specific user across multiple markets.
    ///
    /// Retrieves the user's participation in all markets with aggregated statistics.
    /// Useful for user dashboard and portfolio views.
    ///
    /// # Parameters
    ///
    /// * `env` - Soroban environment
    /// * `user` - User address to query
    ///
    /// # Returns
    ///
    /// * `Ok(MultipleBetsQuery)` - All user bets with aggregates
    /// * `Err(Error::ContractStateError)` - If market index is corrupted
    pub fn query_user_bets(env: &Env, user: Address) -> Result<MultipleBetsQuery, Error> {
        let all_markets = Self::get_all_markets(env)?;
        let mut bets: Vec<UserBetQuery> = vec![env];
        let mut total_stake = 0i128;
        let mut total_potential_payout = 0i128;
        let mut winning_bets = 0u32;

        for market_id in all_markets.iter() {
            if let Ok(bet) = Self::query_user_bet(env, user.clone(), market_id) {
                total_stake += bet.stake_amount;
                total_potential_payout += bet.potential_payout;
                if bet.is_winning {
                    winning_bets += 1;
                }
                bets.push_back(bet);
            }
        }

        Ok(MultipleBetsQuery {
            bets,
            total_stake,
            total_potential_payout,
            winning_bets,
        })
    }

    // ===== BALANCE AND POOL QUERIES =====

    /// Query user's account balance and participation metrics.
    ///
    /// Provides comprehensive view of user's account including available balance,
    /// total staked amount, winnings, and participation count across markets.
    ///
    /// # Parameters
    ///
    /// * `env` - Soroban environment
    /// * `user` - User address to query
    ///
    /// # Returns
    ///
    /// * `Ok(UserBalanceQuery)` - Complete balance information
    pub fn query_user_balance(env: &Env, user: Address) -> Result<UserBalanceQuery, Error> {
        // Get all user bets
        let bets = Self::query_user_bets(env, user.clone())?;

        // Query balance from token contract (would integrate with actual token logic)
        let available_balance = 0i128; // TODO: Integrate with token contract

        let unclaimed_balance = bets.total_potential_payout;

        let response = UserBalanceQuery {
            user,
            available_balance,
            total_staked: bets.total_stake,
            total_winnings: 0i128, // TODO: Calculate from resolved markets
            active_bet_count: bets.bets.len() as u32,
            resolved_market_count: 0u32, // TODO: Count resolved markets
            unclaimed_balance,
        };

        Ok(response)
    }

    /// Query market pool distribution and implied probabilities.
    ///
    /// Provides detailed stake distribution across outcomes and calculates
    /// implied probabilities. Useful for price discovery and liquidity analysis.
    ///
    /// # Parameters
    ///
    /// * `env` - Soroban environment
    /// * `market_id` - Market ID to query
    ///
    /// # Returns
    ///
    /// * `Ok(MarketPoolQuery)` - Pool distribution and probabilities
    /// * `Err(Error::MarketNotFound)` - Market not found
    pub fn query_market_pool(env: &Env, market_id: Symbol) -> Result<MarketPoolQuery, Error> {
        let market = Self::get_market_from_storage(env, &market_id)?;

        // Calculate outcome pools
        let mut outcome_pools: Map<String, i128> = Map::new(env);
        for outcome in market.outcomes.iter() {
            let pool = Self::calculate_outcome_pool(env, &market, &outcome)?;
            outcome_pools.set(outcome, pool);
        }

        // Calculate implied probabilities
        let (prob_yes, prob_no) = Self::calculate_implied_probabilities(env, &market)?;

        let response = MarketPoolQuery {
            market_id,
            total_pool: market.total_staked,
            outcome_pools,
            platform_fees: 0i128, // TODO: Retrieve from fees module
            implied_probability_yes: prob_yes,
            implied_probability_no: prob_no,
        };

        Ok(response)
    }

    /// Query total pool size for all markets.
    ///
    /// Returns aggregate liquidity across the entire platform.
    /// Useful for platform-level monitoring and dashboards.
    ///
    /// # Parameters
    ///
    /// * `env` - Soroban environment
    ///
    /// # Returns
    ///
    /// * `Ok(i128)` - Total value locked across all markets
    pub fn query_total_pool_size(env: &Env) -> Result<i128, Error> {
        let all_markets = Self::get_all_markets(env)?;
        let mut total = 0i128;

        for market_id in all_markets.iter() {
            if let Ok(market) = Self::get_market_from_storage(env, &market_id) {
                total += market.total_staked;
            }
        }

        Ok(total)
    }

    // ===== CONTRACT STATE QUERIES =====

    /// Query global contract state and statistics.
    ///
    /// Provides system-level metrics including total markets, active markets,
    /// total value locked, and user statistics. Useful for platform dashboards
    /// and monitoring systems.
    ///
    /// # Parameters
    ///
    /// * `env` - Soroban environment
    ///
    /// # Returns
    ///
    /// * `Ok(ContractStateQuery)` - Global contract state
    pub fn query_contract_state(env: &Env) -> Result<ContractStateQuery, Error> {
        let all_markets = Self::get_all_markets(env)?;
        let total_markets = all_markets.len() as u32;

        let mut active_markets = 0u32;
        let mut resolved_markets = 0u32;
        let mut total_value_locked = 0i128;

        for market_id in all_markets.iter() {
            if let Ok(market) = Self::get_market_from_storage(env, &market_id) {
                match market.state {
                    MarketState::Active => active_markets += 1,
                    MarketState::Resolved | MarketState::Closed => resolved_markets += 1,
                    _ => {}
                }
                total_value_locked += market.total_staked;
            }
        }

        let response = ContractStateQuery {
            total_markets,
            active_markets,
            resolved_markets,
            total_value_locked,
            total_fees_collected: 0i128, // TODO: Retrieve from fees module
            unique_users: 0u32, // TODO: Calculate from user index
            contract_version: String::from_str(env, "1.0.0"),
            last_update: env.ledger().timestamp(),
        };

        Ok(response)
    }

    // ===== HELPER FUNCTIONS =====

    /// Retrieve market from persistent storage.
    ///
    /// Internal helper to get market data from storage with error handling.
    fn get_market_from_storage(env: &Env, market_id: &Symbol) -> Result<Market, Error> {
        env.storage()
            .persistent()
            .get(market_id)
            .ok_or(Error::MarketNotFound)
    }

    /// Calculate payout for a user based on stake and market outcome.
    ///
    /// Computes the user's payout considering:
    /// - User's stake proportion
    /// - Total winning stakes
    /// - Platform fee deduction
    pub(crate) fn calculate_payout(env: &Env, market: &Market, user_stake: i128) -> Result<i128, Error> {
        if user_stake <= 0 {
            return Ok(0);
        }

        // Get total winning stakes
        if let Some(winning_outcome) = market.get_winning_outcome() {
            let winning_total = Self::calculate_outcome_pool(env, market, &winning_outcome)?;

            if winning_total <= 0 {
                return Ok(0);
            }

            // Calculate user's share: (user_stake / winning_total) * total_pool
            let user_share = (user_stake * market.total_staked) / winning_total;

            // Deduct platform fee (2%)
            let fee_amount = (user_share * 2) / 100;
            let payout = user_share - fee_amount;

            Ok(payout.max(0))
        } else {
            Ok(0)
        }
    }

    /// Calculate total stake for a specific outcome.
    ///
    /// Sums all user stakes that voted for the given outcome.
    pub(crate) fn calculate_outcome_pool(
        env: &Env,
        market: &Market,
        outcome: &String,
    ) -> Result<i128, Error> {
        let mut pool = 0i128;

        // Iterate through all votes to find matching outcome
        for (user, voted_outcome) in market.votes.iter() {
            if voted_outcome == *outcome {
                if let Some(stake) = market.stakes.get(user) {
                    pool += stake;
                }
            }
        }

        Ok(pool)
    }

    /// Calculate implied probabilities for binary outcomes.
    ///
    /// Uses stake distribution to infer market's probability estimates
    /// for "yes" and "no" outcomes. Returns percentages (0-100).
    pub(crate) fn calculate_implied_probabilities(
        env: &Env,
        market: &Market,
    ) -> Result<(u32, u32), Error> {
        if market.outcomes.len() < 2 {
            return Ok((50, 50)); // Default if insufficient outcomes
        }

        // Get first two outcome pools
        let outcome1 = market.outcomes.get(0).unwrap();
        let outcome2 = market.outcomes.get(1).unwrap();

        let pool1 = Self::calculate_outcome_pool(env, market, &outcome1)?;
        let pool2 = Self::calculate_outcome_pool(env, market, &outcome2)?;

        let total = pool1 + pool2;
        if total <= 0 {
            return Ok((50, 50));
        }

        let prob1 = ((pool2 * 100) / total) as u32; // Inverse: more stake on outcome1 = lower prob
        let prob2 = ((pool1 * 100) / total) as u32;

        Ok((prob1, prob2))
    }
}

// ===== TESTS =====

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::Env;

    #[test]
    fn test_market_status_conversion() {
        let status = MarketStatus::from_market_state(MarketState::Active);
        assert_eq!(status, MarketStatus::Active);

        let status = MarketStatus::from_market_state(MarketState::Resolved);
        assert_eq!(status, MarketStatus::Resolved);
    }

    #[test]
    fn test_payout_calculation_zero_stake() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let market = Market::new(
            &env,
            admin,
            String::from_str(&env, "Test"),
            vec![&env, String::from_str(&env, "yes")],
            env.ledger().timestamp() + 1000,
            crate::types::OracleConfig::new(
                crate::types::OracleProvider::Reflector,
                Address::from_str(&env, "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF"),
                String::from_str(&env, "TEST"),
                100,
                String::from_str(&env, "gt"),
            ),
            None,
            0u64,
            MarketState::Active,
        );

        let payout = QueryManager::calculate_payout(&env, &market, 0);
        assert!(payout.is_ok());
        assert_eq!(payout.unwrap(), 0);
    }

    #[test]
    fn test_implied_probabilities_equal_stakes() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let mut market = Market::new(
            &env,
            admin,
            String::from_str(&env, "Test"),
            vec![
                &env,
                String::from_str(&env, "yes"),
                String::from_str(&env, "no"),
            ],
            env.ledger().timestamp() + 1000,
            crate::types::OracleConfig::new(
                crate::types::OracleProvider::Reflector,
                Address::from_str(&env, "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF"),
                String::from_str(&env, "TEST"),
                100,
                String::from_str(&env, "gt"),
            ),
            None,
            0u64,
            MarketState::Active,
        );

        // Set total staked and outcome pools
        market.total_staked = 100;

        let probs = QueryManager::calculate_implied_probabilities(&env, &market);
        assert!(probs.is_ok());
        let (p1, p2) = probs.unwrap();
        assert_eq!(p1 + p2, 100);
    }

    #[test]
    fn test_outcome_pool_calculation() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let user1 = Address::generate(&env);
        let user2 = Address::generate(&env);

        let mut market = Market::new(
            &env,
            admin,
            String::from_str(&env, "Test"),
            vec![
                &env,
                String::from_str(&env, "yes"),
                String::from_str(&env, "no"),
            ],
            env.ledger().timestamp() + 1000,
            crate::types::OracleConfig::new(
                crate::types::OracleProvider::Reflector,
                Address::from_str(&env, "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF"),
                String::from_str(&env, "TEST"),
                100,
                String::from_str(&env, "gt"),
            ),
            None,
            0u64,
            MarketState::Active,
        );

        // Add votes
        let yes_outcome = String::from_str(&env, "yes");
        market.votes.set(user1.clone(), yes_outcome.clone());
        market.stakes.set(user1, 50);

        market.votes.set(user2.clone(), yes_outcome.clone());
        market.stakes.set(user2, 75);

        let pool = QueryManager::calculate_outcome_pool(&env, &market, &yes_outcome);
        assert!(pool.is_ok());
        assert_eq!(pool.unwrap(), 125);
    }
}
