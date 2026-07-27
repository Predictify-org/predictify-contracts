//! Property-based tests: platform fee configuration respects its documented
//! min/max basis-point (bps) bounds.
//!
//! `PredictifyHybrid::set_platform_fee` and the `platform_fee_pct` argument of
//! `PredictifyHybrid::initialize` both gate the platform fee percentage to the
//! closed interval `[MIN_PLATFORM_FEE_BPS, MAX_PLATFORM_FEE_BPS]` = `[0, 1000]`
//! (0%–10%), rejecting anything outside it with `Error::InvalidFeeConfig`
//! instead of clamping, truncating, or panicking.
//!
//! These are integration tests (crate-external, via `PredictifyHybridClient`),
//! so they only exercise the contract's public entrypoints — the same surface
//! a real caller has. No production code changes; this file only adds test
//! coverage.
//!
//! ## Invariants covered
//! 1. `set_platform_fee` succeeds iff the requested percentage is within
//!    `[0, 1000]` bps; outside that range it fails closed with
//!    `Error::InvalidFeeConfig`, for arbitrary `i128` inputs (including the
//!    boundaries and `i128::MIN`/`i128::MAX`, which must not panic or
//!    overflow).
//! 2. The same bound is enforced on `initialize`'s optional
//!    `platform_fee_pct` argument.
//! 3. Authorization is checked *before* the bps bounds: a non-admin caller is
//!    rejected with `Error::Unauthorized` for every fee value, valid or not.

#![cfg(test)]

use predictify_hybrid::{Error, PredictifyHybrid, PredictifyHybridClient};
use proptest::prelude::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, Env};

/// Inclusive bounds documented on `PredictifyHybrid::set_platform_fee` and
/// `PredictifyHybrid::initialize` (0%–10%, in basis points).
const MIN_PLATFORM_FEE_BPS: i128 = 0;
const MAX_PLATFORM_FEE_BPS: i128 = 1000;

/// A registered, initialized contract with a real admin and a default fee.
fn initialized_fixture() -> (Env, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let cid = env.register(PredictifyHybrid, ());
    PredictifyHybridClient::new(&env, &cid).initialize(&admin, &Some(200i128), &None);

    (env, cid, admin)
}

/// A registered but *not yet initialized* contract, for exercising
/// `initialize` itself.
fn uninitialized_contract() -> (Env, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let cid = env.register(PredictifyHybrid, ());

    (env, cid, admin)
}

proptest! {
    // Each case spins up a fresh `Env` and registers a contract, so keep the
    // case count modest to stay fast (mirrors `src/property_based_tests.rs`).
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// Property 1: `set_platform_fee` accepts a percentage exactly when it
    /// falls within `[0, 1000]` bps, and otherwise fails closed.
    #[test]
    fn set_platform_fee_respects_min_max_bps(fee in -50i128..=1050i128) {
        let (env, cid, admin) = initialized_fixture();
        let client = PredictifyHybridClient::new(&env, &cid);

        let result = client.try_set_platform_fee(&admin, &fee);

        if (MIN_PLATFORM_FEE_BPS..=MAX_PLATFORM_FEE_BPS).contains(&fee) {
            prop_assert_eq!(result, Ok(Ok(())), "in-bounds fee {} bps was rejected", fee);
        } else {
            prop_assert_eq!(
                result,
                Err(Ok(Error::InvalidFeeConfig)),
                "out-of-bounds fee {} bps was not rejected with InvalidFeeConfig",
                fee
            );
        }
    }

    /// Property 2: the same `[0, 1000]` bps window is enforced on the
    /// optional `platform_fee_pct` argument to `initialize`.
    #[test]
    fn initialize_platform_fee_respects_min_max_bps(fee in -50i128..=1050i128) {
        let (env, cid, admin) = uninitialized_contract();
        let client = PredictifyHybridClient::new(&env, &cid);

        let result = client.try_initialize(&admin, &Some(fee), &None);

        if (MIN_PLATFORM_FEE_BPS..=MAX_PLATFORM_FEE_BPS).contains(&fee) {
            prop_assert_eq!(result, Ok(Ok(())), "in-bounds fee {} bps was rejected", fee);
        } else {
            prop_assert_eq!(
                result,
                Err(Ok(Error::InvalidFeeConfig)),
                "out-of-bounds fee {} bps was not rejected with InvalidFeeConfig",
                fee
            );
        }
    }

    /// Property 3: the admin check runs before the bps bounds are even
    /// consulted, so a non-admin caller is rejected for every fee value.
    #[test]
    fn set_platform_fee_rejects_non_admin_regardless_of_bps(fee in -50i128..=1050i128) {
        let (env, cid, _admin) = initialized_fixture();
        let attacker = Address::generate(&env);
        let client = PredictifyHybridClient::new(&env, &cid);

        let result = client.try_set_platform_fee(&attacker, &fee);
        prop_assert_eq!(
            result,
            Err(Ok(Error::Unauthorized)),
            "non-admin call with fee {} bps was not rejected as Unauthorized",
            fee
        );
    }
}

