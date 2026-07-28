//! Persistent and instance storage key definitions for the Monitor contract.
//!
//! All storage keys are defined here in one place so that both [`crate::lib`]
//! and [`crate::limits`] share a single source of truth.

use soroban_sdk::{contracttype, Address};

/// Storage keys used by the Monitor contract.
///
/// - Instance keys: `Initialized`, `Admin`, `MaxBets`, `MaxPositions`,
///   `MaxSubscriptions` — read/written on every invocation; cheap to access.
/// - Persistent keys: `BetCount(Address)`, `PositionCount(Address)`,
///   `SubscriptionCount(Address)` — keyed per account; outlive the instance TTL.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// Whether the contract has been initialized (`bool`).
    Initialized,

    /// The current admin address (`Address`).
    Admin,

    /// Configured maximum number of bets per account (`u32`).
    MaxBets,

    /// Configured maximum number of positions per account (`u32`).
    MaxPositions,

    /// Configured maximum number of subscriptions per account (`u32`).
    MaxSubscriptions,

    /// Current active bet count for the given account (`u32`).
    BetCount(Address),

    /// Current active position count for the given account (`u32`).
    PositionCount(Address),

    /// Current active subscription count for the given account (`u32`).
    SubscriptionCount(Address),
}
