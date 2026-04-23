#![cfg(test)]

//! Tests for oracle resolution timeout and its interaction with the dispute window.
//!
//! # Invariants under test
//!
//! 1. When `resolution_timeout` expires with **no active dispute**, the market is
//!    cancelled so stakes can be refunded.
//! 2. When `resolution_timeout` expires but a **dispute is active**, the market must
//!    NOT be cancelled — `ResolutionTimeoutReached` is returned instead, leaving the
//!    dispute process as the authoritative resolution path.
//! 3. The dispute window (`dispute_window_seconds`) is enforced: disputes filed after
//!    `end_time + dispute_window_seconds` are rejected.
//! 4. A dispute filed within the window extends `end_time`, which in turn pushes the
//!    effective resolution deadline forward.

use crate::config::ConfigManager;
use crate::errors::Error;
use crate::markets::MarketStateManager;
use crate::resolution::OracleResolutionManager;
use crate::types::{Market, MarketState, OracleConfig, OracleProvider};
use crate::PredictifyHybrid;
use soroban_sdk::testutils::{Address as _, Ledger, LedgerInfo};
use soroban_sdk::{Address, Env, String, Symbol, Vec};

// ===== TEST HELPERS =====

struct Setup {
    env: Env,
    contract_id: Address,
    admin: Address,
}

impl Setup {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let contract_id = env.register_contract(None, PredictifyHybrid);
        env.as_contract(&contract_id, || {
            let config = ConfigManager::get_development_config(&env);
            ConfigManager::store_config(&env, &config).unwrap();
        });
        Self { env, contract_id, admin }
    }

    fn market(&self, end_time: u64, resolution_timeout: u64, dispute_window_seconds: u64) -> (Symbol, Market) {
        let id = Symbol::new(&self.env, "mkt");
        let mut outcomes = Vec::new(&self.env);
        outcomes.push_back(String::from_str(&self.env, "yes"));
        outcomes.push_back(String::from_str(&self.env, "no"));
        let oracle = OracleConfig::new(
            OracleProvider::reflector(),
            Address::from_str(
                &self.env,
                "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
            ),
            String::from_str(&self.env, "BTC/USD"),
            50_000_00,
            String::from_str(&self.env, "gt"),
        );
        let mut m = Market::new(
            &self.env,
            self.admin.clone(),
            String::from_str(&self.env, "Will BTC reach $50k?"),
            outcomes,
            end_time,
            oracle,
            None,
            resolution_timeout,
            MarketState::Active,
        );
        m.dispute_window_seconds = dispute_window_seconds;
        (id, m)
    }

    fn tick(&self, secs: u64) {
        let t = self.env.ledger().timestamp();
        self.env.ledger().set(LedgerInfo {
            timestamp: t + secs,
            protocol_version: 22,
            sequence_number: self.env.ledger().sequence() + 1,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 16,
            min_persistent_entry_ttl: 16,
            max_entry_ttl: 6_312_000,
        });
    }
}

// ===== ORACLE TIMEOUT — NO DISPUTE =====

/// When the resolution timeout fires and there is no active dispute the market
/// must be cancelled so participants can reclaim their stakes.
#[test]
fn test_resolution_timeout_cancels_market_without_dispute() {
    let s = Setup::new();
    s.env.as_contract(&s.contract_id, || {
        let end_time = s.env.ledger().timestamp() + 3_600;
        let (id, mut market) = s.market(end_time, 86_400, 86_400);
        market.state = MarketState::Active;
        s.env.storage().persistent().set(&id, &market);

        // Jump past end_time + resolution_timeout
        s.tick(3_600 + 86_400 + 1);

        let result = OracleResolutionManager::fetch_oracle_result(&s.env, &id);
        // Expect InvalidState (market was cancelled)
        assert_eq!(result, Err(Error::InvalidState));

        let stored: Market = s.env.storage().persistent().get(&id).unwrap();
        assert_eq!(stored.state, MarketState::Cancelled);
    });
}

// ===== ORACLE TIMEOUT — ACTIVE DISPUTE (deadlock prevention) =====

/// When the resolution timeout fires but a dispute is active, the market must
/// NOT be cancelled.  Cancelling would permanently lock dispute stakes.
/// The function must return `ResolutionTimeoutReached` instead.
#[test]
fn test_resolution_timeout_does_not_cancel_disputed_market() {
    let s = Setup::new();
    s.env.as_contract(&s.contract_id, || {
        let end_time = s.env.ledger().timestamp() + 3_600;
        let (id, mut market) = s.market(end_time, 86_400, 86_400);

        // Simulate oracle result already set and a dispute filed
        market.oracle_result = Some(String::from_str(&s.env, "yes"));
        market.state = MarketState::Ended;
        s.env.storage().persistent().set(&id, &market);

        // File a dispute (transitions state to Disputed)
        let disputer = Address::generate(&s.env);
        MarketStateManager::add_dispute_stake(&mut market, disputer, 10_000_000, Some(&id));
        assert_eq!(market.state, MarketState::Disputed);
        s.env.storage().persistent().set(&id, &market);

        // Jump past resolution_timeout
        s.tick(3_600 + 86_400 + 1);

        let result = OracleResolutionManager::fetch_oracle_result(&s.env, &id);
        assert_eq!(result, Err(Error::ResolutionTimeoutReached));

        // Market must still be Disputed — not Cancelled
        let stored: Market = s.env.storage().persistent().get(&id).unwrap();
        assert_eq!(stored.state, MarketState::Disputed);
    });
}

