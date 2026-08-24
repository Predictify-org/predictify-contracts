//! # Bounded Top-N Leaderboard Heap
//!
//! Maintains a fixed-size min-heap per leaderboard type so that only the top-N
//! entries are ever kept in contract storage.
//!
//! ## Design
//!
//! Two leaderboards are supported:
//!
//! - **Winnings** – ranked by `total_winnings` (descending).
//! - **WinRate**  – ranked by `win_rate` then `total_bets_placed` as tie-breaker (descending).
//!
//! Each heap is stored as a [`soroban_sdk::Vec`] of [`UserLeaderboardEntryV1`].
//! The heap invariant is maintained in-place using standard sift-up / sift-down
//! operations adapted for Soroban's `Vec` (no random mutable access – swap via
//! remove + insert).  Because `N ≤ MAX_CAPACITY` (50), all operations are O(N)
//! worst-case in ledger reads/writes which is acceptable.
//!
//! ## Storage keys
//!
//! | key constant              | type                           |
//! |---------------------------|--------------------------------|
//! | `HEAP_WINNINGS_KEY`       | `Vec<UserLeaderboardEntryV1>`  |
//! | `HEAP_WIN_RATE_KEY`       | `Vec<UserLeaderboardEntryV1>`  |
//!
//! ## API
//!
//! | function                                  | mutating |
//! |-------------------------------------------|---------|
//! | [`LeaderboardHeap::push_winnings`]        | yes     |
//! | [`LeaderboardHeap::push_win_rate`]        | yes     |
//! | [`LeaderboardHeap::top_by_winnings`]      | no      |
//! | [`LeaderboardHeap::top_by_win_rate`]      | no      |

use crate::types::UserLeaderboardEntryV1;
use soroban_sdk::{symbol_short, Address, Env, Symbol, Vec};

// ── storage keys ───────────────────────────────────────────────────────────

const HEAP_WINNINGS_KEY: Symbol = symbol_short!("hpWin");
const HEAP_WIN_RATE_KEY: Symbol = symbol_short!("hpRate");

/// Maximum entries kept in each heap (hard cap for gas safety).
pub const MAX_CAPACITY: u32 = 50;

// ── public API ─────────────────────────────────────────────────────────────

/// Operations on the bounded top-N leaderboard heaps.
pub struct LeaderboardHeap;

impl LeaderboardHeap {
    // ── write operations ───────────────────────────────────────────────────

    /// Insert or update a user in the **winnings** leaderboard.
    ///
    /// Uses `total_winnings` as the ranking key.  When the heap is full the
    /// candidate is only kept if it beats the current minimum.
    ///
    /// # Parameters
    ///
    /// * `env`      – Soroban environment.
    /// * `user`     – Address of the participant.
    /// * `capacity` – Maximum entries to retain (`1..=MAX_CAPACITY`). Clamped
    ///                to [`MAX_CAPACITY`] if larger.
    /// * `winnings` – Candidate total winnings value (overflow-safe `i128`).
    /// * `stats`    – Full statistics snapshot for the user (for display).
    pub fn push_winnings(
        env: &Env,
        user: &Address,
        capacity: u32,
        winnings: i128,
        stats: &UserLeaderboardEntryV1,
    ) {
        let cap = capacity.min(MAX_CAPACITY);
        let mut heap = load(env, &HEAP_WINNINGS_KEY);
        upsert(&mut heap, env, user, cap, winnings, stats, key_winnings);
        store(env, &HEAP_WINNINGS_KEY, &heap);
    }

    /// Insert or update a user in the **win-rate** leaderboard.
    ///
    /// Uses `win_rate` (basis points) as the primary key; `total_bets_placed`
    /// as a secondary key so that higher-volume users rank above equal-rate
    /// newcomers.
    ///
    /// # Parameters
    ///
    /// * `env`      – Soroban environment.
    /// * `user`     – Address of the participant.
    /// * `capacity` – Maximum entries to retain (`1..=MAX_CAPACITY`). Clamped
    ///                to [`MAX_CAPACITY`] if larger.
    /// * `stats`    – Full statistics snapshot for the user.
    pub fn push_win_rate(
        env: &Env,
        user: &Address,
        capacity: u32,
        stats: &UserLeaderboardEntryV1,
    ) {
        let cap = capacity.min(MAX_CAPACITY);
        let composite = key_win_rate(stats);
        let mut heap = load(env, &HEAP_WIN_RATE_KEY);
        upsert(&mut heap, env, user, cap, composite, stats, key_win_rate);
        store(env, &HEAP_WIN_RATE_KEY, &heap);
    }

