# 📚 Predictify Contracts Documentation

Welcome to the Predictify Contracts documentation! This directory contains comprehensive documentation for the Predictify Hybrid prediction market smart contract system.

## 📁 Documentation Structure

### 🚀 [API Documentation](./api/API_DOCUMENTATION.md)

Complete API reference for Predictify Hybrid contract, including:
- Contract functions and their parameters
- Return types and error handling
- Integration examples
- Client usage patterns
- **ReflectorAsset Coverage Matrix** - Comprehensive asset testing and validation
- **Token and Asset Management** - Multi-asset and SAC Token support documentation
- **[Query Implementation Guide](./api/QUERY_IMPLEMENTATION_GUIDE.md)** - Paginated query API, `PagedResult<T>`, security notes, and integrator quick-start
  - **NEW**: [Dashboard Statistics Queries](#-dashboard-statistics-queries) - Platform aggregates, market metrics, leaderboards with stable field versioning

### 🔒 [Security Documentation](./security/)

Comprehensive security documentation and guidelines:

- **[Attack Vectors](./security/ATTACK-VECTORS.md)** - Known attack vectors and mitigation strategies
- **[Audit Checklist](./security/AUDIT_CHECKLIST.md)** - Security audit requirements and checklist
- **[Soroban SDK Workspace Audit](./security/SOROBAN_SDK_AUDIT.md)** - Workspace Soroban SDK target, verification steps, and audit notes for Protocol 25 alignment
- **[Security Best Practices](./security/SECURITY_BEST_PRACTICES.md)** - Development and deployment security guidelines
- **[Security Considerations](./security/SECURITY_CONSIDERATIONS.md)** - Important security considerations for the system
- **[Security Testing Guide](./security/SECURITY_TESTING_GUIDE.md)** - Executable security checklist mapped to automated tests

### ⛽ [Gas Optimization Documentation](./gas/)

Complete gas optimization and cost analysis:

- **[Gas Benchmarking](./gas/GAS_BENCHMARKING.md)** - Performance benchmarks and metrics
- **[Gas Case Studies](./gas/GAS_CASE_STUDIES.md)** - Real-world gas optimization examples
- **[Gas Cost Analysis](./gas/GAS_COST_ANALYSIS.md)** - Detailed cost breakdown and analysis
- **[Gas Monitoring](./gas/GAS_MONITORING.md)** - Tools and techniques for monitoring gas usage
- **[Gas Optimization](./gas/GAS_OPTIMIZATION.md)** - Strategies and best practices for gas optimization
- **[Gas Testing Guidelines](./gas/GAS_TESTING_GUIDELINES.md)** - Testing procedures for gas optimization
- **[Gas Troubleshooting](./gas/GAS_TROUBLESHOOTING.md)** - Common gas-related issues and solutions

### 🛠️ [Operations Documentation](./operations/)

Operational procedures and incident management:

- **[Incident Response](./operations/INCIDENT_RESPONSE.md)** - Incident response procedures and protocols

### 📋 [Contract Documentation](./contracts/)

Implementation-specific documentation for the Predictify Hybrid contract:

- **[Types System](./contracts/TYPES_SYSTEM.md)** - Comprehensive type system and data structures
- **[Voting System](./contracts/VOTING_SYSTEM.md)** - Voting mechanism and dispute resolution
- **[Balance Management](./contracts/BALANCES.md)** - Security invariants and token safety semantics for deposits and withdrawals
- **[Event Archive](./contracts/EVENT_ARCHIVE.md)** - Bounded archive storage, pagination strategy, and pruning API


### 💰 [Claims Documentation](./claims/)
Claim idempotency and payout tracking:

- **[Claim Idempotency Guide](./claims/CLAIM_IDEMPOTENCY.md)** - Comprehensive guide to idempotent winnings claims

## 🎯 Quick Start

1. **For Developers**: Start with [API Documentation](./api/API_DOCUMENTATION.md)
2. **For Dashboard Integrators**: Review [Dashboard Statistics Queries](#-dashboard-statistics-queries) in the Query Implementation Guide
3. **For Contract Contributors**: Review [Contract Documentation](./contracts/)
4. **For Security Auditors**: Review [Security Documentation](./security/)
5. **For Gas Optimization**: Check [Gas Optimization](./gas/GAS_OPTIMIZATION.md)
6. **For Operations**: Read [Incident Response](./operations/INCIDENT_RESPONSE.md)

## 📊 Dashboard Statistics Queries

**NEW**: The Query Implementation Guide now includes comprehensive [Dashboard Statistics Queries](./api/QUERY_IMPLEMENTATION_GUIDE.md#dashboard-statistics-queries-new) with:

- **Platform-Level Aggregates** - `get_dashboard_statistics()` for TVL, active users, and total metrics
- **Per-Market Metrics** - `get_market_statistics()` for consensus strength and volatility
- **Category Analytics** - `get_category_statistics()` for filtered market data
- **User Leaderboards** - `get_top_users_by_winnings()` and `get_top_users_by_win_rate()` for rankings

All response types use stable `V1` versioning for forward compatibility without breaking changes.

**Use cases**: Dashboard display, analytics filtering, leaderboard rendering, TVL tracking


## 🔗 Related Resources

- **[Main Project README](../README.md)** - Project overview and setup instructions
- **[Contracts Directory](../contracts/)** - Source code for all smart contracts
- **[Predictify Hybrid: Reproducible WASM Builds & Checksums](../contracts/predictify-hybrid/README.md#reproducible-wasm-builds--checksums)**
- **[GitHub Repository](https://github.com/your-org/predictify-contracts)** - Source code repository

## 📝 Contributing to Documentation

When adding new documentation:

1. **Choose the appropriate category** based on the content type
2. **Follow the naming convention** (UPPERCASE_WITH_UNDERSCORES.md)
3. **Update this index** to include the new document
4. **Add cross-references** to related documents where appropriate

## 🏷️ Documentation Categories

- **API**: Contract interfaces, function references, and integration guides
- **Contracts**: Implementation-specific documentation for contract systems
- **Security**: Security audits, best practices, and threat analysis
- **Gas**: Performance optimization, cost analysis, and monitoring
- **Operations**: Deployment, maintenance, and incident management

---

*Last updated: 2026-03-30*
*For questions or suggestions about documentation, please open an issue in the repository.*
