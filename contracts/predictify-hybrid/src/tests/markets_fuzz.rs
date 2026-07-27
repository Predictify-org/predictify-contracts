//! Proptest-based fuzz target for market creation boundary cases.
//!
//! Exercises the `create_market` entrypoint with property-based strategies
//! that explore edge conditions around question formatting/length, outcome count
//! and duplicate checks, and duration limits.

use proptest::prelude::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, ConversionError, Env, Error as HostError, InvokeError, String as SorobanString,
    Symbol, Vec as SorobanVec,
};

use crate::{
    err::Error,
    types::{OracleConfig, OracleProvider},
    PredictifyHybrid, PredictifyHybridClient,
};

type TryCreateResult<T> = Result<Result<T, ConversionError>, Result<HostError, InvokeError>>;

fn assert_contract_error<T: core::fmt::Debug>(result: TryCreateResult<T>, expected: Error) {
    let expected_error = HostError::from(soroban_sdk::xdr::ScError::Contract(expected as u32));
    match result {
        Err(Ok(err)) => assert_eq!(err, expected_error),
        other => panic!("expected contract error {:?}, got {:?}", expected, other),
    }
}

struct TestSetup {
    env: Env,
    contract_id: Address,
    admin: Address,
}

impl TestSetup {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let contract_id = env.register(PredictifyHybrid, ());

        let token_admin = Address::generate(&env);
        let token_contract = env.register_stellar_asset_contract_v2(token_admin);
        let token_id = token_contract.address();

        env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .set(&Symbol::new(&env, "TokenID"), &token_id);
        });

        let client = PredictifyHybridClient::new(&env, &contract_id);
        client.initialize(&admin, &None, &None);
        env.as_contract(&contract_id, || {
            crate::circuit_breaker::CircuitBreaker::initialize(&env)
                .expect("circuit breaker should initialize in tests");
        });

        Self {
            env,
            contract_id,
            admin,
        }
    }

    fn client(&self) -> PredictifyHybridClient<'_> {
        PredictifyHybridClient::new(&self.env, &self.contract_id)
    }

    fn valid_oracle_config(&self) -> OracleConfig {
        OracleConfig::new(
            OracleProvider::reflector(),
            Address::generate(&self.env),
            SorobanString::from_str(&self.env, "BTC/USD"),
            50_000_00,
            SorobanString::from_str(&self.env, "gt"),
        )
    }

    fn valid_outcomes(&self) -> SorobanVec<SorobanString> {
        soroban_sdk::vec![
            &self.env,
            SorobanString::from_str(&self.env, "Yes"),
            SorobanString::from_str(&self.env, "No"),
        ]
    }
}

// ===========================================================================
// proptest strategies
// ===========================================================================

/// Strategy for invalid question strings.
fn invalid_question_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        // Too short (0 to 9 characters)
        1 => prop::string::string_regex("[a-zA-Z0-9 ]{0,9}").unwrap(),
        // Whitespace only
        1 => prop::string::string_regex(" +").unwrap(),
        // Too long (501 to 600 characters)
        1 => prop::string::string_regex("[a-zA-Z0-9 ]{501,600}").unwrap(),
    ]
}

/// Strategy for valid question strings.
fn valid_question_strategy() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-zA-Z0-9 ]{10,200}").unwrap()
}

/// Strategy for invalid outcome sets.
fn invalid_outcomes_strategy() -> impl Strategy<Value = Vec<String>> {
    prop_oneof![
        // Fewer than 2 outcomes
        1 => prop::collection::vec(prop::string::string_regex("[a-zA-Z0-9]{2,10}").unwrap(), 0..=1),
        // More than 10 outcomes
        1 => prop::collection::vec(prop::string::string_regex("[a-zA-Z0-9]{2,10}").unwrap(), 11..=15),
        // One outcome is empty or whitespace only
        1 => prop::collection::vec(prop::string::string_regex("[a-zA-Z0-9]{2,10}").unwrap(), 2..=5)
            .prop_map(|mut v| {
                v.push("   ".to_string());
                v
            }),
        // Contains duplicate outcomes (case-insensitive)
        1 => prop::collection::vec(prop::string::string_regex("[a-zA-Z0-9]{2,10}").unwrap(), 2..=5)
            .prop_map(|mut v| {
                if !v.is_empty() {
                    let dup = v[0].clone();
                    v.push(dup.to_uppercase());
                } else {
                    v.push("dup".to_string());
                    v.push("DUP".to_string());
                }
                v
            }),
    ]
}

