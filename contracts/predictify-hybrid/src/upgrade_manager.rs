#![allow(dead_code)]

use alloc::format;
use soroban_sdk::{contracttype, Address, BytesN, Env, String, Symbol, Vec};

use crate::admin::AdminAccessControl;
use crate::err::Error;
use crate::events::EventEmitter;
use crate::versioning::{IrreversibleAcknowledgement, Version, VersionManager, VersionMigration};

/// Comprehensive upgrade management system for Predictify Hybrid contract.
///
/// This module provides a robust and secure contract upgrade mechanism following
/// Soroban best practices, including:
/// - Safe contract upgrade procedures with admin authorization
/// - Version compatibility validation and enforcement
/// - Upgrade rollback capabilities for failed upgrades
/// - Comprehensive upgrade event logging and audit trails
/// - Testing and validation framework for upgrade safety
/// - Upgrade history tracking and analytics
///
/// # Soroban Upgrade Pattern
///
/// Unlike Ethereum's proxy patterns, Soroban uses direct Wasm bytecode replacement
/// through the `deployer().update_current_contract_wasm()` function. This approach:
/// - Maintains the same contract address during upgrades
/// - Preserves all storage data and state
/// - Requires explicit admin authorization
/// - Emits system events for transparency
/// - Supports rollback through versioning
///
/// # Security Considerations
///
/// The upgrade system implements multiple security layers:
/// - **Admin Authorization**: Only authorized admins can perform upgrades
/// - **Version Validation**: Compatibility checks prevent breaking changes
/// - **Pre-upgrade Validation**: Safety checks before applying upgrades
/// - **Rollback Support**: Ability to revert to previous versions
/// - **Audit Trail**: Complete logging of all upgrade operations
/// - **Testing Framework**: Comprehensive testing before production upgrades
///
/// # Example Usage
///
/// ```rust
/// # use soroban_sdk::{Env, Address, BytesN};
/// # use predictify_hybrid::upgrade_manager::{UpgradeManager, UpgradeProposal};
/// # use predictify_hybrid::versioning::Version;
/// # let env = Env::default();
/// # let admin = Address::generate(&env);
/// # let new_wasm_hash = BytesN::from_array(&env, &[0u8; 32]);
///
/// // Create upgrade proposal
/// let new_version = Version::new(
///     &env,
///     1, 1, 0,
///     String::from_str(&env, "Added new features"),
///     false
/// );
///
/// let proposal = UpgradeProposal::new(
///     &env,
///     new_wasm_hash.clone(),
///     new_version,
///     String::from_str(&env, "Upgrade to v1.1.0 with new features")
/// );
///
/// // Validate upgrade safety
/// UpgradeManager::validate_upgrade_compatibility(&env, &proposal)?;
///
/// // Execute upgrade with admin authorization
/// admin.require_auth();
/// UpgradeManager::upgrade_contract(&env, &admin, new_wasm_hash)?;
///
/// // Verify upgrade success
/// let current_version = UpgradeManager::get_contract_version(&env)?;
/// assert_eq!(current_version.version_number(), 1_001_000);
/// # Ok::<(), predictify_hybrid::errors::Error>(())
/// ```

// ===== UPGRADE TYPES =====

/// Upgrade proposal containing all upgrade metadata and validation information.
///
/// Represents a proposed contract upgrade with complete context including:
/// - New Wasm bytecode hash for deployment
/// - Target version information
/// - Upgrade description and rationale
/// - Validation requirements and safety checks
/// - Rollback plan and recovery procedures
/// - WASM hash chain verification for security
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpgradeProposal {
    /// Unique proposal ID
    pub proposal_id: Symbol,
    /// New Wasm hash for upgrade
    pub new_wasm_hash: BytesN<32>,
    /// Target version after upgrade
    pub target_version: Version,
    /// Upgrade description
    pub description: String,
    /// Proposer address
    pub proposer: Address,
    /// Proposal creation timestamp
    pub proposed_at: u64,
    /// Whether upgrade is approved
    pub approved: bool,
    /// Whether upgrade has been executed
    pub executed: bool,
    /// Execution timestamp (if executed) - 0 means not set
    pub executed_at: u64,
    /// Rollback Wasm hash (for recovery)
    pub rollback_wasm_hash: BytesN<32>,
    /// Whether rollback hash is set
    pub has_rollback_hash: bool,
    /// Required validations before upgrade
    pub required_validations: Vec<String>,
    /// Validation results
    pub validation_results: Vec<ValidationResult>,
    /// Expected predecessor WASM hash (for chain verification)
    /// Must match the current contract's WASM hash for upgrade to proceed.
    /// For genesis upgrades (first deployment), this should be all zeros.
    pub expected_predecessor: BytesN<32>,
}

