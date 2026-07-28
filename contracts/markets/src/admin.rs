use soroban_sdk::{contracttype, Address, Env, Symbol};
use crate::errors::ContractError;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    AdminCooldownSeconds,
    AdminLastAction(Symbol),
}

/// Helper struct for managing admin cooldowns.
pub struct AdminManager;

impl AdminManager {
    /// Sets the cooldown period (in seconds) between admin actions on markets.
    ///
    /// A zero value disables the cooldown entirely.
    pub fn set_admin_cooldown(env: &Env, admin: &Address, seconds: u64) -> Result<(), ContractError> {
        admin.require_auth();
        let key = DataKey::AdminCooldownSeconds;
        env.storage().persistent().set(&key, &seconds);
        env.storage().persistent().extend_ttl(&key, 535_680, 535_680);
        Ok(())
    }

    /// Retrieves the configured market admin cooldown period in seconds.
    ///
    /// Returns 0 (no cooldown) when not configured.
    pub fn get_admin_cooldown(env: &Env) -> u64 {
        let key = DataKey::AdminCooldownSeconds;
        if let Some(result) = env.storage().persistent().get(&key) {
            env.storage().persistent().extend_ttl(&key, 535_680, 535_680);
            result
        } else {
            0
        }
    }

    /// Enforces the per-function admin cooldown for a named market operation.
    ///
    /// * `function_name` – a short identifier (`"set_market"`, `"pause_market"`, …).
    ///
    /// # Errors
    /// Returns `ContractError::AdminActionTimelocked` if the cooldown has not yet elapsed
    /// since the last invocation of *this specific* function.
    pub fn check_admin_cooldown(
        env: &Env,
        admin: &Address,
        function_name: &Symbol,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        let cooldown = Self::get_admin_cooldown(env);
        if cooldown == 0 {
            return Ok(());
        }
        let now = env.ledger().timestamp();
        let last_key = DataKey::AdminLastAction(function_name.clone());
        let last_action: u64 = if let Some(val) = env.storage().persistent().get(&last_key) {
            env.storage().persistent().extend_ttl(&last_key, 535_680, 535_680);
            val
        } else {
            0
        };
        
        if last_action > 0 && now < last_action.saturating_add(cooldown) {
            return Err(ContractError::AdminCooldownActive);
        }
        
        env.storage().persistent().set(&last_key, &now);
        env.storage().persistent().extend_ttl(&last_key, 535_680, 535_680);
        Ok(())
    }
}
