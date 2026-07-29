use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{Env, Address, BytesN, String, Vec, Symbol};

use crate::{PredictifyHybridClient, PredictifyHybrid};
use crate::audit_trail::AuditTrailHead;

#[test]
fn test_create_market_rejected_on_ttl_pressure() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, PredictifyHybrid);
    let client = PredictifyHybridClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    // initialize so admin is set
    let _ = client.initialize(&admin, &None, &None);

    // Pre-populate an audit head record so the TTL probe sees an existing key
    let head = AuditTrailHead {
        latest_index: 1,
        latest_hash: BytesN::from_array(&env, &[0u8; 32]),
    };
    env.storage().persistent().set(&Symbol::new(&env, "AUDIT_HEAD"), &head);

    let question = String::from_str(&env, "Will it rain?");
    let outcomes = Vec::new(&env);
    outcomes.push_back(String::from_str(&env, "Yes"));
    outcomes.push_back(String::from_str(&env, "No"));

    // Call create_market via the generated client; expect a storage rent budget error
    let res = client.try_create_market(
        &admin,
        &question,
        &outcomes,
        &1u32,
        &crate::types::OracleConfig::none_sentinel(&env),
        &None,
        &86400u64,
        &None,
        &None,
        &None,
        &None,
        &None,
    );

    assert!(matches!(res, Err(crate::Error::InsufficientStorageRentBudget)));
}
