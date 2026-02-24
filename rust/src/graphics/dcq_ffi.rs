//! DCQ FFI Bridge — C-callable functions that bridge draw command enqueueing
//! to Rust's `DrawCommandQueue`.
//!
//! Global DCQ singleton using `UnsafeCell` + `unsafe impl Sync` pattern
//! (matching `ffi.rs`). FFI functions create `DrawCommand` variants and push
//! them to the queue. Flush calls `process_commands` from `dcqueue.rs`.
//!
//! @plan PLAN-20260223-GFX-FULL-PORT.P18
//! @plan PLAN-20260223-GFX-FULL-PORT.P19
//! @plan PLAN-20260223-GFX-FULL-PORT.P20
//! @requirement REQ-DCQ-010, REQ-DCQ-020, REQ-DCQ-030, REQ-DCQ-040,
//!              REQ-DCQ-050, REQ-FFI-030

use std::cell::UnsafeCell;
use std::ffi::c_int;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, RwLock};

use crate::bridge_log::rust_bridge_log_msg;
use crate::graphics::dcqueue::{
    Color, ColorMapRef, DcqConfig, DrawCommand, DrawCommandQueue, DrawMode, Extent, ImageRef,
    Point, Rect, Screen,
};
use crate::graphics::ffi::SDL_Rect;
use crate::graphics::gfx_common::ScaleMode;
use crate::graphics::render_context::RenderContext;

/// Number of screens (Main, Extra, Transition).
const TFB_GFX_NUMSCREENS: usize = 3;

// ============================================================================
// Global DCQ singleton
// ============================================================================

/// Internal state for the DCQ FFI bridge.
struct DcqState {
    /// The draw command queue.
    queue: DrawCommandQueue,
    /// Current target screen index (0 = Main, 1 = Extra, 2 = Transition).
    current_screen: c_int,
}

/// Thread-local DCQ state wrapper.
struct DcqStateCell(UnsafeCell<Option<DcqState>>);

// SAFETY: All FFI functions are called exclusively from the C graphics thread.
// The UQM C code guarantees single-threaded access (REQ-THR-010, REQ-THR-030).
unsafe impl Sync for DcqStateCell {}

static DCQ_STATE: DcqStateCell = DcqStateCell(UnsafeCell::new(None));

fn get_dcq_state() -> Option<&'static mut DcqState> {
    // SAFETY: Single-threaded access guaranteed by C caller contract.
    unsafe { (*DCQ_STATE.0.get()).as_mut() }
}

fn set_dcq_state(state: Option<DcqState>) {
    // SAFETY: Single-threaded access guaranteed by C caller contract.
    unsafe {
        *DCQ_STATE.0.get() = state;
    }
}

/// Convert a screen index to a `Screen` enum variant.
fn screen_from_index(index: c_int) -> Option<Screen> {
    match index {
        0 => Some(Screen::Main),
        1 => Some(Screen::Extra),
        2 => Some(Screen::Transition),
        _ => None,
    }
}

/// Convert a packed RGBA u32 to a `Color` struct.
///
/// Byte order matches C-side masks: R=0xFF000000, G=0x00FF0000,
/// B=0x0000FF00, A=0x000000FF.
#[inline]
fn color_from_u32(c: u32) -> Color {
    Color::new((c >> 24) as u8, (c >> 16) as u8, (c >> 8) as u8, c as u8)
}

// ============================================================================
// Lifecycle: init / uninit
// ============================================================================

/// Initialize the global DCQ.
///
/// Creates a `DrawCommandQueue` with standard configuration and a fresh
/// `RenderContext`. Returns 0 on success, -1 if already initialized.
///
/// # Safety
///
/// Must be called from the graphics thread only. Typically called from
/// `rust_gfx_init`.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P20
/// @requirement REQ-DCQ-010
// PANIC-FREE: catch_unwind wraps entire body.
#[no_mangle]
pub extern "C" fn rust_dcq_init() -> c_int {
    catch_unwind(AssertUnwindSafe(|| {
        if get_dcq_state().is_some() {
            rust_bridge_log_msg("rust_dcq_init: already initialized");
            return -1;
        }

        let render_context = Arc::new(RwLock::new(RenderContext::new()));
        let queue = DrawCommandQueue::with_config(DcqConfig::standard(), render_context);

        set_dcq_state(Some(DcqState {
            queue,
            current_screen: 0,
        }));

        rust_bridge_log_msg("rust_dcq_init: success");
        0
    }))
    .unwrap_or(-1)
}

