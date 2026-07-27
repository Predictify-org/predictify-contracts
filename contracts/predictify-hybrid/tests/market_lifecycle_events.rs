#![cfg(test)]

use soroban_sdk::{testutils::Events, Env, Symbol, String, vec, symbol_short, Address};
use predictify_hybrid::events::{
    EventEmitter, MarketCreatedEvent, MarketActivatedEvent, MarketEndedEvent,
    MarketDisputeStartedEvent, MarketCancelledEvent, MarketOutcomeSetEvent,
    MarketPausedEvent, MarketResumedEvent,
};
use predictify_hybrid::storage::DataKey;

// ===== MARKET CREATED EVENT TESTS =====

#[test]
fn test_emit_market_created_event() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let market_id = symbol_short!("m_test");
    let question = String::from_str(&env, "Will Bitcoin reach $100k?");
    let outcomes = vec![
        &env,
        String::from_str(&env, "Yes"),
        String::from_str(&env, "No"),
    ];
    let end_time = 1000u64;

    EventEmitter::emit_market_created(&env, &market_id, &question, &outcomes, &admin, end_time);

    let events = env.events().all();
    assert_eq!(events.len(), 1);

    // Verify nonce was incremented
    let key = DataKey::EventNonce(symbol_short!("mkt_crt"));
    let nonce: u64 = env.storage().persistent().get(&key).unwrap();
    assert_eq!(nonce, 1);
}

#[test]
fn test_market_created_event_fields() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let market_id = symbol_short!("m_test");
    let question = String::from_str(&env, "Test question?");
    let outcomes = vec![&env, String::from_str(&env, "Yes"), String::from_str(&env, "No")];
    let end_time = 5000u64;

    EventEmitter::emit_market_created(&env, &market_id, &question, &outcomes, &admin, end_time);

    let stored_events: Vec<MarketCreatedEvent> = env.events().all();
    assert_eq!(stored_events.len(), 1);
    let event = &stored_events[0];

    assert_eq!(event.market_id, market_id);
    assert_eq!(event.question, question);
    assert_eq!(event.outcomes.len(), 2);
    assert_eq!(event.admin, admin);
    assert_eq!(event.end_time, end_time);
    assert_eq!(event.nonce, 1);
}

// ===== MARKET ACTIVATED EVENT TESTS =====

#[test]
fn test_emit_market_activated_event() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let market_id = symbol_short!("m_act");

    EventEmitter::emit_market_activated(&env, &market_id, &admin);

    let events = env.events().all();
    assert_eq!(events.len(), 1);

    let key = DataKey::EventNonce(symbol_short!("mkt_act"));
    let nonce: u64 = env.storage().persistent().get(&key).unwrap();
    assert_eq!(nonce, 1);
}

#[test]
fn test_market_activated_event_fields() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let market_id = symbol_short!("m_act");

    let before_timestamp = env.ledger().timestamp();
    EventEmitter::emit_market_activated(&env, &market_id, &admin);
    let after_timestamp = env.ledger().timestamp();

    let stored_events: Vec<MarketActivatedEvent> = env.events().all();
    assert_eq!(stored_events.len(), 1);
    let event = &stored_events[0];

    assert_eq!(event.market_id, market_id);
    assert_eq!(event.admin, admin);
    assert_eq!(event.nonce, 1);
    assert!(event.timestamp >= before_timestamp && event.timestamp <= after_timestamp);
}

// ===== MARKET ENDED EVENT TESTS =====

#[test]
fn test_emit_market_ended_event() {
    let env = Env::default();
    env.mock_all_auths();

    let market_id = symbol_short!("m_end");
    let total_staked = 1000i128;
    let participant_count = 10u32;

    EventEmitter::emit_market_ended(&env, &market_id, total_staked, participant_count);

    let events = env.events().all();
    assert_eq!(events.len(), 1);

    let key = DataKey::EventNonce(symbol_short!("mkt_end"));
    let nonce: u64 = env.storage().persistent().get(&key).unwrap();
    assert_eq!(nonce, 1);
}

#[test]
fn test_market_ended_event_fields() {
    let env = Env::default();
    env.mock_all_auths();

    let market_id = symbol_short!("m_end");
    let total_staked = 5000i128;
    let participant_count = 25u32;

    EventEmitter::emit_market_ended(&env, &market_id, total_staked, participant_count);

    let stored_events: Vec<MarketEndedEvent> = env.events().all();
    assert_eq!(stored_events.len(), 1);
    let event = &stored_events[0];

    assert_eq!(event.market_id, market_id);
    assert_eq!(event.total_staked, total_staked);
    assert_eq!(event.participant_count, participant_count);
    assert_eq!(event.nonce, 1);
}

