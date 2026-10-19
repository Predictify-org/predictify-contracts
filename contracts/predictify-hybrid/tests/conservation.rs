use predictify_hybrid::types::{OracleConfig, OracleProvider, ReflectorAsset};
use predictify_hybrid::{PredictifyHybrid, PredictifyHybridClient};
use soroban_sdk::testutils::{Address as _, Ledger, LedgerInfo};
use soroban_sdk::token::StellarAssetClient;
use soroban_sdk::{Address, Env, String, Symbol};

const ORACLE_ADDRESS: &str = "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";
const INITIAL_BALANCE: i128 = 1_000_000_000;

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
    client.initialize(&admin, &None, &None);

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
        &100_000_000i128,
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

    assert_eq!(first_after_first_bet, 100_000_000);
    assert_eq!(second_after_first_bet, 0);

    client.place_bet(
        &user,
        &second_market,
        &String::from_str(&env, "no"),
        &250_000_000i128,
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

    assert_eq!(first_after_second_bet, 100_000_000);
    assert_eq!(second_after_second_bet, 250_000_000);
    assert_eq!(
        first_after_second_bet + second_after_second_bet,
        350_000_000,
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

    // Winning stakes are 1M and 2M, losing stake 97M. After the default 2%
    // fee, floored payouts are 32,666,666 and 65,333,333: remainder = 1.
    for (bettor, outcome, amount) in [
        (&user, "yes", 1_000_000i128),
        (&user2, "yes", 2_000_000i128),
        (&user3, "no", 97_000_000i128),
    ] {
        client.place_bet(bettor, &market, &String::from_str(&env, outcome), &amount, &1000);
    }

    assert_eq!(client.get_market(&market).unwrap().total_staked, 100_000_000);
    let token = soroban_sdk::token::Client::new(&env, &token_id);
    assert_eq!(token.balance(&contract_id), 100_000_000);

    advance_ledger(&env, 31 * 24 * 60 * 60);
    client.resolve_market_manual(&admin, &market, &String::from_str(&env, "yes"));
    // Bets use batch distribution, which credits withdrawable internal balances.
    let distributed = client.distribute_payouts(&market);
    assert_eq!(distributed, 97_999_999);
    let fee = client.collect_fees(&admin, &market);
    assert_eq!(fee, 2_000_000);
    let vault: i128 = env.as_contract(&contract_id, || {
        env.storage().persistent().get(&Symbol::new(&env, "tot_fees")).unwrap()
    });
    assert_eq!(vault, fee + 1, "rounding remainder is accounted for in the fee vault");

    for (winner, expected) in [(&user, 32_666_666i128), (&user2, 65_333_333i128)] {
        assert_eq!(client.get_balance(winner, &ReflectorAsset::Stellar).amount, expected);
        let before = token.balance(winner);
        client.withdraw(winner, &ReflectorAsset::Stellar, &expected);
        assert_eq!(token.balance(winner) - before, expected);
        assert_eq!(client.get_balance(winner, &ReflectorAsset::Stellar).amount, 0);
    }
    assert_eq!(distributed + vault, 100_000_000, "all stakes remain accounted for");
    assert_eq!(token.balance(&contract_id), vault, "only tracked fees remain in custody");
    assert_eq!(client.distribute_payouts(&market), 0, "payout replay cannot create new credits");
    assert_eq!(client.collect_fees(&admin, &market), 0, "fee replay cannot duplicate the remainder");
    assert_eq!(env.as_contract(&contract_id, || env.storage().persistent().get::<_, i128>(&Symbol::new(&env, "tot_fees")).unwrap()), vault);
}
