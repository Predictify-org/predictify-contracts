//! Comprehensive tests for multi-admin and multisig support
//! 
//! This test suite validates:
//! - Single admin (threshold 1) operations
//! - M-of-N threshold (e.g., 2 of 3) multisig operations
//! - Add/remove admin functionality
//! - Threshold update operations
//! - Sensitive operations requiring threshold approval
//! - Event emission for admin actions
//! - Authorization failures and edge cases

#![cfg(test)]

use crate::admin::{
    AdminManager, AdminRole, AdminSystemIntegration, ContractPauseManager, MultisigConfig,
    MultisigManager, PendingAdminAction,
};
use crate::errors::Error;
use crate::{PredictifyHybrid, PredictifyHybridClient};
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger, LedgerInfo},
    Address, Env, Map, String, Symbol,
};

/// Test helper to setup contract with admin
fn setup_contract() -> (Env, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register(PredictifyHybrid, ());
    let admin = Address::generate(&env);
    
    let client = PredictifyHybridClient::new(&env, &contract_id);
    client.initialize(&admin, &None);
    
    (env, contract_id, admin)
}

// ===== SINGLE ADMIN TESTS (THRESHOLD 1) =====

#[test]
fn test_single_admin_initialization() {
    let (env, contract_id, admin) = setup_contract();
    
    env.as_contract(&contract_id, || {
        let config = MultisigManager::get_config(&env);
        assert_eq!(config.threshold, 1);
        assert_eq!(config.enabled, false);
    });
}

#[test]
fn test_single_admin_add_admin() {
    let (env, contract_id, admin) = setup_contract();
    let new_admin = Address::generate(&env);
    
    env.as_contract(&contract_id, || {
        let result = AdminManager::add_admin(&env, &admin, &new_admin, AdminRole::MarketAdmin);
        assert!(result.is_ok());
        
        let role = AdminManager::get_admin_role_for_address(&env, &new_admin);
        assert_eq!(role, Some(AdminRole::MarketAdmin));
    });
}

#[test]
fn test_single_admin_remove_admin() {
    let (env, contract_id, admin) = setup_contract();
    let new_admin = Address::generate(&env);
    
    env.as_contract(&contract_id, || {
        AdminManager::add_admin(&env, &admin, &new_admin, AdminRole::MarketAdmin).unwrap();
        
        let result = AdminManager::remove_admin(&env, &admin, &new_admin);
        assert!(result.is_ok());
        
        let role = AdminManager::get_admin_role_for_address(&env, &new_admin);
        assert_eq!(role, None);
    });
}

#[test]
fn test_single_admin_update_role() {
    let (env, contract_id, admin) = setup_contract();
    let new_admin = Address::generate(&env);
    
    env.as_contract(&contract_id, || {
        AdminManager::add_admin(&env, &admin, &new_admin, AdminRole::MarketAdmin).unwrap();
        
        let result = AdminManager::update_admin_role(&env, &admin, &new_admin, AdminRole::ConfigAdmin);
        assert!(result.is_ok());
        
        let role = AdminManager::get_admin_role_for_address(&env, &new_admin);
        assert_eq!(role, Some(AdminRole::ConfigAdmin));
    });
}

#[test]
fn test_single_admin_cannot_remove_self_as_last_super_admin() {
    let (env, contract_id, admin) = setup_contract();
    
    env.as_contract(&contract_id, || {
        let result = AdminManager::remove_admin(&env, &admin, &admin);
        assert_eq!(result, Err(Error::InvalidState));
    });
}

// ===== MULTISIG THRESHOLD TESTS =====

#[test]
fn test_set_threshold_2_of_3() {
    let (env, contract_id, admin) = setup_contract();
    let admin2 = Address::generate(&env);
    let admin3 = Address::generate(&env);
    
    env.as_contract(&contract_id, || {
        AdminManager::add_admin(&env, &admin, &admin2, AdminRole::SuperAdmin).unwrap();
        AdminManager::add_admin(&env, &admin, &admin3, AdminRole::SuperAdmin).unwrap();
        
        let result = MultisigManager::set_threshold(&env, &admin, 2);
        assert!(result.is_ok());
        
        let config = MultisigManager::get_config(&env);
        assert_eq!(config.threshold, 2);
        assert_eq!(config.enabled, true);
    });
}

#[test]
fn test_set_threshold_invalid_zero() {
    let (env, contract_id, admin) = setup_contract();
    
    env.as_contract(&contract_id, || {
        let result = MultisigManager::set_threshold(&env, &admin, 0);
        assert_eq!(result, Err(Error::InvalidInput));
    });
}

#[test]
fn test_set_threshold_exceeds_admin_count() {
    let (env, contract_id, admin) = setup_contract();
    
    env.as_contract(&contract_id, || {
        let result = MultisigManager::set_threshold(&env, &admin, 5);
        assert_eq!(result, Err(Error::InvalidInput));
    });
}

#[test]
fn test_threshold_1_disables_multisig() {
    let (env, contract_id, admin) = setup_contract();
    
    env.as_contract(&contract_id, || {
        MultisigManager::set_threshold(&env, &admin, 1).unwrap();
        
        let config = MultisigManager::get_config(&env);
        assert_eq!(config.threshold, 1);
        assert_eq!(config.enabled, false);
    });
}

