//! Comprehensive lifecycle state transition tests for the metadata-only archive
//! and restore functionality.
//!
//! Tests cover:
//! - Archive transitions (success and rejection cases)
//! - Archive discoverability (archived view + preserved terminal status)
//! - Deterministic pruning
//! - Boundary cases and error handling

use predictify_hybrid::{
    Error, EventHistoryEntry, MarketState, OracleConfig, OracleProvider, PredictifyHybrid,
    PredictifyHybridClient,
};
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger, LedgerInfo},
    vec, Address, Env, String as SorobanString, Symbol,
};
use std::vec::Vec as StdVec;

// ===== TEST SETUP =====

struct TestSetup {
    env: Env,
    contract_id: Address,
    admin: Address,
    user1: Address,
}

impl TestSetup {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(PredictifyHybrid, ());
        let admin = Address::generate(&env);
        let user1 = Address::generate(&env);

        // Initialize contract
        let client = PredictifyHybridClient::new(&env, &contract_id);
        client.initialize(&admin, &None, &None);

        TestSetup {
            env,
            contract_id,
            admin,
            user1,
        }
    }

    fn client(&self) -> PredictifyHybridClient {
        PredictifyHybridClient::new(&self.env, &self.contract_id)
    }

    fn advance_days(&self, days: u64) {
        let ledger = self.env.ledger();
        let timestamp = ledger.timestamp() + days * 24 * 60 * 60;
        self.env.ledger().set(LedgerInfo {
            timestamp,
            protocol_version: ledger.protocol_version(),
            sequence_number: ledger.sequence(),
            network_id: ledger.network_id().into(),
            base_reserve: 10,
            min_temp_entry_ttl: 1,
            min_persistent_entry_ttl: 1,
            max_entry_ttl: 1_000_000,
        });
    }

    fn oracle_config(&self) -> OracleConfig {
        OracleConfig {
            provider: OracleProvider::reflector(),
            oracle_address: Address::generate(&self.env),
            feed_id: SorobanString::from_str(&self.env, "BTC"),
            threshold: 50_000_00,
            comparison: SorobanString::from_str(&self.env, "gt"),
        }
    }

    fn outcomes(&self) -> soroban_sdk::Vec<SorobanString> {
        vec![
            &self.env,
            SorobanString::from_str(&self.env, "Yes"),
            SorobanString::from_str(&self.env, "No"),
        ]
    }

    fn create_market(&self, question: &str) -> Symbol {
        self.client().create_market(
            &self.admin,
            &SorobanString::from_str(&self.env, question),
            &self.outcomes(),
            &1, // 1-day duration
            &self.oracle_config(),
            &None, // fallback oracle
            &0,    // resolution timeout
            &None, // min pool size
            &None, // bet deadline
            &None, // dispute window
            &None, // dispute stake floor
            &None, // max participants
        )
    }

    fn create_resolved_market(&self, question: &str) -> Symbol {
        let market_id = self.create_market(question);
        self.advance_days(2); // past the 1-day end
        self.client()
            .resolve_market_manual(&self.admin, &market_id, &SorobanString::from_str(&self.env, "Yes"));
        market_id
    }

    fn create_cancelled_market(&self, question: &str) -> Symbol {
        let market_id = self.create_market(question);
        self.client().cancel_event(&self.admin, &market_id, &None);
        market_id
    }
}

fn collect_ids(entries: &soroban_sdk::Vec<EventHistoryEntry>) -> StdVec<Symbol> {
    entries.iter().map(|e| e.market_id).collect()
}

// ===== ARCHIVE SUCCESS TESTS =====

#[test]
fn test_archive_from_resolved_state() {
    let setup = TestSetup::new();
    let client = setup.client();

    let market_id = setup.create_resolved_market("test_archive_resolved");

    // Archive should succeed from Resolved state
    let result = client.try_archive_event(&setup.admin, &market_id);
    assert!(result.is_ok(), "Archive from Resolved should succeed");
}

#[test]
fn test_archive_from_cancelled_state() {
    let setup = TestSetup::new();
    let client = setup.client();

    let market_id = setup.create_cancelled_market("test_archive_cancelled");

    // Archive should succeed from Cancelled state
    let result = client.try_archive_event(&setup.admin, &market_id);
    assert!(result.is_ok(), "Archive from Cancelled should succeed");
}

#[test]
fn test_archive_emits_event() {
    let setup = TestSetup::new();
    let client = setup.client();

    let market_id = setup.create_resolved_market("test_archive_event");

    // Archive market
    client.archive_event(&setup.admin, &market_id);

    // Check events were emitted
    let events = setup.env.events().all();
    assert!(
        !events.events().is_empty(),
        "Archive should emit events"
    );
}

// ===== ARCHIVE REJECTION TESTS =====

#[test]
fn test_archive_fails_from_active_state() {
    let setup = TestSetup::new();
    let client = setup.client();

    let market_id = setup.create_market("test_archive_active");

    // Archive should fail from Active state
    let result = client.try_archive_event(&setup.admin, &market_id);
    match result {
        Err(Ok(err)) => {
            assert_eq!(
                err,
                Error::CannotArchiveFromState,
                "Should return CannotArchiveFromState error"
            );
        }
        _ => panic!("Expected CannotArchiveFromState error"),
    }
}

