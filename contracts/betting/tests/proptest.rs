//! # Betting event invariant property tests
//!
//! This module uses [`proptest`] to assert that the core betting event
//! invariants hold across **arbitrary valid action sequences**.
//!
//! ## Invariants exercised
//!
//! | # | Invariant | Description |
//! |---|-----------|-------------|
//! | 1 | Nonce monotonicity | Per-topic nonce strictly increases with each emit |
//! | 2 | Nonce isolation | Different topics maintain independent nonce counters |
//! | 3 | Topic stability | Every emit publishes its frozen `symbol_short!` topic |
//! | 4 | Schema version stability | Every emit carries `BETTING_EVENT_SCHEMA_VERSION` |
//! | 5 | Payout arithmetic | `net_payout == gross_payout - fee_paid` always |
//! | 6 | Batch field alignment | `bet_count == market_ids.len()` always |
//! | 7 | Stats non-negativity | `total_amount_locked` and `total_bets` are non-negative |
//! | 8 | Timestamp consistency | Event timestamp equals `env.ledger().timestamp()` |
//! | 9 | Nonce persistence | Stored nonce counter matches total emits on that topic |
//!
//! ## Running
//!
//! ```bash
//! cargo test -p betting --test proptest -- --nocapture
//! ```

#![cfg(test)]

extern crate alloc;

use betting::events::{
    BetBatchCreatedEvent, BetClaimedEvent, BetCreatedEvent, BetStatsUpdatedEvent,
    BetStatusChangedEvent, BettingEventEmitter, BettingEventSchema, BETTING_EVENT_SCHEMA_VERSION,
    EVENT_NAME_BET_BATCH_CREATED, EVENT_NAME_BET_CLAIMED, EVENT_NAME_BET_CREATED,
    EVENT_NAME_BET_STATS_UPDATED, EVENT_NAME_BET_STATUS_CHANGED, NS_NONCE, STATUS_ACTIVE,
    STATUS_CANCELLED, STATUS_LOST, STATUS_REFUNDED, STATUS_WON, TOPIC_BET_BATCH_CREATED,
    TOPIC_BET_CLAIMED, TOPIC_BET_CREATED, TOPIC_BET_STATS_UPDATED, TOPIC_BET_STATUS_CHANGED,
};
use proptest::{collection::vec as prop_vec, prelude::*};
use soroban_sdk::{testutils::Address as _, vec as sdk_vec, Address, Env, Symbol};

// ─────────────────────────────────────────────────────────────────────────────
// §1  Action model
// ─────────────────────────────────────────────────────────────────────────────

/// One step in an arbitrary betting-event action sequence.
///
/// Every variant maps directly to one of the five public
/// `BettingEventEmitter::emit_*` methods.  Values are kept small so
/// the env stays fast; the focus is on *sequence ordering*, not on
/// boundary values (those are exercised by `betting_tests.rs`).
#[derive(Debug, Clone)]
enum BettingAction {
    /// `emit_bet_created` with (amount, end_time)
    BetCreated { amount: i128, end_time: u64 },
    /// `emit_bet_batch_created` with (total_amount, batch_size 1-5)
    BetBatchCreated { total_amount: i128, batch_size: u32 },
    /// `emit_bet_status_changed` with (old_status_idx, new_status_idx, payout)
    BetStatusChanged {
        old_idx: usize,
        new_idx: usize,
        payout: Option<i128>,
    },
    /// `emit_bet_claimed` with (gross, fee)
    BetClaimed { gross: i128, fee: i128 },
    /// `emit_bet_stats_updated` with (total_bets, total_locked, unique_bettors)
    BetStatsUpdated {
        total_bets: u64,
        total_locked: i128,
        unique_bettors: u32,
    },
}

// ─────────────────────────────────────────────────────────────────────────────
// §2  Proptest strategies
// ─────────────────────────────────────────────────────────────────────────────

/// All valid bet lifecycle status strings in a stable order.
const STATUSES: &[&str] = &[
    STATUS_ACTIVE,
    STATUS_WON,
    STATUS_LOST,
    STATUS_REFUNDED,
    STATUS_CANCELLED,
];

