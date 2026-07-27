# Storage Tier Matrix

Reference for the storage tier and TTL policy applied to every data key in
`contracts/predictify-hybrid`.

Two orthogonal concepts are documented here:

- **Durability tier** — the Soroban storage class a key lives in:
  `Instance`, `Persistent`, or `Temporary`. Chosen at the call site via
  `env.storage().instance() / .persistent() / .temporary()`.
- **TTL tier** — for `Persistent` keys only, which rent-extension budget
  applies. Modelled by the private `StorageTtlTier` enum in `storage.rs`
  and resolved through `StorageOptimizer::persistent_ttl_for_tier`.

A key has exactly one durability tier. Only persistent keys have a TTL tier.

## Tier summary

| Tier       | Durability | TTL behaviour | Typical use |
|------------|------------|---------------|-------------|
| Instance   | Per-contract-instance | Shared TTL bumped on any instance write | Cheap hot cache; treat as ephemeral |
| Persistent | Per-key (ledger rent) | Extended explicitly; clamped by `max_ttl()` | Long-lived contract state |
| Temporary  | Per-key (short-lived) | Auto-deleted when TTL expires | Scratch data, idempotency guards |

## TTL tier definitions

Defaults are declared as constants in `storage.rs` and overridable at runtime
through the `StorageConfig` fields shown below.

| TTL tier | Constant | Ledgers | ≈ Duration | `StorageConfig` field |
| --- | --- | ---: | --- | --- |
| Balance | `BALANCE_TTL_LEDGERS` | 535,680 | ~31 days | `balance_ttl_ledgers` |
| Market  | `MARKET_TTL_LEDGERS` | 6,307,200 | ~365 days | `market_ttl_ledgers` |
| Event   | `EVENT_TTL_LEDGERS` | 1,555,200 | ~90 days | `event_ttl_ledgers` |
| Archive | `ARCHIVE_TTL_LEDGERS` | 6,307,200 | ~365 days | `archive_ttl_ledgers` |

All durations assume `LEDGERS_PER_DAY = 17_280` (~5 s/ledger on Soroban mainnet).

Two further TTLs sit outside the tier enum:

| Constant | Ledgers | ≈ Duration | Applies to |
| --- | ---: | --- | --- |
| `PLACE_BETS_IDEM_TTL_LEDGERS` | 120,960 | ~7 days | `PlaceBetsIdem` (temporary) |
| `MARKET_CACHE_TTL_LEDGERS` | 100 | ~8 minutes | `MarketCache` (instance) |

### Clamping

Every persistent write routed through `set_persistent_with_ttl` /
`extend_persistent_ttl` is clamped:

```rust
effective_ttl = desired_ttl_ledgers.min(env.storage().max_ttl())
```

The configured value is therefore an upper bound, not a guarantee.

`check_market_creation_rent_budget` performs a pre-flight aggregate check
at market creation, returning `Error::InsufficientStorageRentBudget` when the
current ledger sequence plus the aggregate TTL for all new keys would overflow
`u32`.

## Key matrix — `DataKey` variants

All `DataKey` variants are declared in `storage.rs`. Tier assignment is made
at each call site; this table is derived by reading those call sites.

### Access control

| DataKey variant | Durability | TTL tier | ≈ Duration | Written by |
| --- | --- | --- | --- | --- |
| `Whitelisted(Address)` | Persistent | Market | ~365 d | admin access control |
| `Blacklisted(Address)` | Persistent | Market | ~365 d | admin access control |
| `AdminOverrideNonce(Address)` | Persistent | Market | ~365 d | admin override replay guard |

### Market lifecycle

| DataKey variant | Durability | TTL tier | ≈ Duration | Written by |
| --- | --- | --- | --- | --- |
| `MarketMetadata(Symbol)` | Persistent | Market | ~365 d | `promote_market_to_persistent` |
| `MarketScratch(Symbol)` | Temporary | — | set at call site | scratch demotion path |
| `MarketCache(Symbol)` | Instance | — | ~8 min (shared) | `markets.rs` `MarketReadCache` |
| `MarketExtensionTotal(Symbol)` | Persistent | Market | ~365 d | market extension path |
| `ArchivedMarket(Symbol, u64)` | Persistent | Archive | ~365 d | `StorageOptimizer::archive_market_data` |
| `UserStake(Address, Symbol)` | Persistent | Market | ~365 d | `bets.rs` stake accounting |

### Betting

| DataKey variant | Durability | TTL tier | ≈ Duration | Written by |
| --- | --- | --- | --- | --- |
| `PlaceBetsIdem(Address, BytesN<32>)` | Temporary | — | ~7 d | `bets.rs` idempotency guard |
| `MaxBetCap` | Persistent | Market | ~365 d | `bets.rs` admin cap |

### Disputes

| DataKey variant | Durability | TTL tier | ≈ Duration | Written by |
| --- | --- | --- | --- | --- |
| `DisputeHistory(Symbol)` | Persistent | Market | ~365 d | dispute log |
| `DisputeHistoryCap` | Persistent | Market | ~365 d | dispute config |
| `DisputeStakeCap(Symbol, Address)` | Persistent | Market | ~365 d | per-dispute user cap |
| `DisputeCumulativeStakeCap(Address)` | Persistent | Market | ~365 d | per-user cumulative cap |
| `AntiGriefFloor` | Persistent | Market | ~365 d | `disputes.rs` config |
| `DisputeCooldownSeconds` | Persistent | Market | ~365 d | dispute admin cooldown |
| `DisputeAdminLastAction(Symbol)` | Persistent | Market | ~365 d | dispute admin timestamp |
| `CollusionDetectorConfig` | Persistent | Market | ~365 d | collusion-detector governance config |

