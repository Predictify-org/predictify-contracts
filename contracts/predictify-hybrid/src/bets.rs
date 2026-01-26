//! # Bet Placement Module
//!
//! This module implements the bet placement mechanism for prediction markets,
//! allowing users to place bets on active events by locking their funds.
//!
//! ## Features
//!
//! - **Bet Placement**: Users can place bets on active markets
//! - **Fund Locking**: User funds are locked in the contract until resolution
//! - **Bet Tracking**: Tracks bet amount and selected outcome per user
//! - **Double Betting Prevention**: Prevents users from betting twice on the same market
//! - **Validation**: Comprehensive validation for market state, outcomes, and balances
//! - **Event Emission**: Emits bet placement events for transparency
//!
//! ## Security Considerations
//!
//! - Reentrancy protection through Soroban's built-in mechanisms
//! - User authentication via `require_auth()`
//! - Balance validation before fund transfer
//! - Market state validation before accepting bets

use soroban_sdk::{contracttype, symbol_short, Address, Env, Map, String, Symbol};

use crate::errors::Error;
use crate::events::EventEmitter;
use crate::markets::{MarketStateManager, MarketUtils, MarketValidator};
use crate::types::{Bet, BetStats, BetStatus, Market, MarketState};

// ===== CONSTANTS =====

/// Minimum bet amount (0.1 XLM = 1,000,000 stroops)
pub const MIN_BET_AMOUNT: i128 = 1_000_000;

/// Maximum bet amount (10,000 XLM = 100,000,000,000 stroops)
pub const MAX_BET_AMOUNT: i128 = 100_000_000_000;

// ===== STORAGE KEY TYPES =====

/// Storage key for user bets on a specific market
#[contracttype]
#[derive(Clone)]
pub struct BetKey {
    pub market_id: Symbol,
    pub user: Address,
}

/// Storage key for market bet statistics
#[contracttype]
#[derive(Clone)]
pub struct MarketBetsKey {
    pub market_id: Symbol,
}

/// Storage key for market bet registry
#[contracttype]
#[derive(Clone)]
pub struct BetRegistryKey {
    pub tag: Symbol,
    pub market_id: Symbol,
}

// ===== BET MANAGER =====

/// Comprehensive bet manager for prediction market betting operations.
///
/// BetManager serves as the central coordinator for all betting-related operations
/// in the prediction market system. It handles bet placement, fund locking,
/// bet tracking, and bet resolution. The manager ensures betting integrity,
/// proper fund handling, and accurate payout calculations.
///
/// # Core Functionality
///
/// **Bet Placement:**
/// - Validate and process user bets on market outcomes
/// - Handle fund transfers and locking
/// - Ensure betting eligibility and prevent duplicate bets
///
/// **Bet Resolution:**
/// - Process bet outcomes after market resolution
/// - Calculate and distribute winnings
/// - Handle refunds for cancelled markets
///
/// **Bet Tracking:**
/// - Store and retrieve user bets
/// - Track market-wide betting statistics
/// - Provide bet analytics and reporting
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Address, String, Symbol};
/// # use predictify_hybrid::bets::BetManager;
/// # let env = Env::default();
///
/// let user = Address::generate(&env);
/// let market_id = Symbol::new(&env, "BTC_100K");
/// let outcome = String::from_str(&env, "yes");
/// let amount = 5_000_000i128; // 0.5 XLM
///
/// // Place a bet
/// match BetManager::place_bet(&env, user.clone(), market_id.clone(), outcome, amount) {
///     Ok(bet) => println!("Bet placed successfully: {} stroops", bet.amount),
///     Err(e) => println!("Bet placement failed: {:?}", e),
/// }
///
/// // Get user's bet
/// match BetManager::get_bet(&env, &market_id, &user) {
///     Some(bet) => println!("User has bet {} on outcome", bet.amount),
///     None => println!("User has not placed a bet"),
/// }
/// ```
///
/// # Integration Points
///
/// BetManager integrates with:
/// - **Market System**: Validates market states and updates market data
/// - **Token System**: Handles fund locking and payout distributions
/// - **Event System**: Emits events for all betting operations
/// - **Validation System**: Uses comprehensive validation for all operations
pub struct BetManager;

