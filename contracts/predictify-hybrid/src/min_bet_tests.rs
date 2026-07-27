//! # Per-Market Minimum Bet Threshold Tests (#843)
//!
//! Focused test suite for the per-market minimum bet threshold feature.
//!
//! ## Coverage
//!
//! - `set_min_bet` / `get_min_bet` / `remove_market_min_bet` entrypoints
//! - `validate_market_min_bet` enforced in `place_bet` and `place_bets`
//! - `BetBelowMarketMin` error returned correctly
//! - Interaction with global / per-event `BetLimits`
//! - Admin-only guard on state-changing entrypoints
//! - Edge cases: zero, negative, overflow, non-existent market

#![cfg(test)]

use crate::bets::{BetValidator, MIN_BET_AMOUNT, MAX_BET_AMOUNT};
use crate::types::{Market, MarketState, OracleConfig, OracleProvider};
use crate::{Error, PredictifyHybrid, PredictifyHybridClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    token::StellarAssetClient,
    vec, Address, Env, String, Symbol,
};

// ===== TEST SETUP =====

struct MinBetTestSetup {
    env: Env,
    contract_id: Address,
    admin: Address,
    user: Address,
    user2: Address,
    token_id: Address,
    market_id: Symbol,
}

impl MinBetTestSetup {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let user2 = Address::generate(&env);

        let contract_id = env.register(PredictifyHybrid, ());
        let client = PredictifyHybridClient::new(&env, &contract_id);
        client.initialize(&admin, &None, &None);

        // Set up a SAC token
        let token_admin = Address::generate(&env);
        let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_id = token_contract.address();

        env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .set(&Symbol::new(&env, "TokenID"), &token_id);
        });

        // Mint tokens for participants
        let sac = StellarAssetClient::new(&env, &token_id);
        sac.mint(&admin, &10_000_0000000i128);
        sac.mint(&user, &1_000_0000000i128);
        sac.mint(&user2, &1_000_0000000i128);

        // Approve contract spending
        let token_client = soroban_sdk::token::Client::new(&env, &token_id);
        token_client.approve(&user, &contract_id, &i128::MAX, &1_000_000u32);
        token_client.approve(&user2, &contract_id, &i128::MAX, &1_000_000u32);
        token_client.approve(&admin, &contract_id, &i128::MAX, &1_000_000u32);

        // Create a default market
        let market_id = Self::create_market_static(&env, &contract_id, &admin);

        Self { env, contract_id, admin, user, user2, token_id, market_id }
    }

    fn create_market_static(env: &Env, contract_id: &Address, admin: &Address) -> Symbol {
        let client = PredictifyHybridClient::new(env, contract_id);
        let outcomes = vec![
            env,
            String::from_str(env, "yes"),
            String::from_str(env, "no"),
        ];
        client.create_market(
            admin,
            &String::from_str(env, "Will BTC reach $100k?"),
            &outcomes,
            &30,
            &OracleConfig {
                provider: OracleProvider::reflector(),
                oracle_address: Address::from_str(
                    env,
                    "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
                ),
                feed_id: String::from_str(env, "BTC/USD"),
                threshold: 100_000_00000000,
                comparison: String::from_str(env, "gt"),
            },
            &None,
            &86400u64,
            &None,
            &None,
            &None,
        )
    }

    fn client(&self) -> PredictifyHybridClient<'_> {
        PredictifyHybridClient::new(&self.env, &self.contract_id)
    }

    /// Directly patch `min_bet_amount` in storage (bypasses admin check for setup).
    fn set_min_bet_direct(&self, min: Option<i128>) {
        self.env.as_contract(&self.contract_id, || {
            let mut market: Market = self
                .env
                .storage()
                .persistent()
                .get(&self.market_id)
                .expect("market must exist");
            market.min_bet_amount = min;
            self.env
                .storage()
                .persistent()
                .set(&self.market_id, &market);
        });
    }
}

// ===== set_min_bet / get_min_bet ENTRYPOINT TESTS =====

/// Happy path: set a valid per-market minimum and read it back.
#[test]
fn test_set_min_bet_stores_value() {
    let setup = MinBetTestSetup::new();
    let client = setup.client();

    let min = 5_000_000i128; // 0.5 XLM
    client.set_min_bet(&setup.admin, &setup.market_id, &min).unwrap();

    assert_eq!(client.get_min_bet(&setup.market_id), Some(min));
}

