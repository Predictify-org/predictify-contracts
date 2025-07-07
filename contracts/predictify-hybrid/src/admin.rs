use soroban_sdk::{contracttype, vec, Address, Env, Map, String, Symbol, Vec};
use alloc::string::ToString;

use crate::errors::Error;
use crate::markets::MarketStateManager;
use crate::fees::{FeeManager, FeeConfig};
use crate::config::{ConfigManager, ContractConfig, Environment, ConfigUtils};
use crate::resolution::MarketResolutionManager;
use crate::extensions::ExtensionManager;
use crate::events::EventEmitter;

/// Admin management system for Predictify Hybrid contract
///
/// This module provides a comprehensive admin system with:
/// - Admin initialization and setup functions
/// - Access control and permission validation
/// - Admin role management and hierarchy
/// - Admin action logging and tracking
/// - Admin helper utilities and testing functions
/// - Admin event emission and monitoring

// ===== ADMIN TYPES =====

/// Admin role enumeration
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[contracttype]
pub enum AdminRole {
    /// Super admin with all permissions
    SuperAdmin,
    /// Market admin with market management permissions
    MarketAdmin,
    /// Config admin with configuration permissions
    ConfigAdmin,
    /// Fee admin with fee management permissions
    FeeAdmin,
    /// Read-only admin with view permissions only
    ReadOnlyAdmin,
}

/// Admin permission enumeration
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[contracttype]
pub enum AdminPermission {
    /// Initialize contract
    Initialize,
    /// Create markets
    CreateMarket,
    /// Close markets
    CloseMarket,
    /// Finalize markets
    FinalizeMarket,
    /// Extend market duration
    ExtendMarket,
    /// Update fee configuration
    UpdateFees,
    /// Update contract configuration
    UpdateConfig,
    /// Reset configuration
    ResetConfig,
    /// Collect fees
    CollectFees,
    /// Manage disputes
    ManageDisputes,
    /// View analytics
    ViewAnalytics,
    /// Emergency actions
    EmergencyActions,
}

/// Admin action record
#[derive(Clone, Debug)]
#[contracttype]
pub struct AdminAction {
    pub admin: Address,
    pub action: String,
    pub target: Option<String>,
    pub parameters: Map<String, String>,
    pub timestamp: u64,
    pub success: bool,
    pub error_message: Option<String>,
}

/// Admin role assignment
#[derive(Clone, Debug)]
#[contracttype]
pub struct AdminRoleAssignment {
    pub admin: Address,
    pub role: AdminRole,
    pub assigned_by: Address,
    pub assigned_at: u64,
    pub permissions: Vec<AdminPermission>,
    pub is_active: bool,
}

/// Admin analytics
#[derive(Clone, Debug)]
#[contracttype]
pub struct AdminAnalytics {
    pub total_admins: u32,
    pub active_admins: u32,
    pub total_actions: u32,
    pub successful_actions: u32,
    pub failed_actions: u32,
    pub action_distribution: Map<String, u32>,
    pub role_distribution: Map<String, u32>,
    pub recent_actions: Vec<AdminAction>,
}

// ===== ADMIN INITIALIZATION =====

/// Admin initialization management
pub struct AdminInitializer;

impl AdminInitializer {
    /// Initialize contract with admin
    pub fn initialize(env: &Env, admin: &Address) -> Result<(), Error> {
        // Validate admin address
        AdminValidator::validate_admin_address(env, admin)?;

        // Store admin in persistent storage
        env.storage()
            .persistent()
            .set(&Symbol::new(env, "Admin"), admin);

        // Set default admin role
        AdminRoleManager::assign_role(
            env,
            admin,
            AdminRole::SuperAdmin,
            admin,
        )?;

        // Emit admin initialization event
        EventEmitter::emit_admin_initialized(env, admin);

        // Log admin action
        AdminActionLogger::log_action(
            env,
            admin,
            "initialize",
            None,
            Map::new(env),
            true,
            None,
        )?;

        Ok(())
    }

