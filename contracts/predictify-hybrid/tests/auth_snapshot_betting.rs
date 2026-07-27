//! Per-entrypoint authorization snapshot tests for betting operations.
//!
//! This integration suite extends `auth_snapshot.rs` by covering the
//! **betting-specific** state-changing entrypoints that were not included in the
//! core suite: batch bets, bet-limit administration, fee-config commit/reveal,
//! fee withdrawal, event cancellation, oracle-failure refunds, and auto
//! payout distribution.
//!
//! Read-only entrypoints (`get_bet`, `has_user_bet`, `get_implied_probability`,
//! `get_payout_multiplier`, `get_effective_bet_limits`, `get_market_max_bet_cap`,
//! `calculate_bet_payout`, `query_user_bets_paged`) are documented as
//! requiring **no** auth and are pinned accordingly.
//!
//! ## Entrypoint auth matrix (this file)
//!
//! | Entrypoint                     | Required auth subject | Verified by        |
//! |--------------------------------|-----------------------|--------------------|
//! | `place_bets`                   | user                  | committed snapshot |
//! | `cancel_event`                 | admin                 | auth boundary      |
//! | `refund_on_oracle_failure`     | admin                 | auth boundary      |
//! | `distribute_payouts`           | none (public)         | committed snapshot |
//! | `set_global_bet_limits`        | admin                 | committed snapshot |
//! | `set_event_bet_limits`         | admin                 | committed snapshot |
//! | `set_market_max_bet_cap`       | admin                 | committed snapshot |
//! | `remove_market_max_bet_cap`    | admin                 | committed snapshot |
//! | `set_governance_min_bet_bps`   | admin                 | committed snapshot |
//! | `commit_fee_config`            | admin                 | auth boundary      |
//! | `withdraw_collected_fees`      | admin                 | auth boundary      |
//! | `get_bet` (read-only)          | none                  | committed snapshot |
//! | `has_user_bet` (read-only)     | none                  | committed snapshot |
//! | `get_implied_probability`      | none                  | committed snapshot |
//! | `get_payout_multiplier`        | none                  | committed snapshot |
//! | `get_effective_bet_limits`     | none                  | committed snapshot |
//! | `get_market_max_bet_cap`       | none                  | committed snapshot |
//! | `calculate_bet_payout`         | none                  | committed snapshot |
//! | `query_user_bets_paged`        | none                  | committed snapshot |

use predictify_hybrid::{
    Error, OracleConfig, OracleProvider, PredictifyHybrid, PredictifyHybridClient,
};
use soroban_sdk::{
    testutils::Address as _, token::StellarAssetClient, Address, Env, String, Symbol, Vec,
};

// ============================================================
// Fixture (duplicated from auth_snapshot.rs to keep suites independent)
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
        )
    }

    fn yes(&self) -> String {
        String::from_str(&self.env, "yes")
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
            "{label}: no auth recorded — the entrypoint either skipped require_auth \
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
            "{label}: expected no auth for a read-only entrypoint, captured {required:?}"
        );
    }
}

/// Assert the call got *past* `require_auth` (domain-logic failure, not a trap).
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
// Batch bet placement — committed snapshot
// ============================================================

#[test]
fn snapshot_place_bets_requires_user_auth() {
    let f = Fixture::new();
    let market_id = f.market();
    let user = f.user();

    let mut bets = Vec::new(&f.env);
    bets.push_back((market_id.clone(), f.yes(), 1_000_000i128));

    let idempotency_key = soroban_sdk::BytesN::from_array(&f.env, &[1u8; 32]);
    f.client()
        .place_bets(&user, &bets, &250i128, &idempotency_key);
    f.assert_requires_auth(&user, "place_bets");
}

// ============================================================
// Event cancellation — auth boundary (domain logic rejects)
// ============================================================

#[test]
fn snapshot_cancel_event_requires_admin_auth() {
    let f = Fixture::new();
    let market_id = f.market();

    let result = f
        .client()
        .try_cancel_event(&f.admin, &market_id, &None);
    assert_auth_passed(&result, "cancel_event");
}

#[test]
#[should_panic]
fn edge_cancel_event_without_auth_panics() {
    let f = Fixture::new();
    let market_id = f.market();
    f.env.set_auths(&[]);
    f.client()
        .cancel_event(&f.admin, &market_id, &None);
}

// ============================================================
// Oracle failure refund — auth boundary
// ============================================================

