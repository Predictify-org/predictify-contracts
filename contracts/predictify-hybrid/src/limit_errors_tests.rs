//! Tests for the semantic limit error taxonomy (issue #987).
//!
//! Every bound violation in the contract used to surface as a generic
//! [`Error::InvalidInput`] — or, for the monitor queue, as a bare `panic!`. Clients
//! could not tell "your bet is too large" from "your outcome string is malformed",
//! and had no way to decide whether to retry with a smaller value or alert an
//! operator to fix a bad configuration.
//!
//! This suite pins the replacement behaviour:
//!
//! 1. **Call-site mapping** — each limit check returns its own semantic variant.
//! 2. **Precedence** — overlapping bounds resolve in a documented, stable order.
//! 3. **Boundaries** — the inclusive edge of every range is still accepted.
//! 4. **Taxonomy** — every new variant has a unique code, a real description, and a
//!    classification that is never `Unknown`.
//! 5. **Regression** — no limit path silently reverts to `InvalidInput`.
//!
//! The monitor-queue variants ([`Error::QueueCapacityOutOfRange`],
//! [`Error::QueueAlreadyInitialized`]) are exercised by the unit tests inside
//! `monitor.rs` rather than here, because they need the queue's storage fixture.

#![cfg(test)]

use alloc::vec;
use alloc::vec::Vec as StdVec;

use soroban_sdk::{BytesN, Env, String, Symbol};

use crate::bets::{
    get_market_max_bet_cap, set_event_bet_limits, set_global_bet_limits, set_market_max_bet_cap,
    BetManager, BetValidator, MAX_BATCH_SIZE, MAX_BET_AMOUNT, MIN_BET_AMOUNT,
};
use crate::err::{ErrorCategory, ErrorHandler};
use crate::fees::{FeeConfig, FeeValidator, MAX_FEE_AMOUNT, MAX_FEE_PERCENTAGE, MIN_FEE_AMOUNT};
use crate::types::BetLimits;
use crate::Error;

// ─── helpers ────────────────────────────────────────────────────────────────

/// Register the contract so storage-backed helpers can run inside a contract frame.
fn setup() -> (Env, soroban_sdk::Address) {
    let env = Env::default();
    let contract_id = env.register(crate::PredictifyHybrid, ());
    (env, contract_id)
}

/// A fee config that passes every check, used as the base for one-field mutations.
fn valid_fee_config() -> FeeConfig {
    FeeConfig {
        platform_fee_percentage: 200,
        creation_fee: 10_000_000,
        min_fee_amount: MIN_FEE_AMOUNT,
        max_fee_amount: MAX_FEE_AMOUNT,
        collection_threshold: 100_000_000,
        fees_enabled: true,
    }
}

/// Every variant introduced by the limit taxonomy.
fn limit_errors() -> StdVec<Error> {
    vec![
        Error::BetAboveMaximum,
        Error::BetLimitsInverted,
        Error::BetLimitAboveMaximum,
        Error::BetCapOutOfRange,
        Error::BatchEmpty,
        Error::BatchSizeExceeded,
        Error::FeePercentageOutOfRange,
        Error::FeeAmountAboveMaximum,
        Error::CreationFeeOutOfRange,
        Error::FeeLimitsInverted,
        Error::QueueCapacityOutOfRange,
        Error::QueueAlreadyInitialized,
    ]
}

// ─── 1. bet amount limits ───────────────────────────────────────────────────

#[test]
fn bet_above_absolute_maximum_is_semantic() {
    assert_eq!(
        BetValidator::validate_bet_amount(MAX_BET_AMOUNT + 1),
        Err(Error::BetAboveMaximum)
    );
    assert_eq!(
        BetValidator::validate_bet_amount(i128::MAX),
        Err(Error::BetAboveMaximum)
    );
}

