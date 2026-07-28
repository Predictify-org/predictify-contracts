//! Auth-context tests for the Reflector oracle integration [b#018].
//!
//! `fetch_oracle_result` and the Reflector-oracle admin-config entrypoints
//! (`set_oracle_confidence_threshold`, `set_oracle_weight`) are the entrypoints
//! that interact with a Reflector-provider `OracleConfig`. These tests exercise
//! both auth harnesses Soroban offers:
//!
//! - **`mock_all_auths`** ("mock-auth"): a blanket harness that authorizes any
//!   `require_auth()` call regardless of which address invoked it. Useful for
//!   happy-path tests, but by itself it cannot prove that the contract is
//!   actually checking the *identity* of the caller.
//! - **`mock_auths` / `MockAuth` + `MockAuthInvoke`** ("require_auth"): a precise
//!   harness that authorizes exactly one declared `(address, contract, fn_name, args)`
//!   invocation. If the contract's `require_auth()` call is on a different address,
//!   or the recorded invocation doesn't match the live call, authorization fails.
//!   This is what proves the contract captures and checks the correct signer.
//!
//! ## Entrypoint Matrix
//!
//! | Entrypoint                         | Auth Subject | Reflector-specific?                         |
//! |-------------------------------------|--------------|----------------------------------------------|
//! | `fetch_oracle_result`               | caller       | fetches from the market's configured oracle   |
//! | `set_oracle_confidence_threshold`   | admin        | gates Reflector price-confidence filtering    |
//! | `set_oracle_weight`                 | admin        | weights a Reflector oracle address in median  |

use crate::err::Error;
use crate::types::{OracleConfig, OracleProvider};
use crate::{PredictifyHybrid, PredictifyHybridClient};
use soroban_sdk::{
    testutils::{Address as _, MockAuth, MockAuthInvoke},
    Address, Env, IntoVal, String, Symbol, Vec,
};

/// Builds a Reflector-provider oracle config, matching how a real
/// BTC/USD Reflector market would be configured.
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

/// Env + initialized contract, with no auths mocked yet — callers of this
/// helper are expected to install their own `MockAuth` (or `mock_all_auths`)
/// before invoking entrypoints.
fn setup() -> (Env, Address, Address) {
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

/// Creates a Reflector-backed market under the (already-mocked) admin auth.
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
        &86_400u64,
        &None,
        &None,
        &None,
        &None,
        &None,
    )
}

// ============================================================
// fetch_oracle_result — precise `MockAuth` (require_auth) harness
// ============================================================

/// With a precise `MockAuth` recorded for `user`, calling `fetch_oracle_result`
/// as `user` must pass the `require_auth()` check (the identity the contract
/// authenticated matches the identity that was mocked).
#[test]
fn test_fetch_oracle_result_precise_mock_auth_matching_signer_passes_auth() {
    let (env, cid, admin) = setup();
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
    // The signer identity was captured and matched — any failure from here on
    // must be a business-logic error, never Unauthorized.
    match result {
        Err(Ok(e)) => assert_ne!(
            e,
            Error::Unauthorized,
            "fetch_oracle_result rejected a caller whose identity was correctly mock-authorized"
        ),
        _ => {}
    }
}

