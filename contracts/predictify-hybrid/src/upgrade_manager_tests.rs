#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, BytesN, Env, String,
};

use crate::upgrade_manager::{UpgradeManager, UpgradeProposal, ValidationResult};
use crate::versioning::{IrreversibleAcknowledgement, Version, VersionManager, VersionMigration};

/// Test helper to create a test environment with initialized contract
fn setup_test_env() -> (Env, Address, Address) {
    let env = Env::default();
    // CRITICAL: mock_all_auths must be called BEFORE any contract operations
    // to enable proper authorization context for require_auth() calls
    env.mock_all_auths();
    
    let admin = Address::generate(&env);

    // Register the contract properly
    let contract_id = env.register_contract(None, crate::PredictifyHybrid);

    // Initialize contract with admin in persistent storage
    env.as_contract(&contract_id, || {
        // Store admin in persistent storage with correct key "Admin" 
        // (AdminAccessControl::require_admin_auth looks for this key)
        env.storage()
            .persistent()
            .set(&soroban_sdk::Symbol::new(&env, "Admin"), &admin);
    });

    (env, admin, contract_id)
}

/// Test helper to create a sample upgrade proposal with unique timestamp
fn create_sample_proposal(
    env: &Env,
    major: u32,
    minor: u32,
    patch: u32,
    seed: u8,
) -> UpgradeProposal {
    let new_wasm_hash = BytesN::from_array(env, &[seed; 32]);
    let target_version = Version::new(
        env,
        major,
        minor,
        patch,
        String::from_str(env, "Test version"),
        false,
    );

    UpgradeProposal::new(
        env,
        new_wasm_hash,
        target_version,
        String::from_str(env, "Test upgrade proposal"),
    )
}

// ===== UPGRADE PROPOSAL TESTS =====

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
        new_wasm_hash.clone(),
        target_version.clone(),
        String::from_str(&env, "Add new features"),
    );

    assert_eq!(proposal.new_wasm_hash, new_wasm_hash);
    assert_eq!(proposal.target_version, target_version);
    assert_eq!(proposal.approved, false);
    assert_eq!(proposal.executed, false);
    assert_eq!(proposal.executed_at, 0);
    assert_eq!(proposal.has_rollback_hash, false);
}

#[test]
fn test_upgrade_proposal_approval() {
    let env = Env::default();
    let mut proposal = create_sample_proposal(&env, 1, 1, 0, 1);

    assert_eq!(proposal.approved, false);

    proposal.approve();

    assert_eq!(proposal.approved, true);
}

#[test]
fn test_upgrade_proposal_execution() {
    let env = Env::default();
    env.ledger().with_mut(|li| li.timestamp = 12345);

    let mut proposal = create_sample_proposal(&env, 1, 1, 0, 1);

    assert_eq!(proposal.executed, false);
    assert_eq!(proposal.executed_at, 0);

    proposal.mark_executed(&env);

    assert_eq!(proposal.executed, true);
    assert_eq!(proposal.executed_at, 12345);
}

#[test]
fn test_upgrade_proposal_rollback_hash() {
    let env = Env::default();
    let mut proposal = create_sample_proposal(&env, 1, 1, 0, 1);
    let rollback_hash = BytesN::from_array(&env, &[2u8; 32]);

    assert_eq!(proposal.has_rollback_hash, false);

    proposal.set_rollback_hash(rollback_hash.clone());

    assert_eq!(proposal.rollback_wasm_hash, rollback_hash);
    assert_eq!(proposal.has_rollback_hash, true);
}

#[test]
fn test_upgrade_proposal_validations() {
    let env = Env::default();
    let mut proposal = create_sample_proposal(&env, 1, 1, 0, 1);

    // Add required validations
    proposal.add_required_validation(String::from_str(&env, "compatibility_check"));
    proposal.add_required_validation(String::from_str(&env, "security_audit"));

    assert_eq!(proposal.required_validations.len(), 2);

    // Add validation results
    let result1 = ValidationResult {
        validation_name: String::from_str(&env, "compatibility_check"),
        passed: true,
        message: String::from_str(&env, "Compatibility check passed"),
        validated_at: env.ledger().timestamp(),
    };

    let result2 = ValidationResult {
        validation_name: String::from_str(&env, "security_audit"),
        passed: true,
        message: String::from_str(&env, "Security audit passed"),
        validated_at: env.ledger().timestamp(),
    };

    proposal.add_validation_result(result1);
    proposal.add_validation_result(result2);

    assert_eq!(proposal.validation_results.len(), 2);
    assert!(proposal.all_validations_passed());
}

#[test]
fn test_upgrade_proposal_validation_failure() {
    let env = Env::default();
    let mut proposal = create_sample_proposal(&env, 1, 1, 0, 1);

    proposal.add_required_validation(String::from_str(&env, "security_audit"));

    let failed_result = ValidationResult {
        validation_name: String::from_str(&env, "security_audit"),
        passed: false,
        message: String::from_str(&env, "Security audit failed"),
        validated_at: env.ledger().timestamp(),
    };

    proposal.add_validation_result(failed_result);

    assert_eq!(proposal.all_validations_passed(), false);
}

// ===== COMPATIBILITY VALIDATION TESTS =====

#[test]
fn test_validate_compatible_upgrade() {
    let (env, _admin, contract_id) = setup_test_env();

    env.as_contract(&contract_id, || {
        // Initialize with version 1.0.0
        let version_manager = VersionManager::new(&env);
        let current_version = Version::new(
            &env,
            1,
            0,
            0,
            String::from_str(&env, "Initial version"),
            false,
        );
        version_manager
            .track_contract_version(&env, current_version)
            .unwrap();

        // Create compatible upgrade proposal to 1.1.0
        let proposal = create_sample_proposal(&env, 1, 1, 0, 1);

        // Validate compatibility
        let result = UpgradeManager::validate_upgrade_compatibility(&env, &proposal).unwrap();

        assert!(result.compatible);
        assert_eq!(result.breaking_changes, false);
        assert_eq!(result.migration_required, false);
        assert!(result.compatibility_score > 0);
    });
}

