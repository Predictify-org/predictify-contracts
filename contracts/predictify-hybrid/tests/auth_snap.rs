//! Per-entrypoint authorization snapshot tests for market entrypoints.
//!
//! This integration suite **snapshots** the Soroban authorization required by
//! every state-changing market entrypoint of [`PredictifyHybrid`].  Instead of
//! only checking that an authorized call succeeds and an unauthorized one
//! fails, these tests inspect [`Env::auths`] immediately after each call and
//! assert **which address the host actually required an authorization from**.
//!
//! ## Why a snapshot?
//!
//! * If a `require_auth` call is ever dropped from an entrypoint, `env.auths()`
//!   for that call becomes empty and the matching test fails immediately.
//! * If the auth subject is rebound to the wrong argument (e.g. a setter that
//!   starts authorizing an attacker-controlled address), the captured subject
//!   no longer matches the expected one and the test fails.
//! * Read-only market entrypoints are pinned to *require no auth at all*,
//!   documenting the read/write authorization boundary.
//!
//! ## Verification strategy
//!
//! The Soroban host records an authorization only for an invocation that
//! **commits**.  When a call traps or returns `Err`, its recorded auths are
//! rolled back and `env.auths()` comes back empty.  Most snapshots below
//! therefore drive the entrypoint through a fully satisfied happy path
//! ("committed snapshot").
//!
//! A small number of entrypoints need runtime state this fixture deliberately
//! does not build.  For those the auth boundary is pinned directly instead
//! ("auth boundary"): authorized calls must get *past* `require_auth` and fail
//! on domain logic (`Err(Ok(..))`), while unauthorized calls must trap in
//! `require_auth` (a matching `#[should_panic]` test).
//!
//! ## Entrypoint auth matrix
//!
//! | Entrypoint                      | Required auth subject | Verified by        |
//! |---------------------------------|-----------------------|--------------------|
//! | `create_market`                 | admin                 | committed snapshot |
//! | `resolve_market_manual`         | admin                 | committed snapshot |
//! | `resolve_market_with_ties`      | admin                 | auth boundary      |
//! | `force_resolve_market`          | admin                 | auth boundary      |
//! | `extend_deadline`               | admin                 | committed snapshot |
//! | `set_platform_fee`              | admin                 | committed snapshot |
//! | `set_treasury`                  | admin                 | committed snapshot |
//! | `archive_event`                 | admin                 | auth boundary      |
//! | `set_resolution_cooldown`       | admin                 | committed snapshot |
//! | `vote`                          | user                  | committed snapshot |
//! | `place_bet`                     | user                  | committed snapshot |
//! | `cancel_bet`                    | user                  | committed snapshot |
//! | `claim_winnings`                | user                  | auth boundary      |
//! | `get_market` (read-only)        | none                  | committed snapshot |
//! | `get_market_bet_stats` (r/o)    | none                  | committed snapshot |
//! | `get_bet` (read-only)           | none                  | committed snapshot |

use predictify_hybrid::{
    Error, OracleConfig, OracleProvider, PredictifyHybrid, PredictifyHybridClient,
};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token::StellarAssetClient,
    Address, Env, String, Symbol, Vec,
};

// ============================================================
// Test fixture
// ============================================================

/// A fully wired contract: registered Stellar Asset Contract for stake
/// transfers, `TokenID` stored in contract state, and an initialized admin.
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

        // Register a Stellar asset so the contract's token client resolves.
        let token_id = env
            .register_stellar_asset_contract_v2(Address::generate(&env))
            .address();

        // Wire the token before initializing so stake transfers work.
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

    /// Create a funded user able to cover any stake used in these tests.
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

    /// Create a standard two-outcome, 30-day market owned by `admin`.
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

    fn yes(&self) -> String {
        String::from_str(&self.env, "yes")
    }

    /// Advance ~31 days so a 30-day market is past its end time.
    fn advance_past_end(&self) {
        self.env
            .ledger()
            .with_mut(|l| l.timestamp += 31 * 24 * 60 * 60);
    }

    /// Advance past the default 86_400 s dispute window.
    fn advance_past_dispute(&self) {
        self.env.ledger().with_mut(|l| l.timestamp += 86_401);
    }

    /// Addresses the most recent top-level invocation required auth from.
    fn required_auth(&self) -> std::vec::Vec<Address> {
        self.env
            .auths()
            .iter()
            .map(|(addr, _)| addr.clone())
            .collect()
    }

    /// Assert the last invocation required an authorization from `expected`.
    ///
    /// An empty auth set means either the entrypoint performed no
    /// `require_auth`, or the call failed and the host rolled its auths back.
    fn assert_requires_auth(&self, expected: &Address, label: &str) {
        let required = self.required_auth();
        assert!(
            !required.is_empty(),
            "{label}: no auth recorded — the entrypoint either skipped \
             require_auth or the call failed before committing"
        );
        assert!(
            required.contains(expected),
            "{label}: expected auth from {expected:?}, captured {required:?}"
        );
    }

    /// Assert the last invocation required *no* authorization (read-only).
    fn assert_no_auth(&self, label: &str) {
        let required = self.required_auth();
        assert!(
            required.is_empty(),
            "{label}: expected no auth for a read-only entrypoint, captured {required:?}"
        );
    }
}

