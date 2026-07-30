//! Token account-state registry with bounded per-account storage.
//!
//! The contract tracks identifiers for account-owned bets, positions, and
//! subscriptions. Each category has an administrator-configured cap that is
//! enforced independently for every account.
//!
//! All state-changing entrypoints require authentication from the acting
//! address. Read-only views are public.

#![no_std]

pub mod events;
mod limits;

pub use limits::{
    AccountLimits, AccountStateKind, AccountUsage, TokenLimitError, MAX_CONFIGURABLE_ACCOUNT_LIMIT,
};

use soroban_sdk::{contract, contractimpl, Address, BytesN, Env};

/// Token account-state contract.
#[contract]
pub struct TokensContract;

#[contractimpl]
impl TokensContract {
    /// Initializes the contract administrator and global per-account limits.
    ///
    /// The limits apply independently to every account. A zero limit disables
    /// new entries for that category.
    ///
    /// # Authorization
    ///
    /// `admin` must authorize this invocation.
    ///
    /// # Errors
    ///
    /// - [`TokenLimitError::AlreadyInitialized`] if initialization already ran.
    /// - [`TokenLimitError::InvalidLimit`] if any configured cap exceeds
    ///   [`MAX_CONFIGURABLE_ACCOUNT_LIMIT`].
    pub fn initialize(
        env: Env,
        admin: Address,
        account_limits: AccountLimits,
    ) -> Result<(), TokenLimitError> {
        admin.require_auth();
        limits::initialize(&env, &admin, &account_limits)?;
        events::emit_initialized(&env, &admin, &account_limits);
        Ok(())
    }

    /// Replaces the global limits applied independently to every account.
    ///
    /// Existing entries are never deleted when a cap is lowered. An account
    /// already above a new cap cannot track another item in that category until
    /// enough existing items are removed.
    ///
    /// # Authorization
    ///
    /// `admin` must authorize this invocation and match the stored
    /// administrator.
    ///
    /// # Errors
    ///
    /// - [`TokenLimitError::NotInitialized`] if the contract is not initialized.
    /// - [`TokenLimitError::Unauthorized`] if `admin` is not the administrator.
    /// - [`TokenLimitError::InvalidLimit`] if any configured cap exceeds
    ///   [`MAX_CONFIGURABLE_ACCOUNT_LIMIT`].
    pub fn set_account_limits(
        env: Env,
        admin: Address,
        account_limits: AccountLimits,
    ) -> Result<(), TokenLimitError> {
        admin.require_auth();
        limits::set_account_limits(&env, &admin, &account_limits)?;
        events::emit_limits_set(&env, &admin, &account_limits);
        Ok(())
    }

    /// Tracks one item for an account after enforcing its category cap.
    ///
    /// The `(account, kind, item_id)` tuple is unique. Replaying the same item
    /// cannot consume additional capacity.
    ///
    /// # Authorization
    ///
    /// `account` must authorize this invocation.
    ///
    /// # Errors
    ///
    /// - [`TokenLimitError::NotInitialized`] if the contract is not initialized.
    /// - [`TokenLimitError::ItemAlreadyTracked`] if the item is already present.
    /// - [`TokenLimitError::BetLimitExceeded`] if the bet cap is reached.
    /// - [`TokenLimitError::PositionLimitExceeded`] if the position cap is reached.
    /// - [`TokenLimitError::SubscriptionLimitExceeded`] if the subscription cap is reached.
    /// - [`TokenLimitError::Overflow`] if the category counter cannot be incremented.
    pub fn track_account_item(
        env: Env,
        account: Address,
        kind: AccountStateKind,
        item_id: BytesN<32>,
    ) -> Result<AccountUsage, TokenLimitError> {
        account.require_auth();
        let usage = limits::track_account_item(&env, &account, &kind, &item_id)?;
        events::emit_item_tracked(&env, &account, kind, usage);
        Ok(usage)
    }

    /// Removes one tracked item and releases its occupied capacity.
    ///
    /// Capacity is released only when the exact stored item exists, preventing
    /// callers from decrementing counters with fabricated identifiers.
    ///
    /// # Authorization
    ///
    /// `account` must authorize this invocation.
    ///
    /// # Errors
    ///
    /// - [`TokenLimitError::NotInitialized`] if the contract is not initialized.
    /// - [`TokenLimitError::ItemNotFound`] if the exact item is not tracked.
    /// - [`TokenLimitError::Underflow`] if the membership length cannot be decremented.
    pub fn untrack_account_item(
        env: Env,
        account: Address,
        kind: AccountStateKind,
        item_id: BytesN<32>,
    ) -> Result<AccountUsage, TokenLimitError> {
        account.require_auth();
        let usage = limits::untrack_account_item(&env, &account, &kind, &item_id)?;
        events::emit_item_untracked(&env, &account, kind, usage);
        Ok(usage)
    }

    /// Returns the configured global per-account limits.
    ///
    /// # Errors
    ///
    /// Returns [`TokenLimitError::NotInitialized`] when no limits are configured.
    pub fn get_account_limits(env: Env) -> Result<AccountLimits, TokenLimitError> {
        limits::get_account_limits(&env)
    }

    /// Returns the number of tracked items in each category for `account`.
    pub fn get_account_usage(env: Env, account: Address) -> AccountUsage {
        limits::get_account_usage(&env, &account)
    }

    /// Returns the remaining capacity in each category for `account`.
    ///
    /// If an administrator lowered a cap below current usage, the corresponding
    /// remaining value is clamped to zero.
    ///
    /// # Errors
    ///
    /// Returns [`TokenLimitError::NotInitialized`] when no limits are configured.
    pub fn get_remaining_capacity(
        env: Env,
        account: Address,
    ) -> Result<AccountLimits, TokenLimitError> {
        limits::get_remaining_capacity(&env, &account)
    }

    /// Returns whether the exact account item is currently tracked.
    pub fn is_account_item_tracked(
        env: Env,
        account: Address,
        kind: AccountStateKind,
        item_id: BytesN<32>,
    ) -> bool {
        limits::is_account_item_tracked(&env, &account, &kind, &item_id)
    }

    /// Returns the initialized administrator.
    ///
    /// # Errors
    ///
    /// Returns [`TokenLimitError::NotInitialized`] before initialization.
    pub fn get_admin(env: Env) -> Result<Address, TokenLimitError> {
        limits::get_admin(&env)
    }
}
