//! # Archived Event Discoverability — Integration Tests
//!
//! Validates the metadata-only (non-destructive) archive feature:
//!
//! * Archiving a resolved/cancelled market records an `archived_at` metadata
//!   marker and leaves `Market.state` untouched, so the event remains
//!   discoverable by its terminal status via `query_events_by_status`.
//! * `query_archived_events` provides a direct "show me the archive" view
//!   ordered by archived time.
//! * Duplicate archiving is rejected (idempotency), non-terminal markets are
//!   rejected, and pruning is deterministic and capacity-bounded.
//!
//! These are integration tests: they link only the `predictify-hybrid` library
//! (the crate builds cleanly) and drive the public client surface, avoiding the
//! stale inline `#[cfg(test)]` modules.

#![cfg(test)]

use predictify_hybrid::{
    EventHistoryEntry, MarketState, OracleConfig, OracleProvider, PredictifyHybrid,
    PredictifyHybridClient,
};
use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    token::StellarAssetClient,
    vec, Address, Env, String as SorobanString, Symbol,
};
use std::vec::Vec as StdVec;

const INITIAL_BALANCE: i128 = 1_000_000_000_000;

struct Harness {
    env: Env,
    contract_id: Address,
    admin: Address,
}

impl Harness {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let token_admin = Address::generate(&env);
        let token = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_id = token.address();

        let admin = Address::generate(&env);
        let contract_id = env.register(PredictifyHybrid, ());

        // Store the token id for staking/payouts.
        env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .set(&Symbol::new(&env, "TokenID"), &token_id);
        });

        let client = PredictifyHybridClient::new(&env, &contract_id);
        client.initialize(&admin.clone(), &None::<i128>, &None);

        let stellar = StellarAssetClient::new(&env, &token_id);
        stellar.mint(&admin, &INITIAL_BALANCE);

        Self {
            env,
            contract_id,
            admin,
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
            // The rate limiter extends temporary entries to 90_000 ledgers;
            // keep the cap above that so repeated admin ops don't trip
            // `Storage InvalidAction` in the test host.
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

    /// Create a one-day market using the given question string.
    fn create_market(&self, question: &str) -> Symbol {
        PredictifyHybridClient::new(&self.env, &self.contract_id).create_market(
            &self.admin,
            &SorobanString::from_str(&self.env, question),
            &self.outcomes(),
            &1,
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

    /// Create + end + resolve + archive a market, returning its id.
    fn create_resolved_archived(&self, question: &str) -> Symbol {
        let id = self.create_market(question);
        self.advance_days(2); // past 1-day end
        let client = self.client();
        client.resolve_market_manual(&self.admin, &id, &SorobanString::from_str(&self.env, "Yes"));
        client.archive_event(&self.admin, &id);
        id
    }
}

fn collect_ids(entries: &soroban_sdk::Vec<EventHistoryEntry>) -> StdVec<Symbol> {
    entries.iter().map(|e| e.market_id).collect()
}

#[test]
fn archiving_preserves_resolved_state_and_discoverability() {
    let h = Harness::new();
    let client = h.client();

    let mid = h.create_market("Will prices rise?");
    h.advance_days(2);
    client.resolve_market_manual(&h.admin, &mid, &SorobanString::from_str(&h.env, "Yes"));

    // Sanity: resolved and discoverable by status before archiving.
    let (before, _) = client.query_events_by_status(&MarketState::Resolved, &0, &30);
    let ids_before = collect_ids(&before);
    assert!(ids_before.contains(&mid));

    // Archive the resolved market.
    client.archive_event(&h.admin, &mid);

    // Still discoverable by terminal status after archiving (non-destructive).
    let (after, _) = client.query_events_by_status(&MarketState::Resolved, &0, &30);
    let ids_after = collect_ids(&after);
    assert!(
        ids_after.contains(&mid),
        "archived (resolved) event must remain discoverable by status"
    );

    // Exposed via the direct archived view.
    let (archived, _) = client.query_archived_events(&false, &0, &30);
    let archived_ids = collect_ids(&archived);
    assert!(archived_ids.contains(&mid));

    assert_eq!(client.archive_size(), 1);
}

#[test]
fn surviving_rejection_on_non_terminal_and_duplicate() {
    let h = Harness::new();
    let client = h.client();

    // A freshly created (still Active) market may not be archived.
    let active_id = h.create_market("Active market");
    assert!(client.try_archive_event(&h.admin, &active_id).is_err());
    assert_eq!(client.archive_size(), 0);

    // Resolve then archive once; a duplicate archive attempt must fail.
    h.advance_days(2);
    client.resolve_market_manual(&h.admin, &active_id, &SorobanString::from_str(&h.env, "Yes"));
    client.archive_event(&h.admin, &active_id);
    assert_eq!(client.archive_size(), 1);
    assert!(client.try_archive_event(&h.admin, &active_id).is_err());
}

#[test]
fn archived_query_is_ordered_and_paginated() {
    let h = Harness::new();
    let client = h.client();

    // Create two resolved+archived markets, in ascending archive time.
    let a = h.create_resolved_archived("Market Alpha");
    h.advance_days(1);
    let b = h.create_resolved_archived("Market Beta");

    // Ascending: oldest first.
    let (asc, _) = client.query_archived_events(&false, &0, &30);
    let asc_ids = collect_ids(&asc);
    let mut expected_asc = StdVec::new();
    expected_asc.push(a.clone());
    expected_asc.push(b.clone());
    assert_eq!(asc_ids, expected_asc);

    // Descending: newest first.
    let (desc, _) = client.query_archived_events(&true, &0, &30);
    let desc_ids = collect_ids(&desc);
    let mut expected_desc = StdVec::new();
    expected_desc.push(b.clone());
    expected_desc.push(a.clone());
    assert_eq!(desc_ids, expected_desc);
}

#[test]
fn pruning_is_deterministic_and_capacity_bounded() {
    let h = Harness::new();
    let client = h.client();

    // Archive two markets.
    let a = h.create_resolved_archived("Prune Alpha");
    h.advance_days(1);
    let b = h.create_resolved_archived("Prune Beta");

    assert_eq!(client.archive_size(), 2);

    // Prune the oldest 1 entry: A (archived first) must go, B remains.
    let removed = match client.try_prune_archive(&h.admin, &1, &None) {
        Ok(Ok((n, _))) => n,
        _ => panic!("prune_archive failed"),
    };
    assert_eq!(removed, 1);
    assert_eq!(client.archive_size(), 1);

    let (archived, _) = client.query_archived_events(&false, &0, &30);
    let ids = collect_ids(&archived);
    let mut expected = StdVec::new();
    expected.push(b.clone());
    assert_eq!(ids, expected);
}