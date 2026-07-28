//! Client-facing error-code stability tests for the resolution subsystem.
//!
//! These assertions intentionally freeze the numeric values exposed by the
//! resolution contract's `ContractError`.  Client applications may persist or
//! branch on these values, so changing one is a visible API change and
//! requires an explicit migration or versioning decision.
//!
//! The expected code table lives in `contracts/resolution/src/errors.rs`.
//! Any change to a discriminant there MUST be reflected in the
//! `EXPECTED_CODES` table below in the same PR and the rationale must be
//! called out in the rustdoc of the modified variant.
//!
//! See GrantFox FWC26 issue #933 (resolution v7) for the original
//! stabilization request.

use resolution::errors::ContractError;

// ---------------------------------------------------------------------------
// Expected error-code table
// ---------------------------------------------------------------------------
//
// The numeric assignments below are part of the client-facing contract.
// Keep this table explicit rather than deriving values from enum ordering:
// clients depend on the literal numbers, not on the relative order of the
// variants.
//
// When adding a new variant, append it here with the next free discriminant
// and update every assertion that iterates the catalog (e.g. contiguity).

/// `(variant, expected discriminant)` pairs for every `ContractError` variant
/// in the resolution v7 error catalog.
const EXPECTED_CODES: &[(ContractError, u32)] = &[
    (ContractError::Unauthorized, 1),
    (ContractError::MarketNotFound, 2),
    (ContractError::MarketClosed, 3),
    (ContractError::MarketAlreadyResolved, 4),
    (ContractError::InvalidOutcome, 5),
    (ContractError::InvalidInput, 6),
    (ContractError::InvalidState, 7),
    (ContractError::Overflow, 8),
    (ContractError::ResolutionCooldownActive, 9),
    (ContractError::OracleResultNotAvailable, 10),
    (ContractError::InvalidWinningOutcomes, 11),
];

// ---------------------------------------------------------------------------
// Individual code stability
// ---------------------------------------------------------------------------

/// Every documented `ContractError` variant must equal its frozen discriminant.
#[test]
fn resolution_error_codes_are_stable() {
    for (variant, expected) in EXPECTED_CODES {
        let measured = *variant as u32;
        assert_eq!(
            measured, *expected,
            "resolution error code changed for {variant:?}: \
             measured={measured}, expected={expected}; \
             this is a client-facing API change"
        );
    }
}

// ---------------------------------------------------------------------------
// Per-variant assertions (give CI a precise pinpoint when one code regresses)
// ---------------------------------------------------------------------------

#[test]
fn unauthorized_code_is_one() {
    assert_eq!(ContractError::Unauthorized as u32, 1);
}

#[test]
fn market_not_found_code_is_two() {
    assert_eq!(ContractError::MarketNotFound as u32, 2);
}

#[test]
fn market_closed_code_is_three() {
    assert_eq!(ContractError::MarketClosed as u32, 3);
}

#[test]
fn market_already_resolved_code_is_four() {
    assert_eq!(ContractError::MarketAlreadyResolved as u32, 4);
}

#[test]
fn invalid_outcome_code_is_five() {
    assert_eq!(ContractError::InvalidOutcome as u32, 5);
}

#[test]
fn invalid_input_code_is_six() {
    assert_eq!(ContractError::InvalidInput as u32, 6);
}

#[test]
fn invalid_state_code_is_seven() {
    assert_eq!(ContractError::InvalidState as u32, 7);
}

#[test]
fn overflow_code_is_eight() {
    assert_eq!(ContractError::Overflow as u32, 8);
}

#[test]
fn resolution_cooldown_active_code_is_nine() {
    assert_eq!(ContractError::ResolutionCooldownActive as u32, 9);
}

#[test]
fn oracle_result_not_available_code_is_ten() {
    assert_eq!(ContractError::OracleResultNotAvailable as u32, 10);
}

#[test]
fn invalid_winning_outcomes_code_is_eleven() {
    assert_eq!(ContractError::InvalidWinningOutcomes as u32, 11);
}

// ---------------------------------------------------------------------------
// Uniqueness and contiguity
// ---------------------------------------------------------------------------

/// All discriminants must be distinct.  Catches accidental duplication
/// introduced by copy/paste of `= N` values.
#[test]
fn resolution_error_codes_are_unique() {
    let mut codes: Vec<u32> = EXPECTED_CODES.iter().map(|(_, code)| *code).collect();
    codes.sort_unstable();
    let original_len = codes.len();
    codes.dedup();
    assert_eq!(
        codes.len(),
        original_len,
        "duplicate resolution error codes detected"
    );
}

