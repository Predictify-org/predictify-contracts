#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    Address, Env, Symbol, Vec,
};

use allowlist::{AllowlistContract, AllowlistContractClient, AllowlistError};

fn setup_test() -> (Env, TestSetup) {
    let env = Env::default();
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

    let admin = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let user3 = Address::generate(&env);

    let contract_id = env.register_contract(None, AllowlistContract);
    let client = AllowlistContractClient::new(&env, &contract_id);

    let setup = TestSetup {
        admin,
        user1,
        user2,
        user3,
        client,
    };

    // Initialize
    client.initialize(&setup.admin);

    (env, setup)
}

struct TestSetup {
    admin: Address,
    user1: Address,
    user2: Address,
    user3: Address,
    client: AllowlistContractClient,
}

// ===== Initialization tests =====

#[test]
fn test_initialize_sets_admin() {
    let (env, setup) = setup_test();

    let stored_admin = setup.client.get_admin();
    assert_eq!(stored_admin, setup.admin);
}

#[test]
fn test_initialize_rejects_double_init() {
    let env = Env::default();
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

    let admin = Address::generate(&env);
    let contract_id = env.register_contract(None, AllowlistContract);
    let client = AllowlistContractClient::new(&env, &contract_id);

    // First init succeeds
    assert!(client.try_initialize(&admin).is_ok());

    // Second init fails
    let result = client.try_initialize(&admin);
    assert_eq!(result, Ok(Err(AllowlistError::AlreadyInitialized)));
}

// ===== Allowlist creation tests =====

#[test]
fn test_create_allowlist_and_check_exists() {
    let (env, setup) = setup_test();

    let list_id = Symbol::new(&env, "my_list");
    setup.client.create_allowlist(&setup.admin, &list_id);

    let lists = setup.client.list_allowlists();
    assert_eq!(lists.len(), 1);
    assert_eq!(lists.get(0).unwrap(), list_id);
}

#[test]
fn test_create_allowlist_rejects_duplicate() {
    let (env, setup) = setup_test();

    let list_id = Symbol::new(&env, "my_list");
    setup.client.create_allowlist(&setup.admin, &list_id);

    let result = setup.client.try_create_allowlist(&setup.admin, &list_id);
    assert_eq!(result, Ok(Err(AllowlistError::AllowlistAlreadyExists)));
}

#[test]
fn test_create_allowlist_fails_if_not_initialized() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register_contract(None, AllowlistContract);
    let client = AllowlistContractClient::new(&env, &contract_id);

    let list_id = Symbol::new(&env, "my_list");
    let result = client.try_create_allowlist(&admin, &list_id);
    assert_eq!(result, Ok(Err(AllowlistError::NotInitialized)));
}

#[test]
fn test_create_multiple_allowlists() {
    let (env, setup) = setup_test();

    let id1 = Symbol::new(&env, "list_a");
    let id2 = Symbol::new(&env, "list_b");
    let id3 = Symbol::new(&env, "list_c");

    setup.client.create_allowlist(&setup.admin, &id1);
    setup.client.create_allowlist(&setup.admin, &id2);
    setup.client.create_allowlist(&setup.admin, &id3);

    let lists = setup.client.list_allowlists();
    assert_eq!(lists.len(), 3);
}

// ===== Add/remove address tests =====

#[test]
fn test_add_address_and_verify() {
    let (env, setup) = setup_test();
    let list_id = Symbol::new(&env, "players");
    setup.client.create_allowlist(&setup.admin, &list_id);

    setup.client.add_address(&setup.admin, &list_id, &setup.user1);

    let allowed = setup.client.is_allowed(&list_id, &setup.user1);
    assert!(allowed);

    let allowed2 = setup.client.is_allowed(&list_id, &setup.user2);
    assert!(!allowed2);
}

#[test]
fn test_add_address_rejects_duplicate() {
    let (env, setup) = setup_test();
    let list_id = Symbol::new(&env, "players");
    setup.client.create_allowlist(&setup.admin, &list_id);

    setup.client.add_address(&setup.admin, &list_id, &setup.user1);

    let result = setup.client.try_add_address(&setup.admin, &list_id, &setup.user1);
    assert_eq!(result, Ok(Err(AllowlistError::AddressAlreadyInAllowlist)));
}

