# Phase 05: DCQ Command Completeness

## Phase ID
`PLAN-20260314-GRAPHICS.P05`

## Prerequisites
- Required: Phase P04 completed
- Verify: Postprocess cleanup and scanlines pass
- Expected files from previous phase: Modified `ffi.rs`

## Requirements Implemented (Expanded)

### REQ-DQ-001: Single drawing ingress
**Requirement text**: All externally visible drawing work submitted for deferred execution shall enter the rendering pipeline through the draw-command queue.

Behavior contract:
- GIVEN: The DCQ is initialized
- WHEN: Any queue command type from spec S5.1 is submitted
- THEN: A corresponding push function exists and enqueues the command

### REQ-INT-001: Existing API compatibility
**Requirement text**: The subsystem shall preserve the externally visible behavior required by the existing UQM graphics API surface.

Behavior contract:
- GIVEN: C code calls `TFB_DrawScreen_FilledImage`, `TFB_DrawScreen_FontChar`, etc.
- WHEN: These are redirected to Rust DCQ push functions
- THEN: All parameters are preserved and the command is properly enqueued

### REQ-INT-003: FFI symbol compatibility
**Requirement text**: Where integration depends on named exported FFI entry points, the subsystem shall preserve those entry points.

Behavior contract:
- GIVEN: `rust_gfx.h` declares `rust_dcq_push_filledimage`, `rust_dcq_push_fontchar`, etc.
- WHEN: The Rust library is compiled
- THEN: All declared symbols are present and callable

### REQ-INT-008: Context-driven draw compatibility
This phase preserves the queue-side parameters needed so higher-level draw state can propagate through the bridge without being collapsed to defaults.

## Authoritative Queue Command Inventory

This phase must use exactly the 16-command inventory from specification §5.1:
1. `Line`
2. `Rect`
3. `Image`
4. `FilledImage`
5. `FontChar`
6. `Copy`
7. `CopyToImage`
8. `SetMipmap`
9. `DeleteImage`
10. `DeleteData`
11. `SendSignal`
12. `ReinitVideo`
13. `SetPalette`
14. `ScissorEnable`
15. `ScissorDisable`
16. `Callback`

`batch`, `unbatch`, and `set_screen` are queue control operations and are intentionally excluded from this command count.

## Implementation Tasks

### Task 1: Add `SetPalette` variant to `DrawCommand` enum

#### File: `rust/src/graphics/dcqueue.rs`
- Add `SetPalette { colormap_id: u32 }` variant to `DrawCommand` enum
- Add handler in `handle_command()`: update active colormap in the real render context and any dependent invalidation state required by cache reuse semantics
- marker: `@plan PLAN-20260314-GRAPHICS.P05`
- marker: `@requirement REQ-DQ-001, REQ-CMAP-003`

### Task 2: Add 5 missing DCQ push functions

#### File: `rust/src/graphics/dcq_ffi.rs`

For each newly added FFI/API signature, tie the Rust export to the actual C declaration and caller in `rust_gfx.h` / `tfb_draw.c`. Do not introduce signatures that are not grounded in the existing caller surface.

**2a: `rust_dcq_push_filledimage`**
```
pub extern "C" fn rust_dcq_push_filledimage(
    image_id: u32, x: c_int, y: c_int, color: u32,
    scale: c_int, scale_mode: c_int, draw_mode: c_int,
) -> c_int
```
- Creates `DrawCommand::FilledImage` with all parameters
- marker: `@requirement REQ-DQ-001`

**2b: `rust_dcq_push_fontchar`**
```
pub unsafe extern "C" fn rust_dcq_push_fontchar(
    fontchar_data: *const u8, pitch: c_int, w: c_int, h: c_int,
    hs_x: c_int, hs_y: c_int, disp_w: c_int, disp_h: c_int,
    backing_image_id: u32, x: c_int, y: c_int,
    color: u32, draw_mode: c_int,
) -> c_int
```
- Creates `FontCharRef` from raw glyph data
- Creates optional `ImageRef` for backing image (0 = no backing)
- Creates `DrawCommand::FontChar`
- marker: `@requirement REQ-DQ-001, REQ-FONT-001`

**2c: `rust_dcq_push_setmipmap`**
```
pub extern "C" fn rust_dcq_push_setmipmap(
    image_id: u32, mipmap_id: u32, hot_x: c_int, hot_y: c_int,
) -> c_int
```
- Creates `DrawCommand::SetMipmap`
- marker: `@requirement REQ-DQ-001, REQ-IMG-005`

