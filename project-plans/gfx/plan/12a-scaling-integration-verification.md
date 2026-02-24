# Phase 12a: Scaling Integration — Verification

## Phase ID
`PLAN-20260223-GFX-FULL-PORT.P12a`

## Prerequisites
- Required: Phase P12 completed
- Expected artifacts: Scaled ScreenLayer path, conversion helpers, tests

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `convert_rgbx_to_rgba` function exists and is tested
- [ ] `convert_rgba_to_rgbx` function exists and is tested
- [ ] `rust_gfx_screen` branches on `state.scaled_buffers[screen].is_some()`
- [ ] Scaled path creates texture at factor × dimensions
- [ ] Source rect scaled, destination rect unscaled
- [ ] xBRZ and HQ2x paths both implemented

## Semantic Verification Checklist (Mandatory)

```bash
# Verify conversion functions exist
grep -c "fn convert_rgbx_to_rgba\|fn convert_rgba_to_rgbx" rust/src/graphics/ffi.rs
# Should be 2

# Verify no deferred patterns
grep -c "TODO\|FIXME\|HACK\|todo!\|unimplemented!" rust/src/graphics/ffi.rs | xargs test 0 -eq && echo "CLEAN" || echo "CHECK MANUALLY"
```

- [ ] Conversion functions exist and are tested
- [ ] No deferred patterns in any function
- [ ] Pixel format roundtrip test passes
- [ ] All cargo gates pass
- [ ] Scaling code is in ScreenLayer (not in Postprocess)

## Success Criteria
- [ ] All structural checks pass
- [ ] All semantic checks pass
- [ ] All tests pass
- [ ] Postprocess still only calls present()

## Phase Completion Marker
Create: `project-plans/gfx/.completed/P12a.md`
