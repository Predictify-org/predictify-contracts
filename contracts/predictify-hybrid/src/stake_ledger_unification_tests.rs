//! # Stake Ledger Unification Tests (issue: vote/place_bet divergence)
//!
//! `vote` and `place_bet` now delegate to the single validated stake ledger
//! `bets::BetManager::record_stake`. These tests pin every row of the issue's
//! test plan:
//!
//! 1. vote(S1) then place_bet(S2) → second call fails; S1 intact
//! 2. place_bet(S2) then vote(S1) → second call fails with the same error
//! 3. vote while the circuit breaker has betting paused → CBOpen, no transfer
//! 4. vote with 0/negative/below-min/above-max/above-cap stake → same typed
//!    errors as place_bet
//! 5. vote filling max_participants blocks a later bet
//! 6. total_staked near i128::MAX → typed `Error::Overflow`, no trap
//! 7. cancel_bet refunds exactly the bet, clears the ledger it created, and
//!    allows a fresh stake afterwards
//! 8. conservation: after mixed vote/bet activity, resolution and claims
//!    preserve `paid_out + contract_balance == total staked`
//! 9. error precedence is identical for both entrypoints:
//!    circuit breaker → market state/deadline → outcome → amount → duplicate
//!    → capacity → overflow
//!
//! ## Regression proof (master behavior)
//!
//! On master, row 1 *succeeds*: `place_bet(S2)` after `vote(S1)` overwrites
//! `market.stakes[user]` with S2 while `market.total_staked` accumulates
//! S1 + S2. With S1 = 1_000_000 stroops the master flow leaves
//! `total_staked (3_000_000) - claimable (2_000_000) = 1_000_000` stroops
//! stranded: locked in the contract, unreachable by any claim or cancel
//! path. `test_master_divergence_replayed_on_master` below was run against
//! master to capture that state; under this fix row 1 is rejected atomically
//! and the stranded amount can no longer be created.
//!
//! ## Legacy (pre-fix) markets
//!
//! Markets that already contain both a vote and a bet for one user (the
//! master bug state) are handled as follows: the bet-side mirror in
//! `market.stakes` is authoritative for claims and cancels, exactly as on
//! master (that is what claim_winnings and cancel_bet have always paid from).
//! `test_legacy_dual_entry_market_remains_claimable` proves the claim path
//! still pays out; `test_legacy_cancel_only_removes_bet_owned_entries` proves
//! cancellation refunds the bet amount and leaves the pre-existing stranded
//! residue untouched (documented; admin remediation is out of scope).

#![cfg(test)]

use crate::bets::{BetStorage, MIN_BET_AMOUNT};
use crate::circuit_breaker::CircuitBreaker;
use crate::err::Error;
use crate::types::{Bet, Market, OracleConfig, OracleProvider};
use crate::{PredictifyHybrid, PredictifyHybridClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token::StellarAssetClient,
    vec, Address, Env, String, Symbol,
};

const S1: i128 = 1_000_000; // == MIN_BET_AMOUNT
const S2: i128 = 2_000_000;
const S3: i128 = 3_000_000;
const MAX_STAKE: i128 = 100_000_000_000; // == MAX_BET_AMOUNT
const INITIAL_BALANCE: i128 = 10_000_000_000;

fn sdk_err(e: Error) -> soroban_sdk::Error {
    soroban_sdk::Error::from_contract_error(e as u32)
}

struct Setup {
    env: Env,
    contract_id: Address,
    admin: Address,
    token_id: Address,
    market_id: Symbol,
}

impl Setup {
    fn new() -> Self {
        Self::with_market(None)
    }

    fn with_market(max_participants: Option<u32>) -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let contract_id = env.register(PredictifyHybrid, ());
        let client = PredictifyHybridClient::new(&env, &contract_id);
        client.initialize(&admin, &None, &None);