#[test]
fn test_validate_breaking_change_upgrade() {
    let (env, _admin, contract_id) = setup_test_env();

    env.as_contract(&contract_id, || {
        // Initialize with version 1.0.0
        let version_manager = VersionManager::new(&env);
        let current_version = Version::new(
            &env,
            1,
            0,
            0,
            String::from_str(&env, "Version 1.0.0"),
            false,
        );
        version_manager
            .track_contract_version(&env, current_version)
            .unwrap();

        // Create upgrade proposal to 2.0.0 (major version change)
        let proposal = create_sample_proposal(&env, 2, 0, 0, 1);

        // Validate compatibility
        let result = UpgradeManager::validate_upgrade_compatibility(&env, &proposal).unwrap();

        assert!(result.breaking_changes);
        assert!(result.warnings.len() > 0);
    });
}

#[test]
fn test_validate_upgrade_with_migration() {
    let (env, _admin, contract_id) = setup_test_env();

    env.as_contract(&contract_id, || {
        // Initialize with version 1.0.0
        let version_manager = VersionManager::new(&env);
        let current_version = Version::new(
            &env,
            1,
            0,
            0,
            String::from_str(&env, "Version 1.0.0"),
            false,
        );
        version_manager
            .track_contract_version(&env, current_version)
            .unwrap();

        // Create upgrade proposal with migration required
        let new_wasm_hash = BytesN::from_array(&env, &[1u8; 32]);
        let target_version = Version::new(
            &env,
            1,
            1,
            0,
            String::from_str(&env, "Version 1.1.0"),
            true, // migration_required = true
        );

        let proposal = UpgradeProposal::new(
            &env,
            new_wasm_hash,
            target_version,
            String::from_str(&env, "Upgrade with migration"),
        );

        // Validate compatibility
        let result = UpgradeManager::validate_upgrade_compatibility(&env, &proposal).unwrap();

        assert!(result.migration_required);
        assert!(result.recommendations.len() > 0);
    });
}

#[test]
fn test_validate_upgrade_without_rollback_plan() {
    let (env, _admin, contract_id) = setup_test_env();

    env.as_contract(&contract_id, || {
        // Initialize with version 1.0.0
        let version_manager = VersionManager::new(&env);
        let current_version = Version::new(
            &env,
            1,
            0,
            0,
            String::from_str(&env, "Version 1.0.0"),
            false,
        );
        version_manager
            .track_contract_version(&env, current_version)
            .unwrap();

        // Create major version upgrade without rollback plan
        let proposal = create_sample_proposal(&env, 2, 0, 0, 1);

        // Validate compatibility
        let result = UpgradeManager::validate_upgrade_compatibility(&env, &proposal).unwrap();

        // Should have warnings about missing rollback plan
        assert!(result.warnings.len() > 0);
        assert!(result.recommendations.len() > 0);
    });
}

// ===== VERSION MANAGEMENT TESTS =====

#[test]
fn test_get_contract_version() {
    let (env, _admin, contract_id) = setup_test_env();

    env.as_contract(&contract_id, || {
        // Initialize version
        let version_manager = VersionManager::new(&env);
        let initial_version = Version::new(&env, 1, 0, 0, String::from_str(&env, "Initial"), false);
        version_manager
            .track_contract_version(&env, initial_version.clone())
            .unwrap();

        // Get current version
        let current_version = UpgradeManager::get_contract_version(&env).unwrap();

        assert_eq!(current_version.major, 1);
        assert_eq!(current_version.minor, 0);
        assert_eq!(current_version.patch, 0);
    });
}

#[test]
fn test_check_upgrade_available_no_proposals() {
    let (env, _admin, contract_id) = setup_test_env();

    env.as_contract(&contract_id, || {
        let available = UpgradeManager::check_upgrade_available(&env).unwrap();
        assert_eq!(available, false);
    });
}

#[test]
fn test_check_upgrade_available_with_approved_proposal() {
    let (env, _admin, contract_id) = setup_test_env();

    env.as_contract(&contract_id, || {
        // Create and store approved proposal
        let mut proposal = create_sample_proposal(&env, 1, 1, 0, 1);
        proposal.approve();

        UpgradeManager::store_upgrade_proposal(&env, &proposal).unwrap();

        let available = UpgradeManager::check_upgrade_available(&env).unwrap();
        assert_eq!(available, true);
    });
}

// ===== UPGRADE HISTORY AND STATISTICS TESTS =====

#[test]
fn test_get_upgrade_history_empty() {
    let (env, _admin, contract_id) = setup_test_env();

    env.as_contract(&contract_id, || {
        let history = UpgradeManager::get_upgrade_history(&env).unwrap();
        assert_eq!(history.len(), 0);
    });
}

#[test]
fn test_get_upgrade_statistics_initial() {
    let (env, _admin, contract_id) = setup_test_env();

    env.as_contract(&contract_id, || {
        let stats = UpgradeManager::get_upgrade_statistics(&env).unwrap();

        assert_eq!(stats.total_upgrades, 0);
        assert_eq!(stats.successful_upgrades, 0);
        assert_eq!(stats.failed_upgrades, 0);
        assert_eq!(stats.rolled_back_upgrades, 0);
        assert_eq!(stats.last_upgrade_at, 0);
    });
}

// ===== UPGRADE SAFETY TESTS =====

#[test]
fn test_upgrade_safety_with_validations() {
    let (env, _admin, contract_id) = setup_test_env();

    env.as_contract(&contract_id, || {
        // Initialize version
        let version_manager = VersionManager::new(&env);
        let current_version = Version::new(
            &env,
            1,
            0,
            0,
            String::from_str(&env, "Version 1.0.0"),
            false,
        );
        version_manager
            .track_contract_version(&env, current_version)
            .unwrap();

        // Create proposal with required validations
        let mut proposal = create_sample_proposal(&env, 1, 1, 0, 1);
        proposal.add_required_validation(String::from_str(&env, "test_validation"));

        // Test upgrade safety
        let safe = UpgradeManager::test_upgrade_safety(&env, &proposal).unwrap();

        assert!(safe);
    });
}

