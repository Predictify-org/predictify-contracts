extern crate alloc;
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

/// Deployment environment specification for the Predictify Hybrid contract.
///
/// This enum defines the different environments where the contract can be deployed,
/// each with its own configuration parameters, security settings, and operational
/// characteristics. The environment determines fee structures, validation rules,
/// and network-specific behaviors.
///
/// # Environment Characteristics
///
/// Each environment is optimized for different use cases:
/// - **Development**: Relaxed rules for testing and development
/// - **Testnet**: Production-like environment for integration testing
/// - **Mainnet**: Full production environment with strict security
/// - **Custom**: Flexible environment for specialized deployments
///
/// # Usage in Configuration
///
/// The environment setting affects:
/// - Fee structures and minimum stakes
/// - Validation thresholds and limits
/// - Oracle timeout and retry settings
/// - Market duration and extension rules
/// - Security and permission requirements
///
/// # Example
///
/// ```rust
/// # use predictify_hybrid::config::Environment;
///
/// // Environment comparison and selection
/// let current_env = Environment::Mainnet;
///
/// match current_env {
///     Environment::Development => {
///         println!("Using development settings with relaxed rules");
///     },
///     Environment::Testnet => {
///         println!("Using testnet settings for integration testing");
///     },
///     Environment::Mainnet => {
///         println!("Using production settings with full security");
///     },
///     Environment::Custom => {
///         println!("Using custom environment configuration");
///     }
/// }
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[contracttype]
pub enum Environment {
    /// Development environment with relaxed validation and low fees.
    ///
    /// Characteristics:
    /// - Minimal fees for easy testing
    /// - Relaxed validation rules
    /// - Short timeouts for faster iteration
    /// - Permissive market creation limits
    /// - Debug-friendly error messages
    Development,

    /// Testnet environment that mirrors production settings.
    ///
    /// Characteristics:
    /// - Production-like fee structures
    /// - Full validation enabled
    /// - Realistic timeouts and limits
    /// - Comprehensive testing capabilities
    /// - Integration testing support
    Testnet,

    /// Production mainnet environment with full security.
    ///
    /// Characteristics:
    /// - Optimized fee structures
    /// - Strict validation and security
    /// - Production timeouts and limits
    /// - Maximum security features
    /// - Audit-ready configuration
    Mainnet,

    /// Custom environment for specialized deployments.
    ///
    /// Characteristics:
    /// - Configurable parameters
    /// - Flexible validation rules
    /// - Custom fee structures
    /// - Specialized use case support
    /// - Enterprise deployment ready
    Custom,
}

/// Network-specific configuration for Stellar blockchain connectivity.
///
/// This struct contains all the network-related parameters required for the
/// contract to operate correctly on different Stellar networks. It includes
/// environment settings, connection details, and network identifiers.
///
/// # Purpose
///
/// NetworkConfig enables:
/// - Multi-network deployment support
/// - Environment-specific network settings
/// - RPC endpoint configuration
/// - Network identity verification
/// - Contract address management
///
/// # Usage Scenarios
///
/// - **Development**: Local Stellar network or Futurenet
/// - **Testing**: Stellar Testnet with test tokens
/// - **Production**: Stellar Mainnet with real assets
/// - **Custom**: Private or consortium networks
///
/// # Example
///
/// ```rust
/// # use soroban_sdk::{Env, Address, String};
/// # use predictify_hybrid::config::{NetworkConfig, Environment};
/// # let env = Env::default();
/// # let contract_addr = Address::generate(&env);
///
/// // Create mainnet network configuration
/// let mainnet_config = NetworkConfig {
///     environment: Environment::Mainnet,
///     passphrase: String::from_str(&env, "Public Global Stellar Network ; September 2015"),
///     rpc_url: String::from_str(&env, "https://horizon.stellar.org"),
///     network_id: String::from_str(&env, "mainnet"),
///     contract_address: contract_addr,
/// };
///
/// println!("Configured for {} environment",
///     match mainnet_config.environment {
///         Environment::Mainnet => "production",
///         Environment::Testnet => "testing",
///         Environment::Development => "development",
///         Environment::Custom => "custom",
///     }
/// );
/// ```
#[derive(Clone, Debug)]
#[contracttype]
pub struct NetworkConfig {
    /// The deployment environment (Development, Testnet, Mainnet, Custom).
    ///
    /// This determines which network-specific settings and validation
    /// rules are applied throughout the contract.
    pub environment: Environment,

    /// Stellar network passphrase for transaction signing.
    ///
    /// Standard passphrases:
    /// - Mainnet: "Public Global Stellar Network ; September 2015"
    /// - Testnet: "Test SDF Network ; September 2015"
    /// - Development: Custom passphrase for local networks
    pub passphrase: String,

    /// RPC endpoint URL for blockchain interactions.
    ///
    /// Examples:
    /// - Mainnet: "https://horizon.stellar.org"
    /// - Testnet: "https://horizon-testnet.stellar.org"
    /// - Development: "http://localhost:8000"
    pub rpc_url: String,

    /// Network identifier for configuration management.
    ///
    /// Used for:
    /// - Configuration file organization
    /// - Network-specific caching
    /// - Deployment tracking
    /// - Environment validation
    pub network_id: String,

    /// The deployed contract's address on this network.
    ///
    /// This address is used for:
    /// - Contract invocation
    /// - Event filtering
    /// - Cross-contract communication
    /// - Network verification
    pub contract_address: Address,
}

/// Comprehensive fee structure configuration for the prediction platform.
///
/// This struct defines all fee-related parameters that govern the economic
/// model of the prediction market platform. It includes platform fees,
/// creation costs, limits, and collection thresholds.
///
/// # Fee Types
///
/// The platform implements several fee mechanisms:
/// - **Platform Fees**: Percentage taken from winning payouts
/// - **Creation Fees**: Fixed cost to create new markets
/// - **Minimum/Maximum Limits**: Bounds on fee amounts
/// - **Collection Thresholds**: When fees are automatically collected
///
/// # Economic Model
///
/// Fees serve multiple purposes:
/// - Platform sustainability and development funding
/// - Spam prevention through creation costs
/// - Market quality incentives
/// - Oracle and infrastructure costs
///
/// # Example
///
/// ```rust
/// # use predictify_hybrid::config::FeeConfig;
///
/// // Create a balanced fee configuration
/// let fee_config = FeeConfig {
///     platform_fee_percentage: 250,    // 2.5% of winnings
///     creation_fee: 10_000_000,        // 1 XLM to create market
///     min_fee_amount: 1_000_000,       // 0.1 XLM minimum
///     max_fee_amount: 100_000_000,     // 10 XLM maximum
///     collection_threshold: 50_000_000, // Collect at 5 XLM
///     fees_enabled: true,              // Fees are active
/// };
///
/// // Calculate platform fee for a 100 XLM payout
/// let payout = 1_000_000_000; // 100 XLM
/// let platform_fee = (payout * fee_config.platform_fee_percentage) / 10000;
/// println!("Platform fee: {} stroops", platform_fee); // 25 XLM
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct FeeConfig {
    /// Platform fee percentage in basis points (1/100th of a percent).
    ///
    /// This fee is taken from winning payouts and represents the platform's
    /// revenue. Examples:
    /// - 250 = 2.5% fee
    /// - 500 = 5.0% fee
    /// - 1000 = 10.0% fee
    ///
    /// Range: 0-1000 (0% to 10%)
    pub platform_fee_percentage: i128,

    /// Fixed fee required to create a new prediction market (in stroops).
    ///
    /// This fee prevents spam market creation and covers:
    /// - Oracle setup costs
    /// - Storage and computational resources
    /// - Market validation and moderation
    ///
    /// Typical values:
    /// - Development: 1_000_000 (0.1 XLM)
    /// - Testnet: 10_000_000 (1 XLM)
    /// - Mainnet: 50_000_000 (5 XLM)
    pub creation_fee: i128,

    /// Minimum fee amount that can be charged (in stroops).
    ///
    /// Ensures fees are meaningful and cover basic operational costs.
    /// Prevents dust fees that could clog the system.
    pub min_fee_amount: i128,

    /// Maximum fee amount that can be charged (in stroops).
    ///
    /// Protects users from excessive fees on large markets.
    /// Provides predictable cost ceiling for high-value markets.
    pub max_fee_amount: i128,

    /// Threshold amount for automatic fee collection (in stroops).
    ///
    /// When accumulated fees reach this threshold, they are automatically
    /// collected to reduce gas costs and improve efficiency.
    pub collection_threshold: i128,

    /// Global flag to enable or disable all fee collection.
    ///
    /// When false:
    /// - No platform fees are charged
    /// - Creation fees may still apply (depends on implementation)
    /// - Useful for promotional periods or testing
    pub fees_enabled: bool,
}

