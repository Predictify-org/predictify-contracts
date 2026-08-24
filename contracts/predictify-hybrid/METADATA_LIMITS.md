# Metadata Length Limits Implementation

## Overview

This document describes the metadata length limits implementation for the Predictify Hybrid smart contract. These limits are designed to control storage costs, prevent denial-of-service attacks, and ensure predictable gas consumption.

## Security Rationale

### Threat Model

Without metadata length limits, the contract is vulnerable to several attack vectors:

1. **Storage DoS**: Attackers could create markets with extremely large metadata (e.g., 10KB questions, hundreds of outcomes), consuming excessive storage and making the contract expensive or impossible to use.

2. **Gas Exhaustion**: Operations that iterate over large vectors (e.g., validating 1000 outcomes) could exceed gas limits, causing legitimate transactions to fail.

3. **Economic Attack**: Malicious actors could force the platform to pay high storage costs by creating markets with bloated metadata.

4. **Data Integrity**: Unreasonably large inputs may indicate malformed or malicious data that should be rejected.

### Defense Strategy

The implemented limits provide defense-in-depth:

- **Conservative Bounds**: Limits are set well above legitimate use cases but far below abuse thresholds
- **Early Validation**: Checks occur during market creation, before storage costs are incurred
- **Clear Error Messages**: Users receive specific feedback about which limit was exceeded
- **Auditor-Friendly**: All limits are defined as named constants with clear documentation

## Implemented Limits

### String Length Limits

| Field               | Limit     | Rationale                                                      |
| ------------------- | --------- | -------------------------------------------------------------- |
| Question            | 500 chars | Most questions are 50-150 chars; 500 allows detailed questions |
| Outcome Label       | 100 chars | Labels like "yes", "no", "Team A wins" are typically <20 chars |
| Oracle Feed ID      | 200 chars | Accommodates Pyth's 64-char hex IDs with headroom              |
| Comparison Operator | 10 chars  | Valid operators are 2-3 chars ("gt", "lt", "eq")               |
| Category            | 50 chars  | Categories like "crypto", "sports" are typically <20 chars     |
| Tag                 | 30 chars  | Individual tags should be concise keywords                     |
| Extension Reason    | 300 chars | Allows detailed justification for extensions                   |
| Source Identifier   | 100 chars | Oracle source identifiers and URLs                             |
| Error Message       | 200 chars | Informative error descriptions                                 |
| Signature           | 500 chars | Accommodates base64-encoded cryptographic signatures           |

### Vector Length Limits

| Field             | Limit | Rationale                                                        |
| ----------------- | ----- | ---------------------------------------------------------------- |
| Outcomes          | 20    | Most markets are binary (2); multiple choice rarely needs >10    |
| Tags              | 10    | Sufficient for comprehensive categorization                      |
| Extension History | 50    | Prevents unbounded growth; markets shouldn't extend indefinitely |
| Oracle Results    | 10    | Multi-oracle consensus typically uses 3-5 sources                |
| Winning Outcomes  | 10    | Handles tie scenarios without excessive storage                  |

## Implementation Details

### Module Structure

```
contracts/predictify-hybrid/src/
├── metadata_limits.rs          # Core limits and validation functions
├── metadata_limits_tests.rs    # Comprehensive test suite
├── types.rs                    # Integration with existing types
└── err.rs                      # New error codes
```

### New Error Codes

The following error codes were added (420-434):

- `QuestionTooLong` (420)
- `OutcomeTooLong` (421)
- `TooManyOutcomes` (422)
- `FeedIdTooLong` (423)
- `ComparisonTooLong` (424)
- `CategoryTooLong` (425)
- `TagTooLong` (426)
- `TooManyTags` (427)
- `ExtensionReasonTooLong` (428)
- `SourceTooLong` (429)
- `ErrorMessageTooLong` (430)
- `SignatureTooLong` (431)
- `TooManyExtensions` (432)
- `TooManyOracleResults` (433)
- `TooManyWinningOutcomes` (434)

### Validation Integration

Validation is integrated at multiple levels:

1. **Type-Level Validation**: `OracleConfig::validate()` and `Market::validate()` call metadata limit checks
2. **Creation-Time Validation**: Market creation validates all metadata before storage
3. **Extension Validation**: `MarketExtension::validate()` checks extension reasons
4. **Explicit Validation**: Public validation functions can be called directly

### Example Usage

```rust
use predictify_hybrid::metadata_limits::*;

// Validate a question
let question = String::from_str(&env, "Will BTC reach $100k?");
validate_question_length(&question)?;

// Validate outcomes
let outcomes = Vec::from_array(&env, [
    String::from_str(&env, "yes"),
    String::from_str(&env, "no"),
]);
validate_outcomes_count(&outcomes)?;
validate_outcomes_length(&outcomes)?;

// Validate tags
let tags = Vec::from_array(&env, [
    String::from_str(&env, "bitcoin"),
    String::from_str(&env, "crypto"),
]);
validate_tags_count(&tags)?;
validate_tags_length(&tags)?;
```

## Testing

### Test Coverage

The implementation includes comprehensive tests:

- **String Length Tests**: Valid, at-limit, and exceeds-limit cases for all string fields
- **Vector Length Tests**: Valid, at-limit, and exceeds-limit cases for all vector fields
- **Integration Tests**: Validation through `OracleConfig`, `Market`, and `MarketExtension`
- **Edge Case Tests**: Empty strings, empty vectors, zero counts

