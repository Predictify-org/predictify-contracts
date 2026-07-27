use crate::errors::Error;
use crate::events::{
    EventEmitter, EventTestingUtils, EventValidator, OracleLifecycleEvent, OracleLifecycleStage,
    OracleLifecycleStatus,
};
use soroban_sdk::testutils::Events;
use soroban_sdk::{symbol_short, Address, Env, String, Symbol, TryFromVal, TryIntoVal};

fn find_published_event<T>(env: &Env, topic: Symbol) -> Option<T>
where
    T: Clone + TryFromVal<Env, soroban_sdk::xdr::ScVal>,
{
    env.events().all().events().iter().rev().find_map(|event| {
        let body = match &event.body {
            soroban_sdk::xdr::ContractEventBody::V0(v0) => v0,
        };
        let first_topic_scval = body.topics.get(0)?;
        let first_topic: Symbol = first_topic_scval.clone().try_into_val(env).ok()?;
        if first_topic != topic {
            return None;
        }
        T::try_from_val(env, &body.data).ok()
    })
}

#[test]
fn lifecycle_event_emits_and_stores_payload() {
    let env = Env::default();
    let market_id = Symbol::new(&env, "btc_50k");
    let oracle_address = Address::generate(&env);
    let reason = String::from_str(&env, "feed_requested");
    let metadata = String::from_str(&env, "feed=BTC/USD");

    EventEmitter::emit_oracle_lifecycle_event(
        &env,
        &market_id,
        &oracle_address,
        &OracleLifecycleStage::Requested,
        &OracleLifecycleStatus::Pending,
        &reason,
        &metadata,
    );

    let stored: OracleLifecycleEvent = env
        .storage()
        .persistent()
        .get(&symbol_short!("ora_lfcy"))
        .expect("lifecycle event should be stored");

    assert_eq!(stored.market_id, market_id);
    assert_eq!(stored.oracle_address, oracle_address);
    assert_eq!(stored.stage, OracleLifecycleStage::Requested);
    assert_eq!(stored.status, OracleLifecycleStatus::Pending);
    assert_eq!(stored.reason, reason);
    assert_eq!(stored.metadata, metadata);

    let published = find_published_event::<OracleLifecycleEvent>(&env, symbol_short!("ora_lfcy"))
        .expect("lifecycle event should be published");
    assert_eq!(published, stored);
}

#[test]
fn lifecycle_event_validation_rejects_zero_timestamp() {
    let env = Env::default();
    let market_id = Symbol::new(&env, "btc_50k");
    let oracle_address = Address::generate(&env);
    let mut lifecycle_event = EventTestingUtils::create_test_oracle_lifecycle_event(
        &env,
        &market_id,
        &oracle_address,
    );

    assert!(EventValidator::validate_oracle_lifecycle_event(&lifecycle_event).is_ok());

    lifecycle_event.timestamp = 0;
    assert_eq!(
        EventValidator::validate_oracle_lifecycle_event(&lifecycle_event),
        Err(Error::InvalidInput)
    );
}