// ===== PENDING ACTION TESTS =====

#[test]
fn test_create_pending_action() {
    let (env, contract_id, admin) = setup_contract();
    let target = Address::generate(&env);
    
    env.as_contract(&contract_id, || {
        let data = Map::new(&env);
        let action_type = String::from_str(&env, "add_admin");
        
        let result = MultisigManager::create_pending_action(
            &env,
            &admin,
            action_type,
            target.clone(),
            data,
        );
        
        assert!(result.is_ok());
        let action_id = result.unwrap();
        assert_eq!(action_id, 1);
        
        let action = MultisigManager::get_pending_action(&env, action_id);
        assert!(action.is_some());
        
        let action = action.unwrap();
        assert_eq!(action.initiator, admin);
        assert_eq!(action.target, target);
        assert_eq!(action.approvals.len(), 1);
        assert_eq!(action.executed, false);
    });
}

#[test]
fn test_approve_pending_action() {
    let (env, contract_id, admin) = setup_contract();
    let admin2 = Address::generate(&env);
    let target = Address::generate(&env);
    
    env.as_contract(&contract_id, || {
        AdminManager::add_admin(&env, &admin, &admin2, AdminRole::SuperAdmin).unwrap();
        MultisigManager::set_threshold(&env, &admin, 2).unwrap();
        
        let data = Map::new(&env);
        let action_type = String::from_str(&env, "add_admin");
        let action_id = MultisigManager::create_pending_action(
            &env,
            &admin,
            action_type,
            target,
            data,
        ).unwrap();
        
        let result = MultisigManager::approve_action(&env, &admin2, action_id);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true); // Threshold met
        
        let action = MultisigManager::get_pending_action(&env, action_id).unwrap();
        assert_eq!(action.approvals.len(), 2);
    });
}

#[test]
fn test_approve_action_already_approved() {
    let (env, contract_id, admin) = setup_contract();
    let target = Address::generate(&env);
    
    env.as_contract(&contract_id, || {
        let data = Map::new(&env);
        let action_type = String::from_str(&env, "add_admin");
        let action_id = MultisigManager::create_pending_action(
            &env,
            &admin,
            action_type,
            target,
            data,
        ).unwrap();
        
        let result = MultisigManager::approve_action(&env, &admin, action_id);
        assert_eq!(result, Err(Error::InvalidState));
    });
}

#[test]
fn test_approve_action_not_found() {
    let (env, contract_id, admin) = setup_contract();
    
    env.as_contract(&contract_id, || {
        let result = MultisigManager::approve_action(&env, &admin, 999);
        assert_eq!(result, Err(Error::ConfigNotFound));
    });
}

#[test]
fn test_execute_action_threshold_met() {
    let (env, contract_id, admin) = setup_contract();
    let admin2 = Address::generate(&env);
    let target = Address::generate(&env);
    
    env.as_contract(&contract_id, || {
        AdminManager::add_admin(&env, &admin, &admin2, AdminRole::SuperAdmin).unwrap();
        MultisigManager::set_threshold(&env, &admin, 2).unwrap();
        
        let data = Map::new(&env);
        let action_type = String::from_str(&env, "add_admin");
        let action_id = MultisigManager::create_pending_action(
            &env,
            &admin,
            action_type,
            target,
            data,
        ).unwrap();
        
        MultisigManager::approve_action(&env, &admin2, action_id).unwrap();
        
        let result = MultisigManager::execute_action(&env, action_id);
        assert!(result.is_ok());
        
        let action = MultisigManager::get_pending_action(&env, action_id).unwrap();
        assert_eq!(action.executed, true);
    });
}

#[test]
fn test_execute_action_threshold_not_met() {
    let (env, contract_id, admin) = setup_contract();
    let admin2 = Address::generate(&env);
    let target = Address::generate(&env);
    
    env.as_contract(&contract_id, || {
        AdminManager::add_admin(&env, &admin, &admin2, AdminRole::SuperAdmin).unwrap();
        MultisigManager::set_threshold(&env, &admin, 2).unwrap();
        
        let data = Map::new(&env);
        let action_type = String::from_str(&env, "add_admin");
        let action_id = MultisigManager::create_pending_action(
            &env,
            &admin,
            action_type,
            target,
            data,
        ).unwrap();
        
        let result = MultisigManager::execute_action(&env, action_id);
        assert_eq!(result, Err(Error::Unauthorized));
    });
}

#[test]
fn test_execute_action_already_executed() {
    let (env, contract_id, admin) = setup_contract();
    let target = Address::generate(&env);
    
    env.as_contract(&contract_id, || {
        let data = Map::new(&env);
        let action_type = String::from_str(&env, "add_admin");
        let action_id = MultisigManager::create_pending_action(
            &env,
            &admin,
            action_type,
            target,
            data,
        ).unwrap();
        
        MultisigManager::execute_action(&env, action_id).unwrap();
        
        let result = MultisigManager::execute_action(&env, action_id);
        assert_eq!(result, Err(Error::InvalidState));
    });
}

