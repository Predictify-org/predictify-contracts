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
