use soroban_sdk::{contracttype, symbol_short, token, vec, Address, Env, Map, String, Symbol, Vec};

use crate::errors::Error;
use crate::markets::{MarketStateManager, MarketUtils};
use crate::types::Market;

/// Fee management system for Predictify Hybrid contract
///
/// This module provides a comprehensive fee management system with:
/// - Fee collection and distribution functions
/// - Fee calculation and validation utilities
/// - Fee analytics and tracking functions
/// - Fee configuration management
/// - Fee safety checks and validation

// ===== FEE CONSTANTS =====

/// Platform fee percentage (2%)
pub const PLATFORM_FEE_PERCENTAGE: i128 = 2;

/// Market creation fee (1 XLM = 10,000,000 stroops)
pub const MARKET_CREATION_FEE: i128 = 10_000_000;

/// Minimum fee amount (0.1 XLM)
pub const MIN_FEE_AMOUNT: i128 = 1_000_000;

/// Maximum fee amount (100 XLM)
pub const MAX_FEE_AMOUNT: i128 = 1_000_000_000;

/// Fee collection threshold (minimum amount before fees can be collected)
pub const FEE_COLLECTION_THRESHOLD: i128 = 100_000_000; // 10 XLM

// ===== FEE TYPES =====

/// Fee configuration for a market
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeConfig {
    /// Platform fee percentage
    pub platform_fee_percentage: i128,
    /// Market creation fee
    pub creation_fee: i128,
    /// Minimum fee amount
    pub min_fee_amount: i128,
    /// Maximum fee amount
    pub max_fee_amount: i128,
    /// Fee collection threshold
    pub collection_threshold: i128,
    /// Whether fees are enabled
    pub fees_enabled: bool,
}

/// Fee collection record
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeCollection {
    /// Market ID
    pub market_id: Symbol,
    /// Amount collected
    pub amount: i128,
    /// Collected by admin
    pub collected_by: Address,
    /// Collection timestamp
    pub timestamp: u64,
    /// Fee percentage used
    pub fee_percentage: i128,
}

/// Fee analytics data
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeAnalytics {
    /// Total fees collected across all markets
    pub total_fees_collected: i128,
    /// Number of markets with fees collected
    pub markets_with_fees: u32,
    /// Average fee per market
    pub average_fee_per_market: i128,
    /// Fee collection history
    pub collection_history: Vec<FeeCollection>,
    /// Fee distribution by market size
    pub fee_distribution: Map<String, i128>,
}

/// Fee validation result
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeValidationResult {
    /// Whether the fee is valid
    pub is_valid: bool,
    /// Validation errors
    pub errors: Vec<String>,
    /// Suggested fee amount
    pub suggested_amount: i128,
    /// Fee breakdown
    pub breakdown: FeeBreakdown,
}

/// Fee breakdown details
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeBreakdown {
    /// Total staked amount
    pub total_staked: i128,
    /// Fee percentage
    pub fee_percentage: i128,
    /// Calculated fee amount
    pub fee_amount: i128,
    /// Platform fee
    pub platform_fee: i128,
    /// User payout amount (after fees)
    pub user_payout_amount: i128,
}

// ===== FEE MANAGER =====

/// Main fee management system
pub struct FeeManager;

impl FeeManager {
    /// Collect platform fees from a market
    pub fn collect_fees(env: &Env, admin: Address, market_id: Symbol) -> Result<i128, Error> {
        // Require authentication from the admin
        admin.require_auth();

        // Validate admin permissions
        FeeValidator::validate_admin_permissions(env, &admin)?;

        // Get and validate market
        let mut market = MarketStateManager::get_market(env, &market_id)?;
        FeeValidator::validate_market_for_fee_collection(&market)?;

        // Calculate fee amount
        let fee_amount = FeeCalculator::calculate_platform_fee(&market)?;

        // Validate fee amount
        FeeValidator::validate_fee_amount(fee_amount)?;

        // Transfer fees to admin
        FeeUtils::transfer_fees_to_admin(env, &admin, fee_amount)?;

        // Record fee collection
        FeeTracker::record_fee_collection(env, &market_id, fee_amount, &admin)?;

        // Mark fees as collected
        MarketStateManager::mark_fees_collected(&mut market);
        MarketStateManager::update_market(env, &market_id, &market);

        Ok(fee_amount)
    }

