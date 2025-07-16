#![allow(dead_code)]

use soroban_sdk::{contracttype, token, vec, Address, Env, Map, String, Symbol, Vec};

use crate::errors::Error;
use crate::types::*;
// Oracle imports removed - not currently used

/// Market management system for Predictify Hybrid contract
///
/// This module provides a comprehensive market management system with:
/// - Market creation and configuration functions
/// - Market state management and validation
/// - Market analytics and statistics
/// - Market helper utilities and testing functions
/// - Market resolution and dispute handling

// ===== MARKET CREATION =====

/// Market creation utilities
pub struct MarketCreator;

impl MarketCreator {
    /// Create a new market with full configuration
    pub fn create_market(env: &Env, admin: Address, question: String, outcomes: Vec<String>, duration_days: u32, oracle_config: OracleConfig) -> Result<Symbol, Error> {
        // Validate market parameters
        MarketValidator::validate_market_params(env, &question, &outcomes, duration_days)?;

        // Validate oracle configuration
        MarketValidator::validate_oracle_config(env, &oracle_config)?;

        // Generate unique market ID
        let market_id = MarketUtils::generate_market_id(env);

        // Calculate end time
        let end_time = MarketUtils::calculate_end_time(env, duration_days);

        // Create market instance
        let market = Market::new(env, admin.clone(), question, outcomes, end_time, oracle_config, MarketState::Active);
        
        // Process market creation fee
        MarketUtils::process_creation_fee(env, &admin)?;
        
        // Store market
        env.storage().persistent().set(&market_id, &market);

        Ok(market_id)
    }

    /// Create a market with Reflector oracle
    pub fn create_reflector_market(_env: &Env, admin: Address, question: String, outcomes: Vec<String>, duration_days: u32, asset_symbol: String, threshold: i128, comparison: String) -> Result<Symbol, Error> {
        let oracle_config = OracleConfig {
            provider: OracleProvider::Reflector,
            feed_id: asset_symbol,
            threshold,
            comparison,
        };

        Self::create_market(_env, admin, question, outcomes, duration_days, oracle_config)
    }

    /// Create a market with Pyth oracle
    pub fn create_pyth_market(_env: &Env, admin: Address, question: String, outcomes: Vec<String>, duration_days: u32, feed_id: String, threshold: i128, comparison: String) -> Result<Symbol, Error> {
        let oracle_config = OracleConfig {
            provider: OracleProvider::Pyth,
            feed_id,
            threshold,
            comparison,
        };

        Self::create_market(_env, admin, question, outcomes, duration_days, oracle_config)
    }

    /// Create a market with Reflector oracle for specific assets
    pub fn create_reflector_asset_market(_env: &Env, admin: Address, question: String, outcomes: Vec<String>, duration_days: u32, asset_symbol: String, threshold: i128, comparison: String) -> Result<Symbol, Error> {
        Self::create_reflector_market(_env, admin, question, outcomes, duration_days, asset_symbol, threshold, comparison)
    }
}

// ===== MARKET VALIDATION =====

/// Market validation utilities
pub struct MarketValidator;

impl MarketValidator {
    /// Validate market creation parameters

    pub fn validate_market_params(
        _env: &Env,
        question: &String,
        outcomes: &Vec<String>,
        duration_days: u32,
    ) -> Result<(), Error> {

        // Validate question is not empty
        if question.is_empty() {
            return Err(Error::InvalidQuestion);
        }

        // Validate outcomes
        if outcomes.len() < 2 {
            return Err(Error::InvalidOutcomes);
        }

        for outcome in outcomes.iter() {
            if outcome.is_empty() {
                return Err(Error::InvalidOutcomes);
            }
        }

        // Validate duration
        if duration_days == 0 || duration_days > 365 {
            return Err(Error::InvalidDuration);
        }

        Ok(())
    }

    /// Validate oracle configuration
    pub fn validate_oracle_config(_env: &Env, oracle_config: &OracleConfig) -> Result<(), Error> {
        oracle_config.validate(_env)
    }

    /// Validate market state for voting
    pub fn validate_market_for_voting(_env: &Env, market: &Market) -> Result<(), Error> {
        let current_time = _env.ledger().timestamp();

        if current_time >= market.end_time {
            return Err(Error::MarketClosed);
        }

        if market.oracle_result.is_some() {
            return Err(Error::MarketAlreadyResolved);
        }

        Ok(())
    }

