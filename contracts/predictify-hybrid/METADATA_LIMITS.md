# Metadata Length Limits Implementation

## Overview

This document describes the metadata length limits and encoding validation implementation for the Predictify Hybrid smart contract. These limits control ledger storage footprints, prevent denial-of-service and byte-inflation attacks, ensure predictable gas consumption, and reject malformed inputs deterministically before any state mutation occurs.

## Security Rationale

### Threat Model

Without strict byte limits and encoding validation, the contract is vulnerable to several attack vectors:

1. **Storage DoS and Byte-Inflation**: Attackers could submit inputs with multi-byte UTF-8 glyphs to inflate ledger storage beyond character-based expectations, consuming persistent contract storage.
2. **Malformed Encoding and Parser Exploits**: Unvalidated byte streams or non-canonical encodings could cause deserialization crashes or state corruption in indexers and downstream clients.
3. **Control Character Injection**: Embedded control characters (`0x00..=0x1F`) could desynchronize client UIs, break log parsers, or poison oracle feed matching.
4. **Unbounded Collections**: Iterating over unconstrained outcome vectors or tag lists risks gas exhaustion, blocking legitimate market resolution.

### Defense Strategy

The implementation provides defense-in-depth:

- **Byte-Accurate Bounds**: All text length constraints are measured and validated in raw UTF-8 bytes rather than Unicode scalar values, guaranteeing an upper bound on storage size.
- **Fail-Early Validation**: Encoding checks, control character rejections, and length bounds are enforced before any storage reads or mutations.
- **Deterministic Rejection**: Invalid UTF-8 and control characters immediately yield `Error::InvalidInput`.
- **Authorization and Versioning Integrity**: Administrative updates require primary admin signatures and immediately update the cryptographic metadata commitment (`refresh_metadata_commitment`), invalidating stale client state.

## Implemented Limits

### String Byte Limits

| Field | Min Bytes | Max Bytes | Error Code | Rationale |
| --- | --- | --- | --- | --- |
| Question / Description | 10 | 500 | `QuestionTooLong` (420) / `InvalidQuestion` (3) | Accommodates clear, detailed market questions |
| Outcome Label | 2 | 100 | `OutcomeTooLong` (421) / `InvalidOutcomes` (4) | Prevents bloated outcome strings |
| Category | 2 | 100 | `CategoryTooShort` (440) / `CategoryTooLong` (425) | Concise topic taxonomy |
| Tag | 2 | 50 | `TagTooShort` (441) / `TagTooLong` (426) | Keyword indexing |
| Oracle Feed ID | 1 | 200 | `FeedIdTooLong` (423) | Supports Pyth 64-char hex strings with headroom |
| Comparison Operator | 1 | 10 | `ComparisonTooLong` (424) | Operator tokens ("gt", "lt", "eq") |
| Extension Reason | 1 | 300 | `ExtensionReasonTooLong` (428) | Audit trail explanation for resolution delay |
| Source Identifier | 1 | 100 | `SourceTooLong` (429) | Oracle endpoint and identifier strings |
| Error Message | 1 | 200 | `ErrorMessageTooLong` (430) | Bounded failure diagnostic text |
| Signature | 1 | 500 | `SignatureTooLong` (431) | Base64-encoded cryptographic signatures |

### Vector Collection Limits

| Field | Max Count | Error Code | Rationale |
| --- | --- | --- | --- |
| Outcomes | 20 | `TooManyOutcomes` (422) | Minimum 2, maximum 20; prevents high computation loops |
| Tags | 10 | `TooManyTags` (427) | Bounded indexing set per event |
| Extension History | 50 | `TooManyExtensions` (432) | Prevents infinite market extensions |
| Oracle Results | 10 | `TooManyOracleResults` (433) | Multi-oracle quorum cap |
| Winning Outcomes | 10 | `TooManyWinningOutcomes` (434) | Handles tie distribution safely |

## Encoding Validation and Normalization

### UTF-8 and Control Character Scanning

All user-supplied string fields are validated via `metadata_limits::scan_metadata_text`, which:
1. Validates that the underlying byte slice is valid UTF-8 via `core::str::from_utf8`. If decoding fails, it deterministically returns `Error::InvalidInput`.
2. Inspects characters for ASCII control codes (`0x00..=0x1F`). If any control code is detected, it returns `Error::InvalidInput`.
3. Returns the exact UTF-8 byte length for boundary comparison.

### Outcome Normalization and Deduplication

To prevent ambiguous outcomes (such as "Yes", "yes ", "YES!") and front-running on typographical variants:
1. Outcomes are trimmed of leading and trailing whitespace.
2. Internal whitespace sequences are collapsed to single spaces.
3. Common punctuation characters are stripped for comparison.
4. Levenshtein distance similarity (>80%) and semantic synonym clusters are rejected.

## State Mutation and Versioning Preservation

Metadata updates (`update_event_description`, `update_event_outcomes`, `update_event_category`, `update_event_tags`):
- Enforce caller authorization via `Self::require_primary_admin(&env, &admin)`.
- Execute all encoding and limit validations before modifying storage.
- Reject updates if any bets have already been placed (`Error::BetsAlreadyPlaced`).
- Compute and persist updated metadata hashes via `market.refresh_metadata_commitment(&env)`.

## Verification and Testing

The implementation is verified by unit, boundary, and property-based fuzz tests:
- **Exact Byte Boundaries**: Verifies acceptance at byte limit and rejection at limit + 1 byte for single-byte ASCII and multi-byte UTF-8 sequences.
- **Deterministic Rejection**: Verifies rejection of invalid UTF-8 bytes and ASCII control characters before mutation.
- **Property-Based Fuzzing**: Uses `proptest` over arbitrary printable ASCII, Unicode symbols, and control character injections.
