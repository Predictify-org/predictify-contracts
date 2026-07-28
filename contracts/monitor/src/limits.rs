//! Per-account state-cap definitions and enforcement logic.
//!
//! This module is the **heart** of the Monitor contract. It defines:
//!
//! - The hard-coded **default caps** used when no admin override is stored.
//! - The [`Caps`] and [`AccountState`] value types returned by read-only
//!   entrypoints.
//! - The [`CapType`] discriminant used by [`crate::MonitorContract::set_caps`]
//!   to select which cap to update.
//! - Pure **enforcement helpers** (`check_bet_cap`, `check_position_cap`,
//!   `check_subscription_cap`) that are called from every state-changing
//!   record entrypoint.
//! - Overflow-safe **counter helpers** (`increment`, `decrement`) that return
//!   typed errors instead of panicking.
//!
//! # Cap Storage Layout
//!
//! Caps are stored in **instance storage** so a single read fetches all of
//! them together.  Per-account counts are stored in **persistent storage**
//! keyed by `(DataKey::BetCount, user)`, etc., so each account's state
//! outlives the contract instance TTL.
//!
//! # Overflow Safety
//!
//! All arithmetic uses [`u32::checked_add`] / [`u32::checked_sub`].  The
//! sentinel error variants [`MonitorError::Overflow`] and
//! [`MonitorError::Underflow`] are returned instead of panicking.  Given the
//! small cap magnitudes (≤ `u32::MAX`) this path is effectively unreachable,
//! but the guards are present as a defence-in-depth measure.
//!
//! # No `unwrap()` in Production Paths
//!
//! Storage reads that have no sentinel default are guarded by `.unwrap_or`
//! with the corresponding constant default, not `.unwrap()`.

use soroban_sdk::{contracttype, Address, Env};

use crate::errors::MonitorError;
use crate::storage::DataKey;

// ---------------------------------------------------------------------------
// Default caps
// ---------------------------------------------------------------------------

/// Default maximum number of active bets per account.
///
/// Chosen to be generous enough for active users while bounding the worst-case
/// on-chain state footprint per account.
pub const DEFAULT_MAX_BETS: u32 = 50;

/// Default maximum number of active positions per account.
pub const DEFAULT_MAX_POSITIONS: u32 = 20;

/// Default maximum number of active subscriptions per account.
pub const DEFAULT_MAX_SUBSCRIPTIONS: u32 = 10;

/// Absolute upper bound an admin may set for any single cap.
///
/// Prevents an admin from accidentally removing limits entirely.
pub const HARD_UPPER_BOUND: u32 = 10_000;

// ---------------------------------------------------------------------------
// Value types
// ---------------------------------------------------------------------------

/// Identifies which per-account cap to read or update.
///
/// Used as the `cap_type` parameter of [`crate::MonitorContract::set_caps`].
#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum CapType {
    /// The cap on the number of active bets per account.
    Bets,
    /// The cap on the number of active positions per account.
    Positions,
    /// The cap on the number of active subscriptions per account.
    Subscriptions,
}

/// A snapshot of all current per-account caps.
///
/// Returned by [`crate::MonitorContract::get_caps`].
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Caps {
    /// Maximum active bets allowed per account.
    pub max_bets: u32,
    /// Maximum active positions allowed per account.
    pub max_positions: u32,
    /// Maximum active subscriptions allowed per account.
    pub max_subscriptions: u32,
}

/// A snapshot of an individual account's current state counts.
///
/// Returned by [`crate::MonitorContract::get_account_state`].
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountState {
    /// Number of active bets the account currently holds.
    pub bets: u32,
    /// Number of active positions the account currently holds.
    pub positions: u32,
    /// Number of active subscriptions the account currently holds.
    pub subscriptions: u32,
}

// ---------------------------------------------------------------------------
// Cap read helpers
// ---------------------------------------------------------------------------

/// Return the currently configured bet cap (falling back to [`DEFAULT_MAX_BETS`]).
pub fn get_max_bets(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get::<DataKey, u32>(&DataKey::MaxBets)
        .unwrap_or(DEFAULT_MAX_BETS)
}

