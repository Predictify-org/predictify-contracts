#![cfg(test)]

use soroban_sdk::{Env, Symbol};
use crate::markets::MarketStateManager;
use crate::err::Error;

#[test]
fn test_get_market_returns_not_found_for_missing_market() {
    let env = Env::default();
    let market_id = Symbol::new(&env, "mkt_missing");
    
    let result = MarketStateManager::get_market(&env, &market_id);
    assert_eq!(result.unwrap_err(), Error::MarketNotFound);
}

#[test]
fn test_get_market_handles_system_keys_safely() {
    let env = Env::default();
    
    // Write an i128 to a system key simulating "platform_fee"
    let system_key = Symbol::new(&env, "platform_fee");
    env.storage().persistent().set(&system_key, &1000i128);
    
    // Attempting to fetch this as a market should NOT panic, but return MarketNotFound
    let result = MarketStateManager::get_market(&env, &system_key);
    assert_eq!(result.unwrap_err(), Error::MarketNotFound);
}
