#![allow(dead_code)]

use soroban_sdk::{contracttype, Address, Env, Map, String, Symbol, Vec};

// ===== MARKET STATE =====

/// Market state enumeration
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MarketState {
    /// Market is active and accepting votes
    Active,
    /// Market has ended, waiting for resolution
    Ended,
    /// Market is under dispute
    Disputed,
    /// Market has been resolved
    Resolved,
    /// Market is closed
    Closed,
    /// Market has been cancelled
    Cancelled,
}

// ===== ORACLE TYPES =====

/// Oracle provider enumeration
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OracleProvider {
    /// Reflector oracle (primary oracle for Stellar Network)
    Reflector,
    /// Pyth Network oracle (placeholder for Stellar)
    Pyth,
    /// Band Protocol oracle (not available on Stellar)
    BandProtocol,
    /// DIA oracle (not available on Stellar)
    DIA,
}

impl OracleProvider {
    /// Get provider name
    pub fn name(&self) -> &'static str {
        match self {
            OracleProvider::Reflector => "Reflector",
            OracleProvider::Pyth => "Pyth",
            OracleProvider::BandProtocol => "Band Protocol",
            OracleProvider::DIA => "DIA",
        }
    }

    /// Check if provider is supported on Stellar
    pub fn is_supported(&self) -> bool {
        matches!(self, OracleProvider::Reflector)
    }
}

/// Oracle configuration for markets
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleConfig {
    /// The oracle provider to use
    pub provider: OracleProvider,
    /// Oracle-specific identifier (e.g., "BTC/USD" for Pyth, "BTC" for Reflector)
    pub feed_id: String,
    /// Price threshold in cents (e.g., 10_000_00 = $10k)
    pub threshold: i128,
    /// Comparison operator: "gt", "lt", "eq"
    pub comparison: String,
}

impl OracleConfig {
    /// Create a new oracle configuration
    pub fn new(
        provider: OracleProvider,
        feed_id: String,
        threshold: i128,
        comparison: String,
    ) -> Self {
        Self {
            provider,
            feed_id,
            threshold,
            comparison,
        }
    }

    /// Validate the oracle configuration
    pub fn validate(&self, env: &Env) -> Result<(), crate::Error> {
        // Validate threshold
        if self.threshold <= 0 {
            return Err(crate::Error::InvalidThreshold);
        }

        // Validate comparison operator
        if self.comparison != String::from_str(env, "gt")
            && self.comparison != String::from_str(env, "lt")
            && self.comparison != String::from_str(env, "eq")
        {
            return Err(crate::Error::InvalidComparison);
        }

        // Validate provider is supported
        if !self.provider.is_supported() {
            return Err(crate::Error::InvalidOracleConfig);
        }

        Ok(())
    }
}

// ===== MARKET TYPES =====

/// Market state and data structure
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Market {
    /// Market administrator address
    pub admin: Address,
    /// Market question/prediction
    pub question: String,
    /// Available outcomes for the market
    pub outcomes: Vec<String>,
    /// Market end time (Unix timestamp)
    pub end_time: u64,
    /// Oracle configuration for this market
    pub oracle_config: OracleConfig,
    /// Oracle result (set after market ends)
    pub oracle_result: Option<String>,
    /// User votes mapping (address -> outcome)
    pub votes: Map<Address, String>,
    /// User stakes mapping (address -> stake amount)
    pub stakes: Map<Address, i128>,
    /// Claimed status mapping (address -> claimed)
    pub claimed: Map<Address, bool>,
    /// Total amount staked in the market
    pub total_staked: i128,
    /// Dispute stakes mapping (address -> dispute stake)
    pub dispute_stakes: Map<Address, i128>,
    /// Winning outcome (set after resolution)
    pub winning_outcome: Option<String>,
    /// Whether fees have been collected
    pub fee_collected: bool,
    /// Current market state
    pub state: MarketState,
    /// Total extension days
    pub total_extension_days: u32,
    /// Maximum extension days allowed
    pub max_extension_days: u32,
    /// Extension history
    pub extension_history: Vec<MarketExtension>,
}

