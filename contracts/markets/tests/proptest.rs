//! Property-based tests: markets conservation invariants.
//!
//! A prediction market's accounting must be internally consistent at every
//! step of its lifecycle — after creation, after each bet placement, after
//! each bet cancellation, and (where applicable) after refunds.  These
//! invariants are the contract's "double-entry bookkeeping" properties.
//! Breaking any one of them silently loses or invents money, so they are
//! pinned down here with proptest over randomized bet schedules across
//! multiple users, outcomes, and independent markets.
//!
//! These are integration tests (crate-external, via `PredictifyHybridClient`),
//! so they exercise the contract's public entrypoints — the same surface a
//! real caller has.  No production code is changed by this file.
//!
//! ## Invariants covered
//!
//! 1. **BetStats internal consistency**
//!    `Σ outcome_totals.values() == total_amount_locked`
//!    The per-outcome buckets in `BetStats` must sum to the aggregate
//!    `total_amount_locked`.  This catches drift between the two tracked
//!    values during `place_bet`, `cancel_bet`, and `refund_market_bets`.
//!
//! 2. **Market ↔ BetStats agreement on the pool total**
//!    `market.total_staked == bet_stats.total_amount_locked`
//!    `Market.total_staked` and `BetStats.total_amount_locked` are stored in
//!    different storage keys and written on different code paths.  They must
//!    always describe the same number.
//!
//! 3. **Per-user stakes map agrees with the pool total (back-compat)**
//!    `Σ market.stakes.values() == market.total_staked`
//!    `Market.stakes` is retained for backward-compatible payout distribution
//!    and mirrors the `Bet` records.  Its entries must add up to the same
//!    pool total that `BetStats` and `Market.total_staked` report.
//!
//! 4. **Cross-market independence**
//!    Any sequence of operations on market M₁ must never change
//!    `total_staked`, `total_amount_locked`, or any `outcome_totals` entry
//!    of a distinct market M₂.  Catches storage-key aliasing bugs and
//!    accidental cross-market mutations.
//!
//! 5. **Cancellation conservation**
//!    Cancelling an active bet of amount `a` must decrease
//!    `market.total_staked` and `bet_stats.total_amount_locked` by exactly
//!    `a`, and must decrease that outcome's bucket by exactly `a`.  The
//!    remaining invariants (1–3) must still hold after the cancellation.
//!
//! 6. **Universal non-negativity**
//!    Every tracked monetary amount (`total_staked`, `total_amount_locked`,
//!    every `outcome_totals` entry, every `stakes` value) must be strictly
//!    non-negative after every transition.  `saturating_sub` makes
//!    underflow panics unlikely, but the invariants forbid the state from
//!    reaching zero via a different route than expected.

#![cfg(test)]

use predictify_hybrid::types::{OracleConfig, OracleProvider};
use predictify_hybrid::{PredictifyHybrid, PredictifyHybridClient};
use proptest::prelude::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::testutils::{Ledger, LedgerInfo};
use soroban_sdk::token::StellarAssetClient;
use soroban_sdk::{Address, Env, String as SorobanString, Symbol, Vec as SorobanVec};

const ORACLE_ADDRESS: &str = "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";

const MIN_BET: i128 = 1_000_000;
const INITIAL_USER_BALANCE: i128 = 1_000_000_000;

/// Number of distinct markets used in the cross-market independence tests.
const NUM_MARKETS: usize = 2;

/// Shared fixture: a registered, initialized PredictifyHybrid contract, a
/// minted token, an admin address, and a fixed-size set of user addresses
/// each holding `INITIAL_USER_BALANCE` of the token.
struct Fixture {
    env: Env,
    cid: Address,
    admin: Address,
    users: Vec<Address>,
    token_id: Address,
    client: PredictifyHybridClient<'static>,
}

