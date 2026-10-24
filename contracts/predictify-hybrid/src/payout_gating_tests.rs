#![cfg(test)]

//! Regression tests for issue #1456: resolution is one-way and every payout
//! path is gated on the post-resolution dispute window.
//!
//! Timeline enforced by these tests:
//!
//! ```text
//! end_time          resolved_at            resolved_at + dispute_window
//!    |                   |                              |
//!    |<-- can resolve -->|<-- disputes open ----------->|<-- payouts open
//! ```
//!
//! The window is measured from `resolved_at` (persisted as the claim-window
//! start), not from `end_time`.

use crate::config::ConfigManager;
use crate::err::Error;
use crate::recovery::UnclaimedWinningsPolicy;
use crate::storage::BalanceStorage;
use crate::types::{Market, MarketState, OracleConfig, OracleProvider, ReflectorAsset};
use crate::{PredictifyHybrid, PredictifyHybridClient};
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{vec, Address, Env, String, Symbol};

const DAY: u64 = 86_400;
const WINDOW: u64 = 3_600;

struct Setup {
    env: Env,
    contract_id: Address,
    admin: Address,
    token_id: Address,
}

impl Setup {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let contract_id = env.register(PredictifyHybrid, ());
        let token_contract = env.register_stellar_asset_contract_v2(Address::generate(&env));
        let token_id = token_contract.address();

        env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .set(&Symbol::new(&env, "TokenID"), &token_id);
            env.storage()
                .persistent()
                .set(&Symbol::new(&env, "platform_fee"), &200i128);
            crate::circuit_breaker::CircuitBreaker::initialize(&env).unwrap();
            let cfg = ConfigManager::get_development_config(&env);
            ConfigManager::store_config(&env, &cfg).unwrap();
            crate::admin::AdminInitializer::initialize(&env, &admin).unwrap();
        });

        Self {
            env,
            contract_id,
            admin,
            token_id,
        }
    }

    fn client(&self) -> PredictifyHybridClient<'_> {
        PredictifyHybridClient::new(&self.env, &self.contract_id)
    }

    fn user(&self) -> Address {
        let user = Address::generate(&self.env);
        soroban_sdk::token::StellarAssetClient::new(&self.env, &self.token_id)
            .mint(&user, &100_000_000_000i128);
        user
    }

    fn create_market(&self, dispute_window_seconds: u64) -> Symbol {
        let outcomes = vec![
            &self.env,
            String::from_str(&self.env, "yes"),
            String::from_str(&self.env, "no"),
        ];
        self.client().create_market(
            &self.admin,
            &String::from_str(&self.env, "Payout gating market"),
            &outcomes,
            &1u32,
            &OracleConfig::new(
                OracleProvider::reflector(),
                Address::from_str(
                    &self.env,
                    "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
                ),
                String::from_str(&self.env, "BTC/USD"),
                5_000_000,
                String::from_str(&self.env, "gt"),
            ),
            &None,
            &86_400u64,
            &None,
            &None,
            &Some(dispute_window_seconds),
            &None,
            &None,
        )
    }

    fn advance(&self, seconds: u64) {
        self.env.ledger().with_mut(|li| li.timestamp += seconds);
    }

    fn stake_on(&self, user: &Address, market_id: &Symbol, outcome: &str, amount: i128) {
        self.client().place_bet(
            user,
            market_id,
            &String::from_str(&self.env, outcome),
            &amount,
            &250,
        );
    }

    fn resolve(&self, market_id: &Symbol, outcome: &str) {
        self.client().resolve_market_manual(
            &self.admin,
            market_id,
            &String::from_str(&self.env, outcome),
        );
    }

    fn balance(&self, user: &Address) -> i128 {
        self.env.as_contract(&self.contract_id, || {
            BalanceStorage::get_balance(&self.env, user, &ReflectorAsset::Stellar).amount
        })
    }

    fn market(&self, market_id: &Symbol) -> Market {
        self.client().get_market(market_id).unwrap()
    }

    fn set_state(&self, market_id: &Symbol, state: MarketState) {
        self.env.as_contract(&self.contract_id, || {
            let mut market: Market = self.env.storage().persistent().get(market_id).unwrap();
            market.state = state;
            self.env.storage().persistent().set(market_id, &market);
        });
    }
}

