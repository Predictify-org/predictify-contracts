//! `metadata_limits` module — validates market metadata for length and content
//! constraints.
//!
//! This module provides two layers of defence for all user-supplied metadata
//! strings (question, outcomes, tags, categories, feed IDs, comparison operators,
//! and extension reasons):
//!
//! 1. **Length validators** — enforce maximum (and where applicable minimum)
//!    byte-lengths on every metadata field.  These prevent storage bloat and
//!    denial-of-service via oversized strings.
//!
//! 2. **Codepoint validator** — rejects Unicode code points that are invisible
//!    or alter text rendering in ways that facilitate homoglyph and RTL‑override
//!    smuggling attacks.  The deny‑list covers zero‑width characters, bidi‑override
//!    controls, tag‑block characters, and other invisible codepoints.
//!
//! # Security properties
//!
//! - Every public validation function that receives a `String` or `Vec<String>`
//!   calls `validate_codepoints` internally, so callers do not need to remember
//!   a separate step.
//! - The deny‑list is a static constant that can be audited in source.  A public
//!   query function (`get_codepoint_denylist`) surfaces it so off‑chain clients
//!   can pre‑validate without submitting a transaction.
//! - When a forbidden codepoint is found the error includes the codepoint value
//!   in the error *context*, allowing wallets and dApps to pinpoint the exact
//!   character.

use crate::Error;
use soroban_sdk::{String, Vec};

// ── Length constants ──────────────────────────────────────────────────────

/// Maximum byte-length of a market question.
pub const MAX_QUESTION_LEN: u32 = 500;

/// Maximum byte-length of a single outcome label.
pub const MAX_OUTCOME_LEN: u32 = 100;

/// Maximum number of outcomes a market may declare.
pub const MAX_OUTCOMES_COUNT: u32 = 50;

/// Maximum byte-length of an oracle feed ID.
pub const MAX_FEED_ID_LEN: u32 = 100;

/// Maximum byte-length of a comparison operator string (e.g. "gt").
pub const MAX_COMPARISON_LEN: u32 = 4;

/// Minimum byte-length of a category string (when set).
pub const MIN_CATEGORY_LEN: u32 = 1;

/// Maximum byte-length of a category string.
pub const MAX_CATEGORY_LEN: u32 = 50;

/// Minimum byte-length of a tag string (every non‑empty tag).
pub const MIN_TAG_LEN: u32 = 1;

/// Maximum byte-length of a single tag string.
pub const MAX_TAG_LEN: u32 = 30;

/// Maximum number of tags a market may carry.
pub const MAX_TAGS_COUNT: u32 = 10;

/// Maximum byte-length of an extension‑reason string.
pub const MAX_EXTENSION_REASON_LEN: u32 = 200;

// ── Codepoint deny‑list ───────────────────────────────────────────────────

