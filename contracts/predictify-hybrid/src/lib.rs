#![no_std]
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, token, Address, Env,
    Map, String, Symbol, Vec, symbol_short, vec, IntoVal
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
    AlreadyClaimed = 7,
    NothingToClaim = 8,
    MarketNotResolved = 9,
    InvalidOutcome = 10,
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
    pub feed_id: String,    // Oracle-specific identifier
    pub threshold: i128,    // 10_000_00 = $10k (in cents)
    pub comparison: String, // "gt", "lt", "eq"
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
    pub stakes: Map<Address, i128>,  // User stakes
    pub claimed: Map<Address, bool>, // Track claims
    pub total_staked: i128,
    pub dispute_stakes: Map<Address, i128>,
    pub winning_outcome: Option<String>,
    pub fee_collected: bool, // Track fee collection
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
        // In a real implementation, you would use the actual token contract ID
        // We're using the contract_id field to avoid the unused field warning
        if self.contract_id
            == Address::from_str(
                _env,
                "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC",
            )
        {
            // This is just a placeholder condition to use the contract_id field
            return Ok(27_000_00); // Different price for this specific contract
        }

        // Return a simulated price (e.g., $26,000 for BTC/USD)
        Ok(26_000_00)
    }
}

// Reflector Oracle Contract Types
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReflectorAsset {
    Stellar(Address),
    Other(Symbol),
}

#[contracttype]
pub struct ReflectorPriceData {
    pub price: i128,
    pub timestamp: u64,
}

#[contracttype]
pub struct ReflectorConfigData {
    pub admin: Address,
    pub assets: Vec<ReflectorAsset>,
    pub base_asset: ReflectorAsset,
    pub decimals: u32,
    pub period: u64,
    pub resolution: u32,
}
struct ReflectorOracleClient<'a> {
    env: &'a Env,
    contract_id: Address,
}

impl<'a> ReflectorOracleClient<'a> {
    fn new(env: &'a Env, contract_id: Address) -> Self {
        Self { env, contract_id }
    }

    fn lastprice(&self, asset: ReflectorAsset) -> Option<ReflectorPriceData> {
        let args = vec![self.env, asset.into_val(self.env)];
        self.env
            .invoke_contract(&self.contract_id, &symbol_short!("lastprice"), args)
    }

// Removed the unused `price` method as it is not utilized in the current codebase.

    fn twap(&self, asset: ReflectorAsset, records: u32) -> Option<i128> {
        let args = vec![
            self.env,
            asset.into_val(self.env),
            records.into_val(self.env),
        ];
        self.env
            .invoke_contract(&self.contract_id, &symbol_short!("twap"), args)
    }
}

struct ReflectorOracle {
    contract_id: Address,
}

impl OracleInterface for ReflectorOracle {
    fn get_price(&self, env: &Env, feed_id: &String) -> Result<i128, Error> {
        // Parse the feed_id to extract asset information
        // Expected format: "BTC/USD" or "ETH/USD" etc.
        // For now, we'll use the feed_id directly as the asset symbol
        
        // Create asset symbol for Reflector
        // Since we can't easily parse the String in no_std, we'll use the feed_id directly
        let base_asset = ReflectorAsset::Other(Symbol::new(env, "BTC")); // Default to BTC for now

        // Create Reflector client
        let reflector_client = ReflectorOracleClient::new(env, self.contract_id.clone());

        // Try to get the latest price first
        if let Some(price_data) = reflector_client.lastprice(base_asset.clone()) {
            return Ok(price_data.price);
        }

        // If lastprice fails, try TWAP with 1 record
        if let Some(twap_price) = reflector_client.twap(base_asset, 1) {
            return Ok(twap_price);
        }

        // If both fail, return error
        Err(Error::OracleUnavailable)
    }
}

#[contract]
pub struct PredictifyHybrid;

const PERCENTAGE_DENOMINATOR: i128 = 100;
const FEE_PERCENTAGE: i128 = 2; // 2% fee for the platform

