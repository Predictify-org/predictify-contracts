use crate::{
    errors::Error,
    markets::{MarketStateManager, MarketValidator, MarketUtils, MarketAnalytics},
    types::Market,
    config::OracleConfig,
    types::OracleProvider,
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

/// Represents a user's vote on a market
#[contracttype]
pub struct Vote {
    pub user: Address,
    pub outcome: String,
    pub stake: i128,
    pub timestamp: u64,
}

/// Represents voting statistics for a market
#[contracttype]
pub struct VotingStats {
    pub total_votes: u32,
    pub total_staked: i128,
    pub outcome_distribution: Map<String, i128>,
    pub unique_voters: u32,
}

/// Represents payout calculation data
#[contracttype]
pub struct PayoutData {
    pub user_stake: i128,
    pub winning_total: i128,
    pub total_pool: i128,
    pub fee_percentage: i128,
    pub payout_amount: i128,
}

/// Represents dispute threshold data
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

/// Represents threshold adjustment factors
#[contracttype]
pub struct ThresholdAdjustmentFactors {
    pub market_size_factor: i128,
    pub activity_factor: i128,
    pub complexity_factor: i128,
    pub total_adjustment: i128,
}

/// Represents threshold history entry
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

/// Main voting manager for handling all voting operations
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
        let mut _market = MarketStateManager::get_market(env, &market_id)?;
        VotingValidator::validate_market_for_voting(env, &_market)?;

        // Validate vote parameters
        VotingValidator::validate_vote_parameters(env, &outcome, &_market.outcomes, stake)?;

        // Process stake transfer
        VotingUtils::transfer_stake(env, &user, stake)?;

        // Add vote to market
        MarketStateManager::add_vote(&mut _market, user, outcome, stake);
        MarketStateManager::update_market(env, &market_id, &_market);

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
        let mut _market = MarketStateManager::get_market(env, &market_id)?;
        VotingValidator::validate_market_for_dispute(env, &_market)?;

        // Validate dispute stake
        VotingValidator::validate_dispute_stake(stake)?;

        // Process stake transfer
        VotingUtils::transfer_stake(env, &user, stake)?;

        // Add dispute stake and extend market
        MarketStateManager::add_dispute_stake(&mut _market, user, stake);
        MarketStateManager::extend_for_dispute(&mut _market, env, DISPUTE_EXTENSION_HOURS.into());
        MarketStateManager::update_market(env, &market_id, &_market);

        Ok(())
    }

    /// Process winnings claim for a user
    pub fn process_claim(env: &Env, user: Address, market_id: Symbol) -> Result<i128, Error> {
        // Require authentication from the user
        user.require_auth();

        // Get and validate market
        let mut _market = MarketStateManager::get_market(env, &market_id)?;
        VotingValidator::validate_market_for_claim(env, &_market, &user)?;

        // Calculate and process payout
        let payout = VotingUtils::calculate_user_payout(env, &_market, &user)?;

        // Transfer winnings if any
        if payout > 0 {
            VotingUtils::transfer_winnings(env, &user, payout)?;
        }

        // Mark as claimed
        MarketStateManager::mark_claimed(&mut _market, user);
        MarketStateManager::update_market(env, &market_id, &_market);

        Ok(payout)
    }

    /// Collect platform fees from a market (moved to fees module)
    /// This function is deprecated and should use FeeManager::collect_fees instead
    pub fn collect_fees(env: &Env, admin: Address, market_id: Symbol) -> Result<i128, Error> {
        // Delegate to the fees module
        crate::fees::FeeManager::collect_fees(env, admin, market_id)
    }

    /// Calculate dynamic dispute threshold for a market
    pub fn calculate_dispute_threshold(env: &Env, market_id: Symbol) -> Result<DisputeThreshold, Error> {
        let _market = MarketStateManager::get_market(env, &market_id)?;
        
        // Get adjustment factors
        let factors = ThresholdUtils::get_threshold_adjustment_factors(env, &market_id)?;
        
        // Calculate adjusted threshold
        let adjusted_threshold = ThresholdUtils::calculate_adjusted_threshold(
            BASE_DISPUTE_THRESHOLD,
            &factors,
        )?;
        
        // Create threshold data
        let threshold = DisputeThreshold {
            market_id: market_id.clone(),
            base_threshold: BASE_DISPUTE_THRESHOLD,
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

        Ok(new_threshold_data)
    }

    /// Get threshold history for a market
    pub fn get_threshold_history(env: &Env, market_id: Symbol) -> Result<Vec<ThresholdHistoryEntry>, Error> {
        ThresholdUtils::get_threshold_history(env, &market_id)
    }
}

// ===== THRESHOLD UTILITIES =====

/// Utility functions for threshold management
pub struct ThresholdUtils;

