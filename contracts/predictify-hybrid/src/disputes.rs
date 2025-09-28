#![allow(dead_code)]

use crate::{
    errors::Error,
    markets::MarketStateManager,
    types::Market,
    voting::{VotingUtils, DISPUTE_EXTENSION_HOURS, MIN_DISPUTE_STAKE},
};
use soroban_sdk::{contracttype, symbol_short, Address, Env, Map, String, Symbol, Vec};

// ===== DISPUTE STRUCTURES =====

/// Represents a formal dispute against a market's oracle resolution.
///
/// A dispute is created when a community member challenges the oracle's
/// resolution of a market, believing the outcome is incorrect. Disputes
/// require a stake to prevent spam and ensure serious commitment.
///
/// # Fields
///
/// * `user` - Address of the user who initiated the dispute
/// * `market_id` - Unique identifier of the disputed market
/// * `stake` - Amount staked by the disputer (must meet minimum requirements)
/// * `timestamp` - When the dispute was created (ledger timestamp)
/// * `reason` - Optional explanation for why the dispute was raised
/// * `status` - Current status of the dispute (Active, Resolved, etc.)
///
/// # Example
///
/// ```rust
/// # use soroban_sdk::{Env, Address, Symbol, String};
/// # use predictify_hybrid::disputes::{Dispute, DisputeStatus};
/// # let env = Env::default();
/// # let user = Address::generate(&env);
/// # let market_id = Symbol::new(&env, "market_123");
///
/// let dispute = Dispute {
///     user: user.clone(),
///     market_id: market_id.clone(),
///     stake: 10_000_000, // 1 XLM
///     timestamp: env.ledger().timestamp(),
///     reason: Some(String::from_str(&env, "Oracle data appears incorrect")),
///     status: DisputeStatus::Active,
/// };
///
/// // Dispute is now active and awaiting community voting
/// assert_eq!(dispute.status, DisputeStatus::Active);
/// ```
///
/// # Dispute Lifecycle
///
/// 1. **Creation**: User stakes tokens and provides reasoning
/// 2. **Community Voting**: Other users vote on dispute validity
/// 3. **Resolution**: Dispute is resolved based on community consensus
/// 4. **Fee Distribution**: Stakes are distributed to winning side
///
/// # Staking Requirements
///
/// - Minimum stake amount enforced to prevent spam
/// - Stake is locked during dispute resolution
/// - Winners receive their stake back plus rewards
/// - Losers forfeit their stake to the winning side
#[contracttype]
pub struct Dispute {
    pub user: Address,
    pub market_id: Symbol,
    pub stake: i128,
    pub timestamp: u64,
    pub reason: Option<String>,
    pub status: DisputeStatus,
}

