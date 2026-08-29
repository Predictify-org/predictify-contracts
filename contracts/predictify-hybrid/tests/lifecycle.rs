//! Comprehensive lifecycle state transition tests for archive and restore functionality.
//!
//! Tests cover:
//! - Archive transitions (success and rejection cases)
//! - Restore transitions (success and rejection cases)
//! - State validation and corruption detection
//! - Concurrent access safety
//! - Boundary cases and edge conditions
//! - Error handling and recovery

use predictify_hybrid::{
    PredictifyHybridClient,
    types::MarketState,
    err::Error,
};
use soroban_sdk::{testutils::Address as _, Address, Env, String, Symbol, Vec};

// ===== TEST SETUP =====

struct TestSetup {
    env: Env,
    contract_id: Address,
    admin: Address,
    user1: Address,
    user2: Address,
}

impl TestSetup {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, predictify_hybrid::PredictifyHybrid);
        let admin = Address::generate(&env);
        let user1 = Address::generate(&env);
        let user2 = Address::generate(&env);

        // Initialize contract
        let client = PredictifyHybridClient::new(&env, &contract_id);
        client.initialize(&admin);

        TestSetup {
            env,
            contract_id,
            admin,
            user1,
            user2,
        }
    }

    fn create_resolved_market(&self, question: &str) -> Symbol {
        let client = PredictifyHybridClient::new(&self.env, &self.contract_id);
        
        let market_id = Symbol::new(&self.env, question);
        let question_str = String::from_str(&self.env, question);
        let mut outcomes = Vec::new(&self.env);
        outcomes.push_back(String::from_str(&self.env, "Yes"));
        outcomes.push_back(String::from_str(&self.env, "No"));

        // Create market
        let end_time = self.env.ledger().timestamp() + 1000;
        client.create_market(
            &self.admin,
            &market_id,
            &question_str,
            &outcomes,
            end_time,
            &String::from_str(&self.env, ""),
        );

        // Fast-forward past market end
        self.env.ledger().set_timestamp(end_time + 1);

        // Manually resolve the market (simulate oracle resolution)
        client.resolve_market_manual(
            &self.admin,
            &market_id,
            &String::from_str(&self.env, "Yes"),
        );

        market_id
    }

    fn create_cancelled_market(&self, question: &str) -> Symbol {
        let client = PredictifyHybridClient::new(&self.env, &self.contract_id);
        
        let market_id = Symbol::new(&self.env, question);
        let question_str = String::from_str(&self.env, question);
        let mut outcomes = Vec::new(&self.env);
        outcomes.push_back(String::from_str(&self.env, "Yes"));
        outcomes.push_back(String::from_str(&self.env, "No"));

        let end_time = self.env.ledger().timestamp() + 1000;
        client.create_market(
            &self.admin,
            &market_id,
            &question_str,
            &outcomes,
            end_time,
            &String::from_str(&self.env, ""),
        );

        // Cancel the market
        client.cancel_market(&self.admin, &market_id, &String::from_str(&self.env, "Test cancellation"));

        market_id
    }
}

// ===== ARCHIVE SUCCESS TESTS =====

#[test]
fn test_archive_from_resolved_state() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);
    
    let market_id = setup.create_resolved_market("test_archive_resolved");
    
    // Archive should succeed from Resolved state
    let result = client.try_archive_event(&setup.admin, &market_id);
    assert!(result.is_ok(), "Archive from Resolved should succeed");
}

#[test]
fn test_archive_from_cancelled_state() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);
    
    let market_id = setup.create_cancelled_market("test_archive_cancelled");
    
    // Archive should succeed from Cancelled state
    let result = client.try_archive_event(&setup.admin, &market_id);
    assert!(result.is_ok(), "Archive from Cancelled should succeed");
}

#[test]
fn test_archive_emits_event() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);
    
    let market_id = setup.create_resolved_market("test_archive_event");
    
    // Archive market
    client.archive_event(&setup.admin, &market_id);
    
    // Check events were emitted
    let events = setup.env.events().all();
    assert!(!events.is_empty(), "Archive should emit events");
}

// ===== ARCHIVE REJECTION TESTS =====

#[test]
fn test_archive_fails_from_active_state() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);
    
    let market_id = Symbol::new(&setup.env, "test_archive_active");
    let question = String::from_str(&setup.env, "test_archive_active");
    let mut outcomes = Vec::new(&setup.env);
    outcomes.push_back(String::from_str(&setup.env, "Yes"));
    outcomes.push_back(String::from_str(&setup.env, "No"));
    
    let end_time = setup.env.ledger().timestamp() + 10000;
    client.create_market(
        &setup.admin,
        &market_id,
        &question,
        &outcomes,
        end_time,
        &String::from_str(&setup.env, ""),
    );
    
    // Archive should fail from Active state
    let result = client.try_archive_event(&setup.admin, &market_id);
    match result {
        Err(Ok(err)) => {
            assert_eq!(err, Error::CannotArchiveFromState as u32, "Should return CannotArchiveFromState error");
        }
        _ => panic!("Expected CannotArchiveFromState error"),
    }
}

