use soroban_sdk::{Env, Map, String, Symbol, Vec};
use crate::types::{Market, MarketState, ActiveEvent, PlatformStats, EventSnapshot};
use crate::errors::Error;
use crate::queries::QueryManager;

/// Reporting and Analytics Manager for Predictify Hybrid.
///
/// Provides read-only APIs for retrieving state snapshots of events and platform metrics.
/// All functions are designed to be gas-efficient and secure, exposing no private data.
pub struct ReportingManager;

impl ReportingManager {
    /// Retrieve a list of active events with basic stats.
    ///
    /// Supports pagination to ensure bounded result size and gas efficiency.
    ///
    /// # Parameters
    /// * `env` - The Soroban environment.
    /// * `offset` - Number of active events to skip.
    /// * `limit` - Maximum number of active events to return.
    pub fn get_active_events(env: &Env, offset: u32, limit: u32) -> Result<Vec<ActiveEvent>, Error> {
        let all_markets = QueryManager::get_all_markets(env)?;
        let mut active_events = Vec::new(env);
        let mut skipped = 0;
        let mut added = 0;

        for id in all_markets.iter() {
            let market: Market = env.storage().persistent().get(&id).ok_or(Error::MarketNotFound)?;
            if market.state == MarketState::Active {
                if skipped >= offset {
                    active_events.push_back(ActiveEvent {
                        id: id.clone(),
                        question: market.question.clone(),
                        end_time: market.end_time,
                        total_pool: market.total_staked,
                    });
                    added += 1;
                } else {
                    skipped += 1;
                }
            }
            if added >= limit {
                break;
            }
        }
        Ok(active_events)
    }

    /// Retrieve global platform statistics and metrics.
    pub fn get_platform_stats(env: &Env) -> Result<PlatformStats, Error> {
        let contract_state = QueryManager::query_contract_state(env)?;
        
        Ok(PlatformStats {
            total_active_events: contract_state.active_markets,
            total_resolved_events: contract_state.resolved_markets,
            total_pool_all_events: contract_state.total_value_locked,
            total_fees_collected: contract_state.total_fees_collected,
            version: contract_state.contract_version,
        })
    }

    /// Retrieve a detailed snapshot of a specific event.
    ///
    /// # Parameters
    /// * `env` - The Soroban environment.
    /// * `id` - Unique identifier of the event to snapshot.
    pub fn get_event_snapshot(env: &Env, id: Symbol) -> Result<EventSnapshot, Error> {
        let market: Market = env.storage().persistent().get(&id).ok_or(Error::MarketNotFound)?;
        let pool_query = QueryManager::query_market_pool(env, id.clone())?;
        
        Ok(EventSnapshot {
            id,
            question: market.question,
            outcomes: market.outcomes,
            state: market.state,
            total_pool: market.total_staked,
            outcome_pools: pool_query.outcome_pools,
            participant_count: market.votes.len(),
            end_time: market.end_time,
        })
    }
}
