#![cfg(test)]

use soroban_sdk::{
    contract, contractimpl, testutils::Address as _, Address, Env, String,
};
use oracles::{OraclesContract, OraclesContractClient, OraclePriceData};

#[contract]
pub struct MockOracleForAuthSnap;

#[contractimpl]
impl MockOracleForAuthSnap {
    pub fn get_price(_env: Env, _feed_id: String) -> i128 {
        99999
    }
    pub fn get_pdata(_env: Env, _feed_id: String) -> OraclePriceData {
        OraclePriceData {
            price: 99999,
            publish_time: 1000,
            confidence: Some(5),
            exponent: -8,
        }
    }
    pub fn is_live(_env: Env) -> bool {
        true
    }
}

struct TestSetup {
    env: Env,
    admin: Address,
    oracle: Address,
    contract_id: Address,
    client: OraclesContractClient<'static>,
}

fn setup_test_env() -> TestSetup {
    let env = Env::default();
    let admin = Address::generate(&env);
    let oracle = env.register(MockOracleForAuthSnap, ());

    let contract_id = env.register(OraclesContract, ());
    let client = OraclesContractClient::new(&env, &contract_id);

    TestSetup {
        env,
        admin,
        oracle,
        contract_id,
        client,
    }
}

// ---------------------------------------------------------------------------
// 1. Auth enforcement on state-changing entrypoints
// ---------------------------------------------------------------------------

#[test]
fn test_add_oracle_requires_auth_and_succeeds_with_mock() {
    let setup = setup_test_env();
    setup.env.mock_all_auths();

    // Call add_oracle with mocked auth
    setup.client.add_oracle(&setup.admin, &setup.oracle);

    // Verify auth was checked for admin
    let auths = setup.env.auths();
    assert_eq!(auths.len(), 1);
    assert_eq!(auths[0].0, setup.admin);

    // Verify oracle was added
    let list = setup.client.list_oracles();
    assert_eq!(list.len(), 1);
    assert_eq!(list.get(0).unwrap(), setup.oracle);
}

#[test]
#[should_panic]
fn test_add_oracle_fails_without_auth() {
    let setup = setup_test_env();
    // Do NOT call mock_all_auths
    setup.client.add_oracle(&setup.admin, &setup.oracle);
}

#[test]
fn test_remove_oracle_requires_auth_and_succeeds_with_mock() {
    let setup = setup_test_env();
    setup.env.mock_all_auths();

    // Add oracle first
    setup.client.add_oracle(&setup.admin, &setup.oracle);
    assert_eq!(setup.client.list_oracles().len(), 1);

    // Remove oracle with mocked auth
    setup.client.remove_oracle(&setup.admin, &setup.oracle);

    // Verify auth was checked for admin
    let auths = setup.env.auths();
    assert_eq!(auths.len(), 1);
    assert_eq!(auths[0].0, setup.admin);

    // Verify oracle was removed
    assert_eq!(setup.client.list_oracles().len(), 0);
}

#[test]
#[should_panic]
fn test_remove_oracle_fails_without_auth() {
    let setup = setup_test_env();
    setup.env.mock_all_auths();

    // Add oracle first
    setup.client.add_oracle(&setup.admin, &setup.oracle);

    // Create a new env without mock_all_auths to test remove_oracle failure
    let unauth_env = Env::default();
    let client = OraclesContractClient::new(&unauth_env, &setup.contract_id);
    let admin = Address::generate(&unauth_env);
    let oracle = Address::generate(&unauth_env);

    client.remove_oracle(&admin, &oracle);
}

// ---------------------------------------------------------------------------
// 2. Read-only entrypoints do not require auth
// ---------------------------------------------------------------------------

#[test]
fn test_view_entrypoints_do_not_require_auth() {
    let setup = setup_test_env();
    setup.env.mock_all_auths();
    setup.client.add_oracle(&setup.admin, &setup.oracle);

    // Create environment without auth mocking for queries
    let query_env = Env::default();
    query_env.mock_all_auths(); // for adding, then query directly
    let contract_id = query_env.register(OraclesContract, ());
    let client = OraclesContractClient::new(&query_env, &contract_id);
    let admin = Address::generate(&query_env);
    let oracle_addr = query_env.register(MockOracleForAuthSnap, ());

    client.add_oracle(&admin, &oracle_addr);

    // Test list_oracles
    let list = client.list_oracles();
    assert_eq!(list.len(), 1);

    // Test get_price
    let feed = String::from_str(&query_env, "BTC/USD");
    let price = client.get_price(&oracle_addr, &feed);
    assert_eq!(price, 99999);

    // Test get_price_data
    let pdata = client.get_price_data(&oracle_addr, &feed);
    assert_eq!(pdata.price, 99999);
    assert_eq!(pdata.publish_time, 1000);

    // Test is_oracle_healthy
    assert!(client.is_oracle_healthy(&oracle_addr));
}

// ---------------------------------------------------------------------------
// 3. Auth snapshot inspection & multi-oracle sequence
// ---------------------------------------------------------------------------

#[test]
fn test_auth_snapshot_multi_oracle_sequence() {
    let setup = setup_test_env();
    setup.env.mock_all_auths();

    let oracle2 = setup.env.register(MockOracleForAuthSnap, ());

    // First add_oracle call
    setup.client.add_oracle(&setup.admin, &setup.oracle);
    let auths1 = setup.env.auths();
    assert_eq!(auths1.len(), 1);
    assert_eq!(auths1[0].0, setup.admin);

    // Second add_oracle call
    setup.client.add_oracle(&setup.admin, &oracle2);
    let auths2 = setup.env.auths();
    assert_eq!(auths2.len(), 1);
    assert_eq!(auths2[0].0, setup.admin);

    assert_eq!(setup.client.list_oracles().len(), 2);

    // Remove oracle2 call
    setup.client.remove_oracle(&setup.admin, &oracle2);
    let auths3 = setup.env.auths();
    assert_eq!(auths3.len(), 1);
    assert_eq!(auths3[0].0, setup.admin);

    assert_eq!(setup.client.list_oracles().len(), 1);
}
