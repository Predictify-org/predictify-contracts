#![allow(dead_code)]
use soroban_sdk::{contracttype, panic_with_error, symbol_short, Env, Symbol};

/// Stores the gas limit configured by an admin for a specific operation.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GasConfigKey {
    /// Global or operation-specific gas limit (CPU instructions)
    GasLimit(Symbol),
    /// Operation-specific memory limit (bytes)
    MemLimit(Symbol),
    /// Mock cost for tests: (symbol_short!("t_cpu") | symbol_short!("t_mem"), operation)
    TestCost(Symbol, Symbol),
}

/// Represents consumed resources for an operation.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct GasUsage {
    pub cpu: u64,
    pub mem: u64,
}

/// GasTracker provides observability hooks and optimization limits.
///
/// It allows tracking CPU and memory usage in tests via mocks and provides
/// an administrative interface to set limits on production operations.
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
    pub fn set_limit(env: &Env, operation: Symbol, max_cpu: u64, max_mem: u64) {
        env.storage()
            .instance()
            .set(&GasConfigKey::GasLimit(operation.clone()), &max_cpu);
        env.storage()
            .instance()
            .set(&GasConfigKey::MemLimit(operation), &max_mem);
    }

    /// Retrieves the current gas budget limit for an operation.
    pub fn get_limits(env: &Env, operation: Symbol) -> (Option<u64>, Option<u64>) {
        let cpu = env
            .storage()
            .instance()
            .get(&GasConfigKey::GasLimit(operation.clone()));
        let mem = env
            .storage()
            .instance()
            .get(&GasConfigKey::MemLimit(operation));
        (cpu, mem)
    }

    /// Hook to call before an operation begins. Returns a usage marker.
    pub fn start_tracking(_env: &Env) -> u64 {
        // Budget metrics are not directly accessible in contract code via Env.
        // This hook remains for interface compatibility and future host-side logging.
        0
    }

    /// Hook to call immediately after an operation.
    /// It records usage, publishes an observability event, and checks admin caps.
    pub fn end_tracking(env: &Env, operation: Symbol, _start_marker: u64) {
        let cost = Self::get_actual_cost(env, operation.clone());

        // Publish observability event: [ "gas_used", operation ] -> cost
        env.events()
            .publish((symbol_short!("gas_used"), operation.clone()), cost.clone());

        // Optional: admin-set gas budget cap per call (abort if exceeded)
        let (cpu_limit, mem_limit) = Self::get_limits(env, operation);

        if let Some(limit) = cpu_limit {
            if cost.cpu > limit {
                panic_with_error!(env, crate::err::Error::GasBudgetExceeded);
            }
        }
        if let Some(limit) = mem_limit {
            if cost.mem > limit {
                panic_with_error!(env, crate::err::Error::GasBudgetExceeded);
            }
        }
    }

    /// Test helper to set the expected cost for an operation.
    #[cfg(test)]
    pub fn set_test_cost(env: &Env, cost: u64) {
        env.storage()
            .temporary()
            .set(&symbol_short!("t_gas"), &cost);
    }

    fn get_actual_cost(env: &Env, operation: Symbol) -> GasUsage {
        // Contract code cannot read real CPU/memory usage from the host.
        // For tests, allow a mocked cost to be injected via temporary storage.
        #[cfg(test)]
        {
            let cpu: Option<u64> = env.storage().temporary().get(&symbol_short!("t_gas"));
            return GasUsage {
                cpu: cpu.unwrap_or(0),
                mem: 0,
            };
        }

        #[cfg(not(test))]
        {
            // Optional per-operation mock hooks (useful for off-chain simulation),
            // otherwise default to zero.
            let cpu: Option<u64> = env.storage().instance().get(&GasConfigKey::TestCost(
                symbol_short!("t_cpu"),
                operation.clone(),
            ));
            let mem: Option<u64> = env
                .storage()
                .instance()
                .get(&GasConfigKey::TestCost(symbol_short!("t_mem"), operation));
            GasUsage {
                cpu: cpu.unwrap_or(0),
                mem: mem.unwrap_or(0),
            }
        }
    }
}
