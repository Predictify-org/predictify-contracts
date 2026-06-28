//! Issue #617 – Deterministic event ordering test for `resolve_market`.
//!
//! Verifies that `MarketResolutionManager::resolve_market` emits the three
//! resolution-signalling events in the exact, deterministic sequence:
//!
//! 1. `mkt_res`        – market resolved  (`emit_market_resolved`)
//! 2. `st_chng`        – state change     (`emit_state_change_event`)
//! 3. `idx_transition` – indexer hook     (`emit_resolution_transition_hook`)
//!
//! The full event stream from resolve_market also includes a `market_state_change`
//! event (emitted by `set_winning_outcomes`) and storage events from `store_event`.
//! This test verifies the relative ordering of the three resolution events only.

#[cfg(test)]
mod resolution_event_ordering_tests {
    use crate::config::ConfigManager;
    use crate::resolution::MarketResolutionManager;
    use crate::types::{Market, MarketState, OracleConfig, OracleProvider};
    use crate::PredictifyHybrid;
    use soroban_sdk::testutils::{Address as _, Events, Ledger, LedgerInfo};
    use soroban_sdk::{symbol_short, xdr, Address, Env, String, Symbol, TryIntoVal, Vec};

    // ---------------------------------------------------------------------------
    // Helpers
    // ---------------------------------------------------------------------------

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
                let cfg = ConfigManager::get_development_config(&env);
                ConfigManager::store_config(&env, &cfg).unwrap();
            });
            Self { env, contract_id, admin }
        }

        /// Store a market in `Ended` state, with oracle result set and one vote.
        fn store_ready_market(&self, market_id: &Symbol) {
            let end_time = self.env.ledger().timestamp().saturating_sub(10);
            let mut outcomes = Vec::new(&self.env);
            outcomes.push_back(String::from_str(&self.env, "yes"));
            outcomes.push_back(String::from_str(&self.env, "no"));
            let oracle_cfg = OracleConfig::new(
                OracleProvider::reflector(),
                Address::from_str(
                    &self.env,
                    "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
                ),
                String::from_str(&self.env, "BTC/USD"),
                50_000_00,
                String::from_str(&self.env, "gt"),
            );
            let mut market = Market::new(
                &self.env,
                self.admin.clone(),
                String::from_str(&self.env, "Will BTC reach $50k?"),
                outcomes,
                end_time,
                oracle_cfg,
                None,
                86400,
                MarketState::Ended,
            );
            market.oracle_result = Some(String::from_str(&self.env, "yes"));
            market.votes.set(self.admin.clone(), String::from_str(&self.env, "yes"));
            market.stakes.set(self.admin.clone(), 1_000_000_i128);
            market.total_staked = 1_000_000_i128;
            self.env.storage().persistent().set(market_id, &market);
        }
    }

    /// Try to extract the first topic Symbol from an XDR ContractEvent.
    /// Returns None if the first topic is not a Symbol (e.g. a String).
    fn first_topic_sym(env: &Env, event: &xdr::ContractEvent) -> Option<Symbol> {
        let v0 = match &event.body {
            xdr::ContractEventBody::V0(v0) => v0,
        };
        let scval = v0.topics.get(0)?;
        scval.clone().try_into_val(env).ok()
    }

    // ---------------------------------------------------------------------------
    // Issue #617 – core deterministic ordering test
    // ---------------------------------------------------------------------------

    /// Verifies that `resolve_market` emits `mkt_res`, `st_chng`, and
    /// `idx_transition` in that exact order relative to one another.
    #[test]
    fn test_resolve_market_emits_events_in_deterministic_order() {
        let setup = Setup::new();
        let market_id = Symbol::new(&setup.env, "mkt_617");

        setup.env.as_contract(&setup.contract_id, || {
            setup.store_ready_market(&market_id);

            let count_before = setup.env.events().all().events().len();

            MarketResolutionManager::resolve_market(&setup.env, &market_id)
                .expect("resolve_market should succeed");

            let all = setup.env.events().all();
            let emitted = &all.events()[count_before..];

            assert!(!emitted.is_empty(), "resolve_market must emit at least one event");

            // Collect the indices (relative positions) of the three key events.
            let mkt_res_sym = symbol_short!("mkt_res");
            let st_chng_sym = symbol_short!("st_chng");
            let idx_trans_sym = Symbol::new(&setup.env, "idx_transition");

            let pos_mkt_res = emitted
                .iter()
                .position(|e| first_topic_sym(&setup.env, e) == Some(mkt_res_sym.clone()))
                .expect("mkt_res event must be emitted by resolve_market");

            let pos_st_chng = emitted
                .iter()
                .position(|e| first_topic_sym(&setup.env, e) == Some(st_chng_sym.clone()))
                .expect("st_chng event must be emitted by resolve_market");

            let pos_idx = emitted
                .iter()
                .position(|e| first_topic_sym(&setup.env, e) == Some(idx_trans_sym.clone()))
                .expect("idx_transition event must be emitted by resolve_market");

            // Deterministic ordering: mkt_res → st_chng → idx_transition
            assert!(
                pos_mkt_res < pos_st_chng,
                "mkt_res (pos={}) must come before st_chng (pos={})",
                pos_mkt_res,
                pos_st_chng
            );
            assert!(
                pos_st_chng < pos_idx,
                "st_chng (pos={}) must come before idx_transition (pos={})",
                pos_st_chng,
                pos_idx
            );
        });
    }

    /// Tie-outcome edge case: when community votes are split evenly between two
    /// outcomes the event ordering contract still holds — `mkt_res`, `st_chng`,
    /// and `idx_transition` must be emitted in that exact sequence.
    #[test]
    fn test_event_ordering_preserved_on_tie_outcome() {
        let setup = Setup::new();
        let market_id = Symbol::new(&setup.env, "mkt_tie");

        setup.env.as_contract(&setup.contract_id, || {
            let end_time = setup.env.ledger().timestamp().saturating_sub(10);
            let mut outcomes = Vec::new(&setup.env);
            outcomes.push_back(String::from_str(&setup.env, "yes"));
            outcomes.push_back(String::from_str(&setup.env, "no"));
            let oracle_cfg = OracleConfig::new(
                OracleProvider::reflector(),
                Address::from_str(
                    &setup.env,
                    "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
                ),
                String::from_str(&setup.env, "BTC/USD"),
                50_000_00,
                String::from_str(&setup.env, "gt"),
            );
            let voter_a = Address::generate(&setup.env);
            let voter_b = Address::generate(&setup.env);
            let mut market = Market::new(
                &setup.env,
                setup.admin.clone(),
                String::from_str(&setup.env, "Tie market"),
                outcomes,
                end_time,
                oracle_cfg,
                None,
                86400,
                MarketState::Ended,
            );
            market.oracle_result = Some(String::from_str(&setup.env, "yes"));
            // Equal votes and equal stakes → exact community tie
            market.votes.set(voter_a.clone(), String::from_str(&setup.env, "yes"));
            market.votes.set(voter_b.clone(), String::from_str(&setup.env, "no"));
            market.stakes.set(voter_a.clone(), 1_000_000_i128);
            market.stakes.set(voter_b.clone(), 1_000_000_i128);
            market.total_staked = 2_000_000_i128;
            setup.env.storage().persistent().set(&market_id, &market);

            let count_before = setup.env.events().all().events().len();

            MarketResolutionManager::resolve_market(&setup.env, &market_id)
                .expect("resolve_market should succeed even on tie");

            let all = setup.env.events().all();
            let emitted = &all.events()[count_before..];

            assert!(!emitted.is_empty(), "resolve_market must emit events on tie outcome");

            let mkt_res_sym = symbol_short!("mkt_res");
            let st_chng_sym = symbol_short!("st_chng");
            let idx_trans_sym = Symbol::new(&setup.env, "idx_transition");

            let pos_mkt_res = emitted
                .iter()
                .position(|e| first_topic_sym(&setup.env, e) == Some(mkt_res_sym.clone()))
                .expect("mkt_res must be emitted on tie resolution");
            let pos_st_chng = emitted
                .iter()
                .position(|e| first_topic_sym(&setup.env, e) == Some(st_chng_sym.clone()))
                .expect("st_chng must be emitted on tie resolution");
            let pos_idx = emitted
                .iter()
                .position(|e| first_topic_sym(&setup.env, e) == Some(idx_trans_sym.clone()))
                .expect("idx_transition must be emitted on tie resolution");

            assert!(
                pos_mkt_res < pos_st_chng,
                "tie: mkt_res (pos={}) must precede st_chng (pos={})",
                pos_mkt_res,
                pos_st_chng
            );
            assert!(
                pos_st_chng < pos_idx,
                "tie: st_chng (pos={}) must precede idx_transition (pos={})",
                pos_st_chng,
                pos_idx
            );
        });
    }

    /// Sanity check: no `mkt_res`, `st_chng`, or `idx_transition` events are
    /// emitted when resolution fails early (no oracle result available).
    #[test]
    fn test_no_resolution_events_emitted_when_resolution_fails() {
        let setup = Setup::new();
        let market_id = Symbol::new(&setup.env, "mkt_no_res");

        setup.env.as_contract(&setup.contract_id, || {
            let end_time = setup.env.ledger().timestamp().saturating_sub(10);
            let mut outcomes = Vec::new(&setup.env);
            outcomes.push_back(String::from_str(&setup.env, "yes"));
            outcomes.push_back(String::from_str(&setup.env, "no"));
            let oracle_cfg = OracleConfig::new(
                OracleProvider::reflector(),
                Address::from_str(
                    &setup.env,
                    "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
                ),
                String::from_str(&setup.env, "BTC/USD"),
                50_000_00,
                String::from_str(&setup.env, "gt"),
            );
            // No oracle_result — resolution must fail before emitting.
            let market = Market::new(
                &setup.env,
                setup.admin.clone(),
                String::from_str(&setup.env, "No oracle"),
                outcomes,
                end_time,
                oracle_cfg,
                None,
                86400,
                MarketState::Ended,
            );
            setup.env.storage().persistent().set(&market_id, &market);

            let count_before = setup.env.events().all().events().len();
            let result = MarketResolutionManager::resolve_market(&setup.env, &market_id);
            assert!(result.is_err(), "should fail without oracle result");
            let count_after = setup.env.events().all().events().len();

            assert_eq!(
                count_before, count_after,
                "no events should be emitted on failed resolution"
            );
        });
    }
}
