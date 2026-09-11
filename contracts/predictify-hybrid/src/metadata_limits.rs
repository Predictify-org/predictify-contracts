/// Metadata size and encoding limits for controlling storage costs and preventing denial-of-service attacks.
///
/// This module defines maximum size constraints in bytes and vector limits used throughout
/// the Predictify Hybrid smart contract.
///
/// # Length Semantics (UTF-8 Bytes)
///
/// All string limits in this module are measured and enforced in UTF-8 byte length
/// ([`String::len`]). This guarantees that storage allocations and serialized footprints
/// on the ledger are deterministically bounded, preventing byte inflation attacks where
/// multi-byte characters could consume excessive storage within a character-based limit.
///
/// Malformed UTF-8 byte sequences and strings containing Unicode control characters
/// (`char::is_control`) are deterministically rejected with [`crate::Error::InvalidInput`]
/// before any mutation occurs.
///
/// # Security Benefits
///
/// - DoS Prevention: Prevents attackers from creating markets with excessively large metadata
/// - Storage Cost Control: Caps storage requirements to predictable, manageable levels
/// - Gas Optimization: Ensures operations complete within reasonable gas budgets
/// - Data Integrity: Enforces strict bounds on user-provided data
///
/// # Design Principles
///
/// 1. Conservative Limits: Set to accommodate legitimate use cases while preventing abuse
/// 2. Auditor-Friendly: Clear, documented constants that are easy to review
/// 3. Upgrade Path: Limits can be adjusted in future contract versions if needed
/// 4. User Experience: Generous enough to not hinder normal usage patterns
use soroban_sdk::{String, Vec};

// ===== CATEGORY / TAG LIMITS (canonical: `config`) =====

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

// ===== BYTE LIMIT CONSTANTS =====

/// Maximum size in bytes for market question text (500 bytes).
pub const MAX_QUESTION_BYTES: u32 = 500;
/// Minimum size in bytes for market question text (10 bytes).
pub const MIN_QUESTION_BYTES: u32 = 10;

/// Maximum size in bytes for outcome labels (100 bytes).
pub const MAX_OUTCOME_BYTES: u32 = 100;
/// Minimum size in bytes for outcome labels (2 bytes).
pub const MIN_OUTCOME_BYTES: u32 = 2;

/// Maximum size in bytes for event description text (1000 bytes).
pub const MAX_DESCRIPTION_BYTES: u32 = 1000;
/// Minimum size in bytes for event description text (0 bytes).
pub const MIN_DESCRIPTION_BYTES: u32 = 0;

/// Maximum size in bytes for a single tag (50 bytes).
pub const MAX_TAG_BYTES: u32 = 50;
/// Minimum size in bytes for a single tag (2 bytes).
pub const MIN_TAG_BYTES: u32 = 2;

/// Maximum size in bytes for a market category name (100 bytes).
pub const MAX_CATEGORY_BYTES: u32 = 100;
/// Minimum size in bytes for a market category name (2 bytes).
pub const MIN_CATEGORY_BYTES: u32 = 2;

/// Maximum size in bytes for oracle feed IDs (200 bytes).
pub const MAX_FEED_ID_BYTES: u32 = 200;

/// Maximum size in bytes for comparison operators (10 bytes).
pub const MAX_COMPARISON_BYTES: u32 = 10;

/// Maximum size in bytes for extension reason text (300 bytes).
pub const MAX_EXTENSION_REASON_BYTES: u32 = 300;

/// Maximum size in bytes for oracle source identifiers (100 bytes).
pub const MAX_SOURCE_BYTES: u32 = 100;

/// Maximum size in bytes for error messages (200 bytes).
pub const MAX_ERROR_MESSAGE_BYTES: u32 = 200;

/// Maximum size in bytes for signature strings (500 bytes).
pub const MAX_SIGNATURE_BYTES: u32 = 500;

// ===== BACKWARD COMPATIBLE LENGTH ALIASES =====

/// Maximum length for market question text (alias to MAX_QUESTION_BYTES).
pub const MAX_QUESTION_LENGTH: u32 = MAX_QUESTION_BYTES;
/// Minimum length for market question text (alias to MIN_QUESTION_BYTES).
pub const MIN_QUESTION_LENGTH: u32 = MIN_QUESTION_BYTES;

