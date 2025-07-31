use soroban_sdk::{contracttype, Address, Env, String, Symbol};

use crate::errors::Error;

/// Configuration management system for Predictify Hybrid contract
///
/// This module provides a comprehensive configuration system with:
/// - Centralized constants and configuration values
/// - Environment-specific configuration support
/// - Configuration validation and helper functions
/// - Configuration documentation and testing utilities
/// - Modular configuration system for easier maintenance

// ===== CORE CONSTANTS =====

/// Percentage denominator for calculations (100%)
pub const PERCENTAGE_DENOMINATOR: i128 = 100;

/// Maximum market duration in days
pub const MAX_MARKET_DURATION_DAYS: u32 = 365;

/// Minimum market duration in days
pub const MIN_MARKET_DURATION_DAYS: u32 = 1;

/// Maximum number of outcomes per market
pub const MAX_MARKET_OUTCOMES: u32 = 10;

/// Minimum number of outcomes per market
pub const MIN_MARKET_OUTCOMES: u32 = 2;

/// Maximum question length in characters
pub const MAX_QUESTION_LENGTH: u32 = 500;

/// Maximum outcome length in characters
pub const MAX_OUTCOME_LENGTH: u32 = 100;

// ===== FEE CONSTANTS =====

/// Default platform fee percentage (2%)
pub const DEFAULT_PLATFORM_FEE_PERCENTAGE: i128 = 2;

/// Default market creation fee (1 XLM)
pub const DEFAULT_MARKET_CREATION_FEE: i128 = 10_000_000;

/// Minimum fee amount (0.1 XLM)
pub const MIN_FEE_AMOUNT: i128 = 1_000_000;

/// Maximum fee amount (100 XLM)
pub const MAX_FEE_AMOUNT: i128 = 1_000_000_000;

/// Fee collection threshold (10 XLM)
pub const FEE_COLLECTION_THRESHOLD: i128 = 100_000_000;

/// Maximum platform fee percentage
pub const MAX_PLATFORM_FEE_PERCENTAGE: i128 = 10;

/// Minimum platform fee percentage
pub const MIN_PLATFORM_FEE_PERCENTAGE: i128 = 0;

// ===== VOTING CONSTANTS =====

/// Minimum vote stake (0.1 XLM)
pub const MIN_VOTE_STAKE: i128 = 1_000_000;

/// Minimum dispute stake (1 XLM)
pub const MIN_DISPUTE_STAKE: i128 = 10_000_000;

/// Maximum dispute threshold (10 XLM)
pub const MAX_DISPUTE_THRESHOLD: i128 = 100_000_000;

/// Base dispute threshold (1 XLM)
pub const BASE_DISPUTE_THRESHOLD: i128 = 10_000_000;

/// Large market threshold (100 XLM)
pub const LARGE_MARKET_THRESHOLD: i128 = 1_000_000_000;

/// High activity threshold (100 votes)
pub const HIGH_ACTIVITY_THRESHOLD: u32 = 100;

/// Dispute extension hours
pub const DISPUTE_EXTENSION_HOURS: u32 = 24;

// ===== EXTENSION CONSTANTS =====

/// Maximum extension days
pub const MAX_EXTENSION_DAYS: u32 = 30;

/// Minimum extension days
pub const MIN_EXTENSION_DAYS: u32 = 1;

/// Extension fee per day (1 XLM)
pub const EXTENSION_FEE_PER_DAY: i128 = 100_000_000;

/// Maximum total extensions per market
pub const MAX_TOTAL_EXTENSIONS: u32 = 3;

// ===== RESOLUTION CONSTANTS =====

/// Minimum confidence score
pub const MIN_CONFIDENCE_SCORE: u32 = 0;

/// Maximum confidence score
pub const MAX_CONFIDENCE_SCORE: u32 = 100;

/// Oracle weight in hybrid resolution (70%)
pub const ORACLE_WEIGHT_PERCENTAGE: u32 = 70;

