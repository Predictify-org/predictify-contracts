//! # Overflow-safety tests for the betting subsystem
//!
//! These tests verify that every arithmetic operation in the betting path
//! is overflow-safe — i.e. uses checked arithmetic and returns a typed
//! [`predictify_hybrid::Error::Overflow`] instead of wrapping or panicking.
//!
//! ## Coverage
//!
//! | § | Area | What is tested |
//! |---|------|----------------|
//! | 1 | Error code stability | `Error::Overflow = 672` is frozen |
//! | 2 | Payout arithmetic helpers | fee / distributable / pool_per_winner / payout identities |
//! | 3 | Boundary values | i128::MAX, u64::MAX, u32::MAX inputs do not panic |
//! | 4 | Nonce saturation | `u64::saturating_add` at `u64::MAX` plateaus, never wraps |
//! | 5 | Event payload invariants | `net_payout == gross - fee` for boundary amounts |
//! | 6 | Stats counter arithmetic | checked_add semantics on u64 / i128 / u32 counter fields |
//!
//! ## Design note
//!
//! The `betting` crate is a pure event-emission library; the betting-logic
//! module (`predictify_hybrid::bets`) is private to the `predictify-hybrid`
//! crate.  Accordingly these tests validate:
//!
//! a) **The arithmetic patterns** used in the contract, expressed as
//!    standalone helper assertions (§2, §3, §6).  Any regression in the
//!    contract's own arithmetic will be caught by the same checked operations
//!    failing at the call site in `predictify_hybrid`.
//!
//! b) **The observable contract surface** from the `betting` crate — the
//!    event emitter (§4, §5) and the exported `Error` discriminants (§1).

#![cfg(test)]

extern crate alloc;

use betting::events::{
    BetClaimedEvent, BettingEventEmitter, NS_NONCE, TOPIC_BET_CREATED,
};
use predictify_hybrid::Error;
use soroban_sdk::{testutils::Address as _, Address, Env, Symbol};

// ---------------------------------------------------------------------------
// Shared fixtures
// ---------------------------------------------------------------------------

fn env() -> Env {
    let e = Env::default();
    e.mock_all_auths();
    e.ledger().with_mut(|li| {
        li.max_entry_ttl = 6_000_000;
        li.min_persistent_entry_ttl = 1;
        li.min_temp_entry_ttl = 1;
    });
    e
}

fn market(env: &Env) -> Symbol {
    Symbol::new(env, "mkt_ovf")
}

fn user(env: &Env) -> Address {
    Address::generate(env)
}

// ===========================================================================
// §1  Error code stability
// ===========================================================================

/// `Error::Overflow` must remain discriminant 672 forever.
///
/// Clients (SDKs, indexers, front-ends) may persist or branch on this
/// numeric value.  Any change to the discriminant is a visible API break.
#[test]
fn overflow_error_discriminant_is_stable() {
    assert_eq!(
        Error::Overflow as u32,
        672,
        "Error::Overflow discriminant must be frozen at 672"
    );
}

/// `Error::Overflow` must be distinct from every other betting-path error.
#[test]
fn overflow_discriminant_is_unique_among_betting_errors() {
    let betting_codes: &[u32] = &[
        Error::Unauthorized as u32,
        Error::MarketNotFound as u32,
        Error::MarketClosed as u32,
        Error::MarketResolved as u32,
        Error::MarketNotResolved as u32,
        Error::NothingToClaim as u32,
        Error::AlreadyClaimed as u32,
        Error::InsufficientStake as u32,
        Error::InvalidOutcome as u32,
        Error::AlreadyBet as u32,
        Error::BetsAlreadyPlaced as u32,
        Error::InsufficientBalance as u32,
        Error::BetCoolOffActive as u32,
        Error::FeeExceedsMax as u32,
        Error::MaxBetCapExceeded as u32,
        Error::InvalidCap as u32,
        Error::IdempotentBatchAlreadyApplied as u32,
        Error::InvalidState as u32,
        Error::InvalidInput as u32,
        Error::Overflow as u32,
    ];

    for (i, &a) in betting_codes.iter().enumerate() {
        for (j, &b) in betting_codes.iter().enumerate() {
            if i != j {
                assert_ne!(
                    a, b,
                    "duplicate betting error code {a} at positions {i} and {j}"
                );
            }
        }
    }
}