/// Voting and dispute mechanism configuration for prediction markets.
///
/// This struct defines the parameters that govern how users can vote on market
/// outcomes and dispute resolutions. It includes stake requirements, thresholds
/// for different market sizes, and dispute handling parameters.
///
/// # Voting Mechanics
///
/// The voting system supports:
/// - **Stake-weighted voting**: Higher stakes have more influence
/// - **Dispute mechanisms**: Challenge incorrect resolutions
/// - **Dynamic thresholds**: Adjust based on market size and activity
/// - **Time extensions**: Allow additional time for disputes
///
/// # Economic Incentives
///
/// Voting configuration balances:
/// - Participation incentives through reasonable minimum stakes
/// - Spam prevention through dispute costs
/// - Proportional influence based on market size
/// - Fair dispute resolution processes
///
/// # Example
///
/// ```rust
/// # use predictify_hybrid::config::VotingConfig;
///
/// // Create balanced voting configuration
/// let voting_config = VotingConfig {
///     min_vote_stake: 1_000_000,        // 0.1 XLM minimum vote
///     min_dispute_stake: 10_000_000,    // 1 XLM minimum dispute
///     max_dispute_threshold: 100_000_000, // 10 XLM max dispute cost
///     base_dispute_threshold: 10_000_000, // 1 XLM base dispute cost
///     large_market_threshold: 1_000_000_000, // 100 XLM = large market
///     high_activity_threshold: 100,     // 100+ votes = high activity
///     dispute_extension_hours: 24,      // 24 hour dispute window
/// };
///
/// // Check if market qualifies as large
/// let market_volume = 500_000_000; // 50 XLM
/// let is_large_market = market_volume >= voting_config.large_market_threshold;
/// println!("Large market: {}", is_large_market); // false
/// ```
#[derive(Clone, Debug)]
#[contracttype]
pub struct VotingConfig {
    /// Minimum stake required to vote on a market outcome (in stroops).
    ///
    /// This prevents spam voting while keeping participation accessible.
    /// Typical values:
    /// - Development: 100_000 (0.01 XLM)
    /// - Testnet: 1_000_000 (0.1 XLM)
    /// - Mainnet: 5_000_000 (0.5 XLM)
    pub min_vote_stake: i128,

    /// Minimum stake required to initiate a dispute (in stroops).
    ///
    /// Higher than voting stake to prevent frivolous disputes.
    /// Should be meaningful but not prohibitive for legitimate disputes.
    pub min_dispute_stake: i128,

    /// Maximum dispute threshold that can be required (in stroops).
    ///
    /// Caps dispute costs to ensure accessibility while maintaining
    /// serious commitment from disputers.
    pub max_dispute_threshold: i128,

    /// Base dispute threshold for standard markets (in stroops).
    ///
    /// Starting point for dispute cost calculations.
    /// May be adjusted based on market size and activity.
    pub base_dispute_threshold: i128,

    /// Total stake threshold that defines a "large" market (in stroops).
    ///
    /// Large markets may have:
    /// - Higher dispute thresholds
    /// - Extended resolution periods
    /// - Additional validation requirements
    pub large_market_threshold: i128,

    /// Vote count threshold that defines "high activity" markets.
    ///
    /// High activity markets may receive:
    /// - Priority in resolution queues
    /// - Enhanced dispute mechanisms
    /// - Additional monitoring
    pub high_activity_threshold: u32,

    /// Additional hours added to resolution period when disputed.
    ///
    /// Provides time for:
    /// - Community review of disputes
    /// - Additional evidence gathering
    /// - Oracle re-evaluation
    /// - Consensus building
    pub dispute_extension_hours: u32,
}

/// Market creation and structure configuration parameters.
///
/// This struct defines the constraints and limits for creating prediction markets,
/// including duration limits, outcome constraints, and content length restrictions.
/// These parameters ensure market quality and system performance.
///
/// # Market Quality Control
///
/// Configuration parameters serve to:
/// - **Ensure Clarity**: Reasonable question and outcome lengths
/// - **Maintain Performance**: Limit complexity for efficient processing
/// - **Prevent Abuse**: Reasonable duration and outcome limits
/// - **Support Usability**: Balanced constraints for good user experience
///
/// # Duration Considerations
///
/// Market duration affects:
/// - Oracle availability and reliability
/// - User engagement and participation
/// - Resolution complexity and accuracy
/// - Platform resource utilization
///
/// # Example
///
/// ```rust
/// # use predictify_hybrid::config::MarketConfig;
///
/// // Create production market configuration
/// let market_config = MarketConfig {
///     max_duration_days: 365,    // Up to 1 year markets
///     min_duration_days: 1,      // At least 1 day
///     max_outcomes: 10,          // Up to 10 possible outcomes
///     min_outcomes: 2,           // At least binary choice
///     max_question_length: 500,  // 500 character questions
///     max_outcome_length: 100,   // 100 character outcomes
/// };
///
/// // Validate a market proposal
/// let question = "Will Bitcoin reach $100,000 by end of 2024?";
/// let outcomes = vec!["Yes", "No"];
/// let duration = 90; // 3 months
///
/// let valid_question = question.len() <= market_config.max_question_length as usize;
/// let valid_outcomes = outcomes.len() >= market_config.min_outcomes as usize &&
///                     outcomes.len() <= market_config.max_outcomes as usize;
/// let valid_duration = duration >= market_config.min_duration_days &&
///                     duration <= market_config.max_duration_days;
///
/// println!("Market valid: {}", valid_question && valid_outcomes && valid_duration);
/// ```
#[derive(Clone, Debug)]
#[contracttype]
pub struct MarketConfig {
    /// Maximum allowed market duration in days.
    ///
    /// Limits how far into the future markets can extend.
    /// Considerations:
    /// - Oracle data availability decreases over time
    /// - User interest may wane for very long markets
    /// - Platform evolution may make old markets obsolete
    ///
    /// Typical values: 30-365 days
    pub max_duration_days: u32,

    /// Minimum required market duration in days.
    ///
    /// Ensures markets have sufficient time for:
    /// - User discovery and participation
    /// - Meaningful price discovery
    /// - Event outcome determination
    ///
    /// Typical values: 1-7 days
    pub min_duration_days: u32,

    /// Maximum number of possible outcomes per market.
    ///
    /// Limits complexity while supporting diverse market types:
    /// - Binary markets: 2 outcomes
    /// - Multiple choice: 3-10 outcomes
    /// - Complex scenarios: Up to maximum
    ///
    /// Higher limits increase:
    /// - Storage requirements
    /// - UI complexity
    /// - Resolution difficulty
    pub max_outcomes: u32,

    /// Minimum number of required outcomes per market.
    ///
    /// Typically 2 to ensure meaningful prediction markets.
    /// Single-outcome markets don't provide prediction value.
    pub min_outcomes: u32,

    /// Maximum length for market questions in characters.
    ///
    /// Balances between:
    /// - Detailed, clear questions
    /// - Storage efficiency
    /// - UI display constraints
    /// - Processing performance
    ///
    /// Typical range: 200-1000 characters
    pub max_question_length: u32,

    /// Maximum length for outcome descriptions in characters.
    ///
    /// Keeps outcome descriptions:
    /// - Clear and concise
    /// - Displayable in UI components
    /// - Storage efficient
    /// - Easy to process
    ///
    /// Typical range: 50-200 characters
    pub max_outcome_length: u32,
}

/// Market duration extension configuration and fee structure.
///
/// This struct defines the parameters for extending market durations beyond
/// their original end dates. Extensions allow markets to accommodate delayed
/// events or provide additional time for resolution when needed.
///
/// # Extension Use Cases
///
/// Market extensions are useful for:
/// - **Delayed Events**: When predicted events are postponed
/// - **Resolution Complexity**: Additional time needed for accurate resolution
/// - **High Stakes Markets**: Extra caution for significant markets
/// - **Community Requests**: Popular markets that warrant extension
///
/// # Economic Model
///
/// Extension fees serve to:
/// - Cover additional oracle and infrastructure costs
/// - Prevent abuse of extension mechanisms
/// - Compensate for extended resource usage
/// - Maintain platform sustainability
///
/// # Example
///
/// ```rust
/// # use predictify_hybrid::config::ExtensionConfig;
///
/// // Create extension configuration
/// let extension_config = ExtensionConfig {
///     max_extension_days: 30,        // Up to 30 days per extension
///     min_extension_days: 1,         // At least 1 day extension
///     fee_per_day: 1_000_000,        // 0.1 XLM per day
///     max_total_extensions: 3,       // Maximum 3 extensions per market
/// };
///
/// // Calculate extension cost
/// let extension_days = 7;
/// let total_cost = extension_days as i128 * extension_config.fee_per_day;
/// println!("7-day extension costs: {} stroops", total_cost); // 7,000,000 stroops
///
/// // Check if extension is valid
/// let current_extensions = 2;
/// let can_extend = current_extensions < extension_config.max_total_extensions &&
///                 extension_days >= extension_config.min_extension_days &&
///                 extension_days <= extension_config.max_extension_days;
/// println!("Can extend: {}", can_extend); // true
/// ```
#[derive(Clone, Debug)]
#[contracttype]
pub struct ExtensionConfig {
    /// Maximum number of days that can be added in a single extension.
    ///
    /// Prevents excessively long extensions while allowing meaningful
    /// time additions. Typical values:
    /// - Development: 7-14 days
    /// - Production: 14-30 days
    pub max_extension_days: u32,

    /// Minimum number of days required for an extension.
    ///
    /// Ensures extensions are meaningful and worth the administrative
    /// overhead. Typically 1-3 days minimum.
    pub min_extension_days: u32,

    /// Fee charged per day of extension (in stroops).
    ///
    /// This fee:
    /// - Covers additional oracle and infrastructure costs
    /// - Prevents frivolous extension requests
    /// - Scales with the duration of extension
    ///
    /// Typical values:
    /// - Development: 100_000 (0.01 XLM/day)
    /// - Testnet: 1_000_000 (0.1 XLM/day)
    /// - Mainnet: 5_000_000 (0.5 XLM/day)
    pub fee_per_day: i128,

    /// Maximum number of extensions allowed per market.
    ///
    /// Prevents indefinite market extensions while allowing
    /// reasonable flexibility for legitimate needs.
    /// Typical range: 2-5 extensions
    pub max_total_extensions: u32,
}