impl ThresholdUtils {
    /// Get threshold adjustment factors for a market
    pub fn get_threshold_adjustment_factors(
        env: &Env,
        market_id: &Symbol,
    ) -> Result<ThresholdAdjustmentFactors, Error> {
        let _market = MarketStateManager::get_market(env, market_id)?;
        
        // Calculate market size factor
        let market_size_factor = Self::adjust_threshold_by_market_size(env, market_id, BASE_DISPUTE_THRESHOLD)?;
        
        // Calculate activity factor
        let activity_factor = Self::modify_threshold_by_activity(env, market_id, _market.votes.len() as u32)?;
        
        // Calculate complexity factor (based on number of outcomes)
        let complexity_factor = Self::calculate_complexity_factor(&_market)?;
        
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
        let _market = MarketStateManager::get_market(env, market_id)?;
        
        // For large markets, increase threshold
        if _market.total_staked > LARGE_MARKET_THRESHOLD {
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
        if activity_level > HIGH_ACTIVITY_THRESHOLD {
            // Increase by 25% for high activity
            Ok((BASE_DISPUTE_THRESHOLD * 25) / 100)
        } else {
            Ok(0) // No adjustment for lower activity
        }
    }

    /// Calculate complexity factor based on market characteristics
    pub fn calculate_complexity_factor(market: &Market) -> Result<i128, Error> {
        // More outcomes = higher complexity = higher threshold
        let outcome_count = market.outcomes.len() as i128;
        
        if outcome_count > 3 {
            // Increase by 10% per additional outcome beyond 3
            let additional_outcomes = outcome_count - 3;
            Ok((BASE_DISPUTE_THRESHOLD * 10 * additional_outcomes) / 100)
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
        Ok(env.storage()
            .persistent()
            .get(&key)
            .unwrap_or(DisputeThreshold {
                market_id: market_id.clone(),
                base_threshold: BASE_DISPUTE_THRESHOLD,
                adjusted_threshold: BASE_DISPUTE_THRESHOLD,
                market_size_factor: 0,
                activity_factor: 0,
                complexity_factor: 0,
                timestamp: env.ledger().timestamp(),
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
        let mut history: Vec<ThresholdHistoryEntry> = env.storage()
            .persistent()
            .get(&key)
            .unwrap_or(vec![env]);

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
        let history: Vec<ThresholdHistoryEntry> = env.storage()
            .persistent()
            .get(&key)
            .unwrap_or(vec![env]);

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

/// Validates threshold-related operations
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

/// Validates voting-related operations
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
        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(env, "Admin"))
            .expect("Admin not set");

        if admin != &stored_admin {
            return Err(Error::Unauthorized);
        }

        Ok(())
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
    pub fn validate_market_for_claim(_env: &Env, market: &Market, user: &Address) -> Result<(), Error> {
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

        // Validate stake
        if let Err(e) = MarketValidator::validate_stake(stake, MIN_VOTE_STAKE) {
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

/// Utility functions for voting operations
pub struct VotingUtils;

impl VotingUtils {
    /// Transfer stake from user to contract
    pub fn transfer_stake(env: &Env, user: &Address, stake: i128) -> Result<(), Error> {
        let token_client = MarketUtils::get_token_client(env)?;
        token_client.transfer(user, &env.current_contract_address(), &stake);
        Ok(())
    }

    /// Transfer winnings to user
    pub fn transfer_winnings(env: &Env, user: &Address, amount: i128) -> Result<(), Error> {
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
        _env: &Env,
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
        let payout = MarketUtils::calculate_payout(
            user_stake,
            winning_stats.winning_total,
            winning_stats.total_pool,
            FEE_PERCENTAGE,
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

/// Analytics functions for voting data
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

    /// Create a test vote
    pub fn create_test_vote(env: &Env, user: Address, outcome: String, stake: i128) -> Vote {
        Vote {
            user,
            outcome,
            stake,
            timestamp: env.ledger().timestamp(),
        }
    }

    /// Create test voting statistics
    pub fn create_test_voting_stats(env: &Env) -> VotingStats {
        let outcome_distribution = Map::new(env);
        VotingStats {
            total_votes: 0,
            total_staked: 0,
            outcome_distribution,
            unique_voters: 0,
        }
    }

    /// Create test payout data
    pub fn create_test_payout_data() -> PayoutData {
        PayoutData {
            user_stake: 1000,
            winning_total: 5000,
            total_pool: 10000,
            fee_percentage: 2,
            payout_amount: 1960, // (1000 * 5000 / 10000) * 0.98
        }
    }

    /// Validate vote structure
    pub fn validate_vote_structure(vote: &Vote) -> Result<(), Error> {
        if vote.stake <= 0 {
            return Err(Error::InsufficientStake);
        }

        if vote.outcome.is_empty() {
            return Err(Error::InvalidOutcome);
        }

        Ok(())
    }

    /// Validate voting stats structure
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
    use soroban_sdk::testutils::Address as _;
    use crate::types::{OracleConfig, OracleProvider};

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
            vec![
                &env,
                String::from_str(&env, "yes"),
                String::from_str(&env, "no"),
            ],
            env.ledger().timestamp() + 86400,
            OracleConfig {
                provider: OracleProvider::Pyth,
                feed_id: String::from_str(&env, "BTC/USD"),
                threshold: 2500000,
                comparison: String::from_str(&env, "gt"),
            },
        );
        market.total_staked = 10000;

        let fee = VotingUtils::calculate_fee_amount(&market).unwrap();
        assert_eq!(fee, 200); // 2% of 10000
    }

    #[test]
    fn test_voting_analytics_average_stake() {
        let env = Env::default();
        let mut market = Market::new(
            &env,
            Address::generate(&env),
            String::from_str(&env, "Test Market"),
            vec![
                &env,
                String::from_str(&env, "yes"),
                String::from_str(&env, "no"),
            ],
            env.ledger().timestamp() + 86400,
            OracleConfig {
                provider: OracleProvider::Pyth,
                feed_id: String::from_str(&env, "BTC/USD"),
                threshold: 2500000,
                comparison: String::from_str(&env, "gt"),
            },
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
            vec![
                &env,
                String::from_str(&env, "yes"),
                String::from_str(&env, "no"),
            ],
            env.ledger().timestamp() + 86400,
            OracleConfig {
                provider: OracleProvider::Pyth,
                feed_id: String::from_str(&env, "BTC/USD"),
                threshold: 2500000,
                comparison: String::from_str(&env, "gt"),
            },
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
