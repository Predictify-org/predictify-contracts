#![cfg(test)]

use crate::audit_trail::{AuditAction, AuditTrailManager};
use crate::PredictifyHybrid;
use soroban_sdk::{testutils::Address as _, Address, Env, Map, String, Symbol};

fn create_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env
}

#[test]
fn test_market_audit_is_persisted_and_queryable() {
    let env = create_env();
    let contract_id = env.register(PredictifyHybrid {}, ());
    let actor = Address::generate(&env);
    let market_id = Symbol::new(&env, "market_a");

    env.as_contract(&contract_id, || {
        let mut details = Map::new(&env);
        details.set(Symbol::new(&env, "status"), String::from_str(&env, "created"));

        let index = AuditTrailManager::append_market_record(
            &env,
            &market_id,
            AuditAction::MarketCreated,
            actor.clone(),
            details.clone(),
            None,
        );
        assert_eq!(index, 1);

        let record = AuditTrailManager::get_market_record(&env, &market_id, 1).unwrap();
        assert_eq!(record.index, 1);
        assert_eq!(record.action, AuditAction::MarketCreated);
        assert_eq!(record.actor, actor);
        assert_eq!(record.details, details);

        let latest = AuditTrailManager::get_market_latest_records(&env, &market_id, 5);
        assert_eq!(latest.len(), 1);
        assert_eq!(latest.get(0).unwrap().index, 1);

        let head = AuditTrailManager::get_market_head(&env, &market_id).unwrap();
        assert_eq!(head.latest_index, 1);
    });
}

#[test]
fn test_market_audit_is_scoped_per_market() {
    let env = create_env();
    let contract_id = env.register(PredictifyHybrid {}, ());
    let actor = Address::generate(&env);
    let market_a = Symbol::new(&env, "market_a");
    let market_b = Symbol::new(&env, "market_b");

    env.as_contract(&contract_id, || {
        AuditTrailManager::append_market_record(
            &env,
            &market_a,
            AuditAction::MarketCreated,
            actor.clone(),
            Map::new(&env),
            None,
        );
        AuditTrailManager::append_market_record(
            &env,
            &market_b,
            AuditAction::MarketResolved,
            actor.clone(),
            Map::new(&env),
            None,
        );

        let market_a_records = AuditTrailManager::get_market_latest_records(&env, &market_a, 5);
        let market_b_records = AuditTrailManager::get_market_latest_records(&env, &market_b, 5);

        assert_eq!(market_a_records.len(), 1);
        assert_eq!(market_b_records.len(), 1);
        assert_eq!(market_a_records.get(0).unwrap().action, AuditAction::MarketCreated);
        assert_eq!(market_b_records.get(0).unwrap().action, AuditAction::MarketResolved);
    });
}