fn action_strategy() -> impl Strategy<Value = BettingAction> {
    prop_oneof![
        // BetCreated — weight 4
        4 => (1_000_000i128..=10_000_000i128, 0u64..=100_000u64)
            .prop_map(|(amount, end_time)| BettingAction::BetCreated { amount, end_time }),

        // BetBatchCreated — weight 2
        2 => (1_000_000i128..=20_000_000i128, 1u32..=5u32)
            .prop_map(|(total_amount, batch_size)| BettingAction::BetBatchCreated {
                total_amount,
                batch_size,
            }),

        // BetStatusChanged — weight 3
        3 => (
            0usize..STATUSES.len(),
            0usize..STATUSES.len(),
            proptest::option::of(1_000_000i128..=10_000_000i128),
        )
            .prop_map(|(old_idx, new_idx, payout)| BettingAction::BetStatusChanged {
                old_idx,
                new_idx,
                payout,
            }),

        // BetClaimed — weight 2
        2 => (1_000_000i128..=20_000_000i128, 0i128..=500_000i128)
            .prop_map(|(gross, fee)| BettingAction::BetClaimed { gross, fee }),

        // BetStatsUpdated — weight 2
        2 => (0u64..=200u64, 0i128..=100_000_000i128, 0u32..=50u32)
            .prop_map(|(total_bets, total_locked, unique_bettors)| {
                BettingAction::BetStatsUpdated {
                    total_bets,
                    total_locked,
                    unique_bettors,
                }
            }),
    ]
}

// ─────────────────────────────────────────────────────────────────────────────
// §3  Execution engine
// ─────────────────────────────────────────────────────────────────────────────

/// Shared state tracked by the test driver in parallel with the contract.
struct Counters {
    bet_created: u64,
    bet_batch_created: u64,
    bet_status_changed: u64,
    bet_claimed: u64,
    bet_stats_updated: u64,
}

impl Counters {
    fn new() -> Self {
        Self {
            bet_created: 0,
            bet_batch_created: 0,
            bet_status_changed: 0,
            bet_claimed: 0,
            bet_stats_updated: 0,
        }
    }
}

