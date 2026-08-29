use crate::analytics_snapshot::{AnalyticsSnapshotEnvelope, AnalyticsSnapshotManager};
use crate::err::Error;
use crate::types::{Market, MarketState, OracleConfig};
use crate::PredictifyHybrid;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{symbol_short, Address, Env, String, Symbol, Vec};

fn make_market(env: &Env, market_id: Symbol) -> Market {
    let admin = Address::generate(env);
    let question = String::from_str(env, "Will GrantFox ship by Q4?");
    let outcomes = Vec::from_array(env, [String::from_str(env, "yes"), String::from_str(env, "no")]);
    Market::new(
        env,
        admin,
        question,
        outcomes,
        1_000_000,
        OracleConfig::none_sentinel(env),
        None,
        60,
        MarketState::Active,
    )
}

#[test]
fn market_analytics_snapshot_is_deterministic_and_round_trippable() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PredictifyHybrid {}, ());
    let market_id = Symbol::new(&env, "grantfox_market");

    env.as_contract(&contract_id, || {
        let mut market = make_market(&env, market_id.clone());
        let user_a = Address::generate(&env);
        let user_b = Address::generate(&env);
        market.add_vote(user_a.clone(), String::from_str(&env, "yes"), 10_000);
        market.add_vote(user_b.clone(), String::from_str(&env, "no"), 5_000);
        env.storage().persistent().set(&market_id, &market);

        let envelope = PredictifyHybrid::get_market_analytics_snapshot(env.clone(), market_id.clone())
            .expect("snapshot should be available for an existing market");

        assert_eq!(envelope.schema_version, AnalyticsSnapshotManager::schema_version());
        assert_eq!(envelope.taken_at, env.ledger().timestamp());

        let decoded = AnalyticsSnapshotEnvelope::decode(&env, &envelope)
            .expect("snapshot envelope should decode");

        assert_eq!(decoded.market_id, market_id);
        assert_eq!(decoded.total_votes, 2);
        assert_eq!(decoded.total_staked, 15_000);
        assert_eq!(decoded.total_dispute_stakes, 0);
        assert_eq!(decoded.outcome_counts.len(), 2);
        assert_eq!(decoded.outcome_counts.get(0).unwrap().outcome, String::from_str(&env, "yes"));
        assert_eq!(decoded.outcome_counts.get(0).unwrap().count, 1);
        assert_eq!(decoded.outcome_counts.get(1).unwrap().outcome, String::from_str(&env, "no"));
        assert_eq!(decoded.outcome_counts.get(1).unwrap().count, 1);
        assert_eq!(decoded.participant_count, 2);

        let re_encoded = AnalyticsSnapshotEnvelope::encode(&env, &decoded);
        assert_eq!(envelope.payload, re_encoded.payload);
    });
}

#[test]
fn market_analytics_snapshot_returns_market_not_found_for_unknown_market() {
    let env = Env::default();
    let contract_id = env.register(PredictifyHybrid {}, ());
    let market_id = Symbol::new(&env, "missing_market");

    env.as_contract(&contract_id, || {
        let result = PredictifyHybrid::get_market_analytics_snapshot(env.clone(), market_id);
        assert_eq!(result, Err(Error::MarketNotFound));
    });
}
