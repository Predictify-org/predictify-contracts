#![cfg(test)]
extern crate std;

use crate::audit_trail::{AuditAction, AuditEntryV2, AuditReasonTable, AuditRecordVersioned, AuditTrailManager};
use soroban_sdk::{testutils::Address as _, Address, Env, Symbol, String};

#[test]
fn test_audit_compact_encoding() {
    let env = Env::default();
    let admin = Address::generate(&env);
    
    // Setup contract
    let contract_id = env.register(crate::PredictifyHybrid, ());
    
    env.as_contract(&contract_id, || {
        // Initialize admin so require_admin passes
        crate::admin::AdminManager::initialize(&env, &admin, &None, &None);

        let reason_str = String::from_str(&env, "Test Compact Reason");
        let reason_idx = AuditReasonTable::add_reason(&env, &admin, reason_str.clone());
        
        assert_eq!(reason_idx, 0, "First reason should have index 0");
        
        let reasons = AuditReasonTable::get_reasons(&env);
        assert_eq!(reasons.len(), 1);
        assert_eq!(reasons.get(0).unwrap(), reason_str);

        let action = Symbol::new(&env, "CMPT_ACT");
        
        // Append V2 Record
        let index_v2 = AuditTrailManager::append_record_v2(&env, action.clone(), admin.clone(), reason_idx);
        
        // Retrieve and check backward compatibility
        let record_opt = AuditTrailManager::get_record(&env, index_v2);
        assert!(record_opt.is_some());
        
        if let AuditRecordVersioned::V2(v2) = record_opt.unwrap() {
            assert_eq!(v2.action, action);
            assert_eq!(v2.reason_idx, reason_idx);
            assert_eq!(v2.actor, admin.clone());
        } else {
            panic!("Expected V2 record");
        }
        
        // Test Integrity for V2
        assert!(AuditTrailManager::verify_integrity(&env, 1));
        
        // Append V1 Record and check backward compatibility
        let details = soroban_sdk::Map::new(&env);
        let index_v1 = AuditTrailManager::append_record(&env, AuditAction::ContractInitialized, admin.clone(), details, None);
        
        let v1_opt = AuditTrailManager::get_record(&env, index_v1);
        assert!(v1_opt.is_some());
        if let AuditRecordVersioned::V1(v1) = v1_opt.unwrap() {
            assert_eq!(v1.action, AuditAction::ContractInitialized);
        } else {
            panic!("Expected V1 record");
        }
        
        // Test Integrity for both V1 and V2 in the same chain
        assert!(AuditTrailManager::verify_integrity(&env, 2));

        // Test measurable storage size reduction is done in test_audit_compact_size_reduction
    });
}

#[test]
fn test_audit_compact_size_reduction() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let contract_id = env.register(crate::PredictifyHybrid, ());
    
    env.as_contract(&contract_id, || {
        crate::admin::AdminManager::initialize(&env, &admin, &None, &None);
        let reason_idx = AuditReasonTable::add_reason(&env, &admin, String::from_str(&env, "Reason"));
        
        let mut details = soroban_sdk::Map::new(&env);
        details.set(Symbol::new(&env, "reason"), String::from_str(&env, "Reason"));
        let index_v1 = AuditTrailManager::append_record(&env, AuditAction::MarketCreated, admin.clone(), details, None);
        
        let index_v2 = AuditTrailManager::append_record_v2(&env, Symbol::new(&env, "MktCrt"), admin.clone(), reason_idx);
        
        let v1_record = AuditTrailManager::get_record(&env, index_v1).unwrap();
        let v2_record = AuditTrailManager::get_record(&env, index_v2).unwrap();
        
        use soroban_sdk::xdr::ToXdr;
        let len_v1 = v1_record.to_xdr(&env).len();
        let len_v2 = v2_record.to_xdr(&env).len();
        
        assert!(len_v2 < len_v1, "V2 record must be strictly smaller than V1 record");
    });
}
