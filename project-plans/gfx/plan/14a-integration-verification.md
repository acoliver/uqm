# Phase 14a: Integration — Verification

## Phase ID
`PLAN-20260223-GFX-VTABLE-FIX.P14a`

## Prerequisites
- Required: Phase P14 completed
- Expected artifacts: All implementation complete, integration tests added

## Verification Commands

```bash
# Full build + test + lint
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features

# No deferred patterns
grep -c "TODO\|FIXME\|HACK\|todo!\|unimplemented!\|for now\|will be implemented\|placeholder" rust/src/graphics/ffi.rs | xargs test 0 -eq && echo "CLEAN" || echo "FAIL"

# Export count
echo "Exports:"
grep -c '#\[no_mangle\]' rust/src/graphics/ffi.rs

# Test count
echo "Tests:"
grep -c '#\[test\]' rust/src/graphics/ffi.rs

# File size (should be smaller than original 676 lines for postprocess, plus new code)
echo "Lines:"
wc -l rust/src/graphics/ffi.rs
```

## Structural Verification Checklist
- [ ] Zero deferred patterns in ffi.rs
- [ ] All `#[no_mangle]` exports match rust_gfx.h (ABI signature checklist from P14 complete)
- [ ] >= 30 tests in ffi.rs (increased from 25 to account for new THR/SEQ/FFI tests)
- [ ] `cargo fmt` passes
- [ ] `cargo clippy` passes with `-D warnings`
- [ ] `cargo test` passes (all tests green)
- [ ] Threading model verified: REQ-THR-010/020/030/035 (no thread spawning, no sync primitives, UnsafeCell used, Sync impl present with SAFETY comment)
- [ ] Panic boundary policy verified: REQ-FFI-030 (every extern "C" fn annotated PANIC-FREE or wrapped in catch_unwind)
- [ ] All exports annotated: REQ-FFI-040 (`#[no_mangle]` + `extern "C"`)
- [ ] No re-entrant FFI calls: REQ-FFI-060
- [ ] Call sequence coverage: REQ-SEQ-030/040/050/060/065/070 all traced to tests

## Semantic Verification Checklist (Mandatory)

### Function-by-function verification

| Function | Expected Behavior | Verification Method | Verified |
|---|---|---|---|
| `rust_gfx_init` | Creates SDL2 context, window, renderer, surfaces, scaling buffers | `test_init_returns_zero` passes; `get_screen_surface(0..2)` returns non-null after init | ⬜ |
| `rust_gfx_uninit` | Frees all resources in correct order | `test_uninit_then_accessors_return_null` passes; valgrind/ASAN shows no leaks | ⬜ |
| `rust_gfx_get_screen_surface(i)` | Returns surfaces[i] or null | `test_get_screen_surface_valid_indices` asserts non-null for 0,1,2 and null for 3,-1 | ⬜ |
| `rust_gfx_get_sdl_screen` | Returns surfaces[0] | Assert `get_sdl_screen() == get_screen_surface(0)` (pointer equality) | ⬜ |
| `rust_gfx_get_transition_screen` | Returns surfaces[2] | Assert `get_transition_screen() == get_screen_surface(2)` (pointer equality) | ⬜ |
| `rust_gfx_get_format_conv_surf` | Returns format_conv_surf | Assert returned pointer is non-null and surface has RGBA masks (alpha != 0) | ⬜ |
| `rust_gfx_preprocess` | set_blend_mode(None) + set_draw_color(black) + clear | Code inspection: confirm 3-call sequence in function body; `test_preprocess_uninitialized_no_panic` | ⬜ |
| `rust_gfx_screen` (unscaled) | Upload surface → texture, set blend/alpha, canvas.copy | Code inspection: confirm `texture.update` → `set_blend_mode` → `canvas.copy` sequence; tests from P07 pass | ⬜ |
| `rust_gfx_screen` (scaled) | Convert → scale → convert → upload → copy | Code inspection: confirm RGBX→RGBA→scaler→RGBA→RGBX pipeline; `test_pixel_conversion_roundtrip` passes | ⬜ |
| `rust_gfx_color` | set_blend_mode + set_draw_color + fill_rect | Code inspection: confirm 3-call ordering matches `sdl2_pure.c`; `test_color_uninitialized_no_panic` passes | ⬜ |
| `rust_gfx_postprocess` | canvas.present() only | `grep -c 'texture\|canvas.copy\|update' <body>` == 0; body is exactly `state.canvas.present()` | ⬜ |
| `rust_gfx_upload_transition_screen` | No-op (documented invariant) | Function body contains no state access (empty or return-only); `wc -l <body>` ≤ 5 | ⬜ |
| `rust_gfx_process_events` | Poll events, return 1 on quit | `test_process_events_uninitialized` returns 0; code inspection confirms quit→1 mapping | ⬜ |
| `rust_gfx_toggle_fullscreen` | Toggle state, return 1/0/-1 | `test_toggle_fullscreen_uninitialized` returns -1; code inspection confirms state flip | ⬜ |
| `rust_gfx_is_fullscreen` | Return 1/0 | `test_is_fullscreen_uninitialized` returns 0; after init with fullscreen flag, returns 1 | ⬜ |
| `rust_gfx_set_gamma` | Return -1 (unsupported) | `test_set_gamma_returns_minus_one`; code inspection confirms unconditional -1 | ⬜ |
| `rust_gfx_get_width` | Return 320 | `test_get_width_returns_320` asserts == 320 after init | ⬜ |
| `rust_gfx_get_height` | Return 240 | `test_get_height_returns_240` asserts == 240 after init | ⬜ |

