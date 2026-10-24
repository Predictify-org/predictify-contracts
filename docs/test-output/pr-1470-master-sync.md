# PR #1470: master synchronization

The PR branch was merged with upstream master `d806a26` using merge commit
`8cdf946`. The merge preserves the original PR commits and all upstream changes.

## Conflict resolution

The only merge conflict was `.github/workflows/contract-ci.yml`. Both sides are
retained: the ignored-test budget and timed hybrid integration step from #1463,
and the analytics tests/WASM budget steps from master. The existing cache,
production build, library tests, stable-crate tests, and slim check remain.

Verified: every relevant workflow step appears exactly once, the integration Bash
block passes `bash -n`, no conflict markers remain, `git diff --check` passes,
and the ignored-test ratchet passes at 174 / 174. Upstream master is an ancestor
of the merged branch. No force-push is needed.

## Upstream validation blocker

The newly merged analytics package is incomplete in upstream commit `63d4d78`:

- `git ls-tree -r --name-only upstream/master contracts/predictify-analytics`
  lists only `Cargo.toml`; the library source `src/lib.rs` is absent.
- `cargo metadata --no-deps --format-version 1` fails before any tests run:
  `can't find library predictify_analytics, rename file to src/lib.rs or specify lib.path`.
- Its manifest requests the hybrid feature `testutils`, which the hybrid manifest
  does not define, and contains a misspelled dependency-table heading.
- `scripts/build_analytics_wasm.sh` has a malformed shebang and ends with an
  incomplete `ROOT_DIR` assignment. `bash -n` reports an unexpected EOF while
  looking for a matching quotation mark on line 13.

These files are preserved from master. The conflict resolution does not create a
replacement analytics contract or bypass its new CI checks. The complete workspace
cannot be reported as passing until the upstream analytics package is repaired.

## Additional isolated validation

Validation of the merged hybrid code is run from an untracked copy under
`target/merge-validation-1470`, containing the hybrid, hello-world, and oracles
packages, without the incomplete analytics package. This is a diagnostic check;
it does not substitute for running the complete merged workspace in CI.

The user explicitly requested retaining the scope of this PR and documenting the
upstream blocker rather than repairing analytics in this branch.

`cargo check --offline -p predictify-hybrid --no-default-features` passed in the
isolated workspace. `cargo test --offline -p predictify-hybrid --tests --no-fail-fast` also passed:
759 library tests (172 pre-existing ignored) and all 119 integration tests across
11 targets. Both commands ran against the merged sources in the isolated workspace.
These results do not validate the incomplete analytics package.
