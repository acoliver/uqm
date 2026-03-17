# Phase 04: Postprocess Cleanup + Scanline Effect

## Phase ID
`PLAN-20260314-GRAPHICS.P04`

## Prerequisites
- Required: Phase P03 completed
- Verify: Canvas pixel sync tests pass
- Expected files from previous phase: Modified `canvas_ffi.rs`

## Requirements Implemented (Expanded)

### REQ-RL-004: Single final present per frame
**Requirement text**: When a frame is presented, the subsystem shall perform exactly one final display present operation per frame.

Behavior contract:
- GIVEN: `rust_gfx_screen()` has already composited all screen layers onto the renderer
- WHEN: `rust_gfx_postprocess()` is called
- THEN: Only scanline effects (if enabled) and `present()` are called — no texture uploads or surface-to-renderer copies

### REQ-SCAL-006: Scanline effect
**Requirement text**: When the scanline presentation option is enabled, the subsystem shall apply a scanline-like postprocess effect before final present.

Behavior contract:
- GIVEN: The `SCANLINES` flag (bit 2) is set in graphics flags
- WHEN: `rust_gfx_postprocess()` is called
- THEN: Alternating horizontal lines are dimmed before present

### REQ-INT-002: Backend-vtable compatibility
**Requirement text**: The subsystem shall provide behaviorally compatible implementations of postprocess.

Behavior contract:
- GIVEN: C reference `postprocess` only applies scanlines and presents
- WHEN: Rust `postprocess` is called
- THEN: Behavior matches — no upload/scale/copy operations

## Implementation Tasks

### Files to modify
- `rust/src/graphics/ffi.rs`
  - **Strip** `rust_gfx_postprocess()` (currently lines ~477-598): Remove the entire surface upload, scaling, texture creation, and copy block. Reduce to:
    1. Get state
    2. If SCANLINES flag set, apply scanline effect
    3. Call `state.canvas.present()`
  - **Add** private `apply_scanlines()` helper function that draws semi-transparent black horizontal lines on alternating rows
  - marker: `@plan PLAN-20260314-GRAPHICS.P04`
  - marker: `@requirement REQ-RL-004, REQ-SCAL-006`

### Scanline implementation approach
The C reference (`sdl2_pure.c:344-356`) draws horizontal lines every other row using `SDL_RenderDrawLine`. The Rust equivalent:
1. Set renderer draw color to `(0, 0, 0, scanline_alpha)` — use the exact C-reference alpha after verifying the source constant, not an estimated substitute
2. Set blend mode to `Blend`
3. For each even row (0, 2, 4, ... up to logical height): draw a filled rect `(0, y, logical_width, 1)` or equivalent horizontal line primitive
4. Reset blend mode to `None`

### Constants to define
- `SCANLINES_FLAG: u32 = 0x04` (bit 2 per spec §3.8)
- `SCANLINE_ALPHA: u8` — match the verified C reference value exactly

### Pseudocode traceability
- Uses pseudocode lines: PC-02, lines 40-52

## TDD Test Plan

### Tests to add/modify in `ffi.rs`

1. `test_postprocess_uninitialized_no_crash` — already exists, verify still passes after strip
2. `test_scanlines_flag_constant` — verify SCANLINES_FLAG bit matches spec §3.8
3. `test_postprocess_contains_no_upload_path` — structural/unit assertion if feasible around helper boundaries or removed code paths

Note: Full scanline semantic verification requires runtime or image-based output checks. This phase establishes implementation and unit/structural coverage. Stronger semantic confirmation is required later in P09/P11 after migrated-path integration.

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `rust_gfx_postprocess` no longer contains texture upload/scaling code
- [ ] `apply_scanlines` function exists
- [ ] SCANLINES flag constant defined
- [ ] `SCANLINE_ALPHA` matches the verified C reference value
- [ ] `catch_unwind` still wraps the FFI body
- [ ] Plan/requirement traceability markers present

## Semantic Verification Checklist (Mandatory)
- [ ] Postprocess only calls present (+ optional scanlines) — no texture creation
- [ ] Scanline flag detection uses correct bit (0x04)
- [ ] Existing postprocess-uninitialized tests still pass
- [ ] No upload/scale/copy code remains in postprocess
- [ ] All other ffi.rs tests still pass (screen compositing unaffected)
- [ ] P11 includes runtime/image-based verification of scanline output, not just structural checks

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/graphics/ffi.rs | grep -i "postprocess\|scanline"
```

## Success Criteria
- [ ] REQ-RL-004 demonstrated: postprocess is present-only
- [ ] REQ-SCAL-006 demonstrated: scanline function exists, uses C-reference alpha, and is called when flag set
- [ ] Verification commands pass

## Failure Recovery
- Rollback: `git checkout -- rust/src/graphics/ffi.rs`
- Blocking: If removing postprocess upload breaks rendering → screen() compositing has a bug. Debug screen() first.

## Phase Completion Marker
Create: `project-plans/20260311/graphics/.completed/P04.md`