### Invariant verification
- [ ] REQ-INV-010: Postprocess only presents (no upload/copy) — Verify by: `grep -En 'texture|canvas\.copy|update' <postprocess body>` returns empty
- [ ] REQ-INV-020: UploadTransitionScreen is no-op (ScreenLayer uploads unconditionally) — Verify by: function body is empty/return-only (code inspection)
- [ ] REQ-INV-030: Backend does not modify call sequence — Verify by: `grep -En 'preprocess\|postprocess\|screen\|color' <any vtable fn body>` shows no cross-calls between vtable functions
- [ ] REQ-INV-040: Repeated postprocess is safe — Verify by: `test_repeated_postprocess_no_crash` calls postprocess 100× without panic
- [ ] REQ-INV-050: Repeated preprocess clears each time — Verify by: `test_repeated_preprocess_no_crash` calls preprocess 100× without panic; code inspection confirms clear() on every call
- [ ] REQ-INV-060: Failed init leaves fully uninitialized state — Verify by: `test_init_partial_failure_cleanup` confirms `get_gfx_state()` is None after failed init

### Threading Model Coverage (REQ-THR)
- [ ] REQ-THR-010: No thread spawning in ffi.rs — verified by `test_no_thread_spawning_in_ffi`
- [ ] REQ-THR-020: No synchronization primitives in ffi.rs — verified by `test_no_sync_primitives_in_ffi`
- [ ] REQ-THR-030: `GraphicsStateCell` uses `UnsafeCell` — verified by code inspection
- [ ] REQ-THR-035: `unsafe impl Sync` has `// SAFETY:` comment — verified by code inspection

### Call Sequence Coverage (REQ-SEQ)
- [ ] REQ-SEQ-010: Full call sequence supported — verified by `test_full_vtable_sequence_uninitialized`
- [ ] REQ-SEQ-020: Subset correctness (minimal sequence works) — verified by code inspection
- [ ] REQ-SEQ-030: No-vtable-frame tolerance — verified by `test_no_vtable_frame_tolerance`
- [ ] REQ-SEQ-040: REDRAW_EXPOSE full repaint — verified by `test_redraw_mode_invariance`
- [ ] REQ-SEQ-050: REDRAW_FADING correct rendering — verified by `test_redraw_mode_invariance`
- [ ] REQ-SEQ-060: REDRAW_YES after REINITVIDEO — verified by `test_redraw_mode_invariance`
- [ ] REQ-SEQ-065: Redraw mode invariance — verified by `test_redraw_mode_invariance`
- [ ] REQ-SEQ-070: Out-of-sequence robustness — verified by `test_out_of_sequence_no_crash` (P13)

