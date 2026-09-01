//! Tests for event topic compatibility across contract upgrades (issue #1391).
//!
//! # Coverage matrix
//!
//! | Area                         | Tests                                                    |
//! |------------------------------|----------------------------------------------------------|
//! | Registry completeness        | every topic in TOPIC_REGISTRY is retrievable             |
//! | Registry determinism         | repeated lookups return identical descriptors            |
//! | Registry boundary cases      | unknown names return None, no panic                      |
//! | Schema version integrity     | all schema versions are ≥ 1                              |
//! | Schema version map           | get_version_map covers all topics                        |
//! | EventSchemaRegistry bridge   | get_schema delegates to registry for all known names     |
//! | EventSchemaRegistry extended | get_all_schemas returns ≥ TOPIC_REGISTRY.len() entries   |
//! | Nonce preservation           | preserve / restore round-trips correctly                 |
//! | Nonce idempotency            | restore never rolls back a nonce                         |
//! | Nonce no-op on empty         | restore with no snapshot is a no-op                      |
//! | Alias persistence            | register_alias stores and retrieves correctly            |
//! | Alias upgrade hook           | register_all_aliases processes TOPIC_ALIASES             |
//! | Alias not found              | get_alias returns None for unregistered symbol           |
//! | Compat bridge same topic     | single emit when old == new                              |
//! | Compat bridge diff topic     | two emits when old != new                                |
//! | UpgradeManager hooks         | prepare_event_compat / finalize_event_compat round-trip  |
//! | Regression: reset nonce      | nonce never goes backwards after restore                 |
//! | Regression: DataKey variant  | EventTopicAlias round-trips through storage              |

#![cfg(test)]

use soroban_sdk::{symbol_short, testutils::Events, Env, Symbol, Vec};

use alloc::format;
use crate::event_topic_compat::{
    EventCompatBridge, EventNonceGuard, EventTopicRegistry, TOPIC_ALIASES, TOPIC_REGISTRY,
};
use crate::events::EventSchemaRegistry;
use crate::storage::DataKey;

// ─────────────────────────────────────────────────────────────────────────────
// Helper
// ─────────────────────────────────────────────────────────────────────────────

fn fresh() -> Env {
    Env::default()
}

// ─────────────────────────────────────────────────────────────────────────────
// 1. Registry completeness & determinism
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn registry_returns_descriptor_for_every_registered_name() {
    let env = fresh();
    for &(_sym, _version, name) in TOPIC_REGISTRY {
        let desc = EventTopicRegistry::get(&env, name);
        assert!(
            desc.is_some(),
            "EventTopicRegistry::get returned None for registered name \"{}\"",
            name
        );
        let d = desc.unwrap();
        assert_eq!(d.schema_version, EventTopicRegistry::get(&env, name).unwrap().schema_version);
    }
}

#[test]
fn registry_get_by_symbol_covers_all_symbols() {
    let env = fresh();
    for &(sym, _version, _name) in TOPIC_REGISTRY {
        let desc = EventTopicRegistry::get_by_symbol(&env, sym);
        assert!(
            desc.is_some(),
            "get_by_symbol returned None for symbol \"{}\"",
            sym
        );
    }
}

#[test]
fn registry_get_all_topics_length_matches_constant_table() {
    let env = fresh();
    let topics = EventTopicRegistry::get_all_topics(&env);
    assert_eq!(
        topics.len() as usize,
        TOPIC_REGISTRY.len(),
        "get_all_topics length mismatch"
    );
}

#[test]
fn registry_topic_count_is_consistent() {
    assert_eq!(
        EventTopicRegistry::topic_count() as usize,
        TOPIC_REGISTRY.len()
    );
}

#[test]
fn registry_get_returns_none_for_unknown_name() {
    let env = fresh();
    // Must not panic; must return None.
    assert!(EventTopicRegistry::get(&env, "").is_none());
    assert!(EventTopicRegistry::get(&env, "no_such_event_xyzzy").is_none());
}