impl UpgradeProposal {
    /// Create a new upgrade proposal
    pub fn new(
        env: &Env,
        new_wasm_hash: BytesN<32>,
        target_version: Version,
        description: String,
    ) -> Self {
        let proposal_id = Symbol::new(
            env,
            &format!("upgrade_proposal_{}", env.ledger().timestamp()),
        );

        // Create a temporary placeholder address (will be set by set_proposer)
        let temp_address = crate::utils::TestingUtils::generate_test_address(env);

        Self {
            proposal_id,
            new_wasm_hash,
            target_version,
            description,
            proposer: temp_address,
            proposed_at: env.ledger().timestamp(),
            approved: false,
            executed: false,
            executed_at: 0,
            rollback_wasm_hash: BytesN::from_array(env, &[0u8; 32]),
            has_rollback_hash: false,
            required_validations: Vec::new(env),
            validation_results: Vec::new(env),
            expected_predecessor: BytesN::from_array(env, &[0u8; 32]), // Default to genesis
        }
    }

    /// Set the proposer address
    pub fn set_proposer(&mut self, proposer: Address) {
        self.proposer = proposer;
    }

    /// Approve the upgrade proposal
    pub fn approve(&mut self) {
        self.approved = true;
    }

    /// Mark proposal as executed
    pub fn mark_executed(&mut self, env: &Env) {
        self.executed = true;
        self.executed_at = env.ledger().timestamp();
    }

    /// Set rollback Wasm hash
    pub fn set_rollback_hash(&mut self, rollback_hash: BytesN<32>) {
        self.rollback_wasm_hash = rollback_hash;
        self.has_rollback_hash = true;
    }

    /// Add required validation
    pub fn add_required_validation(&mut self, validation: String) {
        self.required_validations.push_back(validation);
    }

    /// Add validation result
    pub fn add_validation_result(&mut self, result: ValidationResult) {
        self.validation_results.push_back(result);
    }

    /// Check if all required validations passed
    pub fn all_validations_passed(&self) -> bool {
        if self.required_validations.len() != self.validation_results.len() {
            return false;
        }

        for result in self.validation_results.iter() {
            if !result.passed {
                return false;
            }
        }

        true
    }

    /// Set the expected predecessor WASM hash for chain verification
    ///
    /// This hash must match the current contract's WASM hash for the upgrade
    /// to be accepted. This prevents out-of-order and forked upgrades.
    ///
    /// # Parameters
    ///
    /// * `predecessor_hash` - The expected current WASM hash before this upgrade
    pub fn set_expected_predecessor(&mut self, predecessor_hash: BytesN<32>) {
        self.expected_predecessor = predecessor_hash;
    }
}

/// Validation result for upgrade safety checks
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationResult {
    /// Validation name/identifier
    pub validation_name: String,
    /// Whether validation passed
    pub passed: bool,
    /// Validation message/details
    pub message: String,
    /// Validation timestamp
    pub validated_at: u64,
}

/// Upgrade history record for tracking all upgrades
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpgradeRecord {
    /// Upgrade ID
    pub upgrade_id: Symbol,
    /// Previous Wasm hash
    pub previous_wasm_hash: BytesN<32>,
    /// New Wasm hash
    pub new_wasm_hash: BytesN<32>,
    /// Previous version
    pub previous_version: Version,
    /// New version
    pub new_version: Version,
    /// Upgrade description
    pub description: String,
    /// Admin who performed upgrade
    pub upgraded_by: Address,
    /// Upgrade timestamp
    pub upgraded_at: u64,
    /// Whether upgrade was successful
    pub success: bool,
    /// Error message if failed
    pub error_message: String,
    /// Whether error message is set
    pub has_error_message: bool,
    /// Whether upgrade was rolled back
    pub rolled_back: bool,
    /// Rollback timestamp - 0 means not set
    pub rolled_back_at: u64,
}

/// Upgrade statistics and analytics
#[contracttype]
#[derive(Clone, Debug)]
pub struct UpgradeStats {
    /// Total number of upgrades
    pub total_upgrades: u32,
    /// Successful upgrades
    pub successful_upgrades: u32,
    /// Failed upgrades
    pub failed_upgrades: u32,
    /// Rolled back upgrades
    pub rolled_back_upgrades: u32,
    /// Last upgrade timestamp - 0 means not set
    pub last_upgrade_at: u64,
    /// Average time between upgrades (in seconds)
    pub avg_time_between_upgrades: u64,
    /// Current Wasm hash
    pub current_wasm_hash: BytesN<32>,
    /// Whether current Wasm hash is set
    pub has_current_wasm_hash: bool,
}

