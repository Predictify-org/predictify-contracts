use soroban_sdk::{contract, contracterror, contractimpl, contracttype, Address, Env, Symbol};

/// How the token bucket refills between requests.
///
/// ## Linear (default)
/// The bucket resets fully at the start of each new time window.  This is
/// the original behaviour and remains the default for backwards compatibility.
///
/// ## HalfLife
/// Tokens are replenished using an exponential (half-life) model:
///
/// ```
/// available = capacity - (remaining_used >> (elapsed / half_life_seconds))
/// ```
///
/// In plain terms: the number of *consumed* tokens halves every
/// `half_life_seconds` of idle time, causing the available count to
/// approach `capacity` asymptotically.  All arithmetic uses pure integer
/// math (no floats) via repeated right-shifts, which are equivalent to
/// division by powers of two.
///
/// ### Integer-safe formula
/// Let `used = capacity - available_at_last_check`.
/// After `elapsed` seconds the new used amount is:
///
/// ```
/// half_lives_elapsed = elapsed / half_life_seconds   (integer division)
/// new_used = used >> half_lives_elapsed               (saturates to 0)
/// new_available = capacity - new_used
/// ```
///
/// Because `new_used >= 0` and `new_used <= capacity`, there is **no
/// overflow** for any combination of inputs.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum RefillMode {
    /// Reset bucket fully at the start of every new time window (original behaviour).
    Linear,
    /// Exponential decay: consumed tokens halve every `half_life_seconds`.
    HalfLife(u64), // half_life_seconds stored as the inner value
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct RateLimitConfig {
    pub voting_limit: u32,           // Max votes per time window
    pub dispute_limit: u32,          // Max disputes per time window
    pub oracle_call_limit: u32,      // Max oracle calls per time window
    pub bet_limit: u32,              // Max bets per user per time window (0 = no limit)
    pub events_per_admin_limit: u32, // Max events per admin per time window (0 = no limit)
    pub time_window_seconds: u64,    // Time window in seconds
    /// Refill strategy.  Defaults to `RefillMode::Linear` for backwards
    /// compatibility with callers that do not set this field explicitly.
    pub refill_mode: RefillMode,
}

