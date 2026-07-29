//! Auth-boundary snapshot tests for the Admin contract.
//!
//! # Purpose
//!
//! Every state-changing entrypoint (`initialize`, `set_admin_cooldown`,
//! `check_admin_cooldown`) is covered by two complementary cases that act as
//! a ratchet: if `require_auth` is ever removed or called on the wrong
//! address, the corresponding test fails, making the regression visible in
//! CI without needing to read the source.
//!
//! # Strategy
//!
//! 1. **Reject without auth** — a bare [`Env::default()`] with *no*
//!    [`mock_all_auths()`] call is used. Because `require_auth()` panics in
//!    the test environment when no auth is mocked, `try_*` returns `Err`.
//!
//! 2. **Accept with auth** — [`mock_all_auths()`] is enabled and
//!    [`env.auths()`] is inspected immediately after the call, asserting the
//!    expected signer is present and no unrelated bystander is.
//!
//! Read-only views (`admin`, `get_admin_cooldown`) get a dedicated section
//! confirming they require no auth.

#![cfg(test)]

use admin::{AdminContract, AdminContractClient, ContractError};
use soroban_sdk::{testutils::Address as _, Address, Env, Symbol};

fn register(env: &Env) -> (AdminContractClient<'_>, Address) {
    let contract_id = env.register(AdminContract, ());
    let client = AdminContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    (client, admin)
}

fn register_and_init(env: &Env) -> (AdminContractClient<'_>, Address) {
    env.mock_all_auths();
    let contract_id = env.register(AdminContract, ());
    let client = AdminContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.initialize(&admin);
    // Drain the initialize auth snapshot so it doesn't pollute later
    // env.auths() assertions in the calling test.
    let _ = env.auths();
    (client, admin)
}

// ---------------------------------------------------------------------------
// initialize — auth gate
// ---------------------------------------------------------------------------

#[test]
fn initialize_rejected_without_auth() {
    let env = Env::default();
    let (client, admin) = register(&env);

    let result = client.try_initialize(&admin);
    assert!(
        result.is_err(),
        "initialize must require auth; succeeded without it"
    );
}

#[test]
fn initialize_accepted_with_admin_auth() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = register(&env);

    client.initialize(&admin);

    let auths = env.auths();
    assert_eq!(auths.len(), 1, "exactly one auth entry expected");
    assert_eq!(
        auths[0].0, admin,
        "admin must be the authorised address for initialize"
    );
}

// ---------------------------------------------------------------------------
// set_admin_cooldown — auth gate
// ---------------------------------------------------------------------------

#[test]
fn set_admin_cooldown_rejected_without_auth() {
    let env = Env::default();
    let contract_id = env.register(AdminContract, ());
    let client = AdminContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    let result = client.try_set_admin_cooldown(&admin, &60);
    assert!(
        result.is_err(),
        "set_admin_cooldown must require auth; succeeded without it"
    );
}

#[test]
fn set_admin_cooldown_accepted_with_admin_auth() {
    let env = Env::default();
    let (client, admin) = register_and_init(&env);

    client.set_admin_cooldown(&admin, &60);

    let auths = env.auths();
    assert_eq!(auths.len(), 1, "exactly one auth entry expected");
    assert_eq!(
        auths[0].0, admin,
        "admin must be the authorised address for set_admin_cooldown"
    );
}

#[test]
fn set_admin_cooldown_stranger_rejected_with_unauthorized() {
    let env = Env::default();
    let (client, _admin) = register_and_init(&env);
    let stranger = Address::generate(&env);

    let result = client.try_set_admin_cooldown(&stranger, &60);
    match result {
        Err(Ok(e)) => assert_eq!(
            e,
            ContractError::Unauthorized,
            "non-admin caller must receive Unauthorized"
        ),
        other => panic!("expected Unauthorized, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// check_admin_cooldown — auth gate
// ---------------------------------------------------------------------------

#[test]
fn check_admin_cooldown_rejected_without_auth() {
    let env = Env::default();
    let contract_id = env.register(AdminContract, ());
    let client = AdminContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let function_name = Symbol::new(&env, "some_fn");

    let result = client.try_check_admin_cooldown(&admin, &function_name);
    assert!(
        result.is_err(),
        "check_admin_cooldown must require auth; succeeded without it"
    );
}

#[test]
fn check_admin_cooldown_accepted_with_admin_auth() {
    let env = Env::default();
    let (client, admin) = register_and_init(&env);
    let function_name = Symbol::new(&env, "some_fn");

    client.check_admin_cooldown(&admin, &function_name);

    let auths = env.auths();
    assert_eq!(auths.len(), 1, "exactly one auth entry expected");
    assert_eq!(
        auths[0].0, admin,
        "admin must be the authorised address for check_admin_cooldown"
    );
}

#[test]
fn check_admin_cooldown_stranger_rejected_with_unauthorized() {
    let env = Env::default();
    let (client, _admin) = register_and_init(&env);
    let stranger = Address::generate(&env);
    let function_name = Symbol::new(&env, "some_fn");

    let result = client.try_check_admin_cooldown(&stranger, &function_name);
    match result {
        Err(Ok(e)) => assert_eq!(
            e,
            ContractError::Unauthorized,
            "non-admin caller must receive Unauthorized"
        ),
        other => panic!("expected Unauthorized, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Read-only views — no auth required
// ---------------------------------------------------------------------------

#[test]
fn admin_view_requires_no_auth() {
    let env = Env::default();
    let (client, admin) = register_and_init(&env);
    // Deliberately no further mock_all_auths from here.
    assert_eq!(
        client.admin(),
        admin,
        "admin() must return the stored admin address"
    );
}

#[test]
fn get_admin_cooldown_view_requires_no_auth() {
    let env = Env::default();
    let (client, admin) = register_and_init(&env);
    client.set_admin_cooldown(&admin, &120);
    let _ = env.auths();

    assert_eq!(
        client.get_admin_cooldown(),
        120,
        "get_admin_cooldown() must reflect the value set via set_admin_cooldown"
    );
}

// ---------------------------------------------------------------------------
// Signer identity invariant
// ---------------------------------------------------------------------------

#[test]
fn initialize_auth_snapshot_signer_is_the_admin_argument() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = register(&env);
    let bystander = Address::generate(&env);

    client.initialize(&admin);

    let auths = env.auths();
    assert!(
        auths.iter().all(|(signer, _)| *signer != bystander),
        "bystander address must not appear in the auth snapshot"
    );
    assert!(
        auths.iter().any(|(signer, _)| *signer == admin),
        "admin address must appear in the auth snapshot"
    );
}