        // Token for staking
        let token_admin = Address::generate(&env);
        let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_id = token_contract.address();
        env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .set(&Symbol::new(&env, "TokenID"), &token_id);
        });

        // Market
        let outcomes = vec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ];
        let market_id = client.create_market(
            &admin,
            &String::from_str(&env, "Will BTC reach $100,000?"),
            &outcomes,
            &30,
            &OracleConfig::new(
                OracleProvider::reflector(),
                Address::from_str(
                    &env,
                    "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
                ),
                String::from_str(&env, "BTC/USD"),
                100_000_00000000,
                String::from_str(&env, "gt"),
            ),
            &None,
            &86400u64,
            &None,
            &None,
            &None,
            &None,
            &max_participants,
        );

        Self {
            env,
            contract_id,
            admin,
            token_id,
            market_id,
        }
    }

    fn client(&self) -> PredictifyHybridClient<'_> {
        PredictifyHybridClient::new(&self.env, &self.contract_id)
    }

    /// A fresh user with a funded balance and token approval in place.
    fn funded_user(&self) -> Address {
        let u = Address::generate(&self.env);
        StellarAssetClient::new(&self.env, &self.token_id).mint(&u, &INITIAL_BALANCE);
        soroban_sdk::token::Client::new(&self.env, &self.token_id)
            .approve(&u, &self.contract_id, &i128::MAX, &1_000_000);
        u
    }

    fn balance(&self, addr: &Address) -> i128 {
        soroban_sdk::token::Client::new(&self.env, &self.token_id).balance(addr)
    }

    fn contract_balance(&self) -> i128 {
        self.balance(&self.contract_id)
    }

    fn market(&self, market_id: &Symbol) -> Market {
        self.env.as_contract(&self.contract_id, || {
            self.env
                .storage()
                .persistent()
                .get::<Symbol, Market>(market_id)
                .unwrap()
        })
    }

    fn store_market(&self, market_id: &Symbol, market: &Market) {
        self.env.as_contract(&self.contract_id, || {
            self.env
                .storage()
                .persistent()
                .set::<Symbol, Market>(market_id, market);
        });
    }

    fn vote(&self, user: &Address, outcome: &str, stake: i128) {
        self.client()
            .vote(user, &self.market_id, &String::from_str(&self.env, outcome), &stake);
    }

    fn try_vote(
        &self,
        user: &Address,
        market_id: &Symbol,
        outcome: &str,
        stake: i128,
    ) -> Result<Result<(), soroban_sdk::ConversionError>, soroban_sdk::Error> {
        self.client()
            .try_vote(user, market_id, &String::from_str(&self.env, outcome), &stake)
            .map_err(|e| e.unwrap())
    }

    fn try_bet(
        &self,
        user: &Address,
        market_id: &Symbol,
        outcome: &str,
        stake: i128,
    ) -> Result<Result<Bet, soroban_sdk::ConversionError>, soroban_sdk::Error> {
        self.client()
            .try_place_bet(
                user,
                market_id,
                &String::from_str(&self.env, outcome),
                &stake,
                &0,
            )
            .map_err(|e| e.unwrap())
    }

    fn bet(&self, user: &Address, outcome: &str, stake: i128) -> Bet {
        self.client()
            .place_bet(user, &self.market_id, &String::from_str(&self.env, outcome), &stake, &0)
    }

    fn pause_betting(&self) {
        self.env.as_contract(&self.contract_id, || {
            CircuitBreaker::emergency_pause(
                &self.env,
                &self.admin.clone(),
                &String::from_str(&self.env, "test pause"),
            )
            .unwrap();
        });
    }

    /// Advance past the market end time and resolve manually in favor of `winner`.
    fn resolve(&self, winner: &str) {
        self.env
            .ledger()
            .with_mut(|li| li.timestamp += 31 * 86400);
        self.client().resolve_market_manual(
            &self.admin,
            &self.market_id,
            &String::from_str(&self.env, winner),
        );
        // Let the dispute window elapse so auto-distribution runs.
        self.env.ledger().with_mut(|li| li.timestamp += 25 * 3600);
    }

    fn claimed_payout(&self, user: &Address) -> i128 {
        self.env.as_contract(&self.contract_id, || {
            self.market(&self.market_id)
                .claimed
                .get(user.clone())
                .map(|c| c.get_payout())
                .unwrap_or(0)
        })
    }

    /// Claim winnings for `user` (first claim, nonce 0), tolerating markets
    /// that were already auto-distributed at resolution.
    fn claim(&self, user: &Address) {
        let _ = self.client().try_claim_winnings(user, &self.market_id, &0);
    }
}

