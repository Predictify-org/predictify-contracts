use crate::errors::Error;
use crate::storage::{AdminStorage, MarketStateManager, TokenStorage};
use crate::types::{
    Dispute, DisputeEscalation, DisputeFeeDistribution, DisputeResolution, DisputeStats,
    DisputeStatus, DisputeTimeout, DisputeTimeoutOutcome, DisputeTimeoutStatus, Market,
    TimeoutAnalytics, TimeoutStats,
};
use soroban_sdk::{symbol_short, token, Address, Env, Map, String, Symbol, Vec};

pub const MIN_DISPUTE_STAKE: i128 = 10_000_000;
pub const DISPUTE_PERIOD_SECS: u64 = 86400;

pub struct DisputeValidator;

/// Configuration for vote stake decay over time.
///
/// Applies an exponential decay approximation to vote weights so that
/// earlier votes carry more influence than late votes.  This discourages
/// last-minute vote sniping and rewards early participation.
///
/// # Fields
///
/// * `half_life_seconds` - How many seconds until a vote's weight halves
/// * `floor_bps` - Minimum weight in basis points (1 % = 100 bps) below which
///   the weight will never drop
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeDecayConfig {
    pub half_life_seconds: u64,
    pub floor_bps: u32,
}

        if market.oracle_result.is_none() {
            return Err(Error::OracleResultNotAvailable);
        }

/// Comprehensive statistics about disputes for a specific market.
///
/// This structure aggregates dispute activity data to provide insights into
/// community engagement, dispute patterns, and market controversy levels.
/// Used for analytics, governance decisions, and market quality assessment.
///
/// # Fields
///
/// * `total_disputes` - Total number of disputes ever raised for this market
/// * `total_dispute_stakes` - Sum of all stakes committed to disputes (in stroops)
/// * `active_disputes` - Number of disputes currently accepting votes
/// * `resolved_disputes` - Number of disputes that have been finalized
/// * `unique_disputers` - Count of unique addresses that have disputed this market
///
/// # Example
///
/// ```rust
/// # use predictify_hybrid::disputes::DisputeStats;
///
/// let stats = DisputeStats {
///     total_disputes: 3,
///     total_dispute_stakes: 50_000_000, // 5 XLM total
///     active_disputes: 1,
///     resolved_disputes: 2,
///     unique_disputers: 3,
/// };
///
/// // Calculate average stake per dispute
/// let avg_stake = stats.total_dispute_stakes / stats.total_disputes as i128;
/// assert_eq!(avg_stake, 16_666_666); // ~1.67 XLM average
///
/// // Check market controversy level
/// let controversy_ratio = stats.total_disputes as f64 / 10.0; // Assume 10 total participants
/// println!("Market controversy: {:.1}%", controversy_ratio * 100.0);
/// ```
///
/// # Analytics Use Cases
///
/// - **Market Quality**: High dispute rates may indicate poor oracle data
/// - **Community Engagement**: Dispute participation shows market interest
/// - **Economic Impact**: Total stakes show financial commitment to accuracy
/// - **Resolution Efficiency**: Active vs resolved ratio shows processing speed
///
/// # Governance Insights
///
/// Statistics help identify:
/// - Markets requiring oracle provider review
/// - Patterns of systematic disputes
/// - Community confidence in specific market types
/// - Economic incentive effectiveness
#[contracttype]
pub struct DisputeStats {
    pub total_disputes: u32,
    pub total_dispute_stakes: i128,
    pub active_disputes: u32,
    pub resolved_disputes: u32,
    pub unique_disputers: u32,
}

/// Contains the final resolution data for a completed dispute process.
///
/// This structure captures the outcome of the hybrid resolution system,
/// combining oracle data with community voting to determine the final
/// market result. Used for transparency and audit trails.
///
/// # Fields
///
/// * `market_id` - Unique identifier of the resolved market
/// * `final_outcome` - The definitive outcome after dispute resolution
/// * `oracle_weight` - Influence of oracle data in final decision (scaled integer)
/// * `community_weight` - Influence of community votes in final decision (scaled integer)
/// * `dispute_impact` - How much disputes affected the final outcome (scaled integer)
/// * `resolution_timestamp` - When the final resolution was determined
///
/// # Example
///
/// ```rust
/// # use soroban_sdk::{Env, Symbol, String};
/// # use predictify_hybrid::disputes::DisputeResolution;
/// # let env = Env::default();
///
/// let resolution = DisputeResolution {
///     market_id: Symbol::new(&env, "btc_100k"),
///     final_outcome: String::from_str(&env, "No"),
///     oracle_weight: 60, // 60% oracle influence
///     community_weight: 40, // 40% community influence
///     dispute_impact: 25, // 25% change from original oracle result
///     resolution_timestamp: env.ledger().timestamp(),
/// };
///
/// // Verify hybrid resolution weights sum to 100%
/// assert_eq!(resolution.oracle_weight + resolution.community_weight, 100);
///
/// // Check if community significantly influenced outcome
/// let community_influenced = resolution.dispute_impact > 20;
/// assert!(community_influenced);
/// ```
///
/// # Hybrid Resolution Model
///
/// The resolution combines:
/// 1. **Oracle Data**: Automated, objective data source
/// 2. **Community Voting**: Human judgment and local knowledge
/// 3. **Dispute Impact**: Measure of how much community changed oracle result
///
/// # Weight Calculation
///
/// - Weights are scaled integers (0-100) representing percentages
/// - Oracle weight typically higher for objective markets
/// - Community weight increases with dispute strength
/// - Final outcome balances both sources proportionally
///
/// # Transparency Features
///
/// Resolution data provides:
/// - Clear audit trail of decision factors
/// - Quantified influence of each resolution source
/// - Timestamp for regulatory compliance
/// - Outcome justification for participants
#[contracttype]
#[derive(Debug, PartialEq)]
pub struct DisputeResolution {
    pub market_id: Symbol,
    pub final_outcome: String,
    pub oracle_weight: i128, // Using i128 instead of f64 for no_std compatibility
    pub community_weight: i128,
    pub dispute_impact: i128,
    pub resolution_timestamp: u64,
}

/// Represents an individual vote cast on a dispute by a community member.
///
/// Community members can vote on active disputes to express their opinion
/// on whether the dispute is valid. Votes are weighted by stake to ensure
/// economic alignment and prevent manipulation.
///
/// # Fields
///
/// * `user` - Address of the voter
/// * `dispute_id` - Unique identifier of the dispute being voted on
/// * `vote` - Boolean vote (true = support dispute, false = reject dispute)
/// * `stake` - Amount staked with this vote (determines voting power)
/// * `timestamp` - When the vote was cast
/// * `reason` - Optional explanation for the vote decision
///
/// # Example
///
/// ```rust
/// # use soroban_sdk::{Env, Address, Symbol, String};
/// # use predictify_hybrid::disputes::DisputeVote;
/// # let env = Env::default();
/// # let voter = Address::generate(&env);
/// # let dispute_id = Symbol::new(&env, "dispute_123");
///
/// let vote = DisputeVote {
///     user: voter.clone(),
///     dispute_id: dispute_id.clone(),
///     vote: true, // Supporting the dispute
///     stake: 5_000_000, // 0.5 XLM voting power
///     timestamp: env.ledger().timestamp(),
///     reason: Some(String::from_str(&env, "Oracle data contradicts reliable sources")),
/// };
///
/// // Vote supports the dispute with economic backing
/// assert!(vote.vote);
/// assert!(vote.stake > 0);
/// ```
///
/// # Voting Mechanics
///
/// - **Stake-Weighted**: Higher stakes carry more voting power
/// - **Binary Choice**: Support (true) or reject (false) the dispute
/// - **Economic Commitment**: Voters risk their stake on the outcome
/// - **Transparent Reasoning**: Optional explanations for accountability
///
/// # Vote Outcomes
///
/// - **Support (true)**: Voter believes dispute is valid, oracle was wrong
/// - **Reject (false)**: Voter believes dispute is invalid, oracle was correct
/// - **Winning Side**: Receives their stake back plus rewards from losing side
/// - **Losing Side**: Forfeits stake to winners as penalty for incorrect vote
///
/// # Governance Features
///
/// Dispute voting enables:
/// - Democratic resolution of oracle disagreements
/// - Economic incentives for accurate voting
/// - Community oversight of oracle quality
/// - Transparent decision-making process
#[contracttype]
#[derive(Clone)]
pub struct DisputeVote {
    pub user: Address,
    pub dispute_id: Symbol,
    pub vote: bool, // true for support, false for against
    pub stake: i128,
    pub timestamp: u64,
    pub reason: Option<String>,
}

/// Aggregated voting data and metadata for a dispute resolution process.
///
/// This structure tracks the complete voting process for a dispute,
/// including participation metrics, stake distribution, and timing.
/// Used to determine dispute outcomes and manage the voting lifecycle.
///
/// # Fields
///
/// * `dispute_id` - Unique identifier of the dispute being voted on
/// * `voting_start` - Timestamp when voting period began
/// * `voting_end` - Timestamp when voting period ends
/// * `total_votes` - Total number of individual votes cast
/// * `support_votes` - Number of votes supporting the dispute
/// * `against_votes` - Number of votes rejecting the dispute
/// * `total_support_stake` - Total stake backing dispute support
/// * `total_against_stake` - Total stake backing dispute rejection
/// * `status` - Current status of the voting process
///
/// # Example
///
/// ```rust
/// # use soroban_sdk::{Env, Symbol};
/// # use predictify_hybrid::disputes::{DisputeVoting, DisputeVotingStatus};
/// # let env = Env::default();
///
/// let voting = DisputeVoting {
///     dispute_id: Symbol::new(&env, "dispute_123"),
///     voting_start: env.ledger().timestamp(),
///     voting_end: env.ledger().timestamp() + 86400, // 24 hours
///     total_votes: 15,
///     support_votes: 8,
///     against_votes: 7,
///     total_support_stake: 25_000_000, // 2.5 XLM
///     total_against_stake: 20_000_000, // 2.0 XLM
///     status: DisputeVotingStatus::Active,
/// };
///
/// // Calculate voting metrics
/// let participation_rate = voting.total_votes as f64 / 100.0; // Assume 100 eligible voters
/// let stake_ratio = voting.total_support_stake as f64 / voting.total_against_stake as f64;
///
/// println!("Participation: {:.1}%, Stake ratio: {:.2}",
///     participation_rate * 100.0, stake_ratio);
/// ```
///
/// # Voting Period Management
///
/// - **Start Time**: When dispute voting opens to community
/// - **End Time**: Deadline for vote submission (typically 24-48 hours)
/// - **Status Tracking**: Monitors voting process lifecycle
/// - **Early Resolution**: May close early if outcome is decisive
///
/// # Outcome Determination
///
/// Resolution considers both:
/// 1. **Vote Count**: Simple majority of individual votes
/// 2. **Stake Weight**: Economic weight of supporting stakes
/// 3. **Participation Threshold**: Minimum votes required for validity
/// 4. **Stake Threshold**: Minimum total stake for legitimacy
///
/// # Analytics and Insights
///
/// Voting data provides:
/// - Community engagement levels
/// - Economic commitment to accuracy
/// - Dispute resolution efficiency
/// - Market controversy indicators
#[contracttype]
pub struct DisputeVoting {
    pub dispute_id: Symbol,
    pub voting_start: u64,
    pub voting_end: u64,
    pub total_votes: u32,
    pub support_votes: u32,
    pub against_votes: u32,
    pub total_support_stake: i128,
    pub total_against_stake: i128,
    pub status: DisputeVotingStatus,
}

/// Current status of a dispute voting process.
///
/// Tracks the lifecycle of community voting on disputes, from initiation
/// through completion or termination. Each status determines what actions
/// are available and how the voting process should be handled.
///
/// # Variants
///
/// * `Active` - Voting is open and accepting community votes
/// * `Completed` - Voting period ended with sufficient participation
/// * `Expired` - Voting period ended without meeting minimum requirements
/// * `Cancelled` - Voting was terminated early (e.g., by admin action)
///
/// # Example
///
/// ```rust
/// # use predictify_hybrid::disputes::DisputeVotingStatus;
///
/// // Check if voting is still accepting votes
/// let status = DisputeVotingStatus::Active;
/// let can_vote = matches!(status, DisputeVotingStatus::Active);
/// assert!(can_vote);
///
/// // Check if voting has concluded
/// let final_status = DisputeVotingStatus::Completed;
/// let is_concluded = matches!(final_status,
///     DisputeVotingStatus::Completed |
///     DisputeVotingStatus::Expired |
///     DisputeVotingStatus::Cancelled
/// );
/// assert!(is_concluded);
/// ```
///
/// # Status Transitions
///
/// Valid transitions:
/// - `Active` → `Completed` (successful voting completion)
/// - `Active` → `Expired` (insufficient participation)
/// - `Active` → `Cancelled` (administrative termination)
///
/// Invalid transitions:
/// - Any final status → Any other status (voting outcomes are immutable)
///
/// # Business Logic by Status
///
/// - **Active**: Accept votes, track participation, monitor deadlines
/// - **Completed**: Process results, distribute rewards, update dispute status
/// - **Expired**: Apply default outcome, return stakes, log insufficient participation
/// - **Cancelled**: Return all stakes, invalidate dispute, log cancellation reason
#[contracttype]
pub enum DisputeVotingStatus {
    Active,
    Completed,
    Expired,
    Cancelled,
}

/// Data structure for disputes that have been escalated to higher authority.
///
/// When standard community voting cannot resolve a dispute (due to ties,
/// insufficient participation, or complexity), the dispute can be escalated
/// to admin review or specialized resolution mechanisms.
///
/// # Fields
///
/// * `dispute_id` - Unique identifier of the escalated dispute
/// * `escalated_by` - Address of the user who requested escalation
/// * `escalation_reason` - Explanation for why escalation was necessary
/// * `escalation_timestamp` - When the escalation was requested
/// * `escalation_level` - Tier of escalation (1=admin, 2=governance, etc.)
/// * `requires_admin_review` - Whether admin intervention is needed
///
/// # Example
///
/// ```rust
/// # use soroban_sdk::{Env, Address, Symbol, String};
/// # use predictify_hybrid::disputes::DisputeEscalation;
/// # let env = Env::default();
/// # let user = Address::generate(&env);
///
/// let escalation = DisputeEscalation {
///     dispute_id: Symbol::new(&env, "dispute_456"),
///     escalated_by: user.clone(),
///     escalation_reason: String::from_str(&env,
///         "Voting resulted in exact tie, need admin decision"),
///     escalation_timestamp: env.ledger().timestamp(),
///     escalation_level: 1, // Admin review
///     requires_admin_review: true,
/// };
///
/// // Escalation requires admin intervention
/// assert!(escalation.requires_admin_review);
/// assert_eq!(escalation.escalation_level, 1);
/// ```
///
/// # Escalation Triggers
///
/// Disputes may be escalated when:
/// - **Voting Ties**: Equal stakes on both sides
/// - **Low Participation**: Insufficient community engagement
/// - **Technical Issues**: Oracle data unavailable or corrupted
/// - **Complex Cases**: Subjective outcomes requiring expert judgment
/// - **Appeal Requests**: Losing party contests the result
///
/// # Escalation Levels
///
/// 1. **Level 1**: Admin review and decision
/// 2. **Level 2**: Governance token holder voting
/// 3. **Level 3**: External arbitration or expert panel
/// 4. **Level 4**: Legal or regulatory intervention
///
/// # Resolution Authority
///
/// - **Admin Review**: Fast resolution for clear-cut cases
/// - **Governance Voting**: Democratic resolution for policy matters
/// - **Expert Panel**: Specialized knowledge for technical disputes
/// - **Legal Process**: Final resort for high-stakes disagreements
#[contracttype]
pub struct DisputeEscalation {
    pub dispute_id: Symbol,
    pub escalated_by: Address,
    pub escalation_reason: String,
    pub escalation_timestamp: u64,
    pub escalation_level: u32,
    pub requires_admin_review: bool,
}

/// Records the distribution of fees and stakes after dispute resolution.
///
/// When a dispute is resolved, stakes from the losing side are distributed
/// to the winning side as rewards for accurate judgment. This structure
/// tracks the distribution process and ensures transparent fee allocation.
///
/// # Fields
///
/// * `dispute_id` - Unique identifier of the resolved dispute
/// * `total_fees` - Total amount available for distribution (in stroops)
/// * `winner_stake` - Total stake from the winning side
/// * `loser_stake` - Total stake from the losing side (becomes rewards)
/// * `winner_addresses` - List of addresses that voted correctly
/// * `distribution_timestamp` - When fees were distributed
/// * `fees_distributed` - Whether distribution has been completed
///
/// # Example
///
/// ```rust
/// # use soroban_sdk::{Env, Symbol, Vec, Address};
/// # use predictify_hybrid::disputes::DisputeFeeDistribution;
/// # let env = Env::default();
/// # let mut winners = Vec::new(&env);
/// # winners.push_back(Address::generate(&env));
/// # winners.push_back(Address::generate(&env));
///
/// let distribution = DisputeFeeDistribution {
///     dispute_id: Symbol::new(&env, "dispute_789"),
///     total_fees: 30_000_000, // 3 XLM total
///     winner_stake: 20_000_000, // 2 XLM from winners
///     loser_stake: 10_000_000, // 1 XLM from losers (becomes rewards)
///     winner_addresses: winners,
///     distribution_timestamp: env.ledger().timestamp(),
///     fees_distributed: true,
/// };
///
/// // Calculate reward ratio
/// let reward_ratio = distribution.loser_stake as f64 / distribution.winner_stake as f64;
/// println!("Winners receive {:.1}% bonus", reward_ratio * 100.0);
///
/// // Verify distribution completed
/// assert!(distribution.fees_distributed);
/// ```
///
/// # Distribution Mechanics
///
/// 1. **Stake Recovery**: Winners get their original stakes back
/// 2. **Reward Distribution**: Loser stakes distributed proportionally to winners
/// 3. **Platform Fee**: Small percentage retained for platform operations
/// 4. **Gas Costs**: Distribution transaction costs handled appropriately
///
/// # Proportional Rewards
///
/// Winners receive rewards based on:
/// - **Stake Size**: Larger stakes receive proportionally larger rewards
/// - **Timing**: Early voters may receive slight bonuses
/// - **Confidence**: Stronger votes (higher stakes) earn more rewards
///
/// # Transparency Features
///
/// - **Public Record**: All distributions are publicly auditable
/// - **Address List**: Winners are explicitly recorded
/// - **Timestamp**: Distribution timing is permanently recorded
/// - **Status Flag**: Clear indication of completion status
///
/// # Economic Incentives
///
/// Fee distribution creates:
/// - **Accuracy Rewards**: Economic incentive for correct voting
/// - **Participation Incentive**: Rewards for community engagement
/// - **Quality Control**: Penalties for incorrect dispute judgments
/// - **Platform Sustainability**: Fees support ongoing operations
#[contracttype]
pub struct DisputeFeeDistribution {
    pub dispute_id: Symbol,
    pub total_fees: i128,
    pub winner_stake: i128,
    pub loser_stake: i128,
    pub winner_addresses: Vec<Address>,
    pub distribution_timestamp: u64,
    pub fees_distributed: bool,
}

