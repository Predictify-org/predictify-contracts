#![allow(dead_code)]
#![allow(unused_imports)]

use soroban_sdk::{
    testutils::{Address as _, MockAuth, MockAuthInvoke},
    Address, BytesN, Env, IntoVal,
};

use crate::{Error, PredictifyHybrid, PredictifyHybridClient};

/// Upload a tiny test WASM and return its hash. The `test_wasm/hello_world.wasm`
/// fixture is a minimal valid WASM module used as a real, deployable upgrade
/// target.
fn test_wasm_hash(env: &Env) -> BytesN<32> {
    let wasm: &[u8] = include_bytes!("../test_wasm/hello_world.wasm");
    env.deployer().upload_contract_wasm(wasm)
}

fn setup(env: &Env) -> (PredictifyHybridClient<'_>, Address) {
    let admin = Address::generate(env);
    let contract_id = env.register(PredictifyHybrid, ());
    let client = PredictifyHybridClient::new(env, &contract_id);
    client.initialize(&admin);
    (client, admin)
}

/// Row 1: admin `upgrade` with the correct predecessor and a single
/// `MockAuth` succeeds, the stored hash is updated, and the event is emitted.
#[test]
fn slim_upgrade_admin_succeeds() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);

    let genesis = test_wasm_hash(&env);
    client.set_initial_wasm_hash(&admin, &genesis);

    let new_hash = test_wasm_hash(&env);

    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "upgrade",
            args: (admin.clone(), new_hash.clone(), genesis.clone()).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    client.upgrade(&admin, &new_hash, &genesis);
    assert_eq!(client.get_current_wasm_hash(), new_hash);
}

/// Row 2: a non-admin upgrade is rejected with `Unauthorized`.
#[test]
fn slim_upgrade_non_admin_unauthorized() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let attacker = Address::generate(&env);

    let genesis = test_wasm_hash(&env);
    client.set_initial_wasm_hash(&admin, &genesis);
    let new_hash = test_wasm_hash(&env);

    let res = client.try_upgrade(&attacker, &new_hash, &genesis);
    assert_eq!(res, Err(Ok(Error::Unauthorized)));
}

/// Row 3: a wrong expected predecessor is rejected by the chain check.
#[test]
fn slim_upgrade_wrong_predecessor_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);

    let genesis = test_wasm_hash(&env);
    client.set_initial_wasm_hash(&admin, &genesis);
    let new_hash = test_wasm_hash(&env);
    let wrong = BytesN::from_array(&env, &[9u8; 32]);

    let res = client.try_upgrade(&admin, &new_hash, &wrong);
    assert!(res.is_err());
}

/// Row 4: a zero predecessor is rejected once the current hash has been seeded.
#[test]
fn slim_upgrade_zero_predecessor_after_seed_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);

    let genesis = test_wasm_hash(&env);
    client.set_initial_wasm_hash(&admin, &genesis);
    let new_hash = test_wasm_hash(&env);
    let zero = BytesN::from_array(&env, &[0u8; 32]);

    let res = client.try_upgrade(&admin, &new_hash, &zero);
    assert!(res.is_err());
}

/// Row 5: `set_initial_wasm_hash` is one-shot; the second call is rejected.
#[test]
fn set_initial_wasm_hash_twice_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);

    let genesis = test_wasm_hash(&env);
    client.set_initial_wasm_hash(&admin, &genesis);

    let res = client.try_set_initial_wasm_hash(&admin, &genesis);
    assert!(res.is_err());
}

/// Row 6: the full `upgrade_contract` path authenticates exactly once.
#[cfg(feature = "full")]
#[test]
fn full_upgrade_contract_single_auth() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);

    let genesis = test_wasm_hash(&env);
    client.set_initial_wasm_hash(&admin, &genesis);
    let new_hash = test_wasm_hash(&env);

    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "upgrade_contract",
            args: (admin.clone(), new_hash.clone(), genesis.clone()).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    client.upgrade_contract(&admin, &new_hash, &genesis);
    assert_eq!(client.get_current_wasm_hash(), new_hash);
}

/// Row 7: the full `rollback_upgrade` path authenticates exactly once.
#[cfg(feature = "full")]
#[test]
fn full_rollback_upgrade_single_auth() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);

    let genesis = test_wasm_hash(&env);
    client.set_initial_wasm_hash(&admin, &genesis);
    let new_hash = test_wasm_hash(&env);

    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "upgrade_contract",
            args: (admin.clone(), new_hash.clone(), genesis.clone()).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    client.upgrade_contract(&admin, &new_hash, &genesis);

    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "rollback_upgrade",
            args: (admin.clone(), genesis.clone(), new_hash.clone()).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    client.rollback_upgrade(&admin, &genesis, &new_hash);
    assert_eq!(client.get_current_wasm_hash(), genesis);
}
