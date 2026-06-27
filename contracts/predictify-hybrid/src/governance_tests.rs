#![cfg(test)]

use crate::governance::{GovernanceContract, GovernanceError};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env, String, Symbol,
};

struct GovernanceFixture {
    env: Env,
    contract_id: Address,
    admin: Address,
    proposer: Address,
    voter_one: Address,
    voter_two: Address,
    voter_three: Address,
}

impl GovernanceFixture {
    fn new(voting_period_seconds: i64, quorum_votes: u128) -> Self {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().with_mut(|li| li.timestamp = 1_000);
        let contract_id = env.register(crate::PredictifyHybrid, ());

        let admin = Address::generate(&env);
        let proposer = Address::generate(&env);
        let voter_one = Address::generate(&env);
        let voter_two = Address::generate(&env);
        let voter_three = Address::generate(&env);

        env.as_contract(&contract_id, || {
            GovernanceContract::initialize(
                env.clone(),
                admin.clone(),
                voting_period_seconds,
                quorum_votes,
            );
        });

        Self {
            env,
            contract_id,
            admin,
            proposer,
            voter_one,
            voter_two,
            voter_three,
        }
    }

    fn create_noop_proposal(&self, id: &str) -> Symbol {
        let proposal_id = Symbol::new(&self.env, id);

        self.env.as_contract(&self.contract_id, || {
            GovernanceContract::create_proposal(
                self.env.clone(),
                self.proposer.clone(),
                proposal_id.clone(),
                String::from_str(&self.env, "Enable feature flag"),
                String::from_str(&self.env, "Roll out predictable governance lifecycle"),
                None,
                None,
            )
            .unwrap();
        });

        proposal_id
    }

    fn vote(&self, voter: Address, proposal_id: Symbol, support: bool) -> Result<(), GovernanceError> {
        self.env.as_contract(&self.contract_id, || {
            GovernanceContract::vote(self.env.clone(), voter, proposal_id, support)
        })
    }

    fn validate(&self, proposal_id: Symbol) -> Result<(bool, String), GovernanceError> {
        self.env.as_contract(&self.contract_id, || {
            GovernanceContract::validate_proposal(self.env.clone(), proposal_id)
        })
    }

    fn execute(&self, proposal_id: Symbol) -> Result<(), GovernanceError> {
        self.env.as_contract(&self.contract_id, || {
            GovernanceContract::execute_proposal(self.env.clone(), self.admin.clone(), proposal_id)
        })
    }

    fn get(&self, proposal_id: Symbol) -> Result<crate::governance::GovernanceProposal, GovernanceError> {
        self.env.as_contract(&self.contract_id, || {
            GovernanceContract::get_proposal(self.env.clone(), proposal_id)
        })
    }

    fn set_quorum(&self, caller: Address, new_quorum: u128) -> Result<(), GovernanceError> {
        self.env.as_contract(&self.contract_id, || {
            GovernanceContract::set_quorum(self.env.clone(), caller, new_quorum)
        })
    }

    fn set_voting_period(&self, caller: Address, new_period_seconds: i64) -> Result<(), GovernanceError> {
        self.env.as_contract(&self.contract_id, || {
            GovernanceContract::set_voting_period(self.env.clone(), caller, new_period_seconds)
        })
    }

    fn advance_time(&self, seconds: u64) {
        self.env
            .ledger()
            .with_mut(|li| li.timestamp = li.timestamp.saturating_add(seconds));
    }

    fn set_delegate(&self, delegator: Address, delegate: Address) -> Result<(), GovernanceError> {
        self.env.as_contract(&self.contract_id, || {
            GovernanceContract::set_delegate(self.env.clone(), delegator, delegate)
        })
    }

    fn unset_delegate(&self, delegator: Address) -> Result<(), GovernanceError> {
        self.env.as_contract(&self.contract_id, || {
            GovernanceContract::unset_delegate(self.env.clone(), delegator)
        })
    }

    fn get_delegate(&self, delegator: Address) -> Option<Address> {
        self.env.as_contract(&self.contract_id, || {
            GovernanceContract::get_delegate(self.env.clone(), delegator)
        })
    }
}

