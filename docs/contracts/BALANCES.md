# Balance Management Invariants and Token Safety

This document outlines the security invariants and token transfer semantics for balance management in the Predictify Hybrid smart contracts.

## Overview

The `BalanceManager` is responsible for handling user deposits and withdrawals. It ensures that internal contract balances are always synchronized with actual token transfers on the Stellar network (Soroban).

## Deposit Invariants

The `deposit` function follows a strict **Transfer-then-Credit** pattern to ensure safety:

1.  **Validation**: The `amount` must be strictly positive (> 0).
2.  **Authentication**: The `user` must authorize the transaction via `require_auth()`.
3.  **Token Transfer**: The contract executes `token_client.transfer(user, contract, amount)`. 
    - In Soroban, if the transfer fails (e.g., insufficient funds, lack of authorization), the entire transaction panics and reverts.
4.  **Balance Credit**: Only after a successful transfer is the user's internal balance updated in `BalanceStorage`.

### Safety Assumptions
- **Atomicity**: We rely on Soroban's transaction atomicity. If any step fails, no state changes (including token transfers and balance updates) are committed.
- **Positive Amounts Only**: We explicitly reject zero or negative deposit amounts to prevent edge-case logic errors.
- **Large Amount Handling**: Deposits are restricted to half of the `i128` maximum value to prevent theoretical overflow when aggregating balances, although `BalanceStorage` uses checked arithmetic.
- **Ledger Reconciliation**: A deposit credit is never written unless the incoming token transfer has already succeeded.

## Withdrawal Invariants

The `withdraw` function follows a strict **Check-Transfer-then-Debit** pattern:

1.  **Checks**:
    - Validate `amount` > 0.
    - Authenticate user.
    - Check if the Circuit Breaker allows withdrawals.
    - Compute the post-withdraw balance and reject `amount > balance` with `Error::InsufficientBalance`.
2.  **Interactions**:
    - Transfer tokens from the contract's account back to the user's wallet.
3.  **Effects**:
    - Persist the debited balance in `BalanceStorage` only after the transfer succeeds.

### Safety Assumptions
- **Balance Separation**: Internal balances track "Available" funds. Funds currently locked in active bets are handled separately by the `bets.rs` module and are not part of the `BalanceStorage` amount unless specifically credited back (e.g., through winnings or refunds).
- **Circuit Breaker**: High-level platform safety is maintained through the circuit breaker mechanism.
- **Typed Underflow Protection**: `BalanceStorage::sub_balance` and `checked_sub_balance` reject over-withdrawal with `Error::InsufficientBalance` before any write, rather than relying on wrapping arithmetic.
- **Transfer Revert Safety**: If the outbound token transfer fails or reverts, the balance write is never reached and Soroban rolls back the call, preventing phantom debits.

## Storage-Level Invariants

`BalanceStorage` is the source of truth for idle user funds and enforces these rules at every mutation site:

1.  Balance deltas passed to `add_balance` and `sub_balance` must be strictly positive.
2.  Stored balances must never become negative.
3.  `checked_add_balance` and `checked_sub_balance` compute the exact next state before persistence so callers can pair the storage write with the corresponding token transfer.

## Fund Locking and Betting Integration

There are two primary ways funds are held in the contract:

1.  **Idle Balances**: Funds deposited via `BalanceManager` and tracked in `BalanceStorage`. These are "idle" and available for withdrawal or for future integration with betting.
2.  **Locked Stakes**: Funds transferred directly to the contract during the `vote` or `place_bet` process. These funds are **NOT** reflected in `BalanceStorage` while the bet is active. They are effectively "locked" in the contract's total token balance but attributed to specific markets/bets.

### Payouts and Refunds
- When a market is resolved or cancelled, funds are either:
    - Transferred directly to the user (e.g., `refund_market_bets`).
    - Credited to the user's balance for later withdrawal (future optimization).
- The current implementation typically handles payouts/refunds through direct transfers to the user's wallet.

## Regression Testing

Test coverage for balance invariants includes:
- `test_deposit_credits_balance_after_transfer`: Deposit credits the internal ledger only after the token transfer succeeds.
- `test_withdraw_exact_balance_reaches_zero`: Withdrawing the full balance reaches an exact zero state.
- `test_withdraw_over_balance_returns_typed_error_without_mutation`: Over-withdrawal returns `Error::InsufficientBalance` and leaves storage untouched.
- `test_withdraw_transfer_failure_does_not_leave_phantom_debit`: A failed outbound transfer does not leave a debited internal balance behind.
- `test_sub_balance_rejects_overdraw_without_mutation`: The storage helper rejects underflow before persisting.
- `test_balance_mutators_reject_non_positive_amounts`: Balance mutation helpers reject zero or negative deltas.

## Security Notes

- **Threat Model**: An attacker might try to credit their balance without transferring tokens, or withdraw more tokens than they have.
- **Proven Invariants**:
    - `Total Internal Balances <= Total Contract Token Balance`.
    - `User A cannot withdraw User B's funds`.
    - `Internal balance change <=> Successful Token Transfer`.
