#[cfg(test)]
#[allow(unused_assignments)]
#[allow(unused_variables)]
#[allow(dead_code)]
mod circuit_breaker_tests {
    use crate::admin::{AdminRole, AdminRoleAssignment, AdminRoleManager};
    use crate::circuit_breaker::*;
    use crate::err::Error;
    use soroban_sdk::{
        testutils::{Address as _, Ledger, LedgerInfo},
        vec, Address, Env, String, Symbol, Vec,
    };

    fn setup_test() -> (Env, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let admin = Address::generate(&env);

        env.as_contract(&contract_id, || {
            // Set primary admin
            env.storage()
                .persistent()
                .set(&Symbol::new(&env, "Admin"), &admin);

            // Assign SuperAdmin role to admin
            let key = Symbol::new(&env, "admin_role");
            let assignment = AdminRoleAssignment {
                admin: admin.clone(),
                role: AdminRole::SuperAdmin,
                assigned_by: admin.clone(),
                assigned_at: env.ledger().timestamp(),
                permissions: AdminRoleManager::get_permissions_for_role(&env, &AdminRole::SuperAdmin),
                is_active: true,
            };
            env.storage().persistent().set(&key, &assignment);

            // Initialize circuit breaker
            CircuitBreaker::initialize(&env).unwrap();
        });

        (env, contract_id, admin)
    }

    #[test]
    fn test_circuit_breaker_initialization() {
        let (env, contract_id, _admin) = setup_test();

        env.as_contract(&contract_id, || {
            // Test get config
            let config = CircuitBreaker::get_config(&env).unwrap();
            assert_eq!(config.max_error_rate, 10);
            assert_eq!(config.max_latency_ms, 5000);
            assert_eq!(config.min_liquidity, 1_000_000_000);
            assert_eq!(config.failure_threshold, 5);
            assert_eq!(config.recovery_timeout, 300);
            assert_eq!(config.half_open_max_requests, 3);
            assert!(config.auto_recovery_enabled);

            // Test get state
            let state = CircuitBreaker::get_state(&env).unwrap();
            assert_eq!(state.state, BreakerState::Closed);
            assert_eq!(state.failure_count, 0);
            assert_eq!(state.total_requests, 0);
            assert_eq!(state.error_count, 0);
            assert_eq!(state.half_open_since, 0);
            assert_eq!(state.half_open_requests, 0);
        });
    }

    #[test]
    fn test_emergency_pause() {
        let (env, contract_id, admin) = setup_test();

        // 1. Initial pause
        env.as_contract(&contract_id, || {
            let reason = String::from_str(&env, "Test emergency pause");
            assert!(CircuitBreaker::emergency_pause(&env, &admin, &reason).is_ok());

            // Verify state is open
            let state = CircuitBreaker::get_state(&env).unwrap();
            assert_eq!(state.state, BreakerState::Open);

            // Test that circuit breaker is open
            assert!(CircuitBreaker::is_open(&env).unwrap());
            assert!(!CircuitBreaker::is_closed(&env).unwrap());
        });

        // 2. Second pause attempt in fresh auth frame must fail with CBError
        env.as_contract(&contract_id, || {
            let reason = String::from_str(&env, "Test emergency pause");
            assert_eq!(
                CircuitBreaker::emergency_pause(&env, &admin, &reason).unwrap_err(),
                Error::CBError
            );
        });
    }

    #[test]
    fn test_circuit_breaker_recovery() {
        let (env, contract_id, admin) = setup_test();

        // First pause the circuit breaker
        env.as_contract(&contract_id, || {
            let reason = String::from_str(&env, "Test pause");
            CircuitBreaker::emergency_pause(&env, &admin, &reason).unwrap();
            assert!(CircuitBreaker::is_open(&env).unwrap());
        });

        // Test recovery in a separate frame
        env.as_contract(&contract_id, || {
            assert!(CircuitBreaker::circuit_breaker_recovery(&env, &admin).is_ok());

            // Verify state is closed
            let state = CircuitBreaker::get_state(&env).unwrap();
            assert_eq!(state.state, BreakerState::Closed);
            assert!(CircuitBreaker::is_closed(&env).unwrap());
            assert!(!CircuitBreaker::is_open(&env).unwrap());
        });
    }

