//! Storage keys and data types for the Reporting contract.

use soroban_sdk::contracttype;

/// Persistent-storage keys used by the Reporting contract.
#[contracttype]
pub enum DataKey {
    /// Whether the contract has been initialized.
    Initialized,
    /// The current admin address.
    Admin,
    /// Whether reporting is paused (`true` = paused).
    ReportingPaused,
    /// Next report ID counter.
    NextReportId,
    /// Next dispute ID counter.
    NextDisputeId,
}