/// Tear down the global DCQ.
///
/// Clears the queue and drops all resources. Safe to call when not
/// initialized (no-op).
///
/// # Safety
///
/// Must be called from the graphics thread only. Typically called from
/// `rust_gfx_uninit`.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P20
/// @requirement REQ-DCQ-010
// PANIC-FREE: catch_unwind wraps entire body.
#[no_mangle]
pub extern "C" fn rust_dcq_uninit() {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if let Some(state) = get_dcq_state() {
            state.queue.clear();
        }
        set_dcq_state(None);
        rust_bridge_log_msg("rust_dcq_uninit: done");
    }));
}

// ============================================================================
// Push commands
// ============================================================================

/// Push a DrawLine command onto the DCQ.
///
/// Returns 0 on success, -1 on error (not initialized, queue full).
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P20
/// @requirement REQ-DCQ-020
// PANIC-FREE: catch_unwind wraps entire body.
#[no_mangle]
pub extern "C" fn rust_dcq_push_drawline(
    x1: c_int,
    y1: c_int,
    x2: c_int,
    y2: c_int,
    color: u32,
) -> c_int {
    catch_unwind(AssertUnwindSafe(|| {
        let state = match get_dcq_state() {
            Some(s) => s,
            None => return -1,
        };
        let dest = match screen_from_index(state.current_screen) {
            Some(s) => s,
            None => return -1,
        };
        let cmd = DrawCommand::Line {
            x1,
            y1,
            x2,
            y2,
            color: color_from_u32(color),
            draw_mode: DrawMode::Normal,
            dest,
        };
        match state.queue.push(cmd) {
            Ok(()) => 0,
            Err(_) => -1,
        }
    }))
    .unwrap_or(-1)
}

/// Push a DrawRect (outline) command onto the DCQ.
///
/// Returns 0 on success, -1 on error.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P20
/// @requirement REQ-DCQ-020
// PANIC-FREE: catch_unwind wraps entire body.
#[no_mangle]
pub extern "C" fn rust_dcq_push_drawrect(
    x: c_int,
    y: c_int,
    w: c_int,
    h: c_int,
    color: u32,
) -> c_int {
    catch_unwind(AssertUnwindSafe(|| {
        let state = match get_dcq_state() {
            Some(s) => s,
            None => return -1,
        };
        let dest = match screen_from_index(state.current_screen) {
            Some(s) => s,
            None => return -1,
        };
        let cmd = DrawCommand::Rect {
            rect: Rect::new(Point::new(x, y), Extent::new(w, h)),
            color: color_from_u32(color),
            draw_mode: DrawMode::Normal,
            dest,
        };
        match state.queue.push(cmd) {
            Ok(()) => 0,
            Err(_) => -1,
        }
    }))
    .unwrap_or(-1)
}

/// Push a FilledRect command onto the DCQ.
///
/// The DCQ `Rect` variant is used for both outline and fill; the
/// `handle_command` dispatch in `dcqueue.rs` currently draws outlines.
/// For filled rectangles we push a `Rect` with `DrawMode::Blended` to
/// distinguish it from outline draws. In the future, a dedicated
/// `FilledRect` variant may be added.
///
/// Returns 0 on success, -1 on error.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P20
/// @requirement REQ-DCQ-020
// PANIC-FREE: catch_unwind wraps entire body.
#[no_mangle]
pub extern "C" fn rust_dcq_push_fillrect(
    x: c_int,
    y: c_int,
    w: c_int,
    h: c_int,
    color: u32,
) -> c_int {
    catch_unwind(AssertUnwindSafe(|| {
        let state = match get_dcq_state() {
            Some(s) => s,
            None => return -1,
        };
        let dest = match screen_from_index(state.current_screen) {
            Some(s) => s,
            None => return -1,
        };
        // Use Blended draw mode to signal fill vs outline to handler
        let cmd = DrawCommand::Rect {
            rect: Rect::new(Point::new(x, y), Extent::new(w, h)),
            color: color_from_u32(color),
            draw_mode: DrawMode::Blended,
            dest,
        };
        match state.queue.push(cmd) {
            Ok(()) => 0,
            Err(_) => -1,
        }
    }))
    .unwrap_or(-1)
}

/// Push a DrawImage command onto the DCQ.
///
/// `image_id` is the Rust-side image resource identifier (from RenderContext).
/// The image is drawn at `(x, y)` on the current screen.
///
/// Returns 0 on success, -1 on error.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P20
/// @requirement REQ-DCQ-020
// PANIC-FREE: catch_unwind wraps entire body.
#[no_mangle]
pub extern "C" fn rust_dcq_push_drawimage(image_id: u32, x: c_int, y: c_int) -> c_int {
    catch_unwind(AssertUnwindSafe(|| {
        let state = match get_dcq_state() {
            Some(s) => s,
            None => return -1,
        };
        let dest = match screen_from_index(state.current_screen) {
            Some(s) => s,
            None => return -1,
        };
        let cmd = DrawCommand::Image {
            image: ImageRef::new(image_id),
            x,
            y,
            dest,
            colormap: None,
            draw_mode: DrawMode::Normal,
            scale: 0,
            scale_mode: ScaleMode::Nearest,
        };
        match state.queue.push(cmd) {
            Ok(()) => 0,
            Err(_) => -1,
        }
    }))
    .unwrap_or(-1)
}

