# Predictify Contracts

> **Platform:** Stellar Soroban  
> **Language:** Rust  
> **License:** MIT

## 📖 About

Predictify Contracts is a comprehensive prediction market platform built on Stellar's Soroban smart contract platform. The system enables users to create and participate in prediction markets with hybrid resolution mechanisms that combine real-time oracle data (from Pyth Network and Reflector oracles) with community voting. This platform is designed for developers, traders, and organizations looking to build decentralized prediction markets with institutional-grade oracle integration, dispute resolution, and governance features.

The project is ideal for developers building on Stellar, smart contract auditors, and teams creating prediction market applications that require reliable price feeds, community governance, and robust dispute resolution mechanisms.

---

## 📋 Table of Contents
1. [Quick Start](#-quick-start)
2. [Project Structure](#-project-structure)
3. [Setup Instructions](#-setup-instructions)
4. [Development](#-development)
5. [Testing](#-testing)
6. [Deployment](#-deployment)
7. [Documentation](#-documentation)
8. [Contributing](#-contributing)

---

## 🚀 Quick Start

```bash
# Clone the repository
git clone <repository-url>
cd predictify-contracts

# Install dependencies (Rust and Soroban CLI)
# See Setup Instructions below

# Build contracts
cd contracts/predictify-hybrid
make build

# Run tests
make test
```

---

## 📁 Project Structure

```
predictify-contracts/
├── contracts/
│   ├── hello-world/          # Example contract for reference
│   └── predictify-hybrid/    # Main prediction market contract
│       ├── src/              # Contract source code
│       └── Makefile          # Build and test commands
├── docs/                     # Comprehensive documentation
│   ├── api/                  # API reference
│   ├── contracts/            # Contract-specific docs
│   ├── gas/                  # Gas optimization guides
│   ├── operations/           # Deployment and operations
│   └── security/             # Security documentation
├── Cargo.toml                # Workspace configuration
└── README.md                 # This file
```

**Key Components:**
- **`predictify-hybrid`**: Main prediction market contract with oracle integration, voting, disputes, and governance
- **`hello-world`**: Reference contract demonstrating basic Soroban patterns

---

## 🛠️ Setup Instructions

### Requirements

- **Rust** (latest stable version)
- **Soroban CLI** (version 20.0.0 or later)
- **Stellar Account** (for deployment and testing)
- **Git** (for cloning the repository)

### Installation

#### 1. Install Rust

```bash
# Install Rust using rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Verify installation
rustc --version
cargo --version
```

#### 2. Install Soroban CLI

```bash
# Install Soroban CLI
cargo install --locked --version 20.0.0 soroban-cli

# Verify installation
soroban --version
```

#### 3. Install Stellar Contract Tools

```bash
# Install Stellar contract build tools
cargo install --locked stellar-cli

# Verify installation
stellar --version
```

### Environment Setup

#### Configure Stellar Networks

```bash
# Add testnet (recommended for development)
soroban config network add testnet \
  --rpc-url https://soroban-testnet.stellar.org:443 \
  --network-passphrase "Test SDF Network ; September 2015"

# Add futurenet (for testing new features)
soroban config network add futurenet \
  --rpc-url https://rpc-futurenet.stellar.org:443 \
  --network-passphrase "Test SDF Future Network ; October 2022"

# Add mainnet (for production deployment)
soroban config network add mainnet \
  --rpc-url https://rpc.mainnet.stellar.org:443 \
  --network-passphrase "Public Global Stellar Network ; September 2015"
```

#### Set Default Network

```bash
# Use testnet for development
soroban config network use testnet
```

#### Environment Variables

Create a `.env` file in the project root (or `.env.testnet` for testnet):

```bash
# .env.testnet
NETWORK=testnet
DEPLOYER_SECRET_KEY="SB..."  # Your deployer account secret key
ADMIN_ADDRESS="GB..."        # Admin account address
ORACLE_CONTRACT="..."        # Oracle contract address (optional for testing)
```

**⚠️ Security Note:** Never commit `.env` files to version control. Add them to `.gitignore`.

### Build the Project

```bash
# From project root
cd contracts/predictify-hybrid

# Build the contract
make build

# Or use cargo directly
cargo build --target wasm32-unknown-unknown --release
```

The compiled WASM file will be at:
```
target/wasm32-unknown-unknown/release/predictify_hybrid.wasm
```

---

## 🧪 Testing

### Run All Tests

```bash
# From contract directory
cd contracts/predictify-hybrid
make test

# Or use cargo directly
cargo test
```

### Run Specific Test Suites

```bash
# Unit tests only
cargo test --lib

# Integration tests
cargo test --test integration_test

# Property-based tests
cargo test --test property_based_tests
```

### Test on Testnet

```bash
# Deploy to testnet for integration testing
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/predictify_hybrid.wasm \
  --network testnet \
  --source $DEPLOYER_SECRET_KEY

# Initialize the contract
soroban contract invoke \
  --id <contract_id> \
  --fn initialize \
  --network testnet \
  --source $DEPLOYER_SECRET_KEY \
  --arg admin=$ADMIN_ADDRESS
```

---

## 🚀 Deployment

### Deploy to Testnet (Recommended First Step)

```bash
# Ensure you're on testnet
soroban config network use testnet

# Deploy contract
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/predictify_hybrid.wasm \
  --network testnet \
  --source $DEPLOYER_SECRET_KEY

# Save the contract ID from the output
CONTRACT_ID="<contract_id_from_output>"

# Initialize contract
soroban contract invoke \
  --id $CONTRACT_ID \
  --fn initialize \
  --network testnet \
  --source $DEPLOYER_SECRET_KEY \
  --arg admin=$ADMIN_ADDRESS
```

### Deploy to Mainnet

**⚠️ Production Deployment Checklist:**

- [ ] All tests passing
- [ ] Security audit completed
- [ ] Testnet deployment verified
- [ ] Admin keys secured (preferably multisig)
- [ ] Oracle contracts configured
- [ ] Monitoring setup ready

```bash
# Switch to mainnet
soroban config network use mainnet

# Deploy contract
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/predictify_hybrid.wasm \
  --network mainnet \
  --source $DEPLOYER_SECRET_KEY

# Initialize contract
soroban contract invoke \
  --id $CONTRACT_ID \
  --fn initialize \
  --network mainnet \
  --source $DEPLOYER_SECRET_KEY \
  --arg admin=$ADMIN_ADDRESS

# Record contract ID securely
echo "Mainnet Contract ID: $CONTRACT_ID" >> deployment.log
```

### Verify Deployment

```bash
# Inspect deployed contract
soroban contract inspect \
  --id $CONTRACT_ID \
  --network $NETWORK
```

---

## 📚 Documentation

### Quick Links

- **[📖 Documentation Index](./docs/README.md)** - Complete documentation overview
- **[🚀 API Documentation](./docs/api/API_DOCUMENTATION.md)** - Complete API reference and integration guides
- **[📋 Contract README](./contracts/predictify-hybrid/README.md)** - Detailed contract documentation

### Documentation Categories

#### 🔒 Security
- **[Security Best Practices](./docs/security/SECURITY_BEST_PRACTICES.md)** - Development security guidelines
- **[Attack Vectors](./docs/security/ATTACK-VECTORS.md)** - Known threats and mitigations
- **[Audit Checklist](./docs/security/AUDIT_CHECKLIST.md)** - Security audit requirements
- **[Security Testing Guide](./docs/security/SECURITY_TESTING_GUIDE.md)** - Security testing procedures

#### ⛽ Gas Optimization
- **[Gas Optimization](./docs/gas/GAS_OPTIMIZATION.md)** - Optimization strategies
- **[Gas Cost Analysis](./docs/gas/GAS_COST_ANALYSIS.md)** - Detailed cost breakdown
- **[Gas Monitoring](./docs/gas/GAS_MONITORING.md)** - Monitoring tools and techniques
- **[Gas Benchmarking](./docs/gas/GAS_BENCHMARKING.md)** - Performance benchmarks

#### 🛠️ Operations
- **[Incident Response](./docs/operations/INCIDENT_RESPONSE.md)** - Incident management procedures

#### 📋 Contracts
- **[Types System](./docs/contracts/TYPES_SYSTEM.md)** - Data structures and types
- **[Voting System](./docs/contracts/VOTING_SYSTEM.md)** - Voting and dispute resolution

---

## 🤝 Contributing

We welcome contributions! Here's how to get started:

### Development Workflow

1. **Fork the repository**
2. **Create a feature branch**
   ```bash
   git checkout -b feature/your-feature-name
   ```
3. **Make your changes**
   - Follow Rust formatting: `cargo fmt --all`
   - Ensure tests pass: `make test`
   - Update documentation as needed
4. **Commit your changes**
   ```bash
   git commit -m "Add: description of your changes"
   ```
5. **Push and create a Pull Request**

### Code Standards

- **Formatting**: Use `cargo fmt --all` before committing
- **Testing**: All new features must include tests
- **Documentation**: Update relevant docs for new features
- **Security**: Review security implications of changes

### Contribution Guidelines

- **Issues**: Use GitHub issues for bug reports and feature requests
- **Pull Requests**: Include description of changes and test results
- **Code Review**: All PRs require review before merging
- **Testing**: Ensure all tests pass and add tests for new features

### Getting Help

- **Documentation**: Check the [docs](./docs/) directory
- **Issues**: Search existing issues or create a new one
- **Discussions**: Use GitHub Discussions for questions

---

## 🔮 Oracle Setup

The Predictify Hybrid contract supports multiple oracle providers for price feeds:

### Supported Oracles

- **Pyth Network**: Institutional-grade price feeds with high-frequency updates (400ms)
- **Reflector Oracle**: Stellar-native oracle with proven track record
- **Custom Oracles**: Extensible architecture for additional providers

### Oracle Configuration

See the [Contract README](./contracts/predictify-hybrid/README.md) for detailed oracle setup instructions and integration examples.

---

## 🔍 Development

### Code Formatting

```bash
# Format all code
cargo fmt --all

# Or from contract directory
cd contracts/predictify-hybrid
make fmt
```

### Running Tests

```bash
# Run all tests
make test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_name
```

### Project Structure

- **`contracts/predictify-hybrid/src/`**: Main contract source code
  - `lib.rs`: Contract entry point and main implementation
  - `markets.rs`: Market creation and management
  - `oracles.rs`: Oracle integration (Pyth, Reflector)
  - `voting.rs`: Community voting system
  - `disputes.rs`: Dispute resolution mechanism
  - `resolution_delay.rs`: **Resolution delay and dispute window system**
  - `governance.rs`: Governance and admin functions
  - `types.rs`: Core data structures
  - `errors.rs`: Error definitions

### Resolution Delay and Dispute Window

The platform implements a mandatory dispute window between market resolution and payout distribution. This allows community members to challenge resolution outcomes before payouts become final.

**Key Features:**

- **Configurable Window Duration**: Admin can set global (1-168 hours) or per-market dispute windows
- **Proposal-Based Resolution**: Resolution is proposed, not immediately finalized
- **Dispute Integration**: Disputes can only be filed during the open window
- **Finalization Gate**: Payouts blocked until window closes and disputes resolved

**Resolution Flow:**

```
Market Ends → Resolution Proposed → Dispute Window Open → Window Closes → Finalized → Payouts
                                          ↓
                                   Disputes Filed → Disputes Resolved ↗
```

**API Functions:**

```rust
// Set global dispute window (admin)
set_dispute_window_duration(env, admin, hours, None);

// Set per-market dispute window (admin)
set_dispute_window_duration(env, admin, hours, Some(market_id));

// Propose resolution (opens dispute window)
propose_market_resolution(env, admin, market_id) -> String;

// Finalize after window closes
finalize_market_resolution(env, market_id) -> String;

// Query dispute window status
get_dispute_window_status(env, market_id) -> (is_open, remaining_seconds, dispute_count);

// File dispute during window
dispute_during_window(env, user, market_id, stake, reason);

// Emergency admin override
force_finalize_resolution(env, admin, market_id, outcome);
```

**Claim Winnings Protection:**

The `claim_winnings` function now requires resolution to be finalized before allowing payouts:

```rust
// This will fail if resolution is not finalized
claim_winnings(env, user, market_id);
// Error::ResolutionNotFinalized if dispute window still open
```

### Adding New Features

1. Create feature branch
2. Implement feature with tests
3. Update documentation
4. Run full test suite
5. Submit PR with description

---

## 📊 Monitoring

### Tools

- **[Stellar Expert](https://stellar.expert/explorer/public)**: Blockchain explorer
- **Soroban CLI**: Contract inspection and interaction
- **Custom Scripts**: Monitor transactions and events

### Key Metrics

- Oracle submission frequency and reliability
- Market creation and resolution rates
- Dispute activations and resolution times
- Gas costs and optimization opportunities

---

## 🔐 Security

### Security Best Practices

- Review [Security Documentation](./docs/security/) before deployment
- Complete security audit checklist
- Use hardware wallets for admin keys
- Implement multisig for critical operations
- Monitor for suspicious activity

### Reporting Security Issues

**⚠️ Do not open public issues for security vulnerabilities.**

Please report security issues privately to the maintainers. See [Security Best Practices](./docs/security/SECURITY_BEST_PRACTICES.md) for details.

---

## 📝 License

This project is open source and available under the **MIT License**.

---

## 🔗 Additional Resources

- **[Stellar Documentation](https://developers.stellar.org/)**: Stellar platform docs
- **[Soroban Documentation](https://soroban.stellar.org/docs)**: Soroban smart contract platform
- **[Pyth Network](https://pyth.network/)**: Oracle provider documentation
- **[Stellar Developer Discord](https://discord.gg/stellar)**: Community support

---

## 💬 Support

- **Issues**: [GitHub Issues](https://github.com/your-org/predictify-contracts/issues)
- **Documentation**: [Documentation Index](./docs/README.md)
- **Questions**: Use GitHub Discussions

---

*Last updated: 2025*

