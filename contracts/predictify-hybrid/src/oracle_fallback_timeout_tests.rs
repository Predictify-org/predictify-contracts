use crate::errors::Error;
use crate::events::{
    FallbackUsedEvent, ManualResolutionRequiredEvent, RefundOnOracleFailureEvent,
    ResolutionTimeoutEvent,
};
use crate::types::{Market, MarketState, OracleConfig, OracleProvider};
use crate::{PredictifyHybrid, PredictifyHybridClient};
use soroban_sdk::testutils::{Address as _, Events, Ledger};
use soroban_sdk::{symbol_short, vec, Address, Env, String, Symbol, TryFromVal, TryIntoVal};

struct TestSetup {
    env: Env,
    contract_id: Address,
    admin: Address,
    token_id: Address,
}

impl TestSetup {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let contract_id = env.register(PredictifyHybrid, ());

        let token_admin = Address::generate(&env);
        let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_id = token_contract.address();

        env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .set(&Symbol::new(&env, "TokenID"), &token_id);
        });

        let client = PredictifyHybridClient::new(&env, &contract_id);
        client.initialize(&admin, &None);
        env.as_contract(&contract_id, || {
            crate::circuit_breaker::CircuitBreaker::initialize(&env)
                .expect("circuit breaker should initialize in tests");
        });

        Self {
            env,
            contract_id,
            admin,
            token_id,
        }
    }

    fn client(&self) -> PredictifyHybridClient<'_> {
        PredictifyHybridClient::new(&self.env, &self.contract_id)
    }

    fn create_user(&self) -> Address {
        let user = Address::generate(&self.env);
        let stellar_client = soroban_sdk::token::StellarAssetClient::new(&self.env, &self.token_id);
        self.env.mock_all_auths();
        stellar_client.mint(&user, &10_000_000_000);
        user
    }

    fn create_market(&self, has_fallback: bool, resolution_timeout: u64) -> Symbol {
        let outcomes = vec![
            &self.env,
            String::from_str(&self.env, "yes"),
            String::from_str(&self.env, "no"),
        ];
        let primary = OracleConfig::new(
            OracleProvider::reflector(),
            Address::generate(&self.env),
            String::from_str(&self.env, "BTC/USD"),
            50_000_00,
            String::from_str(&self.env, "gt"),
        );
        let fallback = has_fallback.then(|| {
            OracleConfig::new(
                OracleProvider::reflector(),
                Address::generate(&self.env),
                String::from_str(&self.env, "ETH/USD"),
                50_000_00,
                String::from_str(&self.env, "gt"),
            )
        });

        self.env.mock_all_auths();
        self.client().create_market(
            &self.admin,
            &String::from_str(&self.env, "Will BTC close above $50k?"),
            &outcomes,
            &1u32,
            &primary,
            &fallback,
            &resolution_timeout,
        )
    }

    fn get_market(&self, market_id: &Symbol) -> Market {
        self.client().get_market(market_id).unwrap()
    }

    fn advance_to(&self, timestamp: u64) {
        self.env.ledger().with_mut(|li| {
            li.timestamp = timestamp;
        });
    }
}

fn find_published_event<T>(env: &Env, topic: Symbol) -> Option<T>
where
    T: Clone + TryFromVal<Env, soroban_sdk::xdr::ScVal>,
{
    let events = env.events().all();
    events.events().iter().rev().find_map(|event| {
        let body = match &event.body {
            soroban_sdk::xdr::ContractEventBody::V0(v0) => v0,
        };
        let first_topic_scval = body.topics.get(0)?;
        let first_topic: Symbol = first_topic_scval.clone().try_into_val(env).ok()?;
        if first_topic != topic {
            return None;
        }
        T::try_from_val(env, &body.data).ok()
    })
}

#[test]
fn fetch_oracle_result_without_fallback_reports_primary_only_failure() {
    let setup = TestSetup::new();
    let market_id = setup.create_market(false, 3_600);
    let market = setup.get_market(&market_id);

    setup.advance_to(market.end_time + 1);

    let result = setup.env.as_contract(&setup.contract_id, || {
        PredictifyHybrid::fetch_oracle_result(
            setup.env.clone(),
            market_id.clone(),
            market.oracle_config.oracle_address.clone(),
        )
    });
    assert_eq!(result, Err(Error::OracleUnavailable));

    let manual =
        find_published_event::<ManualResolutionRequiredEvent>(&setup.env, symbol_short!("man_res"))
            .expect("manual resolution event should be published");
    assert_eq!(manual.market_id, market_id);
    assert_eq!(
        manual.reason,
        String::from_str(&setup.env, "oracle_resolution_failed_primary_only")
    );
}