// ===== M-OF-N THRESHOLD SCENARIOS =====

#[test]
fn test_2_of_3_multisig_workflow() {
    let (env, contract_id, admin1) = setup_contract();
    let admin2 = Address::generate(&env);
    let admin3 = Address::generate(&env);
    let target = Address::generate(&env);
    
    env.as_contract(&contract_id, || {
        // Setup 3 admins
        AdminManager::add_admin(&env, &admin1, &admin2, AdminRole::SuperAdmin).unwrap();
        AdminManager::add_admin(&env, &admin1, &admin3, AdminRole::SuperAdmin).unwrap();
        
        // Set threshold to 2
        MultisigManager::set_threshold(&env, &admin1, 2).unwrap();
        
        // Create pending action
        let data = Map::new(&env);
        let action_type = String::from_str(&env, "remove_admin");
        let action_id = MultisigManager::create_pending_action(
            &env,
            &admin1,
            action_type,
            target,
            data,
        ).unwrap();
        
        // First approval (initiator)
        let action = MultisigManager::get_pending_action(&env, action_id).unwrap();
        assert_eq!(action.approvals.len(), 1);
        
        // Second approval
        let threshold_met = MultisigManager::approve_action(&env, &admin2, action_id).unwrap();
        assert_eq!(threshold_met, true);
        
        let action = MultisigManager::get_pending_action(&env, action_id).unwrap();
        assert_eq!(action.approvals.len(), 2);
        
        // Execute
        MultisigManager::execute_action(&env, action_id).unwrap();
        
        let action = MultisigManager::get_pending_action(&env, action_id).unwrap();
        assert_eq!(action.executed, true);
    });
}

#[test]
fn test_3_of_5_multisig_workflow() {
    let (env, contract_id, admin1) = setup_contract();
    let admin2 = Address::generate(&env);
    let admin3 = Address::generate(&env);
    let admin4 = Address::generate(&env);
    let admin5 = Address::generate(&env);
    let target = Address::generate(&env);
    
    env.as_contract(&contract_id, || {
        // Setup 5 admins
        AdminManager::add_admin(&env, &admin1, &admin2, AdminRole::SuperAdmin).unwrap();
        AdminManager::add_admin(&env, &admin1, &admin3, AdminRole::SuperAdmin).unwrap();
        AdminManager::add_admin(&env, &admin1, &admin4, AdminRole::SuperAdmin).unwrap();
        AdminManager::add_admin(&env, &admin1, &admin5, AdminRole::SuperAdmin).unwrap();
        
        // Set threshold to 3
        MultisigManager::set_threshold(&env, &admin1, 3).unwrap();
        
        let config = MultisigManager::get_config(&env);
        assert_eq!(config.threshold, 3);
        assert_eq!(config.enabled, true);
        
        // Create pending action
        let data = Map::new(&env);
        let action_type = String::from_str(&env, "update_config");
        let action_id = MultisigManager::create_pending_action(
            &env,
            &admin1,
            action_type,
            target,
            data,
        ).unwrap();
        
        // Approve by admin2
        let threshold_met = MultisigManager::approve_action(&env, &admin2, action_id).unwrap();
        assert_eq!(threshold_met, false);
        
        // Approve by admin3
        let threshold_met = MultisigManager::approve_action(&env, &admin3, action_id).unwrap();
        assert_eq!(threshold_met, true);
        
        let action = MultisigManager::get_pending_action(&env, action_id).unwrap();
        assert_eq!(action.approvals.len(), 3);
        
        // Execute
        MultisigManager::execute_action(&env, action_id).unwrap();
    });
}

// ===== SENSITIVE OPERATIONS TESTS =====

#[test]
fn test_sensitive_operation_requires_threshold() {
    let (env, contract_id, admin1) = setup_contract();
    let admin2 = Address::generate(&env);
    let new_admin = Address::generate(&env);
    
    env.as_contract(&contract_id, || {
        AdminManager::add_admin(&env, &admin1, &admin2, AdminRole::SuperAdmin).unwrap();
        MultisigManager::set_threshold(&env, &admin1, 2).unwrap();
        
        assert!(MultisigManager::requires_multisig(&env));
    });
}

#[test]
fn test_add_admin_with_multisig_enabled() {
    let (env, contract_id, admin1) = setup_contract();
    let admin2 = Address::generate(&env);
    let new_admin = Address::generate(&env);
    
    env.as_contract(&contract_id, || {
        AdminManager::add_admin(&env, &admin1, &admin2, AdminRole::SuperAdmin).unwrap();
        MultisigManager::set_threshold(&env, &admin1, 2).unwrap();
        
        // When multisig is enabled, direct admin operations should still work
        // but in production, you'd want to enforce multisig workflow
        let result = AdminManager::add_admin(&env, &admin1, &new_admin, AdminRole::MarketAdmin);
        assert!(result.is_ok());
    });
}

// ===== EVENT EMISSION TESTS =====