/// Push a Copy (screen-to-screen blit) command onto the DCQ.
///
/// Copies a rectangle from `src_screen` to the current screen at
/// `(dst_x, dst_y)`. If `src_rect` is null, copies the full source.
///
/// Returns 0 on success, -1 on error.
///
/// # Safety
///
/// `src_rect` must be a valid `SDL_Rect` pointer or null.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P20
/// @requirement REQ-DCQ-020
// PANIC-FREE: catch_unwind wraps entire body.
#[no_mangle]
pub unsafe extern "C" fn rust_dcq_push_copy(
    src_rect: *const SDL_Rect,
    src_screen: c_int,
    dst_x: c_int,
    dst_y: c_int,
) -> c_int {
    catch_unwind(AssertUnwindSafe(|| {
        let state = match get_dcq_state() {
            Some(s) => s,
            None => return -1,
        };
        let dest = match screen_from_index(state.current_screen) {
            Some(s) => s,
            None => return -1,
        };
        let src = match screen_from_index(src_screen) {
            Some(s) => s,
            None => return -1,
        };

        let rect = if src_rect.is_null() {
            Rect::new(Point::new(dst_x, dst_y), Extent::new(-1, -1))
        } else {
            let r = &*src_rect;
            Rect::new(Point::new(r.x, r.y), Extent::new(r.w, r.h))
        };

        let cmd = DrawCommand::Copy { rect, src, dest };
        match state.queue.push(cmd) {
            Ok(()) => 0,
            Err(_) => -1,
        }
    }))
    .unwrap_or(-1)
}

/// Push a CopyToImage command onto the DCQ.
///
/// Copies a rectangle from the current screen into the specified image.
///
/// Returns 0 on success, -1 on error.
///
/// # Safety
///
/// `src_rect` must be a valid `SDL_Rect` pointer or null.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P20
/// @requirement REQ-DCQ-020
// PANIC-FREE: catch_unwind wraps entire body.
#[no_mangle]
pub unsafe extern "C" fn rust_dcq_push_copytoimage(
    image_id: u32,
    src_rect: *const SDL_Rect,
) -> c_int {
    catch_unwind(AssertUnwindSafe(|| {
        let state = match get_dcq_state() {
            Some(s) => s,
            None => return -1,
        };
        let src = match screen_from_index(state.current_screen) {
            Some(s) => s,
            None => return -1,
        };

        let rect = if src_rect.is_null() {
            Rect::new(Point::new(0, 0), Extent::new(-1, -1))
        } else {
            let r = &*src_rect;
            Rect::new(Point::new(r.x, r.y), Extent::new(r.w, r.h))
        };

        let cmd = DrawCommand::CopyToImage {
            image: ImageRef::new(image_id),
            rect,
            src,
        };
        match state.queue.push(cmd) {
            Ok(()) => 0,
            Err(_) => -1,
        }
    }))
    .unwrap_or(-1)
}

/// Push a DeleteImage command onto the DCQ.
///
/// Returns 0 on success, -1 on error.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P20
/// @requirement REQ-DCQ-020
// PANIC-FREE: catch_unwind wraps entire body.
#[no_mangle]
pub extern "C" fn rust_dcq_push_deleteimage(image_id: u32) -> c_int {
    catch_unwind(AssertUnwindSafe(|| {
        let state = match get_dcq_state() {
            Some(s) => s,
            None => return -1,
        };
        let cmd = DrawCommand::DeleteImage {
            image: ImageRef::new(image_id),
        };
        match state.queue.push(cmd) {
            Ok(()) => 0,
            Err(_) => -1,
        }
    }))
    .unwrap_or(-1)
}

/// Push a SendSignal (wait-for-signal) command onto the DCQ.
///
/// Creates an `AtomicBool` signal that will be set to `true` when the
/// command is processed during flush. Returns 0 on success, -1 on error.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P20
/// @requirement REQ-DCQ-020
// PANIC-FREE: catch_unwind wraps entire body.
#[no_mangle]
pub extern "C" fn rust_dcq_push_waitsignal() -> c_int {
    catch_unwind(AssertUnwindSafe(|| {
        let state = match get_dcq_state() {
            Some(s) => s,
            None => return -1,
        };
        let signal = Arc::new(AtomicBool::new(false));
        let cmd = DrawCommand::SendSignal { signal };
        match state.queue.push(cmd) {
            Ok(()) => 0,
            Err(_) => -1,
        }
    }))
    .unwrap_or(-1)
}

