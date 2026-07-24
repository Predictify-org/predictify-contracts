use crate::events::EventEmitter;
use soroban_sdk::{contracttype, panic_with_error, Address, Bytes, BytesN, Env, String, Symbol, Vec};

/// ---------- CONTRACT TYPES ----------

/// Configuration for quorum decay over a proposal's lifetime.
/// The required quorum drops linearly from the initial value toward a floor,
/// preventing stale proposals from lingering indefinitely.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuorumDecay {
    /// Floor quorum expressed in basis points (1/100th of a percent) of the base quorum.
    /// E.g., 2000 means the floor is 20% of the initial quorum.
    pub floor_bps: u32,
    /// Time in seconds for the effective quorum to reach the midpoint
    /// between the initial quorum and the floor. The full decay to floor
    /// completes in 2× `halving_seconds`.
    pub halving_seconds: u64,
}

#[contracttype]
pub struct GovernanceProposal {
    pub id: Symbol,
    pub proposer: Address,
    pub title: String,
    pub description: String,
    pub target: Option<Address>, // optional contract target to call when executed
    pub call_fn: Option<Symbol>, // optional function name to call on target (no args supported)
    pub start_time: u64,         // ledger timestamp when voting starts
    pub end_time: u64,           // ledger timestamp when voting ends
    pub for_votes: u128,
    pub against_votes: u128,
    pub executed: bool,
    /// Per-proposal random salt generated at creation time using `Env::prng`.
    ///
    /// The salt **must** be included in the canonical vote message signed by
    /// each voter.  This binds each signature to a specific proposal instance
    /// and prevents vote-replay: an off-chain signature authorising a vote on
    /// proposal P₁ cannot be replayed against a re-submitted proposal P₂
    /// that happens to share the same `id`, `title`, and `description`.
    ///
    /// Entropy source: `env.prng().gen::<BytesN<32>>()` at creation — never
    /// derived from the block timestamp or any other predictable value.
    pub salt: BytesN<32>,
}

// Key namespaces used in storage
#[contracttype]
#[derive(Clone)]
enum StorageKey {
    Proposal(Symbol),
    ProposalList,          // Vec<Symbol>
    Vote(Symbol, Address), // proposal id + voter -> i32 (0 none, 1 for, 2 against)
    VotingPeriod,          // u64
    QuorumVotes,           // u128 minimum FOR votes required
    QuorumDecay,           // QuorumDecay config (optional)
    Admin,                 // Address
    /// Maps delegator -> delegate (Address). At most one per delegator (griefing guard).
    Delegate(Address),
    /// Tracks how many delegators are currently delegating to a given address.
    /// Capped at MAX_INCOMING_DELEGATIONS to bound storage.
    DelegateCount(Address),
    /// Commit-reveal: stores sha256(salt ++ [0|1]) per (proposal, voter).
    VoteCommitment(Symbol, Address),
}

/// Maximum number of delegators that may point to a single delegate address.
/// Limits griefing: prevents an attacker from forcing the contract to walk an
/// unbounded list when tallying delegated votes.
const MAX_INCOMING_DELEGATIONS: u32 = 50;

/// Simple errors for the contract
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GovernanceError {
    ProposalExists,
    ProposalNotFound,
    VotingNotStarted,
    VotingEnded,
    VotingStillActive,
    AlreadyVoted,
    NotPassed,
    AlreadyExecuted,
    NotAdmin,
    NotInitialized,
    InvalidParams,
    /// Caller tried to delegate to themselves.
    SelfDelegation,
    /// Caller already has an active delegation; must unset first.
    DelegationAlreadySet,
    /// The target delegate has reached the incoming-delegation cap (griefing guard).
    DelegateLimitReached,
    /// No active delegation found for the caller.
    NoDelegationSet,
    /// Delegation would create a cycle (A→B→A).
    DelegationCycle,
    /// Caller has not committed a vote for this proposal yet.
    NoCommitment,
    /// The revealed (salt, support) pair does not match the stored commitment.
    InvalidReveal,
    /// A commitment already exists for this (proposal, voter) pair.
    CommitmentExists,
}

