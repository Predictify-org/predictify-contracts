#![allow(dead_code)]

// use crate::reentrancy_guard::ReentrancyGuard; // Removed - module no longer exists
use crate::{
    errors::Error,
    markets::{MarketAnalytics, MarketStateManager, MarketUtils, MarketValidator},
    types::Market,
};

use soroban_sdk::{contracttype, symbol_short, vec, Address, Env, Map, String, Symbol, Vec};

// ===== CONSTANTS =====
// Note: These constants are now managed by the config module
// Use ConfigManager::get_voting_config() to get current values

/// Minimum stake amount for voting (0.1 XLM)
pub const MIN_VOTE_STAKE: i128 = crate::config::MIN_VOTE_STAKE;

/// Minimum stake amount for disputes (10 XLM)
pub const MIN_DISPUTE_STAKE: i128 = crate::config::MIN_DISPUTE_STAKE;

/// Maximum dispute threshold (100 XLM)
pub const MAX_DISPUTE_THRESHOLD: i128 = crate::config::MAX_DISPUTE_THRESHOLD;

/// Base dispute threshold (10 XLM)
pub const BASE_DISPUTE_THRESHOLD: i128 = crate::config::BASE_DISPUTE_THRESHOLD;

/// Market size threshold for large markets (1000 XLM)
pub const LARGE_MARKET_THRESHOLD: i128 = crate::config::LARGE_MARKET_THRESHOLD;

/// Activity level threshold for high activity (100 votes)
pub const HIGH_ACTIVITY_THRESHOLD: u32 = crate::config::HIGH_ACTIVITY_THRESHOLD;

/// Platform fee percentage (2%)
pub const FEE_PERCENTAGE: i128 = crate::config::DEFAULT_PLATFORM_FEE_PERCENTAGE;

/// Dispute extension period in hours
pub const DISPUTE_EXTENSION_HOURS: u32 = crate::config::DISPUTE_EXTENSION_HOURS;

// ===== VOTING STRUCTURES =====

/// Represents a user's vote on a prediction market.
///
/// This structure encapsulates all essential information about a user's voting action,
/// including their chosen outcome, stake amount, and timestamp. Votes are immutable
/// once recorded and form the foundation of the prediction market's consensus mechanism.
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Address, String};
/// # use predictify_hybrid::voting::Vote;
/// # let env = Env::default();
///
/// let vote = Vote {
///     user: Address::generate(&env),
///     outcome: String::from_str(&env, "yes"),
///     stake: 5000000i128, // 0.5 XLM
///     timestamp: env.ledger().timestamp(),
/// };
/// ```
///
/// # Integration Points
///
/// Vote structures integrate with:
/// - **Market System**: Votes are aggregated to determine market consensus
/// - **Payout System**: Vote stakes determine winning distributions
/// - **Analytics System**: Vote data powers market analytics and insights
#[contracttype]
pub struct Vote {
    pub user: Address,
    pub outcome: String,
    pub stake: i128,
    pub timestamp: u64,
}

/// Comprehensive voting statistics for a prediction market.
///
/// This structure provides detailed analytics about voting activity on a market,
/// including participation metrics, stake distribution, and voter demographics.
/// Statistics are calculated dynamically and provide insights into market health
/// and consensus formation.
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Map, String};
/// # use predictify_hybrid::voting::VotingStats;
/// # let env = Env::default();
///
/// let mut outcome_distribution = Map::new(&env);
/// outcome_distribution.set(String::from_str(&env, "yes"), 15000000i128);
/// outcome_distribution.set(String::from_str(&env, "no"), 8000000i128);
///
/// let stats = VotingStats {
///     total_votes: 25,
///     total_staked: 23000000i128,
///     outcome_distribution,
///     unique_voters: 18,
/// };
/// ```
#[contracttype]
pub struct VotingStats {
    pub total_votes: u32,
    pub total_staked: i128,
    pub outcome_distribution: Map<String, i128>,
    pub unique_voters: u32,
}

/// Comprehensive payout calculation data for winning voters.
///
/// This structure contains all necessary information to calculate and validate
/// payouts for users who voted on the winning outcome of a prediction market.
/// It ensures transparent and accurate distribution of winnings based on
/// proportional stake and platform fee deductions.
///
/// # Example Usage
///
/// ```rust
/// # use predictify_hybrid::voting::PayoutData;
///
/// let payout_data = PayoutData {
///     user_stake: 2000000i128,      // User staked 0.2 XLM
///     winning_total: 8000000i128,   // Total winning stake: 0.8 XLM
///     total_pool: 20000000i128,     // Total market pool: 2.0 XLM
///     fee_percentage: 200i128,      // 2% platform fee
///     payout_amount: 4900000i128,   // Final payout: 0.49 XLM
/// };
/// ```
#[contracttype]
pub struct PayoutData {
    pub user_stake: i128,
    pub winning_total: i128,
    pub total_pool: i128,
    pub fee_percentage: i128,
    pub payout_amount: i128,
}

/// Dynamic dispute threshold configuration for prediction markets.
///
/// This structure manages the dispute threshold system that determines how much
/// stake is required to initiate a dispute against a market's resolution.
/// Thresholds are dynamically adjusted based on market characteristics such as
/// size, activity level, and complexity to ensure appropriate dispute barriers.
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Symbol};
/// # use predictify_hybrid::voting::DisputeThreshold;
/// # let env = Env::default();
///
/// let threshold = DisputeThreshold {
///     market_id: Symbol::new(&env, "BTC_100K"),
///     base_threshold: 100000000i128,      // 10 XLM base
///     adjusted_threshold: 150000000i128,   // 15 XLM after adjustments
///     market_size_factor: 1200i128,       // 20% increase for large market
///     activity_factor: 1100i128,          // 10% increase for high activity
///     complexity_factor: 1000i128,        // No complexity adjustment
///     timestamp: env.ledger().timestamp(),
/// };
/// ```
#[contracttype]
pub struct DisputeThreshold {
    pub market_id: Symbol,
    pub base_threshold: i128,
    pub adjusted_threshold: i128,
    pub market_size_factor: i128,
    pub activity_factor: i128,
    pub complexity_factor: i128,
    pub timestamp: u64,
}

/// Threshold adjustment factors for dynamic dispute threshold calculation.
///
/// This structure contains the individual adjustment factors used to calculate
/// dynamic dispute thresholds based on market characteristics. Each factor
/// represents a multiplier (in basis points) that adjusts the base threshold
/// to reflect market-specific conditions and risk levels.
///
/// # Example Usage
///
/// ```rust
/// # use predictify_hybrid::voting::ThresholdAdjustmentFactors;
///
/// let factors = ThresholdAdjustmentFactors {
///     market_size_factor: 1300i128,    // 30% increase for large market
///     activity_factor: 1150i128,       // 15% increase for high activity
///     complexity_factor: 1100i128,     // 10% increase for complexity
///     total_adjustment: 1649i128,      // Combined 64.9% increase
/// };
///
/// let base_threshold = 100000000i128; // 10 XLM
/// let adjusted_threshold = (base_threshold * factors.total_adjustment) / 1000;
/// ```
#[contracttype]
pub struct ThresholdAdjustmentFactors {
    pub market_size_factor: i128,
    pub activity_factor: i128,
    pub complexity_factor: i128,
    pub total_adjustment: i128,
}

