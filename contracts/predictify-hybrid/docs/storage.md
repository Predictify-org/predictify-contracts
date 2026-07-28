# Recovery Module — Storage Keys

## Overview

All recovery storage keys use the **Persistent** tier. The recovery
module does not use Instance or Temporary storage; every key must outlive
individual contract calls and survive across ledger upgrades.

---

## Admin Key (shared)

| Key | Type | Tier | TTL | Rationale |
|-----|------|------|-----|-----------|
| `Symbol("Admin")` | `Address` | Persistent | `MARKET_TTL_LEDGERS` | Single admin address governs all privileged recovery actions. Must persist across contract upgrades and be readable in every call. |

---

## Per-Market Recovery Timelock

| Key | Type | Tier | TTL | Rationale |
|-----|------|------|-----|-----------|
| `Symbol("rcv_pending")` | `Map<Symbol, PendingMarketRecovery>` | Persistent | `RECOVERY_TTL_LEDGERS` | Tracks pending recovery requests per market while they await timelock expiry. Must survive between the `initiate` and `execute` calls (up to 7 days). |
| `Symbol("rcv_timelock_cfg")` | `RecoveryTimelockConfig` | Persistent | `RECOVERY_TTL_LEDGERS` | Stores the global timelock delay for recoveries. Read on every recovery flow so the value must always be available. |

---

## Recovery Records & History

| Key | Type | Tier | TTL | Rationale |
|-----|------|------|-----|-----------|
| `Symbol("recovery_records")` | `Map<Symbol, MarketRecovery>` | Persistent | `RECOVERY_TTL_LEDGERS` | Active (unresolved) recovery records per market. Split from history during the v2 migration so that queries for current state are cheap. |
| `Symbol("recovery_status_map")` | `Map<Symbol, String>` | Persistent | `RECOVERY_TTL_LEDGERS` | Quick-lookup status endpoint (`"pending"` / `"recovered"`) without deserialising the full `MarketRecovery` struct. |
| `Symbol("recovery_v2_migrated")` | `bool` | Persistent | `RECOVERY_TTL_LEDGERS` | One-shot migration flag. After the first read that triggers `ensure_migrated()` the legacy map is split and this flag is set to `true` to skip re-migration on every subsequent call. |
| `(Symbol("rcv_hist"), Symbol)` | `Vec<RecoveryHistoryEntry>` | Persistent | `RECOVERY_TTL_LEDGERS` | Per-market completed recovery history, capped at `MAX_RECOVERY_HISTORY_PER_MARKET` (10) entries. Stored separately from active records so that appending history never interferes with the active-map write. |
| `Symbol("recovery_history")` | `Map<Symbol, Vec<RecoveryHistoryEntry>>` | Persistent | `RECOVERY_TTL_LEDGERS` | **Legacy — migration-only.** Holds the combined active-and-completed history before the v2 migration splits it into `recovery_records` (active) and `rcv_hist` (completed). After migration this key is removed and no longer written. Documented for completeness; existing entries from before the upgrade are migrated on first read. |

---

## Unclaimed Winnings Claim Periods

| Key | Type | Tier | TTL | Rationale |
|-----|------|------|-----|-----------|
| `Symbol("claim_period_global")` | `u64` | Persistent | `RECOVERY_TTL_LEDGERS` | Default claim window in seconds (fallback when no per-market override is set). Must outlive individual markets because it applies to all future markets. |
| `Symbol("claim_period_market")` | `Map<Symbol, u64>` | Persistent | `RECOVERY_TTL_LEDGERS` | Per-market claim-period overrides. Markets with a custom claim window get an entry here. |
| `Symbol("claim_window_start")` | `Map<Symbol, u64>` | Persistent | `RECOVERY_TTL_LEDGERS` | Records when each market's claim window started (set once on first resolution). Needed to compute the deadline: `start + effective_claim_period`. |
| `Symbol("treasury_addr")` | `Address` | Persistent | `RECOVERY_TTL_LEDGERS` | Treasury address where swept unclaimed winnings are deposited. Rarely written but must be available on every sweep call. |

---

## TTL Constants

