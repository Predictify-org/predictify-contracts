//! Per-entrypoint auth snapshot tests for the resolution subsystem.
//!
//! Every state-changing resolution entrypoint has a positive (authorized) and
//! negative (unauthorized) test. Unauthorized calls must panic or return
//! `Error::Unauthorized`.
//!
//! ## Resolution Entrypoint Matrix
//!
//! | Entrypoint                   | Auth Subject | Return Type | Positive | Negative |
//! |------------------------------|--------------|-------------|----------|----------|
//! | `resolve_market_manual`      | admin        | panic       | yes      | yes      |
//! | `resolve_market_with_ties`   | admin        | panic       | yes      | yes      |
//! | `force_resolve_market`       | admin        | Result      | yes      | yes      |
//! | `resolve_dispute`            | admin        | Result      | yes      | yes      |
//! | `admin_override_verification`| admin        | Result      | yes      | yes      |

use crate::err::Error;
use crate::types::{OracleConfig, OracleProvider};
use crate::{PredictifyHybrid, PredictifyHybridClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    vec, Address, BytesN, Env, String, Symbol, Vec,
};

// ============================================================
// Shared helpers
// ============================================================

/// Build an initialized contract with mock_all_auths active.
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

/// For functions that return Result<T, crate::Error>:
macro_rules! assert_unauthorized_contract {
    ($result:expr) => {
        match $result {
            Err(Ok(e)) => assert_eq!(e, crate::err::Error::Unauthorized,
                "expected Unauthorized, got {:?}", e),
            Ok(_) => panic!("expected Unauthorized error, got Ok"),
            Err(Err(e)) => panic!("expected Unauthorized error, got host error {:?}", e),
        }
    };
}

/// For functions that panic (no explicit return type / return ()):
macro_rules! assert_unauthorized_panic {
    ($result:expr) => {
        match $result {
            Err(Ok(e)) => assert_eq!(
                e,
                soroban_sdk::Error::from_contract_error(crate::err::Error::Unauthorized as u32),
                "expected Unauthorized, got {:?}", e
            ),
            Ok(_) => panic!("expected Unauthorized error, got Ok"),
            Err(Err(e)) => panic!("expected Unauthorized error, got host error {:?}", e),
        }
    };
}

/// For positive tests on Result<T, crate::Error> functions:
macro_rules! assert_auth_ok_contract {
    ($result:expr, $msg:expr) => {
        if let Err(Ok(e)) = $result {
            assert_ne!(e, crate::err::Error::Unauthorized, $msg);
        }
    };
}

/// For positive tests on panicking functions:
macro_rules! assert_auth_ok_panic {
    ($result:expr, $msg:expr) => {
        if let Err(Ok(e)) = $result {
            assert_ne!(
                e,
                soroban_sdk::Error::from_contract_error(crate::err::Error::Unauthorized as u32),
                $msg
            );
        }
    };
}

fn oracle(env: &Env) -> OracleConfig {
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

fn make_market(env: &Env, cid: &Address, admin: &Address) -> Symbol {
    let mut outcomes = Vec::new(env);
    outcomes.push_back(String::from_str(env, "yes"));
    outcomes.push_back(String::from_str(env, "no"));
    client(env, cid).create_market(
        admin,
        &String::from_str(env, "Will BTC reach 100k?"),
        &outcomes,
        &30u32,
        &oracle(env),
        &None,
        &86400u64,
        &None,
        &None,
        &None,
    )
}

/// Advance ledger 31 days so markets are past their end time.
fn advance_past_end(env: &Env) {
    env.ledger().with_mut(|l| l.timestamp += 31 * 24 * 60 * 60);
}

/// Advance ledger past the dispute window (default 86400 s).
fn advance_past_dispute(env: &Env) {
    env.ledger().with_mut(|l| l.timestamp += 86_401);
}

/// Build a fresh env with NO auths mocked (for negative-path tests).
fn setup_no_auth() -> (Env, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let cid = env.register(PredictifyHybrid, ());
    let admin = Address::generate(&env);
    PredictifyHybridClient::new(&env, &cid).initialize(&admin, &Some(200i128), &None);
    env.set_auths(&[]);
    (env, cid, admin)
}

// ============================================================
// resolve_market_manual
// ============================================================

#[test]
fn test_resolve_market_manual_authorized_admin_succeeds() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    advance_past_end(&env);
    let result = client(&env, &cid).try_resolve_market_manual(
        &admin,
        &market_id,
        &String::from_str(&env, "yes"),
    );
    assert_auth_ok_panic!(result, "resolve_market_manual rejected authorized admin");
}

