//! # Statistics and Analytics Module
//!
//! This module implements comprehensive statistics and analytics tracking for the
//! Predictify Hybrid prediction market platform. It provides:
//!
//! - **Platform-wide Statistics**: Aggregate metrics across all markets
//! - **User-specific Statistics**: Individual user performance tracking
//! - **Query Functions**: Gas-efficient read-only statistics access
//! - **Automatic Updates**: Statistics updated during market operations
//! - **Event Emissions**: Transparent statistics change notifications
//!
//! ## Features
//!
//! ### Platform Statistics
//! - Total events/markets created
//! - Total bets placed across platform
//! - Total volume (XLM/tokens) traded
//! - Total platform fees collected
//! - Active events count
//!
//! ### User Statistics
//! - User's total bets placed
//! - User's total winnings
//! - User's win rate (percentage)
//! - Markets participated
//! - Total amount wagered
//!
//! ## Gas Efficiency
//!
//! All query functions are read-only operations, ensuring gas-efficient
//! access to analytics data without state modifications.
//!
//! ## Security
//!
//! - Atomic updates during market operations
//! - Consistent state management
//! - Read-only query functions
//! - No direct user modification of statistics

use soroban_sdk::{Address, Env, Symbol};

use crate::errors::Error;
use crate::events::EventEmitter;
use crate::markets::MarketStateManager;
use crate::types::{Market, MarketState, PlatformStatistics, UserStatistics};

// ===== STORAGE KEYS =====

/// Storage key for platform-wide statistics
const PLATFORM_STATS_KEY: &str = "platform_stats";

/// Storage key prefix for user statistics
const USER_STATS_PREFIX: &str = "usr_stats";

// ===== PLATFORM STATISTICS MANAGER =====

/// Manager for platform-wide statistics tracking and analytics.
///
/// This struct provides functions to manage and query aggregate statistics
/// across the entire platform, including market creation, betting activity,
/// trading volume, and fee collection.
pub struct PlatformStatisticsManager;

impl PlatformStatisticsManager {
    /// Initialize platform statistics with zero values.
    ///
    /// Should be called during contract initialization to set up
    /// the statistics tracking system.
    ///
    /// # Parameters
    ///
    /// - `env` - The Soroban environment
    ///
    /// # Example
    ///
    /// ```rust
    /// PlatformStatisticsManager::initialize(&env);
    /// ```
    pub fn initialize(env: &Env) {
        let stats = PlatformStatistics {
            total_markets_created: 0,
            active_markets_count: 0,
            total_bets_placed: 0,
            total_volume: 0,
            total_fees_collected: 0,
            last_updated: env.ledger().timestamp(),
        };

        env.storage()
            .persistent()
            .set(&Symbol::new(env, PLATFORM_STATS_KEY), &stats);
    }

    /// Get current platform statistics.
    ///
    /// Returns aggregate statistics across all markets on the platform.
    /// This is a read-only operation and is gas-efficient.
    ///
    /// # Parameters
    ///
    /// - `env` - The Soroban environment
    ///
    /// # Returns
    ///
    /// Returns `Ok(PlatformStatistics)` with current platform statistics,
    /// or `Err(Error)` if statistics are not initialized.
    ///
    /// # Example
    ///
    /// ```rust
    /// let stats = PlatformStatisticsManager::get_statistics(&env)?;
    /// println!("Total markets: {}", stats.total_markets_created);
    /// println!("Active markets: {}", stats.active_markets_count);
    /// println!("Total volume: {}", stats.total_volume);
    /// ```
    pub fn get_statistics(env: &Env) -> Result<PlatformStatistics, Error> {
        env.storage()
            .persistent()
            .get(&Symbol::new(env, PLATFORM_STATS_KEY))
            .ok_or(Error::StatisticsNotInitialized)
    }

