//! # Integration property tests: VotingManager stake conservation
//!
//! Issue #838 — these property tests assert that for any sequence of valid votes cast
//! through the public `PredictifyHybridClient::vote` entrypoint the following invariant
//! always holds:
//!
//! ```text
//! sum(market.stakes.values()) == market.total_staked
//! ```
//!
//! and that the aggregate stake across **all markets** equals the sum of individual
//! per-market totals (cross-market isolation).
//!
//! ## Why integration-level?
//!
//! The unit-level invariants for `MarketStateManager::add_vote` already live in
//! `src/voting_invariants.rs`. This file exercises the **full contract stack** —
//! auth, rate-limiter, state-machine guard, token transfer — via the same public
//! entrypoints a real caller would use. Only the on-chain `total_staked` field and
//! the `stakes` map (readable from `get_market`) are used as observables; no internal
//! module symbols are imported.
//!
//! ## Invariants covered
//!
//! 1. **Single-market conservation**: after every vote,
//!    `sum(stakes) == market.total_staked`.
//! 2. **Multi-market isolation**: votes on market A do not change `total_staked`
//!    on market B, for arbitrary stake sizes and any number of voters.
//! 3. **Duplicate-vote prevention**: a second call to `vote` by the same user on the
//!    same market is rejected (`Error::AlreadyVoted`), so `total_staked` is never
//!    double-counted through this path.
//! 4. **Cross-market aggregate**: the sum of `total_staked` across N independent
//!    markets equals the sum of all stakes submitted to each of those markets.
//! 5. **Minimum-stake boundary**: votes at exactly `MIN_VOTE_STAKE` stroops are
//!    accepted and conserved; votes one stroop below are rejected without
//!    mutating `total_staked`.

#![cfg(test)]

use predictify_hybrid::{Error, PredictifyHybrid, PredictifyHybridClient};
use proptest::prelude::*;
use soroban_sdk::{
    testutils::Address as _,
    token::StellarAssetClient,
    Address, Env, String as SString, Symbol,
};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Minimum valid vote stake enforced by the contract (mirrors `MIN_VOTE_STAKE`
/// in `config.rs` / `voting.rs`).
const MIN_VOTE_STAKE: i128 = 1_000_000;

/// Upper bound for generated stake amounts (100 XLM in stroops).
const MAX_VOTE_STAKE: i128 = 1_000_000_000;

/// Oracle address sentinel used by existing tests; any well-formed
/// G-address is fine here — the Reflector oracle is not actually called.
const ORACLE_ADDRESS: &str = "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";

/// Initial token balance minted to each test user (10 000 XLM in stroops).
const USER_INITIAL_BALANCE: i128 = 10_000_000_000;

// ---------------------------------------------------------------------------
// Test fixture
// ---------------------------------------------------------------------------

/// Shared test environment: a registered + initialized contract with a live
/// token and one funded admin address.
struct Fixture {
    env: Env,
    contract_id: Address,
    token_id: Address,
    admin: Address,
}

impl Fixture {
    /// Create a fresh `Fixture`.
    ///
    /// Sets up a Stellar asset contract, stores `TokenID` inside the
    /// prediction-market contract's persistent storage (matching production
    /// bootstrap), then initialises the market contract.
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let token_admin = Address::generate(&env);
        let token_contract = env.register_stellar_asset_contract_v2(token_admin);
        let token_id = token_contract.address();

        let admin = Address::generate(&env);
        let contract_id = env.register(PredictifyHybrid, ());

        // Wire the token into the contract's persistent store so that
        // `MarketUtils::get_token_client` resolves correctly.
        env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .set(&Symbol::new(&env, "TokenID"), &token_id);
        });

        PredictifyHybridClient::new(&env, &contract_id).initialize(&admin, &None, &None);

        Self { env, contract_id, token_id, admin }
    }

    /// Return a `PredictifyHybridClient` bound to this fixture.
    fn client(&self) -> PredictifyHybridClient<'_> {
        PredictifyHybridClient::new(&self.env, &self.contract_id)
    }

    /// Mint `USER_INITIAL_BALANCE` stroops for a fresh random address and
    /// return that address.
    fn funded_user(&self) -> Address {
        let user = Address::generate(&self.env);
        StellarAssetClient::new(&self.env, &self.token_id)
            .mint(&user, &USER_INITIAL_BALANCE);
        user
    }

    /// Create an Active market with outcomes `["yes", "no"]` and a 30-day
    /// voting period. Returns the market `Symbol` id.
    fn new_market(&self, question: &str, feed_id: &str) -> Symbol {
        use predictify_hybrid::{OracleConfig, OracleProvider};
        self.client().create_market(
            &self.admin,
            &SString::from_str(&self.env, question),
            &soroban_sdk::vec![
                &self.env,
                SString::from_str(&self.env, "yes"),
                SString::from_str(&self.env, "no"),
            ],
            &30_u32,
            &OracleConfig::new(
                OracleProvider::reflector(),
                Address::from_str(&self.env, ORACLE_ADDRESS),
                SString::from_str(&self.env, feed_id),
                100_i128,
                SString::from_str(&self.env, "gt"),
            ),
            &None,
            &86_400_u64,
        )
    }

    /// Read `market.total_staked` via `get_market`.
    fn total_staked(&self, market_id: &Symbol) -> i128 {
        self.client()
            .get_market(market_id)
            .expect("market must exist")
            .total_staked
    }

    /// Compute `sum(market.stakes.values())` — the ground-truth stake sum
    /// derived independently from `total_staked`.
    fn stakes_sum(&self, market_id: &Symbol) -> i128 {
        self.client()
            .get_market(market_id)
            .expect("market must exist")
            .stakes
            .values()
            .iter()
            .sum()
    }
}

