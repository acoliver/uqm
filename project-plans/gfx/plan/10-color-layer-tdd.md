# Phase 10: Color Layer — TDD

## Phase ID
`PLAN-20260223-GFX-FULL-PORT.P10`

## Prerequisites
- Required: Phase P09a (Stub Verification) completed
- Expected files: ColorLayer stub with guards

## Requirements Implemented (Expanded)

### REQ-CLR-060: Uninitialized Guard
**Requirement text**: While not initialized, return immediately.
- GIVEN: Backend not initialized
- WHEN: `rust_gfx_color(0, 0, 0, 128, null)` called
- THEN: No crash, returns immediately

### REQ-CLR-070: Full Alpha Range
**Requirement text**: The backend shall accept `a` values 0–255 without clamping.
- GIVEN: Backend initialized
- WHEN: `rust_gfx_color` called with a=0, a=1, a=128, a=254, a=255
- THEN: Each value is handled correctly without clamping

## Implementation Tasks

### Files to modify
- `rust/src/graphics/ffi.rs`
  - Add tests to `#[cfg(test)] mod tests`:
    - `test_color_uninitialized_no_panic` — @requirement REQ-CLR-060
    - `test_color_negative_rect_no_panic` — @requirement REQ-CLR-055
  - marker: `@plan PLAN-20260223-GFX-FULL-PORT.P10`

### Pseudocode traceability
- Tests validate: component-005 lines 2–10 (guards)

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] New test functions added
- [ ] Tests have plan/requirement markers
- [ ] Tests compile and pass
- [ ] No production code changes

## Semantic Verification Checklist (Mandatory)
- [ ] Guard tests call without init and don't crash
- [ ] Tests verify behavior, not internals
- [ ] Would fail if guard removed (panic from todo!())

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK" rust/src/graphics/ffi.rs || echo "CLEAN"
```

## Success Criteria
- [ ] All new tests pass
- [ ] Tests cover uninitialized and negative rect cases

## Failure Recovery
- rollback: `git checkout -- rust/src/graphics/ffi.rs`

## Phase Completion Marker
Create: `project-plans/gfx/.completed/P10.md`
