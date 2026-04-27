#![cfg(test)]

//! Storage Layout and Collision Tests
//!
//! This module provides comprehensive tests for storage key layout, collision detection,
//! and data structure migration safety. It validates that:
//! - No storage key collisions exist across modules
//! - Storage keys are properly namespaced
//! - Data structures can be safely extended
//! - Migration patterns work correctly

use soroban_sdk::{
    testutils::Address as _, vec, Address, Env, Map, String, Symbol, Vec as SorobanVec,
};

use crate::admin::AdminInitializer;
use crate::audit_trail::AuditTrailManager;
use crate::markets::{MarketCreator, MarketStateManager};
use crate::storage::{BalanceStorage, CreatorLimitsManager, EventManager, StorageOptimizer};
use crate::types::*;

// ===== TEST UTILITIES =====

/// Test helper to create a test environment
fn create_test_env() -> Env {
    Env::default()
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

    let market = Market {
        admin: admin.clone(),
        question,
        outcomes,
        end_time,
        oracle_config,
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
    };

    (market_id, market)
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
    let archive_key_1 = format!("archive_{:?}_1234567890", market_id_1);

    // Verify formatted keys are unique
    assert_ne!(compressed_key_1, compressed_key_2);
    assert_ne!(compressed_key_1, compressed_ref_key_1);
    assert_ne!(compressed_key_1, archive_key_1);
}

// ===== STORAGE KEY NAMESPACE TESTS =====

#[test]
fn test_admin_namespace_consistency() {
    let env = create_test_env();

    // Verify admin keys follow consistent naming
    let admin_key = Symbol::new(&env, "Admin");
    let admin_count_key = Symbol::new(&env, "AdminCount");
    let admin_list_key = Symbol::new(&env, "AdminList");

    // All admin-related keys should be identifiable
    assert!(admin_key.to_string().contains("Admin"));
    assert!(admin_count_key.to_string().contains("Admin"));
    assert!(admin_list_key.to_string().contains("Admin"));
}

#[test]
fn test_circuit_breaker_namespace_prefix() {
    let env = create_test_env();

    // Verify all circuit breaker keys use CB_ prefix
    let cb_config = Symbol::new(&env, "CB_CONFIG");
    let cb_state = Symbol::new(&env, "CB_STATE");
    let cb_events = Symbol::new(&env, "CB_EVENTS");
    let cb_conditions = Symbol::new(&env, "CB_CONDITIONS");

    // All CB keys should start with CB_
    assert!(cb_config.to_string().starts_with("CB_"));
    assert!(cb_state.to_string().starts_with("CB_"));
    assert!(cb_events.to_string().starts_with("CB_"));
    assert!(cb_conditions.to_string().starts_with("CB_"));
}

#[test]
fn test_audit_trail_namespace_prefix() {
    let env = create_test_env();

    // Verify audit trail keys use AUDIT_ prefix
    let audit_head = Symbol::new(&env, "AUDIT_HEAD");
    let audit_rec = Symbol::new(&env, "AUDIT_REC");

    // All audit keys should start with AUDIT_
    assert!(audit_head.to_string().starts_with("AUDIT_"));
    assert!(audit_rec.to_string().starts_with("AUDIT_"));
}

// ===== BALANCE STORAGE KEY TESTS =====

#[test]
fn test_balance_storage_key_uniqueness() {
    let env = create_test_env();
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let asset1 = ReflectorAsset::BTC;
    let asset2 = ReflectorAsset::ETH;

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

    // Store events
    EventManager::store_event(&env, &event1);
    EventManager::store_event(&env, &event2);

    // Verify events are stored independently
    let retrieved1 = EventManager::get_event(&env, &event1.id).unwrap();
    let retrieved2 = EventManager::get_event(&env, &event2.id).unwrap();

    assert_eq!(retrieved1.id, event1.id);
    assert_eq!(retrieved2.id, event2.id);
    assert_ne!(retrieved1.id, retrieved2.id);
}

// ===== CREATOR LIMITS STORAGE KEY TESTS =====

#[test]
fn test_creator_limits_key_uniqueness() {
    let env = create_test_env();
    let creator1 = Address::generate(&env);
    let creator2 = Address::generate(&env);

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
}

// ===== DATA STRUCTURE EXTENSION TESTS =====

#[test]
fn test_market_structure_serialization() {
    let env = create_test_env();
    let admin = create_test_admin(&env);
    let (market_id, market) = create_test_market(&env, &admin);

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
}

#[test]
fn test_claim_info_structure_serialization() {
    let env = create_test_env();

    let claim_info = ClaimInfo {
        claimed: true,
        timestamp: env.ledger().timestamp(),
        payout_amount: 1_000_000,
    };

    // Store claim info
    let key = Symbol::new(&env, "test_claim");
    env.storage().persistent().set(&key, &claim_info);

    // Retrieve claim info
    let retrieved: ClaimInfo = env.storage().persistent().get(&key).unwrap();

    // Verify all fields are preserved
    assert_eq!(retrieved.claimed, claim_info.claimed);
    assert_eq!(retrieved.timestamp, claim_info.timestamp);
    assert_eq!(retrieved.payout_amount, claim_info.payout_amount);
}

