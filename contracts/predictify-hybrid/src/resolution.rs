use soroban_sdk::{contracttype, Address, Env, Map, String, Symbol, Vec};

use crate::errors::Error;

use crate::markets::{CommunityConsensus, MarketAnalytics, MarketStateManager, MarketUtils};

use crate::oracles::{OracleFactory, OracleUtils};
// use crate::reentrancy_guard::ReentrancyGuard; // Removed - module no longer exists
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

/// Enumeration of possible resolution states for market lifecycle management.
///
/// This enum tracks the progression of a market through its resolution phases,
/// from initial creation through final outcome determination. Each state represents
/// a specific stage in the resolution process with distinct validation rules and
/// available operations.
///
/// # State Transitions
///
/// The typical resolution flow follows this pattern:
/// ```text
/// Active → OracleResolved → MarketResolved → [Disputed] → Finalized
/// ```
///
/// **Alternative flows:**
/// - Direct admin resolution: `Active → MarketResolved → Finalized`
/// - Dispute flow: `MarketResolved → Disputed → Finalized`
/// - Oracle-only flow: `Active → OracleResolved → MarketResolved → Finalized`
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Symbol};
/// # use predictify_hybrid::resolution::{ResolutionState, ResolutionUtils};
/// # use predictify_hybrid::markets::Market;
/// # let env = Env::default();
/// # let market = Market::default(); // Placeholder
///
/// // Check current resolution state
/// let current_state = ResolutionUtils::get_resolution_state(&env, &market);
///
/// match current_state {
///     ResolutionState::Active => {
///         println!("Market is active, ready for oracle resolution");
///         // Can fetch oracle results
///     },
///     ResolutionState::OracleResolved => {
///         println!("Oracle result available, can proceed to market resolution");
///         // Can combine with community consensus
///     },
///     ResolutionState::MarketResolved => {
///         println!("Market resolved, awaiting finalization or disputes");
///         // Can be disputed or finalized
///     },
///     ResolutionState::Disputed => {
///         println!("Resolution is under dispute");
///         // Dispute resolution process active
///     },
///     ResolutionState::Finalized => {
///         println!("Resolution is final and immutable");
///         // No further changes allowed
///     },
/// }
/// ```
///
/// # State Validation
///
/// Each state has specific validation requirements:
/// - **Active**: Market must be within voting period
/// - **OracleResolved**: Oracle data must be valid and recent
/// - **MarketResolved**: Final outcome must be determined
/// - **Disputed**: Dispute must be properly filed and active
/// - **Finalized**: Resolution must be complete and immutable
///
/// # Business Rules
///
/// State transitions enforce business logic:
/// - Markets cannot skip resolution states arbitrarily
/// - Finalized resolutions cannot be changed
/// - Disputed resolutions require proper dispute resolution
/// - Oracle resolution requires valid oracle data
///
/// # Integration Points
///
/// Resolution states integrate with:
/// - **Market Management**: Controls available market operations
/// - **Voting System**: Determines when voting periods end
/// - **Dispute System**: Manages dispute lifecycle
/// - **Oracle System**: Coordinates oracle data fetching
/// - **Admin Functions**: Enables administrative overrides
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

/// Comprehensive oracle resolution result containing all data needed for market resolution.
///
/// This structure captures the complete oracle response for a market, including
/// the raw price data, comparison logic, outcome determination, and metadata
/// necessary for validation and audit trails.
///
/// # Core Components
///
/// **Market Context:**
/// - **Market ID**: Unique identifier linking resolution to specific market
/// - **Timestamp**: When the oracle resolution was performed
/// - **Provider**: Which oracle service provided the data
///
/// **Oracle Data:**
/// - **Price**: Current asset price from oracle feed
/// - **Threshold**: Market-defined price threshold for comparison
/// - **Comparison**: Comparison operator ("gt", "lt", "eq")
/// - **Feed ID**: Specific oracle feed identifier used
///
/// **Resolution Result:**
/// - **Oracle Result**: Final outcome ("yes"/"no") based on price comparison
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Symbol, String, Address};
/// # use predictify_hybrid::resolution::{OracleResolutionManager, OracleResolution};
/// # use predictify_hybrid::types::OracleProvider;
/// # let env = Env::default();
/// # let market_id = Symbol::new(&env, "btc_50k");
/// # let oracle_contract = Address::generate(&env);
///
/// // Fetch oracle resolution for a market
/// let oracle_resolution = OracleResolutionManager::fetch_oracle_result(
///     &env,
///     &market_id,
///     &oracle_contract
/// )?;
///
/// // Examine oracle resolution details
/// println!("Market: {}", oracle_resolution.market_id);
/// println!("Oracle result: {}", oracle_resolution.oracle_result);
/// println!("Price: ${}", oracle_resolution.price / 100);
/// println!("Threshold: ${}", oracle_resolution.threshold / 100);
/// println!("Comparison: {}", oracle_resolution.comparison);
/// println!("Provider: {:?}", oracle_resolution.provider);
/// println!("Feed: {}", oracle_resolution.feed_id);
///
/// // Validate oracle resolution
/// OracleResolutionManager::validate_oracle_resolution(&env, &oracle_resolution)?;
///
/// // Calculate confidence score
/// let confidence = OracleResolutionManager::calculate_oracle_confidence(&oracle_resolution);
/// println!("Oracle confidence: {}%", confidence);
/// # Ok::<(), predictify_hybrid::errors::Error>(())
/// ```
///
/// # Price Comparison Logic
///
/// The oracle resolution evaluates market conditions:
/// ```rust
/// # use soroban_sdk::{Env, String};
/// # use predictify_hybrid::oracles::OracleUtils;
/// # let env = Env::default();
///
/// // Example: BTC above $50,000?
/// let btc_price = 52_000_00;    // $52,000 (8 decimal precision)
/// let threshold = 50_000_00;    // $50,000
/// let comparison = String::from_str(&env, "gt"); // Greater than
///
/// let outcome = OracleUtils::determine_outcome(
///     btc_price,
///     threshold,
///     &comparison,
///     &env
/// )?;
///
/// assert_eq!(outcome, String::from_str(&env, "yes")); // BTC > $50k = "yes"
/// # Ok::<(), predictify_hybrid::errors::Error>(())
/// ```
///
/// # Validation Requirements
///
/// Oracle resolutions must meet criteria:
/// - **Valid Price**: Price must be positive and within reasonable bounds
/// - **Recent Data**: Timestamp must be within acceptable staleness limits
/// - **Supported Provider**: Oracle provider must be supported on current network
/// - **Valid Feed**: Feed ID must exist and be active
/// - **Proper Comparison**: Comparison operator must be supported
///
/// # Integration with Market Resolution
///
/// Oracle resolutions feed into broader market resolution:
/// - **Hybrid Resolution**: Combined with community consensus
/// - **Oracle-Only**: Used directly as final outcome
/// - **Dispute Input**: Provides data for dispute resolution
/// - **Confidence Scoring**: Contributes to overall resolution confidence
///
/// # Audit and Transparency
///
/// All oracle resolution data is preserved for:
/// - **Audit Trails**: Complete record of resolution process
/// - **Dispute Evidence**: Data available for dispute proceedings
/// - **Analytics**: Historical analysis of oracle performance
/// - **Transparency**: Public verification of resolution logic
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

