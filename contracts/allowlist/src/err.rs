//! Error types for the Allowlist contract.
//!
//! All state-changing entrypoints use typed errors via `#[contracterror]`.

use soroban_sdk::contracterror;

/// Errors returned by the Allowlist contract.
///
/// Each variant is assigned a stable integer code that forms part of the
/// contract's public API surface.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum AllowlistError {
    /// Caller is not authorized for the action.
    Unauthorized = 1,
    /// Contract has not been initialized.
    NotInitialized = 2,
    /// Contract has already been initialized.
    AlreadyInitialized = 3,
    /// The allowlist ID does not exist.
    AllowlistNotFound = 4,
    /// The allowlist ID already exists.
    AllowlistAlreadyExists = 5,
    /// The provided address is already in the allowlist.
    AddressAlreadyInAllowlist = 6,
    /// The provided address is not in the allowlist.
    AddressNotInAllowlist = 7,
    /// The allowlist is empty.
    AllowlistEmpty = 8,
    /// Invalid input parameters.
    InvalidInput = 9,
    /// Arithmetic overflow occurred.
    Overflow = 10,
    /// The address has reached its configured per-account membership cap.
    AccountLimitExceeded = 11,
    /// Arithmetic underflow occurred while releasing membership capacity.
    Underflow = 12,
}
