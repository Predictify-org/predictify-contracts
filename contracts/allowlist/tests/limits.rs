#![cfg(test)]

//! Focused tests for per-account allowlist-membership limits.

use allowlist::{
    AllowlistContract, AllowlistContractClient, AllowlistError,
    DEFAULT_MAX_MEMBERSHIPS_PER_ACCOUNT, MAX_CONFIGURABLE_ACCOUNT_LIMIT,
};
use soroban_sdk::{
    testutils::{Address as _, MockAuth, MockAuthInvoke},
    Address, Env, IntoVal, Symbol,
};

struct TestSetup<'a> {
    env: Env,
    admin: Address,
    first: Address,
    second: Address,
    client: AllowlistContractClient<'a>,
}

fn setup() -> TestSetup<'static> {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let first = Address::generate(&env);
    let second = Address::generate(&env);
    let contract_id = env.register(AllowlistContract, ());
    let client = AllowlistContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    TestSetup {
        env,
        admin,
        first,
        second,
        client,
    }
}

fn create_list(setup: &TestSetup<'_>, name: &str) -> Symbol {
    let id = Symbol::new(&setup.env, name);
    setup.client.create_allowlist(&setup.admin, &id);
    id
}

#[test]
fn initialization_sets_documented_default_limit() {
    let setup = setup();

    assert_eq!(
        setup.client.get_account_limit(),
        DEFAULT_MAX_MEMBERSHIPS_PER_ACCOUNT
    );
    assert_eq!(setup.client.get_account_usage(&setup.first), 0);
}

#[test]
fn admin_can_set_zero_and_hard_boundary_but_not_exceed_it() {
    let setup = setup();

    setup.client.set_account_limit(&setup.admin, &0);
    assert_eq!(setup.client.get_account_limit(), 0);

    setup
        .client
        .set_account_limit(&setup.admin, &MAX_CONFIGURABLE_ACCOUNT_LIMIT);
    assert_eq!(
        setup.client.get_account_limit(),
        MAX_CONFIGURABLE_ACCOUNT_LIMIT
    );

    assert_eq!(
        setup
            .client
            .try_set_account_limit(&setup.admin, &(MAX_CONFIGURABLE_ACCOUNT_LIMIT + 1),),
        Err(Ok(AllowlistError::InvalidInput))
    );
    assert_eq!(
        setup.client.get_account_limit(),
        MAX_CONFIGURABLE_ACCOUNT_LIMIT
    );
}

#[test]
fn only_admin_can_change_account_limit() {
    let setup = setup();

    assert_eq!(
        setup.client.try_set_account_limit(&setup.first, &1),
        Err(Ok(AllowlistError::Unauthorized))
    );
    assert_eq!(
        setup.client.get_account_limit(),
        DEFAULT_MAX_MEMBERSHIPS_PER_ACCOUNT
    );
}

#[test]
fn setting_limit_captures_admin_as_required_signer() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register(AllowlistContract, ());
    let client = AllowlistContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    client
        .mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "set_account_limit",
                args: (admin.clone(), 1u32).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .set_account_limit(&admin, &1);

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    assert_eq!(auths[0].0, admin);
}

#[test]
fn setting_limit_without_authentication_is_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register(AllowlistContract, ());
    let client = AllowlistContractClient::new(&env, &contract_id);
    client.initialize(&admin);

    env.set_auths(&[]);

    assert!(client.try_set_account_limit(&admin, &1).is_err());
}

#[test]
fn single_add_enforces_limit_without_mutating_rejected_list() {
    let setup = setup();
    let first_list = create_list(&setup, "first");
    let second_list = create_list(&setup, "second");
    setup.client.set_account_limit(&setup.admin, &1);

    setup
        .client
        .add_address(&setup.admin, &first_list, &setup.first);

    assert_eq!(setup.client.get_account_usage(&setup.first), 1);
    assert_eq!(
        setup
            .client
            .try_add_address(&setup.admin, &second_list, &setup.first),
        Err(Ok(AllowlistError::AccountLimitExceeded))
    );
    assert!(!setup.client.is_allowed(&second_list, &setup.first));
    assert_eq!(setup.client.get_account_usage(&setup.first), 1);
}

