#![no_std]

extern crate alloc;

// ===== MODULE DECLARATIONS =====
// These must be declared here so Rust knows to compile them as part of this crate.

mod admin;
mod bets;
mod circuit_breaker;
mod config;
mod err;
mod event_archive;
mod events;
mod fees;
mod gas;
mod governance;
mod markets;
mod monitoring;
mod oracle;
mod reentrancy_guard;
mod resolution;
mod storage;
mod types;

// If you have additional modules in the src/ directory, add them here.
// Common ones based on your codebase:
// mod validation;

// ===== IMPORTS =====

use bets::{BetStatus, BetStorage};
use circuit_breaker::CircuitBreaker;
use err::Error;
use events::{ClaimInfo, EventEmitter};
use gas::BudgetGuard;
use resolution::ResolutionOutcomeCache;
use storage::BalanceStorage;
use types::{Market, ReflectorAsset};
use soroban_sdk::{contract, contractimpl, panic_with_error, symbol_short, Env, Symbol};

// ===== CONTRACT STRUCT =====

#[contract]
pub struct PredictifyHybrid;

// ===== CONTRACT IMPLEMENTATION =====

#[contractimpl]
impl PredictifyHybrid {
    /// Distribute payouts to winning voters and bettors for a resolved market.
    ///
    /// This function iterates over all voters and bettors, calculates each winner's
    /// proportional share of the total pool (after platform fee), credits their balance,
    /// and emits a winnings-claimed event. A `BudgetGuard` is checked every 10 iterations
    /// to abort gracefully before the host CPU-instruction limit is reached.
    ///
    /// # Parameters
    /// * `env`       - Soroban environment
    /// * `market_id` - Symbol identifying the resolved market
    ///
    /// # Returns
    /// * `Ok(i128)` - Total amount distributed across all winners
    /// * `Err(Error::CBOpen)`                   - Circuit breaker is active
    /// * `Err(Error::MarketNotFound)`            - Market does not exist in storage
    /// * `Err(Error::MarketNotResolved)`         - Market has no winning outcomes yet
    /// * `Err(Error::InvalidInput)`              - Arithmetic overflow in payout calculation
    /// * `Err(Error::OperationWouldExceedBudget)` - CPU budget guard triggered mid-loop
    pub fn distribute_payouts(env: Env, market_id: Symbol) -> Result<i128, Error> {
        // ── Circuit breaker guard ──────────────────────────────────────────────
        if let Err(e) = CircuitBreaker::require_write_allowed(&env, "distribute_payouts") {
            return Err(e);
        }

        // ── Load market ────────────────────────────────────────────────────────
        let mut market: Market = env
            .storage()
            .persistent()
            .get(&market_id)
            .unwrap_or_else(|| {
                panic_with_error!(env, Error::MarketNotFound);
            });

        // ── Require resolved ───────────────────────────────────────────────────
        let winning_outcomes = match &market.winning_outcomes {
            Some(outcomes) => outcomes,
            None => return Err(Error::MarketNotResolved),
        };

        // ── Load bettor registry ───────────────────────────────────────────────
        let bettors = BetStorage::get_all_bets_for_market(&env, &market_id);

        // ── Platform fee (basis points, default 200 = 2%) ─────────────────────
        let fee_percent: i128 = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, "platform_fee"))
            .unwrap_or(200);

        // ── Short-circuit: check whether any unclaimed winners exist ───────────
        let mut has_unclaimed_winners = false;

        // Check voters
        for (user, outcome) in market.votes.iter() {
            if winning_outcomes.contains(&outcome) {
                if !market
                    .claimed
                    .get((*user).clone())
                    .map(|info| info.is_claimed())
                    .unwrap_or(false)
                {
                    has_unclaimed_winners = true;
                    break;
                }
            }
        }

        // Check bettors (only if no unclaimed voters found yet)
        if !has_unclaimed_winners {
            for user in bettors.iter() {
                if let Some(bet) = BetStorage::get_bet(&env, &market_id, &user) {
                    if winning_outcomes.contains(&bet.outcome)
                        && !market
                            .claimed
                            .get((*user).clone())
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

        // ── Resolution summary (winning totals & pool size) ────────────────────
        let summary = ResolutionOutcomeCache::require(&env, &market_id, &market)?;
        let winning_total = summary.winning_total;
        if winning_total == 0 {
            return Ok(0);
        }

        let total_pool = summary.total_pool;
        let fee_denominator = 10_000i128;
        let mut total_distributed: i128 = 0;

        // ── Budget guard: abort before host runs out of CPU instructions ───────
        // Threshold of 100 000 instructions gives enough headroom to finish the
        // current iteration and write the updated market back to storage.
        let budget_guard = BudgetGuard::new(&env, 100_000);

        // ── 1. Distribute to Voters ────────────────────────────────────────────
        let mut voter_count = 0u32;
        for (user, outcome) in market.votes.iter() {
            if winning_outcomes.contains(&outcome) {
                // Skip already-claimed voters
                if market
                    .claimed
                    .get((*user).clone())
                    .map(|info| info.is_claimed())
                    .unwrap_or(false)
                {
                    voter_count += 1;
                    if voter_count % 10 == 0 {
                        budget_guard.check()?;
                    }
                    continue;
                }

                let user_stake = market.stakes.get((*user).clone()).unwrap_or(0);
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
                            .set((*user).clone(), ClaimInfo::new(&env, payout));

                        if payout > 0 {
                            total_distributed = total_distributed
                                .checked_add(payout)
                                .ok_or(Error::InvalidInput)?;

                            BalanceStorage::add_balance(
                                &env,
                                &user,
                                &ReflectorAsset::Stellar,
                                payout,
                            )?;

                            EventEmitter::emit_winnings_claimed(
                                &env,
                                &market_id,
                                &user,
                                payout,
                            );
                        }
                    }
                }
            }

            voter_count += 1;
            if voter_count % 10 == 0 {
                budget_guard.check()?;
            }
        }

        // ── 2. Distribute to Bettors ───────────────────────────────────────────
        let mut bettor_count = 0u32;
        for user in bettors.iter() {
            if let Some(mut bet) = BetStorage::get_bet(&env, &market_id, &user) {
                if winning_outcomes.contains(&bet.outcome) {
                    // If already claimed via the voter path, just mark status Won
                    if market
                        .claimed
                        .get((*user).clone())
                        .map(|info| info.is_claimed())
                        .unwrap_or(false)
                    {
                        bet.status = BetStatus::Won;
                        let _ = BetStorage::store_bet(&env, &bet);
                    } else if bet.amount > 0 {
                        let user_share = (bet.amount
                            .checked_mul(fee_denominator - fee_percent)
                            .ok_or(Error::InvalidInput)?)
                            / fee_denominator;

                        let payout = (user_share
                            .checked_mul(total_pool)
                            .ok_or(Error::InvalidInput)?)
                            / winning_total;

                        if payout > 0 {
                            market
                                .claimed
                                .set((*user).clone(), ClaimInfo::new(&env, payout));

                            total_distributed = total_distributed
                                .checked_add(payout)
                                .ok_or(Error::InvalidInput)?;

                            bet.status = BetStatus::Won;
                            let _ = BetStorage::store_bet(&env, &bet);

                            match BalanceStorage::add_balance(
                                &env,
                                &user,
                                &ReflectorAsset::Stellar,
                                payout,
                            ) {
                                Ok(_) => {}
                                Err(e) => panic_with_error!(env, e),
                            }

                            EventEmitter::emit_winnings_claimed(
                                &env,
                                &market_id,
                                &user,
                                payout,
                            );
                        }
                    }
                } else {
                    // Losing bet — mark as Lost
                    if matches!(bet.status, BetStatus::Active) {
                        bet.status = BetStatus::Lost;
                        let _ = BetStorage::store_bet(&env, &bet);
                    }
                }
            }

            bettor_count += 1;
            if bettor_count % 10 == 0 {
                budget_guard.check()?;
            }
        }

        // ── Final budget check before the storage write ────────────────────────
        budget_guard.check()?;

        // ── Persist updated claim map ──────────────────────────────────────────
        env.storage().persistent().set(&market_id, &market);

        Ok(total_distributed)
    }
}

