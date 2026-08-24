# Market ID Generation

Market IDs are `Symbol` values used as persistent-storage keys for every market
and event in the Predictify Hybrid contract.

## Format

```
mkt_{8 hex chars}_{admin_counter}
```

Example: `mkt_3f9a1b2c_0`

- `mkt_` — fixed prefix, used by `validate_market_id_format` to distinguish
  current IDs from legacy ones.
- `{8 hex chars}` — first 4 bytes of a SHA-256 digest (see below).
- `{admin_counter}` — the per-admin counter at creation time; makes IDs
  human-readable and auditable without decoding the hash.

## Entropy sources

The SHA-256 input is:

```
ledger_sequence (4 B, big-endian) || global_nonce (4 B, big-endian)
```

| Source | Contribution |
|--------|-------------|
| Ledger sequence | Distinguishes markets created in different ledgers |
| Global nonce | Monotonically increasing across all admins; ensures two admins calling `generate_market_id` in the same ledger produce different IDs |

The global nonce increments on every call regardless of which admin is creating
the market, so the `(sequence, nonce)` pair is always unique.

## Collision risk

The truncated hash occupies 32 bits.  With two independent monotonic inputs the
effective pre-image space is `2^32 × N_ledgers`, making accidental collisions
negligible in practice.  The generator also performs an explicit storage lookup
and retries up to `MAX_RETRIES = 10` times before panicking — a belt-and-
suspenders guard, not a primary defence.

## Counter limits

Each admin has an independent counter capped at `MAX_COUNTER = 999_999`.
Reaching the cap causes `Error::InvalidInput`.  In practice an admin would need
to create one million markets before hitting this limit.

## Legacy IDs

IDs that do not start with `mkt_` are treated as legacy.
`validate_market_id_format` returns `false` for them;
`parse_market_id_components` returns `is_legacy: true`.

## Security notes

- IDs are **deterministic** given the same inputs — they are not secret.  Do
  not use them as access-control tokens.
- The global nonce and per-admin counters are stored in persistent storage and
  survive contract upgrades.
- Collision detection reads from persistent storage, so it correctly handles
  IDs created by both `create_market` and `create_event`.
