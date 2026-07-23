//! # Deprecated Entrypoint Registry
//!
//! Centralised, compile-time registry of all deprecated contract entrypoints.
//!
//! ## Purpose
//!
//! Deprecated functions are scattered across multiple modules.  This registry
//! provides a **single source of truth** so that:
//!
//! - Developers can list every deprecated entrypoint in one place.
//! - On-chain callers can query [`DeprecatedRegistry::all`] (exposed via
//!   the `list_deprecated` contract entrypoint) to discover deprecated
//!   functions programmatically.
//! - Each deprecated call site routes through
//!   [`DeprecatedRegistry::emit_if_deprecated`], which emits a
//!   `DeprecatedCall` event only for registered entries and is a safe no-op
//!   otherwise.
//!
//! ## Adding a New Entry
//!
//! 1. Add a new arm to [`DeprecatedRegistry::lookup`] with the function name,
//!    replacement, deprecation date, and planned removal version.
//! 2. Add the same entry to [`DeprecatedRegistry::all`].
//! 3. At the deprecated call site, call
//!    `DeprecatedRegistry::emit_if_deprecated(&env, "function_name")`.
//! 4. Update `docs/DEPRECATED.md` with the new entry.
//!
//! ## Design Constraints
//!
//! - All fields use [`Symbol`] (≤ 32 chars on Soroban) to avoid heap
//!   allocations and keep the registry zero-cost at rest.
//! - The registry is **pure** — no storage reads or writes.  Entries are
//!   hard-coded at compile time, matching the [`EventSchemaRegistry`] pattern.

use crate::events::emit_deprecated;
use soroban_sdk::{contracttype, symbol_short, Env, Symbol, Vec};

/// Metadata for a single deprecated entrypoint.
///
/// Each entry captures enough information for both human developers and
/// on-chain tooling to understand the deprecation and plan a migration.
///
/// # Fields
///
/// * `name`             — Symbol name of the deprecated function.
/// * `replacement`      — Symbol name of the recommended replacement.
///                        Shortened to fit Soroban's 32-char limit;
///                        see `docs/DEPRECATED.md` for fully-qualified paths.
/// * `since`            — ISO-8601 date when the deprecation was introduced
///                        (e.g. `"2026-06-28"`).
/// * `removal_version`  — Contract version in which the entrypoint will be
///                        removed.  Set to `"TBD"` when no version is
///                        scheduled yet.
///
/// # Example
///
/// ```rust,ignore
/// let entry = DeprecatedEntrypoint {
///     name: Symbol::new(&env, "verify_result"),
///     replacement: Symbol::new(&env, "fetch_oracle_result"),
///     since: Symbol::new(&env, "2026-06-28"),
///     removal_version: Symbol::new(&env, "TBD"),
/// };
/// ```
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeprecatedEntrypoint {
    /// The name of the deprecated function.
    pub name: Symbol,
    /// The recommended replacement function (shortened for Symbol limits).
    pub replacement: Symbol,
    /// ISO-8601 date the deprecation was introduced.
    pub since: Symbol,
    /// Contract version in which the entrypoint will be removed (`"TBD"` if
    /// not yet scheduled).
    pub removal_version: Symbol,
}

/// Centralised registry of deprecated entrypoints.
///
/// This is a unit struct with only associated functions — no instance state.
/// All data is compiled-in; there are no storage reads.
///
/// # Usage
///
/// ```rust,ignore
/// use crate::deprecated::DeprecatedRegistry;
///
/// // Check a single entrypoint
/// if let Some(entry) = DeprecatedRegistry::lookup(&env, "verify_result") {
///     log!("Use {} instead", entry.replacement);
/// }
///
/// // List everything
/// let all = DeprecatedRegistry::all(&env);
/// ```
pub struct DeprecatedRegistry;

impl DeprecatedRegistry {
    // ===== CONSTANTS =====

    /// Total number of registered deprecated entrypoints.
    ///
    /// Keep this in sync with `all()`.  A compile-time test
    /// (`test_all_returns_expected_count`) guards against drift.
    const ENTRY_COUNT: u32 = 6;

    // ===== PUBLIC API =====

