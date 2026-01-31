//! Event archive and historical query support.
//!
//! Provides archiving of resolved/cancelled events (markets) and gas-efficient,
//! paginated historical query functions for analytics and UI. Exposes only
//! public metadata and outcome; no sensitive data (votes, stakes, addresses).

use crate::errors::Error;
use crate::market_id_generator::MarketIdGenerator;
use crate::types::{EventHistoryEntry, Market, MarketState};
use soroban_sdk::{panic_with_error, Address, Env, String, Symbol, Vec};

/// Maximum number of events returned per query (gas safety).
pub const MAX_QUERY_LIMIT: u32 = 30;

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
    /// * `MarketNotEligibleForArchive` - Market must be Resolved or Cancelled
    /// * `AlreadyArchived` - Event is already archived
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

        let now = env.ledger().timestamp();
        archived.set(market_id.clone(), now);
        env.storage().persistent().set(&key, &archived);

        Ok(())
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
            winning_outcomes: market.winning_outcomes.clone(),
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
