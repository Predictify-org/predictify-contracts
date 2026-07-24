#![cfg(test)]

use soroban_sdk::{testutils::{Address as _, Ledger}, Address, Env, String};
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
fn test_oracle_admin_cooldown() {
    let (env, client, admin) = setup_test();
    
    env.mock_all_auths();

    // Set cooldown to 100 seconds
    client.set_oracle_admin_cooldown(&admin, &100);

    client.set_oracle_val_cfg_global(
        &admin,
        &3600, // max_staleness_secs
        &1000, // max_confidence_bps
        &None, // max_deviation_bps
    );
    
    // Doing it immediately again should fail
    let res = client.try_set_oracle_val_cfg_global(
        &admin,
        &3600,
        &1000,
        &None,
    );
    
    assert_eq!(res.unwrap_err().unwrap(), Error::OracleAdminCooldownActive);
    
    // Advance time by 50 seconds (not enough)
    env.ledger().with_mut(|l| l.timestamp += 50);
    
    let res2 = client.try_set_oracle_val_cfg_global(
        &admin,
        &3600,
        &1000,
        &None,
    );
    
    assert_eq!(res2.unwrap_err().unwrap(), Error::OracleAdminCooldownActive);
    
    // Advance time by another 51 seconds (total 101, enough)
    env.ledger().with_mut(|l| l.timestamp += 51);
    
    client.set_oracle_val_cfg_global(
        &admin,
        &3600,
        &1000,
        &None,
    ); // Should succeed
}
