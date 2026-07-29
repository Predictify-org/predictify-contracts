#![cfg(test)]

use resolution::{
    DataKey, ResolutionContract, ResolutionContractClient, TTL_EXTEND, TTL_THRESHOLD,
};
use soroban_sdk::{
    testutils::{storage::Persistent as _, Address as _, Ledger},
    Address, Env,
};

/// Helper: configure the mock ledger so that the max entry TTL
/// (`sequence + ttl`) fits in `u32`.  Ledger sequence starts at 100_000.
fn setup_ledger(env: &Env, ttl: u32) {
    env.ledger().with_mut(|li| {
        li.timestamp = 1_700_000_000;
        li.sequence_number = 100_000;
        li.protocol_version = 21;
        li.max_entry_ttl = ttl;
        li.min_persistent_entry_ttl = 10;
        li.min_temp_entry_ttl = 10;
    });
}

// ---------------------------------------------------------------------------
// resolve — TTL bump tests
// ---------------------------------------------------------------------------

#[test]
fn resolve_bumps_ttl_on_existing_key() {
    let env = Env::default();
    setup_ledger(&env, TTL_EXTEND + 1000);
    env.mock_all_auths();
    let admin = Address::generate(&env);

    let contract_id = env.register(ResolutionContract, ());

    // Seed a resolution outcome inside the contract's storage so the key exists, with a low TTL.
    let key = DataKey::ResolutionResult(42);
    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&key, &7u32);
        env.storage()
            .persistent()
            .extend_ttl(&key, TTL_THRESHOLD - 10, TTL_THRESHOLD - 10);
    });

    let before: u32 = env.as_contract(&contract_id, || {
        env.storage().persistent().get_ttl(&key)
    });

    let client = ResolutionContractClient::new(&env, &contract_id);
    client.resolve(&admin, &42, &7u32);

    let after: u32 = env.as_contract(&contract_id, || {
        env.storage().persistent().get_ttl(&key)
    });
    assert!(
        after > before,
        "resolve should bump TTL on existing key (before={before}, after={after})",
    );
}

#[test]
fn resolve_sets_and_extends_ttl_on_new_key() {
    let env = Env::default();
    setup_ledger(&env, TTL_EXTEND + 1000);
    env.mock_all_auths();
    let admin = Address::generate(&env);

    let contract_id = env.register(ResolutionContract, ());
    let client = ResolutionContractClient::new(&env, &contract_id);
    client.resolve(&admin, &99, &3u32);

    let key = DataKey::ResolutionResult(99);
    let ttl: u32 = env.as_contract(&contract_id, || {
        env.storage().persistent().get_ttl(&key)
    });
    assert!(
        ttl >= TTL_THRESHOLD,
        "new key should have TTL extended (ttl={ttl})",
    );
    let stored: u32 = env.as_contract(&contract_id, || {
        env.storage().persistent().get(&key).unwrap()
    });
    assert_eq!(stored, 3);
}

#[test]
#[should_panic(expected = "HostError")]
fn resolve_panics_without_auth() {
    let env = Env::default();
    setup_ledger(&env, TTL_EXTEND);
    let attacker = Address::generate(&env);

    let contract_id = env.register(ResolutionContract, ());
    let client = ResolutionContractClient::new(&env, &contract_id);
    client.resolve(&attacker, &1, &0);
}

// ---------------------------------------------------------------------------
// update_config — TTL bump tests
// ---------------------------------------------------------------------------

#[test]
fn update_config_bumps_ttl() {
    let env = Env::default();
    setup_ledger(&env, TTL_EXTEND + 1000);
    env.mock_all_auths();
    let admin = Address::generate(&env);

    let contract_id = env.register(ResolutionContract, ());

    let key = DataKey::MinVotes;
    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&key, &5u32);
        env.storage()
            .persistent()
            .extend_ttl(&key, TTL_THRESHOLD - 10, TTL_THRESHOLD - 10);
    });

    let before: u32 = env.as_contract(&contract_id, || {
        env.storage().persistent().get_ttl(&key)
    });

    let client = ResolutionContractClient::new(&env, &contract_id);
    client.update_config(&admin, &10u32);

    let after: u32 = env.as_contract(&contract_id, || {
        env.storage().persistent().get_ttl(&key)
    });
    assert!(
        after > before,
        "update_config should bump TTL (before={before}, after={after})",
    );
    let stored: u32 = env.as_contract(&contract_id, || {
        env.storage().persistent().get(&key).unwrap()
    });
    assert_eq!(stored, 10);
}

#[test]
#[should_panic(expected = "HostError")]
fn update_config_panics_without_auth() {
    let env = Env::default();
    setup_ledger(&env, TTL_EXTEND);
    let attacker = Address::generate(&env);

    let contract_id = env.register(ResolutionContract, ());
    let client = ResolutionContractClient::new(&env, &contract_id);
    client.update_config(&attacker, &10u32);
}

// ---------------------------------------------------------------------------
// propose — auth-only (storage tracking added in follow-up)
// ---------------------------------------------------------------------------

#[test]
fn propose_succeeds_with_mock_auth() {
    let env = Env::default();
    setup_ledger(&env, TTL_EXTEND);
    env.mock_all_auths();
    let user = Address::generate(&env);

    let contract_id = env.register(ResolutionContract, ());
    let client = ResolutionContractClient::new(&env, &contract_id);
    client.propose(&user, &42);
}

#[test]
#[should_panic(expected = "HostError")]
fn propose_panics_without_auth() {
    let env = Env::default();
    setup_ledger(&env, TTL_EXTEND);
    let attacker = Address::generate(&env);

    let contract_id = env.register(ResolutionContract, ());
    let client = ResolutionContractClient::new(&env, &contract_id);
    client.propose(&attacker, &42);
}

// ---------------------------------------------------------------------------
// version
// ---------------------------------------------------------------------------

#[test]
fn version_returns_7() {
    let env = Env::default();
    let contract_id = env.register(ResolutionContract, ());
    let client = ResolutionContractClient::new(&env, &contract_id);
    assert_eq!(client.version(), 7);
}
