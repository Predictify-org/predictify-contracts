use soroban_sdk::{contracttype, xdr::{FromXdr, ToXdr}, Bytes, Env, Map, String, Symbol, Vec};
use crate::err::Error;
use crate::queries::QueryManager;
use crate::types::{Market, MarketState, MarketPoolQuery};

// ---------------------------------------------------------------------------
// Schema versioning
// ---------------------------------------------------------------------------
//
// Version history
// ---------------
// 1 — Initial layout: `{ total_active_events, total_resolved_events,
//                         total_pool_all_events, total_fees_collected, version }`
//
// Bump this constant and add a new row above whenever `PlatformStats`
// gains, removes, or reorders a field.
pub const SNAPSHOT_SCHEMA_VERSION: u32 = 1;

// ---------------------------------------------------------------------------
// Reporting-local value types
// ---------------------------------------------------------------------------

/// Lightweight summary of one active event, used for paginated listings.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActiveEvent {
    pub id: Symbol,
    pub question: String,
    pub end_time: u64,
    pub total_pool: i128,
}

/// Aggregated platform-wide statistics.
///
/// This is the inner payload of a [`SnapshotEnvelope`].  Increment
/// [`SNAPSHOT_SCHEMA_VERSION`] whenever this struct changes.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlatformStats {
    pub total_active_events: u32,
    pub total_resolved_events: u32,
    pub total_pool_all_events: i128,
    pub total_fees_collected: i128,
    pub version: String,
}

/// Detailed snapshot of a single event.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventSnapshot {
    pub id: Symbol,
    pub question: String,
    pub outcomes: Vec<String>,
    pub state: MarketState,
    pub total_pool: i128,
    pub outcome_pools: Map<String, i128>,
    pub participant_count: u32,
    pub end_time: u64,
}

// ---------------------------------------------------------------------------
// Snapshot Diffing
// ---------------------------------------------------------------------------

/// A snapshot containing multiple event states for offline comparison.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateSnapshot {
    pub events: Map<Symbol, EventSnapshot>,
}

/// A symmetric diff of two `StateSnapshot`s.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnapshotDiff {
    pub changed_markets: Vec<Symbol>,
}

impl StateSnapshot {
    /// Computes a symmetric difference between two `StateSnapshot`s.
    /// Returns a deterministic, ordered list of market IDs whose snapshots differ
    /// (either because they are in one but not the other, or their values differ).
    pub fn diff(env: &Env, a: &Self, b: &Self) -> SnapshotDiff {
        let mut unique_keys: Map<Symbol, ()> = Map::new(env);
        
        for key in a.events.keys().into_iter() {
            unique_keys.set(key.clone(), ());
        }
        for key in b.events.keys().into_iter() {
            unique_keys.set(key.clone(), ());
        }
        
        let mut changed = Vec::new(env);
        for key in unique_keys.keys().into_iter() {
            let val_a = a.events.get(key.clone());
            let val_b = b.events.get(key.clone());
            if val_a != val_b {
                changed.push_back(key);
            }
        }
        
        SnapshotDiff {
            changed_markets: changed,
        }
    }
}


// ---------------------------------------------------------------------------
// SnapshotEnvelope
// ---------------------------------------------------------------------------

/// Versioned, deterministic envelope for a `ReportingManager` state snapshot.
///
/// # Layout
///
/// ```text
/// SnapshotEnvelope {
///     schema_version : u32   // identifies the inner-struct layout
///     taken_at       : u64   // ledger timestamp at encode time
///     payload        : Bytes // XDR-serialised PlatformStats
/// }
/// ```
///
/// # Version-bump policy
///
/// Increment [`SNAPSHOT_SCHEMA_VERSION`] in `reporting.rs` and add a new row
/// to the "Version history" table every time [`PlatformStats`] (or any type it
/// contains) changes in a backward-incompatible way.  Off-chain consumers MUST
/// inspect `schema_version` before deserialising `payload` so they can apply
/// the correct decoder for older snapshots.
///
/// # Determinism guarantee
///
/// `payload` is produced by [`SnapshotEnvelope::encode`], which calls
/// [`PlatformStats::to_xdr`] — the canonical Soroban XDR serialiser.  The byte
/// stream is reproducible: encoding the same [`PlatformStats`] value in the
/// same environment always yields identical bytes.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnapshotEnvelope {
    /// Layout version of the serialised `payload`.  Clients must check this
    /// before deserialising.
    pub schema_version: u32,
    /// Ledger timestamp (`env.ledger().timestamp()`) at encode time.
    pub taken_at: u64,
    /// XDR-serialised [`PlatformStats`] bytes.
    pub payload: Bytes,
}