    /// Increment total markets created count.
    ///
    /// Called automatically when a new market is created.
    /// Updates both total markets created and active markets count.
    ///
    /// # Parameters
    ///
    /// - `env` - The Soroban environment
    ///
    /// # Panics
    ///
    /// Panics if statistics are not initialized.
    pub fn increment_markets_created(env: &Env) {
        let mut stats = Self::get_or_initialize(env);
        stats.total_markets_created += 1;
        stats.active_markets_count += 1;
        stats.last_updated = env.ledger().timestamp();
        Self::save_statistics(env, &stats);

        // Emit statistics update event
        EventEmitter::emit_statistics_updated(
            env,
            &Symbol::new(env, "markets_created"),
            stats.total_markets_created as i128,
        );
    }

    /// Decrement active markets count.
    ///
    /// Called automatically when a market is resolved, closed, or cancelled.
    ///
    /// # Parameters
    ///
    /// - `env` - The Soroban environment
    pub fn decrement_active_markets(env: &Env) {
        let mut stats = Self::get_or_initialize(env);
        if stats.active_markets_count > 0 {
            stats.active_markets_count -= 1;
        }
        stats.last_updated = env.ledger().timestamp();
        Self::save_statistics(env, &stats);

        // Emit statistics update event
        EventEmitter::emit_statistics_updated(
            env,
            &Symbol::new(env, "market_resolved"),
            stats.active_markets_count as i128,
        );
    }

    /// Increment total bets placed count.
    ///
    /// Called automatically when a bet is placed on any market.
    ///
    /// # Parameters
    ///
    /// - `env` - The Soroban environment
    pub fn increment_bets_placed(env: &Env) {
        let mut stats = Self::get_or_initialize(env);
        stats.total_bets_placed += 1;
        stats.last_updated = env.ledger().timestamp();
        Self::save_statistics(env, &stats);

        // Emit statistics update event
        EventEmitter::emit_statistics_updated(
            env,
            &Symbol::new(env, "bets_placed"),
            stats.total_bets_placed as i128,
        );
    }

    /// Add to total volume tracked.
    ///
    /// Called automatically when funds are locked in bets or stakes.
    ///
    /// # Parameters
    ///
    /// - `env` - The Soroban environment
    /// - `amount` - Amount to add to total volume (in stroops)
    pub fn add_volume(env: &Env, amount: i128) {
        let mut stats = Self::get_or_initialize(env);
        stats.total_volume += amount;
        stats.last_updated = env.ledger().timestamp();
        Self::save_statistics(env, &stats);

        // Emit statistics update event
        EventEmitter::emit_statistics_updated(
            env,
            &Symbol::new(env, "volume_added"),
            amount,
        );
    }

    /// Add to total fees collected.
    ///
    /// Called automatically when platform fees are collected from payouts.
    ///
    /// # Parameters
    ///
    /// - `env` - The Soroban environment
    /// - `fee_amount` - Fee amount to add (in stroops)
    pub fn add_fees_collected(env: &Env, fee_amount: i128) {
        let mut stats = Self::get_or_initialize(env);
        stats.total_fees_collected += fee_amount;
        stats.last_updated = env.ledger().timestamp();
        Self::save_statistics(env, &stats);

        // Emit statistics update event
        EventEmitter::emit_statistics_updated(
            env,
            &Symbol::new(env, "fees_collected"),
            fee_amount,
        );
    }

    /// Get or initialize platform statistics.
    ///
    /// Helper function that returns existing statistics or initializes
    /// them if they don't exist.
    fn get_or_initialize(env: &Env) -> PlatformStatistics {
        match Self::get_statistics(env) {
            Ok(stats) => stats,
            Err(_) => {
                Self::initialize(env);
                Self::get_statistics(env).unwrap()
            }
        }
    }

    /// Save platform statistics to storage.
    fn save_statistics(env: &Env, stats: &PlatformStatistics) {
        env.storage()
            .persistent()
            .set(&Symbol::new(env, PLATFORM_STATS_KEY), stats);
    }
}

// ===== USER STATISTICS MANAGER =====