/// Token-bucket state kept in temporary storage per (key, user/market).
///
/// For **Linear** mode:
///   - `count` is the number of actions taken in the current window.
///   - `window_start` is the start of the current window.
///
/// For **HalfLife** mode:
///   - `count` is treated as the number of *consumed* tokens (i.e. `capacity - available`).
///   - `window_start` is the timestamp of the last time the bucket was updated,
///     used to compute how much decay to apply before the next check.
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
    UserBets(Address),             // user (global bet count per window)
    AdminEvents(Address),          // admin (events created per window)
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

    // Check if rate limit is exceeded, respecting the configured RefillMode.
    //
    // For Linear mode: exceeded when count >= limit (same as before).
    // For HalfLife mode: exceeded when the number of available tokens after
    //   decay is 0 (i.e. decayed_used >= capacity).
    fn check_limit_with_config(
        &self,
        limit_state: &RateLimit,
        capacity: u32,
        config: &RateLimitConfig,
    ) -> Result<(), RateLimiterError> {
        match &config.refill_mode {
            RefillMode::Linear => {
                // Original window-reset semantics.
                let current_time = self.env.ledger().timestamp();
                let active = current_time < limit_state.window_start.saturating_add(config.time_window_seconds);
                let effective_count = if active { limit_state.count } else { 0 };
                if effective_count >= capacity {
                    return Err(RateLimiterError::RateLimitExceeded);
                }
            }
            RefillMode::HalfLife(half_life_secs) => {
                let now = self.env.ledger().timestamp();
                let available = Self::halflife_available(limit_state, capacity, *half_life_secs, now);
                if available == 0 {
                    return Err(RateLimiterError::RateLimitExceeded);
                }
            }
        }
        Ok(())
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
        config: &RateLimitConfig,
        capacity: u32,
    ) -> Result<(), RateLimiterError> {
        let current_time = self.env.ledger().timestamp();
        let time_window = config.time_window_seconds;

        match &config.refill_mode {
            RefillMode::Linear => {
                // Original behaviour: reset the bucket at the start of a new window.
                if current_time >= limit.window_start.saturating_add(time_window) {
                    limit.count = 1;
                    limit.window_start = current_time;
                } else {
                    limit.count = limit.count.saturating_add(1);
                }
            }
            RefillMode::HalfLife(half_life_secs) => {
                // Half-life (exponential decay) refill.
                //
                // `limit.count` stores consumed tokens (= capacity - available).
                // Each `half_life_secs` of elapsed time halves the consumed amount,
                // so the bucket asymptotically approaches full capacity.
                //
                // elapsed may be 0 if called multiple times in the same ledger
                // second, which is fine: no decay happens, consumed stays the same.
                let half_life = *half_life_secs;
                if half_life == 0 {
                    // Safety: treat zero half-life as instant full refill (linear reset).
                    limit.count = 1;
                    limit.window_start = current_time;
                } else {
                    let elapsed = current_time.saturating_sub(limit.window_start);
                    // Number of full half-lives elapsed (integer division).
                    let half_lives = elapsed / half_life;

                    // Decay consumed tokens: right-shift by half_lives (capped at 31
                    // to avoid shifting a u32 by >= 32, which would be UB in Rust).
                    let decayed_used = if half_lives >= 32 {
                        0u32
                    } else {
                        limit.count >> half_lives
                    };

                    // Now charge one more token. Use saturating_add to avoid wrap.
                    // If decayed_used >= capacity, this means we're already over —
                    // the check_limit call before us would have blocked it; but we
                    // use saturating_add defensively anyway.
                    let new_used = decayed_used.saturating_add(1).min(capacity);

                    limit.count = new_used;
                    limit.window_start = current_time;
                }
            }
        }

        self.env.storage().temporary().set(key, &limit);
        self.env.storage().temporary().extend_ttl(
            key,
            time_window as u32 + 86400,
            time_window as u32 + 86400,
        );

        Ok(())
    }

    /// Compute the number of *available* tokens for a bucket in HalfLife mode,
    /// given the stored state and current timestamp.
    ///
    /// Returns `capacity` when there is no stored state (first use).
    /// In Linear mode this is unused; callers use the window-reset logic.
    ///
    /// Pure function — does not touch storage.
    fn halflife_available(limit: &RateLimit, capacity: u32, half_life_secs: u64, now: u64) -> u32 {
        if half_life_secs == 0 {
            return capacity;
        }
        let elapsed = now.saturating_sub(limit.window_start);
        let half_lives = elapsed / half_life_secs;
        let decayed_used = if half_lives >= 32 {
            0u32
        } else {
            limit.count >> half_lives
        };
        capacity.saturating_sub(decayed_used)
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

        self.check_limit_with_config(&limit, config.voting_limit, &config)?;
        self.update_limit(&key, limit, &config, config.voting_limit)?;

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

        self.check_limit_with_config(&limit, config.dispute_limit, &config)?;
        self.update_limit(&key, limit, &config, config.dispute_limit)?;

        Ok(())
    }

    // Rate limit oracle calls
    pub fn rate_limit_oracle_calls(&self, market_id: Symbol) -> Result<(), RateLimiterError> {
        let config = self.get_config()?;
        let key = RateLimiterData::OracleCalls(market_id.clone());
        let limit = self.get_or_create_limit(&key);

        self.check_limit_with_config(&limit, config.oracle_call_limit, &config)?;
        self.update_limit(&key, limit, &config, config.oracle_call_limit)?;

        Ok(())
    }

    /// Rate limit bets: max bets per user per time window (global across markets).
    /// Returns Ok(()) if within limit or config not set; ConfigNotFound is used by caller to skip check.
    /// Caller (e.g. place_bet) must have already authenticated user.
    pub fn rate_limit_bets(&self, user: Address) -> Result<(), RateLimiterError> {
        let config = self.get_config()?;
        if config.bet_limit == 0 {
            return Ok(());
        }
        let key = RateLimiterData::UserBets(user.clone());
        let limit = self.get_or_create_limit(&key);
        self.check_limit_with_config(&limit, config.bet_limit, &config)?;
        self.update_limit(&key, limit, &config, config.bet_limit)?;
        Ok(())
    }

    /// Rate limit event creation: max events per admin per time window.
    /// Caller (e.g. create_market) must have already authenticated admin.
    pub fn rate_limit_admin_events(&self, admin: Address) -> Result<(), RateLimiterError> {
        let config = self.get_config()?;
        if config.events_per_admin_limit == 0 {
            return Ok(());
        }
        let key = RateLimiterData::AdminEvents(admin.clone());
        let limit = self.get_or_create_limit(&key);
        self.check_limit_with_config(&limit, config.events_per_admin_limit, &config)?;
        self.update_limit(&key, limit, &config, config.events_per_admin_limit)?;
        Ok(())
    }

    // Update rate limits (admin only). Caller must have already authenticated admin.
    pub fn update_rate_limits(
        &self,
        _admin: Address,
        limits: RateLimitConfig,
    ) -> Result<(), RateLimiterError> {
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

        let (voting_remaining, dispute_remaining) = match &config.refill_mode {
            RefillMode::Linear => {
                let v_active = current_time < voting_limit.window_start.saturating_add(config.time_window_seconds);
                let d_active = current_time < dispute_limit.window_start.saturating_add(config.time_window_seconds);
                let v_count = if v_active { voting_limit.count } else { 0 };
                let d_count = if d_active { dispute_limit.count } else { 0 };
                (
                    config.voting_limit.saturating_sub(v_count),
                    config.dispute_limit.saturating_sub(d_count),
                )
            }
            RefillMode::HalfLife(half_life_secs) => {
                let v_avail = Self::halflife_available(&voting_limit, config.voting_limit, *half_life_secs, current_time);
                let d_avail = Self::halflife_available(&dispute_limit, config.dispute_limit, *half_life_secs, current_time);
                (v_avail, d_avail)
            }
        };

        Ok(RateLimitStatus {
            voting_remaining,
            dispute_remaining,
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

        if config.bet_limit > 10000 {
            return Err(RateLimiterError::InvalidBetLimit);
        }
        if config.events_per_admin_limit > 1000 {
            return Err(RateLimiterError::InvalidEventsLimit);
        }

        // Time window should be between 1 minute and 30 days
        if config.time_window_seconds < 60 || config.time_window_seconds > 2592000 {
            return Err(RateLimiterError::InvalidTimeWindow);
        }

        // Validate HalfLife parameter: half_life_seconds must be > 0 and fit within the time window.
        if let RefillMode::HalfLife(half_life_secs) = config.refill_mode {
            if half_life_secs == 0 {
                return Err(RateLimiterError::InvalidHalfLife);
            }
            // half_life_seconds must not exceed the time window (it would be meaningless).
            if half_life_secs > config.time_window_seconds {
                return Err(RateLimiterError::InvalidHalfLife);
            }
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
    InvalidBetLimit = 8,
    InvalidEventsLimit = 9,
    /// `half_life_seconds` is zero or exceeds the configured `time_window_seconds`.
    InvalidHalfLife = 10,
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

    // Check and enforce admin event creation rate limit
    pub fn check_admin_event_rate_limit(env: Env, admin: Address) -> Result<(), RateLimiterError> {
        let limiter = RateLimiter::new(env);
        limiter.rate_limit_admin_events(admin)
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
            bet_limit: 50,
            events_per_admin_limit: 10,
            time_window_seconds: 3600, // 1 hour
            refill_mode: RefillMode::Linear,
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
            bet_limit: 0,
            events_per_admin_limit: 0,
            time_window_seconds: 3600,
            refill_mode: RefillMode::Linear,
        };
        let result = RateLimiterContract::validate_rate_limit_config(env.clone(), invalid_config);
        assert_eq!(result, Err(RateLimiterError::InvalidVotingLimit));

        // Invalid time window (too short)
        let invalid_config = RateLimitConfig {
            voting_limit: 10,
            dispute_limit: 5,
            oracle_call_limit: 20,
            bet_limit: 0,
            events_per_admin_limit: 0,
            time_window_seconds: 30, // Less than 60
            refill_mode: RefillMode::Linear,
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
            bet_limit: 100,
            events_per_admin_limit: 20,
            time_window_seconds: 7200,
            refill_mode: RefillMode::Linear,
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