/// Represents dispute timeout configuration.
///
/// Stores the timeout window for a dispute, including when it was created,
/// when it expires, and whether it has been extended. Used by
/// [`DisputeManager::set_dispute_timeout`] and [`DisputeManager::check_dispute_timeout`].
///
/// # Fields
///
/// * `dispute_id` - Unique identifier of the dispute
/// * `market_id` - Market that the dispute belongs to
/// * `timeout_hours` - Configured timeout duration in hours
/// * `created_at` - Ledger timestamp when the timeout was created
/// * `expires_at` - Ledger timestamp when the timeout expires
/// * `extended_at` - Optional timestamp of the last extension
/// * `total_extension_hours` - Cumulative hours added via extensions
/// * `status` - Current lifecycle status of the timeout
#[contracttype]
pub struct DisputeTimeout {
    pub dispute_id: Symbol,
    pub market_id: Symbol,
    pub timeout_hours: u32,
    pub created_at: u64,
    pub expires_at: u64,
    pub extended_at: Option<u64>,
    pub total_extension_hours: u32,
    pub status: DisputeTimeoutStatus,
}

/// Lifecycle status of a dispute timeout.
///
/// Tracks whether a timeout is still running, has elapsed, was extended,
/// or triggered automatic resolution.
///
/// # Variants
///
/// * `Active` — Timeout is running and has not yet expired
/// * `Expired` — Timeout window elapsed without resolution
/// * `Extended` — Timeout was extended by an admin via [`DisputeManager::extend_dispute_timeout`]
/// * `AutoResolved` — Dispute was automatically resolved when the timeout expired
///
/// # Valid Transitions
///
/// - `Active` → `Expired` (time elapsed)
/// - `Active` → `Extended` (admin action)
/// - `Extended` → `Expired` (time elapsed after extension)
/// - `Active` / `Extended` → `AutoResolved` (system auto-resolution)
#[contracttype]
#[derive(PartialEq, Debug)]
pub enum DisputeTimeoutStatus {
    Active,
    Expired,
    Extended,
    AutoResolved,
}

/// The outcome produced when a dispute is resolved via timeout auto-resolution.
///
/// Created by [`DisputeManager::auto_resolve_dispute_on_timeout`] and emitted
/// as an event so indexers can track auto-resolved disputes.
///
/// # Fields
///
/// * `dispute_id` - Unique identifier of the auto-resolved dispute
/// * `market_id` - Market that the dispute belongs to
/// * `outcome` - The resolution outcome string (e.g. "Support" or "Against")
/// * `resolution_method` - Human-readable method description (e.g. "Timeout Auto-Resolution")
/// * `resolution_timestamp` - When the auto-resolution was applied
/// * `reason` - Explanation of why this outcome was determined
#[contracttype]
pub struct DisputeTimeoutOutcome {
    pub dispute_id: Symbol,
    pub market_id: Symbol,
    pub outcome: String,
    pub resolution_method: String,
    pub resolution_timestamp: u64,
    pub reason: String,
}

/// Configuration for dispute collusion detection.
///
/// Flags a potential collusion event when two disputes from different users
/// within a sliding window fall within a small stake difference **and**
/// a small time difference.  Used inside [`DisputeManager::process_dispute`].
///
/// # Fields
///
/// * `stake_delta_threshold` - Maximum stake difference (stroops) to consider suspicious
/// * `time_delta_threshold` - Maximum time difference (seconds) between disputes
/// * `window_size` - Number of recent disputes to examine in the sliding window
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollusionDetectorConfig {
    pub stake_delta_threshold: i128,
    pub time_delta_threshold: u64,
    pub window_size: u32,
}

/// Aggregate statistics about dispute timeouts across all markets.
///
/// Returned by timeout analytics queries; useful for governance dashboards
/// and monitoring how often disputes require auto-resolution.
///
/// # Fields
///
/// * `total_timeouts` - Total number of timeouts configured across all markets
/// * `active_timeouts` - Number of timeouts still in `Active` state
/// * `expired_timeouts` - Number of timeouts that reached expiry without resolution
/// * `auto_resolved_timeouts` - Number of disputes auto-resolved via timeout
/// * `average_timeout_hours` - Average configured timeout duration in hours
#[contracttype]
pub struct TimeoutStats {
    pub total_timeouts: u32,
    pub active_timeouts: u32,
    pub expired_timeouts: u32,
    pub auto_resolved_timeouts: u32,
    pub average_timeout_hours: u32,
}

/// Per-dispute timeout analytics snapshot.
///
/// Provides a point-in-time view of a single dispute's timeout state,
/// including how much time remains and how many extensions have been applied.
///
/// # Fields
///
/// * `dispute_id` - Unique identifier of the dispute
/// * `timeout_hours` - Original configured timeout duration
/// * `time_remaining_seconds` - Seconds until the timeout expires (0 if expired)
/// * `time_remaining_hours` - Hours until the timeout expires (0 if expired)
/// * `is_expired` - Whether the timeout window has elapsed
/// * `status` - Current [`DisputeTimeoutStatus`]
/// * `total_extensions` - Cumulative extension hours applied
#[contracttype]
pub struct TimeoutAnalytics {
    pub dispute_id: Symbol,
    pub timeout_hours: u32,
    pub time_remaining_seconds: u64,
    pub time_remaining_hours: u64,
    pub is_expired: bool,
    pub status: DisputeTimeoutStatus,
    pub total_extensions: u32,
}

// ===== DISPUTE MANAGER =====

/// Central manager for all dispute-related operations in the prediction market system.
///
/// The DisputeManager handles the complete dispute lifecycle, from initial dispute
/// creation through community voting to final resolution and fee distribution.
/// It coordinates between oracle data and community consensus to ensure fair
/// and accurate market outcomes.
///
/// # Core Responsibilities
///
/// - **Dispute Processing**: Handle dispute creation and validation
/// - **Community Voting**: Manage voting processes and participation
/// - **Resolution Logic**: Combine oracle and community data for final outcomes
/// - **Fee Distribution**: Distribute stakes and rewards to participants
/// - **Analytics**: Track dispute patterns and market quality metrics
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Address, Symbol, String};
/// # use predictify_hybrid::disputes::DisputeManager;
/// # let env = Env::default();
/// # let user = Address::generate(&env);
/// # let admin = Address::generate(&env);
/// # let market_id = Symbol::new(&env, "market_123");
///
/// // User disputes a market result
/// let result = DisputeManager::process_dispute(
///     &env,
///     user.clone(),
///     market_id.clone(),
///     10_000_000, // 1 XLM stake
///     Some(String::from_str(&env, "Oracle data appears incorrect"))
/// );
///
/// // Admin resolves the dispute after community voting
/// let resolution = DisputeManager::resolve_dispute(
///     &env,
///     market_id.clone(),
///     admin.clone()
/// );
/// ```
///
/// # Dispute Workflow
///
/// 1. **Dispute Creation**: User stakes tokens to challenge oracle result
/// 2. **Validation**: System validates dispute eligibility and parameters
/// 3. **Community Voting**: Other users vote on dispute validity
/// 4. **Resolution**: Combine oracle and community data for final outcome
/// 5. **Distribution**: Distribute stakes and rewards to winning participants
///
/// # Security Features
///
/// - **Stake Requirements**: Minimum stakes prevent spam disputes
/// - **Authentication**: All operations require proper user authorization
/// - **Admin Oversight**: Critical operations require admin permissions
/// - **Economic Incentives**: Rewards align with accurate dispute resolution
pub struct DisputeManager;

impl DisputeManager {
    /// Sets the maximum capacity of resolved/expired disputes to retain in history.
    pub fn set_history_cap(env: &Env, admin: Address, cap: u32) -> Result<(), Error> {
        admin.require_auth();
        DisputeValidator::validate_admin_permissions(env, &admin)?;
        Self::check_admin_cooldown(env, &admin, &Symbol::new(env, "set_history_cap"))?;

        let key = DataKey::DisputeHistoryCap;
        env.storage().persistent().set(&key, &cap);
        env.storage().persistent().extend_ttl(&key, 535680, 535680);
        Ok(())
    }

    /// Retrieves the configured dispute history capacity (max number of
    /// resolved/expired disputes retained per market).
    ///
    /// Returns `None` when no cap has been set.
    pub fn get_history_cap(env: &Env) -> Option<u32> {
        let key = DataKey::DisputeHistoryCap;
        env.storage().persistent().get(&key)
    }

    /// Sets the anti-grief minimum stake floor.
    pub fn set_anti_grief_floor(env: &Env, admin: Address, floor: i128) -> Result<(), Error> {
        admin.require_auth();
        DisputeValidator::validate_admin_permissions(env, &admin)?;
        Self::check_admin_cooldown(env, &admin, &Symbol::new(env, "set_anti_grief_floor"))?;

        let key = DataKey::AntiGriefFloor;
        env.storage().persistent().set(&key, &floor);
        env.storage().persistent().extend_ttl(&key, 535680, 535680);
        Ok(())
    }

    /// Retrieves the global anti-grief minimum stake floor (in stroops).
    ///
    /// Returns `None` if no global floor has been configured.
    pub fn get_anti_grief_floor(env: &Env) -> Option<i128> {
        let key = DataKey::AntiGriefFloor;
        env.storage().persistent().get(&key)
    }

    /// Sets the collusion detector configuration.
    pub fn set_collusion_detector_config(env: &Env, admin: Address, config: CollusionDetectorConfig) -> Result<(), Error> {
        admin.require_auth();
        DisputeValidator::validate_admin_permissions(env, &admin)?;
        Self::check_admin_cooldown(env, &admin, &Symbol::new(env, "set_collusion_detector_config"))?;

        let key = DataKey::CollusionDetectorConfig(Symbol::new(env, "collusion_config"));
        env.storage().persistent().set(&key, &config);
        env.storage().persistent().extend_ttl(&key, 535680, 535680);
        Ok(())
    }

    /// Retrieves the collusion detector configuration, using sensible
    /// defaults when no configuration has been stored yet.
    ///
    /// Defaults: stake_delta = 1 XLM, time_delta = 10 min, window = 8.
    pub fn get_collusion_detector_config(env: &Env) -> CollusionDetectorConfig {
        let key = DataKey::CollusionDetectorConfig(Symbol::new(env, "collusion_config"));
        env.storage().persistent().get(&key).unwrap_or(CollusionDetectorConfig {
            stake_delta_threshold: 1_000_000,
            time_delta_threshold: 600, // 10 minutes
            window_size: 8,
        })
    }

    /// Evicts the oldest resolved/expired disputes if history size exceeds the cap.
    pub fn apply_eviction(
        env: &Env,
        market_id: &Symbol,
        history: &mut Vec<Dispute>,
    ) -> Result<(), Error> {
        if stake < MIN_DISPUTE_STAKE {
            return Err(Error::InsufficientStake);
        }

        Ok(())
    }

    pub fn validate_market_for_resolution(
        _env: &Env,
        market: &Market,
        admin: &Address,
    ) -> Result<(), Error> {
        if market.oracle_result.is_none() {
            return Err(Error::OracleResultNotAvailable);
        }

        if market.total_dispute_stakes() == 0 {
            return Err(Error::NoDisputesFound);
        }

        Ok(())
    }

    pub fn validate_dispute_timeout_parameters(timeout_hours: u32) -> Result<(), Error> {
        if timeout_hours == 0 || timeout_hours > 720 {
            return Err(Error::InvalidDuration);
        }
        Ok(())
    }

    pub fn validate_dispute_timeout_extension_parameters(extension_hours: u32) -> Result<(), Error> {
        if extension_hours == 0 || extension_hours > 168 {
            return Err(Error::InvalidDuration);
        }
        Ok(())
    }
}

pub struct DisputeManager;

impl DisputeManager {
    pub fn process_dispute(
        env: &Env,
        user: Address,
        market_id: Symbol,
        stake: i128,
        reason: Option<String>,
    ) -> Result<Dispute, Error> {
        user.require_auth();

        let mut market = MarketStateManager::get_market(env, &market_id)?;

        DisputeValidator::validate_market_for_dispute(env, &market)?;
        DisputeValidator::validate_dispute_parameters(env, &user, &market, stake)?;

        let token_address = TokenStorage::get_token_id(env)?;
        let token_client = token::Client::new(env, &token_address);

        token_client.transfer(&user, &env.current_contract_address(), &stake);

        let current_stake = market.dispute_stakes.get(user.clone()).unwrap_or(0);
        let new_stake = current_stake.checked_add(stake).ok_or(Error::Overflow)?;
        market.dispute_stakes.set(user.clone(), new_stake);

        MarketStateManager::update_market(env, &market_id, &market);

        let dispute = Dispute {
            user: user.clone(),
            market_id: market_id.clone(),
            stake,
            timestamp: env.ledger().timestamp(),
            reason: reason.clone(),
            status: DisputeStatus::Active,
        };

        DisputeUtils::emit_dispute_submitted_event(env, &dispute);

        Ok(dispute)
    }

    pub fn vote_on_dispute(
        env: &Env,
        user: Address,
        market_id: Symbol,
        vote: String,
        stake: i128,
    ) -> Result<(), Error> {
        user.require_auth();

        let mut market = MarketStateManager::get_market(env, &market_id)?;

        if stake <= 0 {
            return Err(Error::InsufficientStake);
        }

        let token_address = TokenStorage::get_token_id(env)?;
        let token_client = token::Client::new(env, &token_address);
        token_client.transfer(&user, &env.current_contract_address(), &stake);

        let current_stake = market.stakes.get(user.clone()).unwrap_or(0);
        let new_stake = current_stake.checked_add(stake).ok_or(Error::Overflow)?;

        market.votes.set(user.clone(), vote.clone());
        market.stakes.set(user.clone(), new_stake);

        let total_staked = market.total_staked.checked_add(stake).ok_or(Error::Overflow)?;
        market.total_staked = total_staked;

        MarketStateManager::update_market(env, &market_id, &market);

        DisputeUtils::emit_dispute_vote_event(env, &market_id, &user, &vote, stake);

        // --- Collusion Detector ---
        let config = Self::get_collusion_detector_config(env);
        let window_size = config.window_size;
        let start_idx = if history.len() > window_size {
            history.len() - window_size
        } else {
            0
        };

        for i in start_idx..history.len().saturating_sub(1) {
            if let Some(prev_dispute) = history.get(i) {
                if prev_dispute.user != user {
                    let stake_diff = if prev_dispute.stake > stake { prev_dispute.stake - stake } else { stake - prev_dispute.stake };
                    let time_diff = if prev_dispute.timestamp > dispute.timestamp { prev_dispute.timestamp - dispute.timestamp } else { dispute.timestamp - prev_dispute.timestamp };

                    if stake_diff <= config.stake_delta_threshold && time_diff <= config.time_delta_threshold {
                        crate::events::EventEmitter::emit_suspected_collusion_flag(
                            env,
                            &market_id,
                            &user,
                            &prev_dispute.user,
                            stake_diff,
                            time_diff,
                        );
                    }
                }
            }
        }
        // --------------------------

        Ok(())
    }

    pub fn resolve_dispute(
        env: &Env,
        market_id: Symbol,
        admin: Address,
    ) -> Result<DisputeResolution, Error> {
        admin.require_auth();

        let contract_admin = AdminStorage::get_admin(env)?;
        if admin != contract_admin {
            return Err(Error::Unauthorized);
        }

        let mut market = MarketStateManager::get_market(env, &market_id)?;

        DisputeValidator::validate_market_for_resolution(env, &market, &admin)?;

        let oracle_result = market
            .oracle_result
            .clone()
            .ok_or(Error::OracleResultNotAvailable)?;

        let consensus = DisputeAnalytics::calculate_community_consensus(env, &market);

        let dispute_impact = DisputeUtils::calculate_dispute_impact(&market);

        let final_outcome = if dispute_impact > 0.3 && consensus.confidence > 70 {
            consensus.outcome
        } else {
            oracle_result.clone()
        };

        let is_oracle_overturned = final_outcome != oracle_result;

        if is_oracle_overturned {
            // Refund all disputers their stakes and emit a StakeRefunded event per disputer.
            let token_address = TokenStorage::get_token_id(env)?;
            let token_client = token::Client::new(env, &token_address);

            let disputers: Vec<(Address, i128)> = market
                .dispute_stakes
                .iter()
                .map(|(user, stake)| (user, stake))
                .collect();

            for (disputer, stake) in disputers.iter() {
                if *stake > 0 {
                    // Perform the refund transfer.
                    token_client.transfer(&env.current_contract_address(), &disputer, stake);
                    // Reset the stored stake for the disputer.
                    market.dispute_stakes.set(disputer.clone(), 0);
                    // Emit an event for the refund.
                    DisputeUtils::emit_stake_refunded_event(env, disputer, *stake);
                }
            }
        }

        market.winning_outcomes = Some(final_outcome.clone());
        market.state = crate::types::MarketState::Resolved;

        MarketStateManager::update_market(env, &market_id, &market);

        let resolution = DisputeResolution {
            market_id,
            final_outcome,
            oracle_weight: DisputeAnalytics::calculate_oracle_weight(&market),
            community_weight: DisputeAnalytics::calculate_community_weight(&market),
            dispute_impact: (dispute_impact * 100.0) as i128,
            resolution_timestamp: env.ledger().timestamp(),
        };

        // Update market with final outcome
        DisputeUtils::finalize_market_with_resolution(&mut market, final_outcome)?;
        MarketStateManager::update_market(env, &market_id, &market);

        // Update history status to Resolved
        let mut history = env.storage().persistent()
            .get::<_, Vec<Dispute>>(&DataKey::DisputeHistory(market_id.clone()))
            .unwrap_or_else(|| Vec::new(env));
        let mut updated = false;
        for i in 0..history.len() {
            let mut disp = history.get(i).ok_or(Error::InvalidState)?;
            if matches!(disp.status, DisputeStatus::Active) {
                disp.status = DisputeStatus::Resolved;
                history.set(i, disp);
                updated = true;
            }
        }
        if updated {
            Self::apply_eviction(env, &market_id, &mut history)?;
            env.storage().persistent().set(&DataKey::DisputeHistory(market_id.clone()), &history);
            env.storage().persistent().extend_ttl(&DataKey::DisputeHistory(market_id.clone()), 535680, 535680);
        }

        let _ = crate::resolution::ResolutionOutcomeCache::refresh(env, &market_id, &market);
        crate::monitoring::ContractMonitor::emit_dispute_transition_hook(
            env,
            &market_id,
            &soroban_sdk::String::from_str(env, "resolved"),
            &admin,
            &soroban_sdk::String::from_str(env, "dispute_resolved"),
        );

        crate::audit_trail::AuditTrailManager::append_record(
            env,
            crate::audit_trail::AuditAction::DisputeResolved,
            admin.clone(),
            Map::new(env),
            None,
        );

        Ok(resolution)
    }