#[test]
fn governance_lifecycle_create_vote_execute_success() {
    let fixture = GovernanceFixture::new(100, 2);
    let proposal_id = fixture.create_noop_proposal("gov_ok_1");

    fixture
        .vote(fixture.voter_one.clone(), proposal_id.clone(), true)
        .unwrap();
    fixture
        .vote(fixture.voter_two.clone(), proposal_id.clone(), true)
        .unwrap();
    fixture
        .vote(fixture.voter_three.clone(), proposal_id.clone(), false)
        .unwrap();

    let execute_early = fixture.execute(proposal_id.clone());
    assert_eq!(execute_early, Err(GovernanceError::VotingStillActive));

    fixture.advance_time(100);

    let validation = fixture.validate(proposal_id.clone()).unwrap();
    assert_eq!(validation, (true, String::from_str(&fixture.env, "passed")));

    fixture.execute(proposal_id.clone()).unwrap();

    let proposal = fixture.get(proposal_id.clone()).unwrap();
    assert!(proposal.executed);

    let execute_twice = fixture.execute(proposal_id);
    assert_eq!(execute_twice, Err(GovernanceError::AlreadyExecuted));
}

#[test]
fn governance_fails_when_quorum_not_reached() {
    let fixture = GovernanceFixture::new(100, 2);
    let proposal_id = fixture.create_noop_proposal("gov_noq_1");

    fixture
        .vote(fixture.voter_one.clone(), proposal_id.clone(), true)
        .unwrap();

    fixture.advance_time(100);

    let validation = fixture.validate(proposal_id.clone()).unwrap();
    assert_eq!(
        validation,
        (false, String::from_str(&fixture.env, "quorum not reached"))
    );

    let execute_result = fixture.execute(proposal_id);
    assert_eq!(execute_result, Err(GovernanceError::NotPassed));
}

#[test]
fn governance_fails_when_for_votes_not_greater_than_against() {
    let fixture = GovernanceFixture::new(100, 1);
    let proposal_id = fixture.create_noop_proposal("gov_tie_1");

    fixture
        .vote(fixture.voter_one.clone(), proposal_id.clone(), true)
        .unwrap();
    fixture
        .vote(fixture.voter_two.clone(), proposal_id.clone(), false)
        .unwrap();

    fixture.advance_time(100);

    let validation = fixture.validate(proposal_id.clone()).unwrap();
    assert_eq!(
        validation,
        (false, String::from_str(&fixture.env, "not enough for votes"))
    );

    let execute_result = fixture.execute(proposal_id);
    assert_eq!(execute_result, Err(GovernanceError::NotPassed));
}

#[test]
fn governance_vote_rejects_duplicate_and_ended_period() {
    let fixture = GovernanceFixture::new(100, 1);
    let proposal_id = fixture.create_noop_proposal("gov_vote_1");

    fixture
        .vote(fixture.voter_one.clone(), proposal_id.clone(), true)
        .unwrap();

    let duplicate_vote = fixture.vote(fixture.voter_one.clone(), proposal_id.clone(), true);
    assert_eq!(duplicate_vote, Err(GovernanceError::AlreadyVoted));

    fixture.advance_time(100);

    let ended_vote = fixture.vote(fixture.voter_two.clone(), proposal_id, true);
    assert_eq!(ended_vote, Err(GovernanceError::VotingEnded));
}

#[test]
fn governance_vote_rejects_before_start_time() {
    let fixture = GovernanceFixture::new(100, 1);
    let proposal_id = fixture.create_noop_proposal("gov_time_1");

    fixture.env.ledger().with_mut(|li| li.timestamp = li.timestamp - 1);

    let before_start = fixture.vote(fixture.voter_one.clone(), proposal_id, true);
    assert_eq!(before_start, Err(GovernanceError::VotingNotStarted));
}