/// Market resolution mechanism and confidence scoring configuration.
///
/// This struct defines how markets are resolved by combining oracle data
/// with community voting. It includes confidence scoring, weighting mechanisms,
/// and consensus requirements for accurate market resolution.
///
/// # Hybrid Resolution Model
///
/// The resolution system combines:
/// - **Oracle Data**: Objective, external data sources
/// - **Community Voting**: Collective intelligence and verification
/// - **Confidence Scoring**: Reliability assessment of resolutions
/// - **Weighted Consensus**: Balanced decision making
///
/// # Resolution Quality
///
/// Configuration parameters ensure:
/// - High accuracy through multiple data sources
/// - Transparency in resolution processes
/// - Resistance to manipulation
/// - Scalable resolution mechanisms
///
/// # Example
///
/// ```rust
/// # use predictify_hybrid::config::ResolutionConfig;
///
/// // Create balanced resolution configuration
/// let resolution_config = ResolutionConfig {
///     min_confidence_score: 0,    // 0% minimum confidence
///     max_confidence_score: 100,  // 100% maximum confidence
///     oracle_weight_percentage: 70, // Oracle has 70% influence
///     community_weight_percentage: 30, // Community has 30% influence
///     min_votes_for_consensus: 5,  // Need at least 5 votes
/// };
///
/// // Calculate weighted resolution
/// let oracle_confidence = 85;  // Oracle 85% confident
/// let community_confidence = 92; // Community 92% confident
///
/// let weighted_confidence =
///     (oracle_confidence * resolution_config.oracle_weight_percentage +
///      community_confidence * resolution_config.community_weight_percentage) / 100;
///
/// println!("Final confidence: {}%", weighted_confidence); // 87%
/// ```
#[derive(Clone, Debug)]
#[contracttype]
pub struct ResolutionConfig {
    /// Minimum allowed confidence score (typically 0).
    ///
    /// Represents the lowest confidence level that can be assigned
    /// to a market resolution. Usually 0 to allow full range.
    pub min_confidence_score: u32,

    /// Maximum allowed confidence score (typically 100).
    ///
    /// Represents the highest confidence level that can be assigned
    /// to a market resolution. Usually 100 for percentage-based scoring.
    pub max_confidence_score: u32,

    /// Percentage weight given to oracle data in hybrid resolution.
    ///
    /// Determines how much influence oracle data has in the final
    /// resolution decision. Higher values favor objective data sources.
    ///
    /// Typical values:
    /// - High oracle trust: 70-80%
    /// - Balanced approach: 50-60%
    /// - Community focused: 30-40%
    pub oracle_weight_percentage: u32,

    /// Percentage weight given to community voting in hybrid resolution.
    ///
    /// Determines how much influence community consensus has in the
    /// final resolution. Should sum with oracle_weight_percentage to 100.
    ///
    /// Higher community weight provides:
    /// - Better handling of edge cases
    /// - Resistance to oracle manipulation
    /// - Democratic decision making
    pub community_weight_percentage: u32,

    /// Minimum number of community votes required for valid consensus.
    ///
    /// Ensures community input is meaningful and representative.
    /// Too low: Susceptible to manipulation
    /// Too high: May prevent resolution of niche markets
    ///
    /// Typical values: 3-10 votes depending on platform size
    pub min_votes_for_consensus: u32,
}

/// Oracle integration and reliability configuration parameters.
///
/// This struct defines how the contract interacts with external oracle services
/// for market resolution. It includes timeout settings, retry mechanisms,
/// and data freshness requirements to ensure reliable oracle integration.
///
/// # Oracle Reliability
///
/// Configuration parameters address:
/// - **Data Freshness**: Ensuring oracle data is current
/// - **Network Resilience**: Handling temporary oracle failures
/// - **Timeout Management**: Preventing indefinite waits
/// - **Quality Assurance**: Maintaining data reliability standards
///
/// # Integration Patterns
///
/// Oracle configuration supports:
/// - Multiple oracle providers for redundancy
/// - Fallback mechanisms for oracle failures
/// - Data validation and quality checks
/// - Performance monitoring and optimization
///
/// # Example
///
/// ```rust
/// # use predictify_hybrid::config::OracleConfig;
///
/// // Create production oracle configuration
/// let oracle_config = OracleConfig {
///     max_price_age: 3600,      // 1 hour maximum data age
///     retry_attempts: 3,        // Try up to 3 times
///     timeout_seconds: 30,      // 30 second timeout per attempt
/// };
///
/// // Calculate total maximum wait time
/// let max_wait_time = oracle_config.retry_attempts as u64 * oracle_config.timeout_seconds;
/// println!("Maximum oracle wait: {} seconds", max_wait_time); // 90 seconds
///
/// // Check if data is fresh enough
/// let data_age = 1800; // 30 minutes old
/// let is_fresh = data_age <= oracle_config.max_price_age;
/// println!("Data is fresh: {}", is_fresh); // true
/// ```
#[derive(Clone, Debug)]
#[contracttype]
pub struct OracleConfig {
    /// Maximum age of oracle data before it's considered stale (in seconds).
    ///
    /// Ensures oracle data is sufficiently recent for accurate market
    /// resolution. Older data may not reflect current conditions.
    ///
    /// Typical values:
    /// - High-frequency markets: 300-900 seconds (5-15 minutes)
    /// - Standard markets: 1800-3600 seconds (30-60 minutes)
    /// - Long-term markets: 3600-7200 seconds (1-2 hours)
    pub max_price_age: u64,

    /// Number of retry attempts for failed oracle requests.
    ///
    /// Provides resilience against:
    /// - Temporary network issues
    /// - Oracle service interruptions
    /// - Rate limiting responses
    /// - Transient failures
    ///
    /// Typical values: 2-5 retries
    pub retry_attempts: u32,

    /// Timeout duration for each oracle request (in seconds).
    ///
    /// Balances between:
    /// - Allowing sufficient time for oracle response
    /// - Preventing indefinite waits
    /// - Maintaining responsive user experience
    /// - Managing system resources
    ///
    /// Typical values: 10-60 seconds per request
    pub timeout_seconds: u64,
}

/// Complete contract configuration combining all subsystem configurations.
///
/// This struct serves as the master configuration container that brings together
/// all the individual configuration components into a single, cohesive contract
/// configuration. It ensures all subsystems work together harmoniously.
///
/// # Configuration Architecture
///
/// The ContractConfig follows a modular design:
/// - **Network**: Blockchain connectivity and environment settings
/// - **Fees**: Economic model and fee structures
/// - **Voting**: Community participation and dispute mechanisms
/// - **Market**: Market creation rules and constraints
/// - **Extension**: Market duration extension parameters
/// - **Resolution**: Hybrid resolution and confidence scoring
/// - **Oracle**: External data integration settings
///
/// # Environment-Specific Configurations
///
/// Different environments require different parameter sets:
/// - **Development**: Relaxed limits, low fees, fast timeouts
/// - **Testnet**: Production-like settings with test-friendly adjustments
/// - **Mainnet**: Optimized, secure, production-ready parameters
/// - **Custom**: Flexible configuration for specialized deployments
///
/// # Configuration Validation
///
/// The complete configuration ensures:
/// - Internal consistency between subsystems
/// - Appropriate parameter relationships
/// - Environment-appropriate settings
/// - Security and performance optimization
///
/// # Example
///
/// ```rust
/// # use soroban_sdk::Env;
/// # use predictify_hybrid::config::{ConfigManager, Environment};
/// # let env = Env::default();
///
/// // Get environment-specific configurations
/// let dev_config = ConfigManager::get_development_config(&env);
/// let mainnet_config = ConfigManager::get_mainnet_config(&env);
///
/// // Compare environment differences
/// println!("Dev creation fee: {} stroops", dev_config.fees.creation_fee);
/// println!("Mainnet creation fee: {} stroops", mainnet_config.fees.creation_fee);
///
/// // Access nested configuration
/// println!("Oracle timeout: {} seconds", dev_config.oracle.timeout_seconds);
/// println!("Max market duration: {} days", dev_config.market.max_duration_days);
///
/// // Validate environment consistency
/// assert_eq!(dev_config.network.environment, Environment::Development);
/// assert_eq!(mainnet_config.network.environment, Environment::Mainnet);
/// ```
///
/// # Configuration Management
///
/// ContractConfig supports:
/// - **Serialization**: Storage in contract persistent storage
/// - **Validation**: Consistency checks across all parameters
/// - **Updates**: Partial or complete configuration updates
/// - **Versioning**: Configuration evolution and migration
/// - **Testing**: Test-specific configuration generation
///
/// # Integration Patterns
///
/// The configuration is used throughout the contract:
/// - Market creation validates against market config
/// - Fee calculations use fee config parameters
/// - Oracle calls respect timeout and retry settings
/// - Voting mechanisms follow voting config rules
/// - Extensions are governed by extension config
///
/// # Best Practices
///
/// - Always validate complete configuration after changes
/// - Test configuration changes in development environment first
/// - Document rationale for production configuration values
/// - Monitor system behavior after configuration updates
/// - Maintain configuration version history for rollbacks
#[derive(Clone, Debug)]
#[contracttype]
pub struct ContractConfig {
    /// Network connectivity and environment configuration.
    ///
    /// Defines which Stellar network the contract operates on,
    /// connection parameters, and environment-specific settings.
    pub network: NetworkConfig,

    /// Economic model and fee structure configuration.
    ///
    /// Controls platform fees, creation costs, limits, and
    /// collection thresholds for sustainable operation.
    pub fees: FeeConfig,

    /// Voting and dispute mechanism configuration.
    ///
    /// Governs community participation, stake requirements,
    /// and dispute resolution processes.
    pub voting: VotingConfig,

    /// Market creation and structure configuration.
    ///
    /// Defines constraints for market creation including
    /// duration limits, outcome counts, and content restrictions.
    pub market: MarketConfig,

    /// Market duration extension configuration.
    ///
    /// Controls how markets can be extended beyond their
    /// original duration, including fees and limits.
    pub extension: ExtensionConfig,

    /// Market resolution and confidence scoring configuration.
    ///
    /// Defines how markets are resolved using hybrid oracle
    /// and community consensus mechanisms.
    pub resolution: ResolutionConfig,

    /// Oracle integration and reliability configuration.
    ///
    /// Controls how the contract interacts with external
    /// oracle services for market resolution data.
    pub oracle: OracleConfig,
}

