# Oracle Resolution

This document defines the deterministic oracle-resolution policy for `predictify-hybrid` markets and events.

## Scope

The policy applies to:

- `PredictifyHybrid::create_market`
- `PredictifyHybrid::create_event`
- `PredictifyHybrid::fetch_oracle_result`
- `PredictifyHybrid::refund_on_oracle_failure`

It also mirrors the intended behavior of the internal `contracts/predictify-hybrid/src/resolution.rs` flow.

## Stored Configuration

Each market and event stores:

- `oracle_config`: the primary oracle configuration
- `has_fallback`: whether a fallback oracle is enabled
- `fallback_oracle_config`: the backup oracle configuration when enabled
- `resolution_timeout`: the per-market/per-event deadline, in seconds, measured from `end_time`

`has_fallback = false` means `fallback_oracle_config` is the reserved `OracleConfig::none_sentinel()` value and must never be used for live resolution.

## Deterministic Attempt Order

Automatic resolution uses a fixed order:

1. Attempt the primary oracle once.
2. If and only if `has_fallback` is `true`, attempt the fallback oracle once after the primary attempt fails.
3. If both attempts fail, stop and require manual resolution.

There is no dynamic reordering, no provider scoring, and no implicit retry loop in this path. Auditors and integrators can therefore reason about a single, stable resolution sequence.

## Timeout Policy

The automatic resolution deadline is:

`resolution_deadline = end_time + resolution_timeout`

Behavior is:

- Before `end_time`, oracle resolution is rejected because the market is still open.
- From `end_time` up to but not including `resolution_deadline`, automatic oracle resolution may be attempted.
- At or after `resolution_deadline`, automatic oracle resolution stops immediately with `RESOLUTION_TIMEOUT_REACHED`.

Timeout handling is deliberately fail-closed:

- the contract does not continue trying alternate providers after the deadline
- the contract does not silently extend deadlines
- non-admin refund callers become authorized only once the same per-market timeout has elapsed

## Error Mapping

The active contract path uses the following deterministic outcomes:

- Primary failure with no fallback configured: `ORACLE_UNAVAILABLE`
- Primary failure followed by fallback failure: `FALLBACK_ORACLE_UNAVAILABLE`
- Any automatic resolution attempt at or after the deadline: `RESOLUTION_TIMEOUT_REACHED`

## Events and Auditability

The contract emits events to make the resolution path observable:

- `man_res`: manual resolution required after automatic attempts are exhausted
- `fbk_used`: emitted only when a fallback oracle succeeds
- `res_tmo`: emitted when the market/event reaches the automatic resolution deadline
- `ref_oracl`: emitted when the refund path is used after oracle failure/timeout

For failure cases, the manual-resolution reason string is stable:

- `oracle_resolution_failed_primary_only`
- `oracle_resolution_failed_primary_then_fallback`

These reason codes are intended for off-chain monitors, audit tooling, and incident playbooks.

## Security Notes

Threat model and enforced invariants:

- Fixed ordering prevents an attacker from influencing provider selection at runtime.
- A per-market timeout prevents indefinite oracle polling and makes the refund path predictable.
- The fallback sentinel stays outside the valid oracle-config domain, so `has_fallback = false` cannot collide with a live backup configuration.
- Non-admin callers cannot force early cancellation before the stored market timeout expires.

Explicit non-goals:

- This policy does not define provider-specific trust assumptions.
- This policy does not add weighted consensus across multiple oracle providers.
- This policy does not auto-resolve subjective markets without an oracle result.