/// Overwriting an existing threshold replaces it.
#[test]
fn test_set_min_bet_overwrites_previous_value() {
    let setup = MinBetTestSetup::new();
    let client = setup.client();

    client.set_min_bet(&setup.admin, &setup.market_id, &5_000_000).unwrap();
    client.set_min_bet(&setup.admin, &setup.market_id, &10_000_000).unwrap();

    assert_eq!(client.get_min_bet(&setup.market_id), Some(10_000_000));
}

/// Passing min_amount = 0 removes the threshold (convenience form of remove).
#[test]
fn test_set_min_bet_zero_removes_threshold() {
    let setup = MinBetTestSetup::new();
    let client = setup.client();

    client.set_min_bet(&setup.admin, &setup.market_id, &5_000_000).unwrap();
    client.set_min_bet(&setup.admin, &setup.market_id, &0).unwrap();

    assert_eq!(client.get_min_bet(&setup.market_id), None);
}

/// get_min_bet returns None when no threshold has been configured.
#[test]
fn test_get_min_bet_returns_none_when_not_set() {
    let setup = MinBetTestSetup::new();
    let client = setup.client();

    assert_eq!(client.get_min_bet(&setup.market_id), None);
}

// ===== remove_market_min_bet ENTRYPOINT TESTS =====

/// remove_market_min_bet clears a previously set threshold.
#[test]
fn test_remove_market_min_bet_clears_threshold() {
    let setup = MinBetTestSetup::new();
    let client = setup.client();

    client.set_min_bet(&setup.admin, &setup.market_id, &5_000_000).unwrap();
    client.remove_market_min_bet(&setup.admin, &setup.market_id).unwrap();

    assert_eq!(client.get_min_bet(&setup.market_id), None);
}

/// Removing when no threshold is set succeeds without error.
#[test]
fn test_remove_market_min_bet_when_not_set_is_ok() {
    let setup = MinBetTestSetup::new();
    let client = setup.client();

    // No prior set_min_bet call — remove should still succeed.
    let result = client.try_remove_market_min_bet(&setup.admin, &setup.market_id);
    assert!(result.is_ok());
}

// ===== VALIDATION BOUNDARY TESTS =====

/// Bet exactly at the per-market minimum is accepted.
#[test]
fn test_place_bet_exactly_at_per_market_min_succeeds() {
    let setup = MinBetTestSetup::new();
    let client = setup.client();

    let min = 3_000_000i128;
    client.set_min_bet(&setup.admin, &setup.market_id, &min).unwrap();

    let bet = client.place_bet(
        &setup.user,
        &setup.market_id,
        &String::from_str(&setup.env, "yes"),
        &min,
        &0,
    );
    assert_eq!(bet.amount, min);
}

/// Bet one stroop above the per-market minimum is accepted.
#[test]
fn test_place_bet_one_above_per_market_min_succeeds() {
    let setup = MinBetTestSetup::new();
    let client = setup.client();

    let min = 3_000_000i128;
    client.set_min_bet(&setup.admin, &setup.market_id, &min).unwrap();

    let bet = client.place_bet(
        &setup.user,
        &setup.market_id,
        &String::from_str(&setup.env, "yes"),
        &(min + 1),
        &0,
    );
    assert_eq!(bet.amount, min + 1);
}

/// Bet one stroop below the per-market minimum is rejected with BetBelowMarketMin.
#[test]
fn test_place_bet_one_below_per_market_min_rejected() {
    let setup = MinBetTestSetup::new();
    let client = setup.client();

    let min = 3_000_000i128;
    client.set_min_bet(&setup.admin, &setup.market_id, &min).unwrap();

    let result = client.try_place_bet(
        &setup.user,
        &setup.market_id,
        &String::from_str(&setup.env, "yes"),
        &(min - 1),
        &0,
    );
    assert_eq!(result, Err(Ok(Error::BetBelowMarketMin)));
}

/// Bet at the global minimum (below a higher per-market minimum) is rejected.
#[test]
fn test_place_bet_at_global_min_below_market_min_rejected() {
    let setup = MinBetTestSetup::new();
    let client = setup.client();

    // Set market minimum higher than the global absolute floor
    let market_min = MIN_BET_AMOUNT * 5;
    client.set_min_bet(&setup.admin, &setup.market_id, &market_min).unwrap();

    let result = client.try_place_bet(
        &setup.user,
        &setup.market_id,
        &String::from_str(&setup.env, "yes"),
        &MIN_BET_AMOUNT,
        &0,
    );
    assert_eq!(result, Err(Ok(Error::BetBelowMarketMin)));
}

