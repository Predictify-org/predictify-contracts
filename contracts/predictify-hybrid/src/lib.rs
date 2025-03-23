#![no_std]
use soroban_sdk::{
    contract, contractimpl, Address, Env, Map, String, Symbol, Vec
};
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Unauthorized = 1,
    MarketClosed = 2,
    OracleUnavailable = 3,
    InsufficientStake = 4,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OracleProvider {
    BandProtocol,
    DIA,
    Reflector,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleConfig {
    pub provider: OracleProvider,
    pub feed_id: String,       // Oracle-specific identifier
    pub threshold: i128,       // 10_000_00 = $10k (in cents)
    pub comparison: String,    // "gt", "lt", "eq"
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Market {
    pub admin: Address,
    pub question: String,
    pub outcomes: Vec<String>,
    pub end_time: u64,
    pub oracle_config: OracleConfig,
    pub oracle_result: Option<String>,
    pub votes: Map<Address, String>,
    pub total_staked: i128,
    pub dispute_stakes: Map<Address, i128>,
}

#[contract]
pub struct PredictifyHybrid;

#[contractimpl]
impl PredictifyHybrid {
    pub fn initialize(env: Env, admin: Address) {
        env.storage().persistent().set(&Symbol::new(&env, "Admin"), &admin);
    }
}
mod test;

