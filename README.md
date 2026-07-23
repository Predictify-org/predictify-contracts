# Predictify Contracts

Predictify Contracts contains the Soroban smart contracts for the Predictify hybrid prediction market system.

## Workspace Baseline

- Workspace dependency baseline: `soroban-sdk = "25.0.0"`
- Target network line: Stellar Protocol 25 / supported Soroban release
- Primary contract package: `contracts/predictify-hybrid`

## Local Verification
 
Run the focused contract test suite from the workspace root:
 
```sh
cargo test -p predictify-hybrid
```

### WASM Size Budget
The compiled WASM size is monitored in CI to avoid excessive deployment fees. 
The default budget is 96 KiB. You can override this by setting the `WASM_SIZE_BUDGET` environment variable (in bytes).
To check the size locally:
```sh
bash scripts/check_wasm_size.sh
```
 
If you are auditing or upgrading dependencies, regenerate the lockfile and rerun the package tests after any workspace dependency change.


## Documentation

- [Docs index](./docs/README.md)
- [Predictify Hybrid contract guide](./contracts/predictify-hybrid/README.md)
- [Soroban SDK workspace audit](./docs/security/SOROBAN_SDK_AUDIT.md)
