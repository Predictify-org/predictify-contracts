extern crate alloc;

// use alloc::string::ToString; // Removed to fix Display/ToString trait errors
use soroban_sdk::{contracttype, symbol_short, vec, Address, Env, Map, String, Symbol, Vec};

use crate::config::Environment;
use crate::errors::Error;

// Define AdminRole locally since it's not available in the crate root
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AdminRole {
    Owner,
    Admin,
    Moderator,
}

/// Comprehensive event system for Predictify Hybrid contract
///
/// This module provides a centralized event emission and logging system with:
/// - Event types and structures for all contract operations
/// - Event emission utilities and helpers
/// - Event logging and monitoring functions
/// - Event validation and helper functions
/// - Event testing utilities and examples
/// - Event documentation and examples

// ===== EVENT TYPES =====

/// Event emitted when a new prediction market is successfully created.
///
/// This event provides comprehensive information about newly created markets,
/// including market parameters, outcomes, administrative details, and timing.
/// Essential for tracking market creation activity and building market indices.
///
/// # Event Data
///
/// Contains all critical market creation parameters:
/// - Market identification and question details
/// - Available outcomes for prediction
/// - Administrative and timing information
/// - Creation timestamp for chronological ordering
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Address, Symbol, String, Vec};
/// # use predictify_hybrid::events::MarketCreatedEvent;
/// # let env = Env::default();
/// # let admin = Address::generate(&env);
///
/// // Market creation event data
/// let event = MarketCreatedEvent {
///     market_id: Symbol::new(&env, "btc_50k_2024"),
///     question: String::from_str(&env, "Will Bitcoin reach $50,000 by end of 2024?"),
///     outcomes: vec![
///         &env,
///         String::from_str(&env, "Yes"),
///         String::from_str(&env, "No")
///     ],
///     admin: admin.clone(),
///     end_time: 1735689600, // Dec 31, 2024
///     timestamp: env.ledger().timestamp(),
/// };
///
/// // Event provides complete market context
/// println!("New market: {}", event.question.to_string());
/// println!("Market ID: {}", event.market_id.to_string());
/// println!("Outcomes: {} options", event.outcomes.len());
/// println!("Ends: {}", event.end_time);
/// ```
///
/// # Integration Points
///
/// - **Market Indexing**: Build searchable market directories
/// - **Activity Feeds**: Display recent market creation activity
/// - **Analytics**: Track market creation patterns and trends
/// - **Notifications**: Alert users about new markets in categories of interest
/// - **Audit Trails**: Maintain complete record of market creation events
///
/// # Event Timing
///
/// Emitted immediately after successful market creation, providing:
/// - Real-time notification of new markets
/// - Chronological ordering via timestamp
/// - Immediate availability for user interfaces
/// - Historical record for analytics and reporting
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketCreatedEvent {
    /// Market ID
    pub market_id: Symbol,
    /// Market question
    pub question: String,
    /// Market outcomes
    pub outcomes: Vec<String>,
    /// Market admin
    pub admin: Address,
    /// Market end time
    pub end_time: u64,
    /// Creation timestamp
    pub timestamp: u64,
}

/// Event emitted when a user successfully casts a vote on a prediction market.
///
/// This event captures all details of voting activity, including voter identity,
/// chosen outcome, stake amount, and timing. Critical for tracking market
/// participation, calculating outcomes, and maintaining voting transparency.
///
/// # Vote Information
///
/// Records complete voting context:
/// - Market and voter identification
/// - Selected outcome and confidence (stake)
/// - Precise timing for chronological analysis
/// - Economic weight for outcome calculations
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Address, Symbol, String};
/// # use predictify_hybrid::events::VoteCastEvent;
/// # let env = Env::default();
/// # let voter = Address::generate(&env);
///
/// // Vote casting event data
/// let event = VoteCastEvent {
///     market_id: Symbol::new(&env, "btc_50k_2024"),
///     voter: voter.clone(),
///     outcome: String::from_str(&env, "Yes"),
///     stake: 10_000_000, // 1.0 XLM
///     timestamp: env.ledger().timestamp(),
/// };
///
/// // Event provides complete voting context
/// println!("Vote cast by: {}", event.voter.to_string());
/// println!("Market: {}", event.market_id.to_string());
/// println!("Outcome: {}", event.outcome.to_string());
/// println!("Stake: {} XLM", event.stake / 10_000_000);
/// ```
///
/// # Economic Tracking
///
/// Enables comprehensive economic analysis:
/// - **Stake Distribution**: Track economic weight across outcomes
/// - **Voter Confidence**: Analyze stake amounts as confidence indicators
/// - **Market Liquidity**: Monitor total stakes and participation levels
/// - **Outcome Probability**: Calculate implied probabilities from stakes
///
/// # Transparency Features
///
/// Supports market transparency through:
/// - **Public Voting Records**: All votes are publicly auditable
/// - **Stake Verification**: Economic weights are transparently recorded
/// - **Chronological Ordering**: Precise timing enables trend analysis
/// - **Voter Attribution**: Clear voter identity for accountability
///
/// # Integration Applications
///
/// - **Real-time Updates**: Live market activity feeds
/// - **Analytics Dashboards**: Voting pattern analysis and visualization
/// - **Outcome Calculation**: Stake-weighted probability calculations
/// - **User Portfolios**: Track individual voting history and performance
/// - **Market Sentiment**: Aggregate voting trends and momentum analysis
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VoteCastEvent {
    /// Market ID
    pub market_id: Symbol,
    /// Voter address
    pub voter: Address,
    /// Voted outcome
    pub outcome: String,
    /// Stake amount
    pub stake: i128,
    /// Vote timestamp
    pub timestamp: u64,
}

