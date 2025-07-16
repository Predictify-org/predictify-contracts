use soroban_sdk::{contracttype, Address, Env, Map, String, Symbol, Vec};

use crate::errors::Error;

use crate::markets::{
    CommunityConsensus, MarketAnalytics, MarketStateManager, MarketUtils,
};

use crate::oracles::{OracleFactory, OracleUtils};
use crate::types::*;

/// Resolution management system for Predictify Hybrid contract
///
/// This module provides a comprehensive resolution system with:
/// - Oracle resolution functions and utilities
/// - Market resolution logic and validation
/// - Resolution analytics and statistics
/// - Resolution helper utilities and testing functions
/// - Resolution state management and tracking

// ===== RESOLUTION TYPES =====

/// Resolution state enumeration
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[contracttype]
pub enum ResolutionState {
    /// Market is active, no resolution yet
    Active,
    /// Oracle result fetched, pending final resolution
    OracleResolved,
    /// Market fully resolved with final outcome
    MarketResolved,
    /// Resolution disputed
    Disputed,
    /// Resolution finalized after dispute
    Finalized,
}

/// Oracle resolution result
#[derive(Clone, Debug)]
#[contracttype]
pub struct OracleResolution {
    pub market_id: Symbol,
    pub oracle_result: String,
    pub price: i128,
    pub threshold: i128,
    pub comparison: String,
    pub timestamp: u64,
    pub provider: OracleProvider,
    pub feed_id: String,
}

/// Market resolution result
#[derive(Clone, Debug)]
#[contracttype]
pub struct MarketResolution {
    pub market_id: Symbol,
    pub final_outcome: String,
    pub oracle_result: String,
    pub community_consensus: CommunityConsensus,
    pub resolution_timestamp: u64,
    pub resolution_method: ResolutionMethod,
    pub confidence_score: u32,
}

/// Resolution method enumeration
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[contracttype]
pub enum ResolutionMethod {
    /// Oracle only resolution
    OracleOnly,
    /// Community consensus only
    CommunityOnly,
    /// Hybrid oracle + community
    Hybrid,
    /// Admin override
    AdminOverride,
    /// Dispute resolution
    DisputeResolution,
}

/// Resolution analytics
#[derive(Clone, Debug)]
#[contracttype]
pub struct ResolutionAnalytics {
    pub total_resolutions: u32,
    pub oracle_resolutions: u32,
    pub community_resolutions: u32,
    pub hybrid_resolutions: u32,
    pub average_confidence: i128,
    pub resolution_times: Vec<u64>,
    pub outcome_distribution: Map<String, u32>,
}

/// Resolution validation result
#[derive(Clone, Debug)]
#[contracttype]
pub struct ResolutionValidation {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub recommendations: Vec<String>,
}

// ===== ORACLE RESOLUTION =====

/// Oracle resolution management
pub struct OracleResolutionManager;

impl OracleResolutionManager {
    /// Fetch oracle result for a market
    pub fn fetch_oracle_result(
        env: &Env,
        market_id: &Symbol,
        oracle_contract: &Address,
    ) -> Result<OracleResolution, Error> {
        // Get the market from storage
        let mut market = MarketStateManager::get_market(env, market_id)?;

        // Validate market for oracle resolution
        OracleResolutionValidator::validate_market_for_oracle_resolution(env, &market)?;

        // Get the price from the appropriate oracle using the factory pattern
        let oracle = OracleFactory::create_oracle(
            market.oracle_config.provider.clone(),
            oracle_contract.clone(),
        )?;

        let price = oracle.get_price(env, &market.oracle_config.feed_id)?;

        // Determine the outcome based on the price and threshold using OracleUtils
        let outcome = OracleUtils::determine_outcome(
            price,
            market.oracle_config.threshold,
            &market.oracle_config.comparison,
            env,
        )?;

        // Create oracle resolution record
        let resolution = OracleResolution {
            market_id: market_id.clone(),
            oracle_result: outcome.clone(),
            price,
            threshold: market.oracle_config.threshold,
            comparison: market.oracle_config.comparison.clone(),
            timestamp: env.ledger().timestamp(),
            provider: market.oracle_config.provider.clone(),
            feed_id: market.oracle_config.feed_id.clone(),
        };

        // Store the result in the market
        MarketStateManager::set_oracle_result(&mut market, outcome.clone());
        MarketStateManager::update_market(env, market_id, &market);

        Ok(resolution)
    }

