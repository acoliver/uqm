# Phase 0.5: Preflight Verification

## Phase ID
`PLAN-20260223-GFX-VTABLE-FIX.P00.5`

## Purpose
Verify assumptions before implementation.

## Toolchain Verification
- [ ] `cargo --version` — Rust stable toolchain present
- [ ] `rustc --version` — Compiler present
- [ ] `cargo clippy --version` — Clippy available
- [ ] `cargo llvm-cov --version` (optional — coverage gate not required for this plan)

## Dependency Verification
- [ ] `sdl2 = "0.37"` present in `rust/Cargo.toml`
- [ ] `xbrz-rs = "0.1.0"` present in `rust/Cargo.toml`
- [ ] `anyhow = "1.0"` present in `rust/Cargo.toml`
- [ ] `thiserror = "1.0"` present in `rust/Cargo.toml`
- [ ] `libc = "0.2"` present in `rust/Cargo.toml`
- [ ] `sdl2` crate features: default (no special feature flags needed)
- [ ] System SDL2 library available (required by sdl2 crate)

## Type/Interface Verification

### Rust types exist and match
- [ ] `sdl2::pixels::PixelFormatEnum::RGBX8888` exists
- [ ] `sdl2::render::BlendMode::None` and `BlendMode::Blend` exist
- [ ] `sdl2::pixels::Color::RGBA(u8, u8, u8, u8)` constructor exists
- [ ] `sdl2::rect::Rect::new(i32, i32, u32, u32)` constructor exists
- [ ] `Canvas::set_blend_mode(BlendMode)` method exists
- [ ] `Canvas::set_draw_color(Color)` method exists
- [ ] `Canvas::clear()` method exists
- [ ] `Canvas::present()` method exists
- [ ] `Canvas::fill_rect(Option<Rect>)` method exists
- [ ] `Canvas::copy(&Texture, Option<Rect>, Option<Rect>)` method exists
- [ ] `Canvas::texture_creator()` method exists
- [ ] `TextureCreator::create_texture_streaming(PixelFormatEnum, u32, u32)` exists
- [ ] `Texture::update(Option<Rect>, &[u8], usize)` method exists
- [ ] `Texture::set_blend_mode(BlendMode)` method exists
- [ ] `Texture::set_alpha_mod(u8)` method exists

### Internal types exist and match
- [ ] `crate::graphics::pixmap::Pixmap::new(NonZeroU32, u32, u32, PixmapFormat)` exists
- [ ] `crate::graphics::pixmap::PixmapFormat::Rgba32` exists
- [ ] `Pixmap::data_mut() -> &mut [u8]` method exists
- [ ] `crate::graphics::scaling::Hq2xScaler` exists
- [ ] `crate::graphics::scaling::ScaleParams::new(i32, ScaleMode)` exists
- [ ] `crate::graphics::scaling::ScaleMode::Hq2x` exists
- [ ] `crate::graphics::scaling::Scaler` trait with `fn scale(&self, &Pixmap, ScaleParams) -> Result<Pixmap>` exists
- [ ] `xbrz::scale_rgba(&[u8], usize, usize, usize) -> Vec<u8>` exists
- [ ] `crate::bridge_log::rust_bridge_log_msg(&str)` exists

### FFI types exist and match
- [ ] `SDL_Surface` struct is `#[repr(C)]` with fields: flags, format, w, h, pitch, pixels, ...
- [ ] `SDL_Rect` struct is `#[repr(C)]` with fields: x, y, w, h (all `c_int`)
- [ ] `SDL_CreateRGBSurface` extern declaration present
- [ ] `SDL_FreeSurface` extern declaration present
- [ ] `GraphicsStateCell` with `UnsafeCell<Option<RustGraphicsState>>` exists
- [ ] `get_gfx_state() -> Option<&'static mut RustGraphicsState>` exists
- [ ] `set_gfx_state(Option<RustGraphicsState>)` exists