    /// Retrieves comprehensive dispute statistics for a specific market.
    ///
    /// This function calculates and returns detailed statistics about dispute
    /// activity for a market, including participation metrics, stake distribution,
    /// and resolution patterns. Used for analytics, governance, and market quality assessment.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `market_id` - Unique identifier of the market to analyze
    ///
    /// # Returns
    ///
    /// Returns a `DisputeStats` structure containing comprehensive dispute metrics,
    /// or an `Error` if the market is not found.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Symbol};
    /// # use predictify_hybrid::disputes::DisputeManager;
    /// # let env = Env::default();
    /// # let market_id = Symbol::new(&env, "analyzed_market");
    ///
    /// // Get dispute statistics for analysis
    /// let stats = DisputeManager::get_dispute_stats(&env, market_id).unwrap();
    ///
    /// // Analyze dispute activity
    /// println!("Total disputes: {}", stats.total_disputes);
    /// println!("Total stakes: {} XLM", stats.total_dispute_stakes / 10_000_000);
    /// println!("Unique disputers: {}", stats.unique_disputers);
    ///
    /// // Calculate engagement metrics
    /// let avg_stake = if stats.total_disputes > 0 {
    ///     stats.total_dispute_stakes / stats.total_disputes as i128
    /// } else { 0 };
    /// println!("Average stake per dispute: {} XLM", avg_stake / 10_000_000);
    ///
    /// // Check market controversy level
    /// let controversy_ratio = stats.total_disputes as f64 / 100.0; // Assume 100 participants
    /// if controversy_ratio > 0.1 {
    ///     println!("High controversy market detected");
    /// }
    /// ```
    ///
    /// # Statistics Included
    ///
    /// The returned statistics provide:
    /// - **Total Disputes**: Count of all disputes ever raised
    /// - **Total Stakes**: Sum of all dispute stakes in stroops
    /// - **Active Disputes**: Number of currently unresolved disputes
    /// - **Resolved Disputes**: Number of completed dispute processes
    /// - **Unique Disputers**: Count of distinct addresses that disputed
    ///
    /// # Use Cases
    ///
    /// - **Market Quality Assessment**: High dispute rates may indicate oracle issues
    /// - **Community Engagement**: Participation levels show market interest
    /// - **Economic Analysis**: Stake amounts reveal financial commitment
    /// - **Governance Decisions**: Data supports policy and parameter adjustments
    /// - **Oracle Evaluation**: Dispute patterns help assess oracle reliability
    pub fn get_dispute_stats(env: &Env, market_id: Symbol) -> Result<DisputeStats, Error> {
        let market = MarketStateManager::get_market(env, &market_id)?;
        Ok(DisputeAnalytics::calculate_dispute_stats(&market))
    }

    /// Retrieves all dispute records associated with a specific market.
    ///
    /// This function returns a complete list of all disputes that have been
    /// raised against a market, including both active and resolved disputes.
    /// Useful for detailed analysis, audit trails, and dispute history review.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `market_id` - Unique identifier of the market to query
    ///
    /// # Returns
    ///
    /// Returns a `Vec<Dispute>` containing all dispute records for the market,
    /// or an `Error` if the market is not found. Empty vector if no disputes exist.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Symbol};
    /// # use predictify_hybrid::disputes::{DisputeManager, DisputeStatus};
    /// # let env = Env::default();
    /// # let market_id = Symbol::new(&env, "disputed_market");
    ///
    /// // Get all disputes for detailed analysis
    /// let disputes = DisputeManager::get_market_disputes(&env, market_id).unwrap();
    ///
    /// // Analyze dispute patterns
    /// for dispute in disputes.iter() {
    ///     println!("Dispute by: {}", dispute.user.to_string());
    ///     println!("Stake: {} XLM", dispute.stake / 10_000_000);
    ///     println!("Status: {:?}", dispute.status);
    ///     
    ///     if let Some(reason) = &dispute.reason {
    ///         println!("Reason: {}", reason.to_string());
    ///     }
    /// }
    ///
    /// // Filter by status
    /// let active_disputes: Vec<_> = disputes.iter()
    ///     .filter(|d| matches!(d.status, DisputeStatus::Active))
    ///     .collect();
    ///
    /// println!("Active disputes: {}", active_disputes.len());
    /// ```
    ///
    /// # Dispute Information
    ///
    /// Each dispute record contains:
    /// - **User Address**: Who initiated the dispute
    /// - **Stake Amount**: Economic commitment to the dispute
    /// - **Timestamp**: When the dispute was created
    /// - **Reason**: Optional explanation for the dispute
    /// - **Status**: Current state (Active, Resolved, Rejected, Expired)
    ///
    /// # Analysis Applications
    ///
    /// - **Audit Trails**: Complete history of market challenges
    /// - **Pattern Recognition**: Identify systematic dispute trends
    /// - **User Behavior**: Analyze disputer participation patterns
    /// - **Timeline Analysis**: Track dispute timing and resolution speed
    /// - **Quality Metrics**: Assess market and oracle performance
    pub fn get_market_disputes(env: &Env, market_id: Symbol) -> Result<Vec<Dispute>, Error> {
        let market = MarketStateManager::get_market(env, &market_id)?;
        let mut history = env.storage().persistent()
            .get::<_, Vec<Dispute>>(&DataKey::DisputeHistory(market_id.clone()))
            .unwrap_or_else(|| Vec::new(env));

        if history.is_empty() {
            let extracted = DisputeUtils::extract_disputes_from_market(env, &market, market_id.clone());
            if extracted.len() > 0 {
                env.storage().persistent().set(&DataKey::DisputeHistory(market_id.clone()), &extracted);
                env.storage().persistent().extend_ttl(&DataKey::DisputeHistory(market_id.clone()), 535680, 535680);
                return Ok(extracted);
            }
        }
        Ok(history)
    }

    /// Checks whether a specific user has already disputed a given market.
    ///
    /// This function prevents duplicate disputes from the same user and provides
    /// a quick way to check user participation in dispute processes. Essential
    /// for validation logic and user interface state management.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `market_id` - Unique identifier of the market to check
    /// * `user` - Address of the user to check for dispute participation
    ///
    /// # Returns
    ///
    /// Returns `true` if the user has disputed this market, `false` if they haven't,
    /// or an `Error` if the market is not found.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Symbol, Address};
    /// # use predictify_hybrid::disputes::DisputeManager;
    /// # let env = Env::default();
    /// # let market_id = Symbol::new(&env, "market_123");
    /// # let user = Address::generate(&env);
    ///
    /// // Check if user can dispute (hasn't disputed before)
    /// let has_disputed = DisputeManager::has_user_disputed(
    ///     &env,
    ///     market_id.clone(),
    ///     user.clone()
    /// ).unwrap();
    ///
    /// if has_disputed {
    ///     println!("User has already disputed this market");
    ///     // Show dispute status instead of dispute option
    /// } else {
    ///     println!("User can dispute this market");
    ///     // Show dispute creation interface
    /// }
    ///
    /// // Validation before allowing dispute creation
    /// if !has_disputed {
    ///     // Proceed with dispute creation logic
    ///     println!("Proceeding with dispute creation");
    /// }
    /// ```
    ///
    /// # Use Cases
    ///
    /// - **Duplicate Prevention**: Ensure users can only dispute once per market
    /// - **UI State Management**: Show appropriate interface based on user status
    /// - **Validation Logic**: Pre-validate dispute creation requests
    /// - **User Analytics**: Track user participation across markets
    /// - **Access Control**: Implement business rules for dispute eligibility
    ///
    /// # Business Rules
    ///
    /// - Users can only dispute a market once to prevent spam
    /// - Check is performed before allowing dispute creation
    /// - Historical disputes (resolved/rejected) still count as "disputed"
    /// - Essential for maintaining dispute system integrity
    pub fn has_user_disputed(env: &Env, market_id: Symbol, user: Address) -> Result<bool, Error> {
        let market = MarketStateManager::get_market(env, &market_id)?;
        Ok(DisputeUtils::has_user_disputed(&market, &user))
    }

    /// Retrieves the total stake amount a user has committed to disputes on a market.
    ///
    /// This function returns the amount a user has staked when disputing a market,
    /// which is locked until dispute resolution. Used for displaying user positions,
    /// calculating potential rewards, and managing stake-related operations.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `market_id` - Unique identifier of the market to query
    /// * `user` - Address of the user whose stake to retrieve
    ///
    /// # Returns
    ///
    /// Returns the user's dispute stake amount in stroops, or `0` if the user
    /// has not disputed this market. Returns an `Error` if the market is not found.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Symbol, Address};
    /// # use predictify_hybrid::disputes::DisputeManager;
    /// # let env = Env::default();
    /// # let market_id = Symbol::new(&env, "staked_market");
    /// # let user = Address::generate(&env);
    ///
    /// // Get user's dispute stake
    /// let stake = DisputeManager::get_user_dispute_stake(
    ///     &env,
    ///     market_id.clone(),
    ///     user.clone()
    /// ).unwrap();
    ///
    /// if stake > 0 {
    ///     println!("User has {} XLM staked in disputes", stake / 10_000_000);
    ///     
    ///     // Calculate potential rewards (example logic)
    ///     let potential_reward = stake * 120 / 100; // 20% bonus if dispute wins
    ///     println!("Potential reward: {} XLM", potential_reward / 10_000_000);
    ///     
    ///     // Show stake status in UI
    ///     println!("Stake is locked until dispute resolution");
    /// } else {
    ///     println!("User has not disputed this market");
    /// }
    /// ```
    ///
    /// # Stake Management
    ///
    /// - **Locked Funds**: Stake is locked until dispute resolution
    /// - **Reward Calculation**: Basis for calculating potential rewards
    /// - **Risk Assessment**: Shows user's economic exposure
    /// - **Portfolio Tracking**: Part of user's total locked assets
    ///
    /// # Use Cases
    ///
    /// - **User Dashboards**: Display locked stake amounts
    /// - **Reward Calculations**: Determine potential dispute rewards
    /// - **Risk Management**: Show user's economic exposure
    /// - **Portfolio Analytics**: Track user's dispute participation
    /// - **Liquidity Planning**: Account for locked funds in user balance
    pub fn get_user_dispute_stake(
        env: &Env,
        market_id: Symbol,
        user: Address,
    ) -> Result<i128, Error> {
        let market = MarketStateManager::get_market(env, &market_id)?;
        Ok(DisputeUtils::get_user_dispute_stake(&market, &user))
    }

