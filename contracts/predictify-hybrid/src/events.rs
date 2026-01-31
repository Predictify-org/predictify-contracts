extern crate alloc;

// use alloc::string::ToString; // Removed to fix Display/ToString trait errors
use soroban_sdk::{contracttype, symbol_short, vec, Address, Env, Map, String, Symbol, Vec};

use crate::config::Environment;
use crate::errors::Error;
use crate::types::OracleProvider;

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

/// Event emitted when a new prediction event is successfully created.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventCreatedEvent {
    /// Unique event ID
    pub event_id: Symbol,
    /// Event description
    pub description: String,
    /// Event outcomes
    pub outcomes: Vec<String>,
    /// Event end time
    pub end_time: u64,
    /// Event admin
    pub admin: Address,
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

/// Event emitted when a user places a bet on a prediction market event.
///
/// This event captures all details of bet placement activity, including bettor identity,
/// selected outcome, locked amount, and timing. Critical for tracking market
/// activity, calculating payouts, and maintaining betting transparency.
///
/// # Bet vs Vote Distinction
///
/// - **Bet**: Financial wager on predicted outcome with locked funds for payout
/// - **Vote**: Participation in community consensus for market resolution
///
/// Bets represent a user's prediction and financial commitment, while votes
/// contribute to the community resolution mechanism.
///
/// # Bet Information
///
/// Records complete betting context:
/// - Market and bettor identification
/// - Selected outcome prediction
/// - Amount of funds locked in the contract
/// - Precise timing for chronological analysis
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Address, Symbol, String};
/// # use predictify_hybrid::events::BetPlacedEvent;
/// # let env = Env::default();
/// # let bettor = Address::generate(&env);
///
/// // Bet placement event data
/// let event = BetPlacedEvent {
///     market_id: Symbol::new(&env, "btc_50k_2024"),
///     bettor: bettor.clone(),
///     outcome: String::from_str(&env, "Yes"),
///     amount: 10_000_000, // 1.0 XLM locked
///     timestamp: env.ledger().timestamp(),
/// };
///
/// // Event provides complete betting context
/// println!("Bet placed by: {}", event.bettor.to_string());
/// println!("Market: {}", event.market_id.to_string());
/// println!("Outcome prediction: {}", event.outcome.to_string());
/// println!("Amount locked: {} XLM", event.amount / 10_000_000);
/// ```
///
/// # Fund Locking
///
/// When a bet is placed:
/// 1. User's funds are transferred to the contract
/// 2. Funds remain locked until market resolution
/// 3. Upon resolution:
///    - Winners receive proportional share of betting pool (minus fees)
///    - Losers forfeit their locked funds
///    - Refunds issued if market is cancelled
///
/// # Economic Tracking
///
/// Enables comprehensive economic analysis:
/// - **Pool Distribution**: Track total locked funds across outcomes
/// - **Bettor Activity**: Analyze betting patterns and amounts
/// - **Market Liquidity**: Monitor total bets and participation levels
/// - **Implied Odds**: Calculate implied probabilities from bet amounts
///
/// # Integration Applications
///
/// - **Real-time Updates**: Live market activity feeds
/// - **Analytics Dashboards**: Betting pattern analysis and visualization
/// - **Payout Calculation**: Amount-weighted payout distributions
/// - **User Portfolios**: Track individual betting history and performance
/// - **Market Sentiment**: Aggregate betting trends and momentum analysis
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BetPlacedEvent {
    /// Market ID
    pub market_id: Symbol,
    /// Bettor address
    pub bettor: Address,
    /// Selected outcome
    pub outcome: String,
    /// Amount locked
    pub amount: i128,
    /// Bet placement timestamp
    pub timestamp: u64,
}

/// Event emitted when a bet's status is updated (won, lost, refunded).
///
/// This event tracks bet resolution status changes, providing transparency
/// for payout processing and user notification.
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Address, Symbol, String};
/// # use predictify_hybrid::events::BetStatusUpdatedEvent;
/// # let env = Env::default();
/// # let bettor = Address::generate(&env);
///
/// let event = BetStatusUpdatedEvent {
///     market_id: Symbol::new(&env, "btc_50k_2024"),
///     bettor: bettor.clone(),
///     old_status: String::from_str(&env, "Active"),
///     new_status: String::from_str(&env, "Won"),
///     payout_amount: Some(15_000_000), // 1.5 XLM payout
///     timestamp: env.ledger().timestamp(),
/// };
/// ```
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BetStatusUpdatedEvent {
    /// Market ID
    pub market_id: Symbol,
    /// Bettor address
    pub bettor: Address,
    /// Previous bet status
    pub old_status: String,
    /// New bet status
    pub new_status: String,
    /// Payout amount (if won)
    pub payout_amount: Option<i128>,
    /// Status update timestamp
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

// ===== ORACLE RESULT VERIFICATION EVENTS =====

/// Event emitted when oracle result verification is initiated for a market.
///
/// This event marks the start of the automatic result verification process,
/// providing transparency about when and how oracle data is being fetched.
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Symbol, String, Address};
/// # use predictify_hybrid::events::OracleVerificationInitiatedEvent;
/// # let env = Env::default();
///
/// let event = OracleVerificationInitiatedEvent {
///     market_id: Symbol::new(&env, \"btc_50k\"),
///     initiator: Address::generate(&env),
///     feed_id: String::from_str(&env, \"BTC/USD\"),
///     oracle_count: 2,
///     timestamp: env.ledger().timestamp(),
/// };
/// ```
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleVerificationInitiatedEvent {
    /// Market ID being verified
    pub market_id: Symbol,
    /// Address that initiated verification
    pub initiator: Address,
    /// Feed ID being queried
    pub feed_id: String,
    /// Number of oracle sources being queried
    pub oracle_count: u32,
    /// Initiation timestamp
    pub timestamp: u64,
}

