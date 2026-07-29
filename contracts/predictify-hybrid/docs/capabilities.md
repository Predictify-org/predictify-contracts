# Capabilities

The Predictify Hybrid contract exposes `capabilities()` as a read-only `u64`
bitmap describing the recovery and storage features supported by the active
Wasm. The call requires no authorization, reads no storage, and emits no
events. Clients should test individual bits instead of comparing the complete
bitmap so future feature additions remain backward-compatible.

## Bitmap layout

Recovery features occupy bits 0-10. Storage features use the following stable
assignments:

| Bit | Constant | Storage feature |
| ---: | --- | --- |
| 11 | `TTL_MANAGEMENT` | TTL extension and storage-rent preflight checks |
| 12 | `DATA_COMPRESSION` | Market-data compression |
| 13 | `DATA_CLEANUP` | Expired or obsolete market-data cleanup |
| 14 | `FORMAT_MIGRATION` | Versioned storage-format migration |
| 15 | `TIER_MIGRATION` | Persistent/temporary tier promotion and demotion |
| 16 | `USAGE_MONITORING` | Storage-usage monitoring and statistics |
| 17 | `LAYOUT_OPTIMIZATION` | Per-market layout optimization |
| 18 | `INTEGRITY_VALIDATION` | Per-market storage integrity validation |
| 19 | `CONFIGURATION` | Storage configuration reads and updates |
| 20 | `COST_ANALYTICS` | Cost, efficiency, and recommendation views |

Bits 21-63 are reserved. Assigned bits are never reused for a different
feature; an unavailable feature is represented by clearing its existing bit.

## Admin cooldown

The actions that can change the active capability surface are protected by a
fixed 3,600-second cooldown:

- `upgrade_contract`
- `rollback_upgrade`

Each action has an independent cooldown clock. A successful upgrade therefore
does not prevent an immediate emergency rollback, while repeated upgrades or
repeated rollbacks are rejected until their own cooldown expires.

The first action is allowed. Only successful actions start the cooldown, and an
action is allowed when the ledger timestamp reaches the exact cooldown
boundary. Timestamp addition uses checked arithmetic.

Calls made during the cooldown return `Error::AdminActionTimelocked` (contract
error code `443`). Existing primary-admin authentication remains required for
both state-changing entrypoints.

Clients can query `get_capabilities_admin_cooldown()` without authorization to
retrieve the fixed duration in seconds.