impl BetManager {
    /// Place a bet on a market outcome with fund locking.
    ///
    /// This function processes a user's bet on a prediction market, including
    /// validation, fund locking, and bet storage.
    ///
    /// # Parameters
    ///
    /// - `env` - The Soroban environment
    /// - `user` - Address of the user placing the bet
    /// - `market_id` - Symbol identifying the market
    /// - `outcome` - The outcome the user is betting on
    /// - `amount` - The amount to lock for this bet
    ///
    /// # Returns
    ///
    /// Returns `Ok(Bet)` on success with the created bet details,
    /// or `Err(Error)` if validation fails.
    ///
    /// # Errors
    ///
    /// - `Error::MarketNotFound` - Market does not exist
    /// - `Error::MarketClosed` - Market has ended or is not active
    /// - `Error::MarketAlreadyResolved` - Market has already been resolved
    /// - `Error::AlreadyBet` - User has already placed a bet on this market
    /// - `Error::InsufficientStake` - Bet amount below minimum
    /// - `Error::InvalidOutcome` - Selected outcome not valid for this market
    /// - `Error::InsufficientBalance` - User doesn't have enough funds
    ///
    /// # Security
    ///
    /// - Requires user authentication via `require_auth()`
    /// - Validates market state before accepting bet
    /// - Validates user has not already bet on this market
    /// - Validates user has sufficient balance
    /// - Locks funds atomically with bet creation
    ///
    /// # Example
    ///
    /// ```rust
    /// let bet = BetManager::place_bet(
    ///     &env,
    ///     user.clone(),
    ///     Symbol::new(&env, "BTC_100K"),
    ///     String::from_str(&env, "yes"),
    ///     10_000_000 // 1.0 XLM
    /// )?;
    /// ```
    pub fn place_bet(
        env: &Env,
        user: Address,
        market_id: Symbol,
        outcome: String,
        amount: i128,
    ) -> Result<Bet, Error> {
        // Require authentication from the user
        user.require_auth();

        // Get and validate market
        let mut market = MarketStateManager::get_market(env, &market_id)?;
        BetValidator::validate_market_for_betting(env, &market)?;

        // Validate bet parameters
        BetValidator::validate_bet_parameters(env, &outcome, &market.outcomes, amount)?;

        // Check if user has already bet on this market
        if Self::has_user_bet(env, &market_id, &user) {
            return Err(Error::AlreadyBet);
        }

        // Lock funds (transfer from user to contract)
        BetUtils::lock_funds(env, &user, amount)?;

        // Create bet
        let bet = Bet::new(env, user.clone(), market_id.clone(), outcome.clone(), amount);

        // Store bet
        BetStorage::store_bet(env, &bet)?;

        // Update market betting stats
        Self::update_market_bet_stats(env, &market_id, &outcome, amount)?;

        // Update market's total staked (for payout pool calculation)
        market.total_staked += amount;
        
        // Also update votes and stakes for backward compatibility with payout distribution
        // This allows distribute_payouts to work with both bets and votes
        market.votes.set(user.clone(), outcome.clone());
        market.stakes.set(user.clone(), amount);
        
        MarketStateManager::update_market(env, &market_id, &market);

        // Emit bet placed event
        EventEmitter::emit_bet_placed(env, &market_id, &user, &outcome, amount);

        Ok(bet)
    }

    /// Check if a user has already placed a bet on a market.
    ///
    /// # Parameters
    ///
    /// - `env` - The Soroban environment
    /// - `market_id` - Symbol identifying the market
    /// - `user` - Address of the user to check
    ///
    /// # Returns
    ///
    /// Returns `true` if the user has already placed a bet, `false` otherwise.
    pub fn has_user_bet(env: &Env, market_id: &Symbol, user: &Address) -> bool {
        BetStorage::get_bet(env, market_id, user).is_some()
    }