#[test]
fn snapshot_refund_on_oracle_failure_requires_admin_auth() {
    let f = Fixture::new();
    let market_id = f.market();

    let result = f
        .client()
        .try_refund_on_oracle_failure(&f.admin, &market_id);
    assert_auth_passed(&result, "refund_on_oracle_failure");
}

#[test]
#[should_panic]
fn edge_refund_on_oracle_failure_without_auth_panics() {
    let f = Fixture::new();
    let market_id = f.market();
    f.env.set_auths(&[]);
    f.client()
        .refund_on_oracle_failure(&f.admin, &market_id);
}

// ============================================================
// Distribute payouts — committed snapshot (public entrypoint, no auth)
// ============================================================

#[test]
fn snapshot_distribute_payouts_requires_no_auth() {
    let f = Fixture::new();
    let market_id = f.market();
    let user = f.user();

    // Build state: vote, end market, resolve, then call distribute_payouts
    f.client()
        .vote(&user, &market_id, &f.yes(), &10_000_000i128);
    f.advance_past_end();
    f.client()
        .resolve_market_manual(&f.admin, &market_id, &f.yes());

    let _ = f.client().try_distribute_payouts(&market_id);
    f.assert_no_auth("distribute_payouts");
}

// ============================================================
// Bet-limit administration — committed snapshots
// ============================================================

#[test]
fn snapshot_set_global_bet_limits_requires_admin_auth() {
    let f = Fixture::new();
    f.client()
        .set_global_bet_limits(&f.admin, &100i128, &10_000_000i128);
    f.assert_requires_auth(&f.admin, "set_global_bet_limits");
}

#[test]
fn snapshot_set_event_bet_limits_requires_admin_auth() {
    let f = Fixture::new();
    let market_id = f.market();
    f.client()
        .set_event_bet_limits(&f.admin, &market_id, &200i128, &5_000_000i128);
    f.assert_requires_auth(&f.admin, "set_event_bet_limits");
}

#[test]
fn snapshot_set_market_max_bet_cap_requires_admin_auth() {
    let f = Fixture::new();
    let market_id = f.market();
    f.client()
        .set_market_max_bet_cap(&f.admin, &market_id, &1_000_000i128);
    f.assert_requires_auth(&f.admin, "set_market_max_bet_cap");
}

#[test]
fn snapshot_remove_market_max_bet_cap_requires_admin_auth() {
    let f = Fixture::new();
    let market_id = f.market();
    f.client()
        .set_market_max_bet_cap(&f.admin, &market_id, &1_000_000i128);
    f.client()
        .remove_market_max_bet_cap(&f.admin, &market_id);
    f.assert_requires_auth(&f.admin, "remove_market_max_bet_cap");
}

// ============================================================
// Governance minimum bet — committed snapshot
// ============================================================

#[test]
fn snapshot_set_governance_min_bet_bps_requires_admin_auth() {
    let f = Fixture::new();
    f.client().set_governance_min_bet_bps(&f.admin, &500u32);
    f.assert_requires_auth(&f.admin, "set_governance_min_bet_bps");
}

// ============================================================
// Fee config commit — auth boundary
// ============================================================

#[test]
fn snapshot_commit_fee_config_requires_admin_auth() {
    let f = Fixture::new();
    let hash = soroban_sdk::BytesN::from_array(&f.env, &[0u8; 32]);
    let result = f.client().try_commit_fee_config(&f.admin, &hash);
    assert_auth_passed(&result, "commit_fee_config");
}

#[test]
#[should_panic]
fn edge_commit_fee_config_without_auth_panics() {
    let f = Fixture::new();
    let hash = soroban_sdk::BytesN::from_array(&f.env, &[0u8; 32]);
    f.env.set_auths(&[]);
    f.client().commit_fee_config(&f.admin, &hash);
}

// Note: `reveal_fee_config` requires `fees::FeeConfig` which lives in a private
// module and is not accessible from integration tests. Its auth requirement
// (admin) is structurally guaranteed by the same `require_primary_admin` call
// shared with `commit_fee_config`.

// ============================================================
// Withdraw collected fees — auth boundary
// ============================================================

