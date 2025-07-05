use soroban_sdk::{contracttype, vec, Address, Env, Map, String, Symbol, Vec};

/// Comprehensive type system for Predictify Hybrid contract
///
/// This module provides organized type definitions categorized by functionality:
/// - Oracle Types: Oracle providers, configurations, and data structures
/// - Market Types: Market data structures and state management
/// - Price Types: Price data and validation structures
/// - Validation Types: Input validation and business logic types
/// - Utility Types: Helper types and conversion utilities

// ===== ORACLE TYPES =====

/// Supported oracle providers for price feeds
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OracleProvider {
    /// Band Protocol oracle
    BandProtocol,
    /// DIA oracle
    DIA,
    /// Reflector oracle (Stellar-based)
    Reflector,
    /// Pyth Network oracle
    Pyth,
}

impl OracleProvider {
    /// Get a human-readable name for the oracle provider
    pub fn name(&self) -> &'static str {
        match self {
            OracleProvider::BandProtocol => "Band Protocol",
            OracleProvider::DIA => "DIA",
            OracleProvider::Reflector => "Reflector",
            OracleProvider::Pyth => "Pyth Network",
        }
    }

    /// Check if the oracle provider is supported
    pub fn is_supported(&self) -> bool {
        matches!(self, OracleProvider::Pyth | OracleProvider::Reflector)
    }

    /// Get the default feed ID format for this provider
    pub fn default_feed_format(&self) -> &'static str {
        match self {
            OracleProvider::BandProtocol => "BTC/USD",
            OracleProvider::DIA => "BTC/USD",
            OracleProvider::Reflector => "BTC",
            OracleProvider::Pyth => "BTC/USD",
        }
    }
}

/// Configuration for oracle integration
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
    pub fn validate(&self, env: &Env) -> Result<(), crate::errors::Error> {
        // Validate threshold
        if self.threshold <= 0 {
            return Err(crate::errors::Error::InvalidThreshold);
        }

        // Validate comparison operator
        if self.comparison != String::from_str(env, "gt")
            && self.comparison != String::from_str(env, "lt")
            && self.comparison != String::from_str(env, "eq")
        {
            return Err(crate::errors::Error::InvalidComparison);
        }

        // Validate feed_id is not empty
        if self.feed_id.is_empty() {
            return Err(crate::errors::Error::InvalidOracleFeed);
        }

        // Validate provider is supported
        if !self.provider.is_supported() {
            return Err(crate::errors::Error::InvalidOracleConfig);
        }

        Ok(())
    }

    /// Check if the configuration is for a supported provider
    pub fn is_supported(&self) -> bool {
        self.provider.is_supported()
    }

    /// Get the comparison operator as a string
    pub fn comparison_operator(&self) -> &String {
        &self.comparison
    }

    /// Check if the comparison is "greater than"
    pub fn is_greater_than(&self, env: &Env) -> bool {
        self.comparison == String::from_str(env, "gt")
    }

    /// Check if the comparison is "less than"
    pub fn is_less_than(&self, env: &Env) -> bool {
        self.comparison == String::from_str(env, "lt")
    }

    /// Check if the comparison is "equal to"
    pub fn is_equal_to(&self, env: &Env) -> bool {
        self.comparison == String::from_str(env, "eq")
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
    /// Market extension history
    pub extension_history: Vec<MarketExtension>,
    /// Total extension days applied
    pub total_extension_days: u32,
    /// Maximum allowed extension days
    pub max_extension_days: u32,
}

/// Market extension record
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketExtension {
    /// Extension timestamp
    pub timestamp: u64,
    /// Additional days requested
    pub additional_days: u32,
    /// Admin who requested the extension
    pub admin: Address,
    /// Extension reason/justification
    pub reason: String,
    /// Extension fee paid
    pub fee_paid: i128,
}

impl MarketExtension {
    /// Create a new market extension record
    pub fn new(
        env: &Env,
        additional_days: u32,
        admin: Address,
        reason: String,
        fee_paid: i128,
    ) -> Self {
        Self {
            timestamp: env.ledger().timestamp(),
            additional_days,
            admin,
            reason,
            fee_paid,
        }
    }

    /// Validate extension parameters
    pub fn validate(&self, env: &Env) -> Result<(), crate::errors::Error> {
        if self.additional_days == 0 {
            return Err(crate::errors::Error::InvalidExtensionDays);
        }

        if self.additional_days > 30 {
            return Err(crate::errors::Error::ExtensionDaysExceeded);
        }

        if self.reason.is_empty() {
            return Err(crate::errors::Error::InvalidExtensionReason);
        }

        Ok(())
    }
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
            extension_history: vec![env],
            total_extension_days: 0,
            max_extension_days: 30, // Default maximum extension days
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

