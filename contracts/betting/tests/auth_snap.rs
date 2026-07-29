//! # Per-entrypoint auth snapshot tests for the Betting event emitter.
//!
//! ## Design rationale
//!
//! The `betting` crate is a **pure event-emission library**: none of its five
//! public emitter functions call `require_auth`.  They are intentionally
//! open — the auth gate lives upstream in the calling contract
//! (`predictify_hybrid`), not in the emitter.
//!
//! These tests encode that contract explicitly so any accidental addition of
//! an auth gate in a future refactor is caught immediately.
//!
//! ## Per-emitter coverage
//!
//! Each of the five emitters gets three test cases:
//!
//! 1. **No-auth succeeds** — a fresh `Env` with *no* `mock_all_auths()` is
//!    used and the emit call must not panic.  Proves the gate is absent.
//!
//! 2. **Auth snapshot is empty** — `env.auths()` returns an empty slice after
//!    the call.  Proves `require_auth` was never invoked.
//!
//! 3. **Topic/schema snapshot** — the published event's topic tuple and
//!    schema version are frozen to the values declared in `events.rs`.
//!    Changing a topic symbol or version without updating this file is a
//!    deliberate, audited ABI break.
//!
//! ## Emitter matrix
//!
//! | Emitter                  | Topic constant        | Auth required |
//! |--------------------------|-----------------------|---------------|
//! | `emit_bet_created`       | `TOPIC_BET_CREATED`   | none          |
//! | `emit_bet_batch_created` | `TOPIC_BET_BATCH_CREATED` | none      |
//! | `emit_bet_status_changed`| `TOPIC_BET_STATUS_CHANGED` | none     |
//! | `emit_bet_claimed`       | `TOPIC_BET_CLAIMED`   | none          |
//! | `emit_bet_stats_updated` | `TOPIC_BET_STATS_UPDATED` | none      |

#![cfg(test)]

extern crate alloc;

use betting::events::{
    BetBatchCreatedEvent, BetClaimedEvent, BetCreatedEvent, BetStatsUpdatedEvent,
    BetStatusChangedEvent, BettingEventEmitter, BETTING_EVENT_SCHEMA_VERSION,
    STATUS_ACTIVE, STATUS_WON, TOPIC_BET_BATCH_CREATED, TOPIC_BET_CLAIMED,
    TOPIC_BET_CREATED, TOPIC_BET_STATS_UPDATED, TOPIC_BET_STATUS_CHANGED,
};
use soroban_sdk::{testutils::Address as _, vec, Address, Env, String, Symbol, Vec};

// ---------------------------------------------------------------------------
// Shared fixtures
// ---------------------------------------------------------------------------

/// Return a minimal `Env` with **no** auth mocking active.
/// Used to verify emitters succeed without any auth gate.
fn bare_env() -> Env {
    Env::default()
}

/// Return an `Env` with `mock_all_auths()` active.
/// Used to verify the auth snapshot is empty after each emit.
fn authed_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env
}

fn market_id(env: &Env) -> Symbol {
    Symbol::new(env, "mkt_snap")
}

fn bettor(env: &Env) -> Address {
    Address::generate(env)
}

fn outcome(env: &Env) -> String {
    String::from_str(env, "yes")
}

// ===========================================================================
// §1  emit_bet_created
// ===========================================================================

/// `emit_bet_created` must succeed without any authorization being present.
/// A panic here means an unexpected `require_auth` was added to the emitter.
#[test]
fn test_emit_bet_created_no_auth_required() {
    let env = bare_env();
    // No mock_all_auths — must not panic.
    BettingEventEmitter::emit_bet_created(
        &env,
        &market_id(&env),
        &bettor(&env),
        &outcome(&env),
        1_000_000,
        99_999,
    );
}

/// After `emit_bet_created` the auth snapshot must be empty: the emitter
/// never calls `require_auth` on any address.
#[test]
fn test_emit_bet_created_auth_snapshot_is_empty() {
    let env = authed_env();
    BettingEventEmitter::emit_bet_created(
        &env,
        &market_id(&env),
        &bettor(&env),
        &outcome(&env),
        1_000_000,
        99_999,
    );
    assert!(
        env.auths().is_empty(),
        "emit_bet_created must not consume any authorization"
    );
}

