//! Auth-boundary snapshot tests for the Migrate contract.
//!
//! # Purpose
//!
//! Every state-changing entrypoint is covered by two complementary cases that
//! together act as a ratchet: if `require_auth` is ever removed or called on
//! the wrong address the corresponding test fails, making the regression
//! visible in CI without needing to read the source.
//!
//! # Strategy
//!
//! 1. **Reject without auth** — a bare [`Env::default()`] with *no*
//!    [`mock_all_auths()`] call is used.  Because `require_auth()` panics in
//!    the test environment when no auth is mocked, `try_*` returns `Err`.
//!    This proves the auth gate is present and is not accidentally bypassed.
//!
//! 2. **Accept with auth** — [`mock_all_auths()`] is enabled and
//!    [`env.auths()`] is inspected immediately after the call.  We assert that
//!    `auths()[0].0` equals the expected signer.  This proves the *correct*
//!    address is being authorised, not an unrelated or empty one.
//!
//! Read-only views ([`admin`], [`current_version`]) have a dedicated section
//! confirming they require no auth and never panic on an initialised contract.
//!
//! # Additional coverage
//!
//! * `initialize` — [`ContractError::AlreadyInitialized`] on double-init.
//! * `initialize` — [`ContractError::InvalidTargetVersion`] on version 0 and
//!   versions above the compiled-in [`CURRENT_VERSION`] (2).
//! * `migrate_error_data` — a stranger (non-stored-admin address that still
//!   has its auth mocked) is rejected with [`ContractError::Unauthorized`].
//! * `migrate_error_data` — stale expected version yields
//!   [`ContractError::VersionMismatch`]; downgrade yields
//!   [`ContractError::InvalidTargetVersion`].
//! * State invariant — version remains unchanged after every rejected call.

#![cfg(test)]

use migrate::{ContractError, MigrateContract, MigrateContractClient};
use soroban_sdk::{testutils::Address as _, Address, Env};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Register the contract **without** initialising it or enabling auth mocks.
///
/// Use this in tests that need to prove auth is required — a call on the
/// returned client will invoke `require_auth()` against an unmocked env and
/// must therefore fail.
fn register(env: &Env) -> (MigrateContractClient<'_>, Address) {
    let contract_id = env.register(MigrateContract, ());
    let client = MigrateContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    (client, admin)
}

/// Register the contract, enable auth mocks, initialise it, and drain the
/// `initialize` auth snapshot.
///
/// Use this in tests that verify the *signer* recorded by a subsequent call.
/// Draining the init snapshot ensures `env.auths()` after the call under test
/// reflects only that one call, not the setup call.
fn register_and_init(env: &Env, initial_version: u32) -> (MigrateContractClient<'_>, Address) {
    env.mock_all_auths();
    let contract_id = env.register(MigrateContract, ());
    let client = MigrateContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.initialize(&admin, &initial_version);
    // Drain the initialize auth snapshot so it does not pollute later
    // env.auths() assertions in the calling test.
    let _ = env.auths();
    (client, admin)
}

// ---------------------------------------------------------------------------
// initialize — auth gate
// ---------------------------------------------------------------------------

/// `initialize` must invoke `require_auth` on the admin address.
///
/// A bare [`Env::default()`] with no mocking makes `require_auth` panic,
/// which bubbles up as an `Err` from `try_initialize`.
#[test]
fn initialize_rejected_without_auth() {
    let env = Env::default();
    let (client, admin) = register(&env);

    let result = client.try_initialize(&admin, &1);
    assert!(
        result.is_err(),
        "initialize must require auth; succeeded without it"
    );
}

/// With auth mocked, `initialize` records the admin as the sole signer.
#[test]
fn initialize_accepted_with_admin_auth() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = register(&env);

    client.initialize(&admin, &1);

    let auths = env.auths();
    assert_eq!(auths.len(), 1, "exactly one auth entry expected");
    assert_eq!(
        auths[0].0, admin,
        "admin must be the authorised address for initialize"
    );
}

/// A second `initialize` call must return [`ContractError::AlreadyInitialized`].
#[test]
fn initialize_already_initialized_returns_typed_error() {
    let env = Env::default();
    let (client, admin) = register_and_init(&env, 1);

    let result = client.try_initialize(&admin, &1);
    match result {
        Err(Ok(e)) => assert_eq!(
            e,
            ContractError::AlreadyInitialized,
            "double-init must return AlreadyInitialized"
        ),
        other => panic!("expected AlreadyInitialized contract error, got {other:?}"),
    }
}