#[test]
fn bet_below_minimum_still_reports_insufficient_stake() {
    // Unchanged on purpose: `InsufficientStake` is already semantic for the lower
    // bound, so this issue only replaced the *upper*-bound generic error.
    assert_eq!(
        BetValidator::validate_bet_amount(MIN_BET_AMOUNT - 1),
        Err(Error::InsufficientStake)
    );
    assert_eq!(
        BetValidator::validate_bet_amount(0),
        Err(Error::InsufficientStake)
    );
    assert_eq!(
        BetValidator::validate_bet_amount(i128::MIN),
        Err(Error::InsufficientStake)
    );
}

#[test]
fn bet_amount_boundaries_are_inclusive() {
    assert!(BetValidator::validate_bet_amount(MIN_BET_AMOUNT).is_ok());
    assert!(BetValidator::validate_bet_amount(MAX_BET_AMOUNT).is_ok());
}

#[test]
fn bet_above_effective_market_maximum_is_semantic() {
    let (env, contract_id) = setup();
    let market_id = Symbol::new(&env, "mkt_limits");

    env.as_contract(&contract_id, || {
        set_event_bet_limits(
            &env,
            &market_id,
            &BetLimits {
                min_bet: MIN_BET_AMOUNT,
                max_bet: 5_000_000,
            },
        )
        .unwrap();

        assert_eq!(
            BetValidator::validate_bet_amount_against_limits(&env, &market_id, 5_000_001),
            Err(Error::BetAboveMaximum)
        );
        assert!(
            BetValidator::validate_bet_amount_against_limits(&env, &market_id, 5_000_000).is_ok()
        );
    });
}

#[test]
fn market_maximum_takes_precedence_over_per_market_cap() {
    // Both bounds are violated. The effective max is checked first, so the caller
    // learns the market's own ceiling rather than the admin-imposed cap.
    let (env, contract_id) = setup();
    let market_id = Symbol::new(&env, "mkt_prec");

    env.as_contract(&contract_id, || {
        set_event_bet_limits(
            &env,
            &market_id,
            &BetLimits {
                min_bet: MIN_BET_AMOUNT,
                max_bet: 5_000_000,
            },
        )
        .unwrap();
        set_market_max_bet_cap(&env, &market_id, 3_000_000).unwrap();

        assert_eq!(
            BetValidator::validate_bet_amount_against_limits(&env, &market_id, 9_000_000),
            Err(Error::BetAboveMaximum)
        );
        // Within the market max but over the cap → the cap's own error.
        assert_eq!(
            BetValidator::validate_bet_amount_against_limits(&env, &market_id, 4_000_000),
            Err(Error::BetExceedsCap)
        );
    });
}

// ─── 2. bet limit configuration ─────────────────────────────────────────────

#[test]
fn inverted_bet_limits_are_semantic() {
    let (env, contract_id) = setup();
    env.as_contract(&contract_id, || {
        let inverted = BetLimits {
            min_bet: 10_000_000,
            max_bet: 5_000_000,
        };
        assert_eq!(
            set_global_bet_limits(&env, &inverted),
            Err(Error::BetLimitsInverted)
        );
        let market_id = Symbol::new(&env, "mkt_inv");
        assert_eq!(
            set_event_bet_limits(&env, &market_id, &inverted),
            Err(Error::BetLimitsInverted)
        );
    });
}

#[test]
fn bet_limit_above_absolute_ceiling_is_semantic() {
    let (env, contract_id) = setup();
    env.as_contract(&contract_id, || {
        assert_eq!(
            set_global_bet_limits(
                &env,
                &BetLimits {
                    min_bet: MIN_BET_AMOUNT,
                    max_bet: MAX_BET_AMOUNT + 1,
                }
            ),
            Err(Error::BetLimitAboveMaximum)
        );
    });
}

#[test]
fn bet_limit_below_absolute_floor_still_reports_insufficient_stake() {
    let (env, contract_id) = setup();
    env.as_contract(&contract_id, || {
        assert_eq!(
            set_global_bet_limits(
                &env,
                &BetLimits {
                    min_bet: MIN_BET_AMOUNT - 1,
                    max_bet: MAX_BET_AMOUNT,
                }
            ),
            Err(Error::InsufficientStake)
        );
    });
}