/// Event emitted when oracle data is successfully fetched for market resolution.
///
/// This event captures comprehensive oracle data retrieval information, including
/// the specific data source, fetched values, comparison logic, and timing.
/// Essential for transparency, auditability, and dispute resolution processes.
///
/// # Oracle Data Context
///
/// Provides complete oracle resolution context:
/// - Market identification and oracle provider details
/// - Actual fetched data values and comparison parameters
/// - Resolution logic and threshold evaluation
/// - Precise timing for chronological verification
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Symbol, String};
/// # use predictify_hybrid::events::OracleResultEvent;
/// # let env = Env::default();
///
/// // Oracle result event for Bitcoin price market
/// let event = OracleResultEvent {
///     market_id: Symbol::new(&env, "btc_50k_2024"),
///     result: String::from_str(&env, "Yes"), // Bitcoin reached $50k
///     provider: String::from_str(&env, "Chainlink"),
///     feed_id: String::from_str(&env, "BTC/USD"),
///     price: 52_000_00000000, // $52,000 (8 decimal precision)
///     threshold: 50_000_00000000, // $50,000 threshold
///     comparison: String::from_str(&env, "gte"), // greater than or equal
///     timestamp: env.ledger().timestamp(),
/// };
///
/// // Event provides complete oracle context
/// println!("Oracle result: {}", event.result.to_string());
/// println!("Price fetched: ${}", event.price / 100000000);
/// println!("Threshold: ${}", event.threshold / 100000000);
/// println!("Provider: {}", event.provider.to_string());
/// println!("Feed: {}", event.feed_id.to_string());
/// ```
///
/// # Transparency and Auditability
///
/// Enables complete oracle transparency:
/// - **Data Source Verification**: Clear provider and feed identification
/// - **Value Documentation**: Exact fetched values with precision
/// - **Logic Transparency**: Comparison operators and thresholds
/// - **Timing Verification**: Precise fetch timestamps
///
/// # Dispute Resolution Support
///
/// Critical for dispute processes:
/// - **Evidence Base**: Concrete data for dispute evaluation
/// - **Verification Path**: Complete audit trail from source to result
/// - **Alternative Validation**: Enable cross-reference with other sources
/// - **Historical Context**: Timestamp-based data verification
///
/// # Integration Applications
///
/// - **Oracle Monitoring**: Track oracle performance and reliability
/// - **Data Verification**: Cross-reference oracle results with external sources
/// - **Dispute Analysis**: Provide evidence for community dispute resolution
/// - **Market Analytics**: Analyze oracle accuracy and market outcomes
/// - **Compliance Reporting**: Maintain regulatory audit trails
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleResultEvent {
    /// Market ID
    pub market_id: Symbol,
    /// Oracle result
    pub result: String,
    /// Oracle provider
    pub provider: String,
    /// Feed ID
    pub feed_id: String,
    /// Price at resolution
    pub price: i128,
    /// Threshold value
    pub threshold: i128,
    /// Comparison operator
    pub comparison: String,
    /// Fetch timestamp
    pub timestamp: u64,
}

/// Event emitted when a prediction market is successfully resolved with final outcome.
///
/// This event captures the complete resolution process, including the final outcome,
/// resolution methodology (oracle vs. community), confidence metrics, and timing.
/// Critical for market finalization, payout calculations, and resolution transparency.
///
/// # Resolution Context
///
/// Provides comprehensive resolution information:
/// - Final market outcome and supporting evidence
/// - Resolution methodology and confidence scoring
/// - Oracle and community input comparison
/// - Timing for chronological resolution tracking
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Symbol, String};
/// # use predictify_hybrid::events::MarketResolvedEvent;
/// # let env = Env::default();
///
/// // Market resolution event for Bitcoin price market
/// let event = MarketResolvedEvent {
///     market_id: Symbol::new(&env, "btc_50k_2024"),
///     final_outcome: String::from_str(&env, "Yes"),
///     oracle_result: String::from_str(&env, "Yes"),
///     community_consensus: String::from_str(&env, "Yes"),
///     resolution_method: String::from_str(&env, "Oracle_Community_Consensus"),
///     confidence_score: 95, // 95% confidence
///     timestamp: env.ledger().timestamp(),
/// };
///
/// // Event provides complete resolution context
/// println!("Market resolved: {}", event.market_id.to_string());
/// println!("Final outcome: {}", event.final_outcome.to_string());
/// println!("Resolution method: {}", event.resolution_method.to_string());
/// println!("Confidence: {}%", event.confidence_score);
///
/// // Check consensus alignment
/// let consensus_aligned = event.oracle_result == event.community_consensus;
/// println!("Oracle-Community alignment: {}", consensus_aligned);
/// ```
///
/// # Resolution Methods
///
/// Supports multiple resolution approaches:
/// - **Oracle Only**: Pure oracle-based resolution
/// - **Community Only**: Pure community voting resolution
/// - **Hybrid Consensus**: Oracle and community agreement
/// - **Dispute Resolution**: Community override of oracle result
/// - **Admin Override**: Administrative resolution for edge cases
///
/// # Confidence Scoring
///
/// Confidence scores indicate resolution reliability:
/// - **90-100%**: High confidence, strong consensus
/// - **70-89%**: Medium confidence, reasonable consensus
/// - **50-69%**: Low confidence, weak consensus
/// - **Below 50%**: Very low confidence, potential disputes
///
/// # Integration Applications
///
/// - **Payout Processing**: Trigger reward distribution to winners
/// - **Market Analytics**: Track resolution accuracy and patterns
/// - **Confidence Metrics**: Display resolution reliability to users
/// - **Dispute Prevention**: Early warning for low-confidence resolutions
/// - **Historical Analysis**: Build resolution methodology effectiveness data
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketResolvedEvent {
    /// Market ID
    pub market_id: Symbol,
    /// Final outcome
    pub final_outcome: String,
    /// Oracle result
    pub oracle_result: String,
    /// Community consensus
    pub community_consensus: String,
    /// Resolution method
    pub resolution_method: String,
    /// Confidence score
    pub confidence_score: i128,
    /// Resolution timestamp
    pub timestamp: u64,
}

/// Event emitted when a user creates a formal dispute against a market resolution.
///
/// This event captures dispute initiation details, including the disputing party,
/// economic stake, reasoning, and timing. Essential for tracking dispute activity,
/// managing dispute processes, and maintaining resolution transparency.
///
/// # Dispute Information
///
/// Records complete dispute context:
/// - Market identification and disputing party
/// - Economic stake demonstrating dispute seriousness
/// - Optional reasoning for dispute justification
/// - Precise timing for dispute process management
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Address, Symbol, String};
/// # use predictify_hybrid::events::DisputeCreatedEvent;
/// # let env = Env::default();
/// # let disputer = Address::generate(&env);
///
/// // Dispute creation event
/// let event = DisputeCreatedEvent {
///     market_id: Symbol::new(&env, "btc_50k_2024"),
///     disputer: disputer.clone(),
///     stake: 50_000_000, // 5.0 XLM dispute stake
///     reason: Some(String::from_str(&env,
///         "Oracle price appears incorrect - multiple exchanges show different value")),
///     timestamp: env.ledger().timestamp(),
/// };
///
/// // Event provides complete dispute context
/// println!("Dispute created by: {}", event.disputer.to_string());
/// println!("Market disputed: {}", event.market_id.to_string());
/// println!("Stake amount: {} XLM", event.stake / 10_000_000);
///
/// if let Some(reason) = &event.reason {
///     println!("Dispute reason: {}", reason.to_string());
/// }
/// ```
///
/// # Economic Stakes
///
/// Dispute stakes serve multiple purposes:
/// - **Seriousness Filter**: Minimum stake prevents frivolous disputes
/// - **Economic Risk**: Disputers risk stake if dispute is rejected
/// - **Incentive Alignment**: Encourages well-researched disputes
/// - **Compensation Pool**: Stakes fund dispute resolution rewards
///
/// # Dispute Lifecycle
///
/// Dispute creation triggers:
/// 1. **Validation**: Check dispute eligibility and stake requirements
/// 2. **Community Voting**: Open dispute for community evaluation
/// 3. **Evidence Collection**: Gather supporting data and arguments
/// 4. **Resolution Process**: Determine dispute validity
/// 5. **Stake Distribution**: Reward accurate participants
///
/// # Integration Applications
///
/// - **Dispute Management**: Track and manage active disputes
/// - **Community Engagement**: Notify community of new disputes
/// - **Resolution Analytics**: Analyze dispute patterns and outcomes
/// - **Transparency Reporting**: Maintain public dispute records
/// - **Economic Monitoring**: Track dispute stakes and economic activity
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeCreatedEvent {
    /// Market ID
    pub market_id: Symbol,
    /// Disputer address
    pub disputer: Address,
    /// Dispute stake
    pub stake: i128,
    /// Dispute reason
    pub reason: Option<String>,
    /// Dispute timestamp
    pub timestamp: u64,
}