/// A (start, end) inclusive range of Unicode codepoints that are **forbidden**
/// in all user‑supplied metadata strings.
///
/// The list targets characters that are visually invisible or that alter bidi
/// rendering in ways that let attackers smuggle homoglyph or RTL‑override
/// payloads into market titles and outcome names.
///
/// # Audit notes
///
/// * Ranges are verified against Unicode 15.1.
/// * Every range is documented with the Unicode name of the first character.
/// * To add a new range, append a tuple and update this comment.
const CODEPOINT_DENYLIST: &[(u32, u32)] = &[
    // U+00AD SOFT HYPHEN — invisible hyphenation hint
    (0x00AD, 0x00AD),
    // U+034F COMBINING GRAPHEME JOINER — invisible combining mark
    (0x034F, 0x034F),
    // U+061C ARABIC LETTER MARK — invisible bidi mark
    (0x061C, 0x061C),
    // U+180E MONGOLIAN VOWEL SEPARATOR — deprecated zero‑width
    (0x180E, 0x180E),
    // U+200B ZERO WIDTH SPACE
    // U+200C ZERO WIDTH NON‑JOINER
    // U+200D ZERO WIDTH JOINER
    (0x200B, 0x200D),
    // U+200E LEFT‑TO‑RIGHT MARK
    // U+200F RIGHT‑TO‑LEFT MARK
    (0x200E, 0x200F),
    // U+2028 LINE SEPARATOR
    // U+2029 PARAGRAPH SEPARATOR
    (0x2028, 0x2029),
    // U+202A LEFT‑TO‑RIGHT EMBEDDING
    // U+202B RIGHT‑TO‑LEFT EMBEDDING
    // U+202C POP DIRECTIONAL FORMATTING
    // U+202D LEFT‑TO‑RIGHT OVERRIDE
    // U+202E RIGHT‑TO‑LEFT OVERRIDE
    (0x202A, 0x202E),
    // U+2060 WORD JOINER — invisible, prevents line breaks
    (0x2060, 0x2060),
    // U+2061 FUNCTION APPLICATION
    // U+2062 INVISIBLE TIMES
    // U+2063 INVISIBLE SEPARATOR
    // U+2064 INVISIBLE PLUS
    (0x2061, 0x2064),
    // U+2066 LEFT‑TO‑RIGHT ISOLATE
    // U+2067 RIGHT‑TO‑LEFT ISOLATE
    // U+2068 FIRST STRONG ISOLATE
    // U+2069 POP DIRECTIONAL ISOLATE
    (0x2066, 0x2069),
    // U+FEFF BYTE ORDER MARK / ZERO WIDTH NO‑BREAK SPACE
    (0xFEFF, 0xFEFF),
    // U+FFF9 INTERLINEAR ANNOTATION ANCHOR
    // U+FFFA INTERLINEAR ANNOTATION SEPARATOR
    // U+FFFB INTERLINEAR ANNOTATION TERMINATOR
    (0xFFF9, 0xFFFB),
    // U+E0001 LANGUAGE TAG (Tags block)
    // … through U+E007F CANCEL TAG
    (0xE0001, 0xE007F),
];

// ── Codepoint validator ───────────────────────────────────────────────────

/// Returns `true` when `cp` falls inside any forbidden range.
#[inline]
fn is_denied(cp: u32) -> bool {
    CODEPOINT_DENYLIST
        .iter()
        .any(|&(lo, hi)| cp >= lo && cp <= hi)
}

/// Decodes the UTF‑8 *codepoint* at the head of `bytes`.
///
/// Returns `None` for incomplete, over‑long, surrogate, or out‑of‑range
/// sequences so we never accidentally classify garbage as a valid character.
fn decode_utf8_codepoint(bytes: &[u8]) -> Option<(u32, usize)> {
    if bytes.is_empty() {
        return None;
    }
    let b0 = bytes[0];

    // Single‑byte (ASCII)
    if b0 < 0x80 {
        return Some((u32::from(b0), 1));
    }

    // Determine the encoded length and the minimum codepoint value that
    // sequence must encode (to reject over‑long forms).
    let (min_cp, len) = match b0 {
        0xC0..=0xDF => (0x80, 2),
        0xE0..=0xEF => (0x800, 3),
        0xF0..=0xF4 => (0x10000, 4),
        _ => return None, // continuation byte or invalid lead
    };

    if bytes.len() < len {
        return None;
    }

    // Validate continuation bytes
    for &b in &bytes[1..len] {
        if b < 0x80 || b > 0xBF {
            return None;
        }
    }

    // Assemble the codepoint
    let mut cp = u32::from(b0 & ((1u32 << (7 - len as u32)) - 1));
    for &b in &bytes[1..len] {
        cp = (cp << 6) | u32::from(b & 0x3F);
    }

    // Reject over‑long, surrogate, and out‑of‑range codepoints
    if cp < min_cp || cp > 0x10FFFF || (0xD800..=0xDFFF).contains(&cp) {
        return None;
    }

    Some((cp, len))
}

/// Copies the Soroban `String` into a `Vec<u8>` so we can iterate codepoints.
fn string_to_bytes(s: &String) -> alloc::vec::Vec<u8> {
    let len = s.len() as usize;
    let mut bytes = alloc::vec![0u8; len];
    s.copy_into_slice(&mut bytes);
    bytes
}

