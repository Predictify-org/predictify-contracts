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
use crate::reentrancy_guard::{ReentrancyGuard, GuardError as ReentrancyError};
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
        // Guard the external transfer from user -> contract
        ReentrancyGuard::with_external_call(env, || {
            token_client.transfer(&user, &env.current_contract_address(), &amount);
            Ok::<(), ReentrancyError>(())
        })
        .map_err(|_| Error::InvalidState)?;

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
    /// It follows a strict "Check-Transfer-then-Debit" pattern so a failed transfer never leaves
    /// a phantom debit in storage:
    /// 1. Checks: Validate authorization, circuit breaker, and sufficient balance.
    /// 2. Compute: Derive the post-withdraw balance without mutating storage.
    /// 3. Interactions: Execute token transfer (from contract to user).
    /// 4. Effects: Persist the debited balance only after the transfer succeeds.
    ///
    /// # Invariants
    /// - `amount` must be strictly positive.
    /// - Withdrawal is only permitted if the user has sufficient available balance.
    /// - Circuit breaker must allow withdrawals.
    /// - Balance storage is only mutated after the outbound transfer succeeds.
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

        // Resolve token client
        let token_client = match asset {
            ReflectorAsset::Stellar => MarketUtils::get_token_client(env)?,
            _ => return Err(Error::InvalidInput),
        };

        // Compute the resulting balance before interacting with the token contract.
        let balance = BalanceStorage::checked_sub_balance(env, &user, &asset, amount)?;

        // Transfer funds from contract to user (Interactions)
        // If this panics, Soroban rolls back the call and the balance write below is skipped.
        token_client.transfer(&env.current_contract_address(), &user, &amount);

        // Persist the debit only after the token transfer succeeds.
        BalanceStorage::set_balance(env, &balance)?;

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
    extern crate std;

    use super::*;
    use crate::PredictifyHybridClient;
    use soroban_sdk::{
        testutils::{Address as _, EnvTestConfig},
        token::{Client as TokenClient, StellarAssetClient},
        Address, Env, Symbol,
    };
    use std::panic::{catch_unwind, AssertUnwindSafe};

    struct BalanceTestSetup {
        env: Env,
        contract_id: Address,
        token_id: Address,
        user: Address,
        sink: Address,
    }

    impl BalanceTestSetup {
        fn new() -> Self {
            let mut env = Env::default();
            env.set_config(EnvTestConfig {
                capture_snapshot_at_drop: false,
            });
            env.mock_all_auths();

            let token_admin = Address::generate(&env);
            let token_contract = env.register_stellar_asset_contract_v2(token_admin);
            let token_id = token_contract.address();

            let admin = Address::generate(&env);
            let user = Address::generate(&env);
            let sink = Address::generate(&env);

            let contract_id = env.register(crate::PredictifyHybrid, ());
            let client = PredictifyHybridClient::new(&env, &contract_id);
            client.initialize(&admin, &None, &None);

            env.as_contract(&contract_id, || {
                env.storage()
                    .persistent()
                    .set(&Symbol::new(&env, "TokenID"), &token_id);
            });

            StellarAssetClient::new(&env, &token_id).mint(&user, &1_000_0000000);

            BalanceTestSetup {
                env,
                contract_id,
                token_id,
                user,
                sink,
            }
        }

        fn client(&self) -> PredictifyHybridClient<'_> {
            PredictifyHybridClient::new(&self.env, &self.contract_id)
        }

        fn token_client(&self) -> TokenClient<'_> {
            TokenClient::new(&self.env, &self.token_id)
        }
    }

    #[test]
    fn test_deposit_credits_balance_after_transfer() {
        let setup = BalanceTestSetup::new();
        let client = setup.client();
        let token_client = setup.token_client();

        let balance = client.deposit(&setup.user, &ReflectorAsset::Stellar, &500_0000000);

        assert_eq!(balance.amount, 500_0000000);
        assert_eq!(
            client.get_balance(&setup.user, &ReflectorAsset::Stellar).amount,
            500_0000000
        );
        assert_eq!(token_client.balance(&setup.user), 500_0000000);
        assert_eq!(token_client.balance(&setup.contract_id), 500_0000000);
    }

    #[test]
    fn test_withdraw_exact_balance_reaches_zero() {
        let setup = BalanceTestSetup::new();
        let client = setup.client();
        let token_client = setup.token_client();

        client.deposit(&setup.user, &ReflectorAsset::Stellar, &500);
        let balance = client.withdraw(&setup.user, &ReflectorAsset::Stellar, &500);

        assert_eq!(balance.amount, 0);
        assert_eq!(client.get_balance(&setup.user, &ReflectorAsset::Stellar).amount, 0);
        assert_eq!(token_client.balance(&setup.user), 1_000_0000000);
        assert_eq!(token_client.balance(&setup.contract_id), 0);
    }

    #[test]
    fn test_withdraw_over_balance_returns_typed_error_without_mutation() {
        let setup = BalanceTestSetup::new();
        let client = setup.client();

        client.deposit(&setup.user, &ReflectorAsset::Stellar, &100_0000000);

        let result = client.try_withdraw(&setup.user, &ReflectorAsset::Stellar, &150_0000000);

        assert_eq!(result, Err(Ok(Error::InsufficientBalance)));
        assert_eq!(
            client.get_balance(&setup.user, &ReflectorAsset::Stellar).amount,
            100_0000000
        );
    }

    #[test]
    fn test_withdraw_transfer_failure_does_not_leave_phantom_debit() {
        let setup = BalanceTestSetup::new();
        let client = setup.client();
        let token_client = setup.token_client();

        client.deposit(&setup.user, &ReflectorAsset::Stellar, &400_0000000);

        token_client.transfer(&setup.contract_id, &setup.sink, &400_0000000);
        assert_eq!(token_client.balance(&setup.contract_id), 0);

        let result = catch_unwind(AssertUnwindSafe(|| {
            client.withdraw(&setup.user, &ReflectorAsset::Stellar, &100_0000000);
        }));

        assert!(result.is_err());
        assert_eq!(
            client.get_balance(&setup.user, &ReflectorAsset::Stellar).amount,
            400_0000000
        );
        assert_eq!(token_client.balance(&setup.user), 600_0000000);
        assert_eq!(token_client.balance(&setup.contract_id), 0);
    }
}
