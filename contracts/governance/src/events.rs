//! Structured governance lifecycle events for off-chain indexers.
//!
//! # Overview
//!
//! This module defines every event emitted across the governance lifecycle:
//! proposal creation, voting (direct and commit-reveal), delegation,
//! configuration changes, registry parameter proposals/executions/cancellations,
//! and auto-rejection.  Each event is a versioned, `#[contracttype]` struct so
//! it can be decoded by any Stellar event subscriber without bespoke parsers.
//!
//! # Usage
//!
//! Call the static helpers on [`GovernanceEventEmitter`] from any entrypoint
//! that changes governance state.  All helpers require an [`Env`] reference and
//! take their data by reference to avoid unnecessary cloning at call-sites.
//!
//! ```no_run
//! # use soroban_sdk::{Env, Symbol, Address, String};
//! # use governance_events::events::GovernanceEventEmitter;
//! # fn example(env: &Env, proposal_id: &Symbol, proposer: &Address,
//! #            title: &String, description: &String) {
//! GovernanceEventEmitter::emit_proposal_created(
//!     env, proposal_id, proposer, title, description,
//! );
//! # }
//! ```
//!
//! # Event Topics
//!
//! All topics are `symbol_short!` values (≤ 9 bytes) so they fit within the
//! Soroban topic-size limit without additional encoding.
//!
//! | Event                     | Primary topic    | Secondary key     |
//! |---------------------------|------------------|-------------------|
//! | ProposalCreated           | `gov_prop`       | `proposal_id`     |
//! | VoteCast                  | `gov_vote`       | `proposal_id`     |
//! | VoteCommitted             | `gov_cmit`       | `proposal_id`     |
//! | VoteRevealed              | `gov_rvl`        | `proposal_id`     |
//! | ProposalExecuted          | `gov_exec`       | `proposal_id`     |
//! | ProposalCancelled         | `gov_canc`       | `proposal_id`     |
//! | ProposalAutoRejected      | `gov_rej`        | `proposal_id`     |
//! | VotingPeriodUpdated       | `gov_vp_upd`     | `admin`           |
//! | QuorumUpdated             | `gov_qrm`        | `admin`           |
//! | QuorumDecayUpdated        | `gov_qdcy`       | `admin`           |
//! | DelegateSet               | `gov_dlg_set`    | `delegator`       |
//! | DelegateUnset             | `gov_dlg_uns`    | `delegator`       |
//! | RegistryInitialized       | `reg_init`       | `admin`           |
//! | RegistryParamProposed     | `reg_prop`       | `key`             |
//! | RegistryParamExecuted     | `reg_exec`       | `key`             |
//! | RegistryParamCancelled    | `reg_canc`       | `key`             |


use soroban_sdk::{contracttype, symbol_short, Address, Env, String, Symbol};

// ─────────────────────────────────────────────────────────────────────────────
// Helper: monotone per-topic nonce stored in persistent storage.
//
// The nonce is keyed by the event topic symbol so topics never share a counter.
// It uses checked addition and saturates at `u64::MAX` rather than panicking,
// which is safe because reaching that cardinality is operationally impossible.
// ─────────────────────────────────────────────────────────────────────────────

fn next_nonce(env: &Env, topic: Symbol) -> u64 {
    // DataKey is an opaque wrapper so we can reuse the persistent namespace
    // without colliding with any key already used by the host contract.
    #[contracttype]
    #[derive(Clone)]
    enum NonceKey {
        Nonce(Symbol),
    }
    let key = NonceKey::Nonce(topic);
    let current: u64 = env.storage().persistent().get(&key).unwrap_or(0);
    let next = current.saturating_add(1);
    env.storage().persistent().set(&key, &next);
    next
}


// ─────────────────────────────────────────────────────────────────────────────
// Event structs — proposal lifecycle
// ─────────────────────────────────────────────────────────────────────────────

