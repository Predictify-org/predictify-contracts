#![no_std]

use predictify_hybrid::market_analytics::{
    DisputeAnalytics, FeeAnalytics, MarketComparisonAnalytics, MarketStatistics,
    OraclePerformanceStats, ParticipationMetrics, TimeFrame, VotingAnalytics,
};
use predictify_hybrid::{OracleProvider, PredictifyHybridClient};
use soroban_sdk::{contract, contractimpl, Address, Env, Symbol, Vec};

/// Read-only analytics façade for data maintained by the hybrid contract.
#[contract]
pub struct PredictifyAnalytics;

#[contractimpl]
impl PredictifyAnalytics {
    pub fn get_market_statistics(
        env: Env,
        core_contract: Address,
        market_id: Symbol,
    ) -> MarketStatistics {
        PredictifyHybridClient::new(&env, &core_contract)
            .get_market_analytics_statistics(&market_id)
    }

    pub fn get_voting_analytics(
        env: Env,
        core_contract: Address,
        market_id: Symbol,
    ) -> VotingAnalytics {
        PredictifyHybridClient::new(&env, &core_contract).get_voting_analytics(&market_id)
    }

    pub fn get_dispute_analytics(
        env: Env,
        core_contract: Address,
        market_id: Symbol,
    ) -> DisputeAnalytics {
        PredictifyHybridClient::new(&env, &core_contract).get_dispute_analytics(&market_id)
    }

    pub fn get_participation_metrics(
        env: Env,
        core_contract: Address,
        market_id: Symbol,
    ) -> ParticipationMetrics {
        PredictifyHybridClient::new(&env, &core_contract).get_participation_metrics(&market_id)
    }

    pub fn get_market_comparison_analytics(
        env: Env,
        core_contract: Address,
        markets: Vec<Symbol>,
    ) -> MarketComparisonAnalytics {
        PredictifyHybridClient::new(&env, &core_contract).get_market_comparison_analytics(&markets)
    }

    pub fn get_fee_analytics(
        env: Env,
        core_contract: Address,
        timeframe: TimeFrame,
    ) -> FeeAnalytics {
        PredictifyHybridClient::new(&env, &core_contract).get_fee_analytics(&timeframe)
    }

    pub fn get_oracle_performance_stats(
        env: Env,
        core_contract: Address,
        oracle: OracleProvider,
    ) -> OraclePerformanceStats {
        PredictifyHybridClient::new(&env, &core_contract).get_oracle_performance_stats(&oracle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use predictify_hybrid::Error;
    use soroban_sdk::{contract, contractimpl, Env, Map, String};

    #[contract]
    struct MockCoreContract;

    #[contractimpl]
    impl MockCoreContract {
        pub fn get_market_analytics_statistics(
            env: Env,
            market_id: Symbol,
        ) -> Result<MarketStatistics, Error> {
            Ok(MarketStatistics {
                market_id,
                total_participants: 3,
                total_stake: 120,
                total_votes: 3,
                outcome_distribution: Map::new(&env),
                stake_distribution: Map::new(&env),
                average_stake: 40,
                participation_rate: 75,
                market_volatility: 10,
                consensus_strength: 80,
                time_to_resolution: 600,
                resolution_method: String::from_str(&env, "oracle"),
            })
        }
    }

    #[test]
    fn forwards_market_statistics_from_core_contract() {
        let env = Env::default();
        let core_id = env.register(MockCoreContract, ());
        let analytics_id = env.register(PredictifyAnalytics, ());
        let client = PredictifyAnalyticsClient::new(&env, &analytics_id);
        let market_id = Symbol::new(&env, "market_1");

        let statistics = client.get_market_statistics(&core_id, &market_id);

        assert_eq!(statistics.market_id, market_id);
        assert_eq!(statistics.total_participants, 3);
        assert_eq!(statistics.total_stake, 120);
        assert_eq!(statistics.average_stake, 40);
    }
}