/// Maximum length for outcome labels (alias to MAX_OUTCOME_BYTES).
pub const MAX_OUTCOME_LENGTH: u32 = MAX_OUTCOME_BYTES;
/// Minimum length for outcome labels (alias to MIN_OUTCOME_BYTES).
pub const MIN_OUTCOME_LENGTH: u32 = MIN_OUTCOME_BYTES;

/// Maximum length for oracle feed IDs (alias to MAX_FEED_ID_BYTES).
pub const MAX_FEED_ID_LENGTH: u32 = MAX_FEED_ID_BYTES;

/// Maximum length for comparison operators (alias to MAX_COMPARISON_BYTES).
pub const MAX_COMPARISON_LENGTH: u32 = MAX_COMPARISON_BYTES;

/// Maximum length for extension reason text (alias to MAX_EXTENSION_REASON_BYTES).
pub const MAX_EXTENSION_REASON_LENGTH: u32 = MAX_EXTENSION_REASON_BYTES;

/// Maximum length for oracle source identifiers (alias to MAX_SOURCE_BYTES).
pub const MAX_SOURCE_LENGTH: u32 = MAX_SOURCE_BYTES;

/// Maximum length for error messages (alias to MAX_ERROR_MESSAGE_BYTES).
pub const MAX_ERROR_MESSAGE_LENGTH: u32 = MAX_ERROR_MESSAGE_BYTES;

/// Maximum length for signature strings (alias to MAX_SIGNATURE_BYTES).
pub const MAX_SIGNATURE_LENGTH: u32 = MAX_SIGNATURE_BYTES;

/// Maximum length for event description text (alias to MAX_DESCRIPTION_BYTES).
pub const MAX_DESCRIPTION_LENGTH: u32 = MAX_DESCRIPTION_BYTES;
/// Minimum length for event description text (alias to MIN_DESCRIPTION_BYTES).
pub const MIN_DESCRIPTION_LENGTH: u32 = MIN_DESCRIPTION_BYTES;

// ===== VECTOR LENGTH LIMITS =====

/// Maximum number of outcomes per market (20 outcomes).
pub const MAX_OUTCOMES_COUNT: u32 = 20;

/// Maximum number of extension history entries (50 extensions).
pub const MAX_EXTENSION_HISTORY_COUNT: u32 = 50;

/// Maximum number of individual oracle results in multi-oracle aggregation (10 oracles).
pub const MAX_ORACLE_RESULTS_COUNT: u32 = 10;

/// Maximum number of winning outcomes (10 outcomes).
pub const MAX_WINNING_OUTCOMES_COUNT: u32 = 10;

// ===== VALIDATION FUNCTIONS =====

fn scan_metadata_text(value: &String) -> Result<(u32, bool), crate::Error> {
    let byte_len = value.len();
    if byte_len == 0 {
        return Ok((0, false));
    }
    let mut bytes = alloc::vec![0u8; byte_len as usize];
    value.copy_into_slice(&mut bytes);
    let text = core::str::from_utf8(&bytes).map_err(|_| crate::Error::InvalidInput)?;
    let mut has_control = false;
    for c in text.chars() {
        if c.is_control() {
            has_control = true;
            break;
        }
    }
    Ok((byte_len, has_control))
}

fn reject_control_characters(value: &String) -> Result<(), crate::Error> {
    let (_, has_control) = scan_metadata_text(value)?;
    if has_control {
        return Err(crate::Error::InvalidInput);
    }
    Ok(())
}

/// Validates that a question string is within the maximum allowed byte length and contains valid UTF-8.
pub fn validate_question_length(question: &String) -> Result<(), crate::Error> {
    let (byte_len, has_control) = scan_metadata_text(question)?;
    if has_control {
        return Err(crate::Error::InvalidInput);
    }
    if byte_len > MAX_QUESTION_BYTES {
        return Err(crate::Error::QuestionTooLong);
    }
    Ok(())
}

