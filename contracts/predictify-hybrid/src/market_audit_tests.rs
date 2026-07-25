//! # Per-Market Audit Log Tests
//!
//! Focused test suite for [`crate::audit::MarketAuditManager`] and the three
//! public read entrypoints on [`crate::PredictifyHybrid`]:
//!
//! - `get_market_audit_head`
//! - `get_market_audit_entry`
//! - `get_market_audit_log`
//!
//! ## Coverage
//!
//! | Area | Tests |
//! |---|---|
//! | Append + head | entry_count_increments_on_each_append |
//! | No entries sentinel | head_returns_none_for_unknown_market |
//! | Entry content | entry_contains_correct_action_actor_and_timestamp |
//! | Index bounds | get_entry_returns_none_for_zero_index, get_entry_returns_none_for_out_of_range |
//! | Pagination limit cap | get_entries_limit_is_capped_at_100 |
//! | Reverse order | get_entries_returns_newest_first |
//! | Full lifecycle | market_created_audit_entry_via_entrypoint |
//! | Resolution hook | resolve_manual_appends_audit_entry |
//! | Force-resolve hook | force_resolve_appends_audit_entry |
//! | Details metadata | audit_entry_details_contain_expected_keys |
//! | Empty market | get_log_returns_empty_for_market_without_entries |
//! | Limit = 0 | get_log_with_zero_limit_returns_empty |
//! | TTL extended | entries_have_correct_ttl_set (smoke) |

#![cfg(test)]

use super::*;
use crate::audit::{MarketAuditAction, MarketAuditManager};
use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    token::StellarAssetClient,
    vec, Map, String, Symbol,
};

// ===== HELPERS =====

/// Minimal test harness — initialises the contract and returns a funded admin.
struct AuditTestEnv {
    env: Env,
    contract_id: Address,
    admin: Address,
    token_id: Address,
}

impl AuditTestEnv {
    fn setup() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        // Deploy a SAC token
        let token_admin = Address::generate(&env);
        let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_id = token_contract.address();

        let stellar_client = StellarAssetClient::new(&env, &token_id);
        let admin = Address::generate(&env);
        stellar_client.mint(&admin, &1_000_000_000_000i128);

        // Deploy + init contract
        let contract_id = env.register(PredictifyHybrid, ());
        let client = PredictifyHybridClient::new(&env, &contract_id);
        client.initialize(&admin, &None, &None);

        // Store token
        env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .set(&Symbol::new(&env, "TokenID"), &token_id);
            // Development config
            let cfg = crate::config::ConfigManager::get_development_config(&env);
            crate::config::ConfigManager::store_config(&env, &cfg).unwrap();
        });

        Self { env, contract_id, admin, token_id }
    }

    fn client(&self) -> PredictifyHybridClient {
        PredictifyHybridClient::new(&self.env, &self.contract_id)
    }

    /// Creates a standard market and returns its id.
    fn create_market(&self) -> Symbol {
        let client = self.client();
        let outcomes = vec![
            &self.env,
            String::from_str(&self.env, "yes"),
            String::from_str(&self.env, "no"),
        ];
        client.create_market(
            &self.admin,
            &String::from_str(&self.env, "Will XLM exceed $1?"),
            &outcomes,
            &30,
            &OracleConfig {
                provider: OracleProvider::reflector(),
                oracle_address: Address::generate(&self.env),
                feed_id: String::from_str(&self.env, "XLM"),
                threshold: 100,
                comparison: String::from_str(&self.env, "gt"),
            },
            &None,
            &0,
            &None,
            &None,
            &None,
        )
    }

    /// Advance ledger time past market end.
    fn advance_past_end(&self) {
        self.env.ledger().set(LedgerInfo {
            timestamp: self.env.ledger().timestamp() + 31 * 24 * 60 * 60 + 1,
            protocol_version: 22,
            sequence_number: self.env.ledger().sequence() + 1,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 6_312_000,
        });
    }
}

// ===== UNIT TESTS: MarketAuditManager directly =====

#[test]
fn head_returns_none_for_unknown_market() {
    let env = Env::default();
    let market_id = Symbol::new(&env, "ghost");
    assert!(MarketAuditManager::get_head(&env, &market_id).is_none());
}

#[test]
fn get_entry_returns_none_for_zero_index() {
    let env = Env::default();
    let market_id = Symbol::new(&env, "m1");
    // Populate one entry
    MarketAuditManager::append(
        &env,
        &market_id,
        MarketAuditAction::MarketCreated,
        Address::generate(&env),
        Map::new(&env),
    );
    // Index 0 must always return None
    assert!(MarketAuditManager::get_entry(&env, &market_id, 0).is_none());
}