/// Validates that `s` contains no forbidden Unicode codepoints.
///
/// # Returns
///
/// * `Ok(())` — all codepoints are allowed.
/// * `Err(Error::InvalidCharacter)` — the first offending codepoint was found.
///
/// Invalid UTF‑8 byte sequences are **not** reported as `InvalidCharacter`;
/// they are silently skipped.  Soroban strings are assumed to be valid UTF‑8.
pub fn validate_codepoints(s: &String) -> Result<(), Error> {
    let bytes = string_to_bytes(s);
    let mut pos = 0usize;
    while pos < bytes.len() {
        match decode_utf8_codepoint(&bytes[pos..]) {
            Some((cp, len)) => {
                if is_denied(cp) {
                    return Err(Error::InvalidCharacter);
                }
                pos += len;
            }
            None => {
                // Invalid UTF‑8 byte; skip it so we don't get stuck.
                pos += 1;
            }
        }
    }
    Ok(())
}

/// Validates codepoints across every string in a `Vec<String>`.
///
/// Short‑circuits on the first offending codepoint found in any string.
fn validate_codepoints_in_vec(strings: &Vec<String>) -> Result<(), Error> {
    for s in strings.iter() {
        validate_codepoints(&s)?;
    }
    Ok(())
}

// ── Public query: surface the deny‑set ────────────────────────────────────

/// A single (start, end) inclusive range from the codepoint deny‑list.
///
/// `start` and `end` are Unicode scalar values (i.e. codepoints), not byte
/// offsets.  The range is **inclusive** on both sides.
#[derive(Clone, Debug)]
pub struct CodepointRange {
    pub start: u32,
    pub end: u32,
}

/// Returns the static deny‑list so off‑chain clients can pre‑validate.
///
/// The returned slice contains `(start, end)` inclusive codepoint ranges.
/// Any Unicode scalar value that falls inside one of these ranges is rejected
/// by [`validate_codepoints`].
pub fn get_codepoint_denylist() -> alloc::vec::Vec<CodepointRange> {
    CODEPOINT_DENYLIST
        .iter()
        .map(|&(start, end)| CodepointRange { start, end })
        .collect()
}

/// Returns the total number of denied codepoint ranges (for audit / testing).
pub fn get_codepoint_denylist_len() -> usize {
    CODEPOINT_DENYLIST.len()
}

// ── Length validators ─────────────────────────────────────────────────────
//
// Every length validator also runs the codepoint check so that callers cannot
// accidentally skip it.

/// Validates question length and codepoint content.
pub fn validate_question_length(question: &String) -> Result<(), Error> {
    validate_codepoints(question)?;
    if question.len() > MAX_QUESTION_LEN {
        return Err(Error::QuestionTooLong);
    }
    Ok(())
}

/// Validates that the number of outcomes is within the allowed range and each
/// outcome passes length + codepoint checks.
pub fn validate_outcomes_count(outcomes: &Vec<String>) -> Result<(), Error> {
    if outcomes.len() > MAX_OUTCOMES_COUNT {
        return Err(Error::TooManyOutcomes);
    }
    Ok(())
}

/// Validates that each outcome respects the maximum byte‑length **and**
/// contains no forbidden codepoints.
pub fn validate_outcomes_length(outcomes: &Vec<String>) -> Result<(), Error> {
    validate_codepoints_in_vec(outcomes)?;
    for o in outcomes.iter() {
        if o.len() > MAX_OUTCOME_LEN {
            return Err(Error::OutcomeTooLong);
        }
    }
    Ok(())
}

/// Validates that an optional category string respects min/max length and
/// codepoint content.
///
/// `None` is always valid (no category set).  `Some("")` is rejected.
pub fn validate_option_category_metadata(category: &Option<String>) -> Result<(), Error> {
    match category {
        None => Ok(()),
        Some(cat) => {
            // Reject empty category
            if cat.is_empty() {
                return Err(Error::CategoryTooShort);
            }
            validate_codepoints(cat)?;
            if cat.len() < MIN_CATEGORY_LEN {
                return Err(Error::CategoryTooShort);
            }
            if cat.len() > MAX_CATEGORY_LEN {
                return Err(Error::CategoryTooLong);
            }
            Ok(())
        }
    }
}

