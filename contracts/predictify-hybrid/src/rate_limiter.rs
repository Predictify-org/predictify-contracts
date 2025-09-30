use soroban_sdk::{contract, contracterror, contractimpl, contracttype, Address, Env, Symbol};

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct RateLimitConfig {
    pub voting_limit: u32,        // Max votes per time window
    pub dispute_limit: u32,       // Max disputes per time window
    pub oracle_call_limit: u32,   // Max oracle calls per time window
    pub time_window_seconds: u64, // Time window in seconds
}

// Rate limit tracking
#[contracttype]
#[derive(Clone, Debug)]
pub struct RateLimit {
    pub count: u32,
    pub window_start: u64,
}

// Rate limiter state management
#[contracttype]
pub enum RateLimiterData {
    Config,
    UserVoting(Address, Symbol),   // user, market_id
    UserDisputes(Address, Symbol), // user, market_id
    OracleCalls(Symbol),           // market_id
}

pub struct RateLimiter {
    env: Env,
}

impl RateLimiter {
    pub fn new(env: Env) -> Self {
        RateLimiter { env }
    }

    // Initialize rate limiter with default configuration
    pub fn init_rate_limiter(
        &self,
        admin: Address,
        config: RateLimitConfig,
    ) -> Result<(), RateLimiterError> {
        admin.require_auth();
        self.validate_rate_limit_configuration(&config)?;
        self.env
            .storage()
            .persistent()
            .set(&RateLimiterData::Config, &config);

        Ok(())
    }

    // Get current configuration
    fn get_config(&self) -> Result<RateLimitConfig, RateLimiterError> {
        self.env
            .storage()
            .persistent()
            .get(&RateLimiterData::Config)
            .ok_or(RateLimiterError::ConfigNotFound)
    }

    // Check if rate limit is exceeded
    fn check_limit(&self, current_count: u32, limit: u32) -> Result<(), RateLimiterError> {
        if current_count >= limit {
            return Err(RateLimiterError::RateLimitExceeded);
        }
        Ok(())
    }

    // Get or create rate limit entry
    fn get_or_create_limit(&self, key: &RateLimiterData) -> RateLimit {
        self.env
            .storage()
            .temporary()
            .get(key)
            .unwrap_or(RateLimit {
                count: 0,
                window_start: self.env.ledger().timestamp(),
            })
    }

    // Update rate limit entry
    fn update_limit(
        &self,
        key: &RateLimiterData,
        mut limit: RateLimit,
        time_window: u64,
    ) -> Result<(), RateLimiterError> {
        let current_time = self.env.ledger().timestamp();

        if current_time >= limit.window_start + time_window {
            limit.count = 1;
            limit.window_start = current_time;
        } else {
            limit.count += 1;
        }

        self.env.storage().temporary().set(key, &limit);
        self.env.storage().temporary().extend_ttl(
            key,
            time_window as u32 + 86400,
            time_window as u32 + 86400,
        );

        Ok(())
    }

    // Rate limit voting operations
    pub fn rate_limit_voting(
        &self,
        user: Address,
        market_id: Symbol,
    ) -> Result<(), RateLimiterError> {
        user.require_auth();

        let config = self.get_config()?;
        let key = RateLimiterData::UserVoting(user.clone(), market_id.clone());
        let limit = self.get_or_create_limit(&key);

        self.check_limit(limit.count, config.voting_limit)?;
        self.update_limit(&key, limit, config.time_window_seconds)?;

        Ok(())
    }

    // Rate limit dispute operations
    pub fn rate_limit_disputes(
        &self,
        user: Address,
        market_id: Symbol,
    ) -> Result<(), RateLimiterError> {
        user.require_auth();

        let config = self.get_config()?;
        let key = RateLimiterData::UserDisputes(user.clone(), market_id.clone());
        let limit = self.get_or_create_limit(&key);

        self.check_limit(limit.count, config.dispute_limit)?;
        self.update_limit(&key, limit, config.time_window_seconds)?;

        Ok(())
    }

    // Rate limit oracle calls
    pub fn rate_limit_oracle_calls(&self, market_id: Symbol) -> Result<(), RateLimiterError> {
        let config = self.get_config()?;
        let key = RateLimiterData::OracleCalls(market_id.clone());
        let limit = self.get_or_create_limit(&key);

        self.check_limit(limit.count, config.oracle_call_limit)?;
        self.update_limit(&key, limit, config.time_window_seconds)?;

        Ok(())
    }

    // Update rate limits (admin only)
    pub fn update_rate_limits(
        &self,
        admin: Address,
        limits: RateLimitConfig,
    ) -> Result<(), RateLimiterError> {
        admin.require_auth();

        self.validate_rate_limit_configuration(&limits)?;

        self.env
            .storage()
            .persistent()
            .set(&RateLimiterData::Config, &limits);

        Ok(())
    }

