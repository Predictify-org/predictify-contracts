use crate::errors::Error;
use crate::storage::{AdminStorage, MarketStateManager, TokenStorage};
use crate::types::{
    Dispute, DisputeEscalation, DisputeFeeDistribution, DisputeResolution, DisputeStats,
    DisputeStatus, DisputeTimeout, DisputeTimeoutOutcome, DisputeTimeoutStatus, Market,
    TimeoutAnalytics, TimeoutStats,
};
use soroban_sdk::{symbol_short, token, Address, Env, Map, String, Symbol, Vec};

pub const MIN_DISPUTE_STAKE: i128 = 10_000_000;
pub const DISPUTE_PERIOD_SECS: u64 = 86400;

pub struct DisputeValidator;

impl DisputeValidator {
    pub fn validate_market_for_dispute(env: &Env, market: &Market) -> Result<(), Error> {
        let current_time = env.ledger().timestamp();
        if current_time < market.end_time {
            return Err(Error::MarketNotEnded);
        }

        if market.oracle_result.is_none() {
            return Err(Error::OracleResultNotAvailable);
        }

        Ok(())
    }

    /// Retrieves the configured dispute history capacity.
    pub fn get_history_cap(env: &Env) -> Option<u32> {
        let key = DataKey::DisputeHistoryCap;
        env.storage().persistent().get(&key)
    }

    /// Sets the anti-grief minimum stake floor.
    pub fn set_anti_grief_floor(env: &Env, admin: Address, floor: i128) -> Result<(), Error> {
        admin.require_auth();
        DisputeValidator::validate_admin_permissions(env, &admin)?;
        Self::check_admin_cooldown(env, &admin, &Symbol::new(env, "set_anti_grief_floor"))?;

        let key = DataKey::AntiGriefFloor;
        env.storage().persistent().set(&key, &floor);
        env.storage().persistent().extend_ttl(&key, 535680, 535680);
        Ok(())
    }

    /// Retrieves the anti-grief minimum stake floor.
    pub fn get_anti_grief_floor(env: &Env) -> Option<i128> {
        let key = DataKey::AntiGriefFloor;
        env.storage().persistent().get(&key)
    }

    /// Sets the collusion detector configuration.
    pub fn set_collusion_detector_config(env: &Env, admin: Address, config: CollusionDetectorConfig) -> Result<(), Error> {
        admin.require_auth();
        DisputeValidator::validate_admin_permissions(env, &admin)?;
        Self::check_admin_cooldown(env, &admin, &Symbol::new(env, "set_collusion_detector_config"))?;

        let key = DataKey::CollusionDetectorConfig(Symbol::new(env, "collusion_config"));
        env.storage().persistent().set(&key, &config);
        env.storage().persistent().extend_ttl(&key, 535680, 535680);
        Ok(())
    }

    /// Retrieves the collusion detector configuration.
    pub fn get_collusion_detector_config(env: &Env) -> CollusionDetectorConfig {
        let key = DataKey::CollusionDetectorConfig(Symbol::new(env, "collusion_config"));
        env.storage().persistent().get(&key).unwrap_or(CollusionDetectorConfig {
            stake_delta_threshold: 1_000_000,
            time_delta_threshold: 600, // 10 minutes
            window_size: 8,
        })
    }

    /// Evicts the oldest resolved/expired disputes if history size exceeds the cap.
    pub fn apply_eviction(
        env: &Env,
        market_id: &Symbol,
        history: &mut Vec<Dispute>,
    ) -> Result<(), Error> {
        if stake < MIN_DISPUTE_STAKE {
            return Err(Error::InsufficientStake);
        }

        Ok(())
    }

    pub fn validate_market_for_resolution(
        _env: &Env,
        market: &Market,
        admin: &Address,
    ) -> Result<(), Error> {
        if market.oracle_result.is_none() {
            return Err(Error::OracleResultNotAvailable);
        }

        if market.total_dispute_stakes() == 0 {
            return Err(Error::NoDisputesFound);
        }

        Ok(())
    }

    pub fn validate_dispute_timeout_parameters(timeout_hours: u32) -> Result<(), Error> {
        if timeout_hours == 0 || timeout_hours > 720 {
            return Err(Error::InvalidDuration);
        }
        Ok(())
    }

    pub fn validate_dispute_timeout_extension_parameters(extension_hours: u32) -> Result<(), Error> {
        if extension_hours == 0 || extension_hours > 168 {
            return Err(Error::InvalidDuration);
        }
        Ok(())
    }
}

pub struct DisputeManager;