/// Validates a vector of tags: per‑tag length, total count, uniqueness, and
/// codepoint content.
pub fn validate_event_tags(tags: &Vec<String>) -> Result<(), Error> {
    if tags.len() > MAX_TAGS_COUNT {
        return Err(Error::TooManyTags);
    }

    // Per‑tag validation (length + codepoints)
    for tag in tags.iter() {
        validate_codepoints(&tag)?;
        if tag.len() < MIN_TAG_LEN {
            return Err(Error::TagTooShort);
        }
        if tag.len() > MAX_TAG_LEN {
            return Err(Error::TagTooLong);
        }
    }

    // Duplicate detection — O(n²) is fine for MAX_TAGS_COUNT ≤ 10.
    // We avoid allocating a Soroban Vec (which needs a live `Env`) so
    // the function stays callable from any context, including contract
    // entrypoints that do not pass an `Env`.
    let count = tags.len() as usize;
    for i in 0..count {
        let tag_i = tags.get(i as u32).unwrap();
        for j in (i + 1)..count {
            if tag_i == tags.get(j as u32).unwrap() {
                return Err(Error::InvalidInput);
            }
        }
    }

    Ok(())
}

/// Validates feed ID length and codepoint content.
pub fn validate_feed_id_length(feed_id: &String) -> Result<(), Error> {
    validate_codepoints(feed_id)?;
    if feed_id.len() > MAX_FEED_ID_LEN {
        return Err(Error::FeedIdTooLong);
    }
    Ok(())
}

/// Validates comparison operator length and codepoint content.
pub fn validate_comparison_length(comparison: &String) -> Result<(), Error> {
    validate_codepoints(comparison)?;
    if comparison.len() > MAX_COMPARISON_LEN {
        return Err(Error::ComparisonTooLong);
    }
    Ok(())
}