#[test]
fn valid_bet_limits_are_accepted_at_the_boundary() {
    let (env, contract_id) = setup();
    env.as_contract(&contract_id, || {
        assert!(set_global_bet_limits(
            &env,
            &BetLimits {
                min_bet: MIN_BET_AMOUNT,
                max_bet: MAX_BET_AMOUNT,
            }
        )
        .is_ok());
    });
}

// ─── 3. per-market bet cap ──────────────────────────────────────────────────

#[test]
fn bet_cap_out_of_range_is_semantic() {
    let (env, contract_id) = setup();
    let market_id = Symbol::new(&env, "mkt_cap");

    env.as_contract(&contract_id, || {
        for bad in [0i128, -1, i128::MIN, MAX_BET_AMOUNT + 1, i128::MAX] {
            assert_eq!(
                set_market_max_bet_cap(&env, &market_id, bad),
                Err(Error::BetCapOutOfRange),
                "cap {} must be rejected as out of range",
                bad
            );
        }
        // A rejected cap must not have been written.
        assert_eq!(get_market_max_bet_cap(&env, &market_id), None);
    });
}

#[test]
fn bet_cap_boundaries_are_accepted() {
    let (env, contract_id) = setup();
    let market_id = Symbol::new(&env, "mkt_capok");

    env.as_contract(&contract_id, || {
        assert!(set_market_max_bet_cap(&env, &market_id, 1).is_ok());
        assert_eq!(get_market_max_bet_cap(&env, &market_id), Some(1));

        assert!(set_market_max_bet_cap(&env, &market_id, MAX_BET_AMOUNT).is_ok());
        assert_eq!(
            get_market_max_bet_cap(&env, &market_id),
            Some(MAX_BET_AMOUNT)
        );
    });
}

// ─── 4. fee limits ──────────────────────────────────────────────────────────

#[test]
fn fee_amount_above_maximum_is_semantic() {
    assert_eq!(
        FeeValidator::validate_fee_amount(MAX_FEE_AMOUNT + 1),
        Err(Error::FeeAmountAboveMaximum)
    );
    assert_eq!(
        FeeValidator::validate_fee_amount(MIN_FEE_AMOUNT - 1),
        Err(Error::InsufficientStake)
    );
    assert!(FeeValidator::validate_fee_amount(MIN_FEE_AMOUNT).is_ok());
    assert!(FeeValidator::validate_fee_amount(MAX_FEE_AMOUNT).is_ok());
}

#[test]
fn creation_fee_out_of_range_is_semantic() {
    assert_eq!(
        FeeValidator::validate_creation_fee(MIN_FEE_AMOUNT - 1),
        Err(Error::CreationFeeOutOfRange)
    );
    assert_eq!(
        FeeValidator::validate_creation_fee(MAX_FEE_AMOUNT + 1),
        Err(Error::CreationFeeOutOfRange)
    );
    assert!(FeeValidator::validate_creation_fee(MIN_FEE_AMOUNT).is_ok());
    assert!(FeeValidator::validate_creation_fee(MAX_FEE_AMOUNT).is_ok());
}

#[test]
fn fee_percentage_out_of_range_is_semantic() {
    let mut config = valid_fee_config();
    config.platform_fee_percentage = MAX_FEE_PERCENTAGE + 1;
    assert_eq!(
        FeeValidator::validate_fee_config(&config),
        Err(Error::FeePercentageOutOfRange)
    );

    config.platform_fee_percentage = -1;
    assert_eq!(
        FeeValidator::validate_fee_config(&config),
        Err(Error::FeePercentageOutOfRange)
    );

    config.platform_fee_percentage = MAX_FEE_PERCENTAGE;
    assert!(FeeValidator::validate_fee_config(&config).is_ok());
}

#[test]
fn inverted_fee_limits_are_semantic() {
    let mut config = valid_fee_config();
    config.min_fee_amount = 10_000_000;
    config.max_fee_amount = 5_000_000;
    assert_eq!(
        FeeValidator::validate_fee_config(&config),
        Err(Error::FeeLimitsInverted)
    );
}