#[test]
fn test_remove_address() {
    let (env, setup) = setup_test();
    let list_id = Symbol::new(&env, "players");
    setup.client.create_allowlist(&setup.admin, &list_id);

    setup.client.add_address(&setup.admin, &list_id, &setup.user1);
    setup.client.add_address(&setup.admin, &list_id, &setup.user2);

    // Remove user1
    setup.client.remove_address(&setup.admin, &list_id, &setup.user1);

    assert!(!setup.client.is_allowed(&list_id, &setup.user1));
    assert!(setup.client.is_allowed(&list_id, &setup.user2));
}

#[test]
fn test_remove_address_rejects_not_found() {
    let (env, setup) = setup_test();
    let list_id = Symbol::new(&env, "players");
    setup.client.create_allowlist(&setup.admin, &list_id);

    let result = setup.client.try_remove_address(&setup.admin, &list_id, &setup.user1);
    assert_eq!(result, Ok(Err(AllowlistError::AddressNotInAllowlist)));
}

#[test]
fn test_add_address_rejects_nonexistent_allowlist() {
    let (_env, setup) = setup_test();
    let ghost_id = Symbol::new(&_env, "ghost");

    let result = setup.client.try_add_address(&setup.admin, &ghost_id, &setup.user1);
    assert_eq!(result, Ok(Err(AllowlistError::AllowlistNotFound)));
}

// ===== Batch operation tests =====

#[test]
fn test_batch_add_addresses() {
    let (env, setup) = setup_test();
    let list_id = Symbol::new(&env, "team");
    setup.client.create_allowlist(&setup.admin, &list_id);

    let addrs = soroban_sdk::vec![&env, setup.user1.clone(), setup.user2.clone(), setup.user3.clone()];
    setup.client.add_addresses(&setup.admin, &list_id, &addrs);

    assert!(setup.client.is_allowed(&list_id, &setup.user1));
    assert!(setup.client.is_allowed(&list_id, &setup.user2));
    assert!(setup.client.is_allowed(&list_id, &setup.user3));
}

#[test]
fn test_batch_add_is_idempotent() {
    let (env, setup) = setup_test();
    let list_id = Symbol::new(&env, "team");
    setup.client.create_allowlist(&setup.admin, &list_id);

    setup.client.add_address(&setup.admin, &list_id, &setup.user1);

    // Adding user1 again via batch should not error (silently skipped)
    let addrs = soroban_sdk::vec![&env, setup.user1.clone(), setup.user2.clone()];
    let result = setup.client.try_add_addresses(&setup.admin, &list_id, &addrs);
    assert!(result.is_ok());

    let all = setup.client.get_allowlist(&list_id);
    assert_eq!(all.len(), 2);
}

#[test]
fn test_batch_remove_addresses() {
    let (env, setup) = setup_test();
    let list_id = Symbol::new(&env, "team");
    setup.client.create_allowlist(&setup.admin, &list_id);

    let addrs = soroban_sdk::vec![&env, setup.user1.clone(), setup.user2.clone(), setup.user3.clone()];
    setup.client.add_addresses(&setup.admin, &list_id, &addrs);

    // Remove user1 and user3
    let to_remove = soroban_sdk::vec![&env, setup.user1.clone(), setup.user3.clone()];
    setup.client.remove_addresses(&setup.admin, &list_id, &to_remove);

    assert!(!setup.client.is_allowed(&list_id, &setup.user1));
    assert!(setup.client.is_allowed(&list_id, &setup.user2));
    assert!(!setup.client.is_allowed(&list_id, &setup.user3));
}

#[test]
fn test_batch_remove_is_idempotent() {
    let (env, setup) = setup_test();
    let list_id = Symbol::new(&env, "team");
    setup.client.create_allowlist(&setup.admin, &list_id);

    let addrs = soroban_sdk::vec![&env, setup.user1.clone(), setup.user2.clone()];
    setup.client.add_addresses(&setup.admin, &list_id, &addrs);

    // Removing user3 (who is not in the list) should not error (silently skipped)
    let to_remove = soroban_sdk::vec![&env, setup.user1.clone(), setup.user3.clone()];
    let result = setup.client.try_remove_addresses(&setup.admin, &list_id, &to_remove);
    assert!(result.is_ok());

    assert!(!setup.client.is_allowed(&list_id, &setup.user1));
    assert!(setup.client.is_allowed(&list_id, &setup.user2));
}

// ===== Clear and delete tests =====