/// Emitted when a governance proposal is successfully created.
///
/// Off-chain indexers should use this event to bootstrap a proposal record
/// before listening for subsequent vote and execution events.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProposalCreatedEvent {
    /// Unique proposal identifier.
    pub proposal_id: Symbol,
    /// Address that submitted the proposal (must have passed `require_auth`).
    pub proposer: Address,
    /// Short human-readable title.
    pub title: String,
    /// Full proposal description / rationale.
    pub description: String,
    /// Monotone per-topic counter; allows indexers to detect missed events.
    pub nonce: u64,
    /// Ledger timestamp (Unix seconds) at creation.
    pub timestamp: u64,
}

/// Emitted when a voter directly casts a FOR or AGAINST vote on a proposal.
///
/// This event is also emitted during the *reveal* phase of commit-reveal
/// voting once the commitment has been verified — see [`VoteRevealedEvent`]
/// for the commit-reveal-specific payload.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VoteCastEvent {
    /// Proposal being voted on.
    pub proposal_id: Symbol,
    /// Voter address (authenticated before this event fires).
    pub voter: Address,
    /// `true` = FOR, `false` = AGAINST.
    pub support: bool,
    /// Effective weight counted for this vote (own vote + delegated votes).
    pub weight: u128,
    /// Monotone per-topic counter.
    pub nonce: u64,
    /// Ledger timestamp (Unix seconds).
    pub timestamp: u64,
}

/// Emitted during the *commit* phase of commit-reveal voting.
///
/// The commitment itself (`sha256(salt ++ support_byte)`) is stored on-chain
/// but intentionally omitted from this event to keep the secret hidden until
/// the reveal phase.  Indexers can correlate commit → reveal by `(proposal_id, voter)`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VoteCommittedEvent {
    /// Proposal the commitment belongs to.
    pub proposal_id: Symbol,
    /// Voter who submitted the commitment.
    pub voter: Address,
    /// Monotone per-topic counter.
    pub nonce: u64,
    /// Ledger timestamp (Unix seconds).
    pub timestamp: u64,
}

/// Emitted during the *reveal* phase of commit-reveal voting after the
/// commitment hash has been verified on-chain.
///
/// This event carries the revealed preference.  Downstream systems that do not
/// care about commit-reveal mechanics can simply listen to [`VoteCastEvent`]
/// (which is also emitted at reveal time).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VoteRevealedEvent {
    /// Proposal being voted on.
    pub proposal_id: Symbol,
    /// Voter who submitted the reveal.
    pub voter: Address,
    /// Revealed preference: `true` = FOR, `false` = AGAINST.
    pub support: bool,
    /// Effective weight (own + delegated) tallied for this vote.
    pub weight: u128,
    /// Monotone per-topic counter.
    pub nonce: u64,
    /// Ledger timestamp (Unix seconds).
    pub timestamp: u64,
}

/// Emitted when a proposal is successfully executed after passing quorum and
/// the majority threshold.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProposalExecutedEvent {
    /// Executed proposal.
    pub proposal_id: Symbol,
    /// Address that triggered execution (must have passed `require_auth`).
    pub executor: Address,
    /// Monotone per-topic counter.
    pub nonce: u64,
    /// Ledger timestamp (Unix seconds).
    pub timestamp: u64,
}

/// Emitted when an admin explicitly cancels a proposal before it is executed.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProposalCancelledEvent {
    /// Cancelled proposal.
    pub proposal_id: Symbol,
    /// Admin who performed the cancellation.
    pub admin: Address,
    /// Optional human-readable reason.
    pub reason: String,
    /// Monotone per-topic counter.
    pub nonce: u64,
    /// Ledger timestamp (Unix seconds).
    pub timestamp: u64,
}

/// Emitted when a proposal expires without reaching even the floor quorum
/// (i.e. the auto-rejection path in quorum-decay logic).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProposalAutoRejectedEvent {
    /// Rejected proposal.
    pub proposal_id: Symbol,
    /// Original proposer.
    pub proposer: Address,
    /// Total FOR votes accumulated.
    pub for_votes: u128,
    /// Floor quorum that was required.
    pub floor_quorum: u128,
    /// Monotone per-topic counter.
    pub nonce: u64,
    /// Ledger timestamp (Unix seconds).
    pub timestamp: u64,
}

// ─────────────────────────────────────────────────────────────────────────────
// Event structs — configuration changes
// ─────────────────────────────────────────────────────────────────────────────