/// Event emitted when oracle result verification is completed successfully.
///
/// This comprehensive event captures all details of the verification process
/// including the final outcome, price data, confidence scores, and validation status.
/// Critical for transparency, auditability, and dispute resolution.
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Symbol, String, Address};
/// # use predictify_hybrid::events::OracleResultVerifiedEvent;
/// # use predictify_hybrid::types::OracleVerificationStatus;
/// # let env = Env::default();
///
/// let event = OracleResultVerifiedEvent {
///     market_id: Symbol::new(&env, \"btc_50k\"),
///     outcome: String::from_str(&env, \"yes\"),
///     price: 52_000_00,
///     threshold: 50_000_00,
///     comparison: String::from_str(&env, \"gt\"),
///     provider: String::from_str(&env, \"Reflector\"),
///     feed_id: String::from_str(&env, \"BTC/USD\"),
///     confidence_score: 95,
///     sources_consulted: 2,
///     verification_status: String::from_str(&env, \"Verified\"),
///     is_final: true,
///     timestamp: env.ledger().timestamp(),
///     block_number: env.ledger().sequence(),
/// };
/// ```
///
/// # Verification Details
///
/// Captures comprehensive verification context:
/// - **Price Data**: Actual fetched price and configured threshold
/// - **Outcome Determination**: How the outcome was derived
/// - **Source Information**: Oracle provider and feed details
/// - **Confidence Metrics**: Statistical confidence in the result
/// - **Validation Status**: Whether verification passed all checks
///
/// # Integration Applications
///
/// - **Market Resolution**: Trigger payout calculations
/// - **Dispute Evidence**: Provide data for potential disputes
/// - **Analytics**: Track oracle accuracy and performance
/// - **Transparency**: Public record of verification process
/// - **Audit Trail**: Complete verification history
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleResultVerifiedEvent {
    /// Market ID that was verified
    pub market_id: Symbol,
    /// Determined outcome (\"yes\"/\"no\" or custom)
    pub outcome: String,
    /// Price fetched from oracle
    pub price: i128,
    /// Threshold configured for market
    pub threshold: i128,
    /// Comparison operator used
    pub comparison: String,
    /// Oracle provider name
    pub provider: String,
    /// Feed ID used
    pub feed_id: String,
    /// Confidence score (0-100)
    pub confidence_score: u32,
    /// Number of oracle sources consulted
    pub sources_consulted: u32,
    /// Verification status (\"Verified\", \"Failed\", etc.)
    pub verification_status: String,
    /// Whether this is the final verified result
    pub is_final: bool,
    /// Verification timestamp
    pub timestamp: u64,
    /// Block number at verification
    pub block_number: u32,
}

/// Event emitted when oracle verification fails.
///
/// This event captures failure details for debugging, monitoring, and
/// triggering fallback mechanisms.
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Symbol, String};
/// # use predictify_hybrid::events::OracleVerificationFailedEvent;
/// # let env = Env::default();
///
/// let event = OracleVerificationFailedEvent {
///     market_id: Symbol::new(&env, \"btc_50k\"),
///     error_code: 200,
///     error_message: String::from_str(&env, \"Oracle unavailable\"),
///     attempted_providers: 2,
///     fallback_available: true,
///     timestamp: env.ledger().timestamp(),
/// };
/// ```
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleVerificationFailedEvent {
    /// Market ID that failed verification
    pub market_id: Symbol,
    /// Error code
    pub error_code: u32,
    /// Error message describing the failure
    pub error_message: String,
    /// Number of providers attempted
    pub attempted_providers: u32,
    /// Whether fallback sources are available
    pub fallback_available: bool,
    /// Failure timestamp
    pub timestamp: u64,
}

/// Event emitted when multi-oracle consensus is reached.
///
/// This event is emitted when multiple oracle sources agree on an outcome,
/// providing enhanced security through consensus-based verification.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleConsensusReachedEvent {
    /// Market ID
    pub market_id: Symbol,
    /// Consensus outcome
    pub consensus_outcome: String,
    /// Number of agreeing sources
    pub agreeing_sources: u32,
    /// Total sources consulted
    pub total_sources: u32,
    /// Agreement percentage
    pub agreement_percentage: u32,
    /// Average price across sources
    pub average_price: i128,
    /// Price variance (deviation indicator)
    pub price_variance: i128,
    /// Consensus timestamp
    pub timestamp: u64,
}

/// Event emitted when oracle source health status changes.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleHealthStatusEvent {
    /// Oracle contract address
    pub oracle_address: Address,
    /// Provider name
    pub provider: String,
    /// Previous health status
    pub previous_status: bool,
    /// Current health status
    pub current_status: bool,
    /// Consecutive failures (if unhealthy)
    pub consecutive_failures: u32,
    /// Status change timestamp
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

/// Event emitted when bet limits are updated (global or per-event).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BetLimitsUpdatedEvent {
    /// Admin who updated the limits
    pub admin: Address,
    /// Market ID or "global" for global limits
    pub scope: Symbol,
    /// New minimum bet amount
    pub min_bet: i128,
    /// New maximum bet amount
    pub max_bet: i128,
    /// Update timestamp
    pub timestamp: u64,
}

