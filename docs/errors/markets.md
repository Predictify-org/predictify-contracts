# Markets Error Catalog

This document describes the `ContractError` variants for the markets smart contract.

## Error Codes

| Code | Name | Description |
| ---- | ---- | ----------- |
| `1` | `Unauthorized` | Action is unauthorized; typically thrown when an admin action is invoked by a non-admin. |
| `2` | `MarketNotFound` | Market not found; thrown when querying or interacting with a non-existent market. |
| `3` | `MarketClosed` | Market is closed; thrown when trying to interact with a market that has already ended. |
| `4` | `MarketAlreadyResolved` | Market already resolved; thrown when attempting to resolve a market more than once. |
| `5` | `MarketNotResolved` | Market not resolved; thrown when attempting to claim winnings before resolution. |
| `6` | `InvalidOutcome` | Invalid outcome; thrown when the provided outcome is not supported by the market. |
| `7` | `InvalidConfig` | Invalid configuration; thrown when market setup parameters are out of bounds. |
| `8` | `Overflow` | Overflow; thrown when math operations overflow, preventing unsafe state changes. |
| `9` | `StakeTooSmall` | Stake too small; thrown when a deposit or bet is below the minimum threshold. |
| `10` | `InvalidState` | Invalid State; thrown when a generic state transition fails. |

---

## `MarketClosed` — Code `3`

### When returned

The `MarketClosed` error is returned whenever a caller attempts to interact
with a market that is no longer accepting operations:

- Placing a bet after the market's deadline.
- Attempting to vote or otherwise participate once the market has ended.

### Recovery guidance

This error is **terminal** for the current call. The caller should:

1. Verify the market ID is correct.
2. Check that the market's end time has not passed.
3. Look for a different market that is still active.

Retrying without a state change will always fail.

---

> See also: [`predictify-hybrid.md`](./predictify-hybrid.md) for the
> `MarketClosed = 102` error in the hybrid contract.
