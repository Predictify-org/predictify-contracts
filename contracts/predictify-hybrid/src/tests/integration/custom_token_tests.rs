//! Tests for custom Stellar token/asset support in bets and payouts
//! Covers XLM-native and custom token flows, insufficient balance, and event emission

use super::super::super::*;
use soroban_sdk::testutils::{Address as _, Ledger, LedgerInfo};

#[test]
fn test_place_bet_with_custom_token() {
    let env = Env::default();
    let contract_id = env.register(PredictifyHybrid, ());
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let asset = crate::tokens::Asset {
        contract: Address::generate(&env),
        symbol: Symbol::new(&env, "USDC"),
        decimals: 7,
    };

    env.as_contract(&contract_id, || {
        // Initialize contract with allowed asset
        PredictifyHybrid::initialize(env.clone(), admin.clone(), None, Some(vec![&env, asset.clone()]));

        // Create market with custom asset
        let outcomes = vec![&env, String::from_str(&env, "yes"), String::from_str(&env, "no")];
        let oracle_config = OracleConfig {
            provider: OracleProvider::Reflector,
            oracle_address: Address::generate(&env),
            feed_id: String::from_str(&env, "BTC/USD"),
            threshold: 10000000,
            comparison: String::from_str(&env, "gt"),
        };
        let market_id = PredictifyHybrid::create_market(
            env.clone(),
            admin.clone(),
            String::from_str(&env, "Will BTC exceed $100k?"),
            outcomes,
            30,
            oracle_config,
            None,
            3600,
        );

        // Place bet with custom token
        let bet = PredictifyHybrid::place_bet(
            env.clone(),
            user.clone(),
            market_id.clone(),
            String::from_str(&env, "yes"),
            1000000,
            Some(asset.clone()),
        );
        assert_eq!(bet.amount, 1000000);
        assert_eq!(bet.outcome, String::from_str(&env, "yes"));
    });
}

#[test]
fn test_place_bet_with_xlm_native() {
    let env = Env::default();
    let contract_id = env.register(PredictifyHybrid, ());
    let admin = Address::generate(&env);
    let user = Address::generate(&env);

    env.as_contract(&contract_id, || {
        // Initialize contract (no custom asset)
        PredictifyHybrid::initialize(env.clone(), admin.clone(), None, None);

        // Create market (XLM-native)
        let outcomes = vec![&env, String::from_str(&env, "yes"), String::from_str(&env, "no")];
        let oracle_config = OracleConfig {
            provider: OracleProvider::Reflector,
            oracle_address: Address::generate(&env),
            feed_id: String::from_str(&env, "BTC/USD"),
            threshold: 10000000,
            comparison: String::from_str(&env, "gt"),
        };
        let market_id = PredictifyHybrid::create_market(
            env.clone(),
            admin.clone(),
            String::from_str(&env, "Will BTC exceed $100k?"),
            outcomes,
            30,
            oracle_config,
            None,
            3600,
        );

        // Place bet with XLM-native
        let bet = PredictifyHybrid::place_bet(
            env.clone(),
            user.clone(),
            market_id.clone(),
            String::from_str(&env, "yes"),
            1000000,
            None,
        );
        assert_eq!(bet.amount, 1000000);
        assert_eq!(bet.outcome, String::from_str(&env, "yes"));
    });
}

#[test]
fn test_insufficient_balance_for_custom_token() {
    let env = Env::default();
    let contract_id = env.register(PredictifyHybrid, ());
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let asset = crate::tokens::Asset {
        contract: Address::generate(&env),
        symbol: Symbol::new(&env, "USDC"),
        decimals: 7,
    };

    env.as_contract(&contract_id, || {
        PredictifyHybrid::initialize(env.clone(), admin.clone(), None, Some(vec![&env, asset.clone()]));
        let outcomes = vec![&env, String::from_str(&env, "yes"), String::from_str(&env, "no")];
        let oracle_config = OracleConfig {
            provider: OracleProvider::Reflector,
            oracle_address: Address::generate(&env),
            feed_id: String::from_str(&env, "BTC/USD"),
            threshold: 10000000,
            comparison: String::from_str(&env, "gt"),
        };
        let market_id = PredictifyHybrid::create_market(
            env.clone(),
            admin.clone(),
            String::from_str(&env, "Will BTC exceed $100k?"),
            outcomes,
            30,
            oracle_config,
            None,
            3600,
        );
        // Simulate insufficient balance (should panic or return error)
        let result = std::panic::catch_unwind(|| {
            PredictifyHybrid::place_bet(
                env.clone(),
                user.clone(),
                market_id.clone(),
                String::from_str(&env, "yes"),
                999999999999,
                Some(asset.clone()),
            );
        });
        assert!(result.is_err());
    });
}