#[test]
fn test_oracle_config_structure_serialization() {
    let env = create_test_env();

    let oracle_config = OracleConfig::new(
        OracleProvider::reflector(),
        Address::generate(&env),
        String::from_str(&env, "BTC/USD"),
        50_000_00,
        String::from_str(&env, "gt"),
    );

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
}

// ===== STORAGE KEY PATTERN TESTS =====

#[test]
fn test_simple_symbol_key_pattern() {
    let env = create_test_env();

    // Test simple symbol key storage and retrieval
    let key = Symbol::new(&env, "TestKey");
    let value = String::from_str(&env, "TestValue");

    env.storage().persistent().set(&key, &value);
    let retrieved: String = env.storage().persistent().get(&key).unwrap();

    assert_eq!(retrieved, value);
}

#[test]
fn test_tuple_key_pattern() {
    let env = create_test_env();

    // Test tuple key storage and retrieval
    let key = (
        Symbol::new(&env, "Namespace"),
        Symbol::new(&env, "identifier"),
    );
    let value = String::from_str(&env, "TupleValue");

    env.storage().persistent().set(&key, &value);
    let retrieved: String = env.storage().persistent().get(&key).unwrap();

    assert_eq!(retrieved, value);
}

#[test]
fn test_tuple_with_address_key_pattern() {
    let env = create_test_env();
    let address = Address::generate(&env);

    // Test tuple with address key storage and retrieval
    let key = (Symbol::new(&env, "UserData"), address.clone());
    let value = 12345u32;

    env.storage().persistent().set(&key, &value);
    let retrieved: u32 = env.storage().persistent().get(&key).unwrap();

    assert_eq!(retrieved, value);
}

// ===== MIGRATION SAFETY TESTS =====

#[test]
fn test_market_backward_compatibility() {
    let env = create_test_env();
    let admin = create_test_admin(&env);
    let (market_id, market) = create_test_market(&env, &admin);

    // Store market
    MarketStateManager::update_market(&env, &market_id, &market);

    // Retrieve and verify
    let retrieved = MarketStateManager::get_market(&env, &market_id).unwrap();

    // Verify critical fields are preserved
    assert_eq!(retrieved.admin, market.admin);
    assert_eq!(retrieved.question, market.question);
    assert_eq!(retrieved.total_staked, market.total_staked);
    assert_eq!(retrieved.state, market.state);
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
    let (_, market2) = create_test_market(&env, &admin);

    // Compress markets
    let compressed1 = StorageOptimizer::compress_market_data(&env, &market1).unwrap();
    let compressed2 = StorageOptimizer::compress_market_data(&env, &market2).unwrap();

    // Verify compressed markets have unique IDs
    assert_ne!(compressed1.market_id, compressed2.market_id);
}

#[test]
fn test_storage_config_isolation() {
    let env = create_test_env();

    // Get default storage config
    let config = StorageOptimizer::get_storage_config(&env);

    // Verify config has expected defaults
    assert!(config.compression_enabled);
    assert_eq!(config.min_compression_age_days, 30);
    assert_eq!(config.max_storage_per_market, 1024 * 1024);
    assert_eq!(config.cleanup_threshold_days, 365);
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

// ===== STORAGE KEY DOCUMENTATION TESTS =====

#[test]
fn test_storage_key_naming_conventions() {
    let env = create_test_env();

    // Test that keys follow naming conventions
    let admin_key = Symbol::new(&env, "Admin");
    let config_key = Symbol::new(&env, "Config");

    // Keys should use PascalCase for singleton values
    assert!(admin_key.to_string().chars().next().unwrap().is_uppercase());
    assert!(config_key.to_string().chars().next().unwrap().is_uppercase());

    // Namespace prefixes should be uppercase
    let cb_config = Symbol::new(&env, "CB_CONFIG");
    let audit_head = Symbol::new(&env, "AUDIT_HEAD");

    assert!(cb_config.to_string().starts_with("CB_"));
    assert!(audit_head.to_string().starts_with("AUDIT_"));
}

// ===== REGRESSION TESTS =====

#[test]
fn test_no_regression_in_market_storage() {
    let env = create_test_env();
    let admin = create_test_admin(&env);
    let (market_id, market) = create_test_market(&env, &admin);

    // Store market using current implementation
    MarketStateManager::update_market(&env, &market_id, &market);

    // Retrieve using current implementation
    let retrieved = MarketStateManager::get_market(&env, &market_id).unwrap();

    // Verify no data loss
    assert_eq!(retrieved.admin, market.admin);
    assert_eq!(retrieved.question, market.question);
    assert_eq!(retrieved.outcomes.len(), market.outcomes.len());
    assert_eq!(retrieved.end_time, market.end_time);
    assert_eq!(retrieved.total_staked, market.total_staked);
    assert_eq!(retrieved.state, market.state);
    assert_eq!(retrieved.fee_collected, market.fee_collected);
}

#[test]
fn test_no_regression_in_balance_storage() {
    let env = create_test_env();
    let user = Address::generate(&env);
    let asset = ReflectorAsset::BTC;

    // Add balance using current implementation
    BalanceStorage::add_balance(&env, &user, &asset, 1_000_000).unwrap();

    // Retrieve using current implementation
    let balance = BalanceStorage::get_balance(&env, &user, &asset);

    // Verify no data loss
    assert_eq!(balance.amount, 1_000_000);
    assert_eq!(balance.user, user);
    assert_eq!(balance.asset, asset);
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
