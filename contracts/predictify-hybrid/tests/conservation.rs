use predictify_hybrid::types::{OracleConfig, OracleProvider};
use predictify_hybrid::{PredictifyHybrid, PredictifyHybridClient};
use soroban_sdk::testutils::{Address as _, Ledger, LedgerInfo};
use soroban_sdk::token::StellarAssetClient;
use soroban_sdk::{Address, Env, String, Symbol};

const ORACLE_ADDRESS: &str = "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";
const INITIAL_BALANCE: i128 = 1_000;

fn setup() -> (Env, Address, Address, Address, Address) {
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
    client.initialize(&admin, &Some(0i128), &None);

    StellarAssetClient::new(&env, &token_id).mint(&user, &INITIAL_BALANCE);

    (env, contract_id, admin, user, token_id)
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
    client: &PredictifyHybridClient,
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
        &None,
        &None,
        &None,
        &None,
        &None,
    )
}

fn advance_ledger(env: &Env, seconds: u64) {
    let ledger = env.ledger();
    let timestamp = ledger.timestamp() + seconds;
    env.ledger().set(LedgerInfo {
        timestamp,
        protocol_version: ledger.protocol_version(),
        sequence_number: ledger.sequence(),
        network_id: ledger.network_id().into(),
        base_reserve: 10,
        min_temp_entry_ttl: 1,
        min_persistent_entry_ttl: 1,
        max_entry_ttl: 1_000_000,
    });
}

#[test]
fn stake_is_conserved_independently_per_market() {
    let (env, contract_id, admin, user, _token_id) = setup();
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

    client.place_bet(
        &user,
        &first_market,
        &String::from_str(&env, "yes"),
        &100i128,
        &1000,
    );
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

    client.place_bet(
        &user,
        &second_market,
        &String::from_str(&env, "no"),
        &250i128,
        &1000,
    );
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

#[test]
fn payout_remainder_is_conserved() {
    let (env, contract_id, admin, user, token_id) = setup();
    let client = PredictifyHybridClient::new(&env, &contract_id);

    let user2 = Address::generate(&env);
    let user3 = Address::generate(&env);
    StellarAssetClient::new(&env, &token_id).mint(&user2, &INITIAL_BALANCE);
    StellarAssetClient::new(&env, &token_id).mint(&user3, &INITIAL_BALANCE);

    let market = create_market(
        &client,
        &env,
        &admin,
        "Will the payout remainder be conserved?",
        "BTC",
    );

    // Winning stakes: user=1, user2=2, total winning pool=3.
    // Losing stake: user3=97, total staked=100.
    // Proportional payout floors to:
    //   user  = 1 * 100 / 3 = 33
    //   user2 = 2 * 100 / 3 = 66
    // Remainder = 1, which must be allocated to the admin.
    client.place_bet(
        &user,
        &market,
        &String::from_str(&env, "yes"),
        &1i128,
        &1000,
    );
    client.place_bet(
        &user2,
        &market,
        &String::from_str(&env, "yes"),
        &2i128,
        &1000,
    );
    client.place_bet(
        &user3,
        &market,
        &String::from_str(&env, "no"),
        &97i128,
        &1000,
    );

    let market_data = client.get_market(&market).expect("market should exist");
    assert_eq!(market_data.total_staked, 100);

    let user_balance_before = StellarAssetClient::new(&env, &token_id).balance(&user);
    let user2_balance_before = StellarAssetClient::new(&env, &token_id).balance(&user2);
    let admin_balance_before = StellarAssetClient::new(&env, &token_id).balance(&admin);
    let contract_balance_before = StellarAssetClient::new(&env, &token_id).balance(&contract_id);
    assert_eq!(contract_balance_before, 100);

    advance_ledger(&env, 31 * 24 * 60 * 60);
    client.resolve_market_manual(&admin, &market, &String::from_str(&env, "yes"));

    client.claim_winnings(&user, &market, &0u64);
    client.claim_winnings(&user2, &market, &0u64);

    let user_claimed = StellarAssetClient::new(&env, &token_id).balance(&user) - user_balance_before;
    let user2_claimed =
        StellarAssetClient::new(&env, &token_id).balance(&user2) - user2_balance_before;
    let admin_claimed =
        StellarAssetClient::new(&env, &token_id).balance(&admin) - admin_balance_before;
    let contract_balance_after = StellarAssetClient::new(&env, &token_id).balance(&contract_id);

    assert_eq!(user_claimed, 33, "first winner receives the floored share");
    assert_eq!(user2_claimed, 66, "second winner receives the floored share");
    assert_eq!(
        user_claimed + user2_claimed,
        99,
        "payouts sum to the distributable amount"
    );
    assert_eq!(
        admin_claimed, 1,
        "the one-wei remainder is allocated to the admin instead of being locked"
    );
    assert_eq!(
        contract_balance_after, 0,
        "no funds remain stranded in the contract"
    );
}
