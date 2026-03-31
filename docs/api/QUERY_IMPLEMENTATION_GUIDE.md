# Query Functions Implementation Guide

## Overview

This guide documents the query functions for the Predictify Hybrid Soroban smart contract.
All query functions are read-only, gas-bounded, and safe to call from off-chain clients or
other on-chain contracts.

---

## Pagination

### Why pagination?

Soroban imposes per-invocation CPU and memory limits.  Returning an unbounded `Vec` over a
large market list will exhaust those limits and cause the transaction to fail.  Every query
that iterates the market list therefore accepts a `cursor` + `limit` pair and returns a
`next_cursor` that the caller passes on the next request.

### `PagedResult<T>`

```rust
#[contracttype]
pub struct PagedResult<T: soroban_sdk::Val> {
    /// Items in this page.
    pub items: Vec<T>,
    /// Cursor to pass on the next call (index of the first un-returned item).
    pub next_cursor: u32,
    /// Total number of items available (best-effort; may be approximate for
    /// filtered queries).
    pub total_count: u32,
}
```

### Pagination protocol

```
1. Call with cursor = 0, limit = N  (N ≤ 50)
2. Receive PagedResult { items, next_cursor, total_count }
3. If items.len() < N  →  last page, stop.
4. Otherwise call again with cursor = next_cursor.
```

### Server-side cap

`limit` is always capped at **`MAX_PAGE_SIZE = 50`** regardless of what the caller requests.
This prevents callers from forcing unbounded allocations.

---

## Paginated Query Functions

### `get_all_markets_paged`

Returns a page of market IDs from the market index.

```rust
pub fn get_all_markets_paged(env: Env, cursor: u32, limit: u32) -> PagedResult<Symbol>
```

| Parameter | Description |
|-----------|-------------|
| `cursor`  | Zero-based start index (0 for first page) |
| `limit`   | Desired page size; capped at 50 |

**Returns** `PagedResult<Symbol>` — market IDs, next cursor, total count.

**Example (JavaScript / Stellar SDK)**

```js
let cursor = 0;
const limit = 20;
let allMarkets = [];

while (true) {
    const page = await contract.get_all_markets_paged({ cursor, limit });
    allMarkets = allMarkets.concat(page.items);
    if (page.items.length < limit) break;   // last page
    cursor = page.next_cursor;
}
```

---

### `query_user_bets_paged`

Returns a page of a user's bets, scanning the market index slice
`[cursor, cursor+limit)` and including only markets where the user has a bet.

```rust
pub fn query_user_bets_paged(
    env: Env,
    user: Address,
    cursor: u32,
    limit: u32,
) -> PagedResult<UserBetQuery>
```

| Parameter | Description |
|-----------|-------------|
| `user`    | Address to query |
| `cursor`  | Start index into the market list |
| `limit`   | Page size; capped at 50 |

**Returns** `PagedResult<UserBetQuery>`.

> **Note:** `total_count` reflects the total number of markets scanned, not the number of
> bets found.  Callers should iterate until `items.len() < limit`.

**Example (Rust)**

```rust
let mut cursor = 0u32;
let limit = 20u32;
loop {
    let page = client.query_user_bets_paged(&user, &cursor, &limit);
    for bet in page.items.iter() {
        process_bet(bet);
    }
    if page.items.len() < limit { break; }
    cursor = page.next_cursor;
}
```

---

### `query_contract_state_paged`

Returns partial contract statistics for the market slice `[cursor, cursor+limit)`.
Callers accumulate results across pages to build a full aggregate.

```rust
pub fn query_contract_state_paged(
    env: Env,
    cursor: u32,
    limit: u32,
) -> (ContractStateQuery, u32)
```

**Returns** `(ContractStateQuery, next_cursor)`.

---

## Existing Paginated Functions (EventArchive)

These functions were already paginated and are unchanged:

| Function | Description |
|----------|-------------|
| `query_events_history(from_ts, to_ts, cursor, limit)` | Events by creation time range |
| `query_events_by_status(status, cursor, limit)` | Events by `MarketState` |
| `query_events_by_category(category, cursor, limit)` | Events by category string |
| `query_events_by_tags(tags, cursor, limit)` | Events matching any tag (OR) |

All return `(Vec<EventHistoryEntry>, next_cursor)` and cap `limit` at **`MAX_QUERY_LIMIT = 30`**.

---

## Non-Paginated Query Functions

These functions perform a single storage lookup and are safe to call without pagination:

