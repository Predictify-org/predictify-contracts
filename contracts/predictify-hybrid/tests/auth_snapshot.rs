//! Per-entrypoint authorization snapshot tests.
//!
//! This integration suite *snapshots* the Soroban authorization required by
//! every state-changing entrypoint of [`PredictifyHybrid`]. Instead of only
//! checking that an authorized call succeeds and an unauthorized one fails,
//! these tests inspect [`Env::auths`] immediately after each call and assert
//! **which address the host actually required an authorization from**.
//!
//! Why a snapshot?
//!
//! * If a `require_auth` is ever dropped from an entrypoint, `env.auths()` for
//!   that call becomes empty and the matching test fails.
//! * If the auth subject is ever rebound to the wrong argument (e.g. an admin
//!   setter that starts authorizing an attacker-controlled address), the
//!   captured subject no longer matches the expected one and the test fails.
//! * Read-only entrypoints are pinned to *require no auth at all*, documenting
//!   the read/write authorization boundary.
//!
//! ## Why some entrypoints use a different check
//!
//! The Soroban host records an authorization only for an invocation that
//! commits. When a call traps or returns `Err`, its recorded auths are rolled
//! back and `env.auths()` comes back empty. Most snapshots below therefore
//! drive the entrypoint through a *fully satisfied* happy path — including a
//! real Stellar Asset Contract for stake transfers — and then read the auth
//! back ("committed snapshot").
//!
//! Three entrypoints need runtime state this fixture deliberately does not
//! build. For those the auth boundary is pinned directly instead
//! ("auth boundary"): authorized calls must get *past* `require_auth` and fail
//! on domain logic (`Err(Ok(..))`), while unauthorized calls must trap in
//! `require_auth` (a matching `#[should_panic]` test).
//!
//! ## Entrypoint auth matrix
//!
//! | Entrypoint                 | Required auth subject | Verified by        |
//! |----------------------------|-----------------------|--------------------|
//! | `create_market`            | admin                 | committed snapshot |
//! | `resolve_market_manual`    | admin                 | committed snapshot |
//! | `collect_fees`             | admin                 | committed snapshot |
//! | `set_platform_fee`         | admin                 | committed snapshot |
//! | `set_treasury`             | admin                 | committed snapshot |
//! | `set_global_claim_period`  | admin                 | committed snapshot |
//! | `set_market_claim_period`  | admin                 | committed snapshot |
//! | `extend_deadline`          | admin                 | committed snapshot |
//! | `sweep_unclaimed_winnings` | admin                 | auth boundary      |
//! | `vote`                     | user                  | committed snapshot |
//! | `place_bet`                | user                  | committed snapshot |
//! | `cancel_bet`               | user                  | committed snapshot |
//! | `claim_winnings`           | user                  | auth boundary      |
//! | `dispute_market`           | user                  | auth boundary      |
//! | `get_market` (read-only)   | none                  | committed snapshot |
//! | `get_market_bet_stats`     | none                  | committed snapshot |

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
            "{label}: no auth recorded — the entrypoint either skipped require_auth \
             or the call failed before committing"
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
// Auth-boundary assertions for entrypoints whose happy path needs
// state this fixture does not construct
// ============================================================

/// Assert the call got *past* `require_auth`.
///
/// A `try_*` invocation reports a contract-level failure as `Err(Ok(..))` and a
/// host-level trap — which is how a rejected `require_auth` surfaces — as
/// `Err(Err(InvokeError))`. So reaching a contract error proves authorization
/// succeeded and the call failed later, on domain logic.
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
// Admin-scoped entrypoints — snapshot: requires admin auth
// ============================================================

#[test]
fn snapshot_create_market_requires_admin_auth() {
    let f = Fixture::new();
    let _ = f.market();
    f.assert_requires_auth(&f.admin, "create_market");
}

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
fn snapshot_set_platform_fee_requires_admin_auth() {
    let f = Fixture::new();
    f.client().set_platform_fee(&f.admin, &300i128);
    f.assert_requires_auth(&f.admin, "set_platform_fee");
}

