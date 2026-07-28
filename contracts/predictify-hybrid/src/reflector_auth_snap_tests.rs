//! Per-entrypoint auth snapshot tests for the Reflector oracle integration.
//!
//! Every state-changing Reflector entrypoint is tested under three auth
//! conditions:
//!
//! 1. **Precise `MockAuth`** — the correct caller identity is mock-authorized.
//! 2. **Wrong-signer `MockAuth`** — a different address is mock-authorized.
//! 3. **No auth** — no authorization at all.
//!
//! ## Entrypoint Matrix
//!
//! | Entrypoint                         | Auth Subject | Returns         |
//! |------------------------------------|--------------|-----------------|
//! | `fetch_oracle_result`              | caller       | `Result`        |
//! | `set_oracle_confidence_threshold`  | admin        | `Result` / panic|
//! | `set_oracle_weight`                | admin        | `Result`        |
//! | `get_oracle_confidence_threshold`  | none         | value           |
//! | `get_oracle_weight`                | none         | value           |
//! | `set_oracle_val_cfg_global`        | admin        | `Result`        |
//! | `set_oracle_val_cfg_event`         | admin        | `Result`        |

use crate::err::Error;
use crate::types::{OracleConfig, OracleProvider};
use crate::{PredictifyHybrid, PredictifyHybridClient};
use soroban_sdk::{
    testutils::{Address as _, MockAuth, MockAuthInvoke},
    Address, Env, IntoVal, String, Symbol, Vec,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn reflector_oracle(env: &Env) -> OracleConfig {
    OracleConfig {
        provider: OracleProvider::reflector(),
        oracle_address: Address::from_str(
            env,
            "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
        ),
        feed_id: String::from_str(env, "BTC/USD"),
        threshold: 50_000,
        comparison: String::from_str(env, "gt"),
    }
}

fn setup_env() -> (Env, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let cid = env.register(PredictifyHybrid, ());
    let admin = Address::generate(&env);
    PredictifyHybridClient::new(&env, &cid).initialize(&admin, &Some(200i128), &None);
    (env, cid, admin)
}

fn client<'a>(env: &'a Env, cid: &'a Address) -> PredictifyHybridClient<'a> {
    PredictifyHybridClient::new(env, cid)
}

fn make_reflector_market(env: &Env, cid: &Address, admin: &Address) -> Symbol {
    let mut outcomes = Vec::new(env);
    outcomes.push_back(String::from_str(env, "yes"));
    outcomes.push_back(String::from_str(env, "no"));
    client(env, cid).create_market(
        admin,
        &String::from_str(env, "Will BTC reach 100k?"),
        &outcomes,
        &30u32,
        &reflector_oracle(env),
        &None,
        &86400u64,
        &None,
        &None,
        &None,
        &None,
        &None,
    )
}

// ===================================================================
// fetch_oracle_result — precise MockAuth
// ===================================================================

#[test]
fn test_fetch_oracle_result_precise_mock_auth_matching_signer_passes_auth() {
    let (env, cid, admin) = setup_env();
    let market_id = make_reflector_market(&env, &cid, &admin);
    let user = Address::generate(&env);
    let oracle_contract = Address::generate(&env);

    env.mock_auths(&[MockAuth {
        address: &user,
        invoke: &MockAuthInvoke {
            contract: &cid,
            fn_name: "fetch_oracle_result",
            args: (&user, &market_id, &oracle_contract).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    let result = client(&env, &cid).try_fetch_oracle_result(&user, &market_id, &oracle_contract);
    // Must not be an Unauthorized error (business-logic errors are okay).
    match result {
        Err(Ok(e)) => assert_ne!(
            e,
            Error::Unauthorized,
            "fetch_oracle_result rejected a correctly mock-authorized caller"
        ),
        _ => {}
    }
}

#[test]
fn test_fetch_oracle_result_mock_auth_for_wrong_signer_rejected() {
    let (env, cid, admin) = setup_env();
    let market_id = make_reflector_market(&env, &cid, &admin);
    let authorized_user = Address::generate(&env);
    let impostor = Address::generate(&env);
    let oracle_contract = Address::generate(&env);

    env.mock_auths(&[MockAuth {
        address: &authorized_user,
        invoke: &MockAuthInvoke {
            contract: &cid,
            fn_name: "fetch_oracle_result",
            args: (&authorized_user, &market_id, &oracle_contract).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    let result =
        client(&env, &cid).try_fetch_oracle_result(&impostor, &market_id, &oracle_contract);
    assert!(
        result.is_err(),
        "fetch_oracle_result must not succeed for a caller with no matching mocked auth"
    );
}

#[test]
fn test_fetch_oracle_result_no_auth_rejected() {
    let (env, cid, admin) = setup_env();
    let market_id = make_reflector_market(&env, &cid, &admin);
    let user = Address::generate(&env);
    let oracle_contract = Address::generate(&env);

    env.set_auths(&[]);
    let result = client(&env, &cid).try_fetch_oracle_result(&user, &market_id, &oracle_contract);
    assert!(
        result.is_err(),
        "fetch_oracle_result must reject calls with no authorization"
    );
}

// ===================================================================
// fetch_oracle_result — blanket mock_all_auths
// ===================================================================

#[test]
fn test_fetch_oracle_result_mock_all_auths_any_caller_passes_auth() {
    let (env, cid, admin) = setup_env();
    let market_id = make_reflector_market(&env, &cid, &admin);
    let user = Address::generate(&env);
    let oracle_contract = Address::generate(&env);

    let result = client(&env, &cid).try_fetch_oracle_result(&user, &market_id, &oracle_contract);
    match result {
        Err(Ok(e)) => assert_ne!(
            e,
            Error::Unauthorized,
            "fetch_oracle_result rejected an authorized caller under mock_all_auths"
        ),
        _ => {}
    }
}

// ===================================================================
// set_oracle_confidence_threshold — admin-only
// ===================================================================

#[test]
fn test_set_oracle_confidence_threshold_precise_admin_mock_auth_passes() {
    let (env, cid, admin) = setup_env();
    let _market_id = make_reflector_market(&env, &cid, &admin);

    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &cid,
            fn_name: "set_oracle_confidence_threshold",
            args: (&admin, &500u32).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    client(&env, &cid).set_oracle_confidence_threshold(&admin, &500u32);
    assert_eq!(client(&env, &cid).get_oracle_confidence_threshold(), 500u32);
}

#[test]
#[should_panic]
fn test_set_oracle_confidence_threshold_mock_auth_for_wrong_signer_rejected() {
    let (env, cid, admin) = setup_env();
    let _market_id = make_reflector_market(&env, &cid, &admin);
    let impostor = Address::generate(&env);

    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &cid,
            fn_name: "set_oracle_confidence_threshold",
            args: (&impostor, &500u32).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    client(&env, &cid).set_oracle_confidence_threshold(&impostor, &500u32);
}

#[test]
#[should_panic]
fn test_set_oracle_confidence_threshold_no_auth_rejected() {
    let (env, cid, admin) = setup_env();
    let _market_id = make_reflector_market(&env, &cid, &admin);

    env.set_auths(&[]);
    client(&env, &cid).set_oracle_confidence_threshold(&admin, &500u32);
}

// ===================================================================
// set_oracle_weight — admin-only
// ===================================================================

#[test]
fn test_set_oracle_weight_precise_admin_mock_auth_passes() {
    let (env, cid, admin) = setup_env();
    let market_id = make_reflector_market(&env, &cid, &admin);
    let oracle_config = reflector_oracle(&env);
    let _ = market_id;

    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &cid,
            fn_name: "set_oracle_weight",
            args: (&admin, &oracle_config.oracle_address, &3u32).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    let result =
        client(&env, &cid).try_set_oracle_weight(&admin, &oracle_config.oracle_address, &3u32);
    match result {
        Err(Ok(e)) => assert_ne!(
            e,
            Error::Unauthorized,
            "set_oracle_weight rejected an admin whose identity was correctly mock-authorized"
        ),
        _ => {}
    }
}

#[test]
fn test_set_oracle_weight_forged_admin_rejected() {
    let (env, cid, admin) = setup_env();
    let oracle_config = reflector_oracle(&env);
    let attacker = Address::generate(&env);

    let result =
        client(&env, &cid).try_set_oracle_weight(&attacker, &oracle_config.oracle_address, &3u32);
    match result {
        Err(Ok(e)) => assert_eq!(
            e,
            Error::Unauthorized,
            "set_oracle_weight must reject a non-admin caller"
        ),
        _ => panic!("expected Unauthorized for non-admin caller, got Ok"),
    }
    let _ = admin;
}

#[test]
fn test_set_oracle_weight_no_auth_rejected() {
    let (env, cid, admin) = setup_env();
    let oracle_config = reflector_oracle(&env);

    env.set_auths(&[]);
    let result =
        client(&env, &cid).try_set_oracle_weight(&admin, &oracle_config.oracle_address, &3u32);
    assert!(
        result.is_err(),
        "set_oracle_weight must reject calls with no authorization"
    );
}

// ===================================================================
// get_oracle_confidence_threshold — no auth required
// ===================================================================

#[test]
fn test_get_oracle_confidence_threshold_no_auth_succeeds() {
    let (env, cid, _admin) = setup_env();
    let _ = client(&env, &cid).get_oracle_confidence_threshold();
    // Must not panic or return an error.
}

// ===================================================================
// get_oracle_weight — no auth required
// ===================================================================

#[test]
fn test_get_oracle_weight_no_auth_succeeds() {
    let (env, cid, _admin) = setup_env();
    let _ = client(&env, &cid).get_oracle_weight(&Address::generate(&env));
    // Must not panic or return an error.
}

// ===================================================================
// set_oracle_val_cfg_global — admin-only
// ===================================================================

#[test]
fn test_set_oracle_val_cfg_global_precise_admin_mock_auth_passes() {
    let (env, cid, admin) = setup_env();

    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &cid,
            fn_name: "set_oracle_val_cfg_global",
            args: (&admin, &60u64, &500u32, &None::<u32>).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    let result =
        client(&env, &cid).try_set_oracle_val_cfg_global(&admin, &60u64, &500u32, &None::<u32>);
    match result {
        Err(Ok(e)) => assert_ne!(
            e,
            Error::Unauthorized,
            "set_oracle_val_cfg_global rejected authorized admin"
        ),
        _ => {}
    }
}

#[test]
fn test_set_oracle_val_cfg_global_forged_admin_rejected() {
    let (env, cid, admin) = setup_env();
    let attacker = Address::generate(&env);
    let result =
        client(&env, &cid).try_set_oracle_val_cfg_global(&attacker, &60u64, &500u32, &None::<u32>);
    match result {
        Err(Ok(e)) => assert_eq!(e, Error::Unauthorized),
        _ => panic!("expected Unauthorized"),
    }
    let _ = admin;
}

#[test]
fn test_set_oracle_val_cfg_global_no_auth_rejected() {
    let (env, cid, _admin) = setup_env();
    env.set_auths(&[]);
    let result = client(&env, &cid).try_set_oracle_val_cfg_global(
        &Address::generate(&env),
        &60u64,
        &500u32,
        &None::<u32>,
    );
    assert!(result.is_err());
}

// ===================================================================
// set_oracle_val_cfg_event — admin-only
// ===================================================================

#[test]
fn test_set_oracle_val_cfg_event_precise_admin_mock_auth_passes() {
    let (env, cid, admin) = setup_env();
    let market_id = Symbol::new(&env, "test_market");

    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &cid,
            fn_name: "set_oracle_val_cfg_event",
            args: (&admin, &market_id, &60u64, &500u32, &None::<u32>).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    let result = client(&env, &cid).try_set_oracle_val_cfg_event(
        &admin,
        &market_id,
        &60u64,
        &500u32,
        &None::<u32>,
    );
    match result {
        Err(Ok(e)) => assert_ne!(
            e,
            Error::Unauthorized,
            "set_oracle_val_cfg_event rejected authorized admin"
        ),
        _ => {}
    }
}

#[test]
fn test_set_oracle_val_cfg_event_forged_admin_rejected() {
    let (env, cid, admin) = setup_env();
    let attacker = Address::generate(&env);
    let market_id = Symbol::new(&env, "test_market");
    let result = client(&env, &cid).try_set_oracle_val_cfg_event(
        &attacker,
        &market_id,
        &60u64,
        &500u32,
        &None::<u32>,
    );
    match result {
        Err(Ok(e)) => assert_eq!(e, Error::Unauthorized),
        _ => panic!("expected Unauthorized"),
    }
    let _ = admin;
}

#[test]
fn test_set_oracle_val_cfg_event_no_auth_rejected() {
    let (env, cid, _admin) = setup_env();
    let market_id = Symbol::new(&env, "test_market");
    env.set_auths(&[]);
    let result = client(&env, &cid).try_set_oracle_val_cfg_event(
        &Address::generate(&env),
        &market_id,
        &60u64,
        &500u32,
        &None::<u32>,
    );
    assert!(result.is_err());
}
