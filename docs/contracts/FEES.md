# Fee Arithmetic Invariants

This document captures the arithmetic guarantees for platform-fee calculation in
`contracts/predictify-hybrid/src/fees.rs`.

## Core Rules

1. All fee percentage math uses checked arithmetic.
2. Any overflow in fee multiplication, addition, subtraction, or division returns
   `Error::FeeArithmeticOverflow`.
3. Basis-point calculations round down using integer division.
4. Fee breakdowns must reconcile exactly:

```text
platform_fee + user_payout_amount == total_staked
```

## Basis-Point Rounding

Platform fees are computed with checked multiply-then-divide logic:

```text
floor(total_staked * fee_bps / 10_000)
```

Because all supported fee inputs are non-negative, integer division is equivalent
to floor rounding. This intentionally favors reconciliation over accidental
over-collection.

## Overflow Handling

The following operations must never wrap:

- `FeeCalculator::calculate_platform_fee`
- `FeeCalculator::calculate_user_payout_after_fees`
- `FeeCalculator::calculate_fee_breakdown`
- dynamic fee multiplier math
- `FeeManager::collect_fees` through its fee calculation path
- fee vault and creation-fee total accumulation in `FeeTracker`

If an intermediate value exceeds `i128`, the operation fails with
`Error::FeeArithmeticOverflow` and no fee state is updated.

## Regression Coverage

Regression tests cover:

- zero pool rejection
- 1-bps floor rounding
- overflow-adjacent `i128::MAX` stake pools
- fee breakdown reconciliation after floor rounding
- no mutation when `collect_fees` encounters overflow
- fee vault accumulation overflow rejection