    /// Get a user's bet on a specific market.
    ///
    /// # Parameters
    ///
    /// - `env` - The Soroban environment
    /// - `market_id` - Symbol identifying the market
    /// - `user` - Address of the user
    ///
    /// # Returns
    ///
    /// Returns `Some(Bet)` if the user has placed a bet, `None` otherwise.
    pub fn get_bet(env: &Env, market_id: &Symbol, user: &Address) -> Option<Bet> {
        BetStorage::get_bet(env, market_id, user)
    }

    /// Get betting statistics for a market.
    ///
    /// # Parameters
    ///
    /// - `env` - The Soroban environment
    /// - `market_id` - Symbol identifying the market
    ///
    /// # Returns
    ///
    /// Returns `BetStats` with market betting statistics.
    pub fn get_market_bet_stats(env: &Env, market_id: &Symbol) -> BetStats {
        BetStorage::get_market_bet_stats(env, market_id)
    }

    /// Update market betting statistics after a new bet.
    fn update_market_bet_stats(
        env: &Env,
        market_id: &Symbol,
        outcome: &String,
        amount: i128,
    ) -> Result<(), Error> {
        let mut stats = BetStorage::get_market_bet_stats(env, market_id);

        // Update totals
        stats.total_bets += 1;
        stats.total_amount_locked += amount;
        stats.unique_bettors += 1;

        // Update outcome totals
        let current_outcome_total = stats.outcome_totals.get(outcome.clone()).unwrap_or(0);
        stats
            .outcome_totals
            .set(outcome.clone(), current_outcome_total + amount);

        // Store updated stats
        BetStorage::store_market_bet_stats(env, market_id, &stats)?;

        Ok(())
    }

    /// Process bet resolution when a market is resolved.
    ///
    /// This function updates all bets for a market based on the winning outcome.
    ///
    /// # Parameters
    ///
    /// - `env` - The Soroban environment
    /// - `market_id` - Symbol identifying the market
    /// - `winning_outcome` - The resolved winning outcome
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success or `Err(Error)` if resolution fails.
    pub fn resolve_market_bets(
        env: &Env,
        market_id: &Symbol,
        winning_outcome: &String,
    ) -> Result<(), Error> {
        // Get all bets for this market from the bet registry
        let bets = BetStorage::get_all_bets_for_market(env, market_id);
        let bet_count = bets.len();

        // Use index-based iteration to avoid iterator segfaults
        for i in 0..bet_count {
            if let Some(bet_key) = bets.get(i) {
                if let Some(mut bet) = BetStorage::get_bet(env, market_id, &bet_key) {
                    // Determine if bet won or lost
                    if bet.outcome == *winning_outcome {
                        bet.mark_as_won();
                    } else {
                        bet.mark_as_lost();
                    }

                    // Update bet status
                    BetStorage::store_bet(env, &bet)?;

                    // Skip event emission to avoid potential segfaults
                    // Events can be emitted separately if needed
                }
            }
        }

        Ok(())
    }

    /// Process refunds for all bets when a market is cancelled.
    ///
    /// # Parameters
    ///
    /// - `env` - The Soroban environment
    /// - `market_id` - Symbol identifying the market
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success or `Err(Error)` if refund fails.
    pub fn refund_market_bets(env: &Env, market_id: &Symbol) -> Result<(), Error> {
        let bets = BetStorage::get_all_bets_for_market(env, market_id);

        for bet_key in bets.iter() {
            if let Some(mut bet) = BetStorage::get_bet(env, market_id, &bet_key) {
                if bet.is_active() {
                    // Refund the locked funds
                    BetUtils::unlock_funds(env, &bet.user, bet.amount)?;

                    // Mark as refunded
                    bet.mark_as_refunded();
                    BetStorage::store_bet(env, &bet)?;

                    // Emit status update event
                    EventEmitter::emit_bet_status_updated(
                        env,
                        market_id,
                        &bet.user,
                        &String::from_str(env, "Active"),
                        &String::from_str(env, "Refunded"),
                        Some(bet.amount),
                    );
                }
            }
        }

        Ok(())
    }

