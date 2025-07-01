# Soroban Project

This repository contains smart contracts built for the Stellar Soroban platform, organized as a Rust workspace. It includes both example and advanced contracts, with a focus on prediction markets and oracle integration.

---

## Project Structure

```text
.
├── contracts
│   ├── hello-world
│   │   ├── src
│   │   │   ├── lib.rs
│   │   │   └── test.rs
│   │   ├── Cargo.toml
│   │   └── Makefile
│   └── predictify-hybrid
│       ├── src
│       │   ├── lib.rs
│       │   └── test.rs
│       ├── Cargo.toml
│       ├── Makefile
│       └── README.md
├── Cargo.toml
└── README.md
```

- New Soroban contracts can be added in the `contracts` directory, each in their own subdirectory with its own `Cargo.toml`.
- All contracts share dependencies via the top-level workspace `Cargo.toml`.

---

## Contracts Overview

### 1. hello-world
A minimal example contract for Soroban, demonstrating basic contract structure and testing.

**Functionality:**
- Exposes a single function `hello(to: String) -> Vec<String>` that returns a greeting message.
- Includes a simple test in `test.rs`.

**Example:**
```rust
let words = client.hello(&String::from_str(&env, "Dev"));
// Returns: ["Hello", "Dev"]
```

**Build & Test:**
```bash
cd contracts/hello-world
make build   # Build the contract
make test    # Run tests
```

---

### 2. predictify-hybrid
A hybrid prediction market contract that integrates with real oracles (notably the Reflector Oracle) and supports community voting for market resolution. This contract is suitable for real-world prediction markets on Stellar.

**Key Features:**
- Real-time price feeds from the Reflector oracle contract
- Hybrid resolution: combines oracle data with community voting
- Multiple oracle support (Reflector, Pyth, and more)
- Dispute and staking system
- Fee structure (2% platform fee + creation fee)
- Security: authentication, authorization, input validation, and reentrancy protection

**Main Functions:**
- `initialize(admin: Address)`
- `create_reflector_market(...)` and `create_reflector_asset_market(...)`
- `create_pyth_market(...)`
- `fetch_oracle_result(market_id, oracle_contract)`
- `vote(user, market_id, outcome, stake)`
- `resolve_market(market_id)`
- `claim_winnings(user, market_id)`

**Example Usage:**
```javascript
// Create a BTC price prediction market using Reflector oracle
const marketId = await predictifyClient.create_reflector_market(
    adminAddress,
    "Will BTC price be above $50,000 by December 31, 2024?",
    ["yes", "no"],
    30, // days
    "BTC",
    5000000, // $50,000 in cents
    "gt"
);

// Users vote
await predictifyClient.vote(userAddress, marketId, "yes", 1000000000); // 100 XLM stake

// Fetch oracle result and resolve
const oracleResult = await predictifyClient.fetch_oracle_result(marketId, REFLECTOR_CONTRACT);
const finalResult = await predictifyClient.resolve_market(marketId);
```

**Build & Test:**
```bash
cd contracts/predictify-hybrid
make build   # Build the contract
make test    # Run tests
```

**Deployment:**
```bash
cargo build --target wasm32-unknown-unknown --release
soroban contract deploy --wasm target/wasm32-unknown-unknown/release/predictify_hybrid.wasm
soroban contract invoke --id <contract_id> -- initialize --admin <admin_address>
```

**Troubleshooting:**
- Ensure the Reflector oracle contract is accessible and the asset symbol is supported.
- Check network connectivity to the Stellar network.
- Review contract logs for oracle call errors.

---

## Workspace Build & Test

From the project root, you can build and test all contracts:

```bash
cargo build --workspace
cargo test --workspace
```

---

## Resources
- [Soroban Documentation](https://developers.stellar.org/docs/build/smart-contracts/overview)
- [Soroban Examples](https://github.com/stellar/soroban-examples)
- [Reflector Oracle](https://github.com/reflector-labs/reflector-oracle)

---

## License
This project is open source and available under the MIT License.
