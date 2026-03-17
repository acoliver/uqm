# Phase 03: Canvas Pixel Synchronization

## Phase ID
`PLAN-20260314-GRAPHICS.P03`

## Prerequisites
- Required: Phase P02 completed
- Verify: Pseudocode PC-01 reviewed
- Expected files from previous phase: `02-pseudocode.md`

## Requirements Implemented (Expanded)

### REQ-CAN-006: Surface-backed canvas coherence
**Requirement text**: When a canvas wraps a backend surface that is later read for presentation, transition capture, or interoperability, the subsystem shall preserve pixel coherence between the canvas view and the underlying surface-visible pixel data.

Behavior contract:
- GIVEN: A `SurfaceCanvas` wrapping an SDL_Surface with existing pixel data
- WHEN: The canvas is created
- THEN: Existing surface pixels are imported into the Rust Canvas

- GIVEN: Drawing operations have been performed on a `SurfaceCanvas`
- WHEN: The canvas is flushed or destroyed
- THEN: Modified pixels are written back to the underlying SDL_Surface

- GIVEN: Presentation compositing, transition capture, or interoperability readback is about to read the surface
- WHEN: The read occurs
- THEN: The underlying SDL_Surface reflects all prior Rust canvas writes that must be externally visible at that synchronization point

### REQ-INT-006: Transition-source compatibility
Transition capture must observe already-flushed main-screen pixels at the point of capture. This phase defines the synchronization hook(s) required so Rust canvas writes are committed before transition-source reads.

### REQ-INT-007: Extra-screen workflow compatibility
Extra-screen copy workflows depend on surface-backed canvas writes being visible to later copy/read operations. This phase defines the synchronization hook(s) required so those workflows see current pixels.

Why it matters:
- The presentation layer reads directly from SDL_Surface pixel memory during compositing
- Transition capture reads current main-screen SDL surface pixels
- Copy/read interoperability paths rely on surface-backed coherence before destroy-time
- Destroy-only writeback is insufficient for the specification contract

## Implementation Tasks

### Files to modify
- `rust/src/graphics/canvas_ffi.rs`
  - Add pixel import in `rust_canvas_from_surface()` (lines 87-125): after creating `Canvas::new_rgba(w, h)`, copy pixels from `surface.pixels` into the canvas, converting RGBX8888 → RGBA
  - Add dirty tracking on `SurfaceCanvas` so synchronization points can cheaply no-op when nothing changed
  - Add `rust_canvas_flush()` FFI export: write canvas pixels back to surface, converting RGBA → RGBX8888
  - Modify `rust_canvas_destroy()` (lines 141-148): call flush before dropping
  - marker: `@plan PLAN-20260314-GRAPHICS.P03`
  - marker: `@requirement REQ-CAN-006, REQ-INT-006, REQ-INT-007`
- Concrete readback sites that touch surface-backed canvases (must be pinned during this phase; do not leave as "and/or")
  - **Presentation read path**: identify and wire the exact function/file that reads main-screen surface pixels for presentation compositing under `USE_RUST_GFX` before that read occurs
  - **Transition-source capture path**: identify and wire the exact function/file that snapshots main-screen pixels into the transition screen
  - **Interop readback / screen-to-image copy path**: identify and wire the exact function/file(s) that read current surface pixels for external copy/read semantics
  - For each of the three sites above, either add the required flush-before-read call or document, with concrete file/function evidence, why the path already observes coherent pixels without an additional hook
- `rust/src/graphics/ffi.rs`
  - Only modify if it is one of the concrete readback sites above or is the actual centralized hook point used by those sites

### Files to verify (no changes needed unless declaration is missing)
- `sc2/src/libs/graphics/sdl/rust_gfx.h` — verify `rust_canvas_flush` is declared (add if missing)

### Pseudocode traceability
- Uses pseudocode lines: PC-01, lines 01-37

## Pixel Format Details

### Surface format (RGBX8888, little-endian macOS)
Byte layout per pixel: `[X_pad, Blue, Green, Red]`
- Offset 0: padding (ignored)
- Offset 1: Blue
- Offset 2: Green
- Offset 3: Red
- Row stride: `surface.pitch` bytes (may exceed `width * 4`)

### Canvas format (RGBA)
Byte layout per pixel: `[Red, Green, Blue, Alpha]`
- Offset 0: Red
- Offset 1: Green
- Offset 2: Blue
- Offset 3: Alpha (set to 255 on import from non-alpha surface)

### Conversion rules
- **Import** (surface → canvas): `canvas[R,G,B,A] = surface[byte3, byte2, byte1, 255]`
- **Export** (canvas → surface): `surface[byte0,byte1,byte2,byte3] = [0, canvas.B, canvas.G, canvas.R]`

Note: The exact byte mapping depends on the endianness and mask values. The masks from ffi.rs are:
- `R_MASK = 0xFF000000` → R is in the highest byte
- `G_MASK = 0x00FF0000` → G is in the second byte
- `B_MASK = 0x0000FF00` → B is in the third byte

On little-endian, a 32-bit value `0xRRGGBB00` is stored as bytes `[0x00, 0xBB, 0xGG, 0xRR]`.

