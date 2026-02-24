# Phase 07a: Screen Compositing — TDD Verification

## Phase ID
`PLAN-20260223-GFX-FULL-PORT.P07a`

## Prerequisites
- Required: Phase P07 completed
- Expected artifacts: 9+ new tests in ffi.rs

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features -- ffi 2>&1 | grep -E "test.*ok|test.*FAILED"
```

## Structural Verification Checklist
- [ ] At least 9 new tests present
- [ ] All tests compile
- [ ] All tests pass
- [ ] Tests have plan/requirement markers

## Semantic Verification Checklist (Mandatory)
- [ ] `test_screen_uninitialized_no_panic` calls without init and doesn't crash
- [ ] `test_screen_out_of_range_no_panic` uses -1, 3, 99, i32::MAX
- [ ] `test_rgbx_to_rgba_conversion` verifies specific byte values
- [ ] `test_rgba_to_rgbx_conversion` verifies specific byte values
- [ ] `test_convert_c_rect_null` verifies None return for null pointer
- [ ] `test_convert_c_rect_valid` verifies correct x, y, w, h mapping

## Success Criteria
- [ ] All tests pass
- [ ] All cargo gates pass

## Phase Completion Marker
Create: `project-plans/gfx/.completed/P07a.md`
