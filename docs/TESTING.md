# Testing status

The `predictify-analytics` contract provides read-only cross-contract views over
market, voting, dispute, participation, fee, comparison, and oracle analytics
from `predictify-hybrid`. It has no independent state; queries take the core
contract address explicitly and forward the core contract's returned data.
See [the synchronization report](test-output/pr-1470-master-sync.md) for the
upstream package issue and its repair.

```sh
cargo test -p predictify-hybrid --lib     # 730 passed, 0 failed, 172 ignored
cargo test -p predictify-hybrid --lib bets::tests # 41 passed, none ignored
cargo test -p predictify-hybrid --lib balances    # 5 passed, none ignored
cargo test -p predictify-hybrid --tests
cargo test -p hello-world -p oracles
cargo test -p predictify-analytics
rustup target add wasm32v1-none
cargo build --release --target wasm32v1-none -p predictify-analytics
./scripts/build_analytics_wasm.sh
cargo check -p predictify-hybrid --no-default-features
bash scripts/check_ignored_tests.sh
```

CI (`.github/workflows/contract-ci.yml`) runs the library, all 11 active
integration targets, stable crates, analytics tests and its 128 KiB WASM budget
check, the slim feature check, and the ignored-test ratchet on every push/PR
targeting master. Swatinem/rust-cache is shared across these steps. Cargo's
`--tests` also reruns the library tests. The integration step writes its elapsed
seconds, including that rerun, to the GitHub Actions job summary.

## Single-frame authorization

The regressions in
`contracts/predictify-hybrid/src/single_frame_auth_tests.rs` use the generated
contract client with exactly one `Env::mock_auths` entry per invocation. They
avoid `mock_all_auths`, which would mask repeated `require_auth` calls.

Entrypoints authenticate once; authorized manager methods retain business
validation and applicable permission checks without authenticating again:

| Entrypoint | Authentication behavior |
| --- | --- |
| `dispute_market` | Authenticates `user`, then calls `process_dispute_authorized`. |
| `vote_on_dispute` | Authenticates `user`, then calls `vote_on_dispute_authorized`. |
| `resolve_dispute` | `require_primary_admin` authenticates and checks primary-admin identity, then the authorized manager validates permissions. |
| `collect_fees` | `require_primary_admin` authenticates and checks primary-admin identity, then the authorized manager validates permissions. |

## Shared contract context

`#[cfg(test)] pub(crate) mod testutil` provides `TestCtx` with `env`, `contract_id`,
`admin`, and `token`, following the existing `PredictifyTest::setup` pattern in
`src/test.rs`. That older module remains disabled because of unrelated API drift.

Use `TestCtx::new()` for an initialized contract with configuration, circuit
breaker, and staking token. `funded_user(amount)` mints the requested amount;
`market()` creates an active two-outcome market. `TestCtx::uninitialized()` gives
a registered contract with empty application storage for default/legacy fallback
tests. Both constructors mock authentication; authorization boundary tests must
set explicit auth or clear it themselves.

```rust
use crate::testutil::TestCtx;
use crate::types::ReflectorAsset;
use crate::storage::BalanceStorage;

let ctx = TestCtx::new();
let user = ctx.funded_user(1_000_000);
let market_id = ctx.market();
ctx.with_contract(|| {
    BalanceStorage::add_balance(&ctx.env, &user, &ReflectorAsset::Stellar, 42).unwrap();
});
assert_eq!(ctx.client().get_balance(&user, &ReflectorAsset::Stellar).amount, 42);
assert!(ctx.client().get_market(&market_id).is_some());
```

Put direct storage/module calls inside `with_contract`; call the generated client
outside the closure so it establishes its own invocation frame. Snapshots at drop
are disabled in this fixture to avoid generating test artifacts.

## Ignored-test budget

The original revision `78ce97e` had **205 source ignore attributes**, corresponding
to **203 ignored tests** in the compiled default library. All 26 ignored tests in
`bets.rs` (including five fee-slippage cases) and all five in `balances.rs` are now
active: **174 source attributes**, **172 compiled ignored tests**. The difference
comes from modules excluded from the compiled library. Two harness tests are new.

`.ignored-baseline` contains the source-tree budget, currently **174**.
`scripts/check_ignored_tests.sh` scans Rust files recursively under
`contracts/predictify-hybrid/src`, including excluded modules. It fails when the
count exceeds the committed budget. It accepts reasoned and bare ignore
attributes and can be run from any working directory.

When restoring more tests, remove their attributes, run their suites and the full
library/integration checks, then lower `.ignored-baseline` to the new source count.
Do not raise the budget to accommodate a newly ignored test. A new ignore requires
restoring another test first. The validation report records a temporary extra
ignore being rejected and the baseline succeeding; the probe was removed.

Remaining ignored suites are outside #1463: `deprecated_tests`, `oracle_health`,
`market_audit_tests`, `recovery`, event compatibility, force/tie resolution,
participant limits, and storage-tier auditing. Causes include missing contract
context, repeated authentication, and stale fixtures. Run a focused module with
`-- --ignored` when restoring it. The two previously disabled fuzz targets remain
disabled; no active integration file was disabled by this change.

## Evidence and regressions

See [issue #1463 validation](test-output/issue-1463-validation.md) for every restored
test, production defects, baseline failure output, coverage before/after, commands,
and timing limitations. Production changes are confined to duplicate-auth fixes
revealed by the newly enabled integration run; admin identity validation and
missing-auth rejection remain tested.

Previously fixed behavior includes simulation-stable market identifiers and
checked atomic bet preparation. The restored suites now exercise bet limits,
market caps, storage/analytics, fee slippage, stake/stat overflow, token transfer
accounting, insufficient balances, and failed-withdrawal rollback.
