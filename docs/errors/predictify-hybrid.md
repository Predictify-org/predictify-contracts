# Predictify-Hybrid Error Catalog

This document describes the `Error` variants for the `predictify-hybrid` smart
contract (module `err.rs`).

---

## `MarketClosed` — Code `102`

### Summary

The market is closed and cannot accept new bets or operations.

### When returned

| Entrypoint | Condition |
|---|---|
| `place_bet` / `place_bets` | `market.state != Active` **or** `current_time >= bet_deadline` |
| `cancel_bet` | `current_time >= market.end_time` |
| `resolve_market_manual` | `current_time < market.end_time` (market hasn't ended yet) |
| `resolve_market_with_ties` | `current_time < market.end_time` (market hasn't ended yet) |
| `fetch_oracle_result` | `current_time < market.end_time` |
| `MarketResolutionValidator::validate_market_for_resolution` | `market.is_active()` returns `true` |
| `OracleResolutionValidator::validate_market_for_oracle_resolution` | `current_time < market.end_time` |

### Recovery guidance

This is a **terminal** error for the current call.

- **For bettors**: The market's betting window is closed. Look for a different
  active market, or wait for a new one to open.
- **For admins resolving a market**: Wait until `market.end_time` has passed
  before calling `resolve_market_manual` or `resolve_market_with_ties`.

Retrying without changing the market state will always fail.

### Recoverability

`Abort` — the SDK recovery strategy is `RecoveryStrategy::Abort`.

---

## Full Error Table

| Code | Name | Description |
|---|---|---|
| `100` | `Unauthorized` | User lacks the required permissions for this action. |
| `101` | `MarketNotFound` | Market ID is unknown or the market has been removed. |
| `102` | `MarketClosed` | Market has passed its deadline and cannot accept new operations. |
| `103` | `MarketResolved` | Market has already been resolved; no further betting allowed. |
| `104` | `MarketNotResolved` | Oracle resolution is still pending; winnings cannot be claimed yet. |
| `105` | `NothingToClaim` | The caller has no winnings to claim from this market. |
| `106` | `AlreadyClaimed` | Winnings have already been claimed; duplicate claims are rejected. |
| `107` | `InsufficientStake` | Bet amount is below the minimum required threshold. |
| `108` | `InvalidOutcome` | Chosen outcome does not exist in this market. |
| `109` | `AlreadyVoted` | User has already voted; only one vote per market is allowed. |
| `110` | `AlreadyBet` | User has already placed a bet on this market. |
| `111` | `BetsAlreadyPlaced` | Market parameters cannot be updated after bets have been placed. |
| `112` | `InsufficientBalance` | User balance is too low for the requested operation. |
| `200` | `OracleUnavailable` | External oracle is down or unreachable. |
| `201` | `InvalidOracleConfig` | Oracle configuration is malformed or invalid. |
| `202` | `OracleStale` | Oracle data exceeds the freshness threshold. |
| `203` | `OracleNoConsensus` | Multiple oracle instances could not reach consensus. |
| `300` | `InvalidQuestion` | Market question is empty or fails validation. |
| `301` | `InvalidOutcomes` | Outcome list is invalid (too few, duplicates, or empty). |
| `302` | `InvalidDuration` | Market duration is outside the allowed range (1–365 days). |
| `401` | `InvalidInput` | One or more parameters failed validation. |
| `494` | `InvalidState` | Contract or market is in an unexpected/illegal state. |

> For the complete list of all 100+ error codes see `src/err.rs`.
