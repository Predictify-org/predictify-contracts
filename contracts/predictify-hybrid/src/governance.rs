use crate::events::EventEmitter;
use soroban_sdk::{contracttype, Address, Env, String, Symbol, Vec};

/// ---------- CONTRACT TYPES ----------
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
}

// Key namespaces used in storage
#[contracttype]
#[derive(Clone)]
enum StorageKey {
    Proposal(Symbol),
    ProposalList,          // Vec<Symbol>
    Vote(Symbol, Address), // proposal id + voter -> u8 (0 none, 1 for, 2 against)
    VotingPeriod,          // u64
    QuorumVotes,           // u128 minimum FOR votes required
    Admin,                 // Address
}

/// Simple errors for the contract
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GovernanceError {
    ProposalExists,
    ProposalNotFound,
    VotingNotStarted,
    VotingEnded,
    AlreadyVoted,
    NotPassed,
    AlreadyExecuted,
    NotAdmin,
    InvalidParams,
}

/// ---------- CONTRACT ----------
pub struct GovernanceContract;

impl GovernanceContract {
    // Initialize admin, voting period (seconds) and quorum (minimum FOR votes).
    pub fn initialize(env: Env, admin: Address, voting_period_seconds: i64, quorum_votes: u128) {
        // Only allow once (idempotent check)
        if env.storage().persistent().has(&StorageKey::Admin) {
            // Already initialized; nothing to do
            return;
        }
        if voting_period_seconds == 0 || quorum_votes == 0 {
            panic!("invalid params");
        }
        env.storage().persistent().set(&StorageKey::Admin, &admin);
        env.storage()
            .persistent()
            .set(&StorageKey::VotingPeriod, &voting_period_seconds);
        env.storage()
            .persistent()
            .set(&StorageKey::QuorumVotes, &quorum_votes);
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
        // ensure unique
        if env
            .storage()
            .persistent()
            .has(&StorageKey::Proposal(id.clone()))
        {
            return Err(GovernanceError::ProposalExists);
        }

        // fetch voting period
        let period: i64 = env
            .storage()
            .persistent()
            .get(&StorageKey::VotingPeriod)
            .unwrap();
        let now = env.ledger().timestamp();

        let p = GovernanceProposal {
            id: id.clone(),
            proposer: proposer.clone(),
            title: title.clone(),
            description: description.clone(),
            target,
            call_fn,
            start_time: now,
            end_time: now + (period as u64),
            for_votes: 0,
            against_votes: 0,
            executed: false,
        };

        env.storage()
            .persistent()
            .set(&StorageKey::Proposal(id.clone()), &p);

        // push to list
        let mut list: Vec<Symbol> = env
            .storage()
            .persistent()
            .get(&StorageKey::ProposalList)
            .unwrap();
        list.push_back(id.clone());
        env.storage()
            .persistent()
            .set(&StorageKey::ProposalList, &list);

        EventEmitter::emit_governance_proposal_created(&env, &id, &proposer, &title, &description);

        Ok(id)
    }

    /// Vote on a proposal. `support = true` means FOR, false means AGAINST.
    /// One address one vote (no weighting).
    pub fn vote(
        env: Env,
        voter: Address,
        proposal_id: Symbol,
        support: bool,
    ) -> Result<(), GovernanceError> {
        // load proposal
        let p_opt = env
            .storage()
            .persistent()
            .get::<StorageKey, GovernanceProposal>(&StorageKey::Proposal(proposal_id.clone()));
        if p_opt.is_none() {
            return Err(GovernanceError::ProposalNotFound);
        }
        let mut p = p_opt.unwrap();

        let now = env.ledger().timestamp();
        if now < p.start_time {
            return Err(GovernanceError::VotingNotStarted);
        }
        if now > p.end_time {
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

        if support {
            p.for_votes += 1;
            env.storage()
                .persistent()
                .set(&StorageKey::Vote(proposal_id.clone(), voter.clone()), &1i32);
        } else {
            p.against_votes += 1;
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

    /// Validate governance votes for a proposal. Returns (passed: bool, reason: String)
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
        if now <= p.end_time {
            return Ok((false, String::from_str(&env, "voting not finished")));
        }

        // check quorum
        let quorum: u128 = env
            .storage()
            .persistent()
            .get(&StorageKey::QuorumVotes)
            .unwrap();
        let total_votes = p.for_votes + p.against_votes;
        if total_votes < quorum {
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
        let (passed, _reason) = Self::validate_proposal(env.clone(), proposal_id.clone())
            .map_err(|_| GovernanceError::ProposalNotFound)?;
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
            .set(&StorageKey::VotingPeriod, &new_period_seconds);
        Ok(())
    }

    /// Admin-only: set quorum votes (minimum for votes)
    pub fn set_quorum(env: Env, caller: Address, new_quorum: u128) -> Result<(), GovernanceError> {
        Self::ensure_admin(&env, caller)?;
        env.storage()
            .persistent()
            .set(&StorageKey::QuorumVotes, &new_quorum);
        Ok(())
    }

    /// Simple helper to check admin
    fn ensure_admin(env: &Env, caller: Address) -> Result<(), GovernanceError> {
        let admin: Address = env
            .storage()
            .persistent()
            .get(&StorageKey::Admin)
            .ok_or(GovernanceError::NotAdmin)?;
        if admin != caller {
            return Err(GovernanceError::NotAdmin);
        }
        Ok(())
    }
}
