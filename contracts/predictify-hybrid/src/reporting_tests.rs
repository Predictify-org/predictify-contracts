#![cfg(test)]

use crate::reporting::*;
use crate::types::*;
use crate::PredictifyHybrid;
use soroban_sdk::{vec, Address, Env, String, Symbol};
use soroban_sdk::testutils::Address as _;

#[test]
fn test_get_active_events_pagination() {
    let env = Env::default();
    let admin = Address::generate(&env);
    
    // Initialize contract
    PredictifyHybrid::initialize(env.clone(), admin.clone(), None);
    
    // Create some markets
    let q1 = String::from_str(&env, "Question 1");
    let q2 = String::from_str(&env, "Question 2");
    let q3 = String::from_str(&env, "Question 3");
    let outcomes = vec![&env, String::from_str(&env, "yes"), String::from_str(&env, "no")];
    let oracle_config = OracleConfig::new(
        OracleProvider::Reflector,
        Address::generate(&env),
        String::from_str(&env, "BTC"),
        100,
        String::from_str(&env, "gt"),
    );

    let m1 = PredictifyHybrid::create_market(
        env.clone(),
        admin.clone(),
        q1.clone(),
        outcomes.clone(),
        30,
        oracle_config.clone(),
        None,
        3600,
    );
    let m2 = PredictifyHybrid::create_market(
        env.clone(),
        admin.clone(),
        q2.clone(),
        outcomes.clone(),
        30,
        oracle_config.clone(),
        None,
        3600,
    );
    let m3 = PredictifyHybrid::create_market(
        env.clone(),
        admin.clone(),
        q3.clone(),
        outcomes.clone(),
        30,
        oracle_config.clone(),
        None,
        3600,
    );

    // Test pagination
    let active_all = ReportingManager::get_active_events(&env, 0, 10).unwrap();
    assert_eq!(active_all.len(), 3);

    let active_paged = ReportingManager::get_active_events(&env, 1, 1).unwrap();
    assert_eq!(active_paged.len(), 1);
    // Check if it's one of the markets (order depends on implementation of market index)
    let id = active_paged.get(0).unwrap().id;
    assert!(id == m1 || id == m2 || id == m3);

    let active_empty = ReportingManager::get_active_events(&env, 10, 10).unwrap();
    assert_eq!(active_empty.len(), 0);
}

#[test]
fn test_get_platform_stats() {
    let env = Env::default();
    let admin = Address::generate(&env);
    
    // Initialize contract
    PredictifyHybrid::initialize(env.clone(), admin.clone(), None);
    
    // Create a market
    let outcomes = vec![&env, String::from_str(&env, "yes"), String::from_str(&env, "no")];
    let oracle_config = OracleConfig::new(
        OracleProvider::Reflector,
        Address::generate(&env),
        String::from_str(&env, "BTC"),
        100,
        String::from_str(&env, "gt"),
    );

    PredictifyHybrid::create_market(
        env.clone(),
        admin.clone(),
        String::from_str(&env, "Test"),
        outcomes,
        30,
        oracle_config,
        None,
        3600,
    );

    let stats = ReportingManager::get_platform_stats(&env).unwrap();
    assert_eq!(stats.total_active_events, 1);
    assert_eq!(stats.total_resolved_events, 0);
    assert_eq!(stats.total_pool_all_events, 0);
    assert_eq!(stats.version, String::from_str(&env, "1.0.0"));
}

#[test]
fn test_get_event_snapshot() {
    let env = Env::default();
    let admin = Address::generate(&env);
    
    // Initialize contract
    PredictifyHybrid::initialize(env.clone(), admin.clone(), None);
    
    let question = String::from_str(&env, "Snapshot Question");
    let outcomes = vec![&env, String::from_str(&env, "A"), String::from_str(&env, "B")];
    let oracle_config = OracleConfig::new(
        OracleProvider::Reflector,
        Address::generate(&env),
        String::from_str(&env, "BTC"),
        100,
        String::from_str(&env, "gt"),
    );

    let market_id = PredictifyHybrid::create_market(
        env.clone(),
        admin.clone(),
        question.clone(),
        outcomes.clone(),
        30,
        oracle_config,
        None,
        3600,
    );

    let snapshot = ReportingManager::get_event_snapshot(&env, market_id.clone()).unwrap();
    assert_eq!(snapshot.id, market_id);
    assert_eq!(snapshot.question, question);
    assert_eq!(snapshot.outcomes, outcomes);
    assert_eq!(snapshot.state, MarketState::Active);
    assert_eq!(snapshot.total_pool, 0);
    assert_eq!(snapshot.participant_count, 0);
}

#[test]
fn test_get_event_snapshot_not_found() {
    let env = Env::default();
    let id = Symbol::new(&env, "NON_EXISTENT");
    let result = ReportingManager::get_event_snapshot(&env, id);
    assert!(result.is_err());
}