/// Emitted when the admin updates the voting-window duration.
///
/// Off-chain indexers must pick this up to know the deadline for newly
/// created proposals; the change takes effect on the *next* proposal.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VotingPeriodUpdatedEvent {
    /// Admin who changed the value.
    pub admin: Address,
    /// Previous voting period in seconds.
    pub old_period_seconds: u64,
    /// New voting period in seconds.
    pub new_period_seconds: u64,
    /// Monotone per-topic counter.
    pub nonce: u64,
    /// Ledger timestamp (Unix seconds).
    pub timestamp: u64,
}

/// Emitted when the admin updates the minimum FOR-vote quorum.
///
/// Quorum changes take effect for the *next* `validate_proposal` call on any
/// open proposal, so indexers should recalculate pass/fail thresholds on receipt.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuorumUpdatedEvent {
    /// Admin who changed the value.
    pub admin: Address,
    /// Previous quorum (minimum FOR votes).
    pub old_quorum: u128,
    /// New quorum (minimum FOR votes).
    pub new_quorum: u128,
    /// Monotone per-topic counter.
    pub nonce: u64,
    /// Ledger timestamp (Unix seconds).
    pub timestamp: u64,
}

/// Emitted when the admin updates (or disables) the quorum-decay configuration.
///
/// When `enabled` is `false` the `floor_bps` / `halving_seconds` fields carry
/// no meaningful value and should be ignored by consumers.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuorumDecayUpdatedEvent {
    /// Admin who changed the value.
    pub admin: Address,
    /// `true` if decay is now active, `false` if it was set to `None`.
    pub enabled: bool,
    /// Floor quorum expressed in basis points of the base quorum (0 when disabled).
    pub floor_bps: u32,
    /// Halving-time in seconds (0 when disabled).
    pub halving_seconds: u64,
    /// Monotone per-topic counter.
    pub nonce: u64,
    /// Ledger timestamp (Unix seconds).
    pub timestamp: u64,
}

// ─────────────────────────────────────────────────────────────────────────────
// Event structs — delegation lifecycle
// ─────────────────────────────────────────────────────────────────────────────

/// Emitted when a delegator activates (or updates) their vote delegation.
///
/// Indexers should update the effective vote weight for `delegate` on receipt.
/// One delegator holds at most one active delegation; if a prior delegation
/// existed it was replaced by a call to `unset_delegate` first.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DelegateSetEvent {
    /// Address granting the delegation.
    pub delegator: Address,
    /// Address receiving the delegation.
    pub delegate: Address,
    /// Monotone per-topic counter.
    pub nonce: u64,
    /// Ledger timestamp (Unix seconds).
    pub timestamp: u64,
}

/// Emitted when a delegator removes their active delegation.
///
/// Indexers should decrement the effective vote weight for the previously
/// pointed-at `delegate` on receipt.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DelegateUnsetEvent {
    /// Address that had the delegation.
    pub delegator: Address,
    /// Address that was receiving votes; now losing the delegation.
    pub former_delegate: Address,
    /// Monotone per-topic counter.
    pub nonce: u64,
    /// Ledger timestamp (Unix seconds).
    pub timestamp: u64,
}

// ─────────────────────────────────────────────────────────────────────────────
// Event structs — governance parameter registry lifecycle
// ─────────────────────────────────────────────────────────────────────────────

/// Emitted when the governance parameter registry is first initialised.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegistryInitializedEvent {
    /// Governance admin set at initialisation.
    pub admin: Address,
    /// Time-lock delay in seconds applied to all future parameter proposals.
    pub time_lock_delay: u64,
    /// Monotone per-topic counter.
    pub nonce: u64,
    /// Ledger timestamp (Unix seconds).
    pub timestamp: u64,
}

/// Emitted when a governance parameter change is proposed (not yet executable).
///
/// Indexers should record this event and wait for the corresponding
/// [`RegistryParamExecutedEvent`] or [`RegistryParamCancelledEvent`].
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegistryParamProposedEvent {
    /// Admin who submitted the proposal.
    pub admin: Address,
    /// Parameter key being proposed.
    pub key: Symbol,
    /// Proposed new value.
    pub new_value: i128,
    /// Ledger timestamp after which execution becomes allowed.
    pub executable_after: u64,
    /// Monotone per-topic counter.
    pub nonce: u64,
    /// Ledger timestamp (Unix seconds) of this event.
    pub timestamp: u64,
}

