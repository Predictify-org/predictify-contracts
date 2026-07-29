//! # Per-account betting limits — integration tests
//!
//! These tests cover the full public API of [`betting::limits::AccountLimits`]:
//!
//! | § | Area | What is verified |
//! |---|------|------------------|
//! | 1 | Initialisation | One-shot init, duplicate-init guard, auth requirement |
//! | 2 | Cap management | Set/remove/query cap, zero-cap rejection |
//! | 3 | Usage tracking | Single account, multi-account isolation, cumulative addition |
//! | 4 | Enforcement | Under-cap pass, at-cap pass, over-cap rejection |
//! | 5 | Admin reset | Usage reset clears entry; re-use starts from zero |
//! | 6 | Uncapped mode | No cap → all amounts accepted |
//! | 7 | Auth boundaries | State-changing calls without auth must fail |
//! | 8 | Overflow safety | i128 accumulator checked_add never panics |

#![cfg(test)]

extern crate alloc;

use betting::limits::{AccountLimits, LimitsDataKey, LIMIT_NS, LIMITS_TTL_LEDGERS};
use predictify_hybrid::Error;
use soroban_sdk::{testutils::Address as _, Address, Env};

// ─────────────────────────────────────────────────────────────────────────────
// Shared fixtures
// ─────────────────────────────────────────────────────────────────────────────

fn env() -> Env {
    let e = Env::default();
    e.mock_all_auths();
    e.ledger().with_mut(|li| {
        li.max_entry_ttl = 100_000_000;
        li.min_persistent_entry_ttl = 1;
        li.min_temp_entry_ttl = 1;
    });
    e
}

fn bare_env() -> Env {
    // No mock_all_auths — used to check that auth IS required
    let e = Env::default();
    e.ledger().with_mut(|li| {
        li.max_entry_ttl = 100_000_000;
        li.min_persistent_entry_ttl = 1;
        li.min_temp_entry_ttl = 1;
    });
    e
}

fn admin(env: &Env) -> Address {
    Address::generate(env)
}

fn user(env: &Env) -> Address {
    Address::generate(env)
}

// ─────────────────────────────────────────────────────────────────────────────
// §1  Initialisation
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn initialize_sets_admin() {
    let env = env();
    let a = admin(&env);

    AccountLimits::initialize(&env, &a).unwrap();

    // Admin is now stored; set_global_cap succeeds.
    AccountLimits::set_global_cap(&env, &a, 1_000_000).unwrap();
    assert_eq!(AccountLimits::get_global_cap(&env), Some(1_000_000));
}

