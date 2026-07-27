use soroban_sdk::{contracttype, Address, Env};

use crate::err::Error;
use crate::types::Market;

/// Per-market timelock state for admin actions.
///
/// A zero delay means the market does not enforce a cooldown before the next
/// admin action. A non-zero delay blocks the next action until the configured
/// window has elapsed since the last action or the last configuration change.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketTimelockConfig {
    pub delay_seconds: u64,
    pub last_admin_action_at: u64,
}

impl Default for MarketTimelockConfig {
    fn default() -> Self {
        Self {
            delay_seconds: 0,
            last_admin_action_at: 0,
        }
    }
}

/// Utility for enforcing per-market timelocks on administrative actions.
pub struct MarketTimelockManager;

impl MarketTimelockManager {
    /// Configure the cooldown for admin actions on the specified market.
    ///
    /// The caller must be either the market administrator or the primary contract
    /// administrator. The configuration takes effect immediately by setting the
    /// initial timestamp for the next action.
    pub fn configure(
        env: &Env,
        market: &mut Market,
        caller: &Address,
        contract_admin: &Address,
        delay_seconds: u64,
    ) -> Result<(), Error> {
        if market.admin != *caller && contract_admin != caller {
            return Err(Error::Unauthorized);
        }

        market.timelock_config.delay_seconds = delay_seconds;
        market.timelock_config.last_admin_action_at = env.ledger().timestamp();
        Ok(())
    }

    /// Check whether a market admin action is currently allowed.
    ///
    /// If a non-zero timelock is configured, the action is rejected until the
    /// configured interval has passed since the last recorded admin action or
    /// configuration change. On success, the timestamp is refreshed so the next
    /// call must wait again.
    pub fn ensure_admin_action_allowed(
        env: &Env,
        market: &mut Market,
        caller: &Address,
        contract_admin: &Address,
    ) -> Result<(), Error> {
        if market.admin != *caller && contract_admin != caller {
            return Err(Error::Unauthorized);
        }

        let delay = market.timelock_config.delay_seconds;
        let now = env.ledger().timestamp();
        let last_action_at = market.timelock_config.last_admin_action_at;

        if delay > 0 && last_action_at > 0 && now < last_action_at.saturating_add(delay) {
            return Err(Error::AdminActionTimelocked);
        }

        market.timelock_config.last_admin_action_at = now;
        Ok(())
    }
}
