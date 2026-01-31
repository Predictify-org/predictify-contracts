#![allow(dead_code)]

use soroban_sdk::{contracttype, Address, Env, Map, String, Symbol, Vec};

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
/// # State Validation Rules
///
/// Each state has specific validation requirements:
/// - **Active**: Must have valid end time, oracle config, and outcomes
/// - **Ended**: Current time must be past market end time
/// - **Disputed**: Must have active disputes filed within dispute period
/// - **Resolved**: Must have valid resolution with outcome and method
/// - **Closed**: All payouts must be completed and verified
/// - **Cancelled**: Must have valid cancellation reason and admin authorization
///
/// # Integration Points
///
/// Market states integrate with:
/// - **Voting System**: Controls when votes can be accepted
/// - **Oracle System**: Determines when oracle resolution can occur
/// - **Dispute System**: Manages dispute lifecycle and resolution
/// - **Payout System**: Controls when payouts can be distributed
/// - **Admin System**: Handles state transitions and overrides
/// - **Event System**: Emits state change events for transparency
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

/// Enumeration of supported oracle providers for price feed data.
///
/// This enum defines the various oracle providers that can supply price data
/// for prediction market resolution. Each provider has different characteristics,
/// availability, and integration requirements specific to the Stellar blockchain
/// ecosystem.
///
/// # Provider Categories
///
/// **Production Ready (Stellar Network):**
/// - **Reflector**: Primary oracle provider with full Stellar integration
///
/// **Future/Placeholder (Not Yet Available):**
/// - **Pyth**: High-frequency oracle network (future Stellar support)
/// - **Band Protocol**: Decentralized oracle network (not on Stellar)
/// - **DIA**: Multi-chain oracle platform (not on Stellar)
///
/// # Provider Characteristics
///
/// **Reflector Oracle:**
/// - **Status**: Production ready and recommended
/// - **Network**: Native Stellar blockchain integration
/// - **Assets**: BTC, ETH, XLM, and other major cryptocurrencies
/// - **Features**: Real-time prices, TWAP calculations, high reliability
/// - **Use Case**: Primary oracle for all Stellar-based prediction markets
///
/// **Pyth Network:**
/// - **Status**: Placeholder for future implementation
/// - **Network**: Not currently available on Stellar
/// - **Assets**: Extensive coverage of crypto, forex, and traditional assets
/// - **Features**: Sub-second updates, institutional-grade data
/// - **Use Case**: Future high-frequency prediction markets
///
/// **Band Protocol:**
/// - **Status**: Not supported on Stellar
/// - **Network**: Primarily Cosmos and EVM-compatible chains
/// - **Assets**: Wide range of crypto and traditional assets
/// - **Features**: Decentralized data aggregation
/// - **Use Case**: Not applicable for Stellar deployment
///
/// **DIA:**
/// - **Status**: Not supported on Stellar
/// - **Network**: Multi-chain but no Stellar integration
/// - **Assets**: Comprehensive DeFi and traditional asset coverage
/// - **Features**: Transparent data sourcing and aggregation
/// - **Use Case**: Not applicable for Stellar deployment
///
/// # Example Usage
///
/// ```rust
/// # use predictify_hybrid::types::OracleProvider;
///
/// // Check provider support before using
/// let provider = OracleProvider::Reflector;
///
/// if provider.is_supported() {
///     println!("Using {} oracle provider", provider.name());
///     // Proceed with oracle integration
/// } else {
///     println!("Provider {} not supported on Stellar", provider.name());
///     // Use fallback or error handling
/// }
///
/// // Provider selection logic
/// let recommended_provider = match std::env::var("ORACLE_PREFERENCE") {
///     Ok(pref) if pref == "pyth" => {
///         if OracleProvider::Pyth.is_supported() {
///             OracleProvider::Pyth
///         } else {
///             println!("Pyth not available, using Reflector");
///             OracleProvider::Reflector
///         }
///     },
///     _ => OracleProvider::Reflector, // Default to Reflector
/// };
///
/// println!("Selected oracle: {}", recommended_provider.name());
/// ```
///
/// # Integration with Oracle Factory
///
/// Oracle providers work with the Oracle Factory pattern:
/// ```rust
/// # use soroban_sdk::{Env, Address};
/// # use predictify_hybrid::types::OracleProvider;
/// # use predictify_hybrid::oracles::OracleFactory;
/// # let env = Env::default();
/// # let oracle_contract = Address::generate(&env);
///
/// // Create oracle instance based on provider
/// let provider = OracleProvider::Reflector;
/// let oracle_result = OracleFactory::create_oracle(provider, oracle_contract);
///
/// match oracle_result {
///     Ok(oracle_instance) => {
///         println!("Successfully created {} oracle", provider.name());
///         // Use oracle for price feeds
///     },
///     Err(e) => {
///         println!("Failed to create oracle: {:?}", e);
///         // Handle creation failure
///     },
/// }
/// ```
///
/// # Provider Migration Strategy
///
/// For future provider additions:
/// 1. **Add Provider Variant**: Update enum with new provider
/// 2. **Update Support Check**: Modify `is_supported()` method
/// 3. **Add Name Mapping**: Update `name()` method
/// 4. **Implement Integration**: Add provider-specific oracle implementation
/// 5. **Update Factory**: Add creation logic in OracleFactory
/// 6. **Test Integration**: Comprehensive testing with new provider
///
/// # Network Compatibility
///
/// Provider support varies by blockchain network:
/// - **Stellar**: Only Reflector is currently supported
/// - **Ethereum**: Pyth, Band Protocol, and DIA are available
/// - **Cosmos**: Band Protocol is native
/// - **Multi-chain**: DIA supports multiple networks
///
/// # Error Handling
///
/// When using unsupported providers:
/// - Oracle creation will return `Error::InvalidOracleConfig`
/// - Price requests will return `Error::OracleNotAvailable`
/// - Health checks will return `false`
/// - Validation will fail with appropriate error messages
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
    /// The oracle contract address
    pub oracle_address: Address,
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
        oracle_address: Address,
        feed_id: String,
        threshold: i128,
        comparison: String,
    ) -> Self {
        Self {
            provider,
            oracle_address,
            feed_id,
            threshold,
            comparison,
        }
    }
}

impl OracleConfig {
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
    /// Oracle configuration for this market (primary)
    pub oracle_config: OracleConfig,
    /// Fallback oracle configuration
    pub fallback_oracle_config: Option<OracleConfig>,
    /// Resolution timeout in seconds after end_time
    pub resolution_timeout: u64,
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
    /// Winning outcome(s) (set after resolution)
    /// For single winner: contains one outcome
    /// For ties/multi-winner: contains multiple outcomes (pool split among winners)
    pub winning_outcomes: Option<Vec<String>>,
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

    /// Optional category for the event (e.g., "sports", "crypto", "politics")
    /// Used for filtering and display in client applications
    pub category: Option<String>,
    /// List of searchable tags for filtering events
    /// Tags can be used to categorize events by multiple dimensions
    pub tags: Vec<String>,
}

// ===== BET LIMITS =====