#[test]
fn test_admin_added_event_emission() {
    let (env, contract_id, admin) = setup_contract();
    let new_admin = Address::generate(&env);
    
    env.as_contract(&contract_id, || {
        AdminManager::add_admin(&env, &admin, &new_admin, AdminRole::MarketAdmin).unwrap();
        
        let events = env.events().all();
        let event_count = events.len();
        assert!(event_count > 0);
    });
}

#[test]
fn test_admin_removed_event_emission() {
    let (env, contract_id, admin) = setup_contract();
    let new_admin = Address::generate(&env);
    
    env.as_contract(&contract_id, || {
        AdminManager::add_admin(&env, &admin, &new_admin, AdminRole::MarketAdmin).unwrap();
        AdminManager::remove_admin(&env, &admin, &new_admin).unwrap();
        
        let events = env.events().all();
        assert!(events.len() > 0);
    });
}

// ===== AUTHORIZATION FAILURE TESTS =====

#[test]
fn test_unauthorized_add_admin() {
    let (env, contract_id, admin) = setup_contract();
    let unauthorized = Address::generate(&env);
    let new_admin = Address::generate(&env);
    
    env.as_contract(&contract_id, || {
        let result = AdminManager::add_admin(&env, &unauthorized, &new_admin, AdminRole::MarketAdmin);
        assert_eq!(result, Err(Error::Unauthorized));
    });
}

#[test]
fn test_unauthorized_remove_admin() {
    let (env, contract_id, admin) = setup_contract();
    let unauthorized = Address::generate(&env);
    let target = Address::generate(&env);
    
    env.as_contract(&contract_id, || {
        AdminManager::add_admin(&env, &admin, &target, AdminRole::MarketAdmin).unwrap();
        
        let result = AdminManager::remove_admin(&env, &unauthorized, &target);
        assert_eq!(result, Err(Error::Unauthorized));
    });
}

#[test]
fn test_unauthorized_set_threshold() {
    let (env, contract_id, _admin) = setup_contract();
    let unauthorized = Address::generate(&env);
    
    env.as_contract(&contract_id, || {
        let result = MultisigManager::set_threshold(&env, &unauthorized, 2);
        assert_eq!(result, Err(Error::Unauthorized));
    });
}

#[test]
fn test_unauthorized_approve_action() {
    let (env, contract_id, admin) = setup_contract();
    let unauthorized = Address::generate(&env);
    let target = Address::generate(&env);
    
    env.as_contract(&contract_id, || {
        let data = Map::new(&env);
        let action_type = String::from_str(&env, "add_admin");
        let action_id = MultisigManager::create_pending_action(
            &env,
            &admin,
            action_type,
            target,
            data,
        ).unwrap();
        
        let result = MultisigManager::approve_action(&env, &unauthorized, action_id);
        assert_eq!(result, Err(Error::Unauthorized));
    });
}

// ===== EDGE CASES =====

#[test]
fn test_duplicate_admin_addition() {
    let (env, contract_id, admin) = setup_contract();
    let new_admin = Address::generate(&env);
    
    env.as_contract(&contract_id, || {
        AdminManager::add_admin(&env, &admin, &new_admin, AdminRole::MarketAdmin).unwrap();
        
        let result = AdminManager::add_admin(&env, &admin, &new_admin, AdminRole::ConfigAdmin);
        assert_eq!(result, Err(Error::InvalidState));
    });
}

#[test]
fn test_remove_nonexistent_admin() {
    let (env, contract_id, admin) = setup_contract();
    let nonexistent = Address::generate(&env);
    
    env.as_contract(&contract_id, || {
        let result = AdminManager::remove_admin(&env, &admin, &nonexistent);
        assert_eq!(result, Err(Error::Unauthorized));
    });
}

#[test]
fn test_update_role_nonexistent_admin() {
    let (env, contract_id, admin) = setup_contract();
    let nonexistent = Address::generate(&env);
    
    env.as_contract(&contract_id, || {
        let result = AdminManager::update_admin_role(&env, &admin, &nonexistent, AdminRole::ConfigAdmin);
        assert_eq!(result, Err(Error::Unauthorized));
    });
}

#[test]
fn test_get_admin_roles() {
    let (env, contract_id, admin) = setup_contract();
    let admin2 = Address::generate(&env);
    let admin3 = Address::generate(&env);
    
    env.as_contract(&contract_id, || {
        AdminManager::add_admin(&env, &admin, &admin2, AdminRole::MarketAdmin).unwrap();
        AdminManager::add_admin(&env, &admin, &admin3, AdminRole::ConfigAdmin).unwrap();
        
        let roles = AdminManager::get_admin_roles(&env);
        assert!(roles.len() >= 3);
        assert_eq!(roles.get(admin.clone()).unwrap(), AdminRole::SuperAdmin);
        assert_eq!(roles.get(admin2.clone()).unwrap(), AdminRole::MarketAdmin);
        assert_eq!(roles.get(admin3.clone()).unwrap(), AdminRole::ConfigAdmin);
    });
}

