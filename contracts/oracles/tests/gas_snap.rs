//! Oracle gas/resource snapshot tests — regression baseline.
//!
//! # Purpose
//!
//! Each test invokes one oracle entrypoint, reads the Soroban mock-host
//! budget counters **before** and **after**, and asserts that the delta
//! stays within a conservative named threshold constant.  The thresholds
//! form the regression baseline: a PR that causes any delta to exceed its
//! threshold must justify the increase or tighten the constant.
//!
//! # Methodology
//!
//! [`Env::budget()`] (available under the `testutils` feature) exposes:
//! - [`Budget::cpu_instruction_cost()`] — cumulative CPU instructions consumed
//! - [`Budget::memory_bytes_cost()`]    — cumulative memory bytes consumed
//!
//! We snapshot both before and after each call and assert on the *delta*.
//! The mock host is deterministic for a fixed operation sequence, so deltas
//! are stable across machines and CI runs.
//!
//! # Scope
//!
//! Tests cover every entrypoint for both the *happy path* (no error) and the
//! *cheap error path* (oracle not registered → `InvalidOracleConfig`).
//! The `get_price`, `get_price_data`, and `is_oracle_healthy` entrypoints
//! call into a second contract via `env.invoke_contract`; in the mock
//! environment invoking an address that has no contract behind it panics at
//! the host level, so those cross-contract paths are not exercised here —
//! only the registry-validation fast-path is measured.
//!
//! # Threshold Rationale
//!
//! | Entrypoint           | CPU threshold | Mem threshold | Notes                         |
//! |----------------------|--------------|--------------|-------------------------------|
//! | `add_oracle`         | 500 000      | 150 000      | 1 read + 1 write + auth check |
//! | `remove_oracle`      | 500 000      | 150 000      | 1 read + 1 write + auth check |
//! | `list_oracles`       | 200 000      |  60 000      | 1 read, no auth               |
//! | `get_price`          | 300 000      |  80 000      | 1 read, early-exit on no-reg  |
//! | `get_price_data`     | 300 000      |  80 000      | 1 read, early-exit on no-reg  |
//! | `is_oracle_healthy`  | 300 000      |  80 000      | 1 read, early-exit on no-reg  |
//!
//! Values are set at roughly 3× the measured mock-host delta to allow
//! headroom for SDK patch upgrades while still catching regressions.
//! Tighten them once `stellar contract invoke --cost` p99 values are
//! available from a production deployment.

use oracles::{Error, OraclesContract, OraclesContractClient};
use soroban_sdk::{
    testutils::{Address as _, Budget},
    Address, Env, String,
};

// -----------------------------------------------------------------------
// CPU-instruction thresholds (upper bound per call)
// -----------------------------------------------------------------------

/// Maximum CPU instructions for a single `add_oracle` call.
const ADD_ORACLE_CPU: u64 = 500_000;
/// Maximum CPU instructions for a single `remove_oracle` call.
const REMOVE_ORACLE_CPU: u64 = 500_000;
/// Maximum CPU instructions for a single `list_oracles` call.
const LIST_ORACLES_CPU: u64 = 200_000;
/// Maximum CPU instructions for a single `get_price` call (early-exit path).
const GET_PRICE_CPU: u64 = 300_000;
/// Maximum CPU instructions for a single `get_price_data` call (early-exit path).
const GET_PRICE_DATA_CPU: u64 = 300_000;
/// Maximum CPU instructions for a single `is_oracle_healthy` call (early-exit path).
const IS_ORACLE_HEALTHY_CPU: u64 = 300_000;

// -----------------------------------------------------------------------
// Memory-byte thresholds (upper bound per call)
// -----------------------------------------------------------------------

/// Maximum memory bytes for a single `add_oracle` call.
const ADD_ORACLE_MEM: u64 = 150_000;
/// Maximum memory bytes for a single `remove_oracle` call.
const REMOVE_ORACLE_MEM: u64 = 150_000;
/// Maximum memory bytes for a single `list_oracles` call.
const LIST_ORACLES_MEM: u64 = 60_000;
/// Maximum memory bytes for a single `get_price` call (early-exit path).
const GET_PRICE_MEM: u64 = 80_000;
/// Maximum memory bytes for a single `get_price_data` call (early-exit path).
const GET_PRICE_DATA_MEM: u64 = 80_000;
/// Maximum memory bytes for a single `is_oracle_healthy` call (early-exit path).
const IS_ORACLE_HEALTHY_MEM: u64 = 80_000;

// -----------------------------------------------------------------------
// Budget helpers
// -----------------------------------------------------------------------

/// Snapshot the current cumulative budget counters.
///
/// Returns `(cpu_instructions, memory_bytes)`.
fn snap(env: &Env) -> (u64, u64) {
    let b = env.budget();
    (b.cpu_instruction_cost(), b.memory_bytes_cost())
}