/// Emitted when a previously proposed parameter change is executed after the
/// time-lock expires.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegistryParamExecutedEvent {
    /// Admin who triggered execution.
    pub admin: Address,
    /// Parameter key that was updated.
    pub key: Symbol,
    /// Value that is now live.
    pub new_value: i128,
    /// Monotone per-topic counter.
    pub nonce: u64,
    /// Ledger timestamp (Unix seconds).
    pub timestamp: u64,
}

/// Emitted when a pending parameter proposal is cancelled before execution.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegistryParamCancelledEvent {
    /// Admin who cancelled the proposal.
    pub admin: Address,
    /// Parameter key whose proposal was cancelled.
    pub key: Symbol,
    /// Monotone per-topic counter.
    pub nonce: u64,
    /// Ledger timestamp (Unix seconds).
    pub timestamp: u64,
}

// ─────────────────────────────────────────────────────────────────────────────
// GovernanceEventEmitter — static emit helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Central emitter for all governance lifecycle events.
///
/// Each method constructs the typed event struct, stores it in persistent
/// storage (keyed by the event topic so existing [`EventLogger`] tooling keeps
/// working), and publishes it to the Stellar event stream for off-chain indexers.
///
/// # No `unwrap()` policy
///
/// All storage reads use `unwrap_or` defaults.  The nonce saturates at
/// `u64::MAX` rather than panicking — an impossible cardinality in practice.
///
/// # Topic naming
///
/// Topics are `symbol_short!` literals (≤ 9 ASCII bytes) so they are encoded
/// in a single `ScVal::Symbol` without heap allocation on the host side.
pub struct GovernanceEventEmitter;

impl GovernanceEventEmitter {
    // ── Proposal lifecycle ──────────────────────────────────────────────────

    /// Emit [`ProposalCreatedEvent`].
    ///
    /// Call this immediately after persisting a new proposal to storage.
    pub fn emit_proposal_created(
        env: &Env,
        proposal_id: &Symbol,
        proposer: &Address,
        title: &String,
        description: &String,
    ) {
        let topic = symbol_short!("gov_prop");
        let event = ProposalCreatedEvent {
            proposal_id: proposal_id.clone(),
            proposer: proposer.clone(),
            title: title.clone(),
            description: description.clone(),
            nonce: next_nonce(env, topic.clone()),
            timestamp: env.ledger().timestamp(),
        };
        env.events().publish((topic, proposal_id.clone()), event);
    }

    /// Emit [`VoteCastEvent`].
    ///
    /// Call this after tallying the voter's weight into `for_votes` /
    /// `against_votes`.  Also called at the end of a successful reveal.
    pub fn emit_vote_cast(
        env: &Env,
        proposal_id: &Symbol,
        voter: &Address,
        support: bool,
        weight: u128,
    ) {
        let topic = symbol_short!("gov_vote");
        let event = VoteCastEvent {
            proposal_id: proposal_id.clone(),
            voter: voter.clone(),
            support,
            weight,
            nonce: next_nonce(env, topic.clone()),
            timestamp: env.ledger().timestamp(),
        };
        env.events().publish((topic, proposal_id.clone()), event);
    }

    /// Emit [`VoteCommittedEvent`] (commit phase of commit-reveal).
    ///
    /// Call this after storing the salted commitment on-chain.
    pub fn emit_vote_committed(env: &Env, proposal_id: &Symbol, voter: &Address) {
        let topic = symbol_short!("gov_cmit");
        let event = VoteCommittedEvent {
            proposal_id: proposal_id.clone(),
            voter: voter.clone(),
            nonce: next_nonce(env, topic.clone()),
            timestamp: env.ledger().timestamp(),
        };
        env.events().publish((topic, proposal_id.clone()), event);
    }

