use crate::{
    errors::Error,
    markets::{MarketStateManager},
    types::Market,
    voting::{VotingUtils, MIN_DISPUTE_STAKE, DISPUTE_EXTENSION_HOURS},
};
use soroban_sdk::{
    contracttype, Address, Env, Map, String, Symbol, Vec,
};

// ===== DISPUTE STRUCTURES =====

/// Represents a dispute on a market
#[contracttype]
pub struct Dispute {
    pub user: Address,
    pub market_id: Symbol,
    pub stake: i128,
    pub timestamp: u64,
    pub reason: Option<String>,
    pub status: DisputeStatus,
}

/// Represents the status of a dispute
#[contracttype]
pub enum DisputeStatus {
    Active,
    Resolved,
    Rejected,
    Expired,
}

/// Represents dispute statistics for a market
#[contracttype]
pub struct DisputeStats {
    pub total_disputes: u32,
    pub total_dispute_stakes: i128,
    pub active_disputes: u32,
    pub resolved_disputes: u32,
    pub unique_disputers: u32,
}

/// Represents dispute resolution data
#[contracttype]
pub struct DisputeResolution {
    pub market_id: Symbol,
    pub final_outcome: String,
    pub oracle_weight: i128, // Using i128 instead of f64 for no_std compatibility
    pub community_weight: i128,
    pub dispute_impact: i128,
    pub resolution_timestamp: u64,
}

// ===== DISPUTE MANAGER =====

/// Main dispute manager for handling all dispute operations
pub struct DisputeManager;

impl DisputeManager {
    /// Process a user's dispute of market result
    pub fn process_dispute(
        env: &Env,
        user: Address,
        market_id: Symbol,
        stake: i128,
        reason: Option<String>,
    ) -> Result<(), Error> {
        // Require authentication from the user
        user.require_auth();

        // Get and validate market
        let mut market = MarketStateManager::get_market(env, &market_id)?;
        DisputeValidator::validate_market_for_dispute(env, &market)?;

        // Validate dispute parameters
        DisputeValidator::validate_dispute_parameters(env, &user, &market, stake)?;

        // Process stake transfer
        VotingUtils::transfer_stake(env, &user, stake)?;

        // Create dispute record
        let dispute = Dispute {
            user: user.clone(),
            market_id: market_id.clone(),
            stake,
            timestamp: env.ledger().timestamp(),
            reason,
            status: DisputeStatus::Active,
        };

        // Add dispute to market
        DisputeUtils::add_dispute_to_market(&mut market, dispute)?;

        // Extend market for dispute period
        DisputeUtils::extend_market_for_dispute(&mut market, env)?;

        // Update market in storage
        MarketStateManager::update_market(env, &market_id, &market);

        Ok(())
    }

    /// Resolve a dispute by determining final outcome
    pub fn resolve_dispute(
        env: &Env,
        market_id: Symbol,
        admin: Address,
    ) -> Result<DisputeResolution, Error> {
        // Require authentication from the admin
        admin.require_auth();

        // Validate admin permissions
        DisputeValidator::validate_admin_permissions(env, &admin)?;

        // Get and validate market
        let mut market = MarketStateManager::get_market(env, &market_id)?;
        DisputeValidator::validate_market_for_resolution(env, &market)?;

        // Calculate dispute impact
        let dispute_impact = DisputeAnalytics::calculate_dispute_impact(&market);

        // Determine final outcome with dispute consideration
        let final_outcome = DisputeUtils::determine_final_outcome_with_disputes(env, &market)?;

        // Calculate weights
        let oracle_weight = DisputeAnalytics::calculate_oracle_weight(&market);
        let community_weight = DisputeAnalytics::calculate_community_weight(&market);

        // Create resolution record
        let resolution = DisputeResolution {
            market_id: market_id.clone(),
            final_outcome: final_outcome.clone(),
            oracle_weight,
            community_weight,
            dispute_impact,
            resolution_timestamp: env.ledger().timestamp(),
        };

        // Update market with final outcome
        DisputeUtils::finalize_market_with_resolution(&mut market, final_outcome)?;
        MarketStateManager::update_market(env, &market_id, &market);

        Ok(resolution)
    }

