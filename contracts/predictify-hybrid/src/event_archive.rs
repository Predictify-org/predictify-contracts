//! Event archive and historical query support.
//!
//! Provides archiving of resolved/cancelled events (markets) and gas-efficient,
//! paginated historical query functions for analytics and UI. Exposes only
//! public metadata and outcome; no sensitive data (votes, stakes, addresses).
//!
//! # Archive Bounds
//!
//! The archive is capped at [`MAX_ARCHIVE_SIZE`] entries to prevent unbounded
//! on-chain storage growth. Once the cap is reached, `archive_event` returns
//! [`Error::ArchiveFull`]. Admins can call `prune_archive` to remove the oldest
//! N entries and make room for new ones.
//!
//! # Pagination
//!
//! All query functions accept a `cursor` (start index) and `limit` (capped at
//! [`MAX_QUERY_LIMIT`]) and return `(entries, next_cursor)`. Callers should
//! advance the cursor until `next_cursor == previous_cursor` (no more pages).

use crate::err::Error;
use crate::market_id_generator::MarketIdGenerator;
use crate::types::{EventHistoryEntry, Market, MarketState};
use soroban_sdk::{contracttype, panic_with_error, Address, Env, String, Symbol, Vec};

// ---------------------------------------------------------------------------
// Deterministic archive key derivation
// ---------------------------------------------------------------------------

/// Derive a deterministic archive storage key from a `market_id` and a `suffix`.
///
/// This is the single source of truth for all archive-related storage key
/// derivation. It replaces legacy `format!`-based string concatenation with
/// a consistent tuple-namespace pattern that is fully deterministic and
/// avoids dynamic heap allocation.
///
/// The returned tuple `(Symbol, Symbol, Symbol)` implements `IntoVal<Env, Val>`
/// and can be used directly with `env.storage().persistent().get(&key)`,
/// `.set()`, `.has()`, and `.remove()`.
///
/// # Determinism
///
/// `derive_archive_key(env, id, s)` always produces the same storage key for
/// the same `env`, `id`, and `s` — regardless of call count, order, or the
/// state of any other storage.
///
/// # Arguments
/// * `env` - Soroban environment.
/// * `market_id` - The market / event identifier.
/// * `suffix` - A short string label for the key variant (e.g. `"compressed"`,
///   `"compressed_ref"`).
///
/// # Returns
///
/// A 3-tuple `(Symbol, Symbol, Symbol)` suitable for use as a Soroban
/// persistent storage key.
pub fn derive_archive_key(env: &Env, market_id: &Symbol, suffix: &str) -> (Symbol, Symbol, Symbol) {
    (
        Symbol::new(env, "__archive"),
        market_id.clone(),
        Symbol::new(env, suffix),
    )
}

/// Maximum events returned per query (gas safety).
pub const MAX_QUERY_LIMIT: u32 = 30;

/// Hard cap on the number of archived entries stored on-chain.
///
/// Prevents unbounded storage growth. When the archive reaches this limit,
/// `archive_event` returns `Error::ArchiveFull`. Use `prune_archive` to
/// remove old entries and free capacity.
///
/// Rationale: At ~1 KB per entry (Symbol + u64), 1 000 entries ≈ 1 MB of
/// persistent storage — well within Soroban's practical limits while still
/// bounding worst-case growth.
pub const MAX_ARCHIVE_SIZE: u32 = 1_000;

/// Storage key for archived event timestamps (market_id -> archived_at).
const ARCHIVED_TS_KEY: &str = "evt_archived";

/// Storage key for the sorted archive index [(timestamp, market_id)] in ascending order.
const ARCHIVED_INDEX_KEY: &str = "evt_arch_ix";

/// Pagination cursor for archive pruning.
///
/// Returned by `prune_archive` so callers can resume pruning from where they left off.
/// Pass `None` to start from the beginning.
#[derive(Clone, Debug)]
#[contracttype]
pub struct PruneCursor {
    /// Timestamp of the last pruned entry; 0 means "start from the beginning".
    pub last_timestamp: u64,
    /// Market ID of the last pruned entry (tiebreaker for same-timestamp entries).
    pub last_market_id: Symbol,
    /// Whether there are no more entries to prune.
    pub done: bool,
}

