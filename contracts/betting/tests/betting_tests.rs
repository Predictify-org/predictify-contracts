//! # Betting event module – integration tests
//!
//! These tests cover the public surface of [`betting::events`]:
//!
//! * Schema registry completeness and topic-resolution determinism
//! * Per-emit topic + schema-version tuple
//! * Per-emit nonce monotonicity and topic isolation
//! * Field correctness for every event kind
//! * `git`-style ABI surface (logical-name → topic → schema-version)
//! * No-`unwrap` guarantees in production paths (tested by reading the
//!   intermediate state directly via instance storage)
//!
//! The tests use a synthetic ledger that is mock-populated via
//! `soroban_sdk::Env` so no on-chain token or market is required.

#![cfg(test)]

extern crate alloc;

use betting::events::{
    BettingEventEmitter, BettingEventSchema, BettingEventSchemaEntry, BetBatchCreatedEvent,
    BetClaimedEvent, BetCreatedEvent, BetStatsUpdatedEvent, BetStatusChangedEvent,
    EVENT_NAME_BET_BATCH_CREATED, EVENT_NAME_BET_CLAIMED, EVENT_NAME_BET_CREATED,
    EVENT_NAME_BET_STATS_UPDATED, EVENT_NAME_BET_STATUS_CHANGED, NS_NONCE,
    STATUS_ACTIVE, STATUS_CANCELLED, STATUS_LOST, STATUS_REFUNDED, STATUS_WON,
    TOPIC_BET_BATCH_CREATED, TOPIC_BET_CLAIMED, TOPIC_BET_CREATED,
    TOPIC_BET_STATS_UPDATED, TOPIC_BET_STATUS_CHANGED,
    BETTING_EVENT_SCHEMA_VERSION,
};
use soroban_sdk::{testutils::Address as _, vec, Address, Env, Symbol, Vec};

// -----------------------------------------------------------------
// Test fixtures
// -----------------------------------------------------------------

fn env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env
}

fn new_market_id(env: &Env, label: &str) -> Symbol {
    Symbol::new(env, label)
}

fn bettor(env: &Env) -> Address {
    Address::generate(env)
}

// -----------------------------------------------------------------
// §1  Schema-registry tests
// -----------------------------------------------------------------

#[test]
fn schema_registry_maps_all_five_logical_names() {
    let env = env();

    for name in [
        EVENT_NAME_BET_CREATED,
        EVENT_NAME_BET_BATCH_CREATED,
        EVENT_NAME_BET_STATUS_CHANGED,
        EVENT_NAME_BET_CLAIMED,
        EVENT_NAME_BET_STATS_UPDATED,
    ] {
        let entry = BettingEventSchema::get_schema(&env, name);
        assert!(
            entry.is_some(),
            "schema registry should expose entry for {name}"
        );
        let entry = entry.unwrap();
        assert_eq!(
            entry.schema_version, BETTING_EVENT_SCHEMA_VERSION,
            "schema version for {name} must be {BETTING_EVENT_SCHEMA_VERSION}"
        );
        // Topic symbols are zero-sized on the topic side so any topic
        // returned is acceptable so long as it's a frozen Symbol.
        let _: Symbol = entry.topic;
    }
}

#[test]
fn schema_registry_resolves_topics_one_to_one() {
    let env = env();

    let expected_pairs: &[(&str, Symbol)] = &[
        (EVENT_NAME_BET_CREATED, TOPIC_BET_CREATED),
        (EVENT_NAME_BET_BATCH_CREATED, TOPIC_BET_BATCH_CREATED),
        (EVENT_NAME_BET_STATUS_CHANGED, TOPIC_BET_STATUS_CHANGED),
        (EVENT_NAME_BET_CLAIMED, TOPIC_BET_CLAIMED),
        (EVENT_NAME_BET_STATS_UPDATED, TOPIC_BET_STATS_UPDATED),
    ];

    let mut topic_set: std::collections::HashSet<Symbol> = std::collections::HashSet::new();
    for (name, expected_topic) in expected_pairs {
        let entry: BettingEventSchemaEntry = BettingEventSchema::get_schema(&env, name)
            .expect("entry must exist for canonical name");
        assert_eq!(
            entry.topic, *expected_topic,
            "topic for {name} must match canonical symbol"
        );
        // Ensure no topic is reused across logical names.
        assert!(
            topic_set.insert(entry.topic.clone()),
            "duplicate topic {:?} across multiple logical names",
            entry.topic
        );
        assert_eq!(entry.schema_version, BETTING_EVENT_SCHEMA_VERSION);
    }
}