    /// Get dispute statistics for a market
    pub fn get_dispute_stats(env: &Env, market_id: Symbol) -> Result<DisputeStats, Error> {
        let market = MarketStateManager::get_market(env, &market_id)?;
        Ok(DisputeAnalytics::calculate_dispute_stats(&market))
    }

    /// Get all disputes for a market
    pub fn get_market_disputes(env: &Env, market_id: Symbol) -> Result<Vec<Dispute>, Error> {
        let market = MarketStateManager::get_market(env, &market_id)?;
        Ok(DisputeUtils::extract_disputes_from_market(env, &market, market_id))
    }

    /// Check if user has disputed a market
    pub fn has_user_disputed(env: &Env, market_id: Symbol, user: Address) -> Result<bool, Error> {
        let market = MarketStateManager::get_market(env, &market_id)?;
        Ok(DisputeUtils::has_user_disputed(&market, &user))
    }

    /// Get user's dispute stake for a market
    pub fn get_user_dispute_stake(env: &Env, market_id: Symbol, user: Address) -> Result<i128, Error> {
        let market = MarketStateManager::get_market(env, &market_id)?;
        Ok(DisputeUtils::get_user_dispute_stake(&market, &user))
    }
}

// ===== DISPUTE VALIDATOR =====

/// Validates dispute-related operations
pub struct DisputeValidator;

impl DisputeValidator {
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

        // Check if oracle result is available
        if market.oracle_result.is_none() {
            return Err(Error::OracleUnavailable);
        }

