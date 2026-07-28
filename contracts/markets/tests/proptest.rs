#![cfg(test)]

use proptest::prelude::*;
use soroban_sdk::{contract, contractimpl, testutils::{Address as _, Ledger}, Address, Env, Symbol};
use markets::admin::AdminManager;
use markets::errors::ContractError;

#[contract]
pub struct DummyContract;

#[contractimpl]
impl DummyContract {
    pub fn set_cd(env: Env, admin: Address, secs: u64) -> Result<(), ContractError> {
        AdminManager::set_admin_cooldown(&env, &admin, secs)
    }
    
    pub fn get_cd(env: Env) -> u64 {
        AdminManager::get_admin_cooldown(&env)
    }
    
    pub fn check_cd(env: Env, admin: Address, func: Symbol) -> Result<(), ContractError> {
        AdminManager::check_admin_cooldown(&env, &admin, &func)
    }
}

proptest! {
    #[test]
    fn test_admin_cooldown_invariants(
        cooldown in 1u64..1_000_000_000,
        initial_timestamp in 100u64..1_000_000_000_000,
        time_advance in 0u64..2_000_000_000
    ) {
        let env = Env::default();
        env.mock_all_auths();
        
        let contract_id = env.register(DummyContract, ());
        let client = DummyContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let func_name = Symbol::new(&env, "action");
        
        // Initial cooldown should be 0
        prop_assert_eq!(client.get_cd(), 0);
        
        // Set cooldown
        client.set_cd(&admin, &cooldown);
        prop_assert_eq!(client.get_cd(), cooldown);
        
        // Set initial timestamp
        env.ledger().with_mut(|l| {
            l.timestamp = initial_timestamp;
        });
        
        // First action should succeed
        let res = client.try_check_cd(&admin, &func_name);
        prop_assert!(res.is_ok());
        
        // Advance time
        env.ledger().with_mut(|l| {
            l.timestamp = initial_timestamp.saturating_add(time_advance);
        });
        
        // Second action
        let res2 = client.try_check_cd(&admin, &func_name);
        
        if time_advance < cooldown {
            // Should fail due to cooldown
            prop_assert_eq!(res2.unwrap_err().unwrap(), ContractError::AdminActionTimelocked);
        } else {
            // Should succeed
            prop_assert!(res2.is_ok());
        }
    }
}