// ===== MARKET DISPUTE STARTED EVENT TESTS =====

#[test]
fn test_emit_market_dispute_started_event() {
    let env = Env::default();
    env.mock_all_auths();

    let market_id = symbol_short!("m_disp");
    let initiator = Address::generate(&env);
    let dispute_stake = 500i128;
    let disputed_outcome = String::from_str(&env, "No");
    let dispute_end_time = 2000u64;

    EventEmitter::emit_market_dispute_started(
        &env,
        &market_id,
        &initiator,
        dispute_stake,
        &disputed_outcome,
        dispute_end_time,
    );

    let events = env.events().all();
    assert_eq!(events.len(), 1);

    let key = DataKey::EventNonce(symbol_short!("mkt_disp"));
    let nonce: u64 = env.storage().persistent().get(&key).unwrap();
    assert_eq!(nonce, 1);
}

#[test]
fn test_market_dispute_started_event_fields() {
    let env = Env::default();
    env.mock_all_auths();

    let market_id = symbol_short!("m_disp");
    let initiator = Address::generate(&env);
    let dispute_stake = 1000i128;
    let disputed_outcome = String::from_str(&env, "Yes");
    let dispute_end_time = 3000u64;

    EventEmitter::emit_market_dispute_started(
        &env,
        &market_id,
        &initiator,
        dispute_stake,
        &disputed_outcome,
        dispute_end_time,
    );

    let stored_events: Vec<MarketDisputeStartedEvent> = env.events().all();
    assert_eq!(stored_events.len(), 1);
    let event = &stored_events[0];

    assert_eq!(event.market_id, market_id);
    assert_eq!(event.dispute_initiator, initiator);
    assert_eq!(event.dispute_stake, dispute_stake);
    assert_eq!(event.disputed_outcome, disputed_outcome);
    assert_eq!(event.dispute_end_time, dispute_end_time);
    assert_eq!(event.nonce, 1);
}

// ===== MARKET CANCELLED EVENT TESTS =====

#[test]
fn test_emit_market_cancelled_event() {
    let env = Env::default();
    env.mock_all_auths();

    let market_id = symbol_short!("m_canc");
    let admin = Address::generate(&env);
    let reason = String::from_str(&env, "Admin decision");
    let total_refunded = 2000i128;

    EventEmitter::emit_market_cancelled(&env, &market_id, &admin, &reason, total_refunded);

    let events = env.events().all();
    assert_eq!(events.len(), 1);

    let key = DataKey::EventNonce(symbol_short!("mkt_canc"));
    let nonce: u64 = env.storage().persistent().get(&key).unwrap();
    assert_eq!(nonce, 1);
}

#[test]
fn test_market_cancelled_event_fields() {
    let env = Env::default();
    env.mock_all_auths();

    let market_id = symbol_short!("m_canc");
    let admin = Address::generate(&env);
    let reason = String::from_str(&env, "Invalid outcomes");
    let total_refunded = 5000i128;

    EventEmitter::emit_market_cancelled(&env, &market_id, &admin, &reason, total_refunded);

    let stored_events: Vec<MarketCancelledEvent> = env.events().all();
    assert_eq!(stored_events.len(), 1);
    let event = &stored_events[0];

    assert_eq!(event.market_id, market_id);
    assert_eq!(event.admin, admin);
    assert_eq!(event.reason, reason);
    assert_eq!(event.total_refunded, total_refunded);
    assert_eq!(event.nonce, 1);
}

// ===== MARKET OUTCOME SET EVENT TESTS =====

#[test]
fn test_emit_market_outcome_set_event() {
    let env = Env::default();
    env.mock_all_auths();

    let market_id = symbol_short!("m_outc");
    let winning_outcomes = vec![&env, String::from_str(&env, "Yes")];
    let payout_pool = 10000i128;
    let winner_count = 5u32;

    EventEmitter::emit_market_outcome_set(
        &env,
        &market_id,
        &winning_outcomes,
        payout_pool,
        winner_count,
    );

    let events = env.events().all();
    assert_eq!(events.len(), 1);

    let key = DataKey::EventNonce(symbol_short!("mkt_outc"));
    let nonce: u64 = env.storage().persistent().get(&key).unwrap();
    assert_eq!(nonce, 1);
}

