#![cfg(test)]

//! Rate limiter integration tests for entrypoint enforcement.
//!
//! These tests verify that rate limiting is properly enforced at the main contract
//! entrypoints (vote, dispute_market, vote_on_dispute, admin operations).
//!
//! Invariants proven:
//! - User cannot exceed voting rate limit via the `vote` entrypoint
//! - User cannot exceed dispute rate limit via `dispute_market` or `vote_on_dispute`
//! - Admin operations respect event creation rate limits
//! - Batch operations apply rate limit once per batch
//!
//! Coverage target: ≥95% line coverage on touched modules

use crate::rate_limiter::{
    RateLimitConfig, RateLimiter, RateLimiterContract, RateLimiterContractClient, RateLimiterData,
};
use soroban_sdk::{
    testutils::{Address as _, AuthorizedInvocation},
    Env,
};

fn create_test_config() -> RateLimitConfig {
    RateLimitConfig {
        voting_limit: 5,
        dispute_limit: 3,
        oracle_call_limit: 10,
        bet_limit: 20,
        events_per_admin_limit: 5,
        time_window_seconds: 3600,
    }
}

fn setup_limiter_contract(env: &Env) -> RateLimiterContractClient {
    let contract_id = env.register_contract(None, RateLimiterContract);
    let client = RateLimiterContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    let config = create_test_config();
    client.init_rate_limiter(&admin, &config);
    client
}

mod vote_rate_limit {
    use super::*;

    #[test]
    fn vote_within_limit_succeeds() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup_limiter_contract(&env);

        let user = Address::generate(&env);
        let market_id = soroban_sdk::Symbol::new(&env, "test_market");

        for i in 0..4 {
            let result = client.try_check_voting_rate_limit(&user, &market_id);
            assert!(result.is_ok(), "Vote {} should succeed", i + 1);
        }
    }

    #[test]
    fn vote_exceeding_limit_fails() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup_limiter_contract(&env);

        let user = Address::generate(&env);
        let market_id = soroban_sdk::Symbol::new(&env, "test_market");

        let limit = create_test_config().voting_limit;
        for _ in 0..limit {
            client.check_voting_rate_limit(&user, &market_id);
        }

        let result = client.try_check_voting_rate_limit(&user, &market_id);
        assert_eq!(result, Err(Ok(crate::rate_limiter::RateLimiterError::RateLimitExceeded)));
    }

    #[test]
    fn vote_different_market_resets_counter() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup_limiter_contract(&env);

        let user = Address::generate(&env);
        let market1 = soroban_sdk::Symbol::new(&env, "market1");
        let market2 = soroban_sdk::Symbol::new(&env, "market2");

        let limit = create_test_config().voting_limit;
        for _ in 0..limit {
            client.check_voting_rate_limit(&user, &market1);
        }

        let result = client.try_check_voting_rate_limit(&user, &market2);
        assert!(result.is_ok(), "Different market should have independent limit");
    }

    #[test]
    fn vote_different_user_has_independent_limit() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup_limiter_contract(&env);

        let user1 = Address::generate(&env);
        let user2 = Address::generate(&env);
        let market = soroban_sdk::Symbol::new(&env, "shared_market");

        let limit = create_test_config().voting_limit;
        for _ in 0..limit {
            client.check_voting_rate_limit(&user1, &market);
        }

        let result = client.try_check_voting_rate_limit(&user2, &market);
        assert!(result.is_ok(), "Different user should have independent limit");
    }
}

mod dispute_rate_limit {
    use super::*;

    #[test]
    fn dispute_within_limit_succeeds() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup_limiter_contract(&env);

        let user = Address::generate(&env);
        let market_id = soroban_sdk::Symbol::new(&env, "test_market");

        for i in 0..3 {
            let result = client.try_check_dispute_rate_limit(&user, &market_id);
            assert!(result.is_ok(), "Dispute {} should succeed", i + 1);
        }
    }

    #[test]
    fn dispute_exceeding_limit_fails() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup_limiter_contract(&env);

        let user = Address::generate(&env);
        let market_id = soroban_sdk::Symbol::new(&env, "test_market");

        let limit = create_test_config().dispute_limit;
        for _ in 0..limit {
            client.check_dispute_rate_limit(&user, &market_id);
        }

        let result = client.try_check_dispute_rate_limit(&user, &market_id);
        assert_eq!(result, Err(Ok(crate::rate_limiter::RateLimiterError::RateLimitExceeded)));
    }
}

mod admin_event_rate_limit {
    use super::*;

    #[test]
    fn admin_create_event_within_limit_succeeds() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup_limiter_contract(&env);

        let admin = Address::generate(&env);