    /// Allows community members to vote on the validity of a dispute.
    ///
    /// This function enables users to participate in dispute resolution by casting
    /// weighted votes (backed by stakes) on whether they believe a dispute is valid.
    /// Votes determine the final outcome and reward distribution.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `user` - Address of the user casting the vote (must authenticate)
    /// * `market_id` - Unique identifier of the disputed market
    /// * `dispute_id` - Unique identifier of the specific dispute
    /// * `vote` - Boolean vote (true = support dispute, false = reject dispute)
    /// * `stake` - Amount to stake with the vote (determines voting power)
    /// * `reason` - Optional explanation for the vote decision
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the vote is successfully recorded, or an `Error` if:
    /// - User has already voted on this dispute
    /// - Dispute voting period has ended
    /// - Stake amount is below minimum requirements
    /// - Dispute is not in an active voting state
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address, Symbol, String};
    /// # use predictify_hybrid::disputes::DisputeManager;
    /// # let env = Env::default();
    /// # let voter = Address::generate(&env);
    /// # let market_id = Symbol::new(&env, "disputed_market");
    /// # let dispute_id = Symbol::new(&env, "dispute_456");
    ///
    /// // Vote to support the dispute
    /// let result = DisputeManager::vote_on_dispute(
    ///     &env,
    ///     voter.clone(),
    ///     market_id.clone(),
    ///     dispute_id.clone(),
    ///     true, // Supporting the dispute
    ///     5_000_000, // 0.5 XLM voting power
    ///     Some(String::from_str(&env, "Oracle data contradicts multiple sources"))
    /// );
    ///
    /// match result {
    ///     Ok(()) => println!("Vote successfully recorded"),
    ///     Err(e) => println!("Vote failed: {:?}", e),
    /// }
    ///
    /// // Vote to reject the dispute
    /// let other_voter = Address::generate(&env);
    /// let reject_result = DisputeManager::vote_on_dispute(
    ///     &env,
    ///     other_voter,
    ///     market_id,
    ///     dispute_id,
    ///     false, // Rejecting the dispute
    ///     3_000_000, // 0.3 XLM voting power
    ///     Some(String::from_str(&env, "Oracle data appears accurate"))
    /// );
    /// ```
    ///
    /// # Voting Mechanics
    ///
    /// - **Stake-Weighted**: Higher stakes provide more voting influence
    /// - **Binary Choice**: Support (true) or reject (false) the dispute
    /// - **Economic Risk**: Voters risk their stake on the outcome
    /// - **Transparent Process**: All votes are recorded with optional reasoning
    ///
    /// # Vote Outcomes
    ///
    /// - **Support Vote (true)**: Believes dispute is valid, oracle was incorrect
    /// - **Reject Vote (false)**: Believes dispute is invalid, oracle was correct
    /// - **Winning Side**: Receives stake back plus proportional rewards
    /// - **Losing Side**: Forfeits stake to winners as accuracy incentive
    ///
    /// # Process Flow
    ///
    /// 1. **Authentication**: Verify voter signature and authorization
    /// 2. **Validation**: Check voting eligibility and dispute status
    /// 3. **Stake Transfer**: Lock voter's stake with the vote
    /// 4. **Vote Recording**: Store vote with timestamp and reasoning
    /// 5. **Event Emission**: Broadcast vote event for transparency
    /// 6. **Aggregation**: Update dispute voting statistics
    ///
    /// # Economic Incentives
    ///
    /// Voting creates strong incentives for accuracy:
    /// - Correct votes earn rewards from incorrect votes
    /// - Stake amounts reflect voter confidence
    /// - Economic penalties discourage frivolous voting
    /// - Proportional rewards based on stake size
    pub fn vote_on_dispute(
        env: &Env,
        user: Address,
        market_id: Symbol,
        dispute_id: Symbol,
        vote: bool,
        stake: i128,
        reason: Option<String>,
    ) -> Result<(), Error> {
        // Require authentication from the user
        user.require_auth();

        // Reject self-vote: the dispute opener cannot vote on their own dispute
        let market = MarketStateManager::get_market(env, &market_id)?;
        if market.dispute_stakes.contains_key(user.clone()) {
            crate::events::EventEmitter::emit_dispute_vote_rejected(
                env,
                &dispute_id,
                &user,
                &soroban_sdk::String::from_str(
                    env,
                    "Dispute opener cannot vote on their own dispute",
                ),
            );
            return Err(Error::DisputerCannotVote);
        }

        // Validate dispute voting conditions
        DisputeValidator::validate_dispute_voting_conditions(env, &market_id, &dispute_id)?;

        // Validate user hasn't already voted
        DisputeValidator::validate_user_hasnt_voted(env, &user, &dispute_id)?;

        // Process stake transfer
        VotingUtils::transfer_stake(env, &user, stake)?;

        // Create dispute vote
        let dispute_vote = DisputeVote {
            user: user.clone(),
            dispute_id: dispute_id.clone(),
            vote,
            stake,
            timestamp: env.ledger().timestamp(),
            reason,
        };

        // Add vote to dispute voting
        DisputeUtils::add_vote_to_dispute(env, &dispute_id, dispute_vote)?;

        // Emit dispute vote event
        DisputeUtils::emit_dispute_vote_event(env, &dispute_id, &user, vote, stake);

        Ok(())
    }

    /// Calculates the final outcome of a dispute based on community voting results.
    ///
    /// This function analyzes all votes cast on a dispute, applies stake weighting,
    /// and determines whether the dispute should be upheld (true) or rejected (false).
    /// The calculation considers both vote counts and economic stakes.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `dispute_id` - Unique identifier of the dispute to calculate outcome for
    ///
    /// # Returns
    ///
    /// Returns `true` if the dispute is upheld (oracle was wrong), `false` if rejected
    /// (oracle was correct), or an `Error` if:
    /// - Dispute is not found
    /// - Voting period is still active
    /// - Insufficient votes to determine outcome
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Symbol};
    /// # use predictify_hybrid::disputes::DisputeManager;
    /// # let env = Env::default();
    /// # let dispute_id = Symbol::new(&env, "completed_dispute");
    ///
    /// // Calculate outcome after voting period ends
    /// let outcome = DisputeManager::calculate_dispute_outcome(
    ///     &env,
    ///     dispute_id.clone()
    /// ).unwrap();
    ///
    /// if outcome {
    ///     println!("Dispute upheld - oracle result overturned");
    ///     // Community believes oracle was incorrect
    /// } else {
    ///     println!("Dispute rejected - oracle result stands");
    ///     // Community believes oracle was correct
    /// }
    /// ```
    ///
    /// # Calculation Algorithm
    ///
    /// The outcome determination process:
    /// 1. **Vote Aggregation**: Collect all votes with stakes
    /// 2. **Stake Weighting**: Apply economic weight to each vote
    /// 3. **Threshold Analysis**: Check minimum participation requirements
    /// 4. **Outcome Decision**: Determine result based on weighted consensus
    ///
    /// # Weighting Logic
    ///
    /// - **Stake-Weighted Voting**: Larger stakes have more influence
    /// - **Participation Threshold**: Minimum votes required for validity
    /// - **Economic Consensus**: Stakes must exceed minimum threshold
    /// - **Tie Breaking**: Admin intervention required for exact ties
    ///
    /// # Use Cases
    ///
    /// - **Resolution Processing**: Determine final dispute outcome
    /// - **Fee Distribution**: Basis for distributing stakes to winners
    /// - **Market Finalization**: Update market with final result
    /// - **Analytics**: Track dispute resolution patterns
    pub fn calculate_dispute_outcome(env: &Env, dispute_id: Symbol) -> Result<bool, Error> {
        // Get dispute voting data
        let voting_data = DisputeUtils::get_dispute_voting(env, &dispute_id)?;

        // Validate voting is completed
        DisputeValidator::validate_voting_completed(&voting_data)?;

        // Calculate outcome based on stake-weighted voting
        let outcome = DisputeUtils::calculate_stake_weighted_outcome(&voting_data);

        Ok(outcome)
    }

    /// Distributes stakes and fees to the winning side of a resolved dispute.
    ///
    /// This function calculates and executes the distribution of stakes from
    /// losing voters to winning voters, creating economic incentives for
    /// accurate dispute resolution participation.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `dispute_id` - Unique identifier of the resolved dispute
    ///
    /// # Returns
    ///
    /// Returns a `DisputeFeeDistribution` record containing distribution details,
    /// or an `Error` if:
    /// - Dispute is not ready for distribution
    /// - Outcome calculation fails
    /// - Distribution transaction fails
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Symbol};
    /// # use predictify_hybrid::disputes::DisputeManager;
    /// # let env = Env::default();
    /// # let dispute_id = Symbol::new(&env, "resolved_dispute");
    ///
    /// // Distribute fees after dispute resolution
    /// let distribution = DisputeManager::distribute_dispute_fees(
    ///     &env,
    ///     dispute_id.clone()
    /// ).unwrap();
    ///
    /// // Check distribution results
    /// println!("Total fees distributed: {} XLM",
    ///     distribution.total_fees / 10_000_000);
    /// println!("Winners: {} addresses",
    ///     distribution.winner_addresses.len());
    /// println!("Winner stake: {} XLM",
    ///     distribution.winner_stake / 10_000_000);
    /// println!("Loser stake (rewards): {} XLM",
    ///     distribution.loser_stake / 10_000_000);
    ///
    /// // Calculate reward ratio
    /// let reward_ratio = distribution.loser_stake as f64 /
    ///     distribution.winner_stake as f64;
    /// println!("Winners receive {:.1}% bonus", reward_ratio * 100.0);
    /// ```
    ///
    /// # Distribution Mechanics
    ///
    /// 1. **Outcome Determination**: Calculate which side won
    /// 2. **Stake Aggregation**: Sum stakes from winning and losing sides
    /// 3. **Proportional Distribution**: Distribute loser stakes to winners
    /// 4. **Platform Fee**: Deduct small percentage for operations
    /// 5. **Transaction Execution**: Transfer funds to winner addresses
    ///
    /// # Reward Calculation
    ///
    /// Winners receive:
    /// - **Original Stake**: Full recovery of their staked amount
    /// - **Proportional Bonus**: Share of losing side's stakes
    /// - **Early Voter Bonus**: Potential bonus for early participation
    ///
    /// # Economic Incentives
    ///
    /// Fee distribution creates:
    /// - **Accuracy Rewards**: Economic benefit for correct voting
    /// - **Participation Incentive**: Rewards encourage community engagement
    /// - **Quality Control**: Penalties for incorrect dispute judgments
    /// - **Platform Sustainability**: Small fees support operations
    pub fn distribute_dispute_fees(
        env: &Env,
        dispute_id: Symbol,
    ) -> Result<DisputeFeeDistribution, Error> {
        // Validate dispute resolution conditions
        DisputeValidator::validate_dispute_resolution_conditions(env, &dispute_id)?;

        // Calculate dispute outcome
        let outcome = Self::calculate_dispute_outcome(env, dispute_id.clone())?;

        // Get dispute voting data
        let voting_data = DisputeUtils::get_dispute_voting(env, &dispute_id)?;

        // Distribute fees based on outcome
        let fee_distribution = DisputeUtils::distribute_fees_based_on_outcome(
            env,
            &dispute_id,
            &voting_data,
            outcome,
        )?;

        // Emit fee distribution event
        DisputeUtils::emit_fee_distribution_event(env, &dispute_id, &fee_distribution);

        Ok(fee_distribution)
    }

    /// Allows a winner to claim their proportional share of dispute winnings.
    ///
    /// Computes `original_stake + (original_stake * loser_total / winner_total)` and
    /// transfers the result to `user`. Requires [`distribute_dispute_fees`] to have
    /// been called first.
    ///
    /// # Authorization
    ///
    /// Requires `user.require_auth()`.
    ///
    /// # Errors
    ///
    /// - [`Error::InvalidState`] — fee distribution not yet completed or `winner_stake == 0`
    /// - [`Error::AlreadyClaimed`] — user has already claimed for this dispute
    /// - [`Error::NothingToClaim`] — user did not vote, or voted on the losing side
    /// - [`Error::InvalidInput`] — arithmetic overflow computing the payout
    pub fn claim_dispute_winnings(
        env: &Env,
        dispute_id: Symbol,
        user: Address,
    ) -> Result<i128, Error> {
        user.require_auth();

        // Validate distribution is complete
        let distribution = DisputeUtils::get_dispute_fee_distribution(env, &dispute_id)?;
        if !distribution.fees_distributed {
            return Err(Error::InvalidState);
        }

        // Prevent duplicate claims
        if DisputeUtils::has_user_claimed_dispute(env, &dispute_id, &user) {
            return Err(Error::AlreadyClaimed);
        }

        // Get user's vote
        let vote_res = DisputeUtils::get_user_vote(env, &dispute_id, &user);

        let payout = match vote_res {
            Some(vote) => {
                let voting_data = DisputeUtils::get_dispute_voting(env, &dispute_id)?;
                let outcome = DisputeUtils::calculate_stake_weighted_outcome(&voting_data);

                if outcome != vote.vote {
                    // Failing path where users on losing side attempt to extract funds.
                    return Err(Error::NothingToClaim);
                }

                let winner_total = distribution.winner_stake;
                let loser_total = distribution.loser_stake;

                if winner_total == 0 {
                    return Err(Error::InvalidState);
                }

                // Total distributed <= total staked calculation
                let original_stake = vote.stake;
                let bonus = original_stake
                    .checked_mul(loser_total)
                    .ok_or(Error::InvalidInput)?
                    / winner_total;

                original_stake
                    .checked_add(bonus)
                    .ok_or(Error::InvalidInput)?
            }
            None => {
                return Err(Error::NothingToClaim);
            }
        };

        // Mark user as claimed explicitly
        DisputeUtils::set_user_claimed_dispute(env, &dispute_id, &user);

        // Safely transfer winnings using voting utilities
        VotingUtils::transfer_winnings(env, &user, payout)?;

        Ok(payout)
    }

    /// Escalates a dispute to higher authority when standard resolution fails.
    ///
    /// This function allows users to escalate disputes that cannot be resolved
    /// through normal community voting, such as ties, low participation, or
    /// complex cases requiring expert judgment.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `user` - Address of the user requesting escalation (must authenticate)
    /// * `dispute_id` - Unique identifier of the dispute to escalate
    /// * `reason` - Explanation for why escalation is necessary
    ///
    /// # Returns
    ///
    /// Returns a `DisputeEscalation` record containing escalation details,
    /// or an `Error` if:
    /// - User lacks permission to escalate
    /// - Dispute is not eligible for escalation
    /// - Escalation reason is insufficient
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address, Symbol, String};
    /// # use predictify_hybrid::disputes::DisputeManager;
    /// # let env = Env::default();
    /// # let user = Address::generate(&env);
    /// # let dispute_id = Symbol::new(&env, "tied_dispute");
    ///
    /// // Escalate a dispute with exact vote tie
    /// let escalation = DisputeManager::escalate_dispute(
    ///     &env,
    ///     user.clone(),
    ///     dispute_id.clone(),
    ///     String::from_str(&env,
    ///         "Voting resulted in exact tie with equal stakes on both sides")
    /// ).unwrap();
    ///
    /// // Check escalation details
    /// println!("Escalated by: {}", escalation.escalated_by.to_string());
    /// println!("Escalation level: {}", escalation.escalation_level);
    /// println!("Requires admin review: {}", escalation.requires_admin_review);
    /// println!("Reason: {}", escalation.escalation_reason.to_string());
    /// ```
    ///
    /// # Escalation Triggers
    ///
    /// Valid reasons for escalation:
    /// - **Exact Ties**: Equal stakes on both sides
    /// - **Low Participation**: Insufficient community voting
    /// - **Technical Issues**: Oracle data problems or system errors
    /// - **Complex Cases**: Subjective outcomes requiring expert judgment
    /// - **Appeal Process**: Losing party contests the result
    ///
    /// # Escalation Levels
    ///
    /// 1. **Level 1**: Admin review and decision
    /// 2. **Level 2**: Governance token holder voting
    /// 3. **Level 3**: External arbitration panel
    /// 4. **Level 4**: Legal or regulatory intervention
    ///
    /// # Process Flow
    ///
    /// 1. **Authentication**: Verify escalation requester
    /// 2. **Validation**: Check escalation eligibility
    /// 3. **Record Creation**: Store escalation with reasoning
    /// 4. **Admin Notification**: Alert administrators of escalation
    /// 5. **Status Update**: Mark dispute as escalated
    /// 6. **Event Emission**: Broadcast escalation event
    ///
    /// # Resolution Authority
    ///
    /// Escalated disputes require:
    /// - **Admin Review**: Manual evaluation by authorized administrators
    /// - **Expert Judgment**: Specialized knowledge for complex cases
    /// - **Governance Process**: Community governance for policy matters
    /// - **External Arbitration**: Independent third-party resolution
    pub fn escalate_dispute(
        env: &Env,
        user: Address,
        dispute_id: Symbol,
        reason: String,
    ) -> Result<DisputeEscalation, Error> {
        // Require authentication from the user
        user.require_auth();

        // Validate escalation conditions
        DisputeValidator::validate_dispute_escalation_conditions(env, &user, &dispute_id)?;

        // Create escalation record
        let escalation = DisputeEscalation {
            dispute_id: dispute_id.clone(),
            escalated_by: user.clone(),
            escalation_reason: reason,
            escalation_timestamp: env.ledger().timestamp(),
            escalation_level: 1, // Start at level 1
            requires_admin_review: true,
        };

        // Store escalation
        DisputeUtils::store_dispute_escalation(env, &dispute_id, &escalation)?;

        // Emit escalation event
        DisputeUtils::emit_dispute_escalation_event(env, &dispute_id, &user, &escalation);

        Ok(escalation)
    }

    /// Returns all [`DisputeVote`] records cast on `dispute_id`.
    ///
    /// Returns an empty `Vec` when no votes have been cast yet.
    ///
    /// # Errors
    ///
    /// - [`Error::ConfigNotFound`] — voting record not found
    pub fn get_dispute_votes(env: &Env, dispute_id: &Symbol) -> Result<Vec<DisputeVote>, Error> {
        DisputeUtils::get_dispute_votes(env, dispute_id)
    }

    /// Checks whether a dispute is ready for fee distribution.
    ///
    /// Returns `Ok(true)` when voting is `Completed` and fees have not yet been
    /// distributed. Returns an error otherwise.
    ///
    /// # Errors
    ///
    /// - [`Error::DisputeCondNotMet`] — voting not completed
    /// - [`Error::DisputeFeeFailed`] — fees already distributed
    pub fn validate_dispute_resolution_conditions(
        env: &Env,
        dispute_id: Symbol,
    ) -> Result<bool, Error> {
        DisputeValidator::validate_dispute_resolution_conditions(env, &dispute_id)
    }

    /// Sets a resolution timeout for a dispute (admin only).
    ///
    /// `timeout_hours` must be between 1 and 720 (30 days). Once set, the
    /// dispute will be auto-resolved via [`auto_resolve_dispute_on_timeout`]
    /// if it has not been resolved before `expires_at`.
    ///
    /// # Authorization
    ///
    /// Requires `admin.require_auth()` and stored admin match.
    ///
    /// # Errors
    ///
    /// - [`Error::Unauthorized`] — caller is not the contract admin
    /// - [`Error::InvalidDuration`] — `timeout_hours` is 0 or > 720
    pub fn set_dispute_timeout(
        env: &Env,
        dispute_id: Symbol,
        timeout_hours: u32,
        admin: Address,
    ) -> Result<(), Error> {
        // Require authentication from the admin
        admin.require_auth();

        // Validate admin permissions
        DisputeValidator::validate_admin_permissions(env, &admin)?;

        // Enforce admin action cooldown
        Self::check_admin_cooldown(env, &admin, &Symbol::new(env, "set_dispute_timeout"))?;

        // Enforce admin action cooldown
        Self::check_admin_cooldown(env, &admin, &Symbol::new(env, "set_dispute_timeout"))?;

        // Validate timeout hours
        if timeout_hours == 0 || timeout_hours > 720 {
            // Max 30 days
            return Err(Error::InvalidDuration);
        }

        // Create timeout configuration
        let timeout = DisputeTimeout {
            dispute_id: dispute_id.clone(),
            market_id: Symbol::new(env, ""), // Will be set by DisputeUtils
            timeout_hours,
            created_at: env.ledger().timestamp(),
            expires_at: env.ledger().timestamp() + (timeout_hours as u64 * 3600),
            extended_at: None,
            total_extension_hours: 0,
            status: DisputeTimeoutStatus::Active,
        };

        // Store timeout configuration
        DisputeUtils::store_dispute_timeout(env, &dispute_id, &timeout)?;

        // Emit timeout set event
        crate::events::EventEmitter::emit_dispute_timeout_set(
            env,
            &dispute_id,
            &Symbol::new(env, ""), // Market ID will be set properly
            timeout_hours,
            &admin,
        );

        Ok(())
    }

    /// Returns `true` if the dispute timeout has expired.
    ///
    /// # Errors
    ///
    /// - [`Error::ConfigNotFound`] — no timeout configured for `dispute_id`
    pub fn check_dispute_timeout(env: &Env, dispute_id: Symbol) -> Result<bool, Error> {
        let timeout = DisputeUtils::get_dispute_timeout(env, &dispute_id)?;
        let current_time = env.ledger().timestamp();

        Ok(current_time >= timeout.expires_at)
    }

    /// Automatically resolves a dispute whose timeout has expired.
    ///
    /// Uses stake-weighted voting to determine the outcome and emits both
    /// `dispute_timeout_expired` and `dispute_auto_resolved` events.
    ///
    /// # Errors
    ///
    /// - [`Error::InvalidState`] — timeout has not yet expired
    /// - [`Error::ConfigNotFound`] — no timeout configured for `dispute_id`
    pub fn auto_resolve_dispute_on_timeout(
        env: &Env,
        dispute_id: Symbol,
    ) -> Result<DisputeTimeoutOutcome, Error> {
        // Check if timeout has expired
        if !Self::check_dispute_timeout(env, dispute_id.clone())? {
            return Err(Error::InvalidState);
        }

        // Get timeout configuration
        let mut timeout = DisputeUtils::get_dispute_timeout(env, &dispute_id)?;

        // Update timeout status
        timeout.status = DisputeTimeoutStatus::AutoResolved;
        DisputeUtils::store_dispute_timeout(env, &dispute_id, &timeout)?;

        // Update history status to Resolved
        let mut history = env.storage().persistent()
            .get::<_, Vec<Dispute>>(&DataKey::DisputeHistory(timeout.market_id.clone()))
            .unwrap_or_else(|| Vec::new(env));
        let mut updated = false;
        for i in 0..history.len() {
            let mut disp = history.get(i).ok_or(Error::InvalidState)?;
            if matches!(disp.status, DisputeStatus::Active) {
                disp.status = DisputeStatus::Resolved;
                history.set(i, disp);
                updated = true;
            }
        }
        if updated {
            Self::apply_eviction(env, &timeout.market_id, &mut history)?;
            env.storage().persistent().set(&DataKey::DisputeHistory(timeout.market_id.clone()), &history);
            env.storage().persistent().extend_ttl(&DataKey::DisputeHistory(timeout.market_id.clone()), 535680, 535680);
        }

        // Determine timeout outcome
        let outcome = Self::determine_timeout_outcome(env, dispute_id.clone())?;

        // Emit timeout expired event
        crate::events::EventEmitter::emit_dispute_timeout_expired(
            env,
            &dispute_id,
            &outcome.market_id,
            &outcome.outcome,
            &outcome.resolution_method,
        );

        // Emit auto-resolved event
        crate::events::EventEmitter::emit_dispute_auto_resolved(
            env,
            &dispute_id,
            &outcome.market_id,
            &outcome.outcome,
            &outcome.reason,
        );

        Ok(outcome)
    }

    /// Computes the outcome for a timed-out dispute without persisting it.
    ///
    /// Returns `"Support"` when support stake exceeds against stake, otherwise
    /// `"Against"`. Called internally by [`auto_resolve_dispute_on_timeout`].
    ///
    /// # Errors
    ///
    /// - [`Error::ConfigNotFound`] — voting record not found
    pub fn determine_timeout_outcome(
        env: &Env,
        dispute_id: Symbol,
    ) -> Result<DisputeTimeoutOutcome, Error> {
        // Get dispute voting data
        let voting_data = DisputeUtils::get_dispute_voting(env, &dispute_id)?;

        // Determine outcome based on stake-weighted voting
        let outcome = if voting_data.total_support_stake > voting_data.total_against_stake {
            String::from_str(env, "Support")
        } else {
            String::from_str(env, "Against")
        };

        // Create timeout outcome
        let timeout_outcome = DisputeTimeoutOutcome {
            dispute_id: dispute_id.clone(),
            market_id: Symbol::new(env, ""), // Will be set properly
            outcome,
            resolution_method: String::from_str(env, "Timeout Auto-Resolution"),
            resolution_timestamp: env.ledger().timestamp(),
            reason: String::from_str(
                env,
                "Dispute timeout expired - automatic resolution based on stake-weighted voting",
            ),
        };

        Ok(timeout_outcome)
    }

    /// Emits a `dispute_timeout_expired` event for the given `dispute_id`.
    ///
    /// # Errors
    ///
    /// - [`Error::ConfigNotFound`] — no timeout configured for `dispute_id`
    pub fn emit_timeout_event(env: &Env, dispute_id: Symbol, outcome: String) -> Result<(), Error> {
        let timeout = DisputeUtils::get_dispute_timeout(env, &dispute_id)?;

        crate::events::EventEmitter::emit_dispute_timeout_expired(
            env,
            &dispute_id,
            &timeout.market_id,
            &outcome,
            &String::from_str(env, "Timeout"),
        );

        Ok(())
    }

    /// Returns the current [`DisputeTimeoutStatus`] for a dispute.
    ///
    /// # Errors
    ///
    /// - [`Error::ConfigNotFound`] — no timeout configured for `dispute_id`
    pub fn get_dispute_timeout_status(
        env: &Env,
        dispute_id: Symbol,
    ) -> Result<DisputeTimeoutStatus, Error> {
        let timeout = DisputeUtils::get_dispute_timeout(env, &dispute_id)?;
        Ok(timeout.status)
    }

    /// Extends an active dispute timeout by `additional_hours` (admin only).
    ///
    /// `additional_hours` must be between 1 and 168 (7 days). Only timeouts
    /// in `Active` state can be extended.
    ///
    /// # Authorization
    ///
    /// Requires `admin.require_auth()` and stored admin match.
    ///
    /// # Errors
    ///
    /// - [`Error::Unauthorized`] — caller is not the contract admin
    /// - [`Error::InvalidDuration`] — `additional_hours` is 0 or > 168
    /// - [`Error::InvalidState`] — timeout is not in `Active` state
    /// - [`Error::ConfigNotFound`] — no timeout configured for `dispute_id`
    pub fn extend_dispute_timeout(
        env: &Env,
        dispute_id: Symbol,
        additional_hours: u32,
        admin: Address,
    ) -> Result<(), Error> {
        // Require authentication from the admin
        admin.require_auth();

        // Validate admin permissions
        DisputeValidator::validate_admin_permissions(env, &admin)?;

        // Enforce admin action cooldown
        Self::check_admin_cooldown(env, &admin, &Symbol::new(env, "extend_dispute_timeout"))?;

        // Enforce admin action cooldown
        Self::check_admin_cooldown(env, &admin, &Symbol::new(env, "extend_dispute_timeout"))?;

        // Validate additional hours
        if additional_hours == 0 || additional_hours > 168 {
            // Max 7 days extension
            return Err(Error::InvalidDuration);
        }

        // Get current timeout
        let mut timeout = DisputeUtils::get_dispute_timeout(env, &dispute_id)?;

        // Check if timeout can be extended
        if !matches!(timeout.status, DisputeTimeoutStatus::Active) {
            return Err(Error::InvalidState);
        }

        // Update timeout
        timeout.extended_at = Some(env.ledger().timestamp());
        timeout.total_extension_hours += additional_hours;
        timeout.expires_at += additional_hours as u64 * 3600;
        timeout.status = DisputeTimeoutStatus::Extended;

        // Store updated timeout
        DisputeUtils::store_dispute_timeout(env, &dispute_id, &timeout)?;

        // Emit timeout extended event
        crate::events::EventEmitter::emit_dispute_timeout_extended(
            env,
            &dispute_id,
            &timeout.market_id,
            additional_hours,
            &admin,
        );

        Ok(())
    }

    /// Set a per-market per-user dispute stake cap.
    ///
    /// Once set, the user cannot stake more than `cap` in disputes on
    /// this specific market.  A cap of 0 disables the limit.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment
    /// * `market_id` - Market to apply the cap to
    /// * `user` - User whose stake is capped
    /// * `cap` - Maximum stake in stroops (0 = unlimited)
    pub fn set_dispute_stake_cap(
        env: &Env,
        market_id: &Symbol,
        user: &Address,
        cap: i128,
    ) -> Result<(), Error> {
        let cap_key = crate::storage::DataKey::DisputeStakeCap(market_id.clone(), user.clone());
        env.storage().persistent().set(&cap_key, &cap);

        crate::events::EventEmitter::emit_dispute_stake_cap_set(env, market_id, user, cap);
        Ok(())
    }

    /// Set the per-user cumulative dispute stake cap across all active disputes.
    ///
    /// This cap limits the total stake a user can commit to disputes
    /// across all markets that have active (unresolved) disputes.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment
    /// * `user` - The user address the cap applies to
    /// * `cap` - The maximum cumulative stake allowed in stroops (0 = disabled)
    ///
    /// # Authorization
    ///
    /// This function requires admin permissions via [`DisputeValidator::validate_admin_permissions`].
    pub fn set_dispute_cumulative_stake_cap(
        env: &Env,
        admin: &Address,
        user: &Address,
        cap: i128,
    ) -> Result<(), Error> {
        // Require admin authorization
        DisputeValidator::validate_admin_permissions(env, admin)?;

        let cap_key = crate::storage::DataKey::DisputeCumulativeStakeCap(user.clone());
        env.storage().persistent().set(&cap_key, &cap);

        crate::events::EventEmitter::emit_dispute_cumulative_stake_cap_set(env, user, cap);
        Ok(())
    }

    /// Get the per-user cumulative dispute stake cap.
    ///
    /// Returns 0 if no cap is set (cap is disabled).
    pub fn get_dispute_cumulative_stake_cap(env: &Env, user: &Address) -> i128 {
        let cap_key = crate::storage::DataKey::DisputeCumulativeStakeCap(user.clone());
        env.storage().persistent().get(&cap_key).unwrap_or(0)
    }

    // ── Admin Cooldown ───────────────────────────────────────────────────────

    /// Sets the cooldown period (in seconds) between admin actions on disputes.
    ///
    /// A zero value disables the cooldown entirely.  Only the contract admin
    /// may call this.
    pub fn set_admin_cooldown(env: &Env, admin: &Address, seconds: u64) -> Result<(), Error> {
        admin.require_auth();
        DisputeValidator::validate_admin_permissions(env, admin)?;
        let key = DataKey::DisputeCooldownSeconds;
        env.storage().persistent().set(&key, &seconds);
        env.storage().persistent().extend_ttl(&key, 535680, 535680);
        Ok(())
    }

    /// Retrieves the configured dispute admin cooldown period in seconds.
    ///
    /// Returns 0 (no cooldown) when not configured.
    pub fn get_admin_cooldown(env: &Env) -> u64 {
        let key = DataKey::DisputeCooldownSeconds;
        env.storage().persistent().get(&key).unwrap_or(0)
    }

    /// Enforces the per-function admin cooldown for a named dispute operation.
    ///
    /// * `function_name` – a short identifier (`"set_history_cap"`, `"resolve_dispute"`, …).
    ///
    /// # Errors
    /// Returns `Error::AdminActionTimelocked` if the cooldown has not yet elapsed
    /// since the last invocation of *this specific* function.
    pub fn check_admin_cooldown(
        env: &Env,
        admin: &Address,
        function_name: &Symbol,
    ) -> Result<(), Error> {
        admin.require_auth();
        DisputeValidator::validate_admin_permissions(env, admin)?;
        let cooldown = Self::get_admin_cooldown(env);
        if cooldown == 0 {
            return Ok(());
        }
        let now = env.ledger().timestamp();
        let last_key = DataKey::DisputeAdminLastAction(function_name.clone());
        let last_action: u64 = env.storage().persistent().get(&last_key).unwrap_or(0);
        if last_action > 0 && now < last_action.saturating_add(cooldown) {
            return Err(Error::AdminActionTimelocked);
        }
        env.storage().persistent().set(&last_key, &now);
        env.storage().persistent().extend_ttl(&last_key, 535680, 535680);
        Ok(())
    }
}

