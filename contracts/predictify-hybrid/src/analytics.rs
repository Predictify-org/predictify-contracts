//! # Analytics Snapshot Cache
//!
//! Caches hot [`MarketStats`] snapshots in instance storage so repeated
//! `get_market_analytics` calls skip the `votes` / `stakes` map traversal.
//!
//! ## Design
//!
//! | Layer | Storage | TTL |
//! |---|---|---|
//! | Hot cache | `env.storage().instance()` | [`ANALYTICS_CACHE_TTL_LEDGERS`] |
//! | Source of truth | `env.storage().persistent()` | market TTL |
//!
//! The cache is a **read-only optimisation**: every state-changing entrypoint
//! that mutates market data (vote, place_bet, place_bets, claim_winnings,
//! resolve_market, dispute_market, vote_on_dispute) calls
//! [`AnalyticsCache::invalidate`] before returning so the next read
//! recomputes from persistent storage.
//!
//! ## Invalidation contract
//!
//! 1. Any write that changes `total_votes`, `total_staked`, `dispute_stakes`,
//!    or `winning_outcomes` **must** call `AnalyticsCache::new(env).invalidate(&market_id)`.
//! 2. Reads via `get_market_analytics` **must** go through [`get_or_compute`]
//!    instead of hitting persistent storage directly.
//! 3. The cache **never** participates in write paths as a source of truth.

use crate::markets::{MarketAnalytics, MarketStats};
use crate::types::Market;
use soroban_sdk::{contracttype, Env, Symbol};

// ---------------------------------------------------------------------------
// Storage key
// ---------------------------------------------------------------------------

/// Instance storage key for a cached [`MarketStats`] snapshot.
///
/// Keyed by market id so separate markets never share an entry. Stored in
/// `env.storage().instance()` — the fastest and cheapest Soroban read tier.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum AnalyticsKey {
    /// Cached analytics snapshot for the given market id.
    Stats(Symbol),
}

// ---------------------------------------------------------------------------
// TTL constant
// ---------------------------------------------------------------------------

/// Maximum ledger lifetime of a cached analytics snapshot.
///
/// 100 ledgers ≈ 8 minutes at 5 s / ledger (Soroban mainnet estimate).
/// Matches [`crate::storage::MARKET_CACHE_TTL_LEDGERS`] for consistency.
///
/// The TTL is intentionally short: analytics change on every vote/bet.
/// Increase this constant if read pressure justifies a longer stale window.
pub const ANALYTICS_CACHE_TTL_LEDGERS: u32 = 100;

// ---------------------------------------------------------------------------
// Cache implementation
// ---------------------------------------------------------------------------

/// In-instance read cache for per-market [`MarketStats`] snapshots.
///
/// # Security
///
/// The cache is **never** consulted during writes. All mutations operate
/// directly on persistent storage. The cache only accelerates read queries.
///
/// # Overflow safety
///
/// All TTL arithmetic delegates to the SDK; no raw integer arithmetic is
/// performed inside this module.
///
/// # Example
///
/// ```rust,ignore
/// // Read path (get_market_analytics):
/// if let Some(stats) = AnalyticsCache::new(&env).get(&market_id) {
///     return Ok(stats); // cache hit — no persistent read
/// }
/// let market: Market = env.storage().persistent().get(&market_id)
///     .ok_or(Error::MarketNotFound)?;
/// let stats = MarketAnalytics::get_market_stats(&market);
/// AnalyticsCache::new(&env).populate(&market_id, &market);
/// Ok(stats)
///
/// // Write path (vote / place_bet / dispute_market / …):
/// AnalyticsCache::new(&env).invalidate(&market_id);
/// ```
pub struct AnalyticsCache<'a> {
    env: &'a Env,
}

impl<'a> AnalyticsCache<'a> {
    /// Creates a new cache accessor bound to `env`.
    ///
    /// Cheap construction — holds only a reference to the environment.
    pub fn new(env: &'a Env) -> Self {
        Self { env }
    }