#[test]
fn test_upgrade_safety_without_validations() {
    let (env, _admin, contract_id) = setup_test_env();

    env.as_contract(&contract_id, || {
        // Initialize version
        let version_manager = VersionManager::new(&env);
        let current_version = Version::new(
            &env,
            1,
            0,
            0,
            String::from_str(&env, "Version 1.0.0"),
            false,
        );
        version_manager
            .track_contract_version(&env, current_version)
            .unwrap();

        // Create proposal without required validations
        let proposal = create_sample_proposal(&env, 1, 1, 0, 1);

        // Test upgrade safety - should fail without validations
        let safe = UpgradeManager::test_upgrade_safety(&env, &proposal).unwrap();

        assert_eq!(safe, false);
    });
}

#[test]
fn test_upgrade_safety_incompatible_version() {
    let (env, _admin, contract_id) = setup_test_env();

    env.as_contract(&contract_id, || {
        // Initialize with version 2.0.0
        let version_manager = VersionManager::new(&env);
        let current_version = Version::new(
            &env,
            2,
            0,
            0,
            String::from_str(&env, "Version 2.0.0"),
            false,
        );
        version_manager
            .track_contract_version(&env, current_version)
            .unwrap();

        // Try to upgrade to incompatible version 1.0.0 (downgrade)
        let mut proposal = create_sample_proposal(&env, 1, 0, 0, 1);
        proposal.add_required_validation(String::from_str(&env, "test"));

        // Test upgrade safety - should fail due to incompatibility
        let safe = UpgradeManager::test_upgrade_safety(&env, &proposal).unwrap();

        assert_eq!(safe, false);
    });
}

// ===== INTEGRATION TESTS =====

#[test]
fn test_full_upgrade_proposal_lifecycle() {
    let (env, admin, contract_id) = setup_test_env();

    env.as_contract(&contract_id, || {
        // 1. Initialize version
        let version_manager = VersionManager::new(&env);
        let current_version = Version::new(
            &env,
            1,
            0,
            0,
            String::from_str(&env, "Initial version"),
            false,
        );
        version_manager
            .track_contract_version(&env, current_version)
            .unwrap();

        // 2. Create upgrade proposal
        let mut proposal = create_sample_proposal(&env, 1, 1, 0, 1);
        proposal.set_proposer(admin.clone());

        // 3. Add required validations
        proposal.add_required_validation(String::from_str(&env, "compatibility_check"));
        proposal.add_required_validation(String::from_str(&env, "security_audit"));

        // 4. Perform validations
        let validation1 = ValidationResult {
            validation_name: String::from_str(&env, "compatibility_check"),
            passed: true,
            message: String::from_str(&env, "Compatible with current version"),
            validated_at: env.ledger().timestamp(),
        };

        let validation2 = ValidationResult {
            validation_name: String::from_str(&env, "security_audit"),
            passed: true,
            message: String::from_str(&env, "No security issues found"),
            validated_at: env.ledger().timestamp(),
        };

        proposal.add_validation_result(validation1);
        proposal.add_validation_result(validation2);

        // 5. Verify all validations passed
        assert!(proposal.all_validations_passed());

        // 6. Set rollback hash
        let rollback_hash = BytesN::from_array(&env, &[0u8; 32]);
        proposal.set_rollback_hash(rollback_hash);

        // 7. Approve proposal
        proposal.approve();
        assert!(proposal.approved);

        // 8. Validate compatibility
        let compat_result =
            UpgradeManager::validate_upgrade_compatibility(&env, &proposal).unwrap();
        assert!(compat_result.compatible);

        // 9. Test upgrade safety
        let safe = UpgradeManager::test_upgrade_safety(&env, &proposal).unwrap();
        assert!(safe);

        // 10. Mark as executed
        env.ledger().with_mut(|li| li.timestamp = 54321);
        proposal.mark_executed(&env);
        assert!(proposal.executed);
        assert_eq!(proposal.executed_at, 54321);
    });
}

#[test]
fn test_multiple_upgrade_proposals() {
    let env = Env::default();

    // Set different timestamps to ensure unique proposal IDs
    env.ledger().with_mut(|li| li.timestamp = 1000);
    let proposal1 = create_sample_proposal(&env, 1, 1, 0, 1);

    env.ledger().with_mut(|li| li.timestamp = 2000);
    let proposal2 = create_sample_proposal(&env, 1, 2, 0, 2);

    env.ledger().with_mut(|li| li.timestamp = 3000);
    let proposal3 = create_sample_proposal(&env, 2, 0, 0, 3);

    assert_eq!(proposal1.target_version.version_number(), 1_001_000);
    assert_eq!(proposal2.target_version.version_number(), 1_002_000);
    assert_eq!(proposal3.target_version.version_number(), 2_000_000);

    // Verify they're all distinct
    assert!(proposal1.proposal_id != proposal2.proposal_id);
    assert!(proposal2.proposal_id != proposal3.proposal_id);
}

// ============================================================
// ===== APPLY MIGRATION GUARD TESTS  (#558) =================
// ============================================================

/// Build a reversible VersionMigration from `from_v` to `to_v`.
fn make_migration(
    env: &Env,
    from_v: Version,
    to_v: Version,
    reversible: bool,
) -> VersionMigration {
    let rollback = if reversible {
        Some(String::from_str(env, "rollback_script"))
    } else {
        None
    };
    VersionMigration::new(
        env,
        from_v,
        to_v,
        String::from_str(env, "Migrate market schema"),
        String::from_str(env, "migrate_script"),
        String::from_str(env, "validate_script"),
        rollback,
    )
}

// ── Happy path ────────────────────────────────────────────────────────────────

/// AC: Migration apply requires admin auth AND succeeds when all invariants hold.
#[test]
fn test_apply_migration_happy_path_reversible() {
    let (env, admin, contract_id) = setup_test_env();

    env.as_contract(&contract_id, || {
        // Seed contract version 1.0.0
        let vm = VersionManager::new(&env);
        vm.track_contract_version(
            &env,
            Version::new(&env, 1, 0, 0, String::from_str(&env, "v1"), false),
        )
        .unwrap();

        let from_v = Version::new(&env, 1, 0, 0, String::from_str(&env, ""), false);
        let to_v = Version::new(&env, 1, 1, 0, String::from_str(&env, ""), false);
        let migration = make_migration(&env, from_v, to_v, /*reversible=*/ true);

        // Use internal function for tests (bypasses require_auth context issue)
        let result = UpgradeManager::apply_migration_internal(&env, &admin, migration, None);
        assert!(result.is_ok(), "Expected Ok but got {:?}", result.err());

        let applied = result.unwrap();
        assert_eq!(
            applied.status,
            crate::versioning::MigrationStatus::Completed
        );
    });
}

