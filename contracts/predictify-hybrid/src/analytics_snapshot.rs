use crate::err::Error;
use crate::markets::MarketAnalytics;
use crate::types::{Market, MarketState};
use alloc::vec::Vec as StdVec;
use soroban_sdk::{contracttype, xdr::{FromXdr, ToXdr}, Bytes, Env, Map, String, Symbol, Vec};

/// Schema version for per-market analytics snapshots.
pub const ANALYTICS_SNAPSHOT_SCHEMA_VERSION: u32 = 1;

/// Compact per-market analytics payload for off-chain consumers.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketAnalyticsSnapshot {
    /// Market identifier used to correlate the payload with storage.
    pub market_id: Symbol,
    /// Market question for downstream display and debugging.
    pub question: String,
    /// Current market state.
    pub state: MarketState,
    /// Total number of votes cast in this market.
    pub total_votes: u32,
    /// Total stake currently locked in the market.
    pub total_staked: i128,
    /// Total dispute stake currently locked in the market.
    pub total_dispute_stakes: i128,
    /// Outcome vote counts in a deterministic order.
    pub outcome_counts: Vec<OutcomeCount>,
    /// Number of unique participants in the market.
    pub participant_count: u32,
}

/// A single outcome bucket inside a market analytics snapshot.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutcomeCount {
    /// The outcome label.
    pub outcome: String,
    /// The number of votes for that outcome.
    pub count: u32,
}

/// Versioned envelope for per-market analytics snapshots.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnalyticsSnapshotEnvelope {
    /// Schema version of the inner payload.
    pub schema_version: u32,
    /// Ledger timestamp at snapshot creation time.
    pub taken_at: u64,
    /// XDR-encoded bytes of the analytics payload.
    pub payload: Bytes,
}

impl AnalyticsSnapshotEnvelope {
    /// Encode a market analytics snapshot into a versioned envelope.
    pub fn encode(env: &Env, snapshot: &MarketAnalyticsSnapshot) -> Self {
        Self {
            schema_version: ANALYTICS_SNAPSHOT_SCHEMA_VERSION,
            taken_at: env.ledger().timestamp(),
            payload: snapshot.clone().to_xdr(env),
        }
    }

    /// Decode a market analytics snapshot envelope.
    pub fn decode(env: &Env, envelope: &Self) -> Result<MarketAnalyticsSnapshot, Error> {
        if envelope.schema_version != ANALYTICS_SNAPSHOT_SCHEMA_VERSION {
            return Err(Error::InvalidInput);
        }
        MarketAnalyticsSnapshot::from_xdr(env, &envelope.payload).map_err(|_| Error::InvalidInput)
    }
}

/// Manager for deterministic per-market analytics snapshots.
pub struct AnalyticsSnapshotManager;

impl AnalyticsSnapshotManager {
    /// Return the current schema version for this module.
    pub fn schema_version() -> u32 {
        ANALYTICS_SNAPSHOT_SCHEMA_VERSION
    }

    /// Create a deterministic snapshot for a single market.
    pub fn get_snapshot(env: &Env, market_id: Symbol) -> Result<AnalyticsSnapshotEnvelope, Error> {
        let market: Market = env
            .storage()
            .persistent()
            .get(&market_id)
            .ok_or(Error::MarketNotFound)?;

        let stats = MarketAnalytics::get_market_stats(&market);
        let mut outcome_counts = Vec::new(env);
        let mut counts: Map<String, u32> = Map::new(env);

        for (_, outcome) in market.votes.iter() {
            let current = counts.get(outcome.clone()).unwrap_or(0);
            counts.set(outcome.clone(), current + 1);
        }

        let mut ordered: StdVec<(String, u32)> = StdVec::new();
        for (outcome, count) in counts.iter() {
            ordered.push((outcome, count));
        }
        ordered.sort_by(|left, right| left.0.cmp(&right.0));

        for (outcome, count) in ordered {
            outcome_counts.push_back(OutcomeCount { outcome, count });
        }

        let snapshot = MarketAnalyticsSnapshot {
            market_id: market_id.clone(),
            question: market.question.clone(),
            state: market.state.clone(),
            total_votes: stats.total_votes,
            total_staked: stats.total_staked,
            total_dispute_stakes: stats.total_dispute_stakes,
            outcome_counts,
            participant_count: market.votes.len(),
        };

        Ok(AnalyticsSnapshotEnvelope::encode(env, &snapshot))
    }
}
