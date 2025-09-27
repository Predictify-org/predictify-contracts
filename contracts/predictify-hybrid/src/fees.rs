use soroban_sdk::{contracttype, symbol_short, vec, Address, Env, Map, String, Symbol, Vec};

use crate::errors::Error;
use crate::markets::{MarketStateManager, MarketUtils};
use crate::types::Market;
use crate::reentrancy_guard::ReentrancyGuard;

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

/// Platform fee percentage (2%)
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
        // Validate creation fee
        FeeValidator::validate_creation_fee(MARKET_CREATION_FEE)?;

        // Get token client
        ReentrancyGuard::before_external_call(env)?;
        let token_client = MarketUtils::get_token_client(env)?;
        // Transfer creation fee from admin to contract
        token_client.transfer(admin, &env.current_contract_address(), &MARKET_CREATION_FEE);
        ReentrancyGuard::after_external_call(env);

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
        FeeValidator::validate_market_fees(&market)
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

    /// Validate market fees
    pub fn validate_market_fees(market: &Market) -> Result<FeeValidationResult, Error> {
        let mut errors = Vec::new(&Env::default());
        let mut is_valid = true;

        // Check if market has sufficient stakes
        if market.total_staked < FEE_COLLECTION_THRESHOLD {
            errors.push_back(String::from_str(
                &Env::default(),
                "Insufficient stakes for fee collection",
            ));
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
        ReentrancyGuard::before_external_call(env)?;
        let token_client = MarketUtils::get_token_client(env)?;
        token_client.transfer(&env.current_contract_address(), admin, &amount);
        ReentrancyGuard::after_external_call(env);
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

    pub fn record_creation_fee(env: &Env, _admin: &Address, amount: i128) -> Result<(), Error> {
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
            admin: Address::generate(env),
            calculation_factors: testing::create_test_fee_calculation_factors(env),
        }
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
            soroban_sdk::vec![
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
                25_000_00,
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
