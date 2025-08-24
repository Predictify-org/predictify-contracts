use soroban_sdk::{contracttype, symbol_short, vec, Address, Env, Map, String, Symbol, Vec};

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
// Note: These constants are now managed by the config module
// Use ConfigManager::get_fee_config() to get current values
///   Platform fee percentage (2%)
pub const PLATFORM_FEE_PERCENTAGE: i128 = crate::config::DEFAULT_PLATFORM_FEE_PERCENTAGE;

/// Market creation fee (1 XLM = 10,000,000 stroops)
pub const MARKET_CREATION_FEE: i128 = crate::config::DEFAULT_MARKET_CREATION_FEE;

/// Minimum fee amount (0.1 XLM)
pub const MIN_FEE_AMOUNT: i128 = crate::config::MIN_FEE_AMOUNT;

/// Maximum fee amount (100 XLM)
pub const MAX_FEE_AMOUNT: i128 = crate::config::MAX_FEE_AMOUNT;

/// Fee collection threshold (minimum amount before fees can be collected)
pub const FEE_COLLECTION_THRESHOLD: i128 = crate::config::FEE_COLLECTION_THRESHOLD; // 10 XLM

// ===== DYNAMIC FEE CONSTANTS =====

/// Maximum fee percentage (5%)
pub const MAX_FEE_PERCENTAGE: i128 = 500; // 5.00% in basis points

/// Minimum fee percentage (0.1%)
pub const MIN_FEE_PERCENTAGE: i128 = 10; // 0.10% in basis points

/// Activity level thresholds
pub const ACTIVITY_LEVEL_LOW: u32 = 10; // 10 votes
pub const ACTIVITY_LEVEL_MEDIUM: u32 = 50; // 50 votes
pub const ACTIVITY_LEVEL_HIGH: u32 = 100; // 100 votes

/// Market size tiers (in XLM)
pub const MARKET_SIZE_SMALL: i128 = 100_000_000; // 10 XLM
pub const MARKET_SIZE_MEDIUM: i128 = 1_000_000_000; // 100 XLM
pub const MARKET_SIZE_LARGE: i128 = 10_000_000_000; // 1000 XLM

// ===== FEE TYPES =====

/// Comprehensive fee configuration structure for market operations.
///
/// This structure defines all fee-related parameters that govern how fees are
/// calculated, collected, and managed across the Predictify Hybrid platform.
/// It provides flexible configuration for different market types and economic models.
///
/// # Fee Structure
///
/// The fee system supports multiple fee types:
/// - **Platform Fees**: Percentage-based fees on market stakes
/// - **Creation Fees**: Fixed fees for creating new markets
/// - **Collection Thresholds**: Minimum amounts before fee collection
/// - **Fee Limits**: Minimum and maximum fee boundaries
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::Env;
/// # use predictify_hybrid::fees::FeeConfig;
/// # let env = Env::default();
///
/// // Standard fee configuration
/// let config = FeeConfig {
///     platform_fee_percentage: 200, // 2.00% (basis points)
///     creation_fee: 10_000_000, // 1.0 XLM
///     min_fee_amount: 1_000_000, // 0.1 XLM minimum
///     max_fee_amount: 1_000_000_000, // 100 XLM maximum
///     collection_threshold: 100_000_000, // 10 XLM threshold
///     fees_enabled: true,
/// };
///
/// // Calculate platform fee for 50 XLM stake
/// let stake_amount = 500_000_000; // 50 XLM
/// let platform_fee = (stake_amount * config.platform_fee_percentage) / 10_000;
/// println!("Platform fee: {} XLM", platform_fee / 10_000_000);
///
/// // Check if fees are collectible
/// if config.fees_enabled && stake_amount >= config.collection_threshold {
///     println!("Fees can be collected");
/// }
/// ```
///
/// # Configuration Parameters
///
/// - **platform_fee_percentage**: Fee percentage in basis points (100 = 1%)
/// - **creation_fee**: Fixed fee for creating new markets (in stroops)
/// - **min_fee_amount**: Minimum fee that can be charged (prevents dust)
/// - **max_fee_amount**: Maximum fee that can be charged (prevents abuse)
/// - **collection_threshold**: Minimum total stakes before fees can be collected
/// - **fees_enabled**: Global fee system enable/disable flag
///
/// # Economic Model
///
/// Fee configuration supports platform sustainability:
/// - **Revenue Generation**: Platform fees support ongoing operations
/// - **Spam Prevention**: Creation fees prevent market spam
/// - **Fair Pricing**: Configurable limits ensure reasonable fee levels
/// - **Flexible Economics**: Adjustable parameters for different market conditions
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

/// Dynamic fee tier configuration based on market size
///
/// This structure defines fee tiers for different market sizes, allowing
/// for more granular fee structures based on the total amount staked.
/// Larger markets can have different fee rates to reflect their complexity
/// and resource requirements.
///
/// # Fee Tiers
///
/// - **Small Markets** (0-10 XLM): Lower fees for accessibility
/// - **Medium Markets** (10-100 XLM): Standard fees for typical markets
/// - **Large Markets** (100-1000 XLM): Higher fees for complex markets
/// - **Enterprise Markets** (1000+ XLM): Premium fees for large-scale markets
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::Env;
/// # use predictify_hybrid::fees::FeeTier;
/// # let env = Env::default();
///
/// let tier = FeeTier {
///     min_size: 0,
///     max_size: 100_000_000, // 10 XLM
///     fee_percentage: 150, // 1.5%
///     tier_name: String::from_str(&env, "Small"),
/// };
/// ```
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeTier {
    /// Minimum market size for this tier (in stroops)
    pub min_size: i128,
    /// Maximum market size for this tier (in stroops)
    pub max_size: i128,
    /// Fee percentage for this tier (in basis points)
    pub fee_percentage: i128,
    /// Tier name/description
    pub tier_name: String,
}

/// Activity-based fee adjustment configuration
///
/// This structure defines how fees are adjusted based on market activity
/// levels. Higher activity markets may have different fee structures to
/// account for increased resource usage and complexity.
///
/// # Activity Levels
///
/// - **Low Activity** (0-10 votes): Standard fees
/// - **Medium Activity** (10-50 votes): Slight fee adjustment
/// - **High Activity** (50-100 votes): Moderate fee adjustment
/// - **Very High Activity** (100+ votes): Significant fee adjustment
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::Env;
/// # use predictify_hybrid::fees::ActivityAdjustment;
/// # let env = Env::default();
///
/// let adjustment = ActivityAdjustment {
///     activity_level: 50,
///     fee_multiplier: 110, // 10% increase
///     description: String::from_str(&env, "Medium Activity"),
/// };
/// ```
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActivityAdjustment {
    /// Activity level threshold (number of votes)
    pub activity_level: u32,
    /// Fee multiplier (100 = no change, 110 = 10% increase)
    pub fee_multiplier: i128,
    /// Description of this activity level
    pub description: String,
}

/// Dynamic fee calculation factors
///
/// This structure contains all the factors that influence dynamic fee
/// calculation, including market size, activity level, and any special
/// considerations for the specific market.
///
/// # Calculation Factors
///
/// - **Base Fee**: Starting fee percentage
/// - **Size Multiplier**: Adjustment based on market size
/// - **Activity Multiplier**: Adjustment based on activity level
/// - **Complexity Factor**: Additional adjustment for market complexity
/// - **Final Fee**: Calculated final fee percentage
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::Env;
/// # use predictify_hybrid::fees::FeeCalculationFactors;
/// # let env = Env::default();
///
/// let factors = FeeCalculationFactors {
///     base_fee_percentage: 200, // 2%
///     size_multiplier: 110, // 10% increase
///     activity_multiplier: 105, // 5% increase
///     complexity_factor: 100, // No complexity adjustment
///     final_fee_percentage: 231, // 2.31% (calculated)
///     market_size_tier: String::from_str(&env, "Medium"),
///     activity_level: String::from_str(&env, "High"),
/// };
/// ```
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeCalculationFactors {
    /// Base fee percentage (in basis points)
    pub base_fee_percentage: i128,
    /// Size-based multiplier (100 = no change)
    pub size_multiplier: i128,
    /// Activity-based multiplier (100 = no change)
    pub activity_multiplier: i128,
    /// Complexity factor (100 = no change)
    pub complexity_factor: i128,
    /// Final calculated fee percentage (in basis points)
    pub final_fee_percentage: i128,
    /// Market size tier name
    pub market_size_tier: String,
    /// Activity level description
    pub activity_level: String,
}

/// Fee history record for tracking fee changes
///
/// This structure tracks the history of fee calculations and changes
/// for transparency and audit purposes.
///
/// # History Tracking
///
/// - **Fee Changes**: When and why fees were adjusted
/// - **Calculation Records**: How fees were calculated
/// - **Admin Actions**: Who made fee changes and when
/// - **Market Performance**: How fees performed over time
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::Env;
/// # use predictify_hybrid::fees::FeeHistory;
/// # let env = Env::default();
///
/// let history = FeeHistory {
///     market_id: Symbol::new(&env, "market_123"),
///     timestamp: env.ledger().timestamp(),
///     old_fee_percentage: 200, // 2%
///     new_fee_percentage: 220, // 2.2%
///     reason: String::from_str(&env, "Activity level increased"),
///     admin: Address::generate(&env),
///     calculation_factors: factors, // FeeCalculationFactors
/// };
/// ```
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeHistory {
    /// Market ID
    pub market_id: Symbol,
    /// Timestamp of the fee change
    pub timestamp: u64,
    /// Previous fee percentage
    pub old_fee_percentage: i128,
    /// New fee percentage
    pub new_fee_percentage: i128,
    /// Reason for the fee change
    pub reason: String,
    /// Admin who made the change
    pub admin: Address,
    /// Calculation factors used
    pub calculation_factors: FeeCalculationFactors,
}

/// Record of a completed fee collection operation from a market.
///
/// This structure maintains a complete audit trail of fee collection activities,
/// including the amount collected, who collected it, when it occurred, and the
/// fee parameters used. Essential for transparency and financial reporting.
///
/// # Collection Context
///
/// Each fee collection record captures:
/// - Market identification and collection amount
/// - Administrative details and timing
/// - Fee calculation parameters used
/// - Complete audit trail for compliance
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Address, Symbol};
/// # use predictify_hybrid::fees::FeeCollection;
/// # let env = Env::default();
/// # let admin = Address::generate(&env);
///
/// // Fee collection record
/// let collection = FeeCollection {
///     market_id: Symbol::new(&env, "btc_prediction"),
///     amount: 5_000_000, // 0.5 XLM collected
///     collected_by: admin.clone(),
///     timestamp: env.ledger().timestamp(),
///     fee_percentage: 200, // 2% fee rate used
/// };
///
/// // Analyze collection details
/// println!("Fee Collection Report");
/// println!("Market: {}", collection.market_id.to_string());
/// println!("Amount: {} XLM", collection.amount / 10_000_000);
/// println!("Collected by: {}", collection.collected_by.to_string());
/// println!("Fee rate: {}%", collection.fee_percentage as f64 / 100.0);
///
/// // Calculate original stake from fee
/// let original_stake = (collection.amount * 10_000) / collection.fee_percentage;
/// println!("Original stake: {} XLM", original_stake / 10_000_000);
/// ```
///
/// # Audit Trail Features
///
/// Fee collection records provide:
/// - **Complete Traceability**: Full record of who collected what and when
/// - **Financial Reporting**: Data for revenue tracking and analysis
/// - **Compliance Support**: Audit trails for regulatory requirements
/// - **Transparency**: Public record of all fee collection activities
///
/// # Integration Applications
///
/// - **Financial Dashboards**: Display fee collection history and trends
/// - **Audit Systems**: Maintain compliance and verification records
/// - **Analytics**: Analyze fee collection patterns and efficiency
/// - **Reporting**: Generate financial reports and summaries
/// - **Transparency**: Provide public access to fee collection data
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

/// Comprehensive analytics and statistics for the fee system.
///
/// This structure aggregates fee collection data across all markets to provide
/// insights into platform economics, fee efficiency, and revenue patterns.
/// Essential for business intelligence and platform optimization.
///
/// # Analytics Scope
///
/// Fee analytics encompass:
/// - Total fee collection across all markets
/// - Market participation and fee distribution
/// - Historical trends and collection patterns
/// - Performance metrics and efficiency indicators
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Map, String, Vec};
/// # use predictify_hybrid::fees::{FeeAnalytics, FeeCollection};
/// # let env = Env::default();
///
/// // Fee analytics example
/// let analytics = FeeAnalytics {
///     total_fees_collected: 1_000_000_000, // 100 XLM total
///     markets_with_fees: 25, // 25 markets have collected fees
///     average_fee_per_market: 40_000_000, // 4 XLM average
///     collection_history: Vec::new(&env), // Historical records
///     fee_distribution: Map::new(&env), // Distribution by market size
/// };
///
/// // Display analytics summary
/// println!("Fee System Analytics");
/// println!("═══════════════════════════════════════");
/// println!("Total fees collected: {} XLM",
///     analytics.total_fees_collected / 10_000_000);
/// println!("Markets with fees: {}", analytics.markets_with_fees);
/// println!("Average per market: {} XLM",
///     analytics.average_fee_per_market / 10_000_000);
///
/// // Calculate fee collection rate
/// if analytics.markets_with_fees > 0 {
///     let collection_efficiency = (analytics.markets_with_fees as f64 / 100.0) * 100.0;
///     println!("Collection efficiency: {:.1}%", collection_efficiency);
/// }
///
/// // Analyze fee distribution
/// println!("Fee distribution by market category:");
/// for (category, amount) in analytics.fee_distribution.iter() {
///     println!("  {}: {} XLM",
///         category.to_string(),
///         amount / 10_000_000);
/// }
/// ```
///
/// # Key Metrics
///
/// - **total_fees_collected**: Cumulative fees across all markets
/// - **markets_with_fees**: Number of markets that have generated fees
/// - **average_fee_per_market**: Mean fee collection per participating market
/// - **collection_history**: Chronological record of all fee collections
/// - **fee_distribution**: Breakdown of fees by market categories or sizes
///
/// # Business Intelligence
///
/// Analytics enable strategic insights:
/// - **Revenue Tracking**: Monitor platform income and growth
/// - **Market Performance**: Identify high-performing market categories
/// - **Efficiency Analysis**: Measure fee collection effectiveness
/// - **Trend Analysis**: Track fee patterns over time
/// - **Optimization**: Identify opportunities for fee structure improvements
///
/// # Integration Applications
///
/// - **Executive Dashboards**: High-level platform performance metrics
/// - **Financial Reporting**: Revenue analysis and forecasting
/// - **Market Analysis**: Understand which markets generate most fees
/// - **Performance Monitoring**: Track fee system health and efficiency
/// - **Strategic Planning**: Data-driven decisions for fee structure changes
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

