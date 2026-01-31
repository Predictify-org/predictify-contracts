#![no_std]
#![allow(unused_variables)]
#![allow(unused_assignments)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_mut)]
#![allow(deprecated)]

extern crate alloc;
extern crate wee_alloc;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

// Module declarations - all modules enabled
mod admin;
mod balances;
mod batch_operations;
mod bets;
mod circuit_breaker;
mod config;
mod disputes;
mod edge_cases;
mod errors;
mod event_archive;
mod events;
mod extensions;
mod fees;
mod governance;
mod graceful_degradation;
mod market_analytics;
mod market_id_generator;
mod markets;
mod monitoring;
mod oracles;
mod performance_benchmarks;
mod queries;
mod rate_limiter;
mod recovery;
mod reentrancy_guard;
mod resolution;
mod resolution_delay;
mod statistics;
mod storage;
mod types;
mod upgrade_manager;
mod utils;
mod validation;
mod validation_tests;
mod versioning;
mod voting;
// THis is the band protocol wasm std_reference.wasm
mod bandprotocol {
    soroban_sdk::contractimport!(file = "./std_reference.wasm");
}

#[cfg(test)]
mod circuit_breaker_tests;
#[cfg(test)]
mod oracle_fallback_timeout_tests;

#[cfg(test)]
mod batch_operations_tests;

#[cfg(test)]
mod integration_test;

#[cfg(test)]
mod recovery_tests;

#[cfg(test)]
mod property_based_tests;

#[cfg(test)]
mod upgrade_manager_tests;

#[cfg(test)]
mod query_tests;
mod bet_tests;

#[cfg(test)]
mod balance_tests;

#[cfg(test)]
mod event_management_tests;

#[cfg(test)]
mod category_tags_tests;
mod statistics_tests;

#[cfg(test)]
mod resolution_delay_tests;
mod resolution_delay_dispute_window_tests;

#[cfg(test)]
mod event_creation_tests;

// Re-export commonly used items
use admin::{AdminAnalyticsResult, AdminInitializer, AdminManager, AdminPermission, AdminRole};
pub use errors::Error;
pub use queries::QueryManager;
pub use types::*;

use crate::config::{
    DEFAULT_PLATFORM_FEE_PERCENTAGE, MAX_PLATFORM_FEE_PERCENTAGE, MIN_PLATFORM_FEE_PERCENTAGE,
};
use crate::events::EventEmitter;
use crate::graceful_degradation::{OracleBackup, OracleHealth};
use crate::market_id_generator::MarketIdGenerator;
use crate::reentrancy_guard::ReentrancyGuard;
use crate::resolution::OracleResolution;
use alloc::format;
use soroban_sdk::{
    contract, contractimpl, panic_with_error, Address, Env, Map, String, Symbol, Vec,
};

#[contract]
pub struct PredictifyHybrid;

const PERCENTAGE_DENOMINATOR: i128 = 100;