/// The published topic tuple must be `(TOPIC_BET_CREATED, market_id, 1)` and
/// the schema version inside the payload must equal `BETTING_EVENT_SCHEMA_VERSION`.
#[test]
fn test_emit_bet_created_topic_and_schema_snapshot() {
    let env = authed_env();
    let mid = market_id(&env);
    let user = bettor(&env);

    BettingEventEmitter::emit_bet_created(&env, &mid, &user, &outcome(&env), 1_500_000, 88_000);

    let events = env.events().all();
    assert_eq!(events.len(), 1, "exactly one event must be published");

    let (_, topics, payload) = events.get(0).unwrap();
    assert_eq!(
        topics.len(),
        3,
        "topic tuple must have exactly 3 elements: (topic, market_id, schema_version)"
    );
    assert_eq!(
        topics.get(0).unwrap(),
        TOPIC_BET_CREATED,
        "first topic element must be TOPIC_BET_CREATED"
    );
    assert_eq!(
        topics.get(1).unwrap(),
        mid.clone(),
        "second topic element must be the market_id"
    );
    assert_eq!(
        topics.get(2).unwrap(),
        BETTING_EVENT_SCHEMA_VERSION,
        "third topic element must be the schema version"
    );

    let event: BetCreatedEvent = payload.try_into_val().unwrap();
    assert_eq!(event.market_id, mid);
    assert_eq!(event.bettor, user);
    assert_eq!(event.amount, 1_500_000);
    assert_eq!(event.market_end_time, 88_000);
    assert_eq!(event.nonce, 1, "first emit must produce nonce=1");
}

// ===========================================================================
// §2  emit_bet_batch_created
// ===========================================================================

/// `emit_bet_batch_created` must succeed without any authorization present.
#[test]
fn test_emit_bet_batch_created_no_auth_required() {
    let env = bare_env();
    let user = bettor(&env);
    let market_ids: Vec<Symbol> = vec![&env, market_id(&env)];
    // No mock_all_auths — must not panic.
    BettingEventEmitter::emit_bet_batch_created(&env, &user, &market_ids, 2_000_000);
}

/// The auth snapshot must be empty after `emit_bet_batch_created`.
#[test]
fn test_emit_bet_batch_created_auth_snapshot_is_empty() {
    let env = authed_env();
    let user = bettor(&env);
    let market_ids: Vec<Symbol> = vec![&env, market_id(&env)];

    BettingEventEmitter::emit_bet_batch_created(&env, &user, &market_ids, 2_000_000);

    assert!(
        env.auths().is_empty(),
        "emit_bet_batch_created must not consume any authorization"
    );
}

/// The published topic tuple must be `(TOPIC_BET_BATCH_CREATED, bettor, 1)`
/// and the payload must reflect the bettor, bet_count, and total_amount.
#[test]
fn test_emit_bet_batch_created_topic_and_schema_snapshot() {
    let env = authed_env();
    let user = bettor(&env);
    let mkt_a = Symbol::new(&env, "mkt_a");
    let mkt_b = Symbol::new(&env, "mkt_b");
    let market_ids: Vec<Symbol> = vec![&env, mkt_a.clone(), mkt_b.clone()];
    let total: i128 = 5_000_000;

    BettingEventEmitter::emit_bet_batch_created(&env, &user, &market_ids, total);

    let events = env.events().all();
    assert_eq!(events.len(), 1);

    let (_, topics, payload) = events.get(0).unwrap();
    assert_eq!(topics.len(), 3);
    assert_eq!(
        topics.get(0).unwrap(),
        TOPIC_BET_BATCH_CREATED,
        "first topic element must be TOPIC_BET_BATCH_CREATED"
    );
    assert_eq!(
        topics.get(1).unwrap(),
        user.clone(),
        "second topic element must be the bettor address"
    );
    assert_eq!(
        topics.get(2).unwrap(),
        BETTING_EVENT_SCHEMA_VERSION,
        "third topic element must be the schema version"
    );

    let event: BetBatchCreatedEvent = payload.try_into_val().unwrap();
    assert_eq!(event.bettor, user);
    assert_eq!(event.bet_count, 2);
    assert_eq!(event.total_amount, total);
    assert_eq!(event.market_ids.get(0).unwrap(), mkt_a);
    assert_eq!(event.market_ids.get(1).unwrap(), mkt_b);
    assert_eq!(event.nonce, 1);
}

