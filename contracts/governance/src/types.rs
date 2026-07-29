//! Shared data types for the Governance contract.
//!
//! These types are part of the contract's public API and are exported so that
//! clients and off-chain indexers can decode stored state and event payloads.

use soroban_sdk::{contracttype, Address, String};

/// Lifecycle state of a governance proposal.
///
/// A proposal always begins in [`ProposalStatus::Active`] and transitions to
/// exactly one terminal state. The numeric discriminants are stable and form
/// part of the contract's public API — they must not be reordered.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProposalStatus {
    /// The proposal is open for voting.
    Active = 0,
    /// Voting closed with more `for` than `against` votes and the proposal
    /// was executed.
    Executed = 1,
    /// Voting closed without a passing majority.
    Rejected = 2,
    /// The proposer (or admin) withdrew the proposal before finalization.
    Canceled = 3,
}

/// Direction of a cast vote.
///
/// The numeric discriminants are stable and part of the public API.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VoteChoice {
    /// A vote against the proposal.
    Against = 0,
    /// A vote in favor of the proposal.
    For = 1,
}

/// A governance proposal record persisted in contract storage.
///
/// The full proposal is emitted as part of lifecycle events so that indexers
/// can reconstruct proposal state without additional reads.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Proposal {
    /// Monotonic identifier, unique per contract instance.
    pub id: u64,
    /// Account that created the proposal.
    pub proposer: Address,
    /// Human-readable title / summary of the proposal.
    pub title: String,
    /// Current lifecycle state.
    pub status: ProposalStatus,
    /// Cumulative weight of `for` votes.
    pub votes_for: u64,
    /// Cumulative weight of `against` votes.
    pub votes_against: u64,
    /// Ledger timestamp (seconds) at which the proposal was created.
    pub created_at: u64,
    /// Ledger timestamp (seconds) after which voting is closed (exclusive).
    pub voting_ends_at: u64,
}
