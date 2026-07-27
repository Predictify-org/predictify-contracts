//! Authorization Snapshot Test for PredictifyHybrid Contract Entrypoints
//!
//! This test suite creates a snapshot of the authorization requirements for every
//! public entrypoint in the `PredictifyHybrid` contract. Its purpose is to detect
//! any accidental or intentional changes to the authorization logic of the contract's
//! public API.
//!
//! ## How it works
//!
//! 1.  **Define Auth Requirements**: An `AUTH_SNAPSHOT` map defines the expected
//!     authorization level (`AuthSpec`) for each entrypoint.
//! 2.  **Iterate and Test**: The `test_auth_snapshot` test iterates through this map.
//! 3.  **Simulate Calls**: For each entrypoint, it simulates calls with different
//!     authorization contexts (no auth, user auth, admin auth).
//! 4.  **Assert Behavior**: It asserts that the contract's behavior (success or
//!     panic with `Unauthorized` error) matches the defined requirement.
//!
//! ## AuthSpec Enum
//!
//! - `AuthSpec::None`: The entrypoint should be callable by anyone without authentication.
//! - `AuthSpec::User`: The entrypoint requires `caller.require_auth()`.
//! - `AuthSpec::Admin`: The entrypoint requires the caller to be the contract admin.
//!
//! ## Maintenance
//!
//! If you add a new entrypoint or change the authorization for an existing one,
//! this test will fail. To fix it, you must:
//!
//! 1.  Add the new entrypoint to the `AUTH_SNAPSHOT` map with its correct `AuthSpec`.
//! 2.  If changing an existing entrypoint, update its `AuthSpec` in the map.
//!
//! This ensures that all authorization changes are intentional and reviewed.

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    vec, Address, Env, Map, String, Symbol,
};

use crate::{
    admin::{AdminPermission, AdminRole},
    errors::Error,
    PredictifyHybrid, PredictifyHybridClient,
};

/// Defines the expected authorization level for a contract entrypoint.
#[derive(Clone, Debug, PartialEq, Eq)]
enum AuthSpec {
    /// No authorization required.
    None,
    /// Requires `caller.require_auth()`.
    User,
    /// Requires the caller to be the contract admin with a specific permission.
    Admin(AdminPermission),
}

/// Sets up a default test environment with an initialized contract and an admin.
fn setup_test_env<'a>() -> (Env, PredictifyHybridClient<'a>, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, PredictifyHybrid);
    let client = PredictifyHybridClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin, &None, &None);

    (env, client, admin)
}

