#![cfg(test)]

use crate::err::Error;
use crate::force_resolve::ForceResolveManager;
use crate::types::{MarketState, OracleConfig, OracleProvider};
use crate::{PredictifyHybrid, PredictifyHybridClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    vec, Address, Env, String, Symbol, Vec,
};

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
        let token_contract =
            env.register_stellar_asset_contract_v2(Address::generate(&env));
        let token_id = token_contract.address();
        env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .set(&Symbol::new(&env, "TokenID"), &token_id);
            env.storage()
                .persistent()
                .set(&Symbol::new(&env, "platform_fee"), &200i128);
            crate::circuit_breaker::CircuitBreaker::initialize(&env).unwrap();
        });
        PredictifyHybridClient::new(&env, &contract_id).initialize(&admin, &None, &None);
        Ctx { env, contract_id, admin }
    }

    fn client(&self) -> PredictifyHybridClient<'_> {
        PredictifyHybridClient::new(&self.env, &self.contract_id)
    }

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

fn outcomes(env: &Env, vals: &[&str]) -> Vec<String> {
    let mut v = Vec::new(env);
    for s in vals {
        v.push_back(String::from_str(env, s));
    }
    v
}

fn key(env: &Env, s: &str) -> String {
    String::from_str(env, s)
}

fn reason(env: &Env, s: &str) -> String {
    String::from_str(env, s)
}

// ── happy path ────────────────────────────────────────────────────────────

#[test]
fn test_force_resolve_active_market() {
    let ctx = Ctx::new();
    let market_id = ctx.create_market();

    let result = ctx.client().try_force_resolve_market(
        &ctx.admin,
        &market_id,
        &outcomes(&ctx.env, &["yes"]),
        &reason(&ctx.env, "Emergency"),
        &key(&ctx.env, "key-001"),
    );
    assert_eq!(result, Ok(()));

    let market = ctx.client().get_market(&market_id).unwrap();
    assert_eq!(market.state, MarketState::Resolved);
    assert_eq!(market.winning_outcomes, Some(outcomes(&ctx.env, &["yes"])));
}

#[test]
fn test_force_resolve_before_end_time_succeeds() {
    let ctx = Ctx::new();
    let market_id = ctx.create_market();

    let result = ctx.client().try_force_resolve_market(
        &ctx.admin,
        &market_id,
        &outcomes(&ctx.env, &["yes"]),
        &reason(&ctx.env, "Force resolve before end time"),
        &key(&ctx.env, "key-early"),
    );
    assert_eq!(result, Ok(()));

    let market = ctx.client().get_market(&market_id).unwrap();
    assert_eq!(market.state, MarketState::Resolved);
}

#[test]
fn test_force_resolve_ended_market() {
    let ctx = Ctx::new();
    let market_id = ctx.create_market();

    ctx.env.ledger().with_mut(|li| {
        li.timestamp = 1_000_000;
    });

    let result = ctx.client().try_force_resolve_market(
        &ctx.admin,
        &market_id,
        &outcomes(&ctx.env, &["no"]),
        &reason(&ctx.env, "Ended market resolve"),
        &key(&ctx.env, "ended-key"),
    );
    assert_eq!(result, Ok(()));

    let market = ctx.client().get_market(&market_id).unwrap();
    assert_eq!(market.state, MarketState::Resolved);
}

// ── input validation ──────────────────────────────────────────────────────

#[test]
fn test_force_resolve_invalid_outcome() {
    let ctx = Ctx::new();
    let market_id = ctx.create_market();

    let result = ctx.client().try_force_resolve_market(
        &ctx.admin,
        &market_id,
        &outcomes(&ctx.env, &["maybe"]),
        &reason(&ctx.env, "Invalid"),
        &key(&ctx.env, "key-io"),
    );
    assert_eq!(result, Err(Ok(Error::InvalidOutcome)));
}

#[test]
fn test_force_resolve_empty_outcomes() {
    let ctx = Ctx::new();
    let market_id = ctx.create_market();

    let result = ctx.client().try_force_resolve_market(
        &ctx.admin,
        &market_id,
        &Vec::new(&ctx.env),
        &reason(&ctx.env, "Empty outcomes"),
        &key(&ctx.env, "key-empty"),
    );
    assert_eq!(result, Err(Ok(Error::InvalidInput)));
}

#[test]
fn test_force_resolve_empty_reason() {
    let ctx = Ctx::new();
    let market_id = ctx.create_market();

    let result = ctx.client().try_force_resolve_market(
        &ctx.admin,
        &market_id,
        &outcomes(&ctx.env, &["yes"]),
        &reason(&ctx.env, ""),
        &key(&ctx.env, "key-reason"),
    );
    assert_eq!(result, Err(Ok(Error::ForceResolveReasonEmpty)));
}