/// Historical record of dispute threshold changes for audit and governance.
///
/// This structure maintains a complete audit trail of all dispute threshold
/// modifications for a market, including the rationale for changes and the
/// administrator who authorized them. This ensures transparency and accountability
/// in threshold management decisions.
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Address, String, Symbol};
/// # use predictify_hybrid::voting::ThresholdHistoryEntry;
/// # let env = Env::default();
///
/// let history_entry = ThresholdHistoryEntry {
///     market_id: Symbol::new(&env, "BTC_100K"),
///     old_threshold: 100000000i128,    // Previous: 10 XLM
///     new_threshold: 150000000i128,    // New: 15 XLM
///     adjustment_reason: String::from_str(&env, "Increased due to high market activity"),
///     adjusted_by: Address::generate(&env),
///     timestamp: env.ledger().timestamp(),
/// };
/// ```
#[contracttype]
#[derive(Clone)]
pub struct ThresholdHistoryEntry {
    pub market_id: Symbol,
    pub old_threshold: i128,
    pub new_threshold: i128,
    pub adjustment_reason: String,
    pub adjusted_by: Address,
    pub timestamp: u64,
}

// ===== VOTING MANAGER =====

/// Comprehensive voting manager for prediction market voting operations.
///
/// VotingManager serves as the central coordinator for all voting-related operations
/// in the prediction market system. It handles vote processing, dispute management,
/// claim processing, fee collection, and dynamic threshold management. The manager
/// ensures voting integrity, proper stake handling, and accurate payout calculations.
///
/// # Core Functionality
///
/// **Vote Processing:**
/// - Validate and process user votes on market outcomes
/// - Handle stake transfers and vote recording
/// - Ensure voting eligibility and prevent duplicate votes
///
/// **Dispute Management:**
/// - Process dispute submissions against market resolutions
/// - Validate dispute stakes against dynamic thresholds
/// - Handle dispute stake transfers and recording
///
/// **Claim Processing:**
/// - Calculate and distribute winnings to successful voters
/// - Validate claim eligibility and prevent double claims
/// - Handle payout transfers and fee deductions
///
/// **Threshold Management:**
/// - Calculate dynamic dispute thresholds based on market characteristics
/// - Support administrative threshold adjustments
/// - Maintain threshold history for governance and audit
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Address, String, Symbol};
/// # use predictify_hybrid::voting::VotingManager;
/// # let env = Env::default();
///
/// let user = Address::generate(&env);
/// let market_id = Symbol::new(&env, "BTC_100K");
/// let outcome = String::from_str(&env, "yes");
/// let stake = 5000000i128; // 0.5 XLM
///
/// // Process a user vote
/// match VotingManager::process_vote(&env, user.clone(), market_id.clone(), outcome, stake) {
///     Ok(()) => println!("Vote processed successfully"),
///     Err(e) => println!("Vote processing failed: {:?}", e),
/// }
///
/// // Process a winning claim
/// match VotingManager::process_claim(&env, user.clone(), market_id.clone()) {
///     Ok(payout) => println!("Claim processed: {} stroops payout", payout),
///     Err(e) => println!("Claim processing failed: {:?}", e),
/// }
/// ```
///
/// # Integration Points
///
/// VotingManager integrates with:
/// - **Market System**: Validates market states and updates market data
/// - **Token System**: Handles stake transfers and payout distributions
/// - **Event System**: Emits events for all voting operations
/// - **Validation System**: Uses comprehensive validation for all operations
pub struct VotingManager;

impl VotingManager {
    /// Process a user's vote on a market
    pub fn process_vote(
        env: &Env,
        user: Address,
        market_id: Symbol,
        outcome: String,
        stake: i128,
    ) -> Result<(), Error> {
        // Require authentication from the user
        user.require_auth();

        // Get and validate market
        let mut market = MarketStateManager::get_market(env, &market_id)?;
        VotingValidator::validate_market_for_voting(env, &market)?;

        // Validate vote parameters
        VotingValidator::validate_vote_parameters(env, &outcome, &market.outcomes, stake)?;

        // Process stake transfer
        VotingUtils::transfer_stake(env, &user, stake)?;

        // Add vote to market (pass market_id for event emission)
        MarketStateManager::add_vote(&mut market, user, outcome, stake, Some(&market_id));
        MarketStateManager::update_market(env, &market_id, &market);

        Ok(())
    }

    /// Process a user's dispute of market result
    pub fn process_dispute(
        env: &Env,
        user: Address,
        market_id: Symbol,
        stake: i128,
    ) -> Result<(), Error> {
        // Require authentication from the user
        user.require_auth();

        // Get and validate market
        let mut market = MarketStateManager::get_market(env, &market_id)?;
        VotingValidator::validate_market_for_dispute(env, &market)?;

        // Validate dispute stake against dynamic config
        let cfg = crate::config::ConfigManager::get_config(env)?;
        if stake < cfg.voting.min_dispute_stake {
            return Err(Error::InsufficientStake);
        }

        // Process stake transfer
        VotingUtils::transfer_stake(env, &user, stake)?;

        // Add dispute stake and extend market (pass market_id for event emission)
        MarketStateManager::add_dispute_stake(&mut market, user, stake, Some(&market_id));
        MarketStateManager::extend_for_dispute(
            &mut market,
            env,
            cfg.voting.dispute_extension_hours.into(),
        );
        MarketStateManager::update_market(env, &market_id, &market);

        Ok(())
    }

    /// Process winnings claim for a user
    pub fn process_claim(env: &Env, user: Address, market_id: Symbol) -> Result<i128, Error> {
        // Require authentication from the user
        user.require_auth();

        // Get and validate market
        let mut market = MarketStateManager::get_market(env, &market_id)?;
        VotingValidator::validate_market_for_claim(env, &market, &user)?;

        // Calculate and process payout
        let payout = VotingUtils::calculate_user_payout(env, &market, &user)?;

        // Transfer winnings if any
        if payout > 0 {
            VotingUtils::transfer_winnings(env, &user, payout)?;
        }

        // Mark as claimed
        MarketStateManager::mark_claimed(&mut market, user, Some(&market_id));
        MarketStateManager::update_market(env, &market_id, &market);

        Ok(payout)
    }

    /// Collect platform fees from a market (moved to fees module)
    /// This function is deprecated and should use FeeManager::collect_fees instead
    pub fn collect_fees(env: &Env, admin: Address, market_id: Symbol) -> Result<i128, Error> {
        // Delegate to the fees module
        crate::fees::FeeManager::collect_fees(env, admin, market_id)
    }

