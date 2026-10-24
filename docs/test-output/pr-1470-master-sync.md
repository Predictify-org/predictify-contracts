# PR #1470: master synchronization

The PR branch was synchronized with upstream master `49a4762` using merge commit
`91d8bf6`. The merge preserves the original PR commits and upstream changes.

## Conflict resolution

The conflicts in `contracts/predictify-hybrid/src/lib.rs` and `docs/TESTING.md`
are resolved. The upstream contract API and testing guide are retained; the
single-frame authorization entrypoints and their regression tests are preserved.
The CI workflow retains both the ignored-test budget and hybrid integration
steps from #1463 and the analytics test/WASM budget steps from master.

Verified: every relevant workflow step appears exactly once, the integration Bash
block passes `bash -n`, no conflict markers remain, `git diff --check` passes,
and the ignored-test ratchet passes at 174 / 174. Upstream master is an ancestor
of the merged branch. No force-push is needed.

## Analytics package repair

The newly merged analytics package is incomplete in upstream commit `63d4d78`:

- `git ls-tree -r --name-only upstream/master contracts/predictify-analytics`
  lists only `Cargo.toml`; the library source `src/lib.rs` is absent.
- Its manifest contains a misspelled development-dependency section and requests
  an undefined `predictify-hybrid` `testutils` feature.
- `scripts/build_analytics_wasm.sh` has a malformed shebang and is truncated.

The branch now supplies a read-only analytics façade that forwards seven
analytics queries to the hybrid contract, corrects the manifest and build script,
and tests the cross-contract market-statistics path. The build script compiles
the analytics WASM and enforces the 131,072-byte limit. CI must pass the analytics
test and WASM build steps to validate these changes end-to-end.

## Additional isolated validation

Validation of the merged hybrid code is run from an untracked copy under
`target/merge-validation-1470`, containing the hybrid, hello-world, and oracles
packages, without the incomplete analytics package. This is a diagnostic check;
it does not substitute for running the complete merged workspace in CI.

`cargo check --offline -p predictify-hybrid --no-default-features` passed in the
isolated workspace. `cargo test --offline -p predictify-hybrid --tests --no-fail-fast` also passed:
759 library tests (172 pre-existing ignored) and all 119 integration tests across
11 targets. Both commands ran against the merged sources in the isolated workspace.
These earlier results do not include the analytics façade added afterward.