    /// Look up a deprecated entrypoint by function name.
    ///
    /// Returns `Some(DeprecatedEntrypoint)` if `name` is a registered
    /// deprecated function, or `None` otherwise.
    ///
    /// # Arguments
    ///
    /// * `env`  — Soroban environment (needed for `Symbol` construction).
    /// * `name` — Exact function name to look up (e.g. `"verify_result"`).
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let entry = DeprecatedRegistry::lookup(&env, "verify_result");
    /// assert!(entry.is_some());
    /// ```
    pub fn lookup(env: &Env, name: &str) -> Option<DeprecatedEntrypoint> {
        match name {
            "verify_result" => Some(DeprecatedEntrypoint {
                name: Symbol::new(env, "verify_result"),
                replacement: Symbol::new(env, "fetch_oracle_result"),
                since: Symbol::new(env, "2026-06-28"),
                removal_version: Symbol::new(env, "TBD"),
            }),
            "resolve_market" => Some(DeprecatedEntrypoint {
                name: Symbol::new(env, "resolve_market"),
                replacement: Symbol::new(env, "resolve_market_manual"),
                since: Symbol::new(env, "2026-06-28"),
                removal_version: Symbol::new(env, "TBD"),
            }),
            "collect_fees" => Some(DeprecatedEntrypoint {
                name: Symbol::new(env, "collect_fees"),
                replacement: Symbol::new(env, "FeeManager_collect"),
                since: Symbol::new(env, "2026-06-28"),
                removal_version: Symbol::new(env, "TBD"),
            }),
            "transfer_fees" => Some(DeprecatedEntrypoint {
                name: Symbol::new(env, "transfer_fees"),
                replacement: Symbol::new(env, "FeeUtils_transfer"),
                since: Symbol::new(env, "2026-06-28"),
                removal_version: Symbol::new(env, "TBD"),
            }),
            "calculate_fee_amount" => Some(DeprecatedEntrypoint {
                name: Symbol::new(env, "calculate_fee_amount"),
                replacement: Symbol::new(env, "FeeCalc_platform_fee"),
                since: Symbol::new(env, "2026-06-28"),
                removal_version: Symbol::new(env, "TBD"),
            }),
            "process_creation_fee" => Some(DeprecatedEntrypoint {
                name: Symbol::new(env, "process_creation_fee"),
                replacement: Symbol::new(env, "FeeManager_creation"),
                since: Symbol::new(env, "2026-06-28"),
                removal_version: Symbol::new(env, "TBD"),
            }),
            _ => None,
        }
    }

    /// Return all registered deprecated entrypoints.
    ///
    /// The returned [`Vec`] is freshly allocated on each call (no caching).
    /// Order matches the declaration order in this registry.
    ///
    /// # Arguments
    ///
    /// * `env` — Soroban environment.
    pub fn all(env: &Env) -> Vec<DeprecatedEntrypoint> {
        let mut entries = Vec::new(env);

        // ---- Contract-level entrypoints (#[contractimpl]) ----

        entries.push_back(DeprecatedEntrypoint {
            name: Symbol::new(env, "verify_result"),
            replacement: Symbol::new(env, "fetch_oracle_result"),
            since: Symbol::new(env, "2026-06-28"),
            removal_version: Symbol::new(env, "TBD"),
        });

        entries.push_back(DeprecatedEntrypoint {
            name: Symbol::new(env, "resolve_market"),
            replacement: Symbol::new(env, "resolve_market_manual"),
            since: Symbol::new(env, "2026-06-28"),
            removal_version: Symbol::new(env, "TBD"),
        });

        // ---- Module-internal deprecated wrappers ----

        entries.push_back(DeprecatedEntrypoint {
            name: Symbol::new(env, "collect_fees"),
            replacement: Symbol::new(env, "FeeManager_collect"),
            since: Symbol::new(env, "2026-06-28"),
            removal_version: Symbol::new(env, "TBD"),
        });

        entries.push_back(DeprecatedEntrypoint {
            name: Symbol::new(env, "transfer_fees"),
            replacement: Symbol::new(env, "FeeUtils_transfer"),
            since: Symbol::new(env, "2026-06-28"),
            removal_version: Symbol::new(env, "TBD"),
        });

        entries.push_back(DeprecatedEntrypoint {
            name: Symbol::new(env, "calculate_fee_amount"),
            replacement: Symbol::new(env, "FeeCalc_platform_fee"),
            since: Symbol::new(env, "2026-06-28"),
            removal_version: Symbol::new(env, "TBD"),
        });

        entries.push_back(DeprecatedEntrypoint {
            name: Symbol::new(env, "process_creation_fee"),
            replacement: Symbol::new(env, "FeeManager_creation"),
            since: Symbol::new(env, "2026-06-28"),
            removal_version: Symbol::new(env, "TBD"),
        });

        entries
    }

    /// Convenience predicate: returns `true` when `name` is a registered
    /// deprecated entrypoint.
    ///
    /// # Arguments
    ///
    /// * `env`  — Soroban environment.
    /// * `name` — Function name to check.
    pub fn is_deprecated(env: &Env, name: &str) -> bool {
        Self::lookup(env, name).is_some()
    }