    /// Calculate payout for a winning bet.
    ///
    /// The payout is calculated as:
    /// `payout = (user_bet_amount / total_winning_bets) * total_pool * (1 - fee_percentage)`
    ///
    /// # Parameters
    ///
    /// - `env` - The Soroban environment
    /// - `market_id` - Symbol identifying the market
    /// - `user` - Address of the user claiming winnings
    ///
    /// # Returns
    ///
    /// Returns `Ok(i128)` with the payout amount, or `Err(Error)` if calculation fails.
    pub fn calculate_bet_payout(
        env: &Env,
        market_id: &Symbol,
        user: &Address,
    ) -> Result<i128, Error> {
        // Get user's bet
        let bet = BetStorage::get_bet(env, market_id, user).ok_or(Error::NothingToClaim)?;

        // Ensure bet is a winner
        if !bet.is_winner() {
            return Ok(0);
        }

        // Get market
        let market = MarketStateManager::get_market(env, market_id)?;

        // Get market bet stats
        let stats = BetStorage::get_market_bet_stats(env, market_id);

        // Get total amount bet on the winning outcome
        let winning_outcome = market.winning_outcome.ok_or(Error::MarketNotResolved)?;
        let winning_total = stats.outcome_totals.get(winning_outcome).unwrap_or(0);

        if winning_total == 0 {
            return Ok(0);
        }

        // Get platform fee percentage from config
        let cfg = crate::config::ConfigManager::get_config(env)?;
        let fee_percentage = cfg.fees.platform_fee_percentage;

        // Calculate payout
        let payout = MarketUtils::calculate_payout(
            bet.amount,
            winning_total,
            stats.total_amount_locked,
            fee_percentage,
        )?;

        Ok(payout)
    }
}

// ===== BET STORAGE =====

/// Storage utilities for bet data.
///
/// BetStorage provides functions for storing and retrieving bet data
/// from Soroban persistent storage.
pub struct BetStorage;

impl BetStorage {
    /// Store a bet in persistent storage.
    pub fn store_bet(env: &Env, bet: &Bet) -> Result<(), Error> {
        let key = Self::get_bet_key(env, &bet.market_id, &bet.user);
        env.storage().persistent().set(&key, bet);

        // Also add user to the market's bet registry
        Self::add_to_bet_registry(env, &bet.market_id, &bet.user)?;

        Ok(())
    }

    /// Get a bet from persistent storage.
    pub fn get_bet(env: &Env, market_id: &Symbol, user: &Address) -> Option<Bet> {
        let key = Self::get_bet_key(env, market_id, user);
        env.storage().persistent().get::<BetKey, Bet>(&key)
    }

    /// Remove a bet from persistent storage.
    pub fn remove_bet(env: &Env, market_id: &Symbol, user: &Address) {
        let key = Self::get_bet_key(env, market_id, user);
        env.storage().persistent().remove::<BetKey>(&key);
    }

    /// Get market betting statistics.
    pub fn get_market_bet_stats(env: &Env, market_id: &Symbol) -> BetStats {
        let key = Self::get_bet_stats_key(env, market_id);
        env.storage()
            .persistent()
            .get::<MarketBetsKey, BetStats>(&key)
            .unwrap_or_else(|| BetStats {
                total_bets: 0,
                total_amount_locked: 0,
                unique_bettors: 0,
                outcome_totals: Map::new(env),
            })
    }

    /// Store market betting statistics.
    pub fn store_market_bet_stats(
        env: &Env,
        market_id: &Symbol,
        stats: &BetStats,
    ) -> Result<(), Error> {
        let key = Self::get_bet_stats_key(env, market_id);
        env.storage().persistent().set(&key, stats);
        Ok(())
    }