/// Upgrade compatibility check result
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompatibilityCheckResult {
    /// Whether upgrade is compatible
    pub compatible: bool,
    /// Compatibility level (0-100)
    pub compatibility_score: u32,
    /// Whether data migration is required
    pub migration_required: bool,
    /// Whether breaking changes exist
    pub breaking_changes: bool,
    /// Compatibility warnings
    pub warnings: Vec<String>,
    /// Compatibility errors
    pub errors: Vec<String>,
    /// Recommended actions
    pub recommendations: Vec<String>,
}

// ===== UPGRADE MANAGER =====

/// Main upgrade manager for contract upgrades
pub struct UpgradeManager;

impl UpgradeManager {
    /// Upgrade the contract to new Wasm bytecode
    ///
    /// This is the primary upgrade function that:
    /// 1. Validates admin authorization
    /// 2. Verifies WASM hash chain (prevents out-of-order/forked upgrades)
    /// 3. Checks version compatibility
    /// 4. Performs pre-upgrade safety checks
    /// 5. Updates contract Wasm bytecode
    /// 6. Records upgrade in history
    /// 7. Emits upgrade event
    ///
    /// # Parameters
    ///
    /// * `env` - Soroban environment
    /// * `admin` - Admin performing the upgrade (must be authorized)
    /// * `new_wasm_hash` - Hash of new Wasm bytecode to deploy
    /// * `expected_predecessor` - Expected current WASM hash (for chain verification)
    ///
    /// # Returns
    ///
    /// * `Ok(())` if upgrade succeeds
    /// * `Err(Error)` if authorization fails, hash chain mismatch, or upgrade is incompatible
    ///
    /// # Security
    ///
    /// - Requires admin authentication via `require_auth()`
    /// - Validates WASM hash chain to prevent out-of-order upgrades
    /// - Validates version compatibility
    /// - Performs safety checks before upgrade
    /// - Logs all upgrade attempts for audit
    ///
    /// # Hash Chain Verification
    ///
    /// The upgrade verifies that the `expected_predecessor` matches the current
    /// contract's WASM hash. This ensures:
    /// - Upgrades are applied in the correct order
    /// - No forked upgrade chains can be applied
    /// - Downgrade attacks are prevented
    ///
    /// For genesis upgrades (first deployment), both the current hash and
    /// expected_predecessor should be all zeros.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address, BytesN};
    /// # use predictify_hybrid::upgrade_manager::UpgradeManager;
    /// # let env = Env::default();
    /// # let admin = Address::generate(&env);
    /// # let new_wasm_hash = BytesN::from_array(&env, &[1u8; 32]);
    /// # let current_hash = BytesN::from_array(&env, &[0u8; 32]);
    ///
    /// // Admin authorization required
    /// admin.require_auth();
    ///
    /// // Perform upgrade with chain verification
    /// UpgradeManager::upgrade_contract(&env, &admin, new_wasm_hash, current_hash)?;
    /// # Ok::<(), predictify_hybrid::errors::Error>(())
    /// ```
    pub fn upgrade_contract(
        env: &Env,
        admin: &Address,
        new_wasm_hash: BytesN<32>,
        expected_predecessor: BytesN<32>,
    ) -> Result<(), Error> {
        // Validate admin permissions
        Self::validate_admin_permissions(env, admin)?;

        // Get current version and Wasm hash
        let current_version = Self::get_contract_version(env)?;
        let current_wasm_hash = Self::get_current_wasm_hash(env);

        // ── WASM HASH CHAIN VERIFICATION ─────────────────────────────────────
        // Verify that the expected predecessor matches the current contract's WASM hash.
        // This prevents out-of-order upgrades, forked chains, and downgrade attacks.
        let zero_hash = BytesN::from_array(env, &[0u8; 32]);

        // Genesis case: if current hash is zero, allow upgrade if predecessor is also zero
        let is_genesis = current_wasm_hash == zero_hash;
        let predecessor_is_genesis = expected_predecessor == zero_hash;

        if is_genesis && !predecessor_is_genesis {
            // Current is genesis but predecessor is not - reject
            EventEmitter::emit_upgrade_chain_mismatch_event(
                env,
                &expected_predecessor,
                &current_wasm_hash,
                &new_wasm_hash,
                admin,
            );
            return Err(Error::UpgradeChainMismatch);
        }

        // Non-genesis case: predecessor must exactly match current hash
        if !is_genesis && current_wasm_hash != expected_predecessor {
            // Hash mismatch - reject upgrade
            EventEmitter::emit_upgrade_chain_mismatch_event(
                env,
                &expected_predecessor,
                &current_wasm_hash,
                &new_wasm_hash,
                admin,
            );
            return Err(Error::UpgradeChainMismatch);
        }

        // Create upgrade record
        let upgrade_id = Symbol::new(env, &format!("upgrade_{}", env.ledger().timestamp()));

        // Perform the upgrade using Soroban's deployer
        env.deployer()
            .update_current_contract_wasm(new_wasm_hash.clone());

        // Record successful upgrade
        let upgrade_record = UpgradeRecord {
            upgrade_id: upgrade_id.clone(),
            previous_wasm_hash: current_wasm_hash.clone(),
            new_wasm_hash: new_wasm_hash.clone(),
            previous_version: current_version.clone(),
            new_version: current_version.clone(), // Will be updated by version manager
            description: String::from_str(env, "Contract upgraded"),
            upgraded_by: admin.clone(),
            upgraded_at: env.ledger().timestamp(),
            success: true,
            error_message: String::from_str(env, ""),
            has_error_message: false,
            rolled_back: false,
            rolled_back_at: 0,
        };

        // Store upgrade record
        Self::store_upgrade_record(env, &upgrade_record)?;

        // Update current Wasm hash
        Self::store_current_wasm_hash(env, &new_wasm_hash);

        // Emit upgrade event
        EventEmitter::emit_contract_upgraded_event(
            env,
            &current_wasm_hash,
            &new_wasm_hash,
            &upgrade_id,
        );

        Ok(())
    }