impl Market {
    /// Create a new market
    pub fn new(
        env: &Env,
        admin: Address,
        question: String,
        outcomes: Vec<String>,
        end_time: u64,
        oracle_config: OracleConfig,
        state: MarketState,
    ) -> Self {
        Self {
            admin,
            question,
            outcomes,
            end_time,
            oracle_config,
            oracle_result: None,
            votes: Map::new(env),
            stakes: Map::new(env),
            claimed: Map::new(env),
            total_staked: 0,
            dispute_stakes: Map::new(env),
            winning_outcome: None,
            fee_collected: false,
            state,
            total_extension_days: 0,
            max_extension_days: 30, // Default maximum extension days
            extension_history: Vec::new(env),
        }
    }

    /// Check if the market is active (not ended)
    pub fn is_active(&self, current_time: u64) -> bool {
        current_time < self.end_time
    }

    /// Check if the market has ended
    pub fn has_ended(&self, current_time: u64) -> bool {
        current_time >= self.end_time
    }

    /// Check if the market is resolved
    pub fn is_resolved(&self) -> bool {
        self.winning_outcome.is_some()
    }

    /// Get total dispute stakes for the market
    pub fn total_dispute_stakes(&self) -> i128 {
        let mut total = 0;
        for (_, stake) in self.dispute_stakes.iter() {
            total += stake;
        }
        total
    }

    /// Add a vote to the market (for testing)
    pub fn add_vote(&mut self, user: Address, outcome: String, stake: i128) {
        self.votes.set(user.clone(), outcome);
        self.stakes.set(user, stake);
        self.total_staked += stake;
    }

    /// Validate market parameters
    pub fn validate(&self, env: &Env) -> Result<(), crate::Error> {
        // Validate question
        if self.question.is_empty() {
            return Err(crate::Error::InvalidQuestion);
        }

        // Validate outcomes
        if self.outcomes.len() < 2 {
            return Err(crate::Error::InvalidOutcomes);
        }

        // Validate oracle config
        self.oracle_config.validate(env)?;

        // Validate end time
        if self.end_time <= env.ledger().timestamp() {
            return Err(crate::Error::InvalidDuration);
        }

        Ok(())
    }
}

// ===== REFLECTOR ORACLE TYPES =====

/// Reflector asset enumeration
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReflectorAsset {
    /// Stellar Lumens (XLM)
    Stellar,
    /// Other asset identified by symbol
    Other(Symbol),
}

/// Reflector price data structure
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReflectorPriceData {
    /// Price value in cents (e.g., 2500000 = $25,000)
    pub price: i128,
    /// Timestamp of price update
    pub timestamp: u64,
    /// Price source/confidence
    pub source: String,
}

// ===== MARKET EXTENSION TYPES =====

/// Market extension data structure
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketExtension {
    /// Number of additional days
    pub additional_days: u32,
    /// Administrator who requested the extension
    pub admin: Address,
    /// Reason for the extension
    pub reason: String,
    /// Fee amount paid
    pub fee_amount: i128,
    /// Extension timestamp
    pub timestamp: u64,
}

impl MarketExtension {
    /// Create a new market extension
    pub fn new(
        env: &Env,
        additional_days: u32,
        admin: Address,
        reason: String,
        fee_amount: i128,
    ) -> Self {
        Self {
            additional_days,
            admin,
            reason,
            fee_amount,
            timestamp: env.ledger().timestamp(),
        }
    }
}

/// Extension statistics
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtensionStats {
    /// Total number of extensions
    pub total_extensions: u32,
    /// Total extension days
    pub total_extension_days: u32,
    /// Maximum extension days allowed
    pub max_extension_days: u32,
    /// Whether the market can be extended
    pub can_extend: bool,
    /// Extension fee per day
    pub extension_fee_per_day: i128,
}

// ===== MARKET CREATION TYPES =====

