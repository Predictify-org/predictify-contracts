#![no_std]
use soroban_sdk::{
    contract, contractimpl, Address, Env, Map, String, Symbol, Vec, 
    token, contracterror, panic_with_error, contracttype
};

#[contracterror]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Unauthorized = 1,
    MarketClosed = 2,
    OracleUnavailable = 3,
    InsufficientStake = 4,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OracleProvider {
    BandProtocol,
    DIA,
    Reflector,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleConfig {
    pub provider: OracleProvider,
    pub feed_id: String,       // Oracle-specific identifier
    pub threshold: i128,       // 10_000_00 = $10k (in cents)
    pub comparison: String,    // "gt", "lt", "eq"
}

#[contracttype]
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

    // Create a market (we need to add this function for the vote function to work with)
    pub fn create_market(
        env: Env,
        admin: Address,
        market_id: Symbol,
        question: String,
        outcomes: Vec<String>,
        end_time: u64,
        oracle_config: OracleConfig,
    ) {
        // Authenticate that the caller is the admin
        admin.require_auth();

        // Create a new market
        let market = Market {
            admin,
            question,
            outcomes,
            end_time,
            oracle_config,
            oracle_result: None,
            votes: Map::new(&env),
            total_staked: 0,
            dispute_stakes: Map::new(&env),
        };

        // Store the market
        env.storage().persistent().set(&market_id, &market);
    }

    // Allows users to vote on a market outcome by staking tokens
    pub fn vote(
        env: Env,
        user: Address,
        market_id: Symbol,
        outcome: String,
        stake: i128,
    ) {
        // Require authentication from the user
        user.require_auth();

        // Get the market from storage
        let mut market: Market = env.storage().persistent().get(&market_id).unwrap_or_else(|| {
            panic!("Market not found");
        });

        // Check if the market is still active
        if env.ledger().timestamp() >= market.end_time {
            panic_with_error!(env, Error::MarketClosed);
        }

        // Validate that the chosen outcome is valid
        let outcome_exists = market.outcomes.iter().any(|o| o == outcome);
        if !outcome_exists {
            panic!("Invalid outcome");
        }

        // Define the token contract to use for staking
        let token_id = env.storage().persistent().get::<Symbol, Address>(
            &Symbol::new(&env, "TokenID")
        ).unwrap_or_else(|| {
            panic!("Token contract not set");
        });

        // Create a client for the token contract
        let token_client = token::Client::new(&env, &token_id);

        // Transfer the staked amount from the user to this contract
        token_client.transfer(
            &user, 
            &env.current_contract_address(), 
            &stake
        );

        // Store the vote in the market
        market.votes.set(user.clone(), outcome);
        
        // Update the total staked amount
        market.total_staked += stake;

        // Update the market in storage
        env.storage().persistent().set(&market_id, &market);
    }
}
mod test;

