//! # Proptest invariant tests for resolution state transitions
//!
//! Issue: Add invariant proptest for resolution (v7)
//!
//! This module asserts that `ResolutionState` is a **deterministic, monotonic**
//! function of observable market fields.  No matter what combination of oracle
//! result, winning-outcomes, and dispute-stake values proptest generates, the
//! following invariants must always hold:
//!
//! 1. **Determinism** – `get_resolution_state` returns the same value on two
//!    successive calls against the same market (no hidden side effects).
//!
//! 2. **Monotonicity** – once a field that drives a higher-priority state is
//!    set, the returned state never *regresses* to a lower one regardless of
//!    what the lower-priority fields contain.
//!
//! 3. **Exclusivity** – exactly one of the five `ResolutionState` variants is
//!    returned for any market configuration; the function never panics.
//!
//! 4. **Finality** – `MarketResolved` is returned whenever `winning_outcomes`
//!    is `Some(…)`, regardless of oracle result or dispute-stake values.
//!
//! 5. **Oracle gate** – `OracleResolved` is returned only when there is an
//!    oracle result *and* no winning outcomes.
//!
//! 6. **Dispute gate** – `Disputed` is returned only when there are positive
//!    dispute stakes *and* no oracle result *and* no winning outcomes.
//!
//! 7. **Active baseline** – `Active` is returned when none of the above
//!    conditions hold.
//!
//! 8. **Resolution eligibility** – `can_resolve_market` returns `true` only
//!    for markets that have ended **and** have an oracle result **and** are not
//!    yet resolved; it returns `false` in all other configurations.
//!
//! 9. **Validate resolution parameters** – accepting a valid outcome from the
//!    market's outcome list on an unresolved market always returns `Ok(())`;
//!    passing an outcome that is not in the list, or calling on an already-
//!    resolved market, always returns an appropriate `Err`.
//!
//! ## Testing strategy
//!
//! All tests operate at the *unit level* directly on [`Market`] and
//! [`ResolutionUtils`].  This avoids the oracle / token / auth infrastructure
//! required by the full contract client, keeping tests fast and hermetic.
//!
//! The market builder `make_market` follows the exact same pattern used by
//! `voting_invariants.rs` to stay consistent with the rest of the test suite.

#![cfg(test)]

use crate::markets::MarketStateManager;
use crate::resolution::{ResolutionState, ResolutionUtils};
use crate::types::{Market, MarketState, OracleConfig};
use proptest::prelude::*;
use soroban_sdk::{testutils::Address as _, vec as svec, Address, Env, String};

// ── Constants ────────────────────────────────────────────────────────────────

/// Outcome labels used across all generated markets.
const OUTCOMES: &[&str] = &["yes", "no"];

/// Minimum dispute stake; mirrors `MIN_DISPUTE_STAKE` in `voting.rs`.
const MIN_DISPUTE_STAKE: i128 = 10_000_000;

// ── Strategy helpers ─────────────────────────────────────────────────────────

/// Arbitrary oracle result string (always one of the valid outcome labels).
fn arb_oracle_result() -> impl Strategy<Value = Option<usize>> {
    prop_oneof![
        Just(None),
        (0usize..OUTCOMES.len()).prop_map(Some),
    ]
}

/// Arbitrary set of winning outcome indices (None = unresolved).
fn arb_winning_outcomes() -> impl Strategy<Value = Option<Vec<usize>>> {
    prop_oneof![
        Just(None),
        // single winner
        (0usize..OUTCOMES.len()).prop_map(|i| Some(alloc::vec![i])),
        // two-way tie
        Just(Some(alloc::vec![0, 1])),
    ]
}

/// Arbitrary total dispute stake in [0, 1_000_000_000].
fn arb_dispute_stake() -> impl Strategy<Value = i128> {
    prop_oneof![
        Just(0i128),
        (MIN_DISPUTE_STAKE..=1_000_000_000i128),
    ]
}

// ── Market builder ───────────────────────────────────────────────────────────

/// Build a fresh market that has already ended (ledger time > end_time).
///
/// Sets `end_time` one second in the past relative to the default ledger
/// timestamp (0 + 1 = 1 → `end_time = 0`), then advances the ledger to `2`
/// so `has_ended` returns `true`.
fn make_ended_market() -> (Env, Market) {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(2); // current time = 2
    let admin = Address::generate(&env);
    let oracle = OracleConfig::none_sentinel(&env);
    let market = Market::new(
        &env,
        admin,
        String::from_str(&env, "Will BTC reach $100k by year end?"),
        svec![
            &env,
            String::from_str(&env, OUTCOMES[0]),
            String::from_str(&env, OUTCOMES[1]),
        ],
        1, // end_time = 1; ledger is at 2, so the market has ended
        oracle,
        None,
        86_400, // resolution_timeout
        MarketState::Ended,
    );
    (env, market)
}