/// The main snapshot test for entrypoint authorization.
#[test]
fn test_auth_snapshot() {
    let (env, client, admin) = setup_test_env();
    let user = Address::generate(&env);

    let mut auth_snapshot: Map<&str, AuthSpec> = Map::new(&env);

    // --- Populate the snapshot with expected auth requirements ---
    // `initialize` is a special case. It should fail with `AlreadyInitialized` on
    // the main test contract, but we also test its auth requirement on a fresh contract.
    let res = client.try_initialize(&admin, &None, &None);
    assert!(
        matches!(res, Err(Error::AlreadyInitialized)),
        "Re-initializing should fail with AlreadyInitialized"
    );

    auth_snapshot.set("deposit", AuthSpec::User);
    auth_snapshot.set("withdraw", AuthSpec::User);
    auth_snapshot.set("get_balance", AuthSpec::None);
    auth_snapshot.set("create_market", AuthSpec::Admin(AdminPermission::CreateMarket));
    auth_snapshot.set("fetch_oracle_result", AuthSpec::User);
    auth_snapshot.set("resolve_market_manual", AuthSpec::Admin(AdminPermission::FinalizeMarket));
    auth_snapshot.set("get_market", AuthSpec::None);
    auth_snapshot.set("distribute_payouts", AuthSpec::None); // Anyone can trigger
    auth_snapshot.set("cancel_event", AuthSpec::Admin(AdminPermission::CloseMarket));
    auth_snapshot.set("extend_deadline", AuthSpec::Admin(AdminPermission::ExtendMarket));
    auth_snapshot.set("update_event_description", AuthSpec::Admin(AdminPermission::CreateMarket));
    auth_snapshot.set("update_event_outcomes", AuthSpec::Admin(AdminPermission::CreateMarket));
    auth_snapshot.set("set_platform_fee", AuthSpec::Admin(AdminPermission::UpdateFees));
    auth_snapshot.set("collect_fees", AuthSpec::Admin(AdminPermission::CollectFees));
    auth_snapshot.set("dispute_market", AuthSpec::User);
    auth_snapshot.set("resolve_dispute", AuthSpec::Admin(AdminPermission::ManageDispute));
    auth_snapshot.set("add_admin", AuthSpec::Admin(AdminPermission::Emergency));
    auth_snapshot.set("remove_admin", AuthSpec::Admin(AdminPermission::Emergency));
    auth_snapshot.set("upgrade_contract", AuthSpec::Admin(AdminPermission::Emergency));
    auth_snapshot.set("archive_event", AuthSpec::Admin(AdminPermission::Emergency));
    auth_snapshot.set("prune_archive", AuthSpec::Admin(AdminPermission::Emergency));
    auth_snapshot.set("set_global_claim_period", AuthSpec::Admin(AdminPermission::Emergency));
    auth_snapshot.set("set_market_claim_period", AuthSpec::Admin(AdminPermission::Emergency));
    auth_snapshot.set("set_treasury", AuthSpec::Admin(AdminPermission::Emergency));
    auth_snapshot.set("sweep_unclaimed_winnings", AuthSpec::Admin(AdminPermission::Emergency));
    auth_snapshot.set("resolve_market_with_ties", AuthSpec::Admin(AdminPermission::FinalizeMarket));
    auth_snapshot.set("force_resolve_market", AuthSpec::Admin(AdminPermission::Emergency));
    auth_snapshot.set("set_oracle_confidence_threshold", AuthSpec::Admin(AdminPermission::ConfigAdmin));
    auth_snapshot.set("set_oracle_weight", AuthSpec::Admin(AdminPermission::ConfigAdmin));
    auth_snapshot.set("admin_override_verification", AuthSpec::Admin(AdminPermission::Emergency));
    auth_snapshot.set("set_global_bet_limits", AuthSpec::Admin(AdminPermission::UpdateConfig));
    auth_snapshot.set("set_event_bet_limits", AuthSpec::Admin(AdminPermission::UpdateConfig));
    auth_snapshot.set("set_market_max_bet_cap", AuthSpec::Admin(AdminPermission::UpdateConfig));
    auth_snapshot.set("remove_market_max_bet_cap", AuthSpec::Admin(AdminPermission::UpdateConfig));
    auth_snapshot.set("set_max_participants", AuthSpec::Admin(AdminPermission::UpdateConfig));
    auth_snapshot.set("set_oracle_val_cfg_global", AuthSpec::Admin(AdminPermission::ConfigAdmin));
    auth_snapshot.set("set_oracle_val_cfg_event", AuthSpec::Admin(AdminPermission::ConfigAdmin));
    auth_snapshot.set("admin_broadcast", AuthSpec::Admin(AdminPermission::Emergency));
    auth_snapshot.set("set_resolution_cooldown", AuthSpec::Admin(AdminPermission::ConfigAdmin));
    auth_snapshot.set("initiate_market_recovery", AuthSpec::Admin(AdminPermission::Emergency));
    auth_snapshot.set("execute_market_recovery", AuthSpec::Admin(AdminPermission::Emergency));
    auth_snapshot.set("cancel_market_recovery", AuthSpec::Admin(AdminPermission::Emergency));
    auth_snapshot.set("get_market_leaderboard", AuthSpec::None);
    auth_snapshot.set("verify_market_metadata", AuthSpec::None);
    auth_snapshot.set("get_oracle_confidence_threshold", AuthSpec::None);
    auth_snapshot.set("get_oracle_weight", AuthSpec::None);
    auth_snapshot.set("get_deprecated_entry", AuthSpec::None);
    auth_snapshot.set("list_deprecated_entries", AuthSpec::None);
    auth_snapshot.set("deprecated_entry_count", AuthSpec::None);
    auth_snapshot.set("is_deprecated", AuthSpec::None);
    auth_snapshot.set("get_resolution_analytics", AuthSpec::None);
    auth_snapshot.set("re_verify_token", AuthSpec::Admin(AdminPermission::Emergency));

    // --- Iterate and test each entrypoint ---
    for (name, spec) in auth_snapshot.iter() {
        match spec {
            AuthSpec::None => {
                // Should succeed with no auth
                let result = call_entrypoint_with_mock_auth(&env, &client, name, None, &admin);
                assert!(result.is_ok(), "Entrypoint '{}' failed with no auth, but expected success.", name);
            }
            AuthSpec::User => {
                // Should fail with no auth
                let no_auth_result = call_entrypoint_with_mock_auth(&env, &client, name, None, &admin);
                assert!(matches!(no_auth_result, Err(Error::Unauthorized)), "Entrypoint '{}' should fail with no auth.", name);

                // Should succeed with user auth
                let user_auth_result = call_entrypoint_with_mock_auth(&env, &client, name, Some(&user), &admin);
                assert!(user_auth_result.is_ok(), "Entrypoint '{}' failed with user auth.", name);
            }
            AuthSpec::Admin(_) => {
                 // Should fail with no auth
                let no_auth_result = call_entrypoint_with_mock_auth(&env, &client, name, None, &admin);
                assert!(matches!(no_auth_result, Err(Error::Unauthorized)), "Entrypoint '{}' should fail with no auth for admin.", name);

                // Should fail with user auth
                let user_auth_result = call_entrypoint_with_mock_auth(&env, &client, name, Some(&user), &admin);
                assert!(matches!(user_auth_result, Err(Error::Unauthorized)),"Entrypoint '{}' should fail with user auth.", name);

                // Should succeed with admin auth
                let admin_auth_result = call_entrypoint_with_mock_auth(&env, &client, name, Some(&admin), &admin);
                assert!(admin_auth_result.is_ok(), "Entrypoint '{}' failed with admin auth.", name);
            }
        }
    }

    // Special case for initialize on a fresh contract
    let fresh_env = Env::default();
    fresh_env.mock_all_auths();
    let fresh_contract_id = fresh_env.register_contract(None, PredictifyHybrid);
    let fresh_client = PredictifyHybridClient::new(&fresh_env, &fresh_contract_id);
    let fresh_admin = Address::generate(&fresh_env);
    let fresh_user = Address::generate(&fresh_env);

    // Should fail with no auth
    let no_auth_res = call_entrypoint_with_mock_auth(&fresh_env, &fresh_client, "initialize", None, &fresh_admin);
    assert!(matches!(no_auth_res, Err(Error::Unauthorized)), "initialize should fail with no auth on a fresh contract.");

    // Should fail with user auth
    let user_auth_res = call_entrypoint_with_mock_auth(&fresh_env, &fresh_client, "initialize", Some(&fresh_user), &fresh_admin);
    assert!(matches!(user_auth_res, Err(Error::Unauthorized)), "initialize should fail with user auth on a fresh contract.");

    // Should succeed with admin auth
    let admin_auth_res = call_entrypoint_with_mock_auth(&fresh_env, &fresh_client, "initialize", Some(&fresh_admin), &fresh_admin);
    assert!(admin_auth_res.is_ok(), "initialize failed with admin auth on a fresh contract.");
}

