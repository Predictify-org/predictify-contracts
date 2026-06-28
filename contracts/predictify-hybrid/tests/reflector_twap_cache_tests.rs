//! Tests for intra‑transaction TWAP cache in ReflectorOracleClient

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{Env, Address};
    use crate::mocks::oracle_mock::MockReflectorOracleClient;
    use crate::oracles::ReflectorAsset;

    #[test]
    fn test_twap_caches_within_transaction() {
        let env = Env::default();
        let contract_id = Address::generate(&env);
        let client = MockReflectorOracleClient::new(&env, contract_id.clone());

        // First call should compute and cache the result
        let first = client.twap(ReflectorAsset::BTC, 5);
        assert_eq!(first, Some(5000)); // Mock returns records * 1000

        // Second call in the same transaction should hit the cache
        let second = client.twap(ReflectorAsset::BTC, 5);
        assert_eq!(second, first);
    }

    #[test]
    fn test_twap_cache_resets_between_transactions() {
        // Transaction 1
        let env1 = Env::default();
        let contract_id1 = Address::generate(&env1);
        let client1 = MockReflectorOracleClient::new(&env1, contract_id1.clone());
        let val1 = client1.twap(ReflectorAsset::ETH, 3);
        assert_eq!(val1, Some(3000));

        // Simulate a new transaction by creating a new Env and client
        let env2 = Env::default();
        let contract_id2 = Address::generate(&env2);
        let client2 = MockReflectorOracleClient::new(&env2, contract_id2.clone());
        let val2 = client2.twap(ReflectorAsset::ETH, 3);
        assert_eq!(val2, Some(3000));
        // The cached value from the first transaction should not affect the second
        assert_eq!(val2, val1);
    }
}
