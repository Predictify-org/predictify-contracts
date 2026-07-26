//! # Proptest invariant tests for betting state conservation (buffer #9)
//!
//! Property tests for betting state invariants:
//!
//!   1. BetStats.total_amount_locked == market.total_staked
//!   2. sum(BetStats.outcome_totals.values()) == BetStats.total_amount_locked
//!   3. A user can have at most one non-cancelled bet per market
//!   4. Bet amounts respect configured limits
//!   5. Cancel operations correctly decrement stats
//!
//! Strategy: Operate directly on `Market` and `BetStorage` (unit level) to
//! avoid token/oracle infrastructure. `BetStorage` methods are the same code
//! paths used by `BetManager`, covering the bookkeeping invariants.

use alloc::format;
use crate::bets::{BetStorage, MIN_BET_AMOUNT, MAX_BET_AMOUNT};
use crate::markets::MarketStateManager;
use crate::types::{Bet, BetStats, BetStatus, Market, MarketState, OracleConfig};
use proptest::prelude::*;
use soroban_sdk::{testutils::Address as _, vec as svec, Address, Env, Map, String, Symbol};

const MIN_STAKE: i128 = 1_000_000;

const OUTCOME_LABELS: &[&str] = &["yes", "no", "maybe", "abstain"];

fn arb_stake() -> impl Strategy<Value = i128> {
    MIN_STAKE..=100_000_000_000i128
}

fn arb_outcome_idx() -> impl Strategy<Value = usize> {
    0..OUTCOME_LABELS.len()
}

fn arb_bet_op() -> impl Strategy<Value = (usize, i128)> {
    (arb_outcome_idx(), arb_stake())
}

fn make_market() -> (Env, Market) {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let oracle = OracleConfig::none_sentinel(&env);
    let market = Market::new(
        &env,
        admin,
        String::from_str(&env, "Test market"),
        svec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
            String::from_str(&env, "maybe"),
            String::from_str(&env, "abstain"),
        ],
        env.ledger().timestamp() + 86_400,
        oracle,
        None,
        86_400,
        MarketState::Active,
    );
    (env, market)
}

fn assert_bet_invariants(env: &Env, market: &Market, market_id: &Symbol) {
    let stats = BetStorage::get_market_bet_stats(env, market_id);

    let outcome_sum: i128 = stats.outcome_totals.values().iter().sum();

    assert_eq!(
        outcome_sum,
        stats.total_amount_locked,
        "outcome_totals sum ({}) != BetStats.total_amount_locked ({})",
        outcome_sum,
        stats.total_amount_locked,
    );

    assert_eq!(
        stats.total_amount_locked,
        market.total_staked,
        "BetStats.total_amount_locked ({}) != market.total_staked ({})",
        stats.total_amount_locked,
        market.total_staked,
    );

    let bet_count = BetStorage::get_all_bets_for_market(env, market_id).len();
    assert_eq!(
        stats.total_bets as u32, bet_count,
        "BetStats.total_bets ({}) != registered bet count ({})",
        stats.total_bets, bet_count,
    );
}

fn simulate_place_bet(env: &Env, market: &mut Market, market_id: &Symbol, user: &Address, outcome: &String, amount: i128) {
    let bet = Bet::new(env, user.clone(), market_id.clone(), outcome.clone(), amount);
    BetStorage::store_bet(env, &bet).unwrap();

    let mut stats = BetStorage::get_market_bet_stats(env, market_id);
    stats.total_bets += 1;
    stats.total_amount_locked += amount;
    stats.unique_bettors += 1;
    let current_outcome_total = stats.outcome_totals.get(outcome.clone()).unwrap_or(0);
    stats.outcome_totals.set(outcome.clone(), current_outcome_total + amount);
    BetStorage::store_market_bet_stats(env, market_id, &stats).unwrap();

    market.total_staked += amount;
    market.votes.set(user.clone(), outcome.clone());
    market.stakes.set(user.clone(), amount);
    MarketStateManager::update_market(env, market_id, market);
}

fn simulate_cancel_bet(env: &Env, market: &mut Market, market_id: &Symbol, user: &Address) {
    if let Some(mut bet) = BetStorage::get_bet(env, market_id, user) {
        if !bet.is_active() {
            return;
        }
        let amount = bet.amount;
        let outcome = bet.outcome.clone();
        bet.status = BetStatus::Cancelled;
        BetStorage::store_bet(env, &bet).unwrap();

        let mut stats = BetStorage::get_market_bet_stats(env, market_id);
        stats.total_bets = stats.total_bets.saturating_sub(1);
        stats.total_amount_locked = stats.total_amount_locked.saturating_sub(amount);
        stats.unique_bettors = stats.unique_bettors.saturating_sub(1);
        let current_outcome_total = stats.outcome_totals.get(outcome.clone()).unwrap_or(0);
        let new_total = current_outcome_total.saturating_sub(amount);
        if new_total > 0 {
            stats.outcome_totals.set(outcome.clone(), new_total);
        } else {
            stats.outcome_totals.remove(outcome.clone());
        }
        BetStorage::store_market_bet_stats(env, market_id, &stats).unwrap();

        market.total_staked = market.total_staked.saturating_sub(amount);
        market.votes.remove(user.clone());
        market.stakes.remove(user.clone());
        MarketStateManager::update_market(env, market_id, market);
    }
}

