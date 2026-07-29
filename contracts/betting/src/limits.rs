//! # Per-account betting limits
//!
//! This module enforces a configurable, admin-managed cap on how much a single
//! account may stake (cumulatively) across all markets tracked by the betting
//! subsystem.  The cap is **global** — it applies across every market, not
//! per-market — and is enforced before any funds are transferred.
//!
//! ## Design
//!
//! | Concern | Choice |
//! |---------|--------|
//! | Storage | Persistent; keyed on `(LimitNs, account)` for usage, `GlobalCap` for the cap |
//! | Auth | `admin.require_auth()` on every state-changing entrypoint |
//! | Overflow | `i128::checked_add` on all accumulators; `None` → `Error::Overflow` |
//! | TTL | Extended on every write (`LIMITS_TTL_LEDGERS`, ~365 days) |
//! | Zero cap | Rejected as `PerAccountLimitInvalidConfig` |
//! | No cap set | Uncapped — [`check_and_record`] always returns `Ok(())` |
//!
//! ## Storage key layout
//!
//! ```text
//! GlobalCap                     → i128          (the ceiling, optional)
//! LimitAdmin                    → Address        (who can change the cap)
//! (LimitNs, account: Address)   → i128          (cumulative usage per account)
//! ```
//!
//! ## Usage flow
//!
//! ```text
//! 1. initialize(env, admin)               – one-time admin setup
//! 2. set_global_cap(env, admin, cap)      – admin sets cap (cap > 0)
//! 3. check_and_record(env, acct, amount)  – called on every bet, BEFORE funds move
//! 4. get_usage(env, acct)                 – read-only query
//! 5. get_global_cap(env)                  – read-only query
//! 6. reset_usage(env, admin, acct)        – admin clears an account's usage
//! 7. remove_global_cap(env, admin)        – admin removes the cap (uncapped)
//! ```
//!
//! ## Error codes (frozen — see `tests/limits_err_stab.rs`)
//!
//! | Variant | Code | When raised |
//! |---------|------|-------------|
//! | `PerAccountLimitExceeded`     | 677 | Cumulative usage + amount > cap |
//! | `PerAccountLimitInvalidConfig`| 678 | Admin supplies a cap ≤ 0 |

#![allow(dead_code)]

use soroban_sdk::{contracttype, symbol_short, Address, Env, Symbol};

use predictify_hybrid::Error;

// ─────────────────────────────────────────────────────────────────────────────
// §1  Constants
// ─────────────────────────────────────────────────────────────────────────────

/// Namespace symbol used as the first element of per-account storage keys.
/// Frozen: renaming this is a storage migration.
pub const LIMIT_NS: Symbol = symbol_short!("BtLmNs");

/// TTL extension applied to persistent entries on every write (~365 days).
pub const LIMITS_TTL_LEDGERS: u32 = 6_307_200;

// ─────────────────────────────────────────────────────────────────────────────
// §2  Storage keys
// ─────────────────────────────────────────────────────────────────────────────

/// Top-level storage keys for the limits module.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LimitsDataKey {
    /// Optional global per-account cap (`i128`).  Absent means uncapped.
    GlobalCap,
    /// Admin address that is permitted to mutate the cap and reset usage.
    LimitAdmin,
}

/// Composite key for an individual account's cumulative usage.
///
/// Stored as `(LIMIT_NS, account)` so keys are namespaced away from any
/// future module that might reuse the same env.
fn usage_key(account: &Address) -> (Symbol, Address) {
    (LIMIT_NS, account.clone())
}

// ─────────────────────────────────────────────────────────────────────────────
// §3  Public API
// ─────────────────────────────────────────────────────────────────────────────

/// Per-account limits manager for the betting subsystem.
///
/// All methods are stateless helpers that act on a shared [`Env`].
pub struct AccountLimits;

impl AccountLimits {
    // ─────────────────────────────────────────────────────────────────────────
    // §3.1  Initialisation
    // ─────────────────────────────────────────────────────────────────────────

    /// Initialise the limits module with an admin address.
    ///
    /// **Authorization**: `admin` must call `require_auth()`.
    ///
    /// # Errors
    /// - [`Error::AlreadyInitialized`] if the admin is already set.
    pub fn initialize(env: &Env, admin: &Address) -> Result<(), Error> {
        admin.require_auth();

        if env
            .storage()
            .persistent()
            .has(&LimitsDataKey::LimitAdmin)
        {
            return Err(Error::AlreadyInitialized);
        }

        env.storage()
            .persistent()
            .set(&LimitsDataKey::LimitAdmin, admin);
        Self::extend_ttl_instance(env, &LimitsDataKey::LimitAdmin);
        Ok(())
    }

    // ─────────────────────────────────────────────────────────────────────────
    // §3.2  Cap management
    // ─────────────────────────────────────────────────────────────────────────

