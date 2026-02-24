# Phase 12: Scaling Integration — Stub + TDD + Implementation

## Phase ID
`PLAN-20260223-GFX-FULL-PORT.P12`

## Prerequisites
- Required: Phase P11a (ColorLayer Verification) completed
- Expected files: ScreenLayer unscaled and ColorLayer both working

## Requirements Implemented (Expanded)

### REQ-SCALE-010: RGBX8888 to RGBA Conversion Before Scaling
**Requirement text**: Where software scaling is active, convert surface pixel data from RGBX8888 to RGBA format before passing to the scaler.

Behavior contract:
- GIVEN: Software scaling active, surface has RGBX8888 data
- WHEN: ScreenLayer called for a compositable screen
- THEN: Pixels are converted to RGBA before scaler invocation

### REQ-SCALE-020: Scaler Selection
**Requirement text**: Invoke appropriate scaler based on GFX flags: xBRZ for bits 8/9, HQ2x for bit 7.

Behavior contract:
- GIVEN: flags & (1 << 8) set (xBRZ 3×)
- WHEN: Scaler invoked
- THEN: `xbrz::scale_rgba` called with factor=3

### REQ-SCALE-030: RGBA to RGBX8888 Conversion After Scaling
**Requirement text**: Convert scaler output from RGBA back to RGBX8888 before texture upload.

Behavior contract:
- GIVEN: Scaler has produced RGBA output
- WHEN: Preparing texture data
- THEN: Output is converted to RGBX8888 for texture upload

### REQ-SCALE-040: Scaled Texture Dimensions
**Requirement text**: Texture shall be created at `(320 × factor) × (240 × factor)`.

Behavior contract:
- GIVEN: scale_factor=2 (HQ2x)
- WHEN: Texture created
- THEN: Texture dimensions are 640×480

### REQ-SCALE-050: Scaled Source Rect
**Requirement text**: Where software scaling is active and `rect` is non-NULL, source rect coordinates shall be multiplied by scale factor.

Behavior contract:
- GIVEN: scale_factor=2, rect={x:10, y:20, w:100, h:80}
- WHEN: canvas.copy called
- THEN: src_rect={20, 40, 200, 160}, dst_rect={10, 20, 100, 80}

### REQ-WIN-030: Source Rect Scaled by Factor
When software scaling is active and `rect` is non-NULL, the source rect coordinates and dimensions shall be multiplied by `scale_factor` for the texture source region while the destination rect remains unscaled.

### REQ-SCALE-025: Bilinear-Only Skips Software Scaling
**Requirement text**: Where only `SCALE_BILINEAR` is set, use the unscaled path.

Behavior contract:
- GIVEN: flags = SCALE_BILINEAR only (bit 3)
- WHEN: ScreenLayer called
- THEN: Unscaled path used (no software scaler)

### REQ-SCALE-055: Integer Overflow Safety for Scaled Coordinates
**Requirement text**: Multiplication of source rect coordinates by the scale factor shall not overflow `i32`. Given the fixed source resolution (320×240) and maximum scale factor (4), the maximum product (1280) is within `i32` range. Satisfied by construction; no runtime overflow check is required.

Behavior contract:
- GIVEN: scale_factor ≤ 4, source coordinates ≤ 320×240
- WHEN: Source rect multiplied by scale factor
- THEN: Product ≤ 1280, well within `i32` range — no overflow possible

### REQ-SCALE-060: RGBX→RGBA Byte Order
**Requirement text**: `[X,B,G,R]` → `[R,G,B,0xFF]`

### REQ-SCALE-070: RGBA→RGBX Byte Order
**Requirement text**: `[R,G,B,A]` → `[0xFF,B,G,R]`

## Implementation Tasks

This is a combined Stub+TDD+Impl phase because the scaling logic is being
**relocated** from the existing `rust_gfx_postprocess` (which was removed
in P03/P05) into `rust_gfx_screen`. The algorithm is already proven — it
needs to be adapted from single-screen to per-screen operation.

