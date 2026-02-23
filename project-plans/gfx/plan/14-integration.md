# Phase 14: Integration

## Phase ID
`PLAN-20260223-GFX-VTABLE-FIX.P14`

## Prerequisites
- Required: Phase P13a (Error Handling Verification) completed
- Expected files: All vtable functions implemented, all tests passing

## Requirements Implemented (Expanded)

### REQ-SEQ-010: Full Call Sequence Support
**Requirement text**: The backend shall support being called in the deterministic sequence by `TFB_SwapBuffers`: preprocess → screen(MAIN) → screen(TRANSITION)? → color? → screen(MAIN, clip)? → postprocess.

Behavior contract:
- GIVEN: Game is running, `TFB_SwapBuffers` triggers vtable
- WHEN: Full call sequence executed with all optional layers
- THEN: (1) preprocess clears renderer to black with BLENDMODE_NONE, (2) screen(MAIN) uploads surfaces[0] and calls canvas.copy exactly once, (3) screen(TRANSITION) uploads surfaces[2] with BLENDMODE_BLEND and alpha modifier, (4) color() calls canvas.fill_rect exactly once with the given RGBA, (5) screen(MAIN, clip) uploads surfaces[0] with src_rect == dst_rect, (6) postprocess calls canvas.present() exactly once with no texture.update() or canvas.copy()

### REQ-SEQ-020: Subset Correctness
**Requirement text**: The backend shall produce a correctly composed frame when only a subset of conditional calls are made.

Behavior contract:
- GIVEN: No transition, no fade, no system box active
- WHEN: Only preprocess → screen(MAIN, 255, NULL) → postprocess
- THEN: Frame shows main screen correctly

### REQ-SEQ-065: Redraw Mode Invariance
**Requirement text**: The backend shall produce identical output regardless of the TFB_REDRAW mode.

Behavior contract:
- GIVEN: Same surface contents and state
- WHEN: Called with TFB_REDRAW_NO vs TFB_REDRAW_YES
- THEN: Identical pixel output

### REQ-ASM-050: TFB_GFX_NUMSCREENS Sync
**Requirement text**: The Rust-side constant shall match the C-side definition.

Behavior contract:
- GIVEN: Rust `TFB_GFX_NUMSCREENS = 3`
- WHEN: Verified against C `TFB_GFX_NUMSCREENS` in `tfb_draw.h`
- THEN: Values match

## Implementation Tasks

### Integration Contract

#### Who calls this new behavior?
- `Rust_Preprocess` (`sdl_common.c:60`) → `rust_gfx_preprocess` — **Modified**: now sets blend mode
- `Rust_ScreenLayer` (`sdl_common.c:75`) → `rust_gfx_screen` — **NEW behavior**: full compositing
- `Rust_ColorLayer` (`sdl_common.c:80`) → `rust_gfx_color` — **NEW behavior**: fade fills
- `Rust_Postprocess` (`sdl_common.c:65`) → `rust_gfx_postprocess` — **Modified**: present-only
- `Rust_UploadTransitionScreen` (`sdl_common.c:70`) → `rust_gfx_upload_transition_screen` — Unchanged (no-op)

#### What old behavior gets replaced?
- **Old `rust_gfx_postprocess`**: 170-line upload+scale+render+present block → replaced with `canvas.present()` only
- **Old `rust_gfx_screen`**: No-op with incorrect comment → replaced with full compositing implementation
- **Old `rust_gfx_color`**: No-op with TODO comment → replaced with full blend+fill implementation
- **Old `rust_gfx_preprocess`**: Missing blend mode reset → fixed

#### How can a user trigger this end-to-end?
1. Build the project with `USE_RUST_GFX` defined
2. Launch the game
3. **Main menu visible**: A pixel-dump of the first rendered frame after init shows non-zero RGB values in at least 50% of the 320×240 surface. Verification: capture frame buffer via `SDL_RenderReadPixels` after first postprocess and assert `count(pixel != 0x000000) >= (320*240)/2`.
4. **Screen transitions**: `canvas.copy()` is called with `BlendMode::Blend` and `alpha_mod < 255` during a transition frame. Verification: instrument or log-trace `set_blend_mode` and `set_alpha_mod` calls in a test harness; confirm at least one frame has 0 < alpha < 255 during a menu→menu transition.
5. **Correct resolution**: `canvas.logical_size()` returns (320, 240) and `texture.query()` returns w=320, h=240 for unscaled textures. Verification: assert in integration test after init.
6. **Software scaling**: When `--addon hq2x` is enabled, `texture.query()` returns w=640, h=480 (2× scale). Verification: assert in integration test with SCALE_SOFT_ONLY flag.