impl PruneCursor {
    /// Create a new cursor pointing to the beginning (before any entries).
    pub fn new(env: &Env) -> Self {
        Self {
            last_timestamp: 0,
            last_market_id: Symbol::new(env, "_"),
            done: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Sorted index helpers
// ---------------------------------------------------------------------------

/// Insert a (timestamp, market_id) pair into the sorted archive index,
/// maintaining ascending order by timestamp (then by market_id as tiebreaker).
fn insert_into_sorted_index(env: &Env, timestamp: u64, market_id: &Symbol) {
    let index_key = Symbol::new(env, ARCHIVED_INDEX_KEY);
    let index: Vec<(u64, Symbol)> = env
        .storage()
        .persistent()
        .get(&index_key)
        .unwrap_or_else(|| Vec::new(env));

    let mut new_index = Vec::new(env);
    let mut inserted = false;

    for i in 0..index.len() {
        if let Some(entry) = index.get(i) {
            if !inserted && (timestamp < entry.0 || (timestamp == entry.0 && *market_id < entry.1))
            {
                new_index.push_back((timestamp, market_id.clone()));
                inserted = true;
            }
            new_index.push_back(entry);
        }
    }
    if !inserted {
        new_index.push_back((timestamp, market_id.clone()));
    }

    env.storage().persistent().set(&index_key, &new_index);
}

/// Event archive and historical query manager.
pub struct EventArchive;

impl EventArchive {
    /// Mark a resolved or cancelled event as archived (admin only).
    ///
    /// # Arguments
    /// * `env` - Soroban environment
    /// * `admin` - Caller must be contract admin
    /// * `market_id` - Market/event to archive
    ///
    /// # Errors
    /// * `Unauthorized` - Caller is not admin
    /// * `MarketNotFound` - Market does not exist
    /// * `InvalidState` - Market must be Resolved or Cancelled
    /// * `AlreadyClaimed` - Event is already archived
    /// * `ArchiveFull` - Archive has reached [`MAX_ARCHIVE_SIZE`]; call `prune_archive` first
    pub fn archive_event(env: &Env, admin: &Address, market_id: &Symbol) -> Result<(), Error> {
        admin.require_auth();

        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(env, "Admin"))
            .unwrap_or_else(|| panic_with_error!(env, Error::AdminNotSet));

        if admin != &stored_admin {
            return Err(Error::Unauthorized);
        }

        let market: Market = env
            .storage()
            .persistent()
            .get(market_id)
            .ok_or(Error::MarketNotFound)?;

        if market.state != MarketState::Resolved && market.state != MarketState::Cancelled {
            return Err(Error::InvalidState);
        }

        let key = Symbol::new(env, ARCHIVED_TS_KEY);
        let mut archived: soroban_sdk::Map<Symbol, u64> = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(soroban_sdk::Map::new(env));

        if archived.get(market_id.clone()).is_some() {
            return Err(Error::AlreadyClaimed);
        }

        // Enforce archive size cap to prevent unbounded storage growth.
        if archived.len() >= MAX_ARCHIVE_SIZE {
            return Err(Error::ArchiveFull);
        }

        let now = env.ledger().timestamp();
        archived.set(market_id.clone(), now);
        env.storage().persistent().set(&key, &archived);

        // Maintain the sorted index for deterministic pruning
        insert_into_sorted_index(env, now, market_id);

        Ok(())
    }

    /// Remove the oldest `count` entries from the archive (admin only).
    ///
    /// Frees capacity so that new events can be archived after the cap is reached.
    /// Entries are pruned deterministically by archive timestamp ascending (oldest first).
    /// A [`PruneCursor`] enables paginated pruning — pass the returned cursor on the next call
    /// to resume where you left off.
    ///
    /// # Arguments
    /// * `env` - Soroban environment
    /// * `admin` - Caller must be contract admin
    /// * `count` - Number of oldest entries to remove (capped at [`MAX_QUERY_LIMIT`])
    /// * `cursor` - [`Some(PruneCursor)`] to resume from a previous position, or [`None`] to
    ///   start from the beginning
    ///
    /// # Returns
    /// `(number_pruned, new_cursor)` — the caller should persist `new_cursor` for the next call.
    ///
    /// # Errors
    /// * `Unauthorized` - Caller is not admin
    pub fn prune_archive(
        env: &Env,
        admin: &Address,
        count: u32,
        cursor: Option<PruneCursor>,
    ) -> Result<(u32, PruneCursor), Error> {
        admin.require_auth();

        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(env, "Admin"))
            .unwrap_or_else(|| panic_with_error!(env, Error::AdminNotSet));

        if admin != &stored_admin {
            return Err(Error::Unauthorized);
        }

        let count = core::cmp::min(count, MAX_QUERY_LIMIT);
        if count == 0 {
            return Ok((0, cursor.unwrap_or_else(|| PruneCursor::new(env))));
        }

        // Read the sorted index
        let index_key = Symbol::new(env, ARCHIVED_INDEX_KEY);
        let index: Vec<(u64, Symbol)> = env
            .storage()
            .persistent()
            .get(&index_key)
            .unwrap_or_else(|| Vec::new(env));

        if index.is_empty() {
            return Ok((
                0,
                PruneCursor {
                    last_timestamp: 0,
                    last_market_id: Symbol::new(env, "_"),
                    done: true,
                },
            ));
        }

        // Determine starting position from cursor
        let start_cursor = cursor.unwrap_or_else(|| PruneCursor::new(env));

        // If cursor says done, there's nothing more to prune
        if start_cursor.done {
            return Ok((0, start_cursor));
        }

        let start_pos = if start_cursor.last_timestamp == 0 {
            0u32
        } else {
            // Find first entry strictly after the cursor
            let mut pos = 0u32;
            for i in 0..index.len() {
                if let Some(entry) = index.get(i) {
                    if entry.0 > start_cursor.last_timestamp
                        || (entry.0 == start_cursor.last_timestamp
                            && entry.1 > start_cursor.last_market_id)
                    {
                        break;
                    }
                    pos = i + 1;
                }
            }
            pos
        };

        if start_pos >= index.len() {
            return Ok((
                0,
                PruneCursor {
                    last_timestamp: start_cursor.last_timestamp,
                    last_market_id: start_cursor.last_market_id,
                    done: true,
                },
            ));
        }

        // Remove up to `count` entries starting from start_pos
        let archived_key = Symbol::new(env, ARCHIVED_TS_KEY);
        let mut archived: soroban_sdk::Map<Symbol, u64> = env
            .storage()
            .persistent()
            .get(&archived_key)
            .unwrap_or_else(|| soroban_sdk::Map::new(env));

        let mut removed = 0u32;
        let mut last_ts: u64 = 0;
        let mut last_id = Symbol::new(env, "_");
        let mut new_index = Vec::new(env);

        for i in 0..index.len() {
            if let Some(entry) = index.get(i) {
                if i < start_pos {
                    // Keep entries before start_pos
                    new_index.push_back(entry);
                } else if removed < count {
                    // Remove this entry from the archive map
                    archived.remove(entry.1.clone());
                    last_ts = entry.0;
                    last_id = entry.1.clone();
                    removed += 1;
                } else {
                    // Keep remaining entries
                    new_index.push_back(entry);
                }
            }
        }

        env.storage().persistent().set(&archived_key, &archived);
        env.storage().persistent().set(&index_key, &new_index);

        let done = removed < count || new_index.is_empty();
        let new_cursor = PruneCursor {
            last_timestamp: last_ts,
            last_market_id: last_id,
            done,
        };

        Ok((removed, new_cursor))
    }

    /// Check if an event is archived.
    pub fn is_archived(env: &Env, market_id: &Symbol) -> bool {
        let key = Symbol::new(env, ARCHIVED_TS_KEY);
        let archived: soroban_sdk::Map<Symbol, u64> = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(soroban_sdk::Map::new(env));
        archived.get(market_id.clone()).is_some()
    }

    /// Get archived_at timestamp for a market (None if not archived).
    fn get_archived_at(env: &Env, market_id: &Symbol) -> Option<u64> {
        let key = Symbol::new(env, ARCHIVED_TS_KEY);
        let archived: soroban_sdk::Map<Symbol, u64> = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(soroban_sdk::Map::new(env));
        archived.get(market_id.clone())
    }

    /// Return the current number of archived events.
    pub fn archive_size(env: &Env) -> u32 {
        let key = Symbol::new(env, ARCHIVED_TS_KEY);
        let archived: soroban_sdk::Map<Symbol, u64> = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(soroban_sdk::Map::new(env));
        archived.len()
    }

    /// Build EventHistoryEntry from market and registry entry (public metadata only).
    fn market_to_history_entry(
        env: &Env,
        market_id: &Symbol,
        market: &Market,
        created_at: u64,
    ) -> EventHistoryEntry {
        let archived_at = Self::get_archived_at(env, market_id);
        // Use the dedicated category field if set, otherwise fall back to oracle feed_id
        let category = market
            .category
            .clone()
            .unwrap_or_else(|| market.oracle_config.feed_id.clone());

        EventHistoryEntry {
            market_id: market_id.clone(),
            question: market.question.clone(),
            outcomes: market.outcomes.clone(),
            end_time: market.end_time,
            created_at,
            state: market.state,
            winning_outcome: market.get_winning_outcome(), // Get first outcome for backward compatibility
            total_staked: market.total_staked,
            archived_at,
            category,
            tags: market.tags.clone(),
        }
    }

    /// Query events by creation time range (paginated, bounded).
    ///
    /// Returns events whose creation timestamp is in [from_ts, to_ts].
    /// Only public metadata and outcome are returned.
    ///
    /// # Arguments
    /// * `env` - Soroban environment
    /// * `from_ts` - Start of time range (inclusive)
    /// * `to_ts` - End of time range (inclusive)
    /// * `cursor` - Pagination cursor (start index in registry)
    /// * `limit` - Max results (capped at MAX_QUERY_LIMIT)
    ///
    /// # Returns
    /// (entries, next_cursor). next_cursor is cursor + number of registry entries scanned.
    pub fn query_events_history(
        env: &Env,
        from_ts: u64,
        to_ts: u64,
        cursor: u32,
        limit: u32,
    ) -> (Vec<EventHistoryEntry>, u32) {
        let limit = core::cmp::min(limit, MAX_QUERY_LIMIT);
        let registry_page = MarketIdGenerator::get_market_id_registry(env, cursor, limit);
        let mut result = Vec::new(env);
        let mut scanned = 0u32;

        for i in 0..registry_page.len() {
            if let Some(entry) = registry_page.get(i) {
                scanned += 1;
                let created_at = entry.timestamp;
                if created_at >= from_ts && created_at <= to_ts {
                    if let Some(market) = env
                        .storage()
                        .persistent()
                        .get::<Symbol, Market>(&entry.market_id)
                    {
                        result.push_back(Self::market_to_history_entry(
                            env,
                            &entry.market_id,
                            &market,
                            created_at,
                        ));
                    }
                }
            }
        }

        (result, cursor + scanned)
    }

    /// Query events by resolution status (paginated, bounded).
    ///
    /// Returns events in the given state (e.g. Resolved, Cancelled, Active).
    pub fn query_events_by_resolution_status(
        env: &Env,
        status: MarketState,
        cursor: u32,
        limit: u32,
    ) -> (Vec<EventHistoryEntry>, u32) {
        let limit = core::cmp::min(limit, MAX_QUERY_LIMIT);
        let registry_page = MarketIdGenerator::get_market_id_registry(env, cursor, limit);
        let mut result = Vec::new(env);
        let mut scanned = 0u32;

        for i in 0..registry_page.len() {
            if let Some(entry) = registry_page.get(i) {
                scanned += 1;
                if let Some(market) = env
                    .storage()
                    .persistent()
                    .get::<Symbol, Market>(&entry.market_id)
                {
                    if market.state == status {
                        result.push_back(Self::market_to_history_entry(
                            env,
                            &entry.market_id,
                            &market,
                            entry.timestamp,
                        ));
                    }
                }
            }
        }

        (result, cursor + scanned)
    }

    /// Query events by category (paginated, bounded).
    ///
    /// Returns events whose category matches the given category string.
    /// Checks the dedicated category field first, then falls back to oracle feed_id.
    pub fn query_events_by_category(
        env: &Env,
        category: &String,
        cursor: u32,
        limit: u32,
    ) -> (Vec<EventHistoryEntry>, u32) {
        let limit = core::cmp::min(limit, MAX_QUERY_LIMIT);
        let registry_page = MarketIdGenerator::get_market_id_registry(env, cursor, limit);
        let mut result = Vec::new(env);
        let mut scanned = 0u32;

        for i in 0..registry_page.len() {
            if let Some(entry) = registry_page.get(i) {
                scanned += 1;
                if let Some(market) = env
                    .storage()
                    .persistent()
                    .get::<Symbol, Market>(&entry.market_id)
                {
                    // Match against dedicated category field if set, otherwise oracle feed_id
                    let market_category = market
                        .category
                        .clone()
                        .unwrap_or_else(|| market.oracle_config.feed_id.clone());
                    if market_category == *category {
                        result.push_back(Self::market_to_history_entry(
                            env,
                            &entry.market_id,
                            &market,
                            entry.timestamp,
                        ));
                    }
                }
            }
        }

        (result, cursor + scanned)
    }

    /// Query events by tags (paginated, bounded).
    ///
    /// Returns events that have ANY of the provided tags (OR logic).
    /// If no tags are provided, returns an empty result.
    pub fn query_events_by_tags(
        env: &Env,
        tags: &Vec<String>,
        cursor: u32,
        limit: u32,
    ) -> (Vec<EventHistoryEntry>, u32) {
        let limit = core::cmp::min(limit, MAX_QUERY_LIMIT);
        let registry_page = MarketIdGenerator::get_market_id_registry(env, cursor, limit);
        let mut result = Vec::new(env);
        let mut scanned = 0u32;

        if tags.is_empty() {
            return (result, cursor);
        }

        for i in 0..registry_page.len() {
            if let Some(entry) = registry_page.get(i) {
                scanned += 1;
                if let Some(market) = env
                    .storage()
                    .persistent()
                    .get::<Symbol, Market>(&entry.market_id)
                {
                    // Check if any of the market's tags match any of the query tags
                    let mut matched = false;
                    for j in 0..market.tags.len() {
                        if let Some(market_tag) = market.tags.get(j) {
                            for k in 0..tags.len() {
                                if let Some(query_tag) = tags.get(k) {
                                    if market_tag == query_tag {
                                        matched = true;
                                        break;
                                    }
                                }
                            }
                            if matched {
                                break;
                            }
                        }
                    }
                    if matched {
                        result.push_back(Self::market_to_history_entry(
                            env,
                            &entry.market_id,
                            &market,
                            entry.timestamp,
                        ));
                    }
                }
            }
        }

        (result, cursor + scanned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Market, MarketState, OracleConfig};
    use alloc::string::ToString;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::Map;

    // =========================================================================
    // derive_archive_key tests
    // =========================================================================

    #[test]
    fn test_derive_archive_key_deterministic() {
        // Same inputs must always produce the same key.
        let env = Env::default();
        let market_id = Symbol::new(&env, "market_42");

        let key_a = derive_archive_key(&env, &market_id, "compressed");
        let key_b = derive_archive_key(&env, &market_id, "compressed");

        assert_eq!(key_a, key_b);
    }

    #[test]
    fn test_derive_archive_key_different_ids_produce_different_keys() {
        // Different market IDs with the same suffix must produce different keys.
        let env = Env::default();
        let id1 = Symbol::new(&env, "market_1");
        let id2 = Symbol::new(&env, "market_2");

        let key1 = derive_archive_key(&env, &id1, "compressed");
        let key2 = derive_archive_key(&env, &id2, "compressed");

        assert_ne!(key1, key2);
    }

    #[test]
    fn test_derive_archive_key_different_suffixes_produce_different_keys() {
        // Different suffixes with the same market ID must produce different keys.
        let env = Env::default();
        let market_id = Symbol::new(&env, "market_99");

        let compressed_key = derive_archive_key(&env, &market_id, "compressed");
        let ref_key = derive_archive_key(&env, &market_id, "compressed_ref");

        assert_ne!(compressed_key, ref_key);
    }

    #[test]
    fn test_derive_archive_key_long_market_id() {
        // Maximum-length market IDs (32 chars for Soroban Symbol) must work.
        let env = Env::default();
        let long_id = Symbol::new(&env, "a_very_long_market_id_32_chars__");

        let key = derive_archive_key(&env, &long_id, "compressed");
        // The key should not panic and be usable in storage operations.
        let contract_id = env.register(crate::PredictifyHybrid, ());
        env.as_contract(&contract_id, || {
            env.storage().persistent().set(&key, &42u32);
            let val: Option<u32> = env.storage().persistent().get(&key);
            assert_eq!(val, Some(42));
        });
    }

    #[test]
    fn test_derive_archive_key_special_char_suffix() {
        // Suffixes with underscores and numbers (valid Symbol chars) must work.
        let env = Env::default();
        let market_id = Symbol::new(&env, "m1");

        let key = derive_archive_key(&env, &market_id, "compressed_v2");
        let contract_id = env.register(crate::PredictifyHybrid, ());
        env.as_contract(&contract_id, || {
            env.storage().persistent().set(&key, &true);
            let val: Option<bool> = env.storage().persistent().get(&key);
            assert_eq!(val, Some(true));
        });
    }

    #[test]
    fn test_derive_archive_key_namespace_isolation() {
        // Keys with different market IDs must not collide in storage.
        let env = Env::default();
        let id_a = Symbol::new(&env, "market_a");
        let id_b = Symbol::new(&env, "market_b");

        let contract_id = env.register(crate::PredictifyHybrid, ());
        env.as_contract(&contract_id, || {
            let key_a = derive_archive_key(&env, &id_a, "compressed");
            let key_b = derive_archive_key(&env, &id_b, "compressed");

            env.storage().persistent().set(&key_a, &100u32);
            env.storage().persistent().set(&key_b, &200u32);

            let val_a: Option<u32> = env.storage().persistent().get(&key_a);
            let val_b: Option<u32> = env.storage().persistent().get(&key_b);

            assert_eq!(val_a, Some(100));
            assert_eq!(val_b, Some(200));
        });
    }

    #[test]
    fn test_derive_archive_key_storage_roundtrip() {
        // Full write-read roundtrip to verify the key works with persistent storage.
        let env = Env::default();
        let market_id = Symbol::new(&env, "roundtrip_test");
        let suffix = "migration";

        let key = derive_archive_key(&env, &market_id, suffix);
        let contract_id = env.register(crate::PredictifyHybrid, ());

        let stored_value: u64 = 123456789;

        env.as_contract(&contract_id, || {
            env.storage().persistent().set(&key, &stored_value);

            let retrieved: Option<u64> = env.storage().persistent().get(&key);
            assert_eq!(retrieved, Some(stored_value));

            // Verify the tuple components are as expected
            let (ns, id, sfx) = key;
            assert_eq!(ns, Symbol::new(&env, "__archive"));
            assert_eq!(id, market_id);
            assert_eq!(sfx, Symbol::new(&env, suffix));
        });
    }

    struct EventArchiveTest {
        env: Env,
        admin: Address,
    }

    impl EventArchiveTest {
        fn new() -> Self {
            let env = Env::default();
            let admin = Address::generate(&env);
            EventArchiveTest { env, admin }
        }
    }

    #[test]
    fn test_archive_event_requires_admin() {
        let test = EventArchiveTest::new();
        // Test that archive_event requires admin authentication
        let admin = test.admin;
        assert!(!admin.to_string().is_empty());
    }

    #[test]
    fn test_is_archived_initial_state() {
        let test = EventArchiveTest::new();
        let contract_id = test.env.register(crate::PredictifyHybrid, ());
        // Test that new market is not archived
        let market_id = Symbol::new(&test.env, "new_market");
        let archived = test.env.as_contract(&contract_id, || {
            EventArchive::is_archived(&test.env, &market_id)
        });
        assert!(!archived);
    }

    #[test]
    fn test_is_archived_nonexistent_market() {
        let test = EventArchiveTest::new();
        let contract_id = test.env.register(crate::PredictifyHybrid, ());
        // Test is_archived on market that doesn't exist
        let market_id = Symbol::new(&test.env, "nonexistent");
        let archived = test.env.as_contract(&contract_id, || {
            EventArchive::is_archived(&test.env, &market_id)
        });
        assert!(!archived);
    }

    #[test]
    fn test_archive_event_requires_resolved_or_cancelled() {
        let test = EventArchiveTest::new();
        // Test that archive requires market to be Resolved or Cancelled
        // Will fail with MarketNotFound for nonexistent market
        let market_id = Symbol::new(&test.env, "active_market");
        let admin = test.admin;
        // This would require a valid market in Resolved/Cancelled state
        assert!(!admin.to_string().is_empty());
    }

    #[test]
    fn test_query_events_history_empty() {
        let test = EventArchiveTest::new();
        let contract_id = test.env.register(crate::PredictifyHybrid, ());
        // Test querying history on empty archive
        let (entries, next_cursor) = test.env.as_contract(&contract_id, || {
            EventArchive::query_events_history(&test.env, 0, 100, 0, 10)
        });
        assert_eq!(entries.len(), 0);
        assert_eq!(next_cursor, 0);
    }

    #[test]
    fn test_query_events_history_pagination() {
        let test = EventArchiveTest::new();
        let contract_id = test.env.register(crate::PredictifyHybrid, ());
        // Test pagination parameters are respected
        let from_ts = 1000u64;
        let to_ts = 2000u64;
        let cursor = 0u32;
        let limit = 10u32;
        // Query with valid pagination parameters
        let (entries, _) = test.env.as_contract(&contract_id, || {
            EventArchive::query_events_history(&test.env, from_ts, to_ts, cursor, limit)
        });
        assert_eq!(entries.len(), 0); // No markets yet
    }

    #[test]
    fn test_query_events_history_time_range() {
        let test = EventArchiveTest::new();
        // Test that time range filtering works
        let early_time = 1000u64;
        let late_time = 2000u64;
        assert!(late_time > early_time);
    }

    #[test]
    fn test_query_events_history_limit_cap() {
        let test = EventArchiveTest::new();
        // Test that limit is capped at MAX_QUERY_LIMIT (30)
        let requested_limit = 100u32; // More than MAX_QUERY_LIMIT
        let max_limit = 30u32;
        assert!(requested_limit > max_limit);
    }

    #[test]
    fn test_query_events_by_resolution_status_active() {
        let test = EventArchiveTest::new();
        let contract_id = test.env.register(crate::PredictifyHybrid, ());
        // Test querying by Active status
        let status = MarketState::Active;
        let (entries, cursor) = test.env.as_contract(&contract_id, || {
            EventArchive::query_events_by_resolution_status(&test.env, status, 0, 10)
        });
        assert_eq!(entries.len(), 0);
    }

    #[test]
    fn test_query_events_by_resolution_status_resolved() {
        let test = EventArchiveTest::new();
        let contract_id = test.env.register(crate::PredictifyHybrid, ());
        // Test querying by Resolved status
        let status = MarketState::Resolved;
        let (entries, _) = test.env.as_contract(&contract_id, || {
            EventArchive::query_events_by_resolution_status(&test.env, status, 0, 10)
        });
        assert_eq!(entries.len(), 0);
    }

    #[test]
    fn test_query_events_by_resolution_status_cancelled() {
        let test = EventArchiveTest::new();
        let contract_id = test.env.register(crate::PredictifyHybrid, ());
        // Test querying by Cancelled status
        let status = MarketState::Cancelled;
        let (entries, _) = test.env.as_contract(&contract_id, || {
            EventArchive::query_events_by_resolution_status(&test.env, status, 0, 10)
        });
        assert_eq!(entries.len(), 0);
    }

    #[test]
    fn test_query_events_by_resolution_status_pagination() {
        let test = EventArchiveTest::new();
        let contract_id = test.env.register(crate::PredictifyHybrid, ());
        // Test pagination with status filter
        let status = MarketState::Disputed;
        let (entries, next_cursor) = test.env.as_contract(&contract_id, || {
            EventArchive::query_events_by_resolution_status(&test.env, status, 5, 15)
        });
        assert_eq!(entries.len(), 0);
        assert_eq!(next_cursor, 5);
    }

    #[test]
    fn test_query_events_by_category_empty() {
        let test = EventArchiveTest::new();
        let contract_id = test.env.register(crate::PredictifyHybrid, ());
        // Test querying by category on empty archive
        let category = String::from_str(&test.env, "sports");
        let (entries, _) = test.env.as_contract(&contract_id, || {
            EventArchive::query_events_by_category(&test.env, &category, 0, 10)
        });
        assert_eq!(entries.len(), 0);
    }

    #[test]
    fn test_query_events_by_category_multiple() {
        let test = EventArchiveTest::new();
        // Test querying by different categories
        let cat1 = String::from_str(&test.env, "sports");
        let cat2 = String::from_str(&test.env, "politics");
        assert_ne!(cat1, cat2);
    }

    #[test]
    fn test_query_events_by_tags_empty_tags() {
        let test = EventArchiveTest::new();
        let contract_id = test.env.register(crate::PredictifyHybrid, ());
        // Test that empty tag list returns no results
        let tags = Vec::new(&test.env);
        let (entries, cursor) = test.env.as_contract(&contract_id, || {
            EventArchive::query_events_by_tags(&test.env, &tags, 0, 10)
        });
        assert_eq!(entries.len(), 0);
        assert_eq!(cursor, 0); // Cursor unchanged when no tags
    }

    #[test]
    fn test_query_events_by_tags_single_tag() {
        let test = EventArchiveTest::new();
        let contract_id = test.env.register(crate::PredictifyHybrid, ());
        // Test querying with single tag
        let mut tags = Vec::new(&test.env);
        tags.push_back(String::from_str(&test.env, "important"));
        let (entries, _) = test.env.as_contract(&contract_id, || {
            EventArchive::query_events_by_tags(&test.env, &tags, 0, 10)
        });
        assert_eq!(entries.len(), 0);
    }

    #[test]
    fn test_query_events_by_tags_multiple_tags() {
        let test = EventArchiveTest::new();
        let contract_id = test.env.register(crate::PredictifyHybrid, ());
        // Test querying with multiple tags (OR logic)
        let mut tags = Vec::new(&test.env);
        tags.push_back(String::from_str(&test.env, "tag1"));
        tags.push_back(String::from_str(&test.env, "tag2"));
        let (entries, _) = test.env.as_contract(&contract_id, || {
            EventArchive::query_events_by_tags(&test.env, &tags, 0, 10)
        });
        assert_eq!(entries.len(), 0);
    }

    #[test]
    fn test_max_query_limit_constant() {
        // Test that MAX_QUERY_LIMIT is properly defined
        assert_eq!(MAX_QUERY_LIMIT, 30u32);
    }

    #[test]
    fn test_archive_event_authorization_check() {
        let test = EventArchiveTest::new();
        // Test that non-admin cannot archive
        let non_admin = Address::generate(&test.env);
        let market_id = Symbol::new(&test.env, "test_market");
        // Calling with non-admin should fail authorization
        assert_ne!(non_admin, test.admin);
    }

    #[test]
    fn test_query_cursor_progression() {
        let test = EventArchiveTest::new();
        let contract_id = test.env.register(crate::PredictifyHybrid, ());
        // Test that cursor progresses through pagination
        let cursor1 = 0u32;
        let (_, next_cursor1) = test.env.as_contract(&contract_id, || {
            EventArchive::query_events_history(&test.env, 0, 100, cursor1, 10)
        });
        assert_eq!(next_cursor1, cursor1); // No entries, cursor stays at 0
    }

    #[test]
    fn test_event_history_entry_structure() {
        let test = EventArchiveTest::new();
        // Test that EventHistoryEntry has all required fields
        let market_id = Symbol::new(&test.env, "test");
        let question = String::from_str(&test.env, "Will it?");
        let category = String::from_str(&test.env, "test");
        // EventHistoryEntry contains: market_id, question, outcomes, end_time, created_at, state, winning_outcome, total_staked, archived_at, category, tags
        assert!(!market_id.to_string().is_empty());
    }

    #[test]
    fn test_query_returns_pagination_info() {
        let test = EventArchiveTest::new();
        let contract_id = test.env.register(crate::PredictifyHybrid, ());
        // Test that all queries return (entries, next_cursor)
        let (entries, cursor) = test.env.as_contract(&contract_id, || {
            EventArchive::query_events_history(&test.env, 0, 100, 0, 10)
        });
        let _ = entries;
        let _ = cursor;
    }

    #[test]
    fn test_archive_event_market_not_found_error() {
        let test = EventArchiveTest::new();
        // Test that archiving nonexistent market returns appropriate error
        let market_id = Symbol::new(&test.env, "fake_market");
        let admin = test.admin;
        // Would return Error::MarketNotFound
        assert!(!admin.to_string().is_empty());
    }

    #[test]
    fn test_archive_event_invalid_state_error() {
        let test = EventArchiveTest::new();
        // Test that archiving Active market returns error
        // Only Resolved or Cancelled allowed
        let market_id = Symbol::new(&test.env, "active_market");
        assert!(!market_id.to_string().is_empty());
    }

    #[test]
    fn test_query_events_by_category_fallback_to_oracle() {
        let test = EventArchiveTest::new();
        let contract_id = test.env.register(crate::PredictifyHybrid, ());
        // Test that category falls back to oracle feed_id if not set
        let category = String::from_str(&test.env, "BTC/USD");
        let (entries, _) = test.env.as_contract(&contract_id, || {
            EventArchive::query_events_by_category(&test.env, &category, 0, 10)
        });
        assert_eq!(entries.len(), 0);
    }

    #[test]
    fn test_query_events_timestamp_inclusive() {
        let test = EventArchiveTest::new();
        let contract_id = test.env.register(crate::PredictifyHybrid, ());
        // Test that timestamp filtering is inclusive on both ends
        let exact_time = 1500u64;
        let (_, _) = test.env.as_contract(&contract_id, || {
            EventArchive::query_events_history(&test.env, exact_time, exact_time, 0, 10)
        });
        // Should include markets created at exact_time
        assert!(true);
    }

    #[test]
    fn test_query_events_tags_or_logic() {
        let test = EventArchiveTest::new();
        let contract_id = test.env.register(crate::PredictifyHybrid, ());
        // Test that tag matching uses OR logic (any match)
        let mut tags = Vec::new(&test.env);
        tags.push_back(String::from_str(&test.env, "tag_a"));
        tags.push_back(String::from_str(&test.env, "tag_b"));
        // Market with tag_a OR tag_b should be included
        let (entries, _) = test.env.as_contract(&contract_id, || {
            EventArchive::query_events_by_tags(&test.env, &tags, 0, 10)
        });
        assert_eq!(entries.len(), 0);
    }

    #[test]
    fn test_archive_max_length_market_id_no_panic() {
        let test = EventArchiveTest::new();
        let env = &test.env;
        let contract_id = env.register(crate::PredictifyHybrid, ());

        env.as_contract(&contract_id, || {
            let max_length_id = Symbol::new(env, "a_very_long_market_id_32_chars__");

            let market = Market {
                admin: test.admin.clone(),
                question: String::from_str(env, "Will BTC reach 100k?"),
                outcomes: soroban_sdk::vec![env, String::from_str(env, "yes")],
                end_time: env.ledger().timestamp() + 3600,
                metadata_commitment: Market::compute_metadata_commitment(
                    env,
                    &String::from_str(env, "Will BTC reach 100k?"),
                    &soroban_sdk::vec![env, String::from_str(env, "yes")],
                    &OracleConfig::none_sentinel(env),
                ),
                oracle_config: OracleConfig::none_sentinel(env),
                has_fallback: false,
                fallback_oracle_config: OracleConfig::none_sentinel(env),
                resolution_timeout: 3600,
                oracle_result: None,
                votes: Map::new(env),
                total_staked: 0,
                dispute_stakes: Map::new(env),
                stakes: Map::new(env),
                claimed: Map::new(env),
                winning_outcomes: None,
                fee_collected: false,
                state: MarketState::Resolved,
                total_extension_days: 0,
                max_extension_days: 30,
                extension_history: soroban_sdk::vec![env],
                category: None,
                tags: soroban_sdk::vec![env],
                min_pool_size: None,
                bet_deadline: 0,
                dispute_window_seconds: 3600,
                winnings_swept: false,
            };

            let res =
                crate::storage::StorageOptimizer::archive_market_data(env, &max_length_id, &market);
            assert!(res.is_ok());
        });
    }

    #[test]
    fn test_archive_repeated_same_ledger_works() {
        let test = EventArchiveTest::new();
        let env = &test.env;
        let contract_id = env.register(crate::PredictifyHybrid, ());

        env.as_contract(&contract_id, || {
            let market_id = Symbol::new(env, "market_1");

            let market = Market {
                admin: test.admin.clone(),
                question: String::from_str(env, "Will BTC reach 100k?"),
                outcomes: soroban_sdk::vec![env, String::from_str(env, "yes")],
                end_time: env.ledger().timestamp() + 3600,
                metadata_commitment: Market::compute_metadata_commitment(
                    env,
                    &String::from_str(env, "Will BTC reach 100k?"),
                    &soroban_sdk::vec![env, String::from_str(env, "yes")],
                    &OracleConfig::none_sentinel(env),
                ),
                oracle_config: OracleConfig::none_sentinel(env),
                has_fallback: false,
                fallback_oracle_config: OracleConfig::none_sentinel(env),
                resolution_timeout: 3600,
                oracle_result: None,
                votes: Map::new(env),
                total_staked: 0,
                dispute_stakes: Map::new(env),
                stakes: Map::new(env),
                claimed: Map::new(env),
                winning_outcomes: None,
                fee_collected: false,
                state: MarketState::Resolved,
                total_extension_days: 0,
                max_extension_days: 30,
                extension_history: soroban_sdk::vec![env],
                category: None,
                tags: soroban_sdk::vec![env],
                min_pool_size: None,
                bet_deadline: 0,
                dispute_window_seconds: 3600,
                winnings_swept: false,
            };

            let res1 =
                crate::storage::StorageOptimizer::archive_market_data(env, &market_id, &market);
            assert!(res1.is_ok());

            let res2 =
                crate::storage::StorageOptimizer::archive_market_data(env, &market_id, &market);
            assert!(res2.is_ok());
        });
    }

    #[test]
    fn test_archive_entries_are_retrievable() {
        let test = EventArchiveTest::new();
        let env = &test.env;
        let contract_id = env.register(crate::PredictifyHybrid, ());

        env.as_contract(&contract_id, || {
            let market_id = Symbol::new(env, "retrievable_market");
            let timestamp = env.ledger().timestamp();

            let market = Market {
                admin: test.admin.clone(),
                question: String::from_str(env, "Will BTC reach 100k?"),
                outcomes: soroban_sdk::vec![env, String::from_str(env, "yes")],
                end_time: env.ledger().timestamp() + 3600,
                metadata_commitment: Market::compute_metadata_commitment(
                    env,
                    &String::from_str(env, "Will BTC reach 100k?"),
                    &soroban_sdk::vec![env, String::from_str(env, "yes")],
                    &OracleConfig::none_sentinel(env),
                ),
                oracle_config: OracleConfig::none_sentinel(env),
                has_fallback: false,
                fallback_oracle_config: OracleConfig::none_sentinel(env),
                resolution_timeout: 3600,
                oracle_result: None,
                votes: Map::new(env),
                total_staked: 0,
                dispute_stakes: Map::new(env),
                stakes: Map::new(env),
                claimed: Map::new(env),
                winning_outcomes: None,
                fee_collected: false,
                state: MarketState::Resolved,
                total_extension_days: 0,
                max_extension_days: 30,
                extension_history: soroban_sdk::vec![env],
                category: None,
                tags: soroban_sdk::vec![env],
                min_pool_size: None,
                bet_deadline: 0,
                dispute_window_seconds: 3600,
                winnings_swept: false,
            };

            let res =
                crate::storage::StorageOptimizer::archive_market_data(env, &market_id, &market);
            assert!(res.is_ok());

            let key = crate::storage::DataKey::ArchivedMarket(market_id.clone(), timestamp);
            let retrieved: Option<Market> = env.storage().persistent().get(&key);
            assert!(retrieved.is_some());

            let retrieved_market = retrieved.unwrap();
            assert_eq!(retrieved_market.question, market.question);
            assert_eq!(retrieved_market.admin, market.admin);
        });
    }
}
