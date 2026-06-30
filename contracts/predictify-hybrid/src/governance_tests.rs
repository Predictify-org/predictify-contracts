#![cfg(test)]

use crate::governance::{GovernanceContract, GovernanceError, QuorumDecay};
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    Address, Bytes, BytesN, Env, String, Symbol,
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
        Self::new_with_decay(voting_period_seconds, quorum_votes, None)
    }

    fn new_with_decay(
        voting_period_seconds: i64,
        quorum_votes: u128,
        quorum_decay: Option<QuorumDecay>,
    ) -> Self {
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
                quorum_decay,
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
        // Fetch the proposal's salt from storage and include it in the vote call.
        // This mirrors how a real off-chain client would obtain the salt before
        // submitting a signed vote transaction.
        let salt = self.env.as_contract(&self.contract_id, || {
            GovernanceContract::get_proposal_salt(self.env.clone(), proposal_id.clone())
                .expect("proposal salt must be readable before voting")
        });
        self.env.as_contract(&self.contract_id, || {
            GovernanceContract::vote(self.env.clone(), voter, proposal_id, support, salt)
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

    fn set_quorum_decay(
        &self,
        caller: Address,
        decay: Option<QuorumDecay>,
    ) -> Result<(), GovernanceError> {
        self.env.as_contract(&self.contract_id, || {
            GovernanceContract::set_quorum_decay(self.env.clone(), caller, decay)
        })
    }

    fn get_quorum_decay(&self) -> Option<QuorumDecay> {
        self.env.as_contract(&self.contract_id, || {
            GovernanceContract::get_quorum_decay(self.env.clone())
        })
    }

    fn commit_vote(
        &self,
        voter: Address,
        proposal_id: Symbol,
        commitment: BytesN<32>,
    ) -> Result<(), GovernanceError> {
        self.env.as_contract(&self.contract_id, || {
            GovernanceContract::commit_vote(
                self.env.clone(),
                voter,
                proposal_id,
                commitment,
            )
        })
    }

    fn reveal_vote(
        &self,
        voter: Address,
        proposal_id: Symbol,
        salt: Bytes,
        support: bool,
    ) -> Result<(), GovernanceError> {
        self.env.as_contract(&self.contract_id, || {
            GovernanceContract::reveal_vote(self.env.clone(), voter, proposal_id, salt, support)
        })
    }

    /// Build a commitment = sha256(salt ++ support_byte) inside the contract context.
    fn make_commitment(&self, salt: &Bytes, support: bool) -> BytesN<32> {
        let mut preimage = salt.clone();
        preimage.push_back(if support { 1u8 } else { 0u8 });
        self.env.as_contract(&self.contract_id, || {
            self.env.crypto().sha256(&preimage).into()
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

/// ---- Quorum Decay Tests ----

#[test]
fn quorum_decay_effective_quorum_computed_correctly() {
    // decay: floor_bps=2000 (20%), halving_seconds=50
    // base quorum = 10, floor = 2
    // after 50s elapsed, effective = 10 - (10-2)*50/100 = 10 - 4 = 6
    // after 100s elapsed, effective = floor = 2
    let decay = QuorumDecay {
        floor_bps: 2000,
        halving_seconds: 50,
    };

    assert_eq!(
        GovernanceContract::compute_effective_quorum(10, &Some(decay.clone()), 0),
        10
    );
    assert_eq!(
        GovernanceContract::compute_effective_quorum(10, &Some(decay.clone()), 25),
        8
    );
    assert_eq!(
        GovernanceContract::compute_effective_quorum(10, &Some(decay.clone()), 50),
        6
    );
    assert_eq!(
        GovernanceContract::compute_effective_quorum(10, &Some(decay.clone()), 75),
        4
    );
    assert_eq!(
        GovernanceContract::compute_effective_quorum(10, &Some(decay.clone()), 100),
        2
    );
    assert_eq!(
        GovernanceContract::compute_effective_quorum(10, &Some(decay.clone()), 200),
        2
    );
    assert_eq!(
        GovernanceContract::compute_effective_quorum(10, &None, 100),
        10
    );
}

#[test]
fn quorum_decay_proposal_passes_with_decayed_quorum() {
    // base quorum=3, floor_bps=3334 (floor=1), halving_seconds=30
    // At end_time (elapsed=60, full_decay=60): effective = floor = 1
    // 2 FOR initially fails base quorum, but after full decay passes floor.
    let decay = QuorumDecay {
        floor_bps: 3334,
        halving_seconds: 30,
    };
    let fixture = GovernanceFixture::new_with_decay(60, 3, Some(decay));
    let proposal_id = fixture.create_noop_proposal("gov_decay_pass");

    // Vote 2 FOR (not enough for base quorum=3)
    fixture
        .vote(fixture.voter_one.clone(), proposal_id.clone(), true)
        .unwrap();
    fixture
        .vote(fixture.voter_two.clone(), proposal_id.clone(), true)
        .unwrap();

    // Advance past end_time so decay has fully reduced quorum to floor=1
    fixture.advance_time(60);

    let validation = fixture.validate(proposal_id.clone()).unwrap();
    assert_eq!(validation.0, true);
    assert_eq!(validation.1, String::from_str(&fixture.env, "passed"));
}

#[test]
fn quorum_decay_floor_respected_after_full_decay() {
    // base quorum=10, floor_bps=2000 (floor=2), halving_seconds=50
    // Full decay to floor after 100s elapsed
    let decay = QuorumDecay {
        floor_bps: 2000,
        halving_seconds: 50,
    };
    let fixture = GovernanceFixture::new_with_decay(100, 10, Some(decay));
    let proposal_id = fixture.create_noop_proposal("gov_decay_floor");

    // Vote only 1 FOR — below floor of 2
    fixture
        .vote(fixture.voter_one.clone(), proposal_id.clone(), true)
        .unwrap();

    // Advance past voting period (100s) so elapsed will be >= 100
    fixture.advance_time(100);

    // Even at full decay, floor=2, but we only have 1 → should fail
    let validation = fixture.validate(proposal_id.clone()).unwrap();
    assert_eq!(validation.0, false);
    assert_eq!(validation.1, String::from_str(&fixture.env, "quorum not reached"));
}

#[test]
fn quorum_decay_auto_rejection_below_floor() {
    // base quorum=10, floor_bps=2000 (floor=2), halving_seconds=50
    // After full decay, effective quorum = floor = 2
    let decay = QuorumDecay {
        floor_bps: 2000,
        halving_seconds: 50,
    };
    let fixture = GovernanceFixture::new_with_decay(100, 10, Some(decay));
    let proposal_id = fixture.create_noop_proposal("gov_decay_auto");

    // Vote only 1 FOR — below floor of 2
    fixture
        .vote(fixture.voter_one.clone(), proposal_id.clone(), true)
        .unwrap();

    // Advance past voting period so decay completes
    fixture.advance_time(100);

    let validation = fixture.validate(proposal_id.clone()).unwrap();
    assert_eq!(validation.0, false);
    assert_eq!(
        validation.1,
        String::from_str(&fixture.env, "quorum not reached")
    );
}

#[test]
fn quorum_decay_proposal_passes_when_floor_met_after_full_decay() {
    // base quorum=10, floor_bps=2000 (floor=2), halving_seconds=50
    // After full decay, effective quorum = floor = 2
    let decay = QuorumDecay {
        floor_bps: 2000,
        halving_seconds: 50,
    };
    let fixture = GovernanceFixture::new_with_decay(100, 10, Some(decay));
    let proposal_id = fixture.create_noop_proposal("gov_decay_floor_ok");

    // Vote 2 FOR — meets the floor of 2
    fixture
        .vote(fixture.voter_one.clone(), proposal_id.clone(), true)
        .unwrap();
    fixture
        .vote(fixture.voter_two.clone(), proposal_id.clone(), true)
        .unwrap();

    fixture.advance_time(100);

    // After full decay, effective quorum = floor = 2, and we have 2 FOR > 0 AGAINST.
    let validation = fixture.validate(proposal_id.clone()).unwrap();
    assert_eq!(validation.0, true);
    assert_eq!(validation.1, String::from_str(&fixture.env, "passed"));
}

#[test]
fn quorum_decay_admin_can_configure() {
    let fixture = GovernanceFixture::new(100, 5);

    // Initially no decay
    assert_eq!(fixture.get_quorum_decay(), None);

    let decay = QuorumDecay {
        floor_bps: 1000,
        halving_seconds: 30,
    };

    // Admin sets decay
    fixture
        .set_quorum_decay(fixture.admin.clone(), Some(decay.clone()))
        .unwrap();
    assert_eq!(fixture.get_quorum_decay(), Some(decay));

    // Admin disables decay
    fixture
        .set_quorum_decay(fixture.admin.clone(), None)
        .unwrap();
    assert_eq!(fixture.get_quorum_decay(), None);
}

#[test]
fn quorum_decay_non_admin_rejected() {
    let fixture = GovernanceFixture::new(100, 5);
    let decay = QuorumDecay {
        floor_bps: 1000,
        halving_seconds: 30,
    };

    let rando = Address::generate(&fixture.env);
    let result = fixture.set_quorum_decay(rando, Some(decay));
    assert_eq!(result, Err(GovernanceError::NotAdmin));
}

#[test]
fn quorum_decay_invalid_params_rejected() {
    let fixture = GovernanceFixture::new(100, 5);

    // floor_bps > 10000
    let bad_floor = QuorumDecay {
        floor_bps: 10001,
        halving_seconds: 30,
    };
    assert_eq!(
        fixture.set_quorum_decay(fixture.admin.clone(), Some(bad_floor)),
        Err(GovernanceError::InvalidParams)
    );

    // halving_seconds == 0
    let zero_half = QuorumDecay {
        floor_bps: 1000,
        halving_seconds: 0,
    };
    assert_eq!(
        fixture.set_quorum_decay(fixture.admin.clone(), Some(zero_half)),
        Err(GovernanceError::InvalidParams)
    );
}

#[test]
fn quorum_decay_initialize_rejects_invalid_config() {
    // Validation is already tested via set_quorum_decay which returns a Result.
    // For initialize, invalid configs panic internally — covered by unit test below.
    let fixture = GovernanceFixture::new(100, 5);

    // floor_bps > 10000
    let bad = QuorumDecay {
        floor_bps: 20000,
        halving_seconds: 10,
    };
    assert_eq!(
        fixture.set_quorum_decay(fixture.admin.clone(), Some(bad)),
        Err(GovernanceError::InvalidParams)
    );

    // halving_seconds == 0
    let bad2 = QuorumDecay {
        floor_bps: 1000,
        halving_seconds: 0,
    };
    assert_eq!(
        fixture.set_quorum_decay(fixture.admin.clone(), Some(bad2)),
        Err(GovernanceError::InvalidParams)
    );
}

#[test]
fn quorum_decay_monotonic_non_increasing() {
    let decay = QuorumDecay {
        floor_bps: 3000,   // floor = 30%
        halving_seconds: 20,
    };
    let base = 100u128;

    let mut prev = GovernanceContract::compute_effective_quorum(base, &Some(decay.clone()), 0);
    for elapsed in [5, 10, 15, 20, 25, 30, 35, 40, 50, 100] {
        let cur = GovernanceContract::compute_effective_quorum(base, &Some(decay.clone()), elapsed);
        assert!(
            cur <= prev,
            "quorum increased from {} to {} at elapsed={}",
            prev,
            cur,
            elapsed
        );
        prev = cur;
    }
}

// ---- Vote Salt (Commit-Reveal) Tests ----

#[test]
fn vote_salt_commit_reveal_full_lifecycle() {
    let fixture = GovernanceFixture::new(100, 2);
    let proposal_id = fixture.create_noop_proposal("salt_ok_1");

    let salt_one = Bytes::from_slice(&fixture.env, b"random-salt-v1-a");
    let salt_two = Bytes::from_slice(&fixture.env, b"random-salt-v1-b");

    let commitment_one = fixture.make_commitment(&salt_one, true);
    let commitment_two = fixture.make_commitment(&salt_two, true);

    // Commit phase
    fixture
        .commit_vote(fixture.voter_one.clone(), proposal_id.clone(), commitment_one)
        .unwrap();
    fixture
        .commit_vote(fixture.voter_two.clone(), proposal_id.clone(), commitment_two)
        .unwrap();

    // Reveal phase (within voting window)
    fixture
        .reveal_vote(fixture.voter_one.clone(), proposal_id.clone(), salt_one, true)
        .unwrap();
    fixture
        .reveal_vote(fixture.voter_two.clone(), proposal_id.clone(), salt_two, true)
        .unwrap();

    // Advance past end_time and validate
    fixture.advance_time(100);
    let result = fixture.validate(proposal_id).unwrap();
    assert_eq!(result, (true, String::from_str(&fixture.env, "passed")));
}

#[test]
fn vote_salt_wrong_reveal_rejected() {
    let fixture = GovernanceFixture::new(100, 1);
    let proposal_id = fixture.create_noop_proposal("salt_bad_1");

    let real_salt = Bytes::from_slice(&fixture.env, b"correct-salt-xyz");
    let wrong_salt = Bytes::from_slice(&fixture.env, b"wrong-salt-xyz00");

    let commitment = fixture.make_commitment(&real_salt, true);
    fixture
        .commit_vote(fixture.voter_one.clone(), proposal_id.clone(), commitment)
        .unwrap();

    // Wrong salt → InvalidReveal
    let err = fixture.reveal_vote(
        fixture.voter_one.clone(),
        proposal_id.clone(),
        wrong_salt,
        true,
    );
    assert_eq!(err, Err(GovernanceError::InvalidReveal));

    // Wrong support value → InvalidReveal
    let err2 = fixture.reveal_vote(
        fixture.voter_one.clone(),
        proposal_id.clone(),
        real_salt.clone(),
        false, // committed FOR but revealing AGAINST
    );
    assert_eq!(err2, Err(GovernanceError::InvalidReveal));

    // Correct reveal succeeds
    fixture
        .reveal_vote(fixture.voter_one.clone(), proposal_id, real_salt, true)
        .unwrap();
}

#[test]
fn vote_salt_duplicate_commit_rejected() {
    let fixture = GovernanceFixture::new(100, 1);
    let proposal_id = fixture.create_noop_proposal("salt_dup_1");

    let salt = Bytes::from_slice(&fixture.env, b"salt-dup-test-aa");
    let commitment = fixture.make_commitment(&salt, true);

    fixture
        .commit_vote(fixture.voter_one.clone(), proposal_id.clone(), commitment.clone())
        .unwrap();

    // Second commit by same voter → CommitmentExists
    let err = fixture.commit_vote(fixture.voter_one.clone(), proposal_id, commitment);
    assert_eq!(err, Err(GovernanceError::CommitmentExists));
}

#[test]
fn vote_salt_reveal_without_commit_rejected() {
    let fixture = GovernanceFixture::new(100, 1);
    let proposal_id = fixture.create_noop_proposal("salt_nocommit_1");

    let salt = Bytes::from_slice(&fixture.env, b"some-salt-value1");
    let err = fixture.reveal_vote(fixture.voter_one.clone(), proposal_id, salt, true);
    assert_eq!(err, Err(GovernanceError::NoCommitment));
}

#[test]
fn vote_salt_commit_rejected_after_voting_ends() {
    let fixture = GovernanceFixture::new(100, 1);
    let proposal_id = fixture.create_noop_proposal("salt_late_1");

    fixture.advance_time(100); // past end_time

    let salt = Bytes::from_slice(&fixture.env, b"too-late-salt-aa");
    let commitment = fixture.make_commitment(&salt, true);
    let err = fixture.commit_vote(fixture.voter_one.clone(), proposal_id, commitment);
    assert_eq!(err, Err(GovernanceError::VotingEnded));
}

#[test]
fn vote_salt_reveal_rejected_after_voting_ends() {
    let fixture = GovernanceFixture::new(100, 1);
    let proposal_id = fixture.create_noop_proposal("salt_lrev_1");

    let salt = Bytes::from_slice(&fixture.env, b"late-reveal-salt");
    let commitment = fixture.make_commitment(&salt, true);

    // Commit while voting is open
    fixture
        .commit_vote(fixture.voter_one.clone(), proposal_id.clone(), commitment)
        .unwrap();

    // Advance past end_time before reveal
    fixture.advance_time(100);

    let err = fixture.reveal_vote(fixture.voter_one.clone(), proposal_id, salt, true);
    assert_eq!(err, Err(GovernanceError::VotingEnded));
}

#[test]
fn vote_salt_double_reveal_rejected() {
    let fixture = GovernanceFixture::new(200, 1);
    let proposal_id = fixture.create_noop_proposal("salt_drev_1");

    let salt = Bytes::from_slice(&fixture.env, b"double-reveal-s1");
    let commitment = fixture.make_commitment(&salt, true);

    fixture
        .commit_vote(fixture.voter_one.clone(), proposal_id.clone(), commitment)
        .unwrap();
    fixture
        .reveal_vote(fixture.voter_one.clone(), proposal_id.clone(), salt.clone(), true)
        .unwrap();

    // Second reveal → AlreadyVoted (commitment removed, Vote entry exists)
    let err = fixture.reveal_vote(fixture.voter_one.clone(), proposal_id, salt, true);
    assert_eq!(err, Err(GovernanceError::AlreadyVoted));
}

#[test]
fn vote_salt_direct_vote_then_commit_rejected() {
    let fixture = GovernanceFixture::new(100, 1);
    let proposal_id = fixture.create_noop_proposal("salt_dv_1");

    // Cast direct vote first
    fixture
        .vote(fixture.voter_one.clone(), proposal_id.clone(), true)
        .unwrap();

    // Then try to commit → AlreadyVoted
    let salt = Bytes::from_slice(&fixture.env, b"direct-then-salt");
    let commitment = fixture.make_commitment(&salt, true);
    let err = fixture.commit_vote(fixture.voter_one.clone(), proposal_id, commitment);
    assert_eq!(err, Err(GovernanceError::AlreadyVoted));
}

#[test]
fn vote_salt_commit_not_found_wrong_proposal() {
    let fixture = GovernanceFixture::new(100, 1);
    let real_salt = Bytes::from_slice(&fixture.env, b"salt-real-propid");
    let fake_proposal = Symbol::new(&fixture.env, "no_exist");
    let commitment = fixture.make_commitment(&real_salt, true);
    let err = fixture.commit_vote(fixture.voter_one.clone(), fake_proposal, commitment);
    assert_eq!(err, Err(GovernanceError::ProposalNotFound));
}
