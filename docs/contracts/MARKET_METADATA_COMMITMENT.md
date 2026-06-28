# Market Metadata Commitment

`Market.metadata_commitment` is a `BytesN<32>` SHA-256 commitment stored with each newly created market.

## Committed fields

The commitment is computed over the public metadata that defines what a UI displays as the market identity:

1. `question`
2. `outcomes`
3. primary `oracle_config`

The contract builds a `MarketMetadataCommitmentPayload { question, outcomes, oracle_config }`, serializes it with Soroban's canonical XDR serializer (`to_xdr`), and hashes the bytes with `Env::crypto().sha256`.

Fallback oracle settings, votes, stakes, resolution fields, and other lifecycle state are intentionally not part of this commitment.

## Verification

Clients can recompute the same canonical payload off-chain and call:

```rust
verify_market_metadata(market_id, expected_commitment) -> bool
```

The helper returns `true` only when:

- the market exists;
- `expected_commitment` equals the commitment stored with the market; and
- recomputing the commitment from the currently stored `question`, `outcomes`, and `oracle_config` still equals `expected_commitment`.

This double comparison detects stale UI caches and storage reads where a committed field was changed without updating the stored commitment.

## Authorized metadata updates

Authorized metadata update paths must call `Market::refresh_metadata_commitment` before persisting the market. Existing update paths for market description and outcomes refresh the commitment, so clients holding an old commitment will receive `false` until they reload the latest metadata.

## Migration path for pre-existing markets

Markets created before this field existed do not have an on-chain metadata commitment in their stored `Market` value. Before enabling client reliance on `verify_market_metadata` for those markets, operators should run an upgrade migration that:

1. enumerates existing market IDs from the pre-upgrade deployment/index;
2. reads each market's `question`, `outcomes`, and primary `oracle_config` using the old schema or an archival export;
3. computes `sha256(to_xdr(MarketMetadataCommitmentPayload { question, outcomes, oracle_config }))`; and
4. rewrites each market using the new `Market` schema with `metadata_commitment` populated.

If a market cannot be migrated because its original metadata cannot be read reliably, mark it as legacy in the index/UI and do not treat commitment verification as available for that market.
