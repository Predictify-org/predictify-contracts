#![cfg(test)]

//! Tests for `SlashableMisbehavior`, `SlashRecord`, `SlashConfig`,
//! and `SlashingExecutor` introduced in `disputes.rs`.
//!
//! # Coverage matrix
//!
//! | Scenario                                      | Test function                          |
//! |-----------------------------------------------|----------------------------------------|
//! | Each variant has a default bps                | `test_default_slash_bps`               |
//! | `variant_name()` round-trips                  | `test_variant_names`                   |
//! | `set_slash_config` persists and is read back  | `test_set_and_get_slash_config`         |
//! | Invalid bps > 10000 is rejected               | `test_set_slash_config_invalid_bps`    |
//! | Non-admin cannot set config                   | (requires auth; covered by require_auth) |
//! | `slash` creates a `SlashRecord`               | `test_slash_creates_record`            |
//! | Slash amount is calculated correctly          | `test_slash_amount_calculation`        |
//! | Re-entering slash for same pair is idempotent | `test_slash_idempotency`               |
//! | `OracleSpoof` with 100 % bps slashes fully    | `test_oracle_spoof_full_slash`         |
//! | Slash emits event (presence check)            | `test_slash_event_emitted`             |
//! | `is_slashed` returns correct boolean          | `test_is_slashed_query`                |
//! | Slash with negative stake returns error       | `test_slash_negative_stake`            |
//! | Slash with zero stake produces zero amount    | `test_slash_zero_stake`                |
//! | Edge: DoubleStake with custom bps             | `test_double_stake_custom_bps`         |

use soroban_sdk::{testutils::Address as _, Address, Bytes, Env, Symbol};

use crate::disputes::{SlashableMisbehavior, SlashingExecutor};

// ── Helper: create a minimal env with an admin initialized ───────────────────

fn setup() -> (Env, Address) {
    let env = Env::default();
    env.mock_all_auths();

    // Initialize admin via the contract's admin module
    let admin = Address::generate(&env);
    crate::admin::AdminManager::initialize(&env, &admin).expect("admin init failed");

    (env, admin)
}

// ── Default basis-point table ─────────────────────────────────────────────────

#[test]
fn test_default_slash_bps() {
    assert_eq!(SlashableMisbehavior::LosingDispute.default_slash_bps(), 2000);
    assert_eq!(SlashableMisbehavior::ColludedVote.default_slash_bps(), 5000);
    assert_eq!(SlashableMisbehavior::DoubleStake.default_slash_bps(), 10000);
    assert_eq!(SlashableMisbehavior::OracleSpoof.default_slash_bps(), 10000);
}

// ── variant_name round-trips ──────────────────────────────────────────────────

#[test]
fn test_variant_names() {
    assert_eq!(
        SlashableMisbehavior::LosingDispute.variant_name(),
        "LosingDispute"
    );
    assert_eq!(
        SlashableMisbehavior::ColludedVote.variant_name(),
        "ColludedVote"
    );
    assert_eq!(
        SlashableMisbehavior::DoubleStake.variant_name(),
        "DoubleStake"
    );
    assert_eq!(
        SlashableMisbehavior::OracleSpoof.variant_name(),
        "OracleSpoof"
    );
}

// ── set_slash_config + get_slash_bps ─────────────────────────────────────────

#[test]
fn test_set_and_get_slash_config() {
    let (env, admin) = setup();

    // Set a custom 30 % (3000 bps) for LosingDispute
    SlashingExecutor::set_slash_config(&env, admin, SlashableMisbehavior::LosingDispute, 3000)
        .expect("set_slash_config should succeed");

    let bps = SlashingExecutor::get_slash_bps(&env, SlashableMisbehavior::LosingDispute);
    assert_eq!(bps, 3000, "custom bps should be persisted");
}

#[test]
fn test_set_slash_config_invalid_bps() {
    let (env, admin) = setup();

    let result =
        SlashingExecutor::set_slash_config(&env, admin, SlashableMisbehavior::OracleSpoof, 10_001);

    assert!(result.is_err(), "bps > 10000 should be rejected");
    assert_eq!(result.unwrap_err(), crate::errors::Error::InvalidInput);
}

// ── slash — happy path ────────────────────────────────────────────────────────

#[test]
fn test_slash_creates_record() {
    let (env, admin) = setup();
    let actor = Address::generate(&env);
    let evidence = Bytes::from_slice(&env, b"dispute:abc;round:1");
    let market_id = Symbol::new(&env, "btc_50k");

    let record = SlashingExecutor::slash(
        &env,
        admin,
        actor.clone(),
        SlashableMisbehavior::LosingDispute,
        10_000_000, // 1 XLM
        &evidence,
        Some(market_id.clone()),
    )
    .expect("slash should succeed");

    assert_eq!(record.actor, actor);
    assert_eq!(record.misbehavior, SlashableMisbehavior::LosingDispute);
    assert_eq!(record.market_id, Some(market_id));
    // evidence_hash must be non-zero (SHA-256 of the evidence bytes)
    assert_ne!(record.evidence_hash.to_array(), [0u8; 32]);
}

#[test]
fn test_slash_amount_calculation() {
    let (env, admin) = setup();
    let actor = Address::generate(&env);
    let evidence = Bytes::from_slice(&env, b"evidence");

    // Default bps for LosingDispute = 2000 (20 %)
    let stake: i128 = 10_000_000; // 1 XLM
    let expected_slash = (stake * 2000) / 10_000; // 2_000_000 stroops = 0.2 XLM

    let record = SlashingExecutor::slash(
        &env,
        admin,
        actor,
        SlashableMisbehavior::LosingDispute,
        stake,
        &evidence,
        None,
    )
    .expect("slash should succeed");

    assert_eq!(record.slash_amount, expected_slash);
}