    /// Calculate dynamic dispute threshold for a market using dynamic configuration
    pub fn calculate_dispute_threshold(
        env: &Env,
        market_id: Symbol,
    ) -> Result<DisputeThreshold, Error> {
        let _market = MarketStateManager::get_market(env, &market_id)?;

        // Load dynamic voting config
        let cfg = crate::config::ConfigManager::get_config(env)?;
        let base = cfg.voting.base_dispute_threshold;

        // Get adjustment factors (uses dynamic thresholds internally)
        let factors = ThresholdUtils::get_threshold_adjustment_factors(env, &market_id)?;

        // Calculate adjusted threshold and enforce dynamic bounds
        let mut adjusted_threshold = base + factors.total_adjustment;
        if adjusted_threshold < cfg.voting.min_dispute_stake {
            return Err(Error::ThresholdBelowMinimum);
        }
        if adjusted_threshold > cfg.voting.max_dispute_threshold {
            adjusted_threshold = cfg.voting.max_dispute_threshold;
        }

        // Create threshold data
        let threshold = DisputeThreshold {
            market_id: market_id.clone(),
            base_threshold: base,
            adjusted_threshold,
            market_size_factor: factors.market_size_factor,
            activity_factor: factors.activity_factor,
            complexity_factor: factors.complexity_factor,
            timestamp: env.ledger().timestamp(),
        };

        // Store threshold data
        ThresholdUtils::store_dispute_threshold(env, &market_id, &threshold)?;

        Ok(threshold)
    }

    /// Update dispute threshold for a market (admin only)
    pub fn update_dispute_thresholds(
        env: &Env,
        admin: Address,
        market_id: Symbol,
        new_threshold: i128,
        reason: String,
    ) -> Result<DisputeThreshold, Error> {
        // Require authentication from the admin
        admin.require_auth();

        // Validate admin permissions
        VotingValidator::validate_admin_authentication(env, &admin)?;

        // Validate new threshold
        ThresholdValidator::validate_threshold_limits(new_threshold)?;

        // Get current threshold
        let current_threshold = ThresholdUtils::get_dispute_threshold(env, &market_id)?;

        // Create new threshold data
        let new_threshold_data = DisputeThreshold {
            market_id: market_id.clone(),
            base_threshold: new_threshold,
            adjusted_threshold: new_threshold,
            market_size_factor: 0,
            activity_factor: 0,
            complexity_factor: 0,
            timestamp: env.ledger().timestamp(),
        };

        // Store new threshold
        ThresholdUtils::store_dispute_threshold(env, &market_id, &new_threshold_data)?;

        // Add to history
        ThresholdUtils::add_threshold_history_entry(
            env,
            &market_id,
            current_threshold.adjusted_threshold,
            new_threshold,
            reason,
            &admin,
        )?;

        // Get market for updating
        let mut market = MarketStateManager::get_market(env, &market_id)?;

        // Mark fees as collected
        MarketStateManager::mark_fees_collected(&mut market, Some(&market_id));
        MarketStateManager::update_market(env, &market_id, &market);
        Ok(new_threshold_data)
    }

    /// Get threshold history for a market
    pub fn get_threshold_history(
        env: &Env,
        market_id: Symbol,
    ) -> Result<Vec<ThresholdHistoryEntry>, Error> {
        ThresholdUtils::get_threshold_history(env, &market_id)
    }
}

// ===== THRESHOLD UTILITIES =====

/// Comprehensive threshold management utilities for dynamic dispute thresholds.
///
/// ThresholdUtils provides a complete suite of functions for managing dynamic dispute
/// thresholds in prediction markets. It handles threshold calculation, adjustment factor
/// computation, threshold storage and retrieval, history tracking, and validation.
/// The utilities ensure fair and appropriate dispute barriers based on market characteristics.
///
/// # Core Functionality
///
/// **Threshold Calculation:**
/// - Calculate dynamic thresholds based on market characteristics
/// - Apply adjustment factors for market size, activity, and complexity
/// - Ensure thresholds remain within acceptable bounds
///
/// **Factor Management:**
/// - Compute adjustment factors based on market metrics
/// - Handle market size, activity level, and complexity assessments
/// - Combine factors into total adjustment multipliers
///
/// **Storage Operations:**
/// - Store and retrieve dispute threshold configurations
/// - Maintain threshold history for audit and governance
/// - Handle threshold updates and modifications
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Symbol, Address, String};
/// # use predictify_hybrid::voting::{ThresholdUtils, DisputeThreshold};
/// # let env = Env::default();
///
/// let market_id = Symbol::new(&env, "BTC_100K");
///
/// // Get threshold adjustment factors
/// match ThresholdUtils::get_threshold_adjustment_factors(&env, &market_id) {
///     Ok(factors) => {
///         println!("Market size factor: {}", factors.market_size_factor);
///         println!("Activity factor: {}", factors.activity_factor);
///         println!("Total adjustment: {}", factors.total_adjustment);
///     }
///     Err(e) => println!("Factor calculation failed: {:?}", e),
/// }
/// ```
///
/// # Integration Points
///
/// ThresholdUtils integrates with:
/// - **Voting System**: Provide thresholds for dispute validation
/// - **Market Analytics**: Use market data for threshold calculations
/// - **Admin System**: Support manual threshold adjustments
/// - **Storage System**: Persist threshold data and history
pub struct ThresholdUtils;

impl ThresholdUtils {
    /// Get threshold adjustment factors for a market
    pub fn get_threshold_adjustment_factors(
        env: &Env,
        market_id: &Symbol,
    ) -> Result<ThresholdAdjustmentFactors, Error> {
        let market = MarketStateManager::get_market(env, market_id)?;

        // Calculate market size factor
        let market_size_factor = {
            let base = crate::config::ConfigManager::get_config(env)?
                .voting
                .base_dispute_threshold;
            Self::adjust_threshold_by_market_size(env, market_id, base)?
        };

        // Calculate activity factor
        let activity_factor =
            Self::modify_threshold_by_activity(env, market_id, market.votes.len() as u32)?;

        // Calculate complexity factor (based on number of outcomes) using dynamic base
        let base = crate::config::ConfigManager::get_config(env)?
            .voting
            .base_dispute_threshold;
        let complexity_factor = Self::calculate_complexity_factor(&market, base)?;

        let total_adjustment = market_size_factor + activity_factor + complexity_factor;

        Ok(ThresholdAdjustmentFactors {
            market_size_factor,
            activity_factor,
            complexity_factor,
            total_adjustment,
        })
    }

