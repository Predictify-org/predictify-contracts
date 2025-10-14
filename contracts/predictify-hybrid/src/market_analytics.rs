#![allow(dead_code)]

use crate::errors::Error;
use crate::types::*;
use soroban_sdk::{contracttype, vec, Address, Env, Map, String, Symbol, Vec};

/// Market Analytics module for comprehensive data analysis and insights
///
/// This module provides detailed analytics functions for market statistics,
/// voting analytics, oracle performance tracking, fee analytics, dispute
/// analytics, and participation metrics. It enables comprehensive market
/// monitoring and optimization.

// ===== ANALYTICS TYPES =====

/// Comprehensive market statistics for data analysis
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

/// Voting analytics and participation metrics
#[contracttype]
#[derive(Clone, Debug)]
pub struct VotingAnalytics {
    pub market_id: Symbol,
    pub total_votes: u32,
    pub unique_voters: u32,
    pub voting_timeline: Map<u64, u32>, // timestamp -> vote count
    pub outcome_preferences: Map<String, u32>,
    pub stake_concentration: Map<Address, i128>,
    pub voting_patterns: Map<String, u32>,
    pub participation_trends: Vec<u32>,
    pub consensus_evolution: Vec<u32>,
}

/// Oracle performance tracking and statistics
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

/// Fee analytics and revenue tracking
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

/// Dispute analytics and resolution metrics
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

/// Participation metrics for market analysis
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

/// Market comparison analytics for multiple markets
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

/// Time frame enumeration for analytics
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

// ===== MARKET ANALYTICS IMPLEMENTATION =====

/// Market Analytics implementation for comprehensive data analysis
pub struct MarketAnalyticsManager;

impl MarketAnalyticsManager {
    /// Get comprehensive market statistics for a specific market
    pub fn get_market_statistics(env: &Env, market_id: Symbol) -> Result<MarketStatistics, Error> {
        let market = env
            .storage()
            .persistent()
            .get::<Symbol, Market>(&market_id)
            .ok_or(Error::MarketNotFound)?;

        let total_participants = market.votes.len() as u32;
        let total_stake = market.total_staked;
        let total_votes = market.votes.len() as u32;

        // Calculate outcome distribution
        let mut outcome_distribution = Map::new(env);
        let mut stake_distribution = Map::new(env);
        let mut total_stake_by_outcome = Map::new(env);

        for (user, outcome) in market.votes.iter() {
            let stake = market.stakes.get(user.clone()).unwrap_or(0);

            // Count votes per outcome
            let vote_count = outcome_distribution.get(outcome.clone()).unwrap_or(0);
            outcome_distribution.set(outcome.clone(), vote_count + 1);

            // Sum stakes per outcome
            let outcome_stake = total_stake_by_outcome.get(outcome.clone()).unwrap_or(0);
            total_stake_by_outcome.set(outcome.clone(), outcome_stake + stake);
        }

        // Set stake distribution
        for (outcome, stake) in total_stake_by_outcome.iter() {
            stake_distribution.set(outcome, stake);
        }

        let average_stake = if total_participants > 0 {
            total_stake / total_participants as i128
        } else {
            0
        };

        let participation_rate = if total_participants > 0 {
            (total_participants * 100) / (total_participants + 10) // Placeholder calculation
        } else {
            0
        };

        let market_volatility = Self::calculate_market_volatility(&market);
        let consensus_strength = Self::calculate_consensus_strength(&market);
        let time_to_resolution = Self::calculate_time_to_resolution(&market);
        let resolution_method = Self::get_resolution_method(&market);

        Ok(MarketStatistics {
            market_id,
            total_participants,
            total_stake,
            total_votes,
            outcome_distribution,
            stake_distribution,
            average_stake,
            participation_rate,
            market_volatility,
            consensus_strength,
            time_to_resolution,
            resolution_method,
        })
    }

    /// Get voting analytics and participation metrics for a market
    pub fn get_voting_analytics(env: &Env, market_id: Symbol) -> Result<VotingAnalytics, Error> {
        let market = env
            .storage()
            .persistent()
            .get::<Symbol, Market>(&market_id)
            .ok_or(Error::MarketNotFound)?;

        let total_votes = market.votes.len() as u32;
        let unique_voters = market.votes.len() as u32;

        // Create voting timeline (simplified - in real implementation would track timestamps)
        let mut voting_timeline = Map::new(env);
        voting_timeline.set(0, total_votes); // Placeholder

        // Calculate outcome preferences
        let mut outcome_preferences = Map::new(env);
        for (_, outcome) in market.votes.iter() {
            let count = outcome_preferences.get(outcome.clone()).unwrap_or(0);
            outcome_preferences.set(outcome.clone(), count + 1);
        }

        // Calculate stake concentration
        let mut stake_concentration = Map::new(env);
        for (user, stake) in market.stakes.iter() {
            stake_concentration.set(user, stake);
        }

        // Create voting patterns (simplified)
        let mut voting_patterns = Map::new(env);
        voting_patterns.set(String::from_str(env, "binary"), total_votes);

        // Create participation trends (simplified)
        let participation_trends = vec![env, total_votes];

        // Create consensus evolution (simplified)
        let consensus_evolution = vec![env, Self::calculate_consensus_strength(&market)];

        Ok(VotingAnalytics {
            market_id,
            total_votes,
            unique_voters,
            voting_timeline,
            outcome_preferences,
            stake_concentration,
            voting_patterns,
            participation_trends,
            consensus_evolution,
        })
    }