/// The catalog is contiguous from 1 to N where N is the number of variants.
/// Clients (notably off-chain decoders) assume no gaps inside the range.
#[test]
fn resolution_error_codes_are_contiguous_from_one() {
    let mut codes: Vec<u32> = EXPECTED_CODES.iter().map(|(_, code)| *code).collect();
    codes.sort_unstable();

    let expected_count = EXPECTED_CODES.len() as u32;
    for (index, code) in codes.iter().enumerate() {
        assert_eq!(
            *code,
            1 + index as u32,
            "resolution error code gap at index {index}: code={code}"
        );
    }
    assert_eq!(
        codes.len() as u32,
        expected_count,
        "resolution error catalog must list every variant exactly once"
    );
}

/// `EXPECTED_CODES` must list each variant exactly once — duplicates here
/// would silently mask a regression.
#[test]
fn expected_codes_table_has_no_duplicate_variants() {
    let mut seen: Vec<ContractError> = Vec::with_capacity(EXPECTED_CODES.len());
    for (variant, _) in EXPECTED_CODES {
        if seen.contains(variant) {
            panic!("variant {variant:?} appears more than once in EXPECTED_CODES");
        }
        seen.push(*variant);
    }
}

// ---------------------------------------------------------------------------
// Trait guarantees
// ---------------------------------------------------------------------------

/// `ContractError` must be `Copy` so it can be returned by value without
/// allocation.  This is a contract-level guarantee that downstream code
/// (storage, events, error mapping) relies on.
#[test]
fn contract_error_is_copy() {
    let original = ContractError::InvalidOutcome;
    let copied = original;
    // Both bindings must remain usable — `Copy` does not move `original`.
    assert_eq!(original, copied);

    fn assert_copy<T: Copy>(_: T) {}
    assert_copy(ContractError::Overflow);
}

/// `ContractError` must implement `Clone` deterministically (i.e. the cloned
/// value compares equal to the source).
///
/// `Copy: Clone` already guarantees this at the type level; the explicit
/// generic-bound check below pins down the contract for downstream code that
/// may constrain on `Clone` only.
#[test]
fn contract_error_implements_clone() {
    fn assert_clone_eq<T: Clone + PartialEq + std::fmt::Debug>(value: T) -> bool {
        let cloned = value.clone();
        assert_eq!(cloned, value);
        cloned == value
    }
    assert!(assert_clone_eq(ContractError::OracleResultNotAvailable));
    assert!(assert_clone_eq(ContractError::InvalidWinningOutcomes));
}

/// `Debug` output must be well-formed (non-empty, identifier-shaped) and
/// every variant must produce a distinct rendering so client logs and panic
/// messages remain diagnosable.
///
/// Iterates `EXPECTED_CODES` so the assertion stays in lockstep with the
/// catalog: adding a new variant requires no edit to this test.
#[test]
fn contract_error_debug_is_well_formed_and_distinct() {
    let debug_outputs: Vec<String> = EXPECTED_CODES
        .iter()
        .map(|(variant, _)| format!("{variant:?}"))
        .collect();

    // Each Debug output must be a valid Rust identifier-shaped string:
    // non-empty, leading uppercase ASCII letter, alphanumeric / underscore.
    for output in &debug_outputs {
        assert!(!output.is_empty(), "Debug output must not be empty");
        let first = output
            .chars()
            .next()
            .expect("string is non-empty by prior assert");
        assert!(
            first.is_ascii_uppercase(),
            "Debug output \"{output}\" must start with an uppercase ASCII letter"
        );
        assert!(
            output.chars().all(|c| c.is_alphanumeric() || c == '_'),
            "Debug output \"{output}\" must contain only alphanumeric characters \
             or underscores"
        );
    }

    // Every variant must render distinctly — otherwise two distinct error
    // codes would be indistinguishable in logs and observability tooling.
    let mut sorted = debug_outputs.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(
        sorted.len(),
        debug_outputs.len(),
        "duplicate Debug outputs detected — variant Debug strings must be unique"
    );
}

/// Variants must be totally ordered by discriminant so they can be used as
/// keys in `BTreeMap` and similar ordered containers.
///
/// Asserts pairwise `<` across every adjacent pair in discriminant order so
/// adding a new variant requires no edit to this test.
#[test]
fn contract_error_is_ordered_by_discriminant() {
    let mut by_code: Vec<(u32, ContractError)> =
        EXPECTED_CODES.iter().map(|(v, c)| (*c, *v)).collect();
    by_code.sort_unstable_by_key(|(code, _)| *code);

    for pair in by_code.windows(2) {
        let (prev_code, prev_variant) = pair[0];
        let (next_code, next_variant) = pair[1];
        assert!(
            prev_variant < next_variant,
            "ordering broken: {prev_variant:?} ({prev_code}) must be < \
             {next_variant:?} ({next_code})"
        );
    }
}
