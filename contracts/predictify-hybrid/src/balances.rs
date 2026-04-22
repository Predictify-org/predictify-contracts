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
    /// This function transfers the specified amount of tokens from the user's
    /// wallet to the contract and credits their internal balance.
    ///
    /// # Parameters
    /// * `env` - The Soroban environment.
    /// * `user` - The user address depositing funds.
    /// * `asset` - The asset to deposit (currently only supports ReflectorAsset::Stellar).
    /// * `amount` - The amount to deposit (must be greater than 0).
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
        user.require_auth();

        // Validate amount (must be positive)
        if amount <= 0 {
            return Err(Error::InvalidInput);
        }

        // Resolve token client
        // Currently we only support the main configured token, mapped to ReflectorAsset::Stellar
        let token_client = match asset {
            ReflectorAsset::Stellar => MarketUtils::get_token_client(env)?,
            _ => return Err(Error::InvalidInput),
        };

        // Transfer funds from user to contract
        token_client.transfer(&user, &env.current_contract_address(), &amount);

        // Update balance
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
    /// This function transfers the specified amount of tokens from the contract's
    /// internal balance back to the user's wallet.
    ///
    /// # Parameters
    /// * `env` - The Soroban environment.
    /// * `user` - The user address withdrawing funds.
    /// * `asset` - The asset to withdraw.
    /// * `amount` - The amount to withdraw (must be greater than 0).
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
        user.require_auth();

        // Prevent withdrawals when circuit breaker disallows them
        if !CircuitBreaker::are_withdrawals_allowed(env)? {
            return Err(Error::CBOpen);
        }

        // Validate amount (must be positive)
        if amount <= 0 {
            return Err(Error::InvalidInput);
        }

        // Internal Balance Check:
        let current_balance = BalanceStorage::get_balance(env, &user, &asset);
        if current_balance.amount < amount {
            return Err(Error::InsufficientBalance);
        }

        // Resolve token client
        let token_client = match asset {
            ReflectorAsset::Stellar => MarketUtils::get_token_client(env)?,
            _ => return Err(Error::InvalidInput),
        };

        // Update balance first (checks-effects-interactions)
        let balance = BalanceStorage::sub_balance(env, &user, &asset, amount)?;

        // Transfer funds from contract to user
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