// ===========================================================================
// §3  emit_bet_status_changed
// ===========================================================================

/// `emit_bet_status_changed` must succeed without any authorization present.
#[test]
fn test_emit_bet_status_changed_no_auth_required() {
    let env = bare_env();
    // No mock_all_auths — must not panic.
    BettingEventEmitter::emit_bet_status_changed(
        &env,
        &market_id(&env),
        &bettor(&env),
        STATUS_ACTIVE,
        STATUS_WON,
        Some(2_000_000),
    );
}

/// The auth snapshot must be empty after `emit_bet_status_changed`.
#[test]
fn test_emit_bet_status_changed_auth_snapshot_is_empty() {
    let env = authed_env();

    BettingEventEmitter::emit_bet_status_changed(
        &env,
        &market_id(&env),
        &bettor(&env),
        STATUS_ACTIVE,
        STATUS_WON,
        Some(2_000_000),
    );

    assert!(
        env.auths().is_empty(),
        "emit_bet_status_changed must not consume any authorization"
    );
}

/// The published topic tuple must be `(TOPIC_BET_STATUS_CHANGED, market_id, 1)`
/// and the payload must carry `old_status`, `new_status`, and `payout_amount`.
#[test]
fn test_emit_bet_status_changed_topic_and_schema_snapshot() {
    let env = authed_env();
    let mid = market_id(&env);
    let user = bettor(&env);

    BettingEventEmitter::emit_bet_status_changed(
        &env,
        &mid,
        &user,
        STATUS_ACTIVE,
        STATUS_WON,
        Some(3_000_000),
    );

    let events = env.events().all();
    assert_eq!(events.len(), 1);

    let (_, topics, payload) = events.get(0).unwrap();
    assert_eq!(topics.len(), 3);
    assert_eq!(
        topics.get(0).unwrap(),
        TOPIC_BET_STATUS_CHANGED,
        "first topic element must be TOPIC_BET_STATUS_CHANGED"
    );
    assert_eq!(
        topics.get(1).unwrap(),
        mid.clone(),
        "second topic element must be the market_id"
    );
    assert_eq!(
        topics.get(2).unwrap(),
        BETTING_EVENT_SCHEMA_VERSION,
        "third topic element must be the schema version"
    );

    let event: BetStatusChangedEvent = payload.try_into_val().unwrap();
    assert_eq!(event.market_id, mid);
    assert_eq!(event.bettor, user);
    assert_eq!(event.old_status, String::from_str(&env, STATUS_ACTIVE));
    assert_eq!(event.new_status, String::from_str(&env, STATUS_WON));
    assert_eq!(event.payout_amount, Some(3_000_000));
    assert_eq!(event.nonce, 1);
}

/// A `None` payout must also be snapshotted correctly (covers the Lost/Cancelled paths).
#[test]
fn test_emit_bet_status_changed_no_payout_snapshot() {
    let env = authed_env();

    BettingEventEmitter::emit_bet_status_changed(
        &env,
        &market_id(&env),
        &bettor(&env),
        STATUS_ACTIVE,
        "Lost",
        None,
    );

    let events = env.events().all();
    let event: BetStatusChangedEvent = events.get(0).unwrap().2.try_into_val().unwrap();
    assert_eq!(event.payout_amount, None, "payout_amount must be None for Lost transition");
    assert!(env.auths().is_empty());
}

