//! # Markets per-entrypoint gas snapshot (v7)
//!
//! Records Soroban host budget deltas — CPU instructions and memory bytes —
//! for every public markets entrypoint. Results are asserted against the v7
//! ceilings below so CI can catch regressions.
//!
//! ## Snapshot version
//!
//! `GAS_SNAP_VERSION = 7` (must match [`markets::GAS_SNAP_VERSION`]).
//!
//! ## How to read output
//!
//! Each test prints a line of the form:
//!
//! ```text
//! [gas_snap:v7] <entrypoint> cpu=<N> mem=<M>
//! ```
//!
//! Include those lines in the PR body when opening a GrantFox / Stellar Wave PR.
//!
//! ## Thresholds (v7 baseline ceilings)
//!
//! | Entrypoint          | Max CPU | Max Mem |
//! |---------------------|---------|---------|
//! | `initialize`        | 250_000 | 80_000  |
//! | `create_market`     | 400_000 | 120_000 |
//! | `vote`              | 350_000 | 100_000 |
//! | `get_market`        | 150_000 | 60_000  |
//! | `get_stake`         | 120_000 | 50_000  |
//! | `resolve_market`    | 300_000 | 90_000  |
//! | `claim_winnings`    | 350_000 | 100_000 |
//! | `gas_snap_version`  | 80_000  | 40_000  |

use markets::{Error, MarketsContract, MarketsContractClient, GAS_SNAP_VERSION};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env, String, Symbol, Vec,
};

const SNAP_VERSION: u32 = 7;

const MAX_CPU_INITIALIZE: u64 = 250_000;
const MAX_MEM_INITIALIZE: u64 = 80_000;
const MAX_CPU_CREATE: u64 = 400_000;
const MAX_MEM_CREATE: u64 = 120_000;
const MAX_CPU_VOTE: u64 = 350_000;
const MAX_MEM_VOTE: u64 = 100_000;
const MAX_CPU_GET_MARKET: u64 = 150_000;
const MAX_MEM_GET_MARKET: u64 = 60_000;
const MAX_CPU_GET_STAKE: u64 = 120_000;
const MAX_MEM_GET_STAKE: u64 = 50_000;
const MAX_CPU_RESOLVE: u64 = 300_000;
const MAX_MEM_RESOLVE: u64 = 90_000;
const MAX_CPU_CLAIM: u64 = 350_000;
const MAX_MEM_CLAIM: u64 = 100_000;
const MAX_CPU_VERSION: u64 = 80_000;
const MAX_MEM_VERSION: u64 = 40_000;

struct GasSample {
    cpu: u64,
    mem: u64,
}

struct Fixture {
    env: Env,
    cid: Address,
    admin: Address,
}

impl Fixture {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let cid = env.register(MarketsContract, ());
        Fixture { env, cid, admin }
    }

    fn client(&self) -> MarketsContractClient<'_> {
        MarketsContractClient::new(&self.env, &self.cid)
    }

    fn measure<R>(&self, f: impl FnOnce() -> R) -> (R, GasSample) {
        let mut budget = self.env.cost_estimate().budget();
        budget.reset_default();
        budget.reset_tracker();
        let result = f();
        let sample = GasSample {
            cpu: budget.cpu_instruction_cost(),
            mem: budget.memory_bytes_cost(),
        };
        (result, sample)
    }

    fn print(entrypoint: &str, sample: &GasSample) {
        std::println!(
            "[gas_snap:v7] {entrypoint} cpu={} mem={}",
            sample.cpu, sample.mem
        );
    }

    fn assert_under(entrypoint: &str, sample: &GasSample, max_cpu: u64, max_mem: u64) {
        Self::print(entrypoint, sample);
        assert!(
            sample.cpu <= max_cpu,
            "{entrypoint}: cpu {} exceeded v7 ceiling {}",
            sample.cpu,
            max_cpu
        );
        assert!(
            sample.mem <= max_mem,
            "{entrypoint}: mem {} exceeded v7 ceiling {}",
            sample.mem,
            max_mem
        );
    }

    fn outcomes_yes_no(&self) -> Vec<String> {
        let mut outcomes = Vec::new(&self.env);
        outcomes.push_back(String::from_str(&self.env, "yes"));
        outcomes.push_back(String::from_str(&self.env, "no"));
        outcomes
    }

    fn create_default_market(&self) -> Symbol {
        self.client().create_market(
            &self.admin,
            &String::from_str(&self.env, "Will BTC hit 100k?"),
            &self.outcomes_yes_no(),
            &30u32,
        )
    }
}

#[test]
fn gas_snap_version_constant_is_v7() {
    assert_eq!(GAS_SNAP_VERSION, SNAP_VERSION);
    assert_eq!(SNAP_VERSION, 7);
}

#[test]
fn gas_snap_initialize() {
    let fx = Fixture::new();
    let client = fx.client();
    let (_, sample) = fx.measure(|| client.initialize(&fx.admin));
    Fixture::assert_under("initialize", &sample, MAX_CPU_INITIALIZE, MAX_MEM_INITIALIZE);
}

