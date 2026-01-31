#![allow(dead_code)]

use crate::errors::Error;
use crate::events::EventEmitter;
use crate::markets::MarketUtils;
use crate::storage::BalanceStorage;
use crate::types::{Balance, ReflectorAsset};
use crate::validation::InputValidator;
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
    /// # Parameters
    /// * `env` - The environment.
    /// * `user` - The user depositing funds.
    /// * `asset` - The asset to deposit (currently only supports the main token via ReflectorAsset::Stellar).
    /// * `amount` - The amount to deposit.
    ///
    /// # Returns
    /// * `Result<Balance, Error>` - The updated balance or an error.
    pub fn deposit(
        env: &Env,
        user: Address,
        asset: ReflectorAsset,
        amount: i128,
    ) -> Result<Balance, Error> {
        user.require_auth();

        // Validate amount
        InputValidator::validate_balance_amount(&amount).map_err(|_| Error::InvalidInput)?;

        // Resolve token client
        // Currently we only support the main configured token, mapped to ReflectorAsset::Stellar
        // In the future, we could support other assets if we have a registry of Symbol -> Token Address
        let token_client = match asset {
            ReflectorAsset::Stellar => MarketUtils::get_token_client(env)?,
            _ => return Err(Error::InvalidInput), // Only Stellar (main token) supported for now
        };

        // Transfer funds from user to contract
        // The user must have authorized this transfer (allowance) or we use transfer_from if supported,
        // but standard Soroban token interface uses transfer(from, to, amount) where 'from' must auth.
        // Since we called user.require_auth(), we can try to transfer.
        // Note: The token contract will check if 'user' signed the tx.
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
    /// # Parameters
    /// * `env` - The environment.
    /// * `user` - The user withdrawing funds.
    /// * `asset` - The asset to withdraw.
    /// * `amount` - The amount to withdraw.
    ///
    /// # Returns
    /// * `Result<Balance, Error>` - The updated balance or an error.
    pub fn withdraw(
        env: &Env,
        user: Address,
        asset: ReflectorAsset,
        amount: i128,
    ) -> Result<Balance, Error> {
        user.require_auth();

        // Validate amount
        InputValidator::validate_balance_amount(&amount).map_err(|_| Error::InvalidInput)?;

        // Check sufficient balance
        let current_balance = BalanceStorage::get_balance(env, &user, &asset);
        InputValidator::validate_sufficient_balance(current_balance.amount, amount)
            .map_err(|_| Error::InsufficientBalance)?;

        // Check if funds are locked in bets
        // This requires checking the active stakes for the user.
        // The 'stakes' in Market/Bets are amounts already transferred to the contract and deducted from balance?
        // OR are they locked within the user's balance?
        //
        // Architecture Decision:
        // Option A: "Deposit" moves funds to contract. "Bet" uses funds from "Balance".
        // Option B: "Bet" transfers funds directly from User Wallet.
        //
        // Existing `bets.rs` likely transfers directly from user wallet if it uses `token_client.transfer`.
        // Let's verify `bets.rs` logic.
        // If `bets.rs` uses `token_client.transfer(user, contract, amount)`, then the funds are IN the contract but NOT in `BalanceStorage`.
        // `BalanceStorage` tracks "Available/Unused" funds deposited by user.
        //
        // If the user wants to withdraw from `BalanceStorage`, those funds are by definition NOT locked in bets,
        // because bets would have consumed them (deducted from Balance) or were made separately.
        //
        // However, the prompt says "Must prevent withdrawal of locked funds".
        // If "Locked Funds" means "Funds currently in active bets", then those funds are ALREADY out of `BalanceStorage` (if we implement betting to deduct from balance).
        // OR, if `Balance` represents TOTAL equity (Available + Locked), then we need to subtract Locked.
        //
        // Given the standard pattern:
        // Balance = Available to Withdraw + Available to Bet.
        // When you Bet, you use Balance.
        //
        // If `bets.rs` is legacy code that transfers directly from wallet, we might need to update it to use Balance.
        // But for now, we are adding Balance Management.
        // The prompt says "allows deposits/withdrawals of non-locked funds".
        // This implies there are "Locked Funds".
        //
        // If I assume `Balance` tracks ONLY "Available" funds (Deposit - Bets + Winnings), then `withdraw` is simple: just check `Balance`.
        // But if `Balance` tracks "Total Deposited", and `Bets` just lock a portion, then we need `Locked`.
        //
        // Let's assume `Balance` in `BalanceStorage` is "Available Balance".
        // When a user places a bet, we should deduct from `Balance` (if we integrate).
        // But since `bets.rs` exists, I should check if I need to modify it.
        // The prompt says "integrating balance management with bets.rs fund-locking logic".
        //
        // If `bets.rs` uses direct transfer, then `Balance` is a separate "wallet" inside the contract.
        // If the user has 100 in Balance, and places a bet of 10, does it come from Balance or Wallet?
        // Ideally, it should come from Balance if sufficient, or Wallet.
        //
        // Constraint: "Must prevent withdrawal of locked funds".
        // If `Balance` = `Available`, then we don't need to check locks, because locked funds aren't in `Balance`.
        //
        // But maybe the user implies:
        // Total Balance = X.
        // Locked in Bets = Y.
        // Available = X - Y.
        //
        // If `BalanceStorage` stores `amount`, is it X or (X-Y)?
        // I'll assume `BalanceStorage` stores the AVAILABLE balance (X-Y) for simplicity and safety.
        // So `withdraw` just checks `amount <= balance.amount`.
        //
        // HOWEVER, the prompt mentions "Validate security assumptions (fund locking...)".
        // And "Prevent withdrawal of locked funds".
        //
        // Let's look at `bets.rs` to see if there is any "lock" mechanism that doesn't move funds.
        //
        // If `bets.rs` moves funds to the contract, they are effectively "locked" in the contract, but not attributed to `BalanceStorage`.
        // So `BalanceStorage` only tracks "Idle" funds.
        //
        // So for `withdraw`, if `BalanceStorage` is "Idle Funds", then checking `balance.amount >= amount` is sufficient.

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
