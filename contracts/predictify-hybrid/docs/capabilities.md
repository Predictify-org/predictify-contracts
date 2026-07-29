# Capabilities

The Predictify Hybrid contract exposes `capabilities()` as a read-only `u64`
bitmap describing the recovery features supported by the active Wasm.

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
