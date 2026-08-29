//! # Gas Regression Limit Tests
//!
//! Tests that verify the gas regression limits for `create_market` and
//! `claim_winnings` paths are correctly enforced.

#![cfg(test)]

use super::*;
use soroban_sdk::{symbol_short, Env};

// ===== DEFAULT LIMIT CONSTANTS =====

#[test]
fn test_default_create_market_gas_limit_value() {
    assert!(
        super::gas::DEFAULT_CREATE_MARKET_GAS_LIMIT > 0,
        "create_market gas limit must be positive"
    );
    assert!(
        super::gas::DEFAULT_CREATE_MARKET_GAS_LIMIT <= 10_000_000,
        "create_market gas limit should not exceed 10M"
    );
}

#[test]
fn test_default_claim_winnings_gas_limit_value() {
    assert!(
        super::gas::DEFAULT_CLAIM_WINNINGS_GAS_LIMIT > 0,
        "claim_winnings gas limit must be positive"
    );
    assert!(
        super::gas::DEFAULT_CLAIM_WINNINGS_GAS_LIMIT <= 10_000_000,
        "claim_winnings gas limit should not exceed 10M"
    );
}

// ===== DEFAULT LIMIT SEEDING =====

#[test]
fn test_set_default_limits_populates_storage() {
    let env = Env::default();

    env.as_contract(&Address::generate(&env), || {
        GasTracker::set_default_limits(&env);

        let (create_cpu, _create_mem) =
            GasTracker::get_limits(&env, symbol_short!("create"));
        let (claim_cpu, _claim_mem) =
            GasTracker::get_limits(&env, symbol_short!("claim"));

        assert_eq!(
            create_cpu,
            Some(super::gas::DEFAULT_CREATE_MARKET_GAS_LIMIT),
            "create_market default limit should be seeded"
        );
        assert_eq!(
            claim_cpu,
            Some(super::gas::DEFAULT_CLAIM_WINNINGS_GAS_LIMIT),
            "claim_winnings default limit should be seeded"
        );
    });
}

// ===== END_TRACKING ENFORCEMENT =====

#[test]
fn test_end_tracking_within_default_limit_succeeds() {
    let env = Env::default();

    env.as_contract(&Address::generate(&env), || {
        GasTracker::set_default_limits(&env);
    });

    GasTracker::set_test_cost(&env, 1);

    let contract_addr = Address::generate(&env);
    env.as_contract(&contract_addr, || {
        GasTracker::end_tracking(&env, symbol_short!("create"), 0);
    });
}

#[test]
#[should_panic(expected = "Error(ContractError(417))")]
fn test_end_tracking_exceeds_default_limit_panics() {
    let env = Env::default();

    env.as_contract(&Address::generate(&env), || {
        GasTracker::set_default_limits(&env);
    });

    GasTracker::set_test_cost(&env, super::gas::DEFAULT_CREATE_MARKET_GAS_LIMIT + 1);

    let contract_addr = Address::generate(&env);
    env.as_contract(&contract_addr, || {
        GasTracker::end_tracking(&env, symbol_short!("create"), 0);
    });
}

#[test]
#[should_panic(expected = "Error(ContractError(417))")]
fn test_end_tracking_claim_exceeds_default_limit_panics() {
    let env = Env::default();

    env.as_contract(&Address::generate(&env), || {
        GasTracker::set_default_limits(&env);
    });

    GasTracker::set_test_cost(&env, super::gas::DEFAULT_CLAIM_WINNINGS_GAS_LIMIT + 1);

    let contract_addr = Address::generate(&env);
    env.as_contract(&contract_addr, || {
        GasTracker::end_tracking(&env, symbol_short!("claim"), 0);
    });
}

#[test]
fn test_end_tracking_at_exact_limit_succeeds() {
    let env = Env::default();

    env.as_contract(&Address::generate(&env), || {
        GasTracker::set_default_limits(&env);
    });

    GasTracker::set_test_cost(&env, super::gas::DEFAULT_CREATE_MARKET_GAS_LIMIT);

    let contract_addr = Address::generate(&env);
    env.as_contract(&contract_addr, || {
        GasTracker::end_tracking(&env, symbol_short!("create"), 0);
    });
}

// ===== ADMIN OVERRIDE PRECEDENCE =====

#[test]
fn test_admin_override_takes_precedence() {
    let env = Env::default();

    env.as_contract(&Address::generate(&env), || {
        GasTracker::set_default_limits(&env);

        GasTracker::set_limit(
            &env,
            symbol_short!("create"),
            super::gas::DEFAULT_CREATE_MARKET_GAS_LIMIT * 2,
            0,
        );

        let effective = GasTracker::get_effective_cpu_limit(&env, symbol_short!("create"));
        assert_eq!(
            effective,
            Some(super::gas::DEFAULT_CREATE_MARKET_GAS_LIMIT * 2),
            "admin override should take precedence"
        );
    });
}

#[test]
fn test_admin_override_can_tighten_limit() {
    let env = Env::default();

    env.as_contract(&Address::generate(&env), || {
        GasTracker::set_default_limits(&env);

        let tighter = super::gas::DEFAULT_CREATE_MARKET_GAS_LIMIT / 2;
        GasTracker::set_limit(&env, symbol_short!("create"), tighter, 0);

        let effective = GasTracker::get_effective_cpu_limit(&env, symbol_short!("create"));
        assert_eq!(
            effective,
            Some(tighter),
            "admin can tighten the default limit"
        );
    });
}

