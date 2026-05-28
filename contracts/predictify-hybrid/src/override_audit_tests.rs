#![cfg(test)]

use crate::audit_trail::{AuditAction, AuditTrailManager};
use crate::errors::Error;
use crate::types::{MarketState, OracleConfig, OracleProvider};
use crate::{PredictifyHybrid, PredictifyHybridClient};
use soroban_sdk::{
    testutils::Address as _, vec, Address, Env, String, Symbol,
};

// ── shared setup ─────────────────────────────────────────────────────────────

struct Ctx {
    env: Env,
    contract_id: Address,
    admin: Address,
}

impl Ctx {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let contract_id = env.register(PredictifyHybrid, ());
        PredictifyHybridClient::new(&env, &contract_id).initialize(&admin, &None, &None);
        Self { env, contract_id, admin }
    }

    fn client(&self) -> PredictifyHybridClient<'_> {
        PredictifyHybridClient::new(&self.env, &self.contract_id)
    }

    /// Creates a minimal market and returns its id.
    fn create_market(&self) -> Symbol {
        self.client().create_market(
            &self.admin,
            &String::from_str(&self.env, "Will BTC exceed $100k?"),
            &vec![
                &self.env,
                String::from_str(&self.env, "yes"),
                String::from_str(&self.env, "no"),
            ],
            &30u32,
            &OracleConfig {
                provider: OracleProvider::reflector(),
                oracle_address: Address::generate(&self.env),
                feed_id: String::from_str(&self.env, "BTC"),
                threshold: 100_000_00,
                comparison: String::from_str(&self.env, "gt"),
            },
            &None,
            &0u64,
            &None,
            &None,
            &None,
        )
    }
}

// ── empty reason is rejected ──────────────────────────────────────────────────

#[test]
fn test_override_rejects_empty_reason() {
    let ctx = Ctx::new();
    let market_id = ctx.create_market();

    let result = ctx.client().try_admin_override_verification(
        &ctx.admin,
        &market_id,
        &String::from_str(&ctx.env, "yes"),
        &String::from_str(&ctx.env, ""),
    );

    assert_eq!(result, Err(Ok(Error::InvalidInput)));
}

// ── successful override writes audit record ───────────────────────────────────

#[test]
fn test_override_appends_audit_record() {
    let ctx = Ctx::new();
    let market_id = ctx.create_market();

    ctx.client().admin_override_verification(
        &ctx.admin,
        &market_id,
        &String::from_str(&ctx.env, "yes"),
        &String::from_str(&ctx.env, "Oracle feed was stale; manual data confirmed"),
    );

    ctx.env.as_contract(&ctx.contract_id, || {
        let head = AuditTrailManager::get_head(&ctx.env).unwrap();
        assert!(head.latest_index >= 1);

        let record = AuditTrailManager::get_record(&ctx.env, head.latest_index).unwrap();
        assert_eq!(record.action, AuditAction::OracleVerificationOverride);
        assert_eq!(record.actor, ctx.admin);

        let recorded_reason = record
            .details
            .get(Symbol::new(&ctx.env, "reason"))
            .unwrap();
        assert_eq!(
            recorded_reason,
            String::from_str(&ctx.env, "Oracle feed was stale; manual data confirmed")
        );
    });
}

// ── audit chain integrity holds after override ────────────────────────────────

#[test]
fn test_override_preserves_audit_integrity() {
    let ctx = Ctx::new();
    let market_id = ctx.create_market();

    ctx.client().admin_override_verification(
        &ctx.admin,
        &market_id,
        &String::from_str(&ctx.env, "no"),
        &String::from_str(&ctx.env, "Community consensus contradicted oracle"),
    );

    ctx.env.as_contract(&ctx.contract_id, || {
        assert!(AuditTrailManager::verify_integrity(&ctx.env, 10));
    });
}

// ── market state is updated to Resolved ──────────────────────────────────────

#[test]
fn test_override_resolves_market() {
    let ctx = Ctx::new();
    let market_id = ctx.create_market();

    ctx.client().admin_override_verification(
        &ctx.admin,
        &market_id,
        &String::from_str(&ctx.env, "yes"),
        &String::from_str(&ctx.env, "Verified via secondary source"),
    );

    let market = ctx.client().get_market(&market_id).unwrap();
    assert_eq!(market.state, MarketState::Resolved);
    assert_eq!(
        market.oracle_result,
        Some(String::from_str(&ctx.env, "yes"))
    );
}

// ── non-admin cannot override ─────────────────────────────────────────────────
//
// We do NOT call mock_all_auths() here — the stranger has no auth mocked,
// so require_auth() panics and try_ returns Err before any storage write.

#[test]
fn test_override_rejects_non_admin() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register(PredictifyHybrid, ());
    let client = PredictifyHybridClient::new(&env, &contract_id);
    client.initialize(&admin, &None, &None);

    // Create market while auths are still mocked
    let market_id = client.create_market(
        &admin,
        &String::from_str(&env, "Will BTC exceed $100k?"),
        &vec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ],
        &30u32,
        &OracleConfig {
            provider: OracleProvider::reflector(),
            oracle_address: Address::generate(&env),
            feed_id: String::from_str(&env, "BTC"),
            threshold: 100_000_00,
            comparison: String::from_str(&env, "gt"),
        },
        &None,
        &0u64,
        &None,
        &None,
        &None,
    );

    // Now attempt override as a stranger — no auths mocked for this address
    let stranger = Address::generate(&env);
    let result = client.try_admin_override_verification(
        &stranger,
        &market_id,
        &String::from_str(&env, "yes"),
        &String::from_str(&env, "Trying to cheat"),
    );

    assert!(result.is_err());
}

// ── unknown market returns MarketNotFound ─────────────────────────────────────

#[test]
fn test_override_unknown_market() {
    let ctx = Ctx::new();

    let result = ctx.client().try_admin_override_verification(
        &ctx.admin,
        &Symbol::new(&ctx.env, "ghost"),
        &String::from_str(&ctx.env, "yes"),
        &String::from_str(&ctx.env, "Some reason"),
    );

    assert_eq!(result, Err(Ok(Error::MarketNotFound)));
}

// ── no partial state on auth failure ─────────────────────────────────────────
//
// After a failed auth attempt the market must be unchanged.

#[test]
fn test_override_no_partial_state_on_auth_failure() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register(PredictifyHybrid, ());
    let client = PredictifyHybridClient::new(&env, &contract_id);
    client.initialize(&admin, &None, &None);

    let market_id = client.create_market(
        &admin,
        &String::from_str(&env, "Will BTC exceed $100k?"),
        &vec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ],
        &30u32,
        &OracleConfig {
            provider: OracleProvider::reflector(),
            oracle_address: Address::generate(&env),
            feed_id: String::from_str(&env, "BTC"),
            threshold: 100_000_00,
            comparison: String::from_str(&env, "gt"),
        },
        &None,
        &0u64,
        &None,
        &None,
        &None,
    );

    let before = client.get_market(&market_id).unwrap();

    // Attempt override without auth — should fail
    let stranger = Address::generate(&env);
    let _ = client.try_admin_override_verification(
        &stranger,
        &market_id,
        &String::from_str(&env, "yes"),
        &String::from_str(&env, "Sneaky"),
    );

    let after = client.get_market(&market_id).unwrap();
    assert_eq!(before.state, after.state);
    assert_eq!(before.oracle_result, after.oracle_result);
}