/// Push a ReinitVideo command onto the DCQ.
///
/// Parameters encode driver, flags, and new dimensions for video
/// reinitialization. Returns 0 on success, -1 on error.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P20
/// @requirement REQ-DCQ-020
// PANIC-FREE: catch_unwind wraps entire body.
#[no_mangle]
pub extern "C" fn rust_dcq_push_reinitvideo(
    driver: c_int,
    flags: c_int,
    width: c_int,
    height: c_int,
) -> c_int {
    catch_unwind(AssertUnwindSafe(|| {
        let state = match get_dcq_state() {
            Some(s) => s,
            None => return -1,
        };
        let cmd = DrawCommand::ReinitVideo {
            driver,
            flags,
            width,
            height,
        };
        match state.queue.push(cmd) {
            Ok(()) => 0,
            Err(_) => -1,
        }
    }))
    .unwrap_or(-1)
}

/// Push a SetPalette (color map) command onto the DCQ.
///
/// Returns 0 on success, -1 on error.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P20
/// @requirement REQ-DCQ-020
// PANIC-FREE: catch_unwind wraps entire body.
#[no_mangle]
pub extern "C" fn rust_dcq_push_setpalette(colormap_id: u32) -> c_int {
    catch_unwind(AssertUnwindSafe(|| {
        let state = match get_dcq_state() {
            Some(s) => s,
            None => return -1,
        };
        // SetPalette doesn't have a dedicated DCQ command variant yet;
        // enqueue as a Callback that logs the request for future wiring.
        let _cmap_id = ColorMapRef::new(colormap_id);
        let cmd = DrawCommand::Callback {
            callback: |arg| {
                log::debug!("DCQ SetPalette callback: colormap_id={}", arg);
            },
            arg: colormap_id as u64,
        };
        match state.queue.push(cmd) {
            Ok(()) => 0,
            Err(_) => -1,
        }
    }))
    .unwrap_or(-1)
}

/// Push a ScissorEnable command onto the DCQ.
///
/// Returns 0 on success, -1 on error.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P20
/// @requirement REQ-DCQ-020
// PANIC-FREE: catch_unwind wraps entire body.
#[no_mangle]
pub extern "C" fn rust_dcq_push_scissor_enable(x: c_int, y: c_int, w: c_int, h: c_int) -> c_int {
    catch_unwind(AssertUnwindSafe(|| {
        let state = match get_dcq_state() {
            Some(s) => s,
            None => return -1,
        };
        let cmd = DrawCommand::ScissorEnable {
            rect: Rect::new(Point::new(x, y), Extent::new(w, h)),
        };
        match state.queue.push(cmd) {
            Ok(()) => 0,
            Err(_) => -1,
        }
    }))
    .unwrap_or(-1)
}

/// Push a ScissorDisable command onto the DCQ.
///
/// Returns 0 on success, -1 on error.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P20
/// @requirement REQ-DCQ-020
// PANIC-FREE: catch_unwind wraps entire body.
#[no_mangle]
pub extern "C" fn rust_dcq_push_scissor_disable() -> c_int {
    catch_unwind(AssertUnwindSafe(|| {
        let state = match get_dcq_state() {
            Some(s) => s,
            None => return -1,
        };
        let cmd = DrawCommand::ScissorDisable;
        match state.queue.push(cmd) {
            Ok(()) => 0,
            Err(_) => -1,
        }
    }))
    .unwrap_or(-1)
}

// ============================================================================
// Flush / batch / screen
// ============================================================================

/// Flush (process) all enqueued DCQ commands.
///
/// Processes commands in FIFO order via `DrawCommandQueue::process_commands`.
/// A no-op when the queue is batched (batch depth > 0).
///
/// Returns 0 on success, -1 on error.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P20
/// @requirement REQ-DCQ-030
// PANIC-FREE: catch_unwind wraps entire body.
#[no_mangle]
pub extern "C" fn rust_dcq_flush() -> c_int {
    catch_unwind(AssertUnwindSafe(|| {
        let state = match get_dcq_state() {
            Some(s) => s,
            None => return -1,
        };
        match state.queue.process_commands() {
            Ok(()) => 0,
            Err(_) => -1,
        }
    }))
    .unwrap_or(-1)
}