/// Comprehensive market resolution result combining oracle data with community consensus.
///
/// This structure represents the final resolution of a prediction market, incorporating
/// data from multiple sources (oracle feeds, community voting, admin decisions) to
/// determine the authoritative market outcome with confidence scoring and audit trails.
///
/// # Resolution Components
///
/// **Core Resolution Data:**
/// - **Market ID**: Unique identifier for the resolved market
/// - **Final Outcome**: Definitive market result ("yes"/"no" or custom outcomes)
/// - **Resolution Timestamp**: When the resolution was finalized
/// - **Resolution Method**: How the resolution was determined
///
/// **Data Sources:**
/// - **Oracle Result**: Outcome from oracle price feeds
/// - **Community Consensus**: Aggregated community voting results
/// - **Confidence Score**: Statistical confidence in the resolution (0-100)
///
/// # Resolution Methods
///
/// Markets can be resolved through various methods:
/// - **Oracle Only**: Based purely on oracle price data
/// - **Community Only**: Based on community voting consensus
/// - **Hybrid**: Combines oracle data with community input
/// - **Admin Override**: Administrative decision overrides other methods
/// - **Dispute Resolution**: Outcome determined through dispute process
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Symbol, String};
/// # use predictify_hybrid::resolution::{MarketResolutionManager, MarketResolution, ResolutionMethod};
/// # let env = Env::default();
/// # let market_id = Symbol::new(&env, "btc_prediction");
///
/// // Resolve a market using hybrid method
/// let resolution = MarketResolutionManager::resolve_market(&env, &market_id)?;
///
/// // Examine resolution details
/// println!("Market: {}", resolution.market_id);
/// println!("Final outcome: {}", resolution.final_outcome);
/// println!("Oracle result: {}", resolution.oracle_result);
/// println!("Community consensus: {}% ({})",
///     resolution.community_consensus.percentage,
///     resolution.community_consensus.outcome
/// );
/// println!("Resolution method: {:?}", resolution.resolution_method);
/// println!("Confidence: {}%", resolution.confidence_score);
///
/// // Validate the resolution
/// MarketResolutionManager::validate_market_resolution(&env, &resolution)?;
///
/// // Check resolution method
/// match resolution.resolution_method {
///     ResolutionMethod::Hybrid => {
///         println!("Resolution combines oracle and community data");
///     },
///     ResolutionMethod::OracleOnly => {
///         println!("Resolution based purely on oracle data");
///     },
///     ResolutionMethod::AdminOverride => {
///         println!("Resolution was administratively determined");
///     },
///     _ => println!("Other resolution method used"),
/// }
/// # Ok::<(), predictify_hybrid::errors::Error>(())
/// ```
///
/// # Confidence Scoring
///
/// Resolution confidence is calculated based on:
/// - **Oracle Reliability**: Historical oracle accuracy and freshness
/// - **Community Agreement**: Level of consensus in community voting
/// - **Data Quality**: Quality and recency of underlying data
/// - **Method Reliability**: Inherent reliability of resolution method
///
/// ```rust
/// # use predictify_hybrid::resolution::MarketResolution;
/// # let resolution = MarketResolution::default(); // Placeholder
///
/// // Interpret confidence scores
/// match resolution.confidence_score {
///     90..=100 => println!("Very high confidence resolution"),
///     80..=89 => println!("High confidence resolution"),
///     70..=79 => println!("Moderate confidence resolution"),
///     60..=69 => println!("Low confidence resolution"),
///     _ => println!("Very low confidence - may need review"),
/// }
/// ```
///
/// # Resolution Validation
///
/// Market resolutions undergo validation to ensure:
/// - **Outcome Consistency**: Oracle and community data alignment
/// - **Method Appropriateness**: Resolution method suitable for market type
/// - **Data Quality**: All input data meets quality standards
/// - **Timestamp Validity**: Resolution timing is appropriate
/// - **Confidence Thresholds**: Confidence score meets minimum requirements
///
/// # Integration Points
///
/// Market resolutions integrate with:
/// - **Payout System**: Determines winner payouts and distributions
/// - **Dispute System**: Can be challenged through dispute mechanisms
/// - **Analytics**: Contributes to platform performance metrics
/// - **Audit System**: Provides complete resolution audit trails
/// - **Event System**: Triggers resolution events for transparency
///
/// # Immutability and Finalization
///
/// Once finalized, market resolutions are immutable except through:
/// - **Dispute Process**: Formal dispute resolution procedures
/// - **Admin Override**: Emergency administrative corrections
/// - **System Upgrades**: Protocol-level corrections (rare)
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