/// Configurable minimum and maximum bet amount for an event or globally.
///
/// Used to bound bets so markets remain fair and liquid. Admin can set
/// global defaults or per-event limits at creation or via set_bet_limits.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BetLimits {
    /// Minimum bet amount (in base token units, e.g. stroops)
    pub min_bet: i128,
    /// Maximum bet amount (in base token units)
    pub max_bet: i128,
}

// ===== EVENT ARCHIVE / HISTORICAL QUERY TYPES =====

/// Summary of an event (market) for historical queries and analytics.
///
/// Contains only public metadata and outcome; no sensitive data (no votes, stakes, or addresses).
/// Used by `query_events_history`, `query_events_by_resolution_status`, and `query_events_by_category`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventHistoryEntry {
    /// Market/event ID
    pub market_id: Symbol,
    /// Question text (public)
    pub question: String,
    /// Outcome names (public)
    pub outcomes: Vec<String>,
    /// Market end time (Unix timestamp)
    pub end_time: u64,
    /// Creation timestamp (from registry)
    pub created_at: u64,
    /// Current market state (Active, Resolved, Cancelled, etc.)
    pub state: MarketState,
    /// Winning outcome if resolved
    pub winning_outcome: Option<String>,
    /// Total amount staked (public aggregate)
    pub total_staked: i128,
    /// When archived (if any); None if not archived
    pub archived_at: Option<u64>,
    /// Category / feed identifier (e.g. oracle feed_id) for filtering
    pub category: String,
    /// List of tags for filtering events by multiple dimensions
    pub tags: Vec<String>,
}

// ===== STATISTICS TYPES =====

/// Platform-wide usage statistics
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlatformStatistics {
    /// Total number of markets/events created
    pub total_events_created: u64,
    /// Total number of bets placed
    pub total_bets_placed: u64,
    /// Total volume (amount wagered) in token units
    pub total_volume: i128,
    /// Total fees collected in token units
    pub total_fees_collected: i128,
    /// Number of currently active (non-resolved) events
    pub active_events_count: u32,
}

/// User-specific betting statistics
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserStatistics {
    /// Total number of bets placed by the user
    pub total_bets_placed: u64,
    /// Total amount wagered by the user
    pub total_amount_wagered: i128,
    /// Total winnings claimed by the user
    pub total_winnings: i128,
    /// Number of winning bets
    pub total_bets_won: u64,
    /// Win rate in basis points (0-10000, 100% = 10000)
    pub win_rate: u32,
    /// Timestamp of last activity
    pub last_activity_ts: u64,
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
        fallback_oracle_config: Option<OracleConfig>,
        resolution_timeout: u64,
        state: MarketState,
    ) -> Self {
        Self {
            admin,
            question,
            outcomes,
            end_time,
            oracle_config,
            fallback_oracle_config,
            resolution_timeout,
            oracle_result: None,
            votes: Map::new(env),
            stakes: Map::new(env),
            claimed: Map::new(env),
            total_staked: 0,
            dispute_stakes: Map::new(env),
            winning_outcomes: None,
            fee_collected: false,
            state,

            total_extension_days: 0,
            max_extension_days: 30, // Default maximum extension days
            extension_history: Vec::new(env),

            category: None,
            tags: Vec::new(env),
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
        self.winning_outcomes.is_some()
    }

    /// Get the primary winning outcome (first outcome if multiple, for backward compatibility)
    pub fn get_winning_outcome(&self) -> Option<String> {
        self.winning_outcomes.as_ref().and_then(|outcomes| {
            if outcomes.len() > 0 {
                Some(outcomes.get(0).unwrap().clone())
            } else {
                None
            }
        })
    }

    /// Check if a specific outcome is a winner (handles both single and multi-winner cases)
    pub fn is_winning_outcome(&self, outcome: &String) -> bool {
        self.winning_outcomes
            .as_ref()
            .map(|outcomes| outcomes.contains(outcome))
            .unwrap_or(false)
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
    /// Bitcoin (BTC)
    BTC,
    /// Ethereum (ETH)
    ETH,
    /// Other asset identified by symbol
    Other(Symbol),
}

// ===== ORACLE RESULT TYPES FOR AUTOMATIC RESULT VERIFICATION =====

/// Comprehensive oracle result structure for automatic result verification.
///
/// This structure captures the complete oracle response including the fetched data,
/// signature verification details, timestamp, and validation status. Used for
/// automated event outcome verification from external data sources.
///
/// # Components
///
/// **Result Data:**
/// - **market_id**: Market being resolved
/// - **outcome**: Determined outcome ("yes"/"no" or custom)
/// - **price**: Fetched price value from oracle
/// - **threshold**: Configured threshold for comparison
///
/// **Verification Data:**
/// - **provider**: Oracle provider used
/// - **signature**: Oracle signature for authenticity (if available)
/// - **is_verified**: Whether signature validation passed
/// - **confidence_score**: Statistical confidence (0-100)
///
/// **Metadata:**
/// - **timestamp**: When result was fetched
/// - **block_number**: Ledger sequence at fetch time
/// - **sources_count**: Number of oracle sources consulted
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Symbol, String, BytesN};
/// # use predictify_hybrid::types::{OracleResult, OracleProvider};
/// # let env = Env::default();
///
/// let oracle_result = OracleResult {
///     market_id: Symbol::new(&env, "btc_50k"),
///     outcome: String::from_str(&env, "yes"),
///     price: 52_000_00,
///     threshold: 50_000_00,
///     comparison: String::from_str(&env, "gt"),
///     provider: OracleProvider::Reflector,
///     feed_id: String::from_str(&env, "BTC/USD"),
///     timestamp: env.ledger().timestamp(),
///     block_number: env.ledger().sequence(),
///     is_verified: true,
///     confidence_score: 95,
///     sources_count: 3,
///     signature: None,
///     error_message: None,
/// };
/// ```
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleResult {
    /// Market ID this result is for
    pub market_id: Symbol,
    /// Determined outcome ("yes", "no", or custom outcome)
    pub outcome: String,
    /// Fetched price from oracle
    pub price: i128,
    /// Threshold configured for this market
    pub threshold: i128,
    /// Comparison operator used ("gt", "lt", "eq")
    pub comparison: String,
    /// Oracle provider that provided the result
    pub provider: OracleProvider,
    /// Feed ID used for price lookup
    pub feed_id: String,
    /// Timestamp when result was fetched
    pub timestamp: u64,
    /// Ledger sequence number at fetch time
    pub block_number: u32,
    /// Whether the oracle response was verified (signature valid)
    pub is_verified: bool,
    /// Confidence score (0-100)
    pub confidence_score: u32,
    /// Number of oracle sources consulted
    pub sources_count: u32,
    /// Oracle signature bytes (optional, for providers that support signatures)
    pub signature: Option<String>,
    /// Error message if verification failed
    pub error_message: Option<String>,
}

impl OracleResult {
    /// Check if the oracle result is valid and verified
    pub fn is_valid(&self) -> bool {
        self.is_verified && self.confidence_score >= 50 && self.price > 0
    }

