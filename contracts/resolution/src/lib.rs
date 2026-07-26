#![no_std]

use soroban_sdk::{contract, contractimpl, Address, Env, String};

pub mod events;
#[cfg(test)]
mod test;

#[contract]
pub struct ResolutionContract;

#[contractimpl]
impl ResolutionContract {
    /// Starts the resolution process for a given market.
    /// Emits a `ResolutionStarted` event.
    /// 
    /// # Arguments
    /// * `env` - The environment
    /// * `market_id` - The ID of the market to resolve
    /// * `resolved_by` - The address initiating the resolution
    pub fn start_resolution(env: Env, market_id: String, resolved_by: Address) {
        resolved_by.require_auth();
        events::emit_resolution_started(&env, market_id, resolved_by);
    }

    /// Disputes a resolution.
    /// Emits a `ResolutionDisputed` event.
    /// 
    /// # Arguments
    /// * `env` - The environment
    /// * `market_id` - The ID of the market being disputed
    /// * `disputed_by` - The address of the user disputing the resolution
    /// * `reason` - The reason for the dispute
    pub fn dispute_resolution(env: Env, market_id: String, disputed_by: Address, reason: String) {
        disputed_by.require_auth();
        events::emit_resolution_disputed(&env, market_id, disputed_by, reason);
    }

    /// Finalizes a resolution.
    /// Emits a `ResolutionFinalized` event.
    /// 
    /// # Arguments
    /// * `env` - The environment
    /// * `market_id` - The ID of the market being finalized
    /// * `admin` - The address of the admin finalizing the resolution
    /// * `outcome` - The final outcome
    pub fn finalize_resolution(env: Env, market_id: String, admin: Address, outcome: String) {
        admin.require_auth();
        events::emit_resolution_finalized(&env, market_id, outcome);
    }
}