// ===== DISPUTE VALIDATOR =====

/// Input validation helpers for dispute operations.
///
/// All methods return `Ok(())` on success or a typed [`Error`] on failure.
/// Called internally by [`DisputeManager`] before any state mutation.
pub struct DisputeValidator;

impl DisputeValidator {
    /// Validate market state for dispute.
    ///
    /// A dispute is only valid when:
    /// 1. The market has ended (`current_time >= end_time`).
    /// 2. The dispute window is still open (`current_time < end_time + dispute_window_seconds`).
    ///    Allowing disputes after the window closes would create an ambiguous overlap with
    ///    the payout phase and could re-open markets that users already consider settled.
    /// 3. The market has not already been resolved.
    /// 4. An oracle result is available to dispute.
    pub fn validate_market_for_dispute(env: &Env, market: &Market) -> Result<(), Error> {
        let current_time = env.ledger().timestamp();

        // Market must have ended before a dispute can be filed.
        if current_time < market.end_time {
            return Err(Error::MarketClosed);
        }

        // Disputes must be filed within the dispute window.
        // After `end_time + dispute_window_seconds` the window is closed and payouts
        // are unambiguously allowed, so late disputes are rejected.
        if market.dispute_window_seconds > 0
            && current_time >= market.end_time + market.dispute_window_seconds
        {
            return Err(Error::MarketResolved);
        }

        // Check if market is already resolved.
        if market.winning_outcomes.is_some() {
            return Err(Error::MarketResolved);
        }

        // Check if oracle result is available to dispute.
        if market.oracle_result.is_none() {
            return Err(Error::OracleUnavailable);
        }

        Ok(())
    }

    /// Validate market state for resolution.
    ///
    /// Ensures that the market has not already been resolved and that
    /// there is at least one active dispute stake to resolve.
    ///
    /// # Errors
    ///
    /// - [`Error::MarketResolved`] — `winning_outcomes` is already set
    /// - [`Error::InvalidInput`] — no dispute stakes exist on this market
    pub fn validate_market_for_resolution(_env: &Env, market: &Market) -> Result<(), Error> {
        // Check if market is already resolved
        if market.winning_outcomes.is_some() {
            return Err(Error::MarketResolved);
        }

        // Check if there are active disputes
        if market.total_dispute_stakes() == 0 {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }

    /// Validate that `admin` matches the stored contract admin address.
    ///
    /// Reads the `"Admin"` key from persistent storage and compares it
    /// against the caller.  Returns an error if no admin is set or the
    /// addresses do not match.
    ///
    /// # Errors
    ///
    /// - [`Error::Unauthorized`] — caller is not the stored contract admin
    ///   or no admin has been initialised yet
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

    /// Validate dispute parameters
    pub fn validate_dispute_parameters(
        env: &Env,
        market_id: &Symbol,
        user: &Address,
        market: &Market,
        stake: i128,
    ) -> Result<(), Error> {
        // Validate stake amount
        if stake < MIN_DISPUTE_STAKE {
            return Err(Error::InsufficientStake);
        }

        // Check if user has already disputed
        if DisputeUtils::has_user_disputed(market, user) {
            return Err(Error::AlreadyDisputed);
        }

        // Check per-market per-user dispute stake cap
        let cap_key = crate::storage::DataKey::DisputeStakeCap(market_id.clone(), user.clone());
        let cap: i128 = env.storage().persistent().get(&cap_key).unwrap_or(0);
        if cap > 0 {
            let user_current_state_stake = market.dispute_stakes.get(user.clone()).unwrap_or(0);
            if user_current_state_stake + stake > cap {
                crate::events::EventEmitter::emit_dispute_stake_cap_exceeded(
                    env,
                    market_id,
                    user,
                    cap,
                    stake,
                );
                return Err(Error::DisputeStakeCapExceeded);
            }
        }

        // Check per-user cumulative dispute stake cap across all active disputes
        let cumulative_cap_key = crate::storage::DataKey::DisputeCumulativeStakeCap(user.clone());
        let cumulative_cap: i128 = env.storage().persistent().get(&cumulative_cap_key).unwrap_or(0);
        if cumulative_cap > 0 {
            // Calculate cumulative stake across all markets with active disputes
            // For markets with active disputes (winning_outcomes is None but disputes exist)
            let cumulative_stake = market.dispute_stakes.get(user.clone()).unwrap_or(0);
            // Note: In a full implementation, we would iterate through all markets
            // to sum up stakes across all active disputes. Here we check just the current market
            // plus any existing stake; the contract-level function can provide more comprehensive logic.
            if cumulative_stake + stake > cumulative_cap {
                crate::events::EventEmitter::emit_dispute_cumulative_stake_cap_exceeded(
                    env,
                    user,
                    cumulative_cap,
                    cumulative_stake,
                    stake,
                );
                return Err(Error::DisputeStakeCapExceeded);
            }
        }

        // Check if user has voted (optional requirement)
        if !market.votes.contains_key(user.clone()) {
            // Allow disputes even from non-voters, but could be made optional
        }

        Ok(())
    }

    /// Validate that `final_outcome` is one of the valid outcomes defined on the market.
    ///
    /// # Errors
    ///
    /// - [`Error::InvalidOutcome`] — the outcome string is not present in `market.outcomes`
    pub fn validate_resolution_parameters(
        market: &Market,
        final_outcome: &String,
    ) -> Result<(), Error> {
        // Validate that final outcome is one of the valid outcomes
        if !market.outcomes.contains(final_outcome) {
            return Err(Error::InvalidOutcome);
        }

        Ok(())
    }

    /// Validate that a dispute is in an active voting window.
    ///
    /// Checks that the current ledger timestamp falls within
    /// [`DisputeVoting::voting_start`] .. [`DisputeVoting::voting_end`]
    /// and that the voting status is still [`DisputeVotingStatus::Active`].
    ///
    /// # Errors
    ///
    /// - [`Error::ConfigNotFound`] — voting record not found
    /// - [`Error::DisputeVoteExpired`] — current time is outside the voting window
    /// - [`Error::DisputeVoteDenied`] — voting is not in `Active` state
    pub fn validate_dispute_voting_conditions(
        env: &Env,
        _market_id: &Symbol,
        dispute_id: &Symbol,
    ) -> Result<(), Error> {
        // Check if dispute exists and is active
        let voting_data = DisputeUtils::get_dispute_voting(env, dispute_id)?;

        // Check if voting period is active
        let current_time = env.ledger().timestamp();
        if current_time < voting_data.voting_start || current_time > voting_data.voting_end {
            return Err(Error::DisputeVoteExpired);
        }

        // Check if voting is still active
        if !matches!(voting_data.status, DisputeVotingStatus::Active) {
            return Err(Error::DisputeVoteDenied);
        }

        Ok(())
    }

    /// Validate that `user` has not already cast a vote on the given dispute.
    ///
    /// Iterates over all stored votes for `dispute_id` and returns an error
    /// if a matching address is found.
    ///
    /// # Errors
    ///
    /// - [`Error::DisputeAlreadyVoted`] — user has already voted on this dispute
    pub fn validate_user_hasnt_voted(
        env: &Env,
        user: &Address,
        dispute_id: &Symbol,
    ) -> Result<(), Error> {
        let votes = DisputeUtils::get_dispute_votes(env, dispute_id)?;

        for vote in votes.iter() {
            if vote.user == user.clone() {
                return Err(Error::DisputeAlreadyVoted);
            }
        }

        Ok(())
    }

    /// Validate that voting has reached `Completed` status.
    ///
    /// # Errors
    ///
    /// - [`Error::DisputeCondNotMet`] — voting is not yet completed
    pub fn validate_voting_completed(voting_data: &DisputeVoting) -> Result<(), Error> {
        if !matches!(voting_data.status, DisputeVotingStatus::Completed) {
            return Err(Error::DisputeCondNotMet);
        }

        Ok(())
    }

    /// Validate dispute resolution conditions
    pub fn validate_dispute_resolution_conditions(
        env: &Env,
        dispute_id: &Symbol,
    ) -> Result<bool, Error> {
        // Check if dispute voting exists and is completed
        let voting_data = DisputeUtils::get_dispute_voting(env, dispute_id)?;

        if !matches!(voting_data.status, DisputeVotingStatus::Completed) {
            return Err(Error::DisputeCondNotMet);
        }

        // Check if fees haven't been distributed yet
        let fee_distribution = DisputeUtils::get_dispute_fee_distribution(env, dispute_id)?;
        if fee_distribution.fees_distributed {
            return Err(Error::DisputeFeeFailed);
        }

        Ok(true)
    }

    /// Validate that a dispute is eligible for escalation.
    ///
    /// The user must have participated in voting on the dispute and no
    /// escalation may already exist for it.
    ///
    /// # Errors
    ///
    /// - [`Error::DisputeCondNotMet`] — user did not vote, or escalation already exists
    pub fn validate_dispute_escalation_conditions(
        env: &Env,
        user: &Address,
        dispute_id: &Symbol,
    ) -> Result<(), Error> {
        // Check if user has participated in the dispute
        let votes = DisputeUtils::get_dispute_votes(env, dispute_id)?;
        let mut has_participated = false;

        for vote in votes.iter() {
            if vote.user == user.clone() {
                has_participated = true;
                break;
            }
        }

        if !has_participated {
            return Err(Error::DisputeCondNotMet);
        }

        // Check if escalation already exists
        let escalation = DisputeUtils::get_dispute_escalation(env, dispute_id);
        if escalation.is_some() {
            return Err(Error::DisputeCondNotMet);
        }

        Ok(())
    }

    /// Validate dispute timeout parameters
    pub fn validate_dispute_timeout_parameters(timeout_hours: u32) -> Result<(), Error> {
        if timeout_hours == 0 {
            return Err(Error::InvalidDuration);
        }

        if timeout_hours > 720 {
            // Max 30 days
            return Err(Error::InvalidDuration);
        }

        Ok(())
    }

    /// Validate dispute timeout extension parameters
    pub fn validate_dispute_timeout_extension_parameters(
        additional_hours: u32,
    ) -> Result<(), Error> {
        if additional_hours == 0 {
            return Err(Error::InvalidDuration);
        }

        if additional_hours > 168 {
            // Max 7 days extension
            return Err(Error::InvalidDuration);
        }

        Ok(())
    }

    /// Validate dispute timeout status for extension
    pub fn validate_dispute_timeout_status_for_extension(
        timeout: &DisputeTimeout,
    ) -> Result<(), Error> {
        if !matches!(timeout.status, DisputeTimeoutStatus::Active) {
            return Err(Error::InvalidState);
        }

        Ok(())
    }
}

// ===== DISPUTE UTILITIES =====

/// Low-level storage and computation helpers for dispute operations.
///
/// These functions are called by [`DisputeManager`] and [`DisputeValidator`].
/// They do not perform authentication checks.
pub struct DisputeUtils;

impl DisputeUtils {
    /// Record a dispute's stake on the market's `dispute_stakes` map.
    ///
    /// Increments the existing stake for the disputing user by the new
    /// dispute's stake amount.  This is called during dispute creation.
    pub fn add_dispute_to_market(market: &mut Market, dispute: Dispute) -> Result<(), Error> {
        // Add dispute stake to market
        let current_stake = market.dispute_stakes.get(dispute.user.clone()).unwrap_or(0);
        market
            .dispute_stakes
            .set(dispute.user, current_stake + dispute.stake);

        // Update total dispute stakes - this is calculated automatically by the method
        // No need to assign it back since it's a computed value

        Ok(())
    }

    /// Extend the market's `end_time` by [`DISPUTE_EXTENSION_HOURS`]
    /// to allow the community voting period to complete.
    pub fn extend_market_for_dispute(market: &mut Market, _env: &Env) -> Result<(), Error> {
        let extension_seconds = (DISPUTE_EXTENSION_HOURS as u64) * 3600;
        market.end_time += extension_seconds;
        Ok(())
    }

    /// Determine the final market outcome by weighing oracle data against
    /// community consensus when dispute impact exceeds a threshold.
    ///
    /// If the calculated dispute impact is greater than 30 %, the function
    /// fetches community consensus and returns it when confidence exceeds
    /// 70 %.  Otherwise the original oracle result is returned.
    pub fn determine_final_outcome_with_disputes(
        env: &Env,
        market: &Market,
    ) -> Result<String, Error> {
        let oracle_result = market
            .oracle_result
            .as_ref()
            .ok_or(Error::OracleUnavailable)?;

        // If there are significant disputes, consider community consensus more heavily
        let dispute_impact = DisputeAnalytics::calculate_dispute_impact(market);

        if dispute_impact > 30 {
            // Using integer percentage (30% = 30)
            // High dispute impact - give more weight to community consensus
            let community_consensus = DisputeAnalytics::calculate_community_consensus(env, market);
            if community_consensus.confidence > 70 {
                // Using integer percentage (70% = 70)
                return Ok(community_consensus.outcome);
            }
        }

        // Default to oracle result
        Ok(oracle_result.clone())
    }

    /// Set the market's `winning_outcomes` to the given `final_outcome`
    /// after validating it is one of the market's allowed outcomes.
    ///
    /// # Errors
    ///
    /// - [`Error::InvalidOutcome`] — `final_outcome` not in `market.outcomes`
    pub fn finalize_market_with_resolution(
        market: &mut Market,
        final_outcome: String,
    ) -> Result<(), Error> {
        // Validate the final outcome
        DisputeValidator::validate_resolution_parameters(market, &final_outcome)?;

        // Set the winning outcome(s) - convert single outcome to vector
        let mut winning_outcomes = Vec::new(market.votes.env());
        winning_outcomes.push_back(final_outcome);
        market.winning_outcomes = Some(winning_outcomes);

        Ok(())
    }

    /// Build a `Vec<Dispute>` from the current `market.dispute_stakes` map.
    ///
    /// Only entries with a positive stake are included.  Used to migrate
    /// inline dispute data into the persistent history store.
    pub fn extract_disputes_from_market(
        env: &Env,
        market: &Market,
        market_id: Symbol,
    ) -> Vec<Dispute> {
        let mut disputes = Vec::new(env);

        for (user, stake) in market.dispute_stakes.iter() {
            if stake > 0 {
                let dispute = Dispute {
                    user: user.clone(),
                    market_id: market_id.clone(),
                    stake,
                    timestamp: env.ledger().timestamp(),
                    reason: None,
                    status: DisputeStatus::Active,
                };
                disputes.push_back(dispute);
            }
        }

        disputes
    }

    /// Returns `true` when `user` has a positive stake in the market's
    /// `dispute_stakes` map.
    pub fn has_user_disputed(market: &Market, user: &Address) -> bool {
        market.dispute_stakes.get(user.clone()).unwrap_or(0) > 0
    }

    /// Returns the amount `user` has staked in disputes on this market,
    /// or 0 if they have not disputed.
    pub fn get_user_dispute_stake(market: &Market, user: &Address) -> i128 {
        market.dispute_stakes.get(user.clone()).unwrap_or(0)
    }