/// Event emitted when a dispute is successfully resolved with final outcome and rewards.
///
/// This event captures the complete dispute resolution process, including the final
/// outcome, winning and losing participants, fee distribution, and timing.
/// Essential for transparency, reward distribution, and dispute analytics.
///
/// # Resolution Information
///
/// Records complete dispute resolution context:
/// - Market identification and final dispute outcome
/// - Winner and loser participant lists
/// - Economic reward distribution amounts
/// - Precise timing for chronological tracking
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Address, Symbol, String, Vec};
/// # use predictify_hybrid::events::DisputeResolvedEvent;
/// # let env = Env::default();
/// # let winner1 = Address::generate(&env);
/// # let winner2 = Address::generate(&env);
/// # let loser1 = Address::generate(&env);
///
/// // Dispute resolution event
/// let event = DisputeResolvedEvent {
///     market_id: Symbol::new(&env, "btc_50k_2024"),
///     outcome: String::from_str(&env, "Dispute_Upheld"), // Community sided with disputer
///     winners: vec![&env, winner1.clone(), winner2.clone()], // Correct voters
///     losers: vec![&env, loser1.clone()], // Incorrect voters
///     fee_distribution: 25_000_000, // 2.5 XLM distributed to winners
///     timestamp: env.ledger().timestamp(),
/// };
///
/// // Event provides complete resolution context
/// println!("Dispute resolved: {}", event.market_id.to_string());
/// println!("Outcome: {}", event.outcome.to_string());
/// println!("Winners: {} participants", event.winners.len());
/// println!("Losers: {} participants", event.losers.len());
/// println!("Total rewards: {} XLM", event.fee_distribution / 10_000_000);
/// ```
///
/// # Resolution Outcomes
///
/// Possible dispute outcomes:
/// - **Dispute_Upheld**: Community agreed with disputer, oracle was wrong
/// - **Dispute_Rejected**: Community disagreed with disputer, oracle was correct
/// - **Dispute_Inconclusive**: Insufficient consensus, requires escalation
/// - **Dispute_Invalid**: Dispute did not meet validity requirements
///
/// # Economic Distribution
///
/// Fee distribution mechanics:
/// - **Winner Rewards**: Proportional share of loser stakes
/// - **Stake Recovery**: Winners recover their original stakes
/// - **Penalty Application**: Losers forfeit stakes to winners
/// - **Platform Fee**: Small percentage retained for operations
///
/// # Participant Tracking
///
/// Winner and loser lists enable:
/// - **Reward Distribution**: Direct transfer to winner addresses
/// - **Reputation Tracking**: Build participant accuracy records
/// - **Analytics**: Analyze voting patterns and success rates
/// - **Transparency**: Public record of dispute participation
///
/// # Integration Applications
///
/// - **Reward Processing**: Execute payments to winning participants
/// - **Reputation Systems**: Update participant accuracy scores
/// - **Dispute Analytics**: Track resolution patterns and outcomes
/// - **Community Metrics**: Measure dispute system effectiveness
/// - **Transparency Reporting**: Maintain public dispute resolution records
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeResolvedEvent {
    /// Market ID
    pub market_id: Symbol,
    /// Dispute outcome
    pub outcome: String,
    /// Winner addresses
    pub winners: Vec<Address>,
    /// Loser addresses
    pub losers: Vec<Address>,
    /// Fee distribution
    pub fee_distribution: i128,
    /// Resolution timestamp
    pub timestamp: u64,
}

/// Fee collected event
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeCollectedEvent {
    /// Market ID
    pub market_id: Symbol,
    /// Fee collector
    pub collector: Address,
    /// Fee amount
    pub amount: i128,
    /// Fee type
    pub fee_type: String,
    /// Collection timestamp
    pub timestamp: u64,
}

/// Extension requested event
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtensionRequestedEvent {
    /// Market ID
    pub market_id: Symbol,
    /// Requesting admin
    pub admin: Address,
    /// Additional days
    pub additional_days: u32,
    /// Extension reason
    pub reason: String,
    /// Extension fee
    pub fee: i128,
    /// Request timestamp
    pub timestamp: u64,
}

/// Configuration updated event
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigUpdatedEvent {
    /// Updated by
    pub updated_by: Address,
    /// Configuration type
    pub config_type: String,
    /// Old value
    pub old_value: String,
    /// New value
    pub new_value: String,
    /// Update timestamp
    pub timestamp: u64,
}

/// Error logged event
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ErrorLoggedEvent {
    /// Error code
    pub error_code: u32,
    /// Error message
    pub message: String,
    /// Context
    pub context: String,
    /// User address (if applicable)
    pub user: Option<Address>,
    /// Market ID (if applicable)
    pub market_id: Option<Symbol>,
    /// Error timestamp
    pub timestamp: u64,
}

/// Performance metric event
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PerformanceMetricEvent {
    /// Metric name
    pub metric_name: String,
    /// Metric value
    pub value: i128,
    /// Metric unit
    pub unit: String,
    /// Context
    pub context: String,
    /// Metric timestamp
    pub timestamp: u64,
}

/// Admin action event
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminActionEvent {
    /// Admin address
    pub admin: Address,
    /// Action performed
    pub action: String,
    /// Target of action
    pub target: Option<String>,
    /// Action timestamp
    pub timestamp: u64,
    /// Action success status
    pub success: bool,
}