### Running Tests

```bash
cd contracts/predictify-hybrid
cargo test metadata_limits
```

### Test Results

All tests pass, validating:

- ✅ Valid inputs are accepted
- ✅ Inputs at limits are accepted
- ✅ Inputs exceeding limits are rejected with correct error codes
- ✅ Integration with existing types works correctly
- ✅ Edge cases are handled properly

## Security Considerations

### Audit Checklist

- [x] All string fields have maximum length limits
- [x] All vector fields have maximum count limits
- [x] Limits are enforced before storage operations
- [x] Error messages clearly indicate which limit was exceeded
- [x] Limits are documented with rationale
- [x] Tests validate enforcement at boundaries
- [x] Integration with existing validation is complete

### Known Limitations

1. **UTF-8 Considerations**: Limits are based on byte length, not character count. Multi-byte UTF-8 characters may result in fewer visible characters than the limit suggests.

2. **Gas Costs**: While limits prevent excessive gas consumption, they don't guarantee operations will complete within block gas limits in all scenarios.

3. **Future Extensibility**: Increasing limits in future versions requires careful consideration of backward compatibility with existing markets.

### Recommendations for Auditors

1. **Verify Constant Values**: Review that limit constants are reasonable for the use case
2. **Check Validation Coverage**: Ensure all user-provided strings and vectors are validated
3. **Test Boundary Conditions**: Verify behavior at exact limit values
4. **Review Error Handling**: Confirm appropriate errors are returned for each violation
5. **Assess Gas Impact**: Consider gas costs of validation operations

## Integration Guide

### For Frontend Developers

Implement client-side validation to provide immediate feedback:

```javascript
const LIMITS = {
  MAX_QUESTION_LENGTH: 500,
  MAX_OUTCOME_LENGTH: 100,
  MAX_OUTCOMES_COUNT: 20,
  MAX_TAG_LENGTH: 30,
  MAX_TAGS_COUNT: 10,
  MAX_CATEGORY_LENGTH: 50,
  MAX_EXTENSION_REASON_LENGTH: 300,
};

function validateMarketCreation(params) {
  if (params.question.length > LIMITS.MAX_QUESTION_LENGTH) {
    throw new Error(
      `Question exceeds ${LIMITS.MAX_QUESTION_LENGTH} characters`,
    );
  }

  if (params.outcomes.length > LIMITS.MAX_OUTCOMES_COUNT) {
    throw new Error(`Too many outcomes (max ${LIMITS.MAX_OUTCOMES_COUNT})`);
  }

  for (const outcome of params.outcomes) {
    if (outcome.length > LIMITS.MAX_OUTCOME_LENGTH) {
      throw new Error(
        `Outcome "${outcome}" exceeds ${LIMITS.MAX_OUTCOME_LENGTH} characters`,
      );
    }
  }

  // ... additional validations
}
```

### For Backend Services

When creating markets programmatically, validate inputs before submission:

```rust
use predictify_hybrid::metadata_limits::*;

fn create_market_safe(params: MarketParams) -> Result<(), Error> {
    // Validate all metadata before contract call
    validate_question_length(&params.question)?;
    validate_outcomes_count(&params.outcomes)?;
    validate_outcomes_length(&params.outcomes)?;
    validate_tags_count(&params.tags)?;
    validate_tags_length(&params.tags)?;

    // Proceed with contract call
    contract.create_market(params)
}
```

## Performance Impact

### Storage Savings

Assuming average market metadata:

- Question: 100 chars (vs potential 10KB without limits)
- Outcomes: 3 outcomes × 20 chars (vs potential 100 outcomes × 1KB)
- Tags: 5 tags × 15 chars (vs potential 50 tags × 100 chars)

**Estimated storage savings per market**: ~95% reduction in worst-case storage

### Gas Consumption

Validation adds minimal gas overhead:

- String length check: O(1) operation
- Vector length check: O(1) operation
- Per-element validation: O(n) where n is bounded by limits

**Estimated gas overhead**: <1% of total market creation cost

## Future Considerations

### Potential Adjustments

If usage patterns indicate limits are too restrictive:

1. **Increase Limits**: Can be done in contract upgrade with backward compatibility
2. **Tiered Limits**: Different limits for different market types or user tiers
3. **Dynamic Limits**: Adjust limits based on network conditions or governance

### Monitoring

Track metrics to inform future adjustments:

- Distribution of actual metadata sizes
- Frequency of limit violations
- User feedback on restrictiveness
- Storage cost trends

## Conclusion

The metadata length limits implementation provides robust protection against storage DoS attacks and excessive gas consumption while maintaining flexibility for legitimate use cases. The implementation is:

- **Secure**: Prevents known attack vectors
- **Tested**: Comprehensive test coverage validates correctness
- **Documented**: Clear rationale and usage examples
- **Auditor-Friendly**: Easy to review and verify
- **User-Friendly**: Clear error messages guide users to valid inputs

## References

- [Soroban Storage Best Practices](https://soroban.stellar.org/docs/learn/storage)
- [Smart Contract Security Patterns](https://consensys.github.io/smart-contract-best-practices/)
- [Gas Optimization Techniques](https://soroban.stellar.org/docs/learn/optimization)