#### What state/config must migrate?
- None. The global `RustGraphicsState` struct is unchanged.
- The `flags` field already stores scaler configuration.
- All surface pointers remain unchanged.
- No config files need updating.

#### How is backward compatibility handled?
- The C vtable struct (`TFB_GRAPHICS_BACKEND`) is unchanged
- The C wrapper functions in `sdl_common.c` are unchanged
- The `rust_gfx.h` header is unchanged
- The FFI function signatures are unchanged
- Surface format and sharing protocol are unchanged
- When `USE_RUST_GFX` is not defined, C `sdl2_pure` driver is used (fallback)

### ABI Signature Verification (REQ-FFI-040)

For each vtable function, verify the Rust signature matches `rust_gfx.h`
declaration exactly. This is a mandatory checklist:

| C Declaration (`rust_gfx.h`) | Rust Signature (`ffi.rs`) | Match? |
|---|---|---|
| `int rust_gfx_init(int driver, int flags, const char *renderer, int width, int height)` | `pub extern "C" fn rust_gfx_init(driver: c_int, flags: c_int, renderer: *const c_char, width: c_int, height: c_int) -> c_int` | [ ] |
| `void rust_gfx_uninit(void)` | `pub extern "C" fn rust_gfx_uninit()` | [ ] |
| `SDL_Surface* rust_gfx_get_screen_surface(int screen)` | `pub extern "C" fn rust_gfx_get_screen_surface(screen: c_int) -> *mut SDL_Surface` | [ ] |
| `SDL_Surface* rust_gfx_get_sdl_screen(void)` | `pub extern "C" fn rust_gfx_get_sdl_screen() -> *mut SDL_Surface` | [ ] |
| `SDL_Surface* rust_gfx_get_transition_screen(void)` | `pub extern "C" fn rust_gfx_get_transition_screen() -> *mut SDL_Surface` | [ ] |
| `SDL_Surface* rust_gfx_get_format_conv_surf(void)` | `pub extern "C" fn rust_gfx_get_format_conv_surf() -> *mut SDL_Surface` | [ ] |
| `void rust_gfx_preprocess(int force_redraw, int transition_amount, int fade_amount)` | `pub extern "C" fn rust_gfx_preprocess(force_redraw: c_int, transition_amount: c_int, fade_amount: c_int)` | [ ] |
| `void rust_gfx_postprocess(void)` | `pub extern "C" fn rust_gfx_postprocess()` | [ ] |
| `void rust_gfx_screen(int screen, Uint8 alpha, SDL_Rect *rect)` | `pub extern "C" fn rust_gfx_screen(screen: c_int, alpha: u8, rect: *const SDL_Rect)` | [ ] |
| `void rust_gfx_color(Uint8 r, Uint8 g, Uint8 b, Uint8 a, SDL_Rect *rect)` | `pub extern "C" fn rust_gfx_color(r: u8, g: u8, b: u8, a: u8, rect: *const SDL_Rect)` | [ ] |
| `void rust_gfx_upload_transition_screen(void)` | `pub extern "C" fn rust_gfx_upload_transition_screen()` | [ ] |
| `int rust_gfx_process_events(void)` | `pub extern "C" fn rust_gfx_process_events() -> c_int` | [ ] |
| `int rust_gfx_toggle_fullscreen(void)` | `pub extern "C" fn rust_gfx_toggle_fullscreen() -> c_int` | [ ] |
| `int rust_gfx_is_fullscreen(void)` | `pub extern "C" fn rust_gfx_is_fullscreen() -> c_int` | [ ] |
| `int rust_gfx_set_gamma(float gamma)` | `pub extern "C" fn rust_gfx_set_gamma(gamma: f32) -> c_int` | [ ] |
| `int rust_gfx_get_width(void)` | `pub extern "C" fn rust_gfx_get_width() -> c_int` | [ ] |
| `int rust_gfx_get_height(void)` | `pub extern "C" fn rust_gfx_get_height() -> c_int` | [ ] |