    /// Get oracle performance statistics for a specific oracle provider
    pub fn get_oracle_performance_stats(
        env: &Env,
        oracle: OracleProvider,
    ) -> Result<OraclePerformanceStats, Error> {
        // In a real implementation, this would query oracle performance data
        // For now, return placeholder data
        let total_requests = 1000;
        let successful_requests = 950;
        let failed_requests = total_requests - successful_requests;
        let average_response_time = 5000; // 5 seconds
        let accuracy_rate = (successful_requests * 100) / total_requests;
        let uptime_percentage = 99;
        let last_update = env.ledger().timestamp();
        let reliability_score = (accuracy_rate + uptime_percentage) / 2;

        let performance_trends = vec![env, 95, 96, 97, 98, 99];

        Ok(OraclePerformanceStats {
            oracle_provider: oracle,
            total_requests,
            successful_requests,
            failed_requests,
            average_response_time,
            accuracy_rate,
            uptime_percentage,
            last_update,
            reliability_score,
            performance_trends,
        })
    }

    /// Get fee analytics for a specific timeframe
    pub fn get_fee_analytics(env: &Env, timeframe: TimeFrame) -> Result<FeeAnalytics, Error> {
        // In a real implementation, this would query fee collection data
        // For now, return placeholder data
        let total_fees_collected = 1000000; // 1 XLM
        let platform_fees = 800000; // 0.8 XLM
        let dispute_fees = 100000; // 0.1 XLM
        let creation_fees = 100000; // 0.1 XLM

        let mut fee_distribution = Map::new(env);
        fee_distribution.set(String::from_str(env, "platform"), platform_fees);
        fee_distribution.set(String::from_str(env, "dispute"), dispute_fees);
        fee_distribution.set(String::from_str(env, "creation"), creation_fees);

        let average_fee_per_market = total_fees_collected / 10; // Assuming 10 markets
        let fee_collection_rate = 95; // 95% collection rate

        let revenue_trends = vec![env, 800000, 850000, 900000, 950000, 1000000];
        let fee_optimization_score = 85;

        Ok(FeeAnalytics {
            timeframe,
            total_fees_collected,
            platform_fees,
            dispute_fees,
            creation_fees,
            fee_distribution,
            average_fee_per_market,
            fee_collection_rate,
            revenue_trends,
            fee_optimization_score,
        })
    }

    /// Get dispute analytics for a specific market
    pub fn get_dispute_analytics(env: &Env, market_id: Symbol) -> Result<DisputeAnalytics, Error> {
        let market = env
            .storage()
            .persistent()
            .get::<Symbol, Market>(&market_id)
            .ok_or(Error::MarketNotFound)?;

        let total_disputes = market.dispute_stakes.len() as u32;
        let resolved_disputes = if market.state == MarketState::Resolved {
            total_disputes
        } else {
            0
        };
        let pending_disputes = total_disputes - resolved_disputes;
        let dispute_stakes = market.total_dispute_stakes();

        let average_resolution_time = 86400; // 1 day in seconds
        let dispute_success_rate = if total_disputes > 0 {
            (resolved_disputes * 100) / total_disputes
        } else {
            0
        };

        let mut dispute_reasons = Map::new(env);
        dispute_reasons.set(String::from_str(env, "oracle_error"), total_disputes);

        let mut resolution_methods = Map::new(env);
        resolution_methods.set(String::from_str(env, "community_vote"), resolved_disputes);

        let dispute_trends = vec![env, total_disputes];

        Ok(DisputeAnalytics {
            market_id,
            total_disputes,
            resolved_disputes,
            pending_disputes,
            dispute_stakes,
            average_resolution_time,
            dispute_success_rate,
            dispute_reasons,
            resolution_methods,
            dispute_trends,
        })
    }

    /// Get participation metrics for a specific market
    pub fn get_participation_metrics(
        env: &Env,
        market_id: Symbol,
    ) -> Result<ParticipationMetrics, Error> {
        let market = env
            .storage()
            .persistent()
            .get::<Symbol, Market>(&market_id)
            .ok_or(Error::MarketNotFound)?;

        let total_participants = market.votes.len() as u32;
        let active_participants = total_participants; // All voters are considered active
        let new_participants = total_participants; // Simplified - would track new vs returning
        let returning_participants = 0; // Simplified

        let participation_rate = if total_participants > 0 {
            (total_participants * 100) / (total_participants + 5) // Placeholder calculation
        } else {
            0
        };

        let engagement_score = Self::calculate_engagement_score(&market);
        let retention_rate = 85; // Placeholder

        let mut participant_demographics = Map::new(env);
        participant_demographics.set(String::from_str(env, "total"), total_participants);

        let mut activity_patterns = Map::new(env);
        activity_patterns.set(String::from_str(env, "voting"), total_participants);

        Ok(ParticipationMetrics {
            market_id,
            total_participants,
            active_participants,
            new_participants,
            returning_participants,
            participation_rate,
            engagement_score,
            retention_rate,
            participant_demographics,
            activity_patterns,
        })
    }

