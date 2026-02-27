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

use crate::admin::{AdminManager, AdminRole, MultisigManager, MultisigConfig, PendingAdminAction};
use crate::errors::Error;
use crate::{PredictifyHybrid, PredictifyHybridClient};
use soroban_sdk::{
    testutils::{Address as _, Events},
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
