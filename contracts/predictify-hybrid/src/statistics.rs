#![allow(dead_code)]

use crate::events::EventEmitter;
use crate::types::{PlatformStatistics, UserStatistics};
use soroban_sdk::{symbol_short, Address, Env, Symbol};

const PLATFORM_STATS_KEY: Symbol = symbol_short!("p_stats");
const USER_STATS_PREFIX: Symbol = symbol_short!("u_stats");

pub struct StatisticsManager;

impl StatisticsManager {
    /// Get platform statistics, initializing if not present
    pub fn get_platform_stats(env: &Env) -> PlatformStatistics {
        env.storage()
            .persistent()
            .get(&PLATFORM_STATS_KEY)
            .unwrap_or(PlatformStatistics {
                total_events_created: 0,
                total_bets_placed: 0,
                total_volume: 0,
                total_fees_collected: 0,
                active_events_count: 0,
            })
    }

    /// Set platform statistics
    fn set_platform_stats(env: &Env, stats: &PlatformStatistics) {
        env.storage().persistent().set(&PLATFORM_STATS_KEY, stats);
    }

    /// Get user statistics, initializing if not present
    pub fn get_user_stats(env: &Env, user: &Address) -> UserStatistics {
        // We use a tuple key (prefix, user) for user stats
        // Note: Soroban limits key size. Using a vec or tuple key is standard.
        // However, generic keys can be tricky. Using a dedicated storage key constructor is better.
        // For simplicity and efficiency, let's assume we can key by (Symbol, Address).
        env.storage()
            .persistent()
            .get(&(USER_STATS_PREFIX, user.clone()))
            .unwrap_or(UserStatistics {
                total_bets_placed: 0,
                total_amount_wagered: 0,
                total_winnings: 0,
                total_bets_won: 0,
                win_rate: 0,
                last_activity_ts: 0,
            })
    }

    /// Set user statistics
    fn set_user_stats(env: &Env, user: &Address, stats: &UserStatistics) {
        env.storage()
            .persistent()
            .set(&(USER_STATS_PREFIX, user.clone()), stats);
    }

    /// Record a new market creation
    pub fn record_market_created(env: &Env) {
        let mut stats = Self::get_platform_stats(env);
        stats.total_events_created = stats
            .total_events_created
            .checked_add(1)
            .unwrap_or(stats.total_events_created);
        stats.active_events_count = stats
            .active_events_count
            .checked_add(1)
            .unwrap_or(stats.active_events_count);
        Self::set_platform_stats(env, &stats);

        Self::emit_update(env, &stats);
    }

    /// Record a market resolution (decrements active count)
    pub fn record_market_resolved(env: &Env) {
        let mut stats = Self::get_platform_stats(env);
        if stats.active_events_count > 0 {
            stats.active_events_count -= 1;
        }
        Self::set_platform_stats(env, &stats);

        Self::emit_update(env, &stats);
    }

    /// Record a new bet placement
    pub fn record_bet_placed(env: &Env, user: &Address, amount: i128) {
        // Update platform stats
        let mut p_stats = Self::get_platform_stats(env);
        p_stats.total_bets_placed = p_stats
            .total_bets_placed
            .checked_add(1)
            .unwrap_or(p_stats.total_bets_placed);
        p_stats.total_volume = p_stats
            .total_volume
            .checked_add(amount)
            .unwrap_or(p_stats.total_volume);
        Self::set_platform_stats(env, &p_stats);

        Self::emit_update(env, &p_stats);

        // Update user stats
        let mut u_stats = Self::get_user_stats(env, user);
        u_stats.total_bets_placed = u_stats
            .total_bets_placed
            .checked_add(1)
            .unwrap_or(u_stats.total_bets_placed);
        u_stats.total_amount_wagered = u_stats
            .total_amount_wagered
            .checked_add(amount)
            .unwrap_or(u_stats.total_amount_wagered);
        u_stats.last_activity_ts = env.ledger().timestamp();
        // Win rate doesn't change on bet placement, only on resolution/claim
        Self::set_user_stats(env, user, &u_stats);
    }

    /// Record winnings claimed
    pub fn record_winnings_claimed(env: &Env, user: &Address, amount: i128) {
        // Note: fees are already deducted from 'amount' usually?
        // Or do we track total fees collected separately?
        // The implementation plan says "increments fees".
        // But claim_winnings in lib.rs logic:
        // user_share = user_stake * (1 - fee) * total_pool / winning_total
        // The fee part stays in the contract or is sent to fee collector?
        // lib.rs seems to deduct fee from user_share.
        // So the "fee collected" is the difference.
        // I need to update the hook to pass the fee amount.

        // Update user stats
        let mut u_stats = Self::get_user_stats(env, user);
        u_stats.total_winnings = u_stats
            .total_winnings
            .checked_add(amount)
            .unwrap_or(u_stats.total_winnings);
        u_stats.total_bets_won = u_stats
            .total_bets_won
            .checked_add(1)
            .unwrap_or(u_stats.total_bets_won);
        u_stats.last_activity_ts = env.ledger().timestamp();

        // Recalculate win rate
        // Win rate = (bets_won / bets_placed) * 10000
        if u_stats.total_bets_placed > 0 {
            u_stats.win_rate = ((u_stats.total_bets_won as u128 * 10000)
                / u_stats.total_bets_placed as u128) as u32;
        }

        Self::set_user_stats(env, user, &u_stats);
    }

    /// Record fees collected
    pub fn record_fees_collected(env: &Env, amount: i128) {
        let mut p_stats = Self::get_platform_stats(env);
        p_stats.total_fees_collected = p_stats
            .total_fees_collected
            .checked_add(amount)
            .unwrap_or(p_stats.total_fees_collected);
        Self::set_platform_stats(env, &p_stats);

        // We might not want to emit full update on every fee collection if it's frequent, but for now consistent behavior is good.
        Self::emit_update(env, &p_stats);
    }

    fn emit_update(env: &Env, stats: &PlatformStatistics) {
        EventEmitter::emit_statistics_updated(
            env,
            stats.total_volume,
            stats.total_bets_placed,
            stats.active_events_count,
        );
    }
}
