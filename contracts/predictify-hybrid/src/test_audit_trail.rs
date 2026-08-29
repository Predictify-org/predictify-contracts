#![cfg(test)]

use crate::audit_trail::{
    AuditAction, AuditEntryV2, AuditRecord, AuditRecordVersion, AuditTrailHead, AuditTrailManager,
};
use crate::PredictifyHybrid;
use crate::PredictifyHybridClient;
use soroban_sdk::{testutils::Address as _, Address, BytesN, Env, Map, String, Symbol};

fn create_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env
}

#[test]
fn test_append_and_get_record() {
    let env = create_env();
    let contract_id = env.register(PredictifyHybrid {}, ());

    let actor = Address::generate(&env);

    env.as_contract(&contract_id, || {
        let mut details = Map::new(&env);
        details.set(Symbol::new(&env, "key1"), String::from_str(&env, "value1"));

        let index1 = AuditTrailManager::append_record(
            &env,
            AuditAction::ContractInitialized,
            actor.clone(),
            details.clone(),
            None,
        );
        assert_eq!(index1, 1);

        let record1 = AuditTrailManager::get_record(&env, 1).unwrap();
        assert_eq!(record1.index, 1);
        assert_eq!(record1.action, AuditAction::ContractInitialized);
        assert_eq!(record1.actor, actor.clone());
        assert_eq!(record1.details, details);
        assert_eq!(
            record1.prev_record_hash,
            BytesN::from_array(&env, &[0u8; 32])
        );

        // Append second record
        let index2 = AuditTrailManager::append_record(
            &env,
            AuditAction::MarketCreated,
            actor.clone(),
            Map::new(&env),
            None,
        );
        assert_eq!(index2, 2);

        let record2 = AuditTrailManager::get_record(&env, 2).unwrap();
        assert_eq!(record2.index, 2);
        assert_eq!(record2.action, AuditAction::MarketCreated);

        // Check hash links
        let head = AuditTrailManager::get_head(&env).unwrap();
        assert_eq!(head.latest_index, 2);

        use soroban_sdk::xdr::ToXdr;
        let record1_bytes = record1.clone().to_xdr(&env);
        let expected_hash1: BytesN<32> = env.crypto().sha256(&record1_bytes).into();
        assert_eq!(record2.prev_record_hash, expected_hash1);
    });
}

#[test]
fn test_compact_encoding_v2_and_reason_table() {
    let env = create_env();
    let contract_id = env.register(PredictifyHybrid {}, ());
    let admin = Address::generate(&env);
    let actor = Address::generate(&env);

    // Initialize admin in contract persistent storage
    env.as_contract(&contract_id, || {
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, "Admin"), &admin);

        // Register reason in admin-gated reason table
        let reason_str = String::from_str(&env, "Emergency Pause Executed");
        let reason_idx = AuditTrailManager::add_reason(&env, &admin, reason_str.clone()).unwrap();
        assert_eq!(reason_idx, 0);

        let fetched_reason = AuditTrailManager::get_reason(&env, 0).unwrap();
        assert_eq!(fetched_reason, reason_str);

        let ref_id = BytesN::from_array(&env, &[1u8; 32]);
        let index = AuditTrailManager::append_record_v2(
            &env,
            Symbol::new(&env, "PAUSE"),
            reason_idx,
            actor.clone(),
            ref_id.clone(),
            None,
        );
        assert_eq!(index, 1);

        // Read versioned record
        let versioned = AuditTrailManager::get_record_versioned(&env, 1).unwrap();
        match versioned {
            AuditRecordVersion::V2(v2) => {
                assert_eq!(v2.index, 1);
                assert_eq!(v2.action, Symbol::new(&env, "PAUSE"));
                assert_eq!(v2.reason_idx, 0);
                assert_eq!(v2.actor, actor);
                assert_eq!(v2.ref_id, ref_id);
            }
            _ => panic!("Expected V2 record"),
        }

        // Verify chain integrity
        assert!(AuditTrailManager::verify_integrity(&env, 10));
    });
}

#[test]
fn test_mixed_v1_v2_chain_integrity() {
    let env = create_env();
    let contract_id = env.register(PredictifyHybrid {}, ());
    let admin = Address::generate(&env);
    let actor = Address::generate(&env);

    env.as_contract(&contract_id, || {
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, "Admin"), &admin);

        let idx1 = AuditTrailManager::append_record(
            &env,
            AuditAction::ContractInitialized,
            actor.clone(),
            Map::new(&env),
            None,
        );
        assert_eq!(idx1, 1);

        let reason_idx = AuditTrailManager::add_reason(
            &env,
            &admin,
            String::from_str(&env, "Market Resolved Manually"),
        )
        .unwrap();

        let idx2 = AuditTrailManager::append_record_v2(
            &env,
            Symbol::new(&env, "RESOLVE"),
            reason_idx,
            actor.clone(),
            BytesN::from_array(&env, &[2u8; 32]),
            None,
        );
        assert_eq!(idx2, 2);

        let idx3 = AuditTrailManager::append_record(
            &env,
            AuditAction::FeesCollected,
            actor.clone(),
            Map::new(&env),
            None,
        );
        assert_eq!(idx3, 3);

        // Verify mixed chain integrity across V1 -> V2 -> V1
        assert!(AuditTrailManager::verify_integrity(&env, 10));

        let latest_records = AuditTrailManager::get_latest_records_versioned(&env, 3);
        assert_eq!(latest_records.len(), 3);
    });
}