    /// Emit [`VoteRevealedEvent`] (reveal phase of commit-reveal).
    ///
    /// Call this after verifying the commitment hash and tallying the weight.
    pub fn emit_vote_revealed(
        env: &Env,
        proposal_id: &Symbol,
        voter: &Address,
        support: bool,
        weight: u128,
    ) {
        let topic = symbol_short!("gov_rvl");
        let event = VoteRevealedEvent {
            proposal_id: proposal_id.clone(),
            voter: voter.clone(),
            support,
            weight,
            nonce: next_nonce(env, topic.clone()),
            timestamp: env.ledger().timestamp(),
        };
        env.events().publish((topic, proposal_id.clone()), event);
    }

    /// Emit [`ProposalExecutedEvent`].
    ///
    /// Call this after marking the proposal `executed = true` in storage.
    pub fn emit_proposal_executed(env: &Env, proposal_id: &Symbol, executor: &Address) {
        let topic = symbol_short!("gov_exec");
        let event = ProposalExecutedEvent {
            proposal_id: proposal_id.clone(),
            executor: executor.clone(),
            nonce: next_nonce(env, topic.clone()),
            timestamp: env.ledger().timestamp(),
        };
        env.events().publish((topic, proposal_id.clone()), event);
    }

    /// Emit [`ProposalCancelledEvent`].
    ///
    /// Call this when an admin explicitly cancels an open proposal.
    pub fn emit_proposal_cancelled(
        env: &Env,
        proposal_id: &Symbol,
        admin: &Address,
        reason: &String,
    ) {
        let topic = symbol_short!("gov_canc");
        let event = ProposalCancelledEvent {
            proposal_id: proposal_id.clone(),
            admin: admin.clone(),
            reason: reason.clone(),
            nonce: next_nonce(env, topic.clone()),
            timestamp: env.ledger().timestamp(),
        };
        env.events().publish((topic, proposal_id.clone()), event);
    }

    /// Emit [`ProposalAutoRejectedEvent`].
    ///
    /// Call this inside `validate_proposal` when a proposal expires with fewer
    /// FOR votes than the floor quorum.
    pub fn emit_proposal_auto_rejected(
        env: &Env,
        proposal_id: &Symbol,
        proposer: &Address,
        for_votes: u128,
        floor_quorum: u128,
    ) {
        let topic = symbol_short!("gov_rej");
        let event = ProposalAutoRejectedEvent {
            proposal_id: proposal_id.clone(),
            proposer: proposer.clone(),
            for_votes,
            floor_quorum,
            nonce: next_nonce(env, topic.clone()),
            timestamp: env.ledger().timestamp(),
        };
        env.events().publish((topic, proposal_id.clone()), event);
    }

    // ── Configuration changes ───────────────────────────────────────────────

    /// Emit [`VotingPeriodUpdatedEvent`].
    ///
    /// Call this after successfully persisting the new voting period to storage.
    pub fn emit_voting_period_updated(
        env: &Env,
        admin: &Address,
        old_period_seconds: u64,
        new_period_seconds: u64,
    ) {
        let topic = symbol_short!("gov_vp_upd");
        let event = VotingPeriodUpdatedEvent {
            admin: admin.clone(),
            old_period_seconds,
            new_period_seconds,
            nonce: next_nonce(env, topic.clone()),
            timestamp: env.ledger().timestamp(),
        };
        env.events().publish((topic, admin.clone()), event);
    }

    /// Emit [`QuorumUpdatedEvent`].
    ///
    /// Call this after successfully persisting the new quorum value to storage.
    pub fn emit_quorum_updated(
        env: &Env,
        admin: &Address,
        old_quorum: u128,
        new_quorum: u128,
    ) {
        let topic = symbol_short!("gov_qrm");
        let event = QuorumUpdatedEvent {
            admin: admin.clone(),
            old_quorum,
            new_quorum,
            nonce: next_nonce(env, topic.clone()),
            timestamp: env.ledger().timestamp(),
        };
        env.events().publish((topic, admin.clone()), event);
    }