    /// Initialize contract with configuration
    pub fn initialize_with_config(
        env: &Env,
        admin: &Address,
        environment: &Environment,
    ) -> Result<(), Error> {
        // Initialize basic admin setup
        AdminInitializer::initialize(env, admin)?;

        let config = match environment {
            Environment::Development => ConfigManager::get_development_config(env),
            Environment::Testnet => ConfigManager::get_testnet_config(env),
            Environment::Mainnet => ConfigManager::get_mainnet_config(env),
            Environment::Custom => ConfigManager::get_development_config(env),
        };
        ConfigManager::store_config(env, &config)?;

        // Emit configuration initialization event
        EventEmitter::emit_config_initialized(env, admin, environment);

        Ok(())
    }

    /// Validate initialization parameters
    pub fn validate_initialization_params(
        env: &Env,
        admin: &Address,
    ) -> Result<(), Error> {
        AdminValidator::validate_admin_address(env, admin)?;
        AdminValidator::validate_contract_not_initialized(env)?;
        Ok(())
    }
}

// ===== ADMIN ACCESS CONTROL =====

/// Admin access control management
pub struct AdminAccessControl;

impl AdminAccessControl {
    /// Validate admin permissions for an action
    pub fn validate_permission(
        env: &Env,
        admin: &Address,
        permission: &AdminPermission,
    ) -> Result<(), Error> {
        // Get admin role
        let role = AdminRoleManager::get_admin_role(env, admin)?;

        // Check if admin has the required permission
        if !AdminRoleManager::has_permission(env, &role, permission)? {
            return Err(Error::Unauthorized);
        }

        Ok(())
    }

    /// Require admin authentication
    pub fn require_admin_auth(env: &Env, admin: &Address) -> Result<(), Error> {
        // Verify admin authentication
        admin.require_auth();

        // Validate admin exists
        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(env, "Admin"))
            .ok_or(Error::AdminNotSet)?;

        if admin != &stored_admin {
            return Err(Error::Unauthorized);
        }

        Ok(())
    }

    /// Validate admin for specific action
    pub fn validate_admin_for_action(
        env: &Env,
        admin: &Address,
        action: &str,
    ) -> Result<(), Error> {
        // Require admin authentication
        AdminAccessControl::require_admin_auth(env, admin)?;

        // Map action to permission
        let permission = AdminAccessControl::map_action_to_permission(action)?;

        // Validate permission
        AdminAccessControl::validate_permission(env, admin, &permission)?;

        Ok(())
    }

    /// Map action string to permission enum
    pub fn map_action_to_permission(action: &str) -> Result<AdminPermission, Error> {
        match action {
            "initialize" => Ok(AdminPermission::Initialize),
            "create_market" => Ok(AdminPermission::CreateMarket),
            "close_market" => Ok(AdminPermission::CloseMarket),
            "finalize_market" => Ok(AdminPermission::FinalizeMarket),
            "extend_market" => Ok(AdminPermission::ExtendMarket),
            "update_fees" => Ok(AdminPermission::UpdateFees),
            "update_config" => Ok(AdminPermission::UpdateConfig),
            "reset_config" => Ok(AdminPermission::ResetConfig),
            "collect_fees" => Ok(AdminPermission::CollectFees),
            "manage_disputes" => Ok(AdminPermission::ManageDisputes),
            "view_analytics" => Ok(AdminPermission::ViewAnalytics),
            "emergency_actions" => Ok(AdminPermission::EmergencyActions),
            _ => Err(Error::InvalidInput),
        }
    }
}

// ===== ADMIN ROLE MANAGEMENT =====

/// Admin role management
pub struct AdminRoleManager;

