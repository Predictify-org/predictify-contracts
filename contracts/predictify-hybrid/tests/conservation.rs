use predictify_hybrid::types::{OracleConfig, OracleProvider};
use predictify_hybrid::{PredictifyHybrid, PredictifyHybridClient};
use soroban_sdk::testutils::Address as _;
use soroban_sdk::token::StellarAssetClient;
use soroban_sdk::{Address, Env, String, Symbol};

const ORACLE_ADDRESS: &str = "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";
const INITIAL_BALANCE: i128 = 1_000;

fn setup() -> (Env, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
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

    StellarAssetClient::new(&env, &token_id).mint(&user, &INITIAL_BALANCE);

    (env, contract_id, admin, user)
}

fn oracle_config(env: &Env, feed_id: &str) -> OracleConfig {
    OracleConfig::new(
        OracleProvider::reflector(),
        Address::from_str(env, ORACLE_ADDRESS),
        String::from_str(env, feed_id),
        100,
        String::from_str(env, "gt"),
    )
}

fn create_market(
    client: &PredictifyHybridClient<'_>,
    env: &Env,
    admin: &Address,
    question: &str,
    feed_id: &str,
) -> Symbol {
    client.create_market(
        admin,
        &String::from_str(env, question),
        &soroban_sdk::vec![
            env,
            String::from_str(env, "yes"),
            String::from_str(env, "no"),
        ],
        &30u32,
        &oracle_config(env, feed_id),
        &None,
        &86_400u64,
    )
}

#[test]
fn stake_is_conserved_independently_per_market() {
    let (env, contract_id, admin, user) = setup();
    let client = PredictifyHybridClient::new(&env, &contract_id);

    let first_market = create_market(
        &client,
        &env,
        &admin,
        "Will BTC close above the target?",
        "BTC",
    );
    let second_market = create_market(
        &client,
        &env,
        &admin,
        "Will ETH close above the target?",
        "ETH",
    );

    client.place_bet(&user, &first_market, &0u32, &100i128);
    let first_after_first_bet = client
        .get_market(&first_market)
        .expect("first market should exist")
        .total_staked;
    let second_after_first_bet = client
        .get_market(&second_market)
        .expect("second market should exist")
        .total_staked;

    assert_eq!(first_after_first_bet, 100);
    assert_eq!(second_after_first_bet, 0);

    client.place_bet(&user, &second_market, &1u32, &250i128);
    let first_after_second_bet = client
        .get_market(&first_market)
        .expect("first market should exist")
        .total_staked;
    let second_after_second_bet = client
        .get_market(&second_market)
        .expect("second market should exist")
        .total_staked;

    assert_eq!(first_after_second_bet, 100);
    assert_eq!(second_after_second_bet, 250);
    assert_eq!(
        first_after_second_bet + second_after_second_bet,
        350,
        "the aggregate stake must equal the sum of stakes assigned to both markets"
    );
}
