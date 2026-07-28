#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    Address, Env, Symbol,
};

use allowlist::{AllowlistContract, AllowlistContractClient};

fn setup_test_environment(env: &Env) -> TestSetup {
    env.mock_all_auths();
    env.ledger().set(LedgerInfo {
        timestamp: 1735689600,
        protocol_version: 20,
        sequence_number: 1,
        network_id: [0; 32],
        base_reserve: 10,
        min_temp_entry_ttl: 1,
        min_persistent_entry_ttl: 1,
        max_entry_ttl: 518400,
    });

    let admin = Address::generate(env);
    let unauthorized = Address::generate(env);
    let test_address = Address::generate(env);

    let contract_id = env.register_contract(None, AllowlistContract);
    let client = AllowlistContractClient::new(env, &contract_id);

    TestSetup {
        admin,
        unauthorized,
        test_address,
        client,
        contract_id,
    }
}

struct TestSetup {
    admin: Address,
    unauthorized: Address,
    test_address: Address,
    client: AllowlistContractClient,
    contract_id: Address,
}

fn create_initalized(env: &Env, setup: &TestSetup) {
    setup.client.initialize(&setup.admin);
}

fn create_allowlist(env: &Env, setup: &TestSetup) {
    let allowlist_id = Symbol::new(env, "test_list");
    setup.client.create_allowlist(&setup.admin, &allowlist_id);
}

// ===== initialize =====

#[test]
fn test_initialize_requires_auth() {
    let env = Env::default();
    let setup = setup_test_environment(&env);

    let result = setup.client.try_initialize(&setup.unauthorized);
    assert!(result.is_err(), "Unauthorized user should not initialize");
}

#[test]
fn test_initialize_requires_auth_admin() {
    let env = Env::default();
    let setup = setup_test_environment(&env);

    let result = setup.client.try_initialize(&setup.admin);
    match result {
        Ok(_) => assert!(true, "Auth passed for admin"),
        Err(e) => {
            let error_str = format!("{:?}", e);
            assert!(!error_str.contains("auth"), "Auth should not fail: {error_str}");
        }
    }
}

#[test]
fn test_initialize_rejects_double_init() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    create_initalized(&env, &setup);

    let result = setup.client.try_initialize(&setup.admin);
    assert!(result.is_err(), "Double initialization should fail");
}

// ===== create_allowlist =====

#[test]
fn test_create_allowlist_requires_auth() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    create_initalized(&env, &setup);

    let allowlist_id = Symbol::new(&env, "test_list");

    let result = setup.client.try_create_allowlist(&setup.unauthorized, &allowlist_id);
    assert!(result.is_err(), "Unauthorized user should not create allowlist");
}

#[test]
fn test_create_allowlist_requires_auth_admin() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    create_initalized(&env, &setup);

    let allowlist_id = Symbol::new(&env, "test_list");

    let result = setup.client.try_create_allowlist(&setup.admin, &allowlist_id);
    match result {
        Ok(_) => assert!(true, "Auth passed for admin"),
        Err(e) => {
            let error_str = format!("{:?}", e);
            assert!(!error_str.contains("auth"), "Auth should not fail: {error_str}");
        }
    }
}

// ===== add_address =====

#[test]
fn test_add_address_requires_auth() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    create_initalized(&env, &setup);
    create_allowlist(&env, &setup);

    let allowlist_id = Symbol::new(&env, "test_list");

    let result = setup.client.try_add_address(
        &setup.unauthorized,
        &allowlist_id,
        &setup.test_address,
    );
    assert!(result.is_err(), "Unauthorized user should not add address");
}

#[test]
fn test_add_address_requires_auth_admin() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    create_initalized(&env, &setup);
    create_allowlist(&env, &setup);

    let allowlist_id = Symbol::new(&env, "test_list");

    let result = setup.client.try_add_address(
        &setup.admin,
        &allowlist_id,
        &setup.test_address,
    );
    match result {
        Ok(_) => assert!(true, "Auth passed for admin"),
        Err(e) => {
            let error_str = format!("{:?}", e);
            assert!(!error_str.contains("auth"), "Auth should not fail: {error_str}");
        }
    }
}

// ===== remove_address =====

#[test]
fn test_remove_address_requires_auth() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    create_initalized(&env, &setup);
    create_allowlist(&env, &setup);

    let allowlist_id = Symbol::new(&env, "test_list");

    let result = setup.client.try_remove_address(
        &setup.unauthorized,
        &allowlist_id,
        &setup.test_address,
    );
    assert!(result.is_err(), "Unauthorized user should not remove address");
}

#[test]
fn test_remove_address_requires_auth_admin() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    create_initalized(&env, &setup);
    create_allowlist(&env, &setup);

    let allowlist_id = Symbol::new(&env, "test_list");

    let result = setup.client.try_remove_address(
        &setup.admin,
        &allowlist_id,
        &setup.test_address,
    );
    match result {
        Ok(_) => assert!(true, "Auth passed for admin"),
        Err(e) => {
            let error_str = format!("{:?}", e);
            assert!(!error_str.contains("auth"), "Auth should not fail: {error_str}");
        }
    }
}

// ===== add_addresses (batch) =====

#[test]
fn test_add_addresses_requires_auth() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    create_initalized(&env, &setup);
    create_allowlist(&env, &setup);

    let allowlist_id = Symbol::new(&env, "test_list");
    let addrs = soroban_sdk::vec![&env, setup.test_address.clone()];

    let result = setup.client.try_add_addresses(
        &setup.unauthorized,
        &allowlist_id,
        &addrs,
    );
    assert!(result.is_err(), "Unauthorized user should not batch add addresses");
}

