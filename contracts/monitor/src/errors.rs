//! Error types for the Monitor contract.
//!
//! All state-changing entrypoints return typed errors via [`#[contracterror]`].
//! Each variant carries a stable integer discriminant that forms part of the
//! contract's public ABI; never renumber without a version bump.

use soroban_sdk::contracterror;

/// Errors returned by the Monitor contract.
///
/// # Stability
///
/// Discriminant values are **frozen**: changing them is a breaking API change.
/// Add new variants at the end; never reuse or reorder existing codes.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum MonitorError {
    /// The caller is not authorized to perform this action.
    Unauthorized = 1,

    /// The contract has not been initialized; call `initialize` first.
    NotInitialized = 2,

    /// The contract has already been initialized.
    AlreadyInitialized = 3,

    /// Recording a bet would exceed this account's per-account bet cap.
    BetCapExceeded = 4,

    /// Recording a position would exceed this account's per-account position cap.
    PositionCapExceeded = 5,

    /// Recording a subscription would exceed this account's per-account
    /// subscription cap.
    SubscriptionCapExceeded = 6,

    /// One or more input parameters are invalid (e.g. a zero cap value).
    InvalidInput = 7,

    /// An arithmetic overflow was detected on an internal counter.
    ///
    /// This should be unreachable in practice given the small cap magnitudes,
    /// but is provided as a defence-in-depth guard.
    Overflow = 8,

    /// An arithmetic underflow was detected; a counter decrement was attempted
    /// on a zero-valued count.
    Underflow = 9,
}
