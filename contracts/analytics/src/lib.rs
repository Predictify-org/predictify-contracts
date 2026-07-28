#![no_std]
use soroban_sdk::{contract, contractimpl, Env, Vec};

#[contract]
pub struct Analytics;

#[contractimpl]
impl Analytics {
    pub fn record_event(_env: Env, event_type: u32, value: u64) -> u64 { event_type as u64 + value }
    pub fn aggregate(_env: Env, event_types: Vec<u32>) -> u64 { event_types.iter().map(|x| *x as u64).sum() }
    pub fn percentile(_env: Env, values: Vec<u64>, pct: u32) -> u64 {
        if values.is_empty() { return 0; }
        let mut sorted = values.clone();
        sorted.sort();
        let idx = ((pct as usize) * (sorted.len() - 1)) / 100;
        sorted.get(idx).copied().unwrap_or(0)
    }
}

