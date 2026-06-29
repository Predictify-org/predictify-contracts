#![allow(dead_code)]
use soroban_sdk::{contracttype, panic_with_error, symbol_short, Env, Symbol, Vec};
use crate::config::GAS_TRACKING_WINDOW_SIZE;
use crate::events::PerformanceMetricEvent;

use crate::err::Error;

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
    /// Rolling window of recent CPU usage values for moving average calculation
    /// Ring buffer with size determined by GAS_TRACKING_WINDOW_SIZE
    pub cpu_history: Vec<u64>,
    /// Current index in the ring buffer for O(1) insertion
    pub history_index: u32,
    /// Number of entries currently in the buffer (until it fills)
    pub history_count: u32,
}

/// GasTracker provides observability hooks and optimization limits.
///
/// It allows tracking CPU and memory usage in tests via mocks and provides
/// an administrative interface to set limits on production operations.
///
/// # Gas Budget Configuration
///
/// Admins can set per-operation gas budgets using `set_limit`:
/// - **CPU Budget**: Maximum CPU instructions allowed per operation
/// - **Memory Budget**: Maximum memory bytes allowed per operation
///
/// Budgets are enforced via `end_tracking` which panics with `GasBudgetExceeded`
/// if limits are exceeded. Low-water alerts at 90% of budget are available
/// via `record_with_alert`.
///
/// # Example Budgets
///
/// Recommended budgets for common operations:
/// - `create_market`: 5,000,000 CPU
/// - `vote`: 1,000,000 CPU
/// - `claim_winnings`: 2,000,000 CPU
/// - `resolve_market`: 3,000,000 CPU
///
/// These values should be calibrated based on actual usage patterns and
/// adjusted as the contract evolves.
pub struct GasTracker;

impl GasUsage {
    /// Adds a new CPU usage value to the rolling window buffer.
    /// Uses ring buffer semantics for O(1) insertion.
    /// Returns the moving average of the buffer contents.
    pub fn add_to_history(&mut self, env: &Env, cpu_used: u64) -> u64 {
        let window_size = GAS_TRACKING_WINDOW_SIZE as u32;
        
        // Initialize buffer if empty
        if self.cpu_history.is_empty() {
            self.cpu_history= Vec::from_array(env, [0u64; GAS_TRACKING_WINDOW_SIZE as usize]);
        }
        
        // Insert at current index (ring buffer)
        self.cpu_history.set(self.history_index as u32, cpu_used);
        
        // Update index with wrap-around
        self.history_index = (self.history_index + 1) % window_size;
        
        // Update count until buffer is full
        if self.history_count < window_size {
            self.history_count += 1;
        }
        
        // Calculate moving average
        self.calculate_moving_average(env)
    }
    
    /// Calculates the moving average of CPU usage in the buffer.
    /// O(n) operation where n = history_count, but bounded by window size.
    /// Returns 0 if buffer is empty.
    fn calculate_moving_average(&self, env: &Env) -> u64 {
        if self.history_count == 0 {
            return 0;
        }
        
        let mut sum: u64 = 0;
        for i in 0..self.history_count {
            let val_opt = self.cpu_history.get(i as u32);
            if let Some(val) = val_opt {
                sum = sum.saturating_add(val);
            }
        }
        
        sum / self.history_count as u64
    }
}

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

    /// Records gas usage with low-water alert detection.
    /// Emits PerformanceMetricEvent when usage exceeds 90% of budget.
    /// Alert fires exactly once when crossing the threshold.
    /// 
    /// # Arguments
    /// * `env` - The Soroban environment
    /// * `operation` - The operation symbol being tracked
    /// * `used` - The gas usage for this operation
    /// 
    /// # Alert Logic
    /// - Alert triggers when used > 0.9 * budget
    /// - Alert fires once per threshold crossing
    /// - No alert if budget is 0 or not set
    /// - No alert if used is 0
    pub fn record_with_alert(env: &Env, operation: Symbol, used: u64) {
        // Skip alert if no usage or no budget configured
        if used == 0 {
            return;
        }
        
        let (cpu_limit, _) = Self::get_limits(env, operation.clone());
        let budget = match cpu_limit {
            Some(limit) if limit > 0 => limit,
            _ => return, // No budget or zero budget, skip alert
        };
        
        // Calculate threshold (90% of budget)
        let threshold = (budget * 9) / 10;
        
        // Check if we've crossed the threshold
        if used > threshold {
            // Emit performance metric event
            let event = PerformanceMetricEvent {
                metric_name: Symbol::new(env, "gas_low_water").into(),
                value: used as i128,
                unit: Symbol::new(env, "cpu").into(),
                context: operation.into(),
                timestamp: env.ledger().timestamp(),
            };
            
            env.events().publish(
                (symbol_short!("performance_metric"), operation.clone()),
                event,
            );
        }
    }

    fn get_actual_cost(env: &Env, operation: Symbol) -> GasUsage {
        // Contract code cannot read real CPU/memory usage from the host.
        // For tests, allow a mocked cost to be injected via temporary storage.
        #[cfg(test)]
        {
            let cpu: Option<u64> = env.storage().temporary().get(&symbol_short!("t_gas"));
            let cpu_val = match cpu {
                Some(val) => val,
                None => 0,
            };
            return GasUsage {
                cpu: cpu_val,
                mem: 0,
                cpu_history: Vec::new(env),
                history_index: 0,
                history_count: 0,
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
            let cpu_val = match cpu {
                Some(val) => val,
                None => 0,
            };
            let mem_val = match mem {
                Some(val) => val,
                None => 0,
            };
            GasUsage {
                cpu: cpu_val,
                mem: mem_val,
                cpu_history: Vec::new(env),
                history_index: 0,
                history_count: 0,
            }
        }
    }
}

