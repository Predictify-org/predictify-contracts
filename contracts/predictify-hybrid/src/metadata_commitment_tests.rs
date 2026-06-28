#![cfg(test)]

use crate::types::{Market, MarketState, OracleConfig, OracleProvider};
use crate::{PredictifyHybrid, PredictifyHybridClient};
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{vec, Address, Env, String, Symbol};

const ORACLE_ADDRESS: &str = "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";

fn oracle_config(env: &Env, feed_id: &str, threshold: i128) -> OracleConfig {
    OracleConfig::new(
        OracleProvider::reflector(),
        Address::from_str(env, ORACLE_ADDRESS),
        String::from_str(env, feed_id),
        threshold,
        String::from_str(env, "gt"),
    )
}

fn market(env: &Env) -> Market {
    Market::new(
        env,
        Address::generate(env),
        String::from_str(env, "Will BTC close above $100k this month?"),
        vec![
            env,
            String::from_str(env, "yes"),
            String::from_str(env, "no"),
        ],
        env.ledger().timestamp() + 86_400,
        oracle_config(env, "BTC", 100_000_00),
        None,
        86_400,
        MarketState::Active,
    )
}

#[test]
fn metadata_commitment_is_generated_from_canonical_fields() {
    let env = Env::default();
    let market = market(&env);

    let expected = Market::compute_metadata_commitment(
        &env,
        &market.question,
        &market.outcomes,
        &market.oracle_config,
    );

    assert_eq!(market.metadata_commitment, expected);
    assert!(market.verify_metadata_commitment(&env, &expected));
}

#[test]
fn verify_market_metadata_entrypoint_returns_true_for_matching_commitment() {
    let env = Env::default();
    let contract_id = env.register(PredictifyHybrid, ());
    let client = PredictifyHybridClient::new(&env, &contract_id);
    let market_id = Symbol::new(&env, "m_meta_ok");
    let market = market(&env);
    let expected = market.metadata_commitment.clone();

    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&market_id, &market);
    });

    assert!(client.verify_market_metadata(&market_id, &expected));
}

#[test]
fn verify_market_metadata_returns_false_for_each_mutated_committed_field() {
    let env = Env::default();
    let contract_id = env.register(PredictifyHybrid, ());
    let client = PredictifyHybridClient::new(&env, &contract_id);

    let base = market(&env);
    let expected = base.metadata_commitment.clone();

    let question_id = Symbol::new(&env, "m_q_mut");
    let mut changed_question = base.clone();
    changed_question.question = String::from_str(&env, "Will ETH close above $10k this month?");
    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&question_id, &changed_question);
    });
    assert!(!client.verify_market_metadata(&question_id, &expected));

    let outcomes_id = Symbol::new(&env, "m_o_mut");
    let mut changed_outcomes = base.clone();
    changed_outcomes.outcomes = vec![
        &env,
        String::from_str(&env, "above"),
        String::from_str(&env, "below"),
    ];
    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&outcomes_id, &changed_outcomes);
    });
    assert!(!client.verify_market_metadata(&outcomes_id, &expected));

    let oracle_id = Symbol::new(&env, "m_oracle");
    let mut changed_oracle = base.clone();
    changed_oracle.oracle_config = oracle_config(&env, "ETH", 10_000_00);
    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&oracle_id, &changed_oracle);
    });
    assert!(!client.verify_market_metadata(&oracle_id, &expected));
}

#[test]
fn verify_market_metadata_returns_false_for_stale_expected_after_refresh() {
    let env = Env::default();
    let mut market = market(&env);
    let old_expected = market.metadata_commitment.clone();

    market.outcomes = vec![
        &env,
        String::from_str(&env, "above"),
        String::from_str(&env, "not_above"),
    ];
    market.refresh_metadata_commitment(&env);

    assert!(!market.verify_metadata_commitment(&env, &old_expected));
    assert!(market.verify_metadata_commitment(&env, &market.metadata_commitment));
}

#[test]
fn verify_market_metadata_returns_false_for_missing_market() {
    let env = Env::default();
    let contract_id = env.register(PredictifyHybrid, ());
    let client = PredictifyHybridClient::new(&env, &contract_id);
    let missing_id = Symbol::new(&env, "missing");
    let expected = market(&env).metadata_commitment;

    assert!(!client.verify_market_metadata(&missing_id, &expected));
}