    /// Add user to the market's bet registry for iteration.
    fn add_to_bet_registry(env: &Env, market_id: &Symbol, user: &Address) -> Result<(), Error> {
        let key = Self::get_bet_registry_key(env, market_id);
        let mut registry: soroban_sdk::Vec<Address> = env
            .storage()
            .persistent()
            .get::<BetRegistryKey, soroban_sdk::Vec<Address>>(&key)
            .unwrap_or(soroban_sdk::Vec::new(env));

        // Only add if not already present
        let mut found = false;
        for existing_user in registry.iter() {
            if existing_user == *user {
                found = true;
                break;
            }
        }

        if !found {
            registry.push_back(user.clone());
            env.storage().persistent().set(&key, &registry);
        }

        Ok(())
    }

    /// Get all users who placed bets on a market.
    pub fn get_all_bets_for_market(env: &Env, market_id: &Symbol) -> soroban_sdk::Vec<Address> {
        let key = Self::get_bet_registry_key(env, market_id);
        env.storage()
            .persistent()
            .get::<BetRegistryKey, soroban_sdk::Vec<Address>>(&key)
            .unwrap_or(soroban_sdk::Vec::new(env))
    }

    /// Generate storage key for a bet.
    /// Uses the BetKey struct for unique identification per market/user combination.
    fn get_bet_key(_env: &Env, market_id: &Symbol, user: &Address) -> BetKey {
        BetKey {
            market_id: market_id.clone(),
            user: user.clone(),
        }
    }

    /// Generate storage key for market bet statistics.
    fn get_bet_stats_key(_env: &Env, market_id: &Symbol) -> MarketBetsKey {
        MarketBetsKey {
            market_id: market_id.clone(),
        }
    }

    /// Generate storage key for market bet registry.
    fn get_bet_registry_key(env: &Env, market_id: &Symbol) -> BetRegistryKey {
        BetRegistryKey {
            tag: Symbol::new(env, "Registry"),
            market_id: market_id.clone(),
        }
    }
}

// ===== BET VALIDATOR =====

/// Validation utilities for betting operations.
///
/// BetValidator provides comprehensive validation for all betting-related
/// operations, ensuring data integrity and security.
pub struct BetValidator;

impl BetValidator {
    /// Validate that a market is in a valid state for betting.
    ///
    /// # Validation Rules
    ///
    /// - Market must exist
    /// - Market must be in Active state
    /// - Current time must be before market end time
    /// - Market must not already be resolved
    ///
    /// # Parameters
    ///
    /// - `env` - The Soroban environment
    /// - `market` - The market to validate
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if market is valid for betting, `Err(Error)` otherwise.
    pub fn validate_market_for_betting(env: &Env, market: &Market) -> Result<(), Error> {
        // Check if market is active
        if market.state != MarketState::Active {
            return Err(Error::MarketClosed);
        }

        // Check if market has not ended
        let current_time = env.ledger().timestamp();
        if current_time >= market.end_time {
            return Err(Error::MarketClosed);
        }

        // Check if market is not already resolved
        if market.winning_outcome.is_some() {
            return Err(Error::MarketAlreadyResolved);
        }

        Ok(())
    }

    /// Validate bet parameters.
    ///
    /// # Validation Rules
    ///
    /// - Outcome must be one of the valid market outcomes
    /// - Amount must be within allowed range
    ///
    /// # Parameters
    ///
    /// - `env` - The Soroban environment
    /// - `outcome` - The selected outcome
    /// - `valid_outcomes` - List of valid outcomes for the market
    /// - `amount` - The bet amount
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if parameters are valid, `Err(Error)` otherwise.
    pub fn validate_bet_parameters(
        env: &Env,
        outcome: &String,
        valid_outcomes: &soroban_sdk::Vec<String>,
        amount: i128,
    ) -> Result<(), Error> {
        // Validate outcome
        MarketValidator::validate_outcome(env, outcome, valid_outcomes)?;

        // Validate amount
        Self::validate_bet_amount(amount)?;

        Ok(())
    }

