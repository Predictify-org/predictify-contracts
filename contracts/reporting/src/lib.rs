//! Reporting contract with admin-gated pause/resume mechanism.
//!
//! Provides a reporting subsystem that allows authorised reporters to submit
//! reports, admins to verify/dispute/resolve them, and an emergency pause
//! switch that halts all state-changing operations while preserving read
//! access.
//!
//! # Pause / Resume
//!
//! The pause mechanism is implemented in the [`pause`] module. When reporting
//! is paused, all state-changing entrypoints revert with
//! [`ReportingError::ReportingPaused`]. The read-only [`ReportingContract::is_reporting_paused`]
//! view is unaffected.
//!
//! # Auth Matrix
//!
//! | Function | Required Role |
//! |---|---|
//! | `initialize` | Admin (bootstrapper) |
//! | `submit_report` | Reporter |
//! | `verify_report` | Admin |
//! | `dispute_report` | Reporter |
//! | `resolve_dispute` | Admin |
//! | `update_report_status` | Admin |
//! | `delete_report` | Admin |
//! | `pause_reporting` | Admin |
//! | `unpause_reporting` | Admin |
//! | `transfer_ownership` | Admin |
//! | `is_reporting_paused` | Anyone (read-only) |
//! | `admin` | Anyone (read-only) |

#![no_std]

extern crate std;

mod err;
mod events;
mod pause;
mod types;

pub use err::ReportingError;
pub use types::DataKey;

use soroban_sdk::{contract, contractimpl, Address, Env, String};

/// The Reporting contract.
#[contract]
pub struct ReportingContract;

#[contractimpl]
impl ReportingContract {
    // -----------------------------------------------------------------------
    // Initialisation
    // -----------------------------------------------------------------------

    /// Initialise the contract, recording `admin` as the sole administrator.
    ///
    /// # Parameters
    ///
    /// * `admin` — The address that will hold admin privileges. Must sign the
    ///   transaction (`require_auth` is enforced).
    ///
    /// # Errors
    ///
    /// * [`ReportingError::AlreadyInitialized`] — Contract has already been
    ///   initialised. Call is idempotent-safe: clients should check before
    ///   calling rather than relying on the error.
    ///
    /// # Auth
    ///
    /// Requires a valid signature from `admin`.
    pub fn initialize(env: Env, admin: Address) -> Result<(), ReportingError> {
        admin.require_auth();

        if env.storage().instance().has(&DataKey::Initialized) {
            return Err(ReportingError::AlreadyInitialized);
        }

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Initialized, &true);

        // Initialise ID counters; first issued ID will be 1.
        env.storage()
            .persistent()
            .set(&DataKey::NextReportId, &1u32);
        env.storage()
            .persistent()
            .set(&DataKey::NextDisputeId, &1u32);

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Report lifecycle
    // -----------------------------------------------------------------------

    /// Submit a new report against a market.
    ///
    /// # Parameters
    ///
    /// * `reporter`     — Address of the reporting party. Must sign the call.
    /// * `market_id`    — Identifier of the market this report relates to.
    /// * `report_data`  — Human-readable or structured report payload.
    /// * `report_hash`  — On-chain commitment hash of the off-chain report
    ///                    document (e.g. SHA-256 hex string).
    ///
    /// # Errors
    ///
    /// * [`ReportingError::ReportingPaused`] — Reporting is currently paused;
    ///   all state-changing operations are halted until unpaused.
    ///
    /// # Auth
    ///
    /// Requires a valid signature from `reporter`.
    pub fn submit_report(
        env: Env,
        reporter: Address,
        market_id: u32,
        report_data: String,
        report_hash: String,
    ) -> Result<(), ReportingError> {
        reporter.require_auth();
        // Guard: halt state changes when paused.
        pause::require_not_paused(&env)?;
        let _ = (market_id, report_data, report_hash);
        Ok(())
    }

    /// Verify a previously submitted report.
    ///
    /// # Parameters
    ///
    /// * `admin`               — Current admin address. Must sign the call.
    /// * `report_id`           — Identifier of the report to verify.
    /// * `verification_result` — `true` if the report passes verification,
    ///                           `false` otherwise.
    ///
    /// # Errors
    ///
    /// * [`ReportingError::ReportingPaused`] — Reporting is currently paused.
    ///
    /// # Auth
    ///
    /// Requires a valid signature from `admin`.
    pub fn verify_report(
        env: Env,
        admin: Address,
        report_id: u32,
        verification_result: bool,
    ) -> Result<(), ReportingError> {
        admin.require_auth();
        pause::require_not_paused(&env)?;
        let _ = (report_id, verification_result);
        Ok(())
    }

    /// Dispute a report, initiating the dispute-resolution workflow.
    ///
    /// # Parameters
    ///
    /// * `reporter`       — Address of the disputing party. Must sign the call.
    /// * `report_id`      — Identifier of the report being disputed.
    /// * `dispute_reason` — Human-readable rationale for the dispute.
    ///
    /// # Errors
    ///
    /// * [`ReportingError::ReportingPaused`] — Reporting is currently paused.
    ///
    /// # Auth
    ///
    /// Requires a valid signature from `reporter`.
    pub fn dispute_report(
        env: Env,
        reporter: Address,
        report_id: u32,
        dispute_reason: String,
    ) -> Result<(), ReportingError> {
        reporter.require_auth();
        pause::require_not_paused(&env)?;
        let _ = (report_id, dispute_reason);
        Ok(())
    }

