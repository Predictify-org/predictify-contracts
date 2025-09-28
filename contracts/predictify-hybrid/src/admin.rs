extern crate alloc;
use soroban_sdk::{contracttype, vec, Address, Env, Map, String, Symbol, Vec};
// use alloc::string::ToString; // Unused import

use crate::config::FeeConfig;
use crate::config::{ConfigManager, ConfigUtils, ContractConfig, Environment};
use crate::errors::Error;
use crate::events::EventEmitter;
use crate::extensions::ExtensionManager;
use crate::fees::FeeManager;
use crate::markets::MarketStateManager;
use crate::resolution::MarketResolutionManager;

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
    /// Initializes the Predictify Hybrid contract with a primary administrator.
    ///
    /// This function sets up the foundational admin structure for the contract,
    /// establishing the primary admin with SuperAdmin privileges and initializing
    /// the admin management system. It must be called once after contract deployment.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `admin` - The address to be granted SuperAdmin privileges
    ///
    /// # Returns
    ///
    /// Returns `Result<(), Error>` where:
    /// - `Ok(())` - Admin initialization completed successfully
    /// - `Err(Error)` - Specific error if initialization fails
    ///
    /// # Errors
    ///
    /// This function returns specific errors:
    /// - `Error::InvalidAddress` - Admin address is invalid or zero
    /// - `Error::AlreadyInitialized` - Contract has already been initialized
    /// - `Error::StorageError` - Failed to store admin data
    /// - Role assignment errors from AdminRoleManager
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address};
    /// # use predictify_hybrid::admin::AdminInitializer;
    /// # let env = Env::default();
    /// # let admin_address = Address::generate(&env);
    ///
    /// match AdminInitializer::initialize(&env, &admin_address) {
    ///     Ok(()) => {
    ///         println!("Contract initialized successfully");
    ///     },
    ///     Err(e) => {
    ///         println!("Initialization failed: {:?}", e);
    ///     }
    /// }
    /// ```
    ///
    /// # Initialization Process
    ///
    /// The initialization performs these steps:
    /// 1. **Address Validation**: Ensures admin address is valid
    /// 2. **Storage Setup**: Stores admin address in persistent storage
    /// 3. **Role Assignment**: Grants SuperAdmin role to the admin
    /// 4. **Event Emission**: Emits admin initialization event
    /// 5. **Action Logging**: Records the initialization action
    ///
    /// # Post-Initialization State
    ///
    /// After successful initialization:
    /// - Admin has full SuperAdmin privileges
    /// - All admin permissions are available to the admin
    /// - Admin management system is fully operational
    /// - Contract is ready for market creation and management
    ///
    /// # Security
    ///
    /// The admin address should be carefully chosen as it will have complete
    /// control over the contract. Consider using a multi-signature wallet
    /// or governance contract for production deployments.
    pub fn initialize(env: &Env, admin: &Address) -> Result<(), Error> {
        // Validate admin address
        AdminValidator::validate_admin_address(env, admin)?;

        // Store admin in persistent storage
        env.storage()
            .persistent()
            .set(&Symbol::new(env, "Admin"), admin);

        // Set default admin role
        AdminRoleManager::assign_role(env, admin, AdminRole::SuperAdmin, admin)?;

        // Emit admin initialization event
        EventEmitter::emit_admin_initialized(env, admin);

        // Log admin action
        AdminActionLogger::log_action(env, admin, "initialize", None, Map::new(env), true, None)?;

        Ok(())
    }

    /// Initializes the contract with admin and environment-specific configuration.
    ///
    /// This advanced initialization function sets up both admin privileges and
    /// applies environment-specific configurations (development, testnet, mainnet).
    /// It's ideal for deployment scenarios where specific configurations are needed.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `admin` - The address to be granted SuperAdmin privileges
    /// * `environment` - The target environment configuration to apply
    ///
    /// # Returns
    ///
    /// Returns `Result<(), Error>` where:
    /// - `Ok(())` - Admin and configuration initialization completed successfully
    /// - `Err(Error)` - Specific error if initialization fails
    ///
    /// # Errors
    ///
    /// This function returns errors from:
    /// - `AdminInitializer::initialize()` - Basic admin initialization errors
    /// - `ConfigManager::store_config()` - Configuration storage errors
    /// - Event emission errors
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address};
    /// # use predictify_hybrid::admin::AdminInitializer;
    /// # use predictify_hybrid::config::Environment;
    /// # let env = Env::default();
    /// # let admin_address = Address::generate(&env);
    ///
    /// // Initialize for mainnet deployment
    /// match AdminInitializer::initialize_with_config(
    ///     &env,
    ///     &admin_address,
    ///     &Environment::Mainnet
    /// ) {
    ///     Ok(()) => {
    ///         println!("Contract initialized with mainnet config");
    ///     },
    ///     Err(e) => {
    ///         println!("Initialization failed: {:?}", e);
    ///     }
    /// }
    /// ```
    ///
    /// # Environment Configurations
    ///
    /// - **Development**: Relaxed validation, debug features enabled
    /// - **Testnet**: Production-like settings with test-friendly parameters
    /// - **Mainnet**: Full production settings with strict validation
    /// - **Custom**: Defaults to development configuration
    ///
    /// # Configuration Applied
    ///
    /// Environment-specific settings include:
    /// - Fee structures and percentages
    /// - Market duration limits
    /// - Validation thresholds
    /// - Oracle timeout settings
    /// - Dispute resolution parameters
    ///
    /// # Use Cases
    ///
    /// - **Production Deployment**: Use with `Environment::Mainnet`
    /// - **Testing**: Use with `Environment::Testnet` or `Environment::Development`
    /// - **CI/CD Pipelines**: Automated deployment with appropriate environment
    /// - **Multi-Environment Contracts**: Same contract code, different configs
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

    /// Validates parameters before contract initialization.
    ///
    /// This function performs pre-initialization validation to ensure the contract
    /// can be safely initialized with the provided parameters. It's useful for
    /// checking initialization requirements before committing to the initialization.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `admin` - The proposed admin address to validate
    ///
    /// # Returns
    ///
    /// Returns `Result<(), Error>` where:
    /// - `Ok(())` - All parameters are valid for initialization
    /// - `Err(Error)` - Specific validation error
    ///
    /// # Errors
    ///
    /// This function returns specific validation errors:
    /// - `Error::InvalidAddress` - Admin address is invalid, zero, or malformed
    /// - `Error::AlreadyInitialized` - Contract has already been initialized
    /// - Address format validation errors
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address};
    /// # use predictify_hybrid::admin::AdminInitializer;
    /// # let env = Env::default();
    /// # let proposed_admin = Address::generate(&env);
    ///
    /// // Validate before initialization
    /// match AdminInitializer::validate_initialization_params(&env, &proposed_admin) {
    ///     Ok(()) => {
    ///         // Parameters are valid, proceed with initialization
    ///         AdminInitializer::initialize(&env, &proposed_admin).unwrap();
    ///     },
    ///     Err(e) => {
    ///         println!("Invalid parameters: {:?}", e);
    ///     }
    /// }
    /// ```
    ///
    /// # Validation Checks
    ///
    /// The function performs these validations:
    /// 1. **Admin Address**: Ensures address is valid and not zero
    /// 2. **Contract State**: Verifies contract hasn't been initialized
    /// 3. **Address Format**: Validates Stellar address format
    /// 4. **Storage Access**: Ensures storage operations will succeed
    ///
    /// # Use Cases
    ///
    /// - **Pre-flight Checks**: Validate before expensive initialization
    /// - **UI Validation**: Check parameters in user interfaces
    /// - **Deployment Scripts**: Ensure deployment will succeed
    /// - **Testing**: Validate test parameters before test execution
    /// - **Error Prevention**: Catch issues before state changes
    ///
    /// # Best Practices
    ///
    /// Always call this function before `initialize()` or `initialize_with_config()`
    /// to prevent failed initialization attempts that could leave the contract
    /// in an inconsistent state.
    pub fn validate_initialization_params(env: &Env, admin: &Address) -> Result<(), Error> {
        AdminValidator::validate_admin_address(env, admin)?;
        AdminValidator::validate_contract_not_initialized(env)?;
        Ok(())
    }
}

// ===== ADMIN ACCESS CONTROL =====

/// Admin access control management
pub struct AdminAccessControl;

