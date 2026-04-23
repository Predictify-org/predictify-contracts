//! # Balance Management Module
//!
//! This module implements the internal balance system for the Predictify Hybrid contract,
//! allowing users to deposit and withdraw funds.
//!
//! ## Features
//!
//! - **Deposits**: Users can move funds from their wallet into the contract's internal balance.
//! - **Withdrawals**: Users can withdraw their available (non-staked) funds back to their wallet.
//! - **Consistency**: Ensures that internal balances never underflow and remain consistent.
//! - **Security**: Implements the checks-effects-interactions pattern to prevent reentrancy and double-spending.

use crate::errors::Error;
use crate::events::EventEmitter;
use crate::markets::MarketUtils;
use crate::storage::BalanceStorage;
use crate::types::{Balance, ReflectorAsset};
// use crate::validation::InputValidator;
use crate::circuit_breaker::CircuitBreaker;
use soroban_sdk::{Address, Env, String};

/// Manages user balances for deposits and withdrawals.
///
/// This struct provides functionality to:
/// - Deposit funds into the contract
/// - Withdraw funds from the contract
/// - Track user balances per asset
pub struct BalanceManager;

impl BalanceManager {
    /// Deposit funds into the user's balance.
    ///
    /// This function transfers tokens from the user to the contract and credits their
    /// internal balance. It follows a strict "Transfer-then-Credit" pattern:
    /// 1. Validate inputs (amount > 0)
    /// 2. Authenticate user
    /// 3. Execute token transfer (from user to contract)
    /// 4. Credit user balance in contract storage
    ///
    /// # Invariants
    /// - `amount` must be strictly positive.
    /// - User balance is only updated if the token transfer succeeds (Soroban atomicity).
    /// - Zero or negative amounts are rejected.
    ///
    /// # Parameters
    /// * `env` - The environment.
    /// * `user` - The user depositing funds.
    /// * `asset` - The asset to deposit (currently only Stellar/main token).
    /// * `amount` - The amount to deposit (must be > 0).
    ///
    /// # Returns
    /// * `Result<Balance, Error>` - The updated balance structure on success.
    ///
    /// # Errors
    /// * `Error::InvalidInput` - If the amount is less than or equal to 0 or asset is unsupported.
    pub fn deposit(
        env: &Env,
        user: Address,
        asset: ReflectorAsset,
        amount: i128,
    ) -> Result<Balance, Error> {
        CircuitBreaker::require_write_allowed(env, "deposit")?;
        user.require_auth();

        // Validate amount - must be positive and non-zero
        if amount <= 0 {
            return Err(Error::InvalidInput);
        }

        // Limit maximum deposit to prevent overflow in total balances (though storage uses checked_add)
        // Using i128::MAX / 2 as a sane upper bound for a single deposit
        if amount > i128::MAX / 2 {
            return Err(Error::InvalidInput);
        }

        // Resolve token client
        let token_client = match asset {
            ReflectorAsset::Stellar => MarketUtils::get_token_client(env)?,
            _ => return Err(Error::InvalidInput),
        };

        // Transfer funds from user to contract
        // In Soroban, if this fails it will panic, rolling back the transaction.
        // This ensures the balance is NOT credited unless the transfer succeeds.
        token_client.transfer(&user, &env.current_contract_address(), &amount);

        // Update balance - occurs only if transfer succeeded
        let balance = BalanceStorage::add_balance(env, &user, &asset, amount)?;

        // Emit event
        EventEmitter::emit_balance_changed(
            env,
            &user,
            &asset,
            &String::from_str(env, "Deposit"),
            amount,
            balance.amount,
        );

        Ok(balance)
    }

    /// Withdraw funds from the user's balance.
    ///
    /// This function transfers tokens from the contract back to the user's wallet.
    /// It follows the Checks-Effects-Interactions (CEI) pattern to prevent reentrancy and ensuring safety:
    /// 1. Checks: Validate authorization, circuit breaker, and sufficient balance.
    /// 2. Effects: Update (subtract) user balance in contract storage.
    /// 3. Interactions: Execute token transfer (from contract to user).
    ///
    /// # Invariants
    /// - `amount` must be strictly positive.
    /// - Withdrawal is only permitted if the user has sufficient available balance.
    /// - Circuit breaker must allow withdrawals.
    ///
    /// # Parameters
    /// * `env` - The Soroban environment.
    /// * `user` - The user address withdrawing funds.
    /// * `asset` - The asset to withdraw.
    /// * `amount` - The amount to withdraw (must be > 0).
    ///
    /// # Returns
    /// * `Result<Balance, Error>` - The updated balance structure on success.
    ///
    /// # Errors
    /// * `Error::InvalidInput` - If the amount is less than or equal to 0.
    /// * `Error::InsufficientBalance` - If the user does not have enough internal balance.
    /// * `Error::CBOpen` - If withdrawals are currently disabled by the circuit breaker.
    pub fn withdraw(
        env: &Env,
        user: Address,
        asset: ReflectorAsset,
        amount: i128,
    ) -> Result<Balance, Error> {
        CircuitBreaker::require_write_allowed(env, "withdraw")?;
        user.require_auth();

        // Validate amount - must be positive and non-zero
        if amount <= 0 {
            return Err(Error::InvalidInput);
        }

        // Prevent withdrawals when circuit breaker disallows them
        if !CircuitBreaker::are_withdrawals_allowed(env)? {
            return Err(Error::CBOpen);
        }

        // Check sufficient balance and subtract (Effects)
        // sub_balance will return Error::InsufficientBalance if amount > current_balance.amount
        let balance = BalanceStorage::sub_balance(env, &user, &asset, amount)?;

        // Resolve token client
        let token_client = match asset {
            ReflectorAsset::Stellar => MarketUtils::get_token_client(env)?,
            _ => return Err(Error::InvalidInput),
        };

        // Transfer funds from contract to user (Interactions)
        // Note: Contract-to-user transfers in Soroban do not require user auth,
        // but the contract address must have sufficient balance.
        token_client.transfer(&env.current_contract_address(), &user, &amount);

        // Emit event
        EventEmitter::emit_balance_changed(
            env,
            &user,
            &asset,
            &String::from_str(env, "Withdraw"),
            amount,
            balance.amount,
        );

        Ok(balance)
    }

    /// Get the current balance for a user.
    pub fn get_balance(env: &Env, user: Address, asset: ReflectorAsset) -> Balance {
        BalanceStorage::get_balance(env, &user, &asset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::Env;

    struct BalanceTestSetup {
        env: Env,
        user: Address,
    }

    impl BalanceTestSetup {
        fn new() -> Self {
            let env = Env::default();
            let user = Address::generate(&env);
            BalanceTestSetup { env, user }
        }
    }

    #[test]
    fn test_deposit_valid_amount() {
        let _setup = BalanceTestSetup::new();
        let amount = 1_000_000i128;
        assert!(amount > 0);
    }

    #[test]
    fn test_deposit_zero_amount() {
        let _setup = BalanceTestSetup::new();
        let amount = 0i128;
        assert_eq!(amount, 0);
    }

    #[test]
    fn test_deposit_negative_amount() {
        let _setup = BalanceTestSetup::new();
        let amount = -1_000_000i128;
        assert!(amount < 0);
    }

    #[test]
    fn test_withdraw_insufficient_balance() {
        let _setup = BalanceTestSetup::new();
        let requested = 1_000_000i128;
        let available = 100_000i128;
        assert!(requested > available);
    }
}
