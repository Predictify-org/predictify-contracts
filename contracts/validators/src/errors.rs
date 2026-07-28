//! Error types for the Validators contract.
//!
//! All state-changing entrypoints use typed errors via `#[contracterror]`.
//! Each variant has a stable integer discriminant that is part of the
//! contract's public ABI.

use soroban_sdk::contracterror;

/// Errors returned by the Validators contract.
///
/// Each variant is assigned a stable integer code that forms part of the
/// contract's public API surface. Codes **must not** be renumbered once
/// deployed — add new variants at the end.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ValidatorError {
    /// Caller is not authorized for the action (e.g. a non-admin calling an
    /// admin-only entrypoint, or a validator calling on behalf of another).
    Unauthorized = 1,
    /// Contract has not yet been initialized via `initialize`.
    NotInitialized = 2,
    /// Contract has already been initialized; `initialize` may not be called
    /// again.
    AlreadyInitialized = 3,
    /// The validator address provided is not currently registered.
    ValidatorNotFound = 4,
    /// The validator address is already registered; duplicate registration is
    /// not allowed.
    AlreadyRegistered = 5,
    /// The stake amount is below the minimum required threshold.
    StakeTooLow = 6,
    /// The stake amount would exceed the per-validator maximum cap.
    StakeTooHigh = 7,
    /// An arithmetic operation overflowed; the transaction is aborted.
    Overflow = 8,
    /// Validators subsystem is paused — all state-changing operations halt
    /// until `unpause_validators` is called by the admin.
    ValidatorsPaused = 9,
    /// The new owner address supplied to `transfer_ownership` is invalid.
    InvalidNewOwner = 10,
    /// A configuration parameter is out of the accepted range.
    InvalidConfig = 11,
}
