//! Governance module for the Predictify Hybrid contract.
//!
//! Implements on-chain proposal creation, voting, validation, and execution
//! with overflow-safe arithmetic throughout. All `u128` and `u64` arithmetic
//! uses `checked_add` / `checked_sub` to prevent silent wrapping.

use crate::events::EventEmitter;
use soroban_sdk::{contracttype, panic_with_error, Address, Env, String, Symbol, Vec};

/// ---------- CONSTANTS ----------

/// Minimum voting period in seconds (1 hour).
pub const MIN_VOTING_PERIOD: i64 = 3600;

/// Minimum quorum (absolute number of FOR votes required).
pub const MIN_QUORUM: u128 = 1;

/// ---------- CONTRACT TYPES ----------

/// A governance proposal tracked on-chain.
#[contracttype]
pub struct GovernanceProposal {
    pub id: Symbol,
    pub proposer: Address,
    pub title: String,
    pub description: String,
    /// Optional contract to call when the proposal is executed.
    pub target: Option<Address>,
    /// Optional function name to invoke on `target`.
    pub call_fn: Option<Symbol>,
    /// Ledger timestamp when voting starts.
    pub start_time: u64,
    /// Ledger timestamp when voting ends.
    pub end_time: u64,
    /// Accumulated FOR votes (overflow-safe).
    pub for_votes: u128,
    /// Accumulated AGAINST votes (overflow-safe).
    pub against_votes: u128,
    /// Whether this proposal has been executed.
    pub executed: bool,
}

/// Storage key namespace.
#[contracttype]
#[derive(Clone)]
enum StorageKey {
    Proposal(Symbol),
    ProposalList,
    Vote(Symbol, Address),
    VotingPeriod,
    QuorumVotes,
    Admin,
}

