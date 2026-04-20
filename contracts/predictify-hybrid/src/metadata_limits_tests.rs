/// Comprehensive tests for metadata length limits
///
/// This test module validates that all metadata length limits are properly enforced
/// and that validation functions correctly reject oversized inputs while accepting
/// valid inputs.

#[cfg(test)]
mod tests {
    use crate::metadata_limits::*;
    use crate::types::*;
    use crate::Error;
    use alloc::format;
    use alloc::string::ToString;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{Address, Env, String, Vec};

    // ===== STRING LENGTH VALIDATION TESTS =====

    #[test]
    fn test_question_length_valid() {
        let env = Env::default();
        let question = String::from_str(&env, "Will Bitcoin reach $100,000 by end of year?");
        assert!(validate_question_length(&question).is_ok());
    }

    #[test]
    fn test_question_length_at_limit() {
        let env = Env::default();
        let question = String::from_str(&env, &"a".repeat(MAX_QUESTION_LENGTH as usize));
        assert!(validate_question_length(&question).is_ok());
    }

    #[test]
    fn test_question_length_exceeds_limit() {
        let env = Env::default();
        let question = String::from_str(&env, &"a".repeat((MAX_QUESTION_LENGTH + 1) as usize));
        assert_eq!(
            validate_question_length(&question),
            Err(Error::QuestionTooLong)
        );
    }

    #[test]
    fn test_outcome_length_valid() {
        let env = Env::default();
        let outcome = String::from_str(&env, "yes");
        assert!(validate_outcome_length(&outcome).is_ok());
    }

    #[test]
    fn test_outcome_length_exceeds_limit() {
        let env = Env::default();
        let outcome = String::from_str(&env, &"a".repeat((MAX_OUTCOME_LENGTH + 1) as usize));
        assert_eq!(
            validate_outcome_length(&outcome),
            Err(Error::OutcomeTooLong)
        );
    }

    #[test]
    fn test_feed_id_length_valid() {
        let env = Env::default();
        let feed_id = String::from_str(&env, "BTC/USD");
        assert!(validate_feed_id_length(&feed_id).is_ok());
    }

    #[test]
    fn test_feed_id_length_pyth_format() {
        let env = Env::default();
        // Pyth uses 64-character hex strings
        let host_feed_id = format!("0x{}", "a".repeat(64));
        let feed_id = String::from_str(&env, host_feed_id.as_str());
        assert!(validate_feed_id_length(&feed_id).is_ok());
    }

    #[test]
    fn test_feed_id_length_exceeds_limit() {
        let env = Env::default();
        let feed_id = String::from_str(&env, &"a".repeat((MAX_FEED_ID_LENGTH + 1) as usize));
        assert_eq!(validate_feed_id_length(&feed_id), Err(Error::FeedIdTooLong));
    }

    #[test]
    fn test_comparison_length_valid() {
        let env = Env::default();
        let comparison = String::from_str(&env, "gt");
        assert!(validate_comparison_length(&comparison).is_ok());
    }

    #[test]
    fn test_comparison_length_exceeds_limit() {
        let env = Env::default();
        let comparison = String::from_str(&env, &"a".repeat((MAX_COMPARISON_LENGTH + 1) as usize));
        assert_eq!(
            validate_comparison_length(&comparison),
            Err(Error::ComparisonTooLong)
        );
    }

    #[test]
    fn test_category_length_valid() {
        let env = Env::default();
        let category = String::from_str(&env, "cryptocurrency");
        assert!(validate_category_length(&category).is_ok());
    }

    #[test]
    fn test_category_length_exceeds_limit() {
        let env = Env::default();
        let category = String::from_str(&env, &"a".repeat((MAX_CATEGORY_LENGTH + 1) as usize));
        assert_eq!(
            validate_category_length(&category),
            Err(Error::CategoryTooLong)
        );
    }

    #[test]
    fn test_tag_length_valid() {
        let env = Env::default();
        let tag = String::from_str(&env, "bitcoin");
        assert!(validate_tag_length(&tag).is_ok());
    }

    #[test]
    fn test_tag_length_exceeds_limit() {
        let env = Env::default();
        let tag = String::from_str(&env, &"a".repeat((MAX_TAG_LENGTH + 1) as usize));
        assert_eq!(validate_tag_length(&tag), Err(Error::TagTooLong));
    }

