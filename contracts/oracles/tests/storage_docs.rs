const STORAGE_DOCUMENTATION: &str = include_str!("../docs/storage.md");

#[test]
fn storage_documentation_covers_all_storage_tiers_and_registry_key() {
    for required_section in ["Instance", "Persistent", "Temporary"] {
        assert!(
            STORAGE_DOCUMENTATION.contains(required_section),
            "storage documentation must describe the {required_section} tier"
        );
    }

    assert!(
        STORAGE_DOCUMENTATION.contains("Symbol(\"OracleList\")"),
        "storage documentation must identify the OracleList key"
    );
    assert!(
        STORAGE_DOCUMENTATION.contains("Vec<Address>"),
        "storage documentation must describe the registry value"
    );
    assert!(
        STORAGE_DOCUMENTATION.contains("does not change the contract's public API"),
        "storage documentation must state the API impact"
    );
}