// ---------------------------------------------------------------------------
// Strategy helpers
// ---------------------------------------------------------------------------

/// Valid stake amounts in `[MIN_VOTE_STAKE, MAX_VOTE_STAKE]`.
fn arb_valid_stake() -> impl Strategy<Value = i128> {
    MIN_VOTE_STAKE..=MAX_VOTE_STAKE
}

/// Outcome index into `["yes", "no"]`.
fn arb_outcome_idx() -> impl Strategy<Value = usize> {
    0_usize..2
}

// ---------------------------------------------------------------------------
// Proptest suites
// ---------------------------------------------------------------------------

proptest! {
    // Keep the case count modest — each case spins up a full `Env`
    // and registers a contract (mirrors `proptest_fee.rs` / `stateful.rs`).
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// **Property 1 — single-market conservation**: after each individual
    /// vote, `sum(stakes map) == market.total_staked`.
    ///
    /// Votes are submitted through the full public entrypoint so that the
    /// auth guard, rate-limiter, state check, and token transfer are all
    /// exercised.
    #[test]
    fn prop_single_market_stake_conservation(
        votes in prop::collection::vec((arb_outcome_idx(), arb_valid_stake()), 1..=10),
    ) {
        let fx = Fixture::new();
        let market_id = fx.new_market("Will BTC hit 100k?", "BTC");
        let client = fx.client();
        let outcomes = ["yes", "no"];

        for (outcome_idx, stake) in &votes {
            let user = fx.funded_user();
            let outcome = SString::from_str(&fx.env, outcomes[*outcome_idx]);
            // Submit vote through the public entrypoint.
            client.vote(&user, &market_id, &outcome, stake);

            // Invariant: independently derived sum == total_staked field.
            let stakes_sum = fx.stakes_sum(&market_id);
            let total_staked = fx.total_staked(&market_id);
            prop_assert_eq!(
                stakes_sum,
                total_staked,
                "After vote with stake {}: stakes_sum ({}) != total_staked ({})",
                stake,
                stakes_sum,
                total_staked,
            );
        }

        // Cumulative check: total_staked equals the sum of all submitted stakes.
        let expected: i128 = votes.iter().map(|(_, s)| s).sum();
        prop_assert_eq!(
            fx.total_staked(&market_id),
            expected,
            "Cumulative total_staked mismatch: got {}, expected {}",
            fx.total_staked(&market_id),
            expected,
        );
    }

    /// **Property 2 — multi-outcome distribution**: total_staked stays consistent
    /// regardless of which outcome each user votes for.
    ///
    /// Votes are spread across both outcomes so that both buckets of the
    /// outcome distribution are exercised in every test run.
    #[test]
    fn prop_multi_outcome_stake_conservation(
        stakes_yes in prop::collection::vec(arb_valid_stake(), 1..=6),
        stakes_no  in prop::collection::vec(arb_valid_stake(), 1..=6),
    ) {
        let fx = Fixture::new();
        let market_id = fx.new_market("Will ETH hit 5k?", "ETH");
        let client = fx.client();

        let yes_str = SString::from_str(&fx.env, "yes");
        let no_str  = SString::from_str(&fx.env, "no");

        for stake in &stakes_yes {
            let user = fx.funded_user();
            client.vote(&user, &market_id, &yes_str, stake);
        }
        for stake in &stakes_no {
            let user = fx.funded_user();
            client.vote(&user, &market_id, &no_str, stake);
        }

        let expected: i128 = stakes_yes.iter().chain(stakes_no.iter()).sum();
        let total_staked = fx.total_staked(&market_id);
        prop_assert_eq!(
            total_staked,
            expected,
            "Multi-outcome total_staked mismatch: got {}, expected {}",
            total_staked,
            expected,
        );
        prop_assert_eq!(
            fx.stakes_sum(&market_id),
            total_staked,
            "stakes_sum ({}) != total_staked ({}) after multi-outcome votes",
            fx.stakes_sum(&market_id),
            total_staked,
        );
    }

    /// **Property 3 — market isolation**: votes on one market do not mutate
    /// `total_staked` on any other independent market.
    ///
    /// Creates two markets and verifies that staking into one does not bleed
    /// into the other for arbitrary stake amounts.
    #[test]
    fn prop_cross_market_isolation(
        stake_a in arb_valid_stake(),
        stake_b in arb_valid_stake(),
    ) {
        let fx = Fixture::new();
        let market_a = fx.new_market("Will BTC hit 100k?", "BTC");
        let market_b = fx.new_market("Will ETH hit 5k?", "ETH");
        let client = fx.client();

        // Both markets start at zero.
        prop_assert_eq!(fx.total_staked(&market_a), 0);
        prop_assert_eq!(fx.total_staked(&market_b), 0);

        // Vote on A — B must remain unchanged.
        let user_a = fx.funded_user();
        client.vote(&user_a, &market_a, &SString::from_str(&fx.env, "yes"), &stake_a);
        prop_assert_eq!(
            fx.total_staked(&market_a),
            stake_a,
            "market_a.total_staked should be {} after first vote, got {}",
            stake_a,
            fx.total_staked(&market_a),
        );
        prop_assert_eq!(
            fx.total_staked(&market_b),
            0,
            "market_b.total_staked should still be 0 after a vote on market_a, got {}",
            fx.total_staked(&market_b),
        );

        // Vote on B — A must remain unchanged.
        let user_b = fx.funded_user();
        client.vote(&user_b, &market_b, &SString::from_str(&fx.env, "no"), &stake_b);
        prop_assert_eq!(
            fx.total_staked(&market_b),
            stake_b,
            "market_b.total_staked should be {} after vote, got {}",
            stake_b,
            fx.total_staked(&market_b),
        );
        prop_assert_eq!(
            fx.total_staked(&market_a),
            stake_a,
            "market_a.total_staked should remain {} after a vote on market_b, got {}",
            stake_a,
            fx.total_staked(&market_a),
        );

        // Aggregate check.
        let aggregate = fx.total_staked(&market_a) + fx.total_staked(&market_b);
        prop_assert_eq!(
            aggregate,
            stake_a + stake_b,
            "Aggregate total_staked ({}) != stake_a + stake_b ({})",
            aggregate,
            stake_a + stake_b,
        );
    }

    /// **Property 4 — duplicate-vote rejection**: a second `vote` call from the
    /// same user on the same market must be rejected, and `total_staked` must
    /// not increase as a result.
    ///
    /// This confirms the `AlreadyVoted` guard is exercised at the integration
    /// level, preventing double-counting of stakes.
    #[test]
    fn prop_duplicate_vote_rejected_stake_unchanged(
        first_stake  in arb_valid_stake(),
        second_stake in arb_valid_stake(),
        outcome_idx  in arb_outcome_idx(),
    ) {
        let fx = Fixture::new();
        let market_id = fx.new_market("Will XLM hit $1?", "XLM");
        let client = fx.client();
        let outcomes = ["yes", "no"];
        let outcome = SString::from_str(&fx.env, outcomes[outcome_idx]);

        let user = fx.funded_user();

        // First vote must succeed.
        client.vote(&user, &market_id, &outcome, &first_stake);
        prop_assert_eq!(
            fx.total_staked(&market_id),
            first_stake,
            "total_staked should be {} after first vote, got {}",
            first_stake,
            fx.total_staked(&market_id),
        );

        // Second vote by the same user must be rejected.
        let second_result = client.try_vote(&user, &market_id, &outcome, &second_stake);
        prop_assert!(
            second_result.is_err(),
            "Duplicate vote should be rejected but was accepted"
        );

        // total_staked must not have changed.
        prop_assert_eq!(
            fx.total_staked(&market_id),
            first_stake,
            "total_staked ({}) changed after a rejected duplicate vote; expected {}",
            fx.total_staked(&market_id),
            first_stake,
        );

        // stakes_sum == total_staked invariant still holds.
        prop_assert_eq!(
            fx.stakes_sum(&market_id),
            fx.total_staked(&market_id),
            "stakes_sum ({}) != total_staked ({}) after duplicate rejection",
            fx.stakes_sum(&market_id),
            fx.total_staked(&market_id),
        );
    }
}

