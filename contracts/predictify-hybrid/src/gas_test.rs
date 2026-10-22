#[cfg(test)]
mod budget_guard_tests {
    use super::*;
    use soroban_sdk::Env;
    use crate::err::Error;

    #[test]
    fn test_budget_guard_aborts_when_budget_exceeds_threshold() {
        let env = Env::default();
        let guard = BudgetGuard::new(&env, u64::MAX);
        let result = guard.check();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), Error::OperationWouldExceedBudget);
    }

    #[test]
    fn test_budget_guard_passes_with_low_threshold() {
        let env = Env::default();
        let guard = BudgetGuard::new(&env, 0);
        assert!(guard.check().is_ok());
    }

    #[test]
    fn test_budget_guard_tracks_consumed_instructions() {
        let env = Env::default();
        let guard = BudgetGuard::new(&env, 0);
        let consumed = guard.consumed();
        assert!(consumed < 1000);
    }
}