| Function | Returns | Notes |
|----------|---------|-------|
| `query_event_details(market_id)` | `EventDetailsQuery` | Full market info |
| `query_event_status(market_id)` | `(MarketStatus, u64)` | Lightweight status check |
| `query_user_bet(user, market_id)` | `UserBetQuery` | Single bet lookup |
| `query_market_pool(market_id)` | `MarketPoolQuery` | Pool distribution |
| `query_user_balance(user)` | `UserBalanceQuery` | Account summary |
| `query_total_pool_size()` | `i128` | Total TVL (iterates all markets — use with care on large deployments) |
| `query_contract_state()` | `ContractStateQuery` | Full scan — prefer `_paged` variant |

---

## Response Types

All types are `#[contracttype]` for Soroban XDR compatibility.

### `EventDetailsQuery`

```rust
pub struct EventDetailsQuery {
    pub market_id: Symbol,
    pub question: String,
    pub outcomes: Vec<String>,
    pub created_at: u64,
    pub end_time: u64,
    pub status: MarketStatus,
    pub oracle_provider: String,
    pub feed_id: String,
    pub total_staked: i128,
    pub winning_outcome: Option<String>,
    pub oracle_result: Option<String>,
    pub participant_count: u32,
    pub vote_count: u32,
    pub admin: Address,
}
```

### `UserBetQuery`

```rust
pub struct UserBetQuery {
    pub user: Address,
    pub market_id: Symbol,
    pub outcome: String,
    pub stake_amount: i128,
    pub voted_at: u64,
    pub is_winning: bool,
    pub has_claimed: bool,
    pub potential_payout: i128,
    pub dispute_stake: i128,
}
```

### `PagedResult<T>`

See [Pagination](#pagination) above.

---

## Security Notes

### Threat model

- All query functions are **read-only** — no state is modified.
- `limit` is capped server-side; callers cannot force unbounded allocations.
- No sensitive data (private keys, raw vote maps, stake maps) is returned by
  `EventArchive` history queries — only public metadata and outcomes.
- `archive_event` requires admin authentication (`require_auth`) and validates
  market state before writing.

### Invariants proven by tests

1. `items.len() ≤ min(limit, MAX_PAGE_SIZE)` for every paginated call.
2. `next_cursor ≤ total_count` always.
3. `next_cursor ≥ cursor` always (monotone progression).
4. Oversized `limit` values (e.g. 9999) never panic.
5. `cursor` past the end of the list returns an empty page without error.

### Explicit non-goals

- Pagination does **not** guarantee atomicity across pages (the market list may
  change between calls).
- `total_count` in `PagedResult` is a best-effort snapshot; it may differ from
  the true count if markets are created between pages.
- Off-chain aggregation of `query_contract_state_paged` results is the
  caller's responsibility.

---

## Testing

Tests live in `contracts/predictify-hybrid/src/query_tests.rs`.

```bash
# Run all query tests
cargo test -p predictify-hybrid -- query

# Run only pagination tests
cargo test -p predictify-hybrid -- paged
```

### Test categories

| Category | Tests |
|----------|-------|
| Unit (helpers) | payout, probabilities, outcome pool |
| Status conversion | all 6 `MarketState` variants |
| Pagination — empty | cursor=0 on empty index |
| Pagination — limit cap | limit=9999 must not panic |
| Pagination — cursor past end | empty page, no panic |
| Pagination — monotone cursor | `next_cursor ≥ cursor` |
| Invariant / property | items ≤ limit, next_cursor ≤ total |
| Regression | `MAX_PAGE_SIZE == 50` constant |

---

## Integrator Quick-Start

```js
// 1. Paginate market list
const markets = [];
let cursor = 0;
while (true) {
    const { items, next_cursor } = await contract.get_all_markets_paged({ cursor, limit: 50 });
    markets.push(...items);
    if (items.length < 50) break;
    cursor = next_cursor;
}

// 2. Get details for a specific market
const details = await contract.query_event_details({ market_id: "mkt_abc123_0" });

// 3. Paginate a user's bets
const bets = [];
cursor = 0;
while (true) {
    const { items, next_cursor } = await contract.query_user_bets_paged({ user, cursor, limit: 50 });
    bets.push(...items);
    if (items.length < 50) break;
    cursor = next_cursor;
}

// 4. Query event history by time range
const [history, nextCursor] = await contract.query_events_history({
    from_ts: 1700000000n,
    to_ts:   1800000000n,
    cursor: 0,
    limit: 30,
});
```

---

*Last updated: 2026-03-29*
