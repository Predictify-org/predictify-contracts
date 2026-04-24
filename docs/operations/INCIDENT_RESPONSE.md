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

## 5. Post-Incident Review
- Documentation of incident and prevention
- Notfication to regulatory bodies during a security breach

## 6. Unclaimed Winnings Timeout Policy
- Claiming window is enforced per resolved market.
- Default claim window: 90 days from recorded resolution time (legacy fallback uses `market.end_time`).
- Admin can set:
	- Global claim window: `set_global_claim_period(admin, claim_period_seconds)`
	- Market override: `set_market_claim_period(admin, market_id, claim_period_seconds)`
- Claims attempted after deadline revert with `ResolutionTimeoutReached`.

### Sweep Policy
- Sweep entrypoint: `sweep_unclaimed_winnings(admin, market_id, burn)`.
- Authorization: admin only.
- Preconditions:
	- Market must be resolved.
	- Claim window must be expired.
- Sweep scope:
	- Only unclaimed winning payouts are swept.
	- Already claimed payouts are excluded.

### Destination Modes
- `burn = false`: transfer swept amount to configured treasury address.
	- Treasury must be configured first via `set_treasury(admin, treasury)`.
- `burn = true`: burn mode (no treasury credit).

### Operational Runbook
1. Verify market is resolved and claim window has expired.
2. Decide destination mode (`burn` or treasury).
3. If treasury mode, verify treasury address is configured.
4. Execute sweep.
5. Verify emitted `UnclaimedWinningsSweptEvent` and resulting balances.

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

