# Rate Limits

This document describes the rate limiting system in the Predictify Hybrid contract and explains which actions consume which rate limit buckets.

## Overview

The contract implements a token-bucket rate limiter to prevent abuse and protect against flash-trading attacks. Rate limits are configured globally and enforced per-user or per-market depending on the action type.

## Rate Limit Categories

### 1. Per-User Bet Limit (`bet_limit`)

**Bucket Key**: `RateLimiterData::UserBets(Address)` — global per user

**Actions that consume this limit**:
- `place_bet` — placing a single bet
- `place_bets` — placing a batch of bets (consumes one token per bet in the batch)
- `vote` — casting a vote (votes count as bets)

**Default**: 10,000 bets per time window  
**Configured in**: `RateLimitConfig.bet_limit`

**Behavior**:
- A value of `0` disables the limit (no rate limiting)
- The limit is enforced *after* authentication but *before* funds are transferred
- In batch operations, the limit is checked for each bet; if any bet would exceed the limit, the entire batch fails atomically (all-or-nothing)

**Error**: `Error::RateLimitExceeded` (mapped from `RateLimiterError::RateLimitExceeded`)

### 2. Per-User Voting Limit (`voting_limit`)

**Bucket Key**: `RateLimiterData::UserVoting(Address)` — global per user

**Actions that consume this limit**:
- `vote` — casting a vote

**Default**: 10,000 votes per time window  
**Configured in**: `RateLimitConfig.voting_limit`

**Behavior**:
- This is a *separate* limit from `bet_limit`, though `vote` consumes both
- The voting limit is now keyed per user globally, not per (user, market)
- This means a user can cast at most `voting_limit` votes across *all* markets in the time window

**Error**: `Error::RateLimitExceeded`

**Note**: Since `vote` is the only action that can be taken once per market (the second vote on the same market fails with `AlreadyVoted`), the per-market keying was redundant and has been changed to per-user.

### 3. Per-Ledger Bet Cap

**Bucket Key**: `DataKey::PerLedgerBetCounter` (ledger sequence, count)

**Actions that consume this cap**:
- `place_bet`
- `place_bets` (each bet in the batch)

**Default**: 0 (disabled)  
**Configured via**: `set_per_ledger_bet_cap(admin, cap)` entrypoint

**Behavior**:
- This is a *global* limit across all users
- Resets automatically when the ledger sequence advances
- Designed to dampen flash-trading bursts
- Independent of the per-user `bet_limit`

**Error**: `Error::PerLedgerBetCapExceeded`

**Setting the cap**:
```rust
client.set_per_ledger_bet_cap(&admin, &100); // Allow max 100 bets per ledger
```

Only the primary admin can set this cap. A cap of `0` disables the limit.

### 4. Oracle Call Limit (`oracle_call_limit`)

**Bucket Key**: `RateLimiterData::OracleCalls(Symbol)` — per market

**Actions that consume this limit**:
- `fetch_oracle_result` — fetching the oracle result for a market (primary and fallback)

**Default**: 1,000 calls per time window  
**Configured in**: `RateLimitConfig.oracle_call_limit`

**Behavior**:
- Enforced *before* the external oracle call is made
- Prevents excessive oracle invocations that could incur high costs
- Each market has its own independent bucket

**Error**: `Error::RateLimitExceeded`

### 5. Dispute Limit (`dispute_limit`)

**Bucket Key**: `RateLimiterData::UserDisputes(Address, Symbol)` — per (user, market)

**Actions that consume this limit**:
- `vote_on_dispute` — voting on a dispute

**Default**: 1,000 disputes per time window  
**Configured in**: `RateLimitConfig.dispute_limit`

**Behavior**:
- Keyed per user and market (a user can dispute multiple markets independently)

**Error**: `Error::RateLimitExceeded`

### 6. Admin Events Limit (`events_per_admin_limit`)

**Bucket Key**: `RateLimiterData::AdminEvents(Address)` — per admin