    /// Emit a [`DeprecatedCall`](crate::events::DeprecatedCall) event if
    /// `name` is a registered deprecated entrypoint.
    ///
    /// This is the **recommended call-site helper**: drop-in replacement for
    /// manual `emit_deprecated` calls.  For non-deprecated names the function
    /// is a safe, zero-cost no-op.
    ///
    /// # Arguments
    ///
    /// * `env`  — Soroban environment.
    /// * `name` — Function name of the (potentially) deprecated entrypoint.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // In a deprecated function body:
    /// DeprecatedRegistry::emit_if_deprecated(&env, "verify_result");
    /// ```
    pub fn emit_if_deprecated(env: &Env, name: &str) {
        if let Some(entry) = Self::lookup(env, name) {
            emit_deprecated(env, &entry.name);
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::Env;

    // --- lookup tests ---

    #[test]
    fn test_lookup_known_entry() {
        let env = Env::default();
        let entry = DeprecatedRegistry::lookup(&env, "verify_result");
        assert!(entry.is_some(), "verify_result should be in the registry");

        let entry = entry.unwrap();
        assert_eq!(entry.name, Symbol::new(&env, "verify_result"));
        assert_eq!(entry.replacement, Symbol::new(&env, "fetch_oracle_result"));
        assert_eq!(entry.since, Symbol::new(&env, "2026-06-28"));
        assert_eq!(entry.removal_version, Symbol::new(&env, "TBD"));
    }

    #[test]
    fn test_lookup_unknown_returns_none() {
        let env = Env::default();
        let entry = DeprecatedRegistry::lookup(&env, "nonexistent_function");
        assert!(entry.is_none(), "unknown names must return None");
    }

    #[test]
    fn test_lookup_empty_string_returns_none() {
        let env = Env::default();
        let entry = DeprecatedRegistry::lookup(&env, "");
        assert!(entry.is_none(), "empty string must return None");
    }

    // --- is_deprecated tests ---

    #[test]
    fn test_is_deprecated_true() {
        let env = Env::default();
        assert!(DeprecatedRegistry::is_deprecated(&env, "resolve_market"));
        assert!(DeprecatedRegistry::is_deprecated(&env, "collect_fees"));
        assert!(DeprecatedRegistry::is_deprecated(&env, "transfer_fees"));
        assert!(DeprecatedRegistry::is_deprecated(&env, "calculate_fee_amount"));
        assert!(DeprecatedRegistry::is_deprecated(&env, "process_creation_fee"));
    }

    #[test]
    fn test_is_deprecated_false() {
        let env = Env::default();
        assert!(!DeprecatedRegistry::is_deprecated(&env, "create_market"));
        assert!(!DeprecatedRegistry::is_deprecated(&env, "vote"));
        assert!(!DeprecatedRegistry::is_deprecated(&env, "place_bet"));
    }

    // --- all() tests ---

    #[test]
    fn test_all_returns_expected_count() {
        let env = Env::default();
        let entries = DeprecatedRegistry::all(&env);
        assert_eq!(
            entries.len(),
            DeprecatedRegistry::ENTRY_COUNT,
            "all() must return exactly ENTRY_COUNT entries"
        );
    }

    #[test]
    fn test_all_entries_have_nonempty_replacement() {
        let env = Env::default();
        let entries = DeprecatedRegistry::all(&env);
        for entry in entries.iter() {
            // Symbol::new panics on empty strings, so if we got here
            // the replacement is non-empty.  Double-check by comparing
            // against a known non-empty symbol.
            assert_ne!(
                entry.replacement,
                entry.name,
                "replacement must differ from name for {:?}",
                entry.name
            );
        }
    }

    #[test]
    fn test_registry_entries_unique_names() {
        let env = Env::default();
        let entries = DeprecatedRegistry::all(&env);
        for i in 0..entries.len() {
            for j in (i + 1)..entries.len() {
                let a = entries.get(i).unwrap();
                let b = entries.get(j).unwrap();
                assert_ne!(
                    a.name, b.name,
                    "duplicate name detected at indices {} and {}",
                    i, j
                );
            }
        }
    }

    #[test]
    fn test_all_entries_match_lookup() {
        let env = Env::default();
        let entries = DeprecatedRegistry::all(&env);

        // Build the name strings that correspond to each entry.
        let names = [
            "verify_result",
            "resolve_market",
            "collect_fees",
            "transfer_fees",
            "calculate_fee_amount",
            "process_creation_fee",
        ];

        assert_eq!(
            entries.len(),
            names.len() as u32,
            "names array out of sync with all()"
        );

        for (idx, name) in names.iter().enumerate() {
            let from_lookup = DeprecatedRegistry::lookup(&env, name);
            assert!(
                from_lookup.is_some(),
                "all() entry at index {} (name={}) not found via lookup",
                idx,
                name
            );
            let from_all = entries.get(idx as u32).unwrap();
            let from_lookup = from_lookup.unwrap();
            assert_eq!(
                from_all, from_lookup,
                "all()[{}] and lookup(\"{}\") must return identical entries",
                idx, name
            );
        }
    }

    // --- emit_if_deprecated tests ---

    #[test]
    fn test_emit_if_deprecated_known_does_not_panic() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        env.as_contract(&contract_id, || {
            // Must not panic — event should be emitted.
            DeprecatedRegistry::emit_if_deprecated(&env, "verify_result");
        });
    }

    #[test]
    fn test_emit_if_deprecated_unknown_is_noop() {
        let env = Env::default();
        // No contract context needed — unknown name should be a pure no-op.
        DeprecatedRegistry::emit_if_deprecated(&env, "nonexistent_function");
        // Reaching here without panic confirms the no-op behaviour.
    }
}