    /// Set the global per-account cap to `cap`.
    ///
    /// `cap` must be strictly positive; zero is rejected as
    /// [`Error::PerAccountLimitInvalidConfig`].
    ///
    /// **Authorization**: the registered admin must call `require_auth()`.
    ///
    /// # Errors
    /// - [`Error::Unauthorized`] if no admin is registered.
    /// - [`Error::PerAccountLimitInvalidConfig`] if `cap <= 0`.
    pub fn set_global_cap(env: &Env, admin: &Address, cap: i128) -> Result<(), Error> {
        Self::require_admin(env, admin)?;

        if cap <= 0 {
            return Err(Error::PerAccountLimitInvalidConfig);
        }

        env.storage()
            .persistent()
            .set(&LimitsDataKey::GlobalCap, &cap);
        Self::extend_ttl_instance(env, &LimitsDataKey::GlobalCap);
        Ok(())
    }

    /// Remove the global cap entirely, making the system uncapped.
    ///
    /// After this call [`get_global_cap`] returns `None` and
    /// [`check_and_record`] always succeeds.
    ///
    /// **Authorization**: the registered admin must call `require_auth()`.
    ///
    /// # Errors
    /// - [`Error::Unauthorized`] if no admin is registered.
    pub fn remove_global_cap(env: &Env, admin: &Address) -> Result<(), Error> {
        Self::require_admin(env, admin)?;
        env.storage()
            .persistent()
            .remove(&LimitsDataKey::GlobalCap);
        Ok(())
    }

    /// Return the current global cap, or `None` if no cap is set (uncapped).
    pub fn get_global_cap(env: &Env) -> Option<i128> {
        env.storage()
            .persistent()
            .get(&LimitsDataKey::GlobalCap)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // §3.3  Usage tracking
    // ─────────────────────────────────────────────────────────────────────────

    /// Return the cumulative usage recorded for `account`.
    ///
    /// Returns `0` when no usage has been recorded (first bet or after
    /// [`reset_usage`]).
    pub fn get_usage(env: &Env, account: &Address) -> i128 {
        env.storage()
            .persistent()
            .get(&usage_key(account))
            .unwrap_or(0i128)
    }

    /// Check whether `amount` would push `account` above the global cap,
    /// and — if it would not — atomically record the new usage.
    ///
    /// This is the **hot path** and must be called **before** any fund
    /// transfer.  It is intentionally not gated by `require_auth` because
    /// the caller (the betting entrypoint) already authenticated the user.
    ///
    /// # Semantics
    ///
    /// * If no cap is set → always returns `Ok(())` without writing.
    /// * If `current_usage + amount > cap` → returns `Err(PerAccountLimitExceeded)`.
    /// * Otherwise → persists `current_usage + amount` and returns `Ok(())`.
    ///
    /// # Errors
    /// - [`Error::Overflow`] if `current_usage + amount` overflows `i128`.
    /// - [`Error::PerAccountLimitExceeded`] if the new total exceeds the cap.
    pub fn check_and_record(env: &Env, account: &Address, amount: i128) -> Result<(), Error> {
        let cap = match Self::get_global_cap(env) {
            Some(c) => c,
            None => return Ok(()), // uncapped
        };

        let current = Self::get_usage(env, account);
        let new_total = current.checked_add(amount).ok_or(Error::Overflow)?;

        if new_total > cap {
            return Err(Error::PerAccountLimitExceeded);
        }

        let key = usage_key(account);
        env.storage().persistent().set(&key, &new_total);
        env.storage()
            .persistent()
            .extend_ttl(&key, LIMITS_TTL_LEDGERS, LIMITS_TTL_LEDGERS);

        Ok(())
    }

    /// Reset the recorded usage for `account` to zero.
    ///
    /// Useful when an admin needs to clear a stale entry (e.g. after a
    /// market refund that reduces a user's effective exposure).
    ///
    /// **Authorization**: the registered admin must call `require_auth()`.
    ///
    /// # Errors
    /// - [`Error::Unauthorized`] if no admin is registered.
    pub fn reset_usage(env: &Env, admin: &Address, account: &Address) -> Result<(), Error> {
        Self::require_admin(env, admin)?;
        let key = usage_key(account);
        env.storage().persistent().remove(&key);
        Ok(())
    }

    // ─────────────────────────────────────────────────────────────────────────
    // §3.4  Admin helpers (private)
    // ─────────────────────────────────────────────────────────────────────────

    /// Require `caller` to be the registered admin and call `require_auth()`.
    ///
    /// Returns `Err(Error::Unauthorized)` if no admin is set or if `caller`
    /// does not match the registered admin.
    fn require_admin(env: &Env, caller: &Address) -> Result<(), Error> {
        let admin: Address = env
            .storage()
            .persistent()
            .get(&LimitsDataKey::LimitAdmin)
            .ok_or(Error::Unauthorized)?;

        if admin != *caller {
            return Err(Error::Unauthorized);
        }

        caller.require_auth();
        Ok(())
    }

    /// Extend TTL for a top-level limits key.
    fn extend_ttl_instance(env: &Env, key: &LimitsDataKey) {
        env.storage()
            .persistent()
            .extend_ttl(key, LIMITS_TTL_LEDGERS, LIMITS_TTL_LEDGERS);
    }
}
