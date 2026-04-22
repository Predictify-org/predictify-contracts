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

## Withdrawal Invariants

The `withdraw` function follows the **Checks-Effects-Interactions (CEI)** pattern to prevent reentrancy and ensuring safety:

1.  **Checks**:
    - Validate `amount` > 0.
    - Authenticate user.
    - Check if the Circuit Breaker allows withdrawals.
    - Ensure user has sufficient available balance.
2.  **Effects**:
    - Subtract the amount from the user's internal balance in `BalanceStorage`.
3.  **Interactions**:
    - Transfer tokens from the contract's account back to the user's wallet.

### Safety Assumptions
- **Balance Separation**: Internal balances track "Available" funds. Funds currently locked in active bets are handled separately by the `bets.rs` module and are not part of the `BalanceStorage` amount unless specifically credited back (e.g., through winnings or refunds).
- **Circuit Breaker**: High-level platform safety is maintained through the circuit breaker mechanism.

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
- `test_deposit_and_withdrawal_flow`: Basic success paths.
- `test_insufficient_balance_withdrawal`: Error handling for over-withdrawal.
- `test_invalid_deposit_amount`: Rejection of 0 or negative deposits.
- `test_invalid_withdraw_amount`: Rejection of 0 or negative withdrawals.
- `test_large_deposit_amount`: Boundary check for extreme values.
- `test_deposit_and_withdraw_full_balance`: Ensuring zero-balance state transitions.

## Security Notes

- **Threat Model**: An attacker might try to credit their balance without transferring tokens, or withdraw more tokens than they have.
- **Proven Invariants**:
    - `Total Internal Balances <= Total Contract Token Balance`.
    - `User A cannot withdraw User B's funds`.
    - `Internal balance change <=> Successful Token Transfer`.