### FFI Safety Coverage (REQ-FFI)
- [ ] REQ-FFI-030: No panic across FFI — verified by panic boundary audit (P13), every `extern "C" fn` has `// PANIC-FREE:` or `catch_unwind`
- [ ] REQ-FFI-040: `#[no_mangle]` + `extern "C"` on all exports — verified by `test_all_exports_have_no_mangle` and ABI signature checklist (P14)
- [ ] REQ-FFI-060: No re-entrant mutable access — verified by `test_no_reentrant_ffi_calls` and code inspection

### End-to-End Verification
- [ ] `cargo build --release` succeeds (Rust library links) — Verify by: exit code 0 from `cargo build --release`
- [ ] Full C+Rust build succeeds (if build system is set up) — Verify by: `make` or `./build.sh` exits 0; `nm -gU <lib> | grep rust_gfx_` lists all 17 expected symbols
- [ ] Runtime test: game launches and displays content — Verify by: `SDL_RenderReadPixels` after first frame shows ≥50% non-zero pixels in 320×240 surface, OR manual screenshot comparison against C backend (≤5% pixel-level difference)
- [ ] Runtime test: screen transitions work (fade in/out) — Verify by: capture 10 consecutive frames during menu transition; at least 3 frames have `color()` called with 0 < alpha < 255 (partial fade visible)
- [ ] Runtime test: system box visible during fades — Verify by: during a fade, capture frame buffer; the system box region (screen=0, clip rect) has pixel values distinct from the fade color

## Success Criteria
- [ ] All structural checks pass
- [ ] All semantic checks pass (function-by-function and invariant)
- [ ] All cargo gates pass
- [ ] REQ-THR coverage complete (010/020/030/035)
- [ ] REQ-SEQ coverage complete (010/020/030/040/050/060/065/070)
- [ ] REQ-FFI coverage complete (030/040/060)
- [ ] Plan is complete — all phases executed

## Plan Completion Summary

When this phase passes, the plan `PLAN-20260223-GFX-VTABLE-FIX` is complete.

### What was done
1. **Preprocess**: Added blend mode reset (REQ-PRE-010)
2. **ScreenLayer**: Full compositing implementation — unscaled and scaled paths (REQ-SCR-*, REQ-SCALE-*)
3. **ColorLayer**: Full fade/tint implementation (REQ-CLR-*)
4. **Postprocess**: Reduced to present-only (REQ-POST-*, REQ-INV-010)
5. **Error handling**: All FFI functions safe when uninitialized (REQ-ERR-*), panic boundary policy enforced (REQ-FFI-030)
6. **Pixel conversion helpers**: Extracted and tested (REQ-SCALE-060/070)
7. **Integration tests**: End-to-end sequence safety verified
8. **Threading model**: Verified single-threaded access pattern (REQ-THR-*)
9. **ABI verification**: All Rust signatures match C header declarations (REQ-FFI-040)
10. **Call sequence**: All SEQ requirements traced and verified (REQ-SEQ-*)

### What changed
- **Single file modified**: `rust/src/graphics/ffi.rs`
- **Net code change**: ~170 lines removed (old postprocess), ~150 lines added (ScreenLayer + ColorLayer + helpers + tests)
- **No C-side changes**: vtable wiring, header, build system all unchanged

### What the user sees
- The game displays content instead of a black screen
- Screen transitions fade correctly
- Fade-to-black and fade-to-white work
- System box remains visible during fades
- Software scaling (HQ2x, xBRZ) works when enabled

## Phase Completion Marker
Create: `project-plans/gfx/.completed/P14a.md`

Contents:
- phase ID: P14a (final)
- timestamp
- total files modified: 1 (`rust/src/graphics/ffi.rs`)
- total tests: count
- all verification outputs
- plan status: COMPLETE