    /// Process market creation fee
    pub fn process_creation_fee(env: &Env, admin: &Address) -> Result<(), Error> {
        // Validate creation fee
        FeeValidator::validate_creation_fee(MARKET_CREATION_FEE)?;

        // Get token client
        let token_client = MarketUtils::get_token_client(env)?;

        // Transfer creation fee from admin to contract
        token_client.transfer(admin, &env.current_contract_address(), &MARKET_CREATION_FEE);

        // Record creation fee
        FeeTracker::record_creation_fee(env, admin, MARKET_CREATION_FEE)?;

        Ok(())
    }

    /// Get fee analytics for all markets
    pub fn get_fee_analytics(env: &Env) -> Result<FeeAnalytics, Error> {
        FeeAnalytics::calculate_analytics(env)
    }

    /// Update fee configuration (admin only)
    pub fn update_fee_config(
        env: &Env,
        admin: Address,
        new_config: FeeConfig,
    ) -> Result<FeeConfig, Error> {
        // Require authentication from the admin
        admin.require_auth();

        // Validate admin permissions
        FeeValidator::validate_admin_permissions(env, &admin)?;

        // Validate new configuration
        FeeValidator::validate_fee_config(&new_config)?;

        // Store new configuration
        FeeConfigManager::store_fee_config(env, &new_config)?;

        // Record configuration change
        FeeTracker::record_config_change(env, &admin, &new_config)?;

        Ok(new_config)
    }

    /// Get current fee configuration
    pub fn get_fee_config(env: &Env) -> Result<FeeConfig, Error> {
        FeeConfigManager::get_fee_config(env)
    }

    /// Validate fee calculation for a market
    pub fn validate_market_fees(env: &Env, market_id: &Symbol) -> Result<FeeValidationResult, Error> {
        let market = MarketStateManager::get_market(env, market_id)?;
        FeeValidator::validate_market_fees(&market)
    }
}

// ===== FEE CALCULATOR =====

/// Fee calculation utilities
pub struct FeeCalculator;

impl FeeCalculator {
    /// Calculate platform fee for a market
    pub fn calculate_platform_fee(market: &Market) -> Result<i128, Error> {
        if market.total_staked == 0 {
            return Err(Error::NoFeesToCollect);
        }

        let fee_amount = (market.total_staked * PLATFORM_FEE_PERCENTAGE) / 100;
        
        if fee_amount < MIN_FEE_AMOUNT {
            return Err(Error::InsufficientStake);
        }

        Ok(fee_amount)
    }

    /// Calculate user payout after fees
    pub fn calculate_user_payout_after_fees(
        user_stake: i128,
        winning_total: i128,
        total_pool: i128,
    ) -> Result<i128, Error> {
        if winning_total == 0 {
            return Err(Error::NothingToClaim);
        }

        let user_share = (user_stake * (100 - PLATFORM_FEE_PERCENTAGE)) / 100;
        let payout = (user_share * total_pool) / winning_total;

        Ok(payout)
    }

    /// Calculate fee breakdown for a market
    pub fn calculate_fee_breakdown(market: &Market) -> Result<FeeBreakdown, Error> {
        let total_staked = market.total_staked;
        let fee_percentage = PLATFORM_FEE_PERCENTAGE;
        let fee_amount = Self::calculate_platform_fee(market)?;
        let platform_fee = fee_amount;
        let user_payout_amount = total_staked - fee_amount;

        Ok(FeeBreakdown {
            total_staked,
            fee_percentage,
            fee_amount,
            platform_fee,
            user_payout_amount,
        })
    }