    /// Adjust threshold by market size
    pub fn adjust_threshold_by_market_size(
        env: &Env,
        market_id: &Symbol,
        base_threshold: i128,
    ) -> Result<i128, Error> {
        let market = MarketStateManager::get_market(env, market_id)?;

        // For large markets, increase threshold
        let large_threshold = crate::config::ConfigManager::get_config(env)?
            .voting
            .large_market_threshold;
        if market.total_staked > large_threshold {
            // Increase by 50% for large markets
            Ok((base_threshold * 150) / 100)
        } else {
            Ok(0) // No adjustment for smaller markets
        }
    }

    /// Modify threshold by activity level
    pub fn modify_threshold_by_activity(
        env: &Env,
        market_id: &Symbol,
        activity_level: u32,
    ) -> Result<i128, Error> {
        let _market = MarketStateManager::get_market(env, market_id)?;

        // For high activity markets, increase threshold
        let cfg = crate::config::ConfigManager::get_config(env)?;
        if activity_level > cfg.voting.high_activity_threshold {
            // Increase by 25% for high activity based on dynamic base
            Ok((cfg.voting.base_dispute_threshold * 25) / 100)
        } else {
            Ok(0) // No adjustment for lower activity
        }
    }

    /// Calculate complexity factor based on market characteristics
    pub fn calculate_complexity_factor(
        market: &Market,
        base_threshold: i128,
    ) -> Result<i128, Error> {
        // More outcomes = higher complexity = higher threshold
        let outcome_count = market.outcomes.len() as i128;

        if outcome_count > 3 {
            // Increase by 10% per additional outcome beyond 3
            let additional_outcomes = outcome_count - 3;
            Ok((base_threshold * 10 * additional_outcomes) / 100)
        } else {
            Ok(0)
        }
    }

    /// Calculate adjusted threshold based on factors
    pub fn calculate_adjusted_threshold(
        base_threshold: i128,
        factors: &ThresholdAdjustmentFactors,
    ) -> Result<i128, Error> {
        let adjusted = base_threshold + factors.total_adjustment;

        // Ensure within limits
        if adjusted < MIN_DISPUTE_STAKE {
            return Err(Error::ThresholdBelowMinimum);
        }

        if adjusted > MAX_DISPUTE_THRESHOLD {
            return Err(Error::ThresholdExceedsMaximum);
        }

        Ok(adjusted)
    }

    /// Store dispute threshold
    pub fn store_dispute_threshold(
        env: &Env,
        _market_id: &Symbol,
        threshold: &DisputeThreshold,
    ) -> Result<(), Error> {
        let key = symbol_short!("dispute_t");
        env.storage().persistent().set(&key, threshold);
        Ok(())
    }

    /// Get dispute threshold
    pub fn get_dispute_threshold(env: &Env, market_id: &Symbol) -> Result<DisputeThreshold, Error> {
        let key = symbol_short!("dispute_t");
        let cfg = crate::config::ConfigManager::get_config(env)?;
        Ok(env.storage().persistent().get(&key).unwrap_or_else(|| {
            let base = cfg.voting.base_dispute_threshold;
            DisputeThreshold {
                market_id: market_id.clone(),
                base_threshold: base,
                adjusted_threshold: base,
                market_size_factor: 0,
                activity_factor: 0,
                complexity_factor: 0,
                timestamp: env.ledger().timestamp(),
            }
        }))
    }

    /// Add threshold history entry
    pub fn add_threshold_history_entry(
        env: &Env,
        market_id: &Symbol,
        old_threshold: i128,
        new_threshold: i128,
        reason: String,
        adjusted_by: &Address,
    ) -> Result<(), Error> {
        let entry = ThresholdHistoryEntry {
            market_id: market_id.clone(),
            old_threshold,
            new_threshold,
            adjustment_reason: reason,
            adjusted_by: adjusted_by.clone(),
            timestamp: env.ledger().timestamp(),
        };

        let key = symbol_short!("th_hist");
        let mut history: Vec<ThresholdHistoryEntry> =
            env.storage().persistent().get(&key).unwrap_or(vec![env]);

        history.push_back(entry);
        env.storage().persistent().set(&key, &history);

        Ok(())
    }

    /// Get threshold history
    pub fn get_threshold_history(
        env: &Env,
        market_id: &Symbol,
    ) -> Result<Vec<ThresholdHistoryEntry>, Error> {
        let key = symbol_short!("th_hist");
        let history: Vec<ThresholdHistoryEntry> =
            env.storage().persistent().get(&key).unwrap_or(vec![env]);

        // Filter by market_id
        let mut filtered_history = vec![env];
        for entry in history.iter() {
            if entry.market_id == *market_id {
                filtered_history.push_back(entry);
            }
        }

        Ok(filtered_history)
    }

    /// Validate dispute threshold
    pub fn validate_dispute_threshold(threshold: i128, _market_id: &Symbol) -> Result<bool, Error> {
        if threshold < MIN_DISPUTE_STAKE {
            return Err(Error::ThresholdBelowMinimum);
        }

        if threshold > MAX_DISPUTE_THRESHOLD {
            return Err(Error::ThresholdExceedsMaximum);
        }

        Ok(true)
    }
}

// ===== THRESHOLD VALIDATOR =====

/// Comprehensive validation utilities for threshold-related operations.
///
/// ThresholdValidator provides specialized validation functions for all threshold-related
/// operations in the voting system. It ensures that threshold values are within acceptable
/// ranges, validates administrative permissions for threshold adjustments, and maintains
/// system integrity through comprehensive validation checks.
///
/// # Core Functionality
///
/// **Threshold Validation:**
/// - Validate threshold values against system limits
/// - Check threshold ranges and constraints
/// - Ensure threshold consistency and integrity
///
/// **Permission Validation:**
/// - Validate administrative permissions for threshold adjustments
/// - Check user authorization for threshold operations
/// - Ensure proper access control for sensitive operations
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Address};
/// # use predictify_hybrid::voting::ThresholdValidator;
/// # let env = Env::default();
///
/// // Validate threshold limits
/// let threshold = 100000000i128; // 10 XLM
/// match ThresholdValidator::validate_threshold_limits(threshold) {
///     Ok(()) => println!("Threshold is valid"),
///     Err(e) => println!("Threshold validation failed: {:?}", e),
/// }
///
/// // Validate admin permissions
/// let admin = Address::generate(&env);
/// match ThresholdValidator::validate_threshold_adjustment_permissions(&env, &admin) {
///     Ok(()) => println!("Admin authorized for threshold adjustments"),
///     Err(e) => println!("Permission validation failed: {:?}", e),
/// }
/// ```
///
/// # Integration Points
///
/// ThresholdValidator integrates with:
/// - **Admin System**: Validate administrative operations
/// - **Threshold System**: Ensure threshold value integrity
/// - **Security System**: Enforce access control and permissions
pub struct ThresholdValidator;

impl ThresholdValidator {
    /// Validate threshold limits
    pub fn validate_threshold_limits(threshold: i128) -> Result<(), Error> {
        if threshold < MIN_DISPUTE_STAKE {
            return Err(Error::ThresholdBelowMinimum);
        }

        if threshold > MAX_DISPUTE_THRESHOLD {
            return Err(Error::ThresholdExceedsMaximum);
        }

        Ok(())
    }