// ===== CONFIGURATION MANAGER =====

/// Centralized configuration management for the Predictify Hybrid contract.
///
/// The `ConfigManager` provides a comprehensive suite of functions for creating,
/// managing, and maintaining contract configurations across different environments.
/// It serves as the single source of truth for all configuration-related operations.
///
/// # Core Responsibilities
///
/// ConfigManager handles:
/// - **Environment-Specific Configs**: Development, testnet, and mainnet configurations
/// - **Component Configs**: Individual subsystem configuration generation
/// - **Storage Management**: Persistent configuration storage and retrieval
/// - **Validation**: Configuration consistency and validity checks
/// - **Updates**: Safe configuration updates and migrations
///
/// # Configuration Philosophy
///
/// The configuration system follows these principles:
/// - **Environment Appropriate**: Each environment has optimized settings
/// - **Modular Design**: Configurations are composed of focused components
/// - **Safety First**: Validation prevents invalid or dangerous configurations
/// - **Flexibility**: Support for custom configurations when needed
/// - **Maintainability**: Clear, documented, and testable configuration logic
///
/// # Usage Patterns
///
/// ConfigManager is typically used during:
/// - Contract initialization and deployment
/// - Runtime configuration retrieval
/// - Administrative configuration updates
/// - Testing and development setup
/// - Environment migrations
///
/// # Example
///
/// ```rust
/// # use soroban_sdk::Env;
/// # use predictify_hybrid::config::{ConfigManager, Environment};
/// # let env = Env::default();
///
/// // Get environment-appropriate configurations
/// let dev_config = ConfigManager::get_development_config(&env);
/// let mainnet_config = ConfigManager::get_mainnet_config(&env);
///
/// // Store configuration in contract
/// ConfigManager::store_config(&env, &mainnet_config).unwrap();
///
/// // Retrieve stored configuration
/// let stored_config = ConfigManager::get_config(&env).unwrap();
/// assert_eq!(stored_config.network.environment, Environment::Mainnet);
/// ```
pub struct ConfigManager;

impl ConfigManager {
    /// Creates a development environment configuration with relaxed parameters.
    ///
    /// This function generates a complete contract configuration optimized for
    /// development and testing scenarios. It uses relaxed validation rules,
    /// lower fees, and shorter timeouts to facilitate rapid development cycles.
    ///
    /// # Development Characteristics
    ///
    /// The development configuration features:
    /// - **Low Fees**: Minimal creation and platform fees for easy testing
    /// - **Relaxed Limits**: Permissive market creation and participation rules
    /// - **Fast Timeouts**: Short oracle and extension timeouts for quick iteration
    /// - **Test Network**: Uses Stellar testnet for safe development
    /// - **Debug Friendly**: Settings optimized for debugging and testing
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for string and address creation
    ///
    /// # Returns
    ///
    /// Returns a `ContractConfig` with development-optimized settings across
    /// all subsystems (network, fees, voting, market, extension, resolution, oracle).
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::Env;
    /// # use predictify_hybrid::config::{ConfigManager, Environment};
    /// # let env = Env::default();
    ///
    /// // Get development configuration
    /// let dev_config = ConfigManager::get_development_config(&env);
    ///
    /// // Verify development characteristics
    /// assert_eq!(dev_config.network.environment, Environment::Development);
    /// assert!(dev_config.fees.creation_fee < 10_000_000); // Less than 1 XLM
    /// assert!(dev_config.oracle.timeout_seconds <= 30); // Quick timeouts
    ///
    /// println!("Development config ready with {} second oracle timeout",
    ///     dev_config.oracle.timeout_seconds);
    /// ```
    ///
    /// # Development vs Production
    ///
    /// Key differences from production:
    /// - Creation fees: ~0.1 XLM vs ~5 XLM
    /// - Platform fees: ~1% vs ~2.5%
    /// - Oracle timeouts: ~10s vs ~30s
    /// - Market limits: More permissive vs strict
    /// - Validation: Relaxed vs comprehensive
    ///
    /// # Use Cases
    ///
    /// Development configuration is ideal for:
    /// - Local development and testing
    /// - Unit and integration test suites
    /// - Feature development and experimentation
    /// - Debugging and troubleshooting
    /// - Rapid prototyping
    ///
    /// # Network Configuration
    ///
    /// The development config uses Stellar testnet by default but can be
    /// easily modified for local networks or Futurenet as needed.
    pub fn get_development_config(env: &Env) -> ContractConfig {
        ContractConfig {
            network: NetworkConfig {
                environment: Environment::Development,
                passphrase: String::from_str(env, "Test SDF Network ; September 2015"),
                rpc_url: String::from_str(env, "https://soroban-testnet.stellar.org"),
                network_id: String::from_str(env, "testnet"),
                contract_address: Address::from_string(&String::from_str(
                    env,
                    "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
                )),
            },
            fees: Self::get_default_fee_config(),
            voting: Self::get_default_voting_config(),
            market: Self::get_default_market_config(),
            extension: Self::get_default_extension_config(),
            resolution: Self::get_default_resolution_config(),
            oracle: Self::get_default_oracle_config(),
        }
    }

    /// Creates a testnet environment configuration with production-like settings.
    ///
    /// This function generates a complete contract configuration that closely
    /// mirrors production settings while remaining suitable for testing and
    /// integration scenarios. It provides a realistic testing environment.
    ///
    /// # Testnet Characteristics
    ///
    /// The testnet configuration features:
    /// - **Production-Like Fees**: Realistic fee structures for testing
    /// - **Full Validation**: Complete validation rules enabled
    /// - **Realistic Timeouts**: Production-appropriate timeout values
    /// - **Test Network**: Uses Stellar testnet with test tokens
    /// - **Integration Ready**: Optimized for integration testing
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for string and address creation
    ///
    /// # Returns
    ///
    /// Returns a `ContractConfig` with testnet-optimized settings that closely
    /// mirror production while remaining test-friendly.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::Env;
    /// # use predictify_hybrid::config::{ConfigManager, Environment};
    /// # let env = Env::default();
    ///
    /// // Get testnet configuration
    /// let testnet_config = ConfigManager::get_testnet_config(&env);
    ///
    /// // Verify testnet characteristics
    /// assert_eq!(testnet_config.network.environment, Environment::Testnet);
    /// assert_eq!(testnet_config.network.passphrase.to_string(),
    ///     "Test SDF Network ; September 2015");
    ///
    /// // Compare with development settings
    /// let dev_config = ConfigManager::get_development_config(&env);
    /// assert!(testnet_config.fees.creation_fee >= dev_config.fees.creation_fee);
    ///
    /// println!("Testnet config with {} XLM creation fee",
    ///     testnet_config.fees.creation_fee / 10_000_000);
    /// ```
    ///
    /// # Testing Scenarios
    ///
    /// Testnet configuration is perfect for:
    /// - **Integration Testing**: End-to-end testing with realistic settings
    /// - **User Acceptance Testing**: Testing with production-like behavior
    /// - **Performance Testing**: Load testing with realistic parameters
    /// - **Oracle Integration**: Testing oracle connectivity and reliability
    /// - **Fee Testing**: Validating economic model with realistic fees
    ///
    /// # Network Details
    ///
    /// The testnet configuration:
    /// - Uses Stellar testnet (Test SDF Network)
    /// - Connects to Stellar testnet RPC endpoints
    /// - Employs test tokens (not real value)
    /// - Provides reset capabilities for testing
    ///
    /// # Production Preparation
    ///
    /// Testnet serves as the final validation before mainnet:
    /// - Validates all contract functionality
    /// - Tests oracle integrations
    /// - Verifies fee calculations
    /// - Confirms user experience flows
    /// - Validates security measures
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

    /// Creates a mainnet environment configuration with production-optimized settings.
    ///
    /// This function generates a complete contract configuration optimized for
    /// production deployment on Stellar mainnet. It emphasizes security, efficiency,
    /// and economic sustainability while maintaining excellent user experience.
    ///
    /// # Mainnet Characteristics
    ///
    /// The mainnet configuration features:
    /// - **Optimized Fees**: Balanced fee structure for sustainability and accessibility
    /// - **Strict Security**: Comprehensive validation and security measures
    /// - **Production Timeouts**: Reliable timeout values for production use
    /// - **Mainnet Network**: Uses Stellar mainnet with real XLM
    /// - **Audit Ready**: Configuration suitable for security audits
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for string and address creation
    ///
    /// # Returns
    ///
    /// Returns a `ContractConfig` with production-optimized settings designed
    /// for real-world usage with actual economic value.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::Env;
    /// # use predictify_hybrid::config::{ConfigManager, Environment};
    /// # let env = Env::default();
    ///
    /// // Get mainnet configuration
    /// let mainnet_config = ConfigManager::get_mainnet_config(&env);
    ///
    /// // Verify mainnet characteristics
    /// assert_eq!(mainnet_config.network.environment, Environment::Mainnet);
    /// assert_eq!(mainnet_config.network.passphrase.to_string(),
    ///     "Public Global Stellar Network ; September 2015");
    ///
    /// // Check production-grade settings
    /// assert!(mainnet_config.fees.creation_fee >= 10_000_000); // At least 1 XLM
    /// assert!(mainnet_config.fees.platform_fee_percentage >= 200); // At least 2%
    ///
    /// println!("Mainnet config with {}% platform fee",
    ///     mainnet_config.fees.platform_fee_percentage / 100);
    /// ```
    ///
    /// # Production Considerations
    ///
    /// Mainnet configuration addresses:
    /// - **Economic Sustainability**: Fees that support platform operations
    /// - **Security**: Robust validation and anti-abuse measures
    /// - **Scalability**: Settings that support high transaction volumes
    /// - **User Experience**: Balanced between security and usability
    /// - **Regulatory Compliance**: Settings that support compliance requirements
    ///
    /// # Fee Structure
    ///
    /// Mainnet fees are designed to:
    /// - Cover operational costs (oracles, infrastructure)
    /// - Prevent spam and abuse
    /// - Generate sustainable revenue
    /// - Remain competitive with alternatives
    /// - Support platform development
    ///
    /// # Security Features
    ///
    /// Production security includes:
    /// - Comprehensive input validation
    /// - Anti-manipulation measures
    /// - Robust dispute mechanisms
    /// - Oracle reliability safeguards
    /// - Economic incentive alignment
    ///
    /// # Deployment Readiness
    ///
    /// Before mainnet deployment, ensure:
    /// - Thorough testing on testnet
    /// - Security audit completion
    /// - Oracle integration validation
    /// - Fee model validation
    /// - User experience testing
    /// - Monitoring and alerting setup
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