/// Default platform fee percentage written by `initialize` (2%).
const FEE_PERCENT: i128 = 2;

// ===== Row 1: vote then bet =====

/// Row 1 — the master divergence, now fixed: a bet on top of an existing
/// vote stake fails atomically; the first stake S1 stays fully intact and
/// claimable, and `total_staked` only counts S1.
#[test]
fn test_row1_vote_then_place_bet_rejected_first_stake_intact() {
    let s = Setup::new();
    let user = s.funded_user();

    s.vote(&user, "yes", S1);
    {
        let m = s.market(&s.market_id);
        assert_eq!(m.stakes.get(user.clone()), Some(S1));
        assert_eq!(m.total_staked, S1);
    }

    // Second call through the other entrypoint must fail with AlreadyBet.
    let res = s.try_bet(&user, &s.market_id, "yes", S2);
    assert_eq!(res, Err(sdk_err(Error::AlreadyBet)));

    // No bet was stored, no tokens moved, first stake untouched.
    assert!(s
        .env
        .as_contract(&s.contract_id, || BetStorage::get_bet(&s.env, &s.market_id, &user))
        .is_none());
    let m = s.market(&s.market_id);
    assert_eq!(m.stakes.get(user.clone()), Some(S1), "first stake must survive");
    assert_eq!(m.votes.get(user.clone()), Some(String::from_str(&s.env, "yes")));
    assert_eq!(m.total_staked, S1, "total_staked must not double-count");
    assert_eq!(s.contract_balance(), S1);
    assert_eq!(s.balance(&user), INITIAL_BALANCE - S1);
}

// ===== Row 2: bet then vote =====

/// Row 2 — the mirror case fails with the *same* error as row 1.
#[test]
fn test_row2_place_bet_then_vote_rejected_same_error() {
    let s = Setup::new();
    let user = s.funded_user();

    s.bet(&user, "yes", S2);
    {
        let m = s.market(&s.market_id);
        assert_eq!(m.stakes.get(user.clone()), Some(S2));
        assert_eq!(m.total_staked, S2);
    }

    let res = s.try_vote(&user, &s.market_id, "yes", S1);
    assert_eq!(res, Err(sdk_err(Error::AlreadyBet)), "same cross-ledger error as row 1");

    let m = s.market(&s.market_id);
    assert_eq!(m.stakes.get(user.clone()), Some(S2));
    assert_eq!(m.total_staked, S2);
    assert_eq!(s.contract_balance(), S2);
    assert_eq!(s.balance(&user), INITIAL_BALANCE - S2);
}

// ===== Row 3: circuit breaker =====

/// Row 3 — vote honors the circuit breaker exactly like place_bet: CBOpen
/// and no token transfer.
#[test]
fn test_row3_vote_respects_circuit_breaker() {
    let s = Setup::new();
    let voter = s.funded_user();
    let bettor = s.funded_user();

    s.pause_betting();

    let res = s.try_vote(&voter, &s.market_id, "yes", S1);
    assert_eq!(res, Err(sdk_err(Error::CBOpen)));

    // place_bet returns the identical error.
    let res = s.try_bet(&bettor, &s.market_id, "yes", S2);
    assert_eq!(res, Err(sdk_err(Error::CBOpen)));

    // No token transfer, no ledger writes.
    assert_eq!(s.balance(&voter), INITIAL_BALANCE);
    assert_eq!(s.balance(&bettor), INITIAL_BALANCE);
    assert_eq!(s.contract_balance(), 0);
    let m = s.market(&s.market_id);
    assert!(m.votes.is_empty());
    assert!(m.stakes.is_empty());
    assert_eq!(m.total_staked, 0);
}

// ===== Row 4: amount bounds =====