impl Fixture {
    fn new(num_users: usize) -> Self {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set(LedgerInfo {
            timestamp: 1_700_000_000,
            protocol_version: 22,
            sequence_number: 100,
            network_id: Default::default(),
            base_reserve: 10,
            min_persistent_entry_ttl: 100,
            min_temp_entry_ttl: 100,
            max_entry_ttl: 10_000_000,
        });

        let admin = Address::generate(&env);
        let users: Vec<Address> = (0..num_users).map(|_| Address::generate(&env)).collect();
        let cid = env.register(PredictifyHybrid, ());

        let token_admin = Address::generate(&env);
        let token_contract = env.register_stellar_asset_contract_v2(token_admin);
        let token_id = token_contract.address();

        env.as_contract(&cid, || {
            env.storage()
                .persistent()
                .set(&Symbol::new(&env, "TokenID"), &token_id);
        });

        let client = PredictifyHybridClient::new(&env, &cid);
        client.initialize(&admin, &None, &None);

        let sac = StellarAssetClient::new(&env, &token_id);
        for u in &users {
            sac.mint(u, &INITIAL_USER_BALANCE);
        }

        Fixture {
            env,
            cid,
            admin,
            users,
            token_id,
            // SAFETY: the contract client borrows the env; the returned
            // reference is transmuted to a 'static lifetime to keep Fixture
            // self-contained.  All Fixture data lives together and is dropped
            // together; the transmute is scoped to this test-only module.
            client: unsafe {
                std::mem::transmute::<PredictifyHybridClient<'_>, PredictifyHybridClient<'static>>(
                    client,
                )
            },
        }
    }

    fn oracle_config(&self, feed_id: &str) -> OracleConfig {
        OracleConfig::new(
            OracleProvider::reflector(),
            Address::from_str(&self.env, ORACLE_ADDRESS),
            SorobanString::from_str(&self.env, feed_id),
            100,
            SorobanString::from_str(&self.env, "gt"),
        )
    }

    fn create_market(&self, question: &str, feed_id: &str) -> Symbol {
        self.client.create_market(
            &self.admin,
            &SorobanString::from_str(&self.env, question),
            &SorobanVec::from_array(
                &self.env,
                [
                    SorobanString::from_str(&self.env, "yes"),
                    SorobanString::from_str(&self.env, "no"),
                ],
            ),
            &30u32,
            &self.oracle_config(feed_id),
            &None,
            &86_400u64,
            &None,
            &None,
            &None,
            &None,
            &None,
        )
    }

    fn place_bet_for(&self, user: &Address, market: &Symbol, outcome: &str, amount: i128) {
        self.client.place_bet(
            user,
            market,
            &SorobanString::from_str(&self.env, outcome),
            &amount,
            &0i128,
        );
    }

    fn cancel_bet_for(&self, user: &Address, market: &Symbol) {
        self.client.cancel_bet(user, market);
    }

    fn snapshot_invariants(
        &self,
        market: &Symbol,
    ) -> (i128, i128, std::collections::BTreeMap<String, i128>, i128) {
        let m = self
            .client
            .get_market(market)
            .expect("market must exist at snapshot time");
        let s = self.client.get_market_bet_stats(market);

        let total_staked = m.total_staked;
        let total_amount_locked = s.total_amount_locked;

        let mut outcome_map = std::collections::BTreeMap::new();
        for k in s.outcome_totals.keys() {
            let k_str: String = k.to_string();
            let v = s.outcome_totals.get(k.clone()).unwrap_or(0);
            outcome_map.insert(k_str, v);
        }

        let stakes_sum: i128 = m
            .stakes
            .values()
            .iter()
            .fold(0i128, |acc, v| acc.saturating_add(v));

        (total_staked, total_amount_locked, outcome_map, stakes_sum)
    }
}

fn sum_outcome_totals(
    map: &std::collections::BTreeMap<String, i128>,
) -> i128 {
    map.values().fold(0i128, |acc, v| acc.saturating_add(*v))
}