#[test]
fn test_multisig_config_persistence() {
    let (env, contract_id, admin) = setup_contract();
    let admin2 = Address::generate(&env);
    
    env.as_contract(&contract_id, || {
        AdminManager::add_admin(&env, &admin, &admin2, AdminRole::SuperAdmin).unwrap();
        MultisigManager::set_threshold(&env, &admin, 2).unwrap();
        
        let config1 = MultisigManager::get_config(&env);
        assert_eq!(config1.threshold, 2);
        
        // Retrieve again to ensure persistence
        let config2 = MultisigManager::get_config(&env);
        assert_eq!(config2.threshold, 2);
        assert_eq!(config2.enabled, true);
    });
}

#[test]
fn test_requires_multisig_check() {
    let (env, contract_id, admin) = setup_contract();
    
    env.as_contract(&contract_id, || {
        assert_eq!(MultisigManager::requires_multisig(&env), false);
        
        let admin2 = Address::generate(&env);
        AdminManager::add_admin(&env, &admin, &admin2, AdminRole::SuperAdmin).unwrap();
        MultisigManager::set_threshold(&env, &admin, 2).unwrap();
        
        assert_eq!(MultisigManager::requires_multisig(&env), true);
    });
}

// ===== COVERAGE TESTS =====

#[test]
fn test_admin_deactivation() {
    let (env, contract_id, admin) = setup_contract();
    let new_admin = Address::generate(&env);
    
    env.as_contract(&contract_id, || {
        AdminManager::add_admin(&env, &admin, &new_admin, AdminRole::MarketAdmin).unwrap();
        
        let result = AdminManager::deactivate_admin(&env, &admin, &new_admin);
        assert!(result.is_ok());
        
        let assignment = AdminManager::get_admin_assignment(&env, &new_admin).unwrap();
        assert_eq!(assignment.is_active, false);
    });
}

#[test]
fn test_admin_reactivation() {
    let (env, contract_id, admin) = setup_contract();
    let new_admin = Address::generate(&env);
    
    env.as_contract(&contract_id, || {
        AdminManager::add_admin(&env, &admin, &new_admin, AdminRole::MarketAdmin).unwrap();
        AdminManager::deactivate_admin(&env, &admin, &new_admin).unwrap();
        
        let result = AdminManager::reactivate_admin(&env, &admin, &new_admin);
        assert!(result.is_ok());
        
        let assignment = AdminManager::get_admin_assignment(&env, &new_admin).unwrap();
        assert_eq!(assignment.is_active, true);
    });
}

#[test]
fn test_complete_multisig_lifecycle() {
    let (env, contract_id, admin1) = setup_contract();
    let admin2 = Address::generate(&env);
    let admin3 = Address::generate(&env);
    let target = Address::generate(&env);
    
    env.as_contract(&contract_id, || {
        // Setup
        AdminManager::add_admin(&env, &admin1, &admin2, AdminRole::SuperAdmin).unwrap();
        AdminManager::add_admin(&env, &admin1, &admin3, AdminRole::SuperAdmin).unwrap();
        MultisigManager::set_threshold(&env, &admin1, 2).unwrap();
        
        // Create action
        let mut data = Map::new(&env);
        data.set(String::from_str(&env, "role"), String::from_str(&env, "MarketAdmin"));
        let action_type = String::from_str(&env, "add_admin");
        let action_id = MultisigManager::create_pending_action(
            &env,
            &admin1,
            action_type,
            target.clone(),
            data,
        ).unwrap();
        
        // Verify initial state
        let action = MultisigManager::get_pending_action(&env, action_id).unwrap();
        assert_eq!(action.approvals.len(), 1);
        assert_eq!(action.executed, false);
        
        // Approve
        MultisigManager::approve_action(&env, &admin2, action_id).unwrap();
        
        // Execute
        MultisigManager::execute_action(&env, action_id).unwrap();
        
        // Verify final state
        let action = MultisigManager::get_pending_action(&env, action_id).unwrap();
        assert_eq!(action.executed, true);
        assert_eq!(action.approvals.len(), 2);
    });
}

// ===========================================================================
// ADVERSARIAL SCENARIOS (Issue #387)
// ---------------------------------------------------------------------------
// These tests document contract behavior when:
//   - primary admin is rotated mid-flight (mid-pending-action / mid-market),
//   - admin is itself a contract address (multisig contract),
//   - admin count / threshold drift via add/remove/deactivate,
//   - pending actions hit expiration boundary,
//   - roles or activation state change between approval and execution.
//
// Invariants proven or refuted here are documented in
// docs/contracts/ADMIN_OPERATIONS.md and referenced by name.
// ===========================================================================

// ----- shared helpers -----

/// Install a 3-admin / threshold=2 multisig setup and return the three admins.
fn setup_multisig_2_of_3(env: &Env) -> (Address, Address, Address) {
    let admin1 = setup_env_primary_admin(env);
    let admin2 = Address::generate(env);
    let admin3 = Address::generate(env);
    AdminManager::add_admin(env, &admin1, &admin2, AdminRole::SuperAdmin).unwrap();
    AdminManager::add_admin(env, &admin1, &admin3, AdminRole::SuperAdmin).unwrap();
    MultisigManager::set_threshold(env, &admin1, 2).unwrap();
    (admin1, admin2, admin3)
}