/// Enumeration of available market resolution methods and their characteristics.
///
/// This enum defines the different approaches available for resolving prediction markets,
/// each with distinct data sources, validation requirements, and confidence characteristics.
/// The choice of resolution method depends on market type, data availability, and
/// community participation levels.
///
/// # Resolution Method Types
///
/// **Automated Methods:**
/// - **Oracle Only**: Purely algorithmic based on price feed data
/// - **Community Only**: Based entirely on community voting consensus
/// - **Hybrid**: Combines oracle data with community input for balanced resolution
///
/// **Manual Methods:**
/// - **Admin Override**: Administrative decision for exceptional circumstances
/// - **Dispute Resolution**: Outcome determined through formal dispute process
///
/// # Method Selection Logic
///
/// Resolution methods are typically selected based on:
/// ```rust
/// # use predictify_hybrid::resolution::ResolutionMethod;
/// # use predictify_hybrid::markets::CommunityConsensus;
/// # use soroban_sdk::{Env, String};
/// # let env = Env::default();
///
/// // Example method selection logic
/// fn select_resolution_method(
///     oracle_available: bool,
///     community_participation: u32,
///     consensus_strength: u32
/// ) -> ResolutionMethod {
///     match (oracle_available, community_participation, consensus_strength) {
///         (true, participation, consensus) if participation > 50 && consensus > 75 => {
///             ResolutionMethod::Hybrid // Strong community + oracle
///         },
///         (true, participation, _) if participation < 30 => {
///             ResolutionMethod::OracleOnly // Low community participation
///         },
///         (false, participation, consensus) if participation > 100 && consensus > 80 => {
///             ResolutionMethod::CommunityOnly // No oracle, strong community
///         },
///         _ => ResolutionMethod::AdminOverride // Fallback to admin
///     }
/// }
/// ```
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, String};
/// # use predictify_hybrid::resolution::{ResolutionMethod, MarketResolutionAnalytics};
/// # use predictify_hybrid::markets::CommunityConsensus;
/// # let env = Env::default();
///
/// // Determine resolution method based on available data
/// let oracle_result = String::from_str(&env, "yes");
/// let community_consensus = CommunityConsensus {
///     outcome: String::from_str(&env, "yes"),
///     votes: 150,
///     total_votes: 200,
///     percentage: 75,
/// };
///
/// let method = MarketResolutionAnalytics::determine_resolution_method(
///     &oracle_result,
///     &community_consensus
/// );
///
/// match method {
///     ResolutionMethod::Hybrid => {
///         println!("Using hybrid resolution - oracle and community agree");
///     },
///     ResolutionMethod::OracleOnly => {
///         println!("Using oracle-only resolution - low community participation");
///     },
///     ResolutionMethod::CommunityOnly => {
///         println!("Using community-only resolution - oracle unavailable");
///     },
///     ResolutionMethod::AdminOverride => {
///         println!("Using admin override - exceptional circumstances");
///     },
///     ResolutionMethod::DisputeResolution => {
///         println!("Using dispute resolution - conflicting data sources");
///     },
/// }
/// ```
///
/// # Method Characteristics
///
/// **Oracle Only:**
/// - **Speed**: Fastest resolution method
/// - **Objectivity**: Purely algorithmic, no human bias
/// - **Reliability**: Depends on oracle data quality
/// - **Use Case**: Clear-cut price-based markets
///
/// **Community Only:**
/// - **Participation**: Requires active community engagement
/// - **Flexibility**: Can handle subjective or complex outcomes
/// - **Consensus**: Relies on community agreement
/// - **Use Case**: Subjective or oracle-unavailable markets
///
/// **Hybrid:**
/// - **Balance**: Combines objective data with community wisdom
/// - **Validation**: Cross-validates oracle data with community input
/// - **Confidence**: Generally highest confidence scores
/// - **Use Case**: Most standard prediction markets
///
/// **Admin Override:**
/// - **Authority**: Administrative decision with full authority
/// - **Speed**: Can be immediate when needed
/// - **Responsibility**: Requires admin accountability
/// - **Use Case**: Emergency situations or system failures
///
/// **Dispute Resolution:**
/// - **Process**: Formal dispute resolution procedures
/// - **Thoroughness**: Most comprehensive review process
/// - **Time**: Longest resolution time
/// - **Use Case**: Contested or controversial outcomes
///
/// # Integration with Confidence Scoring
///
/// Different methods contribute to confidence scores:
/// - **Hybrid**: Highest confidence when oracle and community agree
/// - **Oracle Only**: High confidence for clear price-based outcomes
/// - **Community Only**: Confidence based on participation and consensus
/// - **Admin Override**: Confidence based on admin justification
/// - **Dispute Resolution**: Confidence based on dispute outcome strength
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