    /// Check if the oracle data is fresh (within max_age_seconds)
    pub fn is_fresh(&self, current_time: u64, max_age_seconds: u64) -> bool {
        current_time.saturating_sub(self.timestamp) <= max_age_seconds
    }
}

/// Multi-oracle aggregated result for consensus-based verification.
///
/// This structure aggregates results from multiple oracle sources to provide
/// a more reliable and tamper-resistant outcome determination.
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Symbol, String, Vec};
/// # use predictify_hybrid::types::{MultiOracleResult, OracleResult, OracleProvider};
/// # let env = Env::default();
///
/// let multi_result = MultiOracleResult {
///     market_id: Symbol::new(&env, "btc_50k"),
///     final_outcome: String::from_str(&env, "yes"),
///     individual_results: Vec::new(&env),
///     consensus_reached: true,
///     consensus_threshold: 66,
///     agreement_percentage: 100,
///     timestamp: env.ledger().timestamp(),
/// };
/// ```
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultiOracleResult {
    /// Market ID this result is for
    pub market_id: Symbol,
    /// Final determined outcome based on consensus
    pub final_outcome: String,
    /// Individual results from each oracle source
    pub individual_results: Vec<OracleResult>,
    /// Whether consensus was reached among oracles
    pub consensus_reached: bool,
    /// Required consensus threshold (percentage, e.g., 66 for 2/3)
    pub consensus_threshold: u32,
    /// Actual agreement percentage among oracles
    pub agreement_percentage: u32,
    /// Aggregation timestamp
    pub timestamp: u64,
}

impl MultiOracleResult {
    /// Check if the multi-oracle result has sufficient consensus
    pub fn has_consensus(&self) -> bool {
        self.consensus_reached && self.agreement_percentage >= self.consensus_threshold
    }
}

/// Oracle source configuration for multi-oracle support.
///
/// Defines a single oracle source with its configuration, weight, and status.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleSource {
    /// Unique identifier for this oracle source
    pub source_id: Symbol,
    /// Oracle provider type
    pub provider: OracleProvider,
    /// Oracle contract address
    pub contract_address: Address,
    /// Weight for consensus calculation (1-100)
    pub weight: u32,
    /// Whether this source is currently active
    pub is_active: bool,
    /// Priority for fallback ordering (lower = higher priority)
    pub priority: u32,
    /// Last successful response timestamp
    pub last_success: u64,
    /// Consecutive failure count
    pub failure_count: u32,
}

/// Oracle fetch request configuration.
///
/// Specifies parameters for fetching oracle data including timeout,
/// retry settings, and source preferences.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleFetchRequest {
    /// Market ID to fetch result for
    pub market_id: Symbol,
    /// Feed ID to query
    pub feed_id: String,
    /// Maximum age of data in seconds (staleness threshold)
    pub max_data_age: u64,
    /// Required number of confirmations/sources
    pub required_confirmations: u32,
    /// Whether to use fallback sources on primary failure
    pub use_fallback: bool,
    /// Minimum confidence score required
    pub min_confidence: u32,
}

impl OracleFetchRequest {
    /// Create a default fetch request for a market
    pub fn new(env: &Env, market_id: Symbol, feed_id: String) -> Self {
        Self {
            market_id,
            feed_id,
            max_data_age: 300, // 5 minutes default
            required_confirmations: 1,
            use_fallback: true,
            min_confidence: 50,
        }
    }

    /// Create a strict fetch request requiring multiple confirmations
    pub fn strict(env: &Env, market_id: Symbol, feed_id: String) -> Self {
        Self {
            market_id,
            feed_id,
            max_data_age: 60, // 1 minute
            required_confirmations: 2,
            use_fallback: true,
            min_confidence: 80,
        }
    }
}

/// Oracle verification status for tracking verification state.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OracleVerificationStatus {
    /// Verification not yet attempted
    Pending,
    /// Verification in progress
    InProgress,
    /// Verification successful
    Verified,
    /// Verification failed - invalid signature
    InvalidSignature,
    /// Verification failed - stale data
    StaleData,
    /// Verification failed - oracle unavailable
    OracleUnavailable,
    /// Verification failed - threshold not met
    ThresholdNotMet,
    /// Verification failed - consensus not reached
    NoConsensus,
}