fn panic_market_error(error: Error) -> soroban_sdk::Error {
    soroban_sdk::Error::from_contract_error(error as u32)
}

// Row 1: resolution alone must not credit anybody.
#[test]
fn resolution_does_not_pay_immediately() {
    let s = Setup::new();
    let winner = s.user();
    let id = s.create_market(WINDOW);
    s.stake_on(&winner, &id, "yes", 10_000_000);

    s.advance(DAY + 1);
    s.resolve(&id, "yes");

    assert_eq!(s.balance(&winner), 0, "resolution must not pay instantly");
    let market = s.market(&id);
    assert!(market.claimed.get(winner.clone()).is_none());
}

// Row 2: distribute_payouts is rejected while the window is open.
#[test]
fn distribute_blocked_inside_window() {
    let s = Setup::new();
    let winner = s.user();
    let id = s.create_market(WINDOW);
    s.stake_on(&winner, &id, "yes", 10_000_000);

    s.advance(DAY + 1);
    s.resolve(&id, "yes");

    let result = s.client().try_distribute_payouts(&id);
    assert_eq!(result, Err(Ok(Error::InvalidState)));
    assert_eq!(
        s.balance(&winner),
        0,
        "blocked distribution must not credit"
    );
}

// Row 3: distribution after the window credits winners exactly once.
#[test]
fn distribute_after_window_credits_once() {
    let s = Setup::new();
    let winner = s.user();
    let id = s.create_market(WINDOW);
    s.stake_on(&winner, &id, "yes", 10_000_000);

    s.advance(DAY + 1);
    s.resolve(&id, "yes");
    s.advance(WINDOW + 1);

    let distributed = s.client().distribute_payouts(&id);
    assert!(distributed > 0);
    let paid = s.balance(&winner);
    assert_eq!(paid, distributed);

    let second = s.client().distribute_payouts(&id);
    assert_eq!(second, 0, "a second distribution must not pay again");
    assert_eq!(s.balance(&winner), paid);
}

// Row 4: a settled market cannot be re-resolved with a different outcome.
#[test]
fn second_resolution_is_rejected_and_outcome_unchanged() {
    let s = Setup::new();
    let id = s.create_market(WINDOW);

    s.advance(DAY + 1);
    s.resolve(&id, "yes");

    let result =
        s.client()
            .try_resolve_market_manual(&s.admin, &id, &String::from_str(&s.env, "no"));
    assert_eq!(result, Err(Ok(panic_market_error(Error::MarketResolved))));

    let market = s.market(&id);
    assert_eq!(
        market.get_winning_outcome(),
        Some(String::from_str(&s.env, "yes"))
    );
}

// Row 5: a cancelled market cannot be resolved.
#[test]
fn resolving_cancelled_market_is_invalid_state() {
    let s = Setup::new();
    let id = s.create_market(WINDOW);

    s.advance(DAY + 1);
    s.set_state(&id, MarketState::Cancelled);

    let result =
        s.client()
            .try_resolve_market_manual(&s.admin, &id, &String::from_str(&s.env, "yes"));
    assert_eq!(result, Err(Ok(panic_market_error(Error::InvalidState))));
}

// Row 6: a late resolution gets a fresh dispute window measured from
// resolution time, and the window closes one window later.
#[test]
fn late_resolution_starts_a_fresh_dispute_window() {
    let s = Setup::new();
    let disputer = s.user();

    let late = s.create_market(DAY);
    s.advance(2 * DAY + 1);
    s.resolve(&late, "yes");

    // Old code measured from end_time and would reject this dispute because
    // end_time + DAY is already in the past.
    s.client().dispute_market(
        &disputer,
        &late,
        &10_000_000i128,
        &Some(String::from_str(&s.env, "late resolution challenged")),
    );

    let on_time = s.create_market(DAY);
    s.advance(2 * DAY + 1);
    s.resolve(&on_time, "yes");
    s.advance(DAY + 1);

    let result = s
        .client()
        .try_dispute_market(&s.user(), &on_time, &10_000_000i128, &None);
    assert_eq!(result, Err(Ok(Error::MarketResolved)));
}

