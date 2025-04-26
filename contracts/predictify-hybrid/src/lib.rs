#![no_std]
use soroban_sdk::{
    contract, contractimpl, Address, Env, Map, String, Symbol, Vec, 
    token, contracterror, panic_with_error, contracttype
};

#[contracterror]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Error {
    Unauthorized = 1,
    MarketClosed = 2,
    OracleUnavailable = 3,
    InsufficientStake = 4,
    MarketAlreadyResolved = 5,
    InvalidOracleConfig = 6,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OracleProvider {
    BandProtocol,
    DIA,
    Reflector,
    Pyth,
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

// Placeholder for Pyth oracle interface
#[contracttype]
pub struct PythPrice {
    pub price: i128,
    pub conf: u64,
    pub expo: i32,
    pub publish_time: u64,
}

trait OracleInterface {
    fn get_price(&self, env: &Env, feed_id: &String) -> Result<i128, Error>;
}

struct PythOracle {
    contract_id: Address,
}

impl OracleInterface for PythOracle {
    fn get_price(&self, _env: &Env, _feed_id: &String) -> Result<i128, Error> {
        // This is a placeholder for the actual Pyth oracle interaction
        // In a real implementation, we would call the Pyth contract here
        // For now, we're returning a mock price
        
        // Simulate a call to the Pyth oracle
        // In a real implementation, we would call something like:
        // let price = pyth_client.get_price(&feed_id.to_string());
        
        // Return a simulated price (e.g., $26,000 for BTC/USD)
        Ok(26_000_00)
    }
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

    // Fetch oracle result to determine market outcome
    pub fn fetch_oracle_result(
        env: Env,
        market_id: Symbol,
        pyth_contract: Address,
    ) -> String {
        // Get the market from storage
        let mut market: Market = env.storage().persistent().get(&market_id).unwrap_or_else(|| {
            panic!("Market not found");
        });

        // Check if the market has already been resolved
        if market.oracle_result.is_some() {
            panic_with_error!(env, Error::MarketAlreadyResolved);
        }

        // Check if the market ended (we can only fetch oracle result after market ends)
        let current_time = env.ledger().timestamp();
        if current_time < market.end_time {
            panic_with_error!(env, Error::MarketClosed);
        }

        // Validate the oracle config
        if market.oracle_config.provider != OracleProvider::Pyth {
            panic_with_error!(env, Error::InvalidOracleConfig);
        }

        // Get the price from the oracle
        let oracle = PythOracle { contract_id: pyth_contract };
        let price = match oracle.get_price(&env, &market.oracle_config.feed_id) {
            Ok(p) => p,
            Err(e) => panic_with_error!(env, e),
        };

        // Determine the outcome based on the price and threshold
        let outcome = if market.oracle_config.comparison == String::from_str(&env, "gt") {
            if price > market.oracle_config.threshold {
                String::from_str(&env, "yes")
            } else {
                String::from_str(&env, "no")
            }
        } else if market.oracle_config.comparison == String::from_str(&env, "lt") {
            if price < market.oracle_config.threshold {
                String::from_str(&env, "yes")
            } else {
                String::from_str(&env, "no")
            }
        } else if market.oracle_config.comparison == String::from_str(&env, "eq") {
            if price == market.oracle_config.threshold {
                String::from_str(&env, "yes")
            } else {
                String::from_str(&env, "no")
            }
        } else {
            panic_with_error!(env, Error::InvalidOracleConfig);
        };

        // Store the result in the market
        market.oracle_result = Some(outcome.clone());

        // Update the market in storage
        env.storage().persistent().set(&market_id, &market);

        // Return the outcome
        outcome
    }

    // Allows users to dispute the market result by staking tokens
    pub fn dispute_result(
        env: Env,
        user: Address,
        market_id: Symbol,
        stake: i128,
    ) {
        // Require authentication from the user
        user.require_auth();

        // Get the market from storage
        let mut market: Market = env.storage().persistent().get(&market_id).unwrap_or_else(|| {
            panic!("Market not found");
        });

        // Ensure disputes are only possible after the market ends
        let current_time = env.ledger().timestamp();
        if current_time < market.end_time {
            panic!("Cannot dispute before market ends");
        }

        // Require a minimum stake (10 XLM) to raise a dispute
        let min_stake: i128 = 10_0000000; // 10 XLM (in stroops, 1 XLM = 10^7 stroops)
        if stake < min_stake {
            panic_with_error!(env, Error::InsufficientStake);
        }

        // Define the token contract to use for staking
        let token_id = env.storage().persistent().get::<Symbol, Address>(
            &Symbol::new(&env, "TokenID")
        ).unwrap_or_else(|| {
            panic!("Token contract not set");
        });

        // Create a client for the token contract
        let token_client = token::Client::new(&env, &token_id);

        // Transfer the stake from the user to the contract
        token_client.transfer(
            &user, 
            &env.current_contract_address(), 
            &stake
        );

        // Store the dispute stake in the market
        if let Some(existing_stake) = market.dispute_stakes.get(user.clone()) {
            market.dispute_stakes.set(user.clone(), existing_stake + stake);
        } else {
            market.dispute_stakes.set(user.clone(), stake);
        }

        // Extend the market end time by 24 hours during a dispute (if not already extended)
        let dispute_extension = 24 * 60 * 60; // 24 hours in seconds
        if market.end_time < current_time + dispute_extension {
            market.end_time = current_time + dispute_extension;
        }

        // Update the market in storage
        env.storage().persistent().set(&market_id, &market);
    }


    // Resolves a market by combining oracle results and community votes
    pub fn resolve_market(
        env: Env,
        market_id: Symbol,
    ) -> String {
        // Get the market from storage
        let mut market: Market = env.storage().persistent().get(&market_id).unwrap_or_else(|| {
            panic!("Market not found");
        });

        // Check if the market end time has passed
        let current_time = env.ledger().timestamp();
        if current_time < market.end_time {
            panic_with_error!(env, Error::MarketClosed);
        }

        // Retrieve the oracle result (or fail if unavailable)
        let oracle_result = match &market.oracle_result {
            Some(result) => result.clone(),
            None => panic_with_error!(env, Error::OracleUnavailable),
        };

        // Count community votes for each outcome
        let mut vote_counts: Map<String, u32> = Map::new(&env);
        for (_, outcome) in market.votes.iter() {
            let count = vote_counts.get(outcome.clone()).unwrap_or(0);
            vote_counts.set(outcome.clone(), count + 1);
        }

        // Find the community consensus (outcome with most votes)
        let mut community_result = oracle_result.clone(); // Default to oracle result if no votes
        let mut max_votes = 0;
        
        for (outcome, count) in vote_counts.iter() {
            if count > max_votes {
                max_votes = count;
                community_result = outcome.clone();
            }
        }

        // Calculate the final result with weights: 70% oracle, 30% community
        let final_result = if oracle_result == community_result {
            // If both agree, use that outcome
            oracle_result
        } else {
            // If they disagree, check if community votes are significant
            let total_votes: u32 = vote_counts.values().into_iter().fold(0, |acc, count| acc + count);
            
            if total_votes == 0 {
                // No community votes, use oracle result
                oracle_result
            } else {
                // Use integer-based calculation to determine if community consensus is strong
                // Check if the winning vote has more than 50% of total votes
                if max_votes * 100 > total_votes * 50 && total_votes >= 5 {
                    // Apply 70-30 weighting using integer arithmetic
                    // We'll use a scale of 0-100 for percentage calculation
                    
                    // Generate a pseudo-random number by combining timestamp and ledger sequence
                    let timestamp = env.ledger().timestamp();
                    let sequence = env.ledger().sequence();
                    let combined = timestamp as u128 + sequence as u128;
                    let random_value = (combined % 100) as u32;
                    
                    // If random_value is less than 30 (representing 30% weight), 
                    // choose community result
                    if random_value < 30 {
                        community_result
                    } else {
                        oracle_result
                    }
                } else {
                    // Not enough community consensus, use oracle result
                    oracle_result
                }
            }
        };

        // Record the final result in the market
        market.oracle_result = Some(final_result.clone());
        
        // Update the market in storage
        env.storage().persistent().set(&market_id, &market);

        // Return the final result
        final_result
    }
}
mod test;