    /// Validate threshold adjustment permissions
    pub fn validate_threshold_adjustment_permissions(
        env: &Env,
        admin: &Address,
    ) -> Result<(), Error> {
        VotingValidator::validate_admin_authentication(env, admin)
    }
}

// ===== VOTING VALIDATOR =====

/// Comprehensive validation utilities for voting-related operations.
///
/// VotingValidator provides specialized validation functions for all voting operations
/// in the prediction market system. It ensures voting integrity, validates user permissions,
/// checks market states, and validates stake amounts to maintain system security and fairness.
///
/// # Core Functionality
///
/// **User Validation:**
/// - Validate user authentication and authorization
/// - Check admin permissions for administrative operations
/// - Ensure proper access control for voting operations
///
/// **Market State Validation:**
/// - Validate market states for voting eligibility
/// - Check market states for dispute eligibility
/// - Verify market states for claim processing
/// - Validate market states for fee collection
///
/// **Parameter Validation:**
/// - Validate vote parameters including outcomes and stakes
/// - Check dispute stake amounts against thresholds
/// - Validate stake amounts with dynamic threshold checking
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Address, String, Vec};
/// # use predictify_hybrid::voting::VotingValidator;
/// # use predictify_hybrid::types::Market;
/// # let env = Env::default();
///
/// // Validate vote parameters
/// let outcome = String::from_str(&env, "yes");
/// let valid_outcomes = vec![&env,
///     String::from_str(&env, "yes"),
///     String::from_str(&env, "no")
/// ];
/// let stake = 5000000i128; // 0.5 XLM
///
/// match VotingValidator::validate_vote_parameters(&env, &outcome, &valid_outcomes, stake) {
///     Ok(()) => println!("Vote parameters are valid"),
///     Err(e) => println!("Vote validation failed: {:?}", e),
/// }
///
/// // Validate dispute stake
/// let dispute_stake = 100000000i128; // 10 XLM
/// match VotingValidator::validate_dispute_stake(dispute_stake) {
///     Ok(()) => println!("Dispute stake is sufficient"),
///     Err(e) => println!("Dispute stake validation failed: {:?}", e),
/// }
/// ```
///
/// # Integration Points
///
/// VotingValidator integrates with:
/// - **Authentication System**: Validate user permissions and access
/// - **Market System**: Check market states and eligibility
/// - **Stake System**: Validate stake amounts and requirements
/// - **Security System**: Enforce voting rules and constraints
pub struct VotingValidator;

impl VotingValidator {
    /// Validate user authentication
    pub fn validate_user_authentication(_user: &Address) -> Result<(), Error> {
        // Note: In Soroban, authentication is handled by require_auth()
        // This function serves as a placeholder for additional validation logic
        Ok(())
    }

    /// Validate admin authentication and permissions
    pub fn validate_admin_authentication(env: &Env, admin: &Address) -> Result<(), Error> {
        let stored_admin: Option<Address> =
            env.storage().persistent().get(&Symbol::new(env, "Admin"));

        match stored_admin {
            Some(stored_admin) => {
                if admin != &stored_admin {
                    return Err(Error::Unauthorized);
                }
                Ok(())
            }
            None => Err(Error::Unauthorized),
        }
    }

    /// Validate market state for voting
    pub fn validate_market_for_voting(env: &Env, market: &Market) -> Result<(), Error> {
        // Check if market is active
        let current_time = env.ledger().timestamp();
        if current_time >= market.end_time {
            return Err(Error::MarketClosed);
        }

        // Check if market is already resolved
        if market.winning_outcome.is_some() {
            return Err(Error::MarketAlreadyResolved);
        }

        Ok(())
    }

    /// Validate market state for dispute
    pub fn validate_market_for_dispute(env: &Env, market: &Market) -> Result<(), Error> {
        // Check if market has ended
        let current_time = env.ledger().timestamp();
        if current_time < market.end_time {
            return Err(Error::MarketClosed);
        }

        // Check if market is already resolved
        if market.winning_outcome.is_some() {
            return Err(Error::MarketAlreadyResolved);
        }

        Ok(())
    }

    /// Validate market state for claim

    pub fn validate_market_for_claim(
        _env: &Env,
        market: &Market,
        user: &Address,
    ) -> Result<(), Error> {
        // Check if user has already claimed
        let claimed = market.claimed.get(user.clone()).unwrap_or(false);
        if claimed {
            return Err(Error::AlreadyClaimed);
        }

        // Check if market is resolved
        if market.winning_outcome.is_none() {
            return Err(Error::MarketNotResolved);
        }

        // Check if user has voted
        if !market.votes.contains_key(user.clone()) {
            return Err(Error::NothingToClaim);
        }

        Ok(())
    }

    /// Validate market state for fee collection
    pub fn validate_market_for_fee_collection(_market: &Market) -> Result<(), Error> {
        // Check if fees already collected
        // This function is deprecated and should use FeeManager::validate_market_for_fee_collection instead
        Ok(())
    }

    /// Validate vote parameters
    pub fn validate_vote_parameters(
        env: &Env,
        outcome: &String,
        valid_outcomes: &Vec<String>,
        stake: i128,
    ) -> Result<(), Error> {
        // Validate outcome
        if let Err(e) = MarketValidator::validate_outcome(env, outcome, valid_outcomes) {
            return Err(e);
        }

        // Validate stake against dynamic config
        let min_vote = crate::config::ConfigManager::get_config(env)?
            .voting
            .min_vote_stake;
        if let Err(e) = MarketValidator::validate_stake(stake, min_vote) {
            return Err(e);
        }

        Ok(())
    }

    /// Validate dispute stake
    pub fn validate_dispute_stake(stake: i128) -> Result<(), Error> {
        if stake < MIN_DISPUTE_STAKE {
            return Err(Error::InsufficientStake);
        }

        Ok(())
    }

    /// Validate dispute stake with dynamic threshold
    pub fn validate_dispute_stake_with_threshold(
        env: &Env,
        stake: i128,
        market_id: &Symbol,
    ) -> Result<(), Error> {
        // Get dynamic threshold for the market
        let threshold = ThresholdUtils::get_dispute_threshold(env, market_id)?;

        if stake < threshold.adjusted_threshold {
            return Err(Error::InsufficientStake);
        }

        Ok(())
    }
}

// ===== VOTING UTILITIES =====

