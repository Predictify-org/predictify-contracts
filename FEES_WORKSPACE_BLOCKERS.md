# Fees Workspace Scaffolding — Blocking Issue Report

- **Campaign**: GrantFox FWC26 (Stellar Wave)
- **Affected crate(s)**: `predictify-hybrid` (declared module, missing source), `disputes` (crate manifest, missing source), workspace root `Cargo.toml`
- **Severity**: BLOCKING — no `cargo build`, `cargo check`, or `cargo test` command can complete at the workspace level in the current checkout; fees fuzz harness and fees proptests are undeliverable until the items below exist.
- **Status summary**: `mod fees;` and numerous other modules are **declared in lib.rs** but their source files are **not present on disk**. Two workspace-globbed crates (`disputes`, `predictify-hybrid`) are missing their crate manifest or crate root. All four conditions must be resolved before any fees fuzzing work can compile.

---

## 1. Workspace-level build blockers

The workspace root `Cargo.toml` declares `members = ["contracts/*"]`. Both crates enumerated below fail when Cargo resolves the glob.

### 1.1 `contracts/predictify-hybrid/` — crate manifest missing

- **Expected**: `contracts/predictify-hybrid/Cargo.toml` — a `[package]` manifest declaring `name = "predictify-hybrid"`, edition, lib, deps (Soroban SDK + `soroban-sdk-macros` with `testutils`, `rand`, `proptest` with `arbitrary`, similar to the shape of `contracts/oracles/Cargo.toml`).
- **On disk**: `src/` directory exists with substantial `.rs` content, but **no `Cargo.toml` exists**.
- **Blast radius**: The following crates have a **path dependency** on `../predictify-hybrid` and will fail to resolve:
  - `contracts/disputes/Cargo.toml` (line 14)
  - `contracts/disputes/fuzz/Cargo.toml` (line 12)
  - `contracts/oracles/Cargo.toml` (referenced in dev-dependencies section)
  - `contracts/predictify-hybrid/fuzz/Cargo.toml` (line 12)

### 1.2 `contracts/disputes/` — crate root missing

- **Expected**: `contracts/disputes/src/lib.rs` — a crate root matching the path dependency consumed by `contracts/disputes/fuzz/Cargo.toml` line 11 (`disputes = { path = "../../disputes" }`).
- **On disk**: `Cargo.toml`, `fuzz/`, `tests/` exist, but **no `src/` directory exists**. `disputes/Cargo.toml` therefore has no `[lib]` root, which breaks the workspace glob.
- **Precedent**: `contracts/disputes/FUZZ_TARGET.md` Section "Environment" explicitly records this: *"The `contracts/disputes/src/lib.rs` is a thin no_std-compatible stub crate whose only purpose is to export the minimum set of types re-exported by the fuzz harness."* The stub has not yet been added to the working tree.

---

## 2. `mod fees;` declared but no source file

### 2.1 Declaration site