/// Compute the per-call delta between two snapshots.
///
/// Uses saturating subtraction so a counter that resets (or snapshots taken
/// in the wrong order) yields 0 rather than wrapping.
fn delta(before: (u64, u64), after: (u64, u64)) -> (u64, u64) {
    (
        after.0.saturating_sub(before.0),
        after.1.saturating_sub(before.1),
    )
}

/// Assert both deltas are within their thresholds, naming the entrypoint
/// so CI output is immediately actionable.
fn assert_budget(label: &str, cpu: u64, mem: u64, cpu_max: u64, mem_max: u64) {
    assert!(
        cpu <= cpu_max,
        "{label}: CPU {cpu} > threshold {cpu_max}"
    );
    assert!(
        mem <= mem_max,
        "{label}: mem {mem} > threshold {mem_max}"
    );
}

// -----------------------------------------------------------------------
// Test fixture
// -----------------------------------------------------------------------

/// Minimal per-test fixture: a fresh environment, registered contract, and
/// a generated admin address with all auths mocked.
struct Fx {
    env: Env,
    admin: Address,
    contract_id: Address,
}

impl Fx {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(OraclesContract, ());
        let admin = Address::generate(&env);
        Self { env, admin, contract_id }
    }

    fn client(&self) -> OraclesContractClient<'_> {
        OraclesContractClient::new(&self.env, &self.contract_id)
    }

    /// Register an oracle and return its address.
    fn add(&self, oracle: &Address) {
        self.client().add_oracle(&self.admin, oracle);
    }
}

// -----------------------------------------------------------------------
// add_oracle
// -----------------------------------------------------------------------

/// Snapshot: first registration — list was empty, one write appended.
#[test]
fn snap_add_oracle_first() {
    let f = Fx::new();
    let oracle = Address::generate(&f.env);

    let b = snap(&f.env);
    f.client().add_oracle(&f.admin, &oracle);
    let (cpu, mem) = delta(b, snap(&f.env));

    assert_budget("add_oracle/first", cpu, mem, ADD_ORACLE_CPU, ADD_ORACLE_MEM);
}

/// Snapshot: duplicate registration — early return, no second write.
///
/// Cost must be ≤ the first-registration ceiling even though no write occurs.
#[test]
fn snap_add_oracle_duplicate() {
    let f = Fx::new();
    let oracle = Address::generate(&f.env);
    f.add(&oracle);

    let b = snap(&f.env);
    f.client().add_oracle(&f.admin, &oracle);
    let (cpu, mem) = delta(b, snap(&f.env));

    assert_budget("add_oracle/duplicate", cpu, mem, ADD_ORACLE_CPU, ADD_ORACLE_MEM);
}

/// Snapshot: add to a non-empty list (3 existing entries).
///
/// The linear dedup scan scales with list size; three entries is a
/// representative small-list baseline.
#[test]
fn snap_add_oracle_with_existing() {
    let f = Fx::new();
    for _ in 0..3 {
        f.add(&Address::generate(&f.env));
    }
    let new_oracle = Address::generate(&f.env);

    let b = snap(&f.env);
    f.client().add_oracle(&f.admin, &new_oracle);
    let (cpu, mem) = delta(b, snap(&f.env));

    assert_budget("add_oracle/with_existing", cpu, mem, ADD_ORACLE_CPU, ADD_ORACLE_MEM);
}

// -----------------------------------------------------------------------
// remove_oracle
// -----------------------------------------------------------------------

/// Snapshot: remove a present oracle — one read, one write (filtered list).
#[test]
fn snap_remove_oracle_present() {
    let f = Fx::new();
    let oracle = Address::generate(&f.env);
    f.add(&oracle);

    let b = snap(&f.env);
    f.client().remove_oracle(&f.admin, &oracle);
    let (cpu, mem) = delta(b, snap(&f.env));

    assert_budget("remove_oracle/present", cpu, mem, REMOVE_ORACLE_CPU, REMOVE_ORACLE_MEM);
}

/// Snapshot: remove an oracle that was never registered (no-op write).
///
/// The contract iterates the full list and writes it back unchanged.
#[test]
fn snap_remove_oracle_absent() {
    let f = Fx::new();
    f.add(&Address::generate(&f.env)); // non-empty list
    let unknown = Address::generate(&f.env);

    let b = snap(&f.env);
    f.client().remove_oracle(&f.admin, &unknown);
    let (cpu, mem) = delta(b, snap(&f.env));

    assert_budget("remove_oracle/absent", cpu, mem, REMOVE_ORACLE_CPU, REMOVE_ORACLE_MEM);
}