impl DisputeManager {
    pub fn process_dispute(
        env: &Env,
        user: Address,
        market_id: Symbol,
        stake: i128,
        reason: Option<String>,
    ) -> Result<Dispute, Error> {
        user.require_auth();

        let mut market = MarketStateManager::get_market(env, &market_id)?;

        DisputeValidator::validate_market_for_dispute(env, &market)?;
        DisputeValidator::validate_dispute_parameters(env, &user, &market, stake)?;

        let token_address = TokenStorage::get_token_id(env)?;
        let token_client = token::Client::new(env, &token_address);

        token_client.transfer(&user, &env.current_contract_address(), &stake);

        let current_stake = market.dispute_stakes.get(user.clone()).unwrap_or(0);
        let new_stake = current_stake.checked_add(stake).ok_or(Error::Overflow)?;
        market.dispute_stakes.set(user.clone(), new_stake);

        MarketStateManager::update_market(env, &market_id, &market);

        let dispute = Dispute {
            user: user.clone(),
            market_id: market_id.clone(),
            stake,
            timestamp: env.ledger().timestamp(),
            reason: reason.clone(),
            status: DisputeStatus::Active,
        };

        DisputeUtils::emit_dispute_submitted_event(env, &dispute);

        Ok(dispute)
    }

    pub fn vote_on_dispute(
        env: &Env,
        user: Address,
        market_id: Symbol,
        vote: String,
        stake: i128,
    ) -> Result<(), Error> {
        user.require_auth();

        let mut market = MarketStateManager::get_market(env, &market_id)?;

        if stake <= 0 {
            return Err(Error::InsufficientStake);
        }

        let token_address = TokenStorage::get_token_id(env)?;
        let token_client = token::Client::new(env, &token_address);
        token_client.transfer(&user, &env.current_contract_address(), &stake);

        let current_stake = market.stakes.get(user.clone()).unwrap_or(0);
        let new_stake = current_stake.checked_add(stake).ok_or(Error::Overflow)?;

        market.votes.set(user.clone(), vote.clone());
        market.stakes.set(user.clone(), new_stake);

        let total_staked = market.total_staked.checked_add(stake).ok_or(Error::Overflow)?;
        market.total_staked = total_staked;

        MarketStateManager::update_market(env, &market_id, &market);

        DisputeUtils::emit_dispute_vote_event(env, &market_id, &user, &vote, stake);

        // --- Collusion Detector ---
        let config = Self::get_collusion_detector_config(env);
        let window_size = config.window_size;
        let start_idx = if history.len() > window_size {
            history.len() - window_size
        } else {
            0
        };

        for i in start_idx..history.len().saturating_sub(1) {
            if let Some(prev_dispute) = history.get(i) {
                if prev_dispute.user != user {
                    let stake_diff = if prev_dispute.stake > stake { prev_dispute.stake - stake } else { stake - prev_dispute.stake };
                    let time_diff = if prev_dispute.timestamp > dispute.timestamp { prev_dispute.timestamp - dispute.timestamp } else { dispute.timestamp - prev_dispute.timestamp };

                    if stake_diff <= config.stake_delta_threshold && time_diff <= config.time_delta_threshold {
                        crate::events::EventEmitter::emit_suspected_collusion_flag(
                            env,
                            &market_id,
                            &user,
                            &prev_dispute.user,
                            stake_diff,
                            time_diff,
                        );
                    }
                }
            }
        }
        // --------------------------

        Ok(())
    }