    /// Calculate dynamic fee based on market characteristics
    pub fn calculate_dynamic_fee(market: &Market) -> Result<i128, Error> {
        let base_fee = Self::calculate_platform_fee(market)?;
        
        // Adjust fee based on market size
        let size_multiplier = if market.total_staked > 1_000_000_000 {
            80 // 20% reduction for large markets
        } else if market.total_staked > 100_000_000 {
            90 // 10% reduction for medium markets
        } else {
            100 // No adjustment for small markets
        };

        let adjusted_fee = (base_fee * size_multiplier) / 100;
        
        // Ensure minimum fee
        if adjusted_fee < MIN_FEE_AMOUNT {
            Ok(MIN_FEE_AMOUNT)
        } else {
            Ok(adjusted_fee)
        }
    }
}

// ===== FEE VALIDATOR =====

/// Fee validation utilities
pub struct FeeValidator;

impl FeeValidator {
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

    /// Validate market for fee collection
    pub fn validate_market_for_fee_collection(market: &Market) -> Result<(), Error> {
        // Check if market is resolved
        if market.winning_outcome.is_none() {
            return Err(Error::MarketNotResolved);
        }

        // Check if fees already collected
        if market.fee_collected {
            return Err(Error::FeeAlreadyCollected);
        }

        // Check if there are sufficient stakes
        if market.total_staked < FEE_COLLECTION_THRESHOLD {
            return Err(Error::InsufficientStake);
        }

        Ok(())
    }

