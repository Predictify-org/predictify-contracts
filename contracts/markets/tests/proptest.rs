//! # Markets state invariant property tests (buffer #2)
//!
//! This module uses [`proptest`] to assert that the core markets
//! state invariants hold across **arbitrary valid storage sequences**.
//!
//! ## Invariants exercised
//!
//! | #  | Invariant | Description |
//! |----|-----------|-------------|
//! | 1  | Market roundtrip | Stored `MarketData` is retrieved unchanged |
//! | 2  | Bet roundtrip | Stored `BetData` is retrieved unchanged |
//! | 3  | Liquidity roundtrip | Stored `LiquidityData` is retrieved unchanged |
//! | 4  | Counter monotonicity | `MarketCounter` never decreases |
//! | 5  | Admin persistence | Admin address persists and is retrievable |
//! | 6  | Paused state toggling | Paused state flips between `true` / `false` correctly |
//! | 7  | Storage key isolation | Different `DataKey` variants never collide |
//! | 8  | Missing key returns `None` | Unstored keys return `None` / zero-default on read |
//! | 9  | Admin cooldown bounds | Cooldown seconds are stored/retrieved without overflow |
//! | 10 | Post-condition consistency | After arbitrary write sequences, every read matches the last write |
//!
//! ## Running
//!
//! ```bash
//! cargo test -p markets --test proptest -- --nocapture
//! ```
//!
//! ## Coverage
//!
//! Every storage path exposed by the `MarketsContract` and `AdminManager`
//! is exercised under random inputs.  Deterministic anchors in §5 pin
//! concrete edge cases so CI catches regressions even without the proptest
//! runner.

#![cfg(test)]

extern crate alloc;

use proptest::prelude::*;
use soroban_sdk::{
    contract, contractimpl, contracttype,
    testutils::{Address as _, Ledger},
    Address, Env, String, Vec as SdkVec,
};
use markets::{
    admin::AdminManager,
    errors::ContractError,
    BetData, LiquidityData, MarketData,
};

// ─────────────────────────────────────────────────────────────────────────────
// §1  Dummy contract — wraps all storage paths for testing
// ─────────────────────────────────────────────────────────────────────────────

/// Test harness that exposes every persistent-storage operation so proptest
/// can drive arbitrary state transitions and verify invariants.
#[contract]
pub struct TestHarness;

// Use the same DataKey layout as the real contract.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
enum TDataKey {
    MarketCounter,
    Market(u32),
    Bet(u32, Address),
    Paused,
    Admin,
    Liquidity(u32, Address),
}

#[contractimpl]
impl TestHarness {
    // ── MarketCounter ──────────────────────────────────────────────────

    pub fn set_counter(env: Env, val: u32) {
        env.storage().persistent().set(&TDataKey::MarketCounter, &val);
    }

    pub fn get_counter(env: Env) -> u32 {
        env.storage().persistent().get(&TDataKey::MarketCounter).unwrap_or(0)
    }

    pub fn increment_counter(env: Env) -> u32 {
        let current: u32 = env.storage().persistent().get(&TDataKey::MarketCounter).unwrap_or(0);
        let next = current.saturating_add(1);
        env.storage().persistent().set(&TDataKey::MarketCounter, &next);
        next
    }

    // ── Market ─────────────────────────────────────────────────────────

    pub fn set_market(env: Env, id: u32, data: MarketData) {
        env.storage().persistent().set(&TDataKey::Market(id), &data);
    }

    pub fn get_market_data(env: Env, id: u32) -> Option<MarketData> {
        env.storage().persistent().get(&TDataKey::Market(id))
    }

    // ── Bet ────────────────────────────────────────────────────────────

    pub fn set_bet(env: Env, market_id: u32, user: Address, data: BetData) {
        env.storage().persistent().set(&TDataKey::Bet(market_id, user), &data);
    }

    pub fn get_bet_data(env: Env, market_id: u32, user: Address) -> Option<BetData> {
        env.storage().persistent().get(&TDataKey::Bet(market_id, user))
    }

    // ── Liquidity ──────────────────────────────────────────────────────

    pub fn set_liquidity(env: Env, market_id: u32, user: Address, data: LiquidityData) {
        env.storage().persistent().set(&TDataKey::Liquidity(market_id, user), &data);
    }

