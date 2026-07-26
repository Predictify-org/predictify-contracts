#![cfg(test)]

use soroban_sdk::{testutils::{Address as _, Ledger}, Address, Env, Symbol, String};
use crate::PredictifyHybridContract;
use crate::PredictifyHybridContractClient;
use crate::err::Error;

fn setup_test() -> (Env, PredictifyHybridContractClient<'static>, Address) {
    let env = Env::default();
    let contract_id = env.register_contract(None, PredictifyHybridContract);
    let client = PredictifyHybridContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    
    // Setup admin
    client.initialize(
        &admin,
        &String::from_str(&env, "Test Token"),
        &String::from_str(&env, "TTK"),
        &18,
    );

    (env, client, admin)
}

#[test]
fn test_betting_admin_cooldown() {
    let (env, client, admin) = setup_test();
    
    env.mock_all_auths();

    // Set betting admin cooldown to 100 seconds
    client.set_betting_admin_cooldown(&admin, &100);

    // First call to set_global_bet_limits should succeed
    client.set_global_bet_limits(&admin, &100_000000i128, &10_000_000000i128);

    // Doing any critical action immediately again should fail
    let res = client.try_set_global_bet_limits(
        &admin,
        &100_000000i128,
        &10_000_000000i128,
    );
    assert_eq!(res.unwrap_err().unwrap(), Error::BettingAdminCooldownActive);

    // Advance time by 50 seconds (cooldown still active)
    env.ledger().with_mut(|l| l.timestamp += 50);

    let res2 = client.try_set_global_bet_limits(
        &admin,
        &100_000000i128,
        &10_000_000000i128,
    );
    assert_eq!(res2.unwrap_err().unwrap(), Error::BettingAdminCooldownActive);

    // Advance time by another 51 seconds (total 101, cooldown elapsed)
    env.ledger().with_mut(|l| l.timestamp += 51);

    client.set_global_bet_limits(&admin, &100_000000i128, &10_000_000000i128); // Should succeed
}

#[test]
fn test_betting_admin_cooldown_cross_functions() {
    let (env, client, admin) = setup_test();
    
    env.mock_all_auths();

    // Set betting admin cooldown to 60 seconds
    client.set_betting_admin_cooldown(&admin, &60);

    // Call set_global_bet_limits
    client.set_global_bet_limits(&admin, &100_000000i128, &10_000_000000i128);

    // Call set_event_bet_limits immediately on a market (should fail)
    let market_id = Symbol::new(&env, "m1");
    let res = client.try_set_event_bet_limits(
        &admin,
        &market_id,
        &100_000000i128,
        &10_000_000000i128,
    );
    assert_eq!(res.unwrap_err().unwrap(), Error::BettingAdminCooldownActive);

    // Advance time by 61 seconds
    env.ledger().with_mut(|l| l.timestamp += 61);

    // Now call set_event_bet_limits (should succeed)
    client.set_event_bet_limits(
        &admin,
        &market_id,
        &100_000000i128,
        &10_000_000000i128,
    );

    // Call set_market_max_bet_cap immediately (should fail)
    let res2 = client.try_set_market_max_bet_cap(&admin, &market_id, &5_000_000000i128);
    assert_eq!(res2.unwrap_err().unwrap(), Error::BettingAdminCooldownActive);
}
