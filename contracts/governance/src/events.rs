//! Structured lifecycle events for the Governance contract.
//!
//! Every governance lifecycle transition emits a typed event with a **stable
//! topic symbol** so that off-chain indexers can filter, decode, and replay
//! governance state transitions deterministically.
//!
//! # Design
//!
//! - The first topic of every event is a stable [`Symbol`]; these strings are
//!   part of the contract's public API and must never change.
//! - Identifying fields (actor [`Address`], proposal id) are placed in
//!   **topics** so indexers can subscribe by key without decoding the payload.
//! - Quantitative fields (vote tallies, timestamps) are placed in the **data**
//!   payload.
//! - In addition to the action-specific event, every change to a proposal's
//!   [`ProposalStatus`] also emits a generic [`emit_status_changed`] event
//!   carrying `(old, new)`. Indexers that only track lifecycle state can rely
//!   on this single event stream regardless of which entrypoint caused the
//!   transition.
//!
//! # Event Summary
//!
//! | Topic Symbol    | Emitted When                          |
//! |-----------------|---------------------------------------|
//! | `gov_init`      | Contract is initialized               |
//! | `gov_created`   | A new proposal is created             |
//! | `gov_voted`     | A vote is cast on a proposal          |
//! | `gov_executed`  | A passing proposal is executed        |
//! | `gov_rejected`  | A proposal is finalized as rejected   |
//! | `gov_canceled`  | A proposal is canceled                |
//! | `gov_status`    | A proposal's status changes (generic) |
//! | `gov_admin_xf`  | Admin/ownership is transferred        |

use soroban_sdk::{Address, Env, String, Symbol};

use crate::types::{ProposalStatus, VoteChoice};

/// Emit a `GovernanceInitialized` event.
///
/// Published once when the contract is initialized via
/// [`crate::GovernanceContract::initialize`].
///
/// * Topics: `(gov_init, admin)`
/// * Data: `(voting_period, timestamp)`
pub fn emit_initialized(env: &Env, admin: &Address, voting_period: u64) {
    let topics = (Symbol::new(env, "gov_init"), admin);
    env.events()
        .publish(topics, (voting_period, env.ledger().timestamp()));
}

/// Emit a `ProposalCreated` event.
///
/// Published when a new proposal is created via
/// [`crate::GovernanceContract::create_proposal`].
///
/// * Topics: `(gov_created, proposer, proposal_id)`
/// * Data: `(title, voting_ends_at, timestamp)`
pub fn emit_proposal_created(
    env: &Env,
    proposer: &Address,
    proposal_id: u64,
    title: &String,
    voting_ends_at: u64,
) {
    let topics = (Symbol::new(env, "gov_created"), proposer, proposal_id);
    env.events().publish(
        topics,
        (title.clone(), voting_ends_at, env.ledger().timestamp()),
    );
}

/// Emit a `VoteCast` event.
///
/// Published when a vote is recorded via
/// [`crate::GovernanceContract::cast_vote`]. The running tallies are included
/// so indexers can track the vote outcome without additional reads.
///
/// * Topics: `(gov_voted, voter, proposal_id)`
/// * Data: `(choice, weight, votes_for, votes_against, timestamp)`
pub fn emit_vote_cast(
    env: &Env,
    voter: &Address,
    proposal_id: u64,
    choice: VoteChoice,
    weight: u64,
    votes_for: u64,
    votes_against: u64,
) {
    let topics = (Symbol::new(env, "gov_voted"), voter, proposal_id);
    env.events().publish(
        topics,
        (
            choice,
            weight,
            votes_for,
            votes_against,
            env.ledger().timestamp(),
        ),
    );
}

/// Emit a `ProposalExecuted` event.
///
/// Published when a passing proposal is executed via
/// [`crate::GovernanceContract::execute_proposal`].
///
/// * Topics: `(gov_executed, executor, proposal_id)`
/// * Data: `(votes_for, votes_against, timestamp)`
pub fn emit_proposal_executed(
    env: &Env,
    executor: &Address,
    proposal_id: u64,
    votes_for: u64,
    votes_against: u64,
) {
    let topics = (Symbol::new(env, "gov_executed"), executor, proposal_id);
    env.events()
        .publish(topics, (votes_for, votes_against, env.ledger().timestamp()));
}

/// Emit a `ProposalRejected` event.
///
/// Published when a proposal is finalized without a passing majority via
/// [`crate::GovernanceContract::execute_proposal`].
///
/// * Topics: `(gov_rejected, proposal_id)`
/// * Data: `(votes_for, votes_against, timestamp)`
pub fn emit_proposal_rejected(
    env: &Env,
    proposal_id: u64,
    votes_for: u64,
    votes_against: u64,
) {
    let topics = (Symbol::new(env, "gov_rejected"), proposal_id);
    env.events()
        .publish(topics, (votes_for, votes_against, env.ledger().timestamp()));
}

/// Emit a `ProposalCanceled` event.
///
/// Published when a proposal is canceled via
/// [`crate::GovernanceContract::cancel_proposal`].
///
/// * Topics: `(gov_canceled, caller, proposal_id)`
/// * Data: `timestamp`
pub fn emit_proposal_canceled(env: &Env, caller: &Address, proposal_id: u64) {
    let topics = (Symbol::new(env, "gov_canceled"), caller, proposal_id);
    env.events().publish(topics, env.ledger().timestamp());
}

/// Emit a generic `ProposalStatusChanged` event.
///
/// Emitted on **every** transition of a proposal's [`ProposalStatus`], in
/// addition to the action-specific event. Indexers that only need to track
/// lifecycle state can subscribe to this single topic.
///
/// * Topics: `(gov_status, proposal_id)`
/// * Data: `(old_status, new_status, timestamp)`
pub fn emit_status_changed(
    env: &Env,
    proposal_id: u64,
    old_status: ProposalStatus,
    new_status: ProposalStatus,
) {
    let topics = (Symbol::new(env, "gov_status"), proposal_id);
    env.events()
        .publish(topics, (old_status, new_status, env.ledger().timestamp()));
}

/// Emit an `AdminTransferred` event.
///
/// Published when governance ownership is transferred via
/// [`crate::GovernanceContract::transfer_admin`].
///
/// * Topics: `(gov_admin_xf, previous_admin, new_admin)`
/// * Data: `timestamp`
pub fn emit_admin_transferred(env: &Env, previous_admin: &Address, new_admin: &Address) {
    let topics = (Symbol::new(env, "gov_admin_xf"), previous_admin, new_admin);
    env.events().publish(topics, env.ledger().timestamp());
}
