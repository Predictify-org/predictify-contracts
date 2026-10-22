# Predictify Contracts

Soroban smart contracts for **Predictify**, a hybrid prediction-market platform on Stellar. Users create markets with a question and outcomes, stake on an outcome, and winners claim payouts after oracle or admin resolution (with a dispute window and fee collection).

- Main contract: [`contracts/predictify-hybrid`](./contracts/predictify-hybrid) (markets, betting, oracles, disputes, fees, admin/roles)
- Supporting crates: `contracts/oracles`, `contracts/hello-world`
- SDK: `soroban-sdk 25`, Rust edition 2021

## Deployed (testnet)

| | |
|---|---|
| Network | Stellar **Testnet** (`Test SDF Network ; September 2015`) |
| Contract ID | `CCWRJXIUBSMFQ4AF2RXXH3AFBW6MEZG4GK4IYES7C4Z3X4PR6K7VAFHC` |
| Explorer | [stellar.expert](https://stellar.expert/explorer/testnet/contract/CCWRJXIUBSMFQ4AF2RXXH3AFBW6MEZG4GK4IYES7C4Z3X4PR6K7VAFHC) / [Stellar Lab](https://lab.stellar.org/r/testnet/contract/CCWRJXIUBSMFQ4AF2RXXH3AFBW6MEZG4GK4IYES7C4Z3X4PR6K7VAFHC) |
| Admin | `GBAW5Y35LFQJLHVIABW2CWGCRPZXKM7MN3UNAYWQXKDJI4LM2NYGWYZL` |
| Build | slim core build (see below) |

Full details, tx hashes and smoke-test commands: [docs/DEPLOYMENTS.md](./docs/DEPLOYMENTS.md).

> **Slim vs full build.** Stellar limits contract WASM to 128 KiB (131,072 bytes). The full contract (every entry point, default `full` feature) compiles to ~630 KB and cannot be deployed as one contract. The testnet deployment is built with `--no-default-features`, which keeps the core market lifecycle entry points (`initialize`, `create_market`, `create_event`, `vote`, `place_bet`, `resolve_market*`, `dispute_market`, `claim_winnings`, `distribute_payouts`, `collect_fees`, balances, queries) and strips doc strings and unused types from the embedded spec. Remaining entry points (analytics, recovery, monitoring, upgrades, ...) are gated behind the `full` feature and need to be split into separate contracts before they can be deployed.

## Quick start

```sh
# run the tests (697 pass, 203 known-failing are #[ignore]d, see docs/TESTING.md)
cargo test -p predictify-hybrid --lib

# build the deployable slim WASM (needs the wasm32v1-none target and stellar-cli)
rustup target add wasm32v1-none
scripts/build_testnet_wasm.sh        # -> target/wasm32v1-none/release/predictify_hybrid.slim.wasm
```

Deploy and try it on testnet:

```sh
stellar keys generate my-key --network testnet --fund
stellar contract deploy --wasm target/wasm32v1-none/release/predictify_hybrid.slim.wasm \
  --source my-key --network testnet
stellar contract invoke --id <CONTRACT_ID> --source my-key --network testnet -- \
  initialize --admin $(stellar keys address my-key) --platform_fee_percentage 2 --allowed_assets null
```

## Continuous integration

`.github/workflows/contract-ci.yml` builds the workspace and runs `hello-world`, `oracles` and the `predictify-hybrid` lib tests on every push/PR.

## Documentation

- [Docs index](./docs/README.md)
- [Deployments](./docs/DEPLOYMENTS.md)
- [Testing status](./docs/TESTING.md)
- [Predictify Hybrid contract guide](./contracts/predictify-hybrid/README.md)
- [Design notes and implementation write-ups](./docs/design/)
- [Soroban SDK workspace audit](./docs/security/SOROBAN_SDK_AUDIT.md)
