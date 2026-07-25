# Dispute invariant property tests

Issue #879 adds state-invariant property coverage for dispute voting in
`contracts/predictify-hybrid/src/tests/disputes_proptest.rs`.

## Invariants

Generated inputs verify that:

- `total_votes` always equals `support_votes + against_votes`;
- support and against stakes are conserved independently after every vote;
- dispute identity, lifecycle status, and unrelated tally fields do not change;
- stake decay stays between zero and the raw stake and is monotonic over time;
- exact stake ties reject the dispute, while only a strict support majority
  upholds it;
- arithmetic overflow returns `Error::Overflow` without partially writing the
  voting record.

The generators include empty vote sequences, zero stakes, maximum `i128` stakes,
oversized decay floors, saturated timestamps, ties, and both voting sides.
Failures are automatically shrunk and can be persisted by proptest as regression
seeds.

## Security changes

Dispute vote counters and stake totals now use checked addition. Stake decay
splits division and multiplication to avoid intermediate `i128` overflow, and
misconfigured floors are capped at 10,000 basis points so decay cannot amplify a
vote.

Every state-changing public dispute entrypoint retains its existing
`require_auth` boundary. This change adds no entrypoint and relaxes no
authorization requirement.

## API changes

There are no public contract API, storage-key, event, or visible UI changes.
Arithmetic overflow from internal dispute tally updates is now reported as the
existing `Error::Overflow` value instead of panicking.

## Running

```sh
cargo test -p predictify-hybrid disputes_proptest
cargo test -p predictify-hybrid
cargo fmt --all -- --check
cargo clippy -p predictify-hybrid --all-targets -- -D warnings
```