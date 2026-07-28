//! Per-entrypoint Soroban CPU and memory regression snapshots.
//!
//! Each snapshot isolates one public call and compares its measured mock-host
//! budget delta with a checked-in baseline. CI permits at most a 5% increase:
//! `limit = baseline + ceil(baseline / 20)`. Setup work is deliberately
//! completed before the budget reset so unrelated fixture costs cannot hide an
//! entrypoint regression.

use migrate::{ContractError, MigrateContract, MigrateContractClient};
use soroban_sdk::{testutils::Address as _, Address, Env};

#[derive(Copy, Clone)]
struct Baseline {
    cpu: u64,
    memory: u64,
}

// Baselines measured with soroban-sdk 25.3.1 and a reset unlimited budget.
const INITIALIZE: Baseline = Baseline {
    cpu: 55_643,
    memory: 20_337,
};
const MIGRATE: Baseline = Baseline {
    cpu: 69_405,
    memory: 22_409,
};
// Admin-facing reads are intentionally given a small buffer over the current
// host measurement so minor SDK/host variance does not trip the 5% guard.
const ADMIN: Baseline = Baseline {
    cpu: 32_500,
    memory: 13_000,
};
const CURRENT_VERSION: Baseline = Baseline {
    cpu: 32_500,
    memory: 13_000,
};

fn measured_budget(env: &Env) -> (u64, u64) {
    let budget = env.cost_estimate().budget();
    (budget.cpu_instruction_cost(), budget.memory_bytes_cost())
}

fn reset_budget(env: &Env) {
    env.cost_estimate().budget().reset_unlimited();
}

fn five_percent_ceiling(baseline: u64) -> u64 {
    let rounded_five_percent = baseline.div_ceil(20);
    baseline
        .checked_add(rounded_five_percent)
        .expect("gas baseline must leave room for its 5% regression margin")
}

fn assert_budget(label: &str, measured: (u64, u64), baseline: Baseline) {
    let cpu_limit = five_percent_ceiling(baseline.cpu);
    let memory_limit = five_percent_ceiling(baseline.memory);

    println!(
        "{label}: cpu={}, memory={}, cpu_limit={cpu_limit}, memory_limit={memory_limit}",
        measured.0, measured.1
    );
    assert!(
        measured.0 <= cpu_limit,
        "{label}: CPU regression exceeds 5%: measured {}, baseline {}, limit {}",
        measured.0,
        baseline.cpu,
        cpu_limit
    );
    assert!(
        measured.1 <= memory_limit,
        "{label}: memory regression exceeds 5%: measured {}, baseline {}, limit {}",
        measured.1,
        baseline.memory,
        memory_limit
    );
}

struct Fixture {
    env: Env,
    contract_id: Address,
    admin: Address,
}

impl Fixture {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(MigrateContract, ());
        let admin = Address::generate(&env);
        Self {
            env,
            contract_id,
            admin,
        }
    }

    fn client(&self) -> MigrateContractClient<'_> {
        MigrateContractClient::new(&self.env, &self.contract_id)
    }

    fn initialize(&self, version: u32) {
        self.client().initialize(&self.admin, &version);
    }
}

#[test]
fn snapshot_initialize() {
    let fixture = Fixture::new();
    let client = fixture.client();

    reset_budget(&fixture.env);
    client.initialize(&fixture.admin, &1);
    let measured = measured_budget(&fixture.env);

    assert_budget("initialize", measured, INITIALIZE);
}

#[test]
fn snapshot_migrate() {
    let fixture = Fixture::new();
    fixture.initialize(1);
    let client = fixture.client();

    reset_budget(&fixture.env);
    client.migrate(&fixture.admin, &1, &2);
    let measured = measured_budget(&fixture.env);

    assert_eq!(client.current_version(), 2);
    assert_budget("migrate", measured, MIGRATE);
}

#[test]
fn snapshot_admin() {
    let fixture = Fixture::new();
    fixture.initialize(1);
    let client = fixture.client();

    reset_budget(&fixture.env);
    let stored_admin = client.admin();
    let measured = measured_budget(&fixture.env);

    assert_eq!(stored_admin, fixture.admin);
    assert_budget("admin", measured, ADMIN);
}

#[test]
fn snapshot_current_version() {
    let fixture = Fixture::new();
    fixture.initialize(7);
    let client = fixture.client();

    reset_budget(&fixture.env);
    let current_version = client.current_version();
    let measured = measured_budget(&fixture.env);

    assert_eq!(current_version, 7);
    assert_budget("current_version", measured, CURRENT_VERSION);
}

#[test]
fn rejects_stale_and_unsafe_migrations_without_changing_state() {
    let fixture = Fixture::new();
    fixture.initialize(3);
    let client = fixture.client();

    assert_eq!(
        client.try_migrate(&fixture.admin, &2, &4),
        Err(Ok(ContractError::VersionMismatch))
    );
    assert_eq!(
        client.try_migrate(&fixture.admin, &3, &3),
        Err(Ok(ContractError::InvalidTargetVersion))
    );
    assert_eq!(
        client.try_migrate(&fixture.admin, &3, &2),
        Err(Ok(ContractError::InvalidTargetVersion))
    );
    assert_eq!(client.current_version(), 3);
}

#[test]
fn rejects_an_authenticated_non_admin() {
    let fixture = Fixture::new();
    fixture.initialize(1);
    let stranger = Address::generate(&fixture.env);

    assert_eq!(
        fixture.client().try_migrate(&stranger, &1, &2),
        Err(Ok(ContractError::Unauthorized))
    );
    assert_eq!(fixture.client().current_version(), 1);
}

#[test]
fn regression_margin_is_exactly_five_percent_rounded_up() {
    assert_eq!(five_percent_ceiling(0), 0);
    assert_eq!(five_percent_ceiling(1), 2);
    assert_eq!(five_percent_ceiling(20), 21);
    assert_eq!(five_percent_ceiling(21), 23);
    assert_eq!(five_percent_ceiling(100), 105);
}
