# Storage Tier Matrix

Reference for the storage tier and TTL policy applied to each data key in
`contracts/predictify-hybrid`.

Two orthogonal concepts are documented here, and they are easy to conflate:

- **Durability tier** — the Soroban storage class a key lives in:
  `instance`, `persistent`, or `temporary`. Chosen at the call site via
  `env.storage().instance() / .persistent() / .temporary()`.
- **TTL tier** — for persistent keys only, which rent-extension budget applies.
  Modelled by the private `StorageTtlTier` enum in `storage.rs` and resolved
  through `StorageOptimizer::persistent_ttl_for_tier`.

A key has exactly one durability tier. Only persistent keys have a TTL tier.

## TTL tier definitions

Defaults are declared as constants in `storage.rs` and are overridable at
runtime through the `StorageConfig` fields shown below.

| TTL tier | Constant | Ledgers | ≈ Duration | `StorageConfig` field |
| --- | --- | ---: | --- | --- |
| Balance | `BALANCE_TTL_LEDGERS` | 535,680 | ~31 days | `balance_ttl_ledgers` |
| Market | `MARKET_TTL_LEDGERS` | 6,307,200 | ~365 days | `market_ttl_ledgers` |
| Event | `EVENT_TTL_LEDGERS` | 1,555,200 | ~90 days | `event_ttl_ledgers` |
| Archive | `ARCHIVE_TTL_LEDGERS` | 6,307,200 | ~365 days | `archive_ttl_ledgers` |

All durations assume `LEDGERS_PER_DAY = 17_280` (~5 s/ledger on Soroban
mainnet). Two further TTLs sit outside the tier enum:

| Constant | Ledgers | ≈ Duration | Applies to |
| --- | ---: | --- | --- |
| `PLACE_BETS_IDEM_TTL_LEDGERS` | 120,960 | ~7 days | `PlaceBetsIdem` (temporary) |
| `MARKET_CACHE_TTL_LEDGERS` | 100 | ~8 minutes | `MarketCache` (instance) |

### Clamping

Every persistent write routed through `set_persistent_with_ttl` /
`extend_persistent_ttl` is clamped by `clamp_persistent_ttl`:

```rust
effective_ttl = desired_ttl_ledgers.min(env.storage().max_ttl())
```

The configured value is therefore an upper bound, not a guarantee. On networks
where `max_ttl()` is below the tier constant, the Market and Archive tiers
collapse to the network maximum and become indistinguishable.

`check_market_creation_rent` performs the matching pre-flight check at market
creation, returning `Error::InsufficientStorageRent` if
`ledger().sequence() + effective_ttl` would overflow `u32`.

## Key matrix

Tier assignment is made at each call site rather than by a central
`DataKey -> tier` function, so this table is derived by reading the writes.
See *Known deviations* for the consequences.

| Data key | Durability | TTL tier | Effective TTL | Written by |
| --- | --- | --- | --- | --- |
| `ArchivedMarket(Symbol, u64)` | persistent | Archive | ~365 d | `archive_market_data` |
| `MarketMetadata(Symbol)` | persistent | Market | ~365 d | `promote_market_to_persistent` |
| `MarketScratch(Symbol)` | temporary | — | set at call site | scratch demotion path |
| `MarketCache(Symbol)` | instance | — | ~8 min (shared) | `markets.rs` read cache |
| `MarketExtensionTotal(Symbol)` | persistent | Market | ~365 d | market extension path |
| `Whitelisted(Address)` | persistent | Market | ~365 d | admin access control |
| `Blacklisted(Address)` | persistent | Market | ~365 d | admin access control |
| `AdminOverrideNonce(Address)` | persistent | Market | ~365 d | admin override replay guard |
| `DisputeHistory(Symbol)` | persistent | Market | ~365 d | dispute log |
| `DisputeHistoryCap` | persistent | Market | ~365 d | dispute config |
| `DisputeStakeCap(Symbol, Address)` | persistent | Market | ~365 d | dispute stake accounting |
| `DisputeCumulativeStakeCap(Address)` | persistent | Market | ~365 d | per-user cumulative cap |
| `AntiGriefFloor` | persistent | Market | ~365 d | `disputes.rs` |
| `GlobalConfig` | persistent | Market | ~365 d | governance config |
| `PlaceBetsIdem(Address, BytesN)` | temporary | — | ~7 d | `bets.rs` idempotency guard |

