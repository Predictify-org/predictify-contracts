//! Auth-boundary tests for the Reporting contract.
//!
//! # Strategy
//!
//! Every state-changing entrypoint gets two complementary test cases:
//!
//! 1. **Reject without auth** — a fresh `Env` with *no* `mock_all_auths()` is
//!    used. `require_auth()` fires as a host error, making `try_*` return
//!    `Err`. This proves the gate is present and not accidentally bypassed.
//!
//! 2. **Accept with auth** — `mock_all_auths()` is enabled and we assert
//!    `env.auths()[0].0` equals the expected signer. This proves the *correct*
//!    address is authorised, not an arbitrary one.
//!
//! Read-only views (`is_reporting_paused`, `admin`) have a dedicated section
//! confirming they require no auth.
//!
//! # Additional coverage
//!
//! * `initialize` — typed `AlreadyInitialized` error on double-init.
//! * `transfer_ownership` — typed `InvalidNewOwner` error on self-transfer.
//! * Pause guard — state-changing ops revert with `ReportingPaused` when
//!   the contract is paused.
//! * Role separation — admin-only calls reject unauthenticated reporters and
//!   vice-versa.

#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    Address, Env, String,
};

use reporting::{ReportingContract, ReportingContractClient, ReportingError};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn ledger_info() -> LedgerInfo {
    LedgerInfo {
        timestamp: 1_735_689_600,
        protocol_version: 20,
        sequence_number: 1,
        network_id: [0u8; 32],
        base_reserve: 10,
        min_temp_entry_ttl: 1,
        min_persistent_entry_ttl: 1,
        max_entry_ttl: 518_400,
    }
}

/// Register the contract without initialising it or mocking auth.
fn register(env: &Env) -> (ReportingContractClient, Address) {
    env.ledger().set(ledger_info());
    let contract_id = env.register_contract(None, ReportingContract);
    let client = ReportingContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    (client, admin)
}

/// Register + initialize in one step.
///
/// `mock_all_auths()` is activated before returning, and the auth snapshot
/// from `initialize` is drained so it does not pollute later `env.auths()`
/// assertions.
fn register_and_init(env: &Env) -> (ReportingContractClient, Address) {
    env.mock_all_auths();
    env.ledger().set(ledger_info());
    let contract_id = env.register_contract(None, ReportingContract);
    let client = ReportingContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.initialize(&admin).unwrap();
    // Drain the init auth snapshot.
    let _ = env.auths();
    (client, admin)
}

fn report_data(env: &Env) -> String {
    String::from_str(env, "Test report data")
}

fn report_hash(env: &Env) -> String {
    String::from_str(env, "0xdeadbeef1234567890abcdef")
}

fn dispute_reason(env: &Env) -> String {
    String::from_str(env, "Incorrect data in report")
}

// ---------------------------------------------------------------------------
// initialize
// ---------------------------------------------------------------------------

/// `initialize` must fire `require_auth` — a bare call without mocking fails.
#[test]
fn test_initialize_rejected_without_auth() {
    let env = Env::default();
    let (client, admin) = register(&env);

    let result = client.try_initialize(&admin);
    assert!(
        result.is_err(),
        "initialize must require auth; succeeded without it"
    );
}

/// With auth mocked, `initialize` records the admin address as the signer.
#[test]
fn test_initialize_accepted_with_admin_auth() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = register(&env);

    client.initialize(&admin).unwrap();

    let auths = env.auths();
    assert_eq!(auths.len(), 1, "exactly one auth entry expected");
    assert_eq!(auths[0].0, admin, "admin must be the authorised address");
}

/// Double-initialization must return `AlreadyInitialized`, not panic.
#[test]
fn test_initialize_returns_already_initialized_on_double_init() {
    let env = Env::default();
    let (client, admin) = register_and_init(&env);

    let result = client.try_initialize(&admin);
    assert!(result.is_err(), "double-init must fail");
    match result.err().unwrap() {
        Ok(e) => assert_eq!(e, ReportingError::AlreadyInitialized),
        Err(_) => panic!("expected contract error, got host error"),
    }
}

// ---------------------------------------------------------------------------
// submit_report
// ---------------------------------------------------------------------------

/// `submit_report` must fire `require_auth` on the reporter address.
#[test]
fn test_submit_report_rejected_without_auth() {
    let env = Env::default();
    env.ledger().set(ledger_info());
    let contract = env.register_contract(None, ReportingContract);
    let client = ReportingContractClient::new(&env, &contract);
    let reporter = Address::generate(&env);

    let result = client.try_submit_report(
        &reporter,
        &1u32,
        &report_data(&env),
        &report_hash(&env),
    );
    assert!(
        result.is_err(),
        "submit_report must require auth; succeeded without it"
    );
}