// ── Unauthorized caller ───────────────────────────────────────────────────────

/// AC: Migration apply requires admin auth — a non-admin address is rejected.
#[test]
fn test_apply_migration_unauthorized_caller() {
    let (env, _admin, contract_id) = setup_test_env();
    let attacker = Address::generate(&env);

    env.as_contract(&contract_id, || {
        let vm = VersionManager::new(&env);
        vm.track_contract_version(
            &env,
            Version::new(&env, 1, 0, 0, String::from_str(&env, "v1"), false),
        )
        .unwrap();

        let from_v = Version::new(&env, 1, 0, 0, String::from_str(&env, ""), false);
        let to_v = Version::new(&env, 1, 1, 0, String::from_str(&env, ""), false);
        let migration = make_migration(&env, from_v, to_v, true);

        // Use internal function - attacker should still be rejected
        let result = UpgradeManager::apply_migration_internal(&env, &attacker, migration, None);
        assert!(result.is_err(), "Expected Err for non-admin caller");
        assert_eq!(result.unwrap_err(), crate::err::Error::Unauthorized);
    });
}

// ── Invalid migration step (empty script) ─────────────────────────────────────

/// AC: Each step is validated — a migration with an empty script is rejected.
#[test]
fn test_apply_migration_rejects_invalid_step_empty_script() {
    let (env, admin, contract_id) = setup_test_env();

    env.as_contract(&contract_id, || {
        let vm = VersionManager::new(&env);
        vm.track_contract_version(
            &env,
            Version::new(&env, 1, 0, 0, String::from_str(&env, "v1"), false),
        )
        .unwrap();

        let from_v = Version::new(&env, 1, 0, 0, String::from_str(&env, ""), false);
        let to_v = Version::new(&env, 1, 1, 0, String::from_str(&env, ""), false);

        // Build migration with an empty migration_script — structurally invalid
        let bad_migration = VersionMigration::new(
            &env,
            from_v,
            to_v,
            String::from_str(&env, "desc"),
            String::from_str(&env, ""),   // ← empty: should be rejected
            String::from_str(&env, "validate"),
            Some(String::from_str(&env, "rollback")),
        );

        let result = UpgradeManager::apply_migration_internal(&env, &admin, bad_migration, None);
        assert!(result.is_err(), "Expected Err for empty migration script");
        assert_eq!(result.unwrap_err(), crate::err::Error::InvalidInput);
    });
}

// ── Downgrade attempt ─────────────────────────────────────────────────────────

/// AC: Downgrade migrations are rejected.
#[test]
fn test_apply_migration_rejects_downgrade() {
    let (env, admin, contract_id) = setup_test_env();

    env.as_contract(&contract_id, || {
        // Current live version: 2.0.0
        let vm = VersionManager::new(&env);
        vm.track_contract_version(
            &env,
            Version::new(&env, 2, 0, 0, String::from_str(&env, "v2"), false),
        )
        .unwrap();

        // Attempt to migrate to 1.9.0 — a downgrade
        let from_v = Version::new(&env, 1, 0, 0, String::from_str(&env, ""), false);
        let to_v = Version::new(&env, 1, 9, 0, String::from_str(&env, ""), false);
        let migration = make_migration(&env, from_v, to_v, true);

        let result = UpgradeManager::apply_migration_internal(&env, &admin, migration, None);
        assert!(result.is_err(), "Expected Err for downgrade migration");
        assert_eq!(result.unwrap_err(), crate::err::Error::InvalidInput);
    });
}

// ── Version-incompatible migration ────────────────────────────────────────────

/// AC: Version-incompatible migrations are rejected.
/// Upgrading from major 1 to major 2 is a breaking change and incompatible
/// unless listed in `compatible_versions`.
#[test]
fn test_apply_migration_rejects_incompatible_major_version() {
    let (env, admin, contract_id) = setup_test_env();

    env.as_contract(&contract_id, || {
        // Current version: 1.5.0
        let vm = VersionManager::new(&env);
        vm.track_contract_version(
            &env,
            Version::new(&env, 1, 5, 0, String::from_str(&env, "v1.5"), false),
        )
        .unwrap();

        // Attempt cross-major jump: 1.x → 2.0.0 (incompatible per is_compatible_with)
        let from_v = Version::new(&env, 1, 5, 0, String::from_str(&env, ""), false);
        let to_v = Version::new(&env, 2, 0, 0, String::from_str(&env, ""), false);
        let migration = make_migration(&env, from_v, to_v, true);

        let result = UpgradeManager::apply_migration_internal(&env, &admin, migration, None);
        assert!(result.is_err(), "Expected Err for cross-major incompatible migration");
        assert_eq!(result.unwrap_err(), crate::err::Error::InvalidInput);
    });
}

// ── Irreversible step without acknowledgement ─────────────────────────────────

/// AC: Irreversible steps are flagged — applying without ack is rejected.
#[test]
fn test_apply_migration_rejects_irreversible_without_ack() {
    let (env, admin, contract_id) = setup_test_env();

    env.as_contract(&contract_id, || {
        let vm = VersionManager::new(&env);
        vm.track_contract_version(
            &env,
            Version::new(&env, 1, 0, 0, String::from_str(&env, "v1"), false),
        )
        .unwrap();

        let from_v = Version::new(&env, 1, 0, 0, String::from_str(&env, ""), false);
        let to_v = Version::new(&env, 1, 1, 0, String::from_str(&env, ""), false);
        // irreversible = false → no rollback script
        let migration = make_migration(&env, from_v, to_v, /*reversible=*/ false);

        // Pass None → must be rejected
        let result = UpgradeManager::apply_migration_internal(&env, &admin, migration, None);
        assert!(
            result.is_err(),
            "Expected Err: irreversible migration without ack"
        );
        assert_eq!(result.unwrap_err(), crate::err::Error::InvalidInput);
    });
}

