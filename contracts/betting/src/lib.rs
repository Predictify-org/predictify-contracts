#![no_std]

use soroban_sdk::{contract, contractimpl, Address, Env, String, Symbol, contracttype};

const DAY_IN_LEDGERS: u32 = 17280;
const THRESHOLD: u32 = DAY_IN_LEDGERS * 7;
const EXTEND_TO: u32 = DAY_IN_LEDGERS * 14;

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Bet(Address, String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct BetInfo {
    pub amount: i128,
    pub outcome: String,
}

#[contract]
pub struct BettingContract;

#[contractimpl]
impl BettingContract {
    /// Places a bet.
    /// 
    /// # Arguments
    /// * `env` - The environment
    /// * `bettor` - The address of the user placing the bet
    /// * `market_id` - The ID of the market
    /// * `amount` - The amount of the bet
    /// * `outcome` - The selected outcome
    pub fn place_bet(env: Env, bettor: Address, market_id: String, amount: i128, outcome: String) {
        bettor.require_auth();
        
        let key = DataKey::Bet(bettor.clone(), market_id.clone());
        let info = BetInfo { amount, outcome };
        
        env.storage().persistent().set(&key, &info);
        env.storage().persistent().extend_ttl(&key, THRESHOLD, EXTEND_TO);
    }

    /// Reads a bet and bumps the TTL if it is within the threshold.
    /// 
    /// # Arguments
    /// * `env` - The environment
    /// * `bettor` - The address of the user placing the bet
    /// * `market_id` - The ID of the market
    pub fn get_bet(env: Env, bettor: Address, market_id: String) -> Option<BetInfo> {
        let key = DataKey::Bet(bettor, market_id);
        
        if env.storage().persistent().has(&key) {
            env.storage().persistent().extend_ttl(&key, THRESHOLD, EXTEND_TO);
            env.storage().persistent().get(&key)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test;