impl SnapshotEnvelope {
    /// Serialise `stats` into a versioned [`SnapshotEnvelope`].
    ///
    /// Records the current ledger timestamp and [`SNAPSHOT_SCHEMA_VERSION`].
    /// Uses `PlatformStats::to_xdr` for a deterministic byte layout.
    pub fn encode(env: &Env, stats: &PlatformStats) -> Self {
        Self {
            schema_version: SNAPSHOT_SCHEMA_VERSION,
            taken_at: env.ledger().timestamp(),
            payload: stats.clone().to_xdr(env),
        }
    }

    /// Deserialise a [`SnapshotEnvelope`] back into [`PlatformStats`].
    ///
    /// Returns `Error::InvalidInput` if the `schema_version` is not recognised
    /// by this build or the XDR payload cannot be decoded.
    pub fn decode(env: &Env, envelope: &Self) -> Result<PlatformStats, Error> {
        if envelope.schema_version != SNAPSHOT_SCHEMA_VERSION {
            return Err(Error::InvalidInput);
        }
        PlatformStats::from_xdr(env, &envelope.payload).map_err(|_| Error::InvalidInput)
    }
}

// ---------------------------------------------------------------------------
// ReportingManager
// ---------------------------------------------------------------------------

/// Reporting and Analytics Manager for Predictify Hybrid.
///
/// Provides read-only APIs for retrieving state snapshots of events and
/// platform metrics.  All functions are gas-efficient and expose no private data.
pub struct ReportingManager;

impl ReportingManager {
    /// Retrieve a paginated list of currently active events.
    pub fn get_active_events(env: &Env, offset: u32, limit: u32) -> Result<Vec<ActiveEvent>, Error> {
        let all_markets = QueryManager::get_all_markets(env)?;
        let mut active_events = Vec::new(env);
        let mut skipped = 0u32;
        let mut added = 0u32;

        for id in all_markets.iter() {
            let market: Market = env
                .storage()
                .persistent()
                .get(&id)
                .ok_or(Error::MarketNotFound)?;
            if market.state == MarketState::Active {
                if skipped >= offset {
                    active_events.push_back(ActiveEvent {
                        id: id.clone(),
                        question: market.question.clone(),
                        end_time: market.end_time,
                        total_pool: market.total_staked,
                    });
                    added += 1;
                } else {
                    skipped += 1;
                }
            }
            if added >= limit {
                break;
            }
        }
        Ok(active_events)
    }

    /// Retrieve global platform statistics.
    pub fn get_platform_stats(env: &Env) -> Result<PlatformStats, Error> {
        let state = QueryManager::query_contract_state(env)?;
        Ok(PlatformStats {
            total_active_events: state.active_markets,
            total_resolved_events: state.resolved_markets,
            total_pool_all_events: state.total_value_locked,
            total_fees_collected: state.total_fees_collected,
            version: state.contract_version,
        })
    }

    /// Retrieve a detailed snapshot of a specific event.
    pub fn get_event_snapshot(env: &Env, id: Symbol) -> Result<EventSnapshot, Error> {
        let market: Market = env
            .storage()
            .persistent()
            .get(&id)
            .ok_or(Error::MarketNotFound)?;
        let pool_query: MarketPoolQuery = QueryManager::query_market_pool(env, id.clone())?;
        Ok(EventSnapshot {
            id,
            question: market.question,
            outcomes: market.outcomes,
            state: market.state,
            total_pool: market.total_staked,
            outcome_pools: pool_query.outcome_pools,
            participant_count: market.votes.len(),
            end_time: market.end_time,
        })
    }

    /// Produce a versioned, XDR-stable snapshot of current platform statistics.
    ///
    /// Encodes the current [`PlatformStats`] into a [`SnapshotEnvelope`] tagged
    /// with [`SNAPSHOT_SCHEMA_VERSION`] and the current ledger timestamp.
    /// Off-chain consumers should persist `schema_version` alongside the bytes
    /// so they can select the right decoder when the schema evolves.
    pub fn get_snapshot_envelope(env: &Env) -> Result<SnapshotEnvelope, Error> {
        let stats = Self::get_platform_stats(env)?;
        Ok(SnapshotEnvelope::encode(env, &stats))
    }
}
