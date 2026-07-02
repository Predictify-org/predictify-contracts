#![cfg(test)]

use crate::reporting::{EventSnapshot, PlatformStats, SnapshotDiff, StateSnapshot};
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

fn create_platform_stats(env: &Env, fees: i128, pool: i128) -> PlatformStats {
    PlatformStats {
        total_active_events: 0,
        total_resolved_events: 0,
        total_pool_all_events: pool,
        total_fees_collected: fees,
        version: String::from_str(env, "1.0.0"),
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
    let stats_a = create_platform_stats(&env, 50, 300);

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
    let stats_b = create_platform_stats(&env, 70, 900);

    let snapshot_a = StateSnapshot { stats: stats_a.clone(), events: events_a };
    let snapshot_b = StateSnapshot { stats: stats_b.clone(), events: events_b };

    // diff(A, B)
    let diff_ab = StateSnapshot::diff(&env, &snapshot_a, &snapshot_b);
    assert_eq!(diff_ab.added.len(), 1);
    assert!(diff_ab.added.contains(&Symbol::new(&env, "market3")));
    assert_eq!(diff_ab.removed.len(), 0);
    assert_eq!(diff_ab.changed.len(), 1);
    assert!(diff_ab.changed.contains(&Symbol::new(&env, "market2")));
    assert_eq!(diff_ab.fee_delta, 20); // 70 - 50
    assert_eq!(diff_ab.total_pool_delta, 600); // 900 - 300

    // diff(B, A)
    let diff_ba = StateSnapshot::diff(&env, &snapshot_b, &snapshot_a);
    assert_eq!(diff_ba.added.len(), 0);
    assert_eq!(diff_ba.removed.len(), 1);
    assert!(diff_ba.removed.contains(&Symbol::new(&env, "market3")));
    assert_eq!(diff_ba.changed.len(), 1);
    assert!(diff_ba.changed.contains(&Symbol::new(&env, "market2")));
    assert_eq!(diff_ba.fee_delta, -20); // 50 - 70
    assert_eq!(diff_ba.total_pool_delta, -600); // 300 - 900

    // symmetric property: diff(a, b).invert() == diff(b, a)
    assert_eq!(diff_ab.invert(&env), diff_ba);

    // identity property: diff(A, A) is empty
    let diff_aa = StateSnapshot::diff(&env, &snapshot_a, &snapshot_a);
    assert_eq!(diff_aa.added.len(), 0);
    assert_eq!(diff_aa.removed.len(), 0);
    assert_eq!(diff_aa.changed.len(), 0);
    assert_eq!(diff_aa.fee_delta, 0);
    assert_eq!(diff_aa.total_pool_delta, 0);
}