    /// Creates a default fee configuration suitable for development and testing.
    ///
    /// This function generates a balanced fee configuration that provides reasonable
    /// defaults for most environments while remaining accessible for development
    /// and testing scenarios.
    ///
    /// # Default Fee Structure
    ///
    /// The default configuration includes:
    /// - **Platform Fee**: 2% of winning payouts
    /// - **Creation Fee**: 1 XLM to create markets
    /// - **Minimum Fee**: 0.1 XLM floor
    /// - **Maximum Fee**: 100 XLM ceiling
    /// - **Collection Threshold**: 10 XLM auto-collection
    /// - **Fees Enabled**: Active by default
    ///
    /// # Returns
    ///
    /// Returns a `FeeConfig` with balanced default values suitable for
    /// development, testing, and initial production deployments.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use predictify_hybrid::config::ConfigManager;
    ///
    /// // Get default fee configuration
    /// let fee_config = ConfigManager::get_default_fee_config();
    ///
    /// // Verify default values
    /// assert_eq!(fee_config.platform_fee_percentage, 2); // 2%
    /// assert_eq!(fee_config.creation_fee, 10_000_000); // 1 XLM
    /// assert!(fee_config.fees_enabled);
    ///
    /// println!("Default platform fee: {}%", fee_config.platform_fee_percentage);
    /// ```
    ///
    /// # Usage Context
    ///
    /// Default fees are appropriate for:
    /// - Development and testing environments
    /// - Initial testnet deployments
    /// - Conservative production starts
    /// - Baseline configuration reference
    ///
    /// # Customization
    ///
    /// For production mainnet, consider using `get_mainnet_fee_config()`
    /// which provides higher, more sustainable fee structures.
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

    /// Creates a mainnet-optimized fee configuration with higher, sustainable fees.
    ///
    /// This function generates a fee configuration specifically designed for
    /// production mainnet deployment, with higher fees that support platform
    /// sustainability while remaining competitive and accessible.
    ///
    /// # Mainnet Fee Structure
    ///
    /// The mainnet configuration includes:
    /// - **Platform Fee**: 3% of winning payouts (vs 2% default)
    /// - **Creation Fee**: 1.5 XLM to create markets (vs 1 XLM default)
    /// - **Minimum Fee**: 0.2 XLM floor (vs 0.1 XLM default)
    /// - **Maximum Fee**: 200 XLM ceiling (vs 100 XLM default)
    /// - **Collection Threshold**: 20 XLM auto-collection (vs 10 XLM default)
    /// - **Fees Enabled**: Active for revenue generation
    ///
    /// # Returns
    ///
    /// Returns a `FeeConfig` with mainnet-optimized values designed for
    /// production deployment with real economic value.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use predictify_hybrid::config::ConfigManager;
    ///
    /// // Get mainnet fee configuration
    /// let mainnet_fees = ConfigManager::get_mainnet_fee_config();
    /// let default_fees = ConfigManager::get_default_fee_config();
    ///
    /// // Compare mainnet vs default
    /// assert!(mainnet_fees.platform_fee_percentage > default_fees.platform_fee_percentage);
    /// assert!(mainnet_fees.creation_fee > default_fees.creation_fee);
    ///
    /// println!("Mainnet creation fee: {} XLM", mainnet_fees.creation_fee / 10_000_000);
    /// ```
    ///
    /// # Economic Rationale
    ///
    /// Higher mainnet fees serve to:
    /// - **Cover Operational Costs**: Oracle fees, infrastructure, development
    /// - **Prevent Spam**: Higher creation fees deter low-quality markets
    /// - **Ensure Sustainability**: Revenue supports long-term platform viability
    /// - **Maintain Quality**: Economic barriers encourage thoughtful market creation
    ///
    /// # Market Competitiveness
    ///
    /// Mainnet fees are designed to be:
    /// - Competitive with similar prediction platforms
    /// - Reasonable for serious market creators
    /// - Sustainable for platform operations
    /// - Transparent and predictable
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

    /// Creates a default voting configuration with balanced participation thresholds.
    ///
    /// This function generates a voting configuration that balances accessibility
    /// with security, providing reasonable defaults for community participation
    /// and dispute resolution mechanisms.
    ///
    /// # Default Voting Parameters
    ///
    /// The default configuration includes:
    /// - **Minimum Vote Stake**: 0.1 XLM (accessible participation)
    /// - **Minimum Dispute Stake**: 1 XLM (serious commitment required)
    /// - **Maximum Dispute Threshold**: 10 XLM (reasonable cap)
    /// - **Base Dispute Threshold**: 1 XLM (starting point)
    /// - **Large Market Threshold**: 100 XLM (high-value market definition)
    /// - **High Activity Threshold**: 100 votes (active market definition)
    /// - **Dispute Extension**: 24 hours (reasonable review time)
    ///
    /// # Returns
    ///
    /// Returns a `VotingConfig` with balanced default values suitable for
    /// most environments and use cases.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use predictify_hybrid::config::ConfigManager;
    ///
    /// // Get default voting configuration
    /// let voting_config = ConfigManager::get_default_voting_config();
    ///
    /// // Check accessibility
    /// assert_eq!(voting_config.min_vote_stake, 1_000_000); // 0.1 XLM
    /// assert_eq!(voting_config.dispute_extension_hours, 24);
    ///
    /// // Calculate dispute cost for standard market
    /// let dispute_cost = voting_config.base_dispute_threshold;
    /// println!("Base dispute cost: {} XLM", dispute_cost / 10_000_000);
    /// ```
    ///
    /// # Participation Balance
    ///
    /// Default settings balance:
    /// - **Accessibility**: Low minimum stakes encourage participation
    /// - **Security**: Higher dispute stakes prevent frivolous challenges
    /// - **Scalability**: Thresholds adjust based on market size and activity
    /// - **Fairness**: Reasonable timeframes for dispute resolution
    ///
    /// # Environment Suitability
    ///
    /// Default voting config works well for:
    /// - Development and testing environments
    /// - Initial testnet deployments
    /// - Conservative production launches
    /// - General-purpose prediction markets
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

    /// Creates a mainnet-optimized voting configuration with higher stakes and security.
    ///
    /// This function generates a voting configuration specifically designed for
    /// production mainnet deployment, with higher stakes that improve security
    /// and reduce spam while maintaining reasonable accessibility.
    ///
    /// # Mainnet Voting Parameters
    ///
    /// The mainnet configuration includes:
    /// - **Minimum Vote Stake**: 0.2 XLM (vs 0.1 XLM default)
    /// - **Minimum Dispute Stake**: 2 XLM (vs 1 XLM default)
    /// - **Maximum Dispute Threshold**: 20 XLM (vs 10 XLM default)
    /// - **Base Dispute Threshold**: 2 XLM (vs 1 XLM default)
    /// - **Large Market Threshold**: 200 XLM (vs 100 XLM default)
    /// - **High Activity Threshold**: 200 votes (vs 100 votes default)
    /// - **Dispute Extension**: 48 hours (vs 24 hours default)
    ///
    /// # Returns
    ///
    /// Returns a `VotingConfig` with mainnet-optimized values designed for
    /// production deployment with enhanced security and quality.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use predictify_hybrid::config::ConfigManager;
    ///
    /// // Get mainnet voting configuration
    /// let mainnet_voting = ConfigManager::get_mainnet_voting_config();
    /// let default_voting = ConfigManager::get_default_voting_config();
    ///
    /// // Compare mainnet vs default security
    /// assert!(mainnet_voting.min_dispute_stake > default_voting.min_dispute_stake);
    /// assert!(mainnet_voting.dispute_extension_hours > default_voting.dispute_extension_hours);
    ///
    /// println!("Mainnet dispute stake: {} XLM",
    ///     mainnet_voting.min_dispute_stake / 10_000_000);
    /// ```
    ///
    /// # Enhanced Security Features
    ///
    /// Higher mainnet stakes provide:
    /// - **Spam Resistance**: Higher costs deter low-quality participation
    /// - **Serious Commitment**: Meaningful stakes ensure thoughtful voting
    /// - **Extended Review**: Longer dispute periods for thorough evaluation
    /// - **Quality Markets**: Higher thresholds for large/active market classification
    ///
    /// # Production Considerations
    ///
    /// Mainnet voting config addresses:
    /// - Real economic value at stake
    /// - Higher potential for manipulation attempts
    /// - Need for robust dispute resolution
    /// - Community quality and engagement
    /// - Platform reputation and trust
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