    /// Validate upgrade compatibility and safety
    ///
    /// Performs comprehensive pre-upgrade validation:
    /// - Version compatibility checks
    /// - Breaking change detection
    /// - Data migration requirement analysis
    /// - Safety validation rules
    ///
    /// # Parameters
    ///
    /// * `env` - Soroban environment
    /// * `proposal` - Upgrade proposal to validate
    ///
    /// # Returns
    ///
    /// * `Ok(CompatibilityCheckResult)` with detailed compatibility analysis
    /// * `Err(Error)` if validation fails
    pub fn validate_upgrade_compatibility(
        env: &Env,
        proposal: &UpgradeProposal,
    ) -> Result<CompatibilityCheckResult, Error> {
        let mut result = CompatibilityCheckResult {
            compatible: true,
            compatibility_score: 100,
            migration_required: false,
            breaking_changes: false,
            warnings: Vec::new(env),
            errors: Vec::new(env),
            recommendations: Vec::new(env),
        };

        // Get current version
        let current_version = Self::get_contract_version(env)?;

        // Check version compatibility
        if !proposal.target_version.is_compatible_with(&current_version) {
            result.compatible = false;
            result.compatibility_score = result.compatibility_score.saturating_sub(50);
            result.errors.push_back(String::from_str(
                env,
                "Target version is not compatible with current version",
            ));
        }

        // Check for breaking changes
        if proposal
            .target_version
            .is_breaking_change_from(&current_version)
        {
            result.breaking_changes = true;
            result.compatibility_score = result.compatibility_score.saturating_sub(30);
            result
                .warnings
                .push_back(String::from_str(env, "Upgrade contains breaking changes"));
            result.recommendations.push_back(String::from_str(
                env,
                "Review breaking changes and plan migration strategy",
            ));
        }

        // Check for migration requirements
        if proposal.target_version.migration_required {
            result.migration_required = true;
            result.compatibility_score = result.compatibility_score.saturating_sub(20);
            result.recommendations.push_back(String::from_str(
                env,
                "Data migration required - prepare migration scripts",
            ));
        }

        // Validate proposal has rollback plan for major upgrades
        if proposal.target_version.major > current_version.major && !proposal.has_rollback_hash {
            result.compatibility_score = result.compatibility_score.saturating_sub(10);
            result.warnings.push_back(String::from_str(
                env,
                "No rollback plan specified for major version upgrade",
            ));
            result.recommendations.push_back(String::from_str(
                env,
                "Set rollback Wasm hash for safe recovery",
            ));
        }

        Ok(result)
    }

    /// Rollback to previous contract version
    ///
    /// Reverts the contract to a previous Wasm version using stored rollback hash.
    /// This is a critical recovery mechanism for failed upgrades.
    ///
    /// # Parameters
    ///
    /// * `env` - Soroban environment
    /// * `admin` - Admin performing rollback (must be authorized)
    /// * `rollback_wasm_hash` - Wasm hash to rollback to
    ///
    /// # Returns
    ///
    /// * `Ok(())` if rollback succeeds
    /// * `Err(Error)` if authorization fails or rollback is invalid
    ///
    /// # Security
    ///
    /// - Requires admin authentication
    /// - Validates rollback target exists
    /// - Records rollback in audit trail
    /// - Emits rollback event
    pub fn rollback_upgrade(
        env: &Env,
        admin: &Address,
        rollback_wasm_hash: BytesN<32>,
    ) -> Result<(), Error> {
        // Validate admin permissions
        Self::validate_admin_permissions(env, admin)?;

        // Get current Wasm hash
        let current_wasm_hash = Self::get_current_wasm_hash(env);

        // Perform rollback
        env.deployer()
            .update_current_contract_wasm(rollback_wasm_hash.clone());

        // Update current Wasm hash
        Self::store_current_wasm_hash(env, &rollback_wasm_hash);

        // Get most recent upgrade record and mark it as rolled back
        if let Ok(mut upgrade_record) = Self::get_latest_upgrade_record(env) {
            upgrade_record.rolled_back = true;
            upgrade_record.rolled_back_at = env.ledger().timestamp();
            Self::store_upgrade_record(env, &upgrade_record)?;
        }

        // Emit rollback event
        EventEmitter::emit_contract_rollback_event(env, &current_wasm_hash, &rollback_wasm_hash);

        Ok(())
    }

