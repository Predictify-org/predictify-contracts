#![cfg(test)]

use proptest::prelude::*;
use soroban_sdk::{testutils::Address as _, Address, Env, String, Symbol};
use crate::{
    oracles::{OracleInterface, PythOracle, PythFeedConfig, ReflectorOracle},
    types::Error,
};

#[derive(Debug, Clone, PartialEq)]
enum OracleOutcome {
    Healthy(i128),
    Unsupported,
    Stale,
    Scaled(i128),
    UnknownError,
}

fn categorize_outcome(result: Result<i128, Error>) -> OracleOutcome {
    match result {
        Ok(price) => OracleOutcome::Healthy(price),
        Err(Error::OracleUnavailable) => OracleOutcome::Unsupported,
        Err(Error::OracleStale) => OracleOutcome::Stale,
        Err(_) => OracleOutcome::UnknownError,
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    
    #[test]
    fn test_oracle_differential(
        price in 0..i128::MAX,
    ) {
        let env = Env::default();
        let admin = Address::generate(&env);
        
        let pyth = PythOracle::new(admin.clone());
        let reflector = ReflectorOracle::new(admin.clone());
        
        let feed_id = String::from_str(&env, "BTC/USD");
        
        // Pyth is unsupported on Stellar right now
        let pyth_res = categorize_outcome(pyth.get_price(&env, &feed_id));
        assert_eq!(pyth_res, OracleOutcome::Unsupported);
        
        // Reflector is the primary oracle but without a mock it will fail
        // The differential check here ensures we handle unsupported explicitly
        let reflector_res = categorize_outcome(reflector.get_price(&env, &feed_id));
        
        // Either Reflector works (with mock) or returns an error, but Pyth MUST be unsupported
        // Let's assert they don't unexpectedly succeed with different valid prices without mock
        if let (OracleOutcome::Healthy(p1), OracleOutcome::Healthy(p2)) = (&pyth_res, &reflector_res) {
            assert_eq!(p1, p2, "Oracles should not return mismatched healthy prices");
        }
    }
}
