# Admin Contract

Admin contract with per-entrypoint gas regression testing for the Predictify platform.

## Overview

This contract provides core admin functionality with comprehensive gas snapshot testing to ensure CPU and memory usage remains within acceptable bounds. Each entrypoint is tested individually with a 5% regression limit enforced by CI.

## Entrypoints

### `initialize(env: Env, admin: Address) -> Result<(), ContractError>`
Initializes the contract with a primary administrator. Can only be called once.

### `admin(env: Env) -> Result<Address, ContractError>`
Returns the configured admin address.

### `set_admin_cooldown(env: Env, admin: Address, seconds: u64) -> Result<(), ContractError>`
Sets the cooldown period (in seconds) between admin actions. Only callable by the current admin.

### `get_admin_cooldown(env: Env) -> u64`
Returns the configured admin cooldown period in seconds (0 if not set).

### `check_admin_cooldown(env: Env, admin: Address, function_name: Symbol) -> Result<(), ContractError>`
Enforces admin cooldown for a specific function. Updates the last action timestamp on success.

## Gas Snapshots

Per-entrypoint gas snapshots are maintained in `tests/gas_snap.rs` with the following baselines:

- `initialize`: CPU 55,643, Memory 20,337
- `admin`: CPU 31,614, Memory 12,495
- `set_admin_cooldown`: CPU 45,000, Memory 18,000
- `get_admin_cooldown`: CPU 28,000, Memory 10,000
- `check_admin_cooldown`: CPU 52,000, Memory 19,000

CI enforces a maximum 5% regression on both CPU and memory metrics.

## Testing

Run gas snapshot tests:
```bash
cargo test -p admin --test gas_snap
```

## Error Codes

- `AlreadyInitialized = 1`: Contract already initialized
- `AdminNotSet = 2`: No admin configured
- `Unauthorized = 3`: Caller is not the admin
- `AdminActionTimelocked = 4`: Cooldown period has not elapsed