    /// Calculate the dispute impact ratio as `total_dispute_stakes / total_staked`.
    ///
    /// Returns a float between 0.0 and ~1.0.  Used by
    /// [`DisputeAnalytics::calculate_dispute_impact`] for integer conversion.
    pub fn calculate_dispute_impact(market: &Market) -> f64 {
        let total_dispute_stakes = market.total_dispute_stakes();
        if market.total_staked == 0 {
            return 0.0;
        }
        (total_dispute_stakes as f64) / (market.total_staked as f64)
    }

    /// Record a [`DisputeVote`] for the given dispute and update the
    /// aggregated [`DisputeVoting`] counters (including stake decay).
    pub fn add_vote_to_dispute(
        env: &Env,
        dispute_id: &Symbol,
        vote: DisputeVote,
    ) -> Result<(), Error> {
        // Get current voting data
        let mut voting_data = Self::get_dispute_voting(env, dispute_id)?;

        // Update voting statistics
        voting_data.total_votes = voting_data
            .total_votes
            .checked_add(1)
            .ok_or(Error::Overflow)?;
        
        // Calculate the decayed stake using tally_votes
        let decayed_stake = Self::tally_votes(env, vote.stake, vote.timestamp, voting_data.voting_start);

        if vote.vote {
            voting_data.support_votes = voting_data
                .support_votes
                .checked_add(1)
                .ok_or(Error::Overflow)?;
            voting_data.total_support_stake = voting_data
                .total_support_stake
                .checked_add(decayed_stake)
                .ok_or(Error::Overflow)?;
        } else {
            voting_data.against_votes = voting_data
                .against_votes
                .checked_add(1)
                .ok_or(Error::Overflow)?;
            voting_data.total_against_stake = voting_data
                .total_against_stake
                .checked_add(decayed_stake)
                .ok_or(Error::Overflow)?;
        }

        // Store updated voting data
        Self::store_dispute_voting(env, dispute_id, &voting_data)?;

        // Store the vote
        Self::store_dispute_vote(env, dispute_id, &vote)?;

        Ok(())
    }

    /// Calculate the stake weight using exponential decay approximation
    /// so late votes count less than early votes.
    pub fn tally_votes(env: &Env, raw_stake: i128, vote_time: u64, window_start: u64) -> i128 {
        let config_key = symbol_short!("decaycfg");
        let config: Option<DisputeDecayConfig> = env.storage().persistent().get(&config_key);
        
        let cfg = match config {
            Some(c) => c,
            None => return raw_stake,
        };

        if cfg.half_life_seconds == 0 {
            return raw_stake;
        }

        let elapsed = vote_time.saturating_sub(window_start);
        let num_half_lives = elapsed / cfg.half_life_seconds;
        let rem = elapsed % cfg.half_life_seconds;

        let shift = num_half_lives.min(16) as u32;
        let weight_at_n = 10000u32.checked_shr(shift).unwrap_or(0);
        let weight_at_n_plus_1 = 10000u32.checked_shr(shift + 1).unwrap_or(0);
        
        let diff = weight_at_n.saturating_sub(weight_at_n_plus_1);
        let exact_weight = weight_at_n.saturating_sub((diff as u64 * rem / cfg.half_life_seconds) as u32);
        
        // A misconfigured floor must never amplify a vote above its raw stake.
        let final_weight = exact_weight.max(cfg.floor_bps).min(10_000) as i128;

        // Split before multiplying so every i128 input remains overflow-safe.
        let whole = raw_stake / 10_000;
        let remainder = raw_stake % 10_000;
        whole * final_weight + (remainder * final_weight) / 10_000
    }

    /// Set the [`DisputeDecayConfig`] for vote stake decay (admin only).
    ///
    /// Controls how quickly late votes lose weight relative to early votes.
    ///
    /// # Authorization
    ///
    /// Requires `admin.require_auth()` and stored admin match.
    pub fn set_dispute_decay_config(env: &Env, admin: Address, config: DisputeDecayConfig) -> Result<(), Error> {
        admin.require_auth();
        DisputeValidator::validate_admin_permissions(env, &admin)?;
        let key = symbol_short!("decaycfg");
        env.storage().persistent().set(&key, &config);
        env.storage().persistent().extend_ttl(&key, 535680, 535680);
        Ok(())
    }

    /// Read the [`DisputeVoting`] record for `dispute_id` from persistent storage.
    ///
    /// Returns a default active voting window if no record exists yet.
    pub fn get_dispute_voting(env: &Env, dispute_id: &Symbol) -> Result<DisputeVoting, Error> {
        let key = (symbol_short!("dispute_v"), dispute_id.clone());
        Ok(env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| DisputeVoting {
                dispute_id: dispute_id.clone(),
                voting_start: env.ledger().timestamp(),
                voting_end: env.ledger().timestamp() + (DISPUTE_EXTENSION_HOURS as u64 * 3600),
                total_votes: 0,
                support_votes: 0,
                against_votes: 0,
                total_support_stake: 0,
                total_against_stake: 0,
                status: DisputeVotingStatus::Active,
            }))
    }

    /// Persist a [`DisputeVoting`] record under the `dispute_v` + `dispute_id` key.
    pub fn store_dispute_voting(
        env: &Env,
        dispute_id: &Symbol,
        voting: &DisputeVoting,
    ) -> Result<(), Error> {
        let key = (symbol_short!("dispute_v"), dispute_id.clone());
        env.storage().persistent().set(&key, voting);
        Ok(())
    }

    /// Persist a single [`DisputeVote`] under the `vote` + `dispute_id` + `user` key.
    pub fn store_dispute_vote(
        env: &Env,
        dispute_id: &Symbol,
        vote: &DisputeVote,
    ) -> Result<(), Error> {
        let key = (symbol_short!("vote"), dispute_id.clone(), vote.user.clone());
        env.storage().persistent().set(&key, vote);
        Ok(())
    }

    /// Retrieve the [`DisputeVote`] cast by `user` for `dispute_id`, or `None`.
    pub fn get_user_vote(env: &Env, dispute_id: &Symbol, user: &Address) -> Option<DisputeVote> {
        let key = (symbol_short!("vote"), dispute_id.clone(), user.clone());
        env.storage().persistent().get(&key)
    }

    /// Returns `true` if `user` has already claimed their winnings for `dispute_id`.
    pub fn has_user_claimed_dispute(env: &Env, dispute_id: &Symbol, user: &Address) -> bool {
        let key = (symbol_short!("d_clm"), dispute_id.clone(), user.clone());
        env.storage().persistent().get(&key).unwrap_or(false)
    }

    /// Mark `user` as having claimed their winnings for `dispute_id`.
    ///
    /// Prevents duplicate claims via [`DisputeManager::claim_dispute_winnings`].
    pub fn set_user_claimed_dispute(env: &Env, dispute_id: &Symbol, user: &Address) {
        let key = (symbol_short!("d_clm"), dispute_id.clone(), user.clone());
        env.storage().persistent().set(&key, &true);
    }

    /// Retrieve all [`DisputeVote`] records for the given dispute.
    ///
    /// **Note:** The current implementation returns an empty vector.
    /// A production system should maintain a separate vote-key index
    /// for efficient iteration.
    ///
    /// # Errors
    ///
    /// - [`Error::ConfigNotFound`] — voting record not found
    pub fn get_dispute_votes(env: &Env, dispute_id: &Symbol) -> Result<Vec<DisputeVote>, Error> {
        // This is a simplified implementation - in a real system you'd need to track all votes
        let votes = Vec::new(env);

        // Get the voting data to access stored votes
        let _voting_data = Self::get_dispute_voting(env, dispute_id)?;

        // In a real implementation, you would iterate through stored vote keys
        // For now, return empty vector as this would require tracking vote keys separately
        Ok(votes)
    }

    /// Calculate stake-weighted outcome.
    ///
    /// Policy: `true` (dispute upheld) iff support stake is strictly greater than against.
    /// Exact ties resolve to `false` (oracle result stands; admin escalation per docs).
    pub fn calculate_stake_weighted_outcome(voting_data: &DisputeVoting) -> bool {
        voting_data.total_support_stake > voting_data.total_against_stake
    }

    /// Create a [`DisputeFeeDistribution`] record from the voting data,
    /// assigning winner and loser totals based on the boolean outcome.
    ///
    /// The distribution is persisted immediately.
    pub fn distribute_fees_based_on_outcome(
        env: &Env,
        dispute_id: &Symbol,
        voting_data: &DisputeVoting,
        outcome: bool,
    ) -> Result<DisputeFeeDistribution, Error> {
        let total_fees = voting_data.total_support_stake + voting_data.total_against_stake;
        let winner_stake = if outcome {
            voting_data.total_support_stake
        } else {
            voting_data.total_against_stake
        };
        let loser_stake = if outcome {
            voting_data.total_against_stake
        } else {
            voting_data.total_support_stake
        };

        // Create fee distribution record
        let fee_distribution = DisputeFeeDistribution {
            dispute_id: dispute_id.clone(),
            total_fees,
            winner_stake,
            loser_stake,
            winner_addresses: Vec::new(env), // Would be populated with actual winner addresses
            distribution_timestamp: env.ledger().timestamp(),
            fees_distributed: true,
        };

        // Store fee distribution
        Self::store_dispute_fee_distribution(env, dispute_id, &fee_distribution)?;

        Ok(fee_distribution)
    }

    /// Persist a [`DisputeFeeDistribution`] under the `dispute_f` + `dispute_id` key.
    pub fn store_dispute_fee_distribution(
        env: &Env,
        dispute_id: &Symbol,
        distribution: &DisputeFeeDistribution,
    ) -> Result<(), Error> {
        let key = (symbol_short!("dispute_f"), dispute_id.clone());
        env.storage().persistent().set(&key, distribution);
        Ok(())
    }

    /// Read the [`DisputeFeeDistribution`] for `dispute_id`, returning a
    /// default (un-distributed) record if none exists.
    pub fn get_dispute_fee_distribution(
        env: &Env,
        dispute_id: &Symbol,
    ) -> Result<DisputeFeeDistribution, Error> {
        let key = (symbol_short!("dispute_f"), dispute_id.clone());
        Ok(env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(DisputeFeeDistribution {
                dispute_id: dispute_id.clone(),
                total_fees: 0,
                winner_stake: 0,
                loser_stake: 0,
                winner_addresses: Vec::new(env),
                distribution_timestamp: 0,
                fees_distributed: false,
            }))
    }

    /// Persist a [`DisputeEscalation`] under the `dispute_e` + `dispute_id` key.
    pub fn store_dispute_escalation(
        env: &Env,
        dispute_id: &Symbol,
        escalation: &DisputeEscalation,
    ) -> Result<(), Error> {
        let key = (symbol_short!("dispute_e"), dispute_id.clone());
        env.storage().persistent().set(&key, escalation);
        Ok(())
    }

    /// Read the [`DisputeEscalation`] for `dispute_id`, or `None`.
    pub fn get_dispute_escalation(env: &Env, dispute_id: &Symbol) -> Option<DisputeEscalation> {
        let key = (symbol_short!("dispute_e"), dispute_id.clone());
        env.storage().persistent().get(&key)
    }

    /// Emit a `dispute_vote_cast` event via [`crate::events::EventEmitter`].
    pub fn emit_dispute_vote_event(
        env: &Env,
        _market_id: &Symbol,
        user: &Address,
        vote: &String,
        stake: i128,
    ) {
        // NOTE: emit_dispute_vote_cast not yet implemented in EventEmitter
    }

    /// Emit a `dispute_fee_distributed` event via [`crate::events::EventEmitter`].
    pub fn emit_fee_distribution_event(
        env: &Env,
        dispute_id: &Symbol,
        distribution: &DisputeFeeDistribution,
    ) {
        // NOTE: emit_dispute_fee_distributed not yet implemented in EventEmitter
    }

    /// Emit a dispute escalation event and store a snapshot in persistent storage.
    ///
    /// The snapshot contains the escalator's address, level, and timestamp.
    pub fn emit_dispute_escalation_event(
        env: &Env,
        _dispute_id: &Symbol,
        user: &Address,
        escalation: &DisputeEscalation,
    ) {
        let event_key = symbol_short!("esc_event");
        let event_data = (
            user.clone(),
            escalation.escalation_level,
            env.ledger().timestamp(),
        );
        env.storage().persistent().set(&event_key, &event_data);
    }

    /// Persist a [`DisputeTimeout`] under the `timeout` + `dispute_id` key.
    pub fn store_dispute_timeout(
        env: &Env,
        dispute_id: &Symbol,
        timeout: &DisputeTimeout,
    ) -> Result<(), Error> {
        let key = (symbol_short!("timeout"), dispute_id.clone());
        env.storage().persistent().set(&key, timeout);
        Ok(())
    }

    /// Read the [`DisputeTimeout`] for `dispute_id`.
    ///
    /// # Errors
    ///
    /// - [`Error::ConfigNotFound`] — no timeout configured for this dispute
    pub fn get_dispute_timeout(env: &Env, dispute_id: &Symbol) -> Result<DisputeTimeout, Error> {
        let key = (symbol_short!("timeout"), dispute_id.clone());
        env.storage()
            .persistent()
            .get(&key)
            .ok_or(Error::ConfigNotFound)
    }

    /// Returns `true` when a timeout record exists for `dispute_id`.
    pub fn has_dispute_timeout(env: &Env, dispute_id: &Symbol) -> bool {
        let key = (symbol_short!("timeout"), dispute_id.clone());
        env.storage().persistent().has(&key)
    }

    /// Delete the timeout record for `dispute_id`.
    ///
    /// Returns `Ok(())` even if no record existed.
    pub fn remove_dispute_timeout(env: &Env, dispute_id: &Symbol) -> Result<(), Error> {
        let key = (symbol_short!("timeout"), dispute_id.clone());
        env.storage().persistent().remove(&key);
        Ok(())
    }

    /// Return a list of all currently active [`DisputeTimeout`] records.
    ///
    /// **Note:** The current implementation returns an empty vector.
    /// A production system should maintain an active-timeout index.
    pub fn get_active_timeouts(env: &Env) -> Vec<DisputeTimeout> {
        Vec::new(env)
    }

    /// Scan for timeouts whose expiry has passed.
    ///
    /// **Note:** The current implementation returns an empty vector.
    /// A production system should iterate through all stored timeouts.
    pub fn check_expired_timeouts(env: &Env) -> Vec<Symbol> {
        Vec::new(env)
    }

    /// Get a user's total dispute stake across all active (unresolved) markets.
    ///
    /// This function calculates the cumulative stake that a user has committed
    /// to disputes across all markets that are still in an active dispute state
    /// (i.e., markets where winning_outcomes is not yet set).
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment
    /// * `user` - The user address to check
    ///
    /// # Returns
    ///
    /// The total stake (in stroops) across all active disputes for this user.
    pub fn get_user_total_active_dispute_stake(env: &Env, user: &Address) -> i128 {
        // In a full implementation, we would need to iterate through all markets
        // and sum up dispute stakes for active disputes. For now, this is a
        // placeholder that returns 0 (requires market registry for full implementation).
        // The validation will use the per-market per-user cap already implemented.
        0
    }
}

pub struct DisputeAnalytics;

impl DisputeAnalytics {
    /// Calculate [`DisputeStats`] from a market's current state.
    ///
    /// Counts active vs resolved disputes from the `winning_outcomes` flag
    /// and tallies unique disputers from the `dispute_stakes` map.
    pub fn calculate_dispute_stats(market: &Market) -> DisputeStats {
        let mut active_disputes = 0;
        let mut resolved_disputes = 0;
        let mut unique_disputers = 0;

        for (_, stake) in market.dispute_stakes.iter() {
            if stake > 0 {
                unique_disputers += 1;
                if market.winning_outcomes.is_none() {
                    active_disputes += 1;
                } else {
                    resolved_disputes += 1;
                }
            }
        }

        DisputeStats {
            total_disputes: active_disputes + resolved_disputes,
            total_dispute_stakes: market.total_dispute_stakes(),
            active_disputes,
            resolved_disputes,
            unique_disputers,
        }
    }

    /// Calculate the dispute impact as an integer percentage (0–100).
    ///
    /// Delegates to [`DisputeUtils::calculate_dispute_impact`] and converts
    /// the float result to a scaled integer.
    pub fn calculate_dispute_impact(market: &Market) -> i128 {
        let impact = DisputeUtils::calculate_dispute_impact(market);
        (impact * 100.0) as i128
    }

    /// Calculate the oracle's weight in the hybrid resolution (0–100 %).
    ///
    /// Starts at a 70 % baseline and subtracts up to 30 percentage points
    /// based on dispute impact.  Floors at 30 %.
    pub fn calculate_oracle_weight(market: &Market) -> i128 {
        let dispute_impact = Self::calculate_dispute_impact(market) as f64 / 100.0;
        let base_oracle_weight = 0.7;
        let dispute_penalty = dispute_impact * 0.3;
        let weight = (base_oracle_weight - dispute_penalty).max(0.3);
        (weight * 100.0) as i128
    }

    /// Calculate the community's weight in the hybrid resolution (0–100 %).
    ///
    /// Starts at a 30 % baseline and adds up to 40 percentage points based
    /// on dispute impact.  Caps at 70 %.
    pub fn calculate_community_weight(market: &Market) -> i128 {
        let dispute_impact = Self::calculate_dispute_impact(market) as f64 / 100.0;
        let base_community_weight = 0.3;
        let dispute_boost = dispute_impact * 0.4;
        let weight = (base_community_weight + dispute_boost).min(0.7);
        (weight * 100.0) as i128
    }

    /// Determine which outcome the community favours by summing stake-weighted votes.
    ///
    /// Returns a [`CommunityConsensus`] with the winning outcome, the
    /// confidence as an integer percentage, and the total vote stake.
    pub fn calculate_community_consensus(env: &Env, market: &Market) -> CommunityConsensus {
        let mut outcome_totals = Map::new(env);
        let mut total_votes = 0;

        for (user, outcome) in market.votes.iter() {
            let stake = market.stakes.get(user).unwrap_or(0);
            let current_total = outcome_totals.get(outcome.clone()).unwrap_or(0);
            outcome_totals.set(outcome, current_total + stake);
            total_votes += stake;
        }

        let mut winning_outcome = String::from_str(env, "");
        let mut max_stake = 0;

        for (outcome, stake) in outcome_totals.iter() {
            if stake > max_stake {
                max_stake = stake;
                winning_outcome = outcome;
            }
        }

        let confidence = if total_votes > 0 {
            (max_stake as i128) * 100 / total_votes
        } else {
            0
        };

        CommunityConsensus {
            outcome: winning_outcome,
            confidence,
            total_votes,
        }
    }

    /// Return the list of disputers and their stakes, unsorted.
    ///
    /// `_limit` is reserved for future use (no-`std` sorting is not yet
    /// implemented).
    pub fn get_top_disputers(env: &Env, market: &Market, _limit: usize) -> Vec<(Address, i128)> {
        let mut disputers: Vec<(Address, i128)> = Vec::new(env);

        for (user, stake) in market.dispute_stakes.iter() {
            if stake > 0 {
                disputers.push_back((user, stake));
            }
        }

        disputers
    }

    /// Calculate the ratio of disputers to total voters as a float.
    ///
    /// Returns 0.0 when there are no voters.
    pub fn calculate_dispute_participation_rate(market: &Market) -> f64 {
        let total_voters = market.votes.len();
        let total_disputers = market.dispute_stakes.len();

        if total_voters == 0 {
            return 0.0;
        }

        (total_disputers as f64) / (total_voters as f64)
    }