// ===== FOCUSED EDGE-CASE TESTS =====
// proptest's default RNG rarely lands exactly on these boundaries (and never
// on i128::MIN/MAX), so they are pinned down as explicit unit tests.

#[test]
fn edge_set_platform_fee_zero_is_min_boundary_and_succeeds() {
    let (env, cid, admin) = initialized_fixture();
    let client = PredictifyHybridClient::new(&env, &cid);
    assert_eq!(client.try_set_platform_fee(&admin, &0i128), Ok(Ok(())));
}

#[test]
fn edge_set_platform_fee_1000_is_max_boundary_and_succeeds() {
    let (env, cid, admin) = initialized_fixture();
    let client = PredictifyHybridClient::new(&env, &cid);
    assert_eq!(client.try_set_platform_fee(&admin, &1000i128), Ok(Ok(())));
}

#[test]
fn edge_set_platform_fee_1001_just_above_max_is_rejected() {
    let (env, cid, admin) = initialized_fixture();
    let client = PredictifyHybridClient::new(&env, &cid);
    assert_eq!(
        client.try_set_platform_fee(&admin, &1001i128),
        Err(Ok(Error::InvalidFeeConfig))
    );
}

#[test]
fn edge_set_platform_fee_negative_one_is_rejected() {
    let (env, cid, admin) = initialized_fixture();
    let client = PredictifyHybridClient::new(&env, &cid);
    assert_eq!(
        client.try_set_platform_fee(&admin, &-1i128),
        Err(Ok(Error::InvalidFeeConfig))
    );
}

#[test]
fn edge_set_platform_fee_i128_min_does_not_panic() {
    let (env, cid, admin) = initialized_fixture();
    let client = PredictifyHybridClient::new(&env, &cid);
    assert_eq!(
        client.try_set_platform_fee(&admin, &i128::MIN),
        Err(Ok(Error::InvalidFeeConfig))
    );
}

#[test]
fn edge_set_platform_fee_i128_max_does_not_panic() {
    let (env, cid, admin) = initialized_fixture();
    let client = PredictifyHybridClient::new(&env, &cid);
    assert_eq!(
        client.try_set_platform_fee(&admin, &i128::MAX),
        Err(Ok(Error::InvalidFeeConfig))
    );
}

#[test]
fn edge_initialize_platform_fee_zero_is_min_boundary_and_succeeds() {
    let (env, cid, admin) = uninitialized_contract();
    let client = PredictifyHybridClient::new(&env, &cid);
    assert_eq!(
        client.try_initialize(&admin, &Some(0i128), &None),
        Ok(Ok(()))
    );
}

#[test]
fn edge_initialize_platform_fee_1000_is_max_boundary_and_succeeds() {
    let (env, cid, admin) = uninitialized_contract();
    let client = PredictifyHybridClient::new(&env, &cid);
    assert_eq!(
        client.try_initialize(&admin, &Some(1000i128), &None),
        Ok(Ok(()))
    );
}

#[test]
fn edge_initialize_platform_fee_1001_just_above_max_is_rejected() {
    let (env, cid, admin) = uninitialized_contract();
    let client = PredictifyHybridClient::new(&env, &cid);
    assert_eq!(
        client.try_initialize(&admin, &Some(1001i128), &None),
        Err(Ok(Error::InvalidFeeConfig))
    );
}

#[test]
fn edge_initialize_platform_fee_i128_min_does_not_panic() {
    let (env, cid, admin) = uninitialized_contract();
    let client = PredictifyHybridClient::new(&env, &cid);
    assert_eq!(
        client.try_initialize(&admin, &Some(i128::MIN), &None),
        Err(Ok(Error::InvalidFeeConfig))
    );
}

#[test]
fn edge_initialize_platform_fee_i128_max_does_not_panic() {
    let (env, cid, admin) = uninitialized_contract();
    let client = PredictifyHybridClient::new(&env, &cid);
    assert_eq!(
        client.try_initialize(&admin, &Some(i128::MAX), &None),
        Err(Ok(Error::InvalidFeeConfig))
    );
}

/// `initialize` with no fee override at all must always succeed and is
/// unaffected by the bounds check (it only runs when `Some(fee)` is given).
#[test]
fn edge_initialize_without_fee_override_succeeds() {
    let (env, cid, admin) = uninitialized_contract();
    let client = PredictifyHybridClient::new(&env, &cid);
    assert_eq!(client.try_initialize(&admin, &None, &None), Ok(Ok(())));
}
