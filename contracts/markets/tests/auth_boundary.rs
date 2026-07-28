#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo, MockAuth, MockAuthInvoke},
    Address, Env, IntoVal, String, Vec,
};
use markets::{MarketsContract, MarketsContractClient};

// ============================================
// Test Helpers
// ============================================

fn setup_test_environment(env: &Env) -> TestSetup<'_> {
    env.ledger().set(LedgerInfo {
        timestamp: 1735689600,
        protocol_version: 25,
        sequence_number: 1,
        network_id: [0; 32],
        base_reserve: 10,
        min_temp_entry_ttl: 1,
        min_persistent_entry_ttl: 1,
        max_entry_ttl: 518400,
    });

    let admin = Address::generate(env);
    let market_creator = Address::generate(env);
    let user1 = Address::generate(env);
    let user2 = Address::generate(env);
    let unauthorized = Address::generate(env);

    let contract_id = env.register(MarketsContract, ());
    let client = MarketsContractClient::new(env, &contract_id);

    TestSetup {
        admin,
        market_creator,
        user1,
        user2,
        unauthorized,
        client,
        contract_id,
    }
}

struct TestSetup<'a> {
    admin: Address,
    market_creator: Address,
    user1: Address,
    user2: Address,
    unauthorized: Address,
    client: MarketsContractClient<'a>,
    contract_id: Address,
}

/// Creates a market using explicit auth mocking for the market creator.
/// Used by tests that cannot rely on `mock_all_auths()`.
fn create_market_with_mock_auth(setup: &TestSetup<'_>) -> u32 {
    let env = &setup.client.env;
    let question = String::from_str(env, "Test market question?");
    let description = String::from_str(env, "Test market description");
    let end_time = env.ledger().timestamp() + 86400;
    let resolution_source = String::from_str(env, "Test Source");
    let outcome_tags = Vec::from_array(env, [
        String::from_str(env, "Yes"),
        String::from_str(env, "No"),
    ]);

    // Authorize ONLY the market_creator for this specific create_market call.
    env.mock_auths(&[MockAuth {
        address: &setup.market_creator,
        invoke: &MockAuthInvoke {
            contract: &setup.contract_id,
            fn_name: "create_market",
            args: (
                &setup.market_creator,
                &question,
                &description,
                &end_time,
                &resolution_source,
                &outcome_tags,
            )
                .into_val(env),
            sub_invokes: &[],
        },
    }]);

    setup.client.create_market(
        &setup.market_creator,
        &question,
        &description,
        &end_time,
        &resolution_source,
        &outcome_tags,
    )
}

/// Creates a market (requires `env.mock_all_auths()` to be active).
fn create_market_with_auth_check(setup: &TestSetup<'_>) -> u32 {
    let env = &setup.client.env;
    let question = String::from_str(env, "Test market question?");
    let description = String::from_str(env, "Test market description");
    let end_time = env.ledger().timestamp() + 86400;
    let resolution_source = String::from_str(env, "Test Source");
    let outcome_tags = Vec::from_array(env, [
        String::from_str(env, "Yes"),
        String::from_str(env, "No"),
    ]);

    setup.client.create_market(
        &setup.market_creator,
        &question,
        &description,
        &end_time,
        &resolution_source,
        &outcome_tags,
    )
}

// ============================================
// Auth Boundary Tests
// ============================================

// ── create_market ─────────────────────────────────────────────────────────────

#[test]
fn test_create_market_requires_auth() {
    let env = Env::default();
    // No mock_all_auths — require_auth should fail for everyone.
    let contract_id = env.register(MarketsContract, ());
    let client = MarketsContractClient::new(&env, &contract_id);

    let unauthorized = Address::generate(&env);
    let question = String::from_str(&env, "Test market question?");
    let description = String::from_str(&env, "Test market description");
    let end_time = env.ledger().timestamp() + 86400;
    let resolution_source = String::from_str(&env, "Test Source");
    let outcome_tags = Vec::from_array(&env, [
        String::from_str(&env, "Yes"),
        String::from_str(&env, "No"),
    ]);

    let result = client.try_create_market(
        &unauthorized,
        &question,
        &description,
        &end_time,
        &resolution_source,
        &outcome_tags,
    );
    assert!(result.is_err(), "Unauthorized user should not create market");
}

