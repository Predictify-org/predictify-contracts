# Analysis of Vector Analysis and Mitigation
This document highlights attack vectors with mitigations.

## 1. SQL Injection
- A web security vulnerability that lets an attacker to change database of an organisation with malicious SQL queries injection.
- Using prepared statements and parameterized queries, can help in preventing SQL injection attacks.
- Using ORM(Object-Relational Mapping) Frameworks like Hibernate or Entity Framework can help in reducing SQL inject by default.

## 2. Cross-Site Request Forgery(CSRF)
- The attacker exploits the trust of a web application with a authenticated user.
- The identity of the victim is used to perform some actions.
**Prevention**:
- Use an anti-CSRF token or synchronizer token. 
- Add a CAPTCHA to validate that the use is human.

## 3. Insecure API Access
- The application can run into broken authentication when APIs are not validated.
- When APIs get too nmany requests from user, resource limit problems can occur. Fake requests can be sent by attackers to the servers.

**Prevention**
- Strong passwords implementation policy with two-step login can be implemented.
- Security standards can be applied to endpoints, like input validation, authentication, and authorization.

## 4. Insider Threat
- Threats originating within an organisation.
- These threats can increase property thefts, data breach and disturbing operations.
**Prevention**
- Training to detect and stop insider threats.
- Regularly implementating montitorization and access reviews.

## 5. Cross-Site Scripting(XSS)
- Malicious scripts are injected into trusted websites.
- Browser side script can be corrupted and send to different user.
**Prevention**
- Use approppriate headers, use Content-Type and X-Content-Type-Options, headers for interpretation of responses by the browsers.
- Use Content Security Policy(CSP) to prevent servere XSS vulnerabilities.

## 6. Misconfiguration
- When security settings are not defined in the configuration, security Misconfiguration occurs.
- Directory Listing should not be enabled with not showing error messages which is sensitive information.
**Prevention**
- Regularly perform scans and identify security misconfigurations.
- Encryption of data at rest to prevent exploittaion of data.