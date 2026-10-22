//! Utility functions for working with claim nonces in tests and client code.
//!
//! This module provides convenience functions to make it easier to work with
//! the replay-safe claim nonce system.

#[cfg(test)]
pub mod test_helpers {
    use super::super::*;
    use soroban_sdk::{Address, Env, Symbol};

    /// Helper to claim winnings with automatic nonce retrieval
    pub fn claim_with_auto_nonce(
        env: &Env,
        contract_id: &Address,
        user: &Address,
        market_id: &Symbol,
    ) -> i128 {
        let client = crate::test::PredictifyHybridClient::new(env, contract_id);
        let nonce = client.get_claim_nonce(user, market_id);
        client.claim_winnings_with_nonce(user, market_id, nonce)
    }

    /// Helper to get current nonce and validate it matches expected
    pub fn validate_nonce_advanced(
        env: &Env,
        contract_id: &Address,
        user: &Address,
        market_id: &Symbol,
        expected: u64,
    ) -> Result<u64, String> {
        let client = crate::test::PredictifyHybridClient::new(env, contract_id);
        let actual = client.get_claim_nonce(user, market_id);
        if actual == expected {
            Ok(actual)
        } else {
            Err(format!("Nonce mismatch: expected {}, got {}", expected, actual))
        }
    }
}