        Ok(())
    }

    /// Validate market state for resolution
    pub fn validate_market_for_resolution(_env: &Env, market: &Market) -> Result<(), Error> {
        // Check if market is already resolved
        if market.winning_outcome.is_some() {
            return Err(Error::MarketAlreadyResolved);
        }

        // Check if there are active disputes
        if market.total_dispute_stakes() == 0 {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }

    /// Validate admin permissions
    pub fn validate_admin_permissions(env: &Env, admin: &Address) -> Result<(), Error> {
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

    /// Validate dispute parameters
    pub fn validate_dispute_parameters(
        _env: &Env,
        user: &Address,
        market: &Market,
        stake: i128,
    ) -> Result<(), Error> {
        // Validate stake amount
        if stake < MIN_DISPUTE_STAKE {
            return Err(Error::InsufficientStake);
        }

        // Check if user has already disputed
        if DisputeUtils::has_user_disputed(market, user) {
            return Err(Error::AlreadyDisputed);
        }

        // Check if user has voted (optional requirement)
        if !market.votes.contains_key(user.clone()) {
            // Allow disputes even from non-voters, but could be made optional
        }

        Ok(())
    }

    /// Validate dispute resolution parameters
    pub fn validate_resolution_parameters(
        market: &Market,
        final_outcome: &String,
    ) -> Result<(), Error> {
        // Validate that final outcome is one of the valid outcomes
        if !market.outcomes.contains(final_outcome) {
            return Err(Error::InvalidOutcome);
        }

        Ok(())
    }
}

// ===== DISPUTE UTILITIES =====

/// Utility functions for dispute operations
pub struct DisputeUtils;

impl DisputeUtils {
    /// Add dispute to market
    pub fn add_dispute_to_market(market: &mut Market, dispute: Dispute) -> Result<(), Error> {
        // Add dispute stake to market
        let current_stake = market.dispute_stakes.get(dispute.user.clone()).unwrap_or(0);
        market.dispute_stakes.set(dispute.user, current_stake + dispute.stake);

        // Update total dispute stakes - this is calculated automatically by the method
        // No need to assign it back since it's a computed value

        Ok(())
    }

    /// Extend market for dispute period
    pub fn extend_market_for_dispute(market: &mut Market, _env: &Env) -> Result<(), Error> {
        let extension_seconds = (DISPUTE_EXTENSION_HOURS as u64) * 3600;
        market.end_time += extension_seconds;
        Ok(())
    }

    /// Determine final outcome considering disputes
    pub fn determine_final_outcome_with_disputes(
        env: &Env,
        market: &Market,
    ) -> Result<String, Error> {
        let oracle_result = market
            .oracle_result
            .as_ref()
            .ok_or(Error::OracleUnavailable)?;

        // If there are significant disputes, consider community consensus more heavily
        let dispute_impact = DisputeAnalytics::calculate_dispute_impact(market);
        
        if dispute_impact > 30 { // Using integer percentage (30% = 30)
            // High dispute impact - give more weight to community consensus
            let community_consensus = DisputeAnalytics::calculate_community_consensus(env, market);
            if community_consensus.confidence > 70 { // Using integer percentage (70% = 70)
                return Ok(community_consensus.outcome);
            }
        }

        // Default to oracle result
        Ok(oracle_result.clone())
    }

    /// Finalize market with resolution
    pub fn finalize_market_with_resolution(
        market: &mut Market,
        final_outcome: String,
    ) -> Result<(), Error> {
        // Validate the final outcome
        DisputeValidator::validate_resolution_parameters(market, &final_outcome)?;

        // Set the winning outcome
        market.winning_outcome = Some(final_outcome);

        Ok(())
    }

    /// Extract disputes from market
    pub fn extract_disputes_from_market(env: &Env, market: &Market, market_id: Symbol) -> Vec<Dispute> {
        let mut disputes = Vec::new(env);
        
        for (user, stake) in market.dispute_stakes.iter() {
            if stake > 0 {
                let dispute = Dispute {
                    user: user.clone(),
                    market_id: market_id.clone(),
                    stake,
                    timestamp: env.ledger().timestamp(),
                    reason: None,
                    status: DisputeStatus::Active,
                };
                disputes.push_back(dispute);
            }
        }

        disputes
    }

    /// Check if user has disputed
    pub fn has_user_disputed(market: &Market, user: &Address) -> bool {
        market.dispute_stakes.get(user.clone()).unwrap_or(0) > 0
    }

    /// Get user's dispute stake
    pub fn get_user_dispute_stake(market: &Market, user: &Address) -> i128 {
        market.dispute_stakes.get(user.clone()).unwrap_or(0)
    }

    /// Calculate dispute impact on market resolution
    pub fn calculate_dispute_impact(market: &Market) -> f64 {
        let total_staked = market.total_staked;
        let total_disputes = market.total_dispute_stakes();

        if total_staked == 0 {
            return 0.0;
        }

        (total_disputes as f64) / (total_staked as f64)
    }
}

// ===== DISPUTE ANALYTICS =====

/// Analytics functions for dispute data
pub struct DisputeAnalytics;

impl DisputeAnalytics {
    /// Calculate dispute statistics for a market
    pub fn calculate_dispute_stats(market: &Market) -> DisputeStats {
        let mut active_disputes = 0;
        let mut resolved_disputes = 0;
        let mut unique_disputers = 0;

        for (_, stake) in market.dispute_stakes.iter() {
            if stake > 0 {
                unique_disputers += 1;
                if market.winning_outcome.is_none() {
                    active_disputes += 1;
                } else {
                    resolved_disputes += 1;
                }
            }
        }

        DisputeStats {
            total_disputes: active_disputes + resolved_disputes,
            total_dispute_stakes: market.total_dispute_stakes(),
            active_disputes,
            resolved_disputes,
            unique_disputers,
        }
    }

    /// Calculate dispute impact on market
    pub fn calculate_dispute_impact(market: &Market) -> i128 {
        let impact = DisputeUtils::calculate_dispute_impact(market);
        (impact * 100.0) as i128 // Convert to integer percentage
    }

    /// Calculate oracle weight in resolution
    pub fn calculate_oracle_weight(market: &Market) -> i128 {
        let dispute_impact = Self::calculate_dispute_impact(market) as f64 / 100.0; // Convert back to decimal
        
        // Oracle weight decreases with dispute impact
        let base_oracle_weight = 0.7;
        let dispute_penalty = dispute_impact * 0.3;
        
        let weight = (base_oracle_weight - dispute_penalty).max(0.3);
        (weight * 100.0) as i128 // Convert to integer percentage
    }

    /// Calculate community weight in resolution
    pub fn calculate_community_weight(market: &Market) -> i128 {
        let dispute_impact = Self::calculate_dispute_impact(market) as f64 / 100.0; // Convert back to decimal
        
        // Community weight increases with dispute impact
        let base_community_weight = 0.3;
        let dispute_boost = dispute_impact * 0.4;
        
        let weight = (base_community_weight + dispute_boost).min(0.7);
        (weight * 100.0) as i128 // Convert to integer percentage
    }

    /// Calculate community consensus
    pub fn calculate_community_consensus(env: &Env, market: &Market) -> CommunityConsensus {
        let mut outcome_totals = Map::new(env);
        let mut total_votes = 0;

        // Calculate total stakes for each outcome
        for (user, outcome) in market.votes.iter() {
            let stake = market.stakes.get(user).unwrap_or(0);
            let current_total = outcome_totals.get(outcome.clone()).unwrap_or(0);
            outcome_totals.set(outcome, current_total + stake);
            total_votes += stake;
        }

        // Find the outcome with highest stake
        let mut winning_outcome = String::from_str(env, "");
        let mut max_stake = 0;

        for (outcome, stake) in outcome_totals.iter() {
            if stake > max_stake {
                max_stake = stake;
                winning_outcome = outcome;
            }
        }

        let confidence = if total_votes > 0 {
            (max_stake as i128) * 100 / total_votes // Using integer percentage instead of f64
        } else {
            0
        };

        CommunityConsensus {
            outcome: winning_outcome,
            confidence,
            total_votes,
        }
    }

    /// Get top disputers by stake amount
    pub fn get_top_disputers(env: &Env, market: &Market, _limit: usize) -> Vec<(Address, i128)> {
        let mut disputers: Vec<(Address, i128)> = Vec::new(env);
        
        for (user, stake) in market.dispute_stakes.iter() {
            if stake > 0 {
                disputers.push_back((user, stake));
            }
        }

        // Note: Sorting is not available in no_std, so we return as-is
        // In a real implementation, you might want to implement a simple sort
        disputers
    }

    /// Calculate dispute participation rate
    pub fn calculate_dispute_participation_rate(market: &Market) -> f64 {
        let total_voters = market.votes.len();
        let total_disputers = market.dispute_stakes.len();

        if total_voters == 0 {
            return 0.0;
        }

        (total_disputers as f64) / (total_voters as f64)
    }
}

// ===== DISPUTE TESTING UTILITIES =====

#[cfg(test)]
pub mod testing {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    /// Create a test dispute
    pub fn create_test_dispute(env: &Env, user: Address, market_id: Symbol, stake: i128) -> Dispute {
        Dispute {
            user,
            market_id,
            stake,
            timestamp: env.ledger().timestamp(),
            reason: Some(String::from_str(env, "Test dispute")),
            status: DisputeStatus::Active,
        }
    }

    /// Create test dispute statistics
    pub fn create_test_dispute_stats() -> DisputeStats {
        DisputeStats {
            total_disputes: 0,
            total_dispute_stakes: 0,
            active_disputes: 0,
            resolved_disputes: 0,
            unique_disputers: 0,
        }
    }

    /// Create test dispute resolution
    pub fn create_test_dispute_resolution(env: &Env, market_id: Symbol) -> DisputeResolution {
        DisputeResolution {
            market_id,
            final_outcome: String::from_str(env, "yes"),
            oracle_weight: 70, // Using integer percentage
            community_weight: 30, // Using integer percentage
            dispute_impact: 10, // Using integer percentage
            resolution_timestamp: env.ledger().timestamp(),
        }
    }

    /// Validate dispute structure
    pub fn validate_dispute_structure(dispute: &Dispute) -> Result<(), Error> {
        if dispute.stake <= 0 {
            return Err(Error::InsufficientStake);
        }

        Ok(())
    }

    /// Validate dispute stats structure
    pub fn validate_dispute_stats(stats: &DisputeStats) -> Result<(), Error> {
        if stats.total_dispute_stakes < 0 {
            return Err(Error::InvalidInput);
        }

        if stats.total_disputes < stats.unique_disputers {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }
}

// ===== HELPER STRUCTURES =====

/// Represents community consensus data
pub struct CommunityConsensus {
    pub outcome: String,
    pub confidence: i128, // Using i128 instead of f64 for no_std compatibility
    pub total_votes: i128,
}

// ===== MODULE TESTS =====

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    fn create_test_market(env: &Env, end_time: u64) -> Market {
        let mut outcomes = Vec::new(env);
        outcomes.push_back(String::from_str(env, "yes"));
        outcomes.push_back(String::from_str(env, "no"));
        
        Market::new(
            env,
            Address::generate(env),
            String::from_str(env, "Test Market"),
            outcomes,
            end_time,
            crate::types::OracleConfig::new(
                crate::types::OracleProvider::Pyth,
                String::from_str(env, "BTC/USD"),
                2500000,
                String::from_str(env, "gt"),
            ),
        )
    }

    #[test]
    fn test_dispute_validator_market_validation() {
        let env = Env::default();
        let mut market = create_test_market(&env, env.ledger().timestamp() + 86400);

        // Market not ended - should fail
        assert!(DisputeValidator::validate_market_for_dispute(&env, &market).is_err());

        // Set market as ended
        market.end_time = env.ledger().timestamp().saturating_sub(1);
        
        // No oracle result - should fail
        assert!(DisputeValidator::validate_market_for_dispute(&env, &market).is_err());

        // Add oracle result
        market.oracle_result = Some(String::from_str(&env, "yes"));
        
        // Should pass
        assert!(DisputeValidator::validate_market_for_dispute(&env, &market).is_ok());
    }

    #[test]
    fn test_dispute_validator_stake_validation() {
        let env = Env::default();
        let user = Address::generate(&env);
        let mut market = create_test_market(&env, env.ledger().timestamp().saturating_sub(1));
        market.oracle_result = Some(String::from_str(&env, "yes"));

        // Valid stake
        assert!(DisputeValidator::validate_dispute_parameters(&env, &user, &market, MIN_DISPUTE_STAKE).is_ok());
        
        // Invalid stake
        assert!(DisputeValidator::validate_dispute_parameters(&env, &user, &market, MIN_DISPUTE_STAKE - 1).is_err());
    }

    #[test]
    fn test_dispute_utils_impact_calculation() {
        let env = Env::default();
        let mut market = create_test_market(&env, env.ledger().timestamp() + 86400);

        market.total_staked = 10000;
        // Add dispute stakes to trigger the calculation
        let user = Address::generate(&env);
        market.dispute_stakes.set(user, 2000);

        let impact = DisputeUtils::calculate_dispute_impact(&market);
        assert_eq!(impact, 0.2); // 2000 / 10000
    }

    #[test]
    fn test_dispute_analytics_stats() {
        let env = Env::default();
        let mut market = create_test_market(&env, env.ledger().timestamp() + 86400);

        let user = Address::generate(&env);
        market.dispute_stakes.set(user, 1000);

        let stats = DisputeAnalytics::calculate_dispute_stats(&market);
        assert_eq!(stats.total_disputes, 1);
        assert_eq!(stats.total_dispute_stakes, 1000);
        assert_eq!(stats.unique_disputers, 1);
        assert_eq!(stats.active_disputes, 1);
    }

    #[test]
    fn test_testing_utilities() {
        let env = Env::default();
        let user = Address::generate(&env);
        
        let dispute = testing::create_test_dispute(
            &env,
            user,
            Symbol::new(&env, "market"),
            1000,
        );

        assert!(testing::validate_dispute_structure(&dispute).is_ok());
        
        let stats = testing::create_test_dispute_stats();
        assert!(testing::validate_dispute_stats(&stats).is_ok());
    }
} 