    /// Validate market state for resolution
    pub fn validate_market_for_resolution(_env: &Env, market: &Market) -> Result<(), Error> {
        let current_time = _env.ledger().timestamp();

        if current_time < market.end_time {
            return Err(Error::MarketClosed);
        }

        if market.oracle_result.is_none() {
            return Err(Error::OracleUnavailable);
        }

        Ok(())
    }

    /// Validate outcome for a market


    pub fn validate_outcome(
        _env: &Env,
        outcome: &String,
        market_outcomes: &Vec<String>,
    ) -> Result<(), Error> {

        for valid_outcome in market_outcomes.iter() {
            if *outcome == valid_outcome {
                return Ok(());
            }
        }

        Err(Error::InvalidOutcome)
    }

    /// Validate stake amount
    pub fn validate_stake(stake: i128, min_stake: i128) -> Result<(), Error> {
        if stake < min_stake {
            return Err(Error::InsufficientStake);
        }

        if stake <= 0 {
            return Err(Error::InvalidState);
        }

        Ok(())
    }
}

// ===== MARKET STATE MANAGEMENT =====

/// Market state management utilities
pub struct MarketStateManager;

impl MarketStateManager {
    /// Get market from storage
    pub fn get_market(_env: &Env, market_id: &Symbol) -> Result<Market, Error> {
        _env.storage()
            .persistent()
            .get(market_id)
            .ok_or(Error::MarketNotFound)
    }

    /// Update market in storage
    pub fn update_market(_env: &Env, market_id: &Symbol, market: &Market) {
        _env.storage().persistent().set(market_id, market);
    }

    /// Remove market from storage
    pub fn remove_market(env: &Env, market_id: &Symbol) {
        let mut market = match Self::get_market(env, market_id) {
            Ok(m) => m,
            Err(_) => return,
        };
        if market.state != MarketState::Closed {
            MarketStateLogic::validate_state_transition(market.state, MarketState::Closed).unwrap();
            let old_state = market.state;
            market.state = MarketState::Closed;
            MarketStateLogic::emit_state_change_event(env, market_id, old_state, market.state);
            Self::update_market(env, market_id, &market);
        }
        env.storage().persistent().remove(market_id);
    }

    /// Add vote to market
    pub fn add_vote(market: &mut Market, user: Address, outcome: String, stake: i128, market_id: Option<&Symbol>) {
        MarketStateLogic::check_function_access_for_state("vote", market.state).unwrap();
        market.votes.set(user.clone(), outcome);
        market.stakes.set(user.clone(), stake);
        market.total_staked += stake;
        // No state change for voting
    }

    /// Add dispute stake to market
    pub fn add_dispute_stake(market: &mut Market, user: Address, stake: i128, market_id: Option<&Symbol>) {
        MarketStateLogic::check_function_access_for_state("dispute", market.state).unwrap();
        let existing_stake = market.dispute_stakes.get(user.clone()).unwrap_or(0);
        market.dispute_stakes.set(user, existing_stake + stake);
        // State transition: Ended -> Disputed
        if market.state == MarketState::Ended {
            MarketStateLogic::validate_state_transition(market.state, MarketState::Disputed).unwrap();
            let old_state = market.state;
            market.state = MarketState::Disputed;
            let env = &market.votes.env();
            let owned_event_id = market_id.cloned().unwrap_or_else(|| Symbol::new(env, "unknown_market_id"));
            MarketStateLogic::emit_state_change_event(env, &owned_event_id, old_state, market.state);
        }
    }

    /// Mark user as claimed
    pub fn mark_claimed(market: &mut Market, user: Address, _market_id: Option<&Symbol>) {
        MarketStateLogic::check_function_access_for_state("claim", market.state).unwrap();
        market.claimed.set(user, true);
    }

    /// Set oracle result
    pub fn set_oracle_result(market: &mut Market, result: String) {
        market.oracle_result = Some(result);
    }