**Actions that consume this limit**:
- `create_market` — creating a new market

**Default**: 1,000 events per time window  
**Configured in**: `RateLimitConfig.events_per_admin_limit`

**Behavior**:
- A value of `0` disables the limit
- Prevents admins from creating excessive markets

**Error**: `Error::RateLimitExceeded`

## Refill Modes

The rate limiter supports two refill strategies:

### Linear (Default)

The bucket resets fully at the start of each new time window. This is the original behavior.

```rust
RateLimitConfig {
    bet_limit: 100,
    time_window_seconds: 3600,
    refill_mode: RefillMode::Linear,
    // ...
}
```

In this mode, if the time window is 1 hour:
- At 12:00, the user has 100 tokens
- User consumes 100 tokens by 12:30
- At 13:00, the bucket resets to 100 tokens

### HalfLife (Exponential Decay)

Tokens are replenished using an exponential (half-life) model. The number of *consumed* tokens halves every `half_life_seconds` of idle time.

```rust
RateLimitConfig {
    bet_limit: 100,
    time_window_seconds: 3600,
    refill_mode: RefillMode::HalfLife(1800), // 30-minute half-life
    // ...
}
```

In this mode:
- If a user consumes 100 tokens, after 30 minutes, 50 tokens have decayed (50 available)
- After another 30 minutes (60 minutes total), 25 more tokens have decayed (75 available)
- The bucket asymptotically approaches full capacity

## Configuration

### Initialization

Rate limits are seeded during contract initialization with permissive defaults:

```rust
RateLimitConfig {
    voting_limit: 10_000,
    dispute_limit: 1_000,
    oracle_call_limit: 1_000,
    bet_limit: 10_000,
    events_per_admin_limit: 1_000,
    time_window_seconds: 3_600, // 1 hour
    refill_mode: RefillMode::Linear,
}
```

### Updating Limits

Admins can update rate limits using the `update_rate_limits` function (if exposed). The configuration is validated to ensure limits are within reasonable bounds:

- `voting_limit`: 1–10,000
- `dispute_limit`: 1–1,000
- `oracle_call_limit`: 1–1,000
- `bet_limit`: 0–10,000 (0 = disabled)
- `events_per_admin_limit`: 0–1,000 (0 = disabled)
- `time_window_seconds`: 60–2,592,000 (1 minute to 30 days)

### Setting the Per-Ledger Cap

The per-ledger bet cap is set separately:

```rust
client.set_per_ledger_bet_cap(&admin, &50); // Max 50 bets per ledger
```

## Storage and TTL

Rate limit buckets are stored in **temporary storage** with a TTL based on the time window:

```
TTL (ledgers) = (time_window_seconds / LEDGER_SECONDS) + 100
```

Where `LEDGER_SECONDS = 5` (approximate ledger duration on Stellar).

For example, a 1-hour (3,600 second) time window results in:
```
TTL = (3600 / 5) + 100 = 820 ledgers
```

The +100 ledger margin ensures the bucket persists beyond the window to handle clock skew.

**Note**: Prior to issue #1464, the TTL incorrectly treated seconds as ledger counts, causing buckets to persist ~5× longer than intended and inflating rent costs.

## Error Precedence

When multiple checks can fail, errors are returned in this order:

1. **Circuit Breaker** — if the circuit is open, return `Error::CBOpen`
2. **Authentication** — if auth fails, return auth error
3. **Per-Ledger Cap** — if the per-ledger cap is exceeded, return `Error::PerLedgerBetCapExceeded`
4. **Per-User Bet Limit** — if the user's bet limit is exceeded, return `Error::RateLimitExceeded`
5. **Fee Slippage** — if the live fee exceeds `max_fee_bps`, return `Error::FeeExceedsMax`
6. **Market Validation** — other market-specific errors

This precedence ensures that security checks (circuit breaker, auth) are enforced before rate limits, and rate limits are enforced before fee checks.

## Examples

### Example 1: Basic Bet Limit

