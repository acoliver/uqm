# Phase 14: E2E Integration

## Phase ID
`PLAN-20260320-BATTLEPT2.P14`

## Prerequisites
- Required: Phase 13a (FFI Layer Verification) completed with PASS
- Expected files: All Rust modules, all C guards, rust_battle_wrappers.c, ffi.rs complete
- Expected artifacts: Both build modes compile, DoBattle thin shell verified, CRC determinism verified

## Requirements Implemented (Expanded)

### REQ: End-to-end integration (all requirements)
**Requirement text**: The Rust-owned battle engine must produce behavior identical to the C reference when wired end-to-end through the FFI layer.

Behavior contract:
- GIVEN: A fully wired Rust-enabled build
- WHEN: A complete battle is executed (from Battle() entry to cleanup)
- THEN: All observable outputs (display, audio, netplay checksums, crew writeback) match the C reference

### REQ: Cross-module integration
**Requirement text**: All 7 Rust battle modules must work together through their defined interfaces.

Behavior contract:
- GIVEN: lifecycle::battle() is called
- WHEN: The full frame loop executes
- THEN: lifecycle → process_loop → ship_runtime → tactical → ai → c_bridge → ffi all interact correctly

### REQ: Regression prevention
**Requirement text**: All existing tests (Phase 1 + Phase 2/3) must pass with no regressions.

Behavior contract:
- GIVEN: The full test suite
- WHEN: `cargo test --workspace --all-features` runs
- THEN: All tests pass including Phase 1's 2,151 tests and all Phase 2/3 tests

## Implementation Tasks

### Files to create

- `rust/src/battle/integration_tests.rs` (or in `mod.rs` tests section)
  - marker: `@plan PLAN-20260320-BATTLEPT2.P14`
  - marker: `@requirement REQ-E2E-INTEGRATION`
  - Integration test scenarios:
    1. **Full battle lifecycle**: battle() → init → frame loop → death → cleanup
    2. **Ship spawn → preprocess → collision → death → new_ship cycle**
    3. **Flee sequence**: ProcessInput ESCAPE → DoRunAway → flee_preprocess → warp-out → crew preserved
    4. **AI dispatch integration**: computer_intelligence → ship_preprocess pipeline
    5. **Simultaneous death**: both ships die same frame → winner handling → new_ship for both
    6. **Queue cascading**: element spawned during preprocess → cascading preprocess + collision
    7. **Zoom/camera**: two ships at varying distances → correct zoom level transitions
    8. **Reference counting**: InitSpace → InitSpace → UninitSpace → UninitSpace (load once, free once)
    9. **Battle counter**: ship deaths decrement correctly, flee doesn't decrement, 0 ends battle

### Files to modify

- `rust/src/battle/mod.rs` — Wire integration tests
  - marker: `@plan PLAN-20260320-BATTLEPT2.P14`

### Regression test verification
- All Phase 1 tests (229 battle-specific, 2,151 total) must pass
- All Phase 2/3 tests must pass
- Cross-module calls verified end-to-end

### C reference functions ported
P14 ports no functions. It verifies the complete system.

### C branches to handle

| Branch | Source Sites | Handling |
|--------|-------------|----------|
| All 7 families | Across all modules | E2E tests cover multiple branch combinations |

### Integration points
- ALL modules interconnected: lifecycle ↔ process_loop ↔ ship_runtime ↔ tactical ↔ ai ↔ c_bridge ↔ ffi
- C-side: rust_battle_wrappers.c → Rust FFI exports → back to C bridge calls

### Pseudocode traceability (if impl phase)
- N/A (integration/verification phase)

## Verification Commands

```bash
# Full verification suite
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# C-side verification (both modes)
# C-only: make clean && make
# Rust-enabled: make clean && CFLAGS=-DUSE_RUST_BATTLE_LOOP make

# Cross-module integration tests specifically
cargo test --lib battle::integration_tests --all-features
```

## Structural Verification Checklist
- [ ] Integration tests exist
- [ ] All Rust modules compile together
- [ ] Both C build modes compile
- [ ] No circular dependencies
- [ ] Plan/requirement traceability markers present

## Semantic Verification Checklist (Mandatory)
- [ ] Full battle lifecycle runs end-to-end
- [ ] Ship spawn→preprocess→collision→death→replacement cycle works
- [ ] Flee sequence works end-to-end
- [ ] AI produces correct inputs feeding ship pipeline
- [ ] Simultaneous death handled correctly in integrated context
- [ ] Queue cascading works with real element spawns
- [ ] Zoom transitions work with real ship positions
- [ ] Reference counting correct under real usage patterns
- [ ] Battle counter management correct
- [ ] All 2,151+ tests pass (no regression)
- [ ] No placeholder/deferred implementation patterns

## Deferred Implementation Detection (Mandatory)

```bash
# Check ALL Rust battle source files
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/battle/
```

## Success Criteria
- [ ] All integration tests pass
- [ ] All unit tests pass (including Phase 1)
- [ ] Both C build modes compile and link
- [ ] No TODO/FIXME/HACK in any battle module
- [ ] All verification commands pass
- [ ] Cross-module interactions verified

## Failure Recovery
- rollback: `git checkout -- rust/src/battle/`
- blocking issues: Cross-module integration failures, unexpected state interactions

## Phase Completion Marker
Create: `project-plans/20260311/battlept2/.completed/P14.md`

Contents:
- phase ID: PLAN-20260320-BATTLEPT2.P14
- timestamp
- files created/changed
- tests added (integration tests)
- all verification outputs
- full regression test results
- semantic verification summary
