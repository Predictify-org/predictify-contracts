#![no_std]

use soroban_sdk::{contract, contractimpl, Env};

pub mod admin;

#[contract]
pub struct DisputesContract;

#[contractimpl]
impl DisputesContract {
    pub fn version(_env: Env) -> u32 {
        7
    }

    /// Return the admin address, or `None` if not yet set.
    pub fn admin(env: Env) -> Option<soroban_sdk::Address> {
        admin::get_admin(&env)
    }

    /// Set the cooldown period for critical admin actions (seconds).
    /// Requires admin auth.
    pub fn set_cooldown(env: Env, caller: soroban_sdk::Address, cooldown_secs: u64) {
        admin::set_cooldown_period(&env, &caller, cooldown_secs);
    }

    /// Record a critical admin action (updates the cooldown clock).
    /// Requires admin auth.
    pub fn record_action(env: Env, caller: soroban_sdk::Address) {
        admin::require_admin(&env, &caller);
        admin::record_admin_action(&env);
    }
}
