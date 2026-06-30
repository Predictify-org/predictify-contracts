#![cfg(test)]
extern crate std;

use alloc::vec;
use soroban_sdk::{
    testutils::Address as _,
    Address, Env, String, Symbol, vec as svec, IntoVal,
};
use crate::events::{EventEmitter, EventPayload, MarketCreatedEvent};

#[test]
fn test_event_replay_nonce() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let market_id = Symbol::new(&env, "BTC_50K");

    let contract_id = env.register(crate::PredictifyHybrid, ());
    
    env.as_contract(&contract_id, || {
        let question = String::from_str(&env, "Q1");
        let outcomes = svec![&env, String::from_str(&env, "O1"), String::from_str(&env, "O2")];
        
        // Emit first event on topic mkt_crt
        EventEmitter::emit_market_created(&env, &market_id, &question, &outcomes, &admin, 1000);

        // Emit second event on the same topic mkt_crt
        EventEmitter::emit_market_created(&env, &market_id, &question, &outcomes, &admin, 1000);
        
        // Emit event on a different topic res_tmo
        EventEmitter::emit_resolution_timeout(&env, &market_id, 2000);
    });

    let events = env.events().all();
    let mut mkt_crt_nonces = std::vec::Vec::new();
    let mut res_tmo_nonces = std::vec::Vec::new();

    for (_contract, topic, event_val) in events.iter() {
        if let Ok(topic_vec) = soroban_sdk::Vec::<Symbol>::try_from_val(&env, &topic) {
            if topic_vec.len() > 0 {
                let primary_topic = topic_vec.get(0).unwrap();
                if primary_topic == Symbol::new(&env, "mkt_crt") {
                    let payload: EventPayload<MarketCreatedEvent> = event_val.try_into_val(&env).unwrap();
                    mkt_crt_nonces.push(payload.nonce);
                } else if primary_topic == Symbol::new(&env, "res_tmo") {
                    let payload: EventPayload<soroban_sdk::Val> = event_val.try_into_val(&env).unwrap();
                    res_tmo_nonces.push(payload.nonce);
                }
            }
        }
    }

    // Two consecutive emissions on same topic increment monotonically
    assert_eq!(mkt_crt_nonces, std::vec![0, 1], "Nonces for the same topic should increment monotonically");
    
    // Topic isolation: nonce of topic A does not affect topic B
    assert_eq!(res_tmo_nonces, std::vec![0], "Different topics should have isolated nonces");
}