    #[test]
    fn test_extension_reason_length_valid() {
        let env = Env::default();
        let reason = String::from_str(
            &env,
            "Low participation detected, extending to allow more users to participate.",
        );
        assert!(validate_extension_reason_length(&reason).is_ok());
    }

    #[test]
    fn test_extension_reason_length_exceeds_limit() {
        let env = Env::default();
        let reason = String::from_str(
            &env,
            &"a".repeat((MAX_EXTENSION_REASON_LENGTH + 1) as usize),
        );
        assert_eq!(
            validate_extension_reason_length(&reason),
            Err(Error::ExtensionReasonTooLong)
        );
    }

    #[test]
    fn test_source_length_valid() {
        let env = Env::default();
        let source = String::from_str(&env, "reflector-mainnet");
        assert!(validate_source_length(&source).is_ok());
    }

    #[test]
    fn test_source_length_exceeds_limit() {
        let env = Env::default();
        let source = String::from_str(&env, &"a".repeat((MAX_SOURCE_LENGTH + 1) as usize));
        assert_eq!(validate_source_length(&source), Err(Error::SourceTooLong));
    }

    #[test]
    fn test_error_message_length_valid() {
        let env = Env::default();
        let error_msg = String::from_str(&env, "Oracle data is stale");
        assert!(validate_error_message_length(&error_msg).is_ok());
    }

    #[test]
    fn test_error_message_length_exceeds_limit() {
        let env = Env::default();
        let error_msg =
            String::from_str(&env, &"a".repeat((MAX_ERROR_MESSAGE_LENGTH + 1) as usize));
        assert_eq!(
            validate_error_message_length(&error_msg),
            Err(Error::ErrorMessageTooLong)
        );
    }

    #[test]
    fn test_signature_length_valid() {
        let env = Env::default();
        // Simulate a base64-encoded signature
        let signature = String::from_str(&env, &"a".repeat(400));
        assert!(validate_signature_length(&signature).is_ok());
    }

    #[test]
    fn test_signature_length_exceeds_limit() {
        let env = Env::default();
        let signature = String::from_str(&env, &"a".repeat((MAX_SIGNATURE_LENGTH + 1) as usize));
        assert_eq!(
            validate_signature_length(&signature),
            Err(Error::SignatureTooLong)
        );
    }

    // ===== VECTOR LENGTH VALIDATION TESTS =====

    #[test]
    fn test_outcomes_count_valid() {
        let env = Env::default();
        let outcomes = Vec::from_array(
            &env,
            [String::from_str(&env, "yes"), String::from_str(&env, "no")],
        );
        assert!(validate_outcomes_count(&outcomes).is_ok());
    }

    #[test]
    fn test_outcomes_count_at_limit() {
        let env = Env::default();
        let mut outcomes = Vec::new(&env);
        for i in 0..MAX_OUTCOMES_COUNT {
            outcomes.push_back(String::from_str(&env, &format!("outcome_{}", i)));
        }
        assert!(validate_outcomes_count(&outcomes).is_ok());
    }

    #[test]
    fn test_outcomes_count_exceeds_limit() {
        let env = Env::default();
        let mut outcomes = Vec::new(&env);
        for i in 0..(MAX_OUTCOMES_COUNT + 1) {
            outcomes.push_back(String::from_str(&env, &format!("outcome_{}", i)));
        }
        assert_eq!(
            validate_outcomes_count(&outcomes),
            Err(Error::TooManyOutcomes)
        );
    }

    #[test]
    fn test_outcomes_length_all_valid() {
        let env = Env::default();
        let outcomes = Vec::from_array(
            &env,
            [
                String::from_str(&env, "yes"),
                String::from_str(&env, "no"),
                String::from_str(&env, "maybe"),
            ],
        );
        assert!(validate_outcomes_length(&outcomes).is_ok());
    }

