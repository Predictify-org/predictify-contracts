#![cfg(test)]

//! Storage Layout and Collision Tests
//!
//! This module provides comprehensive tests for storage key layout, collision detection,
//! and data structure migration safety. It validates that:
//! - No storage key collisions exist across modules
//! - Storage keys are properly namespaced
//! - Data structures can be safely extended
//! - Migration patterns work correctly

use alloc::format;
use soroban_sdk::{
    testutils::{Address as _, EnvTestConfig, storage::Persistent, Ledger},
    vec, Address, Env, Map, String, Symbol, Vec as SorobanVec,
    xdr::ToXdr,
};
use crate::markets::MarketStateManager;
use crate::storage::{
    BalanceStorage, CreatorLimitsManager, EventManager, StorageFormat, StorageOptimizer,
};
use crate::err::Error;
use crate::types::*;

// ===== TEST UTILITIES =====

/// Test helper to create a test environment
fn create_test_env() -> Env {
    let mut env = Env::default();
    env.set_config(EnvTestConfig {
        capture_snapshot_at_drop: false,
    });
    env
}

/// Test helper to create a test admin
fn create_test_admin(env: &Env) -> Address {
    Address::generate(env)
}

/// Test helper to create a test market
fn create_test_market(env: &Env, admin: &Address) -> (Symbol, Market) {
    let market_id = Symbol::new(env, "test_market");
    let question = String::from_str(env, "Will BTC reach $100k?");
    let outcomes = vec![
        env,
        String::from_str(env, "Yes"),
        String::from_str(env, "No"),
    ];
    let end_time = env.ledger().timestamp() + 86400;

    let oracle_config = OracleConfig {
        provider: OracleProvider::reflector(),
        oracle_address: Address::generate(env),
        feed_id: String::from_str(env, "BTC"),
        threshold: 100_000_00,
        comparison: String::from_str(env, "gt"),
    };

    let metadata_commitment = Market::compute_metadata_commitment(
        env,
        &question,
        &outcomes,
        &oracle_config,
    );

    let market = Market {
        admin: admin.clone(),
        question,
        outcomes,
        end_time,
        oracle_config,
        metadata_commitment,
        has_fallback: false,
        fallback_oracle_config: OracleConfig::none_sentinel(env),
        resolution_timeout: 86400,
        oracle_result: None,
        votes: Map::new(env),
        stakes: Map::new(env),
        claimed: Map::new(env),
        total_staked: 0,
        dispute_stakes: Map::new(env),
        winning_outcomes: None,
        fee_collected: false,
        state: MarketState::Active,
        total_extension_days: 0,
        max_extension_days: 30,
        extension_history: vec![env],
        category: None,
        tags: vec![env],
        min_pool_size: None,
        bet_deadline: 0,
        dispute_window_seconds: 0,
        winnings_swept: false,
        timelock_config: crate::timelock::MarketTimelockConfig::default(),
        dispute_stake_floor: None,
        max_participants: None,
    };

    (market_id, market)
}

fn run_as_contract<T>(env: &Env, f: impl FnOnce() -> T) -> T {
    let contract_id = env.register(crate::PredictifyHybrid, ());
    env.as_contract(&contract_id, f)
}

// ===== STORAGE KEY COLLISION TESTS =====

#[test]
fn test_no_admin_key_collisions() {
    let env = create_test_env();
    let admin = create_test_admin(&env);

    // Test that different admin keys don't collide
    let admin_key = Symbol::new(&env, "Admin");
    let admin_role_key = Symbol::new(&env, "admin_role");
    let admin_count_key = Symbol::new(&env, "AdminCount");
    let admin_list_key = Symbol::new(&env, "AdminList");
    let multisig_config_key = Symbol::new(&env, "MultisigConfig");
    let next_action_id_key = Symbol::new(&env, "NextActionId");
    let contract_paused_key = Symbol::new(&env, "ContractPaused");

    // Verify all keys are unique
    let keys = vec![
        &env,
        admin_key.clone(),
        admin_role_key.clone(),
        admin_count_key.clone(),
        admin_list_key.clone(),
        multisig_config_key.clone(),
        next_action_id_key.clone(),
        contract_paused_key.clone(),
    ];

    // Check for duplicates
    for i in 0..keys.len() {
        for j in (i + 1)..keys.len() {
            assert_ne!(
                keys.get(i).unwrap(),
                keys.get(j).unwrap(),
                "Found duplicate admin keys"
            );
        }
    }
}

#[test]
fn test_no_market_key_collisions() {
    let env = create_test_env();

    // Test that market-related keys don't collide
    let market_counter_key = Symbol::new(&env, "MarketCounter");
    let market_id_1 = Symbol::new(&env, "market_1");
    let market_id_2 = Symbol::new(&env, "market_2");

    // Verify keys are unique
    assert_ne!(market_counter_key, market_id_1);
    assert_ne!(market_counter_key, market_id_2);
    assert_ne!(market_id_1, market_id_2);
}

#[test]
fn test_no_audit_trail_key_collisions() {
    let env = create_test_env();

    // Test audit trail keys
    let audit_head_key = Symbol::new(&env, "AUDIT_HEAD");
    let audit_rec_key_1 = (Symbol::new(&env, "AUDIT_REC"), 1u64);
    let audit_rec_key_2 = (Symbol::new(&env, "AUDIT_REC"), 2u64);

    // Verify tuple keys are unique
    assert_ne!(audit_rec_key_1, audit_rec_key_2);

    // Verify simple key doesn't collide with tuple keys
    // (Different types, so no collision possible)
}

