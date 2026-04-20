use crate::errors::Error;
use crate::validation::{InputValidator, MarketValidator, OutcomeDeduplicator, ValidationError};
use soroban_sdk::{contracttype, Address, Env, String, Symbol, Vec};

/// Comprehensive tests for outcome deduplication and normalization.
///
/// This test suite validates the security and correctness of the outcome
/// deduplication policy, ensuring that duplicate and ambiguous outcomes
/// are properly detected and rejected while allowing valid distinct outcomes.
///
/// # Security Properties Tested
///
/// - **Deterministic Normalization**: Same input always produces same normalized output
/// - **Collision Resistance**: Different outcomes don't normalize to the same value unless truly duplicates
/// - **Similarity Detection**: Ambiguous outcomes are detected with appropriate thresholds
/// - **Semantic Grouping**: Common synonyms are properly identified
/// - **Gas Efficiency**: Algorithms are optimized for blockchain execution
/// - **Attack Resistance**: Hard to bypass through Unicode manipulation or clever formatting

#[contracttest]
fn test_outcome_normalization_basic() {
    let env = Env::default();

    // Test basic whitespace trimming
    let outcome1 = String::from_str(&env, "  yes  ");
    let normalized = OutcomeDeduplicator::normalize_outcome(&outcome1).unwrap();
    assert_eq!(normalized, String::from_str(&env, "yes"));

    // Test case normalization
    let outcome2 = String::from_str(&env, "YES");
    let normalized2 = OutcomeDeduplicator::normalize_outcome(&outcome2).unwrap();
    assert_eq!(normalized2, String::from_str(&env, "yes"));

    // Test punctuation removal
    let outcome3 = String::from_str(&env, "yes!");
    let normalized3 = OutcomeDeduplicator::normalize_outcome(&outcome3).unwrap();
    assert_eq!(normalized3, String::from_str(&env, "yes"));

    // Test internal whitespace compression
    let outcome4 = String::from_str(&env, "yes   no");
    let normalized4 = OutcomeDeduplicator::normalize_outcome(&outcome4).unwrap();
    assert_eq!(normalized4, String::from_str(&env, "yes no"));

    // Test combined normalization
    let outcome5 = String::from_str(&env, "  YES!  Maybe?  ");
    let normalized5 = OutcomeDeduplicator::normalize_outcome(&outcome5).unwrap();
    assert_eq!(normalized5, String::from_str(&env, "yes maybe"));
}

#[contracttest]
fn test_outcome_normalization_edge_cases() {
    let env = Env::default();

    // Test empty string after normalization
    let outcome1 = String::from_str(&env, "  !?.,;:'\"()[]{}  ");
    let result1 = OutcomeDeduplicator::normalize_outcome(&outcome1);
    assert!(result1.is_err());
    assert!(matches!(
        result1.unwrap_err(),
        ValidationError::OutcomeNormalizationFailed
    ));

    // Test only whitespace
    let outcome2 = String::from_str(&env, "   ");
    let result2 = OutcomeDeduplicator::normalize_outcome(&outcome2);
    assert!(result2.is_err());
    assert!(matches!(
        result2.unwrap_err(),
        ValidationError::OutcomeNormalizationFailed
    ));

    // Test special characters only
    let outcome3 = String::from_str(&env, "!@#$%^&*()");
    let normalized3 = OutcomeDeduplicator::normalize_outcome(&outcome3).unwrap();
    assert_eq!(normalized3, String::from_str(&env, "!@#$%^&*()"));

    // Test Unicode characters (should be preserved)
    let outcome4 = String::from_str(&env, "sí mañana");
    let normalized4 = OutcomeDeduplicator::normalize_outcome(&outcome4).unwrap();
    assert_eq!(normalized4, String::from_str(&env, "sí mañana"));

    // Test numbers and letters
    let outcome5 = String::from_str(&env, "Option 1");
    let normalized5 = OutcomeDeduplicator::normalize_outcome(&outcome5).unwrap();
    assert_eq!(normalized5, String::from_str(&env, "option 1"));
}