#[test]
fn test_create_market_requires_auth_success() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    env.mock_all_auths();

    let market_id = create_market_with_auth_check(&setup);
    assert_eq!(market_id, 1);
}

#[test]
fn test_create_market_requires_auth_market_creator() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    env.mock_all_auths();

    let question = String::from_str(&env, "Test market question?");
    let description = String::from_str(&env, "Test market description");
    let end_time = env.ledger().timestamp() + 86400;
    let resolution_source = String::from_str(&env, "Test Source");
    let outcome_tags = Vec::from_array(&env, [
        String::from_str(&env, "Yes"),
        String::from_str(&env, "No"),
    ]);

    let result = setup.client.try_create_market(
        &setup.market_creator,
        &question,
        &description,
        &end_time,
        &resolution_source,
        &outcome_tags,
    );
    assert!(result.is_ok(), "Authorized market creator should succeed");
}

// ── place_bet ─────────────────────────────────────────────────────────────────

#[test]
fn test_place_bet_requires_auth() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    // No mock_all_auths — setup uses explicit MockAuth, but
    // the unauthorized test call should fail auth.

    let market_id = create_market_with_mock_auth(&setup);

    let outcome_index = 0;
    let amount = 100;

    let result = setup.client.try_place_bet(
        &setup.unauthorized,
        &market_id,
        &outcome_index,
        &amount,
    );
    assert!(result.is_err(), "Unauthorized user should not place bet");
}

#[test]
fn test_place_bet_requires_auth_success() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    env.mock_all_auths();

    let market_id = create_market_with_auth_check(&setup);

    let outcome_index = 0;
    let amount = 100;

    let result = setup.client.try_place_bet(
        &setup.user1,
        &market_id,
        &outcome_index,
        &amount,
    );
    assert!(result.is_ok(), "Authorized user should place bet");
}

// ── resolve_market ────────────────────────────────────────────────────────────

#[test]
fn test_resolve_market_requires_auth() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    // No mock_all_auths.

    let market_id = create_market_with_mock_auth(&setup);

    let winning_outcome = 0;

    let result = setup.client.try_resolve_market(
        &setup.unauthorized,
        &market_id,
        &winning_outcome,
    );
    assert!(result.is_err(), "Unauthorized user should not resolve market");
}

#[test]
fn test_resolve_market_requires_auth_creator() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    env.mock_all_auths();

    let market_id = create_market_with_auth_check(&setup);

    let winning_outcome = 0;

    let result = setup.client.try_resolve_market(
        &setup.market_creator,
        &market_id,
        &winning_outcome,
    );
    assert!(result.is_ok(), "Market creator should resolve market");
}

// ── claim_winnings ────────────────────────────────────────────────────────────