/// ---------- CONTRACT ----------
pub struct GovernanceContract;

impl GovernanceContract {
    /// Compute the effective quorum for a proposal at a given elapsed time.
    ///
    /// If `decay` is `None` the full base quorum applies.  Otherwise the quorum
    /// decays linearly from `base_quorum` toward `base_quorum * floor_bps / 10000`
    /// over `2 × halving_seconds`.  After that period the floor is the minimum.
    /// The result is guaranteed monotone non-increasing with respect to `elapsed`.
    pub fn compute_effective_quorum(
        base_quorum: u128,
        decay: &Option<QuorumDecay>,
        elapsed: u64,
    ) -> u128 {
        let cfg = match decay {
            Some(d) => d,
            None => return base_quorum,
        };
        let floor = base_quorum * (cfg.floor_bps as u128) / 10000;
        let full_decay_period = cfg.halving_seconds.saturating_mul(2);
        if elapsed >= full_decay_period {
            return floor;
        }
        let decay_amount = (base_quorum - floor) * (elapsed as u128) / (full_decay_period as u128);
        let effective = base_quorum - decay_amount;
        if effective < floor {
            floor
        } else {
            effective
        }
    }

    /// Initialize governance admin, voting period (seconds), and quorum (minimum FOR votes).
    ///
    /// This function is idempotent: if governance is already initialized, it returns early.
    /// Invalid configuration panics with `Error::InvalidInput`.
    /// An optional `quorum_decay` enables automatic quorum reduction over time.
    pub fn initialize(
        env: Env,
        admin: Address,
        voting_period_seconds: i64,
        quorum_votes: u128,
        quorum_decay: Option<QuorumDecay>,
    ) {
        // Only allow once (idempotent check)
        if env.storage().persistent().has(&StorageKey::Admin) {
            // Already initialized; nothing to do
            return;
        }
        if voting_period_seconds <= 0 || quorum_votes == 0 {
            panic_with_error!(env, crate::err::Error::InvalidInput);
        }
        if let Some(ref d) = quorum_decay {
            if d.floor_bps > 10000 || d.halving_seconds == 0 {
                panic_with_error!(env, crate::err::Error::InvalidInput);
            }
        }
        env.storage().persistent().set(&StorageKey::Admin, &admin);
        env.storage()
            .persistent()
            .set(&StorageKey::VotingPeriod, &(voting_period_seconds as u64));
        env.storage()
            .persistent()
            .set(&StorageKey::QuorumVotes, &quorum_votes);
        env.storage()
            .persistent()
            .set(&StorageKey::QuorumDecay, &quorum_decay);
        // initialize empty proposal list
        let empty: Vec<Symbol> = Vec::new(&env);
        env.storage()
            .persistent()
            .set(&StorageKey::ProposalList, &empty);
    }

    /// Create a proposal. Returns the proposal id (Symbol).
    /// The contract uses ledger timestamp for start and end times.
    pub fn create_proposal(
        env: Env,
        proposer: Address,
        id: Symbol,
        title: String,
        description: String,
        target: Option<Address>,
        call_fn: Option<Symbol>,
    ) -> Result<Symbol, GovernanceError> {
        proposer.require_auth();

        if title.is_empty() || description.is_empty() {
            return Err(GovernanceError::InvalidParams);
        }

        if target.is_some() != call_fn.is_some() {
            return Err(GovernanceError::InvalidParams);
        }

        if !env.storage().persistent().has(&StorageKey::Admin) {
            return Err(GovernanceError::NotInitialized);
        }

        // ensure unique
        if env
            .storage()
            .persistent()
            .has(&StorageKey::Proposal(id.clone()))
        {
            return Err(GovernanceError::ProposalExists);
        }

        // fetch voting period
        let period: u64 = env
            .storage()
            .persistent()
            .get(&StorageKey::VotingPeriod)
            .ok_or(GovernanceError::NotInitialized)?;
        let now = env.ledger().timestamp();

        let p = GovernanceProposal {
            id: id.clone(),
            proposer: proposer.clone(),
            title: title.clone(),
            description: description.clone(),
            target,
            call_fn,
            start_time: now,
            end_time: now.saturating_add(period),
            for_votes: 0,
            against_votes: 0,
            executed: false,
            // Generate a cryptographically random salt using Soroban's PRNG.
            // Using env.prng() here — never the block timestamp — ensures that
            // entropy is not predictable by the proposer or any observer.
            salt: env.prng().gen::<BytesN<32>>(),
        };

        env.storage()
            .persistent()
            .set(&StorageKey::Proposal(id.clone()), &p);

        // push to list
        let mut list: Vec<Symbol> = env
            .storage()
            .persistent()
            .get(&StorageKey::ProposalList)
            .unwrap_or(Vec::new(&env));
        list.push_back(id.clone());
        env.storage()
            .persistent()
            .set(&StorageKey::ProposalList, &list);

        EventEmitter::emit_governance_proposal_created(&env, &id, &proposer, &title, &description);

        Ok(id)
    }

