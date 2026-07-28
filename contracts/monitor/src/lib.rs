//! # Monitor contract — per-account state caps
//!
//! The Monitor contract enforces **per-account hard caps** on three classes
//! of on-chain activity within the Predictify platform:
//!
//! | Resource        | Default cap | Storage type   |
//! |-----------------|-------------|----------------|
//! | Active bets     | 50          | Persistent     |
//! | Active positions| 20          | Persistent     |
//! | Subscriptions   | 10          | Persistent     |
//!
//! Caps can be updated by the admin (subject to a hard upper bound of
//! `10 000`) without redeploying. Each cap update is reflected in
//! on-chain events for off-chain indexers.
//!
//! # Authorization Matrix
//!
//! | Entrypoint              | Required Auth  |
//! |-------------------------|----------------|
//! | `initialize`            | `admin`        |
//! | `set_caps`              | `admin`        |
//! | `record_bet`            | `user`         |
//! | `record_position`       | `user`         |
//! | `record_subscription`   | `user`         |
//! | `remove_bet`            | `user`         |
//! | `remove_position`       | `user`         |
//! | `remove_subscription`   | `user`         |
//! | `get_account_state`     | none (view)    |
//! | `get_caps`              | none (view)    |
//! | `version`               | none (view)    |
//!
//! # Overflow Safety
//!
//! All arithmetic uses `checked_add` / `checked_sub`. The contract will
//! return [`MonitorError::Overflow`] or [`MonitorError::Underflow`] instead
//! of panicking. `unwrap()` is never used in production paths.
//!
//! # Events
//!
//! Every state transition emits a typed event. See the [`events`] module for
//! the full topic registry.

#![no_std]

mod errors;
mod events;
mod limits;
mod storage;

pub use errors::MonitorError;
pub use limits::{AccountState, CapType, Caps};
pub use storage::DataKey;

use soroban_sdk::{contract, contractimpl, Address, Env, Symbol};

/// The Monitor contract.
///
/// Deploy this contract alongside the core Predictify contracts to enforce
/// per-account state caps on bets, positions, and subscriptions.
#[contract]
pub struct MonitorContract;

#[contractimpl]
impl MonitorContract {
    // -----------------------------------------------------------------------
    // Initialization
    // -----------------------------------------------------------------------

