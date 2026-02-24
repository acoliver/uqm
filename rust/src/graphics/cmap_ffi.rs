//! Colormap FFI Bridge — C-callable functions that bridge colormap/fade/xform
//! operations to Rust's `ColorMapManager` in `cmap.rs`.
//!
//! Global `ColorMapManager` singleton using `UnsafeCell` + `unsafe impl Sync`
//! pattern (matching `dcq_ffi.rs` and `ffi.rs`). All FFI functions are wrapped
//! with `catch_unwind` for panic safety.
//!
//! # Exported Functions
//!
//! | C function          | Rust export                  | Purpose                    |
//! |---------------------|------------------------------|----------------------------|
//! | `init_colormap`     | `rust_cmap_init`             | Initialize colormap system |
//! | `uninit_colormap`   | `rust_cmap_uninit`           | Tear down colormap system  |
//! | `SetColorMap`       | `rust_cmap_set`              | Set colormap palette data  |
//! | `GetColorMapAddress`| `rust_cmap_get`              | Get colormap palette data  |
//! | `FadeScreen`        | `rust_cmap_fade_screen`      | Initiate a fade            |
//! | `GetFadeAmount`     | `rust_cmap_get_fade_amount`  | Query current fade level   |
//! | `XFormColorMap_step` | `rust_cmap_xform_step`      | Step active xforms         |
//! | `TFB_SetColorMap`   | `rust_cmap_set_palette`      | Set palette from raw bytes |
//! | `TFB_ColorMapFromIndex` | `rust_cmap_from_index`   | Get colormap by index      |
//! | `FlushColorXForms`  | `rust_cmap_flush_xforms`     | Finish all active xforms   |
//!
//! @plan PLAN-20260223-GFX-FULL-PORT.P21
//! @requirement REQ-CMAP-010, REQ-CMAP-020, REQ-CMAP-030, REQ-FFI-030

use std::cell::UnsafeCell;
use std::ffi::c_int;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;

use crate::bridge_log::rust_bridge_log_msg;
use crate::graphics::cmap::{
    ColorMapManager, FadeType, FADE_NORMAL_INTENSITY, NUMBER_OF_PLUTVALS, PLUTVAL_BYTE_SIZE,
};
#[cfg(test)]
use crate::graphics::cmap::{FADE_FULL_INTENSITY, FADE_NO_INTENSITY, MAX_COLORMAPS};

// ============================================================================
// Global ColorMapManager singleton
// ============================================================================

/// Internal state for the colormap FFI bridge.
struct CmapState {
    manager: ColorMapManager,
}

/// Thread-local colormap state wrapper.
struct CmapStateCell(UnsafeCell<Option<CmapState>>);

// SAFETY: All FFI functions are called exclusively from the C graphics thread.
// The UQM C code guarantees single-threaded access (REQ-THR-010, REQ-THR-030).
unsafe impl Sync for CmapStateCell {}

static CMAP_STATE: CmapStateCell = CmapStateCell(UnsafeCell::new(None));

fn get_cmap_state() -> Option<&'static mut CmapState> {
    // SAFETY: Single-threaded access guaranteed by C caller contract.
    unsafe { (*CMAP_STATE.0.get()).as_mut() }
}

fn set_cmap_state(state: Option<CmapState>) {
    // SAFETY: Single-threaded access guaranteed by C caller contract.
    unsafe {
        *CMAP_STATE.0.get() = state;
    }
}

/// Map a C fade direction integer to a `FadeType` enum variant.
///
/// Convention matches C `cmap.c`:
///   0 → FadeToBlack (darken)
///   1 → FadeToColor (normal intensity)
///   2 → FadeToWhite (brighten)
fn fade_type_from_direction(direction: c_int) -> Option<FadeType> {
    match direction {
        0 => Some(FadeType::FadeToBlack),
        1 => Some(FadeType::FadeToColor),
        2 => Some(FadeType::FadeToWhite),
        _ => None,
    }
}

// ============================================================================
// Lifecycle: init / uninit
// ============================================================================

