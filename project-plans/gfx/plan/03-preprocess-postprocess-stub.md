# Phase 03: Preprocess Fix + Postprocess Refactor — Stub

## Phase ID
`PLAN-20260223-GFX-VTABLE-FIX.P03`

## Prerequisites
- Required: Phase P02a (Pseudocode Verification) completed
- Verify previous phase markers/artifacts exist
- Expected files from previous phase: all pseudocode components

## Phase Scope Note

This phase covers **Init + Preprocess + Postprocess** together. The INIT
requirements (REQ-INIT-*) are included here — rather than in a separate init
phase — because initialization is a prerequisite for testing preprocess and
postprocess behavior. Without a working `rust_gfx_init`, there is no
`RustGraphicsState`, no canvas, and no renderer, so preprocess/postprocess
cannot be exercised at all. Grouping them ensures the earliest testable
end-to-end path (init → preprocess → postprocess → black screen) is
delivered as a single atomic unit.

## Requirements Implemented (Expanded)

### REQ-PRE-010: Preprocess Blend Mode Reset
**Requirement text**: When `rust_gfx_preprocess` is called, the backend shall set the renderer blend mode to `BLENDMODE_NONE`.

Behavior contract:
- GIVEN: The backend is initialized and a new frame begins
- WHEN: `rust_gfx_preprocess` is called
- THEN: The renderer blend mode is set to `BLENDMODE_NONE` before clearing

Why it matters:
- Establishes clean renderer state for subsequent ScreenLayer/ColorLayer calls
- Matches C reference behavior (`sdl2_pure.c` line 381)

### REQ-POST-010: Postprocess Presents Frame
**Requirement text**: When `rust_gfx_postprocess` is called, the backend shall call `canvas.present()` to display the composed frame.

Behavior contract:
- GIVEN: The backend is initialized and all compositing calls have completed
- WHEN: `rust_gfx_postprocess` is called
- THEN: `canvas.present()` is called, displaying the frame

Why it matters:
- Frame becomes visible to the player

### REQ-POST-020: Postprocess Does Not Upload
**Requirement text**: The backend shall NOT upload surface pixel data, create textures, or call `canvas.copy` within `rust_gfx_postprocess`.

Behavior contract:
- GIVEN: ScreenLayer has composited all layers onto the renderer
- WHEN: `rust_gfx_postprocess` is called
- THEN: No texture creation, no surface upload, no canvas.copy — only present

Why it matters:
- Prevents double-rendering that clobbers transition/fade/system_box composition (REQ-INV-010)

### REQ-INV-010: Double-Render Guard
**Requirement text**: ScreenLayer and Postprocess shall not both upload and render surface data.

Behavior contract:
- GIVEN: ScreenLayer composites surfaces onto the renderer
- WHEN: Postprocess is called
- THEN: Postprocess only presents — no additional surface rendering

Why it matters:
- Core architectural invariant; violation causes visible rendering bugs

### REQ-INIT-095: Already-Initialized Guard (Moved from P13)
**Requirement text**: If `rust_gfx_init` is called when the backend is already initialized, it shall return -1 without modifying existing state.

Behavior contract:
- GIVEN: Backend already initialized
- WHEN: `rust_gfx_init` called again
- THEN: Returns -1, existing state unchanged

Why it matters:
- During iterative development, repeated init before error handling hardening (P13) could leak or overwrite state. This guard must be present from the earliest implementation phase.

### REQ-INIT-015: Driver/Renderer Params Accepted
**Requirement text**: `rust_gfx_init` shall accept driver, flags, renderer, width, and height parameters, store them in state, and not validate them beyond type safety.

Behavior contract:
- GIVEN: C caller invokes `rust_gfx_init(driver, flags, renderer, width, height)`
- WHEN: Parameters are received
- THEN: Values are stored in `RustGraphicsState`; no semantic validation is performed on driver or renderer

### REQ-INIT-020: SDL2 Software Renderer Creation
**Requirement text**: `rust_gfx_init` shall create an SDL2 software renderer with logical size 320×240 and scale quality "0" (nearest-neighbor).

Behavior contract:
- GIVEN: `rust_gfx_init` is called with valid parameters
- WHEN: SDL2 initialization proceeds
- THEN: A software renderer is created with logical size 320×240 and `SDL_HINT_RENDER_SCALE_QUALITY` set to "0"

### REQ-INIT-030: Screen Surface Allocation
**Requirement text**: `rust_gfx_init` shall create 3 screen surfaces, each 320×240, 32bpp, RGBX8888 format.

Behavior contract:
- GIVEN: `rust_gfx_init` is called
- WHEN: Surface allocation proceeds
- THEN: 3 `SDL_Surface` pointers are allocated via `SDL_CreateRGBSurface` with w=320, h=240, depth=32, RGBX8888 masks

### REQ-INIT-040: Format Conversion Surface
**Requirement text**: `rust_gfx_init` shall create a format conversion surface with 1×1 dimensions, 32bpp, RGBA format. The surface is used only as a format template; 1×1 is used instead of 0×0 because some SDL2 backends reject zero-dimension surfaces.

Behavior contract:
- GIVEN: `rust_gfx_init` is called
- WHEN: Format conversion surface is allocated
- THEN: An `SDL_Surface` is created with w=1, h=1, depth=32, RGBA masks (including alpha)

### REQ-INIT-050: Fullscreen Mode
**Requirement text**: If the fullscreen flag is set, `rust_gfx_init` shall create the window in fullscreen mode.

Behavior contract:
- GIVEN: `rust_gfx_init` is called with fullscreen flag set in `flags`
- WHEN: Window creation proceeds
- THEN: SDL window is created with `SDL_WINDOW_FULLSCREEN_DESKTOP` flag