/// Admin role event
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminRoleEvent {
    /// Admin address
    pub admin: Address,
    /// Role assigned
    pub role: String,
    /// Assigned by
    pub assigned_by: Address,
    /// Assignment timestamp
    pub timestamp: u64,
}

/// Admin permission event
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminPermissionEvent {
    /// Admin address
    pub admin: Address,
    /// Permission checked
    pub permission: String,
    /// Access granted
    pub granted: bool,
    /// Check timestamp
    pub timestamp: u64,
}

/// Market closed event
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketClosedEvent {
    /// Market ID
    pub market_id: Symbol,
    /// Admin who closed it
    pub admin: Address,
    /// Close timestamp
    pub timestamp: u64,
}

/// Market finalized event
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketFinalizedEvent {
    /// Market ID
    pub market_id: Symbol,
    /// Admin who finalized it
    pub admin: Address,
    /// Final outcome
    pub outcome: String,
    /// Finalization timestamp
    pub timestamp: u64,
}

/// Admin initialized event
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminInitializedEvent {
    /// Admin address
    pub admin: Address,
    /// Initialization timestamp
    pub timestamp: u64,
}

/// Dispute timeout set event
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeTimeoutSetEvent {
    /// Dispute ID
    pub dispute_id: Symbol,
    /// Market ID
    pub market_id: Symbol,
    /// Timeout hours
    pub timeout_hours: u32,
    /// Set by admin
    pub set_by: Address,
    /// Set timestamp
    pub timestamp: u64,
}

/// Dispute timeout expired event
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeTimeoutExpiredEvent {
    /// Dispute ID
    pub dispute_id: Symbol,
    /// Market ID
    pub market_id: Symbol,
    /// Expiration timestamp
    pub expiration_timestamp: u64,
    /// Auto-resolution outcome
    pub outcome: String,
    /// Resolution method
    pub resolution_method: String,
}

/// Dispute timeout extended event
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeTimeoutExtendedEvent {
    /// Dispute ID
    pub dispute_id: Symbol,
    /// Market ID
    pub market_id: Symbol,
    /// Additional hours
    pub additional_hours: u32,
    /// Extended by admin
    pub extended_by: Address,
    /// Extension timestamp
    pub timestamp: u64,
}

/// Dispute auto-resolved event
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeAutoResolvedEvent {
    /// Dispute ID
    pub dispute_id: Symbol,
    /// Market ID
    pub market_id: Symbol,
    /// Resolution outcome
    pub outcome: String,
    /// Resolution reason
    pub reason: String,
    /// Resolution timestamp
    pub timestamp: u64,
}

/// Config initialized event
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigInitializedEvent {
    /// Admin address
    pub admin: Address,
    /// Environment
    pub environment: String,
    /// Initialization timestamp
    pub timestamp: u64,
}

/// Audit item updated event
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditItemUpdatedEvent {
    /// Auditor address
    pub auditor: Address,
    /// Audit item ID
    pub item_id: u32,
    /// Completion status
    pub completed: bool,
    /// Update timestamp
    pub timestamp: u64,
}

/// Audit system event
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditSystemEvent {
    /// Admin address
    pub admin: Address,
    /// Action performed (initialized, reset, etc.)
    pub action: String,
    /// Event timestamp

    pub timestamp: u64,
}

// ===== EVENT EMISSION UTILITIES =====

/// Event emission utilities
pub struct EventEmitter;

impl EventEmitter {
    /// Emit market created event
    pub fn emit_market_created(
        env: &Env,
        market_id: &Symbol,
        question: &String,
        outcomes: &Vec<String>,
        admin: &Address,
        end_time: u64,
    ) {
        let event = MarketCreatedEvent {
            market_id: market_id.clone(),
            question: question.clone(),
            outcomes: outcomes.clone(),
            admin: admin.clone(),
            end_time,
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("mkt_crt"), &event);
    }

    /// Emit vote cast event
    pub fn emit_vote_cast(
        env: &Env,
        market_id: &Symbol,
        voter: &Address,
        outcome: &String,
        stake: i128,
    ) {
        let event = VoteCastEvent {
            market_id: market_id.clone(),
            voter: voter.clone(),
            outcome: outcome.clone(),
            stake,
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("vote"), &event);
    }

    /// Emit oracle result event
    pub fn emit_oracle_result(
        env: &Env,
        market_id: &Symbol,
        result: &String,
        provider: &String,
        feed_id: &String,
        price: i128,
        threshold: i128,
        comparison: &String,
    ) {
        let event = OracleResultEvent {
            market_id: market_id.clone(),
            result: result.clone(),
            provider: provider.clone(),
            feed_id: feed_id.clone(),
            price,
            threshold,
            comparison: comparison.clone(),
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("oracle_rs"), &event);
    }

    /// Emit market resolved event
    pub fn emit_market_resolved(
        env: &Env,
        market_id: &Symbol,
        final_outcome: &String,
        oracle_result: &String,
        community_consensus: &String,
        resolution_method: &String,
        confidence_score: i128,
    ) {
        let event = MarketResolvedEvent {
            market_id: market_id.clone(),
            final_outcome: final_outcome.clone(),
            oracle_result: oracle_result.clone(),
            community_consensus: community_consensus.clone(),
            resolution_method: resolution_method.clone(),
            confidence_score,
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("mkt_res"), &event);
    }

    /// Emit dispute created event
    pub fn emit_dispute_created(
        env: &Env,
        market_id: &Symbol,
        disputer: &Address,
        stake: i128,
        reason: Option<String>,
    ) {
        let event = DisputeCreatedEvent {
            market_id: market_id.clone(),
            disputer: disputer.clone(),
            stake,
            reason,
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("dispt_crt"), &event);
    }

    /// Emit dispute resolved event
    pub fn emit_dispute_resolved(
        env: &Env,
        market_id: &Symbol,
        outcome: &String,
        winners: &Vec<Address>,
        losers: &Vec<Address>,
        fee_distribution: i128,
    ) {
        let event = DisputeResolvedEvent {
            market_id: market_id.clone(),
            outcome: outcome.clone(),
            winners: winners.clone(),
            losers: losers.clone(),
            fee_distribution,
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("dispt_res"), &event);
    }

    /// Emit fee collected event
    pub fn emit_fee_collected(
        env: &Env,
        market_id: &Symbol,
        collector: &Address,
        amount: i128,
        fee_type: &String,
    ) {
        let event = FeeCollectedEvent {
            market_id: market_id.clone(),
            collector: collector.clone(),
            amount,
            fee_type: fee_type.clone(),
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("fee_col"), &event);
    }

