# Metadata Limits Quick Reference

## 📏 String Limits

```
Question:          500 characters
Outcome:           100 characters
Feed ID:           200 characters
Comparison:         10 characters
Category:           50 characters
Tag:                30 characters
Extension Reason:  300 characters
Source:            100 characters
Error Message:     200 characters
Signature:         500 characters
```

## 📦 Vector Limits

```
Outcomes:           20 items
Tags:               10 items
Extension History:  50 items
Oracle Results:     10 items
Winning Outcomes:   10 items
```

## ⚠️ Error Codes

```rust
QuestionTooLong = 420
OutcomeTooLong = 421
TooManyOutcomes = 422
FeedIdTooLong = 423
ComparisonTooLong = 424
CategoryTooLong = 425
TagTooLong = 426
TooManyTags = 427
ExtensionReasonTooLong = 428
SourceTooLong = 429
ErrorMessageTooLong = 430
SignatureTooLong = 431
TooManyExtensions = 432
TooManyOracleResults = 433
TooManyWinningOutcomes = 434
```

## 🔧 Usage

### Rust (Contract)

```rust
use predictify_hybrid::metadata_limits::*;

// Validate question
validate_question_length(&question)?;

// Validate outcomes
validate_outcomes_count(&outcomes)?;
validate_outcomes_length(&outcomes)?;

// Validate tags
validate_tags_count(&tags)?;
validate_tags_length(&tags)?;
```

### JavaScript (Frontend)

```javascript
const LIMITS = {
  MAX_QUESTION_LENGTH: 500,
  MAX_OUTCOME_LENGTH: 100,
  MAX_OUTCOMES_COUNT: 20,
  MAX_TAG_LENGTH: 30,
  MAX_TAGS_COUNT: 10,
  MAX_CATEGORY_LENGTH: 50,
};

// Validate before submission
if (question.length > LIMITS.MAX_QUESTION_LENGTH) {
  throw new Error("Question too long");
}

if (outcomes.length > LIMITS.MAX_OUTCOMES_COUNT) {
  throw new Error("Too many outcomes");
}
```

## 📚 Documentation

- **Full Documentation**: `METADATA_LIMITS.md`
- **Implementation Details**: `IMPLEMENTATION_SUMMARY.md`
- **PR Description**: `PR_DESCRIPTION.md`
- **Source Code**: `src/metadata_limits.rs`
- **Tests**: `src/metadata_limits_tests.rs`