// ===========================================================================
// §4  emit_bet_claimed
// ===========================================================================

/// `emit_bet_claimed` must succeed without any authorization present.
#[test]
fn test_emit_bet_claimed_no_auth_required() {
    let env = bare_env();
    // No mock_all_auths — must not panic.
    BettingEventEmitter::emit_bet_claimed(
        &env,
        &market_id(&env),
        &bettor(&env),
        10_000_000,
        200_000,
        9_800_000,
    );
}

/// The auth snapshot must be empty after `emit_bet_claimed`.
#[test]
fn test_emit_bet_claimed_auth_snapshot_is_empty() {
    let env = authed_env();

    BettingEventEmitter::emit_bet_claimed(
        &env,
        &market_id(&env),
        &bettor(&env),
        10_000_000,
        200_000,
        9_800_000,
    );

    assert!(
        env.auths().is_empty(),
        "emit_bet_claimed must not consume any authorization"
    );
}

/// The published topic tuple must be `(TOPIC_BET_CLAIMED, user, 1)` and the
/// payload must carry `gross_payout`, `fee_paid`, and `net_payout`.
#[test]
fn test_emit_bet_claimed_topic_and_schema_snapshot() {
    let env = authed_env();
    let mid = market_id(&env);
    let user = bettor(&env);
    let gross: i128 = 10_000_000;
    let fee: i128 = 200_000;
    let net: i128 = 9_800_000;

    BettingEventEmitter::emit_bet_claimed(&env, &mid, &user, gross, fee, net);

    let events = env.events().all();
    assert_eq!(events.len(), 1);

    let (_, topics, payload) = events.get(0).unwrap();
    assert_eq!(topics.len(), 3);
    assert_eq!(
        topics.get(0).unwrap(),
        TOPIC_BET_CLAIMED,
        "first topic element must be TOPIC_BET_CLAIMED"
    );
    assert_eq!(
        topics.get(1).unwrap(),
        user.clone(),
        "second topic element must be the claiming user address"
    );
    assert_eq!(
        topics.get(2).unwrap(),
        BETTING_EVENT_SCHEMA_VERSION,
        "third topic element must be the schema version"
    );

    let event: BetClaimedEvent = payload.try_into_val().unwrap();
    assert_eq!(event.market_id, mid);
    assert_eq!(event.user, user);
    assert_eq!(event.gross_payout, gross);
    assert_eq!(event.fee_paid, fee);
    assert_eq!(event.net_payout, net);
    assert_eq!(event.nonce, 1);
}

// ===========================================================================
// §5  emit_bet_stats_updated
// ===========================================================================

/// `emit_bet_stats_updated` must succeed without any authorization present.
#[test]
fn test_emit_bet_stats_updated_no_auth_required() {
    let env = bare_env();
    // No mock_all_auths — must not panic.
    BettingEventEmitter::emit_bet_stats_updated(&env, &market_id(&env), 42, 100_000_000, 17);
}

/// The auth snapshot must be empty after `emit_bet_stats_updated`.
#[test]
fn test_emit_bet_stats_updated_auth_snapshot_is_empty() {
    let env = authed_env();

    BettingEventEmitter::emit_bet_stats_updated(&env, &market_id(&env), 42, 100_000_000, 17);

    assert!(
        env.auths().is_empty(),
        "emit_bet_stats_updated must not consume any authorization"
    );
}

