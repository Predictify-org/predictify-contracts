use crate::config::ConfigManager;
use crate::fees::effective_fee_bps;
use crate::types::{Market, MarketState, OracleConfig};
use crate::{Error, PredictifyHybrid, PredictifyHybridClient};
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, Env, String, Symbol, Vec};

fn market(env: &Env) -> Market {
    Market::new(
        env,
        Address::generate(env),
        String::from_str(env, "fee source test"),
        Vec::new(env),
        env.ledger().timestamp() + 86_400,
        OracleConfig::none_sentinel(env),
        None,
        86_400,
        MarketState::Active,
    )
}

#[test]
fn fee_setter_updates_canonical_config_and_legacy_mirror() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register(PredictifyHybrid, ());
    let client = PredictifyHybridClient::new(&env, &contract_id);

    client.initialize(&admin, &Some(200), &None);
    client.set_platform_fee(&admin, &500);

    env.as_contract(&contract_id, || {
        let config = ConfigManager::get_config(&env).unwrap();
        assert_eq!(config.fees.platform_fee_percentage, 500);
        assert_eq!(
            env.storage()
                .persistent()
                .get::<Symbol, i128>(&Symbol::new(&env, "platform_fee")),
            Some(500)
        );
    });
}

#[test]
fn resolver_prefers_canonical_config_over_legacy_mirror() {
    let env = Env::default();
    let contract_id = env.register(PredictifyHybrid, ());
    let test_market = market(&env);

    env.as_contract(&contract_id, || {
        let mut config = ConfigManager::get_development_config(&env);
        config.fees.platform_fee_percentage = 500;
        ConfigManager::store_config(&env, &config).unwrap();
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, "platform_fee"), &200i128);

        assert_eq!(effective_fee_bps(&env, &test_market), Ok(500));
    });
}

#[test]
fn resolver_uses_legacy_compatibility_value_only_without_canonical_config() {
    let env = Env::default();
    let contract_id = env.register(PredictifyHybrid, ());
    let test_market = market(&env);
    env.as_contract(&contract_id, || {
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, "platform_fee"), &400i128);
        assert_eq!(effective_fee_bps(&env, &test_market), Ok(400));
    });

    let missing_env = Env::default();
    let missing_contract_id = missing_env.register(PredictifyHybrid, ());
    let missing_market = market(&missing_env);
    missing_env.as_contract(&missing_contract_id, || {
        assert_eq!(
            effective_fee_bps(&missing_env, &missing_market),
            Err(Error::ConfigNotFound)
        );
    });
}