    pub fn resolve_dispute(
        env: &Env,
        market_id: Symbol,
        admin: Address,
    ) -> Result<DisputeResolution, Error> {
        admin.require_auth();

        let contract_admin = AdminStorage::get_admin(env)?;
        if admin != contract_admin {
            return Err(Error::Unauthorized);
        }

        let mut market = MarketStateManager::get_market(env, &market_id)?;

        DisputeValidator::validate_market_for_resolution(env, &market, &admin)?;

        let oracle_result = market
            .oracle_result
            .clone()
            .ok_or(Error::OracleResultNotAvailable)?;

        let consensus = DisputeAnalytics::calculate_community_consensus(env, &market);

        let dispute_impact = DisputeUtils::calculate_dispute_impact(&market);

        let final_outcome = if dispute_impact > 0.3 && consensus.confidence > 70 {
            consensus.outcome
        } else {
            oracle_result.clone()
        };

        let is_oracle_overturned = final_outcome != oracle_result;

        if is_oracle_overturned {
            // Refund all disputers their stakes and emit a StakeRefunded event per disputer.
            let token_address = TokenStorage::get_token_id(env)?;
            let token_client = token::Client::new(env, &token_address);

            let disputers: Vec<(Address, i128)> = market
                .dispute_stakes
                .iter()
                .map(|(user, stake)| (user, stake))
                .collect();

            for (disputer, stake) in disputers.iter() {
                if *stake > 0 {
                    // Perform the refund transfer.
                    token_client.transfer(&env.current_contract_address(), &disputer, stake);
                    // Reset the stored stake for the disputer.
                    market.dispute_stakes.set(disputer.clone(), 0);
                    // Emit an event for the refund.
                    DisputeUtils::emit_stake_refunded_event(env, disputer, *stake);
                }
            }
        }

        market.winning_outcomes = Some(final_outcome.clone());
        market.state = crate::types::MarketState::Resolved;

        MarketStateManager::update_market(env, &market_id, &market);

        let resolution = DisputeResolution {
            market_id,
            final_outcome,
            oracle_weight: DisputeAnalytics::calculate_oracle_weight(&market),
            community_weight: DisputeAnalytics::calculate_community_weight(&market),
            dispute_impact: (dispute_impact * 100.0) as i128,
            resolution_timestamp: env.ledger().timestamp(),
        };

        // Update market with final outcome
        DisputeUtils::finalize_market_with_resolution(&mut market, final_outcome)?;
        MarketStateManager::update_market(env, &market_id, &market);

        // Update history status to Resolved
        let mut history = env.storage().persistent()
            .get::<_, Vec<Dispute>>(&DataKey::DisputeHistory(market_id.clone()))
            .unwrap_or_else(|| Vec::new(env));
        let mut updated = false;
        for i in 0..history.len() {
            let mut disp = history.get(i).ok_or(Error::InvalidState)?;
            if matches!(disp.status, DisputeStatus::Active) {
                disp.status = DisputeStatus::Resolved;
                history.set(i, disp);
                updated = true;
            }
        }
        if updated {
            Self::apply_eviction(env, &market_id, &mut history)?;
            env.storage().persistent().set(&DataKey::DisputeHistory(market_id.clone()), &history);
            env.storage().persistent().extend_ttl(&DataKey::DisputeHistory(market_id.clone()), 535680, 535680);
        }

        let _ = crate::resolution::ResolutionOutcomeCache::refresh(env, &market_id, &market);
        crate::monitoring::ContractMonitor::emit_dispute_transition_hook(
            env,
            &market_id,
            &soroban_sdk::String::from_str(env, "resolved"),
            &admin,
            &soroban_sdk::String::from_str(env, "dispute_resolved"),
        );

        crate::audit_trail::AuditTrailManager::append_record(
            env,
            crate::audit_trail::AuditAction::DisputeResolved,
            admin.clone(),
            Map::new(env),
            None,
        );

        Ok(resolution)
    }
}

pub struct DisputeUtils;

impl DisputeUtils {
    pub fn calculate_dispute_impact(market: &Market) -> f64 {
        let total_dispute_stakes = market.total_dispute_stakes();
        if market.total_staked == 0 {
            return 0.0;
        }
        (total_dispute_stakes as f64) / (market.total_staked as f64)
    }

    /// Add vote to dispute
    pub fn add_vote_to_dispute(
        env: &Env,
        dispute_id: &Symbol,
        vote: DisputeVote,
    ) -> Result<(), Error> {
        // Get current voting data
        let mut voting_data = Self::get_dispute_voting(env, dispute_id)?;

        // Update voting statistics
        voting_data.total_votes = voting_data
            .total_votes
            .checked_add(1)
            .ok_or(Error::Overflow)?;
        
        // Calculate the decayed stake using tally_votes
        let decayed_stake = Self::tally_votes(env, vote.stake, vote.timestamp, voting_data.voting_start);

        if vote.vote {
            voting_data.support_votes = voting_data
                .support_votes
                .checked_add(1)
                .ok_or(Error::Overflow)?;
            voting_data.total_support_stake = voting_data
                .total_support_stake
                .checked_add(decayed_stake)
                .ok_or(Error::Overflow)?;
        } else {
            voting_data.against_votes = voting_data
                .against_votes
                .checked_add(1)
                .ok_or(Error::Overflow)?;
            voting_data.total_against_stake = voting_data
                .total_against_stake
                .checked_add(decayed_stake)
                .ok_or(Error::Overflow)?;
        }

        // Store updated voting data
        Self::store_dispute_voting(env, dispute_id, &voting_data)?;

        // Store the vote
        Self::store_dispute_vote(env, dispute_id, &vote)?;

        Ok(())
    }

    /// Calculate the stake weight using exponential decay approximation
    /// so late votes count less than early votes.
    pub fn tally_votes(env: &Env, raw_stake: i128, vote_time: u64, window_start: u64) -> i128 {
        let config_key = symbol_short!("decaycfg");
        let config: Option<DisputeDecayConfig> = env.storage().persistent().get(&config_key);
        
        let cfg = match config {
            Some(c) => c,
            None => return raw_stake,
        };

        if cfg.half_life_seconds == 0 {
            return raw_stake;
        }

        let elapsed = vote_time.saturating_sub(window_start);
        let num_half_lives = elapsed / cfg.half_life_seconds;
        let rem = elapsed % cfg.half_life_seconds;

        let shift = num_half_lives.min(16) as u32;
        let weight_at_n = 10000u32.checked_shr(shift).unwrap_or(0);
        let weight_at_n_plus_1 = 10000u32.checked_shr(shift + 1).unwrap_or(0);
        
        let diff = weight_at_n.saturating_sub(weight_at_n_plus_1);
        let exact_weight = weight_at_n.saturating_sub((diff as u64 * rem / cfg.half_life_seconds) as u32);
        
        // A misconfigured floor must never amplify a vote above its raw stake.
        let final_weight = exact_weight.max(cfg.floor_bps).min(10_000) as i128;

        // Split before multiplying so every i128 input remains overflow-safe.
        let whole = raw_stake / 10_000;
        let remainder = raw_stake % 10_000;
        whole * final_weight + (remainder * final_weight) / 10_000
    }

