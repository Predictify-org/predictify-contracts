extern crate alloc;

use soroban_sdk::{contracttype, symbol_short, vec, Address, BytesN, Env, Map, String, Symbol, Vec};

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
    pub fn get_schema(env: &Env, name: &str) -> Option<EventSchemaEntry> {
        match name {
            "oracle_result" => Some(EventSchemaEntry {
                topic: symbol_short!("oracle_rs"),
                schema_version: 1,
            }),
            "dispute_opened" => Some(EventSchemaEntry {
                topic: symbol_short!("dispt_opn"),
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

    pub fn emit_market_created(
        env: &Env, market_id: &Symbol, question: &String, outcomes: &Vec<String>, admin: &Address, end_time: u64,
    ) {
        let event = MarketCreatedEvent {
            market_id: market_id.clone(),
            question: question.clone(),
            outcomes: outcomes.clone(),
            admin: admin.clone(),
            end_time,
            nonce: Self::get_and_increment_nonce(env, symbol_short!("mkt_crt")),
            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("mkt_crt"), &event);
        env.events().publish((symbol_short!("mkt_crt"), market_id.clone()), event);
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
            nonce: Self::get_and_increment_nonce(env, schema.topic),
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

    pub fn emit_oracle_median_quotes(env: &Env, market_id: &Symbol, quotes: &Vec<crate::types::OracleQuote>) {
        env.events().publish((symbol_short!("orc_med_q"), market_id.clone()), quotes.clone());
    }

    pub fn emit_market_resolved(
        env: &Env, market_id: &Symbol, final_outcome: &String, oracle_result: &String, community_consensus: &String, resolution_method: &String, confidence_score: i128,
    ) {
        let event = MarketResolvedEvent {
            market_id: market_id.clone(), final_outcome: final_outcome.clone(), oracle_result: oracle_result.clone(), community_consensus: community_consensus.clone(), resolution_method: resolution_method.clone(), confidence_score,
            nonce: Self::get_and_increment_nonce(env, symbol_short!("mkt_res")),
            timestamp: env.ledger().timestamp(),
        };
        env.storage().persistent().set(&symbol_short!("mkt_res"), &event);
        env.events().publish((symbol_short!("mkt_res"), market_id.clone(), resolution_method.clone()), event);
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
            nonce: Self::get_and_increment_nonce(env, schema.topic),
            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &schema.topic, &event);
        env.events().publish((schema.topic, market_id.clone(), schema.schema_version), event);
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

    pub fn emit_market_closed(env: &Env, market_id: &Symbol, admin: &Address) {
        let event = MarketClosedEvent {
            market_id: market_id.clone(), admin: admin.clone(),
            nonce: Self::get_and_increment_nonce(env, symbol_short!("mkt_close")),
            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("mkt_close"), &event);
        env.events().publish((symbol_short!("mkt_close"), market_id.clone()), event);
    }

    pub fn emit_refund_on_oracle_failure(env: &Env, market_id: &Symbol, total_refunded: i128) {
        let event = RefundOnOracleFailureEvent {
            market_id: market_id.clone(), total_refunded,
            nonce: Self::get_and_increment_nonce(env, symbol_short!("ref_oracl")),
            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("ref_oracl"), &event);
        env.events().publish((symbol_short!("ref_oracl"), market_id.clone()), event);
    }

    pub fn emit_state_change_event(env: &Env, market_id: &Symbol, old_state: &crate::types::MarketState, new_state: &crate::types::MarketState, reason: &String) {
        let event = StateChangeEvent {
            market_id: market_id.clone(), old_state: old_state.clone(), new_state: new_state.clone(), reason: reason.clone(),
            nonce: Self::get_and_increment_nonce(env, symbol_short!("st_chng")),
            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("st_chng"), &event);
        env.events().publish((symbol_short!("st_chng"), market_id.clone()), event);
    }

    pub fn emit_winnings_claimed(env: &Env, market_id: &Symbol, user: &Address, amount: i128) {
        let event = WinningsClaimedEvent {
            market_id: market_id.clone(), user: user.clone(), amount,
            nonce: Self::get_and_increment_nonce(env, symbol_short!("win_clm")),
            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("win_clm"), &event);
        env.events().publish((symbol_short!("win_clm"), market_id.clone()), event);
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
        data.set(String::from_str(env, "last_action"), last_action);
        data.set(String::from_str(env, "cooldown"), cooldown);
        env.events().publish(topics, data);
    }

    pub fn emit_threshold_proposed(env: &Env, admin: &Address, old_threshold: u32, new_threshold: u32, confirm_after: u64) {
        let event = MultisigThresholdProposedEvent {
            admin: admin.clone(), old_threshold, new_threshold, confirm_after,
            nonce: Self::get_and_increment_nonce(env, symbol_short!("thld_prop")),
            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("thld_prop"), &event);
        env.events().publish((symbol_short!("thld_prop"), admin.clone()), event);
    }

    pub fn emit_threshold_confirmed(env: &Env, admin: &Address, old_threshold: u32, new_threshold: u32) {
        let event = MultisigThresholdConfirmedEvent {
            admin: admin.clone(), old_threshold, new_threshold,
            nonce: Self::get_and_increment_nonce(env, symbol_short!("thld_conf")),
            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("thld_conf"), &event);
        env.events().publish((symbol_short!("thld_conf"), admin.clone()), event);
    }

    pub fn emit_signer_rotation_cooldown_hit(env: &Env, admin: &Address, last_rotation: u64, cooldown: u64) {
        let topics = (Symbol::new(env, "Admin"), Symbol::new(env, "SignerRotationCooldownHit"));
        let mut data = Map::new(env);
        data.set(String::from_str(env, "admin"), admin.to_val());
        data.set(String::from_str(env, "last_rotation"), last_rotation);
        data.set(String::from_str(env, "cooldown"), cooldown);
        env.events().publish(topics, data);
    }

    pub fn emit_market_finalized(env: &Env, market_id: &Symbol, admin: &Address, outcome: &String) {
        let event = MarketFinalizedEvent {
            market_id: market_id.clone(), admin: admin.clone(), outcome: outcome.clone(),
            nonce: Self::get_and_increment_nonce(env, symbol_short!("mkt_final")),
            timestamp: env.ledger().timestamp(),
        };
        Self::store_event(env, &symbol_short!("mkt_final"), &event);
        env.events().publish((symbol_short!("mkt_final"), market_id.clone()), event);
    }

    pub fn emit_admin_broadcast(env: &Env, severity: Severity, message_hash: BytesN<32>, reason: String) {
        let event = AdminBroadcastEvent {
            severity, message_hash, reason,
            nonce: Self::get_and_increment_nonce(env, symbol_short!("adm_cast")),
            timestamp: env.ledger().timestamp(),
        };
        env.events().publish((Symbol::new(env, "admin_broadcast"),), event);
    }

    pub fn emit_monitor_queue_overflow(env: &Env, overflow_count: u64, evicted_event_id: Option<Symbol>, capacity: u32) {
        env.events().publish((symbol_short!("mon_ovf"),), (overflow_count, evicted_event_id, capacity, env.ledger().timestamp()));
    }

    pub fn emit_balance_changed(env: &Env, user: &Address, asset: &crate::types::ReflectorAsset, operation: &String, amount: i128, new_balance: i128) {
        env.events().publish(
            (symbol_short!("bal_chg"), user, asset.clone()),
            (operation.clone(), amount, new_balance, env.ledger().timestamp()),
        );
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

    pub fn emit_error_logged(env: &Env, error_code: u32, message: &String, context: &String, user: Option<Address>, market_id: Option<Symbol>) {
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
}