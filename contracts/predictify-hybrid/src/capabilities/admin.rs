//! Cooldown enforcement for admin actions that can change contract capabilities.

use soroban_sdk::{contracttype, Env};

use crate::Error;

/// Cooldown applied between repeated capability-critical admin actions.
///
/// Upgrades and rollbacks have independent clocks so a newly deployed Wasm can
/// still be rolled back immediately if it proves unsafe.
pub const ADMIN_ACTION_COOLDOWN_SECONDS: u64 = 3_600;

const COOLDOWN_TTL_THRESHOLD: u32 = 535_680;
const COOLDOWN_TTL_BUMP: u32 = 535_680;

/// Capability-critical operations protected by the admin cooldown.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum CapabilitiesAdminAction {
    /// Replace the active contract Wasm.
    Upgrade,
    /// Restore a prior contract Wasm.
    Rollback,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
enum CapabilitiesAdminDataKey {
    LastAction(CapabilitiesAdminAction),
}

/// Enforces cooldowns for operations that can change the capability bitmap.
pub(crate) struct CapabilitiesAdminCooldown;

impl CapabilitiesAdminCooldown {
    /// Checks whether the named critical action can execute at the current time.
    ///
    /// This function does not update storage. Call [`Self::record_action`] only
    /// after the protected operation succeeds so failed attempts do not consume
    /// the cooldown window.
    ///
    /// # Errors
    ///
    /// Returns [`Error::AdminActionTimelocked`] while the action's cooldown is
    /// active, or [`Error::Overflow`] if the stored timestamp cannot be safely
    /// combined with the cooldown duration.
    pub(crate) fn require_elapsed(
        env: &Env,
        action: &CapabilitiesAdminAction,
    ) -> Result<(), Error> {
        let key = CapabilitiesAdminDataKey::LastAction(action.clone());
        let Some(last_action) = env.storage().persistent().get::<_, u64>(&key) else {
            return Ok(());
        };

        let cooldown_end = last_action
            .checked_add(ADMIN_ACTION_COOLDOWN_SECONDS)
            .ok_or(Error::Overflow)?;

        if env.ledger().timestamp() < cooldown_end {
            return Err(Error::AdminActionTimelocked);
        }

        env.storage()
            .persistent()
            .extend_ttl(&key, COOLDOWN_TTL_THRESHOLD, COOLDOWN_TTL_BUMP);

        Ok(())
    }

    /// Records a successful capability-critical action.
    pub(crate) fn record_action(env: &Env, action: &CapabilitiesAdminAction) {
        let key = CapabilitiesAdminDataKey::LastAction(action.clone());
        env.storage()
            .persistent()
            .set(&key, &env.ledger().timestamp());
        env.storage()
            .persistent()
            .extend_ttl(&key, COOLDOWN_TTL_THRESHOLD, COOLDOWN_TTL_BUMP);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CapabilitiesAdminAction, CapabilitiesAdminCooldown, CapabilitiesAdminDataKey,
        ADMIN_ACTION_COOLDOWN_SECONDS,
    };
    use crate::{Error, PredictifyHybrid};
    use soroban_sdk::{testutils::Ledger, Env};

    fn in_contract(env: &Env, test: impl FnOnce()) {
        let contract_id = env.register(PredictifyHybrid, ());
        env.as_contract(&contract_id, test);
    }

    #[test]
    fn first_action_is_allowed_and_repeat_is_blocked() {
        let env = Env::default();
        env.ledger().set_timestamp(1_000);

        in_contract(&env, || {
            let action = CapabilitiesAdminAction::Upgrade;
            assert_eq!(
                CapabilitiesAdminCooldown::require_elapsed(&env, &action),
                Ok(())
            );

            CapabilitiesAdminCooldown::record_action(&env, &action);

            assert_eq!(
                CapabilitiesAdminCooldown::require_elapsed(&env, &action),
                Err(Error::AdminActionTimelocked)
            );
        });
    }

    #[test]
    fn action_is_allowed_at_exact_cooldown_boundary() {
        let env = Env::default();
        env.ledger().set_timestamp(5_000);

        in_contract(&env, || {
            let action = CapabilitiesAdminAction::Upgrade;
            CapabilitiesAdminCooldown::record_action(&env, &action);

            env.ledger()
                .set_timestamp(5_000 + ADMIN_ACTION_COOLDOWN_SECONDS);

            assert_eq!(
                CapabilitiesAdminCooldown::require_elapsed(&env, &action),
                Ok(())
            );
        });
    }

    #[test]
    fn upgrade_and_rollback_use_independent_cooldowns() {
        let env = Env::default();
        env.ledger().set_timestamp(10_000);

        in_contract(&env, || {
            CapabilitiesAdminCooldown::record_action(&env, &CapabilitiesAdminAction::Upgrade);

            assert_eq!(
                CapabilitiesAdminCooldown::require_elapsed(&env, &CapabilitiesAdminAction::Upgrade,),
                Err(Error::AdminActionTimelocked)
            );
            assert_eq!(
                CapabilitiesAdminCooldown::require_elapsed(
                    &env,
                    &CapabilitiesAdminAction::Rollback,
                ),
                Ok(())
            );
        });
    }

    #[test]
    fn timestamp_overflow_returns_contract_error() {
        let env = Env::default();

        in_contract(&env, || {
            let action = CapabilitiesAdminAction::Upgrade;
            let key = CapabilitiesAdminDataKey::LastAction(action.clone());
            env.storage().persistent().set(&key, &u64::MAX);

            assert_eq!(
                CapabilitiesAdminCooldown::require_elapsed(&env, &action),
                Err(Error::Overflow)
            );
        });
    }
}