impl AdminRoleManager {
    /// Assign role to admin
    pub fn assign_role(
        env: &Env,
        admin: &Address,
        role: AdminRole,
        assigned_by: &Address,
    ) -> Result<(), Error> {
        // Validate assigner permissions
        AdminAccessControl::validate_permission(
            env,
            assigned_by,
            &AdminPermission::EmergencyActions,
        )?;

        // Create role assignment
        let assignment = AdminRoleAssignment {
            admin: admin.clone(),
            role,
            assigned_by: assigned_by.clone(),
            assigned_at: env.ledger().timestamp(),
            permissions: AdminRoleManager::get_permissions_for_role(&role),
            is_active: true,
        };

        // Store role assignment
        let key = Symbol::new(env, "admin_role");
        env.storage().persistent().set(&key, &assignment);

        // Emit role assignment event
        EventEmitter::emit_admin_role_assigned(env, admin, &role, assigned_by);

        Ok(())
    }

    /// Get admin role
    pub fn get_admin_role(env: &Env, _admin: &Address) -> Result<AdminRole, Error> {
        let key = Symbol::new(env, "admin_role");
        let assignment: AdminRoleAssignment = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(Error::Unauthorized)?;

        if !assignment.is_active {
            return Err(Error::Unauthorized);
        }

        Ok(assignment.role)
    }

    /// Check if admin has permission
    pub fn has_permission(
        _env: &Env,
        role: &AdminRole,
        permission: &AdminPermission,
    ) -> Result<bool, Error> {
        let permissions = AdminRoleManager::get_permissions_for_role(role);
        Ok(permissions.contains(permission))
    }

    /// Get permissions for role
    pub fn get_permissions_for_role(role: &AdminRole) -> Vec<AdminPermission> {
        let env = soroban_sdk::Env::default();
        match role {
            AdminRole::SuperAdmin => vec![
                &env,
                AdminPermission::Initialize,
                AdminPermission::CreateMarket,
                AdminPermission::CloseMarket,
                AdminPermission::FinalizeMarket,
                AdminPermission::ExtendMarket,
                AdminPermission::UpdateFees,
                AdminPermission::UpdateConfig,
                AdminPermission::ResetConfig,
                AdminPermission::CollectFees,
                AdminPermission::ManageDisputes,
                AdminPermission::ViewAnalytics,
                AdminPermission::EmergencyActions,
            ],
            AdminRole::MarketAdmin => vec![
                &env,
                AdminPermission::CreateMarket,
                AdminPermission::CloseMarket,
                AdminPermission::FinalizeMarket,
                AdminPermission::ExtendMarket,
                AdminPermission::ViewAnalytics,
            ],
            AdminRole::ConfigAdmin => vec![
                &env,
                AdminPermission::UpdateConfig,
                AdminPermission::ResetConfig,
                AdminPermission::ViewAnalytics,
            ],
            AdminRole::FeeAdmin => vec![
                &env,
                AdminPermission::UpdateFees,
                AdminPermission::CollectFees,
                AdminPermission::ViewAnalytics,
            ],
            AdminRole::ReadOnlyAdmin => vec![
                &env,
                AdminPermission::ViewAnalytics,
            ],
        }
    }

    /// Deactivate admin role
    pub fn deactivate_role(
        env: &Env,
        admin: &Address,
        deactivated_by: &Address,
    ) -> Result<(), Error> {
        // Validate deactivator permissions
        AdminAccessControl::validate_permission(
            env,
            deactivated_by,
            &AdminPermission::EmergencyActions,
        )?;

        let key = Symbol::new(env, "admin_role");
        let mut assignment: AdminRoleAssignment = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(Error::Unauthorized)?;

        assignment.is_active = false;
        env.storage().persistent().set(&key, &assignment);

        // Emit role deactivation event
        EventEmitter::emit_admin_role_deactivated(env, admin, deactivated_by);

        Ok(())
    }
}

// ===== ADMIN FUNCTIONS =====

/// Admin function management
pub struct AdminFunctions;

