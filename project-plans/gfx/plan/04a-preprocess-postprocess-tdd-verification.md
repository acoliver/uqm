# Phase 04a: Preprocess Fix + Postprocess Refactor — TDD Verification

## Phase ID
`PLAN-20260223-GFX-VTABLE-FIX.P04a`

## Prerequisites
- Required: Phase P04 completed
- Expected artifacts: New tests in `ffi.rs`

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features -- --nocapture 2>&1 | grep -E "test.*ffi.*ok|test.*ffi.*FAILED"
```

## Structural Verification Checklist
- [ ] New test functions exist in `mod tests`
- [ ] Tests have `@plan` and `@requirement` markers
- [ ] Tests compile and pass
- [ ] No production code was changed

## Semantic Verification Checklist (Mandatory)
- [ ] `test_preprocess_uninitialized_no_panic` calls `rust_gfx_preprocess(0,0,0)` without init
- [ ] `test_postprocess_uninitialized_no_panic` calls `rust_gfx_postprocess()` without init
- [ ] Tests verify behavior, not internal state

## Success Criteria
- [ ] All tests pass
- [ ] All cargo gates pass

## Phase Completion Marker
Create: `project-plans/gfx/.completed/P04a.md`
