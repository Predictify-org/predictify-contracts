# Per-Market Extension Count Cap

## Overview
In addition to the existing 72-hour cumulative extension cap
(`total_extension_days` vs `max_extension_days`), markets now enforce
a maximum of **3 extension calls** per market lifecycle via a new
`extension_count: u32` field on `Market`.

## Behavior
- `MarketStateManager::extend_for_dispute` increments `extension_count`
  on every successful extension.
- Once `extension_count` reaches `MAX_EXTENSION_COUNT` (3), further
  calls return `Error::ExtensionCountCapExceeded`, even if the
  cumulative-hours cap still has headroom.
- The two caps are independent: either one alone is sufficient to
  reject a further extension.

## API changes
- New field: `Market::extension_count: u32`
- New error variant: `Error::ExtensionCountCapExceeded = 526`