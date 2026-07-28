//! Per-entrypoint Soroban CPU and memory regression snapshots.
//!
//! Each snapshot isolates one public call and compares its measured mock-host
//! budget delta with a checked-in baseline. CI permits at most a 5% increase:
//! `limit = baseline + ceil(baseline / 20)`. Setup work is deliberately
//! completed before the budget reset so unrelated fixture costs cannot hide an
//! entrypoint regression.

use admin::{AdminContract, AdminContractClient, ContractError};
use soroban_sdk::{testutils::Address as _, Address, Env, Symbol};

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
const ADMIN: Baseline = Baseline {
    cpu: 31_614,
    memory: 12_495,
};
const SET_ADMIN_COOLDOWN: Baseline = Baseline {
    cpu: 45_000,
    memory: 18_000,
};
const GET_ADMIN_COOLDOWN: Baseline = Baseline {
    cpu: 28_000,
    memory: 10_000,
};
const CHECK_ADMIN_COOLDOWN: Baseline = Baseline {
    cpu: 52_000,
    memory: 19_000,
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
        let contract_id = env.register(AdminContract, ());
        let admin = Address::generate(&env);
        Self {
            env,
            contract_id,
            admin,
        }
    }

    fn client(&self) -> AdminContractClient<'_> {
        AdminContractClient::new(&self.env, &self.contract_id)
    }

    fn initialize(&self) {
        self.client().initialize(&self.admin);
    }
}

#[test]
fn snapshot_initialize() {
    let fixture = Fixture::new();
    let client = fixture.client();

    reset_budget(&fixture.env);
    client.initialize(&fixture.admin);
    let measured = measured_budget(&fixture.env);

    assert_budget("initialize", measured, INITIALIZE);
}

#[test]
fn snapshot_admin() {
    let fixture = Fixture::new();
    fixture.initialize();
    let client = fixture.client();

    reset_budget(&fixture.env);
    let stored_admin = client.admin();
    let measured = measured_budget(&fixture.env);

    assert_eq!(stored_admin, Ok(fixture.admin));
    assert_budget("admin", measured, ADMIN);
}

#[test]
fn snapshot_set_admin_cooldown() {
    let fixture = Fixture::new();
    fixture.initialize();
    let client = fixture.client();

    reset_budget(&fixture.env);
    client.set_admin_cooldown(&fixture.admin, &300);
    let measured = measured_budget(&fixture.env);

    assert_eq!(client.get_admin_cooldown(), 300);
    assert_budget("set_admin_cooldown", measured, SET_ADMIN_COOLDOWN);
}

#[test]
fn snapshot_get_admin_cooldown() {
    let fixture = Fixture::new();
    fixture.initialize();
    let client = fixture.client();
    client.set_admin_cooldown(&fixture.admin, &600);

    reset_budget(&fixture.env);
    let cooldown = client.get_admin_cooldown();
    let measured = measured_budget(&fixture.env);

    assert_eq!(cooldown, 600);
    assert_budget("get_admin_cooldown", measured, GET_ADMIN_COOLDOWN);
}

#[test]
fn snapshot_check_admin_cooldown() {
    let fixture = Fixture::new();
    fixture.initialize();
    let client = fixture.client();
    client.set_admin_cooldown(&fixture.admin, &300);
    
    let func_name = Symbol::new(&fixture.env, "test_action");

    reset_budget(&fixture.env);
    let result = client.check_admin_cooldown(&fixture.admin, &func_name);
    let measured = measured_budget(&fixture.env);

    assert!(result.is_ok());
    assert_budget("check_admin_cooldown", measured, CHECK_ADMIN_COOLDOWN);
}

#[test]
fn rejects_duplicate_initialization() {
    let fixture = Fixture::new();
    fixture.initialize();

    assert_eq!(
        fixture.client().try_initialize(&fixture.admin),
        Err(Ok(ContractError::AlreadyInitialized))
    );
}

#[test]
fn rejects_unauthorized_admin_call() {
    let fixture = Fixture::new();
    fixture.initialize();
    let stranger = Address::generate(&fixture.env);

    assert_eq!(
        fixture.client().try_set_admin_cooldown(&stranger, &100),
        Err(Ok(ContractError::Unauthorized))
    );
}

#[test]
fn rejects_cooldown_when_active() {
    let fixture = Fixture::new();
    fixture.initialize();
    let client = fixture.client();
    client.set_admin_cooldown(&fixture.admin, &300);
    
    let func_name = Symbol::new(&fixture.env, "test_action");
    
    // First call should succeed
    assert!(client.check_admin_cooldown(&fixture.admin, &func_name).is_ok());
    
    // Immediate second call should fail due to cooldown
    assert_eq!(
        client.try_check_admin_cooldown(&fixture.admin, &func_name),
        Err(Ok(ContractError::AdminActionTimelocked))
    );
}

#[test]
fn regression_margin_is_exactly_five_percent_rounded_up() {
    assert_eq!(five_percent_ceiling(0), 0);
    assert_eq!(five_percent_ceiling(1), 2);
    assert_eq!(five_percent_ceiling(20), 21);
    assert_eq!(five_percent_ceiling(21), 23);
    assert_eq!(five_percent_ceiling(100), 105);
}