/// Snapshot: remove against an empty registry — storage key absent, writes empty list.
#[test]
fn snap_remove_oracle_empty_registry() {
    let f = Fx::new();
    let oracle = Address::generate(&f.env);

    let b = snap(&f.env);
    f.client().remove_oracle(&f.admin, &oracle);
    let (cpu, mem) = delta(b, snap(&f.env));

    assert_budget("remove_oracle/empty_registry", cpu, mem, REMOVE_ORACLE_CPU, REMOVE_ORACLE_MEM);
}

// -----------------------------------------------------------------------
// list_oracles
// -----------------------------------------------------------------------

/// Snapshot: list with no registered oracles (storage key absent → empty Vec).
#[test]
fn snap_list_oracles_empty() {
    let f = Fx::new();

    let b = snap(&f.env);
    let list = f.client().list_oracles();
    let (cpu, mem) = delta(b, snap(&f.env));

    assert_eq!(list.len(), 0);
    assert_budget("list_oracles/empty", cpu, mem, LIST_ORACLES_CPU, LIST_ORACLES_MEM);
}

/// Snapshot: list with one registered oracle.
#[test]
fn snap_list_oracles_one() {
    let f = Fx::new();
    f.add(&Address::generate(&f.env));

    let b = snap(&f.env);
    let list = f.client().list_oracles();
    let (cpu, mem) = delta(b, snap(&f.env));

    assert_eq!(list.len(), 1);
    assert_budget("list_oracles/one", cpu, mem, LIST_ORACLES_CPU, LIST_ORACLES_MEM);
}

/// Snapshot: list with three registered oracles.
///
/// `list_oracles` is a single storage read regardless of list length; the
/// threshold is shared with the one-entry case.
#[test]
fn snap_list_oracles_three() {
    let f = Fx::new();
    for _ in 0..3 {
        f.add(&Address::generate(&f.env));
    }

    let b = snap(&f.env);
    let list = f.client().list_oracles();
    let (cpu, mem) = delta(b, snap(&f.env));

    assert_eq!(list.len(), 3);
    assert_budget("list_oracles/three", cpu, mem, LIST_ORACLES_CPU, LIST_ORACLES_MEM);
}

// -----------------------------------------------------------------------
// get_price  (early-exit: unregistered oracle → InvalidOracleConfig)
// -----------------------------------------------------------------------

/// Snapshot: `get_price` against an oracle not in the registry.
///
/// The contract performs one storage read, finds the oracle absent, and
/// returns `Err(InvalidOracleConfig)` without making a cross-contract call.
/// This is the cheapest measurable path.
#[test]
fn snap_get_price_unregistered() {
    let f = Fx::new();
    let unregistered = Address::generate(&f.env);
    let feed = String::from_str(&f.env, "BTC/USD");

    let b = snap(&f.env);
    let result = f.client().try_get_price(&unregistered, &feed);
    let (cpu, mem) = delta(b, snap(&f.env));

    assert_eq!(
        result,
        Err(Ok(Error::InvalidOracleConfig)),
        "expected InvalidOracleConfig for unregistered oracle"
    );
    assert_budget("get_price/unregistered", cpu, mem, GET_PRICE_CPU, GET_PRICE_MEM);
}

/// Snapshot: `get_price` — empty registry, same early-exit path.
#[test]
fn snap_get_price_empty_registry() {
    let f = Fx::new();
    let oracle = Address::generate(&f.env);
    let feed = String::from_str(&f.env, "ETH/USD");

    let b = snap(&f.env);
    let result = f.client().try_get_price(&oracle, &feed);
    let (cpu, mem) = delta(b, snap(&f.env));

    assert_eq!(result, Err(Ok(Error::InvalidOracleConfig)));
    assert_budget("get_price/empty_registry", cpu, mem, GET_PRICE_CPU, GET_PRICE_MEM);
}

// -----------------------------------------------------------------------
// get_price_data  (early-exit: unregistered oracle → InvalidOracleConfig)
// -----------------------------------------------------------------------

/// Snapshot: `get_price_data` against an unregistered oracle.
///
/// Same early-exit as `get_price` — one storage read, no cross-contract call.
#[test]
fn snap_get_price_data_unregistered() {
    let f = Fx::new();
    let unregistered = Address::generate(&f.env);
    let feed = String::from_str(&f.env, "BTC/USD");

    let b = snap(&f.env);
    let result = f.client().try_get_price_data(&unregistered, &feed);
    let (cpu, mem) = delta(b, snap(&f.env));

    assert_eq!(
        result,
        Err(Ok(Error::InvalidOracleConfig)),
        "expected InvalidOracleConfig for unregistered oracle"
    );
    assert_budget("get_price_data/unregistered", cpu, mem, GET_PRICE_DATA_CPU, GET_PRICE_DATA_MEM);
}