#[test]
fn governance_create_rejects_invalid_inputs_and_uninitialized_state() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(crate::PredictifyHybrid, ());

    let proposer = Address::generate(&env);
    let invalid_before_init = env.as_contract(&contract_id, || {
        GovernanceContract::create_proposal(
            env.clone(),
            proposer.clone(),
            Symbol::new(&env, "gov_init_1"),
            String::from_str(&env, "Title"),
            String::from_str(&env, "Description"),
            None,
            None,
        )
    });
    assert_eq!(invalid_before_init, Err(GovernanceError::NotInitialized));

    let fixture = GovernanceFixture::new(100, 1);

    let empty_title = fixture.env.as_contract(&fixture.contract_id, || {
        GovernanceContract::create_proposal(
            fixture.env.clone(),
            fixture.proposer.clone(),
            Symbol::new(&fixture.env, "gov_bad_1"),
            String::from_str(&fixture.env, ""),
            String::from_str(&fixture.env, "Description"),
            None,
            None,
        )
    });
    assert_eq!(empty_title, Err(GovernanceError::InvalidParams));

    let mismatched_execution_fields = fixture.env.as_contract(&fixture.contract_id, || {
        GovernanceContract::create_proposal(
            fixture.env.clone(),
            fixture.proposer.clone(),
            Symbol::new(&fixture.env, "gov_bad_2"),
            String::from_str(&fixture.env, "Title"),
            String::from_str(&fixture.env, "Description"),
            Some(Address::generate(&fixture.env)),
            None,
        )
    });
    assert_eq!(mismatched_execution_fields, Err(GovernanceError::InvalidParams));

    let duplicate_id = Symbol::new(&fixture.env, "gov_dup_1");
    fixture
        .env
        .as_contract(&fixture.contract_id, || {
            GovernanceContract::create_proposal(
                fixture.env.clone(),
                fixture.proposer.clone(),
                duplicate_id.clone(),
                String::from_str(&fixture.env, "Title"),
                String::from_str(&fixture.env, "Description"),
                None,
                None,
            )
            .unwrap();
        });

    let duplicate_create = fixture.env.as_contract(&fixture.contract_id, || {
        GovernanceContract::create_proposal(
            fixture.env.clone(),
            fixture.proposer.clone(),
            duplicate_id,
            String::from_str(&fixture.env, "Title"),
            String::from_str(&fixture.env, "Description"),
            None,
            None,
        )
    });
    assert_eq!(duplicate_create, Err(GovernanceError::ProposalExists));
}

#[test]
fn governance_validate_and_admin_config_errors() {
    let fixture = GovernanceFixture::new(100, 1);
    let proposal_id = fixture.create_noop_proposal("gov_cfg_1");

    let still_active = fixture.validate(proposal_id.clone());
    assert_eq!(still_active, Err(GovernanceError::VotingStillActive));

    let bad_period = fixture.set_voting_period(fixture.admin.clone(), 0);
    assert_eq!(bad_period, Err(GovernanceError::InvalidParams));

    let bad_quorum = fixture.set_quorum(fixture.admin.clone(), 0);
    assert_eq!(bad_quorum, Err(GovernanceError::InvalidParams));

    let non_admin = Address::generate(&fixture.env);
    let non_admin_set = fixture.set_quorum(non_admin, 2);
    assert_eq!(non_admin_set, Err(GovernanceError::NotAdmin));
}

#[test]
fn governance_set_voting_period_is_applied_to_new_proposals() {
    let fixture = GovernanceFixture::new(100, 1);

    fixture
        .set_voting_period(fixture.admin.clone(), 250)
        .unwrap();

    let proposal_id = fixture.create_noop_proposal("gov_cfg_2");
    let proposal = fixture.get(proposal_id).unwrap();

    assert_eq!(proposal.end_time - proposal.start_time, 250);
}

// ===== DELEGATION TESTS =====

#[test]
fn delegation_set_and_get_round_trip() {
    let fixture = GovernanceFixture::new(100, 1);
    let delegator = Address::generate(&fixture.env);
    let delegate = Address::generate(&fixture.env);

    assert_eq!(fixture.get_delegate(delegator.clone()), None);

    fixture.set_delegate(delegator.clone(), delegate.clone()).unwrap();
    assert_eq!(fixture.get_delegate(delegator.clone()), Some(delegate));
}

#[test]
fn delegation_unset_removes_entry() {
    let fixture = GovernanceFixture::new(100, 1);
    let delegator = Address::generate(&fixture.env);
    let delegate = Address::generate(&fixture.env);

    fixture.set_delegate(delegator.clone(), delegate.clone()).unwrap();
    fixture.unset_delegate(delegator.clone()).unwrap();

    assert_eq!(fixture.get_delegate(delegator), None);
}

#[test]
fn delegation_rejects_self_delegation() {
    let fixture = GovernanceFixture::new(100, 1);
    let addr = Address::generate(&fixture.env);

    let result = fixture.set_delegate(addr.clone(), addr.clone());
    assert_eq!(result, Err(GovernanceError::SelfDelegation));
}

#[test]
fn delegation_rejects_cycle() {
    let fixture = GovernanceFixture::new(100, 1);
    let a = Address::generate(&fixture.env);
    let b = Address::generate(&fixture.env);

    // A delegates to B; then B tries to delegate back to A
    fixture.set_delegate(a.clone(), b.clone()).unwrap();
    let result = fixture.set_delegate(b.clone(), a.clone());
    assert_eq!(result, Err(GovernanceError::DelegationCycle));
}

#[test]
fn delegation_rejects_double_set_without_unset() {
    let fixture = GovernanceFixture::new(100, 1);
    let delegator = Address::generate(&fixture.env);
    let d1 = Address::generate(&fixture.env);
    let d2 = Address::generate(&fixture.env);

    fixture.set_delegate(delegator.clone(), d1.clone()).unwrap();
    let result = fixture.set_delegate(delegator.clone(), d2.clone());
    assert_eq!(result, Err(GovernanceError::DelegationAlreadySet));
}