**2d: `rust_dcq_push_deletedata`**
```
pub extern "C" fn rust_dcq_push_deletedata(data_ptr: u64) -> c_int
```
- Creates `DrawCommand::DeleteData { data: data_ptr }`
- marker: `@requirement REQ-DQ-001, REQ-DQ-010`

**2e: `rust_dcq_push_callback`**
```
pub unsafe extern "C" fn rust_dcq_push_callback(
    callback: extern "C" fn(u64), arg: u64,
) -> c_int
```
- Wraps C function pointer as Rust-callable
- Creates `DrawCommand::Callback`
- marker: `@requirement REQ-DQ-001, REQ-DQ-009`

### Task 3: Expand `rust_dcq_push_drawimage` signature

#### File: `rust/src/graphics/dcq_ffi.rs`
- Change from `(image_id, x, y)` to `(image_id, x, y, scale, scale_mode, colormap_index, draw_mode)`
- Convert parameters to proper Rust types
- Preserve exactly the caller-visible state supplied by the C draw/context layer; do not hardcode defaults in the bridge
- marker: `@requirement REQ-IMG-003, REQ-IMG-004, REQ-INT-008`

### Task 4: Fix `rust_dcq_push_setpalette`

#### File: `rust/src/graphics/dcq_ffi.rs`
- Replace Callback stub with proper `DrawCommand::SetPalette { colormap_id }`
- marker: `@requirement REQ-CMAP-003`

### Task 5: Update C header against real call signatures

#### File: `sc2/src/libs/graphics/sdl/rust_gfx.h`
- Verify all 5 new functions are declared
- Update `rust_dcq_push_drawimage` declaration to match the expanded signature
- For each declaration, identify the concrete C caller that will use it in P09
- marker: `@plan PLAN-20260314-GRAPHICS.P05`

### Pseudocode traceability
- Uses pseudocode lines: PC-03 (60-94), PC-04 (100-112), PC-05 (120-133)

## TDD Test Plan

### Tests to add in `dcq_ffi.rs`

1. `test_dcq_push_filledimage` — init, push, verify len=1
2. `test_dcq_push_fontchar` — init, push with mock glyph data, verify len=1
3. `test_dcq_push_fontchar_null_data` — null glyph data returns -1
4. `test_dcq_push_setmipmap` — init, push, verify len=1
5. `test_dcq_push_deletedata` — init, push, verify len=1
6. `test_dcq_push_callback` — init, push, verify len=1
7. `test_dcq_push_drawimage_expanded` — init, push with scale/colormap/drawmode, verify len=1 and preserved parameters
8. `test_dcq_push_setpalette_real` — init, push, verify len=1 and it's a SetPalette not Callback
9. `test_dcq_push_all_command_types` — push one of every queue command type, verify len=16

### Tests to add in `dcqueue.rs`

10. `test_handle_setpalette` — push SetPalette, process, verify render context updated

### Bridge preparation checks

11. Header/export signature check — compare each declaration in `rust_gfx.h` with the Rust export signature and planned C caller

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `DrawCommand::SetPalette` variant exists
- [ ] 5 new push functions exported with `#[no_mangle]`
- [ ] `rust_dcq_push_drawimage` has expanded signature
- [ ] `rust_dcq_push_setpalette` creates real `SetPalette` command
- [ ] All `catch_unwind` wrappers present
- [ ] C header declarations match Rust exports exactly
- [ ] Each new export is tied to a concrete C declaration/caller, not a speculative interface

## Semantic Verification Checklist (Mandatory)
- [ ] Every spec §5.1 queue command type has a corresponding push function
- [ ] FilledImage preserves color, scale, scale_mode parameters
- [ ] FontChar handles both with and without backing image
- [ ] DrawImage passes scale/colormap/drawmode through without replacing caller state with defaults
- [ ] SetPalette enqueues a real command (not a Callback wrapper)
- [ ] All previous DCQ tests still pass

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/graphics/dcq_ffi.rs
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/graphics/dcqueue.rs
```

## Success Criteria
- [ ] All 16 queue command types pushable via FFI
- [ ] Queue-side parameter propagation needed for REQ-INT-008 is preserved
- [ ] 10+ new tests pass
- [ ] Verification commands pass

## Failure Recovery
- Rollback: `git checkout -- rust/src/graphics/dcq_ffi.rs rust/src/graphics/dcqueue.rs sc2/src/libs/graphics/sdl/rust_gfx.h`
- Blocking: `DrawCommand` enum change breaks pattern matches → update all match arms

## Phase Completion Marker
Create: `project-plans/20260311/graphics/.completed/P05.md`
