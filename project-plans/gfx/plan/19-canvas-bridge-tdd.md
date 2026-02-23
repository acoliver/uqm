# Phase 19: Canvas FFI Bridge — TDD

## Phase ID
`PLAN-20260223-GFX-FULL-PORT.P19`

## Prerequisites
- Required: Phase P18a (Canvas Stub Verification) completed
- Expected: All `rust_canvas_*` stubs compile and link
- Expected: `SurfaceCanvas` adapter struct defined

## Requirements Implemented (Expanded)

### REQ-CANVAS-010: SurfaceCanvas Adapter (Test Coverage)
Test contracts:
- `test_canvas_from_null_surface` — null surface → returns null handle
- `test_canvas_from_valid_surface` — valid surface → returns non-null handle
- `test_canvas_destroy` — destroy handle, no crash, no double-free
- `test_canvas_destroy_null` — destroy null handle, no crash

### REQ-CANVAS-020: Draw Line (Test Coverage)
Test contracts:
- `test_canvas_draw_line_horizontal` — draw horizontal line, verify pixels
- `test_canvas_draw_line_vertical` — draw vertical line, verify pixels
- `test_canvas_draw_line_diagonal` — draw diagonal, verify start/end pixels
- `test_canvas_draw_line_clipped` — line partially outside canvas bounds

### REQ-CANVAS-030: Draw Rect (Test Coverage)
Test contracts:
- `test_canvas_draw_rect_outline` — verify 4 edges drawn
- `test_canvas_fill_rect_solid` — verify all pixels in rect filled
- `test_canvas_fill_rect_clipped` — rect partially outside bounds
- `test_canvas_fill_rect_zero_size` — zero width/height, no crash

### REQ-CANVAS-040: Draw Image (Test Coverage)
Test contracts:
- `test_canvas_draw_image_basic` — blit test image, verify pixels
- `test_canvas_draw_image_with_hotspot` — hotspot offset applied
- `test_canvas_draw_image_clipped` — image partially outside bounds

### REQ-CANVAS-050: Draw Fontchar (Test Coverage)
Test contracts:
- `test_canvas_draw_fontchar_opaque` — fully opaque glyph
- `test_canvas_draw_fontchar_transparent` — alpha-blended glyph
- `test_canvas_draw_fontchar_clipped` — glyph partially outside bounds

### REQ-CANVAS-060: Scissor Rect (Test Coverage)
Test contracts:
- `test_canvas_scissor_clips_line` — line outside scissor not drawn
- `test_canvas_scissor_clips_fill` — fill clipped to scissor
- `test_canvas_scissor_disable` — disable scissor, full canvas writable

### REQ-CANVAS-070: Canvas Copy (Test Coverage)
Test contracts:
- `test_canvas_copy_basic` — copy region, verify destination pixels
- `test_canvas_copy_overlapping` — overlapping src/dst handled correctly
- `test_canvas_copy_clipped` — copy clipped to destination bounds

## Implementation Tasks

### Test Infrastructure

Tests use synthetic SDL_Surface-like buffers to avoid SDL initialization:

```rust
// Helper: create a test surface with known pixel data
fn create_test_surface(w: i32, h: i32) -> TestSurface {
    // Allocate pixel buffer, create SDL_Surface-compatible struct
    // Populate with known pattern for verification
}

// Helper: read pixel at (x, y) from test surface
fn read_pixel(surface: &TestSurface, x: i32, y: i32) -> u32 { ... }
```

### Files to modify
- `rust/src/graphics/canvas_ffi.rs`
  - Add `#[cfg(test)] mod tests` block
  - Add all test functions listed above (~20 tests)
  - Add test helper functions for surface creation and pixel verification
  - marker: `@plan PLAN-20260223-GFX-FULL-PORT.P19`
  - marker: `@requirement REQ-CANVAS-010..070`

## Verification Commands

```bash
# Structural gate
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features

# Verify test count
grep -c '#\[test\]' rust/src/graphics/canvas_ffi.rs
# Expected: >= 20

# Run canvas tests specifically
cd rust && cargo test --lib -- canvas_ffi::tests --nocapture
```

## Structural Verification Checklist
- [ ] All test functions listed above are present
- [ ] Tests are in `#[cfg(test)] mod tests` block
- [ ] Test helper functions for surface creation and pixel reading
- [ ] Each test has `@requirement` traceability comment
- [ ] Tests compile (stubs may cause test failures — expected in TDD phase)

## Semantic Verification Checklist (Mandatory)
- [ ] Pixel verification tests check actual pixel buffer content
- [ ] Clipping tests verify no out-of-bounds writes
- [ ] Scissor tests verify draw restriction to scissor rect
- [ ] Copy tests verify correct pixel transfer
- [ ] Alpha blending tests use known color values for arithmetic verification

## Deferred Implementation Detection (Mandatory)

```bash
grep -n "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/graphics/canvas_ffi.rs | grep -v 'todo!(' || echo "CLEAN"
```

## Success Criteria
- [ ] >= 20 test functions written
- [ ] Tests compile (failures expected — stubs still have `todo!()`)
- [ ] `cargo fmt` and `cargo clippy` pass
- [ ] Test names are descriptive and traceable to requirements

## Failure Recovery
- rollback: `git checkout -- rust/src/graphics/canvas_ffi.rs`
- blocking issues: test surface creation incompatible with SurfaceCanvas adapter

## Phase Completion Marker
Create: `project-plans/gfx/.completed/P19.md`

Contents:
- phase ID: P19
- timestamp
- files modified: `rust/src/graphics/canvas_ffi.rs`
- total tests: count
- test results: all compile, expected failures from stubs noted
- verification: cargo suite output