    /// Check if the market has oracle result
    pub fn has_oracle_result(&self) -> bool {
        self.oracle_result.is_some()
    }

    /// Get user's vote
    pub fn get_user_vote(&self, user: &Address) -> Option<String> {
        self.votes.get(user.clone())
    }

    /// Get user's stake
    pub fn get_user_stake(&self, user: &Address) -> i128 {
        self.stakes.get(user.clone()).unwrap_or(0)
    }

    /// Check if user has claimed
    pub fn has_user_claimed(&self, user: &Address) -> bool {
        self.claimed.get(user.clone()).unwrap_or(false)
    }

    /// Get user's dispute stake
    pub fn get_user_dispute_stake(&self, user: &Address) -> i128 {
        self.dispute_stakes.get(user.clone()).unwrap_or(0)
    }

    /// Add user vote and stake
    pub fn add_vote(&mut self, user: Address, outcome: String, stake: i128) {
        self.votes.set(user.clone(), outcome);
        self.stakes.set(user.clone(), stake);
        self.total_staked += stake;
    }

    /// Add dispute stake
    pub fn add_dispute_stake(&mut self, user: Address, stake: i128) {
        let current_stake = self.dispute_stakes.get(user.clone()).unwrap_or(0);
        self.dispute_stakes.set(user, current_stake + stake);
    }

    /// Mark user as claimed
    pub fn mark_claimed(&mut self, user: Address) {
        self.claimed.set(user, true);
    }

    /// Set oracle result
    pub fn set_oracle_result(&mut self, result: String) {
        self.oracle_result = Some(result);
    }

    /// Set winning outcome
    pub fn set_winning_outcome(&mut self, outcome: String) {
        self.winning_outcome = Some(outcome);
    }

    /// Mark fees as collected
    pub fn mark_fees_collected(&mut self) {
        self.fee_collected = true;
    }

    /// Get total dispute stakes
    pub fn total_dispute_stakes(&self) -> i128 {
        let mut total = 0;
        for (_, stake) in self.dispute_stakes.iter() {
            total += stake;
        }
        total
    }

    /// Get winning stake total
    pub fn winning_stake_total(&self) -> i128 {
        if let Some(winning_outcome) = &self.winning_outcome {
            let mut total = 0;
            for (user, outcome) in self.votes.iter() {
                if &outcome == winning_outcome {
                    total += self.stakes.get(user.clone()).unwrap_or(0);
                }
            }
            total
        } else {
            0
        }
    }

    /// Validate market parameters
    pub fn validate(&self, env: &Env) -> Result<(), crate::errors::Error> {
        // Validate question
        if self.question.is_empty() {
            return Err(crate::errors::Error::InvalidQuestion);
        }

        // Validate outcomes
        if self.outcomes.len() < 2 {
            return Err(crate::errors::Error::InvalidOutcomes);
        }

        // Validate oracle config
        self.oracle_config.validate(env)?;

        // Validate end time
        if self.end_time <= env.ledger().timestamp() {
            return Err(crate::errors::Error::InvalidDuration);
        }

        Ok(())
    }
}

/// Extension statistics
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtensionStats {
    /// Total number of extensions made
    pub total_extensions: u32,
    /// Total extension days applied
    pub total_extension_days: u32,
    /// Maximum allowed extension days
    pub max_extension_days: u32,
    /// Whether market can still be extended
    pub can_extend: bool,
    /// Extension fee per day
    pub extension_fee_per_day: i128,
}

// ===== PRICE TYPES =====

/// Pyth Network price data structure
#[contracttype]
pub struct PythPrice {
    /// Price value
    pub price: i128,
    /// Confidence interval
    pub conf: u64,
    /// Price exponent
    pub expo: i32,
    /// Publish timestamp
    pub publish_time: u64,
}

impl PythPrice {
    /// Create a new Pyth price
    pub fn new(price: i128, conf: u64, expo: i32, publish_time: u64) -> Self {
        Self {
            price,
            conf,
            expo,
            publish_time,
        }
    }

    /// Get the price in cents
    pub fn price_in_cents(&self) -> i128 {
        self.price
    }

    /// Check if the price is stale (older than max_age seconds)
    pub fn is_stale(&self, current_time: u64, max_age: u64) -> bool {
        current_time - self.publish_time > max_age
    }

    /// Validate the price data
    pub fn validate(&self) -> Result<(), crate::errors::Error> {
        if self.price <= 0 {
            return Err(crate::errors::Error::OraclePriceOutOfRange);
        }

        if self.conf == 0 {
            return Err(crate::errors::Error::OracleDataStale);
        }

        Ok(())
    }
}