```rust
// Configure a limit of 2 bets per hour
let config = RateLimitConfig {
    bet_limit: 2,
    time_window_seconds: 3600,
    // ...
};

// User places 2 bets — OK
client.place_bet(&user, &market1, &"Yes", &1000, &0);
client.place_bet(&user, &market2, &"Yes", &1000, &0);

// Third bet fails
let result = client.try_place_bet(&user, &market3, &"Yes", &1000, &0);
assert_eq!(result.unwrap_err(), Error::RateLimitExceeded);
```

### Example 2: Per-Ledger Cap

```rust
// Set per-ledger cap to 1
client.set_per_ledger_bet_cap(&admin, &1);

// First user's bet succeeds
client.place_bet(&user1, &market1, &"Yes", &1000, &0);

// Second user's bet in the same ledger fails
let result = client.try_place_bet(&user2, &market2, &"Yes", &1000, &0);
assert_eq!(result.unwrap_err(), Error::PerLedgerBetCapExceeded);

// Next ledger, the cap resets
env.ledger().with_mut(|li| li.sequence += 1);
client.place_bet(&user2, &market2, &"Yes", &1000, &0); // OK
```

### Example 3: Batch All-or-Nothing

```rust
// Configure a limit of 2 bets per hour
let config = RateLimitConfig { bet_limit: 2, /* ... */ };

// Try to place a batch of 3 bets
let bets = vec![
    (market1, "Yes", 1000),
    (market2, "Yes", 1000),
    (market3, "Yes", 1000),
];

let result = client.try_place_bets(&user, &bets, &0, &idem_key);
assert!(result.is_err()); // Entire batch fails

// The counter was NOT incremented — all-or-nothing semantics
// User can still place 2 individual bets
client.place_bet(&user, &market1, &"Yes", &1000, &0); // OK
client.place_bet(&user, &market2, &"Yes", &1000, &0); // OK
```

## Testing

Rate limit enforcement is tested in `src/rate_limit_enforcement_tests.rs`. The test suite covers:

- Per-user bet limits (single and batch)
- Per-ledger bet cap
- Oracle call limits
- TTL unit conversion (seconds → ledgers)
- HalfLife mode status reporting
- Error precedence (circuit breaker before rate limit)
- Window reset behavior
- All-or-nothing batch semantics

Run the tests:
```bash
cargo test -p predictify-hybrid --lib rate_limit_enforcement
```

## Migration Notes

### Issue #1464 Changes

The following changes were made to wire the dead rate-limit paths:

1. **Per-user bet limit** now enforced in:
   - `place_bet_inner`
   - `place_bets` (batch)
   - `vote`

2. **Oracle call limit** now enforced in:
   - `fetch_oracle_result` (before external oracle call)

3. **Per-ledger bet cap** exposed via:
   - `set_per_ledger_bet_cap(admin, cap)` entrypoint

4. **Voting limit** key changed from `UserVoting(Address, Symbol)` to `UserVoting(Address)`:
   - Now global per user, not per (user, market)
   - Prevents the limit from being bypassed by voting on different markets

5. **TTL units fixed**:
   - Old: `extend_ttl(key, time_window + 86400, time_window + 86400)` (treated seconds as ledgers)
   - New: `extend_ttl(key, (time_window / 5) + 100, ...)` (converts seconds to ledgers)

6. **HalfLife status** now returns accurate `window_reset_time`:
   - When tokens are available: returns current time
   - When at capacity: returns estimated time for next token availability

### Backward Compatibility

- Existing rate limit configurations remain valid
- The voting limit key change means old `UserVoting(Address, Symbol)` entries are orphaned, but this is safe (they just expire naturally)
- No migration script is needed

## See Also

- [Rate Limiter Implementation](../src/rate_limiter.rs)
- [Rate Limit Enforcement Tests](../src/rate_limit_enforcement_tests.rs)
- [Issue #1464](https://github.com/Predictify-org/predictify-contracts/issues/1464)