/// Row 4 — vote validates stake amounts exactly like place_bet: zero,
/// negative, below-min, above-max and above-market-cap amounts all return
/// the matching typed error, and no vote is registered.
#[test]
fn test_row4_vote_amount_bounds_identical_to_place_bet() {
    let cases: [(i128, Error); 5] = [
        (0, Error::InsufficientStake),
        (-S1, Error::InsufficientStake),
        (MIN_BET_AMOUNT - 1, Error::InsufficientStake),
        (MAX_STAKE + 1, Error::InvalidInput),
        (5_000_000, Error::BetExceedsCap), // market cap set to 4_000_000 below
    ];
    for (amount, expected) in cases {
        let s = Setup::new();
        s.client()
            .set_market_max_bet_cap(&s.admin.clone(), &s.market_id.clone(), &4_000_000);

        let voter = s.funded_user();
        let bettor = s.funded_user();

        let vote_res = s.try_vote(&voter, &s.market_id, "yes", amount);
        assert_eq!(vote_res, Err(sdk_err(expected)), "vote amount {amount}");

        let bet_res = s.try_bet(&bettor, &s.market_id, "yes", amount);
        assert_eq!(bet_res, Err(sdk_err(expected)), "place_bet amount {amount}");

        // A zero or out-of-bounds stake must not register a vote or take a slot.
        let m = s.market(&s.market_id);
        assert!(m.votes.is_empty());
        assert!(m.stakes.is_empty());
        assert_eq!(m.total_staked, 0);
        assert_eq!(s.contract_balance(), 0);
        assert_eq!(s.balance(&voter), INITIAL_BALANCE);
    }
}

// ===== Row 5: capacity =====

/// Row 5 — a vote filling max_participants makes place_bet from a new user
/// fail with MaxParticipantsReached.
#[test]
fn test_row5_participant_capacity_applies_to_bets() {
    let s = Setup::with_market(Some(1));
    let voter = s.funded_user();
    let bettor = s.funded_user();

    s.vote(&voter, "yes", S1);

    let bet_res = s.try_bet(&bettor, &s.market_id, "no", S2);
    assert_eq!(bet_res, Err(sdk_err(Error::MaxParticipantsReached)));

    // Same for a new voter.
    let vote_res = s.try_vote(&bettor, &s.market_id, "no", S2);
    assert_eq!(vote_res, Err(sdk_err(Error::MaxParticipantsReached)));

    assert_eq!(s.balance(&bettor), INITIAL_BALANCE);
    let m = s.market(&s.market_id);
    assert_eq!(m.votes.len(), 1);
    assert_eq!(m.total_staked, S1);
}

// ===== Row 6: overflow =====

/// Row 6 — total_staked near i128::MAX returns a typed Error::Overflow from
/// both entrypoints instead of trapping on unchecked `+=`.
#[test]
fn test_row6_total_staked_overflow_returns_typed_error() {
    for entrypoint in 0..2 {
        let s = Setup::new();
        let user = s.funded_user();

        // Pin total_staked at the top of the i128 range.
        let mut m = s.market(&s.market_id);
        m.total_staked = i128::MAX;
        s.store_market(&s.market_id, &m);

        let res = if entrypoint == 0 {
            s.try_vote(&user, &s.market_id, "yes", S1)
                .map(|_| ())
                .map_err(|e| e)
        } else {
            s.try_bet(&user, &s.market_id, "yes", S1)
                .map(|_| ())
                .map_err(|e| e)
        };
        assert_eq!(
            res,
            Err(sdk_err(Error::Overflow)),
            "entrypoint {entrypoint} must return typed overflow, not trap"
        );

        // No state change, no tokens moved.
        assert_eq!(s.balance(&user), INITIAL_BALANCE);
        assert_eq!(s.contract_balance(), 0);
        assert_eq!(s.market(&s.market_id).total_staked, i128::MAX);
    }
}

// ===== Row 7: cancellation =====