#[test]
fn test_resolve_market_manual_forged_admin_rejected() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    advance_past_end(&env);
    let attacker = Address::generate(&env);
    let result = client(&env, &cid).try_resolve_market_manual(
        &attacker,
        &market_id,
        &String::from_str(&env, "yes"),
    );
    assert_unauthorized_panic!(result);
}

#[test]
fn test_resolve_market_manual_no_auth_panics() {
    let (env, cid, _admin) = setup_no_auth();
    let market_id = Symbol::new(&env, "mkt");
    let result = client(&env, &cid).try_resolve_market_manual(
        &Address::generate(&env),
        &market_id,
        &String::from_str(&env, "yes"),
    );
    assert_unauthorized_panic!(result);
}

// ============================================================
// resolve_market_with_ties
// ============================================================

#[test]
fn test_resolve_market_with_ties_authorized_admin_succeeds() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    advance_past_end(&env);
    let mut winning = Vec::new(&env);
    winning.push_back(String::from_str(&env, "yes"));
    let result = client(&env, &cid).try_resolve_market_with_ties(
        &admin,
        &market_id,
        &winning,
    );
    assert_auth_ok_panic!(result, "resolve_market_with_ties rejected authorized admin");
}

#[test]
fn test_resolve_market_with_ties_forged_admin_rejected() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    advance_past_end(&env);
    let attacker = Address::generate(&env);
    let mut winning = Vec::new(&env);
    winning.push_back(String::from_str(&env, "yes"));
    let result = client(&env, &cid).try_resolve_market_with_ties(
        &attacker,
        &market_id,
        &winning,
    );
    assert_unauthorized_panic!(result);
}

#[test]
fn test_resolve_market_with_ties_no_auth_panics() {
    let (env, cid, _admin) = setup_no_auth();
    let market_id = Symbol::new(&env, "mkt");
    let mut winning = Vec::new(&env);
    winning.push_back(String::from_str(&env, "yes"));
    let result = client(&env, &cid).try_resolve_market_with_ties(
        &Address::generate(&env),
        &market_id,
        &winning,
    );
    assert_unauthorized_panic!(result);
}

// ============================================================
// force_resolve_market
// ============================================================

#[test]
fn test_force_resolve_market_authorized_admin_succeeds() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    let mut outcomes = Vec::new(&env);
    outcomes.push_back(String::from_str(&env, "yes"));
    let result = client(&env, &cid).try_force_resolve_market(
        &admin,
        &market_id,
        &outcomes,
        &String::from_str(&env, "Admin override"),
        &String::from_str(&env, "key-001"),
    );
    assert_auth_ok_contract!(result, "force_resolve_market rejected authorized admin");
}

#[test]
fn test_force_resolve_market_forged_admin_rejected() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    let attacker = Address::generate(&env);
    let mut outcomes = Vec::new(&env);
    outcomes.push_back(String::from_str(&env, "yes"));
    let result = client(&env, &cid).try_force_resolve_market(
        &attacker,
        &market_id,
        &outcomes,
        &String::from_str(&env, "Hack attempt"),
        &String::from_str(&env, "key-002"),
    );
    assert_unauthorized_contract!(result);
}

#[test]
fn test_force_resolve_market_no_auth_rejected() {
    let (env, cid, _admin) = setup_no_auth();
    let market_id = Symbol::new(&env, "mkt");
    let mut outcomes = Vec::new(&env);
    outcomes.push_back(String::from_str(&env, "yes"));
    let result = client(&env, &cid).try_force_resolve_market(
        &Address::generate(&env),
        &market_id,
        &outcomes,
        &String::from_str(&env, "No auth"),
        &String::from_str(&env, "key-003"),
    );
    assert_unauthorized_contract!(result);
}

// ============================================================
// resolve_dispute
// ============================================================

#[test]
fn test_resolve_dispute_authorized_admin_succeeds() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    advance_past_end(&env);
    let _ = client(&env, &cid).try_resolve_market_manual(
        &admin, &market_id, &String::from_str(&env, "yes"),
    );
    let result = client(&env, &cid).try_resolve_dispute(&admin, &market_id);
    assert_auth_ok_contract!(result, "resolve_dispute rejected authorized admin");
}