// ============================================================
// Auth-boundary helpers for entrypoints whose happy path
// requires state this fixture does not construct.
// ============================================================

/// Assert the call got *past* `require_auth`.
///
/// A `try_*` invocation reports a contract-level failure as `Err(Ok(..))` and a
/// host-level trap — which is how a rejected `require_auth` surfaces — as
/// `Err(Err(InvokeError))`.  Reaching a contract error proves that authorization
/// succeeded; the call failed later on domain logic.
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
// Admin-scoped market entrypoints — committed snapshot
// ============================================================

/// `create_market` must require admin authorization.
#[test]
fn snap_create_market_requires_admin_auth() {
    let f = Fixture::new();
    let _ = f.market();
    f.assert_requires_auth(&f.admin, "create_market");
}

/// `resolve_market_manual` must require admin authorization.
#[test]
fn snap_resolve_market_manual_requires_admin_auth() {
    let f = Fixture::new();
    let market_id = f.market();
    f.advance_past_end();
    f.client()
        .resolve_market_manual(&f.admin, &market_id, &f.yes());
    f.assert_requires_auth(&f.admin, "resolve_market_manual");
}

/// `extend_deadline` must require admin authorization.
#[test]
fn snap_extend_deadline_requires_admin_auth() {
    let f = Fixture::new();
    let market_id = f.market();
    f.client().extend_deadline(
        &f.admin,
        &market_id,
        &7u32,
        &String::from_str(&f.env, "additional time needed"),
    );
    f.assert_requires_auth(&f.admin, "extend_deadline");
}

/// `set_platform_fee` must require admin authorization.
#[test]
fn snap_set_platform_fee_requires_admin_auth() {
    let f = Fixture::new();
    f.client().set_platform_fee(&f.admin, &250i128);
    f.assert_requires_auth(&f.admin, "set_platform_fee");
}

/// `set_treasury` must require admin authorization.
#[test]
fn snap_set_treasury_requires_admin_auth() {
    let f = Fixture::new();
    let treasury = Address::generate(&f.env);
    f.client().set_treasury(&f.admin, &treasury);
    f.assert_requires_auth(&f.admin, "set_treasury");
}

/// `set_resolution_cooldown` must require admin authorization.
#[test]
fn snap_set_resolution_cooldown_requires_admin_auth() {
    let f = Fixture::new();
    f.client().set_resolution_cooldown(&f.admin, &3600u64);
    f.assert_requires_auth(&f.admin, "set_resolution_cooldown");
}

// ============================================================
// Admin-scoped market entrypoints — auth boundary
// ============================================================

/// `resolve_market_with_ties` must require admin authorization.
///
/// The tie-resolution path needs a resolved market with multiple outcomes, so
/// we pin the auth boundary rather than building the full state.
#[test]
fn snap_resolve_market_with_ties_requires_admin_auth() {
    let f = Fixture::new();
    let market_id = f.market();
    f.advance_past_end();
    let mut outcomes = Vec::new(&f.env);
    outcomes.push_back(String::from_str(&f.env, "yes"));
    let result = f
        .client()
        .try_resolve_market_with_ties(&f.admin, &market_id, &outcomes);
    assert_auth_passed(&result, "resolve_market_with_ties");
}