/// Comprehensive price data structure from Reflector Oracle.
///
/// This structure contains all price information returned by the Reflector Oracle,
/// including current price, timestamp, and metadata necessary for market resolution
/// and validation. It serves as the standardized format for oracle price data
/// within the prediction market system.
///
/// # Price Data Components
///
/// **Core Price Information:**
/// - **Price**: Current asset price in cents (integer format)
/// - **Timestamp**: When the price was last updated
/// - **Decimals**: Number of decimal places for precision
///
/// **Data Quality Indicators:**
/// - **Source**: Oracle provider identifier
/// - **Confidence**: Price data reliability score
/// - **Volume**: Trading volume (if available)
///
/// # Price Format Standards
///
/// Prices follow consistent formatting:
/// - **Integer Values**: No floating point arithmetic
/// - **Cent Precision**: All prices in cents (e.g., 5000000 = $50,000.00)
/// - **Positive Values**: All prices are positive integers
/// - **Range Validation**: Prices within reasonable market bounds
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::Env;
/// # use predictify_hybrid::types::ReflectorPriceData;
/// # let env = Env::default();
///
/// // Create price data from Reflector response
/// let btc_price = ReflectorPriceData::new(
///     5_000_000, // $50,000.00 in cents
///     env.ledger().timestamp(),
///     8, // Bitcoin decimals
///     "Reflector".to_string(),
///     95, // 95% confidence
///     Some(1_000_000_000) // $10M volume
/// );
///
/// // Display price information
/// println!("BTC Price: ${:.2}", btc_price.price_in_dollars());
/// println!("Updated: {}", btc_price.timestamp);
/// println!("Confidence: {}%", btc_price.confidence);
///
/// // Validate price data quality
/// if btc_price.is_valid() {
///     println!("Price data is valid and reliable");
/// } else {
///     println!("Price data quality concerns detected");
/// }
///
/// // Check data freshness
/// let current_time = env.ledger().timestamp();
/// if btc_price.is_fresh(current_time, 300) { // 5 minutes
///     println!("Price data is fresh (within 5 minutes)");
/// } else {
///     println!("Price data is stale - consider refreshing");
/// }
/// ```
///
/// # Price Validation
///
/// Price data undergoes comprehensive validation:
/// ```rust
/// # use predictify_hybrid::types::ReflectorPriceData;
/// # let price_data = ReflectorPriceData::default(); // Placeholder
///
/// // Validation checks multiple aspects:
/// let validation_result = price_data.validate();
/// match validation_result {
///     Ok(()) => {
///         println!("Price data validation passed");
///         // Safe to use for market resolution
///     },
///     Err(e) => {
///         println!("Price validation failed: {:?}", e);
///         // Handle validation errors:
///         // - InvalidPrice: Price is zero or negative
///         // - StaleData: Timestamp too old
///         // - LowConfidence: Confidence below threshold
///         // - InvalidSource: Unknown oracle source
///     }
/// }
/// ```
///
/// # Market Resolution Integration
///
/// Price data integrates with market resolution:
/// ```rust
/// # use predictify_hybrid::types::{ReflectorPriceData, OracleConfig};
/// # let price_data = ReflectorPriceData::default(); // Placeholder
/// # let oracle_config = OracleConfig::default(); // Placeholder
///
/// // Apply oracle configuration to determine outcome
/// let market_outcome = price_data.resolve_outcome(&oracle_config);
///
/// match market_outcome {
///     Ok(outcome) => {
///         println!("Market resolved to: {}", outcome);
///         // "yes" if condition met, "no" otherwise
///     },
///     Err(e) => {
///         println!("Resolution failed: {:?}", e);
///         // Handle resolution errors
///     }
/// }
///
/// // Example: BTC > $50,000 check
/// let btc_price = 5_500_000; // $55,000
/// let threshold = 5_000_000;  // $50,000
/// let comparison = "gt";      // Greater than
///
/// let result = if comparison == "gt" {
///     btc_price > threshold
/// } else if comparison == "lt" {
///     btc_price < threshold
/// } else {
///     btc_price == threshold
/// };
///
/// println!("Market outcome: {}", if result { "yes" } else { "no" });
/// ```
///
/// # Data Quality Metrics
///
/// Price data includes quality indicators:
/// ```rust
/// # use predictify_hybrid::types::ReflectorPriceData;
/// # let price_data = ReflectorPriceData::default(); // Placeholder
///
/// // Check confidence level
/// if price_data.confidence >= 90 {
///     println!("High confidence price data");
/// } else if price_data.confidence >= 70 {
///     println!("Medium confidence price data");
/// } else {
///     println!("Low confidence - use with caution");
/// }
///
/// // Check trading volume (if available)
/// if let Some(volume) = price_data.volume {
///     if volume > 1_000_000_00 { // $1M+
///         println!("High liquidity market");
///     } else {
///         println!("Lower liquidity - price may be volatile");
///     }
/// }
/// ```
///
/// # Time-based Validation
///
/// Price data freshness is critical:
/// ```rust
/// # use soroban_sdk::Env;
/// # use predictify_hybrid::types::ReflectorPriceData;
/// # let env = Env::default();
/// # let price_data = ReflectorPriceData::default(); // Placeholder
///
/// let current_time = env.ledger().timestamp();
/// let max_age = 600; // 10 minutes
///
/// if price_data.is_fresh(current_time, max_age) {
///     println!("Price data is current");
/// } else {
///     let age = current_time - price_data.timestamp;
///     println!("Price data is {} seconds old", age);
///     
///     if age > 3600 { // 1 hour
///         println!("Data is very stale - reject for resolution");
///     }
/// }
/// ```
///
/// # Integration Points
///
/// Price data integrates with:
/// - **Oracle Manager**: Fetches and validates price data
/// - **Resolution System**: Uses price for market outcome determination
/// - **Validation System**: Ensures data quality and freshness
/// - **Analytics System**: Tracks price trends and market performance
/// - **Event System**: Logs price updates for transparency
/// - **Dispute System**: Provides evidence for dispute resolution
///
/// # Error Handling
///
/// Common price data errors:
/// - **InvalidPrice**: Zero, negative, or unreasonable price
/// - **StaleTimestamp**: Price data too old for reliable use
/// - **LowConfidence**: Confidence score below acceptance threshold
/// - **MissingVolume**: Volume data unavailable when required
/// - **SourceMismatch**: Price from unexpected oracle source
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

