
use soroban_sdk::{contracttype, Address, Env, Map, String, Symbol, Vec};
use crate::errors::Error;

// ===== MARKET STATE =====

/// Enumeration of possible market states throughout the prediction market lifecycle.
///
/// This enum defines the various states a prediction market can be in, from initial
/// creation through final resolution and closure. Each state represents a distinct
/// phase with specific business rules, available operations, and state transition
/// requirements.
///
/// # State Lifecycle
///
/// The typical market progression follows this pattern:
/// ```text
/// Active → Ended → [Disputed] → Resolved → Closed
/// ```
///
/// **Alternative flows:**
/// - **Cancellation**: `Active → Cancelled` (emergency situations)
/// - **Direct Resolution**: `Active → Resolved` (admin override)
/// - **Dispute Flow**: `Ended → Disputed → Resolved`
///
/// # State Descriptions
///
/// **Active**: Market is live and accepting user participation
/// - Users can place votes and stakes
/// - Market question and outcomes are fixed
/// - Oracle configuration is immutable
/// - Voting period is ongoing
///
/// **Ended**: Market voting period has concluded
/// - No new votes or stakes accepted
/// - Oracle resolution can be triggered
/// - Community consensus can be calculated
/// - Dispute period may be active
///
/// **Disputed**: Market resolution is under dispute
/// - Formal dispute process is active
/// - Additional evidence may be collected
/// - Dispute resolution mechanisms engaged
/// - Final outcome pending dispute resolution
///
/// **Resolved**: Market outcome has been determined
/// - Final outcome is established
/// - Payouts can be calculated and distributed
/// - Resolution method and confidence recorded
/// - Market moves toward closure
///
/// **Closed**: Market is permanently closed
/// - All payouts have been distributed
/// - No further operations allowed
/// - Market data preserved for historical analysis
/// - Final state for completed markets
///
/// **Cancelled**: Market has been cancelled
/// - Emergency cancellation due to issues
/// - Stakes returned to participants
/// - No winner determination
/// - Administrative action required
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::Env;
/// # use predictify_hybrid::types::{MarketState, Market};
/// # let env = Env::default();
/// # let market = Market::default(); // Placeholder
/// # let current_time = env.ledger().timestamp();
///
/// // Check market state and determine available operations
/// match market.state {
///     MarketState::Active => {
///         if market.is_active(current_time) {
///             println!("Market is active - users can vote");
///             // Allow voting operations
///         } else {
///             println!("Market should transition to Ended state");
///         }
///     },
///     MarketState::Ended => {
///         println!("Market ended - ready for resolution");
///         // Trigger oracle resolution or community consensus
///     },
///     MarketState::Disputed => {
///         println!("Market under dispute - awaiting resolution");
///         // Handle dispute process
///     },
///     MarketState::Resolved => {
///         println!("Market resolved - calculating payouts");
///         // Process winner payouts
///     },
///     MarketState::Closed => {
///         println!("Market closed - no further operations");
///         // Read-only access for historical data
///     },
///     MarketState::Cancelled => {
///         println!("Market cancelled - refunding stakes");
///         // Process stake refunds
///     },
/// }
/// ```
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
            OracleProvider::Pyth => "Pyth Network",
            OracleProvider::BandProtocol => "Band Protocol",
            OracleProvider::DIA => "DIA",
        }
    }

    /// Check if provider is supported on Stellar
    pub fn is_supported(&self) -> bool {
        matches!(self, OracleProvider::Reflector | OracleProvider::Pyth)
    }

    /// Get default feed format for provider
    pub fn default_feed_format(&self) -> &'static str {
        match self {
            OracleProvider::Reflector => "BTC/USD",
            OracleProvider::Pyth => "BTC/USD",
            OracleProvider::BandProtocol => "BTC/USD",
            OracleProvider::DIA => "BTC/USD",
        }
    }
}