| Constant | Value | Purpose |
|----------|-------|---------|
| `RECOVERY_TTL_LEDGERS` | 365 × 17 280 ≈ 1 year | Maximum TTL assigned to every recovery persistent key. |
| `RECOVERY_LIFETIME_THRESHOLD` | 31 × 17 280 ≈ 31 days | Minimum remaining ledgers before a hot read bumps the key back to `RECOVERY_TTL_LEDGERS`. |

All recovery keys are extended (TTL-bumped) on every hot read path via
`RecoveryStorage::bump_recovery_ttl`, ensuring they stay alive as long
as the contract is actively used.

---

## API Impact

This documentation describes storage internals only. It does not change
the contract's public API. All storage keys are internal implementation
details of the recovery module.
# Storage Layout — `predictify-hybrid`

This document is a reference for every storage key defined in
[`src/storage.rs`](../src/storage.rs) (plus the small number of ad-hoc keys
built by other storage-adjacent managers in that file), the Soroban storage
**tier** each key lives in — Instance, Persistent, or Temporary — and the
rationale for that choice.

Documenting this does not change the contract's public API. It is a reference
for reviewers and future contributors; no entrypoint signatures, storage
layouts, or key encodings are modified by this document.

## Soroban storage tiers, in one paragraph

Soroban gives every contract three storage spaces with different cost and
lifetime characteristics:

- **Instance** — attached to the contract instance itself. Cheapest to read
  and always loaded with the instance, but every instance-storage entry shares
  **one** TTL: bumping any instance key bumps them all. Best for small, hot,
  frequently-read data.
- **Persistent** — the default for durable application state. Has its own
  per-key TTL and rent, survives independently of other keys, and is the right
  choice for anything that must outlive a single hot-cache window or that is
  keyed per-entity (per market, per user, per dispute).
- **Temporary** — cheapest to write, but the entry (and its TTL) can be
  reclaimed by the network once it expires, with no guarantee of recovery.
  Appropriate only for data that is disposable or is itself a cache/staging
  copy of data that also exists (or can be reconstructed) elsewhere.

## `DataKey` — the primary key enum

All of the following are variants of `pub enum DataKey` in `storage.rs`. Tier
and rationale below reflect the tier each variant is *actually* written to at
its call site(s), not just its name.

