//! Issue #1452 – oracle resolution path (`fetch_oracle_result` + `resolve_market`).
//!
//! A mock Reflector contract serves configurable quotes so the tests exercise the
//! real cross-contract lookup, threshold comparison, outcome selection, and the
//! typed error paths.

#![allow(deprecated)]

use crate::err::Error;
use crate::types::{
    Market, MarketState, OracleConfig, OracleProvider, ReflectorAsset, ReflectorPriceData,
};
use crate::{PredictifyHybrid, PredictifyHybridClient};
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{contract, contractimpl, vec, Address, Env, String, Symbol};

#[contract]
pub struct MockReflector;

#[contractimpl]
impl MockReflector {
    pub fn set_quote(env: Env, price: i128, timestamp: u64) {
        env.storage().instance().set(&Symbol::new(&env, "quote"), &(price, timestamp));
    }

    pub fn lastprice(env: Env, _asset: ReflectorAsset) -> Option<ReflectorPriceData> {
        let (price, timestamp): (i128, u64) =
            env.storage().instance().get(&Symbol::new(&env, "quote"))?;
        Some(ReflectorPriceData {
            price,
            timestamp,
            source: String::from_str(&env, "mock"),
        })
    }
}

const THRESHOLD: i128 = 50_000_00;

struct Setup {
    env: Env,
    contract_id: Address,
    admin: Address,
    token_id: Address,
}

impl Setup {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let contract_id = env.register(PredictifyHybrid, ());
        let token_admin = Address::generate(&env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin).address();
        env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .set(&Symbol::new(&env, "TokenID"), &token_id);
        });
        let client = PredictifyHybridClient::new(&env, &contract_id);
        client.initialize(&admin, &None, &None);
        Self { env, contract_id, admin, token_id }
    }

    fn client(&self) -> PredictifyHybridClient<'_> {
        PredictifyHybridClient::new(&self.env, &self.contract_id)
    }

    /// Register a mock Reflector holding a quote stamped `age` seconds before `now`.
    fn oracle(&self, price: i128, age: u64) -> Address {
        let addr = self.env.register(MockReflector, ());
        let ts = self.env.ledger().timestamp().saturating_sub(age);
        MockReflectorClient::new(&self.env, &addr).set_quote(&price, &ts);
        addr
    }

    fn config(&self, oracle: Address, comparison: &str) -> OracleConfig {
        OracleConfig::new(
            OracleProvider::reflector(),
            oracle,
            String::from_str(&self.env, "BTC/USD"),
            THRESHOLD,
            String::from_str(&self.env, comparison),
        )
    }

    fn create_market(&self, primary: OracleConfig, fallback: Option<OracleConfig>) -> Symbol {
        let outcomes = vec![
            &self.env,
            String::from_str(&self.env, "yes"),
            String::from_str(&self.env, "no"),
        ];
        self.client().create_market(
            &self.admin,
            &String::from_str(&self.env, "Will BTC close above $50k?"),
            &outcomes,
            &1u32,
            &primary,
            &fallback,
            &3_600u64,
            &None,
            &None,
            &None,
            &None,
            &None,
        )
    }

    fn market(&self, id: &Symbol) -> Market {
        self.client().get_market(id).unwrap()
    }

    fn end_market(&self, id: &Symbol) {
        let end = self.market(id).end_time;
        self.env.ledger().with_mut(|li| li.timestamp = end + 1);
    }

    fn fetch(&self, id: &Symbol) -> Result<String, Error> {
        let market = self.market(id);
        match self
            .client()
            .try_fetch_oracle_result(id, &market.oracle_config.oracle_address)
        {
            Ok(Ok(outcome)) => Ok(outcome),
            Err(Ok(err)) => Err(err),
            other => panic!("unexpected invocation result: {:?}", other),
        }
    }

    fn bet(&self, id: &Symbol, outcome: &str) {
        let user = Address::generate(&self.env);
        soroban_sdk::token::StellarAssetClient::new(&self.env, &self.token_id)
            .mint(&user, &10_000_000_000);
        self.client().place_bet(
            &user,
            id,
            &String::from_str(&self.env, outcome),
            &10_000_000i128,
            &250,
        );
    }
}

/// Create a market whose end time has passed, backed by a mock Reflector quote.
fn ended_market(setup: &Setup, price: i128, age: u64, comparison: &str) -> Symbol {
    let id = setup.create_market(setup.config(setup.oracle(price, 0), comparison), None);
    setup.end_market(&id);
    // Re-stamp the quote relative to the post-end ledger time.
    let cfg = setup.market(&id).oracle_config;
    let ts = setup.env.ledger().timestamp().saturating_sub(age);
    MockReflectorClient::new(&setup.env, &cfg.oracle_address).set_quote(&price, &ts);
    id
}

#[test]
fn price_above_threshold_resolves_yes_and_stores_result() {
    let setup = Setup::new();
    let id = ended_market(&setup, THRESHOLD + 1, 0, "gt");

    assert_eq!(setup.fetch(&id), Ok(String::from_str(&setup.env, "yes")));
    assert_eq!(
        setup.market(&id).oracle_result,
        Some(String::from_str(&setup.env, "yes"))
    );
}

#[test]
fn price_below_threshold_resolves_no() {
    let setup = Setup::new();
    let id = ended_market(&setup, THRESHOLD - 1, 0, "gt");

    assert_eq!(setup.fetch(&id), Ok(String::from_str(&setup.env, "no")));
    assert_eq!(
        setup.market(&id).oracle_result,
        Some(String::from_str(&setup.env, "no"))
    );
}

