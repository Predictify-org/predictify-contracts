# FUZZ_TARGET_FEES — predictify-hybrid fees fuzzer (cargo-fuzz + libFuzzer)

- **Campaign**: GrantFox FWC26 (Stellar Wave)
- **Issue companion**: `FEES_WORKSPACE_BLOCKERS.md` (workspace prerequisites — must be resolved before any harness code runs)
- **Target crate**: `predictify-hybrid` (package at `contracts/predictify-hybrid/`)
- **Harness home** (after scaffolding resolves): `contracts/predictify-hybrid/fuzz/targets/fees.rs`
- **Sibling harness pattern**: Mirrors `contracts/disputes/fuzz/targets/main.rs` in structure, byte-corpus action dispatch, error-whitelist policy, and `AdvanceLedger` time-travel primitive. No new patterns are introduced by this design.

---

## 1. Purpose

This fuzz target exercises the **fees module** of the Predictify hybrid oracle + prediction-market contract with maximally malformed, boundary-condition, and replay-order inputs. Its job is threefold:

1. Expose arithmetic / boundary bugs in the six numeric fields of `FeeConfig` (platform_fee_percentage, creation_fee, min_fee_amount, max_fee_amount, collection_threshold, plus the bool `fees_enabled`).
2. Hammer the **commit → reveal → (apply/cancel)** state machine with mismatched commitments, double-commits, double-reveals, cross-admin reveal, and time-travel around the `config.apply_eta` window.
3. Verify `collect_fees` / `get_fee_analytics` / `get_fee_withdrawal_schedule` never panic and always return either a whitelisted `Error::*` variant or a sane value, regardless of prior storage corruption or input garbage.

It is a **structure-aware byte-dispatch corpus fuzzer** (not a type-level `Arbitrary` derive fuzzer) so that a single `fuzz_target!(|data: &[u8]|)` loop stays deterministic, corpus-reproducible, and binary-compatible with the disputes harness toolchain.

---

## 2. Environment (prerequisites — unchanged from disputes)

```
rustup toolchain install nightly
cargo +nightly install cargo-fuzz --version "^0.12"
```

Run from repo root after `FEES_WORKSPACE_BLOCKERS.md` checklist is complete:

```
cargo +nightly fuzz run --fuzz-dir contracts/predictify-hybrid/fuzz fees -- \
    -max_len=524288 -rss_limit_mb=4096 -timeout=30 -jobs=16
```

Deterministic-replay of a crashing corpus artifact:

```
cargo +nightly fuzz run --fuzz-dir contracts/predictify-hybrid/fuzz fees \
    path/to/corpus_artifact -- -runs=1
```

These CLI flags are identical to the disputes harness; the two harnesses are intended to be run side-by-side in CI with identical resource budgets.

---

## 3. Harness design

### 3.1 Overall structure (matches disputes/main.rs)

1. `fuzz_target!(|data: &[u8]| {` — outer loop.
2. `let env = Env::default();` with `env.mock_all_auths()` so auth-required entrypoints succeed or fail deterministically with `Error::Unauthorized` on deliberately misused callers.
3. Register the contract once per iteration via `env.register(PredictifyHybrid, ());` and obtain the contract ID with `env.current_contract_address()` for storage reads.
4. Construct a `FixtureState` struct (mirrors disputes `FixtureState`) that owns:
   - `users: Vec<Address>` — 8 predetermined `Address::generate(&env)` users (0 = admin, rest = unprivileged)
   - `markets: Vec<Symbol>` — 8 predetermined `symbol_short!("FA" + u)` markets (0..7)
   - `current_slot: u64` — starts at `MARKETS_LIFETIME_THRESHOLD - 100` (per resolution pattern)
   - `pending_commitment: Option<(BytesN<32>, Address)>` — last active commit phase data so reveal can either match or deliberately mismatch
   - `applied_config_count: u64` — counter for analytics assertions
5. Seed initial state **before** the action loop:
   - Write `env.storage().persistent().set(&Symbol::new(env, "Admin"), &users[0])` so admin gating has a defined target.
   - For each `market_id` in 0..4: write a minimal stub market record (status `Created`, total_staked = 100, `winning_outcomes = Some([yes])`, resolved flag = true) so `collect_fees` has a lookup target.
   - Optionally seed a default `FeeConfig` (platform_fee_percentage = 100 i.e. 1.00%, all other fields at sensible nonzero defaults) so the first action can be a reveal-apply without a commit-reveal first.
