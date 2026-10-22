# Contract Capabilities


The Predictify Hybrid contract exposes a **u64 capabilities bitmap** that allows
clients to discover which features are available without inspecting the Wasm
binary or relying on version-number heuristics.

## Entrypoint

```rust
fn capabilities(env: Env) -> u64
```

This is a **pure read** — it performs no storage writes and emits no events. It
is safe to invoke at any time on any network (testnet, mainnet, future).

## Return Value

A `u64` where each set bit represents an active contract capability. Clients
test for a specific capability by masking with a bitwise AND against the
constants defined in the [`capability`] module.

### Client-side example (Rust/Soroban SDK)

```rust
let caps = client.capabilities();
if caps & 0x0001 != 0 {
    // versioning is supported
}
```

### Client-side example (TypeScript/Stellar SDK)

```typescript
const caps: bigint = contract.capabilities();
if ((caps & 0x0001n) !== 0n) {
    // versioning is supported
}
```

## Bit Assignments

| Bit | Mask (hex)         | Capability                  | Description |
|-----|--------------------|-----------------------------|-------------|
|  0  | `0x0000_0000_0001` | `VERSIONING`                | Contract version tracking and history |
|  1  | `0x0000_0000_0002` | `UPGRADE_MANAGEMENT`        | Upgrade management including migration support |
|  2  | `0x0000_0000_0004` | `QUERY_FUNCTIONS`           | Public read-only query functions |
|  3  | `0x0000_0000_0008` | `MARKET_MANAGEMENT`         | Market creation and lifecycle management |
|  4  | `0x0000_0000_0010` | `BETTING`                   | Bet placement, cancellation, and analytics |
|  5  | `0x0000_0000_0020` | `DISPUTES`                  | Dispute filing, voting, and resolution |
|  6  | `0x0000_0000_0040` | `ORACLE_INTEGRATION`        | Oracle price-feed integration (Reflector, Pyth) |
|  7  | `0x0000_0000_0080` | `GOVERNANCE`                | On-chain governance proposals and voting |
|  8  | `0x0000_0000_0100` | `ANALYTICS`                 | Platform analytics, statistics, leaderboards |
|  9  | `0x0000_0000_0200` | `MONITORING`                | Health monitoring, alerting, graceful degradation |
| 10  | `0x0000_0000_0400` | `FEE_MANAGEMENT`            | Fee calculation, collection, and withdrawal |
| 11  | `0x0000_0000_0800` | `AUDIT_TRAIL`               | Immutable chained audit trail |
| 12  | `0x0000_0000_1000` | `CIRCUIT_BREAKER`           | Circuit breaker for emergency pausing |
| 13  | `0x0000_0000_2000` | `RATE_LIMITING`             | Per-operation rate limiting |
| 14  | `0x0000_0000_4000` | `EVENT_ARCHIVE`             | Historical event archive with pruning |
| 15  | `0x0000_0000_8000` | `METADATA_COMMITMENT`       | SHA-256 metadata commitment for market integrity |
| 16  | `0x0000_0001_0000` | `BATCH_OPERATIONS`          | Atomic batch operations (multi-bet) |
| 17  | `0x0000_0002_0000` | `RECOVERY`                  | Error recovery and unclaimed winnings sweep |
| 18  | `0x0000_0004_0000` | `MULTI_ADMIN_MULTISIG`      | Multi-admin role delegation and multisig |
| 19  | `0x0000_0008_0000` | `STATISTICS`                | Platform-wide statistics tracking |
| 20  | `0x0000_0010_0000` | `TOKEN_REGISTRY`            | Token registry with allowed-asset enforcement |
| 21  | `0x0000_0020_0000` | `EVENT_VISIBILITY`          | Public/private event visibility with allowlists |
| 22  | `0x0000_0040_0000` | `CLAIM_IDEMPOTENCY`         | Idempotent claim tracking |
| 23  | `0x0000_0080_0000` | `BET_CANCELLATION`          | Bet cancellation with full refund |
| 24  | `0x0000_0100_0000` | `FEE_WITHDRAWAL`            | Admin fee withdrawal with timelock and caps |
| 25  | `0x0000_0200_0000` | `PAYOUT_DISTRIBUTION`       | Automatic payout distribution on resolution |

Bits 26 through 63 are reserved for future capabilities and will read as 0.

## Compatibility

- **Adding a capability** sets a previously-zero bit to 1. This is a
  backward-compatible change.
- **Removing a capability** changes a bit from 1 to 0. This is a breaking
  change and must be accompanied by a major version bump.
- **Renumbering bits** is a breaking change. Bit positions are permanent once
  assigned.

## Testing

```rust
let caps = client.capabilities();
assert!(caps > 0);
assert!(caps & 0x0010 != 0); // betting
assert_eq!(caps & !((1u64 << 26) - 1), 0); // no reserved bits set
```