/// Community weight in hybrid resolution (30%)
pub const COMMUNITY_WEIGHT_PERCENTAGE: u32 = 30;

/// Minimum votes for community consensus
pub const MIN_VOTES_FOR_CONSENSUS: u32 = 5;

// ===== ORACLE CONSTANTS =====

/// Maximum oracle price age (1 hour)
pub const MAX_ORACLE_PRICE_AGE: u64 = 3600;

/// Oracle retry attempts
pub const ORACLE_RETRY_ATTEMPTS: u32 = 3;

/// Oracle timeout seconds
pub const ORACLE_TIMEOUT_SECONDS: u64 = 30;

// ===== STORAGE CONSTANTS =====

/// Storage key for admin address
pub const ADMIN_STORAGE_KEY: &str = "Admin";

/// Storage key for token ID
pub const TOKEN_ID_STORAGE_KEY: &str = "TokenID";

/// Storage key for fee configuration
pub const FEE_CONFIG_STORAGE_KEY: &str = "FeeConfig";

/// Storage key for resolution analytics
pub const RESOLUTION_ANALYTICS_STORAGE_KEY: &str = "ResolutionAnalytics";

/// Storage key for oracle statistics
pub const ORACLE_STATS_STORAGE_KEY: &str = "OracleStats";

// ===== CONFIGURATION STRUCTS =====

/// Environment type enumeration
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[contracttype]
pub enum Environment {
    /// Development environment
    Development,
    /// Testnet environment
    Testnet,
    /// Mainnet environment
    Mainnet,
    /// Custom environment
    Custom,
}

/// Network configuration
#[derive(Clone, Debug)]
#[contracttype]
pub struct NetworkConfig {
    /// Network environment
    pub environment: Environment,
    /// Network passphrase
    pub passphrase: String,
    /// RPC URL
    pub rpc_url: String,
    /// Network ID
    pub network_id: String,
    /// Contract deployment address
    pub contract_address: Address,
}

/// Fee configuration
#[derive(Clone, Debug)]
#[contracttype]
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

/// Voting configuration
#[derive(Clone, Debug)]
#[contracttype]
pub struct VotingConfig {
    /// Minimum vote stake
    pub min_vote_stake: i128,
    /// Minimum dispute stake
    pub min_dispute_stake: i128,
    /// Maximum dispute threshold
    pub max_dispute_threshold: i128,
    /// Base dispute threshold
    pub base_dispute_threshold: i128,
    /// Large market threshold
    pub large_market_threshold: i128,
    /// High activity threshold
    pub high_activity_threshold: u32,
    /// Dispute extension hours
    pub dispute_extension_hours: u32,
}

/// Market configuration
#[derive(Clone, Debug)]
#[contracttype]
pub struct MarketConfig {
    /// Maximum market duration in days
    pub max_duration_days: u32,
    /// Minimum market duration in days
    pub min_duration_days: u32,
    /// Maximum number of outcomes
    pub max_outcomes: u32,
    /// Minimum number of outcomes
    pub min_outcomes: u32,
    /// Maximum question length
    pub max_question_length: u32,
    /// Maximum outcome length
    pub max_outcome_length: u32,
}

/// Extension configuration
#[derive(Clone, Debug)]
#[contracttype]
pub struct ExtensionConfig {
    /// Maximum extension days
    pub max_extension_days: u32,
    /// Minimum extension days
    pub min_extension_days: u32,
    /// Extension fee per day
    pub fee_per_day: i128,
    /// Maximum total extensions
    pub max_total_extensions: u32,
}

/// Resolution configuration
#[derive(Clone, Debug)]
#[contracttype]
pub struct ResolutionConfig {
    /// Minimum confidence score
    pub min_confidence_score: u32,
    /// Maximum confidence score
    pub max_confidence_score: u32,
    /// Oracle weight percentage
    pub oracle_weight_percentage: u32,
    /// Community weight percentage
    pub community_weight_percentage: u32,
    /// Minimum votes for consensus
    pub min_votes_for_consensus: u32,
}