    #[test]
    fn test_automatic_trigger_on_repeated_failures() {
        let (env, contract_id, _admin) = setup_test();

        env.as_contract(&contract_id, || {
            let condition = BreakerCondition::HighErrorRate;

            // Initially should not trigger
            assert!(!CircuitBreaker::automatic_circuit_breaker_trigger(&env, &condition).unwrap());

            // Record failures up to threshold
            for _ in 0..10 {
                CircuitBreaker::record_failure(&env).unwrap();
            }

            // Now should trigger
            assert!(CircuitBreaker::automatic_circuit_breaker_trigger(&env, &condition).unwrap());

            // Verify state is open
            let state = CircuitBreaker::get_state(&env).unwrap();
            assert_eq!(state.state, BreakerState::Open);
        });
    }

    #[test]
    fn test_record_success_and_failure() {
        let (env, contract_id, _admin) = setup_test();

        env.as_contract(&contract_id, || {
            // Test recording success
            assert!(CircuitBreaker::record_success(&env).is_ok());

            let state = CircuitBreaker::get_state(&env).unwrap();
            assert_eq!(state.total_requests, 1);
            assert_eq!(state.error_count, 0);

            // Test recording failure
            assert!(CircuitBreaker::record_failure(&env).is_ok());

            let state = CircuitBreaker::get_state(&env).unwrap();
            assert_eq!(state.total_requests, 2);
            assert_eq!(state.error_count, 1);
        });
    }

    #[test]
    fn test_pause_keeps_reads_available_but_blocks_writes() {
        let (env, contract_id, admin) = setup_test();

        env.as_contract(&contract_id, || {
            let reason = String::from_str(&env, "pause for audit");
            CircuitBreaker::pause_with_options(
                &env,
                &admin,
                &reason,
                PauseScope::Full,
                false,
            )
            .unwrap();

            // Reads remain allowed while breaker is open
            assert!(CircuitBreaker::is_read_allowed(&env).unwrap());

            // Writes are blocked
            assert!(!CircuitBreaker::is_write_allowed(&env, "deposit").unwrap());
            assert!(!CircuitBreaker::is_write_allowed(&env, "betting").unwrap());
            assert!(!CircuitBreaker::is_write_allowed(&env, "create_market").unwrap());

            let write_err = CircuitBreaker::require_write_allowed(&env, "deposit").unwrap_err();
            assert_eq!(write_err, Error::CBOpen);
        });
    }

    #[test]
    fn test_betting_only_pause_allows_non_betting_writes() {
        let (env, contract_id, admin) = setup_test();

        env.as_contract(&contract_id, || {
            let reason = String::from_str(&env, "betting paused");
            CircuitBreaker::pause_with_options(
                &env,
                &admin,
                &reason,
                PauseScope::BettingOnly,
                false,
            )
            .unwrap();

            // Betting is blocked
            assert!(!CircuitBreaker::is_write_allowed(&env, "betting").unwrap());
            assert_eq!(
                CircuitBreaker::require_write_allowed(&env, "betting").unwrap_err(),
                Error::CBOpen
            );

            // Non-betting writes are still allowed
            assert!(CircuitBreaker::is_write_allowed(&env, "create_market").unwrap());
            assert!(CircuitBreaker::is_write_allowed(&env, "deposit").unwrap());
        });
    }

    #[test]
    fn test_unauthorized_recovery_and_pause_rejected() {
        let (env, contract_id, _admin) = setup_test();
        let unauthorized_user = Address::generate(&env);

        let reason = String::from_str(&env, "unauthorized pause");

        // Non-admin cannot emergency pause
        env.as_contract(&contract_id, || {
            let pause_res = CircuitBreaker::emergency_pause(&env, &unauthorized_user, &reason);
            assert_eq!(pause_res.unwrap_err(), Error::Unauthorized);
        });

        // Non-admin cannot request resume
        env.as_contract(&contract_id, || {
            let resume_res = CircuitBreaker::request_resume(&env, &unauthorized_user);
            assert_eq!(resume_res.unwrap_err(), Error::Unauthorized);
        });

        // Non-admin cannot recover circuit breaker
        env.as_contract(&contract_id, || {
            let recover_res = CircuitBreaker::circuit_breaker_recovery(&env, &unauthorized_user);
            assert_eq!(recover_res.unwrap_err(), Error::Unauthorized);
        });
    }

