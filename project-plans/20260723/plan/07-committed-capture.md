# Phase 07: Present-Call Observation and Locked Logical Capture

Phase ID: `PLAN-20260723-RUNTIME-AUTOMATION.P07`

Require `.completed/P06.md`. Own `REQ-PRESENT-001..002`, `REQ-SHOT-001..006`, present/capture integration for `REQ-TRACE-001..003`, graphics portions of `REQ-FFI-001/004`, atomic capture-generation integration, and the P07 extension of `REQ-TEST-003`.

## Source integration

Modify `rust_gfx_postprocess` forward while preserving the present-only user edit. Catch panic around the complete extern shell. After normal `canvas.present()` return, acquire-load atomic requested generation; copy only nonzero generation under graphics access; release graphics state; then use the standard pure reserve/unlock/effect/ordered-publish/validated-commit shell. Never recursively acquire graphics or hold runtime/I/O locks during graphics/SDL.

Use `sdl2::sys::SDL_Surface`/`SDL_PixelFormat` from the linked ABI or narrow C accessors compiled against linked headers for w/h/pitch/pixels/BPP/masks and `SDL_MUSTLOCK`. Do not dereference/extend the current partial hand-written `SDL_Surface.format: *mut c_void` as pixel-format layout. Real `SDL_LockSurface` failure is terminal/no-read; a successful lock immediately owns an RAII real-unlock guard across every error/panic.

Completion validates exact pending generation/request sequence and performs destination-directory temporary create-new -> encode/flush/recover/sync/close -> exclusive no-replace final publication -> directory sync classification -> ordered present/capture record -> matching commit. Stale/duplicate/zero/future generation and every I/O/record failure cannot advance.

## Production-linked present harness

Add `present_boundary_main.rs`/`present_boundary_shim.c` under existing `rust/tests/c_harness`, declare `[[bin]] automation-present-boundary ... test=false`, and add it to `run-linked.sh`. As in P06, `build.rs` unconditionally compiles the shim with `cargo_metadata(false)` and emits only explicit `rustc-link-arg-bin=automation-present-boundary` arguments for the shim and `libuqm_c`; it does not branch on a nonexistent per-bin build-script environment variable. Both harness mains explicitly call a small `uqm_rust` link anchor so required Rust exports are retained; build.rs emits the shim archive before `-luqm_c`, uses the existing `OUT_DIR` search path, and preserves external-library ordering.

Extend the P00/P00a proven linked mechanism. The present shim initializes prerequisites and calls `TFB_InitGraphics` exactly once; that function already calls `Init_DrawCommandQueue`, so the harness never calls it directly. It calls `TFB_UninitGraphics` exactly once for DCQ/graphics teardown. Use only the preflight-proven supported setup: `SDL_VIDEODRIVER=dummy`, proof-hidden 320x240 window, software renderer. If unavailable, BLOCK rather than substitute fake surfaces. A fresh process establishes neutral fade/transition history. Case A enqueues a harmless real `TFB_DrawScreen_Line` command to main screen, calls real `TFB_FlushGraphicsEx(TRUE)`, proves the queue drained and observer count stayed zero. Case B resets `TFB_BBox`, neutralizes fade/transition, calls real `TFB_SwapBuffers(TFB_REDRAW_NO)`, and proves zero. Case C calls real `TFB_SwapBuffers(TFB_REDRAW_YES)` and proves one postprocess observation. Public `graphics_backend`, `TFB_BBox`, and test observer counters are controls only; neither tested function is copied. `nm -A` proves origin.

## TDD

1. Pure padded pitch/masks/BPP/null/overflow conversions.
2. Unit ABI conversion plus production-linked real surface satisfying linked `SDL_MUSTLOCK`; verify real lock/read/unlock. A linked fault seam forces `SDL_LockSurface` failure through the same production helper and proves no read/success. Also conversion-error/panic unlock.
3. Full present shell normal/active/inactive/panic: inactive automation subcall allocates/does no work; complete extern never unwinds; ABI vs active counters are correct.
4. Real skip-swap with queued command and no-redraw with invalid `TFB_BBox` do not count/complete; forced redraw counts once; single DCQ initialization is asserted.
5. Generation tests: nonzero arm, stale/duplicate/zero/future/wrap/finalization clear; only exact match can commit.
6. Capture pending through temporary write, exclusive publish, directory classification, ordered trace and commit; inject every step and reservation cancellation.
7. Metadata states logical main surface 0 and overlay/direct-video limitations; decode exact 320x240.
8. Lock-order test proves no runtime/ordered-I/O lock during graphics/SDL/present and no runtime lock during file wait/write.

Run both linked harnesses, `nm`, focused/full tests, production build, and strict gates. Worker handoff only.
