# Specification: GFX Drawing-Pipeline Port

Plan ID: `PLAN-20260223-GFX-FULL-PORT`

## Reference

The full specification for this feature is defined in
[`requirements.md`](./requirements.md) (EARS-format requirements),
supported by [`functional.md`](./functional.md) (behavioral spec) and
[`technical.md`](./technical.md) (implementation spec).

This document summarizes the specification scope for plan traceability.
For full contract details, see the cross-referenced sections below.

### Cross-Reference to requirements.md Sections

| Contract Area | requirements.md Section | REQ-IDs |
|---|---|---|
| Initialization | Section 1 (Initialization) | REQ-INIT-010..100 |
| Teardown | Section 2 (Teardown) | REQ-UNINIT-010..030 |
| Surface Access | Section 3 (Surface Access) | REQ-SURF-010..070 |
| Preprocess | Section 4 (Preprocess) | REQ-PRE-010..050 |
| ScreenLayer | Section 5 (ScreenLayer) | REQ-SCR-010..170 |
| Software Scaling | Section 6 (Software Scaling) | REQ-SCALE-010..070 |
| ColorLayer | Section 7 (ColorLayer) | REQ-CLR-010..070 |
| UploadTransitionScreen | Section 8 (UploadTransitionScreen) | REQ-UTS-010..030 |
| Postprocess | Section 9 (Postprocess) | REQ-POST-010..030 |
| Call Sequence | Section 10 (Call Sequence) | REQ-SEQ-010..070 |
| Threading Model | Section 11 (Threading) | REQ-THR-010..035 |
| Error Handling | Section 12 (Error Handling) | REQ-ERR-010..065 |
| Compositing Invariants | Section 13 (Compositing Invariants) | REQ-INV-005..061 |
| Pixel Format | Section 14 (Pixel Format) | REQ-FMT-010..040 |
| Window/Display | Section 15 (Window/Display) | REQ-WIN-010..030 |
| Auxiliary Functions | Section 16 (Auxiliary Functions) | REQ-AUX-010..060 |
| Non-Parity | Section 17 (Intentional Non-Parity) | REQ-NP-010..070 |
| Assumptions | Section 18 (Assumptions) | REQ-ASM-010..050 |
| FFI Safety | Section 19 (FFI Safety) | REQ-FFI-010..060 |

---

## Purpose / Problem Statement

> Cross-ref: requirements.md §1 (REQ-INIT-010..100), §5 (REQ-SCR-010..170),
> §7 (REQ-CLR-010..070), §9 (REQ-POST-010..030)

The Rust GFX backend produces a **black screen** at runtime. The root cause
is an inverted compositing architecture:

- `rust_gfx_screen` (ScreenLayer) is a **no-op** — it does nothing.
- `rust_gfx_color` (ColorLayer) is a **no-op** — fades don't work.
- `rust_gfx_postprocess` contains the **entire** 170-line upload+scale+present
  block that should be split across ScreenLayer and Postprocess.

The C caller (`TFB_SwapBuffers`) expects: Preprocess clears → ScreenLayer
composites each surface layer → ColorLayer applies fades → Postprocess
presents. The current Rust backend skips steps 2–3 entirely, then does a
single monolithic upload in step 4.

## Architectural Boundaries

> Cross-ref: requirements.md §19 (REQ-FFI-010..060), §18 (REQ-ASM-010..050)

- **Modified module**: `rust/src/graphics/ffi.rs` (primary)
- **Read-only dependencies**: `rust/src/graphics/scaling.rs`, `rust/src/graphics/pixmap.rs`
- **C-side (no changes)**: `sc2/src/libs/graphics/sdl/sdl_common.c`, `rust_gfx.h`
- **FFI boundary**: `#[no_mangle] pub extern "C" fn` symbols matching `rust_gfx.h`

## Data Contracts and Invariants

> Cross-ref: requirements.md §14 (REQ-FMT-010..040), §13 (REQ-INV-005..061)

- Screen surfaces: 320×240, 32bpp, RGBX8888 (`A_MASK=0x00000000`)
- Format conversion surface: 0×0, 32bpp, RGBA (`A_MASK=0x000000FF`)
- Textures: per-call temporary, `PixelFormatEnum::RGBX8888`
- Pixel conversion for scalers: RGBX8888 `[X,B,G,R]` ↔ RGBA `[R,G,B,A]`
- Drop order: scaled_buffers → surfaces → canvas → video → sdl_context

## Integration Points

> Cross-ref: requirements.md §10 (REQ-SEQ-010..070), §3 (REQ-SURF-010..070)

1. C vtable wiring in `sdl_common.c` (lines 85–91, 124) — unchanged
2. `TFB_SwapBuffers` call sequence (lines 275–330) — drives all vtable calls
3. Surface sharing: Rust creates via `SDL_CreateRGBSurface`, C writes pixels
4. `TFB_InitGraphics` retrieves surface pointers after `rust_gfx_init`

## Functional Requirements (REQ-IDs)

> Cross-ref: requirements.md §1–§9 (all functional requirement groups)

The full requirement set is in `requirements.md`. Key groups for this fix:

- **REQ-INIT-***: Initialization (already working, minor tweaks)
- **REQ-UNINIT-***: Teardown (already working)
- **REQ-SURF-***: Surface access (already working)
- **REQ-PRE-***: Preprocess (minor blend mode fix)
- **REQ-SCR-***: ScreenLayer (NEW — core of the fix)
- **REQ-SCALE-***: Software scaling in ScreenLayer (RELOCATED from postprocess)
- **REQ-CLR-***: ColorLayer (NEW)
- **REQ-UTS-***: UploadTransitionScreen (no-op, already correct)
- **REQ-POST-***: Postprocess (REFACTORED to present-only)
- **REQ-SEQ-***: Call sequence contract
- **REQ-THR-***: Threading model
- **REQ-ERR-***: Error handling
- **REQ-INV-***: Compositing invariants
- **REQ-FMT-***: Pixel format
- **REQ-WIN-***: Window/display
- **REQ-AUX-***: Auxiliary functions (already working)
- **REQ-NP-***: Intentional non-parity
- **REQ-ASM-***: Assumptions
- **REQ-FFI-***: FFI safety

## Error/Edge Case Expectations

> Cross-ref: requirements.md §12 (REQ-ERR-010..065)

- Uninitialized state: all FFI functions return safe defaults (REQ-ERR-010)
- Out-of-range screen index: silent return
- Null surface/pixels: silent return
- Negative rect dimensions: silent return (no `u32` overflow)
- Texture creation failure: silent return (frame missing layer, no crash) (REQ-ERR-060)
- Failed init: cleanup all allocated resources, return -1 (REQ-ERR-020)

## Non-Functional Requirements

> Cross-ref: requirements.md §11 (REQ-THR-010..035), §17 (REQ-NP-010..070)

- `unsafe` is explicitly approved for FFI boundary code
- Single-threaded access only (no synchronization primitives)
- Per-call temporary textures (sdl2 crate lifetime constraints)
- No per-frame logging (30 FPS hot path)

## Testability Requirements

> Cross-ref: requirements.md §10 (REQ-SEQ-010..070), §12 (REQ-ERR-010..065),
> §19 (REQ-FFI-010..060)

- Unit tests for rect conversion helper
- Unit tests for pixel format conversion (RGBX8888 ↔ RGBA)
- Integration test: init → preprocess → screen → color → postprocess → uninit
  (requires SDL2, may need `#[ignore]` for CI without display)
- `cargo test --workspace --all-features` must pass
- `cargo clippy` must pass with `-D warnings`