impl AdminFunctions {
    /// Close market (admin only)
    pub fn close_market(
        env: &Env,
        admin: &Address,
        market_id: &Symbol,
    ) -> Result<(), Error> {
        // Validate admin permissions
        AdminAccessControl::validate_admin_for_action(env, admin, "close_market")?;

        // Get market
        let _market = MarketStateManager::get_market(env, market_id)?;

        // Close market
        MarketStateManager::remove_market(env, market_id);

        // Emit market closed event
        EventEmitter::emit_market_closed(env, market_id, admin);

        // Log admin action
        let mut params = Map::new(env);
        params.set(String::from_str(env, "market_id"), String::from_str(env, &market_id.to_string()));
        AdminActionLogger::log_action(env, admin, "close_market", None, params, true, None)?;

        Ok(())
    }

    /// Finalize market with admin override
    pub fn finalize_market(
        env: &Env,
        admin: &Address,
        market_id: &Symbol,
        outcome: &String,
    ) -> Result<(), Error> {
        // Validate admin permissions
        AdminAccessControl::validate_admin_for_action(env, admin, "finalize_market")?;

        // Finalize market using resolution manager
        let resolution = MarketResolutionManager::finalize_market(env, admin, market_id, outcome)?;

        // Emit market finalized event
        EventEmitter::emit_market_finalized(env, market_id, admin, outcome);

        // Log admin action
        let mut params = Map::new(env);
        params.set(String::from_str(env, "market_id"), String::from_str(env, &market_id.to_string()));
        params.set(String::from_str(env, "outcome"), outcome.clone());
        AdminActionLogger::log_action(env, admin, "finalize_market", Some(String::from_str(env, &market_id.to_string())), params, true, None)?;

        Ok(())
    }

    /// Extend market duration
    pub fn extend_market_duration(
        env: &Env,
        admin: &Address,
        market_id: &Symbol,
        additional_days: u32,
        reason: &String,
    ) -> Result<(), Error> {
        // Validate admin permissions
        AdminAccessControl::validate_admin_for_action(env, admin, "extend_market")?;

        // Extend market using extension manager
        ExtensionManager::extend_market_duration(env, admin.clone(), market_id.clone(), additional_days, reason.clone())?;

        // Log admin action
        let mut params = Map::new(env);
        params.set(String::from_str(env, "market_id"), String::from_str(env, &market_id.to_string()));
        params.set(String::from_str(env, "additional_days"), String::from_str(env, &additional_days.to_string()));
        params.set(String::from_str(env, "reason"), reason.clone());
        AdminActionLogger::log_action(env, admin, "extend_market", Some(String::from_str(env, &market_id.to_string())), params, true, None)?;

        Ok(())
    }

    /// Update fee configuration
    pub fn update_fee_config(
        env: &Env,
        admin: &Address,
        new_config: &FeeConfig,
    ) -> Result<FeeConfig, Error> {
        // Validate admin permissions
        AdminAccessControl::validate_admin_for_action(env, admin, "update_fees")?;

        // Update fee configuration
        let updated_config = FeeManager::update_fee_config(env, admin.clone(), new_config.clone())?;

        // Log admin action
        let mut params = Map::new(env);
        params.set(String::from_str(env, "platform_fee"), String::from_str(env, &new_config.platform_fee_percentage.to_string()));
        params.set(String::from_str(env, "creation_fee"), String::from_str(env, &new_config.creation_fee.to_string()));
        AdminActionLogger::log_action(env, admin, "update_fees", None, params, true, None)?;

        Ok(updated_config)
    }

    /// Update contract configuration
    pub fn update_contract_config(
        env: &Env,
        admin: &Address,
        new_config: &ContractConfig,
    ) -> Result<(), Error> {
        // Validate admin permissions
        AdminAccessControl::validate_admin_for_action(env, admin, "update_config")?;

        // Update contract configuration
        ConfigManager::update_config(env, &new_config)?;
        let env_name = ConfigUtils::get_environment_name(&new_config);
        let mut params = Map::new(env);
        params.set(String::from_str(env, "environment"), env_name);
        AdminActionLogger::log_action(env, admin, "update_config", None, params, true, None)?;

        Ok(())
    }

