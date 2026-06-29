/// Metadata length limits for controlling storage costs and preventing denial-of-service attacks.
///
/// This module defines maximum length constraints for strings and vectors used throughout
/// the Predictify Hybrid smart contract. These limits serve multiple purposes:
///
/// # Length semantics (Unicode scalar values, not UTF-8 bytes)
///
/// All string limits in this module are enforced on **Unicode scalar value count** (the same
/// notion as Rust's [`str::chars`]), **not** on [`String::len`] byte length. For example,
/// `"😀"` counts as **one** character toward the limit even though it occupies four UTF-8 bytes.
/// This matches the documented "N characters" limits and aligns with
/// [`crate::validation::CreationValidator`], which also counts `.chars()`.
///
/// Invalid UTF-8 and strings containing Unicode control characters (`char::is_control`) are
/// rejected with [`crate::Error::InvalidInput`].
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

// ===== CATEGORY / TAG LIMITS (canonical: `config`) =====
//
// These re-exports keep a single source of truth in `config` while preserving the
// `metadata_limits` API for integrators and audit review.

/// Maximum length of a market category name (from [`crate::config::MAX_CATEGORY_LENGTH`]).
pub const MAX_CATEGORY_LENGTH: u32 = crate::config::MAX_CATEGORY_LENGTH;
/// Minimum length of a market category when set (from [`crate::config::MIN_CATEGORY_LENGTH`]).
pub const MIN_CATEGORY_LENGTH: u32 = crate::config::MIN_CATEGORY_LENGTH;
/// Maximum length of a single tag (from [`crate::config::MAX_TAG_LENGTH`]).
pub const MAX_TAG_LENGTH: u32 = crate::config::MAX_TAG_LENGTH;
/// Minimum length of a single non-empty tag (from [`crate::config::MIN_TAG_LENGTH`]).
pub const MIN_TAG_LENGTH: u32 = crate::config::MIN_TAG_LENGTH;
/// Maximum number of tags per market (from [`crate::config::MAX_TAGS_PER_MARKET`]).
pub const MAX_TAGS_COUNT: u32 = crate::config::MAX_TAGS_PER_MARKET;

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

fn scan_metadata_text(value: &String) -> Result<(u32, bool), crate::Error> {
    let byte_len = value.len() as usize;
    if byte_len == 0 {
        return Ok((0, false));
    }
    let mut bytes = alloc::vec![0u8; byte_len];
    value.copy_into_slice(&mut bytes);
    let text = core::str::from_utf8(&bytes).map_err(|_| crate::Error::InvalidInput)?;
    let mut char_count = 0u32;
    let mut has_control = false;
    for c in text.chars() {
        char_count = char_count.saturating_add(1);
        if c.is_control() {
            has_control = true;
        }
    }
    Ok((char_count, has_control))
}

fn reject_control_characters(value: &String) -> Result<(), crate::Error> {
    let (_, has_control) = scan_metadata_text(value)?;
    if has_control {
        return Err(crate::Error::InvalidInput);
    }
    Ok(())
}

/// Validates that a question string is within the maximum allowed length.
///
/// Limits are measured in Unicode scalar values, not UTF-8 bytes.
pub fn validate_question_length(question: &String) -> Result<(), crate::Error> {
    let (len, has_control) = scan_metadata_text(question)?;
    if has_control {
        return Err(crate::Error::InvalidInput);
    }
    if len > MAX_QUESTION_LENGTH {
        return Err(crate::Error::QuestionTooLong);
    }
    Ok(())
}

/// Validates that an outcome string is within the maximum allowed length.
pub fn validate_outcome_length(outcome: &String) -> Result<(), crate::Error> {
    let (len, has_control) = scan_metadata_text(outcome)?;
    if has_control {
        return Err(crate::Error::InvalidInput);
    }
    if len > MAX_OUTCOME_LENGTH {
        return Err(crate::Error::OutcomeTooLong);
    }
    Ok(())
}

pub fn validate_outcomes_length(outcomes: &Vec<String>) -> Result<(), crate::Error> {
    for i in 0..outcomes.len() {
        let outcome = outcomes.get(i).ok_or(crate::Error::InvalidInput)?;
        validate_outcome_length(&outcome)?;
    }
    Ok(())
}

pub fn validate_outcomes_count(outcomes: &Vec<String>) -> Result<(), crate::Error> {
    if outcomes.len() > MAX_OUTCOMES_COUNT {
        return Err(crate::Error::TooManyOutcomes);
    }
    Ok(())
}

pub fn validate_feed_id_length(feed_id: &String) -> Result<(), crate::Error> {
    reject_control_characters(feed_id)?;
    let (len, _) = scan_metadata_text(feed_id)?;
    if len > MAX_FEED_ID_LENGTH {
        return Err(crate::Error::FeedIdTooLong);
    }
    Ok(())
}

