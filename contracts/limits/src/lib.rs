#![no_std]
use soroban_sdk::{contract, contractimpl, Env};
pub mod errors;
use errors::LimitError;

#[contract]
pub struct Limits;

#[contractimpl]
impl Limits {
    pub fn validate_bet_amount(env: Env, amount: u64, min: u64, max: u64) -> Result<(), LimitError> {
        if amount < min { return Err(LimitError::BetBelowMinimum); }
        if amount > max { return Err(LimitError::BetExceedsMaximum); }
        Ok(())
    }

    pub fn validate_leverage(env: Env, leverage: u32, max_leverage: u32) -> Result<(), LimitError> {
        if leverage == 0 { return Err(LimitError::LeverageMustBePositive); }
        if leverage > max_leverage { return Err(LimitError::LeverageExceedsMax); }
        Ok(())
    }

    pub fn validate_fee(env: Env, fee_bps: u32, max_fee_bps: u32) -> Result<(), LimitError> {
        if fee_bps > max_fee_bps { return Err(LimitError::FeeExceedsMax); }
        Ok(())
    }
}

