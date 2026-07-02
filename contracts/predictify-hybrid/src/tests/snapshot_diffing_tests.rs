#![cfg(test)]

use crate::reporting::{EventSnapshot, SnapshotDiff, StateSnapshot};
use crate::types::MarketState;
use soroban_sdk::{Env, Map, String, Symbol, Vec};

fn create_event_snapshot(env: &Env, id: &str, pool: i128) -> EventSnapshot {
    EventSnapshot {
        id: Symbol::new(env, id),
        question: String::from_str(env, "Q"),
        outcomes: Vec::new(env),
        state: MarketState::Active,
        total_pool: pool,
        outcome_pools: Map::new(env),
        participant_count: 0,
        end_time: 0,
    }
}

#[test]
fn test_state_snapshot_diff() {
    let env = Env::default();

    let mut events_a = Map::new(&env);
    events_a.set(
        Symbol::new(&env, "market1"),
        create_event_snapshot(&env, "market1", 100),
    );
    events_a.set(
        Symbol::new(&env, "market2"),
        create_event_snapshot(&env, "market2", 200),
    );

    let mut events_b = Map::new(&env);
    events_b.set(
        Symbol::new(&env, "market1"),
        create_event_snapshot(&env, "market1", 100), // unchanged
    );
    events_b.set(
        Symbol::new(&env, "market2"),
        create_event_snapshot(&env, "market2", 300), // changed
    );
    events_b.set(
        Symbol::new(&env, "market3"),
        create_event_snapshot(&env, "market3", 500), // added
    );

    let snapshot_a = StateSnapshot { events: events_a };
    let snapshot_b = StateSnapshot { events: events_b };

    // diff(A, B)
    let diff_ab = StateSnapshot::diff(&env, &snapshot_a, &snapshot_b);
    assert_eq!(diff_ab.changed_markets.len(), 2);
    assert!(diff_ab.changed_markets.contains(&Symbol::new(&env, "market2")));
    assert!(diff_ab.changed_markets.contains(&Symbol::new(&env, "market3")));

    // symmetric property: diff(B, A) == diff(A, B)
    let diff_ba = StateSnapshot::diff(&env, &snapshot_b, &snapshot_a);
    assert_eq!(diff_ab.changed_markets, diff_ba.changed_markets);

    // identity property: diff(A, A) is empty
    let diff_aa = StateSnapshot::diff(&env, &snapshot_a, &snapshot_a);
    assert_eq!(diff_aa.changed_markets.len(), 0);
}