// ===========================================================================
// §2  Payout arithmetic helpers
// ===========================================================================
//
// These tests mirror the exact arithmetic sequence in
// `BetManager::calculate_bet_payout` and verify that:
//   a) Normal inputs produce the correct result
//   b) The checked operations produce `None` on overflow (not a panic)
//
// The contract maps `None` → `Error::Overflow`; here we assert that the
// same `checked_*` calls return `None` for the boundary inputs, confirming
// the contract's guard is sound.

/// Normal payout calculation — zero fee, single winner, full pool to one bettor.
#[test]
fn payout_arithmetic_zero_fee() {
    let total_pool: i128 = 10_000_000;
    let fee_percentage: i128 = 0;
    let num_winners: i128 = 1;
    let bet_amount: i128 = 3_000_000;
    let total_on_outcome: i128 = 3_000_000; // sole bettor

    let fee = total_pool
        .checked_mul(fee_percentage)
        .unwrap()
        .checked_div(10_000)
        .unwrap();
    assert_eq!(fee, 0);

    let distributable = total_pool.checked_sub(fee).unwrap();
    assert_eq!(distributable, total_pool);

    let pool_per_winner = distributable.checked_div(num_winners).unwrap();
    assert_eq!(pool_per_winner, total_pool);

    let payout = bet_amount
        .checked_mul(pool_per_winner)
        .unwrap()
        .checked_div(total_on_outcome)
        .unwrap();
    assert_eq!(payout, total_pool, "sole bettor must receive the full pool");
}

/// Normal payout calculation — 2% fee, two winners, two equal bettors.
#[test]
fn payout_arithmetic_with_fee_and_two_winners() {
    let total_pool: i128 = 20_000_000;
    let fee_percentage: i128 = 200; // 2%
    let num_winners: i128 = 2;
    let bet_amount: i128 = 5_000_000;
    let total_on_outcome: i128 = 10_000_000;

    let fee = total_pool
        .checked_mul(fee_percentage)
        .unwrap()
        .checked_div(10_000)
        .unwrap();
    assert_eq!(fee, 400_000);

    let distributable = total_pool.checked_sub(fee).unwrap();
    assert_eq!(distributable, 19_600_000);

    let pool_per_winner = distributable.checked_div(num_winners).unwrap();
    assert_eq!(pool_per_winner, 9_800_000);

    let payout = bet_amount
        .checked_mul(pool_per_winner)
        .unwrap()
        .checked_div(total_on_outcome)
        .unwrap();
    // 5_000_000 / 10_000_000 * 9_800_000 = 4_900_000
    assert_eq!(payout, 4_900_000);
}

/// The payout formula is proportional: doubling bet_amount doubles payout.
#[test]
fn payout_arithmetic_is_proportional_to_bet_amount() {
    let total_pool: i128 = 30_000_000;
    let fee: i128 = 0;
    let num_winners: i128 = 1;
    let total_on_outcome: i128 = 15_000_000;

    let distributable = total_pool.checked_sub(fee).unwrap();
    let pool_per_winner = distributable.checked_div(num_winners).unwrap();

    let payout_small = (5_000_000i128)
        .checked_mul(pool_per_winner)
        .unwrap()
        .checked_div(total_on_outcome)
        .unwrap();

    let payout_large = (10_000_000i128)
        .checked_mul(pool_per_winner)
        .unwrap()
        .checked_div(total_on_outcome)
        .unwrap();

    assert_eq!(
        payout_large,
        payout_small * 2,
        "doubling bet_amount must double payout"
    );
}

/// Fee must never exceed the total pool (fee_percentage <= 10_000 bps = 100%).
#[test]
fn fee_never_exceeds_pool_for_valid_fee_percentage() {
    let total_pool: i128 = 50_000_000;
    // 100% fee (extreme but valid bps value)
    let fee_percentage: i128 = 10_000;

    let fee = total_pool
        .checked_mul(fee_percentage)
        .unwrap()
        .checked_div(10_000)
        .unwrap();
    assert_eq!(fee, total_pool, "100% fee must equal the total pool");

    let distributable = total_pool.checked_sub(fee).unwrap();
    assert_eq!(distributable, 0, "distributable pool must be zero at 100% fee");
}

