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

## Dashboard Statistics Queries (NEW)

These new query functions expose aggregated market and user statistics optimized for dashboard display. All responses use versioned types (`V1` suffix) to enable forward compatibility without breaking changes.

### Platform-Level Dashboard Statistics

#### `get_dashboard_statistics`

Returns comprehensive platform-level metrics with version information.

```rust
pub fn get_dashboard_statistics(env: Env) -> Result<DashboardStatisticsV1, Error>
```

**Returns** `DashboardStatisticsV1`:

```rust
pub struct DashboardStatisticsV1 {
    /// API version (always 1 for this type)
    pub api_version: u32,
    /// Platform statistics snapshot
    pub platform_stats: PlatformStatistics,
    /// Ledger timestamp when query was executed
    pub query_timestamp: u64,
    /// Number of active users (with at least one bet)
    pub active_user_count: u32,
    /// Total value locked across all markets
    pub total_value_locked: i128,
}

// Underlying platform statistics
pub struct PlatformStatistics {
    pub total_events_created: u64,
    pub total_bets_placed: u64,
    pub total_volume: i128,
    pub total_fees_collected: i128,
    pub active_events_count: u32,
}
```

**Use cases**: Dashboard header, overall metrics, TVL display

**Example (JavaScript)**

```js
const stats = await contract.get_dashboard_statistics();
console.log(`Platform Version: ${stats.api_version}`);
console.log(`Total Markets Created: ${stats.platform_stats.total_events_created}`);
console.log(`Total Volume: ${stats.total_value_locked} stroops`);
console.log(`Active Users: ${stats.active_user_count}`);
console.log(`Query Timestamp: ${stats.query_timestamp}`);
```

---

### Per-Market Statistics

#### `get_market_statistics`

Returns detailed statistics for a specific market with volatility and consensus metrics.

```rust
pub fn get_market_statistics(env: Env, market_id: Symbol) -> Result<MarketStatisticsV1, Error>
```

**Parameters**:
- `market_id`: Market identifier

**Returns** `MarketStatisticsV1`:

```rust
pub struct MarketStatisticsV1 {
    pub market_id: Symbol,
    pub participant_count: u32,              // Number of unique participants
    pub total_volume: i128,                  // Total amount wagered
    pub average_stake: i128,                 // Average stake per participant
    pub consensus_strength: u32,             // 0-10000 (higher = more agreement)
    pub volatility: u32,                     // 0-10000 (inverse of consensus)
    pub state: MarketState,
    pub created_at: u64,
    pub question: String,
    pub api_version: u32,                    // Always 1
}
```

**Metrics Explained**:

- **Consensus Strength**: `(largest_outcome_pool / total_volume) * 10000`
  - 10000 = all participants agreed on one outcome
  - 0 = perfect distribution across outcomes
  
- **Volatility**: `10000 - consensus_strength`
  - Measures opinion diversity
  - High volatility = contentious market
  - Low volatility = strong consensus

**Use cases**: Market detail pages, heat maps, volatility indicators

**Example (Rust)**

```rust
let stats = client.get_market_statistics(&env, market_id)?;
println!("Participants: {}", stats.participant_count);
println!("Total Wagered: {}", stats.total_volume);
println!("Avg Stake: {}", stats.average_stake);
println!("Consensus: {}% (volatility: {}%)", 
    stats.consensus_strength / 100, 
    stats.volatility / 100);
```

**Errors**:
- `Error::MarketNotFound` - Market doesn't exist

---

### Category-Based Statistics

#### `get_category_statistics`

Returns aggregated metrics for all markets in a specific category.

```rust
pub fn get_category_statistics(env: Env, category: String) -> Result<CategoryStatisticsV1, Error>
```

**Parameters**:
- `category`: Category name (e.g., "sports", "crypto", "politics")

**Returns** `CategoryStatisticsV1`:

```rust
pub struct CategoryStatisticsV1 {
    pub category: String,
    pub market_count: u32,                   // Markets in this category
    pub total_volume: i128,                  // Aggregate volume
    pub participant_count: u32,              // Unique participants
    pub resolved_count: u32,                 // Number resolved
    pub average_market_volume: i128,         // Mean volume per market
}
```

**Use cases**: Category filters, category leaderboards, category analytics

**Example (JavaScript)**

```js
const sports = await contract.get_category_statistics({ category: "sports" });
console.log(`Sports Markets: ${sports.market_count}`);
console.log(`Total Sports Volume: ${sports.total_volume}`);
console.log(`Avg Market Volume: ${sports.average_market_volume}`);
console.log(`Resolved: ${sports.resolved_count} / ${sports.market_count}`);
```

---

### Leaderboard Queries

#### `get_top_users_by_winnings`

Returns top users ranked by total winnings claimed.

```rust
pub fn get_top_users_by_winnings(env: Env, limit: u32) -> Result<Vec<UserLeaderboardEntryV1>, Error>
```

**Parameters**:
- `limit`: Maximum results (capped at 50 for gas safety)

**Returns** `Vec<UserLeaderboardEntryV1>`:

```rust
pub struct UserLeaderboardEntryV1 {
    pub user: Address,
    pub rank: u32,
    pub total_winnings: i128,
    pub win_rate: u32,                       // Basis points (0-10000)
    pub total_bets_placed: u64,
    pub winning_bets: u64,
    pub total_wagered: i128,
    pub last_activity: u64,
}
```