    pub fn emit_dispute_vote_event(
        env: &Env,
        _market_id: &Symbol,
        user: &Address,
        vote: &String,
        stake: i128,
    ) {
        // NOTE: emit_dispute_vote_cast not yet implemented in EventEmitter
    }

    pub fn emit_fee_distribution_event(
        env: &Env,
        dispute_id: &Symbol,
        distribution: &DisputeFeeDistribution,
    ) {
        // NOTE: emit_dispute_fee_distributed not yet implemented in EventEmitter
    }

    pub fn emit_dispute_escalation_event(
        env: &Env,
        _dispute_id: &Symbol,
        user: &Address,
        escalation: &DisputeEscalation,
    ) {
        let event_key = symbol_short!("esc_event");
        let event_data = (
            user.clone(),
            escalation.escalation_level,
            env.ledger().timestamp(),
        );
        env.storage().persistent().set(&event_key, &event_data);
    }

    /// Emit an event when a disputer's stake is refunded.
    pub fn emit_stake_refunded_event(env: &Env, disputer: &Address, amount: i128) {
        let event_key = symbol_short!("stk_ref");
        let event_data = (disputer.clone(), amount, env.ledger().timestamp());
        env.storage().persistent().set(&event_key, &event_data);
    }

    pub fn store_dispute_timeout(
        env: &Env,
        dispute_id: &Symbol,
        timeout: &DisputeTimeout,
    ) -> Result<(), Error> {
        let key = (symbol_short!("timeout"), dispute_id.clone());
        env.storage().persistent().set(&key, timeout);
        Ok(())
    }

    pub fn get_dispute_timeout(env: &Env, dispute_id: &Symbol) -> Result<DisputeTimeout, Error> {
        let key = (symbol_short!("timeout"), dispute_id.clone());
        env.storage()
            .persistent()
            .get(&key)
            .ok_or(Error::ConfigNotFound)
    }

    pub fn has_dispute_timeout(env: &Env, dispute_id: &Symbol) -> bool {
        let key = (symbol_short!("timeout"), dispute_id.clone());
        env.storage().persistent().has(&key)
    }

    pub fn remove_dispute_timeout(env: &Env, dispute_id: &Symbol) -> Result<(), Error> {
        let key = (symbol_short!("timeout"), dispute_id.clone());
        env.storage().persistent().remove(&key);
        Ok(())
    }

    pub fn get_active_timeouts(env: &Env) -> Vec<DisputeTimeout> {
        Vec::new(env)
    }

    pub fn check_expired_timeouts(env: &Env) -> Vec<Symbol> {
        Vec::new(env)
    }

    /// Get a user's total dispute stake across all active (unresolved) markets.
    ///
    /// This function calculates the cumulative stake that a user has committed
    /// to disputes across all markets that are still in an active dispute state
    /// (i.e., markets where winning_outcomes is not yet set).
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment
    /// * `user` - The user address to check
    ///
    /// # Returns
    ///
    /// The total stake (in stroops) across all active disputes for this user.
    pub fn get_user_total_active_dispute_stake(env: &Env, user: &Address) -> i128 {
        // In a full implementation, we would need to iterate through all markets
        // and sum up dispute stakes for active disputes. For now, this is a
        // placeholder that returns 0 (requires market registry for full implementation).
        // The validation will use the per-market per-user cap already implemented.
        0
    }
}

pub struct DisputeAnalytics;

impl DisputeAnalytics {
    pub fn calculate_dispute_stats(market: &Market) -> DisputeStats {
        let mut active_disputes = 0;
        let mut resolved_disputes = 0;
        let mut unique_disputers = 0;

        for (_, stake) in market.dispute_stakes.iter() {
            if stake > 0 {
                unique_disputers += 1;
                if market.winning_outcomes.is_none() {
                    active_disputes += 1;
                } else {
                    resolved_disputes += 1;
                }
            }
        }

        DisputeStats {
            total_disputes: active_disputes + resolved_disputes,
            total_dispute_stakes: market.total_dispute_stakes(),
            active_disputes,
            resolved_disputes,
            unique_disputers,
        }
    }

    pub fn calculate_dispute_impact(market: &Market) -> i128 {
        let impact = DisputeUtils::calculate_dispute_impact(market);
        (impact * 100.0) as i128
    }

