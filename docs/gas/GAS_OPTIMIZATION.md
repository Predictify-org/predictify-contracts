## Gas Optimization Guide (Soroban on Stellar)

This guide explains how to write and maintain gas-efficient Soroban contracts in this repository, with concrete recommendations mapped to `predictify-hybrid`.

- Audience: Contract developers and reviewers
- Targets: Soroban SDK 22.x; built with wasm32v1-none

### Key Facts (Resource Limits & Fees)

- Max per-tx: 100M CPU instructions, 40 MB memory
- Ledger access limits: 40 reads, 25 writes; 200 KB read bytes; ~129 KB write bytes
- Fee highlights (stroops):
  - 10,000 instructions: 25
  - Read 1 ledger entry: 6,250; Write 1 ledger entry: 10,000
  - Read 1 KB: 1,786; Write 1 KB: ~11,800
  - Events+return value: 10,000 per KB (up to 8 KB total)
  - Bandwidth: 1,624/KB, History archival: 16,235/KB

Reference: "Resource Limits & Fees" in Stellar docs.

### Golden Rules

- Prefer computation over storage. Reads/writes dominate costs; batch and cache in-memory.
- Read once, write once. Accumulate updates in memory, then persist once at end.
- Avoid per-iteration storage access inside loops. Pull state once, work in `Vec`/`Map`, write once.
- Keep data narrow. Use `Symbol`, `BytesN`, and compact enums/keys; avoid long `String` values.
- Emit events for audit-only data; store only what must be read on-chain later.
- Minimize cross-contract calls. They expand footprint, auth, and costs; batch where feasible.
- Validate inputs early and fail fast. Guard clauses save CPU and storage.
- Use fixed-size math and checked ops where possible; avoid unnecessary big-int math.
- Favor `Vec`/`Map` keyed by compact enums over wide maps with long keys.
- Keep return values small; event+return budget is capped at 8 KB.

### Patterns for Soroban

- Use `env.storage().persistent()` for durable state; consider `temporary()` for short-lived, re-creatable data.
- For lists, keep per-address collections keyed by an enum data key, not one giant vector of structs.
- Bundle external token/oracle transfers: one total transfer into the contract, then internal distributions.
- Avoid growing WASM linear memory repeatedly (e.g., large heap vec); pre-size or use small batches.

### Contract-Specific Hotspots

- `vote` and staking accrual: Favor in-memory aggregation; avoid repeated map lookups/sets.
- `claim_winnings`: Compute totals in-stream and avoid re-reading maps repeatedly; short-circuit losers early.
- `create_market`: Validate and compute once; store a compact `Market` struct; avoid overlong strings.
- Oracle resolution: Keep payloads compact, validate staleness and confidence before persisting.

### Data Layout Recommendations

- Keys: Use `Symbol`-based keys or small enums for storage keys.
- Strings: Restrict question/outcome lengths; validate length to prevent excess write bytes.
- Maps: Avoid nested maps when a single flat map of compact keys suffices.

### Events vs Storage

- Emit events for analytics/telemetry and off-chain consumption.
- Store only state needed for on-chain reads (e.g., current totals, winner, claims bitmap/flags).

### Build & Profile Tips

- Use `profile.release` with `opt-level = "z"`, `lto = true`, `panic = "abort"` (already configured).
- Run cost simulations with CLI `--cost` and RPC `simulateTransaction` before submitting.
- Keep function return values and emitted events small.

### Safe Math

- Keep `overflow-checks = true` (already set). Prefer `checked_*` for user-driven arithmetic.
- Normalize precision early (e.g., cents) and avoid repeated scaling.

### Storage TTL and Rent

- Prefer temporary storage for short-lived data; extend TTL intentionally for persistent data.
- Avoid frequent size growth of entries; growing entries triggers higher rent top-ups.

### Code Review Checklist (Gas)

- Are storage reads/writes minimized and batched?
- Any loops calling storage or cross-contract functions per iteration?
- Are keys and values compact? Any unbounded strings or vectors?
- Are external calls minimized, batched, and gated by pre-checks?
- Do functions fail early on invalid inputs to save CPU/storage?
- Are events used instead of storage where on-chain reads aren’t required?

### References

- Stellar Docs: Analyzing smart contract cost and efficiency
- Stellar Docs: Resource Limits & Fees