/// Initialize the global colormap manager.
///
/// Creates a `ColorMapManager` singleton. Returns 0 on success, -1 if
/// already initialized.
///
/// # Safety
///
/// Must be called from the graphics thread only.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P21
/// @requirement REQ-CMAP-010
// PANIC-FREE: catch_unwind wraps entire body.
#[no_mangle]
pub extern "C" fn rust_cmap_init() -> c_int {
    catch_unwind(AssertUnwindSafe(|| {
        if get_cmap_state().is_some() {
            rust_bridge_log_msg("rust_cmap_init: already initialized");
            return -1;
        }

        let mut manager = ColorMapManager::new();
        manager.init();

        set_cmap_state(Some(CmapState { manager }));

        rust_bridge_log_msg("rust_cmap_init: success");
        0
    }))
    .unwrap_or(-1)
}

/// Tear down the global colormap manager.
///
/// Calls `uninit()` on the manager and drops all resources. Safe to call
/// when not initialized (no-op).
///
/// # Safety
///
/// Must be called from the graphics thread only.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P21
/// @requirement REQ-CMAP-010
// PANIC-FREE: catch_unwind wraps entire body.
#[no_mangle]
pub extern "C" fn rust_cmap_uninit() {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if let Some(state) = get_cmap_state() {
            state.manager.uninit();
        }
        set_cmap_state(None);
        rust_bridge_log_msg("rust_cmap_uninit: done");
    }));
}

// ============================================================================
// Colormap set / get
// ============================================================================

/// Set colormap palette data for a range of maps starting at `index`.
///
/// `data` points to raw RGB triplet data (3 bytes per color, 256 colors per
/// map). `len` is the total byte count. The number of colormaps written is
/// `len / (256 * 3)`. If `len` is not evenly divisible, the remainder is
/// ignored.
///
/// Returns 0 on success, -1 on error (not initialized, invalid index,
/// size mismatch).
///
/// # Safety
///
/// `data` must point to at least `len` bytes of valid memory, or be null
/// (returns -1).
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P21
/// @requirement REQ-CMAP-010
// PANIC-FREE: catch_unwind + null check.
#[no_mangle]
pub unsafe extern "C" fn rust_cmap_set(index: c_int, data: *const u8, len: c_int) -> c_int {
    catch_unwind(AssertUnwindSafe(|| {
        let state = match get_cmap_state() {
            Some(s) => s,
            None => {
                rust_bridge_log_msg("rust_cmap_set: not initialized");
                return -1;
            }
        };

        if data.is_null() || len <= 0 {
            rust_bridge_log_msg("rust_cmap_set: null data or invalid len");
            return -1;
        }

        let bytes_per_map = NUMBER_OF_PLUTVALS * PLUTVAL_BYTE_SIZE;
        let total_len = len as usize;
        let num_maps = total_len / bytes_per_map;

        if num_maps == 0 {
            rust_bridge_log_msg("rust_cmap_set: data too small for one colormap");
            return -1;
        }

        let usable_len = num_maps * bytes_per_map;
        // SAFETY: caller guarantees data points to at least len bytes.
        let slice = std::slice::from_raw_parts(data, usable_len);

        let end_index = index + (num_maps as c_int) - 1;
        match state.manager.set_colors(index, end_index, slice) {
            Ok(()) => 0,
            Err(e) => {
                rust_bridge_log_msg(&format!("rust_cmap_set: {}", e));
                -1
            }
        }
    }))
    .unwrap_or(-1)
}

/// Get colormap palette data for a given index.
///
/// Returns a pointer to a static buffer containing 256×3 = 768 bytes of
/// RGB triplet data for the requested colormap. The buffer is valid until
/// the next call to `rust_cmap_get`. Returns null if not initialized or
/// if the index has no colormap.
///
/// # Safety
///
/// The returned pointer is only valid until the next call to `rust_cmap_get`
/// or `rust_cmap_uninit`. The caller must not free it.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P21
/// @requirement REQ-CMAP-010
// PANIC-FREE: catch_unwind + null checks.
#[no_mangle]
pub extern "C" fn rust_cmap_get(index: c_int) -> *const u8 {
    /// Static buffer for returning colormap data to C. Valid until next call.
    static GET_BUFFER: CmapGetBuffer = CmapGetBuffer(UnsafeCell::new(
        [0u8; NUMBER_OF_PLUTVALS * PLUTVAL_BYTE_SIZE],
    ));

    struct CmapGetBuffer(UnsafeCell<[u8; NUMBER_OF_PLUTVALS * PLUTVAL_BYTE_SIZE]>);
    // SAFETY: single-threaded access from graphics thread.
    unsafe impl Sync for CmapGetBuffer {}

    catch_unwind(AssertUnwindSafe(|| {
        let state = match get_cmap_state() {
            Some(s) => s,
            None => return ptr::null(),
        };

        let cmap = match state.manager.get_colormap(index) {
            Some(c) => c,
            None => return ptr::null(),
        };

        let colors = cmap.get_colors();
        // SAFETY: single-threaded access, buffer is static.
        let buf = unsafe { &mut *GET_BUFFER.0.get() };
        for (i, color) in colors.iter().enumerate() {
            let offset = i * PLUTVAL_BYTE_SIZE;
            if offset + PLUTVAL_BYTE_SIZE <= buf.len() {
                buf[offset] = color.r;
                buf[offset + 1] = color.g;
                buf[offset + 2] = color.b;
            }
        }

        // Return the ref so refcount is balanced
        state.manager.return_colormap(&cmap);

        buf.as_ptr()
    }))
    .unwrap_or(ptr::null())
}