    /// Vote on a proposal. `support = true` means FOR, false means AGAINST.
    /// Each address counts as 1 vote plus 1 for each address that has delegated to it.
    ///
    /// # Vote-replay prevention
    ///
    /// The caller **must** supply the proposal's `salt` value.  The contract
    /// compares it against the salt stored in the proposal and rejects the vote
    /// with `GovernanceError::SaltMismatch` if they differ.
    ///
    /// Off-chain signers should include the salt in the canonical message they
    /// sign, e.g. `sha256(proposal_id || salt || voter || support)`.  This
    /// ensures that a valid signature for one proposal instance cannot be
    /// replayed against a different instance that happens to share the same
    /// payload.
    pub fn vote(
        env: Env,
        voter: Address,
        proposal_id: Symbol,
        support: bool,
        salt: BytesN<32>,
    ) -> Result<(), GovernanceError> {
        voter.require_auth();

        // load proposal
        let p_opt = env
            .storage()
            .persistent()
            .get::<StorageKey, GovernanceProposal>(&StorageKey::Proposal(proposal_id.clone()));
        if p_opt.is_none() {
            return Err(GovernanceError::ProposalNotFound);
        }
        let mut p = p_opt.unwrap();

        // Verify that the salt supplied by the voter matches the stored salt.
        // This prevents vote-replay across re-submitted proposals.
        if salt != p.salt {
            return Err(GovernanceError::SaltMismatch);
        }

        let now = env.ledger().timestamp();
        if now < p.start_time {
            return Err(GovernanceError::VotingNotStarted);
        }
        if now >= p.end_time {
            return Err(GovernanceError::VotingEnded);
        }
        if p.executed {
            return Err(GovernanceError::AlreadyExecuted);
        }

        // check if voter already voted
        if env
            .storage()
            .persistent()
            .has(&StorageKey::Vote(proposal_id.clone(), voter.clone()))
        {
            return Err(GovernanceError::AlreadyVoted);
        }

        // Voting weight = 1 (own vote) + number of addresses delegating to this voter.
        let delegated: u128 = env
            .storage()
            .persistent()
            .get::<StorageKey, u32>(&StorageKey::DelegateCount(voter.clone()))
            .unwrap_or(0) as u128;
        let weight: u128 = 1 + delegated;

        if support {
            p.for_votes += weight;
            env.storage()
                .persistent()
                .set(&StorageKey::Vote(proposal_id.clone(), voter.clone()), &1i32);
        } else {
            p.against_votes += weight;
            env.storage()
                .persistent()
                .set(&StorageKey::Vote(proposal_id.clone(), voter.clone()), &2i32);
        }

        // update proposal
        env.storage()
            .persistent()
            .set(&StorageKey::Proposal(proposal_id.clone()), &p);

        // Emit governance vote event
        EventEmitter::emit_governance_vote_cast(&env, &proposal_id, &voter, support);

        Ok(())
    }

