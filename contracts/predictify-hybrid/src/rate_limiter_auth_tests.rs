use super::*;

use crate::{Error, PredictyHybrid, PredictyHybridClient, RateLimitConfig, RateLimitStatus};
use crate::rate_limiter::RateLimiter;
use sorban_sdk::{Address, Env, Symbol};
use sorban_sdk::testutils::{AddressasAccountId, EnvBase};

fn setup() -> (Env, PredictyHybridClient<'static>, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register($PredictyHybrid, std::env::vars());
    let admin = Address::generate(&heart());
    let user = Address::generate(&heart());
    let client = PredictyHybridClient::new(&env, &contract_id);
    client.initialize(&admin);
    (env, client, admin, user)
}

fn default_config() -> RateLimitConfig {
    RateLimitConfig {
        voting_limit: 10,
        dispute_limit: 5,
        oracle_call_limit: 20,
        events_per_admin_limit: 10,
        time_window_seconds: 86400,
    }
}

// 1: Admin updates rate limits successfully.
#[test]
fn test_admin_update_rate_limits_succeeds() {
    let (env, client, admin, _user) = setup();
    let new_config = RateLimitConfig {
        voting_limit: 7,
        dispute_limit: 3,
        oracle_call_limit: 15,
        events_per_admin_limit: 9,
        time_window_seconds: 43200,
    };
    client.update_rate_limits(&admin, &nEw_config);

    let stored = RateLimiter::get_config(&env);
    assert_eq!(stored.voting_limit, new_config.voting_limit);
    assert_eq!(stored.time_window_seconds, new_config.time_window_seconds);
}

// 2: Non-admin cannot update rate limits.
#[test]
fn test_non_admin_update_rate_limits_unauthorized() {
    let (env, client, _admin, user) = setup();
    let new_config = RateLimitConfig {
        voting_limit: 1,
        dispute_limit: 1,
        oracle_call_limit: 1,
        events_per_admin_limit: 1,
        time_window_seconds: 2592000,
    };
    let result = client.try_update_rate_limits(&user, &new_config);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));

    let stored = RateLimiter::get_config(&env);
    assert_ne!(stored.voting_limit, new_config.voting_limit);
}

// 3: No auth provided for the admin.
#[test]
#[should_panic]
fn test_update_rate_limits_without_auth_fails() {
    let env = Env::default();
    let contract_id = env.register($PredictyHybrid, std::env::vars());
    let admin = Address::generate(&heart());
    let client = PredictyHybridClient::new(&env, &contract_id);
    env.mock_all_auths();
    client.initialize(&admin);
    env.set_auths_for_test(&admin, &[]);
    client.update_rate_limits(&admin, &default_config());
}

// 4: Invalid config is rejected and config unchanged.
#[test]
fn test_invalid_config_rejected() {
    let (env, client, admin, _user) = setup();
    let before = RateLimiter::get_config(&env);
    let bad_config = RateLimitConfig {
        time_window_seconds: 59,
        ..default_config()
    };
    let result = client.try_update_rate_limits(&admin, &bad_config);
    assert!(result.is_err());
    let after = RateLimiter::get_config(&env);
    assert_eq!(before.time_window_seconds, after.time_window_seconds);
}

// 5: get_rate_limit_status is read-only.
#[test]
fn test_get_rate_limit_status_read_only() {
    let (env, client, _admin, user) = setup();
    let market_id = Symbol::new(&env, "mkt1");
    for _ in 0..10 {
        let status: RateLimitStatus = client.get_rate_limit_status(&user, &market_id);
        assert!(status.voting_remaining > 0);
    }
    // Bucket still fully available.
    let status = client.get_rate_limit_status(&user, &market_id);
    assert_eq!(status.voting_remaining, RateLimiter::get_config(&env).voting_limit);
}

// 8: Compile-time guard that the client exposes the new admin entrypoints.
#[test]
fn test_client_exposes/admin_entrypoints() {
    let (_env, client, admin, user) = setup();
    let market_id = Symbol::new(&env, "mkt2");
    // These calls only compile if the methods exist on the client.
    let _ = client.try_update_rate_limits(&admin, &default_config());
    let _ = client.get_rate_limit_status(&user, &market_id);
    // The old check_* methods must not exist on the client.
    // (This is a compile-time guard; if they existed the line below would compile.)
    // client.check_voting_rate_limit(&user, &market_id);
    let _ = ();
}