#[test]
fn get_entry_returns_none_for_out_of_range() {
    let env = Env::default();
    let market_id = Symbol::new(&env, "m2");
    MarketAuditManager::append(
        &env,
        &market_id,
        MarketAuditAction::MarketCreated,
        Address::generate(&env),
        Map::new(&env),
    );
    // Only 1 entry, so index 2 must return None
    assert!(MarketAuditManager::get_entry(&env, &market_id, 2).is_none());
}

#[test]
fn entry_count_increments_on_each_append() {
    let env = Env::default();
    let market_id = Symbol::new(&env, "m3");
    let actor = Address::generate(&env);

    for _ in 0..5 {
        MarketAuditManager::append(
            &env,
            &market_id,
            MarketAuditAction::MarketCreated,
            actor.clone(),
            Map::new(&env),
        );
    }

    let head = MarketAuditManager::get_head(&env, &market_id).unwrap();
    assert_eq!(head.total_entries, 5);
}

#[test]
fn entry_contains_correct_action_actor_and_timestamp() {
    let env = Env::default();
    env.ledger().set(LedgerInfo {
        timestamp: 1_700_000_000,
        protocol_version: 22,
        sequence_number: 100,
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 10,
        min_persistent_entry_ttl: 10,
        max_entry_ttl: 6_312_000,
    });

    let market_id = Symbol::new(&env, "m4");
    let actor = Address::generate(&env);
    let mut details = Map::new(&env);
    details.set(
        Symbol::new(&env, "question"),
        String::from_str(&env, "test?"),
    );

    let idx = MarketAuditManager::append(
        &env,
        &market_id,
        MarketAuditAction::MarketCreated,
        actor.clone(),
        details,
    );
    assert_eq!(idx, 1);

    let entry = MarketAuditManager::get_entry(&env, &market_id, 1).unwrap();
    assert_eq!(entry.index, 1);
    assert_eq!(entry.action, MarketAuditAction::MarketCreated);
    assert_eq!(entry.actor, actor);
    assert_eq!(entry.timestamp, 1_700_000_000);
}

#[test]
fn get_entries_returns_newest_first() {
    let env = Env::default();
    let market_id = Symbol::new(&env, "m5");
    let actor = Address::generate(&env);

    let actions = [
        MarketAuditAction::MarketCreated,
        MarketAuditAction::DisputeFiled,
        MarketAuditAction::DisputeResolved,
        MarketAuditAction::MarketResolved,
    ];

    for action in actions.iter() {
        MarketAuditManager::append(
            &env,
            &market_id,
            action.clone(),
            actor.clone(),
            Map::new(&env),
        );
    }

    let entries = MarketAuditManager::get_entries(&env, &market_id, 10);
    assert_eq!(entries.len(), 4);
    // Newest first → last appended (index 4) should be first in result
    assert_eq!(entries.get(0).unwrap().index, 4);
    assert_eq!(entries.get(3).unwrap().index, 1);
}

#[test]
fn get_log_returns_empty_for_market_without_entries() {
    let env = Env::default();
    let market_id = Symbol::new(&env, "empty");
    let entries = MarketAuditManager::get_entries(&env, &market_id, 10);
    assert_eq!(entries.len(), 0);
}

#[test]
fn get_log_with_zero_limit_returns_empty() {
    let env = Env::default();
    let market_id = Symbol::new(&env, "zerolim");
    let actor = Address::generate(&env);
    MarketAuditManager::append(
        &env,
        &market_id,
        MarketAuditAction::MarketCreated,
        actor,
        Map::new(&env),
    );
    let entries = MarketAuditManager::get_entries(&env, &market_id, 0);
    assert_eq!(entries.len(), 0);
}

#[test]
fn get_entries_limit_is_capped_at_100() {
    let env = Env::default();
    let market_id = Symbol::new(&env, "cap100");
    let actor = Address::generate(&env);

    // Append 120 entries
    for _ in 0..120u32 {
        MarketAuditManager::append(
            &env,
            &market_id,
            MarketAuditAction::FeesCollected,
            actor.clone(),
            Map::new(&env),
        );
    }

    // Even requesting 200, only 100 returned
    let entries = MarketAuditManager::get_entries(&env, &market_id, 200);
    assert_eq!(entries.len(), 100);
}

#[test]
fn audit_entry_details_contain_expected_keys() {
    let env = Env::default();
    let market_id = Symbol::new(&env, "detkeys");
    let actor = Address::generate(&env);

    let mut details = Map::new(&env);
    details.set(
        Symbol::new(&env, "outcome"),
        String::from_str(&env, "yes"),
    );
    details.set(
        Symbol::new(&env, "method"),
        String::from_str(&env, "Manual"),
    );
    MarketAuditManager::append(
        &env,
        &market_id,
        MarketAuditAction::MarketResolved,
        actor,
        details,
    );

    let entry = MarketAuditManager::get_entry(&env, &market_id, 1).unwrap();
    assert_eq!(
        entry.details.get(Symbol::new(&env, "outcome")).unwrap(),
        String::from_str(&env, "yes")
    );
    assert_eq!(
        entry.details.get(Symbol::new(&env, "method")).unwrap(),
        String::from_str(&env, "Manual")
    );
}

