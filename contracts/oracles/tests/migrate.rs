#![cfg(test)]

//! Tests for the oracle registry's migration guard (see `src/migrate.rs`).

use soroban_sdk::{testutils::Address as _, Address, Env};

use oracles::{Error, OraclesContract, OraclesContractClient};

fn deploy(env: &Env) -> OraclesContractClient<'_> {
    let contract_id = env.register(OraclesContract, ());
    OraclesContractClient::new(env, &contract_id)
}

#[test]
fn data_version_defaults_to_one() {
    let env = Env::default();
    let client = deploy(&env);
    assert_eq!(client.data_version(), 1);
}

#[test]
fn migrate_data_bumps_stored_version() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    let admin = Address::generate(&env);

    client.migrate_data(&admin, &1, &2);
    assert_eq!(client.data_version(), 2);

    client.migrate_data(&admin, &2, &5);
    assert_eq!(client.data_version(), 5);
}

#[test]
fn migrate_data_requires_auth() {
    let env = Env::default();
    let client = deploy(&env);
    let admin = Address::generate(&env);

    let result = client.try_migrate_data(&admin, &1, &2);
    assert!(result.is_err(), "migrate_data must require auth");
}

#[test]
fn migrate_data_rejects_stale_expected_version() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    let admin = Address::generate(&env);

    let result = client.try_migrate_data(&admin, &2, &3);
    match result {
        Err(Ok(Error::VersionMismatch)) => {}
        other => panic!("expected VersionMismatch, got {other:?}"),
    }
    assert_eq!(client.data_version(), 1, "version must remain unchanged on rejection");
}

#[test]
fn migrate_data_rejects_non_increasing_target() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    let admin = Address::generate(&env);

    let same = client.try_migrate_data(&admin, &1, &1);
    match same {
        Err(Ok(Error::InvalidTargetVersion)) => {}
        other => panic!("expected InvalidTargetVersion for target == current, got {other:?}"),
    }

    let lower = client.try_migrate_data(&admin, &1, &0);
    match lower {
        Err(Ok(Error::InvalidTargetVersion)) => {}
        other => panic!("expected InvalidTargetVersion for target < current, got {other:?}"),
    }

    assert_eq!(client.data_version(), 1, "version must remain unchanged on rejection");
}