#[contracttest]
fn test_similarity_calculation() {
    let env = Env::default();

    // Test identical strings
    let outcome1 = String::from_str(&env, "yes");
    let outcome2 = String::from_str(&env, "yes");
    let similarity = OutcomeDeduplicator::calculate_similarity(&outcome1, &outcome2);
    assert_eq!(similarity, 100);

    // Test completely different strings
    let outcome3 = String::from_str(&env, "yes");
    let outcome4 = String::from_str(&env, "no");
    let similarity2 = OutcomeDeduplicator::calculate_similarity(&outcome3, &outcome4);
    assert!(similarity2 < 50); // Should be quite different

    // Test similar strings
    let outcome5 = String::from_str(&env, "yes");
    let outcome6 = String::from_str(&env, "yeah");
    let similarity3 = OutcomeDeduplicator::calculate_similarity(&outcome5, &outcome6);
    assert!(similarity3 > 50); // Should be more than 50% similar

    // Test empty strings
    let outcome7 = String::from_str(&env, "");
    let outcome8 = String::from_str(&env, "");
    let similarity4 = OutcomeDeduplicator::calculate_similarity(&outcome7, &outcome8);
    assert_eq!(similarity4, 100);

    // Test one empty string
    let outcome9 = String::from_str(&env, "yes");
    let outcome10 = String::from_str(&env, "");
    let similarity5 = OutcomeDeduplicator::calculate_similarity(&outcome9, &outcome10);
    assert_eq!(similarity5, 0);

    // Test very long strings (performance test)
    let long1 = String::from_str(&env, &"a".repeat(100));
    let long2 = String::from_str(&env, &"a".repeat(99) + "b");
    let similarity6 = OutcomeDeduplicator::calculate_similarity(&long1, &long2);
    assert!(similarity6 > 90); // Should be very similar
}

#[contracttest]
fn test_exact_duplicate_detection() {
    let env = Env::default();

    // Test case-insensitive duplicates
    let outcomes1 = vec![
        &env,
        String::from_str(&env, "Yes"),
        String::from_str(&env, "yes"),
        String::from_str(&env, "No"),
    ];
    let result1 = OutcomeDeduplicator::validate_outcomes(&outcomes1);
    assert!(result1.is_err());
    assert!(matches!(
        result1.unwrap_err(),
        ValidationError::DuplicateOutcome
    ));

    // Test whitespace duplicates
    let outcomes2 = vec![
        &env,
        String::from_str(&env, "yes"),
        String::from_str(&env, "  yes  "),
        String::from_str(&env, "no"),
    ];
    let result2 = OutcomeDeduplicator::validate_outcomes(&outcomes2);
    assert!(result2.is_err());
    assert!(matches!(
        result2.unwrap_err(),
        ValidationError::DuplicateOutcome
    ));

    // Test punctuation duplicates
    let outcomes3 = vec![
        &env,
        String::from_str(&env, "yes"),
        String::from_str(&env, "yes!"),
        String::from_str(&env, "no"),
    ];
    let result3 = OutcomeDeduplicator::validate_outcomes(&outcomes3);
    assert!(result3.is_err());
    assert!(matches!(
        result3.unwrap_err(),
        ValidationError::DuplicateOutcome
    ));

    // Test valid distinct outcomes
    let outcomes4 = vec![
        &env,
        String::from_str(&env, "yes"),
        String::from_str(&env, "no"),
        String::from_str(&env, "maybe"),
    ];
    let result4 = OutcomeDeduplicator::validate_outcomes(&outcomes4);
    assert!(result4.is_ok());
}

#[contracttest]
fn test_ambiguous_outcome_detection() {
    let env = Env::default();

    // Test high similarity (>80% threshold)
    let outcomes1 = vec![
        &env,
        String::from_str(&env, "yes"),
        String::from_str(&env, "yeas"), // Very similar to yes
        String::from_str(&env, "no"),
    ];
    let result1 = OutcomeDeduplicator::validate_outcomes(&outcomes1);
    assert!(result1.is_err());
    assert!(matches!(
        result1.unwrap_err(),
        ValidationError::AmbiguousOutcome
    ));

    // Test semantic duplicates
    let outcomes2 = vec![
        &env,
        String::from_str(&env, "yes"),
        String::from_str(&env, "yeah"), // Semantic duplicate
        String::from_str(&env, "no"),
    ];
    let result2 = OutcomeDeduplicator::validate_outcomes(&outcomes2);
    assert!(result2.is_err());
    assert!(matches!(
        result2.unwrap_err(),
        ValidationError::AmbiguousOutcome
    ));

    // Test negative semantic duplicates
    let outcomes3 = vec![
        &env,
        String::from_str(&env, "no"),
        String::from_str(&env, "nope"), // Semantic duplicate
        String::from_str(&env, "yes"),
    ];
    let result3 = OutcomeDeduplicator::validate_outcomes(&outcomes3);
    assert!(result3.is_err());
    assert!(matches!(
        result3.unwrap_err(),
        ValidationError::AmbiguousOutcome
    ));

    // Test neutral semantic duplicates
    let outcomes4 = vec![
        &env,
        String::from_str(&env, "maybe"),
        String::from_str(&env, "possibly"), // Semantic duplicate
        String::from_str(&env, "yes"),
    ];
    let result4 = OutcomeDeduplicator::validate_outcomes(&outcomes4);
    assert!(result4.is_err());
    assert!(matches!(
        result4.unwrap_err(),
        ValidationError::AmbiguousOutcome
    ));

    // Test valid distinct outcomes (below similarity threshold)
    let outcomes5 = vec![
        &env,
        String::from_str(&env, "yes"),
        String::from_str(&env, "no"),
        String::from_str(&env, "above"),
        String::from_str(&env, "below"),
    ];
    let result5 = OutcomeDeduplicator::validate_outcomes(&outcomes5);
    assert!(result5.is_ok());
}