    // Get rate limit status for a user
    pub fn get_rate_limit_status(
        &self,
        user: Address,
        market_id: Symbol,
    ) -> Result<RateLimitStatus, RateLimiterError> {
        let config = self.get_config()?;

        let voting_key = RateLimiterData::UserVoting(user.clone(), market_id.clone());
        let voting_limit = self.get_or_create_limit(&voting_key);

        let dispute_key = RateLimiterData::UserDisputes(user.clone(), market_id.clone());
        let dispute_limit = self.get_or_create_limit(&dispute_key);

        let current_time = self.env.ledger().timestamp();

        Ok(RateLimitStatus {
            voting_remaining: config.voting_limit.saturating_sub(voting_limit.count),
            dispute_remaining: config.dispute_limit.saturating_sub(dispute_limit.count),
            window_reset_time: voting_limit.window_start + config.time_window_seconds,
            current_time,
        })
    }

    // Validate rate limit configuration
    pub fn validate_rate_limit_configuration(
        &self,
        config: &RateLimitConfig,
    ) -> Result<(), RateLimiterError> {
        if config.voting_limit == 0 || config.voting_limit > 10000 {
            return Err(RateLimiterError::InvalidVotingLimit);
        }

        if config.dispute_limit == 0 || config.dispute_limit > 1000 {
            return Err(RateLimiterError::InvalidDisputeLimit);
        }

        if config.oracle_call_limit == 0 || config.oracle_call_limit > 1000 {
            return Err(RateLimiterError::InvalidOracleCallLimit);
        }

        // Time window should be between 1 minute and 30 days
        if config.time_window_seconds < 60 || config.time_window_seconds > 2592000 {
            return Err(RateLimiterError::InvalidTimeWindow);
        }

        Ok(())
    }
}

// Rate limit status response
#[contracttype]
#[derive(Clone, Debug)]
pub struct RateLimitStatus {
    pub voting_remaining: u32,
    pub dispute_remaining: u32,
    pub window_reset_time: u64,
    pub current_time: u64,
}

// Error types
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum RateLimiterError {
    ConfigNotFound = 1,
    RateLimitExceeded = 2,
    InvalidVotingLimit = 3,
    InvalidDisputeLimit = 4,
    InvalidOracleCallLimit = 5,
    InvalidTimeWindow = 6,
    Unauthorized = 7,
}

#[contract]
pub struct RateLimiterContract;

#[contractimpl]
impl RateLimiterContract {
    // Initialize the rate limiter
    pub fn init_rate_limiter(
        env: Env,
        admin: Address,
        config: RateLimitConfig,
    ) -> Result<(), RateLimiterError> {
        let limiter = RateLimiter::new(env);
        limiter.init_rate_limiter(admin, config)
    }

    // Check and enforce voting rate limit
    pub fn check_voting_rate_limit(
        env: Env,
        user: Address,
        market_id: Symbol,
    ) -> Result<(), RateLimiterError> {
        let limiter = RateLimiter::new(env);
        limiter.rate_limit_voting(user, market_id)
    }

    // Check and enforce dispute rate limit
    pub fn check_dispute_rate_limit(
        env: Env,
        user: Address,
        market_id: Symbol,
    ) -> Result<(), RateLimiterError> {
        let limiter = RateLimiter::new(env);
        limiter.rate_limit_disputes(user, market_id)
    }

    // Check and enforce oracle call rate limit
    pub fn check_oracle_rate_limit(env: Env, market_id: Symbol) -> Result<(), RateLimiterError> {
        let limiter = RateLimiter::new(env);
        limiter.rate_limit_oracle_calls(market_id)
    }

    // Update rate limits (admin only)
    pub fn update_rate_limits(
        env: Env,
        admin: Address,
        limits: RateLimitConfig,
    ) -> Result<(), RateLimiterError> {
        let limiter = RateLimiter::new(env);
        limiter.update_rate_limits(admin, limits)
    }

    // Get rate limit status for a user
    pub fn get_rate_limit_status(
        env: Env,
        user: Address,
        market_id: Symbol,
    ) -> Result<RateLimitStatus, RateLimiterError> {
        let limiter = RateLimiter::new(env);
        limiter.get_rate_limit_status(user, market_id)
    }

    // Validate rate limit configuration
    pub fn validate_rate_limit_config(
        env: Env,
        config: RateLimitConfig,
    ) -> Result<(), RateLimiterError> {
        let limiter = RateLimiter::new(env);
        limiter.validate_rate_limit_configuration(&config)
    }
}

