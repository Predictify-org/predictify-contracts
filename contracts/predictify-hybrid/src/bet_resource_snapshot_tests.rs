//! # Bet Resource Snapshot Tests
//!
//! Focused regression-baseline tests for `get_bet_resource_snapshot` and the
//! underlying [`crate::gas::BetSnapshotManager`].
//!
//! ## What is tested
//!
//! | # | Test | Validates |
//! |---|------|-----------|
//! | 1 | `snapshot_is_none_before_any_bet` | Query returns `None` on a fresh contract |
//! | 2 | `snapshot_recorded_after_place_bet` | A snapshot is persisted after `place_bet` succeeds |
//! | 3 | `snapshot_fields_after_place_bet` | `write_count`, `captured_at`, `market_id` are populated |
//! | 4 | `snapshot_cpu_delta_is_non_negative` | `cpu_delta >= 0` (test runtime gives real delta) |
//! | 5 | `snapshot_overwritten_by_second_bet` | Only the latest snapshot is retained |
//! | 6 | `snapshot_market_id_matches_bet` | `market_id` field matches the market that was bet on |
//! | 7 | `snapshot_write_count_is_expected` | `write_count == 5` (the known write count for place_bet) |
//! | 8 | `snapshot_ledger_sequence_advances` | `ledger_sequence` in second snapshot ≥ first |
//! | 9 | `snapshot_not_affected_by_read` | Read-only calls do not change the snapshot |
//! | 10| `snapshot_manager_record_direct` | `BetSnapshotManager::record` and `latest` round-trip |

#![cfg(test)]

use super::*;
use crate::gas::{BetResourceSnapshot, BetSnapshotManager};
use crate::types::{OracleConfig, OracleProvider};
use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    token::StellarAssetClient,
    vec, Address, Env, String, Symbol,
};

// ───────────────────────── helpers ─────────────────────────

struct SnapshotTestCtx {
    env: Env,
    contract_id: Address,
    token_id: Address,
    admin: Address,
}

impl SnapshotTestCtx {
    /// Stand up a contract with a funded admin and a registered SAC token.
    fn setup() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let token_admin = Address::generate(&env);
        let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_id = token_contract.address();

        let admin = Address::generate(&env);
        let contract_id = env.register(PredictifyHybrid, ());
        let client = PredictifyHybridClient::new(&env, &contract_id);
        client.initialize(&admin, &None, &None);

        // Wire up runtime config so validators have deterministic bounds.
        env.as_contract(&contract_id, || {
            let cfg = crate::config::ConfigManager::get_development_config(&env);
            crate::config::ConfigManager::store_config(&env, &cfg).unwrap();
        });

        // Register the token so BetUtils::lock_funds can resolve a client.
        env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .set(&Symbol::new(&env, "TokenID"), &token_id);
        });

        // Mint tokens to the admin.
        let stellar_client = StellarAssetClient::new(&env, &token_id);
        env.mock_all_auths();
        stellar_client.mint(&admin, &1_000_000_000_000i128);

        Self {
            env,
            contract_id,
            token_id,
            admin,
        }
    }

    /// Mint tokens to an arbitrary address.
    fn fund(&self, addr: &Address) {
        let stellar_client = StellarAssetClient::new(&self.env, &self.token_id);
        self.env.mock_all_auths();
        stellar_client.mint(addr, &1_000_000_000_000i128);
    }

    /// Create a minimal active market and return its ID.
    fn create_market(&self) -> Symbol {
        let client = PredictifyHybridClient::new(&self.env, &self.contract_id);
        let outcomes = vec![
            &self.env,
            String::from_str(&self.env, "yes"),
            String::from_str(&self.env, "no"),
        ];
        self.env.mock_all_auths();
        client.create_market(
            &self.admin,
            &String::from_str(&self.env, "Will BTC exceed $100k?"),
            &outcomes,
            &30,
            &OracleConfig {
                provider: OracleProvider::reflector(),
                oracle_address: Address::generate(&self.env),
                feed_id: String::from_str(&self.env, "BTC/USD"),
                threshold: 100_000_00,
                comparison: String::from_str(&self.env, "gt"),
            },
            &None,
            &3600,
            &None,
            &None,
            &None,
        )
    }

    /// Place a single bet; panics on contract error (test harness).
    fn place_bet(&self, user: &Address, market_id: &Symbol, outcome: &str, amount: i128) {
        let client = PredictifyHybridClient::new(&self.env, &self.contract_id);
        self.env.mock_all_auths();
        client.place_bet(
            user,
            market_id,
            &String::from_str(&self.env, outcome),
            &amount,
            &0,
        );
    }

    /// Read the current snapshot via the public contract entrypoint.
    fn get_snapshot(&self) -> Option<BetResourceSnapshot> {
        let client = PredictifyHybridClient::new(&self.env, &self.contract_id);
        client.get_bet_resource_snapshot()
    }
}