#[test]
fn delegation_allows_reset_after_unset() {
    let fixture = GovernanceFixture::new(100, 1);
    let delegator = Address::generate(&fixture.env);
    let d1 = Address::generate(&fixture.env);
    let d2 = Address::generate(&fixture.env);

    fixture.set_delegate(delegator.clone(), d1.clone()).unwrap();
    fixture.unset_delegate(delegator.clone()).unwrap();
    fixture.set_delegate(delegator.clone(), d2.clone()).unwrap();

    assert_eq!(fixture.get_delegate(delegator), Some(d2));
}

#[test]
fn delegation_unset_without_active_delegation_errors() {
    let fixture = GovernanceFixture::new(100, 1);
    let delegator = Address::generate(&fixture.env);

    let result = fixture.unset_delegate(delegator);
    assert_eq!(result, Err(GovernanceError::NoDelegationSet));
}

#[test]
fn delegation_griefing_guard_rejects_beyond_cap() {
    let fixture = GovernanceFixture::new(100, 1);
    let delegate = Address::generate(&fixture.env);

    // Fill delegate up to the cap (MAX_INCOMING_DELEGATIONS = 50)
    for _ in 0..50 {
        let d = Address::generate(&fixture.env);
        fixture.set_delegate(d, delegate.clone()).unwrap();
    }

    // The 51st should be rejected
    let overflow = Address::generate(&fixture.env);
    let result = fixture.set_delegate(overflow, delegate.clone());
    assert_eq!(result, Err(GovernanceError::DelegateLimitReached));
}

#[test]
fn delegation_weight_counted_in_vote() {
    // voter_one has two delegators; their single vote should count as 3
    let fixture = GovernanceFixture::new(100, 3);
    let proposal_id = fixture.create_noop_proposal("gov_deleg_1");

    let delegator_a = Address::generate(&fixture.env);
    let delegator_b = Address::generate(&fixture.env);

    fixture
        .set_delegate(delegator_a.clone(), fixture.voter_one.clone())
        .unwrap();
    fixture
        .set_delegate(delegator_b.clone(), fixture.voter_one.clone())
        .unwrap();

    // voter_one votes FOR (weight = 1 own + 2 delegated = 3 → meets quorum of 3)
    fixture
        .vote(fixture.voter_one.clone(), proposal_id.clone(), true)
        .unwrap();

    fixture.advance_time(100);
    let (passed, _) = fixture.validate(proposal_id).unwrap();
    assert!(passed, "proposal should pass with delegated weight");
}

#[test]
fn delegation_against_weight_counted_in_vote() {
    // voter_one has 1 delegator; their against vote counts as 2
    let fixture = GovernanceFixture::new(100, 1);
    let proposal_id = fixture.create_noop_proposal("gov_deleg_2");

    let delegator = Address::generate(&fixture.env);
    fixture
        .set_delegate(delegator.clone(), fixture.voter_one.clone())
        .unwrap();

    // voter_two: 1 FOR; voter_one: 2 AGAINST → proposal should not pass
    fixture
        .vote(fixture.voter_two.clone(), proposal_id.clone(), true)
        .unwrap();
    fixture
        .vote(fixture.voter_one.clone(), proposal_id.clone(), false)
        .unwrap();

    fixture.advance_time(100);
    let (passed, _) = fixture.validate(proposal_id).unwrap();
    assert!(!passed, "proposal should fail when against weight exceeds for");
}

#[test]
fn unset_delegation_restores_delegate_capacity() {
    let fixture = GovernanceFixture::new(100, 1);
    let delegate = Address::generate(&fixture.env);

    // Fill to cap
    let mut delegators = soroban_sdk::Vec::new(&fixture.env);
    for _ in 0..50 {
        let d = Address::generate(&fixture.env);
        fixture.set_delegate(d.clone(), delegate.clone()).unwrap();
        delegators.push_back(d);
    }

    // Overflow still rejected
    let overflow = Address::generate(&fixture.env);
    assert_eq!(
        fixture.set_delegate(overflow.clone(), delegate.clone()),
        Err(GovernanceError::DelegateLimitReached)
    );

    // Remove one delegation
    let first = delegators.get(0).unwrap();
    fixture.unset_delegate(first).unwrap();

    // Now the overflow address can delegate
    fixture.set_delegate(overflow, delegate).unwrap();
}