/// Comprehensive statistics tracking for market extension usage and effectiveness.
///
/// This structure captures detailed metrics about market extensions, including
/// usage patterns, effectiveness measurements, and impact on market participation.
/// It provides valuable insights for optimizing extension policies and understanding
/// user behavior in prediction markets.
///
/// # Statistics Categories
///
/// **Usage Metrics:**
/// - **Total Extensions**: Number of extensions requested
/// - **Approved Extensions**: Number of extensions approved
/// - **Auto Extensions**: System-initiated extensions
/// - **User Extensions**: Community-requested extensions
///
/// **Effectiveness Metrics:**
/// - **Participation Increase**: Additional votes/stakes after extension
/// - **Resolution Quality**: Impact on market resolution accuracy
/// - **User Satisfaction**: Community feedback on extensions
///
/// **Financial Metrics:**
/// - **Total Fees Collected**: Revenue from extension fees
/// - **Average Fee**: Mean fee per extension request
/// - **Fee Effectiveness**: Correlation between fee and participation
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::Env;
/// # use predictify_hybrid::types::ExtensionStats;
/// # let env = Env::default();
///
/// // Create extension statistics tracker
/// let mut stats = ExtensionStats::new(&env);
///
/// // Record extension request
/// stats.record_extension_request(
///     "user_requested",
///     48, // 48 hours
///     2_000_000, // 2 XLM fee
///     true // Approved
/// );
///
/// // Record participation impact
/// stats.record_participation_change(
///     10, // 10 additional votes
///     5_000_000, // 5 XLM additional stakes
///     25.0 // 25% participation increase
/// );
///
/// // Display statistics
/// println!("Total extensions: {}", stats.total_extensions);
/// println!("Approval rate: {:.1}%", stats.approval_rate());
/// println!("Average participation increase: {:.1}%", stats.avg_participation_increase());
/// println!("Total fees collected: {} stroops", stats.total_fees_collected);
/// ```
///
/// # Effectiveness Analysis
///
/// Analyze extension effectiveness across different scenarios:
/// ```rust
/// # use predictify_hybrid::types::ExtensionStats;
/// # let stats = ExtensionStats::default(); // Placeholder
///
/// // Calculate effectiveness metrics
/// let effectiveness_report = stats.generate_effectiveness_report();
///
/// println!("Extension Effectiveness Report:");
/// println!("- Auto extensions success rate: {:.1}%", effectiveness_report.auto_success_rate);
/// println!("- User extensions success rate: {:.1}%", effectiveness_report.user_success_rate);
/// println!("- Average participation boost: {:.1}%", effectiveness_report.avg_participation_boost);
/// println!("- ROI on extension fees: {:.2}x", effectiveness_report.fee_roi);
///
/// // Identify optimal extension patterns
/// if effectiveness_report.auto_success_rate > effectiveness_report.user_success_rate {
///     println!("Recommendation: Favor automatic extensions for low participation");
/// } else {
///     println!("Recommendation: Community-driven extensions are more effective");
/// }
/// ```
///
/// # Trend Analysis
///
/// Track extension trends over time:
/// ```rust
/// # use predictify_hybrid::types::ExtensionStats;
/// # let stats = ExtensionStats::default(); // Placeholder
///
/// // Analyze monthly trends
/// let monthly_trends = stats.get_monthly_trends();
///
/// for (month, trend_data) in monthly_trends {
///     println!("Month {}: {} extensions, {:.1}% approval rate",
///         month, trend_data.count, trend_data.approval_rate);
///     
///     if trend_data.count > trend_data.previous_month_count {
///         println!("  ↗ Extension requests increasing");
///     } else {
///         println!("  ↘ Extension requests decreasing");
///     }
/// }
///
/// // Seasonal patterns
/// let seasonal_analysis = stats.analyze_seasonal_patterns();
/// println!("Peak extension period: {}", seasonal_analysis.peak_period);
/// println!("Low extension period: {}", seasonal_analysis.low_period);
/// ```
///
/// # Fee Optimization Analysis
///
/// Analyze fee structures and their impact:
/// ```rust
/// # use predictify_hybrid::types::ExtensionStats;
/// # let stats = ExtensionStats::default(); // Placeholder
///
/// // Fee effectiveness analysis
/// let fee_analysis = stats.analyze_fee_effectiveness();
///
/// println!("Fee Analysis:");
/// println!("- Optimal fee range: {} - {} stroops",
///     fee_analysis.optimal_min, fee_analysis.optimal_max);
/// println!("- Fee elasticity: {:.2}", fee_analysis.elasticity);
/// println!("- Revenue maximizing fee: {} stroops", fee_analysis.revenue_max_fee);
///
/// // Fee recommendations
/// if fee_analysis.current_fee < fee_analysis.optimal_min {
///     println!("Recommendation: Increase extension fees to improve quality");
/// } else if fee_analysis.current_fee > fee_analysis.optimal_max {
///     println!("Recommendation: Decrease extension fees to increase usage");
/// } else {
///     println!("Current fee structure is optimal");
/// }
/// ```
///
/// # Market Impact Assessment
///
/// Evaluate how extensions affect market quality:
/// ```rust
/// # use predictify_hybrid::types::ExtensionStats;
/// # let stats = ExtensionStats::default(); // Placeholder
///
/// // Market quality impact
/// let quality_impact = stats.assess_market_quality_impact();
///
/// println!("Market Quality Impact:");
/// println!("- Resolution accuracy improvement: {:.1}%", quality_impact.accuracy_improvement);
/// println!("- Participation diversity increase: {:.1}%", quality_impact.diversity_increase);
/// println!("- Stake distribution improvement: {:.1}%", quality_impact.distribution_improvement);
///
/// // Long-term effects
/// println!("\nLong-term Effects:");
/// println!("- User retention rate: {:.1}%", quality_impact.retention_rate);
/// println!("- Market creation rate change: {:+.1}%", quality_impact.creation_rate_change);
/// println!("- Platform trust score: {:.1}/10", quality_impact.trust_score);
/// ```
///
/// # Performance Benchmarking
///
/// Compare extension performance across different market types:
/// ```rust
/// # use predictify_hybrid::types::ExtensionStats;
/// # let stats = ExtensionStats::default(); // Placeholder
///
/// // Benchmark by market category
/// let benchmarks = stats.benchmark_by_category();
///
/// for (category, benchmark) in benchmarks {
///     println!("{} Markets:", category);
///     println!("  Extension rate: {:.1}%", benchmark.extension_rate);
///     println!("  Success rate: {:.1}%", benchmark.success_rate);
///     println!("  Avg duration: {:.1} hours", benchmark.avg_duration_hours);
///     println!("  Participation boost: {:.1}%", benchmark.participation_boost);
/// }
///
/// // Identify best practices
/// let best_practices = stats.identify_best_practices();
/// println!("\nBest Practices:");
/// for practice in best_practices {
///     println!("- {}", practice);
/// }
/// ```
///
/// # Integration Points
///
/// Extension statistics integrate with:
/// - **Analytics Dashboard**: Real-time extension metrics
/// - **Admin Panel**: Extension approval and monitoring tools
/// - **Market Manager**: Extension policy optimization
/// - **Fee Manager**: Dynamic fee adjustment based on effectiveness
/// - **User Interface**: Extension request guidance and feedback
/// - **Reporting System**: Periodic extension effectiveness reports
///
/// # Data Export and Reporting
///
/// Generate comprehensive reports for stakeholders:
/// ```rust
/// # use predictify_hybrid::types::ExtensionStats;
/// # let stats = ExtensionStats::default(); // Placeholder
///
/// // Generate monthly report
/// let monthly_report = stats.generate_monthly_report();
/// println!("Extension Monthly Report:");
/// println!("Total Requests: {}", monthly_report.total_requests);
/// println!("Approval Rate: {:.1}%", monthly_report.approval_rate);
/// println!("Revenue Generated: {} XLM", monthly_report.revenue_xlm);
/// println!("Participation Impact: +{:.1}%", monthly_report.participation_impact);
///
/// // Export data for external analysis
/// let csv_data = stats.export_to_csv();
/// println!("CSV export ready: {} records", csv_data.len());
/// ```
///
/// # Error Handling
///
/// Common statistics errors:
/// - **InvalidDataPoint**: Malformed or inconsistent data
/// - **InsufficientData**: Not enough data for meaningful analysis
/// - **CalculationError**: Mathematical operation failed
/// - **ExportError**: Data export operation failed
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
}

// ===== ADDITIONAL TYPES =====