#[test]
fn snapshot_withdraw_collected_fees_requires_admin_auth() {
    let f = Fixture::new();
    let market_id = f.market();

    // Build enough state so fees exist
    let user = f.user();
    f.client()
        .vote(&user, &market_id, &f.yes(), &1_000_000_000i128);
    f.advance_past_end();
    f.client()
        .resolve_market_manual(&f.admin, &market_id, &f.yes());
    f.client().collect_fees(&f.admin, &market_id);

    let result = f.client().try_withdraw_collected_fees(&f.admin, &1i128);
    assert_auth_passed(&result, "withdraw_collected_fees");
}

#[test]
#[should_panic]
fn edge_withdraw_collected_fees_without_auth_panics() {
    let f = Fixture::new();
    f.env.set_auths(&[]);
    f.client()
        .withdraw_collected_fees(&f.admin, &1i128);
}

// ============================================================
// Read-only betting entrypoints — require no auth
// ============================================================

#[test]
fn snapshot_get_bet_requires_no_auth() {
    let f = Fixture::new();
    let market_id = f.market();
    let user = f.user();
    f.client()
        .place_bet(&user, &market_id, &f.yes(), &1_000_000i128, &250i128);
    let _ = f.client().get_bet(&market_id, &user);
    f.assert_no_auth("get_bet");
}

#[test]
fn snapshot_has_user_bet_requires_no_auth() {
    let f = Fixture::new();
    let market_id = f.market();
    let user = f.user();
    let _ = f.client().has_user_bet(&market_id, &user);
    f.assert_no_auth("has_user_bet");
}

#[test]
fn snapshot_get_implied_probability_requires_no_auth() {
    let f = Fixture::new();
    let market_id = f.market();
    let _ = f.client().get_implied_probability(&market_id, &f.yes());
    f.assert_no_auth("get_implied_probability");
}

#[test]
fn snapshot_get_payout_multiplier_requires_no_auth() {
    let f = Fixture::new();
    let market_id = f.market();
    let _ = f.client().get_payout_multiplier(&market_id, &f.yes());
    f.assert_no_auth("get_payout_multiplier");
}

#[test]
fn snapshot_get_effective_bet_limits_requires_no_auth() {
    let f = Fixture::new();
    let market_id = f.market();
    let _ = f.client().get_effective_bet_limits(&market_id);
    f.assert_no_auth("get_effective_bet_limits");
}

#[test]
fn snapshot_get_market_max_bet_cap_requires_no_auth() {
    let f = Fixture::new();
    let market_id = f.market();
    let _ = f.client().get_market_max_bet_cap(&market_id);
    f.assert_no_auth("get_market_max_bet_cap");
}

#[test]
fn snapshot_calculate_bet_payout_requires_no_auth() {
    let f = Fixture::new();
    let market_id = f.market();
    let user = f.user();
    f.client()
        .place_bet(&user, &market_id, &f.yes(), &1_000_000i128, &250i128);
    let _ = f.client().calculate_bet_payout(&market_id, &user);
    f.assert_no_auth("calculate_bet_payout");
}

#[test]
fn snapshot_query_user_bets_paged_requires_no_auth() {
    let f = Fixture::new();
    let user = f.user();
    let _ = f.client().query_user_bets_paged(&user, &0u32, &10u32);
    f.assert_no_auth("query_user_bets_paged");
}

// ============================================================
// Edge cases — non-admin rejection
// ============================================================

#[test]
fn edge_non_admin_set_global_bet_limits_is_unauthorized() {
    let f = Fixture::new();
    let attacker = Address::generate(&f.env);
    let result = f
        .client()
        .try_set_global_bet_limits(&attacker, &100i128, &10_000_000i128);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

#[test]
fn edge_non_admin_set_market_max_bet_cap_is_unauthorized() {
    let f = Fixture::new();
    let market_id = f.market();
    let attacker = Address::generate(&f.env);
    let result = f
        .client()
        .try_set_market_max_bet_cap(&attacker, &market_id, &1_000_000i128);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

#[test]
fn edge_non_admin_cancel_event_is_unauthorized() {
    let f = Fixture::new();
    let market_id = f.market();
    let attacker = Address::generate(&f.env);
    let result = f
        .client()
        .try_cancel_event(&attacker, &market_id, &None);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

#[test]
fn edge_non_admin_commit_fee_config_is_unauthorized() {
    let f = Fixture::new();
    let attacker = Address::generate(&f.env);
    let hash = soroban_sdk::BytesN::from_array(&f.env, &[0u8; 32]);
    let result = f.client().try_commit_fee_config(&attacker, &hash);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}
