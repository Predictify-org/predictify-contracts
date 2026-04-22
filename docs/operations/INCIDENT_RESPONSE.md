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
- Notfication to regulatory bodies during a security breach

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
- Monitor for anamolies everyday
- Validation of Configuration

## Communication
- Notification of critical updates to security teams