/// Result of fee validation operations with detailed feedback and suggestions.
///
/// This structure provides comprehensive validation results for fee calculations,
/// including validity status, specific error messages, suggested corrections,
/// and detailed breakdowns. Essential for ensuring fee accuracy and compliance.
///
/// # Validation Scope
///
/// Fee validation covers:
/// - Fee amount validity and limits
/// - Calculation accuracy and consistency
/// - Configuration compliance
/// - Suggested optimizations and corrections
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, String, Vec};
/// # use predictify_hybrid::fees::{FeeValidationResult, FeeBreakdown};
/// # let env = Env::default();
///
/// // Fee validation result example
/// let validation = FeeValidationResult {
///     is_valid: false,
///     errors: vec![
///         &env,
///         String::from_str(&env, "Fee amount exceeds maximum limit"),
///         String::from_str(&env, "Market stake below collection threshold")
///     ],
///     suggested_amount: 50_000_000, // 5.0 XLM suggested
///     breakdown: FeeBreakdown {
///         total_staked: 1_000_000_000, // 100 XLM
///         fee_percentage: 200, // 2%
///         fee_amount: 20_000_000, // 2 XLM
///         platform_fee: 20_000_000, // 2 XLM
///         user_payout_amount: 980_000_000, // 98 XLM
///     },
/// };
///
/// // Process validation results
/// if validation.is_valid {
///     println!("Fee validation passed");
///     println!("Fee amount: {} XLM", validation.breakdown.fee_amount / 10_000_000);
/// } else {
///     println!("Fee validation failed:");
///     for error in validation.errors.iter() {
///         println!("  - {}", error.to_string());
///     }
///     println!("Suggested amount: {} XLM",
///         validation.suggested_amount / 10_000_000);
/// }
/// ```
///
/// # Validation Features
///
/// - **is_valid**: Boolean indicating overall validation status
/// - **errors**: Detailed list of validation issues found
/// - **suggested_amount**: Recommended fee amount if current is invalid
/// - **breakdown**: Complete fee calculation breakdown for transparency
///
/// # Error Categories
///
/// Common validation errors:
/// - **Amount Limits**: Fee exceeds minimum or maximum bounds
/// - **Calculation Errors**: Mathematical inconsistencies in fee computation
/// - **Configuration Issues**: Fee parameters don't match current config
/// - **Threshold Violations**: Stakes below collection thresholds
///
/// # Integration Applications
///
/// - **UI Feedback**: Display validation errors and suggestions to users
/// - **API Responses**: Provide detailed validation results in API calls
/// - **Automated Correction**: Use suggested amounts for automatic fixes
/// - **Compliance Checking**: Ensure fees meet regulatory requirements
/// - **Quality Assurance**: Validate fee calculations before processing
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

/// Detailed breakdown of fee calculations for complete transparency.
///
/// This structure provides a comprehensive breakdown of how fees are calculated
/// from the total staked amount, showing each component of the fee calculation
/// and the final amounts. Essential for transparency and user understanding.
///
/// # Breakdown Components
///
/// Fee breakdown includes:
/// - Original stake amounts and fee percentages
/// - Calculated fee amounts and platform fees
/// - Final user payout amounts after fee deduction
/// - Complete calculation transparency
///
/// # Example Usage
///
/// ```rust
/// # use predictify_hybrid::fees::FeeBreakdown;
///
/// // Fee breakdown for 100 XLM stake at 2% fee
/// let breakdown = FeeBreakdown {
///     total_staked: 1_000_000_000, // 100 XLM total stake
///     fee_percentage: 200, // 2.00% fee rate
///     fee_amount: 20_000_000, // 2 XLM fee
///     platform_fee: 20_000_000, // 2 XLM platform fee
///     user_payout_amount: 980_000_000, // 98 XLM after fees
/// };
///
/// // Display breakdown to user
/// println!("Fee Calculation Breakdown");
/// println!("─────────────────────────────────────────");
/// println!("Total Staked: {} XLM", breakdown.total_staked / 10_000_000);
/// println!("Fee Rate: {}%", breakdown.fee_percentage as f64 / 100.0);
/// println!("Fee Amount: {} XLM", breakdown.fee_amount / 10_000_000);
/// println!("Platform Fee: {} XLM", breakdown.platform_fee / 10_000_000);
/// println!("User Payout: {} XLM", breakdown.user_payout_amount / 10_000_000);
///
/// // Verify calculation accuracy
/// let expected_fee = (breakdown.total_staked * breakdown.fee_percentage) / 10_000;
/// assert_eq!(breakdown.fee_amount, expected_fee);
///
/// let expected_payout = breakdown.total_staked - breakdown.fee_amount;
/// assert_eq!(breakdown.user_payout_amount, expected_payout);
/// ```
///
/// # Calculation Transparency
///
/// The breakdown ensures users understand:
/// - **How fees are calculated**: Clear percentage-based calculation
/// - **What they pay**: Exact fee amounts in XLM
/// - **What they receive**: Net payout after fee deduction
/// - **Verification**: All calculations can be independently verified
///
/// # Use Cases
///
/// - **User Interfaces**: Display fee calculations before confirmation
/// - **API Responses**: Provide detailed fee information in responses
/// - **Audit Trails**: Maintain records of fee calculation details
/// - **Transparency**: Show users exactly how fees are computed
/// - **Validation**: Verify fee calculations are correct and consistent
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

/// Fee collection status and safety information
///
/// This structure provides comprehensive status information about fee collection
/// operations, including safety checks, validation results, and operational status.
/// Essential for monitoring and ensuring safe fee collection operations.
///
/// # Status Information
///
/// Fee collection status includes:
/// - Collection eligibility and readiness
/// - Safety validation results
/// - Risk assessment and warnings
/// - Operational status and recommendations
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Symbol};
/// # use predictify_hybrid::fees::FeeCollectionStatus;
/// # let env = Env::default();
///
/// // Get fee collection status for a market
/// let status = FeeCollectionStatus {
///     market_id: Symbol::new(&env, "btc_market"),
///     is_eligible: true,
///     safety_checks_passed: true,
///     risk_level: String::from_str(&env, "Low"),
///     warnings: Vec::new(&env),
///     recommendations: Vec::new(&env),
///     last_validation: env.ledger().timestamp(),
/// };
///
/// // Check if collection is safe
/// if status.is_eligible && status.safety_checks_passed {
///     println!("Fee collection is safe to proceed");
/// } else {
///     println!("Fee collection has safety concerns");
///     for warning in status.warnings.iter() {
///         println!("Warning: {}", warning.to_string());
///     }
/// }
/// ```
///
/// # Safety Features
///
/// Status provides safety information:
/// - **Eligibility Check**: Whether fees can be collected
/// - **Safety Validation**: All safety checks passed
/// - **Risk Assessment**: Current risk level
/// - **Warning System**: Any safety concerns
/// - **Recommendations**: Suggested actions
///
/// # Integration Applications
///
/// - **UI Safety Indicators**: Display safety status to users
/// - **Automated Monitoring**: Monitor collection safety
/// - **Risk Management**: Assess collection risks
/// - **Compliance Checking**: Ensure regulatory compliance
/// - **Operational Safety**: Prevent unsafe operations
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeCollectionStatus {
    /// Market ID
    pub market_id: Symbol,
    /// Whether fees can be collected
    pub is_eligible: bool,
    /// Whether all safety checks passed
    pub safety_checks_passed: bool,
    /// Risk level assessment
    pub risk_level: String,
    /// Safety warnings
    pub warnings: Vec<String>,
    /// Recommendations
    pub recommendations: Vec<String>,
    /// Last validation timestamp
    pub last_validation: u64,
}

/// Fee distribution tracking record
///
/// This structure tracks how fees are distributed across different recipients,
/// providing transparency and auditability for fee distribution operations.
/// Essential for financial reporting and compliance.
///
/// # Distribution Tracking
///
/// Fee distribution includes:
/// - Recipient addresses and amounts
/// - Distribution percentages and totals
/// - Timestamp and transaction details
/// - Distribution verification status
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Address, Map};
/// # use predictify_hybrid::fees::FeeDistribution;
/// # let env = Env::default();
///
/// // Create fee distribution record
/// let mut distribution = Map::new(&env);
/// distribution.set(Address::generate(&env), 50_000_000); // 5 XLM
/// distribution.set(Address::generate(&env), 30_000_000); // 3 XLM
///
/// let fee_distribution = FeeDistribution {
///     market_id: Symbol::new(&env, "btc_market"),
///     total_amount: 80_000_000, // 8 XLM total
///     distribution: distribution,
///     distribution_timestamp: env.ledger().timestamp(),
///     verified: true,
///     verification_timestamp: env.ledger().timestamp(),
/// };
///
/// // Verify distribution totals
/// let calculated_total: i128 = fee_distribution.distribution.values().sum();
/// assert_eq!(fee_distribution.total_amount, calculated_total);
/// ```
///
/// # Audit Features
///
/// Distribution provides audit capabilities:
/// - **Complete Tracking**: Full record of all distributions
/// - **Verification Status**: Whether distribution was verified
/// - **Timestamp Records**: When distribution occurred
/// - **Total Validation**: Ensure amounts match totals
/// - **Recipient Tracking**: Track all fee recipients
///
/// # Integration Applications
///
/// - **Financial Reporting**: Generate distribution reports
/// - **Audit Trails**: Maintain compliance records
/// - **Transparency**: Provide public distribution data
/// - **Verification**: Validate distribution accuracy
/// - **Compliance**: Meet regulatory requirements
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeDistribution {
    /// Market ID
    pub market_id: Symbol,
    /// Total amount distributed
    pub total_amount: i128,
    /// Distribution to recipients
    pub distribution: Map<Address, i128>,
    /// Distribution timestamp
    pub distribution_timestamp: u64,
    /// Whether distribution was verified
    pub verified: bool,
    /// Verification timestamp
    pub verification_timestamp: u64,
}

/// Fee refund record for error handling
///
/// This structure tracks fee refunds that occur due to errors or safety issues,
/// providing complete audit trails for refund operations and ensuring user protection.
///
/// # Refund Information
///
/// Fee refunds include:
/// - Refund amount and recipient
/// - Reason for refund
/// - Refund timestamp and status
/// - Error details and context
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Address, Symbol};
/// # use predictify_hybrid::fees::FeeRefund;
/// # let env = Env::default();
///
/// // Create fee refund record
/// let refund = FeeRefund {
///     market_id: Symbol::new(&env, "btc_market"),
///     recipient: Address::generate(&env),
///     amount: 25_000_000, // 2.5 XLM
///     reason: String::from_str(&env, "Safety validation failed"),
///     refund_timestamp: env.ledger().timestamp(),
///     status: String::from_str(&env, "Pending"),
///     error_code: 1001,
///     error_details: String::from_str(&env, "Fee collection safety check failed"),
/// };
///
/// // Process refund
/// println!("Refunding {} XLM to {}", 
///     refund.amount / 10_000_000, 
///     refund.recipient.to_string());
/// println!("Reason: {}", refund.reason.to_string());
/// ```
///
/// # Refund Features
///
/// Refunds provide user protection:
/// - **Error Recovery**: Automatic refunds on errors
/// - **Safety Protection**: Refunds for safety violations
/// - **Complete Tracking**: Full refund audit trail
/// - **Status Monitoring**: Track refund completion
/// - **Error Context**: Detailed error information
///
/// # Integration Applications
///
/// - **Error Handling**: Automatic refund processing
/// - **User Protection**: Ensure users get refunds
/// - **Audit Trails**: Maintain refund records
/// - **Compliance**: Meet refund requirements
/// - **Monitoring**: Track refund patterns
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeRefund {
    /// Market ID
    pub market_id: Symbol,
    /// Refund recipient
    pub recipient: Address,
    /// Refund amount
    pub amount: i128,
    /// Refund reason
    pub reason: String,
    /// Refund timestamp
    pub refund_timestamp: u64,
    /// Refund status
    pub status: String,
    /// Error code
    pub error_code: u32,
    /// Error details
    pub error_details: String,
}