    #[test]
    fn test_half_open_cooldown_not_elapsed() {
        let (env, contract_id, admin) = setup_test();

        // 1. Open the breaker
        env.as_contract(&contract_id, || {
            let reason = String::from_str(&env, "pause for cooldown test");
            CircuitBreaker::emergency_pause(&env, &admin, &reason).unwrap();
            assert_eq!(CircuitBreaker::get_state(&env).unwrap().state, BreakerState::Open);
        });

        // 2. Admin requests resume -> HalfOpen (records half_open_since)
        env.as_contract(&contract_id, || {
            CircuitBreaker::request_resume(&env, &admin).unwrap();
            let state = CircuitBreaker::get_state(&env).unwrap();
            assert_eq!(state.state, BreakerState::HalfOpen);
            assert_eq!(state.half_open_since, env.ledger().timestamp());
            assert_eq!(state.half_open_requests, 0);

            // 3. Record success immediately before recovery_timeout has elapsed
            // Default recovery_timeout is 300s. Current ledger timestamp is 0.
            CircuitBreaker::record_success(&env).unwrap();

            // Probe is ignored for half_open_requests counter while cooldown is active
            let state_after = CircuitBreaker::get_state(&env).unwrap();
            assert_eq!(state_after.half_open_requests, 0);
            assert_eq!(state_after.state, BreakerState::HalfOpen);
        });
    }

    #[test]
    fn test_half_open_probe_failure_reopens() {
        let (env, contract_id, admin) = setup_test();

        // 1. Pause
        env.as_contract(&contract_id, || {
            let reason = String::from_str(&env, "pause for probe failure test");
            CircuitBreaker::emergency_pause(&env, &admin, &reason).unwrap();
        });

        // 2. Request resume -> HalfOpen
        env.as_contract(&contract_id, || {
            CircuitBreaker::request_resume(&env, &admin).unwrap();
            assert_eq!(
                CircuitBreaker::get_state(&env).unwrap().state,
                BreakerState::HalfOpen
            );
        });

        // 3. A failure during HalfOpen must reopen the breaker immediately
        env.as_contract(&contract_id, || {
            CircuitBreaker::record_failure(&env).unwrap();

            let state = CircuitBreaker::get_state(&env).unwrap();
            assert_eq!(
                state.state,
                BreakerState::Open,
                "failure during HalfOpen must reopen the breaker"
            );
            assert_eq!(state.half_open_since, 0, "half_open_since must be cleared");
            assert_eq!(state.half_open_requests, 0);
        });
    }

    #[test]
    fn test_half_open_probe_success_threshold_closes_after_cooldown() {
        let (env, contract_id, admin) = setup_test();

        // 1. Config and Pause
        env.as_contract(&contract_id, || {
            let mut config = CircuitBreaker::get_config(&env).unwrap();
            config.recovery_timeout = 10;
            config.half_open_max_requests = 3;
            env.storage()
                .instance()
                .set(&Symbol::new(&env, "circuit_breaker_config"), &config);

            let reason = String::from_str(&env, "pause for threshold test");
            CircuitBreaker::emergency_pause(&env, &admin, &reason).unwrap();
        });

        // 2. Request resume -> HalfOpen
        env.as_contract(&contract_id, || {
            CircuitBreaker::request_resume(&env, &admin).unwrap();
            assert_eq!(
                CircuitBreaker::get_state(&env).unwrap().state,
                BreakerState::HalfOpen
            );
        });

        // 3. Advance ledger timestamp beyond cooldown (10s)
        let ledger = env.ledger();
        env.ledger().set(LedgerInfo {
            timestamp: ledger.timestamp() + 15,
            protocol_version: ledger.protocol_version(),
            sequence_number: ledger.sequence() + 1,
            network_id: ledger.network_id().into(),
            base_reserve: 10,
            min_temp_entry_ttl: 1,
            min_persistent_entry_ttl: 1,
            max_entry_ttl: 1_000_000,
        });

        // 4. Record successes after cooldown has elapsed
        env.as_contract(&contract_id, || {
            for _ in 0..3 {
                CircuitBreaker::record_success(&env).unwrap();
            }

            let state = CircuitBreaker::get_state(&env).unwrap();
            assert_eq!(
                state.state,
                BreakerState::Closed,
                "breaker must close after reaching the probe success threshold"
            );
            assert_eq!(state.half_open_since, 0);
            assert_eq!(state.failure_count, 0);
        });
    }