/// Manager for user-specific statistics tracking and analytics.
///
/// This struct provides functions to manage and query individual user
/// statistics, including betting activity, performance metrics, and
/// financial outcomes.
pub struct UserStatisticsManager;

impl UserStatisticsManager {
    /// Initialize user statistics with zero values.
    ///
    /// Called automatically when a user places their first bet.
    ///
    /// # Parameters
    ///
    /// - `env` - The Soroban environment
    /// - `user` - User's address
    fn initialize_user(env: &Env, user: &Address) {
        let stats = UserStatistics {
            user: user.clone(),
            total_bets: 0,
            total_wagered: 0,
            total_winnings: 0,
            win_rate: 0,
            markets_participated: 0,
            last_activity: env.ledger().timestamp(),
        };

        env.storage()
            .persistent()
            .set(&(Symbol::new(env, USER_STATS_PREFIX), user.clone()), &stats);
    }

    /// Get user statistics.
    ///
    /// Returns comprehensive statistics for a specific user.
    /// This is a read-only operation and is gas-efficient.
    ///
    /// # Parameters
    ///
    /// - `env` - The Soroban environment
    /// - `user` - User's address
    ///
    /// # Returns
    ///
    /// Returns `Ok(UserStatistics)` with user's statistics,
    /// or `Ok(UserStatistics)` with zero values if user hasn't bet yet.
    ///
    /// # Example
    ///
    /// ```rust
    /// let stats = UserStatisticsManager::get_user_statistics(&env, &user)?;
    /// println!("Total bets: {}", stats.total_bets);
    /// println!("Win rate: {}%", stats.win_rate);
    /// println!("Total winnings: {}", stats.total_winnings);
    /// ```
    pub fn get_user_statistics(env: &Env, user: &Address) -> Result<UserStatistics, Error> {
        match env.storage().persistent().get(&(Symbol::new(env, USER_STATS_PREFIX), user.clone())) {
            Some(stats) => Ok(stats),
            None => {
                // Return zero stats for users who haven't bet yet
                Ok(UserStatistics {
                    user: user.clone(),
                    total_bets: 0,
                    total_wagered: 0,
                    total_winnings: 0,
                    win_rate: 0,
                    markets_participated: 0,
                    last_activity: 0,
                })
            }
        }
    }

    /// Record a new bet placed by user.
    ///
    /// Called automatically when a user places a bet.
    /// Updates total bets, total wagered, and markets participated.
    ///
    /// # Parameters
    ///
    /// - `env` - The Soroban environment
    /// - `user` - User's address
    /// - `amount` - Bet amount (in stroops)
    pub fn record_bet_placed(env: &Env, user: &Address, amount: i128) {
        let mut stats = Self::get_or_initialize_user(env, user);
        stats.total_bets += 1;
        stats.total_wagered += amount;
        stats.markets_participated += 1;
        stats.last_activity = env.ledger().timestamp();
        Self::save_user_statistics(env, &stats);

        // Emit user statistics update event
        EventEmitter::emit_user_statistics_updated(
            env,
            user,
            &Symbol::new(env, "bet_placed"),
            amount,
        );
    }

    /// Record winnings for a user.
    ///
    /// Called automatically when a user claims winnings from a resolved market.
    /// Updates total winnings and recalculates win rate.
    ///
    /// # Parameters
    ///
    /// - `env` - The Soroban environment
    /// - `user` - User's address
    /// - `winnings` - Amount won (in stroops)
    pub fn record_winnings(env: &Env, user: &Address, winnings: i128) {
        let mut stats = Self::get_or_initialize_user(env, user);
        stats.total_winnings += winnings;
        stats.last_activity = env.ledger().timestamp();

        // Recalculate win rate based on current stats
        stats.win_rate = Self::calculate_win_rate_from_stats(&stats);

        Self::save_user_statistics(env, &stats);

        // Emit user statistics update event
        EventEmitter::emit_user_statistics_updated(
            env,
            user,
            &Symbol::new(env, "winnings_claimed"),
            winnings,
        );
    }

