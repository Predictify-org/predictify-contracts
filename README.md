
## Auth Boundary Tests

### Overview
The markets contract has comprehensive auth boundary tests covering all entrypoints.

### Test Coverage
- `create_market` - Requires market creator auth
- `place_bet` - Requires user auth
- `resolve_market` - Requires market creator auth
- `claim_winnings` - Requires winner auth
- `cancel_market` - Requires market creator auth
- `withdraw_funds` - Requires market creator auth
- `update_market_params` - Requires market creator auth
- `add_liquidity` - Requires user auth
- `remove_liquidity` - Requires liquidity provider auth
- `pause_markets` - Requires admin auth
- `unpause_markets` - Requires admin auth
- `transfer_ownership` - Requires admin auth

### Running Tests
```bash
cargo test --test auth_boundary -- --nocapture

## Reporting Auth Boundary Tests

### Overview
The reporting contract has comprehensive auth boundary tests covering all entrypoints.

### Test Coverage
- `initialize` - Requires admin auth
- `submit_report` - Requires reporter auth
- `verify_report` - Requires admin auth
- `dispute_report` - Requires reporter auth
- `resolve_dispute` - Requires admin auth
- `update_report_status` - Requires admin auth
- `delete_report` - Requires admin auth
- `pause_reporting` - Requires admin auth
- `unpause_reporting` - Requires admin auth
- `transfer_ownership` - Requires admin auth

### Running Tests
```bash
cargo test --test auth_boundary -- --nocapture
```