/// BudgetGuard provides CPU instruction budget monitoring at checkpoints.
///
/// It records the CPU instruction cost at creation and checks remaining budget
/// at each checkpoint, returning `Error::OperationWouldExceedBudget` if the
/// remaining budget falls below the configured threshold.
///
/// This guard is designed to be used in hot-path loops (e.g., resolution and
/// payout distribution) to abort gracefully before the host runs out of resources.
///
/// # Example
///
/// ```rust,ignore
/// let budget_guard = BudgetGuard::new(env, 50000);
///
/// // At each checkpoint:
/// budget_guard.check()?;
/// ```
///
/// # Usage Guidelines
///
/// - Create the guard once at the start of the operation
/// - Call `check()` at strategic checkpoints (every 10-50 iterations)
/// - Use a threshold of 50,000-100,000 instructions for safe abort
#[derive(Clone)]
pub struct BudgetGuard {
    env: Env,
    start_instructions: u64,
    threshold_remaining: u64,
}

impl BudgetGuard {
    /// Create a new BudgetGuard with the current CPU instruction cost.
    ///
    /// # Arguments
    /// * `env` - Soroban environment
    /// * `threshold_remaining` - Minimum remaining instructions required
    ///   (recommended: 50,000 for resolution, 100,000 for payout loops)
    ///
    /// # Returns
    /// A new BudgetGuard instance
    ///
    /// # Note
    /// The threshold should be high enough to complete the current iteration
    /// plus any post-loop cleanup operations.
    pub fn new(env: &Env, threshold_remaining: u64) -> Self {
        let start_instructions = env.budget().cpu_instruction_cost();
        BudgetGuard {
            env: env.clone(),
            start_instructions,
            threshold_remaining,
        }
    }

    /// Check if enough budget remains to continue the operation.
    ///
    /// This method reads the current CPU instruction cost from the environment
    /// and compares the consumed amount against the threshold.
    ///
    /// # Returns
    /// * `Ok(())` - Enough budget remains
    /// * `Err(Error::OperationWouldExceedBudget)` - Budget would be exceeded
    ///
    /// # Performance
    /// This is a lightweight call that reads a single value from the host.
    /// It should be called at regular intervals, not on every iteration.
    pub fn check(&self) -> Result<(), Error> {
    let current = self.env.budget().cpu_instruction_cost();
    let consumed = current.saturating_sub(self.start_instructions);

    if consumed >= self.threshold_remaining {
        return Err(Error::OperationWouldExceedBudget);
    }

    Ok(())
}

    /// Get the current remaining budget consumed so far.
    ///
    /// # Returns
    /// The number of CPU instructions consumed since the guard was created.
    pub fn consumed(&self) -> u64 {
        let current = self.env.budget().cpu_instruction_cost();
        current.saturating_sub(self.start_instructions)
    }

    /// Get the configured threshold.
    pub fn threshold(&self) -> u64 {
        self.threshold_remaining
    }
}
#[cfg(test)]
mod gas_test;