    #[test]
    fn test_event_history() {
        let (env, contract_id, admin) = setup_test();

        env.as_contract(&contract_id, || {
            let reason = String::from_str(&env, "Test event");
            CircuitBreaker::emergency_pause(&env, &admin, &reason).unwrap();
        });

        env.as_contract(&contract_id, || {
            CircuitBreaker::circuit_breaker_recovery(&env, &admin).unwrap();
        });

        env.as_contract(&contract_id, || {
            let events = CircuitBreaker::get_event_history(&env).unwrap();
            assert!(events.len() >= 2);

            let first_event = events.get(0).unwrap();
            assert_eq!(first_event.action, BreakerAction::Pause);

            let second_event = events.get(1).unwrap();
            assert_eq!(second_event.action, BreakerAction::Resume);
        });
    }

    #[test]
    fn test_circuit_breaker_status() {
        let (env, contract_id, _admin) = setup_test();

        env.as_contract(&contract_id, || {
            let status = CircuitBreaker::get_circuit_breaker_status(&env).unwrap();
            assert!(status.get(String::from_str(&env, "state")).is_some());
            assert!(status.get(String::from_str(&env, "failure_count")).is_some());
            assert!(status.get(String::from_str(&env, "total_requests")).is_some());
            assert!(status.get(String::from_str(&env, "error_count")).is_some());
            assert!(status.get(String::from_str(&env, "max_error_rate")).is_some());
            assert!(status.get(String::from_str(&env, "failure_threshold")).is_some());
            assert!(status.get(String::from_str(&env, "auto_recovery_enabled")).is_some());
        });
    }

    #[test]
    fn test_validate_circuit_breaker_conditions() {
        let env = Env::default();
        let valid_conditions = vec![
            &env,
            BreakerCondition::HighErrorRate,
            BreakerCondition::HighLatency,
        ];
        assert!(CircuitBreaker::validate_circuit_breaker_conditions(&valid_conditions).is_ok());

        let empty_conditions = Vec::new(&env);
        assert!(CircuitBreaker::validate_circuit_breaker_conditions(&empty_conditions).is_err());

        let duplicate_conditions = vec![
            &env,
            BreakerCondition::HighErrorRate,
            BreakerCondition::HighErrorRate,
        ];
        assert!(CircuitBreaker::validate_circuit_breaker_conditions(&duplicate_conditions).is_err());
    }

    #[test]
    fn test_circuit_breaker_utils() {
        let (env, contract_id, _admin) = setup_test();

        env.as_contract(&contract_id, || {
            assert!(CircuitBreakerUtils::should_allow_operation(&env).unwrap());

            let result = CircuitBreakerUtils::with_circuit_breaker(&env, || {
                Ok::<String, Error>(String::from_str(&env, "success"))
            });
            assert!(result.is_ok());

            let stats = CircuitBreakerUtils::get_statistics(&env).unwrap();
            assert!(stats.get(String::from_str(&env, "total_requests")).is_some());
            assert!(stats.get(String::from_str(&env, "error_count")).is_some());
            assert!(stats.get(String::from_str(&env, "current_state")).is_some());
        });
    }

    #[test]
    fn test_circuit_breaker_testing_utilities() {
        let (env, contract_id, _admin) = setup_test();

        let test_config = CircuitBreakerTesting::create_test_config(&env);
        assert_eq!(test_config.max_error_rate, 5);
        assert_eq!(test_config.max_latency_ms, 1000);
        assert_eq!(test_config.failure_threshold, 3);

        let test_state = CircuitBreakerTesting::create_test_state(&env);
        assert_eq!(test_state.state, BreakerState::Closed);
        assert_eq!(test_state.failure_count, 0);

        env.as_contract(&contract_id, || {
            assert!(CircuitBreakerTesting::simulate_success(&env).is_ok());
            assert!(CircuitBreakerTesting::simulate_failure(&env).is_ok());
        });
    }
}