    /// Reset configuration to defaults
    pub fn reset_config_to_defaults(
        env: &Env,
        admin: &Address,
    ) -> Result<ContractConfig, Error> {
        // Validate admin permissions
        AdminAccessControl::validate_admin_for_action(env, admin, "reset_config")?;

        // Reset configuration
        let default_config = ConfigManager::reset_to_defaults(env)?;

        // Log admin action
        AdminActionLogger::log_action(env, admin, "reset_config", None, Map::new(env), true, None)?;

        Ok(default_config)
    }
}

// ===== ADMIN VALIDATION =====

/// Admin validation utilities
pub struct AdminValidator;

impl AdminValidator {
    /// Validate admin address
    pub fn validate_admin_address(env: &Env, admin: &Address) -> Result<(), Error> {
        // Check if address is valid
        if admin.to_string().is_empty() {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }

    /// Validate contract not already initialized
    pub fn validate_contract_not_initialized(env: &Env) -> Result<(), Error> {
        let admin_exists = env
            .storage()
            .persistent()
            .has(&Symbol::new(env, "Admin"));

        if admin_exists {
            return Err(Error::InvalidState);
        }

        Ok(())
    }

    /// Validate admin action parameters
    pub fn validate_action_parameters(
        env: &Env,
        action: &str,
        parameters: &Map<String, String>,
    ) -> Result<(), Error> {
        match action {
            "close_market" => {
                let market_id = parameters.get(String::from_str(env, "market_id"))
                    .ok_or(Error::InvalidInput)?;
                if market_id.is_empty() {
                    return Err(Error::InvalidInput);
                }
            }
            "finalize_market" => {
                let market_id = parameters.get(String::from_str(env, "market_id"))
                    .ok_or(Error::InvalidInput)?;
                let outcome = parameters.get(String::from_str(env, "outcome"))
                    .ok_or(Error::InvalidInput)?;
                if market_id.is_empty() || outcome.is_empty() {
                    return Err(Error::InvalidInput);
                }
            }
            "extend_market" => {
                let market_id = parameters.get(String::from_str(env, "market_id"))
                    .ok_or(Error::InvalidInput)?;
                let additional_days = parameters.get(String::from_str(env, "additional_days"))
                    .ok_or(Error::InvalidInput)?;
                if market_id.is_empty() || additional_days.is_empty() {
                    return Err(Error::InvalidInput);
                }
            }
            _ => {}
        }

        Ok(())
    }
}

// ===== ADMIN ACTION LOGGING =====

/// Admin action logging
pub struct AdminActionLogger;

impl AdminActionLogger {
    /// Log admin action
    pub fn log_action(
        env: &Env,
        admin: &Address,
        action: &str,
        target: Option<String>,
        parameters: Map<String, String>,
        success: bool,
        error_message: Option<String>,
    ) -> Result<(), Error> {
        let admin_action = AdminAction {
            admin: admin.clone(),
            action: String::from_str(env, action),
            target,
            parameters,
            timestamp: env.ledger().timestamp(),
            success,
            error_message,
        };

        // Store action in persistent storage
        let action_key = Symbol::new(env, "admin_action");
        env.storage().persistent().set(&action_key, &admin_action);

        // Emit admin action event
        EventEmitter::emit_admin_action_logged(env, admin, action, &success);

        Ok(())
    }

    /// Get admin actions
    pub fn get_admin_actions(env: &Env, limit: u32) -> Result<Vec<AdminAction>, Error> {
        // For now, return empty vector since we don't have a way to iterate over storage
        // In a real implementation, you would store actions in a more sophisticated way
        Ok(Vec::new(env))
    }

    /// Get admin actions for specific admin
    pub fn get_admin_actions_for_admin(
        env: &Env,
        admin: &Address,
        limit: u32,
    ) -> Result<Vec<AdminAction>, Error> {
        // For now, return empty vector
        Ok(Vec::new(env))
    }
}

// ===== ADMIN ANALYTICS =====

/// Admin analytics
impl AdminAnalytics {
    /// Calculate admin analytics
    pub fn calculate_admin_analytics(env: &Env) -> Result<AdminAnalytics, Error> {
        // For now, return default analytics since we don't store complex types
        Ok(AdminAnalytics::default())
    }