    /// Set winning outcome
    pub fn set_winning_outcome(market: &mut Market, outcome: String, market_id: Option<&Symbol>) {
        MarketStateLogic::check_function_access_for_state("resolve", market.state).unwrap();
        let old_state = market.state;
        market.winning_outcome = Some(outcome);
        // State transition: Ended/Disputed -> Resolved
        if market.state == MarketState::Ended || market.state == MarketState::Disputed {
            MarketStateLogic::validate_state_transition(market.state, MarketState::Resolved).unwrap();
            market.state = MarketState::Resolved;
            let env = &market.votes.env();
            let owned_event_id = market_id.cloned().unwrap_or_else(|| Symbol::new(env, "unknown_market_id"));
            MarketStateLogic::emit_state_change_event(env, &owned_event_id, old_state, market.state);
        }
    }

    /// Mark fees as collected
    pub fn mark_fees_collected(market: &mut Market, market_id: Option<&Symbol>) {
        MarketStateLogic::check_function_access_for_state("close", market.state).unwrap();
        let old_state = market.state;
        // State transition: Resolved -> Closed
        if market.state == MarketState::Resolved {
            MarketStateLogic::validate_state_transition(market.state, MarketState::Closed).unwrap();
            market.state = MarketState::Closed;
            let env = &market.votes.env();
            let owned_event_id = market_id.cloned().unwrap_or_else(|| Symbol::new(env, "unknown_market_id"));
            MarketStateLogic::emit_state_change_event(env, &owned_event_id, old_state, market.state);
        }
        market.fee_collected = true;
    }

    /// Extend market end time for disputes
    pub fn extend_for_dispute(market: &mut Market, _env: &Env, extension_hours: u64) {
        let current_time = _env.ledger().timestamp();
        let extension_seconds = extension_hours * 60 * 60;

        if market.end_time < current_time + extension_seconds {
            market.end_time = current_time + extension_seconds;
        }
    }
}

// ===== MARKET ANALYTICS =====

/// Market analytics and statistics utilities
pub struct MarketAnalytics;

impl MarketAnalytics {
    /// Get market statistics
    pub fn get_market_stats(market: &Market) -> MarketStats {
        let total_votes = market.votes.len() as u32;
        let total_staked = market.total_staked;
        let total_dispute_stakes = market.total_dispute_stakes();

        // Calculate outcome distribution
        let mut outcome_stats = Map::new(&market.votes.env());
        for (_, outcome) in market.votes.iter() {
            let count = outcome_stats.get(outcome.clone()).unwrap_or(0);
            outcome_stats.set(outcome.clone(), count + 1);
        }

        MarketStats {
            total_votes,
            total_staked,
            total_dispute_stakes,
            outcome_distribution: outcome_stats,
        }
    }

    /// Calculate winning outcome statistics
    pub fn calculate_winning_stats(market: &Market, winning_outcome: &String) -> WinningStats {
        let mut winning_total = 0;
        let mut winning_voters = 0;

        for (user, outcome) in market.votes.iter() {
            if &outcome == winning_outcome {
                winning_total += market.stakes.get(user.clone()).unwrap_or(0);
                winning_voters += 1;
            }
        }

        WinningStats {
            winning_outcome: winning_outcome.clone(),
            winning_total,
            winning_voters,
            total_pool: market.total_staked,
        }
    }

    /// Get user participation statistics
    pub fn get_user_stats(market: &Market, user: &Address) -> UserStats {
        let has_voted = market.votes.contains_key(user.clone());
        let stake = market.stakes.get(user.clone()).unwrap_or(0);
        let dispute_stake = market.dispute_stakes.get(user.clone()).unwrap_or(0);
        let has_claimed = market.claimed.get(user.clone()).unwrap_or(false);
        let voted_outcome = market.votes.get(user.clone());

        UserStats {
            has_voted,
            stake,
            dispute_stake,
            has_claimed,
            voted_outcome,
        }
    }

    /// Calculate community consensus
    pub fn calculate_community_consensus(market: &Market) -> CommunityConsensus {
        let mut vote_counts: Map<String, u32> = Map::new(&market.votes.env());

        for (_, outcome) in market.votes.iter() {
            let count = vote_counts.get(outcome.clone()).unwrap_or(0);
            vote_counts.set(outcome.clone(), count + 1);
        }

        let mut consensus_outcome = String::from_str(&market.votes.env(), "");
        let mut max_votes = 0;
        let mut total_votes = 0;

        for (outcome, count) in vote_counts.iter() {
            total_votes += count;
            if count > max_votes {
                max_votes = count;
                consensus_outcome = outcome.clone();
            }
        }

        let consensus_percentage = if total_votes > 0 {
            (max_votes * 100) / total_votes
        } else {
            0
        };

        CommunityConsensus {
            outcome: consensus_outcome,
            votes: max_votes,
            total_votes,
            percentage: consensus_percentage,
        }
    }
    