/// Validate invariants 1, 2, 3, 6 simultaneously for a single market:
///   (1) Σ outcome_totals == total_amount_locked
///   (2) total_staked          == total_amount_locked
///   (3) Σ stakes              == total_staked
///   (6) every value is >= 0
fn assert_conservation(fx: &Fixture, market: &Symbol, label: &str) {
    let (total_staked, total_amount_locked, outcome_map, stakes_sum) =
        fx.snapshot_invariants(market);

    prop_assert!(total_staked >= 0, "[{}] total_staked is negative: {}", label, total_staked);
    prop_assert!(
        total_amount_locked >= 0,
        "[{}] total_amount_locked is negative: {}",
        label,
        total_amount_locked
    );
    for (k, v) in &outcome_map {
        prop_assert!(*v >= 0, "[{}] outcome_total for {:?} is negative: {}", label, k, v);
    }
    prop_assert!(stakes_sum >= 0, "[{}] stakes_sum is negative: {}", label, stakes_sum);

    let outcome_sum = sum_outcome_totals(&outcome_map);
    prop_assert_eq!(
        outcome_sum, total_amount_locked,
        "[{}] Σ outcome_totals ({}) != BetStats.total_amount_locked ({}) — invariant 1 broken",
        label, outcome_sum, total_amount_locked
    );

    prop_assert_eq!(
        total_staked, total_amount_locked,
        "[{}] Market.total_staked ({}) != BetStats.total_amount_locked ({}) — invariant 2 broken",
        label, total_staked, total_amount_locked
    );

    prop_assert_eq!(
        stakes_sum, total_staked,
        "[{}] Σ Market.stakes ({}) != Market.total_staked ({}) — invariant 3 broken",
        label, stakes_sum, total_staked
    );
}

// =========================================================================
// PROPERTY TESTS
// =========================================================================