/// The published topic tuple must be `(TOPIC_BET_STATS_UPDATED, market_id, 1)`
/// and the payload must carry `total_bets`, `total_amount_locked`, and `unique_bettors`.
#[test]
fn test_emit_bet_stats_updated_topic_and_schema_snapshot() {
    let env = authed_env();
    let mid = market_id(&env);
    let total_bets: u64 = 42;
    let total_locked: i128 = 100_000_000;
    let unique: u32 = 17;

    BettingEventEmitter::emit_bet_stats_updated(&env, &mid, total_bets, total_locked, unique);

    let events = env.events().all();
    assert_eq!(events.len(), 1);

    let (_, topics, payload) = events.get(0).unwrap();
    assert_eq!(topics.len(), 3);
    assert_eq!(
        topics.get(0).unwrap(),
        TOPIC_BET_STATS_UPDATED,
        "first topic element must be TOPIC_BET_STATS_UPDATED"
    );
    assert_eq!(
        topics.get(1).unwrap(),
        mid.clone(),
        "second topic element must be the market_id"
    );
    assert_eq!(
        topics.get(2).unwrap(),
        BETTING_EVENT_SCHEMA_VERSION,
        "third topic element must be the schema version"
    );

    let event: BetStatsUpdatedEvent = payload.try_into_val().unwrap();
    assert_eq!(event.market_id, mid);
    assert_eq!(event.total_bets, total_bets);
    assert_eq!(event.total_amount_locked, total_locked);
    assert_eq!(event.unique_bettors, unique);
    assert_eq!(event.nonce, 1);
}

// ===========================================================================
// §6  Cross-emitter: all five in one env — all auth snapshots stay empty
// ===========================================================================

/// Fires all five emitters in a single `Env`.  After every call `env.auths()`
/// is re-checked: if any emitter ever invokes `require_auth` the assertion
/// will fail on that emitter and the test name will identify it clearly.
#[test]
fn test_all_emitters_produce_empty_auth_snapshots() {
    let env = authed_env();
    let mid = market_id(&env);
    let user = bettor(&env);
    let market_ids: Vec<Symbol> = vec![&env, mid.clone()];

    BettingEventEmitter::emit_bet_created(
        &env, &mid, &user, &outcome(&env), 1_000_000, 0,
    );
    assert!(env.auths().is_empty(), "emit_bet_created must not consume auth");

    BettingEventEmitter::emit_bet_batch_created(&env, &user, &market_ids, 1_000_000);
    assert!(env.auths().is_empty(), "emit_bet_batch_created must not consume auth");

    BettingEventEmitter::emit_bet_status_changed(
        &env, &mid, &user, STATUS_ACTIVE, STATUS_WON, Some(500_000),
    );
    assert!(env.auths().is_empty(), "emit_bet_status_changed must not consume auth");

    BettingEventEmitter::emit_bet_claimed(&env, &mid, &user, 500_000, 10_000, 490_000);
    assert!(env.auths().is_empty(), "emit_bet_claimed must not consume auth");

    BettingEventEmitter::emit_bet_stats_updated(&env, &mid, 1, 1_000_000, 1);
    assert!(env.auths().is_empty(), "emit_bet_stats_updated must not consume auth");

    // Confirm exactly five events were published (one per emitter).
    assert_eq!(env.events().all().len(), 5, "one event per emitter must be published");
}

// ===========================================================================
// §7  Schema version is frozen at v1 for all topics
// ===========================================================================

/// The schema version embedded in every topic tuple must be exactly `1`.
/// This is the ABI stability guarantee: any bump must be intentional and
/// audited.
#[test]
fn test_schema_version_frozen_at_v1_across_all_emitters() {
    let env = authed_env();
    let mid = market_id(&env);
    let user = bettor(&env);
    let market_ids: Vec<Symbol> = vec![&env, mid.clone()];

    BettingEventEmitter::emit_bet_created(&env, &mid, &user, &outcome(&env), 1, 0);
    BettingEventEmitter::emit_bet_batch_created(&env, &user, &market_ids, 1);
    BettingEventEmitter::emit_bet_status_changed(&env, &mid, &user, STATUS_ACTIVE, STATUS_WON, None);
    BettingEventEmitter::emit_bet_claimed(&env, &mid, &user, 1, 0, 1);
    BettingEventEmitter::emit_bet_stats_updated(&env, &mid, 1, 1, 1);

    for (i, ev) in env.events().all().iter().enumerate() {
        let (_, topics, _) = ev;
        let version: u32 = topics.get(2).unwrap();
        assert_eq!(
            version, 1u32,
            "event #{i}: schema_version must be frozen at 1, got {version}"
        );
    }
}