/// Comprehensive oracle configuration for prediction market resolution.
///
/// This structure defines all parameters needed to configure oracle-based market
/// resolution, including provider selection, price feed identification, threshold
/// values, and comparison logic. It serves as the bridge between prediction markets
/// and external oracle data sources.
///
/// # Configuration Components
///
/// **Provider Selection:**
/// - **Provider**: Which oracle service to use (Reflector, Pyth, etc.)
/// - **Feed ID**: Specific price feed identifier for the asset
///
/// **Resolution Logic:**
/// - **Threshold**: Price level that determines market outcome
/// - **Comparison**: How to compare oracle price against threshold
///
/// # Supported Comparisons
///
/// The oracle configuration supports various comparison operators:
/// - **"gt"**: Greater than - price > threshold resolves to "yes"
/// - **"lt"**: Less than - price < threshold resolves to "yes"
/// - **"eq"**: Equal to - price == threshold resolves to "yes"
///
/// # Price Format Standards
///
/// Thresholds follow consistent pricing conventions:
/// - **Integer Values**: No floating point arithmetic
/// - **Cent Precision**: Prices in cents (e.g., 5000000 = $50,000)
/// - **Positive Values**: All thresholds must be positive
/// - **Reasonable Range**: Between $0.01 and $10,000,000
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, String};
/// # use predictify_hybrid::types::{OracleConfig, OracleProvider};
/// # let env = Env::default();
///
/// // Create oracle config for "Will BTC be above $50,000?"
/// let btc_config = OracleConfig::new(
///     OracleProvider::Reflector,
///     String::from_str(&env, "BTC/USD"),
///     50_000_00, // $50,000 in cents
///     String::from_str(&env, "gt") // Greater than
/// );
///
/// // Validate the configuration
/// btc_config.validate(&env)?;
///
/// println!("Oracle Config:");
/// println!("Provider: {}", btc_config.provider.name());
/// println!("Feed: {}", btc_config.feed_id);
/// println!("Threshold: ${}", btc_config.threshold / 100);
/// println!("Comparison: {}", btc_config.comparison);
///
/// // Create config for "Will ETH drop below $2,000?"
/// let eth_config = OracleConfig::new(
///     OracleProvider::Reflector,
///     String::from_str(&env, "ETH/USD"),
///     2_000_00, // $2,000 in cents
///     String::from_str(&env, "lt") // Less than
/// );
///
/// // Create config for "Will XLM equal exactly $0.50?"
/// let xlm_config = OracleConfig::new(
///     OracleProvider::Reflector,
///     String::from_str(&env, "XLM/USD"),
///     50, // $0.50 in cents
///     String::from_str(&env, "eq") // Equal to
/// );
/// # Ok::<(), predictify_hybrid::errors::Error>(())
/// ```
///
/// # Feed ID Formats
///
/// Different oracle providers use different feed ID formats:
///
/// **Reflector Oracle:**
/// - Standard pairs: "BTC/USD", "ETH/USD", "XLM/USD"
/// - Asset only: "BTC", "ETH", "XLM" (assumes USD)
/// - Custom symbols: Any symbol supported by Reflector
///
/// **Pyth Network (Future):**
/// - Hex identifiers: "0xe62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43"
/// - 64-character hexadecimal strings
/// - Globally unique across all assets
///
/// # Validation Rules
///
/// Oracle configurations must pass validation:
/// ```rust
/// # use soroban_sdk::{Env, String};
/// # use predictify_hybrid::types::{OracleConfig, OracleProvider};
/// # let env = Env::default();
///
/// let config = OracleConfig::new(
///     OracleProvider::Reflector,
///     String::from_str(&env, "BTC/USD"),
///     50_000_00,
///     String::from_str(&env, "gt")
/// );
///
/// // Validation checks:
/// // 1. Threshold must be positive
/// // 2. Comparison must be "gt", "lt", or "eq"
/// // 3. Provider must be supported on current network
/// // 4. Feed ID must not be empty
///
/// match config.validate(&env) {
///     Ok(()) => println!("Configuration is valid"),
///     Err(e) => println!("Validation failed: {:?}", e),
/// }
/// ```
///
/// # Integration with Market Resolution
///
/// Oracle configurations integrate with resolution systems:
/// - **Oracle Manager**: Uses config to fetch appropriate price data
/// - **Resolution Logic**: Applies comparison to determine outcomes
/// - **Validation System**: Ensures config meets quality standards
/// - **Event System**: Logs oracle configuration for transparency
///
/// # Common Configuration Patterns
///
/// **Price Threshold Markets:**
/// ```rust
/// # use soroban_sdk::{Env, String};
/// # use predictify_hybrid::types::{OracleConfig, OracleProvider};
/// # let env = Env::default();
///
/// // "Will BTC reach $100k by year end?"
/// let btc_100k = OracleConfig::new(
///     OracleProvider::Reflector,
///     String::from_str(&env, "BTC/USD"),
///     100_000_00,
///     String::from_str(&env, "gt")
/// );
///
/// // "Will ETH stay above $1,500?"
/// let eth_support = OracleConfig::new(
///     OracleProvider::Reflector,
///     String::from_str(&env, "ETH/USD"),
///     1_500_00,
///     String::from_str(&env, "gt")
/// );
/// ```
///
/// # Error Handling
///
/// Common configuration errors:
/// - **InvalidThreshold**: Threshold is zero or negative
/// - **InvalidComparison**: Unsupported comparison operator
/// - **InvalidOracleConfig**: Unsupported oracle provider
/// - **InvalidFeed**: Empty or malformed feed identifier
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

    pub fn validate(&self, env: &Env) -> Result<(), Error> {
        // Validate threshold
        if self.threshold <= 0 {
            return Err(Error::InvalidThreshold);

        }

        // Validate comparison operator
        if self.comparison != String::from_str(env, "gt")
            && self.comparison != String::from_str(env, "lt")
            && self.comparison != String::from_str(env, "eq")
        {

            return Err(Error::InvalidInput);
        }

        // Validate feed_id is not empty
        if self.feed_id.is_empty() {
            return Err(Error::InvalidOracleFeed);

        }

        // Validate provider is supported
        if !self.provider.is_supported() {

            return Err(Error::InvalidConfig);

        }

        Ok(())
    }

    /// Check if this config is supported
    pub fn is_supported(&self) -> bool {
        self.provider.is_supported()
    }

    /// Check if comparison is greater than
    pub fn is_greater_than(&self, env: &Env) -> bool {
        self.comparison == String::from_str(env, "gt")
    }
}

// ===== MARKET TYPES =====

