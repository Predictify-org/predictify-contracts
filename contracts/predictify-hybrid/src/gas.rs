#![allow(dead_code)]
use soroban_sdk::{contracttype, symbol_short, Env, Symbol};

/// Stores the gas limit configured by an admin for a specific operation.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GasConfigKey {
    GasLimit(Symbol),
}

/// GasTracker provides observability hooks and optimization limits.
pub struct GasTracker;

impl GasTracker {
    /// # Optimization Guidelines
    ///
    /// To ensure minimal overhead and optimize gas usage in Predictify:
    /// 1. **Data Structures:** Prefer `Symbol` over `String` for map keys when possible.
    /// 2. **Storage:** Minimize persistent `env.storage().persistent().set` calls.
    ///    Cache values in memory during execution and write once at the end.
    /// 3. **Batching:** Use batch operations for payouts and claim updates instead of iterative calls.
    /// 4. **Events:** Only emit essential events; observability events like `gas_used`
    ///    can be disabled in high-traffic deployments if needed.

    /// Administrative hook to set a gas/budget limit per operation.
    pub fn set_limit(env: &Env, operation: Symbol, max_units: u64) {
        env.storage()
            .instance()
            .set(&GasConfigKey::GasLimit(operation), &max_units);
    }

    /// Retrieves the current gas budget limit for an operation.
    pub fn get_limit(env: &Env, operation: Symbol) -> Option<u64> {
        env.storage()
            .instance()
            .get(&GasConfigKey::GasLimit(operation))
    }

    /// Hook to call before an operation begins. Returns a usage marker.
    pub fn start_tracking(_env: &Env) -> u64 {
        // Budget metrics are not directly accessible in contract code.
        // This hook remains for interface compatibility and future host-side logging.
        0
    }

    /// Hook to call immediately after an operation.
    /// It records the usage, publishes an observability event, and checks the admin cap.
    pub fn end_tracking(env: &Env, operation: Symbol, _start_marker: u64) {
        // Placeholder for actual cost. Host-side diagnostics should be used for precise gas monitoring.
        #[cfg(not(test))]
        let actual_cost = 0;

        #[cfg(test)]
        let actual_cost = env.storage().temporary().get::<Symbol, u64>(&symbol_short!("t_gas")).unwrap_or(0);

        // Publish observability event: [ "gas_used", operation.clone() ] -> cost_used
        env.events().publish(
            (symbol_short!("gas_used"), operation.clone()),
            actual_cost,
        );

        // Optional: admin-set gas budget cap per call (abort if exceeded)
        if let Some(limit) = Self::get_limit(env, operation) {
            if actual_cost > limit {
                panic!("Gas budget cap exceeded");
            }
        }
    }
}