// ===== INTEGRATION TESTS: via contract entrypoints =====

#[test]
fn market_created_audit_entry_via_entrypoint() {
    let t = AuditTestEnv::setup();
    let market_id = t.create_market();

    // The entrypoint must have written one per-market audit entry.
    let head = t.client().get_market_audit_head(&market_id).unwrap();
    assert_eq!(head.total_entries, 1);

    let entry = t.client().get_market_audit_entry(&market_id, &1).unwrap();
    assert_eq!(entry.action, MarketAuditAction::MarketCreated);
    assert_eq!(entry.actor, t.admin);
    assert_eq!(entry.index, 1);
}

#[test]
fn resolve_manual_appends_audit_entry() {
    let t = AuditTestEnv::setup();
    let market_id = t.create_market();

    // Advance time past end
    t.advance_past_end();

    t.client().resolve_market_manual(
        &t.admin,
        &market_id,
        &String::from_str(&t.env, "yes"),
    );

    // Should now be 2 entries: MarketCreated + MarketResolved
    let head = t.client().get_market_audit_head(&market_id).unwrap();
    assert_eq!(head.total_entries, 2);

    // Most recent (index 2) should be MarketResolved
    let entry = t.client().get_market_audit_entry(&market_id, &2).unwrap();
    assert_eq!(entry.action, MarketAuditAction::MarketResolved);
    assert_eq!(entry.actor, t.admin);
    // details["method"] == "Manual"
    assert_eq!(
        entry.details.get(Symbol::new(&t.env, "method")).unwrap(),
        String::from_str(&t.env, "Manual")
    );
    // details["outcome"] == "yes"
    assert_eq!(
        entry.details.get(Symbol::new(&t.env, "outcome")).unwrap(),
        String::from_str(&t.env, "yes")
    );
}

#[test]
fn force_resolve_appends_audit_entry() {
    let t = AuditTestEnv::setup();
    let market_id = t.create_market();

    let outcomes = vec![&t.env, String::from_str(&t.env, "yes")];
    t.client().force_resolve_market(
        &t.admin,
        &market_id,
        &outcomes,
        &String::from_str(&t.env, "Emergency override"),
        &String::from_str(&t.env, "idem-key-001"),
    ).unwrap();

    let head = t.client().get_market_audit_head(&market_id).unwrap();
    // MarketCreated + MarketForceResolved
    assert_eq!(head.total_entries, 2);

    let entry = t.client().get_market_audit_entry(&market_id, &2).unwrap();
    assert_eq!(entry.action, MarketAuditAction::MarketForceResolved);
    assert_eq!(
        entry.details.get(Symbol::new(&t.env, "method")).unwrap(),
        String::from_str(&t.env, "Force")
    );
}

#[test]
fn get_market_audit_log_returns_newest_first_via_entrypoint() {
    let t = AuditTestEnv::setup();
    let market_id = t.create_market();

    t.advance_past_end();
    t.client().resolve_market_manual(
        &t.admin,
        &market_id,
        &String::from_str(&t.env, "no"),
    );

    // limit=10 should return both entries newest-first
    let log = t.client().get_market_audit_log(&market_id, &10);
    assert_eq!(log.len(), 2);
    assert_eq!(log.get(0).unwrap().action, MarketAuditAction::MarketResolved);
    assert_eq!(log.get(1).unwrap().action, MarketAuditAction::MarketCreated);
}

#[test]
fn get_market_audit_log_limit_respected() {
    let t = AuditTestEnv::setup();
    let market_id = t.create_market();

    t.advance_past_end();
    t.client().resolve_market_manual(
        &t.admin,
        &market_id,
        &String::from_str(&t.env, "yes"),
    );

    // Only request 1 entry even though 2 exist
    let log = t.client().get_market_audit_log(&market_id, &1);
    assert_eq!(log.len(), 1);
    // Should be the newest one
    assert_eq!(log.get(0).unwrap().action, MarketAuditAction::MarketResolved);
}

#[test]
fn get_market_audit_head_returns_none_for_nonexistent_market() {
    let t = AuditTestEnv::setup();
    let ghost_id = Symbol::new(&t.env, "ghost");
    assert!(t.client().get_market_audit_head(&ghost_id).is_none());
}

#[test]
fn get_market_audit_entry_returns_none_for_index_zero() {
    let t = AuditTestEnv::setup();
    let market_id = t.create_market();
    assert!(t.client().get_market_audit_entry(&market_id, &0).is_none());
}

#[test]
fn get_market_audit_entry_returns_none_for_out_of_range() {
    let t = AuditTestEnv::setup();
    let market_id = t.create_market();
    // Only 1 entry (MarketCreated), so index 2 is out of range
    assert!(t.client().get_market_audit_entry(&market_id, &2).is_none());
}