Verification procedure:
```bash
# Extract Rust exports
grep '#\[no_mangle\]' -A1 rust/src/graphics/ffi.rs | grep 'pub extern' | sort

# Extract C declarations
grep 'rust_gfx_' sc2/src/libs/graphics/sdl/rust_gfx.h | grep -v '//' | sort

# Manual comparison of each pair
```

### Files to verify (no changes expected)
- `sc2/src/libs/graphics/sdl/sdl_common.c` — verify vtable wiring unchanged
- `sc2/src/libs/graphics/sdl/rust_gfx.h` — verify declarations match Rust exports (per ABI checklist above)
- `rust/Cargo.toml` — verify dependencies unchanged
- `rust/src/graphics/mod.rs` — verify module structure unchanged

### Orphaned Requirement Coverage

The following requirements were identified as insufficiently traced to
concrete plan phases. They are anchored here with explicit verification:

#### Threading Model (REQ-THR-010/020/030/035)

These requirements are architectural constraints verified by code
inspection during integration:

- **REQ-THR-010** (single-thread assumption): Verified by confirming no
  `std::thread::spawn` or `tokio`/`async` in `ffi.rs`, and that all calls
  originate from C's graphics thread.
  - Test: `test_no_thread_spawning_in_ffi` — @requirement REQ-THR-010
    - GIVEN: The `ffi.rs` source code
    - WHEN: Searched for thread spawning constructs
    - THEN: No `thread::spawn`, `tokio`, `async`, `rayon` found

- **REQ-THR-020** (no synchronization primitives): Verified by confirming
  no `Mutex`, `RwLock`, `AtomicBool`, or `Condvar` in `ffi.rs`.
  - Test: `test_no_sync_primitives_in_ffi` — @requirement REQ-THR-020
    - GIVEN: The `ffi.rs` source code
    - WHEN: Searched for synchronization constructs
    - THEN: No `Mutex`, `RwLock`, `Atomic*`, `Condvar` found

- **REQ-THR-030** (UnsafeCell for global state): Verified by confirming
  `GraphicsStateCell` uses `UnsafeCell`.
  - Verified by: code inspection of `GraphicsStateCell` definition

- **REQ-THR-035** (unsafe impl Sync with SAFETY comment): Verified by
  confirming `unsafe impl Sync for GraphicsStateCell` exists with safety
  proof comment.
  - Verified by: code inspection of `unsafe impl Sync` block

#### Call Sequence (REQ-SEQ-030/040/050/060)

- **REQ-SEQ-030** (tolerate no-vtable frame): The backend already handles
  this — if `TFB_SwapBuffers` exits early, no vtable functions are called,
  and the backend state is unchanged.
  - Test: `test_no_vtable_frame_tolerance` — @requirement REQ-SEQ-030
    - GIVEN: Backend is initialized (or uninitialized)
    - WHEN: No vtable functions are called for an entire frame cycle
    - THEN: Backend state is unchanged, no crash, no resource leak
  - Verified by: code inspection — no frame-counting or mandatory-call assumptions

- **REQ-SEQ-040** (REDRAW_EXPOSE full repaint): The backend always does a
  full repaint regardless of redraw mode (REQ-SEQ-065), so EXPOSE is
  inherently handled.
  - Test: `test_redraw_mode_invariance` — @requirement REQ-SEQ-040, REQ-SEQ-065
    - GIVEN: Backend is initialized (or uninitialized for safety)
    - WHEN: preprocess called with different force_redraw values (0, 1, 2, 3)
    - THEN: No crash, behavior is identical (force_redraw is unused per REQ-PRE-030)

- **REQ-SEQ-050** (REDRAW_FADING correct rendering): Same as above — the
  backend unconditionally renders all layers presented to it.
  - Test: `test_redraw_fading_renders_correctly` — @requirement REQ-SEQ-050
    - GIVEN: Backend is initialized (or uninitialized for safety)
    - WHEN: preprocess called with force_redraw=2 (TFB_REDRAW_FADING)
    - THEN: No crash; behavior identical to other redraw modes (confirmed by REQ-SEQ-065)
  - Also verified by: `test_redraw_mode_invariance`

