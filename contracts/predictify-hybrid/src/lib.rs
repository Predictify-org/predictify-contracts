pub fn distribute_payouts(env: Env, market_id: soroban_sdk::Symbol) -> Result<i128, Error> {
    if let Err(e) = crate::circuit_breaker::CircuitBreaker::require_write_allowed(
        &env,
        "distribute_payouts",
    ) {
        return Err(e);
    }
    let mut market: Market = env
        .storage()
        .persistent()
        .get(&market_id)
        .unwrap_or_else(|| {
            panic_with_error!(env, Error::MarketNotFound);
        });

    // Check if market is resolved
    let winning_outcomes = match &market.winning_outcomes {
        Some(outcomes) => outcomes,
        None => return Err(Error::MarketNotResolved),
    };

    // Get all bettors
    let bettors = bets::BetStorage::get_all_bets_for_market(&env, &market_id);

    // Get fee from legacy storage (backward compatible)
    let fee_percent = env
        .storage()
        .persistent()
        .get(&soroban_sdk::Symbol::new(&env, "platform_fee"))
        .unwrap_or(200);

    let mut has_unclaimed_winners = false;

    // Check voters
    for (user, outcome) in market.votes.iter() {
        if winning_outcomes.contains(&outcome) {
            if !market
                .claimed
                .get(user.clone())
                .map(|info| info.is_claimed())
                .unwrap_or(false)
            {
                has_unclaimed_winners = true;
                break;
            }
        }
    }

    if !has_unclaimed_winners {
        for user in bettors.iter() {
            if let Some(bet) = bets::BetStorage::get_bet(&env, &market_id, &user) {
                if winning_outcomes.contains(&bet.outcome)
                    && !market
                        .claimed
                        .get(user.clone())
                        .map(|info| info.is_claimed())
                        .unwrap_or(false)
                {
                    has_unclaimed_winners = true;
                    break;
                }
            }
        }
    }

    if !has_unclaimed_winners {
        return Ok(0);
    }

    let summary = resolution::ResolutionOutcomeCache::require(&env, &market_id, &market)?;
    let winning_total = summary.winning_total;
    if winning_total == 0 {
        return Ok(0);
    }

    let total_pool = summary.total_pool;
    let fee_denominator = 10000i128;
    let mut total_distributed: i128 = 0;

    // Create budget guard with 100,000 instruction threshold
    let budget_guard = crate::gas::BudgetGuard::new(&env, 100000);

    // 1. Distribute to Voters
    let mut voter_count = 0u32;
    for (user, outcome) in market.votes.iter() {
        if winning_outcomes.contains(&outcome) {
            if market
                .claimed
                .get(user.clone())
                .map(|info| info.is_claimed())
                .unwrap_or(false)
            {
                continue;
            }

            let user_stake = market.stakes.get(user.clone()).unwrap_or(0);
            if user_stake > 0 {
                let user_share = (user_stake
                    .checked_mul(fee_denominator - fee_percent)
                    .ok_or(Error::InvalidInput)?)
                    / fee_denominator;
                let payout = (user_share
                    .checked_mul(total_pool)
                    .ok_or(Error::InvalidInput)?)
                    / winning_total;

                if payout >= 0 {
                    market
                        .claimed
                        .set(user.clone(), crate::types::ClaimInfo::new(&env, payout));
                    if payout > 0 {
                        total_distributed = total_distributed
                            .checked_add(payout)
                            .ok_or(Error::InvalidInput)?;

                        storage::BalanceStorage::add_balance(
                            &env,
                            &user,
                            &types::ReflectorAsset::Stellar,
                            payout,
                        )?;

                        crate::events::EventEmitter::emit_winnings_claimed(&env, &market_id, &user, payout);
                    }
                }
            }
        }

        voter_count += 1;
        if voter_count % 10 == 0 {
            budget_guard.check()?;
        }
    }

    // 2. Distribute to Bettors
    let mut bettor_count = 0u32;
    for user in bettors.iter() {
        if let Some(mut bet) = bets::BetStorage::get_bet(&env, &market_id, &user) {
            if winning_outcomes.contains(&bet.outcome) {
                if market
                    .claimed
                    .get(user.clone())
                    .map(|info| info.is_claimed())
                    .unwrap_or(false)
                {
                    bet.status = crate::types::BetStatus::Won;
                    let _ = bets::BetStorage::store_bet(&env, &bet);
                    continue;
                }

                if bet.amount > 0 {
                    let user_share =
                        (bet.amount * (fee_denominator - fee_percent)) / fee_denominator;
                    let payout = (user_share * total_pool) / winning_total;

                    if payout > 0 {
                        market
                            .claimed
                            .set(user.clone(), crate::types::ClaimInfo::new(&env, payout));
                        total_distributed += payout;

                        bet.status = crate::types::BetStatus::Won;
                        let _ = bets::BetStorage::store_bet(&env, &bet);

                        match storage::BalanceStorage::add_balance(
                            &env,
                            &user,
                            &types::ReflectorAsset::Stellar,
                            payout,
                        ) {
                            Ok(_) => {}
                            Err(e) => panic_with_error!(env, e),
                        }
                        crate::events::EventEmitter::emit_winnings_claimed(&env, &market_id, &user, payout);
                    }
                }
            } else {
                if bet.status == crate::types::BetStatus::Active {
                    bet.status = crate::types::BetStatus::Lost;
                    let _ = bets::BetStorage::store_bet(&env, &bet);
                }
            }
        }

        bettor_count += 1;
        if bettor_count % 10 == 0 {
            budget_guard.check()?;
        }
    }

    budget_guard.check()?;
    env.storage().persistent().set(&market_id, &market);

    Ok(total_distributed)
}