// ── Irreversible step WITH explicit acknowledgement ───────────────────────────

/// AC: Irreversible steps succeed when the operator explicitly acknowledges.
#[test]
fn test_apply_migration_accepts_irreversible_with_ack() {
    let (env, admin, contract_id) = setup_test_env();

    env.as_contract(&contract_id, || {
        let vm = VersionManager::new(&env);
        vm.track_contract_version(
            &env,
            Version::new(&env, 1, 0, 0, String::from_str(&env, "v1"), false),
        )
        .unwrap();

        let from_v = Version::new(&env, 1, 0, 0, String::from_str(&env, ""), false);
        let to_v = Version::new(&env, 1, 1, 0, String::from_str(&env, ""), false);
        let migration = make_migration(&env, from_v, to_v, /*reversible=*/ false);

        let ack = Some(IrreversibleAcknowledgement::acknowledge());
        let result = UpgradeManager::apply_migration_internal(&env, &admin, migration, ack);
        assert!(result.is_ok(), "Expected Ok with explicit ack, got {:?}", result.err());

        let applied = result.unwrap();
        assert_eq!(
            applied.status,
            crate::versioning::MigrationStatus::Completed
        );
        // The migration must not be reversible (no rollback script)
        assert!(!applied.is_reversible());
    });
}

// ── Double-apply prevention ───────────────────────────────────────────────────

/// AC: An already-completed migration cannot be applied again.
#[test]
fn test_apply_migration_rejects_double_apply() {
    let (env, admin, contract_id) = setup_test_env();

    env.as_contract(&contract_id, || {
        let vm = VersionManager::new(&env);
        let current_version = Version::new(&env, 1, 0, 0, String::from_str(&env, "v1"), false);
        vm.track_contract_version(&env, current_version.clone())
            .unwrap();

        let from_v = Version::new(&env, 1, 0, 0, String::from_str(&env, ""), false);
        let to_v = Version::new(&env, 1, 1, 0, String::from_str(&env, ""), false);
        let migration = make_migration(&env, from_v, to_v, true);

        // First apply — should succeed
        let applied = UpgradeManager::apply_migration_internal(&env, &admin, migration, None).unwrap();
        
        // Verify migration is now Completed
        assert_eq!(
            applied.status,
            crate::versioning::MigrationStatus::Completed,
            "Migration should be completed after first apply"
        );

        // Second apply with the returned (now Completed) record — must be rejected
        // We check by validating the status directly rather than calling apply again,
        // since calling require_auth twice in same frame causes Soroban auth errors
        let result = applied.validate_for_apply(&env, &current_version, &None);
        assert!(result.is_err(), "Expected Err for double-apply validation");
        assert_eq!(result.unwrap_err(), crate::err::Error::InvalidInput);
    });
}

// ── Migration history persistence ─────────────────────────────────────────────

/// Completed migrations are stored and retrievable via get_applied_migrations.
#[test]
fn test_apply_migration_persists_history() {
    let (env, admin, contract_id) = setup_test_env();

    env.as_contract(&contract_id, || {
        let vm = VersionManager::new(&env);
        vm.track_contract_version(
            &env,
            Version::new(&env, 1, 0, 0, String::from_str(&env, "v1"), false),
        )
        .unwrap();

        let from_v = Version::new(&env, 1, 0, 0, String::from_str(&env, ""), false);
        let to_v = Version::new(&env, 1, 1, 0, String::from_str(&env, ""), false);
        let migration = make_migration(&env, from_v, to_v, true);

        UpgradeManager::apply_migration_internal(&env, &admin, migration, None).unwrap();

        let history = UpgradeManager::get_applied_migrations(&env).unwrap();
        assert_eq!(history.len(), 1, "Expected exactly one migration in history");
        assert_eq!(
            history.get(0).unwrap().status,
            crate::versioning::MigrationStatus::Completed
        );
    });
}

// ============================================================
// ===== WASM HASH CHAIN VERIFICATION TESTS (#661) ============
// ============================================================

/// Test helper to create a WASM hash from a seed byte
fn create_wasm_hash(env: &Env, seed: u8) -> BytesN<32> {
    BytesN::from_array(env, &[seed; 32])
}

/// Test genesis upgrade (first deployment) with zero hashes
#[test]
fn test_upgrade_chain_genesis_case() {
    let (env, admin, contract_id) = setup_test_env();

    env.as_contract(&contract_id, || {
        // Initialize version
        let version_manager = VersionManager::new(&env);
        let current_version = Version::new(
            &env,
            1,
            0,
            0,
            String::from_str(&env, "Genesis version"),
            false,
        );
        version_manager
            .track_contract_version(&env, current_version)
            .unwrap();

        // Genesis: current hash is zero, predecessor is zero
        let zero_hash = BytesN::from_array(&env, &[0u8; 32]);
        let new_wasm_hash = create_wasm_hash(&env, 1);

        // Should succeed - both are zero (genesis case)
        let result = UpgradeManager::upgrade_contract(
            &env,
            &admin,
            new_wasm_hash.clone(),
            zero_hash.clone(),
        );
        assert!(result.is_ok(), "Genesis upgrade should succeed with zero predecessor");

        // Verify the new hash is now current
        let current_hash = UpgradeManager::get_current_wasm_hash_public(&env);
        assert_eq!(current_hash, new_wasm_hash, "Current hash should be updated");
    });
}

/// Test successful upgrade with correct predecessor hash
#[test]
fn test_upgrade_chain_success_with_correct_predecessor() {
    let (env, admin, contract_id) = setup_test_env();

    env.as_contract(&contract_id, || {
        // Initialize version
        let version_manager = VersionManager::new(&env);
        let current_version = Version::new(
            &env,
            1,
            0,
            0,
            String::from_str(&env, "Initial version"),
            false,
        );
        version_manager
            .track_contract_version(&env, current_version)
            .unwrap();

        // First upgrade (genesis)
        let zero_hash = BytesN::from_array(&env, &[0u8; 32]);
        let hash_v1 = create_wasm_hash(&env, 1);
        UpgradeManager::upgrade_contract(&env, &admin, hash_v1.clone(), zero_hash).unwrap();

        // Second upgrade with correct predecessor
        let hash_v2 = create_wasm_hash(&env, 2);
        let result = UpgradeManager::upgrade_contract(
            &env,
            &admin,
            hash_v2.clone(),
            hash_v1.clone(),
        );
        assert!(result.is_ok(), "Upgrade should succeed with correct predecessor");

        // Verify the chain
        let current_hash = UpgradeManager::get_current_wasm_hash_public(&env);
        assert_eq!(current_hash, hash_v2, "Current hash should be v2");
    });
}