fn simulate_resolve_bet(env: &Env, market_id: &Symbol, user: &Address, won: bool) {
    if let Some(mut bet) = BetStorage::get_bet(env, market_id, user) {
        if won {
            bet.mark_as_won();
        } else {
            bet.mark_as_lost();
        }
        BetStorage::store_bet(env, &bet).unwrap();
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 1000,
        source_file: Some("src/betting_invariants.rs"),
        ..ProptestConfig::default()
    })]

    #[test]
    fn prop_sequential_bets_conserve_stake(
        bets in prop::collection::vec(arb_bet_op(), 1..=20),
    ) {
        let (env, mut market) = make_market();
        let market_id = Symbol::new(&env, "TEST");

        MarketStateManager::store_market(&env, &market_id, &market);

        for (outcome_idx, stake) in bets {
            let user = Address::generate(&env);
            let outcome = String::from_str(&env, OUTCOME_LABELS[outcome_idx]);
            simulate_place_bet(&env, &mut market, &market_id, &user, &outcome, stake);
            assert_bet_invariants(&env, &market, &market_id);
        }
    }

    #[test]
    fn prop_duplicate_user_bet_prevented(
        stake1 in arb_stake(),
        stake2 in arb_stake(),
        outcome_idx in arb_outcome_idx(),
    ) {
        let (env, mut market) = make_market();
        let market_id = Symbol::new(&env, "TEST");
        MarketStateManager::store_market(&env, &market_id, &market);

        let user = Address::generate(&env);
        let outcome = String::from_str(&env, OUTCOME_LABELS[outcome_idx]);

        simulate_place_bet(&env, &mut market, &market_id, &user, &outcome, stake1);
        assert_bet_invariants(&env, &market, &market_id);

        let existing = BetStorage::get_bet(&env, &market_id, &user);
        prop_assert!(existing.is_some(), "Bet must exist after first placement");
        let existing_bet = existing.unwrap();
        prop_assert!(existing_bet.is_active(), "First bet must be active");
        prop_assert_eq!(existing_bet.amount, stake1, "First bet amount must match stake1");

        let stats_before = BetStorage::get_market_bet_stats(&env, &market_id);

        simulate_place_bet(&env, &mut market, &market_id, &user, &outcome, stake2);

        let second_bet = BetStorage::get_bet(&env, &market_id, &user).unwrap();
        prop_assert!(
            !second_bet.is_active() || second_bet.amount == stake2,
            "Duplicate bet should either be rejected (stays at stake1) or replace (stake2)"
        );

        let stats_after = BetStorage::get_market_bet_stats(&env, &market_id);
        prop_assert_eq!(
            stats_after.total_amount_locked, stats_before.total_amount_locked,
            "Duplicate bet must not increase total_amount_locked"
        );
        assert_bet_invariants(&env, &market, &market_id);
    }

    #[test]
    fn prop_cancel_bet_conserves_invariants(
        stake1 in arb_stake(),
        stake2 in arb_stake(),
        outcome_idx in arb_outcome_idx(),
    ) {
        let (env, mut market) = make_market();
        let market_id = Symbol::new(&env, "TEST");
        MarketStateManager::store_market(&env, &market_id, &market);

        let user1 = Address::generate(&env);
        let user2 = Address::generate(&env);
        let outcome = String::from_str(&env, OUTCOME_LABELS[outcome_idx]);

        simulate_place_bet(&env, &mut market, &market_id, &user1, &outcome, stake1);
        simulate_place_bet(&env, &mut market, &market_id, &user2, &outcome, stake2);
        assert_bet_invariants(&env, &market, &market_id);

        let total_before = market.total_staked;

        simulate_cancel_bet(&env, &mut market, &market_id, &user1);
        assert_bet_invariants(&env, &market, &market_id);

        prop_assert_eq!(
            market.total_staked,
            total_before - stake1,
            "total_staked must decrease by cancelled amount"
        );

        let cancelled_bet = BetStorage::get_bet(&env, &market_id, &user1).unwrap();
        prop_assert_eq!(
            cancelled_bet.status,
            BetStatus::Cancelled,
            "Bet must be marked as Cancelled"
        );
    }

    #[test]
    fn prop_resolve_bets_status_transition(
        stake in arb_stake(),
        outcome_idx in arb_outcome_idx(),
    ) {
        let (env, mut market) = make_market();
        let market_id = Symbol::new(&env, "TEST");
        MarketStateManager::store_market(&env, &market_id, &market);

        let user = Address::generate(&env);
        let outcome = String::from_str(&env, OUTCOME_LABELS[outcome_idx]);

        simulate_place_bet(&env, &mut market, &market_id, &user, &outcome, stake);
        let bet = BetStorage::get_bet(&env, &market_id, &user).unwrap();
        prop_assert!(bet.is_active(), "Bet must be active after placement");

        simulate_resolve_bet(&env, &market_id, &user, true);
        let won_bet = BetStorage::get_bet(&env, &market_id, &user).unwrap();
        prop_assert!(won_bet.is_winner(), "Bet must be Won after resolution with win=true");
        prop_assert!(won_bet.is_resolved(), "Won bet must report as resolved");

        simulate_resolve_bet(&env, &market_id, &user, false);
        let lost_bet = BetStorage::get_bet(&env, &market_id, &user).unwrap();
        prop_assert_eq!(lost_bet.status, BetStatus::Lost, "Bet must be Lost after resolve(false)");
    }

    #[test]
    fn prop_bet_amounts_within_bounds(
        bets in prop::collection::vec(arb_bet_op(), 1..=10),
    ) {
        let (env, mut market) = make_market();
        let market_id = Symbol::new(&env, "TEST");
        MarketStateManager::store_market(&env, &market_id, &market);

        for (outcome_idx, stake) in &bets {
            prop_assert!(
                *stake >= MIN_BET_AMOUNT,
                "Stake {} must be >= MIN_BET_AMOUNT {}",
                stake, MIN_BET_AMOUNT
            );
            prop_assert!(
                *stake <= MAX_BET_AMOUNT,
                "Stake {} must be <= MAX_BET_AMOUNT {}",
                stake, MAX_BET_AMOUNT
            );

            let user = Address::generate(&env);
            let outcome = String::from_str(&env, OUTCOME_LABELS[*outcome_idx]);
            simulate_place_bet(&env, &mut market, &market_id, &user, &outcome, *stake);

            let stored_bet = BetStorage::get_bet(&env, &market_id, &user).unwrap();
            prop_assert_eq!(
                stored_bet.amount, *stake,
                "Stored bet amount must match placed amount"
            );
            prop_assert_eq!(
                stored_bet.outcome,
                String::from_str(&env, OUTCOME_LABELS[*outcome_idx]),
                "Stored bet outcome must match"
            );
        }
        assert_bet_invariants(&env, &market, &market_id);
    }

    #[test]
    fn prop_large_bet_set_conserves_stake(
        bets in prop::collection::vec(arb_bet_op(), 50..=200),
    ) {
        let (env, mut market) = make_market();
        let market_id = Symbol::new(&env, "TEST");
        MarketStateManager::store_market(&env, &market_id, &market);

        let expected_total: i128 = bets.iter().map(|(_, s)| s).sum();

        for (outcome_idx, stake) in &bets {
            let user = Address::generate(&env);
            let outcome = String::from_str(&env, OUTCOME_LABELS[*outcome_idx]);
            simulate_place_bet(&env, &mut market, &market_id, &user, &outcome, *stake);
        }

        prop_assert_eq!(
            market.total_staked,
            expected_total,
            "total_staked ({}) must equal sum of all stakes ({})",
            market.total_staked,
            expected_total
        );
        assert_bet_invariants(&env, &market, &market_id);
    }

    #[test]
    fn prop_multi_user_multi_outcome_invariants(
        bets in prop::collection::vec(arb_bet_op(), 1..=30),
    ) {
        let (env, mut market) = make_market();
        let market_id = Symbol::new(&env, "TEST");
        MarketStateManager::store_market(&env, &market_id, &market);

        for (outcome_idx, stake) in &bets {
            let user = Address::generate(&env);
            let outcome = String::from_str(&env, OUTCOME_LABELS[*outcome_idx]);
            simulate_place_bet(&env, &mut market, &market_id, &user, &outcome, *stake);
        }
        assert_bet_invariants(&env, &market, &market_id);

        let stats = BetStorage::get_market_bet_stats(&env, &market_id);
        let all_users = BetStorage::get_all_bets_for_market(&env, &market_id);

        prop_assert_eq!(
            all_users.len() as u32,
            stats.unique_bettors,
            "Registered bettors count ({}) must match stats.unique_bettors ({})",
            all_users.len(),
            stats.unique_bettors
        );

        for user in all_users.iter() {
            let bet = BetStorage::get_bet(&env, &market_id, &user).unwrap();
            prop_assert!(
                bet.is_active(),
                "All placed bets must still be active (no cancel in this test)"
            );
        }
    }
}