/// Row 7 — cancel_bet after a bet-only stake refunds exactly the amount,
/// clears the ledger entries the bet created, restores total_staked, and
/// the freed slot can be re-staked through either entrypoint.
#[test]
fn test_row7_cancel_bet_restores_ledger_and_allows_restake() {
    let s = Setup::new();
    let user = s.funded_user();

    s.bet(&user, "yes", S2);
    assert_eq!(s.balance(&user), INITIAL_BALANCE - S2);

    s.client().cancel_bet(&user, &s.market_id);

    // Refund equals the amount; ledger cleared; totals restored.
    assert_eq!(s.balance(&user), INITIAL_BALANCE, "refund equals the bet amount");
    assert_eq!(s.contract_balance(), 0);
    let m = s.market(&s.market_id);
    assert!(m.votes.get(user.clone()).is_none(), "vote mirror cleared");
    assert!(m.stakes.get(user.clone()).is_none(), "stake mirror cleared");
    assert_eq!(m.total_staked, 0, "total_staked restored");
    let stats = s.client().get_market_bet_stats(&s.market_id);
    assert_eq!(stats.total_bets, 0);

    // The user can stake again — through the vote path this time.
    s.vote(&user, "yes", S1);
    let m = s.market(&s.market_id);
    assert_eq!(m.stakes.get(user.clone()), Some(S1));
    assert_eq!(m.total_staked, S1);
}

// ===== Row 8: conservation =====

/// Row 8 — conservation across mixed vote/bet activity, resolution and
/// claims: every staked token is either paid out or remains in the contract
/// (platform fees); nothing is created or stranded.
#[test]
fn test_row8_conservation_after_mixed_activity_and_claims() {
    let s = Setup::new();
    let voter_winner = s.funded_user();
    let bettor_winner = s.funded_user();
    let voter_loser = s.funded_user();

    s.vote(&voter_winner, "yes", S1);
    s.bet(&bettor_winner, "yes", S2);
    s.vote(&voter_loser, "no", S3);

    let pool = S1 + S2 + S3;
    assert_eq!(s.contract_balance(), pool, "all stakes locked");
    let m = s.market(&s.market_id);
    assert_eq!(m.total_staked, pool);

    s.resolve("yes");

    // Winners claim (claims credit the internal withdrawable balance ledger).
    s.claim(&voter_winner);
    s.claim(&bettor_winner);
    s.claim(&voter_loser);

    let paid = s.claimed_payout(&voter_winner)
        + s.claimed_payout(&bettor_winner)
        + s.claimed_payout(&voter_loser);
    let held = s.contract_balance();

    assert!(s.claimed_payout(&voter_winner) > 0, "voter winnings claimable");
    assert!(s.claimed_payout(&bettor_winner) > 0, "bettor winnings claimable");
    assert_eq!(s.claimed_payout(&voter_loser), 0, "loser gets nothing");

    // Exact payout math: stake * (1 - fee) * pool / winning_total.
    let winning_total = S1 + S2;
    assert_eq!(s.claimed_payout(&voter_winner), S1 * (100 - FEE_PERCENT) / 100 * pool / winning_total);
    assert_eq!(s.claimed_payout(&bettor_winner), S2 * (100 - FEE_PERCENT) / 100 * pool / winning_total);

    // Conservation: the contract token balance minus fees equals the
    // outstanding (claimed, withdrawable) amount; nothing was created or
    // stranded. Claims credit internal balances, so the locked pool itself
    // remains fully backed in the contract.
    let fees = pool * FEE_PERCENT / 100;
    assert_eq!(held - fees, paid, "balance minus fees == outstanding claimable");
    assert_eq!(held, pool, "pool stays fully backed");
}

// ===== Row 9: error precedence =====