/// Execute one action, emit the event, then assert every invariant for that
/// emit before returning.
fn execute_action(
    env: &Env,
    market_id: &Symbol,
    user: &Address,
    action: &BettingAction,
    counters: &mut Counters,
) {
    let events_before = env.events().all().len();
    let ledger_ts = env.ledger().timestamp();

    match action {
        BettingAction::BetCreated { amount, end_time } => {
            let outcome = soroban_sdk::String::from_str(env, "yes");
            BettingEventEmitter::emit_bet_created(
                env, market_id, user, &outcome, *amount, *end_time,
            );
            counters.bet_created += 1;

            // Fetch the newly published event.
            let all = env.events().all();
            assert_eq!(
                all.len(),
                events_before + 1,
                "BetCreated must publish exactly one event"
            );
            let (_, topics, payload) = all.get(all.len() - 1).unwrap();

            // INV-3: topic stability
            assert_eq!(
                topics.get(0).unwrap(),
                TOPIC_BET_CREATED,
                "INV-3 topic stability"
            );
            // INV-4: schema version stability
            assert_eq!(
                topics.get(2).unwrap(),
                BETTING_EVENT_SCHEMA_VERSION,
                "INV-4 schema version"
            );

            let ev: BetCreatedEvent = payload.try_into_val().unwrap();
            // INV-1: nonce monotonicity
            assert_eq!(
                ev.nonce, counters.bet_created,
                "INV-1 nonce monotonicity (BetCreated)"
            );
            // INV-8: timestamp consistency
            assert_eq!(ev.timestamp, ledger_ts, "INV-8 timestamp consistency");
        }

        BettingAction::BetBatchCreated {
            total_amount,
            batch_size,
        } => {
            // Build a market_ids vec of length batch_size (reuse market_id for simplicity).
            let mut mids = sdk_vec![env, market_id.clone()];
            for _ in 1..*batch_size {
                mids.push_back(market_id.clone());
            }
            BettingEventEmitter::emit_bet_batch_created(env, user, &mids, *total_amount);
            counters.bet_batch_created += 1;

            let all = env.events().all();
            assert_eq!(
                all.len(),
                events_before + 1,
                "BetBatchCreated must publish exactly one event"
            );
            let (_, topics, payload) = all.get(all.len() - 1).unwrap();

            // INV-3: topic stability
            assert_eq!(
                topics.get(0).unwrap(),
                TOPIC_BET_BATCH_CREATED,
                "INV-3 topic stability (batch)"
            );
            // INV-4: schema version
            assert_eq!(
                topics.get(2).unwrap(),
                BETTING_EVENT_SCHEMA_VERSION,
                "INV-4 schema version (batch)"
            );

            let ev: BetBatchCreatedEvent = payload.try_into_val().unwrap();
            // INV-1: nonce monotonicity
            assert_eq!(
                ev.nonce, counters.bet_batch_created,
                "INV-1 nonce monotonicity (BetBatchCreated)"
            );
            // INV-6: bet_count == market_ids.len()
            assert_eq!(ev.bet_count, mids.len(), "INV-6 batch field alignment");
            // INV-8: timestamp
            assert_eq!(ev.timestamp, ledger_ts, "INV-8 timestamp (batch)");
        }

        BettingAction::BetStatusChanged {
            old_idx,
            new_idx,
            payout,
        } => {
            let old_status = STATUSES[old_idx % STATUSES.len()];
            let new_status = STATUSES[new_idx % STATUSES.len()];
            BettingEventEmitter::emit_bet_status_changed(
                env, market_id, user, old_status, new_status, *payout,
            );
            counters.bet_status_changed += 1;

            let all = env.events().all();
            assert_eq!(
                all.len(),
                events_before + 1,
                "BetStatusChanged must publish exactly one event"
            );
            let (_, topics, payload) = all.get(all.len() - 1).unwrap();

            // INV-3: topic stability
            assert_eq!(
                topics.get(0).unwrap(),
                TOPIC_BET_STATUS_CHANGED,
                "INV-3 topic stability (status)"
            );
            // INV-4: schema version
            assert_eq!(
                topics.get(2).unwrap(),
                BETTING_EVENT_SCHEMA_VERSION,
                "INV-4 schema version (status)"
            );

            let ev: BetStatusChangedEvent = payload.try_into_val().unwrap();
            // INV-1: nonce monotonicity
            assert_eq!(
                ev.nonce, counters.bet_status_changed,
                "INV-1 nonce monotonicity (BetStatusChanged)"
            );
            // INV-8: timestamp
            assert_eq!(ev.timestamp, ledger_ts, "INV-8 timestamp (status)");
        }

        BettingAction::BetClaimed { gross, fee } => {
            // net must equal gross - fee; clamp fee so net >= 0.
            let clamped_fee = (*fee).min(*gross);
            let net = gross - clamped_fee;
            BettingEventEmitter::emit_bet_claimed(env, market_id, user, *gross, clamped_fee, net);
            counters.bet_claimed += 1;

            let all = env.events().all();
            assert_eq!(
                all.len(),
                events_before + 1,
                "BetClaimed must publish exactly one event"
            );
            let (_, topics, payload) = all.get(all.len() - 1).unwrap();

            // INV-3: topic stability
            assert_eq!(
                topics.get(0).unwrap(),
                TOPIC_BET_CLAIMED,
                "INV-3 topic stability (claimed)"
            );
            // INV-4: schema version
            assert_eq!(
                topics.get(2).unwrap(),
                BETTING_EVENT_SCHEMA_VERSION,
                "INV-4 schema version (claimed)"
            );

            let ev: BetClaimedEvent = payload.try_into_val().unwrap();
            // INV-1: nonce monotonicity
            assert_eq!(
                ev.nonce, counters.bet_claimed,
                "INV-1 nonce monotonicity (BetClaimed)"
            );
            // INV-5: payout arithmetic
            assert_eq!(
                ev.net_payout,
                ev.gross_payout - ev.fee_paid,
                "INV-5 payout arithmetic: net must equal gross - fee"
            );
            // INV-8: timestamp
            assert_eq!(ev.timestamp, ledger_ts, "INV-8 timestamp (claimed)");
        }

        BettingAction::BetStatsUpdated {
            total_bets,
            total_locked,
            unique_bettors,
        } => {
            BettingEventEmitter::emit_bet_stats_updated(
                env,
                market_id,
                *total_bets,
                *total_locked,
                *unique_bettors,
            );
            counters.bet_stats_updated += 1;

            let all = env.events().all();
            assert_eq!(
                all.len(),
                events_before + 1,
                "BetStatsUpdated must publish exactly one event"
            );
            let (_, topics, payload) = all.get(all.len() - 1).unwrap();

            // INV-3: topic stability
            assert_eq!(
                topics.get(0).unwrap(),
                TOPIC_BET_STATS_UPDATED,
                "INV-3 topic stability (stats)"
            );
            // INV-4: schema version
            assert_eq!(
                topics.get(2).unwrap(),
                BETTING_EVENT_SCHEMA_VERSION,
                "INV-4 schema version (stats)"
            );

            let ev: BetStatsUpdatedEvent = payload.try_into_val().unwrap();
            // INV-1: nonce monotonicity
            assert_eq!(
                ev.nonce, counters.bet_stats_updated,
                "INV-1 nonce monotonicity (BetStatsUpdated)"
            );
            // INV-7: stats non-negativity
            assert!(
                ev.total_amount_locked >= 0,
                "INV-7 total_amount_locked must be non-negative"
            );
            // INV-8: timestamp
            assert_eq!(ev.timestamp, ledger_ts, "INV-8 timestamp (stats)");
        }
    }
}