| Key | Tier | Rationale |
|---|---|---|
| `PlaceBetsIdem(Address, BytesN<32>)` | Persistent | Idempotency guard for batched `place_bets` calls ([`bets.rs`](../src/bets.rs)). Must survive independently of any single request so a replayed batch is reliably rejected even after the original call's working set has left any cache. |
| `Whitelisted(Address)` | *(unused)* | Declared for an allow-list feature; no current call site sets or reads this key. Documented as a placeholder — if wired up, an allow/deny flag checked on most write paths belongs in Persistent (needs to outlive any single transaction and be independently rent-managed per address). |
| `Blacklisted(Address)` | *(unused)* | Same as `Whitelisted` — declared, not yet wired to a call site. |
| `ArchivedMarket(Symbol, u64)` | Persistent | Immutable historical snapshot of a market at a point in time, written by [`StorageOptimizer::archive_market_data`](../src/storage.rs). Uses the **Archive** TTL tier (`archive_ttl_ledgers`, ~365 days by default) since archived records are meant to remain queryable long after the market itself is gone. |
| `MarketExtensionTotal(Symbol)` | Persistent | Cumulative count of deadline-extension days granted to a market ([`lib.rs`](../src/lib.rs)). Needs to persist for the market's entire lifetime to enforce a lifetime extension cap. |
| `MarketMetadata(Symbol)` | **Dual: Persistent or Temporary** | See [Dual-tier keys](#dual-tier-keys-marketmetadata--marketscratch) below — this key is deliberately migrated between tiers as part of the contract's storage-optimization lifecycle. |
| `MarketScratch(Symbol)` | **Dual: Persistent or Temporary** | See [Dual-tier keys](#dual-tier-keys-marketmetadata--marketscratch) below. |
| `DisputeHistoryCap` | Persistent | Global admin-configured cap on retained dispute history entries ([`disputes.rs`](../src/disputes.rs)). A single global config value that must survive indefinitely once set; Instance would be viable too, but Persistent keeps it independent from the shared instance-TTL budget used by the hot [`MarketCache`](#markets---per-market-instance-cache) keys. |
| `DisputeHistory(Symbol)` | Persistent | Per-market list of resolved/expired disputes, capped by `DisputeHistoryCap`. Keyed per market and needs its own long-lived TTL (explicitly bumped to the ~365-day ledger count on every write), independent of any other market's history. |
| `DisputeStakeCap(Symbol, Address)` | Persistent | Per-(market, user) dispute stake cap, read on every dispute-stake validation. Keyed per-entity, so Persistent (with its own TTL/rent) is required; Instance's shared-TTL model doesn't scale to a key with unbounded cardinality. |
| `DisputeCumulativeStakeCap(Address)` | Persistent | Per-user cumulative dispute stake cap **across all markets** ([`disputes.rs`](../src/disputes.rs)). Same per-entity reasoning as `DisputeStakeCap`. |
| `MarketCache(Symbol)` | **Instance** | See [`MarketCache` — a deliberate Instance cache](#marketcache--a-deliberate-instance-cache) below. |
| `AntiGriefFloor` | Persistent | Global minimum dispute-stake floor. A single config value, but grouped with the rest of the dispute-config keys (`DisputeCooldownSeconds`, `DisputeHistoryCap`) which are also Persistent; keeping the whole config family in one tier avoids splitting related admin-config reads/writes across two TTL models. |
| `DisputeCooldownSeconds` | Persistent | Global cooldown between admin actions on disputes. Same rationale as `AntiGriefFloor`. |
| `DisputeAdminLastAction(Symbol)` | Persistent | Per-admin-function-name last-action timestamp used to enforce `DisputeCooldownSeconds`. Keyed per function name (unbounded-ish, low cardinality but still per-entity), so Persistent. |
| `ResolutionCooldownSeconds` | Persistent | Global cooldown between admin actions on resolution ([`resolution.rs`](../src/resolution.rs)). Same config-family rationale as the dispute cooldown keys. |
| `ResolutionAdminLastAction(Symbol)` | Persistent | Per-function-name last-action timestamp for the resolution cooldown, mirroring `DisputeAdminLastAction`. |
| `GlobalConfig` | *(unused)* | Declared for a future global-config record; no current call site. If wired up as a single shared record read on most paths, Instance would be the natural fit (see the `MarketCache` rationale) — but that decision should be revisited once the record's actual read/write frequency is known. |
| `EventNonce(Symbol)` | Persistent | Per-topic replay-protection nonce for events ([`events.rs`](../src/events.rs)). Needs to be durable and independently keyed per topic so a nonce can never be reused even if unrelated topics' data is evicted. |
| `UserStake(Symbol, Address)` | Persistent | Cumulative stake a user has locked in a specific market ([`bets.rs`](../src/bets.rs)). Explicitly extended to the full `MARKET_TTL_LEDGERS` window on every bet — must outlive the market itself for payout/claim accounting. |
| `MaxBetCap` | Persistent | Global per-user maximum cumulative bet cap. A single config value; extended to `MARKET_TTL_LEDGERS` alongside the stake data it bounds. |
| `UserLastBetTime(Symbol, Address)` | *(unused)* | Declared to support a per-(market, user) cool-off timer; no current call site. Per-entity data of this shape would need Persistent once wired up, matching `UserStake`. |
| `CoolOffPeriod` | *(unused)* | Declared for a global cool-off period in seconds; no current call site. |
| `PerMarketCoolOff(Symbol)` | *(unused)* | Declared for a per-market cool-off override; no current call site. |

### Dual-tier keys: `MarketMetadata` / `MarketScratch`

`MarketMetadata` and `MarketScratch` are the one part of this contract's
storage layout that is *intentionally* tier-mobile, implemented in
[`StorageMigration`](../src/storage.rs):

- `StorageMigration::promote_market_to_persistent` moves a market's metadata
  out of **Temporary** storage and into **Persistent** storage (with the
  configured Market TTL), for a market that has graduated from a short-lived
  staging window into one that needs guaranteed durability.
- `StorageMigration::demote_scratch_keys` does the reverse for scratch data:
  it moves working/derived data (`MarketScratch`) from **Persistent** back
  into **Temporary** storage once it's no longer load-bearing, freeing up
  persistent-storage rent for data that no longer needs a durability
  guarantee.

Because the same `DataKey` variant is legitimately written to either tier
depending on where a given market is in its lifecycle, there is no single
"the tier" for these two keys — call sites always check Persistent first (or
Temporary first, depending on direction) and fall back to the other before
concluding the data doesn't exist.

### `MarketCache` — a deliberate Instance cache

`DataKey::MarketCache(Symbol)` is the one `DataKey` variant stored in
**Instance** storage, via `MarketReadCache` in
[`markets.rs`](../src/markets.rs). This is a deliberate trade-off:

- Market reads are among the hottest paths in the contract, so caching the
  most recently read/written `Market` in Instance storage avoids a
  Persistent-storage read (and its associated rent-extension bookkeeping) on
  every hit.
- The cache's TTL (`MARKET_CACHE_TTL_LEDGERS`, ~100 ledgers / ~8 minutes at
  5s/ledger) is intentionally short — Instance TTL is shared across *all*
  Instance keys in the contract, so this cache is sized to avoid forcing
  unrelated Instance data to be bumped more often than necessary, and to
  bound how stale a cache hit can be relative to the Persistent source of
  truth (the authoritative `Market` record always lives in a
  `DataKey::MarketMetadata`/market-registry Persistent entry — the cache is
  never the only copy of the data).

## Ad-hoc keys (not part of the `DataKey` enum)

A few storage-adjacent managers in `storage.rs` build their own keys directly
(`Symbol`s or tuples) rather than adding a `DataKey` variant. They are
documented here because they are still keys `storage.rs` is responsible for:

| Key shape | Built by | Tier | Rationale |
|---|---|---|---|
| `Symbol("storage_config")` | `StorageOptimizer` | Persistent | Global `StorageConfig` record (TTL tiers, cleanup thresholds, compression preference). Uses the Archive TTL tier — a config record that is read infrequently but must never silently expire. |
| `(Symbol("Balance"), Address, ReflectorAsset)` | `BalanceStorage::get_key` | Persistent | Per-(user, asset) deposit balance. Extended using the **Balance** TTL tier (~31 days by default) on every write — shorter-lived than market data since balances are expected to be actively managed (deposited/withdrawn) rather than dormant for a year. |
| `(Symbol("Event"), event_id)` | `EventManager::event_storage_key` | Persistent | Per-event record. Uses the **Event** TTL tier (~90 days by default) — long enough to outlive typical dispute/resolution windows without matching the full one-year Market tier. |
| `(Symbol("ActiveEvents"), Address)` | `CreatorLimitsManager::get_active_events_key` | Persistent | Per-creator count of currently-active events, used to enforce a creator's concurrent-market limit. Uses the **Market** TTL tier since it's directly coupled to how long a creator's markets stay live. |
| Derived archive keys (`compressed`, `compressed_ref` suffixes via `crate::event_archive::derive_archive_key`) | `StorageOptimizer` (compression) | Persistent | Compressed market payloads and the pointer redirecting a market ID to its compressed form. Uses the **Market** TTL tier — a compressed market is a storage-optimized *replacement* for the original record, not a disposable cache, so it must be at least as durable as the record it replaces. |

## Known inconsistency

[`market_analytics.rs`](../src/market_analytics.rs) (the per-market
leaderboard feature) references `DataKey::MarketLeaderboard(Symbol)`, but no
such variant currently exists on the `DataKey` enum in `storage.rs`. This is a
pre-existing gap unrelated to this documentation change — flagged here rather
than silently omitted, since the goal of this document is to accurately
reflect what the implementation does (and, in this one case, what it
currently fails to compile against) rather than what it was intended to do.

## TTL tier reference

The four named TTL tiers referenced throughout this document, and their
default ledger counts (at ~5s/ledger, from `StorageConfig`'s defaults in
`storage.rs`):

| Tier | Default TTL | Used by |
|---|---|---|
| Balance | ~31 days (`BALANCE_TTL_LEDGERS`) | Deposit/withdrawal balances |
| Market | ~365 days (`MARKET_TTL_LEDGERS`) | Live market records, stakes, caps, compressed-market data |
| Event | ~90 days (`EVENT_TTL_LEDGERS`) | Event records |
| Archive | ~365 days (`ARCHIVE_TTL_LEDGERS`) | Archived markets, storage-migration records, the global storage config |

All four are configurable per-deployment via `StorageConfig` and are always
clamped to the network's `env.storage().max_ttl()` before use.