### REQ-INIT-055: Software Scaling Activation
**Requirement text**: Software scaling shall be active when `flags & SCALE_SOFT_ONLY != 0`.

Behavior contract:
- GIVEN: `rust_gfx_init` is called with `SCALE_SOFT_ONLY` bit set in flags
- WHEN: Scaling configuration is evaluated
- THEN: `software_scaling_active` is set to true in state

### REQ-INIT-060: Scaling Buffer Allocation
**Requirement text**: When software scaling is active, `rust_gfx_init` shall allocate 3 scaling buffers.

Behavior contract:
- GIVEN: Software scaling is active (REQ-INIT-055)
- WHEN: Buffer allocation proceeds
- THEN: 3 scaling buffers are allocated with dimensions `SCREEN_WIDTH * scale_factor × SCREEN_HEIGHT * scale_factor`

### REQ-INIT-080: Success Return
**Requirement text**: `rust_gfx_init` shall return 0 on success.

Behavior contract:
- GIVEN: All initialization steps complete without error
- WHEN: `rust_gfx_init` is about to return
- THEN: Returns `0`

### REQ-INIT-090: Partial Cleanup on Failure
**Requirement text**: If `rust_gfx_init` fails partway, it shall free all resources allocated up to the failure point.

Behavior contract:
- GIVEN: `rust_gfx_init` has allocated some resources
- WHEN: A subsequent initialization step fails
- THEN: All previously allocated resources (surfaces, renderer, window) are freed before returning -1

### REQ-INIT-100: Log Diagnostic on Failure
**Requirement text**: `rust_gfx_init` shall log a diagnostic message via `rust_bridge_log_msg` on failure.

Behavior contract:
- GIVEN: An initialization step fails
- WHEN: Error handling proceeds
- THEN: A diagnostic message describing the failure is logged before returning -1

### REQ-FMT-030: Format Conv Surface Uses RGBA
**Requirement text**: The format conversion surface created during init shall use RGBA format (with alpha mask), distinct from the RGBX8888 screen surfaces.

Behavior contract:
- GIVEN: `rust_gfx_init` allocates the format conversion surface
- WHEN: Surface is created
- THEN: Surface has RGBA masks (alpha channel present)

## Implementation Tasks

### Files to modify
- `rust/src/graphics/ffi.rs`
  - **Preprocess**: Add `state.canvas.set_blend_mode(sdl2::render::BlendMode::None)` before clear
  - **Postprocess**: Replace entire body with `state.canvas.present()` only
  - **Init guard (REQ-INIT-095)**: Verify `get_gfx_state().is_some()` guard exists at top of `rust_gfx_init`. If not, add: `if get_gfx_state().is_some() { return -1; }` before any initialization logic.
  - Note: In this stub phase, Postprocess is reduced to present-only. ScreenLayer is still a no-op. This means the screen will be black (only clearing + presenting). This is intentional — ScreenLayer is implemented in P06-P08.
  - marker: `@plan PLAN-20260223-GFX-VTABLE-FIX.P03`
  - marker: `@requirement REQ-PRE-010, REQ-POST-010, REQ-POST-020, REQ-INV-010, REQ-INIT-095, REQ-INIT-015, REQ-INIT-020, REQ-INIT-030, REQ-INIT-040, REQ-INIT-050, REQ-INIT-055, REQ-INIT-060, REQ-INIT-080, REQ-INIT-090, REQ-INIT-100, REQ-FMT-030`

### Pseudocode traceability
- Uses pseudocode: component-002-preprocess lines 10–14
- Uses pseudocode: component-006-postprocess lines 1–8

## Verification Commands

```bash
# Structural gate
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `rust_gfx_preprocess` now calls `set_blend_mode(BlendMode::None)` before clear
- [ ] `rust_gfx_postprocess` body is only: `state.canvas.present()`
- [ ] No texture creation code remains in postprocess
- [ ] No `canvas.copy` call remains in postprocess
- [ ] No `texture_creator` usage remains in postprocess
- [ ] `rust_gfx_init` has already-initialized guard at top (REQ-INIT-095)
- [ ] Plan/requirement traceability comments present
- [ ] Tests compile and run

## Semantic Verification Checklist (Mandatory)
- [ ] Preprocess clears to black with BLENDMODE_NONE (REQ-PRE-010, REQ-PRE-020)
- [ ] Postprocess only presents — verified by code inspection
- [ ] No surface pixel data access in postprocess
- [ ] Already-initialized guard returns -1 without modifying state (REQ-INIT-095)
- [ ] Game will show black screen (expected — ScreenLayer not yet implemented)
- [ ] No placeholder/deferred implementation patterns in the modified functions

## Deferred Implementation Detection (Mandatory)

```bash
# Reject if these appear in modified functions:
grep -n "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/graphics/ffi.rs | grep -E "preprocess|postprocess" || echo "CLEAN"
```

## Success Criteria
- [ ] Preprocess sets blend mode to None before clear
- [ ] Postprocess is present-only (all upload/scale/copy logic removed)
- [ ] Init already-initialized guard is present and returns -1 (REQ-INIT-095)
- [ ] `cargo fmt`, `cargo clippy`, `cargo test` all pass

## Failure Recovery
- rollback: `git checkout -- rust/src/graphics/ffi.rs`
- blocking issues: if removal of postprocess upload logic breaks compilation

## Phase Completion Marker
Create: `project-plans/gfx/.completed/P03.md`

Contents:
- phase ID: P03
- timestamp
- files modified: `rust/src/graphics/ffi.rs`
- changes: preprocess blend mode added, postprocess reduced to present-only, init guard verified/added (REQ-INIT-095)
- tests: existing tests still pass
- verification outputs: cargo fmt/clippy/test results
- semantic: postprocess no longer contains any surface/texture code, init guard prevents double-init