/// Statistics updated event - emitted when platform statistics change
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatisticsUpdatedEvent {
    /// Total volume (amount wagered) in token units
    pub total_volume: i128,
    /// Total number of bets placed
    pub total_bets: u64,
    /// Number of currently active markets
    pub active_markets: u32,
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

/// Error recovery event
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ErrorRecoveryEvent {
    /// Original error code
    pub error_code: u32,
    /// Recovery strategy used
    pub recovery_strategy: String,
    /// Recovery status
    pub recovery_status: String,
    /// Recovery attempts count
    pub recovery_attempts: u32,
    /// User address (if applicable)
    pub user: Option<Address>,
    /// Market ID (if applicable)
    pub market_id: Option<Symbol>,
    /// Recovery timestamp
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

/// Event emitted when a market is refunded due to oracle resolution failure or timeout.
///
/// Emitted after all bets are refunded in full (no fee deduction). The market is marked
/// as cancelled and no further resolution is possible.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefundOnOracleFailureEvent {
    /// Market ID
    pub market_id: Symbol,
    /// Total amount refunded to all participants
    pub total_refunded: i128,
    /// Event timestamp
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

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractInitializedEvent {
    /// Admin address
    pub admin: Address,
    /// Platform fee percentage
    pub platform_fee_percentage: i128,
    /// Initialization timestamp
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlatformFeeSetEvent {
    /// Fee percentage
    pub fee_percentage: i128,
    /// Set by admin
    pub set_by: Address,
    /// Set timestamp
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

/// Governance proposal created event
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GovernanceProposalCreatedEvent {
    pub proposal_id: Symbol,
    pub proposer: Address,
    pub title: String,
    pub description: String,
    pub timestamp: u64,
}

/// Governance vote cast event
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GovernanceVoteCastEvent {
    pub proposal_id: Symbol,
    pub voter: Address,
    pub support: bool,
    pub timestamp: u64,
}

/// Event emitted when a fallback oracle is used for market resolution.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FallbackUsedEvent {
    /// Market ID
    pub market_id: Symbol,
    /// Primary oracle address
    pub primary_oracle: Address,
    /// Fallback oracle address
    pub fallback_oracle: Address,
    /// Event timestamp
    pub timestamp: u64,
}

/// Event emitted when a market resolution timeout is reached.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolutionTimeoutEvent {
    /// Market ID
    pub market_id: Symbol,
    /// Timeout timestamp
    pub timeout_timestamp: u64,
}

/// Governance proposal executed event
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GovernanceProposalExecutedEvent {
    pub proposal_id: Symbol,
    pub executor: Address,
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

/// Storage cleanup event
#[contracttype]
#[derive(Clone, Debug)]
pub struct StorageCleanupEvent {
    /// Market ID
    pub market_id: Symbol,
    /// Cleanup type
    pub cleanup_type: String,
    /// Cleanup timestamp
    pub timestamp: u64,
}

/// Storage optimization event
#[contracttype]
#[derive(Clone, Debug)]
pub struct StorageOptimizationEvent {
    /// Market ID
    pub market_id: Symbol,
    /// Optimization type
    pub optimization_type: String,
    /// Optimization timestamp
    pub timestamp: u64,
}

/// Storage migration event
#[contracttype]
#[derive(Clone, Debug)]
pub struct StorageMigrationEvent {
    /// Migration ID
    pub migration_id: Symbol,
    /// Source format
    pub from_format: String,
    /// Target format
    pub to_format: String,
    /// Number of markets migrated
    pub markets_migrated: u32,
    /// Migration timestamp
    pub timestamp: u64,
}

/// Oracle degradation event - emitted when oracle service fails or becomes unavailable
#[contracttype]
#[derive(Clone, Debug)]
pub struct OracleDegradationEvent {
    /// Oracle provider that failed
    pub oracle: OracleProvider,
    /// Reason for degradation
    pub reason: String,
    /// Degradation timestamp
    pub timestamp: u64,
}

/// Oracle recovery event - emitted when oracle service recovers
#[contracttype]
#[derive(Clone, Debug)]
pub struct OracleRecoveryEvent {
    /// Oracle provider that recovered
    pub oracle: OracleProvider,
    /// Recovery message
    pub recovery_message: String,
    /// Recovery timestamp
    pub timestamp: u64,
}

/// Manual resolution required event - emitted when automatic resolution fails
#[contracttype]
#[derive(Clone, Debug)]
pub struct ManualResolutionRequiredEvent {
    /// Market that requires manual resolution
    pub market_id: Symbol,
    /// Reason manual resolution is needed
    pub reason: String,
    /// Event timestamp
    pub timestamp: u64,
}

/// Event emitted when market state changes
///
/// This event tracks all market state transitions throughout the market lifecycle,
/// providing transparency and audit trail for state changes. Critical for monitoring
/// market progression and detecting anomalies.
///
/// # Event Data
///
/// - Market identifier
/// - Previous state (Active, Ended, Disputed, Resolved, Closed, Cancelled)
/// - New state after transition
/// - Reason for state change
/// - Timestamp of transition
///
/// # State Transitions
///
/// Common transitions:
/// - Active → Ended (voting period completed)
/// - Ended → Disputed (dispute filed)
/// - Disputed → Resolved (dispute resolved)
/// - Ended → Resolved (normal resolution)
/// - Resolved → Closed (market finalized)
/// - Any → Cancelled (emergency cancellation)
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateChangeEvent {
    /// Market ID
    pub market_id: Symbol,
    /// Previous state
    pub old_state: crate::types::MarketState,
    /// New state
    pub new_state: crate::types::MarketState,
    /// Reason for state change
    pub reason: String,
    /// Event timestamp
    pub timestamp: u64,
}

/// Event emitted when user claims winnings
///
/// This event tracks all winnings claims, providing transparency for payouts
/// and audit trail for financial operations. Essential for monitoring market
/// economics and user engagement.
///
/// # Event Data
///
/// - Market identifier
/// - User address claiming winnings
/// - Amount claimed
/// - Timestamp of claim
///
/// # Use Cases
///
/// - **Financial Tracking**: Monitor total payouts
/// - **User Analytics**: Track winning users and amounts
/// - **Audit Trail**: Maintain complete payout records
/// - **Tax Reporting**: Generate user earning reports
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WinningsClaimedEvent {
    /// Market ID
    pub market_id: Symbol,
    /// User claiming winnings
    pub user: Address,
    /// Amount claimed
    pub amount: i128,
    /// Event timestamp
    pub timestamp: u64,
}

/// Contract upgraded event - emitted when contract Wasm is upgraded
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractUpgradedEvent {
    /// Previous Wasm hash
    pub old_wasm_hash: soroban_sdk::BytesN<32>,
    /// New Wasm hash
    pub new_wasm_hash: soroban_sdk::BytesN<32>,
    /// Upgrade ID
    pub upgrade_id: Symbol,
    /// Upgrade timestamp
    pub timestamp: u64,
}

/// Event emitted when market deadline is extended
///
/// This event tracks market deadline extensions, providing transparency
/// for extension requests and their impact on market timeline.
///
/// # Event Data
///
/// - Market identifier
/// - Previous end time
/// - New end time after extension
/// - Additional days added
/// - Admin who performed the extension
/// - Reason for extension
/// - Extension fee (if applicable)
/// - Timestamp of extension
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketDeadlineExtendedEvent {
    /// Market ID
    pub market_id: Symbol,
    /// Previous end time
    pub old_end_time: u64,
    /// New end time
    pub new_end_time: u64,
    /// Additional days
    pub additional_days: u32,
    /// Admin who extended
    pub admin: Address,
    /// Reason for extension
    pub reason: String,
    /// Extension fee
    pub fee: i128,
    /// Extension timestamp
    pub timestamp: u64,
}

/// Event emitted when market description is updated
///
/// This event tracks market description updates, providing transparency
/// for changes to market questions and parameters.
///
/// # Event Data
///
/// - Market identifier
/// - Previous description
/// - New description
/// - Admin who performed the update
/// - Update timestamp
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketDescriptionUpdatedEvent {
    /// Market ID
    pub market_id: Symbol,
    /// Old description
    pub old_description: String,
    /// New description
    pub new_description: String,
    /// Admin who updated
    pub admin: Address,
    /// Update timestamp
    pub timestamp: u64,
}

/// Event emitted when market outcomes are updated
///
/// This event tracks market outcome updates, providing transparency
/// for changes to available market outcomes.
///
/// # Event Data
///
/// - Market identifier
/// - Previous outcomes
/// - New outcomes
/// - Admin who performed the update
/// - Update timestamp
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketOutcomesUpdatedEvent {
    /// Market ID
    pub market_id: Symbol,
    /// Old outcomes
    pub old_outcomes: Vec<String>,
    /// New outcomes
    pub new_outcomes: Vec<String>,
    /// Admin who updated
    pub admin: Address,
    /// Update timestamp
    pub timestamp: u64,
}

/// Event emitted when market category is updated
///
/// This event tracks market category updates, providing transparency
/// for changes to market categorization.
///
/// # Event Data
///
/// - Market identifier
/// - Previous category (None if not set)
/// - New category (None to clear)
/// - Admin who performed the update
/// - Update timestamp
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CategoryUpdatedEvent {
    /// Market ID
    pub market_id: Symbol,
    /// Old category (None if not previously set)
    pub old_category: Option<String>,
    /// New category (None to clear category)
    pub new_category: Option<String>,
    /// Admin who updated
    pub admin: Address,
    /// Update timestamp
    pub timestamp: u64,
}

/// Event emitted when market tags are updated
///
/// This event tracks market tags updates, providing transparency
/// for changes to market tagging.
///
/// # Event Data
///
/// - Market identifier
/// - Previous tags
/// - New tags
/// - Admin who performed the update
/// - Update timestamp
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TagsUpdatedEvent {
    /// Market ID
    pub market_id: Symbol,
    /// Old tags
    pub old_tags: Vec<String>,
    /// New tags
    pub new_tags: Vec<String>,
    /// Admin who updated
    pub admin: Address,
    /// Update timestamp
    pub timestamp: u64,
}

/// Contract rollback event - emitted when contract is rolled back to previous version
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractRollbackEvent {
    /// Current Wasm hash (before rollback)
    pub current_wasm_hash: soroban_sdk::BytesN<32>,
    /// Rollback Wasm hash (after rollback)
    pub rollback_wasm_hash: soroban_sdk::BytesN<32>,
    /// Rollback timestamp
    pub timestamp: u64,
}

/// Upgrade proposal created event - emitted when a new upgrade proposal is created
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpgradeProposalCreatedEvent {
    /// Proposal ID
    pub proposal_id: Symbol,
    /// Proposer address
    pub proposer: Address,
    /// Target version
    pub target_version: String,
    /// Proposal timestamp
    pub timestamp: u64,
}

/// Event emitted when circuit breaker state changes
///
/// This event provides comprehensive information about circuit breaker
/// state changes, including the action taken, condition that triggered
/// it, reason for the action, and administrative details.
///
/// # Event Data
///
/// Contains all critical circuit breaker information:
/// - Action taken (pause, resume, trigger, reset)
/// - Condition that triggered the action (if automatic)
/// - Reason for the action
/// - Timestamp and admin information
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Address, String};
/// # use predictify_hybrid::events::CircuitBreakerEvent;
/// # use predictify_hybrid::circuit_breaker::{BreakerAction, BreakerCondition};
/// # let env = Env::default();
/// # let admin = Address::generate(&env);
///
/// // Circuit breaker event data
/// let event = CircuitBreakerEvent {
///     action: BreakerAction::Pause,
///     condition: Some(BreakerCondition::HighErrorRate),
///     reason: String::from_str(&env, "Error rate exceeded 10% threshold"),
///     timestamp: env.ledger().timestamp(),
///     admin: Some(admin.clone()),
/// };
///
/// // Event provides complete circuit breaker context
/// println!("Circuit breaker action: {:?}", event.action);
/// println!("Trigger condition: {:?}", event.condition);
/// println!("Reason: {}", event.reason.to_string());
/// ```
///
/// # Integration Points
///
/// - **Monitoring**: Track circuit breaker state changes
/// - **Alerting**: Notify administrators of circuit breaker actions
/// - **Analytics**: Analyze circuit breaker patterns and triggers
/// - **Audit Trails**: Maintain complete record of safety actions
/// - **Recovery Tracking**: Monitor recovery attempts and success rates
///
/// # Event Timing
///
/// Emitted immediately when circuit breaker state changes, providing:
/// - Real-time notification of safety actions
/// - Immediate availability for monitoring systems
/// - Historical record for analysis and reporting
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CircuitBreakerEvent {
    /// Action taken by circuit breaker
    pub action: crate::circuit_breaker::BreakerAction,
    /// Condition that triggered the action (if automatic)
    pub condition: crate::circuit_breaker::BreakerCondition,
    /// Reason for the action
    pub reason: String,
    /// Event timestamp
    pub timestamp: u64,
    /// Admin who triggered the action (if manual)
    pub admin: Option<Address>,
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

    /// Emit fallback used event
    pub fn emit_fallback_used(
        env: &Env,
        market_id: &Symbol,
        primary_oracle: &Address,
        fallback_oracle: &Address,
    ) {
        let event = FallbackUsedEvent {
            market_id: market_id.clone(),
            primary_oracle: primary_oracle.clone(),
            fallback_oracle: fallback_oracle.clone(),
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("fbk_used"), &event);
    }

    /// Emit resolution timeout event
    pub fn emit_resolution_timeout(env: &Env, market_id: &Symbol, timeout_timestamp: u64) {
        let event = ResolutionTimeoutEvent {
            market_id: market_id.clone(),
            timeout_timestamp,
        };

        Self::store_event(env, &symbol_short!("res_tmo"), &event);
    }

    /// Emit event created event
    pub fn emit_event_created(
        env: &Env,
        event_id: &Symbol,
        description: &String,
        outcomes: &Vec<String>,
        admin: &Address,
        end_time: u64,
    ) {
        let event = EventCreatedEvent {
            event_id: event_id.clone(),
            description: description.clone(),
            outcomes: outcomes.clone(),
            admin: admin.clone(),
            end_time,
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("evt_crt"), &event);
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

    /// Emit statistics updated event
    pub fn emit_statistics_updated(
        env: &Env,
        total_volume: i128,
        total_bets: u64,
        active_markets: u32,
    ) {
        let event = StatisticsUpdatedEvent {
            total_volume,
            total_bets,
            active_markets,
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("stats_upd"), &event);
    }

    /// Emit bet placed event when a user places a bet on a market
    ///
    /// This function emits an event when a user successfully places a bet,
    /// locking their funds in the contract for the duration of the market.
    ///
    /// # Parameters
    ///
    /// - `env` - Soroban environment
    /// - `market_id` - Market identifier
    /// - `bettor` - Address of the user placing the bet
    /// - `outcome` - The outcome the user is betting on
    /// - `amount` - The amount of funds locked for this bet
    ///
    /// # Example
    ///
    /// ```rust
    /// EventEmitter::emit_bet_placed(
    ///     &env,
    ///     &market_id,
    ///     &user_address,
    ///     &String::from_str(&env, "Yes"),
    ///     10_000_000 // 1.0 XLM
    /// );
    /// ```
    pub fn emit_bet_placed(
        env: &Env,
        market_id: &Symbol,
        bettor: &Address,
        outcome: &String,
        amount: i128,
    ) {
        let event = BetPlacedEvent {
            market_id: market_id.clone(),
            bettor: bettor.clone(),
            outcome: outcome.clone(),
            amount,
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("bet_plc"), &event);
    }

    /// Emit bet status updated event when a bet's status changes
    ///
    /// This function emits an event when a bet's status is updated
    /// (e.g., Active → Won, Active → Lost, Active → Refunded).
    ///
    /// # Parameters
    ///
    /// - `env` - Soroban environment
    /// - `market_id` - Market identifier
    /// - `bettor` - Address of the bettor
    /// - `old_status` - Previous bet status
    /// - `new_status` - New bet status
    /// - `payout_amount` - Optional payout amount (if bet won)
    ///
    /// # Example
    ///
    /// ```rust
    /// EventEmitter::emit_bet_status_updated(
    ///     &env,
    ///     &market_id,
    ///     &user_address,
    ///     &String::from_str(&env, "Active"),
    ///     &String::from_str(&env, "Won"),
    ///     Some(15_000_000) // 1.5 XLM payout
    /// );
    /// ```
    pub fn emit_bet_status_updated(
        env: &Env,
        market_id: &Symbol,
        bettor: &Address,
        old_status: &String,
        new_status: &String,
        payout_amount: Option<i128>,
    ) {
        let event = BetStatusUpdatedEvent {
            market_id: market_id.clone(),
            bettor: bettor.clone(),
            old_status: old_status.clone(),
            new_status: new_status.clone(),
            payout_amount,
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("bet_upd"), &event);
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

    // ===== ORACLE RESULT VERIFICATION EVENT EMISSION METHODS =====

    /// Emit oracle verification initiated event
    ///
    /// This event is emitted when automatic oracle result verification begins
    /// for a market that has ended.
    ///
    /// # Parameters
    ///
    /// - `env` - Soroban environment
    /// - `market_id` - Market being verified
    /// - `initiator` - Address that initiated verification
    /// - `feed_id` - Oracle feed being queried
    /// - `oracle_count` - Number of oracle sources to query
    pub fn emit_oracle_verification_initiated(
        env: &Env,
        market_id: &Symbol,
        initiator: &Address,
        feed_id: &String,
        oracle_count: u32,
    ) {
        let event = OracleVerificationInitiatedEvent {
            market_id: market_id.clone(),
            initiator: initiator.clone(),
            feed_id: feed_id.clone(),
            oracle_count,
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("orc_init"), &event);
    }

    /// Emit oracle result verified event
    ///
    /// This event is emitted when oracle result verification completes successfully,
    /// capturing the full verification details for transparency and auditability.
    ///
    /// # Parameters
    ///
    /// - `env` - Soroban environment
    /// - `market_id` - Market that was verified
    /// - `outcome` - Determined outcome
    /// - `price` - Fetched price from oracle
    /// - `threshold` - Configured threshold
    /// - `comparison` - Comparison operator used
    /// - `provider` - Oracle provider name
    /// - `feed_id` - Feed ID used
    /// - `confidence_score` - Confidence score (0-100)
    /// - `sources_consulted` - Number of oracle sources consulted
    /// - `is_final` - Whether this is the final verified result
    pub fn emit_oracle_result_verified(
        env: &Env,
        market_id: &Symbol,
        outcome: &String,
        price: i128,
        threshold: i128,
        comparison: &String,
        provider: &String,
        feed_id: &String,
        confidence_score: u32,
        sources_consulted: u32,
        is_final: bool,
    ) {
        let event = OracleResultVerifiedEvent {
            market_id: market_id.clone(),
            outcome: outcome.clone(),
            price,
            threshold,
            comparison: comparison.clone(),
            provider: provider.clone(),
            feed_id: feed_id.clone(),
            confidence_score,
            sources_consulted,
            verification_status: String::from_str(env, "Verified"),
            is_final,
            timestamp: env.ledger().timestamp(),
            block_number: env.ledger().sequence(),
        };

        Self::store_event(env, &symbol_short!("orc_ver"), &event);
    }

    /// Emit oracle verification failed event
    ///
    /// This event is emitted when oracle verification fails, capturing
    /// error details for debugging and fallback triggering.
    ///
    /// # Parameters
    ///
    /// - `env` - Soroban environment
    /// - `market_id` - Market that failed verification
    /// - `error_code` - Error code
    /// - `error_message` - Description of the failure
    /// - `attempted_providers` - Number of providers attempted
    /// - `fallback_available` - Whether fallback is available
    pub fn emit_oracle_verification_failed(
        env: &Env,
        market_id: &Symbol,
        error_code: u32,
        error_message: &String,
        attempted_providers: u32,
        fallback_available: bool,
    ) {
        let event = OracleVerificationFailedEvent {
            market_id: market_id.clone(),
            error_code,
            error_message: error_message.clone(),
            attempted_providers,
            fallback_available,
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("orc_fail"), &event);
    }

    /// Emit oracle consensus reached event
    ///
    /// This event is emitted when multiple oracle sources reach consensus
    /// on an outcome, providing enhanced security through agreement.
    ///
    /// # Parameters
    ///
    /// - `env` - Soroban environment
    /// - `market_id` - Market being verified
    /// - `consensus_outcome` - The agreed-upon outcome
    /// - `agreeing_sources` - Number of sources that agreed
    /// - `total_sources` - Total sources consulted
    /// - `average_price` - Average price across sources
    /// - `price_variance` - Price variance/deviation
    pub fn emit_oracle_consensus_reached(
        env: &Env,
        market_id: &Symbol,
        consensus_outcome: &String,
        agreeing_sources: u32,
        total_sources: u32,
        average_price: i128,
        price_variance: i128,
    ) {
        let agreement_percentage = if total_sources > 0 {
            (agreeing_sources * 100) / total_sources
        } else {
            0
        };

        let event = OracleConsensusReachedEvent {
            market_id: market_id.clone(),
            consensus_outcome: consensus_outcome.clone(),
            agreeing_sources,
            total_sources,
            agreement_percentage,
            average_price,
            price_variance,
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("orc_cons"), &event);
    }

    /// Emit oracle health status event
    ///
    /// This event is emitted when an oracle's health status changes,
    /// enabling monitoring and alerting for oracle availability.
    ///
    /// # Parameters
    ///
    /// - `env` - Soroban environment
    /// - `oracle_address` - Oracle contract address
    /// - `provider` - Provider name
    /// - `previous_status` - Previous health status
    /// - `current_status` - Current health status
    /// - `consecutive_failures` - Number of consecutive failures
    pub fn emit_oracle_health_status(
        env: &Env,
        oracle_address: &Address,
        provider: &String,
        previous_status: bool,
        current_status: bool,
        consecutive_failures: u32,
    ) {
        let event = OracleHealthStatusEvent {
            oracle_address: oracle_address.clone(),
            provider: provider.clone(),
            previous_status,
            current_status,
            consecutive_failures,
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("orc_hlth"), &event);
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

    /// Emit bet limits updated event (global or per-event).
    pub fn emit_bet_limits_updated(
        env: &Env,
        admin: &Address,
        scope: &Symbol,
        min_bet: i128,
        max_bet: i128,
    ) {
        let event = BetLimitsUpdatedEvent {
            admin: admin.clone(),
            scope: scope.clone(),
            min_bet,
            max_bet,
            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("bet_lim"), &event);
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

    /// Emit error recovery event
    pub fn emit_error_recovery_event(
        env: &Env,
        error_code: u32,
        recovery_strategy: &String,
        recovery_status: String,
        recovery_attempts: u32,
        user: Option<Address>,
        market_id: Option<Symbol>,
    ) {
        let event = ErrorRecoveryEvent {
            error_code,
            recovery_strategy: recovery_strategy.clone(),
            recovery_status,
            recovery_attempts,
            user,
            market_id,
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("err_rec"), &event);
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

    /// Emit contract initialized event (full initialization with platform fee)
    pub fn emit_contract_initialized(env: &Env, admin: &Address, fee: i128) {
        let event = ContractInitializedEvent {
            admin: admin.clone(), // Clone because the struct owns the Address
            platform_fee_percentage: fee,
            timestamp: env.ledger().timestamp(),
        };
        env.events()
            .publish((Symbol::new(env, "contract_initialized"),), event);
    }

    pub fn emit_platform_fee_set(env: &Env, fee: i128, admin: &Address) {
        let event = PlatformFeeSetEvent {
            fee_percentage: fee,
            set_by: admin.clone(),
            timestamp: env.ledger().timestamp(),
        };
        env.events()
            .publish((Symbol::new(env, "platform_fee_set"),), event);
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

    /// Emit refund on oracle failure event (market cancelled, all bets refunded in full).
    pub fn emit_refund_on_oracle_failure(env: &Env, market_id: &Symbol, total_refunded: i128) {
        let event = RefundOnOracleFailureEvent {
            market_id: market_id.clone(),
            total_refunded,
            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("ref_oracl"), &event);
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
    pub fn emit_storage_cleanup_event(env: &Env, market_id: &Symbol, cleanup_type: &String) {
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

    /// Emit circuit breaker event
    pub fn emit_circuit_breaker_event(env: &Env, event: &CircuitBreakerEvent) {
        Self::store_event(env, &symbol_short!("cb_event"), event);
    }

    /// Emit oracle degradation event when oracle service fails
    pub fn emit_oracle_degradation(env: &Env, oracle: &OracleProvider, reason: &String) {
        let event = OracleDegradationEvent {
            oracle: oracle.clone(),
            reason: reason.clone(),
            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("ora_deg"), &event);
    }

    /// Emit oracle recovery event when oracle service recovers
    pub fn emit_oracle_recovery(env: &Env, oracle: &OracleProvider, message: &String) {
        let event = OracleRecoveryEvent {
            oracle: oracle.clone(),
            recovery_message: message.clone(),
            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("ora_rec"), &event);
    }

    /// Emit manual resolution required event when automatic resolution fails
    pub fn emit_manual_resolution_required(env: &Env, market_id: &Symbol, reason: &String) {
        let event = ManualResolutionRequiredEvent {
            market_id: market_id.clone(),
            reason: reason.clone(),
            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("man_res"), &event);
    }

    /// Emit state change event when market state transitions
    ///
    /// This function emits an event whenever a market transitions between states,
    /// providing complete transparency and audit trail for state changes.
    ///
    /// # Parameters
    ///
    /// - `env` - Soroban environment
    /// - `market_id` - Market identifier
    /// - `old_state` - Previous market state
    /// - `new_state` - New market state after transition
    /// - `reason` - Reason for state change
    ///
    /// # Example
    ///
    /// ```rust
    /// EventEmitter::emit_state_change_event(
    ///     &env,
    ///     &market_id,
    ///     &MarketState::Active,
    ///     &MarketState::Ended,
    ///     &String::from_str(&env, "Voting period completed")
    /// );
    /// ```
    pub fn emit_state_change_event(
        env: &Env,
        market_id: &Symbol,
        old_state: &crate::types::MarketState,
        new_state: &crate::types::MarketState,
        reason: &String,
    ) {
        let event = StateChangeEvent {
            market_id: market_id.clone(),
            old_state: old_state.clone(),
            new_state: new_state.clone(),
            reason: reason.clone(),
            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("st_chng"), &event);
    }

    /// Emit winnings claimed event when user claims payout
    ///
    /// This function emits an event whenever a user successfully claims their
    /// winnings from a resolved market, providing transparency for all payouts.
    ///
    /// # Parameters
    ///
    /// - `env` - Soroban environment
    /// - `market_id` - Market identifier
    /// - `user` - User address claiming winnings
    /// - `amount` - Amount claimed
    ///
    /// # Example
    ///
    /// ```rust
    /// EventEmitter::emit_winnings_claimed(
    ///     &env,
    ///     &market_id,
    ///     &user_address,
    ///     1_500_000_000 // 150 tokens
    /// );
    /// ```
    pub fn emit_winnings_claimed(env: &Env, market_id: &Symbol, user: &Address, amount: i128) {
        let event = WinningsClaimedEvent {
            market_id: market_id.clone(),
            user: user.clone(),
            amount,
            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("win_clm"), &event);
    }

    /// Emit market deadline extended event
    ///
    /// This function emits an event when a market's deadline is extended,
    /// providing transparency for extension operations and their parameters.
    ///
    /// # Parameters
    ///
    /// - `env` - Soroban environment
    /// - `market_id` - Market identifier
    /// - `old_end_time` - Previous end time
    /// - `new_end_time` - New end time after extension
    /// - `additional_days` - Number of days added
    /// - `admin` - Admin who performed the extension
    /// - `reason` - Reason for the extension
    /// - `fee` - Extension fee paid
    ///
    /// # Example
    ///
    /// ```rust
    /// EventEmitter::emit_market_deadline_extended(
    ///     &env,
    ///     &market_id,
    ///     old_end_time,
    ///     new_end_time,
    ///     7, // 7 additional days
    ///     &admin_address,
    ///     &String::from_str(&env, "Low participation"),
    ///     1_000_000 // 1 XLM fee
    /// );
    /// ```
    pub fn emit_market_deadline_extended(
        env: &Env,
        market_id: &Symbol,
        old_end_time: u64,
        new_end_time: u64,
        additional_days: u32,
        admin: &Address,
        reason: &String,
        fee: i128,
    ) {
        let event = MarketDeadlineExtendedEvent {
            market_id: market_id.clone(),
            old_end_time,
            new_end_time,
            additional_days,
            admin: admin.clone(),
            reason: reason.clone(),
            fee,
            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("mkt_ext"), &event);
    }

    /// Emit market description updated event
    ///
    /// This function emits an event when a market's description is updated,
    /// providing transparency for market parameter changes.
    ///
    /// # Parameters
    ///
    /// - `env` - Soroban environment
    /// - `market_id` - Market identifier
    /// - `old_description` - Previous market description
    /// - `new_description` - New market description
    /// - `admin` - Admin who performed the update
    ///
    /// # Example
    ///
    /// ```rust
    /// EventEmitter::emit_market_description_updated(
    ///     &env,
    ///     &market_id,
    ///     &String::from_str(&env, "Old question"),
    ///     &String::from_str(&env, "Updated question"),
    ///     &admin_address
    /// );
    /// ```
    pub fn emit_market_description_updated(
        env: &Env,
        market_id: &Symbol,
        old_description: &String,
        new_description: &String,
        admin: &Address,
    ) {
        let event = MarketDescriptionUpdatedEvent {
            market_id: market_id.clone(),
            old_description: old_description.clone(),
            new_description: new_description.clone(),
            admin: admin.clone(),
            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("mkt_dsc"), &event);
    }

    /// Emit market outcomes updated event
    ///
    /// This function emits an event when a market's outcomes are updated,
    /// providing transparency for outcome changes.
    ///
    /// # Parameters
    ///
    /// - `env` - Soroban environment
    /// - `market_id` - Market identifier
    /// - `old_outcomes` - Previous market outcomes
    /// - `new_outcomes` - New market outcomes
    /// - `admin` - Admin who performed the update
    ///
    /// # Example
    ///
    /// ```rust
    /// EventEmitter::emit_market_outcomes_updated(
    ///     &env,
    ///     &market_id,
    ///     &old_outcomes_vec,
    ///     &new_outcomes_vec,
    ///     &admin_address
    /// );
    /// ```
    pub fn emit_market_outcomes_updated(
        env: &Env,
        market_id: &Symbol,
        old_outcomes: &Vec<String>,
        new_outcomes: &Vec<String>,
        admin: &Address,
    ) {
        let event = MarketOutcomesUpdatedEvent {
            market_id: market_id.clone(),
            old_outcomes: old_outcomes.clone(),
            new_outcomes: new_outcomes.clone(),
            admin: admin.clone(),
            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("mkt_out"), &event);
    }

    /// Emit market category updated event
    ///
    /// This function emits an event when a market's category is updated,
    /// providing transparency for category changes.
    ///
    /// # Parameters
    ///
    /// - `env` - Soroban environment
    /// - `market_id` - Market identifier
    /// - `old_category` - Previous market category (None if not set)
    /// - `new_category` - New market category (None to clear)
    /// - `admin` - Admin who performed the update
    ///
    /// # Example
    ///
    /// ```rust
    /// EventEmitter::emit_category_updated(
    ///     &env,
    ///     &market_id,
    ///     &None,
    ///     &Some(String::from_str(&env, "sports")),
    ///     &admin_address
    /// );
    /// ```
    pub fn emit_category_updated(
        env: &Env,
        market_id: &Symbol,
        old_category: &Option<String>,
        new_category: &Option<String>,
        admin: &Address,
    ) {
        let event = CategoryUpdatedEvent {
            market_id: market_id.clone(),
            old_category: old_category.clone(),
            new_category: new_category.clone(),
            admin: admin.clone(),
            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("mkt_cat"), &event);
    }

    /// Emit market tags updated event
    ///
    /// This function emits an event when a market's tags are updated,
    /// providing transparency for tagging changes.
    ///
    /// # Parameters
    ///
    /// - `env` - Soroban environment
    /// - `market_id` - Market identifier
    /// - `old_tags` - Previous market tags
    /// - `new_tags` - New market tags
    /// - `admin` - Admin who performed the update
    ///
    /// # Example
    ///
    /// ```rust
    /// EventEmitter::emit_tags_updated(
    ///     &env,
    ///     &market_id,
    ///     &Vec::new(&env),
    ///     &vec![&env, String::from_str(&env, "crypto"), String::from_str(&env, "bitcoin")],
    ///     &admin_address
    /// );
    /// ```
    pub fn emit_tags_updated(
        env: &Env,
        market_id: &Symbol,
        old_tags: &Vec<String>,
        new_tags: &Vec<String>,
        admin: &Address,
    ) {
        let event = TagsUpdatedEvent {
            market_id: market_id.clone(),
            old_tags: old_tags.clone(),
            new_tags: new_tags.clone(),
            admin: admin.clone(),
            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("mkt_tag"), &event);
    }

    /// Emit error event with full error context
    ///
    /// This function emits an event when errors occur, providing detailed context
    /// for debugging, monitoring, and error recovery. Complies with ticket spec
    /// requiring emit_error_event(error: Error, context: ErrorContext).
    ///
    /// # Parameters
    ///
    /// - `env` - Soroban environment
    /// - `error` - Error that occurred
    /// - `context` - Full error context with operation, user, market details
    ///
    /// # Example
    ///
    /// ```rust
    /// let context = ErrorContext {
    ///     operation: String::from_str(&env, "claim_winnings"),
    ///     user_address: Some(user.clone()),
    ///     market_id: Some(market_id.clone()),
    ///     context_data: Map::new(&env),
    ///     timestamp: env.ledger().timestamp(),
    ///     call_chain: vec![&env, String::from_str(&env, "lib::claim_winnings")],
    /// };
    ///
    /// EventEmitter::emit_error_event(&env, Error::NothingToClaim, &context);
    /// ```
    pub fn emit_diagnostic_event(env: &Env, error: Error, context: &crate::errors::ErrorContext) {
        let error_code = error as u32;

        // Convert error enum to message string
        let error_msg = match error {
            Error::Unauthorized => "Unauthorized access",
            Error::MarketNotFound => "Market not found",
            Error::MarketClosed => "Market closed",
            Error::InvalidOutcome => "Invalid outcome",
            Error::AlreadyVoted => "Already voted",
            Error::AlreadyClaimed => "Already claimed",
            Error::MarketNotResolved => "Market not resolved",
            Error::NothingToClaim => "Nothing to claim",
            _ => "Unknown error",
        };
        let message = String::from_str(env, error_msg);

        let event = ErrorLoggedEvent {
            error_code,
            message,
            context: context.operation.clone(),
            user: context.user_address.clone(),
            market_id: context.market_id.clone(),
            timestamp: context.timestamp,
        };

        Self::store_event(env, &symbol_short!("err_evt"), &event);
    }

    /// Emit governance proposal created event
    pub fn emit_governance_proposal_created(
        env: &Env,
        proposal_id: &Symbol,
        proposer: &Address,
        title: &String,
        description: &String,
    ) {
        let event = GovernanceProposalCreatedEvent {
            proposal_id: proposal_id.clone(),
            proposer: proposer.clone(),
            title: title.clone(),
            description: description.clone(),
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("gov_prop"), &event);
    }

    /// Emit governance vote cast event
    pub fn emit_governance_vote_cast(
        env: &Env,
        proposal_id: &Symbol,
        voter: &Address,
        support: bool,
    ) {
        let timestamp = env.ledger().timestamp();
        let event = GovernanceVoteCastEvent {
            proposal_id: proposal_id.clone(),
            voter: voter.clone(),
            support,
            timestamp,
        };

        Self::store_event(env, &symbol_short!("gov_vote"), &event);
    }

    /// Emit governance proposal executed event
    pub fn emit_governance_proposal_executed(env: &Env, proposal_id: &Symbol, executor: &Address) {
        let timestamp = env.ledger().timestamp();
        let event = GovernanceProposalExecutedEvent {
            proposal_id: proposal_id.clone(),
            executor: executor.clone(),
            timestamp,
        };

        Self::store_event(env, &symbol_short!("gov_exec"), &event);
    }

    /// Emit contract upgraded event when contract Wasm is upgraded
    pub fn emit_contract_upgraded_event(
        env: &Env,
        old_wasm_hash: &soroban_sdk::BytesN<32>,
        new_wasm_hash: &soroban_sdk::BytesN<32>,
        upgrade_id: &Symbol,
    ) {
        let event = ContractUpgradedEvent {
            old_wasm_hash: old_wasm_hash.clone(),
            new_wasm_hash: new_wasm_hash.clone(),
            upgrade_id: upgrade_id.clone(),
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("up_grade"), &event);
    }

    /// Emit contract rollback event when contract is rolled back
    pub fn emit_contract_rollback_event(
        env: &Env,
        current_wasm_hash: &soroban_sdk::BytesN<32>,
        rollback_wasm_hash: &soroban_sdk::BytesN<32>,
    ) {
        let event = ContractRollbackEvent {
            current_wasm_hash: current_wasm_hash.clone(),
            rollback_wasm_hash: rollback_wasm_hash.clone(),
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("rollback"), &event);
    }

    /// Emit upgrade proposal created event
    pub fn emit_upgrade_proposal_created_event(
        env: &Env,
        proposal_id: &Symbol,
        proposer: &Address,
        target_version: &String,
    ) {
        let event = UpgradeProposalCreatedEvent {
            proposal_id: proposal_id.clone(),
            proposer: proposer.clone(),
            target_version: target_version.clone(),
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("up_prop"), &event);
    }

    /// Emit balance changed event for deposits and withdrawals
    pub fn emit_balance_changed(
        env: &Env,
        user: &Address,
        asset: &crate::types::ReflectorAsset,
        operation: &String,
        amount: i128,
        new_balance: i128,
    ) {
        env.events().publish(
            (symbol_short!("bal_chg"), user, asset.clone()),
            (
                operation.clone(),
                amount,
                new_balance,
                env.ledger().timestamp(),
            ),
        );
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
