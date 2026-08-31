use crate::admin::{
    AdminManager, AdminPermission, AdminRole, AdminRoleAssignment, ContractPauseManager, Severity,
};
use crate::err::Error;
use crate::{PredictifyHybrid, PredictifyHybridClient};
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, BytesN, Env, String, Symbol};

struct TestSetup {
    env: Env,
    contract_id: Address,
    admin: Address,
}

impl TestSetup {
    fn uninitialized() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(PredictifyHybrid, ());
        let admin = Address::generate(&env);

        Self {
            env,
            contract_id,
            admin,
        }
    }

    fn initialized() -> Self {
        let setup = Self::uninitialized();
        setup.client().initialize(&setup.admin, &None, &None);
        setup
    }

    fn client(&self) -> PredictifyHybridClient<'_> {
        PredictifyHybridClient::new(&self.env, &self.contract_id)
    }
}

// ============================================================================
// 1. Entrypoint Authorization Matrix & Unauthorized Caller Rejections
// ============================================================================

#[test]
fn test_upgrade_contract_requires_persistent_primary_admin() {
    let setup = TestSetup::uninitialized();
    let wasm_hash = BytesN::from_array(&setup.env, &[7; 32]);
    let predecessor = BytesN::from_array(&setup.env, &[0; 32]);

    let result = setup
        .client()
        .try_upgrade_contract(&setup.admin, &wasm_hash, &predecessor);

    assert_eq!(result, Err(Ok(Error::AdminNotSet)));
}