    /// Initialize the contract with an `admin` address.
    ///
    /// Must be called exactly once before any other state-changing entrypoint.
    /// Stores `admin` as the privileged actor for cap management. Emits a
    /// `mon_init` event.
    ///
    /// # Authorization
    ///
    /// `admin.require_auth()` is enforced.
    ///
    /// # Errors
    ///
    /// - [`MonitorError::AlreadyInitialized`] — called a second time.
    pub fn initialize(env: Env, admin: Address) -> Result<(), MonitorError> {
        admin.require_auth();

        if env.storage().instance().has(&DataKey::Initialized) {
            return Err(MonitorError::AlreadyInitialized);
        }

        env.storage().instance().set(&DataKey::Initialized, &true);
        env.storage().instance().set(&DataKey::Admin, &admin);

        events::emit_initialized(&env, &admin);

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Cap management
    // -----------------------------------------------------------------------

    /// Update the per-account cap for the resource identified by `cap_type`.
    ///
    /// The new value must be in the range `1 ..= HARD_UPPER_BOUND`.
    ///
    /// Emits a `mon_caps_set` event.
    ///
    /// # Authorization
    ///
    /// `admin.require_auth()` is enforced; the caller must also be the
    /// registered admin.
    ///
    /// # Errors
    ///
    /// - [`MonitorError::NotInitialized`] — contract not yet initialized.
    /// - [`MonitorError::Unauthorized`] — caller is not the registered admin.
    /// - [`MonitorError::InvalidInput`] — `new_max` is 0 or exceeds
    ///   [`limits::HARD_UPPER_BOUND`].
    pub fn set_caps(
        env: Env,
        admin: Address,
        cap_type: CapType,
        new_max: u32,
    ) -> Result<(), MonitorError> {
        admin.require_auth();
        Self::require_initialized(&env)?;
        Self::require_admin(&env, &admin)?;

        if new_max == 0 || new_max > limits::HARD_UPPER_BOUND {
            return Err(MonitorError::InvalidInput);
        }

        let (storage_key, topic_str) = match cap_type {
            CapType::Bets => (DataKey::MaxBets, "bets"),
            CapType::Positions => (DataKey::MaxPositions, "positions"),
            CapType::Subscriptions => (DataKey::MaxSubscriptions, "subscriptions"),
        };

        env.storage().instance().set(&storage_key, &new_max);

        let cap_sym = Symbol::new(&env, topic_str);
        events::emit_caps_set(&env, &admin, &cap_sym, new_max);

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Record entrypoints
    // -----------------------------------------------------------------------

    /// Record one additional active bet for `user`.
    ///
    /// Checks the bet cap before incrementing. Emits `mon_bet_rec`.
    ///
    /// # Authorization
    ///
    /// `user.require_auth()` is enforced.
    ///
    /// # Errors
    ///
    /// - [`MonitorError::NotInitialized`]
    /// - [`MonitorError::BetCapExceeded`] — cap already reached.
    /// - [`MonitorError::Overflow`] — internal counter overflow (unreachable in practice).
    pub fn record_bet(env: Env, user: Address) -> Result<u32, MonitorError> {
        user.require_auth();
        Self::require_initialized(&env)?;

        let new_count = limits::increment_bets(&env, &user)?;
        events::emit_bet_recorded(&env, &user, new_count);

        Ok(new_count)
    }

    /// Record one additional active position for `user`.
    ///
    /// Checks the position cap before incrementing. Emits `mon_pos_rec`.
    ///
    /// # Authorization
    ///
    /// `user.require_auth()` is enforced.
    ///
    /// # Errors
    ///
    /// - [`MonitorError::NotInitialized`]
    /// - [`MonitorError::PositionCapExceeded`] — cap already reached.
    /// - [`MonitorError::Overflow`] — internal counter overflow (unreachable in practice).
    pub fn record_position(env: Env, user: Address) -> Result<u32, MonitorError> {
        user.require_auth();
        Self::require_initialized(&env)?;

        let new_count = limits::increment_positions(&env, &user)?;
        events::emit_position_recorded(&env, &user, new_count);

        Ok(new_count)
    }

    /// Record one additional active subscription for `user`.
    ///
    /// Checks the subscription cap before incrementing. Emits `mon_sub_rec`.
    ///
    /// # Authorization
    ///
    /// `user.require_auth()` is enforced.
    ///
    /// # Errors
    ///
    /// - [`MonitorError::NotInitialized`]
    /// - [`MonitorError::SubscriptionCapExceeded`] — cap already reached.
    /// - [`MonitorError::Overflow`] — internal counter overflow (unreachable in practice).
    pub fn record_subscription(env: Env, user: Address) -> Result<u32, MonitorError> {
        user.require_auth();
        Self::require_initialized(&env)?;

        let new_count = limits::increment_subscriptions(&env, &user)?;
        events::emit_subscription_recorded(&env, &user, new_count);

        Ok(new_count)
    }

    // -----------------------------------------------------------------------
    // Remove entrypoints
    // -----------------------------------------------------------------------

    /// Remove one active bet from `user`'s count.
    ///
    /// Should be called when a bet is resolved or cancelled. Emits `mon_bet_rem`.
    ///
    /// # Authorization
    ///
    /// `user.require_auth()` is enforced.
    ///
    /// # Errors
    ///
    /// - [`MonitorError::NotInitialized`]
    /// - [`MonitorError::Underflow`] — count is already zero.
    pub fn remove_bet(env: Env, user: Address) -> Result<u32, MonitorError> {
        user.require_auth();
        Self::require_initialized(&env)?;

        let new_count = limits::decrement_bets(&env, &user)?;
        events::emit_bet_removed(&env, &user, new_count);

        Ok(new_count)
    }

    /// Remove one active position from `user`'s count.
    ///
    /// Should be called when a position is closed. Emits `mon_pos_rem`.
    ///
    /// # Authorization
    ///
    /// `user.require_auth()` is enforced.
    ///
    /// # Errors
    ///
    /// - [`MonitorError::NotInitialized`]
    /// - [`MonitorError::Underflow`] — count is already zero.
    pub fn remove_position(env: Env, user: Address) -> Result<u32, MonitorError> {
        user.require_auth();
        Self::require_initialized(&env)?;

        let new_count = limits::decrement_positions(&env, &user)?;
        events::emit_position_removed(&env, &user, new_count);

        Ok(new_count)
    }

    /// Remove one active subscription from `user`'s count.
    ///
    /// Should be called when a subscription is cancelled. Emits `mon_sub_rem`.
    ///
    /// # Authorization
    ///
    /// `user.require_auth()` is enforced.
    ///
    /// # Errors
    ///
    /// - [`MonitorError::NotInitialized`]
    /// - [`MonitorError::Underflow`] — count is already zero.
    pub fn remove_subscription(env: Env, user: Address) -> Result<u32, MonitorError> {
        user.require_auth();
        Self::require_initialized(&env)?;

        let new_count = limits::decrement_subscriptions(&env, &user)?;
        events::emit_subscription_removed(&env, &user, new_count);

        Ok(new_count)
    }

    // -----------------------------------------------------------------------
    // Read-only queries
    // -----------------------------------------------------------------------

    /// Return the current state counts for `user`.
    ///
    /// No authentication required; counts are public ledger state.
    ///
    /// Returns an [`AccountState`] with all counts as zero if the user has no
    /// recorded activity.
    pub fn get_account_state(env: Env, user: Address) -> AccountState {
        limits::load_account_state(&env, &user)
    }

    /// Return the currently active per-account caps.
    ///
    /// No authentication required.
    pub fn get_caps(env: Env) -> Caps {
        limits::load_caps(&env)
    }

    /// Return the contract's semantic version number.
    ///
    /// No authentication required.
    pub fn version(_env: Env) -> u32 {
        1
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    /// Return `Err(NotInitialized)` if the contract has not been initialized.
    fn require_initialized(env: &Env) -> Result<(), MonitorError> {
        if !env.storage().instance().has(&DataKey::Initialized) {
            return Err(MonitorError::NotInitialized);
        }
        Ok(())
    }

    /// Return `Err(Unauthorized)` if `caller` is not the registered admin.
    fn require_admin(env: &Env, caller: &Address) -> Result<(), MonitorError> {
        let stored: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(MonitorError::NotInitialized)?;
        if caller != &stored {
            return Err(MonitorError::Unauthorized);
        }
        Ok(())
    }
}