#[test]
fn test_force_resolve_market_not_found() {
    let ctx = Ctx::new();
    let fake_id = Symbol::new(&ctx.env, "nonexistent");

    let result = ctx.client().try_force_resolve_market(
        &ctx.admin,
        &fake_id,
        &outcomes(&ctx.env, &["yes"]),
        &reason(&ctx.env, "Not found"),
        &key(&ctx.env, "key-nf"),
    );
    assert_eq!(result, Err(Ok(Error::MarketNotFound)));
}

// ── idempotency ───────────────────────────────────────────────────────────

#[test]
fn test_force_resolve_idempotency_key_replay() {
    let ctx = Ctx::new();
    let market_id = ctx.create_market();

    ctx.client()
        .try_force_resolve_market(
            &ctx.admin,
            &market_id,
            &outcomes(&ctx.env, &["yes"]),
            &reason(&ctx.env, "First"),
            &key(&ctx.env, "idem-key"),
        )
        .unwrap();

    let result = ctx.client().try_force_resolve_market(
        &ctx.admin,
        &market_id,
        &outcomes(&ctx.env, &["no"]),
        &reason(&ctx.env, "Replay"),
        &key(&ctx.env, "idem-key"),
    );
    assert_eq!(result, Err(Ok(Error::ForceResolveReplayed)));

    let market = ctx.client().get_market(&market_id).unwrap();
    assert!(market.is_resolved());
}

#[test]
fn test_force_resolve_different_keys_same_market() {
    let ctx = Ctx::new();
    let market_id = ctx.create_market();

    ctx.client()
        .try_force_resolve_market(
            &ctx.admin,
            &market_id,
            &outcomes(&ctx.env, &["yes"]),
            &reason(&ctx.env, "First"),
            &key(&ctx.env, "key-a"),
        )
        .unwrap();

    ctx.client()
        .try_force_resolve_market(
            &ctx.admin,
            &market_id,
            &outcomes(&ctx.env, &["no"]),
            &reason(&ctx.env, "Second"),
            &key(&ctx.env, "key-b"),
        )
        .unwrap();

    let market = ctx.client().get_market(&market_id).unwrap();
    assert_eq!(market.winning_outcomes, Some(outcomes(&ctx.env, &["no"])));
}

#[test]
fn test_force_resolve_same_key_different_market() {
    let ctx = Ctx::new();
    let market_a = ctx.create_market();
    let market_b = ctx.create_market();

    let shared_key = key(&ctx.env, "shared-key");

    ctx.client()
        .try_force_resolve_market(
            &ctx.admin,
            &market_a,
            &outcomes(&ctx.env, &["yes"]),
            &reason(&ctx.env, "Market A"),
            &shared_key,
        )
        .unwrap();

    ctx.client()
        .try_force_resolve_market(
            &ctx.admin,
            &market_b,
            &outcomes(&ctx.env, &["no"]),
            &reason(&ctx.env, "Market B"),
            &shared_key,
        )
        .unwrap();

    assert_eq!(
        ctx.client().get_market(&market_a).unwrap().state,
        MarketState::Resolved
    );
    assert_eq!(
        ctx.client().get_market(&market_b).unwrap().state,
        MarketState::Resolved
    );
}

// ── multiple winning outcomes ─────────────────────────────────────────────

#[test]
fn test_force_resolve_multiple_winning_outcomes() {
    let ctx = Ctx::new();
    let market_id = ctx.create_market();

    let winners = outcomes(&ctx.env, &["yes", "no"]);
    let result = ctx.client().try_force_resolve_market(
        &ctx.admin,
        &market_id,
        &winners,
        &reason(&ctx.env, "Tie"),
        &key(&ctx.env, "multi-key"),
    );
    assert_eq!(result, Ok(()));

    let market = ctx.client().get_market(&market_id).unwrap();
    assert_eq!(market.state, MarketState::Resolved);
    assert_eq!(market.winning_outcomes, Some(winners));
}

// ── unauthorized ──────────────────────────────────────────────────────────

#[test]
fn test_force_resolve_unauthorized() {
    let ctx = Ctx::new();
    let market_id = ctx.create_market();
    let stranger = Address::generate(&ctx.env);

    let result = ctx.client().try_force_resolve_market(
        &stranger,
        &market_id,
        &outcomes(&ctx.env, &["yes"]),
        &reason(&ctx.env, "Hack attempt"),
        &key(&ctx.env, "key-unauth"),
    );
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}