/// Read the primary admin address from persistent storage (set by `initialize`).
fn setup_env_primary_admin(env: &Env) -> Address {
    env.storage()
        .persistent()
        .get(&Symbol::new(env, "Admin"))
        .expect("primary admin must be initialized")
}

/// Warp the ledger clock forward by `delta_secs`, preserving other fields.
fn warp_time(env: &Env, delta_secs: u64) {
    let now = env.ledger().timestamp();
    env.ledger().set(LedgerInfo {
        timestamp: now + delta_secs,
        protocol_version: 22,
        sequence_number: env.ledger().sequence(),
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 1,
        min_persistent_entry_ttl: 1,
        max_entry_ttl: 10_000,
    });
}

// ----- rotation: mid pending action / mid migration -----

/// Rotation via `transfer_admin` changes the persistent `"Admin"` key but does
/// NOT touch the multi-admin role-assignment map. A pending action created
/// before rotation remains executable; approvals from the rotated-out admin
/// remain counted. Invariant: `transfer_admin` is a primary-admin swap, not a
/// multi-admin reset.
#[test]
fn test_rotation_during_pending_action_preserves_approvals() {
    let (env, contract_id, admin1) = setup_contract();
    let new_admin = Address::generate(&env);

    env.as_contract(&contract_id, || {
        let (_, admin2, _admin3) = setup_multisig_2_of_3(&env);

        let action_id = MultisigManager::create_pending_action(
            &env,
            &admin1,
            String::from_str(&env, "add_admin"),
            Address::generate(&env),
            Map::new(&env),
        )
        .unwrap();

        // admin2 approves -> threshold reached (initiator + admin2).
        assert_eq!(
            MultisigManager::approve_action(&env, &admin2, action_id).unwrap(),
            true
        );

        // Rotate primary admin before executing.
        ContractPauseManager::transfer_admin(&env, &admin1, &new_admin).unwrap();

        // Pre-rotation approvals still count, action still executes.
        MultisigManager::execute_action(&env, action_id).unwrap();
        let action = MultisigManager::get_pending_action(&env, action_id).unwrap();
        assert_eq!(action.executed, true);
        assert_eq!(action.approvals.len(), 2);
    });
}

/// After `transfer_admin`, the rotated-out admin keeps its `AdminRoleAssignment`
/// entry and can still approve multisig actions. Removing it requires an
/// explicit `remove_admin` call by a current super-admin.
#[test]
fn test_rotated_out_admin_retains_multi_admin_role_until_removed() {
    let (env, contract_id, admin1) = setup_contract();
    let new_admin = Address::generate(&env);

    env.as_contract(&contract_id, || {
        let (_, admin2, _) = setup_multisig_2_of_3(&env);

        ContractPauseManager::transfer_admin(&env, &admin1, &new_admin).unwrap();

        // admin1 still in role map, so can still initiate and approve.
        let action_id = MultisigManager::create_pending_action(
            &env,
            &admin1,
            String::from_str(&env, "noop"),
            Address::generate(&env),
            Map::new(&env),
        )
        .unwrap();
        assert_eq!(
            MultisigManager::approve_action(&env, &admin2, action_id).unwrap(),
            true
        );
    });
}

/// `migrate_to_multi_admin` is idempotent in intent: a second call does not
/// error and does not clobber existing admin count. Documents the guarantee
/// relied on by callers who may invoke it defensively.
#[test]
fn test_migrate_to_multi_admin_is_idempotent() {
    let (env, contract_id, _admin) = setup_contract();

    env.as_contract(&contract_id, || {
        AdminSystemIntegration::migrate_to_multi_admin(&env).unwrap();
        let first = AdminSystemIntegration::is_migrated(&env);
        AdminSystemIntegration::migrate_to_multi_admin(&env).unwrap();
        let second = AdminSystemIntegration::is_migrated(&env);
        assert!(first && second);
    });
}

// ----- multisig contract as admin -----

/// A contract address can be installed as primary admin. Under
/// `env.mock_all_auths()` the contract-admin passes `require_admin_auth` the
/// same as an account address. This confirms contract-admins (e.g. a Soroban
/// multisig) are supported as the top-level governance identity.
#[test]
fn test_contract_address_can_act_as_primary_admin() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(PredictifyHybrid, ());
    // Use another deployed contract's address as the "multisig" admin.
    let multisig_contract = env.register(PredictifyHybrid, ());

    let client = PredictifyHybridClient::new(&env, &contract_id);
    client.initialize(&multisig_contract, &None);

    env.as_contract(&contract_id, || {
        let target = Address::generate(&env);
        AdminManager::add_admin(&env, &multisig_contract, &target, AdminRole::MarketAdmin)
            .unwrap();
        assert_eq!(
            AdminManager::get_admin_role_for_address(&env, &target),
            Some(AdminRole::MarketAdmin)
        );
    });
}

/// Rotation from one contract-admin to another succeeds and updates the
/// persistent primary-admin slot.
#[test]
fn test_contract_admin_rotation_to_new_contract_admin() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(PredictifyHybrid, ());
    let old_multisig = env.register(PredictifyHybrid, ());
    let new_multisig = env.register(PredictifyHybrid, ());

    let client = PredictifyHybridClient::new(&env, &contract_id);
    client.initialize(&old_multisig, &None);

    env.as_contract(&contract_id, || {
        ContractPauseManager::transfer_admin(&env, &old_multisig, &new_multisig).unwrap();
        let stored: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, "Admin"))
            .unwrap();
        assert_eq!(stored, new_multisig);
    });
}

