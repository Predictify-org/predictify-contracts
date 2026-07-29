# Tokens Contract

The `tokens` contract bounds token-adjacent state stored for each account.
Authenticated accounts may track and untrack unique 32-byte identifiers for:

- bets;
- positions;
- subscriptions.

## Per-account limits

`AccountLimits` configures independent caps for each category. The same limits
apply separately to every account, and every configured value must be no larger
than `MAX_CONFIGURABLE_ACCOUNT_LIMIT` (`256`).

Tracking an item:

1. requires authentication from the affected account;
2. rejects duplicate `(account, category, item_id)` tuples;
3. increments usage with `checked_add`;
4. verifies the next count does not exceed the configured category cap;
5. stores the bounded category membership set only after every validation
   succeeds.

Untracking requires the exact stored item and uses `checked_sub`, so a fabricated
identifier cannot release capacity.

Each account/category membership set is stored as one persistent entry. Its
membership and count therefore share a TTL and cannot drift if an old entry
expires.

## Public API changes

The crate adds `TokensContract` with these state-changing entrypoints:

| Entrypoint | Required signer | Purpose |
|---|---|---|
| `initialize` | Initial admin | Set admin and limits |
| `set_account_limits` | Current admin | Replace global per-account caps |
| `track_account_item` | Affected account | Add one bounded item |
| `untrack_account_item` | Affected account | Remove one exact item |

Read-only views:

- `get_account_limits`
- `get_account_usage`
- `get_remaining_capacity`
- `is_account_item_tracked`
- `get_admin`

Lowering a cap below existing usage does not delete account state. Remaining
capacity is reported as zero, and new items in that category are rejected until
usage falls below the new cap.

## Testing

Run the focused suite with:

```bash
cargo test -p tokens
```