/// Return the currently configured position cap (falling back to
/// [`DEFAULT_MAX_POSITIONS`]).
pub fn get_max_positions(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get::<DataKey, u32>(&DataKey::MaxPositions)
        .unwrap_or(DEFAULT_MAX_POSITIONS)
}

/// Return the currently configured subscription cap (falling back to
/// [`DEFAULT_MAX_SUBSCRIPTIONS`]).
pub fn get_max_subscriptions(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get::<DataKey, u32>(&DataKey::MaxSubscriptions)
        .unwrap_or(DEFAULT_MAX_SUBSCRIPTIONS)
}

/// Build a [`Caps`] snapshot from the current instance storage.
pub fn load_caps(env: &Env) -> Caps {
    Caps {
        max_bets: get_max_bets(env),
        max_positions: get_max_positions(env),
        max_subscriptions: get_max_subscriptions(env),
    }
}

// ---------------------------------------------------------------------------
// Account count helpers
// ---------------------------------------------------------------------------

/// Load the current bet count for `user` (defaults to 0).
pub fn get_bet_count(env: &Env, user: &Address) -> u32 {
    env.storage()
        .persistent()
        .get::<DataKey, u32>(&DataKey::BetCount(user.clone()))
        .unwrap_or(0)
}

/// Load the current position count for `user` (defaults to 0).
pub fn get_position_count(env: &Env, user: &Address) -> u32 {
    env.storage()
        .persistent()
        .get::<DataKey, u32>(&DataKey::PositionCount(user.clone()))
        .unwrap_or(0)
}

/// Load the current subscription count for `user` (defaults to 0).
pub fn get_subscription_count(env: &Env, user: &Address) -> u32 {
    env.storage()
        .persistent()
        .get::<DataKey, u32>(&DataKey::SubscriptionCount(user.clone()))
        .unwrap_or(0)
}

/// Build an [`AccountState`] snapshot for `user`.
pub fn load_account_state(env: &Env, user: &Address) -> AccountState {
    AccountState {
        bets: get_bet_count(env, user),
        positions: get_position_count(env, user),
        subscriptions: get_subscription_count(env, user),
    }
}

// ---------------------------------------------------------------------------
// Enforcement checks
// ---------------------------------------------------------------------------

/// Assert that recording an additional bet for `user` would not exceed the
/// configured bet cap.
///
/// # Errors
///
/// - [`MonitorError::BetCapExceeded`] if `current_bets >= max_bets`.
pub fn check_bet_cap(env: &Env, user: &Address) -> Result<(), MonitorError> {
    let current = get_bet_count(env, user);
    let max = get_max_bets(env);
    if current >= max {
        return Err(MonitorError::BetCapExceeded);
    }
    Ok(())
}

/// Assert that recording an additional position for `user` would not exceed the
/// configured position cap.
///
/// # Errors
///
/// - [`MonitorError::PositionCapExceeded`] if `current_positions >= max_positions`.
pub fn check_position_cap(env: &Env, user: &Address) -> Result<(), MonitorError> {
    let current = get_position_count(env, user);
    let max = get_max_positions(env);
    if current >= max {
        return Err(MonitorError::PositionCapExceeded);
    }
    Ok(())
}