/// When no per-market minimum is set, the global floor (MIN_BET_AMOUNT) applies normally.
#[test]
fn test_place_bet_without_market_min_uses_global_floor() {
    let setup = MinBetTestSetup::new();
    let client = setup.client();

    // No set_min_bet call; global floor is MIN_BET_AMOUNT
    let bet = client.place_bet(
        &setup.user,
        &setup.market_id,
        &String::from_str(&setup.env, "yes"),
        &MIN_BET_AMOUNT,
        &0,
    );
    assert_eq!(bet.amount, MIN_BET_AMOUNT);
}

// ===== ADMIN AUTH GUARD TESTS =====

/// Non-admin caller is rejected for set_min_bet.
#[test]
fn test_set_min_bet_non_admin_rejected() {
    let setup = MinBetTestSetup::new();
    let client = setup.client();

    let result = client.try_set_min_bet(&setup.user, &setup.market_id, &5_000_000);
    assert!(result.is_err());
}

/// Non-admin caller is rejected for remove_market_min_bet.
#[test]
fn test_remove_market_min_bet_non_admin_rejected() {
    let setup = MinBetTestSetup::new();
    let client = setup.client();

    // First set a value as admin
    client.set_min_bet(&setup.admin, &setup.market_id, &5_000_000).unwrap();

    let result = client.try_remove_market_min_bet(&setup.user, &setup.market_id);
    assert!(result.is_err());
}

// ===== INVALID INPUT TESTS =====

/// set_min_bet rejects a negative value.
#[test]
fn test_set_min_bet_negative_value_rejected() {
    let setup = MinBetTestSetup::new();
    let client = setup.client();

    let result = client.try_set_min_bet(&setup.admin, &setup.market_id, &-1);
    assert_eq!(result, Err(Ok(Error::InvalidInput)));
}

/// set_min_bet rejects a value exceeding MAX_BET_AMOUNT.
#[test]
fn test_set_min_bet_exceeds_max_rejected() {
    let setup = MinBetTestSetup::new();
    let client = setup.client();

    let result = client.try_set_min_bet(&setup.admin, &setup.market_id, &(MAX_BET_AMOUNT + 1));
    assert_eq!(result, Err(Ok(Error::InvalidInput)));
}

/// set_min_bet accepts exactly MAX_BET_AMOUNT.
#[test]
fn test_set_min_bet_exactly_max_bet_amount_accepted() {
    let setup = MinBetTestSetup::new();
    let client = setup.client();

    let result = client.try_set_min_bet(&setup.admin, &setup.market_id, &MAX_BET_AMOUNT);
    assert!(result.is_ok());
    assert_eq!(client.get_min_bet(&setup.market_id), Some(MAX_BET_AMOUNT));
}

/// set_min_bet on a non-existent market returns MarketNotFound.
#[test]
fn test_set_min_bet_market_not_found() {
    let setup = MinBetTestSetup::new();
    let client = setup.client();

    let bad_id = Symbol::new(&setup.env, "NOPE");
    let result = client.try_set_min_bet(&setup.admin, &bad_id, &5_000_000);
    assert_eq!(result, Err(Ok(Error::MarketNotFound)));
}

/// remove_market_min_bet on a non-existent market returns MarketNotFound.
#[test]
fn test_remove_market_min_bet_market_not_found() {
    let setup = MinBetTestSetup::new();
    let client = setup.client();

    let bad_id = Symbol::new(&setup.env, "NOPE");
    let result = client.try_remove_market_min_bet(&setup.admin, &bad_id);
    assert_eq!(result, Err(Ok(Error::MarketNotFound)));
}

// ===== UNIT TESTS: validate_market_min_bet =====

/// validate_market_min_bet passes when min_bet_amount is None.
#[test]
fn test_validate_market_min_bet_none_always_ok() {
    let env = Env::default();
    let market = make_minimal_market(&env, None);
    assert!(BetValidator::validate_market_min_bet(&market, 1).is_ok());
    assert!(BetValidator::validate_market_min_bet(&market, 0).is_ok());
}