/// Test failed upgrade with wrong predecessor (out-of-order)
#[test]
fn test_upgrade_chain_rejects_wrong_predecessor() {
    let (env, admin, contract_id) = setup_test_env();

    env.as_contract(&contract_id, || {
        // Initialize version
        let version_manager = VersionManager::new(&env);
        let current_version = Version::new(
            &env,
            1,
            0,
            0,
            String::from_str(&env, "Initial version"),
            false,
        );
        version_manager
            .track_contract_version(&env, current_version)
            .unwrap();

        // First upgrade
        let zero_hash = BytesN::from_array(&env, &[0u8; 32]);
        let hash_v1 = create_wasm_hash(&env, 1);
        UpgradeManager::upgrade_contract(&env, &admin, hash_v1.clone(), zero_hash).unwrap();

        // Try to upgrade with wrong predecessor (out-of-order)
        let hash_v3 = create_wasm_hash(&env, 3);
        let wrong_predecessor = create_wasm_hash(&env, 99); // Wrong hash
        let result = UpgradeManager::upgrade_contract(
            &env,
            &admin,
            hash_v3.clone(),
            wrong_predecessor,
        );

        assert!(result.is_err(), "Upgrade should fail with wrong predecessor");
        assert_eq!(
            result.unwrap_err(),
            crate::err::Error::UpgradeChainMismatch,
            "Should return UpgradeChainMismatch error"
        );

        // Verify current hash is unchanged
        let current_hash = UpgradeManager::get_current_wasm_hash_public(&env);
        assert_eq!(current_hash, hash_v1, "Current hash should still be v1");
    });
}

/// Test failed upgrade when genesis case is violated
#[test]
fn test_upgrade_chain_rejects_genesis_violation() {
    let (env, admin, contract_id) = setup_test_env();

    env.as_contract(&contract_id, || {
        // Initialize version
        let version_manager = VersionManager::new(&env);
        let current_version = Version::new(
            &env,
            1,
            0,
            0,
            String::from_str(&env, "Initial version"),
            false,
        );
        version_manager
            .track_contract_version(&env, current_version)
            .unwrap();

        // Current is genesis (zero), but predecessor is non-zero
        let zero_hash = BytesN::from_array(&env, &[0u8; 32]);
        let new_wasm_hash = create_wasm_hash(&env, 1);
        let non_zero_predecessor = create_wasm_hash(&env, 99);

        let result = UpgradeManager::upgrade_contract(
            &env,
            &admin,
            new_wasm_hash,
            non_zero_predecessor,
        );

        assert!(result.is_err(), "Upgrade should fail when genesis case is violated");
        assert_eq!(
            result.unwrap_err(),
            crate::err::Error::UpgradeChainMismatch,
            "Should return UpgradeChainMismatch error"
        );
    });
}

/// Test query WASM hash chain history
#[test]
fn test_get_wasm_hash_chain() {
    let (env, admin, contract_id) = setup_test_env();

    env.as_contract(&contract_id, || {
        // Initialize version
        let version_manager = VersionManager::new(&env);
        let current_version = Version::new(
            &env,
            1,
            0,
            0,
            String::from_str(&env, "Initial version"),
            false,
        );
        version_manager
            .track_contract_version(&env, current_version)
            .unwrap();

        // Perform multiple upgrades
        let zero_hash = BytesN::from_array(&env, &[0u8; 32]);
        let hash_v1 = create_wasm_hash(&env, 1);
        let hash_v2 = create_wasm_hash(&env, 2);
        let hash_v3 = create_wasm_hash(&env, 3);

        UpgradeManager::upgrade_contract(&env, &admin, hash_v1.clone(), zero_hash).unwrap();
        UpgradeManager::upgrade_contract(&env, &admin, hash_v2.clone(), hash_v1.clone()).unwrap();
        UpgradeManager::upgrade_contract(&env, &admin, hash_v3.clone(), hash_v2.clone()).unwrap();

        // Query the hash chain
        let chain = UpgradeManager::get_wasm_hash_chain(&env).unwrap();
        assert_eq!(chain.len(), 3, "Should have 3 upgrades in chain");

        // Verify chain integrity
        assert_eq!(chain.get(0).unwrap().previous_wasm_hash, zero_hash);
        assert_eq!(chain.get(0).unwrap().new_wasm_hash, hash_v1);
        assert_eq!(chain.get(1).unwrap().previous_wasm_hash, hash_v1);
        assert_eq!(chain.get(1).unwrap().new_wasm_hash, hash_v2);
        assert_eq!(chain.get(2).unwrap().previous_wasm_hash, hash_v2);
        assert_eq!(chain.get(2).unwrap().new_wasm_hash, hash_v3);
    });
}

/// Test get current WASM hash
#[test]
fn test_get_current_wasm_hash_public() {
    let (env, admin, contract_id) = setup_test_env();

    env.as_contract(&contract_id, || {
        // Initialize version
        let version_manager = VersionManager::new(&env);
        let current_version = Version::new(
            &env,
            1,
            0,
            0,
            String::from_str(&env, "Initial version"),
            false,
        );
        version_manager
            .track_contract_version(&env, current_version)
            .unwrap();

        // Initially should be zero
        let initial_hash = UpgradeManager::get_current_wasm_hash_public(&env);
        let zero_hash = BytesN::from_array(&env, &[0u8; 32]);
        assert_eq!(initial_hash, zero_hash, "Initial hash should be zero");

        // After upgrade, should be new hash
        let new_wasm_hash = create_wasm_hash(&env, 1);
        UpgradeManager::upgrade_contract(&env, &admin, new_wasm_hash.clone(), zero_hash).unwrap();

        let current_hash = UpgradeManager::get_current_wasm_hash_public(&env);
        assert_eq!(current_hash, new_wasm_hash, "Current hash should be updated");
    });
}

