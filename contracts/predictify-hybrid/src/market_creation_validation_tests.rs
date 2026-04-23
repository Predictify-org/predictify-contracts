use crate::errors::Error;
use crate::types::{OracleConfig, OracleProvider};
use crate::{PredictifyHybrid, PredictifyHybridClient};
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{
    vec, Address, ConversionError, Env, Error as HostError, InvokeError, String, Symbol, Vec,
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
        client.initialize(&admin, &None);
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
            String::from_str(&self.env, "BTC/USD"),
            50_000_00,
            String::from_str(&self.env, "gt"),
        )
    }

    fn valid_outcomes(&self) -> Vec<String> {
        vec![
            &self.env,
            String::from_str(&self.env, "Yes"),
            String::from_str(&self.env, "No"),
        ]
    }
}

#[test]
fn create_market_rejects_question_shorter_than_minimum() {
    let setup = TestSetup::new();
    let result = setup.client().try_create_market(
        &setup.admin,
        &String::from_str(&setup.env, "Too short"),
        &setup.valid_outcomes(),
        &30u32,
        &setup.valid_oracle_config(),
        &None,
        &86_400u64,
    );

    assert_contract_error(result, Error::InvalidQuestion);
}

#[test]
fn create_market_rejects_whitespace_only_question() {
    let setup = TestSetup::new();
    let result = setup.client().try_create_market(
        &setup.admin,
        &String::from_str(&setup.env, "          "),
        &setup.valid_outcomes(),
        &30u32,
        &setup.valid_oracle_config(),
        &None,
        &86_400u64,
    );

    assert_contract_error(result, Error::InvalidQuestion);
}

#[test]
fn create_market_rejects_outcome_count_out_of_bounds() {
    let setup = TestSetup::new();
    let too_few = vec![&setup.env, String::from_str(&setup.env, "Only one")];
    let too_many = vec![
        &setup.env,
        String::from_str(&setup.env, "One"),
        String::from_str(&setup.env, "Two"),
        String::from_str(&setup.env, "Three"),
        String::from_str(&setup.env, "Four"),
        String::from_str(&setup.env, "Five"),
        String::from_str(&setup.env, "Six"),
        String::from_str(&setup.env, "Seven"),
        String::from_str(&setup.env, "Eight"),
        String::from_str(&setup.env, "Nine"),
        String::from_str(&setup.env, "Ten"),
        String::from_str(&setup.env, "Eleven"),
    ];

    let too_few_result = setup.client().try_create_market(
        &setup.admin,
        &String::from_str(&setup.env, "Will the count bounds be enforced?"),
        &too_few,
        &30u32,
        &setup.valid_oracle_config(),
        &None,
        &86_400u64,
    );
    assert_contract_error(too_few_result, Error::InvalidOutcomes);

    let too_many_result = setup.client().try_create_market(
        &setup.admin,
        &String::from_str(&setup.env, "Will the count bounds be enforced?"),
        &too_many,
        &30u32,
        &setup.valid_oracle_config(),
        &None,
        &86_400u64,
    );
    assert_contract_error(too_many_result, Error::InvalidOutcomes);
}

#[test]
fn create_market_rejects_blank_duplicate_and_ambiguous_outcomes() {
    let setup = TestSetup::new();
    let blank = vec![
        &setup.env,
        String::from_str(&setup.env, "Yes"),
        String::from_str(&setup.env, "   "),
    ];
    let duplicate = vec![
        &setup.env,
        String::from_str(&setup.env, "Yes"),
        String::from_str(&setup.env, " yes "),
    ];
    let ambiguous = vec![
        &setup.env,
        String::from_str(&setup.env, "Maybe"),
        String::from_str(&setup.env, "Possibly"),
    ];

    let blank_result = setup.client().try_create_market(
        &setup.admin,
        &String::from_str(&setup.env, "Will blank outcomes be rejected?"),
        &blank,
        &30u32,
        &setup.valid_oracle_config(),
        &None,
        &86_400u64,
    );
    assert_contract_error(blank_result, Error::InvalidOutcomes);

    let duplicate_result = setup.client().try_create_market(
        &setup.admin,
        &String::from_str(&setup.env, "Will duplicate outcomes be rejected?"),
        &duplicate,
        &30u32,
        &setup.valid_oracle_config(),
        &None,
        &86_400u64,
    );
    assert_contract_error(duplicate_result, Error::InvalidOutcomes);

    let ambiguous_result = setup.client().try_create_market(
        &setup.admin,
        &String::from_str(&setup.env, "Will ambiguous outcomes be rejected?"),
        &ambiguous,
        &30u32,
        &setup.valid_oracle_config(),
        &None,
        &86_400u64,
    );
    assert_contract_error(ambiguous_result, Error::InvalidOutcomes);
}

#[test]
fn create_market_rejects_duration_out_of_bounds() {
    let setup = TestSetup::new();
    let question = String::from_str(&setup.env, "Will duration limits be enforced?");
    let outcomes = setup.valid_outcomes();
    let oracle = setup.valid_oracle_config();

    let too_short = setup.client().try_create_market(
        &setup.admin,
        &question,
        &outcomes,
        &0u32,
        &oracle,
        &None,
        &86_400u64,
    );
    assert_contract_error(too_short, Error::InvalidDuration);

    let too_long = setup.client().try_create_market(
        &setup.admin,
        &question,
        &outcomes,
        &366u32,
        &oracle,
        &None,
        &86_400u64,
    );
    assert_contract_error(too_long, Error::InvalidDuration);
}

#[test]
fn create_market_accepts_valid_boundary_inputs() {
    let setup = TestSetup::new();
    let market_id = setup.client().create_market(
        &setup.admin,
        &String::from_str(&setup.env, "1234567890"),
        &setup.valid_outcomes(),
        &1u32,
        &setup.valid_oracle_config(),
        &None,
        &86_400u64,
    );

    let market = setup.client().get_market(&market_id).unwrap();
    assert_eq!(market.question, String::from_str(&setup.env, "1234567890"));
    assert_eq!(market.outcomes.len(), 2);
}

#[test]
fn create_event_reuses_shared_description_and_outcome_validation() {
    let setup = TestSetup::new();
    let duplicate = vec![
        &setup.env,
        String::from_str(&setup.env, "Up"),
        String::from_str(&setup.env, " up "),
    ];

    let blank_description = setup.client().try_create_event(
        &setup.admin,
        &String::from_str(&setup.env, "          "),
        &setup.valid_outcomes(),
        &(setup.env.ledger().timestamp() + 86_400),
        &setup.valid_oracle_config(),
        &None,
        &86_400u64,
    );
    assert_contract_error(blank_description, Error::InvalidQuestion);

    let duplicate_outcomes = setup.client().try_create_event(
        &setup.admin,
        &String::from_str(&setup.env, "Will event creation share outcome validation?"),
        &duplicate,
        &(setup.env.ledger().timestamp() + 86_400),
        &setup.valid_oracle_config(),
        &None,
        &86_400u64,
    );
    assert_contract_error(duplicate_outcomes, Error::InvalidOutcomes);
}

fn create_event_rejects_past_end_time() {
    let setup = TestSetup::new();
    setup.env.ledger().with_mut(|li| {
        li.timestamp = 1_000;
    });

    let result = setup.client().try_create_event(
        &setup.admin,
        &String::from_str(&setup.env, "Will past end times be rejected?"),
        &setup.valid_outcomes(),
        &1_000u64,
        &setup.valid_oracle_config(),
        &None,
        &86_400u64,
    );

    assert_contract_error(result, Error::InvalidDuration);
}