/// Fee collection safety validation result
///
/// This structure provides comprehensive safety validation results for fee collection
/// operations, including risk assessments, safety checks, and recommendations.
/// Essential for ensuring safe and compliant fee collection.
///
/// # Safety Validation
///
/// Safety validation includes:
/// - Overall safety status
/// - Individual safety checks
/// - Risk assessments
/// - Safety recommendations
/// - Validation details
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Symbol, Vec};
/// # use predictify_hybrid::fees::FeeSafetyValidation;
/// # let env = Env::default();
///
/// // Create safety validation result
/// let mut safety_checks = Vec::new(&env);
/// safety_checks.push_back(String::from_str(&env, "Market state validation: PASSED"));
/// safety_checks.push_back(String::from_str(&env, "Fee amount validation: PASSED"));
/// safety_checks.push_back(String::from_str(&env, "Admin authorization: PASSED"));
///
/// let validation = FeeSafetyValidation {
///     market_id: Symbol::new(&env, "btc_market"),
///     is_safe: true,
///     safety_score: 95, // 95% safety score
///     safety_checks: safety_checks,
///     risk_factors: Vec::new(&env),
///     recommendations: Vec::new(&env),
///     validation_timestamp: env.ledger().timestamp(),
/// };
///
/// // Check safety status
/// if validation.is_safe && validation.safety_score >= 90 {
///     println!("Fee collection is safe to proceed");
/// } else {
///     println!("Fee collection has safety concerns");
///     for recommendation in validation.recommendations.iter() {
///         println!("Recommendation: {}", recommendation.to_string());
///     }
/// }
/// ```
///
/// # Safety Features
///
/// Safety validation provides:
/// - **Comprehensive Checks**: Multiple safety validations
/// - **Risk Assessment**: Identify potential risks
/// - **Safety Scoring**: Quantified safety assessment
/// - **Recommendations**: Suggested safety improvements
/// - **Validation Tracking**: Complete validation history
///
/// # Integration Applications
///
/// - **Safety Monitoring**: Monitor collection safety
/// - **Risk Management**: Assess and mitigate risks
/// - **Compliance**: Ensure regulatory compliance
/// - **User Protection**: Protect user interests
/// - **Operational Safety**: Prevent unsafe operations
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeSafetyValidation {
    /// Market ID
    pub market_id: Symbol,
    /// Whether collection is safe
    pub is_safe: bool,
    /// Safety score (0-100)
    pub safety_score: u32,
    /// Safety check results
    pub safety_checks: Vec<String>,
    /// Risk factors identified
    pub risk_factors: Vec<String>,
    /// Safety recommendations
    pub recommendations: Vec<String>,
    /// Validation timestamp
    pub validation_timestamp: u64,
}

// ===== FEE MANAGER =====