6. Enter the action loop: `for byte in data.chunks_exact(CHUNK_SIZE)` (chunk size = 16 bytes, same as disputes to keep uniform decoder across harnesses). Each chunk is 1 action opcode byte + 15 bytes payload per Section 3.2.
7. Error policy: every `let result = action(…); match result { Ok(_) => …, Err(e) => if !is_allowed_fee_error(&e) { panic!("unexpected error: {:?}", e); } }`. Any error not in the whitelist (Section 3.4) is a crash.
8. Every 16th action, run `AdvanceLedger` by up to `2^32 - 1` slots (action 7, Section 3.2).
9. Action loop terminates when `data` is exhausted. After the loop, read all 6 fee storage keys (Section 5) with `env.storage().persistent().get(…)`; no get may panic.

### 3.2 Action corpus byte dispatch (8 actions)

| Opcode (byte mod 8) | Name | Payload layout (15 payload bytes) | Decodes to | Domain: what entrypoint / internal fn |
|---|---|---|---|---|
| 0 | `SetPlatformFee` | `user_idx: u8` (1 B), `fee_raw: [u8; 8]` (8 B), `pad: [u8; 6]` | `admin = users[user_idx % 8]`, `fee_percentage: i128 = i64::from_le_bytes(fee_raw) as i128` | `env.as_contract(&cid, &env.current_contract_address(), || set_platform_fee(&env, admin, fee_percentage))` |
| 1 | `CommitFeeConfig` | `user_idx: u8` (1 B), `hash_raw: [u8; 11]` (11 B), `pad: [u8; 3]` | `admin = users[user_idx]`, `hash: BytesN<32> = sha256(concat(hash_raw, 21 zero bytes))` so corpus 11-byte prefix controls the preimage | `commit_fee_config(&env, admin, hash)` — save `(hash, admin)` into `FixtureState.pending_commitment` |
| 2 | `RevealFeeConfig` | `user_idx: u8`, `match_flag: u8` (0 = reveal a BytesN that hashes to the committed one → valid reveal; non-zero = garbage bytes → invalid reveal), then 6 × 2-byte i128 fields (raw): `platform`, `creation`, `min_amt`, `max_amt`, `coll_thresh` (10 B), `flags: u8` (low bit = `fees_enabled`), `pad: [u8; 1]` | Decodes 6 `i16::from_le_bytes → i128`. If `match_flag == 0`, write the commitment for this config into `pending_commitment` before calling reveal so it will match. Otherwise leave last commitment intact. | `reveal_fee_config(&env, admin, FeeConfig { 6 numeric + 1 bool decoded })` |
| 3 | `CollectFees` | `user_idx: u8` (1 B), `market_idx: u8` (1 B), `pad: [u8; 13]` | `caller = users[user_idx]`, `m_id = markets[market_idx]`. Market 8..=255 (modulo) maps to `symbol_short!("ZZ")` — a nonexistent market. | `collect_fees(&env, caller, m_id)` — exercises admin-vs-nonadmin, existing vs non-existent market, unresolved market (fixture state 0..3 resolved, 4..7 unresolved). |
| 4 | `GetFeeAnalytics` | `discriminant: u8` (1 B), `pad: [u8; 14]` | `discriminant mod 4` → `TimeFrame::Hour \| Day \| Week \| All`. Higher discriminants (4..=255) transmute via raw repr — for enums without `#[repr(u8)]` Soroban may panic on raw transmute; the harness uses `TimeFrame::try_from(discriminant).unwrap_or(TimeFrame::All)` and counts any raw-repr panic as a valid fuzzer crash if contract-level guards don't catch it first. | `get_fee_analytics(&env, tf)` |
| 5 | `GetFeeWithdrawalSchedule` | `pad: [u8; 15]` (ignored) | No parameters — pure read. | `get_fee_withdrawal_schedule(&env)` — storage can be empty/malformed. Must not panic. |
| 6 | `ApplyOrCancelQueued` | `user_idx: u8` (1 B), `opcode_flag: u8` (0 = cancel, 1 = apply, else invalid → admin op with flag still passed), `pad: [u8; 13]` | `admin = users[user_idx]`, `if opcode_flag == 0 → cancel_fee_update(env, admin) else if opcode_flag == 1 → apply_fee_update(env, admin)`. Run after several Reveal actions so queued state is hit. | `FeeManager::cancel_fee_update(env, admin)` / `FeeManager::apply_fee_update(env, admin)` (lib.rs admin-gated public wrappers, Section 2.3) |
| 7 | `AdvanceLedger` | `delta_raw: [u8; 8]` (big-endian → u64), `pad: [u8; 7]` | `delta = u64::from_be_bytes(delta_raw) % (u32::MAX as u64 + 1)` (so fits in a ledger timestamp). `env.ledger().with_mut(\|l\| l.timestamp += delta; l.number += (delta/5) as u32); fixture.current_slot += delta;` | No entrypoint — time-travel primitive so corpus can reach `apply_eta` windows and TTL expiry for fee keys. |

