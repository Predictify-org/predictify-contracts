# Event Archive

Bounded on-chain storage for resolved and cancelled prediction markets, with paginated historical queries.

## Overview

`EventArchive` (in `contracts/predictify-hybrid/src/event_archive.rs`) lets admins mark resolved or cancelled markets as archived and exposes read-only paginated queries for analytics and UI. Only public metadata is returned — no votes, individual stakes, or addresses.

## Storage bounds

| Constant | Value | Purpose |
|---|---|---|
| `MAX_ARCHIVE_SIZE` | 1 000 | Hard cap on archived entries |
| `MAX_QUERY_LIMIT` | 30 | Max entries returned per query call |

Once `MAX_ARCHIVE_SIZE` is reached, `archive_event` returns `Error::ArchiveFull` (code `435`). Call `prune_archive` to remove the oldest entries and free capacity.

## API

### `archive_event(admin, market_id) → Result<(), Error>`

Mark a `Resolved` or `Cancelled` market as archived. Admin-only.

Errors: `Unauthorized`, `MarketNotFound`, `InvalidState`, `AlreadyClaimed`, `ArchiveFull`.

### `prune_archive(admin, count) → Result<u32, Error>`

Remove the oldest `count` entries (capped at 30) from the archive. Returns the number actually removed. Admin-only.

Errors: `Unauthorized`.

### `archive_size() → u32`

Return the current number of archived entries. Read-only, no auth required.

### `query_events_history(from_ts, to_ts, cursor, limit) → (Vec<EventHistoryEntry>, u32)`

Return events whose creation timestamp falls in `[from_ts, to_ts]`. Paginated: pass the returned `next_cursor` as `cursor` on the next call. Stop when `next_cursor == cursor`.

### `query_events_by_resolution_status(status, cursor, limit) → (Vec<EventHistoryEntry>, u32)`

Filter by `MarketState` (Active, Resolved, Cancelled, …).

### `query_events_by_category(category, cursor, limit) → (Vec<EventHistoryEntry>, u32)`

Match against the market's `category` field, falling back to the oracle `feed_id`.

### `query_events_by_tags(tags, cursor, limit) → (Vec<EventHistoryEntry>, u32)`

OR logic: returns markets that have any of the provided tags. Empty tag list returns nothing.

## Pagination pattern

```
cursor = 0
loop:
    (entries, next) = query_events_history(from, to, cursor, 30)
    process(entries)
    if next == cursor: break   # no more pages
    cursor = next
```

## Pruning strategy

When `archive_size()` approaches `MAX_ARCHIVE_SIZE`, call `prune_archive(admin, N)` to remove the N oldest entries (by market registry order). Pruned entries are gone from the archive but the underlying market data remains in persistent storage.

## Security notes

- All write operations (`archive_event`, `prune_archive`) require admin authentication.
- Queries expose only aggregate public data (`EventHistoryEntry`); no individual stakes, votes, or addresses are returned.
- The `MAX_ARCHIVE_SIZE` cap prevents a DoS vector where an attacker (or runaway admin script) fills persistent storage with archive entries.
- `prune_archive` count is capped at `MAX_QUERY_LIMIT` (30) per call to bound gas usage.

## Invariants

1. `archive_size() ≤ MAX_ARCHIVE_SIZE` at all times.
2. A market can only be archived once (`AlreadyClaimed` on duplicate).
3. Only `Resolved` or `Cancelled` markets can be archived.
4. Query results contain at most `MAX_QUERY_LIMIT` entries per call.