/// Comprehensive fee management system for the Predictify Hybrid platform.
///
/// The FeeManager provides centralized fee operations including collection,
/// calculation, validation, and configuration management. It handles all
/// fee-related operations with proper authentication, validation, and transparency.
///
/// # Core Responsibilities
///
/// - **Fee Collection**: Collect platform fees from resolved markets
/// - **Fee Processing**: Handle market creation and operation fees
/// - **Configuration Management**: Update and retrieve fee configurations
/// - **Analytics**: Generate fee analytics and performance metrics
/// - **Validation**: Ensure fee calculations are accurate and compliant
///
/// # Fee Operations
///
/// The system supports multiple fee types:
/// - **Platform Fees**: Percentage-based fees on market stakes
/// - **Creation Fees**: Fixed fees for creating new markets
/// - **Collection Operations**: Automated fee collection from resolved markets
/// - **Configuration Updates**: Dynamic fee parameter adjustments
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Address, Symbol};
/// # use predictify_hybrid::fees::FeeManager;
/// # let env = Env::default();
/// # let admin = Address::generate(&env);
/// # let market_id = Symbol::new(&env, "btc_market");
///
/// // Collect fees from a resolved market
/// let collected_amount = FeeManager::collect_fees(
///     &env,
///     admin.clone(),
///     market_id.clone()
/// ).unwrap();
///
/// println!("Collected {} XLM in fees", collected_amount / 10_000_000);
///
/// // Get fee analytics
/// let analytics = FeeManager::get_fee_analytics(&env).unwrap();
/// println!("Total platform fees: {} XLM",
///     analytics.total_fees_collected / 10_000_000);
///
/// // Validate market fees
/// let validation = FeeManager::validate_market_fees(&env, &market_id).unwrap();
/// if validation.is_valid {
///     println!("Market fees are valid");
/// } else {
///     println!("Fee validation issues found");
/// }
/// ```
///
/// # Security and Authentication
///
/// Fee operations include:
/// - **Admin Authentication**: All fee operations require proper admin authentication
/// - **Permission Validation**: Verify admin has necessary permissions
/// - **Amount Validation**: Ensure fee amounts are within acceptable limits
/// - **State Validation**: Check market states before fee operations
///
/// # Economic Model
///
/// The fee system supports platform sustainability:
/// - **Revenue Generation**: Platform fees provide ongoing operational funding
/// - **Spam Prevention**: Creation fees prevent market spam and abuse
/// - **Fair Distribution**: Transparent fee calculation and collection
/// - **Configurable Economics**: Adjustable fee parameters for different conditions
///
/// # Integration Points
///
/// - **Market Resolution**: Automatic fee collection when markets resolve
/// - **Market Creation**: Fee processing during market creation
/// - **Administrative Tools**: Fee configuration and management interfaces
/// - **Analytics Dashboards**: Fee performance and revenue tracking
/// - **User Interfaces**: Fee display and transparency features
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
        MarketStateManager::mark_fees_collected(&mut market, Some(&market_id));
        MarketStateManager::update_market(env, &market_id, &market);

        Ok(fee_amount)
    }

    /// Process market creation fee
    pub fn process_creation_fee(env: &Env, admin: &Address) -> Result<(), Error> {
        // Note: Authentication is handled at the contract entry point level
        // No need to call require_auth() again here

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
    pub fn validate_market_fees(
        env: &Env,
        market_id: &Symbol,
    ) -> Result<FeeValidationResult, Error> {
        let market = MarketStateManager::get_market(env, market_id)?;

        // Always return a validation result, even if there are issues
        match FeeValidator::validate_market_fees(&market) {
            Ok(result) => Ok(result),
            Err(_) => {
                // If validation fails, return a validation result indicating failure
                let mut errors = Vec::new(env);
                errors.push_back(String::from_str(env, "Market validation failed"));

                // Create a default breakdown for failed validation
                let default_breakdown = FeeBreakdown {
                    total_staked: 0,
                    fee_percentage: PLATFORM_FEE_PERCENTAGE,
                    fee_amount: 0,
                    platform_fee: 0,
                    user_payout_amount: 0,
                };

                Ok(FeeValidationResult {
                    is_valid: false,
                    errors,
                    suggested_amount: 0,
                    breakdown: default_breakdown,
                })
            }
        }
    }

    /// Update fee structure with new fee tiers
    pub fn update_fee_structure(
        env: &Env,
        admin: Address,
        new_fee_tiers: Map<u32, i128>,
    ) -> Result<(), Error> {
        // Require authentication from the admin
        admin.require_auth();

        // Validate admin permissions
        FeeValidator::validate_admin_permissions(env, &admin)?;

        // Validate fee tiers
        for (tier_id, fee_percentage) in new_fee_tiers.iter() {
            if fee_percentage < MIN_FEE_PERCENTAGE || fee_percentage > MAX_FEE_PERCENTAGE {
                return Err(Error::InvalidInput);
            }
        }

        // Store new fee tiers
        let storage_key = symbol_short!("fee_tiers");
        env.storage().persistent().set(&storage_key, &new_fee_tiers);

        // Record fee structure update
        FeeTracker::record_fee_structure_update(env, &admin, &new_fee_tiers)?;

        Ok(())
    }

    /// Get fee history for a specific market
    pub fn get_fee_history(env: &Env, market_id: Symbol) -> Result<Vec<FeeHistory>, Error> {
        let history_key = Symbol::new(env, "fee_history");

        match env
            .storage()
            .persistent()
            .get::<Symbol, Vec<FeeHistory>>(&history_key)
        {
            Some(history) => Ok(history),
            None => Ok(Vec::new(env)),
        }
    }

    /// Validate fee collection for a market with comprehensive safety checks
    pub fn validate_fee_collection(
        env: &Env,
        market_id: Symbol,
        fee_amount: i128,
    ) -> Result<FeeValidationResult, Error> {
        // Get market
        let market = MarketStateManager::get_market(env, &market_id)?;

        // Perform comprehensive validation
        let mut errors = Vec::new(env);
        let mut is_valid = true;

        // Check market state
        if market.winning_outcome.is_none() {
            errors.push_back(String::from_str(env, "Market not resolved"));
            is_valid = false;
        }

        // Check if fees already collected
        if market.fee_collected {
            errors.push_back(String::from_str(env, "Fees already collected"));
            is_valid = false;
        }

        // Check fee amount validity
        if fee_amount < MIN_FEE_AMOUNT {
            errors.push_back(String::from_str(env, "Fee amount below minimum"));
            is_valid = false;
        }

        if fee_amount > MAX_FEE_AMOUNT {
            errors.push_back(String::from_str(env, "Fee amount exceeds maximum"));
            is_valid = false;
        }

        // Check if market has sufficient stakes
        if market.total_staked < FEE_COLLECTION_THRESHOLD {
            errors.push_back(String::from_str(env, "Insufficient stakes for fee collection"));
            is_valid = false;
        }

        // Calculate fee breakdown
        let breakdown = FeeCalculator::calculate_fee_breakdown(&market)?;
        let suggested_amount = breakdown.fee_amount;

        // Validate calculated fee matches provided fee
        if fee_amount != suggested_amount {
            errors.push_back(String::from_str(env, "Fee amount does not match calculated amount"));
            is_valid = false;
        }

        Ok(FeeValidationResult {
            is_valid,
            errors,
            suggested_amount,
            breakdown,
        })
    }

    /// Track fee distribution across recipients
    pub fn track_fee_distribution(
        env: &Env,
        market_id: Symbol,
        distribution: Map<Address, i128>,
    ) -> Result<FeeDistribution, Error> {
        // Validate distribution
        FeeValidator::validate_fee_distribution(&distribution)?;

        // Calculate total amount
        let mut total_amount: i128 = 0;
        for (_, amount) in distribution.iter() {
            total_amount += amount;
        }

        // Validate total amount
        if total_amount <= 0 {
            return Err(Error::InvalidInput);
        }

        // Create distribution record
        let fee_distribution = FeeDistribution {
            market_id: market_id.clone(),
            total_amount,
            distribution: distribution.clone(),
            distribution_timestamp: env.ledger().timestamp(),
            verified: false,
            verification_timestamp: 0,
        };

        // Store distribution record
        let distribution_key = symbol_short!("fee_dist");
        let mut distributions: Vec<FeeDistribution> = env
            .storage()
            .persistent()
            .get(&distribution_key)
            .unwrap_or(vec![env]);

        distributions.push_back(fee_distribution.clone());
        env.storage().persistent().set(&distribution_key, &distributions);

        // Emit distribution tracking event
        FeeTracker::emit_fee_distribution_tracked(env, &market_id, &distribution, total_amount)?;

        Ok(fee_distribution)
    }

    /// Verify fee collection safety with comprehensive checks
    pub fn verify_fee_collection_safety(
        env: &Env,
        market_id: Symbol,
    ) -> Result<FeeSafetyValidation, Error> {
        // Get market
        let market = MarketStateManager::get_market(env, &market_id)?;

        // Perform safety checks
        let mut safety_checks = Vec::new(env);
        let mut risk_factors = Vec::new(env);
        let mut recommendations = Vec::new(env);
        let mut safety_score = 100;

        // Check market state
        if market.winning_outcome.is_some() {
            safety_checks.push_back(String::from_str(env, "Market state validation: PASSED"));
        } else {
            safety_checks.push_back(String::from_str(env, "Market state validation: FAILED"));
            risk_factors.push_back(String::from_str(env, "Market not resolved"));
            recommendations.push_back(String::from_str(env, "Wait for market resolution"));
            safety_score -= 30;
        }

        // Check fee collection status
        if !market.fee_collected {
            safety_checks.push_back(String::from_str(env, "Fee collection status: PASSED"));
        } else {
            safety_checks.push_back(String::from_str(env, "Fee collection status: FAILED"));
            risk_factors.push_back(String::from_str(env, "Fees already collected"));
            recommendations.push_back(String::from_str(env, "Fees cannot be collected again"));
            safety_score -= 50;
        }

        // Check stake threshold
        if market.total_staked >= FEE_COLLECTION_THRESHOLD {
            safety_checks.push_back(String::from_str(env, "Stake threshold: PASSED"));
        } else {
            safety_checks.push_back(String::from_str(env, "Stake threshold: FAILED"));
            risk_factors.push_back(String::from_str(env, "Insufficient stakes"));
            recommendations.push_back(String::from_str(env, "Wait for more stakes"));
            safety_score -= 20;
        }

        // Check market age (use end_time as creation time approximation)
        let current_time = env.ledger().timestamp();
        let market_age = current_time - market.end_time;
        let min_market_age = 3600; // 1 hour minimum

        if market_age >= min_market_age {
            safety_checks.push_back(String::from_str(env, "Market age validation: PASSED"));
        } else {
            safety_checks.push_back(String::from_str(env, "Market age validation: FAILED"));
            risk_factors.push_back(String::from_str(env, "Market too new"));
            recommendations.push_back(String::from_str(env, "Wait for market to mature"));
            safety_score -= 10;
        }

        let is_safe = safety_score >= 80;

        Ok(FeeSafetyValidation {
            market_id,
            is_safe,
            safety_score,
            safety_checks,
            risk_factors,
            recommendations,
            validation_timestamp: current_time,
        })
    }

    /// Emit comprehensive fee collection event
    pub fn emit_fee_collection_event(
        env: &Env,
        market_id: Symbol,
        fee_amount: i128,
    ) -> Result<(), Error> {
        // Get market for additional context
        let market = MarketStateManager::get_market(env, &market_id)?;

        // Emit fee collection event
        use crate::events::EventEmitter;
        EventEmitter::emit_fee_collected(
            env,
            &market_id,
            &env.current_contract_address(),
            fee_amount,
            &String::from_str(env, "Platform Fee"),
        );

        // Emit performance metric
        EventEmitter::emit_performance_metric(
            env,
            &String::from_str(env, "fee_coll_amt"),
            fee_amount,
            &String::from_str(env, "stroops"),
            &String::from_str(env, "Fee collection completed"),
        );

        // Log fee collection analytics
        FeeTracker::record_fee_collection_analytics(env, &market_id, fee_amount)?;

        Ok(())
    }

    /// Refund fees on error with comprehensive tracking
    pub fn refund_fee_on_error(
        env: &Env,
        market_id: Symbol,
        fee_amount: i128,
        recipient: Address,
        error_code: u32,
        error_details: String,
    ) -> Result<FeeRefund, Error> {
        // Validate refund parameters
        if fee_amount <= 0 {
            return Err(Error::InvalidInput);
        }

        // Create refund record
        let refund = FeeRefund {
            market_id: market_id.clone(),
            recipient: recipient.clone(),
            amount: fee_amount,
            reason: String::from_str(env, "Fee collection error"),
            refund_timestamp: env.ledger().timestamp(),
            status: String::from_str(env, "Pending"),
            error_code,
            error_details: error_details.clone(),
        };

        // Store refund record
        let refund_key = symbol_short!("fee_ref");
        let mut refunds: Vec<FeeRefund> = env
            .storage()
            .persistent()
            .get(&refund_key)
            .unwrap_or(vec![env]);

        refunds.push_back(refund.clone());
        env.storage().persistent().set(&refund_key, &refunds);

        // Transfer refund amount
        FeeUtils::transfer_fees_to_admin(env, &recipient, fee_amount)?;

        // Update refund status
        let mut updated_refund = refund.clone();
        updated_refund.status = String::from_str(env, "Completed");

        // Emit refund event
        use crate::events::EventEmitter;
        EventEmitter::emit_error_logged(
            env,
            error_code,
            &String::from_str(env, "Fee refund processed"),
            &error_details,
            Some(recipient),
            Some(market_id.clone()),
        );

        // Log refund analytics
        FeeTracker::record_fee_refund_analytics(env, &market_id, fee_amount)?;

        Ok(updated_refund)
    }

    /// Get comprehensive fee collection status
    pub fn get_fee_collection_status(
        env: &Env,
        market_id: Symbol,
    ) -> Result<FeeCollectionStatus, Error> {
        // Get market
        let market = MarketStateManager::get_market(env, &market_id)?;

        // Perform safety validation
        let safety_validation = Self::verify_fee_collection_safety(env, market_id.clone())?;

        // Determine eligibility
        let is_eligible = market.winning_outcome.is_some()
            && !market.fee_collected
            && market.total_staked >= FEE_COLLECTION_THRESHOLD;

        // Generate warnings and recommendations
        let mut warnings = Vec::new(env);
        let mut recommendations = Vec::new(env);

        if market.winning_outcome.is_none() {
            warnings.push_back(String::from_str(env, "Market not yet resolved"));
            recommendations.push_back(String::from_str(env, "Wait for market resolution"));
        }

        if market.fee_collected {
            warnings.push_back(String::from_str(env, "Fees already collected"));
            recommendations.push_back(String::from_str(env, "No further action needed"));
        }

        if market.total_staked < FEE_COLLECTION_THRESHOLD {
            warnings.push_back(String::from_str(env, "Insufficient stakes for fee collection"));
            recommendations.push_back(String::from_str(env, "Wait for more stakes"));
        }

        if safety_validation.safety_score < 80 {
            warnings.push_back(String::from_str(env, "Safety score below threshold"));
            recommendations.push_back(String::from_str(env, "Review safety validation results"));
        }

        // Add safety validation recommendations
        for recommendation in safety_validation.recommendations.iter() {
            recommendations.push_back(recommendation.clone());
        }

        Ok(FeeCollectionStatus {
            market_id,
            is_eligible,
            safety_checks_passed: safety_validation.is_safe,
            risk_level: if safety_validation.safety_score >= 90 {
                String::from_str(env, "Low")
            } else if safety_validation.safety_score >= 70 {
                String::from_str(env, "Medium")
            } else if safety_validation.safety_score >= 50 {
                String::from_str(env, "High")
            } else {
                String::from_str(env, "Critical")
            },
            warnings,
            recommendations,
            last_validation: env.ledger().timestamp(),
        })
    }

    /// Validate fee distribution for accuracy and compliance
    pub fn validate_fee_distribution(
        env: &Env,
        distribution: &Map<Address, i128>,
    ) -> Result<bool, Error> {
        // Check if distribution is empty
        if distribution.is_empty() {
            return Err(Error::InvalidInput);
        }

        // Validate each distribution entry
        for (recipient, amount) in distribution.iter() {
            // Check amount validity
            if amount <= 0 {
                return Err(Error::InvalidInput);
            }

            // Check for reasonable amount limits
            if amount > MAX_FEE_AMOUNT {
                return Err(Error::InvalidInput);
            }
        }

        // Calculate and validate total
        let mut total_amount: i128 = 0;
        for (_, amount) in distribution.iter() {
            total_amount += amount;
        }
        
        if total_amount <= 0 {
            return Err(Error::InvalidInput);
        }

        // Check for reasonable total limits
        if total_amount > MAX_FEE_AMOUNT * 10 {
            return Err(Error::InvalidInput);
        }

        Ok(true)
    }

    /// Distribute fees to multiple parties
    pub fn distribute_fees_to_multiple_parties(
        env: &Env,
        admin: Address,
        market_id: Symbol,
        distribution: Map<Address, i128>,
    ) -> Result<FeeDistributionExecution, Error> {
        // Require authentication from the admin
        admin.require_auth();

        // Validate admin permissions
        FeeValidator::validate_admin_permissions(env, &admin)?;

        // Validate distribution
        Self::validate_fee_distribution(env, &distribution)?;

        // Get market for validation
        let market = MarketStateManager::get_market(env, &market_id)?;
        
        // Validate market state for fee distribution
        if market.winning_outcome.is_none() {
            return Err(Error::MarketNotResolved);
        }

        if !market.fee_collected {
            return Err(Error::FeeAlreadyCollected);
        }

        // Calculate total distribution amount
        let mut total_amount: i128 = 0;
        for (_, amount) in distribution.iter() {
            total_amount += amount;
        }

        // Validate total amount
        if total_amount <= 0 {
            return Err(Error::InvalidInput);
        }

        // Create distribution execution record
        let execution = FeeDistributionExecution {
            market_id: market_id.clone(),
            total_fee_amount: total_amount,
            distribution: distribution.clone(),
            distribution_config_id: Symbol::new(env, "active_config"),
            executed_by: admin.clone(),
            execution_timestamp: env.ledger().timestamp(),
            verification_status: String::from_str(env, "Pending"),
            verification_timestamp: 0,
            verification_notes: String::from_str(env, "Awaiting verification"),
            success: true,
            error_message: None,
        };

        // Validate execution
        if !execution.is_valid() {
            return Err(Error::InvalidInput);
        }

        // Transfer fees to recipients
        Self::transfer_fees_to_recipients(env, &distribution)?;

        // Store execution record
        Self::store_distribution_execution(env, &execution)?;

        // Emit distribution event
        Self::emit_fee_distribution_event(env, &market_id, &distribution)?;

        Ok(execution)
    }

    /// Validate fee distribution percentages
    pub fn validate_fee_distribution_percentages(
        env: &Env,
        distribution: &Map<Address, i128>,
    ) -> Result<bool, Error> {
        // Check if distribution is empty
        if distribution.is_empty() {
            return Err(Error::InvalidInput);
        }

        // Calculate total percentage
        let mut total_percentage: i128 = 0;
        for (_, percentage) in distribution.iter() {
            // Validate percentage bounds
            if percentage < 0 || percentage > 100 {
                return Err(Error::InvalidInput);
            }
            total_percentage += percentage;
        }

        // Check if total equals 100%
        if total_percentage != 100 {
            return Err(Error::InvalidInput);
        }

        Ok(true)
    }

    /// Get fee distribution configuration
    pub fn get_fee_distribution_config(env: &Env) -> Result<FeeDistributionConfig, Error> {
        let config_key = symbol_short!("dist_cfg");
        match env.storage().persistent().get(&config_key) {
            Some(config) => Ok(config),
            None => {
                // Return default configuration
                let mut default_distribution = Map::new(env);
                let admin: Option<Address> = env.storage().persistent().get(&Symbol::new(env, "Admin"));
                
                if let Some(ref admin_address) = admin {
                    default_distribution.set(admin_address.clone(), 100); // 100% to admin
                }

                Ok(FeeDistributionConfig {
                    distribution: default_distribution,
                    total_percentage: 100,
                    governance_enabled: false,
                    community_participation: false,
                    min_distribution_percentage: 5,
                    max_distribution_percentage: 80,
                    distribution_name: String::from_str(env, "Default Distribution"),
                    created_by: admin.unwrap_or_else(|| Address::from_str(env, "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF")),
                    created_at: env.ledger().timestamp(),
                    is_active: true,
                })
            }
        }
    }

    /// Update fee distribution configuration
    pub fn update_fee_distribution_config(
        env: &Env,
        admin: Address,
        new_distribution: Map<Address, i128>,
    ) -> Result<FeeDistributionConfig, Error> {
        // Require authentication from the admin
        admin.require_auth();

        // Validate admin permissions
        FeeValidator::validate_admin_permissions(env, &admin)?;

        // Validate distribution percentages
        Self::validate_fee_distribution_percentages(env, &new_distribution)?;

        // Calculate total percentage
        let mut total_percentage: i128 = 0;
        for (_, percentage) in new_distribution.iter() {
            total_percentage += percentage;
        }

        // Create new configuration
        let config = FeeDistributionConfig {
            distribution: new_distribution,
            total_percentage,
            governance_enabled: true,
            community_participation: true,
            min_distribution_percentage: 5,
            max_distribution_percentage: 80,
            distribution_name: String::from_str(env, "Updated Distribution"),
            created_by: admin.clone(),
            created_at: env.ledger().timestamp(),
            is_active: true,
        };

        // Validate configuration
        if !config.is_valid() {
            return Err(Error::InvalidInput);
        }

        // Store configuration
        Self::store_distribution_config(env, &config)?;

        // Emit configuration update event
        Self::emit_distribution_config_updated_event(env, &admin, &config)?;

        Ok(config)
    }

    /// Emit fee distribution event
    pub fn emit_fee_distribution_event(
        env: &Env,
        market_id: &Symbol,
        distribution: &Map<Address, i128>,
    ) -> Result<(), Error> {
        // Calculate total amount
        let mut total_amount: i128 = 0;
        for (_, amount) in distribution.iter() {
            total_amount += amount;
        }

        // Emit distribution event
        use crate::events::EventEmitter;
        EventEmitter::emit_performance_metric(
            env,
            &String::from_str(env, "fee_dist_total"),
            total_amount,
            &String::from_str(env, "stroops"),
            &String::from_str(env, "Fee distribution completed"),
        );

        Ok(())
    }

    /// Track fee distribution history
    pub fn track_fee_distribution_history(
        env: &Env,
        market_id: Symbol,
    ) -> Result<Vec<FeeDistributionExecution>, Error> {
        let history_key = symbol_short!("dist_hist");
        let mut history: Vec<FeeDistributionExecution> = env
            .storage()
            .persistent()
            .get(&history_key)
            .unwrap_or(vec![env]);

        // Filter history for specific market
        let mut market_history = Vec::new(env);
        for execution in history.iter() {
            if execution.market_id == market_id {
                market_history.push_back(execution);
            }
        }

        Ok(market_history)
    }

    /// Validate distribution totals
    pub fn validate_distribution_totals(
        env: &Env,
        distribution: &Map<Address, i128>,
    ) -> Result<bool, Error> {
        // Check if distribution is empty
        if distribution.is_empty() {
            return Err(Error::InvalidInput);
        }

        // Calculate total amount
        let mut total_amount: i128 = 0;
        for (_, amount) in distribution.iter() {
            if amount <= 0 {
                return Err(Error::InvalidInput);
            }
            total_amount += amount;
        }

        // Validate total amount
        if total_amount <= 0 {
            return Err(Error::InvalidInput);
        }

        // Check for reasonable limits
        if total_amount > MAX_FEE_AMOUNT * 10 {
            return Err(Error::InvalidInput);
        }

        Ok(true)
    }

    // ===== PRIVATE HELPER METHODS =====

    /// Transfer fees to recipients
    fn transfer_fees_to_recipients(
        env: &Env,
        distribution: &Map<Address, i128>,
    ) -> Result<(), Error> {
        let token_client = MarketUtils::get_token_client(env)?;

        for (recipient, amount) in distribution.iter() {
            token_client.transfer(&env.current_contract_address(), &recipient, &amount);
        }

        Ok(())
    }

    /// Store distribution execution
    fn store_distribution_execution(
        env: &Env,
        execution: &FeeDistributionExecution,
    ) -> Result<(), Error> {
        let history_key = symbol_short!("dist_hist");
        let mut history: Vec<FeeDistributionExecution> = env
            .storage()
            .persistent()
            .get(&history_key)
            .unwrap_or(vec![env]);

        history.push_back(execution.clone());
        env.storage().persistent().set(&history_key, &history);

        Ok(())
    }

    /// Store distribution configuration
    fn store_distribution_config(
        env: &Env,
        config: &FeeDistributionConfig,
    ) -> Result<(), Error> {
        let config_key = symbol_short!("dist_cfg");
        env.storage().persistent().set(&config_key, config);
        Ok(())
    }

    /// Emit distribution config updated event
    fn emit_distribution_config_updated_event(
        env: &Env,
        admin: &Address,
        config: &FeeDistributionConfig,
    ) -> Result<(), Error> {
        use crate::events::EventEmitter;
        EventEmitter::emit_config_updated(
            env,
            admin,
            &String::from_str(env, "Fee Distribution"),
            &String::from_str(env, "Previous Config"),
            &config.distribution_name,
        );
        Ok(())
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

        let base_payout = (user_stake * total_pool) / winning_total;
        let payout = (base_payout * (100 - PLATFORM_FEE_PERCENTAGE)) / 100;

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

    /// Calculate dynamic fee based on market size and activity
    pub fn calculate_dynamic_fee_by_market_id(env: &Env, market_id: Symbol) -> Result<i128, Error> {
        let market = crate::markets::MarketStateManager::get_market(env, &market_id)?;
        Self::calculate_dynamic_fee(&market)
    }

    /// Get fee tier based on market size
    pub fn get_fee_tier_by_market_size(env: &Env, total_staked: i128) -> Result<FeeTier, Error> {
        let tier_name = if total_staked >= MARKET_SIZE_LARGE {
            String::from_str(env, "Large")
        } else if total_staked >= MARKET_SIZE_MEDIUM {
            String::from_str(env, "Medium")
        } else if total_staked >= MARKET_SIZE_SMALL {
            String::from_str(env, "Small")
        } else {
            String::from_str(env, "Micro")
        };

        let fee_percentage = if tier_name == String::from_str(env, "Large") {
            250 // 2.5%
        } else if tier_name == String::from_str(env, "Medium") {
            200 // 2.0%
        } else if tier_name == String::from_str(env, "Small") {
            150 // 1.5%
        } else if tier_name == String::from_str(env, "Micro") {
            100 // 1.0%
        } else {
            200 // Default 2.0%
        };

        let min_size = if tier_name == String::from_str(env, "Large") {
            MARKET_SIZE_LARGE
        } else if tier_name == String::from_str(env, "Medium") {
            MARKET_SIZE_MEDIUM
        } else if tier_name == String::from_str(env, "Small") {
            MARKET_SIZE_SMALL
        } else if tier_name == String::from_str(env, "Micro") {
            0
        } else {
            0
        };

        let max_size = if tier_name == String::from_str(env, "Large") {
            i128::MAX
        } else if tier_name == String::from_str(env, "Medium") {
            MARKET_SIZE_LARGE - 1
        } else if tier_name == String::from_str(env, "Small") {
            MARKET_SIZE_MEDIUM - 1
        } else if tier_name == String::from_str(env, "Micro") {
            MARKET_SIZE_SMALL - 1
        } else {
            MARKET_SIZE_SMALL - 1
        };

        Ok(FeeTier {
            min_size,
            max_size,
            fee_percentage,
            tier_name,
        })
    }

    /// Adjust fee by activity level
    pub fn adjust_fee_by_activity(
        env: &Env,
        market_id: Symbol,
        activity_level: u32,
    ) -> Result<i128, Error> {
        let market = crate::markets::MarketStateManager::get_market(env, &market_id)?;
        let base_fee = Self::calculate_dynamic_fee(&market)?;

        let activity_multiplier = if activity_level >= ACTIVITY_LEVEL_HIGH {
            120 // 20% increase for high activity
        } else if activity_level >= ACTIVITY_LEVEL_MEDIUM {
            110 // 10% increase for medium activity
        } else if activity_level >= ACTIVITY_LEVEL_LOW {
            105 // 5% increase for low activity
        } else {
            100 // No adjustment for very low activity
        };

        let adjusted_fee = (base_fee * activity_multiplier) / 100;

        // Ensure fee is within limits
        if adjusted_fee < MIN_FEE_AMOUNT {
            Ok(MIN_FEE_AMOUNT)
        } else if adjusted_fee > MAX_FEE_AMOUNT {
            Ok(MAX_FEE_AMOUNT)
        } else {
            Ok(adjusted_fee)
        }
    }

    /// Validate fee percentage
    pub fn validate_fee_percentage(env: &Env, fee: i128, market_id: Symbol) -> Result<bool, Error> {
        if fee < MIN_FEE_PERCENTAGE {
            return Err(Error::InvalidInput);
        }

        if fee > MAX_FEE_PERCENTAGE {
            return Err(Error::InvalidInput);
        }

        // Check if fee is reasonable for the market size
        let market = crate::markets::MarketStateManager::get_market(env, &market_id)?;
        let tier = Self::get_fee_tier_by_market_size(env, market.total_staked)?;

        // Allow some flexibility around the tier fee
        let min_allowed = (tier.fee_percentage * 80) / 100; // 20% below tier
        let max_allowed = (tier.fee_percentage * 120) / 100; // 20% above tier

        if fee < min_allowed || fee > max_allowed {
            return Err(Error::InvalidInput);
        }

        Ok(true)
    }

    /// Get fee calculation factors for a market
    pub fn get_fee_calculation_factors(
        env: &Env,
        market_id: Symbol,
    ) -> Result<FeeCalculationFactors, Error> {
        let market = crate::markets::MarketStateManager::get_market(env, &market_id)?;

        // Get base fee tier
        let tier = Self::get_fee_tier_by_market_size(env, market.total_staked)?;

        // Calculate activity level
        let vote_count = market.votes.len() as u32;
        let activity_level = if vote_count >= ACTIVITY_LEVEL_HIGH {
            String::from_str(env, "High")
        } else if vote_count >= ACTIVITY_LEVEL_MEDIUM {
            String::from_str(env, "Medium")
        } else if vote_count >= ACTIVITY_LEVEL_LOW {
            String::from_str(env, "Low")
        } else {
            String::from_str(env, "Very Low")
        };

        // Calculate multipliers
        let size_multiplier = if tier.tier_name == String::from_str(env, "Large") {
            110 // 10% increase
        } else if tier.tier_name == String::from_str(env, "Medium") {
            100 // No change
        } else if tier.tier_name == String::from_str(env, "Small") {
            95 // 5% decrease
        } else if tier.tier_name == String::from_str(env, "Micro") {
            90 // 10% decrease
        } else {
            100
        };

        let activity_multiplier = if activity_level == String::from_str(env, "High") {
            120 // 20% increase
        } else if activity_level == String::from_str(env, "Medium") {
            110 // 10% increase
        } else if activity_level == String::from_str(env, "Low") {
            105 // 5% increase
        } else if activity_level == String::from_str(env, "Very Low") {
            100 // No change
        } else {
            100
        };

        let complexity_factor = 100; // No complexity adjustment for now

        // Calculate final fee percentage
        let final_fee_percentage =
            (tier.fee_percentage * size_multiplier * activity_multiplier * complexity_factor)
                / (100 * 100 * 100);

        // Ensure final fee is within limits
        let final_fee_percentage = if final_fee_percentage < MIN_FEE_PERCENTAGE {
            MIN_FEE_PERCENTAGE
        } else if final_fee_percentage > MAX_FEE_PERCENTAGE {
            MAX_FEE_PERCENTAGE
        } else {
            final_fee_percentage
        };

        Ok(FeeCalculationFactors {
            base_fee_percentage: tier.fee_percentage,
            size_multiplier,
            activity_multiplier,
            complexity_factor,
            final_fee_percentage,
            market_size_tier: tier.tier_name,
            activity_level,
        })
    }
}

// ===== FEE VALIDATOR =====

/// Fee validation utilities
pub struct FeeValidator;

impl FeeValidator {
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

    /// Validate fee distribution for accuracy and compliance
    pub fn validate_fee_distribution(distribution: &Map<Address, i128>) -> Result<(), Error> {
        // Check if distribution is empty
        if distribution.is_empty() {
            return Err(Error::InvalidInput);
        }

        // Validate each distribution entry
        for (recipient, amount) in distribution.iter() {
            // Check amount validity
            if amount <= 0 {
                return Err(Error::InvalidInput);
            }

            // Check for reasonable amount limits
            if amount > MAX_FEE_AMOUNT {
                return Err(Error::InvalidInput);
            }
        }

        // Calculate and validate total
        let mut total_amount: i128 = 0;
        for (_, amount) in distribution.iter() {
            total_amount += amount;
        }
        
        if total_amount <= 0 {
            return Err(Error::InvalidInput);
        }

        // Check for reasonable total limits
        if total_amount > MAX_FEE_AMOUNT * 10 {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }

    /// Validate market fees
    pub fn validate_market_fees(market: &Market) -> Result<FeeValidationResult, Error> {
        let env = market.outcomes.env(); // Get environment from market
        let mut errors = Vec::new(env);
        let mut is_valid = true;

        // Check if market has sufficient stakes
        if market.total_staked < FEE_COLLECTION_THRESHOLD {
            errors.push_back(String::from_str(
                env,
                "Insufficient stakes for fee collection",
            ));
            is_valid = false;
        }

        // Check if fees already collected
        if market.fee_collected {
            errors.push_back(String::from_str(env, "Fees already collected"));
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
            return (
                false,
                String::from_str(&Env::default(), "Market not resolved"),
            );
        }

        if market.fee_collected {
            return (
                false,
                String::from_str(&Env::default(), "Fees already collected"),
            );
        }

        if market.total_staked < FEE_COLLECTION_THRESHOLD {
            return (
                false,
                String::from_str(&Env::default(), "Insufficient stakes"),
            );
        }

        (
            true,
            String::from_str(&Env::default(), "Eligible for fee collection"),
        )
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
        let current_total: i128 = env.storage().persistent().get(&total_key).unwrap_or(0);

        env.storage()
            .persistent()
            .set(&total_key, &(current_total + amount));

        Ok(())
    }

    /// Record creation fee

    pub fn record_creation_fee(
        env: &Env,
        _admin: &Address,
        amount: i128,
    ) -> Result<(), Error> {

        // Record creation fee in analytics
        let creation_key = symbol_short!("creat_fee");
        let current_total: i128 = env.storage().persistent().get(&creation_key).unwrap_or(0);

        env.storage()
            .persistent()
            .set(&creation_key, &(current_total + amount));

        Ok(())
    }

    /// Record configuration change
    pub fn record_config_change(
        env: &Env,
        _admin: &Address,
        _config: &FeeConfig,
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
        Ok(env.storage().persistent().get(&total_key).unwrap_or(0))
    }

    /// Record fee structure update
    pub fn record_fee_structure_update(
        env: &Env,
        admin: &Address,
        new_fee_tiers: &Map<u32, i128>,
    ) -> Result<(), Error> {
        let storage_key = symbol_short!("fee_str");
        let update_data = (
            admin.clone(),
            new_fee_tiers.clone(),
            env.ledger().timestamp(),
        );
        env.storage().persistent().set(&storage_key, &update_data);
        Ok(())
    }

    /// Emit fee distribution tracking event
    pub fn emit_fee_distribution_tracked(
        env: &Env,
        market_id: &Symbol,
        distribution: &Map<Address, i128>,
        total_amount: i128,
    ) -> Result<(), Error> {
        // Emit distribution event
        use crate::events::EventEmitter;
        EventEmitter::emit_performance_metric(
            env,
            &String::from_str(env, "fee_dist_total"),
            total_amount,
            &String::from_str(env, "stroops"),
            &String::from_str(env, "Fee distribution tracked"),
        );

        // Log distribution analytics
        let distribution_key = symbol_short!("dist_anal");
        let current_total: i128 = env.storage().persistent().get(&distribution_key).unwrap_or(0);
        env.storage()
            .persistent()
            .set(&distribution_key, &(current_total + total_amount));

        Ok(())
    }

    /// Record fee collection analytics
    pub fn record_fee_collection_analytics(
        env: &Env,
        market_id: &Symbol,
        fee_amount: i128,
    ) -> Result<(), Error> {
        // Update collection analytics
        let analytics_key = symbol_short!("coll_anal");
        let current_total: i128 = env.storage().persistent().get(&analytics_key).unwrap_or(0);
        env.storage()
            .persistent()
            .set(&analytics_key, &(current_total + fee_amount));

        // Record collection timestamp
        let timestamp_key = symbol_short!("coll_t");
        env.storage()
            .persistent()
            .set(&timestamp_key, &env.ledger().timestamp());

        Ok(())
    }

    /// Record fee refund analytics
    pub fn record_fee_refund_analytics(
        env: &Env,
        market_id: &Symbol,
        refund_amount: i128,
    ) -> Result<(), Error> {
        // Update refund analytics
        let refund_key = symbol_short!("refund_a");
        let current_total: i128 = env.storage().persistent().get(&refund_key).unwrap_or(0);
        env.storage()
            .persistent()
            .set(&refund_key, &(current_total + refund_amount));

        // Record refund timestamp
        let timestamp_key = symbol_short!("refund_t");
        env.storage()
            .persistent()
            .set(&timestamp_key, &env.ledger().timestamp());

        Ok(())
    }

    /// Get fee distribution history
    pub fn get_fee_distribution_history(env: &Env) -> Result<Vec<FeeDistribution>, Error> {
        let distribution_key = symbol_short!("fee_dist");
        Ok(env
            .storage()
            .persistent()
            .get(&distribution_key)
            .unwrap_or(vec![env]))
    }

    /// Get fee refund history
    pub fn get_fee_refund_history(env: &Env) -> Result<Vec<FeeRefund>, Error> {
        let refund_key = symbol_short!("fee_ref");
        Ok(env
            .storage()
            .persistent()
            .get(&refund_key)
            .unwrap_or(vec![env]))
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
        let markets_with_fees = history.len();

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
        let actual_fee = if market.fee_collected {
            potential_fee
        } else {
            0
        };

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

    /// Create test fee tier
    pub fn create_test_fee_tier(env: &Env) -> FeeTier {
        FeeTier {
            min_size: 0,
            max_size: 100_000_000, // 10 XLM
            fee_percentage: 150,   // 1.5%
            tier_name: String::from_str(env, "Small"),
        }
    }

    /// Create test activity adjustment
    pub fn create_test_activity_adjustment(env: &Env) -> ActivityAdjustment {
        ActivityAdjustment {
            activity_level: 50,
            fee_multiplier: 110, // 10% increase
            description: String::from_str(env, "Medium Activity"),
        }
    }

    /// Create test fee calculation factors
    pub fn create_test_fee_calculation_factors(env: &Env) -> FeeCalculationFactors {
        FeeCalculationFactors {
            base_fee_percentage: 200,  // 2%
            size_multiplier: 100,      // No change
            activity_multiplier: 110,  // 10% increase
            complexity_factor: 100,    // No change
            final_fee_percentage: 220, // 2.2%
            market_size_tier: String::from_str(env, "Medium"),
            activity_level: String::from_str(env, "Medium"),
        }
    }

    /// Create test fee history
    pub fn create_test_fee_history(env: &Env, market_id: Symbol) -> FeeHistory {
        FeeHistory {
            market_id,
            timestamp: env.ledger().timestamp(),
            old_fee_percentage: 200, // 2%
            new_fee_percentage: 220, // 2.2%
            reason: String::from_str(env, "Activity level increased"),
            admin: Address::from_str(env, "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF"),
            calculation_factors: testing::create_test_fee_calculation_factors(env),
        }
    }
}

pub fn validate_fee_collection_permissions(_admin: &Address) -> Result<(), Error> {
    // Implementation
    Ok(())
}

pub fn validate_fee_config_update(_admin: &Address, _config: &FeeConfig) -> Result<(), Error> {
    // Implementation
    Ok(())
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
            soroban_sdk::vec![
                &env,
                String::from_str(&env, "yes"),
                String::from_str(&env, "no"),
            ],
            env.ledger().timestamp() + 86400,
            crate::types::OracleConfig::new(
                crate::types::OracleProvider::Pyth,
                String::from_str(&env, "BTC/USD"),
                2_500_000,
                String::from_str(&env, "gt"),
            ),
            crate::types::MarketState::Active,
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
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let admin = Address::generate(&env);

        env.as_contract(&contract_id, || {
            // Set admin in storage
            env.storage()
                .persistent()
                .set(&Symbol::new(&env, "Admin"), &admin);

            // Valid admin
            assert!(FeeValidator::validate_admin_permissions(&env, &admin).is_ok());

            // Invalid admin
            let invalid_admin = Address::generate(&env);
            assert!(FeeValidator::validate_admin_permissions(&env, &invalid_admin).is_err());
        });
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
            soroban_sdk::vec![
                &env,
                String::from_str(&env, "yes"),
                String::from_str(&env, "no"),
            ],
            env.ledger().timestamp() + 86400,
            crate::types::OracleConfig::new(
                crate::types::OracleProvider::Pyth,
                String::from_str(&env, "BTC/USD"),
                2_500_000,
                String::from_str(&env, "gt"),
            ),
            crate::types::MarketState::Active,
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
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let config = testing::create_test_fee_config();

        env.as_contract(&contract_id, || {
            // Store and retrieve config
            FeeConfigManager::store_fee_config(&env, &config).unwrap();
            let retrieved_config = FeeConfigManager::get_fee_config(&env).unwrap();

            assert_eq!(config, retrieved_config);
        });
    }

    #[test]
    fn test_fee_analytics_calculation() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());

        env.as_contract(&contract_id, || {
            // Test with no fee history
            let analytics = FeeAnalytics::calculate_analytics(&env).unwrap();
            assert_eq!(analytics.total_fees_collected, 0);
            assert_eq!(analytics.markets_with_fees, 0);
            assert_eq!(analytics.average_fee_per_market, 0);
        });
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

    #[test]
    fn test_dynamic_fee_tier_calculation() {
        let env = Env::default();

        // Test small market tier
        let small_tier = FeeCalculator::get_fee_tier_by_market_size(&env, 50_000_000).unwrap();
        assert_eq!(small_tier.fee_percentage, 100); // 1.0%
        assert_eq!(small_tier.tier_name, String::from_str(&env, "Micro"));

        // Test medium market tier
        let medium_tier = FeeCalculator::get_fee_tier_by_market_size(&env, 500_000_000).unwrap();
        assert_eq!(medium_tier.fee_percentage, 150); // 1.5%
        assert_eq!(medium_tier.tier_name, String::from_str(&env, "Small"));

        // Test large market tier
        let large_tier = FeeCalculator::get_fee_tier_by_market_size(&env, 5_000_000_000).unwrap();
        assert_eq!(large_tier.fee_percentage, 200); // 2.0%
        assert_eq!(large_tier.tier_name, String::from_str(&env, "Medium"));
    }

    #[test]
    fn test_fee_calculation_factors() {
        let env = Env::default();

        // Test the structure creation
        let factors = testing::create_test_fee_calculation_factors(&env);
        assert_eq!(factors.base_fee_percentage, 200);
        assert_eq!(factors.final_fee_percentage, 220);
        assert_eq!(factors.market_size_tier, String::from_str(&env, "Medium"));
        assert_eq!(factors.activity_level, String::from_str(&env, "Medium"));
    }

    #[test]
    fn test_fee_history_creation() {
        let env = Env::default();
        let market_id = Symbol::new(&env, "test_market");

        let history = testing::create_test_fee_history(&env, market_id);
        assert_eq!(history.old_fee_percentage, 200);
        assert_eq!(history.new_fee_percentage, 220);
        assert_eq!(
            history.reason,
            String::from_str(&env, "Activity level increased")
        );
    }
}

/// Fee distribution configuration for multiple parties
///
/// This structure defines how fees are distributed across multiple recipients,
/// including percentages, addresses, and distribution rules. Essential for
/// implementing multi-party fee distribution systems.
///
/// # Distribution Configuration
///
/// Fee distribution includes:
/// - Multiple recipient addresses and their percentages
/// - Distribution validation rules
/// - Governance and management settings
/// - Distribution history tracking
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Address, Map};
/// # use predictify_hybrid::fees::FeeDistributionConfig;
/// # let env = Env::default();
///
/// // Create fee distribution configuration
/// let mut distribution = Map::new(&env);
/// distribution.set(Address::generate(&env), 60); // 60% to platform
/// distribution.set(Address::generate(&env), 25); // 25% to governance
/// distribution.set(Address::generate(&env), 15); // 15% to community
///
/// let config = FeeDistributionConfig {
///     distribution: distribution,
///     total_percentage: 100,
///     governance_enabled: true,
///     community_participation: true,
///     min_distribution_percentage: 5,
///     max_distribution_percentage: 80,
///     distribution_name: String::from_str(&env, "Standard Distribution"),
///     created_by: Address::generate(&env),
///     created_at: env.ledger().timestamp(),
///     is_active: true,
/// };
///
/// // Validate distribution configuration
/// assert_eq!(config.total_percentage, 100);
/// assert!(config.is_valid());
/// ```
///
/// # Governance Features
///
/// Distribution configuration supports:
/// - **Multi-party Governance**: Multiple addresses can receive fees
/// - **Percentage-based Distribution**: Flexible allocation percentages
/// - **Validation Rules**: Ensure percentages add up correctly
/// - **Active/Inactive States**: Enable/disable distribution configurations
/// - **Audit Trail**: Track who created and modified configurations
///
/// # Integration Applications
///
/// - **Platform Revenue Sharing**: Distribute fees to multiple stakeholders
/// - **Governance Participation**: Reward governance participants
/// - **Community Incentives**: Fund community initiatives
/// - **Partner Revenue**: Share fees with partners and integrators
/// - **Treasury Management**: Manage platform treasury allocations
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeDistributionConfig {
    /// Distribution mapping (address -> percentage)
    pub distribution: Map<Address, i128>,
    /// Total percentage (must equal 100)
    pub total_percentage: i128,
    /// Whether governance participation is enabled
    pub governance_enabled: bool,
    /// Whether community participation is enabled
    pub community_participation: bool,
    /// Minimum percentage per recipient
    pub min_distribution_percentage: i128,
    /// Maximum percentage per recipient
    pub max_distribution_percentage: i128,
    /// Distribution configuration name
    pub distribution_name: String,
    /// Created by admin
    pub created_by: Address,
    /// Creation timestamp
    pub created_at: u64,
    /// Whether this configuration is active
    pub is_active: bool,
}