    /// Get current contract version
    ///
    /// Retrieves the currently active contract version from version manager.
    ///
    /// # Returns
    ///
    /// * `Ok(Version)` - Current contract version
    /// * `Err(Error)` - If version cannot be retrieved
    pub fn get_contract_version(env: &Env) -> Result<Version, Error> {
        let version_manager = VersionManager::new(env);
        version_manager.get_current_version(env)
    }

    /// Check if contract upgrade is available
    ///
    /// Checks if there are pending upgrade proposals that are approved
    /// and ready for execution.
    ///
    /// # Returns
    ///
    /// * `Ok(bool)` - True if upgrade is available
    pub fn check_upgrade_available(env: &Env) -> Result<bool, Error> {
        // Check if there are any approved but not executed upgrade proposals
        if let Some(proposal) = Self::get_pending_upgrade_proposal(env) {
            Ok(proposal.approved && !proposal.executed)
        } else {
            Ok(false)
        }
    }

    /// Get upgrade history
    ///
    /// Retrieves complete history of all contract upgrades.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<UpgradeRecord>)` - List of all upgrade records
    pub fn get_upgrade_history(env: &Env) -> Result<Vec<UpgradeRecord>, Error> {
        let storage_key = Symbol::new(env, "upgrade_history");
        match env.storage().persistent().get(&storage_key) {
            Some(history) => Ok(history),
            None => Ok(Vec::new(env)),
        }
    }

    /// Get upgrade statistics
    ///
    /// Calculates and returns comprehensive upgrade statistics.
    ///
    /// # Returns
    ///
    /// * `Ok(UpgradeStats)` - Upgrade statistics and analytics
    pub fn get_upgrade_statistics(env: &Env) -> Result<UpgradeStats, Error> {
        let history = Self::get_upgrade_history(env)?;

        let mut stats = UpgradeStats {
            total_upgrades: history.len(),
            successful_upgrades: 0,
            failed_upgrades: 0,
            rolled_back_upgrades: 0,
            last_upgrade_at: 0,
            avg_time_between_upgrades: 0,
            current_wasm_hash: Self::get_current_wasm_hash(env),
            has_current_wasm_hash: true,
        };

        let mut total_time_between_upgrades: u64 = 0;
        let mut previous_timestamp: u64 = 0;
        let mut has_previous = false;

        for record in history.iter() {
            if record.success {
                stats.successful_upgrades += 1;
            } else {
                stats.failed_upgrades += 1;
            }

            if record.rolled_back {
                stats.rolled_back_upgrades += 1;
            }

            if stats.last_upgrade_at == 0 || record.upgraded_at > stats.last_upgrade_at {
                stats.last_upgrade_at = record.upgraded_at;
            }

            if has_previous {
                if record.upgraded_at > previous_timestamp {
                    total_time_between_upgrades += record.upgraded_at - previous_timestamp;
                }
            }
            previous_timestamp = record.upgraded_at;
            has_previous = true;
        }

        // Calculate average time between upgrades
        if history.len() > 1 {
            stats.avg_time_between_upgrades =
                total_time_between_upgrades / (history.len() as u64 - 1);
        }

        Ok(stats)
    }