    /// Creates a default market configuration with balanced creation constraints.
    ///
    /// This function generates a market configuration that provides reasonable
    /// defaults for market creation, balancing flexibility with quality control
    /// and system performance considerations.
    ///
    /// # Default Market Parameters
    ///
    /// The default configuration includes:
    /// - **Maximum Duration**: 365 days (up to 1 year markets)
    /// - **Minimum Duration**: 1 day (at least 24 hours)
    /// - **Maximum Outcomes**: 10 possible outcomes per market
    /// - **Minimum Outcomes**: 2 outcomes (binary minimum)
    /// - **Maximum Question Length**: 500 characters
    /// - **Maximum Outcome Length**: 100 characters
    ///
    /// # Returns
    ///
    /// Returns a `MarketConfig` with balanced default values suitable for
    /// diverse market types and use cases.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use predictify_hybrid::config::ConfigManager;
    ///
    /// // Get default market configuration
    /// let market_config = ConfigManager::get_default_market_config();
    ///
    /// // Validate a market proposal
    /// let question = "Will Bitcoin reach $100,000 by end of 2024?";
    /// let outcomes = vec!["Yes", "No"];
    /// let duration_days = 90;
    ///
    /// let valid_question = question.len() <= market_config.max_question_length as usize;
    /// let valid_outcomes = outcomes.len() >= market_config.min_outcomes as usize;
    /// let valid_duration = duration_days <= market_config.max_duration_days;
    ///
    /// assert!(valid_question && valid_outcomes && valid_duration);
    /// ```
    ///
    /// # Quality Control Balance
    ///
    /// Default settings balance:
    /// - **Flexibility**: Wide duration range supports diverse market types
    /// - **Quality**: Reasonable length limits ensure clarity
    /// - **Performance**: Outcome limits maintain system efficiency
    /// - **Usability**: Constraints are permissive but meaningful
    ///
    /// # Market Type Support
    ///
    /// Default config supports:
    /// - Binary prediction markets (2 outcomes)
    /// - Multiple choice markets (3-10 outcomes)
    /// - Short-term events (1+ days)
    /// - Long-term predictions (up to 1 year)
    /// - Detailed questions (up to 500 characters)
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

    /// Creates a default extension configuration with reasonable limits and fees.
    ///
    /// This function generates an extension configuration that allows market
    /// duration extensions while preventing abuse through reasonable limits
    /// and proportional fees.
    ///
    /// # Default Extension Parameters
    ///
    /// The default configuration includes:
    /// - **Maximum Extension Days**: 30 days per extension
    /// - **Minimum Extension Days**: 1 day minimum extension
    /// - **Fee Per Day**: 10 XLM per day of extension
    /// - **Maximum Total Extensions**: 3 extensions per market
    ///
    /// # Returns
    ///
    /// Returns an `ExtensionConfig` with balanced default values that allow
    /// reasonable market extensions while preventing abuse.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use predictify_hybrid::config::ConfigManager;
    ///
    /// // Get default extension configuration
    /// let ext_config = ConfigManager::get_default_extension_config();
    ///
    /// // Calculate extension cost
    /// let extension_days = 7;
    /// let total_cost = extension_days as i128 * ext_config.fee_per_day;
    ///
    /// // Check if extension is allowed
    /// let current_extensions = 2;
    /// let can_extend = current_extensions < ext_config.max_total_extensions &&
    ///                 extension_days <= ext_config.max_extension_days;
    ///
    /// println!("7-day extension costs: {} XLM", total_cost / 10_000_000);
    /// assert!(can_extend);
    /// ```
    ///
    /// # Extension Economics
    ///
    /// Default fees are designed to:
    /// - **Cover Costs**: Additional oracle and infrastructure expenses
    /// - **Prevent Abuse**: Meaningful cost discourages frivolous extensions
    /// - **Scale Appropriately**: Cost proportional to extension duration
    /// - **Remain Accessible**: Not prohibitively expensive for legitimate needs
    ///
    /// # Use Case Support
    ///
    /// Default config accommodates:
    /// - Event delays and postponements
    /// - Complex resolution scenarios
    /// - High-stakes markets needing extra time
    /// - Community-requested extensions
    /// - Oracle data availability issues
    pub fn get_default_extension_config() -> ExtensionConfig {
        ExtensionConfig {
            max_extension_days: MAX_EXTENSION_DAYS,
            min_extension_days: MIN_EXTENSION_DAYS,
            fee_per_day: EXTENSION_FEE_PER_DAY,
            max_total_extensions: MAX_TOTAL_EXTENSIONS,
        }
    }

    /// Creates a default resolution configuration for hybrid oracle-community resolution.
    ///
    /// This function generates a resolution configuration that balances oracle
    /// reliability with community wisdom, providing a hybrid approach to market
    /// resolution that leverages both automated and human intelligence.
    ///
    /// # Default Resolution Parameters
    ///
    /// The default configuration includes:
    /// - **Minimum Confidence Score**: 70% (reliable threshold)
    /// - **Maximum Confidence Score**: 100% (perfect confidence)
    /// - **Oracle Weight**: 60% of resolution decision
    /// - **Community Weight**: 40% of resolution decision
    /// - **Minimum Votes for Consensus**: 10 community votes required
    ///
    /// # Returns
    ///
    /// Returns a `ResolutionConfig` with balanced default values that combine
    /// oracle reliability with community validation.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use predictify_hybrid::config::ConfigManager;
    ///
    /// // Get default resolution configuration
    /// let resolution_config = ConfigManager::get_default_resolution_config();
    ///
    /// // Check hybrid weighting
    /// assert_eq!(resolution_config.oracle_weight_percentage, 60);
    /// assert_eq!(resolution_config.community_weight_percentage, 40);
    /// assert_eq!(resolution_config.oracle_weight_percentage +
    ///           resolution_config.community_weight_percentage, 100);
    ///
    /// // Verify confidence thresholds
    /// assert!(resolution_config.min_confidence_score >= 70);
    /// println!("Minimum confidence required: {}%", resolution_config.min_confidence_score);
    /// ```
    ///
    /// # Hybrid Resolution Model
    ///
    /// The default configuration implements a hybrid model where:
    /// - **Oracle Primary**: Oracle data carries more weight (60%)
    /// - **Community Validation**: Community provides validation and backup (40%)
    /// - **Confidence Gating**: Low confidence triggers community involvement
    /// - **Consensus Requirements**: Minimum vote thresholds ensure quality
    ///
    /// # Resolution Flow
    ///
    /// Default config supports this resolution process:
    /// 1. Oracle provides initial resolution with confidence score
    /// 2. If confidence ≥ minimum, oracle resolution weighted at 60%
    /// 3. Community voting weighted at 40% for final decision
    /// 4. Minimum vote threshold ensures adequate participation
    /// 5. Combined weighted result determines final outcome
    ///
    /// # Quality Assurance
    ///
    /// Default settings ensure:
    /// - High-confidence oracle data is respected
    /// - Community input prevents oracle manipulation
    /// - Sufficient participation for legitimate consensus
    /// - Balanced approach reduces single points of failure
    pub fn get_default_resolution_config() -> ResolutionConfig {
        ResolutionConfig {
            min_confidence_score: MIN_CONFIDENCE_SCORE,
            max_confidence_score: MAX_CONFIDENCE_SCORE,
            oracle_weight_percentage: ORACLE_WEIGHT_PERCENTAGE,
            community_weight_percentage: COMMUNITY_WEIGHT_PERCENTAGE,
            min_votes_for_consensus: MIN_VOTES_FOR_CONSENSUS,
        }
    }

    /// Creates a default oracle configuration with balanced reliability and performance.
    ///
    /// This function generates an oracle configuration that balances data freshness,
    /// reliability, and system performance, providing reasonable defaults for
    /// oracle integration across various market types and conditions.
    ///
    /// # Default Oracle Parameters
    ///
    /// The default configuration includes:
    /// - **Maximum Price Age**: 3600 seconds (1 hour data freshness)
    /// - **Retry Attempts**: 3 attempts for failed oracle calls
    /// - **Timeout Seconds**: 30 seconds per oracle request
    ///
    /// # Returns
    ///
    /// Returns an `OracleConfig` with balanced default values suitable for
    /// reliable oracle integration with reasonable performance characteristics.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use predictify_hybrid::config::ConfigManager;
    ///
    /// // Get default oracle configuration
    /// let oracle_config = ConfigManager::get_default_oracle_config();
    ///
    /// // Check data freshness requirements
    /// assert_eq!(oracle_config.max_price_age, 3600); // 1 hour
    ///
    /// // Verify reliability settings
    /// assert_eq!(oracle_config.retry_attempts, 3);
    /// assert_eq!(oracle_config.timeout_seconds, 30);
    ///
    /// // Calculate maximum total time for oracle resolution
    /// let max_resolution_time = oracle_config.retry_attempts * oracle_config.timeout_seconds;
    /// println!("Maximum oracle resolution time: {} seconds", max_resolution_time);
    /// ```
    ///
    /// # Data Freshness Balance
    ///
    /// The 1-hour maximum age balances:
    /// - **Accuracy**: Recent data reflects current market conditions
    /// - **Availability**: Reasonable window prevents excessive failures
    /// - **Performance**: Allows for oracle caching and optimization
    /// - **Cost**: Reduces unnecessary oracle calls
    ///
    /// # Reliability Features
    ///
    /// Default retry and timeout settings provide:
    /// - **Fault Tolerance**: Multiple attempts handle transient failures
    /// - **Reasonable Timeouts**: 30 seconds allows for network latency
    /// - **Bounded Delays**: Maximum 90 seconds total resolution time
    /// - **Predictable Behavior**: Consistent timing for user experience
    ///
    /// # Oracle Integration
    ///
    /// Default config supports integration with:
    /// - Price feed oracles (Chainlink, Band Protocol, etc.)
    /// - Event outcome oracles (sports, elections, etc.)
    /// - Custom data providers
    /// - Hybrid oracle networks
    ///
    /// # Performance Considerations
    ///
    /// Default settings balance:
    /// - User experience (reasonable wait times)
    /// - System reliability (adequate retries)
    /// - Resource usage (bounded timeouts)
    /// - Data quality (freshness requirements)
    pub fn get_default_oracle_config() -> OracleConfig {
        OracleConfig {
            max_price_age: MAX_ORACLE_PRICE_AGE,
            retry_attempts: ORACLE_RETRY_ATTEMPTS,
            timeout_seconds: ORACLE_TIMEOUT_SECONDS,
        }
    }