- **REQ-SEQ-060** (REDRAW_YES after REINITVIDEO): Same as above — full
  repaint always occurs.
  - Test: `test_redraw_after_reinitvideo` — @requirement REQ-SEQ-060
    - GIVEN: Backend is initialized (or uninitialized for safety)
    - WHEN: preprocess called with force_redraw=1 (TFB_REDRAW_YES), simulating post-REINITVIDEO
    - THEN: No crash; full repaint occurs unconditionally (confirmed by REQ-SEQ-065)
  - Also verified by: `test_redraw_mode_invariance`

#### FFI Safety (REQ-FFI-040/060)

- **REQ-FFI-040** (no_mangle + extern "C"): Verified by the ABI Signature
  Verification checklist above — every exported function must have both
  `#[no_mangle]` and `extern "C"`.
  - Test: `test_all_exports_have_no_mangle` — @requirement REQ-FFI-040
    - GIVEN: The `ffi.rs` source code
    - WHEN: All `pub extern "C" fn` declarations are enumerated
    - THEN: Each has a preceding `#[no_mangle]` attribute

- **REQ-FFI-060** (no re-entrant FFI calls): Verified by code inspection —
  no FFI-exported function calls another FFI-exported function. Each
  `get_gfx_state()` call does not overlap with another.
  - Verified by: code inspection — grep for `rust_gfx_` calls within function bodies

### Integration tests to add
- `test_full_vtable_sequence_uninitialized` — all vtable functions called without init, no crash
- `test_constants_match_c_side` — verify TFB_GFX_NUMSCREENS = 3, SCREEN_WIDTH = 320, SCREEN_HEIGHT = 240
- `test_sdl_surface_layout_matches_c` — verify struct size/offsets
- `test_no_thread_spawning_in_ffi` — @requirement REQ-THR-010 (grep-based)
- `test_no_sync_primitives_in_ffi` — @requirement REQ-THR-020 (grep-based)
- `test_no_vtable_frame_tolerance` — @requirement REQ-SEQ-030
- `test_redraw_mode_invariance` — @requirement REQ-SEQ-040, REQ-SEQ-050, REQ-SEQ-060, REQ-SEQ-065
- `test_redraw_fading_renders_correctly` — @requirement REQ-SEQ-050 (may alias test_redraw_mode_invariance)
- `test_redraw_after_reinitvideo` — @requirement REQ-SEQ-060 (may alias test_redraw_mode_invariance)
- `test_all_exports_have_no_mangle` — @requirement REQ-FFI-040
- `test_no_reentrant_ffi_calls` — @requirement REQ-FFI-060 (grep-based)

### Files to modify
- `rust/src/graphics/ffi.rs`
  - Add integration-level tests to `#[cfg(test)] mod tests`
  - marker: `@plan PLAN-20260223-GFX-VTABLE-FIX.P14`
  - marker: `@requirement REQ-SEQ-010, REQ-SEQ-020, REQ-SEQ-030, REQ-SEQ-040, REQ-SEQ-050, REQ-SEQ-060, REQ-SEQ-065, REQ-ASM-050, REQ-THR-010, REQ-THR-020, REQ-THR-030, REQ-THR-035, REQ-FFI-040, REQ-FFI-060`

### Pseudocode traceability
- Validates all pseudocode components end-to-end

## Verification Commands

```bash
# Full verification suite
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features

# Verify C header matches Rust exports
# (Manual check — compare rust_gfx.h declarations with ffi.rs #[no_mangle] exports)

# Verify no deferred patterns ANYWHERE in the file
grep -c "TODO\|FIXME\|HACK\|todo!\|unimplemented!\|for now\|will be implemented\|placeholder" rust/src/graphics/ffi.rs | xargs test 0 -eq && echo "CLEAN" || echo "FAIL"

# Verify vtable completeness — all 5 entry points + init/uninit + accessors + aux
grep -c '#\[no_mangle\]' rust/src/graphics/ffi.rs
# Expected: >= 15 (init, uninit, 3 accessors, 5 vtable, events, fullscreen×2, gamma, width, height)

# Total test count
grep -c '#\[test\]' rust/src/graphics/ffi.rs
# Expected: >= 30 (increased to account for THR/SEQ/FFI verification tests)
```

