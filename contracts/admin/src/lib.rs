#![no_std]

//! Admin contract with gas-efficient entrypoints.
//!
//! Provides core admin functionality with per-entrypoint gas regression testing.

use soroban_sdk::{contract, contracterror, contractimpl, contracttype, Address, Env, Symbol};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    AdminCooldownSeconds,
    AdminLastAction(Symbol),
    Admin,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum ContractError {
    AlreadyInitialized = 1,
    AdminNotSet = 2,
    Unauthorized = 3,
    AdminActionTimelocked = 4,
}

#[contract]
pub struct AdminContract;

#[contractimpl]
impl AdminContract {
    /// Initialize the admin contract with a primary administrator.
    pub fn initialize(env: Env, admin: Address) -> Result<(), ContractError> {
        admin.require_auth();

        if env.storage().instance().has(&DataKey::Admin) {
            return Err(ContractError::AlreadyInitialized);
        }

        env.storage().instance().set(&DataKey::Admin, &admin);
        Ok(())
    }

    /// Get the configured admin address.
    pub fn admin(env: Env) -> Result<Address, ContractError> {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(ContractError::AdminNotSet)
    }

    /// Set the cooldown period (in seconds) between admin actions.
    pub fn set_admin_cooldown(env: Env, admin: Address, seconds: u64) -> Result<(), ContractError> {
        admin.require_auth();

        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(ContractError::AdminNotSet)?;

        if admin != stored_admin {
            return Err(ContractError::Unauthorized);
        }

        env.storage()
            .persistent()
            .set(&DataKey::AdminCooldownSeconds, &seconds);
        Ok(())
    }

    /// Get the configured admin cooldown period in seconds.
    pub fn get_admin_cooldown(env: Env) -> u64 {
        env.storage()
            .persistent()
            .get(&DataKey::AdminCooldownSeconds)
            .unwrap_or(0)
    }

    /// Check and enforce admin cooldown for a specific function.
    pub fn check_admin_cooldown(
        env: Env,
        admin: Address,
        function_name: Symbol,
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

        let cooldown = Self::get_admin_cooldown(env.clone());
        if cooldown == 0 {
            return Ok(());
        }

        let now = env.ledger().timestamp();
        let last_key = DataKey::AdminLastAction(function_name);
        // `has()` distinguishes "never recorded" from "recorded at ledger
        // timestamp 0" -- a bare `last_action > 0` sentinel would wrongly
        // skip the cooldown check when the first action happens at the
        // default/initial ledger timestamp.
        if let Some(last_action) = env.storage().persistent().get::<_, u64>(&last_key) {
            if now < last_action.saturating_add(cooldown) {
                return Err(ContractError::AdminActionTimelocked);
            }
        }

        env.storage().persistent().set(&last_key, &now);
        Ok(())
    }
}