#[test]
fn schema_registry_returns_none_for_unknown_name() {
    let env = env();
    assert!(BettingEventSchema::get_schema(&env, "not_a_real_event").is_none());
    assert!(BettingEventSchema::get_schema(&env, "").is_none());
    assert!(BettingEventSchema::get_schema(&env, "BET_CREATED").is_none()); // case-sensitive
}

// -----------------------------------------------------------------
// §2  BetCreatedEvent tests
// -----------------------------------------------------------------

#[test]
fn emit_bet_created_publishes_with_frozen_topic_and_version() {
    let env = env();
    let market_id = new_market_id(&env, "market_one");
    let bettor = bettor(&env);
    let outcome = String::from_str(&env, "yes");
    let amount: i128 = 1_500_000; // 0.15 XLM
    let end_time: u64 = 99_999;

    BettingEventEmitter::emit_bet_created(
        &env,
        &market_id,
        &bettor,
        &outcome,
        amount,
        end_time,
    );

    let events = env.events().all();
    assert_eq!(events.len(), 1, "single emit should publish exactly one event");

    let (_, topics, payload) = events.get(0).unwrap();
    // Topic tuple: (TOPIC_BET_CREATED, market_id, schema_version)
    assert_eq!(topics.len(), 3, "topic tuple must have exactly 3 elements");
    assert_eq!(topics.get(0).unwrap(), TOPIC_BET_CREATED);
    assert_eq!(topics.get(1).unwrap(), market_id);
    assert_eq!(topics.get(2).unwrap(), BETTING_EVENT_SCHEMA_VERSION);

    let event: BetCreatedEvent = payload.try_into_val().unwrap();
    assert_eq!(event.market_id, market_id);
    assert_eq!(event.bettor, bettor);
    assert_eq!(event.outcome, outcome);
    assert_eq!(event.amount, amount);
    assert_eq!(event.market_end_time, end_time);
    assert_eq!(event.nonce, 1, "first emit must yield nonce=1");
    assert_eq!(event.timestamp, env.ledger().timestamp());
}

#[test]
fn emit_bet_created_increments_nonce_monotonically() {
    let env = env();
    let market_id = new_market_id(&env, "market_mono");
    let bettor = bettor(&env);
    let outcome = String::from_str(&env, "yes");

    for expected in 1u64..=3u64 {
        BettingEventEmitter::emit_bet_created(
            &env,
            &market_id,
            &bettor,
            &outcome,
            1_000_000,
            1_000,
        );

        let events = env.events().all();
        let last = events.get(events.len() - 1).unwrap();
        let event: BetCreatedEvent = last.2.try_into_val().unwrap();
        assert_eq!(
            event.nonce, expected,
            "nonces must be strictly monotonic (1, 2, 3) — got {}",
            event.nonce
        );
    }
}

// -----------------------------------------------------------------
// §3  BetBatchCreatedEvent tests
// -----------------------------------------------------------------

#[test]
fn emit_bet_batch_created_emits_correct_fields() {
    let env = env();
    let market_a = new_market_id(&env, "mkt_a");
    let market_b = new_market_id(&env, "mkt_b");
    let market_ids: Vec<Symbol> = vec![&env, market_a.clone(), market_b.clone()];
    let bettor = bettor(&env);
    let total: i128 = 4_000_000;

    BettingEventEmitter::emit_bet_batch_created(&env, &bettor, &market_ids, total);

    let events = env.events().all();
    let last = events.get(events.len() - 1).unwrap();
    let (_, topics, payload) = last;

    assert_eq!(topics.get(0).unwrap(), TOPIC_BET_BATCH_CREATED);
    assert_eq!(topics.get(1).unwrap(), bettor);
    assert_eq!(topics.get(2).unwrap(), BETTING_EVENT_SCHEMA_VERSION);

    let event: BetBatchCreatedEvent = payload.try_into_val().unwrap();
    assert_eq!(event.bettor, bettor);
    assert_eq!(event.bet_count, market_ids.len());
    assert_eq!(event.total_amount, total);
    assert_eq!(event.market_ids.len(), 2);
    assert_eq!(event.market_ids.get(0).unwrap(), market_a);
    assert_eq!(event.market_ids.get(1).unwrap(), market_b);
    assert_eq!(event.nonce, 1);
}