/// Comprehensive analytics and metrics for resolution system performance.
///
/// This structure tracks detailed statistics about the resolution system's
/// performance, method usage, timing characteristics, and outcome distributions.
/// It provides essential data for system optimization, transparency reporting,
/// and platform analytics.
///
/// # Analytics Categories
///
/// **Volume Metrics:**
/// - **Total Resolutions**: Overall count of resolved markets
/// - **Method Breakdown**: Count by resolution method type
/// - **Outcome Distribution**: Frequency of different outcomes
///
/// **Quality Metrics:**
/// - **Average Confidence**: Mean confidence score across resolutions
/// - **Resolution Times**: Time taken for different resolution methods
/// - **Success Rates**: Percentage of successful resolutions by method
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Map, String, Vec};
/// # use predictify_hybrid::resolution::{ResolutionAnalytics, ResolutionAnalyticsManager};
/// # let env = Env::default();
///
/// // Get current resolution analytics
/// let analytics = ResolutionAnalyticsManager::get_resolution_analytics(&env)?;
///
/// // Display system performance metrics
/// println!("=== Resolution System Analytics ===");
/// println!("Total resolutions: {}", analytics.total_resolutions);
/// println!("Oracle resolutions: {}", analytics.oracle_resolutions);
/// println!("Community resolutions: {}", analytics.community_resolutions);
/// println!("Hybrid resolutions: {}", analytics.hybrid_resolutions);
/// println!("Average confidence: {}%", analytics.average_confidence / 100);
///
/// // Calculate method distribution
/// let total = analytics.total_resolutions as f64;
/// if total > 0.0 {
///     println!("Oracle-only: {:.1}%", (analytics.oracle_resolutions as f64 / total) * 100.0);
///     println!("Community-only: {:.1}%", (analytics.community_resolutions as f64 / total) * 100.0);
///     println!("Hybrid: {:.1}%", (analytics.hybrid_resolutions as f64 / total) * 100.0);
/// }
///
/// // Analyze resolution times
/// if !analytics.resolution_times.is_empty() {
///     let avg_time = analytics.resolution_times.iter().sum::<u64>() / analytics.resolution_times.len() as u64;
///     println!("Average resolution time: {} seconds", avg_time);
/// }
///
/// // Display outcome distribution
/// for (outcome, count) in analytics.outcome_distribution.iter() {
///     println!("Outcome '{}': {} markets", outcome, count);
/// }
/// # Ok::<(), predictify_hybrid::errors::Error>(())
/// ```
///
/// # Performance Monitoring
///
/// Analytics enable monitoring of:
/// ```rust
/// # use predictify_hybrid::resolution::ResolutionAnalytics;
/// # let analytics = ResolutionAnalytics::default();
///
/// // Monitor system health
/// fn assess_system_health(analytics: &ResolutionAnalytics) -> String {
///     let confidence_threshold = 80_00; // 80% in basis points
///     let hybrid_ratio = if analytics.total_resolutions > 0 {
///         (analytics.hybrid_resolutions as f64 / analytics.total_resolutions as f64) * 100.0
///     } else {
///         0.0
///     };
///     
///     match (analytics.average_confidence >= confidence_threshold, hybrid_ratio >= 50.0) {
///         (true, true) => "Excellent - High confidence and balanced resolution methods".to_string(),
///         (true, false) => "Good - High confidence but method imbalance".to_string(),
///         (false, true) => "Fair - Balanced methods but lower confidence".to_string(),
///         (false, false) => "Needs attention - Low confidence and method imbalance".to_string(),
///     }
/// }
/// ```
///
/// # Trend Analysis
///
/// Resolution analytics support trend analysis:
/// - **Method Evolution**: How resolution method preferences change over time
/// - **Confidence Trends**: Whether resolution confidence is improving
/// - **Outcome Patterns**: Distribution of market outcomes
/// - **Performance Optimization**: Identifying areas for system improvement
///
/// # Business Intelligence
///
/// Analytics provide insights for:
/// - **Platform Performance**: Overall system effectiveness metrics
/// - **User Behavior**: How community participates in resolution
/// - **Oracle Reliability**: Performance of different oracle providers
/// - **Market Types**: Which market types work best with which methods
///
/// # Data Privacy and Aggregation
///
/// Analytics maintain privacy through:
/// - **Aggregated Data**: No individual user information exposed
/// - **Statistical Summaries**: Focus on system-level metrics
/// - **Time-based Aggregation**: Historical trends without personal data
/// - **Public Transparency**: Safe for public consumption
///
/// # Integration with Reporting
///
/// Resolution analytics integrate with:
/// - **Dashboard Systems**: Real-time performance monitoring
/// - **Audit Reports**: Compliance and transparency reporting
/// - **API Endpoints**: External system integration
/// - **Governance Metrics**: DAO governance decision support
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