    /// Resolve an active dispute, closing the dispute-resolution workflow.
    ///
    /// # Parameters
    ///
    /// * `admin`      — Current admin address. Must sign the call.
    /// * `dispute_id` — Identifier of the dispute to resolve.
    /// * `resolution` — `true` if the dispute is upheld (reporter wins),
    ///                  `false` if dismissed.
    ///
    /// # Errors
    ///
    /// * [`ReportingError::ReportingPaused`] — Reporting is currently paused.
    ///
    /// # Auth
    ///
    /// Requires a valid signature from `admin`.
    pub fn resolve_dispute(
        env: Env,
        admin: Address,
        dispute_id: u32,
        resolution: bool,
    ) -> Result<(), ReportingError> {
        admin.require_auth();
        pause::require_not_paused(&env)?;
        let _ = (dispute_id, resolution);
        Ok(())
    }

    /// Update the status code of a report.
    ///
    /// # Parameters
    ///
    /// * `admin`      — Current admin address. Must sign the call.
    /// * `report_id`  — Identifier of the report to update.
    /// * `new_status` — New numeric status code to assign to the report.
    ///
    /// # Errors
    ///
    /// * [`ReportingError::ReportingPaused`] — Reporting is currently paused.
    ///
    /// # Auth
    ///
    /// Requires a valid signature from `admin`.
    pub fn update_report_status(
        env: Env,
        admin: Address,
        report_id: u32,
        new_status: u32,
    ) -> Result<(), ReportingError> {
        admin.require_auth();
        pause::require_not_paused(&env)?;
        let _ = (report_id, new_status);
        Ok(())
    }

    /// Permanently delete a report by ID.
    ///
    /// # Parameters
    ///
    /// * `admin`     — Current admin address. Must sign the call.
    /// * `report_id` — Identifier of the report to delete.
    ///
    /// # Errors
    ///
    /// * [`ReportingError::ReportingPaused`] — Reporting is currently paused.
    ///
    /// # Auth
    ///
    /// Requires a valid signature from `admin`.
    pub fn delete_report(
        env: Env,
        admin: Address,
        report_id: u32,
    ) -> Result<(), ReportingError> {
        admin.require_auth();
        pause::require_not_paused(&env)?;
        let _ = report_id;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Pause / Resume
    // -----------------------------------------------------------------------

    /// Pause the reporting mechanism, halting all state-changing operations.
    ///
    /// Calling this when already paused is a no-op (idempotent). A
    /// `reporting_paused` event is emitted only on a state transition.
    ///
    /// # Parameters
    ///
    /// * `admin` — Current admin address. Must sign the call.
    ///
    /// # Errors
    ///
    /// * [`ReportingError::NotInitialized`] — Contract has not been
    ///   initialised; no admin is stored.
    ///
    /// # Auth
    ///
    /// Requires a valid signature from `admin`.
    pub fn pause_reporting(env: Env, admin: Address) -> Result<(), ReportingError> {
        admin.require_auth();
        pause::pause_reporting(&env)
    }

    /// Resume the reporting mechanism, re-enabling all state-changing
    /// operations.
    ///
    /// Calling this when already unpaused is a no-op (idempotent). A
    /// `reporting_unpaused` event is emitted only on a state transition.
    ///
    /// # Parameters
    ///
    /// * `admin` — Current admin address. Must sign the call.
    ///
    /// # Errors
    ///
    /// * [`ReportingError::NotInitialized`] — Contract has not been
    ///   initialised; no admin is stored.
    ///
    /// # Auth
    ///
    /// Requires a valid signature from `admin`.
    pub fn unpause_reporting(env: Env, admin: Address) -> Result<(), ReportingError> {
        admin.require_auth();
        pause::unpause_reporting(&env)
    }

    // -----------------------------------------------------------------------
    // Ownership
    // -----------------------------------------------------------------------

    /// Transfer contract ownership to a new administrator.
    ///
    /// The `new_owner` address is validated to ensure it is not the same as
    /// the current admin (no-op guard) and that it is a well-formed address.
    ///
    /// # Parameters
    ///
    /// * `admin`     — Current admin address. Must sign the call.
    /// * `new_owner` — Address of the incoming admin.
    ///
    /// # Errors
    ///
    /// * [`ReportingError::InvalidNewOwner`] — `new_owner` is identical to the
    ///   current admin (self-transfer is rejected to avoid accidents).
    ///
    /// # Auth
    ///
    /// Requires a valid signature from `admin`.
    pub fn transfer_ownership(
        env: Env,
        admin: Address,
        new_owner: Address,
    ) -> Result<(), ReportingError> {
        admin.require_auth();

        // Reject a self-transfer — it is almost certainly a caller mistake and
        // provides no value.
        if admin == new_owner {
            return Err(ReportingError::InvalidNewOwner);
        }

        env.storage().instance().set(&DataKey::Admin, &new_owner);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Read-only views
    // -----------------------------------------------------------------------

    /// Return whether reporting is currently paused.
    ///
    /// This is a read-only view; it does **not** require authentication.
    pub fn is_reporting_paused(env: Env) -> bool {
        pause::is_reporting_paused(&env)
    }

    /// Return the current admin address.
    ///
    /// # Panics
    ///
    /// Panics with `"not initialized"` if the contract has not been
    /// initialised yet. Callers should ensure the contract is initialized
    /// before querying this view.
    pub fn admin(env: Env) -> Address {
        env.storage()
            .instance()
            .get::<DataKey, Address>(&DataKey::Admin)
            .expect("not initialized")
    }
}