#[test]
fn lt_comparison_is_honoured() {
    let setup = Setup::new();
    let id = ended_market(&setup, THRESHOLD - 1, 0, "lt");

    assert_eq!(setup.fetch(&id), Ok(String::from_str(&setup.env, "yes")));
}

#[test]
fn result_is_never_the_pending_placeholder() {
    let setup = Setup::new();
    let id = ended_market(&setup, THRESHOLD + 5, 0, "gt");

    let outcome = setup.fetch(&id).unwrap();
    assert_ne!(outcome, String::from_str(&setup.env, "pending"));
}

#[test]
fn resolve_market_sets_winning_outcome_and_state() {
    let setup = Setup::new();
    let id = setup.create_market(setup.config(setup.oracle(THRESHOLD + 1, 0), "gt"), None);
    setup.bet(&id, "yes");
    setup.end_market(&id);
    let oracle = setup.market(&id).oracle_config.oracle_address;
    MockReflectorClient::new(&setup.env, &oracle)
        .set_quote(&(THRESHOLD + 1), &setup.env.ledger().timestamp());

    setup.fetch(&id).unwrap();
    setup.client().resolve_market(&Address::generate(&setup.env), &id);

    let market = setup.market(&id);
    assert_eq!(market.state, MarketState::Resolved);
    let winners = market.winning_outcomes.expect("winning outcome must be recorded");
    assert_eq!(winners.len(), 1);
    assert_eq!(winners.get(0).unwrap(), String::from_str(&setup.env, "yes"));
}

#[test]
fn resolve_market_without_oracle_result_errors_and_leaves_market_open() {
    let setup = Setup::new();
    let id = ended_market(&setup, THRESHOLD + 1, 0, "gt");

    let res = setup
        .client()
        .try_resolve_market(&Address::generate(&setup.env), &id);
    assert_eq!(res, Err(Ok(Error::OracleUnavailable)));
    assert!(setup.market(&id).winning_outcomes.is_none());
    assert_ne!(setup.market(&id).state, MarketState::Resolved);
}

#[test]
fn unreachable_oracle_contract_returns_typed_error_without_storing_result() {
    let setup = Setup::new();
    // A fresh address with no contract behind it: the cross-contract call fails.
    let dead = Address::generate(&setup.env);
    let id = setup.create_market(setup.config(dead, "gt"), None);
    setup.end_market(&id);

    assert_eq!(setup.fetch(&id), Err(Error::OracleUnavailable));
    assert!(setup.market(&id).oracle_result.is_none());
}

#[test]
fn oracle_without_price_data_returns_oracle_unavailable() {
    let setup = Setup::new();
    // Registered mock with no quote set: `lastprice` returns None.
    let empty = setup.env.register(MockReflector, ());
    let id = setup.create_market(setup.config(empty, "gt"), None);
    setup.end_market(&id);

    assert_eq!(setup.fetch(&id), Err(Error::OracleUnavailable));
    assert!(setup.market(&id).oracle_result.is_none());
}

#[test]
fn stale_quote_is_rejected() {
    let setup = Setup::new();
    let id = ended_market(&setup, THRESHOLD + 1, 10_000, "gt");

    assert_eq!(setup.fetch(&id), Err(Error::OracleStale));
    assert!(setup.market(&id).oracle_result.is_none());
}

#[test]
fn non_positive_price_is_rejected() {
    let setup = Setup::new();
    let id = ended_market(&setup, 0, 0, "gt");

    assert_eq!(setup.fetch(&id), Err(Error::OracleUnavailable));
}

#[test]
fn fallback_oracle_used_when_primary_fails() {
    let setup = Setup::new();
    let dead = Address::generate(&setup.env);
    let fallback = setup.oracle(THRESHOLD + 1, 0);
    let id = setup.create_market(
        setup.config(dead, "gt"),
        Some(setup.config(fallback.clone(), "gt")),
    );
    setup.end_market(&id);
    MockReflectorClient::new(&setup.env, &fallback)
        .set_quote(&(THRESHOLD + 1), &setup.env.ledger().timestamp());

    assert_eq!(setup.fetch(&id), Ok(String::from_str(&setup.env, "yes")));
}

#[test]
fn primary_and_fallback_failure_reports_fallback_unavailable() {
    let setup = Setup::new();
    let id = setup.create_market(
        setup.config(Address::generate(&setup.env), "gt"),
        Some(setup.config(Address::generate(&setup.env), "gt")),
    );
    setup.end_market(&id);

    assert_eq!(setup.fetch(&id), Err(Error::FallbackOracleUnavailable));
    assert!(setup.market(&id).oracle_result.is_none());
}

#[test]
fn active_market_cannot_fetch_oracle_result() {
    let setup = Setup::new();
    let id = setup.create_market(setup.config(setup.oracle(THRESHOLD + 1, 0), "gt"), None);

    assert_eq!(setup.fetch(&id), Err(Error::MarketClosed));
}

#[test]
fn oracle_result_cannot_be_fetched_twice() {
    let setup = Setup::new();
    let id = ended_market(&setup, THRESHOLD + 1, 0, "gt");

    setup.fetch(&id).unwrap();
    assert_eq!(setup.fetch(&id), Err(Error::MarketResolved));
}