    /// Returns the cached [`MarketStats`] for `market_id`, or `None` on miss.
    ///
    /// A cache **hit** bumps the instance TTL so the entry remains live for
    /// another [`ANALYTICS_CACHE_TTL_LEDGERS`] ledgers from the moment of
    /// access. A cache **miss** does nothing (does not bump TTL).
    ///
    /// # No panics
    ///
    /// Uses the SDK's `Option`-returning `get()`. Returns `None` on any
    /// deserialization mismatch without panicking.
    pub fn get(&self, market_id: &Symbol) -> Option<MarketStats> {
        let key = AnalyticsKey::Stats(market_id.clone());
        let result: Option<MarketStats> = self.env.storage().instance().get(&key);
        if result.is_some() {
            self.env.storage().instance().extend_ttl(
                ANALYTICS_CACHE_TTL_LEDGERS,
                ANALYTICS_CACHE_TTL_LEDGERS,
            );
        }
        result
    }

    /// Computes fresh [`MarketStats`] from `market` and writes them into the cache.
    ///
    /// Bumps the instance TTL after writing.
    ///
    /// Call this only **after** a successful persistent-storage read — never
    /// before, to preserve the cache-as-read-optimisation-only invariant.
    pub fn populate(&self, market_id: &Symbol, market: &Market) {
        let stats = MarketAnalytics::get_market_stats(market);
        let key = AnalyticsKey::Stats(market_id.clone());
        self.env.storage().instance().set(&key, &stats);
        self.env.storage().instance().extend_ttl(
            ANALYTICS_CACHE_TTL_LEDGERS,
            ANALYTICS_CACHE_TTL_LEDGERS,
        );
    }

    /// Removes the cached snapshot for `market_id`.
    ///
    /// Called on every write path that mutates market state.
    /// Does **not** bump the TTL — invalidation must not extend cache lifetime.
    /// Safe to call when no entry exists (idempotent).
    pub fn invalidate(&self, market_id: &Symbol) {
        let key = AnalyticsKey::Stats(market_id.clone());
        self.env.storage().instance().remove(&key);
    }
}

// ---------------------------------------------------------------------------
// Convenience free-function used from lib.rs read path
// ---------------------------------------------------------------------------