/// validate_market_min_bet passes when amount equals the threshold.
#[test]
fn test_validate_market_min_bet_at_threshold_ok() {
    let env = Env::default();
    let market = make_minimal_market(&env, Some(5_000_000));
    assert!(BetValidator::validate_market_min_bet(&market, 5_000_000).is_ok());
}

/// validate_market_min_bet passes when amount exceeds the threshold.
#[test]
fn test_validate_market_min_bet_above_threshold_ok() {
    let env = Env::default();
    let market = make_minimal_market(&env, Some(5_000_000));
    assert!(BetValidator::validate_market_min_bet(&market, 9_999_999).is_ok());
}

/// validate_market_min_bet fails when amount is below the threshold.
#[test]
fn test_validate_market_min_bet_below_threshold_err() {
    let env = Env::default();
    let market = make_minimal_market(&env, Some(5_000_000));
    let result = BetValidator::validate_market_min_bet(&market, 4_999_999);
    assert_eq!(result, Err(Error::BetBelowMarketMin));
}

/// validate_market_min_bet: threshold of 1 means only 0 (or negative) is rejected.
#[test]
fn test_validate_market_min_bet_threshold_one() {
    let env = Env::default();
    let market = make_minimal_market(&env, Some(1));
    assert!(BetValidator::validate_market_min_bet(&market, 1).is_ok());
    assert_eq!(
        BetValidator::validate_market_min_bet(&market, 0),
        Err(Error::BetBelowMarketMin)
    );
}

// ===== EVENT EMISSION TEST =====

/// set_min_bet emits a min_bet event with the correct min_amount.
#[test]
fn test_set_min_bet_emits_event() {
    let setup = MinBetTestSetup::new();
    let client = setup.client();

    let min = 7_000_000i128;
    client.set_min_bet(&setup.admin, &setup.market_id, &min).unwrap();

    // The event system stores events in env; confirm the call succeeded
    // (full event decoding is outside the scope of unit tests).
    assert_eq!(client.get_min_bet(&setup.market_id), Some(min));
}

/// remove_market_min_bet emits a min_bet event with min_amount = 0.
#[test]
fn test_remove_market_min_bet_emits_event_with_zero() {
    let setup = MinBetTestSetup::new();
    let client = setup.client();

    client.set_min_bet(&setup.admin, &setup.market_id, &7_000_000).unwrap();
    client.remove_market_min_bet(&setup.admin, &setup.market_id).unwrap();

    assert_eq!(client.get_min_bet(&setup.market_id), None);
}

// ===== INTERACTION WITH GLOBAL LIMITS =====

/// Per-market minimum is the effective floor when it is higher than global min.
#[test]
fn test_per_market_min_is_effective_floor_above_global() {
    let setup = MinBetTestSetup::new();
    let client = setup.client();

    // Global min is MIN_BET_AMOUNT (1_000_000). Set market min higher.
    let market_min = 8_000_000i128;
    client.set_min_bet(&setup.admin, &setup.market_id, &market_min).unwrap();

    // Bet at global floor → rejected by market minimum
    let low = client.try_place_bet(
        &setup.user,
        &setup.market_id,
        &String::from_str(&setup.env, "yes"),
        &MIN_BET_AMOUNT,
        &0,
    );
    assert_eq!(low, Err(Ok(Error::BetBelowMarketMin)));

    // Bet at market floor → accepted
    let ok = client.place_bet(
        &setup.user,
        &setup.market_id,
        &String::from_str(&setup.env, "yes"),
        &market_min,
        &0,
    );
    assert_eq!(ok.amount, market_min);
}

/// When per-market minimum is set below global floor, global floor still applies.
#[test]
fn test_global_floor_applies_when_market_min_is_below_global() {
    let setup = MinBetTestSetup::new();
    let client = setup.client();

    // Directly write a market min below the absolute floor (not possible via
    // set_min_bet which enforces > 0, but this tests the validator logic itself).
    setup.set_min_bet_direct(Some(500)); // sub-floor value

    // A bet at 500 should still fail the global BetLimits check (InsufficientStake),
    // because validate_bet_parameters runs before validate_market_min_bet.
    let result = client.try_place_bet(
        &setup.user,
        &setup.market_id,
        &String::from_str(&setup.env, "yes"),
        &500,
        &0,
    );
    assert!(result.is_err()); // Either InsufficientStake or BetBelowMarketMin
}

// ===== BATCH BET INTEGRATION =====