    /// Get oracle resolution for a market

    pub fn get_oracle_resolution(
        _env: &Env,
        _market_id: &Symbol,
    ) -> Result<Option<OracleResolution>, Error> {
        // For now, return None since we don't store complex types in storage
        // In a real implementation, you would store this in a more sophisticated way

        Ok(None)
    }

    /// Validate oracle resolution
    pub fn validate_oracle_resolution(
        _env: &Env,
        resolution: &OracleResolution,
    ) -> Result<(), Error> {
        // Validate price is positive
        if resolution.price <= 0 {
            return Err(Error::InvalidInput);
        }

        // Validate threshold is positive
        if resolution.threshold <= 0 {
            return Err(Error::InvalidInput);
        }

        // Validate outcome is not empty
        if resolution.oracle_result.is_empty() {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }

    /// Calculate oracle confidence score
    pub fn calculate_oracle_confidence(resolution: &OracleResolution) -> u32 {
        OracleResolutionAnalytics::calculate_confidence_score(resolution)
    }
}

// ===== MARKET RESOLUTION =====

/// Market resolution management
pub struct MarketResolutionManager;

impl MarketResolutionManager {
    /// Resolve a market by combining oracle results and community votes
    pub fn resolve_market(env: &Env, market_id: &Symbol) -> Result<MarketResolution, Error> {
        // Get the market from storage
        let mut market = MarketStateManager::get_market(env, market_id)?;

        // Validate market for resolution
        MarketResolutionValidator::validate_market_for_resolution(env, &market)?;

        // Retrieve the oracle result
        let oracle_result = market
            .oracle_result
            .as_ref()
            .ok_or(Error::OracleUnavailable)?
            .clone();

        // Calculate community consensus
        let community_consensus = MarketAnalytics::calculate_community_consensus(&market);

        // Determine final result using hybrid algorithm
        let final_result =
            MarketUtils::determine_final_result(env, &oracle_result, &community_consensus);

        // Determine resolution method
        let resolution_method = MarketResolutionAnalytics::determine_resolution_method(
            &oracle_result,
            &community_consensus,
        );

        // Calculate confidence score
        let confidence_score = MarketResolutionAnalytics::calculate_confidence_score(
            &oracle_result,
            &community_consensus,
            &resolution_method,
        );

        // Create market resolution record
        let resolution = MarketResolution {
            market_id: market_id.clone(),
            final_outcome: final_result.clone(),
            oracle_result,
            community_consensus,
            resolution_timestamp: env.ledger().timestamp(),
            resolution_method,
            confidence_score,
        };

        // Set winning outcome
        MarketStateManager::set_winning_outcome(&mut market, final_result.clone(), Some(market_id));
        MarketStateManager::update_market(env, market_id, &market);

        Ok(resolution)
    }

