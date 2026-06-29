//! Per-entrypoint event topic catalog (issue #736).
//!
//! Provides a machine-readable catalog of every event topic emitted by the
//! contract, intended for off-chain indexers and integrators.

use soroban_sdk::{contracttype, Env, String, Vec};

#[contracttype]
#[derive(Clone, Debug)]
pub struct EventTopicEntry {
    /// Human-readable entrypoint that triggers this event.
    pub entrypoint: String,
    /// Event topic symbol name (matches what is passed to env.events().publish()).
    pub topic: String,
    /// Brief description of when this event fires.
    pub description: String,
}

/// Return the full catalog of event topics emitted by this contract.
pub fn get_event_topic_catalog(env: &Env) -> Vec<EventTopicEntry> {
    let mut catalog = Vec::new(env);

    let entries: &[(&str, &str, &str)] = &[
        ("create_market",              "market_created",             "Fired when a new prediction market is created"),
        ("create_event",               "event_created",              "Fired when a new event is registered"),
        ("vote",                       "vote_cast",                  "Fired when a user casts a vote on a market"),
        ("place_bet",                  "bet_placed",                 "Fired when a bet is placed on a market"),
        ("cancel_bet",                 "bet_cancelled",              "Fired when a bet is cancelled by the bettor"),
        ("resolve_market",             "market_resolved",            "Fired on successful market resolution"),
        ("resolve_market_manual",      "market_resolved_manual",     "Fired on admin manual resolution"),
        ("force_resolve_market",       "market_force_resolved",      "Fired on forced resolution"),
        ("dispute_market",             "dispute_filed",              "Fired when a user disputes a market outcome"),
        ("vote_on_dispute",            "dispute_vote_cast",          "Fired when a user votes on a dispute"),
        ("resolve_dispute",            "dispute_resolved",           "Fired when a dispute is resolved"),
        ("claim_winnings",             "winnings_claimed",           "Fired when a user claims their winnings"),
        ("sweep_unclaimed_winnings",   "unclaimed_winnings_swept",   "Fired when unclaimed winnings are swept to treasury"),
        ("admin_override_verification","oracle_admin_override",      "Fired on admin oracle verification override"),
        ("fetch_oracle_result",        "oracle_result_fetched",      "Fired after fetching oracle result"),
        ("verify_result",              "oracle_result_verified",     "Fired after successful oracle verification"),
        ("accumulate_dispute_fee",     "dispute_fee_accumulated",    "Fired when a dispute fee is accumulated"),
        ("set_governance_min_bet_bps", "governance_min_bet_updated", "Fired when governance updates min bet bps"),
    ];

    for (entrypoint, topic, description) in entries {
        catalog.push_back(EventTopicEntry {
            entrypoint: String::from_str(env, entrypoint),
            topic: String::from_str(env, topic),
            description: String::from_str(env, description),
        });
    }

    catalog
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::Env;

    #[test]
    fn test_catalog_is_non_empty() {
        let env = Env::default();
        let catalog = get_event_topic_catalog(&env);
        assert!(catalog.len() >= 10);
    }

    #[test]
    fn test_catalog_contains_market_created() {
        let env = Env::default();
        let catalog = get_event_topic_catalog(&env);
        let found = catalog.iter().any(|e| e.topic == String::from_str(&env, "market_created"));
        assert!(found);
    }
}
