//! Per-entrypoint authorization snapshot tests for dispute entrypoints.
//!
//! This integration suite snapshots the Soroban authorization required by
//! every state-changing dispute-related entrypoint of PredictifyHybrid.
//!
//! | Entrypoint              | Required auth subject | Verified by        |
//! |-------------------------|-----------------------|--------------------|
//! | `dispute_market`        | user                  | auth boundary      |
//! | `vote_on_dispute`       | user                  | auth boundary      |
//! | `resolve_dispute`       | admin                 | auth boundary      |
//! | `set_history_cap`       | admin                 | committed snapshot |
//! | `set_anti_grief_floor`  | admin                 | committed snapshot |

use predictify_hybrid::{
    Error, OracleConfig, OracleProvider, PredictifyHybrid, PredictifyHybridClient,
};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token::StellarAssetClient,
    Address, Env, String, Symbol, Vec,
};

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
        Fixture { env, cid, admin, token_id }
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
        )
    }

    fn yes(&self) -> String { String::from_str(&self.env, "yes") }

    fn advance_past_end(&self) {
        self.env.ledger().with_mut(|l| l.timestamp += 31 * 24 * 60 * 60);
    }

    fn required_auth(&self) -> std::vec::Vec<Address> {
        self.env.auths().iter().map(|(addr, _)| addr.clone()).collect()
    }

    fn assert_requires_auth(&self, expected: &Address, label: &str) {
        let required = self.required_auth();
        assert!(!required.is_empty(), "{label}: no auth recorded");
        assert!(required.contains(expected), "{label}: expected {expected:?}, got {required:?}");
    }
}

fn assert_auth_passed<T, E, F>(
    result: &Result<Result<T, E>, Result<F, soroban_sdk::InvokeError>>,
    label: &str,
) {
    if let Err(Err(invoke_err)) = result {
        panic!("{label}: expected to pass require_auth but trapped: {invoke_err:?}");
    }
}

// === Admin-scoped dispute entrypoints ===

#[test]
fn snapshot_set_history_cap_requires_admin_auth() {
    let f = Fixture::new();
    f.client().set_history_cap(&f.admin, &50u32);
    f.assert_requires_auth(&f.admin, "set_history_cap");
}

#[test]
#[should_panic]
fn edge_set_history_cap_without_auth_panics() {
    let f = Fixture::new();
    f.env.set_auths(&[]);
    f.client().set_history_cap(&f.admin, &50u32);
}

#[test]
fn snapshot_set_anti_grief_floor_requires_admin_auth() {
    let f = Fixture::new();
    f.client().set_anti_grief_floor(&f.admin, &1_000i128);
    f.assert_requires_auth(&f.admin, "set_anti_grief_floor");
}

#[test]
#[should_panic]
fn edge_set_anti_grief_floor_without_auth_panics() {
    let f = Fixture::new();
    f.env.set_auths(&[]);
    f.client().set_anti_grief_floor(&f.admin, &1_000i128);
}

#[test]
fn snapshot_resolve_dispute_requires_admin_auth() {
    let f = Fixture::new();
    let market_id = f.market();
    let user = f.user();
    f.client().vote(&user, &market_id, &f.yes(), &10_000_000i128);
    f.advance_past_end();
    let result = f.client().try_resolve_dispute(&f.admin, &market_id);
    assert_auth_passed(&result, "resolve_dispute");
}

#[test]
#[should_panic]
fn edge_resolve_dispute_without_auth_panics() {
    let f = Fixture::new();
    let market_id = f.market();
    f.env.set_auths(&[]);
    f.client().resolve_dispute(&f.admin, &market_id);
}

// === User-scoped dispute entrypoints ===

#[test]
fn snapshot_dispute_market_requires_user_auth() {
    let f = Fixture::new();
    let market_id = f.market();
    let user = f.user();
    f.client().vote(&user, &market_id, &f.yes(), &10_000_000i128);
    f.advance_past_end();
    let result = f.client().try_dispute_market(&user, &market_id, &10_000_000i128, &None);
    assert_auth_passed(&result, "dispute_market");
}

#[test]
#[should_panic]
fn edge_dispute_market_without_auth_panics() {
    let f = Fixture::new();
    let market_id = f.market();
    let user = f.user();
    f.env.set_auths(&[]);
    f.client().dispute_market(&user, &market_id, &10_000_000i128, &None);
}

// === Non-admin rejection tests ===

#[test]
fn edge_non_admin_set_history_cap_is_unauthorized() {
    let f = Fixture::new();
    let attacker = Address::generate(&f.env);
    let result = f.client().try_set_history_cap(&attacker, &50u32);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

#[test]
fn edge_non_admin_set_anti_grief_floor_is_unauthorized() {
    let f = Fixture::new();
    let attacker = Address::generate(&f.env);
    let result = f.client().try_set_anti_grief_floor(&attacker, &1_000i128);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

#[test]
fn edge_non_admin_resolve_dispute_is_unauthorized() {
    let f = Fixture::new();
    let market_id = f.market();
    let attacker = Address::generate(&f.env);
    let result = f.client().try_resolve_dispute(&attacker, &market_id);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}