#[test]
fn test_archive_duplicate_rejected() {
    let setup = TestSetup::new();
    let client = setup.client();

    let market_id = setup.create_resolved_market("test_archive_duplicate");

    // First archive succeeds
    client.archive_event(&setup.admin, &market_id);

    // Second archive should fail with MarketAlreadyArchived
    let result = client.try_archive_event(&setup.admin, &market_id);
    match result {
        Err(Ok(err)) => {
            assert_eq!(
                err,
                Error::MarketAlreadyArchived,
                "Should return MarketAlreadyArchived error"
            );
        }
        _ => panic!("Expected MarketAlreadyArchived error"),
    }
}

#[test]
fn test_archive_requires_admin_authorization() {
    let setup = TestSetup::new();
    let client = setup.client();

    let market_id = setup.create_resolved_market("test_archive_auth");

    // Archive as non-admin should fail
    let result = client.try_archive_event(&setup.user1, &market_id);
    match result {
        Err(Ok(err)) => {
            assert_eq!(err, Error::Unauthorized, "Should return Unauthorized error");
        }
        _ => panic!("Expected Unauthorized error"),
    }
}

#[test]
fn test_archive_nonexistent_market() {
    let setup = TestSetup::new();
    let client = setup.client();

    let nonexistent_id = Symbol::new(&setup.env, "nonexistent");

    // Archive on nonexistent market should fail
    let result = client.try_archive_event(&setup.admin, &nonexistent_id);
    match result {
        Err(Ok(err)) => {
            assert_eq!(
                err,
                Error::MarketNotFound,
                "Should return MarketNotFound error"
            );
        }
        _ => panic!("Expected MarketNotFound error"),
    }
}

// ===== DISCOVERABILITY TESTS =====

#[test]
fn test_archive_preserves_terminal_state_and_discoverability() {
    let setup = TestSetup::new();
    let client = setup.client();

    let market_id = setup.create_resolved_market("test_state_consistency");

    // Sanity: resolved and discoverable by status before archiving.
    let (before, _) = client.query_events_by_status(&MarketState::Resolved, &0, &30);
    assert!(collect_ids(&before).contains(&market_id));

    // Archive the market (non-destructive).
    client.archive_event(&setup.admin, &market_id);

    // The archived event keeps its terminal Resolved state.
    let (after, _) = client.query_events_by_status(&MarketState::Resolved, &0, &30);
    assert!(
        collect_ids(&after).contains(&market_id),
        "archived (resolved) event must remain discoverable by status"
    );

    // Exposed via the direct archived view.
    let (archived, _) = client.query_archived_events(&false, &0, &30);
    assert!(collect_ids(&archived).contains(&market_id));
}

#[test]
fn test_archived_view_reports_archived_at() {
    let setup = TestSetup::new();
    let client = setup.client();

    let market_id = setup.create_resolved_market("test_archived_at");
    client.archive_event(&setup.admin, &market_id);

    let (archived, _) = client.query_archived_events(&false, &0, &30);
    let entry = archived
        .iter()
        .find(|e| e.market_id == market_id)
        .expect("archived market should be listed");
    assert!(
        entry.archived_at.unwrap_or(0) > 0,
        "archived entry must carry a non-zero archived_at marker"
    );
}

// ===== PRUNING / CAPACITY TESTS =====

#[test]
fn test_archive_capacity_queryable() {
    let setup = TestSetup::new();
    let client = setup.client();

    let size = client.archive_size();
    assert_eq!(size, 0, "fresh deployment should start with an empty archive");
}

#[test]
fn test_multiple_archives_in_sequence() {
    let setup = TestSetup::new();
    let client = setup.client();

    // Create and archive multiple markets
    let m1 = setup.create_resolved_market("test_multi_1");
    let m2 = setup.create_resolved_market("test_multi_2");
    let m3 = setup.create_resolved_market("test_multi_3");

    // Archive all
    client.archive_event(&setup.admin, &m1);
    client.archive_event(&setup.admin, &m2);
    client.archive_event(&setup.admin, &m3);

    // Verify all are archived
    let (archived, _) = client.query_archived_events(&false, &0, &30);
    let ids = collect_ids(&archived);
    assert!(ids.contains(&m1), "Market 1 should be archived");
    assert!(ids.contains(&m2), "Market 2 should be archived");
    assert!(ids.contains(&m3), "Market 3 should be archived");
    assert_eq!(client.archive_size(), 3);
}

#[test]
fn test_prune_archive_reduces_size() {
    let setup = TestSetup::new();
    let client = setup.client();

    for i in 0..3 {
        let mid = setup.create_resolved_market(&format!("test_prune_{i}"));
        client.archive_event(&setup.admin, &mid);
    }
    assert_eq!(client.archive_size(), 3);

    // Prune deterministically from the oldest entry.
    let pruned = match client.try_prune_archive(&setup.admin, &1, &None) {
        Ok(Ok((count, _))) => count,
        _ => panic!("prune should succeed"),
    };
    assert_eq!(pruned, 1, "exactly one entry should be pruned");
    assert_eq!(client.archive_size(), 2);
}