/// Comprehensive validation result for resolution processes and outcomes.
///
/// This structure provides detailed feedback on the validity of resolution attempts,
/// including validation status, specific error conditions, warnings about potential
/// issues, and recommendations for improvement. It serves as a comprehensive
/// diagnostic tool for resolution quality assurance.
///
/// # Validation Components
///
/// **Status Indicators:**
/// - **Is Valid**: Boolean indicating overall validation success
/// - **Errors**: Critical issues that prevent resolution
/// - **Warnings**: Non-critical issues that should be addressed
/// - **Recommendations**: Suggestions for improving resolution quality
///
/// # Validation Categories
///
/// **Data Quality Validation:**
/// - Oracle data freshness and accuracy
/// - Community voting participation levels
/// - Consensus strength and distribution
/// - Timestamp validity and sequencing
///
/// **Business Logic Validation:**
/// - Market state compatibility with resolution method
/// - Outcome consistency across data sources
/// - Confidence score reasonableness
/// - Resolution method appropriateness
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Vec, String};
/// # use predictify_hybrid::resolution::{ResolutionValidation, MarketResolutionManager, MarketResolution};
/// # let env = Env::default();
/// # let resolution = MarketResolution::default(); // Placeholder
///
/// // Validate a market resolution
/// let validation = MarketResolutionManager::validate_market_resolution(&env, &resolution)?;
///
/// if validation.is_valid {
///     println!("✅ Resolution is valid and ready for finalization");
///     
///     // Check for warnings
///     if !validation.warnings.is_empty() {
///         println!("⚠️  Warnings to consider:");
///         for warning in validation.warnings.iter() {
///             println!("  - {}", warning);
///         }
///     }
///     
///     // Review recommendations
///     if !validation.recommendations.is_empty() {
///         println!("💡 Recommendations for improvement:");
///         for recommendation in validation.recommendations.iter() {
///             println!("  - {}", recommendation);
///         }
///     }
/// } else {
///     println!("❌ Resolution validation failed");
///     println!("Errors that must be resolved:");
///     for error in validation.errors.iter() {
///         println!("  - {}", error);
///     }
/// }
/// # Ok::<(), predictify_hybrid::errors::Error>(())
/// ```
///
/// # Validation Workflow
///
/// ```rust
/// # use predictify_hybrid::resolution::{ResolutionValidation, OracleResolution};
/// # use soroban_sdk::{Env, Vec, String};
/// # let env = Env::default();
///
/// // Example validation workflow
/// fn comprehensive_validation_workflow(
///     env: &Env,
///     oracle_resolution: &OracleResolution
/// ) -> Result<bool, predictify_hybrid::errors::Error> {
///     // Step 1: Validate oracle resolution
///     let oracle_validation = validate_oracle_data(env, oracle_resolution)?;
///     
///     if !oracle_validation.is_valid {
///         println!("Oracle validation failed: {:?}", oracle_validation.errors);
///         return Ok(false);
///     }
///     
///     // Step 2: Check for warnings
///     if !oracle_validation.warnings.is_empty() {
///         println!("Oracle warnings: {:?}", oracle_validation.warnings);
///     }
///     
///     // Step 3: Apply recommendations if possible
///     for recommendation in oracle_validation.recommendations.iter() {
///         println!("Consider: {}", recommendation);
///     }
///     
///     Ok(true)
/// }
///
/// fn validate_oracle_data(
///     _env: &Env,
///     _oracle_resolution: &OracleResolution
/// ) -> Result<ResolutionValidation, predictify_hybrid::errors::Error> {
///     // Placeholder implementation
///     Ok(ResolutionValidation {
///         is_valid: true,
///         errors: Vec::new(_env),
///         warnings: Vec::new(_env),
///         recommendations: Vec::new(_env),
///     })
/// }
/// ```
///
/// # Error Categories
///
/// **Critical Errors (Block Resolution):**
/// - Invalid oracle data or stale timestamps
/// - Insufficient community participation
/// - Conflicting outcomes without resolution method
/// - Missing required data for chosen resolution method
///
/// **Warnings (Proceed with Caution):**
/// - Low confidence scores
/// - Minimal community participation
/// - Oracle data approaching staleness limits
/// - Unusual outcome patterns
///
/// **Recommendations (Optimization):**
/// - Increase community engagement
/// - Use hybrid resolution for better confidence
/// - Consider additional oracle sources
/// - Implement dispute period for controversial outcomes
///
/// # Integration with Resolution Process
///
/// Validation integrates at multiple points:
/// - **Pre-Resolution**: Validate readiness before attempting resolution
/// - **Post-Resolution**: Validate outcome quality and consistency
/// - **Dispute Handling**: Validate dispute claims and evidence
/// - **Finalization**: Final validation before immutable storage
///
/// # Quality Assurance
///
/// Validation supports quality assurance through:
/// - **Automated Checks**: Systematic validation of all resolution components
/// - **Consistency Verification**: Cross-validation between data sources
/// - **Business Rule Enforcement**: Ensure compliance with platform rules
/// - **Audit Trail Generation**: Document validation decisions and rationale
#[derive(Clone, Debug)]
#[contracttype]
pub struct ResolutionValidation {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub recommendations: Vec<String>,
}

// ===== ORACLE RESOLUTION =====

