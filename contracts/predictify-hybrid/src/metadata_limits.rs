/// Metadata length limits for controlling storage costs and preventing denial-of-service attacks.
///
/// This module defines maximum length constraints for strings and vectors used throughout
/// the Predictify Hybrid smart contract. These limits serve multiple purposes:
///
/// # Security Benefits
///
/// - **DoS Prevention**: Prevents attackers from creating markets with excessively large metadata
/// - **Storage Cost Control**: Caps storage requirements to predictable, manageable levels
/// - **Gas Optimization**: Ensures operations complete within reasonable gas budgets
/// - **Data Integrity**: Enforces reasonable bounds on user-provided data
///
/// # Design Principles
///
/// 1. **Conservative Limits**: Set to accommodate legitimate use cases while preventing abuse
/// 2. **Auditor-Friendly**: Clear, documented constants that are easy to review
/// 3. **Upgrade Path**: Limits can be adjusted in future contract versions if needed
/// 4. **User Experience**: Generous enough to not hinder normal usage patterns
///
/// # Usage Example
///
/// ```rust
/// use predictify_hybrid::metadata_limits::{MAX_QUESTION_LENGTH, validate_question_length};
///
/// let question = String::from_str(&env, "Will BTC reach $100k?");
/// validate_question_length(&question)?; // Returns Ok if within limits
/// ```

use soroban_sdk::{String, Vec};

// ===== STRING LENGTH LIMITS =====

/// Maximum length for market question text (500 characters)
///
/// Rationale: Questions should be concise and clear. 500 characters allows for
/// detailed questions while preventing storage abuse. Most questions are 50-150 chars.
pub const MAX_QUESTION_LENGTH: u32 = 500;

/// Maximum length for outcome labels (100 characters)
///
/// Rationale: Outcome labels should be short and descriptive. 100 characters is
/// generous for labels like "yes", "no", "under_50k", "Team A wins", etc.
pub const MAX_OUTCOME_LENGTH: u32 = 100;

/// Maximum length for oracle feed IDs (200 characters)
///
/// Rationale: Most feed IDs are short (e.g., "BTC/USD" = 7 chars). Pyth uses
/// 64-char hex strings. 200 chars provides headroom for future oracle formats.
pub const MAX_FEED_ID_LENGTH: u32 = 200;

/// Maximum length for comparison operators (10 characters)
///
/// Rationale: Valid operators are "gt", "lt", "eq" (2-3 chars). 10 chars allows
/// for future operators like "gte", "lte", "between" while preventing abuse.
pub const MAX_COMPARISON_LENGTH: u32 = 10;

/// Maximum length for category strings (50 characters)
///
/// Rationale: Categories like "sports", "crypto", "politics" are typically short.
/// 50 chars allows for descriptive categories like "cryptocurrency-price-predictions".
pub const MAX_CATEGORY_LENGTH: u32 = 50;

/// Maximum length for individual tag strings (30 characters)
///
/// Rationale: Tags should be concise keywords. 30 chars accommodates tags like
/// "bitcoin", "price-prediction", "short-term" while preventing abuse.
pub const MAX_TAG_LENGTH: u32 = 30;

/// Maximum length for extension reason text (300 characters)
///
/// Rationale: Extension reasons should explain the justification. 300 chars allows
/// for detailed explanations like "Low participation detected, extending to allow
/// more users to participate and ensure fair market resolution."
pub const MAX_EXTENSION_REASON_LENGTH: u32 = 300;

/// Maximum length for oracle source identifiers (100 characters)
///
/// Rationale: Source identifiers like "reflector-mainnet" or oracle URLs should
/// be reasonably short. 100 chars accommodates most identifier formats.
pub const MAX_SOURCE_LENGTH: u32 = 100;

/// Maximum length for error messages (200 characters)
///
/// Rationale: Error messages should be informative but concise. 200 chars allows
/// for detailed error descriptions without excessive storage costs.
pub const MAX_ERROR_MESSAGE_LENGTH: u32 = 200;

/// Maximum length for signature strings (500 characters)
///
/// Rationale: Cryptographic signatures can be lengthy when encoded. 500 chars
/// accommodates most signature formats including base64-encoded signatures.
pub const MAX_SIGNATURE_LENGTH: u32 = 500;

