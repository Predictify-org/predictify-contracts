# Markets gas snapshots (v7)

Per-entrypoint CPU / memory baselines for the `markets` contract.

## Run

```bash
cargo test -p markets --test gas_snap -- --nocapture
```

Each test prints:

```text
[gas_snap:v7] <entrypoint> cpu=<N> mem=<M>
```

Include that output in the PR for issue #919.

## Schema

- Snapshot version: **7** (`GAS_SNAP_VERSION`)
- Measured host metrics: `cpu_instruction_cost`, `memory_bytes_cost`
- Package path: `contracts/markets/tests/gas_snap.rs`

## Visible API

New workspace package `markets` with lifecycle entrypoints
(`initialize`, `create_market`, `vote`, `resolve_market`, `claim_winnings`,
`get_market`, `get_stake`, `gas_snap_version`). This is a focused harness used
for regression baselines; the full PredictifyHybrid surface remains in
`predictify-hybrid`.
