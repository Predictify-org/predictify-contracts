use soroban_sdk::{Env, String, Symbol, Address};

/// Emits an event when the resolution process for a market has started.
/// 
/// # Arguments
/// * `env` - The environment
/// * `market_id` - The ID of the market being resolved
/// * `resolved_by` - The address of the user initiating the resolution
pub fn emit_resolution_started(env: &Env, market_id: String, resolved_by: Address) {
    let topics = (Symbol::new(env, "resolution"), Symbol::new(env, "started"), market_id);
    env.events().publish(topics, resolved_by);
}

/// Emits an event when a market resolution is disputed.
/// 
/// # Arguments
/// * `env` - The environment
/// * `market_id` - The ID of the market being disputed
/// * `disputed_by` - The address of the user raising the dispute
/// * `reason` - The reason for the dispute
pub fn emit_resolution_disputed(env: &Env, market_id: String, disputed_by: Address, reason: String) {
    let topics = (Symbol::new(env, "resolution"), Symbol::new(env, "disputed"), market_id);
    env.events().publish(topics, (disputed_by, reason));
}

/// Emits an event when a market resolution is finalized.
/// 
/// # Arguments
/// * `env` - The environment
/// * `market_id` - The ID of the market being finalized
/// * `outcome` - The final agreed outcome
pub fn emit_resolution_finalized(env: &Env, market_id: String, outcome: String) {
    let topics = (Symbol::new(env, "resolution"), Symbol::new(env, "finalized"), market_id);
    env.events().publish(topics, outcome);
}