#[test]
fn test_clear_allowlist() {
    let (env, setup) = setup_test();
    let list_id = Symbol::new(&env, "players");
    setup.client.create_allowlist(&setup.admin, &list_id);

    setup.client.add_address(&setup.admin, &list_id, &setup.user1);
    setup.client.add_address(&setup.admin, &list_id, &setup.user2);
    setup.client.add_address(&setup.admin, &list_id, &setup.user3);

    setup.client.clear_allowlist(&setup.admin, &list_id);

    // All addresses should be removed
    assert!(!setup.client.is_allowed(&list_id, &setup.user1));
    assert!(!setup.client.is_allowed(&list_id, &setup.user2));
    assert!(!setup.client.is_allowed(&list_id, &setup.user3));

    // Allowlist still exists (empty)
    let all = setup.client.get_allowlist(&list_id);
    assert_eq!(all.len(), 0);
}

#[test]
fn test_clear_empty_allowlist() {
    let (env, setup) = setup_test();
    let list_id = Symbol::new(&env, "empty");
    setup.client.create_allowlist(&setup.admin, &list_id);

    // Clearing an empty list should succeed
    let result = setup.client.try_clear_allowlist(&setup.admin, &list_id);
    assert!(result.is_ok());
}

#[test]
fn test_delete_allowlist() {
    let (env, setup) = setup_test();
    let list_id = Symbol::new(&env, "players");
    setup.client.create_allowlist(&setup.admin, &list_id);

    setup.client.add_address(&setup.admin, &list_id, &setup.user1);
    setup.client.add_address(&setup.admin, &list_id, &setup.user2);

    setup.client.delete_allowlist(&setup.admin, &list_id);

    // Allowlist should no longer exist
    let result = setup.client.try_is_allowed(&list_id, &setup.user1);
    assert_eq!(result, Ok(Err(AllowlistError::AllowlistNotFound)));

    // Registry should no longer contain the list
    let lists = setup.client.list_allowlists();
    assert_eq!(lists.len(), 0);
}

#[test]
fn test_delete_nonexistent_allowlist() {
    let (_env, setup) = setup_test();
    let ghost_id = Symbol::new(&_env, "ghost");

    let result = setup.client.try_delete_allowlist(&setup.admin, &ghost_id);
    assert_eq!(result, Ok(Err(AllowlistError::AllowlistNotFound)));
}

// ===== Ownership transfer tests =====

#[test]
fn test_transfer_ownership() {
    let (env, setup) = setup_test();
    let new_admin = Address::generate(&env);

    setup.client.transfer_ownership(&setup.admin, &new_admin);

    let stored = setup.client.get_admin();
    assert_eq!(stored, new_admin);
}

#[test]
fn test_transfer_ownership_rejects_same_admin() {
    let (env, setup) = setup_test();

    let result = setup.client.try_transfer_ownership(&setup.admin, &setup.admin);
    assert_eq!(result, Ok(Err(AllowlistError::InvalidInput)));
}

#[test]
fn test_new_admin_can_manage_allowlists() {
    let (env, setup) = setup_test();
    let new_admin = Address::generate(&env);

    setup.client.transfer_ownership(&setup.admin, &new_admin);

    let list_id = Symbol::new(&env, "new_list");
    let result = setup.client.try_create_allowlist(&new_admin, &list_id);
    assert!(result.is_ok());

    // Old admin should no longer be able to manage
    let result2 = setup.client.try_create_allowlist(&setup.admin, &list_id);
    assert!(result2.is_err());
}

// ===== Version and utility tests =====

#[test]
fn test_version() {
    let (_env, setup) = setup_test();
    assert_eq!(setup.client.version(), 1);
}

#[test]
fn test_get_allowlist_contents() {
    let (env, setup) = setup_test();
    let list_id = Symbol::new(&env, "team");
    setup.client.create_allowlist(&setup.admin, &list_id);

    setup.client.add_address(&setup.admin, &list_id, &setup.user1);
    setup.client.add_address(&setup.admin, &list_id, &setup.user2);

    let all = setup.client.get_allowlist(&list_id);
    assert_eq!(all.len(), 2);

    // The order should be preserved (user1 added first, user2 second)
    let first = all.get(0).unwrap();
    let second = all.get(1).unwrap();
    assert_eq!(first, setup.user1);
    assert_eq!(second, setup.user2);
}

#[test]
fn test_get_nonexistent_allowlist() {
    let (_env, setup) = setup_test();
    let ghost_id = Symbol::new(&_env, "ghost");

    let result = setup.client.try_get_allowlist(&ghost_id);
    assert_eq!(result, Ok(Err(AllowlistError::AllowlistNotFound)));
}