#[test]
fn test_market_outcome_set_event_fields() {
    let env = Env::default();
    env.mock_all_auths();

    let market_id = symbol_short!("m_outc");
    let winning_outcomes = vec![
        &env,
        String::from_str(&env, "Yes"),
        String::from_str(&env, "Maybe"),
    ];
    let payout_pool = 15000i128;
    let winner_count = 12u32;

    EventEmitter::emit_market_outcome_set(
        &env,
        &market_id,
        &winning_outcomes,
        payout_pool,
        winner_count,
    );

    let stored_events: Vec<MarketOutcomeSetEvent> = env.events().all();
    assert_eq!(stored_events.len(), 1);
    let event = &stored_events[0];

    assert_eq!(event.market_id, market_id);
    assert_eq!(event.winning_outcomes.len(), 2);
    assert_eq!(event.payout_pool, payout_pool);
    assert_eq!(event.winner_count, winner_count);
    assert_eq!(event.nonce, 1);
}

// ===== MARKET PAUSED EVENT TESTS =====

#[test]
fn test_emit_market_paused_event() {
    let env = Env::default();
    env.mock_all_auths();

    let market_id = symbol_short!("m_paus");
    let reason = String::from_str(&env, "Circuit breaker triggered");
    let paused_by = Address::generate(&env);

    EventEmitter::emit_market_paused(&env, &market_id, &reason, true, &paused_by);

    let events = env.events().all();
    assert_eq!(events.len(), 1);

    let key = DataKey::EventNonce(symbol_short!("mkt_paus"));
    let nonce: u64 = env.storage().persistent().get(&key).unwrap();
    assert_eq!(nonce, 1);
}

#[test]
fn test_market_paused_event_fields_circuit_breaker() {
    let env = Env::default();
    env.mock_all_auths();

    let market_id = symbol_short!("m_paus");
    let reason = String::from_str(&env, "Anomaly detected");
    let paused_by = Address::generate(&env);

    EventEmitter::emit_market_paused(&env, &market_id, &reason, true, &paused_by);

    let stored_events: Vec<MarketPausedEvent> = env.events().all();
    assert_eq!(stored_events.len(), 1);
    let event = &stored_events[0];

    assert_eq!(event.market_id, market_id);
    assert_eq!(event.reason, reason);
    assert_eq!(event.is_circuit_breaker, true);
    assert_eq!(event.paused_by, paused_by);
    assert_eq!(event.nonce, 1);
}

#[test]
fn test_market_paused_event_fields_admin() {
    let env = Env::default();
    env.mock_all_auths();

    let market_id = symbol_short!("m_paus");
    let reason = String::from_str(&env, "Maintenance");
    let admin = Address::generate(&env);

    EventEmitter::emit_market_paused(&env, &market_id, &reason, false, &admin);

    let stored_events: Vec<MarketPausedEvent> = env.events().all();
    let event = &stored_events[0];

    assert_eq!(event.is_circuit_breaker, false);
}

// ===== MARKET RESUMED EVENT TESTS =====

#[test]
fn test_emit_market_resumed_event() {
    let env = Env::default();
    env.mock_all_auths();

    let market_id = symbol_short!("m_res");
    let resumed_by = Address::generate(&env);
    let reason = String::from_str(&env, "Issue resolved");

    EventEmitter::emit_market_resumed(&env, &market_id, &resumed_by, &reason);

    let events = env.events().all();
    assert_eq!(events.len(), 1);

    let key = DataKey::EventNonce(symbol_short!("mkt_res"));
    let nonce: u64 = env.storage().persistent().get(&key).unwrap();
    assert_eq!(nonce, 1);
}

#[test]
fn test_market_resumed_event_fields() {
    let env = Env::default();
    env.mock_all_auths();

    let market_id = symbol_short!("m_res");
    let resumed_by = Address::generate(&env);
    let reason = String::from_str(&env, "System stabilized");

    EventEmitter::emit_market_resumed(&env, &market_id, &resumed_by, &reason);

    let stored_events: Vec<MarketResumedEvent> = env.events().all();
    assert_eq!(stored_events.len(), 1);
    let event = &stored_events[0];

    assert_eq!(event.market_id, market_id);
    assert_eq!(event.resumed_by, resumed_by);
    assert_eq!(event.reason, reason);
    assert_eq!(event.nonce, 1);
}

// ===== NONCE ISOLATION TESTS =====

#[test]
fn test_lifecycle_events_nonce_isolation() {
    let env = Env::default();
    env.mock_all_auths();

    let market_id = symbol_short!("m_test");
    let admin = Address::generate(&env);

    // Emit different event types
    EventEmitter::emit_market_activated(&env, &market_id, &admin);
    EventEmitter::emit_market_ended(&env, &market_id, 1000, 5);
    EventEmitter::emit_market_cancelled(&env, &market_id, &admin, &String::from_str(&env, "reason"), 500);

    // Each event type should have its own nonce counter
    let key_act = DataKey::EventNonce(symbol_short!("mkt_act"));
    let key_end = DataKey::EventNonce(symbol_short!("mkt_end"));
    let key_canc = DataKey::EventNonce(symbol_short!("mkt_canc"));

    let nonce_act: u64 = env.storage().persistent().get(&key_act).unwrap();
    let nonce_end: u64 = env.storage().persistent().get(&key_end).unwrap();
    let nonce_canc: u64 = env.storage().persistent().get(&key_canc).unwrap();

    assert_eq!(nonce_act, 1);
    assert_eq!(nonce_end, 1);
    assert_eq!(nonce_canc, 1);
}