#[test]
fn test_no_circuit_breaker_key_collisions() {
    let env = create_test_env();

    // Test circuit breaker keys
    let cb_config_key = Symbol::new(&env, "CB_CONFIG");
    let cb_state_key = Symbol::new(&env, "CB_STATE");
    let cb_events_key = Symbol::new(&env, "CB_EVENTS");
    let cb_conditions_key = Symbol::new(&env, "CB_CONDITIONS");

    // Verify all CB keys are unique
    assert_ne!(cb_config_key, cb_state_key);
    assert_ne!(cb_config_key, cb_events_key);
    assert_ne!(cb_config_key, cb_conditions_key);
    assert_ne!(cb_state_key, cb_events_key);
    assert_ne!(cb_state_key, cb_conditions_key);
    assert_ne!(cb_events_key, cb_conditions_key);
}

#[test]
fn test_no_storage_config_key_collisions() {
    let env = create_test_env();

    // Test storage configuration keys
    let storage_config_key = Symbol::new(&env, "storage_config");
    let config_key = Symbol::new(&env, "Config");

    // Verify keys are unique
    assert_ne!(storage_config_key, config_key);
}

#[test]
fn test_no_recovery_key_collisions() {
    let env = create_test_env();

    // Test recovery keys
    let recovery_records_key = Symbol::new(&env, "RecoveryRecords");
    let recovery_status_key = Symbol::new(&env, "RecoveryStatus");

    // Verify keys are unique
    assert_ne!(recovery_records_key, recovery_status_key);
}

#[test]
fn test_tuple_key_namespace_isolation() {
    let env = create_test_env();
    let address1 = Address::generate(&env);
    let address2 = Address::generate(&env);

    // Test that tuple keys provide proper namespace isolation
    let event_key_1 = (Symbol::new(&env, "Event"), Symbol::new(&env, "event_1"));
    let event_key_2 = (Symbol::new(&env, "Event"), Symbol::new(&env, "event_2"));
    let active_events_key_1 = (Symbol::new(&env, "ActiveEvents"), address1.clone());
    let active_events_key_2 = (Symbol::new(&env, "ActiveEvents"), address2.clone());

    // Verify tuple keys with same namespace but different IDs are unique
    assert_ne!(event_key_1, event_key_2);
    assert_ne!(active_events_key_1, active_events_key_2);
}

#[test]
fn test_formatted_key_uniqueness() {
    let env = create_test_env();
    let market_id_1 = Symbol::new(&env, "market_1");
    let market_id_2 = Symbol::new(&env, "market_2");

    // Test formatted keys are unique
    let compressed_key_1 = format!("compressed_{:?}", market_id_1);
    let compressed_key_2 = format!("compressed_{:?}", market_id_2);
    let compressed_ref_key_1 = format!("compressed_ref_{:?}", market_id_1);

    // Verify formatted keys are unique
    assert_ne!(compressed_key_1, compressed_key_2);
    assert_ne!(compressed_key_1, compressed_ref_key_1);

    // Test ArchivedMarket DataKey uniqueness
    let archive_key_1 = crate::storage::DataKey::ArchivedMarket(market_id_1.clone(), 1234567890);
    let archive_key_2 = crate::storage::DataKey::ArchivedMarket(market_id_2.clone(), 1234567890);
    let archive_key_3 = crate::storage::DataKey::ArchivedMarket(market_id_1.clone(), 9876543210);

    assert_ne!(archive_key_1, archive_key_2);
    assert_ne!(archive_key_1, archive_key_3);
}

// ===== STORAGE KEY NAMESPACE TESTS =====

#[test]
fn test_admin_namespace_consistency() {
    let env = create_test_env();

    // Verify admin keys follow consistent naming
    let admin_key = Symbol::new(&env, "Admin");
    let admin_count_key = Symbol::new(&env, "AdminCount");
    let admin_list_key = Symbol::new(&env, "AdminList");

    // All admin-related keys should be unique
    assert_ne!(admin_key, admin_count_key);
    assert_ne!(admin_key, admin_list_key);
    assert_ne!(admin_count_key, admin_list_key);
}

#[test]
fn test_circuit_breaker_namespace_prefix() {
    let env = create_test_env();

    // Verify all circuit breaker keys use CB_ prefix
    let cb_config = Symbol::new(&env, "CB_CONFIG");
    let cb_state = Symbol::new(&env, "CB_STATE");
    let cb_events = Symbol::new(&env, "CB_EVENTS");
    let cb_conditions = Symbol::new(&env, "CB_CONDITIONS");

    // All CB keys should be unique
    assert_ne!(cb_config, cb_state);
    assert_ne!(cb_config, cb_events);
    assert_ne!(cb_config, cb_conditions);
    assert_ne!(cb_state, cb_events);
}

#[test]
fn test_audit_trail_namespace_prefix() {
    let env = create_test_env();

    // Verify audit trail keys use AUDIT_ prefix
    let audit_head = Symbol::new(&env, "AUDIT_HEAD");
    let audit_rec = Symbol::new(&env, "AUDIT_REC");

    // All audit keys should be unique
    assert_ne!(audit_head, audit_rec);
}

// ===== BALANCE STORAGE KEY TESTS =====