    /// Calculate basic analytics for a market
    pub fn calculate_basic_analytics(market: &Market) -> MarketAnalytics {
        // This is a placeholder implementation
        // In a real implementation, you would calculate comprehensive analytics
        MarketAnalytics
    }
}

// ===== MARKET UTILITIES =====

/// General market utilities
pub struct MarketUtils;

impl MarketUtils {
    /// Generate unique market ID
    pub fn generate_market_id(_env: &Env) -> Symbol {
        let counter_key = Symbol::new(_env, "MarketCounter");
        let counter: u32 = _env.storage().persistent().get(&counter_key).unwrap_or(0);
        let new_counter = counter + 1;
        _env.storage().persistent().set(&counter_key, &new_counter);

        Symbol::new(_env, "market")
    }

    /// Calculate market end time
    pub fn calculate_end_time(_env: &Env, duration_days: u32) -> u64 {
        let seconds_per_day: u64 = 24 * 60 * 60;
        let duration_seconds: u64 = (duration_days as u64) * seconds_per_day;
        _env.ledger().timestamp() + duration_seconds
    }

    /// Process market creation fee (moved to fees module)
    /// This function is deprecated and should use FeeManager::process_creation_fee instead
    pub fn process_creation_fee(_env: &Env, admin: &Address) -> Result<(), Error> {
        // Delegate to the fees module
        crate::fees::FeeManager::process_creation_fee(_env, admin)
    }

    /// Get token client for market operations
    pub fn get_token_client(_env: &Env) -> Result<token::Client, Error> {
        let token_id: Address = _env
            .storage()
            .persistent()
            .get(&Symbol::new(_env, "TokenID"))
            .ok_or(Error::InvalidState)?;

        Ok(token::Client::new(_env, &token_id))
    }

    /// Calculate payout for winning user
    pub fn calculate_payout(
        user_stake: i128,
        winning_total: i128,
        total_pool: i128,
        fee_percentage: i128,
    ) -> Result<i128, Error> {
        if winning_total == 0 {
            return Err(Error::NothingToClaim);
        }

        let user_share = (user_stake * (100 - fee_percentage)) / 100;
        let payout = (user_share * total_pool) / winning_total;

        Ok(payout)
    }

    /// Determine final market result using hybrid algorithm
    pub fn determine_final_result(
        _env: &Env,
        oracle_result: &String,
        community_consensus: &CommunityConsensus,
    ) -> String {
        if oracle_result == &community_consensus.outcome {
            // If both agree, use that outcome
            oracle_result.clone()
        } else {
            // If they disagree, check if community consensus is strong
            if community_consensus.percentage > 50 && community_consensus.total_votes >= 5 {
                // Apply 70-30 weighting using pseudo-random selection
                let timestamp = _env.ledger().timestamp();
                let sequence = _env.ledger().sequence();
                let combined = timestamp as u128 + sequence as u128;
                let random_value = (combined % 100) as u32;

                if random_value < 30 {
                    // 30% chance to choose community result
                    community_consensus.outcome.clone()
                } else {
                    // 70% chance to choose oracle result
                    oracle_result.clone()
                }
            } else {
                // Not enough community consensus, use oracle result
                oracle_result.clone()
            }
        }
    }
}

// ===== MARKET STATISTICS TYPES =====

/// Market statistics
#[contracttype]
#[derive(Clone, Debug)]
pub struct MarketStats {
    pub total_votes: u32,
    pub total_staked: i128,
    pub total_dispute_stakes: i128,
    pub outcome_distribution: Map<String, u32>,
}

/// Winning outcome statistics
#[derive(Clone, Debug)]
pub struct WinningStats {
    pub winning_outcome: String,
    pub winning_total: i128,
    pub winning_voters: u32,
    pub total_pool: i128,
}

/// User participation statistics
#[derive(Clone, Debug)]
pub struct UserStats {
    pub has_voted: bool,
    pub stake: i128,
    pub dispute_stake: i128,
    pub has_claimed: bool,
    pub voted_outcome: Option<String>,
}

/// Community consensus statistics
#[derive(Clone, Debug)]
#[contracttype]
pub struct CommunityConsensus {
    pub outcome: String,
    pub votes: u32,
    pub total_votes: u32,
    pub percentage: u32,
}