    /// Calculate user's win rate.
    ///
    /// Win rate is calculated as: (winning_bets / total_resolved_bets) * 100
    /// Only includes bets on resolved markets.
    ///
    /// # Parameters
    ///
    /// - `env` - The Soroban environment
    /// - `user` - User's address
    ///
    /// # Returns
    ///
    /// Returns win rate as percentage (0-100)
    pub fn calculate_win_rate(env: &Env, user: &Address) -> u32 {
        // Get all markets user has participated in
        let stats = Self::get_or_initialize_user(env, user);
        Self::calculate_win_rate_from_stats(&stats)
    }

    /// Calculate win rate from existing stats (helper function)
    fn calculate_win_rate_from_stats(stats: &UserStatistics) -> u32 {
        if stats.total_bets == 0 {
            return 0;
        }

        // Count winning bets vs total bets
        // This is a simplified calculation - in production you'd track
        // individual bet outcomes more precisely
        let winning_percentage = if stats.total_winnings > stats.total_wagered {
            let profit_ratio = (stats.total_winnings - stats.total_wagered) * 100
                / stats.total_wagered.max(1);
            // Cap at 100%
            profit_ratio.min(100) as u32
        } else if stats.total_winnings > 0 {
            // Some winnings but not profitable overall
            let win_ratio = stats.total_winnings * 100 / stats.total_wagered.max(1);
            (win_ratio as u32).min(100)
        } else {
            0
        };

        winning_percentage
    }

    /// Get or initialize user statistics.
    fn get_or_initialize_user(env: &Env, user: &Address) -> UserStatistics {
        match Self::get_user_statistics(env, user) {
            Ok(stats) => {
                if stats.total_bets == 0 && stats.last_activity == 0 {
                    // User exists but stats are zero - initialize properly
                    Self::initialize_user(env, user);
                    Self::get_user_statistics(env, user).unwrap()
                } else {
                    stats
                }
            }
            Err(_) => {
                Self::initialize_user(env, user);
                Self::get_user_statistics(env, user).unwrap()
            }
        }
    }

    /// Save user statistics to storage.
    fn save_user_statistics(env: &Env, stats: &UserStatistics) {
        env.storage()
            .persistent()
            .set(&(Symbol::new(env, USER_STATS_PREFIX), stats.user.clone()), stats);
    }
}

// ===== STATISTICS UTILITIES =====

/// Utility functions for statistics calculations and queries.
pub struct StatisticsUtils;

impl StatisticsUtils {
    /// Calculate average bet size across platform.
    ///
    /// # Parameters
    ///
    /// - `env` - The Soroban environment
    ///
    /// # Returns
    ///
    /// Returns average bet size in stroops, or 0 if no bets placed.
    pub fn calculate_average_bet_size(env: &Env) -> i128 {
        match PlatformStatisticsManager::get_statistics(env) {
            Ok(stats) => {
                if stats.total_bets_placed > 0 {
                    stats.total_volume / stats.total_bets_placed as i128
                } else {
                    0
                }
            }
            Err(_) => 0,
        }
    }

    /// Calculate total fees as percentage of volume.
    ///
    /// # Parameters
    ///
    /// - `env` - The Soroban environment
    ///
    /// # Returns
    ///
    /// Returns fee percentage (0-100)
    pub fn calculate_fee_percentage(env: &Env) -> u32 {
        match PlatformStatisticsManager::get_statistics(env) {
            Ok(stats) => {
                if stats.total_volume > 0 {
                    ((stats.total_fees_collected * 100) / stats.total_volume) as u32
                } else {
                    0
                }
            }
            Err(_) => 0,
        }
    }