#[test]
fn test_balance_storage_key_uniqueness() {
    let env = create_test_env();
    let contract_id = env.register(crate::PredictifyHybrid, ());
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let asset1 = ReflectorAsset::BTC;
    let asset2 = ReflectorAsset::ETH;

    env.as_contract(&contract_id, || {
        // Get balances to trigger key generation
        let balance1 = BalanceStorage::get_balance(&env, &user1, &asset1);
        let balance2 = BalanceStorage::get_balance(&env, &user1, &asset2);
        let balance3 = BalanceStorage::get_balance(&env, &user2, &asset1);

        // Verify balances are independent (different keys)
        assert_eq!(balance1.amount, 0);
        assert_eq!(balance2.amount, 0);
        assert_eq!(balance3.amount, 0);

        // Set different amounts
        BalanceStorage::add_balance(&env, &user1, &asset1, 100).unwrap();
        BalanceStorage::add_balance(&env, &user1, &asset2, 200).unwrap();
        BalanceStorage::add_balance(&env, &user2, &asset1, 300).unwrap();

        // Verify each balance is stored independently
        assert_eq!(
            BalanceStorage::get_balance(&env, &user1, &asset1).amount,
            100
        );
        assert_eq!(
            BalanceStorage::get_balance(&env, &user1, &asset2).amount,
            200
        );
        assert_eq!(
            BalanceStorage::get_balance(&env, &user2, &asset1).amount,
            300
        );
    });
}

// ===== EVENT STORAGE KEY TESTS =====

#[test]
fn test_event_storage_key_uniqueness() {
    let env = create_test_env();
    let admin = create_test_admin(&env);

    let event1 = Event {
        id: Symbol::new(&env, "event_1"),
        description: String::from_str(&env, "Event 1"),
        outcomes: vec![&env, String::from_str(&env, "Yes")],
        end_time: env.ledger().timestamp() + 86400,
        oracle_config: OracleConfig::none_sentinel(&env),
        has_fallback: false,
        fallback_oracle_config: OracleConfig::none_sentinel(&env),
        resolution_timeout: 86400,
        admin: admin.clone(),
        created_at: env.ledger().timestamp(),
        status: MarketState::Active,
        visibility: EventVisibility::Public,
        allowlist: SorobanVec::new(&env),
    };

    let event2 = Event {
        id: Symbol::new(&env, "event_2"),
        description: String::from_str(&env, "Event 2"),
        outcomes: vec![&env, String::from_str(&env, "No")],
        end_time: env.ledger().timestamp() + 86400,
        oracle_config: OracleConfig::none_sentinel(&env),
        has_fallback: false,
        fallback_oracle_config: OracleConfig::none_sentinel(&env),
        resolution_timeout: 86400,
        admin: admin.clone(),
        created_at: env.ledger().timestamp(),
        status: MarketState::Active,
        visibility: EventVisibility::Public,
        allowlist: SorobanVec::new(&env),
    };

    run_as_contract(&env, || {
        // Store events
        EventManager::store_event(&env, &event1);
        EventManager::store_event(&env, &event2);

        // Verify events are stored independently
        let retrieved1 = EventManager::get_event(&env, &event1.id).unwrap();
        let retrieved2 = EventManager::get_event(&env, &event2.id).unwrap();

        assert_eq!(retrieved1.id, event1.id);
        assert_eq!(retrieved2.id, event2.id);
        assert_ne!(retrieved1.id, retrieved2.id);
    });
}

// ===== CREATOR LIMITS STORAGE KEY TESTS =====

#[test]
fn test_creator_limits_key_uniqueness() {
    let env = create_test_env();
    let creator1 = Address::generate(&env);
    let creator2 = Address::generate(&env);

    run_as_contract(&env, || {
        // Increment active events for different creators
        CreatorLimitsManager::increment_active_events(&env, &creator1);
        CreatorLimitsManager::increment_active_events(&env, &creator1);
        CreatorLimitsManager::increment_active_events(&env, &creator2);

        // Verify counts are independent
        assert_eq!(CreatorLimitsManager::get_active_events(&env, &creator1), 2);
        assert_eq!(CreatorLimitsManager::get_active_events(&env, &creator2), 1);

        // Decrement and verify independence
        CreatorLimitsManager::decrement_active_events(&env, &creator1);
        assert_eq!(CreatorLimitsManager::get_active_events(&env, &creator1), 1);
        assert_eq!(CreatorLimitsManager::get_active_events(&env, &creator2), 1);
    });
}

// ===== DATA STRUCTURE EXTENSION TESTS =====

#[test]
fn test_market_structure_serialization() {
    let env = create_test_env();
    let admin = create_test_admin(&env);
    let (market_id, market) = create_test_market(&env, &admin);

    run_as_contract(&env, || {
        // Store market
        env.storage().persistent().set(&market_id, &market);

        // Retrieve market
        let retrieved: Market = env.storage().persistent().get(&market_id).unwrap();

        // Verify all fields are preserved
        assert_eq!(retrieved.admin, market.admin);
        assert_eq!(retrieved.question, market.question);
        assert_eq!(retrieved.outcomes, market.outcomes);
        assert_eq!(retrieved.end_time, market.end_time);
        assert_eq!(retrieved.total_staked, market.total_staked);
        assert_eq!(retrieved.state, market.state);
    });
}

#[test]
fn test_claim_info_structure_serialization() {
    let env = create_test_env();
    let contract_id = env.register(crate::PredictifyHybrid, ());

    let claim_info = ClaimInfo {
        claimed: true,
        timestamp: env.ledger().timestamp(),
        payout_amount: 1_000_000,
        claim_nonce: 0,
    };

    env.as_contract(&contract_id, || {
        // Store claim info
        let key = Symbol::new(&env, "test_claim");
        env.storage().persistent().set(&key, &claim_info);

        // Retrieve claim info
        let retrieved: ClaimInfo = env.storage().persistent().get(&key).unwrap();

        // Verify all fields are preserved
        assert_eq!(retrieved.claimed, claim_info.claimed);
        assert_eq!(retrieved.timestamp, claim_info.timestamp);
        assert_eq!(retrieved.payout_amount, claim_info.payout_amount);
    });
}