pub fn validate_comparison_length(comparison: &String) -> Result<(), crate::Error> {
    reject_control_characters(comparison)?;
    let (len, _) = scan_metadata_text(comparison)?;
    if len > MAX_COMPARISON_LENGTH {
        return Err(crate::Error::ComparisonTooLong);
    }
    Ok(())
}

pub fn validate_category_length(category: &String) -> Result<(), crate::Error> {
    reject_control_characters(category)?;
    let (len, _) = scan_metadata_text(category)?;
    if len > MAX_CATEGORY_LENGTH {
        return Err(crate::Error::CategoryTooLong);
    }
    Ok(())
}

pub fn validate_category_metadata(category: &String) -> Result<(), crate::Error> {
    let (len, has_control) = scan_metadata_text(category)?;
    if has_control {
        return Err(crate::Error::InvalidInput);
    }
    if len < MIN_CATEGORY_LENGTH {
        return Err(crate::Error::CategoryTooShort);
    }
    if len > MAX_CATEGORY_LENGTH {
        return Err(crate::Error::CategoryTooLong);
    }
    Ok(())
}

pub fn validate_option_category_metadata(opt: &Option<String>) -> Result<(), crate::Error> {
    match opt {
        None => Ok(()),
        Some(s) if s.is_empty() => Err(crate::Error::InvalidInput),
        Some(s) => validate_category_metadata(s),
    }
}

pub fn validate_tag_length(tag: &String) -> Result<(), crate::Error> {
    reject_control_characters(tag)?;
    let (len, _) = scan_metadata_text(tag)?;
    if len > MAX_TAG_LENGTH {
        return Err(crate::Error::TagTooLong);
    }
    Ok(())
}

pub fn validate_tag_metadata(tag: &String) -> Result<(), crate::Error> {
    let (len, has_control) = scan_metadata_text(tag)?;
    if has_control {
        return Err(crate::Error::InvalidInput);
    }
    if len == 0 {
        return Err(crate::Error::InvalidInput);
    }
    if len < MIN_TAG_LENGTH {
        return Err(crate::Error::TagTooShort);
    }
    if len > MAX_TAG_LENGTH {
        return Err(crate::Error::TagTooLong);
    }
    Ok(())
}

pub fn validate_tags_length(tags: &Vec<String>) -> Result<(), crate::Error> {
    for i in 0..tags.len() {
        let tag = tags.get(i).ok_or(crate::Error::InvalidInput)?;
        validate_tag_length(&tag)?;
    }
    Ok(())
}

pub fn validate_tags_count(tags: &Vec<String>) -> Result<(), crate::Error> {
    if tags.len() > MAX_TAGS_COUNT {
        return Err(crate::Error::TooManyTags);
    }
    Ok(())
}

pub fn validate_event_tags(tags: &Vec<String>) -> Result<(), crate::Error> {
    validate_tags_count(tags)?;
    for i in 0..tags.len() {
        let tag = tags.get(i).ok_or(crate::Error::InvalidInput)?;
        validate_tag_metadata(&tag)?;
    }
    for i in 0..tags.len() {
        let left = tags.get(i).ok_or(crate::Error::InvalidInput)?;
        for j in (i + 1)..tags.len() {
            let right = tags.get(j).ok_or(crate::Error::InvalidInput)?;
            if left == right {
                return Err(crate::Error::InvalidInput);
            }
        }
    }
    Ok(())
}

pub fn validate_extension_reason_length(reason: &String) -> Result<(), crate::Error> {
    reject_control_characters(reason)?;
    let (len, _) = scan_metadata_text(reason)?;
    if len > MAX_EXTENSION_REASON_LENGTH {
        return Err(crate::Error::ExtensionReasonTooLong);
    }
    Ok(())
}

pub fn validate_source_length(source: &String) -> Result<(), crate::Error> {
    reject_control_characters(source)?;
    let (len, _) = scan_metadata_text(source)?;
    if len > MAX_SOURCE_LENGTH {
        return Err(crate::Error::SourceTooLong);
    }
    Ok(())
}

pub fn validate_error_message_length(error_message: &String) -> Result<(), crate::Error> {
    reject_control_characters(error_message)?;
    let (len, _) = scan_metadata_text(error_message)?;
    if len > MAX_ERROR_MESSAGE_LENGTH {
        return Err(crate::Error::ErrorMessageTooLong);
    }
    Ok(())
}

pub fn validate_signature_length(signature: &String) -> Result<(), crate::Error> {
    reject_control_characters(signature)?;
    let (len, _) = scan_metadata_text(signature)?;
    if len > MAX_SIGNATURE_LENGTH {
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
    use alloc::format;
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
            [String::from_str(&env, "yes"), String::from_str(&env, "no")],
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
        assert_eq!(validate_tags_count(&tags), Err(crate::Error::TooManyTags));
    }
}
