//! Governance smart contract with structured lifecycle events.
//!
//! Implements a minimal, self-contained on-chain governance module: accounts
//! create proposals, cast weighted votes during a bounded voting window, and
//! proposals are finalized (executed or rejected) once voting closes. Every
//! lifecycle transition emits a typed, structured event with a stable topic
//! symbol for off-chain indexers — see [`events`].
//!
//! # Lifecycle
//!
//! ```text
//!                       cast_vote (many)
//!                      ┌──────────────┐
//!                      ▼              │
//! create_proposal → Active ──────────┘
//!                      │
//!        ┌─────────────┼──────────────┐
//!        ▼ execute     ▼ execute       ▼ cancel
//!    Executed       Rejected        Canceled
//! ```
//!
//! A proposal starts [`Active`](types::ProposalStatus::Active) and moves to
//! exactly one terminal state: `Executed`, `Rejected`, or `Canceled`.
//!
//! # Authorization
//!
//! - `initialize` — the initial admin must `require_auth()`.
//! - `create_proposal` — the proposer must `require_auth()`.
//! - `cast_vote` — the voter must `require_auth()`.
//! - `cancel_proposal` — the original proposer **or** the admin.
//! - `execute_proposal` — any authenticated caller (permissionless
//!   finalization), but only after the voting window has closed.
//! - `transfer_admin` — the current admin must `require_auth()`.
//! - Read-only entrypoints require no authentication.
//!
//! # Event Topics
//!
//! | Topic          | Description                          |
//! |----------------|--------------------------------------|
//! | `gov_init`     | Contract initialized                 |
//! | `gov_created`  | Proposal created                     |
//! | `gov_voted`    | Vote cast                            |
//! | `gov_executed` | Proposal executed                    |
//! | `gov_rejected` | Proposal rejected                    |
//! | `gov_canceled` | Proposal canceled                    |
//! | `gov_status`   | Proposal status changed (generic)    |
//! | `gov_admin_xf` | Admin transferred                    |

#![no_std]

mod err;
mod events;
mod types;

pub use err::GovernanceError;
pub use types::{Proposal, ProposalStatus, VoteChoice};

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Minimum remaining ledgers before a proposal entry is bumped on write.
const PROPOSAL_TTL_THRESHOLD: u32 = 100_000;
/// Extended TTL ledgers for proposal entries.
const PROPOSAL_TTL_TO: u32 = 518_400;

// ---------------------------------------------------------------------------
// Storage keys
// ---------------------------------------------------------------------------

/// Persistent / instance storage keys used by the Governance contract.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// Whether the contract has been initialized (`bool`).
    Initialized,
    /// The current admin address (`Address`).
    Admin,
    /// Default voting period in seconds applied to new proposals (`u64`).
    VotingPeriod,
    /// Monotonic counter for the next proposal id (`u64`).
    NextId,
    /// A stored proposal, keyed by id (`Proposal`).
    Proposal(u64),
    /// Whether `(proposal_id, voter)` has already voted (`bool`).
    HasVoted(u64, Address),
}

// ---------------------------------------------------------------------------
// Contract
// ---------------------------------------------------------------------------

#[contract]
pub struct GovernanceContract;

#[contractimpl]
impl GovernanceContract {
    // =======================================================================
    // Initialization
    // =======================================================================

    /// Initialize the contract with an `admin` and a default `voting_period`
    /// (in seconds) applied to newly created proposals.
    ///
    /// May only be called once. Emits a `gov_init` event.
    ///
    /// # Authorization
    /// The `admin` must authenticate via `require_auth()`.
    ///
    /// # Errors
    /// - [`GovernanceError::AlreadyInitialized`] if already initialized.
    /// - [`GovernanceError::InvalidVotingPeriod`] if `voting_period` is zero.
    pub fn initialize(
        env: Env,
        admin: Address,
        voting_period: u64,
    ) -> Result<(), GovernanceError> {
        admin.require_auth();

        if env.storage().instance().has(&DataKey::Initialized) {
            return Err(GovernanceError::AlreadyInitialized);
        }
        if voting_period == 0 {
            return Err(GovernanceError::InvalidVotingPeriod);
        }

        env.storage().instance().set(&DataKey::Initialized, &true);
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::VotingPeriod, &voting_period);
        env.storage().instance().set(&DataKey::NextId, &0u64);

