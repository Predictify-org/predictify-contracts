//! Error types for the Governance contract.
//!
//! All state-changing entrypoints return typed errors via `#[contracterror]`.
//! Each variant carries a stable integer code that forms part of the
//! contract's public API surface and must not be reused or reordered.

use soroban_sdk::contracterror;

/// Errors returned by the Governance contract.
///
/// Each variant is assigned a stable integer code. Off-chain consumers may
/// rely on these codes; existing codes must never be reassigned.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum GovernanceError {
    /// Caller is not authorized for the action.
    Unauthorized = 1,
    /// Contract has not been initialized.
    NotInitialized = 2,
    /// Contract has already been initialized.
    AlreadyInitialized = 3,
    /// The referenced proposal does not exist.
    ProposalNotFound = 4,
    /// The proposal is not in a state that permits this transition.
    InvalidStateTransition = 5,
    /// The account has already voted on this proposal.
    AlreadyVoted = 6,
    /// The voting window for the proposal is still open.
    VotingOpen = 7,
    /// The voting window for the proposal has closed.
    VotingClosed = 8,
    /// The provided voting period is invalid (e.g. zero).
    InvalidVotingPeriod = 9,
    /// Arithmetic overflow occurred.
    Overflow = 10,
}
