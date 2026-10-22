#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Events},
    vec, symbol_short, Address, Env, String, Symbol,
};
use predictify_hybrid::events::{EventEmitter, MarketCreatedEvent};
use predictify_hybrid::storage::DataKey;
use predictify_hybrid::PredictifyHybrid;

fn contract_env() -> (Env, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PredictifyHybrid, ());
    (env, contract_id)
}

#[test]
fn test_event_replay_nonce_monotonic() {
    let (env, contract_id) = contract_env();

    env.as_contract(&contract_id, || {
        let admin = Address::generate(&env);
        let market_id = symbol_short!("m1");
        let question = String::from_str(&env, "Q1?");
        let outcomes =
            vec![&env, String::from_str(&env, "Yes"), String::from_str(&env, "No")];

        // First emission
        EventEmitter::emit_market_created(&env, &market_id, &question, &outcomes, &admin, 1000);

        let events = env.events().all();
        assert_eq!(events.events().len(), 1);

        let key = DataKey::EventNonce(symbol_short!("mkt_crt"));
        let nonce: u64 = env.storage().persistent().get(&key).unwrap();
        assert_eq!(nonce, 1);

        // Second emission
        EventEmitter::emit_market_created(&env, &market_id, &question, &outcomes, &admin, 1000);

        let events = env.events().all();
        assert_eq!(events.events().len(), 2);

        let nonce2: u64 = env.storage().persistent().get(&key).unwrap();
        assert_eq!(nonce2, 2);
    });
}

#[test]
fn test_topic_isolation() {
    let (env, contract_id) = contract_env();

    env.as_contract(&contract_id, || {
        let admin = Address::generate(&env);
        let market_id = symbol_short!("m1");
        let question = String::from_str(&env, "Q1?");
        let outcomes =
            vec![&env, String::from_str(&env, "Yes"), String::from_str(&env, "No")];

        // Emit event A
        EventEmitter::emit_market_created(&env, &market_id, &question, &outcomes, &admin, 1000);

        let key1 = DataKey::EventNonce(symbol_short!("mkt_crt"));
        let nonce1: u64 = env.storage().persistent().get(&key1).unwrap();
        assert_eq!(nonce1, 1);

        // Emit event B
        EventEmitter::emit_fallback_used(&env, &market_id, &admin, &admin);

        let key2 = DataKey::EventNonce(symbol_short!("fbk_used"));
        let nonce2: u64 = env.storage().persistent().get(&key2).unwrap();
        assert_eq!(nonce2, 1);

        // Event A nonce is isolated and unaffected by Event B
        let nonce1_again: u64 = env.storage().persistent().get(&key1).unwrap();
        assert_eq!(nonce1_again, 1);
    });
}