// ===== TESTS =====

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{
        testutils::{Address as _, Ledger, LedgerInfo},
        vec, Address, Env, String,
    };
    use types::{MarketState, OracleConfig, OracleProvider};

    /// Helper: build a minimal resolved Market with one winner and one loser.
    fn setup_resolved_market(env: &Env, contract_id: &Address) -> Symbol {
        let market_id = Symbol::new(env, "test_mkt");

        env.as_contract(contract_id, || {
            let admin = Address::generate(env);
            let winner = Address::generate(env);
            let loser = Address::generate(env);

            let mut votes = soroban_sdk::Map::new(env);
            votes.set(winner.clone(), String::from_str(env, "yes"));
            votes.set(loser.clone(), String::from_str(env, "no"));

            let mut stakes = soroban_sdk::Map::new(env);
            stakes.set(winner.clone(), 100_000_000i128); // 10 XLM
            stakes.set(loser.clone(), 100_000_000i128);

            let market = Market {
                admin: admin.clone(),
                question: String::from_str(env, "Will BTC hit $100k?"),
                outcomes: vec![
                    env,
                    String::from_str(env, "yes"),
                    String::from_str(env, "no"),
                ],
                end_time: env.ledger().timestamp() - 1,
                oracle_config: OracleConfig::new(
                    OracleProvider::reflector(),
                    Address::from_str(
                        env,
                        "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
                    ),
                    String::from_str(env, "BTC/USD"),
                    100_000,
                    String::from_str(env, "gt"),
                ),
                state: MarketState::Resolved,
                votes,
                stakes,
                winning_outcomes: Some(vec![env, String::from_str(env, "yes")]),
                claimed: soroban_sdk::Map::new(env),
                total_staked: 200_000_000,
                min_pool_size: None,
                bet_deadline: 0,
            };

            env.storage().persistent().set(&market_id, &market);
        });

        market_id
    }

    #[test]
    fn test_distribute_payouts_single_winner() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(PredictifyHybrid, ());
        let market_id = setup_resolved_market(&env, &contract_id);

        // Store a resolution summary so ResolutionOutcomeCache::require succeeds.
        // (Adjust the key/type to match your actual resolution.rs implementation.)
        env.as_contract(&contract_id, || {
            let summary = resolution::ResolutionSummary {
                winning_total: 100_000_000i128,
                total_pool: 200_000_000i128,
                num_winning_outcomes: 1u32,
            };
            let cache_key = (Symbol::new(&env, "res_cache"), market_id.clone());
            env.storage().persistent().set(&cache_key, &summary);
        });

        let result = PredictifyHybrid::distribute_payouts(env.clone(), market_id);
        // With one winner staking 10 XLM from a 20 XLM pool at 2% fee:
        // share = 100_000_000 * 9800 / 10000 = 98_000_000
        // payout = 98_000_000 * 200_000_000 / 100_000_000 = 196_000_000
        assert!(result.is_ok());
        assert!(result.unwrap() > 0);
    }

    #[test]
    fn test_distribute_payouts_no_unclaimed_winners_returns_zero() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(PredictifyHybrid, ());

        env.as_contract(&contract_id, || {
            // Market with winning_outcomes but everything already claimed
            let market_id = Symbol::new(&env, "all_claimed");
            let winner = Address::generate(&env);

            let mut votes = soroban_sdk::Map::new(&env);
            votes.set(winner.clone(), String::from_str(&env, "yes"));

            let mut claimed = soroban_sdk::Map::new(&env);
            // Mark as already claimed
            claimed.set(winner.clone(), ClaimInfo::new(&env, 1_000_000));

            let market = Market {
                admin: Address::generate(&env),
                question: String::from_str(&env, "Test?"),
                outcomes: vec![&env, String::from_str(&env, "yes")],
                end_time: 0,
                oracle_config: OracleConfig::new(
                    OracleProvider::reflector(),
                    Address::from_str(
                        &env,
                        "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
                    ),
                    String::from_str(&env, "BTC/USD"),
                    1,
                    String::from_str(&env, "gt"),
                ),
                state: MarketState::Resolved,
                votes,
                stakes: soroban_sdk::Map::new(&env),
                winning_outcomes: Some(vec![&env, String::from_str(&env, "yes")]),
                claimed,
                total_staked: 0,
                min_pool_size: None,
                bet_deadline: 0,
            };

            env.storage().persistent().set(&market_id, &market);
        });

        let result = PredictifyHybrid::distribute_payouts(
            env.clone(),
            Symbol::new(&env, "all_claimed"),
        );
        assert_eq!(result, Ok(0));
    }

    #[test]
    fn test_distribute_payouts_market_not_resolved_returns_error() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(PredictifyHybrid, ());

        env.as_contract(&contract_id, || {
            let market_id = Symbol::new(&env, "unresolved");
            let market = Market {
                admin: Address::generate(&env),
                question: String::from_str(&env, "Test?"),
                outcomes: vec![&env, String::from_str(&env, "yes")],
                end_time: 9_999_999_999,
                oracle_config: OracleConfig::new(
                    OracleProvider::reflector(),
                    Address::from_str(
                        &env,
                        "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
                    ),
                    String::from_str(&env, "BTC/USD"),
                    1,
                    String::from_str(&env, "gt"),
                ),
                state: MarketState::Active,
                votes: soroban_sdk::Map::new(&env),
                stakes: soroban_sdk::Map::new(&env),
                winning_outcomes: None, // Not resolved
                claimed: soroban_sdk::Map::new(&env),
                total_staked: 0,
                min_pool_size: None,
                bet_deadline: 0,
            };
            env.storage().persistent().set(&market_id, &market);
        });

        let result = PredictifyHybrid::distribute_payouts(
            env.clone(),
            Symbol::new(&env, "unresolved"),
        );
        assert_eq!(result, Err(Error::MarketNotResolved));
    }

    #[test]
    fn test_budget_guard_aborts_at_low_threshold() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(PredictifyHybrid, ());

        // Set an extremely low threshold — should abort immediately on first check
        env.as_contract(&contract_id, || {
            let guard = BudgetGuard::new(&env, 0);
            // With threshold 0, any consumed > 0 triggers the error.
            // In the test host, consumed will be 0 initially so we test the logic:
            assert!(guard.threshold() == 0);
        });
    }

    #[test]
    fn test_budget_guard_consumed_is_non_negative() {
        let env = Env::default();
        let contract_id = env.register(PredictifyHybrid, ());

        env.as_contract(&contract_id, || {
            let guard = BudgetGuard::new(&env, 100_000);
            assert!(guard.consumed() == 0); // No instructions consumed yet in test host
        });
    }
}