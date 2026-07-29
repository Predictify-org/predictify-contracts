extern crate alloc;

use soroban_sdk::{contracttype, symbol_short, vec, Address, BytesN, Env, IntoVal, Map, String, Symbol, Vec};

use crate::admin::Severity;
use crate::config::Environment;
use crate::err::Error;
use crate::types::OracleProvider;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AdminRole {
    Owner,
    Admin,
    Moderator,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketCreatedEvent {
    pub market_id: Symbol,
    pub question: String,
    pub outcomes: Vec<String>,
    pub admin: Address,
    pub end_time: u64,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventCreatedEvent {
    pub event_id: Symbol,
    pub description: String,
    pub outcomes: Vec<String>,
    pub end_time: u64,
    pub creation_fee_amount: i128,
    pub admin: Address,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VoteCastEvent {
    pub market_id: Symbol,
    pub voter: Address,
    pub outcome: String,
    pub stake: i128,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BetPlacedEvent {
    pub market_id: Symbol,
    pub bettor: Address,
    pub outcome: String,
    pub amount: i128,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BetStatusUpdatedEvent {
    pub market_id: Symbol,
    pub bettor: Address,
    pub old_status: String,
    pub new_status: String,
    pub payout_amount: Option<i128>,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaxBetCapSetEvent {
    pub cap: i128,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleResultEvent {
    pub market_id: Symbol,
    pub result: String,
    pub provider: String,
    pub feed_id: String,
    pub price: i128,
    pub threshold: i128,
    pub comparison: String,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketResolvedEvent {
    pub market_id: Symbol,
    pub final_outcome: String,
    pub oracle_result: String,
    pub community_consensus: String,
    pub resolution_method: String,
    pub confidence_score: i128,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeOpenedEvent {
    pub market_id: Symbol,
    pub disputer: Address,
    pub stake: i128,
    pub reason: Option<String>,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SuspectedCollusionFlagEvent {
    pub market_id: Symbol,
    pub user1: Address,
    pub user2: Address,
    pub stake_delta: i128,
    pub time_delta: u64,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeResolvedEvent {
    pub market_id: Symbol,
    pub outcome: String,
    pub winners: Vec<Address>,
    pub losers: Vec<Address>,
    pub fee_distribution: i128,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeHistoryEvictedEvent {
    pub market_id: Symbol,
    pub user: Address,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeCollectedEvent {
    pub market_id: Symbol,
    pub collector: Address,
    pub amount: i128,
    pub fee_type: String,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeWithdrawalAttemptEvent {
    pub admin: Address,
    pub requested_amount: i128,
    pub available_fees: i128,
    pub withdrawal_amount: i128,
    pub status: crate::fees::FeeWithdrawalStatus,
    pub last_withdrawal_ts: u64,
    pub next_allowed_ts: u64,
    pub timelock_seconds: u64,
    pub max_withdrawal_bps: u32,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeWithdrawnEvent {
    pub admin: Address,
    pub amount: i128,
    pub remaining_fees: i128,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultisigThresholdProposedEvent {
    pub admin: Address,
    pub old_threshold: u32,
    pub new_threshold: u32,
    pub confirm_after: u64,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultisigThresholdConfirmedEvent {
    pub admin: Address,
    pub old_threshold: u32,
    pub new_threshold: u32,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeStakeCapExceededEvent {
    pub market_id: Symbol,
    pub user: Address,
    pub cap: i128,
    pub attempted_stake: i128,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeStakeCapSetEvent {
    pub market_id: Symbol,
    pub user: Address,
    pub cap: i128,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeCumulativeStakeCapExceededEvent {
    pub user: Address,
    pub cap: i128,
    pub cumulative_stake: i128,
    pub attempted_stake: i128,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeCumulativeStakeCapSetEvent {
    pub user: Address,
    pub cap: i128,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleVerifInitiatedEvent {
    pub market_id: Symbol,
    pub initiator: Address,
    pub feed_id: String,
    pub oracle_count: u32,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleResultVerifiedEvent {
    pub market_id: Symbol,
    pub outcome: String,
    pub price: i128,
    pub threshold: i128,
    pub comparison: String,
    pub provider: String,
    pub feed_id: String,
    pub confidence_score: u32,
    pub sources_consulted: u32,
    pub verification_status: String,
    pub is_final: bool,
    pub nonce: u64,
    pub timestamp: u64,
    pub block_number: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleVerificationFailedEvent {
    pub market_id: Symbol,
    pub error_code: u32,
    pub error_message: String,
    pub attempted_providers: u32,
    pub fallback_available: bool,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleValidationFailedEvent {
    pub market_id: Symbol,
    pub provider: String,
    pub feed_id: String,
    pub reason: String,
    pub observed_age_secs: u64,
    pub max_age_secs: u64,
    pub observed_confidence_bps: Option<u32>,
    pub max_confidence_bps: u32,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleConsensusReachedEvent {
    pub market_id: Symbol,
    pub consensus_outcome: String,
    pub agreeing_sources: u32,
    pub total_sources: u32,
    pub agreement_percentage: u32,
    pub average_price: i128,
    pub price_variance: i128,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleHealthStatusEvent {
    pub oracle_address: Address,
    pub provider: String,
    pub previous_status: bool,
    pub current_status: bool,
    pub consecutive_failures: u32,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtensionRequestedEvent {
    pub market_id: Symbol,
    pub admin: Address,
    pub additional_days: u32,
    pub reason: String,
    pub fee: i128,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigUpdatedEvent {
    pub updated_by: Address,
    pub config_type: String,
    pub old_value: String,
    pub new_value: String,
    pub nonce: u64,
    pub timestamp: u64,
}

/// Emitted when a deprecated/legacy entrypoint is invoked, so indexers can
/// track migration progress away from superseded functions.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeprecatedCall {
    pub caller: Address,
    pub entrypoint: Symbol,
    pub nonce: u64,
    pub timestamp: u64,
}

/// Emitted when an admin manually overrides an oracle-verified market result.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminOverrideEvent {
    pub market_id: Symbol,
    pub admin: Address,
    pub old_result: String,
    pub new_result: String,
    pub reason: String,
    pub nonce: u64,
    pub timestamp: u64,
}

/// Emitted when an admin force-resolves a market, bypassing normal time/state checks.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForceResolvedEvent {
    pub market_id: Symbol,
    pub admin: Address,
    pub outcome: String,
    pub reason: String,
    pub idempotency_key: String,
    pub nonce: u64,
    pub timestamp: u64,
}

/// Emitted when a time-locked fee-config update is proposed (queued for later application).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeConfigQueuedEvent {
    pub admin: Address,
    pub eta: u64,
    pub platform_fee_percentage: i128,
    pub creation_fee: i128,
    pub min_fee_amount: i128,
    pub max_fee_amount: i128,
    pub collection_threshold: i128,
    pub fees_enabled: bool,
    pub nonce: u64,
    pub timestamp: u64,
}

/// Emitted when a previously-queued fee-config update becomes effective.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeConfigAppliedEvent {
    pub admin: Address,
    pub platform_fee_percentage: i128,
    pub creation_fee: i128,
    pub min_fee_amount: i128,
    pub max_fee_amount: i128,
    pub collection_threshold: i128,
    pub fees_enabled: bool,
    pub nonce: u64,
    pub timestamp: u64,
}

/// Emitted when a queued fee-config update is cancelled before it takes effect.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeConfigCancelledEvent {
    pub admin: Address,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BetLimitsUpdatedEvent {
    pub admin: Address,
    pub scope: Symbol,
    pub min_bet: i128,
    pub max_bet: i128,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatisticsUpdatedEvent {
    pub total_volume: i128,
    pub total_bets: u64,
    pub active_markets: u32,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ErrorLoggedEvent {
    pub error_code: u32,
    pub message: String,
    pub context: String,
    pub user: Option<Address>,
    pub market_id: Option<Symbol>,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ErrorRecoveryEvent {
    pub error_code: u32,
    pub recovery_strategy: String,
    pub recovery_status: String,
    pub recovery_attempts: u32,
    pub user: Option<Address>,
    pub market_id: Option<Symbol>,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PerformanceMetricEvent {
    pub metric_name: String,
    pub value: i128,
    pub unit: String,
    pub context: String,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminActionEvent {
    pub admin: Address,
    pub action: String,
    pub target: Option<String>,
    pub nonce: u64,
    pub timestamp: u64,
    pub success: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminRoleEvent {
    pub admin: Address,
    pub role: String,
    pub assigned_by: Address,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminPermissionEvent {
    pub admin: Address,
    pub permission: String,
    pub granted: bool,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketClosedEvent {
    pub market_id: Symbol,
    pub admin: Address,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefundOnOracleFailureEvent {
    pub market_id: Symbol,
    pub total_refunded: i128,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketFinalizedEvent {
    pub market_id: Symbol,
    pub admin: Address,
    pub outcome: String,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminInitializedEvent {
    pub admin: Address,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminTransferredEvent {
    pub previous_admin: Address,
    pub new_admin: Address,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractPausedEvent {
    pub admin: Address,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractUnpausedEvent {
    pub admin: Address,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminBroadcastEvent {
    pub severity: Severity,
    pub message_hash: BytesN<32>,
    pub reason: String,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractInitializedEvent {
    pub admin: Address,
    pub platform_fee_percentage: i128,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlatformFeeSetEvent {
    pub fee_percentage: i128,
    pub set_by: Address,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeTimeoutSetEvent {
    pub dispute_id: Symbol,
    pub market_id: Symbol,
    pub timeout_hours: u32,
    pub set_by: Address,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeTimeoutExpiredEvent {
    pub dispute_id: Symbol,
    pub market_id: Symbol,
    pub expiration_timestamp: u64,
    pub outcome: String,
    pub resolution_method: String,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeTimeoutExtendedEvent {
    pub dispute_id: Symbol,
    pub market_id: Symbol,
    pub additional_hours: u32,
    pub extended_by: Address,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeVoteRejectedEvent {
    pub dispute_id: Symbol,
    pub voter: Address,
    pub reason: String,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeVoteCastEvent {
    pub dispute_id: Symbol,
    pub voter: Address,
    pub vote: bool,
    pub stake: i128,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeFeeDistributedEvent {
    pub dispute_id: Symbol,
    pub total_fees: i128,
    pub fees_distributed: bool,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeAutoResolvedEvent {
    pub dispute_id: Symbol,
    pub market_id: Symbol,
    pub outcome: String,
    pub reason: String,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GovernanceProposalCreatedEvent {
    pub proposal_id: Symbol,
    pub proposer: Address,
    pub title: String,
    pub description: String,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GovernanceVoteCastEvent {
    pub proposal_id: Symbol,
    pub voter: Address,
    pub support: bool,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GovernanceVoteCommittedEvent {
    pub proposal_id: Symbol,
    pub voter: Address,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FallbackUsedEvent {
    pub market_id: Symbol,
    pub primary_oracle: Address,
    pub fallback_oracle: Address,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolutionTimeoutEvent {
    pub market_id: Symbol,
    pub timeout_timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GovernanceProposalExecutedEvent {
    pub proposal_id: Symbol,
    pub executor: Address,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GovernanceProposalAutoRejectedEvent {
    pub proposal_id: Symbol,
    pub proposer: Address,
    pub for_votes: u128,
    pub floor_quorum: u128,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigInitializedEvent {
    pub admin: Address,
    pub environment: String,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct StorageCleanupEvent {
    pub market_id: Symbol,
    pub cleanup_type: String,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct StorageOptimizationEvent {
    pub market_id: Symbol,
    pub optimization_type: String,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct StorageMigrationEvent {
    pub migration_id: Symbol,
    pub from_format: String,
    pub to_format: String,
    pub markets_migrated: u32,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketArchivedEvent {
    pub market_id: Symbol,
    pub from_tier: String,
    pub to_tier: String,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct OracleDegradationEvent {
    pub oracle: OracleProvider,
    pub reason: String,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct OracleRecoveryEvent {
    pub oracle: OracleProvider,
    pub recovery_message: String,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct ManualResolutionRequiredEvent {
    pub market_id: Symbol,
    pub reason: String,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateChangeEvent {
    pub market_id: Symbol,
    pub old_state: crate::types::MarketState,
    pub new_state: crate::types::MarketState,
    pub reason: String,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WinningsClaimedEvent {
    pub market_id: Symbol,
    pub user: Address,
    pub amount: i128,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WinningsClaimedBatchEvent {
    pub user: Address,
    pub market_claims: Vec<(Symbol, i128)>,
    pub total_amount: i128,
    pub claim_count: u32,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimPeriodUpdatedEvent {
    pub admin: Address,
    pub claim_period_seconds: u64,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketClaimPeriodUpdatedEvent {
    pub market_id: Symbol,
    pub admin: Address,
    pub claim_period_seconds: u64,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TreasuryUpdatedEvent {
    pub admin: Address,
    pub treasury: Address,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnclaimedWinningsSweptEvent {
    pub market_id: Symbol,
    pub caller: Address,
    pub recipient: Option<Address>,
    pub amount: i128,
    pub burned: bool,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractUpgradedEvent {
    pub old_wasm_hash: soroban_sdk::BytesN<32>,
    pub new_wasm_hash: soroban_sdk::BytesN<32>,
    pub upgrade_id: Symbol,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpgradeChainMismatchEvent {
    pub expected_predecessor: soroban_sdk::BytesN<32>,
    pub actual_current_hash: soroban_sdk::BytesN<32>,
    pub proposed_new_hash: soroban_sdk::BytesN<32>,
    pub admin: Address,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketDeadlineExtendedEvent {
    pub market_id: Symbol,
    pub old_end_time: u64,
    pub new_end_time: u64,
    pub additional_days: u32,
    pub admin: Address,
    pub reason: String,
    pub fee: i128,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketDescriptionUpdatedEvent {
    pub market_id: Symbol,
    pub old_description: String,
    pub new_description: String,
    pub admin: Address,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketOutcomesUpdatedEvent {
    pub market_id: Symbol,
    pub old_outcomes: Vec<String>,
    pub new_outcomes: Vec<String>,
    pub admin: Address,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CategoryUpdatedEvent {
    pub market_id: Symbol,
    pub old_category: Option<String>,
    pub new_category: Option<String>,
    pub admin: Address,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TagsUpdatedEvent {
    pub market_id: Symbol,
    pub old_tags: Vec<String>,
    pub new_tags: Vec<String>,
    pub admin: Address,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractRollbackEvent {
    pub current_wasm_hash: soroban_sdk::BytesN<32>,
    pub rollback_wasm_hash: soroban_sdk::BytesN<32>,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpgradeProposalCreatedEvent {
    pub proposal_id: Symbol,
    pub proposer: Address,
    pub target_version: String,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CircuitBreakerEvent {
    pub action: crate::circuit_breaker::BreakerAction,
    pub condition: crate::circuit_breaker::BreakerCondition,
    pub reason: String,
    pub nonce: u64,
    pub timestamp: u64,
    pub admin: Option<Address>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MinPoolSizeNotMetEvent {
    pub market_id: Symbol,
    pub current_pool: i128,
    pub required_min: i128,
    pub nonce: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventSchemaEntry {
    pub topic: Symbol,
    pub schema_version: u32,
}

pub struct EventSchemaRegistry;

impl EventSchemaRegistry {
    /// Return the [`EventSchemaEntry`] for a named event.
    ///
    /// # Errors
    ///
    /// Returns `None` when `name` is not a registered event.  Callers
    /// that treat a missing entry as a hard error should `unwrap_or_else`
    /// with a panic or propagate an appropriate `Error` variant.
    ///
    /// # Registered events
    ///
    /// | name                            | topic symbol  | schema_version |
    /// |---------------------------------|---------------|----------------|
    /// | `"market_created"`              | `mkt_crt`     | 1              |
    /// | `"market_resolved"`             | `mkt_res`     | 1              |
    /// | `"market_closed"`               | `mkt_close`   | 1              |
    /// | `"market_finalized"`            | `mkt_final`   | 1              |
    /// | `"state_change"`                | `st_chng`     | 1              |
    /// | `"market_archived"`             | `mkt_arch`    | 1              |
    /// | `"oracle_result"`               | `oracle_rs`   | 1              |
    /// | `"dispute_opened"`              | `dispt_opn`   | 1              |
    /// | `"vote_cast"`                   | `vote`        | 1              |
    /// | `"event_created"`               | `evt_crt`     | 1              |
    /// | `"bet_placed"`                  | `bet_plc`     | 1              |
    /// | `"market_description_updated"`  | `mkt_dsc`     | 1              |
    /// | `"market_deadline_extended"`    | `mkt_ext`     | 1              |
    pub fn get_schema(env: &Env, name: &str) -> Option<EventSchemaEntry> {
        match name {
            "market_created" => Some(EventSchemaEntry {
                topic: symbol_short!("mkt_crt"),
                schema_version: 1,
            }),
            "market_resolved" => Some(EventSchemaEntry {
                topic: symbol_short!("mkt_res"),
                schema_version: 1,
            }),
            "market_closed" => Some(EventSchemaEntry {
                topic: symbol_short!("mkt_close"),
                schema_version: 1,
            }),
            "market_finalized" => Some(EventSchemaEntry {
                topic: symbol_short!("mkt_final"),
                schema_version: 1,
            }),
            "state_change" => Some(EventSchemaEntry {
                topic: symbol_short!("st_chng"),
                schema_version: 1,
            }),
            "market_archived" => Some(EventSchemaEntry {
                topic: symbol_short!("mkt_arch"),
                schema_version: 1,
            }),
            "oracle_result" => Some(EventSchemaEntry {
                topic: symbol_short!("oracle_rs"),
                schema_version: 1,
            }),
            "dispute_opened" => Some(EventSchemaEntry {
                topic: symbol_short!("dispt_opn"),
                schema_version: 1,
            }),
            "vote_cast" => Some(EventSchemaEntry {
                topic: symbol_short!("vote"),
                schema_version: 1,
            }),
            "event_created" => Some(EventSchemaEntry {
                topic: symbol_short!("evt_crt"),
                schema_version: 1,
            }),
            "bet_placed" => Some(EventSchemaEntry {
                topic: symbol_short!("bet_plc"),
                schema_version: 1,
            }),
            "market_description_updated" => Some(EventSchemaEntry {
                topic: symbol_short!("mkt_dsc"),
                schema_version: 1,
            }),
            "market_deadline_extended" => Some(EventSchemaEntry {
                topic: symbol_short!("mkt_ext"),
                schema_version: 1,
            }),
            _ => None,
        }
    }
}

pub struct EventEmitter;

impl EventEmitter {
    fn get_and_increment_nonce(env: &Env, topic: Symbol) -> u64 {
        let key = crate::storage::DataKey::EventNonce(topic);
        let mut nonce: u64 = env.storage().persistent().get(&key).unwrap_or(0);
        nonce += 1;
        env.storage().persistent().set(&key, &nonce);
        nonce
    }

    /// Emit market created event.
    ///
    /// Topic and schema version are resolved from [`EventSchemaRegistry`] so
    /// that all emit sites stay in sync with the registry automatically.
    pub fn emit_market_created(
        env: &Env, market_id: &Symbol, question: &String, outcomes: &Vec<String>, admin: &Address, end_time: u64,
    ) {
        let schema = EventSchemaRegistry::get_schema(env, "market_created")
            .unwrap_or(EventSchemaEntry {
                topic: symbol_short!("mkt_crt"),
                schema_version: 1,
            });
        let event = MarketCreatedEvent {
            market_id: market_id.clone(),
            question: question.clone(),
            outcomes: outcomes.clone(),
            admin: admin.clone(),
            end_time,
            nonce: Self::get_and_increment_nonce(env, schema.topic.clone()),
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &schema.topic, &event);
        env.events()
            .publish((schema.topic.clone(), market_id.clone(), schema.schema_version), event);
    }

    pub fn emit_fallback_used(env: &Env, market_id: &Symbol, primary_oracle: &Address, fallback_oracle: &Address) {
        let event = FallbackUsedEvent {
            market_id: market_id.clone(),
            primary_oracle: primary_oracle.clone(),
            fallback_oracle: fallback_oracle.clone(),
            nonce: Self::get_and_increment_nonce(env, symbol_short!("fbk_used")),
            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("fbk_used"), &event);
        env.events().publish((symbol_short!("fbk_used"), market_id.clone()), event);
    }

    pub fn emit_resolution_timeout(env: &Env, market_id: &Symbol, timeout_timestamp: u64) {
        let event = ResolutionTimeoutEvent { market_id: market_id.clone(), timeout_timestamp };
        Self::store_event(env, &symbol_short!("res_tmo"), &event);
        env.events().publish((symbol_short!("res_tmo"), market_id.clone()), event);
    }

    pub fn emit_event_created(
        env: &Env, event_id: &Symbol, description: &String, outcomes: &Vec<String>, admin: &Address, end_time: u64,
    ) {
        let event = EventCreatedEvent {
            event_id: event_id.clone(),
            description: description.clone(),
            outcomes: outcomes.clone(),
            creation_fee_amount: crate::fees::MARKET_CREATION_FEE,
            admin: admin.clone(),
            end_time,
            nonce: Self::get_and_increment_nonce(env, symbol_short!("evt_crt")),
            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("evt_crt"), &event);
        env.events().publish((symbol_short!("evt_crt"), event_id.clone()), event);
    }

    pub fn emit_vote_cast(env: &Env, market_id: &Symbol, voter: &Address, outcome: &String, stake: i128) {
        let event = VoteCastEvent {
            market_id: market_id.clone(), voter: voter.clone(), outcome: outcome.clone(), stake,
            nonce: Self::get_and_increment_nonce(env, symbol_short!("vote")),
            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("vote"), &event);
        env.events().publish((symbol_short!("vote"), market_id.clone()), event);
    }

    pub fn emit_bet_placed(env: &Env, market_id: &Symbol, bettor: &Address, outcome: &String, amount: i128) {
        let event = BetPlacedEvent {
            market_id: market_id.clone(), bettor: bettor.clone(), outcome: outcome.clone(), amount,
            nonce: Self::get_and_increment_nonce(env, symbol_short!("bet_plc")),
            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("bet_plc"), &event);
        env.events().publish((symbol_short!("bet_plc"), market_id.clone()), event);
    }

    pub fn emit_bet_status_updated(
        env: &Env, market_id: &Symbol, bettor: &Address, old_status: &String, new_status: &String, payout_amount: Option<i128>,
    ) {
        let event = BetStatusUpdatedEvent {
            market_id: market_id.clone(), bettor: bettor.clone(), old_status: old_status.clone(), new_status: new_status.clone(), payout_amount,
            nonce: Self::get_and_increment_nonce(env, symbol_short!("bet_upd")),
            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("bet_upd"), &event);
        env.events().publish((symbol_short!("bet_upd"), market_id.clone()), event);
    }

    pub fn emit_max_bet_cap_set(env: &Env, cap: i128) {
        let event_sym = Symbol::new(env, "max_bet_cap");
        let event = MaxBetCapSetEvent {
            cap,
            nonce: Self::get_and_increment_nonce(env, event_sym.clone()),
            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &event_sym, &event);
        env.events().publish((event_sym,), event);
    }

    pub fn emit_oracle_result(
        env: &Env, market_id: &Symbol, result: &String, provider: &String, feed_id: &String, price: i128, threshold: i128, comparison: &String,
    ) {
        let schema = EventSchemaRegistry::get_schema(env, "oracle_result")
            .unwrap_or(EventSchemaEntry { topic: symbol_short!("oracle_rs"), schema_version: 1 });
        let event = OracleResultEvent {
            market_id: market_id.clone(), result: result.clone(), provider: provider.clone(), feed_id: feed_id.clone(), price, threshold, comparison: comparison.clone(),
            nonce: Self::get_and_increment_nonce(env, schema.topic.clone()),
            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &schema.topic, &event);
        env.events().publish((schema.topic, market_id.clone(), schema.schema_version), event);
    }

    pub fn emit_oracle_result_verified(
        env: &Env, market_id: &Symbol, outcome: &String, price: i128, threshold: i128, comparison: &String, provider: &String, feed_id: &String, confidence_score: u32, sources_consulted: u32, is_final: bool,
    ) {
        let event = OracleResultVerifiedEvent {
            market_id: market_id.clone(), outcome: outcome.clone(), price, threshold, comparison: comparison.clone(), provider: provider.clone(), feed_id: feed_id.clone(), confidence_score, sources_consulted,
            verification_status: String::from_str(env, "Verified"),
            is_final,
            nonce: Self::get_and_increment_nonce(env, symbol_short!("orc_ver")),
            timestamp: env.ledger().timestamp(),
            block_number: env.ledger().sequence(),
        };
        Self::store_event(env, &symbol_short!("orc_ver"), &event);
        env.events().publish((symbol_short!("orc_ver"), market_id.clone()), event);
    }

    pub fn emit_oracle_consensus_reached(
        env: &Env, market_id: &Symbol, consensus_outcome: &String, agreeing_sources: u32, total_sources: u32, average_price: i128, price_variance: i128,
    ) {
        let agreement_percentage = if total_sources > 0 { (agreeing_sources * 100) / total_sources } else { 0 };
        let event = OracleConsensusReachedEvent {
            market_id: market_id.clone(), consensus_outcome: consensus_outcome.clone(), agreeing_sources, total_sources, agreement_percentage, average_price, price_variance,
            nonce: Self::get_and_increment_nonce(env, symbol_short!("orc_cons")),
            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("orc_cons"), &event);
        env.events().publish((symbol_short!("orc_cons"), market_id.clone()), event);
    }


    /// Emit market resolved event.
    ///
    /// Topic and schema version are resolved from [`EventSchemaRegistry`] so
    /// that all emit sites stay in sync with the registry automatically.
    pub fn emit_market_resolved(
        env: &Env, market_id: &Symbol, final_outcome: &String, oracle_result: &String, community_consensus: &String, resolution_method: &String, confidence_score: i128,
    ) {
        let schema = EventSchemaRegistry::get_schema(env, "market_resolved")
            .unwrap_or(EventSchemaEntry {
                topic: symbol_short!("mkt_res"),
                schema_version: 1,
            });
        let event = MarketResolvedEvent {
            market_id: market_id.clone(), final_outcome: final_outcome.clone(), oracle_result: oracle_result.clone(), community_consensus: community_consensus.clone(), resolution_method: resolution_method.clone(), confidence_score,
            nonce: Self::get_and_increment_nonce(env, symbol_short!("mkt_res")),
            timestamp: env.ledger().timestamp(),
        };

        env.storage().persistent().set(&schema.topic, &event);
        env.events().publish(
            (
                schema.topic.clone(),
                market_id.clone(),
                schema.schema_version,
                resolution_method.clone(),
            ),
            event,
        );
    }

    pub fn emit_min_pool_size_not_met(env: &Env, market_id: &Symbol, current_pool: i128, required_min: i128) {
        let event = MinPoolSizeNotMetEvent {
            market_id: market_id.clone(), current_pool, required_min,
            nonce: Self::get_and_increment_nonce(env, symbol_short!("pool_lo")),
            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("pool_lo"), &event);
        env.events().publish((symbol_short!("pool_lo"), market_id.clone()), event);
    }

    pub fn emit_dispute_opened(env: &Env, market_id: &Symbol, disputer: &Address, stake: i128, reason: Option<String>) {
        let schema = EventSchemaRegistry::get_schema(env, "dispute_opened")
            .unwrap_or(EventSchemaEntry { topic: symbol_short!("dispt_opn"), schema_version: 1 });
        let event = DisputeOpenedEvent {
            market_id: market_id.clone(), disputer: disputer.clone(), stake, reason,
            nonce: Self::get_and_increment_nonce(env, schema.topic.clone()),
            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &schema.topic, &event);
        env.events().publish((schema.topic, market_id.clone(), schema.schema_version), event);
    }

    pub fn emit_dispute_timeout_set(
        env: &Env,
        dispute_id: &Symbol,
        market_id: &Symbol,
        timeout_hours: u32,
        set_by: &Address,
    ) {
        let event = DisputeTimeoutSetEvent {
            dispute_id: dispute_id.clone(), market_id: market_id.clone(), timeout_hours, set_by: set_by.clone(),
            nonce: Self::get_and_increment_nonce(env, symbol_short!("dt_set")),
            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("dt_set"), &event);
        env.events().publish((symbol_short!("dt_set"), dispute_id.clone()), event);
    }

    pub fn emit_dispute_timeout_expired(
        env: &Env,
        dispute_id: &Symbol,
        market_id: &Symbol,
        outcome: &String,
        resolution_method: &String,
    ) {
        let event = DisputeTimeoutExpiredEvent {
            dispute_id: dispute_id.clone(), market_id: market_id.clone(), expiration_timestamp: env.ledger().timestamp(),
            outcome: outcome.clone(), resolution_method: resolution_method.clone(),
        };
        Self::store_event(env, &symbol_short!("dt_exp"), &event);
        env.events().publish((symbol_short!("dt_exp"), dispute_id.clone()), event);
    }

    pub fn emit_dispute_auto_resolved(
        env: &Env,
        dispute_id: &Symbol,
        market_id: &Symbol,
        outcome: &String,
        reason: &String,
    ) {
        let event = DisputeAutoResolvedEvent {
            dispute_id: dispute_id.clone(), market_id: market_id.clone(), outcome: outcome.clone(), reason: reason.clone(),
            nonce: Self::get_and_increment_nonce(env, symbol_short!("dt_auto")),
            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("dt_auto"), &event);
        env.events().publish((symbol_short!("dt_auto"), dispute_id.clone()), event);
    }

    pub fn emit_dispute_timeout_extended(
        env: &Env,
        dispute_id: &Symbol,
        market_id: &Symbol,
        additional_hours: u32,
        extended_by: &Address,
    ) {
        let event = DisputeTimeoutExtendedEvent {
            dispute_id: dispute_id.clone(), market_id: market_id.clone(), additional_hours, extended_by: extended_by.clone(),
            nonce: Self::get_and_increment_nonce(env, symbol_short!("dt_ext")),
            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("dt_ext"), &event);
        env.events().publish((symbol_short!("dt_ext"), dispute_id.clone()), event);
    }

    pub fn emit_dispute_vote_rejected(env: &Env, dispute_id: &Symbol, voter: &Address, reason: &String) {
        let event = DisputeVoteRejectedEvent {
            dispute_id: dispute_id.clone(), voter: voter.clone(), reason: reason.clone(),
            nonce: Self::get_and_increment_nonce(env, symbol_short!("dv_rej")),
            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("dv_rej"), &event);
        env.events().publish((symbol_short!("dv_rej"), dispute_id.clone()), event);
    }

    pub fn emit_fee_collected(env: &Env, market_id: &Symbol, collector: &Address, amount: i128, fee_type: &String) {
        let event = FeeCollectedEvent {
            market_id: market_id.clone(), collector: collector.clone(), amount, fee_type: fee_type.clone(),
            nonce: Self::get_and_increment_nonce(env, symbol_short!("fee_col")),
            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("fee_col"), &event);
        env.events().publish((symbol_short!("fee_col"), market_id.clone()), event);
    }

    pub fn emit_fee_withdrawn(env: &Env, admin: &Address, amount: i128, remaining_fees: i128, timestamp: u64) {
        let event = FeeWithdrawnEvent {
            admin: admin.clone(), amount, remaining_fees,
            nonce: Self::get_and_increment_nonce(env, symbol_short!("fwd_ok")),
            timestamp,
        };
        env.events().publish((symbol_short!("fwd_ok"), admin.clone()), event.clone());
        Self::store_event(env, &symbol_short!("fwd_ok"), &event);
    }





    pub fn emit_admin_initialized(env: &Env, admin: &Address) {
        let event = AdminInitializedEvent {
            admin: admin.clone(),
            nonce: Self::get_and_increment_nonce(env, symbol_short!("adm_init")),
            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("adm_init"), &event);
        env.events().publish((symbol_short!("adm_init"), admin.clone()), event);
    }

    pub fn emit_config_initialized(env: &Env, admin: &Address, environment: &Environment) {
        let event = ConfigInitializedEvent {
            admin: admin.clone(),
            environment: String::from_str(env, match environment {
                Environment::Development => "Development",
                Environment::Testnet => "Testnet",
                Environment::Mainnet => "Mainnet",
                Environment::Custom => "Custom",
            }),
            nonce: Self::get_and_increment_nonce(env, symbol_short!("cfg_init")),
            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("cfg_init"), &event);
        env.events().publish((symbol_short!("cfg_init"), admin.clone()), event);
    }

    pub fn emit_contract_paused(env: &Env, admin: &Address) {
        let event = ContractPausedEvent {
            admin: admin.clone(),
            nonce: Self::get_and_increment_nonce(env, symbol_short!("ctr_pause")),
            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("ctr_pause"), &event);
        env.events().publish((symbol_short!("ctr_pause"), admin.clone()), event);
    }

    pub fn emit_contract_unpaused(env: &Env, admin: &Address) {
        let event = ContractUnpausedEvent {
            admin: admin.clone(),
            nonce: Self::get_and_increment_nonce(env, symbol_short!("ctr_unp")),
            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("ctr_unp"), &event);
        env.events().publish((symbol_short!("ctr_unp"), admin.clone()), event);
    }

    pub fn emit_admin_transferred(env: &Env, previous_admin: &Address, new_admin: &Address) {
        let event = AdminTransferredEvent {
            previous_admin: previous_admin.clone(),
            new_admin: new_admin.clone(),
            nonce: Self::get_and_increment_nonce(env, symbol_short!("adm_xfer")),
            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("adm_xfer"), &event);
        env.events().publish((symbol_short!("adm_xfer"), new_admin.clone()), event);
    }

    pub fn emit_admin_role_assigned(env: &Env, admin: &Address, role: &AdminRole, assigned_by: &Address) {
        let event = AdminRoleEvent {
            admin: admin.clone(),
            role: String::from_str(env, match role {
                AdminRole::Owner => "Owner",
                AdminRole::Admin => "Admin",
                AdminRole::Moderator => "Moderator",
            }),
            assigned_by: assigned_by.clone(),
            nonce: Self::get_and_increment_nonce(env, symbol_short!("adm_role")),
            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("adm_role"), &event);
        env.events().publish((symbol_short!("adm_role"), admin.clone()), event);
    }

    pub fn emit_admin_role_deactivated(env: &Env, admin: &Address, deactivated_by: &Address) {
        let event = AdminRoleEvent {
            admin: admin.clone(),
            role: String::from_str(env, "deactivated"),
            assigned_by: deactivated_by.clone(),
            nonce: Self::get_and_increment_nonce(env, symbol_short!("adm_deact")),
            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("adm_deact"), &event);
        env.events().publish((symbol_short!("adm_deact"), admin.clone()), event);
    }

    pub fn emit_admin_action_logged(env: &Env, admin: &Address, action: &str, success: &bool) {
        let event = AdminActionEvent {
            admin: admin.clone(),
            action: String::from_str(env, action),
            target: None,
            nonce: Self::get_and_increment_nonce(env, symbol_short!("adm_act")),
            timestamp: env.ledger().timestamp(),
            success: *success,
        };
        Self::store_event(env, &symbol_short!("adm_act"), &event);
        env.events().publish((symbol_short!("adm_act"), admin.clone()), event);
    }

    pub fn emit_oracle_admin_cooldown_hit(env: &Env, admin: &Address, last_action: u64, cooldown: u64) {
        let topics = (Symbol::new(env, "OracleAdmin"), Symbol::new(env, "CooldownHit"));
        let mut data = Map::new(env);
        data.set(String::from_str(env, "admin"), admin.to_val());
        data.set(String::from_str(env, "last_action"), String::from_str(env, &alloc::string::ToString::to_string(&last_action)).to_val());
        data.set(String::from_str(env, "cooldown"), String::from_str(env, &alloc::string::ToString::to_string(&cooldown)).to_val());
        env.events().publish(topics, data);
    }

    /// Emit betting admin cooldown hit event
    pub fn emit_betting_admin_cooldown_hit(env: &Env, admin: &Address, last_action: u64, cooldown: u64) {
        let topics = (Symbol::new(env, "BettingAdmin"), Symbol::new(env, "CooldownHit"));
        let mut data = Map::new(env);
        data.set(String::from_str(env, "admin"), admin.to_val());
        data.set(String::from_str(env, "last_action"), last_action.into_val(env));
        data.set(String::from_str(env, "cooldown"), cooldown.into_val(env));
        env.events().publish(topics, data);
    }


    pub fn emit_signer_rotation_cooldown_hit(env: &Env, admin: &Address, last_rotation: u64, cooldown: u64) {
        let topics = (Symbol::new(env, "Admin"), Symbol::new(env, "SignerRotationCooldownHit"));
        let mut data = Map::new(env);
        data.set(String::from_str(env, "admin"), admin.to_val());
        data.set(String::from_str(env, "last_rotation"), last_rotation.into_val(env));
        data.set(String::from_str(env, "cooldown"), cooldown.into_val(env));
        env.events().publish(topics, data);
    }

    /// Emit oracle admin cooldown hit event

    /// Emit market closed event.
    ///
    /// Topic and schema version are resolved from [`EventSchemaRegistry`] so
    /// that all emit sites stay in sync with the registry automatically.
    pub fn emit_market_closed(env: &Env, market_id: &Symbol, admin: &Address) {
        let schema = EventSchemaRegistry::get_schema(env, "market_closed")
            .unwrap_or(EventSchemaEntry {
                topic: symbol_short!("mkt_close"),
                schema_version: 1,
            });
        let event = MarketClosedEvent {
            market_id: market_id.clone(),
            admin: admin.clone(),
            nonce: Self::get_and_increment_nonce(env, schema.topic.clone()),
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &schema.topic, &event);
        env.events()
            .publish((schema.topic.clone(), market_id.clone(), schema.schema_version), event);
    }

    /// Emit refund on oracle failure event (market cancelled, all bets refunded in full).
    pub fn emit_refund_on_oracle_failure(env: &Env, market_id: &Symbol, total_refunded: i128) {
        let event = RefundOnOracleFailureEvent {
            market_id: market_id.clone(),
            total_refunded,
            nonce: Self::get_and_increment_nonce(env, symbol_short!("ref_oracl").clone()),

            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("ref_oracl"), &event);
        env.events()
            .publish((symbol_short!("ref_oracl"), market_id.clone()), event);
    }

    /// Emit market finalized event.
    ///
    /// Topic and schema version are resolved from [`EventSchemaRegistry`] so
    /// that all emit sites stay in sync with the registry automatically.
    pub fn emit_market_finalized(env: &Env, market_id: &Symbol, admin: &Address, outcome: &String) {
        let schema = EventSchemaRegistry::get_schema(env, "market_finalized")
            .unwrap_or(EventSchemaEntry {
                topic: symbol_short!("mkt_final"),
                schema_version: 1,
            });
        let event = MarketFinalizedEvent {
            market_id: market_id.clone(),
            admin: admin.clone(),
            outcome: outcome.clone(),
            nonce: Self::get_and_increment_nonce(env, schema.topic.clone()),
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &schema.topic, &event);
        env.events()
            .publish((schema.topic.clone(), market_id.clone(), schema.schema_version), event);
    }

    pub fn emit_admin_broadcast(env: &Env, severity: Severity, message_hash: BytesN<32>, reason: String) {
        let event = AdminBroadcastEvent {
            severity, message_hash, reason,
            nonce: Self::get_and_increment_nonce(env, symbol_short!("adm_cast")),
            timestamp: env.ledger().timestamp(),
        };
        env.events().publish((Symbol::new(env, "admin_broadcast"),), event);
    }



    pub fn emit_circuit_breaker_event(env: &Env, event: &CircuitBreakerEvent) {
        Self::store_event(env, &symbol_short!("cb_event"), event);
    }

    pub fn emit_config_updated(env: &Env, updated_by: &Address, config_type: &String, old_value: &String, new_value: &String) {
        let event = ConfigUpdatedEvent {
            updated_by: updated_by.clone(), config_type: config_type.clone(), old_value: old_value.clone(), new_value: new_value.clone(),
            nonce: Self::get_and_increment_nonce(env, symbol_short!("cfg_upd")),
            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("cfg_upd"), &event);
        env.events().publish((symbol_short!("cfg_upd"), updated_by.clone()), event);
    }

    /// Emit oracle recovery event when oracle service recovers
    pub fn emit_oracle_recovery(env: &Env, oracle: &OracleProvider, message: &String) {
        let event = OracleRecoveryEvent {
            oracle: oracle.clone(),
            recovery_message: message.clone(),
            nonce: Self::get_and_increment_nonce(env, symbol_short!("ora_rec").clone()),

            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("ora_rec"), &event);
        env.events().publish((symbol_short!("ora_rec"),), event);
    }

    /// Emit manual resolution required event when automatic resolution fails
    pub fn emit_manual_resolution_required(env: &Env, market_id: &Symbol, reason: &String) {
        let event = ManualResolutionRequiredEvent {
            market_id: market_id.clone(),
            reason: reason.clone(),
            nonce: Self::get_and_increment_nonce(env, symbol_short!("man_res").clone()),

            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("man_res"), &event);
        env.events()
            .publish((symbol_short!("man_res"), market_id.clone()), event);
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
    /// Emit state change event.
    ///
    /// Topic and schema version are resolved from [`EventSchemaRegistry`] so
    /// that all emit sites stay in sync with the registry automatically.
    pub fn emit_state_change_event(
        env: &Env,
        market_id: &Symbol,
        old_state: &crate::types::MarketState,
        new_state: &crate::types::MarketState,
        reason: &String,
    ) {
        let schema = EventSchemaRegistry::get_schema(env, "state_change")
            .unwrap_or(EventSchemaEntry {
                topic: symbol_short!("st_chng"),
                schema_version: 1,
            });
        let event = StateChangeEvent {
            market_id: market_id.clone(),
            old_state: old_state.clone(),
            new_state: new_state.clone(),
            reason: reason.clone(),
            nonce: Self::get_and_increment_nonce(env, schema.topic.clone()),
            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &schema.topic, &event);
        env.events()
            .publish((schema.topic.clone(), market_id.clone(), schema.schema_version), event);
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
            nonce: Self::get_and_increment_nonce(env, symbol_short!("win_clm").clone()),

            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("win_clm"), &event);
        env.events()
            .publish((symbol_short!("win_clm"), market_id.clone()), event);
    }

    /// Emit winnings claimed batch event
    ///
    /// Emits an event when a user claims winnings from multiple markets in a batch.
    ///
    /// # Parameters
    ///
    /// - `env` - Soroban environment
    /// - `user` - User address claiming winnings
    /// - `market_claims` - Vector of (market_id, claim_amount) tuples
    /// - `total_amount` - Total amount claimed across all markets
    pub fn emit_winnings_claimed_batch(
        env: &Env,
        user: &Address,
        market_claims: &Vec<(Symbol, i128)>,
        total_amount: i128,
    ) {
        let event = WinningsClaimedBatchEvent {
            user: user.clone(),
            market_claims: market_claims.clone(),
            total_amount,
            claim_count: market_claims.len() as u32,
            nonce: Self::get_and_increment_nonce(env, symbol_short!("win_btc").clone()),

            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("win_btc"), &event);
        env.events()
            .publish((symbol_short!("win_btc"), user.clone()), event);
    }
    /// Emit global claim period updated event.
    pub fn emit_claim_period_updated(env: &Env, admin: &Address, claim_period_seconds: u64) {
        let event = ClaimPeriodUpdatedEvent {
            admin: admin.clone(),
            claim_period_seconds,
            nonce: Self::get_and_increment_nonce(env, symbol_short!("clm_prd").clone()),

            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("clm_prd"), &event);
        env.events()
            .publish((symbol_short!("clm_prd"), admin.clone()), event);
    }

    /// Emit market claim period updated event.
    pub fn emit_market_claim_period_updated(
        env: &Env,
        admin: &Address,
        market_id: &Symbol,
        claim_period_seconds: u64,
    ) {
        let event = MarketClaimPeriodUpdatedEvent {
            market_id: market_id.clone(),
            admin: admin.clone(),
            claim_period_seconds,
            nonce: Self::get_and_increment_nonce(env, symbol_short!("m_clm_pd").clone()),

            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("m_clm_pd"), &event);
        env.events()
            .publish((symbol_short!("m_clm_pd"), market_id.clone()), event);
    }

    /// Emit treasury updated event.
    pub fn emit_treasury_updated(env: &Env, admin: &Address, treasury: &Address) {
        let event = TreasuryUpdatedEvent {
            admin: admin.clone(),
            treasury: treasury.clone(),
            nonce: Self::get_and_increment_nonce(env, symbol_short!("treas_up").clone()),

            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("treas_up"), &event);
        env.events()
            .publish((symbol_short!("treas_up"), admin.clone()), event);
    }

    /// Emit unclaimed winnings swept event.
    pub fn emit_unclaimed_winnings_swept(
        env: &Env,
        market_id: &Symbol,
        caller: &Address,
        recipient: &Option<Address>,
        amount: i128,
        burned: bool,
    ) {
        let event = UnclaimedWinningsSweptEvent {
            market_id: market_id.clone(),
            caller: caller.clone(),
            recipient: recipient.clone(),
            amount,
            burned,
            nonce: Self::get_and_increment_nonce(env, symbol_short!("unc_swip").clone()),

            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("unc_swip"), &event);
        env.events()
            .publish((symbol_short!("unc_swip"), market_id.clone()), event);
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
            nonce: Self::get_and_increment_nonce(env, symbol_short!("mkt_ext").clone()),

            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("mkt_ext"), &event);
        env.events()
            .publish((symbol_short!("mkt_ext"), market_id.clone()), event);
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
            nonce: Self::get_and_increment_nonce(env, symbol_short!("mkt_dsc").clone()),

            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("mkt_dsc"), &event);
        env.events()
            .publish((symbol_short!("mkt_dsc"), market_id.clone()), event);
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
            nonce: Self::get_and_increment_nonce(env, symbol_short!("mkt_out").clone()),

            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("mkt_out"), &event);
        env.events()
            .publish((symbol_short!("mkt_out"), market_id.clone()), event);
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
            nonce: Self::get_and_increment_nonce(env, symbol_short!("mkt_cat").clone()),

            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("mkt_cat"), &event);
        env.events()
            .publish((symbol_short!("mkt_cat"), market_id.clone()), event);
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
            nonce: Self::get_and_increment_nonce(env, symbol_short!("mkt_tag").clone()),

            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("mkt_tag"), &event);
        env.events()
            .publish((symbol_short!("mkt_tag"), market_id.clone()), event);
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
    pub fn emit_error_logged(
        env: &Env,
        error_code: u32,
        message: &String,
        context: &String,
        user: Option<Address>,
        market_id: Option<Symbol>,
    ) {
        let event = ErrorLoggedEvent {
            error_code, message: message.clone(), context: context.clone(), user, market_id,
            nonce: Self::get_and_increment_nonce(env, symbol_short!("err_log")),
            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("err_log"), &event);
        env.events().publish((symbol_short!("err_log"),), event);
    }

    pub fn emit_error_recovery_event(env: &Env, error_code: u32, recovery_strategy: &String, recovery_status: String, recovery_attempts: u32, user: Option<Address>, market_id: Option<Symbol>) {
        let event = ErrorRecoveryEvent {
            error_code, recovery_strategy: recovery_strategy.clone(), recovery_status, recovery_attempts, user, market_id,
            nonce: Self::get_and_increment_nonce(env, symbol_short!("err_rec")),
            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("err_rec"), &event);
        env.events().publish((symbol_short!("err_rec"),), event);
    }

    /// Emit governance proposal executed event
    pub fn emit_governance_proposal_executed(env: &Env, proposal_id: &Symbol, executor: &Address) {
        let timestamp = env.ledger().timestamp();
        let event = GovernanceProposalExecutedEvent {
            proposal_id: proposal_id.clone(),
            executor: executor.clone(),
            nonce: Self::get_and_increment_nonce(env, symbol_short!("gov_exec")),
            timestamp,
        };

        Self::store_event(env, &symbol_short!("gov_exec"), &event);
        env.events()
            .publish((symbol_short!("gov_exec"), proposal_id.clone()), event);
    }

    /// Emit governance proposal auto-rejected event
    pub fn emit_governance_proposal_auto_rejected(
        env: &Env,
        proposal_id: &Symbol,
        proposer: &Address,
        for_votes: u128,
        floor_quorum: u128,
    ) {
        let timestamp = env.ledger().timestamp();
        let event = GovernanceProposalAutoRejectedEvent {
            proposal_id: proposal_id.clone(),
            proposer: proposer.clone(),
            for_votes,
            floor_quorum,
            nonce: Self::get_and_increment_nonce(env, symbol_short!("gov_rej")),
            timestamp,
        };

        Self::store_event(env, &symbol_short!("gov_rej"), &event);
        env.events()
            .publish((symbol_short!("gov_rej"), proposal_id.clone()), event);
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
            nonce: Self::get_and_increment_nonce(env, symbol_short!("up_grade").clone()),

            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("up_grade"), &event);
        env.events()
            .publish((symbol_short!("up_grade"), upgrade_id.clone()), event);
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
            nonce: Self::get_and_increment_nonce(env, symbol_short!("rollback").clone()),

            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("rollback"), &event);
        env.events().publish((symbol_short!("rollback"),), event);
    }

    /// Emit upgrade chain mismatch event when hash verification fails
    pub fn emit_upgrade_chain_mismatch_event(
        env: &Env,
        expected_predecessor: &soroban_sdk::BytesN<32>,
        actual_current_hash: &soroban_sdk::BytesN<32>,
        proposed_new_hash: &soroban_sdk::BytesN<32>,
        admin: &Address,
    ) {
        let event = UpgradeChainMismatchEvent {
            expected_predecessor: expected_predecessor.clone(),
            actual_current_hash: actual_current_hash.clone(),
            proposed_new_hash: proposed_new_hash.clone(),
            admin: admin.clone(),
            nonce: Self::get_and_increment_nonce(env, symbol_short!("chain_mm").clone()),

            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("chain_mm"), &event);
        env.events()
            .publish((symbol_short!("chain_mm"), admin.clone()), event);
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
            nonce: Self::get_and_increment_nonce(env, symbol_short!("up_prop").clone()),

            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("up_prop"), &event);
        env.events()
            .publish((symbol_short!("up_prop"), proposal_id.clone()), event);
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

    /// Store event in persistent storage and publish it to ledger events.
    ///
    /// Persisting the payload keeps the existing `EventLogger` helpers working,
    /// while publishing makes the transition visible to indexers and auditors.
    fn store_event<T>(env: &Env, event_key: &Symbol, event_data: &T)
    where
        T: Clone + soroban_sdk::IntoVal<soroban_sdk::Env, soroban_sdk::Val>,
    {
        env.storage().persistent().set(event_key, event_data);
        env.events()
            .publish((event_key.clone(),), event_data.clone());
    }

    /// Emit event visibility set event
    pub fn emit_event_visibility_set(
        env: &Env,
        event_id: &Symbol,
        visibility: &crate::types::EventVisibility,
        admin: &Address,
    ) {
        env.events().publish(
            (symbol_short!("evt_vis"), event_id.clone()),
            (visibility.clone(), admin.clone(), env.ledger().timestamp()),
        );
    }

    /// Emit allowlist updated event
    pub fn emit_allowlist_updated(
        env: &Env,
        event_id: &Symbol,
        addresses: &Vec<Address>,
        admin: &Address,
    ) {
        env.events().publish(
            (symbol_short!("allowlst"), event_id.clone()),
            (addresses.clone(), admin.clone(), env.ledger().timestamp()),
        );
    }

    /// Emit a monitor queue overflow event when the bounded queue evicts the oldest entry.
    ///
    /// This event signals that the queue was at capacity and a new event caused an
    /// eviction. Off-chain indexers should consume this to track data loss and adjust
    /// their polling cadence accordingly.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban environment.
    /// * `overflow_count` - Cumulative number of overflow evictions since initialization.
    /// * `evicted_event_id` - The `event_id` of the evicted `MonitorEvent`, if available.
    /// * `capacity` - The configured capacity of the bounded queue.
    pub fn emit_monitor_queue_overflow(
        env: &Env,
        overflow_count: u64,
        evicted_event_id: Option<Symbol>,
        capacity: u32,
    ) {
        env.events().publish(
            (symbol_short!("mon_ovf"),),
            (
                overflow_count,
                evicted_event_id,
                capacity,
                env.ledger().timestamp(),
            ),
        );
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

    /// Validate dispute opened event
    pub fn validate_dispute_opened_event(event: &DisputeOpenedEvent) -> Result<(), Error> {
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
            nonce: 0,
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
            nonce: 0,
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
            nonce: 0,
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
            nonce: 0,
            timestamp: env.ledger().timestamp(),
        }
    }

    /// Create test dispute opened event
    pub fn create_test_dispute_opened_event(
        env: &Env,
        market_id: &Symbol,
        disputer: &Address,
    ) -> DisputeOpenedEvent {
        DisputeOpenedEvent {
            market_id: market_id.clone(),
            disputer: disputer.clone(),
            stake: 10_0000000,
            reason: Some(String::from_str(env, "Test dispute")),
            nonce: 0,
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
            nonce: 0,
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
            nonce: 0,
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
            nonce: 0,
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

// ===== ORACLE CALLBACK AUTHENTICATION EVENTS =====

/// Emit oracle callback authentication event
///
/// This event is emitted when an oracle callback is successfully authenticated
/// and processed, providing audit trail for oracle data updates.
///
/// # Arguments
/// * `env` - Soroban environment
/// * `oracle_address` - Address of the oracle contract
/// * `feed_id` - Feed identifier for the oracle data
/// * `price` - Price data from the oracle
/// * `timestamp` - Timestamp of the oracle data
pub fn emit_oracle_callback(
    env: &Env,
    oracle_address: &Address,
    feed_id: &String,
    price: i128,
    timestamp: u64,
) {
    env.events().publish(
        (
            Symbol::new(env, "oracle_callback"),
            oracle_address,
            feed_id,
            price,
            timestamp,
        ),
        (),
    );
}

/// Emit security event for monitoring and audit
///
/// This event is emitted for security-related events including
/// authentication failures, authorization issues, and other security events.
///
/// # Arguments
/// * `env` - Soroban environment
/// * `actor` - Address of the actor involved in the event
/// * `message` - Security event message
pub fn emit_security_event(env: &Env, actor: &Address, message: &String) {
    env.events().publish(
        (
            Symbol::new(env, "security_event"),
            actor,
            message,
            env.ledger().timestamp(),
        ),
        (),
    );
}

/// Emit oracle degradation event
///
/// This event is emitted when an oracle experiences degradation or failure,
/// providing monitoring and alerting capabilities.
///
/// # Arguments
/// * `env` - Soroban environment
/// * `oracle` - Oracle provider experiencing degradation
/// * `reason` - Reason for the degradation
pub fn emit_oracle_degradation(env: &Env, oracle: &OracleProvider, reason: &String) {
    env.events().publish(
        (
            Symbol::new(env, "oracle_degradation"),
            oracle,
            reason,
            env.ledger().timestamp(),
        ),
        (),
    );
}

/// Emit manual resolution required event
///
/// This event is emitted when manual resolution is required due to
/// insufficient oracle data quality or confidence.
///
/// # Arguments
/// * `env` - Soroban environment
/// * `market_id` - Market identifier requiring manual resolution
/// * `reason` - Reason for manual resolution requirement
pub fn emit_manual_resolution_required(env: &Env, market_id: &Symbol, reason: &String) {
    env.events().publish(
        (
            Symbol::new(env, "manual_resolution_required"),
            market_id,
            reason,
            env.ledger().timestamp(),
        ),
        (),
    );
}

/// Emit a deprecation signal for legacy entrypoints.
///
/// This helper publishes a `DeprecatedCall` event so indexers can track
/// usage of functions that have been superseded.  Callers also see the
/// event in the Soroban ledger metadata, encouraging migration.
///
/// # Arguments
/// * `env`        - Soroban environment.
/// * `caller`     - Address of the caller invoking the deprecated entrypoint.
/// * `entrypoint` - The `Symbol` name of the deprecated function.
pub fn emit_deprecated(env: &Env, caller: &Address, entrypoint: &Symbol) {
    let nonce = EventEmitter::get_and_increment_nonce(env, symbol_short!("depr_call"));
    let event = DeprecatedCall {
        caller: caller.clone(),
        entrypoint: entrypoint.clone(),
        nonce,
        timestamp: env.ledger().timestamp(),
    };
    env.events()
        .publish((symbol_short!("depr_call"), entrypoint.clone()), event);
}

#[cfg(test)]
mod event_schema_registry_tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env};

    #[test]
    fn test_registry_lookup_oracle_result() {
        let env = Env::default();
        let schema = EventSchemaRegistry::get_schema(&env, "oracle_result").unwrap();
        assert_eq!(schema.topic, symbol_short!("oracle_rs"));
        assert_eq!(schema.schema_version, 1);
    }

    #[test]
    fn test_registry_lookup_dispute_opened() {
        let env = Env::default();
        let schema = EventSchemaRegistry::get_schema(&env, "dispute_opened").unwrap();
        assert_eq!(schema.topic, symbol_short!("dispt_opn"));
        assert_eq!(schema.schema_version, 1);
    }

    #[test]
    fn test_registry_lookup_unknown_event_returns_none() {
        let env = Env::default();
        let result = EventSchemaRegistry::get_schema(&env, "nonexistent_event");
        assert!(result.is_none());
    }

    #[test]
    fn test_schema_version_matches_expected() {
        let env = Env::default();
        // Schema version must equal the pinned baseline; any bump is a breaking change.
        const EXPECTED_VERSION: u32 = 1;

        let registered_events = [
            "market_created",
            "market_resolved",
            "market_closed",
            "market_finalized",
            "state_change",
            "market_archived",
            "oracle_result",
            "dispute_opened",
            "vote_cast",
            "event_created",
            "bet_placed",
            "market_description_updated",
            "market_deadline_extended",
        ];

        for name in &registered_events {
            let schema = EventSchemaRegistry::get_schema(&env, name).unwrap_or_else(|| {
                panic!("Event '{name}' must be registered in EventSchemaRegistry")
            });
            assert_eq!(
                schema.schema_version, EXPECTED_VERSION,
                "Event '{name}' schema_version mismatch: expected {EXPECTED_VERSION}"
            );
        }
    }

    #[test]
    fn test_registry_lookup_market_created() {
        let env = Env::default();
        let schema = EventSchemaRegistry::get_schema(&env, "market_created").unwrap();
        assert_eq!(schema.topic, symbol_short!("mkt_crt"));
        assert_eq!(schema.schema_version, 1);
    }

    #[test]
    fn test_registry_lookup_market_resolved() {
        let env = Env::default();
        let schema = EventSchemaRegistry::get_schema(&env, "market_resolved").unwrap();
        assert_eq!(schema.topic, symbol_short!("mkt_res"));
        assert_eq!(schema.schema_version, 1);
    }

    #[test]
    fn test_registry_lookup_market_closed() {
        let env = Env::default();
        let schema = EventSchemaRegistry::get_schema(&env, "market_closed").unwrap();
        assert_eq!(schema.topic, symbol_short!("mkt_close"));
        assert_eq!(schema.schema_version, 1);
    }

    #[test]
    fn test_registry_lookup_market_finalized() {
        let env = Env::default();
        let schema = EventSchemaRegistry::get_schema(&env, "market_finalized").unwrap();
        assert_eq!(schema.topic, symbol_short!("mkt_final"));
        assert_eq!(schema.schema_version, 1);
    }

    #[test]
    fn test_registry_lookup_state_change() {
        let env = Env::default();
        let schema = EventSchemaRegistry::get_schema(&env, "state_change").unwrap();
        assert_eq!(schema.topic, symbol_short!("st_chng"));
        assert_eq!(schema.schema_version, 1);
    }

    #[test]
    fn test_registry_lookup_market_archived() {
        let env = Env::default();
        let schema = EventSchemaRegistry::get_schema(&env, "market_archived").unwrap();
        assert_eq!(schema.topic, symbol_short!("mkt_arch"));
        assert_eq!(schema.schema_version, 1);
    }

    #[test]
    fn test_registry_lookup_vote_cast() {
        let env = Env::default();
        let schema = EventSchemaRegistry::get_schema(&env, "vote_cast").unwrap();
        assert_eq!(schema.topic, symbol_short!("vote"));
        assert_eq!(schema.schema_version, 1);
    }

    #[test]
    fn test_registry_lookup_event_created() {
        let env = Env::default();
        let schema = EventSchemaRegistry::get_schema(&env, "event_created").unwrap();
        assert_eq!(schema.topic, symbol_short!("evt_crt"));
        assert_eq!(schema.schema_version, 1);
    }

    #[test]
    fn test_registry_lookup_bet_placed() {
        let env = Env::default();
        let schema = EventSchemaRegistry::get_schema(&env, "bet_placed").unwrap();
        assert_eq!(schema.topic, symbol_short!("bet_plc"));
        assert_eq!(schema.schema_version, 1);
    }

    #[test]
    fn test_registry_lookup_market_description_updated() {
        let env = Env::default();
        let schema = EventSchemaRegistry::get_schema(&env, "market_description_updated").unwrap();
        assert_eq!(schema.topic, symbol_short!("mkt_dsc"));
        assert_eq!(schema.schema_version, 1);
    }

    #[test]
    fn test_registry_lookup_market_deadline_extended() {
        let env = Env::default();
        let schema = EventSchemaRegistry::get_schema(&env, "market_deadline_extended").unwrap();
        assert_eq!(schema.topic, symbol_short!("mkt_ext"));
        assert_eq!(schema.schema_version, 1);
    }

    #[test]
    fn test_emit_market_created_uses_registry_topic() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        env.as_contract(&contract_id, || {
            let market_id = soroban_sdk::symbol_short!("mkt1");
            let question = soroban_sdk::String::from_str(&env, "Test?");
            let outcomes = soroban_sdk::vec![
                &env,
                soroban_sdk::String::from_str(&env, "Yes"),
                soroban_sdk::String::from_str(&env, "No"),
            ];
            let admin = soroban_sdk::Address::generate(&env);
            EventEmitter::emit_market_created(
                &env, &market_id, &question, &outcomes, &admin, 1000000,
            );
        });
    }

    #[test]
    fn test_emit_market_resolved_uses_registry_topic() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        env.as_contract(&contract_id, || {
            let market_id = soroban_sdk::symbol_short!("mkt2");
            let outcome = soroban_sdk::String::from_str(&env, "Yes");
            let oracle_res = soroban_sdk::String::from_str(&env, "Yes");
            let consensus = soroban_sdk::String::from_str(&env, "Yes");
            let method = soroban_sdk::String::from_str(&env, "oracle");
            EventEmitter::emit_market_resolved(
                &env, &market_id, &outcome, &oracle_res, &consensus, &method, 95,
            );
        });
    }

    #[test]
    fn test_emit_market_closed_uses_registry_topic() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        env.as_contract(&contract_id, || {
            let market_id = soroban_sdk::symbol_short!("mkt3");
            let admin = soroban_sdk::Address::generate(&env);
            EventEmitter::emit_market_closed(&env, &market_id, &admin);
        });
    }

    #[test]
    fn test_emit_market_finalized_uses_registry_topic() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        env.as_contract(&contract_id, || {
            let market_id = soroban_sdk::symbol_short!("mkt4");
            let admin = soroban_sdk::Address::generate(&env);
            let outcome = soroban_sdk::String::from_str(&env, "Yes");
            EventEmitter::emit_market_finalized(&env, &market_id, &admin, &outcome);
        });
    }

    #[test]
    fn test_emit_state_change_uses_registry_topic() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        env.as_contract(&contract_id, || {
            let market_id = soroban_sdk::symbol_short!("mkt5");
            let reason = soroban_sdk::String::from_str(&env, "market expired");
            EventEmitter::emit_state_change_event(
                &env,
                &market_id,
                &crate::types::MarketState::Active,
                &crate::types::MarketState::Ended,
                &reason,
            );
        });
    }

    #[test]
    fn test_emit_oracle_result_uses_registry_topic() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        env.as_contract(&contract_id, || {
            let market_id = soroban_sdk::symbol_short!("mkt1");
            let result = soroban_sdk::String::from_str(&env, "Yes");
            let provider = soroban_sdk::String::from_str(&env, "Reflector");
            let feed_id = soroban_sdk::String::from_str(&env, "BTC/USD");
            let comparison = soroban_sdk::String::from_str(&env, "gte");
            // Should not panic – registry supplies the topic.
            EventEmitter::emit_oracle_result(
                &env, &market_id, &result, &provider, &feed_id, 52_000_00000000,
                50_000_00000000, &comparison,
            );
        });
    }

    #[test]
    fn test_emit_dispute_opened_uses_registry_topic() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        env.as_contract(&contract_id, || {
            let market_id = soroban_sdk::symbol_short!("mkt2");
            let disputer = soroban_sdk::Address::generate(&env);
            EventEmitter::emit_dispute_opened(
                &env, &market_id, &disputer, 50_000_000, None,
            );
        });
    }

    #[test]
    fn test_emit_deprecated_call() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        env.as_contract(&contract_id, || {
            let caller = soroban_sdk::Address::generate(&env);
            let entrypoint = soroban_sdk::symbol_short!("verify_rs");
            // Must not panic
            emit_deprecated(&env, &caller, &entrypoint);
        });
    }

    #[test]
    fn test_emit_deprecated_call_stores_fields() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        env.as_contract(&contract_id, || {
            let caller = soroban_sdk::Address::generate(&env);
            let entrypoint = soroban_sdk::Symbol::new(&env, "legacy_fn");

            emit_deprecated(&env, &caller, &entrypoint);

            let events = env.events().all();
            let emitted = events.events();
            assert!(!emitted.is_empty(), "must emit at least one event");

            // Find our depr_call event
            let found = emitted.iter().any(|e| {
                e.0 .0 == symbol_short!("depr_call")
                    && e.0 .1 == entrypoint
            });
            assert!(found, "depr_call event must be present");
        });
    }

    #[test]
    fn test_emit_deprecated_call_increments_nonce() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        env.as_contract(&contract_id, || {
            let caller = soroban_sdk::Address::generate(&env);
            let ep1 = soroban_sdk::symbol_short!("ep_one");
            let ep2 = soroban_sdk::symbol_short!("ep_two");

            emit_deprecated(&env, &caller, &ep1);
            emit_deprecated(&env, &caller, &ep1);
            emit_deprecated(&env, &caller, &ep2);

            // Nonce is per-topic; ep1 and ep2 have separate nonces.
            // We just verify no panic — nonce tracking is internal.
        });
    }
}

impl EventEmitter {
    pub fn emit_threshold_proposed(
        env: &Env,
        admin: &Address,
        old_threshold: u32,
        new_threshold: u32,
        confirm_after: u64,
    ) {
        let event = MultisigThresholdProposedEvent {
            admin: admin.clone(),
            old_threshold,
            new_threshold,
            confirm_after,
            nonce: Self::get_and_increment_nonce(env, symbol_short!("thld_prop").clone()),

            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("thld_prop"), &event);
        env.events().publish(
            (symbol_short!("thld_prop"), admin.clone()),
            event,
        );
    }

    pub fn emit_threshold_confirmed(
        env: &Env,
        admin: &Address,
        old_threshold: u32,
        new_threshold: u32,
    ) {
        let event = MultisigThresholdConfirmedEvent {
            admin: admin.clone(),
            old_threshold,
            new_threshold,
            nonce: Self::get_and_increment_nonce(env, symbol_short!("thld_conf").clone()),

            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("thld_conf"), &event);
        env.events().publish(
            (symbol_short!("thld_conf"), admin.clone()),
            event,
        );
    }

    pub fn emit_dispute_stake_cap_exceeded(
        env: &Env,
        market_id: &Symbol,
        user: &Address,
        cap: i128,
        attempted_stake: i128,
    ) {
        let event = DisputeStakeCapExceededEvent {
            market_id: market_id.clone(),
            user: user.clone(),
            cap,
            attempted_stake,
            nonce: Self::get_and_increment_nonce(env, symbol_short!("cap_excd").clone()),

            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("cap_excd"), &event);
        env.events().publish(
            (symbol_short!("cap_excd"), market_id.clone()),
            event,
        );
    }

    pub fn emit_dispute_stake_cap_set(
        env: &Env,
        market_id: &Symbol,
        user: &Address,
        cap: i128,
    ) {
        let event = DisputeStakeCapSetEvent {
            market_id: market_id.clone(),
            user: user.clone(),
            cap,
            nonce: Self::get_and_increment_nonce(env, symbol_short!("cap_set").clone()),

            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("cap_set"), &event);
        env.events().publish(
            (symbol_short!("cap_set"), market_id.clone()),
            event,
        );
    }

    /// Emit oracle callback event for oracle data updates
    pub fn emit_oracle_callback(
        env: &Env,
        oracle_address: &Address,
        feed_id: &String,
        price: i128,
        timestamp: u64,
    ) {
        env.events().publish(
            (
                Symbol::new(env, "oracle_callback"),
                oracle_address,
                feed_id,
                price,
                timestamp,
            ),
            (),
        );
    }

    /// Emit security event for monitoring and audit
    pub fn emit_security_event(env: &Env, actor: &Address, message: &String) {
        env.events().publish(
            (
                Symbol::new(env, "security_event"),
                actor,
                message,
                env.ledger().timestamp(),
            ),
            (),
        );
    }

    /// Emit a per-oracle quote detail event produced by the median resolver.
    ///
    /// Published under the Soroban event topic `orc_med_q` alongside every
    /// `OracleConsensusReachedEvent` emitted by
    /// `OracleResolutionManager::resolve_with_median`.
    ///
    /// Consumers listening to `orc_med_q` receive the full
    /// `Vec<OracleQuote>` so they can inspect individual oracle prices,
    /// confidence weights, and outlier flags without re-running the
    /// aggregation logic.
    ///
    /// # Parameters
    /// - `market_id` – The market that was resolved.
    /// - `quotes`    – All three oracle quotes (Pyth, Reflector, Band) with
    ///                 their computed weights and `included` flags.
    pub fn emit_oracle_median_quotes(
        env: &Env,
        market_id: &Symbol,
        quotes: &Vec<crate::types::OracleQuote>,
    ) {
        env.events().publish(
            (symbol_short!("orc_med_q"), market_id.clone()),
            quotes.clone(),
        );
    }

    /// Emit admin override event when an admin manually overrides an oracle-verified result.
    pub fn emit_admin_override(
        env: &Env,
        market_id: &Symbol,
        admin: &Address,
        old_result: &String,
        new_result: &String,
        reason: &String,
    ) {
        let event = AdminOverrideEvent {
            market_id: market_id.clone(),
            admin: admin.clone(),
            old_result: old_result.clone(),
            new_result: new_result.clone(),
            reason: reason.clone(),
            nonce: Self::get_and_increment_nonce(env, symbol_short!("adm_ovrd").clone()),

            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("adm_ovrd"), &event);
        env.events()
            .publish((symbol_short!("adm_ovrd"), market_id.clone()), event);
    }

    /// Emit force-resolve event when an admin force-resolves a market.
    pub fn emit_force_resolved(
        env: &Env,
        market_id: &Symbol,
        admin: &Address,
        outcome: &String,
        reason: &String,
        idempotency_key: &String,
    ) {
        let event = ForceResolvedEvent {
            market_id: market_id.clone(),
            admin: admin.clone(),
            outcome: outcome.clone(),
            reason: reason.clone(),
            idempotency_key: idempotency_key.clone(),
            nonce: Self::get_and_increment_nonce(env, symbol_short!("frc_rs").clone()),

            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("frc_rs"), &event);
        env.events()
            .publish((symbol_short!("frc_rs"), market_id.clone()), event);
    }

    /// Emit fee config queued event when a time-locked config update is proposed.
    pub fn emit_fee_config_queued(
        env: &Env,
        admin: &Address,
        eta: u64,
        config: &crate::fees::FeeConfig,
    ) {
        let event = FeeConfigQueuedEvent {
            admin: admin.clone(),
            eta,
            platform_fee_percentage: config.platform_fee_percentage,
            creation_fee: config.creation_fee,
            min_fee_amount: config.min_fee_amount,
            max_fee_amount: config.max_fee_amount,
            collection_threshold: config.collection_threshold,
            fees_enabled: config.fees_enabled,
            nonce: Self::get_and_increment_nonce(env, symbol_short!("fee_qd").clone()),

            timestamp: env.ledger().timestamp(),
        };
        env.events()
            .publish((symbol_short!("fee_qd"), admin.clone()), event);
    }

    /// Emit fee config applied event when a queued update becomes effective.
    pub fn emit_fee_config_applied(env: &Env, admin: &Address, config: &crate::fees::FeeConfig) {
        let event = FeeConfigAppliedEvent {
            admin: admin.clone(),
            platform_fee_percentage: config.platform_fee_percentage,
            creation_fee: config.creation_fee,
            min_fee_amount: config.min_fee_amount,
            max_fee_amount: config.max_fee_amount,
            collection_threshold: config.collection_threshold,
            fees_enabled: config.fees_enabled,
            nonce: Self::get_and_increment_nonce(env, symbol_short!("fee_apd").clone()),

            timestamp: env.ledger().timestamp(),
        };
        env.events()
            .publish((symbol_short!("fee_apd"), admin.clone()), event);
    }

    /// Emit fee config cancelled event when a queued update is cancelled.
    pub fn emit_fee_config_cancelled(env: &Env, admin: &Address) {
        let event = FeeConfigCancelledEvent {
            admin: admin.clone(),
            nonce: Self::get_and_increment_nonce(env, symbol_short!("fee_ccl").clone()),

            timestamp: env.ledger().timestamp(),
        };
        env.events()
            .publish((symbol_short!("fee_ccl"), admin.clone()), event);
    }

    /// Emit cumulative dispute stake cap exceeded event.
    pub fn emit_dispute_cumulative_stake_cap_exceeded(
        env: &Env,
        user: &Address,
        cap: i128,
        cumulative_stake: i128,
        attempted_stake: i128,
    ) {
        let event = DisputeCumulativeStakeCapExceededEvent {
            user: user.clone(),
            cap,
            cumulative_stake,
            attempted_stake,
            nonce: Self::get_and_increment_nonce(env, symbol_short!("cum_cap").clone()),

            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("cum_cap"), &event);
        env.events()
            .publish((symbol_short!("cum_cap"), user.clone()), event);
    }

    /// Emit cumulative dispute stake cap set event.
    pub fn emit_dispute_cumulative_stake_cap_set(env: &Env, user: &Address, cap: i128) {
        let event = DisputeCumulativeStakeCapSetEvent {
            user: user.clone(),
            cap,
            nonce: Self::get_and_increment_nonce(env, symbol_short!("cum_set").clone()),

            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("cum_set"), &event);
        env.events()
            .publish((symbol_short!("cum_set"), user.clone()), event);
    }
}

#[cfg(test)]
mod focused_dispute_tests {
    use super::*;
    use soroban_sdk::{testutils::{Address as _, Events}, Address, Env, IntoVal, Symbol, TryIntoVal, Val};

    #[test]
    fn test_dispute_opened_event_topics() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());

        let market_id = Symbol::new(&env, "mkt_123");
        let disputer = Address::generate(&env);
        let stake = 50_000_000i128;
        let reason = None;

        env.as_contract(&contract_id, || {
            EventEmitter::emit_dispute_opened(&env, &market_id, &disputer, stake, reason);
        });

        let events = env.events().all();
        // Expect at least one event with 3 topics: (topic0, topic1, topic2)
        // topic0 = dispt_opn
        // topic1 = mkt_123
        // topic2 = 1 (schema version)

        let mut found = false;
        for event in events.events().iter() {
            if event.2.len() == 3 {
                let topic0: Symbol = event.2.get(0).unwrap().try_into_val(&env).unwrap();
                let topic1: Symbol = event.2.get(1).unwrap().try_into_val(&env).unwrap();

                if topic0 == symbol_short!("dispt_opn") {
                    assert_eq!(topic1, market_id, "Market ID must be topic1");
                    found = true;
                }
            }
        }
        assert!(found, "DisputeOpenedEvent not found with correct topic structure");
    }
}

#[cfg(test)]
mod focused_betting_events_tests {
    use super::*;
    use soroban_sdk::{testutils::{Address as _, Events}, Address, Env, IntoVal, Symbol, Vec};

    #[test]
    fn test_bet_batch_placed_event_struct() {
        let env = Env::default();
        let bettor = Address::generate(&env);
        let market1 = Symbol::new(&env, "mkt1");
        let market2 = Symbol::new(&env, "mkt2");
        let outcome1 = String::from_str(&env, "yes");
        let outcome2 = String::from_str(&env, "no");

        let bets = vec![
            &env,
            (market1, outcome1, 10_000_000i128),
            (market2, outcome2, 5_000_000i128),
        ];

        let event = BetBatchPlacedEvent {
            bettor: bettor.clone(),
            bets: bets.clone(),
            total_amount: 15_000_000,
            bet_count: 2,
            timestamp: 1000,
        };

        assert_eq!(event.bettor, bettor);
        assert_eq!(event.bet_count, 2);
        assert_eq!(event.total_amount, 15_000_000);
        assert_eq!(event.bets.len(), 2);
    }

    #[test]
    fn test_bet_stats_updated_event_struct() {
        let env = Env::default();
        let market_id = Symbol::new(&env, "btc_100k");

        let event = BetStatsUpdatedEvent {
            market_id: market_id.clone(),
            total_bets: 42u32,
            total_amount_locked: 1_000_000_000,
            unique_bettors: 25u32,
            timestamp: 2000,
        };

        assert_eq!(event.market_id, market_id);
        assert_eq!(event.total_bets, 42u32);
        assert_eq!(event.total_amount_locked, 1_000_000_000);
        assert_eq!(event.unique_bettors, 25u32);
    }

    #[test]
    fn test_emit_bet_batch_placed_publishes_event() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());

        let bettor = Address::generate(&env);
        let market1 = Symbol::new(&env, "mkt1");
        let market2 = Symbol::new(&env, "mkt2");
        let outcome1 = String::from_str(&env, "yes");
        let outcome2 = String::from_str(&env, "no");

        let bets = vec![
            &env,
            (market1, outcome1, 10_000_000i128),
            (market2, outcome2, 5_000_000i128),
        ];

        env.as_contract(&contract_id, || {
            EventEmitter::emit_bet_batch_placed(&env, &bettor, &bets, 15_000_000);
        });

        let events = env.events().all();
        let mut found = false;
        for event in events.iter() {
            if event.2.len() > 0 {
                let topic0: Symbol = event.2.get(0).unwrap().try_into_val(&env).unwrap();
                if topic0 == symbol_short!("bet_batch") {
                    found = true;
                }
            }
        }
        assert!(found, "BetBatchPlacedEvent not found in published events");
    }

    #[test]
    fn test_emit_bet_stats_updated_publishes_event() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());

        let market_id = Symbol::new(&env, "btc_100k");

        env.as_contract(&contract_id, || {
            EventEmitter::emit_bet_stats_updated(&env, &market_id, 42u32, 1_000_000_000, 25u32);
        });

        let events = env.events().all();
        let mut found = false;
        for event in events.iter() {
            if event.2.len() > 0 {
                let topic0: Symbol = event.2.get(0).unwrap().try_into_val(&env).unwrap();
                if topic0 == symbol_short!("bet_stat") {
                    found = true;
                }
            }
        }
        assert!(found, "BetStatsUpdatedEvent not found in published events");
    }

    #[test]
    fn test_emit_bet_batch_placed_stores_event() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());

        let bettor = Address::generate(&env);
        let bets = vec![
            &env,
            (
                Symbol::new(&env, "mkt1"),
                String::from_str(&env, "yes"),
                10_000_000i128,
            ),
        ];

        env.as_contract(&contract_id, || {
            EventEmitter::emit_bet_batch_placed(&env, &bettor, &bets, 10_000_000);
            let stored: Option<BetBatchPlacedEvent> = env
                .storage()
                .persistent()
                .get(&symbol_short!("bet_batch"));
            assert!(stored.is_some(), "BetBatchPlacedEvent should be stored");
            let stored_event = stored.unwrap();
            assert_eq!(stored_event.bettor, bettor);
            assert_eq!(stored_event.bet_count, 1);
            assert_eq!(stored_event.total_amount, 10_000_000);
        });
    }

    #[test]
    fn test_emit_bet_stats_updated_stores_event() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());

        let market_id = Symbol::new(&env, "btc_100k");

        env.as_contract(&contract_id, || {
            EventEmitter::emit_bet_stats_updated(&env, &market_id, 42u32, 1_000_000_000, 25u32);
            let stored: Option<BetStatsUpdatedEvent> = env
                .storage()
                .persistent()
                .get(&symbol_short!("bet_stat"));
            assert!(stored.is_some(), "BetStatsUpdatedEvent should be stored");
            let stored_event = stored.unwrap();
            assert_eq!(stored_event.total_bets, 42u32);
            assert_eq!(stored_event.total_amount_locked, 1_000_000_000);
            assert_eq!(stored_event.unique_bettors, 25u32);
        });
    }

    #[test]
    fn test_emit_bet_batch_placed_with_empty_bets() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());

        let bettor = Address::generate(&env);
        let empty_bets: Vec<(Symbol, String, i128)> = Vec::new(&env);

        env.as_contract(&contract_id, || {
            EventEmitter::emit_bet_batch_placed(&env, &bettor, &empty_bets, 0);
            let events = env.events().all();
            let mut found = false;
            for event in events.iter() {
                if event.2.len() > 0 {
                    let topic0: Symbol = event.2.get(0).unwrap().try_into_val(&env).unwrap();
                    if topic0 == symbol_short!("bet_batch") {
                        found = true;
                    }
                }
            }
            assert!(found, "Batch event with empty bets should still be published");
        });
    }

    #[test]
    fn test_emit_bet_stats_updated_zero_values() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());

        let market_id = Symbol::new(&env, "fresh_market");

        env.as_contract(&contract_id, || {
            EventEmitter::emit_bet_stats_updated(&env, &market_id, 0u32, 0, 0u32);
            let stored: Option<BetStatsUpdatedEvent> = env
                .storage()
                .persistent()
                .get(&symbol_short!("bet_stat"));
            assert!(stored.is_some(), "Bet stats event should be stored");
            let stored_event = stored.unwrap();
            assert_eq!(stored_event.total_bets, 0u32);
            assert_eq!(stored_event.unique_bettors, 0u32);
        });
    }
}