#[test]
fn test_nonce_monotonic_increment() {
    let env = Env::default();
    env.mock_all_auths();

    let market_id1 = symbol_short!("m_1");
    let market_id2 = symbol_short!("m_2");
    let admin = Address::generate(&env);

    // Emit same event type twice
    EventEmitter::emit_market_activated(&env, &market_id1, &admin);
    EventEmitter::emit_market_activated(&env, &market_id2, &admin);

    let key = DataKey::EventNonce(symbol_short!("mkt_act"));
    let nonce: u64 = env.storage().persistent().get(&key).unwrap();
    assert_eq!(nonce, 2, "Nonce should increment monotonically");
}

// ===== EDGE CASE TESTS =====

#[test]
fn test_market_outcome_set_with_multiple_winners() {
    let env = Env::default();
    env.mock_all_auths();

    let market_id = symbol_short!("m_tie");
    let winning_outcomes = vec![
        &env,
        String::from_str(&env, "Yes"),
        String::from_str(&env, "No"),
        String::from_str(&env, "Maybe"),
    ];
    let payout_pool = 3000i128;
    let winner_count = 15u32;

    EventEmitter::emit_market_outcome_set(
        &env,
        &market_id,
        &winning_outcomes,
        payout_pool,
        winner_count,
    );

    let stored_events: Vec<MarketOutcomeSetEvent> = env.events().all();
    let event = &stored_events[0];

    assert_eq!(event.winning_outcomes.len(), 3);
    assert_eq!(event.winner_count, winner_count);
}

#[test]
fn test_zero_participants_edge_case() {
    let env = Env::default();
    env.mock_all_auths();

    let market_id = symbol_short!("m_zero");

    // Market with 0 participants and 0 stake
    EventEmitter::emit_market_ended(&env, &market_id, 0, 0);

    let stored_events: Vec<MarketEndedEvent> = env.events().all();
    let event = &stored_events[0];

    assert_eq!(event.total_staked, 0);
    assert_eq!(event.participant_count, 0);
}

#[test]
fn test_large_stake_amounts() {
    let env = Env::default();
    env.mock_all_auths();

    let market_id = symbol_short!("m_big");
    let large_stake = i128::MAX / 2; // Large but safe value

    EventEmitter::emit_market_ended(&env, &market_id, large_stake, 1000000u32);

    let stored_events: Vec<MarketEndedEvent> = env.events().all();
    let event = &stored_events[0];

    assert_eq!(event.total_staked, large_stake);
}

#[test]
fn test_long_reason_strings() {
    let env = Env::default();
    env.mock_all_auths();

    let market_id = symbol_short!("m_long");
    let admin = Address::generate(&env);
    let long_reason = String::from_str(
        &env,
        "This is a very detailed reason for cancellation that provides comprehensive information about why the market was cancelled",
    );

    EventEmitter::emit_market_cancelled(&env, &market_id, &admin, &long_reason, 1000);

    let stored_events: Vec<MarketCancelledEvent> = env.events().all();
    let event = &stored_events[0];

    assert_eq!(event.reason, long_reason);
}

// ===== TIMESTAMP VALIDATION TESTS =====

#[test]
fn test_event_timestamps_are_consistent() {
    let env = Env::default();
    env.mock_all_auths();

    let market_id = symbol_short!("m_ts");
    let admin = Address::generate(&env);
    let ts_before = env.ledger().timestamp();

    EventEmitter::emit_market_activated(&env, &market_id, &admin);

    let ts_after = env.ledger().timestamp();

    let stored_events: Vec<MarketActivatedEvent> = env.events().all();
    let event = &stored_events[0];

    assert!(event.timestamp >= ts_before);
    assert!(event.timestamp <= ts_after);
}

#[test]
fn test_multiple_events_timestamps_ordered() {
    let env = Env::default();
    env.mock_all_auths();

    let market_id = symbol_short!("m_order");
    let admin = Address::generate(&env);

    EventEmitter::emit_market_activated(&env, &market_id, &admin);
    EventEmitter::emit_market_ended(&env, &market_id, 1000, 5);
    EventEmitter::emit_market_cancelled(&env, &market_id, &admin, &String::from_str(&env, "reason"), 500);

    let activated_events: Vec<MarketActivatedEvent> = env.events().all();
    assert_eq!(activated_events.len(), 1);
}