### Resolution

| DataKey variant | Durability | TTL tier | ≈ Duration | Written by |
| --- | --- | --- | --- | --- |
| `ResolutionCooldownSeconds` | Persistent | Market | ~365 d | resolution admin cooldown |
| `ResolutionAdminLastAction(Symbol)` | Persistent | Market | ~365 d | resolution admin timestamp |

### Governance / Config

| DataKey variant | Durability | TTL tier | ≈ Duration | Written by |
| --- | --- | --- | --- | --- |
| `GlobalConfig` | Persistent | Market | ~365 d | governance config |

### Rate limiting

| DataKey variant | Durability | TTL tier | ≈ Duration | Written by |
| --- | --- | --- | --- | --- |
| `PerLedgerBetCap` | Persistent | Market | ~365 d | rate limiter admin config |
| `PerLedgerBetCounter` | Persistent | Market | ~365 d | rate limiter rolling counter |

### Admin subsystems

| DataKey variant | Durability | TTL tier | ≈ Duration | Written by |
| --- | --- | --- | --- | --- |
| `OracleAdminCooldownState` | Persistent | Market | ~365 d | oracle admin cooldown |
| `MultisigRotationState` | Persistent | Market | ~365 d | multisig rotation state |

### Events / Nonces

| DataKey variant | Durability | TTL tier | ≈ Duration | Written by |
| --- | --- | --- | --- | --- |
| `EventNonce(Symbol)` | Persistent | Market | ~365 d | event replay-protection nonce |

### Audit trail

| DataKey variant | Durability | TTL tier | ≈ Duration | Written by |
| --- | --- | --- | --- | --- |
| `MarketAuditHead(Symbol)` | Persistent | Market | ~365 d | `audit.rs` — log head record |
| `MarketAuditLog(Symbol, u32)` | Persistent | Market | ~365 d | `audit.rs` — individual log entry |

### Deprecated registry

| DataKey variant | Durability | TTL tier | ≈ Duration | Written by |
| --- | --- | --- | --- | --- |
| `DeprecatedRegistry` | Persistent | Market | ~365 d | `deprecated.rs` |

---

## Key matrix — non-DataKey composite keys

Not all storage keys use `DataKey`. These composite keys are also written by the contract:

| Key shape | Durability | TTL tier | ≈ Duration | Written by |
| --- | --- | --- | --- | --- |
| `Vec<Val>` balance key (`BalanceStorage`) | Persistent | Balance | ~31 d | `BalanceStorage::set_balance` |
| `(Symbol("Event"), Symbol)` | Persistent | Event | ~90 d | `EventManager::store_event` |
| `(Symbol("ActiveEvents"), Address)` | Persistent | Market | ~365 d | `CreatorLimitsManager` |
| `Symbol("storage_config")` | Persistent | Archive | ~365 d | `StorageOptimizer::update_storage_config` |
| Archive-derived keys (`compressed`, `compressed_ref`) | Persistent | Market | ~365 d | `StorageOptimizer` compression path |
| Migration record `Symbol` | Persistent | Archive | ~365 d | `StorageOptimizer::store_migration_record` |

---

## Instance storage note

Soroban instance storage shares a single TTL across all instance keys,
bounded by the contract instance's own lifetime. Bumping any instance key
extends all of them. `MARKET_CACHE_TTL_LEDGERS` is therefore a floor on
cache freshness, not a per-key expiry. Treat instance storage as a cache
that may vanish — never as a source of truth.

## Choosing a tier for a new key

1. Is it a cache, cheaply rebuildable from persistent state? → **Instance**
2. Does it expire on a fixed, short schedule with no audit value?
   → **Temporary**, with an explicit TTL constant.
3. Otherwise → **Persistent**, picking the TTL tier by retention need:
   - Balance (~31 d) for user balances
   - Event (~90 d) for event records
   - Market (~365 d) for live market state and most admin config
   - Archive (~365 d) for audit records or migration state

Route persistent writes through `StorageOptimizer::set_persistent_with_ttl`
with a `persistent_ttl_for_tier` value rather than a raw literal so the write
picks up both the `StorageConfig` override and the `max_ttl()` clamp.

## Testing

Storage tier and TTL behaviour is verified by:

- `src/storage_tier_audit.rs` — audit table tests verifying tiers, completeness, and purity
- `src/storage_layout_tests.rs` — TTL clamping, rent budget checks, and per-tier extension
- `src/place_bets_idempotency_tests.rs` — temporary `PlaceBetsIdem` key lifecycle

## References

- `contracts/predictify-hybrid/src/storage.rs` — tier constants, `DataKey` enum, `StorageConfig`, TTL helpers
- `contracts/predictify-hybrid/src/storage_layout_tests.rs` — TTL behaviour tests
- `contracts/predictify-hybrid/src/storage_tier_audit.rs` — runtime audit table
- `contracts/predictify-hybrid/src/bets.rs` — `PlaceBetsIdem` lifecycle
- `contracts/predictify-hybrid/src/markets.rs` — `MarketCache` read cache
- `contracts/predictify-hybrid/src/audit.rs` — `MarketAuditHead` / `MarketAuditLog`
