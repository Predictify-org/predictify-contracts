#![no_std]

use soroban_sdk::{contract, contractimpl, Env};

#[contract]
pub struct OraclesContract;

#[contractimpl]
impl OraclesContract {
    pub fn version(_env: Env) -> u32 {
        7
    }
}
