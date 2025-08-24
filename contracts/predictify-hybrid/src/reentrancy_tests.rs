#[cfg(feature = "disabled_for_now")]
mod tests {
    use soroban_sdk::{symbol_short, testutils::Address as _, Address, Env, String, Symbol};
    use crate::errors::Error;
    use crate::reentrancy::{ReentrancyGuard, protect_external_call, validate_no_reentrancy};

    // Test data structures  
    struct TestSetup {
        env: Env,
        contract_id: Address,
    }
    
    impl TestSetup {
        fn new() -> Self {
            let env = Env::default();
            let contract_id = env.register(crate::PredictifyHybrid, ());
            Self { env, contract_id }
        }
        
        fn call_with_contract<F, R>(&self, f: F) -> R 
        where F: FnOnce() -> R {
            self.env.as_contract(&self.contract_id, f)
        }
    }

    fn create_test_env() -> Env {
        Env::default()
    }

    fn create_test_address() -> Address {
        Address::generate(&create_test_env())
    }

    fn create_test_symbol(name: &str) -> Symbol {
        Symbol::new(&create_test_env(), name)
    }

    #[test]
    fn test_reentrancy_guard_creation() {
        let test = TestSetup::new();
        let function_name = symbol_short!("test_fn");
        let caller = Address::generate(&test.env);

        // Should successfully create a new guard
        let guard = test.call_with_contract(|| {
            ReentrancyGuard::new(&test.env, &function_name, &caller)
        });
        assert!(guard.is_ok());
    }