/// Oracle configuration
#[derive(Clone, Debug)]
#[contracttype]
pub struct OracleConfig {
    /// Maximum oracle price age
    pub max_price_age: u64,
    /// Oracle retry attempts
    pub retry_attempts: u32,
    /// Oracle timeout seconds
    pub timeout_seconds: u64,
}

/// Complete contract configuration
#[derive(Clone, Debug)]
#[contracttype]
pub struct ContractConfig {
    /// Network configuration
    pub network: NetworkConfig,
    /// Fee configuration
    pub fees: FeeConfig,
    /// Voting configuration
    pub voting: VotingConfig,
    /// Market configuration
    pub market: MarketConfig,
    /// Extension configuration
    pub extension: ExtensionConfig,
    /// Resolution configuration
    pub resolution: ResolutionConfig,
    /// Oracle configuration
    pub oracle: OracleConfig,
}

// ===== CONFIGURATION MANAGER =====

/// Configuration management utilities
pub struct ConfigManager;

impl ConfigManager {
    /// Get default configuration for development environment
    pub fn get_development_config(env: &Env) -> ContractConfig {
        ContractConfig {
            network: NetworkConfig {
                environment: Environment::Development,
                passphrase: String::from_str(env, "Test SDF Network ; September 2015"),
                rpc_url: String::from_str(env, "https://soroban-testnet.stellar.org"),
                network_id: String::from_str(env, "testnet"),
                contract_address: Address::from_str(
                    env,
                    "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
                ),
            },
            fees: Self::get_default_fee_config(),
            voting: Self::get_default_voting_config(),
            market: Self::get_default_market_config(),
            extension: Self::get_default_extension_config(),
            resolution: Self::get_default_resolution_config(),
            oracle: Self::get_default_oracle_config(),
        }
    }

    /// Get default configuration for testnet environment
    pub fn get_testnet_config(env: &Env) -> ContractConfig {
        ContractConfig {
            network: NetworkConfig {
                environment: Environment::Testnet,
                passphrase: String::from_str(env, "Test SDF Network ; September 2015"),
                rpc_url: String::from_str(env, "https://soroban-testnet.stellar.org"),
                network_id: String::from_str(env, "testnet"),
                contract_address: Address::from_str(
                    env,
                    "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
                ),
            },
            fees: Self::get_default_fee_config(),
            voting: Self::get_default_voting_config(),
            market: Self::get_default_market_config(),
            extension: Self::get_default_extension_config(),
            resolution: Self::get_default_resolution_config(),
            oracle: Self::get_default_oracle_config(),
        }
    }

    /// Get default configuration for mainnet environment
    pub fn get_mainnet_config(env: &Env) -> ContractConfig {
        ContractConfig {
            network: NetworkConfig {
                environment: Environment::Mainnet,
                passphrase: String::from_str(env, "Public Global Stellar Network ; September 2015"),
                rpc_url: String::from_str(env, "https://rpc.mainnet.stellar.org"),
                network_id: String::from_str(env, "mainnet"),
                contract_address: Address::from_str(
                    env,
                    "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
                ),
            },
            fees: Self::get_mainnet_fee_config(),
            voting: Self::get_mainnet_voting_config(),
            market: Self::get_default_market_config(),
            extension: Self::get_default_extension_config(),
            resolution: Self::get_default_resolution_config(),
            oracle: Self::get_mainnet_oracle_config(),
        }
    }

    /// Get default fee configuration
    pub fn get_default_fee_config() -> FeeConfig {
        FeeConfig {
            platform_fee_percentage: DEFAULT_PLATFORM_FEE_PERCENTAGE,
            creation_fee: DEFAULT_MARKET_CREATION_FEE,
            min_fee_amount: MIN_FEE_AMOUNT,
            max_fee_amount: MAX_FEE_AMOUNT,
            collection_threshold: FEE_COLLECTION_THRESHOLD,
            fees_enabled: true,
        }
    }