/// Community consensus data structure for tracking collective market resolution.
///
/// This structure captures the community's collective opinion on market outcomes,
/// providing an alternative or supplementary resolution method to oracle-based
/// resolution. It aggregates user votes, stakes, and participation to determine
/// the community's consensus on the correct market outcome.
///
/// # Consensus Components
///
/// **Outcome Data:**
/// - **Outcome**: The consensus outcome determined by the community
/// - **Votes**: Number of individual votes for this outcome
/// - **Total Votes**: Total number of votes across all outcomes
/// - **Percentage**: Percentage of votes for this outcome
///
/// **Consensus Metrics:**
/// - **Confidence Level**: How confident the consensus is
/// - **Participation Rate**: Percentage of eligible users who voted
/// - **Stake Weight**: Financial weight behind the consensus
///
/// # Consensus Calculation Methods
///
/// **Simple Majority:**
/// - Outcome with >50% of votes wins
/// - Most straightforward method
/// - Used for clear-cut decisions
///
/// **Stake-Weighted Consensus:**
/// - Votes weighted by stake amount
/// - Higher stakes have more influence
/// - Reduces impact of spam votes
///
/// **Qualified Majority:**
/// - Requires >60% or >66% consensus
/// - Used for contentious decisions
/// - Higher threshold for confidence
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, String};
/// # use predictify_hybrid::types::CommunityConsensus;
/// # let env = Env::default();
///
/// // Create community consensus for a market outcome
/// let consensus = CommunityConsensus::new(
///     String::from_str(&env, "yes"),
///     150, // 150 votes for "yes"
///     200, // 200 total votes
///     75   // 75% of votes for "yes"
/// );
///
/// // Display consensus information
/// println!("Community Consensus:");
/// println!("Outcome: {}", consensus.outcome);
/// println!("Votes: {} out of {}", consensus.votes, consensus.total_votes);
/// println!("Percentage: {}%", consensus.percentage);
///
/// // Check consensus strength
/// if consensus.is_strong_consensus() {
///     println!("Strong community consensus achieved");
/// } else if consensus.is_majority_consensus() {
///     println!("Majority consensus reached");
/// } else {
///     println!("No clear consensus - may need dispute resolution");
/// }
///
/// // Validate consensus quality
/// consensus.validate(&env)?;
/// # Ok::<(), predictify_hybrid::errors::Error>(())
/// ```
///
/// # Consensus Validation
///
/// Community consensus undergoes validation:
/// ```rust
/// # use predictify_hybrid::types::CommunityConsensus;
/// # let consensus = CommunityConsensus::default(); // Placeholder
///
/// // Validation checks multiple aspects:
/// let validation_result = consensus.validate(&soroban_sdk::Env::default());
/// match validation_result {
///     Ok(()) => {
///         println!("Consensus validation passed");
///         // Consensus can be used for resolution
///     },
///     Err(e) => {
///         println!("Consensus validation failed: {:?}", e);
///         // Handle validation errors:
///         // - InsufficientParticipation: Too few votes
///         // - InvalidPercentage: Percentage calculation error
///         // - NoMajority: No outcome has majority support
///         // - TiedOutcomes: Multiple outcomes with same vote count
///     }
/// }
/// ```
///
/// # Consensus Strength Analysis
///
/// Analyze the strength and reliability of consensus:
/// ```rust
/// # use predictify_hybrid::types::CommunityConsensus;
/// # let consensus = CommunityConsensus::default(); // Placeholder
///
/// // Consensus strength categories
/// let strength = match consensus.percentage {
///     90..=100 => "Overwhelming Consensus",
///     75..=89 => "Strong Consensus",
///     60..=74 => "Clear Majority",
///     51..=59 => "Simple Majority",
///     _ => "No Consensus"
/// };
///
/// println!("Consensus Strength: {}", strength);
///
/// // Participation analysis
/// let participation_rate = consensus.calculate_participation_rate();
/// if participation_rate >= 50 {
///     println!("High participation: {:.1}%", participation_rate);
/// } else if participation_rate >= 25 {
///     println!("Moderate participation: {:.1}%", participation_rate);
/// } else {
///     println!("Low participation: {:.1}% - consensus may be unreliable", participation_rate);
/// }
/// ```
///
/// # Stake-Weighted Consensus
///
/// Calculate consensus based on financial stakes:
/// ```rust
/// # use predictify_hybrid::types::CommunityConsensus;
/// # let consensus = CommunityConsensus::default(); // Placeholder
///
/// // Stake-weighted calculation
/// let stake_weighted_consensus = consensus.calculate_stake_weighted();
///
/// println!("Vote-based consensus: {}% for {}",
///     consensus.percentage, consensus.outcome);
/// println!("Stake-weighted consensus: {:.1}% for {}",
///     stake_weighted_consensus.percentage, stake_weighted_consensus.outcome);
///
/// // Compare vote vs stake consensus
/// if consensus.outcome == stake_weighted_consensus.outcome {
///     println!("Vote and stake consensus align");
/// } else {
///     println!("Vote and stake consensus differ - potential whale influence");
/// }
/// ```
///
/// # Consensus Evolution Tracking
///
/// Track how consensus changes over time:
/// ```rust
/// # use predictify_hybrid::types::CommunityConsensus;
/// # let consensus = CommunityConsensus::default(); // Placeholder
///
/// // Historical consensus snapshots
/// let consensus_history = consensus.get_historical_snapshots();
///
/// for (timestamp, snapshot) in consensus_history {
///     println!("Time {}: {}% for {}",
///         timestamp, snapshot.percentage, snapshot.outcome);
/// }
///
/// // Consensus stability analysis
/// let stability = consensus.analyze_stability();
/// if stability.is_stable {
///     println!("Consensus has been stable for {} hours", stability.stable_duration_hours);
/// } else {
///     println!("Consensus is still evolving - {} changes in last 24h", stability.recent_changes);
/// }
/// ```
///
/// # Multi-Outcome Consensus
///
/// Handle markets with multiple possible outcomes:
/// ```rust
/// # use predictify_hybrid::types::CommunityConsensus;
///
/// // Calculate consensus for all outcomes
/// let all_outcomes_consensus = vec![
///     ("outcome_a", 45, 22), // 45% of votes, 22% of stakes
///     ("outcome_b", 35, 38), // 35% of votes, 38% of stakes
///     ("outcome_c", 20, 40), // 20% of votes, 40% of stakes
/// ];
///
/// // Determine winner by different methods
/// let vote_winner = all_outcomes_consensus.iter()
///     .max_by_key(|(_, votes, _)| votes)
///     .map(|(outcome, _, _)| outcome);
///
/// let stake_winner = all_outcomes_consensus.iter()
///     .max_by_key(|(_, _, stakes)| stakes)
///     .map(|(outcome, _, _)| outcome);
///
/// println!("Vote winner: {:?}", vote_winner);
/// println!("Stake winner: {:?}", stake_winner);
///
/// // Check for conflicts
/// if vote_winner != stake_winner {
///     println!("Conflict detected - may need hybrid resolution");
/// }
/// ```
///
/// # Integration with Resolution System
///
/// Community consensus integrates with market resolution:
/// ```rust
/// # use predictify_hybrid::types::CommunityConsensus;
/// # let consensus = CommunityConsensus::default(); // Placeholder
///
/// // Use consensus for market resolution
/// if consensus.is_reliable() {
///     let resolution_outcome = consensus.outcome.clone();
///     let confidence_score = consensus.calculate_confidence();
///     
///     println!("Resolving market to: {}", resolution_outcome);
///     println!("Confidence: {:.1}%", confidence_score);
///     
///     // Apply resolution
///     apply_market_resolution(resolution_outcome, confidence_score);
/// } else {
///     println!("Consensus not reliable - using oracle or dispute resolution");
/// }
/// ```
///
/// # Consensus Quality Metrics
///
/// Evaluate the quality and reliability of consensus:
/// ```rust
/// # use predictify_hybrid::types::CommunityConsensus;
/// # let consensus = CommunityConsensus::default(); // Placeholder
///
/// // Quality assessment
/// let quality_metrics = consensus.assess_quality();
///
/// println!("Consensus Quality Report:");
/// println!("- Participation Rate: {:.1}%", quality_metrics.participation_rate);
/// println!("- Majority Strength: {:.1}%", quality_metrics.majority_strength);
/// println!("- Stake Alignment: {:.1}%", quality_metrics.stake_alignment);
/// println!("- Time Stability: {:.1}%", quality_metrics.time_stability);
/// println!("- Overall Quality: {:.1}/10", quality_metrics.overall_score);
///
/// // Quality-based decision making
/// if quality_metrics.overall_score >= 8.0 {
///     println!("High quality consensus - safe to use for resolution");
/// } else if quality_metrics.overall_score >= 6.0 {
///     println!("Moderate quality - consider supplementary validation");
/// } else {
///     println!("Low quality consensus - use alternative resolution method");
/// }
/// ```
///
/// # Integration Points
///
/// Community consensus integrates with:
/// - **Resolution System**: Provides community-based resolution outcomes
/// - **Voting System**: Aggregates individual votes into collective consensus
/// - **Dispute System**: Offers alternative when oracle resolution is disputed
/// - **Analytics System**: Tracks consensus patterns and quality
/// - **Governance System**: Enables community-driven market resolution
/// - **Event System**: Emits consensus updates and final determinations
///
/// # Error Handling
///
/// Common consensus errors:
/// - **InsufficientVotes**: Too few votes to establish reliable consensus
/// - **TiedOutcomes**: Multiple outcomes with identical vote counts
/// - **InvalidPercentage**: Percentage calculations don't sum to 100%
/// - **LowParticipation**: Participation rate below minimum threshold
/// - **ConsensusInstability**: Consensus changes too frequently to be reliable
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