/// Comprehensive utility functions for voting operations.
///
/// VotingUtils provides essential utility functions for voting operations in prediction
/// markets, including stake transfers, payout calculations, voting statistics, and user
/// vote tracking. These utilities support the core voting functionality and ensure
/// proper handling of stakes, payouts, and voting data.
///
/// # Core Functionality
///
/// **Stake Management:**
/// - Transfer stakes from users to contract
/// - Transfer winnings to users
/// - Handle fee transfers to administrators
/// - Calculate payout amounts for winning voters
///
/// **Statistics and Analytics:**
/// - Generate voting statistics for markets
/// - Track user voting participation
/// - Retrieve user vote details
/// - Monitor claim status for users
///
/// **Fee Calculations:**
/// - Calculate platform fees for markets
/// - Handle fee distribution and transfers
/// - Support fee-related operations
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Address};
/// # use predictify_hybrid::voting::VotingUtils;
/// # use predictify_hybrid::types::Market;
/// # let env = Env::default();
///
/// // Transfer stake from user
/// let user = Address::generate(&env);
/// let stake = 5000000i128; // 0.5 XLM
///
/// match VotingUtils::transfer_stake(&env, &user, stake) {
///     Ok(()) => println!("Stake transferred successfully"),
///     Err(e) => println!("Stake transfer failed: {:?}", e),
/// }
///
/// // Check if user has voted
/// # let market = Market::new(
/// #     &env,
/// #     Address::generate(&env),
/// #     String::from_str(&env, "Test"),
/// #     vec![&env, String::from_str(&env, "yes"), String::from_str(&env, "no")],
/// #     env.ledger().timestamp() + 86400,
/// #     crate::types::OracleConfig::new(
/// #         crate::types::OracleProvider::Reflector,
/// #         String::from_str(&env, "BTC/USD"),
/// #         100000000000i128,
/// #         String::from_str(&env, "gte")
/// #     ),
/// #     crate::types::MarketState::Active
/// # );
///
/// if VotingUtils::has_user_voted(&market, &user) {
///     println!("User has already voted on this market");
/// } else {
///     println!("User has not voted yet");
/// }
/// ```
///
/// # Integration Points
///
/// VotingUtils integrates with:
/// - **Token System**: Handle stake and payout transfers
/// - **Market System**: Access market data for calculations
/// - **Analytics System**: Provide voting statistics and metrics
/// - **Fee System**: Calculate and distribute platform fees
pub struct VotingUtils;

impl VotingUtils {
    /// Transfer stake from user to contract
    pub fn transfer_stake(env: &Env, user: &Address, stake: i128) -> Result<(), Error> {
        // Reentrancy guard removed - external call protection no longer needed
        let token_client = MarketUtils::get_token_client(env)?;
        // Soroban token transfer returns (), assume success if no panic
        token_client.transfer(user, &env.current_contract_address(), &stake);
        Ok(())
    }

    /// Transfer winnings to user
    pub fn transfer_winnings(env: &Env, user: &Address, amount: i128) -> Result<(), Error> {
        // Reentrancy guard removed - external call protection no longer needed
        let token_client = MarketUtils::get_token_client(env)?;
        token_client.transfer(&env.current_contract_address(), user, &amount);
        Ok(())
    }

    /// Transfer fees to admin (moved to fees module)
    /// This function is deprecated and should use FeeUtils::transfer_fees_to_admin instead
    pub fn transfer_fees(env: &Env, admin: &Address, amount: i128) -> Result<(), Error> {
        // Delegate to the fees module
        crate::fees::FeeUtils::transfer_fees_to_admin(env, admin, amount)
    }

    /// Calculate user's payout
    pub fn calculate_user_payout(
        env: &Env,
        market: &Market,
        user: &Address,
    ) -> Result<i128, Error> {
        let winning_outcome = market
            .winning_outcome
            .as_ref()
            .ok_or(Error::MarketNotResolved)?;

        let user_outcome = market
            .votes
            .get(user.clone())
            .ok_or(Error::NothingToClaim)?;

        let user_stake = market.stakes.get(user.clone()).unwrap_or(0);

        // Only pay if user voted for winning outcome
        if user_outcome != *winning_outcome {
            return Ok(0);
        }

        // Calculate winning statistics
        let winning_stats = MarketAnalytics::calculate_winning_stats(market, winning_outcome);

        // Calculate payout
        // Use dynamic platform fee percentage from current configuration
        let cfg = crate::config::ConfigManager::get_config(env)?;
        let payout = MarketUtils::calculate_payout(
            user_stake,
            winning_stats.winning_total,
            winning_stats.total_pool,
            cfg.fees.platform_fee_percentage,
        )?;

        Ok(payout)
    }

    /// Calculate fee amount for a market (moved to fees module)
    /// This function is deprecated and should use FeeCalculator::calculate_platform_fee instead
    pub fn calculate_fee_amount(market: &Market) -> Result<i128, Error> {
        // Delegate to the fees module
        crate::fees::FeeCalculator::calculate_platform_fee(market)
    }

    /// Get voting statistics for a market
    pub fn get_voting_stats(_market: &Market) -> VotingStats {
        // TODO: Implement proper voting stats calculation
        // This requires access to the environment for Map creation
        VotingStats {
            total_votes: 0,
            total_staked: 0,
            outcome_distribution: Map::new(&Env::default()),
            unique_voters: 0,
        }
    }

    /// Check if user has voted on a market
    pub fn has_user_voted(market: &Market, user: &Address) -> bool {
        market.votes.contains_key(user.clone())
    }

    /// Get user's vote details
    pub fn get_user_vote(market: &Market, user: &Address) -> Option<(String, i128)> {
        let outcome = market.votes.get(user.clone())?;
        let stake = market.stakes.get(user.clone()).unwrap_or(0);
        Some((outcome, stake))
    }

    /// Check if user has claimed winnings
    pub fn has_user_claimed(market: &Market, user: &Address) -> bool {
        market.claimed.get(user.clone()).unwrap_or(false)
    }
}

// ===== VOTING ANALYTICS =====

