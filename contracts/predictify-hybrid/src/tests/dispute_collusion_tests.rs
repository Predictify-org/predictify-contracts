#![cfg(test)]
extern crate std;

use alloc::vec;
use soroban_sdk::{
    testutils::Address as _,
    Address, Env, String, Symbol,
};

use crate::{
    disputes::{DisputeManager, CollusionDetectorConfig},
    types::{Market, MarketState, OracleConfig, GlobalConfig},
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
        
        let config = GlobalConfig {
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
fn test_collusion_detector() {
    let (env, admin, market_id) = setup_env_and_market();
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let user3 = Address::generate(&env);
    
    env.mock_all_auths();
    
    let contract_id = env.register(crate::PredictifyHybrid, ());
    
    env.as_contract(&contract_id, || {
        crate::storage::BalanceStorage::add_balance(&env, &user1, &crate::types::ReflectorAsset::Stellar, 10_000).unwrap();
        crate::storage::BalanceStorage::add_balance(&env, &user2, &crate::types::ReflectorAsset::Stellar, 10_000).unwrap();
        crate::storage::BalanceStorage::add_balance(&env, &user3, &crate::types::ReflectorAsset::Stellar, 10_000).unwrap();

        // Configure the detector
        let config = CollusionDetectorConfig {
            stake_delta_threshold: 100,
            time_delta_threshold: 60,
            window_size: 8,
        };
        DisputeManager::set_collusion_detector_config(&env, admin.clone(), config).unwrap();

        // Dispute 1: user1 stakes 1000
        env.ledger().with_mut(|l| l.timestamp = 1000);
        DisputeManager::process_dispute(&env, user1.clone(), market_id.clone(), 1000, None).unwrap();

        // Dispute 2: user2 stakes 1050 (stake_delta=50, time_delta=10) -> SHOULD FIRE
        env.ledger().with_mut(|l| l.timestamp = 1010);
        DisputeManager::process_dispute(&env, user2.clone(), market_id.clone(), 1050, None).unwrap();

        // Dispute 3: user3 stakes 2000 (stake_delta=950, time_delta=20) -> SHOULD BE SUPPRESSED
        env.ledger().with_mut(|l| l.timestamp = 1030);
        DisputeManager::process_dispute(&env, user3.clone(), market_id.clone(), 2000, None).unwrap();
    });

    // We verify the events.
    let events = env.events().all();
    let mut collision_flags_count = 0;

    for (contract, topic, event_val) in events.iter() {
        if let Ok(topic_vec) = soroban_sdk::Vec::<Symbol>::try_from_val(&env, &topic) {
            if topic_vec.len() > 0 && topic_vec.get(0).unwrap() == Symbol::new(&env, "sus_col") {
                collision_flags_count += 1;
                
                let event: crate::events::SuspectedCollusionFlagEvent = event_val.try_into_val(&env).unwrap();
                
                // Assert the details of the expected flag
                assert_eq!(event.user1, user2);
                assert_eq!(event.user2, user1);
                assert_eq!(event.stake_delta, 50);
                assert_eq!(event.time_delta, 10);
            }
        }
    }

    assert_eq!(collision_flags_count, 1, "Exactly one collision flag should have been emitted");
}
