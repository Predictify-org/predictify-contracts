pub fn resolve_market(env: &Env, market_id: &Symbol) -> Result<MarketResolution, Error> {
    // Create budget guard with 50,000 instruction threshold for resolution path
    // This threshold provides enough budget to complete the current iteration
    // plus the final state updates and event emissions.
    let budget_guard = crate::gas::BudgetGuard::new(env, 50000);

    // Get the market from storage
    let mut market = MarketStateManager::get_market(env, market_id)?;

    // Validate market for resolution (includes min pool size check)
    let validation = MarketResolutionValidator::validate_market_for_resolution(env, &market);
    if let Err(Error::InvalidState) = validation {
        let global_min: i128 = env
            .storage()
            .persistent()
            .get(&Symbol::new(env, "global_min_pool"))
            .unwrap_or(0);
        let min_pool = market.min_pool_size.unwrap_or(global_min);
        crate::events::EventEmitter::emit_min_pool_size_not_met(
            env,
            market_id,
            market.total_staked,
            min_pool,
        );
        return Err(Error::InvalidState);
    }
    validation?;

    // CHECKPOINT 1: Before retrieving oracle result
    budget_guard.check()?;

    // Retrieve the oracle result
    let oracle_result = market
        .oracle_result
        .as_ref()
        .ok_or(Error::OracleUnavailable)?
        .clone();

    // CHECKPOINT 2: Before calculating community consensus
    budget_guard.check()?;

    // Calculate community consensus
    let community_consensus = MarketAnalytics::calculate_community_consensus(&market);

    // CHECKPOINT 3: Before determining winning outcomes
    budget_guard.check()?;

    // Determine winning outcome(s) using multi-outcome resolution with tie detection
    // This handles both single winner and tie cases (pool split)
    let winning_outcomes = MarketUtils::determine_winning_outcomes(
        env,
        &market,
        &oracle_result,
        &community_consensus,
        0, // Tie threshold: 0 = exact ties only
    );

    // For resolution record, use first outcome (or comma-separated for display)
    let final_result = if winning_outcomes.len() > 0 {
        if winning_outcomes.len() == 1 {
            winning_outcomes.get(0).unwrap().clone()
        } else {
            // For ties, just use the first outcome for the final result field
            // The full list is stored in winning_outcomes
            winning_outcomes.get(0).unwrap().clone()
        }
    } else {
        oracle_result.clone()
    };

    // Determine resolution method
    let resolution_method = MarketResolutionAnalytics::determine_resolution_method(
        &oracle_result,
        &community_consensus,
    );

    // Calculate confidence score
    let confidence_score = MarketResolutionAnalytics::calculate_confidence_score(
        &oracle_result,
        &community_consensus,
        &resolution_method,
    );

    // Create market resolution record
    let resolution = MarketResolution {
        market_id: market_id.clone(),
        final_outcome: final_result.clone(),
        oracle_result,
        community_consensus,
        resolution_timestamp: env.ledger().timestamp(),
        resolution_method,
        confidence_score,
    };

    // Capture old state for event
    let old_state = market.state.clone();

    // CHECKPOINT 4: Before updating market state
    budget_guard.check()?;

    // Set winning outcome(s) - supports both single winner and ties
    MarketStateManager::set_winning_outcomes(
        &mut market,
        winning_outcomes.clone(),
        Some(market_id),
    );
    MarketStateManager::update_market(env, market_id, &market);
    ResolutionOutcomeCache::refresh(env, market_id, &market)?;

    // Decrement active event count since the event is resolved
    crate::storage::CreatorLimitsManager::decrement_active_events(env, &market.admin);

    // CHECKPOINT 5: Before emitting events
    budget_guard.check()?;

    // Emit market resolved event
    let oracle_result_str = market
        .oracle_result
        .clone()
        .unwrap_or_else(|| soroban_sdk::String::from_str(env, "N/A"));
    let community_consensus_str = soroban_sdk::String::from_str(env, "Consensus");
    let method_str = match resolution_method {
        ResolutionMethod::OracleOnly => "OracleOnly",
        ResolutionMethod::CommunityOnly => "CommunityOnly",
        ResolutionMethod::Hybrid => "Hybrid",
        ResolutionMethod::AdminOverride => "AdminOverride",
        ResolutionMethod::DisputeResolution => "DisputeResolution",
    };
    let resolution_method_str = soroban_sdk::String::from_str(env, method_str);

    crate::events::EventEmitter::emit_market_resolved(
        env,
        market_id,
        &final_result,
        &oracle_result_str,
        &community_consensus_str,
        &resolution_method_str,
        confidence_score as i128,
    );

    // Emit state change event
    crate::events::EventEmitter::emit_state_change_event(
        env,
        market_id,
        &old_state,
        &crate::types::MarketState::Resolved,
        &soroban_sdk::String::from_str(env, "Automated resolution completed"),
    );
    crate::monitoring::ContractMonitor::emit_resolution_transition_hook(
        env,
        market_id,
        &old_state,
        &crate::types::MarketState::Resolved,
        &resolution_method_str,
    );

    // Final checkpoint before returning
    budget_guard.check()?;

    Ok(resolution)
}