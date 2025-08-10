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

### Operational Playbooks

- If costs climb due to strings: enforce length caps at API layer and/or contract validation.
- If claim/resolve costs spike: batch payouts off-chain via token escrows or staged claims.

