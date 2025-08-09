use soroban_sdk::{symbol_short, Env, Symbol, Address};
use crate::errors::Error;

// Reentrancy status constants
const REENTRANCY_NOT_ENTERED: u32 = 1;
const REENTRANCY_ENTERED: u32 = 2;

// Storage keys for reentrancy state
const REENTRANCY_STATUS_KEY: Symbol = symbol_short!("REENT_ST");
const CALL_COUNT_KEY: Symbol = symbol_short!("CALL_CNT");

pub struct ReentrancyGuard {
    env: Env,
    #[allow(dead_code)]
    call_id: Symbol,
    is_active: bool,
}

impl ReentrancyGuard {
    /// Create a new reentrancy guard for a specific function call
    pub fn new(env: &Env, _function_name: &Symbol, _caller: &Address) -> Result<Self, Error> {
        // Check if we're already in a reentrant call
        let current_status = Self::get_reentrancy_status(env);
        if current_status == REENTRANCY_ENTERED {
            return Err(Error::ReentrancyAttack);
        }

        let call_id = Self::generate_call_id(env);
        
        let guard = Self {
            env: env.clone(),
            call_id,
            is_active: false,
        };

        Ok(guard)
    }

    /// Execute before making external calls
    pub fn before_external_call(&mut self, _function_name: &Symbol, _caller: &Address) -> Result<(), Error> {
        // Check current reentrancy status
        let current_status = self.get_reentrancy_status_instance();
        if current_status == REENTRANCY_ENTERED {
            return Err(Error::ReentrancyAttack);
        }

        // Set reentrancy status to entered
        self.set_reentrancy_status(REENTRANCY_ENTERED)?;

        // Increment call count
        self.increment_call_count()?;

        self.is_active = true;

        Ok(())
    }

    /// Execute after external calls complete
    pub fn after_external_call(&mut self, success: bool) -> Result<(), Error> {
        if !self.is_active {
            return Err(Error::InvalidReentrancyState);
        }

        // Validate state consistency
        self.check_reentrancy_state()?;

        if success {
            // Decrement call count
            self.decrement_call_count()?;
        } else {
            // On failure, reset the call count and status
            self.reset_call_state()?;
        }

        // Reset reentrancy status
        self.set_reentrancy_status(REENTRANCY_NOT_ENTERED)?;
        
        self.is_active = false;

        Ok(())
    }

    /// Check state consistency during external calls
    pub fn check_reentrancy_state(&self) -> Result<(), Error> {
        // Verify we're in the correct state
        let current_status = self.get_reentrancy_status_instance();
        if current_status != REENTRANCY_ENTERED {
            return Err(Error::InconsistentReentrancyState);
        }

        // Check call count is reasonable
        let call_count = self.get_call_count();
        if call_count > 10 {
            return Err(Error::CallStackOverflow);
        }

        Ok(())
    }

    /// Restore state after failed external calls
    pub fn restore_state_on_failure(&self) -> Result<(), Error> {
        // Reset call state
        self.reset_call_state()?;
        Ok(())
    }

    /// Validate external call success (simplified for Soroban)
    pub fn validate_external_call_success(&self, _expected_state: &()) -> Result<bool, Error> {
        // Simplified validation - just check no unexpected reentrancy occurred
        let current_status = self.get_reentrancy_status_instance();
        Ok(current_status == REENTRANCY_ENTERED)
    }

    // Private helper methods

    fn get_reentrancy_status_instance(&self) -> u32 {
        self.env
            .storage()
            .instance()
            .get(&REENTRANCY_STATUS_KEY)
            .unwrap_or(REENTRANCY_NOT_ENTERED)
    }

    fn get_reentrancy_status(env: &Env) -> u32 {
        env
            .storage()
            .instance()
            .get(&REENTRANCY_STATUS_KEY)
            .unwrap_or(REENTRANCY_NOT_ENTERED)
    }

    fn set_reentrancy_status(&self, status: u32) -> Result<(), Error> {
        self.env
            .storage()
            .instance()
            .set(&REENTRANCY_STATUS_KEY, &status);
        Ok(())
    }

    fn generate_call_id(env: &Env) -> Symbol {
        let timestamp = env.ledger().timestamp();
        // Create a unique call ID based on timestamp
        let _timestamp_u32 = (timestamp % 1000000) as u32; // Take last 6 digits to ensure uniqueness
        // Use a simple hardcoded symbol for Soroban compatibility
        soroban_sdk::symbol_short!("call_id")
    }

    fn get_call_count(&self) -> u32 {
        self.env
            .storage()
            .instance()
            .get(&CALL_COUNT_KEY)
            .unwrap_or(0)
    }

    fn increment_call_count(&self) -> Result<(), Error> {
        let current_count = self.get_call_count();
        self.env
            .storage()
            .instance()
            .set(&CALL_COUNT_KEY, &(current_count + 1));
        Ok(())
    }

    fn decrement_call_count(&self) -> Result<(), Error> {
        let current_count = self.get_call_count();
        if current_count > 0 {
            self.env
                .storage()
                .instance()
                .set(&CALL_COUNT_KEY, &(current_count - 1));
        }
        Ok(())
    }

    fn reset_call_state(&self) -> Result<(), Error> {
        self.env
            .storage()
            .instance()
            .set(&CALL_COUNT_KEY, &0u32);
        self.env
            .storage()
            .instance()
            .set(&REENTRANCY_STATUS_KEY, &REENTRANCY_NOT_ENTERED);
        Ok(())
    }
}

// Convenience macro for protecting functions
#[macro_export]
macro_rules! with_reentrancy_guard {
    ($env:expr, $function_name:expr, $caller:expr, $body:expr) => {{
        let mut guard = $crate::reentrancy::ReentrancyGuard::new($env, &$function_name, &$caller)?;
        guard.before_external_call(&$function_name, &$caller)?;
        
        let result = $body;
        
        match result {
            Ok(value) => {
                guard.after_external_call(true)?;
                Ok(value)
            }
            Err(error) => {
                guard.after_external_call(false)?;
                Err(error)
            }
        }
    }};
}

// Helper functions for common reentrancy protection patterns
pub fn protect_external_call<T, F>(
    env: &Env,
    function_name: Symbol,
    caller: Address,
    operation: F,
) -> Result<T, Error>
where
    F: FnOnce() -> Result<T, Error>,
{
    let mut guard = ReentrancyGuard::new(env, &function_name, &caller)?;
    guard.before_external_call(&function_name, &caller)?;
    
    let result = operation();
    
    let success = result.is_ok();
    guard.after_external_call(success)?;
    
    result
}

pub fn validate_no_reentrancy(env: &Env) -> Result<(), Error> {
    let status = env
        .storage()
        .instance()
        .get(&REENTRANCY_STATUS_KEY)
        .unwrap_or(REENTRANCY_NOT_ENTERED);
    
    if status == REENTRANCY_ENTERED {
        return Err(Error::ReentrancyAttack);
    }
    
    Ok(())
} 