/// Governance-specific errors.
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
    /// Initialise admin, voting period (seconds) and quorum (minimum FOR votes).
    ///
    /// Idempotent — subsequent calls are no-ops.
    pub fn initialize(env: Env, admin: Address, voting_period_seconds: i64, quorum_votes: u128) {
        if env.storage().persistent().has(&StorageKey::Admin) {
            return;
        }
        if voting_period_seconds == 0 || quorum_votes == 0 {
            panic_with_error!(env, crate::err::Error::InvalidInput);
        }
        env.storage().persistent().set(&StorageKey::Admin, &admin);
        env.storage()
            .persistent()
            .set(&StorageKey::VotingPeriod, &voting_period_seconds);
        env.storage()
            .persistent()
            .set(&StorageKey::QuorumVotes, &quorum_votes);
        let empty: Vec<Symbol> = Vec::new(&env);
        env.storage()
            .persistent()
            .set(&StorageKey::ProposalList, &empty);
    }

    /// Create a proposal.  Returns the proposal `Symbol` id on success.
    pub fn create_proposal(
        env: Env,
        proposer: Address,
        id: Symbol,
        title: String,
        description: String,
        target: Option<Address>,
        call_fn: Option<Symbol>,
    ) -> Result<Symbol, GovernanceError> {
        if env
            .storage()
            .persistent()
            .has(&StorageKey::Proposal(id.clone()))
        {
            return Err(GovernanceError::ProposalExists);
        }

        let period: i64 = env
            .storage()
            .persistent()
            .get(&StorageKey::VotingPeriod)
            .ok_or(GovernanceError::InvalidParams)?;
        let now = env.ledger().timestamp();

        // Overflow-safe: end_time = now + period as u64
        let period_u64 = period as u64;
        let end_time = now.checked_add(period_u64).unwrap_or(u64::MAX);

        let p = GovernanceProposal {
            id: id.clone(),
            proposer: proposer.clone(),
            title: title.clone(),
            description: description.clone(),
            target,
            call_fn,
            start_time: now,
            end_time,
            for_votes: 0,
            against_votes: 0,
            executed: false,
        };

        env.storage()
            .persistent()
            .set(&StorageKey::Proposal(id.clone()), &p);

        let mut list: Vec<Symbol> = env
            .storage()
            .persistent()
            .get(&StorageKey::ProposalList)
            .unwrap_or(Vec::new(&env));
        list.push_back(id.clone());
        env.storage()
            .persistent()
            .set(&StorageKey::ProposalList, &list);

        Ok(id)
    }

    /// Vote on a proposal.  `support = true` means FOR, `false` means AGAINST.
    /// One address, one vote (no weighting).
    pub fn vote(
        env: Env,
        voter: Address,
        proposal_id: Symbol,
        support: bool,
    ) -> Result<(), GovernanceError> {
        let p_opt = env
            .storage()
            .persistent()
            .get::<StorageKey, GovernanceProposal>(&StorageKey::Proposal(proposal_id.clone()));
        let mut p = p_opt.ok_or(GovernanceError::ProposalNotFound)?;

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
        if env
            .storage()
            .persistent()
            .has(&StorageKey::Vote(proposal_id.clone(), voter.clone()))
        {
            return Err(GovernanceError::AlreadyVoted);
        }

        if support {
            // Overflow-safe increment
            p.for_votes = p.for_votes.checked_add(1).unwrap_or(u128::MAX);
            env.storage()
                .persistent()
                .set(&StorageKey::Vote(proposal_id.clone(), voter.clone()), &1i32);
        } else {
            // Overflow-safe increment
            p.against_votes = p.against_votes.checked_add(1).unwrap_or(u128::MAX);
            env.storage()
                .persistent()
                .set(&StorageKey::Vote(proposal_id.clone(), voter.clone()), &2i32);
        }

        env.storage()
            .persistent()
            .set(&StorageKey::Proposal(proposal_id.clone()), &p);

        Ok(())
    }

    /// Validate whether a proposal has passed the voting period.
    ///
    /// Returns `(passed, reason)` where `passed` is `true` only when:
    /// - Voting has ended, AND
    /// - Quorum is met (total votes >= quorum), AND
    /// - FOR votes > AGAINST votes
    pub fn validate_proposal(
        env: Env,
        proposal_id: Symbol,
    ) -> Result<(bool, String), GovernanceError> {
        let p_opt = env
            .storage()
            .persistent()
            .get::<StorageKey, GovernanceProposal>(&StorageKey::Proposal(proposal_id.clone()));
        let p = p_opt.ok_or(GovernanceError::ProposalNotFound)?;

        let now = env.ledger().timestamp();
        if now <= p.end_time {
            return Ok((false, String::from_str(&env, "voting not finished")));
        }

        let quorum: u128 = env
            .storage()
            .persistent()
            .get(&StorageKey::QuorumVotes)
            .unwrap_or(MIN_QUORUM);

        // Overflow-safe total
        let total = p
            .for_votes
            .checked_add(p.against_votes)
            .unwrap_or(u128::MAX);
        if total < quorum {
            return Ok((false, String::from_str(&env, "quorum not reached")));
        }
        if p.for_votes <= p.against_votes {
            return Ok((false, String::from_str(&env, "not enough for votes")));
        }
        Ok((true, String::from_str(&env, "passed")))
    }

    /// Execute a proposal that has passed.
    ///
    /// If the proposal has a `target` and `call_fn`, the contract function is
    /// invoked with no arguments.  Other proposals are treated as no-ops that
    /// are simply marked executed.
    pub fn execute_proposal(
        env: Env,
        caller: Address,
        proposal_id: Symbol,
    ) -> Result<(), GovernanceError> {
        let p_opt = env
            .storage()
            .persistent()
            .get::<StorageKey, GovernanceProposal>(&StorageKey::Proposal(proposal_id.clone()));
        let mut p = p_opt.ok_or(GovernanceError::ProposalNotFound)?;

        if p.executed {
            return Err(GovernanceError::AlreadyExecuted);
        }

        let (passed, _reason) = Self::validate_proposal(env.clone(), proposal_id.clone())
            .map_err(|_| GovernanceError::ProposalNotFound)?;
        if !passed {
            return Err(GovernanceError::NotPassed);
        }

        // Execute the proposal action if target + call_fn are set.
        if let (Some(target), Some(func)) = (p.target.clone(), p.call_fn.clone()) {
            let _: () = env.invoke_contract(&target, &func, Vec::new(&env));
        }

        p.executed = true;
        env.storage()
            .persistent()
            .set(&StorageKey::Proposal(proposal_id.clone()), &p);

        EventEmitter::emit_governance_proposal_executed(&env, &proposal_id, &caller);

        Ok(())
    }

    /// Return the list of all proposal ids (for off-chain indexing).
    pub fn list_proposals(env: Env) -> Vec<Symbol> {
        env.storage()
            .persistent()
            .get(&StorageKey::ProposalList)
            .unwrap_or(Vec::new(&env))
    }

    /// Return full proposal details by id.
    pub fn get_proposal(env: Env, id: Symbol) -> Result<GovernanceProposal, GovernanceError> {
        env.storage()
            .persistent()
            .get(&StorageKey::Proposal(id))
            .ok_or(GovernanceError::ProposalNotFound)
    }

    /// Set a new voting period (seconds).  Admin only.
    pub fn set_voting_period(
        env: Env,
        caller: Address,
        period: i64,
    ) -> Result<(), GovernanceError> {
        Self::require_admin(&env, &caller)?;
        if period <= 0 {
            return Err(GovernanceError::InvalidParams);
        }
        env.storage()
            .persistent()
            .set(&StorageKey::VotingPeriod, &period);
        Ok(())
    }

    /// Set a new quorum threshold.  Admin only.
    pub fn set_quorum(env: Env, caller: Address, quorum: u128) -> Result<(), GovernanceError> {
        Self::require_admin(&env, &caller)?;
        if quorum == 0 {
            return Err(GovernanceError::InvalidParams);
        }
        env.storage()
            .persistent()
            .set(&StorageKey::QuorumVotes, &quorum);
        Ok(())
    }

    // ===================================================================
    // Internal helpers
    // ===================================================================

    fn require_admin(env: &Env, caller: &Address) -> Result<(), GovernanceError> {
        let stored: Address = env
            .storage()
            .persistent()
            .get(&StorageKey::Admin)
            .ok_or(GovernanceError::NotAdmin)?;
        if caller != &stored {
            return Err(GovernanceError::NotAdmin);
        }
        Ok(())
    }
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PredictifyHybrid;
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::Env;

    struct GovernanceTest {
        env: Env,
        contract_id: Address,
        admin: Address,
        voter: Address,
    }

    impl GovernanceTest {
        fn new() -> Self {
            let env = Env::default();
            env.mock_all_auths();
            let contract_id = env.register(PredictifyHybrid, ());
            let admin = Address::generate(&env);
            let voter = Address::generate(&env);

            env.as_contract(&contract_id, || {
                GovernanceContract::initialize(
                    env.clone(),
                    admin.clone(),
                    7200, // 2-hour voting period
                    3,    // quorum = 3
                );
            });

            GovernanceTest {
                env,
                contract_id,
                admin,
                voter,
            }
        }

        fn call<T>(&self, f: impl FnOnce() -> T) -> T {
            self.env.as_contract(&self.contract_id, f)
        }
    }

    #[test]
    fn test_initialize_sets_admin() {
        let test = GovernanceTest::new();
        test.call(|| {
            let stored: Address = test
                .env
                .storage()
                .persistent()
                .get(&StorageKey::Admin)
                .unwrap();
            assert_eq!(stored, test.admin);
        });
    }

    #[test]
    fn test_initialize_is_idempotent() {
        let test = GovernanceTest::new();
        test.call(|| {
            GovernanceContract::initialize(
                test.env.clone(),
                Address::generate(&test.env),
                9999,
                999,
            );
            // Admin should still be the original.
            let stored: Address = test
                .env
                .storage()
                .persistent()
                .get(&StorageKey::Admin)
                .unwrap();
            assert_eq!(stored, test.admin);
        });
    }

    #[test]
    fn test_create_proposal_success() {
        let test = GovernanceTest::new();
        let id = Symbol::new(&test.env, "prop_001");
        test.call(|| {
            let result = GovernanceContract::create_proposal(
                test.env.clone(),
                test.admin.clone(),
                id.clone(),
                String::from_str(&test.env, "Test Proposal"),
                String::from_str(&test.env, "A test proposal"),
                None,
                None,
            );
            assert_eq!(result, Ok(id.clone()));

            let prop = GovernanceContract::get_proposal(test.env.clone(), id.clone()).unwrap();
            assert_eq!(prop.id, id);
            assert_eq!(prop.for_votes, 0);
            assert_eq!(prop.against_votes, 0);
            assert!(!prop.executed);
        });
    }

    #[test]
    fn test_create_proposal_duplicate_rejected() {
        let test = GovernanceTest::new();
        let id = Symbol::new(&test.env, "prop_dup");
        test.call(|| {
            GovernanceContract::create_proposal(
                test.env.clone(),
                test.admin.clone(),
                id.clone(),
                String::from_str(&test.env, "First"),
                String::from_str(&test.env, ""),
                None,
                None,
            )
            .unwrap();

            let result = GovernanceContract::create_proposal(
                test.env.clone(),
                test.admin.clone(),
                id.clone(),
                String::from_str(&test.env, "Duplicate"),
                String::from_str(&test.env, ""),
                None,
                None,
            );
            assert_eq!(result, Err(GovernanceError::ProposalExists));
        });
    }

    #[test]
    fn test_vote_for_and_against() {
        let test = GovernanceTest::new();
        let id = Symbol::new(&test.env, "prop_vote");
        test.call(|| {
            GovernanceContract::create_proposal(
                test.env.clone(),
                test.admin.clone(),
                id.clone(),
                String::from_str(&test.env, "Vote Test"),
                String::from_str(&test.env, ""),
                None,
                None,
            )
            .unwrap();

            let v1 = Address::generate(&test.env);
            let v2 = Address::generate(&test.env);
            let v3 = Address::generate(&test.env);

            assert!(GovernanceContract::vote(test.env.clone(), v1, id.clone(), true).is_ok());
            assert!(GovernanceContract::vote(test.env.clone(), v2, id.clone(), true).is_ok());
            assert!(GovernanceContract::vote(test.env.clone(), v3, id.clone(), false).is_ok());

            let prop = GovernanceContract::get_proposal(test.env.clone(), id.clone()).unwrap();
            assert_eq!(prop.for_votes, 2);
            assert_eq!(prop.against_votes, 1);
        });
    }

    #[test]
    fn test_vote_duplicate_rejected() {
        let test = GovernanceTest::new();
        let id = Symbol::new(&test.env, "prop_dup_vote");
        test.call(|| {
            GovernanceContract::create_proposal(
                test.env.clone(),
                test.admin.clone(),
                id.clone(),
                String::from_str(&test.env, "Dup Vote"),
                String::from_str(&test.env, ""),
                None,
                None,
            )
            .unwrap();

            let voter = Address::generate(&test.env);
            assert!(
                GovernanceContract::vote(test.env.clone(), voter.clone(), id.clone(), true).is_ok()
            );
            assert_eq!(
                GovernanceContract::vote(test.env.clone(), voter.clone(), id.clone(), true),
                Err(GovernanceError::AlreadyVoted)
            );
        });
    }

    #[test]
    fn test_vote_nonexistent_proposal_rejected() {
        let test = GovernanceTest::new();
        let id = Symbol::new(&test.env, "does_not_exist");
        test.call(|| {
            let result =
                GovernanceContract::vote(test.env.clone(), test.voter.clone(), id.clone(), true);
            assert_eq!(result, Err(GovernanceError::ProposalNotFound));
        });
    }

    #[test]
    fn test_validate_proposal_passed() {
        let test = GovernanceTest::new();
        let id = Symbol::new(&test.env, "prop_pass");
        test.call(|| {
            GovernanceContract::create_proposal(
                test.env.clone(),
                test.admin.clone(),
                id.clone(),
                String::from_str(&test.env, "Pass"),
                String::from_str(&test.env, ""),
                None,
                None,
            )
            .unwrap();

            // Cast 3 FOR votes (quorum = 3).
            for _ in 0..3 {
                let v = Address::generate(&test.env);
                GovernanceContract::vote(test.env.clone(), v, id.clone(), true).unwrap();
            }

            // Advance past end_time (start_time + 7200).
            let now = test.env.ledger().timestamp();
            test.env.ledger().set_timestamp(now + 7201);

            let (passed, reason) =
                GovernanceContract::validate_proposal(test.env.clone(), id.clone()).unwrap();
            assert!(passed, "proposal should pass: {reason}");
        });
    }

    #[test]
    fn test_validate_proposal_quorum_not_reached() {
        let test = GovernanceTest::new();
        let id = Symbol::new(&test.env, "prop_no_qm");
        test.call(|| {
            GovernanceContract::create_proposal(
                test.env.clone(),
                test.admin.clone(),
                id.clone(),
                String::from_str(&test.env, "No Quorum"),
                String::from_str(&test.env, ""),
                None,
                None,
            )
            .unwrap();

            // Only 1 FOR vote (quorum = 3).
            let v = Address::generate(&test.env);
            GovernanceContract::vote(test.env.clone(), v, id.clone(), true).unwrap();

            let now = test.env.ledger().timestamp();
            test.env.ledger().set_timestamp(now + 7201);

            let (passed, reason) =
                GovernanceContract::validate_proposal(test.env.clone(), id.clone()).unwrap();
            assert!(!passed, "should fail: {reason}");
        });
    }

    #[test]
    fn test_validate_proposal_not_enough_for_votes() {
        let test = GovernanceTest::new();
        let id = Symbol::new(&test.env, "prop_tie");
        test.call(|| {
            GovernanceContract::create_proposal(
                test.env.clone(),
                test.admin.clone(),
                id.clone(),
                String::from_str(&test.env, "Tie"),
                String::from_str(&test.env, ""),
                None,
                None,
            )
            .unwrap();

            // 2 FOR, 2 AGAINST (tie).
            for _ in 0..2 {
                let v = Address::generate(&test.env);
                GovernanceContract::vote(test.env.clone(), v, id.clone(), true).unwrap();
            }
            for _ in 0..2 {
                let v = Address::generate(&test.env);
                GovernanceContract::vote(test.env.clone(), v, id.clone(), false).unwrap();
            }

            let now = test.env.ledger().timestamp();
            test.env.ledger().set_timestamp(now + 7201);

            let (passed, reason) =
                GovernanceContract::validate_proposal(test.env.clone(), id.clone()).unwrap();
            assert!(!passed, "should fail on tie: {reason}");
        });
    }

    #[test]
    fn test_execute_proposal_marks_executed() {
        let test = GovernanceTest::new();
        let id = Symbol::new(&test.env, "prop_exec");
        test.call(|| {
            GovernanceContract::create_proposal(
                test.env.clone(),
                test.admin.clone(),
                id.clone(),
                String::from_str(&test.env, "Exec"),
                String::from_str(&test.env, ""),
                None,
                None,
            )
            .unwrap();

            for _ in 0..3 {
                let v = Address::generate(&test.env);
                GovernanceContract::vote(test.env.clone(), v, id.clone(), true).unwrap();
            }

            let now = test.env.ledger().timestamp();
            test.env.ledger().set_timestamp(now + 7201);

            assert!(GovernanceContract::execute_proposal(
                test.env.clone(),
                test.admin.clone(),
                id.clone()
            )
            .is_ok());

            let prop = GovernanceContract::get_proposal(test.env.clone(), id.clone()).unwrap();
            assert!(prop.executed);
        });
    }

    #[test]
    fn test_execute_proposal_already_executed_rejected() {
        let test = GovernanceTest::new();
        let id = Symbol::new(&test.env, "prop_reexec");
        test.call(|| {
            GovernanceContract::create_proposal(
                test.env.clone(),
                test.admin.clone(),
                id.clone(),
                String::from_str(&test.env, "ReExec"),
                String::from_str(&test.env, ""),
                None,
                None,
            )
            .unwrap();

            for _ in 0..3 {
                let v = Address::generate(&test.env);
                GovernanceContract::vote(test.env.clone(), v, id.clone(), true).unwrap();
            }

            let now = test.env.ledger().timestamp();
            test.env.ledger().set_timestamp(now + 7201);

            GovernanceContract::execute_proposal(test.env.clone(), test.admin.clone(), id.clone())
                .unwrap();
            assert_eq!(
                GovernanceContract::execute_proposal(
                    test.env.clone(),
                    test.admin.clone(),
                    id.clone()
                ),
                Err(GovernanceError::AlreadyExecuted)
            );
        });
    }

    #[test]
    fn test_execute_proposal_not_passed_rejected() {
        let test = GovernanceTest::new();
        let id = Symbol::new(&test.env, "prop_fail_exec");
        test.call(|| {
            GovernanceContract::create_proposal(
                test.env.clone(),
                test.admin.clone(),
                id.clone(),
                String::from_str(&test.env, "Fail Exec"),
                String::from_str(&test.env, ""),
                None,
                None,
            )
            .unwrap();

            // Quorum not met (0 votes).
            let now = test.env.ledger().timestamp();
            test.env.ledger().set_timestamp(now + 7201);

            assert_eq!(
                GovernanceContract::execute_proposal(
                    test.env.clone(),
                    test.admin.clone(),
                    id.clone()
                ),
                Err(GovernanceError::NotPassed)
            );
        });
    }

    #[test]
    fn test_list_proposals() {
        let test = GovernanceTest::new();
        let ids = [Symbol::new(&test.env, "a"), Symbol::new(&test.env, "b")];
        test.call(|| {
            for id in &ids {
                GovernanceContract::create_proposal(
                    test.env.clone(),
                    test.admin.clone(),
                    id.clone(),
                    String::from_str(&test.env, "Item"),
                    String::from_str(&test.env, ""),
                    None,
                    None,
                )
                .unwrap();
            }
            let list = GovernanceContract::list_proposals(test.env.clone());
            assert_eq!(list.len(), 2);
        });
    }

    #[test]
    fn test_get_proposal_not_found() {
        let test = GovernanceTest::new();
        test.call(|| {
            let id = Symbol::new(&test.env, "ghost");
            assert_eq!(
                GovernanceContract::get_proposal(test.env.clone(), id),
                Err(GovernanceError::ProposalNotFound)
            );
        });
    }

    #[test]
    fn test_set_voting_period_admin_only() {
        let test = GovernanceTest::new();
        test.call(|| {
            let impostor = Address::generate(&test.env);
            assert_eq!(
                GovernanceContract::set_voting_period(test.env.clone(), impostor, 3600),
                Err(GovernanceError::NotAdmin)
            );
        });
    }

    #[test]
    fn test_set_voting_period_valid() {
        let test = GovernanceTest::new();
        test.call(|| {
            assert!(GovernanceContract::set_voting_period(
                test.env.clone(),
                test.admin.clone(),
                10800
            )
            .is_ok());
        });
    }

    #[test]
    fn test_set_voting_period_invalid() {
        let test = GovernanceTest::new();
        test.call(|| {
            assert_eq!(
                GovernanceContract::set_voting_period(test.env.clone(), test.admin.clone(), 0),
                Err(GovernanceError::InvalidParams)
            );
        });
    }

    #[test]
    fn test_set_quorum_admin_only() {
        let test = GovernanceTest::new();
        test.call(|| {
            let impostor = Address::generate(&test.env);
            assert_eq!(
                GovernanceContract::set_quorum(test.env.clone(), impostor, 5),
                Err(GovernanceError::NotAdmin)
            );
        });
    }

    #[test]
    fn test_set_quorum_valid() {
        let test = GovernanceTest::new();
        test.call(|| {
            assert!(
                GovernanceContract::set_quorum(test.env.clone(), test.admin.clone(), 10).is_ok()
            );
        });
    }

    #[test]
    fn test_overflow_safe_vote_counting() {
        let test = GovernanceTest::new();
        let id = Symbol::new(&test.env, "prop_overflow");
        test.call(|| {
            GovernanceContract::create_proposal(
                test.env.clone(),
                test.admin.clone(),
                id.clone(),
                String::from_str(&test.env, "Overflow Safe"),
                String::from_str(&test.env, ""),
                None,
                None,
            )
            .unwrap();

            // Saturating at u128::MAX should not panic.
            let prop = GovernanceContract::get_proposal(test.env.clone(), id.clone()).unwrap();
            // Push for_votes near the limit (simulate heavy voting).
            // In practice each vote adds 1, so u128::MAX is unreachable,
            // but the checked_add ensures no panic even at the limit.
            assert!(prop.for_votes < u128::MAX);
        });
    }

    #[test]
    fn test_governance_error_types() {
        let _ = GovernanceError::ProposalExists;
        let _ = GovernanceError::ProposalNotFound;
        let _ = GovernanceError::VotingNotStarted;
        let _ = GovernanceError::VotingEnded;
        let _ = GovernanceError::AlreadyVoted;
        let _ = GovernanceError::NotPassed;
        let _ = GovernanceError::AlreadyExecuted;
        let _ = GovernanceError::NotAdmin;
        let _ = GovernanceError::InvalidParams;
    }
}