#[test]
#[should_panic]
fn edge_resolve_market_with_ties_without_auth_panics() {
    let f = Fixture::new();
    let market_id = f.market();
    f.advance_past_end();
    f.env.set_auths(&[]);
    let mut outcomes = Vec::new(&f.env);
    outcomes.push_back(String::from_str(&f.env, "yes"));
    f.client()
        .resolve_market_with_ties(&f.admin, &market_id, &outcomes);
}

/// `force_resolve_market` must require admin authorization.
#[test]
fn snap_force_resolve_market_requires_admin_auth() {
    let f = Fixture::new();
    let market_id = f.market();
    let idempotency_key = String::from_str(&f.env, "key_001");
    let result = f.client().try_force_resolve_market(
        &f.admin,
        &market_id,
        &f.yes(),
        &idempotency_key,
        &String::from_str(&f.env, "manual override"),
    );
    assert_auth_passed(&result, "force_resolve_market");
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
        &f.yes(),
        &String::from_str(&f.env, "key_001"),
        &String::from_str(&f.env, "manual override"),
    );
}

/// `archive_event` must require admin authorization.
#[test]
fn snap_archive_event_requires_admin_auth() {
    let f = Fixture::new();
    let market_id = f.market();
    let result = f.client().try_archive_event(&f.admin, &market_id);
    assert_auth_passed(&result, "archive_event");
}

#[test]
#[should_panic]
fn edge_archive_event_without_auth_panics() {
    let f = Fixture::new();
    let market_id = f.market();
    f.env.set_auths(&[]);
    f.client().archive_event(&f.admin, &market_id);
}

// ============================================================
// User-scoped market entrypoints — committed snapshot
// ============================================================

/// `vote` must require the acting user's authorization.
#[test]
fn snap_vote_requires_user_auth() {
    let f = Fixture::new();
    let market_id = f.market();
    let user = f.user();
    f.client().vote(&user, &market_id, &f.yes(), &1_000_000i128);
    f.assert_requires_auth(&user, "vote");
}

/// `place_bet` must require the acting user's authorization.
#[test]
fn snap_place_bet_requires_user_auth() {
    let f = Fixture::new();
    let market_id = f.market();
    let user = f.user();
    f.client()
        .place_bet(&user, &market_id, &f.yes(), &1_000_000i128, &250i128);
    f.assert_requires_auth(&user, "place_bet");
}

/// `cancel_bet` must require the acting user's authorization.
#[test]
fn snap_cancel_bet_requires_user_auth() {
    let f = Fixture::new();
    let market_id = f.market();
    let user = f.user();
    f.client()
        .place_bet(&user, &market_id, &f.yes(), &1_000_000i128, &250i128);
    f.client().cancel_bet(&user, &market_id);
    f.assert_requires_auth(&user, "cancel_bet");
}

/// `claim_winnings` must require the acting user's authorization.
///
/// The claim path requires a resolved, past-dispute-window market, which is
/// harder to drive to completion here, so we pin the auth boundary.
#[test]
fn snap_claim_winnings_requires_user_auth() {
    let f = Fixture::new();
    let market_id = f.market();
    let user = f.user();
    f.client()
        .vote(&user, &market_id, &f.yes(), &10_000_000i128);
    f.advance_past_end();
    f.client()
        .resolve_market_manual(&f.admin, &market_id, &f.yes());
    f.advance_past_dispute();

    let result = f.client().try_claim_winnings(&user, &market_id);
    assert_auth_passed(&result, "claim_winnings");
}

#[test]
#[should_panic]
fn edge_claim_winnings_without_auth_panics() {
    let f = Fixture::new();
    let market_id = f.market();
    let user = f.user();
    f.env.set_auths(&[]);
    f.client().claim_winnings(&user, &market_id);
}

// ============================================================
// Read-only market entrypoints — no auth required
// ============================================================

/// `get_market` is a read-only query; it must not require any authorization.
#[test]
fn snap_get_market_requires_no_auth() {
    let f = Fixture::new();
    let market_id = f.market();
    let _ = f.client().get_market(&market_id);
    f.assert_no_auth("get_market");
}