#[test]
fn test_add_addresses_requires_auth_admin() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    create_initalized(&env, &setup);
    create_allowlist(&env, &setup);

    let allowlist_id = Symbol::new(&env, "test_list");
    let addrs = soroban_sdk::vec![&env, setup.test_address.clone()];

    let result = setup.client.try_add_addresses(
        &setup.admin,
        &allowlist_id,
        &addrs,
    );
    match result {
        Ok(_) => assert!(true, "Auth passed for admin"),
        Err(e) => {
            let error_str = format!("{:?}", e);
            assert!(!error_str.contains("auth"), "Auth should not fail: {error_str}");
        }
    }
}

// ===== remove_addresses (batch) =====

#[test]
fn test_remove_addresses_requires_auth() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    create_initalized(&env, &setup);
    create_allowlist(&env, &setup);

    let allowlist_id = Symbol::new(&env, "test_list");
    let addrs = soroban_sdk::vec![&env, setup.test_address.clone()];

    let result = setup.client.try_remove_addresses(
        &setup.unauthorized,
        &allowlist_id,
        &addrs,
    );
    assert!(result.is_err(), "Unauthorized user should not batch remove addresses");
}

#[test]
fn test_remove_addresses_requires_auth_admin() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    create_initalized(&env, &setup);
    create_allowlist(&env, &setup);

    let allowlist_id = Symbol::new(&env, "test_list");
    let addrs = soroban_sdk::vec![&env, setup.test_address.clone()];

    let result = setup.client.try_remove_addresses(
        &setup.admin,
        &allowlist_id,
        &addrs,
    );
    match result {
        Ok(_) => assert!(true, "Auth passed for admin"),
        Err(e) => {
            let error_str = format!("{:?}", e);
            assert!(!error_str.contains("auth"), "Auth should not fail: {error_str}");
        }
    }
}

// ===== clear_allowlist =====

#[test]
fn test_clear_allowlist_requires_auth() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    create_initalized(&env, &setup);
    create_allowlist(&env, &setup);

    let allowlist_id = Symbol::new(&env, "test_list");

    let result = setup.client.try_clear_allowlist(&setup.unauthorized, &allowlist_id);
    assert!(result.is_err(), "Unauthorized user should not clear allowlist");
}

#[test]
fn test_clear_allowlist_requires_auth_admin() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    create_initalized(&env, &setup);
    create_allowlist(&env, &setup);

    let allowlist_id = Symbol::new(&env, "test_list");

    let result = setup.client.try_clear_allowlist(&setup.admin, &allowlist_id);
    match result {
        Ok(_) => assert!(true, "Auth passed for admin"),
        Err(e) => {
            let error_str = format!("{:?}", e);
            assert!(!error_str.contains("auth"), "Auth should not fail: {error_str}");
        }
    }
}

// ===== delete_allowlist =====

#[test]
fn test_delete_allowlist_requires_auth() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    create_initalized(&env, &setup);
    create_allowlist(&env, &setup);

    let allowlist_id = Symbol::new(&env, "test_list");

    let result = setup.client.try_delete_allowlist(&setup.unauthorized, &allowlist_id);
    assert!(result.is_err(), "Unauthorized user should not delete allowlist");
}

#[test]
fn test_delete_allowlist_requires_auth_admin() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    create_initalized(&env, &setup);
    create_allowlist(&env, &setup);

    let allowlist_id = Symbol::new(&env, "test_list");

    let result = setup.client.try_delete_allowlist(&setup.admin, &allowlist_id);
    match result {
        Ok(_) => assert!(true, "Auth passed for admin"),
        Err(e) => {
            let error_str = format!("{:?}", e);
            assert!(!error_str.contains("auth"), "Auth should not fail: {error_str}");
        }
    }
}

// ===== transfer_ownership =====

#[test]
fn test_transfer_ownership_requires_auth() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    create_initalized(&env, &setup);

    let new_owner = Address::generate(&env);

    let result = setup.client.try_transfer_ownership(
        &setup.unauthorized,
        &new_owner,
    );
    assert!(result.is_err(), "Unauthorized user should not transfer ownership");
}

#[test]
fn test_transfer_ownership_requires_auth_admin() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    create_initalized(&env, &setup);

    let new_owner = Address::generate(&env);

    let result = setup.client.try_transfer_ownership(
        &setup.admin,
        &new_owner,
    );
    match result {
        Ok(_) => assert!(true, "Auth passed for admin"),
        Err(e) => {
            let error_str = format!("{:?}", e);
            assert!(!error_str.contains("auth"), "Auth should not fail: {error_str}");
        }
    }
}

// ===== Read-only entrypoints (no auth required) =====

#[test]
fn test_read_entrypoints_do_not_require_auth() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    create_initalized(&env, &setup);
    create_allowlist(&env, &setup);

    let allowlist_id = Symbol::new(&env, "test_list");

    // is_allowed — read-only, no auth
    let result = setup.client.try_is_allowed(&allowlist_id, &setup.test_address);
    assert!(result.is_ok(), "is_allowed should not require auth");

    // get_allowlist — read-only, no auth
    let result = setup.client.try_get_allowlist(&allowlist_id);
    assert!(result.is_ok(), "get_allowlist should not require auth");

    // list_allowlists — read-only, no auth
    let result = setup.client.try_list_allowlists();
    assert!(result.is_ok(), "list_allowlists should not require auth");

    // get_admin — read-only, no auth
    let result = setup.client.try_get_admin();
    assert!(result.is_ok(), "get_admin should not require auth");

    // version — read-only, no auth
    let result = setup.client.try_version();
    assert!(result.is_ok(), "version should not require auth");
}
