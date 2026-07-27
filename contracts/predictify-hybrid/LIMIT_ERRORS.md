# Semantic Limit Errors (600–611)

Bound violations in the contract previously surfaced as a generic
`Error::InvalidInput` (401) or, in one case, as a bare `panic!`. A client that
received 401 could not tell *which* limit it had hit, and could not decide whether
to retry with a smaller value or escalate a bad configuration to an operator.

This document describes the semantic limit codes that replace them.

## Error codes

| Code | Variant | String code | Raised when |
|------|---------|-------------|-------------|
| 600 | `BetAboveMaximum` | `BET_ABOVE_MAXIMUM` | Bet amount exceeds the market's effective `max_bet` (or `MAX_BET_AMOUNT`) |
| 601 | `BetLimitsInverted` | `BET_LIMITS_INVERTED` | Admin submits `BetLimits` with `min_bet > max_bet` |
| 602 | `BetLimitAboveMaximum` | `BET_LIMIT_ABOVE_MAXIMUM` | Admin submits `max_bet` above `MAX_BET_AMOUNT` |
| 603 | `BetCapOutOfRange` | `BET_CAP_OUT_OF_RANGE` | Per-market single-bet cap is not in `(0, MAX_BET_AMOUNT]` |
| 604 | `BatchEmpty` | `BATCH_EMPTY` | `place_bets` called with zero entries |
| 605 | `BatchSizeExceeded` | `BATCH_SIZE_EXCEEDED` | `place_bets` called with more than `MAX_BATCH_SIZE` (50) entries |
| 606 | `FeePercentageOutOfRange` | `FEE_PERCENTAGE_OUT_OF_RANGE` | Fee basis points outside `[MIN_FEE_PERCENTAGE, MAX_FEE_PERCENTAGE]`, or outside ±20% of the tier fee |
| 607 | `FeeAmountAboveMaximum` | `FEE_AMOUNT_ABOVE_MAXIMUM` | Fee amount above `MAX_FEE_AMOUNT` |
| 608 | `CreationFeeOutOfRange` | `CREATION_FEE_OUT_OF_RANGE` | Creation fee outside `[MIN_FEE_AMOUNT, MAX_FEE_AMOUNT]` |
| 609 | `FeeLimitsInverted` | `FEE_LIMITS_INVERTED` | `FeeConfig` with `max_fee_amount < min_fee_amount` |
| 610 | `QueueCapacityOutOfRange` | `QUEUE_CAPACITY_OUT_OF_RANGE` | Monitor queue capacity outside `[MIN_QUEUE_CAPACITY, MAX_QUEUE_CAPACITY]` |
| 611 | `QueueAlreadyInitialized` | `QUEUE_ALREADY_INITIALIZED` | Monitor queue re-initialization attempt (previously a `panic!`) |

Codes 612–619 are reserved for future limit variants.

## Naming convention

| Suffix | Meaning |
|--------|---------|
| `*AboveMaximum` | A value exceeded an upper bound |
| `*OutOfRange` | A value fell outside an inclusive `[min, max]` window |
| `*LimitsInverted` | An admin-supplied `min`/`max` pair is self-contradictory |

## Breaking changes

These are **behavioural changes to returned error codes**. Off-chain clients that
matched on `InvalidInput` (401) for the paths below must be updated.

| Location | Before | After |
|----------|--------|-------|
| `bets::set_market_max_bet_cap` — cap ≤ 0 or > `MAX_BET_AMOUNT` | `InvalidInput` | `BetCapOutOfRange` |
| `bets::validate_limits_bounds` — `min_bet > max_bet` | `InvalidInput` | `BetLimitsInverted` |
| `bets::validate_limits_bounds` — `max_bet > MAX_BET_AMOUNT` | `InvalidInput` | `BetLimitAboveMaximum` |
| `BetValidator::validate_bet_amount` — above max | `InvalidInput` | `BetAboveMaximum` |
| `BetValidator::validate_bet_amount_against_limits` — above effective max | `InvalidInput` | `BetAboveMaximum` |
| `BetManager::place_bets` — empty batch | `InvalidInput` | `BatchEmpty` |
| `BetManager::place_bets` — oversized batch | `InvalidInput` | `BatchSizeExceeded` |
| `FeeManager::update_fee_structure` — tier fee out of range | `InvalidInput` | `FeePercentageOutOfRange` |
| `FeeCalculator::validate_fee_percentage` — out of range or off-tier | `InvalidInput` | `FeePercentageOutOfRange` |
| `FeeValidator::validate_fee_amount` — above max | `InvalidInput` | `FeeAmountAboveMaximum` |
| `FeeValidator::validate_creation_fee` — out of range | `InvalidInput` | `CreationFeeOutOfRange` |
| `FeeValidator::validate_fee_config` — percentage out of range | `InvalidInput` | `FeePercentageOutOfRange` |
| `FeeValidator::validate_fee_config` — `max_fee_amount < min_fee_amount` | `InvalidInput` | `FeeLimitsInverted` |
| `BoundedMonitorQueue::initialize` — capacity out of range | `InvalidInput` | `QueueCapacityOutOfRange` |
| `BoundedMonitorQueue::initialize` — already initialized | `panic!` | `Err(QueueAlreadyInitialized)` |

### Deliberately unchanged

* **Lower bounds on stake and fee amounts** keep returning `InsufficientStake` —
  that code is already semantic, and changing it would break clients for no gain.
* **Sign checks** (`creation_fee < 0`, `min_fee_amount < 0`, …) keep returning
  `InvalidInput`. A negative amount is a malformed input, not a bound violation.
* **`BetExceedsCap` (509)** is unchanged. It remains the code for a bet that
  breaches an admin-configured per-market cap; `BetAboveMaximum` (600) is the
  market's own ceiling. Both can apply, in which case `BetAboveMaximum` wins —
  see "Precedence" below.

## Precedence

`validate_bet_amount_against_limits` checks bounds from general to specific:

1. `amount < min_bet` → `InsufficientStake`
2. `amount > max_bet` → `BetAboveMaximum`
3. `amount > per-market cap` → `BetExceedsCap`

A bet that violates both (2) and (3) reports `BetAboveMaximum`, so the caller
learns the bound that applies to everyone before the one an admin added.

## Client handling

Each variant carries a `RecoveryStrategy` reflecting who can act on it:

| Group | Variants | Strategy |
|-------|----------|----------|
| Caller supplied a value that is too large/small | `BetAboveMaximum`, `BatchEmpty`, `BatchSizeExceeded` | `Retry` — resubmit with a smaller value |
| Admin configuration is itself invalid | `BetLimitsInverted`, `BetLimitAboveMaximum`, `BetCapOutOfRange`, `QueueCapacityOutOfRange`, `FeePercentageOutOfRange`, `FeeAmountAboveMaximum`, `CreationFeeOutOfRange`, `FeeLimitsInverted` | `Abort` — an operator must correct the configuration |
| Benign no-op | `QueueAlreadyInitialized` | `Skip` |

`Error::code()` and `Error::description()` return a stable uppercase identifier and
a human-readable string for every variant above.

## Tests

| File | Covers |
|------|--------|
| `src/limit_errors_tests.rs` | Call-site mapping, boundary inclusivity, precedence, taxonomy invariants (unique codes, reserved range, non-`Unknown` classification, no fallback to `InvalidInput`) |
| `src/monitor.rs` (`mod tests`) | Queue capacity range and typed double-initialization rejection |
| `tests/err_stability.rs` | Freezes discriminants 600–611 against reordering or deletion |