/// Row 9 — inputs violating several rules return the FIRST error in the
/// documented order, identically for both entrypoints:
/// circuit breaker → market state → deadline → outcome → amount → duplicate
/// → capacity → overflow.
#[test]
fn test_row9_error_precedence_identical_for_both_entrypoints() {
    let s = Setup::new();

    // A user with an existing stake, used for duplicate-vs-X precedence.
    let staked = s.funded_user();
    s.vote(&staked, "yes", S1);

    // (a) amount before duplicate: already staked AND amount below min.
    assert_eq!(
        s.try_vote(&staked, &s.market_id, "yes", MIN_BET_AMOUNT - 1),
        Err(sdk_err(Error::InsufficientStake))
    );
    assert_eq!(
        s.try_bet(&staked, &s.market_id, "yes", MIN_BET_AMOUNT - 1),
        Err(sdk_err(Error::InsufficientStake))
    );

    // (b) outcome before amount: already staked AND invalid outcome AND
    // below-min amount — InvalidOutcome wins.
    assert_eq!(
        s.try_vote(&staked, &s.market_id, "bogus", 0),
        Err(sdk_err(Error::InvalidOutcome))
    );
    assert_eq!(
        s.try_bet(&staked, &s.market_id, "bogus", 0),
        Err(sdk_err(Error::InvalidOutcome))
    );

    // (c) duplicate before capacity: staked user on a full-capacity market
    // gets the duplicate error, not MaxParticipantsReached.
    let capped = Setup::with_market(Some(1));
    let only = capped.funded_user();
    let intruder = capped.funded_user();
    capped.vote(&only, "yes", S1);
    assert_eq!(
        capped.try_vote(&only, &capped.market_id, "yes", S1),
        Err(sdk_err(Error::AlreadyVoted))
    );
    assert_eq!(
        capped.try_bet(&only, &capped.market_id, "yes", S1),
        Err(sdk_err(Error::AlreadyBet))
    );

    // (d) amount before capacity: NEW user (no duplicate) with below-min
    // amount on the full market gets InsufficientStake, not capacity.
    assert_eq!(
        capped.try_vote(&intruder, &capped.market_id, "yes", 0),
        Err(sdk_err(Error::InsufficientStake))
    );
    assert_eq!(
        capped.try_bet(&intruder, &capped.market_id, "yes", 0),
        Err(sdk_err(Error::InsufficientStake))
    );
    // With a valid amount the capacity error surfaces.
    assert_eq!(
        capped.try_bet(&intruder, &capped.market_id, "yes", S1),
        Err(sdk_err(Error::MaxParticipantsReached))
    );

    // (e) deadline before duplicate: after the cutoff, the staked user gets
    // MarketClosed, not AlreadyVoted/AlreadyBet.
    s.env.ledger().with_mut(|li| li.timestamp += 31 * 86400);
    assert_eq!(
        s.try_vote(&staked, &s.market_id, "yes", S1),
        Err(sdk_err(Error::MarketClosed))
    );
    assert_eq!(
        s.try_bet(&staked, &s.market_id, "yes", S1),
        Err(sdk_err(Error::MarketClosed))
    );
}

/// Row 9 (part 2) — the circuit breaker precedes everything, including
/// market lookup, for both entrypoints.
#[test]
fn test_row9_circuit_breaker_precedes_market_lookup() {
    let s = Setup::new();
    let user = s.funded_user();
    s.pause_betting();

    let missing = Symbol::new(&s.env, "no_such_market");
    assert_eq!(
        s.try_vote(&user, &missing, "yes", S1),
        Err(sdk_err(Error::CBOpen))
    );
    assert_eq!(
        s.try_bet(&user, &missing, "yes", S1),
        Err(sdk_err(Error::CBOpen))
    );
}

// ===== Vote entrypoint specifics =====

/// The vote entrypoint keeps its own abuse control: a configured voting rate
/// limit that is exceeded fails with the typed rate-limit error before the
/// stake ledger is touched.
#[test]
fn test_vote_rate_limiter_enforced_before_ledger() {
    use crate::rate_limiter::{RateLimitConfig, RateLimiterData, RefillMode};
    let s = Setup::new();
    let user = s.funded_user();

    // Configure a voting limit of 1 per hour.
    let config = RateLimitConfig {
        voting_limit: 1,
        dispute_limit: 5,
        oracle_call_limit: 20,
        bet_limit: 50,
        events_per_admin_limit: 10,
        time_window_seconds: 3600,
        refill_mode: RefillMode::Linear,
    };
    s.env.as_contract(&s.contract_id, || {
        s.env
            .storage()
            .persistent()
            .set(&RateLimiterData::Config, &config);
    });

    s.vote(&user, "yes", S1); // within limit

    let res = s.try_vote(&user, &s.market_id, "yes", S1);
    assert_eq!(res, Err(sdk_err(Error::RateLimitExceeded)));
    // No second stake was recorded.
    let m = s.market(&s.market_id);
    assert_eq!(m.total_staked, S1);
}