/// Market creation parameters
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketCreationParams {
    /// Market administrator address
    pub admin: Address,
    /// Market question/prediction
    pub question: String,
    /// Available outcomes for the market
    pub outcomes: Vec<String>,
    /// Market duration in days
    pub duration_days: u32,
    /// Oracle configuration for this market
    pub oracle_config: OracleConfig,
    /// Creation fee amount
    pub creation_fee: i128,
}

impl MarketCreationParams {
    /// Create new market creation parameters
    pub fn new(
        admin: Address,
        question: String,
        outcomes: Vec<String>,
        duration_days: u32,
        oracle_config: OracleConfig,
        creation_fee: i128,
    ) -> Self {
        Self {
            admin,
            question,
            outcomes,
            duration_days,
            oracle_config,
            creation_fee,
        }
    }
}

// ===== ADDITIONAL TYPES =====

/// Community consensus data
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommunityConsensus {
    /// Consensus outcome
    pub outcome: String,
    /// Number of votes for this outcome
    pub votes: u32,
    /// Total number of votes
    pub total_votes: u32,
    /// Percentage of votes for this outcome
    pub percentage: i128,
}

/// Oracle result enumeration
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OracleResult {
    /// Oracle returned a price
    Price(i128),
    /// Oracle is unavailable
    Unavailable,
    /// Oracle data is stale
    Stale,
}

impl OracleResult {
    /// Create from price
    pub fn price(price: i128) -> Self {
        OracleResult::Price(price)
    }

    /// Create unavailable result
    pub fn unavailable() -> Self {
        OracleResult::Unavailable
    }

    /// Create stale result
    pub fn stale() -> Self {
        OracleResult::Stale
    }

    /// Check if result is available
    pub fn is_available(&self) -> bool {
        matches!(self, OracleResult::Price(_))
    }

    /// Get price if available
    pub fn get_price(&self) -> Option<i128> {
        match self {
            OracleResult::Price(price) => Some(*price),
            _ => None,
        }
    }
}

// ===== HELPER FUNCTIONS =====

/// Type validation helpers
pub mod validation {
    use super::*;

    /// Validate oracle provider
    pub fn validate_oracle_provider(provider: &OracleProvider) -> Result<(), crate::errors::Error> {
        if !provider.is_supported() {
            return Err(crate::errors::Error::InvalidOracleConfig);
        }
        Ok(())
    }

    /// Validate price data
    pub fn validate_price(price: i128) -> Result<(), crate::errors::Error> {
        if price <= 0 {
            return Err(crate::errors::Error::OraclePriceOutOfRange);
        }
        Ok(())
    }

    /// Validate stake amount
    pub fn validate_stake(stake: i128, min_stake: i128) -> Result<(), crate::errors::Error> {
        if stake < min_stake {
            return Err(crate::errors::Error::InsufficientStake);
        }
        Ok(())
    }

    /// Validate market duration
    pub fn validate_duration(duration_days: u32) -> Result<(), crate::errors::Error> {
        if duration_days == 0 || duration_days > 365 {
            return Err(crate::errors::Error::InvalidDuration);
        }
        Ok(())
    }
}

/// Type conversion helpers
pub mod conversion {
    use super::*;

    /// Convert string to oracle provider
    pub fn string_to_oracle_provider(s: &str) -> Option<OracleProvider> {
        match s.to_lowercase().as_str() {
            "band" | "bandprotocol" => Some(OracleProvider::BandProtocol),
            "dia" => Some(OracleProvider::DIA),
            "reflector" => Some(OracleProvider::Reflector),
            "pyth" => Some(OracleProvider::Pyth),
            _ => None,
        }
    }

    /// Convert oracle provider to string
    pub fn oracle_provider_to_string(provider: &OracleProvider) -> &'static str {
        provider.name()
    }

    /// Convert comparison string to validation
    pub fn validate_comparison(comparison: &String, env: &Env) -> Result<(), crate::errors::Error> {
        if comparison != &String::from_str(env, "gt")
            && comparison != &String::from_str(env, "lt")
            && comparison != &String::from_str(env, "eq")
        {
            return Err(crate::errors::Error::InvalidComparison);
        }
        Ok(())
    }
}

