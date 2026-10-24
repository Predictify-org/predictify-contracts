#![no_std]

use soroban_sdk::{
    contract, contractclient, contractimpl, contracttype, Address, Env, Map, String, Symbol, Vec,
};

// Keep these ABI types local so this facade does not link the hybrid contract.
#[contracttype]
#[derive(Clone, Debug)]
pub struct MarketStatistics {
    pub market_id: Symbol,
    pub total_participants: u32,
    pub total_stake: i128,
    pub total_votes: u32,
    pub outcome_distribution: Map<String, u32>,
    pub stake_distribution: Map<String, i128>,
    pub average_stake: i128,
    pub participation_rate: u32,
    pub market_volatility: u32,
    pub consensus_strength: u32,
    pub time_to_resolution: u64,
    pub resolution_method: String,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct VotingAnalytics {
    pub market_id: Symbol,
    pub total_votes: u32,
    pub unique_voters: u32,
    pub voting_timeline: Map<u64, u32>,
    pub outcome_preferences: Map<String, u32>,
    pub stake_concentration: Map<Address, i128>,
    pub voting_patterns: Map<String, u32>,
    pub participation_trends: Vec<u32>,
    pub consensus_evolution: Vec<u32>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OracleProvider {
    Reflector,
    Pyth,
    BandProtocol,
    DIA,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct OraclePerformanceStats {
    pub oracle_provider: OracleProvider,
    pub total_requests: u32,
    pub successful_requests: u32,
    pub failed_requests: u32,
    pub average_response_time: u64,
    pub accuracy_rate: u32,
    pub uptime_percentage: u32,
    pub last_update: u64,
    pub reliability_score: u32,
    pub performance_trends: Vec<u32>,
}

#[contracttype]
#[derive(Clone, Debug)]
pub enum TimeFrame {
    Hour,
    Day,
    Week,
    Month,
    Quarter,
    Year,
    AllTime,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct FeeAnalytics {
    pub timeframe: TimeFrame,
    pub total_fees_collected: i128,
    pub platform_fees: i128,
    pub dispute_fees: i128,
    pub creation_fees: i128,
    pub fee_distribution: Map<String, i128>,
    pub average_fee_per_market: i128,
    pub fee_collection_rate: u32,
    pub revenue_trends: Vec<i128>,
    pub fee_optimization_score: u32,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct DisputeAnalytics {
    pub market_id: Symbol,
    pub total_disputes: u32,
    pub resolved_disputes: u32,
    pub pending_disputes: u32,
    pub dispute_stakes: i128,
    pub average_resolution_time: u64,
    pub dispute_success_rate: u32,
    pub dispute_reasons: Map<String, u32>,
    pub resolution_methods: Map<String, u32>,
    pub dispute_trends: Vec<u32>,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct ParticipationMetrics {
    pub market_id: Symbol,
    pub total_participants: u32,
    pub active_participants: u32,
    pub new_participants: u32,
    pub returning_participants: u32,
    pub participation_rate: u32,
    pub engagement_score: u32,
    pub retention_rate: u32,
    pub participant_demographics: Map<String, u32>,
    pub activity_patterns: Map<String, u32>,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct MarketComparisonAnalytics {
    pub markets: Vec<Symbol>,
    pub total_markets: u32,
    pub average_participation: u32,
    pub average_stake: i128,
    pub success_rate: u32,
    pub resolution_efficiency: u32,
    pub market_performance_ranking: Map<Symbol, u32>,
    pub comparative_metrics: Map<String, i128>,
    pub market_categories: Map<Symbol, String>,
    pub performance_insights: Vec<String>,
}

#[contractclient(name = "PredictifyHybridClient")]
pub trait PredictifyHybrid {
    fn get_market_analytics_statistics(env: Env, market_id: Symbol) -> MarketStatistics;
    fn get_voting_analytics(env: Env, market_id: Symbol) -> VotingAnalytics;
    fn get_dispute_analytics(env: Env, market_id: Symbol) -> DisputeAnalytics;
    fn get_participation_metrics(env: Env, market_id: Symbol) -> ParticipationMetrics;
    fn get_market_comparison_analytics(env: Env, markets: Vec<Symbol>)
        -> MarketComparisonAnalytics;
    fn get_fee_analytics(env: Env, timeframe: TimeFrame) -> FeeAnalytics;
    fn get_oracle_performance_stats(env: Env, oracle: OracleProvider) -> OraclePerformanceStats;
}

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
    use soroban_sdk::{contract, contractimpl, Env, Map, String};

    #[contract]
    struct MockCoreContract;

    #[contractimpl]
    impl MockCoreContract {
        pub fn get_market_analytics_statistics(env: Env, market_id: Symbol) -> MarketStatistics {
            MarketStatistics {
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
            }
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