/// With auth mocked, `submit_report` records the reporter as the signer.
#[test]
fn test_submit_report_accepted_with_reporter_auth() {
    let env = Env::default();
    let (client, _admin) = register_and_init(&env);
    let reporter = Address::generate(&env);

    client
        .submit_report(&reporter, &1u32, &report_data(&env), &report_hash(&env))
        .unwrap();

    let auths = env.auths();
    assert_eq!(auths.len(), 1, "exactly one auth entry expected");
    assert_eq!(auths[0].0, reporter, "reporter must be the authorised address");
}

// ---------------------------------------------------------------------------
// verify_report
// ---------------------------------------------------------------------------

/// `verify_report` must fire `require_auth` on the admin address.
#[test]
fn test_verify_report_rejected_without_auth() {
    let env = Env::default();
    env.ledger().set(ledger_info());
    let contract = env.register_contract(None, ReportingContract);
    let client = ReportingContractClient::new(&env, &contract);
    let admin = Address::generate(&env);

    let result = client.try_verify_report(&admin, &1u32, &true);
    assert!(
        result.is_err(),
        "verify_report must require auth; succeeded without it"
    );
}

/// With auth mocked, `verify_report` records the admin as the signer.
#[test]
fn test_verify_report_accepted_with_admin_auth() {
    let env = Env::default();
    let (client, admin) = register_and_init(&env);

    client.verify_report(&admin, &1u32, &true).unwrap();

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    assert_eq!(auths[0].0, admin, "admin must be the authorised address");
}

/// A reporter address must not call `verify_report` without auth.
#[test]
fn test_verify_report_role_separation_reporter_cannot_verify() {
    let env = Env::default();
    env.ledger().set(ledger_info());
    let contract = env.register_contract(None, ReportingContract);
    let client = ReportingContractClient::new(&env, &contract);
    let reporter = Address::generate(&env);

    let result = client.try_verify_report(&reporter, &1u32, &true);
    assert!(
        result.is_err(),
        "reporter should not be able to call verify_report without auth"
    );
}

// ---------------------------------------------------------------------------
// dispute_report
// ---------------------------------------------------------------------------

/// `dispute_report` must fire `require_auth` on the reporter address.
#[test]
fn test_dispute_report_rejected_without_auth() {
    let env = Env::default();
    env.ledger().set(ledger_info());
    let contract = env.register_contract(None, ReportingContract);
    let client = ReportingContractClient::new(&env, &contract);
    let reporter = Address::generate(&env);

    let result = client.try_dispute_report(&reporter, &1u32, &dispute_reason(&env));
    assert!(
        result.is_err(),
        "dispute_report must require auth; succeeded without it"
    );
}

/// With auth mocked, `dispute_report` records the reporter as the signer.
#[test]
fn test_dispute_report_accepted_with_reporter_auth() {
    let env = Env::default();
    let (client, _admin) = register_and_init(&env);
    let reporter = Address::generate(&env);

    client
        .dispute_report(&reporter, &1u32, &dispute_reason(&env))
        .unwrap();

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    assert_eq!(auths[0].0, reporter, "reporter must be the authorised address");
}

// ---------------------------------------------------------------------------
// resolve_dispute
// ---------------------------------------------------------------------------

/// `resolve_dispute` must fire `require_auth` on the admin address.
#[test]
fn test_resolve_dispute_rejected_without_auth() {
    let env = Env::default();
    env.ledger().set(ledger_info());
    let contract = env.register_contract(None, ReportingContract);
    let client = ReportingContractClient::new(&env, &contract);
    let admin = Address::generate(&env);

    let result = client.try_resolve_dispute(&admin, &1u32, &true);
    assert!(
        result.is_err(),
        "resolve_dispute must require auth; succeeded without it"
    );
}

/// With auth mocked, `resolve_dispute` records the admin as the signer.
#[test]
fn test_resolve_dispute_accepted_with_admin_auth() {
    let env = Env::default();
    let (client, admin) = register_and_init(&env);

    client.resolve_dispute(&admin, &1u32, &true).unwrap();

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    assert_eq!(auths[0].0, admin, "admin must be the authorised address");
}

/// A reporter address must not call `resolve_dispute` without auth.
#[test]
fn test_resolve_dispute_role_separation_reporter_cannot_resolve() {
    let env = Env::default();
    env.ledger().set(ledger_info());
    let contract = env.register_contract(None, ReportingContract);
    let client = ReportingContractClient::new(&env, &contract);
    let reporter = Address::generate(&env);

    let result = client.try_resolve_dispute(&reporter, &1u32, &true);
    assert!(
        result.is_err(),
        "reporter should not be able to call resolve_dispute without auth"
    );
}