    #[test]
    fn test_outcomes_length_one_too_long() {
        let env = Env::default();
        let outcomes = Vec::from_array(
            &env,
            [
                String::from_str(&env, "yes"),
                String::from_str(&env, &"a".repeat((MAX_OUTCOME_LENGTH + 1) as usize)),
            ],
        );
        assert_eq!(
            validate_outcomes_length(&outcomes),
            Err(Error::OutcomeTooLong)
        );
    }

    #[test]
    fn test_tags_count_valid() {
        let env = Env::default();
        let tags = Vec::from_array(
            &env,
            [
                String::from_str(&env, "bitcoin"),
                String::from_str(&env, "crypto"),
                String::from_str(&env, "price"),
            ],
        );
        assert!(validate_tags_count(&tags).is_ok());
    }

    #[test]
    fn test_tags_count_at_limit() {
        let env = Env::default();
        let mut tags = Vec::new(&env);
        for i in 0..MAX_TAGS_COUNT {
            tags.push_back(String::from_str(&env, &format!("tag{}", i)));
        }
        assert!(validate_tags_count(&tags).is_ok());
    }

    #[test]
    fn test_tags_count_exceeds_limit() {
        let env = Env::default();
        let mut tags = Vec::new(&env);
        for i in 0..(MAX_TAGS_COUNT + 1) {
            tags.push_back(String::from_str(&env, &format!("tag{}", i)));
        }
        assert_eq!(validate_tags_count(&tags), Err(Error::TooManyTags));
    }

    #[test]
    fn test_tags_length_all_valid() {
        let env = Env::default();
        let tags = Vec::from_array(
            &env,
            [
                String::from_str(&env, "bitcoin"),
                String::from_str(&env, "ethereum"),
                String::from_str(&env, "stellar"),
            ],
        );
        assert!(validate_tags_length(&tags).is_ok());
    }

    #[test]
    fn test_tags_length_one_too_long() {
        let env = Env::default();
        let tags = Vec::from_array(
            &env,
            [
                String::from_str(&env, "bitcoin"),
                String::from_str(&env, &"a".repeat((MAX_TAG_LENGTH + 1) as usize)),
            ],
        );
        assert_eq!(validate_tags_length(&tags), Err(Error::TagTooLong));
    }

    #[test]
    fn test_extension_history_count_valid() {
        assert!(validate_extension_history_count(10).is_ok());
    }

    #[test]
    fn test_extension_history_count_at_limit() {
        assert!(validate_extension_history_count(MAX_EXTENSION_HISTORY_COUNT).is_ok());
    }

    #[test]
    fn test_extension_history_count_exceeds_limit() {
        assert_eq!(
            validate_extension_history_count(MAX_EXTENSION_HISTORY_COUNT + 1),
            Err(Error::TooManyExtensions)
        );
    }

    #[test]
    fn test_oracle_results_count_valid() {
        assert!(validate_oracle_results_count(5).is_ok());
    }

    #[test]
    fn test_oracle_results_count_at_limit() {
        assert!(validate_oracle_results_count(MAX_ORACLE_RESULTS_COUNT).is_ok());
    }

    #[test]
    fn test_oracle_results_count_exceeds_limit() {
        assert_eq!(
            validate_oracle_results_count(MAX_ORACLE_RESULTS_COUNT + 1),
            Err(Error::TooManyOracleResults)
        );
    }

    #[test]
    fn test_winning_outcomes_count_valid() {
        let env = Env::default();
        let outcomes = Vec::from_array(
            &env,
            [String::from_str(&env, "yes"), String::from_str(&env, "no")],
        );
        assert!(validate_winning_outcomes_count(&outcomes).is_ok());
    }

    #[test]
    fn test_winning_outcomes_count_at_limit() {
        let env = Env::default();
        let mut outcomes = Vec::new(&env);
        for i in 0..MAX_WINNING_OUTCOMES_COUNT {
            outcomes.push_back(String::from_str(&env, &format!("outcome_{}", i)));
        }
        assert!(validate_winning_outcomes_count(&outcomes).is_ok());
    }

    #[test]
    fn test_winning_outcomes_count_exceeds_limit() {
        let env = Env::default();
        let mut outcomes = Vec::new(&env);
        for i in 0..(MAX_WINNING_OUTCOMES_COUNT + 1) {
            outcomes.push_back(String::from_str(&env, &format!("outcome_{}", i)));
        }
        assert_eq!(
            validate_winning_outcomes_count(&outcomes),
            Err(Error::TooManyWinningOutcomes)
        );
    }