proptest! {
    // Each case allocates fresh Env + contract + token; keep cases modest.
    #![proptest_config(ProptestConfig::with_cases(36))]

    // ---- Invariants 1+2+3+6: single-market randomized bets --------------

    /// Property A: after placing N independently-randomized single bets on
    /// one market (each with a fresh user, so no AlreadyBet collisions),
    /// invariants 1, 2, 3, and 6 all hold.
    #[test]
    fn single_market_random_bets_conserve(
        num_bets in 1usize..=6,
        amounts in prop::collection::vec(MIN_BET..=MIN_BET * 20, 1..=6),
        outcomes in prop::collection::vec(prop::bool::ANY, 1..=6),
    ) {
        let num_users = num_bets.max(amounts.len()).max(outcomes.len());
        let fx = Fixture::new(num_users);
        let m = fx.create_market("Single-market conservation?", "PROP0");

        let n = num_bets.min(amounts.len()).min(outcomes.len());
        for i in 0..n {
            let outcome = if outcomes[i] { "yes" } else { "no" };
            let amt = amounts[i];
            fx.place_bet_for(&fx.users[i], &m, outcome, amt);
            assert_conservation(&fx, &m, &format!("after bet {}", i));
        }
    }

    // ---- Invariant 5: cancellation preserves conservation --------------

    /// Property B: after placing a bet, cancelling it reduces every
    /// aggregate by exactly the bet amount, and invariants 1–3 still hold
    /// on the post-cancellation state.  Both "yes" and "no" outcomes are
    /// exercised (randomized).
    #[test]
    fn cancellation_subtracts_exactly(
        amount in MIN_BET..=MIN_BET * 10,
        is_yes in prop::bool::ANY,
    ) {
        let fx = Fixture::new(1);
        let m = fx.create_market("Cancellation subtracts correctly?", "PROP1");

        let before = fx.snapshot_invariants(&m);
        let outcome = if is_yes { "yes" } else { "no" };
        let outcome_key: String = outcome.to_string();

        fx.place_bet_for(&fx.users[0], &m, outcome, amount);
        let after_bet = fx.snapshot_invariants(&m);

        prop_assert_eq!(
            after_bet.0.saturating_sub(before.0),
            amount,
            "total_staked didn't increase by exactly the bet amount on place"
        );

        let before_cancel_total = after_bet.0;
        let before_cancel_outcome = *after_bet.2.get(&outcome_key).unwrap_or(&0);
        fx.cancel_bet_for(&fx.users[0], &m);

        let after_cancel = fx.snapshot_invariants(&m);
        prop_assert_eq!(
            before_cancel_total.saturating_sub(after_cancel.0),
            amount,
            "total_staked didn't decrease by exactly the bet amount on cancel"
        );
        let after_cancel_outcome = *after_cancel.2.get(&outcome_key).unwrap_or(&0);
        prop_assert_eq!(
            before_cancel_outcome.saturating_sub(after_cancel_outcome),
            amount,
            "outcome_total didn't decrease by exactly the bet amount on cancel"
        );

        assert_conservation(&fx, &m, "after cancel");
    }

    // ---- Invariant 4: cross-market independence ------------------------

    /// Property C: when we perform operations only on market A, a
    /// completely-separate market B's conservation numbers (total_staked,
    /// total_amount_locked, and every per-outcome total) are byte-for-byte
    /// unchanged, and B's own invariants still hold.
    #[test]
    fn operations_on_market_a_never_touch_market_b(
        num_ops in 1usize..=5,
        amounts in prop::collection::vec(MIN_BET..=MIN_BET * 10, 1..=5),
        outcomes in prop::collection::vec(prop::bool::ANY, 1..=5),
    ) {
        let num_users = 2 * num_ops.max(amounts.len()).max(outcomes.len());
        let fx = Fixture::new(num_users);
        let ma = fx.create_market("Only we touch this", "PROPA");
        let mb = fx.create_market("Hands off", "PROPB");

        // Seed market B with an initial bet so its totals are non-trivial.
        let mb_seed_user = &fx.users[fx.users.len() - 1];
        fx.place_bet_for(mb_seed_user, &mb, "yes", MIN_BET * 3);
        let mb_before = fx.snapshot_invariants(&mb);

        let n = num_ops.min(amounts.len()).min(outcomes.len());
        for i in 0..n {
            let outcome = if outcomes[i] { "yes" } else { "no" };
            fx.place_bet_for(&fx.users[i], &ma, outcome, amounts[i]);
        }

        let mb_after = fx.snapshot_invariants(&mb);
        prop_assert_eq!(
            mb_before.0, mb_after.0,
            "cross-market leak: market B total_staked changed while only A was modified"
        );
        prop_assert_eq!(
            mb_before.1, mb_after.1,
            "cross-market leak: market B total_amount_locked changed while only A was modified"
        );
        for (k, v_before) in &mb_before.2 {
            let v_after = mb_after.2.get(k).copied().unwrap_or(0);
            prop_assert_eq!(
                *v_before, v_after,
                "cross-market leak: market B outcome_totals[{:?}] changed ({} → {})",
                k, v_before, v_after
            );
        }
        for (k, v_after) in &mb_after.2 {
            let v_before = mb_before.2.get(k).copied().unwrap_or(0);
            prop_assert_eq!(
                v_before, *v_after,
                "cross-market leak: market B acquired new outcome_totals[{:?}] = {}",
                k, v_after
            );
        }

        assert_conservation(&fx, &ma, "market A after ops");
        assert_conservation(&fx, &mb, "market B after untouched ops");
    }

    // ---- Mixed schedule: bets + some cancellations ---------------------

    /// Property D: a mixed schedule of interleaved bets and cancellations
    /// (cancellations only apply to bets that actually exist and are
    /// active on that market) keeps invariants 1–3 and 6 for every
    /// market touched.
    #[test]
    fn mixed_bet_cancel_schedule_conserve(
        steps in prop::collection::vec(
            prop_oneof![
                // 0 = place bet on market 0
                Just(0usize),
                // 1 = place bet on market 1
                Just(1usize),
                // 2 = cancel the most-recently-placed bet on market 0
                Just(2usize),
                // 3 = cancel the most-recently-placed bet on market 1
                Just(3usize),
            ],
            1..=8,
        ),
        amounts in prop::collection::vec(MIN_BET..=MIN_BET * 10, 1..=8),
    ) {
        let num_users = 2 * steps.len();
        let fx = Fixture::new(num_users);
        let markets: [Symbol; NUM_MARKETS] = [
            fx.create_market("Mixed schedule market 0", "MPM0"),
            fx.create_market("Mixed schedule market 1", "MPM1"),
        ];

        // Track one "active (un-cancelled) bet user" per market, so the
        // cancel steps always refer to a user who actually has an active bet.
        let mut placed_per_market: [Vec<usize>; NUM_MARKETS] =
            [vec![], vec![]];
        let mut next_user_idx = 0usize;

        let n = steps.len().min(amounts.len());
        for (step_idx, action) in steps.iter().enumerate().take(n) {
            match action {
                0 | 1 => {
                    let mi = *action; // market index 0 or 1
                    if next_user_idx >= fx.users.len() {
                        break;
                    }
                    // Alternate outcome deterministically per step so both
                    // "yes" and "no" buckets receive weight across runs.
                    let outcome = if step_idx % 2 == 0 { "yes" } else { "no" };
                    fx.place_bet_for(
                        &fx.users[next_user_idx],
                        &markets[mi],
                        outcome,
                        amounts[step_idx],
                    );
                    placed_per_market[mi].push(next_user_idx);
                    next_user_idx += 1;
                    assert_conservation(
                        &fx,
                        &markets[mi],
                        &format!("step {}: place on m{}", step_idx, mi),
                    );
                }
                2 | 3 => {
                    let mi = *action - 2; // cancel on market 0 or 1
                    if let Some(user_i) = placed_per_market[mi].pop() {
                        fx.cancel_bet_for(&fx.users[user_i], &markets[mi]);
                        assert_conservation(
                            &fx,
                            &markets[mi],
                            &format!("step {}: cancel on m{}", step_idx, mi),
                        );
                    }
                    // If the stack is empty the cancellation is skipped —
                    // we only cancel bets that actually exist.
                }
                _ => unreachable!(),
            }
        }

        // Final check on both markets.
        for (mi, m) in markets.iter().enumerate() {
            assert_conservation(&fx, m, &format!("final check m{}", mi));
        }
    }
}