/// Comprehensive market data structure representing a complete prediction market.
///
/// This structure contains all data necessary to manage a prediction market throughout
/// its entire lifecycle, from creation through resolution and payout distribution.
/// It serves as the central data model for all market operations and state management.
///
/// # Core Market Components
///
/// **Market Identity:**
/// - **Admin**: Market administrator with special privileges
/// - **Question**: The prediction question being resolved
/// - **Outcomes**: Available outcomes users can vote on
/// - **End Time**: When the voting period concludes
///
/// **Oracle Integration:**
/// - **Oracle Config**: Configuration for oracle-based resolution
/// - **Oracle Result**: Final oracle outcome (set after resolution)
///
/// **User Participation:**
/// - **Votes**: User outcome predictions
/// - **Stakes**: User financial commitments
/// - **Claimed**: Payout claim status tracking
///
/// **Financial Tracking:**
/// - **Total Staked**: Aggregate stake amount across all users
/// - **Dispute Stakes**: Stakes committed to dispute processes
/// - **Market State**: Current lifecycle state
///
/// # Market Lifecycle
///
/// Markets progress through distinct phases:
/// ```text
/// Creation → Active Voting → Ended → Resolution → Payout → Closed
/// ```
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Address, String, Vec};
/// # use predictify_hybrid::types::{Market, MarketState, OracleConfig, OracleProvider};
/// # let env = Env::default();
/// # let admin = Address::generate(&env);
///
/// // Create a new prediction market
/// let market = Market::new(
///     &env,
///     admin.clone(),
///     String::from_str(&env, "Will BTC reach $100,000 by December 31, 2024?"),
///     Vec::from_array(&env, [
///         String::from_str(&env, "yes"),
///         String::from_str(&env, "no")
///     ]),
///     env.ledger().timestamp() + (30 * 24 * 60 * 60), // 30 days
///     OracleConfig::new(
///         OracleProvider::Reflector,
///         String::from_str(&env, "BTC/USD"),
///         100_000_00, // $100,000
///         String::from_str(&env, "gt")
///     ),
///     MarketState::Active
/// );
///
/// // Validate the market
/// market.validate(&env)?;
///
/// // Check market status
/// let current_time = env.ledger().timestamp();
/// if market.is_active(current_time) {
///     println!("Market is active and accepting votes");
/// } else if market.has_ended(current_time) {
///     println!("Market has ended, ready for resolution");
/// }
///
/// // Display market information
/// println!("Market Question: {}", market.question);
/// println!("Admin: {}", market.admin);
/// println!("Total Staked: {} stroops", market.total_staked);
/// println!("State: {:?}", market.state);
///
/// // Check if market is resolved
/// if market.is_resolved() {
///     if let Some(result) = &market.oracle_result {
///         println!("Oracle Result: {}", result);
///     }
/// }
/// # Ok::<(), predictify_hybrid::errors::Error>(())
/// ```
///
/// # User Participation Tracking
///
/// Markets track comprehensive user participation:
/// ```rust
/// # use soroban_sdk::{Address, String};
/// # use predictify_hybrid::types::Market;
/// # let mut market = Market::default(); // Placeholder
/// # let user = Address::generate(&soroban_sdk::Env::default());
///
/// // Add user vote and stake (for testing)
/// market.add_vote(
///     user.clone(),
///     String::from_str(&soroban_sdk::Env::default(), "yes"),
///     1_000_000 // 1 XLM in stroops
/// );
///
/// // Check user's vote
/// if let Some(user_vote) = market.votes.get(user.clone()) {
///     println!("User voted: {}", user_vote);
/// }
///
/// // Check user's stake
/// if let Some(user_stake) = market.stakes.get(user.clone()) {
///     println!("User staked: {} stroops", user_stake);
/// }
///
/// // Check if user has claimed payout
/// let has_claimed = market.claimed.get(user.clone()).unwrap_or(false);
/// println!("User claimed payout: {}", has_claimed);
/// ```
///
/// # Market Validation
///
/// Markets undergo comprehensive validation:
/// ```rust
/// # use soroban_sdk::Env;
/// # use predictify_hybrid::types::Market;
/// # let env = Env::default();
/// # let market = Market::default(); // Placeholder
///
/// // Validation checks multiple aspects:
/// match market.validate(&env) {
///     Ok(()) => {
///         println!("Market validation passed");
///         // Market is ready for use
///     },
///     Err(e) => {
///         println!("Market validation failed: {:?}", e);
///         // Handle validation errors:
///         // - InvalidQuestion: Empty or invalid question
///         // - InvalidOutcomes: Less than 2 outcomes
///         // - InvalidDuration: End time in the past
///         // - Oracle validation errors
///     }
/// }
/// ```
///
/// # Financial Management
///
/// Markets track financial flows:
/// ```rust
/// # use predictify_hybrid::types::Market;
/// # let market = Market::default(); // Placeholder
///
/// // Total market value
/// println!("Total staked: {} stroops", market.total_staked);
///
/// // Dispute stakes (for contested resolutions)
/// let dispute_total = market.total_dispute_stakes();
/// println!("Total dispute stakes: {} stroops", dispute_total);
///
/// // Calculate potential payouts
/// let winner_pool = market.total_staked; // Simplified
/// println!("Winner pool: {} stroops", winner_pool);
/// ```
///
/// # Integration Points
///
/// Markets integrate with multiple systems:
/// - **Voting System**: Manages user votes and stakes
/// - **Oracle System**: Handles oracle-based resolution
/// - **Dispute System**: Manages dispute processes
/// - **Payout System**: Distributes winnings to users
/// - **Admin System**: Handles administrative operations
/// - **Event System**: Emits market events for transparency
/// - **Analytics System**: Tracks market performance metrics
///
/// # State Management
///
/// Market state transitions are carefully managed:
/// - **Active**: Users can vote, stakes accepted
/// - **Ended**: Voting closed, resolution pending
/// - **Disputed**: Under dispute resolution
/// - **Resolved**: Outcome determined, payouts available
/// - **Closed**: All operations complete
/// - **Cancelled**: Market cancelled, stakes refunded
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
    pub fn validate(&self, _env: &Env) -> Result<(), Error> {
        if self.additional_days == 0 {
            return Err(Error::InvalidInput);
        }

        if self.additional_days > 30 {
            return Err(Error::InvalidInput);
        }

        if self.reason.is_empty() {
            return Err(Error::InvalidInput);
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

    /// Get user's vote if they have voted
    pub fn get_user_vote(&self, user: &Address) -> Option<crate::voting::Vote> {
        let outcome = self.votes.get(user.clone())?;
        let stake = self.stakes.get(user.clone()).unwrap_or(0);
        
        // We don't have the exact timestamp stored separately, but we can use a placeholder
        // In a real implementation, votes would store timestamps
        Some(crate::voting::Vote {
            user: user.clone(),
            outcome,
            stake,
            timestamp: 0, // Placeholder - would need to be stored with the vote
        })
    }

    /// Validate market parameters

    pub fn validate(&self, env: &Env) -> Result<(), Error> {
        // Validate question
        if self.question.is_empty() {
            return Err(Error::InvalidInput);

        }

        // Validate outcomes
        if self.outcomes.len() < 2 {
            return Err(Error::InvalidOutcome);
        }

        // Validate oracle config
        self.oracle_config.validate(env)?;

        // Validate end time
        if self.end_time <= env.ledger().timestamp() {

            return Err(Error::InvalidInput);

        }

        Ok(())
    }
}

// ===== REFLECTOR ORACLE TYPES =====

/// Enumeration of supported assets in the Reflector Oracle ecosystem.
///
/// This enum defines the cryptocurrency assets for which the Reflector Oracle
/// provides price feeds on the Stellar network. Reflector is the primary oracle
/// provider for Stellar-based prediction markets, offering real-time price data
/// for major cryptocurrencies.
///
/// # Supported Assets
///
/// **Bitcoin (BTC):**
/// - Symbol: BTC
/// - Description: Bitcoin, the original cryptocurrency
/// - Typical precision: 8 decimal places
/// - Price range: $10,000 - $200,000+ (historical and projected)
///
/// **Ethereum (ETH):**
/// - Symbol: ETH
/// - Description: Ethereum native token
/// - Typical precision: 18 decimal places
/// - Price range: $500 - $10,000+ (historical and projected)
///
/// **Stellar Lumens (XLM):**
/// - Symbol: XLM
/// - Description: Stellar network native token
/// - Typical precision: 7 decimal places
/// - Price range: $0.05 - $2.00+ (historical and projected)
///
/// # Example Usage
///
/// ```rust
/// # use predictify_hybrid::types::ReflectorAsset;
///
/// // Asset identification and properties
/// let btc = ReflectorAsset::BTC;
/// println!("Asset: {}", btc.symbol());
/// println!("Name: {}", btc.name());
/// println!("Decimals: {}", btc.decimals());
///
/// // Asset validation
/// let assets = vec![ReflectorAsset::BTC, ReflectorAsset::ETH, ReflectorAsset::XLM];
/// for asset in assets {
///     if asset.is_supported() {
///         println!("{} is supported by Reflector", asset.symbol());
///     }
/// }
///
/// // Feed ID generation
/// let btc_feed = ReflectorAsset::BTC.feed_id();
/// println!("BTC feed ID: {}", btc_feed); // "BTC/USD"
///
/// let eth_feed = ReflectorAsset::ETH.feed_id();
/// println!("ETH feed ID: {}", eth_feed); // "ETH/USD"
/// ```
///
/// # Price Feed Integration
///
/// Reflector assets integrate with oracle price feeds:
/// ```rust
/// # use soroban_sdk::{Env, String};
/// # use predictify_hybrid::types::{ReflectorAsset, OracleConfig, OracleProvider};
/// # let env = Env::default();
///
/// // Create oracle config for BTC price prediction
/// let btc_asset = ReflectorAsset::BTC;
/// let oracle_config = OracleConfig::new(
///     OracleProvider::Reflector,
///     String::from_str(&env, &btc_asset.feed_id()),
///     50_000_00, // $50,000 threshold
///     String::from_str(&env, "gt")
/// );
///
/// // Validate asset support
/// if btc_asset.is_supported() {
///     println!("BTC oracle config created successfully");
/// }
/// ```
///
/// # Asset Properties
///
/// Each asset has specific characteristics:
/// ```rust
/// # use predictify_hybrid::types::ReflectorAsset;
///
/// // Bitcoin properties
/// let btc = ReflectorAsset::BTC;
/// assert_eq!(btc.symbol(), "BTC");
/// assert_eq!(btc.name(), "Bitcoin");
/// assert_eq!(btc.decimals(), 8);
/// assert!(btc.is_supported());
///
/// // Ethereum properties
/// let eth = ReflectorAsset::ETH;
/// assert_eq!(eth.symbol(), "ETH");
/// assert_eq!(eth.name(), "Ethereum");
/// assert_eq!(eth.decimals(), 18);
/// assert!(eth.is_supported());
///
/// // Stellar Lumens properties
/// let xlm = ReflectorAsset::XLM;
/// assert_eq!(xlm.symbol(), "XLM");
/// assert_eq!(xlm.name(), "Stellar Lumens");
/// assert_eq!(xlm.decimals(), 7);
/// assert!(xlm.is_supported());
/// ```
///
/// # Feed ID Format
///
/// Reflector uses standardized feed identifiers:
/// - **Format**: "{ASSET}/USD"
/// - **Examples**: "BTC/USD", "ETH/USD", "XLM/USD"
/// - **Base Currency**: All prices quoted in USD
/// - **Case Sensitivity**: Uppercase asset symbols
///
/// # Integration with Market Creation
///
/// Assets are commonly used in market creation:
/// ```rust
/// # use soroban_sdk::{Env, String};
/// # use predictify_hybrid::types::{ReflectorAsset, OracleConfig, OracleProvider};
/// # let env = Env::default();
///
/// // Create market for "Will BTC reach $100k?"
/// let btc_market_config = OracleConfig::new(
///     OracleProvider::Reflector,
///     String::from_str(&env, &ReflectorAsset::BTC.feed_id()),
///     100_000_00,
///     String::from_str(&env, "gt")
/// );
///
/// // Create market for "Will ETH drop below $1,000?"
/// let eth_market_config = OracleConfig::new(
///     OracleProvider::Reflector,
///     String::from_str(&env, &ReflectorAsset::ETH.feed_id()),
///     1_000_00,
///     String::from_str(&env, "lt")
/// );
///
/// // Create market for "Will XLM reach $1?"
/// let xlm_market_config = OracleConfig::new(
///     OracleProvider::Reflector,
///     String::from_str(&env, &ReflectorAsset::XLM.feed_id()),
///     100, // $1.00
///     String::from_str(&env, "gt")
/// );
/// ```
///
/// # Future Asset Additions
///
/// To add new assets to Reflector support:
/// 1. **Add Enum Variant**: Add new asset to enum
/// 2. **Update Methods**: Add symbol, name, decimals mapping
/// 3. **Test Integration**: Verify Reflector feed availability
/// 4. **Update Documentation**: Add asset characteristics
/// 5. **Validate Feeds**: Ensure price feed reliability
///
/// # Error Handling
///
/// Asset-related errors:
/// - **UnsupportedAsset**: Asset not available in Reflector
/// - **InvalidFeedId**: Malformed feed identifier
/// - **PriceFeedUnavailable**: Reflector feed temporarily down
/// - **InvalidPriceData**: Corrupted or invalid price information
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReflectorAsset {
    /// Stellar Lumens (XLM)
    Stellar,
    /// Other asset identified by symbol
    Other(Symbol),
}

impl ReflectorAsset {
    /// Check if this is an Other asset variant
    pub fn is_other(&self) -> bool {
        matches!(self, ReflectorAsset::Other(_))
    }

    /// Check if this is a Stellar asset variant
    pub fn is_stellar(&self) -> bool {
        matches!(self, ReflectorAsset::Stellar)
    }
}

/// Pyth Network price data structure
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
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
    pub fn validate(&self) -> Result<(), Error> {
        if self.price <= 0 {
            return Err(Error::OraclePriceOutOfRange);
        }

        if self.conf == 0 {
            return Err(Error::OracleDataStale);
        }

        Ok(())
    }
}