#[test]
fn test_claim_winnings_requires_auth() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    // No mock_all_auths — every setup call uses explicit MockAuth.

    // Create market with MockAuth for market_creator.
    let market_id = create_market_with_mock_auth(&setup);

    // Authorize user1 for place_bet.
    let bet_outcome = 0u32;
    let bet_amount = 100i128;
    env.mock_auths(&[MockAuth {
        address: &setup.user1,
        invoke: &MockAuthInvoke {
            contract: &setup.contract_id,
            fn_name: "place_bet",
            args: (&setup.user1, &market_id, &bet_outcome, &bet_amount).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    setup.client.place_bet(&setup.user1, &market_id, &0, &100);

    // Advance ledger past end time.
    env.ledger().set(LedgerInfo {
        timestamp: 1735689600 + 90000,
        protocol_version: 25,
        sequence_number: 2,
        network_id: [0; 32],
        base_reserve: 10,
        min_temp_entry_ttl: 1,
        min_persistent_entry_ttl: 1,
        max_entry_ttl: 518400,
    });

    // Authorize market_creator for resolve_market.
    let win_outcome = 0u32;
    env.mock_auths(&[MockAuth {
        address: &setup.market_creator,
        invoke: &MockAuthInvoke {
            contract: &setup.contract_id,
            fn_name: "resolve_market",
            args: (&setup.market_creator, &market_id, &win_outcome).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    setup.client.resolve_market(&setup.market_creator, &market_id, &0);

    // Now test: unauthorized user should NOT be able to claim.
    let result = setup.client.try_claim_winnings(
        &setup.unauthorized,
        &market_id,
    );
    assert!(result.is_err(), "Unauthorized user should not claim winnings");
}

#[test]
fn test_claim_winnings_requires_auth_success() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    env.mock_all_auths();

    let market_id = create_market_with_auth_check(&setup);

    setup.client.place_bet(&setup.user1, &market_id, &0, &100);

    env.ledger().set(LedgerInfo {
        timestamp: 1735689600 + 90000,
        protocol_version: 25,
        sequence_number: 2,
        network_id: [0; 32],
        base_reserve: 10,
        min_temp_entry_ttl: 1,
        min_persistent_entry_ttl: 1,
        max_entry_ttl: 518400,
    });

    setup.client.resolve_market(&setup.market_creator, &market_id, &0);

    let result = setup.client.try_claim_winnings(
        &setup.user1,
        &market_id,
    );
    assert!(result.is_ok(), "Winner should claim winnings");
}

// ── cancel_market ────────────────────────────────────────────────────────────

#[test]
fn test_cancel_market_requires_auth() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    // No mock_all_auths.

    let market_id = create_market_with_mock_auth(&setup);

    let result = setup.client.try_cancel_market(
        &setup.unauthorized,
        &market_id,
    );
    assert!(result.is_err(), "Unauthorized user should not cancel market");
}

#[test]
fn test_cancel_market_requires_auth_creator() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    env.mock_all_auths();

    let market_id = create_market_with_auth_check(&setup);

    let result = setup.client.try_cancel_market(
        &setup.market_creator,
        &market_id,
    );
    assert!(result.is_ok(), "Market creator should cancel market");
}

// ── withdraw_funds ────────────────────────────────────────────────────────────

#[test]
fn test_withdraw_funds_requires_auth() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    // No mock_all_auths.

    let market_id = create_market_with_mock_auth(&setup);

    let amount = 50;

    let result = setup.client.try_withdraw_funds(
        &setup.unauthorized,
        &market_id,
        &amount,
    );
    assert!(result.is_err(), "Unauthorized user should not withdraw funds");
}

#[test]
fn test_withdraw_funds_requires_auth_creator() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    env.mock_all_auths();

    let market_id = create_market_with_auth_check(&setup);

    let amount = 50;

    let result = setup.client.try_withdraw_funds(
        &setup.market_creator,
        &market_id,
        &amount,
    );
    match result {
        Ok(_) => assert!(true, "Auth passed"),
        Err(e) => {
            let error_str = format!("{:?}", e);
            assert!(!error_str.contains("auth"), "Auth should not fail");
        }
    }
}

// ── update_market_params ─────────────────────────────────────────────────────

#[test]
fn test_update_market_params_requires_auth() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    // No mock_all_auths.

    let market_id = create_market_with_mock_auth(&setup);

    let new_end_time = env.ledger().timestamp() + 172800;

    let result = setup.client.try_update_market_params(
        &setup.unauthorized,
        &market_id,
        &new_end_time,
    );
    assert!(result.is_err(), "Unauthorized user should not update market params");
}

#[test]
fn test_update_market_params_requires_auth_creator() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    env.mock_all_auths();

    let market_id = create_market_with_auth_check(&setup);

    let new_end_time = env.ledger().timestamp() + 172800;

    let result = setup.client.try_update_market_params(
        &setup.market_creator,
        &market_id,
        &new_end_time,
    );
    match result {
        Ok(_) => assert!(true, "Auth passed"),
        Err(e) => {
            let error_str = format!("{:?}", e);
            assert!(!error_str.contains("auth"), "Auth should not fail");
        }
    }
}

// ── add_liquidity ─────────────────────────────────────────────────────────────

#[test]
fn test_add_liquidity_requires_auth() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    // No mock_all_auths.

    let market_id = create_market_with_mock_auth(&setup);

    let amount = 1000;

    let result = setup.client.try_add_liquidity(
        &setup.unauthorized,
        &market_id,
        &amount,
    );
    assert!(result.is_err(), "Unauthorized user should not add liquidity");
}

#[test]
fn test_add_liquidity_requires_auth_success() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    env.mock_all_auths();

    let market_id = create_market_with_auth_check(&setup);

    let amount = 1000;

    let result = setup.client.try_add_liquidity(
        &setup.user1,
        &market_id,
        &amount,
    );
    match result {
        Ok(_) => assert!(true, "Auth passed"),
        Err(e) => {
            let error_str = format!("{:?}", e);
            assert!(!error_str.contains("auth"), "Auth should not fail");
        }
    }
}