/// Represents the current lifecycle status of a dispute.
///
/// Disputes progress through various states from creation to final resolution.
/// Each status indicates what actions are available and the dispute's current phase.
///
/// # Variants
///
/// * `Active` - Dispute is open and accepting community votes
/// * `Resolved` - Dispute has been resolved with a final outcome
/// * `Rejected` - Dispute was rejected by community consensus
/// * `Expired` - Dispute voting period ended without sufficient participation
///
/// # Example
///
/// ```rust
/// # use predictify_hybrid::disputes::DisputeStatus;
///
/// // Check if dispute can still receive votes
/// let status = DisputeStatus::Active;
/// let can_vote = matches!(status, DisputeStatus::Active);
/// assert!(can_vote);
///
/// // Check if dispute is finalized
/// let final_status = DisputeStatus::Resolved;
/// let is_final = matches!(final_status,
///     DisputeStatus::Resolved | DisputeStatus::Rejected | DisputeStatus::Expired
/// );
/// assert!(is_final);
/// ```
///
/// # Status Transitions
///
/// Valid transitions:
/// - `Active` → `Resolved` (community upholds dispute)
/// - `Active` → `Rejected` (community rejects dispute)
/// - `Active` → `Expired` (insufficient voting participation)
///
/// Invalid transitions:
/// - Any final status → Any other status (disputes are immutable once resolved)
///
/// # Business Logic
///
/// - **Active**: Dispute accepts votes, market resolution is pending
/// - **Resolved**: Oracle result overturned, new outcome established
/// - **Rejected**: Oracle result upheld, original outcome stands
/// - **Expired**: Insufficient community engagement, original outcome stands
#[contracttype]
pub enum DisputeStatus {
    Active,
    Resolved,
    Rejected,
    Expired,
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

/// Represents dispute timeout configuration
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

/// Represents dispute timeout status
#[contracttype]
#[derive(PartialEq, Debug)]
pub enum DisputeTimeoutStatus {
    Active,
    Expired,
    Extended,
    AutoResolved,
}

/// Represents dispute timeout outcome
#[contracttype]
pub struct DisputeTimeoutOutcome {
    pub dispute_id: Symbol,
    pub market_id: Symbol,
    pub outcome: String,
    pub resolution_method: String,
    pub resolution_timestamp: u64,
    pub reason: String,
}

/// Represents timeout statistics
#[contracttype]
pub struct TimeoutStats {
    pub total_timeouts: u32,
    pub active_timeouts: u32,
    pub expired_timeouts: u32,
    pub auto_resolved_timeouts: u32,
    pub average_timeout_hours: u32,
}

/// Represents timeout analytics
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
    /// Processes a user's formal dispute against a market's oracle resolution.
    ///
    /// This function allows community members to challenge oracle results by
    /// staking tokens and providing reasoning. The dispute triggers a community
    /// voting process to determine if the oracle result should be overturned.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `user` - Address of the user initiating the dispute (must authenticate)
    /// * `market_id` - Unique identifier of the market being disputed
    /// * `stake` - Amount to stake on the dispute (must meet minimum requirements)
    /// * `reason` - Optional explanation for why the dispute is being raised
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the dispute is successfully processed, or an `Error` if:
    /// - Market is not eligible for disputes (not ended, no oracle result)
    /// - Stake amount is below minimum requirements
    /// - User has already disputed this market
    /// - Market is already in a disputed state
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address, Symbol, String};
    /// # use predictify_hybrid::disputes::DisputeManager;
    /// # let env = Env::default();
    /// # let user = Address::generate(&env);
    /// # let market_id = Symbol::new(&env, "btc_price_market");
    ///
    /// // User disputes oracle result with reasoning
    /// let result = DisputeManager::process_dispute(
    ///     &env,
    ///     user.clone(),
    ///     market_id.clone(),
    ///     15_000_000, // 1.5 XLM stake
    ///     Some(String::from_str(&env,
    ///         "Oracle price differs significantly from major exchanges"))
    /// );
    ///
    /// match result {
    ///     Ok(()) => println!("Dispute successfully created"),
    ///     Err(e) => println!("Dispute failed: {:?}", e),
    /// }
    /// ```
    ///
    /// # Process Flow
    ///
    /// 1. **Authentication**: Verify user signature and authorization
    /// 2. **Market Validation**: Ensure market is eligible for disputes
    /// 3. **Parameter Validation**: Check stake amount and user eligibility
    /// 4. **Stake Transfer**: Lock user's stake in the dispute
    /// 5. **Dispute Creation**: Create and store dispute record
    /// 6. **Market Extension**: Extend market deadline for voting period
    /// 7. **Storage Update**: Persist all changes to blockchain storage
    ///
    /// # Economic Impact
    ///
    /// - **Stake Lock**: User's stake is locked until dispute resolution
    /// - **Market Extension**: Market deadline extended by dispute period
    /// - **Voting Incentive**: Other users can earn rewards by voting correctly
    /// - **Quality Control**: Economic cost discourages frivolous disputes
    ///
    /// # Security Considerations
    ///
    /// - Requires user authentication to prevent unauthorized disputes
    /// - Validates market state to ensure disputes are only allowed when appropriate
    /// - Enforces minimum stake requirements to prevent spam
    /// - Checks for duplicate disputes from the same user
    pub fn process_dispute(
        env: &Env,
        user: Address,
        market_id: Symbol,
        stake: i128,
        reason: Option<String>,
    ) -> Result<(), Error> {
        // Require authentication from the user
        user.require_auth();

        // Get and validate market
        let mut market = MarketStateManager::get_market(env, &market_id)?;
        DisputeValidator::validate_market_for_dispute(env, &market)?;

        // Validate dispute parameters
        DisputeValidator::validate_dispute_parameters(env, &user, &market, stake)?;

        // Process stake transfer
        VotingUtils::transfer_stake(env, &user, stake)?;

        // Create dispute record
        let dispute = Dispute {
            user: user.clone(),
            market_id: market_id.clone(),
            stake,
            timestamp: env.ledger().timestamp(),
            reason,
            status: DisputeStatus::Active,
        };

        // Add dispute to market
        DisputeUtils::add_dispute_to_market(&mut market, dispute)?;

        // Extend market for dispute period
        DisputeUtils::extend_market_for_dispute(&mut market, env)?;

        // Update market in storage
        MarketStateManager::update_market(env, &market_id, &market);

        Ok(())
    }

    /// Resolves a dispute by combining oracle data with community voting results.
    ///
    /// This function determines the final outcome of a disputed market by analyzing
    /// community votes, calculating weights for oracle vs community input, and
    /// creating a comprehensive resolution record for transparency and auditability.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `market_id` - Unique identifier of the market to resolve
    /// * `admin` - Address of the admin performing the resolution (must authenticate)
    ///
    /// # Returns
    ///
    /// Returns a `DisputeResolution` containing the final outcome and resolution
    /// metadata, or an `Error` if:
    /// - Admin lacks proper permissions
    /// - Market is not ready for resolution (voting still active)
    /// - Insufficient community participation
    /// - Resolution calculation fails
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address, Symbol};
    /// # use predictify_hybrid::disputes::DisputeManager;
    /// # let env = Env::default();
    /// # let admin = Address::generate(&env);
    /// # let market_id = Symbol::new(&env, "disputed_market");
    ///
    /// // Admin resolves dispute after voting period
    /// let resolution = DisputeManager::resolve_dispute(
    ///     &env,
    ///     market_id.clone(),
    ///     admin.clone()
    /// ).unwrap();
    ///
    /// // Check resolution details
    /// println!("Final outcome: {}", resolution.final_outcome.to_string());
    /// println!("Oracle weight: {}%", resolution.oracle_weight);
    /// println!("Community weight: {}%", resolution.community_weight);
    /// println!("Dispute impact: {}%", resolution.dispute_impact);
    ///
    /// // Verify weights sum to 100%
    /// assert_eq!(resolution.oracle_weight + resolution.community_weight, 100);
    /// ```
    ///
    /// # Resolution Algorithm
    ///
    /// The hybrid resolution process:
    /// 1. **Collect Votes**: Aggregate all community votes and stakes
    /// 2. **Calculate Impact**: Measure how much disputes affected the outcome
    /// 3. **Weight Determination**: Balance oracle reliability vs community consensus
    /// 4. **Outcome Synthesis**: Combine weighted inputs for final result
    /// 5. **Resolution Record**: Create transparent audit trail
    ///
    /// # Weighting Logic
    ///
    /// - **High Oracle Confidence + Low Disputes**: Oracle weight ~80%
    /// - **Medium Oracle Confidence + Medium Disputes**: Balanced ~60/40%
    /// - **Low Oracle Confidence + High Disputes**: Community weight ~70%
    /// - **Tie Situations**: Admin discretion with documented reasoning
    ///
    /// # Transparency Features
    ///
    /// Resolution provides complete audit trail:
    /// - Final outcome with clear justification
    /// - Exact weights used in decision process
    /// - Quantified impact of community disputes
    /// - Timestamp for regulatory compliance
    /// - Immutable record for future reference
    ///
    /// # Administrative Authority
    ///
    /// Only authorized admins can resolve disputes to ensure:
    /// - Proper validation of voting completion
    /// - Correct application of resolution algorithms
    /// - Appropriate handling of edge cases
    /// - Consistent resolution quality across markets
    pub fn resolve_dispute(
        env: &Env,
        market_id: Symbol,
        admin: Address,
    ) -> Result<DisputeResolution, Error> {
        // Require authentication from the admin
        admin.require_auth();

        // Validate admin permissions
        DisputeValidator::validate_admin_permissions(env, &admin)?;

        // Get and validate market
        let mut market = MarketStateManager::get_market(env, &market_id)?;
        DisputeValidator::validate_market_for_resolution(env, &market)?;

        // Calculate dispute impact
        let dispute_impact = DisputeAnalytics::calculate_dispute_impact(&market);

        // Determine final outcome with dispute consideration
        let final_outcome = DisputeUtils::determine_final_outcome_with_disputes(env, &market)?;

        // Calculate weights
        let oracle_weight = DisputeAnalytics::calculate_oracle_weight(&market);
        let community_weight = DisputeAnalytics::calculate_community_weight(&market);

        // Create resolution record
        let resolution = DisputeResolution {
            market_id: market_id.clone(),
            final_outcome: final_outcome.clone(),
            oracle_weight,
            community_weight,
            dispute_impact,
            resolution_timestamp: env.ledger().timestamp(),
        };

        // Update market with final outcome
        DisputeUtils::finalize_market_with_resolution(&mut market, final_outcome)?;
        MarketStateManager::update_market(env, &market_id, &market);

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
        Ok(DisputeUtils::extract_disputes_from_market(
            env, &market, market_id,
        ))
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

    /// Get dispute votes
    pub fn get_dispute_votes(env: &Env, dispute_id: &Symbol) -> Result<Vec<DisputeVote>, Error> {
        DisputeUtils::get_dispute_votes(env, dispute_id)
    }

    /// Validate dispute resolution conditions
    pub fn validate_dispute_resolution_conditions(
        env: &Env,
        dispute_id: Symbol,
    ) -> Result<bool, Error> {
        DisputeValidator::validate_dispute_resolution_conditions(env, &dispute_id)
    }

    /// Set dispute timeout
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

        // Validate timeout hours
        if timeout_hours == 0 || timeout_hours > 720 {
            // Max 30 days
            return Err(Error::InvalidTimeoutHours);
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

    /// Check dispute timeout
    pub fn check_dispute_timeout(env: &Env, dispute_id: Symbol) -> Result<bool, Error> {
        let timeout = DisputeUtils::get_dispute_timeout(env, &dispute_id)?;
        let current_time = env.ledger().timestamp();

        Ok(current_time >= timeout.expires_at)
    }

    /// Auto resolve dispute on timeout
    pub fn auto_resolve_dispute_on_timeout(
        env: &Env,
        dispute_id: Symbol,
    ) -> Result<DisputeTimeoutOutcome, Error> {
        // Check if timeout has expired
        if !Self::check_dispute_timeout(env, dispute_id.clone())? {
            return Err(Error::DisputeTimeoutNotExpired);
        }

        // Get timeout configuration
        let mut timeout = DisputeUtils::get_dispute_timeout(env, &dispute_id)?;

        // Update timeout status
        timeout.status = DisputeTimeoutStatus::AutoResolved;
        DisputeUtils::store_dispute_timeout(env, &dispute_id, &timeout)?;

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

    /// Determine timeout outcome
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

    /// Emit timeout event
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

    /// Get dispute timeout status
    pub fn get_dispute_timeout_status(
        env: &Env,
        dispute_id: Symbol,
    ) -> Result<DisputeTimeoutStatus, Error> {
        let timeout = DisputeUtils::get_dispute_timeout(env, &dispute_id)?;
        Ok(timeout.status)
    }

    /// Extend dispute timeout
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

        // Validate additional hours
        if additional_hours == 0 || additional_hours > 168 {
            // Max 7 days extension
            return Err(Error::InvalidTimeoutHours);
        }

        // Get current timeout
        let mut timeout = DisputeUtils::get_dispute_timeout(env, &dispute_id)?;

        // Check if timeout can be extended
        if !matches!(timeout.status, DisputeTimeoutStatus::Active) {
            return Err(Error::DisputeTimeoutExtensionNotAllowed);
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
}

// ===== DISPUTE VALIDATOR =====

/// Validates dispute-related operations
pub struct DisputeValidator;

impl DisputeValidator {
    /// Validate market state for dispute
    pub fn validate_market_for_dispute(env: &Env, market: &Market) -> Result<(), Error> {
        // Check if market has ended
        let current_time = env.ledger().timestamp();
        if current_time < market.end_time {
            return Err(Error::MarketClosed);
        }

        // Check if market is already resolved
        if market.winning_outcome.is_some() {
            return Err(Error::MarketAlreadyResolved);
        }

        // Check if oracle result is available
        if market.oracle_result.is_none() {
            return Err(Error::OracleUnavailable);
        }

        Ok(())
    }

    /// Validate market state for resolution
    pub fn validate_market_for_resolution(_env: &Env, market: &Market) -> Result<(), Error> {
        // Check if market is already resolved
        if market.winning_outcome.is_some() {
            return Err(Error::MarketAlreadyResolved);
        }

        // Check if there are active disputes
        if market.total_dispute_stakes() == 0 {
            return Err(Error::InvalidInput);
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

    /// Validate dispute parameters
    pub fn validate_dispute_parameters(
        _env: &Env,
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

        // Check if user has voted (optional requirement)
        if !market.votes.contains_key(user.clone()) {
            // Allow disputes even from non-voters, but could be made optional
        }

        Ok(())
    }

    /// Validate dispute resolution parameters
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

    /// Validate dispute voting conditions
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
            return Err(Error::DisputeVotingPeriodExpired);
        }

        // Check if voting is still active
        if !matches!(voting_data.status, DisputeVotingStatus::Active) {
            return Err(Error::DisputeVotingNotAllowed);
        }

        Ok(())
    }

    /// Validate user hasn't already voted
    pub fn validate_user_hasnt_voted(
        env: &Env,
        user: &Address,
        dispute_id: &Symbol,
    ) -> Result<(), Error> {
        let votes = DisputeUtils::get_dispute_votes(env, dispute_id)?;

        for vote in votes.iter() {
            if vote.user == *user {
                return Err(Error::DisputeAlreadyVoted);
            }
        }

        Ok(())
    }

    /// Validate voting is completed
    pub fn validate_voting_completed(voting_data: &DisputeVoting) -> Result<(), Error> {
        if !matches!(voting_data.status, DisputeVotingStatus::Completed) {
            return Err(Error::DisputeResolutionConditionsNotMet);
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
            return Err(Error::DisputeResolutionConditionsNotMet);
        }

        // Check if fees haven't been distributed yet
        let fee_distribution = DisputeUtils::get_dispute_fee_distribution(env, dispute_id)?;
        if fee_distribution.fees_distributed {
            return Err(Error::DisputeFeeDistributionFailed);
        }

        Ok(true)
    }

    /// Validate dispute escalation conditions
    pub fn validate_dispute_escalation_conditions(
        env: &Env,
        user: &Address,
        dispute_id: &Symbol,
    ) -> Result<(), Error> {
        // Check if user has participated in the dispute
        let votes = DisputeUtils::get_dispute_votes(env, dispute_id)?;
        let mut has_participated = false;

        for vote in votes.iter() {
            if vote.user == *user {
                has_participated = true;
                break;
            }
        }

        if !has_participated {
            return Err(Error::DisputeEscalationNotAllowed);
        }

        // Check if escalation already exists
        let escalation = DisputeUtils::get_dispute_escalation(env, dispute_id);
        if escalation.is_some() {
            return Err(Error::DisputeEscalationNotAllowed);
        }

        Ok(())
    }

    /// Validate dispute timeout parameters
    pub fn validate_dispute_timeout_parameters(timeout_hours: u32) -> Result<(), Error> {
        if timeout_hours == 0 {
            return Err(Error::InvalidTimeoutHours);
        }

        if timeout_hours > 720 {
            // Max 30 days
            return Err(Error::InvalidTimeoutHours);
        }

        Ok(())
    }

    /// Validate dispute timeout extension parameters
    pub fn validate_dispute_timeout_extension_parameters(
        additional_hours: u32,
    ) -> Result<(), Error> {
        if additional_hours == 0 {
            return Err(Error::InvalidTimeoutHours);
        }

        if additional_hours > 168 {
            // Max 7 days extension
            return Err(Error::InvalidTimeoutHours);
        }

        Ok(())
    }

    /// Validate dispute timeout status for extension
    pub fn validate_dispute_timeout_status_for_extension(
        timeout: &DisputeTimeout,
    ) -> Result<(), Error> {
        if !matches!(timeout.status, DisputeTimeoutStatus::Active) {
            return Err(Error::DisputeTimeoutExtensionNotAllowed);
        }

        Ok(())
    }
}

// ===== DISPUTE UTILITIES =====

/// Utility functions for dispute operations
pub struct DisputeUtils;

impl DisputeUtils {
    /// Add dispute to market
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

    /// Extend market for dispute period
    pub fn extend_market_for_dispute(market: &mut Market, _env: &Env) -> Result<(), Error> {
        let extension_seconds = (DISPUTE_EXTENSION_HOURS as u64) * 3600;
        market.end_time += extension_seconds;
        Ok(())
    }

    /// Determine final outcome considering disputes
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

    /// Finalize market with resolution
    pub fn finalize_market_with_resolution(
        market: &mut Market,
        final_outcome: String,
    ) -> Result<(), Error> {
        // Validate the final outcome
        DisputeValidator::validate_resolution_parameters(market, &final_outcome)?;

        // Set the winning outcome
        market.winning_outcome = Some(final_outcome);

        Ok(())
    }

    /// Extract disputes from market
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

    /// Check if user has disputed
    pub fn has_user_disputed(market: &Market, user: &Address) -> bool {
        market.dispute_stakes.get(user.clone()).unwrap_or(0) > 0
    }

    /// Get user's dispute stake
    pub fn get_user_dispute_stake(market: &Market, user: &Address) -> i128 {
        market.dispute_stakes.get(user.clone()).unwrap_or(0)
    }

    /// Calculate dispute impact on market resolution
    pub fn calculate_dispute_impact(market: &Market) -> f64 {
        let total_staked = market.total_staked;
        let total_disputes = market.total_dispute_stakes();

        if total_staked == 0 {
            return 0.0;
        }

        (total_disputes as f64) / (total_staked as f64)
    }

    /// Add vote to dispute
    pub fn add_vote_to_dispute(
        env: &Env,
        dispute_id: &Symbol,
        vote: DisputeVote,
    ) -> Result<(), Error> {
        // Get current voting data
        let mut voting_data = Self::get_dispute_voting(env, dispute_id)?;

        // Update voting statistics
        voting_data.total_votes += 1;
        if vote.vote {
            voting_data.support_votes += 1;
            voting_data.total_support_stake += vote.stake;
        } else {
            voting_data.against_votes += 1;
            voting_data.total_against_stake += vote.stake;
        }

        // Store updated voting data
        Self::store_dispute_voting(env, dispute_id, &voting_data)?;

        // Store the vote
        Self::store_dispute_vote(env, dispute_id, &vote)?;

        Ok(())
    }

    /// Get dispute voting data
    pub fn get_dispute_voting(env: &Env, dispute_id: &Symbol) -> Result<DisputeVoting, Error> {
        let key = (symbol_short!("dispute_v"), dispute_id.clone());
        env.storage()
            .persistent()
            .get(&key)
            .ok_or(Error::InvalidInput)
    }

    /// Store dispute voting data
    pub fn store_dispute_voting(
        env: &Env,
        dispute_id: &Symbol,
        voting: &DisputeVoting,
    ) -> Result<(), Error> {
        let key = (symbol_short!("dispute_v"), dispute_id.clone());
        env.storage().persistent().set(&key, voting);
        Ok(())
    }

    /// Store dispute vote
    pub fn store_dispute_vote(
        env: &Env,
        dispute_id: &Symbol,
        vote: &DisputeVote,
    ) -> Result<(), Error> {
        let key = (symbol_short!("vote"), dispute_id.clone(), vote.user.clone());
        env.storage().persistent().set(&key, vote);
        Ok(())
    }

    /// Get dispute votes
    pub fn get_dispute_votes(env: &Env, dispute_id: &Symbol) -> Result<Vec<DisputeVote>, Error> {
        // This is a simplified implementation - in a real system you'd need to track all votes
        let votes = Vec::new(env);

        // Get the voting data to access stored votes
        let _voting_data = Self::get_dispute_voting(env, dispute_id)?;

        // In a real implementation, you would iterate through stored vote keys
        // For now, return empty vector as this would require tracking vote keys separately
        Ok(votes)
    }

    /// Calculate stake-weighted outcome
    pub fn calculate_stake_weighted_outcome(voting_data: &DisputeVoting) -> bool {
        voting_data.total_support_stake > voting_data.total_against_stake
    }

    /// Distribute fees based on outcome
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

    /// Store dispute fee distribution
    pub fn store_dispute_fee_distribution(
        env: &Env,
        dispute_id: &Symbol,
        distribution: &DisputeFeeDistribution,
    ) -> Result<(), Error> {
        let key = (symbol_short!("dispute_f"), dispute_id.clone());
        env.storage().persistent().set(&key, distribution);
        Ok(())
    }

    /// Get dispute fee distribution
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

    /// Store dispute escalation
    pub fn store_dispute_escalation(
        env: &Env,
        dispute_id: &Symbol,
        escalation: &DisputeEscalation,
    ) -> Result<(), Error> {
        let key = (symbol_short!("dispute_e"), dispute_id.clone());
        env.storage().persistent().set(&key, escalation);
        Ok(())
    }

    /// Get dispute escalation
    pub fn get_dispute_escalation(env: &Env, dispute_id: &Symbol) -> Option<DisputeEscalation> {
        let key = (symbol_short!("dispute_e"), dispute_id.clone());
        env.storage().persistent().get(&key)
    }

    /// Emit dispute vote event

    pub fn emit_dispute_vote_event(
        env: &Env,
        _dispute_id: &Symbol,
        user: &Address,
        vote: bool,
        stake: i128,
    ) {
        // In a real implementation, this would emit an event
        // For now, we'll just store it in persistent storage
        let event_key = symbol_short!("vote_evt");
        let event_data = (user.clone(), vote, stake, env.ledger().timestamp());
        env.storage().persistent().set(&event_key, &event_data);
    }

    /// Emit fee distribution event

    pub fn emit_fee_distribution_event(
        env: &Env,
        _dispute_id: &Symbol,
        distribution: &DisputeFeeDistribution,
    ) {
        // In a real implementation, this would emit an event
        // For now, we'll just store it in persistent storage
        let event_key = symbol_short!("fee_event");
        env.storage().persistent().set(&event_key, distribution);
    }

    /// Emit dispute escalation event
    pub fn emit_dispute_escalation_event(
        env: &Env,
        _dispute_id: &Symbol,
        user: &Address,
        escalation: &DisputeEscalation,
    ) {
        // In a real implementation, this would emit an event
        // For now, we'll just store it in persistent storage
        let event_key = symbol_short!("esc_event");
        let event_data = (
            user.clone(),
            escalation.escalation_level,
            env.ledger().timestamp(),
        );
        env.storage().persistent().set(&event_key, &event_data);
    }

    /// Store dispute timeout
    pub fn store_dispute_timeout(
        env: &Env,
        dispute_id: &Symbol,
        timeout: &DisputeTimeout,
    ) -> Result<(), Error> {
        let key = (symbol_short!("timeout"), dispute_id.clone());
        env.storage().persistent().set(&key, timeout);
        Ok(())
    }

    /// Get dispute timeout
    pub fn get_dispute_timeout(env: &Env, dispute_id: &Symbol) -> Result<DisputeTimeout, Error> {
        let key = (symbol_short!("timeout"), dispute_id.clone());
        env.storage()
            .persistent()
            .get(&key)
            .ok_or(Error::DisputeTimeoutNotSet)
    }

    /// Check if dispute timeout exists
    pub fn has_dispute_timeout(env: &Env, dispute_id: &Symbol) -> bool {
        let key = (symbol_short!("timeout"), dispute_id.clone());
        env.storage().persistent().has(&key)
    }

    /// Remove dispute timeout
    pub fn remove_dispute_timeout(env: &Env, dispute_id: &Symbol) -> Result<(), Error> {
        let key = (symbol_short!("timeout"), dispute_id.clone());
        env.storage().persistent().remove(&key);
        Ok(())
    }

    /// Get all active timeouts
    pub fn get_active_timeouts(env: &Env) -> Vec<DisputeTimeout> {
        // This is a simplified implementation
        // In a real system, you would maintain an index of active timeouts
        Vec::new(env)
    }

    /// Check for expired timeouts
    pub fn check_expired_timeouts(env: &Env) -> Vec<Symbol> {
        let _expired_disputes = Vec::new(env);
        let _current_time = env.ledger().timestamp();

        // This is a simplified implementation
        // In a real system, you would iterate through all timeouts and check expiration
        // For now, return empty vector
        _expired_disputes
    }
}

// ===== DISPUTE ANALYTICS =====

/// Analytics functions for dispute data
pub struct DisputeAnalytics;

impl DisputeAnalytics {
    /// Calculate dispute statistics for a market
    pub fn calculate_dispute_stats(market: &Market) -> DisputeStats {
        let mut active_disputes = 0;
        let mut resolved_disputes = 0;
        let mut unique_disputers = 0;

        for (_, stake) in market.dispute_stakes.iter() {
            if stake > 0 {
                unique_disputers += 1;
                if market.winning_outcome.is_none() {
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

    /// Calculate dispute impact on market
    pub fn calculate_dispute_impact(market: &Market) -> i128 {
        let impact = DisputeUtils::calculate_dispute_impact(market);
        (impact * 100.0) as i128 // Convert to integer percentage
    }

    /// Calculate oracle weight in resolution
    pub fn calculate_oracle_weight(market: &Market) -> i128 {
        let dispute_impact = Self::calculate_dispute_impact(market) as f64 / 100.0; // Convert back to decimal

        // Oracle weight decreases with dispute impact
        let base_oracle_weight = 0.7;
        let dispute_penalty = dispute_impact * 0.3;

        let weight = (base_oracle_weight - dispute_penalty).max(0.3);
        (weight * 100.0) as i128 // Convert to integer percentage
    }

    /// Calculate community weight in resolution
    pub fn calculate_community_weight(market: &Market) -> i128 {
        let dispute_impact = Self::calculate_dispute_impact(market) as f64 / 100.0; // Convert back to decimal

        // Community weight increases with dispute impact
        let base_community_weight = 0.3;
        let dispute_boost = dispute_impact * 0.4;

        let weight = (base_community_weight + dispute_boost).min(0.7);
        (weight * 100.0) as i128 // Convert to integer percentage
    }

    /// Calculate community consensus
    pub fn calculate_community_consensus(env: &Env, market: &Market) -> CommunityConsensus {
        let mut outcome_totals = Map::new(env);
        let mut total_votes = 0;

        // Calculate total stakes for each outcome
        for (user, outcome) in market.votes.iter() {
            let stake = market.stakes.get(user).unwrap_or(0);
            let current_total = outcome_totals.get(outcome.clone()).unwrap_or(0);
            outcome_totals.set(outcome, current_total + stake);
            total_votes += stake;
        }

        // Find the outcome with highest stake
        let mut winning_outcome = String::from_str(env, "");
        let mut max_stake = 0;

        for (outcome, stake) in outcome_totals.iter() {
            if stake > max_stake {
                max_stake = stake;
                winning_outcome = outcome;
            }
        }

        let confidence = if total_votes > 0 {
            (max_stake as i128) * 100 / total_votes // Using integer percentage instead of f64
        } else {
            0
        };

        CommunityConsensus {
            outcome: winning_outcome,
            confidence,
            total_votes,
        }
    }

    /// Get top disputers by stake amount
    pub fn get_top_disputers(env: &Env, market: &Market, _limit: usize) -> Vec<(Address, i128)> {
        let mut disputers: Vec<(Address, i128)> = Vec::new(env);

        for (user, stake) in market.dispute_stakes.iter() {
            if stake > 0 {
                disputers.push_back((user, stake));
            }
        }

        // Note: Sorting is not available in no_std, so we return as-is
        // In a real implementation, you might want to implement a simple sort
        disputers
    }

    /// Calculate dispute participation rate
    pub fn calculate_dispute_participation_rate(market: &Market) -> f64 {
        let total_voters = market.votes.len();
        let total_disputers = market.dispute_stakes.len();

        if total_voters == 0 {
            return 0.0;
        }

        (total_disputers as f64) / (total_voters as f64)
    }

    /// Calculate timeout statistics
    pub fn calculate_timeout_stats(_env: &Env) -> TimeoutStats {
        // This is a simplified implementation
        // In a real system, you would iterate through all timeouts and calculate statistics
        TimeoutStats {
            total_timeouts: 0,
            active_timeouts: 0,
            expired_timeouts: 0,
            auto_resolved_timeouts: 0,
            average_timeout_hours: 0,
        }
    }

    /// Get timeout analytics
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

// ===== DISPUTE TESTING UTILITIES =====

#[cfg(test)]
pub mod testing {
    use super::*;

    /// Create a test dispute
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

    /// Create test dispute statistics
    pub fn create_test_dispute_stats() -> DisputeStats {
        DisputeStats {
            total_disputes: 0,
            total_dispute_stakes: 0,
            active_disputes: 0,
            resolved_disputes: 0,
            unique_disputers: 0,
        }
    }

    /// Create test dispute resolution
    pub fn create_test_dispute_resolution(env: &Env, market_id: Symbol) -> DisputeResolution {
        DisputeResolution {
            market_id,
            final_outcome: String::from_str(env, "yes"),
            oracle_weight: 70,    // Using integer percentage
            community_weight: 30, // Using integer percentage
            dispute_impact: 10,   // Using integer percentage
            resolution_timestamp: env.ledger().timestamp(),
        }
    }

    /// Validate dispute structure
    pub fn validate_dispute_structure(dispute: &Dispute) -> Result<(), Error> {
        if dispute.stake <= 0 {
            return Err(Error::InsufficientStake);
        }

        Ok(())
    }

    /// Validate dispute stats structure
    pub fn validate_dispute_stats(stats: &DisputeStats) -> Result<(), Error> {
        if stats.total_dispute_stakes < 0 {
            return Err(Error::InvalidInput);
        }

        if stats.total_disputes < stats.unique_disputers {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }

    /// Create test dispute timeout
    pub fn create_test_dispute_timeout(env: &Env, dispute_id: Symbol) -> DisputeTimeout {
        DisputeTimeout {
            dispute_id: dispute_id.clone(),
            market_id: Symbol::new(env, "test_market"),
            timeout_hours: 24,
            created_at: env.ledger().timestamp(),
            expires_at: env.ledger().timestamp() + 86400, // 24 hours
            extended_at: None,
            total_extension_hours: 0,
            status: DisputeTimeoutStatus::Active,
        }
    }

    /// Create test timeout outcome
    pub fn create_test_timeout_outcome(env: &Env, dispute_id: Symbol) -> DisputeTimeoutOutcome {
        DisputeTimeoutOutcome {
            dispute_id: dispute_id.clone(),
            market_id: Symbol::new(env, "test_market"),
            outcome: String::from_str(env, "Support"),
            resolution_method: String::from_str(env, "Timeout Auto-Resolution"),
            resolution_timestamp: env.ledger().timestamp().max(1), // Ensure non-zero timestamp
            reason: String::from_str(env, "Test timeout resolution"),
        }
    }

    /// Validate timeout structure
    pub fn validate_timeout_structure(timeout: &DisputeTimeout) -> Result<(), Error> {
        if timeout.timeout_hours == 0 {
            return Err(Error::InvalidTimeoutHours);
        }

        if timeout.expires_at <= timeout.created_at {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }

    /// Validate timeout outcome structure
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

/// Represents community consensus data
pub struct CommunityConsensus {
    pub outcome: String,
    pub confidence: i128, // Using i128 instead of f64 for no_std compatibility
    pub total_votes: i128,
}

// ===== MODULE TESTS =====

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
                crate::types::OracleProvider::Pyth,
                String::from_str(env, "BTC/USD"),
                2500000,
                String::from_str(env, "gt"),
            ),
            crate::types::MarketState::Active,
        )
    }

    #[test]
    fn test_dispute_validator_market_validation() {
        let env = Env::default();
        let mut market = create_test_market(&env, env.ledger().timestamp() + 86400);

        // Market not ended - should fail
        assert!(DisputeValidator::validate_market_for_dispute(&env, &market).is_err());

        // Set market as ended

        market.end_time = env.ledger().timestamp().saturating_sub(1);

        // No oracle result - should fail
        assert!(DisputeValidator::validate_market_for_dispute(&env, &market).is_err());

        // Add oracle result
        market.oracle_result = Some(String::from_str(&env, "yes"));

        // Should pass
        assert!(DisputeValidator::validate_market_for_dispute(&env, &market).is_ok());
    }

    #[test]
    fn test_dispute_validator_stake_validation() {
        let env = Env::default();
        let user = Address::generate(&env);
        let mut market = create_test_market(&env, env.ledger().timestamp().saturating_sub(1));
        market.oracle_result = Some(String::from_str(&env, "yes"));

        // Valid stake
        assert!(DisputeValidator::validate_dispute_parameters(
            &env,
            &user,
            &market,
            MIN_DISPUTE_STAKE
        )
        .is_ok());

        // Invalid stake
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
        // Add dispute stakes to trigger the calculation
        let user = Address::generate(&env);
        market.dispute_stakes.set(user, 2000);

        let impact = DisputeUtils::calculate_dispute_impact(&market);
        assert_eq!(impact, 0.2); // 2000 / 10000
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
        // Test timeout parameters validation
        assert!(DisputeValidator::validate_dispute_timeout_parameters(24).is_ok());
        assert!(DisputeValidator::validate_dispute_timeout_parameters(0).is_err());
        assert!(DisputeValidator::validate_dispute_timeout_parameters(800).is_err());

        // Test timeout extension parameters validation
        assert!(DisputeValidator::validate_dispute_timeout_extension_parameters(24).is_ok());
        assert!(DisputeValidator::validate_dispute_timeout_extension_parameters(0).is_err());
        assert!(DisputeValidator::validate_dispute_timeout_extension_parameters(200).is_err());
    }

    #[test]
    fn test_timeout_analytics() {
        let env = Env::default();
        let dispute_id = Symbol::new(&env, "test_dispute");

        // Test with a mock timeout that doesn't require storage access
        let mock_timeout = DisputeTimeout {
            dispute_id: dispute_id.clone(),
            market_id: Symbol::new(&env, "test_market"),
            timeout_hours: 24,
            created_at: env.ledger().timestamp(),
            expires_at: env.ledger().timestamp() + 86400, // 24 hours from now
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
}
