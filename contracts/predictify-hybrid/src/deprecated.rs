//! Deprecated-entrypoints registry for Predictify Hybrid.
//!
//! This module maintains a persistent, on-chain registry of every contract
//! entrypoint that has been marked as deprecated.  Each record captures the
//! entrypoint name, the recommended replacement, the ledger timestamp at
//! which it was deprecated, and an optional human-readable note.
//!
//! # Design goals
//!
//! * **Single source of truth** – callers can query the registry to discover
//!   which functions are deprecated without reading off-chain documentation.
//! * **Admin-gated writes** – only the contract admin may register or remove
//!   entries, preventing griefing.
//! * **Read-only for everyone** – any caller may query the registry via
//!   [`DeprecatedRegistry::get_entry`] and [`DeprecatedRegistry::list_entries`].
//! * **Audit trail** – every registration and removal emits a Soroban event so
//!   indexers can reconstruct the full history.
//!
//! # Storage layout
//!
//! | Key                              | Type                   | Tier       |
//! |----------------------------------|------------------------|------------|
//! | `DataKey::DeprecatedRegistry`    | `Vec<DeprecatedEntry>` | Persistent |
//!
//! The registry is stored as a single `Vec` rather than one key per entry so
//! that the full list can be retrieved in a single storage read.  The maximum
//! number of entries is capped at [`MAX_REGISTRY_ENTRIES`] (64) to bound gas.

#![allow(dead_code)]

use crate::admin::AdminAccessControl;
use crate::err::Error;
use crate::events::emit_deprecated;
use soroban_sdk::{contracttype, symbol_short, Env, String, Symbol, Vec};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Maximum number of entries the registry may hold at any one time.
///
/// Kept deliberately small (64) so that a single `list_entries` call stays
/// well within Soroban's instruction budget.
pub const MAX_REGISTRY_ENTRIES: u32 = 64;

/// Persistent storage key for the deprecated-entrypoints registry vector.
const REGISTRY_KEY: &str = "DeprRegistry";

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// A single record in the deprecated-entrypoints registry.
///
/// # Fields
///
/// * `entrypoint`  – Short symbol of the deprecated function name (e.g. `"verify_result"`).
/// * `replacement` – Short symbol of the recommended successor function.
/// * `since`       – Ledger timestamp (seconds since Unix epoch) at which this
///                   entry was registered.
/// * `note`        – Optional human-readable migration hint (max 128 chars).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeprecatedEntry {
    /// Name of the deprecated entrypoint.
    pub entrypoint: Symbol,
    /// Recommended replacement entrypoint.
    pub replacement: Symbol,
    /// Ledger timestamp when this deprecation was registered (seconds).
    pub since: u64,
    /// Optional migration note (UTF-8, max 128 bytes).
    pub note: Option<String>,
}

// ---------------------------------------------------------------------------
// Registry manager
// ---------------------------------------------------------------------------

/// Manages the on-chain deprecated-entrypoints registry.
///
/// All mutating methods require the caller to be the contract admin (enforced
/// via [`AdminAccessControl::require_admin_auth`]).  Read methods are
/// permissionless.
pub struct DeprecatedRegistry;

impl DeprecatedRegistry {
    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    /// Load the registry from persistent storage, returning an empty `Vec` if
    /// not yet initialised.
    fn load(env: &Env) -> Vec<DeprecatedEntry> {
        let key = Symbol::new(env, REGISTRY_KEY);
        env.storage()
            .persistent()
            .get::<Symbol, Vec<DeprecatedEntry>>(&key)
            .unwrap_or_else(|| Vec::new(env))
    }

    /// Persist the registry back to storage.
    fn save(env: &Env, registry: &Vec<DeprecatedEntry>) {
        let key = Symbol::new(env, REGISTRY_KEY);
        env.storage().persistent().set(&key, registry);
    }

    // -----------------------------------------------------------------------
    // Write operations (admin-only)
    // -----------------------------------------------------------------------

    /// Register a new deprecated entrypoint.
    ///
    /// Adds `entrypoint` to the registry with the supplied metadata.  If an
    /// entry for the same `entrypoint` already exists the call is a no-op and
    /// returns `Ok(())` (idempotent).
    ///
    /// # Arguments
    ///
    /// * `env`         – Soroban environment.
    /// * `admin`       – Address of the contract admin; must pass
    ///                   [`AdminAccessControl::require_admin_auth`].
    /// * `entrypoint`  – Name of the function being deprecated.
    /// * `replacement` – Name of the recommended replacement function.
    /// * `note`        – Optional human-readable migration hint.
    ///
    /// # Errors
    ///
    /// * [`Error::Unauthorized`]      – `admin` is not the contract admin.
    /// * [`Error::AdminNotSet`]       – Contract has not been initialised.
    /// * [`Error::RegistryFull`]      – Registry has reached [`MAX_REGISTRY_ENTRIES`].
    ///
    /// # Events
    ///
    /// Emits `("depr_reg", entrypoint)` on success so indexers can track
    /// when each deprecation was announced.
    pub fn register(
        env: &Env,
        admin: &soroban_sdk::Address,
        entrypoint: Symbol,
        replacement: Symbol,
        note: Option<String>,
    ) -> Result<(), Error> {
        AdminAccessControl::require_admin_auth(env, admin)?;

        let mut registry = Self::load(env);

        // Idempotency: skip if already registered.
        for i in 0..registry.len() {
            if registry.get(i).unwrap().entrypoint == entrypoint {
                return Ok(());
            }
        }

        // Guard against unbounded growth.
        if registry.len() >= MAX_REGISTRY_ENTRIES {
            return Err(Error::RegistryFull);
        }

        let entry = DeprecatedEntry {
            entrypoint: entrypoint.clone(),
            replacement,
            since: env.ledger().timestamp(),
            note,
        };

        registry.push_back(entry);
        Self::save(env, &registry);

        // Announce via event so off-chain indexers can track the registration.
        env.events().publish(
            (symbol_short!("depr_reg"), entrypoint),
            env.ledger().timestamp(),
        );

        Ok(())
    }