// ===== VECTOR LENGTH LIMITS =====

/// Maximum number of outcomes per market (20 outcomes)
///
/// Rationale: Most markets are binary (2 outcomes). Multiple choice markets rarely
/// need more than 5-10 options. 20 provides flexibility while preventing abuse.
pub const MAX_OUTCOMES_COUNT: u32 = 20;

/// Maximum number of tags per market (10 tags)
///
/// Rationale: Tags are for categorization and filtering. 10 tags is generous for
/// most use cases (e.g., "bitcoin", "crypto", "price", "short-term", "volatile").
pub const MAX_TAGS_COUNT: u32 = 10;

/// Maximum number of extension history entries (50 extensions)
///
/// Rationale: Markets should not be extended indefinitely. 50 extensions is
/// extremely generous and prevents unbounded growth of extension history.
pub const MAX_EXTENSION_HISTORY_COUNT: u32 = 50;

/// Maximum number of individual oracle results in multi-oracle aggregation (10 oracles)
///
/// Rationale: Multi-oracle consensus typically uses 3-5 sources. 10 provides
/// headroom for high-security markets while preventing storage bloat.
pub const MAX_ORACLE_RESULTS_COUNT: u32 = 10;

/// Maximum number of winning outcomes (10 outcomes)
///
/// Rationale: In tie scenarios, multiple outcomes can win. 10 is generous for
/// most tie-breaking scenarios while preventing abuse.
pub const MAX_WINNING_OUTCOMES_COUNT: u32 = 10;

// ===== VALIDATION FUNCTIONS =====

/// Validates that a question string is within the maximum allowed length.
///
/// # Arguments
///
/// * `question` - The market question to validate
///
/// # Returns
///
/// * `Ok(())` if the question length is valid
/// * `Err(Error::QuestionTooLong)` if the question exceeds MAX_QUESTION_LENGTH
///
/// # Example
///
/// ```rust
/// let question = String::from_str(&env, "Will BTC reach $100k?");
/// validate_question_length(&question)?;
/// ```
pub fn validate_question_length(question: &String) -> Result<(), crate::Error> {
    if question.len() > MAX_QUESTION_LENGTH {
        return Err(crate::Error::QuestionTooLong);
    }
    Ok(())
}

/// Validates that an outcome string is within the maximum allowed length.
///
/// # Arguments
///
/// * `outcome` - The outcome label to validate
///
/// # Returns
///
/// * `Ok(())` if the outcome length is valid
/// * `Err(Error::OutcomeTooLong)` if the outcome exceeds MAX_OUTCOME_LENGTH
pub fn validate_outcome_length(outcome: &String) -> Result<(), crate::Error> {
    if outcome.len() > MAX_OUTCOME_LENGTH {
        return Err(crate::Error::OutcomeTooLong);
    }
    Ok(())
}

/// Validates that all outcomes in a vector are within the maximum allowed length.
///
/// # Arguments
///
/// * `outcomes` - Vector of outcome labels to validate
///
/// # Returns
///
/// * `Ok(())` if all outcomes are valid length
/// * `Err(Error::OutcomeTooLong)` if any outcome exceeds MAX_OUTCOME_LENGTH
pub fn validate_outcomes_length(outcomes: &Vec<String>) -> Result<(), crate::Error> {
    for i in 0..outcomes.len() {
        validate_outcome_length(&outcomes.get(i).unwrap())?;
    }
    Ok(())
}

/// Validates that the number of outcomes is within the maximum allowed count.
///
/// # Arguments
///
/// * `outcomes` - Vector of outcomes to validate
///
/// # Returns
///
/// * `Ok(())` if the count is valid
/// * `Err(Error::TooManyOutcomes)` if the count exceeds MAX_OUTCOMES_COUNT
pub fn validate_outcomes_count(outcomes: &Vec<String>) -> Result<(), crate::Error> {
    if outcomes.len() > MAX_OUTCOMES_COUNT {
        return Err(crate::Error::TooManyOutcomes);
    }
    Ok(())
}