#[test]
fn registry_get_by_symbol_returns_none_for_unknown_symbol() {
    let env = fresh();
    assert!(EventTopicRegistry::get_by_symbol(&env, "").is_none());
    assert!(EventTopicRegistry::get_by_symbol(&env, "zzz_nope").is_none());
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. Schema version integrity
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn all_schema_versions_are_at_least_one() {
    let env = fresh();
    for desc in EventTopicRegistry::get_all_topics(&env).iter() {
        assert!(
            desc.schema_version >= 1,
            "schema_version must be ≥ 1 for topic {:?}",
            desc.topic
        );
    }
}

#[test]
fn schema_version_lookup_returns_zero_for_unknown() {
    let env = fresh();
    assert_eq!(EventTopicRegistry::schema_version(&env, "zzz_none"), 0);
}

#[test]
fn get_version_map_covers_all_topics() {
    let env = fresh();
    let map = EventTopicRegistry::get_version_map(&env);
    for &(sym, version, _name) in TOPIC_REGISTRY {
        let key = Symbol::new(&env, sym);
        let stored = map.get(key.clone());
        assert!(
            stored.is_some(),
            "get_version_map missing symbol \"{}\"",
            sym
        );
        assert_eq!(stored.unwrap(), version);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. EventSchemaRegistry delegation (backward-compat layer)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn event_schema_registry_delegates_to_topic_registry_for_all_names() {
    let env = fresh();
    // Every registered human-readable name must be resolvable via the old API.
    for &(_sym, version, name) in TOPIC_REGISTRY {
        let schema = EventSchemaRegistry::get_schema(&env, name);
        assert!(
            schema.is_some(),
            "EventSchemaRegistry::get_schema returned None for \"{}\"",
            name
        );
        assert_eq!(
            schema.unwrap().schema_version,
            version,
            "schema_version mismatch for \"{}\"",
            name
        );
    }
}

#[test]
fn event_schema_registry_returns_none_for_unknown() {
    let env = fresh();
    assert!(EventSchemaRegistry::get_schema(&env, "no_such_xyzzy").is_none());
}

#[test]
fn event_schema_registry_get_all_schemas_non_empty() {
    let env = fresh();
    let all = EventSchemaRegistry::get_all_schemas(&env);
    assert!(
        all.len() as usize >= TOPIC_REGISTRY.len(),
        "get_all_schemas must return at least {} entries",
        TOPIC_REGISTRY.len()
    );
}

#[test]
fn event_schema_registry_topic_count_matches() {
    assert_eq!(
        EventSchemaRegistry::topic_count() as usize,
        TOPIC_REGISTRY.len()
    );
}

// Legacy hard-coded names that existed before #1391.
#[test]
fn legacy_schema_names_still_resolve() {
    let env = fresh();
    for name in &["oracle_result", "dispute_opened", "storage_tier_changed", "payout_remainder_allocated"] {
        assert!(
            EventSchemaRegistry::get_schema(&env, name).is_some(),
            "Legacy name \"{}\" should still resolve",
            name
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. DataKey::EventTopicAlias round-trip through storage
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn datakey_event_topic_alias_stores_and_retrieves() {
    let env = fresh();
    let old_topic = symbol_short!("old_t");
    let new_topic = symbol_short!("new_t");
    let alias = crate::event_topic_compat::TopicAlias {
        old_topic: old_topic.clone(),
        new_topic: new_topic.clone(),
        registered_at: env.ledger().sequence(),
        since_version: 1_001_000,
    };

    let key = DataKey::EventTopicAlias(old_topic.clone());
    env.storage().persistent().set(&key, &alias);

    let retrieved: Option<crate::event_topic_compat::TopicAlias> =
        env.storage().persistent().get(&DataKey::EventTopicAlias(old_topic.clone()));
    assert!(retrieved.is_some());
    let r = retrieved.unwrap();
    assert_eq!(r.old_topic, old_topic);
    assert_eq!(r.new_topic, new_topic);
    assert_eq!(r.since_version, 1_001_000);
}

// ─────────────────────────────────────────────────────────────────────────────
// 5. Topic alias registration
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn register_alias_persists_and_get_alias_retrieves() {
    let env = fresh();
    let old_topic = symbol_short!("stale_t");
    let new_topic = symbol_short!("fresh_t");
    let since = 2_000_000_u64;

    EventCompatBridge::register_alias(&env, old_topic.clone(), new_topic.clone(), since);

    let alias = EventCompatBridge::get_alias(&env, &old_topic).expect("alias must be present");
    assert_eq!(alias.old_topic, old_topic);
    assert_eq!(alias.new_topic, new_topic);
    assert_eq!(alias.since_version, since);
}

#[test]
fn get_alias_returns_none_for_unregistered_symbol() {
    let env = fresh();
    let sym = symbol_short!("nope_t");
    assert!(EventCompatBridge::get_alias(&env, &sym).is_none());
}

#[test]
fn register_alias_is_idempotent_last_write_wins() {
    let env = fresh();
    let old_topic = symbol_short!("idem_t");
    let first_new = symbol_short!("first_t");
    let second_new = symbol_short!("secnd_t");

    EventCompatBridge::register_alias(&env, old_topic.clone(), first_new.clone(), 1_000);
    EventCompatBridge::register_alias(&env, old_topic.clone(), second_new.clone(), 2_000);

    let alias = EventCompatBridge::get_alias(&env, &old_topic).unwrap();
    // Second call wins.
    assert_eq!(alias.new_topic, second_new);
    assert_eq!(alias.since_version, 2_000);
}

#[test]
fn register_all_aliases_processes_constant_table() {
    let env = fresh();
    // Should not panic; idempotent even if TOPIC_ALIASES is empty.
    EventCompatBridge::register_all_aliases(&env, 1_001_000);

    // After registration, every pair in TOPIC_ALIASES should be retrievable.
    for &(old_sym, new_sym) in TOPIC_ALIASES {
        let old_topic = Symbol::new(&env, old_sym);
        let alias = EventCompatBridge::get_alias(&env, &old_topic)
            .expect(&format!("alias for \"{}\" must be present", old_sym));
        assert_eq!(alias.new_topic, Symbol::new(&env, new_sym));
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 6. Nonce preservation
// ─────────────────────────────────────────────────────────────────────────────

fn write_nonce(env: &Env, sym: &str, value: u64) {
    let topic = Symbol::new(env, sym);
    env.storage()
        .persistent()
        .set(&DataKey::EventNonce(topic), &value);
}

fn read_nonce(env: &Env, sym: &str) -> u64 {
    let topic = Symbol::new(env, sym);
    env.storage()
        .persistent()
        .get(&DataKey::EventNonce(topic))
        .unwrap_or(0)
}

#[test]
fn preserve_and_restore_nonces_round_trip() {
    let env = fresh();

    // Write known nonces for a few topics.
    write_nonce(&env, "mkt_crt", 42);
    write_nonce(&env, "bet_plc", 7);
    write_nonce(&env, "vote",    99);

    // Snapshot.
    EventNonceGuard::preserve_nonces(&env);

    // Simulate migration clearing the nonces.
    write_nonce(&env, "mkt_crt", 0);
    write_nonce(&env, "bet_plc", 0);
    write_nonce(&env, "vote",    0);

    // Restore.
    let count = EventNonceGuard::restore_nonces(&env);
    assert!(count >= 3, "expected at least 3 nonces restored, got {}", count);

    assert_eq!(read_nonce(&env, "mkt_crt"), 42);
    assert_eq!(read_nonce(&env, "bet_plc"), 7);
    assert_eq!(read_nonce(&env, "vote"),    99);
}

#[test]
fn restore_never_rolls_back_a_nonce_that_advanced() {
    let env = fresh();

    // Snapshot at value 10.
    write_nonce(&env, "mkt_crt", 10);
    EventNonceGuard::preserve_nonces(&env);

    // A post-upgrade emission already advanced the nonce to 20.
    write_nonce(&env, "mkt_crt", 20);

    EventNonceGuard::restore_nonces(&env);

    // Must stay at 20, not roll back to 10.
    assert_eq!(read_nonce(&env, "mkt_crt"), 20);
}

#[test]
fn restore_with_no_snapshot_is_a_no_op() {
    let env = fresh();
    write_nonce(&env, "mkt_crt", 5);

    // No prior preserve call — snapshot key does not exist.
    let count = EventNonceGuard::restore_nonces(&env);

    // Nothing should have been modified.
    assert_eq!(count, 0);
    assert_eq!(read_nonce(&env, "mkt_crt"), 5);
}

#[test]
fn clear_snapshot_removes_stored_data() {
    let env = fresh();
    write_nonce(&env, "vote", 3);
    EventNonceGuard::preserve_nonces(&env);

    let snap = EventNonceGuard::read_snapshot(&env);
    assert!(snap.len() > 0, "snapshot should be non-empty");

    EventNonceGuard::clear_snapshot(&env);
    let after = EventNonceGuard::read_snapshot(&env);
    assert_eq!(after.len(), 0, "snapshot should be empty after clear");
}

#[test]
fn preserve_ignores_zero_nonces() {
    let env = fresh();

    // Write a zero nonce — should NOT be included in snapshot.
    write_nonce(&env, "mkt_crt", 0);

    EventNonceGuard::preserve_nonces(&env);

    let snap = EventNonceGuard::read_snapshot(&env);
    let has_mkt_crt = snap.iter().any(|s| {
        s.topic == Symbol::new(&env, "mkt_crt")
    });
    assert!(!has_mkt_crt, "zero nonces must not be snapshotted");
}

// ─────────────────────────────────────────────────────────────────────────────
// 7. Compatibility bridge – emit behaviour
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn compat_bridge_emits_single_event_when_topics_are_equal() {
    let env = fresh();
    let topic = symbol_short!("same_t");
    let data: i128 = 1234;

    EventCompatBridge::publish_with_compat(&env, topic.clone(), topic.clone(), &data);

    let events = env.events().all();
    // Only one event should have been emitted.
    assert_eq!(events.events().len(), 1, "expected exactly 1 event when old == new topic");
}

#[test]
fn compat_bridge_emits_two_events_when_topics_differ() {
    let env = fresh();
    let old_topic = symbol_short!("old_ev");
    let new_topic = symbol_short!("new_ev");
    let data: i128 = 9876;

    EventCompatBridge::publish_with_compat(&env, old_topic.clone(), new_topic.clone(), &data);

    let events = env.events().all();
    // Two events: one under new_topic, one under old_topic.
    assert_eq!(events.events().len(), 2, "expected 2 events for a renamed topic");
}

#[test]
fn compat_bridge_emits_are_idempotent_on_repeated_calls() {
    let env = fresh();
    let old = symbol_short!("rep_o");
    let new = symbol_short!("rep_n");
    let data: u32 = 7;

    EventCompatBridge::publish_with_compat(&env, old.clone(), new.clone(), &data);
    EventCompatBridge::publish_with_compat(&env, old.clone(), new.clone(), &data);

    // Soroban events are append-only; two calls produce four events total.
    // This test documents the expected behaviour (not a defect) and ensures
    // that repeated calls do not panic or corrupt storage.
    let events = env.events().all();
    assert_eq!(events.events().len(), 4);
}

// ─────────────────────────────────────────────────────────────────────────────
// 8. UpgradeManager hooks
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn upgrade_manager_preserve_and_restore_event_nonces_round_trip() {
    use crate::upgrade_manager::UpgradeManager;

    let env = fresh();
    write_nonce(&env, "mkt_crt", 100);
    write_nonce(&env, "bet_plc", 50);

    UpgradeManager::preserve_event_nonces(&env);

    // Simulate migration resetting nonces.
    write_nonce(&env, "mkt_crt", 0);
    write_nonce(&env, "bet_plc", 0);

    let restored = UpgradeManager::restore_event_nonces(&env);
    assert!(restored >= 2);

    assert_eq!(read_nonce(&env, "mkt_crt"), 100);
    assert_eq!(read_nonce(&env, "bet_plc"), 50);
}

#[test]
fn upgrade_manager_prepare_and_finalize_event_compat() {
    use crate::upgrade_manager::UpgradeManager;

    let env = fresh();
    write_nonce(&env, "vote", 77);

    UpgradeManager::prepare_event_compat(&env, 1_001_000);

    // Simulate nonce reset by migration.
    write_nonce(&env, "vote", 0);

    UpgradeManager::finalize_event_compat(&env);

    assert_eq!(read_nonce(&env, "vote"), 77);
}

#[test]
fn upgrade_manager_register_topic_aliases_does_not_panic() {
    use crate::upgrade_manager::UpgradeManager;
    let env = fresh();
    // Should be a no-op when TOPIC_ALIASES is empty; must not panic.
    UpgradeManager::register_topic_aliases(&env, 1_000_000);
}

// ─────────────────────────────────────────────────────────────────────────────
// 9. Regression: concurrent execution safety
//
// Soroban transactions are single-threaded and deterministic, but partial
// failure (panic mid-transaction) rolls back the whole transaction.  This
// test validates that a failed migration (nonces preserved but restore not
// called) leaves the snapshot accessible for a retry.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn snapshot_survives_failed_restore_and_can_be_retried() {
    let env = fresh();
    write_nonce(&env, "mkt_crt", 55);

    EventNonceGuard::preserve_nonces(&env);

    // Simulate "failed upgrade" — snapshot written but nonce cleared.
    write_nonce(&env, "mkt_crt", 0);

    // The snapshot is still there (not cleared yet).
    let snap = EventNonceGuard::read_snapshot(&env);
    let mkt_snap = snap.iter().find(|s| s.topic == Symbol::new(&env, "mkt_crt"));
    assert!(mkt_snap.is_some(), "snapshot must be readable after failed restore");
    assert_eq!(mkt_snap.unwrap().value, 55);

    // A retry can still restore the nonce.
    EventNonceGuard::restore_nonces(&env);
    assert_eq!(read_nonce(&env, "mkt_crt"), 55);
}

// ─────────────────────────────────────────────────────────────────────────────
// 10. Duplicate / boundary inputs
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn registry_lookup_is_deterministic_across_repeated_calls() {
    let env = fresh();
    let d1 = EventTopicRegistry::get(&env, "market_created").unwrap();
    let d2 = EventTopicRegistry::get(&env, "market_created").unwrap();
    assert_eq!(d1.schema_version, d2.schema_version);
    assert_eq!(d1.topic, d2.topic);
}

#[test]
fn schema_registry_get_schema_same_result_for_repeated_calls() {
    let env = fresh();
    let s1 = EventSchemaRegistry::get_schema(&env, "oracle_result").unwrap();
    let s2 = EventSchemaRegistry::get_schema(&env, "oracle_result").unwrap();
    assert_eq!(s1.schema_version, s2.schema_version);
    assert_eq!(s1.topic, s2.topic);
}

#[test]
fn all_topic_symbols_are_valid_soroban_symbols() {
    let env = fresh();
    // Symbol::new panics on invalid strings; this test will fail if any
    // entry in TOPIC_REGISTRY contains an invalid symbol string.
    for &(sym, _version, _name) in TOPIC_REGISTRY {
        let _s = Symbol::new(&env, sym); // must not panic
    }
}
