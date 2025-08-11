# Predictify Contracts Mainnet Deployment Guide

> **Platform:** Stellar Soroban Mainnet  
> **Audience:** Developers, DevOps, and maintainers deploying Predictify contracts to production

---

## 📋 Table of Contents
1. [Project Summary](#project-summary)  
2. [API Documentation](#api-documentation)  
3. [Prerequisites](#prerequisites)  
4. [Configuration for Mainnet](#configuration-for-mainnet)  
5. [Deployment Instructions](#deployment-instructions)  
6. [Oracle Setup](#oracle-setup)  
7. [Testing Deployment](#testing-deployment)  
8. [Monitoring and Alerts](#monitoring-and-alerts)  
9. [Security Checklist](#security-checklist)  
10. [Rollback Procedures](#rollback-procedures)  
11. [Maintenance Procedures](#maintenance-procedures)

---

## 🧠 Project Summary
This repository contains smart contracts for Stellar's Soroban platform, organized in a Rust workspace. Key components include:

- `predictify-hybrid`: The main prediction market contract with oracle integration.
- `predictify-hybrid`: A hybrid prediction market with oracle integration (Reflector, Pyth), staking, dispute resolution, and community voting.

---

## 📚 Documentation

For comprehensive documentation, please refer to our organized documentation structure:

**📖 [Documentation Index](./docs/README.md)**

### 🚀 API Documentation
Complete API reference and integration guides: **[API Documentation](./docs/api/API_DOCUMENTATION.md)**

### ⛽ Gas Optimization
Performance optimization and cost analysis: **[Gas Documentation](./docs/gas/)**

### 🔒 Security
Security audits, best practices, and threat analysis: **[Security Documentation](./docs/security/)**

### 🛠️ Operations
Deployment, maintenance, and incident management: **[Operations Documentation](./docs/operations/)**

---



---

## 🛠️ Prerequisites
- [Rust](https://www.rust-lang.org/tools/install)
- [Soroban CLI](https://github.com/stellar/soroban-tools)
- Stellar-funded deployer account (mainnet)
- Admin account (preferably multisig-secured)

Install tools:
```bash
rustup update
cargo install --locked --version 20.0.0 soroban-cli
```

---

## ⚙️ Configuration for Mainnet

Add Stellar mainnet config:
```bash
soroban config network add mainnet \
  --rpc-url https://rpc.mainnet.stellar.org:443 \
  --network-passphrase "Public Global Stellar Network ; September 2015"
```

Create a `.env.mainnet` file:
```env
NETWORK=mainnet
DEPLOYER_SECRET_KEY="SB..."
ADMIN_ADDRESS="GB..."
ORACLE_CONTRACT="..."
```

---

## 🚀 Deployment Instructions

### Build Contracts
```bash
cd contracts/predictify-hybrid
make build
```

### Deploy to Mainnet
```bash
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/predictify_hybrid.wasm \
  --network $NETWORK \
  --source $DEPLOYER_SECRET_KEY
```

### Initialize Contract
```bash
soroban contract invoke \
  --id <contract_id> \
  --fn initialize \
  --network $NETWORK \
  --source $DEPLOYER_SECRET_KEY \
  --arg admin=$ADMIN_ADDRESS
```

Record and store the contract ID securely.

---

## 🔮 Oracle Setup

### Oracle Options
- Primary support: Reflector Oracle
- Others: Pyth or custom signed payloads

### Setup Steps
1. Deploy the oracle contract (if required).
2. Ensure oracle contract ID is stored in the main contract via admin call.
3. Off-chain oracle should:
   - Sign market outcomes
   - Submit results via `fetch_oracle_result()` or similar entrypoints

Oracle JSON Payload Example:
```json
{
  "market_id": "001",
  "result": "yes",
  "timestamp": "2025-07-04T12:00:00Z"
}
```

---

## 🧪 Testing Deployment

### Unit and Integration Tests
```bash
make test
```

### Dry-Run on Futurenet
```bash
soroban config network use futurenet
soroban contract deploy ...
```

### Post-Mainnet Checks
- Use `soroban contract inspect` to verify deployment
- Validate end-to-end market creation, voting, oracle submission, and claiming

---

## 📊 Monitoring and Alerts

### Tools:
- [Stellar Expert](https://stellar.expert/explorer/public)
- Custom CLI scripts to watch tx status
- Error tracking via logs + alerting via Slack/Discord/Webhooks

### Metrics to Monitor:
- Oracle submission frequency and failures
- Market volume anomalies
- Dispute activations and unresolved markets

---

## 🔐 Security Checklist

### ✅ Account Security
- [ ] Admin/deployer keys stored in hardware wallets or secure key vaults
- [ ] Avoid deploying from hot wallets or CLI-stored keys
- [ ] Multisig setup for critical contract ownership (if supported)

### ✅ Smart Contract Safeguards
- [ ] All admin functions require `require_auth(admin)`
- [ ] Oracle IDs must be validated against allowlist
- [ ] Reentrancy protected via Soroban execution model
- [ ] Input sanitization for all string, numeric, and enum arguments
- [ ] Dispute logic isolated from oracle resolution path
- [ ] `initialize()` callable once only; enforce init guard

### ✅ Network and Deployment
- [ ] Contract ID recorded and versioned
- [ ] Use Soroban’s `--network` config to prevent misdeployments
- [ ] Securely store and manage all `.env` files
- [ ] Validate deployed WASM checksum matches build artifact

---

## 🔁 Rollback Procedures
- Use pausable logic if available (e.g., freeze all markets via admin call)
- Deploy new contract instance if bug is unpatchable
- Migrate state via admin oracles (if implemented)
- Revoke oracle privileges for breached sources

---

## 🛠️ Maintenance Procedures
- Monitor oracle reliability and submission cadence
- Add/remove oracles via controlled admin processes
- Periodically test and patch contracts via redeployments
- Log usage metrics for governance and market integrity
- Respond to disputes in <48 hours using automated + manual review

---

## 📎 Suggested Enhancements
- GitHub Actions for CI + testnet deploy
- Soroban integration test suite with mocked oracles
- Publish deployed contract IDs in README
- Oracle dashboard or visual monitor tool (Grafana, etc.)



For deployment support or technical questions, please open an issue or contact the Predictify core team.

## License
This project is open source and available under the MIT License.