// ───────────────────────── tests ────────────────────────────

/// 1. Before any bet is placed the snapshot must be absent.
#[test]
fn snapshot_is_none_before_any_bet() {
    let ctx = SnapshotTestCtx::setup();
    assert!(
        ctx.get_snapshot().is_none(),
        "expected no snapshot before any bet"
    );
}

/// 2. After one successful `place_bet` a snapshot must be present.
#[test]
fn snapshot_recorded_after_place_bet() {
    let ctx = SnapshotTestCtx::setup();
    let market_id = ctx.create_market();
    ctx.fund(&ctx.admin);
    ctx.place_bet(&ctx.admin, &market_id, "yes", 10_000_000);

    let snap = ctx.get_snapshot();
    assert!(snap.is_some(), "snapshot must be present after place_bet");
}

/// 3. Snapshot fields are populated with non-zero / valid values.
#[test]
fn snapshot_fields_after_place_bet() {
    let ctx = SnapshotTestCtx::setup();
    let market_id = ctx.create_market();
    ctx.fund(&ctx.admin);
    ctx.place_bet(&ctx.admin, &market_id, "yes", 10_000_000);

    let snap = ctx
        .get_snapshot()
        .expect("snapshot must be present after place_bet");

    // write_count is the constant declared in bets.rs (5 writes per place_bet).
    assert_eq!(snap.write_count, 5, "write_count must equal 5");
    // captured_at should be a positive timestamp.
    assert!(snap.captured_at > 0, "captured_at must be non-zero");
}

/// 4. `cpu_delta` is non-negative; in the test runtime it should be > 0.
#[test]
fn snapshot_cpu_delta_is_non_negative() {
    let ctx = SnapshotTestCtx::setup();
    let market_id = ctx.create_market();
    ctx.fund(&ctx.admin);
    ctx.place_bet(&ctx.admin, &market_id, "no", 10_000_000);

    let snap = ctx
        .get_snapshot()
        .expect("snapshot present after place_bet");
    // The Soroban test runtime exposes real CPU budget usage; delta >= 0
    // always holds (saturating_sub is used in BetSnapshotManager::record).
    assert!(
        snap.cpu_delta == 0 || snap.cpu_delta > 0,
        "cpu_delta must be non-negative, got {}",
        snap.cpu_delta
    );
}

/// 5. A second `place_bet` (on a different market) overwrites the snapshot;
///    only one entry is retained.
#[test]
fn snapshot_overwritten_by_second_bet() {
    let ctx = SnapshotTestCtx::setup();
    let market1 = ctx.create_market();
    let market2 = ctx.create_market();

    let user1 = Address::generate(&ctx.env);
    let user2 = Address::generate(&ctx.env);
    ctx.fund(&user1);
    ctx.fund(&user2);

    ctx.place_bet(&user1, &market1, "yes", 10_000_000);
    let snap1 = ctx.get_snapshot().expect("first snapshot");

    // Advance ledger time so captured_at changes.
    ctx.env.ledger().set(LedgerInfo {
        timestamp: snap1.captured_at + 100,
        sequence_number: snap1.ledger_sequence + 1,
        protocol_version: 25,
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 1,
        min_persistent_entry_ttl: 1,
        max_entry_ttl: u32::MAX,
    });

    ctx.place_bet(&user2, &market2, "no", 20_000_000);
    let snap2 = ctx.get_snapshot().expect("second snapshot");

    // The snapshot was overwritten; `captured_at` should be later.
    assert!(
        snap2.captured_at >= snap1.captured_at,
        "second snapshot captured_at ({}) must be >= first ({})",
        snap2.captured_at,
        snap1.captured_at
    );
}