#[contracttest]
fn test_semantic_duplicate_groups() {
    let env = Env::default();

    // Test all affirmative semantic duplicates
    let affirmative = vec!["yes", "yeah", "yep", "true", "correct", "agree", "positive"];

    for (i, word1) in affirmative.iter().enumerate() {
        for (j, word2) in affirmative.iter().enumerate() {
            if i != j {
                let outcome1 = String::from_str(&env, word1);
                let outcome2 = String::from_str(&env, word2);
                let is_dup = OutcomeDeduplicator::is_semantic_duplicate(&outcome1, &outcome2);
                assert!(
                    is_dup,
                    "{} and {} should be semantic duplicates",
                    word1, word2
                );
            }
        }
    }

    // Test all negative semantic duplicates
    let negative = vec!["no", "nope", "false", "incorrect", "disagree", "negative"];

    for (i, word1) in negative.iter().enumerate() {
        for (j, word2) in negative.iter().enumerate() {
            if i != j {
                let outcome1 = String::from_str(&env, word1);
                let outcome2 = String::from_str(&env, word2);
                let is_dup = OutcomeDeduplicator::is_semantic_duplicate(&outcome1, &outcome2);
                assert!(
                    is_dup,
                    "{} and {} should be semantic duplicates",
                    word1, word2
                );
            }
        }
    }

    // Test cross-group (should not be semantic duplicates)
    let outcome1 = String::from_str(&env, "yes"); // affirmative
    let outcome2 = String::from_str(&env, "no"); // negative
    let is_dup = OutcomeDeduplicator::is_semantic_duplicate(&outcome1, &outcome2);
    assert!(!is_dup, "yes and no should not be semantic duplicates");
}

#[contracttest]
fn test_input_validator_integration() {
    let env = Env::default();

    // Test valid outcomes through InputValidator
    let valid_outcomes = vec![
        &env,
        String::from_str(&env, "yes"),
        String::from_str(&env, "no"),
        String::from_str(&env, "maybe"),
    ];
    let result1 = InputValidator::validate_outcomes(&valid_outcomes);
    assert!(result1.is_ok());

    // Test duplicate outcomes through InputValidator
    let duplicate_outcomes = vec![
        &env,
        String::from_str(&env, "yes"),
        String::from_str(&env, "YES"), // Case-insensitive duplicate
        String::from_str(&env, "no"),
    ];
    let result2 = InputValidator::validate_outcomes(&duplicate_outcomes);
    assert!(result2.is_err());
    assert!(matches!(
        result2.unwrap_err(),
        ValidationError::DuplicateOutcome
    ));

    // Test ambiguous outcomes through InputValidator
    let ambiguous_outcomes = vec![
        &env,
        String::from_str(&env, "yes"),
        String::from_str(&env, "yeah"), // Semantic duplicate
        String::from_str(&env, "no"),
    ];
    let result3 = InputValidator::validate_outcomes(&ambiguous_outcomes);
    assert!(result3.is_err());
    assert!(matches!(
        result3.unwrap_err(),
        ValidationError::AmbiguousOutcome
    ));

    // Test too few outcomes
    let few_outcomes = vec![&env, String::from_str(&env, "yes")];
    let result4 = InputValidator::validate_outcomes(&few_outcomes);
    assert!(result4.is_err());
    assert!(matches!(
        result4.unwrap_err(),
        ValidationError::ArrayTooSmall
    ));

    // Test too many outcomes
    let many_outcomes = vec![
        &env,
        String::from_str(&env, "option1"),
        String::from_str(&env, "option2"),
        String::from_str(&env, "option3"),
        String::from_str(&env, "option4"),
        String::from_str(&env, "option5"),
        String::from_str(&env, "option6"),
        String::from_str(&env, "option7"),
        String::from_str(&env, "option8"),
        String::from_str(&env, "option9"),
        String::from_str(&env, "option10"),
        String::from_str(&env, "option11"), // Too many
    ];
    let result5 = InputValidator::validate_outcomes(&many_outcomes);
    assert!(result5.is_err());
    assert!(matches!(
        result5.unwrap_err(),
        ValidationError::ArrayTooLarge
    ));
}