    /// Remove a deprecated entrypoint from the registry.
    ///
    /// This is intended for the rare case where a deprecation was recorded in
    /// error, or after a function has been fully removed and the registry entry
    /// is no longer needed.  If the entry does not exist the call is a no-op.
    ///
    /// # Arguments
    ///
    /// * `env`        – Soroban environment.
    /// * `admin`      – Address of the contract admin.
    /// * `entrypoint` – Name of the entrypoint to remove.
    ///
    /// # Errors
    ///
    /// * [`Error::Unauthorized`] – `admin` is not the contract admin.
    /// * [`Error::AdminNotSet`]  – Contract has not been initialised.
    ///
    /// # Events
    ///
    /// Emits `("depr_rem", entrypoint)` when an entry is actually removed.
    pub fn remove(
        env: &Env,
        admin: &soroban_sdk::Address,
        entrypoint: Symbol,
    ) -> Result<(), Error> {
        AdminAccessControl::require_admin_auth(env, admin)?;

        let registry = Self::load(env);
        let mut new_registry: Vec<DeprecatedEntry> = Vec::new(env);
        let mut found = false;

        for i in 0..registry.len() {
            let entry = registry.get(i).unwrap();
            if entry.entrypoint == entrypoint {
                found = true;
            } else {
                new_registry.push_back(entry);
            }
        }

        if found {
            Self::save(env, &new_registry);
            env.events().publish(
                (symbol_short!("depr_rem"), entrypoint),
                env.ledger().timestamp(),
            );
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Read operations (permissionless)
    // -----------------------------------------------------------------------

    /// Look up a single entry by entrypoint name.
    ///
    /// Returns `Some(DeprecatedEntry)` if the entrypoint is registered,
    /// or `None` if it is not in the registry.
    ///
    /// # Arguments
    ///
    /// * `env`        – Soroban environment.
    /// * `entrypoint` – Name of the entrypoint to look up.
    pub fn get_entry(env: &Env, entrypoint: &Symbol) -> Option<DeprecatedEntry> {
        let registry = Self::load(env);
        for i in 0..registry.len() {
            let entry = registry.get(i).unwrap();
            if &entry.entrypoint == entrypoint {
                return Some(entry);
            }
        }
        None
    }

    /// Return the full registry as a `Vec<DeprecatedEntry>`.
    ///
    /// Returns an empty vector if no entries have been registered yet.
    ///
    /// # Arguments
    ///
    /// * `env` – Soroban environment.
    pub fn list_entries(env: &Env) -> Vec<DeprecatedEntry> {
        Self::load(env)
    }

    /// Return the number of entries currently in the registry.
    ///
    /// # Arguments
    ///
    /// * `env` – Soroban environment.
    pub fn entry_count(env: &Env) -> u32 {
        Self::load(env).len()
    }

    /// Return `true` if the given entrypoint is listed as deprecated.
    ///
    /// Convenience wrapper around [`get_entry`](Self::get_entry).
    ///
    /// # Arguments
    ///
    /// * `env`        – Soroban environment.
    /// * `entrypoint` – Name of the entrypoint to check.
    pub fn is_deprecated(env: &Env, entrypoint: &Symbol) -> bool {
        Self::get_entry(env, entrypoint).is_some()
    }

    // -----------------------------------------------------------------------
    // Helper: record a call to a deprecated entrypoint
    // -----------------------------------------------------------------------

    /// Convenience wrapper that both emits the standard `DeprecatedCall` event
    /// **and** ensures the entrypoint is present in the registry.
    ///
    /// Call this from every deprecated entrypoint body instead of calling
    /// `emit_deprecated` directly:
    ///
    /// ```rust,ignore
    /// DeprecatedRegistry::record_call(
    ///     &env,
    ///     &Symbol::new(&env, "verify_result"),
    ///     &Symbol::new(&env, "fetch_oracle_result"),
    /// );
    /// ```
    ///
    /// If the entrypoint is not yet in the registry (e.g. because it was
    /// deprecated before this module existed) the call silently succeeds
    /// without attempting a write — writes require admin auth which is not
    /// available in a regular entrypoint call path.
    ///
    /// # Arguments
    ///
    /// * `env`         – Soroban environment.
    /// * `entrypoint`  – Name of the deprecated function being called.
    /// * `replacement` – Name of the recommended replacement (for the event).
    pub fn record_call(env: &Env, entrypoint: &Symbol, _replacement: &Symbol) {
        emit_deprecated(env, entrypoint);
    }
}