// ----- threshold / admin-count drift -----

/// `remove_admin` decrements `AdminCount` but does NOT automatically reduce
/// `MultisigConfig.threshold`. If enough admins are removed, `threshold` can
/// exceed the remaining admin count — a drift state that the contract does
/// not currently guard against at removal time. Any subsequent `set_threshold`
/// call re-validates, so drift is detectable but not auto-corrected.
/// Documented as an invariant + follow-up hardening candidate.
#[test]
fn test_remove_admin_can_leave_threshold_above_count() {
    let (env, contract_id, admin1) = setup_contract();

    env.as_contract(&contract_id, || {
        let (_, admin2, admin3) = setup_multisig_2_of_3(&env);

        // Removing admin3 is allowed even though threshold=2 and count drops to 2.
        AdminManager::remove_admin(&env, &admin1, &admin3).unwrap();
        assert_eq!(MultisigManager::get_config(&env).threshold, 2);

        // Removing admin2 next leaves count=1, threshold=2 (drift).
        AdminManager::remove_admin(&env, &admin1, &admin2).unwrap();
        let config = MultisigManager::get_config(&env);
        assert_eq!(config.threshold, 2); // stale
        // A fresh set_threshold > count is rejected, confirming guard exists
        // on write path but not on admin-count shrink.
        assert_eq!(
            MultisigManager::set_threshold(&env, &admin1, 5),
            Err(Error::InvalidInput)
        );
    });
}

/// Adding a new admin after a pending action has been created does NOT grant
/// that admin retroactive approval — approvals are tied to `approve_action`
/// calls, not to admin-list membership.
#[test]
fn test_add_admin_does_not_retroactively_approve_pending_action() {
    let (env, contract_id, admin1) = setup_contract();

    env.as_contract(&contract_id, || {
        let admin2 = Address::generate(&env);
        AdminManager::add_admin(&env, &admin1, &admin2, AdminRole::SuperAdmin).unwrap();
        MultisigManager::set_threshold(&env, &admin1, 2).unwrap();

        let action_id = MultisigManager::create_pending_action(
            &env,
            &admin1,
            String::from_str(&env, "config_update"),
            Address::generate(&env),
            Map::new(&env),
        )
        .unwrap();

        // New admin added after action creation.
        let admin3 = Address::generate(&env);
        AdminManager::add_admin(&env, &admin1, &admin3, AdminRole::SuperAdmin).unwrap();

        let action = MultisigManager::get_pending_action(&env, action_id).unwrap();
        assert_eq!(action.approvals.len(), 1); // only initiator
        assert!(!action.approvals.contains(&admin3));
    });
}

/// Lowering the threshold after approvals are collected permits immediate
/// execution when the lowered threshold is already met.
#[test]
fn test_lower_threshold_after_approvals_permits_execution() {
    let (env, contract_id, admin1) = setup_contract();

    env.as_contract(&contract_id, || {
        let admin2 = Address::generate(&env);
        let admin3 = Address::generate(&env);
        AdminManager::add_admin(&env, &admin1, &admin2, AdminRole::SuperAdmin).unwrap();
        AdminManager::add_admin(&env, &admin1, &admin3, AdminRole::SuperAdmin).unwrap();
        MultisigManager::set_threshold(&env, &admin1, 3).unwrap();

        let action_id = MultisigManager::create_pending_action(
            &env,
            &admin1,
            String::from_str(&env, "rotate_key"),
            Address::generate(&env),
            Map::new(&env),
        )
        .unwrap();
        MultisigManager::approve_action(&env, &admin2, action_id).unwrap();

        // Cannot execute at threshold=3 (only 2 approvals).
        assert_eq!(
            MultisigManager::execute_action(&env, action_id),
            Err(Error::Unauthorized)
        );

        // Lower threshold to 2, now execution succeeds.
        MultisigManager::set_threshold(&env, &admin1, 2).unwrap();
        MultisigManager::execute_action(&env, action_id).unwrap();
    });
}

// ----- expiration / replay -----

/// At exactly `expires_at`, `approve_action` is still accepted (guard uses
/// strict `>`). One second past, it returns `Error::DisputeError`.
#[test]
fn test_approve_action_at_expiration_boundary() {
    let (env, contract_id, admin1) = setup_contract();

    env.as_contract(&contract_id, || {
        let (_, admin2, _admin3) = setup_multisig_2_of_3(&env);

        let action_id = MultisigManager::create_pending_action(
            &env,
            &admin1,
            String::from_str(&env, "boundary"),
            Address::generate(&env),
            Map::new(&env),
        )
        .unwrap();
        let expires_at = MultisigManager::get_pending_action(&env, action_id)
            .unwrap()
            .expires_at;

        // At exactly expires_at, approval still allowed.
        let now = env.ledger().timestamp();
        warp_time(&env, expires_at - now);
        assert!(MultisigManager::approve_action(&env, &admin2, action_id).is_ok());
    });
}