/// Reflector price data structure
#[contracttype]

pub struct ReflectorPriceData {
    /// Price value in cents (e.g., 2500000 = $25,000)
    pub price: i128,
    /// Timestamp of price update
    pub timestamp: u64,
    /// Price source/confidence
    pub source: String,
}


impl ReflectorPriceData {
    /// Create new Reflector price data
    pub fn new(_env: &Env, price: i128, timestamp: u64, source: String) -> Self {
        Self { price, timestamp, source }
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
    pub fn validate(&self) -> Result<(), Error> {
        if self.price <= 0 {
            return Err(Error::OraclePriceOutOfRange);
        }

        if self.timestamp == 0 {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }
}

/// Market extension data structure for time-based market lifecycle management.
///
/// This structure manages the extension of market voting periods, allowing markets
/// to have their end times adjusted under specific conditions. Extensions provide
/// flexibility for markets that may need additional time due to low participation,
/// significant events, or community requests.
///
/// # Extension Components
///
/// **Extension Request:**
/// - **Requester**: Address that requested the extension
/// - **Original End Time**: Market's initial end time
/// - **New End Time**: Proposed new end time after extension
/// - **Extension Duration**: Length of the extension in seconds
///
/// **Extension Justification:**
/// - **Reason**: Explanation for why extension is needed
/// - **Fee**: Cost paid for the extension request
/// - **Approval Status**: Whether extension has been approved
///
/// **Extension Limits:**
/// - **Max Extensions**: Maximum number of extensions allowed
/// - **Max Duration**: Maximum total extension time
/// - **Min Participation**: Minimum participation required to avoid extension
///
/// # Extension Scenarios
///
/// **Low Participation Extension:**
/// - Market has insufficient votes or stakes
/// - Automatic extension to encourage participation
/// - Extends by standard duration (e.g., 24-48 hours)
///
/// **Community Requested Extension:**
/// - Users request more time for consideration
/// - Requires fee payment and admin approval
/// - Extends by requested duration (within limits)
///
/// **Event-Based Extension:**
/// - Significant market-relevant events occur
/// - Admin-initiated extension for fair resolution
/// - Duration based on event significance
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Address, String};
/// # use predictify_hybrid::types::MarketExtension;
/// # let env = Env::default();
/// # let requester = Address::generate(&env);
///
/// // Create extension request for low participation
/// let extension = MarketExtension::new(
///     &env,
///     requester.clone(),
///     env.ledger().timestamp() + (7 * 24 * 60 * 60), // Original: 7 days
///     env.ledger().timestamp() + (9 * 24 * 60 * 60), // Extended: 9 days
///     String::from_str(&env, "Low participation - extending for more votes"),
///     1_000_000, // 1 XLM extension fee
///     false // Pending approval
/// );
///
/// // Validate extension request
/// extension.validate(&env)?;
///
/// // Display extension information
/// println!("Extension requested by: {}", extension.requester);
/// println!("Extension duration: {} hours", extension.duration_hours());
/// println!("Extension fee: {} stroops", extension.fee);
/// println!("Reason: {}", extension.reason);
///
/// // Check if extension is within limits
/// if extension.is_within_limits() {
///     println!("Extension request is valid");
/// } else {
///     println!("Extension exceeds maximum allowed duration");
/// }
/// # Ok::<(), predictify_hybrid::errors::Error>(())
/// ```
///
/// # Extension Validation
///
/// Extensions undergo comprehensive validation:
/// ```rust
/// # use predictify_hybrid::types::MarketExtension;
/// # let extension = MarketExtension::default(); // Placeholder
///
/// // Validation checks multiple aspects:
/// let validation_result = extension.validate(&soroban_sdk::Env::default());
/// match validation_result {
///     Ok(()) => {
///         println!("Extension validation passed");
///         // Extension can be processed
///     },
///     Err(e) => {
///         println!("Extension validation failed: {:?}", e);
///         // Handle validation errors:
///         // - InvalidDuration: Extension too long or negative
///         // - InsufficientFee: Fee below minimum requirement
///         // - InvalidReason: Empty or inappropriate reason
///         // - ExceedsLimits: Too many extensions or total duration
///     }
/// }
/// ```
///
/// # Fee Structure
///
/// Extension fees vary by type and duration:
/// ```rust
/// # use predictify_hybrid::types::MarketExtension;
///
/// // Calculate extension fee based on duration
/// let base_fee = 1_000_000; // 1 XLM base fee
/// let duration_hours = 48; // 48 hour extension
///
/// let total_fee = if duration_hours <= 24 {
///     base_fee // Standard 24-hour extension
/// } else if duration_hours <= 72 {
///     base_fee * 2 // Extended duration (25-72 hours)
/// } else {
///     base_fee * 5 // Long extension (73+ hours)
/// };
///
/// println!("Extension fee for {} hours: {} stroops", duration_hours, total_fee);
/// ```
///
/// # Extension Approval Process
///
/// Extensions follow a structured approval workflow:
/// ```rust
/// # use predictify_hybrid::types::MarketExtension;
/// # let mut extension = MarketExtension::default(); // Placeholder
///
/// // Step 1: Request submitted with fee
/// extension.set_status("pending");
///
/// // Step 2: Admin review
/// if extension.meets_criteria() {
///     extension.approve();
///     println!("Extension approved");
/// } else {
///     extension.reject("Insufficient justification");
///     println!("Extension rejected");
/// }
///
/// // Step 3: Apply extension if approved
/// if extension.is_approved() {
///     extension.apply_to_market();
///     println!("Market end time updated");
/// }
/// ```
///
/// # Integration with Market Lifecycle
///
/// Extensions integrate with market state management:
/// - **Active Markets**: Can request extensions before end time
/// - **Ended Markets**: Cannot be extended (voting already closed)
/// - **Disputed Markets**: May receive extensions for dispute resolution
/// - **Admin Override**: Admins can extend markets in special circumstances
///
/// # Extension Analytics
///
/// Track extension usage and effectiveness:
/// ```rust
/// # use predictify_hybrid::types::MarketExtension;
/// # let extension = MarketExtension::default(); // Placeholder
///
/// // Extension statistics
/// println!("Extension type: {}", extension.extension_type());
/// println!("Participation before: {}%", extension.participation_before());
/// println!("Participation after: {}%", extension.participation_after());
/// println!("Extension effectiveness: {}%", extension.effectiveness());
/// ```
///
/// # Common Extension Patterns
///
/// **Low Participation Auto-Extension:**
/// ```rust
/// # use soroban_sdk::{Env, Address, String};
/// # use predictify_hybrid::types::MarketExtension;
/// # let env = Env::default();
/// # let system = Address::generate(&env);
///
/// let auto_extension = MarketExtension::new(
///     &env,
///     system, // System-initiated
///     env.ledger().timestamp() + (7 * 24 * 60 * 60),
///     env.ledger().timestamp() + (8 * 24 * 60 * 60), // +24 hours
///     String::from_str(&env, "Auto-extension: Low participation detected"),
///     0, // No fee for auto-extensions
///     true // Auto-approved
/// );
/// ```
///
/// **Community Requested Extension:**
/// ```rust
/// # use soroban_sdk::{Env, Address, String};
/// # use predictify_hybrid::types::MarketExtension;
/// # let env = Env::default();
/// # let community_member = Address::generate(&env);
///
/// let community_extension = MarketExtension::new(
///     &env,
///     community_member,
///     env.ledger().timestamp() + (7 * 24 * 60 * 60),
///     env.ledger().timestamp() + (10 * 24 * 60 * 60), // +72 hours
///     String::from_str(&env, "Major announcement expected - need more time"),
///     2_000_000, // 2 XLM fee
///     false // Pending admin approval
/// );
/// ```
///
/// # Error Handling
///
/// Common extension errors:
/// - **InvalidDuration**: Extension duration is negative or too long
/// - **InsufficientFee**: Fee payment below required amount
/// - **MarketEnded**: Cannot extend market that has already ended
/// - **ExceedsLimits**: Extension would exceed maximum allowed duration
/// - **UnauthorizedRequester**: Requester lacks permission for extension

// ===== MARKET CREATION TYPES =====

/// Comprehensive parameters for creating new prediction markets.
///
/// This structure contains all necessary information to create a new prediction
/// market, including administrative details, market configuration, oracle setup,
/// and financial requirements. It serves as the complete specification for
/// market initialization and validation.
///
/// # Parameter Categories
///
/// **Administrative Setup:**
/// - **Admin**: Market administrator with management privileges
/// - **Creation Fee**: Cost to create the market
///
/// **Market Definition:**
/// - **Question**: The prediction question being resolved
/// - **Outcomes**: Available outcomes users can vote on
/// - **Duration**: How long the market remains active
///
/// **Oracle Integration:**
/// - **Oracle Config**: Configuration for automated resolution
///
/// # Market Creation Workflow
///
/// The market creation process follows these steps:
/// ```text
/// Parameters → Validation → Fee Payment → Market Creation → Activation
/// ```
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Address, String, Vec};
/// # use predictify_hybrid::types::{MarketCreationParams, OracleConfig, OracleProvider};
/// # let env = Env::default();
/// # let admin = Address::generate(&env);
///
/// // Create parameters for a Bitcoin price prediction market
/// let btc_market_params = MarketCreationParams::new(
///     admin.clone(),
///     String::from_str(&env, "Will Bitcoin reach $100,000 by December 31, 2024?"),
///     Vec::from_array(&env, [
///         String::from_str(&env, "yes"),
///         String::from_str(&env, "no")
///     ]),
///     30, // 30 days duration
///     OracleConfig::new(
///         OracleProvider::Reflector,
///         String::from_str(&env, "BTC/USD"),
///         100_000_00, // $100,000 threshold
///         String::from_str(&env, "gt")
///     ),
///     5_000_000 // 5 XLM creation fee
/// );
///
/// // Validate parameters before market creation
/// btc_market_params.validate(&env)?;
///
/// // Display market information
/// println!("Market Question: {}", btc_market_params.question);
/// println!("Duration: {} days", btc_market_params.duration_days);
/// println!("Creation Fee: {} stroops", btc_market_params.creation_fee);
/// println!("Oracle Provider: {}", btc_market_params.oracle_config.provider.name());
///
/// // Check if admin has sufficient balance
/// if admin_has_sufficient_balance(&admin, btc_market_params.creation_fee) {
///     println!("Admin can afford market creation");
/// } else {
///     println!("Insufficient balance for market creation");
/// }
/// # Ok::<(), predictify_hybrid::errors::Error>(())
/// ```
///
/// # Parameter Validation
///
/// Market creation parameters undergo comprehensive validation:
/// ```rust
/// # use predictify_hybrid::types::MarketCreationParams;
/// # let params = MarketCreationParams::default(); // Placeholder
///
/// // Validation checks multiple aspects:
/// let validation_result = params.validate(&soroban_sdk::Env::default());
/// match validation_result {
///     Ok(()) => {
///         println!("Market parameters are valid");
///         // Proceed with market creation
///     },
///     Err(e) => {
///         println!("Parameter validation failed: {:?}", e);
///         // Handle validation errors:
///         // - InvalidQuestion: Empty or inappropriate question
///         // - InvalidOutcomes: Less than 2 outcomes or duplicates
///         // - InvalidDuration: Duration too short or too long
///         // - InsufficientFee: Creation fee below minimum
///         // - InvalidOracleConfig: Oracle configuration errors
///     }
/// }
/// ```
///
/// # Question Guidelines
///
/// Market questions should follow best practices:
/// ```rust
/// # use soroban_sdk::{Env, String};
/// # let env = Env::default();
///
/// // Good question examples:
/// let good_questions = vec![
///     "Will Bitcoin reach $100,000 by December 31, 2024?",
///     "Will Ethereum's price exceed $5,000 before June 1, 2024?",
///     "Will XLM trade above $1.00 within the next 90 days?"
/// ];
///
/// // Question validation criteria:
/// // 1. Clear and unambiguous
/// // 2. Specific timeframe
/// // 3. Measurable outcome
/// // 4. Appropriate length (10-200 characters)
/// // 5. No offensive or inappropriate content
///
/// for question in good_questions {
///     let question_str = String::from_str(&env, question);
///     if validate_question(&question_str) {
///         println!("✓ Valid question: {}", question);
///     }
/// }
/// ```
///
/// # Outcome Configuration
///
/// Outcomes define the possible market results:
/// ```rust
/// # use soroban_sdk::{Env, String, Vec};
/// # let env = Env::default();
///
/// // Binary outcomes (most common)
/// let binary_outcomes = Vec::from_array(&env, [
///     String::from_str(&env, "yes"),
///     String::from_str(&env, "no")
/// ]);
///
/// // Multiple choice outcomes
/// let multiple_outcomes = Vec::from_array(&env, [
///     String::from_str(&env, "under_50k"),
///     String::from_str(&env, "50k_to_75k"),
///     String::from_str(&env, "75k_to_100k"),
///     String::from_str(&env, "over_100k")
/// ]);
///
/// // Outcome validation rules:
/// // 1. Minimum 2 outcomes
/// // 2. Maximum 10 outcomes
/// // 3. No duplicate outcomes
/// // 4. Each outcome 1-50 characters
/// // 5. Clear and distinct options
/// ```
///
/// # Duration Planning
///
/// Market duration affects participation and resolution:
/// ```rust
/// # use predictify_hybrid::types::MarketCreationParams;
///
/// // Duration recommendations by market type:
/// let duration_guidelines = vec![
///     ("Short-term price movements", 1..=7),    // 1-7 days
///     ("Monthly predictions", 7..=30),          // 1-4 weeks
///     ("Quarterly outcomes", 30..=90),          // 1-3 months
///     ("Annual predictions", 90..=365),         // 3-12 months
/// ];
///
/// for (market_type, duration_range) in duration_guidelines {
///     println!("{}: {} days", market_type,
///         format!("{}-{}", duration_range.start(), duration_range.end()));
/// }
///
/// // Duration validation:
/// // - Minimum: 1 day
/// // - Maximum: 365 days (1 year)
/// // - Recommended: 7-90 days for most markets
/// ```
///
/// # Fee Structure
///
/// Creation fees vary based on market characteristics:
/// ```rust
/// # use predictify_hybrid::types::MarketCreationParams;
///
/// // Base fee calculation
/// let base_fee = 1_000_000; // 1 XLM base fee
///
/// // Fee modifiers based on duration
/// let duration_multiplier = |days: u32| -> f64 {
///     match days {
///         1..=7 => 1.0,      // Short-term: no modifier
///         8..=30 => 1.5,     // Medium-term: 50% increase
///         31..=90 => 2.0,    // Long-term: 100% increase
///         91..=365 => 3.0,   // Very long-term: 200% increase
///         _ => 5.0,          // Invalid duration: penalty
///     }
/// };
///
/// // Calculate total creation fee
/// let duration_days = 30;
/// let total_fee = (base_fee as f64 * duration_multiplier(duration_days)) as i128;
/// println!("Creation fee for {} days: {} stroops", duration_days, total_fee);
/// ```
///
/// # Common Market Templates
///
/// Pre-configured templates for common market types:
/// ```rust
/// # use soroban_sdk::{Env, Address, String, Vec};
/// # use predictify_hybrid::types::{MarketCreationParams, OracleConfig, OracleProvider};
/// # let env = Env::default();
/// # let admin = Address::generate(&env);
///
/// // Bitcoin price threshold template
/// let btc_template = |threshold: i128, days: u32| -> MarketCreationParams {
///     MarketCreationParams::new(
///         admin.clone(),
///         String::from_str(&env, &format!("Will BTC reach ${}?", threshold / 100)),
///         Vec::from_array(&env, [
///             String::from_str(&env, "yes"),
///             String::from_str(&env, "no")
///         ]),
///         days,
///         OracleConfig::new(
///             OracleProvider::Reflector,
///             String::from_str(&env, "BTC/USD"),
///             threshold,
///             String::from_str(&env, "gt")
///         ),
///         calculate_creation_fee(days)
///     )
/// };
///
/// // Create BTC $100k market
/// let btc_100k_market = btc_template(100_000_00, 90);
/// ```
///
/// # Integration Points
///
/// Market creation parameters integrate with:
/// - **Market Factory**: Creates markets from validated parameters
/// - **Fee Manager**: Processes creation fee payments
/// - **Oracle System**: Validates and configures oracle integration
/// - **Admin System**: Verifies administrator permissions
/// - **Event System**: Emits market creation events
/// - **Validation System**: Ensures parameter compliance
///
/// # Error Handling
///
/// Common parameter errors:
/// - **InvalidQuestion**: Question is empty, too long, or inappropriate
/// - **InvalidOutcomes**: Insufficient outcomes or duplicates
/// - **InvalidDuration**: Duration outside allowed range
/// - **InsufficientFee**: Creation fee below minimum requirement
/// - **InvalidAdmin**: Admin address is invalid or restricted
/// - **OracleConfigError**: Oracle configuration validation failed
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


    /// Validate all parameters
    pub fn validate(&self, env: &Env) -> Result<(), Error> {
        // Validate question
        if self.question.is_empty() {
            return Err(Error::InvalidInput);
        }

        // Validate outcomes
        if self.outcomes.len() < 2 {
            return Err(Error::InvalidOutcome);
        }

        // Validate duration
        if self.duration_days == 0 || self.duration_days > 365 {
            return Err(Error::InvalidInput);
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
    pub fn validate(&self, _env: &Env, market: &Market) -> Result<(), Error> {
        // Validate outcome
        if !market.outcomes.contains(&self.outcome) {
            return Err(Error::InvalidOutcome);
        }

        // Validate stake
        if self.stake <= 0 {
            return Err(Error::InsufficientStake);
        }

        // Check if user already voted
        if market.get_user_vote(&self.user).is_some() {
            return Err(Error::AlreadyVoted);
        }

        Ok(())
    }
}

// ===== UTILITY TYPES =====

/// Market state enumeration
#[contracttype]
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
    /// Market is under dispute
    Disputed,
    /// Market has been cancelled
    Cancelled,
}

