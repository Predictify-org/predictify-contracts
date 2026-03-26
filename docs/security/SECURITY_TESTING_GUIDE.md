# Security Test Guide

## 1. Dependency Scanning
- Regularly check for source-code files with changes
- Check for compatibility and resolve performance issues

## 2. Penetration Testing
- Use Kali Linux and Burp Suite to identify vulnerabilities
- Use Wireshark to check network traffic

## 3. Dynamic Application Security Testing(DASP)
- DASP tools are used for identifying security misconfiguration, broken authentication and input/output validation
- ZED Attack Proxy is an open source tool for security testing provided by OWASP

## 4. Static Application Security Testing(SAST)
- Tools help in detecting SQL injections,and other vulnerabilities
- SonarQube, Fortify are commonly used tools
- Integrate with IDEs and CI/CD pipelines

## 5. Property-Based Testing (Proptest)
- Smart contract invariants (especially around financial logic like stake distributions, payouts, and fee deductions) are verified using property-based fuzzing.
- **Threat Model Covered**: Payout calculation overflow/underflow, rounding errors giving away more funds than total pooled, double-claim attacks, zero-winner scenarios, fee evasion.
- **Invariants Proven**:
  - `distribute_payouts`: Total distributed to all users is `total_pool` (minus fees/truncation) and mathematically proportional.
  - Payout is strictly zero when there are no winners.
  - Fees are deducted exactly according to the percentage configuration.
  - Double distributions and double claims result in zero extra payouts.
- **Explicit Non-Goals**: Property testing of off-chain components or exact sub-stroop distribution (small 1 stroop differences due to integer div truncation are securely kept in contract).
- **Execution**: Run with `cargo test -p predictify-hybrid --test property_based_tests`.
