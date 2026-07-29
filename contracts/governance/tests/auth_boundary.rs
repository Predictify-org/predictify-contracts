#![cfg(test)]

//! Authorization-boundary tests for the Governance contract.
//!
//! Every state-changing entrypoint must call `require_auth()` on its acting
//! principal. These tests use `mock_auths` (rather than `mock_all_auths`) so
//! that the host records exactly which address each entrypoint required, then
//! assert that the recorded principal is the expected one. They also assert
//! that role-gated entrypoints reject the wrong caller with a typed error.

use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo, MockAuth, MockAuthInvoke},
    Address, Env, IntoVal, String,
};

use governance::{GovernanceContract, GovernanceContractClient, GovernanceError, VoteChoice};

const VOTING_PERIOD: u64 = 3_600;
const START_TS: u64 = 1_735_689_600;

fn base_env() -> Env {
    let env = Env::default();
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
    env
}

/// `initialize` must require the admin's authorization.
#[test]
fn test_initialize_requires_admin_auth() {
    let env = base_env();
    let admin = Address::generate(&env);
    let contract_id = env.register_contract(None, GovernanceContract);
    let client = GovernanceContractClient::new(&env, &contract_id);

    client
        .mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "initialize",
                args: (admin.clone(), VOTING_PERIOD).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .initialize(&admin, &VOTING_PERIOD);

    // The recorded auth must be attributed to `admin`.
    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    assert_eq!(auths[0].0, admin);
}

/// `create_proposal` must require the proposer's auth, not anyone else's.
#[test]
fn test_create_proposal_requires_proposer_auth() {
    let env = base_env();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register_contract(None, GovernanceContract);
    let client = GovernanceContractClient::new(&env, &contract_id);
    client.initialize(&admin, &VOTING_PERIOD);

    let proposer = Address::generate(&env);
    client
        .mock_auths(&[MockAuth {
            address: &proposer,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "create_proposal",
                args: (proposer.clone(), String::from_str(&env, "P")).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .create_proposal(&proposer, &String::from_str(&env, "P"));

    assert_eq!(env.auths()[0].0, proposer);
}

/// `cast_vote` must require the voter's auth.
#[test]
fn test_cast_vote_requires_voter_auth() {
    let env = base_env();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register_contract(None, GovernanceContract);
    let client = GovernanceContractClient::new(&env, &contract_id);
    client.initialize(&admin, &VOTING_PERIOD);
    let id = client.create_proposal(&admin, &String::from_str(&env, "P"));

    let voter = Address::generate(&env);
    client
        .mock_auths(&[MockAuth {
            address: &voter,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "cast_vote",
                args: (voter.clone(), id, VoteChoice::For, 3u64).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .cast_vote(&voter, &id, &VoteChoice::For, &3);

    assert_eq!(env.auths()[0].0, voter);
}

/// `transfer_admin` must require the current admin's auth.
#[test]
fn test_transfer_admin_requires_current_admin_auth() {
    let env = base_env();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register_contract(None, GovernanceContract);
    let client = GovernanceContractClient::new(&env, &contract_id);
    client.initialize(&admin, &VOTING_PERIOD);

    let new_admin = Address::generate(&env);
    client
        .mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "transfer_admin",
                args: (admin.clone(), new_admin.clone()).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .transfer_admin(&admin, &new_admin);

    assert_eq!(env.auths()[0].0, admin);
    assert_eq!(client.get_admin(), new_admin);
}

/// A non-admin, non-proposer caller cannot cancel a proposal even with a
/// valid signature: the contract rejects it with `Unauthorized`.
#[test]
fn test_cancel_by_stranger_rejected_even_with_auth() {
    let env = base_env();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register_contract(None, GovernanceContract);
    let client = GovernanceContractClient::new(&env, &contract_id);
    client.initialize(&admin, &VOTING_PERIOD);

    let proposer = Address::generate(&env);
    let stranger = Address::generate(&env);
    let id = client.create_proposal(&proposer, &String::from_str(&env, "P"));

    let res = client.try_cancel_proposal(&stranger, &id);
    assert_eq!(res, Ok(Err(GovernanceError::Unauthorized)));
}

/// A non-admin caller cannot transfer admin even with a valid signature.
#[test]
fn test_transfer_admin_by_non_admin_rejected() {
    let env = base_env();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register_contract(None, GovernanceContract);
    let client = GovernanceContractClient::new(&env, &contract_id);
    client.initialize(&admin, &VOTING_PERIOD);

    let stranger = Address::generate(&env);
    let res = client.try_transfer_admin(&stranger, &stranger);
    assert_eq!(res, Ok(Err(GovernanceError::Unauthorized)));
}