    // ── read operations ────────────────────────────────────────────────────

    /// Return up to `limit` entries from the **winnings** leaderboard, sorted
    /// highest-first.
    ///
    /// # Parameters
    ///
    /// * `env`   – Soroban environment.
    /// * `limit` – Maximum entries to return. Clamped to [`MAX_CAPACITY`].
    ///
    /// # Returns
    ///
    /// [`Vec<UserLeaderboardEntryV1>`] ranked 1…N (rank field filled).
    pub fn top_by_winnings(env: &Env, limit: u32) -> Vec<UserLeaderboardEntryV1> {
        let cap = limit.min(MAX_CAPACITY);
        let heap = load(env, &HEAP_WINNINGS_KEY);
        sorted_top(env, heap, cap, key_winnings)
    }

    /// Return up to `limit` entries from the **win-rate** leaderboard, sorted
    /// highest-first.
    ///
    /// # Parameters
    ///
    /// * `env`   – Soroban environment.
    /// * `limit` – Maximum entries to return. Clamped to [`MAX_CAPACITY`].
    ///
    /// # Returns
    ///
    /// [`Vec<UserLeaderboardEntryV1>`] ranked 1…N (rank field filled).
    pub fn top_by_win_rate(env: &Env, limit: u32) -> Vec<UserLeaderboardEntryV1> {
        let cap = limit.min(MAX_CAPACITY);
        let heap = load(env, &HEAP_WIN_RATE_KEY);
        sorted_top(env, heap, cap, key_win_rate)
    }
}

// ── key extractors ─────────────────────────────────────────────────────────

/// Ranking key for the winnings heap: higher `total_winnings` → higher rank.
#[inline]
fn key_winnings(e: &UserLeaderboardEntryV1) -> i128 {
    e.total_winnings
}

/// Ranking key for the win-rate heap.
///
/// Encodes `(win_rate as i128) << 40 | total_bets_placed.min(1<<40-1)` so
/// that win-rate is primary and volume is secondary, both fitting in one i128
/// comparison.
#[inline]
fn key_win_rate(e: &UserLeaderboardEntryV1) -> i128 {
    let rate = e.win_rate as i128;
    let bets = (e.total_bets_placed as i128).min((1i128 << 40) - 1);
    (rate << 40) | bets
}

// ── heap internals ─────────────────────────────────────────────────────────

/// Load a heap from persistent storage (empty Vec if absent).
fn load(env: &Env, key: &Symbol) -> Vec<UserLeaderboardEntryV1> {
    env.storage()
        .persistent()
        .get(key)
        .unwrap_or_else(|| Vec::new(env))
}

/// Persist a heap.
fn store(env: &Env, key: &Symbol, heap: &Vec<UserLeaderboardEntryV1>) {
    env.storage().persistent().set(key, heap);
}

/// Find the position of `user` in the heap, or `None`.
fn find_user(heap: &Vec<UserLeaderboardEntryV1>, user: &Address) -> Option<u32> {
    for i in 0..heap.len() {
        if &heap.get(i).unwrap().user == user {
            return Some(i);
        }
    }
    None
}

/// Return the index of the minimum-keyed element in `heap`.
fn min_index(heap: &Vec<UserLeaderboardEntryV1>, key_fn: fn(&UserLeaderboardEntryV1) -> i128) -> u32 {
    let mut min_idx = 0u32;
    let mut min_key = key_fn(&heap.get(0).unwrap());
    for i in 1..heap.len() {
        let k = key_fn(&heap.get(i).unwrap());
        if k < min_key {
            min_key = k;
            min_idx = i;
        }
    }
    min_idx
}

/// Replace the entry at `idx` with `entry`.
fn replace_at(heap: &mut Vec<UserLeaderboardEntryV1>, idx: u32, entry: UserLeaderboardEntryV1) {
    heap.remove(idx);
    heap.insert(idx, entry);
}

