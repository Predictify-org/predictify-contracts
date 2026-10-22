# Admin Operations

Operational reference for the Predictify Hybrid admin model: roles, rotation,
and M-of-N multisig. Written for integrators and auditors who need to reason
about admin behavior without reading the full `admin.rs` source.

## Admin model

Admin state is split across two layers in contract storage:

| Layer | Storage key | Written by | Read by |
|------|------------|-----------|---------|
| Primary admin | persistent `"Admin"` (Address) | `initialize`, `transfer_admin` | `require_admin_auth` (entry-point gating) |
| Multi-admin role map | persistent `AdminAssignment_<addr>` | `add_admin`, `remove_admin`, `update_admin_role`, `deactivate_admin` / `reactivate_admin` | `validate_permission` (role-based gating) |
| Admin count | persistent `"AdminCount"` (u32) | `add_admin`, `remove_admin`, `migrate_to_multi_admin` | `MultisigManager::set_threshold` |
| Multisig config | persistent `"MultisigConfig"` | `set_threshold` | `approve_action`, `execute_action` |

Roles (`AdminRole`): `SuperAdmin`, `MarketAdmin`, `ConfigAdmin`, `FeeAdmin`,
`ReadOnlyAdmin`. Each role maps to a static set of `AdminPermission`s.

Two gating flows coexist:

- **Primary-admin-only entry points** (e.g. `set_platform_fee`,
  `upgrade_contract`) call `AdminAccessControl::require_admin_auth`, which
  reads the persistent `"Admin"` slot.
- **Multi-admin / role-based entry points** (add/remove admin, multisig
  actions) call `AdminAccessControl::validate_permission`, which — once
  `migrate_to_multi_admin` has run — reads the role map.

This split is intentional: the primary-admin slot is the root-of-trust for
rotation, while the role map is the operational surface for delegated work.

## Primary admin rotation (`transfer_admin`)

`ContractPauseManager::transfer_admin(current, new)`:

- Requires `current` to match the persistent `"Admin"` slot.
- Rejects rotating to the same address (`Error::InvalidInput`).
- Validates the new address via `AdminValidator::validate_admin_address`.
- Writes the new address into `"Admin"`.
- Emits `AdminTransferred` and appends an audit-trail record.

**What it does not do:**

- It does not modify any `AdminRoleAssignment` entry. The rotated-out admin,
  if it was in the role map, remains in the role map until explicitly removed
  via `remove_admin`. See
  `test_rotated_out_admin_retains_multi_admin_role_until_removed`.
- It does not inspect or expire any pending multisig actions. An action
  initiated before rotation remains executable, and any approvals already
  collected (including from the rotated-out admin) still count. See
  `test_rotation_during_pending_action_preserves_approvals`.

Integrators rotating an admin mid-market should either (a) rotate only the
primary slot and accept the multi-admin map as an independent concern, or
(b) follow `transfer_admin` with `remove_admin` / `update_admin_role` calls
to reconcile the role map.

## Multisig (M-of-N) workflow

1. `set_threshold(admin, M)` — requires `Emergency` permission. Rejects
   `M == 0` or `M > active admin count` (`Error::InvalidInput`). Setting
   `M = 1` disables multisig (`enabled = false`). Setting `M > 1` enables it.
2. `create_pending_action(initiator, action_type, target, data)` — requires
   `Emergency` permission. Returns a new `action_id`. Automatically counts
   the initiator as the first approval. `expires_at = now + 86400` (24h).
3. `approve_action(admin, action_id)` — requires `Emergency` permission.
   Rejects if the action is already executed (`InvalidState`), if
   `now > expires_at` (`DisputeError`), or if the admin has already approved
   (`InvalidState`). Returns `true` when the approval count reaches the
   configured threshold.
4. `execute_action(action_id)` — no caller permission check; relies on the
   prior gating. Rejects if already executed (`InvalidState`) or if approval
   count < threshold (`Unauthorized`). Marks `executed = true`.

### Invariants proven by tests

- **Approvals are immutable once recorded.** Deactivating an admin after
  they approve does not retract their approval.
  (`test_deactivated_admin_approval_still_counts`)
- **Deactivated admins cannot approve new actions.**
  (`test_deactivated_admin_cannot_approve`)
- **New admin membership does not retroactively approve open actions.**
  (`test_add_admin_does_not_retroactively_approve_pending_action`)
- **Threshold boundary at `expires_at` is inclusive on approve.**
  (`test_approve_action_at_expiration_boundary`,
  `test_approve_action_after_expiration_rejected`)
- **Double execute is rejected.**
  (`test_double_execute_blocked_even_after_extra_approval`)
- **Lowering the threshold unblocks already-approved actions.**
  (`test_lower_threshold_after_approvals_permits_execution`)

### Known sharp edges (follow-up hardening)

These behaviors are intentional today but worth flagging for integrators:

- **`execute_action` does not re-check expiration.** An action that reached
  threshold before `expires_at` can be executed at any time afterward.
  Callers relying on time-boxed actions should re-verify before submit.
  (`test_execute_expired_action_with_enough_approvals_still_runs`)
- **`remove_admin` does not re-validate `MultisigConfig.threshold`.**
  Removing enough admins can leave `threshold > count`. Re-running
  `set_threshold` is the only way to detect and correct this.
  (`test_remove_admin_can_leave_threshold_above_count`)

## Multisig contract as admin

A Soroban contract address can be installed as either the primary admin or a
delegated admin. Under `env.mock_all_auths()` such contract-admins pass
`require_auth` identically to account addresses; in production, the contract
admin must authorize the call via its own signing logic.

- Initialization with a contract address as primary admin is supported.
  (`test_contract_address_can_act_as_primary_admin`)
- Primary rotation between two contract admins is supported and updates
  only the `"Admin"` slot.
  (`test_contract_admin_rotation_to_new_contract_admin`)

## Threat model and non-goals

In scope for these tests and docs:

- Rotation races (pending-action lifecycle vs. `transfer_admin`).
- Stale approvals after deactivate/reactivate.
- Threshold / admin-count drift.
- Expiration-boundary correctness.
- Contract-address admins passing standard auth.

Non-goals:

- Off-chain key management and signing policy of a contract-admin.
- Time-locked rotation (none is enforced today).
- Social recovery of a lost primary admin.
- Emergency override outside of the `SuperAdmin` path.