#[test]
fn negative_fee_fields_still_report_invalid_input() {
    // Sign checks are not bound violations, so they intentionally keep the generic
    // code. This guards against over-eager future replacement.
    let mut config = valid_fee_config();
    config.creation_fee = -1;
    assert_eq!(
        FeeValidator::validate_fee_config(&config),
        Err(Error::InvalidInput)
    );

    let mut config = valid_fee_config();
    config.min_fee_amount = -1;
    assert_eq!(
        FeeValidator::validate_fee_config(&config),
        Err(Error::InvalidInput)
    );
}

#[test]
fn valid_fee_config_is_accepted() {
    assert!(FeeValidator::validate_fee_config(&valid_fee_config()).is_ok());
}

// ─── 5. batch size limits ───────────────────────────────────────────────────

/// Build `n` syntactically valid batch entries. The batch-size guard runs before
/// any market lookup, so the markets these name need not exist.
fn dummy_batch(env: &Env, n: u32) -> soroban_sdk::Vec<(Symbol, String, i128)> {
    let mut bets = soroban_sdk::Vec::new(env);
    for _ in 0..n {
        bets.push_back((
            Symbol::new(env, "mkt_batch"),
            String::from_str(env, "yes"),
            MIN_BET_AMOUNT,
        ));
    }
    bets
}

#[test]
fn empty_batch_is_semantic() {
    let (env, contract_id) = setup();
    env.mock_all_auths();
    let user = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);

    env.as_contract(&contract_id, || {
        let result = BetManager::place_bets(
            &env,
            user.clone(),
            soroban_sdk::Vec::new(&env),
            0,
            BytesN::from_array(&env, &[1u8; 32]),
        );
        assert_eq!(result.err(), Some(Error::BatchEmpty));
    });
}

#[test]
fn oversized_batch_is_semantic() {
    let (env, contract_id) = setup();
    env.mock_all_auths();
    let user = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);

    env.as_contract(&contract_id, || {
        let result = BetManager::place_bets(
            &env,
            user.clone(),
            dummy_batch(&env, MAX_BATCH_SIZE + 1),
            0,
            BytesN::from_array(&env, &[2u8; 32]),
        );
        assert_eq!(result.err(), Some(Error::BatchSizeExceeded));
    });
}

#[test]
fn batch_bounds_are_distinct_errors() {
    assert_eq!(MAX_BATCH_SIZE, 50);
    assert_ne!(Error::BatchEmpty, Error::BatchSizeExceeded);
}

// ─── 6. taxonomy invariants ─────────────────────────────────────────────────

#[test]
fn limit_error_codes_are_unique() {
    let errors = limit_errors();
    for (i, a) in errors.iter().enumerate() {
        for b in errors.iter().skip(i + 1) {
            assert_ne!(
                *a as u32, *b as u32,
                "{:?} and {:?} share a discriminant",
                a, b
            );
            assert_ne!(a.code(), b.code(), "{:?} and {:?} share a string code", a, b);
        }
    }
}

#[test]
fn limit_errors_have_explicit_descriptions_and_codes() {
    for err in limit_errors() {
        assert_ne!(
            err.code(),
            "UNSPECIFIED_ERROR",
            "{:?} fell through to the fallback string code",
            err
        );
        assert_ne!(
            err.description(),
            "An unspecified error occurred.",
            "{:?} fell through to the fallback description",
            err
        );
        assert!(!err.code().contains(' '));
    }
}

#[test]
fn limit_errors_occupy_their_reserved_range() {
    for err in limit_errors() {
        let code = err as u32;
        assert!(
            (600..=611).contains(&code),
            "{:?} = {} is outside the reserved limit range 600-611",
            err,
            code
        );
    }
}

#[test]
fn limit_errors_are_classified() {
    for err in limit_errors() {
        let (_, category, _) = ErrorHandler::get_error_classification(&err);
        assert_ne!(
            category,
            ErrorCategory::Unknown,
            "{:?} has no explicit classification",
            err
        );
    }
}

#[test]
fn limit_errors_are_distinct_from_invalid_input() {
    for err in limit_errors() {
        assert_ne!(err, Error::InvalidInput);
        assert_ne!(err as u32, Error::InvalidInput as u32);
    }
}
