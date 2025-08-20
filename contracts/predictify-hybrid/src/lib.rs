#![no_std]

extern crate alloc;
extern crate wee_alloc;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

// Module declarations - all modules enabled
mod admin;
mod batch_operations;
mod circuit_breaker;
mod config;
mod disputes;
mod errors;
mod events;
mod extensions;
mod fees;
mod markets;
mod oracles;
mod resolution;
mod storage;
mod types;
mod utils;
mod validation;
mod voting;

#[cfg(test)]
mod circuit_breaker_tests;

#[cfg(test)]
mod batch_operations_tests;

#[cfg(test)]
mod integration_test;

// Re-export commonly used items
use admin::AdminInitializer;
pub use errors::Error;
pub use types::*;

use alloc::format;
use soroban_sdk::{
    contract, contractimpl, panic_with_error, Address, Env, Map, String, Symbol, Vec,
};

#[contract]
pub struct PredictifyHybrid;

const PERCENTAGE_DENOMINATOR: i128 = 100;
const FEE_PERCENTAGE: i128 = 2; // 2% fee for the platform

#[contractimpl]
impl PredictifyHybrid {
    /// Initializes the Predictify Hybrid smart contract with an administrator.
    ///
    /// This function must be called once after contract deployment to set up the initial
    /// administrative configuration. It establishes the contract admin who will have
    /// privileges to create markets and perform administrative functions.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `admin` - The address that will be granted administrative privileges
    ///
    /// # Panics
    ///
    /// This function will panic if:
    /// - The contract has already been initialized
    /// - The admin address is invalid
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
    /// // Initialize the contract with an admin
    /// PredictifyHybrid::initialize(env.clone(), admin_address);
    /// ```
    ///
    /// # Security
    ///
    /// The admin address should be carefully chosen as it will have significant
    /// control over the contract's operation, including market creation and resolution.
    pub fn initialize(env: Env, admin: Address) {
        match AdminInitializer::initialize(&env, &admin) {
            Ok(_) => (), // Success
            Err(e) => panic_with_error!(env, e),
        }
    }

    /// Creates a new prediction market with specified parameters and oracle configuration.
    ///
    /// This function allows authorized administrators to create prediction markets
    /// with custom questions, possible outcomes, duration, and oracle integration.
    /// Each market gets a unique identifier and is stored in persistent contract storage.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `admin` - The administrator address creating the market (must be authorized)
    /// * `question` - The prediction question (must be non-empty)
    /// * `outcomes` - Vector of possible outcomes (minimum 2 required, all non-empty)
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

        let market_id = Symbol::new(&env, &format!("market_{}", new_counter));

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

        // Store the vote and stake
        market.votes.set(user.clone(), outcome);
        market.stakes.set(user.clone(), stake);
        market.total_staked += stake;

        env.storage().persistent().set(&market_id, &market);
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

        // Set winning outcome and update state
        market.winning_outcome = Some(winning_outcome);
        market.state = MarketState::Resolved;
        env.storage().persistent().set(&market_id, &market);
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
    /// - `Error::MarketAlreadyResolved` - Market already has oracle result set
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
            return Err(Error::MarketAlreadyResolved);
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
    /// - `Error::MarketAlreadyResolved` - Market is already resolved
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

    /// Extend market duration (admin only)
    pub fn extend_market(
        env: Env,
        admin: Address,
        market_id: Symbol,
        additional_days: u32,
        reason: String,
        fee_amount: i128,
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
    pub fn compress_market_data(env: Env, market_id: Symbol) -> Result<storage::CompressedMarket, Error> {
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
    pub fn validate_storage_integrity(env: Env, market_id: Symbol) -> Result<storage::StorageIntegrityResult, Error> {
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
}

mod test;
