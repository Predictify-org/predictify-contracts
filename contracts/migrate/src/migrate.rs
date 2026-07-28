//! Migration script for error types across the Predictify contract ecosystem.
//!
//! This module provides a sketch for migrating error data between contract
//! versions. It is intended to be extended as the error surface of each
//! contract evolves.
//!
//! # Overview
//!
//! During a version migration, it may be necessary to re-map or re-encode
//! stored error-related state (discriminants, error counts, etc.). This module
//! defines the scaffolding for such a migration—a single entrypoint that
//! a data-upgrade script can invoke to reshape error data while preserving
//! backward compatibility.
//!
//! # Usage
//!
//! Call `migrate_error_state(...)` during a contract upgrade, passing the
//! previous and target version numbers. The function checks that the stored
//! version matches `expected_version`, applies the error-data reshape, and
//! bumps the stored version to `target_version`.

use soroban_sdk::{Address, Env};

use crate::{ContractError, DataKey};

/// Migrate error-related state from one version to the next.
///
/// # Arguments
///
/// * `env`        - Contract environment.
/// * `admin`      - Authenticated administrator.
/// * `expected`   - The version we expect to find stored. Must equal the
///                  current contract version.
/// * `target`     - The version to upgrade to. Must be strictly greater than
///                  `expected`.
///
/// # Errors
///
/// * [`ContractError::NotInitialized`] if the contract has not been
///   initialized.
/// * [`ContractError::Unauthorized`] if `admin` does not match the stored
///   administrator.
/// * [`ContractError::VersionMismatch`] if `expected` != stored version.
/// * [`ContractError::InvalidTargetVersion`] if `target` <= stored version.
///
/// # Notes
///
/// The current implementation is a no-op for the data-reshape itself —
/// it validates version pre-conditions and delegates the actual migration
/// work to a future `reshape_errors` hook.
pub fn migrate_error_state(
    env: &Env,
    admin: &Address,
    expected: u32,
    target: u32,
) -> Result<(), ContractError> {
    // --- Guard: contract must be initialised ---
    if !env.storage().instance().has(&DataKey::Admin) {
        return Err(ContractError::NotInitialized);
    }

    // --- Guard: caller must match stored admin ---
    let stored_admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(ContractError::NotInitialized)?;
    if admin != &stored_admin {
        return Err(ContractError::Unauthorized);
    }

    // --- Guard: version compare-and-set ---
    let current: u32 = env
        .storage()
        .instance()
        .get(&DataKey::Version)
        .ok_or(ContractError::NotInitialized)?;
    if expected != current {
        return Err(ContractError::VersionMismatch);
    }
    if target <= current {
        return Err(ContractError::InvalidTargetVersion);
    }

    // --- Data reshape (to be extended per-version) ---
    reshape_errors(env, current, target);

    // --- Bump version ---
    env.storage().instance().set(&DataKey::Version, &target);

    Ok(())
}

/// Apply error-data transforms between two versions.
///
/// Each version pair that introduces an error-structure change should add a
/// match arm here. The default arm is a no-op.
fn reshape_errors(_env: &Env, from: u32, to: u32) {
    let _ = (from, to);
}