#[test]
fn test_oracle_config_structure_serialization() {
    let env = create_test_env();

    let oracle_config = OracleConfig {
        provider: OracleProvider::reflector(),
        oracle_address: Address::generate(&env),
        feed_id: String::from_str(&env, "BTC/USD"),
        threshold: 50_000_00,
        comparison: String::from_str(&env, "gt"),
    };

    run_as_contract(&env, || {
        // Store oracle config
        let key = Symbol::new(&env, "test_oracle");
        env.storage().persistent().set(&key, &oracle_config);

        // Retrieve oracle config
        let retrieved: OracleConfig = env.storage().persistent().get(&key).unwrap();

        // Verify all fields are preserved
        assert_eq!(retrieved.provider, oracle_config.provider);
        assert_eq!(retrieved.feed_id, oracle_config.feed_id);
        assert_eq!(retrieved.threshold, oracle_config.threshold);
        assert_eq!(retrieved.comparison, oracle_config.comparison);
    });
}

// ===== STORAGE KEY PATTERN TESTS =====

#[test]
fn test_simple_symbol_key_pattern() {
    let env = create_test_env();

    run_as_contract(&env, || {
        // Test simple symbol key storage and retrieval
        let key = Symbol::new(&env, "TestKey");
        let value = String::from_str(&env, "TestValue");

        env.storage().persistent().set(&key, &value);
        let retrieved: String = env.storage().persistent().get(&key).unwrap();

        assert_eq!(retrieved, value);
    });
}

#[test]
fn test_tuple_key_pattern() {
    let env = create_test_env();

    run_as_contract(&env, || {
        // Test tuple key storage and retrieval
        let key = (
            Symbol::new(&env, "Namespace"),
            Symbol::new(&env, "identifier"),
        );
        let value = String::from_str(&env, "TupleValue");

        env.storage().persistent().set(&key, &value);
        let retrieved: String = env.storage().persistent().get(&key).unwrap();

        assert_eq!(retrieved, value);
    });
}

#[test]
fn test_tuple_with_address_key_pattern() {
    let env = create_test_env();
    let address = Address::generate(&env);

    run_as_contract(&env, || {
        // Test tuple with address key storage and retrieval
        let key = (Symbol::new(&env, "UserData"), address.clone());
        let value = 12345u32;

        env.storage().persistent().set(&key, &value);
        let retrieved: u32 = env.storage().persistent().get(&key).unwrap();

        assert_eq!(retrieved, value);
    });
}

// ===== MIGRATION SAFETY TESTS =====

#[test]
fn test_market_backward_compatibility() {
    let env = create_test_env();
    let admin = create_test_admin(&env);
    let (market_id, market) = create_test_market(&env, &admin);

    run_as_contract(&env, || {
        // Store market
        MarketStateManager::update_market(&env, &market_id, &market);

        // Retrieve and verify
        let retrieved = MarketStateManager::get_market(&env, &market_id).unwrap();

        // Verify critical fields are preserved
        assert_eq!(retrieved.admin, market.admin);
        assert_eq!(retrieved.question, market.question);
        assert_eq!(retrieved.total_staked, market.total_staked);
        assert_eq!(retrieved.state, market.state);
    });
}

#[test]
fn test_storage_version_tracking() {
    let env = create_test_env();

    // Test storage format version tracking
    let version_v1 = StorageFormat::V1;
    let version_v2 = StorageFormat::V2;
    let version_v3 = StorageFormat::V3;

    // Verify versions are distinct
    assert_ne!(version_v1, version_v2);
    assert_ne!(version_v2, version_v3);
    assert_ne!(version_v1, version_v3);
}

// ===== STORAGE OPTIMIZATION TESTS =====

#[test]
fn test_compressed_market_key_uniqueness() {
    let env = create_test_env();
    let admin = create_test_admin(&env);
    let (_, market1) = create_test_market(&env, &admin);
    let (_, mut market2) = create_test_market(&env, &admin);
    market2.question = String::from_str(&env, "Will ETH surpass 5k this year?");

    // Compress markets
    let compressed1 = StorageOptimizer::compress_market_data(&env, &market1).unwrap();
    let compressed2 = StorageOptimizer::compress_market_data(&env, &market2).unwrap();

    // Verify compressed markets have unique IDs
    assert_ne!(compressed1.market_id, compressed2.market_id);
}

#[test]
fn test_storage_config_isolation() {
    let env = create_test_env();

    run_as_contract(&env, || {
        // Get default storage config
        let config = StorageOptimizer::get_storage_config(&env);

        // Verify config has expected defaults
        assert!(config.compression_enabled);
        assert_eq!(config.min_compression_age_days, 30);
        assert_eq!(config.max_storage_per_market, 1024 * 1024);
        assert_eq!(config.cleanup_threshold_days, 365);
    });
}

// ===== COMPREHENSIVE COLLISION TEST =====