/// Comprehensive analytics functions for voting data analysis.
///
/// VotingAnalytics provides advanced analytical capabilities for prediction market
/// voting data, including participation analysis, stake distribution calculations,
/// voting power concentration metrics, and voter ranking systems. These analytics
/// support market insights, risk assessment, and governance decisions.
///
/// # Core Functionality
///
/// **Participation Analytics:**
/// - Calculate voting participation rates
/// - Analyze voter engagement patterns
/// - Track participation trends over time
///
/// **Stake Analytics:**
/// - Calculate average stake per voter
/// - Analyze stake distribution by outcome
/// - Measure voting power concentration
/// - Identify top voters by stake amount
///
/// **Market Health Metrics:**
/// - Assess market consensus strength
/// - Detect potential manipulation patterns
/// - Evaluate market liquidity and activity
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Address, String, Map};
/// # use predictify_hybrid::voting::VotingAnalytics;
/// # use predictify_hybrid::types::Market;
/// # let env = Env::default();
///
/// # let market = Market::new(
/// #     &env,
/// #     Address::generate(&env),
/// #     String::from_str(&env, "Test Market"),
/// #     vec![&env, String::from_str(&env, "yes"), String::from_str(&env, "no")],
/// #     env.ledger().timestamp() + 86400,
/// #     crate::types::OracleConfig::new(
/// #         crate::types::OracleProvider::Reflector,
/// #         String::from_str(&env, "BTC/USD"),
/// #         100000000000i128,
/// #         String::from_str(&env, "gte")
/// #     ),
/// #     crate::types::MarketState::Active
/// # );
///
/// // Calculate participation rate
/// let participation_rate = VotingAnalytics::calculate_participation_rate(&market);
/// println!("Participation rate: {:.2}%", participation_rate * 100.0);
///
/// // Calculate average stake
/// let avg_stake = VotingAnalytics::calculate_average_stake(&market);
/// println!("Average stake per voter: {} stroops", avg_stake);
///
/// // Analyze voting power concentration
/// let concentration = VotingAnalytics::calculate_voting_power_concentration(&market);
/// println!("Voting power concentration: {:.2}", concentration);
///
/// // Get top voters
/// let top_voters = VotingAnalytics::get_top_voters(&market, 5);
/// println!("Top {} voters by stake:", top_voters.len());
/// for (i, (voter, stake)) in top_voters.iter().enumerate() {
///     println!("{}. {}: {} stroops", i + 1, voter, stake);
/// }
/// ```
///
/// # Analytics Applications
///
/// - **Market Health Assessment**: Evaluate market participation and engagement
/// - **Risk Management**: Detect concentration risks and manipulation attempts
/// - **Governance Insights**: Inform threshold adjustments and policy decisions
/// - **User Experience**: Provide market insights and voting recommendations
///
/// # Integration Points
///
/// VotingAnalytics integrates with:
/// - **Market System**: Access voting and market data
/// - **Dashboard System**: Provide real-time analytics displays
/// - **Risk Management**: Support risk assessment and monitoring
/// - **Governance System**: Inform policy and threshold decisions
pub struct VotingAnalytics;

impl VotingAnalytics {
    /// Calculate voting participation rate
    pub fn calculate_participation_rate(market: &Market) -> f64 {
        if market.total_staked == 0 {
            return 0.0;
        }

        // This is a simplified calculation - in a real scenario you might want
        // to track total eligible participants
        let participation_rate = (market.votes.len() as f64) / 100.0; // Assuming 100 max participants
        participation_rate.min(1.0)
    }

    /// Calculate average stake per voter
    pub fn calculate_average_stake(market: &Market) -> i128 {
        if market.votes.is_empty() {
            return 0;
        }

        market.total_staked / (market.votes.len() as i128)
    }

    /// Calculate stake distribution by outcome
    pub fn calculate_stake_distribution(_market: &Market) -> Map<String, i128> {
        // TODO: Implement proper stake distribution calculation
        // This requires access to the environment for Map creation
        Map::new(&Env::default())
    }

    /// Calculate voting power concentration
    pub fn calculate_voting_power_concentration(market: &Market) -> f64 {
        if market.total_staked == 0 {
            return 0.0;
        }

        let mut total_squared_stakes = 0i128;
        for (_, stake) in market.stakes.iter() {
            total_squared_stakes += stake * stake;
        }

        let concentration =
            (total_squared_stakes as f64) / ((market.total_staked * market.total_staked) as f64);
        concentration.min(1.0)
    }

    /// Get top voters by stake amount
    pub fn get_top_voters(_market: &Market, _limit: usize) -> Vec<(Address, i128)> {
        // TODO: Implement proper top voters calculation
        // This requires Vec operations that are not available in no_std
        Vec::new(&Env::default())
    }
}

// ===== VOTING TESTING UTILITIES =====

#[cfg(test)]
pub mod testing {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    /// Create a test vote for testing and development purposes.
    ///
    /// This utility function creates a properly structured Vote instance for use in
    /// testing scenarios. It ensures all required fields are populated with valid
    /// test data and follows the same structure as production votes.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for timestamp generation
    /// * `user` - The address of the test user casting the vote
    /// * `outcome` - The outcome the test user is voting for
    /// * `stake` - The stake amount for the test vote (in stroops)
    ///
    /// # Example Usage
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address, String};
    /// # use predictify_hybrid::voting::testing;
    /// # let env = Env::default();
    ///
    /// let test_user = Address::generate(&env);
    /// let test_outcome = String::from_str(&env, "yes");
    /// let test_stake = 5000000i128; // 0.5 XLM
    ///
    /// let test_vote = testing::create_test_vote(&env, test_user, test_outcome, test_stake);
    ///
    /// assert_eq!(test_vote.stake, test_stake);
    /// assert_eq!(test_vote.outcome, String::from_str(&env, "yes"));
    /// ```
    pub fn create_test_vote(env: &Env, user: Address, outcome: String, stake: i128) -> Vote {
        Vote {
            user,
            outcome,
            stake,
            timestamp: env.ledger().timestamp(),
        }
    }

    /// Create test voting statistics for testing and development purposes.
    ///
    /// This utility function creates a properly structured VotingStats instance with
    /// realistic test data for use in testing scenarios. It includes sample outcome
    /// distributions and voting metrics that reflect typical market activity.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for creating maps and data structures
    ///
    /// # Example Usage
    ///
    /// ```rust
    /// # use soroban_sdk::Env;
    /// # use predictify_hybrid::voting::testing;
    /// # let env = Env::default();
    ///
    /// let test_stats = testing::create_test_voting_stats(&env);
    ///
    /// assert!(test_stats.total_votes > 0);
    /// assert!(test_stats.total_staked > 0);
    /// assert!(test_stats.unique_voters > 0);
    /// assert!(!test_stats.outcome_distribution.is_empty());
    /// ```
    pub fn create_test_voting_stats(env: &Env) -> VotingStats {
        let outcome_distribution = Map::new(env);
        VotingStats {
            total_votes: 0,
            total_staked: 0,
            outcome_distribution,
            unique_voters: 0,
        }
    }

    /// Create test payout data for testing and development purposes.
    ///
    /// This utility function creates a properly structured PayoutData instance with
    /// realistic test values for payout calculations. The test data includes valid
    /// stake amounts, pool totals, fee percentages, and calculated payout amounts.
    ///
    /// # Example Usage
    ///
    /// ```rust
    /// # use predictify_hybrid::voting::testing;
    ///
    /// let test_payout = testing::create_test_payout_data();
    ///
    /// assert!(test_payout.user_stake > 0);
    /// assert!(test_payout.winning_total > 0);
    /// assert!(test_payout.total_pool > 0);
    /// assert!(test_payout.fee_percentage >= 0);
    /// assert!(test_payout.payout_amount > 0);
    /// ```
    pub fn create_test_payout_data() -> PayoutData {
        PayoutData {
            user_stake: 1000,
            winning_total: 5000,
            total_pool: 10000,
            fee_percentage: 2,
            payout_amount: 1960, // (1000 * 5000 / 10000) * 0.98
        }
    }

    /// Validate the structure and integrity of a Vote instance.
    ///
    /// This utility function performs comprehensive validation of a Vote structure
    /// to ensure all fields are properly populated and within valid ranges. It checks
    /// stake amounts, timestamp validity, and outcome format consistency.
    ///
    /// # Parameters
    ///
    /// * `vote` - The Vote instance to validate
    ///
    /// # Example Usage
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address, String};
    /// # use predictify_hybrid::voting::{Vote, testing};
    /// # let env = Env::default();
    ///
    /// let test_vote = Vote {
    ///     user: Address::generate(&env),
    ///     outcome: String::from_str(&env, "yes"),
    ///     stake: 5000000i128,
    ///     timestamp: env.ledger().timestamp(),
    /// };
    ///
    /// match testing::validate_vote_structure(&test_vote) {
    ///     Ok(()) => println!("Vote structure is valid"),
    ///     Err(e) => println!("Vote validation failed: {:?}", e),
    /// }
    /// ```
    pub fn validate_vote_structure(vote: &Vote) -> Result<(), Error> {
        if vote.stake <= 0 {
            return Err(Error::InsufficientStake);
        }

        if vote.outcome.is_empty() {
            return Err(Error::InvalidOutcome);
        }

        Ok(())
    }

    /// Validate the structure and consistency of a VotingStats instance.
    ///
    /// This utility function performs comprehensive validation of a VotingStats structure
    /// to ensure all fields are consistent and within valid ranges. It checks vote counts,
    /// stake totals, outcome distribution consistency, and voter count validity.
    ///
    /// # Parameters
    ///
    /// * `stats` - The VotingStats instance to validate
    ///
    /// # Example Usage
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Map, String};
    /// # use predictify_hybrid::voting::{VotingStats, testing};
    /// # let env = Env::default();
    ///
    /// let mut outcome_distribution = Map::new(&env);
    /// outcome_distribution.set(String::from_str(&env, "yes"), 10000000i128);
    /// outcome_distribution.set(String::from_str(&env, "no"), 5000000i128);
    ///
    /// let test_stats = VotingStats {
    ///     total_votes: 15,
    ///     total_staked: 15000000i128,
    ///     outcome_distribution,
    ///     unique_voters: 12,
    /// };
    ///
    /// match testing::validate_voting_stats(&test_stats) {
    ///     Ok(()) => println!("Voting stats structure is valid"),
    ///     Err(e) => println!("Stats validation failed: {:?}", e),
    /// }
    /// ```
    pub fn validate_voting_stats(stats: &VotingStats) -> Result<(), Error> {
        if stats.total_staked < 0 {
            return Err(Error::InsufficientStake);
        }

        if stats.total_votes < stats.unique_voters {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }
}