#[test]
fn limits_are_isolated_per_account() {
    let setup = setup();
    let first_list = create_list(&setup, "first");
    let second_list = create_list(&setup, "second");
    setup.client.set_account_limit(&setup.admin, &1);

    setup
        .client
        .add_address(&setup.admin, &first_list, &setup.first);
    setup
        .client
        .add_address(&setup.admin, &second_list, &setup.second);

    assert_eq!(setup.client.get_account_usage(&setup.first), 1);
    assert_eq!(setup.client.get_account_usage(&setup.second), 1);
}

#[test]
fn duplicate_batch_values_do_not_consume_extra_capacity() {
    let setup = setup();
    let list = create_list(&setup, "batch");
    setup.client.set_account_limit(&setup.admin, &1);
    let addresses = soroban_sdk::vec![&setup.env, setup.first.clone(), setup.first.clone(),];

    setup.client.add_addresses(&setup.admin, &list, &addresses);

    assert_eq!(setup.client.get_allowlist(&list).len(), 1);
    assert_eq!(setup.client.get_account_usage(&setup.first), 1);
}

#[test]
fn failed_batch_add_does_not_partially_mutate_state_or_usage() {
    let setup = setup();
    let occupied = create_list(&setup, "occupied");
    let target = create_list(&setup, "target");
    setup.client.set_account_limit(&setup.admin, &1);
    setup
        .client
        .add_address(&setup.admin, &occupied, &setup.first);
    let addresses = soroban_sdk::vec![&setup.env, setup.second.clone(), setup.first.clone()];

    assert_eq!(
        setup
            .client
            .try_add_addresses(&setup.admin, &target, &addresses),
        Err(Ok(AllowlistError::AccountLimitExceeded))
    );
    assert_eq!(setup.client.get_allowlist(&target).len(), 0);
    assert_eq!(setup.client.get_account_usage(&setup.second), 0);
    assert_eq!(setup.client.get_account_usage(&setup.first), 1);
}

#[test]
fn every_removal_path_releases_capacity() {
    let setup = setup();
    setup.client.set_account_limit(&setup.admin, &1);
    let first = create_list(&setup, "first");
    let second = create_list(&setup, "second");
    let third = create_list(&setup, "third");
    let fourth = create_list(&setup, "fourth");

    setup.client.add_address(&setup.admin, &first, &setup.first);
    setup
        .client
        .remove_address(&setup.admin, &first, &setup.first);
    setup
        .client
        .add_address(&setup.admin, &second, &setup.first);

    let remove = soroban_sdk::vec![&setup.env, setup.first.clone()];
    setup
        .client
        .remove_addresses(&setup.admin, &second, &remove);
    setup.client.add_address(&setup.admin, &third, &setup.first);

    setup.client.clear_allowlist(&setup.admin, &third);
    setup
        .client
        .add_address(&setup.admin, &fourth, &setup.first);

    setup.client.delete_allowlist(&setup.admin, &fourth);
    assert_eq!(setup.client.get_account_usage(&setup.first), 0);
}

#[test]
fn lowering_limit_preserves_memberships_and_blocks_growth() {
    let setup = setup();
    let first = create_list(&setup, "first");
    let second = create_list(&setup, "second");
    let third = create_list(&setup, "third");
    setup.client.set_account_limit(&setup.admin, &2);
    setup.client.add_address(&setup.admin, &first, &setup.first);
    setup
        .client
        .add_address(&setup.admin, &second, &setup.first);

    setup.client.set_account_limit(&setup.admin, &1);

    assert_eq!(setup.client.get_account_usage(&setup.first), 2);
    assert!(setup.client.is_allowed(&first, &setup.first));
    assert!(setup.client.is_allowed(&second, &setup.first));
    assert_eq!(
        setup
            .client
            .try_add_address(&setup.admin, &third, &setup.first),
        Err(Ok(AllowlistError::AccountLimitExceeded))
    );
}