#[test]
fn gas_snap_create_market() {
    let fx = Fixture::new();
    let client = fx.client();
    client.initialize(&fx.admin);

    let (market_id, sample) = fx.measure(|| {
        client.create_market(
            &fx.admin,
            &String::from_str(&fx.env, "Will ETH flip BTC?"),
            &fx.outcomes_yes_no(),
            &14u32,
        )
    });
    let _ = market_id;
    Fixture::assert_under("create_market", &sample, MAX_CPU_CREATE, MAX_MEM_CREATE);
}

#[test]
fn gas_snap_vote() {
    let fx = Fixture::new();
    let client = fx.client();
    client.initialize(&fx.admin);
    let market_id = fx.create_default_market();
    let user = Address::generate(&fx.env);

    let (_, sample) = fx.measure(|| {
        client.vote(
            &user,
            &market_id,
            &String::from_str(&fx.env, "yes"),
            &1_000i128,
        )
    });
    Fixture::assert_under("vote", &sample, MAX_CPU_VOTE, MAX_MEM_VOTE);
}

#[test]
fn gas_snap_get_market_readonly() {
    let fx = Fixture::new();
    let client = fx.client();
    client.initialize(&fx.admin);
    let market_id = fx.create_default_market();

    let (market, sample) = fx.measure(|| client.get_market(&market_id));
    assert!(!market.resolved);
    Fixture::assert_under("get_market", &sample, MAX_CPU_GET_MARKET, MAX_MEM_GET_MARKET);
}

#[test]
fn gas_snap_get_stake_readonly() {
    let fx = Fixture::new();
    let client = fx.client();
    client.initialize(&fx.admin);
    let market_id = fx.create_default_market();
    let user = Address::generate(&fx.env);
    client.vote(
        &user,
        &market_id,
        &String::from_str(&fx.env, "no"),
        &500i128,
    );

    let (stake, sample) = fx.measure(|| client.get_stake(&market_id, &user));
    assert_eq!(stake, 500);
    Fixture::assert_under("get_stake", &sample, MAX_CPU_GET_STAKE, MAX_MEM_GET_STAKE);
}

#[test]
fn gas_snap_resolve_market() {
    let fx = Fixture::new();
    let client = fx.client();
    client.initialize(&fx.admin);
    let market_id = fx.create_default_market();
    fx.env
        .ledger()
        .with_mut(|l| l.timestamp += 31 * 24 * 60 * 60);

    let (_, sample) = fx.measure(|| {
        client.resolve_market(&fx.admin, &market_id, &String::from_str(&fx.env, "yes"))
    });
    Fixture::assert_under("resolve_market", &sample, MAX_CPU_RESOLVE, MAX_MEM_RESOLVE);
}

#[test]
fn gas_snap_claim_winnings() {
    let fx = Fixture::new();
    let client = fx.client();
    client.initialize(&fx.admin);
    let market_id = fx.create_default_market();
    let user = Address::generate(&fx.env);
    client.vote(
        &user,
        &market_id,
        &String::from_str(&fx.env, "yes"),
        &2_000i128,
    );
    fx.env
        .ledger()
        .with_mut(|l| l.timestamp += 31 * 24 * 60 * 60);
    client.resolve_market(&fx.admin, &market_id, &String::from_str(&fx.env, "yes"));

    let (claimed, sample) = fx.measure(|| client.claim_winnings(&user, &market_id));
    assert_eq!(claimed, 2_000);
    Fixture::assert_under("claim_winnings", &sample, MAX_CPU_CLAIM, MAX_MEM_CLAIM);
}

#[test]
fn gas_snap_version_entrypoint() {
    let fx = Fixture::new();
    let client = fx.client();
    let (version, sample) = fx.measure(|| client.gas_snap_version());
    assert_eq!(version, SNAP_VERSION);
    Fixture::assert_under("gas_snap_version", &sample, MAX_CPU_VERSION, MAX_MEM_VERSION);
}

#[test]
fn gas_snap_edge_invalid_stake_rejects() {
    let fx = Fixture::new();
    let client = fx.client();
    client.initialize(&fx.admin);
    let market_id = fx.create_default_market();
    let user = Address::generate(&fx.env);

    let (result, sample) = fx.measure(|| {
        client.try_vote(
            &user,
            &market_id,
            &String::from_str(&fx.env, "yes"),
            &0i128,
        )
    });
    match result {
        Err(Ok(Error::InvalidStake)) => {}
        other => panic!("expected InvalidStake, got {other:?}"),
    }
    Fixture::print("vote_invalid_stake", &sample);
    assert!(sample.cpu <= MAX_CPU_VOTE);
}

#[test]
fn gas_snap_edge_create_requires_two_outcomes() {
    let fx = Fixture::new();
    let client = fx.client();
    client.initialize(&fx.admin);
    let mut one = Vec::new(&fx.env);
    one.push_back(String::from_str(&fx.env, "only"));

    let (result, sample) = fx.measure(|| {
        client.try_create_market(
            &fx.admin,
            &String::from_str(&fx.env, "bad"),
            &one,
            &7u32,
        )
    });
    match result {
        Err(Ok(Error::InvalidOutcomes)) => {}
        other => panic!("expected InvalidOutcomes, got {other:?}"),
    }
    Fixture::print("create_market_invalid_outcomes", &sample);
    assert!(sample.cpu <= MAX_CPU_CREATE);
}