/////////////////////////////////////////////////////////////
////                     TEST                        ///////
///////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{
        testutils::{Address as _, AuthorizedInvocation},
        Env,
    };

    fn create_test_config() -> RateLimitConfig {
        RateLimitConfig {
            voting_limit: 10,
            dispute_limit: 5,
            oracle_call_limit: 20,
            time_window_seconds: 3600, // 1 hour
        }
    }

    #[test]
    fn test_rate_limiting_scenarios() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let market_id = Symbol::new(&env, "market1");

        let config = create_test_config();

        // Deploy & init
        let contract_id = env.register_contract(None, RateLimiterContract);
        let client = RateLimiterContractClient::new(&env, &contract_id);
        client.init_rate_limiter(&admin, &config);

        // Test voting rate limit
        for i in 0..config.voting_limit {
            client.check_voting_rate_limit(&user, &market_id);
        }

        // Next vote should exceed limit
        let res = client.try_check_voting_rate_limit(&user, &market_id);
        assert_eq!(res, Err(Ok(RateLimiterError::RateLimitExceeded.into())));

        // Test dispute rate limit
        for _ in 0..config.dispute_limit {
            client.check_dispute_rate_limit(&user, &market_id);
        }

        // Next dispute should exceed limit
        let res = client.try_check_dispute_rate_limit(&user, &market_id);
        assert_eq!(res, Err(Ok(RateLimiterError::RateLimitExceeded.into())));

        // Test oracle call rate limit
        for _ in 0..config.oracle_call_limit {
            client.check_oracle_rate_limit(&market_id);
        }

        // Next oracle call should exceed limit
        let res = client.try_check_oracle_rate_limit(&market_id);
        assert_eq!(res, Err(Ok(RateLimiterError::RateLimitExceeded.into())));
    }

    #[test]
    fn test_rate_limit_status() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let market_id = Symbol::new(&env, "market1");

        let config = create_test_config();

        let contract_id = env.register_contract(None, RateLimiterContract);
        let client = RateLimiterContractClient::new(&env, &contract_id);

        // Init
        client.init_rate_limiter(&admin, &config);

        // Make some votes
        for _ in 0..3 {
            client.check_voting_rate_limit(&user, &market_id);
        }

        // Check status
        let status = client.get_rate_limit_status(&user, &market_id);

        assert_eq!(status.voting_remaining, config.voting_limit - 3);
        assert_eq!(status.dispute_remaining, config.dispute_limit);
    }

    #[test]
    fn test_validate_rate_limit_configuration() {
        let env = Env::default();
        env.mock_all_auths();

        // Valid configuration
        let valid_config = create_test_config();
        let result = RateLimiterContract::validate_rate_limit_config(env.clone(), valid_config);
        assert!(result.is_ok());

        // Invalid voting limit (too high)
        let invalid_config = RateLimitConfig {
            voting_limit: 20000,
            dispute_limit: 5,
            oracle_call_limit: 20,
            time_window_seconds: 3600,
        };
        let result = RateLimiterContract::validate_rate_limit_config(env.clone(), invalid_config);
        assert_eq!(result, Err(RateLimiterError::InvalidVotingLimit));

        // Invalid time window (too short)
        let invalid_config = RateLimitConfig {
            voting_limit: 10,
            dispute_limit: 5,
            oracle_call_limit: 20,
            time_window_seconds: 30, // Less than 60
        };
        let result = RateLimiterContract::validate_rate_limit_config(env.clone(), invalid_config);
        assert_eq!(result, Err(RateLimiterError::InvalidTimeWindow));
    }

    #[test]
    fn test_update_rate_limits() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);

        let initial_config = create_test_config();
        let contract_id = env.register_contract(None, RateLimiterContract);
        let client = RateLimiterContractClient::new(&env, &contract_id);

        // Init with initial config
        client.init_rate_limiter(&admin, &initial_config);

        // Update with new limits
        let new_config = RateLimitConfig {
            voting_limit: 20,
            dispute_limit: 10,
            oracle_call_limit: 30,
            time_window_seconds: 7200,
        };

        client.update_rate_limits(&admin, &new_config);
    }

    #[test]
    fn test_different_markets() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let market1 = Symbol::new(&env, "market1");
        let market2 = Symbol::new(&env, "market2");

        let config = create_test_config();
        let contract_id = env.register_contract(None, RateLimiterContract);
        let client = RateLimiterContractClient::new(&env, &contract_id);

        // Init with client
        client.init_rate_limiter(&admin, &config);

        // Use up limit on market1
        for _ in 0..config.voting_limit {
            client.check_voting_rate_limit(&user, &market1);
        }

        // Should still be able to vote on market2
        client.check_voting_rate_limit(&user, &market2);
    }
}
