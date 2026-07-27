//! Per-entrypoint authorization snapshot tests for resolution entrypoints.
//!
//! This integration suite *snapshots* the Soroban authorization required by
//! every state-changing resolution entrypoint of [`PredictifyHybrid`]. Instead
//! of only checking that an authorized call succeeds and an unauthorized one
//! fails, these tests inspect [`Env::auths`] immediately after each call and
//! assert **which address the host actually required an authorization from**.
//!
//! ## Entrypoint auth matrix
//!
//! | Entrypoint                       | Required auth subject | Verified by        |
//! |----------------------------------|-----------------------|--------------------|
//! | `resolve_market_manual`          | admin                 | committed snapshot |
//! | `resolve_market_with_ties`       | admin                 | committed snapshot |
//! | `force_resolve_market`           | admin                 | committed snapshot |
//! | `admin_override_verification`    | admin                 | committed snapshot |
//! | `set_resolution_cooldown`        | admin                 | committed snapshot |
//! | `fetch_oracle_result`            | none                  | committed snapshot |
//! | `distribute_payouts`             | none                  | committed snapshot |
//! | `resolve_market` (deprecated)    | caller                | auth boundary      |
//! | `verify_result` (deprecated)     | caller                | auth boundary      |
//! | `verify_result_with_retry`       | caller                | auth boundary      |
//! | `get_resolution_analytics`       | none                  | committed snapshot |
//! | `get_verified_result`            | none                  | committed snapshot |
//! | `is_result_verified`             | none                  | committed snapshot |

use predictify_hybrid::{
    Error, OracleConfig, OracleProvider, PredictifyHybrid, PredictifyHybridClient,
};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token::StellarAssetClient,
    Address, Env, String, Symbol, Vec,
};

// ============================================================
// Fixture
// ============================================================

struct Fixture {
    env: Env,
    cid: Address,
    admin: Address,
    token_id: Address,
}

impl Fixture {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let cid = env.register(PredictifyHybrid, ());

        let token_id = env
            .register_stellar_asset_contract_v2(Address::generate(&env))
            .address();

        env.as_contract(&cid, || {
            env.storage()
                .persistent()
                .set(&Symbol::new(&env, "TokenID"), &token_id);
        });

        PredictifyHybridClient::new(&env, &cid).initialize(&admin, &Some(200i128), &None);

        Fixture {
            env,
            cid,
            admin,
            token_id,
        }
    }

    fn client(&self) -> PredictifyHybridClient<'_> {
        PredictifyHybridClient::new(&self.env, &self.cid)
    }

    fn user(&self) -> Address {
        let u = Address::generate(&self.env);
        StellarAssetClient::new(&self.env, &self.token_id).mint(&u, &100_000_000_000i128);
        u
    }

    fn oracle(&self) -> OracleConfig {
        OracleConfig {
            provider: OracleProvider::reflector(),
            oracle_address: Address::from_str(
                &self.env,
                "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
            ),
            feed_id: String::from_str(&self.env, "BTC/USD"),
            threshold: 50_000,
            comparison: String::from_str(&self.env, "gt"),
        }
    }

    fn market(&self) -> Symbol {
        let mut outcomes = Vec::new(&self.env);
        outcomes.push_back(String::from_str(&self.env, "yes"));
        outcomes.push_back(String::from_str(&self.env, "no"));
        self.client().create_market(
            &self.admin,
            &String::from_str(&self.env, "Will BTC reach 100k?"),
            &outcomes,
            &30u32,
            &self.oracle(),
            &None,
            &86_400u64,
            &None,
            &None,
            &None,
            &None,
            &None,
        )
    }

    fn yes(&self) -> String {
        String::from_str(&self.env, "yes")
    }

    fn no(&self) -> String {
        String::from_str(&self.env, "no")
    }

    fn advance_past_end(&self) {
        self.env
            .ledger()
            .with_mut(|l| l.timestamp += 31 * 24 * 60 * 60);
    }

    fn advance_past_dispute(&self) {
        self.env.ledger().with_mut(|l| l.timestamp += 86_401);
    }

    fn required_auth(&self) -> std::vec::Vec<Address> {
        self.env
            .auths()
            .iter()
            .map(|(addr, _)| addr.clone())
            .collect()
    }

    fn assert_requires_auth(&self, expected: &Address, label: &str) {
        let required = self.required_auth();
        assert!(
            !required.is_empty(),
            "{label}: no auth recorded - the entrypoint either skipped require_auth \
             or the call failed before committing"
        );
        assert!(
            required.contains(expected),
            "{label}: expected auth from {expected:?}, captured {required:?}"
        );
    }

    fn assert_no_auth(&self, label: &str) {
        let required = self.required_auth();
        assert!(
            required.is_empty(),
            "{label}: expected no auth for a permissionless entrypoint, captured {required:?}"
        );
    }
}

/// Assert the call got *past* `require_auth`.
fn assert_auth_passed<T, E, F>(
    result: &Result<Result<T, E>, Result<F, soroban_sdk::InvokeError>>,
    label: &str,
) {
    if let Err(Err(invoke_err)) = result {
        panic!(
            "{label}: expected the call to pass require_auth and fail on domain \
             logic, but it trapped at the host level: {invoke_err:?}"
        );
    }
}