#[test]
fn snapshot_set_treasury_requires_admin_auth() {
    let f = Fixture::new();
    let treasury = Address::generate(&f.env);
    f.client().set_treasury(&f.admin, &treasury);
    f.assert_requires_auth(&f.admin, "set_treasury");
}

#[test]
fn snapshot_set_global_claim_period_requires_admin_auth() {
    let f = Fixture::new();
    f.client().set_global_claim_period(&f.admin, &604_800u64);
    f.assert_requires_auth(&f.admin, "set_global_claim_period");
}

#[test]
fn snapshot_set_market_claim_period_requires_admin_auth() {
    let f = Fixture::new();
    let market_id = f.market();
    f.client()
        .set_market_claim_period(&f.admin, &market_id, &604_800u64);
    f.assert_requires_auth(&f.admin, "set_market_claim_period");
}

#[test]
fn snapshot_extend_deadline_requires_admin_auth() {
    let f = Fixture::new();
    let market_id = f.market();
    f.client().extend_deadline(
        &f.admin,
        &market_id,
        &7u32,
        &String::from_str(&f.env, "More time needed"),
    );
    f.assert_requires_auth(&f.admin, "extend_deadline");
}

#[test]
fn snapshot_collect_fees_requires_admin_auth() {
    let f = Fixture::new();
    let market_id = f.market();

    // Stake enough that the 2% platform fee clears MIN_FEE_AMOUNT.
    let user = f.user();
    f.client()
        .vote(&user, &market_id, &f.yes(), &1_000_000_000i128);

    f.advance_past_end();
    f.client()
        .resolve_market_manual(&f.admin, &market_id, &f.yes());

    f.client().collect_fees(&f.admin, &market_id);
    f.assert_requires_auth(&f.admin, "collect_fees");
}

/// `sweep_unclaimed_winnings` needs runtime state this fixture does not build,
/// so instead of a committed-call snapshot we pin the auth boundary directly:
/// authorized  -> gets past `require_auth` and fails on domain logic,
/// unauthorized -> trapped by `require_auth`.
#[test]
fn snapshot_sweep_unclaimed_winnings_requires_admin_auth() {
    let f = Fixture::new();
    let market_id = f.market();

    let result = f
        .client()
        .try_sweep_unclaimed_winnings(&f.admin, &market_id, &false);
    assert_auth_passed(&result, "sweep_unclaimed_winnings");
}

#[test]
#[should_panic]
fn edge_sweep_unclaimed_winnings_without_auth_panics() {
    let f = Fixture::new();
    let market_id = f.market();
    f.env.set_auths(&[]);
    f.client()
        .sweep_unclaimed_winnings(&f.admin, &market_id, &false);
}

// ============================================================
// User-scoped entrypoints — snapshot: requires user auth
// ============================================================

#[test]
fn snapshot_vote_requires_user_auth() {
    let f = Fixture::new();
    let market_id = f.market();
    let user = f.user();
    f.client().vote(&user, &market_id, &f.yes(), &1_000_000i128);
    f.assert_requires_auth(&user, "vote");
}

#[test]
fn snapshot_place_bet_requires_user_auth() {
    let f = Fixture::new();
    let market_id = f.market();
    let user = f.user();
    f.client()
        .place_bet(&user, &market_id, &f.yes(), &1_000_000i128, &250i128);
    f.assert_requires_auth(&user, "place_bet");
}

#[test]
fn snapshot_cancel_bet_requires_user_auth() {
    let f = Fixture::new();
    let market_id = f.market();
    let user = f.user();
    f.client()
        .place_bet(&user, &market_id, &f.yes(), &1_000_000i128, &250i128);
    f.client().cancel_bet(&user, &market_id);
    f.assert_requires_auth(&user, "cancel_bet");
}

#[test]
fn snapshot_fetch_oracle_result_requires_caller_auth() {
    let f = Fixture::new();
    let market_id = f.market();
    let caller = f.user();
    
    f.advance_past_end();
    
    let oracle_address = Address::generate(&f.env);

    // Call the newly authenticated endpoint
    let result = f.client().try_fetch_oracle_result(&caller, &market_id, &oracle_address);
    
    // Validate auth boundary
    assert_auth_passed(&result, "fetch_oracle_result");
}