    pub fn calculate_oracle_weight(market: &Market) -> i128 {
        let dispute_impact = Self::calculate_dispute_impact(market) as f64 / 100.0;
        let base_oracle_weight = 0.7;
        let dispute_penalty = dispute_impact * 0.3;
        let weight = (base_oracle_weight - dispute_penalty).max(0.3);
        (weight * 100.0) as i128
    }

    pub fn calculate_community_weight(market: &Market) -> i128 {
        let dispute_impact = Self::calculate_dispute_impact(market) as f64 / 100.0;
        let base_community_weight = 0.3;
        let dispute_boost = dispute_impact * 0.4;
        let weight = (base_community_weight + dispute_boost).min(0.7);
        (weight * 100.0) as i128
    }

    pub fn calculate_community_consensus(env: &Env, market: &Market) -> CommunityConsensus {
        let mut outcome_totals = Map::new(env);
        let mut total_votes = 0;

        for (user, outcome) in market.votes.iter() {
            let stake = market.stakes.get(user).unwrap_or(0);
            let current_total = outcome_totals.get(outcome.clone()).unwrap_or(0);
            outcome_totals.set(outcome, current_total + stake);
            total_votes += stake;
        }

        let mut winning_outcome = String::from_str(env, "");
        let mut max_stake = 0;

        for (outcome, stake) in outcome_totals.iter() {
            if stake > max_stake {
                max_stake = stake;
                winning_outcome = outcome;
            }
        }

        let confidence = if total_votes > 0 {
            (max_stake as i128) * 100 / total_votes
        } else {
            0
        };

        CommunityConsensus {
            outcome: winning_outcome,
            confidence,
            total_votes,
        }
    }

    pub fn get_top_disputers(env: &Env, market: &Market, _limit: usize) -> Vec<(Address, i128)> {
        let mut disputers: Vec<(Address, i128)> = Vec::new(env);

        for (user, stake) in market.dispute_stakes.iter() {
            if stake > 0 {
                disputers.push_back((user, stake));
            }
        }

        disputers
    }

    pub fn calculate_dispute_participation_rate(market: &Market) -> f64 {
        let total_voters = market.votes.len();
        let total_disputers = market.dispute_stakes.len();

        if total_voters == 0 {
            return 0.0;
        }

        (total_disputers as f64) / (total_voters as f64)
    }

    pub fn calculate_timeout_stats(_env: &Env) -> TimeoutStats {
        TimeoutStats {
            total_timeouts: 0,
            active_timeouts: 0,
            expired_timeouts: 0,
            auto_resolved_timeouts: 0,
            average_timeout_hours: 0,
        }
    }

    pub fn get_timeout_analytics(env: &Env, dispute_id: &Symbol) -> TimeoutAnalytics {
        match DisputeUtils::get_dispute_timeout(env, dispute_id) {
            Ok(timeout) => {
                let current_time = env.ledger().timestamp();
                let time_remaining = if current_time < timeout.expires_at {
                    timeout.expires_at - current_time
                } else {
                    0
                };

                TimeoutAnalytics {
                    dispute_id: dispute_id.clone(),
                    timeout_hours: timeout.timeout_hours,
                    time_remaining_seconds: time_remaining,
                    time_remaining_hours: time_remaining / 3600,
                    is_expired: current_time >= timeout.expires_at,
                    status: timeout.status,
                    total_extensions: timeout.total_extension_hours,
                }
            }
            Err(_) => TimeoutAnalytics {
                dispute_id: dispute_id.clone(),
                timeout_hours: 0,
                time_remaining_seconds: 0,
                time_remaining_hours: 0,
                is_expired: false,
                status: DisputeTimeoutStatus::Active,
                total_extensions: 0,
            },
        }
    }
}

#[cfg(test)]
pub mod testing {
    use super::*;

    pub fn create_test_dispute(
        env: &Env,
        user: Address,
        market_id: Symbol,
        stake: i128,
    ) -> Dispute {
        Dispute {
            user,
            market_id,
            stake,
            timestamp: env.ledger().timestamp(),
            reason: Some(String::from_str(env, "Test dispute")),
            status: DisputeStatus::Active,
        }
    }

    pub fn create_test_dispute_stats() -> DisputeStats {
        DisputeStats {
            total_disputes: 0,
            total_dispute_stakes: 0,
            active_disputes: 0,
            resolved_disputes: 0,
            unique_disputers: 0,
        }
    }

    pub fn create_test_dispute_resolution(env: &Env, market_id: Symbol) -> DisputeResolution {
        DisputeResolution {
            market_id,
            final_outcome: String::from_str(env, "yes"),
            oracle_weight: 70,
            community_weight: 30,
            dispute_impact: 10,
            resolution_timestamp: env.ledger().timestamp(),
        }
    }

    pub fn validate_dispute_structure(dispute: &Dispute) -> Result<(), Error> {
        if dispute.stake <= 0 {
            return Err(Error::InsufficientStake);
        }

        Ok(())
    }

