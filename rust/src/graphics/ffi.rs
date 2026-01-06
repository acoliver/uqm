//! Graphics FFI layer - C ABI bindings.
//!
//! This module provides raw unsafe FFI bindings to the C graphics subsystem
//! defined in `sc2/src/libs/graphics/gfx_common.h` and related headers.
//!
//! # Safety Requirements
//!
//! ## Thread Safety
//!
//! The C graphics subsystem has specific threading constraints:
//!
//! - **Init/uninit operations** must be called from the main thread only.
//! - **Buffer swapping** (`TFB_SwapBuffers`) may have threading requirements.
//! - **Queue operations** (`TFB_BatchGraphics`, `TFB_UnbatchGraphics`) coordinate
//!   with the rendering thread.
//!
//! ## Initialization Sequence
//!
//! The graphics subsystem must be initialized in the following order:
//!
//! 1. `TFB_PreInit()` - Set up driver prerequisites
//! 2. `TFB_InitGraphics()` - Initialize the graphics driver
//! 3. (Optional) `TFB_ReInitGraphics()` - Reinitialize with new parameters
//! 4. Use graphics operations
//! 5. `TFB_UninitGraphics()` - Clean up and shutdown
//!
//! Calling any graphics operation before initialization or after uninitialization
//! results in **undefined behavior**.
//!
//! ## Draw Command Queue (DCQ) Batching
//!
//! The DCQ may be batched to optimize command submission:
//!
//! ```c
//! TFB_BatchGraphics();      // Start batching
//! // ... enqueue draw commands ...
//! TFB_UnbatchGraphics();    // End batching and trigger processing
//! ```
//!
//! **Invariants:**
//! - Batching must be properly nested. Calls to `TFB_BatchGraphics()` must be
//!   matched with `TFB_UnbatchGraphics()`.
//! - Between batch start and end, draw commands are queued but not processed.
//! - After unbatching, commands are processed asynchronously by the rendering thread.
//!
//! ## Null Pointers
//!
//! Most C functions in this module accept pointers. The following require valid,
//! non-null pointers unless otherwise documented:
//!
//! - `TFB_InitGraphics`: `renderer` may be NULL to use default renderer.
//! - Other pointer parameters: Generally required to be non-null.
//!
//! Passing invalid pointers to FFI functions results in **undefined behavior**.
//!
//! ## Global State
//!
//! The following global variables are exported from C and should be accessed
//! *after* successful initialization:
//!
//! - `GfxFlags`: Active graphics flags bitmask.
//! - `ScreenWidth`, `ScreenHeight`: Logical display dimensions.
//! - `ScreenWidthActual`, `ScreenHeightActual`: Physical display dimensions.
//! - `ScreenColorDepth`: Display color depth in bits.
//! - `GraphicsDriver`: Active driver ID.
//! - `FrameRate`: Current frame rate (FPS).
//! - `FrameRateTickBase`: Timing reference for frame rate calculation.
//!
//! Accessing these before `TFB_InitGraphics()` or after `TFB_UninitGraphics()`
//! may yield undefined values.

use std::ffi::{c_char, c_int};

// ============================================================================
// Type Definitions and Constants
// ============================================================================

/// Graphics driver backend IDs.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TfbGfxDriver {
    /// SDL OpenGL hardware-accelerated backend.
    SdlOpengl = 0,
    /// SDL pure software rendering backend.
    SdlPure = 1,
}

/// Forced redraw mode flags for buffer swapping.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TfbRedrawMode {
    /// No forced redraw.
    #[default]
    No = 0,
    /// Fading effect active - force redraw.
    Fading = 1,
    /// Exposure event (e.g., window moved/uncovered) - force redraw.
    Expose = 2,
    /// Full forced redraw.
    Yes = 3,
}

/// Graphics initialization and display flags.
///
/// These flags control rendering behavior and display options.
pub mod gfx_flags {
    /// Enable fullscreen mode.
    pub const FULLSCREEN: u32 = 1 << 0;
    /// Display frame rate (FPS) counter.
    pub const SHOWFPS: u32 = 1 << 1;
    /// Enable CRT-style scanline effect.
    pub const SCANLINES: u32 = 1 << 2;
    /// Use bilinear scaling filter.
    pub const SCALE_BILINEAR: u32 = 1 << 3;
    /// Use adaptive bilinear scaling.
    pub const SCALE_BIADAPT: u32 = 1 << 4;
    /// Use advanced adaptive bilinear scaling.
    pub const SCALE_BIADAPTADV: u32 = 1 << 5;
    /// Use triscan scaling algorithm.
    pub const SCALE_TRISCAN: u32 = 1 << 6;
    /// Use HQxx scaling family (e.g., HQ2x, HQ3x).
    pub const SCALE_HQXX: u32 = 1 << 7;