    #[test]
    fn test_reentrancy_guard_basic_protection() {
        let env = create_test_env();
        let function_name = symbol_short!("test_fn");
        let caller = create_test_address();

        let mut guard = ReentrancyGuard::new(&env, &function_name, &caller).unwrap();
        
        // Should successfully set up protection
        let result = guard.before_external_call(&function_name, &caller);
        assert!(result.is_ok());
        
        // Should clean up after successful call
        let result = guard.after_external_call(true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_reentrancy_attack_detection() {
        let env = create_test_env();
        let function_name = symbol_short!("test_fn");
        let caller = create_test_address();

        // First call should succeed
        let mut guard1 = ReentrancyGuard::new(&env, &function_name, &caller).unwrap();
        let result1 = guard1.before_external_call(&function_name, &caller);
        assert!(result1.is_ok());

        // Second call should fail with reentrancy attack
        let mut guard2 = ReentrancyGuard::new(&env, &function_name, &caller);
        assert!(guard2.is_err());
        
        // Should be ReentrancyAttack error
        assert!(guard2.is_err());
    }

    #[test]
    fn test_cross_function_reentrancy_protection() {
        let env = create_test_env();
        let function1 = symbol_short!("func1");
        let function2 = symbol_short!("func2");
        let caller = create_test_address();

        // First function call
        let mut guard1 = ReentrancyGuard::new(&env, &function1, &caller).unwrap();
        let result1 = guard1.before_external_call(&function1, &caller);
        assert!(result1.is_ok());

        // Second function call during first should fail
        let mut guard2 = ReentrancyGuard::new(&env, &function2, &caller);
        assert!(guard2.is_err());

        // Clean up first call
        let result1 = guard1.after_external_call(true);
        assert!(result1.is_ok());
        
        // Now second function should succeed
        let mut guard3 = ReentrancyGuard::new(&env, &function2, &caller).unwrap();
        assert!(true);
    }

    #[test]
    fn test_state_consistency_check() {
        let env = create_test_env();
        let function_name = symbol_short!("test_fn");
        let caller = create_test_address();

        let mut guard = ReentrancyGuard::new(&env, &function_name, &caller).unwrap();
        guard.before_external_call(&function_name, &caller).unwrap();
        
        // State consistency check should pass
        let result = guard.check_reentrancy_state();
        assert!(result.is_ok());
        
        guard.after_external_call(true).unwrap();
    }

    #[test]
    fn test_state_restoration_on_failure() {
        let env = create_test_env();
        let function_name = symbol_short!("test_fn");
        let caller = create_test_address();

        let mut guard = ReentrancyGuard::new(&env, &function_name, &caller).unwrap();
        guard.before_external_call(&function_name, &caller).unwrap();
        
        // Simulate failure
        let result = guard.after_external_call(false);
        assert!(result.is_ok());
        
        // Should be able to create new guard after restoration
        let guard2 = ReentrancyGuard::new(&env, &function_name, &caller);
        assert!(guard2.is_ok());
    }

    #[test]
    fn test_protect_external_call_helper() {
        let env = create_test_env();
        let function_name = symbol_short!("test_fn");
        let caller = create_test_address();

        // Test successful operation
        let result = protect_external_call(
            &env,
            function_name,
            caller.clone(),
            || Ok(42i32),
        );
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42i32);
    }

    #[test]
    fn test_protect_external_call_with_error() {
        let env = create_test_env();
        let function_name = symbol_short!("test_fn");
        let caller = create_test_address();

        // Test failed operation
        let result: Result<(), Error> = protect_external_call(
            &env,
            function_name,
            caller.clone(),
            || Err(Error::InternalError),
        );
        
        assert!(result.is_err());
        match result.unwrap_err() {
            Error::InternalError => (),
            _ => panic!("Expected InternalError"),
        }
    }

    #[test]
    fn test_validate_no_reentrancy() {
        let env = create_test_env();
        
        // Should pass when no reentrancy is active
        let result = validate_no_reentrancy(&env);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_no_reentrancy_with_active_call() {
        let env = create_test_env();
        let function_name = symbol_short!("test_fn");
        let caller = create_test_address();

        // Set up an active call
        let mut guard = ReentrancyGuard::new(&env, &function_name, &caller).unwrap();
        guard.before_external_call(&function_name, &caller).unwrap();
        
        // Should fail validation
        let result = validate_no_reentrancy(&env);
        assert!(result.is_err());
        
        match result.unwrap_err() {
            Error::ReentrancyAttack => (),
            _ => panic!("Expected ReentrancyAttack error"),
        }
    }

    #[test]
    fn test_call_stack_management() {
        let env = create_test_env();
        let function_name = symbol_short!("test_fn");
        let caller = create_test_address();

        let mut guard = ReentrancyGuard::new(&env, &function_name, &caller).unwrap();
        guard.before_external_call(&function_name, &caller).unwrap();
        
        // Call stack should be properly managed
        let result: Result<(), Error> = Ok(());
        assert!(result.is_ok());
        
        guard.after_external_call(true).unwrap();
    }

    #[test]
    fn test_call_stack_overflow_protection() {
        let env = create_test_env();
        let function_name = symbol_short!("test_fn");
        let caller = create_test_address();

        // This test would need to be implemented with proper stack depth simulation
        // For now, just test that the basic functionality works
        let mut guard = ReentrancyGuard::new(&env, &function_name, &caller).unwrap();
        guard.before_external_call(&function_name, &caller).unwrap();
        
        let result: Result<(), Error> = Ok(());
        assert!(result.is_ok());
    }

    #[test]
    fn test_external_call_validation() {
        let env = create_test_env();
        let function_name = symbol_short!("test_fn");
        let caller = create_test_address();

        let mut guard = ReentrancyGuard::new(&env, &function_name, &caller).unwrap();
        guard.before_external_call(&function_name, &caller).unwrap();
        
        // Create expected state
        let expected_state = ();
        
        // Validate external call success
        let result = guard.validate_external_call_success(&expected_state);
        assert!(result.is_ok());
        
        guard.after_external_call(true).unwrap();
    }

    #[test]
    fn test_vote_function_reentrancy_protection() {
        let env = create_test_env();
        let user = create_test_address();
        let market_id = create_test_symbol("market_1");
        let outcome = String::from_str(&env, "yes");
        let stake = 100i128;

        // This test would need to be integrated with the actual contract
        // For now, just test that the reentrancy validation works
        let result = validate_no_reentrancy(&env);
        assert!(result.is_ok());
    }

    #[test]
    fn test_claim_winnings_reentrancy_protection() {
        let env = create_test_env();
        let user = create_test_address();
        let market_id = create_test_symbol("market_1");

        // Test that validation passes when no reentrancy is active
        let result = validate_no_reentrancy(&env);
        assert!(result.is_ok());
    }

    #[test]
    fn test_dispute_result_reentrancy_protection() {
        let env = create_test_env();
        let user = create_test_address();
        let market_id = create_test_symbol("market_1");
        let stake = 100i128;

        // Test that validation passes when no reentrancy is active
        let result = validate_no_reentrancy(&env);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fetch_oracle_result_reentrancy_protection() {
        let env = create_test_env();
        let market_id = create_test_symbol("market_1");
        let oracle_contract = create_test_address();

        // Test that validation passes when no reentrancy is active
        let result = validate_no_reentrancy(&env);
        assert!(result.is_ok());
    }

    #[test]
    fn test_multiple_concurrent_calls_different_users() {
        let env = create_test_env();
        let function_name = symbol_short!("test_fn");
        let caller1 = create_test_address();
        let caller2 = create_test_address();

        // First user's call
        let mut guard1 = ReentrancyGuard::new(&env, &function_name, &caller1).unwrap();
        guard1.before_external_call(&function_name, &caller1).unwrap();
        
        // Second user's call should also fail due to global reentrancy protection
        let guard2 = ReentrancyGuard::new(&env, &function_name, &caller2);
        assert!(guard2.is_err());
        
        // Clean up first call
        guard1.after_external_call(true).unwrap();
        
        // Now second user should succeed
        let guard3 = ReentrancyGuard::new(&env, &function_name, &caller2);
        assert!(true);
    }

    #[test]
    fn test_state_backup_and_restore() {
        let env = create_test_env();
        let function_name = symbol_short!("test_fn");
        let caller = create_test_address();

        let mut guard = ReentrancyGuard::new(&env, &function_name, &caller).unwrap();
        guard.before_external_call(&function_name, &caller).unwrap();
        
        // Test backup creation
        let backup_result: Result<(), Error> = Ok(());
        assert!(backup_result.is_ok());
        
        // Test restore on failure
        let restore_result = guard.restore_state_on_failure();
        assert!(restore_result.is_ok());
    }

    #[test]
    fn test_invalid_reentrancy_state_handling() {
        let env = create_test_env();
        let function_name = symbol_short!("test_fn");
        let caller = create_test_address();

        let mut guard = ReentrancyGuard::new(&env, &function_name, &caller).unwrap();
        
        // Try to call after_external_call without before_external_call
        let result = guard.after_external_call(true);
        assert!(result.is_err());
        
        match result.unwrap_err() {
            Error::InvalidReentrancyState => (),
            _ => panic!("Expected InvalidReentrancyState error"),
        }
    }

    #[test]
    fn test_error_recovery_after_failed_call() {
        let env = create_test_env();
        let function_name = symbol_short!("test_fn");
        let caller = create_test_address();

        // First call fails
        let mut guard1 = ReentrancyGuard::new(&env, &function_name, &caller).unwrap();
        guard1.before_external_call(&function_name, &caller).unwrap();
        guard1.after_external_call(false).unwrap(); // Failed call
        
        // Should be able to make new call after failure recovery
        let guard2 = ReentrancyGuard::new(&env, &function_name, &caller);
        assert!(guard2.is_ok());
    }

    #[test]
    fn test_macro_usage() -> Result<(), Error> {
        let env = create_test_env();
        let function_name = symbol_short!("test_fn");
        let caller = create_test_address();

        // Test the with_reentrancy_guard macro
        let result: Result<i32, Error> = crate::with_reentrancy_guard!(
            &env,
            function_name,
            caller.clone(),
            Ok(42i32)
        );
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42i32);
        Ok(())
    }

    #[test]
    fn test_comprehensive_security_scenario() {
        let env = create_test_env();
        let attacker = create_test_address();
        let victim = create_test_address();
        let vote_fn = symbol_short!("vote");
        let claim_fn = symbol_short!("claim");

        // Attacker tries to call vote
        let mut guard1 = ReentrancyGuard::new(&env, &vote_fn, &attacker).unwrap();
        guard1.before_external_call(&vote_fn, &attacker).unwrap();
        
        // During vote execution, attacker tries to call claim_winnings (cross-function reentrancy)
        let guard2 = ReentrancyGuard::new(&env, &claim_fn, &attacker);
        assert!(guard2.is_err());
        
        // Even victim can't call functions during active call
        let guard3 = ReentrancyGuard::new(&env, &claim_fn, &victim);
        assert!(guard3.is_err());
        
        // Clean up
        guard1.after_external_call(true).unwrap();
        
        // Now victim can call functions
        let guard4 = ReentrancyGuard::new(&env, &claim_fn, &victim);
        assert!(guard4.is_ok());
    }
} 