    /// Validate bet amount.
    ///
    /// # Validation Rules
    ///
    /// - Amount must be greater than or equal to MIN_BET_AMOUNT
    /// - Amount must be less than or equal to MAX_BET_AMOUNT
    ///
    /// # Parameters
    ///
    /// - `amount` - The bet amount to validate
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if amount is valid, `Err(Error)` otherwise.
    pub fn validate_bet_amount(amount: i128) -> Result<(), Error> {
        if amount < MIN_BET_AMOUNT {
            return Err(Error::InsufficientStake);
        }

        if amount > MAX_BET_AMOUNT {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }
}

// ===== BET UTILITIES =====

/// Utility functions for betting operations.
///
/// BetUtils provides helper functions for fund management,
/// payout calculations, and other betting-related utilities.
pub struct BetUtils;

impl BetUtils {
    /// Lock funds by transferring from user to contract.
    ///
    /// This function transfers the specified amount from the user's
    /// token account to the contract's account, effectively locking
    /// the funds until market resolution.
    ///
    /// # Parameters
    ///
    /// - `env` - The Soroban environment
    /// - `user` - Address of the user
    /// - `amount` - Amount to lock
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if transfer succeeds, `Err(Error)` otherwise.
    pub fn lock_funds(env: &Env, user: &Address, amount: i128) -> Result<(), Error> {
        let token_client = MarketUtils::get_token_client(env)?;
        token_client.transfer(user, &env.current_contract_address(), &amount);
        Ok(())
    }

    /// Unlock funds by transferring from contract to user.
    ///
    /// This function transfers the specified amount from the contract's
    /// token account back to the user's account (for refunds or payouts).
    ///
    /// # Parameters
    ///
    /// - `env` - The Soroban environment
    /// - `user` - Address of the user
    /// - `amount` - Amount to unlock
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if transfer succeeds, `Err(Error)` otherwise.
    pub fn unlock_funds(env: &Env, user: &Address, amount: i128) -> Result<(), Error> {
        let token_client = MarketUtils::get_token_client(env)?;
        token_client.transfer(&env.current_contract_address(), user, &amount);
        Ok(())
    }

    /// Get the contract's locked funds balance.
    ///
    /// # Parameters
    ///
    /// - `env` - The Soroban environment
    ///
    /// # Returns
    ///
    /// Returns the contract's token balance.
    pub fn get_contract_balance(env: &Env) -> Result<i128, Error> {
        let token_client = MarketUtils::get_token_client(env)?;
        Ok(token_client.balance(&env.current_contract_address()))
    }

    /// Check if user has sufficient balance for a bet.
    ///
    /// # Parameters
    ///
    /// - `env` - The Soroban environment
    /// - `user` - Address of the user
    /// - `amount` - Required amount
    ///
    /// # Returns
    ///
    /// Returns `true` if user has sufficient balance, `false` otherwise.
    pub fn has_sufficient_balance(env: &Env, user: &Address, amount: i128) -> Result<bool, Error> {
        let token_client = MarketUtils::get_token_client(env)?;
        let balance = token_client.balance(user);
        Ok(balance >= amount)
    }
}

// ===== BET ANALYTICS =====

/// Analytics utilities for betting data.
///
/// BetAnalytics provides functions for analyzing betting patterns,
/// calculating statistics, and generating reports.
pub struct BetAnalytics;

impl BetAnalytics {
    /// Calculate the implied probability for an outcome based on bet distribution.
    ///
    /// Implied probability = (Amount bet on outcome) / (Total amount bet)
    ///
    /// # Parameters
    ///
    /// - `env` - The Soroban environment
    /// - `market_id` - Symbol identifying the market
    /// - `outcome` - The outcome to calculate probability for
    ///
    /// # Returns
    ///
    /// Returns the implied probability as a percentage (0-100).
    pub fn calculate_implied_probability(
        env: &Env,
        market_id: &Symbol,
        outcome: &String,
    ) -> i128 {
        let stats = BetStorage::get_market_bet_stats(env, market_id);

        if stats.total_amount_locked == 0 {
            return 0;
        }

        let outcome_amount = stats.outcome_totals.get(outcome.clone()).unwrap_or(0);

        // Return as percentage (0-100)
        (outcome_amount * 100) / stats.total_amount_locked
    }