/// Validates that a feed ID string is within the maximum allowed length.
///
/// # Arguments
///
/// * `feed_id` - The oracle feed ID to validate
///
/// # Returns
///
/// * `Ok(())` if the feed ID length is valid
/// * `Err(Error::FeedIdTooLong)` if the feed ID exceeds MAX_FEED_ID_LENGTH
pub fn validate_feed_id_length(feed_id: &String) -> Result<(), crate::Error> {
    if feed_id.len() > MAX_FEED_ID_LENGTH {
        return Err(crate::Error::FeedIdTooLong);
    }
    Ok(())
}

/// Validates that a comparison operator string is within the maximum allowed length.
///
/// # Arguments
///
/// * `comparison` - The comparison operator to validate
///
/// # Returns
///
/// * `Ok(())` if the comparison length is valid
/// * `Err(Error::ComparisonTooLong)` if the comparison exceeds MAX_COMPARISON_LENGTH
pub fn validate_comparison_length(comparison: &String) -> Result<(), crate::Error> {
    if comparison.len() > MAX_COMPARISON_LENGTH {
        return Err(crate::Error::ComparisonTooLong);
    }
    Ok(())
}

/// Validates that a category string is within the maximum allowed length.
///
/// # Arguments
///
/// * `category` - The category string to validate
///
/// # Returns
///
/// * `Ok(())` if the category length is valid
/// * `Err(Error::CategoryTooLong)` if the category exceeds MAX_CATEGORY_LENGTH
pub fn validate_category_length(category: &String) -> Result<(), crate::Error> {
    if category.len() > MAX_CATEGORY_LENGTH {
        return Err(crate::Error::CategoryTooLong);
    }
    Ok(())
}

/// Validates that a tag string is within the maximum allowed length.
///
/// # Arguments
///
/// * `tag` - The tag string to validate
///
/// # Returns
///
/// * `Ok(())` if the tag length is valid
/// * `Err(Error::TagTooLong)` if the tag exceeds MAX_TAG_LENGTH
pub fn validate_tag_length(tag: &String) -> Result<(), crate::Error> {
    if tag.len() > MAX_TAG_LENGTH {
        return Err(crate::Error::TagTooLong);
    }
    Ok(())
}

/// Validates that all tags in a vector are within the maximum allowed length.
///
/// # Arguments
///
/// * `tags` - Vector of tags to validate
///
/// # Returns
///
/// * `Ok(())` if all tags are valid length
/// * `Err(Error::TagTooLong)` if any tag exceeds MAX_TAG_LENGTH
pub fn validate_tags_length(tags: &Vec<String>) -> Result<(), crate::Error> {
    for i in 0..tags.len() {
        validate_tag_length(&tags.get(i).unwrap())?;
    }
    Ok(())
}

/// Validates that the number of tags is within the maximum allowed count.
///
/// # Arguments
///
/// * `tags` - Vector of tags to validate
///
/// # Returns
///
/// * `Ok(())` if the count is valid
/// * `Err(Error::TooManyTags)` if the count exceeds MAX_TAGS_COUNT
pub fn validate_tags_count(tags: &Vec<String>) -> Result<(), crate::Error> {
    if tags.len() > MAX_TAGS_COUNT {
        return Err(crate::Error::TooManyTags);
    }
    Ok(())
}

/// Validates that an extension reason string is within the maximum allowed length.
///
/// # Arguments
///
/// * `reason` - The extension reason to validate
///
/// # Returns
///
/// * `Ok(())` if the reason length is valid
/// * `Err(Error::ExtensionReasonTooLong)` if the reason exceeds MAX_EXTENSION_REASON_LENGTH
pub fn validate_extension_reason_length(reason: &String) -> Result<(), crate::Error> {
    if reason.len() > MAX_EXTENSION_REASON_LENGTH {
        return Err(crate::Error::ExtensionReasonTooLong);
    }
    Ok(())
}

/// Validates that a source identifier string is within the maximum allowed length.
///
/// # Arguments
///
/// * `source` - The source identifier to validate
///
/// # Returns
///
/// * `Ok(())` if the source length is valid
/// * `Err(Error::SourceTooLong)` if the source exceeds MAX_SOURCE_LENGTH
pub fn validate_source_length(source: &String) -> Result<(), crate::Error> {
    if source.len() > MAX_SOURCE_LENGTH {
        return Err(crate::Error::SourceTooLong);
    }
    Ok(())
}

