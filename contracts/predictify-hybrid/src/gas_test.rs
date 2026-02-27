#![cfg(test)]

use soroban_sdk::{testutils::{Events, Address as _, Ledger}, vec, Env, String, Symbol, symbol_short, Val, TryIntoVal, Address, token::StellarAssetClient};
use crate::gas::GasTracker;
use crate::PredictifyHybrid;

#[test]
fn test_gas_limit_storage() {
    let env = Env::default();
    let contract_id = env.register(PredictifyHybrid, ());
    let operation = symbol_short!("test_op");
    
    env.as_contract(&contract_id, || {
        // Default should be None
        assert_eq!(GasTracker::get_limit(&env, operation.clone()), None);
        
        // Set limit
        GasTracker::set_limit(&env, operation.clone(), 5000);
        assert_eq!(GasTracker::get_limit(&env, operation), Some(5000));
    });
}

#[test]
fn test_gas_tracking_observability() {
    let env = Env::default();
    let contract_id = env.register(PredictifyHybrid, ());
    let operation = symbol_short!("test_op");
    
    env.as_contract(&contract_id, || {
        let marker = GasTracker::start_tracking(&env);
        GasTracker::end_tracking(&env, operation.clone(), marker);
    });
    
    // Verify event emission
    let events = env.events().all();
    let last_event = events.last().expect("Event should have been published");
    
    // Event structure: (ContractAddress, Topics, Data)
    let topics = &last_event.1;
    let topic_0: Symbol = topics.get(0).unwrap().try_into_val(&env).unwrap();
    let topic_1: Symbol = topics.get(1).unwrap().try_into_val(&env).unwrap();
    
    assert_eq!(topic_0, symbol_short!("gas_used"));
    assert_eq!(topic_1, operation);
}

#[test]
#[should_panic(expected = "Gas budget cap exceeded")]
fn test_gas_limit_enforcement() {
    let env = Env::default();
    let contract_id = env.register(PredictifyHybrid, ());
    let operation = symbol_short!("test_op");
    
    env.as_contract(&contract_id, || {
        // Set limit to 500
        GasTracker::set_limit(&env, operation.clone(), 500);
        
        // Mock the cost to 1000 (exceeds limit)
        env.storage().temporary().set(&symbol_short!("t_gas"), &1000u64);
        
        let marker = GasTracker::start_tracking(&env);
        GasTracker::end_tracking(&env, operation, marker);
    });
}

#[test]
fn test_gas_limit_not_exceeded() {
    let env = Env::default();
    let contract_id = env.register(PredictifyHybrid, ());
    let operation = symbol_short!("test_op");
    
    env.as_contract(&contract_id, || {
        // Set limit to 1500
        GasTracker::set_limit(&env, operation.clone(), 1500);
        
        // Mock the cost to 1000 (within limit)
        env.storage().temporary().set(&symbol_short!("t_gas"), &1000u64);
        
        let marker = GasTracker::start_tracking(&env);
        GasTracker::end_tracking(&env, operation, marker);
    });
}
#[test]
fn test_integration_with_vote() {
    let env = Env::default();
    env.mock_all_auths(); // Fix auth issues in tests
    let contract_id = env.register(PredictifyHybrid, ());
    let client = crate::PredictifyHybridClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    
    // Initialize
    client.initialize(&admin, &None);
    
    // Create a market
    let question = String::from_str(&env, "Test Question?");
    let outcomes = vec![&env, String::from_str(&env, "Yes"), String::from_str(&env, "No")];
    let oracle_config = crate::OracleConfig::none_sentinel(&env);
    
    let market_id = client.create_market(
        &admin,
        &question,
        &outcomes,
        &30,
        &oracle_config,
        &None,
        &86400,
        &None,
        &None,
        &None,
    );
    
    // Setup token for staking
    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_id = token_contract.address();

    // Set token for staking in contract storage
    env.as_contract(&contract_id, || {
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, "TokenID"), &token_id);
    });

    // Fund user with tokens and approve contract
    let stellar_client = StellarAssetClient::new(&env, &token_id);
    stellar_client.mint(&user, &1000_0000000); // 1,000 XLM
    
    let token_client = soroban_sdk::token::Client::new(&env, &token_id);
    token_client.approve(&user, &contract_id, &i128::MAX, &1000000);

    // Clear previous events
    let _ = env.events().all();
    
    // Vote
    client.vote(&user, &market_id, &String::from_str(&env, "Yes"), &1000000);
    
    // Verify gas_used event for "vote"
    let events = env.events().all();
    let gas_event = events.iter().find(|e| {
        let topics = &e.1;
        let topic_0: Result<Symbol, _> = topics.get(0).unwrap().try_into_val(&env);
        topic_0.is_ok() && topic_0.unwrap() == symbol_short!("gas_used")
    }).expect("Gas used event should be emitted");
    
    let topics = &gas_event.1;
    let operation: Symbol = topics.get(1).unwrap().try_into_val(&env).unwrap();
    assert_eq!(operation, symbol_short!("vote"));
}

#[test]
fn test_integration_with_resolve_manual() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PredictifyHybrid, ());
    let client = crate::PredictifyHybridClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    
    // Initialize
    client.initialize(&admin, &None);
    
    // Create a market
    let question = String::from_str(&env, "Test Question?");
    let outcomes = vec![&env, String::from_str(&env, "Yes"), String::from_str(&env, "No")];
    let oracle_config = crate::OracleConfig::none_sentinel(&env);
    
    let market_id = client.create_market(
        &admin,
        &question,
        &outcomes,
        &30,
        &oracle_config,
        &None,
        &86400,
        &None,
        &None,
        &None,
    );
    
    // Setup token for staking
    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_id = token_contract.address();

    // Set token for staking in contract storage
    env.as_contract(&contract_id, || {
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, "TokenID"), &token_id);
    });

    // Fast forward to end of market
    env.ledger().set_timestamp(env.ledger().timestamp() + (30 * 24 * 60 * 60) + 1);
    
    // Clear previous events
    let _ = env.events().all();
    
    // Resolve manually
    client.resolve_market_manual(&admin, &market_id, &String::from_str(&env, "Yes"));
    
    // Verify gas_used event for "res_man"
    let events = env.events().all();
    let gas_event = events.iter().find(|e| {
        let topics = &e.1;
        let topic_0: Result<Symbol, _> = topics.get(0).unwrap().try_into_val(&env);
        topic_0.is_ok() && topic_0.unwrap() == symbol_short!("gas_used")
    }).expect("Gas used event should be emitted");
    
    let topics = &gas_event.1;
    let operation: Symbol = topics.get(1).unwrap().try_into_val(&env).unwrap();
    assert_eq!(operation, symbol_short!("res_man"));
}
