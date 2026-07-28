#![no_std]

use soroban_sdk::{contract, contractimpl, Env};

pub mod errors;
pub mod admin;

#[contract]
pub struct MarketsContract;

#[contractimpl]
impl MarketsContract {
    pub fn version(_env: Env) -> u32 {
        7
    }
}