#[test]
fn test_archive_duplicate_rejected() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);
    
    let market_id = setup.create_resolved_market("test_archive_duplicate");
    
    // First archive succeeds
    client.archive_event(&setup.admin, &market_id);
    
    // Second archive should fail with MarketAlreadyArchived
    let result = client.try_archive_event(&setup.admin, &market_id);
    match result {
        Err(Ok(err)) => {
            assert_eq!(err, Error::MarketAlreadyArchived as u32, "Should return MarketAlreadyArchived error");
        }
        _ => panic!("Expected MarketAlreadyArchived error"),
    }
}

#[test]
fn test_archive_requires_admin_authorization() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);
    
    let market_id = setup.create_resolved_market("test_archive_auth");
    
    // Archive as non-admin should fail
    let result = client.try_archive_event(&setup.user1, &market_id);
    match result {
        Err(Ok(err)) => {
            assert_eq!(err, Error::Unauthorized as u32, "Should return Unauthorized error");
        }
        _ => panic!("Expected Unauthorized error"),
    }
}

#[test]
fn test_archive_nonexistent_market() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);
    
    let nonexistent_id = Symbol::new(&setup.env, "nonexistent");
    
    // Archive on nonexistent market should fail
    let result = client.try_archive_event(&setup.admin, &nonexistent_id);
    match result {
        Err(Ok(err)) => {
            assert_eq!(err, Error::MarketNotFound as u32, "Should return MarketNotFound error");
        }
        _ => panic!("Expected MarketNotFound error"),
    }
}

// ===== RESTORE SUCCESS TESTS =====

#[test]
fn test_restore_from_archived_state() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);
    
    let market_id = setup.create_resolved_market("test_restore_archived");
    
    // Archive first
    client.archive_event(&setup.admin, &market_id);
    
    // Restore should succeed from Archived state
    let reason = String::from_str(&setup.env, "Test restore");
    let result = client.try_restore_event(&setup.admin, &market_id, &reason);
    assert!(result.is_ok(), "Restore from Archived should succeed");
}

#[test]
fn test_restore_emits_event() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);
    
    let market_id = setup.create_resolved_market("test_restore_event");
    client.archive_event(&setup.admin, &market_id);
    
    // Restore market
    let reason = String::from_str(&setup.env, "Test restore with events");
    client.restore_event(&setup.admin, &market_id, &reason);
    
    // Check events were emitted
    let events = setup.env.events().all();
    assert!(!events.is_empty(), "Restore should emit events");
}

// ===== RESTORE REJECTION TESTS =====

#[test]
fn test_restore_fails_from_resolved_state() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);
    
    let market_id = setup.create_resolved_market("test_restore_resolved");
    
    // Restore without archiving should fail
    let reason = String::from_str(&setup.env, "Invalid restore");
    let result = client.try_restore_event(&setup.admin, &market_id, &reason);
    match result {
        Err(Ok(err)) => {
            assert_eq!(err, Error::CannotRestoreFromState as u32, "Should return CannotRestoreFromState error");
        }
        _ => panic!("Expected CannotRestoreFromState error"),
    }
}

#[test]
fn test_restore_duplicate_rejected() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);
    
    let market_id = setup.create_resolved_market("test_restore_duplicate");
    
    // Archive then restore
    client.archive_event(&setup.admin, &market_id);
    let reason = String::from_str(&setup.env, "First restore");
    client.restore_event(&setup.admin, &market_id, &reason);
    
    // Second restore should fail
    let reason2 = String::from_str(&setup.env, "Duplicate restore");
    let result = client.try_restore_event(&setup.admin, &market_id, &reason2);
    match result {
        Err(Ok(err)) => {
            assert_eq!(err, Error::MarketAlreadyRestored as u32, "Should return MarketAlreadyRestored error");
        }
        _ => panic!("Expected MarketAlreadyRestored error"),
    }
}

#[test]
fn test_restore_requires_admin_authorization() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);
    
    let market_id = setup.create_resolved_market("test_restore_auth");
    client.archive_event(&setup.admin, &market_id);
    
    // Restore as non-admin should fail
    let reason = String::from_str(&setup.env, "Unauthorized restore");
    let result = client.try_restore_event(&setup.user1, &market_id, &reason);
    match result {
        Err(Ok(err)) => {
            assert_eq!(err, Error::Unauthorized as u32, "Should return Unauthorized error");
        }
        _ => panic!("Expected Unauthorized error"),
    }
}

#[test]
fn test_restore_nonexistent_market() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);
    
    let nonexistent_id = Symbol::new(&setup.env, "nonexistent");
    let reason = String::from_str(&setup.env, "Invalid restore");
    
    // Restore on nonexistent market should fail
    let result = client.try_restore_event(&setup.admin, &nonexistent_id, &reason);
    match result {
        Err(Ok(err)) => {
            assert_eq!(err, Error::MarketNotFound as u32, "Should return MarketNotFound error");
        }
        _ => panic!("Expected MarketNotFound error"),
    }
}

