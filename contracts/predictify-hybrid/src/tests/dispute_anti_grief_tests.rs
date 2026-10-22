#![cfg(test)]
extern crate std;

use alloc::vec;
use soroban_sdk::{
    testutils::Address as _,
    Address, Env, String, Symbol,
};

use crate::{
    disputes::DisputeManager,
    types::{Error, Market, MarketState, OracleConfig},
};

fn setup_env_and_market() -> (Env, Address, Symbol) {
    let env = Env::default();
    let admin = Address::generate(&env);
    let market_id = Symbol::new(&env, "BTC_50K");

    let market = Market {
        admin: admin.clone(),
        question: String::from_str(&env, "Will BTC hit 50k?"),
        outcomes: vec![
            String::from_str(&env, "Yes"),
            String::from_str(&env, "No"),
        ],
        end_time: env.ledger().timestamp() + 3600,
        oracle_config: OracleConfig {
            feed_id: Symbol::new(&env, "BTC_USD"),
            oracle_address: admin.clone(),
            minimum_confidence: 80,
            required_validations: 1,
            fallback_duration: 3600,
        },
        state: MarketState::Active,
        total_staked: 0,
        bets: vec![],
        votes: soroban_sdk::Map::new(&env),
        stakes: soroban_sdk::Map::new(&env),
        disputes: vec![],
        dispute_stakes: soroban_sdk::Map::new(&env),
        resolutions: vec![],
        winning_outcomes: None,
        claimed: soroban_sdk::Map::new(&env),
        created_at: env.ledger().timestamp(),
        updated_at: env.ledger().timestamp(),
        fee_collected: false,
        resolution_duration: 3600,
        dispute_window_seconds: 3600,
        extensions_count: 0,
        metadata: None,
        tags: vec![],
    };

    let contract_id = env.register(crate::PredictifyHybrid, ());
    env.as_contract(&contract_id, || {
        crate::markets::MarketStateManager::update_market(&env, &market_id, &market);
        
        // Also set admin in global config
        let config = crate::types::GlobalConfig {
            admin: admin.clone(),
            fee_address: Address::generate(&env),
            fee_percent: 1,
            creation_fee: 10,
            paused: false,
        };
        env.storage().persistent().set(&crate::storage::DataKey::GlobalConfig, &config);
    });

    (env, admin, market_id)
}

#[test]
fn test_anti_grief_floor() {
    let (env, admin, market_id) = setup_env_and_market();
    let user = Address::generate(&env);
    
    env.mock_all_auths();
    
    // Give user balance
    env.as_contract(&env.register(crate::PredictifyHybrid, ()), || {
        crate::storage::BalanceStorage::add_balance(&env, &user, &crate::types::ReflectorAsset::Stellar, 1000).unwrap();
    });

    let contract_id = env.register(crate::PredictifyHybrid, ());
    env.as_contract(&contract_id, || {
        // Set anti-grief floor to 500
        DisputeManager::set_anti_grief_floor(&env, admin.clone(), 500).unwrap();
        assert_eq!(DisputeManager::get_anti_grief_floor(&env), Some(500));

        // Attempt to file dispute with stake 100 (below floor) -> should fail
        let err = DisputeManager::process_dispute(&env, user.clone(), market_id.clone(), 100, None).unwrap_err();
        assert_eq!(err, Error::InvalidStakeAmount);

        // Attempt to file dispute with stake 500 (at floor) -> should succeed
        DisputeManager::process_dispute(&env, user.clone(), market_id.clone(), 500, None).unwrap();
    });
}