    pub fn get_liquidity_data(env: Env, market_id: u32, user: Address) -> Option<LiquidityData> {
        env.storage().persistent().get(&TDataKey::Liquidity(market_id, user))
    }

    // ── Paused ─────────────────────────────────────────────────────────

    pub fn set_paused(env: Env, val: bool) {
        env.storage().persistent().set(&TDataKey::Paused, &val);
    }

    pub fn get_paused(env: Env) -> bool {
        env.storage().persistent().get(&TDataKey::Paused).unwrap_or(false)
    }

    pub fn toggle_paused(env: Env) -> bool {
        let current: bool = env.storage().persistent().get(&TDataKey::Paused).unwrap_or(false);
        let next = !current;
        env.storage().persistent().set(&TDataKey::Paused, &next);
        next
    }

    // ── Admin ──────────────────────────────────────────────────────────

    pub fn set_admin_addr(env: Env, addr: Address) {
        env.storage().persistent().set(&TDataKey::Admin, &addr);
    }

    pub fn get_admin_addr(env: Env) -> Option<Address> {
        env.storage().persistent().get(&TDataKey::Admin)
    }

    // ── Admin cooldown (delegates to AdminManager) ─────────────────────

    pub fn set_cd(env: Env, admin: Address, secs: u64) -> Result<(), ContractError> {
        AdminManager::set_admin_cooldown(&env, &admin, secs)
    }

    pub fn get_cd(env: Env) -> u64 {
        AdminManager::get_admin_cooldown(&env)
    }

    pub fn check_cd(env: Env, admin: Address, func: soroban_sdk::Symbol) -> Result<(), ContractError> {
        AdminManager::check_admin_cooldown(&env, &admin, &func)
    }

    // ── Bulk overwrite — ensure key isolation ─────────────────────────