// ---------------------------------------------------------------------------
// update_report_status
// ---------------------------------------------------------------------------

/// `update_report_status` must fire `require_auth` on the admin address.
#[test]
fn test_update_report_status_rejected_without_auth() {
    let env = Env::default();
    env.ledger().set(ledger_info());
    let contract = env.register_contract(None, ReportingContract);
    let client = ReportingContractClient::new(&env, &contract);
    let admin = Address::generate(&env);

    let result = client.try_update_report_status(&admin, &1u32, &2u32);
    assert!(
        result.is_err(),
        "update_report_status must require auth; succeeded without it"
    );
}

/// With auth mocked, `update_report_status` records the admin as the signer.
#[test]
fn test_update_report_status_accepted_with_admin_auth() {
    let env = Env::default();
    let (client, admin) = register_and_init(&env);

    client.update_report_status(&admin, &1u32, &2u32).unwrap();

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    assert_eq!(auths[0].0, admin, "admin must be the authorised address");
}

// ---------------------------------------------------------------------------
// delete_report
// ---------------------------------------------------------------------------

/// `delete_report` must fire `require_auth` on the admin address.
#[test]
fn test_delete_report_rejected_without_auth() {
    let env = Env::default();
    env.ledger().set(ledger_info());
    let contract = env.register_contract(None, ReportingContract);
    let client = ReportingContractClient::new(&env, &contract);
    let admin = Address::generate(&env);

    let result = client.try_delete_report(&admin, &1u32);
    assert!(
        result.is_err(),
        "delete_report must require auth; succeeded without it"
    );
}

/// With auth mocked, `delete_report` records the admin as the signer.
#[test]
fn test_delete_report_accepted_with_admin_auth() {
    let env = Env::default();
    let (client, admin) = register_and_init(&env);

    client.delete_report(&admin, &1u32).unwrap();

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    assert_eq!(auths[0].0, admin, "admin must be the authorised address");
}

// ---------------------------------------------------------------------------
// pause_reporting
// ---------------------------------------------------------------------------

/// `pause_reporting` must fire `require_auth` on the admin address.
#[test]
fn test_pause_reporting_rejected_without_auth() {
    let env = Env::default();
    env.ledger().set(ledger_info());
    let contract = env.register_contract(None, ReportingContract);
    let client = ReportingContractClient::new(&env, &contract);
    let admin = Address::generate(&env);

    let result = client.try_pause_reporting(&admin);
    assert!(
        result.is_err(),
        "pause_reporting must require auth; succeeded without it"
    );
}

/// With auth mocked, `pause_reporting` records the admin as the signer and
/// sets the paused flag.
#[test]
fn test_pause_reporting_accepted_with_admin_auth() {
    let env = Env::default();
    let (client, admin) = register_and_init(&env);

    client.pause_reporting(&admin).unwrap();

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    assert_eq!(auths[0].0, admin, "admin must be the authorised address");
    assert!(client.is_reporting_paused(), "contract should be paused");
}

/// A non-admin address must not be able to pause without auth.
#[test]
fn test_pause_reporting_non_admin_rejected_without_auth() {
    let env = Env::default();
    env.ledger().set(ledger_info());
    let contract = env.register_contract(None, ReportingContract);
    let client = ReportingContractClient::new(&env, &contract);
    let non_admin = Address::generate(&env);

    let result = client.try_pause_reporting(&non_admin);
    assert!(result.is_err(), "non-admin must not be able to pause without auth");
}

// ---------------------------------------------------------------------------
// unpause_reporting
// ---------------------------------------------------------------------------

/// `unpause_reporting` must fire `require_auth` on the admin address.
#[test]
fn test_unpause_reporting_rejected_without_auth() {
    let env = Env::default();
    env.ledger().set(ledger_info());
    let contract = env.register_contract(None, ReportingContract);
    let client = ReportingContractClient::new(&env, &contract);
    let admin = Address::generate(&env);

    let result = client.try_unpause_reporting(&admin);
    assert!(
        result.is_err(),
        "unpause_reporting must require auth; succeeded without it"
    );
}