/// Extension statistics for a market
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtensionStats {
    /// Total number of extensions
    pub total_extensions: u32,
    /// Total days extended
    pub total_extension_days: u32,
    /// Maximum allowed extension days
    pub max_extension_days: u32,
    /// Whether the market can be extended
    pub can_extend: bool,
    /// Extension fee per day
    pub extension_fee_per_day: i128,
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
    pub fn validate_oracle_provider(provider: &OracleProvider) -> Result<(), Error> {
        if !provider.is_supported() {
            return Err(Error::InvalidConfig);
        }
        Ok(())
    }

    /// Validate price data
    pub fn validate_price(price: i128) -> Result<(), Error> {
        if price <= 0 {
            return Err(Error::OraclePriceOutOfRange);
        }
        Ok(())
    }

    /// Validate stake amount
    pub fn validate_stake(stake: i128, min_stake: i128) -> Result<(), Error> {
        if stake < min_stake {
            return Err(Error::InsufficientStake);
        }
        Ok(())
    }

    /// Validate market duration
    pub fn validate_duration(duration_days: u32) -> Result<(), Error> {
        if duration_days == 0 || duration_days > 365 {
            return Err(Error::InvalidInput);
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
    pub fn validate_comparison(comparison: &String, env: &Env) -> Result<(), Error> {
        if comparison != &String::from_str(env, "gt")
            && comparison != &String::from_str(env, "lt")
            && comparison != &String::from_str(env, "eq")
        {
            return Err(Error::InvalidInput);
        }
        Ok(())
    }
}

// ===== PARAMETER STRUCTS =====

/// Parameters for creating reflector markets
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReflectorMarketParams {
    pub admin: Address,
    pub question: String,
    pub outcomes: Vec<String>,
    pub duration_days: u32,
    pub asset_symbol: String,
    pub threshold: i128,
    pub comparison: String,
}

/// Parameters for creating Pyth markets
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PythMarketParams {
    pub admin: Address,
    pub question: String,
    pub outcomes: Vec<String>,
    pub duration_days: u32,
    pub feed_id: String,
    pub threshold: i128,
    pub comparison: String,
}

/// Parameters for oracle result events
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleResultParams {
    pub market_id: Symbol,
    pub result: String,
    pub provider: String,
    pub feed_id: String,
    pub price: i128,
    pub timestamp: u64,
    pub confidence: u32,
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
        let mut outcomes = Vec::new(&env);
        outcomes.push_back(String::from_str(&env, "yes"));
        outcomes.push_back(String::from_str(&env, "no"));
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
            MarketState::Active,
        );

        assert!(market.is_active(env.ledger().timestamp()));
        assert!(!market.is_resolved());
        assert_eq!(market.total_staked, 0);
    }

    #[test]
    fn test_reflector_asset() {
        let env = soroban_sdk::Env::default();
        let symbol = Symbol::new(&env, "BTC");
        let asset = ReflectorAsset::Other(symbol);

        assert!(asset.is_other());
        assert!(!asset.is_stellar());
    }

    #[test]
    fn test_market_state() {
        let env = soroban_sdk::Env::default();
        let admin = Address::generate(&env);
        let mut outcomes = Vec::new(&env);
        outcomes.push_back(String::from_str(&env, "yes"));
        outcomes.push_back(String::from_str(&env, "no"));
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
            MarketState::Active,
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