// ============================================================
// Admin-scoped resolution entrypoints - committed snapshot
// ============================================================

#[test]
fn snapshot_resolve_market_manual_requires_admin_auth() {
    let f = Fixture::new();
    let market_id = f.market();
    f.advance_past_end();
    f.client()
        .resolve_market_manual(&f.admin, &market_id, &f.yes());
    f.assert_requires_auth(&f.admin, "resolve_market_manual");
}

#[test]
fn snapshot_resolve_market_with_ties_requires_admin_auth() {
    let f = Fixture::new();
    let market_id = f.market();
    f.advance_past_end();
    f.client()
        .resolve_market_with_ties(&f.admin, &market_id, &Vec::from_array(&f.env, [f.yes()]));
    f.assert_requires_auth(&f.admin, "resolve_market_with_ties");
}

#[test]
fn snapshot_force_resolve_market_requires_admin_auth() {
    let f = Fixture::new();
    let market_id = f.market();
    f.client().force_resolve_market(
        &f.admin,
        &market_id,
        &Vec::from_array(&f.env, [f.yes()]),
        &String::from_str(&f.env, "Emergency override"),
        &String::from_str(&f.env, "key-001"),
    );
    f.assert_requires_auth(&f.admin, "force_resolve_market");
}

#[test]
fn snapshot_admin_override_verification_requires_admin_auth() {
    let f = Fixture::new();
    let market_id = f.market();
    let result = f.client().admin_override_verification(
        &f.admin,
        &market_id,
        &f.yes(),
        &String::from_str(&f.env, "Oracle failure"),
        &1u64,
    );
    assert_auth_passed(&result, "admin_override_verification");
    f.assert_requires_auth(&f.admin, "admin_override_verification");
}

#[test]
fn snapshot_set_resolution_cooldown_requires_admin_auth() {
    let f = Fixture::new();
    let _ = f.client().set_resolution_cooldown(&f.admin, &3600u64);
    f.assert_requires_auth(&f.admin, "set_resolution_cooldown");
}

// ============================================================
// Permissionless resolution entrypoints - committed snapshot (no auth)
// ============================================================

#[test]
fn snapshot_fetch_oracle_result_requires_no_auth() {
    let f = Fixture::new();
    let market_id = f.market();
    f.advance_past_end();
    let oracle_addr = Address::generate(&f.env);
    let _ = f
        .client()
        .try_fetch_oracle_result(&market_id, &oracle_addr);
    f.assert_no_auth("fetch_oracle_result");
}

#[test]
fn snapshot_distribute_payouts_requires_no_auth() {
    let f = Fixture::new();
    let market_id = f.market();
    let user = f.user();
    f.client()
        .vote(&user, &market_id, &f.yes(), &1_000_000i128);
    f.advance_past_end();
    f.client()
        .resolve_market_manual(&f.admin, &market_id, &f.yes());
    let _ = f.client().try_distribute_payouts(&market_id);
    f.assert_no_auth("distribute_payouts");
}

// ============================================================
// Read-only resolution entrypoints - committed snapshot (no auth)
// ============================================================

#[test]
fn snapshot_get_resolution_analytics_requires_no_auth() {
    let f = Fixture::new();
    let _ = f.client().try_get_resolution_analytics();
    f.assert_no_auth("get_resolution_analytics");
}

#[test]
fn snapshot_get_verified_result_requires_no_auth() {
    let f = Fixture::new();
    let market_id = f.market();
    let _ = f.client().get_verified_result(&market_id);
    f.assert_no_auth("get_verified_result");
}

#[test]
fn snapshot_is_result_verified_requires_no_auth() {
    let f = Fixture::new();
    let market_id = f.market();
    let _ = f.client().is_result_verified(&market_id);
    f.assert_no_auth("is_result_verified");
}

// ============================================================
// Deprecated resolution entrypoints - auth boundary
// ============================================================

#[test]
fn snapshot_resolve_market_requires_caller_auth() {
    let f = Fixture::new();
    let market_id = f.market();
    let user = f.user();
    let result = f.client().try_resolve_market(&user, &market_id);
    assert_auth_passed(&result, "resolve_market (deprecated)");
}

#[test]
#[should_panic]
fn edge_resolve_market_without_auth_panics() {
    let f = Fixture::new();
    let market_id = f.market();
    let user = f.user();
    f.env.set_auths(&[]);
    f.client().resolve_market(&user, &market_id);
}

#[test]
fn snapshot_verify_result_requires_caller_auth() {
    let f = Fixture::new();
    let market_id = f.market();
    let user = f.user();
    let result = f.client().try_verify_result(&user, &market_id);
    assert_auth_passed(&result, "verify_result (deprecated)");
}

#[test]
#[should_panic]
fn edge_verify_result_without_auth_panics() {
    let f = Fixture::new();
    let market_id = f.market();
    let user = f.user();
    f.env.set_auths(&[]);
    f.client().verify_result(&user, &market_id);
}