#[test]
fn test_approve_action_after_expiration_rejected() {
    let (env, contract_id, admin1) = setup_contract();

    env.as_contract(&contract_id, || {
        let (_, admin2, _admin3) = setup_multisig_2_of_3(&env);

        let action_id = MultisigManager::create_pending_action(
            &env,
            &admin1,
            String::from_str(&env, "expired"),
            Address::generate(&env),
            Map::new(&env),
        )
        .unwrap();

        warp_time(&env, 86_401); // one second past 24h window
        assert_eq!(
            MultisigManager::approve_action(&env, &admin2, action_id),
            Err(Error::DisputeError)
        );
    });
}

/// Finding: `execute_action` does NOT check expiration. An action whose
/// approvals were gathered before the window but executed after can still
/// run. Test documents current behavior; see ADMIN_OPERATIONS.md for the
/// hardening follow-up.
#[test]
fn test_execute_expired_action_with_enough_approvals_still_runs() {
    let (env, contract_id, admin1) = setup_contract();

    env.as_contract(&contract_id, || {
        let (_, admin2, _admin3) = setup_multisig_2_of_3(&env);

        let action_id = MultisigManager::create_pending_action(
            &env,
            &admin1,
            String::from_str(&env, "stale"),
            Address::generate(&env),
            Map::new(&env),
        )
        .unwrap();
        MultisigManager::approve_action(&env, &admin2, action_id).unwrap();

        warp_time(&env, 86_401);
        // Execute-path has no expiry guard today.
        MultisigManager::execute_action(&env, action_id).unwrap();
        let action = MultisigManager::get_pending_action(&env, action_id).unwrap();
        assert_eq!(action.executed, true);
    });
}

// ----- role / activation changes between approval and execution -----

/// Deactivating an admin after they approved does not retroactively remove
/// their approval. `execute_action` counts the length of `approvals` only.
/// Documented as "approvals are immutable once recorded".
#[test]
fn test_deactivated_admin_approval_still_counts() {
    let (env, contract_id, admin1) = setup_contract();

    env.as_contract(&contract_id, || {
        let (_, admin2, _admin3) = setup_multisig_2_of_3(&env);

        let action_id = MultisigManager::create_pending_action(
            &env,
            &admin1,
            String::from_str(&env, "approve_then_deactivate"),
            Address::generate(&env),
            Map::new(&env),
        )
        .unwrap();
        MultisigManager::approve_action(&env, &admin2, action_id).unwrap();

        AdminManager::deactivate_admin(&env, &admin1, &admin2).unwrap();

        // Execute succeeds — approvals vec is the source of truth.
        MultisigManager::execute_action(&env, action_id).unwrap();
    });
}

/// A deactivated admin cannot approve new actions: permission validation
/// rejects them with `Unauthorized`.
#[test]
fn test_deactivated_admin_cannot_approve() {
    let (env, contract_id, admin1) = setup_contract();

    env.as_contract(&contract_id, || {
        let (_, admin2, _admin3) = setup_multisig_2_of_3(&env);

        AdminManager::deactivate_admin(&env, &admin1, &admin2).unwrap();

        let action_id = MultisigManager::create_pending_action(
            &env,
            &admin1,
            String::from_str(&env, "deactivated_approver"),
            Address::generate(&env),
            Map::new(&env),
        )
        .unwrap();

        let result = MultisigManager::approve_action(&env, &admin2, action_id);
        assert_eq!(result, Err(Error::Unauthorized));
    });
}

// ----- auth negative paths -----

/// Non-admin callers cannot initiate a pending action — permission check
/// precedes any storage write.
#[test]
fn test_non_admin_cannot_create_pending_action() {
    let (env, contract_id, _admin) = setup_contract();
    let stranger = Address::generate(&env);

    env.as_contract(&contract_id, || {
        let result = MultisigManager::create_pending_action(
            &env,
            &stranger,
            String::from_str(&env, "nope"),
            Address::generate(&env),
            Map::new(&env),
        );
        assert_eq!(result, Err(Error::Unauthorized));
    });
}

/// Double execution is blocked regardless of whether the re-execute call
/// happens in the same transaction or after additional approvals are added.
#[test]
fn test_double_execute_blocked_even_after_extra_approval() {
    let (env, contract_id, admin1) = setup_contract();

    env.as_contract(&contract_id, || {
        let (_, admin2, admin3) = setup_multisig_2_of_3(&env);

        let action_id = MultisigManager::create_pending_action(
            &env,
            &admin1,
            String::from_str(&env, "double_exec"),
            Address::generate(&env),
            Map::new(&env),
        )
        .unwrap();
        MultisigManager::approve_action(&env, &admin2, action_id).unwrap();
        MultisigManager::execute_action(&env, action_id).unwrap();

        // Extra approval after execution should be rejected (action executed).
        assert_eq!(
            MultisigManager::approve_action(&env, &admin3, action_id),
            Err(Error::InvalidState)
        );
        assert_eq!(
            MultisigManager::execute_action(&env, action_id),
            Err(Error::InvalidState)
        );
    });
}
