# Partial refund mechanism

Admin-only recovery for **selected users** on a market that is **not** `Resolved` / `Closed` / `Archived`.

## What it does

- Credits each listed user with **internal** `ReflectorAsset::Stellar` balance equal to their stake.
- Marks the user claimed, removes their stake/vote, decreases `total_staked` by the same amount.
- Sets their bet status to **Refunded** when an active bet exists.
- **Idempotent**: already-claimed users and zero/no stake are skipped; total refunded for those users is 0.

## What it does not do

- Not a full market cancellation (`cancel_event` exists for that).
- Does not refund dispute stakes.
- Does not run on resolved/closed/archived markets (`InvalidState`).
- Caps `users.len()` at **50** (`InvalidInput` above the cap).

## Conservation

For each successful user: balance credit == stake removed == decrease in `total_staked`.