**Use cases**: Leaderboard pages, top earners, achievements

**Example**:

```js
const topWinners = await contract.get_top_users_by_winnings({ limit: 10 });
for (const entry of topWinners) {
    console.log(`#${entry.rank}: ${entry.user} earned ${entry.total_winnings}`);
}
```

---

#### `get_top_users_by_win_rate`

Returns top users ranked by win rate percentage (minimum bet requirement).

```rust
pub fn get_top_users_by_win_rate(
    env: Env,
    limit: u32,
    min_bets: u64,
) -> Result<Vec<UserLeaderboardEntryV1>, Error>
```

**Parameters**:
- `limit`: Maximum results (capped at 50)
- `min_bets`: Minimum bets required for inclusion (e.g., 10 to filter lucky users with few bets)

**Returns** Same as `get_top_users_by_winnings`

**Use cases**: Skill leaderboards, prediction accuracy rankings

**Example**:

```js
// Top 10 predictors with at least 5 bets
const topSkills = await contract.get_top_users_by_win_rate({ 
    limit: 10, 
    min_bets: 5n 
});
```

---

### Versioning and Compatibility

All dashboard response types use `V1` versioning:

```rust
pub struct DashboardStatisticsV1 { pub api_version: u32, ... }
pub struct MarketStatisticsV1 { pub api_version: u32, ... }
pub struct UserLeaderboardEntryV1 { ... }
pub struct CategoryStatisticsV1 { ... }
```

**Forward Compatibility**:
- `api_version` field enables soft upgrades
- New fields may be added to V1 types in future versions
- Clients should ignore unknown fields (Soroban XDR feature)
- New response types use V2, V3 naming if breaking changes occur

---

## Testing

Dashboard statistics tests are in `contracts/predictify-hybrid/src/query_tests.rs` under the `// ===== DASHBOARD STATISTICS TESTS =====` section.

```bash
# Run dashboard stats tests
cargo test -p predictify-hybrid -- dashboard

# Run all statistics tests
cargo test -p predictify-hybrid -- statistics
```

### Test coverage

| Function | Tests |
|----------|-------|
| `get_dashboard_statistics` | Empty state, API version |
| `get_market_statistics` | Empty market, with participants, partial consensus, version, ranges |
| `get_category_statistics` | No markets, multiple markets, version |
| `get_top_users_by_winnings` | Limit cap |
| `get_top_users_by_win_rate` | Limit cap, min_bets filter |
| **Invariants** | Consensus + Volatility = 10000, ranges 0-10000 |

---

## Dashboard Integration Example

```javascript
// Complete dashboard initialization
async function initializeDashboard() {
    // 1. Get platform stats
    const platformStats = await contract.get_dashboard_statistics();
    
    // 2. Get featured markets with stats
    const markets = [];
    let cursor = 0;
    while (true) {
        const page = await contract.get_all_markets_paged({ cursor, limit: 50 });
        for (const id of page.items) {
            const details = await contract.query_event_details({ market_id: id });
            const stats = await contract.get_market_statistics({ market_id: id });
            markets.push({ ...details, ...stats });
            if (markets.length >= 10) break;  // Featured section
        }
        if (page.items.length < 50 || markets.length >= 10) break;
        cursor = page.next_cursor;
    }
    
    // 3. Get category filters with stats
    const categories = ["sports", "crypto", "politics"];
    const categoryStats = await Promise.all(
        categories.map(cat => 
            contract.get_category_statistics({ category: cat })
        )
    );
    
    // 4. Get leaderboards
    const topWinners = await contract.get_top_users_by_winnings({ limit: 10 });
    const topSkills = await contract.get_top_users_by_win_rate({ limit: 10, min_bets: 5n });
    
    return {
        platformStats,
        featuredMarkets: markets,
        categoryStats: Object.fromEntries(
            categories.map((cat, i) => [cat, categoryStats[i]])
        ),
        leaderboards: { topWinners, topSkills }
    };
}
```

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
| **Dashboard stats** | API versioning, metric ranges, aggregation |

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

// 3. Get market statistics for dashboard
const stats = await contract.get_market_statistics({ market_id: "mkt_abc123_0" });
console.log(`Consensus: ${stats.consensus_strength / 100}%`);

// 4. Get platform dashboard stats
const dashboard = await contract.get_dashboard_statistics();
console.log(`Total Volume: ${dashboard.total_value_locked}`);

// 5. Get category-filtered stats
const sportsStats = await contract.get_category_statistics({ category: "sports" });

// 6. Get leaderboards
const topUsers = await contract.get_top_users_by_winnings({ limit: 10 });

// 7. Paginate a user's bets
const bets = [];
cursor = 0;
while (true) {
    const { items, next_cursor } = await contract.query_user_bets_paged({ user, cursor, limit: 50 });
    bets.push(...items);
    if (items.length < 50) break;
    cursor = next_cursor;
}

// 8. Query event history by time range
const [history, nextCursor] = await contract.query_events_history({
    from_ts: 1700000000n,
    to_ts:   1800000000n,
    cursor: 0,
    limit: 30,
});
```

---

*Last updated: 2026-03-30*