/// Reflector asset types
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReflectorAsset {
    /// Stellar asset (using contract address)
    Stellar(Address),
    /// Other asset (using symbol)
    Other(Symbol),
}

impl ReflectorAsset {
    /// Create a Stellar asset
    pub fn stellar(contract_id: Address) -> Self {
        ReflectorAsset::Stellar(contract_id)
    }

    /// Create an other asset
    pub fn other(symbol: Symbol) -> Self {
        ReflectorAsset::Other(symbol)
    }

    /// Get the asset identifier as a string
    pub fn to_string(&self, env: &Env) -> String {
        match self {
            ReflectorAsset::Stellar(addr) => String::from_str(env, "stellar_asset"),
            ReflectorAsset::Other(symbol) => String::from_str(env, "other_asset"),
        }
    }

    /// Check if this is a Stellar asset
    pub fn is_stellar(&self) -> bool {
        matches!(self, ReflectorAsset::Stellar(_))
    }

    /// Check if this is an other asset
    pub fn is_other(&self) -> bool {
        matches!(self, ReflectorAsset::Other(_))
    }
}

/// Reflector price data structure
#[contracttype]
pub struct ReflectorPriceData {
    /// Price value
    pub price: i128,
    /// Timestamp
    pub timestamp: u64,
}

impl ReflectorPriceData {
    /// Create new Reflector price data
    pub fn new(price: i128, timestamp: u64) -> Self {
        Self { price, timestamp }
    }

    /// Get the price in cents
    pub fn price_in_cents(&self) -> i128 {
        self.price
    }

    /// Check if the price is stale
    pub fn is_stale(&self, current_time: u64, max_age: u64) -> bool {
        current_time - self.timestamp > max_age
    }

    /// Validate the price data
    pub fn validate(&self) -> Result<(), crate::errors::Error> {
        if self.price <= 0 {
            return Err(crate::errors::Error::OraclePriceOutOfRange);
        }

        Ok(())
    }
}

/// Reflector configuration data
#[contracttype]
pub struct ReflectorConfigData {
    /// Admin address
    pub admin: Address,
    /// Supported assets
    pub assets: Vec<ReflectorAsset>,
    /// Base asset
    pub base_asset: ReflectorAsset,
    /// Decimal places
    pub decimals: u32,
    /// Update period
    pub period: u64,
    /// Resolution
    pub resolution: u32,
}

impl ReflectorConfigData {
    /// Create new Reflector config data
    pub fn new(
        admin: Address,
        assets: Vec<ReflectorAsset>,
        base_asset: ReflectorAsset,
        decimals: u32,
        period: u64,
        resolution: u32,
    ) -> Self {
        Self {
            admin,
            assets,
            base_asset,
            decimals,
            period,
            resolution,
        }
    }

    /// Check if an asset is supported
    pub fn supports_asset(&self, asset: &ReflectorAsset) -> bool {
        self.assets.contains(asset)
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), crate::errors::Error> {
        if self.assets.is_empty() {
            return Err(crate::errors::Error::InvalidOracleConfig);
        }

        if self.decimals == 0 {
            return Err(crate::errors::Error::InvalidOracleConfig);
        }

        if self.period == 0 {
            return Err(crate::errors::Error::InvalidOracleConfig);
        }

        if self.resolution == 0 {
            return Err(crate::errors::Error::InvalidOracleConfig);
        }

        Ok(())
    }
}

// ===== VALIDATION TYPES =====

/// Market creation parameters
#[derive(Clone, Debug)]
pub struct MarketCreationParams {
    pub admin: Address,
    pub question: String,
    pub outcomes: Vec<String>,
    pub duration_days: u32,
    pub oracle_config: OracleConfig,
}

impl MarketCreationParams {
    /// Create new market creation parameters
    pub fn new(
        admin: Address,
        question: String,
        outcomes: Vec<String>,
        duration_days: u32,
        oracle_config: OracleConfig,
    ) -> Self {
        Self {
            admin,
            question,
            outcomes,
            duration_days,
            oracle_config,
        }
    }

    /// Validate all parameters
    pub fn validate(&self, env: &Env) -> Result<(), crate::errors::Error> {
        // Validate question
        if self.question.is_empty() {
            return Err(crate::errors::Error::InvalidQuestion);
        }

        // Validate outcomes
        if self.outcomes.len() < 2 {
            return Err(crate::errors::Error::InvalidOutcomes);
        }

        // Validate duration
        if self.duration_days == 0 || self.duration_days > 365 {
            return Err(crate::errors::Error::InvalidDuration);
        }

        // Validate oracle config
        self.oracle_config.validate(env)?;

        Ok(())
    }

