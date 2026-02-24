# Phase 10a: Color Layer — TDD Verification

## Phase ID
`PLAN-20260223-GFX-FULL-PORT.P10a`

## Prerequisites
- Required: Phase P10 completed
- Expected artifacts: ColorLayer tests

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features -- ffi 2>&1 | grep -E "test.*ok|test.*FAILED"
```

## Structural Verification Checklist
- [ ] New tests present
- [ ] Tests compile and pass
- [ ] Plan/requirement markers present

## Semantic Verification Checklist (Mandatory)
- [ ] `test_color_uninitialized_no_panic` calls without init
- [ ] `test_color_negative_rect_no_panic` uses negative w/h values

## Success Criteria
- [ ] All tests pass
- [ ] All cargo gates pass

## Phase Completion Marker
Create: `project-plans/gfx/.completed/P10a.md`
