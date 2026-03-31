## Gas Usage Monitoring and Operations

### Pre-Submit Simulation

- Always simulate and log `--cost` before sending transactions.
- Use RPC `getFeeStats()` to set inclusion fee (p90 recommended under load).

### Metrics to Track

- Distribution of resource fees per function
- Average read/write entries and bytes per function
- Event+return sizes (aim << 8 KB cap)
- Oracle call failure rates and retries

### Alerting

- Spike in write-bytes or write-entries
- Repeated tx failures due to under-estimated event/return size
- Inclusion fee surge vs baseline

### Dashboards

- Per-endpoint cost over time
- Top costly calls and scenarios
- WASM size trend per release

### Indexer Transition Hooks

Critical lifecycle transitions now emit indexer-friendly events via monitoring hooks under a
shared topic prefix:

- Topic root: `idx_transition`
- Domains:
  - `resolution`
  - `dispute`
  - `pause`

Hook payload fields:

- `domain` - transition domain (`Resolution`, `Dispute`, `Pause`)
- `action` - action label (for example `state_transition`, `created`, `resolved`, `paused`, `unpaused`)
- `market_id` - present for resolution/dispute transitions
- `old_state` / `new_state` - present for resolution state transitions
- `actor` - present when an initiating address is known
- `details` - implementation-defined detail string
- `timestamp` - ledger timestamp

This allows indexers to track:

- Resolution transitions such as `Ended -> Resolved` and timeout cancellations
- Dispute lifecycle transitions (`created`, `resolved`)
- Pause lifecycle transitions (`paused`, `unpaused`) from circuit breaker operations

### Operational Playbooks

- If costs climb due to strings: enforce length caps at API layer and/or contract validation.
- If claim/resolve costs spike: batch payouts off-chain via token escrows or staged claims.