    /// Validate fee amount
    pub fn validate_fee_amount(fee_amount: i128) -> Result<(), Error> {
        if fee_amount < MIN_FEE_AMOUNT {
            return Err(Error::InsufficientStake);
        }

        if fee_amount > MAX_FEE_AMOUNT {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }

    /// Validate creation fee
    pub fn validate_creation_fee(fee_amount: i128) -> Result<(), Error> {
        if fee_amount != MARKET_CREATION_FEE {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }

    /// Validate fee configuration
    pub fn validate_fee_config(config: &FeeConfig) -> Result<(), Error> {
        if config.platform_fee_percentage < 0 || config.platform_fee_percentage > 10 {
            return Err(Error::InvalidInput);
        }

        if config.creation_fee < 0 {
            return Err(Error::InvalidInput);
        }

        if config.min_fee_amount < 0 {
            return Err(Error::InvalidInput);
        }

        if config.max_fee_amount < config.min_fee_amount {
            return Err(Error::InvalidInput);
        }

        if config.collection_threshold < 0 {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }

    /// Validate market fees
    pub fn validate_market_fees(market: &Market) -> Result<FeeValidationResult, Error> {
        let mut errors = Vec::new(&Env::default());
        let mut is_valid = true;

        // Check if market has sufficient stakes
        if market.total_staked < FEE_COLLECTION_THRESHOLD {
            errors.push_back(String::from_str(&Env::default(), "Insufficient stakes for fee collection"));
            is_valid = false;
        }

        // Check if fees already collected
        if market.fee_collected {
            errors.push_back(String::from_str(&Env::default(), "Fees already collected"));
            is_valid = false;
        }

        // Calculate fee breakdown
        let breakdown = FeeCalculator::calculate_fee_breakdown(market)?;
        let suggested_amount = breakdown.fee_amount;

        Ok(FeeValidationResult {
            is_valid,
            errors,
            suggested_amount,
            breakdown,
        })
    }
}

// ===== FEE UTILS =====

/// Fee utility functions
pub struct FeeUtils;

impl FeeUtils {
    /// Transfer fees to admin
    pub fn transfer_fees_to_admin(env: &Env, admin: &Address, amount: i128) -> Result<(), Error> {
        let token_client = MarketUtils::get_token_client(env)?;
        token_client.transfer(&env.current_contract_address(), admin, &amount);
        Ok(())
    }

    /// Get fee statistics for a market
    pub fn get_market_fee_stats(market: &Market) -> Result<FeeBreakdown, Error> {
        FeeCalculator::calculate_fee_breakdown(market)
    }

    /// Check if fees can be collected for a market
    pub fn can_collect_fees(market: &Market) -> bool {
        market.winning_outcome.is_some() 
            && !market.fee_collected 
            && market.total_staked >= FEE_COLLECTION_THRESHOLD
    }

    /// Get fee collection eligibility for a market
    pub fn get_fee_eligibility(market: &Market) -> (bool, String) {
        if market.winning_outcome.is_none() {
            return (false, String::from_str(&Env::default(), "Market not resolved"));
        }

        if market.fee_collected {
            return (false, String::from_str(&Env::default(), "Fees already collected"));
        }

        if market.total_staked < FEE_COLLECTION_THRESHOLD {
            return (false, String::from_str(&Env::default(), "Insufficient stakes"));
        }

        (true, String::from_str(&Env::default(), "Eligible for fee collection"))
    }
}

// ===== FEE TRACKER =====

/// Fee tracking and analytics
pub struct FeeTracker;

impl FeeTracker {
    /// Record fee collection
    pub fn record_fee_collection(
        env: &Env,
        market_id: &Symbol,
        amount: i128,
        admin: &Address,
    ) -> Result<(), Error> {
        let collection = FeeCollection {
            market_id: market_id.clone(),
            amount,
            collected_by: admin.clone(),
            timestamp: env.ledger().timestamp(),
            fee_percentage: PLATFORM_FEE_PERCENTAGE,
        };

        // Store in fee collection history
        let history_key = symbol_short!("fee_hist");
        let mut history: Vec<FeeCollection> = env
            .storage()
            .persistent()
            .get(&history_key)
            .unwrap_or(vec![env]);

        history.push_back(collection);
        env.storage().persistent().set(&history_key, &history);

        // Update total fees collected
        let total_key = symbol_short!("tot_fees");
        let current_total: i128 = env
            .storage()
            .persistent()
            .get(&total_key)
            .unwrap_or(0);

        env.storage()
            .persistent()
            .set(&total_key, &(current_total + amount));

        Ok(())
    }

    /// Record creation fee
    pub fn record_creation_fee(
        env: &Env,
        admin: &Address,
        amount: i128,
    ) -> Result<(), Error> {
        // Record creation fee in analytics
        let creation_key = symbol_short!("creat_fee");
        let current_total: i128 = env
            .storage()
            .persistent()
            .get(&creation_key)
            .unwrap_or(0);

        env.storage()
            .persistent()
            .set(&creation_key, &(current_total + amount));

        Ok(())
    }

    /// Record configuration change
    pub fn record_config_change(
        env: &Env,
        admin: &Address,
        config: &FeeConfig,
    ) -> Result<(), Error> {
        // Store configuration change timestamp
        let config_key = symbol_short!("cfg_time");
        env.storage()
            .persistent()
            .set(&config_key, &env.ledger().timestamp());

        Ok(())
    }

    /// Get fee collection history
    pub fn get_fee_history(env: &Env) -> Result<Vec<FeeCollection>, Error> {
        let history_key = symbol_short!("fee_hist");
        Ok(env
            .storage()
            .persistent()
            .get(&history_key)
            .unwrap_or(vec![env]))
    }

    /// Get total fees collected
    pub fn get_total_fees_collected(env: &Env) -> Result<i128, Error> {
        let total_key = symbol_short!("tot_fees");
        Ok(env
            .storage()
            .persistent()
            .get(&total_key)
            .unwrap_or(0))
    }
}

// ===== FEE CONFIG MANAGER =====

/// Fee configuration management
pub struct FeeConfigManager;

impl FeeConfigManager {
    /// Store fee configuration
    pub fn store_fee_config(env: &Env, config: &FeeConfig) -> Result<(), Error> {
        let config_key = symbol_short!("fee_cfg");
        env.storage().persistent().set(&config_key, config);
        Ok(())
    }

    /// Get fee configuration
    pub fn get_fee_config(env: &Env) -> Result<FeeConfig, Error> {
        let config_key = symbol_short!("fee_cfg");
        Ok(env
            .storage()
            .persistent()
            .get(&config_key)
            .unwrap_or(FeeConfig {
                platform_fee_percentage: PLATFORM_FEE_PERCENTAGE,
                creation_fee: MARKET_CREATION_FEE,
                min_fee_amount: MIN_FEE_AMOUNT,
                max_fee_amount: MAX_FEE_AMOUNT,
                collection_threshold: FEE_COLLECTION_THRESHOLD,
                fees_enabled: true,
            }))
    }

    /// Reset fee configuration to defaults
    pub fn reset_to_defaults(env: &Env) -> Result<FeeConfig, Error> {
        let default_config = FeeConfig {
            platform_fee_percentage: PLATFORM_FEE_PERCENTAGE,
            creation_fee: MARKET_CREATION_FEE,
            min_fee_amount: MIN_FEE_AMOUNT,
            max_fee_amount: MAX_FEE_AMOUNT,
            collection_threshold: FEE_COLLECTION_THRESHOLD,
            fees_enabled: true,
        };

        Self::store_fee_config(env, &default_config)?;
        Ok(default_config)
    }
}

// ===== FEE ANALYTICS =====

impl FeeAnalytics {
    /// Calculate fee analytics
    pub fn calculate_analytics(env: &Env) -> Result<FeeAnalytics, Error> {
        let total_fees = FeeTracker::get_total_fees_collected(env)?;
        let history = FeeTracker::get_fee_history(env)?;
        let markets_with_fees = history.len() as u32;

        let average_fee = if markets_with_fees > 0 {
            total_fees / (markets_with_fees as i128)
        } else {
            0
        };

        // Create fee distribution map
        let fee_distribution = Map::new(env);
        // TODO: Implement proper fee distribution calculation

        Ok(FeeAnalytics {
            total_fees_collected: total_fees,
            markets_with_fees,
            average_fee_per_market: average_fee,
            collection_history: history,
            fee_distribution,
        })
    }

    /// Get fee statistics for a specific market
    pub fn get_market_fee_stats(market: &Market) -> Result<FeeBreakdown, Error> {
        FeeCalculator::calculate_fee_breakdown(market)
    }

    /// Calculate fee efficiency (fees collected vs potential)
    pub fn calculate_fee_efficiency(market: &Market) -> Result<f64, Error> {
        let potential_fee = FeeCalculator::calculate_platform_fee(market)?;
        let actual_fee = if market.fee_collected { potential_fee } else { 0 };
        
        if potential_fee == 0 {
            return Ok(0.0);
        }

        Ok((actual_fee as f64) / (potential_fee as f64))
    }
}

// ===== FEE TESTING UTILITIES =====

#[cfg(test)]
pub mod testing {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    /// Create a test fee configuration
    pub fn create_test_fee_config() -> FeeConfig {
        FeeConfig {
            platform_fee_percentage: PLATFORM_FEE_PERCENTAGE,
            creation_fee: MARKET_CREATION_FEE,
            min_fee_amount: MIN_FEE_AMOUNT,
            max_fee_amount: MAX_FEE_AMOUNT,
            collection_threshold: FEE_COLLECTION_THRESHOLD,
            fees_enabled: true,
        }
    }

    /// Create a test fee collection record
    pub fn create_test_fee_collection(
        env: &Env,
        market_id: Symbol,
        amount: i128,
        admin: Address,
    ) -> FeeCollection {
        FeeCollection {
            market_id,
            amount,
            collected_by: admin,
            timestamp: env.ledger().timestamp(),
            fee_percentage: PLATFORM_FEE_PERCENTAGE,
        }
    }

    /// Create a test fee breakdown
    pub fn create_test_fee_breakdown() -> FeeBreakdown {
        FeeBreakdown {
            total_staked: 1_000_000_000, // 100 XLM
            fee_percentage: PLATFORM_FEE_PERCENTAGE,
            fee_amount: 20_000_000, // 2 XLM
            platform_fee: 20_000_000,
            user_payout_amount: 980_000_000, // 98 XLM
        }
    }

    /// Validate fee configuration
    pub fn validate_fee_config_structure(config: &FeeConfig) -> Result<(), Error> {
        if config.platform_fee_percentage < 0 {
            return Err(Error::InvalidInput);
        }

        if config.creation_fee < 0 {
            return Err(Error::InvalidInput);
        }

        if config.min_fee_amount < 0 {
            return Err(Error::InvalidInput);
        }

        if config.max_fee_amount < config.min_fee_amount {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }

    /// Validate fee collection record
    pub fn validate_fee_collection_structure(collection: &FeeCollection) -> Result<(), Error> {
        if collection.amount <= 0 {
            return Err(Error::InvalidInput);
        }

        if collection.fee_percentage < 0 {
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
    fn test_fee_calculator_platform_fee() {
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
            crate::types::OracleConfig::new(
                crate::types::OracleProvider::Pyth,
                String::from_str(&env, "BTC/USD"),
                25_000_00,
                String::from_str(&env, "gt"),
            ),
        );

        // Set total staked
        market.total_staked = 1_000_000_000; // 100 XLM

        // Calculate fee
        let fee = FeeCalculator::calculate_platform_fee(&market).unwrap();
        assert_eq!(fee, 20_000_000); // 2% of 100 XLM = 2 XLM
    }

    #[test]
    fn test_fee_validator_admin_permissions() {
        let env = Env::default();
        let admin = Address::generate(&env);

        // Set admin in storage
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, "Admin"), &admin);

        // Valid admin
        assert!(FeeValidator::validate_admin_permissions(&env, &admin).is_ok());

        // Invalid admin
        let invalid_admin = Address::generate(&env);
        assert!(FeeValidator::validate_admin_permissions(&env, &invalid_admin).is_err());
    }

    #[test]
    fn test_fee_validator_fee_amount() {
        // Valid fee amount
        assert!(FeeValidator::validate_fee_amount(MIN_FEE_AMOUNT).is_ok());

        // Invalid fee amount (too small)
        assert!(FeeValidator::validate_fee_amount(MIN_FEE_AMOUNT - 1).is_err());

        // Invalid fee amount (too large)
        assert!(FeeValidator::validate_fee_amount(MAX_FEE_AMOUNT + 1).is_err());
    }

    #[test]
    fn test_fee_utils_can_collect_fees() {
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
            crate::types::OracleConfig::new(
                crate::types::OracleProvider::Pyth,
                String::from_str(&env, "BTC/USD"),
                25_000_00,
                String::from_str(&env, "gt"),
            ),
        );

        // Market not resolved
        assert!(!FeeUtils::can_collect_fees(&market));

        // Set winning outcome
        market.winning_outcome = Some(String::from_str(&env, "yes"));

        // Insufficient stakes
        market.total_staked = FEE_COLLECTION_THRESHOLD - 1;
        assert!(!FeeUtils::can_collect_fees(&market));

        // Sufficient stakes
        market.total_staked = FEE_COLLECTION_THRESHOLD;
        assert!(FeeUtils::can_collect_fees(&market));

        // Fees already collected
        market.fee_collected = true;
        assert!(!FeeUtils::can_collect_fees(&market));
    }

    #[test]
    fn test_fee_config_manager() {
        let env = Env::default();
        let config = testing::create_test_fee_config();

        // Store and retrieve config
        FeeConfigManager::store_fee_config(&env, &config).unwrap();
        let retrieved_config = FeeConfigManager::get_fee_config(&env).unwrap();

        assert_eq!(config, retrieved_config);
    }

    #[test]
    fn test_fee_analytics_calculation() {
        let env = Env::default();
        
        // Test with no fee history
        let analytics = FeeAnalytics::calculate_analytics(&env).unwrap();
        assert_eq!(analytics.total_fees_collected, 0);
        assert_eq!(analytics.markets_with_fees, 0);
        assert_eq!(analytics.average_fee_per_market, 0);
    }

    #[test]
    fn test_testing_utilities() {
        // Test fee config validation
        let config = testing::create_test_fee_config();
        assert!(testing::validate_fee_config_structure(&config).is_ok());

        // Test fee collection validation
        let env = Env::default();
        let collection = testing::create_test_fee_collection(
            &env,
            Symbol::new(&env, "test"),
            1_000_000,
            Address::generate(&env),
        );
        assert!(testing::validate_fee_collection_structure(&collection).is_ok());
    }
} 