    /// Bitmask of all scaling mode flags.
    pub const SCALE_ANY: u32 = SCALE_BILINEAR | SCALE_BIADAPT | SCALE_BIADAPTADV | SCALE_TRISCAN | SCALE_HQXX;

    /// Scaling flags achievable in software (no OpenGL).
    pub const SCALE_SOFT_ONLY: u32 = SCALE_ANY & !SCALE_BILINEAR;
}

// ============================================================================
// Raw FFI Bindings
// ============================================================================

// Link to the C graphics library.
//
// The library is compiled from `sc2/src/libs/graphics/` sources.
//
// NOTE: To build this library, you must compile the C graphics sources.
// See build.rs for the compilation configuration.
// During unit tests, only the declarations are type-checked; linking is not performed.
#[cfg_attr(test, allow(dead_code))]
#[link(name = "uqm_graphics", kind = "static")]
extern "C" {
    // ------------------------------------------------------------------------
    // Initialization and Shutdown Functions
    // ------------------------------------------------------------------------

    /// Perform pre-initialization setup before graphics driver init.
    ///
    /// # Safety
    ///
    /// - Must be called from the main thread.
    /// - Must be called before any other graphics functions.
    /// - May not be called after graphics are already initialized.
    pub fn TFB_PreInit();

    /// Initialize the graphics subsystem with specified driver and parameters.
    ///
    /// # Parameters
    ///
    /// - `driver`: Graphics driver backend ID (0=SDL_OPENGL, 1=SDL_PURE).
    /// - `flags`: Bitmask of `gfx_flags` values.
    /// - `renderer`: Renderer name string, or NULL for default.
    /// - `width`: Requested display width in pixels.
    /// - `height`: Requested display height in pixels.
    ///
    /// # Returns
    ///
    /// Non-zero on success, zero on failure.
    ///
    /// # Safety
    ///
    /// - Must be called from the main thread.
    /// - Must be called after `TFB_PreInit()`.
    /// - `renderer` must be either NULL or a valid nul-terminated string pointer.
    /// - Must not be called when graphics are already initialized.
    #[allow(non_snake_case)]
    pub fn TFB_InitGraphics(
        driver: c_int,
        flags: c_int,
        renderer: *const c_char,
        width: c_int,
        height: c_int,
    ) -> c_int;

    /// Reinitialize graphics with new driver, flags, and dimensions.
    ///
    /// # Parameters
    ///
    /// - `driver`: Graphics driver backend ID.
    /// - `flags`: Bitmask of `gfx_flags` values.
    /// - `width`: New display width in pixels.
    /// - `height`: New display height in pixels.
    ///
    /// # Returns
    ///
    /// Non-zero on success, zero on failure.
    ///
    /// # Safety
    ///
    /// - Must be called from the main thread.
    /// - graphics must already be initialized via `TFB_InitGraphics()`.
    /// - Must not be called after `TFB_UninitGraphics()`.
    /// - Should be called when rendering is idle (no pending draw commands).
    #[allow(non_snake_case)]
    pub fn TFB_ReInitGraphics(
        driver: c_int,
        flags: c_int,
        width: c_int,
        height: c_int,
    ) -> c_int;

    /// Shutdown and cleanup the graphics subsystem.
    ///
    /// # Safety
    ///
    /// - Must be called from the main thread.
    /// - Must be called when graphics are initialized.
    /// - All draw commands should be flushed and processed before calling.
    /// - Must be called before program exit if graphics were initialized.
    #[allow(non_snake_case)]
    pub fn TFB_UninitGraphics();

    // ------------------------------------------------------------------------
    // Rendering and Buffer Operations
    // ------------------------------------------------------------------------

    /// Swap backbuffers with frontbuffer and update the display.
    ///
    /// # Parameters
    ///
    /// - `force_full_redraw`: Redraw mode (0=NO, 1=FADING, 2=EXPOSE, 3=YES).
    ///
    /// # Safety
    ///
    /// - graphics must be initialized.
    /// - Should be called from the main rendering thread.
    /// - Must not be called concurrently with init/uninit operations.
    #[allow(non_snake_case)]
    pub fn TFB_SwapBuffers(force_full_redraw: c_int);

    /// Process pending SDL events (input, window, etc.).
    ///
    /// # Safety
    ///
    /// - graphics may be in any initialized state.
    /// - Typically called from the main thread each frame.
    pub fn TFB_ProcessEvents();

    // ------------------------------------------------------------------------
    // Draw Command Queue (DCQ) Batch Operations
    // ------------------------------------------------------------------------

    /// Begin batching draw commands to the DCQ.
    ///
    /// When batching is enabled, draw commands are queued but not processed
    /// until `TFB_UnbatchGraphics()` is called. Multiple batch levels may be
    /// nested.
    ///
    /// # Safety
    ///
    /// - Must be paired with `TFB_UnbatchGraphics()`.
    /// - Excessive nesting may cause queue overrun.
    /// - Should not be called from within a DCQ callback.
    #[allow(non_snake_case)]
    pub fn TFB_BatchGraphics();

    /// End batching and trigger processing of queued draw commands.
    ///
    /// # Safety
    ///
    /// - Must be called after a matching `TFB_BatchGraphics()`.
    /// - Calling without an active batch results in undefined behavior.
    #[allow(non_snake_case)]
    pub fn TFB_UnbatchGraphics();

    /// Reset batch state, discarding any pending batched commands.
    ///
    /// # Safety
    ///
    /// - Should only be called during error recovery or special shutdown paths.
    #[allow(non_snake_case)]
    pub fn TFB_BatchReset();

    // ------------------------------------------------------------------------
    // Graphics Flush and Cleanup
    // ------------------------------------------------------------------------

    /// Flush and process all pending draw commands.
    ///
    /// Blocks until the DCQ is processed. Must be called from the main thread.
    ///
    /// # Safety
    ///
    /// - graphics must be initialized.
    /// - Must be called from the main thread only.
    /// - May block indefinitely if drawing commands are continuously queued.
    #[allow(non_snake_case)]
    pub fn TFB_FlushGraphics();

    /// Purge any dangling graphics resources during shutdown.
    ///
    /// Called as part of shutdown sequence to ensure all resources are freed.
    ///
    /// # Safety
    ///
    /// - Must be called from the main thread.
    /// - Should be called after all rendering has stopped.
    #[allow(non_snake_case)]
    pub fn TFB_PurgeDanglingGraphics();

    // ------------------------------------------------------------------------
    // Display Configuration
    // ------------------------------------------------------------------------

    /// Set the display gamma correction value.
    ///
    /// # Parameters
    ///
    /// - `gamma`: Gamma value (typically 0.5 to 3.0, where 1.0 is no correction).
    ///
    /// # Returns
    ///
    /// `true` if gamma was successfully set, `false` on failure.
    ///
    /// # Safety
    ///
    /// - graphics must be initialized.
    /// - Gamma setting may not be supported in pure software mode.
    #[allow(non_snake_case)]
    pub fn TFB_SetGamma(gamma: libc::c_float) -> bool;

    /// Upload the transition screen image.
    ///
    /// Used for crossfading transitions between screens.
    ///
    /// # Safety
    ///
    /// - graphics must be initialized.
    #[allow(non_snake_case)]
    pub fn TFB_UploadTransitionScreen();

    /// Check if hardware scaling is supported by the current driver.
    ///
    /// # Returns
    ///
    /// Non-zero if hardware scaling is available, zero if not.
    ///
    /// # Safety
    ///
    /// - graphics must be initialized.
    #[allow(non_snake_case)]
    pub fn TFB_SupportsHardwareScaling() -> c_int;

    // ------------------------------------------------------------------------
    // Global State Variables (Read Access)
    // ------------------------------------------------------------------------

    /// Active graphics flags bitmask.
    ///
    /// Modified by `TFB_InitGraphics()` and `TFB_ReInitGraphics()`.
    pub static GfxFlags: c_int;

    /// Logical screen width in pixels.
    pub static ScreenWidth: c_int;

    /// Logical screen height in pixels.
    pub static ScreenHeight: c_int;

    /// Actual displayed width (may differ from logical due to scaling).
    pub static ScreenWidthActual: c_int;

    /// Actual displayed height (may differ from logical due to scaling).
    pub static ScreenHeightActual: c_int;

    /// Display color depth in bits (typically 32).
    pub static ScreenColorDepth: c_int;

    /// Active graphics driver ID (0=SDL_OPENGL, 1=SDL_PURE).
    pub static GraphicsDriver: c_int;

    /// Current frame rate in frames per second.
    ///
    /// Updated periodically by the rendering driver.
    pub static FrameRate: libc::c_float;

    /// Timing reference base for frame rate calculation.
    pub static FrameRateTickBase: c_int;
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that enum values match C header constants.
    #[test]
    fn test_driver_enum_values() {
        assert_eq!(TfbGfxDriver::SdlOpengl as u32, 0);
        assert_eq!(TfbGfxDriver::SdlPure as u32, 1);
    }

    /// Test that redraw mode values match C header constants.
    #[test]
    fn test_redraw_mode_values() {
        assert_eq!(TfbRedrawMode::No as u32, 0);
        assert_eq!(TfbRedrawMode::Fading as u32, 1);
        assert_eq!(TfbRedrawMode::Expose as u32, 2);
        assert_eq!(TfbRedrawMode::Yes as u32, 3);
    }

    /// Test that gfx flags have correct bit values.
    #[test]
    fn test_gfx_flags_values() {
        assert_eq!(gfx_flags::FULLSCREEN, 1 << 0);
        assert_eq!(gfx_flags::SHOWFPS, 1 << 1);
        assert_eq!(gfx_flags::SCANLINES, 1 << 2);
        assert_eq!(gfx_flags::SCALE_BILINEAR, 1 << 3);
        assert_eq!(gfx_flags::SCALE_BIADAPT, 1 << 4);
        assert_eq!(gfx_flags::SCALE_BIADAPTADV, 1 << 5);
        assert_eq!(gfx_flags::SCALE_TRISCAN, 1 << 6);
        assert_eq!(gfx_flags::SCALE_HQXX, 1 << 7);
    }

    /// Test that SCALE_ANY combines all scaling flags.
    #[test]
    fn test_gfx_flags_scale_any() {
        let expected = gfx_flags::SCALE_BILINEAR
            | gfx_flags::SCALE_BIADAPT
            | gfx_flags::SCALE_BIADAPTADV
            | gfx_flags::SCALE_TRISCAN
            | gfx_flags::SCALE_HQXX;
        assert_eq!(gfx_flags::SCALE_ANY, expected);
    }

    /// Test that SCALE_SOFT_ONLY excludes bilinear scaling.
    #[test]
    fn test_gfx_flags_scale_soft_only() {
        let expected = gfx_flags::SCALE_ANY & !gfx_flags::SCALE_BILINEAR;
        assert_eq!(gfx_flags::SCALE_SOFT_ONLY, expected);
        assert!(gfx_flags::SCALE_SOFT_ONLY & gfx_flags::SCALE_BILINEAR == 0);
    }

    /// Test null pointer handling in InitGraphics.
    ///
    /// This test verifies that NULL may be passed for the renderer parameter.
    /// The actual graphics initialization is not performed to avoid dependencies
    /// on system display and event handling.
    #[test]
    fn test_null_renderer_pointer() {
        // Verify that NULL (nullptr) is a valid renderer argument.
        // In Rust, we represent this with std::ptr::null().
        let null_renderer: *const c_char = std::ptr::null();

        // We don't actually call TFB_InitGraphics because it requires
        // a full SDL/OpenGL context. This test verifies the type safety.
        let _ = null_renderer as usize;

        // Verify that null pointer has the expected representation.
        assert_eq!(std::ptr::null::<c_char>() as usize, 0);
    }

    /// Test pointer validity expectations.
    #[test]
    fn test_pointer_safety_notes() {
        // This test documents our safety requirements without calling FFI.

        // Safe: renderer can be NULL
        let _safe_null_renderer: *const c_char = std::ptr::null();

        // Unsafe: other pointers should generally be non-NULL
        // (This is documented in the safety section above)
        let _non_null_example: *const c_char = std::ptr::NonNull::dangling().as_ptr();

        // Document that we expect pointer size to be 64-bit on modern systems
        #[cfg(target_pointer_width = "64")]
        {
            assert_eq!(std::mem::size_of::<*const c_char>(), 8);
        }

        #[cfg(target_pointer_width = "32")]
        {
            assert_eq!(std::mem::size_of::<*const c_char>(), 4);
        }
    }

    /// Test that our types have the expected representation.
    #[test]
    fn test_type_representations() {
        // c_int should be 32 bits on all platforms we target
        assert_eq!(std::mem::size_of::<c_int>(), 4);

        // bool should match C bool (1 byte)
        assert_eq!(std::mem::size_of::<bool>(), 1);

        // c_float should match float (4 bytes)
        assert_eq!(std::mem::size_of::<libc::c_float>(), 4);

        // Our enums should be 32-bit to match C enum/u32
        assert_eq!(std::mem::size_of::<TfbGfxDriver>(), 4);
        assert_eq!(std::mem::size_of::<TfbRedrawMode>(), 4);
    }
}
