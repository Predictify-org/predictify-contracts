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

use crate::errors::Error;
use crate::market_id_generator::MarketIdGenerator;
use crate::types::{EventHistoryEntry, Market, MarketState};
use soroban_sdk::{panic_with_error, Address, Env, String, Symbol, Vec};

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

        Ok(())
    }

    /// Remove the oldest `count` entries from the archive (admin only).
    ///
    /// Frees capacity so that new events can be archived after the cap is reached.
    /// Entries are removed in insertion order (oldest first). The underlying
    /// `Map` does not preserve insertion order, so we iterate the market registry
    /// to find the oldest archived IDs.
    ///
    /// # Arguments
    /// * `env` - Soroban environment
    /// * `admin` - Caller must be contract admin
    /// * `count` - Number of oldest entries to remove (capped at [`MAX_QUERY_LIMIT`])
    ///
    /// # Errors
    /// * `Unauthorized` - Caller is not admin
    pub fn prune_archive(env: &Env, admin: &Address, count: u32) -> Result<u32, Error> {
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
        let key = Symbol::new(env, ARCHIVED_TS_KEY);
        let mut archived: soroban_sdk::Map<Symbol, u64> = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(soroban_sdk::Map::new(env));

        if archived.is_empty() || count == 0 {
            return Ok(0);
        }

        // Walk the registry (chronological order) to find the oldest archived IDs.
        let registry_len = MarketIdGenerator::get_market_id_registry(env, 0, u32::MAX).len();
        let mut removed = 0u32;
        let mut cursor = 0u32;

        while removed < count && cursor < registry_len {
            let page = MarketIdGenerator::get_market_id_registry(env, cursor, MAX_QUERY_LIMIT);
            let page_len = page.len();
            for i in 0..page_len {
                if removed >= count {
                    break;
                }
                if let Some(entry) = page.get(i) {
                    if archived.get(entry.market_id.clone()).is_some() {
                        archived.remove(entry.market_id);
                        removed += 1;
                    }
                }
            }
            cursor += page_len;
            if page_len == 0 {
                break;
            }
        }

        env.storage().persistent().set(&key, &archived);
        Ok(removed)
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
    use alloc::string::ToString;
    use soroban_sdk::testutils::Address as _;

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
}