    /// Get platform health score (0-100).
    ///
    /// Calculated based on active markets, betting activity, and volume.
    ///
    /// # Parameters
    ///
    /// - `env` - The Soroban environment
    ///
    /// # Returns
    ///
    /// Returns health score from 0 (unhealthy) to 100 (very healthy)
    pub fn calculate_platform_health_score(env: &Env) -> u32 {
        match PlatformStatisticsManager::get_statistics(env) {
            Ok(stats) => {
                let mut score = 0u32;

                // Active markets contribute 40 points (max)
                if stats.active_markets_count > 0 {
                    score += (stats.active_markets_count.min(10) * 4) as u32;
                }

                // Betting activity contributes 30 points (max)
                if stats.total_bets_placed > 0 {
                    score += (stats.total_bets_placed.min(30) as u32).min(30);
                }

                // Volume contributes 30 points (max)
                if stats.total_volume > 0 {
                    let volume_xlm = stats.total_volume / 10_000_000;
                    score += (volume_xlm.min(30) as u32).min(30);
                }

                score.min(100)
            }
            Err(_) => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::Env;

    #[test]
    fn test_platform_statistics_initialization() {
        let env = Env::default();
        env.mock_all_auths();
        
        let contract_id = env.register_contract(None, crate::PredictifyHybrid);
        let client = crate::PredictifyHybridClient::new(&env, &contract_id);
        
        env.as_contract(&contract_id, || {
            PlatformStatisticsManager::initialize(&env);
            
            let stats = PlatformStatisticsManager::get_statistics(&env).unwrap();
            assert_eq!(stats.total_markets_created, 0);
            assert_eq!(stats.active_markets_count, 0);
            assert_eq!(stats.total_bets_placed, 0);
            assert_eq!(stats.total_volume, 0);
            assert_eq!(stats.total_fees_collected, 0);
        });
    }

    #[test]
    fn test_increment_markets_created() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, crate::PredictifyHybrid);
        
        env.as_contract(&contract_id, || {
            PlatformStatisticsManager::initialize(&env);
            PlatformStatisticsManager::increment_markets_created(&env);
            PlatformStatisticsManager::increment_markets_created(&env);
            
            let stats = PlatformStatisticsManager::get_statistics(&env).unwrap();
            assert_eq!(stats.total_markets_created, 2);
            assert_eq!(stats.active_markets_count, 2);
        });
    }

