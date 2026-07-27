#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Events},
    vec, Env, String, Symbol, IntoVal,
};

#[test]
fn test_resolution_flow() {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register_contract(None, ResolutionContract);
    let client = ResolutionContractClient::new(&env, &contract_id);
    
    let market_id = String::from_str(&env, "market_123");
    let resolved_by = Address::generate(&env);
    
    client.start_resolution(&market_id, &resolved_by);
    
    let events = env.events().all();
    assert_eq!(
        events,
        vec![
            &env,
            (
                contract_id.clone(),
                (Symbol::new(&env, "resolution"), Symbol::new(&env, "started"), market_id.clone()).into_val(&env),
                resolved_by.into_val(&env)
            )
        ]
    );
    
    // Test dispute
    let disputed_by = Address::generate(&env);
    let reason = String::from_str(&env, "invalid outcome");
    
    client.dispute_resolution(&market_id, &disputed_by, &reason);
    
    let events = env.events().all();
    assert_eq!(
        events.get(1).unwrap(),
        (
            contract_id.clone(),
            (Symbol::new(&env, "resolution"), Symbol::new(&env, "disputed"), market_id.clone()).into_val(&env),
            (disputed_by.clone(), reason.clone()).into_val(&env)
        )
    );
    
    // Test finalize
    let admin = Address::generate(&env);
    let outcome = String::from_str(&env, "yes");
    
    client.finalize_resolution(&market_id, &admin, &outcome);
    
    let events = env.events().all();
    assert_eq!(
        events.get(2).unwrap(),
        (
            contract_id.clone(),
            (Symbol::new(&env, "resolution"), Symbol::new(&env, "finalized"), market_id.clone()).into_val(&env),
            outcome.into_val(&env)
        )
    );
}
