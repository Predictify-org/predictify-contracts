#![cfg(test)]

use soroban_sdk::{
    contract, contractimpl, Address, Env, String,
    testutils::Address as _,
};
use oracles::{OraclesContract, OraclesContractClient, Error, OraclePriceData};

#[contract]
pub struct MockOracle;

#[contractimpl]
impl MockOracle {
    pub fn get_price(_env: Env, _feed_id: String) -> i128 {
        12345
    }
    pub fn get_pdata(_env: Env, _feed_id: String) -> OraclePriceData {
        OraclePriceData {
            price: 12345,
            publish_time: 1000,
            confidence: Some(10),
            exponent: -8,
        }
    }
    pub fn is_live(_env: Env) -> bool {
        true
    }
}

#[contract]
pub struct MockOraclePriceOnly;

#[contractimpl]
impl MockOraclePriceOnly {
    pub fn get_price(_env: Env, _feed_id: String) -> i128 {
        54321
    }
    pub fn is_live(_env: Env) -> bool {
        true
    }
}

#[contract]
pub struct MockOracleUnhealthy;

#[contractimpl]
impl MockOracleUnhealthy {
    pub fn get_price(_env: Env, _feed_id: String) -> i128 {
        0
    }
    pub fn is_live(_env: Env) -> bool {
        false
    }
}

#[contract]
pub struct MockOracleFailing;

#[contractimpl]
impl MockOracleFailing {
    // Empty implementation to cause dynamic invocations to fail/return None
}

#[test]
fn test_oracle_lifecycle_and_queries() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register_contract(None, OraclesContract);
    let client = OraclesContractClient::new(&env, &contract_id);

    // 1. Initial list should be empty
    assert_eq!(client.list_oracles().len(), 0);

    // Register a valid mock oracle
    let oracle_addr = env.register_contract(None, MockOracle);
    client.add_oracle(&admin, &oracle_addr).unwrap();

    // Verify it is registered
    let list = client.list_oracles();
    assert_eq!(list.len(), 1);
    assert_eq!(list.get(0).unwrap(), oracle_addr);

    // Test duplicate registration (should be a no-op)
    client.add_oracle(&admin, &oracle_addr).unwrap();
    assert_eq!(client.list_oracles().len(), 1);

    // 2. Query price
    let feed = String::from_str(&env, "BTC/USD");
    let price = client.get_price(&oracle_addr, &feed).unwrap();
    assert_eq!(price, 12345);

    // 3. Query price data (using get_pdata)
    let pdata = client.get_price_data(&oracle_addr, &feed).unwrap();
    assert_eq!(pdata.price, 12345);
    assert_eq!(pdata.publish_time, 1000);
    assert_eq!(pdata.confidence, Some(10));
    assert_eq!(pdata.exponent, -8);

    // 4. Check health
    assert!(client.is_oracle_healthy(&oracle_addr).unwrap());

    // 5. Register Price-only oracle
    let price_only_addr = env.register_contract(None, MockOraclePriceOnly);
    client.add_oracle(&admin, &price_only_addr).unwrap();

    // Verify list contains both
    let list = client.list_oracles();
    assert_eq!(list.len(), 2);
    assert_eq!(list.get(1).unwrap(), price_only_addr);

    // Query price from price-only oracle
    assert_eq!(client.get_price(&price_only_addr, &feed).unwrap(), 54321);

    // Query price data from price-only oracle (should fallback)
    let fallback_pdata = client.get_price_data(&price_only_addr, &feed).unwrap();
    assert_eq!(fallback_pdata.price, 54321);
    assert_eq!(fallback_pdata.publish_time, env.ledger().timestamp());
    assert_eq!(fallback_pdata.confidence, None);
    assert_eq!(fallback_pdata.exponent, 0);

    // 6. Register Unhealthy oracle
    let unhealthy_addr = env.register_contract(None, MockOracleUnhealthy);
    client.add_oracle(&admin, &unhealthy_addr).unwrap();
    assert!(!client.is_oracle_healthy(&unhealthy_addr).unwrap());

    // 7. Register Failing oracle
    let failing_addr = env.register_contract(None, MockOracleFailing);
    client.add_oracle(&admin, &failing_addr).unwrap();

    // Failing oracle is not healthy
    assert!(!client.is_oracle_healthy(&failing_addr).unwrap());

    // Failing oracle get_price returns error
    let price_res = client.get_price(&failing_addr, &feed);
    assert_eq!(price_res.unwrap_err(), Error::OracleUnavailable);

    // Failing oracle get_price_data returns error
    let pdata_res = client.get_price_data(&failing_addr, &feed);
    assert_eq!(pdata_res.unwrap_err(), Error::OracleUnavailable);

    // 8. Query unregistered oracle
    let unregistered_addr = Address::generate(&env);
    assert_eq!(
        client.get_price(&unregistered_addr, &feed).unwrap_err(),
        Error::InvalidOracleConfig
    );
    assert_eq!(
        client.get_price_data(&unregistered_addr, &feed).unwrap_err(),
        Error::InvalidOracleConfig
    );
    assert_eq!(
        client.is_oracle_healthy(&unregistered_addr).unwrap_err(),
        Error::InvalidOracleConfig
    );

    // 9. Remove oracle
    client.remove_oracle(&admin, &oracle_addr).unwrap();
    let list = client.list_oracles();
    assert_eq!(list.len(), 3);
    assert!(!list.iter().any(|x| x == oracle_addr));

    // Remove nonexistent oracle (should be a no-op)
    client.remove_oracle(&admin, &oracle_addr).unwrap();
    assert_eq!(client.list_oracles().len(), 3);
}

#[test]
#[should_panic]
fn test_add_oracle_requires_auth() {
    let env = Env::default();
    // Do not call mock_all_auths
    let admin = Address::generate(&env);
    let oracle = Address::generate(&env);
    let contract_id = env.register_contract(None, OraclesContract);
    let client = OraclesContractClient::new(&env, &contract_id);
    
    // This should panic due to missing authentication/authorization
    let _ = client.add_oracle(&admin, &oracle);
}

#[test]
#[should_panic]
fn test_remove_oracle_requires_auth() {
    let env = Env::default();
    // Do not call mock_all_auths
    let admin = Address::generate(&env);
    let oracle = Address::generate(&env);
    let contract_id = env.register_contract(None, OraclesContract);
    let client = OraclesContractClient::new(&env, &contract_id);

    // This should panic due to missing authentication/authorization
    let _ = client.remove_oracle(&admin, &oracle);
}