    // ===== INTEGRATION TESTS WITH TYPES =====

    #[test]
    fn test_oracle_config_validates_feed_id_length() {
        let env = Env::default();
        let oracle_address = Address::generate(&env);

        let config = OracleConfig::new(
            OracleProvider::Reflector,
            oracle_address,
            String::from_str(&env, &"a".repeat((MAX_FEED_ID_LENGTH + 1) as usize)),
            10_000_000,
            String::from_str(&env, "gt"),
        );

        assert_eq!(config.validate(&env), Err(Error::FeedIdTooLong));
    }

    #[test]
    fn test_oracle_config_validates_comparison_length() {
        let env = Env::default();
        let oracle_address = Address::generate(&env);

        let config = OracleConfig::new(
            OracleProvider::Reflector,
            oracle_address,
            String::from_str(&env, "BTC/USD"),
            10_000_000,
            String::from_str(&env, &"a".repeat((MAX_COMPARISON_LENGTH + 1) as usize)),
        );

        assert_eq!(config.validate(&env), Err(Error::ComparisonTooLong));
    }

    #[test]
    fn test_market_validates_question_length() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let oracle_address = Address::generate(&env);

        let mut market = Market::new(
            &env,
            admin,
            String::from_str(&env, &"a".repeat((MAX_QUESTION_LENGTH + 1) as usize)),
            Vec::from_array(
                &env,
                [String::from_str(&env, "yes"), String::from_str(&env, "no")],
            ),
            env.ledger().timestamp() + 86400,
            OracleConfig::new(
                OracleProvider::Reflector,
                oracle_address,
                String::from_str(&env, "BTC/USD"),
                10_000_000,
                String::from_str(&env, "gt"),
            ),
            None,
            3600,
            MarketState::Active,
        );

        assert_eq!(market.validate(&env), Err(Error::QuestionTooLong));
    }

    #[test]
    fn test_market_validates_outcomes_count() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let oracle_address = Address::generate(&env);

        let mut outcomes = Vec::new(&env);
        for i in 0..(MAX_OUTCOMES_COUNT + 1) {
            outcomes.push_back(String::from_str(&env, &format!("outcome_{}", i)));
        }

        let mut market = Market::new(
            &env,
            admin,
            String::from_str(&env, "Will BTC reach $100k?"),
            outcomes,
            env.ledger().timestamp() + 86400,
            OracleConfig::new(
                OracleProvider::Reflector,
                oracle_address,
                String::from_str(&env, "BTC/USD"),
                10_000_000,
                String::from_str(&env, "gt"),
            ),
            None,
            3600,
            MarketState::Active,
        );

        assert_eq!(market.validate(&env), Err(Error::TooManyOutcomes));
    }

    #[test]
    fn test_market_extension_validates_reason_length() {
        let env = Env::default();
        let admin = Address::generate(&env);

        let extension = MarketExtension::new(
            &env,
            7,
            admin,
            String::from_str(
                &env,
                &"a".repeat((MAX_EXTENSION_REASON_LENGTH + 1) as usize),
            ),
            1_000_000,
        );

        assert_eq!(extension.validate(), Err(Error::ExtensionReasonTooLong));
    }

    // ===== EDGE CASE TESTS =====

    #[test]
    fn test_empty_string_is_valid() {
        let env = Env::default();
        let empty = String::from_str(&env, "");
        assert!(validate_question_length(&empty).is_ok());
        assert!(validate_outcome_length(&empty).is_ok());
        assert!(validate_feed_id_length(&empty).is_ok());
    }

    #[test]
    fn test_empty_vector_is_valid() {
        let env = Env::default();
        let empty = Vec::new(&env);
        assert!(validate_outcomes_count(&empty).is_ok());
        assert!(validate_tags_count(&empty).is_ok());
        assert!(validate_winning_outcomes_count(&empty).is_ok());
    }

    #[test]
    fn test_zero_count_is_valid() {
        assert!(validate_extension_history_count(0).is_ok());
        assert!(validate_oracle_results_count(0).is_ok());
    }
}