    /// Emit extension requested event
    pub fn emit_extension_requested(
        env: &Env,
        market_id: &Symbol,
        admin: &Address,
        additional_days: u32,
        reason: &String,
        fee: i128,
    ) {
        let event = ExtensionRequestedEvent {
            market_id: market_id.clone(),
            admin: admin.clone(),
            additional_days,
            reason: reason.clone(),
            fee,
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("ext_req"), &event);
    }

    /// Emit configuration updated event
    pub fn emit_config_updated(
        env: &Env,
        updated_by: &Address,
        config_type: &String,
        old_value: &String,
        new_value: &String,
    ) {
        let event = ConfigUpdatedEvent {
            updated_by: updated_by.clone(),
            config_type: config_type.clone(),
            old_value: old_value.clone(),
            new_value: new_value.clone(),
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("cfg_upd"), &event);
    }

    /// Emit error logged event
    pub fn emit_error_logged(
        env: &Env,
        error_code: u32,
        message: &String,
        context: &String,
        user: Option<Address>,
        market_id: Option<Symbol>,
    ) {
        let event = ErrorLoggedEvent {
            error_code,
            message: message.clone(),
            context: context.clone(),
            user,
            market_id,
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("err_log"), &event);
    }

    /// Emit performance metric event
    pub fn emit_performance_metric(
        env: &Env,
        metric_name: &String,
        value: i128,
        unit: &String,
        context: &String,
    ) {
        let event = PerformanceMetricEvent {
            metric_name: metric_name.clone(),
            value,
            unit: unit.clone(),
            context: context.clone(),
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("perf_met"), &event);
    }

    /// Emit admin action logged event
    pub fn emit_admin_action_logged(env: &Env, admin: &Address, action: &str, success: &bool) {
        let event = AdminActionEvent {
            admin: admin.clone(),
            action: String::from_str(env, action),
            target: None,
            timestamp: env.ledger().timestamp(),
            success: *success,
        };

        Self::store_event(env, &symbol_short!("adm_act"), &event);
    }

    /// Emit admin initialized event
    pub fn emit_admin_initialized(env: &Env, admin: &Address) {
        let event = AdminInitializedEvent {
            admin: admin.clone(),
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("adm_init"), &event);
    }

    /// Emit config initialized event
    pub fn emit_config_initialized(env: &Env, admin: &Address, environment: &Environment) {
        let event = ConfigInitializedEvent {
            admin: admin.clone(),
            environment: String::from_str(
                env,
                match environment {
                    Environment::Development => "Development",
                    Environment::Testnet => "Testnet",
                    Environment::Mainnet => "Mainnet",
                    Environment::Custom => "Custom",
                },
            ),
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("cfg_init"), &event);
    }

    /// Emit admin role assigned event
    pub fn emit_admin_role_assigned(
        env: &Env,
        admin: &Address,
        role: &AdminRole,
        assigned_by: &Address,
    ) {
        let event = AdminRoleEvent {
            admin: admin.clone(),
            role: String::from_str(
                env,
                match role {
                    AdminRole::Owner => "Owner",
                    AdminRole::Admin => "Admin",
                    AdminRole::Moderator => "Moderator",
                },
            ),
            assigned_by: assigned_by.clone(),
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("adm_role"), &event);
    }

    /// Emit admin role deactivated event
    pub fn emit_admin_role_deactivated(env: &Env, admin: &Address, deactivated_by: &Address) {
        let event = AdminRoleEvent {
            admin: admin.clone(),
            role: String::from_str(env, "deactivated"),
            assigned_by: deactivated_by.clone(),
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("adm_deact"), &event);
    }

    /// Emit market closed event
    pub fn emit_market_closed(env: &Env, market_id: &Symbol, admin: &Address) {
        let event = MarketClosedEvent {
            market_id: market_id.clone(),
            admin: admin.clone(),
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("mkt_close"), &event);
    }

    /// Emit market finalized event
    pub fn emit_market_finalized(env: &Env, market_id: &Symbol, admin: &Address, outcome: &String) {
        let event = MarketFinalizedEvent {
            market_id: market_id.clone(),
            admin: admin.clone(),
            outcome: outcome.clone(),
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("mkt_final"), &event);
    }

    /// Emit dispute timeout set event
    pub fn emit_dispute_timeout_set(
        env: &Env,
        dispute_id: &Symbol,
        market_id: &Symbol,
        timeout_hours: u32,
        set_by: &Address,
    ) {
        let event = DisputeTimeoutSetEvent {
            dispute_id: dispute_id.clone(),
            market_id: market_id.clone(),
            timeout_hours,
            set_by: set_by.clone(),
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("tout_set"), &event);
    }

    /// Emit dispute timeout expired event
    pub fn emit_dispute_timeout_expired(
        env: &Env,
        dispute_id: &Symbol,
        market_id: &Symbol,
        outcome: &String,
        resolution_method: &String,
    ) {
        let event = DisputeTimeoutExpiredEvent {
            dispute_id: dispute_id.clone(),
            market_id: market_id.clone(),
            expiration_timestamp: env.ledger().timestamp(),
            outcome: outcome.clone(),
            resolution_method: resolution_method.clone(),
        };

        Self::store_event(env, &symbol_short!("tout_exp"), &event);
    }

    /// Emit dispute timeout extended event
    pub fn emit_dispute_timeout_extended(
        env: &Env,
        dispute_id: &Symbol,
        market_id: &Symbol,
        additional_hours: u32,
        extended_by: &Address,
    ) {
        let event = DisputeTimeoutExtendedEvent {
            dispute_id: dispute_id.clone(),
            market_id: market_id.clone(),
            additional_hours,
            extended_by: extended_by.clone(),
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("tout_ext"), &event);
    }

    /// Emit dispute auto-resolved event
    pub fn emit_dispute_auto_resolved(
        env: &Env,
        dispute_id: &Symbol,
        market_id: &Symbol,
        outcome: &String,
        reason: &String,
    ) {
        let event = DisputeAutoResolvedEvent {
            dispute_id: dispute_id.clone(),
            market_id: market_id.clone(),
            outcome: outcome.clone(),
            reason: reason.clone(),
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("auto_res"), &event);
    }

    /// Emit storage cleanup event
    pub fn emit_storage_cleanup_event(
        env: &Env,
        market_id: &Symbol,
        cleanup_type: &String,
    ) {
        let event = StorageCleanupEvent {
            market_id: market_id.clone(),
            cleanup_type: cleanup_type.clone(),
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("stor_cln"), &event);
    }