    /// Get admin role distribution
    pub fn get_role_distribution(env: &Env) -> Result<Map<AdminRole, u32>, Error> {
        // For now, return empty map
        Ok(Map::new(env))
    }

    /// Get action distribution
    pub fn get_action_distribution(env: &Env) -> Result<Map<String, u32>, Error> {
        // For now, return empty map
        Ok(Map::new(env))
    }
}

// ===== ADMIN UTILITIES =====

/// Admin utility functions
pub struct AdminUtils;

impl AdminUtils {
    /// Check if address is admin
    pub fn is_admin(env: &Env, address: &Address) -> bool {
        AdminRoleManager::get_admin_role(env, address).is_ok()
    }

    /// Check if address is super admin
    pub fn is_super_admin(env: &Env, address: &Address) -> bool {
        match AdminRoleManager::get_admin_role(env, address) {
            Ok(role) => role == AdminRole::SuperAdmin,
            Err(_) => false,
        }
    }

    /// Get admin role name
    pub fn get_role_name(role: &AdminRole) -> String {
        match role {
            AdminRole::SuperAdmin => String::from_str(&soroban_sdk::Env::default(), "SuperAdmin"),
            AdminRole::MarketAdmin => String::from_str(&soroban_sdk::Env::default(), "MarketAdmin"),
            AdminRole::ConfigAdmin => String::from_str(&soroban_sdk::Env::default(), "ConfigAdmin"),
            AdminRole::FeeAdmin => String::from_str(&soroban_sdk::Env::default(), "FeeAdmin"),
            AdminRole::ReadOnlyAdmin => String::from_str(&soroban_sdk::Env::default(), "ReadOnlyAdmin"),
        }
    }

    /// Get permission name
    pub fn get_permission_name(permission: &AdminPermission) -> String {
        match permission {
            AdminPermission::Initialize => String::from_str(&soroban_sdk::Env::default(), "Initialize"),
            AdminPermission::CreateMarket => String::from_str(&soroban_sdk::Env::default(), "CreateMarket"),
            AdminPermission::CloseMarket => String::from_str(&soroban_sdk::Env::default(), "CloseMarket"),
            AdminPermission::FinalizeMarket => String::from_str(&soroban_sdk::Env::default(), "FinalizeMarket"),
            AdminPermission::ExtendMarket => String::from_str(&soroban_sdk::Env::default(), "ExtendMarket"),
            AdminPermission::UpdateFees => String::from_str(&soroban_sdk::Env::default(), "UpdateFees"),
            AdminPermission::UpdateConfig => String::from_str(&soroban_sdk::Env::default(), "UpdateConfig"),
            AdminPermission::ResetConfig => String::from_str(&soroban_sdk::Env::default(), "ResetConfig"),
            AdminPermission::CollectFees => String::from_str(&soroban_sdk::Env::default(), "CollectFees"),
            AdminPermission::ManageDisputes => String::from_str(&soroban_sdk::Env::default(), "ManageDisputes"),
            AdminPermission::ViewAnalytics => String::from_str(&soroban_sdk::Env::default(), "ViewAnalytics"),
            AdminPermission::EmergencyActions => String::from_str(&soroban_sdk::Env::default(), "EmergencyActions"),
        }
    }
}

// ===== ADMIN TESTING =====

/// Admin testing utilities
pub struct AdminTesting;

impl AdminTesting {
    /// Create test admin action
    pub fn create_test_admin_action(env: &Env, admin: &Address) -> AdminAction {
        AdminAction {
            admin: admin.clone(),
            action: String::from_str(env, "test_action"),
            target: Some(String::from_str(env, "test_target")),
            parameters: Map::new(env),
            timestamp: env.ledger().timestamp(),
            success: true,
            error_message: None,
        }
    }