// ===== MARKET TESTING UTILITIES =====

/// Market testing utilities
pub struct MarketTestHelpers;

impl MarketTestHelpers {
    /// Create a test market configuration
    pub fn create_test_market_config(_env: &Env) -> MarketCreationParams {
        MarketCreationParams::new(
            Address::from_str(
                _env,
                "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            ),
            String::from_str(_env, "Will BTC go above $25,000 by December 31?"),
            vec![
                _env,
                String::from_str(_env, "yes"),
                String::from_str(_env, "no"),
            ],
            30,
            OracleConfig::new(
                OracleProvider::Pyth,
                String::from_str(_env, "BTC/USD"),
                25_000_00,
                String::from_str(_env, "gt"),
            ),
            1_000_000, // Creation fee: 1 XLM
        )
    }

    /// Create a test market
    pub fn create_test_market(_env: &Env) -> Result<Symbol, Error> {
        let config = Self::create_test_market_config(_env);

        MarketCreator::create_market(_env, config.admin, config.question, config.outcomes, config.duration_days, config.oracle_config)
    }

    /// Add test vote to market
    pub fn add_test_vote(
        env: &Env,
        market_id: &Symbol,
        user: Address,
        outcome: String,
        stake: i128,
    ) -> Result<(), Error> {
        let mut market = MarketStateManager::get_market(env, market_id)?;

        MarketValidator::validate_market_for_voting(env, &market)?;
        MarketValidator::validate_outcome(env, &outcome, &market.outcomes)?;
        MarketValidator::validate_stake(stake, 1_000_000)?; // 0.1 XLM minimum

        // Transfer stake
        let token_client = MarketUtils::get_token_client(env)?;
        token_client.transfer(&user, &env.current_contract_address(), &stake);

        // Add vote
        MarketStateManager::add_vote(&mut market, user, outcome, stake, None);
        MarketStateManager::update_market(env, market_id, &market);


        Ok(())
    }

    /// Simulate market resolution
    pub fn simulate_market_resolution(
        env: &Env,
        market_id: &Symbol,
        oracle_result: String,
    ) -> Result<String, Error> {
        let mut market = MarketStateManager::get_market(env, market_id)?;

        MarketValidator::validate_market_for_resolution(env, &market)?;

        // Set oracle result
        MarketStateManager::set_oracle_result(&mut market, oracle_result.clone());

        // Calculate community consensus
        let community_consensus = MarketAnalytics::calculate_community_consensus(&market);

        // Determine final result
        let final_result =
            MarketUtils::determine_final_result(env, &oracle_result, &community_consensus);

        // Set winning outcome
        MarketStateManager::set_winning_outcome(&mut market, final_result.clone(), None);
        MarketStateManager::update_market(env, market_id, &market);

        Ok(final_result)
    }
}

// ===== MARKET STATE LOGIC =====

pub struct MarketStateLogic;

impl MarketStateLogic {
    /// Validate allowed state transitions
    pub fn validate_state_transition(from: MarketState, to: MarketState) -> Result<(), Error> {
        use MarketState::*;
        let allowed = match from {
            Active => matches!(to, Ended | Cancelled | Closed | Disputed),
            Ended => matches!(to, Resolved | Disputed | Closed | Cancelled),
            Disputed => matches!(to, Resolved | Closed | Cancelled),
            Resolved => matches!(to, Closed),
            Closed => false,
            Cancelled => false,
        };
        if allowed {
            Ok(())
        } else {
            Err(Error::InvalidState)
        }
    }

    /// Check if a function is allowed in the given state
    pub fn check_function_access_for_state(function: &str, state: MarketState) -> Result<(), Error> {
        use MarketState::*;
        let allowed = match function {
            "vote" => matches!(state, Active),
            "dispute" => matches!(state, Ended),
            "resolve" => matches!(state, Ended | Disputed),
            "claim" => matches!(state, Resolved),
            "close" => matches!(state, Resolved | Cancelled | Closed),
            _ => true, // By default allow
        };
        if allowed {
            Ok(())
        } else {
            Err(Error::MarketClosed)
        }
    }

    /// Emit a state change event (placeholder: use env.events().publish)
    pub fn emit_state_change_event(env: &Env, market_id: &Symbol, old_state: MarketState, new_state: MarketState) {
        env.events().publish(("market_state_change", market_id), (old_state, new_state));
    }