/// Assert that recording an additional subscription for `user` would not exceed
/// the configured subscription cap.
///
/// # Errors
///
/// - [`MonitorError::SubscriptionCapExceeded`] if `current_subscriptions >= max_subscriptions`.
pub fn check_subscription_cap(env: &Env, user: &Address) -> Result<(), MonitorError> {
    let current = get_subscription_count(env, user);
    let max = get_max_subscriptions(env);
    if current >= max {
        return Err(MonitorError::SubscriptionCapExceeded);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Overflow-safe counter mutators
// ---------------------------------------------------------------------------

/// Increment the bet count for `user` by one.
///
/// Calls [`check_bet_cap`] before mutating; stores the updated count.
/// Extends persistent-entry TTL after every write.
///
/// # Errors
///
/// - [`MonitorError::BetCapExceeded`] – cap would be exceeded.
/// - [`MonitorError::Overflow`] – arithmetic overflow on the counter
///   (defence-in-depth; practically unreachable given cap magnitudes).
pub fn increment_bets(env: &Env, user: &Address) -> Result<u32, MonitorError> {
    check_bet_cap(env, user)?;
    let current = get_bet_count(env, user);
    let new_count = current
        .checked_add(1)
        .ok_or(MonitorError::Overflow)?;
    let key = DataKey::BetCount(user.clone());
    env.storage().persistent().set(&key, &new_count);
    extend_count_ttl(env, &key);
    Ok(new_count)
}

/// Decrement the bet count for `user` by one.
///
/// # Errors
///
/// - [`MonitorError::Underflow`] – the current count is already zero.
pub fn decrement_bets(env: &Env, user: &Address) -> Result<u32, MonitorError> {
    let current = get_bet_count(env, user);
    let new_count = current
        .checked_sub(1)
        .ok_or(MonitorError::Underflow)?;
    let key = DataKey::BetCount(user.clone());
    env.storage().persistent().set(&key, &new_count);
    extend_count_ttl(env, &key);
    Ok(new_count)
}

/// Increment the position count for `user` by one.
///
/// Calls [`check_position_cap`] before mutating.
///
/// # Errors
///
/// - [`MonitorError::PositionCapExceeded`] – cap would be exceeded.
/// - [`MonitorError::Overflow`] – arithmetic overflow (defence-in-depth).
pub fn increment_positions(env: &Env, user: &Address) -> Result<u32, MonitorError> {
    check_position_cap(env, user)?;
    let current = get_position_count(env, user);
    let new_count = current
        .checked_add(1)
        .ok_or(MonitorError::Overflow)?;
    let key = DataKey::PositionCount(user.clone());
    env.storage().persistent().set(&key, &new_count);
    extend_count_ttl(env, &key);
    Ok(new_count)
}

/// Decrement the position count for `user` by one.
///
/// # Errors
///
/// - [`MonitorError::Underflow`] – the current count is already zero.
pub fn decrement_positions(env: &Env, user: &Address) -> Result<u32, MonitorError> {
    let current = get_position_count(env, user);
    let new_count = current
        .checked_sub(1)
        .ok_or(MonitorError::Underflow)?;
    let key = DataKey::PositionCount(user.clone());
    env.storage().persistent().set(&key, &new_count);
    extend_count_ttl(env, &key);
    Ok(new_count)
}

/// Increment the subscription count for `user` by one.
///
/// Calls [`check_subscription_cap`] before mutating.
///
/// # Errors
///
/// - [`MonitorError::SubscriptionCapExceeded`] – cap would be exceeded.
/// - [`MonitorError::Overflow`] – arithmetic overflow (defence-in-depth).
pub fn increment_subscriptions(env: &Env, user: &Address) -> Result<u32, MonitorError> {
    check_subscription_cap(env, user)?;
    let current = get_subscription_count(env, user);
    let new_count = current
        .checked_add(1)
        .ok_or(MonitorError::Overflow)?;
    let key = DataKey::SubscriptionCount(user.clone());
    env.storage().persistent().set(&key, &new_count);
    extend_count_ttl(env, &key);
    Ok(new_count)
}

/// Decrement the subscription count for `user` by one.
///
/// # Errors
///
/// - [`MonitorError::Underflow`] – the current count is already zero.
pub fn decrement_subscriptions(env: &Env, user: &Address) -> Result<u32, MonitorError> {
    let current = get_subscription_count(env, user);
    let new_count = current
        .checked_sub(1)
        .ok_or(MonitorError::Underflow)?;
    let key = DataKey::SubscriptionCount(user.clone());
    env.storage().persistent().set(&key, &new_count);
    extend_count_ttl(env, &key);
    Ok(new_count)
}

// ---------------------------------------------------------------------------
// TTL management
// ---------------------------------------------------------------------------

/// Minimum remaining ledgers before a per-account count entry is refreshed.
///
/// ~7 days at 5-second ledger time.
const COUNT_TTL_THRESHOLD: u32 = 120_960;

/// Target TTL in ledgers for per-account count entries after a bump.
///
/// ~30 days at 5-second ledger time.
const COUNT_TTL_TO: u32 = 535_680;

/// Extend the TTL of a per-account count entry after each write.
fn extend_count_ttl(env: &Env, key: &DataKey) {
    env.storage()
        .persistent()
        .extend_ttl(key, COUNT_TTL_THRESHOLD, COUNT_TTL_TO);
}
