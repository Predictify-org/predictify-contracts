# Contract testing

Run the contract unit tests from the workspace root:

```sh
cargo test -p predictify-hybrid --lib
```

The single-frame authorization regressions are in
`contracts/predictify-hybrid/src/single_frame_auth_tests.rs`. They invoke the
generated contract client with `Env::mock_auths` and exactly one authorization
entry per invocation. They deliberately avoid `mock_all_auths`, which would
mask repeated `require_auth` calls.

Authentication is performed at the contract entrypoint. The dispute and fee
managers are called through crate-private `*_authorized` methods, which retain
business validation and applicable permission checks but do not authenticate
again. Entrypoints that require the primary admin also retain their identity
check.
The corresponding call chains are:

| Entrypoint | Before | After |
| --- | --- | --- |
| `dispute_market` | Entrypoint and `DisputeManager::process_dispute` both authenticated `user` | Entrypoint calls `user.require_auth()` once, then `process_dispute_authorized` |
| `vote_on_dispute` | Entrypoint and `DisputeManager::vote_on_dispute` both authenticated `user` | Entrypoint calls `user.require_auth()` once, then `vote_on_dispute_authorized` |
| `resolve_dispute` | Entrypoint and `DisputeManager::resolve_dispute` both authenticated `admin` | Entrypoint authenticates then checks primary-admin identity; `resolve_dispute_authorized` retains manager permission validation |
| `collect_fees` | Entrypoint authenticated `admin`; manager authenticated again in non-test builds | Entrypoint authenticates then checks primary-admin identity; `collect_fees_authorized` retains manager permission validation |

There are zero active `#[ignore]` test attributes in `src/`; no previously
ignored tests could be restored from this checkout.
There are zero active `#[ignore]` test attributes in `src/`; the matches in
`bet_tests.rs` are comments, not ignored tests.
`upgrade_contract` and `rollback_upgrade` remain excluded as requested.

For the contract's no-default-features compilation check, run:

```sh
cargo check -p predictify-hybrid --no-default-features
```