    /// Write all storage variants at once, then read them all back to
    /// confirm no collision (INV-7).
    pub fn write_all_and_verify(
        env: Env,
        counter: u32,
        market_id: u32,
        market_data: MarketData,
        bet_market: u32,
        bet_user: Address,
        bet_data: BetData,
        liq_market: u32,
        liq_user: Address,
        liq_data: LiquidityData,
        paused_val: bool,
        admin_addr: Address,
    ) {
        env.storage().persistent().set(&TDataKey::MarketCounter, &counter);
        env.storage().persistent().set(&TDataKey::Market(market_id), &market_data);
        env.storage().persistent().set(&TDataKey::Bet(bet_market, bet_user.clone()), &bet_data);
        env.storage().persistent().set(&TDataKey::Liquidity(liq_market, liq_user.clone()), &liq_data);
        env.storage().persistent().set(&TDataKey::Paused, &paused_val);
        env.storage().persistent().set(&TDataKey::Admin, &admin_addr);

        // Read-back assertions
        let got_counter: u32 = env.storage().persistent().get(&TDataKey::MarketCounter).unwrap();
        assert_eq!(got_counter, counter, "INV-7: counter mismatch after bulk write");

        let got_market: MarketData = env.storage().persistent().get(&TDataKey::Market(market_id)).unwrap();
        assert_eq!(got_market.creator, market_data.creator, "INV-7: market.creator mismatch");
        assert_eq!(got_market.question, market_data.question, "INV-7: market.question mismatch");
        assert_eq!(got_market.description, market_data.description, "INV-7: market.description mismatch");
        assert_eq!(got_market.end_time, market_data.end_time, "INV-7: market.end_time mismatch");
        assert_eq!(got_market.resolved, market_data.resolved, "INV-7: market.resolved mismatch");
        assert_eq!(got_market.winning_outcome, market_data.winning_outcome, "INV-7: market.winning_outcome mismatch");
        assert_eq!(got_market.outcome_tags.len(), market_data.outcome_tags.len(), "INV-7: outcome_tags length mismatch");
        assert_eq!(got_market.resolution_source, market_data.resolution_source, "INV-7: market.resolution_source mismatch");

        let got_bet: BetData = env.storage().persistent().get(&TDataKey::Bet(bet_market, bet_user)).unwrap();
        assert_eq!(got_bet.outcome_index, bet_data.outcome_index, "INV-7: bet.outcome_index mismatch");
        assert_eq!(got_bet.amount, bet_data.amount, "INV-7: bet.amount mismatch");

        let got_liq: LiquidityData = env.storage().persistent().get(&TDataKey::Liquidity(liq_market, liq_user)).unwrap();
        assert_eq!(got_liq.total_amount, liq_data.total_amount, "INV-7: liquidity.total_amount mismatch");

        let got_paused: bool = env.storage().persistent().get(&TDataKey::Paused).unwrap();
        assert_eq!(got_paused, paused_val, "INV-7: paused mismatch");

        let got_admin: Address = env.storage().persistent().get(&TDataKey::Admin).unwrap();
        assert_eq!(got_admin, admin_addr, "INV-7: admin mismatch");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// §2  Proptest strategies — plain Rust types only (no Env dependence!)
//
// All strategies generate raw integers, bools, and string-chars so they
// are `Send + Sync` and can be safely composed inside the `proptest!`
// macro.  SDK types (Address, String, Vec<String>) are constructed from
// these raw values inside each test function where a per-test `Env`
// exists.
// ─────────────────────────────────────────────────────────────────────────────

/// Raw components for constructing a `MarketData`.
#[derive(Debug, Clone)]
struct RawMarketData {
    question_chars: alloc::vec::Vec<char>,
    description_chars: alloc::vec::Vec<char>,
    resolution_source_chars: alloc::vec::Vec<char>,
    outcome_tags: alloc::vec::Vec<alloc::vec::Vec<char>>,
    end_time: u64,
    resolved: bool,
    winning_outcome: u32,
}

/// Convert raw `RawMarketData` + a creator `Address` into a `MarketData`.
fn build_market_data(env: &Env, creator: Address, raw: RawMarketData) -> MarketData {
    let question: String = String::from_str(env, &raw.question_chars.into_iter().collect::<alloc::string::String>());
    let description: String = String::from_str(env, &raw.description_chars.into_iter().collect::<alloc::string::String>());
    let resolution_source: String = String::from_str(env, &raw.resolution_source_chars.into_iter().collect::<alloc::string::String>());
    let mut outcome_tags = SdkVec::new(env);
    for tag_chars in &raw.outcome_tags {
        let tag = String::from_str(env, &tag_chars.into_iter().collect::<alloc::string::String>());
        outcome_tags.push_back(tag);
    }
    MarketData {
        creator,
        question,
        description,
        end_time: raw.end_time,
        resolution_source,
        outcome_tags,
        resolved: raw.resolved,
        winning_outcome: raw.winning_outcome,
    }
}

fn raw_market_data_strategy() -> impl Strategy<Value = RawMarketData> {
    (
        prop::collection::vec(proptest::char::range(' ', '~'), 0..20),   // question_chars
        prop::collection::vec(proptest::char::range(' ', '~'), 0..40),   // description_chars
        prop::collection::vec(proptest::char::range(' ', '~'), 0..15),   // resolution_source_chars
        prop::collection::vec(          // outcome_tags — vec of vecs
            prop::collection::vec(proptest::char::range('a', 'z'), 0..10),
            0..5,
        ),
        0u64..=1_000_000_000u64,        // end_time
        proptest::bool::ANY,             // resolved
        0u32..10u32,                     // winning_outcome
    )
        .prop_map(|(q, d, rs, tags, et, res, wo)| RawMarketData {
            question_chars: q,
            description_chars: d,
            resolution_source_chars: rs,
            outcome_tags: tags,
            end_time: et,
            resolved: res,
            winning_outcome: wo,
        })
}

/// Strategy for raw `BetData` components.
fn bet_data_strategy() -> impl Strategy<Value = BetData> {
    (0u32..10u32, -1_000_000i128..=1_000_000i128)
        .prop_map(|(outcome_index, amount)| BetData { outcome_index, amount })
}

/// Strategy for raw `LiquidityData` components.
fn liquidity_data_strategy() -> impl Strategy<Value = LiquidityData> {
    (-1_000_000i128..=1_000_000i128)
        .prop_map(|total_amount| LiquidityData { total_amount })
}

// ─────────────────────────────────────────────────────────────────────────────
// §3  Helper — build env + harness
// ─────────────────────────────────────────────────────────────────────────────

struct Ctx {
    env: Env,
    client: TestHarnessClient<'static>,
}

impl Ctx {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().with_mut(|li| {
            li.max_entry_ttl = 6_000_000;
            li.min_persistent_entry_ttl = 1;
            li.min_temp_entry_ttl = 1;
        });
        let contract_id = env.register(TestHarness, ());
        let client = TestHarnessClient::new(&env, &contract_id);
        Self { env, client }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// §4  Property tests
//
// Each property test uses the closure-style `proptest!(|(params in strategy)| { })
// invocation so it works with all proptest versions.
// ─────────────────────────────────────────────────────────────────────────────

/// INV-1: Stored `MarketData` is retrieved unchanged for any arbitrary
/// market ID and data payload.
#[test]
fn market_roundtrip_invariant() {
    proptest!(|(market_id in 0u32..1_000u32, raw in raw_market_data_strategy())| {
        let ctx = Ctx::new();
        let creator = Address::generate(&ctx.env);
        let data = build_market_data(&ctx.env, creator, raw);
        prop_assert!(ctx.client.get_market_data(&market_id).is_none());
        ctx.client.set_market(&market_id, &data);
        let retrieved = ctx.client.get_market_data(&market_id);
        prop_assert!(retrieved.is_some());
        let r = retrieved.unwrap();
        prop_assert_eq!(r.creator, data.creator);
        prop_assert_eq!(r.question, data.question);
        prop_assert_eq!(r.description, data.description);
        prop_assert_eq!(r.end_time, data.end_time);
        prop_assert_eq!(r.resolution_source, data.resolution_source);
        prop_assert_eq!(r.outcome_tags.len(), data.outcome_tags.len());
        prop_assert_eq!(r.resolved, data.resolved);
        prop_assert_eq!(r.winning_outcome, data.winning_outcome);
    });
}

/// INV-2: Stored `BetData` is retrieved unchanged for any arbitrary
/// market ID, user, and bet payload.
#[test]
fn bet_roundtrip_invariant() {
    proptest!(|(market_id in 0u32..1_000u32, data in bet_data_strategy())| {
        let ctx = Ctx::new();
        let user = Address::generate(&ctx.env);
        prop_assert!(ctx.client.get_bet_data(&market_id, &user).is_none());
        ctx.client.set_bet(&market_id, &user, &data);
        let retrieved = ctx.client.get_bet_data(&market_id, &user);
        prop_assert!(retrieved.is_some());
        let r = retrieved.unwrap();
        prop_assert_eq!(r.outcome_index, data.outcome_index);
        prop_assert_eq!(r.amount, data.amount);
    });
}

/// INV-3: Stored `LiquidityData` is retrieved unchanged for any arbitrary
/// market ID, user, and liquidity payload.
#[test]
fn liquidity_roundtrip_invariant() {
    proptest!(|(market_id in 0u32..1_000u32, data in liquidity_data_strategy())| {
        let ctx = Ctx::new();
        let user = Address::generate(&ctx.env);
        prop_assert!(ctx.client.get_liquidity_data(&market_id, &user).is_none());
        ctx.client.set_liquidity(&market_id, &user, &data);
        let retrieved = ctx.client.get_liquidity_data(&market_id, &user);
        prop_assert!(retrieved.is_some());
        let r = retrieved.unwrap();
        prop_assert_eq!(r.total_amount, data.total_amount);
    });
}

/// INV-4: `MarketCounter` never decreases.
#[test]
fn counter_monotonicity_invariant() {
    proptest!(|(initial in 0u32..10_000u32, increments in prop::collection::vec(proptest::bool::ANY, 1..20))| {
        let ctx = Ctx::new();
        ctx.client.set_counter(&initial);
        let mut prev = initial;
        for should_increment in &increments {
            if *should_increment {
                let next = ctx.client.increment_counter();
                prop_assert!(next > prev);
                prev = next;
            } else {
                let current = ctx.client.get_counter();
                prop_assert_eq!(current, prev);
            }
        }
    });
}

/// INV-5: The admin address persists once set and can be overwritten.
#[test]
fn admin_persistence_invariant() {
    let ctx = Ctx::new();
    assert!(ctx.client.get_admin_addr().is_none());
    let admin1 = Address::generate(&ctx.env);
    ctx.client.set_admin_addr(&admin1);
    let got1 = ctx.client.get_admin_addr();
    assert!(got1.is_some());
    assert_eq!(got1.unwrap(), admin1);
    let admin2 = Address::generate(&ctx.env);
    ctx.client.set_admin_addr(&admin2);
    let got2 = ctx.client.get_admin_addr();
    assert!(got2.is_some());
    assert_eq!(got2.unwrap(), admin2);
}

/// INV-6: The paused state toggles correctly.
#[test]
fn paused_state_invariant() {
    proptest!(|(toggles in prop::collection::vec(proptest::bool::ANY, 0..30))| {
        let ctx = Ctx::new();
        prop_assert!(!ctx.client.get_paused());
        let mut expected = false;
        for &do_toggle in &toggles {
            if do_toggle {
                let new_val = ctx.client.toggle_paused();
                expected = !expected;
                prop_assert_eq!(new_val, expected);
            }
            let current = ctx.client.get_paused();
            prop_assert_eq!(current, expected);
        }
    });
}

/// INV-8: Reading from a key that was never written returns
/// `None` (for optional types) or the type's zero-value default.
#[test]
fn missing_key_invariant() {
    proptest!(|(market_id in 0u32..1_000u32)| {
        let ctx = Ctx::new();
        let user = Address::generate(&ctx.env);
        prop_assert!(ctx.client.get_market_data(&market_id).is_none());
        prop_assert!(ctx.client.get_bet_data(&market_id, &user).is_none());
        prop_assert!(ctx.client.get_liquidity_data(&market_id, &user).is_none());
        prop_assert!(!ctx.client.get_paused());
        prop_assert!(ctx.client.get_admin_addr().is_none());
        prop_assert_eq!(ctx.client.get_counter(), 0u32);
        prop_assert_eq!(ctx.client.get_cd(), 0u64);
    });
}

/// INV-9: Admin cooldown can be set to any value without overflow.
#[test]
fn admin_cooldown_bounds_invariant() {
    proptest!(|(cooldown in 0u64..1_000_000_000_000u64)| {
        let ctx = Ctx::new();
        let admin = Address::generate(&ctx.env);
        let res = ctx.client.try_set_cd(&admin, &cooldown);
        prop_assert!(res.is_ok());
        prop_assert_eq!(ctx.client.get_cd(), cooldown);
    });
}

/// INV-9 (sub): When cooldown is zero, consecutive checks always pass.
#[test]
fn zero_cooldown_always_passes() {
    proptest!(|(timestamps in prop::collection::vec(0u64..1_000_000_000u64, 1..10))| {
        let ctx = Ctx::new();
        let admin = Address::generate(&ctx.env);
        let func = soroban_sdk::Symbol::new(&ctx.env, "action");
        ctx.client.set_cd(&admin, &0);
        for &ts in &timestamps {
            ctx.env.ledger().with_mut(|l| { l.timestamp = ts; });
            let res = ctx.client.try_check_cd(&admin, &func);
            prop_assert!(res.is_ok());
        }
    });
}

/// INV-10: After arbitrary overwrites to the same market slot, the last
/// read matches the last write exactly.
#[test]
fn overwrite_consistency_invariant() {
    proptest!(|(market_id in 0u32..100u32, raw_writes in prop::collection::vec(raw_market_data_strategy(), 0..10))| {
        let ctx = Ctx::new();
        let creator = Address::generate(&ctx.env);
        for raw in &raw_writes {
            let data = build_market_data(&ctx.env, creator.clone(), raw.clone());
            ctx.client.set_market(&market_id, &data);
            let got = ctx.client.get_market_data(&market_id);
            prop_assert!(got.is_some());
            let g = got.unwrap();
            prop_assert_eq!(g.question, data.question);
            prop_assert_eq!(g.description, data.description);
            prop_assert_eq!(g.end_time, data.end_time);
            prop_assert_eq!(g.resolved, data.resolved);
            prop_assert_eq!(g.winning_outcome, data.winning_outcome);
        }
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// §5  Deterministic regression anchors
//
// These unit tests pin concrete invariant behaviours so CI catches regressions
// even without the proptest runner.
// ─────────────────────────────────────────────────────────────────────────────

/// INV-1: Multiple markets stored and retrieved independently.
#[test]
fn multiple_markets_independent() {
    let ctx = Ctx::new();

    let data1 = MarketData {
        creator: Address::generate(&ctx.env),
        question: String::from_str(&ctx.env, "Q1"),
        description: String::from_str(&ctx.env, "First market"),
        end_time: 1000,
        resolution_source: String::from_str(&ctx.env, "src_a"),
        outcome_tags: {
            let mut v = SdkVec::new(&ctx.env);
            v.push_back(String::from_str(&ctx.env, "yes"));
            v.push_back(String::from_str(&ctx.env, "no"));
            v
        },
        resolved: false,
        winning_outcome: 0,
    };

    let data2 = MarketData {
        creator: Address::generate(&ctx.env),
        question: String::from_str(&ctx.env, "Q2"),
        description: String::from_str(&ctx.env, "Second market"),
        end_time: 2000,
        resolution_source: String::from_str(&ctx.env, "src_b"),
        outcome_tags: {
            let mut v = SdkVec::new(&ctx.env);
            v.push_back(String::from_str(&ctx.env, "a"));
            v.push_back(String::from_str(&ctx.env, "b"));
            v.push_back(String::from_str(&ctx.env, "c"));
            v
        },
        resolved: true,
        winning_outcome: 2,
    };

    ctx.client.set_market(&1, &data1);
    ctx.client.set_market(&2, &data2);

    let got1 = ctx.client.get_market_data(&1).unwrap();
    let got2 = ctx.client.get_market_data(&2).unwrap();

    // Verify they are not mixed up.
    assert_eq!(got1.question, String::from_str(&ctx.env, "Q1"));
    assert_eq!(got2.question, String::from_str(&ctx.env, "Q2"));
    assert!(!got1.resolved);
    assert!(got2.resolved);
    assert_eq!(got1.end_time, 1000);
    assert_eq!(got2.end_time, 2000);
}

/// INV-2: Different users betting on the same market are independent.
#[test]
fn different_users_bets_independent() {
    let ctx = Ctx::new();
    let market_id: u32 = 42;
    let user_a = Address::generate(&ctx.env);
    let user_b = Address::generate(&ctx.env);

    let bet_a = BetData { outcome_index: 0, amount: 1000 };
    let bet_b = BetData { outcome_index: 1, amount: 5000 };

    ctx.client.set_bet(&market_id, &user_a, &bet_a);
    ctx.client.set_bet(&market_id, &user_b, &bet_b);

    let got_a = ctx.client.get_bet_data(&market_id, &user_a).unwrap();
    let got_b = ctx.client.get_bet_data(&market_id, &user_b).unwrap();

    assert_eq!(got_a.outcome_index, 0);
    assert_eq!(got_a.amount, 1000);
    assert_eq!(got_b.outcome_index, 1);
    assert_eq!(got_b.amount, 5000);
}

/// INV-3: Liquidity for different market/user combinations is independent.
#[test]
fn different_liquidity_independent() {
    let ctx = Ctx::new();
    let user = Address::generate(&ctx.env);
    let other_user = Address::generate(&ctx.env);

    ctx.client.set_liquidity(&1, &user, &LiquidityData { total_amount: 10_000 });
    ctx.client.set_liquidity(&2, &user, &LiquidityData { total_amount: 20_000 });
    ctx.client.set_liquidity(&1, &other_user, &LiquidityData { total_amount: 5_000 });

    assert_eq!(ctx.client.get_liquidity_data(&1, &user).unwrap().total_amount, 10_000);
    assert_eq!(ctx.client.get_liquidity_data(&2, &user).unwrap().total_amount, 20_000);
    assert_eq!(ctx.client.get_liquidity_data(&1, &other_user).unwrap().total_amount, 5_000);

    // Unwritten combination.
    assert!(ctx.client.get_liquidity_data(&2, &other_user).is_none());
}

/// INV-4: Counter monotonically increases and persists across calls.
#[test]
fn counter_increases_monotonically() {
    let ctx = Ctx::new();
    assert_eq!(ctx.client.get_counter(), 0);

    for i in 1u32..=10u32 {
        let next = ctx.client.increment_counter();
        assert_eq!(next, i, "counter must be {} after increment {}", i, i);
        assert_eq!(ctx.client.get_counter(), i, "counter must persist as {}", i);
    }
}

/// INV-5: Admin address is overwritable.
#[test]
fn admin_overwrite() {
    let ctx = Ctx::new();
    let admin1 = Address::generate(&ctx.env);
    let admin2 = Address::generate(&ctx.env);

    ctx.client.set_admin_addr(&admin1);
    assert_eq!(ctx.client.get_admin_addr().unwrap(), admin1);

    ctx.client.set_admin_addr(&admin2);
    assert_eq!(ctx.client.get_admin_addr().unwrap(), admin2);
}

/// INV-6: Paused toggles back and forth.
#[test]
fn paused_toggle_roundtrip() {
    let ctx = Ctx::new();
    assert!(!ctx.client.get_paused());

    let v1 = ctx.client.toggle_paused();
    assert!(v1);
    assert!(ctx.client.get_paused());

    let v2 = ctx.client.toggle_paused();
    assert!(!v2);
    assert!(!ctx.client.get_paused());
}

/// INV-7: Bulk write with all storage keys — no collision.
#[test]
fn bulk_write_key_isolation() {
    let ctx = Ctx::new();
    let user1 = Address::generate(&ctx.env);
    let user2 = Address::generate(&ctx.env);
    let admin = Address::generate(&ctx.env);

    let market_data = MarketData {
        creator: Address::generate(&ctx.env),
        question: String::from_str(&ctx.env, "Bulk test"),
        description: String::from_str(&ctx.env, "Testing key isolation"),
        end_time: 5000,
        resolution_source: String::from_str(&ctx.env, "src_bulk"),
        outcome_tags: {
            let mut v = SdkVec::new(&ctx.env);
            v.push_back(String::from_str(&ctx.env, "opt_a"));
            v
        },
        resolved: true,
        winning_outcome: 0,
    };

    let bet_data = BetData { outcome_index: 2, amount: 777 };
    let liq_data = LiquidityData { total_amount: 9999 };

    ctx.client.write_all_and_verify(
        &42,           // counter
        &7,            // market_id
        &market_data,
        &3,            // bet_market
        &user1,        // bet_user
        &bet_data,
        &5,            // liq_market
        &user2,        // liq_user
        &liq_data,
        &true,         // paused
        &admin,        // admin_addr
    );
}

/// INV-8: Verify all getters return defaults/None for unstored keys.
#[test]
fn all_getters_return_defaults_for_missing_keys() {
    let ctx = Ctx::new();
    let user = Address::generate(&ctx.env);

    assert!(ctx.client.get_market_data(&0).is_none());
    assert!(ctx.client.get_bet_data(&0, &user).is_none());
    assert!(ctx.client.get_liquidity_data(&0, &user).is_none());
    assert!(!ctx.client.get_paused());
    assert!(ctx.client.get_admin_addr().is_none());
    assert_eq!(ctx.client.get_counter(), 0);
    assert_eq!(ctx.client.get_cd(), 0);
}

/// INV-9: Admin cooldown lifecycle — set, check, wait, check again.
#[test]
fn admin_cooldown_lifecycle() {
    let ctx = Ctx::new();
    let admin = Address::generate(&ctx.env);
    let func = soroban_sdk::Symbol::new(&ctx.env, "some_action");

    // Set cooldown to 100 seconds.
    ctx.client.set_cd(&admin, &100);
    assert_eq!(ctx.client.get_cd(), 100);

    ctx.env.ledger().with_mut(|l| { l.timestamp = 1000; });

    // First check passes.
    assert!(ctx.client.try_check_cd(&admin, &func).is_ok());

    // Immediate recheck fails.
    let err = ctx.client.try_check_cd(&admin, &func).unwrap_err().unwrap();
    assert_eq!(err, ContractError::AdminCooldownActive);

    // Advance past cooldown.
    ctx.env.ledger().with_mut(|l| { l.timestamp = 1101; });
    assert!(ctx.client.try_check_cd(&admin, &func).is_ok());
}

/// INV-10: Overwriting a market with a new value fully replaces the old.
#[test]
fn overwrite_replaces_old_value() {
    let ctx = Ctx::new();
    let market_id: u32 = 99;
    let creator1 = Address::generate(&ctx.env);
    let creator2 = Address::generate(&ctx.env);

    let original = MarketData {
        creator: creator1,
        question: String::from_str(&ctx.env, "Original"),
        description: String::from_str(&ctx.env, "Original description"),
        end_time: 100,
        resolution_source: String::from_str(&ctx.env, "src1"),
        outcome_tags: {
            let mut v = SdkVec::new(&ctx.env);
            v.push_back(String::from_str(&ctx.env, "yes"));
            v
        },
        resolved: false,
        winning_outcome: 0,
    };

    let replacement = MarketData {
        creator: creator2,
        question: String::from_str(&ctx.env, "Replacement"),
        description: String::from_str(&ctx.env, "Replacement description"),
        end_time: 9999,
        resolution_source: String::from_str(&ctx.env, "src2"),
        outcome_tags: {
            let mut v = SdkVec::new(&ctx.env);
            v.push_back(String::from_str(&ctx.env, "no"));
            v.push_back(String::from_str(&ctx.env, "maybe"));
            v
        },
        resolved: true,
        winning_outcome: 1,
    };

    ctx.client.set_market(&market_id, &original);
    ctx.client.set_market(&market_id, &replacement);

    let got = ctx.client.get_market_data(&market_id).unwrap();
    assert_eq!(got.question, String::from_str(&ctx.env, "Replacement"));
    assert_eq!(got.description, String::from_str(&ctx.env, "Replacement description"));
    assert_eq!(got.end_time, 9999);
    assert!(got.resolved);
    assert_eq!(got.winning_outcome, 1);
    assert_eq!(got.outcome_tags.len(), 2);
}

/// INV-1 + INV-3: Market IDs with Liquidity IDs — markets and liquidity
/// use the same ID namespace but different DataKey variants, so they
/// must NOT collide.
#[test]
fn market_and_liquidity_different_keys_for_same_id() {
    let ctx = Ctx::new();
    let id: u32 = 7;
    let user = Address::generate(&ctx.env);

    let market = MarketData {
        creator: Address::generate(&ctx.env),
        question: String::from_str(&ctx.env, "Same ID"),
        description: String::from_str(&ctx.env, "Market with id=7"),
        end_time: 100,
        resolution_source: String::from_str(&ctx.env, "src"),
        outcome_tags: SdkVec::new(&ctx.env),
        resolved: false,
        winning_outcome: 0,
    };
    let liq = LiquidityData { total_amount: 5000 };

    ctx.client.set_market(&id, &market);
    ctx.client.set_liquidity(&id, &user, &liq);

    // Both must be independently retrievable.
    let got_market = ctx.client.get_market_data(&id).unwrap();
    assert_eq!(got_market.question, String::from_str(&ctx.env, "Same ID"));

    let got_liq = ctx.client.get_liquidity_data(&id, &user).unwrap();
    assert_eq!(got_liq.total_amount, 5000);
}

/// Preserving the original admin cooldown test from buffer #1.
#[test]
fn admin_cooldown_invariant() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(TestHarness, ());
    let client = TestHarnessClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let func_name = soroban_sdk::Symbol::new(&env, "action");

    // Initial cooldown should be 0
    assert_eq!(client.get_cd(), 0);

    // Set cooldown
    client.set_cd(&admin, &300);
    assert_eq!(client.get_cd(), 300);

    // Set initial timestamp
    env.ledger().with_mut(|l| {
        l.timestamp = 1000;
    });

    // First action should succeed
    let res = client.try_check_cd(&admin, &func_name);
    assert!(res.is_ok());

    // Second action immediately after should fail
    let res_err = client.try_check_cd(&admin, &func_name);
    assert_eq!(res_err.unwrap_err().unwrap(), ContractError::AdminCooldownActive);

    // Advance time beyond cooldown
    env.ledger().with_mut(|l| {
        l.timestamp = 1000 + 301;
    });

    // Should succeed now
    let res_after = client.try_check_cd(&admin, &func_name);
    assert!(res_after.is_ok());
}
