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
- Update `../gas/GAS_COST_ANALYSIS.md` with highlights (e.g., hot paths, bytes drivers).


---

## Performance Threshold Constants

The following 18 named constants are defined in
`contracts/predictify-hybrid/src/performance_benchmarks.rs`. They represent conservative
upper bounds derived from mock-delta measurements + headroom. Tighten them to observed
p99 values + 20% once real `stellar contract invoke --cost` measurements are available.

| Constant | Value | Function | Metric | Unit |
|---|---|---|---|---|
| `CREATE_MARKET_GAS_THRESHOLD` | 500,000 | `create_market` | gas usage | instructions |
| `CREATE_MARKET_STORAGE_THRESHOLD` | 2,048 | `create_market` | storage usage | bytes |
| `CREATE_MARKET_TIME_THRESHOLD` | 1,000 | `create_market` | execution time | ms |
| `VOTE_GAS_THRESHOLD` | 200,000 | `vote` | gas usage | instructions |
| `VOTE_STORAGE_THRESHOLD` | 512 | `vote` | storage usage | bytes |
| `VOTE_TIME_THRESHOLD` | 500 | `vote` | execution time | ms |
| `CLAIM_WINNINGS_GAS_THRESHOLD` | 400,000 | `claim_winnings` | gas usage | instructions |
| `CLAIM_WINNINGS_STORAGE_THRESHOLD` | 1,024 | `claim_winnings` | storage usage | bytes |
| `CLAIM_WINNINGS_TIME_THRESHOLD` | 800 | `claim_winnings` | execution time | ms |
| `RESOLVE_MARKET_GAS_THRESHOLD` | 600,000 | `resolve_market` | gas usage | instructions |
| `RESOLVE_MARKET_STORAGE_THRESHOLD` | 2,048 | `resolve_market` | storage usage | bytes |
| `RESOLVE_MARKET_TIME_THRESHOLD` | 1,200 | `resolve_market` | execution time | ms |
| `FETCH_ORACLE_RESULT_GAS_THRESHOLD` | 300,000 | `fetch_oracle_result` | gas usage | instructions |
| `FETCH_ORACLE_RESULT_STORAGE_THRESHOLD` | 256 | `fetch_oracle_result` | storage usage | bytes |
| `FETCH_ORACLE_RESULT_TIME_THRESHOLD` | 600 | `fetch_oracle_result` | execution time | ms |
| `COLLECT_FEES_GAS_THRESHOLD` | 250,000 | `collect_fees` | gas usage | instructions |
| `COLLECT_FEES_STORAGE_THRESHOLD` | 512 | `collect_fees` | storage usage | bytes |
| `COLLECT_FEES_TIME_THRESHOLD` | 500 | `collect_fees` | execution time | ms |

### `default_thresholds()` Constructor

`default_thresholds()` returns a `PerformanceThresholds` instance pre-populated from the
constants above. It uses the highest single-operation values (`resolve_market`) for
`max_gas_usage` and `max_execution_time`, giving a safe envelope for suite-level
validation:

```rust
use predictify_hybrid::performance_benchmarks::{default_thresholds, PerformanceBenchmarkManager};

let thresholds = default_thresholds();
// thresholds.max_gas_usage      == RESOLVE_MARKET_GAS_THRESHOLD      (600_000)
// thresholds.max_execution_time == RESOLVE_MARKET_TIME_THRESHOLD      (1_200)
// thresholds.max_storage_usage  == CREATE_MARKET_STORAGE_THRESHOLD * 100 (204_800)

let within_bounds = PerformanceBenchmarkManager::validate_performance_thresholds(
    &env,
    my_metrics,
    thresholds,
)?;
assert!(within_bounds, "performance regression detected");
```

Integrators can also construct a tighter `PerformanceThresholds` manually using the
per-function constants and pass it to `validate_performance_thresholds` for function-level
assertions.

---

## CI Usage

Run the full benchmark test suite with:

```bash
cargo test -p predictify-hybrid
```

To run only the performance benchmark tests:

```bash
cargo test -p predictify-hybrid performance_benchmarks
```

### Interpreting Pass/Fail Output

- **All tests pass** — every `benchmark_*` result had `gas_usage`, `storage_usage`, and
  `success` within the threshold constants. No regressions detected.
- **A test fails with `assertion failed`** — a measured value exceeded its threshold
  constant. The failing test name indicates which function and metric regressed (e.g.,
  `test_create_market_threshold` failing means `create_market` gas or storage exceeded
  its constant).
- **A property test fails** — `proptest` will print a minimal counterexample. Check
  whether the threshold constant needs updating or whether the implementation regressed.

### Updating Thresholds

When real `stellar contract invoke --cost` measurements are available:

1. Record p99 values for each critical-path function.
2. Add 20% headroom: `new_threshold = ceil(p99 * 1.2)`.
3. Update the corresponding constant in `performance_benchmarks.rs`.
4. Update the table in this document.
5. Re-run `cargo test -p predictify-hybrid` to confirm all tests still pass.