#[test]
fn test_upgrade_contract_rejects_legacy_instance_admin_bypass() {
    let setup = TestSetup::initialized();
    let attacker = Address::generate(&setup.env);
    let wasm_hash = BytesN::from_array(&setup.env, &[9; 32]);
    let predecessor = BytesN::from_array(&setup.env, &[0; 32]);

    setup.env.as_contract(&setup.contract_id, || {
        setup
            .env
            .storage()
            .instance()
            .set(&Symbol::new(&setup.env, "admin"), &attacker);
    });

    let result = setup
        .client()
        .try_upgrade_contract(&attacker, &wasm_hash, &predecessor);

    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

#[test]
fn test_upgrade_contract_rejects_delegated_super_admin() {
    let setup = TestSetup::initialized();
    let delegated_super_admin = Address::generate(&setup.env);
    let wasm_hash = BytesN::from_array(&setup.env, &[9; 32]);
    let predecessor = BytesN::from_array(&setup.env, &[0; 32]);

    setup.client().migrate_to_multi_admin(&setup.admin);
    setup
        .client()
        .add_admin(&setup.admin, &delegated_super_admin, &AdminRole::SuperAdmin);

    // Delegated SuperAdmin must NOT be allowed to upgrade contract; only primary admin can.
    let result = setup.client().try_upgrade_contract(
        &delegated_super_admin,
        &wasm_hash,
        &predecessor,
    );

    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

#[test]
fn test_rollback_upgrade_requires_primary_admin() {
    let setup = TestSetup::initialized();
    let outsider = Address::generate(&setup.env);
    let rollback_hash = BytesN::from_array(&setup.env, &[3; 32]);

    let uninit_setup = TestSetup::uninitialized();
    let uninit_hash = BytesN::from_array(&uninit_setup.env, &[3; 32]);
    let uninit_res = uninit_setup
        .client()
        .try_rollback_upgrade(&uninit_setup.admin, &uninit_hash);
    assert_eq!(uninit_res, Err(Ok(Error::AdminNotSet)));

    let outsider_res = setup
        .client()
        .try_rollback_upgrade(&outsider, &rollback_hash);
    assert_eq!(outsider_res, Err(Ok(Error::Unauthorized)));
}

#[test]
fn test_validate_admin_permission_requires_initialized_admin_root() {
    let setup = TestSetup::uninitialized();

    let result = setup
        .client()
        .try_validate_admin_permission(&setup.admin, &AdminPermission::Emergency);

    assert_eq!(result, Err(Ok(Error::AdminNotSet)));
}

#[test]
fn test_migrate_to_multi_admin_requires_primary_admin() {
    let setup = TestSetup::initialized();
    let outsider = Address::generate(&setup.env);

    let result = setup.client().try_migrate_to_multi_admin(&outsider);

    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

#[test]
fn test_delegated_super_admin_can_manage_admins_after_migration() {
    let setup = TestSetup::initialized();
    let delegated_admin = Address::generate(&setup.env);
    let target_admin = Address::generate(&setup.env);

    setup.client().migrate_to_multi_admin(&setup.admin);
    setup
        .client()
        .add_admin(&setup.admin, &delegated_admin, &AdminRole::SuperAdmin);

    let result = setup
        .client()
        .try_add_admin(&delegated_admin, &target_admin, &AdminRole::MarketAdmin);

    assert_eq!(result, Ok(Ok(())));

    let assignment = setup.env.as_contract(&setup.contract_id, || {
        AdminManager::get_admin_assignment(&setup.env, &target_admin)
    });
    assert_eq!(
        assignment.map(|value: AdminRoleAssignment| value.role),
        Some(AdminRole::MarketAdmin)
    );
}

#[test]
fn test_primary_admin_transfer_rotates_entrypoint_access() {
    let setup = TestSetup::initialized();
    let new_admin = Address::generate(&setup.env);

    setup.env.as_contract(&setup.contract_id, || {
        ContractPauseManager::transfer_admin(&setup.env, &setup.admin, &new_admin).unwrap();
    });

    let old_admin_result = setup.client().try_set_platform_fee(&setup.admin, &250i128);
    assert_eq!(old_admin_result, Err(Ok(Error::Unauthorized)));

    let new_admin_result = setup.client().try_set_platform_fee(&new_admin, &250i128);
    assert_eq!(new_admin_result, Ok(Ok(())));
}

#[test]
fn test_admin_broadcast_entrypoint_authorization_matrix() {
    let setup = TestSetup::initialized();
    let outsider = Address::generate(&setup.env);
    let delegated_super = Address::generate(&setup.env);
    let msg_hash = BytesN::from_array(&setup.env, &[42; 32]);
    let reason = String::from_str(&setup.env, "Maintenance scheduled");

    // Primary admin can broadcast
    let primary_res = setup.client().try_admin_broadcast(
        &setup.admin,
        &Severity::Info,
        &msg_hash,
        &reason,
    );
    assert_eq!(primary_res, Ok(Ok(())));

    // Outsider cannot broadcast
    let outsider_res = setup.client().try_admin_broadcast(
        &outsider,
        &Severity::Critical,
        &msg_hash,
        &reason,
    );
    assert_eq!(outsider_res, Err(Ok(Error::Unauthorized)));

    // Multi-admin enabled
    setup.client().migrate_to_multi_admin(&setup.admin);
    setup
        .client()
        .add_admin(&setup.admin, &delegated_super, &AdminRole::SuperAdmin);

    // Delegated SuperAdmin cannot call broadcast (requires primary admin)
    let super_res = setup.client().try_admin_broadcast(
        &delegated_super,
        &Severity::Warning,
        &msg_hash,
        &reason,
    );
    assert_eq!(super_res, Err(Ok(Error::Unauthorized)));
}

// ============================================================================
// 2. Role Rotation & Role Transition Strict Boundary Tests
// ============================================================================

#[test]
fn test_role_rotation_demotion_instantly_drops_privileged_access() {
    let setup = TestSetup::initialized();
    let delegated = Address::generate(&setup.env);
    let target = Address::generate(&setup.env);

    setup.client().migrate_to_multi_admin(&setup.admin);
    setup
        .client()
        .add_admin(&setup.admin, &delegated, &AdminRole::SuperAdmin);

    // SuperAdmin can add an admin
    let add_res = setup
        .client()
        .try_add_admin(&delegated, &target, &AdminRole::FeeAdmin);
    assert_eq!(add_res, Ok(Ok(())));

    // Demote delegated admin from SuperAdmin to MarketAdmin
    let update_res =
        setup
            .client()
            .try_update_admin_role(&setup.admin, &delegated, &AdminRole::MarketAdmin);
    assert_eq!(update_res, Ok(Ok(())));

    // Verify role assignment is updated
    let assignment = setup.env.as_contract(&setup.contract_id, || {
        AdminManager::get_admin_assignment(&setup.env, &delegated)
    });
    assert_eq!(
        assignment.map(|a: AdminRoleAssignment| a.role),
        Some(AdminRole::MarketAdmin)
    );

    // Attempted admin management by demoted MarketAdmin MUST fail immediately with Unauthorized
    let new_target = Address::generate(&setup.env);
    let fail_add = setup
        .client()
        .try_add_admin(&delegated, &new_target, &AdminRole::ConfigAdmin);
    assert_eq!(fail_add, Err(Ok(Error::Unauthorized)));

    let fail_remove = setup.client().try_remove_admin(&delegated, &target);
    assert_eq!(fail_remove, Err(Ok(Error::Unauthorized)));

    let fail_update = setup.client().try_update_admin_role(
        &delegated,
        &target,
        &AdminRole::SuperAdmin,
    );
    assert_eq!(fail_update, Err(Ok(Error::Unauthorized)));
}

#[test]
fn test_role_rotation_demotion_to_readonly_drops_all_write_permissions() {
    let setup = TestSetup::initialized();
    let delegated = Address::generate(&setup.env);

    setup.client().migrate_to_multi_admin(&setup.admin);
    setup
        .client()
        .add_admin(&setup.admin, &delegated, &AdminRole::MarketAdmin);

    // Demote MarketAdmin to ReadOnlyAdmin
    setup
        .client()
        .update_admin_role(&setup.admin, &delegated, &AdminRole::ReadOnlyAdmin);

    // Verify ReadOnlyAdmin cannot validate CreateMarket, UpdateFees, Emergency
    let create_perm = setup
        .client()
        .try_validate_admin_permission(&delegated, &AdminPermission::CreateMarket);
    assert_eq!(create_perm, Err(Ok(Error::Unauthorized)));

    let fee_perm = setup
        .client()
        .try_validate_admin_permission(&delegated, &AdminPermission::UpdateFees);
    assert_eq!(fee_perm, Err(Ok(Error::Unauthorized)));

    let emergency_perm = setup
        .client()
        .try_validate_admin_permission(&delegated, &AdminPermission::Emergency);
    assert_eq!(emergency_perm, Err(Ok(Error::Unauthorized)));

    // ReadOnlyAdmin retains ViewAnalytic permission
    let view_perm = setup
        .client()
        .try_validate_admin_permission(&delegated, &AdminPermission::ViewAnalytic);
    assert_eq!(view_perm, Ok(Ok(())));
}

#[test]
fn test_role_rotation_demoted_admin_cannot_re_escalate_themselves() {
    let setup = TestSetup::initialized();
    let demoted_admin = Address::generate(&setup.env);

    setup.client().migrate_to_multi_admin(&setup.admin);
    setup
        .client()
        .add_admin(&setup.admin, &demoted_admin, &AdminRole::MarketAdmin);

    // Demoted admin attempts to elevate themselves to SuperAdmin
    let escalate_res = setup.client().try_update_admin_role(
        &demoted_admin,
        &demoted_admin,
        &AdminRole::SuperAdmin,
    );
    assert_eq!(escalate_res, Err(Ok(Error::Unauthorized)));

    // Verify role remains MarketAdmin
    let assignment = setup.env.as_contract(&setup.contract_id, || {
        AdminManager::get_admin_assignment(&setup.env, &demoted_admin)
    });
    assert_eq!(
        assignment.map(|a: AdminRoleAssignment| a.role),
        Some(AdminRole::MarketAdmin)
    );
}

#[test]
fn test_admin_removal_immediately_revokes_all_permissions() {
    let setup = TestSetup::initialized();
    let delegated = Address::generate(&setup.env);
    let target = Address::generate(&setup.env);

    setup.client().migrate_to_multi_admin(&setup.admin);
    setup
        .client()
        .add_admin(&setup.admin, &delegated, &AdminRole::SuperAdmin);

    // Remove delegated super admin
    let remove_res = setup.client().try_remove_admin(&setup.admin, &delegated);
    assert_eq!(remove_res, Ok(Ok(())));

    // Verify delegated admin assignment is removed from storage
    let assignment = setup.env.as_contract(&setup.contract_id, || {
        AdminManager::get_admin_assignment(&setup.env, &delegated)
    });
    assert!(assignment.is_none());

    // Removed admin cannot add, update, remove, or validate permissions
    let fail_add = setup
        .client()
        .try_add_admin(&delegated, &target, &AdminRole::MarketAdmin);
    assert_eq!(fail_add, Err(Ok(Error::Unauthorized)));

    let has_perm = setup
        .client()
        .try_validate_admin_permission(&delegated, &AdminPermission::Emergency);
    assert_eq!(has_perm, Err(Ok(Error::Unauthorized)));
}

#[test]
fn test_cannot_remove_or_downgrade_last_super_admin() {
    let setup = TestSetup::initialized();
    let super_admin = Address::generate(&setup.env);

    setup.client().migrate_to_multi_admin(&setup.admin);
    setup
        .client()
        .add_admin(&setup.admin, &super_admin, &AdminRole::SuperAdmin);

    setup.env.as_contract(&setup.contract_id, || {
        let assignment = AdminManager::get_admin_assignment(&setup.env, &super_admin);
        assert_eq!(
            assignment.map(|a| a.role),
            Some(AdminRole::SuperAdmin)
        );
    });

    // Remove the added super admin
    setup.client().remove_admin(&setup.admin, &super_admin);

    // Now only 1 super admin left (primary admin)
    // Removing the last super admin must fail with InvalidState
    let remove_last = setup.client().try_remove_admin(&setup.admin, &setup.admin);
    assert_eq!(remove_last, Err(Ok(Error::InvalidState)));

    let downgrade_last =
        setup
            .client()
            .try_update_admin_role(&setup.admin, &setup.admin, &AdminRole::MarketAdmin);
    assert_eq!(downgrade_last, Err(Ok(Error::InvalidState)));
}

// ============================================================================
// 3. Events Identify the Actor and Action
// ============================================================================

#[test]
fn test_admin_addition_removal_and_role_update_event_emission() {
    let setup = TestSetup::initialized();
    let new_admin = Address::generate(&setup.env);

    setup.client().migrate_to_multi_admin(&setup.admin);

    // Add admin
    setup
        .client()
        .add_admin(&setup.admin, &new_admin, &AdminRole::MarketAdmin);

    // Update admin role
    setup
        .client()
        .update_admin_role(&setup.admin, &new_admin, &AdminRole::FeeAdmin);

    // Remove admin
    setup.client().remove_admin(&setup.admin, &new_admin);

    // Verify assignment is removed
    let assignment = setup.env.as_contract(&setup.contract_id, || {
        AdminManager::get_admin_assignment(&setup.env, &new_admin)
    });
    assert!(assignment.is_none());
}

// ============================================================================
// 4. Negative Tests Covering Every Privileged Variant
// ============================================================================

#[test]
fn test_negative_authorization_matrix_all_privileged_variants() {
    let setup = TestSetup::initialized();
    let outsider = Address::generate(&setup.env);
    let target = Address::generate(&setup.env);
    let wasm_hash = BytesN::from_array(&setup.env, &[8; 32]);
    let predecessor = BytesN::from_array(&setup.env, &[0; 32]);

    setup.client().migrate_to_multi_admin(&setup.admin);

    let roles_to_test = [
        (AdminRole::MarketAdmin, "MarketAdmin"),
        (AdminRole::FeeAdmin, "FeeAdmin"),
        (AdminRole::ConfigAdmin, "ConfigAdmin"),
        (AdminRole::ReadOnlyAdmin, "ReadOnlyAdmin"),
    ];

    for (role, _name) in roles_to_test.iter() {
        let role_admin = Address::generate(&setup.env);
        setup.client().add_admin(&setup.admin, &role_admin, role);

        // None of these restricted roles should be allowed to:
        // 1. Upgrade contract
        let up_res = setup
            .client()
            .try_upgrade_contract(&role_admin, &wasm_hash, &predecessor);
        assert_eq!(up_res, Err(Ok(Error::Unauthorized)));

        // 2. Rollback upgrade
        let rb_res = setup.client().try_rollback_upgrade(&role_admin, &wasm_hash);
        assert_eq!(rb_res, Err(Ok(Error::Unauthorized)));

        // 3. Add admin
        let add_res = setup
            .client()
            .try_add_admin(&role_admin, &target, &AdminRole::ReadOnlyAdmin);
        assert_eq!(add_res, Err(Ok(Error::Unauthorized)));

        // 4. Remove admin
        let rem_res = setup.client().try_remove_admin(&role_admin, &target);
        assert_eq!(rem_res, Err(Ok(Error::Unauthorized)));

        // 5. Update admin role
        let upd_res = setup.client().try_update_admin_role(
            &role_admin,
            &target,
            &AdminRole::SuperAdmin,
        );
        assert_eq!(upd_res, Err(Ok(Error::Unauthorized)));
    }

    // Outsider (non-admin) negative coverage across all privileged endpoints
    assert_eq!(
        setup
            .client()
            .try_upgrade_contract(&outsider, &wasm_hash, &predecessor),
        Err(Ok(Error::Unauthorized))
    );
    assert_eq!(
        setup.client().try_rollback_upgrade(&outsider, &wasm_hash),
        Err(Ok(Error::Unauthorized))
    );
    assert_eq!(
        setup
            .client()
            .try_add_admin(&outsider, &target, &AdminRole::SuperAdmin),
        Err(Ok(Error::Unauthorized))
    );
    assert_eq!(
        setup.client().try_remove_admin(&outsider, &target),
        Err(Ok(Error::Unauthorized))
    );
    assert_eq!(
        setup
            .client()
            .try_update_admin_role(&outsider, &target, &AdminRole::SuperAdmin),
        Err(Ok(Error::Unauthorized))
    );
}

#[test]
fn test_uninitialized_contract_rejects_all_admin_entrypoints() {
    let setup = TestSetup::uninitialized();
    let caller = Address::generate(&setup.env);
    let target = Address::generate(&setup.env);
    let wasm_hash = BytesN::from_array(&setup.env, &[1; 32]);
    let predecessor = BytesN::from_array(&setup.env, &[0; 32]);

    assert_eq!(
        setup
            .client()
            .try_upgrade_contract(&caller, &wasm_hash, &predecessor),
        Err(Ok(Error::AdminNotSet))
    );
    assert_eq!(
        setup.client().try_rollback_upgrade(&caller, &wasm_hash),
        Err(Ok(Error::AdminNotSet))
    );
    assert_eq!(
        setup
            .client()
            .try_validate_admin_permission(&caller, &AdminPermission::Emergency),
        Err(Ok(Error::AdminNotSet))
    );
    assert_eq!(
        setup.client().try_migrate_to_multi_admin(&caller),
        Err(Ok(Error::AdminNotSet))
    );
    assert_eq!(
        setup
            .client()
            .try_add_admin(&caller, &target, &AdminRole::SuperAdmin),
        Err(Ok(Error::AdminNotSet))
    );
    assert_eq!(
        setup.client().try_remove_admin(&caller, &target),
        Err(Ok(Error::AdminNotSet))
    );
    assert_eq!(
        setup
            .client()
            .try_update_admin_role(&caller, &target, &AdminRole::SuperAdmin),
        Err(Ok(Error::AdminNotSet))
    );
}
