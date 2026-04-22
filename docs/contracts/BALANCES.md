# Balance Management System

The Balance Management system in Predictify Hybrid handles the internal accounting of user funds. It allows users to deposit assets into the contract, use those funds for betting, and withdraw unused funds back to their wallets.

## Core Concepts

### Available vs. Locked Funds
In the Predictify Hybrid architecture:
- **Available Balance**: Funds that have been deposited but are not currently committed to any active bets. These are tracked in `BalanceStorage`.
- **Locked Funds**: Funds that are actively staked in prediction markets. When a bet is placed, funds are either moved directly from the user's wallet to the contract's address or deducted from their internal balance. Once staked, these funds are no longer withdrawable until the market resolves.

## Withdrawal Security

To ensure the safety of user funds and the integrity of the contract, the withdrawal process follows these security invariants:

1. **Amount Validation**: All withdrawal requests must specify an amount greater than zero. Requests for zero or negative amounts are rejected with `Error::InvalidInput`.
2. **Sufficient Balance Check**: The contract verifies that the user has a sufficient internal balance in `BalanceStorage` before proceeding. This balance represents "liquid" funds.
3. **Checks-Effects-Interactions**: The contract updates the internal state (deducting the balance) *before* performing the external token transfer. This prevents reentrancy attacks and ensures consistency.
4. **Circuit Breaker Integration**: Withdrawals can be globally paused by administrators in case of emergencies or detected anomalies using the Circuit Breaker system.

## Withdrawal Flow

1.  **Authentication**: The user must authenticate the withdrawal request via `require_auth()`.
2.  **Circuit Breaker Check**: The system verifies that withdrawals are currently enabled.
3.  **Validation**: The amount is checked for positivity.
4.  **Balance Check**: The user's current liquid balance is retrieved and checked against the requested amount.
5.  **State Update**: The balance is deducted from `BalanceStorage` using safe arithmetic (`checked_sub`).
6.  **Token Transfer**: Tokens are transferred from the contract's address to the user's address.
7.  **Event Emission**: A `balance_changed` event is emitted with the "Withdraw" operation type.

## Error Handling

- `Error::InsufficientBalance`: Returned if the user attempts to withdraw more than their available liquid balance.
- `Error::InvalidInput`: Returned if the amount is non-positive or if an unsupported asset is specified.
- `Error::CBOpen`: Returned if withdrawals are currently paused by the circuit breaker.