/// Enter batch mode — commands accumulate but are not visible to consumers
/// until unbatch.
///
/// Nestable: multiple calls increment the batch depth.
///
/// Returns 0 on success, -1 on error.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P20
/// @requirement REQ-DCQ-050
// PANIC-FREE: catch_unwind wraps entire body.
#[no_mangle]
pub extern "C" fn rust_dcq_batch() -> c_int {
    catch_unwind(AssertUnwindSafe(|| {
        let state = match get_dcq_state() {
            Some(s) => s,
            None => return -1,
        };
        // batch() returns a BatchGuard; we intentionally forget it since
        // unbatch is managed explicitly via rust_dcq_unbatch.
        let guard = state.queue.batch();
        guard.cancel();
        0
    }))
    .unwrap_or(-1)
}

/// Exit batch mode — decrements batch depth. When depth reaches 0,
/// accumulated commands become visible to the consumer.
///
/// Returns 0 on success, -1 on error.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P20
/// @requirement REQ-DCQ-050
// PANIC-FREE: catch_unwind wraps entire body.
#[no_mangle]
pub extern "C" fn rust_dcq_unbatch() -> c_int {
    catch_unwind(AssertUnwindSafe(|| {
        let state = match get_dcq_state() {
            Some(s) => s,
            None => return -1,
        };
        state.queue.unbatch();
        0
    }))
    .unwrap_or(-1)
}

/// Set the current target screen for subsequent draw commands.
///
/// Valid indices: 0 (Main), 1 (Extra), 2 (Transition).
/// Returns 0 on success, -1 on invalid index.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P20
/// @requirement REQ-DCQ-040
// PANIC-FREE: catch_unwind wraps entire body.
#[no_mangle]
pub extern "C" fn rust_dcq_set_screen(index: c_int) -> c_int {
    catch_unwind(AssertUnwindSafe(|| {
        let state = match get_dcq_state() {
            Some(s) => s,
            None => return -1,
        };
        if index < 0 || index >= TFB_GFX_NUMSCREENS as c_int {
            return -1;
        }
        state.current_screen = index;
        0
    }))
    .unwrap_or(-1)
}

/// Get the current target screen index.
///
/// Returns the screen index (0-2), or -1 if DCQ is not initialized.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P20
/// @requirement REQ-DCQ-040
// PANIC-FREE: catch_unwind wraps entire body.
#[no_mangle]
pub extern "C" fn rust_dcq_get_screen() -> c_int {
    catch_unwind(AssertUnwindSafe(|| {
        let state = match get_dcq_state() {
            Some(s) => s,
            None => return -1,
        };
        state.current_screen
    }))
    .unwrap_or(-1)
}