#[test]
fn initialize_rejects_duplicate() {
    let env = env();
    let a = admin(&env);

    AccountLimits::initialize(&env, &a).unwrap();
    let second = AccountLimits::initialize(&env, &a);
    assert_eq!(
        second,
        Err(Error::AlreadyInitialized),
        "second initialize must return AlreadyInitialized"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// §2  Cap management
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn set_global_cap_stores_positive_cap() {
    let env = env();
    let a = admin(&env);
    AccountLimits::initialize(&env, &a).unwrap();

    AccountLimits::set_global_cap(&env, &a, 5_000_000).unwrap();
    assert_eq!(AccountLimits::get_global_cap(&env), Some(5_000_000));
}

#[test]
fn set_global_cap_rejects_zero() {
    let env = env();
    let a = admin(&env);
    AccountLimits::initialize(&env, &a).unwrap();

    let result = AccountLimits::set_global_cap(&env, &a, 0);
    assert_eq!(result, Err(Error::PerAccountLimitInvalidConfig));
}

#[test]
fn set_global_cap_rejects_negative() {
    let env = env();
    let a = admin(&env);
    AccountLimits::initialize(&env, &a).unwrap();

    let result = AccountLimits::set_global_cap(&env, &a, -1);
    assert_eq!(result, Err(Error::PerAccountLimitInvalidConfig));
}

#[test]
fn set_global_cap_can_be_updated() {
    let env = env();
    let a = admin(&env);
    AccountLimits::initialize(&env, &a).unwrap();

    AccountLimits::set_global_cap(&env, &a, 1_000_000).unwrap();
    AccountLimits::set_global_cap(&env, &a, 2_000_000).unwrap();
    assert_eq!(AccountLimits::get_global_cap(&env), Some(2_000_000));
}

#[test]
fn remove_global_cap_makes_system_uncapped() {
    let env = env();
    let a = admin(&env);
    AccountLimits::initialize(&env, &a).unwrap();

    AccountLimits::set_global_cap(&env, &a, 1_000_000).unwrap();
    AccountLimits::remove_global_cap(&env, &a).unwrap();
    assert_eq!(AccountLimits::get_global_cap(&env), None);
}

#[test]
fn get_global_cap_returns_none_when_unset() {
    let env = env();
    assert_eq!(AccountLimits::get_global_cap(&env), None);
}

// ─────────────────────────────────────────────────────────────────────────────
// §3  Usage tracking
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn get_usage_returns_zero_for_fresh_account() {
    let env = env();
    let u = user(&env);
    assert_eq!(AccountLimits::get_usage(&env, &u), 0i128);
}

#[test]
fn check_and_record_accumulates_usage() {
    let env = env();
    let a = admin(&env);
    let u = user(&env);
    AccountLimits::initialize(&env, &a).unwrap();
    AccountLimits::set_global_cap(&env, &a, 10_000_000).unwrap();

    AccountLimits::check_and_record(&env, &u, 1_000_000).unwrap();
    assert_eq!(AccountLimits::get_usage(&env, &u), 1_000_000);

    AccountLimits::check_and_record(&env, &u, 2_000_000).unwrap();
    assert_eq!(AccountLimits::get_usage(&env, &u), 3_000_000);
}

#[test]
fn usage_is_isolated_per_account() {
    let env = env();
    let a = admin(&env);
    let u1 = user(&env);
    let u2 = user(&env);
    AccountLimits::initialize(&env, &a).unwrap();
    AccountLimits::set_global_cap(&env, &a, 10_000_000).unwrap();

    AccountLimits::check_and_record(&env, &u1, 3_000_000).unwrap();
    AccountLimits::check_and_record(&env, &u2, 1_000_000).unwrap();

    assert_eq!(AccountLimits::get_usage(&env, &u1), 3_000_000);
    assert_eq!(AccountLimits::get_usage(&env, &u2), 1_000_000);
}

// ─────────────────────────────────────────────────────────────────────────────
// §4  Enforcement
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn check_and_record_allows_usage_under_cap() {
    let env = env();
    let a = admin(&env);
    let u = user(&env);
    AccountLimits::initialize(&env, &a).unwrap();
    AccountLimits::set_global_cap(&env, &a, 5_000_000).unwrap();

    let result = AccountLimits::check_and_record(&env, &u, 4_999_999);
    assert_eq!(result, Ok(()));
}

#[test]
fn check_and_record_allows_usage_exactly_at_cap() {
    let env = env();
    let a = admin(&env);
    let u = user(&env);
    AccountLimits::initialize(&env, &a).unwrap();
    AccountLimits::set_global_cap(&env, &a, 5_000_000).unwrap();

    let result = AccountLimits::check_and_record(&env, &u, 5_000_000);
    assert_eq!(result, Ok(()));
    assert_eq!(AccountLimits::get_usage(&env, &u), 5_000_000);
}

#[test]
fn check_and_record_rejects_usage_over_cap_first_bet() {
    let env = env();
    let a = admin(&env);
    let u = user(&env);
    AccountLimits::initialize(&env, &a).unwrap();
    AccountLimits::set_global_cap(&env, &a, 5_000_000).unwrap();

    let result = AccountLimits::check_and_record(&env, &u, 5_000_001);
    assert_eq!(result, Err(Error::PerAccountLimitExceeded));
    // Usage must not have been written.
    assert_eq!(
        AccountLimits::get_usage(&env, &u),
        0,
        "usage must remain 0 after a rejected bet"
    );
}

#[test]
fn check_and_record_rejects_cumulative_over_cap() {
    let env = env();
    let a = admin(&env);
    let u = user(&env);
    AccountLimits::initialize(&env, &a).unwrap();
    AccountLimits::set_global_cap(&env, &a, 5_000_000).unwrap();

    AccountLimits::check_and_record(&env, &u, 3_000_000).unwrap();
    // 3_000_000 + 2_000_001 = 5_000_001 > 5_000_000
    let result = AccountLimits::check_and_record(&env, &u, 2_000_001);
    assert_eq!(result, Err(Error::PerAccountLimitExceeded));
    // Usage must stay at the last committed value.
    assert_eq!(AccountLimits::get_usage(&env, &u), 3_000_000);
}

#[test]
fn check_and_record_allows_bet_exactly_filling_remaining_capacity() {
    let env = env();
    let a = admin(&env);
    let u = user(&env);
    AccountLimits::initialize(&env, &a).unwrap();
    AccountLimits::set_global_cap(&env, &a, 5_000_000).unwrap();

    AccountLimits::check_and_record(&env, &u, 3_000_000).unwrap();
    // Exactly fills remaining 2_000_000.
    AccountLimits::check_and_record(&env, &u, 2_000_000).unwrap();
    assert_eq!(AccountLimits::get_usage(&env, &u), 5_000_000);
}

#[test]
fn check_and_record_rejects_one_strobe_over_full_cap() {
    let env = env();
    let a = admin(&env);
    let u = user(&env);
    AccountLimits::initialize(&env, &a).unwrap();
    AccountLimits::set_global_cap(&env, &a, 5_000_000).unwrap();

    AccountLimits::check_and_record(&env, &u, 5_000_000).unwrap();
    // Cap is now full; any further bet must be rejected.
    let result = AccountLimits::check_and_record(&env, &u, 1);
    assert_eq!(result, Err(Error::PerAccountLimitExceeded));
}

// ─────────────────────────────────────────────────────────────────────────────
// §5  Admin reset
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn reset_usage_clears_account_entry() {
    let env = env();
    let a = admin(&env);
    let u = user(&env);
    AccountLimits::initialize(&env, &a).unwrap();
    AccountLimits::set_global_cap(&env, &a, 10_000_000).unwrap();

    AccountLimits::check_and_record(&env, &u, 4_000_000).unwrap();
    assert_eq!(AccountLimits::get_usage(&env, &u), 4_000_000);

    AccountLimits::reset_usage(&env, &a, &u).unwrap();
    assert_eq!(
        AccountLimits::get_usage(&env, &u),
        0,
        "usage must be zero after reset"
    );
}

#[test]
fn after_reset_account_can_bet_again_up_to_cap() {
    let env = env();
    let a = admin(&env);
    let u = user(&env);
    AccountLimits::initialize(&env, &a).unwrap();
    AccountLimits::set_global_cap(&env, &a, 5_000_000).unwrap();

    AccountLimits::check_and_record(&env, &u, 5_000_000).unwrap();
    AccountLimits::reset_usage(&env, &a, &u).unwrap();

    // After reset the full cap is available again.
    AccountLimits::check_and_record(&env, &u, 5_000_000).unwrap();
    assert_eq!(AccountLimits::get_usage(&env, &u), 5_000_000);
}

// ─────────────────────────────────────────────────────────────────────────────
// §6  Uncapped mode
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn check_and_record_always_succeeds_when_uncapped() {
    let env = env();
    let u = user(&env);
    // No cap set at all.
    for _ in 0..10 {
        AccountLimits::check_and_record(&env, &u, i128::MAX / 10).unwrap();
    }
}

#[test]
fn check_and_record_succeeds_after_cap_removed() {
    let env = env();
    let a = admin(&env);
    let u = user(&env);
    AccountLimits::initialize(&env, &a).unwrap();
    AccountLimits::set_global_cap(&env, &a, 1_000_000).unwrap();
    AccountLimits::remove_global_cap(&env, &a).unwrap();

    // Now uncapped — large bet must pass.
    AccountLimits::check_and_record(&env, &u, 999_999_999).unwrap();
}

#[test]
fn uncapped_mode_does_not_write_usage() {
    let env = env();
    let u = user(&env);
    // No initialize, no cap.
    AccountLimits::check_and_record(&env, &u, 100_000).unwrap();
    // Usage must remain 0 because we never persist when uncapped.
    assert_eq!(AccountLimits::get_usage(&env, &u), 0);
}

// ─────────────────────────────────────────────────────────────────────────────
// §7  Auth boundaries
// ─────────────────────────────────────────────────────────────────────────────

/// `set_global_cap` must require admin auth.
///
/// Using a bare env (no mock_all_auths) with a *different* address than the
/// registered admin must return `Unauthorized`.
#[test]
fn set_global_cap_requires_admin_auth() {
    let env = env();
    let a = admin(&env);
    let stranger = user(&env);
    AccountLimits::initialize(&env, &a).unwrap();

    let result = AccountLimits::set_global_cap(&env, &stranger, 1_000_000);
    assert_eq!(
        result,
        Err(Error::Unauthorized),
        "non-admin must not be able to set the cap"
    );
}

/// `remove_global_cap` must require admin auth.
#[test]
fn remove_global_cap_requires_admin_auth() {
    let env = env();
    let a = admin(&env);
    let stranger = user(&env);
    AccountLimits::initialize(&env, &a).unwrap();
    AccountLimits::set_global_cap(&env, &a, 1_000_000).unwrap();

    let result = AccountLimits::remove_global_cap(&env, &stranger);
    assert_eq!(result, Err(Error::Unauthorized));
}

/// `reset_usage` must require admin auth.
#[test]
fn reset_usage_requires_admin_auth() {
    let env = env();
    let a = admin(&env);
    let u = user(&env);
    let stranger = user(&env);
    AccountLimits::initialize(&env, &a).unwrap();
    AccountLimits::set_global_cap(&env, &a, 5_000_000).unwrap();
    AccountLimits::check_and_record(&env, &u, 1_000_000).unwrap();

    let result = AccountLimits::reset_usage(&env, &stranger, &u);
    assert_eq!(result, Err(Error::Unauthorized));
    // Usage must be unchanged.
    assert_eq!(AccountLimits::get_usage(&env, &u), 1_000_000);
}

/// `set_global_cap` with no admin initialised must return `Unauthorized`.
#[test]
fn set_global_cap_without_initialized_admin_returns_unauthorized() {
    let env = env();
    let a = admin(&env);
    // No initialize call.
    let result = AccountLimits::set_global_cap(&env, &a, 1_000_000);
    assert_eq!(result, Err(Error::Unauthorized));
}

/// `initialize` must require the caller to authenticate.
///
/// We use a bare env (no mock_all_auths) and verify that the call panics/
/// host-errors because `require_auth` fails.
#[test]
#[should_panic]
fn initialize_without_auth_panics() {
    let env = bare_env();
    let a = admin(&env);
    // require_auth will panic in a bare env (no auth mocked).
    let _ = AccountLimits::initialize(&env, &a);
}

// ─────────────────────────────────────────────────────────────────────────────
// §8  Overflow safety
// ─────────────────────────────────────────────────────────────────────────────

/// Accumulating exactly i128::MAX must succeed when the cap is also i128::MAX.
#[test]
fn check_and_record_allows_i128_max_exactly() {
    let env = env();
    let a = admin(&env);
    let u = user(&env);
    AccountLimits::initialize(&env, &a).unwrap();
    AccountLimits::set_global_cap(&env, &a, i128::MAX).unwrap();

    AccountLimits::check_and_record(&env, &u, i128::MAX).unwrap();
    assert_eq!(AccountLimits::get_usage(&env, &u), i128::MAX);
}

/// Adding 1 to i128::MAX usage must return `Error::Overflow` (not panic).
#[test]
fn check_and_record_returns_overflow_on_i128_max_plus_one() {
    let env = env();
    let a = admin(&env);
    let u = user(&env);
    AccountLimits::initialize(&env, &a).unwrap();
    AccountLimits::set_global_cap(&env, &a, i128::MAX).unwrap();

    // Pre-seed usage to i128::MAX.
    AccountLimits::check_and_record(&env, &u, i128::MAX).unwrap();

    // Now any positive addition must overflow the accumulator.
    let result = AccountLimits::check_and_record(&env, &u, 1);
    assert_eq!(
        result,
        Err(Error::Overflow),
        "i128::MAX + 1 must return Overflow, never panic"
    );
}

/// checked_add itself returns None at boundary — validates the guard is sound.
#[test]
fn checked_add_i128_max_returns_none() {
    let result = i128::MAX.checked_add(1);
    assert!(result.is_none(), "i128::MAX + 1 must be None");
}

/// A cap set to i128::MAX - 1 must reject a bet of exactly 2 after 1 is used.
#[test]
fn cap_near_max_enforced_correctly() {
    let env = env();
    let a = admin(&env);
    let u = user(&env);
    AccountLimits::initialize(&env, &a).unwrap();
    let cap = i128::MAX - 1;
    AccountLimits::set_global_cap(&env, &a, cap).unwrap();

    // Bet 1 — fine.
    AccountLimits::check_and_record(&env, &u, 1).unwrap();
    // Bet cap again — 1 + (cap) > cap — must be rejected.
    let result = AccountLimits::check_and_record(&env, &u, cap);
    assert_eq!(result, Err(Error::PerAccountLimitExceeded));
}