    /// Commit a salted vote for a proposal (commit phase of commit-reveal).
    ///
    /// The caller supplies a 32-byte commitment = `sha256(salt ++ support_byte)` where
    /// `support_byte` is `0x01` for FOR and `0x00` for AGAINST.  The commitment is
    /// stored on-chain but the actual vote preference stays hidden until `reveal_vote`.
    ///
    /// Constraints
    /// - Voting window must be open (`start_time <= now < end_time`).
    /// - Each (proposal, voter) pair may only commit once.
    /// - The voter must not have already voted via the direct `vote()` path.
    pub fn commit_vote(
        env: Env,
        voter: Address,
        proposal_id: Symbol,
        commitment: BytesN<32>,
    ) -> Result<(), GovernanceError> {
        voter.require_auth();

        let p: GovernanceProposal = env
            .storage()
            .persistent()
            .get::<StorageKey, GovernanceProposal>(&StorageKey::Proposal(proposal_id.clone()))
            .ok_or(GovernanceError::ProposalNotFound)?;

        let now = env.ledger().timestamp();
        if now < p.start_time {
            return Err(GovernanceError::VotingNotStarted);
        }
        if now >= p.end_time {
            return Err(GovernanceError::VotingEnded);
        }

        // Guard: direct vote already cast
        if env
            .storage()
            .persistent()
            .has(&StorageKey::Vote(proposal_id.clone(), voter.clone()))
        {
            return Err(GovernanceError::AlreadyVoted);
        }

        // Guard: commitment already stored
        if env
            .storage()
            .persistent()
            .has(&StorageKey::VoteCommitment(proposal_id.clone(), voter.clone()))
        {
            return Err(GovernanceError::CommitmentExists);
        }

        env.storage().persistent().set(
            &StorageKey::VoteCommitment(proposal_id.clone(), voter.clone()),
            &commitment,
        );

        EventEmitter::emit_governance_vote_committed(&env, &proposal_id, &voter);

        Ok(())
    }

    /// Reveal a previously committed vote (reveal phase of commit-reveal).
    ///
    /// The contract recomputes `sha256(salt ++ support_byte)` and verifies it matches
    /// the stored commitment.  On success the vote weight is tallied and the commitment
    /// entry is replaced with the recorded vote, preventing double-reveals.
    ///
    /// Constraints
    /// - A commitment for this (proposal, voter) must exist.
    /// - Voting window must still be open when revealing.
    /// - The voter must not have already been counted (guards against re-reveal).
    pub fn reveal_vote(
        env: Env,
        voter: Address,
        proposal_id: Symbol,
        salt: Bytes,
        support: bool,
    ) -> Result<(), GovernanceError> {
        voter.require_auth();

        let stored: BytesN<32> = env
            .storage()
            .persistent()
            .get::<StorageKey, BytesN<32>>(&StorageKey::VoteCommitment(
                proposal_id.clone(),
                voter.clone(),
            ))
            .ok_or(GovernanceError::NoCommitment)?;

        // Verify commitment: sha256(salt ++ support_byte)
        let mut preimage = Bytes::new(&env);
        preimage.append(&salt);
        preimage.push_back(if support { 1u8 } else { 0u8 });
        let expected: BytesN<32> = env.crypto().sha256(&preimage).into();
        if stored != expected {
            return Err(GovernanceError::InvalidReveal);
        }

        let mut p: GovernanceProposal = env
            .storage()
            .persistent()
            .get::<StorageKey, GovernanceProposal>(&StorageKey::Proposal(proposal_id.clone()))
            .ok_or(GovernanceError::ProposalNotFound)?;

        let now = env.ledger().timestamp();
        if now < p.start_time {
            return Err(GovernanceError::VotingNotStarted);
        }
        if now >= p.end_time {
            return Err(GovernanceError::VotingEnded);
        }

        // Guard: already counted via direct vote or prior reveal
        if env
            .storage()
            .persistent()
            .has(&StorageKey::Vote(proposal_id.clone(), voter.clone()))
        {
            return Err(GovernanceError::AlreadyVoted);
        }

        // Tally with delegation weight
        let delegated: u128 = env
            .storage()
            .persistent()
            .get::<StorageKey, u32>(&StorageKey::DelegateCount(voter.clone()))
            .unwrap_or(0) as u128;
        let weight: u128 = 1 + delegated;

        if support {
            p.for_votes += weight;
            env.storage()
                .persistent()
                .set(&StorageKey::Vote(proposal_id.clone(), voter.clone()), &1i32);
        } else {
            p.against_votes += weight;
            env.storage()
                .persistent()
                .set(&StorageKey::Vote(proposal_id.clone(), voter.clone()), &2i32);
        }

        env.storage()
            .persistent()
            .set(&StorageKey::Proposal(proposal_id.clone()), &p);

        // Remove commitment after reveal to free storage
        env.storage()
            .persistent()
            .remove(&StorageKey::VoteCommitment(proposal_id.clone(), voter.clone()));

        EventEmitter::emit_governance_vote_cast(&env, &proposal_id, &voter, support);

        Ok(())
    }