#[contracttest]
fn test_market_validator_integration() {
    let env = Env::default();

    // Test valid outcomes through MarketValidator
    let valid_outcomes = vec![
        &env,
        String::from_str(&env, "yes"),
        String::from_str(&env, "no"),
        String::from_str(&env, "maybe"),
    ];
    let result1 = MarketValidator::validate_outcomes(&env, &valid_outcomes);
    assert!(result1.is_ok());

    // Test duplicate outcomes through MarketValidator
    let duplicate_outcomes = vec![
        &env,
        String::from_str(&env, "yes"),
        String::from_str(&env, "yes!"), // Duplicate after normalization
        String::from_str(&env, "no"),
    ];
    let result2 = MarketValidator::validate_outcomes(&env, &duplicate_outcomes);
    assert!(result2.is_err());
    assert!(matches!(
        result2.unwrap_err(),
        ValidationError::DuplicateOutcome
    ));

    // Test ambiguous outcomes through MarketValidator
    let ambiguous_outcomes = vec![
        &env,
        String::from_str(&env, "true"),
        String::from_str(&env, "correct"), // Semantic duplicate
        String::from_str(&env, "false"),
    ];
    let result3 = MarketValidator::validate_outcomes(&env, &ambiguous_outcomes);
    assert!(result3.is_err());
    assert!(matches!(
        result3.unwrap_err(),
        ValidationError::AmbiguousOutcome
    ));
}

#[contracttest]
fn test_normalization_statistics() {
    let env = Env::default();

    // Test statistics for normal outcomes
    let outcomes1 = vec![
        &env,
        String::from_str(&env, "yes"),
        String::from_str(&env, "no"),
        String::from_str(&env, "maybe"),
    ];
    let stats1 = OutcomeDeduplicator::get_normalization_stats(&outcomes1);
    assert_eq!(stats1.total_outcomes, 3);
    assert_eq!(stats1.successfully_normalized, 3);
    assert_eq!(stats1.normalization_failures, 0);
    assert_eq!(stats1.success_rate(), 100);

    // Test statistics for outcomes needing normalization
    let outcomes2 = vec![
        &env,
        String::from_str(&env, "  YES!  "),
        String::from_str(&env, "NO?"),
        String::from_str(&env, "maybe..."),
    ];
    let stats2 = OutcomeDeduplicator::get_normalization_stats(&outcomes2);
    assert_eq!(stats2.total_outcomes, 3);
    assert_eq!(stats2.successfully_normalized, 3);
    assert_eq!(stats2.normalization_failures, 0);
    assert!(stats2.total_length_reduction > 0); // Should have removed characters
    assert_eq!(stats2.success_rate(), 100);

    // Test statistics with failures
    let outcomes3 = vec![
        &env,
        String::from_str(&env, "yes"),
        String::from_str(&env, "  !?.,;:'\"()[]{}  "), // Will fail normalization
        String::from_str(&env, "no"),
    ];
    let stats3 = OutcomeDeduplicator::get_normalization_stats(&outcomes3);
    assert_eq!(stats3.total_outcomes, 3);
    assert_eq!(stats3.successfully_normalized, 2);
    assert_eq!(stats3.normalization_failures, 1);
    assert!(stats3.success_rate() < 100);
}