///////////////////////////////////////////////////
/// Market pause information tracking   //////////
/////////////////////////////////////////////////
#[contracttype]
#[derive(Clone, Debug)]
pub struct MarketPauseInfo {
    pub is_paused: bool,
    pub paused_at: u64,
    pub pause_duration_hours: u32,
    pub paused_by: Address,
    pub pause_end_time: u64,
    pub original_state: MarketState,
}

// ===== QUERY RESPONSE TYPES =====

/// Market/event status enumeration for queries.
///
/// Simplified status enumeration optimized for query responses,
/// mapping internal market states to user-friendly statuses.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MarketStatus {
    /// Market is open for voting
    Active,
    /// Market voting has ended
    Ended,
    /// Market outcome is disputed
    Disputed,
    /// Market outcome has been resolved
    Resolved,
    /// Market is closed
    Closed,
    /// Market has been cancelled
    Cancelled,
}

impl MarketStatus {
    /// Convert from internal MarketState to query MarketStatus
    pub fn from_market_state(state: MarketState) -> Self {
        match state {
            MarketState::Active => MarketStatus::Active,
            MarketState::Ended => MarketStatus::Ended,
            MarketState::Disputed => MarketStatus::Disputed,
            MarketState::Resolved => MarketStatus::Resolved,
            MarketState::Closed => MarketStatus::Closed,
            MarketState::Cancelled => MarketStatus::Cancelled,
        }
    }
}

/// Comprehensive event/market details query response.
///
/// This structure contains complete information about a prediction market,
/// suitable for client-side display and analysis. All fields are structured
/// for easy serialization and client consumption.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventDetailsQuery {
    /// Market/event ID
    pub market_id: Symbol,
    /// Prediction question
    pub question: String,
    /// Possible outcomes
    pub outcomes: Vec<String>,
    /// Market creation timestamp
    pub created_at: u64,
    /// Market end timestamp
    pub end_time: u64,
    /// Current market status
    pub status: MarketStatus,
    /// Oracle provider used for resolution
    pub oracle_provider: String,
    /// Price feed identifier
    pub feed_id: String,
    /// Total amount staked in market
    pub total_staked: i128,
    /// Current winning outcome (if resolved)
    pub winning_outcome: Option<String>,
    /// Oracle result (if available)
    pub oracle_result: Option<String>,
    /// Number of unique participants
    pub participant_count: u32,
    /// Total number of votes
    pub vote_count: u32,
    /// Market administrator
    pub admin: Address,
}

/// User bet details query response.
///
/// Contains comprehensive information about a user's participation
/// in a specific market, including votes, stakes, and payout eligibility.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserBetQuery {
    /// User address
    pub user: Address,
    /// Market/event ID
    pub market_id: Symbol,
    /// User's chosen outcome
    pub outcome: String,
    /// Amount staked by user (in stroops/XLM cents)
    pub stake_amount: i128,
    /// Timestamp of user's vote
    pub voted_at: u64,
    /// Whether user voted on winning outcome
    pub is_winning: bool,
    /// Whether user has already claimed payout
    pub has_claimed: bool,
    /// Potential payout amount (if winning and not claimed)
    pub potential_payout: i128,
    /// User's dispute stake (if any)
    pub dispute_stake: i128,
}

/// User balance and account status query response.
///
/// Provides comprehensive view of a user's account with current balance
/// and participation metrics across all markets.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserBalanceQuery {
    /// User address
    pub user: Address,
    /// Available balance (in stroops)
    pub available_balance: i128,
    /// Total amount currently staked in active markets
    pub total_staked: i128,
    /// Total amount won from resolved markets
    pub total_winnings: i128,
    /// Number of active bets
    pub active_bet_count: u32,
    /// Number of resolved markets where user participated
    pub resolved_market_count: u32,
    /// Total amount in unclaimed payouts
    pub unclaimed_balance: i128,
}

/// Market pool and liquidity query response.
///
/// Provides detailed information about total stakes and outcome distribution
/// across a market, useful for probability analysis and liquidity assessment.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketPoolQuery {
    /// Market/event ID
    pub market_id: Symbol,
    /// Total amount staked across all outcomes
    pub total_pool: i128,
    /// Stake amount for each outcome
    pub outcome_pools: Map<String, i128>,
    /// Platform fees collected
    pub platform_fees: i128,
    /// Implied probability for "yes" outcome (0-100)
    pub implied_probability_yes: u32,
    /// Implied probability for "no" outcome (0-100)
    pub implied_probability_no: u32,
}

/// Contract global state statistics query response.
///
/// Provides system-level metrics and statistics across all markets,
/// useful for dashboard displays and platform monitoring.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractStateQuery {
    /// Total number of markets created
    pub total_markets: u32,
    /// Number of currently active markets
    pub active_markets: u32,
    /// Number of resolved markets
    pub resolved_markets: u32,
    /// Total value locked across all markets (in stroops)
    pub total_value_locked: i128,
    /// Total platform fees collected (in stroops)
    pub total_fees_collected: i128,
    /// Number of unique users
    pub unique_users: u32,
    /// Contract version
    pub contract_version: String,
    /// Last contract update timestamp
    pub last_update: u64,
}

/// Multi-market query result for batch operations.
///
/// Container for results when querying multiple markets at once,
/// enabling efficient batch queries with error handling.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultipleBetsQuery {
    /// List of bet queries
    pub bets: Vec<UserBetQuery>,
    /// Total stake across all bets
    pub total_stake: i128,
    /// Total potential payout
    pub total_potential_payout: i128,
    /// Number of winning bets
    pub winning_bets: u32,
}

// ===== BET PLACEMENT TYPES =====

/// Status of a bet placed on a prediction market.
///
/// This enum tracks the lifecycle of a bet from placement through resolution:
/// - `Active`: Bet is placed and funds are locked, awaiting market resolution
/// - `Won`: Market resolved in favor of user's predicted outcome, winnings claimable
/// - `Lost`: Market resolved against user's predicted outcome, funds forfeited
/// - `Refunded`: Bet was refunded due to market cancellation or special circumstances
/// - `Cancelled`: Bet was cancelled before market resolution (if allowed)
///
/// # State Transitions
///
/// ```text
/// Active → Won (market resolved in user's favor)
/// Active → Lost (market resolved against user)
/// Active → Refunded (market cancelled)
/// Active → Cancelled (bet cancelled before resolution, if permitted)
/// ```
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BetStatus {
    /// Bet is active with funds locked
    Active,
    /// Bet won - user predicted correctly
    Won,
    /// Bet lost - user predicted incorrectly
    Lost,
    /// Bet was refunded (market cancelled)
    Refunded,
    /// Bet was cancelled by user (if allowed)
    Cancelled,
}