#[contractimpl]
impl PredictifyHybrid {
    // Recovery methods appended later in file after existing functions to maintain readability.
    /// Initializes the Predictify Hybrid smart contract with administrator and platform configuration.
    ///
    /// This function must be called once after contract deployment to set up the initial
    /// administrative configuration and platform fee structure. It establishes the contract admin who
    /// will have privileges to create markets and perform administrative functions, and configures
    /// the platform fee percentage for market operations.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `admin` - The address that will be granted administrative privileges
    /// * `platform_fee_percentage` - Optional platform fee percentage (0-10%). If `None`, defaults to 2%
    ///
    /// # Panics
    ///
    /// This function will panic if:
    /// - The contract has already been initialized (Error code 504: AlreadyInitialized)
    /// - The admin address is invalid
    /// - The platform fee percentage is negative or exceeds 10%
    /// - Storage operations fail
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let admin_address = Address::generate(&env);
    ///
    /// // Initialize with default 2% platform fee
    /// PredictifyHybrid::initialize(env.clone(), admin_address.clone(), None);
    ///
    /// // Or initialize with custom 5% platform fee
    /// PredictifyHybrid::initialize(env.clone(), admin_address, Some(5));
    /// ```
    ///
    /// # Platform Fee
    ///
    /// The platform fee is a percentage (0-10%) taken from winning payouts to support
    /// platform operations. Fee is applied during payout calculation:
    /// - Default: 2% (200 basis points)
    /// - Minimum: 0% (no fee)
    /// - Maximum: 10% (1000 basis points)
    ///
    /// # Security
    ///
    /// The admin address should be carefully chosen as it will have significant
    /// control over the contract's operation, including market creation and resolution.
    /// Consider using a multi-signature wallet or governance contract for production.
    ///
    /// # Re-initialization Prevention
    ///
    /// This function can only be called once. Any subsequent calls will panic with
    /// `Error::AlreadyInitialized` to prevent admin takeover attacks.
    pub fn initialize(env: Env, admin: Address, platform_fee_percentage: Option<i128>) {
        // Determine platform fee (default 2% if not specified)
        let fee_percentage = platform_fee_percentage.unwrap_or(DEFAULT_PLATFORM_FEE_PERCENTAGE);

        // Validate fee percentage bounds (0-10%)
        if fee_percentage < MIN_PLATFORM_FEE_PERCENTAGE
            || fee_percentage > MAX_PLATFORM_FEE_PERCENTAGE
        {
            panic_with_error!(env, Error::InvalidFeeConfig);
        }

        // Initialize admin (includes re-initialization check)
        match AdminInitializer::initialize(&env, &admin) {
            Ok(_) => (),
            Err(e) => panic_with_error!(env, e),
        }

        // Store platform fee configuration in persistent storage
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, "platform_fee"), &fee_percentage);

        // Emit contract initialized event
        EventEmitter::emit_contract_initialized(&env, &admin, fee_percentage);

        // Emit platform fee set event
        EventEmitter::emit_platform_fee_set(&env, fee_percentage, &admin);
    }

    /// Deposits funds into the user's balance.
    ///
    /// # Parameters
    /// * `env` - The environment.
    /// * `user` - The user depositing funds.
    /// * `asset` - The asset to deposit (e.g., XLM, BTC, ETH).
    /// * `amount` - The amount to deposit.
    pub fn deposit(
        env: Env,
        user: Address,
        asset: ReflectorAsset,
        amount: i128,
    ) -> Result<Balance, Error> {
        balances::BalanceManager::deposit(&env, user, asset, amount)
    }

    /// Withdraws funds from the user's balance.
    ///
    /// # Parameters
    /// * `env` - The environment.
    /// * `user` - The user withdrawing funds.
    /// * `asset` - The asset to withdraw.
    /// * `amount` - The amount to withdraw.
    pub fn withdraw(
        env: Env,
        user: Address,
        asset: ReflectorAsset,
        amount: i128,
    ) -> Result<Balance, Error> {
        balances::BalanceManager::withdraw(&env, user, asset, amount)
    }

    /// Gets the current balance of a user for a specific asset.
    ///
    /// # Parameters
    /// * `env` - The environment.
    /// * `user` - The user to check.
    /// * `asset` - The asset to check.
    pub fn get_balance(env: Env, user: Address, asset: ReflectorAsset) -> Balance {
        storage::BalanceStorage::get_balance(&env, &user, &asset)
    }

    /// Creates a new prediction market with specified parameters and oracle configuration.
    ///
    /// This function allows authorized administrators to create prediction markets
    /// with custom questions, possible outcomes, duration, and oracle integration.
    /// Each market gets a unique identifier and is stored in persistent contract storage.
    ///
    /// # Multi-Outcome Support
    ///
    /// Markets support 2 to N outcomes, enabling both binary (yes/no) and multi-outcome
    /// markets (e.g., Team A / Team B / Draw). The contract handles:
    /// - Single winner resolution (one outcome wins)
    /// - Tie/multi-winner resolution (multiple outcomes win, pool split proportionally)
    /// - Outcome validation during bet placement
    /// - Proportional payout distribution for ties
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `admin` - The administrator address creating the market (must be authorized)
    /// * `question` - The prediction question (must be non-empty)
    /// * `outcomes` - Vector of possible outcomes (minimum 2 required, all non-empty, no duplicates)
    /// * `duration_days` - Market duration in days (must be between 1-365 days)
    /// * `oracle_config` - Configuration for oracle integration (Reflector, Pyth, etc.)
    ///
    /// # Returns
    ///
    /// Returns a unique `Symbol` that serves as the market identifier for all future operations.
    ///
    /// # Panics
    ///
    /// This function will panic with specific errors if:
    /// - `Error::Unauthorized` - Caller is not the contract admin
    /// - `Error::InvalidQuestion` - Question is empty
    /// - `Error::InvalidOutcomes` - Less than 2 outcomes or any outcome is empty
    /// - Storage operations fail
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address, String, Vec};
    /// # use predictify_hybrid::{PredictifyHybrid, OracleConfig, OracleType};
    /// # let env = Env::default();
    /// # let admin = Address::generate(&env);
    ///
    /// let question = String::from_str(&env, "Will Bitcoin reach $100,000 by 2024?");
    /// let outcomes = vec![
    ///     String::from_str(&env, "Yes"),
    ///     String::from_str(&env, "No")
    /// ];
    /// let oracle_config = OracleConfig {
    ///     oracle_type: OracleType::Reflector,
    ///     oracle_contract: Address::generate(&env),
    ///     asset_code: Some(String::from_str(&env, "BTC")),
    ///     threshold_value: Some(100000),
    /// };
    ///
    /// let market_id = PredictifyHybrid::create_market(
    ///     env.clone(),
    ///     admin,
    ///     question,
    ///     outcomes,
    ///     30, // 30 days duration
    ///     oracle_config
    /// );
    /// ```
    ///
    /// # Multi-Outcome Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address, String, Vec};
    /// # use predictify_hybrid::{PredictifyHybrid, OracleConfig, OracleProvider};
    /// # let env = Env::default();
    /// # let admin = Address::generate(&env);
    ///
    /// // Create a 3-outcome market (e.g., match result)
    /// let question = String::from_str(&env, "Match result?");
    /// let outcomes = vec![
    ///     &env,
    ///     String::from_str(&env, "Team A"),
    ///     String::from_str(&env, "Team B"),
    ///     String::from_str(&env, "Draw"),
    /// ];
    /// let oracle_config = OracleConfig::new(
    ///     OracleProvider::Reflector,
    ///     String::from_str(&env, "BTC/USD"),
    ///     50_000_00,
    ///     String::from_str(&env, "gt"),
    /// );
    ///
    /// let market_id = PredictifyHybrid::create_market(
    ///     env.clone(),
    ///     admin,
    ///     question,
    ///     outcomes,
    ///     30,
    ///     oracle_config
    /// );
    /// ```
    ///
    /// # Market State
    ///
    /// New markets are created in `MarketState::Active` state, allowing immediate voting.
    /// The market will automatically transition to `MarketState::Ended` when the duration expires.
    pub fn create_market(
        env: Env,
        admin: Address,
        question: String,
        outcomes: Vec<String>,
        duration_days: u32,
        oracle_config: OracleConfig,
        fallback_oracle_config: Option<OracleConfig>,
        resolution_timeout: u64,
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

        // Generate a unique collision-resistant market ID
        let market_id = MarketIdGenerator::generate_market_id(&env, &admin);

        // Calculate end time
        let seconds_per_day: u64 = 24 * 60 * 60;
        let duration_seconds: u64 = (duration_days as u64) * seconds_per_day;
        let end_time: u64 = env.ledger().timestamp() + duration_seconds;

        // Create a new market
        let market = Market {
            admin: admin.clone(),
            question: question.clone(),
            outcomes: outcomes.clone(),
            end_time,
            oracle_config,
            fallback_oracle_config,
            resolution_timeout,
            oracle_result: None,
            votes: Map::new(&env),
            total_staked: 0,
            dispute_stakes: Map::new(&env),
            stakes: Map::new(&env),
            claimed: Map::new(&env),
            winning_outcomes: None,
            fee_collected: false,
            state: MarketState::Active,
            total_extension_days: 0,
            max_extension_days: 30,
            extension_history: Vec::new(&env),
            // Resolution delay fields
            resolution_proposed_outcome: None,
            resolution_proposed_at: 0,
            resolution_window_end_time: 0,
            resolution_is_finalized: false,
            resolution_dispute_count: 0,
            resolution_source: None,
            dispute_window_hours: 0, // 0 means use global config
            category: None,
            tags: Vec::new(&env),
        };

        // Store the market
        env.storage().persistent().set(&market_id, &market);

        // Emit market created event
        EventEmitter::emit_market_created(&env, &market_id, &question, &outcomes, &admin, end_time);

        // Record statistics
        statistics::StatisticsManager::record_market_created(&env);

        market_id
    }

    /// Creates a new prediction event with specified parameters.
    ///
    /// This function allows authorized admins to create prediction events
    /// with specific descriptions, possible outcomes, and end times. Unlike `create_market`,
    /// this function accepts an absolute Unix timestamp for the end time.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment
    /// * `admin` - The administrator address (must be authorized)
    /// * `description` - The event description or question
    /// * `outcomes` - Vector of possible outcomes
    /// * `end_time` - Absolute Unix timestamp for when the event ends
    /// * `oracle_config` - Configuration for oracle integration
    ///
    /// # Returns
    ///
    /// Returns a unique `Symbol` serving as the event identifier.
    ///
    /// # Panics
    ///
    /// Panics if:
    /// - Caller is not the contract admin
    /// - validation fails (invalid description, outcomes, or end time)
    pub fn create_event(
        env: Env,
        admin: Address,
        description: String,
        outcomes: Vec<String>,
        end_time: u64,
        oracle_config: OracleConfig,
        fallback_oracle_config: Option<OracleConfig>,
        resolution_timeout: u64,
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

        // Validate inputs using EventValidator
        if let Err(e) = crate::validation::EventValidator::validate_event_creation(
            &env,
            &admin,
            &description,
            &outcomes,
            &end_time,
        ) {
            panic_with_error!(env, e.to_contract_error());
        }

        // Generate a unique collision-resistant event ID (reusing market ID generator)
        let event_id = MarketIdGenerator::generate_market_id(&env, &admin);

        // Create a new event
        let event = Event {
            id: event_id.clone(),
            description: description.clone(),
            outcomes: outcomes.clone(),
            end_time,
            oracle_config,
            fallback_oracle_config,
            resolution_timeout,
            admin: admin.clone(),
            created_at: env.ledger().timestamp(),
            status: MarketState::Active,
        };

        // Store the event
        crate::storage::EventManager::store_event(&env, &event);

        // Emit event created event
        EventEmitter::emit_event_created(
            &env,
            &event_id,
            &description,
            &outcomes,
            &admin,
            end_time,
        );

        // Record statistics (optional, can reuse market stats for now)
        // statistics::StatisticsManager::record_market_created(&env);

        event_id
    }

    /// Retrieves an event by its unique identifier.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment
    /// * `event_id` - Unique identifier of the event to retrieve
    ///
    /// # Returns
    ///
    /// Returns `Some(Event)` if found, or `None` otherwise.
    pub fn get_event(env: Env, event_id: Symbol) -> Option<Event> {
        crate::storage::EventManager::get_event(&env, &event_id).ok()
    }

    /// Allows users to vote on a market outcome by staking tokens.
    ///
    /// This function enables users to participate in prediction markets by voting
    /// for their predicted outcome and staking tokens to back their prediction.
    /// Users can only vote once per market, and votes cannot be changed after submission.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `user` - The address of the user casting the vote (must be authenticated)
    /// * `market_id` - Unique identifier of the market to vote on
    /// * `outcome` - The outcome the user is voting for (must match a market outcome)
    /// * `stake` - Amount of tokens to stake on this prediction (in base token units)
    ///
    /// # Panics
    ///
    /// This function will panic with specific errors if:
    /// - `Error::MarketNotFound` - Market with given ID doesn't exist
    /// - `Error::MarketClosed` - Market voting period has ended
    /// - `Error::InvalidOutcome` - Outcome doesn't match any market outcomes
    /// - `Error::AlreadyVoted` - User has already voted on this market
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address, String, Symbol};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let user = Address::generate(&env);
    /// # let market_id = Symbol::new(&env, "market_1");
    ///
    /// // Vote "Yes" with 1000 token units stake
    /// PredictifyHybrid::vote(
    ///     env.clone(),
    ///     user,
    ///     market_id,
    ///     String::from_str(&env, "Yes"),
    ///     1000
    /// );
    /// ```
    ///
    /// # Token Staking
    ///
    /// The stake amount represents the user's confidence in their prediction.
    /// Higher stakes increase potential rewards but also increase risk.
    /// Stakes are locked until market resolution and cannot be withdrawn early.
    ///
    /// # Market State Requirements
    ///
    /// - Market must be in `Active` state
    /// - Current time must be before market end time
    /// - Market must not be cancelled or resolved
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

        // Lock funds (transfer from user to contract)
        match bets::BetUtils::lock_funds(&env, &user, stake) {
            Ok(_) => {}
            Err(e) => panic_with_error!(env, e),
        }

        // Store the vote and stake
        market.votes.set(user.clone(), outcome.clone());
        market.stakes.set(user.clone(), stake);
        market.total_staked += stake;

        env.storage().persistent().set(&market_id, &market);

        // Emit vote cast event
        EventEmitter::emit_vote_cast(&env, &market_id, &user, &outcome, stake);
    }

    /// Places a bet on a prediction market event by locking user funds.
    ///
    /// This function enables users to place bets on active prediction markets,
    /// selecting an outcome they predict will occur and locking funds as their wager.
    /// Bets are distinct from votes - bets represent financial wagers while votes
    /// participate in community resolution consensus.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `user` - The address of the user placing the bet (must be authenticated)
    /// * `market_id` - Unique identifier of the market to bet on
    /// * `outcome` - The outcome the user predicts will occur
    /// * `amount` - Amount of tokens to lock for this bet (in base token units)
    ///
    /// # Returns
    ///
    /// Returns the created `Bet` struct containing bet details on success.
    ///
    /// # Panics
    ///
    /// This function will panic with specific errors if:
    /// - `Error::MarketNotFound` - Market with given ID doesn't exist
    /// - `Error::MarketClosed` - Market betting period has ended or market is not active
    /// - `Error::MarketResolved` - Market has already been resolved
    /// - `Error::InvalidOutcome` - Outcome doesn't match any market outcomes
    /// - `Error::AlreadyBet` - User has already placed a bet on this market
    /// - `Error::InsufficientStake` - Bet amount is below minimum (0.1 XLM)
    /// - `Error::InvalidInput` - Bet amount exceeds maximum (10,000 XLM)
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address, String, Symbol};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let user = Address::generate(&env);
    /// # let market_id = Symbol::new(&env, "btc_50k");
    ///
    /// // Place a bet of 1 XLM on "Yes" outcome
    /// let bet = PredictifyHybrid::place_bet(
    ///     env.clone(),
    ///     user,
    ///     market_id,
    ///     String::from_str(&env, "Yes"),
    ///     10_000_000 // 1.0 XLM in stroops
    /// );
    /// ```
    ///
    /// # Fund Locking
    ///
    /// When a bet is placed:
    /// 1. User's funds (XLM or Stellar tokens) are transferred to the contract
    /// 2. Funds remain locked until market resolution
    /// 3. Upon resolution:
    ///    - Winners receive proportional share of total bet pool (minus fees)
    ///    - Losers forfeit their locked funds
    ///    - Refunds issued if market is cancelled
    ///
    /// # Double Betting Prevention
    ///
    /// Users can only place ONE bet per market. Attempting to bet again will
    /// result in an `Error::AlreadyBet` error. This ensures fair distribution
    /// of rewards and prevents manipulation.
    ///
    /// # Market State Requirements
    ///
    /// - Market must be in `Active` state
    /// - Current time must be before market end time
    /// - Market must not be resolved or cancelled
    ///
    /// # Security
    ///
    /// - User authentication via `require_auth()`
    /// - Balance validation before fund transfer
    /// - Atomic fund locking with bet creation
    /// - Reentrancy protection via reentrancy guard (guard flag in storage)
    /// Places a bet on a specific outcome in a prediction market.
    ///
    /// This function allows users to place bets on markets with 2 or more outcomes.
    /// The outcome must be one of the valid outcomes defined when the market was created.
    /// Users can only place one bet per market.
    ///
    /// # Multi-Outcome Support
    ///
    /// - Validates that the selected outcome exists in the market's outcome list
    /// - Works with binary (2 outcomes) and multi-outcome (N outcomes) markets
    /// - Rejects invalid outcomes that don't match any market outcome
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `user` - The address of the user placing the bet (must be authenticated)
    /// * `market_id` - Unique identifier of the market to bet on
    /// * `outcome` - The outcome to bet on (must match one of the market's outcomes)
    /// * `amount` - Amount of tokens to bet (must meet minimum/maximum bet limits)
    ///
    /// # Returns
    ///
    /// Returns the created `Bet` struct containing bet details.
    ///
    /// # Panics
    ///
    /// This function will panic with specific errors if:
    /// - `Error::MarketNotFound` - Market with given ID doesn't exist
    /// - `Error::MarketClosed` - Market is not active or has ended
    /// - `Error::InvalidOutcome` - Outcome doesn't match any market outcomes
    /// - `Error::AlreadyBet` - User has already placed a bet on this market
    /// - `Error::InsufficientStake` - Bet amount is below minimum
    /// - `Error::InvalidInput` - Bet amount exceeds maximum
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address, Symbol, String};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let user = Address::generate(&env);
    /// # let market_id = Symbol::new(&env, "market_1");
    ///
    /// // Place bet on "Team A" outcome
    /// let bet = PredictifyHybrid::place_bet(
    ///     env.clone(),
    ///     user,
    ///     market_id,
    ///     String::from_str(&env, "Team A"),
    ///     10_0000000, // 10 XLM
    /// );
    /// ```
    pub fn place_bet(
        env: Env,
        user: Address,
        market_id: Symbol,
        outcome: String,
        amount: i128,
    ) -> crate::types::Bet {
        if ReentrancyGuard::check_reentrancy_state(&env).is_err() {
            panic_with_error!(env, Error::InvalidState);
        }
        // Use the BetManager to handle the bet placement
        match bets::BetManager::place_bet(&env, user.clone(), market_id, outcome, amount) {
            Ok(bet) => {
                // Record statistics
                statistics::StatisticsManager::record_bet_placed(&env, &user, amount);
                bet
            }
            Err(e) => panic_with_error!(env, e),
        }
    }

    /// Places multiple bets in a single atomic transaction.
    ///
    /// This function enables users to place multiple bets across different markets
    /// or outcomes in a single transaction, providing gas efficiency and atomicity.
    /// All bets must succeed or the entire transaction reverts.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `user` - The address of the user placing the bets (must be authenticated)
    /// * `bets` - Vector of tuples containing (market_id, outcome, amount) for each bet
    ///
    /// # Returns
    ///
    /// Returns a `Vec<Bet>` containing all successfully placed bets.
    ///
    /// # Panics
    ///
    /// This function will panic with specific errors if:
    /// - Any bet fails validation (market not found, closed, invalid outcome, etc.)
    /// - User has insufficient balance for the total amount
    /// - User has already bet on any of the markets
    /// - Any bet amount is below minimum or above maximum
    /// - The batch is empty or exceeds maximum batch size
    ///
    /// # Atomicity
    ///
    /// All bets are validated before any funds are locked. If any single bet
    /// fails validation, the entire transaction reverts with no state changes.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address, String, Symbol, Vec};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let user = Address::generate(&env);
    ///
    /// let bets = vec![
    ///     &env,
    ///     (
    ///         Symbol::new(&env, "btc_100k"),
    ///         String::from_str(&env, "yes"),
    ///         10_000_000i128  // 1.0 XLM
    ///     ),
    ///     (
    ///         Symbol::new(&env, "eth_5k"),
    ///         String::from_str(&env, "no"),
    ///         5_000_000i128   // 0.5 XLM
    ///     ),
    /// ];
    ///
    /// let placed_bets = PredictifyHybrid::place_bets(env.clone(), user, bets);
    /// ```
    pub fn place_bets(
        env: Env,
        user: Address,
        bets: Vec<(Symbol, String, i128)>,
    ) -> Vec<crate::types::Bet> {
        if ReentrancyGuard::check_reentrancy_state(&env).is_err() {
            panic_with_error!(env, Error::InvalidState);
        }
        match bets::BetManager::place_bets(&env, user, bets) {
            Ok(placed_bets) => placed_bets,
            Err(e) => panic_with_error!(env, e),
        }
    }

    /// Retrieves a user's bet on a specific market.
    ///
    /// This function provides read-only access to a user's bet details including
    /// the selected outcome, locked amount, and bet status.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `market_id` - Unique identifier of the market
    /// * `user` - Address of the user whose bet to retrieve
    ///
    /// # Returns
    ///
    /// Returns `Some(Bet)` if the user has placed a bet on this market,
    /// `None` if no bet exists.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address, Symbol};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let user = Address::generate(&env);
    /// # let market_id = Symbol::new(&env, "btc_50k");
    ///
    /// match PredictifyHybrid::get_bet(env.clone(), market_id, user) {
    ///     Some(bet) => {
    ///         // User has a bet
    ///         println!("Bet amount: {}", bet.amount);
    ///         println!("Selected outcome: {:?}", bet.outcome);
    ///         println!("Status: {:?}", bet.status);
    ///     },
    ///     None => {
    ///         // User has not placed a bet on this market
    ///     }
    /// }
    /// ```
    pub fn get_bet(env: Env, market_id: Symbol, user: Address) -> Option<crate::types::Bet> {
        bets::BetManager::get_bet(&env, &market_id, &user)
    }

    /// Checks if a user has already placed a bet on a specific market.
    ///
    /// This function provides a quick check to determine if a user has
    /// an existing bet on a market before attempting to place a new bet.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `market_id` - Unique identifier of the market
    /// * `user` - Address of the user to check
    ///
    /// # Returns
    ///
    /// Returns `true` if the user has already placed a bet, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address, Symbol};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let user = Address::generate(&env);
    /// # let market_id = Symbol::new(&env, "btc_50k");
    ///
    /// if PredictifyHybrid::has_user_bet(env.clone(), market_id.clone(), user.clone()) {
    ///     println!("User has already placed a bet on this market");
    /// } else {
    ///     println!("User can place a bet");
    /// }
    /// ```
    pub fn has_user_bet(env: Env, market_id: Symbol, user: Address) -> bool {
        bets::BetManager::has_user_bet(&env, &market_id, &user)
    }

    /// Retrieves betting statistics for a specific market.
    ///
    /// This function provides aggregate information about betting activity
    /// on a market, including total bets, locked amounts, and per-outcome totals.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `market_id` - Unique identifier of the market
    ///
    /// # Returns
    ///
    /// Returns `BetStats` with comprehensive betting statistics.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Symbol};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let market_id = Symbol::new(&env, "btc_50k");
    ///
    /// let stats = PredictifyHybrid::get_market_bet_stats(env.clone(), market_id);
    /// println!("Total bets: {}", stats.total_bets);
    /// println!("Total locked: {} stroops", stats.total_amount_locked);
    /// println!("Unique bettors: {}", stats.unique_bettors);
    /// ```
    pub fn get_market_bet_stats(env: Env, market_id: Symbol) -> crate::types::BetStats {
        bets::BetManager::get_market_bet_stats(&env, &market_id)
    }

    /// Calculate the payout amount for a user's bet on a resolved market.
    ///
    /// This function calculates how much a user will receive if they won their bet.
    /// For multi-outcome markets with ties, the payout is calculated based on
    /// the proportional share of the total pool split among all winners.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `market_id` - Unique identifier of the market
    /// * `user` - Address of the user to calculate payout for
    ///
    /// # Returns
    ///
    /// Returns `Ok(i128)` with the payout amount in base token units, or `Err(Error)` if calculation fails.
    /// Returns `Ok(0)` if the user didn't win or has no bet.
    ///
    /// # Errors
    ///
    /// - `Error::MarketNotFound` - Market doesn't exist
    /// - `Error::MarketNotResolved` - Market hasn't been resolved yet
    /// - `Error::NothingToClaim` - User has no bet on this market
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address, Symbol};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let market_id = Symbol::new(&env, "resolved_market");
    /// # let user = Address::generate(&env);
    ///
    /// match PredictifyHybrid::calculate_bet_payout(env.clone(), market_id, user) {
    ///     Ok(payout) => println!("User will receive {} stroops", payout),
    ///     Err(e) => println!("Calculation failed: {:?}", e),
    /// }
    /// ```
    ///
    /// # Payout Calculation for Ties
    ///
    /// When multiple outcomes win (tie):
    /// - Total pool is split proportionally among all winners
    /// - Each winner's payout = (their_stake / total_winning_stakes) * total_pool * (1 - fee)
    /// - This ensures fair distribution even when outcomes are tied
    /// Calculates the payout amount for a user's bet on a resolved market.
    ///
    /// This function computes the payout based on:
    /// - Whether the user's bet outcome is a winning outcome
    /// - The user's stake relative to total winning stakes
    /// - The total pool size
    /// - Platform fees
    ///
    /// # Multi-Outcome Support
    ///
    /// For markets with multiple winning outcomes (ties):
    /// - Payouts are calculated proportionally across all winning outcomes
    /// - Total winning stakes = sum of all stakes on all winning outcomes
    /// - User's share = (user_stake / total_winning_stakes) * total_pool * (1 - fee)
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `market_id` - Unique identifier of the market
    /// * `user` - Address of the user whose payout to calculate
    ///
    /// # Returns
    ///
    /// Returns `Ok(i128)` with the payout amount in base token units if:
    /// - Market is resolved
    /// - User placed a bet
    /// - User's outcome is a winning outcome
    ///
    /// Returns `Err(Error)` if:
    /// - Market is not resolved
    /// - User has no bet
    /// - User's outcome did not win
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address, Symbol};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let user = Address::generate(&env);
    /// # let market_id = Symbol::new(&env, "market_1");
    ///
    /// // Calculate payout for user's winning bet
    /// match PredictifyHybrid::calculate_bet_payout(env.clone(), market_id, user) {
    ///     Ok(payout) => println!("Payout: {}", payout),
    ///     Err(e) => println!("Error: {:?}", e),
    /// }
    /// ```
    pub fn calculate_bet_payout(env: Env, market_id: Symbol, user: Address) -> Result<i128, Error> {
        bets::BetManager::calculate_bet_payout(&env, &market_id, &user)
    }

    /// Calculates the implied probability for an outcome based on bet distribution.
    ///
    /// The implied probability indicates the market's collective prediction for
    /// an outcome based on the distribution of bets.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment
    /// * `market_id` - Unique identifier of the market
    /// * `outcome` - The outcome to calculate probability for
    ///
    /// # Returns
    ///
    /// Returns the implied probability as a percentage (0-100).
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Symbol, String};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let market_id = Symbol::new(&env, "btc_50k");
    ///
    /// let prob = PredictifyHybrid::get_implied_probability(
    ///     env.clone(),
    ///     market_id,
    ///     String::from_str(&env, "Yes")
    /// );
    /// println!("Implied probability for 'Yes': {}%", prob);
    /// ```
    pub fn get_implied_probability(env: Env, market_id: Symbol, outcome: String) -> i128 {
        bets::BetAnalytics::calculate_implied_probability(&env, &market_id, &outcome)
    }

    /// Calculates the potential payout multiplier for an outcome.
    ///
    /// The multiplier indicates how much a bet would pay out relative to
    /// the bet amount if the selected outcome wins.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment
    /// * `market_id` - Unique identifier of the market
    /// * `outcome` - The outcome to calculate multiplier for
    ///
    /// # Returns
    ///
    /// Returns the payout multiplier scaled by 100 (e.g., 250 = 2.5x).
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Symbol, String};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let market_id = Symbol::new(&env, "btc_50k");
    ///
    /// let multiplier = PredictifyHybrid::get_payout_multiplier(
    ///     env.clone(),
    ///     market_id,
    ///     String::from_str(&env, "Yes")
    /// );
    /// let actual_multiplier = multiplier as f64 / 100.0;
    /// println!("Payout multiplier for 'Yes': {:.2}x", actual_multiplier);
    /// ```
    pub fn get_payout_multiplier(env: Env, market_id: Symbol, outcome: String) -> i128 {
        bets::BetAnalytics::calculate_payout_multiplier(&env, &market_id, &outcome)
    }

    /// Allows users to claim their winnings from resolved prediction markets.
    ///
    /// This function enables users who voted for the winning outcome to claim
    /// their proportional share of the total market pool, minus platform fees.
    /// Users can only claim once per market, and only after the market is resolved.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `user` - The address of the user claiming winnings (must be authenticated)
    /// * `market_id` - Unique identifier of the resolved market
    ///
    /// # Panics
    ///
    /// This function will panic with specific errors if:
    /// - `Error::MarketNotFound` - Market with given ID doesn't exist
    /// - `Error::AlreadyClaimed` - User has already claimed winnings from this market
    /// - `Error::MarketNotResolved` - Market hasn't been resolved yet
    /// - `Error::NothingToClaim` - User didn't vote or voted for losing outcome
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address, Symbol};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let user = Address::generate(&env);
    /// # let market_id = Symbol::new(&env, "resolved_market");
    ///
    /// // Claim winnings from a resolved market
    /// PredictifyHybrid::claim_winnings(
    ///     env.clone(),
    ///     user,
    ///     market_id
    /// );
    /// ```
    ///
    /// # Payout Calculation
    ///
    /// Winnings are calculated using the formula:
    /// ```text
    /// user_payout = (user_stake * (100 - fee_percentage) / 100) * total_pool / winning_total
    /// ```
    ///
    /// Where:
    /// - `user_stake` - Amount the user staked on the winning outcome
    /// - `fee_percentage` - Platform fee (currently 2%)
    /// - `total_pool` - Sum of all stakes in the market
    /// - `winning_total` - Sum of stakes on the winning outcome
    ///
    /// # Market State Requirements
    ///
    /// - Market must be in `Resolved` state with a winning outcome set
    /// - User must have voted for the winning outcome
    /// - User must not have previously claimed winnings
    pub fn claim_winnings(env: Env, user: Address, market_id: Symbol) {
        user.require_auth();
        if ReentrancyGuard::check_reentrancy_state(&env).is_err() {
            panic_with_error!(env, Error::InvalidState);
        }

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

        // NEW: Verify resolution is finalized (dispute window closed)
        // This ensures payouts only occur after the dispute window closes
        // and all disputes are resolved
        if !resolution_delay::ResolutionDelayManager::is_resolution_finalized(&env, &market_id) {
            panic_with_error!(env, Error::MarketNotResolved);
        }

        // Check if market is resolved
        let winning_outcomes = match &market.winning_outcomes {
            Some(outcomes) => outcomes,
            None => panic_with_error!(env, Error::MarketNotResolved),
        };

        // Get user's vote
        let user_outcome = market
            .votes
            .get(user.clone())
            .unwrap_or_else(|| panic_with_error!(env, Error::NothingToClaim));

        let user_stake = market.stakes.get(user.clone()).unwrap_or(0);

        // Calculate payout if user won (check if outcome is in winning outcomes)
        if winning_outcomes.contains(&user_outcome) {
            // Calculate total winning stakes across all winning outcomes
            let mut winning_total = 0;
            for (voter, outcome) in market.votes.iter() {
                if winning_outcomes.contains(&outcome) {
                    winning_total += market.stakes.get(voter.clone()).unwrap_or(0);
                }
            }

            if winning_total > 0 {
                // Retrieve dynamic platform fee percentage from configuration
                let cfg = match crate::config::ConfigManager::get_config(&env) {
                    Ok(c) => c,
                    Err(_) => panic_with_error!(env, Error::ConfigNotFound),
                };
                let fee_percent = cfg.fees.platform_fee_percentage;
                let user_share = (user_stake
                    .checked_mul(PERCENTAGE_DENOMINATOR - fee_percent)
                    .unwrap_or_else(|| panic_with_error!(env, Error::InvalidInput)))
                    / PERCENTAGE_DENOMINATOR;
                let total_pool = market.total_staked;
                let product = user_share
                    .checked_mul(total_pool)
                    .unwrap_or_else(|| panic_with_error!(env, Error::InvalidInput));
                let payout = product / winning_total;

                // Calculate fee amount for statistics
                // Payout is net of fee. Fee was deducted in user_share calculation.
                // Gross payout would be (user_stake * total_pool) / winning_total
                // Logic check:
                // user_share = user_stake * (1 - fee)
                // payout = user_share * pool / winning_total
                // payout = user_stake * (1-fee) * pool / winning_total
                // payout = (user_stake * pool / winning_total) - (user_stake * pool / winning_total * fee)
                // So Fee = (user_stake * pool / winning_total) * fee
                // Or Fee = Payout / (1 - fee) * fee ? No, division precision.
                // Simpler: Fee = (Payout * fee_percent) / (100 - fee_percent)?
                // Let's rely on explicit calculation if possible or approximation.
                // Actually, let's re-calculate gross to get fee.
                // Gross = (user_stake * total_pool) / winning_total.
                // Fee = Gross - Payout.

                let gross_share = (user_stake
                    .checked_mul(PERCENTAGE_DENOMINATOR)
                    .unwrap_or_else(|| panic_with_error!(env, Error::InvalidInput)))
                    / PERCENTAGE_DENOMINATOR;
                // Wait, user_stake * 100 / 100 = user_stake.
                // The math above used PERCENTAGE_DENOMINATOR (100).

                let product_gross = user_stake
                    .checked_mul(total_pool)
                    .unwrap_or_else(|| panic_with_error!(env, Error::InvalidInput));
                let gross_payout = product_gross / winning_total;
                let fee_amount = gross_payout - payout;

                statistics::StatisticsManager::record_winnings_claimed(&env, &user, payout);
                statistics::StatisticsManager::record_fees_collected(&env, fee_amount);

                // Mark as claimed
                market.claimed.set(user.clone(), true);
                env.storage().persistent().set(&market_id, &market);

                // Emit winnings claimed event
                EventEmitter::emit_winnings_claimed(&env, &market_id, &user, payout);

                // Credit tokens to user balance
                match storage::BalanceStorage::add_balance(
                    &env,
                    &user,
                    &types::ReflectorAsset::Stellar,
                    payout,
                ) {
                    Ok(_) => {}
                    Err(e) => panic_with_error!(env, e),
                }

                return;
            }
        }

        // If no winnings (user didn't win or zero payout), still mark as claimed to prevent re-attempts
        market.claimed.set(user.clone(), true);
        env.storage().persistent().set(&market_id, &market);
    }

    /// Retrieves complete market information by market identifier.
    ///
    /// This function provides read-only access to all market data including
    /// configuration, current state, voting results, stakes, and resolution status.
    /// It's the primary way to query market information for display or analysis.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `market_id` - Unique identifier of the market to retrieve
    ///
    /// # Returns
    ///
    /// Returns `Some(Market)` if the market exists, `None` if not found.
    /// The `Market` struct contains:
    /// - Basic info: admin, question, outcomes, end_time
    /// - Oracle configuration and results
    /// - Voting data: votes, stakes, total_staked
    /// - Resolution data: winning_outcome, claimed status
    /// - State information: current state, extensions, fee collection
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Symbol};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let market_id = Symbol::new(&env, "market_1");
    ///
    /// match PredictifyHybrid::get_market(env.clone(), market_id) {
    ///     Some(market) => {
    ///         // Market found - access market data
    ///         let question = market.question;
    ///         let state = market.state;
    ///         let total_staked = market.total_staked;
    ///     },
    ///     None => {
    ///         // Market not found
    ///     }
    /// }
    /// ```
    ///
    /// # Use Cases
    ///
    /// - **UI Display**: Show market details, voting status, and results
    /// - **Analytics**: Calculate market statistics and user positions
    /// - **Validation**: Check market state before performing operations
    /// - **Monitoring**: Track market progress and resolution status
    ///
    /// # Performance
    ///
    /// This is a read-only operation that doesn't modify contract state.
    /// It retrieves data from persistent storage with minimal computational overhead.
    pub fn get_market(env: Env, market_id: Symbol) -> Option<Market> {
        env.storage().persistent().get(&market_id)
    }

    // ===== RESOLUTION DELAY / DISPUTE WINDOW FUNCTIONS =====

    /// Sets the dispute window duration for resolution delay.
    ///
    /// This function configures how long the dispute window remains open after a
    /// market resolution is proposed. During this window, community members can
    /// file disputes if they believe the proposed outcome is incorrect.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `admin` - The administrator address (must be authorized)
    /// * `hours` - Duration of the dispute window in hours (1-168)
    /// * `market_id` - Optional market ID for per-market override (None = global setting)
    ///
    /// # Panics
    ///
    /// This function will panic if:
    /// - `Error::Unauthorized` - Caller is not the contract admin
    /// - `Error::InvalidTimeoutHours` - Hours is 0 or exceeds 168 (1 week)
    /// - `Error::MarketNotFound` - Market ID specified but doesn't exist
    ///
    /// # Example
    ///
    /// ```rust
    /// // Set global dispute window to 72 hours
    /// PredictifyHybrid::set_dispute_window_duration(env, admin, 72, None);
    ///
    /// // Set per-market override to 24 hours
    /// PredictifyHybrid::set_dispute_window_duration(env, admin, 24, Some(market_id));
    /// ```
    ///
    /// # Security
    ///
    /// Only the contract admin can modify dispute window configuration.
    /// Changes take effect for future resolution proposals, not retroactively.
    pub fn set_dispute_window_duration(
        env: Env,
        admin: Address,
        hours: u32,
        market_id: Option<Symbol>,
    ) {
        admin.require_auth();

        match market_id {
            Some(mid) => {
                match resolution_delay::ResolutionDelayManager::set_market_dispute_window(
                    &env, &admin, &mid, hours,
                ) {
                    Ok(_) => (),
                    Err(e) => panic_with_error!(env, e),
                }
            }
            None => {
                match resolution_delay::ResolutionDelayManager::set_global_dispute_window(
                    &env, &admin, hours,
                ) {
                    Ok(_) => (),
                    Err(e) => panic_with_error!(env, e),
                }
            }
        }
    }

    /// Proposes a resolution for a market, opening the dispute window.
    ///
    /// This function is called after a market ends to propose the winning outcome.
    /// It opens a dispute window during which community members can challenge the
    /// proposed outcome. Payouts are blocked until the window closes and resolution
    /// is finalized.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `admin` - The administrator address proposing resolution (must be authorized)
    /// * `market_id` - The market to propose resolution for
    ///
    /// # Returns
    ///
    /// Returns the proposed outcome string.
    ///
    /// # Panics
    ///
    /// This function will panic if:
    /// - `Error::Unauthorized` - Caller is not the contract admin
    /// - `Error::MarketNotFound` - Market doesn't exist
    /// - `Error::MarketClosed` - Market hasn't ended yet
    /// - `Error::ResolutionAlreadyFinalized` - Resolution already finalized
    /// - `Error::OracleUnavailable` - Oracle result not available
    ///
    /// # Example
    ///
    /// ```rust
    /// // After market ends, propose resolution
    /// let outcome = PredictifyHybrid::propose_market_resolution(env, admin, market_id);
    /// // Dispute window is now open for configured duration
    /// ```
    ///
    /// # Events
    ///
    /// Emits `ResolutionProposedEvent` with window details.
    ///
    /// # Resolution Flow
    ///
    /// 1. Market ends (end_time reached)
    /// 2. Admin calls `propose_market_resolution` (this function)
    /// 3. Dispute window opens for configured duration
    /// 4. Community can file disputes during window
    /// 5. After window closes, call `finalize_market_resolution`
    /// 6. Payouts enabled after finalization
    pub fn propose_market_resolution(env: Env, admin: Address, market_id: Symbol) -> String {
        admin.require_auth();

        // Validate admin
        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, "Admin"))
            .unwrap_or_else(|| panic_with_error!(env, Error::AdminNotSet));

        if admin != stored_admin {
            panic_with_error!(env, Error::Unauthorized);
        }

        // Get the market
        let market: Market = env
            .storage()
            .persistent()
            .get(&market_id)
            .unwrap_or_else(|| panic_with_error!(env, Error::MarketNotFound));

        // Get the oracle result (must be set before proposing resolution)
        let outcome = match &market.oracle_result {
            Some(result) => result.clone(),
            None => panic_with_error!(env, Error::OracleUnavailable),
        };

        // Propose resolution using the delay manager
        let resolution_source = String::from_str(&env, "Oracle");
        match resolution_delay::ResolutionDelayManager::propose_resolution(
            &env,
            &market_id,
            outcome.clone(),
            resolution_source,
        ) {
            Ok(_) => outcome,
            Err(e) => panic_with_error!(env, e),
        }
    }

    /// Finalizes a market resolution after the dispute window closes.
    ///
    /// This function completes the resolution process, making the outcome permanent
    /// and enabling payouts. It can only be called after the dispute window has
    /// closed and all disputes are resolved.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `market_id` - The market to finalize
    ///
    /// # Returns
    ///
    /// Returns the final winning outcome string.
    ///
    /// # Panics
    ///
    /// This function will panic if:
    /// - `Error::MarketNotFound` - Market doesn't exist
    /// - `Error::ResolutionNotProposed` - Resolution hasn't been proposed
    /// - `Error::DisputeWindowStillOpen` - Window hasn't closed yet
    /// - `Error::UnresolvedDisputes` - There are unresolved disputes
    /// - `Error::ResolutionAlreadyFinalized` - Already finalized
    ///
    /// # Example
    ///
    /// ```rust
    /// // After dispute window closes
    /// let final_outcome = PredictifyHybrid::finalize_market_resolution(env, market_id);
    /// // Users can now call claim_winnings
    /// ```
    ///
    /// # Events
    ///
    /// Emits `DisputeWindowClosedEvent` and `ResolutionFinalizedEvent`.
    ///
    /// # Security
    ///
    /// This function can be called by anyone (permissionless) since it only
    /// succeeds when legitimate conditions are met (window closed, no disputes).
    pub fn finalize_market_resolution(env: Env, market_id: Symbol) -> String {
        match resolution_delay::ResolutionDelayManager::finalize_resolution(&env, &market_id) {
            Ok(outcome) => outcome,
            Err(e) => panic_with_error!(env, e),
        }
    }

    /// Gets the current status of a market's dispute window.
    ///
    /// This function returns information about the dispute window for a market,
    /// including whether it's open, time remaining, and dispute count.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `market_id` - The market to query
    ///
    /// # Returns
    ///
    /// Returns a tuple of `(is_open, remaining_seconds, dispute_count)`:
    /// - `is_open`: Whether the dispute window is currently open
    /// - `remaining_seconds`: Seconds until window closes (0 if closed)
    /// - `dispute_count`: Number of disputes filed during the window
    ///
    /// # Example
    ///
    /// ```rust
    /// let (is_open, remaining, disputes) = PredictifyHybrid::get_dispute_window_status(env, market_id);
    /// if is_open {
    ///     println!("Window open for {} more seconds", remaining);
    ///     println!("{} disputes filed", disputes);
    /// } else {
    ///     println!("Window closed, ready for finalization");
    /// }
    /// ```
    pub fn get_dispute_window_status(env: Env, market_id: Symbol) -> (bool, u64, u32) {
        resolution_delay::ResolutionDelayManager::get_dispute_window_status(&env, &market_id)
    }

    /// Files a dispute during the dispute window.
    ///
    /// This function allows users to challenge a proposed resolution during the
    /// dispute window period. Disputes require a stake and block finalization
    /// until resolved.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `user` - The user filing the dispute (must be authorized)
    /// * `market_id` - The market to dispute
    /// * `stake` - Stake amount backing the dispute
    /// * `reason` - Reason for the dispute
    ///
    /// # Panics
    ///
    /// This function will panic if:
    /// - `Error::MarketNotFound` - Market doesn't exist
    /// - `Error::ResolutionNotProposed` - No resolution to dispute
    /// - `Error::DisputeWindowNotOpen` - Window is closed
    /// - `Error::ResolutionAlreadyFinalized` - Resolution already finalized
    /// - `Error::InsufficientStake` - Stake below minimum
    ///
    /// # Example
    ///
    /// ```rust
    /// PredictifyHybrid::dispute_during_window(
    ///     env,
    ///     user,
    ///     market_id,
    ///     10_000_000, // 1 XLM stake
    ///     String::from_str(&env, "Oracle data appears incorrect")
    /// );
    /// ```
    ///
    /// # Events
    ///
    /// Emits `DisputeCreatedEvent`.
    pub fn dispute_during_window(
        env: Env,
        user: Address,
        market_id: Symbol,
        stake: i128,
        reason: String,
    ) {
        user.require_auth();

        // Validate dispute is allowed during window
        match resolution_delay::ResolutionDelayManager::validate_dispute_allowed(&env, &market_id) {
            Ok(_) => (),
            Err(e) => panic_with_error!(env, e),
        }

        // Check minimum stake
        let config = resolution_delay::ResolutionDelayManager::get_dispute_window_config(&env, &market_id);
        if stake < config.min_dispute_stake {
            panic_with_error!(env, Error::InsufficientStake);
        }

        // Record the dispute in the window
        match resolution_delay::ResolutionDelayManager::record_dispute(&env, &market_id) {
            Ok(_) => (),
            Err(e) => panic_with_error!(env, e),
        }

        // Also create the actual dispute using existing dispute system
        // The disputes module will handle the dispute creation and voting
        EventEmitter::emit_dispute_created(&env, &market_id, &user, stake, Some(reason));
    }

    /// Force finalizes a resolution (admin emergency override).
    ///
    /// This function allows an admin to bypass normal finalization requirements
    /// and immediately finalize a resolution. Should only be used in emergency
    /// situations where normal dispute resolution is blocked or malfunctioning.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `admin` - The administrator address (must be authorized)
    /// * `market_id` - The market to force finalize
    /// * `outcome` - The final outcome to set
    ///
    /// # Panics
    ///
    /// This function will panic if:
    /// - `Error::Unauthorized` - Caller is not the contract admin
    /// - `Error::MarketNotFound` - Market doesn't exist
    ///
    /// # Security
    ///
    /// This is an admin-only emergency function that bypasses the dispute window.
    /// Use with extreme caution as it overrides community governance.
    pub fn force_finalize_resolution(
        env: Env,
        admin: Address,
        market_id: Symbol,
        outcome: String,
    ) {
        admin.require_auth();

        match resolution_delay::ResolutionDelayManager::force_finalize(
            &env, &admin, &market_id, outcome,
        ) {
            Ok(_) => (),
            Err(e) => panic_with_error!(env, e),
        }
    }

    /// Manually resolves a prediction market by setting the winning outcome (admin only).
    ///
    /// This function allows contract administrators to manually resolve markets
    /// when automatic oracle resolution is not available or needs override.
    /// It's typically used for markets with subjective outcomes or when oracle
    /// data is unavailable or disputed.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `admin` - The administrator address performing the resolution (must be authorized)
    /// * `market_id` - Unique identifier of the market to resolve
    /// * `winning_outcome` - The outcome to be declared as the winner
    ///
    /// # Panics
    ///
    /// This function will panic with specific errors if:
    /// - `Error::Unauthorized` - Caller is not the contract admin
    /// - `Error::MarketNotFound` - Market with given ID doesn't exist
    /// - `Error::MarketClosed` - Market hasn't reached its end time yet
    /// - `Error::InvalidOutcome` - Winning outcome doesn't match any market outcomes
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address, String, Symbol};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let admin = Address::generate(&env);
    /// # let market_id = Symbol::new(&env, "market_1");
    ///
    /// // Manually resolve market with "Yes" as winning outcome
    /// PredictifyHybrid::resolve_market_manual(
    ///     env.clone(),
    ///     admin,
    ///     market_id,
    ///     String::from_str(&env, "Yes")
    /// );
    /// ```
    ///
    /// # Resolution Process
    ///
    /// 1. **Authentication**: Verifies caller is the contract admin
    /// 2. **Market Validation**: Ensures market exists and has ended
    /// 3. **Outcome Validation**: Confirms winning outcome is valid
    /// 4. **State Update**: Sets winning outcome and updates market state
    ///
    /// # Use Cases
    ///
    /// - **Subjective Markets**: Markets requiring human judgment
    /// - **Oracle Failures**: When automated oracles are unavailable
    /// - **Dispute Resolution**: Override disputed automatic resolutions
    /// - **Emergency Resolution**: Resolve markets in exceptional circumstances
    ///
    /// # Security
    ///
    /// This function requires admin privileges and should be used carefully.
    /// Manual resolutions should be transparent and follow established governance procedures.
    pub fn resolve_market_manual(
        env: Env,
        admin: Address,
        market_id: Symbol,
        winning_outcome: String,
    ) {
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

        // Capture old state for event
        let old_state = market.state.clone();

        // Set winning outcome(s) as a vector (single outcome for now, supports future multi-winner)
        let mut winning_outcomes_vec = Vec::new(&env);
        winning_outcomes_vec.push_back(winning_outcome.clone());
        market.winning_outcomes = Some(winning_outcomes_vec.clone());
        market.state = MarketState::Resolved;
        env.storage().persistent().set(&market_id, &market);

        // Resolve bets to mark them as won/lost
        let _ = bets::BetManager::resolve_market_bets(&env, &market_id, &winning_outcomes_vec);

        // Emit market resolved event (simplified to avoid segfaults)
        let oracle_result_str = market
            .oracle_result
            .clone()
            .unwrap_or_else(|| String::from_str(&env, "N/A"));
        let community_consensus_str = String::from_str(&env, "Manual");
        let resolution_method = String::from_str(&env, "Manual");

        // Emit events with defensive approach
        EventEmitter::emit_market_resolved(
            &env,
            &market_id,
            &winning_outcome,
            &oracle_result_str,
            &community_consensus_str,
            &resolution_method,
            100, // confidence score for manual resolution
        );

        // Emit state change event
        let reason = String::from_str(&env, "Manual resolution by admin");
        EventEmitter::emit_state_change_event(
            &env,
            &market_id,
            &old_state,
            &MarketState::Resolved,
            &reason,
        );

        // Automatically distribute payouts to winners after resolution
        let _ = Self::distribute_payouts(env.clone(), market_id);
    }

    /// Resolves a market with multiple winning outcomes (for tie cases).
    ///
    /// This function allows authorized administrators to resolve a market with
    /// multiple winners when there's a tie. The pool will be split proportionally
    /// among all winning outcomes based on stake distribution.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `admin` - The administrator address performing the resolution (must be authorized)
    /// * `market_id` - Unique identifier of the market to resolve
    /// * `winning_outcomes` - Vector of outcomes to be declared as winners (minimum 1, all must be valid)
    ///
    /// # Panics
    ///
    /// This function will panic with specific errors if:
    /// - `Error::Unauthorized` - Caller is not the contract admin
    /// - `Error::MarketNotFound` - Market with given ID doesn't exist
    /// - `Error::MarketClosed` - Market hasn't ended yet
    /// - `Error::InvalidOutcome` - One or more outcomes are not valid for this market
    /// - `Error::InvalidInput` - Empty outcomes vector
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address, Symbol, String, Vec};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let admin = Address::generate(&env);
    /// # let market_id = Symbol::new(&env, "sports_match");
    ///
    /// // Resolve with tie (Team A and Team B both win)
    /// let winning_outcomes = vec![
    ///     &env,
    ///     String::from_str(&env, "Team A"),
    ///     String::from_str(&env, "Team B"),
    /// ];
    ///
    /// PredictifyHybrid::resolve_market_with_ties(
    ///     env.clone(),
    ///     admin,
    ///     market_id,
    ///     winning_outcomes
    /// );
    /// ```
    ///
    /// # Pool Split Logic
    ///
    /// When multiple outcomes win:
    /// - Total pool is split proportionally among all winners
    /// - Each winner receives: (their_stake / total_winning_stakes) * total_pool * (1 - fee)
    /// - This ensures fair distribution even when outcomes are tied
    pub fn resolve_market_with_ties(
        env: Env,
        admin: Address,
        market_id: Symbol,
        winning_outcomes: Vec<String>,
    ) {
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

        // Validate outcomes vector is not empty
        if winning_outcomes.len() == 0 {
            panic_with_error!(env, Error::InvalidInput);
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

        // Validate all winning outcomes exist in market outcomes
        for outcome in winning_outcomes.iter() {
            let outcome_exists = market.outcomes.iter().any(|o| o == outcome);
            if !outcome_exists {
                panic_with_error!(env, Error::InvalidOutcome);
            }
        }

        // Capture old state for event
        let old_state = market.state.clone();

        // Set winning outcome(s) - supports multiple winners for ties
        market.winning_outcomes = Some(winning_outcomes.clone());
        market.state = MarketState::Resolved;
        env.storage().persistent().set(&market_id, &market);

        // Resolve bets to mark them as won/lost
        let _ = bets::BetManager::resolve_market_bets(&env, &market_id, &winning_outcomes);

        // Emit market resolved event
        let primary_outcome = winning_outcomes.get(0).unwrap().clone();
        let oracle_result_str = market
            .oracle_result
            .clone()
            .unwrap_or_else(|| String::from_str(&env, "N/A"));
        let community_consensus_str = String::from_str(&env, "Manual");
        let resolution_method = String::from_str(&env, "Manual");

        EventEmitter::emit_market_resolved(
            &env,
            &market_id,
            &primary_outcome,
            &oracle_result_str,
            &community_consensus_str,
            &resolution_method,
            100, // confidence score for manual resolution
        );

        // Emit state change event
        let reason = String::from_str(&env, "Manual resolution with ties by admin");
        EventEmitter::emit_state_change_event(
            &env,
            &market_id,
            &old_state,
            &MarketState::Resolved,
            &reason,
        );

        // Automatically distribute payouts (handles split pool for ties)
        let _ = Self::distribute_payouts(env.clone(), market_id);
    }

    /// Fetches oracle result for a market from external oracle contracts.
    ///
    /// This function retrieves prediction results from configured oracle sources
    /// such as Reflector or Pyth networks. It's used to obtain objective data
    /// for market resolution when manual resolution is not appropriate.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `market_id` - Unique identifier of the market to fetch oracle data for
    /// * `oracle_contract` - Address of the oracle contract to query
    ///
    /// # Returns
    ///
    /// Returns `Result<String, Error>` where:
    /// - `Ok(String)` - The oracle result as a string representation
    /// - `Err(Error)` - Specific error if operation fails
    ///
    /// # Errors
    ///
    /// This function returns specific errors:
    /// - `Error::MarketNotFound` - Market with given ID doesn't exist
    /// - `Error::MarketResolved` - Market already has oracle result set
    /// - `Error::MarketClosed` - Market hasn't reached its end time yet
    /// - Oracle-specific errors from the resolution module
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address, Symbol};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let market_id = Symbol::new(&env, "btc_market");
    /// # let oracle_address = Address::generate(&env);
    ///
    /// match PredictifyHybrid::fetch_oracle_result(
    ///     env.clone(),
    ///     market_id,
    ///     oracle_address
    /// ) {
    ///     Ok(result) => {
    ///         // Oracle result retrieved successfully
    ///         println!("Oracle result: {}", result);
    ///     },
    ///     Err(e) => {
    ///         // Handle error
    ///         println!("Failed to fetch oracle result: {:?}", e);
    ///     }
    /// }
    /// ```
    ///
    /// # Oracle Integration
    ///
    /// This function integrates with various oracle types:
    /// - **Reflector**: For asset price data and market conditions
    /// - **Pyth**: For high-frequency financial data feeds
    /// - **Custom Oracles**: For specialized data sources
    ///
    /// # Market State Requirements
    ///
    /// - Market must exist and be past its end time
    /// - Market must not already have an oracle result
    /// - Oracle contract must be accessible and responsive
    pub fn fetch_oracle_result(
        env: Env,
        market_id: Symbol,
        oracle_contract: Address,
    ) -> Result<String, Error> {
        // Get the market from storage
        let market = env
            .storage()
            .persistent()
            .get::<Symbol, Market>(&market_id)
            .ok_or(Error::MarketNotFound)?;

        // Validate market state
        if market.oracle_result.is_some() {
            return Err(Error::MarketResolved);
        }

        // Check if market has ended
        let current_time = env.ledger().timestamp();
        if current_time < market.end_time {
            return Err(Error::MarketClosed);
        }

        // Get oracle result using the resolution module
        let oracle_resolution = resolution::OracleResolutionManager::fetch_oracle_result(
            &env,
            &market_id,
            &oracle_contract,
        )?;

        Ok(oracle_resolution.oracle_result)
    pub fn fetch_oracle_result(env: Env, market_id: Symbol) -> Result<OracleResolution, Error> {
        resolution::OracleResolutionManager::fetch_oracle_result(&env, &market_id)
    }

    /// Verifies and fetches event outcome from external oracle sources automatically.
    ///
    /// This function implements the complete oracle integration mechanism that:
    /// - Automatically fetches event outcomes from configured external data sources
    /// - Validates oracle responses and signatures/authority
    /// - Supports multiple oracle sources with consensus-based verification
    /// - Handles oracle failures gracefully with fallback mechanisms
    /// - Emits result verification events for transparency
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `caller` - The address initiating the verification (must be authenticated)
    /// * `market_id` - Unique identifier of the market to verify
    ///
    /// # Returns
    ///
    /// Returns `Result<OracleResult, Error>` where:
    /// - `Ok(OracleResult)` - Complete oracle verification result including:
    ///   - `outcome`: The determined outcome ("yes"/"no" or custom)
    ///   - `price`: The fetched price from oracle
    ///   - `threshold`: The configured threshold for comparison
    ///   - `confidence_score`: Statistical confidence (0-100)
    ///   - `is_verified`: Whether the result passed all validations
    ///   - `sources_count`: Number of oracle sources consulted
    /// - `Err(Error)` - Specific error if verification fails
    ///
    /// # Errors
    ///
    /// This function returns specific errors:
    /// - `Error::MarketNotFound` - Market with given ID doesn't exist
    /// - `Error::MarketNotReadyForVerification` - Market hasn't ended yet
    /// - `Error::OracleVerified` - Result already verified for this market
    /// - `Error::OracleUnavailable` - Oracle service is unavailable
    /// - `Error::OracleStale` - Oracle data is too old
    /// - `Error::OracleConsensusNotReached` - Multiple oracles disagree
    /// - `Error::InvalidOracleConfig` - Oracle not whitelisted/authorized
    /// - `Error::OracleAllSourcesFailed` - All oracle sources failed
    /// - `Error::InsufficientOracleSources` - No active oracle sources available
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address, Symbol};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let caller = Address::generate(&env);
    /// # let market_id = Symbol::new(&env, "btc_50k_2024");
    ///
    /// // Verify result for an ended market
    /// match PredictifyHybrid::verify_result(env.clone(), caller, market_id) {
    ///     Ok(result) => {
    ///         println!("Outcome: {}", result.outcome);
    ///         println!("Price: ${}", result.price / 100);
    ///         println!("Confidence: {}%", result.confidence_score);
    ///         println!("Sources consulted: {}", result.sources_count);
    ///         
    ///         if result.is_verified {
    ///             println!("Result is verified and authoritative");
    ///         }
    ///     },
    ///     Err(e) => {
    ///         println!("Verification failed: {:?}", e);
    ///     }
    /// }
    /// ```
    ///
    /// # Oracle Integration
    ///
    /// This function integrates with multiple oracle providers:
    /// - **Reflector**: Primary oracle for Stellar Network (production ready)
    /// - **Band Protocol**: Decentralized oracle network
    /// - **Custom Oracles**: Can be added via whitelist system
    ///
    /// # Multi-Oracle Consensus
    ///
    /// When multiple oracle sources are configured:
    /// 1. All active sources are queried in parallel
    /// 2. Responses are validated for freshness and authority
    /// 3. Consensus is calculated (default: 66% agreement required)
    /// 4. Confidence score reflects agreement level and price stability
    ///
    /// # Security Features
    ///
    /// - **Whitelist Validation**: Only whitelisted oracles are queried
    /// - **Authority Verification**: Oracle responses are validated for authenticity
    /// - **Staleness Protection**: Data older than 5 minutes is rejected
    /// - **Price Range Validation**: Ensures prices are within reasonable bounds
    /// - **Consensus Requirement**: Multiple sources must agree for high-value markets
    ///
    /// # Events Emitted
    ///
    /// - `OracleVerificationInitiated`: When verification begins
    /// - `OracleResultVerified`: When verification succeeds
    /// - `OracleVerificationFailed`: When verification fails
    /// - `OracleConsensusReached`: When multiple sources agree
    ///
    /// # Market State Requirements
    ///
    /// - Market must exist in storage
    /// - Market end time must have passed
    /// - Result must not already be verified
    /// - At least one active oracle source must be available
    pub fn verify_result(
        env: Env,
        caller: Address,
        market_id: Symbol,
    ) -> Result<OracleResult, Error> {
        // Authenticate the caller
        caller.require_auth();

        // Use the OracleIntegrationManager to perform verification
        oracles::OracleIntegrationManager::verify_result(&env, &market_id, &caller)
    }

    /// Verifies oracle result with retry logic for resilience.
    ///
    /// This function is similar to `verify_result` but includes automatic
    /// retry logic to handle transient oracle failures. Useful in production
    /// environments where network issues may cause temporary unavailability.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `caller` - The address initiating the verification
    /// * `market_id` - Unique identifier of the market to verify
    /// * `max_retries` - Maximum number of retry attempts (capped at 3)
    ///
    /// # Returns
    ///
    /// Returns `Result<OracleResult, Error>` - Same as `verify_result`
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address, Symbol};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let caller = Address::generate(&env);
    /// # let market_id = Symbol::new(&env, "btc_50k_2024");
    ///
    /// // Verify with up to 3 retries
    /// let result = PredictifyHybrid::verify_result_with_retry(
    ///     env.clone(),
    ///     caller,
    ///     market_id,
    ///     3
    /// );
    /// ```
    pub fn verify_result_with_retry(
        env: Env,
        caller: Address,
        market_id: Symbol,
        max_retries: u32,
    ) -> Result<OracleResult, Error> {
        caller.require_auth();
        oracles::OracleIntegrationManager::verify_result_with_retry(
            &env,
            &market_id,
            &caller,
            max_retries,
        )
    }

    /// Retrieves a previously verified oracle result for a market.
    ///
    /// This function returns the stored oracle verification result for a market
    /// that has already been verified. Useful for checking verification status
    /// and retrieving historical verification data.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `market_id` - Unique identifier of the market
    ///
    /// # Returns
    ///
    /// Returns `Option<OracleResult>`:
    /// - `Some(OracleResult)` - The stored verification result
    /// - `None` - Market has not been verified yet
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Symbol};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let market_id = Symbol::new(&env, "btc_50k_2024");
    ///
    /// match PredictifyHybrid::get_verified_result(env.clone(), market_id) {
    ///     Some(result) => {
    ///         println!("Market verified with outcome: {}", result.outcome);
    ///     },
    ///     None => {
    ///         println!("Market not yet verified");
    ///     }
    /// }
    /// ```
    pub fn get_verified_result(env: Env, market_id: Symbol) -> Option<OracleResult> {
        oracles::OracleIntegrationManager::get_oracle_result(&env, &market_id)
    }

    /// Checks if a market's result has been verified via oracle.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment
    /// * `market_id` - Unique identifier of the market
    ///
    /// # Returns
    ///
    /// Returns `bool` - `true` if verified, `false` otherwise
    pub fn is_result_verified(env: Env, market_id: Symbol) -> bool {
        oracles::OracleIntegrationManager::is_result_verified(&env, &market_id)
    }

    /// Admin override for oracle result verification.
    ///
    /// Allows an authorized admin to manually set the verification result
    /// when automatic verification fails or produces incorrect results.
    /// This is a privileged operation requiring admin authorization.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment
    /// * `admin` - Admin address (must be authorized)
    /// * `market_id` - Market to override
    /// * `outcome` - The outcome to set ("yes"/"no" or custom)
    /// * `reason` - Reason for the manual override
    ///
    /// # Returns
    ///
    /// Returns `Result<(), Error>`:
    /// - `Ok(())` - Override successful
    /// - `Err(Error::Unauthorized)` - Caller is not admin
    ///
    /// # Security
    ///
    /// This function should be used sparingly and only when:
    /// - Automatic oracle verification has failed repeatedly
    /// - Oracle data is known to be incorrect
    /// - Emergency situations requiring immediate resolution
    pub fn admin_override_verification(
        env: Env,
        admin: Address,
        market_id: Symbol,
        outcome: String,
        reason: String,
    ) -> Result<(), Error> {
        admin.require_auth();
        oracles::OracleIntegrationManager::admin_override_result(
            &env,
            &admin,
            &market_id,
            &outcome,
            &reason,
        )
    }

    /// Resolves a market automatically using oracle data and community consensus.
    ///
    /// This function implements the hybrid resolution algorithm that combines
    /// objective oracle data with community voting patterns to determine the
    /// final market outcome. It's the primary automated resolution mechanism.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `market_id` - Unique identifier of the market to resolve
    ///
    /// # Returns
    ///
    /// Returns `Result<(), Error>` where:
    /// - `Ok(())` - Market resolved successfully
    /// - `Err(Error)` - Specific error if resolution fails
    ///
    /// # Errors
    ///
    /// This function returns specific errors:
    /// - `Error::MarketNotFound` - Market with given ID doesn't exist
    /// - `Error::MarketNotEnded` - Market hasn't reached its end time
    /// - `Error::MarketResolved` - Market is already resolved
    /// - `Error::InsufficientData` - Not enough data for resolution
    /// - Resolution-specific errors from the resolution module
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Symbol};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let market_id = Symbol::new(&env, "ended_market");
    ///
    /// match PredictifyHybrid::resolve_market(env.clone(), market_id) {
    ///     Ok(()) => {
    ///         // Market resolved successfully
    ///         println!("Market resolved successfully");
    ///     },
    ///     Err(e) => {
    ///         // Handle resolution error
    ///         println!("Resolution failed: {:?}", e);
    ///     }
    /// }
    /// ```
    ///
    /// # Hybrid Resolution Algorithm
    ///
    /// The resolution process follows these steps:
    /// 1. **Data Collection**: Gather oracle data and community votes
    /// 2. **Consensus Analysis**: Analyze agreement between oracle and community
    /// 3. **Conflict Resolution**: Handle disagreements using weighted algorithms
    /// 4. **Final Determination**: Set winning outcome based on hybrid result
    /// 5. **State Update**: Update market state to resolved
    ///
    /// # Resolution Criteria
    ///
    /// - Market must be past its end time
    /// - Sufficient voting participation required
    /// - Oracle data must be available (if configured)
    /// - No active disputes that would prevent resolution
    ///
    /// # Post-Resolution
    ///
    /// After successful resolution:
    /// - Market state changes to `Resolved`
    /// - Winning outcome is set
    /// - Users can claim winnings
    /// - Market statistics are finalized
    pub fn resolve_market(env: Env, market_id: Symbol) -> Result<(), Error> {
        // Use the resolution module to resolve the market
        let _resolution = resolution::MarketResolutionManager::resolve_market(&env, &market_id)?;

        statistics::StatisticsManager::record_market_resolved(&env);

        Ok(())
    }

    /// Retrieves comprehensive analytics about market resolution performance.
    ///
    /// This function provides detailed statistics about how markets are being
    /// resolved across the platform, including success rates, resolution methods,
    /// oracle performance, and community consensus patterns.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    ///
    /// # Returns
    ///
    /// Returns `Result<ResolutionAnalytics, Error>` where:
    /// - `Ok(ResolutionAnalytics)` - Complete resolution analytics data
    /// - `Err(Error)` - Error if analytics calculation fails
    ///
    /// The `ResolutionAnalytics` struct contains:
    /// - Total markets resolved
    /// - Resolution method breakdown (manual vs automatic)
    /// - Oracle accuracy statistics
    /// - Community consensus metrics
    /// - Average resolution time
    /// - Dispute frequency and outcomes
    ///
    /// # Errors
    ///
    /// This function may return:
    /// - `Error::InsufficientData` - Not enough resolved markets for analytics
    /// - Storage access errors
    /// - Calculation errors from the analytics module
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::Env;
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    ///
    /// match PredictifyHybrid::get_resolution_analytics(env.clone()) {
    ///     Ok(analytics) => {
    ///         // Access resolution statistics
    ///         let total_resolved = analytics.total_markets_resolved;
    ///         let oracle_accuracy = analytics.oracle_accuracy_rate;
    ///         let avg_resolution_time = analytics.average_resolution_time;
    ///         
    ///         println!("Resolved markets: {}", total_resolved);
    ///         println!("Oracle accuracy: {}%", oracle_accuracy);
    ///     },
    ///     Err(e) => {
    ///         println!("Analytics unavailable: {:?}", e);
    ///     }
    /// }
    /// ```
    ///
    /// # Use Cases
    ///
    /// - **Platform Monitoring**: Track overall resolution system health
    /// - **Oracle Evaluation**: Assess oracle performance and reliability
    /// - **Community Analysis**: Understand voting patterns and accuracy
    /// - **System Optimization**: Identify areas for improvement
    /// - **Governance Reporting**: Provide transparency to stakeholders
    ///
    /// # Analytics Metrics
    ///
    /// Key metrics included:
    /// - **Resolution Rate**: Percentage of markets successfully resolved
    /// - **Method Distribution**: Manual vs automatic resolution breakdown
    /// - **Accuracy Scores**: Oracle vs community prediction accuracy
    /// - **Time Metrics**: Average time from market end to resolution
    /// - **Dispute Analytics**: Frequency and resolution of disputes
    ///
    /// # Performance
    ///
    /// This function performs read-only analytics calculations and may take
    /// longer for platforms with many resolved markets. Results may be cached
    /// for performance optimization.
    pub fn get_resolution_analytics(env: Env) -> Result<resolution::ResolutionAnalytics, Error> {
        resolution::MarketResolutionAnalytics::calculate_resolution_analytics(&env)
    }

    /// Retrieves comprehensive analytics and statistics for a specific market.
    ///
    /// This function provides detailed statistical analysis of a market including
    /// participation metrics, voting patterns, stake distribution, and performance
    /// indicators. It's essential for market analysis and user interfaces.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `market_id` - Unique identifier of the market to analyze
    ///
    /// # Returns
    ///
    /// Returns `Result<MarketStats, Error>` where:
    /// - `Ok(MarketStats)` - Complete market statistics and analytics
    /// - `Err(Error)` - Error if market not found or analysis fails
    ///
    /// The `MarketStats` struct contains:
    /// - Participation metrics (total voters, total stake)
    /// - Outcome distribution (stakes per outcome)
    /// - Market activity timeline
    /// - Consensus and confidence indicators
    /// - Resolution status and results
    ///
    /// # Errors
    ///
    /// This function returns:
    /// - `Error::MarketNotFound` - Market with given ID doesn't exist
    /// - Calculation errors from the analytics module
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Symbol};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let market_id = Symbol::new(&env, "market_1");
    ///
    /// match PredictifyHybrid::get_market_analytics(env.clone(), market_id) {
    ///     Ok(stats) => {
    ///         // Access market statistics
    ///         let total_participants = stats.total_participants;
    ///         let total_stake = stats.total_stake;
    ///         let leading_outcome = stats.leading_outcome;
    ///         
    ///         println!("Participants: {}", total_participants);
    ///         println!("Total stake: {}", total_stake);
    ///         println!("Leading outcome: {:?}", leading_outcome);
    ///     },
    ///     Err(e) => {
    ///         println!("Analytics unavailable: {:?}", e);
    ///     }
    /// }
    /// ```
    ///
    /// # Statistical Metrics
    ///
    /// Key analytics provided:
    /// - **Participation**: Number of unique voters and total stake
    /// - **Distribution**: Stake distribution across outcomes
    /// - **Confidence**: Market confidence indicators and consensus strength
    /// - **Activity**: Voting timeline and participation patterns
    /// - **Performance**: Market liquidity and engagement metrics
    ///
    /// # Use Cases
    ///
    /// - **UI Display**: Show market statistics to users
    /// - **Market Analysis**: Understand market dynamics and trends
    /// - **Risk Assessment**: Evaluate market confidence and volatility
    /// - **Performance Tracking**: Monitor market engagement over time
    /// - **Research**: Academic and commercial market research
    ///
    /// # Real-time Updates
    ///
    /// Statistics are calculated in real-time based on current market state.
    /// For active markets, analytics reflect the most current voting and staking data.
    /// For resolved markets, analytics include final resolution information.
    ///
    /// # Performance
    ///
    /// This function performs calculations on market data and may have
    /// computational overhead for markets with many participants. Consider
    /// caching results for frequently accessed markets.
    pub fn get_market_analytics(
        env: Env,
        market_id: Symbol,
    ) -> Result<markets::MarketStats, Error> {
        let market = env
            .storage()
            .persistent()
            .get::<Symbol, Market>(&market_id)
            .ok_or(Error::MarketNotFound)?;

        // Calculate market statistics
        let stats = markets::MarketAnalytics::get_market_stats(&market);

        Ok(stats)
    }

    /// Dispute a market resolution
    pub fn dispute_market(
        env: Env,
        user: Address,
        market_id: Symbol,
        stake: i128,
        reason: Option<String>,
    ) -> Result<(), Error> {
        user.require_auth();
        disputes::DisputeManager::process_dispute(&env, user, market_id, stake, reason)
    }

    /// Vote on a dispute
    pub fn vote_on_dispute(
        env: Env,
        user: Address,
        market_id: Symbol,
        dispute_id: Symbol,
        vote: bool,
        stake: i128,
        reason: Option<String>,
    ) -> Result<(), Error> {
        user.require_auth();
        disputes::DisputeManager::vote_on_dispute(
            &env, user, market_id, dispute_id, vote, stake, reason,
        )
    }

    /// Resolve a dispute (admin only)
    pub fn resolve_dispute(
        env: Env,
        admin: Address,
        market_id: Symbol,
    ) -> Result<disputes::DisputeResolution, Error> {
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

        disputes::DisputeManager::resolve_dispute(&env, market_id, admin)
    }

    /// Collect fees from a market (admin only)
    pub fn collect_fees(env: Env, admin: Address, market_id: Symbol) -> Result<i128, Error> {
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

        fees::FeeManager::collect_fees(&env, admin, market_id)
    }

    /// Automatically distribute payouts to all winners after market resolution.
    ///
    /// This function automatically calculates and distributes winnings to all users
    /// who bet on the winning outcome, eliminating the need for manual claiming.
    /// It handles edge cases like no winners, all winners, and prevents double payouts.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `market_id` - Unique identifier of the resolved market
    ///
    /// # Returns
    ///
    /// Returns `Result<i128, Error>` where:
    /// - `Ok(total_distributed)` - Total amount distributed to winners
    /// - `Err(Error)` - Error if distribution fails
    ///
    /// # Panics
    ///
    /// This function will panic with specific errors if:
    /// - `Error::MarketNotFound` - Market with given ID doesn't exist
    /// - `Error::MarketNotResolved` - Market hasn't been resolved yet
    /// - `Error::MarketResolved` - Payouts have already been distributed
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Symbol};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let market_id = Symbol::new(&env, "resolved_market");
    ///
    /// match PredictifyHybrid::distribute_payouts(env.clone(), market_id) {
    ///     Ok(total) => println!("Distributed {} stroops to winners", total),
    ///     Err(e) => println!("Distribution failed: {:?}", e),
    /// }
    /// ```
    ///
    /// # Payout Calculation
    ///
    /// Payouts are calculated using the formula:
    /// ```text
    /// user_payout = (user_stake * (100 - fee_percentage) / 100) * total_pool / winning_total
    /// ```
    ///
    /// # Edge Cases
    ///
    /// - **No Winners**: If no users bet on the winning outcome, no payouts are made
    /// - **All Winners**: If all users bet on the winning outcome, they receive proportional shares
    /// - **Double Payout Prevention**: Users who already claimed are skipped
    ///
    /// # Events
    ///
    /// This function emits `WinningsClaimedEvent` for each user who receives a payout.
    pub fn distribute_payouts(env: Env, market_id: Symbol) -> Result<i128, Error> {
        if ReentrancyGuard::check_reentrancy_state(&env).is_err() {
            return Err(Error::InvalidState);
        }
        let mut market: Market = env
            .storage()
            .persistent()
            .get(&market_id)
            .unwrap_or_else(|| {
                panic_with_error!(env, Error::MarketNotFound);
            });

        // Check if market is resolved
        let winning_outcomes = match &market.winning_outcomes {
            Some(outcomes) => outcomes,
            None => return Err(Error::MarketNotResolved),
        };

        // Get all bettors
        let bettors = bets::BetStorage::get_all_bets_for_market(&env, &market_id);

        // Get fee from legacy storage (backward compatible)
        let fee_percent = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, "platform_fee"))
            .unwrap_or(200); // Default 2% if not set

        // Since place_bet now updates market.votes and market.stakes,
        // we can use the vote-based payout system for both bets and votes
        let _total_distributed = 0;

        // Check if payouts have already been distributed
        let mut has_unclaimed_winners = false;

        // Check voters
        for (user, outcome) in market.votes.iter() {
            if winning_outcomes.contains(&outcome) {
                if !market.claimed.get(user.clone()).unwrap_or(false) {
                    has_unclaimed_winners = true;
                    break;
                }
            }
        }

        // Check bettors
        if !has_unclaimed_winners {
            for user in bettors.iter() {
                if let Some(bet) = bets::BetStorage::get_bet(&env, &market_id, &user) {
                    if winning_outcomes.contains(&bet.outcome)
                        && !market.claimed.get(user.clone()).unwrap_or(false)
                    {
                        has_unclaimed_winners = true;
                        break;
                    }
                }
            }
        }

        if !has_unclaimed_winners {
            return Ok(0);
        }

        // Calculate total winning stakes across all winning outcomes (for split pool calculation)
        // Supports both single winner and multi-winner (tie) scenarios
        let mut winning_total = 0;

        // Sum voter stakes
        for (voter, outcome) in market.votes.iter() {
            if winning_outcomes.contains(&outcome) {
                winning_total += market.stakes.get(voter.clone()).unwrap_or(0);
            }
        }

        // Sum bet amounts (check if bet outcome is in winning outcomes for multi-outcome support)
        for user in bettors.iter() {
            // Avoid double counting if user is already in votes (legacy support)
            if market.votes.contains_key(user.clone()) {
                continue;
            }

            if let Some(bet) = bets::BetStorage::get_bet(&env, &market_id, &user) {
                if winning_outcomes.contains(&bet.outcome) {
                    winning_total += bet.amount;
                }
            }
        }

        if winning_total == 0 {
            return Ok(0);
        }

        let total_pool = market.total_staked;
        let fee_denominator = 10000i128; // Fee is in basis points

        let mut total_distributed: i128 = 0;

        // 1. Distribute to Voters
        // Distribute payouts to all winners (handles both single and multi-winner cases)
        // For multi-winner (ties), pool is split proportionally among all winners
        for (user, outcome) in market.votes.iter() {
            if winning_outcomes.contains(&outcome) {
                if market.claimed.get(user.clone()).unwrap_or(false) {
                    continue;
                }

                let user_stake = market.stakes.get(user.clone()).unwrap_or(0);
                if user_stake > 0 {
                    let fee_denominator = 10000i128;
                    let user_share = (user_stake
                        .checked_mul(fee_denominator - fee_percent)
                        .ok_or(Error::InvalidInput)?)
                        / fee_denominator;
                    // Payout calculation: (user_stake / total_winning_stakes) * total_pool
                    // This automatically handles split pools for ties - each winner gets proportional share
                    let payout = (user_share
                        .checked_mul(total_pool)
                        .ok_or(Error::InvalidInput)?)
                        / winning_total;

                    if payout >= 0 {
                        // Allow 0 payout but mark as claimed
                        market.claimed.set(user.clone(), true);
                        if payout > 0 {
                            total_distributed = total_distributed
                                .checked_add(payout)
                                .ok_or(Error::InvalidInput)?;

                            // Credit winnings to user balance
                            storage::BalanceStorage::add_balance(
                                &env,
                                &user,
                                &types::ReflectorAsset::Stellar,
                                payout,
                            )?;

                            EventEmitter::emit_winnings_claimed(&env, &market_id, &user, payout);
                        }
                    }
                }
            }
        }

        // 2. Distribute to Bettors
        // Check if bet outcome is in winning outcomes (supports multi-outcome/tie scenarios)
        for user in bettors.iter() {
            if let Some(mut bet) = bets::BetStorage::get_bet(&env, &market_id, &user) {
                if winning_outcomes.contains(&bet.outcome) {
                    if market.claimed.get(user.clone()).unwrap_or(false) {
                        // Already claimed (perhaps as a voter or double check)
                        bet.status = BetStatus::Won;
                        let _ = bets::BetStorage::store_bet(&env, &bet);
                        continue;
                    }

                    if bet.amount > 0 {
                        let user_share =
                            (bet.amount * (fee_denominator - fee_percent)) / fee_denominator;
                        let payout = (user_share * total_pool) / winning_total;

                        if payout > 0 {
                            market.claimed.set(user.clone(), true);
                            total_distributed += payout;

                            // Update bet status
                            bet.status = BetStatus::Won;
                            let _ = bets::BetStorage::store_bet(&env, &bet);

                            // Credit winnings to user balance instead of direct transfer
                            match storage::BalanceStorage::add_balance(
                                &env,
                                &user,
                                &types::ReflectorAsset::Stellar,
                                payout,
                            ) {
                                Ok(_) => {}
                                Err(e) => panic_with_error!(env, e),
                            }
                            EventEmitter::emit_winnings_claimed(&env, &market_id, &user, payout);
                        }
                    }
                } else {
                    // Mark losing bet
                    if bet.status == BetStatus::Active {
                        bet.status = BetStatus::Lost;
                        let _ = bets::BetStorage::store_bet(&env, &bet);
                    }
                }
            }
        }

        // Save final market state
        env.storage().persistent().set(&market_id, &market);

        Ok(total_distributed)
    }

    // ===== EVENT ARCHIVE AND HISTORICAL QUERY =====

    /// Mark a resolved or cancelled event (market) as archived. Admin only.
    /// Market must be in Resolved or Cancelled state. Returns InvalidState if not
    /// eligible, AlreadyClaimed if already archived.
    pub fn archive_event(env: Env, admin: Address, market_id: Symbol) -> Result<(), Error> {
        crate::event_archive::EventArchive::archive_event(&env, &admin, &market_id)
    }

    /// Query events by creation time range. Returns public metadata only (no votes/stakes).
    /// Paginated: cursor is start index, limit capped at 30. Returns (entries, next_cursor).
    pub fn query_events_history(
        env: Env,
        from_ts: u64,
        to_ts: u64,
        cursor: u32,
        limit: u32,
    ) -> (Vec<EventHistoryEntry>, u32) {
        crate::event_archive::EventArchive::query_events_history(
            &env, from_ts, to_ts, cursor, limit,
        )
    }

    /// Query events by resolution status (e.g. Resolved, Cancelled). Paginated.
    pub fn query_events_by_status(
        env: Env,
        status: MarketState,
        cursor: u32,
        limit: u32,
    ) -> (Vec<EventHistoryEntry>, u32) {
        crate::event_archive::EventArchive::query_events_by_resolution_status(
            &env, status, cursor, limit,
        )
    }

    /// Query events by category (oracle feed_id). Paginated.
    pub fn query_events_by_category(
        env: Env,
        category: String,
        cursor: u32,
        limit: u32,
    ) -> (Vec<EventHistoryEntry>, u32) {
        crate::event_archive::EventArchive::query_events_by_category(&env, &category, cursor, limit)
    }

    /// Set the platform fee percentage (admin only).
    ///
    /// This function allows the admin to update the platform fee percentage
    /// within the allowed limits (0-10%). The fee is applied to winning payouts.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `admin` - The administrator address (must be authorized)
    /// * `fee_percentage` - New fee percentage in basis points (e.g., 200 = 2%)
    ///
    /// # Returns
    ///
    /// Returns `Result<(), Error>` where:
    /// - `Ok(())` - Fee percentage updated successfully
    /// - `Err(Error)` - Error if update fails
    ///
    /// # Panics
    ///
    /// This function will panic with specific errors if:
    /// - `Error::Unauthorized` - Caller is not the contract admin
    /// - `Error::InvalidFeeConfig` - Fee percentage is outside valid range (0-10%)
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let admin = Address::generate(&env);
    ///
    /// // Set platform fee to 2.5% (250 basis points)
    /// match PredictifyHybrid::set_platform_fee(env.clone(), admin, 250) {
    ///     Ok(()) => println!("Fee updated successfully"),
    ///     Err(e) => println!("Fee update failed: {:?}", e),
    /// }
    /// ```
    ///
    /// # Fee Limits
    ///
    /// - Minimum fee: 0% (0 basis points)
    /// - Maximum fee: 10% (1000 basis points)
    /// - Default fee: 2% (200 basis points)
    pub fn set_platform_fee(env: Env, admin: Address, fee_percentage: i128) -> Result<(), Error> {
        // Require authentication
        admin.require_auth();

        // Verify admin - get from storage with defensive check
        let admin_key = Symbol::new(&env, "Admin");
        if !env.storage().persistent().has(&admin_key) {
            return Err(Error::Unauthorized);
        }

        let stored_admin: Address = env.storage().persistent().get(&admin_key).unwrap();
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }

        // Validate fee percentage (0-10%)
        if fee_percentage < 0 || fee_percentage > 1000 {
            return Err(Error::InvalidFeeConfig);
        }

        // Update fee in legacy storage
        let fee_key = Symbol::new(&env, "platform_fee");
        env.storage().persistent().set(&fee_key, &fee_percentage);

        Ok(())
    }

    /// Set global minimum and maximum bet limits (admin only).
    /// Applies to all events that do not have per-event limits.
    /// Rejects if min > max or outside absolute bounds (MIN_BET_AMOUNT..=MAX_BET_AMOUNT).
    pub fn set_global_bet_limits(
        env: Env,
        admin: Address,
        min_bet: i128,
        max_bet: i128,
    ) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, "Admin"))
            .unwrap_or_else(|| panic_with_error!(env, Error::AdminNotSet));
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }
        let limits = BetLimits { min_bet, max_bet };
        crate::bets::set_global_bet_limits(&env, &limits)?;
        let scope = Symbol::new(&env, "global");
        EventEmitter::emit_bet_limits_updated(&env, &admin, &scope, min_bet, max_bet);
        Ok(())
    }

    /// Set per-event minimum and maximum bet limits (admin only).
    /// Overrides global limits for the given market.
    pub fn set_event_bet_limits(
        env: Env,
        admin: Address,
        market_id: Symbol,
        min_bet: i128,
        max_bet: i128,
    ) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, "Admin"))
            .unwrap_or_else(|| panic_with_error!(env, Error::AdminNotSet));
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }
        let limits = BetLimits { min_bet, max_bet };
        crate::bets::set_event_bet_limits(&env, &market_id, &limits)?;
        EventEmitter::emit_bet_limits_updated(&env, &admin, &market_id, min_bet, max_bet);
        Ok(())
    }

    /// Get effective bet limits for a market (per-event if set, else global, else defaults).
    pub fn get_effective_bet_limits(env: Env, market_id: Symbol) -> BetLimits {
        crate::bets::get_effective_bet_limits(&env, &market_id)
    }

    /// Withdraw collected platform fees (admin only).
    ///
    /// This function allows the admin to withdraw fees that have been collected
    /// from market payouts. Fees are accumulated across all markets and can be
    /// withdrawn by the admin.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `admin` - The administrator address (must be authorized)
    /// * `amount` - Amount to withdraw (in stroops). If 0, withdraws all available fees.
    ///
    /// # Returns
    ///
    /// Returns `Result<i128, Error>` where:
    /// - `Ok(amount_withdrawn)` - Amount successfully withdrawn
    /// - `Err(Error)` - Error if withdrawal fails
    ///
    /// # Panics
    ///
    /// This function will panic with specific errors if:
    /// - `Error::Unauthorized` - Caller is not the contract admin
    /// - `Error::NoFeesToCollect` - No fees available to withdraw
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let admin = Address::generate(&env);
    ///
    /// // Withdraw all available fees
    /// match PredictifyHybrid::withdraw_collected_fees(env.clone(), admin, 0) {
    ///     Ok(amount) => println!("Withdrew {} stroops", amount),
    ///     Err(e) => println!("Withdrawal failed: {:?}", e),
    /// }
    /// ```
    pub fn withdraw_collected_fees(env: Env, admin: Address, amount: i128) -> Result<i128, Error> {
        admin.require_auth();
        if ReentrancyGuard::check_reentrancy_state(&env).is_err() {
            return Err(Error::InvalidState);
        }

        // Verify admin
        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, "Admin"))
            .unwrap_or_else(|| {
                panic_with_error!(env, Error::Unauthorized);
            });

        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }

        // Get collected fees from storage (using the same key as FeeTracker)
        let fees_key = Symbol::new(&env, "tot_fees");
        let collected_fees: i128 = env.storage().persistent().get(&fees_key).unwrap_or(0);

        if collected_fees == 0 {
            return Err(Error::NoFeesToCollect);
        }

        // Determine withdrawal amount
        let withdrawal_amount = if amount == 0 || amount > collected_fees {
            collected_fees
        } else {
            amount
        };

        // Update collected fees (checked to prevent underflow)
        let remaining_fees = collected_fees
            .checked_sub(withdrawal_amount)
            .ok_or(Error::InvalidInput)?;
        env.storage().persistent().set(&fees_key, &remaining_fees);

        // Emit fee withdrawal event
        EventEmitter::emit_fee_collected(
            &env,
            &Symbol::new(&env, "withdrawal"),
            &admin,
            withdrawal_amount,
            &String::from_str(&env, "fee_withdrawal"),
        );

        // In a real implementation, transfer tokens to admin here
        // For now, we'll just track the withdrawal

        Ok(withdrawal_amount)
    }

    /// Extends the deadline of an active market by a specified number of days (admin only).
    ///
    /// This function allows contract administrators to extend the voting/betting period
    /// of active markets. Extensions can be used to allow more time for participation,
    /// respond to unforeseen circumstances, or adjust to market conditions. The function
    /// enforces maximum extension limits and validates market state before applying changes.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `admin` - The administrator address performing the extension (must be authorized)
    /// * `market_id` - Unique identifier of the market to extend
    /// * `additional_days` - Number of days to add to the current end time
    /// * `reason` - Explanation for why the extension is needed
    ///
    /// # Returns
    ///
    /// Returns `Result<(), Error>` where:
    /// - `Ok(())` - Market deadline extended successfully
    /// - `Err(Error)` - Specific error if extension fails
    ///
    /// # Errors
    ///
    /// This function returns specific errors:
    /// - `Error::Unauthorized` - Caller is not the contract admin
    /// - `Error::MarketNotFound` - Market with given ID doesn't exist
    /// - `Error::MarketResolved` - Cannot extend a resolved market
    /// - `Error::InvalidDuration` - Extension would exceed maximum allowed limit
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address, Symbol, String};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let admin = Address::generate(&env);
    /// # let market_id = Symbol::new(&env, "market_1");
    ///
    /// // Extend market by 7 days
    /// match PredictifyHybrid::extend_deadline(
    ///     env.clone(),
    ///     admin,
    ///     market_id,
    ///     7,
    ///     String::from_str(&env, "Low participation - extending to allow more votes")
    /// ) {
    ///     Ok(()) => println!("Market deadline extended successfully"),
    ///     Err(e) => println!("Extension failed: {:?}", e),
    /// }
    /// ```
    ///
    /// # Extension Rules
    ///
    /// - Market must be in Active or Ended state (not Resolved, Closed, or Cancelled)
    /// - Total extensions cannot exceed `max_extension_days` (default 30 days)
    /// - Extensions are recorded in market's extension history
    /// - Admin must pay extension fee if configured
    ///
    /// # Security
    ///
    /// This function requires admin authentication and should be used carefully.
    /// Excessive extensions may affect user trust and market integrity. All
    /// extensions are logged with timestamps and reasons for transparency.
    pub fn extend_deadline(
        env: Env,
        admin: Address,
        market_id: Symbol,
        additional_days: u32,
        reason: String,
    ) -> Result<(), Error> {
        admin.require_auth();

        // Verify admin
        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, "Admin"))
            .unwrap_or_else(|| panic_with_error!(env, Error::Unauthorized));

        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }

        // Get market
        let mut market: Market = env
            .storage()
            .persistent()
            .get(&market_id)
            .ok_or(Error::MarketNotFound)?;

        // Validate market state - cannot extend resolved, closed, or cancelled markets
        if market.state == MarketState::Resolved
            || market.state == MarketState::Closed
            || market.state == MarketState::Cancelled
        {
            return Err(Error::MarketResolved);
        }

        // Validate extension limit
        let new_total_extension_days = market.total_extension_days + additional_days;
        if new_total_extension_days > market.max_extension_days {
            return Err(Error::InvalidDuration);
        }

        // Calculate new end time
        let seconds_per_day: u64 = 24 * 60 * 60;
        let extension_seconds: u64 = (additional_days as u64) * seconds_per_day;
        let old_end_time = market.end_time;
        let new_end_time = old_end_time + extension_seconds;

        // Calculate extension fee (could be configured per market or globally)
        let extension_fee = 0i128; // No fee for now, but can be configured

        // Create extension record
        let extension = MarketExtension::new(
            &env,
            additional_days,
            admin.clone(),
            reason.clone(),
            extension_fee,
        );

        // Update market
        market.end_time = new_end_time;
        market.total_extension_days = new_total_extension_days;
        market.extension_history.push_back(extension);

        // Save market
        env.storage().persistent().set(&market_id, &market);

        // Emit extension event
        EventEmitter::emit_market_deadline_extended(
            &env,
            &market_id,
            old_end_time,
            new_end_time,
            additional_days,
            &admin,
            &reason,
            extension_fee,
        );

        Ok(())
    }

    /// Updates the description/question of a market (admin only, before betting starts).
    ///
    /// This function allows contract administrators to update the market question
    /// or description before any bets have been placed. This ensures that market
    /// parameters can be corrected or clarified without affecting existing user
    /// commitments or predictions.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `admin` - The administrator address performing the update (must be authorized)
    /// * `market_id` - Unique identifier of the market to update
    /// * `new_description` - The updated market question or description
    ///
    /// # Returns
    ///
    /// Returns `Result<(), Error>` where:
    /// - `Ok(())` - Market description updated successfully
    /// - `Err(Error)` - Specific error if update fails
    ///
    /// # Errors
    ///
    /// This function returns specific errors:
    /// - `Error::Unauthorized` - Caller is not the contract admin
    /// - `Error::MarketNotFound` - Market with given ID doesn't exist
    /// - `Error::MarketResolved` - Cannot update a resolved market
    /// - `Error::BetsAlreadyPlaced` - Cannot update after bets have been placed
    /// - `Error::InvalidQuestion` - New description is empty or invalid
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address, Symbol, String};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let admin = Address::generate(&env);
    /// # let market_id = Symbol::new(&env, "market_1");
    ///
    /// // Update market description
    /// match PredictifyHybrid::update_event_description(
    ///     env.clone(),
    ///     admin,
    ///     market_id,
    ///     String::from_str(&env, "Will Bitcoin reach $100,000 by December 31, 2024?")
    /// ) {
    ///     Ok(()) => println!("Market description updated successfully"),
    ///     Err(e) => println!("Update failed: {:?}", e),
    /// }
    /// ```
    ///
    /// # Update Rules
    ///
    /// - Market must be in Active state
    /// - No bets can have been placed yet
    /// - Market must not be resolved
    /// - New description must be non-empty and meet length requirements
    ///
    /// # Security
    ///
    /// This function requires admin authentication and validates that no user
    /// funds are at risk. Updates are only allowed before any betting activity
    /// to maintain fairness and transparency.
    pub fn update_event_description(
        env: Env,
        admin: Address,
        market_id: Symbol,
        new_description: String,
    ) -> Result<(), Error> {
        admin.require_auth();

        // Verify admin
        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, "Admin"))
            .unwrap_or_else(|| panic_with_error!(env, Error::Unauthorized));

        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }

        // Validate new description
        if new_description.is_empty() {
            return Err(Error::InvalidQuestion);
        }

        // Get market
        let mut market: Market = env
            .storage()
            .persistent()
            .get(&market_id)
            .ok_or(Error::MarketNotFound)?;

        // Validate market state - cannot update resolved, closed, or cancelled markets
        if market.state != MarketState::Active {
            return Err(Error::MarketResolved);
        }

        // Check if any bets have been placed
        let bet_stats = bets::BetManager::get_market_bet_stats(&env, &market_id);
        if bet_stats.total_bets > 0 {
            return Err(Error::BetsAlreadyPlaced);
        }

        // Check if any votes have been placed
        if market.total_staked > 0 {
            return Err(Error::AlreadyVoted);
        }

        // Store old description for event
        let old_description = market.question.clone();

        // Update market description
        market.question = new_description.clone();

        // Save market
        env.storage().persistent().set(&market_id, &market);

        // Emit description update event
        EventEmitter::emit_market_description_updated(
            &env,
            &market_id,
            &old_description,
            &new_description,
            &admin,
        );

        Ok(())
    }

    /// Updates the outcomes of a market (admin only, before betting starts).
    ///
    /// This function allows contract administrators to update the available
    /// outcomes for a market before any bets have been placed. This ensures
    /// that market parameters can be corrected or adjusted without affecting
    /// existing user commitments.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `admin` - The administrator address performing the update (must be authorized)
    /// * `market_id` - Unique identifier of the market to update
    /// * `new_outcomes` - The updated list of possible outcomes
    ///
    /// # Returns
    ///
    /// Returns `Result<(), Error>` where:
    /// - `Ok(())` - Market outcomes updated successfully
    /// - `Err(Error)` - Specific error if update fails
    ///
    /// # Errors
    ///
    /// This function returns specific errors:
    /// - `Error::Unauthorized` - Caller is not the contract admin
    /// - `Error::MarketNotFound` - Market with given ID doesn't exist
    /// - `Error::MarketResolved` - Cannot update a resolved market
    /// - `Error::BetsAlreadyPlaced` - Cannot update after bets have been placed
    /// - `Error::InvalidOutcomes` - New outcomes list is invalid (< 2 outcomes or empty strings)
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address, Symbol, String, Vec};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let admin = Address::generate(&env);
    /// # let market_id = Symbol::new(&env, "market_1");
    ///
    /// // Update market outcomes
    /// let new_outcomes = Vec::from_array(&env, [
    ///     String::from_str(&env, "Yes"),
    ///     String::from_str(&env, "No"),
    ///     String::from_str(&env, "Uncertain")
    /// ]);
    ///
    /// match PredictifyHybrid::update_event_outcomes(
    ///     env.clone(),
    ///     admin,
    ///     market_id,
    ///     new_outcomes
    /// ) {
    ///     Ok(()) => println!("Market outcomes updated successfully"),
    ///     Err(e) => println!("Update failed: {:?}", e),
    /// }
    /// ```
    ///
    /// # Update Rules
    ///
    /// - Market must be in Active state
    /// - No bets can have been placed yet
    /// - Market must not be resolved
    /// - New outcomes must have at least 2 options
    /// - All outcome strings must be non-empty
    ///
    /// # Security
    ///
    /// This function requires admin authentication and validates that no user
    /// funds are at risk. Updates are only allowed before any betting activity
    /// to maintain fairness and transparency.
    pub fn update_event_outcomes(
        env: Env,
        admin: Address,
        market_id: Symbol,
        new_outcomes: Vec<String>,
    ) -> Result<(), Error> {
        admin.require_auth();

        // Verify admin
        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, "Admin"))
            .unwrap_or_else(|| panic_with_error!(env, Error::Unauthorized));

        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }

        // Validate new outcomes
        if new_outcomes.len() < 2 {
            return Err(Error::InvalidOutcomes);
        }

        // Check all outcomes are non-empty
        for outcome in new_outcomes.iter() {
            if outcome.is_empty() {
                return Err(Error::InvalidOutcome);
            }
        }

        // Get market
        let mut market: Market = env
            .storage()
            .persistent()
            .get(&market_id)
            .ok_or(Error::MarketNotFound)?;

        // Validate market state - cannot update resolved, closed, or cancelled markets
        if market.state != MarketState::Active {
            return Err(Error::MarketResolved);
        }

        // Check if any bets have been placed
        let bet_stats = bets::BetManager::get_market_bet_stats(&env, &market_id);
        if bet_stats.total_bets > 0 {
            return Err(Error::BetsAlreadyPlaced);
        }

        // Check if any votes have been placed
        if market.total_staked > 0 {
            return Err(Error::AlreadyVoted);
        }

        // Store old outcomes for event
        let old_outcomes = market.outcomes.clone();

        // Update market outcomes
        market.outcomes = new_outcomes.clone();

        // Save market
        env.storage().persistent().set(&market_id, &market);

        // Emit outcomes update event
        EventEmitter::emit_market_outcomes_updated(
            &env,
            &market_id,
            &old_outcomes,
            &new_outcomes,
            &admin,
        );

        Ok(())
    }

    /// Updates the category of a market (admin only, before betting starts).
    ///
    /// This function allows contract administrators to set or update the category
    /// for a market before any bets have been placed. Categories help clients
    /// filter and display markets by type (e.g., sports, crypto, politics).
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `admin` - The administrator address performing the update (must be authorized)
    /// * `market_id` - Unique identifier of the market to update
    /// * `category` - The new category (None to clear the category)
    ///
    /// # Returns
    ///
    /// Returns `Result<(), Error>` where:
    /// - `Ok(())` - Market category updated successfully
    /// - `Err(Error)` - Specific error if update fails
    ///
    /// # Errors
    ///
    /// This function returns specific errors:
    /// - `Error::Unauthorized` - Caller is not the contract admin
    /// - `Error::MarketNotFound` - Market with given ID doesn't exist
    /// - `Error::MarketResolved` - Cannot update a resolved market
    /// - `Error::BetsAlreadyPlaced` - Cannot update after bets have been placed
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address, Symbol, String};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let admin = Address::generate(&env);
    /// # let market_id = Symbol::new(&env, "market_1");
    ///
    /// // Set market category
    /// match PredictifyHybrid::update_event_category(
    ///     env.clone(),
    ///     admin,
    ///     market_id,
    ///     Some(String::from_str(&env, "sports"))
    /// ) {
    ///     Ok(()) => println!("Market category updated successfully"),
    ///     Err(e) => println!("Update failed: {:?}", e),
    /// }
    /// ```
    pub fn update_event_category(
        env: Env,
        admin: Address,
        market_id: Symbol,
        category: Option<String>,
    ) -> Result<(), Error> {
        admin.require_auth();

        // Verify admin
        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, "Admin"))
            .unwrap_or_else(|| panic_with_error!(env, Error::Unauthorized));

        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }

        // Get market
        let mut market: Market = env
            .storage()
            .persistent()
            .get(&market_id)
            .ok_or(Error::MarketNotFound)?;

        // Validate market state - cannot update resolved, closed, or cancelled markets
        if market.state != MarketState::Active {
            return Err(Error::MarketResolved);
        }

        // Check if any bets have been placed
        let bet_stats = bets::BetManager::get_market_bet_stats(&env, &market_id);
        if bet_stats.total_bets > 0 {
            return Err(Error::BetsAlreadyPlaced);
        }

        // Check if any votes have been placed
        if market.total_staked > 0 {
            return Err(Error::AlreadyVoted);
        }

        // Store old category for event
        let old_category = market.category.clone();

        // Update market category
        market.category = category.clone();

        // Save market
        env.storage().persistent().set(&market_id, &market);

        // Emit category update event
        EventEmitter::emit_category_updated(&env, &market_id, &old_category, &category, &admin);

        Ok(())
    }

    /// Updates the tags of a market (admin only, before betting starts).
    ///
    /// This function allows contract administrators to set or update tags
    /// for a market before any bets have been placed. Tags help clients
    /// filter and search markets by multiple dimensions.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `admin` - The administrator address performing the update (must be authorized)
    /// * `market_id` - Unique identifier of the market to update
    /// * `tags` - The new list of tags (empty Vec to clear all tags)
    ///
    /// # Returns
    ///
    /// Returns `Result<(), Error>` where:
    /// - `Ok(())` - Market tags updated successfully
    /// - `Err(Error)` - Specific error if update fails
    ///
    /// # Errors
    ///
    /// This function returns specific errors:
    /// - `Error::Unauthorized` - Caller is not the contract admin
    /// - `Error::MarketNotFound` - Market with given ID doesn't exist
    /// - `Error::MarketResolved` - Cannot update a resolved market
    /// - `Error::BetsAlreadyPlaced` - Cannot update after bets have been placed
    /// - `Error::InvalidInput` - One or more tags are empty strings
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address, Symbol, String, vec};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let admin = Address::generate(&env);
    /// # let market_id = Symbol::new(&env, "market_1");
    ///
    /// // Set market tags
    /// let tags = vec![
    ///     &env,
    ///     String::from_str(&env, "bitcoin"),
    ///     String::from_str(&env, "crypto"),
    ///     String::from_str(&env, "price-prediction")
    /// ];
    ///
    /// match PredictifyHybrid::update_event_tags(
    ///     env.clone(),
    ///     admin,
    ///     market_id,
    ///     tags
    /// ) {
    ///     Ok(()) => println!("Market tags updated successfully"),
    ///     Err(e) => println!("Update failed: {:?}", e),
    /// }
    /// ```
    pub fn update_event_tags(
        env: Env,
        admin: Address,
        market_id: Symbol,
        tags: Vec<String>,
    ) -> Result<(), Error> {
        admin.require_auth();

        // Verify admin
        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, "Admin"))
            .unwrap_or_else(|| panic_with_error!(env, Error::Unauthorized));

        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }

        // Validate tags - none should be empty
        for tag in tags.iter() {
            if tag.is_empty() {
                return Err(Error::InvalidInput);
            }
        }

        // Get market
        let mut market: Market = env
            .storage()
            .persistent()
            .get(&market_id)
            .ok_or(Error::MarketNotFound)?;

        // Validate market state - cannot update resolved, closed, or cancelled markets
        if market.state != MarketState::Active {
            return Err(Error::MarketResolved);
        }

        // Check if any bets have been placed
        let bet_stats = bets::BetManager::get_market_bet_stats(&env, &market_id);
        if bet_stats.total_bets > 0 {
            return Err(Error::BetsAlreadyPlaced);
        }

        // Check if any votes have been placed
        if market.total_staked > 0 {
            return Err(Error::AlreadyVoted);
        }

        // Store old tags for event
        let old_tags = market.tags.clone();

        // Update market tags
        market.tags = tags.clone();

        // Save market
        env.storage().persistent().set(&market_id, &market);

        // Emit tags update event
        EventEmitter::emit_tags_updated(&env, &market_id, &old_tags, &tags, &admin);

        Ok(())
    }

    /// Query events by tags (paginated, bounded).
    ///
    /// Returns events that have ANY of the provided tags (OR logic).
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment
    /// * `tags` - Tags to filter by (events matching any tag are returned)
    /// * `cursor` - Pagination cursor
    /// * `limit` - Maximum results per page
    ///
    /// # Returns
    ///
    /// Tuple of (events, next_cursor)
    pub fn query_events_by_tags(
        env: Env,
        tags: Vec<String>,
        cursor: u32,
        limit: u32,
    ) -> (Vec<EventHistoryEntry>, u32) {
        event_archive::EventArchive::query_events_by_tags(&env, &tags, cursor, limit)
    }

    /// Cancel an event and automatically refund all placed bets (admin only).
    ///
    /// This function allows admins to cancel events before resolution and
    /// automatically refund all bets placed on the market. It validates
    /// cancellation conditions, updates market status, and processes refunds.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `admin` - The administrator address (must be authorized)
    /// * `market_id` - Unique identifier of the market to cancel
    /// * `reason` - Optional reason for cancellation
    ///
    /// # Returns
    ///
    /// Returns `Result<i128, Error>` where:
    /// - `Ok(total_refunded)` - Total amount refunded to users
    /// - `Err(Error)` - Error if cancellation fails
    ///
    /// # Panics
    ///
    /// This function will panic with specific errors if:
    /// - `Error::Unauthorized` - Caller is not the contract admin
    /// - `Error::MarketNotFound` - Market with given ID doesn't exist
    /// - `Error::MarketResolved` - Market has already been resolved
    /// - `Error::InvalidState` - Market is in an invalid state for cancellation
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address, String, Symbol};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let admin = Address::generate(&env);
    /// # let market_id = Symbol::new(&env, "market_1");
    ///
    /// match PredictifyHybrid::cancel_event(
    ///     env.clone(),
    ///     admin,
    ///     market_id,
    ///     Some(String::from_str(&env, "Oracle data unavailable"))
    /// ) {
    ///     Ok(total) => println!("Refunded {} stroops", total),
    ///     Err(e) => println!("Cancellation failed: {:?}", e),
    /// }
    /// ```
    ///
    /// # Cancellation Conditions
    ///
    /// - Market must exist and be active
    /// - Market must not be resolved
    /// - Market must not already be cancelled
    /// - Only admin can cancel events
    ///
    /// # Refund Process
    ///
    /// 1. All active bets are identified
    /// 2. Funds are unlocked and returned to users
    /// 3. Bet status is updated to "Refunded"
    /// 4. Market state is updated to "Cancelled"
    /// 5. Cancellation and refund events are emitted
    pub fn cancel_event(
        env: Env,
        admin: Address,
        market_id: Symbol,
        reason: Option<String>,
    ) -> Result<i128, Error> {
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
            return Err(Error::Unauthorized);
        }

        // Get and validate market
        let mut market: Market = env
            .storage()
            .persistent()
            .get(&market_id)
            .unwrap_or_else(|| {
                panic_with_error!(env, Error::MarketNotFound);
            });

        // Validate cancellation conditions
        if market.state == MarketState::Resolved {
            return Err(Error::MarketResolved);
        }

        if market.state == MarketState::Cancelled {
            // Already cancelled, return 0 refunded
            return Ok(0);
        }

        // Market must be active or ended (not resolved)
        if !matches!(market.state, MarketState::Active | MarketState::Ended) {
            return Err(Error::InvalidState);
        }

        // Capture old state for event
        let old_state = market.state.clone();

        // Update market state to cancelled
        market.state = MarketState::Cancelled;
        env.storage().persistent().set(&market_id, &market);

        // Refund all bets under reentrancy lock (batch of token transfers)
        if ReentrancyGuard::check_reentrancy_state(&env).is_err() {
            return Err(Error::InvalidState);
        }
        if ReentrancyGuard::before_external_call(&env).is_err() {
            return Err(Error::InvalidState);
        }
        let refund_result = bets::BetManager::refund_market_bets(&env, &market_id);
        ReentrancyGuard::after_external_call(&env);
        refund_result?;

        // Calculate total refunded (sum of all bets)
        let total_refunded = market.total_staked;

        // Emit cancellation event
        EventEmitter::emit_state_change_event(
            &env,
            &market_id,
            &old_state,
            &MarketState::Cancelled,
            &reason.unwrap_or_else(|| String::from_str(&env, "Event cancelled by admin")),
        );

        // Emit market closed event
        EventEmitter::emit_market_closed(&env, &market_id, &admin);

        Ok(total_refunded)
    }

    /// Refund all bets when oracle resolution fails or times out (automatic refund path).
    ///
    /// Callable when: market has ended, no oracle result, and either (1) resolution
    /// timeout has passed since market end, or (2) caller is admin (confirmed failure).
    /// Refunds full bet amount per user (no fee deduction). Marks market as cancelled and
    /// prevents further resolution. Emits refund events. Idempotent when already cancelled.
    pub fn refund_on_oracle_failure(
        env: Env,
        caller: Address,
        market_id: Symbol,
    ) -> Result<i128, Error> {
        caller.require_auth();

        let mut market: Market = env
            .storage()
            .persistent()
            .get(&market_id)
            .ok_or(Error::MarketNotFound)?;

        if market.state == MarketState::Cancelled {
            return Ok(0);
        }
        if market.winning_outcomes.is_some() {
            return Err(Error::MarketResolved);
        }
        if market.oracle_result.is_some() {
            return Err(Error::MarketResolved);
        }
        let current_time = env.ledger().timestamp();
        if current_time < market.end_time {
            return Err(Error::MarketClosed);
        }

        let stored_admin: Option<Address> =
            env.storage().persistent().get(&Symbol::new(&env, "Admin"));
        let is_admin = stored_admin.as_ref().map_or(false, |a| a == &caller);
        let timeout_passed = current_time.saturating_sub(market.end_time)
            >= config::DEFAULT_RESOLUTION_TIMEOUT_SECONDS;
        if !is_admin && !timeout_passed {
            return Err(Error::Unauthorized);
        }

        let old_state = market.state.clone();
        market.state = MarketState::Cancelled;
        env.storage().persistent().set(&market_id, &market);

        if reentrancy_guard::ReentrancyGuard::check_reentrancy_state(&env).is_err() {
            return Err(Error::InvalidState);
        }
        if reentrancy_guard::ReentrancyGuard::before_external_call(&env).is_err() {
            return Err(Error::InvalidState);
        }
        let refund_result = bets::BetManager::refund_market_bets(&env, &market_id);
        reentrancy_guard::ReentrancyGuard::after_external_call(&env);
        refund_result?;

        let total_refunded = market.total_staked;
        EventEmitter::emit_state_change_event(
            &env,
            &market_id,
            &old_state,
            &MarketState::Cancelled,
            &String::from_str(&env, "Refund on oracle failure/timeout"),
        );
        EventEmitter::emit_refund_on_oracle_failure(&env, &market_id, total_refunded);

        Ok(total_refunded)
    }

    /// Extend market duration (admin only)
    pub fn extend_market(
        env: Env,
        admin: Address,
        market_id: Symbol,
        additional_days: u32,
        reason: String,
        _fee_amount: i128,
    ) -> Result<(), Error> {
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

        extensions::ExtensionManager::extend_market_duration(
            &env,
            admin,
            market_id,
            additional_days,
            reason,
        )
    }

    // ===== STORAGE OPTIMIZATION FUNCTIONS =====

    /// Compress market data for storage optimization
    pub fn compress_market_data(
        env: Env,
        market_id: Symbol,
    ) -> Result<storage::CompressedMarket, Error> {
        let market = match markets::MarketStateManager::get_market(&env, &market_id) {
            Ok(m) => m,
            Err(e) => return Err(e),
        };

        storage::StorageOptimizer::compress_market_data(&env, &market)
    }

    /// Clean up old market data based on age and state
    pub fn cleanup_old_market_data(env: Env, market_id: Symbol) -> Result<bool, Error> {
        storage::StorageOptimizer::cleanup_old_market_data(&env, &market_id)
    }

    /// Migrate storage format from old to new format
    pub fn migrate_storage_format(
        env: Env,
        from_format: storage::StorageFormat,
        to_format: storage::StorageFormat,
    ) -> Result<storage::StorageMigration, Error> {
        storage::StorageOptimizer::migrate_storage_format(&env, from_format, to_format)
    }

    /// Monitor storage usage and return statistics
    pub fn monitor_storage_usage(env: Env) -> Result<storage::StorageUsageStats, Error> {
        storage::StorageOptimizer::monitor_storage_usage(&env)
    }

    /// Optimize storage layout for a specific market
    pub fn optimize_storage_layout(env: Env, market_id: Symbol) -> Result<bool, Error> {
        storage::StorageOptimizer::optimize_storage_layout(&env, &market_id)
    }

    /// Get storage usage statistics
    pub fn get_storage_usage_statistics(env: Env) -> Result<storage::StorageUsageStats, Error> {
        storage::StorageOptimizer::get_storage_usage_statistics(&env)
    }

    /// Validate storage integrity for a specific market
    pub fn validate_storage_integrity(
        env: Env,
        market_id: Symbol,
    ) -> Result<storage::StorageIntegrityResult, Error> {
        storage::StorageOptimizer::validate_storage_integrity(&env, &market_id)
    }

    /// Get storage configuration
    pub fn get_storage_config(env: Env) -> storage::StorageConfig {
        storage::StorageOptimizer::get_storage_config(&env)
    }

    /// Update storage configuration
    pub fn update_storage_config(env: Env, config: storage::StorageConfig) -> Result<(), Error> {
        storage::StorageOptimizer::update_storage_config(&env, &config)
    }

    /// Calculate storage cost for a market
    pub fn calculate_storage_cost(env: Env, market_id: Symbol) -> Result<u64, Error> {
        let market = match markets::MarketStateManager::get_market(&env, &market_id) {
            Ok(m) => m,
            Err(e) => return Err(e),
        };

        Ok(storage::StorageUtils::calculate_storage_cost(&market))
    }

    /// Get storage efficiency score for a market
    pub fn get_storage_efficiency_score(env: Env, market_id: Symbol) -> Result<u32, Error> {
        let market = match markets::MarketStateManager::get_market(&env, &market_id) {
            Ok(m) => m,
            Err(e) => return Err(e),
        };

        Ok(storage::StorageUtils::get_storage_efficiency_score(&market))
    }

    /// Get storage recommendations for a market
    pub fn get_storage_recommendations(env: Env, market_id: Symbol) -> Result<Vec<String>, Error> {
        let market = match markets::MarketStateManager::get_market(&env, &market_id) {
            Ok(m) => m,
            Err(e) => return Err(e),
        };

        Ok(storage::StorageUtils::get_storage_recommendations(&market))
    }

    // ===== ERROR RECOVERY FUNCTIONS =====

    /// Recover from an error using appropriate recovery strategy
    pub fn recover_from_error(
        env: Env,
        error: Error,
        context: errors::ErrorContext,
    ) -> Result<errors::ErrorRecovery, Error> {
        errors::ErrorHandler::recover_from_error(&env, error, context)
    }

    /// Validate error recovery configuration and state
    pub fn validate_error_recovery(
        env: Env,
        recovery: errors::ErrorRecovery,
    ) -> Result<bool, Error> {
        errors::ErrorHandler::validate_error_recovery(&env, &recovery)
    }

    /// Get current error recovery status and statistics
    pub fn get_error_recovery_status(env: Env) -> Result<errors::ErrorRecoveryStatus, Error> {
        errors::ErrorHandler::get_error_recovery_status(&env)
    }

    /// Emit error recovery event for monitoring and logging
    pub fn emit_error_recovery_event(env: Env, recovery: errors::ErrorRecovery) {
        errors::ErrorHandler::emit_error_recovery_event(&env, &recovery);
    }

    /// Validate resilience patterns configuration
    pub fn validate_resilience_patterns(
        env: Env,
        patterns: Vec<errors::ResiliencePattern>,
    ) -> Result<bool, Error> {
        errors::ErrorHandler::validate_resilience_patterns(&env, &patterns)
    }

    /// Document error recovery procedures and best practices
    pub fn document_error_recovery(env: Env) -> Result<soroban_sdk::Map<String, String>, Error> {
        errors::ErrorHandler::document_error_recovery_procedures(&env)
    }

    // ===== EDGE CASE HANDLING ENTRY POINTS =====

    /// Handle zero stake scenario for a specific market
    pub fn handle_zero_stake_scenario(env: Env, market_id: Symbol) -> Result<(), Error> {
        edge_cases::EdgeCaseHandler::handle_zero_stake_scenario(&env, market_id)
    }

    /// Implement tie-breaking mechanism for equal outcomes
    pub fn implement_tie_breaking_mechanism(
        env: Env,
        outcomes: Vec<String>,
    ) -> Result<String, Error> {
        edge_cases::EdgeCaseHandler::implement_tie_breaking_mechanism(&env, outcomes)
    }

    /// Detect orphaned markets and return their IDs
    pub fn detect_orphaned_markets(env: Env) -> Result<Vec<Symbol>, Error> {
        edge_cases::EdgeCaseHandler::detect_orphaned_markets(&env)
    }

    /// Handle partial resolution with incomplete data
    pub fn handle_partial_resolution(
        env: Env,
        market_id: Symbol,
        partial_data: edge_cases::PartialData,
    ) -> Result<(), Error> {
        edge_cases::EdgeCaseHandler::handle_partial_resolution(&env, market_id, partial_data)
    }

    /// Validate edge case handling scenario
    pub fn validate_edge_case_handling(
        env: Env,
        scenario: edge_cases::EdgeCaseScenario,
    ) -> Result<(), Error> {
        edge_cases::EdgeCaseHandler::validate_edge_case_handling(&env, scenario)
    }

    /// Run comprehensive edge case testing scenarios
    pub fn test_edge_case_scenarios(env: Env) -> Result<(), Error> {
        edge_cases::EdgeCaseHandler::test_edge_case_scenarios(&env)
    }

    /// Get comprehensive edge case statistics
    pub fn get_edge_case_statistics(env: Env) -> Result<edge_cases::EdgeCaseStats, Error> {
        edge_cases::EdgeCaseHandler::get_edge_case_statistics(&env)
    }

    // ===== RECOVERY PUBLIC METHODS =====
    /// Initiates or performs recovery of a potentially corrupted market state. Only admin.
    pub fn recover_market_state(env: Env, admin: Address, market_id: Symbol) -> bool {
        admin.require_auth();
        if let Err(e) = crate::recovery::RecoveryManager::assert_is_admin(&env, &admin) {
            panic_with_error!(env, e);
        }
        match crate::recovery::RecoveryManager::recover_market_state(&env, &market_id) {
            Ok(res) => res,
            Err(e) => panic_with_error!(env, e),
        }
    }

    /// Executes partial refund mechanism for selected users in a failed/corrupted market. Only admin.
    pub fn partial_refund_mechanism(
        env: Env,
        admin: Address,
        market_id: Symbol,
        users: Vec<Address>,
    ) -> i128 {
        admin.require_auth();
        if let Err(e) = crate::recovery::RecoveryManager::assert_is_admin(&env, &admin) {
            panic_with_error!(env, e);
        }
        match crate::recovery::RecoveryManager::partial_refund_mechanism(&env, &market_id, &users) {
            Ok(total_refunded) => total_refunded,
            Err(e) => panic_with_error!(env, e),
        }
    }

    /// Validates market state integrity; returns true if consistent.
    pub fn validate_market_state_integrity(env: Env, market_id: Symbol) -> bool {
        match crate::recovery::RecoveryValidator::validate_market_state_integrity(&env, &market_id)
        {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    /// Returns recovery status for a market.
    pub fn get_recovery_status(env: Env, market_id: Symbol) -> String {
        crate::recovery::RecoveryManager::get_recovery_status(&env, &market_id)
            .unwrap_or_else(|_| String::from_str(&env, "unknown"))
    }

    // ===== VERSIONING FUNCTIONS =====

    /// Track contract version for versioning system
    pub fn track_contract_version(env: Env, version: versioning::Version) -> Result<(), Error> {
        versioning::VersionManager::new(&env).track_contract_version(&env, version)
    }

    /// Migrate data between contract versions
    pub fn migrate_data_between_versions(
        env: Env,
        old_version: versioning::Version,
        new_version: versioning::Version,
    ) -> Result<versioning::VersionMigration, Error> {
        versioning::VersionManager::new(&env).migrate_data_between_versions(
            &env,
            old_version,
            new_version,
        )
    }

    /// Validate version compatibility
    pub fn validate_version_compatibility(
        env: Env,
        old_version: versioning::Version,
        new_version: versioning::Version,
    ) -> Result<bool, Error> {
        versioning::VersionManager::new(&env).validate_version_compatibility(
            &env,
            &old_version,
            &new_version,
        )
    }

    /// Upgrade to a specific version
    pub fn upgrade_to_version(env: Env, target_version: versioning::Version) -> Result<(), Error> {
        versioning::VersionManager::new(&env).upgrade_to_version(&env, target_version)
    }

    /// Rollback to a specific version
    pub fn rollback_to_version(env: Env, target_version: versioning::Version) -> Result<(), Error> {
        versioning::VersionManager::new(&env).rollback_to_version(&env, target_version)
    }

    /// Get version history
    pub fn get_version_history(env: Env) -> Result<versioning::VersionHistory, Error> {
        versioning::VersionManager::new(&env).get_version_history(&env)
    }

    /// Test version migration
    pub fn test_version_migration(
        env: Env,
        migration: versioning::VersionMigration,
    ) -> Result<bool, Error> {
        versioning::VersionManager::new(&env).test_version_migration(&env, migration)
    }

    // ===== MONITORING FUNCTIONS =====

    /// Monitor market health for a specific market
    pub fn monitor_market_health(
        env: Env,
        market_id: Symbol,
    ) -> Result<monitoring::MarketHealthMetrics, Error> {
        monitoring::ContractMonitor::monitor_market_health(&env, market_id)
    }

    /// Monitor oracle health for a specific oracle provider
    pub fn monitor_oracle_health(
        env: Env,
        oracle: OracleProvider,
    ) -> Result<monitoring::OracleHealthMetrics, Error> {
        monitoring::ContractMonitor::monitor_oracle_health(&env, oracle)
    }

    /// Monitor fee collection performance
    pub fn monitor_fee_collection(
        env: Env,
        timeframe: monitoring::TimeFrame,
    ) -> Result<monitoring::FeeCollectionMetrics, Error> {
        monitoring::ContractMonitor::monitor_fee_collection(&env, timeframe)
    }

    /// Monitor dispute resolution performance
    pub fn monitor_dispute_resolution(
        env: Env,
        market_id: Symbol,
    ) -> Result<monitoring::DisputeResolutionMetrics, Error> {
        monitoring::ContractMonitor::monitor_dispute_resolution(&env, market_id)
    }

    /// Get comprehensive contract performance metrics
    pub fn get_contract_performance_metrics(
        env: Env,
        timeframe: monitoring::TimeFrame,
    ) -> Result<monitoring::PerformanceMetrics, Error> {
        monitoring::ContractMonitor::get_contract_performance_metrics(&env, timeframe)
    }

    /// Emit monitoring alert
    pub fn emit_monitoring_alert(
        env: Env,
        alert: monitoring::MonitoringAlert,
    ) -> Result<(), Error> {
        monitoring::ContractMonitor::emit_monitoring_alert(&env, alert)
    }

    /// Validate monitoring data integrity
    pub fn validate_monitoring_data(
        env: Env,
        data: monitoring::MonitoringData,
    ) -> Result<bool, Error> {
        monitoring::ContractMonitor::validate_monitoring_data(&env, &data)
    }

    // ===== ORACLE FALLBACK FUNCTIONS =====

    /// Get oracle data with backup if primary fails
    pub fn get_oracle_with_backup(
        env: Env,
        market_id: Symbol,
        oracle_contract: Address,
        primary_oracle: OracleProvider,
        backup_oracle: OracleProvider,
    ) -> Result<String, Error> {
        // Get market info
        let market = env
            .storage()
            .persistent()
            .get::<Symbol, Market>(&market_id)
            .ok_or(Error::MarketNotFound)?;

        // Check if market ended
        let current_time = env.ledger().timestamp();
        if current_time < market.end_time {
            return Err(Error::MarketClosed);
        }

        // Try to get price with backup
        let backup = OracleBackup::new(primary_oracle, backup_oracle);
        match backup.get_price(&env, &oracle_contract, &market.oracle_config.feed_id) {
            Ok(price) => {
                // Simple comparison logic
                let threshold = market.oracle_config.threshold;
                let comparison = &market.oracle_config.comparison;

                let result = if comparison == &String::from_str(&env, "gt") {
                    if price > threshold {
                        "yes"
                    } else {
                        "no"
                    }
                } else if comparison == &String::from_str(&env, "lt") {
                    if price < threshold {
                        "yes"
                    } else {
                        "no"
                    }
                } else {
                    if price == threshold {
                        "yes"
                    } else {
                        "no"
                    }
                };

                Ok(String::from_str(&env, result))
            }
            Err(_) => {
                // Both oracles failed
                let reason = String::from_str(&env, "All oracles failed");
                events::EventEmitter::emit_manual_resolution_required(&env, &market_id, &reason);
                Err(Error::OracleUnavailable)
            }
        }
    }

    /// Check if oracle is working
    pub fn check_oracle_status(
        env: Env,
        oracle: OracleProvider,
        oracle_contract: Address,
    ) -> String {
        let health = graceful_degradation::monitor_oracle_health(&env, oracle, &oracle_contract);
        match health {
            OracleHealth::Working => String::from_str(&env, "working"),
            OracleHealth::Broken => String::from_str(&env, "broken"),
        }
    }

    // ===== MULTI-ADMIN MANAGEMENT FUNCTIONS =====

    /// Add a new admin with specified role (SuperAdmin only)
    pub fn add_admin(
        env: Env,
        current_admin: Address,
        new_admin: Address,
        role: AdminRole,
    ) -> Result<(), Error> {
        current_admin.require_auth();
        AdminManager::add_admin(&env, &current_admin, &new_admin, role)
    }

    /// Remove an admin from the system (SuperAdmin only)
    pub fn remove_admin(
        env: Env,
        current_admin: Address,
        admin_to_remove: Address,
    ) -> Result<(), Error> {
        current_admin.require_auth();
        AdminManager::remove_admin(&env, &current_admin, &admin_to_remove)
    }

    /// Update an admin's role (SuperAdmin only)
    pub fn update_admin_role(
        env: Env,
        current_admin: Address,
        target_admin: Address,
        new_role: AdminRole,
    ) -> Result<(), Error> {
        current_admin.require_auth();
        AdminManager::update_admin_role(&env, &current_admin, &target_admin, new_role)
    }

    /// Validate admin permission for specific action
    pub fn validate_admin_permission(
        env: Env,
        admin: Address,
        permission: AdminPermission,
    ) -> Result<(), Error> {
        AdminManager::validate_admin_permission(&env, &admin, permission)
    }

    /// Get all admin roles in the system
    pub fn get_admin_roles(env: Env) -> Map<Address, AdminRole> {
        AdminManager::get_admin_roles(&env)
    }

    /// Get comprehensive admin analytics
    pub fn get_admin_analytics(env: Env) -> AdminAnalyticsResult {
        admin::EnhancedAdminAnalytics::get_admin_analytics(&env)
    }

    /// Migrate from single-admin to multi-admin system
    pub fn migrate_to_multi_admin(env: Env, admin: Address) -> Result<(), Error> {
        admin.require_auth();
        admin::AdminSystemIntegration::migrate_to_multi_admin(&env)
    }

    /// Check if multi-admin migration is complete
    pub fn is_multi_admin_migrated(env: Env) -> bool {
        admin::AdminSystemIntegration::is_migrated(&env)
    }

    /// Check role permissions against a specific permission
    pub fn check_role_permissions(env: Env, role: AdminRole, permission: AdminPermission) -> bool {
        AdminManager::check_role_permissions(&env, role, permission)
    }

    // ===== CONTRACT UPGRADE METHODS =====

    /// Upgrade the contract to new Wasm bytecode
    ///
    /// This function allows authorized admins to upgrade the contract to a new
    /// version by replacing the Wasm bytecode. It includes comprehensive validation,
    /// version checking, and event logging.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment
    /// * `admin` - The admin performing the upgrade (must be authorized)
    /// * `new_wasm_hash` - Hash of the new Wasm bytecode to deploy
    ///
    /// # Returns
    ///
    /// * `Ok(())` if upgrade succeeds
    /// * `Err(Error)` if authorization fails or upgrade is incompatible
    ///
    /// # Security
    ///
    /// - Requires admin authentication via `require_auth()`
    /// - Validates version compatibility
    /// - Performs safety checks before upgrade
    /// - Logs all upgrade attempts for audit trail
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address, BytesN};
    /// # let env = Env::default();
    /// # let admin = Address::generate(&env);
    /// # let new_wasm_hash = BytesN::from_array(&env, &[0u8; 32]);
    ///
    /// // Perform upgrade with admin authorization
    /// admin.require_auth();
    /// PredictifyHybrid::upgrade_contract(env, admin, new_wasm_hash)?;
    /// # Ok::<(), predictify_hybrid::errors::Error>(())
    /// ```
    pub fn upgrade_contract(
        env: Env,
        admin: Address,
        new_wasm_hash: soroban_sdk::BytesN<32>,
    ) -> Result<(), Error> {
        admin.require_auth();
        upgrade_manager::UpgradeManager::upgrade_contract(&env, &admin, new_wasm_hash)
    }

    /// Rollback contract to previous version
    ///
    /// Reverts the contract to a previous Wasm version. This is a critical
    /// recovery mechanism for failed upgrades.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment
    /// * `admin` - The admin performing the rollback (must be authorized)
    /// * `rollback_wasm_hash` - Hash of the Wasm bytecode to rollback to
    ///
    /// # Returns
    ///
    /// * `Ok(())` if rollback succeeds
    /// * `Err(Error)` if authorization fails or rollback is invalid
    pub fn rollback_upgrade(
        env: Env,
        admin: Address,
        rollback_wasm_hash: soroban_sdk::BytesN<32>,
    ) -> Result<(), Error> {
        admin.require_auth();
        upgrade_manager::UpgradeManager::rollback_upgrade(&env, &admin, rollback_wasm_hash)
    }

    /// Get current contract version
    ///
    /// Returns the currently active contract version information.
    ///
    /// # Returns
    ///
    /// * `Ok(Version)` - Current contract version
    /// * `Err(Error)` - If version cannot be retrieved
    pub fn get_contract_version(env: Env) -> Result<versioning::Version, Error> {
        upgrade_manager::UpgradeManager::get_contract_version(&env)
    }

    /// Check if upgrade is available
    ///
    /// Checks if there are approved upgrade proposals ready for execution.
    ///
    /// # Returns
    ///
    /// * `Ok(bool)` - True if upgrade is available
    pub fn check_upgrade_available(env: Env) -> Result<bool, Error> {
        upgrade_manager::UpgradeManager::check_upgrade_available(&env)
    }

    /// Get upgrade history
    ///
    /// Retrieves complete history of all contract upgrades.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<UpgradeRecord>)` - List of all upgrade records
    pub fn get_upgrade_history(env: Env) -> Result<Vec<upgrade_manager::UpgradeRecord>, Error> {
        upgrade_manager::UpgradeManager::get_upgrade_history(&env)
    }

    /// Get upgrade statistics
    ///
    /// Calculates and returns comprehensive upgrade statistics.
    ///
    /// # Returns
    ///
    /// * `Ok(UpgradeStats)` - Upgrade statistics and analytics
    pub fn get_upgrade_statistics(env: Env) -> Result<upgrade_manager::UpgradeStats, Error> {
        upgrade_manager::UpgradeManager::get_upgrade_statistics(&env)
    }

    /// Validate upgrade compatibility
    ///
    /// Performs comprehensive validation of an upgrade proposal without
    /// executing the upgrade. Useful for testing and validation.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment
    /// * `proposal` - The upgrade proposal to validate
    ///
    /// # Returns
    ///
    /// * `Ok(CompatibilityCheckResult)` - Detailed compatibility analysis
    pub fn validate_upgrade_compatibility(
        env: Env,
        proposal: upgrade_manager::UpgradeProposal,
    ) -> Result<upgrade_manager::CompatibilityCheckResult, Error> {
        upgrade_manager::UpgradeManager::validate_upgrade_compatibility(&env, &proposal)
    }

    /// Test upgrade safety
    ///
    /// Performs dry-run validation of an upgrade proposal without executing.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment
    /// * `proposal` - The upgrade proposal to test
    ///
    /// # Returns
    ///
    /// * `Ok(bool)` - True if upgrade would succeed
    pub fn test_upgrade_safety(
        env: Env,
        proposal: upgrade_manager::UpgradeProposal,
    ) -> Result<bool, Error> {
        upgrade_manager::UpgradeManager::test_upgrade_safety(&env, &proposal)
    }

    // ===== MARKET ANALYTICS FUNCTIONS =====

    /// Get comprehensive market statistics for data analysis and insights
    ///
    /// This function provides detailed statistics about a specific market including
    /// participation metrics, stake distribution, outcome analysis, and performance
    /// indicators. It's essential for market monitoring and user interfaces.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `market_id` - Unique identifier of the market to analyze
    ///
    /// # Returns
    ///
    /// Returns `Result<MarketStatistics, Error>` where:
    /// - `Ok(MarketStatistics)` - Complete market statistics and analytics
    /// - `Err(Error)` - Error if market not found or analysis fails
    ///
    /// # Errors
    ///
    /// This function returns:
    /// - `Error::MarketNotFound` - Market with given ID doesn't exist
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Symbol};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let market_id = Symbol::new(&env, "market_1");
    ///
    /// match PredictifyHybrid::get_market_statistics(env.clone(), market_id) {
    ///     Ok(stats) => {
    ///         println!("Total participants: {}", stats.total_participants);
    ///         println!("Total stake: {}", stats.total_stake);
    ///         println!("Consensus strength: {}%", stats.consensus_strength);
    ///     },
    ///     Err(e) => println!("Analytics unavailable: {:?}", e),
    /// }
    /// ```
    pub fn get_market_statistics(
        env: Env,
        market_id: Symbol,
    ) -> Result<market_analytics::MarketStatistics, Error> {
        market_analytics::MarketAnalyticsManager::get_market_statistics(&env, market_id)
    }

    /// Get voting analytics and participation metrics for a market
    ///
    /// This function provides detailed analysis of voting patterns, participation
    /// trends, and community engagement within a specific market. It's useful
    /// for understanding market dynamics and user behavior.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `market_id` - Unique identifier of the market to analyze
    ///
    /// # Returns
    ///
    /// Returns `Result<VotingAnalytics, Error>` where:
    /// - `Ok(VotingAnalytics)` - Complete voting analytics and metrics
    /// - `Err(Error)` - Error if market not found or analysis fails
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Symbol};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let market_id = Symbol::new(&env, "market_1");
    ///
    /// match PredictifyHybrid::get_voting_analytics(env.clone(), market_id) {
    ///     Ok(analytics) => {
    ///         println!("Total votes: {}", analytics.total_votes);
    ///         println!("Unique voters: {}", analytics.unique_voters);
    ///     },
    ///     Err(e) => println!("Voting analytics unavailable: {:?}", e),
    /// }
    /// ```
    pub fn get_voting_analytics(
        env: Env,
        market_id: Symbol,
    ) -> Result<market_analytics::VotingAnalytics, Error> {
        market_analytics::MarketAnalyticsManager::get_voting_analytics(&env, market_id)
    }

    /// Get oracle performance statistics for a specific oracle provider
    ///
    /// This function provides comprehensive performance metrics for oracle providers,
    /// including accuracy rates, response times, uptime statistics, and reliability
    /// scores. It's essential for oracle monitoring and optimization.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `oracle` - The oracle provider to analyze
    ///
    /// # Returns
    ///
    /// Returns `Result<OraclePerformanceStats, Error>` where:
    /// - `Ok(OraclePerformanceStats)` - Complete oracle performance statistics
    /// - `Err(Error)` - Error if oracle data unavailable
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::Env;
    /// # use predictify_hybrid::{PredictifyHybrid, OracleProvider};
    /// # let env = Env::default();
    ///
    /// match PredictifyHybrid::get_oracle_performance_stats(env.clone(), OracleProvider::Reflector) {
    ///     Ok(stats) => {
    ///         println!("Oracle accuracy: {}%", stats.accuracy_rate);
    ///         println!("Uptime: {}%", stats.uptime_percentage);
    ///         println!("Reliability score: {}", stats.reliability_score);
    ///     },
    ///     Err(e) => println!("Oracle stats unavailable: {:?}", e),
    /// }
    /// ```
    pub fn get_oracle_performance_stats(
        env: Env,
        oracle: OracleProvider,
    ) -> Result<market_analytics::OraclePerformanceStats, Error> {
        market_analytics::MarketAnalyticsManager::get_oracle_performance_stats(&env, oracle)
    }

    /// Get fee analytics and revenue tracking for a specific timeframe
    ///
    /// This function provides comprehensive fee collection analytics including
    /// revenue tracking, fee distribution analysis, and collection efficiency
    /// metrics. It's essential for financial monitoring and optimization.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `timeframe` - The time period for fee analysis
    ///
    /// # Returns
    ///
    /// Returns `Result<FeeAnalytics, Error>` where:
    /// - `Ok(FeeAnalytics)` - Complete fee analytics and revenue data
    /// - `Err(Error)` - Error if fee data unavailable
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::Env;
    /// # use predictify_hybrid::{PredictifyHybrid, TimeFrame};
    /// # let env = Env::default();
    ///
    /// match PredictifyHybrid::get_fee_analytics(env.clone(), TimeFrame::Month) {
    ///     Ok(analytics) => {
    ///         println!("Total fees collected: {}", analytics.total_fees_collected);
    ///         println!("Collection rate: {}%", analytics.fee_collection_rate);
    ///     },
    ///     Err(e) => println!("Fee analytics unavailable: {:?}", e),
    /// }
    /// ```
    pub fn get_fee_analytics(
        env: Env,
        timeframe: market_analytics::TimeFrame,
    ) -> Result<market_analytics::FeeAnalytics, Error> {
        market_analytics::MarketAnalyticsManager::get_fee_analytics(&env, timeframe)
    }

    /// Get dispute analytics and resolution metrics for a market
    ///
    /// This function provides detailed analysis of dispute patterns, resolution
    /// efficiency, and dispute-related metrics for a specific market. It's
    /// essential for understanding dispute dynamics and improving resolution processes.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `market_id` - Unique identifier of the market to analyze
    ///
    /// # Returns
    ///
    /// Returns `Result<DisputeAnalytics, Error>` where:
    /// - `Ok(DisputeAnalytics)` - Complete dispute analytics and metrics
    /// - `Err(Error)` - Error if market not found or analysis fails
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Symbol};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let market_id = Symbol::new(&env, "market_1");
    ///
    /// match PredictifyHybrid::get_dispute_analytics(env.clone(), market_id) {
    ///     Ok(analytics) => {
    ///         println!("Total disputes: {}", analytics.total_disputes);
    ///         println!("Success rate: {}%", analytics.dispute_success_rate);
    ///     },
    ///     Err(e) => println!("Dispute analytics unavailable: {:?}", e),
    /// }
    /// ```
    pub fn get_dispute_analytics(
        env: Env,
        market_id: Symbol,
    ) -> Result<market_analytics::DisputeAnalytics, Error> {
        market_analytics::MarketAnalyticsManager::get_dispute_analytics(&env, market_id)
    }

    /// Get participation metrics for a specific market
    ///
    /// This function provides comprehensive participation analysis including
    /// user engagement, retention rates, and activity patterns for a specific
    /// market. It's essential for understanding user behavior and market health.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `market_id` - Unique identifier of the market to analyze
    ///
    /// # Returns
    ///
    /// Returns `Result<ParticipationMetrics, Error>` where:
    /// - `Ok(ParticipationMetrics)` - Complete participation metrics and analysis
    /// - `Err(Error)` - Error if market not found or analysis fails
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Symbol};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let market_id = Symbol::new(&env, "market_1");
    ///
    /// match PredictifyHybrid::get_participation_metrics(env.clone(), market_id) {
    ///     Ok(metrics) => {
    ///         println!("Total participants: {}", metrics.total_participants);
    ///         println!("Engagement score: {}", metrics.engagement_score);
    ///         println!("Retention rate: {}%", metrics.retention_rate);
    ///     },
    ///     Err(e) => println!("Participation metrics unavailable: {:?}", e),
    /// }
    /// ```
    pub fn get_participation_metrics(
        env: Env,
        market_id: Symbol,
    ) -> Result<market_analytics::ParticipationMetrics, Error> {
        market_analytics::MarketAnalyticsManager::get_participation_metrics(&env, market_id)
    }

    /// Get market comparison analytics for multiple markets
    ///
    /// This function provides comparative analysis across multiple markets,
    /// including performance rankings, comparative metrics, and market insights.
    /// It's essential for understanding market trends and performance patterns.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `markets` - Vector of market identifiers to compare
    ///
    /// # Returns
    ///
    /// Returns `Result<MarketComparisonAnalytics, Error>` where:
    /// - `Ok(MarketComparisonAnalytics)` - Complete comparative analytics
    /// - `Err(Error)` - Error if analysis fails
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Symbol, vec};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let markets = vec![
    /// #     &env,
    /// #     Symbol::new(&env, "market_1"),
    /// #     Symbol::new(&env, "market_2"),
    /// # ];
    ///
    /// match PredictifyHybrid::get_market_comparison_analytics(env.clone(), markets) {
    ///     Ok(comparison) => {
    ///         println!("Total markets: {}", comparison.total_markets);
    ///         println!("Average participation: {}", comparison.average_participation);
    ///         println!("Success rate: {}%", comparison.success_rate);
    ///     },
    ///     Err(e) => println!("Comparison analytics unavailable: {:?}", e),
    /// }
    /// ```
    pub fn get_market_comparison_analytics(
        env: Env,
        markets: Vec<Symbol>,
    ) -> Result<market_analytics::MarketComparisonAnalytics, Error> {
        market_analytics::MarketAnalyticsManager::get_market_comparison_analytics(&env, markets)
    }

    // ===== PERFORMANCE BENCHMARK FUNCTIONS =====

    /// Benchmark gas usage for a specific function with given inputs
    ///
    /// This function measures the gas consumption and execution time for a specific
    /// contract function with provided inputs. It's essential for performance
    /// optimization and gas cost analysis.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `function` - Name of the function to benchmark
    /// * `inputs` - Vector of input parameters for the function
    ///
    /// # Returns
    ///
    /// Returns `Result<BenchmarkResult, Error>` where:
    /// - `Ok(BenchmarkResult)` - Complete benchmark results including gas usage and execution time
    /// - `Err(Error)` - Error if benchmarking fails
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, String, vec};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let inputs = vec![&env, String::from_str(&env, "test_input")];
    ///
    /// match PredictifyHybrid::benchmark_gas_usage(
    ///     env.clone(),
    ///     String::from_str(&env, "create_market"),
    ///     inputs
    /// ) {
    ///     Ok(result) => {
    ///         println!("Gas usage: {}", result.gas_usage);
    ///         println!("Execution time: {}", result.execution_time);
    ///         println!("Performance score: {}", result.performance_score);
    ///     },
    ///     Err(e) => println!("Benchmark failed: {:?}", e),
    /// }
    /// ```
    pub fn benchmark_gas_usage(
        env: Env,
        function: String,
        inputs: Vec<String>,
    ) -> Result<performance_benchmarks::BenchmarkResult, Error> {
        performance_benchmarks::PerformanceBenchmarkManager::benchmark_gas_usage(
            &env, function, inputs,
        )
    }

    /// Benchmark storage usage for a specific operation
    ///
    /// This function measures storage consumption and performance for various
    /// storage operations including read, write, and delete operations.
    /// It's essential for storage optimization and cost analysis.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `operation` - Storage operation configuration to benchmark
    ///
    /// # Returns
    ///
    /// Returns `Result<BenchmarkResult, Error>` where:
    /// - `Ok(BenchmarkResult)` - Complete storage benchmark results
    /// - `Err(Error)` - Error if benchmarking fails
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::Env;
    /// # use predictify_hybrid::{PredictifyHybrid, StorageOperation};
    /// # let env = Env::default();
    /// # let operation = StorageOperation {
    /// #     operation_type: String::from_str(&env, "write"),
    /// #     data_size: 1024,
    /// #     key_count: 10,
    /// #     value_count: 10,
    /// #     operation_count: 100,
    /// # };
    ///
    /// match PredictifyHybrid::benchmark_storage_usage(env.clone(), operation) {
    ///     Ok(result) => {
    ///         println!("Storage usage: {}", result.storage_usage);
    ///         println!("Gas usage: {}", result.gas_usage);
    ///     },
    ///     Err(e) => println!("Storage benchmark failed: {:?}", e),
    /// }
    /// ```
    pub fn benchmark_storage_usage(
        env: Env,
        operation: performance_benchmarks::StorageOperation,
    ) -> Result<performance_benchmarks::BenchmarkResult, Error> {
        performance_benchmarks::PerformanceBenchmarkManager::benchmark_storage_usage(
            &env, operation,
        )
    }

    /// Benchmark oracle call performance for a specific oracle provider
    ///
    /// This function measures the performance characteristics of oracle calls
    /// including response time, gas usage, and reliability metrics.
    /// It's essential for oracle performance monitoring and optimization.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `oracle` - The oracle provider to benchmark
    ///
    /// # Returns
    ///
    /// Returns `Result<BenchmarkResult, Error>` where:
    /// - `Ok(BenchmarkResult)` - Complete oracle performance benchmark results
    /// - `Err(Error)` - Error if benchmarking fails
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::Env;
    /// # use predictify_hybrid::{PredictifyHybrid, OracleProvider};
    /// # let env = Env::default();
    ///
    /// match PredictifyHybrid::benchmark_oracle_call_performance(
    ///     env.clone(),
    ///     OracleProvider::Reflector
    /// ) {
    ///     Ok(result) => {
    ///         println!("Oracle response time: {}", result.execution_time);
    ///         println!("Oracle gas usage: {}", result.gas_usage);
    ///     },
    ///     Err(e) => println!("Oracle benchmark failed: {:?}", e),
    /// }
    /// ```
    pub fn benchmark_oracle_performance(
        env: Env,
        oracle: OracleProvider,
    ) -> Result<performance_benchmarks::BenchmarkResult, Error> {
        performance_benchmarks::PerformanceBenchmarkManager::benchmark_oracle_call_performance(
            &env, oracle,
        )
    }

    /// Benchmark batch operations performance
    ///
    /// This function measures the performance of batch operations including
    /// gas efficiency, execution time, and throughput characteristics.
    /// It's essential for batch operation optimization and scalability analysis.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `operations` - Vector of batch operations to benchmark
    ///
    /// # Returns
    ///
    /// Returns `Result<BenchmarkResult, Error>` where:
    /// - `Ok(BenchmarkResult)` - Complete batch operation benchmark results
    /// - `Err(Error)` - Error if benchmarking fails
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, vec};
    /// # use predictify_hybrid::{PredictifyHybrid, BatchOperation};
    /// # let env = Env::default();
    /// # let operations = vec![
    /// #     &env,
    /// #     BatchOperation {
    /// #         operation_type: String::from_str(&env, "batch_vote"),
    /// #         batch_size: 100,
    /// #         operation_count: 10,
    /// #         data_size: 1024,
    /// #     }
    /// # ];
    ///
    /// match PredictifyHybrid::benchmark_batch_operations(env.clone(), operations) {
    ///     Ok(result) => {
    ///         println!("Batch execution time: {}", result.execution_time);
    ///         println!("Batch gas usage: {}", result.gas_usage);
    ///     },
    ///     Err(e) => println!("Batch benchmark failed: {:?}", e),
    /// }
    /// ```
    pub fn benchmark_batch_operations(
        env: Env,
        operations: Vec<performance_benchmarks::BatchOperation>,
    ) -> Result<performance_benchmarks::BenchmarkResult, Error> {
        performance_benchmarks::PerformanceBenchmarkManager::benchmark_batch_operations(
            &env, operations,
        )
    }

    /// Benchmark scalability with large markets and user counts
    ///
    /// This function measures the contract's performance under high load
    /// scenarios with large numbers of markets and users. It's essential
    /// for scalability testing and performance validation.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `market_size` - Number of markets to simulate
    /// * `user_count` - Number of users to simulate
    ///
    /// # Returns
    ///
    /// Returns `Result<BenchmarkResult, Error>` where:
    /// - `Ok(BenchmarkResult)` - Complete scalability benchmark results
    /// - `Err(Error)` - Error if benchmarking fails
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::Env;
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    ///
    /// match PredictifyHybrid::benchmark_scalability(env.clone(), 1000, 10000) {
    ///     Ok(result) => {
    ///         println!("Scalability test completed");
    ///         println!("Total gas usage: {}", result.gas_usage);
    ///         println!("Total execution time: {}", result.execution_time);
    ///     },
    ///     Err(e) => println!("Scalability benchmark failed: {:?}", e),
    /// }
    /// ```
    pub fn benchmark_scalability(
        env: Env,
        market_size: u32,
        user_count: u32,
    ) -> Result<performance_benchmarks::BenchmarkResult, Error> {
        performance_benchmarks::PerformanceBenchmarkManager::benchmark_scalability(
            &env,
            market_size,
            user_count,
        )
    }

    /// Generate comprehensive performance report
    ///
    /// This function creates a detailed performance report including metrics,
    /// recommendations, and optimization opportunities based on benchmark results.
    /// It's essential for performance analysis and optimization planning.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `benchmark_suite` - The benchmark suite to generate report for
    ///
    /// # Returns
    ///
    /// Returns `Result<PerformanceReport, Error>` where:
    /// - `Ok(PerformanceReport)` - Complete performance report with analysis
    /// - `Err(Error)` - Error if report generation fails
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::Env;
    /// # use predictify_hybrid::{PredictifyHybrid, PerformanceBenchmarkSuite};
    /// # let env = Env::default();
    /// # let suite = PerformanceBenchmarkSuite::default(); // Placeholder
    ///
    /// match PredictifyHybrid::generate_performance_report(env.clone(), suite) {
    ///     Ok(report) => {
    ///         println!("Performance report generated");
    ///         println!("Overall score: {}", report.performance_metrics.overall_performance_score);
    ///         println!("Recommendations: {}", report.recommendations.len());
    ///     },
    ///     Err(e) => println!("Report generation failed: {:?}", e),
    /// }
    /// ```
    pub fn generate_performance_report(
        env: Env,
        benchmark_suite: performance_benchmarks::PerformanceBenchmarkSuite,
    ) -> Result<performance_benchmarks::PerformanceReport, Error> {
        performance_benchmarks::PerformanceBenchmarkManager::generate_performance_report(
            &env,
            benchmark_suite,
        )
    }

    /// Validate performance against thresholds
    ///
    /// This function validates performance metrics against predefined thresholds
    /// to ensure the contract meets performance requirements. It's essential
    /// for performance validation and quality assurance.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `metrics` - Performance metrics to validate
    /// * `thresholds` - Performance thresholds to validate against
    ///
    /// # Returns
    ///
    /// Returns `Result<bool, Error>` where:
    /// - `Ok(true)` - Performance meets all thresholds
    /// - `Ok(false)` - Performance does not meet thresholds
    /// - `Err(Error)` - Error if validation fails
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::Env;
    /// # use predictify_hybrid::{PredictifyHybrid, PerformanceMetrics, PerformanceThresholds};
    /// # let env = Env::default();
    /// # let metrics = PerformanceMetrics::default(); // Placeholder
    /// # let thresholds = PerformanceThresholds::default(); // Placeholder
    ///
    /// match PredictifyHybrid::validate_performance_thresholds(env.clone(), metrics, thresholds) {
    ///     Ok(true) => println!("Performance meets all thresholds"),
    ///     Ok(false) => println!("Performance does not meet thresholds"),
    ///     Err(e) => println!("Validation failed: {:?}", e),
    /// }
    /// ```
    pub fn validate_performance_thresholds(
        env: Env,
        metrics: performance_benchmarks::PerformanceMetrics,
        thresholds: performance_benchmarks::PerformanceThresholds,
    ) -> Result<bool, Error> {
        performance_benchmarks::PerformanceBenchmarkManager::validate_performance_thresholds(
            &env, metrics, thresholds,
        )
    }
    /// Get platform-wide statistics
    pub fn get_platform_statistics(env: Env) -> PlatformStatistics {
        statistics::StatisticsManager::get_platform_stats(&env)
    }

    /// Get user-specific statistics
    pub fn get_user_statistics(env: Env, user: Address) -> UserStatistics {
        statistics::StatisticsManager::get_user_stats(&env, &user)
    }
}

mod test;