    /// Delegate the caller's vote to `delegate`.
    ///
    /// Storage griefing guard:
    /// - A delegator may hold at most **one** active delegation at a time.
    /// - A delegate may receive at most `MAX_INCOMING_DELEGATIONS` incoming delegations.
    /// - Self-delegation and two-hop cycles (A→B while B→A exists) are rejected.
    pub fn set_delegate(
        env: Env,
        delegator: Address,
        delegate: Address,
    ) -> Result<(), GovernanceError> {
        delegator.require_auth();

        // No self-delegation
        if delegator == delegate {
            return Err(GovernanceError::SelfDelegation);
        }

        // Detect two-hop cycle: reject if delegate has already delegated back to delegator
        if let Some(delegates_delegate) = env
            .storage()
            .persistent()
            .get::<StorageKey, Address>(&StorageKey::Delegate(delegate.clone()))
        {
            if delegates_delegate == delegator {
                return Err(GovernanceError::DelegationCycle);
            }
        }

        // Enforce max-1 active delegation per delegator (griefing guard)
        if env
            .storage()
            .persistent()
            .has(&StorageKey::Delegate(delegator.clone()))
        {
            return Err(GovernanceError::DelegationAlreadySet);
        }

        // Enforce incoming-delegation cap on the delegate (griefing guard)
        let incoming: u32 = env
            .storage()
            .persistent()
            .get::<StorageKey, u32>(&StorageKey::DelegateCount(delegate.clone()))
            .unwrap_or(0);
        if incoming >= MAX_INCOMING_DELEGATIONS {
            return Err(GovernanceError::DelegateLimitReached);
        }

        // Persist delegation and bump counter
        env.storage()
            .persistent()
            .set(&StorageKey::Delegate(delegator.clone()), &delegate);
        env.storage()
            .persistent()
            .set(&StorageKey::DelegateCount(delegate.clone()), &(incoming + 1));

        Ok(())
    }

    /// Remove the caller's active delegation.
    pub fn unset_delegate(env: Env, delegator: Address) -> Result<(), GovernanceError> {
        delegator.require_auth();

        let delegate: Address = env
            .storage()
            .persistent()
            .get::<StorageKey, Address>(&StorageKey::Delegate(delegator.clone()))
            .ok_or(GovernanceError::NoDelegationSet)?;

        // Decrement incoming count on the previously-pointed-at delegate
        let incoming: u32 = env
            .storage()
            .persistent()
            .get::<StorageKey, u32>(&StorageKey::DelegateCount(delegate.clone()))
            .unwrap_or(0);
        if incoming > 0 {
            env.storage()
                .persistent()
                .set(&StorageKey::DelegateCount(delegate.clone()), &(incoming - 1));
        }

        env.storage()
            .persistent()
            .remove(&StorageKey::Delegate(delegator.clone()));

        Ok(())
    }