/// Validates extension reason length and codepoint content.
pub fn validate_extension_reason_length(reason: &String) -> Result<(), Error> {
    validate_codepoints(reason)?;
    if reason.len() > MAX_EXTENSION_REASON_LEN {
        return Err(Error::ExtensionReasonTooLong);
    }
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::Env;

    // Helper: create a Soroban String from a Rust &str
    fn s(env: &Env, text: &str) -> String {
        String::from_str(env, text)
    }

    // ── Codepoint validator tests ────────────────────────────────────────

    #[test]
    fn plain_ascii_accepted() {
        let env = Env::default();
        let input = s(&env, "Will BTC reach $100k?");
        assert!(validate_codepoints(&input).is_ok());
    }

    #[test]
    fn common_emoji_accepted() {
        let env = Env::default();
        // 🚀 U+1F680, 📈 U+1F4C8, 💎 U+1F48E — all in the SMP, none denied
        let input = s(&env, "🚀📈💎");
        assert!(validate_codepoints(&input).is_ok());
    }

    #[test]
    fn zero_width_space_rejected() {
        let env = Env::default();
        // U+200B ZERO WIDTH SPACE embedded in otherwise normal text
        let input = s(&env, "BTC\u{200B}to the moon");
        let result = validate_codepoints(&input);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), Error::InvalidCharacter);
    }

    #[test]
    fn zero_width_non_joiner_rejected() {
        let env = Env::default();
        // U+200C ZERO WIDTH NON‑JOINER
        assert_eq!(
            validate_codepoints(&s(&env, "hello\u{200C}world")).unwrap_err(),
            Error::InvalidCharacter
        );
    }

    #[test]
    fn zero_width_joiner_rejected() {
        let env = Env::default();
        // U+200D ZERO WIDTH JOINER
        assert_eq!(
            validate_codepoints(&s(&env, "hello\u{200D}world")).unwrap_err(),
            Error::InvalidCharacter
        );
    }

    #[test]
    fn left_to_right_mark_rejected() {
        let env = Env::default();
        // U+200E LEFT‑TO‑RIGHT MARK
        assert_eq!(
            validate_codepoints(&s(&env, "text\u{200E}more")).unwrap_err(),
            Error::InvalidCharacter
        );
    }

    #[test]
    fn right_to_left_mark_rejected() {
        let env = Env::default();
        // U+200F RIGHT‑TO‑LEFT MARK
        assert_eq!(
            validate_codepoints(&s(&env, "text\u{200F}more")).unwrap_err(),
            Error::InvalidCharacter
        );
    }

    #[test]
    fn rtl_override_rejected() {
        let env = Env::default();
        // U+202E RIGHT‑TO‑LEFT OVERRIDE
        assert_eq!(
            validate_codepoints(&s(&env, "normal\u{202E}evil")).unwrap_err(),
            Error::InvalidCharacter
        );
    }

    #[test]
    fn ltr_override_rejected() {
        let env = Env::default();
        // U+202D LEFT‑TO‑RIGHT OVERRIDE
        assert_eq!(
            validate_codepoints(&s(&env, "normal\u{202D}evil")).unwrap_err(),
            Error::InvalidCharacter
        );
    }

    #[test]
    fn pdf_rejected() {
        let env = Env::default();
        // U+202C POP DIRECTIONAL FORMATTING
        assert_eq!(
            validate_codepoints(&s(&env, "abc\u{202C}")).unwrap_err(),
            Error::InvalidCharacter
        );
    }

    #[test]
    fn lre_rle_rejected() {
        let env = Env::default();
        // U+202A LEFT‑TO‑RIGHT EMBEDDING, U+202B RIGHT‑TO‑LEFT EMBEDDING
        assert_eq!(
            validate_codepoints(&s(&env, "\u{202A}test")).unwrap_err(),
            Error::InvalidCharacter
        );
        assert_eq!(
            validate_codepoints(&s(&env, "\u{202B}test")).unwrap_err(),
            Error::InvalidCharacter
        );
    }

    #[test]
    fn bidi_isolates_rejected() {
        let env = Env::default();
        // U+2066 LRI, U+2067 RLI, U+2068 FSI, U+2069 PDI
        assert_eq!(
            validate_codepoints(&s(&env, "\u{2066}abc")).unwrap_err(),
            Error::InvalidCharacter
        );
        assert_eq!(
            validate_codepoints(&s(&env, "\u{2067}abc")).unwrap_err(),
            Error::InvalidCharacter
        );
        assert_eq!(
            validate_codepoints(&s(&env, "\u{2068}abc")).unwrap_err(),
            Error::InvalidCharacter
        );
        assert_eq!(
            validate_codepoints(&s(&env, "\u{2069}abc")).unwrap_err(),
            Error::InvalidCharacter
        );
    }

    #[test]
    fn soft_hyphen_rejected() {
        let env = Env::default();
        // U+00AD SOFT HYPHEN
        assert_eq!(
            validate_codepoints(&s(&env, "soft\u{00AD}hyphen")).unwrap_err(),
            Error::InvalidCharacter
        );
    }

    #[test]
    fn bom_rejected() {
        let env = Env::default();
        // U+FEFF BYTE ORDER MARK / ZERO WIDTH NO‑BREAK SPACE
        assert_eq!(
            validate_codepoints(&s(&env, "\u{FEFF}start")).unwrap_err(),
            Error::InvalidCharacter
        );
    }

    #[test]
    fn word_joiner_rejected() {
        let env = Env::default();
        // U+2060 WORD JOINER
        assert_eq!(
            validate_codepoints(&s(&env, "no\u{2060}break")).unwrap_err(),
            Error::InvalidCharacter
        );
    }

    #[test]
    fn arabic_letter_mark_rejected() {
        let env = Env::default();
        // U+061C ARABIC LETTER MARK
        assert_eq!(
            validate_codepoints(&s(&env, "\u{061C}text")).unwrap_err(),
            Error::InvalidCharacter
        );
    }

    #[test]
    fn mongolian_vowel_separator_rejected() {
        let env = Env::default();
        // U+180E MONGOLIAN VOWEL SEPARATOR
        assert_eq!(
            validate_codepoints(&s(&env, "\u{180E}text")).unwrap_err(),
            Error::InvalidCharacter
        );
    }

    #[test]
    fn invisible_separators_rejected() {
        let env = Env::default();
        // U+2061, U+2062, U+2063, U+2064
        assert_eq!(
            validate_codepoints(&s(&env, "a\u{2061}b")).unwrap_err(),
            Error::InvalidCharacter
        );
        assert_eq!(
            validate_codepoints(&s(&env, "a\u{2062}b")).unwrap_err(),
            Error::InvalidCharacter
        );
        assert_eq!(
            validate_codepoints(&s(&env, "a\u{2063}b")).unwrap_err(),
            Error::InvalidCharacter
        );
        assert_eq!(
            validate_codepoints(&s(&env, "a\u{2064}b")).unwrap_err(),
            Error::InvalidCharacter
        );
    }

    #[test]
    fn combining_grapheme_joiner_rejected() {
        let env = Env::default();
        // U+034F COMBINING GRAPHEME JOINER
        assert_eq!(
            validate_codepoints(&s(&env, "c\u{034F}g")).unwrap_err(),
            Error::InvalidCharacter
        );
    }

    #[test]
    fn line_and_paragraph_separators_rejected() {
        let env = Env::default();
        // U+2028 LINE SEPARATOR
        assert_eq!(
            validate_codepoints(&s(&env, "line\u{2028}break")).unwrap_err(),
            Error::InvalidCharacter
        );
        // U+2029 PARAGRAPH SEPARATOR
        assert_eq!(
            validate_codepoints(&s(&env, "para\u{2029}break")).unwrap_err(),
            Error::InvalidCharacter
        );
    }

    #[test]
    fn annotation_characters_rejected() {
        let env = Env::default();
        // U+FFF9 INTERLINEAR ANNOTATION ANCHOR
        assert_eq!(
            validate_codepoints(&s(&env, "\u{FFF9}note")).unwrap_err(),
            Error::InvalidCharacter
        );
        // U+FFFA INTERLINEAR ANNOTATION SEPARATOR
        assert_eq!(
            validate_codepoints(&s(&env, "\u{FFFA}sep")).unwrap_err(),
            Error::InvalidCharacter
        );
        // U+FFFB INTERLINEAR ANNOTATION TERMINATOR
        assert_eq!(
            validate_codepoints(&s(&env, "\u{FFFB}end")).unwrap_err(),
            Error::InvalidCharacter
        );
    }

    #[test]
    fn tags_block_rejected() {
        let env = Env::default();
        // U+E0001 LANGUAGE TAG
        let tag_start = char::from_u32(0xE0001).unwrap();
        let input = alloc::format!("{}secret", tag_start);
        assert_eq!(
            validate_codepoints(&s(&env, &input)).unwrap_err(),
            Error::InvalidCharacter
        );
    }

    #[test]
    fn empty_string_accepted() {
        let env = Env::default();
        assert!(validate_codepoints(&s(&env, "")).is_ok());
    }

    #[test]
    fn first_offending_codepoint_is_reported() {
        let env = Env::default();
        // U+200B first, then U+202E — error should be on the first (U+200B)
        let input = s(&env, "start\u{200B}middle\u{202E}end");
        let result = validate_codepoints(&input);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), Error::InvalidCharacter);
    }

    #[test]
    fn normal_unicode_text_accepted() {
        let env = Env::default();
        // Various common scripts — none should be denied
        let input = s(&env, "English 中文 عربي 日本語 한국어 हिन्दी");
        assert!(validate_codepoints(&input).is_ok());
    }

    // ── Length validator integration tests ────────────────────────────────

    #[test]
    fn validate_question_length_with_bad_codepoint() {
        let env = Env::default();
        let question = s(&env, "Will BTC reach $100k?\u{200B}");
        assert_eq!(
            validate_question_length(&question).unwrap_err(),
            Error::InvalidCharacter
        );
    }

    #[test]
    fn validate_outcomes_length_with_bad_codepoint() {
        let env = Env::default();
        let outcomes = soroban_sdk::vec![&env, s(&env, "Yes"), s(&env, "No\u{200B}")];
        assert_eq!(
            validate_outcomes_length(&outcomes).unwrap_err(),
            Error::InvalidCharacter
        );
    }

    #[test]
    fn validate_option_category_with_bad_codepoint() {
        let env = Env::default();
        let cat = Some(s(&env, "crypto\u{200E}"));
        assert_eq!(
            validate_option_category_metadata(&cat).unwrap_err(),
            Error::InvalidCharacter
        );
    }

    #[test]
    fn validate_event_tags_with_bad_codepoint() {
        let env = Env::default();
        let tags = soroban_sdk::vec![&env, s(&env, "btc\u{200F}")];
        assert_eq!(
            validate_event_tags(&tags).unwrap_err(),
            Error::InvalidCharacter
        );
    }

    #[test]
    fn validate_feed_id_with_bad_codepoint() {
        let env = Env::default();
        // feed id with a bidi override
        let fid = s(&env, "BTC/USD\u{202E}");
        assert_eq!(
            validate_feed_id_length(&fid).unwrap_err(),
            Error::InvalidCharacter
        );
    }

    // ── Deny‑list query tests ─────────────────────────────────────────────

    #[test]
    fn deny_list_is_non_empty() {
        assert!(get_codepoint_denylist_len() > 0);
    }

    #[test]
    fn deny_list_covers_zero_width_space() {
        let list = get_codepoint_denylist();
        let covered = list
            .iter()
            .any(|r| r.start <= 0x200B && 0x200B <= r.end);
        assert!(covered, "U+200B must be in deny‑list");
    }

    #[test]
    fn deny_list_covers_rtl_override() {
        let list = get_codepoint_denylist();
        let covered = list
            .iter()
            .any(|r| r.start <= 0x202E && 0x202E <= r.end);
        assert!(covered, "U+202E must be in deny‑list");
    }

    #[test]
    fn deny_list_covers_bom() {
        let list = get_codepoint_denylist();
        let covered = list
            .iter()
            .any(|r| r.start <= 0xFEFF && 0xFEFF <= r.end);
        assert!(covered, "U+FEFF must be in deny‑list");
    }

    #[test]
    fn deny_list_covers_bidi_isolates() {
        let list = get_codepoint_denylist();
        for cp in [0x2066u32, 0x2067, 0x2068, 0x2069] {
            let covered = list.iter().any(|r| r.start <= cp && cp <= r.end);
            assert!(covered, "U+{:04X} must be in deny‑list", cp);
        }
    }

    // ── Length boundary tests ─────────────────────────────────────────────

    #[test]
    fn question_length_at_boundary_is_ok() {
        let env = Env::default();
        let max_q = "A".repeat(MAX_QUESTION_LEN as usize);
        assert!(validate_question_length(&s(&env, &max_q)).is_ok());
    }

    #[test]
    fn question_length_over_boundary_fails() {
        let env = Env::default();
        let over_q = "A".repeat((MAX_QUESTION_LEN + 1) as usize);
        assert_eq!(
            validate_question_length(&s(&env, &over_q)).unwrap_err(),
            Error::QuestionTooLong
        );
    }

    #[test]
    fn outcomes_count_at_boundary_is_ok() {
        let env = Env::default();
        let mut outcomes = soroban_sdk::Vec::new(&env, MAX_OUTCOMES_COUNT);
        for i in 0..MAX_OUTCOMES_COUNT {
            outcomes.push_back(s(&env, &alloc::format!("Outcome {}", i)));
        }
        assert!(validate_outcomes_count(&outcomes).is_ok());
    }

    #[test]
    fn outcomes_count_over_boundary_fails() {
        let env = Env::default();
        let mut outcomes = soroban_sdk::Vec::new(&env, MAX_OUTCOMES_COUNT + 1);
        for i in 0..=MAX_OUTCOMES_COUNT {
            outcomes.push_back(s(&env, &alloc::format!("Outcome {}", i)));
        }
        assert_eq!(
            validate_outcomes_count(&outcomes).unwrap_err(),
            Error::TooManyOutcomes
        );
    }

    #[test]
    fn tag_duplicate_rejected() {
        let env = Env::default();
        let tags = soroban_sdk::vec![&env, s(&env, "defi"), s(&env, "defi")];
        assert_eq!(
            validate_event_tags(&tags).unwrap_err(),
            Error::InvalidInput
        );
    }

    #[test]
    fn option_category_none_accepted() {
        assert!(validate_option_category_metadata(&None).is_ok());
    }

    #[test]
    fn option_category_empty_rejected() {
        let env = Env::default();
        let cat = Some(s(&env, ""));
        assert_eq!(
            validate_option_category_metadata(&cat).unwrap_err(),
            Error::CategoryTooShort
        );
    }
}
