#![no_std]

//! Version migration contract.
//!
//! The contract keeps a single administrator and monotonically increasing
//! version number. A migration must name the version it expects to replace;
//! this compare-and-set rule prevents stale operators from overwriting a
//! newer migration.

use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, Address, Env,
};

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Admin,
    Version,
}

/// Errors returned by migration entrypoints.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ContractError {
    /// The contract has already been initialized.
    AlreadyInitialized = 1,
    /// The contract has not been initialized.
    NotInitialized = 2,
    /// The authenticated caller is not the migration administrator.
    Unauthorized = 3,
    /// The target version is not strictly greater than the current version.
    InvalidTargetVersion = 4,
    /// The supplied expected version does not match the stored version.
    VersionMismatch = 5,
}

/// Emitted after migration state is initialized.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Initialized {
    /// Administrator authorized to perform migrations.
    #[topic]
    pub admin: Address,
    /// First active version.
    pub initial_version: u32,
}

/// Emitted after a version migration succeeds.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Migrated {
    /// Administrator that authorized the migration.
    #[topic]
    pub admin: Address,
    /// Version replaced by this migration.
    pub previous_version: u32,
    /// Version activated by this migration.
    pub target_version: u32,
}

#[contract]
pub struct MigrateContract;

#[contractimpl]
impl MigrateContract {
    /// Initializes migration state with an administrator and starting version.
    ///
    /// The administrator must authorize the call. Re-initialization returns
    /// [`ContractError::AlreadyInitialized`] and leaves state unchanged.
    pub fn initialize(env: Env, admin: Address, initial_version: u32) -> Result<(), ContractError> {
        admin.require_auth();

        if env.storage().instance().has(&DataKey::Admin) {
            return Err(ContractError::AlreadyInitialized);
        }

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::Version, &initial_version);
        Initialized {
            admin,
            initial_version,
        }
        .publish(&env);
        Ok(())
    }

    /// Advances the stored version using an authenticated compare-and-set.
    ///
    /// `expected_version` must equal the stored version and `target_version`
    /// must be strictly greater. These checks prevent stale, duplicate, and
    /// downgrade migrations. Successful calls emit a `migrated` event whose
    /// data is `(previous_version, target_version)`.
    pub fn migrate(
        env: Env,
        admin: Address,
        expected_version: u32,
        target_version: u32,
    ) -> Result<(), ContractError> {
        admin.require_auth();

        let stored_admin = Self::load_admin(&env)?;
        if admin != stored_admin {
            return Err(ContractError::Unauthorized);
        }

        let current_version = Self::load_version(&env)?;
        if expected_version != current_version {
            return Err(ContractError::VersionMismatch);
        }
        if target_version <= current_version {
            return Err(ContractError::InvalidTargetVersion);
        }

        env.storage()
            .instance()
            .set(&DataKey::Version, &target_version);
        Migrated {
            admin,
            previous_version: current_version,
            target_version,
        }
        .publish(&env);
        Ok(())
    }

    /// Returns the configured migration administrator.
    pub fn admin(env: Env) -> Result<Address, ContractError> {
        Self::load_admin(&env)
    }

    /// Returns the current stored version.
    pub fn current_version(env: Env) -> Result<u32, ContractError> {
        Self::load_version(&env)
    }

    fn load_admin(env: &Env) -> Result<Address, ContractError> {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(ContractError::NotInitialized)
    }

    fn load_version(env: &Env) -> Result<u32, ContractError> {
        env.storage()
            .instance()
            .get(&DataKey::Version)
            .ok_or(ContractError::NotInitialized)
    }
}
