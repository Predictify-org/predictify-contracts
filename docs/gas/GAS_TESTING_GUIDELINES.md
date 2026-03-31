## Gas Optimization Testing Guidelines

Objective: ensure PRs do not introduce significant cost regressions and follow best practices.

### Unit Tests

- Cover all public entrypoints with valid and invalid inputs (fail fast saves gas).
- Include large-market tests (e.g., many voters) to catch algorithmic costs.

### Snapshot-Based Validation

- For stable scenarios, snapshot CLI `--cost` outputs and diff on PRs.
- Store under `test_snapshots/cost/` with scenario descriptions.

### Lints and Review

- Review loops for storage/cross-contract calls per iteration.
- Check for repeated `.get()`/`.set()` rather than single read/single write patterns.
- Ensure strings/bytes sizes are validated.

### PR Checklist (Gas)

- [ ] Storage ops minimized and batched
- [ ] No per-iteration storage writes in loops
- [ ] External calls minimized/batched
- [ ] Return/events payloads small
- [ ] Enforced input length caps

### Optional Static Analysis

- Consider running a Soroban-focused analyzer to detect storage-in-loop and repeated indirect storage access patterns.