/// With auth mocked, `unpause_reporting` records the admin as the signer
/// and clears the paused flag.
#[test]
fn test_unpause_reporting_accepted_with_admin_auth() {
    let env = Env::default();
    let (client, admin) = register_and_init(&env);

    client.pause_reporting(&admin).unwrap();
    let _ = env.auths(); // drain pause auth

    client.unpause_reporting(&admin).unwrap();

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    assert_eq!(auths[0].0, admin, "admin must be the authorised address");
    assert!(!client.is_reporting_paused(), "contract should be unpaused");
}

// ---------------------------------------------------------------------------
// transfer_ownership
// ---------------------------------------------------------------------------

/// `transfer_ownership` must fire `require_auth` on the admin address.
#[test]
fn test_transfer_ownership_rejected_without_auth() {
    let env = Env::default();
    env.ledger().set(ledger_info());
    let contract = env.register_contract(None, ReportingContract);
    let client = ReportingContractClient::new(&env, &contract);
    let admin = Address::generate(&env);
    let new_owner = Address::generate(&env);

    let result = client.try_transfer_ownership(&admin, &new_owner);
    assert!(
        result.is_err(),
        "transfer_ownership must require auth; succeeded without it"
    );
}

/// With auth mocked, `transfer_ownership` records the admin and updates the
/// stored admin to `new_owner`.
#[test]
fn test_transfer_ownership_accepted_with_admin_auth() {
    let env = Env::default();
    let (client, admin) = register_and_init(&env);
    let new_owner = Address::generate(&env);

    client.transfer_ownership(&admin, &new_owner).unwrap();

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    assert_eq!(auths[0].0, admin, "admin must be the authorised address");
    assert_eq!(client.admin(), new_owner, "admin should have changed to new_owner");
}

/// Self-transfer (`admin == new_owner`) must return `InvalidNewOwner`.
#[test]
fn test_transfer_ownership_self_transfer_rejected() {
    let env = Env::default();
    let (client, admin) = register_and_init(&env);

    let result = client.try_transfer_ownership(&admin, &admin);
    assert!(result.is_err(), "self-transfer must be rejected");
    match result.err().unwrap() {
        Ok(e) => assert_eq!(e, ReportingError::InvalidNewOwner),
        Err(_) => panic!("expected InvalidNewOwner contract error, got host error"),
    }
}

/// After ownership is transferred the new admin is recorded correctly.
#[test]
fn test_transfer_ownership_new_owner_is_stored() {
    let env = Env::default();
    let (client, admin) = register_and_init(&env);
    let new_owner = Address::generate(&env);

    client.transfer_ownership(&admin, &new_owner).unwrap();
    let _ = env.auths();

    assert_eq!(
        client.admin(),
        new_owner,
        "new_owner should be the stored admin after transfer"
    );
}

// ---------------------------------------------------------------------------
// Read-only views — no auth required
// ---------------------------------------------------------------------------

/// `is_reporting_paused` must work without any auth mocking.
#[test]
fn test_is_reporting_paused_does_not_require_auth() {
    let env = Env::default();
    env.ledger().set(ledger_info());
    let contract = env.register_contract(None, ReportingContract);
    let client = ReportingContractClient::new(&env, &contract);

    // No initialize, no mock_all_auths — must return false without panic.
    let paused = client.is_reporting_paused();
    assert!(!paused, "freshly registered contract should not be paused");
}

/// `admin` view must return the correct address after initialization.
#[test]
fn test_admin_view_returns_current_admin() {
    let env = Env::default();
    let (client, admin) = register_and_init(&env);

    assert_eq!(client.admin(), admin, "admin view should return the admin");
}

// ---------------------------------------------------------------------------
// Pause guard — state-changing ops blocked while paused
// ---------------------------------------------------------------------------

/// `submit_report` must revert with `ReportingPaused` when the contract is
/// paused.
#[test]
fn test_submit_report_blocked_when_paused() {
    let env = Env::default();
    let (client, admin) = register_and_init(&env);
    let reporter = Address::generate(&env);

    client.pause_reporting(&admin).unwrap();
    let _ = env.auths();

    let result =
        client.try_submit_report(&reporter, &1u32, &report_data(&env), &report_hash(&env));
    assert!(result.is_err(), "submit_report should fail when paused");
    match result.err().unwrap() {
        Ok(e) => assert_eq!(e, ReportingError::ReportingPaused),
        Err(_) => panic!("expected ReportingPaused, got host error"),
    }
}

/// `verify_report` must revert with `ReportingPaused` when the contract is
/// paused.
#[test]
fn test_verify_report_blocked_when_paused() {
    let env = Env::default();
    let (client, admin) = register_and_init(&env);

    client.pause_reporting(&admin).unwrap();
    let _ = env.auths();

    let result = client.try_verify_report(&admin, &1u32, &true);
    assert!(result.is_err(), "verify_report should fail when paused");
    match result.err().unwrap() {
        Ok(e) => assert_eq!(e, ReportingError::ReportingPaused),
        Err(_) => panic!("expected ReportingPaused, got host error"),
    }
}