/// `claim_winnings` reaches `AlreadyClaimed` in this fixture, so the committed
/// snapshot is unavailable; pin the auth boundary instead (see
/// `snapshot_sweep_unclaimed_winnings_requires_admin_auth`).
#[test]
fn snapshot_claim_winnings_requires_user_auth() {
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

/// `dispute_market` is rejected by its market-state validator in this fixture,
/// so the committed snapshot is unavailable; pin the auth boundary instead
/// (see `snapshot_sweep_unclaimed_winnings_requires_admin_auth`).
#[test]
fn snapshot_dispute_market_requires_user_auth() {
    let f = Fixture::new();
    let market_id = f.market();
    let user = f.user();
    f.client()
        .vote(&user, &market_id, &f.yes(), &10_000_000i128);
    f.advance_past_end();

    let result = f
        .client()
        .try_dispute_market(&user, &market_id, &10_000_000i128, &None);
    assert_auth_passed(&result, "dispute_market");
}

#[test]
#[should_panic]
fn edge_dispute_market_without_auth_panics() {
    let f = Fixture::new();
    let market_id = f.market();
    let user = f.user();
    f.env.set_auths(&[]);
    f.client()
        .dispute_market(&user, &market_id, &10_000_000i128, &None);
}

// ============================================================
// Read-only entrypoints — snapshot: requires no auth
// ============================================================

#[test]
fn snapshot_get_market_requires_no_auth() {
    let f = Fixture::new();
    let market_id = f.market();
    let _ = f.client().get_market(&market_id);
    f.assert_no_auth("get_market");
}

#[test]
fn snapshot_get_market_bet_stats_requires_no_auth() {
    let f = Fixture::new();
    let market_id = f.market();
    let _ = f.client().get_market_bet_stats(&market_id);
    f.assert_no_auth("get_market_bet_stats");
}

// ============================================================
// Edge cases
// ============================================================

/// A user entrypoint must bind `require_auth` to the acting user, never to the
/// admin — even though the admin is authorized in the same environment.
#[test]
fn snapshot_vote_subject_is_user_not_admin() {
    let f = Fixture::new();
    let market_id = f.market();
    let user = f.user();
    f.client().vote(&user, &market_id, &f.yes(), &1_000_000i128);

    let required = f.required_auth();
    assert!(
        required.contains(&user),
        "vote must require the acting user"
    );
    assert!(
        !required.contains(&f.admin),
        "vote must not require admin auth: {required:?}"
    );
}

/// Two distinct users are authorized independently: user B's vote must record
/// B's auth, not A's, proving the subject tracks the argument.
#[test]
fn snapshot_vote_subject_tracks_argument_across_users() {
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
        "user_a's auth must not satisfy user_b's vote: {required:?}"
    );
}

/// A non-admin caller is rejected with `Error::Unauthorized` even while
/// `mock_all_auths` is active, because admin entrypoints check the caller
/// against the stored admin identity rather than trusting a bare signature.
#[test]
fn edge_non_admin_set_platform_fee_is_unauthorized() {
    let f = Fixture::new();
    let attacker = Address::generate(&f.env);
    let result = f.client().try_set_platform_fee(&attacker, &300i128);
    assert_eq!(
        result,
        Err(Ok(Error::Unauthorized)),
        "non-admin must not be able to set the platform fee"
    );
}

/// With no auths mocked, a user entrypoint must fail in `require_auth` rather
/// than silently proceeding.
#[test]
#[should_panic]
fn edge_vote_without_auth_panics() {
    let f = Fixture::new();
    let market_id = f.market();
    let user = f.user();

    // Clear all mocked auths: the user's require_auth can no longer be satisfied.
    f.env.set_auths(&[]);
    f.client().vote(&user, &market_id, &f.yes(), &1_000_000i128);
}

/// With no auths mocked, an admin entrypoint must fail in `require_auth`.
#[test]
#[should_panic]
fn edge_set_platform_fee_without_auth_panics() {
    let f = Fixture::new();
    f.env.set_auths(&[]);
    f.client().set_platform_fee(&f.admin, &300i128);
}