/// Snapshot: `get_price_data` — empty registry.
#[test]
fn snap_get_price_data_empty_registry() {
    let f = Fx::new();
    let oracle = Address::generate(&f.env);
    let feed = String::from_str(&f.env, "ETH/USD");

    let b = snap(&f.env);
    let result = f.client().try_get_price_data(&oracle, &feed);
    let (cpu, mem) = delta(b, snap(&f.env));

    assert_eq!(result, Err(Ok(Error::InvalidOracleConfig)));
    assert_budget("get_price_data/empty_registry", cpu, mem, GET_PRICE_DATA_CPU, GET_PRICE_DATA_MEM);
}

// -----------------------------------------------------------------------
// is_oracle_healthy  (early-exit: unregistered oracle → InvalidOracleConfig)
// -----------------------------------------------------------------------

/// Snapshot: `is_oracle_healthy` against an unregistered oracle.
///
/// Returns `Err(InvalidOracleConfig)` after one storage read without
/// making the `is_live` cross-contract call.
#[test]
fn snap_is_oracle_healthy_unregistered() {
    let f = Fx::new();
    let unregistered = Address::generate(&f.env);

    let b = snap(&f.env);
    let result = f.client().try_is_oracle_healthy(&unregistered);
    let (cpu, mem) = delta(b, snap(&f.env));

    assert_eq!(
        result,
        Err(Ok(Error::InvalidOracleConfig)),
        "expected InvalidOracleConfig for unregistered oracle"
    );
    assert_budget(
        "is_oracle_healthy/unregistered",
        cpu,
        mem,
        IS_ORACLE_HEALTHY_CPU,
        IS_ORACLE_HEALTHY_MEM,
    );
}

/// Snapshot: `is_oracle_healthy` — empty registry.
#[test]
fn snap_is_oracle_healthy_empty_registry() {
    let f = Fx::new();
    let oracle = Address::generate(&f.env);

    let b = snap(&f.env);
    let result = f.client().try_is_oracle_healthy(&oracle);
    let (cpu, mem) = delta(b, snap(&f.env));

    assert_eq!(result, Err(Ok(Error::InvalidOracleConfig)));
    assert_budget(
        "is_oracle_healthy/empty_registry",
        cpu,
        mem,
        IS_ORACLE_HEALTHY_CPU,
        IS_ORACLE_HEALTHY_MEM,
    );
}

// -----------------------------------------------------------------------
// Threshold sanity
// -----------------------------------------------------------------------

/// Assert all threshold constants are positive and logically ordered.
///
/// Guards against copy-paste errors in the constant table.  Ordering rules:
/// - Write ops (`add`, `remove`) are more expensive than read-only `list`.
/// - `get_price_data` (≥2 cross-contract hops) ≥ `get_price` (1 hop).
#[test]
fn threshold_constants_are_sane() {
    // All positive
    for (name, v) in [
        ("ADD_ORACLE_CPU",        ADD_ORACLE_CPU),
        ("ADD_ORACLE_MEM",        ADD_ORACLE_MEM),
        ("REMOVE_ORACLE_CPU",     REMOVE_ORACLE_CPU),
        ("REMOVE_ORACLE_MEM",     REMOVE_ORACLE_MEM),
        ("LIST_ORACLES_CPU",      LIST_ORACLES_CPU),
        ("LIST_ORACLES_MEM",      LIST_ORACLES_MEM),
        ("GET_PRICE_CPU",         GET_PRICE_CPU),
        ("GET_PRICE_MEM",         GET_PRICE_MEM),
        ("GET_PRICE_DATA_CPU",    GET_PRICE_DATA_CPU),
        ("GET_PRICE_DATA_MEM",    GET_PRICE_DATA_MEM),
        ("IS_ORACLE_HEALTHY_CPU", IS_ORACLE_HEALTHY_CPU),
        ("IS_ORACLE_HEALTHY_MEM", IS_ORACLE_HEALTHY_MEM),
    ] {
        assert!(v > 0, "{name} threshold must be > 0");
    }

    // Write ops cost at least as much as a read-only list.
    assert!(ADD_ORACLE_CPU    >= LIST_ORACLES_CPU, "add >= list (CPU)");
    assert!(ADD_ORACLE_MEM    >= LIST_ORACLES_MEM, "add >= list (mem)");
    assert!(REMOVE_ORACLE_CPU >= LIST_ORACLES_CPU, "remove >= list (CPU)");
    assert!(REMOVE_ORACLE_MEM >= LIST_ORACLES_MEM, "remove >= list (mem)");

    // get_price_data has more work than get_price in the full path.
    assert!(GET_PRICE_DATA_CPU >= GET_PRICE_CPU, "get_price_data >= get_price (CPU)");
    assert!(GET_PRICE_DATA_MEM >= GET_PRICE_MEM, "get_price_data >= get_price (mem)");
}