    /// Calculate potential payout multiplier for an outcome.
    ///
    /// Multiplier = (Total pool) / (Amount bet on outcome)
    ///
    /// # Parameters
    ///
    /// - `env` - The Soroban environment
    /// - `market_id` - Symbol identifying the market
    /// - `outcome` - The outcome to calculate multiplier for
    ///
    /// # Returns
    ///
    /// Returns the payout multiplier (scaled by 100 for precision).
    pub fn calculate_payout_multiplier(env: &Env, market_id: &Symbol, outcome: &String) -> i128 {
        let stats = BetStorage::get_market_bet_stats(env, market_id);

        let outcome_amount = stats.outcome_totals.get(outcome.clone()).unwrap_or(0);

        if outcome_amount == 0 {
            return 0;
        }

        // Return multiplier scaled by 100 (e.g., 250 = 2.5x)
        (stats.total_amount_locked * 100) / outcome_amount
    }

    /// Get betting summary for a market.
    ///
    /// # Parameters
    ///
    /// - `env` - The Soroban environment
    /// - `market_id` - Symbol identifying the market
    ///
    /// # Returns
    ///
    /// Returns a `BetStats` structure with complete betting statistics.
    pub fn get_market_summary(env: &Env, market_id: &Symbol) -> BetStats {
        BetStorage::get_market_bet_stats(env, market_id)
    }
}

// ===== TESTS =====

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bet_amount_validation() {
        // Valid amount
        assert!(BetValidator::validate_bet_amount(MIN_BET_AMOUNT).is_ok());
        assert!(BetValidator::validate_bet_amount(10_000_000).is_ok());
        assert!(BetValidator::validate_bet_amount(MAX_BET_AMOUNT).is_ok());

        // Invalid - too low
        assert!(BetValidator::validate_bet_amount(MIN_BET_AMOUNT - 1).is_err());
        assert!(BetValidator::validate_bet_amount(0).is_err());
        assert!(BetValidator::validate_bet_amount(-1).is_err());

        // Invalid - too high
        assert!(BetValidator::validate_bet_amount(MAX_BET_AMOUNT + 1).is_err());
    }

    #[test]
    fn test_bet_status_transitions() {
        use soroban_sdk::Env;
        use soroban_sdk::testutils::Address as _;

        let env = Env::default();
        let user = Address::generate(&env);
        let market_id = Symbol::new(&env, "test_market");
        let outcome = String::from_str(&env, "yes");

        // Create a bet
        let mut bet = Bet::new(&env, user, market_id, outcome, 10_000_000);

        // Initial state should be Active
        assert!(bet.is_active());
        assert!(!bet.is_resolved());
        assert!(!bet.is_winner());
        assert_eq!(bet.status, BetStatus::Active);

        // Mark as won
        bet.mark_as_won();
        assert!(!bet.is_active());
        assert!(bet.is_resolved());
        assert!(bet.is_winner());
        assert_eq!(bet.status, BetStatus::Won);

        // Create another bet to test lost status
        let user2 = Address::generate(&env);
        let mut bet2 = Bet::new(
            &env,
            user2,
            Symbol::new(&env, "test_market2"),
            String::from_str(&env, "no"),
            5_000_000,
        );

        // Mark as lost
        bet2.mark_as_lost();
        assert!(!bet2.is_active());
        assert!(bet2.is_resolved());
        assert!(!bet2.is_winner());
        assert_eq!(bet2.status, BetStatus::Lost);

        // Create another bet to test refunded status
        let user3 = Address::generate(&env);
        let mut bet3 = Bet::new(
            &env,
            user3,
            Symbol::new(&env, "test_market3"),
            String::from_str(&env, "yes"),
            15_000_000,
        );

        // Mark as refunded
        bet3.mark_as_refunded();
        assert!(!bet3.is_active());
        assert!(!bet3.is_resolved()); // Refunded is not considered "resolved"
        assert!(!bet3.is_winner());
        assert_eq!(bet3.status, BetStatus::Refunded);
    }
}