    /// Get mainnet fee configuration (higher fees)
    pub fn get_mainnet_fee_config() -> FeeConfig {
        FeeConfig {
            platform_fee_percentage: 3,        // 3% for mainnet
            creation_fee: 15_000_000,          // 1.5 XLM for mainnet
            min_fee_amount: 2_000_000,         // 0.2 XLM for mainnet
            max_fee_amount: 2_000_000_000,     // 200 XLM for mainnet
            collection_threshold: 200_000_000, // 20 XLM for mainnet
            fees_enabled: true,
        }
    }

    /// Get default voting configuration
    pub fn get_default_voting_config() -> VotingConfig {
        VotingConfig {
            min_vote_stake: MIN_VOTE_STAKE,
            min_dispute_stake: MIN_DISPUTE_STAKE,
            max_dispute_threshold: MAX_DISPUTE_THRESHOLD,
            base_dispute_threshold: BASE_DISPUTE_THRESHOLD,
            large_market_threshold: LARGE_MARKET_THRESHOLD,
            high_activity_threshold: HIGH_ACTIVITY_THRESHOLD,
            dispute_extension_hours: DISPUTE_EXTENSION_HOURS,
        }
    }

    /// Get mainnet voting configuration (higher stakes)
    pub fn get_mainnet_voting_config() -> VotingConfig {
        VotingConfig {
            min_vote_stake: 2_000_000,             // 0.2 XLM for mainnet
            min_dispute_stake: 20_000_000,         // 2 XLM for mainnet
            max_dispute_threshold: 200_000_000,    // 20 XLM for mainnet
            base_dispute_threshold: 20_000_000,    // 2 XLM for mainnet
            large_market_threshold: 2_000_000_000, // 200 XLM for mainnet
            high_activity_threshold: 200,          // 200 votes for mainnet
            dispute_extension_hours: 48,           // 48 hours for mainnet
        }
    }

    /// Get default market configuration
    pub fn get_default_market_config() -> MarketConfig {
        MarketConfig {
            max_duration_days: MAX_MARKET_DURATION_DAYS,
            min_duration_days: MIN_MARKET_DURATION_DAYS,
            max_outcomes: MAX_MARKET_OUTCOMES,
            min_outcomes: MIN_MARKET_OUTCOMES,
            max_question_length: MAX_QUESTION_LENGTH,
            max_outcome_length: MAX_OUTCOME_LENGTH,
        }
    }

    /// Get default extension configuration
    pub fn get_default_extension_config() -> ExtensionConfig {
        ExtensionConfig {
            max_extension_days: MAX_EXTENSION_DAYS,
            min_extension_days: MIN_EXTENSION_DAYS,
            fee_per_day: EXTENSION_FEE_PER_DAY,
            max_total_extensions: MAX_TOTAL_EXTENSIONS,
        }
    }

    /// Get default resolution configuration
    pub fn get_default_resolution_config() -> ResolutionConfig {
        ResolutionConfig {
            min_confidence_score: MIN_CONFIDENCE_SCORE,
            max_confidence_score: MAX_CONFIDENCE_SCORE,
            oracle_weight_percentage: ORACLE_WEIGHT_PERCENTAGE,
            community_weight_percentage: COMMUNITY_WEIGHT_PERCENTAGE,
            min_votes_for_consensus: MIN_VOTES_FOR_CONSENSUS,
        }
    }

    /// Get default oracle configuration
    pub fn get_default_oracle_config() -> OracleConfig {
        OracleConfig {
            max_price_age: MAX_ORACLE_PRICE_AGE,
            retry_attempts: ORACLE_RETRY_ATTEMPTS,
            timeout_seconds: ORACLE_TIMEOUT_SECONDS,
        }
    }

    /// Get mainnet oracle configuration (stricter requirements)
    pub fn get_mainnet_oracle_config() -> OracleConfig {
        OracleConfig {
            max_price_age: 1800, // 30 minutes for mainnet
            retry_attempts: 5,   // More retries for mainnet
            timeout_seconds: 60, // Longer timeout for mainnet
        }
    }