Non-`DataKey` composite keys also carry tier assignments:

| Key shape | Durability | TTL tier | Effective TTL | Written by |
| --- | --- | --- | --- | --- |
| `(Symbol("Event"), Symbol)` | persistent | Event | ~90 d | `EventManager::store_event` |
| `(Symbol("ActiveEvents"), Address)` | persistent | Market | ~365 d | active-event counters |
| `Symbol("storage_config")` | persistent | Archive | ~365 d | `update_storage_config` |
| `Vec<Val>` balance key | persistent | Balance | ~31 d | `BalanceStorage` |
| archive-derived keys (`compressed`, `compressed_ref`) | persistent | Market | ~365 d | compression path |
| migration record (`Symbol`) | persistent | Archive | ~365 d | `store_migration_record` |

## Instance storage note

Soroban instance storage shares a single TTL across all instance keys, and it
is bounded by the contract instance's own lifetime. Bumping any instance key
extends every instance key. `MARKET_CACHE_TTL_LEDGERS` is therefore a floor on
cache freshness, not a per-key expiry — `MarketCache` entries may outlive
~8 minutes if another instance write bumps the shared TTL. Treat instance
storage as a cache that may vanish, never as a source of truth.

## Choosing a tier for a new key

1. Is it a cache, cheaply rebuildable from persistent state? → **instance**.
2. Does it expire on a fixed, short schedule with no audit value?
   → **temporary**, with an explicit TTL constant.
3. Otherwise → **persistent**, and pick the TTL tier by retention need:
   Balance (~31 d) for user balances, Event (~90 d) for event records,
   Market (~365 d) for live market state, Archive (~365 d) for records kept
   for audit or migration.

Route persistent writes through `StorageOptimizer::set_persistent_with_ttl`
with a `persistent_ttl_for_tier` value rather than a raw literal, so the write
picks up both the `StorageConfig` override and the `max_ttl()` clamp.

## Known deviations

These are accurate as of this document and are recorded rather than corrected,
since this change is documentation-only.

1. **`DataKey` does not compile.** `AdminOverrideNonce(Address)` is declared
   twice in `storage.rs` (lines 74 and 89), which is `E0428`. Additionally
   `AntiGriefFloor`, `GlobalConfig`, and `PlaceBetsIdem` are constructed in
   `disputes.rs`, governance tests, and `bets.rs` respectively but are absent
   from the enum. The tiers recorded above for those four keys reflect the
   call sites that use them, not a declaration that currently exists.

2. **`BalanceStorage::update_balance` bypasses the tier helpers.** It calls
   `extend_ttl(&key, 535680, 535680)` with hardcoded literals rather than
   `persistent_ttl_for_tier(env, StorageTtlTier::Balance)`. The literal equals
   `BALANCE_TTL_LEDGERS` today, so behaviour matches by coincidence, but the
   write ignores any `StorageConfig` override and skips the `max_ttl()` clamp.

3. **No central key-to-tier mapping exists.** `StorageTtlTier` is private and
   there is no `fn tier_for(key: &DataKey)`. Tier assignment lives at roughly
   a dozen call sites, so this table can drift from the code without any test
   failing.

4. **`storage_tier_audit.rs` is unregistered and stale.** The module exists and
   exports `get_storage_tier_audit`, but it is not declared in `lib.rs`, so it
   is dead code that never compiles or runs — its `#[cfg(test)]` tests do not
   execute. Its table also names keys that are not `DataKey` variants
   (`Admin`, `Market`, `DisputeMultiSig`, `GovernanceMinBps`, `CumDisputeFee`,
   `PlatformFee`, `OracleConfidence`, `AdminEmergency`) and omits most keys
   that are. Where it disagrees with this document, this document reflects the
   code. Reconciling or removing that module is left to a follow-up.

## References

- `contracts/predictify-hybrid/src/storage.rs` — tier constants, `StorageConfig`, TTL helpers
- `contracts/predictify-hybrid/src/storage_layout_tests.rs` — TTL behaviour tests
- `contracts/predictify-hybrid/src/storage_tier_audit.rs` — unregistered prior audit (see deviation 4)
- `contracts/predictify-hybrid/src/bets.rs` — `PlaceBetsIdem` lifecycle
- `contracts/predictify-hybrid/src/markets.rs` — `MarketCache` read cache