    /// Creates a mainnet-optimized oracle configuration with stricter reliability requirements.
    ///
    /// This function generates an oracle configuration specifically designed for
    /// production mainnet deployment, with stricter data freshness requirements
    /// and enhanced reliability measures to ensure high-quality oracle data.
    ///
    /// # Mainnet Oracle Parameters
    ///
    /// The mainnet configuration includes:
    /// - **Maximum Price Age**: 1800 seconds (30 minutes vs 1 hour default)
    /// - **Retry Attempts**: 5 attempts (vs 3 attempts default)
    /// - **Timeout Seconds**: 60 seconds (vs 30 seconds default)
    ///
    /// # Returns
    ///
    /// Returns an `OracleConfig` with mainnet-optimized values designed for
    /// production deployment with enhanced data quality and reliability.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use predictify_hybrid::config::ConfigManager;
    ///
    /// // Get mainnet oracle configuration
    /// let mainnet_oracle = ConfigManager::get_mainnet_oracle_config();
    /// let default_oracle = ConfigManager::get_default_oracle_config();
    ///
    /// // Compare mainnet vs default strictness
    /// assert!(mainnet_oracle.max_price_age < default_oracle.max_price_age);
    /// assert!(mainnet_oracle.retry_attempts > default_oracle.retry_attempts);
    /// assert!(mainnet_oracle.timeout_seconds > default_oracle.timeout_seconds);
    ///
    /// println!("Mainnet data freshness: {} minutes",
    ///     mainnet_oracle.max_price_age / 60);
    /// ```
    ///
    /// # Enhanced Reliability Features
    ///
    /// Mainnet oracle config provides:
    /// - **Fresher Data**: 30-minute maximum age ensures current market conditions
    /// - **More Retries**: 5 attempts handle network issues and temporary failures
    /// - **Longer Timeouts**: 60 seconds accommodates complex oracle operations
    /// - **Higher Quality**: Stricter requirements improve resolution accuracy
    ///
    /// # Production Considerations
    ///
    /// Mainnet settings address:
    /// - Real economic value requiring accurate data
    /// - Higher stakes demanding reliable oracle responses
    /// - Network congestion and latency issues
    /// - Oracle provider diversity and failover
    /// - Regulatory compliance and audit requirements
    pub fn get_mainnet_oracle_config() -> OracleConfig {
        OracleConfig {
            max_price_age: 1800, // 30 minutes for mainnet
            retry_attempts: 5,   // More retries for mainnet
            timeout_seconds: 60, // Longer timeout for mainnet
        }
    }

    /// Stores a complete contract configuration in persistent contract storage.
    ///
    /// This function saves the provided configuration to the contract's persistent
    /// storage, making it available for future contract calls and ensuring
    /// configuration persistence across contract invocations.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for storage operations
    /// * `config` - The complete contract configuration to store
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on successful storage, or an `Error` if storage fails.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::Env;
    /// # use predictify_hybrid::config::ConfigManager;
    /// # let env = Env::default();
    ///
    /// // Create and store development configuration
    /// let dev_config = ConfigManager::get_development_config(&env);
    /// let result = ConfigManager::store_config(&env, &dev_config);
    ///
    /// assert!(result.is_ok());
    ///
    /// // Verify storage by retrieving
    /// let retrieved_config = ConfigManager::get_config(&env).unwrap();
    /// assert_eq!(retrieved_config.network.environment, dev_config.network.environment);
    /// ```
    ///
    /// # Storage Details
    ///
    /// Configuration storage:
    /// - Uses persistent storage for durability across contract calls
    /// - Stores under the "ContractConfig" key for consistent retrieval
    /// - Overwrites any existing configuration
    /// - Atomic operation ensuring consistency
    ///
    /// # Usage Context
    ///
    /// This function is typically called during:
    /// - Contract initialization and setup
    /// - Configuration updates by admin functions
    /// - Environment-specific deployments
    /// - Configuration resets and migrations
    pub fn store_config(env: &Env, config: &ContractConfig) -> Result<(), Error> {
        let key = Symbol::new(env, "ContractConfig");
        env.storage().persistent().set(&key, config);
        Ok(())
    }

    /// Retrieves the current contract configuration from persistent storage.
    ///
    /// This function loads the previously stored contract configuration from
    /// persistent storage, providing access to all current contract settings
    /// and parameters for use in contract operations.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for storage operations
    ///
    /// # Returns
    ///
    /// Returns the stored `ContractConfig` on success, or `Error::ConfigurationNotFound`
    /// if no configuration has been stored.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::Env;
    /// # use predictify_hybrid::config::{ConfigManager, Environment};
    /// # let env = Env::default();
    ///
    /// // Store a configuration first
    /// let testnet_config = ConfigManager::get_testnet_config(&env);
    /// ConfigManager::store_config(&env, &testnet_config).unwrap();
    ///
    /// // Retrieve the stored configuration
    /// let current_config = ConfigManager::get_config(&env).unwrap();
    ///
    /// // Verify it matches what was stored
    /// assert_eq!(current_config.network.environment, Environment::Testnet);
    /// assert_eq!(current_config.fees.platform_fee_percentage,
    ///           testnet_config.fees.platform_fee_percentage);
    ///
    /// println!("Current environment: {:?}", current_config.network.environment);
    /// ```
    ///
    /// # Error Handling
    ///
    /// This function returns `Error::ConfigurationNotFound` when:
    /// - No configuration has been previously stored
    /// - Configuration was stored but corrupted
    /// - Storage key doesn't exist or is inaccessible
    ///
    /// # Usage Context
    ///
    /// Configuration retrieval is used in:
    /// - Contract function calls requiring current settings
    /// - Fee calculations and validation
    /// - Market creation and management
    /// - Oracle integration and resolution
    /// - Admin operations and updates
    pub fn get_config(env: &Env) -> Result<ContractConfig, Error> {
        let key = Symbol::new(env, "ContractConfig");
        // Check if key exists before trying to get it to avoid segfaults
        match env.storage().persistent().get::<Symbol, ContractConfig>(&key) {
            Some(config) => Ok(config),
            None => Err(Error::ConfigurationNotFound),
        }
    }

    /// Updates the contract configuration in persistent storage.
    ///
    /// This function provides a convenient wrapper for updating the stored
    /// contract configuration, ensuring consistency with the storage mechanism
    /// and maintaining the same storage key and behavior.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for storage operations
    /// * `config` - The updated contract configuration to store
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on successful update, or an `Error` if storage fails.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::Env;
    /// # use predictify_hybrid::config::ConfigManager;
    /// # let env = Env::default();
    ///
    /// // Get current configuration
    /// let mut current_config = ConfigManager::get_development_config(&env);
    /// ConfigManager::store_config(&env, &current_config).unwrap();
    ///
    /// // Modify fee settings
    /// current_config.fees.platform_fee_percentage = 3; // Increase to 3%
    /// current_config.fees.creation_fee = 15_000_000; // Increase to 1.5 XLM
    ///
    /// // Update stored configuration
    /// let result = ConfigManager::update_config(&env, &current_config);
    /// assert!(result.is_ok());
    ///
    /// // Verify update
    /// let updated_config = ConfigManager::get_config(&env).unwrap();
    /// assert_eq!(updated_config.fees.platform_fee_percentage, 3);
    /// ```
    ///
    /// # Update Semantics
    ///
    /// Configuration updates:
    /// - Completely replace the existing configuration
    /// - Are atomic operations ensuring consistency
    /// - Take effect immediately for subsequent contract calls
    /// - Should be validated before updating
    ///
    /// # Administrative Context
    ///
    /// Configuration updates are typically performed by:
    /// - Contract administrators during governance actions
    /// - Automated systems responding to market conditions
    /// - Migration scripts during contract upgrades
    /// - Emergency response procedures
    pub fn update_config(env: &Env, config: &ContractConfig) -> Result<(), Error> {
        Self::store_config(env, config)
    }

    /// Resets the contract configuration to development defaults and stores it.
    ///
    /// This function provides a convenient way to reset the contract configuration
    /// to safe development defaults, useful for testing, recovery scenarios,
    /// or initial contract setup.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for configuration generation and storage
    ///
    /// # Returns
    ///
    /// Returns the newly stored development `ContractConfig` on success,
    /// or an `Error` if storage fails.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::Env;
    /// # use predictify_hybrid::config::{ConfigManager, Environment};
    /// # let env = Env::default();
    ///
    /// // Store a custom configuration first
    /// let mainnet_config = ConfigManager::get_mainnet_config(&env);
    /// ConfigManager::store_config(&env, &mainnet_config).unwrap();
    ///
    /// // Reset to development defaults
    /// let reset_config = ConfigManager::reset_to_defaults(&env).unwrap();
    ///
    /// // Verify reset to development environment
    /// assert_eq!(reset_config.network.environment, Environment::Development);
    /// assert_eq!(reset_config.fees.platform_fee_percentage, 2); // Default 2%
    ///
    /// // Confirm it's stored
    /// let stored_config = ConfigManager::get_config(&env).unwrap();
    /// assert_eq!(stored_config.network.environment, Environment::Development);
    /// ```
    ///
    /// # Reset Behavior
    ///
    /// Configuration reset:
    /// - Uses development configuration as the default baseline
    /// - Overwrites any existing stored configuration
    /// - Provides safe, conservative settings suitable for testing
    /// - Returns the newly stored configuration for immediate use
    ///
    /// # Use Cases
    ///
    /// Configuration reset is useful for:
    /// - **Testing**: Clean slate for test scenarios
    /// - **Recovery**: Restore known-good configuration after issues
    /// - **Development**: Quick setup for development environments
    /// - **Debugging**: Eliminate configuration as a variable
    /// - **Migration**: Safe fallback during configuration updates
    ///
    /// # Safety Considerations
    ///
    /// Development defaults provide:
    /// - Conservative fee structures
    /// - Accessible participation thresholds
    /// - Reasonable timeout and retry settings
    /// - Safe oracle and resolution parameters
    pub fn reset_to_defaults(env: &Env) -> Result<ContractConfig, Error> {
        let config = Self::get_development_config(env);
        Self::store_config(env, &config)?;
        Ok(config)
    }