/// Get a colormap by index and return an opaque handle.
///
/// Increments the refcount of the colormap. The caller should eventually
/// release it (not currently exposed; the manager owns the lifecycle).
///
/// Returns the colormap index on success (for C to use as a handle),
/// or -1 if not found.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P21
/// @requirement REQ-CMAP-010
// PANIC-FREE: catch_unwind wraps entire body.
#[no_mangle]
pub extern "C" fn rust_cmap_from_index(index: c_int) -> c_int {
    catch_unwind(AssertUnwindSafe(|| {
        let state = match get_cmap_state() {
            Some(s) => s,
            None => return -1,
        };

        match state.manager.get_colormap(index) {
            Some(cmap) => {
                let idx = cmap.index() as c_int;
                state.manager.return_colormap(&cmap);
                idx
            }
            None => -1,
        }
    }))
    .unwrap_or(-1)
}

// ============================================================================
// Fade operations
// ============================================================================

/// Initiate a screen fade.
///
/// `direction`: 0 = fade to black, 1 = fade to normal, 2 = fade to white.
/// `steps`: number of milliseconds for the fade duration.
///
/// Returns 0 on success, -1 on error.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P21
/// @requirement REQ-CMAP-020
// PANIC-FREE: catch_unwind wraps entire body.
#[no_mangle]
pub extern "C" fn rust_cmap_fade_screen(direction: c_int, steps: c_int) -> c_int {
    catch_unwind(AssertUnwindSafe(|| {
        let state = match get_cmap_state() {
            Some(s) => s,
            None => {
                rust_bridge_log_msg("rust_cmap_fade_screen: not initialized");
                return -1;
            }
        };

        let fade_type = match fade_type_from_direction(direction) {
            Some(ft) => ft,
            None => {
                rust_bridge_log_msg("rust_cmap_fade_screen: invalid direction");
                return -1;
            }
        };

        let duration_ms = if steps > 0 { steps as u64 } else { 0 };
        state.manager.fade_screen(fade_type, duration_ms);
        0
    }))
    .unwrap_or(-1)
}

/// Query the current fade amount.
///
/// Returns the current fade intensity (0 = black, 255 = normal, 510 = white),
/// or `FADE_NORMAL_INTENSITY` (255) if not initialized.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P21
/// @requirement REQ-CMAP-020
// PANIC-FREE: catch_unwind wraps entire body.
#[no_mangle]
pub extern "C" fn rust_cmap_get_fade_amount() -> c_int {
    catch_unwind(AssertUnwindSafe(|| {
        let state = match get_cmap_state() {
            Some(s) => s,
            None => return FADE_NORMAL_INTENSITY,
        };
        state.manager.get_fade_amount()
    }))
    .unwrap_or(FADE_NORMAL_INTENSITY)
}

// ============================================================================
// Color transform operations
// ============================================================================

/// Step all active colormap transformations.
///
/// Advances interpolation for all active xforms. Returns 1 if any xforms
/// are still active, 0 if all are complete or none were active, -1 on error.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P21
/// @requirement REQ-CMAP-020
// PANIC-FREE: catch_unwind wraps entire body.
#[no_mangle]
pub extern "C" fn rust_cmap_xform_step() -> c_int {
    catch_unwind(AssertUnwindSafe(|| {
        let state = match get_cmap_state() {
            Some(s) => s,
            None => return -1,
        };
        if state.manager.step_transformations() {
            1
        } else {
            0
        }
    }))
    .unwrap_or(-1)
}

