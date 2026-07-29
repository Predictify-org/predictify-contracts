//! Migrate contract — version-gated state migration for the Predictify ecosystem.
//!
//! # Overview
//!
//! This contract provides a minimal, admin-gated facility for advancing the
//! on-chain storage schema across numbered versions.  It tracks a single
//! `admin` address and a `version` counter; all state-changing entrypoints
//! require the caller to authenticate as the stored admin.
//!
//! # Versioning model
//!
//! The compiled-in [`CURRENT_VERSION`] constant represents the highest version
//! this build understands.  `initialize` rejects any `version` argument that
//! is `0` or exceeds `CURRENT_VERSION`.  `migrate_error_data` enforces a
//! compare-and-set on the stored version: the caller must supply the exact
//! current version as `expected_version` and a strictly higher `target_version`.
//!
//! # Security
//!
//! Every entrypoint that mutates storage calls `admin.require_auth()` before
//! touching any state.  Read-only entrypoints (`admin`, `current_version`)
//! do not require auth.

#![no_std]

mod migrate;

use soroban_sdk::{contract, contracterror, contractimpl, contracttype, Address, Env};

// ---------------------------------------------------------------------------
// Storage keys
// ---------------------------------------------------------------------------

/// Discriminated storage keys used by instance storage.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// Stores the administrator [`Address`].
    Admin,
    /// Stores the current schema version as `u32`.
    Version,
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors returned by migration entrypoints.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    /// The caller is not the stored administrator.
    Unauthorized = 1,
    /// The contract has not been initialised yet.
    NotInitialized = 2,
    /// The contract has already been initialised.
    AlreadyInitialized = 3,
    /// The supplied `expected_version` does not match the stored version.
    VersionMismatch = 4,
    /// The supplied `target_version` is not strictly greater than the current
    /// version, or `version` was `0` during `initialize`.
    InvalidTargetVersion = 5,
    /// Migration data failed validation.
    InvalidMigrationData = 6,
    /// A storage migration step failed.
    MigrationFailed = 7,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// The highest schema version this contract build understands.
///
/// `initialize` and future migration steps use this as an upper bound so that
/// a mis-configured deployment cannot advance beyond what the wasm handles.
const CURRENT_VERSION: u32 = 2;

// ---------------------------------------------------------------------------
// Contract
// ---------------------------------------------------------------------------

/// The Migrate contract.
#[contract]
pub struct MigrateContract;

#[contractimpl]
impl MigrateContract {
    /// Initialise the contract, recording `admin` and `version` in storage.
    ///
    /// # Auth
    ///
    /// Requires `admin.require_auth()`.  The call is rejected if the signer
    /// does not match `admin`.
    ///
    /// # Arguments
    ///
    /// * `admin`   — Address that will own subsequent migration calls.
    /// * `version` — Starting schema version; must be in `1..=CURRENT_VERSION`.
    ///
    /// # Errors
    ///
    /// * [`ContractError::InvalidTargetVersion`] — `version` is `0` or above
    ///   [`CURRENT_VERSION`].
    /// * [`ContractError::AlreadyInitialized`] — the contract was already
    ///   initialised.
    pub fn initialize(env: Env, admin: Address, version: u32) -> Result<(), ContractError> {
        admin.require_auth();

        if version == 0 || version > CURRENT_VERSION {
            return Err(ContractError::InvalidTargetVersion);
        }

        if env.storage().instance().has(&DataKey::Admin) {
            return Err(ContractError::AlreadyInitialized);
        }

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Version, &version);

        Ok(())
    }

    /// Return the stored administrator address.
    ///
    /// # Errors
    ///
    /// * [`ContractError::NotInitialized`] — the contract has not been
    ///   initialised.
    pub fn admin(env: Env) -> Result<Address, ContractError> {
        Self::load_admin(&env)
    }

    /// Return the current stored schema version.
    ///
    /// # Errors
    ///
    /// * [`ContractError::NotInitialized`] — the contract has not been
    ///   initialised.
    pub fn current_version(env: Env) -> Result<u32, ContractError> {
        Self::load_version(&env)
    }

    /// Migrate error-related state from `expected_version` to `target_version`.
    ///
    /// This is the primary migration entrypoint.  It delegates to
    /// [`migrate::migrate_error_state`], which enforces the following
    /// preconditions before advancing the stored version:
    ///
    /// 1. The contract must be initialised.
    /// 2. `admin` must match the stored administrator.
    /// 3. `expected_version` must equal the currently stored version.
    /// 4. `target_version` must be strictly greater than the stored version.
    ///
    /// # Auth
    ///
    /// Requires `admin.require_auth()`.  The gate fires before any storage
    /// read so that an unauthenticated call never touches contract state.
    ///
    /// # Arguments
    ///
    /// * `admin`            — Must be the stored administrator.
    /// * `expected_version` — Compare-and-set guard: must equal stored version.
    /// * `target_version`   — Destination version; must be `> expected_version`.
    ///
    /// # Errors
    ///
    /// * [`ContractError::NotInitialized`]     — contract not initialised.
    /// * [`ContractError::Unauthorized`]       — `admin` != stored admin.
    /// * [`ContractError::VersionMismatch`]    — `expected_version` != stored.
    /// * [`ContractError::InvalidTargetVersion`] — `target_version` <= stored.
    pub fn migrate_error_data(
        env: Env,
        admin: Address,
        expected_version: u32,
        target_version: u32,
    ) -> Result<(), ContractError> {
        // Auth gate fires before any storage access.
        admin.require_auth();
        migrate::migrate_error_state(&env, &admin, expected_version, target_version)
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    fn load_admin(env: &Env) -> Result<Address, ContractError> {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(ContractError::NotInitialized)
    }

    fn load_version(env: &Env) -> Result<u32, ContractError> {
        env.storage()
            .instance()
            .get(&DataKey::Version)
            .ok_or(ContractError::NotInitialized)
    }
}