/// Validates that an error message string is within the maximum allowed length.
///
/// # Arguments
///
/// * `error_message` - The error message to validate
///
/// # Returns
///
/// * `Ok(())` if the error message length is valid
/// * `Err(Error::ErrorMessageTooLong)` if the message exceeds MAX_ERROR_MESSAGE_LENGTH
pub fn validate_error_message_length(error_message: &String) -> Result<(), crate::Error> {
    if error_message.len() > MAX_ERROR_MESSAGE_LENGTH {
        return Err(crate::Error::ErrorMessageTooLong);
    }
    Ok(())
}

/// Validates that a signature string is within the maximum allowed length.
///
/// # Arguments
///
/// * `signature` - The signature string to validate
///
/// # Returns
///
/// * `Ok(())` if the signature length is valid
/// * `Err(Error::SignatureTooLong)` if the signature exceeds MAX_SIGNATURE_LENGTH
pub fn validate_signature_length(signature: &String) -> Result<(), crate::Error> {
    if signature.len() > MAX_SIGNATURE_LENGTH {
        return Err(crate::Error::SignatureTooLong);
    }
    Ok(())
}

/// Validates that the number of extension history entries is within the maximum allowed count.
///
/// # Arguments
///
/// * `count` - Number of extension history entries
///
/// # Returns
///
/// * `Ok(())` if the count is valid
/// * `Err(Error::TooManyExtensions)` if the count exceeds MAX_EXTENSION_HISTORY_COUNT
pub fn validate_extension_history_count(count: u32) -> Result<(), crate::Error> {
    if count > MAX_EXTENSION_HISTORY_COUNT {
        return Err(crate::Error::TooManyExtensions);
    }
    Ok(())
}

/// Validates that the number of oracle results is within the maximum allowed count.
///
/// # Arguments
///
/// * `count` - Number of oracle results
///
/// # Returns
///
/// * `Ok(())` if the count is valid
/// * `Err(Error::TooManyOracleResults)` if the count exceeds MAX_ORACLE_RESULTS_COUNT
pub fn validate_oracle_results_count(count: u32) -> Result<(), crate::Error> {
    if count > MAX_ORACLE_RESULTS_COUNT {
        return Err(crate::Error::TooManyOracleResults);
    }
    Ok(())
}

/// Validates that the number of winning outcomes is within the maximum allowed count.
///
/// # Arguments
///
/// * `winning_outcomes` - Vector of winning outcomes
///
/// # Returns
///
/// * `Ok(())` if the count is valid
/// * `Err(Error::TooManyWinningOutcomes)` if the count exceeds MAX_WINNING_OUTCOMES_COUNT
pub fn validate_winning_outcomes_count(winning_outcomes: &Vec<String>) -> Result<(), crate::Error> {
    if winning_outcomes.len() > MAX_WINNING_OUTCOMES_COUNT {
        return Err(crate::Error::TooManyWinningOutcomes);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{Env, String, Vec};

    #[test]
    fn test_validate_question_length_valid() {
        let env = Env::default();
        let question = String::from_str(&env, "Will BTC reach $100k?");
        assert!(validate_question_length(&question).is_ok());
    }

    #[test]
    fn test_validate_question_length_too_long() {
        let env = Env::default();
        let long_question = String::from_str(&env, &"a".repeat(501));
        assert_eq!(
            validate_question_length(&long_question),
            Err(crate::Error::QuestionTooLong)
        );
    }

    #[test]
    fn test_validate_outcome_length_valid() {
        let env = Env::default();
        let outcome = String::from_str(&env, "yes");
        assert!(validate_outcome_length(&outcome).is_ok());
    }

    #[test]
    fn test_validate_outcomes_count_valid() {
        let env = Env::default();
        let outcomes = Vec::from_array(
            &env,
            [
                String::from_str(&env, "yes"),
                String::from_str(&env, "no"),
            ],
        );
        assert!(validate_outcomes_count(&outcomes).is_ok());
    }

    #[test]
    fn test_validate_tags_count_too_many() {
        let env = Env::default();
        let mut tags = Vec::new(&env);
        for i in 0..11 {
            tags.push_back(String::from_str(&env, &format!("tag{}", i)));
        }
        assert_eq!(
            validate_tags_count(&tags),
            Err(crate::Error::TooManyTags)
        );
    }
}