    /// Return the delegate for `delegator`, if any.
    pub fn get_delegate(env: Env, delegator: Address) -> Option<Address> {
        env.storage()
            .persistent()
            .get::<StorageKey, Address>(&StorageKey::Delegate(delegator))
    }

    /// Validate governance votes for a proposal. Returns (passed: bool, reason: String)
    ///
    /// When quorum decay is configured the effective quorum is computed based on
    /// time elapsed since the proposal opened.  If the proposal has expired and
    /// not even the floor quorum was reached an auto-rejection event is emitted.
    pub fn validate_proposal(
        env: Env,
        proposal_id: Symbol,
    ) -> Result<(bool, String), GovernanceError> {
        let p_opt = env
            .storage()
            .persistent()
            .get::<StorageKey, GovernanceProposal>(&StorageKey::Proposal(proposal_id.clone()));
        if p_opt.is_none() {
            return Err(GovernanceError::ProposalNotFound);
        }
        let p = p_opt.unwrap();
        let now = env.ledger().timestamp();
        if now < p.end_time {
            return Err(GovernanceError::VotingStillActive);
        }

        // load base quorum and optional decay config
        let base_quorum: u128 = env
            .storage()
            .persistent()
            .get(&StorageKey::QuorumVotes)
            .ok_or(GovernanceError::NotInitialized)?;

        let decay: Option<QuorumDecay> = env
            .storage()
            .persistent()
            .get::<StorageKey, Option<QuorumDecay>>(&StorageKey::QuorumDecay)
            .unwrap_or(None);

        let elapsed = now.saturating_sub(p.start_time);
        let effective_quorum =
            Self::compute_effective_quorum(base_quorum, &decay, elapsed);

        if p.for_votes < effective_quorum {
            // check whether even the floor was missed
            let floor = match &decay {
                Some(d) => base_quorum * (d.floor_bps as u128) / 10000,
                None => effective_quorum,
            };
            if p.for_votes < floor {
                EventEmitter::emit_governance_proposal_auto_rejected(
                    &env,
                    &proposal_id,
                    &p.proposer,
                    p.for_votes,
                    floor,
                );
            }
            return Ok((false, String::from_str(&env, "quorum not reached")));
        }
        if p.for_votes <= p.against_votes {
            return Ok((false, String::from_str(&env, "not enough for votes")));
        }
        Ok((true, String::from_str(&env, "passed")))
    }

    /// Execute governance proposal. If `target` and `call_fn` are None -> treated as no-op,
    /// mark executed and emit event. If `target` is contract address and `call_fn` is present,
    /// we attempt to invoke that function on the target with no args. (Extend as needed.)
    pub fn execute_proposal(
        env: Env,
        caller: Address,
        proposal_id: Symbol,
    ) -> Result<(), GovernanceError> {
        caller.require_auth();

        // load proposal
        let p_opt = env
            .storage()
            .persistent()
            .get::<StorageKey, GovernanceProposal>(&StorageKey::Proposal(proposal_id.clone()));
        if p_opt.is_none() {
            return Err(GovernanceError::ProposalNotFound);
        }
        let mut p = p_opt.unwrap();

        if p.executed {
            return Err(GovernanceError::AlreadyExecuted);
        }

        // validate
        let (passed, _reason) = Self::validate_proposal(env.clone(), proposal_id.clone())?;
        if !passed {
            return Err(GovernanceError::NotPassed);
        }

        // Execution semantics:
        // - if no target or no call_fn: treat as no-op, mark executed.
        // - if target is Contract and call_fn is present, call that function on the contract with no arguments.
        if p.target.is_none() || p.call_fn.is_none() {
            p.executed = true;
            env.storage()
                .persistent()
                .set(&StorageKey::Proposal(proposal_id.clone()), &p);
            EventEmitter::emit_governance_proposal_executed(&env, &proposal_id, &caller);
            return Ok(());
        }

        // attempt invocation on contract target
        let target = p.target.clone().unwrap();
        let func = p.call_fn.clone().unwrap();

        // Try invoking the contract function with no args.
        let _result: () = env.invoke_contract(&target, &func, Vec::new(&env));

        // Mark executed after successful call
        p.executed = true;
        env.storage()
            .persistent()
            .set(&StorageKey::Proposal(proposal_id.clone()), &p);

        // Emit governance execution event
        EventEmitter::emit_governance_proposal_executed(&env, &proposal_id, &caller);

        Ok(())
    }