/// Strategy for valid outcome sets.
fn valid_outcomes_strategy() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(
        prop::string::string_regex("[a-zA-Z0-9]{2,10}").unwrap(),
        2..=10,
    )
    .prop_filter("unique outcomes", |v| {
        let mut unique = std::collections::HashSet::new();
        for item in v {
            let normalized = item.trim().to_lowercase();
            if !unique.insert(normalized) {
                return false;
            }
        }
        true
    })
}

/// Strategy for invalid market duration.
fn invalid_duration_strategy() -> impl Strategy<Value = u32> {
    prop_oneof![
        1 => Just(0u32),
        1 => 366u32..10_000u32,
    ]
}

/// Strategy for valid market duration.
fn valid_duration_strategy() -> impl Strategy<Value = u32> {
    1u32..=365u32
}

// ===========================================================================
// Fuzz targets (proptest)
// ===========================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Verify that invalid market questions are always rejected.
    #[test]
    fn fuzz_create_market_invalid_question(
        question in invalid_question_strategy(),
    ) {
        let setup = TestSetup::new();
        let question_soroban = SorobanString::from_str(&setup.env, &question);

        let result = setup.client().try_create_market(
            &setup.admin,
            &question_soroban,
            &setup.valid_outcomes(),
            &30u32,
            &setup.valid_oracle_config(),
            &None,
            &86_400u64,
        );

        assert_contract_error(result, Error::InvalidQuestion);
    }

    /// Verify that invalid outcome configurations are always rejected.
    #[test]
    fn fuzz_create_market_invalid_outcomes(
        outcomes in invalid_outcomes_strategy(),
    ) {
        let setup = TestSetup::new();
        let mut outcomes_soroban = SorobanVec::new(&setup.env);
        for o in outcomes {
            outcomes_soroban.push_back(SorobanString::from_str(&setup.env, &o));
        }

        let result = setup.client().try_create_market(
            &setup.admin,
            &SorobanString::from_str(&setup.env, "Will outcomes validation work?"),
            &outcomes_soroban,
            &30u32,
            &setup.valid_oracle_config(),
            &None,
            &86_400u64,
        );

        assert_contract_error(result, Error::InvalidOutcomes);
    }

    /// Verify that invalid duration configurations are always rejected.
    #[test]
    fn fuzz_create_market_invalid_duration(
        duration in invalid_duration_strategy(),
    ) {
        let setup = TestSetup::new();

        let result = setup.client().try_create_market(
            &setup.admin,
            &SorobanString::from_str(&setup.env, "Will duration validation work?"),
            &setup.valid_outcomes(),
            &duration,
            &setup.valid_oracle_config(),
            &None,
            &86_400u64,
        );

        assert_contract_error(result, Error::InvalidDuration);
    }

    /// Verify that valid market parameters succeed in market creation.
    #[test]
    fn fuzz_create_market_success(
        question in valid_question_strategy(),
        outcomes in valid_outcomes_strategy(),
        duration in valid_duration_strategy(),
    ) {
        let setup = TestSetup::new();
        let question_soroban = SorobanString::from_str(&setup.env, &question);

        let mut outcomes_soroban = SorobanVec::new(&setup.env);
        for o in outcomes {
            outcomes_soroban.push_back(SorobanString::from_str(&setup.env, &o));
        }

        let result = setup.client().try_create_market(
            &setup.admin,
            &question_soroban,
            &outcomes_soroban,
            &duration,
            &setup.valid_oracle_config(),
            &None,
            &86_400u64,
        );

        prop_assert!(result.is_ok(), "Market creation failed with valid params: {:?}", result);
    }
}