Total: 8 actions. Payload size is **always 15 bytes per action** so corpus chunks are uniform (disputes harness uses same uniform chunk size for binary reproducibility).

---

## 4. Fixture state model

Identical to disputes `FixtureState` in structure, fees-specific fields are new:

```
FixtureState {
    users:        [Address; 8],   // index 0 = Admin, 1..=7 = unprivileged
    markets:      [Symbol; 8],    // index 0..=3 = Resolved markets, 4..=7 = Created (unresolved)
    pending_commit: Option<(BytesN<32>, Address)>,  // last CommitFeeConfig (action 1) output
    queued_configs: Vec<(FeeConfig, Address, apply_eta: u64)>,  // parallel in-fixture shadow of the
                                  // contract's actual storage, for delta assertions
    analytics_sum: i128,          // running sum of analytics returns, monotonicity assertion
    slot:         u64,            // mirror of env.ledger().timestamp
}
```

The `queued_configs` parallel shadow is important: it's how the harness asserts that `apply_fee_update` returns a config **structurally equal** to the config the fuzzer wrote earlier via `reveal_fee_config`. Any mismatch → panic → reproducible crash.

---

## 5. Storage keys exercised

Derived from call sites and fee module conventions (same prefix pattern as `("res_out", market_id)` for resolution, plus the Symbol-namespaced globals used in `Admin` / `global_min_pool`):

| Key (conceptual) | Written by action(s) | Read by action(s) | TTL tier (same tier bumped on hot read) |
|---|---|---|---|
| `Symbol::new("Admin")` → `Address` | Fixture seed, never mutated in the loop | Action 0, 1, 2, 3, 6 — all admin-gated | Global |
| `Symbol::new("FeeCurrent")` → `FeeConfig` | Action 0 (set_platform_fee mutates percentage), Action 6 apply_fee_update | Action 4 (analytics), Action 3 collect_fees → FeeConfigManager::get_fee_config | Market |
| `Symbol::new("FeeQueued")` → `(FeeConfig, apply_eta: u64)` | Action 2 reveal (success path) | Action 6 apply/cancel | Market |
| `Symbol::new("FeeCommit")` → `(BytesN<32>, from: Address)` | Action 1 commit | Action 2 reveal | Market |
| `("FeeCollected", market_id)` → `i128` cumulative | Action 3 collect_fees (success path) | Action 3 collect_fees on re-entry (to detect double-dip) | Market |
| `Symbol::new("FeeSchedule")` → `FeeWithdrawalSchedule` | Fixture seed (optional) | Action 5 get_fee_withdrawal_schedule | Global |
| Market records `(Symbol)` → `Market` struct | Fixture seed | Action 3 collect_fees → looks up market_id | Market |

---

## 6. Error whitelist

`is_allowed_fee_error(e: &Error) -> bool` — any `Err` variant not in this set is an **unexpected crash** the fuzzer reports:

### 6.1 Fee-coded-specific errors (explicit from err.rs / call-site documentation)

| Variant | When it's allowed |
|---|---|
| `Error::InvalidFeeConfig` (code 402) | Action 0 (range 0..=1000 reject), Action 2 (reveal numeric-range reject on any of the 6 fields), Action 6 cancel when nothing queued / apply when `slot < apply_eta` |
| `Error::MarketNotFound` (code 101) | Action 3 when `market_idx` points to a market symbol not seeded → all 4..=255 % cases should land here |
| `Error::Unauthorized` (code 401) | Any action 0, 1, 2, 3, 6 where `user_idx` is not 0 (the admin) — intentional as fuzz space coverage of non-admin callers |
| `Error::MarketNotResolved` (code 115) | Action 3 when target market is one of fixture state index 4..=7 (still `Created`) |
| `Error::NoPendingUpdate` / `Error::CommitmentMismatch` (or equivalent) — if these variants don't exist in err.rs yet, the harness maps them to `InvalidFeeConfig` for the purposes of the whitelist until dedicated codes land | Action 2 (commitment mismatch on reveal), Action 6 (cancel when nothing queued) |
| `Error::TimeFrameInvalid` / equivalent | Action 4 bad discriminant path — if contract-level `TimeFrame` validation returns a specific code, add it; for now any of the standard contract validation error codes (100..=199, 400..=499) are allowed. |

