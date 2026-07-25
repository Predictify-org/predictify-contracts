#![cfg(test)]

use soroban_sdk::{contract, contractimpl, Env, Symbol, Address};
use crate::oracles::ReflectorOracleClient;
use crate::types::ReflectorAsset;

#[contract]
pub struct MockReflectorOracle;

#[contractimpl]
impl MockReflectorOracle {
    pub fn twap(env: Env, _asset: ReflectorAsset, _records: u32) -> Option<i128> {
        let count: u32 = env.storage().instance().get(&Symbol::short(&env, "calls")).unwrap_or(0);
        env.storage().instance().set(&Symbol::short(&env, "calls"), &(count + 1));
        Some(100_000_000) // Mock price
    }
    
    pub fn get_calls(env: Env) -> u32 {
        env.storage().instance().get(&Symbol::short(&env, "calls")).unwrap_or(0)
    }
}

#[test]
fn test_reflector_twap_cache_within_transaction() {
    let env = Env::default();
    
    // Register the mock oracle contract
    let mock_id = env.register_contract(None, MockReflectorOracle);
    let mock_client = MockReflectorOracleClient::new(&env, &mock_id);
    
    // Create the client
    let client = ReflectorOracleClient::new(&env, mock_id.clone());
    let asset = ReflectorAsset::Other(Symbol::new(&env, "BTC"));
    let records = 10;
    
    // First call, should hit the mock contract
    let res1 = client.twap(asset.clone(), records, false);
    assert_eq!(res1, Some(100_000_000));
    assert_eq!(mock_client.get_calls(), 1);
    
    // Second call with force_refresh = false, should hit the cache
    let res2 = client.twap(asset.clone(), records, false);
    assert_eq!(res2, Some(100_000_000));
    assert_eq!(mock_client.get_calls(), 1);
    
    // Third call with force_refresh = true, should hit the mock contract
    let res3 = client.twap(asset.clone(), records, true);
    assert_eq!(res3, Some(100_000_000));
    assert_eq!(mock_client.get_calls(), 2);
    
    // Fourth call with force_refresh = false, should hit the cache again
    let res4 = client.twap(asset, records, false);
    assert_eq!(res4, Some(100_000_000));
    assert_eq!(mock_client.get_calls(), 2);
}

#[test]
fn test_twap_cache_resets_between_transactions() {
    // Transaction 1
    let env1 = Env::default();
    let mock_id1 = env1.register_contract(None, MockReflectorOracle);
    let mock_client1 = MockReflectorOracleClient::new(&env1, &mock_id1);
    let client1 = ReflectorOracleClient::new(&env1, mock_id1.clone());
    let asset = ReflectorAsset::Other(Symbol::new(&env1, "BTC"));
    
    let val1 = client1.twap(asset.clone(), 5, false);
    assert_eq!(val1, Some(100_000_000));
    assert_eq!(mock_client1.get_calls(), 1);
    
    // Transaction 2 — fresh Env, cache must not persist
    let env2 = Env::default();
    let mock_id2 = env2.register_contract(None, MockReflectorOracle);
    let mock_client2 = MockReflectorOracleClient::new(&env2, &mock_id2);
    let client2 = ReflectorOracleClient::new(&env2, mock_id2.clone());
    
    let val2 = client2.twap(asset, 5, false);
    assert_eq!(val2, Some(100_000_000));
    // In a new transaction the cache is empty, so the mock must be called again.
    assert_eq!(mock_client2.get_calls(), 1);
}
