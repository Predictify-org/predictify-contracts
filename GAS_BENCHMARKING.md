## Gas Cost Benchmarking Procedures

Goal: produce reproducible cost metrics per entrypoint across typical scenarios and catch regressions.

### Tools

- Stellar CLI (`stellar`) with `--cost`
- RPC simulateTransaction (client SDKs)

### Build

```bash
stellar contract build
```

### Local Simulation (recommended)

- Use `stellar contract invoke --cost` (or `tx simulate`) to print execution cost breakdown before submit.
- For each function, craft inputs for small/medium/large cases.

Example (pseudocode; replace ids/args):

```bash
# Simulate vote cost
stellar contract invoke --id $CONTRACT_ID \
  --network futurenet --cost -- \
  vote --user $USER --market-id market_1 --outcome Yes --stake 1000
```

Capture output (instructions, ledger read/write counts, bytes) into `benchmarks/results/*.csv`.

### RPC Simulation (programmatic)

- Use SDKs to build a tx that invokes the function and call `simulateTransaction`.
- Record `resourceFee`, `cpuInsns`, `readBytes`, `writeBytes`, `readEntries`, `writeEntries`, and events/return sizes.

### Scenarios to Benchmark

- create_market: short vs long question/outcomes
- vote: single voter; 100 voters; 1,000 voters
- claim_winnings: winner vs loser; large market iteration
- resolve_market: with/without oracle result, with disputes
- fetch_oracle_result: Reflector vs Pyth paths
- collect_fees: resolved vs unresolved

### WASM Size Optimization

```bash
stellar contract optimize --wasm target/wasm32v1-none/release/predictify_hybrid.wasm
```

Track optimized size and ensure below network limits.

### Reporting

- Commit CSVs and a short summary per release under `benchmarks/`.
- Update `GAS_COST_ANALYSIS.md` with highlights (e.g., hot paths, bytes drivers).

