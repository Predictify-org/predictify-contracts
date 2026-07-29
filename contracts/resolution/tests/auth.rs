//! Auth-context tests for the Resolution contract.
//!
//! Every state-changing entrypoint (`resolve`, `propose`, `update_config`) is
//! covered under two complementary harnesses:
//!
//! 1. **Bare env, no mocking** — `require_auth()` panics when no auth is
//!    mocked, so calling on a fresh [`Env::default()`] must panic. This
//!    proves the auth gate is present.
//! 2. **`mock_all_auths()`** — the call must succeed, and [`Env::auths`]
//!    must report the expected caller as the sole signer. This proves the
//!    *correct* address is the one being authorized, not an unrelated one.

use soroban_sdk::{testutils::Address as _, Address, Env};

use resolution::{ResolutionContract, ResolutionContractClient};

fn deploy(env: &Env) -> ResolutionContractClient<'_> {
    let contract_id = env.register(ResolutionContract, ());
    ResolutionContractClient::new(env, &contract_id)
}

// ---------------------------------------------------------------------------
// resolve
// ---------------------------------------------------------------------------

#[test]
#[should_panic]
fn resolve_rejected_without_auth() {
    let env = Env::default();
    let client = deploy(&env);
    let admin = Address::generate(&env);

    client.resolve(&admin, &1u64, &2u32);
}

#[test]
fn resolve_accepted_with_auth_and_signer_captured() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    let admin = Address::generate(&env);

    client.resolve(&admin, &1u64, &2u32);

    let auths = env.auths();
    assert_eq!(auths.len(), 1, "exactly one auth entry expected");
    assert_eq!(auths[0].0, admin, "admin must be the authorized signer");
}

// ---------------------------------------------------------------------------
// propose
// ---------------------------------------------------------------------------

#[test]
#[should_panic]
fn propose_rejected_without_auth() {
    let env = Env::default();
    let client = deploy(&env);
    let user = Address::generate(&env);

    client.propose(&user, &1u64);
}

#[test]
fn propose_accepted_with_auth_and_signer_captured() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    let user = Address::generate(&env);

    client.propose(&user, &1u64);

    let auths = env.auths();
    assert_eq!(auths.len(), 1, "exactly one auth entry expected");
    assert_eq!(auths[0].0, user, "user must be the authorized signer");
}

// ---------------------------------------------------------------------------
// update_config
// ---------------------------------------------------------------------------

#[test]
#[should_panic]
fn update_config_rejected_without_auth() {
    let env = Env::default();
    let client = deploy(&env);
    let admin = Address::generate(&env);

    client.update_config(&admin, &5u32);
}

#[test]
fn update_config_accepted_with_auth_and_signer_captured() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    let admin = Address::generate(&env);

    client.update_config(&admin, &5u32);

    let auths = env.auths();
    assert_eq!(auths.len(), 1, "exactly one auth entry expected");
    assert_eq!(auths[0].0, admin, "admin must be the authorized signer");
}

// ---------------------------------------------------------------------------
// Signer-identity invariant across a mixed-caller sequence
// ---------------------------------------------------------------------------

/// The auth snapshot must name the exact caller passed to each entrypoint,
/// not some other in-scope address, even when multiple distinct callers act
/// on the same contract instance in sequence.
#[test]
fn each_entrypoint_captures_its_own_distinct_signer() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);

    let resolver = Address::generate(&env);
    let proposer = Address::generate(&env);
    let configurer = Address::generate(&env);

    client.resolve(&resolver, &1u64, &0u32);
    assert_eq!(env.auths()[0].0, resolver);

    client.propose(&proposer, &2u64);
    assert_eq!(env.auths()[0].0, proposer);

    client.update_config(&configurer, &3u32);
    assert_eq!(env.auths()[0].0, configurer);
}