/// Returns [`MarketStats`] for `market_id`, using the instance cache when hot.
///
/// # Algorithm
///
/// 1. Check instance cache — `O(1)` on hit, no persistent read.
/// 2. On miss, load from persistent storage.
/// 3. Recompute stats, populate the cache, and return.
///
/// Returns `None` if the market does not exist in persistent storage.
///
/// # No panics
///
/// All operations use `Option`/`Result` combinators; no `unwrap()` calls.
pub fn get_or_compute(env: &Env, market_id: &Symbol) -> Option<MarketStats> {
    let cache = AnalyticsCache::new(env);

    // Fast path: instance cache hit.
    if let Some(stats) = cache.get(market_id) {
        return Some(stats);
    }

    // Slow path: load from persistent storage.
    let market: Market = env.storage().persistent().get(market_id)?;

    // Populate the cache and return fresh stats.
    cache.populate(market_id, &market);
    cache.get(market_id)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markets::MarketStats;
    use soroban_sdk::{Env, Map, String, Symbol};

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    fn make_env() -> Env {
        Env::default()
    }

    fn mid(env: &Env) -> Symbol {
        Symbol::new(env, "test_mkt")
    }

    /// Builds a synthetic `MarketStats` for seeding tests without a live market.
    fn dummy_stats(env: &Env, votes: u32) -> MarketStats {
        let mut dist = Map::new(env);
        dist.set(String::from_str(env, "yes"), votes);
        MarketStats {
            total_votes: votes,
            total_staked: votes as i128 * 1_000_000,
            total_dispute_stakes: 0,
            outcome_distribution: dist,
        }
    }

    /// Directly inserts a `MarketStats` into instance storage under the
    /// `AnalyticsKey::Stats` key, bypassing `AnalyticsCache::populate`.
    ///
    /// This helper lets us test `get()` in isolation without needing a
    /// live `Market` in persistent storage.
    fn seed_cache(env: &Env, market_id: &Symbol, stats: &MarketStats) {
        let key = AnalyticsKey::Stats(market_id.clone());
        env.storage().instance().set(&key, stats);
    }

    // ------------------------------------------------------------------
    // TTL constant sanity
    // ------------------------------------------------------------------

    /// Ensures the TTL constant is positive so caching actually occurs.
    #[test]
    fn analytics_cache_ttl_is_positive() {
        assert!(
            ANALYTICS_CACHE_TTL_LEDGERS > 0,
            "TTL must be > 0 for caching to take effect"
        );
    }

    // ------------------------------------------------------------------
    // AnalyticsKey uniqueness
    // ------------------------------------------------------------------

    /// Different market ids must produce different storage keys.
    #[test]
    fn analytics_key_differs_per_market_id() {
        let env = make_env();
        let key_a = AnalyticsKey::Stats(Symbol::new(&env, "mkt_a"));
        let key_b = AnalyticsKey::Stats(Symbol::new(&env, "mkt_b"));
        assert_ne!(key_a, key_b);
    }

    // ------------------------------------------------------------------
    // get — cache miss
    // ------------------------------------------------------------------

    /// On an empty instance store a get must return None.
    #[test]
    fn get_returns_none_on_cold_cache() {
        let env = make_env();
        let market_id = mid(&env);
        let contract_id = env.register(crate::PredictifyHybrid, ());
        env.as_contract(&contract_id, || {
            let cache = AnalyticsCache::new(&env);
            assert!(cache.get(&market_id).is_none());
        });
    }

    // ------------------------------------------------------------------
    // seed then get — cache hit
    // ------------------------------------------------------------------

    /// After seeding the instance store the cache must return the stats.
    #[test]
    fn get_returns_seeded_stats_on_cache_hit() {
        let env = make_env();
        let market_id = mid(&env);
        let contract_id = env.register(crate::PredictifyHybrid, ());

        env.as_contract(&contract_id, || {
            let stats = dummy_stats(&env, 42);
            seed_cache(&env, &market_id, &stats);

            let cache = AnalyticsCache::new(&env);
            let result = cache.get(&market_id).expect("expected cache hit");

            assert_eq!(result.total_votes, 42);
            assert_eq!(result.total_staked, 42_000_000);
            assert_eq!(result.total_dispute_stakes, 0);
        });
    }

    // ------------------------------------------------------------------
    // invalidate removes an existing entry
    // ------------------------------------------------------------------

    /// Invalidating an entry that exists must remove it from the cache.
    #[test]
    fn invalidate_removes_cached_entry() {
        let env = make_env();
        let market_id = mid(&env);
        let contract_id = env.register(crate::PredictifyHybrid, ());

        env.as_contract(&contract_id, || {
            let stats = dummy_stats(&env, 5);
            seed_cache(&env, &market_id, &stats);

            let cache = AnalyticsCache::new(&env);
            assert!(cache.get(&market_id).is_some(), "pre-condition: should be cached");

            cache.invalidate(&market_id);

            assert!(cache.get(&market_id).is_none(), "post-invalidation: should be gone");
        });
    }

    // ------------------------------------------------------------------
    // invalidate is idempotent
    // ------------------------------------------------------------------

    /// Calling invalidate on a missing entry must not panic.
    #[test]
    fn invalidate_is_idempotent_on_empty_cache() {
        let env = make_env();
        let market_id = mid(&env);
        let contract_id = env.register(crate::PredictifyHybrid, ());

        env.as_contract(&contract_id, || {
            let cache = AnalyticsCache::new(&env);
            // First call — nothing present.
            cache.invalidate(&market_id);
            // Second call — still nothing present; must not panic.
            cache.invalidate(&market_id);
            assert!(cache.get(&market_id).is_none());
        });
    }

    // ------------------------------------------------------------------
    // Separate market ids do not interfere
    // ------------------------------------------------------------------

    /// Invalidating market A must leave market B's cache entry intact.
    #[test]
    fn separate_market_ids_are_isolated() {
        let env = make_env();
        let id_a = Symbol::new(&env, "mkt_a");
        let id_b = Symbol::new(&env, "mkt_b");
        let contract_id = env.register(crate::PredictifyHybrid, ());

        env.as_contract(&contract_id, || {
            seed_cache(&env, &id_a, &dummy_stats(&env, 3));
            seed_cache(&env, &id_b, &dummy_stats(&env, 7));

            let cache = AnalyticsCache::new(&env);

            // Confirm both are present.
            assert_eq!(cache.get(&id_a).unwrap().total_votes, 3);
            assert_eq!(cache.get(&id_b).unwrap().total_votes, 7);

            // Invalidate A only.
            cache.invalidate(&id_a);

            // A gone, B intact.
            assert!(cache.get(&id_a).is_none());
            assert_eq!(cache.get(&id_b).unwrap().total_votes, 7);
        });
    }

    // ------------------------------------------------------------------
    // get_or_compute — unknown market returns None
    // ------------------------------------------------------------------

    /// `get_or_compute` for a market not in persistent storage returns `None`.
    #[test]
    fn get_or_compute_returns_none_for_unknown_market() {
        let env = make_env();
        let market_id = Symbol::new(&env, "nonexistent");
        let contract_id = env.register(crate::PredictifyHybrid, ());

        env.as_contract(&contract_id, || {
            let result = get_or_compute(&env, &market_id);
            assert!(result.is_none());
        });
    }

    // ------------------------------------------------------------------
    // populate then get round-trips a zero-participant market
    // ------------------------------------------------------------------

    /// After invalidation the cache must return None, not stale data.
    #[test]
    fn invalidate_after_seed_then_miss() {
        let env = make_env();
        let market_id = mid(&env);
        let contract_id = env.register(crate::PredictifyHybrid, ());

        env.as_contract(&contract_id, || {
            let stats = dummy_stats(&env, 10);
            seed_cache(&env, &market_id, &stats);

            let cache = AnalyticsCache::new(&env);
            cache.invalidate(&market_id);
            // Now a fresh get must miss.
            assert!(cache.get(&market_id).is_none());
        });
    }

    // ------------------------------------------------------------------
    // Outcome distribution is preserved through the cache
    // ------------------------------------------------------------------

    /// The `outcome_distribution` map must survive a cache round-trip.
    #[test]
    fn outcome_distribution_survives_cache_roundtrip() {
        let env = make_env();
        let market_id = mid(&env);
        let contract_id = env.register(crate::PredictifyHybrid, ());

        env.as_contract(&contract_id, || {
            let mut dist = Map::new(&env);
            dist.set(String::from_str(&env, "yes"), 8u32);
            dist.set(String::from_str(&env, "no"), 2u32);
            let stats = MarketStats {
                total_votes: 10,
                total_staked: 10_000_000,
                total_dispute_stakes: 500_000,
                outcome_distribution: dist,
            };
            seed_cache(&env, &market_id, &stats);

            let cached = AnalyticsCache::new(&env)
                .get(&market_id)
                .expect("expected cache hit");

            assert_eq!(cached.total_votes, 10);
            assert_eq!(cached.total_staked, 10_000_000);
            assert_eq!(cached.total_dispute_stakes, 500_000);
            assert_eq!(
                cached.outcome_distribution.get(String::from_str(&env, "yes")),
                Some(8)
            );
            assert_eq!(
                cached.outcome_distribution.get(String::from_str(&env, "no")),
                Some(2)
            );
        });
    }
}