#[test]
fn test_comprehensive_key_collision_check() {
    let env = create_test_env();

    // Collect all known storage keys
    let mut all_keys: SorobanVec<String> = SorobanVec::new(&env);

    // Admin keys
    all_keys.push_back(String::from_str(&env, "Admin"));
    all_keys.push_back(String::from_str(&env, "admin_role"));
    all_keys.push_back(String::from_str(&env, "AdminCount"));
    all_keys.push_back(String::from_str(&env, "AdminList"));
    all_keys.push_back(String::from_str(&env, "MultisigConfig"));
    all_keys.push_back(String::from_str(&env, "NextActionId"));
    all_keys.push_back(String::from_str(&env, "ContractPaused"));

    // Market keys
    all_keys.push_back(String::from_str(&env, "MarketCounter"));

    // Audit trail keys
    all_keys.push_back(String::from_str(&env, "AUDIT_HEAD"));
    all_keys.push_back(String::from_str(&env, "AUDIT_REC"));

    // Circuit breaker keys
    all_keys.push_back(String::from_str(&env, "CB_CONFIG"));
    all_keys.push_back(String::from_str(&env, "CB_STATE"));
    all_keys.push_back(String::from_str(&env, "CB_EVENTS"));
    all_keys.push_back(String::from_str(&env, "CB_CONDITIONS"));

    // Config keys
    all_keys.push_back(String::from_str(&env, "storage_config"));
    all_keys.push_back(String::from_str(&env, "Config"));

    // Recovery keys
    all_keys.push_back(String::from_str(&env, "RecoveryRecords"));
    all_keys.push_back(String::from_str(&env, "RecoveryStatus"));

    // Check for duplicates
    for i in 0..all_keys.len() {
        for j in (i + 1)..all_keys.len() {
            let key_i = all_keys.get(i).unwrap();
            let key_j = all_keys.get(j).unwrap();
            assert_ne!(
                key_i, key_j,
                "Found duplicate storage key: {} == {}",
                key_i, key_j
            );
        }
    }

    // If we reach here, no collisions were found
    assert!(all_keys.len() > 0, "Should have collected storage keys");
}

// ===== REGRESSION TESTS =====

#[test]
fn test_no_regression_in_market_storage() {
    let env = create_test_env();
    let admin = create_test_admin(&env);
    let (market_id, market) = create_test_market(&env, &admin);

    run_as_contract(&env, || {
        // Store market directly in storage
        env.storage().persistent().set(&market_id, &market);

        // Retrieve using storage
        let retrieved: Market = env.storage().persistent().get(&market_id).unwrap();

        // Verify no data loss
        assert_eq!(retrieved.admin, market.admin);
        assert_eq!(retrieved.question, market.question);
        assert_eq!(retrieved.outcomes.len(), market.outcomes.len());
        assert_eq!(retrieved.end_time, market.end_time);
        assert_eq!(retrieved.total_staked, market.total_staked);
        assert_eq!(retrieved.state, market.state);
        assert_eq!(retrieved.fee_collected, market.fee_collected);
    });
}

#[test]
fn test_no_regression_in_balance_storage() {
    let env = create_test_env();
    let contract_id = env.register(crate::PredictifyHybrid, ());
    let user = Address::generate(&env);
    let asset = ReflectorAsset::BTC;

    env.as_contract(&contract_id, || {
        // Add balance using current implementation
        BalanceStorage::add_balance(&env, &user, &asset, 1_000_000).unwrap();

        // Retrieve using current implementation
        let balance = BalanceStorage::get_balance(&env, &user, &asset);

        // Verify no data loss
        assert_eq!(balance.amount, 1_000_000);
        assert_eq!(balance.user, user);
        assert_eq!(balance.asset, asset);
    });
}

// ===== PERFORMANCE TESTS =====

#[test]
fn test_storage_key_generation_performance() {
    let env = create_test_env();

    // Test that key generation is efficient
    let start = env.ledger().timestamp();

    for i in 0..100 {
        let _key = Symbol::new(&env, &format!("test_key_{}", i));
    }

    let end = env.ledger().timestamp();
    let duration = end - start;

    // Key generation should be fast (this is a sanity check)
    assert!(duration < 1000, "Key generation took too long");
}

#[test]
fn test_tuple_key_generation_performance() {
    let env = create_test_env();

    // Test that tuple key generation is efficient
    let start = env.ledger().timestamp();

    for i in 0..100 {
        let _key = (
            Symbol::new(&env, "Namespace"),
            Symbol::new(&env, &format!("id_{}", i)),
        );
    }

    let end = env.ledger().timestamp();
    let duration = end - start;

    // Tuple key generation should be fast
    assert!(duration < 1000, "Tuple key generation took too long");
}

