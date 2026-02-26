**Description:**

This PR resolves #305 by implementing a gas cost tracking module and adding optimization hooks for key operations to support cost observability and arbitrary budget limit enforcement.

**Key Changes:**

- **GasTracker Module (`src/gas.rs`):** Introduced a flexible gas monitoring and limits enforcement module, storing limits in contract instance storage.
- **Observability Hooks injected:** Added `start_tracking` and `end_tracking` lifecycle hooks into the primary entrypoints:
  - `create_event`
  - `place_bet`
  - `resolve_market`
  - `distribute_payouts` 
- **Gas Event Publications:** Included explicit reporting via `soroban_sdk::events::publish` emitting `gas_used` analytics symbols alongside their corresponding market action keys for indexing. 
- **Admin Configuration (Optional Caps):** Exposes `set_limit` allowing contract administrators to dynamically define the gas capacity limits for explicit contract functions.
- **Optimization Guidelines:** Embedded explicit optimization rules as NatSpec-style comments directly inside the `GasTracker` documentation covering maps, batching, and memory caching strategies.

**Verification:**
- Validated compatibility with existing structs.
- Verified test correctness: All 440 property and unit tests complete successfully, maintaining the >95% confidence baseline.
