//! Error types for the Reporting contract.
//!
//! All state-changing entrypoints use typed errors via `#[contracterror]`.

use soroban_sdk::contracterror;

/// Errors returned by the Reporting contract.
///
/// Each variant is assigned a stable integer code that forms part of the
/// contract's public API surface.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ReportingError {
    /// Caller is not authorized for the action.
    Unauthorized = 1,
    /// Contract has not been initialized.
    NotInitialized = 2,
    /// Contract has already been initialized.
    AlreadyInitialized = 3,
    /// Reporting is paused — state-changing operations are halted.
    ReportingPaused = 4,
    /// An admin address is required but the provided address is invalid.
    InvalidAdmin = 5,
    /// A reporter address is required but the provided address is invalid.
    InvalidReporter = 6,
    /// The report ID does not exist.
    ReportNotFound = 7,
    /// The dispute ID does not exist.
    DisputeNotFound = 8,
    /// The new owner address is invalid.
    InvalidNewOwner = 9,
}
