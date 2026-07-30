//! Per-entrypoint Soroban CPU and memory regression snapshots.
//!
//! Each snapshot isolates one public call and compares its measured mock-host
//! budget delta with a checked-in baseline. CI permits at most a 5% increase:
//! `limit = baseline + ceil(baseline / 20)`. Setup work is deliberately
//! completed before the budget reset so unrelated fixture costs cannot hide an
//! entrypoint regression. Mirrors the pattern in `contracts/admin/tests/gas_snap.rs`.

use limits::{Limits, LimitsClient};
use soroban_sdk::Env;

#[derive(Copy, Clone)]
struct Baseline {
    cpu: u64,
    memory: u64,
}

// Baselines measured with soroban-sdk 25.3.2 and a reset unlimited budget.
const VALIDATE_BET_AMOUNT: Baseline = Baseline {
    cpu: 16_390,
    memory: 5_795,
};
const VALIDATE_LEVERAGE: Baseline = Baseline {
    cpu: 16_388,
    memory: 5_787,
};
const VALIDATE_FEE: Baseline = Baseline {
    cpu: 16_386,
    memory: 5_787,
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

fn deploy(env: &Env) -> LimitsClient<'_> {
    let contract_id = env.register(Limits, ());
    LimitsClient::new(env, &contract_id)
}

#[test]
fn snapshot_validate_bet_amount() {
    let env = Env::default();
    let client = deploy(&env);

    reset_budget(&env);
    let result = client.validate_bet_amount(&500, &100, &1_000);
    let measured = measured_budget(&env);

    assert_eq!(result, ());
    assert_budget("validate_bet_amount", measured, VALIDATE_BET_AMOUNT);
}

#[test]
fn snapshot_validate_leverage() {
    let env = Env::default();
    let client = deploy(&env);

    reset_budget(&env);
    let result = client.validate_leverage(&5, &10);
    let measured = measured_budget(&env);

    assert_eq!(result, ());
    assert_budget("validate_leverage", measured, VALIDATE_LEVERAGE);
}

#[test]
fn snapshot_validate_fee() {
    let env = Env::default();
    let client = deploy(&env);

    reset_budget(&env);
    let result = client.validate_fee(&50, &100);
    let measured = measured_budget(&env);

    assert_eq!(result, ());
    assert_budget("validate_fee", measured, VALIDATE_FEE);
}

#[test]
fn rejects_bet_below_minimum() {
    let env = Env::default();
    let client = deploy(&env);

    assert_eq!(
        client.try_validate_bet_amount(&50, &100, &1_000),
        Err(Ok(limits::errors::LimitError::BetBelowMinimum))
    );
}

#[test]
fn rejects_bet_above_maximum() {
    let env = Env::default();
    let client = deploy(&env);

    assert_eq!(
        client.try_validate_bet_amount(&2_000, &100, &1_000),
        Err(Ok(limits::errors::LimitError::BetExceedsMaximum))
    );
}

#[test]
fn rejects_zero_leverage() {
    let env = Env::default();
    let client = deploy(&env);

    assert_eq!(
        client.try_validate_leverage(&0, &10),
        Err(Ok(limits::errors::LimitError::LeverageMustBePositive))
    );
}

#[test]
fn rejects_leverage_above_max() {
    let env = Env::default();
    let client = deploy(&env);

    assert_eq!(
        client.try_validate_leverage(&20, &10),
        Err(Ok(limits::errors::LimitError::LeverageExceedsMax))
    );
}

#[test]
fn rejects_fee_above_max() {
    let env = Env::default();
    let client = deploy(&env);

    assert_eq!(
        client.try_validate_fee(&200, &100),
        Err(Ok(limits::errors::LimitError::FeeExceedsMax))
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