impl FeeDistributionConfig {
    /// Validate the distribution configuration
    pub fn is_valid(&self) -> bool {
        // Check if total percentage equals 100
        if self.total_percentage != 100 {
            return false;
        }

        // Check if distribution is not empty
        if self.distribution.is_empty() {
            return false;
        }

        // Validate each distribution entry
        let mut calculated_total: i128 = 0;
        for (_, percentage) in self.distribution.iter() {
            // Check percentage bounds
            if percentage < self.min_distribution_percentage || percentage > self.max_distribution_percentage {
                return false;
            }
            calculated_total += percentage;
        }

        // Check if calculated total matches expected total
        calculated_total == self.total_percentage
    }

    /// Get distribution percentage for a specific address
    pub fn get_percentage_for_address(&self, address: &Address) -> Option<i128> {
        self.distribution.get(address.clone())
    }

    /// Calculate fee amount for a specific address
    pub fn calculate_fee_amount(&self, total_fee: i128, address: &Address) -> Option<i128> {
        let percentage = self.get_percentage_for_address(address)?;
        Some((total_fee * percentage) / 100)
    }

    /// Get all recipient addresses
    pub fn get_recipients(&self) -> Vec<Address> {
        let mut recipients = Vec::new(&Env::default());
        for (address, _) in self.distribution.iter() {
            recipients.push_back(address);
        }
        recipients
    }