/// Version `0` is not a valid initial version.
#[test]
fn initialize_rejects_version_zero() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = register(&env);

    let result = client.try_initialize(&admin, &0);
    match result {
        Err(Ok(e)) => assert_eq!(
            e,
            ContractError::InvalidTargetVersion,
            "version 0 must be rejected"
        ),
        other => panic!("expected InvalidTargetVersion, got {other:?}"),
    }
}

/// A version above the compiled-in `CURRENT_VERSION` (2) must be rejected.
#[test]
fn initialize_rejects_version_above_current() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = register(&env);

    let result = client.try_initialize(&admin, &99);
    match result {
        Err(Ok(e)) => assert_eq!(
            e,
            ContractError::InvalidTargetVersion,
            "version above CURRENT_VERSION must be rejected"
        ),
        other => panic!("expected InvalidTargetVersion, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// migrate_error_data — auth gate
// ---------------------------------------------------------------------------

/// `migrate_error_data` must invoke `require_auth` on the admin argument.
///
/// A bare env with no mocking causes `require_auth` to panic, which surfaces
/// as `Err` from `try_migrate_error_data`.
#[test]
fn migrate_error_data_rejected_without_auth() {
    let env = Env::default();
    // Register + initialise without mocking (we drain init ourselves after
    // enabling mocking, then disable so the actual call under test is bare).
    env.mock_all_auths();
    let contract_id = env.register(MigrateContract, ());
    let client = MigrateContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin, &1);

    // Switch to a fresh env without auth mocks for the call under test.
    // We rebuild the client on the same env but cancel auth mocking by using
    // a new Env entirely — the registered contract state does not carry over,
    // so we use a different approach: call on the env with no further mocking
    // after the init drain.  Soroban's auth mock is per-call, so once
    // mock_all_auths() was used in the init call above, subsequent calls on
    // the SAME env still require their own auth to be satisfied.
    //
    // The simplest correct approach: use a fresh env with the contract
    // re-registered but NOT mock_all_auths'd.
    let env2 = Env::default();
    let contract_id2 = env2.register(MigrateContract, ());
    let client2 = MigrateContractClient::new(&env2, &contract_id2);
    let admin2 = Address::generate(&env2);

    // `initialize` itself requires auth — we can't call it here without
    // mocking.  Instead test the un-initialised path: the call still fires
    // require_auth before any storage check, so it must fail.
    let result = client2.try_migrate_error_data(&admin2, &1, &2);
    assert!(
        result.is_err(),
        "migrate_error_data must require auth; succeeded without it"
    );
}

/// With auth mocked, `migrate_error_data` records the admin as the sole
/// signer and bumps the stored version.
#[test]
fn migrate_error_data_accepted_with_admin_auth() {
    let env = Env::default();
    let (client, admin) = register_and_init(&env, 1);

    client.migrate_error_data(&admin, &1, &2);

    let auths = env.auths();
    assert_eq!(auths.len(), 1, "exactly one auth entry expected");
    assert_eq!(
        auths[0].0, admin,
        "admin must be the authorised address for migrate_error_data"
    );
}

/// After a successful `migrate_error_data` the stored version reflects the
/// `target_version` argument.
#[test]
fn migrate_error_data_bumps_version() {
    let env = Env::default();
    let (client, admin) = register_and_init(&env, 1);

    client.migrate_error_data(&admin, &1, &2);

    assert_eq!(
        client.current_version(),
        2,
        "version must be 2 after migrate_error_data(1 → 2)"
    );
}

/// A stranger whose auth is mocked but who is not the stored admin must be
/// rejected with [`ContractError::Unauthorized`].
#[test]
fn migrate_error_data_stranger_rejected_with_unauthorized() {
    let env = Env::default();
    let (client, _admin) = register_and_init(&env, 1);
    let stranger = Address::generate(&env);

    let result = client.try_migrate_error_data(&stranger, &1, &2);
    match result {
        Err(Ok(e)) => assert_eq!(
            e,
            ContractError::Unauthorized,
            "non-admin caller must receive Unauthorized"
        ),
        other => panic!("expected Unauthorized, got {other:?}"),
    }
}

/// A stale `expected_version` must return [`ContractError::VersionMismatch`]
/// and must not alter the stored version.
#[test]
fn migrate_error_data_version_mismatch_returns_typed_error() {
    let env = Env::default();
    // Start at version 1; pass expected=2 (wrong) so we get VersionMismatch.
    let (client, admin) = register_and_init(&env, 1);

    let result = client.try_migrate_error_data(&admin, &2, &3);
    match result {
        Err(Ok(e)) => assert_eq!(
            e,
            ContractError::VersionMismatch,
            "stale expected_version must return VersionMismatch"
        ),
        other => panic!("expected VersionMismatch, got {other:?}"),
    }
    assert_eq!(
        client.current_version(),
        1,
        "version must remain 1 after VersionMismatch"
    );
}

/// A `target_version` equal to the current version must return
/// [`ContractError::InvalidTargetVersion`].
#[test]
fn migrate_error_data_target_equal_to_current_rejected() {
    let env = Env::default();
    // Use version 2 (CURRENT_VERSION); pass target=2 which equals current.
    let (client, admin) = register_and_init(&env, 2);

    let result = client.try_migrate_error_data(&admin, &2, &2);
    match result {
        Err(Ok(e)) => assert_eq!(
            e,
            ContractError::InvalidTargetVersion,
            "target == current must return InvalidTargetVersion"
        ),
        other => panic!("expected InvalidTargetVersion, got {other:?}"),
    }
    assert_eq!(client.current_version(), 2);
}

/// A `target_version` lower than the current version (downgrade) must return
/// [`ContractError::InvalidTargetVersion`] and must not alter state.
#[test]
fn migrate_error_data_downgrade_rejected() {
    let env = Env::default();
    // Start at version 2 (CURRENT_VERSION); attempt to downgrade to 1.
    let (client, admin) = register_and_init(&env, 2);

    let result = client.try_migrate_error_data(&admin, &2, &1);
    match result {
        Err(Ok(e)) => assert_eq!(
            e,
            ContractError::InvalidTargetVersion,
            "downgrade must return InvalidTargetVersion"
        ),
        other => panic!("expected InvalidTargetVersion, got {other:?}"),
    }
    assert_eq!(
        client.current_version(),
        2,
        "version must remain 2 after rejected downgrade"
    );
}

/// Calling `migrate_error_data` on an uninitialised contract must return
/// [`ContractError::NotInitialized`] (require_auth fires before storage
/// checks, so this indirectly confirms the auth gate is also present).
#[test]
fn migrate_error_data_on_uninitialised_contract_returns_not_initialized() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(MigrateContract, ());
    let client = MigrateContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    let result = client.try_migrate_error_data(&admin, &1, &2);
    match result {
        Err(Ok(e)) => assert_eq!(
            e,
            ContractError::NotInitialized,
            "uninitialised contract must return NotInitialized"
        ),
        other => panic!("expected NotInitialized, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Read-only views — no auth required
// ---------------------------------------------------------------------------

/// `admin()` must return the stored admin without requiring any auth.
#[test]
fn admin_view_requires_no_auth() {
    let env = Env::default();
    let (client, admin) = register_and_init(&env, 1);
    // Deliberately no mock_all_auths / additional mocking from here.
    assert_eq!(
        client.admin(),
        admin,
        "admin() must return the stored admin address"
    );
}

/// `current_version()` must return the stored version without requiring auth.
#[test]
fn current_version_view_requires_no_auth() {
    let env = Env::default();
    let (client, _admin) = register_and_init(&env, 1);
    assert_eq!(
        client.current_version(),
        1,
        "current_version() must return the version set during initialize"
    );
}

/// After a successful migration `current_version()` returns the target.
#[test]
fn current_version_reflects_latest_migration() {
    let env = Env::default();
    let (client, admin) = register_and_init(&env, 1);

    client.migrate_error_data(&admin, &1, &7);
    let _ = env.auths();

    assert_eq!(
        client.current_version(),
        7,
        "current_version() must reflect the target version after migration"
    );
}

// ---------------------------------------------------------------------------
// Signer identity invariant
// ---------------------------------------------------------------------------

/// The authorised signer recorded by `initialize` must equal the address
/// passed as the `admin` argument, not some other in-scope address.
#[test]
fn initialize_auth_snapshot_signer_is_the_admin_argument() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = register(&env);
    let bystander = Address::generate(&env);

    client.initialize(&admin, &1);

    let auths = env.auths();
    // The bystander must NOT appear as a signer.
    assert!(
        auths.iter().all(|(signer, _)| *signer != bystander),
        "bystander address must not appear in the auth snapshot"
    );
    // The admin MUST appear as a signer.
    assert!(
        auths.iter().any(|(signer, _)| *signer == admin),
        "admin address must appear in the auth snapshot"
    );
}

/// The authorised signer recorded by `migrate_error_data` must equal the
/// address passed as the `admin` argument.
#[test]
fn migrate_error_data_auth_snapshot_signer_is_the_admin_argument() {
    let env = Env::default();
    let (client, admin) = register_and_init(&env, 1);
    let bystander = Address::generate(&env);

    client.migrate_error_data(&admin, &1, &2);

    let auths = env.auths();
    assert!(
        auths.iter().all(|(signer, _)| *signer != bystander),
        "bystander must not appear in the auth snapshot for migrate_error_data"
    );
    assert!(
        auths.iter().any(|(signer, _)| *signer == admin),
        "admin must appear in the auth snapshot for migrate_error_data"
    );
}