    /// Get market comparison analytics for multiple markets
    pub fn get_market_comparison_analytics(
        env: &Env,
        markets: Vec<Symbol>,
    ) -> Result<MarketComparisonAnalytics, Error> {
        let total_markets = markets.len() as u32;
        let mut total_participation = 0;
        let mut total_stake = 0;
        let mut successful_markets = 0;

        let mut market_performance_ranking = Map::new(env);
        let mut comparative_metrics = Map::new(env);
        let mut market_categories = Map::new(env);

        for (i, market_id) in markets.iter().enumerate() {
            if let Some(market) = env.storage().persistent().get::<Symbol, Market>(&market_id) {
                let participants = market.votes.len() as u32;
                let stake = market.total_staked;

                total_participation += participants;
                total_stake += stake;

                if market.state == MarketState::Resolved {
                    successful_markets += 1;
                }

                // Rank markets by participation
                market_performance_ranking.set(market_id.clone(), participants);

                // Categorize markets (simplified)
                market_categories.set(market_id.clone(), String::from_str(env, "prediction"));
            }
        }

        let average_participation = if total_markets > 0 {
            total_participation / total_markets
        } else {
            0
        };

        let average_stake = if total_markets > 0 {
            total_stake / total_markets as i128
        } else {
            0
        };

        let success_rate = if total_markets > 0 {
            (successful_markets * 100) / total_markets
        } else {
            0
        };

        let resolution_efficiency = 90; // Placeholder

        comparative_metrics.set(
            String::from_str(env, "avg_participation"),
            average_participation as i128,
        );
        comparative_metrics.set(String::from_str(env, "avg_stake"), average_stake);
        comparative_metrics.set(String::from_str(env, "success_rate"), success_rate as i128);

        let performance_insights = vec![
            env,
            String::from_str(env, "High participation markets show better accuracy"),
            String::from_str(env, "Stake distribution affects market stability"),
        ];

        Ok(MarketComparisonAnalytics {
            markets,
            total_markets,
            average_participation,
            average_stake,
            success_rate,
            resolution_efficiency,
            market_performance_ranking,
            comparative_metrics,
            market_categories,
            performance_insights,
        })
    }

    // ===== HELPER FUNCTIONS =====

    /// Calculate market volatility based on stake distribution
    fn calculate_market_volatility(market: &Market) -> u32 {
        if market.votes.len() == 0 {
            return 0;
        }

        let mut stake_values = Vec::new(&market.votes.env());
        for (_, stake) in market.stakes.iter() {
            stake_values.push_back(stake);
        }

        // Simplified volatility calculation
        let total_stake = market.total_staked;
        let average_stake = total_stake / market.votes.len() as i128;

        let mut variance = 0;
        for stake in stake_values.iter() {
            let diff = stake - average_stake;
            variance += diff * diff;
        }

        let volatility = (variance / market.votes.len() as i128) / 1000; // Scale down
        volatility as u32
    }

    /// Calculate consensus strength based on vote distribution
    fn calculate_consensus_strength(market: &Market) -> u32 {
        if market.votes.len() == 0 {
            return 0;
        }

        let mut outcome_counts = Map::new(&market.votes.env());
        for (_, outcome) in market.votes.iter() {
            let count = outcome_counts.get(outcome.clone()).unwrap_or(0);
            outcome_counts.set(outcome.clone(), count + 1);
        }

        let mut max_votes = 0;
        for (_, count) in outcome_counts.iter() {
            if count > max_votes {
                max_votes = count;
            }
        }

        (max_votes * 100) / market.votes.len() as u32
    }

    /// Calculate time to resolution for a market
    fn calculate_time_to_resolution(market: &Market) -> u64 {
        if market.state == MarketState::Resolved || market.state == MarketState::Closed {
            // In a real implementation, would track actual resolution time
            return 86400; // 1 day placeholder
        }
        0
    }

    /// Get resolution method for a market
    fn get_resolution_method(market: &Market) -> String {
        match market.state {
            MarketState::Resolved => {
                if market.oracle_result.is_some() {
                    String::from_str(&market.votes.env(), "oracle")
                } else {
                    String::from_str(&market.votes.env(), "manual")
                }
            }
            _ => String::from_str(&market.votes.env(), "pending"),
        }
    }

    /// Calculate engagement score for a market
    fn calculate_engagement_score(market: &Market) -> u32 {
        let participation = market.votes.len() as u32;
        let stake_ratio = if market.total_staked > 0 {
            (market.total_staked / 1000000) as u32 // Scale down
        } else {
            0
        };

        (participation + stake_ratio) / 2
    }
}