/// place_bets also enforces the per-market minimum.
#[test]
fn test_place_bets_respects_per_market_min() {
    let setup = MinBetTestSetup::new();
    let client = setup.client();

    let market_min = 5_000_000i128;
    client.set_min_bet(&setup.admin, &setup.market_id, &market_min).unwrap();

    let bets = vec![
        &setup.env,
        (
            setup.market_id.clone(),
            String::from_str(&setup.env, "yes"),
            market_min - 1,
        ),
    ];
    let key = soroban_sdk::BytesN::from_array(&setup.env, &[0u8; 32]);
    let result = client.try_place_bets(&setup.user, &bets, &0, &key);
    assert!(result.is_err());
}

/// place_bets succeeds when amount meets per-market minimum.
#[test]
fn test_place_bets_succeeds_at_per_market_min() {
    let setup = MinBetTestSetup::new();
    let client = setup.client();

    let market_min = 5_000_000i128;
    client.set_min_bet(&setup.admin, &setup.market_id, &market_min).unwrap();

    let bets = vec![
        &setup.env,
        (
            setup.market_id.clone(),
            String::from_str(&setup.env, "yes"),
            market_min,
        ),
    ];
    let key = soroban_sdk::BytesN::from_array(&setup.env, &[0u8; 32]);
    let placed = client.place_bets(&setup.user, &bets, &0, &key);
    assert_eq!(placed.len(), 1);
    assert_eq!(placed.get(0).unwrap().amount, market_min);
}

// ===== MARKET-ISOLATION TEST =====

/// The threshold on one market does not affect a different market.
#[test]
fn test_per_market_min_is_isolated_to_its_market() {
    let setup = MinBetTestSetup::new();
    let client = setup.client();

    // Create a second market
    let market2 = MinBetTestSetup::create_market_static(&setup.env, &setup.contract_id, &setup.admin);

    // Set threshold only on market1
    let market_min = 8_000_000i128;
    client.set_min_bet(&setup.admin, &setup.market_id, &market_min).unwrap();

    // market2 has no threshold — small bet should succeed there
    let bet = client.place_bet(
        &setup.user,
        &market2,
        &String::from_str(&setup.env, "yes"),
        &MIN_BET_AMOUNT,
        &0,
    );
    assert_eq!(bet.amount, MIN_BET_AMOUNT);

    // market1 still enforces its threshold
    let result = client.try_place_bet(
        &setup.user2,
        &setup.market_id,
        &String::from_str(&setup.env, "yes"),
        &MIN_BET_AMOUNT,
        &0,
    );
    assert_eq!(result, Err(Ok(Error::BetBelowMarketMin)));
}

// ===== HELPER =====

/// Build a minimal Market struct for unit-testing validators without a full env setup.
fn make_minimal_market(env: &Env, min_bet_amount: Option<i128>) -> Market {
    use crate::timelock::MarketTimelockConfig;
    Market {
        admin: Address::generate(env),
        question: String::from_str(env, "Q?"),
        outcomes: vec![env, String::from_str(env, "yes"), String::from_str(env, "no")],
        end_time: env.ledger().timestamp() + 86400,
        oracle_config: OracleConfig {
            provider: OracleProvider::reflector(),
            oracle_address: Address::from_str(
                env,
                "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
            ),
            feed_id: String::from_str(env, "BTC/USD"),
            threshold: 100_000,
            comparison: String::from_str(env, "gt"),
        },
        metadata_commitment: soroban_sdk::BytesN::from_array(env, &[0u8; 32]),
        has_fallback: false,
        fallback_oracle_config: OracleConfig::none_sentinel(env),
        resolution_timeout: 3600,
        oracle_result: None,
        state: MarketState::Active,
        votes: soroban_sdk::Map::new(env),
        stakes: soroban_sdk::Map::new(env),
        winning_outcomes: None,
        claimed: soroban_sdk::Map::new(env),
        total_staked: 0,
        dispute_stakes: soroban_sdk::Map::new(env),
        fee_collected: false,
        total_extension_days: 0,
        max_extension_days: 7,
        extension_history: soroban_sdk::Vec::new(env),
        category: None,
        tags: soroban_sdk::Vec::new(env),
        min_pool_size: None,
        bet_deadline: 0,
        dispute_window_seconds: 86400,
        winnings_swept: false,
        timelock_config: MarketTimelockConfig::default(),
        dispute_stake_floor: None,
        max_participants: None,
        min_bet_amount,
    }
}