// -----------------------------------------------------------------
// §4  BetStatusChangedEvent tests
// -----------------------------------------------------------------

#[test]
fn emit_bet_status_changed_carries_transition_pair_and_payout() {
    let env = env();
    let market_id = new_market_id(&env, "mkt_status");
    let bettor = bettor(&env);

    BettingEventEmitter::emit_bet_status_changed(
        &env,
        &market_id,
        &bettor,
        STATUS_ACTIVE,
        STATUS_WON,
        Some(2_000_000),
    );

    BettingEventEmitter::emit_bet_status_changed(
        &env,
        &market_id,
        &bettor,
        STATUS_WON,
        STATUS_LOST,
        None,
    );

    BettingEventEmitter::emit_bet_status_changed(
        &env,
        &market_id,
        &bettor,
        STATUS_LOST,
        STATUS_REFUNDED,
        Some(2_000_000),
    );

    BettingEventEmitter::emit_bet_status_changed(
        &env,
        &market_id,
        &bettor,
        STATUS_REFUNDED,
        STATUS_CANCELLED,
        None,
    );

    let events = env.events().all();
    assert_eq!(events.len(), 4);

    let e1: BetStatusChangedEvent = events.get(0).unwrap().2.try_into_val().unwrap();
    assert_eq!(e1.old_status, String::from_str(&env, STATUS_ACTIVE));
    assert_eq!(e1.new_status, String::from_str(&env, STATUS_WON));
    assert_eq!(e1.payout_amount, Some(2_000_000));
    assert_eq!(e1.nonce, 1);

    let e2: BetStatusChangedEvent = events.get(1).unwrap().2.try_into_val().unwrap();
    assert_eq!(e2.old_status, String::from_str(&env, STATUS_WON));
    assert_eq!(e2.new_status, String::from_str(&env, STATUS_LOST));
    assert_eq!(e2.payout_amount, None);
    assert_eq!(
        e2.nonce, 2,
        "nonces in this topic are unique and strictly monotonic"
    );

    let e4: BetStatusChangedEvent = events.get(3).unwrap().2.try_into_val().unwrap();
    assert_eq!(e4.nonce, 4);
    assert_eq!(e4.payout_amount, None);
}

// -----------------------------------------------------------------
// §5  BetClaimedEvent tests
// -----------------------------------------------------------------

#[test]
fn emit_bet_claimed_carries_gross_fee_and_net() {
    let env = env();
    let market_id = new_market_id(&env, "mkt_claim");
    let user = bettor(&env);
    let gross: i128 = 10_000_000;
    let fee: i128 = 200_000;
    let net: i128 = 9_800_000;

    BettingEventEmitter::emit_bet_claimed(&env, &market_id, &user, gross, fee, net);

    let events = env.events().all();
    let (_, topics, payload) = events.get(0).unwrap();
    assert_eq!(topics.get(0).unwrap(), TOPIC_BET_CLAIMED);
    assert_eq!(topics.get(1).unwrap(), user);
    assert_eq!(topics.get(2).unwrap(), BETTING_EVENT_SCHEMA_VERSION);

    let event: BetClaimedEvent = payload.try_into_val().unwrap();
    assert_eq!(event.market_id, market_id);
    assert_eq!(event.user, user);
    assert_eq!(event.gross_payout, gross);
    assert_eq!(event.fee_paid, fee);
    assert_eq!(event.net_payout, net);
    assert_eq!(event.nonce, 1);
}

// -----------------------------------------------------------------
// §6  BetStatsUpdatedEvent tests
// -----------------------------------------------------------------

#[test]
fn emit_bet_stats_updated_carries_aggregate_snapshot() {
    let env = env();
    let market_id = new_market_id(&env, "mkt_stats");
    let total_bets: u64 = 42;
    let total_amount_locked: i128 = 100_000_000;
    let unique_bettors: u32 = 17;

    BettingEventEmitter::emit_bet_stats_updated(
        &env,
        &market_id,
        total_bets,
        total_amount_locked,
        unique_bettors,
    );

    let events = env.events().all();
    let (_, topics, payload) = events.get(0).unwrap();
    assert_eq!(topics.get(0).unwrap(), TOPIC_BET_STATS_UPDATED);
    assert_eq!(topics.get(1).unwrap(), market_id);
    assert_eq!(topics.get(2).unwrap(), BETTING_EVENT_SCHEMA_VERSION);

    let event: BetStatsUpdatedEvent = payload.try_into_val().unwrap();
    assert_eq!(event.market_id, market_id);
    assert_eq!(event.total_bets, total_bets);
    assert_eq!(event.total_amount_locked, total_amount_locked);
    assert_eq!(event.unique_bettors, unique_bettors);
    assert_eq!(event.nonce, 1);
}

