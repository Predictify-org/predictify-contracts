#![cfg(test)]

//! Lifecycle tests for the Governance contract: creation, voting, execution,
//! rejection, cancellation, and admin transfer, plus the guard rails on each
//! transition.

use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    Address, Env, String,
};

use governance::{
    GovernanceContract, GovernanceContractClient, GovernanceError, ProposalStatus, VoteChoice,
};

const VOTING_PERIOD: u64 = 3_600;
const START_TS: u64 = 1_735_689_600;

struct TestSetup {
    admin: Address,
    alice: Address,
    bob: Address,
    carol: Address,
    client: GovernanceContractClient<'static>,
}

fn setup() -> (Env, TestSetup) {    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set(LedgerInfo {
        timestamp: START_TS,
        protocol_version: 20,
        sequence_number: 1,
        network_id: [0; 32],
        base_reserve: 10,
        min_temp_entry_ttl: 1,
        min_persistent_entry_ttl: 1,
        max_entry_ttl: 518_400,
    });

    let admin = Address::generate(&env);
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    let carol = Address::generate(&env);

    let contract_id = env.register_contract(None, GovernanceContract);
    let client = GovernanceContractClient::new(&env, &contract_id);
    client.initialize(&admin, &VOTING_PERIOD);

    (
        env,
        TestSetup {
            admin,
            alice,
            bob,
            carol,
            client,
        },
    )
}

fn advance_to(env: &Env, ts: u64) {
    env.ledger().set(LedgerInfo {
        timestamp: ts,
        protocol_version: 20,
        sequence_number: 1,
        network_id: [0; 32],
        base_reserve: 10,
        min_temp_entry_ttl: 1,
        min_persistent_entry_ttl: 1,
        max_entry_ttl: 518_400,
    });
}

#[test]
fn test_initialize_sets_admin_and_version() {
    let (_env, s) = setup();
    assert_eq!(s.client.get_admin(), s.admin);
    assert_eq!(s.client.version(), 1);
}

#[test]
fn test_initialize_rejects_double_init() {
    let (_env, s) = setup();
    let res = s.client.try_initialize(&s.admin, &VOTING_PERIOD);
    assert_eq!(res, Ok(Err(GovernanceError::AlreadyInitialized)));
}

#[test]
fn test_initialize_rejects_zero_voting_period() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register_contract(None, GovernanceContract);
    let client = GovernanceContractClient::new(&env, &contract_id);
    let res = client.try_initialize(&admin, &0u64);
    assert_eq!(res, Ok(Err(GovernanceError::InvalidVotingPeriod)));
}

#[test]
fn test_create_proposal_increments_ids() {
    let (env, s) = setup();
    let id0 = s
        .client
        .create_proposal(&s.alice, &String::from_str(&env, "First"));
    let id1 = s
        .client
        .create_proposal(&s.bob, &String::from_str(&env, "Second"));
    assert_eq!(id0, 0);
    assert_eq!(id1, 1);

    let p = s.client.get_proposal(&id0);
    assert_eq!(p.proposer, s.alice);
    assert_eq!(p.status, ProposalStatus::Active);
    assert_eq!(p.votes_for, 0);
    assert_eq!(p.votes_against, 0);
    assert_eq!(p.created_at, START_TS);
    assert_eq!(p.voting_ends_at, START_TS + VOTING_PERIOD);
}

#[test]
fn test_get_missing_proposal_errors() {
    let (_env, s) = setup();
    let res = s.client.try_get_proposal(&99);
    assert_eq!(res, Ok(Err(GovernanceError::ProposalNotFound)));
}

#[test]
fn test_vote_tallies_and_execution_pass() {
    let (env, s) = setup();
    let id = s
        .client
        .create_proposal(&s.alice, &String::from_str(&env, "Fund grant"));

    s.client.cast_vote(&s.alice, &id, &VoteChoice::For, &10);
    s.client.cast_vote(&s.bob, &id, &VoteChoice::For, &5);
    s.client.cast_vote(&s.carol, &id, &VoteChoice::Against, &4);

    let p = s.client.get_proposal(&id);
    assert_eq!(p.votes_for, 15);
    assert_eq!(p.votes_against, 4);
    assert!(s.client.has_voted(&id, &s.alice));

    advance_to(&env, START_TS + VOTING_PERIOD);
    let outcome = s.client.execute_proposal(&s.admin, &id);
    assert_eq!(outcome, ProposalStatus::Executed);
    assert_eq!(s.client.get_proposal(&id).status, ProposalStatus::Executed);
}