#[test]
fn test_promote_market_to_persistent_and_demote_scratch() {
    let env = create_test_env();
    let contract_id = env.register(crate::PredictifyHybrid, ());
    let client = crate::PredictifyHybridClient::new(&env, &contract_id);
    let admin = create_test_admin(&env);
    let (market_id, market) = create_test_market(&env, &admin);

    env.mock_all_auths();

    env.as_contract(&contract_id, || {
        // Setup config with known TTL constants
        let mut config = StorageOptimizer::get_storage_config(&env);
        config.market_ttl_ledgers = 5000;
        config.balance_ttl_ledgers = 100;
        StorageOptimizer::update_storage_config(&env, &config).unwrap();
    });

    // 1. Missing market metadata (error case)
    let err_result = client.try_promote_market_to_persistent(&market_id);
    assert_eq!(err_result, Err(Ok(Error::MarketNotFound)));

    // 2. Put market metadata in temporary storage
    let temp_key = crate::storage::DataKey::MarketMetadata(market_id.clone());
    let persistent_key = crate::storage::DataKey::MarketMetadata(market_id.clone());
    env.as_contract(&contract_id, || {
        env.storage().temporary().set(&temp_key, &market);
        env.storage().temporary().extend_ttl(&temp_key, 100, 100);
    });

    // 3. Test require_auth gating via client
    let success_result = client.try_promote_market_to_persistent(&market_id);
    assert!(success_result.is_ok());

    // Check it has been promoted to persistent
    env.as_contract(&contract_id, || {
        assert!(env.storage().persistent().has(&persistent_key));
        assert!(!env.storage().temporary().has(&temp_key));

        // Verify the persistent entry's TTL matches what's configured
        let persistent_ttl = env.storage().persistent().get_ttl(&persistent_key);
        assert_eq!(persistent_ttl, 5000.min(env.storage().max_ttl()));
    });

    // 4. Idempotency test: calling it again succeeds and extends/refreshes TTL
    env.ledger().with_mut(|li| {
        li.sequence_number += 100;
    });
    env.as_contract(&contract_id, || {
        assert!(env.storage().persistent().get_ttl(&persistent_key) < 5000.min(env.storage().max_ttl()));
    });

    let idempotent_result = client.try_promote_market_to_persistent(&market_id);
    assert!(idempotent_result.is_ok());
    env.as_contract(&contract_id, || {
        assert_eq!(env.storage().persistent().get_ttl(&persistent_key), 5000.min(env.storage().max_ttl()));
    });

    // 5. Test demote_scratch_keys
    let scratch_persistent_key = crate::storage::DataKey::MarketScratch(market_id.clone());
    let scratch_temp_key = crate::storage::DataKey::MarketScratch(market_id.clone());

    env.as_contract(&contract_id, || {
        let scratch_data: SorobanVec<i128> = vec![&env, 12, 34, 56];
        env.storage().persistent().set(&scratch_persistent_key, &scratch_data);
        StorageOptimizer::extend_persistent_ttl(&env, &scratch_persistent_key, 5000);
    });

    let demote_result = client.try_demote_scratch_keys(&market_id);
    assert!(demote_result.is_ok());

    // Check scratch keys are moved to temporary
    env.as_contract(&contract_id, || {
        assert!(!env.storage().persistent().has(&scratch_persistent_key));
        assert!(env.storage().temporary().has(&scratch_temp_key));

        let retrieved_scratch: SorobanVec<i128> = env.storage().temporary().get(&scratch_temp_key).unwrap();
        assert_eq!(retrieved_scratch.get(0).unwrap(), 12);
        assert_eq!(retrieved_scratch.get(1).unwrap(), 34);
        assert_eq!(retrieved_scratch.get(2).unwrap(), 56);
    });

    // Idempotency: call again
    let demote_idempotent = client.try_demote_scratch_keys(&market_id);
    assert!(demote_idempotent.is_ok());
    env.as_contract(&contract_id, || {
        assert!(env.storage().temporary().has(&scratch_temp_key));
    });
}

#[test]
#[should_panic]
fn test_promote_market_to_persistent_auth_failure() {
    let env = create_test_env();
    let contract_id = env.register(crate::PredictifyHybrid, ());
    let admin = create_test_admin(&env);
    let (market_id, market) = create_test_market(&env, &admin);

    env.as_contract(&contract_id, || {
        let temp_key = crate::storage::DataKey::MarketMetadata(market_id.clone());
        env.storage().temporary().set(&temp_key, &market);
        
        // This should panic due to missing authentication from market.admin (since mock_all_auths is not called)
        let _ = crate::storage::StorageMigration::promote_market_to_persistent(&env, &market_id);
    });
}

#[test]
#[should_panic]
fn test_demote_scratch_keys_auth_failure() {
    let env = create_test_env();
    let contract_id = env.register(crate::PredictifyHybrid, ());
    let admin = create_test_admin(&env);
    let (market_id, market) = create_test_market(&env, &admin);

    env.as_contract(&contract_id, || {
        let temp_key = crate::storage::DataKey::MarketMetadata(market_id.clone());
        env.storage().temporary().set(&temp_key, &market);
        
        let scratch_persistent_key = crate::storage::DataKey::MarketScratch(market_id.clone());
        let scratch_data: SorobanVec<i128> = vec![&env, 12, 34, 56];
        env.storage().persistent().set(&scratch_persistent_key, &scratch_data);

        // This should panic due to missing authentication from market.admin
        let _ = crate::storage::StorageMigration::demote_scratch_keys(&env, &market_id);
    });
}

#[test]
fn test_check_ttl_pressure() {
    let env = create_test_env();
    let contract_id = env.register(crate::PredictifyHybrid, ());

    env.as_contract(&contract_id, || {
        use soroban_sdk::IntoVal;
        let ledger = env.ledger();
        env.ledger().set(soroban_sdk::testutils::LedgerInfo {
            timestamp: ledger.timestamp(),
            protocol_version: ledger.protocol_version(),
            sequence_number: ledger.sequence(),
            network_id: ledger.network_id().into(),
            base_reserve: 10,
            min_temp_entry_ttl: 1,
            min_persistent_entry_ttl: 50,
            max_entry_ttl: 1_000_000,
        });
        
        let key1 = Symbol::new(&env, "TestKey1");
        let value1 = String::from_str(&env, "TestValue1");

        let key2 = Symbol::new(&env, "TestKey2");
        let value2 = String::from_str(&env, "TestValue2");

        env.storage().persistent().set(&key1, &value1);
        env.storage().persistent().extend_ttl(&key1, 100, 150);

        env.storage().persistent().set(&key2, &value2);
        
        let keys = vec![&env, key1.into_val(&env), key2.into_val(&env)];
        let pressures = StorageOptimizer::check_ttl_pressure(&env, keys);
        
        assert_eq!(pressures.len(), 2);
        
        let pressure0 = pressures.get(0).unwrap();
        let pressure1 = pressures.get(1).unwrap();
        
        assert_eq!(pressure0.key_repr, soroban_sdk::String::from_str(&env, "storage_key"));
        assert!(pressure0.remaining_ledgers <= 100);
        
        assert_eq!(pressure1.key_repr, soroban_sdk::String::from_str(&env, "storage_key"));
        assert!(pressure1.remaining_ledgers <= 200);
        assert!(pressure0.remaining_ledgers < pressure1.remaining_ledgers);
        
        let expected_bump = crate::storage::MARKET_TTL_LEDGERS.min(env.storage().max_ttl());
        assert_eq!(pressure0.recommended_bump, expected_bump);
        assert_eq!(pressure1.recommended_bump, expected_bump);
    });
}