    /// Validate that the market's state is consistent with its data
    pub fn validate_market_state_consistency(env: &Env, market: &Market) -> Result<(), Error> {
        use MarketState::*;
        let now = env.ledger().timestamp();
        match market.state {
            Active => {
                if market.end_time <= now {
                    return Err(Error::InvalidState);
                }
                if market.winning_outcome.is_some() {
                    return Err(Error::InvalidState);
                }
            }
            Ended => {
                if market.end_time > now {
                    return Err(Error::InvalidState);
                }
                if market.winning_outcome.is_some() {
                    return Err(Error::InvalidState);
                }
            }
            Disputed => {
                if market.dispute_stakes.is_empty() {
                    return Err(Error::InvalidState);
                }
            }
            Resolved => {
                if market.winning_outcome.is_none() {
                    return Err(Error::InvalidState);
                }
            }
            Closed | Cancelled => {}
        }
        Ok(())
    }

    /// Get the current state of a market
    pub fn get_market_state(env: &Env, market_id: &Symbol) -> Result<MarketState, Error> {
        let market = MarketStateManager::get_market(env, market_id)?;
        Ok(market.state)
    }

    /// Check if a market can transition to a target state
    pub fn can_transition_to_state(env: &Env, market_id: &Symbol, target_state: MarketState) -> Result<bool, Error> {
        let market = MarketStateManager::get_market(env, market_id)?;
        Ok(MarketStateLogic::validate_state_transition(market.state, target_state).is_ok())
    }
}

// ===== MODULE TESTS =====

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn test_market_validation() {
        let env = Env::default();

        // Test valid market params
        let valid_question = String::from_str(&env, "Test question?");
        let valid_outcomes = vec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ];

        assert!(MarketValidator::validate_market_params(
            &env,
            &valid_question,
            &valid_outcomes,
            30
        )
        .is_ok());

        // Test invalid question
        let invalid_question = String::from_str(&env, "");
        assert!(MarketValidator::validate_market_params(
            &env,
            &invalid_question,
            &valid_outcomes,
            30
        )
        .is_err());

        // Test invalid outcomes
        let invalid_outcomes = vec![&env, String::from_str(&env, "yes")];
        assert!(MarketValidator::validate_market_params(
            &env,
            &valid_question,
            &invalid_outcomes,
            30
        )
        .is_err());

        // Test invalid duration
        assert!(
            MarketValidator::validate_market_params(&env, &valid_question, &valid_outcomes, 0)
                .is_err()
        );
        assert!(MarketValidator::validate_market_params(
            &env,
            &valid_question,
            &valid_outcomes,
            400
        )
        .is_err());
    }

    #[test]
    fn test_market_utils() {
        let env = Env::default();

        // Test end time calculation
        let current_time = env.ledger().timestamp();
        let end_time = MarketUtils::calculate_end_time(&env, 30);
        assert_eq!(end_time, current_time + 30 * 24 * 60 * 60);

        // Test payout calculation
        let payout = MarketUtils::calculate_payout(1000, 5000, 10000, 2).unwrap();
        assert_eq!(payout, 1960); // (1000 * 98 / 100) * 10000 / 5000

        // Test payout with zero winning total
        assert!(MarketUtils::calculate_payout(1000, 0, 10000, 2).is_err());
    }

    #[test]
    fn test_market_analytics() {
        let env = Env::default();

        // Create a test market
        let market = Market::new(
            &env,
            Address::generate(&env),
            String::from_str(&env, "Test?"),
            vec![
                &env,
                String::from_str(&env, "yes"),
                String::from_str(&env, "no"),
            ],
            env.ledger().timestamp() + 86400,
            OracleConfig::new(
                OracleProvider::Pyth,
                String::from_str(&env, "BTC/USD"),
                25_000_00,
                String::from_str(&env, "gt"),
            ),
            MarketState::Active,
        );

        // Test market stats
        let stats = MarketAnalytics::get_market_stats(&market);
        assert_eq!(stats.total_votes, 0);
        assert_eq!(stats.total_staked, 0);

        // Test community consensus with no votes
        let consensus = MarketAnalytics::calculate_community_consensus(&market);
        assert_eq!(consensus.total_votes, 0);
        assert_eq!(consensus.percentage, 0);
    }
}