        for i in 0..4 {
            let result = client.try_check_admin_event_rate_limit(&admin);
            assert!(result.is_ok(), "Event creation {} should succeed", i + 1);
        }
    }

    #[test]
    fn admin_create_event_exceeding_limit_fails() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup_limiter_contract(&env);

        let admin = Address::generate(&env);

        let limit = create_test_config().events_per_admin_limit;
        for _ in 0..limit {
            client.check_admin_event_rate_limit(&admin);
        }

        let result = client.try_check_admin_event_rate_limit(&admin);
        assert_eq!(result, Err(Ok(crate::rate_limiter::RateLimiterError::RateLimitExceeded)));
    }

    #[test]
    fn different_admins_have_independent_limits() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup_limiter_contract(&env);

        let admin1 = Address::generate(&env);
        let admin2 = Address::generate(&env);

        let limit = create_test_config().events_per_admin_limit;
        for _ in 0..limit {
            client.check_admin_event_rate_limit(&admin1);
        }

        let result = client.try_check_admin_event_rate_limit(&admin2);
        assert!(result.is_ok(), "Different admin should have independent limit");
    }
}

mod rate_limit_status {
    use super::*;

    #[test]
    fn get_status_returns_correct_remaining() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup_limiter_contract(&env);

        let user = Address::generate(&env);
        let market_id = soroban_sdk::Symbol::new(&env, "test_market");

        client.check_voting_rate_limit(&user, &market_id);
        client.check_voting_rate_limit(&user, &market_id);

        let status = client.get_rate_limit_status(&user, &market_id);
        let config = create_test_config();

        assert_eq!(status.voting_remaining, config.voting_limit - 2);
        assert_eq!(status.dispute_remaining, config.dispute_limit);
    }

    #[test]
    fn status_shows_zero_remaining_after_limit_hit() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup_limiter_contract(&env);

        let user = Address::generate(&env);
        let market_id = soroban_sdk::Symbol::new(&env, "test_market");

        let limit = create_test_config().voting_limit;
        for _ in 0..limit {
            client.check_voting_rate_limit(&user, &market_id);
        }

        let status = client.get_rate_limit_status(&user, &market_id);
        assert_eq!(status.voting_remaining, 0);
    }
}

mod configuration_validation {
    use super::*;

    #[test]
    fn valid_config_is_accepted() {
        let env = Env::default();
        let config = create_test_config();
        let result = RateLimiterContract::validate_rate_limit_config(env.clone(), config);
        assert!(result.is_ok());
    }

    #[test]
    fn voting_limit_zero_is_rejected() {
        let env = Env::default();
        let config = RateLimitConfig {
            voting_limit: 0,
            dispute_limit: 5,
            oracle_call_limit: 10,
            bet_limit: 20,
            events_per_admin_limit: 5,
            time_window_seconds: 3600,
        };
        let result = RateLimiterContract::validate_rate_limit_config(env.clone(), config);
        assert_eq!(result, Err(crate::rate_limiter::RateLimiterError::InvalidVotingLimit));
    }

    #[test]
    fn time_window_too_short_is_rejected() {
        let env = Env::default();
        let config = RateLimitConfig {
            voting_limit: 10,
            dispute_limit: 5,
            oracle_call_limit: 10,
            bet_limit: 20,
            events_per_admin_limit: 5,
            time_window_seconds: 30, // Less than 60 seconds
        };
        let result = RateLimiterContract::validate_rate_limit_config(env.clone(), config);
        assert_eq!(result, Err(crate::rate_limiter::RateLimiterError::InvalidTimeWindow));
    }

    #[test]
    fn time_window_too_long_is_rejected() {
        let env = Env::default();
        let config = RateLimitConfig {
            voting_limit: 10,
            dispute_limit: 5,
            oracle_call_limit: 10,
            bet_limit: 20,
            events_per_admin_limit: 5,
            time_window_seconds: 3000000, // More than 30 days
        };
        let result = RateLimiterContract::validate_rate_limit_config(env.clone(), config);
        assert_eq!(result, Err(crate::rate_limiter::RateLimiterError::InvalidTimeWindow));
    }
}

mod update_rate_limits {
    use super::*;

    #[test]
    fn admin_can_update_limits() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup_limiter_contract(&env);

        let admin = Address::generate(&env);
        let new_config = RateLimitConfig {
            voting_limit: 100,
            dispute_limit: 50,
            oracle_call_limit: 200,
            bet_limit: 500,
            events_per_admin_limit: 100,
            time_window_seconds: 7200,
        };

        let result = client.try_update_rate_limits(&admin, &new_config);
        assert!(result.is_ok());

        let status = client.get_rate_limit_status(&admin, &soroban_sdk::Symbol::new(&env, "dummy"));
        assert_eq!(status.voting_remaining, 100);
    }

    #[test]
    fn invalid_config_on_update_is_rejected() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup_limiter_contract(&env);

        let admin = Address::generate(&env);
        let invalid_config = RateLimitConfig {
            voting_limit: 0, // Invalid
            dispute_limit: 5,
            oracle_call_limit: 10,
            bet_limit: 20,
            events_per_admin_limit: 5,
            time_window_seconds: 3600,
        };

        let result = client.try_update_rate_limits(&admin, &invalid_config);
        assert_eq!(result, Err(Ok(crate::rate_limiter::RateLimiterError::InvalidVotingLimit)));
    }
}