    /// Return a vector of proposal ids (for off-chain indexing/UI)
    pub fn list_proposals(env: Env) -> Vec<Symbol> {
        env.storage()
            .persistent()
            .get(&StorageKey::ProposalList)
            .unwrap_or(Vec::new(&env))
    }

    /// Get full proposal details by id
    pub fn get_proposal(env: Env, id: Symbol) -> Result<GovernanceProposal, GovernanceError> {
        let p_opt = env
            .storage()
            .persistent()
            .get(&StorageKey::Proposal(id.clone()));
        if p_opt.is_none() {
            return Err(GovernanceError::ProposalNotFound);
        }
        Ok(p_opt.unwrap())
    }

    /// Return the salt for a proposal.
    ///
    /// Off-chain clients use this to build the canonical vote message:
    /// `sha256(proposal_id || salt || voter || support)`.
    ///
    /// The salt is generated by `Env::prng` at proposal creation and never
    /// derived from predictable data, so it cannot be forged or pre-computed
    /// by an attacker before the proposal is submitted.
    pub fn get_proposal_salt(env: Env, id: Symbol) -> Result<BytesN<32>, GovernanceError> {
        let p: GovernanceProposal = env
            .storage()
            .persistent()
            .get(&StorageKey::Proposal(id.clone()))
            .ok_or(GovernanceError::ProposalNotFound)?;
        Ok(p.salt)
    }

    /// Admin-only: set voting period (seconds)
    pub fn set_voting_period(
        env: Env,
        caller: Address,
        new_period_seconds: i64,
    ) -> Result<(), GovernanceError> {
        Self::ensure_admin(&env, caller)?;
        if new_period_seconds <= 0 {
            return Err(GovernanceError::InvalidParams);
        }
        env.storage()
            .persistent()
            .set(&StorageKey::VotingPeriod, &(new_period_seconds as u64));
        Ok(())
    }

    /// Admin-only: set quorum votes (minimum for votes)
    pub fn set_quorum(env: Env, caller: Address, new_quorum: u128) -> Result<(), GovernanceError> {
        Self::ensure_admin(&env, caller)?;
        if new_quorum == 0 {
            return Err(GovernanceError::InvalidParams);
        }
        env.storage()
            .persistent()
            .set(&StorageKey::QuorumVotes, &new_quorum);
        Ok(())
    }

    /// Admin-only: configure or disable quorum decay.
    /// Pass `None` to disable decay (static quorum).
    pub fn set_quorum_decay(
        env: Env,
        caller: Address,
        decay: Option<QuorumDecay>,
    ) -> Result<(), GovernanceError> {
        Self::ensure_admin(&env, caller)?;
        if let Some(ref d) = decay {
            if d.floor_bps > 10000 || d.halving_seconds == 0 {
                return Err(GovernanceError::InvalidParams);
            }
        }
        env.storage()
            .persistent()
            .set(&StorageKey::QuorumDecay, &decay);
        Ok(())
    }

    /// View the current quorum decay configuration (if any).
    pub fn get_quorum_decay(env: Env) -> Option<QuorumDecay> {
        env.storage()
            .persistent()
            .get::<StorageKey, Option<QuorumDecay>>(&StorageKey::QuorumDecay)
            .unwrap_or(None)
    }

    /// Simple helper to check admin
    fn ensure_admin(env: &Env, caller: Address) -> Result<(), GovernanceError> {
        caller.require_auth();
        let admin: Address = env
            .storage()
            .persistent()
            .get(&StorageKey::Admin)
            .ok_or(GovernanceError::NotInitialized)?;
        if admin != caller {
            return Err(GovernanceError::NotAdmin);
        }
        Ok(())
    }
}