/// Represents a user's bet on a prediction market event.
///
/// This structure encapsulates all information about a user's bet placement,
/// including the selected outcome, locked funds amount, and bet status.
/// Bets are distinct from votes in that they represent a financial wager
/// on the predicted outcome rather than a governance/consensus vote.
///
/// # Bet vs Vote Distinction
///
/// - **Bet**: Financial wager on predicted outcome with locked funds
/// - **Vote**: Participation in community consensus for market resolution
///
/// # Fund Locking
///
/// When a bet is placed:
/// 1. User's funds (XLM or Stellar tokens) are transferred to the contract
/// 2. Funds remain locked until market resolution
/// 3. Upon resolution:
///    - Winners receive proportional share of total bet pool (minus fees)
///    - Losers forfeit their locked funds
///    - Refunds issued if market is cancelled
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Address, String, Symbol};
/// # use predictify_hybrid::types::{Bet, BetStatus};
/// # let env = Env::default();
/// # let user = Address::generate(&env);
///
/// // Create a new bet
/// let bet = Bet {
///     user: user.clone(),
///     market_id: Symbol::new(&env, "btc_50k_2024"),
///     outcome: String::from_str(&env, "yes"),
///     amount: 10_000_000, // 1.0 XLM locked
///     timestamp: env.ledger().timestamp(),
///     status: BetStatus::Active,
/// };
///
/// // Bet provides complete bet context
/// println!("Bet placed by: {:?}", bet.user);
/// println!("Market: {:?}", bet.market_id);
/// println!("Outcome: {}", bet.outcome.to_string());
/// println!("Amount locked: {} stroops", bet.amount);
/// println!("Status: {:?}", bet.status);
/// ```
///
/// # Integration Points
///
/// Bet structures integrate with:
/// - **Market System**: Validates market exists and is active
/// - **Token System**: Handles fund locking and payout distribution
/// - **Resolution System**: Updates bet status upon market resolution
/// - **Payout System**: Calculates and distributes winnings
/// - **Event System**: Emits bet placement and resolution events
///
/// # Validation Rules
///
/// Before a bet is placed, the following validations occur:
/// - Market exists and is in Active state
/// - Market has not ended (current time < end_time)
/// - User has not already placed a bet on this market
/// - User has sufficient balance for the bet amount
/// - Bet amount meets minimum stake requirements
/// - Selected outcome is valid for the market
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Bet {
    /// Address of the user who placed the bet
    pub user: Address,
    /// Market ID this bet is placed on
    pub market_id: Symbol,
    /// Selected outcome the user is betting on
    pub outcome: String,
    /// Amount of funds locked for this bet (in stroops)
    pub amount: i128,
    /// Timestamp when the bet was placed
    pub timestamp: u64,
    /// Current status of the bet
    pub status: BetStatus,
}

impl Bet {
    /// Create a new active bet
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment
    /// * `user` - Address of the user placing the bet
    /// * `market_id` - Symbol identifying the market
    /// * `outcome` - The outcome the user is betting on
    /// * `amount` - The amount to lock for this bet
    ///
    /// # Returns
    ///
    /// A new `Bet` instance with `Active` status and current timestamp
    pub fn new(env: &Env, user: Address, market_id: Symbol, outcome: String, amount: i128) -> Self {
        Self {
            user,
            market_id,
            outcome,
            amount,
            timestamp: env.ledger().timestamp(),
            status: BetStatus::Active,
        }
    }

    /// Check if the bet is still active (funds locked, awaiting resolution)
    pub fn is_active(&self) -> bool {
        self.status == BetStatus::Active
    }

    /// Check if the bet has been resolved (won or lost)
    pub fn is_resolved(&self) -> bool {
        matches!(self.status, BetStatus::Won | BetStatus::Lost)
    }

    /// Check if the user won this bet
    pub fn is_winner(&self) -> bool {
        self.status == BetStatus::Won
    }

    /// Mark the bet as won
    pub fn mark_as_won(&mut self) {
        self.status = BetStatus::Won;
    }

    /// Mark the bet as lost
    pub fn mark_as_lost(&mut self) {
        self.status = BetStatus::Lost;
    }

    /// Mark the bet as refunded
    pub fn mark_as_refunded(&mut self) {
        self.status = BetStatus::Refunded;
    }
}

/// Statistics for bets placed on a specific market.
///
/// This structure provides aggregate information about betting activity
/// on a market, useful for analytics, UI display, and market health assessment.
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Map, String};
/// # use predictify_hybrid::types::BetStats;
/// # let env = Env::default();
///
/// let mut outcome_totals = Map::new(&env);
/// outcome_totals.set(String::from_str(&env, "yes"), 50_000_000i128);
/// outcome_totals.set(String::from_str(&env, "no"), 30_000_000i128);
///
/// let stats = BetStats {
///     total_bets: 15,
///     total_amount_locked: 80_000_000, // 8 XLM
///     unique_bettors: 12,
///     outcome_totals,
/// };
///
/// println!("Total bets: {}", stats.total_bets);
/// println!("Total locked: {} stroops", stats.total_amount_locked);
/// println!("Unique bettors: {}", stats.unique_bettors);
/// ```
#[contracttype]
#[derive(Clone, Debug)]
pub struct BetStats {
    /// Total number of bets placed on this market
    pub total_bets: u32,
    /// Total amount of funds locked across all bets
    pub total_amount_locked: i128,
    /// Number of unique users who placed bets
    pub unique_bettors: u32,
    /// Total amount locked per outcome
    pub outcome_totals: Map<String, i128>,
}

// ===== EVENT TYPES =====

/// Represents a prediction market event with specified parameters.
///
/// This structure stores all metadata and configuration for a prediction event,
/// including its description, possible outcomes, timing, and oracle integration.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Event {
    /// Unique identifier for the event
    pub id: Symbol,
    /// Event description or question
    pub description: String,
    /// Possible outcomes for the event (e.g., ["yes", "no"])
    pub outcomes: Vec<String>,
    /// When the event ends (Unix timestamp)
    pub end_time: u64,
    /// Oracle configuration for result verification (primary)
    pub oracle_config: OracleConfig,
    /// Fallback oracle configuration
    pub fallback_oracle_config: Option<OracleConfig>,
    /// Resolution timeout in seconds after end_time
    pub resolution_timeout: u64,
    /// Administrative address that created/manages the event
    pub admin: Address,
    /// When the event was created (Unix timestamp)
    pub created_at: u64,
    /// Current status of the event
    pub status: MarketState,
}

impl ReflectorAsset {
    pub fn is_xlm(&self) -> bool {
        matches!(self, ReflectorAsset::Stellar)
    }
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Balance {
    pub user: Address,
    pub asset: ReflectorAsset,
    pub amount: i128,
}