// ===== MONITORING TYPES =====

/// Alert information emitted by monitoring functions
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MonitoringAlert {
    /// Type/code of alert
    pub alert_type: String,
    /// Human-readable message
    pub message: String,
    /// Severity: 0 = info, 1 = warning, 2 = critical
    pub severity: u32,
    /// Unix timestamp
    pub timestamp: u64,
}

/// Generic monitoring data used for validation/logging
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MonitoringData {
    /// Market health information
    MarketHealth { market_id: Symbol, liquidity: i128, open_interest: i128 },
    /// Oracle provider status
    OracleHealth { provider: String, is_online: bool },
    /// Fee revenue within timeframe
    FeeRevenue { timeframe: TimeFrame, amount: i128 },
    /// Dispute information
    DisputeStatus { market_id: Symbol, open_disputes: u32 },
    /// Contract performance metrics
    PerformanceMetrics { tx_count: u32, avg_gas: i128 },
    /// Custom payload
    Custom(String),
}

/// Time frame definitions for monitoring analytics
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TimeFrame {
    LastHour,
    LastDay,
    LastWeek,
    Custom(u64),
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn test_oracle_provider() {
        let provider = OracleProvider::Pyth;
        assert_eq!(provider.name(), "Pyth");
        assert!(!provider.is_supported()); // Only Reflector is supported
    }

    #[test]
    fn test_oracle_config() {
        let env = soroban_sdk::Env::default();
        let config = OracleConfig::new(
            OracleProvider::Reflector,
            String::from_str(&env, "BTC"),
            2500000,
            String::from_str(&env, "gt"),
        );

        assert!(config.validate(&env).is_ok());
    }

    #[test]
    fn test_market_creation() {
        let env = soroban_sdk::Env::default();
        let admin = Address::generate(&env);
        let outcomes = vec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ];
        let oracle_config = OracleConfig::new(
            OracleProvider::Reflector,
            String::from_str(&env, "BTC"),
            2500000,
            String::from_str(&env, "gt"),
        );

        let market = Market::new(
            &env,
            admin.clone(),
            String::from_str(&env, "Test question"),
            outcomes,
            env.ledger().timestamp() + 86400,
            oracle_config,
            MarketState::Active,
        );

        assert!(market.is_active(env.ledger().timestamp()));
        assert!(!market.is_resolved());
        assert_eq!(market.total_staked, 0);
    }

    #[test]
    fn test_oracle_result() {
        let result = OracleResult::price(2500000);
        assert!(result.is_available());
        assert_eq!(result.get_price(), Some(2500000));

        let unavailable = OracleResult::unavailable();
        assert!(!unavailable.is_available());
        assert_eq!(unavailable.get_price(), None);
    }

    #[test]
    fn test_validation_helpers() {
        assert!(validation::validate_oracle_provider(&OracleProvider::Reflector).is_ok());
        assert!(validation::validate_price(2500000).is_ok());
        assert!(validation::validate_stake(1000000, 500000).is_ok());
        assert!(validation::validate_duration(30).is_ok());
    }

    #[test]
    fn test_conversion_helpers() {
        assert_eq!(
            conversion::string_to_oracle_provider("reflector"),
            Some(OracleProvider::Reflector)
        );
        assert_eq!(
            conversion::oracle_provider_to_string(&OracleProvider::Reflector),
            "Reflector"
        );
    }

    #[test]
    fn test_community_consensus() {
        let consensus = CommunityConsensus {
            outcome: String::from_str(&soroban_sdk::Env::default(), "yes"),
            votes: 75,
            total_votes: 100,
            percentage: 75,
        };
        
        assert_eq!(consensus.votes, 75);
        assert_eq!(consensus.percentage, 75);
    }
}