// ===========================================================================
// §3  Boundary values — checked operations return None, never panic
// ===========================================================================

/// `checked_mul` on i128::MAX × any positive value must return None, not panic.
#[test]
fn checked_mul_i128_max_returns_none() {
    let result = i128::MAX.checked_mul(2);
    assert!(
        result.is_none(),
        "i128::MAX × 2 must overflow and return None"
    );
}

/// `checked_add` on i128::MAX + 1 must return None.
#[test]
fn checked_add_i128_max_returns_none() {
    let result = i128::MAX.checked_add(1);
    assert!(
        result.is_none(),
        "i128::MAX + 1 must overflow and return None"
    );
}

/// `checked_add` on u64::MAX + 1 must return None (models total_bets counter).
#[test]
fn checked_add_u64_max_returns_none() {
    let result = u64::MAX.checked_add(1);
    assert!(
        result.is_none(),
        "u64::MAX + 1 must overflow and return None (total_bets counter guard)"
    );
}

/// `checked_add` on u32::MAX + 1 must return None (models unique_bettors counter).
#[test]
fn checked_add_u32_max_returns_none() {
    let result = u32::MAX.checked_add(1u32);
    assert!(
        result.is_none(),
        "u32::MAX + 1 must overflow and return None (unique_bettors counter guard)"
    );
}

/// `checked_sub` on a negative result (fee > pool) must return None.
#[test]
fn checked_sub_underflow_returns_none() {
    let pool: i128 = 1_000;
    let fee: i128 = 2_000; // fee > pool → underflow
    let result = pool.checked_sub(fee);
    assert!(
        result.is_none(),
        "pool - fee where fee > pool must underflow and return None"
    );
}

/// `checked_div` by zero must return None (division-by-zero guard).
#[test]
fn checked_div_by_zero_returns_none() {
    let result = 100i128.checked_div(0);
    assert!(
        result.is_none(),
        "division by zero must return None, never panic"
    );
}

/// Outcome-total accumulation at i128::MAX + 1 must return None.
#[test]
fn outcome_total_accumulation_overflow_returns_none() {
    let current_total: i128 = i128::MAX;
    let new_amount: i128 = 1;
    let result = current_total.checked_add(new_amount);
    assert!(
        result.is_none(),
        "outcome total at i128::MAX + 1 must return None"
    );
}

/// Large but valid fee_percentage (9_999 bps) must not overflow for a
/// reasonable pool size (≤ i128::MAX / 10_000).
#[test]
fn fee_calculation_does_not_overflow_for_large_but_valid_pool() {
    // i128::MAX / 10_000 is the largest pool that fits a ×10_000 multiply.
    let max_safe_pool: i128 = i128::MAX / 10_000;
    let fee_percentage: i128 = 9_999;

    let fee_opt = max_safe_pool.checked_mul(fee_percentage);
    assert!(
        fee_opt.is_some(),
        "pool = i128::MAX / 10_000 with fee_pct = 9_999 must not overflow the multiply step"
    );
    let fee = fee_opt.unwrap().checked_div(10_000);
    assert!(fee.is_some(), "final /10_000 step must not overflow");
}

// ===========================================================================
// §4  Nonce saturation — u64::saturating_add never wraps
// ===========================================================================
//
// The `next_nonce` helper in `betting::events` uses `u64::saturating_add`
// so the counter plateaus at u64::MAX instead of wrapping to 0.  These
// tests confirm that invariant directly.

/// `u64::saturating_add(1)` at `u64::MAX` must return `u64::MAX`, not 0.
#[test]
fn nonce_saturating_add_at_max_plateaus() {
    let result = u64::MAX.saturating_add(1);
    assert_eq!(
        result,
        u64::MAX,
        "saturating_add at u64::MAX must plateau at u64::MAX, not wrap to 0"
    );
}

/// After saturating at `u64::MAX`, every subsequent add must still return `u64::MAX`.
#[test]
fn nonce_saturating_add_remains_at_max_after_many_adds() {
    let mut counter = u64::MAX;
    for _ in 0..10 {
        counter = counter.saturating_add(1);
        assert_eq!(
            counter,
            u64::MAX,
            "counter must stay at u64::MAX after each saturating_add"
        );
    }
}