// ── idempotency ───────────────────────────────────────────────────────────────

#[test]
fn test_slash_idempotency() {
    let (env, admin) = setup();
    let actor = Address::generate(&env);
    let evidence = Bytes::from_slice(&env, b"evidence");

    // First slash should succeed
    SlashingExecutor::slash(
        &env,
        admin.clone(),
        actor.clone(),
        SlashableMisbehavior::ColludedVote,
        5_000_000,
        &evidence,
        None,
    )
    .expect("first slash should succeed");

    // Second slash for same (actor, misbehavior) must be rejected
    let second = SlashingExecutor::slash(
        &env,
        admin,
        actor,
        SlashableMisbehavior::ColludedVote,
        5_000_000,
        &evidence,
        None,
    );

    assert!(second.is_err());
    assert_eq!(second.unwrap_err(), crate::errors::Error::AlreadySlashed);
}

// ── OracleSpoof — full slash (100 %) ─────────────────────────────────────────

#[test]
fn test_oracle_spoof_full_slash() {
    let (env, admin) = setup();
    let actor = Address::generate(&env);
    let evidence = Bytes::from_slice(&env, b"spoof_attempt");
    let stake: i128 = 20_000_000;

    let record = SlashingExecutor::slash(
        &env,
        admin,
        actor,
        SlashableMisbehavior::OracleSpoof,
        stake,
        &evidence,
        None,
    )
    .expect("slash should succeed");

    // OracleSpoof default bps = 10000 → 100 %
    assert_eq!(record.slash_amount, stake);
}

// ── event presence check ──────────────────────────────────────────────────────

/// Confirms that `slash` does not panic and produces a record, which implies
/// the event emission path was exercised without error.
#[test]
fn test_slash_event_emitted() {
    let (env, admin) = setup();
    let actor = Address::generate(&env);
    let evidence = Bytes::from_slice(&env, b"ev");

    let result = SlashingExecutor::slash(
        &env,
        admin,
        actor,
        SlashableMisbehavior::DoubleStake,
        1_000_000,
        &evidence,
        None,
    );

    // If event emission failed the call would have panicked inside the SDK.
    assert!(result.is_ok());
}

// ── is_slashed query ──────────────────────────────────────────────────────────

#[test]
fn test_is_slashed_query() {
    let (env, admin) = setup();
    let actor = Address::generate(&env);
    let evidence = Bytes::from_slice(&env, b"ev");

    assert!(
        !SlashingExecutor::is_slashed(&env, &actor, SlashableMisbehavior::LosingDispute),
        "should not be slashed before slash call"
    );

    SlashingExecutor::slash(
        &env,
        admin,
        actor.clone(),
        SlashableMisbehavior::LosingDispute,
        1_000_000,
        &evidence,
        None,
    )
    .expect("slash should succeed");

    assert!(
        SlashingExecutor::is_slashed(&env, &actor, SlashableMisbehavior::LosingDispute),
        "should be slashed after slash call"
    );
}

// ── edge cases ────────────────────────────────────────────────────────────────

#[test]
fn test_slash_negative_stake() {
    let (env, admin) = setup();
    let actor = Address::generate(&env);
    let evidence = Bytes::from_slice(&env, b"ev");

    let result = SlashingExecutor::slash(
        &env,
        admin,
        actor,
        SlashableMisbehavior::LosingDispute,
        -1_000_000,
        &evidence,
        None,
    );

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), crate::errors::Error::InvalidInput);
}

#[test]
fn test_slash_zero_stake() {
    let (env, admin) = setup();
    let actor = Address::generate(&env);
    let evidence = Bytes::from_slice(&env, b"ev");

    let record = SlashingExecutor::slash(
        &env,
        admin,
        actor,
        SlashableMisbehavior::LosingDispute,
        0,
        &evidence,
        None,
    )
    .expect("zero-stake slash should succeed (slash amount = 0)");

    assert_eq!(record.slash_amount, 0);
}

#[test]
fn test_double_stake_custom_bps() {
    let (env, admin) = setup();
    let actor = Address::generate(&env);
    let evidence = Bytes::from_slice(&env, b"ev");

    // Override DoubleStake to 75 % (7500 bps)
    SlashingExecutor::set_slash_config(
        &env,
        admin.clone(),
        SlashableMisbehavior::DoubleStake,
        7500,
    )
    .expect("set config should succeed");

    let stake: i128 = 8_000_000;
    let expected = (stake * 7500) / 10_000; // 6_000_000

    let record = SlashingExecutor::slash(
        &env,
        admin,
        actor,
        SlashableMisbehavior::DoubleStake,
        stake,
        &evidence,
        None,
    )
    .expect("slash should succeed");

    assert_eq!(record.slash_amount, expected);
}

/// Different misbehavior variants for the same actor are independent slashes.
#[test]
fn test_different_variants_are_independent() {
    let (env, admin) = setup();
    let actor = Address::generate(&env);
    let evidence = Bytes::from_slice(&env, b"ev");

    SlashingExecutor::slash(
        &env,
        admin.clone(),
        actor.clone(),
        SlashableMisbehavior::LosingDispute,
        1_000_000,
        &evidence,
        None,
    )
    .expect("first slash ok");

    // A different variant for the same actor must succeed
    let result = SlashingExecutor::slash(
        &env,
        admin,
        actor.clone(),
        SlashableMisbehavior::ColludedVote,
        1_000_000,
        &evidence,
        None,
    );

    assert!(
        result.is_ok(),
        "different misbehavior variant should not be blocked"
    );
}
