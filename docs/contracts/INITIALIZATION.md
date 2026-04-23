# Contract Initialization

This document describes the initialization process for the Predictify Hybrid smart contract.

## Overview

The contract must be initialized once after deployment to set up administrative privileges, platform fee configuration, and allowed assets. Initialization can only be performed once to prevent security vulnerabilities.

## Initialization Function

```rust
pub fn initialize(
    env: Env,
    admin: Address,
    platform_fee_percentage: Option<i128>,
    allowed_assets: Option<Vec<Address>>
) -> Result<(), Error>
```

### Parameters

- `env`: Soroban environment
- `admin`: Address to be granted SuperAdmin privileges
- `platform_fee_percentage`: Optional platform fee percentage (0-10%). Defaults to 2% if None
- `allowed_assets`: Optional list of allowed asset contract addresses. Uses defaults if None

### Security Features

1. **Re-initialization Prevention**: The function checks if `platform_fee` is already stored in persistent storage. If found, returns `Error::InvalidState`.

2. **Fee Validation**: Platform fee must be between `MIN_PLATFORM_FEE_PERCENTAGE` (0) and `MAX_PLATFORM_FEE_PERCENTAGE` (10). Invalid fees return `Error::InvalidFeeConfig`.

3. **Admin Validation**: The admin address is validated through `AdminInitializer::initialize`, which includes its own re-initialization check for the "Admin" key.

### Events Emitted

- `contract_initialized`: Emitted with admin address and platform fee percentage
- `platform_fee_set`: Emitted with fee percentage and admin address

### Error Handling

All initialization failures return `Error` instead of panicking, allowing callers to handle errors appropriately:

- `InvalidState`: Contract already initialized
- `InvalidFeeConfig`: Fee percentage out of bounds
- `Unauthorized`: Admin validation failed (from admin initializer)

### Examples

```rust
// Initialize with default settings
PredictifyHybrid::initialize(env.clone(), admin, None, None)?;

// Initialize with custom 5% fee
PredictifyHybrid::initialize(env.clone(), admin, Some(5), None)?;

// Initialize with custom assets
let assets = vec![&env, asset1, asset2];
PredictifyHybrid::initialize(env.clone(), admin, None, Some(assets))?;
```

## Post-Initialization State

After successful initialization:

- Admin is stored in persistent storage with SuperAdmin role
- Platform fee percentage is stored in persistent storage
- Allowed assets are configured (defaults or custom)
- Audit trail records the initialization
- Contract is ready for market creation and other operations

## Security Considerations

- Initialization should be done immediately after deployment
- Use a secure admin address (consider multi-sig for production)
- Validate all parameters before calling
- Handle potential errors in deployment scripts</content>
<parameter name="filePath">/home/semicolon/Drip/predictify-contracts/docs/contracts/INITIALIZATION.md