const STORAGE_DOCUMENTATION: &str = include_str!("../docs/storage.md");

#[test]
fn storage_documentation_covers_all_storage_tiers_and_recovery_keys() {
    for required_section in ["Instance", "Persistent", "Temporary"] {
        assert!(
            STORAGE_DOCUMENTATION.contains(required_section),
            "storage documentation must describe the {required_section} tier"
        );
    }

    // Verify recovery-specific storage keys are documented.
    assert!(
        STORAGE_DOCUMENTATION.contains("recovery_records"),
        "storage documentation must identify the recovery_records key"
    );
    assert!(
        STORAGE_DOCUMENTATION.contains("rcv_pending"),
        "storage documentation must identify the rcv_pending key"
    );
    assert!(
        STORAGE_DOCUMENTATION.contains("rcv_timelock_cfg"),
        "storage documentation must identify the rcv_timelock_cfg key"
    );
    assert!(
        STORAGE_DOCUMENTATION.contains("recovery_status_map"),
        "storage documentation must identify the recovery_status_map key"
    );
    assert!(
        STORAGE_DOCUMENTATION.contains("recovery_v2_migrated"),
        "storage documentation must identify the recovery_v2_migrated key"
    );
    assert!(
        STORAGE_DOCUMENTATION.contains("rcv_hist"),
        "storage documentation must identify the rcv_hist key"
    );
    assert!(
        STORAGE_DOCUMENTATION.contains("recovery_history"),
        "storage documentation must identify the recovery_history legacy key"
    );
    assert!(
        STORAGE_DOCUMENTATION.contains("claim_period_global"),
        "storage documentation must identify the claim_period_global key"
    );
    assert!(
        STORAGE_DOCUMENTATION.contains("claim_period_market"),
        "storage documentation must identify the claim_period_market key"
    );
    assert!(
        STORAGE_DOCUMENTATION.contains("claim_window_start"),
        "storage documentation must identify the claim_window_start key"
    );
    assert!(
        STORAGE_DOCUMENTATION.contains("treasury_addr"),
        "storage documentation must identify the treasury_addr key"
    );

    // Verify value types are documented.
    assert!(
        STORAGE_DOCUMENTATION.contains("Address"),
        "storage documentation must describe the Address value type"
    );
    assert!(
        STORAGE_DOCUMENTATION.contains("Map<Symbol, PendingMarketRecovery>"),
        "storage documentation must describe the PendingMarketRecovery map type"
    );
    assert!(
        STORAGE_DOCUMENTATION.contains("Map<Symbol, String>"),
        "storage documentation must describe the status map type"
    );
    assert!(
        STORAGE_DOCUMENTATION.contains("Map<Symbol, u64>"),
        "storage documentation must describe the claim period map types"
    );
    assert!(
        STORAGE_DOCUMENTATION.contains("MarketRecovery"),
        "storage documentation must reference the MarketRecovery struct"
    );
    assert!(
        STORAGE_DOCUMENTATION.contains("Vec<RecoveryHistoryEntry>"),
        "storage documentation must describe the recovery history value type"
    );
    assert!(
        STORAGE_DOCUMENTATION.contains("Map<Symbol, Vec<RecoveryHistoryEntry>>"),
        "storage documentation must describe the legacy recovery_history map type"
    );
    assert!(
        STORAGE_DOCUMENTATION.contains("PendingMarketRecovery"),
        "storage documentation must reference the PendingMarketRecovery struct"
    );
    assert!(
        STORAGE_DOCUMENTATION.contains("RecoveryTimelockConfig"),
        "storage documentation must reference the RecoveryTimelockConfig struct"
    );

    // Verify TTL constants are documented.
    assert!(
        STORAGE_DOCUMENTATION.contains("RECOVERY_TTL_LEDGERS"),
        "storage documentation must describe the recovery TTL constant"
    );
    assert!(
        STORAGE_DOCUMENTATION.contains("RECOVERY_LIFETIME_THRESHOLD"),
        "storage documentation must describe the recovery lifetime threshold"
    );

    // Verify API impact statement.
    assert!(
        STORAGE_DOCUMENTATION.contains("does not change the contract's public API"),
        "storage documentation must state the API impact"
    );
}