    /// Calculate end time from duration
    pub fn calculate_end_time(&self, env: &Env) -> u64 {
        let seconds_per_day: u64 = 24 * 60 * 60;
        let duration_seconds: u64 = (self.duration_days as u64) * seconds_per_day;
        env.ledger().timestamp() + duration_seconds
    }
}

/// Vote parameters
#[derive(Clone, Debug)]
pub struct VoteParams {
    pub user: Address,
    pub outcome: String,
    pub stake: i128,
}

impl VoteParams {
    /// Create new vote parameters
    pub fn new(user: Address, outcome: String, stake: i128) -> Self {
        Self {
            user,
            outcome,
            stake,
        }
    }

    /// Validate vote parameters
    pub fn validate(&self, _env: &Env, market: &Market) -> Result<(), crate::errors::Error> {
        // Validate outcome
        if !market.outcomes.contains(&self.outcome) {
            return Err(crate::errors::Error::InvalidOutcome);
        }

        // Validate stake
        if self.stake <= 0 {
            return Err(crate::errors::Error::InsufficientStake);
        }

        // Check if user already voted
        if market.get_user_vote(&self.user).is_some() {
            return Err(crate::errors::Error::AlreadyVoted);
        }

        Ok(())
    }
}

// ===== UTILITY TYPES =====

/// Market state enumeration
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MarketState {
    /// Market is active and accepting votes
    Active,
    /// Market has ended but not resolved
    Ended,
    /// Market has been resolved
    Resolved,
    /// Market has been closed
    Closed,
}

impl MarketState {
    /// Get state from market
    pub fn from_market(market: &Market, current_time: u64) -> Self {
        if market.is_resolved() {
            MarketState::Resolved
        } else if market.has_ended(current_time) {
            MarketState::Ended
        } else {
            MarketState::Active
        }
    }

    /// Check if market is active
    pub fn is_active(&self) -> bool {
        matches!(self, MarketState::Active)
    }

    /// Check if market has ended
    pub fn has_ended(&self) -> bool {
        matches!(
            self,
            MarketState::Ended | MarketState::Resolved | MarketState::Closed
        )
    }

    /// Check if market is resolved
    pub fn is_resolved(&self) -> bool {
        matches!(self, MarketState::Resolved)
    }
}

/// Oracle result type
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

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn test_oracle_provider() {
        let provider = OracleProvider::Pyth;
        assert_eq!(provider.name(), "Pyth Network");
        assert!(provider.is_supported());
        assert_eq!(provider.default_feed_format(), "BTC/USD");
    }

    #[test]
    fn test_oracle_config() {
        let env = soroban_sdk::Env::default();
        let config = OracleConfig::new(
            OracleProvider::Pyth,
            String::from_str(&env, "BTC/USD"),
            2500000,
            String::from_str(&env, "gt"),
        );

        assert!(config.validate(&env).is_ok());
        assert!(config.is_supported());
        assert!(config.is_greater_than(&env));
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
            OracleProvider::Pyth,
            String::from_str(&env, "BTC/USD"),
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
        );

        assert!(market.is_active(env.ledger().timestamp()));
        assert!(!market.is_resolved());
        assert_eq!(market.total_staked, 0);
    }

    #[test]
    fn test_reflector_asset() {
        let env = soroban_sdk::Env::default();
        let symbol = Symbol::new(&env, "BTC");
        let asset = ReflectorAsset::other(symbol);

        assert!(asset.is_other());
        assert!(!asset.is_stellar());
    }

    #[test]
    fn test_market_state() {
        let env = soroban_sdk::Env::default();
        let admin = Address::generate(&env);
        let outcomes = vec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ];
        let oracle_config = OracleConfig::new(
            OracleProvider::Pyth,
            String::from_str(&env, "BTC/USD"),
            2500000,
            String::from_str(&env, "gt"),
        );

        let market = Market::new(
            &env,
            admin,
            String::from_str(&env, "Test question"),
            outcomes,
            env.ledger().timestamp() + 86400,
            oracle_config,
        );

        let state = MarketState::from_market(&market, env.ledger().timestamp());
        assert!(state.is_active());
        assert!(!state.has_ended());
        assert!(!state.is_resolved());
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
        assert!(validation::validate_oracle_provider(&OracleProvider::Pyth).is_ok());
        assert!(validation::validate_price(2500000).is_ok());
        assert!(validation::validate_stake(1000000, 500000).is_ok());
        assert!(validation::validate_duration(30).is_ok());
    }

    #[test]
    fn test_conversion_helpers() {
        assert_eq!(
            conversion::string_to_oracle_provider("pyth"),
            Some(OracleProvider::Pyth)
        );
        assert_eq!(
            conversion::oracle_provider_to_string(&OracleProvider::Pyth),
            "Pyth Network"
        );
    }
}
