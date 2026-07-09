#![cfg(test)]

extern crate std;

use predictify_hybrid::{
    Error, MarketState, OracleConfig, OracleProvider,
    PredictifyHybrid, PredictifyHybridClient,
};
use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    vec, Address, Env, String, Symbol,
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
        PredictifyHybridClient::new(&env, &contract_id).initialize(&admin, &None, &None);
        Self { env, contract_id, admin }
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

    fn market_state(&self, market_id: &Symbol) -> MarketState {
        self.client().get_market(market_id).unwrap().state
    }
}

#[test]
fn test_force_resolve_requires_admin_auth() {
    let ctx = Ctx::new();
    let market_id = ctx.create_market();

    let non_admin = Address::generate(&ctx.env);
    let result = PredictifyHybridClient::new(&ctx.env, &ctx.contract_id)
        .try_admin_force_resolve(
            &non_admin,
            &market_id,
            &String::from_str(&ctx.env, "yes"),
            &Symbol::new(&ctx.env, "key_001"),
        );

    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

#[test]
fn test_force_resolve_rejects_nonexistent_market() {
    let ctx = Ctx::new();
    let fake_id = Symbol::new(&ctx.env, "nonexistent");

    let result = ctx.client().try_admin_force_resolve(
        &ctx.admin,
        &fake_id,
        &String::from_str(&ctx.env, "yes"),
        &Symbol::new(&ctx.env, "key_002"),
    );

    assert_eq!(result, Err(Ok(Error::MarketNotFound)));
}

#[test]
fn test_force_resolve_rejects_invalid_outcome() {
    let ctx = Ctx::new();
    let market_id = ctx.create_market();

    let result = ctx.client().try_admin_force_resolve(
        &ctx.admin,
        &market_id,
        &String::from_str(&ctx.env, "invalid_outcome"),
        &Symbol::new(&ctx.env, "key_003"),
    );

    assert_eq!(result, Err(Ok(Error::InvalidOutcome)));
}

#[test]
fn test_force_resolve_success() {
    let ctx = Ctx::new();
    let market_id = ctx.create_market();

    assert_eq!(ctx.market_state(&market_id), MarketState::Active);

    let result = ctx.client().try_admin_force_resolve(
        &ctx.admin,
        &market_id,
        &String::from_str(&ctx.env, "yes"),
        &Symbol::new(&ctx.env, "key_004"),
    );

    assert_eq!(result, Ok(Ok(())));
    assert_eq!(ctx.market_state(&market_id), MarketState::Resolved);
}

#[test]
fn test_force_resolve_rejects_replayed_idempotency_key() {
    let ctx = Ctx::new();
    let market_id = ctx.create_market();

    let result = ctx.client().try_admin_force_resolve(
        &ctx.admin,
        &market_id,
        &String::from_str(&ctx.env, "yes"),
        &Symbol::new(&ctx.env, "key_005"),
    );
    assert_eq!(result, Ok(Ok(())));

    let result = ctx.client().try_admin_force_resolve(
        &ctx.admin,
        &market_id,
        &String::from_str(&ctx.env, "yes"),
        &Symbol::new(&ctx.env, "key_005"),
    );
    assert_eq!(result, Err(Ok(Error::ForceResolveReplayed)));
}

#[test]
fn test_force_resolve_different_keys_both_succeed() {
    let ctx = Ctx::new();
    let market_id = ctx.create_market();

    let result = ctx.client().try_admin_force_resolve(
        &ctx.admin,
        &market_id,
        &String::from_str(&ctx.env, "yes"),
        &Symbol::new(&ctx.env, "key_006"),
    );
    assert_eq!(result, Ok(Ok(())));

    let result = ctx.client().try_admin_force_resolve(
        &ctx.admin,
        &market_id,
        &String::from_str(&ctx.env, "yes"),
        &Symbol::new(&ctx.env, "key_007"),
    );
    assert_eq!(result, Ok(Ok(())));
}

#[test]
fn test_force_resolve_ended_market() {
    let ctx = Ctx::new();
    let market_id = ctx.create_market();

    ctx.env.ledger().set(LedgerInfo {
        timestamp: 30 * 24 * 60 * 60 + 1,
        protocol_version: 25,
        sequence_number: ctx.env.ledger().sequence(),
        network_id: Default::default(),
        base_reserve: 10,
        min_persistent_entry_ttl: 1,
        min_temp_entry_ttl: 1,
        max_entry_ttl: 535680,
    });

    let result = ctx.client().try_admin_force_resolve(
        &ctx.admin,
        &market_id,
        &String::from_str(&ctx.env, "yes"),
        &Symbol::new(&ctx.env, "key_008"),
    );
    assert_eq!(result, Ok(Ok(())));
    assert_eq!(ctx.market_state(&market_id), MarketState::Resolved);
}

#[test]
fn test_force_resolve_rejects_empty_outcome() {
    let ctx = Ctx::new();
    let market_id = ctx.create_market();

    let result = ctx.client().try_admin_force_resolve(
        &ctx.admin,
        &market_id,
        &String::from_str(&ctx.env, ""),
        &Symbol::new(&ctx.env, "key_009"),
    );

    assert_eq!(result, Err(Ok(Error::InvalidOutcome)));
}