So the per-pixel byte mapping is:
- Surface byte 0 = low byte of 32-bit = padding (from A_MASK=0)
- Surface byte 1 = B
- Surface byte 2 = G
- Surface byte 3 = R (high byte)

Import: `R = surface_bytes[3], G = surface_bytes[2], B = surface_bytes[1], A = 255`
Export: `surface_bytes[0] = 0, surface_bytes[1] = canvas.B, surface_bytes[2] = canvas.G, surface_bytes[3] = canvas.R`

## Synchronization Strategy (Mandatory)

This phase must leave an explicit, end-to-end synchronization design in place:

1. **Create/import:** surface pixels are imported when the wrapper is created.
2. **Mutate:** Rust canvas operations mark the wrapper dirty.
3. **Read synchronization:** any path that reads current pixels from the underlying surface must flush dirty wrappers first.
4. **Destroy fallback:** destroy still flushes as a safety net, but destroy-time export is not the primary correctness mechanism.

Acceptable implementations:
- centralized registry of surface-backed canvases keyed by surface pointer, with flush-before-read hooks
- direct shared-memory drawing into the SDL surface pixel buffer, if that fully satisfies the same external coherence contract
- another equivalent strategy, provided all three read synchronization points above are concretely covered

## Concrete synchronization-point inventory (must be completed in this phase)

For parity safety, Phase P03 must replace abstract wording with exact function/file ownership for each required hook:

1. **Presentation compositing read**
   - record exact file and function name
   - state whether the hook is direct, centralized, or unnecessary due to already-coherent storage
2. **Transition-source capture read**
   - record exact file and function name
   - state exactly where flush-before-read occurs
3. **Interop readback / screen-to-image copy**
   - record exact file and function name(s)
   - state exactly where flush-before-read occurs or why no extra hook is needed

If any of these sites are discovered to be unreachable under `USE_RUST_GFX`, record that with concrete call-path evidence rather than leaving the path unspecified.

## TDD Test Plan

### Tests to add in `canvas_ffi.rs`

1. `test_canvas_imports_surface_pixels` — Create a test surface with known pixel values, create SurfaceCanvas, verify canvas pixels match imported data
2. `test_canvas_flush_exports_pixels` — Draw on canvas via FFI, call flush, verify surface pixels reflect the drawing
3. `test_canvas_destroy_flushes` — Draw on canvas, destroy without explicit flush, verify surface pixels updated
4. `test_canvas_roundtrip_identity` — Import surface pixels, flush without modification, verify surface pixels unchanged
5. `test_canvas_partial_draw_preserves_untouched` — Import, draw in one corner, flush, verify untouched pixels preserved
6. `test_canvas_flush_before_presentation_read` — simulate draw + presentation synchronization hook, verify surface pixels visible before renderer read
7. `test_canvas_flush_before_transition_capture` — simulate draw + transition capture hook, verify capture sees flushed pixels
8. `test_canvas_flush_before_interop_read` — simulate draw + copy/readback hook, verify interop path sees current pixels

## Verification Commands

```bash
# Structural gate
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `rust_canvas_from_surface` now imports pixels from surface
- [ ] `rust_canvas_flush` exported as `#[no_mangle] pub unsafe extern "C" fn`
- [ ] `rust_canvas_destroy` calls flush before drop
- [ ] Dirty tracking or equivalent synchronization state exists
- [ ] Exact file/function ownership is documented for presentation, transition capture, and interop readback synchronization points
- [ ] Explicit flush-before-read hooks exist for presentation, transition capture, and interop readback, or each omission is justified with concrete call-path evidence
- [ ] No `.unwrap()` or `.expect()` in FFI functions
- [ ] `catch_unwind` wraps all FFI bodies

## Semantic Verification Checklist (Mandatory)
- [ ] Test creates surface with known RGBX pixels, canvas reads correct RGBA values
- [ ] Test modifies canvas, flush writes correct RGBX pixels to surface
- [ ] Roundtrip (import → flush without changes) preserves original pixel values
- [ ] Pitch-strided surfaces (pitch > width*4) handled correctly
- [ ] Presentation synchronization hook exposes pixels before compositing reads
- [ ] Transition capture synchronization hook exposes pixels before capture reads
- [ ] Interop read synchronization hook exposes pixels before readback/copy reads
- [ ] Null surface / null pixels / zero dimensions all fail safely

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/graphics/canvas_ffi.rs rust/src/graphics/ffi.rs
```

## Success Criteria
- [ ] REQ-CAN-006 behavior demonstrated via tests
- [ ] Synchronization points for REQ-INT-006 / REQ-INT-007 are concretely implemented and tested
- [ ] Exact file/function ownership for each read-synchronization point is recorded
- [ ] Verification commands pass
- [ ] Semantic checks pass

## Failure Recovery
- Rollback: `git checkout -- rust/src/graphics/canvas_ffi.rs rust/src/graphics/ffi.rs`
- Blocking issues: Surface pixel layout assumptions incorrect → verify with hex dump of test surface

## Phase Completion Marker
Create: `project-plans/20260311/graphics/.completed/P03.md`