/// Build a fresh market that has **not** ended (end_time is in the future).
fn make_active_market() -> (Env, Market) {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let oracle = OracleConfig::none_sentinel(&env);
    let market = Market::new(
        &env,
        admin,
        String::from_str(&env, "Will ETH reach $5k?"),
        svec![
            &env,
            String::from_str(&env, OUTCOMES[0]),
            String::from_str(&env, OUTCOMES[1]),
        ],
        env.ledger().timestamp() + 86_400, // ends in 24 h
        oracle,
        None,
        86_400,
        MarketState::Active,
    );
    (env, market)
}

// ── Invariant helpers ────────────────────────────────────────────────────────

/// Apply `oracle_idx`, `winning_idxs`, and `dispute_stake` to a market,
/// then return the resulting `ResolutionState`.
fn apply_and_get_state(
    env: &Env,
    market: &mut Market,
    oracle_idx: Option<usize>,
    winning_idxs: Option<alloc::vec::Vec<usize>>,
    dispute_stake: i128,
) -> ResolutionState {
    // Set oracle result
    if let Some(idx) = oracle_idx {
        let outcome = String::from_str(env, OUTCOMES[idx]);
        MarketStateManager::set_oracle_result(market, outcome);
    }

    // Set winning outcomes
    if let Some(idxs) = winning_idxs {
        let mut winning = soroban_sdk::Vec::new(env);
        for i in idxs {
            winning.push_back(String::from_str(env, OUTCOMES[i]));
        }
        MarketStateManager::set_winning_outcomes(market, winning, None);
    }

    // Add dispute stake from a fresh user
    if dispute_stake > 0 {
        let disputer = Address::generate(env);
        MarketStateManager::add_dispute_stake(market, disputer, dispute_stake);
    }

    ResolutionUtils::get_resolution_state(env, market)
}