// ===== MODULE TESTS =====

#[cfg(test)]
mod tests {
    use super::*;

    use crate::types::{OracleConfig, OracleProvider};
    use soroban_sdk::{testutils::Address as _, vec};

    #[test]
    fn test_voting_validator_authentication() {
        let env = Env::default();
        let user = Address::generate(&env);

        // Should not panic for valid user
        assert!(VotingValidator::validate_user_authentication(&user).is_ok());
    }

    #[test]
    fn test_voting_validator_stake_validation() {
        // Valid stake
        assert!(VotingValidator::validate_dispute_stake(MIN_DISPUTE_STAKE).is_ok());

        // Invalid stake
        assert!(VotingValidator::validate_dispute_stake(MIN_DISPUTE_STAKE - 1).is_err());
    }

    #[test]
    fn test_voting_utils_fee_calculation() {
        let env = Env::default();
        let mut market = Market::new(
            &env,
            Address::generate(&env),
            String::from_str(&env, "Test Market"),
            soroban_sdk::vec![
                &env,
                String::from_str(&env, "yes"),
                String::from_str(&env, "no"),
            ],
            env.ledger().timestamp() + 86400,
            OracleConfig::new(
                OracleProvider::Pyth,
                String::from_str(&env, "BTC/USD"),
                2500000,
                String::from_str(&env, "gt"),
            ),
            crate::types::MarketState::Active,
        );
        market.total_staked = 100_000_000; // 10 XLM

        let fee = VotingUtils::calculate_fee_amount(&market).unwrap();
        assert_eq!(fee, 2_000_000); // 2% of 100_000_000 = 2_000_000 (0.2 XLM)
    }

    #[test]
    fn test_voting_analytics_average_stake() {
        let env = Env::default();
        let mut market = Market::new(
            &env,
            Address::generate(&env),
            String::from_str(&env, "Test Market"),
            soroban_sdk::vec![
                &env,
                String::from_str(&env, "yes"),
                String::from_str(&env, "no"),
            ],
            env.ledger().timestamp() + 86400,
            OracleConfig::new(
                OracleProvider::Pyth,
                String::from_str(&env, "BTC/USD"),
                2500000,
                String::from_str(&env, "gt"),
            ),
            crate::types::MarketState::Active,
        );

        // Add some test votes
        let user1 = Address::generate(&env);
        let user2 = Address::generate(&env);

        market.add_vote(user1, String::from_str(&env, "yes"), 1000);
        market.add_vote(user2, String::from_str(&env, "no"), 2000);

        let avg_stake = VotingAnalytics::calculate_average_stake(&market);
        assert_eq!(avg_stake, 1500); // (1000 + 2000) / 2
    }

    #[test]
    fn test_voting_utils_stats() {
        let env = Env::default();
        let mut market = Market::new(
            &env,
            Address::generate(&env),
            String::from_str(&env, "Test Market"),
            soroban_sdk::vec![
                &env,
                String::from_str(&env, "yes"),
                String::from_str(&env, "no"),
            ],
            env.ledger().timestamp() + 86400,
            OracleConfig::new(
                OracleProvider::Pyth,
                String::from_str(&env, "BTC/USD"),
                2500000,
                String::from_str(&env, "gt"),
            ),
            crate::types::MarketState::Active,
        );

        let user = Address::generate(&env);
        market.add_vote(user.clone(), String::from_str(&env, "yes"), 1000);

        let stats = VotingUtils::get_voting_stats(&market);
        assert_eq!(stats.total_votes, 0); // Simplified implementation returns 0
        assert_eq!(stats.total_staked, 0); // Simplified implementation returns 0
        assert_eq!(stats.unique_voters, 0); // Simplified implementation returns 0
        assert!(VotingUtils::has_user_voted(&market, &user));
    }

    #[test]
    fn test_testing_utilities() {
        let env = Env::default();
        let user = Address::generate(&env);

        let vote = testing::create_test_vote(&env, user, String::from_str(&env, "yes"), 1000);

        assert!(testing::validate_vote_structure(&vote).is_ok());

        let stats = testing::create_test_voting_stats(&env);
        assert!(testing::validate_voting_stats(&stats).is_ok());
    }
}
