#![no_std]

use soroban_sdk::{contract, contracterror, contractimpl, contracttype, Address, BytesN, Env, Symbol};

/// Storage key for pause flag.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// Boolean flag: true = paused, false = active.
    Paused,
    /// Administrator address authorized to modify contract state.
    Admin,
    /// Current WASM bytecode hash in persistent storage.
    CurrentWasmHash,
}

/// Errors that can be returned by the upgrade contract.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ContractError {
    /// The contract has already been initialized (admin already set).
    AlreadyInitialized = 1,
    /// The administrator has not been configured (contract not initialized).
    AdminNotSet = 2,
    /// The caller is not the stored administrator.
    Unauthorized = 3,
    /// The contract is paused preventing state-changing operations.
    ContractPaused = 4,
    /// No WASM hash has been recorded yet.
    UpgradeHashNotSet = 5,
}

/// The main upgrade contract with pause/resume capability.
#[contract]
pub struct UpgradeContract;

#[contractimpl]
impl UpgradeContract {
    /// Initialize the upgrade contract with an administrator.
    ///
    /// This must be called once to set the admin address. After initialization,
    /// the contract defaults to an unpaused state with no active WASM hash.
    ///
    /// # Arguments
    /// * `admin` - The address that will be granted administrator privileges.
    ///
    /// # Panics
    /// * If called more than once (admin already set).
    ///
    /// # Security
    /// * Requires admin authentication via `require_auth()`.
    pub fn initialize(env: Env, admin: Address) -> Result<(), ContractError> {
        admin.require_auth();
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(ContractError::AlreadyInitialized);
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Paused, &false);
        let zero_hash = BytesN::from_array(&env, &[0u8; 32]);
        env.storage().instance().set(&DataKey::CurrentWasmHash, &zero_hash);
        Ok(())
    }

    /// Pause all state-changing operations.
    ///
    /// When paused, all entrypoints that modify state (upgrade, rollback, etc.)
    /// will return `ContractError::ContractPaused`. Read-only methods remain
    /// functional. Pausing is idempotent — calling it while already paused
    /// is a no-op.
    ///
    /// # Arguments
    /// * `admin` - Must be the stored administrator to authorize.
    ///
    /// # Returns
    /// * `Ok(())` on success.
    ///
    /// # Errors
    /// * `AdminNotSet` - Contract not initialized.
    /// * `Unauthorized` - Caller is not the admin.
    pub fn pause(env: Env, admin: Address) -> Result<(), ContractError> {
        admin.require_auth();

        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(ContractError::AdminNotSet)?;

        if admin != stored_admin {
            return Err(ContractError::Unauthorized);
        }

        env.storage().instance().set(&DataKey::Paused, &true);
        env.events().publish((Symbol::new(&env, "paused"), admin), ()));
        Ok(())
    }

    /// Resume all state-changing operations.
    ///
    /// Unpauses the contract. Idempotent — safe to call while already active.
    /// Admin-only.
    ///
    /// # Arguments
    /// * `admin` - Must be the stored administrator.
    ///
    /// # Returns
    /// * `Ok(())` on success.
    ///
    /// # Errors
    /// * `AdminNotSet` - Contract not initialized.
    /// * `Unauthorized` - Caller is not the admin.
    pub fn resume(env: Env, admin: Address) -> Result<(), ContractError> {
        admin.require_auth();

        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(ContractError::AdminNotSet)?;

        if admin != stored_admin {
            return Err(ContractError::Unauthorized);
        }

        env.storage().instance().set(&DataKey::Paused, &false);
        env.events().publish((Symbol::new(&env, "resumed"), admin), ()));
        Ok(())
    }

    /// Returns the pause status.
    ///
    /// # Returns
    /// * `true` if the contract is paused.
    /// * `false` if the contract is active.
    pub fn is_paused(env: Env) -> bool {
        env.storage().instance().get(&DataKey::Paused).unwrap_or(false)
    }

    /// Verifies the contract is not paused before performing state changes.
    ///
    /// # Returns
    /// * `Ok(())` if active.
    ///
    /// # Errors
    /// * `ContractPaused` if paused.
    pub fn require_not_paused(env: Env) -> Result<(), ContractError> {
        if Self::is_paused(env) {
            return Err(ContractError::ContractPaused);
        }
        Ok(())
    }

    /// Provides the current WASM hash.
    ///
    /// Read-only entrypoint; works while paused.
    ///
    /// # Returns
    /// * The current WASM hash (zero if not set).
    pub fn current_wasm_hash(env: Env) -> BytesN<32> {
        env.storage()
            .instance()
            .get(&DataKey::CurrentWasmHash)
            .unwrap_or(BytesN::from_array(&env, &[0u8; 32]))
    }

    /// Upgrade the contract to a new WASM bytecode.
    ///
    /// This is the primary state-changing upgrade entrypoint. Must be authorized
    /// by admin and the contract must be active. Paused contracts block this.
    ///
    /// # Arguments
    /// * `admin` - The administrator authorizing the upgrade.
    /// * `new_hash` - The 32-byte hash of the new WASM bytecode.
    ///
    /// # Returns
    /// * `Ok(())` on successful upgrade.
    ///
    /// # Errors
    /// * `AdminNotSet` - Contract not initialized.
    /// * `Unauthorized` - Caller is not admin.
    /// * `ContractPaused` - Contract is paused.
    pub fn upgrade_wasm(
        env: Env,
        admin: Address,
        new_hash: BytesN<32>,
    ) -> Result<(), ContractError> {
        admin.require_auth();

        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(ContractError::AdminNotSet)?;

        if admin != stored_admin {
            return Err(ContractError::Unauthorized);
        }

        Self::require_not_paused(env.clone())?;

        let old_hash = env
            .storage()
            .instance()
            .get(&DataKey::CurrentWasmHash)
            .unwrap_or(BytesN::from_array(&env, &[0u8; 32]));

        // In production: env.deployer().update_current_contract_wasm(new_hash);

        env.storage()
            .instance()
            .set(&DataKey::CurrentWasmHash, &new_hash);

        env.events().publish(
            (Symbol::new(&env, "wasm_upgraded"), admin),
            (old_hash, new_hash),
        );

        Ok(())
    }

    /// Rollback to a previous WASM hash.
    ///
    /// State-changing entrypoint; admin-authorized and requires active state.
    /// Used to recover from a failed upgrade.
    ///
    /// # Arguments
    /// * `admin` - The administrator authorizing the rollback.
    /// * `previous_hash` - The 32-byte hash of the target version.
    ///
    /// # Returns
    /// * `Ok(())` on successful rollback.
    ///
    /// # Errors
    /// * `AdminNotSet` - Contract not initialized.
    /// * `Unauthorized` - Caller is not admin.
    /// * `ContractPaused` - Contract is paused.
    pub fn rollback(
        env: Env,
        admin: Address,
        previous_hash: BytesN<32>,
    ) -> Result<(), ContractError> {
        admin.require_auth();

        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(ContractError::AdminNotSet)?;

        if admin != stored_admin {
            return Err(ContractError::Unauthorized);
        }

        Self::require_not_paused(env.clone())?;

        // Real rollback would verify hash is in upgrade chain.
        env.storage()
            .instance()
            .set(&DataKey::CurrentWasmHash, &previous_hash);

        env.events().publish(
            (Symbol::new(&env, "wasm_rolled_back"), admin),
            (previous_hash),
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    /// Test initialization sets admin correctly and defaults to unpaused
    #[test]
    fn test_initialize() {
        let env = Env::default();
        let admin = Address::generate(&env);

        let contract_id = env.register_contract(None, UpgradeContract);
        env.as_contract(&contract_id, || {
            let result = UpgradeContract::initialize(env.clone(), admin);
            assert!(result.is_ok(), "Initialize should succeed");

            let stored_admin = UpgradeContract::admin(env.clone()).unwrap();
            assert_eq!(stored_admin, admin, "Admin should be stored");

            let paused = UpgradeContract::is_paused(env.clone());
            assert!(!paused, "Contract should be unpaused after init");

            let current_hash = UpgradeContract::current_wasm_hash(env.clone());
            assert_eq!(current_hash, BytesN::from_array(&env, &[0u8; 32]), "Default hash should be zero");
        });
    }

    /// Test admin can pause the contract
    #[test]
    fn test_pause() {
        let env = Env::default();
        let admin = Address::generate(&env);

        let contract_id = env.register_contract(None, UpgradeContract);
        env.as_contract(&contract_id, || {
            // Initialize
            UpgradeContract::initialize(env.clone(), admin);

            // Pause by admin
            let result = UpgradeContract::pause(env.clone(), admin);
            assert!(result.is_ok(), "Admin should be able to pause");

            let paused = UpgradeContract::is_paused(env.clone());
            assert!(paused, "Contract should be paused");
        });
    }

    /// Test admin cannot pause when not initialized
    #[test]
    fn test_pause_not_initialized() {
        let env = Env::default();
        let admin = Address::generate(&env);

        let contract_id = env.register_contract(None, UpgradeContract);
        env.as_contract(&contract_id, || {
            let result = UpgradeContract::pause(env.clone(), admin);
            assert!(result.is_err(), "Should fail when not initialized");
            match result.unwrap_err() {
                ContractError::AdminNotSet => (),
                _ => panic!("Should return AdminNotSet"),
            }
        });
    }

    /// Test non-admin cannot pause
    #[test]
    fn test_non_admin_cannot_pause() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let unauthorized = Address::generate(&env);

        let contract_id = env.register_contract(None, UpgradeContract);
        env.as_contract(&contract_id, || {
            UpgradeContract::initialize(env.clone(), admin);

            let result = UpgradeContract::pause(env.clone(), unauthorized);
            assert!(result.is_err(), "Non-admin should not be able to pause");
            match result.unwrap_err() {
                ContractError::Unauthorized => (),
                _ => panic!("Should return Unauthorized"),
            }
        });
    }

    /// Test admin can resume when paused
    #[test]
    fn test_resume_from_paused() {
        let env = Env::default();
        let admin = Address::generate(&env);

        let contract_id = env.register_contract(None, UpgradeContract);
        env.as_contract(&contract_id, || {
            UpgradeContract::initialize(env.clone(), admin);
            UpgradeContract::pause(env.clone(), admin);

            let result = UpgradeContract::resume(env.clone(), admin);
            assert!(result.is_ok(), "Admin should be able to resume");

            let paused = UpgradeContract::is_paused(env.clone());
            assert!(!paused, "Contract should be active after resume");
        });
    }

    /// Test non-admin cannot resume
    #[test]
    fn test_non_admin_cannot_resume() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let unauthorized = Address::generate(&env);

        let contract_id = env.register_contract(None, UpgradeContract);
        env.as_contract(&contract_id, || {
            UpgradeContract::initialize(env.clone(), admin);

            let result = UpgradeContract::resume(env.clone(), unauthorized);
            assert!(result.is_err(), "Non-admin should not be able to resume");
            match result.unwrap_err() {
                ContractError::Unauthorized => (),
                _ => panic!("Should return Unauthorized"),
            }
        });
    }

    /// Test state-changing operation succeeds when active
    #[test]
    fn test_upgrade_wasm_active() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let new_hash = BytesN::from_array(&env, &[1u8; 32]);

        let contract_id = env.register_contract(None, UpgradeContract);
        env.as_contract(&contract_id, || {
            UpgradeContract::initialize(env.clone(), admin);

            let result = UpgradeContract::upgrade_wasm(env.clone(), admin, new_hash.clone());
            assert!(result.is_ok(), "Upgrade should succeed when active");

            let stored_hash = UpgradeContract::current_wasm_hash(env.clone());
            assert_eq!(stored_hash, new_hash, "Upgrade should store new hash");
        });
    }

    /// Test state-changing operation is blocked when paused
    #[test]
    fn test_upgrade_wasm_paused() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let new_hash = BytesN::from_array(&env, &[1u8; 32]);

        let contract_id = env.register_contract(None, UpgradeContract);
        env.as_contract(&contract_id, || {
            UpgradeContract::initialize(env.clone(), admin);
            UpgradeContract::pause(env.clone(), admin);

            let result = UpgradeContract::upgrade_wasm(env.clone(), admin, new_hash);
            assert!(result.is_err(), "Upgrade should be blocked when paused");
            match result.unwrap_err() {
                ContractError::ContractPaused => (),
                _ => panic!("Should return ContractPaused"),
            }
        });
    }

    /// Test state-changing operation succeeds after resume
    #[test]
    fn test_upgrade_wasm_after_resume() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let initial_hash = BytesN::from_array(&env, &[1u8; 32]);
        let new_hash = BytesN::from_array(&env, &[2u8; 32]);

        let contract_id = env.register_contract(None, UpgradeContract);
        env.as_contract(&contract_id, || {
            UpgradeContract::initialize(env.clone(), admin);
            UpgradeContract::upgrade_wasm(env.clone(), admin, initial_hash);
            UpgradeContract::pause(env.clone(), admin);

            let result = UpgradeContract::resume(env.clone(), admin);
            assert!(result.is_ok(), "Resume should succeed");

            let upgrade_result = UpgradeContract::upgrade_wasm(env.clone(), admin, new_hash.clone());
            assert!(upgrade_result.is_ok(), "Upgrade should succeed after resume");

            let stored_hash = UpgradeContract::current_wasm_hash(env.clone());
            assert_eq!(stored_hash, new_hash, "Upgrade should store new hash");
        });
    }

    /// Test rollback succeeds when active
    #[test]
    fn test_rollback_active() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let initial_hash = BytesN::from_array(&env, &[1u8; 32]);
        let rollback_hash = BytesN::from_array(&env, &[2u8; 32]);

        let contract_id = env.register_contract(None, UpgradeContract);
        env.as_contract(&contract_id, || {
            UpgradeContract::initialize(env.clone(), admin);
            UpgradeContract::upgrade_wasm(env.clone(), admin, initial_hash);

            let result = UpgradeContract::rollback(env.clone(), admin, rollback_hash.clone());
            assert!(result.is_ok(), "Rollback should succeed when active");

            let stored_hash = UpgradeContract::current_wasm_hash(env.clone());
            assert_eq!(stored_hash, rollback_hash, "Rollback should set correct hash");
        });
    }

    /// Test rollback is blocked when paused
    #[test]
    fn test_rollback_paused() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let rollback_hash = BytesN::from_array(&env, &[1u8; 32]);

        let contract_id = env.register_contract(None, UpgradeContract);
        env.as_contract(&contract_id, || {
            UpgradeContract::initialize(env.clone(), admin);
            UpgradeContract::pause(env.clone(), admin);

            let result = UpgradeContract::rollback(env.clone(), admin, rollback_hash);
            assert!(result.is_err(), "Rollback should be blocked when paused");
            match result.unwrap_err() {
                ContractError::ContractPaused => (),
                _ => panic!("Should return ContractPaused"),
            }
        });
    }

    /// Test read-only methods continue working when paused
    #[test]
    fn test_read_only_while_paused() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let hash = BytesN::from_array(&env, &[1u8; 32]);

        let contract_id = env.register_contract(None, UpgradeContract);
        env.as_contract(&contract_id, || {
            UpgradeContract::initialize(env.clone(), admin);
            UpgradeContract::upgrade_wasm(env.clone(), admin, hash);
            UpgradeContract::pause(env.clone(), admin);

            let admin_result = UpgradeContract::admin(env.clone());
            assert!(admin_result.is_ok(), "Admin should be readable while paused");

            let paused_result = UpgradeContract::is_paused(env.clone());
            assert!(paused_result, "is_paused should work while paused");

            let hash_result = UpgradeContract::current_wasm_hash(env.clone());
            assert_eq!(hash_result, hash, "Current hash should be readable while paused");
        });
    }

    /// Test initial state is unpaused
    #[test]
    fn test_initial_state() {
        let env = Env::default();
        let admin = Address::generate(&env);

        let contract_id = env.register_contract(None, UpgradeContract);
        env.as_contract(&contract_id, || {
            UpgradeContract::initialize(env.clone(), admin);

            let paused = UpgradeContract::is_paused(env.clone());
            assert!(!paused, "Initial state should be unpaused");
        });
    }

    /// Test multiple pause calls are safe
    #[test]
    fn test_multiple_pause() {
        let env = Env::default();
        let admin = Address::generate(&env);

        let contract_id = env.register_contract(None, UpgradeContract);
        env.as_contract(&contract_id, || {
            UpgradeContract::initialize(env.clone(), admin);
            UpgradeContract::pause(env.clone(), admin);

            let result1 = UpgradeContract::pause(env.clone(), admin);
            assert!(result1.is_ok(), "Second pause should succeed");

            let result2 = UpgradeContract::pause(env.clone(), admin);
            assert!(result2.is_ok(), "Third pause should also succeed");

            let paused = UpgradeContract::is_paused(env.clone());
            assert!(paused, "Contract should remain paused");
        });
    }

    /// Test multiple resume calls are safe
    #[test]
    fn test_multiple_resume() {
        let env = Env::default();
        let admin = Address::generate(&env);

        let contract_id = env.register_contract(None, UpgradeContract);
        env.as_contract(&contract_id, || {
            UpgradeContract::initialize(env.clone(), admin);
            UpgradeContract::pause(env.clone(), admin);
            UpgradeContract::resume(env.clone(), admin);

            let result1 = UpgradeContract::resume(env.clone(), admin);
            assert!(result1.is_ok(), "Resume after active should succeed");

            let result2 = UpgradeContract::resume(env.clone(), admin);
            assert!(result2.is_ok(), "Second resume should also succeed");

            let paused = UpgradeContract::is_paused(env.clone());
            assert!(!paused, "Contract should remain active");
        });
    }
}
