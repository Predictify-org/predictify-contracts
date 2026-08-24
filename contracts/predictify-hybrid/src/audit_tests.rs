use soroban_sdk::testutils::Address as _;
use soroban_sdk::{vec, Address, Env, String, Vec};

use crate::audit::*;

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_env() -> Env {
        let env = Env::default();
        env.mock_all_auths();
        env
    }

    fn with_contract_context<F, R>(env: &Env, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let contract_id = env.register(crate::PredictifyHybrid {}, ());
        env.as_contract(&contract_id, f)
    }

    #[test]
    fn test_audit_manager_initialization() {
        let env = create_test_env();

        with_contract_context(&env, || {
            // Initialize audit system
            let result = AuditManager::initialize(&env);
            assert!(result.is_ok());

            // Verify config is stored
            let config = AuditManager::get_config(&env);
            assert!(config.is_ok());
            let config = config.unwrap();
            assert_eq!(config.environment, String::from_str(&env, "development"));
            assert_eq!(config.critical_threshold, 0);
            assert_eq!(config.high_threshold, 2);
            assert_eq!(config.medium_threshold, 5);
        });
    }

    #[test]
    fn test_security_audit_checklist_creation() {
        let env = create_test_env();
        let auditor = Address::generate(&env);

        with_contract_context(&env, || {
            // Create security audit checklist
            let result =
                AuditManager::create_audit_checklist(&env, AuditType::Security, auditor.clone());
            assert!(result.is_ok());

            let checklist = result.unwrap();
            assert_eq!(checklist.audit_type, AuditType::Security);
            assert_eq!(checklist.auditor, auditor);
            assert_eq!(checklist.overall_status, AuditStatus::NotStarted);
            assert_eq!(checklist.completion_percentage, 0);

            // Verify security items are present
            assert!(!checklist.items.is_empty());
            let has_access_control = checklist
                .items
                .iter()
                .any(|item| item.id == String::from_str(&env, "SEC_001"));
            assert!(has_access_control);
        });
    }

    #[test]
    fn test_code_review_audit_checklist_creation() {
        let env = create_test_env();
        let auditor = Address::generate(&env);

        with_contract_context(&env, || {
            // Create code review audit checklist
            let result =
                AuditManager::create_audit_checklist(&env, AuditType::CodeReview, auditor.clone());
            assert!(result.is_ok());

            let checklist = result.unwrap();
            assert_eq!(checklist.audit_type, AuditType::CodeReview);
            assert_eq!(checklist.auditor, auditor);
            assert_eq!(checklist.overall_status, AuditStatus::NotStarted);

            // Verify code review items are present
            assert!(!checklist.items.is_empty());
            let has_code_structure = checklist
                .items
                .iter()
                .any(|item| item.id == String::from_str(&env, "CR_001"));
            assert!(has_code_structure);
        });
    }

    #[test]
    fn test_testing_audit_checklist_creation() {
        let env = create_test_env();
        let auditor = Address::generate(&env);

        with_contract_context(&env, || {
            // Create testing audit checklist
            let result =
                AuditManager::create_audit_checklist(&env, AuditType::Testing, auditor.clone());
            assert!(result.is_ok());

            let checklist = result.unwrap();
            assert_eq!(checklist.audit_type, AuditType::Testing);
            assert_eq!(checklist.auditor, auditor);

            // Verify testing items are present
            assert!(!checklist.items.is_empty());
            let has_unit_tests = checklist
                .items
                .iter()
                .any(|item| item.id == String::from_str(&env, "TEST_001"));
            assert!(has_unit_tests);
        });
    }

    #[test]
    fn test_documentation_audit_checklist_creation() {
        let env = create_test_env();
        let auditor = Address::generate(&env);

        with_contract_context(&env, || {
            // Create documentation audit checklist
            let result = AuditManager::create_audit_checklist(
                &env,
                AuditType::Documentation,
                auditor.clone(),
            );
            assert!(result.is_ok());

            let checklist = result.unwrap();
            assert_eq!(checklist.audit_type, AuditType::Documentation);
            assert_eq!(checklist.auditor, auditor);

            // Verify documentation items are present
            assert!(!checklist.items.is_empty());
            let has_api_docs = checklist
                .items
                .iter()
                .any(|item| item.id == String::from_str(&env, "DOC_001"));
            assert!(has_api_docs);
        });
    }

    #[test]
    fn test_deployment_audit_checklist_creation() {
        let env = create_test_env();
        let auditor = Address::generate(&env);

        with_contract_context(&env, || {
            // Create deployment audit checklist
            let result =
                AuditManager::create_audit_checklist(&env, AuditType::Deployment, auditor.clone());
            assert!(result.is_ok());

            let checklist = result.unwrap();
            assert_eq!(checklist.audit_type, AuditType::Deployment);
            assert_eq!(checklist.auditor, auditor);

            // Verify deployment items are present
            assert!(!checklist.items.is_empty());
            let has_compilation = checklist
                .items
                .iter()
                .any(|item| item.id == String::from_str(&env, "DEP_001"));
            assert!(has_compilation);
        });
    }

    #[test]
    fn test_comprehensive_audit_checklist_creation() {
        let env = create_test_env();
        let auditor = Address::generate(&env);

        with_contract_context(&env, || {
            // Create comprehensive audit checklist
            let result = AuditManager::create_audit_checklist(
                &env,
                AuditType::Comprehensive,
                auditor.clone(),
            );
            assert!(result.is_ok());

            let checklist = result.unwrap();
            assert_eq!(checklist.audit_type, AuditType::Comprehensive);
            assert_eq!(checklist.auditor, auditor);

            // Verify comprehensive checklist has items from all types
            assert!(!checklist.items.is_empty());
            let has_security = checklist
                .items
                .iter()
                .any(|item| item.id == String::from_str(&env, "SEC_001"));
            let has_code_review = checklist
                .items
                .iter()
                .any(|item| item.id == String::from_str(&env, "CR_001"));
            let has_testing = checklist
                .items
                .iter()
                .any(|item| item.id == String::from_str(&env, "TEST_001"));
            let has_documentation = checklist
                .items
                .iter()
                .any(|item| item.id == String::from_str(&env, "DOC_001"));
            let has_deployment = checklist
                .items
                .iter()
                .any(|item| item.id == String::from_str(&env, "DEP_001"));

            assert!(has_security);
            assert!(has_code_review);
            assert!(has_testing);
            assert!(has_documentation);
            assert!(has_deployment);
        });
    }

    #[test]
    fn test_audit_item_update() {
        let env = create_test_env();
        let auditor = Address::generate(&env);

        with_contract_context(&env, || {
            // Create security audit checklist
            let _ =
                AuditManager::create_audit_checklist(&env, AuditType::Security, auditor.clone());

            // Update first audit item
            let item_id = String::from_str(&env, "SEC_001");
            let notes = String::from_str(&env, "Access control review completed");
            let evidence = String::from_str(&env, "Evidence file: access_control_review.pdf");

            let result = AuditManager::update_audit_item(
                &env,
                &AuditType::Security,
                &item_id,
                AuditStatus::Completed,
                Some(notes),
                Some(evidence),
            );
            assert!(result.is_ok());

            // Verify item was updated
            let checklist = AuditManager::get_audit_checklist(&env, &AuditType::Security).unwrap();
            let updated_item = checklist
                .items
                .iter()
                .find(|item| item.id == item_id)
                .unwrap();

            assert_eq!(updated_item.status, AuditStatus::Completed);
            assert!(updated_item.notes.is_some());
            assert!(updated_item.evidence.is_some());
        });
    }

    #[test]
    fn test_audit_status_tracking() {
        let env = create_test_env();
        let auditor = Address::generate(&env);

        with_contract_context(&env, || {
            // Initialize audit system
            let _ = AuditManager::initialize(&env);

            // Create security audit checklist
            let _ =
                AuditManager::create_audit_checklist(&env, AuditType::Security, auditor.clone());

            // Get audit status
            let status = AuditManager::get_audit_status(&env);
            assert!(status.is_ok());

            let status_map = status.unwrap();
            assert!(!status_map.is_empty());

            // Verify security audit status is tracked
            let security_status = status_map.get(String::from_str(&env, "security_status"));
            assert!(security_status.is_some());
        });
    }

    #[test]
    fn test_audit_completion_validation() {
        let env = create_test_env();
        let auditor = Address::generate(&env);

        with_contract_context(&env, || {
            // Initialize audit system
            let _ = AuditManager::initialize(&env);

            // Create security audit checklist
            let checklist =
                AuditManager::create_audit_checklist(&env, AuditType::Security, auditor.clone())
                    .unwrap();

            // Initially, audit should not be complete
            let is_complete = AuditManager::validate_audit_completion(&env, &checklist);
            assert!(is_complete.is_ok());
            assert!(!is_complete.unwrap());

            // Complete all critical items
            let mut critical_items = Vec::new(&env);
            for item in checklist.items.iter() {
                if item.severity == AuditSeverity::Critical {
                    critical_items.push_back(item.clone());
                }
            }

            for item in critical_items.iter() {
                let _ = AuditManager::update_audit_item(
                    &env,
                    &AuditType::Security,
                    &item.id,
                    AuditStatus::Completed,
                    Some(String::from_str(&env, "Completed")),
                    Some(String::from_str(&env, "Evidence")),
                );
            }

            // Get updated checklist
            let updated_checklist =
                AuditManager::get_audit_checklist(&env, &AuditType::Security).unwrap();

            // Now validation should pass for critical items
            let is_complete = AuditManager::validate_audit_completion(&env, &updated_checklist);
            assert!(is_complete.is_ok());
        });
    }

    #[test]
    fn test_audit_configuration_update() {
        let env = create_test_env();

        with_contract_context(&env, || {
            // Initialize audit system
            let _ = AuditManager::initialize(&env);

            // Create new configuration
            let new_config = AuditConfig {
                environment: String::from_str(&env, "mainnet"),
                required_audits: vec![&env, AuditType::Security, AuditType::Deployment],
                critical_threshold: 0,
                high_threshold: 1,
                medium_threshold: 3,
                auto_approve_low: true,
                require_evidence: true,
                max_audit_duration: 14 * 24 * 60 * 60, // 14 days
            };

            // Update configuration
            let result = AuditManager::update_config(&env, &new_config);
            assert!(result.is_ok());

            // Verify configuration was updated
            let config = AuditManager::get_config(&env).unwrap();
            assert_eq!(config.environment, String::from_str(&env, "mainnet"));
            assert_eq!(config.high_threshold, 1);
            assert_eq!(config.auto_approve_low, true);
        });
    }

    #[test]
    fn test_audit_testing_utilities() {
        let env = create_test_env();

        with_contract_context(&env, || {
            // Test creating test audit checklist
            let test_checklist =
                AuditTesting::create_test_audit_checklist(&env, AuditType::Security);
            assert!(test_checklist.is_ok());

            // Test creating test audit item
            let test_item = AuditTesting::create_test_audit_item(&env, "TEST_001", "Test Item");
            assert_eq!(test_item.id, String::from_str(&env, "TEST_001"));
            assert_eq!(test_item.title, String::from_str(&env, "Test Item"));
            assert_eq!(test_item.severity, AuditSeverity::Medium);
            assert_eq!(test_item.status, AuditStatus::NotStarted);

            // Test simulating audit completion
            let _ = AuditManager::create_audit_checklist(
                &env,
                AuditType::Security,
                Address::generate(&env),
            );
            let result = AuditTesting::simulate_audit_completion(&env, &AuditType::Security);
            assert!(result.is_ok());
        });
    }

    #[test]
    fn test_audit_severity_levels() {
        let _env = create_test_env();

        // Test severity level ordering
        assert!(AuditSeverity::Critical > AuditSeverity::High);
        assert!(AuditSeverity::High > AuditSeverity::Medium);
        assert!(AuditSeverity::Medium > AuditSeverity::Low);
        assert!(AuditSeverity::Low > AuditSeverity::Info);

        // Test severity equality
        assert_eq!(AuditSeverity::Critical, AuditSeverity::Critical);
        assert_ne!(AuditSeverity::Critical, AuditSeverity::High);
    }

    #[test]
    fn test_audit_status_transitions() {
        let _env = create_test_env();

        // Test status equality
        assert_eq!(AuditStatus::NotStarted, AuditStatus::NotStarted);
        assert_eq!(AuditStatus::InProgress, AuditStatus::InProgress);
        assert_eq!(AuditStatus::Completed, AuditStatus::Completed);
        assert_eq!(AuditStatus::Failed, AuditStatus::Failed);
        assert_eq!(AuditStatus::Skipped, AuditStatus::Skipped);

        // Test status inequality
        assert_ne!(AuditStatus::NotStarted, AuditStatus::Completed);
        assert_ne!(AuditStatus::InProgress, AuditStatus::Failed);
    }

    #[test]
    fn test_audit_type_enum() {
        let _env = create_test_env();

        // Test audit type equality
        assert_eq!(AuditType::Security, AuditType::Security);
        assert_eq!(AuditType::CodeReview, AuditType::CodeReview);
        assert_eq!(AuditType::Testing, AuditType::Testing);
        assert_eq!(AuditType::Documentation, AuditType::Documentation);
        assert_eq!(AuditType::Deployment, AuditType::Deployment);
        assert_eq!(AuditType::Comprehensive, AuditType::Comprehensive);

        // Test audit type inequality
        assert_ne!(AuditType::Security, AuditType::CodeReview);
        assert_ne!(AuditType::Testing, AuditType::Documentation);
    }

    #[test]
    fn test_audit_item_structure() {
        let env = create_test_env();
        let auditor = Address::generate(&env);
        let timestamp = env.ledger().timestamp();

        let item = AuditItem {
            id: String::from_str(&env, "TEST_001"),
            title: String::from_str(&env, "Test Audit Item"),
            description: String::from_str(&env, "Test description"),
            severity: AuditSeverity::High,
            status: AuditStatus::InProgress,
            notes: Some(String::from_str(&env, "Test notes")),
            evidence: Some(String::from_str(&env, "Test evidence")),
            auditor: Some(auditor.clone()),
            timestamp,
        };

        assert_eq!(item.id, String::from_str(&env, "TEST_001"));
        assert_eq!(item.title, String::from_str(&env, "Test Audit Item"));
        assert_eq!(item.severity, AuditSeverity::High);
        assert_eq!(item.status, AuditStatus::InProgress);
        assert!(item.notes.is_some());
        assert!(item.evidence.is_some());
        assert_eq!(item.auditor, Some(auditor));
    }

    #[test]
    fn test_audit_checklist_structure() {
        let env = create_test_env();
        let auditor = Address::generate(&env);
        let timestamp = env.ledger().timestamp();

        let items = Vec::new(&env);
        let checklist = AuditChecklist {
            audit_type: AuditType::Security,
            version: String::from_str(&env, "1.0.0"),
            created_at: timestamp,
            updated_at: timestamp,
            auditor: auditor.clone(),
            items: items.clone(),
            overall_status: AuditStatus::NotStarted,
            completion_percentage: 0,
            critical_issues: 0,
            high_issues: 0,
            medium_issues: 0,
            low_issues: 0,
        };

        assert_eq!(checklist.audit_type, AuditType::Security);
        assert_eq!(checklist.version, String::from_str(&env, "1.0.0"));
        assert_eq!(checklist.auditor, auditor);
        assert_eq!(checklist.overall_status, AuditStatus::NotStarted);
        assert_eq!(checklist.completion_percentage, 0);
    }

    #[test]
    fn test_audit_finding_structure() {
        let env = create_test_env();

        let finding = AuditFinding {
            id: String::from_str(&env, "FINDING_001"),
            title: String::from_str(&env, "Test Finding"),
            description: String::from_str(&env, "Test finding description"),
            severity: AuditSeverity::High,
            category: String::from_str(&env, "Security"),
            file_location: Some(String::from_str(&env, "src/contract.rs")),
            line_number: Some(42),
            recommendation: String::from_str(&env, "Fix this issue"),
            status: AuditStatus::Failed,
            evidence: Some(String::from_str(&env, "Evidence file")),
        };

        assert_eq!(finding.id, String::from_str(&env, "FINDING_001"));
        assert_eq!(finding.title, String::from_str(&env, "Test Finding"));
        assert_eq!(finding.severity, AuditSeverity::High);
        assert_eq!(finding.category, String::from_str(&env, "Security"));
        assert!(finding.file_location.is_some());
        assert_eq!(finding.line_number, Some(42));
        assert_eq!(finding.status, AuditStatus::Failed);
    }

    #[test]
    fn test_audit_report_structure() {
        let env = create_test_env();
        let auditor = Address::generate(&env);
        let approver = Address::generate(&env);
        let timestamp = env.ledger().timestamp();

        let checklist = AuditChecklist {
            audit_type: AuditType::Security,
            version: String::from_str(&env, "1.0.0"),
            created_at: timestamp,
            updated_at: timestamp,
            auditor: auditor.clone(),
            items: Vec::new(&env),
            overall_status: AuditStatus::Completed,
            completion_percentage: 100,
            critical_issues: 0,
            high_issues: 0,
            medium_issues: 0,
            low_issues: 0,
        };

        let findings = Vec::new(&env);
        let recommendations = Vec::new(&env);

        let report = AuditReport {
            audit_id: String::from_str(&env, "AUDIT_001"),
            audit_type: AuditType::Security,
            auditor: auditor.clone(),
            created_at: timestamp,
            completed_at: Some(timestamp),
            checklist: checklist.clone(),
            findings: findings.clone(),
            recommendations: recommendations.clone(),
            risk_score: 25,
            approved: true,
            approver: Some(approver.clone()),
        };

        assert_eq!(report.audit_id, String::from_str(&env, "AUDIT_001"));
        assert_eq!(report.audit_type, AuditType::Security);
        assert_eq!(report.auditor, auditor);
        assert_eq!(report.risk_score, 25);
        assert!(report.approved);
        assert_eq!(report.approver, Some(approver));
    }

    #[test]
    fn test_audit_config_structure() {
        let env = create_test_env();

        let config = AuditConfig {
            environment: String::from_str(&env, "testnet"),
            required_audits: vec![&env, AuditType::Security, AuditType::Testing],
            critical_threshold: 0,
            high_threshold: 1,
            medium_threshold: 3,
            auto_approve_low: false,
            require_evidence: true,
            max_audit_duration: 7 * 24 * 60 * 60,
        };

        assert_eq!(config.environment, String::from_str(&env, "testnet"));
        assert_eq!(config.critical_threshold, 0);
        assert_eq!(config.high_threshold, 1);
        assert_eq!(config.medium_threshold, 3);
        assert!(!config.auto_approve_low);
        assert!(config.require_evidence);
        assert_eq!(config.max_audit_duration, 7 * 24 * 60 * 60);
    }
}