    /// Store configuration in contract storage
    pub fn store_config(env: &Env, config: &ContractConfig) -> Result<(), Error> {
        let key = Symbol::new(env, "ContractConfig");
        env.storage().persistent().set(&key, config);
        Ok(())
    }

    /// Retrieve configuration from contract storage
    pub fn get_config(env: &Env) -> Result<ContractConfig, Error> {
        let key = Symbol::new(env, "ContractConfig");
        env.storage()
            .persistent()
            .get::<Symbol, ContractConfig>(&key)
            .ok_or(Error::ConfigurationNotFound)
    }

    /// Update configuration in contract storage
    pub fn update_config(env: &Env, config: &ContractConfig) -> Result<(), Error> {
        Self::store_config(env, config)
    }

    /// Reset configuration to defaults
    pub fn reset_to_defaults(env: &Env) -> Result<ContractConfig, Error> {
        let config = Self::get_development_config(env);
        Self::store_config(env, &config)?;
        Ok(config)
    }
}

// ===== CONFIGURATION VALIDATOR =====

/// Configuration validation utilities
pub struct ConfigValidator;

impl ConfigValidator {
    /// Validate complete contract configuration
    pub fn validate_contract_config(config: &ContractConfig) -> Result<(), Error> {
        Self::validate_fee_config(&config.fees)?;
        Self::validate_voting_config(&config.voting)?;
        Self::validate_market_config(&config.market)?;
        Self::validate_extension_config(&config.extension)?;
        Self::validate_resolution_config(&config.resolution)?;
        Self::validate_oracle_config(&config.oracle)?;
        Ok(())
    }

    /// Validate fee configuration
    pub fn validate_fee_config(config: &FeeConfig) -> Result<(), Error> {
        if config.platform_fee_percentage < MIN_PLATFORM_FEE_PERCENTAGE
            || config.platform_fee_percentage > MAX_PLATFORM_FEE_PERCENTAGE
        {
            return Err(Error::InvalidFeeConfig);
        }

        if config.min_fee_amount > config.max_fee_amount {
            return Err(Error::InvalidFeeConfig);
        }

        if config.creation_fee < config.min_fee_amount
            || config.creation_fee > config.max_fee_amount
        {
            return Err(Error::InvalidFeeConfig);
        }

        if config.collection_threshold <= 0 {
            return Err(Error::InvalidFeeConfig);
        }

        Ok(())
    }