/// Validates that an outcome string is within the maximum allowed byte length and contains valid UTF-8.
pub fn validate_outcome_length(outcome: &String) -> Result<(), crate::Error> {
    let (byte_len, has_control) = scan_metadata_text(outcome)?;
    if has_control {
        return Err(crate::Error::InvalidInput);
    }
    if byte_len > MAX_OUTCOME_BYTES {
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
    let (byte_len, has_control) = scan_metadata_text(feed_id)?;
    if has_control {
        return Err(crate::Error::InvalidInput);
    }
    if byte_len > MAX_FEED_ID_BYTES {
        return Err(crate::Error::FeedIdTooLong);
    }
    Ok(())
}

pub fn validate_comparison_length(comparison: &String) -> Result<(), crate::Error> {
    let (byte_len, has_control) = scan_metadata_text(comparison)?;
    if has_control {
        return Err(crate::Error::InvalidInput);
    }
    if byte_len > MAX_COMPARISON_BYTES {
        return Err(crate::Error::ComparisonTooLong);
    }
    Ok(())
}

pub fn validate_category_length(category: &String) -> Result<(), crate::Error> {
    let (byte_len, has_control) = scan_metadata_text(category)?;
    if has_control {
        return Err(crate::Error::InvalidInput);
    }
    if byte_len > MAX_CATEGORY_BYTES {
        return Err(crate::Error::CategoryTooLong);
    }
    Ok(())
}

pub fn validate_category_metadata(category: &String) -> Result<(), crate::Error> {
    let (byte_len, has_control) = scan_metadata_text(category)?;
    if has_control {
        return Err(crate::Error::InvalidInput);
    }
    if byte_len < MIN_CATEGORY_BYTES {
        return Err(crate::Error::CategoryTooShort);
    }
    if byte_len > MAX_CATEGORY_BYTES {
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
    let (byte_len, has_control) = scan_metadata_text(tag)?;
    if has_control {
        return Err(crate::Error::InvalidInput);
    }
    if byte_len > MAX_TAG_BYTES {
        return Err(crate::Error::TagTooLong);
    }
    Ok(())
}

pub fn validate_tag_metadata(tag: &String) -> Result<(), crate::Error> {
    let (byte_len, has_control) = scan_metadata_text(tag)?;
    if has_control {
        return Err(crate::Error::InvalidInput);
    }
    if byte_len == 0 {
        return Err(crate::Error::InvalidInput);
    }
    if byte_len < MIN_TAG_BYTES {
        return Err(crate::Error::TagTooShort);
    }
    if byte_len > MAX_TAG_BYTES {
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
    let (byte_len, has_control) = scan_metadata_text(reason)?;
    if has_control {
        return Err(crate::Error::InvalidInput);
    }
    if byte_len > MAX_EXTENSION_REASON_BYTES {
        return Err(crate::Error::ExtensionReasonTooLong);
    }
    Ok(())
}

pub fn validate_source_length(source: &String) -> Result<(), crate::Error> {
    let (byte_len, has_control) = scan_metadata_text(source)?;
    if has_control {
        return Err(crate::Error::InvalidInput);
    }
    if byte_len > MAX_SOURCE_BYTES {
        return Err(crate::Error::SourceTooLong);
    }
    Ok(())
}

pub fn validate_error_message_length(error_message: &String) -> Result<(), crate::Error> {
    let (byte_len, has_control) = scan_metadata_text(error_message)?;
    if has_control {
        return Err(crate::Error::InvalidInput);
    }
    if byte_len > MAX_ERROR_MESSAGE_BYTES {
        return Err(crate::Error::ErrorMessageTooLong);
    }
    Ok(())
}

pub fn validate_signature_length(signature: &String) -> Result<(), crate::Error> {
    let (byte_len, has_control) = scan_metadata_text(signature)?;
    if has_control {
        return Err(crate::Error::InvalidInput);
    }
    if byte_len > MAX_SIGNATURE_BYTES {
        return Err(crate::Error::SignatureTooLong);
    }
    Ok(())
}

pub fn validate_description_length(description: &String) -> Result<(), crate::Error> {
    let (byte_len, has_control) = scan_metadata_text(description)?;
    if has_control {
        return Err(crate::Error::InvalidInput);
    }
    if byte_len > MAX_DESCRIPTION_BYTES {
        return Err(crate::Error::InvalidQuestion);
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