/// Get the number of commands currently in the DCQ.
///
/// Returns the queue length, or -1 if not initialized.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P20
/// @requirement REQ-DCQ-010
// PANIC-FREE: catch_unwind wraps entire body.
#[no_mangle]
pub extern "C" fn rust_dcq_len() -> c_int {
    catch_unwind(AssertUnwindSafe(|| {
        let state = match get_dcq_state() {
            Some(s) => s,
            None => return -1,
        };
        state.queue.len() as c_int
    }))
    .unwrap_or(-1)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::sync::atomic::Ordering;

    /// Reset the global DCQ state for test isolation.
    fn reset_dcq() {
        set_dcq_state(None);
    }

    /// Initialize DCQ and return success status. Resets first for isolation.
    fn init_dcq() -> c_int {
        reset_dcq();
        rust_dcq_init()
    }

    // -- REQ-DCQ-010: Lifecycle tests ----------------------------------------

    /// @requirement REQ-DCQ-010
    #[serial]
    #[test]
    fn test_dcq_init_success() {
        let rc = init_dcq();
        assert_eq!(rc, 0);
        reset_dcq();
    }

    /// @requirement REQ-DCQ-010
    #[serial]
    #[test]
    fn test_dcq_init_double_init_fails() {
        reset_dcq();
        assert_eq!(rust_dcq_init(), 0);
        assert_eq!(rust_dcq_init(), -1);
        reset_dcq();
    }

    /// @requirement REQ-DCQ-010
    #[serial]
    #[test]
    fn test_dcq_uninit_cleans_up() {
        assert_eq!(init_dcq(), 0);
        rust_dcq_uninit();
        assert!(get_dcq_state().is_none());
    }

    /// @requirement REQ-DCQ-010
    #[serial]
    #[test]
    fn test_dcq_uninit_when_not_initialized() {
        reset_dcq();
        rust_dcq_uninit();
        assert!(get_dcq_state().is_none());
    }

    // -- REQ-DCQ-020: Push command tests -------------------------------------

    /// @requirement REQ-DCQ-020
    #[serial]
    #[test]
    fn test_dcq_push_drawline() {
        assert_eq!(init_dcq(), 0);
        let white = 0xFFFFFFFF_u32;
        let rc = rust_dcq_push_drawline(0, 0, 10, 10, white);
        assert_eq!(rc, 0);
        assert_eq!(rust_dcq_len(), 1);
        reset_dcq();
    }

    /// @requirement REQ-DCQ-020
    #[serial]
    #[test]
    fn test_dcq_push_drawrect() {
        assert_eq!(init_dcq(), 0);
        let red = 0xFF0000FF_u32;
        let rc = rust_dcq_push_drawrect(5, 5, 20, 15, red);
        assert_eq!(rc, 0);
        assert_eq!(rust_dcq_len(), 1);
        reset_dcq();
    }

    /// @requirement REQ-DCQ-020
    #[serial]
    #[test]
    fn test_dcq_push_fillrect() {
        assert_eq!(init_dcq(), 0);
        let blue = 0x0000FFFF_u32;
        let rc = rust_dcq_push_fillrect(0, 0, 100, 50, blue);
        assert_eq!(rc, 0);
        assert_eq!(rust_dcq_len(), 1);
        reset_dcq();
    }

    /// @requirement REQ-DCQ-020
    #[serial]
    #[test]
    fn test_dcq_push_drawimage() {
        assert_eq!(init_dcq(), 0);
        let rc = rust_dcq_push_drawimage(42, 10, 20);
        assert_eq!(rc, 0);
        assert_eq!(rust_dcq_len(), 1);
        reset_dcq();
    }

    /// @requirement REQ-DCQ-020
    #[serial]
    #[test]
    fn test_dcq_push_deleteimage() {
        assert_eq!(init_dcq(), 0);
        let rc = rust_dcq_push_deleteimage(7);
        assert_eq!(rc, 0);
        assert_eq!(rust_dcq_len(), 1);
        reset_dcq();
    }

    /// @requirement REQ-DCQ-020
    #[serial]
    #[test]
    fn test_dcq_push_waitsignal() {
        assert_eq!(init_dcq(), 0);
        let rc = rust_dcq_push_waitsignal();
        assert_eq!(rc, 0);
        assert_eq!(rust_dcq_len(), 1);
        reset_dcq();
    }

    /// @requirement REQ-DCQ-020
    #[serial]
    #[test]
    fn test_dcq_push_reinitvideo() {
        assert_eq!(init_dcq(), 0);
        let rc = rust_dcq_push_reinitvideo(0, 0x01, 640, 480);
        assert_eq!(rc, 0);
        assert_eq!(rust_dcq_len(), 1);
        reset_dcq();
    }

    /// @requirement REQ-DCQ-020
    #[serial]
    #[test]
    fn test_dcq_push_multiple() {
        assert_eq!(init_dcq(), 0);
        let c = 0xFFFFFFFF_u32;
        assert_eq!(rust_dcq_push_drawline(0, 0, 1, 1, c), 0);
        assert_eq!(rust_dcq_push_drawrect(0, 0, 5, 5, c), 0);
        assert_eq!(rust_dcq_push_fillrect(0, 0, 5, 5, c), 0);
        assert_eq!(rust_dcq_push_drawimage(1, 0, 0), 0);
        assert_eq!(rust_dcq_push_deleteimage(1), 0);
        assert_eq!(rust_dcq_len(), 5);
        reset_dcq();
    }

    /// @requirement REQ-DCQ-020
    #[serial]
    #[test]
    fn test_dcq_push_copy_null_rect() {
        assert_eq!(init_dcq(), 0);
        let rc = unsafe { rust_dcq_push_copy(std::ptr::null(), 0, 10, 20) };
        assert_eq!(rc, 0);
        assert_eq!(rust_dcq_len(), 1);
        reset_dcq();
    }

    /// @requirement REQ-DCQ-020
    #[serial]
    #[test]
    fn test_dcq_push_copy_with_rect() {
        assert_eq!(init_dcq(), 0);
        let rect = SDL_Rect {
            x: 0,
            y: 0,
            w: 50,
            h: 50,
        };
        let rc = unsafe { rust_dcq_push_copy(&rect as *const SDL_Rect, 0, 100, 100) };
        assert_eq!(rc, 0);
        assert_eq!(rust_dcq_len(), 1);
        reset_dcq();
    }

    /// @requirement REQ-DCQ-020
    #[serial]
    #[test]
    fn test_dcq_push_copytoimage() {
        assert_eq!(init_dcq(), 0);
        let rect = SDL_Rect {
            x: 0,
            y: 0,
            w: 32,
            h: 32,
        };
        let rc = unsafe { rust_dcq_push_copytoimage(99, &rect as *const SDL_Rect) };
        assert_eq!(rc, 0);
        assert_eq!(rust_dcq_len(), 1);
        reset_dcq();
    }

    /// @requirement REQ-DCQ-020
    #[serial]
    #[test]
    fn test_dcq_push_setpalette() {
        assert_eq!(init_dcq(), 0);
        let rc = rust_dcq_push_setpalette(5);
        assert_eq!(rc, 0);
        assert_eq!(rust_dcq_len(), 1);
        reset_dcq();
    }

    /// @requirement REQ-DCQ-020
    #[serial]
    #[test]
    fn test_dcq_push_scissor_enable() {
        assert_eq!(init_dcq(), 0);
        let rc = rust_dcq_push_scissor_enable(10, 20, 100, 80);
        assert_eq!(rc, 0);
        assert_eq!(rust_dcq_len(), 1);
        reset_dcq();
    }

    /// @requirement REQ-DCQ-020
    #[serial]
    #[test]
    fn test_dcq_push_scissor_disable() {
        assert_eq!(init_dcq(), 0);
        let rc = rust_dcq_push_scissor_disable();
        assert_eq!(rc, 0);
        assert_eq!(rust_dcq_len(), 1);
        reset_dcq();
    }

    // -- REQ-DCQ-030: Flush tests --------------------------------------------

    /// @requirement REQ-DCQ-030
    #[serial]
    #[test]
    fn test_dcq_flush_empty() {
        assert_eq!(init_dcq(), 0);
        let rc = rust_dcq_flush();
        assert_eq!(rc, 0);
        reset_dcq();
    }

    /// @requirement REQ-DCQ-030
    #[serial]
    #[test]
    fn test_dcq_flush_processes_all() {
        assert_eq!(init_dcq(), 0);
        let c = 0xFFFFFFFF_u32;
        assert_eq!(rust_dcq_push_drawline(0, 0, 1, 1, c), 0);
        assert_eq!(rust_dcq_push_drawrect(0, 0, 5, 5, c), 0);
        assert_eq!(rust_dcq_push_deleteimage(1), 0);
        assert_eq!(rust_dcq_len(), 3);
        assert_eq!(rust_dcq_flush(), 0);
        assert_eq!(rust_dcq_len(), 0);
        reset_dcq();
    }

    /// @requirement REQ-DCQ-030
    #[serial]
    #[test]
    fn test_dcq_flush_signal_processed() {
        assert_eq!(init_dcq(), 0);
        let signal = Arc::new(AtomicBool::new(false));
        let state = get_dcq_state().expect("dcq initialized");
        let cmd = DrawCommand::SendSignal {
            signal: Arc::clone(&signal),
        };
        assert!(state.queue.push(cmd).is_ok());
        assert_eq!(rust_dcq_flush(), 0);
        assert!(signal.load(Ordering::Acquire));
        reset_dcq();
    }

    /// @requirement REQ-DCQ-030
    #[serial]
    #[test]
    fn test_dcq_flush_not_initialized() {
        reset_dcq();
        assert_eq!(rust_dcq_flush(), -1);
    }

    // -- REQ-DCQ-040: Screen binding tests -----------------------------------

    /// @requirement REQ-DCQ-040
    #[serial]
    #[test]
    fn test_dcq_set_screen_valid() {
        assert_eq!(init_dcq(), 0);
        assert_eq!(rust_dcq_set_screen(0), 0);
        assert_eq!(rust_dcq_get_screen(), 0);
        reset_dcq();
    }

    /// @requirement REQ-DCQ-040
    #[serial]
    #[test]
    fn test_dcq_set_screen_invalid() {
        assert_eq!(init_dcq(), 0);
        assert_eq!(rust_dcq_set_screen(99), -1);
        assert_eq!(rust_dcq_set_screen(-1), -1);
        reset_dcq();
    }

    /// @requirement REQ-DCQ-040
    #[serial]
    #[test]
    fn test_dcq_set_screen_roundtrip() {
        assert_eq!(init_dcq(), 0);
        assert_eq!(rust_dcq_set_screen(2), 0);
        assert_eq!(rust_dcq_get_screen(), 2);
        assert_eq!(rust_dcq_set_screen(1), 0);
        assert_eq!(rust_dcq_get_screen(), 1);
        reset_dcq();
    }

    /// @requirement REQ-DCQ-040
    #[serial]
    #[test]
    fn test_dcq_get_screen_not_initialized() {
        reset_dcq();
        assert_eq!(rust_dcq_get_screen(), -1);
    }

    // -- REQ-DCQ-050: Batch mode tests ---------------------------------------

    /// @requirement REQ-DCQ-050
    #[serial]
    #[test]
    fn test_dcq_batch_mode() {
        assert_eq!(init_dcq(), 0);
        assert_eq!(rust_dcq_batch(), 0);
        let c = 0xFFFFFFFF_u32;
        assert_eq!(rust_dcq_push_drawline(0, 0, 1, 1, c), 0);
        assert_eq!(rust_dcq_push_drawline(2, 2, 3, 3, c), 0);
        // Commands pushed but not visible to consumer during batch
        let state = get_dcq_state().expect("dcq initialized");
        assert_eq!(state.queue.len(), 0);
        assert_eq!(state.queue.full_size(), 2);

        assert_eq!(rust_dcq_unbatch(), 0);
        // After unbatch, commands become visible
        let state = get_dcq_state().expect("dcq initialized");
        assert_eq!(state.queue.len(), 2);
        reset_dcq();
    }

    /// @requirement REQ-DCQ-050
    #[serial]
    #[test]
    fn test_dcq_nested_batch() {
        assert_eq!(init_dcq(), 0);
        assert_eq!(rust_dcq_batch(), 0);
        assert_eq!(rust_dcq_batch(), 0);
        let c = 0xFFFFFFFF_u32;
        assert_eq!(rust_dcq_push_drawline(0, 0, 1, 1, c), 0);

        assert_eq!(rust_dcq_unbatch(), 0);
        // Still batched (depth was 2, now 1)
        let state = get_dcq_state().expect("dcq initialized");
        assert_eq!(state.queue.len(), 0);

        assert_eq!(rust_dcq_unbatch(), 0);
        // Now fully unbatched, commands visible
        let state = get_dcq_state().expect("dcq initialized");
        assert_eq!(state.queue.len(), 1);
        reset_dcq();
    }

    /// @requirement REQ-DCQ-050
    #[serial]
    #[test]
    fn test_dcq_unbatch_without_batch() {
        assert_eq!(init_dcq(), 0);
        assert_eq!(rust_dcq_unbatch(), 0);
        reset_dcq();
    }

    // -- REQ-FFI-030: Panic safety tests -------------------------------------

    /// @requirement REQ-FFI-030
    #[serial]
    #[test]
    fn test_dcq_push_not_initialized() {
        reset_dcq();
        assert_eq!(rust_dcq_push_drawline(0, 0, 1, 1, 0xFFFFFFFF), -1);
        assert_eq!(rust_dcq_push_drawrect(0, 0, 5, 5, 0xFFFFFFFF), -1);
        assert_eq!(rust_dcq_push_fillrect(0, 0, 5, 5, 0xFFFFFFFF), -1);
        assert_eq!(rust_dcq_push_drawimage(1, 0, 0), -1);
        assert_eq!(rust_dcq_push_deleteimage(1), -1);
        assert_eq!(rust_dcq_push_waitsignal(), -1);
        assert_eq!(unsafe { rust_dcq_push_copy(std::ptr::null(), 0, 0, 0) }, -1);
    }

    /// @requirement REQ-FFI-030
    #[serial]
    #[test]
    fn test_dcq_batch_not_initialized() {
        reset_dcq();
        assert_eq!(rust_dcq_batch(), -1);
        assert_eq!(rust_dcq_unbatch(), -1);
    }

    /// @requirement REQ-FFI-030
    #[serial]
    #[test]
    fn test_dcq_set_screen_not_initialized() {
        reset_dcq();
        assert_eq!(rust_dcq_set_screen(0), -1);
    }

    /// @requirement REQ-DCQ-010
    #[serial]
    #[test]
    fn test_dcq_len_not_initialized() {
        reset_dcq();
        assert_eq!(rust_dcq_len(), -1);
    }

    // -- Color conversion test -----------------------------------------------

    #[test]
    fn test_color_from_u32() {
        let c = color_from_u32(0xFF8040C0);
        assert_eq!(c.r, 0xFF);
        assert_eq!(c.g, 0x80);
        assert_eq!(c.b, 0x40);
        assert_eq!(c.a, 0xC0);
    }

    #[test]
    fn test_color_from_u32_black() {
        let c = color_from_u32(0x00000000);
        assert_eq!(c.r, 0);
        assert_eq!(c.g, 0);
        assert_eq!(c.b, 0);
        assert_eq!(c.a, 0);
    }

    #[test]
    fn test_color_from_u32_white() {
        let c = color_from_u32(0xFFFFFFFF);
        assert_eq!(c.r, 0xFF);
        assert_eq!(c.g, 0xFF);
        assert_eq!(c.b, 0xFF);
        assert_eq!(c.a, 0xFF);
    }

    // -- Screen conversion test ----------------------------------------------

    #[test]
    fn test_screen_from_index() {
        assert_eq!(screen_from_index(0), Some(Screen::Main));
        assert_eq!(screen_from_index(1), Some(Screen::Extra));
        assert_eq!(screen_from_index(2), Some(Screen::Transition));
        assert_eq!(screen_from_index(3), None);
        assert_eq!(screen_from_index(-1), None);
    }
}