    /// Check if address is a recipient
    pub fn is_recipient(&self, address: &Address) -> bool {
        self.distribution.contains_key(address.clone())
    }
}

/// Fee distribution execution record
///
/// This structure tracks the execution of fee distributions to multiple parties,
/// providing complete audit trails and verification for multi-party fee distribution.
/// Essential for transparency and compliance in fee distribution operations.
///
/// # Distribution Execution
///
/// Distribution execution includes:
/// - Market identification and total fee amount
/// - Distribution to multiple recipients
/// - Execution verification and status
/// - Complete audit trail and timestamps
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Address, Map, Symbol};
/// # use predictify_hybrid::fees::FeeDistributionExecution;
/// # let env = Env::default();
///
/// // Create distribution execution record
/// let mut distribution = Map::new(&env);
/// distribution.set(Address::generate(&env), 60_000_000); // 60 XLM
/// distribution.set(Address::generate(&env), 25_000_000); // 25 XLM
/// distribution.set(Address::generate(&env), 15_000_000); // 15 XLM
///
/// let execution = FeeDistributionExecution {
///     market_id: Symbol::new(&env, "btc_market"),
///     total_fee_amount: 100_000_000, // 100 XLM total
///     distribution: distribution,
///     distribution_config_id: Symbol::new(&env, "standard_dist"),
///     executed_by: Address::generate(&env),
///     execution_timestamp: env.ledger().timestamp(),
///     verification_status: String::from_str(&env, "Pending"),
///     verification_timestamp: 0,
///     verification_notes: String::from_str(&env, "Awaiting verification"),
///     success: true,
///     error_message: None,
/// };
///
/// // Verify distribution totals
/// let calculated_total: i128 = execution.distribution.values().sum();
/// assert_eq!(execution.total_fee_amount, calculated_total);
/// ```
///
/// # Audit Features
///
/// Execution provides audit capabilities:
/// - **Complete Tracking**: Full record of distribution execution
/// - **Verification Status**: Track verification and approval process
/// - **Success/Failure Tracking**: Monitor execution success rates
/// - **Error Handling**: Capture and track distribution errors
/// - **Timestamp Records**: Complete chronological audit trail
///
/// # Integration Applications
///
/// - **Financial Reporting**: Generate distribution execution reports
/// - **Audit Trails**: Maintain compliance records
/// - **Transparency**: Provide public execution data
/// - **Error Monitoring**: Track and resolve distribution issues
/// - **Performance Analytics**: Analyze distribution efficiency
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeDistributionExecution {
    /// Market ID
    pub market_id: Symbol,
    /// Total fee amount distributed
    pub total_fee_amount: i128,
    /// Distribution to recipients (address -> amount)
    pub distribution: Map<Address, i128>,
    /// Distribution configuration ID used
    pub distribution_config_id: Symbol,
    /// Executed by admin
    pub executed_by: Address,
    /// Execution timestamp
    pub execution_timestamp: u64,
    /// Verification status
    pub verification_status: String,
    /// Verification timestamp
    pub verification_timestamp: u64,
    /// Verification notes
    pub verification_notes: String,
    /// Whether execution was successful
    pub success: bool,
    /// Error message if execution failed
    pub error_message: Option<String>,
}

