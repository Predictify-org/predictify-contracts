#![cfg(test)]

use crate::gas::{GasTracker, GasUsage};
use crate::PredictifyHybrid;
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events, Ledger},
    token::StellarAssetClient,
    vec, Address, Env, String, Symbol, TryIntoVal, Val,
};

#[test]
fn test_gas_limit_storage() {
    let env = Env::default();
    let contract_id = env.register(PredictifyHybrid, ());
    let operation = symbol_short!("test_op");

    env.as_contract(&contract_id, || {
        // Default should be None
        let (cpu, mem) = GasTracker::get_limits(&env, operation.clone());
        assert_eq!(cpu, None);
        assert_eq!(mem, None);

        // Set limits
        GasTracker::set_limit(&env, operation.clone(), 5000, 1000);
        let (cpu, mem) = GasTracker::get_limits(&env, operation);
        assert_eq!(cpu, Some(5000));
        assert_eq!(mem, Some(1000));
    });
}

#[test]
fn test_gas_tracking_observability() {
    let env = Env::default();
    let contract_id = env.register(PredictifyHybrid, ());
    let operation = symbol_short!("test_op");

    env.as_contract(&contract_id, || {
        // Set mock cost
        GasTracker::set_test_cost(&env, operation.clone(), 1234, 567);

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

    let cost: GasUsage = last_event.2.try_into_val(&env).unwrap();
    assert_eq!(cost.cpu, 1234);
    assert_eq!(cost.mem, 567);
}

#[test]
#[should_panic(expected = "Gas budget cap exceeded")]
fn test_gas_limit_enforcement_cpu() {
    let env = Env::default();
    let contract_id = env.register(PredictifyHybrid, ());
    let operation = symbol_short!("test_op");

    env.as_contract(&contract_id, || {
        // Set CPU limit to 500
        GasTracker::set_limit(&env, operation.clone(), 500, 2000);

        // Mock the cost to 1000 (exceeds CPU limit)
        GasTracker::set_test_cost(&env, operation.clone(), 1000, 1000);

        let marker = GasTracker::start_tracking(&env);
        GasTracker::end_tracking(&env, operation, marker);
    });
}

#[test]
#[should_panic(expected = "Gas budget cap exceeded")]
fn test_gas_limit_enforcement_mem() {
    let env = Env::default();
    let contract_id = env.register(PredictifyHybrid, ());
    let operation = symbol_short!("test_op");

    env.as_contract(&contract_id, || {
        // Set Mem limit to 500
        GasTracker::set_limit(&env, operation.clone(), 2000, 500);

        // Mock the cost to 1000 (exceeds Mem limit)
        GasTracker::set_test_cost(&env, operation.clone(), 1000, 1000);

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
        // Set limits
        GasTracker::set_limit(&env, operation.clone(), 1500, 1500);

        // Mock the cost to 1000 (within limits)
        GasTracker::set_test_cost(&env, operation.clone(), 1000, 1000);

        let marker = GasTracker::start_tracking(&env);
        GasTracker::end_tracking(&env, operation, marker);
    });
}
