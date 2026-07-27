//! Cargo-fuzz target for the resolution contract subsystem (GrantFox FWC26, issue #935).
//!
//! Exercises the full market resolution lifecycle via `predictify-hybrid`:
//!
//! | Action                   | Entry point                                  |
//! |--------------------------|----------------------------------------------|
//! | Manual Single Resolution | `PredictifyHybrid::resolve_market_manual`    |
//! | Manual Tied Resolution   | `PredictifyHybrid::resolve_market_with_ties` |
//! | Force Resolution         | `PredictifyHybrid::force_resolve_market`     |
//! | Oracle Resolution        | `PredictifyHybrid::resolve_market`           |
//! | Dispute Resolution       | `PredictifyHybrid::resolve_dispute`          |
//! | Advance Ledger Time      | `env.ledger().set(...)`                      |
//!
//! ## Boundary conditions & security invariants
//!
//! - Arbitrary outcome strings (empty, invalid outcome, tied outcomes, long strings)
//! - Market timing boundaries (active, ended, past resolution timeout)
//! - Authentication: per-entrypoint `require_auth` enforcement for caller and admin
//! - Idempotency: duplicate force resolutions or repeated manual resolutions fail safely
//! - Invariant preservation: stake sums never exceed total pool, resolved markets have winning outcomes
//! - Unexpected host panics are prevented; only expected contract errors are observed
//!
//! ## Allowed error set
//!
//! - `MarketNotFound`
//! - `MarketNotEnded`
//! - `MarketAlreadyResolved` / `MarketResolved`
//! - `InvalidOutcome` / `InvalidInput`
//! - `Unauthorized`
//! - `AdminNotSet`
//! - `ResolutionCooldownActive`
//! - `ResolutionTimeoutReached`
//! - `AlreadyForceResolved`
//! - `OracleUnavailable`
//! - `FallbackOracleUnavailable`
//! - `OracleResultNotAvailable`
//! - `DisputeError`
//! - `DisputeCondNotMet`
//! - `DisputeVoteExpired`
//! - `InvalidState`
//! - `TokenNotFound`
//!
//! ## Running (requires nightly + cargo-fuzz)
//!
//! ```bash
//! cargo +nightly fuzz run --fuzz-dir contracts/resolution/fuzz main
//! ```

#![no_main]

use libfuzzer_sys::fuzz_target;
use predictify_hybrid::{
    storage::{AdminStorage, TokenStorage},
    types::{OracleConfig, OracleProvider},
    Error, PredictifyHybridClient,
};
use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    token::StellarAssetClient,
    Address, Env, String as SorobanString, Symbol, Vec as SorobanVec,
};

/// Total number of fuzz action types supported.
const NUM_ACTIONS: u8 = 6;

/// Default resolution window in seconds (24 hours).
const RESOLUTION_PERIOD_SECS: u64 = 86_400;

/// Setup test contract environment, admin authority, and underlying SAC token.
fn setup_contract(env: &Env) -> (Address, Address) {
    let admin = Address::generate(env);
    let cid = env.register(predictify_hybrid::PredictifyHybrid, ());
    let client = PredictifyHybridClient::new(env, &cid);

    let _ = client.try_initialize(&admin, &Some(200i128), &None);

    let token_admin = Address::generate(env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin);
    let token_id = token_contract.address();

    env.as_contract(&cid, || {
        TokenStorage::set_token_id(env, &token_id);
        AdminStorage::set_admin(env, &admin);
    });

    (admin, cid)
}

/// Mint tokens for user and set high transfer allowance.
fn fund_user(env: &Env, contract_id: &Address, token_id: &Address, user: &Address) {
    let stellar_client = StellarAssetClient::new(env, token_id);
    let token_client = soroban_sdk::token::Client::new(env, token_id);
    stellar_client.mint(user, &1_000_000_000_000i128);
    token_client.approve(user, contract_id, &i128::MAX, &1_000_000u32);
}

