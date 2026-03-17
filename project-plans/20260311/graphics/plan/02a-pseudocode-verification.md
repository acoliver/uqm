# Phase 02a: Pseudocode Verification

## Phase ID
`PLAN-20260314-GRAPHICS.P02a`

## Prerequisites
- Required: Phase P02 completed

## Structural Verification
- [ ] All 10 pseudocode sections (PC-01 through PC-10) present
- [ ] Each pseudocode section maps to specific gap IDs from analysis
- [ ] Line numbers are sequential within each section
- [ ] All substantive gaps G1-G15 covered algorithmically or procedurally (G16 is cleanup)

## Semantic Verification

### PC-01 (Canvas Pixel Sync)
- [ ] Import direction correctly maps RGBX8888 → RGBA (surface → canvas)
- [ ] Export direction correctly maps RGBA → RGBX8888 (canvas → surface)
- [ ] Pitch handling accounts for potential row padding in SDL surfaces
- [ ] Flush is called before destroy to ensure writeback
- [ ] Synchronization points before presentation, transition capture, and interop reads are explicitly covered
- [ ] Alpha channel set to 255 on import (surfaces are non-alpha RGBX)

### PC-02 (Postprocess + Scanlines)
- [ ] All upload/scale/copy logic removed from postprocess
- [ ] Scanline effect only applied when SCANLINES flag is set
- [ ] Scanlines draw semi-transparent black lines on alternating rows
- [ ] Present call retained as final step

### PC-03 (Missing DCQ Push Functions)
- [ ] FilledImage includes all spec §5.1 parameters (image, x, y, scale, scale_mode, color, draw_mode, dest)
- [ ] FontChar handles both with-backing and without-backing cases
- [ ] SetMipmap associates mipmap to image by ID
- [ ] DeleteData accepts raw pointer for deferred free
- [ ] Callback wraps C function pointer safely

### PC-04 (DrawImage Parameters)
- [ ] All spec §5.1 Image parameters present: image_ref, x, y, scale, scale_mode, colormap, draw_mode, dest
- [ ] Colormap index uses -1 sentinel for "no colormap"
- [ ] Scale and scale_mode properly converted from C int to Rust enum
- [ ] Higher-level caller state propagation is preserved, not replaced with defaults

### PC-05 (SetPalette)
- [ ] New DrawCommand variant added to enum
- [ ] Handler updates active colormap / dependent invalidation state in the real render context
- [ ] Push function creates proper command (not a Callback stub)

### PC-06 (Flush + Queue Semantics)
- [ ] Empty-queue + active fade → swap_buffers with REDRAW_FADING
- [ ] Bounding box accumulated for Main screen commands
- [ ] Rendering condition variable broadcast after flush
- [ ] Livelock detection blocks producers (not just logs)
- [ ] Batch visibility and nested batching semantics are explicit
- [ ] Deferred free ordering relative to prior queued uses is explicit
- [ ] Image synchronization obligations are explicit

### PC-07 (System Box)
- [ ] Correctly identifies that this is a C-side orchestration concern
- [ ] Rust `screen()` already supports clipped compositing
- [ ] Verification needed that C sends system-box call in the correct order

### PC-08 (ReinitVideo)
- [ ] Saves current config before teardown
- [ ] Attempts reversion on failure
- [ ] Exits process on double failure (matching C behavior)
- [ ] Rebinds state to newly initialized resources after success

### PC-09 (Image Rotation)
- [ ] Inverse rotation matrix for pixel sampling
- [ ] Hotspot rotation and extent handling match object-level image semantics
- [ ] Rotated `TFB_Image` lifecycle / ownership path is explicit
- [ ] No speculative new FFI export is required without a proven caller-backed boundary

### PC-10 (C Wiring)
- [ ] Every TFB_DrawScreen_* function listed for modification
- [ ] USE_RUST_GFX guards properly scoped
- [ ] Init/uninit lifecycle calls wired for DCQ and colormap at the real lifecycle owner
- [ ] Canvas.c is only modified if concrete call-site analysis proves it necessary
- [ ] Migration-sensitive behaviors are explicitly revalidated after wiring

## Gate Decision
- [ ] All pseudocode sections semantically sound
- [ ] PASS: proceed to implementation phases
