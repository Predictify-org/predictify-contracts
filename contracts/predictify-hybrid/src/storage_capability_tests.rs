use crate::{
    capabilities::{storage, SUPPORTED},
    PredictifyHybrid, PredictifyHybridClient,
};
use soroban_sdk::{
    testutils::{EnvTestConfig, Events},
    Env, Symbol,
};

fn test_env() -> Env {
    let mut env = Env::default();
    env.set_config(EnvTestConfig {
        capture_snapshot_at_drop: false,
    });
    env
}

#[test]
fn public_view_advertises_storage_capabilities() {
    let env = test_env();
    let contract_id = env.register(PredictifyHybrid, ());
    let client = PredictifyHybridClient::new(&env, &contract_id);

    let bitmap = client.capabilities();

    assert_eq!(bitmap, SUPPORTED);
    assert_eq!(bitmap & storage::SUPPORTED, storage::SUPPORTED);
}

#[test]
fn public_view_requires_no_auth_and_has_no_side_effects() {
    let env = test_env();
    let contract_id = env.register(PredictifyHybrid, ());
    let client = PredictifyHybridClient::new(&env, &contract_id);
    let sentinel = Symbol::new(&env, "cap_test");

    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&sentinel, &7u32);
    });
    let events_before = env.events().all().len();

    let first = client.capabilities();
    let second = client.capabilities();

    assert_eq!(
        first, second,
        "the compile-time bitmap must be deterministic"
    );
    assert!(
        env.auths().is_empty(),
        "capabilities() must not require authorization"
    );
    assert_eq!(
        env.events().all().len(),
        events_before,
        "capabilities() must not emit events"
    );
    env.as_contract(&contract_id, || {
        assert_eq!(
            env.storage().persistent().get::<_, u32>(&sentinel),
            Some(7),
            "capabilities() must not mutate existing contract storage"
        );
    });
}