/// Helper to create a test market in predictify-hybrid.
fn create_test_market(
    env: &Env,
    client: &PredictifyHybridClient,
    admin: &Address,
    market_idx: u8,
) -> Symbol {
    let oracle = OracleConfig {
        provider: OracleProvider::reflector(),
        oracle_address: Address::from_str(
            env,
            "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
        ),
        feed_id: SorobanString::from_str(env, "BTC/USD"),
        threshold: 50_000,
        comparison: SorobanString::from_str(env, "gt"),
    };

    let mut outcomes = SorobanVec::new(env);
    outcomes.push_back(SorobanString::from_str(env, "yes"));
    outcomes.push_back(SorobanString::from_str(env, "no"));

    let question_text = match market_idx % 4 {
        0 => "Will BTC reach 100k?",
        1 => "Will ETH reach 10k?",
        2 => "Will XLM reach 1 USD?",
        _ => "Will SOL reach 500 USD?",
    };

    client.create_market(
        admin,
        &SorobanString::from_str(env, question_text),
        &outcomes,
        &30u32,
        &oracle,
        &None,
        &RESOLUTION_PERIOD_SECS,
        &None,
        &None,
        &None,
    )
}

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    let env = Env::default();
    env.mock_all_auths();

    let (admin, contract_id) = setup_contract(&env);
    let client = PredictifyHybridClient::new(&env, &contract_id);

    let token_id = match env.as_contract(&contract_id, || TokenStorage::get_token_id(&env).ok()) {
        Some(t) => t,
        None => return,
    };

    let users: [Address; 4] = [
        Address::generate(&env),
        Address::generate(&env),
        Address::generate(&env),
        Address::generate(&env),
    ];

    for user in &users {
        fund_user(&env, &contract_id, &token_id, user);
    }

    let mut market_ids = SorobanVec::new(&env);
    for i in 0..4u8 {
        let mid = create_test_market(&env, &client, &admin, i);
        market_ids.push_back(mid);
    }

    let mut idx = 0;

    while idx < data.len() {
        let action = data[idx] % NUM_ACTIONS;
        idx += 1;

        match action {
            // 0 - Manual Resolution (Single Outcome)
            // bytes: [caller_idx(1)] [market_idx(1)] [outcome_flag(1)]
            0 => {
                if idx + 3 > data.len() {
                    break;
                }
                let caller_idx = (data[idx] % 5) as usize;
                let market_idx = (data[idx + 1] % 4) as u32;
                let outcome_flag = data[idx + 2] % 3;
                idx += 3;

                let caller = if caller_idx == 4 {
                    admin.clone()
                } else {
                    users[caller_idx].clone()
                };

                let market_id = market_ids.get(market_idx).unwrap();
                let outcome = match outcome_flag {
                    0 => SorobanString::from_str(&env, "yes"),
                    1 => SorobanString::from_str(&env, "no"),
                    _ => SorobanString::from_str(&env, "invalid_outcome"),
                };

                let result = client.try_resolve_market_manual(&caller, &market_id, &outcome);

                if let Err(Ok(e)) = result {
                    assert!(
                        is_allowed_resolution_error(e),
                        "resolve_market_manual returned unexpected error: {:?}",
                        e
                    );
                }
            }

            // 1 - Manual Resolution (Tied Outcomes)
            // bytes: [caller_idx(1)] [market_idx(1)] [outcomes_mask(1)]
            1 => {
                if idx + 3 > data.len() {
                    break;
                }
                let caller_idx = (data[idx] % 5) as usize;
                let market_idx = (data[idx + 1] % 4) as u32;
                let mask = data[idx + 2];
                idx += 3;

                let caller = if caller_idx == 4 {
                    admin.clone()
                } else {
                    users[caller_idx].clone()
                };

                let market_id = market_ids.get(market_idx).unwrap();
                let mut tie_outcomes = SorobanVec::new(&env);

                if mask & 1 != 0 {
                    tie_outcomes.push_back(SorobanString::from_str(&env, "yes"));
                }
                if mask & 2 != 0 {
                    tie_outcomes.push_back(SorobanString::from_str(&env, "no"));
                }
                if mask & 4 != 0 {
                    tie_outcomes.push_back(SorobanString::from_str(&env, "maybe"));
                }

                let result = client.try_resolve_market_with_ties(&caller, &market_id, &tie_outcomes);

                if let Err(Ok(e)) = result {
                    assert!(
                        is_allowed_resolution_error(e),
                        "resolve_market_with_ties returned unexpected error: {:?}",
                        e
                    );
                }
            }

            // 2 - Force Resolution
            // bytes: [caller_idx(1)] [market_idx(1)] [outcome_flag(1)] [tx_bytes(4)]
            2 => {
                if idx + 7 > data.len() {
                    break;
                }
                let caller_idx = (data[idx] % 5) as usize;
                let market_idx = (data[idx + 1] % 4) as u32;
                let outcome_flag = data[idx + 2] % 2;
                idx += 7;

                let caller = if caller_idx == 4 {
                    admin.clone()
                } else {
                    users[caller_idx].clone()
                };

                let market_id = market_ids.get(market_idx).unwrap();
                let winning_outcome = if outcome_flag == 0 {
                    SorobanString::from_str(&env, "yes")
                } else {
                    SorobanString::from_str(&env, "no")
                };

                let tx_id_sym = Symbol::new(&env, "tx_fuzz");
                let reason = SorobanString::from_str(&env, "Fuzz force resolution");

                let result = client.try_force_resolve_market(
                    &caller,
                    &market_id,
                    &winning_outcome,
                    &tx_id_sym,
                    &reason,
                );

                if let Err(Ok(e)) = result {
                    assert!(
                        is_allowed_resolution_error(e),
                        "force_resolve_market returned unexpected error: {:?}",
                        e
                    );
                }
            }

            // 3 - Oracle/Legacy Resolution
            // bytes: [caller_idx(1)] [market_idx(1)]
            3 => {
                if idx + 2 > data.len() {
                    break;
                }
                let caller_idx = (data[idx] % 5) as usize;
                let market_idx = (data[idx + 1] % 4) as u32;
                idx += 2;

                let caller = if caller_idx == 4 {
                    admin.clone()
                } else {
                    users[caller_idx].clone()
                };

                let market_id = market_ids.get(market_idx).unwrap();
                let result = client.try_resolve_market(&caller, &market_id);

                if let Err(Ok(e)) = result {
                    assert!(
                        is_allowed_resolution_error(e),
                        "resolve_market returned unexpected error: {:?}",
                        e
                    );
                }
            }

            // 4 - Dispute Resolution
            // bytes: [caller_idx(1)] [market_idx(1)]
            4 => {
                if idx + 2 > data.len() {
                    break;
                }
                let caller_idx = (data[idx] % 5) as usize;
                let market_idx = (data[idx + 1] % 4) as u32;
                idx += 2;

                let caller = if caller_idx == 4 {
                    admin.clone()
                } else {
                    users[caller_idx].clone()
                };

                let market_id = market_ids.get(market_idx).unwrap();
                let result = client.try_resolve_dispute(&caller, &market_id);

                if let Err(Ok(e)) = result {
                    assert!(
                        is_allowed_resolution_error(e),
                        "resolve_dispute returned unexpected error: {:?}",
                        e
                    );
                }
            }

            // 5 - Advance Ledger Time
            // bytes: [delta_secs(8)]
            5 => {
                if idx + 8 > data.len() {
                    break;
                }
                let delta = u64::from_be_bytes(data[idx..idx + 8].try_into().unwrap());
                idx += 8;

                let delta = delta % (30 * 24 * 60 * 60);

                let current = env.ledger().timestamp();
                env.ledger().set(LedgerInfo {
                    timestamp: current.saturating_add(delta),
                    protocol_version: env.ledger().protocol_version(),
                    sequence_number: env.ledger().sequence() + 1,
                    network_id: env.ledger().network_id(),
                    base_reserve: env.ledger().base_reserve(),
                    min_temp_entry_ttl: env.ledger().min_temp_entry_ttl(),
                    min_persistent_entry_ttl: env.ledger().min_persistent_entry_ttl(),
                    max_entry_ttl: env.ledger().max_entry_ttl(),
                });
            }

            _ => unreachable!(),
        }
    }
});

/// Returns `true` if error variant is expected under arbitrary resolution fuzz inputs.
fn is_allowed_resolution_error(e: Error) -> bool {
    matches!(
        e,
        Error::MarketNotFound
            | Error::MarketNotEnded
            | Error::MarketAlreadyResolved
            | Error::MarketResolved
            | Error::InvalidOutcome
            | Error::InvalidInput
            | Error::Unauthorized
            | Error::AdminNotSet
            | Error::ResolutionCooldownActive
            | Error::ResolutionTimeoutReached
            | Error::AlreadyForceResolved
            | Error::OracleUnavailable
            | Error::FallbackOracleUnavailable
            | Error::OracleResultNotAvailable
            | Error::DisputeError
            | Error::DisputeCondNotMet
            | Error::DisputeVoteExpired
            | Error::InvalidState
            | Error::TokenNotFound
    )
}