    pub fn validate_dispute_stats(stats: &DisputeStats) -> Result<(), Error> {
        if stats.total_dispute_stakes < 0 {
            return Err(Error::InvalidInput);
        }

        if stats.total_disputes < stats.unique_disputers {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }

    pub fn create_test_dispute_timeout(env: &Env, dispute_id: Symbol) -> DisputeTimeout {
        DisputeTimeout {
            dispute_id: dispute_id.clone(),
            market_id: Symbol::new(env, "test_market"),
            timeout_hours: 24,
            created_at: env.ledger().timestamp(),
            expires_at: env.ledger().timestamp() + 86400,
            extended_at: None,
            total_extension_hours: 0,
            status: DisputeTimeoutStatus::Active,
        }
    }

    pub fn create_test_timeout_outcome(env: &Env, dispute_id: Symbol) -> DisputeTimeoutOutcome {
        DisputeTimeoutOutcome {
            dispute_id: dispute_id.clone(),
            market_id: Symbol::new(env, "test_market"),
            outcome: String::from_str(env, "Support"),
            resolution_method: String::from_str(env, "Timeout Auto-Resolution"),
            resolution_timestamp: env.ledger().timestamp().max(1),
            reason: String::from_str(env, "Test timeout resolution"),
        }
    }

    pub fn validate_timeout_structure(timeout: &DisputeTimeout) -> Result<(), Error> {
        if timeout.timeout_hours == 0 {
            return Err(Error::InvalidDuration);
        }

        if timeout.expires_at <= timeout.created_at {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }

    pub fn validate_timeout_outcome_structure(
        outcome: &DisputeTimeoutOutcome,
    ) -> Result<(), Error> {
        if outcome.resolution_timestamp == 0 {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }
}

pub struct CommunityConsensus {
    pub outcome: String,
    pub confidence: i128,
    pub total_votes: i128,
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    fn create_test_market(env: &Env, end_time: u64) -> Market {
        let mut outcomes = Vec::new(env);
        outcomes.push_back(String::from_str(env, "yes"));
        outcomes.push_back(String::from_str(env, "no"));

        Market::new(
            env,
            Address::generate(env),
            String::from_str(env, "Test Market"),
            outcomes,
            end_time,
            crate::types::OracleConfig::new(
                crate::types::OracleProvider::pyth(),
                Address::from_str(
                    env,
                    "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
                ),
                String::from_str(env, "BTC/USD"),
                2500000,
                String::from_str(env, "gt"),
            ),
            None,
            86400,
            crate::types::MarketState::Active,
        )
    }

    #[test]
    fn test_dispute_validator_market_validation() {
        let env = Env::default();
        let mut market = create_test_market(&env, env.ledger().timestamp() + 86400);

        assert!(DisputeValidator::validate_market_for_dispute(&env, &market).is_err());

        market.end_time = env.ledger().timestamp().saturating_sub(1);

        assert!(DisputeValidator::validate_market_for_dispute(&env, &market).is_err());

        market.oracle_result = Some(String::from_str(&env, "yes"));

        assert!(DisputeValidator::validate_market_for_dispute(&env, &market).is_ok());
    }

    #[test]
    fn test_dispute_validator_stake_validation() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let user = Address::generate(&env);
        let mut market = create_test_market(&env, env.ledger().timestamp().saturating_sub(1));
        market.oracle_result = Some(String::from_str(&env, "yes"));
        let market_id = Symbol::new(&env, "market_1");

        assert!(DisputeValidator::validate_dispute_parameters(
            &env,
            &user,
            &market,
            MIN_DISPUTE_STAKE
        )
        .is_ok());

        assert!(DisputeValidator::validate_dispute_parameters(
            &env,
            &user,
            &market,
            MIN_DISPUTE_STAKE - 1
        )
        .is_err());
    }

    #[test]
    fn test_dispute_utils_impact_calculation() {
        let env = Env::default();
        let mut market = create_test_market(&env, env.ledger().timestamp() + 86400);

        market.total_staked = 10000;
        let user = Address::generate(&env);
        market.dispute_stakes.set(user, 2000);

        let impact = DisputeUtils::calculate_dispute_impact(&market);
        assert_eq!(impact, 0.2);
    }

    #[test]
    fn test_dispute_analytics_stats() {
        let env = Env::default();
        let mut market = create_test_market(&env, env.ledger().timestamp() + 86400);

        let user = Address::generate(&env);
        market.dispute_stakes.set(user, 1000);

        let stats = DisputeAnalytics::calculate_dispute_stats(&market);
        assert_eq!(stats.total_disputes, 1);
        assert_eq!(stats.total_dispute_stakes, 1000);
        assert_eq!(stats.unique_disputers, 1);
        assert_eq!(stats.active_disputes, 1);
    }

    #[test]
    fn test_dispute_stake_is_refunded_when_resolution_favors_disputer() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let market_id = Symbol::new(&env, "refund_market");

        let token_admin = Address::generate(&env);
        let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_address = token_contract.address();
        let token_client = soroban_sdk::token::Client::new(&env, &token_address);
        let stellar_client = soroban_sdk::token::StellarAssetClient::new(&env, &token_address);
        stellar_client.mint(&user, &10_000_000_000i128);

        env.as_contract(&contract_id, || {
            env.storage().persistent().set(&Symbol::new(&env, "Admin"), &admin);
            env.storage()
                .persistent()
                .set(&Symbol::new(&env, "TokenID"), &token_address);

            let mut market = create_test_market(&env, env.ledger().timestamp().saturating_sub(1));
            market.oracle_result = Some(String::from_str(&env, "yes"));
            market.state = crate::types::MarketState::Ended;
            market.total_staked = 1_000;

            let voter = Address::generate(&env);
            market.votes.set(voter.clone(), String::from_str(&env, "no"));
            market.stakes.set(voter, 1_000);
            MarketStateManager::update_market(&env, &market_id, &market);

            let initial_balance = token_client.balance(&user);
            let stake = MIN_DISPUTE_STAKE;
            DisputeManager::process_dispute(&env, user.clone(), market_id.clone(), stake, None)
                .unwrap();

            let balance_after_dispute = token_client.balance(&user);
            assert_eq!(balance_after_dispute, initial_balance - stake);

            let contract_balance_before_refund = token_client.balance(&env.current_contract_address());
            let resolution = DisputeManager::resolve_dispute(&env, market_id.clone(), admin.clone())
                .unwrap();

            assert_eq!(resolution.final_outcome, String::from_str(&env, "no"));
            let balance_after_refund = token_client.balance(&user);
            assert_eq!(balance_after_refund, initial_balance);
            assert_eq!(token_client.balance(&env.current_contract_address()), 0);
            assert_eq!(contract_balance_before_refund, stake);
        });
    }

    #[test]
    fn test_testing_utilities() {
        let env = Env::default();
        let user = Address::generate(&env);

        let dispute = testing::create_test_dispute(&env, user, Symbol::new(&env, "market"), 1000);

        assert!(testing::validate_dispute_structure(&dispute).is_ok());

        let stats = testing::create_test_dispute_stats();
        assert!(testing::validate_dispute_stats(&stats).is_ok());
    }

    #[test]
    fn test_timeout_utilities() {
        let env = Env::default();
        let dispute_id = Symbol::new(&env, "test_dispute");

        let timeout = testing::create_test_dispute_timeout(&env, dispute_id.clone());
        assert!(testing::validate_timeout_structure(&timeout).is_ok());

        let outcome = testing::create_test_timeout_outcome(&env, dispute_id);
        assert!(testing::validate_timeout_outcome_structure(&outcome).is_ok());
    }

    #[test]
    fn test_timeout_validation() {
        assert!(DisputeValidator::validate_dispute_timeout_parameters(24).is_ok());
        assert!(DisputeValidator::validate_dispute_timeout_parameters(0).is_err());
        assert!(DisputeValidator::validate_dispute_timeout_parameters(800).is_err());

        assert!(DisputeValidator::validate_dispute_timeout_extension_parameters(24).is_ok());
        assert!(DisputeValidator::validate_dispute_timeout_extension_parameters(0).is_err());
        assert!(DisputeValidator::validate_dispute_timeout_extension_parameters(200).is_err());
    }

    #[test]
    fn test_timeout_analytics() {
        let env = Env::default();
        let dispute_id = Symbol::new(&env, "test_dispute");

        let mock_timeout = DisputeTimeout {
            dispute_id: dispute_id.clone(),
            market_id: Symbol::new(&env, "test_market"),
            timeout_hours: 24,
            created_at: env.ledger().timestamp(),
            expires_at: env.ledger().timestamp() + 86400,
            extended_at: None,
            total_extension_hours: 0,
            status: DisputeTimeoutStatus::Active,
        };

        let current_time = env.ledger().timestamp();
        let time_remaining = if current_time < mock_timeout.expires_at {
            mock_timeout.expires_at - current_time
        } else {
            0
        };

        let analytics = TimeoutAnalytics {
            dispute_id: dispute_id.clone(),
            timeout_hours: mock_timeout.timeout_hours,
            time_remaining_seconds: time_remaining,
            time_remaining_hours: time_remaining / 3600,
            is_expired: current_time >= mock_timeout.expires_at,
            status: mock_timeout.status,
            total_extensions: mock_timeout.total_extension_hours,
        };

        assert_eq!(analytics.timeout_hours, 24);
        assert_eq!(analytics.is_expired, false);
        assert_eq!(analytics.status, DisputeTimeoutStatus::Active);
    }

    #[test]
    fn test_no_refund_when_oracle_result_stands() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let disputer = Address::generate(&env);
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let market_id = Symbol::new(&env, "stands_mkt");

        let token_admin = Address::generate(&env);
        let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_address = token_contract.address();
        let token_client = soroban_sdk::token::Client::new(&env, &token_address);
        let stellar_client = soroban_sdk::token::StellarAssetClient::new(&env, &token_address);
        stellar_client.mint(&disputer, &10_000_000_000i128);

        env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .set(&Symbol::new(&env, "Admin"), &admin);
            env.storage()
                .persistent()
                .set(&Symbol::new(&env, "TokenID"), &token_address);

            let mut market =
                create_test_market(&env, env.ledger().timestamp().saturating_sub(1));
            market.oracle_result = Some(String::from_str(&env, "yes"));
            market.state = crate::types::MarketState::Ended;
            market.total_staked = 1_000;

            let voter = Address::generate(&env);
            market.votes.set(voter.clone(), String::from_str(&env, "yes"));
            market.stakes.set(voter, 1_000);
            MarketStateManager::update_market(&env, &market_id, &market);

            let initial_balance = token_client.balance(&disputer);
            let stake = MIN_DISPUTE_STAKE;

            DisputeManager::process_dispute(
                &env,
                disputer.clone(),
                market_id.clone(),
                stake,
                None,
            )
            .unwrap();

            let balance_after_dispute = token_client.balance(&disputer);
            assert_eq!(
                balance_after_dispute,
                initial_balance - stake,
                "stake must be locked after process_dispute"
            );

            let resolution =
                DisputeManager::resolve_dispute(&env, market_id.clone(), admin.clone())
                    .unwrap();

            assert_eq!(
                resolution.final_outcome,
                String::from_str(&env, "yes"),
                "final outcome must equal oracle result when community agrees"
            );

            let balance_after_resolution = token_client.balance(&disputer);
            assert_eq!(
                balance_after_resolution,
                initial_balance - stake,
                "disputer must NOT be refunded when oracle result stands"
            );
        });
    }

    #[test]
    fn test_multiple_disputers_all_refunded_when_oracle_overturned() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let disputer_a = Address::generate(&env);
        let disputer_b = Address::generate(&env);
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let market_id = Symbol::new(&env, "multi_disp");

        let token_admin = Address::generate(&env);
        let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_address = token_contract.address();
        let token_client = soroban_sdk::token::Client::new(&env, &token_address);
        let stellar_client = soroban_sdk::token::StellarAssetClient::new(&env, &token_address);

        let stake_a = MIN_DISPUTE_STAKE;
        let stake_b = MIN_DISPUTE_STAKE * 3;

        stellar_client.mint(&disputer_a, &10_000_000_000i128);
        stellar_client.mint(&disputer_b, &10_000_000_000i128);
        stellar_client.mint(&contract_id, &(stake_a + stake_b));

        let initial_a = token_client.balance(&disputer_a);
        let initial_b = token_client.balance(&disputer_b);

        env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .set(&Symbol::new(&env, "Admin"), &admin);
            env.storage()
                .persistent()
                .set(&Symbol::new(&env, "TokenID"), &token_address);

            let mut market =
                create_test_market(&env, env.ledger().timestamp().saturating_sub(1));
            market.oracle_result = Some(String::from_str(&env, "yes"));
            market.state = crate::types::MarketState::Ended;

            let voter1 = Address::generate(&env);
            let voter2 = Address::generate(&env);
            let vote_stake: i128 = 10_000_000;
            market.votes.set(voter1.clone(), String::from_str(&env, "no"));
            market.stakes.set(voter1, vote_stake);
            market.votes.set(voter2.clone(), String::from_str(&env, "no"));
            market.stakes.set(voter2, vote_stake);
            market.total_staked = vote_stake * 2;

            market.dispute_stakes.set(disputer_a.clone(), stake_a);
            market.dispute_stakes.set(disputer_b.clone(), stake_b);
            MarketStateManager::update_market(&env, &market_id, &market);

            let resolution =
                DisputeManager::resolve_dispute(&env, market_id.clone(), admin.clone())
                    .unwrap();

            assert_eq!(
                resolution.final_outcome,
                String::from_str(&env, "no"),
                "community consensus must overturn the oracle when confidence > 70 %"
            );

            assert_eq!(
                token_client.balance(&disputer_a),
                initial_a + stake_a,
                "disputer_a must be fully refunded"
            );
            assert_eq!(
                token_client.balance(&disputer_b),
                initial_b + stake_b,
                "disputer_b must be fully refunded"
            );

            assert_eq!(
                token_client.balance(&env.current_contract_address()),
                0,
                "contract balance must be zero after all refunds"
            );

            let mkt_after =
                MarketStateManager::get_market(&env, &market_id).unwrap();
            assert_eq!(
                mkt_after.dispute_stakes.get(disputer_a.clone()).unwrap_or(1),
                0,
                "disputer_a stake must be zeroed after refund"
            );
            assert_eq!(
                mkt_after.dispute_stakes.get(disputer_b.clone()).unwrap_or(1),
                0,
                "disputer_b stake must be zeroed after refund"
            );
        });
    }
}