## Structural Verification Checklist
- [ ] All 5 vtable functions are fully implemented (no stubs)
- [ ] `rust_gfx_init` and `rust_gfx_uninit` unchanged or minimally modified
- [ ] All surface accessors unchanged
- [ ] All auxiliary functions unchanged
- [ ] Integration tests added
- [ ] No deferred patterns in entire file
- [ ] All `#[no_mangle] pub extern "C" fn` match `rust_gfx.h` declarations (ABI checklist complete)
- [ ] Threading model constraints verified (REQ-THR-010/020/030/035)
- [ ] No re-entrant FFI calls (REQ-FFI-060)
- [ ] All exports use `#[no_mangle]` + `extern "C"` (REQ-FFI-040)

## Semantic Verification Checklist (Mandatory)

Each item includes an explicit verification method.

- [ ] **Preprocess**: Clears to black with BLENDMODE_NONE — Verify by: read `rust_gfx_preprocess` body and confirm `set_blend_mode(BlendMode::None)` precedes `clear()`; compare call sequence against `sdl2_pure.c` line 381 side-by-side.
- [ ] **ScreenLayer**: Uploads surface, sets blend/alpha, renders texture — Verify by: read `rust_gfx_screen` body and confirm `texture.update()` → `set_blend_mode` → `set_alpha_mod` → `canvas.copy()` sequence; compare against `sdl2_pure.c` Rust_ScreenLayer.
- [ ] **ScreenLayer scaled**: Converts pixels, runs scaler, uploads, scales rect — Verify by: read `screen_layer_scaled` body and confirm RGBX→RGBA→scaler→RGBA→RGBX→upload pipeline; run `test_pixel_conversion_roundtrip` and confirm lossless round-trip.
- [ ] **ColorLayer**: Sets blend mode, color, fills rect — Verify by: read `rust_gfx_color` body and confirm `set_blend_mode` → `set_draw_color` → `fill_rect` ordering; compare against `sdl2_pure.c` Rust_ColorLayer.
- [ ] **Postprocess**: Only calls present() — Verify by: `grep -c 'texture\|canvas.copy\|update' <postprocess body>` returns 0; confirm body is exactly `state.canvas.present()`.
- [ ] **UploadTransitionScreen**: No-op — Verify by: confirm function body is empty (no state access); `wc -l <function body>` ≤ 5 lines including comments.
- [ ] **Error handling**: All FFI functions safe when uninitialized — Verify by: `test_full_vtable_sequence_uninitialized` passes (calls every extern "C" fn with no prior init, asserts no panic/crash).
- [ ] **Double-render guard**: REQ-INV-010 satisfied — Verify by: `grep -n 'texture\|canvas.copy\|update' <postprocess body>` returns empty; `grep -n 'canvas.present' <screen body>` returns empty.
- [ ] **Integration path**: C → vtable → Rust → SDL2 is complete — Verify by: `cargo build --release` succeeds and `nm -gU <lib> | grep rust_gfx_` lists all 17 expected symbols.
- [ ] **No unused imports**: Verify by: `cargo clippy -- -D unused-imports` passes.
- [ ] **No dead code**: Verify by: `cargo clippy -- -D dead-code` passes; every helper function in ffi.rs has at least one call site (grep confirms).

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|todo!\|unimplemented!\|placeholder\|for now\|will be implemented" rust/src/graphics/ffi.rs && echo "FAIL" || echo "CLEAN"
```

## Success Criteria
- [ ] All vtable functions fully implemented
- [ ] All tests pass (>= 30 total)
- [ ] All cargo gates pass
- [ ] No deferred patterns
- [ ] C header matches Rust exports (ABI signature checklist complete)
- [ ] Integration tests verify end-to-end sequence safety
- [ ] Threading model constraints verified (REQ-THR-*)
- [ ] Call sequence requirements traced (REQ-SEQ-030/040/050/060)
- [ ] FFI safety requirements verified (REQ-FFI-040/060)
- [ ] **Runtime verification**: After init+preprocess+screen(MAIN,255,NULL)+postprocess, `SDL_RenderReadPixels` of the 320×240 target shows non-zero RGB in ≥50% of pixels (not a black screen)

## Failure Recovery
- rollback: `git stash` or `git checkout -- rust/src/graphics/ffi.rs`
- blocking issues: runtime rendering bugs that need display-server testing

## Phase Completion Marker
Create: `project-plans/gfx/.completed/P14.md`

Contents:
- phase ID: P14
- timestamp
- files modified: `rust/src/graphics/ffi.rs`
- total tests: count
- total #[no_mangle] exports: count
- verification: full cargo suite output
- semantic: end-to-end integration verified
- runtime: game visual state (if tested)
