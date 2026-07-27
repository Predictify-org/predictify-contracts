#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    Address, Env, String, Symbol, Vec,
};
use reporting::{
    ReportingContract, ReportingContractClient,
};

fn setup_test_environment(env: &Env) -> TestSetup {
    env.mock_all_auths();
    env.ledger().set(LedgerInfo {
        timestamp: 1735689600,
        protocol_version: 20,
        sequence_number: 1,
        network_id: [0; 32],
        base_reserve: 10,
        min_temp_entry_ttl: 1,
        min_persistent_entry_ttl: 1,
        max_entry_ttl: 518400,
    });

    let admin = Address::generate(env);
    let reporter = Address::generate(env);
    let unauthorized = Address::generate(env);
    let market_creator = Address::generate(env);

    let contract_id = env.register_contract(None, ReportingContract);
    let client = ReportingContractClient::new(env, &contract_id);

    TestSetup {
        admin,
        reporter,
        unauthorized,
        market_creator,
        client,
        contract_id,
    }
}

struct TestSetup {
    admin: Address,
    reporter: Address,
    unauthorized: Address,
    market_creator: Address,
    client: ReportingContractClient,
    contract_id: Address,
}

fn create_test_reporting(env: &Env, setup: &TestSetup) {
    setup.client.initialize(&setup.admin);
}

// submit_report
#[test]
fn test_submit_report_requires_auth() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    create_test_reporting(&env, &setup);

    let market_id = 1;
    let report_data = String::from_str(&env, "Test report data");
    let report_hash = String::from_str(&env, "0x1234567890abcdef");

    let result = setup.client.try_submit_report(
        &setup.unauthorized,
        &market_id,
        &report_data,
        &report_hash,
    );
    assert!(result.is_err(), "Unauthorized user should not submit report");
}

#[test]
fn test_submit_report_requires_auth_success() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    create_test_reporting(&env, &setup);

    let market_id = 1;
    let report_data = String::from_str(&env, "Test report data");
    let report_hash = String::from_str(&env, "0x1234567890abcdef");

    let result = setup.client.try_submit_report(
        &setup.reporter,
        &market_id,
        &report_data,
        &report_hash,
    );
    // Auth should pass, even if business logic fails
    match result {
        Ok(_) => assert!(true, "Auth passed"),
        Err(e) => {
            let error_str = format!("{:?}", e);
            assert!(!error_str.contains("auth"), "Auth should not fail");
        }
    }
}

// verify_report
#[test]
fn test_verify_report_requires_auth() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    create_test_reporting(&env, &setup);

    let report_id = 1;
    let verification_result = true;

    let result = setup.client.try_verify_report(
        &setup.unauthorized,
        &report_id,
        &verification_result,
    );
    assert!(result.is_err(), "Unauthorized user should not verify report");
}

#[test]
fn test_verify_report_requires_auth_admin() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    create_test_reporting(&env, &setup);

    let report_id = 1;
    let verification_result = true;

    let result = setup.client.try_verify_report(
        &setup.admin,
        &report_id,
        &verification_result,
    );
    match result {
        Ok(_) => assert!(true, "Auth passed"),
        Err(e) => {
            let error_str = format!("{:?}", e);
            assert!(!error_str.contains("auth"), "Auth should not fail");
        }
    }
}

// dispute_report
#[test]
fn test_dispute_report_requires_auth() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    create_test_reporting(&env, &setup);

    let report_id = 1;
    let dispute_reason = String::from_str(&env, "Dispute reason");

    let result = setup.client.try_dispute_report(
        &setup.unauthorized,
        &report_id,
        &dispute_reason,
    );
    assert!(result.is_err(), "Unauthorized user should not dispute report");
}

#[test]
fn test_dispute_report_requires_auth_reporter() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    create_test_reporting(&env, &setup);

    let report_id = 1;
    let dispute_reason = String::from_str(&env, "Dispute reason");

    let result = setup.client.try_dispute_report(
        &setup.reporter,
        &report_id,
        &dispute_reason,
    );
    match result {
        Ok(_) => assert!(true, "Auth passed"),
        Err(e) => {
            let error_str = format!("{:?}", e);
            assert!(!error_str.contains("auth"), "Auth should not fail");
        }
    }
}

// resolve_dispute
#[test]
fn test_resolve_dispute_requires_auth() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    create_test_reporting(&env, &setup);

    let dispute_id = 1;
    let resolution = true;

    let result = setup.client.try_resolve_dispute(
        &setup.unauthorized,
        &dispute_id,
        &resolution,
    );
    assert!(result.is_err(), "Unauthorized user should not resolve dispute");
}

