#![no_std]

use soroban_sdk::{contract, contractimpl, Env};

pub mod errors;

#[contract]
pub struct ResolutionContract;

#[contractimpl]
impl ResolutionContract {
    /// Returns the contract version for the resolution subsystem.
    pub fn version(_env: Env) -> u32 {
        7
    }
}
