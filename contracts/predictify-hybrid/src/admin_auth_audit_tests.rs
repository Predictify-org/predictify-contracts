use crate::admin::{AdminManager, AdminPermission, AdminRole, ContractPauseManager};
use crate::errors::Error;
use crate::{PredictifyHybrid, PredictifyHybridClient};
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, BytesN, Env, Symbol};

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
        setup.client().initialize(&setup.admin, &None);
        setup
    }

    fn client(&self) -> PredictifyHybridClient<'_> {
        PredictifyHybridClient::new(&self.env, &self.contract_id)
    }
}

#[test]
fn test_upgrade_contract_requires_persistent_primary_admin() {
    let setup = TestSetup::uninitialized();
    let wasm_hash = BytesN::from_array(&setup.env, &[7; 32]);

    let result = setup.client().try_upgrade_contract(&setup.admin, &wasm_hash);

    assert_eq!(result, Err(Ok(Error::AdminNotSet)));
}

#[test]
fn test_upgrade_contract_rejects_legacy_instance_admin_bypass() {
    let setup = TestSetup::initialized();
    let attacker = Address::generate(&setup.env);
    let wasm_hash = BytesN::from_array(&setup.env, &[9; 32]);

    setup.env.as_contract(&setup.contract_id, || {
        setup
            .env
            .storage()
            .instance()
            .set(&Symbol::new(&setup.env, "admin"), &attacker);
    });

    let result = setup.client().try_upgrade_contract(&attacker, &wasm_hash);

    assert_eq!(result, Err(Ok(Error::Unauthorized)));
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
    assert_eq!(assignment.map(|value| value.role), Some(AdminRole::MarketAdmin));
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