// ---------------------------------------------------------------------------
// Focused edge-case unit tests
// ---------------------------------------------------------------------------
// proptest's RNG rarely lands on the exact boundary values; pin them here.

/// One vote at exactly the minimum valid stake must be accepted and
/// `total_staked` must reflect it exactly.
#[test]
fn edge_min_stake_is_accepted_and_conserved() {
    let fx = Fixture::new();
    let market_id = fx.new_market("Edge: min stake", "BTC");
    let user = fx.funded_user();

    fx.client().vote(
        &user,
        &market_id,
        &SString::from_str(&fx.env, "yes"),
        &MIN_VOTE_STAKE,
    );

    assert_eq!(fx.total_staked(&market_id), MIN_VOTE_STAKE);
    assert_eq!(fx.stakes_sum(&market_id), MIN_VOTE_STAKE);
}

/// A vote one stroop below the minimum must be rejected and `total_staked`
/// must remain zero.
#[test]
fn edge_below_min_stake_rejected_total_unchanged() {
    let fx = Fixture::new();
    let market_id = fx.new_market("Edge: below-min stake", "ETH");
    let user = fx.funded_user();
    let below_min = MIN_VOTE_STAKE - 1;

    let result = fx.client().try_vote(
        &user,
        &market_id,
        &SString::from_str(&fx.env, "no"),
        &below_min,
    );

    assert!(result.is_err(), "stake below minimum should be rejected");
    assert_eq!(
        fx.total_staked(&market_id),
        0,
        "total_staked must remain 0 after a rejected below-min vote"
    );
}