### 6.2 Generic infrastructure errors (always allowed, same set as disputes harness)

- `Error::MarketAlreadyResolved` (fixture can re-trigger collect on already resolved)
- `Error::DisputeAlreadyExists` / `Error::DisputeNotPending` (cross-module)
- Any `Overflow` / `Underflow` arithmetic codes from err.rs (overflow-safe policy — these indicate a correctly-caught math edge case, not a panic)
- `Error::TtlBumpRequired` if the contract returns it explicitly, though the fuzzer will typically seed TTLs with enough runway that this isn't reached

### 6.3 What NEVER returns an error — zero-tolerance assertions

If the contract ever panics / aborts / returns a non-`Err` unwrap on these paths, the fuzzer treats it as a crash regardless of error whitelist:

- Action 5 `get_fee_withdrawal_schedule` on empty storage → returns a default struct, never `unwrap()`s.
- `FeeConfigManager::get_fee_config(env)` in hot read path (bets.rs L366) → `Option<FeeConfig>`, callers must handle `None` by fallthrough to default config.
- `FeeManager::get_fee_percentage_for_timestamp(env, ts)` for extreme timestamps → `Result<i128, Error>`, never `unwrap!()`.

---

## 7. Targeted entrypoints

| ID | Entrypoint / function | Opcode coverage | Storage keys touched (Section 5) |
|---|---|---|---|
| F-EP1 | `set_platform_fee` | Action 0 | `FeeCurrent` |
| F-EP2 | `commit_fee_config` | Action 1 | `FeeCommit` |
| F-EP3 | `reveal_fee_config` / `FeeManager::update_fee_config` | Action 2 | `FeeQueued`, `FeeCommit` (consumed on match) |
| F-EP4 | `FeeManager::apply_fee_update` | Action 6 (opcode_flag=1) | `FeeQueued` → consumed, `FeeCurrent` → overwritten |
| F-EP5 | `FeeManager::cancel_fee_update` | Action 6 (opcode_flag=0) | `FeeQueued` → consumed, `FeeCurrent` unchanged |
| F-EP6 | `FeeManager::collect_fees` (entrypoint `collect_fees`) | Action 3 | `("FeeCollected", m_id)`, `FeeCurrent` (read for math) |
| F-EP7 | `get_fee_analytics` | Action 4 | `FeeCurrent`, `FeeCollected` aggregate |
| F-EP8 | `get_fee_withdrawal_schedule` / `FeeWithdrawalManager::get_schedule` | Action 5 | `FeeSchedule` |
| F-EP9 | `FeeManager::get_fee_percentage_for_timestamp` (internal) | Action 4 + Action 3 (indirect, via collect math) | `FeeCurrent` / `FeeQueued` timestamp-aware switch |

Total: 9 targets, all reachable via the 8 actions (Action 6 covers two variants = EP4 + EP5).

---

## 8. Security invariants (crash the fuzzer if violated)

These are **post-action assertions** encoded directly in the harness — not in the contract. Each one expresses what "correct fee module behavior" must mean, independent of contract Error coverage.

1. **Fee percentages stay in range**: After any successful `set_platform_fee` or `apply_fee_update`, `FeeCurrent.platform_fee_percentage` ∈ `[0, 1000]` (0–10%). Reading it via `FeeConfigManager::get_fee_config(env)` → `Some(cfg)` and the check passes, or harness panics (→ crash).
2. **Monotonic collected totals**: For any fixed `market_id`, two consecutive successful `collect_fees(env, admin, market_id)` calls — the second one must return 0 (already collected) OR return a value ≤ first. Negative cumulative deltas → crash.
3. **Commit-reveal structural match**: After a commit(Hash(H)) → reveal(Cfg) where Cfg hashes to H and apply succeeds, the resulting `FeeCurrent.platform_fee_percentage` must equal `Cfg.platform_fee_percentage` exactly (struct field comparison via parallel FixtureState). Mismatch → crash.
4. **No-panic reads**: At end of every 16th action, read all 6 storage keys (Section 5) with `storage().persistent().get::<_, RawVal>(&key)`. Any `unwrap()`-style panic from the contract on corrupted storage → crash.
5. **`Err -> no state change`**: For any action that returns `Err(e)` where `e` is on the whitelist, compare the sha256 hash of (all 6 storage keys serialized) before and after. If they differ → crash (a whitelisted Error must not mutate state). This is the single most important invariant of the whole harness and is **identical to the disputes harness invariant #3** (mutations-on-error detection).

