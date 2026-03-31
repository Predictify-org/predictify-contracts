#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, BytesN, Env, String,
};

use crate::upgrade_manager::{UpgradeManager, UpgradeProposal, ValidationResult};
use crate::versioning::{Version, VersionManager};

/// Test helper to create a test environment with initialized contract
fn setup_test_env() -> (Env, Address, Address) {
    let env = Env::default();
    let admin = Address::generate(&env);

    // Register the contract properly
    let contract_id = env.register_contract(None, crate::PredictifyHybrid);

    // Initialize contract with admin in contract context
    env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .set(&soroban_sdk::Symbol::new(&env, "admin"), &admin);
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