/// Flush all active color transforms and fades.
///
/// Immediately finishes any in-progress fades and xforms, jumping to their
/// target values. Returns 0 on success, -1 if not initialized.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P21
/// @requirement REQ-CMAP-020
// PANIC-FREE: catch_unwind wraps entire body.
#[no_mangle]
pub extern "C" fn rust_cmap_flush_xforms() -> c_int {
    catch_unwind(AssertUnwindSafe(|| {
        let state = match get_cmap_state() {
            Some(s) => s,
            None => return -1,
        };
        state.manager.flush_color_xforms();
        0
    }))
    .unwrap_or(-1)
}

// ============================================================================
// Palette operations
// ============================================================================

/// Set a palette from raw RGB byte data.
///
/// `palette_data` points to `num_colors * 3` bytes of packed RGB triplets.
/// This sets the palette on colormap index 0 (the default/active palette).
///
/// Returns 0 on success, -1 on error.
///
/// # Safety
///
/// `palette_data` must point to at least `num_colors * 3` bytes.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P21
/// @requirement REQ-CMAP-030
// PANIC-FREE: catch_unwind + null check.
#[no_mangle]
pub unsafe extern "C" fn rust_cmap_set_palette(
    palette_data: *const u8,
    num_colors: c_int,
) -> c_int {
    catch_unwind(AssertUnwindSafe(|| {
        let state = match get_cmap_state() {
            Some(s) => s,
            None => {
                rust_bridge_log_msg("rust_cmap_set_palette: not initialized");
                return -1;
            }
        };

        if palette_data.is_null() || num_colors <= 0 {
            rust_bridge_log_msg("rust_cmap_set_palette: null data or invalid count");
            return -1;
        }

        let color_count = (num_colors as usize).min(NUMBER_OF_PLUTVALS);
        let byte_count = color_count * PLUTVAL_BYTE_SIZE;

        // Build a full-size buffer padded with zeros for any missing colors.
        let full_size = NUMBER_OF_PLUTVALS * PLUTVAL_BYTE_SIZE;
        let mut buf = vec![0u8; full_size];

        // SAFETY: caller guarantees palette_data points to at least byte_count bytes.
        let src = std::slice::from_raw_parts(palette_data, byte_count);
        buf[..byte_count].copy_from_slice(src);

        match state.manager.set_colors(0, 0, &buf) {
            Ok(()) => 0,
            Err(e) => {
                rust_bridge_log_msg(&format!("rust_cmap_set_palette: {}", e));
                -1
            }
        }
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

    /// Reset the global colormap state for test isolation.
    fn reset_cmap() {
        set_cmap_state(None);
    }

    /// Initialize colormap and return success status. Resets first.
    fn init_cmap() -> c_int {
        reset_cmap();
        rust_cmap_init()
    }

    // -- REQ-CMAP-010: Lifecycle tests ---------------------------------------

    /// @requirement REQ-CMAP-010
    #[serial]
    #[test]
    fn test_cmap_init_success() {
        let rc = init_cmap();
        assert_eq!(rc, 0);
        reset_cmap();
    }

    /// @requirement REQ-CMAP-010
    #[serial]
    #[test]
    fn test_cmap_init_double_init_fails() {
        reset_cmap();
        assert_eq!(rust_cmap_init(), 0);
        assert_eq!(rust_cmap_init(), -1);
        reset_cmap();
    }

    /// @requirement REQ-CMAP-010
    #[serial]
    #[test]
    fn test_cmap_uninit_cleans_up() {
        assert_eq!(init_cmap(), 0);
        rust_cmap_uninit();
        assert!(get_cmap_state().is_none());
    }

    /// @requirement REQ-CMAP-010
    #[serial]
    #[test]
    fn test_cmap_uninit_when_not_initialized() {
        reset_cmap();
        rust_cmap_uninit();
        assert!(get_cmap_state().is_none());
    }

    // -- REQ-CMAP-010: Set / Get colormap tests ------------------------------

    /// @requirement REQ-CMAP-010
    #[serial]
    #[test]
    fn test_cmap_set_basic() {
        assert_eq!(init_cmap(), 0);
        let data = vec![0u8; NUMBER_OF_PLUTVALS * PLUTVAL_BYTE_SIZE];
        let rc = unsafe { rust_cmap_set(0, data.as_ptr(), data.len() as c_int) };
        assert_eq!(rc, 0);
        reset_cmap();
    }

    /// @requirement REQ-CMAP-010
    #[serial]
    #[test]
    fn test_cmap_set_multiple_maps() {
        assert_eq!(init_cmap(), 0);
        let map_size = NUMBER_OF_PLUTVALS * PLUTVAL_BYTE_SIZE;
        let data = vec![0u8; map_size * 3];
        let rc = unsafe { rust_cmap_set(0, data.as_ptr(), data.len() as c_int) };
        assert_eq!(rc, 0);
        reset_cmap();
    }

    /// @requirement REQ-CMAP-010
    #[serial]
    #[test]
    fn test_cmap_set_null_data() {
        assert_eq!(init_cmap(), 0);
        let rc = unsafe { rust_cmap_set(0, ptr::null(), 768) };
        assert_eq!(rc, -1);
        reset_cmap();
    }

    /// @requirement REQ-CMAP-010
    #[serial]
    #[test]
    fn test_cmap_set_zero_len() {
        assert_eq!(init_cmap(), 0);
        let data = [0u8; 1];
        let rc = unsafe { rust_cmap_set(0, data.as_ptr(), 0) };
        assert_eq!(rc, -1);
        reset_cmap();
    }

    /// @requirement REQ-CMAP-010
    #[serial]
    #[test]
    fn test_cmap_set_not_initialized() {
        reset_cmap();
        let data = vec![0u8; NUMBER_OF_PLUTVALS * PLUTVAL_BYTE_SIZE];
        let rc = unsafe { rust_cmap_set(0, data.as_ptr(), data.len() as c_int) };
        assert_eq!(rc, -1);
    }

    /// @requirement REQ-CMAP-010
    #[serial]
    #[test]
    fn test_cmap_get_roundtrip() {
        assert_eq!(init_cmap(), 0);
        let map_size = NUMBER_OF_PLUTVALS * PLUTVAL_BYTE_SIZE;
        let mut data = vec![0u8; map_size];
        // Set a known color at index 0: R=0xAA, G=0xBB, B=0xCC
        data[0] = 0xAA;
        data[1] = 0xBB;
        data[2] = 0xCC;
        let rc = unsafe { rust_cmap_set(0, data.as_ptr(), data.len() as c_int) };
        assert_eq!(rc, 0);

        let result = rust_cmap_get(0);
        assert!(!result.is_null());
        unsafe {
            assert_eq!(*result, 0xAA);
            assert_eq!(*result.add(1), 0xBB);
            assert_eq!(*result.add(2), 0xCC);
        }
        reset_cmap();
    }

    /// @requirement REQ-CMAP-010
    #[serial]
    #[test]
    fn test_cmap_get_invalid_index() {
        assert_eq!(init_cmap(), 0);
        let result = rust_cmap_get(-1);
        assert!(result.is_null());
        let result = rust_cmap_get(MAX_COLORMAPS as c_int);
        assert!(result.is_null());
        reset_cmap();
    }

    /// @requirement REQ-CMAP-010
    #[serial]
    #[test]
    fn test_cmap_get_unset_index() {
        assert_eq!(init_cmap(), 0);
        let result = rust_cmap_get(5);
        assert!(result.is_null());
        reset_cmap();
    }

    /// @requirement REQ-CMAP-010
    #[serial]
    #[test]
    fn test_cmap_get_not_initialized() {
        reset_cmap();
        let result = rust_cmap_get(0);
        assert!(result.is_null());
    }

    /// @requirement REQ-CMAP-010
    #[serial]
    #[test]
    fn test_cmap_from_index_found() {
        assert_eq!(init_cmap(), 0);
        let data = vec![0u8; NUMBER_OF_PLUTVALS * PLUTVAL_BYTE_SIZE];
        unsafe { rust_cmap_set(3, data.as_ptr(), data.len() as c_int) };

        let idx = rust_cmap_from_index(3);
        assert_eq!(idx, 3);
        reset_cmap();
    }

    /// @requirement REQ-CMAP-010
    #[serial]
    #[test]
    fn test_cmap_from_index_not_found() {
        assert_eq!(init_cmap(), 0);
        let idx = rust_cmap_from_index(42);
        assert_eq!(idx, -1);
        reset_cmap();
    }

    /// @requirement REQ-CMAP-010
    #[serial]
    #[test]
    fn test_cmap_from_index_not_initialized() {
        reset_cmap();
        let idx = rust_cmap_from_index(0);
        assert_eq!(idx, -1);
    }

    // -- REQ-CMAP-020: Fade tests -------------------------------------------

    /// @requirement REQ-CMAP-020
    #[serial]
    #[test]
    fn test_cmap_fade_screen_to_black_immediate() {
        assert_eq!(init_cmap(), 0);
        let rc = rust_cmap_fade_screen(0, 0);
        assert_eq!(rc, 0);
        assert_eq!(rust_cmap_get_fade_amount(), FADE_NO_INTENSITY);
        reset_cmap();
    }

    /// @requirement REQ-CMAP-020
    #[serial]
    #[test]
    fn test_cmap_fade_screen_to_white_immediate() {
        assert_eq!(init_cmap(), 0);
        let rc = rust_cmap_fade_screen(2, 0);
        assert_eq!(rc, 0);
        assert_eq!(rust_cmap_get_fade_amount(), FADE_FULL_INTENSITY);
        reset_cmap();
    }

    /// @requirement REQ-CMAP-020
    #[serial]
    #[test]
    fn test_cmap_fade_screen_to_normal_immediate() {
        assert_eq!(init_cmap(), 0);
        // First fade to black
        rust_cmap_fade_screen(0, 0);
        assert_eq!(rust_cmap_get_fade_amount(), FADE_NO_INTENSITY);
        // Then back to normal
        let rc = rust_cmap_fade_screen(1, 0);
        assert_eq!(rc, 0);
        assert_eq!(rust_cmap_get_fade_amount(), FADE_NORMAL_INTENSITY);
        reset_cmap();
    }

    /// @requirement REQ-CMAP-020
    #[serial]
    #[test]
    fn test_cmap_fade_screen_invalid_direction() {
        assert_eq!(init_cmap(), 0);
        let rc = rust_cmap_fade_screen(99, 100);
        assert_eq!(rc, -1);
        reset_cmap();
    }

    /// @requirement REQ-CMAP-020
    #[serial]
    #[test]
    fn test_cmap_fade_screen_not_initialized() {
        reset_cmap();
        let rc = rust_cmap_fade_screen(0, 100);
        assert_eq!(rc, -1);
    }

    /// @requirement REQ-CMAP-020
    #[serial]
    #[test]
    fn test_cmap_get_fade_amount_default() {
        assert_eq!(init_cmap(), 0);
        assert_eq!(rust_cmap_get_fade_amount(), FADE_NORMAL_INTENSITY);
        reset_cmap();
    }

    /// @requirement REQ-CMAP-020
    #[serial]
    #[test]
    fn test_cmap_get_fade_amount_not_initialized() {
        reset_cmap();
        assert_eq!(rust_cmap_get_fade_amount(), FADE_NORMAL_INTENSITY);
    }

    // -- REQ-CMAP-020: Xform step tests -------------------------------------

    /// @requirement REQ-CMAP-020
    #[serial]
    #[test]
    fn test_cmap_xform_step_no_active() {
        assert_eq!(init_cmap(), 0);
        let rc = rust_cmap_xform_step();
        assert_eq!(rc, 0);
        reset_cmap();
    }

    /// @requirement REQ-CMAP-020
    #[serial]
    #[test]
    fn test_cmap_xform_step_not_initialized() {
        reset_cmap();
        let rc = rust_cmap_xform_step();
        assert_eq!(rc, -1);
    }

    /// @requirement REQ-CMAP-020
    #[serial]
    #[test]
    fn test_cmap_flush_xforms_success() {
        assert_eq!(init_cmap(), 0);
        let rc = rust_cmap_flush_xforms();
        assert_eq!(rc, 0);
        reset_cmap();
    }

    /// @requirement REQ-CMAP-020
    #[serial]
    #[test]
    fn test_cmap_flush_xforms_not_initialized() {
        reset_cmap();
        let rc = rust_cmap_flush_xforms();
        assert_eq!(rc, -1);
    }

    // -- REQ-CMAP-030: Set palette tests ------------------------------------

    /// @requirement REQ-CMAP-030
    #[serial]
    #[test]
    fn test_cmap_set_palette_basic() {
        assert_eq!(init_cmap(), 0);
        let mut palette_data = vec![0u8; NUMBER_OF_PLUTVALS * 3];
        palette_data[0] = 0xFF;
        palette_data[1] = 0x00;
        palette_data[2] = 0x80;
        let rc =
            unsafe { rust_cmap_set_palette(palette_data.as_ptr(), NUMBER_OF_PLUTVALS as c_int) };
        assert_eq!(rc, 0);

        // Verify via get
        let result = rust_cmap_get(0);
        assert!(!result.is_null());
        unsafe {
            assert_eq!(*result, 0xFF);
            assert_eq!(*result.add(1), 0x00);
            assert_eq!(*result.add(2), 0x80);
        }
        reset_cmap();
    }

    /// @requirement REQ-CMAP-030
    #[serial]
    #[test]
    fn test_cmap_set_palette_partial() {
        assert_eq!(init_cmap(), 0);
        let palette_data = [0xAAu8, 0xBB, 0xCC]; // Just one color
        let rc = unsafe { rust_cmap_set_palette(palette_data.as_ptr(), 1) };
        assert_eq!(rc, 0);
        reset_cmap();
    }

    /// @requirement REQ-CMAP-030
    #[serial]
    #[test]
    fn test_cmap_set_palette_null_data() {
        assert_eq!(init_cmap(), 0);
        let rc = unsafe { rust_cmap_set_palette(ptr::null(), 256) };
        assert_eq!(rc, -1);
        reset_cmap();
    }

    /// @requirement REQ-CMAP-030
    #[serial]
    #[test]
    fn test_cmap_set_palette_zero_count() {
        assert_eq!(init_cmap(), 0);
        let data = [0u8; 3];
        let rc = unsafe { rust_cmap_set_palette(data.as_ptr(), 0) };
        assert_eq!(rc, -1);
        reset_cmap();
    }

    /// @requirement REQ-CMAP-030
    #[serial]
    #[test]
    fn test_cmap_set_palette_not_initialized() {
        reset_cmap();
        let data = vec![0u8; 768];
        let rc = unsafe { rust_cmap_set_palette(data.as_ptr(), 256) };
        assert_eq!(rc, -1);
    }

    // -- REQ-FFI-030: Panic safety / edge case tests -------------------------

    /// @requirement REQ-FFI-030
    #[serial]
    #[test]
    fn test_cmap_set_too_small_data() {
        assert_eq!(init_cmap(), 0);
        let data = [0u8; 10];
        let rc = unsafe { rust_cmap_set(0, data.as_ptr(), 10) };
        assert_eq!(rc, -1);
        reset_cmap();
    }

    /// @requirement REQ-FFI-030
    #[serial]
    #[test]
    fn test_cmap_set_out_of_range_index() {
        assert_eq!(init_cmap(), 0);
        let data = vec![0u8; NUMBER_OF_PLUTVALS * PLUTVAL_BYTE_SIZE];
        let rc =
            unsafe { rust_cmap_set(MAX_COLORMAPS as c_int, data.as_ptr(), data.len() as c_int) };
        assert_eq!(rc, -1);
        reset_cmap();
    }

    /// @requirement REQ-FFI-030
    #[serial]
    #[test]
    fn test_cmap_full_lifecycle() {
        assert_eq!(init_cmap(), 0);

        // Set a palette
        let mut data = vec![0u8; NUMBER_OF_PLUTVALS * PLUTVAL_BYTE_SIZE];
        data[0] = 255;
        data[1] = 128;
        data[2] = 64;
        unsafe { rust_cmap_set(0, data.as_ptr(), data.len() as c_int) };

        // Read it back
        let result = rust_cmap_get(0);
        assert!(!result.is_null());
        unsafe {
            assert_eq!(*result, 255);
            assert_eq!(*result.add(1), 128);
            assert_eq!(*result.add(2), 64);
        }

        // Check fade default
        assert_eq!(rust_cmap_get_fade_amount(), FADE_NORMAL_INTENSITY);

        // Fade to black immediately
        assert_eq!(rust_cmap_fade_screen(0, 0), 0);
        assert_eq!(rust_cmap_get_fade_amount(), FADE_NO_INTENSITY);

        // Fade back to normal
        assert_eq!(rust_cmap_fade_screen(1, 0), 0);
        assert_eq!(rust_cmap_get_fade_amount(), FADE_NORMAL_INTENSITY);

        // Step xforms (none active)
        assert_eq!(rust_cmap_xform_step(), 0);

        // Flush xforms
        assert_eq!(rust_cmap_flush_xforms(), 0);

        // Look up by index
        assert_eq!(rust_cmap_from_index(0), 0);
        assert_eq!(rust_cmap_from_index(99), -1);

        // Uninit
        rust_cmap_uninit();
        assert!(get_cmap_state().is_none());
    }
}