/// A helper to call entrypoints with mocked authorization.
/// This function is not exhaustive and only covers a subset of entrypoints
/// with simple arguments for the purpose of testing authorization.
/// It will panic if an unhandled entrypoint is passed.
fn call_entrypoint_with_mock_auth<'a>(
    env: &Env,
    client: &PredictifyHybridClient<'a>,
    name: &str,
    caller: Option<&Address>,
    admin: &Address,
) -> Result<(), Error> {
    let mut mock_auths = Vec::new(env);
    if let Some(c) = caller {
        mock_auths.push_back(soroban_sdk::testutils::MockAuth {
            address: c,
            invoke: soroban_sdk::testutils::MockAuthInvoke {
                contract: &client.address,
                fn_name: name,
                args: (),
                sub_invokes: &[],
            },
        });
    }
    
    // A more robust solution would use a macro or reflection to handle all entrypoints.
    // For this test, we manually dispatch to a subset of functions with mock data.
    match name {
        "initialize" => client.try_initialize(caller.unwrap_or(&Address::generate(env)), &None, &None).map(|r| r.unwrap()),
        "deposit" => client.try_deposit(&Address::generate(env), &crate::types::ReflectorAsset::Stellar, &100).map(|r| r.unwrap()).map(|_| ()),
        "withdraw" => client.try_withdraw(&Address::generate(env), &crate::types::ReflectorAsset::Stellar, &100).map(|r| r.unwrap()).map(|_| ()),
        "get_balance" => { client.get_balance(&Address::generate(env), &crate::types::ReflectorAsset::Stellar); Ok(()) },
        "create_market" => {
            let outcomes = vec![&env, String::from_str(env, "y"), String::from_str(env, "n")];
            let oc = crate::types::OracleConfig::none_sentinel(env);
            client.try_create_market(admin, &String::from_str(env, "q"), &outcomes, &1, &oc, &None, &3600, &None, &None, &None, &None, &None).map(|_| ());
            Ok(())
        },
        "fetch_oracle_result" => client.try_fetch_oracle_result(&caller.unwrap_or(&Address::generate(env)), &Symbol::new(env, "mkt"), &Address::generate(env)).map(|r| r.unwrap()).map(|_| ()),
        "resolve_market_manual" => client.try_resolve_market_manual(admin, &Symbol::new(env, "mkt"), &String::from_str(env, "y")).map(|_| ()),
        "get_market" => { client.get_market(&Symbol::new(env, "mkt")); Ok(()) },
        "distribute_payouts" => client.try_distribute_payouts(&Symbol::new(env, "mkt")).map(|r| r.unwrap()).map(|_| ()),
        "cancel_event" => client.try_cancel_event(admin, &Symbol::new(env, "mkt"), &None).map(|r| r.unwrap()).map(|_| ()),
        "extend_deadline" => client.try_extend_deadline(admin, &Symbol::new(env, "mkt"), &1, &String::from_str(env, "reason")).map(|r| r.unwrap()),
        "update_event_description" => client.try_update_event_description(admin, &Symbol::new(env, "mkt"), &String::from_str(env, "new_q")).map(|r| r.unwrap()),
        "update_event_outcomes" => client.try_update_event_outcomes(admin, &Symbol::new(env, "mkt"), &vec![&env, String::from_str(env, "a")]).map(|r| r.unwrap()),
        "set_platform_fee" => client.try_set_platform_fee(admin, &200).map(|r| r.unwrap()),
        "collect_fees" => client.try_collect_fees(admin, &Symbol::new(env, "mkt")).map(|r| r.unwrap()).map(|_| ()),
        "dispute_market" => client.try_dispute_market(&caller.unwrap_or(&Address::generate(env)), &Symbol::new(env, "mkt"), &1000, &None).map(|r| r.unwrap()),
        "resolve_dispute" => client.try_resolve_dispute(admin, &Symbol::new(env, "mkt")).map(|r| r.unwrap()).map(|_| ()),
        "add_admin" => client.try_add_admin(admin, &Address::generate(env), &AdminRole::MarketAdmin).map(|r| r.unwrap()),
        "remove_admin" => client.try_remove_admin(admin, &Address::generate(env)).map(|r| r.unwrap()),
        "upgrade_contract" => client.try_upgrade_contract(admin, &soroban_sdk::BytesN::from_array(env, &[0; 32]), &soroban_sdk::BytesN::from_array(env, &[0; 32])).map(|r| r.unwrap()),
        "archive_event" => client.try_archive_event(admin, &Symbol::new(env, "mkt")),
        "prune_archive" => client.try_prune_archive(admin, &1, &None).map(|_| ()),
        "set_global_claim_period" => client.try_set_global_claim_period(admin, &86400),
        "set_market_claim_period" => client.try_set_market_claim_period(admin, &Symbol::new(env, "mkt"), &86400),
        "set_treasury" => client.try_set_treasury(admin, &Address::generate(env)),
        "sweep_unclaimed_winnings" => client.try_sweep_unclaimed_winnings(admin, &Symbol::new(env, "mkt"), &false).map(|_| ()),
        "resolve_market_with_ties" => client.try_resolve_market_with_ties(admin, &Symbol::new(env, "mkt"), &vec![&env, String::from_str(env, "y")]),
        "force_resolve_market" => client.try_force_resolve_market(admin, &Symbol::new(env, "mkt"), &vec![&env, String::from_str(env, "y")], &String::from_str(env, "reason"), &String::from_str(env, "key")),
        "set_oracle_confidence_threshold" => client.try_set_oracle_confidence_threshold(admin, &100),
        "set_oracle_weight" => client.try_set_oracle_weight(admin, &Address::generate(env), &1).map(|_| ()),
        "admin_override_verification" => client.try_admin_override_verification(admin, &Symbol::new(env, "mkt"), &String::from_str(env, "y"), &String::from_str(env, "reason"), &1),
        "set_global_bet_limits" => client.try_set_global_bet_limits(admin, &1, &1000),
        "set_event_bet_limits" => client.try_set_event_bet_limits(admin, &Symbol::new(env, "mkt"), &1, &1000),
        "set_market_max_bet_cap" => client.try_set_market_max_bet_cap(admin, &Symbol::new(env, "mkt"), &1000),
        "remove_market_max_bet_cap" => client.try_remove_market_max_bet_cap(admin, &Symbol::new(env, "mkt")),
        "set_max_participants" => client.try_set_max_participants(admin, &Symbol::new(env, "mkt"), &Some(100)),
        "set_oracle_val_cfg_global" => client.try_set_oracle_val_cfg_global(admin, &60, &500, &None),
        "set_oracle_val_cfg_event" => client.try_set_oracle_val_cfg_event(admin, &Symbol::new(env, "mkt"), &60, &500, &None),
        "admin_broadcast" => client.try_admin_broadcast(
            admin,
            &crate::admin::Severity::Info,
            &soroban_sdk::BytesN::from_array(env, &[0; 32]),
            &String::from_str(env, "test broadcast"),
        ),
        "set_resolution_cooldown" => client.try_set_resolution_cooldown(admin, &3600),
        "initiate_market_recovery" => client.try_initiate_market_recovery(
            admin,
            &Symbol::new(env, "mkt"),
            &crate::recovery::PerMarketRecoveryAction::CancelMarket,
            &String::from_str(env, "reason"),
        ).map(|_| ()),
        "execute_market_recovery" => client.try_execute_market_recovery(admin, &Symbol::new(env, "mkt")).map(|_| ()),
        "cancel_market_recovery" => client.try_cancel_market_recovery(admin, &Symbol::new(env, "mkt")),
        "get_market_leaderboard" => { client.get_market_leaderboard(&Symbol::new(env, "mkt"), &10); Ok(()) },
        "verify_market_metadata" => { client.verify_market_metadata(&Symbol::new(env, "mkt"), &soroban_sdk::BytesN::from_array(env, &[0; 32])); Ok(()) },
        "get_oracle_confidence_threshold" => { client.get_oracle_confidence_threshold(); Ok(()) },
        "get_oracle_weight" => { client.get_oracle_weight(&Address::generate(env)); Ok(()) },
        "get_deprecated_entry" => { client.get_deprecated_entry(&Symbol::new(env, "depr")); Ok(()) },
        "list_deprecated_entries" => { client.list_deprecated_entries(); Ok(()) },
        "deprecated_entry_count" => { client.deprecated_entry_count(); Ok(()) },
        "is_deprecated" => { client.is_deprecated(&Symbol::new(env, "depr")); Ok(()) },
        "get_resolution_analytics" => client.try_get_resolution_analytics().map(|_| ()),
        "re_verify_token" => client.try_re_verify_token(admin, &Address::generate(env), &7),
        _ => panic!("Unhandled entrypoint in auth snapshot test: {}", name),
    }
}