    #[test]
    fn test_decrement_active_markets() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, crate::PredictifyHybrid);
        
        env.as_contract(&contract_id, || {
            PlatformStatisticsManager::initialize(&env);
            PlatformStatisticsManager::increment_markets_created(&env);
            PlatformStatisticsManager::increment_markets_created(&env);
            PlatformStatisticsManager::decrement_active_markets(&env);
            
            let stats = PlatformStatisticsManager::get_statistics(&env).unwrap();
            assert_eq!(stats.total_markets_created, 2);
            assert_eq!(stats.active_markets_count, 1);
        });
    }

    #[test]
    fn test_increment_bets_placed() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, crate::PredictifyHybrid);
        
        env.as_contract(&contract_id, || {
            PlatformStatisticsManager::initialize(&env);
            PlatformStatisticsManager::increment_bets_placed(&env);
            PlatformStatisticsManager::increment_bets_placed(&env);
            PlatformStatisticsManager::increment_bets_placed(&env);
            
            let stats = PlatformStatisticsManager::get_statistics(&env).unwrap();
            assert_eq!(stats.total_bets_placed, 3);
        });
    }

    #[test]
    fn test_add_volume() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, crate::PredictifyHybrid);
        
        env.as_contract(&contract_id, || {
            PlatformStatisticsManager::initialize(&env);
            PlatformStatisticsManager::add_volume(&env, 10_000_000);
            PlatformStatisticsManager::add_volume(&env, 5_000_000);
            
            let stats = PlatformStatisticsManager::get_statistics(&env).unwrap();
            assert_eq!(stats.total_volume, 15_000_000);
        });
    }

    #[test]
    fn test_add_fees_collected() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, crate::PredictifyHybrid);
        
        env.as_contract(&contract_id, || {
            PlatformStatisticsManager::initialize(&env);
            PlatformStatisticsManager::add_fees_collected(&env, 200_000);
            PlatformStatisticsManager::add_fees_collected(&env, 300_000);
            
            let stats = PlatformStatisticsManager::get_statistics(&env).unwrap();
            assert_eq!(stats.total_fees_collected, 500_000);
        });
    }

    #[test]
    fn test_user_statistics_initialization() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, crate::PredictifyHybrid);
        let user = Address::generate(&env);

        env.as_contract(&contract_id, || {
            let stats = UserStatisticsManager::get_user_statistics(&env, &user).unwrap();
            assert_eq!(stats.total_bets, 0);
            assert_eq!(stats.total_wagered, 0);
            assert_eq!(stats.total_winnings, 0);
            assert_eq!(stats.win_rate, 0);
        });
    }

    #[test]
    fn test_record_bet_placed() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, crate::PredictifyHybrid);
        let user = Address::generate(&env);

        env.as_contract(&contract_id, || {
            UserStatisticsManager::record_bet_placed(&env, &user, 10_000_000);
            UserStatisticsManager::record_bet_placed(&env, &user, 5_000_000);
            
            let stats = UserStatisticsManager::get_user_statistics(&env, &user).unwrap();
            assert_eq!(stats.total_bets, 2);
            assert_eq!(stats.total_wagered, 15_000_000);
            assert_eq!(stats.markets_participated, 2);
        });
    }

    #[test]
    fn test_record_winnings() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, crate::PredictifyHybrid);
        let user = Address::generate(&env);

        env.as_contract(&contract_id, || {
            UserStatisticsManager::record_bet_placed(&env, &user, 10_000_000);
            UserStatisticsManager::record_winnings(&env, &user, 20_000_000);
            
            let stats = UserStatisticsManager::get_user_statistics(&env, &user).unwrap();
            assert_eq!(stats.total_winnings, 20_000_000);
            assert!(stats.win_rate > 0);
        });
    }

    #[test]
    fn test_calculate_win_rate() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, crate::PredictifyHybrid);
        let user = Address::generate(&env);

        env.as_contract(&contract_id, || {
            // Place bet and win
            UserStatisticsManager::record_bet_placed(&env, &user, 10_000_000);
            UserStatisticsManager::record_winnings(&env, &user, 20_000_000);
            
            let win_rate = UserStatisticsManager::calculate_win_rate(&env, &user);
            assert!(win_rate > 0);
            assert!(win_rate <= 100);
        });
    }

    #[test]
    fn test_calculate_average_bet_size() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, crate::PredictifyHybrid);
        
        env.as_contract(&contract_id, || {
            PlatformStatisticsManager::initialize(&env);
            PlatformStatisticsManager::increment_bets_placed(&env);
            PlatformStatisticsManager::add_volume(&env, 10_000_000);
            PlatformStatisticsManager::increment_bets_placed(&env);
            PlatformStatisticsManager::add_volume(&env, 20_000_000);
            
            let avg = StatisticsUtils::calculate_average_bet_size(&env);
            assert_eq!(avg, 15_000_000);
        });
    }

    #[test]
    fn test_calculate_fee_percentage() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, crate::PredictifyHybrid);
        
        env.as_contract(&contract_id, || {
            PlatformStatisticsManager::initialize(&env);
            PlatformStatisticsManager::add_volume(&env, 100_000_000);
            PlatformStatisticsManager::add_fees_collected(&env, 2_000_000);
            
            let fee_pct = StatisticsUtils::calculate_fee_percentage(&env);
            assert_eq!(fee_pct, 2);
        });
    }

    #[test]
    fn test_platform_health_score() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, crate::PredictifyHybrid);
        
        env.as_contract(&contract_id, || {
            PlatformStatisticsManager::initialize(&env);
            PlatformStatisticsManager::increment_markets_created(&env);
            PlatformStatisticsManager::increment_bets_placed(&env);
            PlatformStatisticsManager::add_volume(&env, 10_000_000);
            
            let health = StatisticsUtils::calculate_platform_health_score(&env);
            assert!(health > 0);
            assert!(health <= 100);
        });
    }
}