/// After all actions complete, verify persisted nonce counters equal the
/// per-topic emit counts (INV-9: nonce persistence).
fn assert_persisted_nonces(env: &Env, counters: &Counters) {
    let topics_and_counts: &[(soroban_sdk::Symbol, u64)] = &[
        (TOPIC_BET_CREATED, counters.bet_created),
        (TOPIC_BET_BATCH_CREATED, counters.bet_batch_created),
        (TOPIC_BET_STATUS_CHANGED, counters.bet_status_changed),
        (TOPIC_BET_CLAIMED, counters.bet_claimed),
        (TOPIC_BET_STATS_UPDATED, counters.bet_stats_updated),
    ];
    for (topic, expected) in topics_and_counts {
        let key = (NS_NONCE, topic.clone());
        let stored: u64 = env.storage().instance().get(&key).unwrap_or(0);
        assert_eq!(
            stored, *expected,
            "INV-9 persisted nonce for topic {:?} must equal emit count {}",
            topic, expected
        );
    }
}

/// Assert schema-registry invariants after an arbitrary sequence.
/// INV-4 complement: registry always returns `BETTING_EVENT_SCHEMA_VERSION`.
fn assert_schema_registry(env: &Env) {
    for name in [
        EVENT_NAME_BET_CREATED,
        EVENT_NAME_BET_BATCH_CREATED,
        EVENT_NAME_BET_STATUS_CHANGED,
        EVENT_NAME_BET_CLAIMED,
        EVENT_NAME_BET_STATS_UPDATED,
    ] {
        let entry = BettingEventSchema::get_schema(env, name);
        assert!(
            entry.is_some(),
            "schema registry must return an entry for {name}"
        );
        assert_eq!(
            entry.unwrap().schema_version,
            BETTING_EVENT_SCHEMA_VERSION,
            "schema_version must be {BETTING_EVENT_SCHEMA_VERSION} for {name}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// §4  Property tests
// ─────────────────────────────────────────────────────────────────────────────

proptest! {
    /// **Core invariant sweep.**
    ///
    /// Given any sequence of 1-40 arbitrary betting actions, every invariant
    /// listed in the module doc must hold after each individual emit *and*
    /// after the whole sequence completes.
    #[test]
    fn betting_event_invariants_hold_across_arbitrary_sequences(
        actions in prop_vec(action_strategy(), 1..40)
    ) {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().with_mut(|li| {
            li.min_temp_entry_ttl = 1;
            li.min_persistent_entry_ttl = 1;
            li.max_entry_ttl = 6_000_000;
        });

        let market_id = Symbol::new(&env, "mkt_prop");
        let user = Address::generate(&env);
        let mut counters = Counters::new();

        for action in &actions {
            execute_action(&env, &market_id, &user, action, &mut counters);
        }

        // Post-sequence invariants
        assert_persisted_nonces(&env, &counters);
        assert_schema_registry(&env);
    }

    /// **INV-2: Nonce isolation across topics.**
    ///
    /// Interleaving emits from two topics must not corrupt either counter.
    #[test]
    fn nonce_isolation_across_topics(
        created_count in 1u64..=20u64,
        stats_count in 1u64..=20u64
    ) {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().with_mut(|li| {
            li.max_entry_ttl = 6_000_000;
            li.min_persistent_entry_ttl = 1;
            li.min_temp_entry_ttl = 1;
        });

        let market_id = Symbol::new(&env, "mkt_iso");
        let user = Address::generate(&env);
        let outcome = soroban_sdk::String::from_str(&env, "yes");

        // Interleave the two topic emits.
        let total = (created_count + stats_count) as usize;
        let mut cr = 0u64;
        let mut st = 0u64;

        for i in 0..total {
            if i % 2 == 0 && cr < created_count {
                BettingEventEmitter::emit_bet_created(&env, &market_id, &user, &outcome, 1_000_000, 0);
                cr += 1;
            } else if st < stats_count {
                BettingEventEmitter::emit_bet_stats_updated(&env, &market_id, st + 1, (st as i128 + 1) * 1_000_000, (st as u32) + 1);
                st += 1;
            } else {
                BettingEventEmitter::emit_bet_created(&env, &market_id, &user, &outcome, 1_000_000, 0);
                cr += 1;
            }
        }

        // Drain remaining.
        while cr < created_count {
            BettingEventEmitter::emit_bet_created(&env, &market_id, &user, &outcome, 1_000_000, 0);
            cr += 1;
        }
        while st < stats_count {
            BettingEventEmitter::emit_bet_stats_updated(&env, &market_id, st + 1, (st as i128 + 1) * 1_000_000, (st as u32) + 1);
            st += 1;
        }

        // INV-2: independent counters in storage.
        let key_cr = (NS_NONCE, TOPIC_BET_CREATED);
        let key_st = (NS_NONCE, TOPIC_BET_STATS_UPDATED);
        let stored_cr: u64 = env.storage().instance().get(&key_cr).unwrap_or(0);
        let stored_st: u64 = env.storage().instance().get(&key_st).unwrap_or(0);

        prop_assert_eq!(stored_cr, created_count, "INV-2: BetCreated nonce must equal created_count");
        prop_assert_eq!(stored_st, stats_count, "INV-2: BetStatsUpdated nonce must equal stats_count");
    }

    /// **INV-5: Payout arithmetic is tight.**
    ///
    /// For any (gross, fee) where 0 <= fee <= gross, the emitter must record
    /// net == gross - fee.
    #[test]
    fn payout_arithmetic_invariant(
        gross in 1_000_000i128..=100_000_000i128,
        fee_fraction in 0u32..=1000u32  // fee = gross * fee_fraction / 1000
    ) {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().with_mut(|li| {
            li.max_entry_ttl = 6_000_000;
            li.min_persistent_entry_ttl = 1;
            li.min_temp_entry_ttl = 1;
        });

        let market_id = Symbol::new(&env, "mkt_pay");
        let user = Address::generate(&env);

        let fee = (gross as u128 * fee_fraction as u128 / 1000u128) as i128;
        let net = gross - fee;

        BettingEventEmitter::emit_bet_claimed(&env, &market_id, &user, gross, fee, net);

        let events = env.events().all();
        let ev: BetClaimedEvent = events.get(0).unwrap().2.try_into_val().unwrap();

        prop_assert_eq!(ev.net_payout, ev.gross_payout - ev.fee_paid, "INV-5 payout arithmetic");
        prop_assert!(ev.net_payout >= 0, "net_payout must be non-negative");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// §5  Deterministic regression anchors
//
// These unit tests pin concrete invariant behaviours so CI catches regressions
// even without the proptest runner.
// ─────────────────────────────────────────────────────────────────────────────

/// INV-1 + INV-9: A long monotonic sequence persists the correct counter.
#[test]
fn nonce_advances_and_persists_over_long_sequence() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|li| {
        li.max_entry_ttl = 6_000_000;
        li.min_persistent_entry_ttl = 1;
        li.min_temp_entry_ttl = 1;
    });

    let market_id = Symbol::new(&env, "mkt_long");
    let user = Address::generate(&env);
    let outcome = soroban_sdk::String::from_str(&env, "yes");

    const N: u64 = 25;
    for i in 1..=N {
        BettingEventEmitter::emit_bet_created(&env, &market_id, &user, &outcome, 1_000_000, 0);
        let events = env.events().all();
        let ev: BetCreatedEvent = events
            .get(events.len() - 1)
            .unwrap()
            .2
            .try_into_val()
            .unwrap();
        assert_eq!(ev.nonce, i, "nonce must equal emit index at step {i}");
    }

    let key = (NS_NONCE, TOPIC_BET_CREATED);
    let stored: u64 = env.storage().instance().get(&key).unwrap_or(0);
    assert_eq!(
        stored, N,
        "persisted nonce must equal total emits after long sequence"
    );
}

/// INV-2: Five topics simultaneously — each counter is independent.
#[test]
fn all_five_topics_have_independent_nonces() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|li| {
        li.max_entry_ttl = 6_000_000;
        li.min_persistent_entry_ttl = 1;
        li.min_temp_entry_ttl = 1;
    });

    let market_id = Symbol::new(&env, "mkt_5t");
    let user = Address::generate(&env);
    let outcome = soroban_sdk::String::from_str(&env, "yes");
    let mids = sdk_vec![&env, market_id.clone()];

    // Emit different counts per topic: 3, 2, 4, 1, 5.
    let created_n: u64 = 3;
    let batch_n: u64 = 2;
    let status_n: u64 = 4;
    let claimed_n: u64 = 1;
    let stats_n: u64 = 5;

    for _ in 0..created_n {
        BettingEventEmitter::emit_bet_created(&env, &market_id, &user, &outcome, 1_000_000, 0);
    }
    for _ in 0..batch_n {
        BettingEventEmitter::emit_bet_batch_created(&env, &user, &mids, 1_000_000);
    }
    for _ in 0..status_n {
        BettingEventEmitter::emit_bet_status_changed(
            &env,
            &market_id,
            &user,
            STATUS_ACTIVE,
            STATUS_WON,
            None,
        );
    }
    for _ in 0..claimed_n {
        BettingEventEmitter::emit_bet_claimed(
            &env, &market_id, &user, 5_000_000, 100_000, 4_900_000,
        );
    }
    for _ in 0..stats_n {
        BettingEventEmitter::emit_bet_stats_updated(&env, &market_id, 1, 1_000_000, 1);
    }

    let topics_and_expected: &[(soroban_sdk::Symbol, u64)] = &[
        (TOPIC_BET_CREATED, created_n),
        (TOPIC_BET_BATCH_CREATED, batch_n),
        (TOPIC_BET_STATUS_CHANGED, status_n),
        (TOPIC_BET_CLAIMED, claimed_n),
        (TOPIC_BET_STATS_UPDATED, stats_n),
    ];
    for (topic, expected) in topics_and_expected {
        let key = (NS_NONCE, topic.clone());
        let stored: u64 = env.storage().instance().get(&key).unwrap_or(0);
        assert_eq!(
            stored, *expected,
            "INV-2 topic {:?}: stored nonce {} != expected {}",
            topic, stored, expected
        );
    }
}

/// INV-3 + INV-4: Every event carries the right topic and schema_version.
#[test]
fn all_events_carry_correct_topic_and_schema_version() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|li| {
        li.max_entry_ttl = 6_000_000;
        li.min_persistent_entry_ttl = 1;
        li.min_temp_entry_ttl = 1;
    });

    let market_id = Symbol::new(&env, "mkt_top");
    let user = Address::generate(&env);
    let outcome = soroban_sdk::String::from_str(&env, "yes");
    let mids = sdk_vec![&env, market_id.clone()];

    BettingEventEmitter::emit_bet_created(&env, &market_id, &user, &outcome, 1_000_000, 0);
    BettingEventEmitter::emit_bet_batch_created(&env, &user, &mids, 1_000_000);
    BettingEventEmitter::emit_bet_status_changed(
        &env,
        &market_id,
        &user,
        STATUS_ACTIVE,
        STATUS_LOST,
        None,
    );
    BettingEventEmitter::emit_bet_claimed(&env, &market_id, &user, 2_000_000, 40_000, 1_960_000);
    BettingEventEmitter::emit_bet_stats_updated(&env, &market_id, 5, 5_000_000, 3);

    let expected_topics = [
        TOPIC_BET_CREATED,
        TOPIC_BET_BATCH_CREATED,
        TOPIC_BET_STATUS_CHANGED,
        TOPIC_BET_CLAIMED,
        TOPIC_BET_STATS_UPDATED,
    ];

    let events = env.events().all();
    assert_eq!(events.len(), 5, "exactly five events must be published");

    for (i, (_, topics, _)) in events.iter().enumerate() {
        let first: soroban_sdk::Symbol = topics.get(0).unwrap();
        assert_eq!(first, expected_topics[i], "INV-3 wrong topic at index {i}");
        let version: u32 = topics.get(2).unwrap();
        assert_eq!(
            version, BETTING_EVENT_SCHEMA_VERSION,
            "INV-4 wrong schema_version at index {i}"
        );
    }
}