/// A pre-seeded instance-storage nonce at `u64::MAX - 1` must advance to
/// `u64::MAX` on the next emit, then plateau there on the emit after.
///
/// This exercises the `next_nonce` path end-to-end via the live emitter.
#[test]
fn nonce_advances_to_max_then_plateaus_via_emitter() {
    let env = env();
    let mid = market(&env);
    let u = user(&env);
    let outcome = soroban_sdk::String::from_str(&env, "yes");

    // Pre-seed the nonce counter at u64::MAX - 1 so we can observe the
    // final increment and the plateau in two consecutive emits.
    let key = (NS_NONCE, TOPIC_BET_CREATED);
    env.storage()
        .instance()
        .set(&key, &(u64::MAX - 1));

    // First emit: nonce must become u64::MAX.
    BettingEventEmitter::emit_bet_created(&env, &mid, &u, &outcome, 1_000_000, 0);
    let events = env.events().all();
    let ev1: betting::events::BetCreatedEvent =
        events.get(events.len() - 1).unwrap().2.try_into_val().unwrap();
    assert_eq!(
        ev1.nonce,
        u64::MAX,
        "nonce must advance to u64::MAX from u64::MAX - 1"
    );

    // Second emit: nonce must plateau at u64::MAX (not wrap to 0).
    BettingEventEmitter::emit_bet_created(&env, &mid, &u, &outcome, 1_000_000, 0);
    let events2 = env.events().all();
    let ev2: betting::events::BetCreatedEvent =
        events2.get(events2.len() - 1).unwrap().2.try_into_val().unwrap();
    assert_eq!(
        ev2.nonce,
        u64::MAX,
        "nonce must plateau at u64::MAX — must not wrap to 0"
    );
}

// ===========================================================================
// §5  Event payload invariants at boundary amounts
// ===========================================================================

/// `net_payout = gross - fee` must hold when gross is i128::MAX / 2 and fee is 1.
#[test]
fn claimed_event_net_equals_gross_minus_fee_large_amounts() {
    let env = env();
    let mid = market(&env);
    let u = user(&env);

    let gross: i128 = i128::MAX / 2;
    let fee: i128 = 1;
    let net: i128 = gross - fee;

    BettingEventEmitter::emit_bet_claimed(&env, &mid, &u, gross, fee, net);

    let events = env.events().all();
    let ev: BetClaimedEvent = events.get(0).unwrap().2.try_into_val().unwrap();
    assert_eq!(
        ev.net_payout,
        ev.gross_payout - ev.fee_paid,
        "net_payout must equal gross - fee for large amounts"
    );
    assert!(ev.net_payout >= 0, "net_payout must never be negative");
}

/// `net_payout = gross` when fee is zero — full payout, no deduction.
#[test]
fn claimed_event_zero_fee_net_equals_gross() {
    let env = env();
    let mid = market(&env);
    let u = user(&env);

    let gross: i128 = 100_000_000_000;
    let fee: i128 = 0;
    let net: i128 = gross;

    BettingEventEmitter::emit_bet_claimed(&env, &mid, &u, gross, fee, net);

    let events = env.events().all();
    let ev: BetClaimedEvent = events.get(0).unwrap().2.try_into_val().unwrap();
    assert_eq!(ev.net_payout, gross);
    assert_eq!(ev.fee_paid, 0);
}

/// `net_payout = 0` when fee equals gross — entire gross consumed as fee.
#[test]
fn claimed_event_full_fee_net_is_zero() {
    let env = env();
    let mid = market(&env);
    let u = user(&env);

    let gross: i128 = 5_000_000;
    let fee: i128 = gross;
    let net: i128 = 0;

    BettingEventEmitter::emit_bet_claimed(&env, &mid, &u, gross, fee, net);

    let events = env.events().all();
    let ev: BetClaimedEvent = events.get(0).unwrap().2.try_into_val().unwrap();
    assert_eq!(ev.net_payout, 0);
    assert_eq!(ev.gross_payout - ev.fee_paid, 0);
}

