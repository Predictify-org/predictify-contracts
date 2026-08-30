#![cfg(test)]

use crate::err::Error;
use crate::timelock::MarketTimelockManager;
use crate::types::{Market, OracleConfig, OracleProvider};
use crate::{PredictifyHybrid, PredictifyHybridClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    vec, Address, Env, String, Symbol, Vec,
};

struct TestContext {
    env: Env,
    contract_id: Address,
    admin: Address,
}

impl TestContext {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let contract_id = env.register(PredictifyHybrid, ());
        let token_contract = env.register_stellar_asset_contract_v2(Address::generate(&env));
        let token_id = token_contract.address();
        env.as_contract(&contract_id, || {
            env.storage().persistent().set(&Symbol::new(&env, "TokenID"), &token_id);
            env.storage().persistent().set(&Symbol::new(&env, "platform_fee"), &200i128);
            crate::circuit_breaker::CircuitBreaker::initialize(&env).unwrap();
        });
        PredictifyHybridClient::new(&env, &contract_id).initialize(&admin, &None, &None);

        Self {
            env,
            contract_id,
            admin,
        }
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
            &None,
            &None,
        )
    }
}

#[test]
fn test_market_timelock_blocks_admin_action_until_delay_passes() {
    let ctx = TestContext::new();
    let market_id = ctx.create_market();

    ctx.env.as_contract(&ctx.contract_id, || {
        let mut market: Market = ctx
            .env
            .storage()
            .persistent()
            .get(&market_id)
            .expect("market should be stored");

        // Configure a 10-second timelock on the market.
        MarketTimelockManager::configure(&ctx.env, &mut market, &ctx.admin, &ctx.admin, 10)
            .expect("admin should be able to configure the timelock");

        // An admin action is rejected while the delay has not elapsed.
        let early = MarketTimelockManager::ensure_admin_action_allowed(
            &ctx.env,
            &mut market,
            &ctx.admin,
            &ctx.admin,
        );
        assert_eq!(early, Err(Error::AdminActionTimelocked));

        ctx.env.ledger().with_mut(|li| {
            li.timestamp = li.timestamp.saturating_add(11);
        });

        // After the delay, the admin action is allowed and the clock refreshes.
        let later = MarketTimelockManager::ensure_admin_action_allowed(
            &ctx.env,
            &mut market,
            &ctx.admin,
            &ctx.admin,
        );
        assert_eq!(later, Ok(()));
    });
}

#[test]
fn test_market_timelock_rejects_unauthorized_configuration() {
    let ctx = TestContext::new();
    let market_id = ctx.create_market();
    let stranger = Address::generate(&ctx.env);

    ctx.env.as_contract(&ctx.contract_id, || {
        let mut market: Market = ctx
            .env
            .storage()
            .persistent()
            .get(&market_id)
            .expect("market should be stored");

        // Neither the market admin nor the contract admin: rejected.
        let result = MarketTimelockManager::configure(
            &ctx.env,
            &mut market,
            &stranger,
            &ctx.admin,
            10,
        );
        assert_eq!(result, Err(Error::Unauthorized));

        // A zero-delay configuration never blocks admin actions.
        MarketTimelockManager::configure(&ctx.env, &mut market, &ctx.admin, &ctx.admin, 0)
            .expect("admin should be able to configure the timelock");
        let allowed = MarketTimelockManager::ensure_admin_action_allowed(
            &ctx.env,
            &mut market,
            &ctx.admin,
            &ctx.admin,
        );
        assert_eq!(allowed, Ok(()));
    });
}