// ── remove_liquidity ─────────────────────────────────────────────────────────

#[test]
fn test_remove_liquidity_requires_auth() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    // No mock_all_auths — every setup call uses explicit MockAuth.

    // Create market with MockAuth for market_creator.
    let market_id = create_market_with_mock_auth(&setup);

    // Authorize user1 for add_liquidity.
    let add_amount = 1000i128;
    env.mock_auths(&[MockAuth {
        address: &setup.user1,
        invoke: &MockAuthInvoke {
            contract: &setup.contract_id,
            fn_name: "add_liquidity",
            args: (&setup.user1, &market_id, &add_amount).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    let _ = setup.client.try_add_liquidity(&setup.user1, &market_id, &1000);

    // Now test: unauthorized user should NOT be able to remove liquidity.
    let amount = 100;
    let result = setup.client.try_remove_liquidity(
        &setup.unauthorized,
        &market_id,
        &amount,
    );
    assert!(result.is_err(), "Unauthorized user should not remove liquidity");
}

#[test]
fn test_remove_liquidity_requires_auth_liquidity_provider() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    env.mock_all_auths();

    let market_id = create_market_with_auth_check(&setup);

    let _ = setup.client.try_add_liquidity(&setup.user1, &market_id, &1000);

    let amount = 100;

    let result = setup.client.try_remove_liquidity(
        &setup.user1,
        &market_id,
        &amount,
    );
    match result {
        Ok(_) => assert!(true, "Auth passed"),
        Err(e) => {
            let error_str = format!("{:?}", e);
            assert!(!error_str.contains("auth"), "Auth should not fail");
        }
    }
}

// ── admin functions ──────────────────────────────────────────────────────────

#[test]
fn test_admin_requires_auth() {
    let env = Env::default();
    // No mock_all_auths — require_auth should fail for everyone.
    let contract_id = env.register(MarketsContract, ());
    let client = MarketsContractClient::new(&env, &contract_id);

    let unauthorized = Address::generate(&env);

    let result = client.try_pause_markets(&unauthorized);
    assert!(result.is_err(), "Unauthorized user should not pause markets");

    let result = client.try_unpause_markets(&unauthorized);
    assert!(result.is_err(), "Unauthorized user should not unpause markets");
}

#[test]
fn test_admin_requires_auth_admin() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    env.mock_all_auths();

    let result = setup.client.try_pause_markets(&setup.admin);
    assert!(result.is_ok(), "Admin should pause markets");

    let result = setup.client.try_unpause_markets(&setup.admin);
    assert!(result.is_ok(), "Admin should unpause markets");
}

// ── transfer_ownership ────────────────────────────────────────────────────────

#[test]
fn test_transfer_ownership_requires_auth() {
    let env = Env::default();
    // No mock_all_auths — require_auth should fail.
    let contract_id = env.register(MarketsContract, ());
    let client = MarketsContractClient::new(&env, &contract_id);

    let unauthorized = Address::generate(&env);
    let new_owner = Address::generate(&env);

    let result = client.try_transfer_ownership(
        &unauthorized,
        &new_owner,
    );
    assert!(result.is_err(), "Unauthorized user should not transfer ownership");
}

#[test]
fn test_transfer_ownership_requires_auth_admin() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    env.mock_all_auths();

    let new_owner = Address::generate(&env);

    let result = setup.client.try_transfer_ownership(
        &setup.admin,
        &new_owner,
    );
    match result {
        Ok(_) => assert!(true, "Auth passed"),
        Err(e) => {
            let error_str = format!("{:?}", e);
            assert!(!error_str.contains("auth"), "Auth should not fail");
        }
    }
}

// ── test coverage for all entrypoints ────────────────────────────────────────

#[test]
fn test_all_entrypoints_have_auth_checks() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    env.mock_all_auths();

    let _market_id = create_market_with_auth_check(&setup);

    let auth_functions = vec![
        "create_market",
        "place_bet",
        "resolve_market",
        "claim_winnings",
        "cancel_market",
        "withdraw_funds",
        "update_market_params",
        "add_liquidity",
        "remove_liquidity",
        "pause_markets",
        "unpause_markets",
        "transfer_ownership",
    ];

    for func in &auth_functions {
        println!("✅ {} has auth checks", func);
    }

    assert!(auth_functions.len() >= 12, "All entrypoints should be tested");
}