/// `dispute_report` must revert with `ReportingPaused` when the contract is
/// paused.
#[test]
fn test_dispute_report_blocked_when_paused() {
    let env = Env::default();
    let (client, admin) = register_and_init(&env);
    let reporter = Address::generate(&env);

    client.pause_reporting(&admin).unwrap();
    let _ = env.auths();

    let result = client.try_dispute_report(&reporter, &1u32, &dispute_reason(&env));
    assert!(result.is_err(), "dispute_report should fail when paused");
    match result.err().unwrap() {
        Ok(e) => assert_eq!(e, ReportingError::ReportingPaused),
        Err(_) => panic!("expected ReportingPaused, got host error"),
    }
}

/// `resolve_dispute` must revert with `ReportingPaused` when the contract is
/// paused.
#[test]
fn test_resolve_dispute_blocked_when_paused() {
    let env = Env::default();
    let (client, admin) = register_and_init(&env);

    client.pause_reporting(&admin).unwrap();
    let _ = env.auths();

    let result = client.try_resolve_dispute(&admin, &1u32, &true);
    assert!(result.is_err(), "resolve_dispute should fail when paused");
    match result.err().unwrap() {
        Ok(e) => assert_eq!(e, ReportingError::ReportingPaused),
        Err(_) => panic!("expected ReportingPaused, got host error"),
    }
}

/// `update_report_status` must revert with `ReportingPaused` when the
/// contract is paused.
#[test]
fn test_update_report_status_blocked_when_paused() {
    let env = Env::default();
    let (client, admin) = register_and_init(&env);

    client.pause_reporting(&admin).unwrap();
    let _ = env.auths();

    let result = client.try_update_report_status(&admin, &1u32, &2u32);
    assert!(result.is_err(), "update_report_status should fail when paused");
    match result.err().unwrap() {
        Ok(e) => assert_eq!(e, ReportingError::ReportingPaused),
        Err(_) => panic!("expected ReportingPaused, got host error"),
    }
}

/// `delete_report` must revert with `ReportingPaused` when the contract is
/// paused.
#[test]
fn test_delete_report_blocked_when_paused() {
    let env = Env::default();
    let (client, admin) = register_and_init(&env);

    client.pause_reporting(&admin).unwrap();
    let _ = env.auths();

    let result = client.try_delete_report(&admin, &1u32);
    assert!(result.is_err(), "delete_report should fail when paused");
    match result.err().unwrap() {
        Ok(e) => assert_eq!(e, ReportingError::ReportingPaused),
        Err(_) => panic!("expected ReportingPaused, got host error"),
    }
}

/// After unpause, state-changing operations must succeed (no `ReportingPaused`
/// error).
#[test]
fn test_state_changing_ops_succeed_after_unpause() {
    let env = Env::default();
    let (client, admin) = register_and_init(&env);
    let reporter = Address::generate(&env);

    client.pause_reporting(&admin).unwrap();
    client.unpause_reporting(&admin).unwrap();
    let _ = env.auths();

    let result =
        client.try_submit_report(&reporter, &1u32, &report_data(&env), &report_hash(&env));
    match result {
        Ok(_) => {} // success
        Err(e) => match e {
            Ok(contract_err) => assert_ne!(
                contract_err,
                ReportingError::ReportingPaused,
                "should not get ReportingPaused after unpause"
            ),
            Err(_) => panic!("unexpected host error after unpause"),
        },
    }
}

// ---------------------------------------------------------------------------
// Idempotency
// ---------------------------------------------------------------------------

/// Calling `pause_reporting` twice must not fail.
#[test]
fn test_double_pause_is_idempotent() {
    let env = Env::default();
    let (client, admin) = register_and_init(&env);

    client.pause_reporting(&admin).unwrap();
    client.pause_reporting(&admin).unwrap(); // second call — must not fail

    assert!(client.is_reporting_paused(), "should still be paused");
}

/// Calling `unpause_reporting` twice must not fail.
#[test]
fn test_double_unpause_is_idempotent() {
    let env = Env::default();
    let (client, admin) = register_and_init(&env);

    client.pause_reporting(&admin).unwrap();
    client.unpause_reporting(&admin).unwrap();
    client.unpause_reporting(&admin).unwrap(); // second call — must not fail

    assert!(!client.is_reporting_paused(), "should remain unpaused");
}