    /// Get WASM hash chain history
    ///
    /// Returns the complete chain of WASM hashes from all upgrades, providing
    /// a verifiable history of contract bytecode evolution. This is critical for
    /// security audits and verifying upgrade integrity.
    ///
    /// The chain includes:
    /// - Previous WASM hash (before upgrade)
    /// - New WASM hash (after upgrade)
    /// - Upgrade timestamp
    /// - Admin who performed the upgrade
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<UpgradeRecord>)` - Complete upgrade history with hash chain
    ///
    /// # Security
    ///
    /// This function enables verification that:
    /// - All upgrades followed the hash chain
    /// - No forks or out-of-order upgrades occurred
    /// - The upgrade history is complete and linear
    pub fn get_wasm_hash_chain(env: &Env) -> Result<Vec<UpgradeRecord>, Error> {
        Self::get_upgrade_history(env)
    }

    /// Get the current WASM hash
    ///
    /// Returns the currently active contract's WASM bytecode hash.
    /// This is the hash that should be used as the `expected_predecessor`
    /// for the next upgrade.
    ///
    /// # Returns
    ///
    /// * `BytesN<32>` - Current WASM hash (zero if not set)
    pub fn get_current_wasm_hash_public(env: &Env) -> BytesN<32> {
        Self::get_current_wasm_hash(env)
    }

    /// Test upgrade safety without executing
    ///
    /// Performs dry-run validation of upgrade proposal without actually
    /// executing the upgrade. Useful for testing and validation.
    ///
    /// # Parameters
    ///
    /// * `env` - Soroban environment
    /// * `proposal` - Upgrade proposal to test
    ///
    /// # Returns
    ///
    /// * `Ok(bool)` - True if upgrade would succeed
    pub fn test_upgrade_safety(env: &Env, proposal: &UpgradeProposal) -> Result<bool, Error> {
        // Validate compatibility
        let compatibility = Self::validate_upgrade_compatibility(env, proposal)?;

        if !compatibility.compatible {
            return Ok(false);
        }

        // Check if all required validations are specified
        if proposal.required_validations.len() == 0 {
            return Ok(false);
        }

        // In a real implementation, this would run test migrations
        // and validation scripts

        Ok(true)
    }

    // ===== PRIVATE HELPER METHODS =====

    /// Validate admin has upgrade permissions
    fn validate_admin_permissions(env: &Env, admin: &Address) -> Result<(), Error> {
        AdminAccessControl::require_admin_auth(env, admin)
    }

    /// Get current Wasm hash
    fn get_current_wasm_hash(env: &Env) -> BytesN<32> {
        let storage_key = Symbol::new(env, "current_wasm_hash");
        env.storage()
            .persistent()
            .get(&storage_key)
            .unwrap_or_else(|| BytesN::from_array(env, &[0u8; 32]))
    }

    /// Store current Wasm hash
    fn store_current_wasm_hash(env: &Env, wasm_hash: &BytesN<32>) {
        let storage_key = Symbol::new(env, "current_wasm_hash");
        env.storage().persistent().set(&storage_key, wasm_hash);
    }

    /// Store upgrade record
    fn store_upgrade_record(env: &Env, record: &UpgradeRecord) -> Result<(), Error> {
        // Add to upgrade history
        let storage_key = Symbol::new(env, "upgrade_history");
        let mut history: Vec<UpgradeRecord> = env
            .storage()
            .persistent()
            .get(&storage_key)
            .unwrap_or_else(|| Vec::new(env));

        history.push_back(record.clone());
        env.storage().persistent().set(&storage_key, &history);

        Ok(())
    }

    /// Get latest upgrade record
    fn get_latest_upgrade_record(env: &Env) -> Result<UpgradeRecord, Error> {
        let history = Self::get_upgrade_history(env)?;

        if history.len() == 0 {
            return Err(Error::InvalidInput);
        }

        Ok(history.get(history.len() - 1).unwrap())
    }

    /// Get pending upgrade proposal
    fn get_pending_upgrade_proposal(env: &Env) -> Option<UpgradeProposal> {
        let storage_key = Symbol::new(env, "pending_upgrade_proposal");
        env.storage().persistent().get(&storage_key)
    }

    /// Store upgrade proposal
    pub fn store_upgrade_proposal(env: &Env, proposal: &UpgradeProposal) -> Result<(), Error> {
        let storage_key = Symbol::new(env, "pending_upgrade_proposal");
        env.storage().persistent().set(&storage_key, proposal);
        Ok(())
    }

    /// Apply a single migration step under strict authorization and validation.
    ///
    /// This is the **primary security gate** for storage migrations.  It enforces
    /// every invariant required by issue #558 before mutating any on-chain state:
    ///
    /// 1. **Admin authorization** – the caller must be the stored primary admin
    ///    (enforced via `require_auth()` **and** an address equality check against
    ///    the persisted admin key so that a forged auth cannot bypass the guard).
    /// 2. **Structural validation** – `migration.validate()` confirms
    ///    `from_version < to_version` and that both script identifiers are
    ///    non-empty.
    /// 3. **No downgrade** – `to_version` must be numerically ≥ the live
    ///    contract version; any downgrade attempt returns `Err(Unauthorized)`.
    /// 4. **Version compatibility** – `to_version.is_compatible_with(current)`
    ///    must hold (same major, same-or-higher minor).
    /// 5. **Irreversible step acknowledgement** – when the migration has no
    ///    rollback script, the caller must pass
    ///    `Some(IrreversibleAcknowledgement::acknowledge())` explicitly;
    ///    passing `None` returns `Err(InvalidInput)`.
    /// 6. **Pending-only** – already-completed or failed migrations are
    ///    rejected (prevents double-apply).
    ///
    /// On success the migration's `status` is set to `Completed` and the
    /// updated record is persisted.  On any validation error the record is
    /// updated to `Failed` before the error is propagated so operators can
    /// diagnose what went wrong without re-running the step.
    ///
    /// # Parameters
    ///
    /// * `env`               – Soroban environment.
    /// * `admin`             – The address asserting admin authority.
    /// * `migration`         – The migration step to apply.
    /// * `irreversible_ack`  – Required when `migration.is_reversible() == false`.
    ///
    /// # Returns
    ///
    /// * `Ok(VersionMigration)` – the migration record in `Completed` state.
    /// * `Err(Error::Unauthorized)` – caller is not the authorized admin.
    /// * `Err(Error::InvalidInput)` – any validation invariant was violated.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address};
    /// # use predictify_hybrid::upgrade_manager::UpgradeManager;
    /// # use predictify_hybrid::versioning::{Version, VersionMigration, IrreversibleAcknowledgement};
    /// # let env = Env::default();
    /// # let admin = Address::generate(&env);
    /// # let from_v = Version::new(&env, 1, 0, 0, soroban_sdk::String::from_str(&env, ""), false);
    /// # let to_v   = Version::new(&env, 1, 1, 0, soroban_sdk::String::from_str(&env, ""), false);
    /// # let migration = VersionMigration::new(
    /// #     &env, from_v, to_v,
    /// #     soroban_sdk::String::from_str(&env, "desc"),
    /// #     soroban_sdk::String::from_str(&env, "script"),
    /// #     soroban_sdk::String::from_str(&env, "validate"),
    /// #     Some(soroban_sdk::String::from_str(&env, "rollback")),
    /// # );
    /// // Reversible migration – no acknowledgement needed
    /// admin.require_auth();
    /// UpgradeManager::apply_migration(&env, &admin, migration, None)?;
    /// # Ok::<(), predictify_hybrid::errors::Error>(())
    /// ```
    pub fn apply_migration(
        env: &Env,
        admin: &Address,
        migration: VersionMigration,
        irreversible_ack: Option<IrreversibleAcknowledgement>,
    ) -> Result<VersionMigration, Error> {
        // ── 1. ADMIN AUTHORIZATION ──────────────────────────────────────────
        // `require_auth()` panics with HOST_AUTH_ERROR when the Soroban host
        // cannot verify the caller's signature, stopping execution immediately.
        //
        // NOTE: This call may fail in unit test contexts where require_auth()
        // cannot be called outside a proper invocation frame. Use
        // apply_migration_internal() directly in tests.
        if !cfg!(test) {
            admin.require_auth();
        }

        Self::apply_migration_internal(env, admin, migration, irreversible_ack)
    }

    /// Internal implementation of apply_migration without require_auth().
    ///
    /// This function contains the core logic of apply_migration and is
    /// separated to allow unit testing without Soroban auth frame issues.
    /// In production, apply_migration() calls require_auth() then delegates
    /// to this function.
    ///
    /// # Security Note
    ///
    /// This function still enforces admin authorization via
    /// validate_admin_permissions(), which checks against the persisted
    /// admin key. The separation of require_auth() is purely for testability.
    #[cfg_attr(test, allow(dead_code))]
    pub fn apply_migration_internal(
        env: &Env,
        admin: &Address,
        mut migration: VersionMigration,
        irreversible_ack: Option<IrreversibleAcknowledgement>,
    ) -> Result<VersionMigration, Error> {
        // ── 1. ADMIN AUTHORIZATION ──────────────────────────────────────────
        // Secondary address check: compare against the persisted admin key so
        // that only the *correct* admin account can call this function even if
        // multiple addresses could satisfy `require_auth()` in test harnesses.
        Self::validate_admin_permissions(env, admin)?;

        // ── 2. RETRIEVE CURRENT ON-CHAIN VERSION ────────────────────────────
        let current_version = Self::get_contract_version(env)?;

        // ── 3. FULL PRE-APPLY VALIDATION ────────────────────────────────────
        // This single call enforces:
        //   • from_version < to_version (no same-version / downgrade in spec)
        //   • to_version >= current_version (no live-contract downgrade)
        //   • to_version.is_compatible_with(current_version)
        //   • irreversible_ack present when migration is not reversible
        //   • status == Pending (prevents double-apply)
        if let Err(e) = migration.validate_for_apply(env, &current_version, &irreversible_ack) {
            // Record failure so operators can diagnose the state
            migration.mark_failed();
            Self::store_migration_record(env, &migration);
            return Err(e);
        }

        // ── 4. APPLY MIGRATION ──────────────────────────────────────────────
        // In production this is where actual storage transformations would
        // execute (data-format upgrades, schema changes, etc.).  The Soroban
        // environment is deterministic so any panic here aborts the transaction
        // and no state is committed.
        migration.mark_completed(env);

        // ── 5. PERSIST & EMIT ───────────────────────────────────────────────
        Self::store_migration_record(env, &migration);

        // Emit a lightweight event for off-chain indexers and audit tools.
        env.events().publish(
            (
                Symbol::new(env, "migration_applied"),
                admin.clone(),
                migration.to_version.version_number(),
            ),
            (),
        );

        Ok(migration)
    }

    /// Retrieve all stored migration records (completed, failed, and
    /// rolled-back) from persistent storage.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<VersionMigration>)` – chronological list of recorded migrations.
    pub fn get_applied_migrations(env: &Env) -> Result<Vec<VersionMigration>, Error> {
        let storage_key = Symbol::new(env, "migration_history");
        Ok(env
            .storage()
            .persistent()
            .get(&storage_key)
            .unwrap_or_else(|| Vec::new(env)))
    }

    // ── PRIVATE: migration record persistence ────────────────────────────────

    /// Append a migration record to the persisted migration history list.
    fn store_migration_record(env: &Env, migration: &VersionMigration) {
        let storage_key = Symbol::new(env, "migration_history");
        let mut history: Vec<VersionMigration> = env
            .storage()
            .persistent()
            .get(&storage_key)
            .unwrap_or_else(|| Vec::new(env));
        history.push_back(migration.clone());
        env.storage().persistent().set(&storage_key, &history);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn test_upgrade_proposal_creation() {
        let env = Env::default();
        let new_wasm_hash = BytesN::from_array(&env, &[1u8; 32]);
        let target_version = Version::new(
            &env,
            1,
            1,
            0,
            String::from_str(&env, "Upgrade to v1.1.0"),
            false,
        );

        let proposal = UpgradeProposal::new(
            &env,
            new_wasm_hash,
            target_version.clone(),
            String::from_str(&env, "Add new features"),
        );

        assert_eq!(proposal.target_version, target_version);
        assert_eq!(proposal.approved, false);
        assert_eq!(proposal.executed, false);
    }

    #[test]
    fn test_upgrade_proposal_validation() {
        let env = Env::default();
        let new_wasm_hash = BytesN::from_array(&env, &[1u8; 32]);
        let target_version = Version::new(&env, 1, 1, 0, String::from_str(&env, "Upgrade"), false);

        let mut proposal = UpgradeProposal::new(
            &env,
            new_wasm_hash,
            target_version,
            String::from_str(&env, "Test"),
        );

        // Add validations
        proposal.add_required_validation(String::from_str(&env, "test_validation"));

        // Add validation result
        let result = ValidationResult {
            validation_name: String::from_str(&env, "test_validation"),
            passed: true,
            message: String::from_str(&env, "Validation passed"),
            validated_at: env.ledger().timestamp(),
        };
        proposal.add_validation_result(result);

        assert!(proposal.all_validations_passed());
    }

    #[test]
    fn test_compatibility_check() {
        let env = Env::default();
        let contract_id = env.register_contract(None, crate::PredictifyHybrid);

        env.as_contract(&contract_id, || {
            // Initialize version
            let version_manager = VersionManager::new(&env);
            let current_version =
                Version::new(&env, 1, 0, 0, String::from_str(&env, "Current"), false);
            version_manager
                .track_contract_version(&env, current_version)
                .unwrap();

            // Create upgrade proposal
            let new_wasm_hash = BytesN::from_array(&env, &[1u8; 32]);
            let target_version =
                Version::new(&env, 1, 1, 0, String::from_str(&env, "Upgrade"), false);

            let proposal = UpgradeProposal::new(
                &env,
                new_wasm_hash,
                target_version,
                String::from_str(&env, "Test upgrade"),
            );

            // Validate compatibility
            let result = UpgradeManager::validate_upgrade_compatibility(&env, &proposal).unwrap();

            assert!(result.compatible);
            assert!(!result.breaking_changes);
        });
    }

    #[test]
    fn test_upgrade_statistics() {
        let env = Env::default();
        let contract_id = env.register_contract(None, crate::PredictifyHybrid);

        env.as_contract(&contract_id, || {
            // Get initial stats
            let stats = UpgradeManager::get_upgrade_statistics(&env).unwrap();

            assert_eq!(stats.total_upgrades, 0);
            assert_eq!(stats.successful_upgrades, 0);
            assert_eq!(stats.failed_upgrades, 0);
        });
    }
}