/// Core upsert logic shared by both leaderboards.
///
/// 1. If the user already exists, update their entry in place.
/// 2. If the heap is not full, append the new entry.
/// 3. If the heap is full and the candidate beats the current minimum, evict
///    the minimum and insert the candidate.
/// 4. Otherwise do nothing.
fn upsert(
    heap: &mut Vec<UserLeaderboardEntryV1>,
    env: &Env,
    user: &Address,
    capacity: u32,
    candidate_key: i128,
    stats: &UserLeaderboardEntryV1,
    key_fn: fn(&UserLeaderboardEntryV1) -> i128,
) {
    // Case 1 – update existing entry.
    if let Some(idx) = find_user(heap, user) {
        replace_at(heap, idx, stats.clone());
        return;
    }

    let entry = stats.clone();

    // Case 2 – room available.
    if heap.len() < capacity {
        heap.push_back(entry);
        return;
    }

    // Case 3 – evict minimum if candidate is better.
    if heap.is_empty() {
        return;
    }
    let min_idx = min_index(heap, key_fn);
    let min_key = key_fn(&heap.get(min_idx).unwrap());
    if candidate_key > min_key {
        replace_at(heap, min_idx, entry);
    }
    // Case 4 – candidate does not qualify; nothing to do.
    let _ = env; // env unused after storage ops above; suppress lint
}

/// Sort a heap copy descending by `key_fn`, cap at `limit`, assign ranks.
///
/// Uses insertion sort (O(N²)) which is fine for N ≤ 50.
fn sorted_top(
    env: &Env,
    heap: Vec<UserLeaderboardEntryV1>,
    limit: u32,
    key_fn: fn(&UserLeaderboardEntryV1) -> i128,
) -> Vec<UserLeaderboardEntryV1> {
    // Collect into a plain array-like structure using a soroban Vec.
    let mut sorted: Vec<UserLeaderboardEntryV1> = Vec::new(env);
    for i in 0..heap.len() {
        sorted.push_back(heap.get(i).unwrap());
    }

    // Insertion sort descending.
    let n = sorted.len();
    for i in 1..n {
        let mut j = i;
        while j > 0 {
            let a = key_fn(&sorted.get(j - 1).unwrap());
            let b = key_fn(&sorted.get(j).unwrap());
            if a >= b {
                break;
            }
            // swap j-1 and j
            let ea = sorted.get(j - 1).unwrap();
            let eb = sorted.get(j).unwrap();
            replace_at(&mut sorted, j - 1, eb);
            replace_at(&mut sorted, j, ea);
            j -= 1;
        }
    }

    // Trim to limit and assign ranks.
    let take = limit.min(sorted.len());
    let mut result: Vec<UserLeaderboardEntryV1> = Vec::new(env);
    for i in 0..take {
        let mut entry = sorted.get(i).unwrap();
        entry.rank = i.checked_add(1).unwrap_or(u32::MAX);
        result.push_back(entry);
    }
    result
}

// ── tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PredictifyHybrid;
    use soroban_sdk::{testutils::Address as _, Address, Env};

    fn make_entry(env: &Env, user: &Address, winnings: i128, win_rate: u32, bets: u64) -> UserLeaderboardEntryV1 {
        UserLeaderboardEntryV1 {
            user: user.clone(),
            rank: 0,
            total_winnings: winnings,
            win_rate,
            total_bets_placed: bets,
            winning_bets: 0,
            total_wagered: 0,
            last_activity: 0,
        }
    }

    fn setup() -> (Env, Address) {
        let env = Env::default();
        let cid = env.register_contract(None, PredictifyHybrid);
        (env, cid)
    }

    // ── push_winnings / top_by_winnings ────────────────────────────────────

    #[test]
    fn test_empty_heap_returns_empty() {
        let (env, cid) = setup();
        env.as_contract(&cid, || {
            let top = LeaderboardHeap::top_by_winnings(&env, 10);
            assert_eq!(top.len(), 0);
        });
    }

    #[test]
    fn test_single_entry_inserted_and_returned() {
        let (env, cid) = setup();
        let user = Address::generate(&env);
        env.as_contract(&cid, || {
            let entry = make_entry(&env, &user, 500, 6000, 10);
            LeaderboardHeap::push_winnings(&env, &user, 5, 500, &entry);
            let top = LeaderboardHeap::top_by_winnings(&env, 5);
            assert_eq!(top.len(), 1);
            assert_eq!(top.get(0).unwrap().total_winnings, 500);
            assert_eq!(top.get(0).unwrap().rank, 1);
        });
    }

    #[test]
    fn test_heap_keeps_top_n_by_winnings() {
        let (env, cid) = setup();
        env.as_contract(&cid, || {
            // Insert 6 entries into a capacity-3 heap.
            for w in [10i128, 50, 30, 80, 20, 60] {
                let user = Address::generate(&env);
                let entry = make_entry(&env, &user, w, 5000, 5);
                LeaderboardHeap::push_winnings(&env, &user, 3, w, &entry);
            }
            let top = LeaderboardHeap::top_by_winnings(&env, 3);
            assert_eq!(top.len(), 3);
            // Should contain 80, 60, 50 in descending order.
            assert_eq!(top.get(0).unwrap().total_winnings, 80);
            assert_eq!(top.get(1).unwrap().total_winnings, 60);
            assert_eq!(top.get(2).unwrap().total_winnings, 50);
        });
    }

    #[test]
    fn test_low_score_not_inserted_when_full() {
        let (env, cid) = setup();
        env.as_contract(&cid, || {
            // Fill a capacity-2 heap with high scores.
            for w in [100i128, 200] {
                let user = Address::generate(&env);
                let entry = make_entry(&env, &user, w, 5000, 5);
                LeaderboardHeap::push_winnings(&env, &user, 2, w, &entry);
            }
            // Attempt to insert a low score.
            let loser = Address::generate(&env);
            let entry = make_entry(&env, &loser, 1, 5000, 5);
            LeaderboardHeap::push_winnings(&env, &loser, 2, 1, &entry);

            let top = LeaderboardHeap::top_by_winnings(&env, 2);
            assert_eq!(top.len(), 2);
            // The low score should not appear.
            for i in 0..top.len() {
                assert!(top.get(i).unwrap().total_winnings >= 100);
            }
        });
    }

    #[test]
    fn test_existing_user_entry_is_updated() {
        let (env, cid) = setup();
        let user = Address::generate(&env);
        env.as_contract(&cid, || {
            let e1 = make_entry(&env, &user, 100, 5000, 2);
            LeaderboardHeap::push_winnings(&env, &user, 5, 100, &e1);

            // Update the same user with a higher score.
            let e2 = make_entry(&env, &user, 900, 8000, 5);
            LeaderboardHeap::push_winnings(&env, &user, 5, 900, &e2);

            let top = LeaderboardHeap::top_by_winnings(&env, 5);
            assert_eq!(top.len(), 1);
            assert_eq!(top.get(0).unwrap().total_winnings, 900);
        });
    }

    #[test]
    fn test_ranks_are_sequential() {
        let (env, cid) = setup();
        env.as_contract(&cid, || {
            for w in [40i128, 10, 70, 20] {
                let user = Address::generate(&env);
                let entry = make_entry(&env, &user, w, 5000, 5);
                LeaderboardHeap::push_winnings(&env, &user, 10, w, &entry);
            }
            let top = LeaderboardHeap::top_by_winnings(&env, 10);
            for i in 0..top.len() {
                assert_eq!(top.get(i).unwrap().rank, i + 1);
            }
        });
    }

    #[test]
    fn test_limit_caps_results() {
        let (env, cid) = setup();
        env.as_contract(&cid, || {
            for w in [10i128, 20, 30, 40, 50] {
                let user = Address::generate(&env);
                let entry = make_entry(&env, &user, w, 5000, 5);
                LeaderboardHeap::push_winnings(&env, &user, 10, w, &entry);
            }
            let top = LeaderboardHeap::top_by_winnings(&env, 3);
            assert_eq!(top.len(), 3);
        });
    }

    #[test]
    fn test_capacity_clamped_to_max_capacity() {
        let (env, cid) = setup();
        env.as_contract(&cid, || {
            // Passing capacity > MAX_CAPACITY should be silently clamped.
            let user = Address::generate(&env);
            let entry = make_entry(&env, &user, 999, 9000, 100);
            // Should not panic.
            LeaderboardHeap::push_winnings(&env, &user, 9999, 999, &entry);
            let top = LeaderboardHeap::top_by_winnings(&env, 9999);
            assert_eq!(top.len(), 1);
        });
    }

    // ── push_win_rate / top_by_win_rate ────────────────────────────────────

    #[test]
    fn test_win_rate_heap_basic() {
        let (env, cid) = setup();
        env.as_contract(&cid, || {
            for (rate, bets) in [(8000u32, 20u64), (9000, 5), (7000, 50)] {
                let user = Address::generate(&env);
                let entry = make_entry(&env, &user, 0, rate, bets);
                LeaderboardHeap::push_win_rate(&env, &user, 10, &entry);
            }
            let top = LeaderboardHeap::top_by_win_rate(&env, 10);
            assert_eq!(top.len(), 3);
            // 9000 should be first.
            assert_eq!(top.get(0).unwrap().win_rate, 9000);
        });
    }

    #[test]
    fn test_win_rate_heap_keeps_top_n() {
        let (env, cid) = setup();
        env.as_contract(&cid, || {
            for (rate, bets) in [
                (5000u32, 10u64),
                (6000, 10),
                (7000, 10),
                (3000, 10),
                (8000, 10),
            ] {
                let user = Address::generate(&env);
                let entry = make_entry(&env, &user, 0, rate, bets);
                LeaderboardHeap::push_win_rate(&env, &user, 3, &entry);
            }
            let top = LeaderboardHeap::top_by_win_rate(&env, 3);
            assert_eq!(top.len(), 3);
            // Top 3 should be 8000, 7000, 6000.
            assert_eq!(top.get(0).unwrap().win_rate, 8000);
            assert_eq!(top.get(1).unwrap().win_rate, 7000);
            assert_eq!(top.get(2).unwrap().win_rate, 6000);
        });
    }

    #[test]
    fn test_win_rate_tie_broken_by_volume() {
        let (env, cid) = setup();
        env.as_contract(&cid, || {
            // Three users with the same win-rate; more bets → higher rank.
            let users: [Address; 3] = [
                Address::generate(&env),
                Address::generate(&env),
                Address::generate(&env),
            ];
            let volumes = [5u64, 20, 10];
            for (u, bets) in users.iter().zip(volumes.iter()) {
                let entry = make_entry(&env, u, 0, 7000, *bets);
                LeaderboardHeap::push_win_rate(&env, u, 10, &entry);
            }
            let top = LeaderboardHeap::top_by_win_rate(&env, 10);
            assert_eq!(top.len(), 3);
            assert_eq!(top.get(0).unwrap().total_bets_placed, 20);
            assert_eq!(top.get(1).unwrap().total_bets_placed, 10);
            assert_eq!(top.get(2).unwrap().total_bets_placed, 5);
        });
    }

    // ── edge cases ─────────────────────────────────────────────────────────

    #[test]
    fn test_capacity_one() {
        let (env, cid) = setup();
        env.as_contract(&cid, || {
            let a = Address::generate(&env);
            let b = Address::generate(&env);
            let ea = make_entry(&env, &a, 10, 5000, 1);
            let eb = make_entry(&env, &b, 50, 5000, 1);
            LeaderboardHeap::push_winnings(&env, &a, 1, 10, &ea);
            LeaderboardHeap::push_winnings(&env, &b, 1, 50, &eb);
            let top = LeaderboardHeap::top_by_winnings(&env, 1);
            assert_eq!(top.len(), 1);
            assert_eq!(top.get(0).unwrap().total_winnings, 50);
        });
    }

    #[test]
    fn test_zero_winnings_allowed() {
        let (env, cid) = setup();
        let user = Address::generate(&env);
        env.as_contract(&cid, || {
            let entry = make_entry(&env, &user, 0, 0, 0);
            LeaderboardHeap::push_winnings(&env, &user, 5, 0, &entry);
            let top = LeaderboardHeap::top_by_winnings(&env, 5);
            assert_eq!(top.len(), 1);
            assert_eq!(top.get(0).unwrap().total_winnings, 0);
        });
    }

    #[test]
    fn test_i128_max_winnings() {
        let (env, cid) = setup();
        let user = Address::generate(&env);
        env.as_contract(&cid, || {
            let entry = make_entry(&env, &user, i128::MAX, 10000, 1);
            LeaderboardHeap::push_winnings(&env, &user, 5, i128::MAX, &entry);
            let top = LeaderboardHeap::top_by_winnings(&env, 5);
            assert_eq!(top.get(0).unwrap().total_winnings, i128::MAX);
        });
    }
}