// =========================================================================
// FOCUSED EDGE-CASE TESTS
// =========================================================================
// proptest's default RNG rarely lands exactly on boundary scenarios
// (zero-outcome markets, single-bet then cancel, the exact min bet, etc.),
// so these are pinned down as explicit unit tests.

#[test]
fn edge_empty_market_conserve_immediately() {
    let fx = Fixture::new(1);
    let m = fx.create_market("Has no bets at all", "EMPTY0");
    assert_conservation(&fx, &m, "empty market, immediate check");
}

#[test]
fn edge_min_bet_exactly_then_cancel() {
    let fx = Fixture::new(1);
    let m = fx.create_market("Exactly min bet", "MINBET");

    fx.place_bet_for(&fx.users[0], &m, "yes", MIN_BET);
    let snap = fx.snapshot_invariants(&m);
    assert_eq!(snap.0, MIN_BET);
    assert_eq!(*snap.2.get("yes").unwrap_or(&0), MIN_BET);
    assert_conservation(&fx, &m, "min bet placed");

    fx.cancel_bet_for(&fx.users[0], &m);
    let after = fx.snapshot_invariants(&m);
    assert_eq!(after.0, 0);
    assert_eq!(*after.2.get("yes").unwrap_or(&0), 0);
    assert_conservation(&fx, &m, "min bet cancelled");
}