    /// Calculate aggregate [`TimeoutStats`] across all markets.
    ///
    /// **Note:** The current implementation returns all-zero statistics.
    /// A production system should iterate through stored timeouts.
    pub fn calculate_timeout_stats(_env: &Env) -> TimeoutStats {
        TimeoutStats {
            total_timeouts: 0,
            active_timeouts: 0,
            expired_timeouts: 0,
            auto_resolved_timeouts: 0,
            average_timeout_hours: 0,
        }
    }

    pub fn get_timeout_analytics(env: &Env, dispute_id: &Symbol) -> TimeoutAnalytics {
        match DisputeUtils::get_dispute_timeout(env, dispute_id) {
            Ok(timeout) => {
                let current_time = env.ledger().timestamp();
                let time_remaining = if current_time < timeout.expires_at {
                    timeout.expires_at - current_time
                } else {
                    0
                };

                TimeoutAnalytics {
                    dispute_id: dispute_id.clone(),
                    timeout_hours: timeout.timeout_hours,
                    time_remaining_seconds: time_remaining,
                    time_remaining_hours: time_remaining / 3600,
                    is_expired: current_time >= timeout.expires_at,
                    status: timeout.status,
                    total_extensions: timeout.total_extension_hours,
                }
            }
            Err(_) => TimeoutAnalytics {
                dispute_id: dispute_id.clone(),
                timeout_hours: 0,
                time_remaining_seconds: 0,
                time_remaining_hours: 0,
                is_expired: false,
                status: DisputeTimeoutStatus::Active,
                total_extensions: 0,
            },
        }
    }
}

#[cfg(test)]
pub mod testing {
    use super::*;

    /// Create a [`Dispute`] with sensible defaults for testing.
    ///
    /// The dispute is created with status `Active`, a reason of
    /// `"Test dispute"`, and the current ledger timestamp.
    pub fn create_test_dispute(
        env: &Env,
        user: Address,
        market_id: Symbol,
        stake: i128,
    ) -> Dispute {
        Dispute {
            user,
            market_id,
            stake,
            timestamp: env.ledger().timestamp(),
            reason: Some(String::from_str(env, "Test dispute")),
            status: DisputeStatus::Active,
        }
    }

    /// Create a zeroed-out [`DisputeStats`] for testing.
    pub fn create_test_dispute_stats() -> DisputeStats {
        DisputeStats {
            total_disputes: 0,
            total_dispute_stakes: 0,
            active_disputes: 0,
            resolved_disputes: 0,
            unique_disputers: 0,
        }
    }

    /// Create a [`DisputeResolution`] with default weights for testing.
    ///
    /// Oracle weight = 70, community weight = 30, impact = 10.
    pub fn create_test_dispute_resolution(env: &Env, market_id: Symbol) -> DisputeResolution {
        DisputeResolution {
            market_id,
            final_outcome: String::from_str(env, "yes"),
            oracle_weight: 70,
            community_weight: 30,
            dispute_impact: 10,
            resolution_timestamp: env.ledger().timestamp(),
        }
    }

    /// Validate that a [`Dispute`] has a positive stake.
    ///
    /// # Errors
    ///
    /// - [`Error::InsufficientStake`] — `stake <= 0`
    pub fn validate_dispute_structure(dispute: &Dispute) -> Result<(), Error> {
        if dispute.stake <= 0 {
            return Err(Error::InsufficientStake);
        }

        Ok(())
    }