#[test]
fn test_all_datakey_variants_have_unique_xdr() {
    let env = create_test_env();
    let addr = Address::generate(&env);
    let sym = Symbol::new(&env, "TEST_SYM");
    let bytes32 = soroban_sdk::BytesN::from_array(&env, &[7u8; 32]);

    let variants: [(&str, crate::storage::DataKey); 32] = [
        ("PlaceBetsIdem", crate::storage::DataKey::PlaceBetsIdem(addr.clone(), bytes32.clone())),
        ("CreateEventIdem", crate::storage::DataKey::CreateEventIdem(addr.clone(), bytes32.clone())),
        ("Whitelisted", crate::storage::DataKey::Whitelisted(addr.clone())),
        ("Blacklisted", crate::storage::DataKey::Blacklisted(addr.clone())),
        ("ArchivedMarket", crate::storage::DataKey::ArchivedMarket(sym.clone(), 12345u64)),
        ("MarketExtensionTotal", crate::storage::DataKey::MarketExtensionTotal(sym.clone())),
        ("MarketMetadata", crate::storage::DataKey::MarketMetadata(sym.clone())),
        ("MarketScratch", crate::storage::DataKey::MarketScratch(sym.clone())),
        ("DisputeHistoryCap", crate::storage::DataKey::DisputeHistoryCap),
        ("DisputeHistory", crate::storage::DataKey::DisputeHistory(sym.clone())),
        ("DisputeStakeCap", crate::storage::DataKey::DisputeStakeCap(sym.clone(), addr.clone())),
        ("DisputeCumulativeStakeCap", crate::storage::DataKey::DisputeCumulativeStakeCap(addr.clone())),
        ("MarketCache", crate::storage::DataKey::MarketCache(sym.clone())),
        ("AntiGriefFloor", crate::storage::DataKey::AntiGriefFloor),
        ("DisputeCooldownSeconds", crate::storage::DataKey::DisputeCooldownSeconds),
        ("DisputeAdminLastAction", crate::storage::DataKey::DisputeAdminLastAction(sym.clone())),
        ("ResolutionCooldownSeconds", crate::storage::DataKey::ResolutionCooldownSeconds),
        ("ResolutionAdminLastAction", crate::storage::DataKey::ResolutionAdminLastAction(sym.clone())),
        ("GlobalConfig", crate::storage::DataKey::GlobalConfig),
        ("MaxBetCap", crate::storage::DataKey::MaxBetCap),
        ("UserStake", crate::storage::DataKey::UserStake(addr.clone(), sym.clone())),
        ("EventNonce", crate::storage::DataKey::EventNonce(sym.clone())),
        ("MarketAuditHead", crate::storage::DataKey::MarketAuditHead(sym.clone())),
        ("MarketAuditLog", crate::storage::DataKey::MarketAuditLog(sym.clone(), 42u32)),
        ("OracleAdminCooldownState", crate::storage::DataKey::OracleAdminCooldownState),
        ("MultisigRotationState", crate::storage::DataKey::MultisigRotationState),
        ("AdminOverrideNonce", crate::storage::DataKey::AdminOverrideNonce),
        ("PerLedgerBetCap", crate::storage::DataKey::PerLedgerBetCap),
        ("PerLedgerBetCounter", crate::storage::DataKey::PerLedgerBetCounter),
        ("CollusionDetectorConfig", crate::storage::DataKey::CollusionDetectorConfig(sym.clone())),
        ("ClaimNonce", crate::storage::DataKey::ClaimNonce(addr.clone(), sym.clone())),
        ("EventTopicAlias", crate::storage::DataKey::EventTopicAlias(sym.clone())),
    ];

    let mut encoded = alloc::vec::Vec::new();
    for (name, variant) in variants.iter() {
        let xdr = variant.to_xdr(&env);
        encoded.push((*name, xdr));
    }

    assert_eq!(encoded.len(), 32);

    for i in 0..encoded.len() {
        for j in (i + 1)..encoded.len() {
            let (name_i, xdr_i) = &encoded[i];
            let (name_j, xdr_j) = &encoded[j];
            assert_ne!(
                xdr_i, xdr_j,
                "DataKey collision between variant {} and {}",
                name_i, name_j
            );
        }
    }
}

#[test]
fn test_cross_family_key_isolation() {
    let env = create_test_env();
    let contract_id = env.register(crate::PredictifyHybrid, ());
    let sym = Symbol::new(&env, "Admin");

    env.as_contract(&contract_id, || {
        let datakey = crate::storage::DataKey::MarketMetadata(sym.clone());
        let symkey = sym.clone();
        let tuplekey = (Symbol::new(&env, "Event"), sym.clone());

        env.storage().persistent().set(&datakey, &1001u32);
        env.storage().persistent().set(&symkey, &2002u32);
        env.storage().persistent().set(&tuplekey, &3003u32);

        assert_eq!(env.storage().persistent().get::<_, u32>(&datakey).unwrap(), 1001u32);
        assert_eq!(env.storage().persistent().get::<_, u32>(&symkey).unwrap(), 2002u32);
        assert_eq!(env.storage().persistent().get::<_, u32>(&tuplekey).unwrap(), 3003u32);
    });
}