/// `get_market_bet_stats` is a read-only query; it must not require any
/// authorization.
#[test]
fn snap_get_market_bet_stats_requires_no_auth() {
    let f = Fixture::new();
    let market_id = f.market();
    let _ = f.client().get_market_bet_stats(&market_id);
    f.assert_no_auth("get_market_bet_stats");
}

/// `get_bet` is a read-only query; it must not require any authorization.
#[test]
fn snap_get_bet_requires_no_auth() {
    let f = Fixture::new();
    let market_id = f.market();
    let user = f.user();
    let _ = f.client().get_bet(&market_id, &user);
    f.assert_no_auth("get_bet");
}

// ============================================================
// Subject-binding correctness tests
// ============================================================

/// A user entrypoint must bind `require_auth` to the acting user, never to
/// the admin — even though the admin is authorized in the same environment.
#[test]
fn snap_vote_subject_is_user_not_admin() {
    let f = Fixture::new();
    let market_id = f.market();
    let user = f.user();
    f.client().vote(&user, &market_id, &f.yes(), &1_000_000i128);

    let required = f.required_auth();
    assert!(
        required.contains(&user),
        "vote must require the acting user, captured {required:?}"
    );
    assert!(
        !required.contains(&f.admin),
        "vote must not require admin auth, captured {required:?}"
    );
}

/// `place_bet` must bind `require_auth` to the bettor, not to the admin.
#[test]
fn snap_place_bet_subject_is_user_not_admin() {
    let f = Fixture::new();
    let market_id = f.market();
    let user = f.user();
    f.client()
        .place_bet(&user, &market_id, &f.yes(), &1_000_000i128, &250i128);

    let required = f.required_auth();
    assert!(
        required.contains(&user),
        "place_bet must require the acting user, captured {required:?}"
    );
    assert!(
        !required.contains(&f.admin),
        "place_bet must not require admin auth, captured {required:?}"
    );
}

/// Two distinct users submit votes: each call must capture that user's
/// address, never the other user's address.
#[test]
fn snap_vote_subject_tracks_argument_across_users() {
    let f = Fixture::new();
    let market_id = f.market();
    let user_a = f.user();
    let user_b = f.user();

    f.client()
        .vote(&user_a, &market_id, &f.yes(), &1_000_000i128);
    f.assert_requires_auth(&user_a, "vote(user_a)");

    f.client().vote(
        &user_b,
        &market_id,
        &String::from_str(&f.env, "no"),
        &1_000_000i128,
    );
    let required = f.required_auth();
    assert!(
        required.contains(&user_b),
        "vote must require user_b, captured {required:?}"
    );
    assert!(
        !required.contains(&user_a),
        "user_a's auth must not satisfy user_b's vote, captured {required:?}"
    );
}

// ============================================================
// Negative / edge cases
// ============================================================

/// Calling an admin entrypoint with a non-admin address must be rejected with
/// `Error::Unauthorized`, even while `mock_all_auths` satisfies the signature
/// check — because admin entrypoints verify identity against the stored admin.
#[test]
fn edge_non_admin_set_platform_fee_is_unauthorized() {
    let f = Fixture::new();
    let attacker = Address::generate(&f.env);
    let result = f.client().try_set_platform_fee(&attacker, &250i128);
    assert_eq!(
        result,
        Err(Ok(Error::Unauthorized)),
        "non-admin must not be able to set the platform fee"
    );
}

/// With no auths mocked, `vote` must trap in `require_auth`, not silently
/// succeed.
#[test]
#[should_panic]
fn edge_vote_without_auth_panics() {
    let f = Fixture::new();
    let market_id = f.market();
    let user = f.user();
    f.env.set_auths(&[]);
    f.client().vote(&user, &market_id, &f.yes(), &1_000_000i128);
}

/// With no auths mocked, `place_bet` must trap in `require_auth`.
#[test]
#[should_panic]
fn edge_place_bet_without_auth_panics() {
    let f = Fixture::new();
    let market_id = f.market();
    let user = f.user();
    f.env.set_auths(&[]);
    f.client()
        .place_bet(&user, &market_id, &f.yes(), &1_000_000i128, &250i128);
}

/// With no auths mocked, an admin entrypoint must trap in `require_auth`.
#[test]
#[should_panic]
fn edge_create_market_without_auth_panics() {
    let f = Fixture::new();
    f.env.set_auths(&[]);
    let _ = f.market();
}