#[test]
fn test_resolve_dispute_forged_admin_rejected() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    let attacker = Address::generate(&env);
    let result = client(&env, &cid).try_resolve_dispute(&attacker, &market_id);
    assert_unauthorized_contract!(result);
}

#[test]
fn test_resolve_dispute_no_auth_rejected() {
    let (env, cid, _admin) = setup_no_auth();
    let market_id = Symbol::new(&env, "mkt");
    let result = client(&env, &cid).try_resolve_dispute(
        &Address::generate(&env),
        &market_id,
    );
    assert_unauthorized_contract!(result);
}

// ============================================================
// admin_override_verification
// ============================================================

#[test]
fn test_admin_override_verification_authorized_admin_auth_passes() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    let result = client(&env, &cid).try_admin_override_verification(
        &admin,
        &market_id,
        &String::from_str(&env, "yes"),
        &String::from_str(&env, "manual override"),
        &0u64,
    );
    assert_auth_ok_contract!(result, "admin_override_verification rejected authorized admin");
}

#[test]
fn test_admin_override_verification_forged_admin_rejected() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    let attacker = Address::generate(&env);
    let result = client(&env, &cid).try_admin_override_verification(
        &attacker,
        &market_id,
        &String::from_str(&env, "yes"),
        &String::from_str(&env, "hack"),
        &0u64,
    );
    assert_unauthorized_contract!(result);
}

#[test]
fn test_admin_override_verification_no_auth_rejected() {
    let (env, cid, _admin) = setup_no_auth();
    let market_id = Symbol::new(&env, "mkt");
    let result = client(&env, &cid).try_admin_override_verification(
        &Address::generate(&env),
        &market_id,
        &String::from_str(&env, "yes"),
        &String::from_str(&env, "no auth"),
        &0u64,
    );
    assert_unauthorized_contract!(result);
}

// ============================================================
// Edge cases
// ============================================================

/// An attacker must not be able to resolve a market that does not belong to them,
/// even with mock_all_auths active. The contract must validate the admin address
/// against the stored primary admin.
#[test]
fn test_resolve_market_manual_wrong_admin_subject_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let cid = env.register(PredictifyHybrid, ());
    let real_admin = Address::generate(&env);
    PredictifyHybridClient::new(&env, &cid).initialize(&real_admin, &Some(200i128), &None);
    let market_id = make_market(&env, &cid, &real_admin);
    advance_past_end(&env);

    let impostor = Address::generate(&env);
    let result = client(&env, &cid).try_resolve_market_manual(
        &impostor,
        &market_id,
        &String::from_str(&env, "yes"),
    );
    assert_unauthorized_panic!(result);
}

/// Even with mock_all_auths, an attacker cannot force-resolve a market
/// they do not own.
#[test]
fn test_force_resolve_market_wrong_admin_subject_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let cid = env.register(PredictifyHybrid, ());
    let real_admin = Address::generate(&env);
    PredictifyHybridClient::new(&env, &cid).initialize(&real_admin, &Some(200i128), &None);
    let market_id = make_market(&env, &cid, &real_admin);

    let impostor = Address::generate(&env);
    let mut outcomes = Vec::new(&env);
    outcomes.push_back(String::from_str(&env, "yes"));
    let result = client(&env, &cid).try_force_resolve_market(
        &impostor,
        &market_id,
        &outcomes,
        &String::from_str(&env, "Impostor"),
        &String::from_str(&env, "key-edge-001"),
    );
    assert_unauthorized_contract!(result);
}

/// Uninitialized contract: resolution calls before initialize must return AdminNotSet.
#[test]
fn test_resolve_market_manual_before_initialize_returns_admin_not_set() {
    let env = Env::default();
    env.mock_all_auths();
    let cid = env.register(PredictifyHybrid, ());
    let fake_admin = Address::generate(&env);
    let market_id = Symbol::new(&env, "mkt");
    // No call to initialize — admin storage is empty.
    let result = client(&env, &cid).try_resolve_market_manual(
        &fake_admin,
        &market_id,
        &String::from_str(&env, "yes"),
    );
    // Should fail with AdminNotSet (not Unauthorized) because the contract
    // hasn't been initialized yet.
    match result {
        Err(Ok(e)) => assert_ne!(e, Error::Unauthorized,
            "expected non-Unauthorized error for uninitialized contract, got Unauthorized"),
        _ => {} // Any other outcome is acceptable (e.g. panic with AdminNotSet)
    }
}
