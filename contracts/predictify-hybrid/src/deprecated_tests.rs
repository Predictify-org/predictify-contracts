//! Focused tests for the deprecated-entrypoints registry.
//!
//! These tests exercise [`DeprecatedRegistry`] end-to-end:
//!
//! * `register` – happy-path, idempotency, capacity limit, auth guard
//! * `remove`   – present / absent, auth guard
//! * `get_entry` / `is_deprecated` / `list_entries` / `entry_count` – read paths
//! * integration with `record_call` (event emission)

#[cfg(test)]
mod deprecated_registry_tests {
    use crate::deprecated::{DeprecatedRegistry, MAX_REGISTRY_ENTRIES};
    use crate::err::Error;
    use soroban_sdk::{
        symbol_short,
        testutils::{Address as _, Events},
        Address, Env, Symbol, String, TryIntoVal,
    };

    // -----------------------------------------------------------------------
    // Test helpers
    // -----------------------------------------------------------------------

    /// Set up an env + admin following the same pattern used throughout the
    /// contract test suite.
    fn setup_env_with_admin() -> (Env, Address) {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);

        // Initialise the "Admin" persistent storage key so that
        // AdminAccessControl::require_admin_auth can validate the caller.
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, "Admin"), &admin);

        (env, admin)
    }

    fn sym(env: &Env, s: &str) -> Symbol {
        Symbol::new(env, s)
    }

    fn opt_note(env: &Env, s: &str) -> Option<String> {
        Some(String::from_str(env, s))
    }

    // -----------------------------------------------------------------------
    // register – happy path
    // -----------------------------------------------------------------------

    #[test]
    fn test_register_adds_entry() {
        let (env, admin) = setup_env_with_admin();

        DeprecatedRegistry::register(
            &env,
            &admin,
            sym(&env, "verify_result"),
            sym(&env, "fetch_oracle"),
            opt_note(&env, "Use fetch_oracle instead"),
        )
        .unwrap();

        let entry = DeprecatedRegistry::get_entry(&env, &sym(&env, "verify_result"))
            .expect("entry should be present");

        assert_eq!(entry.entrypoint, sym(&env, "verify_result"));
        assert_eq!(entry.replacement, sym(&env, "fetch_oracle"));
        assert!(entry.note.is_some());
    }

    #[test]
    fn test_register_without_note() {
        let (env, admin) = setup_env_with_admin();

        DeprecatedRegistry::register(
            &env,
            &admin,
            sym(&env, "old_fn"),
            sym(&env, "new_fn"),
            None,
        )
        .unwrap();

        let entry = DeprecatedRegistry::get_entry(&env, &sym(&env, "old_fn")).unwrap();
        assert!(entry.note.is_none());
    }

    #[test]
    fn test_register_sets_since_timestamp() {
        let (env, admin) = setup_env_with_admin();

        let before = env.ledger().timestamp();
        DeprecatedRegistry::register(
            &env,
            &admin,
            sym(&env, "legacy"),
            sym(&env, "modern"),
            None,
        )
        .unwrap();
        let after = env.ledger().timestamp();

        let entry = DeprecatedRegistry::get_entry(&env, &sym(&env, "legacy")).unwrap();
        assert!(entry.since >= before && entry.since <= after);
    }

    // -----------------------------------------------------------------------
    // register – idempotency
    // -----------------------------------------------------------------------

    #[test]
    fn test_register_idempotent() {
        let (env, admin) = setup_env_with_admin();

        DeprecatedRegistry::register(
            &env,
            &admin,
            sym(&env, "fn_a"),
            sym(&env, "fn_b"),
            None,
        )
        .unwrap();

        // Second call with same entrypoint should succeed silently.
        DeprecatedRegistry::register(
            &env,
            &admin,
            sym(&env, "fn_a"),
            sym(&env, "fn_c"), // different replacement, ignored
            None,
        )
        .unwrap();

        // Count should still be 1.
        assert_eq!(DeprecatedRegistry::entry_count(&env), 1);

        // Original replacement preserved.
        let entry = DeprecatedRegistry::get_entry(&env, &sym(&env, "fn_a")).unwrap();
        assert_eq!(entry.replacement, sym(&env, "fn_b"));
    }

    // -----------------------------------------------------------------------
    // register – capacity guard
    // -----------------------------------------------------------------------

    #[test]
    fn test_register_returns_error_when_full() {
        let (env, admin) = setup_env_with_admin();

        // Fill the registry to capacity.
        // Symbol names must be ≤ 9 chars in Soroban; use short indexed names.
        for i in 0..MAX_REGISTRY_ENTRIES {
            // Build names like "fn0", "fn1", … "fn63"
            let name = alloc::format!("fn{}", i);
            let repl = alloc::format!("nw{}", i);
            DeprecatedRegistry::register(
                &env,
                &admin,
                Symbol::new(&env, &name),
                Symbol::new(&env, &repl),
                None,
            )
            .unwrap();
        }

        // One more should fail.
        let result = DeprecatedRegistry::register(
            &env,
            &admin,
            sym(&env, "overflow"),
            sym(&env, "none"),
            None,
        );
        assert_eq!(result, Err(Error::RegistryFull));
    }

    // -----------------------------------------------------------------------
    // register – auth guard
    // -----------------------------------------------------------------------

    #[test]
    fn test_register_rejects_non_admin() {
        let (env, _admin) = setup_env_with_admin();
        let attacker = Address::generate(&env);

        let result = DeprecatedRegistry::register(
            &env,
            &attacker,
            sym(&env, "steal"),
            sym(&env, "none"),
            None,
        );
        assert_eq!(result, Err(Error::Unauthorized));
    }

    // -----------------------------------------------------------------------
    // remove – present entry
    // -----------------------------------------------------------------------

    #[test]
    fn test_remove_existing_entry() {
        let (env, admin) = setup_env_with_admin();

        DeprecatedRegistry::register(
            &env,
            &admin,
            sym(&env, "old"),
            sym(&env, "new"),
            None,
        )
        .unwrap();

        assert_eq!(DeprecatedRegistry::entry_count(&env), 1);

        DeprecatedRegistry::remove(&env, &admin, sym(&env, "old")).unwrap();

        assert_eq!(DeprecatedRegistry::entry_count(&env), 0);
        assert!(DeprecatedRegistry::get_entry(&env, &sym(&env, "old")).is_none());
    }

    #[test]
    fn test_remove_preserves_other_entries() {
        let (env, admin) = setup_env_with_admin();

        DeprecatedRegistry::register(&env, &admin, sym(&env, "a"), sym(&env, "aa"), None).unwrap();
        DeprecatedRegistry::register(&env, &admin, sym(&env, "b"), sym(&env, "bb"), None).unwrap();
        DeprecatedRegistry::register(&env, &admin, sym(&env, "c"), sym(&env, "cc"), None).unwrap();

        DeprecatedRegistry::remove(&env, &admin, sym(&env, "b")).unwrap();

        assert_eq!(DeprecatedRegistry::entry_count(&env), 2);
        assert!(DeprecatedRegistry::get_entry(&env, &sym(&env, "a")).is_some());
        assert!(DeprecatedRegistry::get_entry(&env, &sym(&env, "b")).is_none());
        assert!(DeprecatedRegistry::get_entry(&env, &sym(&env, "c")).is_some());
    }

    // -----------------------------------------------------------------------
    // remove – absent entry (no-op)
    // -----------------------------------------------------------------------

    #[test]
    fn test_remove_absent_entry_is_noop() {
        let (env, admin) = setup_env_with_admin();

        // Registry is empty; removal should succeed silently.
        DeprecatedRegistry::remove(&env, &admin, sym(&env, "ghost")).unwrap();
        assert_eq!(DeprecatedRegistry::entry_count(&env), 0);
    }

    // -----------------------------------------------------------------------
    // remove – auth guard
    // -----------------------------------------------------------------------

    #[test]
    fn test_remove_rejects_non_admin() {
        let (env, admin) = setup_env_with_admin();
        let attacker = Address::generate(&env);

        DeprecatedRegistry::register(&env, &admin, sym(&env, "fn"), sym(&env, "nfn"), None)
            .unwrap();

        let result = DeprecatedRegistry::remove(&env, &attacker, sym(&env, "fn"));
        assert_eq!(result, Err(Error::Unauthorized));

        // Entry must still be present.
        assert!(DeprecatedRegistry::get_entry(&env, &sym(&env, "fn")).is_some());
    }

    // -----------------------------------------------------------------------
    // read operations
    // -----------------------------------------------------------------------

    #[test]
    fn test_get_entry_returns_none_for_unknown() {
        let (env, _) = setup_env_with_admin();
        assert!(DeprecatedRegistry::get_entry(&env, &sym(&env, "unknown")).is_none());
    }

    #[test]
    fn test_list_entries_empty() {
        let (env, _) = setup_env_with_admin();
        assert_eq!(DeprecatedRegistry::list_entries(&env).len(), 0);
    }

    #[test]
    fn test_list_entries_returns_all() {
        let (env, admin) = setup_env_with_admin();

        DeprecatedRegistry::register(&env, &admin, sym(&env, "f1"), sym(&env, "g1"), None).unwrap();
        DeprecatedRegistry::register(&env, &admin, sym(&env, "f2"), sym(&env, "g2"), None).unwrap();

        let list = DeprecatedRegistry::list_entries(&env);
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_entry_count_tracks_changes() {
        let (env, admin) = setup_env_with_admin();

        assert_eq!(DeprecatedRegistry::entry_count(&env), 0);

        DeprecatedRegistry::register(&env, &admin, sym(&env, "x"), sym(&env, "y"), None).unwrap();
        assert_eq!(DeprecatedRegistry::entry_count(&env), 1);

        DeprecatedRegistry::remove(&env, &admin, sym(&env, "x")).unwrap();
        assert_eq!(DeprecatedRegistry::entry_count(&env), 0);
    }

    #[test]
    fn test_is_deprecated_true_and_false() {
        let (env, admin) = setup_env_with_admin();

        assert!(!DeprecatedRegistry::is_deprecated(&env, &sym(&env, "fn")));

        DeprecatedRegistry::register(&env, &admin, sym(&env, "fn"), sym(&env, "nfn"), None)
            .unwrap();

        assert!(DeprecatedRegistry::is_deprecated(&env, &sym(&env, "fn")));
    }

    // -----------------------------------------------------------------------
    // record_call – emits event
    // -----------------------------------------------------------------------

    #[test]
    fn test_record_call_emits_deprecated_event() {
        let env = Env::default();
        env.mock_all_auths();

        let caller = Address::generate(&env);

        DeprecatedRegistry::record_call(
            &env,
            &caller,
            &sym(&env, "verify_r"),
            &sym(&env, "fetch_or"),
        );

        let contract_events = env.events().all();
        let events = contract_events.events();
        assert!(!events.is_empty(), "must emit at least one event");

        // Verify the emitted event contains expected fields
        let found = events.iter().any(|e| {
            if let soroban_sdk::xdr::ContractEventBody::V0(v0) = &e.body {
                let topic0: Symbol = v0
                    .topics
                    .get(0)
                    .unwrap()
                    .clone()
                    .try_into_val(&env)
                    .unwrap();
                let topic1: Symbol = v0
                    .topics
                    .get(1)
                    .unwrap()
                    .clone()
                    .try_into_val(&env)
                    .unwrap();
                topic0 == symbol_short!("depr_call") && topic1 == sym(&env, "verify_r")
            } else {
                false
            }
        });
        assert!(found, "depr_call event must be present with correct entrypoint");
    }

    #[test]
    fn test_record_call_emits_event_with_caller() {
        let env = Env::default();
        env.mock_all_auths();

        let caller = Address::generate(&env);

        DeprecatedRegistry::record_call(
            &env,
            &caller,
            &sym(&env, "old_fn"),
            &sym(&env, "new_fn"),
        );

        let contract_events = env.events().all();
        let events = contract_events.events();
        assert!(!events.is_empty(), "must emit at least one event");

        // The first topic in the tuple is the event type, entrypoint is second
        let found = events.iter().any(|e| {
            if let soroban_sdk::xdr::ContractEventBody::V0(v0) = &e.body {
                v0.topics
                    .get(0)
                    .map(|t| t.clone().try_into_val(&env).ok())
                    == Some(Some(symbol_short!("depr_call")))
            } else {
                false
            }
        });
        assert!(found, "depr_call topic must be present");
    }

    // -----------------------------------------------------------------------
    // register – event emission
    // -----------------------------------------------------------------------

    #[test]
    fn test_register_emits_event() {
        let (env, admin) = setup_env_with_admin();

        DeprecatedRegistry::register(
            &env,
            &admin,
            sym(&env, "dep_fn"),
            sym(&env, "new_fn"),
            None,
        )
        .unwrap();

        assert!(env.events().all().events().len() > 0);
    }

    // -----------------------------------------------------------------------
    // remove – event emission
    // -----------------------------------------------------------------------

    #[test]
    fn test_remove_emits_event_when_found() {
        let (env, admin) = setup_env_with_admin();

        DeprecatedRegistry::register(&env, &admin, sym(&env, "fn"), sym(&env, "nfn"), None)
            .unwrap();

        let events_before = env.events().all().events().len();

        DeprecatedRegistry::remove(&env, &admin, sym(&env, "fn")).unwrap();

        assert!(env.events().all().events().len() > events_before);
    }

    #[test]
    fn test_remove_no_event_when_absent() {
        let (env, admin) = setup_env_with_admin();

        let events_before = env.events().all().events().len();

        DeprecatedRegistry::remove(&env, &admin, sym(&env, "ghost")).unwrap();

        // No new events should have been emitted.
        assert_eq!(env.events().all().events().len(), events_before);
    }

    // -----------------------------------------------------------------------
    // re-register after removal
    // -----------------------------------------------------------------------

    #[test]
    fn test_reregister_after_removal() {
        let (env, admin) = setup_env_with_admin();

        DeprecatedRegistry::register(&env, &admin, sym(&env, "fn"), sym(&env, "nfn"), None)
            .unwrap();
        DeprecatedRegistry::remove(&env, &admin, sym(&env, "fn")).unwrap();

        // Re-register with a different replacement.
        DeprecatedRegistry::register(
            &env,
            &admin,
            sym(&env, "fn"),
            sym(&env, "nfn2"),
            opt_note(&env, "updated"),
        )
        .unwrap();

        let entry = DeprecatedRegistry::get_entry(&env, &sym(&env, "fn")).unwrap();
        assert_eq!(entry.replacement, sym(&env, "nfn2"));
    }
}