/// INV-5: Zero-fee claim has net == gross.
#[test]
fn zero_fee_claim_net_equals_gross() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|li| {
        li.max_entry_ttl = 6_000_000;
        li.min_persistent_entry_ttl = 1;
        li.min_temp_entry_ttl = 1;
    });

    let market_id = Symbol::new(&env, "mkt_zf");
    let user = Address::generate(&env);
    let gross = 7_500_000i128;

    BettingEventEmitter::emit_bet_claimed(&env, &market_id, &user, gross, 0, gross);

    let events = env.events().all();
    let ev: BetClaimedEvent = events.get(0).unwrap().2.try_into_val().unwrap();
    assert_eq!(ev.net_payout, ev.gross_payout - ev.fee_paid);
    assert_eq!(ev.net_payout, gross);
}

/// INV-6: Batch event always has bet_count == market_ids.len().
#[test]
fn batch_bet_count_equals_market_ids_len() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|li| {
        li.max_entry_ttl = 6_000_000;
        li.min_persistent_entry_ttl = 1;
        li.min_temp_entry_ttl = 1;
    });

    let user = Address::generate(&env);

    for size in 1u32..=5u32 {
        let mut mids = soroban_sdk::Vec::new(&env);
        for i in 0..size {
            // Create distinct market symbols.
            let label = alloc::format!("mkt{i}");
            mids.push_back(Symbol::new(&env, &label));
        }
        BettingEventEmitter::emit_bet_batch_created(
            &env,
            &user,
            &mids,
            1_000_000i128 * size as i128,
        );
        let events = env.events().all();
        let ev: BetBatchCreatedEvent = events
            .get(events.len() - 1)
            .unwrap()
            .2
            .try_into_val()
            .unwrap();
        assert_eq!(
            ev.bet_count, size,
            "INV-6 bet_count must match batch size {size}"
        );
        assert_eq!(
            ev.market_ids.len(),
            size,
            "INV-6 market_ids.len() must match batch size {size}"
        );
    }
}