| Location | What is declared | Expected source | On disk? |
|---|---|---|---|
| [`contracts/predictify-hybrid/src/lib.rs` line 34](file:///C:/Users/DELL/Desktop/Drips_contribution/Namiikaze/predictify-contracts/contracts/predictify-hybrid/src/lib.rs#L34) | `mod fees;` | `contracts/predictify-hybrid/src/fees.rs` **or** `contracts/predictify-hybrid/src/fees/mod.rs` | ❌ Neither exists |

### 2.2 Minimum `#[contracttype]` type surface required by cross-references

These types are referenced in other files and must be defined inside `fees.rs` / `fees/mod.rs` for the crate to resolve. Deduplicated list, with evidence:

| # | Type (suggested shape) | Referenced at | Role |
|---|---|---|---|
| T1 | `pub struct FeeManager;` (unit, inherent impl provider) | admin.rs L2530, admin.rs L2542, admin.rs L2552, lib.rs L2536, lib.rs L3082, lib.rs L3100, lib.rs L3105, bets.rs L985 | Namespacing for the 9 fee-related inherent impls used across the codebase |
| T2 | `pub struct FeeConfigManager;` (unit) | bets.rs L366 | Reads current (unexpired TTL-bumped) fee configuration |
| T3 | `#[contracttype] pub struct FeeConfig` — 6 numeric + 1 bool field derived from event definitions: `platform_fee_percentage: i128`, `creation_fee: i128`, `min_fee_amount: i128`, `max_fee_amount: i128`, `collection_threshold: i128`, `fees_enabled: bool` | events.rs L3536–L3541 (event payloads), admin.rs L2530 signature, lib.rs L3104 signature | Canonical commit-reveal payload |
| T4 | `#[contracttype] pub struct FeeWithdrawalSchedule;` (struct or alias) | lib.rs L7308 (entrypoint return type) | Returned by public read endpoint |
| T5 | `pub struct FeeWithdrawalManager;` (unit) | lib.rs L7309 (call site `FeeWithdrawalManager::get_schedule`) | Namespacing for schedule reads |
| T6 | `#[contracttype] pub enum FeeWithdrawalStatus` — at minimum variants `Ready`, `Pending`, `Failed`, `Completed` (derived from event names) | events.rs L173 (used in `FeeWithdrawalAttemptEvent`), event emission sites | Enum payload on withdrawal attempt event |
| T7 | `pub const MARKET_CREATION_FEE: i128 = 10_000_000;` (value matches event emit L1101 precedent) | events.rs L1101 (fee amount in event) | Compile-time constant emitted in `MarketCreatedWithFee` event |

### 2.3 Minimum inherent-impl surface required by cross-references

Every call site that uses `crate::fees::FeeManager::*` or `crate::fees::FeeConfigManager::*`. All of these **must resolve** (either as real impls or as placeholder stubs) before anything compiles:

| Signature | Call site count | Call sites |
|---|---|---|
| `FeeConfigManager::get_fee_config(env: &Env) -> Option<FeeConfig>` (returns Option) | 1 | bets.rs L366 |
| `FeeManager::get_fee_percentage_for_timestamp(env: &Env, ts: u64) -> Result<i128, Error>` | 1 | bets.rs L985 |
| `FeeManager::collect_fees(env: &Env, admin: &Address, market_id: &Symbol) -> Result<i128, Error>` | 1 | lib.rs L2536 |
| `FeeManager::update_fee_config(env: &Env, admin: &Address, cfg: &FeeConfig) -> Result<(), Error>` | 2 | admin.rs L2530, lib.rs L3105 |
| `FeeManager::cancel_fee_update(env: &Env, admin: &Address) -> Result<(), Error>` | 1 | admin.rs L2542 |
| `FeeManager::apply_fee_update(env: &Env, admin: &Address) -> Result<FeeConfig, Error>` | 1 | admin.rs L2552 |
| `FeeManager::commit_fee_config(env: &Env, admin: &Address, hash: BytesN<32>) -> Result<(), Error>` | 1 | lib.rs L3100 |
| `FeeWithdrawalManager::get_schedule(env: &Env) -> FeeWithdrawalSchedule` (struct return) | 1 | lib.rs L7309 |

Additionally, `set_platform_fee(...)` (lib.rs L3076–L3094) is an entrypoint that embeds fee validation inline; it does not call into the `FeeManager` inherent impl, but it must still compile when the surrounding module does.

### 2.4 Public entrypoints in lib.rs that gate on the module

These are already listed in the contract's public interface and are wired to delegate to the module once it exists. Each one must either delegate successfully or be temporarily stubbed:

| # | Entrypoint | Location |
|---|---|---|
| EP1 | `collect_fees(env, admin: Address, market_id: Symbol) -> Result<i128, Error>` | lib.rs L2528 |
| EP2 | `set_platform_fee(env, admin: Address, fee_percentage: i128) -> Result<(), Error>` | lib.rs L3076 |
| EP3 | `commit_fee_config(env, admin: Address, commitment: BytesN<32>) -> Result<(), Error>` | lib.rs L3099 |
| EP4 | `reveal_fee_config(env, admin: Address, new_config: FeeConfig) -> Result<FeeConfig, Error>` | lib.rs L3104 |
| EP5 | `get_fee_analytics(env, tf: TimeFrame) -> Result<FeeAnalytics, Error>` | lib.rs L6166 |
| EP6 | `get_fee_withdrawal_schedule(env) -> FeeWithdrawalSchedule` | lib.rs L7308 |

EP5 returns a separate `FeeAnalytics` type — if that struct lives elsewhere (e.g. analytics.rs, another declared-but-missing module) then an additional stub is required. A grep of the tree for `FeeAnalytics` definition should be done before PR merge; if none exists, the fuzzer will need at least a unit-struct stub of it as well.

---

## 3. Fees test modules declared but missing

Location: [`contracts/predictify-hybrid/src/tests/mod.rs` line 15](file:///C:/Users/DELL/Desktop/Drips_contribution/Namiikaze/predictify-contracts/contracts/predictify-hybrid/src/tests/mod.rs#L15):

```rust
pub mod fee_calculator_proptest;
pub mod fee_config_commit_reveal_tests;
```

| Expected source file | On disk? |
|---|---|
| `contracts/predictify-hybrid/src/tests/fee_calculator_proptest.rs` | ❌ |
| `contracts/predictify-hybrid/src/tests/fee_config_commit_reveal_tests.rs` | ❌ |

Any `cargo test --package predictify-hybrid` invocation fails immediately on mod resolution even if everything else is scaffolded. These two files must exist (even as empty `#[cfg(test)] mod {}` stubs) or their declarations must be removed from `mod.rs`.

---

## 4. Other declared-but-missing modules (collateral damage to a full build)

For completeness, these are the *other* `pub mod X;` declarations in [`lib.rs`](file:///C:/Users/DELL/Desktop/Drips_contribution/Namiikaze/predictify-contracts/contracts/predictify-hybrid/src/lib.rs) that have no corresponding `.rs` or `X/mod.rs` on disk. Fixing `fees.rs` alone will not produce a compiling crate; every one of these must also be stubbed if the project owner wants `cargo check --package predictify-hybrid` to succeed.

| Declared at (line) | `mod` declaration | Expected file pattern | On disk? |
|---|---|---|---|
| L13 | `mod analytics;` | `analytics.rs` or `analytics/mod.rs` | ❌ |
| L16 | `mod audit;` | `audit.rs` or `audit/mod.rs` | ❌ |
| L21 | `mod audit_trail;` | `audit_trail.rs` or `audit_trail/mod.rs` | ❌ |
| L27 | `mod batch_operations;` | `batch_operations.rs` or `batch_operations/mod.rs` | ❌ |
| L31 | `mod capabilities;` | `capabilities.rs` or `capabilities/mod.rs` | ❌ |
| L35 / L71 (duplicate) | `mod config;` | `config.rs` or `config/mod.rs` | ❌ |
| L37 | `mod dispute_multisig;` | `dispute_multisig.rs` or `dispute_multisig/mod.rs` | ❌ |
| L41 | `mod edge_cases;` | `edge_cases.rs` or `edge_cases/mod.rs` | ❌ |
| L42 | `mod extensions;` | `extensions.rs` or `extensions/mod.rs` | ❌ |
| L47 | `mod force_resolve;` | `force_resolve.rs` or `force_resolve/mod.rs` | ❌ |
| L48 | `mod gas;` | `gas.rs` or `gas/mod.rs` | ❌ |
| L53 | `mod governance;` | `governance.rs` or `governance/mod.rs` | ❌ |
| L54 | `mod gov_registry;` | `gov_registry.rs` or `gov_registry/mod.rs` | ❌ |
| L55 | `mod graceful_degradation;` | `graceful_degradation.rs` or `graceful_degradation/mod.rs` | ❌ |
| L56 | `mod lists;` | `lists.rs` or `lists/mod.rs` | ❌ |
| L57 | `mod market_id_generator;` | `market_id_generator.rs` or `market_id_generator/mod.rs` | ❌ |
| L58 | `mod markets;` | `markets.rs` or `markets/mod.rs` | ❌ |
| L60 / L72 (duplicate) | `mod metadata_limits;` | `metadata_limits.rs` or `metadata_limits/mod.rs` | ❌ |
| L61 | `mod monitor;` | `monitor.rs` or `monitor/mod.rs` | ❌ |
| L63 | `mod oracle_health;` | `oracle_health.rs` or `oracle_health/mod.rs` | ❌ |
| L64 | `mod oracles;` | `oracles.rs` or `oracles/mod.rs` | ❌ |
| L67 | `mod performance_benchmarks;` | `performance_benchmarks.rs` or `performance_benchmarks/mod.rs` | ❌ |
| L68 | `mod queries;` | `queries.rs` or `queries/mod.rs` | ❌ |
| L69 | `mod rate_limiter;` | `rate_limiter.rs` or `rate_limiter/mod.rs` | ❌ |
| L70 | `mod reentrancy_guard;` | `reentrancy_guard.rs` or `reentrancy_guard/mod.rs` | ❌ |
| L71 (duplicate) | `mod reporting;` | `reporting.rs` or `reporting/mod.rs` | ❌ |
| L72 (duplicate) | `mod statistics;` | `statistics.rs` or `statistics/mod.rs` | ❌ |
| L73 | `mod storage_tier_audit;` | `storage_tier_audit.rs` or `storage_tier_audit/mod.rs` | ❌ |
| L75 | `mod tokens;` | `tokens.rs` or `tokens/mod.rs` | ❌ |
| L76 | `mod upgrade_manager;` | `upgrade_manager.rs` or `upgrade_manager/mod.rs` | ❌ |
| L78 | `mod versioning;` | `versioning.rs` or `versioning/mod.rs` | ❌ |
| L80 | `mod voting;` | `vot.rs` or `voting/mod.rs` | ❌ |

Also 3 `pub mod X;` declarations in the contract impl block inside [`lib.rs`](file:///C:/Users/DELL/Desktop/Drips_contribution/Namiikaze/predictify-contracts/contracts/predictify-hybrid/src/lib.rs) that duplicate the above `extensions`, `config`, `metadata_limits` (lib.rs L71–L73) — these are `pub mod` inside the contract block, not the crate root, but they still resolve by file path via the crate-root `mod` declarations. De-duplicating them is optional.

---

## 5. Checklist of minimum actions to unblock fees fuzz delivery

Ordered from smallest diff → largest:

| # | Action | File(s) touched | Required for cargo check? | Required for fees fuzz? |
|---|---|---|---|---|
| S1 | Create `contracts/predictify-hybrid/Cargo.toml` (package manifest, copy shape of oracles) | `contracts/predictify-hybrid/Cargo.toml` (new) | ✅ Yes | ✅ Yes |
| S2 | Create `contracts/disputes/src/lib.rs` stub (unit crate, per its own FUZZ_TARGET.md) | `contracts/disputes/src/lib.rs` (new) | ✅ Yes | ✅ Yes (workspace glob) |
| S3 | Create `contracts/predictify-hybrid/src/fees.rs` with types T1–T7 + 8 inherent impls (real or `unimplemented!()`) from Section 2.3 | `contracts/predictify-hybrid/src/fees.rs` (new) | ✅ Yes | ✅ Yes |
| S4 | Create stubs for the 2 fees-specific test modules | `src/tests/fee_calculator_proptest.rs`, `src/tests/fee_config_commit_reveal_tests.rs` (new) | ❌ Only needed for `#[cfg(test)]` | ✅ Yes (cargo test path) |
| S5 | Create stubs (unit/empty mod) for the ~32 other declared-but-missing modules in Section 4 | ~32 new `.rs` files | ✅ Yes | ✅ Yes (build prerequisite) |

S5 is large; an **alternative** to S5: instead of 32 files, comment out or `#[cfg(FALSE)]`-gate the `mod` declarations in `lib.rs` that are not needed for fees fuzz coverage, leaving only the modules whose source actually exists on disk today (the ones visible via `LS src/`: `admin`, `bets`, `events`, `disputes`*, `market` types`, `resolution`, `storage`, `tests`, plus fees). The project owner can pick whichever direction is lower-risk.

---

## 6. Expected outcomes after this issue is resolved

1. `cargo check --workspace` completes without resolution errors.
2. `cargo check --package predictify-hybrid --tests` resolves all modules.
3. The fees fuzz harness (described in `FUZZ_TARGET_FEES.md`, delivered alongside this report) can be wired up inside the already-existing `contracts/predictify-hybrid/fuzz/` tree without any further scaffolding changes.
4. Public fees entrypoints (EP1–EP6) have a type-resolved target to delegate into, so future (non-stub) implementation work can proceed incrementally without re-breaking the workspace.