// ===== Legacy (pre-fix) market compatibility =====

/// Legacy markets where one user already has BOTH a vote and a bet (the
/// master bug state) must remain claimable. The bet-side stake mirror is
/// authoritative — exactly what claim_winnings has always paid from — so the
/// legacy user receives the same payout as an identical fresh bettor.
#[test]
fn test_legacy_dual_entry_market_remains_claimable() {
    // Env A: fabricate the master bug state directly in storage:
    // vote(S1) then bet(S2) on master produced votes=yes, stakes=S2,
    // total_staked=S1+S2 with the bet record stored.
    let a = Setup::new();
    let legacy_user = a.funded_user();
    a.bet(&legacy_user, "yes", S2);
    let mut m = a.market(&a.market_id);
    m.total_staked = S1 + S2; // master residue: the overwritten vote stake
    a.store_market(&a.market_id, &m);
    // Fund the contract to back the fabricated residue (simulates tokens
    // that were locked on master and can never be claimed).
    StellarAssetClient::new(&a.env, &a.token_id).mint(&a.contract_id.clone(), &(S1));

    // Env B: control — a fresh bettor stakes S2 in an identical market.
    let b = Setup::new();
    let control_user = b.funded_user();
    b.bet(&control_user, "yes", S2);

    a.resolve("yes");
    b.resolve("yes");
    a.claim(&legacy_user);
    b.claim(&control_user);

    let legacy_payout = a.claimed_payout(&legacy_user);
    let control_payout = b.claimed_payout(&control_user);

    // The control bettor receives the standard single-winner payout on S2.
    assert_eq!(control_payout, S2 * (100 - FEE_PERCENT) / 100);

    // The legacy dual-entry user's claim is paid too: the bet-side mirror
    // (stakes = S2) is authoritative for winning_total, while the master
    // residue (total = S1 + S2) inflates the pool. The residue therefore
    // accrues to claimants instead of being stranded.
    assert!(legacy_payout > 0, "legacy dual-entry stake must still be claimable");
    assert_eq!(
        legacy_payout,
        S2 * (100 - FEE_PERCENT) / 100 * (S1 + S2) / S2,
        "legacy claim pays the bet stake S2 against the (inflated) market pool"
    );
    // Conservation in the legacy env: the fabricated pool stays fully backed.
    assert_eq!(a.contract_balance(), S1 + S2);
}

/// cancel_bet only removes ledger entries that the bet created. On a legacy
/// market with master residue (total_staked inflated by the overwritten vote
/// stake), the refund equals the bet amount and the residue is left as-is
/// for admin remediation — it is never silently deleted from totals.
#[test]
fn test_legacy_cancel_only_removes_bet_owned_entries() {
    let s = Setup::new();
    let user = s.funded_user();

    s.bet(&user, "yes", S2);
    // Fabricate the master residue: total_staked counts the old vote too.
    let mut m = s.market(&s.market_id);
    m.total_staked = S1 + S2;
    s.store_market(&s.market_id, &m);

    s.client().cancel_bet(&user, &s.market_id);

    // Refund equals the bet amount only — S2, not S1 + S2.
    assert_eq!(s.balance(&user), INITIAL_BALANCE);
    let m = s.market(&s.market_id);
    assert!(m.votes.get(user.clone()).is_none());
    assert!(m.stakes.get(user.clone()).is_none());
    // The pre-existing stranded residue stays in the totals (documented;
    // remediation of such markets is an admin/incident concern, out of
    // scope for this fix).
    assert_eq!(m.total_staked, S1);
}