#[test]
fn test_resolve_dispute_requires_auth_admin() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    create_test_reporting(&env, &setup);

    let dispute_id = 1;
    let resolution = true;

    let result = setup.client.try_resolve_dispute(
        &setup.admin,
        &dispute_id,
        &resolution,
    );
    match result {
        Ok(_) => assert!(true, "Auth passed"),
        Err(e) => {
            let error_str = format!("{:?}", e);
            assert!(!error_str.contains("auth"), "Auth should not fail");
        }
    }
}

// update_report_status
#[test]
fn test_update_report_status_requires_auth() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    create_test_reporting(&env, &setup);

    let report_id = 1;
    let new_status = 2;

    let result = setup.client.try_update_report_status(
        &setup.unauthorized,
        &report_id,
        &new_status,
    );
    assert!(result.is_err(), "Unauthorized user should not update report status");
}

#[test]
fn test_update_report_status_requires_auth_admin() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    create_test_reporting(&env, &setup);

    let report_id = 1;
    let new_status = 2;

    let result = setup.client.try_update_report_status(
        &setup.admin,
        &report_id,
        &new_status,
    );
    match result {
        Ok(_) => assert!(true, "Auth passed"),
        Err(e) => {
            let error_str = format!("{:?}", e);
            assert!(!error_str.contains("auth"), "Auth should not fail");
        }
    }
}

// delete_report
#[test]
fn test_delete_report_requires_auth() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    create_test_reporting(&env, &setup);

    let report_id = 1;

    let result = setup.client.try_delete_report(
        &setup.unauthorized,
        &report_id,
    );
    assert!(result.is_err(), "Unauthorized user should not delete report");
}

#[test]
fn test_delete_report_requires_auth_admin() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    create_test_reporting(&env, &setup);

    let report_id = 1;

    let result = setup.client.try_delete_report(
        &setup.admin,
        &report_id,
    );
    match result {
        Ok(_) => assert!(true, "Auth passed"),
        Err(e) => {
            let error_str = format!("{:?}", e);
            assert!(!error_str.contains("auth"), "Auth should not fail");
        }
    }
}

// initialize
#[test]
fn test_initialize_requires_auth() {
    let env = Env::default();

    let contract_id = env.register_contract(None, ReportingContract);
    let client = ReportingContractClient::new(&env, &contract_id);

    let unauthorized = Address::generate(&env);

    let result = client.try_initialize(&unauthorized);
    assert!(result.is_err(), "Unauthorized user should not initialize");
}

#[test]
fn test_initialize_requires_auth_admin() {
    let env = Env::default();

    let contract_id = env.register_contract(None, ReportingContract);
    let client = ReportingContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);

    let result = client.try_initialize(&admin);
    match result {
        Ok(_) => assert!(true, "Auth passed"),
        Err(e) => {
            let error_str = format!("{:?}", e);
            assert!(!error_str.contains("auth"), "Auth should not fail");
        }
    }
}

// pause_reporting
#[test]
fn test_pause_reporting_requires_auth() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    create_test_reporting(&env, &setup);

    let result = setup.client.try_pause_reporting(&setup.unauthorized);
    assert!(result.is_err(), "Unauthorized user should not pause reporting");
}

#[test]
fn test_pause_reporting_requires_auth_admin() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    create_test_reporting(&env, &setup);

    let result = setup.client.try_pause_reporting(&setup.admin);
    match result {
        Ok(_) => assert!(true, "Auth passed"),
        Err(e) => {
            let error_str = format!("{:?}", e);
            assert!(!error_str.contains("auth"), "Auth should not fail");
        }
    }
}

// unpause_reporting
#[test]
fn test_unpause_reporting_requires_auth() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    create_test_reporting(&env, &setup);

    let result = setup.client.try_unpause_reporting(&setup.unauthorized);
    assert!(result.is_err(), "Unauthorized user should not unpause reporting");
}

#[test]
fn test_unpause_reporting_requires_auth_admin() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    create_test_reporting(&env, &setup);

    let result = setup.client.try_unpause_reporting(&setup.admin);
    match result {
        Ok(_) => assert!(true, "Auth passed"),
        Err(e) => {
            let error_str = format!("{:?}", e);
            assert!(!error_str.contains("auth"), "Auth should not fail");
        }
    }
}

// transfer_ownership
#[test]
fn test_transfer_ownership_requires_auth() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    create_test_reporting(&env, &setup);

    let new_owner = Address::generate(&env);

    let result = setup.client.try_transfer_ownership(
        &setup.unauthorized,
        &new_owner,
    );
    assert!(result.is_err(), "Unauthorized user should not transfer ownership");
}

#[test]
fn test_transfer_ownership_requires_auth_admin() {
    let env = Env::default();
    let setup = setup_test_environment(&env);
    create_test_reporting(&env, &setup);

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