### Files to modify
- `rust/src/graphics/ffi.rs`
  - **Add helper functions**:
    - `fn convert_rgbx_to_rgba(src: &[u8], dst: &mut [u8], width: usize, height: usize, pitch: usize)`
    - `fn convert_rgba_to_rgbx(src: &[u8], dst: &mut [u8], width: usize, height: usize)`
    - These extract the inline pixel conversion loops from the old postprocess
  - **Modify `rust_gfx_screen`**:
    - After the unscaled path check (line 37 of pseudocode component-003),
      add the scaled path branch that:
      1. Determines scale factor from flags
      2. Creates texture at scaled dimensions
      3. Converts RGBX→RGBA via helper
      4. Runs scaler (xBRZ or HQ2x)
      5. Converts RGBA→RGBX via helper
      6. Uploads scaled buffer to texture
      7. Computes scaled source rect
      8. Calls canvas.copy with scaled src / unscaled dst
  - **Add tests**:
    - `test_convert_rgbx_to_rgba_basic` — @requirement REQ-SCALE-060
    - `test_convert_rgba_to_rgbx_basic` — @requirement REQ-SCALE-070
    - `test_convert_rgbx_rgba_roundtrip` — verify conversion roundtrip fidelity
    - `test_scale_factor_determination` — @requirement REQ-INIT-070
    - `test_bilinear_only_no_software_scale` — @requirement REQ-SCALE-025
  - marker: `@plan PLAN-20260223-GFX-FULL-PORT.P12`
  - marker: `@requirement REQ-SCALE-010..070, REQ-SCALE-025, REQ-SCALE-055, REQ-WIN-030`

### Pseudocode traceability
- Uses pseudocode: component-004 lines 1–114 (full scaled path)
- Uses pseudocode: component-003 lines 37–41 (scaled path branch)

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `convert_rgbx_to_rgba` helper function exists
- [ ] `convert_rgba_to_rgbx` helper function exists
- [ ] `rust_gfx_screen` handles both scaled and unscaled paths
- [ ] Scale factor determination matches component-004 lines 3–5
- [ ] Scaled texture dimensions use `SCREEN_WIDTH * factor` × `SCREEN_HEIGHT * factor`
- [ ] Source rect multiplication uses `scale_factor` for src, unscaled for dst
- [ ] All new tests compile and pass
- [ ] All existing tests still pass

## Semantic Verification Checklist (Mandatory)
- [ ] Pixel conversion matches byte ordering from REQ-SCALE-060/070
- [ ] xBRZ path invokes `xbrz::scale_rgba` with correct parameters
- [ ] HQ2x path invokes `state.hq2x.scale` with correct parameters
- [ ] HQ2x scale factor is always 2 (REQ-INIT-070 default)
- [ ] xBRZ3 scale factor is 3, xBRZ4 is 4
- [ ] Bilinear-only flag does NOT trigger software scaling
- [ ] One-time logging for scaler activation preserved
- [ ] No per-frame logging in hot path
- [ ] Scaling code relocated from old postprocess (not duplicated)

## Deferred Implementation Detection (Mandatory)

```bash
# No deferred patterns in screen function or conversion helpers
grep -n "TODO\|FIXME\|HACK\|todo!\|unimplemented!" rust/src/graphics/ffi.rs || echo "CLEAN"
```

## Success Criteria
- [ ] Scaled ScreenLayer path works for HQ2x, xBRZ3, xBRZ4
- [ ] Pixel conversion helpers are unit-tested
- [ ] All cargo gates pass
- [ ] No placeholder patterns anywhere in ffi.rs

## Failure Recovery
- rollback: `git checkout -- rust/src/graphics/ffi.rs`
- blocking issues: scaler API changes, pixel format mismatches

## Phase Completion Marker
Create: `project-plans/gfx/.completed/P12.md`

Contents:
- phase ID: P12
- timestamp
- files modified: `rust/src/graphics/ffi.rs`
- functions: convert_rgbx_to_rgba, convert_rgba_to_rgbx, rust_gfx_screen (scaled path)
- tests: conversion tests, scale factor tests
- verification: cargo fmt/clippy/test outputs
- semantic: software scaling works via ScreenLayer