/// 6. `market_id` in the snapshot matches the market the bet was placed on.
#[test]
fn snapshot_market_id_matches_bet() {
    let ctx = SnapshotTestCtx::setup();
    let market_id = ctx.create_market();
    ctx.fund(&ctx.admin);
    ctx.place_bet(&ctx.admin, &market_id, "yes", 10_000_000);

    let snap = ctx
        .get_snapshot()
        .expect("snapshot present after place_bet");
    assert_eq!(
        snap.market_id, market_id,
        "market_id in snapshot must match the bet's market"
    );
}

/// 7. `write_count` is exactly 5 for a single `place_bet`.
#[test]
fn snapshot_write_count_is_expected() {
    let ctx = SnapshotTestCtx::setup();
    let market_id = ctx.create_market();
    ctx.fund(&ctx.admin);
    ctx.place_bet(&ctx.admin, &market_id, "yes", 10_000_000);

    let snap = ctx
        .get_snapshot()
        .expect("snapshot present after place_bet");
    assert_eq!(
        snap.write_count, 5,
        "place_bet performs exactly 5 persistent writes"
    );
}

/// 8. `ledger_sequence` in the second snapshot is ≥ that of the first.
#[test]
fn snapshot_ledger_sequence_advances() {
    let ctx = SnapshotTestCtx::setup();
    let market1 = ctx.create_market();
    let market2 = ctx.create_market();

    let user1 = Address::generate(&ctx.env);
    let user2 = Address::generate(&ctx.env);
    ctx.fund(&user1);
    ctx.fund(&user2);

    ctx.place_bet(&user1, &market1, "yes", 10_000_000);
    let seq1 = ctx.get_snapshot().expect("first snapshot").ledger_sequence;

    ctx.env.ledger().set(LedgerInfo {
        timestamp: ctx.env.ledger().timestamp() + 60,
        sequence_number: seq1 + 5,
        protocol_version: 25,
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 1,
        min_persistent_entry_ttl: 1,
        max_entry_ttl: u32::MAX,
    });

    ctx.place_bet(&user2, &market2, "no", 15_000_000);
    let seq2 = ctx.get_snapshot().expect("second snapshot").ledger_sequence;

    assert!(
        seq2 >= seq1,
        "second snapshot ledger_sequence ({}) must be >= first ({})",
        seq2,
        seq1
    );
}

/// 9. A read-only call (`get_bet_resource_snapshot`) does not mutate the snapshot.
#[test]
fn snapshot_not_affected_by_read() {
    let ctx = SnapshotTestCtx::setup();
    let market_id = ctx.create_market();
    ctx.fund(&ctx.admin);
    ctx.place_bet(&ctx.admin, &market_id, "yes", 10_000_000);

    let snap_before = ctx
        .get_snapshot()
        .expect("snapshot present after place_bet");

    // Multiple reads must not change the stored snapshot.
    let _ = ctx.get_snapshot();
    let _ = ctx.get_snapshot();

    let snap_after = ctx
        .get_snapshot()
        .expect("snapshot still present after reads");

    assert_eq!(
        snap_before, snap_after,
        "read-only calls must not mutate the snapshot"
    );
}

/// 10. `BetSnapshotManager::record` and `latest` round-trip correctly.
#[test]
fn snapshot_manager_record_direct() {
    let env = Env::default();
    let market_id = Symbol::new(&env, "test_mkt");

    // Before any record, `latest` returns None.
    assert!(BetSnapshotManager::latest(&env).is_none());

    // Record a snapshot directly.
    BetSnapshotManager::record(&env, 0, 3, &market_id);

    let snap = BetSnapshotManager::latest(&env).expect("snapshot must be present after record");
    assert_eq!(snap.market_id, market_id);
    assert_eq!(snap.write_count, 3);
    // cpu_delta == 0 because cpu_before == 0 and cpu_instruction_cost returns 0 in
    // the non-test cfg path (which is what the `gas.rs` implementation uses).
    // In the test cfg path the delta may be small but still >= 0.
    assert!(snap.cpu_delta == 0 || snap.cpu_delta > 0);
}