#[test]
fn fetch_oracle_result_with_fallback_reports_primary_then_fallback_failure() {
    let setup = TestSetup::new();
    let market_id = setup.create_market(true, 3_600);
    let market = setup.get_market(&market_id);

    setup.advance_to(market.end_time + 1);

    let result = setup.env.as_contract(&setup.contract_id, || {
        PredictifyHybrid::fetch_oracle_result(
            setup.env.clone(),
            market_id.clone(),
            market.oracle_config.oracle_address.clone(),
        )
    });
    assert_eq!(result, Err(Error::FallbackOracleUnavailable));

    let manual =
        find_published_event::<ManualResolutionRequiredEvent>(&setup.env, symbol_short!("man_res"))
            .expect("manual resolution event should be published");
    assert_eq!(manual.market_id, market_id);
    assert_eq!(
        manual.reason,
        String::from_str(&setup.env, "oracle_resolution_failed_primary_then_fallback",)
    );

    assert!(
        find_published_event::<FallbackUsedEvent>(&setup.env, symbol_short!("fbk_used")).is_none(),
        "fallback-used event should only be emitted after a successful fallback result"
    );
}

#[test]
fn fetch_oracle_result_stops_at_market_resolution_deadline() {
    let setup = TestSetup::new();
    let market_id = setup.create_market(true, 3_600);
    let market = setup.get_market(&market_id);
    let deadline = market.end_time + market.resolution_timeout;

    setup.advance_to(deadline);

    let result = setup.env.as_contract(&setup.contract_id, || {
        PredictifyHybrid::fetch_oracle_result(
            setup.env.clone(),
            market_id.clone(),
            market.oracle_config.oracle_address.clone(),
        )
    });
    assert_eq!(result, Err(Error::ResolutionTimeoutReached));

    let timeout_event =
        find_published_event::<ResolutionTimeoutEvent>(&setup.env, symbol_short!("res_tmo"))
            .expect("resolution-timeout event should be published");
    assert_eq!(timeout_event.market_id, market_id);
    assert_eq!(timeout_event.timeout_timestamp, deadline);
}

/// Disputes filed within the window must be accepted.
#[test]
fn refund_on_oracle_failure_uses_market_specific_timeout_for_non_admins() {
    let setup = TestSetup::new();
    let resolution_timeout = 3_600u64;
    let market_id = setup.create_market(false, resolution_timeout);
    let user = setup.create_user();
    let caller = setup.create_user();

    setup.env.mock_all_auths();
    setup.client().place_bet(
        &user,
        &market_id,
        &String::from_str(&setup.env, "yes"),
        &10_000_000i128,
    );

    let market = setup.get_market(&market_id);
    setup.advance_to(market.end_time + resolution_timeout - 1);

    let early = setup.env.as_contract(&setup.contract_id, || {
        PredictifyHybrid::refund_on_oracle_failure(
            setup.env.clone(),
            caller.clone(),
            market_id.clone(),
        )
    });
    assert_eq!(early, Err(Error::Unauthorized));

    let deadline = market.end_time + resolution_timeout;
    setup.advance_to(deadline);
    setup.env.mock_all_auths();
    let refunded = setup.env.as_contract(&setup.contract_id, || {
        PredictifyHybrid::refund_on_oracle_failure(
            setup.env.clone(),
            caller.clone(),
            market_id.clone(),
        )
    });
    assert_eq!(refunded, Ok(10_000_000i128));

    let cancelled_market = setup.get_market(&market_id);
    assert_eq!(cancelled_market.state, MarketState::Cancelled);

    let refund_event: RefundOnOracleFailureEvent =
        setup.env.as_contract(&setup.contract_id, || {
            setup
                .env
                .storage()
                .persistent()
                .get(&symbol_short!("ref_oracl"))
                .expect("refund event should be stored")
        });
    assert_eq!(refund_event.market_id, market_id);
    assert_eq!(refund_event.total_refunded, 10_000_000i128);
}