#[test]
#[should_panic(expected = "Error(ContractError(417))")]
fn test_tighter_admin_limit_enforced() {
    let env = Env::default();

    env.as_contract(&Address::generate(&env), || {
        GasTracker::set_default_limits(&env);

        GasTracker::set_limit(&env, symbol_short!("create"), 100, 0);
    });

    GasTracker::set_test_cost(&env, 101);

    let contract_addr = Address::generate(&env);
    env.as_contract(&contract_addr, || {
        GasTracker::end_tracking(&env, symbol_short!("create"), 0);
    });
}

// ===== HAS_LIMIT / GET_EFFECTIVE_LIMIT =====

#[test]
fn test_has_limit_for_default_operations() {
    let env = Env::default();

    env.as_contract(&Address::generate(&env), || {
        assert!(
            GasTracker::has_limit(&env, symbol_short!("create")),
            "create should have a default limit"
        );
        assert!(
            GasTracker::has_limit(&env, symbol_short!("claim")),
            "claim should have a default limit"
        );
    });
}

#[test]
fn test_has_limit_false_for_unknown_operation() {
    let env = Env::default();

    env.as_contract(&Address::generate(&env), || {
        assert!(
            !GasTracker::has_limit(&env, symbol_short!("vote")),
            "vote should not have a default limit"
        );
    });
}

#[test]
fn test_effective_limit_none_for_unknown() {
    let env = Env::default();

    env.as_contract(&Address::generate(&env), || {
        let limit = GasTracker::get_effective_cpu_limit(&env, symbol_short!("vote"));
        assert_eq!(limit, None, "unknown op should have no effective limit");
    });
}

#[test]
fn test_effective_limit_returns_default() {
    let env = Env::default();

    env.as_contract(&Address::generate(&env), || {
        let create_limit =
            GasTracker::get_effective_cpu_limit(&env, symbol_short!("create"));
        assert_eq!(
            create_limit,
            Some(super::gas::DEFAULT_CREATE_MARKET_GAS_LIMIT)
        );

        let claim_limit =
            GasTracker::get_effective_cpu_limit(&env, symbol_short!("claim"));
        assert_eq!(
            claim_limit,
            Some(super::gas::DEFAULT_CLAIM_WINNINGS_GAS_LIMIT)
        );
    });
}

// ===== RECORD_WITH_ALERT WITH DEFAULT LIMITS =====

#[test]
fn test_record_with_alert_uses_default_limit() {
    let env = Env::default();

    env.as_contract(&Address::generate(&env), || {
        GasTracker::set_default_limits(&env);

        let threshold_91 =
            (super::gas::DEFAULT_CREATE_MARKET_GAS_LIMIT * 91) / 100;
        GasTracker::record_with_alert(&env, symbol_short!("create"), threshold_91);

        let events = env.events().all();
        assert!(
            !events.is_empty(),
            "low-water alert should fire at 91% of default limit"
        );
    });
}

#[test]
fn test_record_with_alert_no_alert_below_threshold() {
    let env = Env::default();

    env.as_contract(&Address::generate(&env), || {
        GasTracker::set_default_limits(&env);

        let threshold_89 =
            (super::gas::DEFAULT_CREATE_MARKET_GAS_LIMIT * 89) / 100;
        GasTracker::record_with_alert(&env, symbol_short!("create"), threshold_89);

        let events = env.events().all();
        assert!(
            events.is_empty(),
            "no alert should fire below 90% of default limit"
        );
    });
}

#[test]
fn test_record_with_alert_zero_usage_no_alert() {
    let env = Env::default();

    env.as_contract(&Address::generate(&env), || {
        GasTracker::set_default_limits(&env);

        GasTracker::record_with_alert(&env, symbol_short!("create"), 0);

        let events = env.events().all();
        assert!(events.is_empty(), "no alert for zero usage");
    });
}

// ===== BOUNDARY / EDGE CASES =====

#[test]
fn test_zero_cost_always_succeeds() {
    let env = Env::default();

    env.as_contract(&Address::generate(&env), || {
        GasTracker::set_default_limits(&env);
    });

    GasTracker::set_test_cost(&env, 0);

    let contract_addr = Address::generate(&env);
    env.as_contract(&contract_addr, || {
        GasTracker::end_tracking(&env, symbol_short!("create"), 0);
        GasTracker::end_tracking(&env, symbol_short!("claim"), 0);
    });
}

#[test]
fn test_untracked_operation_no_panic() {
    let env = Env::default();

    GasTracker::set_test_cost(&env, u64::MAX);

    let contract_addr = Address::generate(&env);
    env.as_contract(&contract_addr, || {
        GasTracker::end_tracking(&env, symbol_short!("vote"), 0);
    });
}

#[test]
fn test_sequential_end_tracking_calls() {
    let env = Env::default();

    env.as_contract(&Address::generate(&env), || {
        GasTracker::set_default_limits(&env);
    });

    GasTracker::set_test_cost(&env, 100);

    let contract_addr = Address::generate(&env);
    env.as_contract(&contract_addr, || {
        for _ in 0..5 {
            GasTracker::end_tracking(&env, symbol_short!("create"), 0);
        }
    });
}

#[test]
fn test_limit_ordering() {
    assert!(
        super::gas::DEFAULT_CREATE_MARKET_GAS_LIMIT
            >= super::gas::DEFAULT_CLAIM_WINNINGS_GAS_LIMIT,
        "create_market limit should be >= claim_winnings limit"
    );
}
