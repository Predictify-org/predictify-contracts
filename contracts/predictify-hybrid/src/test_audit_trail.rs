#![cfg(test)]

use crate::audit_trail::{AuditAction, AuditRecord, AuditTrailHead, AuditTrailManager};
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
        );
        AuditTrailManager::append_record(
            &env,
            AuditAction::ContractUnpaused,
            actor.clone(),
            Map::new(&env),
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
            );
        }
    });

    let record1 = client.get_audit_record(&1).unwrap();
    assert_eq!(record1.index, 1);

    let latest = client.get_latest_audit_records(&2);
    assert_eq!(latest.len(), 2);
    assert_eq!(latest.get(0).unwrap().index, 3);
    assert_eq!(latest.get(1).unwrap().index, 2);

    assert!(client.verify_audit_integrity(&5));
}