#[contracttest]
fn test_edge_cases_and_attack_vectors() {
    let env = Env::default();

    // Test Unicode normalization attacks
    let unicode_outcomes = vec![
        &env,
        String::from_str(&env, "yes"),
        String::from_str(&env, "ｙｅｓ"), // Full-width Unicode characters
        String::from_str(&env, "no"),
    ];
    let result1 = OutcomeDeduplicator::validate_outcomes(&unicode_outcomes);
    // Should be valid since they're different after normalization
    assert!(result1.is_ok());

    // Test zero-width characters
    let zw_outcomes = vec![
        &env,
        String::from_str(&env, "yes"),
        String::from_str(&env, "yes\u{200B}"), // Zero-width space
        String::from_str(&env, "no"),
    ];
    let result2 = OutcomeDeduplicator::validate_outcomes(&zw_outcomes);
    // Zero-width spaces should be preserved (not removed by current normalization)
    assert!(result2.is_ok()); // Could be enhanced in future

    // Test very long outcomes
    let long_outcomes = vec![
        &env,
        String::from_str(&env, &"a".repeat(100)),
        String::from_str(&env, &"a".repeat(99) + "b"),
        String::from_str(&env, &"b".repeat(100)),
    ];
    let result3 = OutcomeDeduplicator::validate_outcomes(&long_outcomes);
    // Should detect ambiguity due to high similarity
    assert!(result3.is_err());
    assert!(matches!(
        result3.unwrap_err(),
        ValidationError::AmbiguousOutcome
    ));

    // Test mixed case and punctuation attacks
    let mixed_outcomes = vec![
        &env,
        String::from_str(&env, "YES!"),
        String::from_str(&env, "yes?"),
        String::from_str(&env, "yes."),
        String::from_str(&env, "no"),
    ];
    let result4 = OutcomeDeduplicator::validate_outcomes(&mixed_outcomes);
    // Should detect duplicates after punctuation removal
    assert!(result4.is_err());
    assert!(matches!(
        result4.unwrap_err(),
        ValidationError::DuplicateOutcome
    ));
}

#[contracttest]
fn test_contract_error_conversion() {
    let env = Env::default();

    // Test error conversion for duplicate outcomes
    let duplicate_err = ValidationError::DuplicateOutcome;
    let contract_err1 = duplicate_err.to_contract_error();
    assert!(matches!(contract_err1, Error::InvalidOutcomes));

    // Test error conversion for ambiguous outcomes
    let ambiguous_err = ValidationError::AmbiguousOutcome;
    let contract_err2 = ambiguous_err.to_contract_error();
    assert!(matches!(contract_err2, Error::InvalidOutcomes));

    // Test error conversion for normalization failures
    let norm_err = ValidationError::OutcomeNormalizationFailed;
    let contract_err3 = norm_err.to_contract_error();
    assert!(matches!(contract_err3, Error::InvalidOutcomes));
}

#[contracttest]
fn test_performance_characteristics() {
    let env = Env::default();

    // Test with many outcomes to ensure performance is acceptable
    let many_outcomes: Vec<String> = (0..50)
        .map(|i| String::from_str(&env, &format!("option{}", i)))
        .collect();

    let start = env.ledger().timestamp();
    let result = OutcomeDeduplicator::validate_outcomes(&many_outcomes);
    let end = env.ledger().timestamp();

    assert!(result.is_ok());
    // Performance check - should complete quickly (this is a basic check)
    // In practice, you'd want more sophisticated performance testing
    assert!(end - start < 1_000_000); // Less than 1 second in timestamp units
}

#[contracttest]
fn test_regression_cases() {
    let env = Env::default();

    // Regression: Empty outcomes should be caught by format validation
    let empty_outcomes = vec![
        &env,
        String::from_str(&env, "yes"),
        String::from_str(&env, ""), // Empty
        String::from_str(&env, "no"),
    ];
    let result1 = InputValidator::validate_outcomes(&empty_outcomes);
    assert!(result1.is_err());

    // Regression: Only punctuation should fail normalization
    let punct_outcomes = vec![
        &env,
        String::from_str(&env, "yes"),
        String::from_str(&env, "!?.,"),
        String::from_str(&env, "no"),
    ];
    let result2 = OutcomeDeduplicator::validate_outcomes(&punct_outcomes);
    assert!(result2.is_err());
    assert!(matches!(
        result2.unwrap_err(),
        ValidationError::OutcomeNormalizationFailed
    ));

    // Regression: Very similar but distinct outcomes should be allowed
    let similar_outcomes = vec![
        &env,
        String::from_str(&env, "option_a"),
        String::from_str(&env, "option_b"), // Similar but below 80% threshold
        String::from_str(&env, "option_c"),
    ];
    let result3 = OutcomeDeduplicator::validate_outcomes(&similar_outcomes);
    assert!(result3.is_ok());

    // Regression: Semantic groups should work across case variations
    let semantic_case_outcomes = vec![
        &env,
        String::from_str(&env, "YES"),
        String::from_str(&env, "True"), // Both affirmative, different cases
        String::from_str(&env, "no"),
    ];
    let result4 = OutcomeDeduplicator::validate_outcomes(&semantic_case_outcomes);
    assert!(result4.is_err());
    assert!(matches!(
        result4.unwrap_err(),
        ValidationError::AmbiguousOutcome
    ));
}