### C header matches
- [ ] `rust_gfx.h` declares all symbols exported by `ffi.rs`
- [ ] `rust_gfx_screen(int screen, Uint8 alpha, SDL_Rect *rect)` signature matches Rust
- [ ] `rust_gfx_color(Uint8 r, Uint8 g, Uint8 b, Uint8 a, SDL_Rect *rect)` matches Rust
- [ ] `rust_gfx_preprocess(int, int, int)` matches Rust
- [ ] `rust_gfx_postprocess(void)` matches Rust

## Test Infrastructure Verification
- [ ] `rust/src/graphics/ffi.rs` has a `#[cfg(test)] mod tests` section
- [ ] Existing tests compile: `cargo test --workspace --all-features`
- [ ] `test_sdl_rect_size` test exists and passes

## Call-Path Feasibility
- [ ] `TFB_SwapBuffers` in `sdl_common.c` calls the vtable in documented order
- [ ] `Rust_ScreenLayer` wrapper calls `rust_gfx_screen`
- [ ] `Rust_ColorLayer` wrapper calls `rust_gfx_color`
- [ ] `Rust_Preprocess` wrapper calls `rust_gfx_preprocess`
- [ ] `Rust_Postprocess` wrapper calls `rust_gfx_postprocess`

## Threading Model Verification (REQ-THR-010/020/030/035)
- [ ] `GraphicsStateCell` uses `UnsafeCell` (not `Mutex`/`RwLock`) — REQ-THR-030
- [ ] `unsafe impl Sync for GraphicsStateCell` present with `// SAFETY:` comment — REQ-THR-035
- [ ] No `Mutex`, `RwLock`, `Atomic*`, or `Condvar` in `ffi.rs` — REQ-THR-020
- [ ] All FFI calls originate from graphics/main thread (verified by C-side `dcqueue.c` serialization) — REQ-THR-010

## Design Constraints Verified

The following requirements are confirmed by design and do not require
dedicated implementation phases — they are architectural invariants or
no-op declarations verified during preflight and integration.

- **REQ-ASM-020**: Single-threaded assumption — all vtable calls originate from C graphics thread; no multi-thread dispatch.
- **REQ-ASM-030**: Conditional compilation behind `USE_RUST_GFX` — Rust backend is only compiled/linked when this flag is defined.
- **REQ-ASM-040**: C provides valid pointers — all `SDL_Surface*` and `SDL_Rect*` pointers passed across FFI are assumed valid by contract with C caller.
- **REQ-NP-020**: Per-call temporary textures — covered by REQ-SCR-070; textures are created and dropped within each `rust_gfx_screen` call.
- **REQ-NP-030**: Software renderer — covered by REQ-INIT-020; the backend creates an SDL2 software renderer.
- **REQ-NP-040**: Scale quality "0" — covered by REQ-INIT-020; nearest-neighbor scaling is set at init.
- **REQ-NP-050**: Scanlines no-op — the Rust backend does not implement scanline rendering; this is intentionally omitted.
- **REQ-NP-052**: Scanline pixel equivalence — since scanlines are a no-op (REQ-NP-050), pixel output is equivalent to scanlines-disabled mode.
- **REQ-NP-060**: Bits 4-6 no-op in initial implementation — scaler flag bits 4-6 are not interpreted by the Rust backend.
- **REQ-NP-061**: Bits 4-6 still activate buffer allocation — when these bits are set, scaling buffers are allocated (REQ-INIT-060) even though the scaler is not invoked.
- **REQ-NP-070**: FPS not handled by backend — FPS display is handled by the C-side `TFB_DrawFPS`, not the Rust graphics backend.
- **REQ-WIN-020**: No coordinate transform beyond SDL2 — the backend passes rect coordinates directly to SDL2 without additional transformation (for unscaled path).

## Blocking Issues
- [ ] If SDL2 system library is not installed, all graphical tests will fail
- [ ] Tests requiring a display (SDL_Init, window creation) must be `#[ignore]` for headless CI

## Gate Decision
- [ ] PASS: proceed
- [ ] FAIL: revise plan