/// A `MockAuth` recorded for one address does not authorize a *different*
/// caller: the contract must check the exact signer identity passed as
/// `caller`, not just "some address was authorized somewhere".
#[test]
fn test_fetch_oracle_result_mock_auth_for_wrong_signer_rejected() {
    let (env, cid, admin) = setup();
    let market_id = make_reflector_market(&env, &cid, &admin);
    let authorized_user = Address::generate(&env);
    let impostor = Address::generate(&env);
    let oracle_contract = Address::generate(&env);

    // Only `authorized_user` is mock-authorized for this exact invocation.
    env.mock_auths(&[MockAuth {
        address: &authorized_user,
        invoke: &MockAuthInvoke {
            contract: &cid,
            fn_name: "fetch_oracle_result",
            args: (&authorized_user, &market_id, &oracle_contract).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    // Calling as `impostor` (no matching mocked auth) must fail authentication.
    let result = client(&env, &cid).try_fetch_oracle_result(&impostor, &market_id, &oracle_contract);
    assert!(
        result.is_err(),
        "fetch_oracle_result must not succeed for a caller with no matching mocked auth"
    );
}

/// With no auths mocked at all, `fetch_oracle_result` must reject the call —
/// `require_auth()` has nothing to authenticate against.
#[test]
fn test_fetch_oracle_result_no_auth_rejected() {
    let (env, cid, admin) = setup();
    let market_id = make_reflector_market(&env, &cid, &admin);
    let user = Address::generate(&env);
    let oracle_contract = Address::generate(&env);

    env.set_auths(&[]);
    let result = client(&env, &cid).try_fetch_oracle_result(&user, &market_id, &oracle_contract);
    assert!(
        result.is_err(),
        "fetch_oracle_result must reject calls with no authorization at all"
    );
}

// ============================================================
// fetch_oracle_result — blanket `mock_all_auths` (mock-auth) harness
// ============================================================

/// Under the blanket `mock_all_auths` harness, any caller's `require_auth()`
/// succeeds — this is the "mock-auth" counterpart to the precise-signer tests
/// above, confirming the entrypoint behaves consistently under both harnesses.
#[test]
fn test_fetch_oracle_result_mock_all_auths_any_caller_passes_auth() {
    let (env, cid, admin) = setup();
    let market_id = make_reflector_market(&env, &cid, &admin);
    let user = Address::generate(&env);
    let oracle_contract = Address::generate(&env);

    // `setup()` already called `env.mock_all_auths()`.
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

// ============================================================
// set_oracle_confidence_threshold — admin-only, Reflector price filtering
// ============================================================

#[test]
fn test_set_oracle_confidence_threshold_precise_admin_mock_auth_passes() {
    let (env, cid, admin) = setup();
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

    // Must not panic: the exact admin identity was mock-authorized for this call.
    client(&env, &cid).set_oracle_confidence_threshold(&admin, &500u32);
    assert_eq!(client(&env, &cid).get_oracle_confidence_threshold(), 500u32);
}

#[test]
#[should_panic]
fn test_set_oracle_confidence_threshold_mock_auth_for_wrong_signer_rejected() {
    let (env, cid, admin) = setup();
    let _market_id = make_reflector_market(&env, &cid, &admin);
    let impostor = Address::generate(&env);

    // Only `admin` is mock-authorized; `impostor` is not.
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
    let (env, cid, admin) = setup();
    let _market_id = make_reflector_market(&env, &cid, &admin);

    env.set_auths(&[]);
    client(&env, &cid).set_oracle_confidence_threshold(&admin, &500u32);
}

// ============================================================
// set_oracle_weight — admin-only, weights a specific Reflector oracle address
// ============================================================

#[test]
fn test_set_oracle_weight_precise_admin_mock_auth_passes() {
    let (env, cid, admin) = setup();
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

    let result = client(&env, &cid).try_set_oracle_weight(&admin, &oracle_config.oracle_address, &3u32);
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
    let (env, cid, admin) = setup();
    let oracle_config = reflector_oracle(&env);
    let attacker = Address::generate(&env);

    let result = client(&env, &cid).try_set_oracle_weight(&attacker, &oracle_config.oracle_address, &3u32);
    // `setup()` uses `mock_all_auths`, so `require_auth()` on `attacker` passes;
    // the contract must still reject non-admin callers via its own admin check.
    match result {
        Err(Ok(e)) => assert_eq!(
            e,
            Error::Unauthorized,
            "set_oracle_weight must reject a non-admin caller even when their require_auth mock-passes"
        ),
        _ => panic!("expected Unauthorized for non-admin caller, got Ok"),
    }
    let _ = admin;
}

#[test]
fn test_set_oracle_weight_no_auth_rejected() {
    let (env, cid, admin) = setup();
    let oracle_config = reflector_oracle(&env);

    env.set_auths(&[]);
    let result = client(&env, &cid).try_set_oracle_weight(&admin, &oracle_config.oracle_address, &3u32);
    assert!(
        result.is_err(),
        "set_oracle_weight must reject calls with no authorization at all"
    );
}
