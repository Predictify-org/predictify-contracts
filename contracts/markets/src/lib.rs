#![no_std]

use soroban_sdk::{contract, contractimpl, Env};

#[contract]
pub struct MarketsContract;

#[contractimpl]
impl MarketsContract {
    pub fn version(_env: Env) -> u32 {
        7
    }

    /// Read a market from persistent storage and bump its TTL.
    pub fn get_market(env: Env, market_id: soroban_sdk::Symbol) -> Option<soroban_sdk::Val> {
        let market: Option<soroban_sdk::Val> = env.storage().persistent().get(&market_id);
        if market.is_some() {
            // Bump TTL: 365 days * 17280 ledgers per day = 6307200
            env.storage().persistent().extend_ttl(&market_id, 6307200, 6307200);
        }
        market
    }

}