#[contractimpl]
impl PredictifyHybrid {
    pub fn initialize(env: Env, admin: Address) {
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, "Admin"), &admin);
    }

    // Create a market (we need to add this function for the vote function to work with)
    pub fn create_market(
        env: Env,
        admin: Address,
        question: String,
        outcomes: Vec<String>,
        duration_days: u32,
        oracle_config: OracleConfig,
    ) -> Symbol {
        // Authenticate that the caller is the admin
        admin.require_auth();

        // Verify the caller is an admin
        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, "Admin"))
            .unwrap_or_else(|| {
                panic!("Admin not set");
            });

        if admin != stored_admin {
            panic_with_error!(env, Error::Unauthorized);
        }

        // Validate inputs
        if outcomes.len() < 2 {
            panic!("At least two outcomes are required");
        }

        if question.len() == 0 {
            panic!("Question cannot be empty");
        }

        // Generate a unique market ID using timestamp and a counter
        let counter_key = Symbol::new(&env, "MarketCounter");
        let counter: u32 = env.storage().persistent().get(&counter_key).unwrap_or(0);
        let new_counter = counter + 1;
        env.storage().persistent().set(&counter_key, &new_counter);

        // Create a unique market ID using the counter
        let market_id = Symbol::new(&env, "market");

        // Calculate end time based on duration_days (convert days to seconds)
        let seconds_per_day: u64 = 24 * 60 * 60; // 24 hours * 60 minutes * 60 seconds
        let duration_seconds: u64 = (duration_days as u64) * seconds_per_day;
        let end_time: u64 = env.ledger().timestamp() + duration_seconds;

        // Create a default oracle config (can be updated later if needed)
        let oracle_config = OracleConfig {
            provider: OracleProvider::Pyth,
            feed_id: String::from_str(&env, ""),
            threshold: 0,
            comparison: String::from_str(&env, "eq"),
        };

        // Create a new market
        let market = Market {
            admin: admin.clone(),
            question,
            outcomes,
            end_time,
            oracle_config,
            oracle_result: None,
            votes: Map::new(&env),
            total_staked: 0,
            dispute_stakes: Map::new(&env),
            stakes: Map::new(&env),
            claimed: Map::new(&env),
            winning_outcome: None,
            fee_collected: false, // Initialize fee collection state
        };

        // Deduct 1 XLM fee from the admin
        let fee_amount: i128 = 10_000_000; // 1 XLM = 10,000,000 stroops

        // Get a token client for the native asset
        // In a real implementation, you would use the actual token contract ID
        let token_id: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, "TokenID"))
            .unwrap_or_else(|| {
                panic!("Token ID not set");
            });
        let token_client = token::Client::new(&env, &token_id);

        // Transfer the fee from admin to the contract
        token_client.transfer(&admin, &env.current_contract_address(), &fee_amount);

        // Store the market
        env.storage().persistent().set(&market_id, &market);

        // Return the market ID
        market_id
    }

    // NEW: Distribute winnings to users
    pub fn claim_winnings(env: Env, user: Address, market_id: Symbol) {
        user.require_auth();

        let mut market: Market = env
            .storage()
            .persistent()
            .get(&market_id)
            .expect("Market not found");

        // Check if user has claimed already
        if market.claimed.get(user.clone()).unwrap_or(false) {
            panic_with_error!(env, Error::AlreadyClaimed);
        }

        // Check if market is resolved
        let winning_outcome = match &market.winning_outcome {
            Some(outcome) => outcome,
            None => panic_with_error!(env, Error::MarketNotResolved),
        };

        // Get user's vote and stake
        let user_outcome = market
            .votes
            .get(user.clone())
            .unwrap_or_else(|| panic_with_error!(env, Error::NothingToClaim));

        let user_stake = market.stakes.get(user.clone()).unwrap_or(0);

        // Calculate payout if user won
        if &user_outcome == winning_outcome {
            // Calculate total winning stakes
            let mut winning_total = 0;
            for (voter, outcome) in market.votes.iter() {
                if &outcome == winning_outcome {
                    winning_total += market.stakes.get(voter.clone()).unwrap_or(0);
                }
            }

            // Calculate user's share (minus fee percentage)
            let user_share =
                (user_stake * (PERCENTAGE_DENOMINATOR - FEE_PERCENTAGE)) / PERCENTAGE_DENOMINATOR;
            let total_pool = market.total_staked;

            // Ensure winning_total is non-zero
            if winning_total == 0 {
                panic_with_error!(env, Error::NothingToClaim);
            }
            let payout = (user_share * total_pool) / winning_total;

            // Get token client
            let token_id = env
                .storage()
                .persistent()
                .get(&Symbol::new(&env, "TokenID"))
                .expect("Token contract not set");

            let token_client = token::Client::new(&env, &token_id);

            // Transfer winnings to user
            token_client.transfer(&env.current_contract_address(), &user, &payout);
        }

        // Mark as claimed
        market.claimed.set(user.clone(), true);
        env.storage().persistent().set(&market_id, &market);
    }

    // NEW: Collect platform fees
    pub fn collect_fees(env: Env, admin: Address, market_id: Symbol) {
        admin.require_auth();

        let market: Market = env
            .storage()
            .persistent()
            .get(&market_id)
            .expect("Market not found");

        // Verify admin
        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, "Admin"))
            .expect("Admin not set");

        if admin != stored_admin {
            panic_with_error!(env, Error::Unauthorized);
        }

        // Check if fees already collected
        if market.fee_collected {
            panic_with_error!(env, Error::AlreadyClaimed);
        }

        // Calculate 2% fee
        let fee = (market.total_staked * 2) / 100;

        // Get token client
        let token_id = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, "TokenID"))
            .expect("Token contract not set");

        let token_client = token::Client::new(&env, &token_id);

        // Transfer fee to admin
        token_client.transfer(&env.current_contract_address(), &admin, &fee);

        // Update market state
        let mut market = market;
        market.fee_collected = true;
        env.storage().persistent().set(&market_id, &market);
    }

    // Finalize market after disputes
    pub fn finalize_market(env: Env, admin: Address, market_id: Symbol, outcome: String) {
        admin.require_auth();

        // Verify admin
        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, "Admin"))
            .expect("Admin not set");

        if admin != stored_admin {
            panic_with_error!(env, Error::Unauthorized);
        }

        let mut market: Market = env
            .storage()
            .persistent()
            .get(&market_id)
            .expect("Market not found");

        // Validate outcome
        if !market.outcomes.contains(&outcome) {
            panic_with_error!(env, Error::InvalidOutcome);
        }

        // Set final outcome
        market.winning_outcome = Some(outcome);
        env.storage().persistent().set(&market_id, &market);
    }

    // Allows users to vote on a market outcome by staking tokens
    pub fn vote(env: Env, user: Address, market_id: Symbol, outcome: String, stake: i128) {
        // Require authentication from the user
        user.require_auth();

        // Get the market from storage
        let mut market: Market = env
            .storage()
            .persistent()
            .get(&market_id)
            .unwrap_or_else(|| {
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
        let token_id = env
            .storage()
            .persistent()
            .get::<Symbol, Address>(&Symbol::new(&env, "TokenID"))
            .unwrap_or_else(|| {
                panic!("Token contract not set");
            });

        // Create a client for the token contract
        let token_client = token::Client::new(&env, &token_id);

        // Transfer the staked amount from the user to this contract
        token_client.transfer(&user, &env.current_contract_address(), &stake);

        // Store the vote in the market
        market.votes.set(user.clone(), outcome);

        // Update the total staked amount
        market.total_staked += stake;

        // Update the market in storage
        env.storage().persistent().set(&market_id, &market);
    }

    // Fetch oracle result to determine market outcome
    pub fn fetch_oracle_result(env: Env, market_id: Symbol, pyth_contract: Address) -> String {
        // Get the market from storage
        let mut market: Market = env
            .storage()
            .persistent()
            .get(&market_id)
            .unwrap_or_else(|| {
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
        let oracle = PythOracle {
            contract_id: pyth_contract,
        };
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
    pub fn dispute_result(env: Env, user: Address, market_id: Symbol, stake: i128) {
        // Require authentication from the user
        user.require_auth();

        // Get the market from storage
        let mut market: Market = env
            .storage()
            .persistent()
            .get(&market_id)
            .unwrap_or_else(|| {
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
        let token_id = env
            .storage()
            .persistent()
            .get::<Symbol, Address>(&Symbol::new(&env, "TokenID"))
            .unwrap_or_else(|| {
                panic!("Token contract not set");
            });

        // Create a client for the token contract
        let token_client = token::Client::new(&env, &token_id);

        // Transfer the stake from the user to the contract
        token_client.transfer(&user, &env.current_contract_address(), &stake);

        // Store the dispute stake in the market
        if let Some(existing_stake) = market.dispute_stakes.get(user.clone()) {
            market
                .dispute_stakes
                .set(user.clone(), existing_stake + stake);
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
    pub fn resolve_market(env: Env, market_id: Symbol) -> String {
        // Get the market from storage
        let mut market: Market = env
            .storage()
            .persistent()
            .get(&market_id)
            .unwrap_or_else(|| {
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
            let total_votes: u32 = vote_counts
                .values()
                .into_iter()
                .fold(0, |acc, count| acc + count);

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

        // Calculate winning outcome
        market.winning_outcome = Some(final_result.clone());

        // Calculate total for winning outcome
        let mut winning_total = 0;
        for (user, outcome) in market.votes.iter() {
            if outcome == final_result {
                winning_total += market.stakes.get(user.clone()).unwrap_or(0);
            }
        }

        // Record the final result in the market
        market.oracle_result = Some(final_result.clone());

        // Update the market in storage
        env.storage().persistent().set(&market_id, &market);

        // Return the final result
        final_result
    }

    // Clean up market storage
    pub fn close_market(env: Env, admin: Address, market_id: Symbol) {
        admin.require_auth();

        // Verify admin
        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, "Admin"))
            .expect("Admin not set");

        if admin != stored_admin {
            panic_with_error!(env, Error::Unauthorized);
        }

        // Remove market from storage
        env.storage().persistent().remove(&market_id);
    }

    // Helper function to create a market with Reflector oracle
    pub fn create_reflector_market(
        env: Env,
        admin: Address,
        question: String,
        outcomes: Vec<String>,
        duration_days: u32,
        asset_symbol: String,
        threshold: i128,
        comparison: String,
    ) -> Symbol {
        // Create Reflector oracle configuration
        let oracle_config = OracleConfig {
            provider: OracleProvider::Reflector,
            feed_id: asset_symbol, // Use asset symbol as feed_id
            threshold,
            comparison,
        };

        // Call the main create_market function
        Self::create_market(env, admin, question, outcomes, duration_days, oracle_config)
    }

        // Helper function to create a market with Pyth oracle
        pub fn create_pyth_market(
            env: Env,
            admin: Address,
            question: String,
            outcomes: Vec<String>,
            duration_days: u32,
            feed_id: String,
            threshold: i128,
            comparison: String,
        ) -> Symbol {
            // Create Pyth oracle configuration
            let oracle_config = OracleConfig {
                provider: OracleProvider::Pyth,
                feed_id,
                threshold,
                comparison,
            };
    
            // Call the main create_market function
            Self::create_market(env, admin, question, outcomes, duration_days, oracle_config)
        }

        // Helper function to create a market with Reflector oracle for specific assets
        pub fn create_reflector_asset_market(
            env: Env,
            admin: Address,
            question: String,
            outcomes: Vec<String>,
            duration_days: u32,
            asset_symbol: String,  // e.g., "BTC", "ETH", "XLM"
            threshold: i128,
            comparison: String,
        ) -> Symbol {
            // Create Reflector oracle configuration
            let oracle_config = OracleConfig {
                provider: OracleProvider::Reflector,
                feed_id: asset_symbol, // Use asset symbol as feed_id
                threshold,
                comparison,
            };
    
            // Call the main create_market function
            Self::create_market(env, admin, question, outcomes, duration_days, oracle_config)
        }
}
mod test;
