# Per-Entrypoint Auth Snapshot Test

Closes #829

## Summary

Adds an integration suite that **snapshots the Soroban authorization required by every
state-changing entrypoint** of `PredictifyHybrid`. Rather than only asserting "authorized
succeeds / unauthorized fails", each test inspects `env.auths()` after the call and pins
**which address the host actually required an authorization from**.

This catches two regressions a pass/fail test misses: a `require_auth` silently dropped
(auth set becomes empty), and an auth subject rebound to the wrong argument.

Writing the suite surfaced three real authorization defects and a set of pre-existing build
breakages that stopped the crate compiling at all. Both are fixed here so the suite can run.

## Changes

**Tests**

- `tests/auth_snapshot.rs` (new) — 24 tests: committed-snapshot assertions for 13
  entrypoints, auth-boundary assertions for 3, read-only entrypoints pinned to *no auth*,
  plus edge cases (wrong-subject binding, subject tracking across users, non-admin
  rejection, `#[should_panic]` no-auth traps). The fixture registers a real Stellar Asset
  Contract so stake transfers commit.
- `Cargo.toml` — registered the `auth_snapshot` test target.

**Authorization fixes**

Soroban rejects a second `require_auth` on an already-authorized frame
(`Error(Auth, ExistingValue)`). Three paths authorized the same address twice — entrypoint
*and* inner manager — so they could **never succeed on-chain**:

- `fees.rs` — redundant `admin.require_auth()` in `FeeManager::collect_fees`. It was
  `#[cfg(not(test))]`-gated, so unit tests masked it while production builds hit it.
- `disputes.rs` — redundant `user.require_auth()` in `DisputeManager::dispute_market`.
- `rate_limiter.rs` — redundant `user.require_auth()` in `rate_limit_voting` /
  `rate_limit_disputes`.

Auth strength is unchanged: each is still enforced by the calling entrypoint
(`require_primary_admin` for admin paths, `user.require_auth()` for user paths).

**Build restoration**

The crate did not compile on `master`: `e5db2d8` collapsed `lib.rs` to a "minimal working
version" and later PRs re-added call sites without restoring their definitions.

- `lib.rs` — restored 14 dropped `mod` declarations; the `initialize` / `deposit` /
  `withdraw` / `get_balance` entrypoints; the admin-auth helpers (`stored_primary_admin`,
  `require_primary_admin{,_or_panic}`, `require_initialized_admin_root`,
  `require_admin_permission`); the `SYM_*` / `PERCENTAGE_DENOMINATOR` constants and the
  `resolution_timeout_reached` / `automatic_oracle_result_unavailable` helpers. Removed
  duplicate imports, fixed 7 invalid `Address` derefs, repointed `capabilities()`.
- `resolution.rs` — repaired a mis-merge that spliced a payout-distribution body into
  `impl ResolutionOutcomeCache` and into `determine_outcome_from_oracle_data`; restored the
  cache methods 6 modules call, `ResolutionAnalytics` / `MedianResolutionResult`, and the
  `OracleResolutionManager` impl boundary.
- `market_id_generator.rs` — restored the clean file from `28c8db4`.
- `err.rs` — removed a duplicate `ReplayedOverride`; added 6 declared-nowhere variants;
  catch-all arms on `description()` / `code()`.
- `storage.rs` — removed a duplicate `AdminOverrideNonce`; added `PlaceBetsIdem` and
  `AntiGriefFloor`.
- `gas.rs` — `Env::budget()` is test-only, so `BudgetGuard` degrades to a no-op outside
  tests; fixed an oversized `symbol_short!` and an invalid `Default` derive.
- `upgrade_manager.rs` — fixed a `u32`/`u64` branch mismatch.

## Auth matrix

| Entrypoint | Subject | Verified by |
|---|---|---|
| `create_market`, `resolve_market_manual`, `collect_fees`, `set_platform_fee`, `set_treasury`, `set_global_claim_period`, `set_market_claim_period`, `extend_deadline` | admin | committed snapshot |
| `sweep_unclaimed_winnings` | admin | auth boundary |
| `vote`, `place_bet`, `cancel_bet` | user | committed snapshot |
| `claim_winnings`, `dispute_market` | user | auth boundary |
| `get_market`, `get_market_bet_stats` | none | committed snapshot |

*Committed snapshot* — the call is driven through a fully satisfied happy path and the auth
is read back from `env.auths()`. *Auth boundary* — the host rolls back recorded auths when a
call traps or returns `Err`, so entrypoints needing state the fixture does not build assert
instead that an authorized call gets *past* `require_auth` while an unauthorized one traps.

## API / visible changes

None. The suite is test-only, and the three fixes remove redundant authorization calls
without weakening any check.

## Testing

```bash
cargo test -p predictify-hybrid --test auth_snapshot
```

```
test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

`cargo build -p predictify-hybrid --lib` also succeeds, where it previously failed with ~40
parse and resolution errors.

## Notes for reviewers

- **Auth strength is unchanged** — every removed call was a redundant second authorization
  within the same frame.
- **Out of scope** — several `#[cfg(test)]` modules under `src/` still fail to compile from
  pre-existing API drift, and `tests/err_stability.rs` needs nightly (`variant_count`).
- **Local Windows builds** hit a MinGW `export ordinal too large` link error on the `cdylib`
  when `testutils` is enabled; run locally with `crate-type = ["lib"]`. Linux CI is
  unaffected.