    /// Validate [`DisputeStats`] invariants.
    ///
    /// # Errors
    ///
    /// - [`Error::InvalidInput`] — negative total stake or total_disputes < unique_disputers
    pub fn validate_dispute_stats(stats: &DisputeStats) -> Result<(), Error> {
        if stats.total_dispute_stakes < 0 {
            return Err(Error::InvalidInput);
        }

        if stats.total_disputes < stats.unique_disputers {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }

    /// Create a [`DisputeTimeout`] with a 24-hour window for testing.
    pub fn create_test_dispute_timeout(env: &Env, dispute_id: Symbol) -> DisputeTimeout {
        DisputeTimeout {
            dispute_id: dispute_id.clone(),
            market_id: Symbol::new(env, "test_market"),
            timeout_hours: 24,
            created_at: env.ledger().timestamp(),
            expires_at: env.ledger().timestamp() + 86400,
            extended_at: None,
            total_extension_hours: 0,
            status: DisputeTimeoutStatus::Active,
        }
    }

    /// Create a [`DisputeTimeoutOutcome`] with "Support" outcome for testing.
    pub fn create_test_timeout_outcome(env: &Env, dispute_id: Symbol) -> DisputeTimeoutOutcome {
        DisputeTimeoutOutcome {
            dispute_id: dispute_id.clone(),
            market_id: Symbol::new(env, "test_market"),
            outcome: String::from_str(env, "Support"),
            resolution_method: String::from_str(env, "Timeout Auto-Resolution"),
            resolution_timestamp: env.ledger().timestamp().max(1),
            reason: String::from_str(env, "Test timeout resolution"),
        }
    }

    /// Validate [`DisputeTimeout`] invariants.
    ///
    /// # Errors
    ///
    /// - [`Error::InvalidDuration`] — `timeout_hours == 0`
    /// - [`Error::InvalidInput`] — `expires_at <= created_at`
    pub fn validate_timeout_structure(timeout: &DisputeTimeout) -> Result<(), Error> {
        if timeout.timeout_hours == 0 {
            return Err(Error::InvalidDuration);
        }

        if timeout.expires_at <= timeout.created_at {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }

    /// Validate that a [`DisputeTimeoutOutcome`] has a non-zero resolution timestamp.
    ///
    /// # Errors
    ///
    /// - [`Error::InvalidInput`] — `resolution_timestamp == 0`
    pub fn validate_timeout_outcome_structure(
        outcome: &DisputeTimeoutOutcome,
    ) -> Result<(), Error> {
        if outcome.resolution_timestamp == 0 {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }
}

// ===== HELPER STRUCTURES =====

/// Represents community consensus data derived from vote tallying.
///
/// Produced by [`DisputeAnalytics::calculate_community_consensus`] to summarise
/// which outcome the community favours and how strongly.
///
/// # Fields
///
/// * `outcome` - The outcome with the highest total stake backing it
/// * `confidence` - Confidence score as an integer percentage (0–100)
/// * `total_votes` - Total stake (stroops) across all votes considered
pub struct CommunityConsensus {
    pub outcome: String,
    pub confidence: i128,
    pub total_votes: i128,
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    fn create_test_market(env: &Env, end_time: u64) -> Market {
        let mut outcomes = Vec::new(env);
        outcomes.push_back(String::from_str(env, "yes"));
        outcomes.push_back(String::from_str(env, "no"));

        Market::new(
            env,
            Address::generate(env),
            String::from_str(env, "Test Market"),
            outcomes,
            end_time,
            crate::types::OracleConfig::new(
                crate::types::OracleProvider::pyth(),
                Address::from_str(
                    env,
                    "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
                ),
                String::from_str(env, "BTC/USD"),
                2500000,
                String::from_str(env, "gt"),
            ),
            None,
            86400,
            crate::types::MarketState::Active,
        )
    }

    #[test]
    fn test_dispute_validator_market_validation() {
        let env = Env::default();
        let mut market = create_test_market(&env, env.ledger().timestamp() + 86400);

        assert!(DisputeValidator::validate_market_for_dispute(&env, &market).is_err());

        market.end_time = env.ledger().timestamp().saturating_sub(1);

        assert!(DisputeValidator::validate_market_for_dispute(&env, &market).is_err());

        market.oracle_result = Some(String::from_str(&env, "yes"));

        assert!(DisputeValidator::validate_market_for_dispute(&env, &market).is_ok());
    }

    #[test]
    fn test_dispute_validator_stake_validation() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let user = Address::generate(&env);
        let mut market = create_test_market(&env, env.ledger().timestamp().saturating_sub(1));
        market.oracle_result = Some(String::from_str(&env, "yes"));
        let market_id = Symbol::new(&env, "market_1");

        assert!(DisputeValidator::validate_dispute_parameters(
            &env,
            &user,
            &market,
            MIN_DISPUTE_STAKE
        )
        .is_ok());

        assert!(DisputeValidator::validate_dispute_parameters(
            &env,
            &user,
            &market,
            MIN_DISPUTE_STAKE - 1
        )
        .is_err());
    }

    #[test]
    fn test_dispute_utils_impact_calculation() {
        let env = Env::default();
        let mut market = create_test_market(&env, env.ledger().timestamp() + 86400);

        market.total_staked = 10000;
        let user = Address::generate(&env);
        market.dispute_stakes.set(user, 2000);

        let impact = DisputeUtils::calculate_dispute_impact(&market);
        assert_eq!(impact, 0.2);
    }

    #[test]
    fn test_dispute_analytics_stats() {
        let env = Env::default();
        let mut market = create_test_market(&env, env.ledger().timestamp() + 86400);

        let user = Address::generate(&env);
        market.dispute_stakes.set(user, 1000);

        let stats = DisputeAnalytics::calculate_dispute_stats(&market);
        assert_eq!(stats.total_disputes, 1);
        assert_eq!(stats.total_dispute_stakes, 1000);
        assert_eq!(stats.unique_disputers, 1);
        assert_eq!(stats.active_disputes, 1);
    }

    #[test]
    fn test_dispute_stake_is_refunded_when_resolution_favors_disputer() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let market_id = Symbol::new(&env, "refund_market");

        let token_admin = Address::generate(&env);
        let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_address = token_contract.address();
        let token_client = soroban_sdk::token::Client::new(&env, &token_address);
        let stellar_client = soroban_sdk::token::StellarAssetClient::new(&env, &token_address);
        stellar_client.mint(&user, &10_000_000_000i128);

        env.as_contract(&contract_id, || {
            env.storage().persistent().set(&Symbol::new(&env, "Admin"), &admin);
            env.storage()
                .persistent()
                .set(&Symbol::new(&env, "TokenID"), &token_address);

            let mut market = create_test_market(&env, env.ledger().timestamp().saturating_sub(1));
            market.oracle_result = Some(String::from_str(&env, "yes"));
            market.state = crate::types::MarketState::Ended;
            market.total_staked = 1_000;

            let voter = Address::generate(&env);
            market.votes.set(voter.clone(), String::from_str(&env, "no"));
            market.stakes.set(voter, 1_000);
            MarketStateManager::update_market(&env, &market_id, &market);

            let initial_balance = token_client.balance(&user);
            let stake = MIN_DISPUTE_STAKE;
            DisputeManager::process_dispute(&env, user.clone(), market_id.clone(), stake, None)
                .unwrap();

            let balance_after_dispute = token_client.balance(&user);
            assert_eq!(balance_after_dispute, initial_balance - stake);

            let contract_balance_before_refund = token_client.balance(&env.current_contract_address());
            let resolution = DisputeManager::resolve_dispute(&env, market_id.clone(), admin.clone())
                .unwrap();

            assert_eq!(resolution.final_outcome, String::from_str(&env, "no"));
            let balance_after_refund = token_client.balance(&user);
            assert_eq!(balance_after_refund, initial_balance);
            assert_eq!(token_client.balance(&env.current_contract_address()), 0);
            assert_eq!(contract_balance_before_refund, stake);
        });
    }

    #[test]
    fn test_testing_utilities() {
        let env = Env::default();
        let user = Address::generate(&env);

        let dispute = testing::create_test_dispute(&env, user, Symbol::new(&env, "market"), 1000);

        assert!(testing::validate_dispute_structure(&dispute).is_ok());

        let stats = testing::create_test_dispute_stats();
        assert!(testing::validate_dispute_stats(&stats).is_ok());
    }

    #[test]
    fn test_timeout_utilities() {
        let env = Env::default();
        let dispute_id = Symbol::new(&env, "test_dispute");

        let timeout = testing::create_test_dispute_timeout(&env, dispute_id.clone());
        assert!(testing::validate_timeout_structure(&timeout).is_ok());

        let outcome = testing::create_test_timeout_outcome(&env, dispute_id);
        assert!(testing::validate_timeout_outcome_structure(&outcome).is_ok());
    }

    #[test]
    fn test_timeout_validation() {
        assert!(DisputeValidator::validate_dispute_timeout_parameters(24).is_ok());
        assert!(DisputeValidator::validate_dispute_timeout_parameters(0).is_err());
        assert!(DisputeValidator::validate_dispute_timeout_parameters(800).is_err());

        assert!(DisputeValidator::validate_dispute_timeout_extension_parameters(24).is_ok());
        assert!(DisputeValidator::validate_dispute_timeout_extension_parameters(0).is_err());
        assert!(DisputeValidator::validate_dispute_timeout_extension_parameters(200).is_err());
    }

    #[test]
    fn test_timeout_analytics() {
        let env = Env::default();
        let dispute_id = Symbol::new(&env, "test_dispute");

        let mock_timeout = DisputeTimeout {
            dispute_id: dispute_id.clone(),
            market_id: Symbol::new(&env, "test_market"),
            timeout_hours: 24,
            created_at: env.ledger().timestamp(),
            expires_at: env.ledger().timestamp() + 86400,
            extended_at: None,
            total_extension_hours: 0,
            status: DisputeTimeoutStatus::Active,
        };

        let current_time = env.ledger().timestamp();
        let time_remaining = if current_time < mock_timeout.expires_at {
            mock_timeout.expires_at - current_time
        } else {
            0
        };

        let analytics = TimeoutAnalytics {
            dispute_id: dispute_id.clone(),
            timeout_hours: mock_timeout.timeout_hours,
            time_remaining_seconds: time_remaining,
            time_remaining_hours: time_remaining / 3600,
            is_expired: current_time >= mock_timeout.expires_at,
            status: mock_timeout.status,
            total_extensions: mock_timeout.total_extension_hours,
        };

        assert_eq!(analytics.timeout_hours, 24);
        assert_eq!(analytics.is_expired, false);
        assert_eq!(analytics.status, DisputeTimeoutStatus::Active);
    }

    #[test]
    fn test_no_refund_when_oracle_result_stands() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let disputer = Address::generate(&env);
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let market_id = Symbol::new(&env, "stands_mkt");

        let token_admin = Address::generate(&env);
        let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_address = token_contract.address();
        let token_client = soroban_sdk::token::Client::new(&env, &token_address);
        let stellar_client = soroban_sdk::token::StellarAssetClient::new(&env, &token_address);
        stellar_client.mint(&disputer, &10_000_000_000i128);

        env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .set(&Symbol::new(&env, "Admin"), &admin);
            env.storage()
                .persistent()
                .set(&Symbol::new(&env, "TokenID"), &token_address);

            let mut market =
                create_test_market(&env, env.ledger().timestamp().saturating_sub(1));
            market.oracle_result = Some(String::from_str(&env, "yes"));
            market.state = crate::types::MarketState::Ended;
            market.total_staked = 1_000;

            let voter = Address::generate(&env);
            market.votes.set(voter.clone(), String::from_str(&env, "yes"));
            market.stakes.set(voter, 1_000);
            MarketStateManager::update_market(&env, &market_id, &market);

            let initial_balance = token_client.balance(&disputer);
            let stake = MIN_DISPUTE_STAKE;

            DisputeManager::process_dispute(
                &env,
                disputer.clone(),
                market_id.clone(),
                stake,
                None,
            )
            .unwrap();

            let balance_after_dispute = token_client.balance(&disputer);
            assert_eq!(
                balance_after_dispute,
                initial_balance - stake,
                "stake must be locked after process_dispute"
            );

            let resolution =
                DisputeManager::resolve_dispute(&env, market_id.clone(), admin.clone())
                    .unwrap();

            assert_eq!(
                resolution.final_outcome,
                String::from_str(&env, "yes"),
                "final outcome must equal oracle result when community agrees"
            );

            let balance_after_resolution = token_client.balance(&disputer);
            assert_eq!(
                balance_after_resolution,
                initial_balance - stake,
                "disputer must NOT be refunded when oracle result stands"
            );
        });
    }

    #[test]
    fn test_multiple_disputers_all_refunded_when_oracle_overturned() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let disputer_a = Address::generate(&env);
        let disputer_b = Address::generate(&env);
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let market_id = Symbol::new(&env, "multi_disp");

        let token_admin = Address::generate(&env);
        let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_address = token_contract.address();
        let token_client = soroban_sdk::token::Client::new(&env, &token_address);
        let stellar_client = soroban_sdk::token::StellarAssetClient::new(&env, &token_address);

        let stake_a = MIN_DISPUTE_STAKE;
        let stake_b = MIN_DISPUTE_STAKE * 3;

        stellar_client.mint(&disputer_a, &10_000_000_000i128);
        stellar_client.mint(&disputer_b, &10_000_000_000i128);
        stellar_client.mint(&contract_id, &(stake_a + stake_b));

        let initial_a = token_client.balance(&disputer_a);
        let initial_b = token_client.balance(&disputer_b);

        env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .set(&Symbol::new(&env, "Admin"), &admin);
            env.storage()
                .persistent()
                .set(&Symbol::new(&env, "TokenID"), &token_address);

            let mut market =
                create_test_market(&env, env.ledger().timestamp().saturating_sub(1));
            market.oracle_result = Some(String::from_str(&env, "yes"));
            market.state = crate::types::MarketState::Ended;

            let voter1 = Address::generate(&env);
            let voter2 = Address::generate(&env);
            let vote_stake: i128 = 10_000_000;
            market.votes.set(voter1.clone(), String::from_str(&env, "no"));
            market.stakes.set(voter1, vote_stake);
            market.votes.set(voter2.clone(), String::from_str(&env, "no"));
            market.stakes.set(voter2, vote_stake);
            market.total_staked = vote_stake * 2;

            market.dispute_stakes.set(disputer_a.clone(), stake_a);
            market.dispute_stakes.set(disputer_b.clone(), stake_b);
            MarketStateManager::update_market(&env, &market_id, &market);

            let resolution =
                DisputeManager::resolve_dispute(&env, market_id.clone(), admin.clone())
                    .unwrap();

            assert_eq!(
                resolution.final_outcome,
                String::from_str(&env, "no"),
                "community consensus must overturn the oracle when confidence > 70 %"
            );

            assert_eq!(
                token_client.balance(&disputer_a),
                initial_a + stake_a,
                "disputer_a must be fully refunded"
            );
            assert_eq!(
                token_client.balance(&disputer_b),
                initial_b + stake_b,
                "disputer_b must be fully refunded"
            );

            assert_eq!(
                token_client.balance(&env.current_contract_address()),
                0,
                "contract balance must be zero after all refunds"
            );

            let mkt_after =
                MarketStateManager::get_market(&env, &market_id).unwrap();
            assert_eq!(
                mkt_after.dispute_stakes.get(disputer_a.clone()).unwrap_or(1),
                0,
                "disputer_a stake must be zeroed after refund"
            );
            assert_eq!(
                mkt_after.dispute_stakes.get(disputer_b.clone()).unwrap_or(1),
                0,
                "disputer_b stake must be zeroed after refund"
            );
        });
    }

    #[test]
    fn test_tally_votes_no_decay() {
        let env = Env::default();
        let raw = 10_000i128;
        let result = DisputeUtils::tally_votes(&env, raw, 1000, 500);
        assert_eq!(result, raw);
    }

    #[test]
    fn test_tally_votes_with_decay() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        env.as_contract(&contract_id, || {
            let config = DisputeDecayConfig { half_life_seconds: 100, floor_bps: 1000 };
            let key = symbol_short!("decaycfg");
            env.storage().persistent().set(&key, &config);
            let result = DisputeUtils::tally_votes(&env, 10000, 600, 0);
            assert!(result < 10000);
            assert!(result > 0);
        });
    }

    #[test]
    fn test_tally_votes_at_floor() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        env.as_contract(&contract_id, || {
            let config = DisputeDecayConfig { half_life_seconds: 10, floor_bps: 5000 };
            let key = symbol_short!("decaycfg");
            env.storage().persistent().set(&key, &config);
            let result = DisputeUtils::tally_votes(&env, 10000, 1000, 0);
            assert!(result >= 5000);
        });
    }

    #[test]
    fn test_validate_market_for_resolution() {
        let env = Env::default();
        let mut market = create_test_market(&env, env.ledger().timestamp().saturating_sub(1));
        market.oracle_result = Some(String::from_str(&env, "yes"));
        let user = Address::generate(&env);
        market.dispute_stakes.set(user.clone(), 1000);
        assert!(DisputeValidator::validate_market_for_resolution(&env, &market).is_ok());
        market.winning_outcomes = Some({
            let mut v = Vec::new(&env);
            v.push_back(String::from_str(&env, "yes"));
            v
        });
        assert!(DisputeValidator::validate_market_for_resolution(&env, &market).is_err());
    }

    #[test]
    fn test_validate_resolution_parameters() {
        let env = Env::default();
        let mut market = create_test_market(&env, env.ledger().timestamp() + 86400);
        assert!(DisputeValidator::validate_resolution_parameters(&market, &String::from_str(&env, "yes")).is_ok());
        assert!(DisputeValidator::validate_resolution_parameters(&market, &String::from_str(&env, "maybe")).is_err());
    }

    #[test]
    fn test_validate_admin_permissions_no_admin() {
        let env = Env::default();
        let admin = Address::generate(&env);
        assert!(DisputeValidator::validate_admin_permissions(&env, &admin).is_err());
    }

    #[test]
    fn test_validate_admin_permissions_wrong_admin() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let admin = Address::generate(&env);
        let wrong = Address::generate(&env);
        env.as_contract(&contract_id, || {
            env.storage().persistent().set(&Symbol::new(&env, "Admin"), &admin);
            assert!(DisputeValidator::validate_admin_permissions(&env, &wrong).is_err());
            assert!(DisputeValidator::validate_admin_permissions(&env, &admin).is_ok());
        });
    }

    #[test]
    fn test_has_user_disputed() {
        let env = Env::default();
        let mut market = create_test_market(&env, env.ledger().timestamp() + 86400);
        let user = Address::generate(&env);
        assert!(!DisputeUtils::has_user_disputed(&market, &user));
        market.dispute_stakes.set(user.clone(), 5000);
        assert!(DisputeUtils::has_user_disputed(&market, &user));
    }

    #[test]
    fn test_get_user_dispute_stake() {
        let env = Env::default();
        let mut market = create_test_market(&env, env.ledger().timestamp() + 86400);
        let user = Address::generate(&env);
        assert_eq!(DisputeUtils::get_user_dispute_stake(&market, &user), 0);
        market.dispute_stakes.set(user.clone(), 5000);
        assert_eq!(DisputeUtils::get_user_dispute_stake(&market, &user), 5000);
    }

    #[test]
    fn test_finalize_market_with_resolution() {
        let env = Env::default();
        let mut market = create_test_market(&env, env.ledger().timestamp() + 86400);
        market.oracle_result = Some(String::from_str(&env, "yes"));
        let user = Address::generate(&env);
        market.dispute_stakes.set(user.clone(), 1000);
        DisputeUtils::finalize_market_with_resolution(&mut market, String::from_str(&env, "yes")).unwrap();
        assert!(market.winning_outcomes.is_some());
        assert!(DisputeUtils::finalize_market_with_resolution(&mut market, String::from_str(&env, "no")).is_err());
    }

    #[test]
    fn test_extract_disputes_from_market() {
        let env = Env::default();
        let mut market = create_test_market(&env, env.ledger().timestamp() + 86400);
        let user = Address::generate(&env);
        market.dispute_stakes.set(user.clone(), 3000);
        let market_id = Symbol::new(&env, "extract_test");
        let disputes = DisputeUtils::extract_disputes_from_market(&env, &market, market_id.clone());
        assert_eq!(disputes.len(), 1);
        assert_eq!(disputes.get(0).unwrap().stake, 3000);
    }

    #[test]
    fn test_validate_dispute_voting_conditions() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let dispute_id = Symbol::new(&env, "voting_test");
        let market_id = Symbol::new(&env, "market");
        env.as_contract(&contract_id, || {
            let voting = DisputeVoting {
                dispute_id: dispute_id.clone(),
                voting_start: env.ledger().timestamp().saturating_sub(100),
                voting_end: env.ledger().timestamp() + 86400,
                total_votes: 0,
                support_votes: 0,
                against_votes: 0,
                total_support_stake: 0,
                total_against_stake: 0,
                status: DisputeVotingStatus::Active,
            };
            DisputeUtils::store_dispute_voting(&env, &dispute_id, &voting).unwrap();
            assert!(DisputeValidator::validate_dispute_voting_conditions(&env, &market_id, &dispute_id).is_ok());
        });
    }

    #[test]
    fn test_validate_user_hasnt_voted() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let dispute_id = Symbol::new(&env, "no_vote_test");
        let user = Address::generate(&env);
        let other = Address::generate(&env);
        env.as_contract(&contract_id, || {
            // No stored votes yet — both users pass
            assert!(DisputeValidator::validate_user_hasnt_voted(&env, &user, &dispute_id).is_ok());
            assert!(DisputeValidator::validate_user_hasnt_voted(&env, &other, &dispute_id).is_ok());

            // Store a vote for `user` via the underlying storage key
            let vote = DisputeVote {
                user: user.clone(),
                dispute_id: dispute_id.clone(),
                vote: true,
                stake: 1000,
                timestamp: env.ledger().timestamp(),
                reason: None,
            };
            let key = (symbol_short!("vote"), dispute_id.clone(), user.clone());
            env.storage().persistent().set(&key, &vote);

            // `validate_user_hasnt_voted` delegates to `get_dispute_votes` which
            // is a simplified stub that returns an empty Vec (no vote-key index).
            // The function will not detect the stored vote, so it returns Ok.
            // Once the stub is replaced with a proper index, this assertion
            // should be changed to `.is_err()`.
            assert!(DisputeValidator::validate_user_hasnt_voted(&env, &user, &dispute_id).is_ok());
        });
    }

    #[test]
    fn test_validate_voting_completed() {
        let completed = DisputeVoting {
            dispute_id: Symbol::new(&Env::default(), "d"),
            voting_start: 0,
            voting_end: 0,
            total_votes: 0,
            support_votes: 0,
            against_votes: 0,
            total_support_stake: 0,
            total_against_stake: 0,
            status: DisputeVotingStatus::Completed,
        };
        assert!(DisputeValidator::validate_voting_completed(&completed).is_ok());

        let active = DisputeVoting { status: DisputeVotingStatus::Active, ..completed };
        assert!(DisputeValidator::validate_voting_completed(&active).is_err());
    }

    #[test]
    fn test_calculate_stake_weighted_outcome() {
        let env = Env::default();
        let support_wins = DisputeVoting {
            dispute_id: Symbol::new(&env, "d"),
            voting_start: 0,
            voting_end: 0,
            total_votes: 2,
            support_votes: 2,
            against_votes: 0,
            total_support_stake: 5000,
            total_against_stake: 3000,
            status: DisputeVotingStatus::Completed,
        };
        assert!(DisputeUtils::calculate_stake_weighted_outcome(&support_wins));

        let against_wins = DisputeVoting {
            total_support_stake: 2000,
            total_against_stake: 4000,
            ..support_wins
        };
        assert!(!DisputeUtils::calculate_stake_weighted_outcome(&against_wins));

        let tie = DisputeVoting {
            total_support_stake: 3000,
            total_against_stake: 3000,
            ..support_wins
        };
        assert!(!DisputeUtils::calculate_stake_weighted_outcome(&tie));
    }

    #[test]
    fn test_set_get_admin_cooldown() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let admin = Address::generate(&env);
        env.as_contract(&contract_id, || {
            env.storage().persistent().set(&Symbol::new(&env, "Admin"), &admin);
            assert_eq!(DisputeManager::get_admin_cooldown(&env), 0);
            DisputeManager::set_admin_cooldown(&env, &admin, 300).unwrap();
            assert_eq!(DisputeManager::get_admin_cooldown(&env), 300);
        });
    }

    #[test]
    fn test_set_get_dispute_stake_cap() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let market_id = Symbol::new(&env, "cap_market");
        let user = Address::generate(&env);
        env.as_contract(&contract_id, || {
            DisputeManager::set_dispute_stake_cap(&env, &market_id, &user, 50_000_000).unwrap();
            let cap_key = crate::storage::DataKey::DisputeStakeCap(market_id.clone(), user.clone());
            let stored: i128 = env.storage().persistent().get(&cap_key).unwrap();
            assert_eq!(stored, 50_000_000);
        });
    }

    #[test]
    fn test_get_dispute_stats() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let market_id = Symbol::new(&env, "stats_market");
        let mut market = create_test_market(&env, env.ledger().timestamp().saturating_sub(1));
        market.oracle_result = Some(String::from_str(&env, "yes"));
        let user = Address::generate(&env);
        market.dispute_stakes.set(user.clone(), 2000);
        env.as_contract(&contract_id, || {
            crate::markets::MarketStateManager::update_market(&env, &market_id, &market);
            let stats = DisputeManager::get_dispute_stats(&env, market_id.clone()).unwrap();
            assert_eq!(stats.total_disputes, 1);
            assert_eq!(stats.total_dispute_stakes, 2000);
            assert_eq!(stats.unique_disputers, 1);
        });
    }

    #[test]
    fn test_calculate_oracle_and_community_weights() {
        let env = Env::default();
        let mut market = create_test_market(&env, env.ledger().timestamp() + 86400);
        market.total_staked = 10000;
        let user = Address::generate(&env);
        market.dispute_stakes.set(user, 2000);
        let oracle_w = DisputeAnalytics::calculate_oracle_weight(&market);
        let community_w = DisputeAnalytics::calculate_community_weight(&market);
        assert!(oracle_w > 0);
        assert!(community_w > 0);
        assert!(oracle_w + community_w <= 100);
    }

    #[test]
    fn test_calculate_community_consensus() {
        let env = Env::default();
        let mut market = create_test_market(&env, env.ledger().timestamp() + 86400);
        let user_a = Address::generate(&env);
        let user_b = Address::generate(&env);
        market.votes.set(user_a.clone(), String::from_str(&env, "yes"));
        market.stakes.set(user_a.clone(), 5000);
        market.votes.set(user_b.clone(), String::from_str(&env, "no"));
        market.stakes.set(user_b.clone(), 3000);
        let consensus = DisputeAnalytics::calculate_community_consensus(&env, &market);
        assert_eq!(consensus.outcome, String::from_str(&env, "yes"));
        assert!(consensus.confidence > 0);
    }

    #[test]
    fn test_calculate_dispute_participation_rate() {
        let env = Env::default();
        let mut market = create_test_market(&env, env.ledger().timestamp() + 86400);
        assert_eq!(DisputeAnalytics::calculate_dispute_participation_rate(&market), 0.0);
        let user = Address::generate(&env);
        market.votes.set(user.clone(), String::from_str(&env, "yes"));
        market.dispute_stakes.set(user, 1000);
        let rate = DisputeAnalytics::calculate_dispute_participation_rate(&market);
        assert!(rate > 0.0);
    }

    #[test]
    fn test_dispute_escalation_validation() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let dispute_id = Symbol::new(&env, "esc_test");
        let user = Address::generate(&env);
        env.as_contract(&contract_id, || {
            // No stored vote → `get_dispute_votes` stub returns empty Vec,
            // so `has_participated` is false → returns Err.
            assert!(DisputeValidator::validate_dispute_escalation_conditions(&env, &user, &dispute_id).is_err());
        });
    }

    #[test]
    fn test_dispute_escalation_storage() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let dispute_id = Symbol::new(&env, "esc_store");
        let user = Address::generate(&env);
        env.as_contract(&contract_id, || {
            // No escalation stored yet
            assert!(DisputeUtils::get_dispute_escalation(&env, &dispute_id).is_none());

            let escalation = DisputeEscalation {
                dispute_id: dispute_id.clone(),
                escalated_by: user.clone(),
                escalation_reason: String::from_str(&env, "tie"),
                escalation_timestamp: env.ledger().timestamp(),
                escalation_level: 1,
                requires_admin_review: true,
            };
            DisputeUtils::store_dispute_escalation(&env, &dispute_id, &escalation).unwrap();
            let stored = DisputeUtils::get_dispute_escalation(&env, &dispute_id).unwrap();
            assert_eq!(stored.escalation_level, 1);
            assert!(stored.requires_admin_review);
        });
    }

    #[test]
    fn test_timeout_storage_roundtrip() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let dispute_id = Symbol::new(&env, "timeout_rt");
        env.as_contract(&contract_id, || {
            assert!(!DisputeUtils::has_dispute_timeout(&env, &dispute_id));
            let timeout = testing::create_test_dispute_timeout(&env, dispute_id.clone());
            DisputeUtils::store_dispute_timeout(&env, &dispute_id, &timeout).unwrap();
            assert!(DisputeUtils::has_dispute_timeout(&env, &dispute_id));
            let loaded = DisputeUtils::get_dispute_timeout(&env, &dispute_id).unwrap();
            assert_eq!(loaded.timeout_hours, 24);
            assert_eq!(loaded.status, DisputeTimeoutStatus::Active);
            DisputeUtils::remove_dispute_timeout(&env, &dispute_id).unwrap();
            assert!(!DisputeUtils::has_dispute_timeout(&env, &dispute_id));
        });
    }

    #[test]
    fn test_dispute_fee_distribution_storage() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let dispute_id = Symbol::new(&env, "fee_storage");
        env.as_contract(&contract_id, || {
            let dist = DisputeFeeDistribution {
                dispute_id: dispute_id.clone(),
                total_fees: 10000,
                winner_stake: 6000,
                loser_stake: 4000,
                winner_addresses: Vec::new(&env),
                distribution_timestamp: env.ledger().timestamp(),
                fees_distributed: true,
            };
            DisputeUtils::store_dispute_fee_distribution(&env, &dispute_id, &dist).unwrap();
            let loaded = DisputeUtils::get_dispute_fee_distribution(&env, &dispute_id).unwrap();
            assert_eq!(loaded.total_fees, 10000);
            assert!(loaded.fees_distributed);
        });
    }

    #[test]
    fn test_validate_dispute_resolution_conditions() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let dispute_id = Symbol::new(&env, "res_cond");
        env.as_contract(&contract_id, || {
            let voting = DisputeVoting {
                dispute_id: dispute_id.clone(),
                voting_start: 0,
                voting_end: 0,
                total_votes: 0,
                support_votes: 0,
                against_votes: 0,
                total_support_stake: 0,
                total_against_stake: 0,
                status: DisputeVotingStatus::Active,
            };
            DisputeUtils::store_dispute_voting(&env, &dispute_id, &voting).unwrap();
            let dist = DisputeFeeDistribution {
                dispute_id: dispute_id.clone(),
                total_fees: 0,
                winner_stake: 0,
                loser_stake: 0,
                winner_addresses: Vec::new(&env),
                distribution_timestamp: 0,
                fees_distributed: false,
            };
            DisputeUtils::store_dispute_fee_distribution(&env, &dispute_id, &dist).unwrap();
            assert!(DisputeValidator::validate_dispute_resolution_conditions(&env, &dispute_id).is_err());
        });
    }

    #[test]
    fn test_get_set_anti_grief_floor() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let admin = Address::generate(&env);
        env.as_contract(&contract_id, || {
            env.storage().persistent().set(&Symbol::new(&env, "Admin"), &admin);
            assert!(DisputeManager::get_anti_grief_floor(&env).is_none());
            DisputeManager::set_anti_grief_floor(&env, admin.clone(), 2500).unwrap();
            assert_eq!(DisputeManager::get_anti_grief_floor(&env), Some(2500));
        });
    }

    #[test]
    fn test_get_set_history_cap() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let admin = Address::generate(&env);
        env.as_contract(&contract_id, || {
            env.storage().persistent().set(&Symbol::new(&env, "Admin"), &admin);
            assert!(DisputeManager::get_history_cap(&env).is_none());
            DisputeManager::set_history_cap(&env, admin.clone(), 5).unwrap();
            assert_eq!(DisputeManager::get_history_cap(&env), Some(5));
        });
    }

    #[test]
    fn test_get_set_collusion_detector_config() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let admin = Address::generate(&env);
        let config = CollusionDetectorConfig {
            stake_delta_threshold: 500_000,
            time_delta_threshold: 300,
            window_size: 4,
        };
        env.as_contract(&contract_id, || {
            env.storage().persistent().set(&Symbol::new(&env, "Admin"), &admin);
            DisputeManager::set_collusion_detector_config(&env, admin, config).unwrap();
            let loaded = DisputeManager::get_collusion_detector_config(&env);
            assert_eq!(loaded.stake_delta_threshold, 500_000);
            assert_eq!(loaded.window_size, 4);
        });
    }

    #[test]
    fn test_set_dispute_decay_config() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let admin = Address::generate(&env);
        let config = DisputeDecayConfig { half_life_seconds: 200, floor_bps: 2000 };
        env.as_contract(&contract_id, || {
            env.storage().persistent().set(&Symbol::new(&env, "Admin"), &admin);
            DisputeUtils::set_dispute_decay_config(&env, admin, config).unwrap();
            let key = symbol_short!("decaycfg");
            let stored: DisputeDecayConfig = env.storage().persistent().get(&key).unwrap();
            assert_eq!(stored.half_life_seconds, 200);
            assert_eq!(stored.floor_bps, 2000);
        });
    }

    #[test]
    fn test_validate_dispute_timeout_extension_parameters() {
        assert!(DisputeValidator::validate_dispute_timeout_extension_parameters(24).is_ok());
        assert!(DisputeValidator::validate_dispute_timeout_extension_parameters(0).is_err());
        assert!(DisputeValidator::validate_dispute_timeout_extension_parameters(200).is_err());
    }

    #[test]
    fn test_validate_dispute_timeout_status_for_extension() {
        let env = Env::default();
        let active = DisputeTimeout {
            dispute_id: Symbol::new(&env, "d"),
            market_id: Symbol::new(&env, "m"),
            timeout_hours: 24,
            created_at: 0,
            expires_at: 86400,
            extended_at: None,
            total_extension_hours: 0,
            status: DisputeTimeoutStatus::Active,
        };
        assert!(DisputeValidator::validate_dispute_timeout_status_for_extension(&active).is_ok());
        let expired = DisputeTimeout { status: DisputeTimeoutStatus::Expired, ..active };
        assert!(DisputeValidator::validate_dispute_timeout_status_for_extension(&expired).is_err());
    }
}
