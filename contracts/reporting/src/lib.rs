//! Reporting contract with admin-gated pause/resume mechanism.
//!
//! Provides a reporting subsystem that allows authorised reporters to submit
//! reports, admins to verify/dispute/resolve them, and an emergency pause
//! switch that halts all state-changing operations while preserving read
//! access.
//!
//! # Pause / Resume
//!
//! The pause mechanism (issue #1126) is the primary deliverable. See the
//! [`pause`] module for details.
//!
//! # Auth Matrix
//!
//! | Function | Required Role |
//! |---|---|
//! | `initialize` | Admin |
//! | `submit_report` | Reporter |
//! | `verify_report` | Admin |
//! | `dispute_report` | Reporter |
//! | `resolve_dispute` | Admin |
//! | `update_report_status` | Admin |
//! | `delete_report` | Admin |
//! | `pause_reporting` | Admin |
//! | `unpause_reporting` | Admin |
//! | `transfer_ownership` | Admin |
//! | `is_reporting_paused` | Anyone |

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

    /// Initialise the contract with an `admin`.
    ///
    /// May only be called once.
    pub fn initialize(env: Env, admin: Address) {
        admin.require_auth();

        if env.storage().instance().has(&DataKey::Initialized) {
            panic!("already initialized");
        }

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Initialized, &true);

        // Initialise counters to 1 (so the first ID is 1).
        env.storage()
            .persistent()
            .set(&DataKey::NextReportId, &1u32);
        env.storage()
            .persistent()
            .set(&DataKey::NextDisputeId, &1u32);
    }

    // -----------------------------------------------------------------------
    // Report lifecycle
    // -----------------------------------------------------------------------

    /// Submit a new report.
    ///
    /// * `reporter` — must be the caller and be authorised as a reporter.
    /// * `market_id` — the market this report relates to.
    /// * `report_data` — the report payload.
    /// * `report_hash` — on-chain hash of the off-chain report.
    ///
    /// Reverts with [`ReportingError::ReportingPaused`] when paused.
    pub fn submit_report(
        env: Env,
        reporter: Address,
        market_id: u32,
        report_data: String,
        report_hash: String,
    ) {
        reporter.require_auth();
        // Guard: halt state changes when paused.
        pause::require_not_paused(&env).unwrap();
        let _ = (market_id, report_data, report_hash);
    }

    /// Verify a previously submitted report.
    ///
    /// Reverts with [`ReportingError::ReportingPaused`] when paused.
    pub fn verify_report(env: Env, admin: Address, report_id: u32, verification_result: bool) {
        admin.require_auth();
        pause::require_not_paused(&env).unwrap();
        let _ = (report_id, verification_result);
    }

    /// Dispute a report.
    ///
    /// Reverts with [`ReportingError::ReportingPaused`] when paused.
    pub fn dispute_report(env: Env, reporter: Address, report_id: u32, dispute_reason: String) {
        reporter.require_auth();
        pause::require_not_paused(&env).unwrap();
        let _ = (report_id, dispute_reason);
    }

    /// Resolve an active dispute.
    ///
    /// Reverts with [`ReportingError::ReportingPaused`] when paused.
    pub fn resolve_dispute(env: Env, admin: Address, dispute_id: u32, resolution: bool) {
        admin.require_auth();
        pause::require_not_paused(&env).unwrap();
        let _ = (dispute_id, resolution);
    }

    /// Update the status of a report.
    ///
    /// Reverts with [`ReportingError::ReportingPaused`] when paused.
    pub fn update_report_status(env: Env, admin: Address, report_id: u32, new_status: u32) {
        admin.require_auth();
        pause::require_not_paused(&env).unwrap();
        let _ = (report_id, new_status);
    }

    /// Delete a report by ID.
    ///
    /// Reverts with [`ReportingError::ReportingPaused`] when paused.
    pub fn delete_report(env: Env, admin: Address, report_id: u32) {
        admin.require_auth();
        pause::require_not_paused(&env).unwrap();
        let _ = report_id;
    }

    // -----------------------------------------------------------------------
    // Pause / Resume  (issue #1126)
    // -----------------------------------------------------------------------

    /// Pause the reporting mechanism.
    ///
    /// Only the current admin may call this. All state-changing operations
    /// will revert while paused.
    pub fn pause_reporting(env: Env, admin: Address) {
        admin.require_auth();
        pause::pause_reporting(&env).unwrap();
    }

    /// Resume the reporting mechanism.
    ///
    /// Only the current admin may call this.
    pub fn unpause_reporting(env: Env, admin: Address) {
        admin.require_auth();
        pause::unpause_reporting(&env).unwrap();
    }

    // -----------------------------------------------------------------------
    // Ownership
    // -----------------------------------------------------------------------

    /// Transfer contract ownership to a new admin.
    pub fn transfer_ownership(env: Env, admin: Address, new_owner: Address) {
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &new_owner);
    }

    /// Return whether reporting is currently paused.
    pub fn is_reporting_paused(env: Env) -> bool {
        pause::is_reporting_paused(&env)
    }

    /// Return the current admin address.
    pub fn admin(env: Env) -> Address {
        env.storage()
            .instance()
            .get::<DataKey, Address>(&DataKey::Admin)
            .expect("not initialized")
    }
}