// Row 7: an open dispute keeps payouts frozen after the window closes.
#[test]
fn open_dispute_blocks_payouts_after_window() {
    let s = Setup::new();
    let winner = s.user();
    let disputer = s.user();
    let id = s.create_market(WINDOW);
    s.stake_on(&winner, &id, "yes", 10_000_000);

    s.advance(DAY + 1);
    s.resolve(&id, "yes");
    s.advance(10);
    s.client()
        .dispute_market(&disputer, &id, &10_000_000i128, &None);

    s.advance(WINDOW + 1);

    let distribute = s.client().try_distribute_payouts(&id);
    assert_eq!(distribute, Err(Ok(Error::InvalidState)));

    let claim = s.client().try_claim_winnings(&winner, &id, &0u64);
    assert_eq!(claim, Err(Ok(panic_market_error(Error::InvalidState))));
    assert_eq!(s.balance(&winner), 0);
}

// Row 8: claim_winnings and distribute_payouts cannot double credit.
#[test]
fn claim_and_distribute_do_not_double_credit() {
    let s = Setup::new();
    let winner = s.user();
    let id = s.create_market(WINDOW);
    s.stake_on(&winner, &id, "yes", 10_000_000);

    s.advance(DAY + 1);
    s.resolve(&id, "yes");
    s.advance(WINDOW + 1);

    s.client().claim_winnings(&winner, &id, &0u64);
    let after_claim = s.balance(&winner);
    assert!(after_claim > 0);

    let distributed = s.client().distribute_payouts(&id);
    assert_eq!(distributed, 0);
    assert_eq!(s.balance(&winner), after_claim);
}

// Row 9: a legacy resolved market without a recorded resolution time loads
// and is gated using `end_time` as the documented fallback.
#[test]
fn legacy_market_without_resolved_at_uses_end_time_fallback() {
    let s = Setup::new();
    let id = Symbol::new(&s.env, "legacy_market");
    let end_time = s.env.ledger().timestamp().saturating_sub(WINDOW / 2);
    let outcomes = vec![
        &s.env,
        String::from_str(&s.env, "yes"),
        String::from_str(&s.env, "no"),
    ];
    let mut market = Market::new(
        &s.env,
        s.admin.clone(),
        String::from_str(&s.env, "Legacy resolved market"),
        outcomes,
        end_time,
        OracleConfig::new(
            OracleProvider::reflector(),
            Address::from_str(
                &s.env,
                "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
            ),
            String::from_str(&s.env, "BTC/USD"),
            5_000_000,
            String::from_str(&s.env, "gt"),
        ),
        None,
        0,
        MarketState::Resolved,
    );
    market.dispute_window_seconds = WINDOW;
    let mut winning = soroban_sdk::Vec::new(&s.env);
    winning.push_back(String::from_str(&s.env, "yes"));
    market.winning_outcomes = Some(winning);

    s.env.as_contract(&s.contract_id, || {
        s.env.storage().persistent().set(&id, &market);
        // Explicitly ensure no resolution timestamp was recorded.
        assert_eq!(
            UnclaimedWinningsPolicy::get_claim_window_start(&s.env, &id, end_time),
            end_time
        );
        assert_eq!(
            PredictifyHybrid::require_payout_allowed(&s.env, &id, &market),
            Err(Error::InvalidState)
        );
    });

    s.advance(WINDOW);

    s.env.as_contract(&s.contract_id, || {
        assert_eq!(
            PredictifyHybrid::require_payout_allowed(&s.env, &id, &market),
            Ok(())
        );
    });
}

// Row 10: conservation — the credited total never exceeds the pool.
#[test]
fn total_credited_never_exceeds_pool() {
    let s = Setup::new();
    let winner = s.user();
    let loser = s.user();
    let id = s.create_market(WINDOW);
    s.stake_on(&winner, &id, "yes", 10_000_000);
    s.stake_on(&loser, &id, "no", 5_000_000);

    s.advance(DAY + 1);
    s.resolve(&id, "yes");
    s.advance(WINDOW + 1);

    let distributed = s.client().distribute_payouts(&id);
    let credited = s.balance(&winner) + s.balance(&loser);
    let market = s.market(&id);

    assert!(distributed > 0);
    assert_eq!(credited, distributed);
    assert!(
        credited <= market.total_staked,
        "payouts must not exceed the pool"
    );
}
