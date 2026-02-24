# Phase 07: Screen Compositing — TDD

## Phase ID
`PLAN-20260223-GFX-FULL-PORT.P07`

## Prerequisites
- Required: Phase P06a (Stub Verification) completed
- Expected files: ScreenLayer stub with guards, convert_c_rect helper

## Requirements Implemented (Expanded)

### REQ-SCR-140: Uninitialized Guard
**Requirement text**: While the backend is not initialized, `rust_gfx_screen` shall return immediately.
- GIVEN: Backend not initialized
- WHEN: `rust_gfx_screen` called
- THEN: Returns immediately, no crash

### REQ-SCR-090: Extra Screen Skip
**Requirement text**: Where `screen` is 1, `rust_gfx_screen` shall return immediately.
- GIVEN: Backend initialized
- WHEN: `rust_gfx_screen(1, 255, null)` called
- THEN: Returns immediately, no rendering

### REQ-SCR-100: Out-of-Range Screen
**Requirement text**: If `screen` is out of range, return immediately.
- GIVEN: Backend initialized
- WHEN: `rust_gfx_screen(-1, 255, null)` or `rust_gfx_screen(99, 255, null)` called
- THEN: Returns immediately

### REQ-SCR-160: Negative Rect Dimensions
**Requirement text**: If `rect->w < 0` or `rect->h < 0`, return immediately.
- GIVEN: Backend initialized, valid screen
- WHEN: `rust_gfx_screen` called with rect having negative w or h
- THEN: Returns immediately (prevents u32 overflow in Rect::new)

### REQ-FMT-040: SDL_Rect Layout Compatibility
**Requirement text**: The Rust `SDL_Rect` shall be layout-compatible with C.
- GIVEN: N/A
- WHEN: `std::mem::size_of::<SDL_Rect>()` queried
- THEN: Returns 16 (4 × sizeof(c_int))

### REQ-SCALE-060: RGBX8888 to RGBA Conversion
**Requirement text**: The RGBX8888-to-RGBA conversion shall transform `[X,B,G,R]` → `[R,G,B,0xFF]`.
- GIVEN: A pixel in RGBX8888 format [0xFF, 0x00, 0x80, 0xC0]
- WHEN: Conversion applied
- THEN: Output is [0xC0, 0x80, 0x00, 0xFF] (RGBA)

### REQ-SCALE-070: RGBA to RGBX8888 Conversion
**Requirement text**: The RGBA-to-RGBX8888 conversion shall transform `[R,G,B,A]` → `[0xFF,B,G,R]`.
- GIVEN: A pixel in RGBA format [0xC0, 0x80, 0x00, 0xFF]
- WHEN: Conversion applied
- THEN: Output is [0xFF, 0x00, 0x80, 0xC0] (RGBX8888)

## Implementation Tasks

### Files to modify
- `rust/src/graphics/ffi.rs`
  - Add tests to `#[cfg(test)] mod tests`:
    - `test_screen_uninitialized_no_panic` — @requirement REQ-SCR-140
    - `test_screen_out_of_range_no_panic` — @requirement REQ-SCR-100
    - `test_screen_extra_skip` — @requirement REQ-SCR-090
    - `test_convert_c_rect_null` — verify None for null pointer
    - `test_convert_c_rect_valid` — verify correct Rect conversion
    - `test_sdl_rect_layout` — @requirement REQ-FMT-040 (size + alignment)
    - `test_rgbx_to_rgba_conversion` — @requirement REQ-SCALE-060
    - `test_rgba_to_rgbx_conversion` — @requirement REQ-SCALE-070
    - `test_screen_negative_rect_no_panic` — @requirement REQ-SCR-160
  - marker: `@plan PLAN-20260223-GFX-FULL-PORT.P07`

### Pseudocode traceability
- Tests validate: component-003 lines 3–10 (guards)
- Tests validate: component-003B lines 1–11 (convert_rect)
- Tests validate: component-004 lines 33–45 (RGBX→RGBA), lines 72–84 (RGBA→RGBX)

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] At least 9 new test functions added
- [ ] Tests have @plan and @requirement markers
- [ ] Tests compile
- [ ] No production code changes in this phase

## Semantic Verification Checklist (Mandatory)
- [ ] Guard tests call FFI functions without prior init
- [ ] Pixel conversion tests use concrete byte values
- [ ] Rect conversion tests check null and valid cases
- [ ] Tests verify behavior, not implementation details
- [ ] Pixel conversion tests would fail with wrong byte ordering

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/graphics/ffi.rs || echo "CLEAN"
```

## Success Criteria
- [ ] All new tests pass
- [ ] Tests cover: uninitialized, out-of-range, extra skip, rect conversion, pixel conversion

## Failure Recovery
- rollback: `git checkout -- rust/src/graphics/ffi.rs`
- blocking issues: pixel conversion functions may not exist yet as separate functions

## Phase Completion Marker
Create: `project-plans/gfx/.completed/P07.md`
