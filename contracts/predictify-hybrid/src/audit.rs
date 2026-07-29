//! # Per-Market Audit Log
//!
//! This module provides a persistent, per-market audit trail for off-chain reads.
//! Every significant state change on a market (creation, resolution, dispute lifecycle,
//! fee collection) is appended to an immutable, append-only log that is keyed by
//! `market_id`. Off-chain clients can paginate the log through the read entrypoints
//! [`crate::PredictifyHybrid::get_market_audit_log`] and
//! [`crate::PredictifyHybrid::get_market_audit_entry`].
//!
//! ## Design
//!
//! Each market maintains:
//!
//! - A **head record** (`DataKey::MarketAuditHead(market_id)`) storing
//!   `total_entries: u32` — the count of entries written so far.
//! - An indexed sequence of **entry records**
//!   (`DataKey::MarketAuditLog(market_id, index)`) where `index` starts at `1`.
//!
//! Indices are 1-based so that `total_entries == 0` unambiguously means "no
//! entries" without requiring a sentinel value.
//!
//! ## Security
//!
//! - No `require_auth` call is needed in this module because all writes are
//!   performed by contract-internal code paths that have already verified
//!   caller authentication.
//! - Arithmetic for `new_index` uses `saturating_add` to stay overflow-safe; an
//!   audit log that has reached `u32::MAX` entries stops silently (markets are
//!   not expected to reach that cardinality).
//!
//! ## Storage
//!
//! All keys use the [`DataKey`] enum variants added to `storage.rs`:
//!
//! ```text
//! DataKey::MarketAuditHead(market_id)  → MarketAuditHead { total_entries: u32 }
//! DataKey::MarketAuditLog(market_id, index) → MarketAuditEntry { … }
//! ```
//!
//! Both families use persistent storage with the same TTL as the market record
//! (`MARKET_TTL_LEDGERS`) so that audit entries expire with the market they
//! describe.

use soroban_sdk::{contracttype, Address, Env, Map, String, Symbol, Vec};

use crate::storage::{DataKey, MARKET_TTL_LEDGERS};

// ===== TYPES =====

/// The category of action recorded in a per-market audit entry.
///
/// Each variant maps to a concrete state-changing entrypoint in
/// [`crate::PredictifyHybrid`]. Variants must not be reordered or removed once
/// deployed because the `#[contracttype]` macro encodes them by ordinal position
/// in XDR; doing so would silently misinterpret stored entries.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MarketAuditAction {
    /// The market was created via `create_market`.
    MarketCreated,
    /// The market was manually resolved by an admin via `resolve_market_manual`
    /// or `resolve_market_with_ties`.
    MarketResolved,
    /// The market was force-resolved by an admin via `force_resolve_market`.
    MarketForceResolved,
    /// A dispute was filed against the market outcome via `dispute_market`.
    DisputeFiled,
    /// An open dispute on this market was resolved via `resolve_dispute`.
    DisputeResolved,
    /// Platform fees were collected from this market via `collect_fees`.
    FeesCollected,
}

/// A single immutable entry in a market's per-market audit log.
///
/// Entries are written in the order they occur (index 1 is the oldest).
/// The `details` map holds action-specific key/value metadata for
/// off-chain consumers; keys are short Soroban `Symbol`s and values
/// are human-readable `String`s.
///
/// # Example (MarketCreated)
///
/// ```text
/// details["question"] = "Will BTC reach $100k?"
/// details["duration"] = "30"         // days
/// details["end_time"]  = "1721000000" // Unix seconds
/// ```
///
/// # Example (MarketResolved)
///
/// ```text
/// details["outcome"]  = "yes"
/// details["method"]   = "Manual"
/// ```
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketAuditEntry {
    /// 1-based position within this market's log.
    pub index: u32,
    /// The action that produced this entry.
    pub action: MarketAuditAction,
    /// The address that triggered the action (admin or user).
    pub actor: Address,
    /// Ledger timestamp (Unix seconds) when the action occurred.
    pub timestamp: u64,
    /// Structured, action-specific metadata for off-chain consumers.
    pub details: Map<Symbol, String>,
}

/// Head metadata for a market's audit log.
///
/// Stored once per market; contains only the running count of entries.
/// Callers can read this to determine valid index bounds before paginating.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketAuditHead {
    /// Total number of entries written to this market's log.
    /// Entries are indexed `[1, total_entries]`.
    pub total_entries: u32,
}

// ===== MANAGER =====

