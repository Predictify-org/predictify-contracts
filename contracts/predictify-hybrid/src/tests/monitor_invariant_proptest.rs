//! Proptest coverage for monitor invariants.
//!
//! This module asserts that the monitor queue invariant holds across arbitrary
//! valid sequences of monitor actions. It focuses on the bounded queue behavior
//! and the relationship between pushes, drains, and overflow tracking.
//!
//! ## Invariant
//!
//! - The queue length must never exceed capacity.
//! - Draining returns events in FIFO order.
//! - `is_overflow` is `true` if and only if a push occurred when the queue was
//!   already at capacity.
//!
//! ## Running
//!
//! ```bash
//! cargo test -p predictify-hybrid -- monitor_invariant_proptest
//! ```

use proptest::{collection::vec as prop_vec, prelude::*};
use soroban_sdk::{testutils::Address as _, Address, Env, Symbol};

use crate::{
    monitor::{BoundedMonitorQueue, MonitorEvent, MonitorEventCategory, MonitorEventSeverity},
    PredictifyHybrid,
};

/// Monitor queue action types used in proptest sequences.
#[derive(Debug, Clone)]
enum MonitorAction {
    Initialize(u32),
    Push(String),
    Drain(u32),
}

fn action_strategy() -> impl Strategy<Value = MonitorAction> {
    prop_oneof![
        2 => (1u32..10u32).prop_map(MonitorAction::Initialize),
        5 => (1u32..5u32).prop_map(MonitorAction::Push),
        3 => (0u32..5u32).prop_map(MonitorAction::Drain),
    ]
}

fn make_event(env: &Env, id: &str) -> MonitorEvent {
    MonitorEvent {
        event_id: Symbol::new(env, id),
        category: MonitorEventCategory::System,
        severity: MonitorEventSeverity::Info,
        message: soroban_sdk::String::from_str(env, "test"),
        market_id: None,
        actor: None,
        timestamp: env.ledger().timestamp(),
        metadata: soroban_sdk::Map::new(env),
    }
}

/// Apply a sequence of actions and validate the monitor queue invariant.
fn execute_actions(env: &Env, contract_id: &Address, actions: Vec<MonitorAction>) {
    let mut expected_queue: Vec<String> = Vec::new();
    let mut capacity = 4u32;
    let mut had_overflow = false;

    env.as_contract(contract_id, || {
        BoundedMonitorQueue::initialize(env, capacity).unwrap();
    });

    for action in actions {
        match action {
            MonitorAction::Initialize(new_capacity) => {
                capacity = new_capacity.max(1);
                env.as_contract(contract_id, || {
                    BoundedMonitorQueue::initialize(env, capacity).unwrap();
                });
                expected_queue.clear();
                had_overflow = false;
            }
            MonitorAction::Push(id) => {
                let event = make_event(env, &id);
                let overflowed = env
                    .as_contract(contract_id, || BoundedMonitorQueue::push(env, &event));

                if expected_queue.len() as u32 >= capacity {
                    had_overflow = true;
                    assert!(overflowed);
                    if capacity > 0 {
                        expected_queue.remove(0);
                    }
                } else {
                    assert!(!overflowed);
                }

                if expected_queue.len() as u32 >= capacity {
                    expected_queue.remove(0);
                }
                expected_queue.push(id);
            }
            MonitorAction::Drain(count) => {
                let drained = env
                    .as_contract(contract_id, || BoundedMonitorQueue::drain(env, count))
                    .unwrap();
                assert!(drained.len() as u32 <= count);

                for (idx, event) in drained.iter().enumerate() {
                    assert_eq!(event.event_id, Symbol::new(env, &expected_queue[idx]));
                }

                expected_queue.drain(0..drained.len());
            }
        }

        let queue_stats = env.as_contract(contract_id, || BoundedMonitorQueue::stats(env)).unwrap();
        assert!(queue_stats.current_len <= capacity);
        assert_eq!(BoundedMonitorQueue::is_overflow(env), had_overflow);
    }
}

proptest! {
    #[test]
    fn monitor_queue_invariant(actions in prop_vec(action_strategy(), 1..30)) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(PredictifyHybrid, ());

        execute_actions(&env, &contract_id, actions);
    }
}