---

## 9. Coverage goals

Target: **95% line coverage on the `mod fees;` source** once scaffolding delivers it. Initial-corpus strategies to reach that fast:

1. Seed the corpus with 16 hand-crafted `.corpus` files (16 actions each = 256 bytes) — one for each path:
   - C1: `set_platform_fee(500)` → success (admin).
   - C2: `set_platform_fee(-1)` → InvalidFeeConfig.
   - C3: `set_platform_fee(1001)` → InvalidFeeConfig.
   - C4: `commit(valid_hash)` → `reveal(matching)` → `AdvanceLedger(delta past eta)` → `apply` → success.
   - C5: `commit(H)` → `reveal(non-matching config)` → CommitmentMismatch.
   - C6: `reveal` without prior `commit` → No pending.
   - C7: `apply` before `eta` → InvalidFeeConfig (early apply).
   - C8: non-admin Action 3 collect → Unauthorized.
   - C9: collect on unresolved market → MarketNotResolved.
   - C10: collect on nonexistent market → MarketNotFound.
   - C11: collect twice on same resolved market → second returns 0 or `already-collected` error.
   - C12: `cancel` nothing queued → error.
   - C13: commit → apply (cancel in between) → error.
   - C14: `get_fee_analytics` for TimeFrame::Hour with empty storage → default analytics struct.
   - C15: `get_fee_withdrawal_schedule` on empty storage → default schedule, never panics.
   - C16: cycle Set → Reveal → Apply → Collect → Analytics → Schedule → AdvanceLedger, all in one, for round-trip.
2. Coverage tool: `cargo +nightly fuzz coverage --fuzz-dir contracts/predictify-hybrid/fuzz fees` (same cmd as disputes harness — doc parity).

---

## 10. Typical failure cases

Any of these outcomes when reproducing a corpus artifact is a real bug to fix before merge:

| Category | Example | Severity |
|---|---|---|
| **Arithmetic** | Overflow in `fee = pool_size * percentage / 10_000` when `percentage` was stored above 1000 by a buggy apply → `TryFromIntError` / `panic` | High |
| **State machine breakage** | Cancel after Apply → still lets config get applied; or Reveal twice with same commitment → writes two queued entries with same apply_eta → double-withdraw on final collect | Critical |
| **Unauthorized access** | Non-admin user (fixture idx ≥1) calls `reveal` with a pre-committed admin hash → commitment check skipped → writes queued config without admin privilege | Critical |
| **Time-travel bug** | `apply_eta` check uses `env.ledger().timestamp` but `reveal` stored it against `env.ledger().sequence` → mismatch → config never applies or applies instantly | High |
| **Storage corruption panic** | `get_fee_withdrawal_schedule` reads schedule from storage with `.unwrap()` on a malformed Bytes → Abort | High |
| **Cross-contract invariant break** | `collect_fees` writes cumulative i128 but then next `get_fee_analytics` aggregates via u128 and underflows on negative fees (fee refund edge case) → Arithmetic Error outside whitelist → crash → Medium severity, real bug |
| **TTL expiry bug** | If a hot read of `FeeCurrent` forgets its TTL bump (see resolution bug), `global_min_pool`-style expiry → silent default-to-zero → test harness can't catch it but coverage-guided mutation of `AdvanceLedger` delta to huge values can; add a "TTL-stress" seed corpus case with slot jump = 3× threshold + immediate collect/read | Medium |

---

## 11. Relationship to existing test modules (once they exist)

The harness is intentionally **orthogonal to** (not a replacement for) the fees test modules declared in `src/tests/mod.rs`:

| Test module (declared) | Coverage style | Relation to fuzzer |
|---|---|---|
| `fee_calculator_proptest` | `proptest!` over numeric inputs → deterministic property checks of pure math | Fuzzer exercises the **storage + commit-reveal state machine** layer on top of the same calculator, with arbitrary action order. Proptest unit tests should be the place for tight numeric invariants; the fuzzer for cross-action stateful invariants. |
| `fee_config_commit_reveal_tests` | Sequential happy-path + sad-path tests of commit → reveal → apply → cancel | Fuzzer discovers arbitrary interleavings with time travel that these ordered tests would never write manually (commit → commit → cancel → reveal → cancel → …). |

Run `cargo test` first; deliver a green `cargo test` before launching fuzzing runs, per repo guidelines.