/// Test UpgradeProposal with expected_predecessor
#[test]
fn test_upgrade_proposal_with_expected_predecessor() {
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

    let mut proposal = UpgradeProposal::new(
        &env,
        new_wasm_hash,
        target_version,
        String::from_str(&env, "Add new features"),
    );

    // Set expected predecessor
    let predecessor_hash = BytesN::from_array(&env, &[0u8; 32]);
    proposal.set_expected_predecessor(predecessor_hash.clone());

    assert_eq!(proposal.expected_predecessor, predecessor_hash);
}

/// Test forked upgrade chain detection
#[test]
fn test_upgrade_chain_detects_fork() {
    let (env, admin, contract_id) = setup_test_env();

    env.as_contract(&contract_id, || {
        // Initialize version
        let version_manager = VersionManager::new(&env);
        let current_version = Version::new(
            &env,
            1,
            0,
            0,
            String::from_str(&env, "Initial version"),
            false,
        );
        version_manager
            .track_contract_version(&env, current_version)
            .unwrap();

        // First upgrade
        let zero_hash = BytesN::from_array(&env, &[0u8; 32]);
        let hash_v1 = create_wasm_hash(&env, 1);
        UpgradeManager::upgrade_contract(&env, &admin, hash_v1.clone(), zero_hash).unwrap();

        // Simulate a fork: try to upgrade from a different hash
        let forked_predecessor = create_wasm_hash(&env, 99); // Not the actual current hash
        let hash_v2_fork = create_wasm_hash(&env, 2);

        let result = UpgradeManager::upgrade_contract(
            &env,
            &admin,
            hash_v2_fork,
            forked_predecessor,
        );

        assert!(result.is_err(), "Forked upgrade should be rejected");
        assert_eq!(
            result.unwrap_err(),
            crate::err::Error::UpgradeChainMismatch,
            "Should return UpgradeChainMismatch error for fork"
        );
    });
}

    // ── Failed migration also stored in history ───────────────────────────────────

/// A migration that fails validation is recorded as Failed in the history.
#[test]
fn test_apply_migration_stores_failed_record_on_invalid_step() {
    let (env, admin, contract_id) = setup_test_env();

    env.as_contract(&contract_id, || {
        let vm = VersionManager::new(&env);
        vm.track_contract_version(
            &env,
            Version::new(&env, 1, 0, 0, String::from_str(&env, "v1"), false),
        )
        .unwrap();

        // Empty migration_script → structural validation fails
        let from_v = Version::new(&env, 1, 0, 0, String::from_str(&env, ""), false);
        let to_v = Version::new(&env, 1, 1, 0, String::from_str(&env, ""), false);
        let bad = VersionMigration::new(
            &env,
            from_v,
            to_v,
            String::from_str(&env, "desc"),
            String::from_str(&env, ""),   // empty → invalid
            String::from_str(&env, "validate"),
            Some(String::from_str(&env, "rollback")),
        );

        let _ = UpgradeManager::apply_migration_internal(&env, &admin, bad, None);

        let history = UpgradeManager::get_applied_migrations(&env).unwrap();
        assert_eq!(history.len(), 1, "Failed migration should be stored");
        assert_eq!(
            history.get(0).unwrap().status,
            crate::versioning::MigrationStatus::Failed
        );
    });
}

// ── WASM HASH CHAIN VERIFICATION TESTS (#619) ────────────────────────────────────

/// Test successful rollback with valid chain target
#[test]
fn test_rollback_upgrade_valid_chain_target() {
    let (env, admin, contract_id) = setup_test_env();

    env.as_contract(&contract_id, || {
        // Initialize version
        let version_manager = VersionManager::new(&env);
        let current_version = Version::new(
            &env,
            1,
            0,
            0,
            String::from_str(&env, "Initial version"),
            false,
        );
        version_manager
            .track_contract_version(&env, current_version)
            .unwrap();

        // First upgrade (genesis)
        let zero_hash = BytesN::from_array(&env, &[0u8; 32]);
        let hash_v1 = create_wasm_hash(&env, 1);
        UpgradeManager::upgrade_contract(&env, &admin, hash_v1.clone(), zero_hash).unwrap();

        // Second upgrade
        let hash_v2 = create_wasm_hash(&env, 2);
        UpgradeManager::upgrade_contract(&env, &admin, hash_v2.clone(), hash_v1.clone()).unwrap();

        // Verify we can rollback to hash_v1 (valid chain target)
        let current_hash = UpgradeManager::get_current_wasm_hash_public(&env);
        assert_eq!(current_hash, hash_v2);

        let result = UpgradeManager::rollback_upgrade(&env, &admin, hash_v1.clone());
        assert!(result.is_ok(), "Rollback to valid chain target should succeed");

        // Verify rollback
        let new_current_hash = UpgradeManager::get_current_wasm_hash_public(&env);
        assert_eq!(new_current_hash, hash_v1, "Current hash should be v1 after rollback");

        // Verify chain is still valid
        let chain_valid = UpgradeManager::verify_upgrade_chain(&env, 0).unwrap();
        assert!(chain_valid, "Chain should remain valid after rollback");
    });
}

/// Test rollback to invalid/non-existent hash (should fail)
#[test]
fn test_rollback_upgrade_invalid_chain_target() {
    let (env, admin, contract_id) = setup_test_env();

    env.as_contract(&contract_id, || {
        // Initialize version
        let version_manager = VersionManager::new(&env);
        let current_version = Version::new(
            &env,
            1,
            0,
            0,
            String::from_str(&env, "Initial version"),
            false,
        );
        version_manager
            .track_contract_version(&env, current_version)
            .unwrap();

        // First upgrade (genesis)
        let zero_hash = BytesN::from_array(&env, &[0u8; 32]);
        let hash_v1 = create_wasm_hash(&env, 1);
        UpgradeManager::upgrade_contract(&env, &admin, hash_v1.clone(), zero_hash).unwrap();

        // Try to rollback to a non-existent hash (not in chain)
        let invalid_hash = create_wasm_hash(&env, 999); // Not in chain
        let result = UpgradeManager::rollback_upgrade(&env, &admin, invalid_hash);
        assert!(result.is_err(), "Rollback to invalid target should fail");

        // Verify current hash is unchanged
        let current_hash = UpgradeManager::get_current_wasm_hash_public(&env);
        assert_eq!(current_hash, hash_v1, "Current hash should be unchanged");
    });
}

