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
        ("place_bets",                 "bet_batch_placed",           "Fired when multiple bets are placed in a single batch"),
        ("resolve_market_bets",        "bet_status_updated",         "Fired when a bet's status changes (won/lost/refunded/cancelled)"),
        ("set_global_bet_limits",      "bet_limits_updated",         "Fired when global or per-event bet limits are updated"),
        ("update_market_bet_stats",    "bet_stats_updated",          "Fired when per-market betting statistics are updated"),
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
        // ── Governance lifecycle events (FWC26) ────────────────────────────────
        ("create_proposal",            "gov_prop",                   "Fired when a governance proposal is created"),
        ("vote",                       "gov_vote",                   "Fired when a direct vote is cast on a proposal"),
        ("commit_vote",                "gov_cmit",                   "Fired when a commit-reveal commitment is submitted"),
        ("reveal_vote",                "gov_rvl",                    "Fired when a commit-reveal vote is revealed"),
        ("execute_proposal",           "gov_exec",                   "Fired when a governance proposal is executed"),
        ("cancel_proposal",            "gov_canc",                   "Fired when a governance proposal is cancelled"),
        ("validate_proposal",          "gov_rej",                    "Fired when a proposal is auto-rejected below floor quorum"),
        ("set_voting_period",          "gov_vp_upd",                 "Fired when the governance voting period is updated"),
        ("set_quorum",                 "gov_qrm",                    "Fired when the governance quorum is updated"),
        ("set_quorum_decay",           "gov_qdcy",                   "Fired when quorum decay config is updated or disabled"),
        ("set_delegate",               "gov_dlgset",                 "Fired when a delegator activates a vote delegation"),
        ("unset_delegate",             "gov_dlguns",                 "Fired when a delegator removes their vote delegation"),
        // ── Governance registry events (FWC26) ─────────────────────────────────
        ("governance_registry_init",   "reg_init",                   "Fired when the governance parameter registry is initialised"),
        ("propose_parameter",          "reg_prop",                   "Fired when a registry parameter change is proposed"),
        ("execute_parameter",          "reg_exec",                   "Fired when a pending registry parameter change is executed"),
        ("cancel_parameter",           "reg_canc",                   "Fired when a pending registry parameter change is cancelled"),
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
        // Base entries + 16 new governance-lifecycle entries added in FWC26.
        assert!(catalog.len() >= 18);
    }

    #[test]
    fn test_catalog_contains_market_created() {
        let env = Env::default();
        let catalog = get_event_topic_catalog(&env);
        let found = catalog.iter().any(|e| e.topic == String::from_str(&env, "market_created"));
        assert!(found);
    }

    #[test]
    fn test_catalog_contains_governance_lifecycle_topics() {
        let env = Env::default();
        let catalog = get_event_topic_catalog(&env);
        for topic in &[
            "gov_prop", "gov_vote", "gov_cmit", "gov_rvl", "gov_exec",
            "gov_canc", "gov_rej", "gov_vp_upd", "gov_qrm", "gov_qdcy",
            "gov_dlgset", "gov_dlguns",
            "reg_init", "reg_prop", "reg_exec", "reg_canc",
        ] {
            let found = catalog
                .iter()
                .any(|e| e.topic == String::from_str(&env, topic));
            assert!(found, "catalog must contain topic: {}", topic);
        }
    }
}