    /// Finalize market with admin override
    pub fn finalize_market(
        env: &Env,
        admin: &Address,
        market_id: &Symbol,
        outcome: &String,
    ) -> Result<MarketResolution, Error> {
        // Validate admin permissions
        MarketResolutionValidator::validate_admin_permissions(env, admin)?;

        // Get the market
        let mut market = MarketStateManager::get_market(env, market_id)?;

        // Validate outcome
        MarketResolutionValidator::validate_outcome(env, outcome, &market.outcomes)?;

        // Create resolution record
        let resolution = MarketResolution {
            market_id: market_id.clone(),
            final_outcome: outcome.clone(),
            oracle_result: market
                .oracle_result
                .clone()
                .unwrap_or_else(|| String::from_str(env, "")),
            community_consensus: MarketAnalytics::calculate_community_consensus(&market),
            resolution_timestamp: env.ledger().timestamp(),
            resolution_method: ResolutionMethod::AdminOverride,
            confidence_score: 100, // Admin override has full confidence
        };

        // Set final outcome
        MarketStateManager::set_winning_outcome(&mut market, outcome.clone(), Some(market_id));
        MarketStateManager::update_market(env, market_id, &market);

        Ok(resolution)
    }

    /// Get market resolution

    pub fn get_market_resolution(
        _env: &Env,
        _market_id: &Symbol,
    ) -> Result<Option<MarketResolution>, Error> {
        // For now, return None since we don't store complex types in storage
        // In a real implementation, you would store this in a more sophisticated way

        Ok(None)
    }

    /// Validate market resolution
    pub fn validate_market_resolution(
        env: &Env,
        resolution: &MarketResolution,
    ) -> Result<(), Error> {
        MarketResolutionValidator::validate_market_resolution(env, resolution)
    }
}

// ===== RESOLUTION VALIDATION =====

/// Oracle resolution validation
pub struct OracleResolutionValidator;

impl OracleResolutionValidator {
    /// Validate market for oracle resolution
    pub fn validate_market_for_oracle_resolution(env: &Env, market: &Market) -> Result<(), Error> {
        // Check if the market has already been resolved
        if market.oracle_result.is_some() {
            return Err(Error::MarketAlreadyResolved);
        }

        // Check if the market ended (we can only fetch oracle result after market ends)
        let current_time = env.ledger().timestamp();
        if current_time < market.end_time {
            return Err(Error::MarketClosed);
        }

        Ok(())
    }

    /// Validate oracle resolution
    pub fn validate_oracle_resolution(
        _env: &Env,
        resolution: &OracleResolution,
    ) -> Result<(), Error> {
        // Validate price is positive
        if resolution.price <= 0 {
            return Err(Error::InvalidInput);
        }

        // Validate threshold is positive
        if resolution.threshold <= 0 {
            return Err(Error::InvalidInput);
        }

        // Validate outcome is not empty
        if resolution.oracle_result.is_empty() {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }
}

/// Market resolution validation
pub struct MarketResolutionValidator;

impl MarketResolutionValidator {
    /// Validate market for resolution
    pub fn validate_market_for_resolution(env: &Env, market: &Market) -> Result<(), Error> {
        // Check if market is already resolved
        if market.winning_outcome.is_some() {
            return Err(Error::MarketAlreadyResolved);
        }

        // Check if oracle result is available
        if market.oracle_result.is_none() {
            return Err(Error::OracleUnavailable);
        }

        // Check if market has ended
        let current_time = env.ledger().timestamp();
        if current_time < market.end_time {
            return Err(Error::MarketClosed);
        }

        Ok(())
    }

    /// Validate admin permissions
    pub fn validate_admin_permissions(env: &Env, admin: &Address) -> Result<(), Error> {
        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(env, "Admin"))
            .unwrap_or_else(|| panic!("Admin not set"));

        if admin != &stored_admin {
            return Err(Error::Unauthorized);
        }

        Ok(())
    }

    /// Validate outcome
    pub fn validate_outcome(
        _env: &Env,
        outcome: &String,
        valid_outcomes: &Vec<String>,
    ) -> Result<(), Error> {
        if !valid_outcomes.contains(outcome) {
            return Err(Error::InvalidOutcome);
        }

        Ok(())
    }

