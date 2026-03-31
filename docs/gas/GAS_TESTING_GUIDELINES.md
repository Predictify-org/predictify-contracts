## Gas Optimization Testing Guidelines

Objective: Ensure PRs do not introduce significant cost regressions and follow best practices while maintaining test stability across SDK versions.

### Unit Tests

- Cover all public entrypoints with valid and invalid inputs (fail fast saves gas).
- **Stable Mocks:** Use `GasTracker::set_test_cost(env, operation, cpu, mem)` to set expected costs for specific operations. This avoids global state clashes in parallel tests.
- **Resource Tracking:** Always track both CPU and Memory. Memory leaks or spikes can be just as impactful as CPU cycles.

### Integration Tests

- When testing core contract methods (e.g., `vote`, `create_market`), provide realistic baseline mocks.
- For stable scenarios, snapshot CLI `--cost` outputs and diff on PRs.
- Store under `test_snapshots/cost/` with scenario descriptions.

### Lints and Review

- Review loops for storage/cross-contract calls per iteration.
- Check for repeated `.get()`/`.set()` rather than single read/single write patterns.
- Ensure strings/bytes sizes are validated with reasonable caps.

### PR Checklist (Gas)

- [ ] Storage ops minimized and batched
- [ ] No per-iteration storage writes in loops
- [ ] External calls minimized/batched
- [ ] Return/events payloads small
- [ ] Enforced input length caps
- [ ] `GasTracker::end_tracking` called at the end of each public, state-changing method.

### Static Analysis

- Use `cargo llvm-cov` to ensure ≥ 95% coverage on gas-related modules.
- Consider running a Soroban-focused analyzer to detect storage-in-loop and repeated indirect storage access patterns.