#[test]
fn snapshot_verify_result_with_retry_requires_caller_auth() {
    let f = Fixture::new();
    let market_id = f.market();
    let user = f.user();
    let result = f
        .client()
        .try_verify_result_with_retry(&user, &market_id, &3u32);
    assert_auth_passed(&result, "verify_result_with_retry");
}

#[test]
#[should_panic]
fn edge_verify_result_with_retry_without_auth_panics() {
    let f = Fixture::new();
    let market_id = f.market();
    let user = f.user();
    f.env.set_auths(&[]);
    f.client()
        .verify_result_with_retry(&user, &market_id, &3u32);
}

// ============================================================
// Edge cases - no-auth panics for admin resolution entrypoints
// ============================================================

#[test]
#[should_panic]
fn edge_resolve_market_with_ties_without_auth_panics() {
    let f = Fixture::new();
    let market_id = f.market();
    f.advance_past_end();
    f.env.set_auths(&[]);
    f.client()
        .resolve_market_with_ties(&f.admin, &market_id, &Vec::from_array(&f.env, [f.yes()]));
}

#[test]
#[should_panic]
fn edge_force_resolve_market_without_auth_panics() {
    let f = Fixture::new();
    let market_id = f.market();
    f.env.set_auths(&[]);
    f.client().force_resolve_market(
        &f.admin,
        &market_id,
        &Vec::from_array(&f.env, [f.yes()]),
        &String::from_str(&f.env, "Emergency"),
        &String::from_str(&f.env, "key-002"),
    );
}

#[test]
#[should_panic]
fn edge_admin_override_verification_without_auth_panics() {
    let f = Fixture::new();
    let market_id = f.market();
    f.env.set_auths(&[]);
    f.client().admin_override_verification(
        &f.admin,
        &market_id,
        &f.yes(),
        &String::from_str(&f.env, "Reason"),
        &1u64,
    );
}

#[test]
#[should_panic]
fn edge_set_resolution_cooldown_without_auth_panics() {
    let f = Fixture::new();
    f.env.set_auths(&[]);
    f.client().set_resolution_cooldown(&f.admin, &3600u64);
}

// ============================================================
// Non-admin rejection tests
// ============================================================

#[test]
fn edge_non_admin_resolve_market_with_ties_is_unauthorized() {
    let f = Fixture::new();
    let market_id = f.market();
    let attacker = Address::generate(&f.env);
    f.advance_past_end();
    let result = f.client().try_resolve_market_with_ties(
        &attacker,
        &market_id,
        &Vec::from_array(&f.env, [f.yes()]),
    );
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

#[test]
fn edge_non_admin_force_resolve_market_is_unauthorized() {
    let f = Fixture::new();
    let market_id = f.market();
    let attacker = Address::generate(&f.env);
    let result = f.client().try_force_resolve_market(
        &attacker,
        &market_id,
        &Vec::from_array(&f.env, [f.yes()]),
        &String::from_str(&f.env, "Reason"),
        &String::from_str(&f.env, "key-003"),
    );
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

#[test]
fn edge_non_admin_admin_override_verification_is_unauthorized() {
    let f = Fixture::new();
    let market_id = f.market();
    let attacker = Address::generate(&f.env);
    let result = f.client().try_admin_override_verification(
        &attacker,
        &market_id,
        &f.yes(),
        &String::from_str(&f.env, "Reason"),
        &1u64,
    );
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

#[test]
fn edge_non_admin_set_resolution_cooldown_is_unauthorized() {
    let f = Fixture::new();
    let attacker = Address::generate(&f.env);
    let result = f.client().try_set_resolution_cooldown(&attacker, &3600u64);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

// ============================================================
// Edge case - auth subject isolation
// ============================================================

/// `resolve_market_manual` must bind auth to the admin argument, not to any
/// other address in the environment.
#[test]
fn snapshot_resolve_market_manual_subject_is_admin_not_user() {
    let f = Fixture::new();
    let market_id = f.market();
    let user = f.user();
    f.advance_past_end();
    f.client()
        .resolve_market_manual(&f.admin, &market_id, &f.yes());

    let required = f.required_auth();
    assert!(
        required.contains(&f.admin),
        "resolve_market_manual must require admin auth"
    );
    assert!(
        !required.contains(&user),
        "resolve_market_manual must not require user auth: {required:?}"
    );
}

/// `force_resolve_market` must bind auth to the admin argument.
#[test]
fn snapshot_force_resolve_market_subject_is_admin_not_user() {
    let f = Fixture::new();
    let market_id = f.market();
    let user = f.user();
    f.client().force_resolve_market(
        &f.admin,
        &market_id,
        &Vec::from_array(&f.env, [f.yes()]),
        &String::from_str(&f.env, "Emergency"),
        &String::from_str(&f.env, "key-iso"),
    );

    let required = f.required_auth();
    assert!(
        required.contains(&f.admin),
        "force_resolve_market must require admin auth"
    );
    assert!(
        !required.contains(&user),
        "force_resolve_market must not require user auth: {required:?}"
    );
}