        events::emit_initialized(&env, &admin, voting_period);

        Ok(())
    }

    // =======================================================================
    // State-changing entrypoints
    // =======================================================================

    /// Create a new proposal with the given `title`.
    ///
    /// The proposal starts in [`ProposalStatus::Active`] and its voting window
    /// closes `voting_period` seconds after creation. Returns the new proposal
    /// id. Emits `gov_created` and a `gov_status` event.
    ///
    /// # Authorization
    /// The `proposer` must authenticate via `require_auth()`.
    ///
    /// # Errors
    /// - [`GovernanceError::NotInitialized`] if contract not initialized.
    /// - [`GovernanceError::Overflow`] if the id counter or voting deadline
    ///   would overflow.
    pub fn create_proposal(
        env: Env,
        proposer: Address,
        title: String,
    ) -> Result<u64, GovernanceError> {
        proposer.require_auth();
        Self::require_initialized(&env)?;

        let id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::NextId)
            .unwrap_or(0u64);
        let next = id.checked_add(1).ok_or(GovernanceError::Overflow)?;

        let voting_period: u64 = env
            .storage()
            .instance()
            .get(&DataKey::VotingPeriod)
            .ok_or(GovernanceError::NotInitialized)?;

        let created_at = env.ledger().timestamp();
        let voting_ends_at = created_at
            .checked_add(voting_period)
            .ok_or(GovernanceError::Overflow)?;

        let proposal = Proposal {
            id,
            proposer: proposer.clone(),
            title: title.clone(),
            status: ProposalStatus::Active,
            votes_for: 0,
            votes_against: 0,
            created_at,
            voting_ends_at,
        };

        Self::save_proposal(&env, &proposal);
        env.storage().instance().set(&DataKey::NextId, &next);

        events::emit_proposal_created(&env, &proposer, id, &title, voting_ends_at);
        // A creation is, semantically, a transition into the Active state.
        events::emit_status_changed(&env, id, ProposalStatus::Active, ProposalStatus::Active);

        Ok(id)
    }

    /// Cast a weighted vote on an active proposal.
    ///
    /// Each account may vote at most once per proposal. Voting is only allowed
    /// while the proposal is [`Active`](ProposalStatus::Active) and before its
    /// `voting_ends_at` deadline. Emits a `gov_voted` event.
    ///
    /// # Authorization
    /// The `voter` must authenticate via `require_auth()`.
    ///
    /// # Errors
    /// - [`GovernanceError::NotInitialized`] if contract not initialized.
    /// - [`GovernanceError::ProposalNotFound`] if the proposal does not exist.
    /// - [`GovernanceError::InvalidStateTransition`] if the proposal is not active.
    /// - [`GovernanceError::VotingClosed`] if the voting window has closed.
    /// - [`GovernanceError::AlreadyVoted`] if the account already voted.
    /// - [`GovernanceError::Overflow`] if a vote tally would overflow.
    pub fn cast_vote(
        env: Env,
        voter: Address,
        proposal_id: u64,
        choice: VoteChoice,
        weight: u64,
    ) -> Result<(), GovernanceError> {
        voter.require_auth();
        Self::require_initialized(&env)?;

        let mut proposal = Self::load_proposal(&env, proposal_id)?;

        if proposal.status != ProposalStatus::Active {
            return Err(GovernanceError::InvalidStateTransition);
        }
        if env.ledger().timestamp() >= proposal.voting_ends_at {
            return Err(GovernanceError::VotingClosed);
        }

        let vote_key = DataKey::HasVoted(proposal_id, voter.clone());
        if env.storage().persistent().has(&vote_key) {
            return Err(GovernanceError::AlreadyVoted);
        }

        match choice {
            VoteChoice::For => {
                proposal.votes_for = proposal
                    .votes_for
                    .checked_add(weight)
                    .ok_or(GovernanceError::Overflow)?;
            }
            VoteChoice::Against => {
                proposal.votes_against = proposal
                    .votes_against
                    .checked_add(weight)
                    .ok_or(GovernanceError::Overflow)?;
            }
        }

        env.storage().persistent().set(&vote_key, &true);
        Self::save_proposal(&env, &proposal);

        events::emit_vote_cast(
            &env,
            &voter,
            proposal_id,
            choice,
            weight,
            proposal.votes_for,
            proposal.votes_against,
        );

        Ok(())
    }

    /// Finalize an active proposal after its voting window has closed.
    ///
    /// The proposal transitions to [`Executed`](ProposalStatus::Executed) if
    /// `votes_for > votes_against`, otherwise to
    /// [`Rejected`](ProposalStatus::Rejected). Permissionless: any
    /// authenticated caller may finalize once voting has ended. Emits either a
    /// `gov_executed` or `gov_rejected` event plus a `gov_status` event.
    ///
    /// # Authorization
    /// The `caller` must authenticate via `require_auth()`.
    ///
    /// # Errors
    /// - [`GovernanceError::NotInitialized`] if contract not initialized.
    /// - [`GovernanceError::ProposalNotFound`] if the proposal does not exist.
    /// - [`GovernanceError::InvalidStateTransition`] if not active.
    /// - [`GovernanceError::VotingOpen`] if the voting window is still open.
    pub fn execute_proposal(
        env: Env,
        caller: Address,
        proposal_id: u64,
    ) -> Result<ProposalStatus, GovernanceError> {
        caller.require_auth();
        Self::require_initialized(&env)?;

        let mut proposal = Self::load_proposal(&env, proposal_id)?;

        if proposal.status != ProposalStatus::Active {
            return Err(GovernanceError::InvalidStateTransition);
        }
        if env.ledger().timestamp() < proposal.voting_ends_at {
            return Err(GovernanceError::VotingOpen);
        }

        let old_status = proposal.status;
        let new_status = if proposal.votes_for > proposal.votes_against {
            ProposalStatus::Executed
        } else {
            ProposalStatus::Rejected
        };
        proposal.status = new_status;
        Self::save_proposal(&env, &proposal);

        match new_status {
            ProposalStatus::Executed => events::emit_proposal_executed(
                &env,
                &caller,
                proposal_id,
                proposal.votes_for,
                proposal.votes_against,
            ),
            _ => events::emit_proposal_rejected(
                &env,
                proposal_id,
                proposal.votes_for,
                proposal.votes_against,
            ),
        }
        events::emit_status_changed(&env, proposal_id, old_status, new_status);

        Ok(new_status)
    }

    /// Cancel an active proposal before it is finalized.
    ///
    /// Only the original proposer or the admin may cancel. The proposal
    /// transitions to [`Canceled`](ProposalStatus::Canceled). Emits a
    /// `gov_canceled` event plus a `gov_status` event.
    ///
    /// # Authorization
    /// The `caller` must authenticate via `require_auth()` and be either the
    /// proposer or the admin.
    ///
    /// # Errors
    /// - [`GovernanceError::NotInitialized`] if contract not initialized.
    /// - [`GovernanceError::ProposalNotFound`] if the proposal does not exist.
    /// - [`GovernanceError::InvalidStateTransition`] if not active.
    /// - [`GovernanceError::Unauthorized`] if caller is neither proposer nor admin.
    pub fn cancel_proposal(
        env: Env,
        caller: Address,
        proposal_id: u64,
    ) -> Result<(), GovernanceError> {
        caller.require_auth();
        Self::require_initialized(&env)?;

        let mut proposal = Self::load_proposal(&env, proposal_id)?;

        if proposal.status != ProposalStatus::Active {
            return Err(GovernanceError::InvalidStateTransition);
        }

        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(GovernanceError::NotInitialized)?;
        if caller != proposal.proposer && caller != admin {
            return Err(GovernanceError::Unauthorized);
        }

        let old_status = proposal.status;
        proposal.status = ProposalStatus::Canceled;
        Self::save_proposal(&env, &proposal);

        events::emit_proposal_canceled(&env, &caller, proposal_id);
        events::emit_status_changed(
            &env,
            proposal_id,
            old_status,
            ProposalStatus::Canceled,
        );

        Ok(())
    }

    /// Transfer contract administration to a `new_admin`.
    ///
    /// Emits a `gov_admin_xf` event.
    ///
    /// # Authorization
    /// The `current_admin` must authenticate via `require_auth()` and be the
    /// registered admin.
    ///
    /// # Errors
    /// - [`GovernanceError::NotInitialized`] if contract not initialized.
    /// - [`GovernanceError::Unauthorized`] if caller is not the admin.
    pub fn transfer_admin(
        env: Env,
        current_admin: Address,
        new_admin: Address,
    ) -> Result<(), GovernanceError> {
        current_admin.require_auth();
        Self::require_initialized(&env)?;
        Self::require_admin(&env, &current_admin)?;

        env.storage().instance().set(&DataKey::Admin, &new_admin);

        events::emit_admin_transferred(&env, &current_admin, &new_admin);

        Ok(())
    }

    // =======================================================================
    // Read-only entrypoints (no auth required)
    // =======================================================================

    /// Return the contract's numeric version.
    pub fn version(_env: Env) -> u32 {
        1
    }

    /// Return the current admin address.
    ///
    /// # Errors
    /// - [`GovernanceError::NotInitialized`] if contract not initialized.
    pub fn get_admin(env: Env) -> Result<Address, GovernanceError> {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(GovernanceError::NotInitialized)
    }

    /// Return a stored proposal by `proposal_id`.
    ///
    /// # Errors
    /// - [`GovernanceError::ProposalNotFound`] if the proposal does not exist.
    pub fn get_proposal(env: Env, proposal_id: u64) -> Result<Proposal, GovernanceError> {
        Self::load_proposal(&env, proposal_id)
    }

    /// Return whether `voter` has already voted on `proposal_id`.
    pub fn has_voted(env: Env, proposal_id: u64, voter: Address) -> bool {
        env.storage()
            .persistent()
            .has(&DataKey::HasVoted(proposal_id, voter))
    }

    // =======================================================================
    // Internal helpers
    // =======================================================================

    /// Assert the contract is initialized.
    fn require_initialized(env: &Env) -> Result<(), GovernanceError> {
        if !env.storage().instance().has(&DataKey::Initialized) {
            return Err(GovernanceError::NotInitialized);
        }
        Ok(())
    }

    /// Assert that `caller` is the registered admin.
    fn require_admin(env: &Env, caller: &Address) -> Result<(), GovernanceError> {
        let stored: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(GovernanceError::NotInitialized)?;
        if caller != &stored {
            return Err(GovernanceError::Unauthorized);
        }
        Ok(())
    }

    /// Load a proposal from persistent storage.
    ///
    /// # Errors
    /// - [`GovernanceError::ProposalNotFound`] if the proposal does not exist.
    fn load_proposal(env: &Env, proposal_id: u64) -> Result<Proposal, GovernanceError> {
        env.storage()
            .persistent()
            .get(&DataKey::Proposal(proposal_id))
            .ok_or(GovernanceError::ProposalNotFound)
    }

    /// Persist a proposal and extend its TTL.
    fn save_proposal(env: &Env, proposal: &Proposal) {
        let key = DataKey::Proposal(proposal.id);
        env.storage().persistent().set(&key, proposal);
        env.storage()
            .persistent()
            .extend_ttl(&key, PROPOSAL_TTL_THRESHOLD, PROPOSAL_TTL_TO);
    }
}