    /// Create test admin role assignment
    pub fn create_test_role_assignment(env: &Env, admin: &Address) -> AdminRoleAssignment {
        AdminRoleAssignment {
            admin: admin.clone(),
            role: AdminRole::MarketAdmin,
            assigned_by: admin.clone(),
            assigned_at: env.ledger().timestamp(),
            permissions: AdminRoleManager::get_permissions_for_role(&AdminRole::MarketAdmin),
            is_active: true,
        }
    }

    /// Validate admin action structure
    pub fn validate_admin_action_structure(action: &AdminAction) -> Result<(), Error> {
        if action.action.is_empty() {
            return Err(Error::InvalidInput);
        }

        if action.timestamp == 0 {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }

    /// Simulate admin action
    pub fn simulate_admin_action(
        env: &Env,
        admin: &Address,
        action: &str,
    ) -> Result<(), Error> {
        // Log test action
        AdminActionLogger::log_action(
            env,
            admin,
            action,
            Some(String::from_str(env, "test_target")),
            Map::new(env),
            true,
            None,
        )?;

        Ok(())
    }
}

// ===== DEFAULT IMPLEMENTATIONS =====

impl Default for AdminAnalytics {
    fn default() -> Self {
        let env = soroban_sdk::Env::default();
        Self {
            total_admins: 0,
            active_admins: 0,
            total_actions: 0,
            successful_actions: 0,
            failed_actions: 0,
            action_distribution: Map::new(&env),
            role_distribution: Map::new(&env),
            recent_actions: Vec::new(&env),
        }
    }
}

// ===== MODULE TESTS =====

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger, LedgerInfo};

    #[test]
    fn test_admin_initializer_initialize() {
        let env = Env::default();
        let admin = Address::generate(&env);

        // Test initialization
        assert!(AdminInitializer::initialize(&env, &admin).is_ok());

        // Verify admin is stored
        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, "Admin"))
            .unwrap();
        assert_eq!(stored_admin, admin);
    }

    #[test]
    fn test_admin_access_control_validate_permission() {
        let env = Env::default();
        let admin = Address::generate(&env);

        // Initialize admin
        AdminInitializer::initialize(&env, &admin).unwrap();

        // Test permission validation
        assert!(AdminAccessControl::validate_permission(
            &env,
            &admin,
            &AdminPermission::CreateMarket
        ).is_ok());
    }

    #[test]
    fn test_admin_role_manager_assign_role() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let new_admin = Address::generate(&env);

        // Initialize admin
        AdminInitializer::initialize(&env, &admin).unwrap();

        // Assign role
        assert!(AdminRoleManager::assign_role(
            &env,
            &new_admin,
            AdminRole::MarketAdmin,
            &admin
        ).is_ok());

        // Verify role assignment
        let role = AdminRoleManager::get_admin_role(&env, &new_admin).unwrap();
        assert_eq!(role, AdminRole::MarketAdmin);
    }

    #[test]
    fn test_admin_functions_close_market() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let market_id = Symbol::new(&env, "test_market");

        // Initialize admin
        AdminInitializer::initialize(&env, &admin).unwrap();

        // Test close market (would need a real market setup)
        // For now, just test the validation
        assert!(AdminAccessControl::validate_admin_for_action(
            &env,
            &admin,
            "close_market"
        ).is_ok());
    }

    #[test]
    fn test_admin_utils_is_admin() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let non_admin = Address::generate(&env);

        // Initialize admin
        AdminInitializer::initialize(&env, &admin).unwrap();

        // Test admin check
        assert!(AdminUtils::is_admin(&env, &admin));
        assert!(!AdminUtils::is_admin(&env, &non_admin));
    }

    #[test]
    fn test_admin_testing_utilities() {
        let env = Env::default();
        let admin = Address::generate(&env);

        let action = AdminTesting::create_test_admin_action(&env, &admin);
        assert!(AdminTesting::validate_admin_action_structure(&action).is_ok());

        let role_assignment = AdminTesting::create_test_role_assignment(&env, &admin);
        assert_eq!(role_assignment.role, AdminRole::MarketAdmin);
        assert!(role_assignment.is_active);
    }
} 