# Oracles Contract Storage

## Persistent Storage

| Key | Type | Description |
|-----|------|-------------|
| `DataKey::OracleList` | `Vec<Address>` | Ordered list of registered oracle addresses |

## State Diagram

```
EmptyRegistry
   │
   ├── add_oracle(admin, oracle)
   │      └── push_back → NonEmptyRegistry
   │
   ├── remove_oracle(admin, oracle)
   │      └── ItemCount─1 → EmptyRegistry / NonEmptyRegistry
   │
   └── list_oracles()
          └── Reads registry (with TTL bump)
```

## TTL Management

The oracle list key is bumped on every read via `bump_registry_ttl()`:

- **Threshold**: `REGISTRY_TTL_BUMP_THRESHOLD = 120_960` ledgers
- **Extend to**: `REGISTRY_TTL_BUMP_TO = 518_400` ledgers