    /// Validate voting configuration
    pub fn validate_voting_config(config: &VotingConfig) -> Result<(), Error> {
        if config.min_vote_stake <= 0 {
            return Err(Error::InvalidInput);
        }

        if config.min_dispute_stake <= 0 {
            return Err(Error::InvalidInput);
        }

        if config.max_dispute_threshold < config.base_dispute_threshold {
            return Err(Error::InvalidInput);
        }

        if config.large_market_threshold <= 0 {
            return Err(Error::InvalidInput);
        }

        if config.high_activity_threshold == 0 {
            return Err(Error::InvalidInput);
        }

        if config.dispute_extension_hours == 0 {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }

    /// Validate market configuration
    pub fn validate_market_config(config: &MarketConfig) -> Result<(), Error> {
        if config.max_duration_days < config.min_duration_days {
            return Err(Error::InvalidInput);
        }

        if config.max_outcomes < config.min_outcomes {
            return Err(Error::InvalidInput);
        }

        if config.max_question_length == 0 {
            return Err(Error::InvalidInput);
        }

        if config.max_outcome_length == 0 {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }

    /// Validate extension configuration
    pub fn validate_extension_config(config: &ExtensionConfig) -> Result<(), Error> {
        if config.max_extension_days < config.min_extension_days {
            return Err(Error::InvalidInput);
        }

        if config.fee_per_day <= 0 {
            return Err(Error::InvalidInput);
        }

        if config.max_total_extensions == 0 {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }

    /// Validate resolution configuration
    pub fn validate_resolution_config(config: &ResolutionConfig) -> Result<(), Error> {
        if config.min_confidence_score > config.max_confidence_score {
            return Err(Error::InvalidInput);
        }

        if config.oracle_weight_percentage + config.community_weight_percentage != 100 {
            return Err(Error::InvalidInput);
        }

        if config.min_votes_for_consensus == 0 {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }

    /// Validate oracle configuration
    pub fn validate_oracle_config(config: &OracleConfig) -> Result<(), Error> {
        if config.max_price_age == 0 {
            return Err(Error::InvalidInput);
        }

        if config.retry_attempts == 0 {
            return Err(Error::InvalidInput);
        }

        if config.timeout_seconds == 0 {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }
}

// ===== CONFIGURATION UTILS =====

/// Configuration utility functions
pub struct ConfigUtils;

impl ConfigUtils {
    /// Check if configuration is for mainnet
    pub fn is_mainnet(config: &ContractConfig) -> bool {
        matches!(config.network.environment, Environment::Mainnet)
    }

    /// Check if configuration is for testnet
    pub fn is_testnet(config: &ContractConfig) -> bool {
        matches!(config.network.environment, Environment::Testnet)
    }

    /// Check if configuration is for development
    pub fn is_development(config: &ContractConfig) -> bool {
        matches!(config.network.environment, Environment::Development)
    }

    /// Get environment name as string
    pub fn get_environment_name(config: &ContractConfig) -> String {
        match config.network.environment {
            Environment::Development => {
                String::from_str(&config.network.passphrase.env(), "development")
            }
            Environment::Testnet => String::from_str(&config.network.passphrase.env(), "testnet"),
            Environment::Mainnet => String::from_str(&config.network.passphrase.env(), "mainnet"),
            Environment::Custom => String::from_str(&config.network.passphrase.env(), "custom"),
        }
    }

    /// Get configuration summary
    pub fn get_config_summary(config: &ContractConfig) -> String {
        let env_name = Self::get_environment_name(config);
        let fee_percentage = config.fees.platform_fee_percentage;

        // Create simple summary since string concatenation is complex in no_std
        if fee_percentage == 2 {
            String::from_str(&env_name.env(), "Development config with 2% fees")
        } else if fee_percentage == 3 {
            String::from_str(&env_name.env(), "Mainnet config with 3% fees")
        } else {
            String::from_str(&env_name.env(), "Custom config")
        }
    }

    /// Check if fees are enabled
    pub fn fees_enabled(config: &ContractConfig) -> bool {
        config.fees.fees_enabled
    }

    /// Get fee configuration
    pub fn get_fee_config(config: &ContractConfig) -> &FeeConfig {
        &config.fees
    }

    /// Get voting configuration
    pub fn get_voting_config(config: &ContractConfig) -> &VotingConfig {
        &config.voting
    }

    /// Get market configuration
    pub fn get_market_config(config: &ContractConfig) -> &MarketConfig {
        &config.market
    }

    /// Get extension configuration
    pub fn get_extension_config(config: &ContractConfig) -> &ExtensionConfig {
        &config.extension
    }

    /// Get resolution configuration
    pub fn get_resolution_config(config: &ContractConfig) -> &ResolutionConfig {
        &config.resolution
    }

    /// Get oracle configuration
    pub fn get_oracle_config(config: &ContractConfig) -> &OracleConfig {
        &config.oracle
    }
}

// ===== CONFIGURATION TESTING =====

/// Configuration testing utilities
pub struct ConfigTesting;

impl ConfigTesting {
    /// Create test configuration for development
    pub fn create_test_config(env: &Env) -> ContractConfig {
        ConfigManager::get_development_config(env)
    }

    /// Create test configuration for mainnet
    pub fn create_mainnet_test_config(env: &Env) -> ContractConfig {
        ConfigManager::get_mainnet_config(env)
    }

    /// Validate test configuration structure
    pub fn validate_test_config_structure(config: &ContractConfig) -> Result<(), Error> {
        ConfigValidator::validate_contract_config(config)
    }

    /// Create minimal test configuration
    pub fn create_minimal_test_config(env: &Env) -> ContractConfig {
        ContractConfig {
            network: NetworkConfig {
                environment: Environment::Development,
                passphrase: String::from_str(env, "Test"),
                rpc_url: String::from_str(env, "http://localhost"),
                network_id: String::from_str(env, "test"),
                contract_address: Address::from_str(
                    env,
                    "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
                ),
            },
            fees: FeeConfig {
                platform_fee_percentage: 1,
                creation_fee: 5_000_000,
                min_fee_amount: 500_000,
                max_fee_amount: 500_000_000,
                collection_threshold: 50_000_000,
                fees_enabled: true,
            },
            voting: VotingConfig {
                min_vote_stake: 500_000,
                min_dispute_stake: 5_000_000,
                max_dispute_threshold: 50_000_000,
                base_dispute_threshold: 5_000_000,
                large_market_threshold: 500_000_000,
                high_activity_threshold: 50,
                dispute_extension_hours: 12,
            },
            market: MarketConfig {
                max_duration_days: 30,
                min_duration_days: 1,
                max_outcomes: 5,
                min_outcomes: 2,
                max_question_length: 200,
                max_outcome_length: 50,
            },
            extension: ExtensionConfig {
                max_extension_days: 7,
                min_extension_days: 1,
                fee_per_day: 50_000_000,
                max_total_extensions: 2,
            },
            resolution: ResolutionConfig {
                min_confidence_score: 0,
                max_confidence_score: 100,
                oracle_weight_percentage: 60,
                community_weight_percentage: 40,
                min_votes_for_consensus: 3,
            },
            oracle: OracleConfig {
                max_price_age: 1800,
                retry_attempts: 2,
                timeout_seconds: 15,
            },
        }
    }
}

// ===== MODULE TESTS =====

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_manager_default_configs() {
        let env = Env::default();

        // Test development config
        let dev_config = ConfigManager::get_development_config(&env);
        assert_eq!(dev_config.network.environment, Environment::Development);
        assert_eq!(
            dev_config.fees.platform_fee_percentage,
            DEFAULT_PLATFORM_FEE_PERCENTAGE
        );

        // Test testnet config
        let testnet_config = ConfigManager::get_testnet_config(&env);
        assert_eq!(testnet_config.network.environment, Environment::Testnet);

        // Test mainnet config
        let mainnet_config = ConfigManager::get_mainnet_config(&env);
        assert_eq!(mainnet_config.network.environment, Environment::Mainnet);
        assert_eq!(mainnet_config.fees.platform_fee_percentage, 3);
    }

    #[test]
    fn test_config_validator() {
        let env = Env::default();
        let config = ConfigManager::get_development_config(&env);

        // Test valid configuration
        assert!(ConfigValidator::validate_contract_config(&config).is_ok());

        // Test invalid fee configuration
        let mut invalid_config = config.clone();
        invalid_config.fees.platform_fee_percentage = 15; // Too high
        assert!(ConfigValidator::validate_contract_config(&invalid_config).is_err());

        // Test invalid voting configuration
        let mut invalid_config = config.clone();
        invalid_config.voting.min_vote_stake = 0; // Invalid
        assert!(ConfigValidator::validate_contract_config(&invalid_config).is_err());
    }

    #[test]
    fn test_config_utils() {
        let env = Env::default();
        let dev_config = ConfigManager::get_development_config(&env);
        let mainnet_config = ConfigManager::get_mainnet_config(&env);

        // Test environment detection
        assert!(ConfigUtils::is_development(&dev_config));
        assert!(!ConfigUtils::is_mainnet(&dev_config));
        assert!(ConfigUtils::is_mainnet(&mainnet_config));

        // Test fee enabled check
        assert!(ConfigUtils::fees_enabled(&dev_config));
        assert!(ConfigUtils::fees_enabled(&mainnet_config));

        // Test configuration access
        assert_eq!(
            ConfigUtils::get_fee_config(&dev_config).platform_fee_percentage,
            2
        );
        assert_eq!(
            ConfigUtils::get_fee_config(&mainnet_config).platform_fee_percentage,
            3
        );
    }

    #[test]
    fn test_config_storage() {
        let env = Env::default();
        let config = ConfigManager::get_development_config(&env);

        // Test storage and retrieval
        assert!(ConfigManager::store_config(&env, &config).is_ok());
        let retrieved_config = ConfigManager::get_config(&env).unwrap();
        assert_eq!(
            retrieved_config.fees.platform_fee_percentage,
            config.fees.platform_fee_percentage
        );

        // Test reset to defaults
        let reset_config = ConfigManager::reset_to_defaults(&env).unwrap();
        assert_eq!(
            reset_config.fees.platform_fee_percentage,
            DEFAULT_PLATFORM_FEE_PERCENTAGE
        );
    }

    #[test]
    fn test_config_testing() {
        let env = Env::default();

        // Test test configuration creation
        let test_config = ConfigTesting::create_test_config(&env);
        assert!(ConfigTesting::validate_test_config_structure(&test_config).is_ok());

        // Test mainnet test configuration
        let mainnet_test_config = ConfigTesting::create_mainnet_test_config(&env);
        assert!(ConfigTesting::validate_test_config_structure(&mainnet_test_config).is_ok());

        // Test minimal test configuration
        let minimal_config = ConfigTesting::create_minimal_test_config(&env);
        assert!(ConfigTesting::validate_test_config_structure(&minimal_config).is_ok());
        assert_eq!(minimal_config.fees.platform_fee_percentage, 1);
    }

    #[test]
    fn test_environment_enum() {
        let env = Env::default();

        // Test environment creation
        let dev_env = Environment::Development;
        let _testnet_env = Environment::Testnet;
        let mainnet_env = Environment::Mainnet;
        let _custom_env = Environment::Custom;

        // Test environment comparison
        assert_eq!(dev_env, Environment::Development);
        assert_ne!(dev_env, mainnet_env);

        // Test environment in configuration
        let config = ConfigManager::get_development_config(&env);
        assert_eq!(config.network.environment, dev_env);
    }

    #[test]
    fn test_configuration_constants() {
        // Test fee constants
        assert_eq!(DEFAULT_PLATFORM_FEE_PERCENTAGE, 2);
        assert_eq!(DEFAULT_MARKET_CREATION_FEE, 10_000_000);
        assert_eq!(MIN_FEE_AMOUNT, 1_000_000);
        assert_eq!(MAX_FEE_AMOUNT, 1_000_000_000);

        // Test voting constants
        assert_eq!(MIN_VOTE_STAKE, 1_000_000);
        assert_eq!(MIN_DISPUTE_STAKE, 10_000_000);
        assert_eq!(DISPUTE_EXTENSION_HOURS, 24);

        // Test market constants
        assert_eq!(MAX_MARKET_DURATION_DAYS, 365);
        assert_eq!(MIN_MARKET_DURATION_DAYS, 1);
        assert_eq!(MAX_MARKET_OUTCOMES, 10);
        assert_eq!(MIN_MARKET_OUTCOMES, 2);

        // Test extension constants
        assert_eq!(MAX_EXTENSION_DAYS, 30);
        assert_eq!(MIN_EXTENSION_DAYS, 1);
        assert_eq!(EXTENSION_FEE_PER_DAY, 100_000_000);

        // Test resolution constants
        assert_eq!(MIN_CONFIDENCE_SCORE, 0);
        assert_eq!(MAX_CONFIDENCE_SCORE, 100);
        assert_eq!(ORACLE_WEIGHT_PERCENTAGE, 70);
        assert_eq!(COMMUNITY_WEIGHT_PERCENTAGE, 30);

        // Test oracle constants
        assert_eq!(MAX_ORACLE_PRICE_AGE, 3600);
        assert_eq!(ORACLE_RETRY_ATTEMPTS, 3);
        assert_eq!(ORACLE_TIMEOUT_SECONDS, 30);
    }
}
