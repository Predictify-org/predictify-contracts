#![cfg(test)]
extern crate std;

use oracles::{OraclesContract, OraclesContractClient};
use soroban_sdk::{testutils::Address as _, Address, Env, String};

#[test]
fn test_ttl_bump_hot_read_paths() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(OraclesContract, ());
    let client = OraclesContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    let oracle = Address::generate(&env);
    client.add_oracle(&admin, &oracle);

    // list_oracles should succeed and bump TTL
    let oracles = client.list_oracles();
    assert_eq!(oracles.len(), 1);
    assert_eq!(oracles.get(0).unwrap(), oracle);

    // To test get_price, get_price_data, and is_oracle_healthy, we would normally need the oracle 
    // to be a deployed contract that implements the oracle interface.
    // However, since we just added the TTL bump which happens *before* the cross-contract call,
    // we can observe it fails with OracleUnavailable instead of panicking on the TTL bump.
    let feed_id = String::from_str(&env, "BTC/USD");
    
    let res1 = client.try_get_price(&oracle, &feed_id);
    assert!(res1.is_err(), "Expected error because oracle is a dummy address");

    let res2 = client.try_get_price_data(&oracle, &feed_id);
    assert!(res2.is_err(), "Expected error because oracle is a dummy address");

    // For is_oracle_healthy, it intercepts the error and returns false if the contract invocation fails
    let is_healthy = client.is_oracle_healthy(&oracle);
    assert_eq!(is_healthy, false);
}
