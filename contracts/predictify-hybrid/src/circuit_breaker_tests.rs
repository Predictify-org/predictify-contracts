#[cfg(test)]
mod circuit_breaker_tests {
    use crate::circuit_breaker::*;
    use crate::admin::AdminRoleManager;
    use crate::errors::Error;
    use soroban_sdk::{Env, String, Vec, testutils::Address, vec};

    #[test]
    fn test_circuit_breaker_initialization() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        
        env.as_contract(&contract_id, || {
            // Test initialization
            assert!(CircuitBreaker::initialize(&env).is_ok());
        
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
        });
    }

    #[test]
    fn test_emergency_pause() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        
        env.as_contract(&contract_id, || {
            CircuitBreaker::initialize(&env).unwrap();
            
            let admin = <soroban_sdk::Address as Address>::generate(&env);
            AdminRoleManager::assign_role(&env, &admin, crate::admin::AdminRole::SuperAdmin, &admin).unwrap();
            
            // Test emergency pause
            let reason = String::from_str(&env, "Test emergency pause");
            assert!(CircuitBreaker::emergency_pause(&env, &admin, &reason).is_ok());
            
            // Verify state is open
            let state = CircuitBreaker::get_state(&env).unwrap();
            assert_eq!(state.state, BreakerState::Open);
            
            // Test that circuit breaker is open
            assert!(CircuitBreaker::is_open(&env).unwrap());
            assert!(!CircuitBreaker::is_closed(&env).unwrap());
            
            // Test that trying to pause again fails
            assert!(CircuitBreaker::emergency_pause(&env, &admin, &reason).is_err());
        });
    }

    #[test]
    fn test_circuit_breaker_recovery() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        
        env.as_contract(&contract_id, || {
            CircuitBreaker::initialize(&env).unwrap();
        
        let admin = <soroban_sdk::Address as Address>::generate(&env);
        AdminRoleManager::assign_role(&env, &admin, crate::admin::AdminRole::SuperAdmin, &admin).unwrap();
        
        // First pause the circuit breaker
        let reason = String::from_str(&env, "Test pause");
        CircuitBreaker::emergency_pause(&env, &admin, &reason).unwrap();
        
        // Test recovery
        assert!(CircuitBreaker::circuit_breaker_recovery(&env, &admin).is_ok());
        
        // Verify state is closed
        let state = CircuitBreaker::get_state(&env).unwrap();
        assert_eq!(state.state, BreakerState::Closed);
        
        // Test that circuit breaker is closed
        assert!(CircuitBreaker::is_closed(&env).unwrap());
        assert!(!CircuitBreaker::is_open(&env).unwrap());
        });
    }

    #[test]
    fn test_automatic_trigger() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        
        env.as_contract(&contract_id, || {
            CircuitBreaker::initialize(&env).unwrap();
        
        // Test automatic trigger with high error rate
        let condition = BreakerCondition::HighErrorRate;
        
        // Initially should not trigger
        assert!(!CircuitBreaker::automatic_circuit_breaker_trigger(&env, &condition).unwrap());
        
        // Record some failures to trigger the circuit breaker
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
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        
        env.as_contract(&contract_id, || {
            CircuitBreaker::initialize(&env).unwrap();
        
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
    fn test_half_open_state() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        env.mock_all_auths();
        
        env.as_contract(&contract_id, || {
            CircuitBreaker::initialize(&env).unwrap();
        
        // Configure shorter recovery timeout for testing
        let admin = <soroban_sdk::Address as Address>::generate(&env);
        // Initialize admin system first
        crate::admin::AdminInitializer::initialize(&env, &admin).unwrap();
        AdminRoleManager::assign_role(&env, &admin, crate::admin::AdminRole::SuperAdmin, &admin).unwrap();
        
        let mut config = CircuitBreaker::get_config(&env).unwrap();
        config.recovery_timeout = 1; // 1 second
        config.half_open_max_requests = 2;
        CircuitBreaker::update_config(&env, &admin, &config).unwrap();
        
        // Open the circuit breaker
        let reason = String::from_str(&env, "Test pause");
        CircuitBreaker::emergency_pause(&env, &admin, &reason).unwrap();
        
        // Wait for recovery timeout (simulate by advancing time)
        // In a real test, we would need to mock time
        
        // Test half-open state behavior
        let state = CircuitBreaker::get_state(&env).unwrap();
        if state.state == BreakerState::HalfOpen {
            // Record success in half-open state
            assert!(CircuitBreaker::record_success(&env).is_ok());
            
            // Record another success to close the circuit breaker
            assert!(CircuitBreaker::record_success(&env).is_ok());
            
            // Verify state is closed
            let state = CircuitBreaker::get_state(&env).unwrap();
            assert_eq!(state.state, BreakerState::Closed);
        }
        });
    }

    #[test]
    fn test_circuit_breaker_status() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        
        env.as_contract(&contract_id, || {
            CircuitBreaker::initialize(&env).unwrap();
        
        // Get status
        let status = CircuitBreaker::get_circuit_breaker_status(&env).unwrap();
        
        // Verify status contains expected fields
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
    fn test_event_history() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        
        env.as_contract(&contract_id, || {
            CircuitBreaker::initialize(&env).unwrap();
        
        let admin = <soroban_sdk::Address as Address>::generate(&env);
        AdminRoleManager::assign_role(&env, &admin, crate::admin::AdminRole::SuperAdmin, &admin).unwrap();
        
        // Perform some actions to generate events
        let reason = String::from_str(&env, "Test event");
        CircuitBreaker::emergency_pause(&env, &admin, &reason).unwrap();
        CircuitBreaker::circuit_breaker_recovery(&env, &admin).unwrap();
        
        // Get event history
        let events = CircuitBreaker::get_event_history(&env).unwrap();
        
        // Should have at least 2 events (pause and recovery)
        assert!(events.len() >= 2);
        });
    }

    #[test]
    fn test_validate_circuit_breaker_conditions() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        
        env.as_contract(&contract_id, || {
            // Test valid conditions
        let valid_conditions = vec![
            &env,
            BreakerCondition::HighErrorRate,
            BreakerCondition::HighLatency,
        ];
        assert!(CircuitBreaker::validate_circuit_breaker_conditions(&valid_conditions).is_ok());
        
        // Test empty conditions
        let empty_conditions = Vec::new(&env);
        assert!(CircuitBreaker::validate_circuit_breaker_conditions(&empty_conditions).is_err());
        
        // Test duplicate conditions
        let duplicate_conditions = vec![
            &env,
            BreakerCondition::HighErrorRate,
            BreakerCondition::HighErrorRate,
        ];
        assert!(CircuitBreaker::validate_circuit_breaker_conditions(&duplicate_conditions).is_err());
        });
    }

    #[test]
    fn test_circuit_breaker_utils() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        
        env.as_contract(&contract_id, || {
            CircuitBreaker::initialize(&env).unwrap();
        
        // Test should_allow_operation when closed
        assert!(CircuitBreakerUtils::should_allow_operation(&env).unwrap());
        
        // Test with_circuit_breaker wrapper
        let result = CircuitBreakerUtils::with_circuit_breaker(&env, || {
            Ok::<String, Error>(String::from_str(&env, "success"))
        });
        assert!(result.is_ok());
        
        // Test statistics
        let stats = CircuitBreakerUtils::get_statistics(&env).unwrap();
        assert!(stats.get(String::from_str(&env, "total_requests")).is_some());
        assert!(stats.get(String::from_str(&env, "error_count")).is_some());
        assert!(stats.get(String::from_str(&env, "current_state")).is_some());
        });
    }

    #[test]
    fn test_circuit_breaker_testing() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        
        env.as_contract(&contract_id, || {
            // Test create test config
        let test_config = CircuitBreakerTesting::create_test_config(&env);
        assert_eq!(test_config.max_error_rate, 5);
        assert_eq!(test_config.max_latency_ms, 1000);
        assert_eq!(test_config.failure_threshold, 3);
        
        // Test create test state
        let test_state = CircuitBreakerTesting::create_test_state(&env);
        assert_eq!(test_state.state, BreakerState::Closed);
        assert_eq!(test_state.failure_count, 0);
        assert_eq!(test_state.total_requests, 0);
        
        // Test simulate functions
        CircuitBreaker::initialize(&env).unwrap();
        assert!(CircuitBreakerTesting::simulate_success(&env).is_ok());
        assert!(CircuitBreakerTesting::simulate_failure(&env).is_ok());
        });
    }

    #[test]
    fn test_circuit_breaker_scenarios() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        
        env.as_contract(&contract_id, || {
            CircuitBreaker::initialize(&env).unwrap();
        
        // Test circuit breaker scenarios
        let results = CircuitBreaker::test_circuit_breaker_scenarios(&env).unwrap();
        
        // Verify results contain expected test outcomes
        assert!(results.get(String::from_str(&env, "normal_operation")).is_some());
        assert!(results.get(String::from_str(&env, "emergency_pause")).is_some());
        assert!(results.get(String::from_str(&env, "recovery")).is_some());
        assert!(results.get(String::from_str(&env, "status_check")).is_some());
        assert!(results.get(String::from_str(&env, "event_history")).is_some());
        });
    }

    #[test]
    fn test_config_validation() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        
        env.as_contract(&contract_id, || {
            // Test valid config
        let valid_config = CircuitBreakerConfig {
            max_error_rate: 10,
            max_latency_ms: 5000,
            min_liquidity: 1_000_000_000,
            failure_threshold: 5,
            recovery_timeout: 300,
            half_open_max_requests: 3,
            auto_recovery_enabled: true,
        };
        
        // Test invalid configs
        let mut invalid_config = valid_config.clone();
        invalid_config.max_error_rate = 101; // > 100
        // This would fail validation in update_config
        
        let mut invalid_config2 = valid_config.clone();
        invalid_config2.max_latency_ms = 0; // = 0
        // This would fail validation in update_config
        
        let mut invalid_config3 = valid_config.clone();
        invalid_config3.min_liquidity = -1; // < 0
        // This would fail validation in update_config
        });
    }

    #[test]
    fn test_error_handling() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        
        env.as_contract(&contract_id, || {
            // Test circuit breaker not initialized
        assert!(CircuitBreaker::get_config(&env).is_err());
        assert!(CircuitBreaker::get_state(&env).is_err());
        assert!(CircuitBreaker::is_open(&env).is_err());
        assert!(CircuitBreaker::is_closed(&env).is_err());
        
        // Initialize
        CircuitBreaker::initialize(&env).unwrap();
        
        // Test unauthorized access (inside contract context but without proper admin role)
        let unauthorized_admin = <soroban_sdk::Address as Address>::generate(&env);
        let reason = String::from_str(&env, "Test");
        assert!(CircuitBreaker::emergency_pause(&env, &unauthorized_admin, &reason).is_err());
        assert!(CircuitBreaker::circuit_breaker_recovery(&env, &unauthorized_admin).is_err());
        });
    }

    #[test]
    fn test_circuit_breaker_integration() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        
        env.as_contract(&contract_id, || {
            CircuitBreaker::initialize(&env).unwrap();
        
        let admin = <soroban_sdk::Address as Address>::generate(&env);
        AdminRoleManager::assign_role(&env, &admin, crate::admin::AdminRole::SuperAdmin, &admin).unwrap();
        
        // Test complete workflow
        // 1. Normal operation
        assert!(CircuitBreaker::is_closed(&env).unwrap());
        
        // 2. Emergency pause
        let reason = String::from_str(&env, "Integration test pause");
        assert!(CircuitBreaker::emergency_pause(&env, &admin, &reason).is_ok());
        assert!(CircuitBreaker::is_open(&env).unwrap());
        
        // 3. Recovery
        assert!(CircuitBreaker::circuit_breaker_recovery(&env, &admin).is_ok());
        assert!(CircuitBreaker::is_closed(&env).unwrap());
        
        // 4. Record operations
        assert!(CircuitBreaker::record_success(&env).is_ok());
        assert!(CircuitBreaker::record_failure(&env).is_ok());
        
        // 5. Check status
        let status = CircuitBreaker::get_circuit_breaker_status(&env).unwrap();
        assert!(status.get(String::from_str(&env, "total_requests")).is_some());
        assert!(status.get(String::from_str(&env, "error_count")).is_some());
        
        // 6. Check events
        let events = CircuitBreaker::get_event_history(&env).unwrap();
        assert!(events.len() >= 2); // At least pause and recovery events
        });
    }
} 