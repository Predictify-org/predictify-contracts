#![cfg(test)]

//! Event-emission tests for the Governance contract. Verifies that every
//! lifecycle transition publishes a structured event with the expected stable
//! topic symbol and payload, so off-chain indexers can rely on the schema.

use soroban_sdk::{
    testutils::{Address as _, Events, Ledger, LedgerInfo},
    Address, Env, String, Symbol, TryIntoVal, Val,
};

use governance::{GovernanceContract, GovernanceContractClient, ProposalStatus, VoteChoice};

const VOTING_PERIOD: u64 = 3_600;
const START_TS: u64 = 1_735_689_600;

fn ledger(env: &Env, ts: u64) {
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

/// Returns true if any published event has `symbol` as its first topic.
fn has_event_topic(env: &Env, contract: &Address, symbol: &str) -> bool {
    let target = Symbol::new(env, symbol);
    env.events().all().iter().any(|(cid, topics, _data)| {
        if &cid != contract || topics.is_empty() {
            return false;
        }
        let first: Val = topics.get(0).unwrap();
        let first_sym: Result<Symbol, _> = first.try_into_val(env);
        matches!(first_sym, Ok(sym) if sym == target)
    })
}

fn setup() -> (Env, Address, GovernanceContractClient<'static>, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    ledger(&env, START_TS);

    let admin = Address::generate(&env);
    let voter = Address::generate(&env);
    let contract_id = env.register_contract(None, GovernanceContract);
    let client = GovernanceContractClient::new(&env, &contract_id);
    client.initialize(&admin, &VOTING_PERIOD);

    (env, contract_id, client, admin, voter)
}

#[test]
fn test_initialize_emits_event() {
    let (env, cid, _client, _admin, _voter) = setup();
    assert!(has_event_topic(&env, &cid, "gov_init"));
}

#[test]
fn test_create_proposal_emits_created_and_status() {
    let (env, cid, client, _admin, _voter) = setup();
    client.create_proposal(&_admin, &String::from_str(&env, "P"));
    assert!(has_event_topic(&env, &cid, "gov_created"));
    assert!(has_event_topic(&env, &cid, "gov_status"));
}

#[test]
fn test_cast_vote_emits_voted() {
    let (env, cid, client, _admin, voter) = setup();
    let id = client.create_proposal(&_admin, &String::from_str(&env, "P"));
    client.cast_vote(&voter, &id, &VoteChoice::For, &7);
    assert!(has_event_topic(&env, &cid, "gov_voted"));
}

#[test]
fn test_execute_emits_executed() {
    let (env, cid, client, admin, voter) = setup();
    let id = client.create_proposal(&admin, &String::from_str(&env, "P"));
    client.cast_vote(&voter, &id, &VoteChoice::For, &1);
    ledger(&env, START_TS + VOTING_PERIOD);
    client.execute_proposal(&admin, &id);
    assert!(has_event_topic(&env, &cid, "gov_executed"));
}

#[test]
fn test_reject_emits_rejected() {
    let (env, cid, client, admin, _voter) = setup();
    let id = client.create_proposal(&admin, &String::from_str(&env, "P"));
    ledger(&env, START_TS + VOTING_PERIOD);
    client.execute_proposal(&admin, &id);
    assert!(has_event_topic(&env, &cid, "gov_rejected"));
}

#[test]
fn test_cancel_emits_canceled() {
    let (env, cid, client, admin, _voter) = setup();
    let id = client.create_proposal(&admin, &String::from_str(&env, "P"));
    client.cancel_proposal(&admin, &id);
    assert!(has_event_topic(&env, &cid, "gov_canceled"));
}

#[test]
fn test_transfer_admin_emits_event() {
    let (env, cid, client, admin, voter) = setup();
    client.transfer_admin(&admin, &voter);
    assert!(has_event_topic(&env, &cid, "gov_admin_xf"));
}

#[test]
fn test_vote_event_payload_carries_tallies() {
    let (env, _cid, client, admin, voter) = setup();
    let id = client.create_proposal(&admin, &String::from_str(&env, "P"));
    client.cast_vote(&voter, &id, &VoteChoice::For, &9);

    // Locate the gov_voted event and assert its data payload matches
    // (choice, weight, votes_for, votes_against, timestamp).
    let target = Symbol::new(&env, "gov_voted");
    let found = env.events().all().iter().find_map(|(_cid, topics, data)| {
        let first: Val = topics.get(0)?;
        let sym: Symbol = first.try_into_val(&env).ok()?;
        if sym == target {
            Some(data)
        } else {
            None
        }
    });
    let data = found.expect("gov_voted event present");
    // Decode the payload into a Rust tuple and compare by value. Comparing
    // `Val == Val` directly is wrong for object-typed payloads: the tuple is
    // stored behind a host object handle, so two structurally-equal tuples
    // carry different handles and never compare equal.
    let decoded: (VoteChoice, u64, u64, u64, u64) = data.try_into_val(&env).unwrap();
    assert_eq!(decoded, (VoteChoice::For, 9u64, 9u64, 0u64, START_TS));
}

#[test]
fn test_status_event_payload_on_execution() {
    let (env, _cid, client, admin, voter) = setup();
    let id = client.create_proposal(&admin, &String::from_str(&env, "P"));
    client.cast_vote(&voter, &id, &VoteChoice::For, &1);
    ledger(&env, START_TS + VOTING_PERIOD);
    client.execute_proposal(&admin, &id);

    // The final gov_status event should carry (Active, Executed, timestamp).
    let target = Symbol::new(&env, "gov_status");
    let last = env
        .events()
        .all()
        .iter()
        .filter_map(|(_cid, topics, data)| {
            let first: Val = topics.get(0)?;
            let sym: Symbol = first.try_into_val(&env).ok()?;
            if sym == target {
                Some(data)
            } else {
                None
            }
        })
        .last()
        .expect("gov_status event present");
    // Decode into a Rust tuple and compare by value (see note above).
    let decoded: (ProposalStatus, ProposalStatus, u64) = last.try_into_val(&env).unwrap();
    assert_eq!(
        decoded,
        (
            ProposalStatus::Active,
            ProposalStatus::Executed,
            START_TS + VOTING_PERIOD,
        )
    );
}
