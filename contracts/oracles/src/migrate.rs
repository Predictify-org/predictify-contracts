//! Migration script sketch for the Oracles contract.
//!
//! Provides a versioned guard around future on-chain reshapes of the oracle
//! registry's storage layout. The reshape itself is a no-op stub — each
//! schema change should add a match arm to [`reshape_oracle_data`].
//!
//! # Usage
//!
//! Call [`migrate_data`] during a contract upgrade. It checks the stored
//! data version matches `expected`, applies the reshape, and bumps the
//! stored version to `target`.

use soroban_sdk::{Address, Env};

use crate::{DataKey, Error};

/// Data-schema version assumed when none has been stored yet.
const DEFAULT_DATA_VERSION: u32 = 1;

/// Read the oracle registry's current data-schema version.
///
/// Defaults to [`DEFAULT_DATA_VERSION`] when no migration has ever run.
pub fn data_version(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::DataVersion)
        .unwrap_or(DEFAULT_DATA_VERSION)
}

/// Migrate the oracle registry's stored data from `expected` to `target`.
///
/// # Arguments
/// * `env`      - Contract environment.
/// * `admin`    - Caller; must authenticate via `require_auth()`.
/// * `expected` - The data version we expect to find stored.
/// * `target`   - The version to upgrade to; must be strictly greater.
///
/// # Errors
/// * [`Error::VersionMismatch`] if `expected` != the stored data version.
/// * [`Error::InvalidTargetVersion`] if `target` <= the stored data version.
///
/// # Notes
/// The data reshape itself is a no-op stub — see [`reshape_oracle_data`].
pub fn migrate_data(env: &Env, admin: &Address, expected: u32, target: u32) -> Result<(), Error> {
    admin.require_auth();

    let current = data_version(env);
    if expected != current {
        return Err(Error::VersionMismatch);
    }
    if target <= current {
        return Err(Error::InvalidTargetVersion);
    }

    reshape_oracle_data(env, current, target);

    env.storage().instance().set(&DataKey::DataVersion, &target);

    Ok(())
}

/// Apply data-layout transforms between two oracle-registry schema versions.
///
/// Each version pair that changes the stored shape of oracle data should add
/// a match arm here. The default arm is a no-op.
fn reshape_oracle_data(_env: &Env, from: u32, to: u32) {
    let _ = (from, to);
}