#[test]
fn test_execution_rejects_when_not_passing() {
    let (env, s) = setup();
    let id = s
        .client
        .create_proposal(&s.alice, &String::from_str(&env, "Tie proposal"));
    s.client.cast_vote(&s.alice, &id, &VoteChoice::For, &3);
    s.client.cast_vote(&s.bob, &id, &VoteChoice::Against, &3);

    advance_to(&env, START_TS + VOTING_PERIOD);
    // A tie does not pass (requires strict majority for).
    let outcome = s.client.execute_proposal(&s.carol, &id);
    assert_eq!(outcome, ProposalStatus::Rejected);
}

#[test]
fn test_double_vote_rejected() {
    let (env, s) = setup();
    let id = s
        .client
        .create_proposal(&s.alice, &String::from_str(&env, "P"));
    s.client.cast_vote(&s.alice, &id, &VoteChoice::For, &1);
    let res = s
        .client
        .try_cast_vote(&s.alice, &id, &VoteChoice::For, &1);
    assert_eq!(res, Ok(Err(GovernanceError::AlreadyVoted)));
}

#[test]
fn test_vote_after_close_rejected() {
    let (env, s) = setup();
    let id = s
        .client
        .create_proposal(&s.alice, &String::from_str(&env, "P"));
    advance_to(&env, START_TS + VOTING_PERIOD);
    let res = s
        .client
        .try_cast_vote(&s.bob, &id, &VoteChoice::For, &1);
    assert_eq!(res, Ok(Err(GovernanceError::VotingClosed)));
}

#[test]
fn test_execute_before_close_rejected() {
    let (env, s) = setup();
    let id = s
        .client
        .create_proposal(&s.alice, &String::from_str(&env, "P"));
    let res = s.client.try_execute_proposal(&s.admin, &id);
    assert_eq!(res, Ok(Err(GovernanceError::VotingOpen)));
}

#[test]
fn test_cancel_by_proposer_and_blocks_votes() {
    let (env, s) = setup();
    let id = s
        .client
        .create_proposal(&s.alice, &String::from_str(&env, "P"));
    s.client.cancel_proposal(&s.alice, &id);
    assert_eq!(s.client.get_proposal(&id).status, ProposalStatus::Canceled);

    let res = s
        .client
        .try_cast_vote(&s.bob, &id, &VoteChoice::For, &1);
    assert_eq!(res, Ok(Err(GovernanceError::InvalidStateTransition)));
}

#[test]
fn test_cancel_by_admin() {
    let (env, s) = setup();
    let id = s
        .client
        .create_proposal(&s.alice, &String::from_str(&env, "P"));
    s.client.cancel_proposal(&s.admin, &id);
    assert_eq!(s.client.get_proposal(&id).status, ProposalStatus::Canceled);
}

#[test]
fn test_cancel_by_stranger_rejected() {
    let (env, s) = setup();
    let id = s
        .client
        .create_proposal(&s.alice, &String::from_str(&env, "P"));
    let res = s.client.try_cancel_proposal(&s.bob, &id);
    assert_eq!(res, Ok(Err(GovernanceError::Unauthorized)));
}

#[test]
fn test_double_execute_rejected() {
    let (env, s) = setup();
    let id = s
        .client
        .create_proposal(&s.alice, &String::from_str(&env, "P"));
    s.client.cast_vote(&s.alice, &id, &VoteChoice::For, &1);
    advance_to(&env, START_TS + VOTING_PERIOD);
    s.client.execute_proposal(&s.admin, &id);
    let res = s.client.try_execute_proposal(&s.admin, &id);
    assert_eq!(res, Ok(Err(GovernanceError::InvalidStateTransition)));
}

#[test]
fn test_vote_overflow_guarded() {
    let (env, s) = setup();
    let id = s
        .client
        .create_proposal(&s.alice, &String::from_str(&env, "P"));
    s.client
        .cast_vote(&s.alice, &id, &VoteChoice::For, &u64::MAX);
    let res = s.client.try_cast_vote(&s.bob, &id, &VoteChoice::For, &1);
    assert_eq!(res, Ok(Err(GovernanceError::Overflow)));
}

#[test]
fn test_transfer_admin() {
    let (_env, s) = setup();
    s.client.transfer_admin(&s.admin, &s.bob);
    assert_eq!(s.client.get_admin(), s.bob);
}
