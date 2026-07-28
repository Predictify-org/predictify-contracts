//! Admin-gated pause/resume mechanism for the Reporting contract (issue #1126).
//!
//! When reporting is paused, all state-changing entrypoints (`submit_report`,
//! `verify_report`, `dispute_report`, `resolve_dispute`, `update_report_status`,
//! `delete_report`) are halted. Read-only functions (`is_reporting_paused`) are
//! unaffected.
//!
//! # Usage
//!
//! ```ignore
//! // Pause
//! ReportingContract::pause_reporting(env, admin);
//!
//! // Unpause
//! ReportingContract::unpause_reporting(env, admin);
//!
//! // Guard in state-changing functions
//! require_not_paused(&env)?;
//! ```

use crate::err::ReportingError;
use crate::events;
use crate::types::DataKey;
use soroban_sdk::Env;

/// Check whether reporting is currently paused.
///
/// Returns `true` if paused, `false` otherwise (default). This is a pure
/// read — it is not gated.
pub fn is_reporting_paused(env: &Env) -> bool {
    env.storage()
        .persistent()
        .get::<DataKey, bool>(&DataKey::ReportingPaused)
        .unwrap_or(false)
}

/// Pause the reporting mechanism.
///
/// Only the current admin may call this. Emits a `reporting_paused` event
/// on success.
pub fn pause_reporting(env: &Env) -> Result<(), ReportingError> {
    if is_reporting_paused(env) {
        return Ok(()); // idempotent — already paused
    }

    env.storage()
        .persistent()
        .set(&DataKey::ReportingPaused, &true);

    // Extend TTL so the flag does not expire.
    env.storage()
        .persistent()
        .extend_ttl(&DataKey::ReportingPaused, 100_000, 100_000);

    events::emit_reporting_paused(env, &get_admin(env)?);

    Ok(())
}

/// Resume (unpause) the reporting mechanism.
///
/// Only the current admin may call this. Emits a `reporting_unpaused` event
/// on success.
pub fn unpause_reporting(env: &Env) -> Result<(), ReportingError> {
    if !is_reporting_paused(env) {
        return Ok(()); // idempotent — already unpaused
    }

    env.storage()
        .persistent()
        .set(&DataKey::ReportingPaused, &false);

    // Extend TTL so the flag does not expire.
    env.storage()
        .persistent()
        .extend_ttl(&DataKey::ReportingPaused, 100_000, 100_000);

    events::emit_reporting_unpaused(env, &get_admin(env)?);

    Ok(())
}

/// Guard function for state-changing entrypoints.
///
/// Returns [`ReportingError::ReportingPaused`] if the contract is currently
/// paused, allowing the caller to continue with `Ok(())` otherwise.
pub fn require_not_paused(env: &Env) -> Result<(), ReportingError> {
    if is_reporting_paused(env) {
        Err(ReportingError::ReportingPaused)
    } else {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn get_admin(env: &Env) -> Result<soroban_sdk::Address, ReportingError> {
    env.storage()
        .instance()
        .get::<DataKey, soroban_sdk::Address>(&DataKey::Admin)
        .ok_or(ReportingError::NotInitialized)
}