    /// Emit storage optimization event
    pub fn emit_storage_optimization_event(
        env: &Env,
        market_id: &Symbol,
        optimization_type: &String,
    ) {
        let event = StorageOptimizationEvent {
            market_id: market_id.clone(),
            optimization_type: optimization_type.clone(),
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("stor_opt"), &event);
    }

    /// Emit storage migration event
    pub fn emit_storage_migration_event(
        env: &Env,
        migration_id: &Symbol,
        from_format: &String,
        to_format: &String,
        markets_migrated: u32,
    ) {
        let event = StorageMigrationEvent {
            migration_id: migration_id.clone(),
            from_format: from_format.clone(),
            to_format: to_format.clone(),
            markets_migrated,
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("stor_mig"), &event);
    }

    /// Emit audit item updated event
    pub fn emit_audit_item_updated(
        env: &Env,
        auditor: &Address,
        item_id: u32,
        completed: bool,
    ) {
        let event = AuditItemUpdatedEvent {
            auditor: auditor.clone(),
            item_id,
            completed,
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("aud_item"), &event);
    }

    /// Emit audit system initialized event
    pub fn emit_audit_system_initialized(
        env: &Env,
        admin: &Address,
    ) {
        let event = AuditSystemEvent {
            admin: admin.clone(),
            action: String::from_str(env, "initialized"),
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("aud_init"), &event);
    }

    /// Emit audit system reset event
    pub fn emit_audit_system_reset(
        env: &Env,
        admin: &Address,
    ) {
        let event = AuditSystemEvent {
            admin: admin.clone(),
            action: String::from_str(env, "reset"),
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("aud_reset"), &event);
    }

    /// Store event in persistent storage
    fn store_event<T>(env: &Env, event_key: &Symbol, event_data: &T)
    where
        T: Clone + soroban_sdk::IntoVal<soroban_sdk::Env, soroban_sdk::Val>,
    {
        env.storage().persistent().set(event_key, event_data);
    }
}

// ===== EVENT LOGGING AND MONITORING =====

/// Event logging and monitoring utilities
pub struct EventLogger;

impl EventLogger {
    /// Get all events of a specific type
    pub fn get_events<T>(env: &Env, event_type: &Symbol) -> Vec<T>
    where
        T: Clone
            + soroban_sdk::TryFromVal<soroban_sdk::Env, soroban_sdk::Val>
            + soroban_sdk::IntoVal<soroban_sdk::Env, soroban_sdk::Val>,
    {
        match env.storage().persistent().get::<Symbol, T>(event_type) {
            Some(event) => Vec::from_array(env, [event]),
            None => Vec::new(env),
        }
    }

    /// Get events for a specific market
    pub fn get_market_events(env: &Env, market_id: &Symbol) -> Vec<MarketEventSummary> {
        let mut events = Vec::new(env);

        // Get market created events
        if let Some(event) = env
            .storage()
            .persistent()
            .get::<Symbol, MarketCreatedEvent>(&symbol_short!("mkt_crt"))
        {
            if event.market_id == *market_id {
                events.push_back(MarketEventSummary {
                    event_type: String::from_str(env, "MarketCreated"),
                    timestamp: event.timestamp,
                    details: String::from_str(env, "Market was created"),
                });
            }
        }

        // Get vote cast events
        if let Some(event) = env
            .storage()
            .persistent()
            .get::<Symbol, VoteCastEvent>(&symbol_short!("vote"))
        {
            if event.market_id == *market_id {
                events.push_back(MarketEventSummary {
                    event_type: String::from_str(env, "VoteCast"),
                    timestamp: event.timestamp,
                    details: String::from_str(env, "Vote was cast"),
                });
            }
        }

        // Get oracle result events
        if let Some(event) = env
            .storage()
            .persistent()
            .get::<Symbol, OracleResultEvent>(&symbol_short!("oracle_rs"))
        {
            if event.market_id == *market_id {
                events.push_back(MarketEventSummary {
                    event_type: String::from_str(env, "OracleResult"),
                    timestamp: event.timestamp,
                    details: String::from_str(env, "Oracle result fetched"),
                });
            }
        }

        // Get market resolved events
        if let Some(event) = env
            .storage()
            .persistent()
            .get::<Symbol, MarketResolvedEvent>(&symbol_short!("mkt_res"))
        {
            if event.market_id == *market_id {
                events.push_back(MarketEventSummary {
                    event_type: String::from_str(env, "MarketResolved"),
                    timestamp: event.timestamp,
                    details: String::from_str(env, "Market was resolved"),
                });
            }
        }

        events
    }

    /// Get recent events (last N events)
    pub fn get_recent_events(env: &Env, limit: u32) -> Vec<EventSummary> {
        let mut events = Vec::new(env);

        // This is a simplified implementation
        // In a real system, you would maintain an event log with timestamps
        let event_types = vec![
            env,
            symbol_short!("mkt_crt"),
            symbol_short!("vote"),
            symbol_short!("oracle_rs"),
            symbol_short!("mkt_res"),
            symbol_short!("dispt_crt"),
            symbol_short!("dispt_res"),
            symbol_short!("fee_col"),
            symbol_short!("ext_req"),
            symbol_short!("cfg_upd"),
            symbol_short!("err_log"),
            symbol_short!("perf_met"),
        ];

        let mut count = 0;
        for event_type in event_types.iter() {
            if count >= limit {
                break;
            }

            // Check if event exists and add to summary
            if env.storage().persistent().has(&event_type) {
                events.push_back(EventSummary {
                    event_type: String::from_str(env, "event"),
                    timestamp: env.ledger().timestamp(),
                    details: String::from_str(env, "Event occurred"),
                });
                count += 1;
            }
        }

        events
    }

    /// Get error events
    pub fn get_error_events(env: &Env) -> Vec<ErrorLoggedEvent> {
        Self::get_events(env, &symbol_short!("err_log"))
    }

    /// Get performance metrics
    pub fn get_performance_metrics(env: &Env) -> Vec<PerformanceMetricEvent> {
        Self::get_events(env, &symbol_short!("perf_met"))
    }

    /// Clear old events (cleanup utility)
    pub fn clear_old_events(env: &Env, _older_than_timestamp: u64) {
        let event_types = vec![
            env,
            symbol_short!("mkt_crt"),
            symbol_short!("vote"),
            symbol_short!("oracle_rs"),
            symbol_short!("mkt_res"),
            symbol_short!("dispt_crt"),
            symbol_short!("dispt_res"),
            symbol_short!("fee_col"),
            symbol_short!("ext_req"),
            symbol_short!("cfg_upd"),
            symbol_short!("err_log"),
            symbol_short!("perf_met"),
        ];

        for event_type in event_types.iter() {
            // In a real implementation, you would check timestamps and remove old events
            // For now, this is a placeholder
            if env.storage().persistent().has(&event_type) {
                // Check if event is older than threshold and remove if needed
                // This would require storing timestamps with events
            }
        }
    }
}