#[test]
fn test_storage_size_reduction_benchmark() {
    let env = create_env();
    let contract_id = env.register(PredictifyHybrid {}, ());
    let actor = Address::generate(&env);

    env.as_contract(&contract_id, || {
        use soroban_sdk::xdr::ToXdr;

        let mut details = Map::new(&env);
        details.set(
            Symbol::new(&env, "reason"),
            String::from_str(&env, "Admin manually triggered emergency fallback mechanism"),
        );
        details.set(
            Symbol::new(&env, "executor"),
            String::from_str(&env, "0x1234567890abcdef1234567890abcdef12345678"),
        );

        let v1_record = AuditRecord {
            index: 1,
            action: AuditAction::ContractPaused,
            actor: actor.clone(),
            timestamp: 1700000000,
            details,
            prev_record_hash: BytesN::from_array(&env, &[0u8; 32]),
            override_nonce: Some(10),
        };
        let v1_bytes = v1_record.to_xdr(&env);

        let v2_record = AuditEntryV2 {
            index: 1,
            action: Symbol::new(&env, "PAUSE"),
            reason_idx: 0,
            actor: actor.clone(),
            ts: 1700000000,
            ref_id: BytesN::from_array(&env, &[0u8; 32]),
            prev_record_hash: BytesN::from_array(&env, &[0u8; 32]),
            override_nonce: Some(10),
        };
        let v2_bytes = v2_record.to_xdr(&env);

        assert!(
            v2_bytes.len() < v1_bytes.len(),
            "V2 compact encoding must consume fewer bytes than V1 (V2: {} bytes, V1: {} bytes)",
            v2_bytes.len(),
            v1_bytes.len()
        );
    });
}

#[test]
fn test_verify_integrity() {
    let env = create_env();
    let contract_id = env.register(PredictifyHybrid {}, ());
    let actor = Address::generate(&env);

    env.as_contract(&contract_id, || {
        // Initial verify should be true (empty trail)
        assert!(AuditTrailManager::verify_integrity(&env, 10));

        for _ in 0..5 {
            AuditTrailManager::append_record(
                &env,
                AuditAction::ContractPaused,
                actor.clone(),
                Map::new(&env),
                None,
            );
        }

        assert!(AuditTrailManager::verify_integrity(&env, 5));
        assert!(AuditTrailManager::verify_integrity(&env, 10));
    });
}

#[test]
fn test_verify_integrity_tampering() {
    let env = create_env();
    let contract_id = env.register(PredictifyHybrid {}, ());
    let actor = Address::generate(&env);

    env.as_contract(&contract_id, || {
        AuditTrailManager::append_record(
            &env,
            AuditAction::ContractPaused,
            actor.clone(),
            Map::new(&env),
            None,
        );
        AuditTrailManager::append_record(
            &env,
            AuditAction::ContractUnpaused,
            actor.clone(),
            Map::new(&env),
            None,
        );

        // Tamper with record 1
        let mut record1 = AuditTrailManager::get_record(&env, 1).unwrap();
        record1.action = AuditAction::AdminAdded; // Mutate action
        env.storage()
            .persistent()
            .set(&(Symbol::new(&env, "AUDIT_REC"), 1u64), &record1);

        // Verification should fail because hash of tampered record1 won't match record2.prev_record_hash
        assert!(!AuditTrailManager::verify_integrity(&env, 2));
    });
}

#[test]
fn test_public_queries() {
    let env = create_env();
    let contract_id = env.register(PredictifyHybrid {}, ());
    let client = PredictifyHybridClient::new(&env, &contract_id);
    let actor = Address::generate(&env);

    env.as_contract(&contract_id, || {
        for _ in 1..=3 {
            AuditTrailManager::append_record(
                &env,
                AuditAction::AdminRoleUpdated,
                actor.clone(),
                Map::new(&env),
                None,
            );
        }
    });

    env.as_contract(&contract_id, || {
        let record1 = AuditTrailManager::get_record(&env, 1).unwrap();
        assert_eq!(record1.index, 1);

        let latest = AuditTrailManager::get_latest_records(&env, 2);
        assert_eq!(latest.len(), 2);
        assert_eq!(latest.get(0).unwrap().index, 3);
        assert_eq!(latest.get(1).unwrap().index, 2);

        assert!(AuditTrailManager::verify_integrity(&env, 5));
    });
}
