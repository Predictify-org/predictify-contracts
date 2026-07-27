#![no_main]

use libfuzzer_sys::fuzz_target;
use soroban_sdk::{
    Address, Env, String as SorobanString, Symbol, Vec as SorobanVec,
    testutils::{Address as _, LedgerInfo, Ledger},
};
use predictify_hybrid::{
    PredictifyHybrid, PredictifyHybridClient, OracleConfig, OracleProvider,
};

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);

    // Pool of 3 users
    let users = [
        Address::generate(&env),
        Address::generate(&env),
        Address::generate(&env),
    ];

    // Register and initialize PredictifyHybrid
    let contract_id = env.register(PredictifyHybrid, ());
    let client = PredictifyHybridClient::new(&env, &contract_id);

    // Initialize with a default platform fee of 200 bps (2%)
    if client.try_initialize(&admin, &Some(200), &None).is_err() {
        return;
    }

    // Set up mock token for staking
    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin);
    let token_id = token_contract.address();

    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&Symbol::new(&env, "TokenID"), &token_id);
    });

    // Fund the users and approve the contract to spend
    let stellar_client = soroban_sdk::token::StellarAssetClient::new(&env, &token_id);
    let token_client = soroban_sdk::token::Client::new(&env, &token_id);
    for user in &users {
        stellar_client.mint(user, &100_000_000_000); // 100,000 stroops/tokens
        token_client.approve(user, &contract_id, &i128::MAX, &100_000);
    }

    // Pre-create 3 markets for fuzzing
    let mut market_ids = SorobanVec::new(&env);
    let outcomes = soroban_sdk::vec![
        &env,
        SorobanString::from_str(&env, "yes"),
        SorobanString::from_str(&env, "no"),
    ];

    for i in 0..3 {
        let question = SorobanString::from_str(&env, "Will prediction markets be popular?");
        let market_id = client.create_market(
            &admin,
            &question,
            &outcomes,
            &30,
            &OracleConfig {
                provider: OracleProvider::reflector(),
                oracle_address: Address::from_str(
                    &env,
                    "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
                ),
                feed_id: SorobanString::from_str(&env, "BTC/USD"),
                threshold: 100_000_00000000,
                comparison: SorobanString::from_str(&env, "gt"),
            },
            &None,
            &86400u64,
            &None,
            &None,
            &None,
            &None,
            &None,
        );
        market_ids.push_back(market_id);
    }

    let mut idx = 0;
    while idx < data.len() {
        let action_type = data[idx] % 9;
        idx += 1;

        match action_type {
            0 => {
                // PlaceBet: places a fuzzed bet on a fuzzed market for a fuzzed user
                if idx + 19 > data.len() {
                    break;
                }
                let user_idx = (data[idx] % 3) as usize;
                let market_idx = (data[idx + 1] % 3) as u32;
                let outcome_is_yes = data[idx + 2] % 2 == 0;
                let amount = i128::from_be_bytes(data[idx + 3..idx + 11].try_into().unwrap());
                let max_fee_bps = i128::from_be_bytes(data[idx + 11..idx + 19].try_into().unwrap());
                idx += 19;

                let user = &users[user_idx];
                let market_id = market_ids.get(market_idx).unwrap();
                let outcome = SorobanString::from_str(&env, if outcome_is_yes { "yes" } else { "no" });

                let _ = client.try_place_bet(user, &market_id, &outcome, &amount, &max_fee_bps);
            }
            1 => {
                // PlaceBets: places multiple fuzzed bets in batch
                if idx + 20 > data.len() {
                    break;
                }
                let user_idx = (data[idx] % 3) as usize;
                let max_fee_bps = i128::from_be_bytes(data[idx + 1..idx + 9].try_into().unwrap());
                let bets_count = (data[idx + 9] % 3) as usize;
                idx += 10;

                let mut bets_vec = SorobanVec::new(&env);
                for _ in 0..bets_count {
                    if idx + 10 > data.len() {
                        break;
                    }
                    let market_idx = (data[idx] % 3) as u32;
                    let outcome_is_yes = data[idx + 1] % 2 == 0;
                    let amount = i128::from_be_bytes(data[idx + 2..idx + 10].try_into().unwrap());
                    idx += 10;

                    let market_id = market_ids.get(market_idx).unwrap();
                    let outcome = SorobanString::from_str(&env, if outcome_is_yes { "yes" } else { "no" });
                    bets_vec.push_back((market_id, outcome, amount));
                }

                let user = &users[user_idx];
                let _ = client.try_place_bets(user, &bets_vec, &max_fee_bps, &None);
            }
            2 => {
                // CancelBet: cancels a fuzzed bet
                if idx + 2 > data.len() {
                    break;
                }
                let user_idx = (data[idx] % 3) as usize;
                let market_idx = (data[idx + 1] % 3) as u32;
                idx += 2;

                let user = &users[user_idx];
                let market_id = market_ids.get(market_idx).unwrap();

                let _ = client.try_cancel_bet(user, &market_id);
            }
            3 => {
                // ClaimWinnings: claims winnings for a resolved market
                if idx + 2 > data.len() {
                    break;
                }
                let user_idx = (data[idx] % 3) as usize;
                let market_idx = (data[idx + 1] % 3) as u32;
                idx += 2;

                let user = &users[user_idx];
                let market_id = market_ids.get(market_idx).unwrap();

                let _ = client.try_claim_winnings(user, &market_id);
            }
            4 => {
                // SetGlobalBetLimits: configures minimum and maximum bet limits globally
                if idx + 16 > data.len() {
                    break;
                }
                let min_bet = i128::from_be_bytes(data[idx..idx + 8].try_into().unwrap());
                let max_bet = i128::from_be_bytes(data[idx + 8..idx + 16].try_into().unwrap());
                idx += 16;

                let _ = client.try_set_global_bet_limits(&admin, &min_bet, &max_bet);
            }
            5 => {
                // SetEventBetLimits: configures minimum and maximum bet limits for a specific market
                if idx + 17 > data.len() {
                    break;
                }
                let market_idx = (data[idx] % 3) as u32;
                let min_bet = i128::from_be_bytes(data[idx + 1..idx + 9].try_into().unwrap());
                let max_bet = i128::from_be_bytes(data[idx + 9..idx + 17].try_into().unwrap());
                idx += 17;

                let market_id = market_ids.get(market_idx).unwrap();
                let _ = client.try_set_event_bet_limits(&admin, &market_id, &min_bet, &max_bet);
            }
            6 => {
                // SetMarketMaxBetCap: sets max bet cap on a market
                if idx + 9 > data.len() {
                    break;
                }
                let market_idx = (data[idx] % 3) as u32;
                let cap = i128::from_be_bytes(data[idx + 1..idx + 9].try_into().unwrap());
                idx += 9;

                let market_id = market_ids.get(market_idx).unwrap();
                let _ = client.try_set_market_max_bet_cap(&admin, &market_id, &cap);
            }
            7 => {
                // Pause/Unpause contract circuit breaker
                if idx + 1 > data.len() {
                    break;
                }
                let should_pause = data[idx] % 2 == 0;
                idx += 1;

                if should_pause {
                    let _ = client.try_pause(&admin);
                } else {
                    let _ = client.try_unpause(&admin);
                }
            }
            8 => {
                // ResolveMarket: resolves a market manually
                if idx + 3 > data.len() {
                    break;
                }
                let market_idx = (data[idx] % 3) as u32;
                let outcome_is_yes = data[idx + 1] % 2 == 0;
                idx += 2;

                let market_id = market_ids.get(market_idx).unwrap();
                let outcome = SorobanString::from_str(&env, if outcome_is_yes { "yes" } else { "no" });

                // Try to advance ledger past the market end time to satisfy manual resolution checks
                if let Some(market) = client.get_market(&market_id) {
                    env.ledger().set(LedgerInfo {
                        timestamp: market.end_time + 86400,
                        protocol_version: env.ledger().protocol_version(),
                        sequence_number: env.ledger().sequence() + 1,
                        network_id: env.ledger().network_id(),
                        base_reserve: env.ledger().base_reserve(),
                        min_temp_entry_ttl: env.ledger().min_temp_entry_ttl(),
                        min_persistent_entry_ttl: env.ledger().min_persistent_entry_ttl(),
                        max_entry_ttl: env.ledger().max_entry_ttl(),
                    });
                }

                let _ = client.try_resolve_market_manual(&admin, &market_id, &outcome);
            }
            _ => unreachable!(),
        }
    }
});