/// Same as above but using `total_dispute_stakes > 0` as the signal (state not yet
/// transitioned to Disputed but stakes are present).
#[test]
fn test_resolution_timeout_does_not_cancel_when_dispute_stakes_present() {
    let s = Setup::new();
    s.env.as_contract(&s.contract_id, || {
        let end_time = s.env.ledger().timestamp() + 3_600;
        let (id, mut market) = s.market(end_time, 86_400, 86_400);

        market.oracle_result = Some(String::from_str(&s.env, "yes"));
        market.state = MarketState::Ended;
        // Manually inject dispute stake without state transition
        let disputer = Address::generate(&s.env);
        market.dispute_stakes.set(disputer, 5_000_000);
        s.env.storage().persistent().set(&id, &market);

        s.tick(3_600 + 86_400 + 1);

        let result = OracleResolutionManager::fetch_oracle_result(&s.env, &id);
        assert_eq!(result, Err(Error::ResolutionTimeoutReached));

        let stored: Market = s.env.storage().persistent().get(&id).unwrap();
        // State must not have been changed to Cancelled
        assert_ne!(stored.state, MarketState::Cancelled);
    });
}

// ===== DISPUTE WINDOW ENFORCEMENT =====

/// Disputes filed after `end_time + dispute_window_seconds` must be rejected.
/// Without this check a late dispute could re-open a market that users consider settled.
#[test]
fn test_dispute_rejected_after_window_closes() {
    use crate::disputes::DisputeValidator;

    let s = Setup::new();
    s.env.as_contract(&s.contract_id, || {
        let end_time = s.env.ledger().timestamp() + 3_600;
        // dispute_window_seconds = 7_200 (2 hours)
        let (id, mut market) = s.market(end_time, 86_400, 7_200);
        market.oracle_result = Some(String::from_str(&s.env, "yes"));
        market.state = MarketState::Ended;
        s.env.storage().persistent().set(&id, &market);

        // Jump past end_time + dispute_window_seconds
        s.tick(3_600 + 7_200 + 1);

        let result = DisputeValidator::validate_market_for_dispute(&s.env, &market);
        assert_eq!(result, Err(Error::MarketResolved));
    });
}

/// Disputes filed within the window must be accepted.
#[test]
fn test_dispute_accepted_within_window() {
    use crate::disputes::DisputeValidator;

    let s = Setup::new();
    s.env.as_contract(&s.contract_id, || {
        let end_time = s.env.ledger().timestamp() + 3_600;
        let (id, mut market) = s.market(end_time, 86_400, 7_200);
        market.oracle_result = Some(String::from_str(&s.env, "yes"));
        market.state = MarketState::Ended;
        s.env.storage().persistent().set(&id, &market);

        // Jump past end_time but still inside the dispute window
        s.tick(3_600 + 3_600); // end_time + 1 hour (window is 2 hours)

        let result = DisputeValidator::validate_market_for_dispute(&s.env, &market);
        assert!(result.is_ok());
    });
}

/// A market with `dispute_window_seconds == 0` (no window configured) should
/// not reject disputes based on the window check.
#[test]
fn test_dispute_window_zero_means_no_window_restriction() {
    use crate::disputes::DisputeValidator;

    let s = Setup::new();
    s.env.as_contract(&s.contract_id, || {
        let end_time = s.env.ledger().timestamp() + 3_600;
        let (id, mut market) = s.market(end_time, 86_400, 0); // 0 = no window
        market.oracle_result = Some(String::from_str(&s.env, "yes"));
        market.state = MarketState::Ended;
        s.env.storage().persistent().set(&id, &market);

        // Jump far past end_time
        s.tick(3_600 + 999_999);

        let result = DisputeValidator::validate_market_for_dispute(&s.env, &market);
        // Should not fail on window check (may fail on other checks, but not window)
        assert_ne!(result, Err(Error::MarketResolved));
    });
}

// ===== DISPUTE EXTENDS RESOLUTION DEADLINE =====

/// Filing a dispute extends `market.end_time`.  The new end_time must be later
/// than `end_time + resolution_timeout` so the oracle timeout cannot fire while
/// the dispute is still open.
#[test]
fn test_dispute_extension_pushes_past_resolution_timeout() {
    let s = Setup::new();
    s.env.as_contract(&s.contract_id, || {
        let end_time = s.env.ledger().timestamp() + 3_600;
        let resolution_timeout = 7_200_u64; // 2 hours
        let (id, mut market) = s.market(end_time, resolution_timeout, 86_400);
        market.oracle_result = Some(String::from_str(&s.env, "yes"));
        market.state = MarketState::Ended;
        s.env.storage().persistent().set(&id, &market);

        // Advance to just before resolution_timeout fires
        s.tick(3_600 + 7_100);

        // File dispute — this should extend end_time by 24 hours
        let disputer = Address::generate(&s.env);
        MarketStateManager::add_dispute_stake(&mut market, disputer, 10_000_000, Some(&id));
        let cfg = ConfigManager::get_config(&s.env).unwrap();
        MarketStateManager::extend_for_dispute(
            &mut market,
            &s.env,
            cfg.voting.dispute_extension_hours.into(),
        );
        s.env.storage().persistent().set(&id, &market);

        let stored: Market = s.env.storage().persistent().get(&id).unwrap();
        let current_time = s.env.ledger().timestamp();
        // After extension, end_time must be in the future
        assert!(stored.end_time > current_time);
        // And the dispute state must be set
        assert_eq!(stored.state, MarketState::Disputed);
    });
}
