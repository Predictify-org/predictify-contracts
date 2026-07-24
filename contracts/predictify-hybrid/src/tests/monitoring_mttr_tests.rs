#![cfg(test)]

use crate::monitoring::{ContractMonitor, RollingWindow};
use soroban_sdk::{testutils::Address as _, Address, Env};
use crate::admin::AdminInitializer;

fn setup() -> (Env, Address) {
    let env = Env::default();
    let admin = Address::generate(&env);
    AdminInitializer::initialize(&env, &admin).unwrap();
    (env, admin)
}

#[test]
fn test_rolling_window_brute_force() {
    let env = Env::default();
    let mut window = RollingWindow::new(&env, 5);
    
    // push 1, 2, 3
    window.push(&env, 10);
    window.push(&env, 20);
    window.push(&env, 30);
    assert_eq!(window.average(), 20);
    
    // push more to trigger replace
    window.push(&env, 40);
    window.push(&env, 50);
    window.push(&env, 60); // replaces 10
    
    // entries: [60, 20, 30, 40, 50]
    // sum: 200, average: 40
    assert_eq!(window.average(), 40);
}

#[test]
fn test_mttr_mtbf_queries() {
    let (env, admin) = setup();
    
    ContractMonitor::set_window_capacity(&env, &admin, 10).unwrap();
    assert_eq!(ContractMonitor::get_window_capacity(&env), 10);
    
    // Record incident 1
    ContractMonitor::record_incident(&env, 1000).unwrap();
    // Record recovery 1
    ContractMonitor::record_recovery(&env, 50).unwrap();
    
    // Record incident 2
    ContractMonitor::record_incident(&env, 2000).unwrap(); // MTBF = 1000
    // Record recovery 2
    ContractMonitor::record_recovery(&env, 150).unwrap();
    
    let mttr = ContractMonitor::get_mttr(&env);
    let mtbf = ContractMonitor::get_mtbf(&env);
    
    assert_eq!(mttr, 100); // (50 + 150) / 2
    assert_eq!(mtbf, 1000); // 1000
}
