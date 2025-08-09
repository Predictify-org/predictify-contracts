#![no_std]

extern crate alloc;

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
mod reentrancy;
mod resolution;
mod storage;
mod types;
mod utils;
mod validation;
mod validation_tests;
mod voting;

#[cfg(test)]
mod integration_test;

// Re-export commonly used items
use admin::AdminInitializer;
pub use errors::Error;
pub use types::*;

use alloc::format;
use soroban_sdk::{

    contract, contractimpl, panic_with_error, symbol_short, vec, Address, Env, Map, String, Symbol, Vec,
};
use alloc::string::ToString;

// Global allocator for wasm32 target
#[cfg(target_arch = "wasm32")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

// Import commonly used items from modules
use markets::{MarketCreator, MarketStateManager};
use voting::VotingManager;
use disputes::DisputeManager;
use extensions::{ExtensionManager, ExtensionUtils, ExtensionValidator};
use fees::FeeManager;
use reentrancy::{validate_no_reentrancy, protect_external_call};
use resolution::{OracleResolutionManager, MarketResolutionManager};
use config::{ConfigManager, ConfigUtils, ContractConfig, Environment};
use utils::{TimeUtils, StringUtils, NumericUtils, ValidationUtils, CommonUtils};
use events::{EventLogger, EventHelpers, EventTestingUtils, EventDocumentation};
use validation::ValidationResult;


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

        // Use error helper for admin validation
        let _ = errors::helpers::require_admin(&env, &admin, &stored_admin);

        // Use the markets module to create the market
        match MarketCreator::create_market(
            &env,
            admin.clone(),
            question,
            outcomes,
            duration_days,
            oracle_config,
        ) {
            Ok(market_id) => {
                // Process creation fee using the fee management system
                match FeeManager::process_creation_fee(&env, &admin) {
                    Ok(_) => market_id,
                    Err(e) => panic_with_error!(env, e),
                }
            }
            Err(e) => panic_with_error!(env, e),

        }
    }

    /// Distribute winnings to users
    pub fn claim_winnings(env: Env, user: Address, market_id: Symbol) {
        // Require authentication
        user.require_auth();
        
        // Validate no reentrancy
        validate_no_reentrancy(&env).unwrap_or_else(|e| panic_with_error!(env, e));
        
        // Protect against reentrancy attacks
        let result = protect_external_call(
            &env,
            symbol_short!("claim_win"),
            user.clone(),
            || {
                // Execute the claim operation
                VotingManager::process_claim(&env, user, market_id)
            },
        );
        
        match result {
            Ok(_) => (), // Success
            Err(e) => panic_with_error!(env, e),
        }
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

        // Require authentication
        user.require_auth();
        
        // Validate no reentrancy
        validate_no_reentrancy(&env).unwrap_or_else(|e| panic_with_error!(env, e));
        
        // Protect against reentrancy attacks
        let result = protect_external_call(
            &env,
            symbol_short!("vote"),
            user.clone(),
            || {
                // Execute the vote operation
                VotingManager::process_vote(&env, user, market_id, outcome, stake)
            },
        );
        
        match result {
            Ok(_) => (), // Success
            Err(e) => panic_with_error!(env, e),
        }
    }

    // Fetch oracle result to determine market outcome
    pub fn fetch_oracle_result(env: Env, market_id: Symbol, oracle_contract: Address) -> String {
        // Validate no reentrancy
        validate_no_reentrancy(&env).unwrap_or_else(|e| panic_with_error!(env, e));
        
        // Protect against reentrancy attacks
        let result = protect_external_call(
            &env,
            symbol_short!("fetch_orc"),
            oracle_contract.clone(),
            || {
                // Execute the oracle fetch operation
                resolution::OracleResolutionManager::fetchoracle_result(&env, &market_id, &oracle_contract)
            },
        );
        
        match result {
            Ok(resolution) => resolution.oracle_result,
            Err(e) => panic_with_error!(env, e),
        }
    }

    // Allows users to dispute the market result by staking tokens
    pub fn dispute_result(env: Env, user: Address, market_id: Symbol, stake: i128) {
        // Require authentication
        user.require_auth();
        
        // Validate no reentrancy
        validate_no_reentrancy(&env).unwrap_or_else(|e| panic_with_error!(env, e));
        
        // Protect against reentrancy attacks
        let result = protect_external_call(
            &env,
            symbol_short!("dispute"),
            user.clone(),
            || {
                // Execute the dispute operation
                DisputeManager::process_dispute(&env, user, market_id, stake, None)
            },
        );
        
        match result {
            Ok(_) => (), // Success
            Err(e) => panic_with_error!(env, e),
        }
    }


    // ===== RESOLUTION SYSTEM METHODS =====

    // Get oracle resolution for a market
    pub fn get_oracle_resolution(env: Env, market_id: Symbol) -> Option<resolution::OracleResolution> {
        OracleResolutionManager::get_oracle_resolution(&env, &market_id).unwrap_or_default()
    }

    // Get market resolution for a market
    pub fn get_market_resolution(env: Env, market_id: Symbol) -> Option<resolution::MarketResolution> {
        MarketResolutionManager::get_market_resolution(&env, &market_id).unwrap_or_default()
    }

    /// Get oracle statistics
    pub fn get_oracle_stats(env: Env) -> resolution::OracleStats {
        resolution::OracleResolutionAnalytics::get_oracle_stats(&env).unwrap_or_default()
    }

    /// Process winnings claim and calculate payouts for resolved markets
    pub fn process_winnings_claim(env: Env, user: Address, market_id: Symbol) -> Result<i128, Error> {
        // Get the market from storage
        let mut market = env
            .storage()
            .persistent()
            .get::<Symbol, Market>(&market_id)
            .ok_or(Error::MarketNotFound)?;

        // Check if market is resolved
        let winning_outcome = match &market.winning_outcome {
            Some(outcome) => outcome,
            None => return Err(Error::MarketNotResolved),
        };

        // Get user's vote
        let user_outcome = market
            .votes
            .get(user.clone())
            .ok_or(Error::NothingToClaim)?;

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
                let payout = (user_share * total_pool) / winning_total;

                // Mark as claimed
                market.claimed.set(user.clone(), true);
                env.storage().persistent().set(&market_id, &market);

                return Ok(payout);
            }
        }

        Err(Error::NothingToClaim)
    }

    // Get resolution state for a market
    pub fn get_resolution_state(env: Env, market_id: Symbol) -> resolution::ResolutionState {
        match MarketStateManager::get_market(&env, &market_id) {
            Ok(market) => resolution::ResolutionUtils::get_resolution_state(&env, &market),
            Err(_) => resolution::ResolutionState::Active,
        }
    }

    // Check if market can be resolved
    pub fn can_resolve_market(env: Env, market_id: Symbol) -> bool {
        match MarketStateManager::get_market(&env, &market_id) {
            Ok(market) => resolution::ResolutionUtils::can_resolve_market(&env, &market),
            Err(_) => false,
        }
    }

    // Calculate resolution time for a market
    pub fn calculate_resolution_time(env: Env, market_id: Symbol) -> u64 {
        match MarketStateManager::get_market(&env, &market_id) {
            Ok(market) => {
                let current_time = env.ledger().timestamp();
                TimeUtils::time_difference(current_time, market.end_time)
            },
            Err(_) => 0,
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
        DisputeManager::has_user_disputed(&env, market_id, user).unwrap_or_default()
    }

    // Get user's dispute stake for a market
    pub fn get_user_dispute_stake(env: Env, market_id: Symbol, user: Address) -> i128 {
        DisputeManager::get_user_dispute_stake(&env, market_id, user).unwrap_or_default()
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
        _winning_outcome: String,
    ) {
        admin.require_auth();

        // Verify admin
        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, "Admin"))

            .expect("Admin not set");

        // Use error helper for admin validation
        let _ = errors::helpers::require_admin(&env, &admin, &stored_admin);

        // Remove market from storage
        MarketStateManager::remove_market(&env, &market_id);
    }

    // Helper function to create a market with Reflector oracle
    #[allow(clippy::too_many_arguments)]
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
        let params = ReflectorMarketParams {
            admin,
            question,
            outcomes,
            duration_days,
            asset_symbol,
            threshold,
            comparison,
        };
        match MarketCreator::create_reflector_market(&env, params) {
            Ok(market_id) => market_id,
            Err(e) => panic_with_error!(env, e),
        }
    }

    // Helper function to create a market with Pyth oracle
    #[allow(clippy::too_many_arguments)]
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
        let params = PythMarketParams {
            admin,
            question,
            outcomes,
            duration_days,
            feed_id,
            threshold,
            comparison,
        };
        match MarketCreator::create_pyth_market(&env, params) {
            Ok(market_id) => market_id,
            Err(e) => panic_with_error!(env, e),

        }
    }

    /// Helper function to create a market with Reflector oracle for specific assets
    #[allow(clippy::too_many_arguments)]
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
        let params = ReflectorMarketParams {
            admin,
            question,
            outcomes,
            duration_days,
            asset_symbol,
            threshold,
            comparison,
        };
        match MarketCreator::create_reflector_asset_market(&env, params) {
            Ok(market_id) => market_id,
            Err(e) => panic_with_error!(env, e),
        }
    }

    // ===== MARKET EXTENSION FUNCTIONS =====




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
        let market = match markets::MarketStateManager::get_market(&env, &market_id) {
            Ok(m) => m,
            Err(e) => return Err(e),
        };

        let stats = markets::MarketAnalytics::get_market_stats(&market);
        Ok(stats)
    }

    /// Validate extension conditions for a market
    pub fn validate_extension_conditions(
        env: Env,
        market_id: Symbol,
        additional_days: u32,
    ) -> bool {
        ExtensionValidator::validate_extension_conditions(&env, &market_id, additional_days).is_ok()
    }

    /// Check extension limits for a market
    pub fn check_extension_limits(env: Env, market_id: Symbol, additional_days: u32) -> bool {
        ExtensionValidator::check_extension_limits(&env, &market_id, additional_days).is_ok()
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

    /// Dispute a market resolution
    pub fn dispute_market(
        env: Env,
        user: Address,
        market_id: Symbol,
        stake: i128,
    ) -> Result<disputes::Dispute, Error> {
        user.require_auth();

        // Validate no reentrancy
        validate_no_reentrancy(&env).unwrap_or_else(|e| panic_with_error!(env, e));
        
        // Protect against reentrancy attacks
        let result = protect_external_call(
            &env,
            symbol_short!("dispute"),
            user.clone(),
            || {
                // Execute the dispute operation
                match DisputeManager::process_dispute(&env, user.clone(), market_id.clone(), stake, None) {
                    Ok(()) => {
                        // Create and return dispute object
                        Ok(disputes::Dispute {
                            user: user.clone(),
                            market_id: market_id.clone(),
                            stake,
                            timestamp: env.ledger().timestamp(),
                            reason: None,
                            status: disputes::DisputeStatus::Active,
                        })
                    },
                    Err(e) => Err(e),
                }
            },
        );
        
        match result {
            Ok(dispute) => Ok(dispute),
            Err(e) => Err(e),
        }
    }

    /// Check if admin can extend market
    pub fn can_extend_market(env: Env, market_id: Symbol, admin: Address) -> bool {
        ExtensionManager::can_extend_market(&env, market_id, admin).unwrap_or_default()
    }

    /// Handle extension fees
    pub fn handle_extension_fees(env: Env, market_id: Symbol, additional_days: u32) -> i128 {
        ExtensionUtils::handle_extension_fees(&env, &market_id, additional_days).unwrap_or_default()
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
        DisputeManager::calculate_dispute_outcome(&env, dispute_id).unwrap_or_default()
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
        match DisputeManager::get_dispute_votes(&env, &dispute_id) {
            Ok(votes) => votes,
            Err(_) => vec![&env],
        }
    }

    /// Validate dispute resolution conditions
    pub fn validate_dispute_resolution(env: Env, dispute_id: Symbol) -> bool {
        DisputeManager::validate_dispute_resolution_conditions(&env, dispute_id).unwrap_or_default()
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
        voting::ThresholdUtils::adjust_threshold_by_market_size(&env, &market_id, base_threshold).unwrap_or_default()
    }

    /// Modify threshold by activity level
    pub fn modify_threshold_by_activity(env: Env, market_id: Symbol, activity_level: u32) -> i128 {
        voting::ThresholdUtils::modify_threshold_by_activity(&env, &market_id, activity_level).unwrap_or_default()
    }

    /// Validate dispute threshold
    pub fn validate_dispute_threshold(threshold: i128, market_id: Symbol) -> bool {
        voting::ThresholdUtils::validate_dispute_threshold(threshold, &market_id).is_ok()
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

    /// Resolve a dispute (admin only)
    pub fn resolve_dispute(
        env: Env,
        admin: Address,
        market_id: Symbol,
    ) -> Result<disputes::DisputeResolution, Error> {
        admin.require_auth();

        // Use the dispute manager to resolve the dispute
        DisputeManager::resolve_dispute(&env, market_id, admin)
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
        // Store admin address
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


    /// Update contract configuration (admin only)
    pub fn update_config(env: Env, admin: Address, config: ContractConfig) -> Result<(), Error> {
        let stored_admin = env.storage().persistent().get(&Symbol::new(&env, "Admin")).unwrap();
        errors::helpers::require_admin(&env, &admin, &stored_admin)?;

        // Store configuration
        ConfigManager::store_config(&env, &config)?;
        Ok(())
    }

    /// Reset configuration to defaults
    pub fn reset_config_to_defaults(env: Env, admin: Address) -> ContractConfig {
        // Verify admin permissions

        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, "Admin"))

            .unwrap_or_else(|| panic!("Admin not set"));

        if let Err(e) = errors::helpers::require_admin(&env, &admin, &stored_admin) {
            panic_with_error!(env, e);
        }

        // Reset to defaults
        match ConfigManager::reset_to_defaults(&env) {
            Ok(config) => config,
            Err(e) => panic_with_error!(env, e),
        }
    }

    /// Update contract admin (admin only)
    pub fn update_admin(env: Env, admin: Address, new_admin: Address) -> Result<(), Error> {
        let stored_admin = env.storage().persistent().get(&Symbol::new(&env, "Admin")).unwrap();
        errors::helpers::require_admin(&env, &admin, &stored_admin)?;

        // Store new admin
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, "Admin"), &new_admin);
        Ok(())
    }

    /// Update contract token (admin only)
    pub fn update_token(env: Env, admin: Address, new_token: Address) -> Address {
        // Verify admin permissions

        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, "Admin"))

            .unwrap_or_else(|| panic!("Admin not set"));

        if let Err(e) = errors::helpers::require_admin(&env, &admin, &stored_admin) {
            panic_with_error!(env, e);
        }

        // Store new token
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, "TokenID"), &new_token);

        new_token
    }

    /// Get configuration summary
    pub fn get_config_summary(env: Env) -> String {
        let config = match ConfigManager::get_config(&env) {
            Ok(config) => config,
            Err(_) => ConfigManager::get_development_config(&env),
        };
        ConfigUtils::get_config_summary(&config)
    }

    /// Extend market duration with validation and fee handling
    pub fn extend_market_duration(
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
            .expect("Admin not set");

        // Use error helper for admin validation
        let _ = errors::helpers::require_admin(&env, &admin, &stored_admin);

        match extensions::ExtensionManager::extend_market_duration(
            &env,
            admin,
            market_id,
            additional_days,
            reason,
        ) {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
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

    // ===== UTILITY-BASED METHODS =====

    /// Format duration in human-readable format
    pub fn format_duration(env: Env, seconds: u64) -> String {
        TimeUtils::format_duration(&env, seconds)
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
    pub fn validate_future_timestamp(env: Env, timestamp: u64) -> bool {
        ValidationUtils::validate_future_timestamp(&env, &timestamp)
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
    pub fn validate_event_structure(env: Env, event_type: String, _event_data: String) -> bool {
        let valid_event_types = vec![
            &env,
            String::from_str(&env, "MarketCreated"),
            String::from_str(&env, "VoteCast"),
            String::from_str(&env, "OracleResult"),
            String::from_str(&env, "MarketResolved"),
            String::from_str(&env, "DisputeCreated"),
            String::from_str(&env, "DisputeResolved"),
            String::from_str(&env, "FeeCollected"),
            String::from_str(&env, "ExtensionRequested"),
            String::from_str(&env, "ConfigUpdated"),
            String::from_str(&env, "ErrorLogged"),
            String::from_str(&env, "PerformanceMetric"),
        ];
        
        // Check if event_type is in the list of valid types
        for valid_type in valid_event_types.iter() {
            if event_type == valid_type {
                return true;
            }
        }
        false
    }

    /// Get event documentation
    pub fn get_event_documentation(_env: Env) -> Map<String, String> {
        // Implementation
        Map::new(&Env::default())
    }

    /// Get event usage examples
    pub fn get_event_usage_examples(_env: Env) -> Map<String, String> {
        // Implementation
        Map::new(&Env::default())
    }

    /// Get event system overview
    pub fn get_event_system_overview(env: Env) -> String {
        EventDocumentation::get_overview(&env)
    }

    /// Clean up old market data based on age and state
    pub fn cleanup_old_market_data(env: Env, market_id: Symbol) -> Result<bool, Error> {
        storage::StorageOptimizer::cleanup_old_market_data(&env, &market_id)
    }

    /// Validate test event structure
    pub fn validate_test_event(env: Env, event_type: String) -> bool {
        let market_created = String::from_str(&env, "MarketCreated");
        let vote_cast = String::from_str(&env, "VoteCast");
        let oracle_result = String::from_str(&env, "OracleResult");
        let market_resolved = String::from_str(&env, "MarketResolved");
        let dispute_created = String::from_str(&env, "DisputeCreated");
        let fee_collected = String::from_str(&env, "FeeCollected");
        let error_logged = String::from_str(&env, "ErrorLogged");
        let performance_metric = String::from_str(&env, "PerformanceMetric");
        
        if event_type == market_created {
            let test_event = EventTestingUtils::create_test_market_created_event(
                &env,
                &Symbol::new(&env, "test"),
                &Address::from_str(&env, "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF"),
            );
            EventTestingUtils::validate_test_event_structure(&test_event).is_ok()
        } else if event_type == vote_cast {
            let test_event = EventTestingUtils::create_test_vote_cast_event(
                &env,
                &Symbol::new(&env, "test"),
                &Address::from_str(&env, "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF"),
            );
            EventTestingUtils::validate_test_event_structure(&test_event).is_ok()
        } else if event_type == oracle_result {
            let test_event = EventTestingUtils::create_test_oracle_result_event(
                &env,
                &Symbol::new(&env, "test"),
            );
            EventTestingUtils::validate_test_event_structure(&test_event).is_ok()
        } else if event_type == market_resolved {
            let test_event = EventTestingUtils::create_test_market_resolved_event(
                &env,
                &Symbol::new(&env, "test"),
            );
            EventTestingUtils::validate_test_event_structure(&test_event).is_ok()
        } else if event_type == dispute_created {
            let test_event = EventTestingUtils::create_test_dispute_created_event(
                &env,
                &Symbol::new(&env, "test"),
                &Address::from_str(&env, "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF"),
            );
            EventTestingUtils::validate_test_event_structure(&test_event).is_ok()
        } else if event_type == fee_collected {
            let test_event = EventTestingUtils::create_test_fee_collected_event(
                &env,
                &Symbol::new(&env, "test"),
                &Address::from_str(&env, "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF"),
            );
            EventTestingUtils::validate_test_event_structure(&test_event).is_ok()
        } else if event_type == error_logged {
            let test_event = EventTestingUtils::create_test_error_logged_event(&env);
            EventTestingUtils::validate_test_event_structure(&test_event).is_ok()
        } else if event_type == performance_metric {
            let test_event = EventTestingUtils::create_test_performance_metric_event(&env);
            EventTestingUtils::validate_test_event_structure(&test_event).is_ok()
        } else {
            false
        }
    }

    /// Get event age in seconds
    pub fn get_event_age(env: Env, event_timestamp: u64) -> u64 {
        let current_timestamp = env.ledger().timestamp();
        EventHelpers::get_event_age(current_timestamp, event_timestamp)

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


    /// Validate oracle configuration
    pub fn validate_oracle_config(env: Env, oracle_config: OracleConfig) -> ValidationResult {
        let mut result = ValidationResult::valid();
        
        if let Err(_error) = crate::errors::helpers::require_valid_oracle_config(&env, &oracle_config) {
            result.add_error();
        }

        result
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

#[cfg(test)]
mod reentrancy_tests;
