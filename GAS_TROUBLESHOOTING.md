## Gas Optimization Troubleshooting

### Symptom: Transactions fail with insufficient fee or cost

- Cause: Under-estimated events/return size or large write bytes.
- Fix: Increase refundable events/return budget on client; enforce input size caps.

### Symptom: Cost spikes on claims/resolution

- Cause: O(n) scans over voters; growing maps.
- Fix: Maintain per-outcome aggregates; paginate or stage claims.

### Symptom: Reaching read/write entry limits

- Cause: Too many storage keys touched per call.
- Fix: Normalize layout; merge maps; reduce per-user keys.

### Symptom: Large WASM or linear memory grows

- Cause: Big heap vecs; heavy deps.
- Fix: Pre-size small arrays; remove unused deps; run `stellar contract optimize`.

### Symptom: Cross-contract call errors inflate retries

- Cause: Oracle/Token contract misconfig or network flakiness.
- Fix: Validate addresses and pre-checks; implement fallback path; cache results if acceptable.