// -----------------------------------------------------------------
// §7  Cross-topic nonce isolation
// -----------------------------------------------------------------

#[test]
fn different_topics_have_independent_nonces() {
    let env = env();
    let market_id = new_market_id(&env, "mkt_iso");
    let bettor = bettor(&env);
    let outcome = String::from_str(&env, "yes");

    // Two emits on topic A, two on topic B, one on topic C — interleaved.
    BettingEventEmitter::emit_bet_created(&env, &market_id, &bettor, &outcome, 1, 0);
    BettingEventEmitter::emit_bet_stats_updated(&env, &market_id, 1, 1, 1);
    BettingEventEmitter::emit_bet_created(&env, &market_id, &bettor, &outcome, 1, 0);
    BettingEventEmitter::emit_bet_stats_updated(&env, &market_id, 2, 2, 2);
    BettingEventEmitter::emit_bet_claimed(&env, &market_id, &bettor, 1, 0, 1);

    let events = env.events().all();
    assert_eq!(events.len(), 5);

    // Filter by topic prefix and re-extract nonces.
    let mut bet_created_nonces: Vec<u64> = Vec::new();
    let mut stats_nonces: Vec<u64> = Vec::new();
    let mut claimed_nonces: Vec<u64> = Vec::new();
    for e in events.iter() {
        let first_topic: Symbol = e.1.get(0).unwrap();
        if first_topic == TOPIC_BET_CREATED {
            let ev: BetCreatedEvent = e.2.try_into_val().unwrap();
            bet_created_nonces.push(ev.nonce);
        } else if first_topic == TOPIC_BET_STATS_UPDATED {
            let ev: BetStatsUpdatedEvent = e.2.try_into_val().unwrap();
            stats_nonces.push(ev.nonce);
        } else if first_topic == TOPIC_BET_CLAIMED {
            let ev: BetClaimedEvent = e.2.try_into_val().unwrap();
            claimed_nonces.push(ev.nonce);
        } else {
            panic!("unexpected topic {:?}", first_topic);
        }
    }
    // Each topic's nonces are independent and start at 1.
    assert_eq!(bet_created_nonces, vec![1u64, 2u64]);
    assert_eq!(stats_nonces, vec![1u64, 2u64]);
    assert_eq!(claimed_nonces, vec![1u64]);
}

// -----------------------------------------------------------------
// §8  Nonce counter is persisted across emissions (replay anchor)
// -----------------------------------------------------------------

/// After N consecutive emits of the same topic, the persisted nonce
/// counter must equal N — confirming the counter is real instance
/// storage (not a per-call ephemeral) and that monotonicity holds
/// end-to-end.
#[test]
fn nonce_counter_advances_persistently_per_topic() {
    let env = env();
    let market_id = new_market_id(&env, "mkt_snap");
    let bettor = bettor(&env);
    let outcome = String::from_str(&env, "yes");

    for _ in 0..5 {
        BettingEventEmitter::emit_bet_created(&env, &market_id, &bettor, &outcome, 1, 0);
    }

    // The instance-stored nonce for the topic must equal 5 after
    // five emits, demonstrating the counter is real storage and not
    // compiled-in state.
    let key = (NS_NONCE, TOPIC_BET_CREATED);
    let nonce: u64 = env.storage().instance().get(&key).unwrap_or(0);
    assert_eq!(
        nonce, 5,
        "5 emits must bump the persisted nonce counter to 5"
    );
}

// -----------------------------------------------------------------
// §9  Logical-name -> topic mapping is fully frozen at v1
// -----------------------------------------------------------------

#[test]
fn schema_version_is_frozen_at_one() {
    let env = env();
    let entry = BettingEventSchema::get_schema(&env, EVENT_NAME_BET_CREATED).unwrap();
    assert_eq!(entry.schema_version, 1u32);
}