/// Votes from N distinct users must yield `total_staked == sum of their stakes`.
#[test]
fn edge_multiple_distinct_users_stake_conserved() {
    let fx = Fixture::new();
    let market_id = fx.new_market("Edge: multi-user", "XLM");
    let client = fx.client();
    let stakes: &[i128] = &[1_000_000, 2_000_000, 3_000_000, 5_000_000, 10_000_000];

    for &stake in stakes {
        let user = fx.funded_user();
        client.vote(&user, &market_id, &SString::from_str(&fx.env, "yes"), &stake);
    }

    let expected: i128 = stakes.iter().sum();
    assert_eq!(fx.total_staked(&market_id), expected);
    assert_eq!(fx.stakes_sum(&market_id), expected);
}

/// `total_staked` starts at zero and increases by each successive vote amount.
#[test]
fn edge_total_staked_monotonically_increases() {
    let fx = Fixture::new();
    let market_id = fx.new_market("Edge: monotonic", "BTC");
    let client = fx.client();
    let stake_sequence: &[i128] = &[2_000_000, 5_000_000, 1_000_000, 8_000_000];

    let mut running_total: i128 = 0;
    for &stake in stake_sequence {
        let user = fx.funded_user();
        client.vote(&user, &market_id, &SString::from_str(&fx.env, "no"), &stake);
        running_total += stake;
        assert_eq!(
            fx.total_staked(&market_id),
            running_total,
            "total_staked should be {} after adding stake {}, got {}",
            running_total,
            stake,
            fx.total_staked(&market_id),
        );
    }
}

/// Two markets share no state: arbitrary votes on one must not affect the other.
#[test]
fn edge_two_markets_fully_isolated() {
    let fx = Fixture::new();
    let mkt_a = fx.new_market("Edge: isolation A", "BTC");
    let mkt_b = fx.new_market("Edge: isolation B", "ETH");
    let client = fx.client();

    // Three users vote on A only.
    for stake in [1_000_000_i128, 2_000_000, 4_000_000] {
        let u = fx.funded_user();
        client.vote(&u, &mkt_a, &SString::from_str(&fx.env, "yes"), &stake);
    }

    assert_eq!(fx.total_staked(&mkt_a), 7_000_000, "mkt_a total_staked");
    assert_eq!(fx.total_staked(&mkt_b), 0, "mkt_b must stay at 0");

    // One user votes on B.
    let u = fx.funded_user();
    client.vote(&u, &mkt_b, &SString::from_str(&fx.env, "no"), &3_000_000);

    assert_eq!(fx.total_staked(&mkt_a), 7_000_000, "mkt_a unchanged after mkt_b vote");
    assert_eq!(fx.total_staked(&mkt_b), 3_000_000, "mkt_b total_staked after one vote");
    assert_eq!(
        fx.total_staked(&mkt_a) + fx.total_staked(&mkt_b),
        10_000_000,
        "aggregate stake must equal 10_000_000"
    );
}

/// A single large stake (near i128 safe range for the token) must be accepted
/// and conserved without overflow.
#[test]
fn edge_large_stake_no_overflow() {
    let fx = Fixture::new();
    let market_id = fx.new_market("Edge: large stake", "BTC");
    // Use a large but bounded value; the token balance is capped at USER_INITIAL_BALANCE.
    let large_stake = USER_INITIAL_BALANCE; // 10_000 XLM in stroops

    let user = fx.funded_user();
    fx.client().vote(
        &user,
        &market_id,
        &SString::from_str(&fx.env, "yes"),
        &large_stake,
    );

    assert_eq!(fx.total_staked(&market_id), large_stake);
    assert_eq!(fx.stakes_sum(&market_id), large_stake);
}