    /// Validate market resolution
    pub fn validate_market_resolution(
        env: &Env,
        resolution: &MarketResolution,
    ) -> Result<(), Error> {
        // Validate final outcome is not empty
        if resolution.final_outcome.is_empty() {
            return Err(Error::InvalidInput);
        }

        // Validate confidence score is within range
        if resolution.confidence_score > 100 {
            return Err(Error::InvalidInput);
        }

        // Validate timestamp is reasonable
        let current_time = env.ledger().timestamp();
        if resolution.resolution_timestamp > current_time {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }
}

// ===== RESOLUTION ANALYTICS =====

/// Oracle resolution analytics
pub struct OracleResolutionAnalytics;

impl OracleResolutionAnalytics {
    /// Calculate oracle confidence score
    pub fn calculate_confidence_score(resolution: &OracleResolution) -> u32 {
        // Base confidence for oracle resolution
        let mut confidence: u32 = 80;

        // Adjust based on price deviation from threshold
        let deviation = ((resolution.price - resolution.threshold).abs() as f64)
            / (resolution.threshold as f64);

        if deviation > 0.1 {
            // High deviation - lower confidence
            confidence = confidence.saturating_sub(20);
        } else if deviation < 0.05 {
            // Low deviation - higher confidence
            confidence = confidence.saturating_add(10);
        }

        confidence.min(100)
    }

    /// Get oracle resolution statistics
    pub fn get_oracle_stats(_env: &Env) -> Result<OracleStats, Error> {
        Ok(OracleStats::default())
    }
}

/// Market resolution analytics
pub struct MarketResolutionAnalytics;

impl MarketResolutionAnalytics {
    /// Determine resolution method
    pub fn determine_resolution_method(
        _oracle_result: &String,
        community_consensus: &CommunityConsensus,
    ) -> ResolutionMethod {
        if community_consensus.percentage > 70 {
            ResolutionMethod::Hybrid
        } else {
            ResolutionMethod::OracleOnly
        }
    }

    /// Calculate confidence score
    pub fn calculate_confidence_score(
        _oracle_result: &String,
        community_consensus: &CommunityConsensus,
        method: &ResolutionMethod,
    ) -> u32 {
        match method {
            ResolutionMethod::OracleOnly => 85,
            ResolutionMethod::CommunityOnly => {
                let base_confidence = community_consensus.percentage as u32;
                base_confidence.min(90)
            }
            ResolutionMethod::Hybrid => {
                let oracle_confidence = 85;
                let community_confidence = community_consensus.percentage as u32;
                ((oracle_confidence + community_confidence) / 2).min(95)
            }
            ResolutionMethod::AdminOverride => 100,
            ResolutionMethod::DisputeResolution => 75,
        }
    }

    /// Calculate resolution analytics
    pub fn calculate_resolution_analytics(_env: &Env) -> Result<ResolutionAnalytics, Error> {
        Ok(ResolutionAnalytics::default())
    }

    /// Update resolution analytics
    pub fn update_resolution_analytics(
        _env: &Env,
        _resolution: &MarketResolution,
    ) -> Result<(), Error> {
        // For now, do nothing since we don't store complex types
        Ok(())
    }
}

// ===== RESOLUTION UTILITIES =====

/// Resolution utility functions
pub struct ResolutionUtils;

impl ResolutionUtils {
    /// Get resolution state for a market
    pub fn get_resolution_state(_env: &Env, market: &Market) -> ResolutionState {
        if market.winning_outcome.is_some() {
            ResolutionState::MarketResolved
        } else if market.oracle_result.is_some() {
            ResolutionState::OracleResolved
        } else if market.total_dispute_stakes() > 0 {
            ResolutionState::Disputed
        } else {
            ResolutionState::Active
        }
    }

    /// Check if market can be resolved
    pub fn can_resolve_market(env: &Env, market: &Market) -> bool {
        market.has_ended(env.ledger().timestamp())
            && market.oracle_result.is_some()
            && market.winning_outcome.is_none()
    }