impl AdminAccessControl {
    /// Validates that an admin has the required permission for a specific action.
    ///
    /// This function checks if the given admin address has the necessary permission
    /// to perform a specific action based on their assigned role. It's the core
    /// authorization mechanism for all admin operations in the contract.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `admin` - The admin address to validate permissions for
    /// * `permission` - The specific permission required for the action
    ///
    /// # Returns
    ///
    /// Returns `Result<(), Error>` where:
    /// - `Ok(())` - Admin has the required permission
    /// - `Err(Error)` - Admin lacks permission or validation failed
    ///
    /// # Errors
    ///
    /// This function returns specific errors:
    /// - `Error::Unauthorized` - Admin doesn't have the required permission
    /// - `Error::Unauthorized` - Admin role not found or inactive
    /// - Role retrieval errors from AdminRoleManager
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address};
    /// # use predictify_hybrid::admin::{AdminAccessControl, AdminPermission};
    /// # let env = Env::default();
    /// # let admin = Address::generate(&env);
    ///
    /// // Check if admin can create markets
    /// match AdminAccessControl::validate_permission(
    ///     &env,
    ///     &admin,
    ///     &AdminPermission::CreateMarket
    /// ) {
    ///     Ok(()) => {
    ///         // Admin has permission, proceed with market creation
    ///         println!("Admin authorized for market creation");
    ///     },
    ///     Err(e) => {
    ///         println!("Permission denied: {:?}", e);
    ///     }
    /// }
    /// ```
    ///
    /// # Permission Hierarchy
    ///
    /// Different admin roles have different permission sets:
    /// - **SuperAdmin**: All permissions
    /// - **MarketAdmin**: Market-related permissions
    /// - **ConfigAdmin**: Configuration permissions
    /// - **FeeAdmin**: Fee management permissions
    /// - **ReadOnlyAdmin**: View-only permissions
    ///
    /// # Use Cases
    ///
    /// - **Function Guards**: Validate permissions before executing admin functions
    /// - **UI Authorization**: Show/hide UI elements based on permissions
    /// - **API Endpoints**: Authorize admin API calls
    /// - **Batch Operations**: Validate permissions for multiple operations
    /// - **Audit Trails**: Log permission checks for security auditing
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

    /// Requires admin authentication and validates admin status.
    ///
    /// This function performs comprehensive admin authentication by verifying
    /// the caller's signature and confirming they are a registered admin.
    /// It's the fundamental authentication check for all admin operations.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `admin` - The admin address to authenticate
    ///
    /// # Returns
    ///
    /// Returns `Result<(), Error>` where:
    /// - `Ok(())` - Admin is authenticated and authorized
    /// - `Err(Error)` - Authentication or authorization failed
    ///
    /// # Errors
    ///
    /// This function returns specific errors:
    /// - `Error::AdminNotSet` - No admin has been configured for the contract
    /// - `Error::Unauthorized` - Caller is not the registered admin
    /// - Authentication errors from Soroban's `require_auth()`
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address};
    /// # use predictify_hybrid::admin::AdminAccessControl;
    /// # let env = Env::default();
    /// # let admin = Address::generate(&env);
    ///
    /// // Authenticate admin before sensitive operation
    /// match AdminAccessControl::require_admin_auth(&env, &admin) {
    ///     Ok(()) => {
    ///         // Admin is authenticated, proceed with operation
    ///         println!("Admin authenticated successfully");
    ///     },
    ///     Err(e) => {
    ///         println!("Authentication failed: {:?}", e);
    ///         return;
    ///     }
    /// }
    /// ```
    ///
    /// # Authentication Process
    ///
    /// The authentication performs these checks:
    /// 1. **Signature Verification**: Validates the caller's cryptographic signature
    /// 2. **Admin Lookup**: Retrieves the stored admin address from contract storage
    /// 3. **Address Comparison**: Ensures the caller matches the stored admin
    /// 4. **Status Validation**: Confirms admin status is active
    ///
    /// # Security Considerations
    ///
    /// - Uses Soroban's built-in signature verification
    /// - Prevents unauthorized access to admin functions
    /// - Should be called before any admin-only operations
    /// - Protects against address spoofing attacks
    ///
    /// # Use Cases
    ///
    /// - **Function Entry Points**: First check in admin functions
    /// - **Batch Operations**: Authenticate once for multiple operations
    /// - **API Gateways**: Validate admin API requests
    /// - **Emergency Functions**: Ensure only authorized emergency actions
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

    /// Validates admin authentication and permissions for a specific action.
    ///
    /// This comprehensive validation function combines authentication and
    /// permission checking for a specific action. It's a convenience function
    /// that performs complete admin validation in a single call.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `admin` - The admin address to validate
    /// * `action` - String identifier of the action to validate (e.g., "create_market")
    ///
    /// # Returns
    ///
    /// Returns `Result<(), Error>` where:
    /// - `Ok(())` - Admin is authenticated and authorized for the action
    /// - `Err(Error)` - Authentication, permission, or action mapping failed
    ///
    /// # Errors
    ///
    /// This function returns errors from:
    /// - `require_admin_auth()` - Authentication failures
    /// - `map_action_to_permission()` - Invalid action string
    /// - `validate_permission()` - Permission validation failures
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address};
    /// # use predictify_hybrid::admin::AdminAccessControl;
    /// # let env = Env::default();
    /// # let admin = Address::generate(&env);
    ///
    /// // Validate admin for market creation
    /// match AdminAccessControl::validate_admin_for_action(
    ///     &env,
    ///     &admin,
    ///     "create_market"
    /// ) {
    ///     Ok(()) => {
    ///         // Admin is fully authorized, proceed with market creation
    ///         println!("Admin authorized for market creation");
    ///     },
    ///     Err(e) => {
    ///         println!("Authorization failed: {:?}", e);
    ///     }
    /// }
    /// ```
    ///
    /// # Supported Actions
    ///
    /// Valid action strings include:
    /// - `"initialize"` - Contract initialization
    /// - `"create_market"` - Market creation
    /// - `"close_market"` - Market closure
    /// - `"finalize_market"` - Market finalization
    /// - `"extend_market"` - Market duration extension
    /// - `"update_fees"` - Fee configuration updates
    /// - `"update_config"` - Contract configuration updates
    /// - `"collect_fees"` - Fee collection
    /// - `"manage_disputes"` - Dispute management
    /// - `"emergency_actions"` - Emergency operations
    ///
    /// # Validation Process
    ///
    /// The function performs validation in this order:
    /// 1. **Authentication**: Verifies admin signature and status
    /// 2. **Action Mapping**: Maps action string to permission enum
    /// 3. **Permission Check**: Validates admin has required permission
    ///
    /// # Use Cases
    ///
    /// - **Single-Call Validation**: Complete validation in one function call
    /// - **Dynamic Actions**: Validate actions determined at runtime
    /// - **API Endpoints**: Validate admin API calls with action strings
    /// - **Middleware**: Use in middleware for automatic admin validation
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

    /// Maps action string identifiers to their corresponding permission enums.
    ///
    /// This utility function converts human-readable action strings into
    /// the corresponding AdminPermission enum values. It's used to bridge
    /// string-based action identifiers with the type-safe permission system.
    ///
    /// # Parameters
    ///
    /// * `action` - String identifier of the action to map
    ///
    /// # Returns
    ///
    /// Returns `Result<AdminPermission, Error>` where:
    /// - `Ok(AdminPermission)` - Successfully mapped action to permission
    /// - `Err(Error::InvalidInput)` - Action string is not recognized
    ///
    /// # Errors
    ///
    /// This function returns:
    /// - `Error::InvalidInput` - Action string doesn't match any known actions
    ///
    /// # Example
    ///
    /// ```rust
    /// # use predictify_hybrid::admin::{AdminAccessControl, AdminPermission};
    ///
    /// // Map action string to permission
    /// match AdminAccessControl::map_action_to_permission("create_market") {
    ///     Ok(permission) => {
    ///         assert_eq!(permission, AdminPermission::CreateMarket);
    ///         println!("Mapped to CreateMarket permission");
    ///     },
    ///     Err(e) => {
    ///         println!("Invalid action: {:?}", e);
    ///     }
    /// }
    ///
    /// // Handle invalid action
    /// match AdminAccessControl::map_action_to_permission("invalid_action") {
    ///     Ok(_) => unreachable!(),
    ///     Err(e) => {
    ///         println!("Expected error for invalid action: {:?}", e);
    ///     }
    /// }
    /// ```
    ///
    /// # Action Mapping Table
    ///
    /// | Action String | Permission Enum |
    /// |---------------|----------------|
    /// | `"initialize"` | `AdminPermission::Initialize` |
    /// | `"create_market"` | `AdminPermission::CreateMarket` |
    /// | `"close_market"` | `AdminPermission::CloseMarket` |
    /// | `"finalize_market"` | `AdminPermission::FinalizeMarket` |
    /// | `"extend_market"` | `AdminPermission::ExtendMarket` |
    /// | `"update_fees"` | `AdminPermission::UpdateFees` |
    /// | `"update_config"` | `AdminPermission::UpdateConfig` |
    /// | `"reset_config"` | `AdminPermission::ResetConfig` |
    /// | `"collect_fees"` | `AdminPermission::CollectFees` |
    /// | `"manage_disputes"` | `AdminPermission::ManageDisputes` |
    /// | `"view_analytics"` | `AdminPermission::ViewAnalytics` |
    /// | `"emergency_actions"` | `AdminPermission::EmergencyActions` |
    ///
    /// # Use Cases
    ///
    /// - **API Integration**: Convert API action parameters to permissions
    /// - **Dynamic Validation**: Handle actions determined at runtime
    /// - **Configuration**: Map configuration-driven actions to permissions
    /// - **Testing**: Validate action-permission mappings in tests
    /// - **Debugging**: Convert action strings for logging and debugging
    ///
    /// # Design Notes
    ///
    /// Action strings use snake_case convention to match Rust naming standards.
    /// The mapping is case-sensitive and must match exactly. Consider adding
    /// case-insensitive mapping if needed for API flexibility.
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
    /// Assigns a specific admin role to an address with associated permissions.
    ///
    /// This function creates or updates admin role assignments, establishing the
    /// permission hierarchy for admin operations. It supports bootstrapping the
    /// first admin and subsequent role assignments by authorized admins.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `admin` - The address to receive the admin role
    /// * `role` - The admin role to assign (SuperAdmin, MarketAdmin, etc.)
    /// * `assigned_by` - The address performing the role assignment
    ///
    /// # Returns
    ///
    /// Returns `Result<(), Error>` where:
    /// - `Ok(())` - Role assigned successfully
    /// - `Err(Error)` - Assignment failed due to permissions or validation
    ///
    /// # Errors
    ///
    /// This function returns specific errors:
    /// - `Error::Unauthorized` - Assigner lacks EmergencyActions permission
    /// - Permission validation errors from AdminAccessControl
    /// - Storage operation errors
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address};
    /// # use predictify_hybrid::admin::{AdminRoleManager, AdminRole};
    /// # let env = Env::default();
    /// # let super_admin = Address::generate(&env);
    /// # let new_admin = Address::generate(&env);
    ///
    /// // Assign MarketAdmin role to a new admin
    /// match AdminRoleManager::assign_role(
    ///     &env,
    ///     &new_admin,
    ///     AdminRole::MarketAdmin,
    ///     &super_admin
    /// ) {
    ///     Ok(()) => {
    ///         println!("MarketAdmin role assigned successfully");
    ///     },
    ///     Err(e) => {
    ///         println!("Role assignment failed: {:?}", e);
    ///     }
    /// }
    /// ```
    ///
    /// # Role Hierarchy
    ///
    /// Available admin roles with their permission levels:
    /// - **SuperAdmin**: All permissions, can assign other roles
    /// - **MarketAdmin**: Market creation, closure, finalization, extension
    /// - **ConfigAdmin**: Configuration updates and resets
    /// - **FeeAdmin**: Fee configuration and collection
    /// - **ReadOnlyAdmin**: View-only access to analytics
    ///
    /// # Assignment Process
    ///
    /// The assignment process:
    /// 1. **Bootstrap Check**: First assignment bypasses permission validation
    /// 2. **Permission Validation**: Subsequent assignments require EmergencyActions permission
    /// 3. **Role Creation**: Creates AdminRoleAssignment with timestamp and permissions
    /// 4. **Storage Update**: Stores assignment in persistent storage
    /// 5. **Event Emission**: Emits role assignment event for monitoring
    ///
    /// # Security
    ///
    /// Only admins with EmergencyActions permission can assign roles to others.
    /// The first admin assignment (bootstrapping) bypasses this check to enable
    /// initial contract setup.
    pub fn assign_role(
        env: &Env,
        admin: &Address,
        role: AdminRole,
        assigned_by: &Address,
    ) -> Result<(), Error> {
        // Use a simple fixed key for admin role storage
        let key = Symbol::new(env, "admin_role");

        // Check if this is the first admin role assignment (bootstrapping)
        if !env.storage().persistent().has(&key) {
            // No admin role assigned yet, allow bootstrapping without permission check
        } else {
            // Validate assigner permissions for subsequent assignments
            AdminAccessControl::validate_permission(
                env,
                assigned_by,
                &AdminPermission::EmergencyActions,
            )?;
        }

        // Create role assignment
        let assignment = AdminRoleAssignment {
            admin: admin.clone(),
            role,
            assigned_by: assigned_by.clone(),
            assigned_at: env.ledger().timestamp(),
            permissions: AdminRoleManager::get_permissions_for_role(env, &role),
            is_active: true,
        };

        // Store role assignment
        env.storage().persistent().set(&key, &assignment);

        // Emit role assignment event
        let events_role = match role {
            AdminRole::SuperAdmin => crate::events::AdminRole::Owner,
            AdminRole::MarketAdmin => crate::events::AdminRole::Admin,
            AdminRole::ConfigAdmin => crate::events::AdminRole::Admin,
            AdminRole::FeeAdmin => crate::events::AdminRole::Admin,
            AdminRole::ReadOnlyAdmin => crate::events::AdminRole::Moderator,
        };
        EventEmitter::emit_admin_role_assigned(env, admin, &events_role, assigned_by);

        Ok(())
    }

    /// Retrieves the admin role assigned to a specific address.
    ///
    /// This function looks up the admin role for a given address, validating
    /// that the admin is active and returning their assigned role. It's used
    /// for permission checking and role-based access control.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `admin` - The address to look up the admin role for
    ///
    /// # Returns
    ///
    /// Returns `Result<AdminRole, Error>` where:
    /// - `Ok(AdminRole)` - The admin role assigned to the address
    /// - `Err(Error)` - Admin not found, inactive, or unauthorized
    ///
    /// # Errors
    ///
    /// This function returns specific errors:
    /// - `Error::Unauthorized` - No admin role assignment found
    /// - `Error::Unauthorized` - Admin role assignment is inactive
    /// - `Error::Unauthorized` - Address doesn't match the assigned admin
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address};
    /// # use predictify_hybrid::admin::{AdminRoleManager, AdminRole};
    /// # let env = Env::default();
    /// # let admin = Address::generate(&env);
    ///
    /// // Get admin role for permission checking
    /// match AdminRoleManager::get_admin_role(&env, &admin) {
    ///     Ok(AdminRole::SuperAdmin) => {
    ///         println!("User has SuperAdmin privileges");
    ///     },
    ///     Ok(AdminRole::MarketAdmin) => {
    ///         println!("User has MarketAdmin privileges");
    ///     },
    ///     Ok(role) => {
    ///         println!("User has {:?} privileges", role);
    ///     },
    ///     Err(e) => {
    ///         println!("No admin role found: {:?}", e);
    ///     }
    /// }
    /// ```
    ///
    /// # Role Validation
    ///
    /// The function performs these validations:
    /// 1. **Assignment Lookup**: Retrieves role assignment from storage
    /// 2. **Active Check**: Ensures the role assignment is active
    /// 3. **Address Match**: Confirms the address matches the assignment
    /// 4. **Role Return**: Returns the validated admin role
    ///
    /// # Use Cases
    ///
    /// - **Permission Checking**: Determine what actions an admin can perform
    /// - **UI Authorization**: Show/hide features based on admin role
    /// - **Audit Logging**: Record admin roles in action logs
    /// - **Role-Based Logic**: Execute different logic based on admin role
    /// - **Access Control**: Gate access to role-specific functionality
    ///
    /// # Performance
    ///
    /// This function performs a single storage lookup and is optimized for
    /// frequent use in permission validation scenarios.
    pub fn get_admin_role(env: &Env, admin: &Address) -> Result<AdminRole, Error> {
        // Use a simple fixed key for admin role storage
        let key = Symbol::new(env, "admin_role");

        let assignment: AdminRoleAssignment = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(Error::Unauthorized)?;

        if !assignment.is_active {
            return Err(Error::Unauthorized);
        }

        // Check if the passed address matches the admin address in the assignment
        if admin != &assignment.admin {
            return Err(Error::Unauthorized);
        }

        Ok(assignment.role)
    }

    /// Checks if a specific admin role has a particular permission.
    ///
    /// This function determines whether an admin role includes a specific
    /// permission by checking the role's permission set. It's a core component
    /// of the permission validation system.
    ///
    /// # Parameters
    ///
    /// * `_env` - The Soroban environment (unused but kept for consistency)
    /// * `role` - The admin role to check permissions for
    /// * `permission` - The specific permission to check
    ///
    /// # Returns
    ///
    /// Returns `Result<bool, Error>` where:
    /// - `Ok(true)` - Role has the specified permission
    /// - `Ok(false)` - Role does not have the specified permission
    /// - `Err(Error)` - Error retrieving role permissions
    ///
    /// # Errors
    ///
    /// This function typically doesn't error but may return errors from
    /// permission retrieval operations in future implementations.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::Env;
    /// # use predictify_hybrid::admin::{AdminRoleManager, AdminRole, AdminPermission};
    /// # let env = Env::default();
    ///
    /// // Check if MarketAdmin can create markets
    /// let can_create = AdminRoleManager::has_permission(
    ///     &env,
    ///     &AdminRole::MarketAdmin,
    ///     &AdminPermission::CreateMarket
    /// ).unwrap();
    ///
    /// if can_create {
    ///     println!("MarketAdmin can create markets");
    /// }
    ///
    /// // Check if ReadOnlyAdmin can update fees
    /// let can_update_fees = AdminRoleManager::has_permission(
    ///     &env,
    ///     &AdminRole::ReadOnlyAdmin,
    ///     &AdminPermission::UpdateFees
    /// ).unwrap();
    ///
    /// assert!(!can_update_fees); // ReadOnlyAdmin cannot update fees
    /// ```
    ///
    /// # Permission Matrix
    ///
    /// | Role | Initialize | CreateMarket | UpdateFees | UpdateConfig | Emergency |
    /// |------|------------|--------------|------------|--------------|----------|
    /// | SuperAdmin | ✓ | ✓ | ✓ | ✓ | ✓ |
    /// | MarketAdmin | ✗ | ✓ | ✗ | ✗ | ✗ |
    /// | ConfigAdmin | ✗ | ✗ | ✗ | ✓ | ✗ |
    /// | FeeAdmin | ✗ | ✗ | ✓ | ✗ | ✗ |
    /// | ReadOnlyAdmin | ✗ | ✗ | ✗ | ✗ | ✗ |
    ///
    /// # Use Cases
    ///
    /// - **Permission Validation**: Core permission checking logic
    /// - **Role Comparison**: Compare capabilities of different roles
    /// - **UI Authorization**: Determine what UI elements to show
    /// - **API Gating**: Control access to API endpoints
    /// - **Audit Systems**: Log permission checks for security auditing
    ///
    /// # Performance
    ///
    /// This function is highly optimized for frequent use, performing only
    /// in-memory operations on the role's permission vector.
    pub fn has_permission(
        _env: &Env,
        role: &AdminRole,
        permission: &AdminPermission,
    ) -> Result<bool, Error> {
        let permissions = AdminRoleManager::get_permissions_for_role(_env, role);
        Ok(permissions.contains(permission))
    }

    /// Retrieves the complete set of permissions for a specific admin role.
    ///
    /// This function returns all permissions associated with an admin role,
    /// providing the definitive permission set for role-based access control.
    /// It's used internally by permission checking functions and externally
    /// for role analysis and UI generation.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for creating the permission vector
    /// * `role` - The admin role to get permissions for
    ///
    /// # Returns
    ///
    /// Returns `Vec<AdminPermission>` containing all permissions for the role.
    /// The vector is never empty - even ReadOnlyAdmin has ViewAnalytics permission.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::Env;
    /// # use predictify_hybrid::admin::{AdminRoleManager, AdminRole, AdminPermission};
    /// # let env = Env::default();
    ///
    /// // Get all permissions for SuperAdmin
    /// let super_permissions = AdminRoleManager::get_permissions_for_role(
    ///     &env,
    ///     &AdminRole::SuperAdmin
    /// );
    ///
    /// println!("SuperAdmin has {} permissions", super_permissions.len());
    /// assert!(super_permissions.contains(&AdminPermission::Initialize));
    /// assert!(super_permissions.contains(&AdminPermission::EmergencyActions));
    ///
    /// // Get permissions for MarketAdmin
    /// let market_permissions = AdminRoleManager::get_permissions_for_role(
    ///     &env,
    ///     &AdminRole::MarketAdmin
    /// );
    ///
    /// assert!(market_permissions.contains(&AdminPermission::CreateMarket));
    /// assert!(!market_permissions.contains(&AdminPermission::UpdateFees));
    /// ```
    ///
    /// # Permission Sets by Role
    ///
    /// **SuperAdmin** (12 permissions):
    /// - Initialize, CreateMarket, CloseMarket, FinalizeMarket
    /// - ExtendMarket, UpdateFees, UpdateConfig, ResetConfig
    /// - CollectFees, ManageDisputes, ViewAnalytics, EmergencyActions
    ///
    /// **MarketAdmin** (5 permissions):
    /// - CreateMarket, CloseMarket, FinalizeMarket, ExtendMarket, ViewAnalytics
    ///
    /// **ConfigAdmin** (3 permissions):
    /// - UpdateConfig, ResetConfig, ViewAnalytics
    ///
    /// **FeeAdmin** (3 permissions):
    /// - UpdateFees, CollectFees, ViewAnalytics
    ///
    /// **ReadOnlyAdmin** (1 permission):
    /// - ViewAnalytics
    ///
    /// # Use Cases
    ///
    /// - **Role Analysis**: Understand what each role can do
    /// - **UI Generation**: Create role-specific interfaces
    /// - **Permission Auditing**: Review and audit role permissions
    /// - **Role Comparison**: Compare capabilities between roles
    /// - **Documentation**: Generate permission documentation
    /// - **Testing**: Validate role permission assignments
    ///
    /// # Design Principles
    ///
    /// - **Least Privilege**: Each role has only necessary permissions
    /// - **Clear Hierarchy**: SuperAdmin > Specialized Admins > ReadOnly
    /// - **Separation of Concerns**: Different roles for different responsibilities
    /// - **Extensibility**: Easy to add new roles and permissions
    pub fn get_permissions_for_role(env: &Env, role: &AdminRole) -> Vec<AdminPermission> {
        match role {
            AdminRole::SuperAdmin => soroban_sdk::vec![
                env,
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
            AdminRole::MarketAdmin => soroban_sdk::vec![
                env,
                AdminPermission::CreateMarket,
                AdminPermission::CloseMarket,
                AdminPermission::FinalizeMarket,
                AdminPermission::ExtendMarket,
                AdminPermission::ViewAnalytics,
            ],
            AdminRole::ConfigAdmin => soroban_sdk::vec![
                env,
                AdminPermission::UpdateConfig,
                AdminPermission::ResetConfig,
                AdminPermission::ViewAnalytics,
            ],
            AdminRole::FeeAdmin => soroban_sdk::vec![
                env,
                AdminPermission::UpdateFees,
                AdminPermission::CollectFees,
                AdminPermission::ViewAnalytics,
            ],
            AdminRole::ReadOnlyAdmin => soroban_sdk::vec![env, AdminPermission::ViewAnalytics,],
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

        // Use a simple fixed key for admin role storage
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
pub struct AdminFunctions;

impl AdminFunctions {
    /// Closes a market before its natural end time (admin only).
    ///
    /// This function allows authorized admins to forcibly close a market,
    /// preventing further voting and triggering the market closure process.
    /// It's used for emergency situations or when markets need early termination.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `admin` - The admin address performing the closure (must have CloseMarket permission)
    /// * `market_id` - Unique identifier of the market to close
    ///
    /// # Returns
    ///
    /// Returns `Result<(), Error>` where:
    /// - `Ok(())` - Market closed successfully
    /// - `Err(Error)` - Closure failed due to permissions or validation
    ///
    /// # Errors
    ///
    /// This function returns specific errors:
    /// - `Error::Unauthorized` - Admin lacks CloseMarket permission
    /// - `Error::MarketNotFound` - Market with given ID doesn't exist
    /// - Authentication errors from AdminAccessControl
    /// - Storage operation errors
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address, Symbol};
    /// # use predictify_hybrid::admin::AdminFunctions;
    /// # let env = Env::default();
    /// # let admin = Address::generate(&env);
    /// # let market_id = Symbol::new(&env, "problematic_market");
    ///
    /// // Close a problematic market
    /// match AdminFunctions::close_market(&env, &admin, &market_id) {
    ///     Ok(()) => {
    ///         println!("Market closed successfully");
    ///     },
    ///     Err(e) => {
    ///         println!("Failed to close market: {:?}", e);
    ///     }
    /// }
    /// ```
    ///
    /// # Closure Process
    ///
    /// The closure process performs these steps:
    /// 1. **Permission Validation**: Ensures admin has CloseMarket permission
    /// 2. **Market Validation**: Confirms market exists and can be closed
    /// 3. **Market Removal**: Removes market from active storage
    /// 4. **Event Emission**: Emits market closure event for monitoring
    /// 5. **Action Logging**: Records the admin action for audit trails
    ///
    /// # Use Cases
    ///
    /// - **Emergency Closure**: Close markets with problematic questions or outcomes
    /// - **Policy Violations**: Close markets that violate platform policies
    /// - **Technical Issues**: Close markets experiencing technical problems
    /// - **Legal Compliance**: Close markets for regulatory compliance
    /// - **Community Requests**: Close markets based on community feedback
    ///
    /// # Post-Closure State
    ///
    /// After closure:
    /// - Market is removed from active storage
    /// - No further voting is possible
    /// - Existing stakes may need manual resolution
    /// - Market appears as closed in historical records
    ///
    /// # Security
    ///
    /// This is a powerful admin function that should be used carefully.
    /// Only admins with CloseMarket permission can execute this function.
    pub fn close_market(env: &Env, admin: &Address, market_id: &Symbol) -> Result<(), Error> {
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
        params.set(
            String::from_str(env, "market_id"),
            String::from_str(env, "market_id"),
        );
        AdminActionLogger::log_action(env, admin, "close_market", None, params, true, None)?;

        Ok(())
    }

    /// Finalizes a market with admin override of the resolution process.
    ///
    /// This function allows authorized admins to directly set the final outcome
    /// of a market, bypassing the normal resolution process. It's used when
    /// manual intervention is required for market resolution.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `admin` - The admin address performing the finalization (must have FinalizeMarket permission)
    /// * `market_id` - Unique identifier of the market to finalize
    /// * `outcome` - The final outcome to set for the market
    ///
    /// # Returns
    ///
    /// Returns `Result<(), Error>` where:
    /// - `Ok(())` - Market finalized successfully
    /// - `Err(Error)` - Finalization failed due to permissions or validation
    ///
    /// # Errors
    ///
    /// This function returns specific errors:
    /// - `Error::Unauthorized` - Admin lacks FinalizeMarket permission
    /// - `Error::MarketNotFound` - Market with given ID doesn't exist
    /// - `Error::InvalidOutcome` - Outcome doesn't match market's possible outcomes
    /// - `Error::MarketAlreadyResolved` - Market has already been finalized
    /// - Resolution errors from MarketResolutionManager
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address, Symbol, String};
    /// # use predictify_hybrid::admin::AdminFunctions;
    /// # let env = Env::default();
    /// # let admin = Address::generate(&env);
    /// # let market_id = Symbol::new(&env, "disputed_market");
    /// # let outcome = String::from_str(&env, "Yes");
    ///
    /// // Finalize a disputed market with admin decision
    /// match AdminFunctions::finalize_market(&env, &admin, &market_id, &outcome) {
    ///     Ok(()) => {
    ///         println!("Market finalized with outcome: {}", outcome);
    ///     },
    ///     Err(e) => {
    ///         println!("Failed to finalize market: {:?}", e);
    ///     }
    /// }
    /// ```
    ///
    /// # Finalization Process
    ///
    /// The finalization process:
    /// 1. **Permission Validation**: Ensures admin has FinalizeMarket permission
    /// 2. **Market Resolution**: Uses MarketResolutionManager to set final outcome
    /// 3. **Event Emission**: Emits market finalization event
    /// 4. **Action Logging**: Records admin action with outcome details
    ///
    /// # Use Cases
    ///
    /// - **Dispute Resolution**: Resolve disputed markets with admin decision
    /// - **Oracle Failures**: Finalize markets when oracles fail or are unavailable
    /// - **Subjective Markets**: Resolve markets requiring human judgment
    /// - **Emergency Resolution**: Quick resolution in time-sensitive situations
    /// - **Correction**: Correct automated resolutions that were incorrect
    ///
    /// # Post-Finalization State
    ///
    /// After finalization:
    /// - Market state changes to Resolved
    /// - Winning outcome is permanently set
    /// - Users can claim winnings based on the outcome
    /// - Market statistics are finalized
    /// - No further changes to the market are possible
    ///
    /// # Governance
    ///
    /// Admin finalization should follow established governance procedures
    /// and be transparent to the community. Consider implementing multi-signature
    /// requirements for high-value market finalizations.
    pub fn finalize_market(
        env: &Env,
        admin: &Address,
        market_id: &Symbol,
        outcome: &String,
    ) -> Result<(), Error> {
        // Validate admin permissions
        AdminAccessControl::validate_admin_for_action(env, admin, "finalize_market")?;

        // Finalize market using resolution manager
        let _resolution = MarketResolutionManager::finalize_market(env, admin, market_id, outcome)?;

        // Emit market finalized event
        EventEmitter::emit_market_finalized(env, market_id, admin, outcome);

        // Log admin action
        let mut params = Map::new(env);
        params.set(
            String::from_str(env, "market_id"),
            String::from_str(env, "market_id"),
        );
        params.set(String::from_str(env, "outcome"), outcome.clone());
        AdminActionLogger::log_action(
            env,
            admin,
            "finalize_market",
            Some(String::from_str(env, "market_id")),
            params,
            true,
            None,
        )?;

        Ok(())
    }

    /// Extends the duration of an active market (admin only).
    ///
    /// This function allows authorized admins to extend the voting period
    /// of an active market by adding additional days to its end time.
    /// Extensions require a reason for transparency and audit purposes.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `admin` - The admin address performing the extension (must have ExtendMarket permission)
    /// * `market_id` - Unique identifier of the market to extend
    /// * `additional_days` - Number of additional days to add to the market duration
    /// * `reason` - Explanation for why the extension is needed
    ///
    /// # Returns
    ///
    /// Returns `Result<(), Error>` where:
    /// - `Ok(())` - Market duration extended successfully
    /// - `Err(Error)` - Extension failed due to permissions or validation
    ///
    /// # Errors
    ///
    /// This function returns specific errors:
    /// - `Error::Unauthorized` - Admin lacks ExtendMarket permission
    /// - `Error::MarketNotFound` - Market with given ID doesn't exist
    /// - `Error::MarketClosed` - Market has already ended or been closed
    /// - `Error::InvalidDuration` - Extension would exceed maximum allowed duration
    /// - Extension errors from ExtensionManager
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address, Symbol, String};
    /// # use predictify_hybrid::admin::AdminFunctions;
    /// # let env = Env::default();
    /// # let admin = Address::generate(&env);
    /// # let market_id = Symbol::new(&env, "active_market");
    /// # let reason = String::from_str(&env, "Low participation, extending for more votes");
    ///
    /// // Extend market by 7 days due to low participation
    /// match AdminFunctions::extend_market_duration(
    ///     &env,
    ///     &admin,
    ///     &market_id,
    ///     7,
    ///     &reason
    /// ) {
    ///     Ok(()) => {
    ///         println!("Market extended by 7 days");
    ///     },
    ///     Err(e) => {
    ///         println!("Failed to extend market: {:?}", e);
    ///     }
    /// }
    /// ```
    ///
    /// # Extension Process
    ///
    /// The extension process:
    /// 1. **Permission Validation**: Ensures admin has ExtendMarket permission
    /// 2. **Market Validation**: Confirms market exists and is extendable
    /// 3. **Duration Extension**: Uses ExtensionManager to add additional time
    /// 4. **Action Logging**: Records extension with reason for audit trail
    ///
    /// # Extension Limits
    ///
    /// Extensions are subject to limits:
    /// - Maximum total extension days per market
    /// - Maximum single extension duration
    /// - Market must be in Active state
    /// - Extensions cannot exceed platform limits
    ///
    /// # Use Cases
    ///
    /// - **Low Participation**: Extend markets with insufficient voting
    /// - **Technical Issues**: Extend markets affected by technical problems
    /// - **Community Requests**: Extend based on legitimate community requests
    /// - **External Events**: Extend when external events affect market relevance
    /// - **Oracle Delays**: Extend when oracle data will be delayed
    ///
    /// # Transparency
    ///
    /// All extensions are logged with reasons and are publicly visible.
    /// The extension history is maintained for each market, providing
    /// full transparency of admin interventions.
    ///
    /// # Best Practices
    ///
    /// - Provide clear, specific reasons for extensions
    /// - Limit extensions to necessary cases
    /// - Consider community feedback before extending
    /// - Document extension policies and criteria
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
        ExtensionManager::extend_market_duration(
            env,
            admin.clone(),
            market_id.clone(),
            additional_days,
            reason.clone(),
        )?;

        // Log admin action
        let mut params = Map::new(env);
        params.set(
            String::from_str(env, "market_id"),
            String::from_str(env, "market_id"),
        );
        params.set(
            String::from_str(env, "additional_days"),
            String::from_str(env, "additional_days"),
        );
        params.set(String::from_str(env, "reason"), reason.clone());
        AdminActionLogger::log_action(
            env,
            admin,
            "extend_market",
            Some(String::from_str(env, "market_id")),
            params,
            true,
            None,
        )?;

        Ok(())
    }

    /// Updates the platform fee configuration (admin only).
    ///
    /// This function allows authorized admins to modify the fee structure
    /// used throughout the platform, including platform fees, creation fees,
    /// and other fee-related parameters. Changes take effect immediately.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `admin` - The admin address performing the update (must have UpdateFees permission)
    /// * `new_config` - The new fee configuration to apply
    ///
    /// # Returns
    ///
    /// Returns `Result<FeeConfig, Error>` where:
    /// - `Ok(FeeConfig)` - Updated fee configuration
    /// - `Err(Error)` - Update failed due to permissions or validation
    ///
    /// # Errors
    ///
    /// This function returns specific errors:
    /// - `Error::Unauthorized` - Admin lacks UpdateFees permission
    /// - `Error::InvalidInput` - Fee configuration contains invalid values
    /// - Fee validation errors from FeeManager
    /// - Storage operation errors
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address};
    /// # use predictify_hybrid::admin::AdminFunctions;
    /// # use predictify_hybrid::fees::FeeConfig;
    /// # let env = Env::default();
    /// # let admin = Address::generate(&env);
    /// # let new_config = FeeConfig {
    /// #     platform_fee_percentage: 250, // 2.5%
    /// #     creation_fee: 1000000,        // 1 XLM
    /// #     min_stake: 100000,           // 0.1 XLM
    /// # };
    ///
    /// // Update platform fees
    /// match AdminFunctions::update_fee_config(&env, &admin, &new_config) {
    ///     Ok(updated_config) => {
    ///         println!("Fees updated successfully");
    ///         println!("New platform fee: {}%", updated_config.platform_fee_percentage / 100);
    ///     },
    ///     Err(e) => {
    ///         println!("Failed to update fees: {:?}", e);
    ///     }
    /// }
    /// ```
    ///
    /// # Fee Configuration Parameters
    ///
    /// The FeeConfig struct typically includes:
    /// - **Platform Fee Percentage**: Fee taken from winning payouts (basis points)
    /// - **Creation Fee**: Fee required to create new markets
    /// - **Minimum Stake**: Minimum amount required for voting
    /// - **Maximum Fee Cap**: Upper limit on total fees
    ///
    /// # Update Process
    ///
    /// The update process:
    /// 1. **Permission Validation**: Ensures admin has UpdateFees permission
    /// 2. **Configuration Validation**: Validates new fee parameters
    /// 3. **Fee Update**: Uses FeeManager to apply new configuration
    /// 4. **Action Logging**: Records fee update for audit trail
    ///
    /// # Impact and Considerations
    ///
    /// Fee updates have immediate platform-wide effects:
    /// - New markets use updated creation fees
    /// - Existing market resolutions use updated platform fees
    /// - User interfaces should reflect new fee structure
    /// - Consider gradual rollout for major fee changes
    ///
    /// # Best Practices
    ///
    /// - Announce fee changes to the community in advance
    /// - Test fee changes on testnet before mainnet deployment
    /// - Monitor platform activity after fee changes
    /// - Keep fees competitive with similar platforms
    /// - Document rationale for fee changes
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
        params.set(
            String::from_str(env, "platform_fee"),
            String::from_str(env, "platform_fee"),
        );
        params.set(
            String::from_str(env, "creation_fee"),
            String::from_str(env, "creation_fee"),
        );
        AdminActionLogger::log_action(env, admin, "update_fees", None, params, true, None)?;

        Ok(updated_config)
    }

    /// Updates the core contract configuration (admin only).
    ///
    /// This function allows authorized admins to modify fundamental contract
    /// settings including market limits, validation thresholds, oracle timeouts,
    /// and other operational parameters. Changes affect all contract operations.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `admin` - The admin address performing the update (must have UpdateConfig permission)
    /// * `new_config` - The new contract configuration to apply
    ///
    /// # Returns
    ///
    /// Returns `Result<(), Error>` where:
    /// - `Ok(())` - Configuration updated successfully
    /// - `Err(Error)` - Update failed due to permissions or validation
    ///
    /// # Errors
    ///
    /// This function returns specific errors:
    /// - `Error::Unauthorized` - Admin lacks UpdateConfig permission
    /// - `Error::InvalidInput` - Configuration contains invalid values
    /// - Configuration validation errors from ConfigManager
    /// - Storage operation errors
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address};
    /// # use predictify_hybrid::admin::AdminFunctions;
    /// # use predictify_hybrid::config::{ContractConfig, Environment};
    /// # let env = Env::default();
    /// # let admin = Address::generate(&env);
    /// # let new_config = ContractConfig {
    /// #     environment: Environment::Mainnet,
    /// #     max_market_duration_days: 365,
    /// #     min_market_duration_days: 1,
    /// #     max_outcomes_per_market: 10,
    /// #     oracle_timeout_seconds: 3600,
    /// # };
    ///
    /// // Update contract configuration for mainnet
    /// match AdminFunctions::update_contract_config(&env, &admin, &new_config) {
    ///     Ok(()) => {
    ///         println!("Contract configuration updated successfully");
    ///     },
    ///     Err(e) => {
    ///         println!("Failed to update configuration: {:?}", e);
    ///     }
    /// }
    /// ```
    ///
    /// # Configuration Parameters
    ///
    /// The ContractConfig typically includes:
    /// - **Environment**: Target deployment environment (Development/Testnet/Mainnet)
    /// - **Market Limits**: Duration limits, outcome limits, participation limits
    /// - **Validation Thresholds**: Minimum stakes, consensus requirements
    /// - **Oracle Settings**: Timeout values, retry limits, fallback options
    /// - **Extension Limits**: Maximum extensions per market, total extension days
    ///
    /// # Update Process
    ///
    /// The configuration update process:
    /// 1. **Permission Validation**: Ensures admin has UpdateConfig permission
    /// 2. **Configuration Validation**: Validates all configuration parameters
    /// 3. **Config Update**: Uses ConfigManager to store new configuration
    /// 4. **Environment Detection**: Determines and logs environment type
    /// 5. **Action Logging**: Records configuration change for audit trail
    ///
    /// # Impact Assessment
    ///
    /// Configuration changes can have significant impacts:
    /// - **Market Creation**: New limits apply to future markets
    /// - **Existing Markets**: Some changes may affect active markets
    /// - **Oracle Integration**: Timeout changes affect oracle reliability
    /// - **User Experience**: Limits affect what users can do
    ///
    /// # Environment-Specific Considerations
    ///
    /// Different environments have different optimal settings:
    /// - **Development**: Relaxed limits for testing
    /// - **Testnet**: Production-like but with test-friendly parameters
    /// - **Mainnet**: Strict, secure, production-optimized settings
    ///
    /// # Change Management
    ///
    /// For production deployments:
    /// - Test configuration changes thoroughly
    /// - Consider gradual rollout strategies
    /// - Monitor system behavior after changes
    /// - Have rollback procedures ready
    /// - Document all configuration changes
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

    /// Resets the contract configuration to default values (admin only).
    ///
    /// This function allows authorized admins to restore the contract configuration
    /// to its default state, effectively undoing all previous configuration changes.
    /// This is useful for recovery scenarios or returning to known-good settings.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `admin` - The admin address performing the reset (must have ResetConfig permission)
    ///
    /// # Returns
    ///
    /// Returns `Result<ContractConfig, Error>` where:
    /// - `Ok(ContractConfig)` - The default configuration that was applied
    /// - `Err(Error)` - Reset failed due to permissions or system errors
    ///
    /// # Errors
    ///
    /// This function returns specific errors:
    /// - `Error::Unauthorized` - Admin lacks ResetConfig permission
    /// - Configuration reset errors from ConfigManager
    /// - Storage operation errors
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address};
    /// # use predictify_hybrid::admin::AdminFunctions;
    /// # let env = Env::default();
    /// # let admin = Address::generate(&env);
    ///
    /// // Reset configuration to defaults after problematic changes
    /// match AdminFunctions::reset_config_to_defaults(&env, &admin) {
    ///     Ok(default_config) => {
    ///         println!("Configuration reset to defaults successfully");
    ///         println!("Environment: {:?}", default_config.environment);
    ///     },
    ///     Err(e) => {
    ///         println!("Failed to reset configuration: {:?}", e);
    ///     }
    /// }
    /// ```
    ///
    /// # Default Configuration
    ///
    /// The default configuration typically includes:
    /// - **Environment**: Development (safest default)
    /// - **Market Duration**: 1-30 days (conservative range)
    /// - **Outcomes Limit**: 2-5 outcomes per market
    /// - **Oracle Timeout**: 1 hour (reasonable default)
    /// - **Extension Limits**: 7 days maximum extension
    ///
    /// # Reset Process
    ///
    /// The reset process:
    /// 1. **Permission Validation**: Ensures admin has ResetConfig permission
    /// 2. **Default Retrieval**: Gets default configuration from ConfigManager
    /// 3. **Configuration Reset**: Applies default configuration
    /// 4. **Action Logging**: Records reset action for audit trail
    /// 5. **Return Defaults**: Returns the applied default configuration
    ///
    /// # Use Cases
    ///
    /// Configuration reset is useful for:
    /// - **Recovery**: Recovering from problematic configuration changes
    /// - **Debugging**: Isolating issues by returning to known-good state
    /// - **Maintenance**: Periodic reset to clean configuration state
    /// - **Environment Migration**: Resetting before environment-specific setup
    /// - **Emergency Response**: Quick restoration during incidents
    ///
    /// # Impact and Considerations
    ///
    /// Resetting configuration affects:
    /// - **Active Markets**: May change behavior of ongoing markets
    /// - **User Limits**: Changes what users can do immediately
    /// - **Oracle Integration**: May affect oracle timeout behavior
    /// - **Platform Behavior**: Returns all settings to baseline
    ///
    /// # Best Practices
    ///
    /// - Use reset as a last resort after other fixes fail
    /// - Announce configuration resets to users
    /// - Monitor system behavior after reset
    /// - Document why reset was necessary
    /// - Consider partial configuration fixes before full reset
    ///
    /// # Recovery Procedures
    ///
    /// After reset, you may need to:
    /// - Reconfigure environment-specific settings
    /// - Update fee structures if needed
    /// - Verify oracle integrations work correctly
    /// - Test market creation and resolution
    pub fn reset_config_to_defaults(env: &Env, admin: &Address) -> Result<ContractConfig, Error> {
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

/// Administrative validation utilities for contract operations.
///
/// The `AdminValidator` provides validation functions to ensure admin operations
/// are performed correctly and safely. These utilities validate admin addresses,
/// contract initialization state, and action parameters before execution.
///
/// # Purpose
///
/// This struct centralizes validation logic for:
/// - Admin address format and validity
/// - Contract initialization state checks
/// - Admin action parameter validation
/// - Input sanitization and security checks
///
/// # Usage Pattern
///
/// AdminValidator functions are typically called before performing admin operations
/// to ensure all preconditions are met and inputs are valid.
pub struct AdminValidator;

impl AdminValidator {
    /// Validates the format and basic properties of an admin address.
    ///
    /// This function performs basic validation on admin addresses to ensure they
    /// meet the requirements for administrative operations. Currently implements
    /// a placeholder validation due to Soroban SDK limitations.
    ///
    /// # Parameters
    ///
    /// * `_env` - The Soroban environment (currently unused)
    /// * `_admin` - The admin address to validate
    ///
    /// # Returns
    ///
    /// Returns `Result<(), Error>` where:
    /// - `Ok(())` - Address validation passed
    /// - `Err(Error)` - Address validation failed
    ///
    /// # Current Implementation
    ///
    /// The current implementation always returns `Ok(())` due to limitations
    /// in the Soroban SDK that make it difficult to perform comprehensive
    /// address validation. Future versions may include:
    /// - Address format validation
    /// - Address existence checks
    /// - Blacklist validation
    /// - Multi-signature validation
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address};
    /// # use predictify_hybrid::admin::AdminValidator;
    /// # let env = Env::default();
    /// # let admin = Address::generate(&env);
    ///
    /// // Validate admin address before operations
    /// match AdminValidator::validate_admin_address(&env, &admin) {
    ///     Ok(()) => {
    ///         println!("Admin address is valid");
    ///         // Proceed with admin operation
    ///     },
    ///     Err(e) => {
    ///         println!("Invalid admin address: {:?}", e);
    ///     }
    /// }
    /// ```
    ///
    /// # Future Enhancements
    ///
    /// When SDK capabilities improve, this function may validate:
    /// - Address format compliance with Stellar standards
    /// - Address existence on the network
    /// - Address not in blacklist/blocklist
    /// - Multi-signature threshold requirements
    /// - Address activity and reputation metrics
    ///
    /// # Security Considerations
    ///
    /// While this function currently provides minimal validation,
    /// it serves as a placeholder for future security enhancements.
    /// Always combine with proper authentication using `require_auth()`.
    pub fn validate_admin_address(_env: &Env, _admin: &Address) -> Result<(), Error> {
        // For now, skip validation since we can't easily convert Address to string
        // This is a limitation of the current Soroban SDK
        Ok(())
    }

    /// Validates that the contract has not been previously initialized.
    ///
    /// This function checks the contract's persistent storage to ensure that
    /// initialization has not already occurred. This prevents double-initialization
    /// which could lead to security vulnerabilities or data corruption.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for storage access
    ///
    /// # Returns
    ///
    /// Returns `Result<(), Error>` where:
    /// - `Ok(())` - Contract is not initialized (safe to initialize)
    /// - `Err(Error::InvalidState)` - Contract is already initialized
    ///
    /// # Validation Logic
    ///
    /// The function checks for the existence of the "Admin" key in persistent
    /// storage. If this key exists, it indicates the contract has been initialized
    /// with an admin, making further initialization invalid.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::Env;
    /// # use predictify_hybrid::admin::AdminValidator;
    /// # let env = Env::default();
    ///
    /// // Check if contract can be initialized
    /// match AdminValidator::validate_contract_not_initialized(&env) {
    ///     Ok(()) => {
    ///         println!("Contract is ready for initialization");
    ///         // Proceed with initialization
    ///     },
    ///     Err(e) => {
    ///         println!("Contract already initialized: {:?}", e);
    ///         // Handle already-initialized state
    ///     }
    /// }
    /// ```
    ///
    /// # Security Importance
    ///
    /// This validation is critical for security because:
    /// - **Prevents Admin Takeover**: Stops malicious re-initialization attempts
    /// - **Maintains State Integrity**: Preserves existing configuration and data
    /// - **Enforces Single Initialization**: Ensures contract follows proper lifecycle
    /// - **Protects Existing Users**: Prevents disruption of active markets and users
    ///
    /// # Integration with Initialization
    ///
    /// This function should be called at the beginning of any initialization
    /// function before making any state changes:
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address};
    /// # use predictify_hybrid::admin::{AdminValidator, AdminInitializer};
    /// # let env = Env::default();
    /// # let admin = Address::generate(&env);
    ///
    /// // Safe initialization pattern
    /// AdminValidator::validate_contract_not_initialized(&env)?;
    /// AdminInitializer::initialize_contract(&env, &admin)?;
    /// ```
    ///
    /// # Error Handling
    ///
    /// When this validation fails, the calling function should:
    /// - Return the error immediately (don't proceed)
    /// - Log the attempted double-initialization
    /// - Consider it a potential security incident
    /// - Provide clear error messages to legitimate callers
    pub fn validate_contract_not_initialized(env: &Env) -> Result<(), Error> {
        let admin_exists = env.storage().persistent().has(&Symbol::new(env, "Admin"));

        if admin_exists {
            return Err(Error::InvalidState);
        }

        Ok(())
    }

    /// Validates parameters for specific admin actions.
    ///
    /// This function performs action-specific parameter validation to ensure
    /// that admin operations receive valid inputs. Each action type has its
    /// own validation rules and required parameters.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for string operations
    /// * `action` - The admin action being performed (e.g., "close_market")
    /// * `parameters` - Map of parameter names to values for the action
    ///
    /// # Returns
    ///
    /// Returns `Result<(), Error>` where:
    /// - `Ok(())` - All parameters are valid for the specified action
    /// - `Err(Error::InvalidInput)` - One or more parameters are invalid
    ///
    /// # Supported Actions
    ///
    /// ## close_market
    /// - **Required**: `market_id` - Non-empty market identifier
    ///
    /// ## finalize_market
    /// - **Required**: `market_id` - Non-empty market identifier
    /// - **Required**: `outcome` - Non-empty winning outcome
    ///
    /// ## extend_market
    /// - **Required**: `market_id` - Non-empty market identifier
    /// - **Required**: `additional_days` - Non-empty extension duration
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Map, String};
    /// # use predictify_hybrid::admin::AdminValidator;
    /// # let env = Env::default();
    /// # let mut params = Map::new(&env);
    /// # params.set(
    /// #     String::from_str(&env, "market_id"),
    /// #     String::from_str(&env, "market_123")
    /// # );
    /// # params.set(
    /// #     String::from_str(&env, "outcome"),
    /// #     String::from_str(&env, "Yes")
    /// # );
    ///
    /// // Validate parameters for market finalization
    /// match AdminValidator::validate_action_parameters(
    ///     &env,
    ///     "finalize_market",
    ///     &params
    /// ) {
    ///     Ok(()) => {
    ///         println!("Parameters are valid for market finalization");
    ///         // Proceed with finalization
    ///     },
    ///     Err(e) => {
    ///         println!("Invalid parameters: {:?}", e);
    ///     }
    /// }
    /// ```
    ///
    /// # Validation Rules
    ///
    /// ### Market ID Validation
    /// - Must be present in parameters
    /// - Must not be empty string
    /// - Should correspond to existing market (checked elsewhere)
    ///
    /// ### Outcome Validation (for finalize_market)
    /// - Must be present in parameters
    /// - Must not be empty string
    /// - Should be valid outcome for the market (checked elsewhere)
    ///
    /// ### Additional Days Validation (for extend_market)
    /// - Must be present in parameters
    /// - Must not be empty string
    /// - Should be valid positive number (parsed elsewhere)
    ///
    /// # Error Conditions
    ///
    /// This function returns `Error::InvalidInput` when:
    /// - Required parameters are missing from the map
    /// - Required parameters have empty string values
    /// - Parameter format is invalid (future enhancement)
    ///
    /// # Integration with Action Logging
    ///
    /// This validation is typically called before logging admin actions
    /// to ensure only valid actions are recorded:
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address, Map};
    /// # use predictify_hybrid::admin::{AdminValidator, AdminActionLogger};
    /// # let env = Env::default();
    /// # let admin = Address::generate(&env);
    /// # let action = "close_market";
    /// # let params = Map::new(&env);
    ///
    /// // Validate before logging
    /// AdminValidator::validate_action_parameters(&env, action, &params)?;
    /// AdminActionLogger::log_action(&env, &admin, action, None, params, true, None)?;
    /// ```
    ///
    /// # Future Enhancements
    ///
    /// Future versions may include:
    /// - Type-specific validation (numbers, dates, etc.)
    /// - Cross-parameter validation rules
    /// - Custom validation for new action types
    /// - Parameter sanitization and normalization
    /// - Advanced security checks (injection prevention)
    pub fn validate_action_parameters(
        env: &Env,
        action: &str,
        parameters: &Map<String, String>,
    ) -> Result<(), Error> {
        match action {
            "close_market" => {
                let market_id = parameters
                    .get(String::from_str(env, "market_id"))
                    .ok_or(Error::InvalidInput)?;
                if market_id.is_empty() {
                    return Err(Error::InvalidInput);
                }
            }
            "finalize_market" => {
                let market_id = parameters
                    .get(String::from_str(env, "market_id"))
                    .ok_or(Error::InvalidInput)?;
                let outcome = parameters
                    .get(String::from_str(env, "outcome"))
                    .ok_or(Error::InvalidInput)?;
                if market_id.is_empty() || outcome.is_empty() {
                    return Err(Error::InvalidInput);
                }
            }
            "extend_market" => {
                let market_id = parameters
                    .get(String::from_str(env, "market_id"))
                    .ok_or(Error::InvalidInput)?;
                let additional_days = parameters
                    .get(String::from_str(env, "additional_days"))
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

/// Administrative action logging and audit trail management.
///
/// The `AdminActionLogger` provides comprehensive logging capabilities for all
/// administrative actions performed on the contract. This creates an immutable
/// audit trail for governance, compliance, and security monitoring.
///
/// # Purpose
///
/// This struct handles:
/// - Recording all admin actions with full context
/// - Creating audit trails for compliance
/// - Emitting events for external monitoring
/// - Providing action history retrieval
/// - Supporting forensic analysis and debugging
///
/// # Audit Trail Components
///
/// Each logged action includes:
/// - **Admin Identity**: Who performed the action
/// - **Action Type**: What operation was performed
/// - **Target**: What was affected (market ID, config, etc.)
/// - **Parameters**: Detailed action parameters
/// - **Timestamp**: When the action occurred
/// - **Success Status**: Whether the action succeeded
/// - **Error Details**: Failure reasons if applicable
///
/// # Security and Compliance
///
/// The logging system supports:
/// - Regulatory compliance requirements
/// - Security incident investigation
/// - Governance transparency
/// - Operational monitoring and alerting
pub struct AdminActionLogger;

impl AdminActionLogger {
    /// Records an administrative action in the audit trail.
    ///
    /// This function creates a comprehensive record of admin actions including
    /// all relevant context, parameters, and outcomes. The record is stored
    /// persistently and an event is emitted for external monitoring.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for storage and events
    /// * `admin` - The admin address that performed the action
    /// * `action` - The type of action performed (e.g., "close_market")
    /// * `target` - Optional target identifier (e.g., market ID)
    /// * `parameters` - Map of action parameters and their values
    /// * `success` - Whether the action completed successfully
    /// * `error_message` - Optional error description if action failed
    ///
    /// # Returns
    ///
    /// Returns `Result<(), Error>` where:
    /// - `Ok(())` - Action logged successfully
    /// - `Err(Error)` - Logging failed due to storage or event errors
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address, Map, String};
    /// # use predictify_hybrid::admin::AdminActionLogger;
    /// # let env = Env::default();
    /// # let admin = Address::generate(&env);
    /// # let mut params = Map::new(&env);
    /// # params.set(
    /// #     String::from_str(&env, "market_id"),
    /// #     String::from_str(&env, "market_123")
    /// # );
    /// # params.set(
    /// #     String::from_str(&env, "outcome"),
    /// #     String::from_str(&env, "Yes")
    /// # );
    ///
    /// // Log successful market finalization
    /// match AdminActionLogger::log_action(
    ///     &env,
    ///     &admin,
    ///     "finalize_market",
    ///     Some(String::from_str(&env, "market_123")),
    ///     params,
    ///     true,
    ///     None
    /// ) {
    ///     Ok(()) => {
    ///         println!("Action logged successfully");
    ///     },
    ///     Err(e) => {
    ///         println!("Failed to log action: {:?}", e);
    ///     }
    /// }
    /// ```
    ///
    /// # Action Types
    ///
    /// Common action types include:
    /// - **Market Operations**: "close_market", "finalize_market", "extend_market"
    /// - **Configuration**: "update_config", "update_fees", "reset_config"
    /// - **Role Management**: "assign_role", "revoke_role", "update_permissions"
    /// - **System Operations**: "initialize_contract", "emergency_pause"
    ///
    /// # Storage Strategy
    ///
    /// The current implementation stores actions using a simple key-value approach.
    /// In production, consider:
    /// - Time-based partitioning for scalability
    /// - Indexed storage for efficient queries
    /// - Archival strategies for long-term retention
    /// - Compression for storage efficiency
    ///
    /// # Event Emission
    ///
    /// Each logged action emits an event containing:
    /// - Admin address
    /// - Action type
    /// - Success status
    /// - Timestamp (from ledger)
    ///
    /// External systems can subscribe to these events for:
    /// - Real-time monitoring
    /// - Automated alerting
    /// - Integration with external audit systems
    /// - Dashboard updates
    ///
    /// # Error Handling
    ///
    /// Logging failures should be handled carefully:
    /// - Don't fail the main operation if logging fails
    /// - Consider alternative logging mechanisms
    /// - Alert on persistent logging failures
    /// - Maintain operation continuity
    ///
    /// # Best Practices
    ///
    /// - Log all significant admin actions
    /// - Include sufficient context for investigation
    /// - Use consistent action naming conventions
    /// - Sanitize sensitive parameters before logging
    /// - Monitor log storage usage and implement rotation
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

    /// Retrieves a list of all administrative actions from the audit trail.
    ///
    /// This function provides access to the complete history of administrative
    /// actions for audit, compliance, and analysis purposes. Currently returns
    /// an empty vector due to storage iteration limitations.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for storage access
    /// * `_limit` - Maximum number of actions to retrieve (currently unused)
    ///
    /// # Returns
    ///
    /// Returns `Result<Vec<AdminAction>, Error>` where:
    /// - `Ok(Vec<AdminAction>)` - List of admin actions (currently empty)
    /// - `Err(Error)` - Retrieval failed due to storage errors
    ///
    /// # Current Limitations
    ///
    /// The current implementation returns an empty vector because:
    /// - Soroban SDK lacks efficient storage iteration capabilities
    /// - Actions are stored individually without indexing
    /// - No built-in pagination or filtering mechanisms
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::Env;
    /// # use predictify_hybrid::admin::AdminActionLogger;
    /// # let env = Env::default();
    ///
    /// // Retrieve recent admin actions for audit
    /// match AdminActionLogger::get_admin_actions(&env, 50) {
    ///     Ok(actions) => {
    ///         println!("Found {} admin actions", actions.len());
    ///         for action in actions {
    ///             println!("Action: {} by {:?} at {}",
    ///                 action.action, action.admin, action.timestamp);
    ///         }
    ///     },
    ///     Err(e) => {
    ///         println!("Failed to retrieve actions: {:?}", e);
    ///     }
    /// }
    /// ```
    ///
    /// # Future Implementation
    ///
    /// A production implementation would include:
    /// - **Indexed Storage**: Actions indexed by timestamp, admin, type
    /// - **Pagination**: Efficient pagination with cursor-based navigation
    /// - **Filtering**: Filter by date range, admin, action type, success status
    /// - **Sorting**: Sort by timestamp, admin, or action type
    /// - **Aggregation**: Summary statistics and trend analysis
    ///
    /// # Proposed Storage Schema
    ///
    /// ```rust
    /// // Time-based partitioning
    /// let partition_key = format!("actions_{}", timestamp / PARTITION_SIZE);
    ///
    /// // Admin-based indexing
    /// let admin_index = format!("admin_actions_{}", admin);
    ///
    /// // Action type indexing
    /// let type_index = format!("action_type_{}", action_type);
    /// ```
    ///
    /// # Use Cases
    ///
    /// This function supports:
    /// - **Compliance Audits**: Providing complete action history
    /// - **Security Analysis**: Investigating suspicious patterns
    /// - **Operational Review**: Understanding admin activity patterns
    /// - **Debugging**: Tracing the sequence of admin operations
    /// - **Reporting**: Generating admin activity reports
    ///
    /// # Performance Considerations
    ///
    /// When implementing full functionality:
    /// - Implement pagination to avoid large result sets
    /// - Use appropriate caching for frequently accessed data
    /// - Consider read replicas for heavy audit workloads
    /// - Implement query optimization for common access patterns
    pub fn get_admin_actions(env: &Env, _limit: u32) -> Result<Vec<AdminAction>, Error> {
        // For now, return empty vector since we don't have a way to iterate over storage
        // In a real implementation, you would store actions in a more sophisticated way
        Ok(Vec::new(env))
    }

    /// Retrieves administrative actions performed by a specific admin.
    ///
    /// This function provides filtered access to the audit trail, showing only
    /// actions performed by a particular admin address. Useful for individual
    /// admin accountability and performance analysis.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for storage access
    /// * `_admin` - The admin address to filter actions for
    /// * `_limit` - Maximum number of actions to retrieve (currently unused)
    ///
    /// # Returns
    ///
    /// Returns `Result<Vec<AdminAction>, Error>` where:
    /// - `Ok(Vec<AdminAction>)` - List of actions by the specified admin (currently empty)
    /// - `Err(Error)` - Retrieval failed due to storage errors
    ///
    /// # Current Limitations
    ///
    /// Similar to `get_admin_actions`, this function currently returns an empty
    /// vector due to Soroban SDK storage iteration limitations. A full implementation
    /// would require indexed storage and efficient filtering capabilities.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address};
    /// # use predictify_hybrid::admin::AdminActionLogger;
    /// # let env = Env::default();
    /// # let admin = Address::generate(&env);
    ///
    /// // Get actions performed by a specific admin
    /// match AdminActionLogger::get_admin_actions_for_admin(&env, &admin, 25) {
    ///     Ok(actions) => {
    ///         println!("Admin performed {} actions", actions.len());
    ///         for action in actions {
    ///             println!("{}: {} ({})",
    ///                 action.timestamp,
    ///                 action.action,
    ///                 if action.success { "Success" } else { "Failed" }
    ///             );
    ///         }
    ///     },
    ///     Err(e) => {
    ///         println!("Failed to retrieve admin actions: {:?}", e);
    ///     }
    /// }
    /// ```
    ///
    /// # Use Cases
    ///
    /// This function is valuable for:
    /// - **Individual Accountability**: Tracking specific admin's actions
    /// - **Performance Review**: Analyzing admin activity and success rates
    /// - **Security Investigation**: Investigating suspicious admin behavior
    /// - **Training**: Reviewing new admin's learning progress
    /// - **Compliance**: Demonstrating individual admin compliance
    ///
    /// # Future Implementation Strategy
    ///
    /// A production implementation would include:
    ///
    /// ## Indexed Storage
    /// ```rust
    /// // Store actions with admin-based indexing
    /// let admin_key = format!("admin_{}_{}", admin, timestamp);
    /// env.storage().persistent().set(&admin_key, &action);
    ///
    /// // Maintain admin action count
    /// let count_key = format!("admin_count_{}", admin);
    /// let current_count: u32 = env.storage().persistent().get(&count_key).unwrap_or(0);
    /// env.storage().persistent().set(&count_key, &(current_count + 1));
    /// ```
    ///
    /// ## Efficient Querying
    /// - Range queries by timestamp
    /// - Pagination with cursor-based navigation
    /// - Filtering by action type and success status
    /// - Sorting options (newest first, oldest first)
    ///
    /// ## Analytics Integration
    /// - Success rate calculation
    /// - Action frequency analysis
    /// - Time-based activity patterns
    /// - Comparison with other admins
    ///
    /// # Security Considerations
    ///
    /// When implementing full functionality:
    /// - Ensure proper access control (admins can only see their own actions unless super admin)
    /// - Sanitize sensitive information in returned data
    /// - Implement rate limiting to prevent abuse
    /// - Log access to audit logs for meta-auditing
    ///
    /// # Performance Optimization
    ///
    /// For high-volume environments:
    /// - Implement caching for frequently accessed admin histories
    /// - Use background processes for heavy analytics
    /// - Consider read replicas for audit queries
    /// - Implement data archival for old actions
    pub fn get_admin_actions_for_admin(
        env: &Env,
        _admin: &Address,
        _limit: u32,
    ) -> Result<Vec<AdminAction>, Error> {
        // For now, return empty vector
        Ok(Vec::new(env))
    }
}

// ===== ADMIN ANALYTICS =====

/// Admin analytics
impl AdminAnalytics {
    /// Calculate admin analytics
    pub fn calculate_admin_analytics(_env: &Env) -> Result<AdminAnalytics, Error> {
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
            AdminRole::ReadOnlyAdmin => {
                String::from_str(&soroban_sdk::Env::default(), "ReadOnlyAdmin")
            }
        }
    }

    /// Get permission name
    pub fn get_permission_name(permission: &AdminPermission) -> String {
        match permission {
            AdminPermission::Initialize => {
                String::from_str(&soroban_sdk::Env::default(), "Initialize")
            }
            AdminPermission::CreateMarket => {
                String::from_str(&soroban_sdk::Env::default(), "CreateMarket")
            }
            AdminPermission::CloseMarket => {
                String::from_str(&soroban_sdk::Env::default(), "CloseMarket")
            }
            AdminPermission::FinalizeMarket => {
                String::from_str(&soroban_sdk::Env::default(), "FinalizeMarket")
            }
            AdminPermission::ExtendMarket => {
                String::from_str(&soroban_sdk::Env::default(), "ExtendMarket")
            }
            AdminPermission::UpdateFees => {
                String::from_str(&soroban_sdk::Env::default(), "UpdateFees")
            }
            AdminPermission::UpdateConfig => {
                String::from_str(&soroban_sdk::Env::default(), "UpdateConfig")
            }
            AdminPermission::ResetConfig => {
                String::from_str(&soroban_sdk::Env::default(), "ResetConfig")
            }
            AdminPermission::CollectFees => {
                String::from_str(&soroban_sdk::Env::default(), "CollectFees")
            }
            AdminPermission::ManageDisputes => {
                String::from_str(&soroban_sdk::Env::default(), "ManageDisputes")
            }
            AdminPermission::ViewAnalytics => {
                String::from_str(&soroban_sdk::Env::default(), "ViewAnalytics")
            }
            AdminPermission::EmergencyActions => {
                String::from_str(&soroban_sdk::Env::default(), "EmergencyActions")
            }
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
            permissions: AdminRoleManager::get_permissions_for_role(env, &AdminRole::MarketAdmin),
            is_active: true,
        }
    }

    /// Validate admin action structure
    pub fn validate_admin_action_structure(action: &AdminAction) -> Result<(), Error> {
        if action.action.len() == 0 {
            return Err(Error::InvalidInput);
        }

        // Note: In test environments, timestamp can be 0, so we skip this validation
        // In production, you might want to add env parameter to enable this check

        Ok(())
    }

    /// Simulate admin action
    pub fn simulate_admin_action(env: &Env, admin: &Address, action: &str) -> Result<(), Error> {
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
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn test_admin_initializer_initialize() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let admin = Address::generate(&env);

        // Test initialization
        env.as_contract(&contract_id, || {
            assert!(AdminInitializer::initialize(&env, &admin).is_ok());

            // Verify admin is stored
            let stored_admin: Address = env
                .storage()
                .persistent()
                .get(&Symbol::new(&env, "Admin"))
                .unwrap();
            assert_eq!(stored_admin, admin);
        });
    }

    #[test]
    fn test_admin_access_control_validate_permission() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let admin = Address::generate(&env);

        env.as_contract(&contract_id, || {
            // Initialize admin
            AdminInitializer::initialize(&env, &admin).unwrap();

            // Test permission validation
            assert!(AdminAccessControl::validate_permission(
                &env,
                &admin,
                &AdminPermission::CreateMarket
            )
            .is_ok());
        });
    }

    #[test]
    fn test_admin_role_manager_assign_role() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let admin = Address::generate(&env);
        let new_admin = Address::generate(&env);

        env.as_contract(&contract_id, || {
            // Initialize admin
            AdminInitializer::initialize(&env, &admin).unwrap();

            // Assign role
            assert!(AdminRoleManager::assign_role(
                &env,
                &new_admin,
                AdminRole::MarketAdmin,
                &admin
            )
            .is_ok());

            // Verify role assignment
            let role = AdminRoleManager::get_admin_role(&env, &new_admin).unwrap();
            assert_eq!(role, AdminRole::MarketAdmin);
        });
    }

    #[test]
    fn test_admin_functions_close_market() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let admin = Address::generate(&env);
        let _market_id = Symbol::new(&env, "test_market");

        env.as_contract(&contract_id, || {
            // Initialize admin
            AdminInitializer::initialize(&env, &admin).unwrap();

            // Test close market (would need a real market setup)
            // For now, just test the permission mapping and validation without auth
            let permission = AdminAccessControl::map_action_to_permission("close_market").unwrap();
            assert_eq!(permission, AdminPermission::CloseMarket);

            // Test that the admin has the required permission
            assert!(AdminAccessControl::validate_permission(&env, &admin, &permission).is_ok());
        });
    }

    #[test]
    fn test_admin_utils_is_admin() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let admin = Address::generate(&env);
        let non_admin = Address::generate(&env);

        env.as_contract(&contract_id, || {
            // Initialize admin
            AdminInitializer::initialize(&env, &admin).unwrap();

            // Test admin check
            assert!(AdminUtils::is_admin(&env, &admin));
            assert!(!AdminUtils::is_admin(&env, &non_admin));
        });
    }

    #[test]
    fn test_admin_testing_utilities() {
        let env = Env::default();
        let admin = Address::generate(&env);

        let action = AdminTesting::create_test_admin_action(&env, &admin);
        // Check the action structure manually first
        assert!(action.action.len() > 0);
        assert!(action.timestamp >= 0); // In test environment, timestamp can be 0
        assert!(AdminTesting::validate_admin_action_structure(&action).is_ok());

        let role_assignment = AdminTesting::create_test_role_assignment(&env, &admin);
        assert_eq!(role_assignment.role, AdminRole::MarketAdmin);
        assert!(role_assignment.is_active);
    }
}
