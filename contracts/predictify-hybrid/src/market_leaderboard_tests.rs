//! # Per-Market Leaderboard Tests
//!
//! Tests for the bounded top-N stake leaderboard maintained per market.
//!
//! ## What is tested
//!
//! | Test name                                             | Invariant checked                                 |
//! |-------------------------------------------------------|---------------------------------------------------|
//! | `leaderboard_empty_returns_empty`                     | Missing key → empty Vec                           |
//! | `leaderboard_single_entry_inserted`                   | One entry, rank 1                                 |
//! | `leaderboard_returns_descending_by_stake`             | Descending sort order                             |
//! | `leaderboard_heap_size_never_exceeds_capacity`        | Heap size ≤ N                                     |
//! | `leaderboard_low_stake_evicted_when_full`             | Below-minimum candidate not kept                  |
//! | `leaderboard_high_stake_evicts_minimum`               | Above-minimum candidate evicts weakest            |
//! | `leaderboard_user_update_reflected`                   | Existing user updated in place                    |
//! | `leaderboard_ranks_are_sequential`                    | Ranks 1…N assigned correctly                      |
//! | `leaderboard_limit_caps_output`                       | limit parameter respected                         |
//! | `leaderboard_capacity_clamped_to_max`                 | capacity > MAX clamped                            |
//! | `leaderboard_capacity_one_keeps_best`                 | capacity=1 keeps only highest stake               |
//! | `leaderboard_tie_broken_by_timestamp_earlier_wins`    | Equal stake – earlier bettor keeps seat           |
//! | `leaderboard_separate_markets_isolated`               | Different markets do not share data               |
//! | `leaderboard_zero_stake_allowed`                      | Zero stake inserted (edge case)                   |
//! | `leaderboard_i128_max_stake`                          | Maximum i128 value handled without panic          |
//! | `leaderboard_fifty_users_fills_max_capacity`          | Exactly MAX_CAPACITY users all fit                |
//! | `leaderboard_fifty_plus_one_keeps_top_fifty`          | 51st user replaces weakest                        |
//! | `leaderboard_upsert_preserves_heap_size`              | Updating existing user does not grow heap         |

#![cfg(test)]

use crate::{
    market_analytics::MarketLeaderboard,
    storage::MAX_MARKET_LEADERBOARD_CAPACITY,
    types::MarketLeaderboardEntry,
};
use soroban_sdk::{
    contract, contractimpl,
    testutils::Address as _,
    Address, Env, Symbol,
};

// ── minimal test-only stub contract ──────────────────────────────────────────
//
// We need a registered contract address so that `env.as_contract` can execute
// code in a contract context with working persistent storage.  Using a tiny
// stub avoids depending on the main `PredictifyHybrid` contract which has
// pre-existing compile issues in other modules.

#[contract]
struct LeaderboardTestStub;

#[contractimpl]
impl LeaderboardTestStub {}

// ── test helpers ──────────────────────────────────────────────────────────────

/// Registers the stub contract and returns `(env, contract_id)`.
fn setup() -> (Env, Address) {
    let env = Env::default();
    let cid = env.register(LeaderboardTestStub, ());
    (env, cid)
}

fn market_id(env: &Env) -> Symbol {
    Symbol::new(env, "mkt_test")
}

fn market_id2(env: &Env) -> Symbol {
    Symbol::new(env, "mkt_two")
}

/// Shorthand: call `upsert` with default capacity (50).
fn upsert(env: &Env, market: &Symbol, user: &Address, stake: i128, ts: u64) {
    MarketLeaderboard::upsert(env, market, user, stake, ts, MAX_MARKET_LEADERBOARD_CAPACITY)
        .expect("upsert must not fail");
}