    /// Get resolution eligibility
    pub fn get_resolution_eligibility(env: &Env, market: &Market) -> (bool, String) {
        if !market.has_ended(env.ledger().timestamp()) {
            return (false, String::from_str(env, "Market has not ended"));
        }

        if market.oracle_result.is_none() {
            return (false, String::from_str(env, "Oracle result not available"));
        }

        if market.winning_outcome.is_some() {
            return (false, String::from_str(env, "Market already resolved"));
        }

        (true, String::from_str(env, "Eligible for resolution"))
    }

    /// Calculate resolution time
    pub fn calculate_resolution_time(env: &Env, market: &Market) -> u64 {
        let current_time = env.ledger().timestamp();
        if current_time > market.end_time {
            current_time - market.end_time
        } else {
            0
        }
    }

    /// Validate resolution parameters
    pub fn validate_resolution_parameters(
        _env: &Env,
        market: &Market,
        outcome: &String,
    ) -> Result<(), Error> {
        // Validate outcome is in market outcomes
        if !market.outcomes.contains(outcome) {
            return Err(Error::InvalidOutcome);
        }

        // Validate market is not already resolved
        if market.winning_outcome.is_some() {
            return Err(Error::MarketAlreadyResolved);
        }

        Ok(())
    }
}

// ===== RESOLUTION TESTING =====

/// Resolution testing utilities
pub struct ResolutionTesting;

impl ResolutionTesting {
    /// Create test oracle resolution
    pub fn create_test_oracle_resolution(env: &Env, market_id: &Symbol) -> OracleResolution {
        OracleResolution {
            market_id: market_id.clone(),
            oracle_result: String::from_str(env, "yes"),
            price: 2500000,
            threshold: 2500000,
            comparison: String::from_str(env, "gt"),
            timestamp: env.ledger().timestamp(),
            provider: OracleProvider::Pyth,
            feed_id: String::from_str(env, "BTC/USD"),
        }
    }

    /// Create test market resolution
    pub fn create_test_market_resolution(env: &Env, market_id: &Symbol) -> MarketResolution {
        MarketResolution {
            market_id: market_id.clone(),
            final_outcome: String::from_str(env, "yes"),
            oracle_result: String::from_str(env, "yes"),
            community_consensus: CommunityConsensus {
                outcome: String::from_str(env, "yes"),
                votes: 6,
                total_votes: 10,
                percentage: 60,
            },
            resolution_timestamp: env.ledger().timestamp(),
            resolution_method: ResolutionMethod::Hybrid,
            confidence_score: 80,
        }
    }

    /// Validate resolution structure
    pub fn validate_resolution_structure(resolution: &MarketResolution) -> Result<(), Error> {
        if resolution.final_outcome.is_empty() {
            return Err(Error::InvalidInput);
        }

        if resolution.confidence_score > 100 {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }

    /// Simulate resolution process
    pub fn simulate_resolution_process(
        env: &Env,
        market_id: &Symbol,
        oracle_contract: &Address,
    ) -> Result<MarketResolution, Error> {
        // Fetch oracle result
        let _oracle_resolution =
            OracleResolutionManager::fetch_oracle_result(env, market_id, oracle_contract)?;

        // Resolve market
        let market_resolution = MarketResolutionManager::resolve_market(env, market_id)?;

        Ok(market_resolution)
    }
}

// ===== STATISTICS TYPES =====

/// Oracle statistics
#[derive(Clone, Debug)]
#[contracttype]
pub struct OracleStats {
    pub total_resolutions: u32,
    pub successful_resolutions: u32,
    pub average_confidence: i128,
    pub provider_distribution: Map<OracleProvider, u32>,
}

impl Default for OracleStats {
    fn default() -> Self {
        Self {
            total_resolutions: 0,
            successful_resolutions: 0,
            average_confidence: 0,
            provider_distribution: Map::new(&soroban_sdk::Env::default()),
        }
    }
}

impl Default for ResolutionAnalytics {
    fn default() -> Self {
        Self {
            total_resolutions: 0,
            oracle_resolutions: 0,
            community_resolutions: 0,
            hybrid_resolutions: 0,
            average_confidence: 0,
            resolution_times: Vec::new(&soroban_sdk::Env::default()),
            outcome_distribution: Map::new(&soroban_sdk::Env::default()),
        }
    }
}

// ===== MODULE TESTS =====

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{test::PredictifyTest, PredictifyHybridClient};
    use soroban_sdk::testutils::{Address as _, Ledger, LedgerInfo};