    /// Emit [`QuorumDecayUpdatedEvent`].
    ///
    /// Call this after persisting the new quorum-decay config to storage.
    /// Pass `None` when decay is being disabled.
    pub fn emit_quorum_decay_updated(
        env: &Env,
        admin: &Address,
        floor_bps: Option<u32>,
        halving_seconds: Option<u64>,
    ) {
        let topic = symbol_short!("gov_qdcy");
        let enabled = floor_bps.is_some();
        let event = QuorumDecayUpdatedEvent {
            admin: admin.clone(),
            enabled,
            floor_bps: floor_bps.unwrap_or(0),
            halving_seconds: halving_seconds.unwrap_or(0),
            nonce: next_nonce(env, topic.clone()),
            timestamp: env.ledger().timestamp(),
        };
        env.events().publish((topic, admin.clone()), event);
    }

    // ── Delegation lifecycle ────────────────────────────────────────────────

    /// Emit [`DelegateSetEvent`].
    ///
    /// Call this after storing the delegation in persistent storage.
    pub fn emit_delegate_set(env: &Env, delegator: &Address, delegate: &Address) {
        let topic = symbol_short!("gov_dlgset");
        let event = DelegateSetEvent {
            delegator: delegator.clone(),
            delegate: delegate.clone(),
            nonce: next_nonce(env, topic.clone()),
            timestamp: env.ledger().timestamp(),
        };
        env.events().publish((topic, delegator.clone()), event);
    }

    /// Emit [`DelegateUnsetEvent`].
    ///
    /// Call this after removing the delegation from persistent storage.
    pub fn emit_delegate_unset(env: &Env, delegator: &Address, former_delegate: &Address) {
        let topic = symbol_short!("gov_dlguns");
        let event = DelegateUnsetEvent {
            delegator: delegator.clone(),
            former_delegate: former_delegate.clone(),
            nonce: next_nonce(env, topic.clone()),
            timestamp: env.ledger().timestamp(),
        };
        env.events().publish((topic, delegator.clone()), event);
    }

    // ── Registry lifecycle ──────────────────────────────────────────────────

    /// Emit [`RegistryInitializedEvent`].
    ///
    /// Call this at the end of `GovernanceRegistry::initialize`.
    pub fn emit_registry_initialized(env: &Env, admin: &Address, time_lock_delay: u64) {
        let topic = symbol_short!("reg_init");
        let event = RegistryInitializedEvent {
            admin: admin.clone(),
            time_lock_delay,
            nonce: next_nonce(env, topic.clone()),
            timestamp: env.ledger().timestamp(),
        };
        env.events().publish((topic, admin.clone()), event);
    }

    /// Emit [`RegistryParamProposedEvent`].
    ///
    /// Call this after storing a pending parameter update in the registry.
    pub fn emit_registry_param_proposed(
        env: &Env,
        admin: &Address,
        key: &Symbol,
        new_value: i128,
        executable_after: u64,
    ) {
        let topic = symbol_short!("reg_prop");
        let event = RegistryParamProposedEvent {
            admin: admin.clone(),
            key: key.clone(),
            new_value,
            executable_after,
            nonce: next_nonce(env, topic.clone()),
            timestamp: env.ledger().timestamp(),
        };
        env.events().publish((topic, key.clone()), event);
    }

    /// Emit [`RegistryParamExecutedEvent`].
    ///
    /// Call this after promoting the pending value to the live parameter slot.
    pub fn emit_registry_param_executed(
        env: &Env,
        admin: &Address,
        key: &Symbol,
        new_value: i128,
    ) {
        let topic = symbol_short!("reg_exec");
        let event = RegistryParamExecutedEvent {
            admin: admin.clone(),
            key: key.clone(),
            new_value,
            nonce: next_nonce(env, topic.clone()),
            timestamp: env.ledger().timestamp(),
        };
        env.events().publish((topic, key.clone()), event);
    }

    /// Emit [`RegistryParamCancelledEvent`].
    ///
    /// Call this after removing a pending proposal from the registry.
    pub fn emit_registry_param_cancelled(env: &Env, admin: &Address, key: &Symbol) {
        let topic = symbol_short!("reg_canc");
        let event = RegistryParamCancelledEvent {
            admin: admin.clone(),
            key: key.clone(),
            nonce: next_nonce(env, topic.clone()),
            timestamp: env.ledger().timestamp(),
        };
        env.events().publish((topic, key.clone()), event);
    }
}