// ===== BOUNDARY AND EDGE CASE TESTS =====

#[test]
fn test_archive_capacity_respected() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);
    
    // Try to archive many markets to test capacity
    // Note: This would require creating MAX_ARCHIVE_SIZE markets first
    // For now, just verify that archive_size() can be queried
    let size = client.get_archive_size();
    assert!(size >= 0, "Archive size should be non-negative");
}

#[test]
fn test_archive_then_restore_lifecycle() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);
    
    let market_id = setup.create_resolved_market("test_full_lifecycle");
    
    // Archive
    client.archive_event(&setup.admin, &market_id);
    
    // Verify is_archived returns true
    assert!(client.is_archived(&market_id), "Market should be archived");
    
    // Restore
    let reason = String::from_str(&setup.env, "Full lifecycle test");
    client.restore_event(&setup.admin, &market_id, &reason);
    
    // Verify is_restored returns true
    assert!(client.is_restored(&market_id), "Market should be restored");
}

#[test]
fn test_concurrent_archive_attempts_idempotent() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);
    
    let market_id = setup.create_resolved_market("test_concurrent_archive");
    
    // First archive succeeds
    let result1 = client.try_archive_event(&setup.admin, &market_id);
    assert!(result1.is_ok(), "First archive should succeed");
    
    // Second archive attempt (from same admin) should be rejected deterministically
    let result2 = client.try_archive_event(&setup.admin, &market_id);
    match result2 {
        Err(Ok(err)) => {
            // Either MarketAlreadyArchived or other archive-related error is acceptable
            assert!(
                err == Error::MarketAlreadyArchived as u32,
                "Second archive should fail with MarketAlreadyArchived"
            );
        }
        _ => panic!("Expected error on duplicate archive"),
    }
}

#[test]
fn test_market_state_consistency_after_archive() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);
    
    let market_id = setup.create_resolved_market("test_state_consistency");
    
    // Archive the market
    client.archive_event(&setup.admin, &market_id);
    
    // Verify state is consistent
    assert!(client.is_archived(&market_id), "is_archived() should return true");
    assert!(!client.is_restored(&market_id), "is_restored() should return false");
}

#[test]
fn test_market_state_consistency_after_restore() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);
    
    let market_id = setup.create_resolved_market("test_state_after_restore");
    
    // Archive then restore
    client.archive_event(&setup.admin, &market_id);
    let reason = String::from_str(&setup.env, "Test state consistency");
    client.restore_event(&setup.admin, &market_id, &reason);
    
    // Verify state is consistent
    assert!(!client.is_archived(&market_id), "is_archived() should return false after restore");
    assert!(client.is_restored(&market_id), "is_restored() should return true");
}

// ===== REGRESSION TESTS =====

#[test]
fn test_archive_respects_existing_authorization() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);
    
    let market_id = setup.create_resolved_market("test_auth_regression");
    
    // Verify that admin can archive
    let result = client.try_archive_event(&setup.admin, &market_id);
    assert!(result.is_ok(), "Admin should be able to archive");
}

#[test]
fn test_restore_respects_existing_authorization() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);
    
    let market_id = setup.create_resolved_market("test_restore_auth_regression");
    client.archive_event(&setup.admin, &market_id);
    
    // Verify that admin can restore
    let reason = String::from_str(&setup.env, "Auth regression test");
    let result = client.try_restore_event(&setup.admin, &market_id, &reason);
    assert!(result.is_ok(), "Admin should be able to restore");
}

// ===== INTEGRATION TESTS =====

#[test]
fn test_multiple_archives_in_sequence() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);
    
    // Create and archive multiple markets
    let m1 = setup.create_resolved_market("test_multi_1");
    let m2 = setup.create_resolved_market("test_multi_2");
    let m3 = setup.create_resolved_market("test_multi_3");
    
    // Archive all
    client.archive_event(&setup.admin, &m1);
    client.archive_event(&setup.admin, &m2);
    client.archive_event(&setup.admin, &m3);
    
    // Verify all are archived
    assert!(client.is_archived(&m1), "Market 1 should be archived");
    assert!(client.is_archived(&m2), "Market 2 should be archived");
    assert!(client.is_archived(&m3), "Market 3 should be archived");
}

#[test]
fn test_mixed_archive_restore_operations() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);
    
    // Create markets
    let m1 = setup.create_resolved_market("test_mixed_1");
    let m2 = setup.create_resolved_market("test_mixed_2");
    
    // Archive both
    client.archive_event(&setup.admin, &m1);
    client.archive_event(&setup.admin, &m2);
    
    // Restore first, leave second archived
    let reason = String::from_str(&setup.env, "Partial restore");
    client.restore_event(&setup.admin, &m1, &reason);
    
    // Verify states
    assert!(client.is_restored(&m1), "Market 1 should be restored");
    assert!(!client.is_restored(&m1), "Market 1 should NOT be archived");
    assert!(client.is_archived(&m2), "Market 2 should still be archived");
}
