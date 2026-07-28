//! Error types for the Analytics contract.
//!
//! Every variant is assigned an explicit `u32` discriminant that forms part of
//! the contract's **client-facing API surface**. The codes are consumed by
//! off-chain analytics dashboards, indexers, and monitoring tooling that may
//! persist or branch on the raw numeric value.
//!
//! # Stability guarantee
//!
//! Once a code appears in a deployed version it **must not** be renumbered,
//! removed, or reused for a different meaning without an explicit versioning
//! decision and a corresponding migration note in the PR description.
//!
//! New variants must be appended with a fresh, previously-unused code.  The
//! stability snapshot in `tests/err_stab.rs` will fail to compile until the
//! addition is deliberately registered there.

use soroban_sdk::contracterror;

/// Errors returned by the Analytics contract.
///
/// Code ranges:
/// * `1–9`   — general / auth errors
/// * `10–19` — data / query errors
/// * `20–29` — metric / aggregation errors
/// * `30–39` — configuration / admin errors
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    // ----- General / auth (1-9) -----
    /// Caller is not authorized to perform the requested action.
    Unauthorized = 1,
    /// Admin address has not been set; contract may not have been initialized.
    AdminNotSet = 2,
    /// Contract is not yet initialized; call `initialize` first.
    NotInitialized = 3,
    /// Contract has already been initialized and cannot be initialized again.
    AlreadyInitialized = 4,

    // ----- Data / query (10-19) -----
    /// The requested market was not found in the analytics store.
    MarketNotFound = 10,
    /// The requested metric snapshot does not exist.
    SnapshotNotFound = 11,
    /// The requested time-range is invalid (e.g. end < start).
    InvalidTimeRange = 12,
    /// The requested aggregation window is not supported.
    UnsupportedWindow = 13,

    // ----- Metric / aggregation (20-29) -----
    /// An arithmetic overflow occurred while computing a metric.
    Overflow = 20,
    /// The analytics store has reached its maximum capacity.
    StoreFull = 21,
    /// The submitted data point is a duplicate of an already-recorded entry.
    DuplicateEntry = 22,
    /// The data point value is out of the accepted range.
    ValueOutOfRange = 23,

    // ----- Configuration / admin (30-39) -----
    /// One or more configuration parameters are invalid.
    InvalidConfig = 30,
    /// Analytics collection is currently paused.
    AnalyticsPaused = 31,
    /// The requested operation is not permitted in the current contract state.
    InvalidState = 32,
}

#[cfg(test)]
mod tests {
    use super::ContractError;

    /// Guard that the inline discriminants match expectations.
    ///
    /// This in-module test is intentionally minimal; the canonical, exhaustive
    /// stability snapshot lives in `tests/err_stab.rs`.
    #[test]
    fn error_discriminants_match_docs() {
        assert_eq!(ContractError::Unauthorized as u32, 1);
        assert_eq!(ContractError::AdminNotSet as u32, 2);
        assert_eq!(ContractError::NotInitialized as u32, 3);
        assert_eq!(ContractError::AlreadyInitialized as u32, 4);
        assert_eq!(ContractError::MarketNotFound as u32, 10);
        assert_eq!(ContractError::SnapshotNotFound as u32, 11);
        assert_eq!(ContractError::InvalidTimeRange as u32, 12);
        assert_eq!(ContractError::UnsupportedWindow as u32, 13);
        assert_eq!(ContractError::Overflow as u32, 20);
        assert_eq!(ContractError::StoreFull as u32, 21);
        assert_eq!(ContractError::DuplicateEntry as u32, 22);
        assert_eq!(ContractError::ValueOutOfRange as u32, 23);
        assert_eq!(ContractError::InvalidConfig as u32, 30);
        assert_eq!(ContractError::AnalyticsPaused as u32, 31);
        assert_eq!(ContractError::InvalidState as u32, 32);
    }
}
