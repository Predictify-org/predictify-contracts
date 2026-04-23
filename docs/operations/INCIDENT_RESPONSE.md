This documentation helps in detecting incident response procedures, provides insights for security monitoring and security updates process

## 1. Detection
- Alerts for traffic patterns
- Look for inappropriate user behavior or requests

## 2. Containment
- Isolation od affected systems and revoke access tokens and secrets
- Disable accounts affected

## 3. Eradication
- Detect for malicious code and patch the exploited vulnerability with clearing of logs

## 4. Recovery
- Keep backup for recovery

## Recovery Emergency Procedures (Predictify Hybrid)

- Only authorized admins may perform on-chain recovery actions; calls must be authenticated and auditable.
- Recovery actions MUST emit on-chain events containing: admin address, market_id, action, status, amount (when applicable), and timestamp.
- Prefer non-invasive heuristics: reconstruct totals, mark claims, or perform targeted partial refunds.
- Do not perform forced payouts or unilateral transfers unless approved and logged.
- After any recovery action: record a post-incident summary, include invariant checks performed, and list affected market IDs.

## Event Visibility and Auditing

- Indexers and auditors should monitor `recovery_evt` events emitted by the contract. Each event includes the acting admin and structured details for integrators to consume.
- Maintain an off-chain incident log correlated to on-chain event timestamps for forensic analysis.

## 5. Post-Incident Review
- Documentation of incident and prevention
- Notification to regulatory bodies during a security breach

# Guidelines for Security Monitoring

## Metrices to watch
- Look for unauthorised access endpoints
- Increase in user requests
- Failure in Login continuously

## Log Retention Policies
- Regularly check for logs and monitor them

## Tools and Frameworks
- Use tools like Datadog, Splunk, and ELK stack

## Montoring Targets
- Look for admin activities
- Health of system and access to database and authentications details

# Procedures for Security Updates

## Regular Updates
- Patching of OS and Apps regularly
- Merge dependencies with low risks

## Post-Update Verification
- Monitor for anomalies everyday
- Validation of Configuration

## Communication
- Notification of critical updates to security teams

# Specific Runbooks: Oracle Health Degradation

With the integration of graceful degradation in the Predictify Hybrid contracts, operators must actively monitor on-chain events to detect oracle failures, as the system no longer fails silently.

## 1. On-Chain Detection
Operators must configure their indexing services (e.g., Datadog, subgraph, or custom indexer) to listen for specific smart contract events emitted during oracle fallbacks or failures.
- **Event Topic/Signature:** Listen for the `OracleDegradationEvent` (typically keyed by the symbol `ora_deg`).
- **Payload Inspection:** The event payload will contain:
  - `oracle`: The `OracleProvider` that failed.
  - `reason`: A string indicating the failure context (e.g., "Primary oracle failed" or "Backup oracle failed").
  - `timestamp`: The ledger timestamp of the failure.

## 2. Active Monitoring Queries
In addition to passive event listening, monitoring targets should periodically poll the contract's health state.
- **Query `monitor_oracle_health`:** Schedule regular calls to this function for all active oracle providers.
- **Alerting Thresholds:** Set immediate alerts if the returned `OracleHealthMetrics` drops to `MonitoringStatus::Warning` or `MonitoringStatus::Critical`, or if the `confidence_score` drops below acceptable integration thresholds.

## 3. Incident Response Protocol for Oracles
- **Level 1 (Primary Failure):** If an event states "Primary oracle failed", the system has automatically routed to the fallback. **Action:** Log the incident, verify the fallback oracle's `confidence_score`, and investigate the primary provider's API/node health. No immediate containment is required as the contract handles this gracefully.
- **Level 2 (Total Failure):** If a subsequent event states "Backup oracle failed", the market resolution is blocked. **Action:** Immediate intervention is required. Operators may need to pause affected markets or trigger an emergency administrative resolution depending on the governance configuration.
