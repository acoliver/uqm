# Phase 05a: Core Types & State Machine — Implementation Verification

## Phase ID
`PLAN-20260314-NETPLAY.P05a`

## Prerequisites
- Required: Phase 05 (Core Types Implementation) completed
- Expected: all tests from P04 passing, no todo!() in core type files

## Verification Tasks

### Test Results
- [ ] ALL tests in `state.rs::tests` pass
- [ ] ALL tests in `error.rs::tests` pass
- [ ] ALL tests in `options.rs::tests` pass
- [ ] ALL tests in `constants.rs::tests` pass
- [ ] No test panics or skips

### Implementation Quality
- [ ] No `todo!()`, `unimplemented!()`, `FIXME`, `HACK` in production code
- [ ] `thiserror` derive used properly on `NetplayError`
- [ ] All `AbortReason` / `ResetReason` numeric conversions are exhaustive
- [ ] `StateFlags::clear_*` methods reset the correct sub-fields
- [ ] `validate_transition` is exhaustive (no catch-all that silently accepts)

### Wire Compatibility
- [ ] `AbortReason` integer values match C `packet.h:47-55`:
  - `Unspecified = 0`, `VersionMismatch = 1`, `InvalidHash = 2`, `ProtocolError = 3`
- [ ] `NetState` ordering preserves C comparison semantics where used (e.g., `state > InSetup`)

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --workspace  # also passes without netplay feature
```

## Gate Decision
- [ ] PASS: proceed to Phase 06
- [ ] FAIL: return to Phase 05 and fix issues

## Phase Completion Marker
Create: `project-plans/20260311/netplay/.completed/P05a.md`