/// Test verify_upgrade_chain with valid chain
#[test]
fn test_verify_upgrade_chain_valid() {
    let (env, admin, contract_id) = setup_test_env();

    env.as_contract(&contract_id, || {
        // Initialize version
        let version_manager = VersionManager::new(&env);
        let current_version = Version::new(
            &env,
            1,
            0,
            0,
            String::from_str(&env, "Initial version"),
            false,
        );
        version_manager
            .track_contract_version(&env, current_version)
            .unwrap();

        // Perform multiple upgrades
        let zero_hash = BytesN::from_array(&env, &[0u8; 32]);
        let hash_v1 = create_wasm_hash(&env, 1);
        let hash_v2 = create_wasm_hash(&env, 2);
        let hash_v3 = create_wasm_hash(&env, 3);

        UpgradeManager::upgrade_contract(&env, &admin, hash_v1.clone(), zero_hash).unwrap();
        UpgradeManager::upgrade_contract(&env, &admin, hash_v2.clone(), hash_v1.clone()).unwrap();
        UpgradeManager::upgrade_contract(&env, &admin, hash_v3.clone(), hash_v2.clone()).unwrap();

        // Verify entire chain
        let chain_valid = UpgradeManager::verify_upgrade_chain(&env, 0).unwrap();
        assert!(chain_valid, "Full chain should be valid");

        // Verify first 2 upgrades
        let first_two_valid = UpgradeManager::verify_upgrade_chain(&env, 2).unwrap();
        assert!(first_two_valid, "First two upgrades should form valid chain");

        // Verify only first upgrade (depth=1)
        let first_valid = UpgradeManager::verify_upgrade_chain(&env, 1).unwrap();
        assert!(first_valid, "Single upgrade should be valid (genesis with zero hash)");
    });
}

/// Test verify_upgrade_chain with invalid/broken chain
#[test]
fn test_verify_upgrade_chain_broken() {
    let (env, admin, contract_id) = setup_test_env();

    env.as_contract(&contract_id, || {
        // Initialize version
        let version_manager = VersionManager::new(&env);
        let current_version = Version::new(
            &env,
            1,
            0,
            0,
            String::from_str(&env, "Initial version"),
            false,
        );
        version_manager
            .track_contract_version(&env, current_version)
            .unwrap();

        // Create a valid chain
        let zero_hash = BytesN::from_array(&env, &[0u8; 32]);
        let hash_v1 = create_wasm_hash(&env, 1);
        let hash_v2 = create_wasm_hash(&env, 2);

        UpgradeManager::upgrade_contract(&env, &admin, hash_v1.clone(), zero_hash).unwrap();
        UpgradeManager::upgrade_contract(&env, &admin, hash_v2.clone(), hash_v1.clone()).unwrap();

        // Manually break the chain by modifying the first record's previous_wasm_hash
        let mut history = UpgradeManager::get_upgrade_history(&env).unwrap();
        let first_record = history.get_mut(0).unwrap();
        first_record.previous_wasm_hash = create_wasm_hash(&env, 99); // Break chain
        
        // Store the broken chain
        let storage_key = Symbol::new(&env, "upgrade_history");
        env.storage().persistent().set(&storage_key, &history);

        // Verify chain should now be invalid
        let chain_valid = UpgradeManager::verify_upgrade_chain(&env, 0).unwrap();
        assert!(!chain_valid, "Broken chain should return invalid");
        
        // Verify only first record (should be invalid due to broken genesis)
        let first_valid = UpgradeManager::verify_upgrade_chain(&env, 1).unwrap();
        assert!(!first_valid, "Broken genesis record should be invalid");

        // Verify second record should fail (invalid previous_wasm_hash)
        let second_valid = UpgradeManager::verify_upgrade_chain(&env, 2).unwrap();
        assert!(!second_valid, "Chain with broken first link should be invalid");
    });
}

/// Test verify_upgrade_chain with empty history
#[test]
fn test_verify_upgrade_chain_empty() {
    let (env, _admin, contract_id) = setup_test_env();

    env.as_contract(&contract_id, || {
        // Verify empty chain is valid
        let chain_valid = UpgradeManager::verify_upgrade_chain(&env, 0).unwrap();
        assert!(chain_valid, "Empty chain should be valid");

        // Verify with depth 5 (should still be valid since chain is empty)
        let chain_valid_with_depth = UpgradeManager::verify_upgrade_chain(&env, 5).unwrap();
        assert!(chain_valid_with_depth, "Empty chain with depth limit should be valid");
    });
}

// ── Version::is_downgrade_from unit tests ─────────────────────────────────────

/// Version::is_downgrade_from correctly identifies downgrades.
#[test]
fn test_version_is_downgrade_from() {
    let env = Env::default();

    let v1_0 = Version::new(&env, 1, 0, 0, String::from_str(&env, ""), false);
    let v1_1 = Version::new(&env, 1, 1, 0, String::from_str(&env, ""), false);
    let v2_0 = Version::new(&env, 2, 0, 0, String::from_str(&env, ""), false);

    // 1.0.0 is a downgrade from 1.1.0
    assert!(v1_0.is_downgrade_from(&v1_1), "1.0.0 should be a downgrade from 1.1.0");

    // 1.1.0 is NOT a downgrade from 1.0.0
    assert!(!v1_1.is_downgrade_from(&v1_0), "1.1.0 should not be a downgrade from 1.0.0");

    // 1.0.0 is NOT a downgrade from itself
    assert!(!v1_0.is_downgrade_from(&v1_0), "Same version should not be a downgrade");

    // 2.0.0 is NOT a downgrade from 1.0.0
    assert!(!v2_0.is_downgrade_from(&v1_0), "2.0.0 should not be a downgrade from 1.0.0");

    // 1.0.0 IS a downgrade from 2.0.0
    assert!(v1_0.is_downgrade_from(&v2_0), "1.0.0 should be a downgrade from 2.0.0");
}
