# Disputes Fuzz Target

## Overview

This document describes the `cargo-fuzz` target for the disputes subsystem
of the Predictify Hybrid contract.  The target lives at
`contracts/disputes/fuzz/targets/main.rs` and exercises
`predictify_hybrid::disputes::DisputeManager` via a structured byte corpus.

No new public API was introduced.  The fuzz target and edge-case test suite
are **test-only** additions that do not affect the deployed contract.

---

## Package layout

```
contracts/disputes/
├── Cargo.toml                        # [[test]] entries for err_stab and dispute_edge_cases
├── src/lib.rs                        # placeholder (no_std stub)
├── fuzz/
│   ├── Cargo.toml                    # disputes-fuzz crate; [[bin]] main
│   └── targets/
│       └── main.rs                   # cargo-fuzz entry point
└── tests/
    ├── err_stab.rs                   # error-code stability assertions (pre-existing)
    └── dispute_edge_cases.rs         # focused deterministic edge-case tests (new)
```

---

## Running the fuzzer

Requires the nightly toolchain and `cargo-fuzz`:

```bash
cargo install cargo-fuzz          # one-time setup
rustup install nightly

# run with an in-process fuzzer (libFuzzer)
cargo +nightly fuzz run \
  --fuzz-dir contracts/disputes/fuzz \
  main
```

Crashes are stored in `contracts/disputes/fuzz/artifacts/main/`.

---

## Running the focused edge-case tests

These use the standard test harness and do not require nightly:

```bash
cargo test -p disputes --test dispute_edge_cases
```

---

## Fuzz actions

Each byte slice drives a loop of up to six distinct actions:

| Action (byte mod 6) | Description |
|---------------------|-------------|
| 0 — OpenDispute | Calls `DisputeManager::process_dispute` with a fuzz-derived stake |
| 1 — VoteSimple | Calls `vote_on_dispute(user, market, outcome_str, stake)` |
| 2 — VoteExtended | Calls `vote_on_dispute(user, market, dispute_id, bool, stake, reason)` |
| 3 — ResolveDispute | Calls `DisputeManager::resolve_dispute` |
| 4 — AdvanceLedger | Time-travels the test ledger by up to 10 × `DISPUTE_PERIOD_SECS` |
| 5 — SetStakeCap | Injects a per-user stake cap directly into contract storage |

---

## Boundary conditions covered

- Stake amounts: 0, −1, `MIN_DISPUTE_STAKE − 1`, `MIN_DISPUTE_STAKE`, `i128::MAX`
- Market timing: active, ended-but-in-window, past dispute window
- Duplicate disputes by the same user → `AlreadyDisputed`
- Per-user stake cap enforcement → `DisputeStakeCapExceeded`
- Vote by the dispute opener → `DisputerCannotVote`
- Double-voting by the same voter → `DisputeAlreadyVoted`
- Resolution before / after votes are cast
- Arbitrary byte sequences must not panic

---

## Expected error set

The fuzzer accepts only the following errors from dispute entry points.
Any other panic is treated as a crash.

| Variant | Code |
|---------|------|
| `AlreadyDisputed` | 404 |
| `DisputeVoteExpired` | 405 |
| `DisputeVoteDenied` | 406 |
| `DisputeAlreadyVoted` | 407 |
| `DisputeCondNotMet` | 408 |
| `DisputeFeeFailed` | 409 |
| `DisputeError` | 410 |
| `DisputerCannotVote` | 438 |
| `DisputeStakeCapExceeded` | 522 |

General infrastructure errors (`Unauthorized`, `MarketNotFound`, `Overflow`,
etc.) are also permitted when the environment is partially wired during corpus
replay.

---

## Cargo.toml changes

| File | Change |
|------|--------|
| `Cargo.toml` (workspace root) | Added `"contracts/disputes/fuzz"` to `[workspace] members` |
| `contracts/disputes/Cargo.toml` | Added `[[test]] dispute_edge_cases` entry |
| `contracts/disputes/fuzz/Cargo.toml` | Added `arbitrary` dependency; added `[[bin]] main` entry |

---

## Bug fix

Two stray closing braces (`}`) in
`contracts/predictify-hybrid/src/disputes.rs` (lines 2907 and 3053) that
caused an "unexpected closing delimiter" parse error have been removed.
These were orphaned method-close braces duplicated inside the `DisputeUtils`
`impl` block.