impl FeeDistributionExecution {
    /// Validate the distribution execution
    pub fn is_valid(&self) -> bool {
        // Check if total fee amount is positive
        if self.total_fee_amount <= 0 {
            return false;
        }

        // Check if distribution is not empty
        if self.distribution.is_empty() {
            return false;
        }

        // Validate distribution totals
        let mut calculated_total: i128 = 0;
        for (_, amount) in self.distribution.iter() {
            if amount <= 0 {
                return false;
            }
            calculated_total += amount;
        }

        // Check if calculated total matches expected total
        calculated_total == self.total_fee_amount
    }

    /// Get distribution amount for a specific address
    pub fn get_amount_for_address(&self, address: &Address) -> Option<i128> {
        self.distribution.get(address.clone())
    }

    /// Get all recipient addresses
    pub fn get_recipients(&self) -> Vec<Address> {
        let mut recipients = Vec::new(&Env::default());
        for (address, _) in self.distribution.iter() {
            recipients.push_back(address);
        }
        recipients
    }

    /// Check if execution is verified
    pub fn is_verified(&self) -> bool {
        self.verification_status == String::from_str(&Env::default(), "Verified")
    }

    /// Mark execution as verified
    pub fn mark_verified(&mut self, env: &Env, notes: String) {
        self.verification_status = String::from_str(env, "Verified");
        self.verification_timestamp = env.ledger().timestamp();
        self.verification_notes = notes;
    }
}

/// Fee distribution governance record
///
/// This structure tracks governance decisions and changes related to fee distribution,
/// providing transparency and auditability for distribution governance operations.
/// Essential for democratic and transparent fee distribution management.
///
/// # Governance Features
///
/// Distribution governance includes:
/// - Governance proposals and decisions
/// - Voting records and outcomes
/// - Distribution configuration changes
/// - Governance participation tracking
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Address, Map, Symbol};
/// # use predictify_hybrid::fees::FeeDistributionGovernance;
/// # let env = Env::default();
///
/// // Create governance record
/// let mut votes = Map::new(&env);
/// votes.set(Address::generate(&env), true); // Yes vote
/// votes.set(Address::generate(&env), false); // No vote
///
/// let governance = FeeDistributionGovernance {
///     proposal_id: Symbol::new(&env, "prop_001"),
///     proposal_type: String::from_str(&env, "Distribution Change"),
///     proposal_description: String::from_str(&env, "Increase community allocation to 20%"),
///     proposed_by: Address::generate(&env),
///     votes: votes,
///     total_votes: 2,
///     yes_votes: 1,
///     no_votes: 1,
///     voting_start: env.ledger().timestamp(),
///     voting_end: env.ledger().timestamp() + 86400, // 24 hours
///     status: String::from_str(&env, "Active"),
///     outcome: None,
///     executed: false,
///     execution_timestamp: 0,
/// };
///
/// // Check voting status
/// let approval_percentage = (governance.yes_votes * 100) / governance.total_votes;
/// println!("Approval: {}%", approval_percentage);
/// ```
///
/// # Governance Features
///
/// Governance provides democratic features:
/// - **Proposal System**: Submit and vote on distribution changes
/// - **Voting Mechanism**: Democratic decision-making process
/// - **Transparency**: Public voting records and outcomes
/// - **Execution Tracking**: Monitor proposal implementation
/// - **Participation Incentives**: Encourage governance participation
///
/// # Integration Applications
///
/// - **Democratic Governance**: Enable community-driven decisions
/// - **Transparency**: Provide public governance records
/// - **Participation Tracking**: Monitor governance engagement
/// - **Proposal Management**: Manage distribution change proposals
/// - **Voting Analytics**: Analyze governance participation patterns
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeDistributionGovernance {
    /// Proposal ID
    pub proposal_id: Symbol,
    /// Type of proposal
    pub proposal_type: String,
    /// Proposal description
    pub proposal_description: String,
    /// Proposed by address
    pub proposed_by: Address,
    /// Votes mapping (address -> vote)
    pub votes: Map<Address, bool>,
    /// Total number of votes
    pub total_votes: u32,
    /// Number of yes votes
    pub yes_votes: u32,
    /// Number of no votes
    pub no_votes: u32,
    /// Voting start timestamp
    pub voting_start: u64,
    /// Voting end timestamp
    pub voting_end: u64,
    /// Proposal status
    pub status: String,
    /// Voting outcome
    pub outcome: Option<String>,
    /// Whether proposal was executed
    pub executed: bool,
    /// Execution timestamp
    pub execution_timestamp: u64,
}

impl FeeDistributionGovernance {
    /// Check if proposal is active
    pub fn is_active(&self, current_time: u64) -> bool {
        self.status == String::from_str(&Env::default(), "Active") 
            && current_time >= self.voting_start 
            && current_time <= self.voting_end
    }

    /// Check if proposal has ended
    pub fn has_ended(&self, current_time: u64) -> bool {
        current_time > self.voting_end
    }

    /// Get approval percentage
    pub fn get_approval_percentage(&self) -> u32 {
        if self.total_votes == 0 {
            return 0;
        }
        (self.yes_votes * 100) / self.total_votes
    }

    /// Check if proposal is approved (requires >50% yes votes)
    pub fn is_approved(&self) -> bool {
        self.get_approval_percentage() > 50
    }

    /// Add a vote to the proposal
    pub fn add_vote(&mut self, voter: Address, vote: bool) {
        // Check if voter already voted
        if self.votes.contains_key(voter.clone()) {
            // Update existing vote
            let old_vote = self.votes.get(voter.clone()).unwrap();
            if old_vote != vote {
                if old_vote {
                    self.yes_votes -= 1;
                } else {
                    self.no_votes -= 1;
                }
                
                if vote {
                    self.yes_votes += 1;
                } else {
                    self.no_votes += 1;
                }
            }
        } else {
            // Add new vote
            if vote {
                self.yes_votes += 1;
            } else {
                self.no_votes += 1;
            }
            self.total_votes += 1;
        }
        
        self.votes.set(voter, vote);
    }

    /// Finalize proposal outcome
    pub fn finalize_outcome(&mut self) {
        if self.is_approved() {
            self.outcome = Some(String::from_str(&Env::default(), "Approved"));
            self.status = String::from_str(&Env::default(), "Approved");
        } else {
            self.outcome = Some(String::from_str(&Env::default(), "Rejected"));
            self.status = String::from_str(&Env::default(), "Rejected");
        }
    }
}

/// Fee distribution analytics and statistics
///
/// This structure provides comprehensive analytics for multi-party fee distribution,
/// including distribution patterns, recipient performance, and governance metrics.
/// Essential for understanding and optimizing fee distribution systems.
///
/// # Analytics Scope
///
/// Distribution analytics include:
/// - Total distributions and amounts
/// - Recipient performance and patterns
/// - Governance participation metrics
/// - Distribution efficiency indicators
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Map, String, Vec};
/// # use predictify_hybrid::fees::FeeDistributionAnalytics;
/// # let env = Env::default();
///
/// // Create distribution analytics
/// let mut recipient_stats = Map::new(&env);
/// recipient_stats.set(String::from_str(&env, "platform"), 60_000_000); // 60 XLM
/// recipient_stats.set(String::from_str(&env, "governance"), 25_000_000); // 25 XLM
/// recipient_stats.set(String::from_str(&env, "community"), 15_000_000); // 15 XLM
///
/// let analytics = FeeDistributionAnalytics {
///     total_distributions: 50,
///     total_amount_distributed: 1_000_000_000, // 1000 XLM
///     average_distribution_amount: 20_000_000, // 20 XLM average
///     recipient_statistics: recipient_stats,
///     governance_proposals: 10,
///     governance_participation_rate: 75, // 75%
///     distribution_efficiency: 95, // 95%
///     last_distribution_timestamp: env.ledger().timestamp(),
///     distribution_history: Vec::new(&env),
/// };
///
/// // Display analytics summary
/// println!("Total distributed: {} XLM", analytics.total_amount_distributed / 10_000_000);
/// println!("Average per distribution: {} XLM", analytics.average_distribution_amount / 10_000_000);
/// println!("Governance participation: {}%", analytics.governance_participation_rate);
/// ```
///
/// # Analytics Features
///
/// Analytics provide insights into:
/// - **Distribution Performance**: Track distribution efficiency and patterns
/// - **Recipient Analysis**: Understand recipient behavior and performance
/// - **Governance Metrics**: Monitor governance participation and effectiveness
/// - **Efficiency Tracking**: Measure distribution system performance
/// - **Historical Trends**: Analyze distribution patterns over time
///
/// # Integration Applications
///
/// - **Performance Dashboards**: Display distribution performance metrics
/// - **Governance Reporting**: Generate governance participation reports
/// - **Efficiency Analysis**: Identify optimization opportunities
/// - **Recipient Management**: Monitor and manage recipient performance
/// - **Strategic Planning**: Data-driven distribution strategy decisions
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeDistributionAnalytics {
    /// Total number of distributions
    pub total_distributions: u32,
    /// Total amount distributed
    pub total_amount_distributed: i128,
    /// Average distribution amount
    pub average_distribution_amount: i128,
    /// Recipient statistics (recipient -> amount)
    pub recipient_statistics: Map<String, i128>,
    /// Number of governance proposals
    pub governance_proposals: u32,
    /// Governance participation rate (percentage)
    pub governance_participation_rate: u32,
    /// Distribution efficiency (percentage)
    pub distribution_efficiency: u32,
    /// Last distribution timestamp
    pub last_distribution_timestamp: u64,
    /// Distribution history
    pub distribution_history: Vec<FeeDistributionExecution>,
}



impl FeeDistributionAnalytics {
    /// Calculate analytics from distribution history
    pub fn calculate_from_history(env: &Env, history: Vec<FeeDistributionExecution>) -> Self {
        let mut total_distributions = 0;
        let mut total_amount_distributed: i128 = 0;
        let mut recipient_stats = Map::new(env);
        let mut last_timestamp = 0;

        for execution in history.iter() {
            total_distributions += 1;
            total_amount_distributed += execution.total_fee_amount;
            
            if execution.execution_timestamp > last_timestamp {
                last_timestamp = execution.execution_timestamp;
            }

            // Aggregate recipient statistics
            for (address, amount) in execution.distribution.iter() {
                let address_str = address.to_string();
                let current_amount = recipient_stats.get(address_str.clone()).unwrap_or(0);
                recipient_stats.set(address_str.clone(), current_amount + amount);
            }
        }

        let average_distribution_amount = if total_distributions > 0 {
            total_amount_distributed / (total_distributions as i128)
        } else {
            0
        };

        Self {
            total_distributions,
            total_amount_distributed,
            average_distribution_amount,
            recipient_statistics: recipient_stats,
            governance_proposals: 0, // TODO: Calculate from governance history
            governance_participation_rate: 0, // TODO: Calculate from governance history
            distribution_efficiency: 95, // Default efficiency
            last_distribution_timestamp: last_timestamp,
            distribution_history: history,
        }
    }

    /// Get recipient with highest distribution
    pub fn get_top_recipient(&self) -> Option<(String, i128)> {
        let mut top_recipient: Option<(String, i128)> = None;
        
        for (recipient, amount) in self.recipient_statistics.iter() {
            match top_recipient {
                Some((_, top_amount)) => {
                    if amount > top_amount {
                        top_recipient = Some((recipient, amount));
                    }
                }
                None => {
                    top_recipient = Some((recipient, amount));
                }
            }
        }
        
        top_recipient
    }

    /// Calculate distribution efficiency
    pub fn calculate_efficiency(&self) -> u32 {
        if self.total_distributions == 0 {
            return 0;
        }

        let successful_distributions = self.distribution_history
            .iter()
            .filter(|execution| execution.success)
            .count() as u32;

        (successful_distributions * 100) / self.total_distributions
    }
}

// ===== FEE DISTRIBUTION MANAGER =====

/// Comprehensive fee distribution management system for multiple parties
///
/// The FeeDistributionManager provides centralized fee distribution operations
/// including configuration management, execution, governance, and analytics.
/// It handles all multi-party fee distribution operations with proper validation,
/// transparency, and audit trails.
///
/// # Core Responsibilities
///
/// - **Distribution Configuration**: Manage multi-party distribution settings
/// - **Fee Distribution**: Execute fee distributions to multiple recipients
/// - **Governance Management**: Handle distribution governance and voting
/// - **Analytics**: Generate distribution analytics and performance metrics
/// - **Validation**: Ensure distribution accuracy and compliance
///
/// # Distribution Operations
///
/// The system supports:
/// - **Multi-party Distribution**: Distribute fees to multiple recipients
/// - **Percentage-based Allocation**: Flexible distribution percentages
/// - **Governance Integration**: Democratic distribution decisions
/// - **Audit Trails**: Complete transparency and tracking
/// - **Analytics**: Performance monitoring and optimization
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Address, Map, Symbol};
/// # use predictify_hybrid::fees::FeeDistributionManager;
/// # let env = Env::default();
/// # let admin = Address::generate(&env);
/// # let market_id = Symbol::new(&env, "btc_market");
///
/// // Distribute fees to multiple parties
/// let mut distribution = Map::new(&env);
/// distribution.set(Address::generate(&env), 60_000_000); // 60 XLM
/// distribution.set(Address::generate(&env), 25_000_000); // 25 XLM
/// distribution.set(Address::generate(&env), 15_000_000); // 15 XLM
///
/// let execution = FeeDistributionManager::distribute_fees_to_multiple_parties(
///     &env,
///     admin.clone(),
///     market_id.clone(),
///     distribution
/// ).unwrap();
///
/// println!("Distributed {} XLM to {} recipients", 
///     execution.total_fee_amount / 10_000_000,
///     execution.distribution.len());
///
/// // Get distribution analytics
/// let analytics = FeeDistributionManager::get_distribution_analytics(&env).unwrap();
/// println!("Total distributed: {} XLM", 
///     analytics.total_amount_distributed / 10_000_000);
/// ```
///
/// # Security and Validation
///
/// Distribution operations include:
/// - **Admin Authentication**: All operations require proper admin authentication
/// - **Distribution Validation**: Verify distribution percentages and amounts
/// - **Governance Checks**: Ensure governance compliance for changes
/// - **Audit Trails**: Complete transparency and tracking
/// - **Error Handling**: Comprehensive error management and recovery
///
/// # Economic Model
///
/// The distribution system supports:
/// - **Multi-stakeholder Economics**: Fair distribution to all participants
/// - **Governance Incentives**: Reward governance participation
/// - **Community Funding**: Support community initiatives
/// - **Platform Sustainability**: Maintain platform operations
/// - **Transparent Economics**: Clear and auditable distribution
///
/// # Integration Points
///
/// - **Fee Collection**: Automatic distribution after fee collection
/// - **Governance Systems**: Integration with governance mechanisms
/// - **Analytics Dashboards**: Distribution performance monitoring
/// - **Financial Reporting**: Distribution reporting and compliance
/// - **User Interfaces**: Distribution transparency and tracking
pub struct FeeDistributionManager;