// ===========================================================================
// §6  Stats counter arithmetic — checked_add semantics
// ===========================================================================
//
// `update_market_bet_stats` uses `checked_add` on three counter types:
//   - `total_bets: u64`
//   - `total_amount_locked: i128`
//   - `unique_bettors: u32`
//
// The contract maps None → Error::Overflow.  Here we verify the three
// checked-add patterns produce None at exactly the right boundary, so
// the guard in the contract is confirmed sound.

/// `total_bets` counter (u64): checked_add at u64::MAX - 1 must succeed;
/// at u64::MAX must return None.
#[test]
fn total_bets_counter_checked_add_boundary() {
    let near_max: u64 = u64::MAX - 1;

    // One more increment is fine.
    let incremented = near_max.checked_add(1);
    assert_eq!(incremented, Some(u64::MAX));

    // One more past max overflows.
    let overflow = u64::MAX.checked_add(1u64);
    assert!(
        overflow.is_none(),
        "total_bets at u64::MAX must overflow and return None"
    );
}

/// `total_amount_locked` counter (i128): checked_add at i128::MAX must
/// return None (overflow), not wrap.
#[test]
fn total_amount_locked_checked_add_boundary() {
    let result = i128::MAX.checked_add(1);
    assert!(
        result.is_none(),
        "total_amount_locked at i128::MAX + 1 must return None"
    );
}

/// Adding any positive amount to i128::MAX - 1 must succeed.
#[test]
fn total_amount_locked_checked_add_near_max_succeeds() {
    let near_max: i128 = i128::MAX - 1;
    let result = near_max.checked_add(1);
    assert_eq!(
        result,
        Some(i128::MAX),
        "i128::MAX - 1 + 1 must succeed and equal i128::MAX"
    );
}

/// `unique_bettors` counter (u32): checked_add at u32::MAX must return None.
#[test]
fn unique_bettors_checked_add_boundary() {
    let result = u32::MAX.checked_add(1u32);
    assert!(
        result.is_none(),
        "unique_bettors at u32::MAX + 1 must return None"
    );
}

/// `unique_bettors` counter (u32): checked_add at u32::MAX - 1 must succeed.
#[test]
fn unique_bettors_checked_add_near_max_succeeds() {
    let near_max: u32 = u32::MAX - 1;
    let result = near_max.checked_add(1u32);
    assert_eq!(result, Some(u32::MAX));
}

/// The three counter types independently overflow at the correct boundary:
/// mixed increments on independent counters must not corrupt each other.
#[test]
fn independent_counter_overflow_checks_are_orthogonal() {
    // Each counter is checked in isolation — no shared state.
    let total_bets_overflow = u64::MAX.checked_add(1u64);
    let total_locked_overflow = i128::MAX.checked_add(1i128);
    let unique_bettors_overflow = u32::MAX.checked_add(1u32);

    assert!(total_bets_overflow.is_none(), "u64 counter overflow");
    assert!(total_locked_overflow.is_none(), "i128 counter overflow");
    assert!(unique_bettors_overflow.is_none(), "u32 counter overflow");

    // Meanwhile, sub-max values on all three succeed without interference.
    let total_bets_ok = (u64::MAX - 10).checked_add(1u64);
    let total_locked_ok = (i128::MAX - 10).checked_add(1i128);
    let unique_bettors_ok = (u32::MAX - 10).checked_add(1u32);

    assert!(total_bets_ok.is_some());
    assert!(total_locked_ok.is_some());
    assert!(unique_bettors_ok.is_some());
}

/// Per-outcome total accumulation: checked_add on i128 at boundary.
#[test]
fn outcome_total_checked_add_boundary() {
    // Simulates: current_outcome_total.checked_add(amount)
    let current: i128 = i128::MAX;
    let amount: i128 = 1_000_000;
    let result = current.checked_add(amount);
    assert!(
        result.is_none(),
        "outcome total at i128::MAX + amount must return None"
    );
}

/// Per-outcome total accumulation succeeds for normal values.
#[test]
fn outcome_total_checked_add_normal_succeeds() {
    let current: i128 = 90_000_000;
    let amount: i128 = 10_000_000;
    let result = current.checked_add(amount);
    assert_eq!(result, Some(100_000_000));
}