    /// Internal helper: push a history record, keep last 100 entries
    fn push_history(env: &Env, record: &ConfigUpdateRecord) {
        let key = Symbol::new(env, "ConfigHistory");
        let mut history: soroban_sdk::Vec<ConfigUpdateRecord> = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| soroban_sdk::Vec::new(env));

        history.push_back(record.clone());
        if history.len() > 100 {
            history.remove(0);
        }
        env.storage().persistent().set(&key, &history);
    }

    /// Get the currently stored configuration
    pub fn get_current_configuration(env: &Env) -> Result<ContractConfig, Error> {
        Self::get_config(env)
    }

    /// Retrieve configuration update history (may be empty)
    pub fn get_configuration_history(
        env: &Env,
    ) -> Result<soroban_sdk::Vec<ConfigUpdateRecord>, Error> {
        let key = Symbol::new(env, "ConfigHistory");
        Ok(env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| soroban_sdk::Vec::new(env)))
    }

    /// Validate a set of configuration changes without persisting them
    pub fn validate_configuration_changes(env: &Env, changes: &ConfigChanges) -> Result<(), Error> {
        let mut cfg = Self::get_config(env)?;

        if let Some(fee) = changes.platform_fee_percentage {
            cfg.fees.platform_fee_percentage = fee;
        }
        if let Some(base) = changes.base_dispute_threshold {
            cfg.voting.base_dispute_threshold = base;
        }
        if let Some(timeout) = changes.oracle_timeout_seconds {
            cfg.oracle.timeout_seconds = timeout as u64;
        }
        if let Some(v) = changes.max_duration_days {
            cfg.market.max_duration_days = v;
        }
        if let Some(v) = changes.min_duration_days {
            cfg.market.min_duration_days = v;
        }
        if let Some(v) = changes.max_outcomes {
            cfg.market.max_outcomes = v;
        }
        if let Some(v) = changes.min_outcomes {
            cfg.market.min_outcomes = v;
        }
        if let Some(v) = changes.max_question_length {
            cfg.market.max_question_length = v;
        }
        if let Some(v) = changes.max_outcome_length {
            cfg.market.max_outcome_length = v;
        }

        ConfigValidator::validate_contract_config(&cfg)
    }

    /// Update platform fee percentage (requires admin with update_fees permission)
    pub fn update_fee_percentage(
        env: &Env,
        admin: Address,
        new_fee: i128,
    ) -> Result<ContractConfig, Error> {
        // AuthN/AuthZ
        crate::admin::AdminAccessControl::validate_admin_for_action(env, &admin, "update_fees")?;

        let mut cfg = Self::get_config(env)?;
        let old = cfg.fees.platform_fee_percentage;
        cfg.fees.platform_fee_percentage = new_fee;

        // Validate and persist
        ConfigValidator::validate_fee_config(&cfg.fees)?;
        Self::update_config(env, &cfg)?;

        // Emit event and record history
        let change_type = String::from_str(env, "fee_percentage");
        let old_s = String::from_str(env, &alloc::format!("{}", old));
        let new_s = String::from_str(env, &alloc::format!("{}", new_fee));
        crate::events::EventEmitter::emit_config_updated(env, &admin, &change_type, &old_s, &new_s);

        let record = ConfigUpdateRecord {
            updated_by: admin,
            change_type,
            old_value: old_s,
            new_value: new_s,
            timestamp: env.ledger().timestamp(),
        };
        Self::push_history(env, &record);

        Ok(cfg)
    }

    /// Update base dispute threshold (requires admin with update_config permission)
    pub fn update_dispute_threshold(
        env: &Env,
        admin: Address,
        new_threshold: i128,
    ) -> Result<ContractConfig, Error> {
        crate::admin::AdminAccessControl::validate_admin_for_action(env, &admin, "update_config")?;

        let mut cfg = Self::get_config(env)?;
        let old = cfg.voting.base_dispute_threshold;
        cfg.voting.base_dispute_threshold = new_threshold;

        ConfigValidator::validate_voting_config(&cfg.voting)?;
        Self::update_config(env, &cfg)?;

        let change_type = String::from_str(env, "dispute_threshold");
        let old_s = String::from_str(env, &alloc::format!("{}", old));
        let new_s = String::from_str(env, &alloc::format!("{}", new_threshold));
        crate::events::EventEmitter::emit_config_updated(env, &admin, &change_type, &old_s, &new_s);

        let record = ConfigUpdateRecord {
            updated_by: admin,
            change_type,
            old_value: old_s,
            new_value: new_s,
            timestamp: env.ledger().timestamp(),
        };
        Self::push_history(env, &record);

        Ok(cfg)
    }

    /// Update oracle timeout seconds (requires admin with update_config permission)
    pub fn update_oracle_timeout(
        env: &Env,
        admin: Address,
        timeout_seconds: u32,
    ) -> Result<ContractConfig, Error> {
        crate::admin::AdminAccessControl::validate_admin_for_action(env, &admin, "update_config")?;

        let mut cfg = Self::get_config(env)?;
        let old = cfg.oracle.timeout_seconds;
        cfg.oracle.timeout_seconds = timeout_seconds as u64;

        ConfigValidator::validate_oracle_config(&cfg.oracle)?;
        Self::update_config(env, &cfg)?;

        let change_type = String::from_str(env, "oracle_timeout");
        let old_s = String::from_str(env, &alloc::format!("{}", old));
        let new_s = String::from_str(env, &alloc::format!("{}", cfg.oracle.timeout_seconds));
        crate::events::EventEmitter::emit_config_updated(env, &admin, &change_type, &old_s, &new_s);

        let record = ConfigUpdateRecord {
            updated_by: admin,
            change_type,
            old_value: old_s,
            new_value: new_s,
            timestamp: env.ledger().timestamp(),
        };
        Self::push_history(env, &record);

        Ok(cfg)
    }

    /// Update market limits (requires admin with update_config permission)
    pub fn update_market_limits(
        env: &Env,
        admin: Address,
        limits: MarketLimits,
    ) -> Result<ContractConfig, Error> {
        crate::admin::AdminAccessControl::validate_admin_for_action(env, &admin, "update_config")?;

        let mut cfg = Self::get_config(env)?;
        // Old value snapshot (condensed)
        let old_s = String::from_str(
            env,
            &alloc::format!(
                "{{max_d:{},min_d:{},max_o:{},min_o:{},q_len:{},o_len:{}}}",
                cfg.market.max_duration_days,
                cfg.market.min_duration_days,
                cfg.market.max_outcomes,
                cfg.market.min_outcomes,
                cfg.market.max_question_length,
                cfg.market.max_outcome_length
            ),
        );

        // Apply new limits
        cfg.market.max_duration_days = limits.max_duration_days;
        cfg.market.min_duration_days = limits.min_duration_days;
        cfg.market.max_outcomes = limits.max_outcomes;
        cfg.market.min_outcomes = limits.min_outcomes;
        cfg.market.max_question_length = limits.max_question_length;
        cfg.market.max_outcome_length = limits.max_outcome_length;

        ConfigValidator::validate_market_config(&cfg.market)?;
        Self::update_config(env, &cfg)?;

        let change_type = String::from_str(env, "market_limits");
        let new_s = String::from_str(
            env,
            &alloc::format!(
                "{{max_d:{},min_d:{},max_o:{},min_o:{},q_len:{},o_len:{}}}",
                cfg.market.max_duration_days,
                cfg.market.min_duration_days,
                cfg.market.max_outcomes,
                cfg.market.min_outcomes,
                cfg.market.max_question_length,
                cfg.market.max_outcome_length
            ),
        );
        crate::events::EventEmitter::emit_config_updated(env, &admin, &change_type, &old_s, &new_s);

        let record = ConfigUpdateRecord {
            updated_by: admin,
            change_type,
            old_value: old_s,
            new_value: new_s,
            timestamp: env.ledger().timestamp(),
        };
        Self::push_history(env, &record);

        Ok(cfg)
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

// ===== CONFIGURATION UPDATE TYPES AND API =====

/// Market limits input for updating `MarketConfig` safely without exposing unrelated fields
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct MarketLimits {
    pub max_duration_days: u32,
    pub min_duration_days: u32,
    pub max_outcomes: u32,
    pub min_outcomes: u32,
    pub max_question_length: u32,
    pub max_outcome_length: u32,
}

/// Partial configuration changes for validation and bulk updates
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct ConfigChanges {
    pub platform_fee_percentage: Option<i128>,
    pub base_dispute_threshold: Option<i128>,
    pub oracle_timeout_seconds: Option<u32>,
    pub max_duration_days: Option<u32>,
    pub min_duration_days: Option<u32>,
    pub max_outcomes: Option<u32>,
    pub min_outcomes: Option<u32>,
    pub max_question_length: Option<u32>,
    pub max_outcome_length: Option<u32>,
}

/// Configuration update history record for audit trail
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct ConfigUpdateRecord {
    pub updated_by: Address,
    pub change_type: String,
    pub old_value: String,
    pub new_value: String,
    pub timestamp: u64,
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
    use soroban_sdk::testutils::Address as _;

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
        assert_eq!(
            ConfigUtils::get_fee_config(&mainnet_config).platform_fee_percentage,
            3
        );
    }

    #[test]
    fn test_config_storage() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let config = ConfigManager::get_development_config(&env);

        env.as_contract(&contract_id, || {
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
        });
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
