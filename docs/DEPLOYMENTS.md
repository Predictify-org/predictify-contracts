# Deployments

## Stellar Testnet

| Field | Value |
|---|---|
| Network | Testnet (`Test SDF Network ; September 2015`), RPC `https://soroban-testnet.stellar.org` |
| **Contract ID** | `CCWRJXIUBSMFQ4AF2RXXH3AFBW6MEZG4GK4IYES7C4Z3X4PR6K7VAFHC` |
| Explorer | https://stellar.expert/explorer/testnet/contract/CCWRJXIUBSMFQ4AF2RXXH3AFBW6MEZG4GK4IYES7C4Z3X4PR6K7VAFHC |
| Stellar Lab | https://lab.stellar.org/r/testnet/contract/CCWRJXIUBSMFQ4AF2RXXH3AFBW6MEZG4GK4IYES7C4Z3X4PR6K7VAFHC |
| Deploy tx | https://stellar.expert/explorer/testnet/tx/8ec2ad92f7cf236c46df942b4812d3d5619934e369d4a7f81c18f6763b9193c0 |
| Admin / deployer | `GBAW5Y35LFQJLHVIABW2CWGCRPZXKM7MN3UNAYWQXKDJI4LM2NYGWYZL` |
| Platform fee | 2% |
| WASM | slim build, 128,890 bytes (limit 131,072) |
| Initialized | yes (`initialize`, admin = deployer, fee 2, default assets) |

Sample market created as a smoke test: id `mkt_5cae97f9_0` ("Will BTC trade above $100,000 within 30 days?", outcomes `yes`/`no`), create tx https://stellar.expert/explorer/testnet/tx/5f4b41e96c859533432a95c8a0bf6a62bf1a7fd0b8e7d6cacb7b26b10ab40f69

The admin key is a fresh testnet-only key held by the deployer; it holds no value and is not used anywhere else.

### Superseded test deployments (same admin, abandoned)

These were earlier iterations of the build. Do not use them.

- `CABTS6VQCRQOIUATCH4SBMVUNUUZLOPK6I3DSGTPK24LEXOEBQNNNG7I` - `initialize` failed (double `require_auth`, fixed)
- `CCPLT3V2CQKEXQMLYLBDYQC4SMNYKEJJFL44BKA4YII25VX6MVKWZ5DK` - `create_market` trapped on-chain (market id depended on ledger sequence, fixed)

## Why a slim build

Stellar caps contract WASM at 128 KiB. The full contract is ~630 KB (about 260 KB of that is the embedded `contractspecv0` metadata with doc strings). Deployment therefore uses:

1. `cargo build --no-default-features` (drops the `full` feature: analytics, recovery, monitoring, upgrade, storage tooling, etc.),
2. `scripts/strip_wasm_spec.py`, which removes doc strings, duplicate spec entries and spec types not reachable from the exported functions.

Both steps are in `scripts/build_testnet_wasm.sh`.

### Exposed entry points (slim build)

`initialize`, `deposit`, `withdraw`, `get_balance`, `create_market`, `create_event`, `get_event`, `get_market`, `get_all_markets_paged`, `vote`, `place_bet`, `get_bet`, `has_user_bet`, `get_market_bet_stats`, `resolve_market_manual`, `resolve_market`, `dispute_market`, `claim_winnings`, `distribute_payouts`, `collect_fees`, `set_platform_fee`, `cancel_event`, `get_contract_version`, `capabilities`, plus the rate-limiter functions.

## Reproduce

```sh
scripts/build_testnet_wasm.sh
stellar keys generate predictify-deployer --network testnet --fund
stellar contract deploy --wasm target/wasm32v1-none/release/predictify_hybrid.slim.wasm \
  --source predictify-deployer --network testnet
ADMIN=$(stellar keys address predictify-deployer)
ID=<contract id>
stellar contract invoke --id $ID --source predictify-deployer --network testnet -- \
  initialize --admin $ADMIN --platform_fee_percentage 2 --allowed_assets null
stellar contract invoke --id $ID --source predictify-deployer --network testnet -- \
  create_market --admin $ADMIN --question '"Will BTC trade above $100,000 within 30 days?"' \
  --outcomes '["yes","no"]' --duration_days 30 \
  --oracle_config "{\"oracle_address\":\"$ADMIN\",\"feed_id\":\"BTC/USD\",\"threshold\":\"10000000000000\",\"comparison\":\"gt\",\"provider\":\"Reflector\"}" \
  --fallback_oracle_config null --resolution_timeout 86400 --min_pool_size null \
  --bet_deadline_mins_before_end null --dispute_window_seconds null --dispute_stake_floor null
stellar contract invoke --id $ID --network testnet --source predictify-deployer -- get_market --market_id mkt_5cae97f9_0
```

Note: the sample market's `oracle_address` is the admin account (a placeholder), not a real Reflector oracle, so oracle-based resolution will not work for it; use `resolve_market_manual`.

## For frontend integrators

Use contract ID `CCWRJXIUBSMFQ4AF2RXXH3AFBW6MEZG4GK4IYES7C4Z3X4PR6K7VAFHC` on testnet with RPC `https://soroban-testnet.stellar.org`. Generate bindings with `stellar contract bindings typescript --network testnet --contract-id <ID> --output-dir ./bindings`.