/// INV-7: Stats events always carry non-negative amounts.
#[test]
fn stats_event_amounts_are_non_negative() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|li| {
        li.max_entry_ttl = 6_000_000;
        li.min_persistent_entry_ttl = 1;
        li.min_temp_entry_ttl = 1;
    });

    let market_id = Symbol::new(&env, "mkt_nn");

    for locked in [0i128, 1_000_000, 99_999_999] {
        BettingEventEmitter::emit_bet_stats_updated(&env, &market_id, 1, locked, 1);
        let events = env.events().all();
        let ev: BetStatsUpdatedEvent = events
            .get(events.len() - 1)
            .unwrap()
            .2
            .try_into_val()
            .unwrap();
        assert!(
            ev.total_amount_locked >= 0,
            "INV-7 total_amount_locked must be non-negative"
        );
    }
}

/// INV-8: Timestamp in event payload matches ledger timestamp at emit time.
#[test]
fn event_timestamp_matches_ledger_at_emit_time() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|li| {
        li.timestamp = 1_700_000_000;
        li.max_entry_ttl = 6_000_000;
        li.min_persistent_entry_ttl = 1;
        li.min_temp_entry_ttl = 1;
    });

    let market_id = Symbol::new(&env, "mkt_ts");
    let user = Address::generate(&env);
    let outcome = soroban_sdk::String::from_str(&env, "yes");
    let ts = env.ledger().timestamp();

    BettingEventEmitter::emit_bet_created(&env, &market_id, &user, &outcome, 1_000_000, 0);

    let events = env.events().all();
    let ev: BetCreatedEvent = events.get(0).unwrap().2.try_into_val().unwrap();
    assert_eq!(
        ev.timestamp, ts,
        "INV-8 event timestamp must equal ledger.timestamp() at emit"
    );
}
