use crate::{
    errors::{Error, ErrorCategory},
    markets::{MarketAnalytics, MarketCreator, MarketStateManager, MarketUtils, MarketValidator},
    types::{Market, OracleConfig, OracleProvider},
};
use soroban_sdk::{
    contracttype, panic_with_error, vec, Address, Env, Map, String, Symbol, Vec,
};

// ===== CONSTANTS =====

/// Minimum stake amount for voting (0.1 XLM)
pub const MIN_VOTE_STAKE: i128 = 1_000_000;

/// Minimum stake amount for disputes (10 XLM)
pub const MIN_DISPUTE_STAKE: i128 = 10_000_000;

/// Platform fee percentage (2%)
pub const FEE_PERCENTAGE: i128 = 2;

/// Dispute extension period in hours
pub const DISPUTE_EXTENSION_HOURS: u32 = 24;

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

        // Validate dispute stake
        VotingValidator::validate_dispute_stake(stake)?;

        // Process stake transfer
        VotingUtils::transfer_stake(env, &user, stake)?;

        // Add dispute stake and extend market (pass market_id for event emission)
        MarketStateManager::add_dispute_stake(&mut market, user, stake, Some(&market_id));
        MarketStateManager::extend_for_dispute(&mut market, env, DISPUTE_EXTENSION_HOURS.into());
        MarketStateManager::update_market(env, &market_id, &market);

        Ok(())
    }

    /// Process winnings claim for a user
    pub fn process_claim(
        env: &Env,
        user: Address,
        market_id: Symbol,
    ) -> Result<i128, Error> {
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

    /// Collect platform fees from a market
    pub fn collect_fees(
        env: &Env,
        admin: Address,
        market_id: Symbol,
    ) -> Result<i128, Error> {
        // Require authentication from the admin
        admin.require_auth();
        
        // Validate admin permissions
        VotingValidator::validate_admin_authentication(env, &admin)?;

        // Get and validate market
        let mut market = MarketStateManager::get_market(env, &market_id)?;
        VotingValidator::validate_market_for_fee_collection(&market)?;

        // Calculate fee amount
        let fee_amount = VotingUtils::calculate_fee_amount(&market)?;

        // Transfer fees to admin
        VotingUtils::transfer_fees(env, &admin, fee_amount)?;

        // Mark fees as collected
        MarketStateManager::mark_fees_collected(&mut market, Some(&market_id));
        MarketStateManager::update_market(env, &market_id, &market);

        Ok(fee_amount)
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
    pub fn validate_market_for_claim(
        env: &Env,
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
    pub fn validate_market_for_fee_collection(market: &Market) -> Result<(), Error> {
        // Check if fees already collected
        if market.fee_collected {
            return Err(Error::FeeAlreadyCollected);
        }

        // Check if market is resolved
        if market.winning_outcome.is_none() {
            return Err(Error::MarketNotResolved);
        }

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

    /// Transfer fees to admin
    pub fn transfer_fees(env: &Env, admin: &Address, amount: i128) -> Result<(), Error> {
        let token_client = MarketUtils::get_token_client(env)?;
        token_client.transfer(&env.current_contract_address(), admin, &amount);
        Ok(())
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
        let payout = MarketUtils::calculate_payout(
            user_stake,
            winning_stats.winning_total,
            winning_stats.total_pool,
            FEE_PERCENTAGE,
        )?;

        Ok(payout)
    }

    /// Calculate fee amount for a market
    pub fn calculate_fee_amount(market: &Market) -> Result<i128, Error> {
        let fee = (market.total_staked * FEE_PERCENTAGE) / 100;
        Ok(fee)
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

        let concentration = (total_squared_stakes as f64) / ((market.total_staked * market.total_staked) as f64);
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
            vec![&env, String::from_str(&env, "yes"), String::from_str(&env, "no")],
            env.ledger().timestamp() + 86400,
            OracleConfig::new(
                OracleProvider::Pyth,
                String::from_str(&env, "BTC/USD"),
                2500000,
                String::from_str(&env, "gt"),
            ),
            crate::types::MarketState::Active
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
            vec![&env, String::from_str(&env, "yes"), String::from_str(&env, "no")],
            env.ledger().timestamp() + 86400,
            OracleConfig::new(
                OracleProvider::Pyth,
                String::from_str(&env, "BTC/USD"),
                2500000,
                String::from_str(&env, "gt"),
            ),
            crate::types::MarketState::Active
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
            vec![&env, String::from_str(&env, "yes"), String::from_str(&env, "no")],
            env.ledger().timestamp() + 86400,
            OracleConfig::new(
                OracleProvider::Pyth,
                String::from_str(&env, "BTC/USD"),
                2500000,
                String::from_str(&env, "gt"),
            ),
            crate::types::MarketState::Active
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
        
        let vote = testing::create_test_vote(
            &env,
            user,
            String::from_str(&env, "yes"),
            1000,
        );

        assert!(testing::validate_vote_structure(&vote).is_ok());
        
        let stats = testing::create_test_voting_stats(&env);
        assert!(testing::validate_voting_stats(&stats).is_ok());
    }
}