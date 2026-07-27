#![no_std]

use soroban_sdk::{contract, contractimpl, Env};

#[contract]
pub struct DisputesContract;

#[contractimpl]
impl DisputesContract {
    pub fn version(_env: Env) -> u32 {
        7
    }
}