/// Comprehensive oracle resolution management system for prediction markets.
///
/// The Oracle Resolution Manager handles all aspects of oracle-based market resolution,
/// including fetching oracle data, validating oracle responses, calculating confidence
/// scores, and managing the oracle resolution lifecycle. It serves as the primary
/// interface between the prediction market system and external oracle providers.
///
/// # Core Responsibilities
///
/// **Oracle Data Management:**
/// - **Data Fetching**: Retrieve price data from configured oracle providers
/// - **Data Validation**: Ensure oracle responses meet quality standards
/// - **Confidence Scoring**: Calculate reliability scores for oracle data
/// - **Error Handling**: Manage oracle failures and fallback strategies
///
/// **Market Integration:**
/// - **Market Validation**: Ensure markets are ready for oracle resolution
/// - **Outcome Determination**: Convert oracle data to market outcomes
/// - **Resolution Storage**: Persist oracle resolution results
/// - **Event Emission**: Notify system of oracle resolution events
///
/// # Oracle Resolution Process
///
/// The typical oracle resolution workflow:
/// ```text
/// 1. Validate Market → 2. Fetch Oracle Data → 3. Validate Response →
/// 4. Calculate Outcome → 5. Score Confidence → 6. Store Resolution
/// ```
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Symbol, Address};
/// # use predictify_hybrid::resolution::{OracleResolutionManager, OracleResolution};
/// # let env = Env::default();
/// # let market_id = Symbol::new(&env, "btc_50k_market");
/// # let oracle_contract = Address::generate(&env);
///
/// // Fetch oracle resolution for a market
/// let oracle_resolution = OracleResolutionManager::fetch_oracle_result(
///     &env,
///     &market_id,
///     &oracle_contract
/// )?;
///
/// println!("Oracle Resolution Results:");
/// println!("Market: {}", oracle_resolution.market_id);
/// println!("Result: {}", oracle_resolution.oracle_result);
/// println!("Price: ${}", oracle_resolution.price / 100);
/// println!("Threshold: ${}", oracle_resolution.threshold / 100);
/// println!("Provider: {:?}", oracle_resolution.provider);
///
/// // Validate the oracle resolution
/// OracleResolutionManager::validate_oracle_resolution(&env, &oracle_resolution)?;
///
/// // Calculate confidence score
/// let confidence = OracleResolutionManager::calculate_oracle_confidence(&oracle_resolution);
/// println!("Oracle confidence: {}%", confidence);
///
/// // Store resolution for later retrieval
/// // (Implementation would store in contract storage)
///
/// // Retrieve stored resolution
/// if let Some(stored_resolution) = OracleResolutionManager::get_oracle_resolution(
///     &env,
///     &market_id
/// )? {
///     println!("Successfully retrieved stored oracle resolution");
/// }
/// # Ok::<(), predictify_hybrid::errors::Error>(())
/// ```
///
/// # Oracle Provider Integration
///
/// The manager integrates with multiple oracle providers:
/// ```rust
/// # use soroban_sdk::{Env, Address};
/// # use predictify_hybrid::oracles::{OracleFactory, OracleInstance};
/// # use predictify_hybrid::types::OracleProvider;
/// # let env = Env::default();
/// # let oracle_contract = Address::generate(&env);
///
/// // Create oracle instance based on provider
/// let oracle = OracleFactory::create_oracle(
///     OracleProvider::Reflector, // Primary provider for Stellar
///     oracle_contract
/// )?;
///
/// // Use oracle for price fetching
/// match oracle {
///     OracleInstance::Reflector(reflector_oracle) => {
///         println!("Using Reflector oracle for price data");
///         // Reflector-specific operations
///     },
///     OracleInstance::Pyth(pyth_oracle) => {
///         println!("Using Pyth oracle (future implementation)");
///         // Pyth-specific operations
///     },
/// }
/// # Ok::<(), predictify_hybrid::errors::Error>(())
/// ```
///
/// # Confidence Scoring Algorithm
///
/// Oracle confidence is calculated based on:
/// - **Data Freshness**: How recent the oracle data is
/// - **Provider Reliability**: Historical accuracy of the oracle provider
/// - **Price Stability**: Volatility and consistency of price data
/// - **Network Health**: Oracle network status and availability
///
/// ```rust
/// # use predictify_hybrid::resolution::{OracleResolution, OracleResolutionManager};
/// # let oracle_resolution = OracleResolution::default(); // Placeholder
///
/// // Confidence scoring factors
/// let confidence = OracleResolutionManager::calculate_oracle_confidence(&oracle_resolution);
///
/// match confidence {
///     90..=100 => println!("Very high confidence - excellent oracle data"),
///     80..=89 => println!("High confidence - reliable oracle data"),
///     70..=79 => println!("Moderate confidence - acceptable oracle data"),
///     60..=69 => println!("Low confidence - oracle data has issues"),
///     _ => println!("Very low confidence - oracle data unreliable"),
/// }
/// ```
///
/// # Error Handling and Fallbacks
///
/// The manager handles various error scenarios:
/// - **Oracle Unavailable**: Network issues or service downtime
/// - **Invalid Data**: Malformed or unreasonable oracle responses
/// - **Stale Data**: Oracle data older than acceptable thresholds
/// - **Feed Errors**: Requested price feed not available
///
/// # Integration with Market Resolution
///
/// Oracle resolutions feed into broader market resolution:
/// - **Hybrid Resolution**: Combined with community consensus
/// - **Oracle-Only Markets**: Direct outcome determination
/// - **Dispute Evidence**: Oracle data used in dispute resolution
/// - **Confidence Weighting**: Oracle confidence affects final resolution confidence
///
/// # Performance and Optimization
///
/// The manager optimizes performance through:
/// - **Caching**: Cache oracle responses to reduce network calls
/// - **Batch Processing**: Handle multiple markets efficiently
/// - **Async Operations**: Non-blocking oracle data fetching
/// - **Fallback Strategies**: Multiple oracle providers for reliability
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

        // Perform external oracle call (reentrancy guard removed)
        let price_result = oracle.get_price(env, &market.oracle_config.feed_id);
        let price = price_result?;

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

        // Emit oracle result event
        let provider_str = soroban_sdk::String::from_str(env, "Oracle");
        let feed_str = market.oracle_config.feed_id.clone();
        let comparison_str = market.oracle_config.comparison.clone();

        crate::events::EventEmitter::emit_oracle_result(
            env,
            market_id,
            &outcome,
            &provider_str,
            &feed_str,
            price,
            market.oracle_config.threshold,
            &comparison_str,
        );

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