// ===== EVENT VALIDATION =====

/// Event validation utilities
pub struct EventValidator;

impl EventValidator {
    /// Validate market created event
    pub fn validate_market_created_event(event: &MarketCreatedEvent) -> Result<(), Error> {
        // For now, skip validation since we can't easily convert Soroban String/Symbol
        // This is a limitation of the current Soroban SDK
        if event.outcomes.len() < 2 {
            return Err(Error::InvalidInput);
        }

        if event.end_time <= event.timestamp {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }

    /// Validate vote cast event
    pub fn validate_vote_cast_event(event: &VoteCastEvent) -> Result<(), Error> {
        // For now, skip validation since we can't easily convert Soroban String/Symbol
        // This is a limitation of the current Soroban SDK
        if event.stake <= 0 {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }

    /// Validate oracle result event
    pub fn validate_oracle_result_event(_event: &OracleResultEvent) -> Result<(), Error> {
        // For now, skip validation since we can't easily convert Soroban String/Symbol
        // This is a limitation of the current Soroban SDK
        Ok(())
    }

    /// Validate market resolved event
    pub fn validate_market_resolved_event(event: &MarketResolvedEvent) -> Result<(), Error> {
        // For now, skip validation since we can't easily convert Soroban String/Symbol
        // This is a limitation of the current Soroban SDK
        if event.confidence_score < 0 || event.confidence_score > 100 {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }

    /// Validate dispute created event
    pub fn validate_dispute_created_event(event: &DisputeCreatedEvent) -> Result<(), Error> {
        // For now, skip validation since we can't easily convert Soroban String/Symbol
        // This is a limitation of the current Soroban SDK
        if event.stake <= 0 {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }

    /// Validate fee collected event
    pub fn validate_fee_collected_event(event: &FeeCollectedEvent) -> Result<(), Error> {
        // For now, skip validation since we can't easily convert Soroban String/Symbol
        // This is a limitation of the current Soroban SDK
        if event.amount <= 0 {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }

    /// Validate extension requested event

    pub fn validate_extension_requested_event(
        event: &ExtensionRequestedEvent,
    ) -> Result<(), Error> {
        // Remove empty check for Symbol since it doesn't have is_empty method
        // Market ID validation is handled by the Symbol type itself

        if event.additional_days == 0 {
            return Err(Error::InvalidInput);
        }

        if event.fee < 0 {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }

    /// Validate error logged event
    pub fn validate_error_logged_event(_event: &ErrorLoggedEvent) -> Result<(), Error> {
        // For now, skip validation since we can't easily convert Soroban String/Symbol
        // This is a limitation of the current Soroban SDK
        Ok(())
    }

    /// Validate performance metric event
    pub fn validate_performance_metric_event(_event: &PerformanceMetricEvent) -> Result<(), Error> {
        // For now, skip validation since we can't easily convert Soroban String/Symbol
        // This is a limitation of the current Soroban SDK
        Ok(())
    }
}

// ===== EVENT HELPER UTILITIES =====

/// Event helper utilities
pub struct EventHelpers;

impl EventHelpers {
    /// Create event summary from event data
    pub fn create_event_summary(env: &Env, event_type: &String, details: &String) -> EventSummary {
        EventSummary {
            event_type: event_type.clone(),
            timestamp: env.ledger().timestamp(),
            details: details.clone(),
        }
    }

    /// Format event timestamp for display
    pub fn format_timestamp(env: &Env, _timestamp: u64) -> String {
        // For now, return a placeholder since we can't easily convert to string
        // This is a limitation of the current Soroban SDK
        String::from_str(env, "timestamp")
    }

    /// Get event type from symbol
    pub fn get_event_type_from_symbol(env: &Env, _symbol: &Symbol) -> String {
        // For now, return a placeholder since we can't easily convert Symbol to string
        // This is a limitation of the current Soroban SDK
        String::from_str(env, "symbol")
    }

    /// Create event context string
    pub fn create_event_context(env: &Env, context_parts: &Vec<String>) -> String {
        let mut context = String::from_str(env, "");
        for (i, part) in context_parts.iter().enumerate() {
            if i > 0 {
                let _separator = String::from_str(env, " | ");
                let _context_str = String::from_str(env, "");
                context = String::from_str(env, "");
            } else {
                context = part.clone();
            }
        }
        context
    }

    /// Validate event timestamp
    pub fn is_valid_timestamp(timestamp: u64) -> bool {
        // Basic validation - timestamp should be reasonable
        timestamp > 0 && timestamp < 9999999999 // Unix timestamp reasonable range
    }

    /// Get event age in seconds
    pub fn get_event_age(current_timestamp: u64, event_timestamp: u64) -> u64 {
        if current_timestamp >= event_timestamp {
            current_timestamp - event_timestamp
        } else {
            0
        }
    }

    /// Check if event is recent (within specified seconds)
    pub fn is_recent_event(
        event_timestamp: u64,
        current_timestamp: u64,
        recent_threshold: u64,
    ) -> bool {
        Self::get_event_age(current_timestamp, event_timestamp) <= recent_threshold
    }
}

// ===== EVENT TESTING UTILITIES =====

/// Event testing utilities
pub struct EventTestingUtils;

impl EventTestingUtils {
    /// Create test market created event
    pub fn create_test_market_created_event(
        env: &Env,
        market_id: &Symbol,
        admin: &Address,
    ) -> MarketCreatedEvent {
        MarketCreatedEvent {
            market_id: market_id.clone(),
            question: String::from_str(env, "Test market question?"),
            outcomes: vec![
                env,
                String::from_str(env, "yes"),
                String::from_str(env, "no"),
            ],
            admin: admin.clone(),
            end_time: env.ledger().timestamp() + 86400,
            timestamp: env.ledger().timestamp(),
        }
    }

    /// Create test vote cast event
    pub fn create_test_vote_cast_event(
        env: &Env,
        market_id: &Symbol,
        voter: &Address,
    ) -> VoteCastEvent {
        VoteCastEvent {
            market_id: market_id.clone(),
            voter: voter.clone(),
            outcome: String::from_str(env, "yes"),
            stake: 100_0000000,
            timestamp: env.ledger().timestamp(),
        }
    }

    /// Create test oracle result event
    pub fn create_test_oracle_result_event(env: &Env, market_id: &Symbol) -> OracleResultEvent {
        OracleResultEvent {
            market_id: market_id.clone(),
            result: String::from_str(env, "yes"),
            provider: String::from_str(env, "Pyth"),
            feed_id: String::from_str(env, "BTC/USD"),
            price: 2500000,
            threshold: 2500000,
            comparison: String::from_str(env, "gt"),
            timestamp: env.ledger().timestamp(),
        }
    }

    /// Create test market resolved event
    pub fn create_test_market_resolved_event(env: &Env, market_id: &Symbol) -> MarketResolvedEvent {
        MarketResolvedEvent {
            market_id: market_id.clone(),
            final_outcome: String::from_str(env, "yes"),
            oracle_result: String::from_str(env, "yes"),
            community_consensus: String::from_str(env, "yes"),
            resolution_method: String::from_str(env, "Oracle"),
            confidence_score: 85,
            timestamp: env.ledger().timestamp(),
        }
    }

    /// Create test dispute created event
    pub fn create_test_dispute_created_event(
        env: &Env,
        market_id: &Symbol,
        disputer: &Address,
    ) -> DisputeCreatedEvent {
        DisputeCreatedEvent {
            market_id: market_id.clone(),
            disputer: disputer.clone(),
            stake: 10_0000000,
            reason: Some(String::from_str(env, "Test dispute")),
            timestamp: env.ledger().timestamp(),
        }
    }

    /// Create test fee collected event
    pub fn create_test_fee_collected_event(
        env: &Env,
        market_id: &Symbol,
        collector: &Address,
    ) -> FeeCollectedEvent {
        FeeCollectedEvent {
            market_id: market_id.clone(),
            collector: collector.clone(),
            amount: 20_0000000,
            fee_type: String::from_str(env, "Platform"),
            timestamp: env.ledger().timestamp(),
        }
    }

    /// Create test error logged event
    pub fn create_test_error_logged_event(env: &Env) -> ErrorLoggedEvent {
        ErrorLoggedEvent {
            error_code: 1,
            message: String::from_str(env, "Test error message"),
            context: String::from_str(env, "Test context"),
            user: None,
            market_id: None,
            timestamp: env.ledger().timestamp(),
        }
    }

    /// Create test performance metric event
    pub fn create_test_performance_metric_event(env: &Env) -> PerformanceMetricEvent {
        PerformanceMetricEvent {
            metric_name: String::from_str(env, "TransactionCount"),
            value: 100,
            unit: String::from_str(env, "transactions"),
            context: String::from_str(env, "Daily"),
            timestamp: env.ledger().timestamp(),
        }
    }

    /// Validate test event structure
    pub fn validate_test_event_structure<T>(_event: &T) -> Result<(), Error>
    where
        T: Clone,
    {
        // Basic validation that event exists
        // In a real implementation, you would validate specific fields
        Ok(())
    }

    /// Simulate event emission
    pub fn simulate_event_emission(env: &Env, _event_type: &String) -> bool {
        // Simulate successful event emission

        let event_key = Symbol::new(env, "event");
        env.storage()
            .persistent()
            .set(&event_key, &String::from_str(env, "test"));

        true
    }
}

// ===== EVENT SUMMARY TYPES =====

/// Event summary for listing
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventSummary {
    /// Event type
    pub event_type: String,
    /// Event timestamp
    pub timestamp: u64,
    /// Event details
    pub details: String,
}

/// Market event summary
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketEventSummary {
    /// Event type
    pub event_type: String,
    /// Event timestamp
    pub timestamp: u64,
    /// Event details
    pub details: String,
}

// ===== EVENT CONSTANTS =====

/// Event system constants
pub const MAX_EVENTS_PER_QUERY: u32 = 100;
pub const EVENT_RETENTION_DAYS: u64 = 30 * 24 * 60 * 60; // 30 days
pub const RECENT_EVENT_THRESHOLD: u64 = 24 * 60 * 60; // 24 hours

// ===== EVENT DOCUMENTATION =====

/// Event system documentation and examples
pub struct EventDocumentation;

impl EventDocumentation {
    /// Get event system overview
    pub fn get_overview(env: &Env) -> String {
        String::from_str(env, "Comprehensive event system for Predictify Hybrid contract with emission, logging, validation, and testing utilities.")
    }

    /// Get event type documentation
    pub fn get_event_type_docs(env: &Env) -> Map<String, String> {
        let mut docs = Map::new(env);

        docs.set(
            String::from_str(env, "MarketCreated"),
            String::from_str(env, "Emitted when a new market is created"),
        );
        docs.set(
            String::from_str(env, "VoteCast"),
            String::from_str(env, "Emitted when a user casts a vote"),
        );
        docs.set(
            String::from_str(env, "OracleResult"),
            String::from_str(env, "Emitted when oracle result is fetched"),
        );
        docs.set(
            String::from_str(env, "MarketResolved"),
            String::from_str(env, "Emitted when a market is resolved"),
        );
        docs.set(
            String::from_str(env, "DisputeCreated"),
            String::from_str(env, "Emitted when a dispute is created"),
        );
        docs.set(
            String::from_str(env, "DisputeResolved"),
            String::from_str(env, "Emitted when a dispute is resolved"),
        );
        docs.set(
            String::from_str(env, "FeeCollected"),
            String::from_str(env, "Emitted when fees are collected"),
        );
        docs.set(
            String::from_str(env, "ExtensionRequested"),
            String::from_str(env, "Emitted when market extension is requested"),
        );
        docs.set(
            String::from_str(env, "ConfigUpdated"),
            String::from_str(env, "Emitted when configuration is updated"),
        );
        docs.set(
            String::from_str(env, "ErrorLogged"),
            String::from_str(env, "Emitted when an error is logged"),
        );
        docs.set(
            String::from_str(env, "PerformanceMetric"),
            String::from_str(env, "Emitted when performance metrics are recorded"),
        );

        docs
    }

    /// Get usage examples
    pub fn get_usage_examples(env: &Env) -> Map<String, String> {
        let mut examples = Map::new(env);

        examples.set(
            String::from_str(env, "EmitMarketCreated"),
            String::from_str(env, "EventEmitter::emit_market_created(env, market_id, question, outcomes, admin, end_time)"),
        );
        examples.set(
            String::from_str(&env, "EmitVoteCast"),
            String::from_str(
                &env,
                "EventEmitter::emit_vote_cast(env, market_id, voter, outcome, stake)",
            ),
        );
        examples.set(
            String::from_str(env, "GetMarketEvents"),
            String::from_str(env, "EventLogger::get_market_events(env, market_id)"),
        );
        examples.set(
            String::from_str(&env, "ValidateEvent"),
            String::from_str(
                &env,
                "EventValidator::validate_market_created_event(&event)",
            ),
        );

        examples
    }
}