/// Manages the per-market audit log: appending entries and serving reads.
///
/// All mutating methods are `pub(crate)` — they must only be called from
/// entrypoints that have already validated caller authentication.
///
/// # Read Methods
///
/// [`MarketAuditManager::get_entry`] — fetch one entry by 1-based index.
/// [`MarketAuditManager::get_entries`] — paginated reverse-chronological slice.
/// [`MarketAuditManager::get_head`] — fetch the log head (entry count).
pub struct MarketAuditManager;

impl MarketAuditManager {
    // ===== WRITES =====

    /// Append a new entry to the audit log for `market_id`.
    ///
    /// Returns the 1-based index of the newly written entry.
    /// If the entry counter would overflow `u32::MAX` the call is a no-op and
    /// returns `u32::MAX` — that cardinality is not reachable in practice.
    ///
    /// # Parameters
    ///
    /// - `env`       - Soroban environment.
    /// - `market_id` - The market this entry belongs to.
    /// - `action`    - The type of event being recorded.
    /// - `actor`     - The address responsible for the action.
    /// - `details`   - Free-form key/value metadata (use short `Symbol` keys).
    pub(crate) fn append(
        env: &Env,
        market_id: &Symbol,
        action: MarketAuditAction,
        actor: Address,
        details: Map<Symbol, String>,
    ) -> u32 {
        let head_key = DataKey::MarketAuditHead(market_id.clone());

        let mut head: MarketAuditHead = env
            .storage()
            .persistent()
            .get(&head_key)
            .unwrap_or(MarketAuditHead { total_entries: 0 });

        // Overflow guard: stop silently at u32::MAX.
        let new_index = match head.total_entries.checked_add(1) {
            Some(i) => i,
            None => return u32::MAX,
        };

        let entry = MarketAuditEntry {
            index: new_index,
            action,
            actor,
            timestamp: env.ledger().timestamp(),
            details,
        };

        let entry_key = DataKey::MarketAuditLog(market_id.clone(), new_index);
        env.storage().persistent().set(&entry_key, &entry);
        env.storage()
            .persistent()
            .extend_ttl(&entry_key, MARKET_TTL_LEDGERS, MARKET_TTL_LEDGERS);

        head.total_entries = new_index;
        env.storage().persistent().set(&head_key, &head);
        env.storage()
            .persistent()
            .extend_ttl(&head_key, MARKET_TTL_LEDGERS, MARKET_TTL_LEDGERS);

        new_index
    }

    // ===== READS =====

    /// Returns the log head for `market_id`, or `None` if the market has no
    /// audit entries yet.
    pub fn get_head(env: &Env, market_id: &Symbol) -> Option<MarketAuditHead> {
        let head_key = DataKey::MarketAuditHead(market_id.clone());
        env.storage().persistent().get(&head_key)
    }

    /// Fetches one entry by its 1-based `index` from the market's audit log.
    ///
    /// Returns `None` when `index` is 0 or exceeds `total_entries`.
    pub fn get_entry(env: &Env, market_id: &Symbol, index: u32) -> Option<MarketAuditEntry> {
        if index == 0 {
            return None;
        }
        let key = DataKey::MarketAuditLog(market_id.clone(), index);
        env.storage().persistent().get(&key)
    }

    /// Returns a reverse-chronological page of at most `limit` entries for
    /// `market_id`, starting from the most-recent entry (index ==
    /// `total_entries`) and walking backwards.
    ///
    /// - `limit` is capped at 100 to bound ledger computation cost.
    /// - Returns an empty `Vec` if the market has no audit entries.
    ///
    /// # Parameters
    ///
    /// - `env`       - Soroban environment.
    /// - `market_id` - Target market.
    /// - `limit`     - Maximum number of entries to return (capped at 100).
    pub fn get_entries(env: &Env, market_id: &Symbol, limit: u32) -> Vec<MarketAuditEntry> {
        let mut result = Vec::new(env);

        let head = match Self::get_head(env, market_id) {
            Some(h) => h,
            None => return result,
        };

        if head.total_entries == 0 {
            return result;
        }

        // Cap limit to prevent unbounded computation.
        let effective_limit = limit.min(100);

        let mut idx = head.total_entries;
        let mut count = 0u32;

        while idx >= 1 && count < effective_limit {
            if let Some(entry) = Self::get_entry(env, market_id, idx) {
                result.push_back(entry);
            }
            idx = idx.saturating_sub(1);
            count = count.saturating_add(1);
        }

        result
    }
}