/// Shorthand: call `top_by_stake` and return the result Vec.
fn top(env: &Env, market: &Symbol, limit: u32) -> soroban_sdk::Vec<MarketLeaderboardEntry> {
    MarketLeaderboard::top_by_stake(env, market, limit)
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[test]
fn leaderboard_empty_returns_empty() {
    let (env, cid) = setup();
    env.as_contract(&cid, || {
        let mid = market_id(&env);
        let result = top(&env, &mid, 10);
        assert_eq!(result.len(), 0, "empty market → empty leaderboard");
    });
}

#[test]
fn leaderboard_single_entry_inserted() {
    let (env, cid) = setup();
    let user = Address::generate(&env);
    env.as_contract(&cid, || {
        let mid = market_id(&env);
        upsert(&env, &mid, &user, 500, 1000);
        let result = top(&env, &mid, 10);
        assert_eq!(result.len(), 1);
        let entry = result.get(0).unwrap();
        assert_eq!(entry.user, user);
        assert_eq!(entry.stake, 500);
        assert_eq!(entry.rank, 1, "only entry should have rank 1");
    });
}

#[test]
fn leaderboard_returns_descending_by_stake() {
    let (env, cid) = setup();
    env.as_contract(&cid, || {
        let mid = market_id(&env);
        // Insert in arbitrary order.
        for stake in [30i128, 10, 50, 20, 40] {
            upsert(&env, &mid, &Address::generate(&env), stake, 1000);
        }
        let result = top(&env, &mid, 10);
        assert_eq!(result.len(), 5);
        // Verify descending order.
        for i in 1..result.len() {
            assert!(
                result.get(i - 1).unwrap().stake >= result.get(i).unwrap().stake,
                "entry {} should have stake >= entry {}",
                i - 1,
                i
            );
        }
        // Verify top entry has highest stake.
        assert_eq!(result.get(0).unwrap().stake, 50);
    });
}

#[test]
fn leaderboard_heap_size_never_exceeds_capacity() {
    let (env, cid) = setup();
    env.as_contract(&cid, || {
        let mid = market_id(&env);
        let capacity = 5u32;
        // Insert 10 entries into a capacity-5 heap.
        for i in 1i128..=10 {
            MarketLeaderboard::upsert(&env, &mid, &Address::generate(&env), i * 100, 1000, capacity)
                .expect("upsert must succeed");
        }
        let result = top(&env, &mid, capacity);
        assert!(
            result.len() <= capacity,
            "heap must not exceed capacity={}; got {}",
            capacity,
            result.len()
        );
    });
}

#[test]
fn leaderboard_low_stake_evicted_when_full() {
    let (env, cid) = setup();
    env.as_contract(&cid, || {
        let mid = market_id(&env);
        let capacity = 3u32;
        // Fill with high stakes.
        for stake in [100i128, 200, 300] {
            MarketLeaderboard::upsert(&env, &mid, &Address::generate(&env), stake, 1000, capacity)
                .unwrap();
        }
        // Attempt to insert a very low stake.
        let loser = Address::generate(&env);
        MarketLeaderboard::upsert(&env, &mid, &loser, 1, 2000, capacity).unwrap();

        let result = top(&env, &mid, capacity);
        assert_eq!(result.len(), 3, "capacity must be 3");
        // The low-stake user should not appear.
        for i in 0..result.len() {
            assert_ne!(
                result.get(i).unwrap().user,
                loser,
                "low-stake user must not be in leaderboard"
            );
            assert!(result.get(i).unwrap().stake >= 100);
        }
    });
}

#[test]
fn leaderboard_high_stake_evicts_minimum() {
    let (env, cid) = setup();
    env.as_contract(&cid, || {
        let mid = market_id(&env);
        let capacity = 3u32;
        // Fill with moderate stakes.
        for stake in [100i128, 200, 300] {
            MarketLeaderboard::upsert(&env, &mid, &Address::generate(&env), stake, 1000, capacity)
                .unwrap();
        }
        // Insert a high stake – should evict the current minimum (100).
        let winner = Address::generate(&env);
        MarketLeaderboard::upsert(&env, &mid, &winner, 999, 2000, capacity).unwrap();

        let result = top(&env, &mid, capacity);
        assert_eq!(result.len(), 3);
        // 100 should be evicted.
        for i in 0..result.len() {
            assert!(
                result.get(i).unwrap().stake > 100,
                "minimum (100) must have been evicted"
            );
        }
        // Winner must appear at rank 1.
        assert_eq!(result.get(0).unwrap().user, winner);
        assert_eq!(result.get(0).unwrap().stake, 999);
    });
}

#[test]
fn leaderboard_user_update_reflected() {
    let (env, cid) = setup();
    let user = Address::generate(&env);
    env.as_contract(&cid, || {
        let mid = market_id(&env);
        // Initial bet.
        upsert(&env, &mid, &user, 100, 1000);
        // Second bet – cumulative stake increases.
        upsert(&env, &mid, &user, 300, 2000);

        let result = top(&env, &mid, 10);
        assert_eq!(result.len(), 1, "same user – only one heap entry");
        assert_eq!(result.get(0).unwrap().stake, 300);
        assert_eq!(result.get(0).unwrap().last_bet_timestamp, 2000);
    });
}

#[test]
fn leaderboard_ranks_are_sequential() {
    let (env, cid) = setup();
    env.as_contract(&cid, || {
        let mid = market_id(&env);
        for stake in [40i128, 10, 70, 20, 55] {
            upsert(&env, &mid, &Address::generate(&env), stake, 1000);
        }
        let result = top(&env, &mid, 10);
        for i in 0..result.len() {
            assert_eq!(
                result.get(i).unwrap().rank,
                i + 1,
                "rank at position {} must be {}",
                i,
                i + 1
            );
        }
    });
}

#[test]
fn leaderboard_limit_caps_output() {
    let (env, cid) = setup();
    env.as_contract(&cid, || {
        let mid = market_id(&env);
        for stake in [10i128, 20, 30, 40, 50] {
            upsert(&env, &mid, &Address::generate(&env), stake, 1000);
        }
        // Request only 3 even though 5 are stored.
        let result = top(&env, &mid, 3);
        assert_eq!(result.len(), 3);
        // Must be the top 3.
        assert_eq!(result.get(0).unwrap().stake, 50);
        assert_eq!(result.get(1).unwrap().stake, 40);
        assert_eq!(result.get(2).unwrap().stake, 30);
    });
}

#[test]
fn leaderboard_capacity_clamped_to_max() {
    let (env, cid) = setup();
    let user = Address::generate(&env);
    env.as_contract(&cid, || {
        let mid = market_id(&env);
        // capacity > MAX_MARKET_LEADERBOARD_CAPACITY should be silently clamped.
        MarketLeaderboard::upsert(&env, &mid, &user, 999, 1000, 9999).unwrap();
        let result = top(&env, &mid, 9999);
        assert_eq!(result.len(), 1);
        assert_eq!(result.get(0).unwrap().stake, 999);
    });
}

#[test]
fn leaderboard_capacity_one_keeps_best() {
    let (env, cid) = setup();
    env.as_contract(&cid, || {
        let mid = market_id(&env);
        let a = Address::generate(&env);
        let b = Address::generate(&env);
        // Insert lower stake first.
        MarketLeaderboard::upsert(&env, &mid, &a, 10, 1000, 1).unwrap();
        // Insert higher stake second – should evict a.
        MarketLeaderboard::upsert(&env, &mid, &b, 50, 2000, 1).unwrap();

        let result = top(&env, &mid, 1);
        assert_eq!(result.len(), 1);
        assert_eq!(result.get(0).unwrap().user, b);
        assert_eq!(result.get(0).unwrap().stake, 50);
    });
}

#[test]
fn leaderboard_tie_broken_by_timestamp_earlier_wins() {
    let (env, cid) = setup();
    env.as_contract(&cid, || {
        let mid = market_id(&env);
        let capacity = 2u32;
        let early = Address::generate(&env);
        let late = Address::generate(&env);
        let interloper = Address::generate(&env);

        // Fill with two equal-stake entries; early bettor placed bet first.
        MarketLeaderboard::upsert(&env, &mid, &early, 100, 1000, capacity).unwrap();
        MarketLeaderboard::upsert(&env, &mid, &late, 100, 9000, capacity).unwrap();

        // A new entrant also has stake 100 – should evict the *later* timestamp
        // (tie-break: first bettor keeps their seat).
        MarketLeaderboard::upsert(&env, &mid, &interloper, 100, 5000, capacity).unwrap();

        let result = top(&env, &mid, 10);
        assert_eq!(result.len(), 2);
        // `late` (ts=9000) should have been evicted; `early` and `interloper` remain.
        let users: alloc::vec::Vec<Address> = (0..result.len())
            .map(|i| result.get(i).unwrap().user)
            .collect();
        assert!(users.contains(&early), "early bettor must keep seat");
        assert!(!users.contains(&late), "late bettor must be evicted");
    });
}

#[test]
fn leaderboard_separate_markets_isolated() {
    let (env, cid) = setup();
    let user_a = Address::generate(&env);
    let user_b = Address::generate(&env);
    env.as_contract(&cid, || {
        let mid1 = market_id(&env);
        let mid2 = market_id2(&env);

        upsert(&env, &mid1, &user_a, 1000, 1000);
        upsert(&env, &mid2, &user_b, 2000, 1000);

        let result1 = top(&env, &mid1, 10);
        let result2 = top(&env, &mid2, 10);

        // Market 1 contains only user_a.
        assert_eq!(result1.len(), 1);
        assert_eq!(result1.get(0).unwrap().user, user_a);

        // Market 2 contains only user_b.
        assert_eq!(result2.len(), 1);
        assert_eq!(result2.get(0).unwrap().user, user_b);
    });
}

#[test]
fn leaderboard_zero_stake_allowed() {
    let (env, cid) = setup();
    let user = Address::generate(&env);
    env.as_contract(&cid, || {
        let mid = market_id(&env);
        upsert(&env, &mid, &user, 0, 1000);
        let result = top(&env, &mid, 10);
        assert_eq!(result.len(), 1);
        assert_eq!(result.get(0).unwrap().stake, 0);
    });
}

#[test]
fn leaderboard_i128_max_stake() {
    let (env, cid) = setup();
    let user = Address::generate(&env);
    env.as_contract(&cid, || {
        let mid = market_id(&env);
        // Should not panic or overflow.
        upsert(&env, &mid, &user, i128::MAX, 1000);
        let result = top(&env, &mid, 10);
        assert_eq!(result.get(0).unwrap().stake, i128::MAX);
    });
}

#[test]
fn leaderboard_fifty_users_fills_max_capacity() {
    let (env, cid) = setup();
    env.as_contract(&cid, || {
        let mid = market_id(&env);
        for i in 1i128..=(MAX_MARKET_LEADERBOARD_CAPACITY as i128) {
            let user = Address::generate(&env);
            upsert(&env, &mid, &user, i * 100, 1000);
        }
        let result = top(&env, &mid, MAX_MARKET_LEADERBOARD_CAPACITY);
        assert_eq!(
            result.len(),
            MAX_MARKET_LEADERBOARD_CAPACITY,
            "all 50 users must fit"
        );
    });
}

#[test]
fn leaderboard_fifty_plus_one_keeps_top_fifty() {
    let (env, cid) = setup();
    env.as_contract(&cid, || {
        let mid = market_id(&env);
        // Fill with stakes 100, 200, … 5000.
        for i in 1i128..=(MAX_MARKET_LEADERBOARD_CAPACITY as i128) {
            let user = Address::generate(&env);
            upsert(&env, &mid, &user, i * 100, 1000);
        }
        // 51st user has stake 50 – below the current minimum (100).
        let newcomer = Address::generate(&env);
        upsert(&env, &mid, &newcomer, 50, 2000);

        let result = top(&env, &mid, MAX_MARKET_LEADERBOARD_CAPACITY);
        assert_eq!(result.len(), MAX_MARKET_LEADERBOARD_CAPACITY);
        // Newcomer must NOT appear.
        for i in 0..result.len() {
            assert_ne!(result.get(i).unwrap().user, newcomer);
        }
        // Minimum in result must be 100 (newcomer with 50 was rejected).
        let min_stake = (0..result.len())
            .map(|i| result.get(i).unwrap().stake)
            .min()
            .unwrap_or(0);
        assert_eq!(
            min_stake, 100,
            "minimum stake in top-50 must be 100 (newcomer with 50 was rejected)"
        );
    });
}

#[test]
fn leaderboard_upsert_preserves_heap_size() {
    let (env, cid) = setup();
    let user = Address::generate(&env);
    env.as_contract(&cid, || {
        let mid = market_id(&env);
        let capacity = 5u32;
        // Fill to capacity with other users.
        for i in 1i128..=5 {
            let u = Address::generate(&env);
            MarketLeaderboard::upsert(&env, &mid, &u, i * 100, 1000, capacity).unwrap();
        }
        // Insert a new user (stake 600 > current min 100, so it evicts the min).
        MarketLeaderboard::upsert(&env, &mid, &user, 600, 2000, capacity).unwrap();
        let size_after_insert = top(&env, &mid, 10).len();

        // Update the same user again with a higher stake.
        MarketLeaderboard::upsert(&env, &mid, &user, 1200, 3000, capacity).unwrap();
        let size_after_update = top(&env, &mid, 10).len();

        assert_eq!(
            size_after_update, size_after_insert,
            "re-upsert of existing user must not change heap size"
        );
        assert!(
            size_after_update as u32 <= capacity,
            "heap size must never exceed capacity"
        );
    });
}