impl FeeDistributionManager {
    /// Distribute fees to multiple parties
    pub fn distribute_fees_to_multiple_parties(
        env: &Env,
        admin: Address,
        market_id: Symbol,
        distribution: Map<Address, i128>,
    ) -> Result<FeeDistributionExecution, Error> {
        // Require authentication from the admin
        admin.require_auth();

        // Validate admin permissions
        FeeValidator::validate_admin_permissions(env, &admin)?;

        // Validate distribution
        FeeValidator::validate_fee_distribution(&distribution)?;

        // Get market for validation
        let market = MarketStateManager::get_market(env, &market_id)?;
        
        // Validate market state for fee distribution
        if market.winning_outcome.is_none() {
            return Err(Error::MarketNotResolved);
        }

        if !market.fee_collected {
            return Err(Error::FeeAlreadyCollected);
        }

        // Calculate total distribution amount
        let mut total_amount: i128 = 0;
        for (_, amount) in distribution.iter() {
            total_amount += amount;
        }

        // Validate total amount
        if total_amount <= 0 {
            return Err(Error::InvalidInput);
        }

        // Get active distribution configuration
        let config = Self::get_active_distribution_config(env)?;

        // Create distribution execution record
        let execution = FeeDistributionExecution {
            market_id: market_id.clone(),
            total_fee_amount: total_amount,
            distribution: distribution.clone(),
            distribution_config_id: Symbol::new(env, "active_config"),
            executed_by: admin.clone(),
            execution_timestamp: env.ledger().timestamp(),
            verification_status: String::from_str(env, "Pending"),
            verification_timestamp: 0,
            verification_notes: String::from_str(env, "Awaiting verification"),
            success: true,
            error_message: None,
        };

        // Validate execution
        if !execution.is_valid() {
            return Err(Error::InvalidInput);
        }

        // Transfer fees to recipients
        Self::transfer_fees_to_recipients(env, &distribution)?;

        // Store execution record
        Self::store_distribution_execution(env, &execution)?;

        // Emit distribution event
        Self::emit_fee_distribution_event(env, &market_id, &distribution)?;

        // Update analytics
        Self::update_distribution_analytics(env, &execution)?;

        Ok(execution)
    }

    /// Validate fee distribution percentages
    pub fn validate_fee_distribution_percentages(
        env: &Env,
        distribution: &Map<Address, i128>,
    ) -> Result<bool, Error> {
        // Check if distribution is empty
        if distribution.is_empty() {
            return Err(Error::InvalidInput);
        }

        // Calculate total percentage
        let mut total_percentage: i128 = 0;
        for (_, percentage) in distribution.iter() {
            // Validate percentage bounds
            if percentage < 0 || percentage > 100 {
                return Err(Error::InvalidInput);
            }
            total_percentage += percentage;
        }

        // Check if total equals 100%
        if total_percentage != 100 {
            return Err(Error::InvalidInput);
        }

        Ok(true)
    }

    /// Get fee distribution configuration
    pub fn get_fee_distribution_config(env: &Env) -> Result<FeeDistributionConfig, Error> {
        let config_key = symbol_short!("dist_cfg");
        match env.storage().persistent().get(&config_key) {
            Some(config) => Ok(config),
            None => {
                // Return default configuration
                let mut default_distribution = Map::new(env);
                let admin: Option<Address> = env.storage().persistent().get(&Symbol::new(env, "Admin"));
                
                let admin_clone = admin.clone();
                if let Some(admin_address) = admin {
                    default_distribution.set(admin_address, 100); // 100% to admin
                }

                Ok(FeeDistributionConfig {
                    distribution: default_distribution,
                    total_percentage: 100,
                    governance_enabled: false,
                    community_participation: false,
                    min_distribution_percentage: 5,
                    max_distribution_percentage: 80,
                    distribution_name: String::from_str(env, "Default Distribution"),
                    created_by: admin_clone.unwrap_or_else(|| Address::from_str(env, "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF")),
                    created_at: env.ledger().timestamp(),
                    is_active: true,
                })
            }
        }
    }

    /// Update fee distribution configuration
    pub fn update_fee_distribution_config(
        env: &Env,
        admin: Address,
        new_distribution: Map<Address, i128>,
    ) -> Result<FeeDistributionConfig, Error> {
        // Require authentication from the admin
        admin.require_auth();

        // Validate admin permissions
        FeeValidator::validate_admin_permissions(env, &admin)?;

        // Validate distribution percentages
        Self::validate_fee_distribution_percentages(env, &new_distribution)?;

        // Calculate total percentage
        let mut total_percentage: i128 = 0;
        for (_, percentage) in new_distribution.iter() {
            total_percentage += percentage;
        }

        // Create new configuration
        let config = FeeDistributionConfig {
            distribution: new_distribution,
            total_percentage,
            governance_enabled: true,
            community_participation: true,
            min_distribution_percentage: 5,
            max_distribution_percentage: 80,
            distribution_name: String::from_str(env, "Updated Distribution"),
            created_by: admin.clone(),
            created_at: env.ledger().timestamp(),
            is_active: true,
        };

        // Validate configuration
        if !config.is_valid() {
            return Err(Error::InvalidInput);
        }

        // Store configuration
        Self::store_distribution_config(env, &config)?;

        // Emit configuration update event
        Self::emit_distribution_config_updated_event(env, &admin, &config)?;

        Ok(config)
    }

    /// Emit fee distribution event
    pub fn emit_fee_distribution_event(
        env: &Env,
        market_id: &Symbol,
        distribution: &Map<Address, i128>,
    ) -> Result<(), Error> {
        // Calculate total amount
        let mut total_amount: i128 = 0;
        for (_, amount) in distribution.iter() {
            total_amount += amount;
        }

        // Emit distribution event
        use crate::events::EventEmitter;
        EventEmitter::emit_performance_metric(
            env,
            &String::from_str(env, "fee_dist_total"),
            total_amount,
            &String::from_str(env, "stroops"),
            &String::from_str(env, "Fee distribution completed"),
        );

        Ok(())
    }

    /// Track fee distribution history
    pub fn track_fee_distribution_history(
        env: &Env,
        market_id: Symbol,
    ) -> Result<Vec<FeeDistributionExecution>, Error> {
        let history_key = symbol_short!("dist_hist");
        let mut history: Vec<FeeDistributionExecution> = env
            .storage()
            .persistent()
            .get(&history_key)
            .unwrap_or(vec![env]);

        // Filter history for specific market
        let mut market_history = Vec::new(env);
        for execution in history.iter() {
            if execution.market_id == market_id {
                market_history.push_back(execution);
            }
        }

        Ok(market_history)
    }

    /// Validate distribution totals
    pub fn validate_distribution_totals(
        env: &Env,
        distribution: &Map<Address, i128>,
    ) -> Result<bool, Error> {
        // Check if distribution is empty
        if distribution.is_empty() {
            return Err(Error::InvalidInput);
        }

        // Calculate total amount
        let mut total_amount: i128 = 0;
        for (_, amount) in distribution.iter() {
            if amount <= 0 {
                return Err(Error::InvalidInput);
            }
            total_amount += amount;
        }

        // Validate total amount
        if total_amount <= 0 {
            return Err(Error::InvalidInput);
        }

        // Check for reasonable limits
        if total_amount > MAX_FEE_AMOUNT * 10 {
            return Err(Error::InvalidInput);
        }

        Ok(true)
    }

    // ===== PRIVATE HELPER METHODS =====

    /// Transfer fees to recipients
    fn transfer_fees_to_recipients(
        env: &Env,
        distribution: &Map<Address, i128>,
    ) -> Result<(), Error> {
        let token_client = MarketUtils::get_token_client(env)?;

        for (recipient, amount) in distribution.iter() {
            token_client.transfer(&env.current_contract_address(), &recipient, &amount);
        }

        Ok(())
    }

    /// Store distribution execution
    fn store_distribution_execution(
        env: &Env,
        execution: &FeeDistributionExecution,
    ) -> Result<(), Error> {
        let history_key = symbol_short!("dist_hist");
        let mut history: Vec<FeeDistributionExecution> = env
            .storage()
            .persistent()
            .get(&history_key)
            .unwrap_or(vec![env]);

        history.push_back(execution.clone());
        env.storage().persistent().set(&history_key, &history);

        Ok(())
    }

    /// Store distribution configuration
    fn store_distribution_config(
        env: &Env,
        config: &FeeDistributionConfig,
    ) -> Result<(), Error> {
        let config_key = symbol_short!("dist_cfg");
        env.storage().persistent().set(&config_key, config);
        Ok(())
    }

    /// Emit distribution config updated event
    fn emit_distribution_config_updated_event(
        env: &Env,
        admin: &Address,
        config: &FeeDistributionConfig,
    ) -> Result<(), Error> {
        use crate::events::EventEmitter;
        EventEmitter::emit_config_updated(
            env,
            admin,
            &String::from_str(env, "Fee Distribution"),
            &String::from_str(env, "Previous Config"),
            &config.distribution_name,
        );
        Ok(())
    }

    /// Get active distribution configuration
    fn get_active_distribution_config(env: &Env) -> Result<FeeDistributionConfig, Error> {
        Self::get_fee_distribution_config(env)
    }

    /// Store governance proposal
    fn store_governance_proposal(
        env: &Env,
        governance: &FeeDistributionGovernance,
    ) -> Result<(), Error> {
        let governance_key = symbol_short!("gov_prop");
        let mut proposals: Vec<FeeDistributionGovernance> = env
            .storage()
            .persistent()
            .get(&governance_key)
            .unwrap_or(vec![env]);

        // Update existing proposal or add new one
        let mut found = false;
        for i in 0..proposals.len() {
            if proposals.get(i).unwrap().proposal_id == governance.proposal_id {
                proposals.set(i, governance.clone());
                found = true;
                break;
            }
        }

        if !found {
            proposals.push_back(governance.clone());
        }

        env.storage().persistent().set(&governance_key, &proposals);
        Ok(())
    }

    /// Get governance proposal
    fn get_governance_proposal(
        env: &Env,
        proposal_id: &Symbol,
    ) -> Result<FeeDistributionGovernance, Error> {
        let governance_key = symbol_short!("gov_prop");
        let proposals: Vec<FeeDistributionGovernance> = env
            .storage()
            .persistent()
            .get(&governance_key)
            .unwrap_or(vec![env]);

        for proposal in proposals.iter() {
            if proposal.proposal_id == *proposal_id {
                return Ok(proposal);
            }
        }

        Err(Error::InvalidInput)
    }

    /// Emit governance proposal event
    fn emit_governance_proposal_event(
        env: &Env,
        proposer: &Address,
        proposal_id: &Symbol,
    ) -> Result<(), Error> {
        use crate::events::EventEmitter;
        EventEmitter::emit_admin_action_logged(
            env,
            proposer,
            "governance_proposal_created",
            &true,
        );
        Ok(())
    }

    /// Emit governance vote event
    fn emit_governance_vote_event(
        env: &Env,
        voter: &Address,
        proposal_id: &Symbol,
        vote: bool,
    ) -> Result<(), Error> {
        use crate::events::EventEmitter;
        EventEmitter::emit_admin_action_logged(
            env,
            voter,
            "governance_vote_cast",
            &vote,
        );
        Ok(())
    }

    /// Emit governance execution event
    fn emit_governance_execution_event(
        env: &Env,
        admin: &Address,
        proposal_id: &Symbol,
    ) -> Result<(), Error> {
        use crate::events::EventEmitter;
        EventEmitter::emit_admin_action_logged(
            env,
            admin,
            "governance_proposal_executed",
            &true,
        );
        Ok(())
    }

    /// Update distribution analytics
    fn update_distribution_analytics(
        env: &Env,
        execution: &FeeDistributionExecution,
    ) -> Result<(), Error> {
        // Update analytics counters
        let analytics_key = symbol_short!("dist_anal");
        let current_total: i128 = env.storage().persistent().get(&analytics_key).unwrap_or(0);
        env.storage()
            .persistent()
            .set(&analytics_key, &(current_total + execution.total_fee_amount));

        Ok(())
    }

    /// Get all distribution history
    fn get_all_distribution_history(env: &Env) -> Result<Vec<FeeDistributionExecution>, Error> {
        let history_key = symbol_short!("dist_hist");
        Ok(env
            .storage()
            .persistent()
            .get(&history_key)
            .unwrap_or(vec![env]))
    }
}