// ── Proptest suites ──────────────────────────────────────────────────────────

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 1_000,
        source_file: Some("src/resolution_invariants.rs"),
        ..ProptestConfig::default()
    })]

    // ── Invariant 1: Determinism ─────────────────────────────────────────────

    /// `get_resolution_state` is a pure function of the market fields:
    /// calling it twice returns the same value.
    #[test]
    fn prop_resolution_state_is_deterministic(
        oracle_idx   in arb_oracle_result(),
        winning_idxs in arb_winning_outcomes(),
        dispute_stake in arb_dispute_stake(),
    ) {
        let (env, mut market) = make_ended_market();
        let state1 = apply_and_get_state(&env, &mut market, oracle_idx, winning_idxs.clone(), dispute_stake);
        // Call again without mutating — must return the same value.
        let state2 = ResolutionUtils::get_resolution_state(&env, &market);
        prop_assert_eq!(
            state1, state2,
            "get_resolution_state returned different values on consecutive calls: {:?} vs {:?}",
            state1, state2
        );
    }

    // ── Invariant 2 & 4: Finality / MarketResolved dominates ─────────────────

    /// When `winning_outcomes` is `Some(…)` the state is always `MarketResolved`
    /// regardless of oracle result or dispute-stake values.
    #[test]
    fn prop_winning_outcomes_implies_market_resolved(
        oracle_idx   in arb_oracle_result(),
        winning_idxs in (0usize..OUTCOMES.len()).prop_map(|i| Some(alloc::vec![i])),
        dispute_stake in arb_dispute_stake(),
    ) {
        let (env, mut market) = make_ended_market();
        let state = apply_and_get_state(&env, &mut market, oracle_idx, winning_idxs, dispute_stake);
        prop_assert_eq!(
            state,
            ResolutionState::MarketResolved,
            "expected MarketResolved when winning_outcomes is Some, got {:?}",
            state
        );
    }

    // ── Invariant 5: Oracle gate ──────────────────────────────────────────────

    /// When oracle result is set but market is NOT yet resolved (no winning
    /// outcomes), the state is `OracleResolved` regardless of dispute stakes.
    #[test]
    fn prop_oracle_result_without_winning_outcomes_is_oracle_resolved(
        oracle_idx    in (0usize..OUTCOMES.len()),   // always Some
        dispute_stake in arb_dispute_stake(),
    ) {
        let (env, mut market) = make_ended_market();
        let state = apply_and_get_state(
            &env,
            &mut market,
            Some(oracle_idx),
            None,             // no winning outcomes
            dispute_stake,
        );
        prop_assert_eq!(
            state,
            ResolutionState::OracleResolved,
            "expected OracleResolved when oracle_result is Some and winning_outcomes is None, got {:?}",
            state
        );
    }

    // ── Invariant 6: Dispute gate ─────────────────────────────────────────────

    /// When there are positive dispute stakes but no oracle result and no winning
    /// outcomes the state is `Disputed`.
    #[test]
    fn prop_dispute_stake_without_oracle_or_resolution_is_disputed(
        dispute_stake in (MIN_DISPUTE_STAKE..=1_000_000_000i128),
    ) {
        let (env, mut market) = make_ended_market();
        let state = apply_and_get_state(
            &env,
            &mut market,
            None,  // no oracle result
            None,  // no winning outcomes
            dispute_stake,
        );
        prop_assert_eq!(
            state,
            ResolutionState::Disputed,
            "expected Disputed when dispute_stake > 0 and no oracle/resolution, got {:?}",
            state
        );
    }

    // ── Invariant 7: Active baseline ──────────────────────────────────────────

    /// When none of the above conditions hold (no oracle result, no winning
    /// outcomes, zero dispute stakes) the state is `Active`.
    #[test]
    fn prop_empty_market_is_active() {
        let (env, market) = make_ended_market();
        let state = ResolutionUtils::get_resolution_state(&env, &market);
        prop_assert_eq!(
            state,
            ResolutionState::Active,
            "expected Active for a brand-new market, got {:?}",
            state
        );
    }

    // ── Invariant 3: Exclusivity / no panic ───────────────────────────────────

    /// For any combination of inputs, exactly one state variant is returned and
    /// no panic occurs.
    #[test]
    fn prop_resolution_state_never_panics(
        oracle_idx    in arb_oracle_result(),
        winning_idxs  in arb_winning_outcomes(),
        dispute_stake in arb_dispute_stake(),
    ) {
        let (env, mut market) = make_ended_market();
        // The call itself must not panic; the returned value is checked for validity.
        let state = apply_and_get_state(&env, &mut market, oracle_idx, winning_idxs, dispute_stake);
        let valid = matches!(
            state,
            ResolutionState::Active
                | ResolutionState::OracleResolved
                | ResolutionState::MarketResolved
                | ResolutionState::Disputed
                | ResolutionState::Finalized
        );
        prop_assert!(valid, "get_resolution_state returned an unexpected variant: {:?}", state);
    }

    // ── Invariant 8: Resolution eligibility ──────────────────────────────────

    /// `can_resolve_market` must return `true` only for markets that have ended,
    /// have an oracle result, and are not yet resolved.
    #[test]
    fn prop_can_resolve_requires_ended_oracle_unresolved(
        oracle_idx   in arb_oracle_result(),
        winning_idxs in arb_winning_outcomes(),
    ) {
        let (env, mut market) = make_ended_market();

        // Apply oracle result and winning outcomes (dispute stake is irrelevant here).
        apply_and_get_state(&env, &mut market, oracle_idx, winning_idxs.clone(), 0);

        let has_oracle = oracle_idx.is_some();
        let is_resolved = winning_idxs.is_some();
        let has_ended = market.has_ended(&env);

        let can_resolve = ResolutionUtils::can_resolve_market(&env, &market);
        let expected = has_ended && has_oracle && !is_resolved;

        prop_assert_eq!(
            can_resolve,
            expected,
            "can_resolve_market mismatch: ended={} oracle={} resolved={} → expected {} got {}",
            has_ended, has_oracle, is_resolved, expected, can_resolve
        );
    }

    /// `can_resolve_market` is always `false` for a market that has not ended,
    /// regardless of oracle result or resolution status.
    #[test]
    fn prop_active_market_cannot_be_resolved(
        oracle_idx in arb_oracle_result(),
    ) {
        let (env, mut market) = make_active_market();

        if let Some(idx) = oracle_idx {
            let outcome = String::from_str(&env, OUTCOMES[idx]);
            MarketStateManager::set_oracle_result(&mut market, outcome);
        }

        prop_assert!(
            !ResolutionUtils::can_resolve_market(&env, &market),
            "can_resolve_market must be false for a market that has not ended"
        );
    }

    // ── Invariant 9: validate_resolution_parameters ───────────────────────────

    /// Passing a valid outcome from the market's outcome list on an unresolved
    /// market always returns `Ok(())`.
    #[test]
    fn prop_valid_outcome_on_unresolved_market_accepted(
        outcome_idx in 0usize..OUTCOMES.len(),
    ) {
        let (env, market) = make_ended_market();
        let outcome = String::from_str(&env, OUTCOMES[outcome_idx]);
        let result = ResolutionUtils::validate_resolution_parameters(&env, &market, &outcome);
        prop_assert!(
            result.is_ok(),
            "validate_resolution_parameters rejected a valid outcome '{}': {:?}",
            OUTCOMES[outcome_idx],
            result
        );
    }

    /// Passing an outcome string that is NOT in the market's outcome list always
    /// returns an error.
    #[test]
    fn prop_invalid_outcome_rejected(
        bad_label in "[a-z]{3,8}",   // random lowercase word unlikely to match "yes"/"no"
    ) {
        // Filter out any strings that accidentally match a real outcome.
        let is_valid = OUTCOMES.iter().any(|&o| o == bad_label.as_str());
        prop_assume!(!is_valid);

        let (env, market) = make_ended_market();
        let outcome = String::from_str(&env, bad_label.as_str());
        let result = ResolutionUtils::validate_resolution_parameters(&env, &market, &outcome);
        prop_assert!(
            result.is_err(),
            "validate_resolution_parameters must reject unknown outcome '{}'",
            bad_label
        );
    }

    /// Calling `validate_resolution_parameters` on an already-resolved market
    /// always returns an error regardless of the outcome value.
    #[test]
    fn prop_resolved_market_rejects_further_resolution(
        outcome_idx in 0usize..OUTCOMES.len(),
    ) {
        let (env, mut market) = make_ended_market();
        // Resolve the market first.
        let winning = svec![&env, String::from_str(&env, OUTCOMES[0])];
        MarketStateManager::set_winning_outcomes(&mut market, winning, None);

        let outcome = String::from_str(&env, OUTCOMES[outcome_idx]);
        let result = ResolutionUtils::validate_resolution_parameters(&env, &market, &outcome);
        prop_assert!(
            result.is_err(),
            "validate_resolution_parameters must reject resolution of an already-resolved market"
        );
    }

    // ── Monotonicity: winning_outcomes cannot regress state ───────────────────

    /// Once `winning_outcomes` is set (state = `MarketResolved`), setting an
    /// oracle result afterwards must not change the state — it stays
    /// `MarketResolved`.
    #[test]
    fn prop_market_resolved_is_monotone_after_oracle(
        oracle_idx in 0usize..OUTCOMES.len(),
    ) {
        let (env, mut market) = make_ended_market();

        // First set winning outcomes → state must be MarketResolved.
        let winning = svec![&env, String::from_str(&env, OUTCOMES[0])];
        MarketStateManager::set_winning_outcomes(&mut market, winning, None);
        let state_before = ResolutionUtils::get_resolution_state(&env, &market);
        prop_assert_eq!(state_before, ResolutionState::MarketResolved);

        // Now additionally set an oracle result.
        let oracle_outcome = String::from_str(&env, OUTCOMES[oracle_idx]);
        MarketStateManager::set_oracle_result(&mut market, oracle_outcome);
        let state_after = ResolutionUtils::get_resolution_state(&env, &market);

        prop_assert_eq!(
            state_after,
            ResolutionState::MarketResolved,
            "state must remain MarketResolved after oracle result is set on a resolved market"
        );
    }

    /// Once `winning_outcomes` is set (state = `MarketResolved`), adding a
    /// dispute stake must not change the state — it stays `MarketResolved`.
    #[test]
    fn prop_market_resolved_is_monotone_after_dispute(
        dispute_stake in (MIN_DISPUTE_STAKE..=1_000_000_000i128),
    ) {
        let (env, mut market) = make_ended_market();

        let winning = svec![&env, String::from_str(&env, OUTCOMES[0])];
        MarketStateManager::set_winning_outcomes(&mut market, winning, None);

        let disputer = Address::generate(&env);
        MarketStateManager::add_dispute_stake(&mut market, disputer, dispute_stake);

        let state = ResolutionUtils::get_resolution_state(&env, &market);
        prop_assert_eq!(
            state,
            ResolutionState::MarketResolved,
            "state must remain MarketResolved after a dispute stake is added"
        );
    }

    /// `OracleResolved` cannot regress to `Disputed` when a dispute stake is
    /// added after the oracle result is already set (and no winning outcomes).
    #[test]
    fn prop_oracle_resolved_dominates_disputed(
        oracle_idx    in 0usize..OUTCOMES.len(),
        dispute_stake in (MIN_DISPUTE_STAKE..=1_000_000_000i128),
    ) {
        let (env, mut market) = make_ended_market();

        // Set oracle result first.
        let oracle_outcome = String::from_str(&env, OUTCOMES[oracle_idx]);
        MarketStateManager::set_oracle_result(&mut market, oracle_outcome);

        // Then add a dispute stake — state must still be OracleResolved.
        let disputer = Address::generate(&env);
        MarketStateManager::add_dispute_stake(&mut market, disputer, dispute_stake);

        let state = ResolutionUtils::get_resolution_state(&env, &market);
        prop_assert_eq!(
            state,
            ResolutionState::OracleResolved,
            "OracleResolved must dominate Disputed when oracle_result is Some"
        );
    }
}