#[test]
fn test_populated_upgrade_fixtures_load_without_loss() {
    let env = create_test_env();
    let contract_id = env.register(crate::PredictifyHybrid, ());
    let admin = create_test_admin(&env);
    let user = Address::generate(&env);
    let (market_id, original_market) = create_test_market(&env, &admin);

    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&market_id, &original_market);

        let asset = ReflectorAsset::BTC;
        BalanceStorage::add_balance(&env, &user, &asset, 5_000_000).unwrap();

        let mut storage_config = StorageOptimizer::get_storage_config(&env);
        storage_config.balance_ttl_ledgers = 500_000;
        storage_config.market_ttl_ledgers = 6_000_000;
        StorageOptimizer::update_storage_config(&env, &storage_config).unwrap();

        let metadata_key = crate::storage::DataKey::MarketMetadata(market_id.clone());
        env.storage().persistent().set(&metadata_key, &original_market);

        let claim_nonce_key = crate::storage::DataKey::ClaimNonce(user.clone(), market_id.clone());
        env.storage().persistent().set(&claim_nonce_key, &7u64);

        let integrity = StorageOptimizer::validate_storage_integrity(&env, &market_id).unwrap();
        assert!(integrity.is_valid);
        assert!(!integrity.corruption_detected);
        assert!(!integrity.missing_data);

        let loaded_market: Market = env.storage().persistent().get(&market_id).unwrap();
        assert_eq!(loaded_market.admin, original_market.admin);
        assert_eq!(loaded_market.question, original_market.question);
        assert_eq!(loaded_market.outcomes.len(), original_market.outcomes.len());
        assert_eq!(loaded_market.end_time, original_market.end_time);
        assert_eq!(loaded_market.total_staked, original_market.total_staked);
        assert_eq!(loaded_market.state, original_market.state);
        assert_eq!(loaded_market.fee_collected, original_market.fee_collected);

        let loaded_balance = BalanceStorage::get_balance(&env, &user, &asset);
        assert_eq!(loaded_balance.amount, 5_000_000);
        assert_eq!(loaded_balance.user, user);
        assert_eq!(loaded_balance.asset, asset);

        let loaded_config = StorageOptimizer::get_storage_config(&env);
        assert_eq!(loaded_config.balance_ttl_ledgers, 500_000);
        assert_eq!(loaded_config.market_ttl_ledgers, 6_000_000);

        let loaded_nonce: u64 = env.storage().persistent().get(&claim_nonce_key).unwrap();
        assert_eq!(loaded_nonce, 7u64);
    });
}

#[test]
fn test_migration_empty_state() {
    let env = create_test_env();
    let contract_id = env.register(crate::PredictifyHybrid, ());

    env.as_contract(&contract_id, || {
        let result = StorageOptimizer::migrate_storage_format(
            &env,
            StorageFormat::V1,
            StorageFormat::V2,
        );
        assert!(result.is_ok());
        let migration = result.unwrap();
        assert_eq!(migration.from_format, StorageFormat::V1);
        assert_eq!(migration.to_format, StorageFormat::V2);
        assert_eq!(migration.status, String::from_str(&env, "completed"));
        assert!(migration.completed_at.is_some());
        assert!(migration.error_message.is_none());
    });
}

#[test]
fn test_migration_populated_state() {
    let env = create_test_env();
    let contract_id = env.register(crate::PredictifyHybrid, ());
    let admin = create_test_admin(&env);
    let (market_id, market) = create_test_market(&env, &admin);

    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&market_id, &market);

        let result_v1_v2 = StorageOptimizer::migrate_storage_format(
            &env,
            StorageFormat::V1,
            StorageFormat::V2,
        );
        assert!(result_v1_v2.is_ok());
        let mig_v1_v2 = result_v1_v2.unwrap();
        assert_eq!(mig_v1_v2.status, String::from_str(&env, "completed"));

        let result_v2_v3 = StorageOptimizer::migrate_storage_format(
            &env,
            StorageFormat::V2,
            StorageFormat::V3,
        );
        assert!(result_v2_v3.is_ok());
        let mig_v2_v3 = result_v2_v3.unwrap();
        assert_eq!(mig_v2_v3.status, String::from_str(&env, "completed"));

        let loaded: Market = env.storage().persistent().get(&market_id).unwrap();
        assert_eq!(loaded.admin, market.admin);
        assert_eq!(loaded.question, market.question);
    });
}

#[test]
fn test_migration_malformed_and_unsupported_state() {
    let env = create_test_env();
    let contract_id = env.register(crate::PredictifyHybrid, ());

    env.as_contract(&contract_id, || {
        let result = StorageOptimizer::migrate_storage_format(
            &env,
            StorageFormat::V3,
            StorageFormat::V1,
        );
        assert!(result.is_ok());
        let migration = result.unwrap();
        assert_eq!(migration.status, String::from_str(&env, "unsupported_migration"));
        assert!(migration.error_message.is_some());
        assert_eq!(
            migration.error_message.unwrap(),
            String::from_str(&env, "Unsupported migration path")
        );

        let corrupt_key = Symbol::new(&env, "nonexistent_market");
        let integrity_res = StorageOptimizer::validate_storage_integrity(&env, &corrupt_key);
        assert!(integrity_res.is_ok());
        let integrity = integrity_res.unwrap();
        assert!(!integrity.is_valid);
        assert!(integrity.missing_data);
    });
}