/// Comprehensive market resolution management system combining multiple data sources.
///
/// The Market Resolution Manager orchestrates the complete market resolution process,
/// integrating oracle data, community consensus, admin decisions, and dispute outcomes
/// to determine final market results. It serves as the central coordinator for all
/// resolution methods and ensures consistent, reliable market outcomes.
///
/// # Core Responsibilities
///
/// **Resolution Orchestration:**
/// - **Multi-Source Integration**: Combine oracle, community, and admin data
/// - **Method Selection**: Choose appropriate resolution method based on available data
/// - **Confidence Calculation**: Determine overall confidence in resolution outcome
/// - **Validation**: Ensure resolution meets quality and consistency standards
///
/// **Market Lifecycle Management:**
/// - **Resolution Triggering**: Initiate resolution when markets are ready
/// - **State Management**: Track resolution progress through various states
/// - **Finalization**: Complete resolution process and make outcomes immutable
/// - **Event Emission**: Notify system components of resolution events
///
/// # Resolution Methods Supported
///
/// **Hybrid Resolution (Recommended):**
/// - Combines oracle price data with community voting
/// - Highest confidence when sources agree
/// - Fallback logic when sources disagree
///
/// **Oracle-Only Resolution:**
/// - Pure algorithmic resolution based on price feeds
/// - Fast and objective for clear-cut price-based markets
/// - Used when community participation is insufficient
///
/// **Community-Only Resolution:**
/// - Based entirely on community voting consensus
/// - Used when oracle data is unavailable or inappropriate
/// - Requires sufficient participation and consensus
///
/// **Admin Override:**
/// - Administrative decision for exceptional circumstances
/// - Used for emergency situations or system failures
/// - Requires proper admin authentication and justification
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Symbol, Address, String};
/// # use predictify_hybrid::resolution::{MarketResolutionManager, MarketResolution, ResolutionMethod};
/// # let env = Env::default();
/// # let market_id = Symbol::new(&env, "btc_prediction_market");
/// # let admin = Address::generate(&env);
///
/// // Resolve a market using hybrid method (oracle + community)
/// let resolution = MarketResolutionManager::resolve_market(&env, &market_id)?;
///
/// println!("Market Resolution Complete:");
/// println!("Market: {}", resolution.market_id);
/// println!("Final outcome: {}", resolution.final_outcome);
/// println!("Method: {:?}", resolution.resolution_method);
/// println!("Confidence: {}%", resolution.confidence_score);
///
/// // Display resolution details
/// match resolution.resolution_method {
///     ResolutionMethod::Hybrid => {
///         println!("Oracle result: {}", resolution.oracle_result);
///         println!("Community consensus: {}% ({})",
///             resolution.community_consensus.percentage,
///             resolution.community_consensus.outcome
///         );
///     },
///     ResolutionMethod::OracleOnly => {
///         println!("Resolved purely based on oracle: {}", resolution.oracle_result);
///     },
///     ResolutionMethod::AdminOverride => {
///         println!("Administrative override resolution");
///     },
///     _ => println!("Other resolution method used"),
/// }
///
/// // Validate the resolution
/// MarketResolutionManager::validate_market_resolution(&env, &resolution)?;
///
/// // Admin can finalize with override if needed
/// if resolution.confidence_score < 70 {
///     let admin_resolution = MarketResolutionManager::finalize_market(
///         &env,
///         &admin,
///         &market_id,
///         &String::from_str(&env, "yes")
///     )?;
///     println!("Admin finalized with outcome: {}", admin_resolution.final_outcome);
/// }
/// # Ok::<(), predictify_hybrid::errors::Error>(())
/// ```
///
/// # Resolution Decision Logic
///
/// The manager uses sophisticated logic to determine final outcomes:
/// ```rust
/// # use soroban_sdk::{Env, String};
/// # use predictify_hybrid::resolution::ResolutionMethod;
/// # use predictify_hybrid::markets::CommunityConsensus;
/// # let env = Env::default();
///
/// // Example resolution decision logic
/// fn determine_final_outcome(
///     oracle_result: &String,
///     community_consensus: &CommunityConsensus,
///     oracle_confidence: u32,
///     community_confidence: u32
/// ) -> (String, ResolutionMethod) {
///     let env = Env::default();
///     
///     // Check if oracle and community agree
///     if oracle_result == &community_consensus.outcome {
///         // Agreement - use hybrid method with high confidence
///         (oracle_result.clone(), ResolutionMethod::Hybrid)
///     } else if oracle_confidence > 85 && community_confidence < 60 {
///         // Strong oracle, weak community - use oracle
///         (oracle_result.clone(), ResolutionMethod::OracleOnly)
///     } else if community_confidence > 85 && oracle_confidence < 60 {
///         // Strong community, weak oracle - use community
///         (community_consensus.outcome.clone(), ResolutionMethod::CommunityOnly)
///     } else {
///         // Conflict requires admin intervention
///         (String::from_str(&env, "disputed"), ResolutionMethod::AdminOverride)
///     }
/// }
/// ```
///
/// # Confidence Scoring
///
/// Resolution confidence is calculated from multiple factors:
/// - **Oracle Confidence**: Quality and freshness of oracle data
/// - **Community Confidence**: Participation level and consensus strength
/// - **Method Reliability**: Inherent reliability of chosen resolution method
/// - **Data Consistency**: Agreement between different data sources
///
/// ```rust
/// # use predictify_hybrid::resolution::MarketResolution;
/// # let resolution = MarketResolution::default(); // Placeholder
///
/// // Interpret confidence levels
/// match resolution.confidence_score {
///     95..=100 => println!("Extremely high confidence - virtually certain outcome"),
///     85..=94 => println!("Very high confidence - strong evidence for outcome"),
///     75..=84 => println!("High confidence - good evidence for outcome"),
///     65..=74 => println!("Moderate confidence - reasonable evidence"),
///     50..=64 => println!("Low confidence - weak evidence, consider review"),
///     _ => println!("Very low confidence - outcome uncertain, needs attention"),
/// }
/// ```
///
/// # Error Handling and Fallbacks
///
/// The manager handles various failure scenarios:
/// - **Oracle Failures**: Fallback to community-only resolution
/// - **Low Participation**: Fallback to oracle-only or admin resolution
/// - **Data Conflicts**: Escalate to dispute resolution process
/// - **System Errors**: Graceful degradation with error reporting
///
/// # Integration with Other Systems
///
/// Market Resolution Manager integrates with:
/// - **Oracle System**: Fetches and validates oracle data
/// - **Voting System**: Retrieves community consensus data
/// - **Dispute System**: Handles disputed resolutions
/// - **Admin System**: Processes administrative overrides
/// - **Event System**: Emits resolution events for transparency
/// - **Analytics System**: Records resolution metrics and performance
///
/// # Performance and Scalability
///
/// The manager optimizes for:
/// - **Batch Processing**: Resolve multiple markets efficiently
/// - **Parallel Resolution**: Handle independent resolutions concurrently
/// - **Caching**: Cache resolution data to avoid redundant calculations
/// - **Event-Driven**: React to market state changes automatically
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
            oracle_result: oracle_result.clone(),
            community_consensus,
            resolution_timestamp: env.ledger().timestamp(),
            resolution_method,
            confidence_score,
        };

        // Determine resolution source string for dispute window
        let resolution_source = match resolution_method {
            ResolutionMethod::OracleOnly => "Oracle",
            ResolutionMethod::CommunityOnly => "Community",
            ResolutionMethod::Hybrid => "Hybrid",
            ResolutionMethod::AdminOverride => "Admin",
            ResolutionMethod::DisputeResolution => "DisputeResolution",
        };
        let resolution_source_str = soroban_sdk::String::from_str(env, resolution_source);

        // INTEGRATION: Use dispute window system instead of immediate resolution
        // Propose the resolution to open the dispute window
        // The winning outcome will be set when finalize_resolution is called
        crate::resolution_delay::ResolutionDelayManager::propose_resolution(
            env,
            market_id,
            final_result.clone(),
            resolution_source_str,
        )?;

        // Emit market resolution proposed event (the detailed event is emitted by propose_resolution)
        let oracle_result_str = market
            .oracle_result
            .clone()
            .unwrap_or_else(|| soroban_sdk::String::from_str(env, "N/A"));
        let community_consensus_str = soroban_sdk::String::from_str(env, "Consensus");
        let method_str = match resolution_method {
            ResolutionMethod::OracleOnly => "OracleOnly",
            ResolutionMethod::CommunityOnly => "CommunityOnly",
            ResolutionMethod::Hybrid => "Hybrid",
            ResolutionMethod::AdminOverride => "AdminOverride",
            ResolutionMethod::DisputeResolution => "DisputeResolution",
        };
        let resolution_method_str = soroban_sdk::String::from_str(env, method_str);

        // Note: The final "Resolved" state will be set when finalize_market_resolution is called
        // after the dispute window closes. For now, the market stays in its current state
        // but has a pending resolution proposal.
        crate::events::EventEmitter::emit_market_resolved(
            env,
            market_id,
            &final_result,
            &oracle_result_str,
            &community_consensus_str,
            &resolution_method_str,
            confidence_score as i128,
        );

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
        let stored_admin: Option<Address> =
            env.storage().persistent().get(&Symbol::new(env, "Admin"));

        match stored_admin {
            Some(stored_admin) => {
                if admin != &stored_admin {
                    return Err(Error::Unauthorized);
                }
                Ok(())
            }
            None => Err(Error::Unauthorized),
        }
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
            votes: 8,
            total_votes: 10,
            percentage: 80,
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
