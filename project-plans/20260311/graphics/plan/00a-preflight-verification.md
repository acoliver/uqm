# Phase 0.5: Preflight Verification

## Phase ID
`PLAN-20260314-GRAPHICS.P00.5`

## Purpose
Verify assumptions about the current codebase before implementation begins.

## Toolchain Verification
- [ ] `cargo --version`
- [ ] `rustc --version`
- [ ] `cargo clippy --version`

## Dependency Verification
- [ ] `sdl2` crate present in `rust/Cargo.toml` with `raw-window-handle` feature
- [ ] `xbrz` crate present in `rust/Cargo.toml`
- [ ] `serial_test` crate present for test serialization
- [ ] `log` crate present for bridge logging
- [ ] `anyhow` crate present for error handling

## Build Verification
- [ ] `cargo build --workspace` succeeds
- [ ] `cargo test --workspace --all-features` passes (current baseline)
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes
- [ ] `cargo fmt --all --check` passes

## Type/Interface Verification

### Canvas system
- [ ] `Canvas::new_rgba(w, h)` exists in `rust/src/graphics/tfb_draw.rs`
- [ ] `Canvas::with_pixels_mut()` exists for direct pixel access
- [ ] `Canvas::pixels()` exists for read access
- [ ] `Canvas::width()` / `Canvas::height()` exist
- [ ] `Canvas::format()` exists returning `CanvasFormat`
- [ ] `SurfaceCanvas` struct in `rust/src/graphics/canvas_ffi.rs` has `surface`, `canvas`, `width`, `height` fields

### DCQ system
- [ ] `DrawCommand` enum in `rust/src/graphics/dcqueue.rs` has all 16 variants: Line, Rect, Image, FilledImage, FontChar, Copy, CopyToImage, SetMipmap, DeleteImage, DeleteData, SendSignal, ReinitVideo, SetPalette, ScissorEnable, ScissorDisable, Callback
- [ ] `DrawCommandQueue::push()` exists
- [ ] `DrawCommandQueue::process_commands()` exists
- [ ] `handle_command()` dispatches all 16 variants
- [ ] `ImageRef`, `FontCharRef`, `ColorMapRef` exist in dcqueue.rs
- [ ] Batch/unbatch/set_screen are represented separately from queue commands (not counted as command variants)

### FFI system
- [ ] `rust_gfx_postprocess()` exists in `rust/src/graphics/ffi.rs`
- [ ] `rust_gfx_screen()` exists with screen/alpha/rect parameters
- [ ] `rust_gfx_process_events()` exists and is reachable from the active graphics path
- [ ] `RustGraphicsState` has `canvas` (SDL2 renderer), `surfaces`, `flags` fields
- [ ] `SDL_Surface` struct defined in ffi.rs with `pixels`, `w`, `h`, `pitch` fields

### C headers
- [ ] `rust_gfx.h` declares all `rust_canvas_*`, `rust_cmap_*`, `rust_dcq_*` functions
- [ ] `sdl_common.c` has `#ifdef USE_RUST_GFX` gates for backend selection
- [ ] `TFB_DrawScreen_*` functions exist in `sc2/src/libs/graphics/tfb_draw.c`

## Call-Path Feasibility

### Canvas pixel sync path
- [ ] `SDL_Surface.pixels` field is accessible as `*mut c_void` from Rust
- [ ] `SDL_Surface.pitch` gives row stride in bytes
- [ ] Surface format is RGBX8888 (4 bytes per pixel, no alpha)
- [ ] Canvas internal format is RGBA (4 bytes per pixel)

### C→Rust DCQ wiring path
- [ ] C `TFB_DrawScreen_Line` in `tfb_draw.c` can be modified to call `rust_dcq_push_drawline`
- [ ] The `USE_RUST_GFX` preprocessor guard is available in `tfb_draw.c` (or can be added via include)
- [ ] Linker will resolve `rust_dcq_push_*` symbols from `libuqm_rust.a`

### Presentation path
- [ ] `rust_gfx_screen()` accesses `state.canvas` (SDL2 renderer)
- [ ] `TextureCreator` accessible from `state.canvas` for streaming texture creation
- [ ] `BlendMode::Blend` and `BlendMode::None` available

### Event path
- [ ] The active Rust graphics lifecycle path initializes and retains SDL event-pump state
- [ ] `rust_gfx_process_events()` forwards SDL events through the established external contract without owning gameplay interpretation
- [ ] Reinit/uninit code paths clearly show where event-pump state is replaced or destroyed

## Test Infrastructure Verification
- [ ] `rust/src/graphics/canvas_ffi.rs` has `#[cfg(test)] mod tests` with `make_test_surface` helper
- [ ] `rust/src/graphics/dcq_ffi.rs` has `#[cfg(test)] mod tests` with `serial` attribute
- [ ] `rust/src/graphics/ffi.rs` has `#[cfg(test)] mod tests`
- [ ] `rust/src/graphics/dcqueue.rs` has `#[cfg(test)] mod tests`

## Blocking Issues
[To be filled during execution. If non-empty, stop and revise plan first.]

## Gate Decision
- [ ] PASS: proceed
- [ ] FAIL: revise plan