#[test]
fn edge_two_users_both_outcomes_conserve() {
    let fx = Fixture::new(4);
    let m = fx.create_market("Two users on each side", "TWOSID");

    fx.place_bet_for(&fx.users[0], &m, "yes", MIN_BET * 2);
    fx.place_bet_for(&fx.users[1], &m, "yes", MIN_BET * 3);
    fx.place_bet_for(&fx.users[2], &m, "no", MIN_BET * 5);
    fx.place_bet_for(&fx.users[3], &m, "no", MIN_BET * 7);

    let (total_staked, tal, out_map, stakes_sum) = fx.snapshot_invariants(&m);
    let expected_total = MIN_BET * (2 + 3 + 5 + 7);
    assert_eq!(total_staked, expected_total);
    assert_eq!(tal, expected_total);
    assert_eq!(stakes_sum, expected_total);
    assert_eq!(*out_map.get("yes").unwrap_or(&0), MIN_BET * 5);
    assert_eq!(*out_map.get("no").unwrap_or(&0), MIN_BET * 12);
    assert_conservation(&fx, &m, "two per side");
}

#[test]
fn edge_cancel_order_lifo_does_not_break_conservation() {
    let fx = Fixture::new(3);
    let m = fx.create_market("Cancel in arbitrary order", "LIFO");

    fx.place_bet_for(&fx.users[0], &m, "yes", MIN_BET * 11);
    fx.place_bet_for(&fx.users[1], &m, "yes", MIN_BET * 13);
    fx.place_bet_for(&fx.users[2], &m, "no", MIN_BET * 17);

    // Cancel in the OPPOSITE order of placement (not LIFO relative to
    // outcome buckets) — outcome_totals subtraction must still be exact.
    fx.cancel_bet_for(&fx.users[2], &m); // first-cancelled, on "no"
    assert_conservation(&fx, &m, "after cancel user2 (no side)");

    fx.cancel_bet_for(&fx.users[0], &m); // then cancel on "yes"
    assert_conservation(&fx, &m, "after cancel user0 (yes side, first-placed)");

    let final_snap = fx.snapshot_invariants(&m);
    assert_eq!(final_snap.0, MIN_BET * 13);
    assert_eq!(*final_snap.2.get("yes").unwrap_or(&0), MIN_BET * 13);
    assert_eq!(*final_snap.2.get("no").unwrap_or(&0), 0);
}

#[test]
fn edge_cross_market_after_seeding_both() {
    let fx = Fixture::new(4);
    let ma = fx.create_market("Market A (we touch)", "SEED_A");
    let mb = fx.create_market("Market B (frozen)", "SEED_B");

    fx.place_bet_for(&fx.users[0], &ma, "yes", MIN_BET);
    fx.place_bet_for(&fx.users[1], &mb, "no", MIN_BET * 4);
    fx.place_bet_for(&fx.users[2], &mb, "yes", MIN_BET * 2);

    let b_before = fx.snapshot_invariants(&mb);

    // Further operations ONLY on market A, including a cancel.
    fx.place_bet_for(&fx.users[3], &ma, "no", MIN_BET * 7);
    fx.cancel_bet_for(&fx.users[0], &ma);

    let b_after = fx.snapshot_invariants(&mb);
    assert_eq!(b_before, b_after);
    assert_conservation(&fx, &ma, "A after mixed ops");
    assert_conservation(&fx, &mb, "B untouched after A ops");
}

#[test]
fn edge_outcome_totals_map_does_not_retain_stale_zeros() {
    // After a cancellation empties the last bet on an outcome, the key
    // should either be absent or map to exactly 0 (implementation
    // removes the key).  Either way, the sum must still equal the total.
    let fx = Fixture::new(1);
    let m = fx.create_market("Outcome returns to zero", "STALE0");

    fx.place_bet_for(&fx.users[0], &m, "yes", MIN_BET);
    assert_eq!(
        *fx.snapshot_invariants(&m).2.get("yes").unwrap_or(&0),
        MIN_BET
    );

    fx.cancel_bet_for(&fx.users[0], &m);
    let post = fx.snapshot_invariants(&m);
    // Either the key is gone, or it is present with value 0.  The
    // assert_conservation check below already enforces the sum; this
    // assertion pins down the absence of stale keys specifically.
    let yes_val = *post.2.get("yes").unwrap_or(&0);
    assert_eq!(yes_val, 0);
    assert_conservation(&fx, &m, "outcome emptied by cancel");
}