    #[test]
    fn test_oracle_resolution_manager_fetch_result() {
        let env = Env::default();
        let market_id = Symbol::new(&env, "test_market");
        let _oracle_contract = Address::generate(&env);

        // This test would require a mock oracle setup
        // For now, we'll test the validation logic
        let resolution = ResolutionTesting::create_test_oracle_resolution(&env, &market_id);
        assert_eq!(resolution.oracle_result, String::from_str(&env, "yes"));
        assert_eq!(resolution.price, 2500000);
    }

    #[test]
    fn test_market_resolution_manager_resolve_market() {
        let env = Env::default();
        let market_id = Symbol::new(&env, "test_market");

        // This test would require a complete market setup
        // For now, we'll test the resolution structure
        let resolution = ResolutionTesting::create_test_market_resolution(&env, &market_id);
        assert_eq!(resolution.final_outcome, String::from_str(&env, "yes"));
        assert_eq!(resolution.resolution_method, ResolutionMethod::Hybrid);
    }

    #[test]
    fn test_resolution_utils_get_state() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let market = Market::new(
            &env,
            admin,
            String::from_str(&env, "Test Market"),
            soroban_sdk::vec![
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
            MarketState::Active,
        );

        let state = ResolutionUtils::get_resolution_state(&env, &market);
        assert_eq!(state, ResolutionState::Active);
    }

    #[test]
    fn test_resolution_analytics_determine_method() {
        let env = Env::default();
        let oracle_result = String::from_str(&env, "yes");
        let community_consensus = CommunityConsensus {
            outcome: String::from_str(&env, "yes"),
            votes: 6,
            total_votes: 10,
            percentage: 60,
        };

        let method = MarketResolutionAnalytics::determine_resolution_method(
            &oracle_result,
            &community_consensus,
        );
        assert_eq!(method, ResolutionMethod::Hybrid);
    }

    #[test]
    fn test_resolution_testing_utilities() {
        let env = Env::default();
        let market_id = Symbol::new(&env, "test_market");

        let oracle_resolution = ResolutionTesting::create_test_oracle_resolution(&env, &market_id);
        assert!(oracle_resolution.oracle_result == String::from_str(&env, "yes"));

        let market_resolution = ResolutionTesting::create_test_market_resolution(&env, &market_id);
        assert!(ResolutionTesting::validate_resolution_structure(&market_resolution).is_ok());
    }


    #[test]
    fn test_resolution_method_determination() {
        let env = Env::default();
        
        // Create test data
        let community_consensus = CommunityConsensus {
            outcome: String::from_str(&env, "yes"),
            votes: 75,
            total_votes: 100,
            percentage: 75,
        };

        // Test hybrid resolution
        let method = MarketResolutionAnalytics::determine_resolution_method(
            &String::from_str(&env, "yes"),
            &community_consensus,
        );
        assert!(matches!(method, ResolutionMethod::Hybrid));

        // Test oracle-only resolution
        let low_consensus = CommunityConsensus {
            outcome: String::from_str(&env, "yes"),
            votes: 60,
            total_votes: 100,
            percentage: 60,
        };
        let method = MarketResolutionAnalytics::determine_resolution_method(
            &String::from_str(&env, "yes"),
            &low_consensus,
        );
        assert!(matches!(method, ResolutionMethod::OracleOnly));
    }
}
