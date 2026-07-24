//! Init-sequence wrappers for the UQM startup.
//!
//! Provides Rust-side equivalents of the C init functions called from
//! `uqm_c_do_init()`. Each is exported as `#[no_mangle] extern "C"` for
//! C to call when `RUST_OWNS_MAIN`.

use libc::c_int;

// ===========================================================================
// #1 TFB_PreInit — SDL video init
// ===========================================================================

extern "C" {
    fn SDL_Init(flags: u32) -> c_int;
    fn SDL_GetVersion(ver: *mut SdlVersion);
}

#[repr(C)]
struct SdlVersion {
    major: u8,
    minor: u8,
    patch: u8,
}

const SDL_INIT_VIDEO: u32 = 0x00000020;

/// Initialize SDL video subsystem from Rust.
///
/// Equivalent to C `TFB_PreInit()`: calls `SDL_Init(SDL_INIT_VIDEO)` and
/// logs the SDL version. SDL is cleaned up at process exit by the OS.
#[no_mangle]
pub extern "C" fn rust_tfb_preinit() -> c_int {
    tracing::info!("Initializing base SDL functionality.");

    let result = unsafe { SDL_Init(SDL_INIT_VIDEO) };
    if result != 0 {
        tracing::error!("Could not initialize SDL");
        return -1;
    }

    let mut ver = SdlVersion {
        major: 0,
        minor: 0,
        patch: 0,
    };
    unsafe { SDL_GetVersion(&mut ver) };
    tracing::info!(
        "Using SDL version {}.{}.{} (compiled with {}.{}.{})",
        ver.major,
        ver.minor,
        ver.patch,
        ver.major,
        ver.minor,
        ver.patch,
    );

    0
}

// ===========================================================================
// #2 log_initThreads — create logging lock
// ===========================================================================

/// Initialize the logging thread lock.
///
/// The C version creates a Mutex for the logging system. With tracing,
/// logging is already thread-safe, so this is a no-op that just ensures
/// compatibility with C code that checks the lock.
#[no_mangle]
pub extern "C" fn rust_log_init_threads() {
    // tracing::subscriber is already thread-safe; no mutex needed.
    tracing::debug!("log_initThreads: tracing handles thread safety natively");
}

// #6 InitColorMaps — delegate to existing Rust colormap system
// ===========================================================================

extern "C" {
    fn rust_cmap_init() -> c_int;
}

/// Initialize the colormap system via the existing Rust implementation.
///
/// Equivalent to C `InitColorMaps()` but delegates to `rust_cmap_init()`
/// from `graphics::cmap_ffi`.
#[no_mangle]
pub extern "C" fn rust_init_color_maps() -> c_int {
    let result = unsafe { rust_cmap_init() };
    if result == 0 {
        tracing::debug!("ColorMap system initialized (Rust)");
    } else {
        tracing::error!("ColorMap system init failed (code {})", result);
    }
    result
}

// ===========================================================================
// #5 TFB_InitGraphics — delegate to C (Rust driver is configured by C)
// ===========================================================================

extern "C" {
    fn TFB_InitGraphics(
        driver: c_int,
        flags: c_int,
        backend: *const libc::c_char,
        width: c_int,
        height: c_int,
    ) -> c_int;
}

/// Initialize the graphics subsystem.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
///
/// Currently delegates to C `TFB_InitGraphics()` which configures the SDL
/// driver. The Rust graphics driver is selected via build config.
#[no_mangle]
pub unsafe extern "C" fn rust_tfb_init_graphics(
    driver: c_int,
    flags: c_int,
    backend: *const libc::c_char,
    width: c_int,
    height: c_int,
) -> c_int {
    TFB_InitGraphics(driver, flags, backend, width, height)
}

// ===========================================================================
// #7 init_communication — delegate to C
// ===========================================================================

extern "C" {
    fn init_communication();
}

/// Initialize the communication system.
///
/// Delegates to C `init_communication()` which sets up comm globals.
/// The Rust comm module (`comm::`) has its own FFI layer that runs alongside.
#[no_mangle]
pub extern "C" fn rust_init_communication() {
    unsafe { init_communication() };
    tracing::debug!("Communication system initialized");
}

// ===========================================================================
// #8 TFB_InitInput — delegate to C
// ===========================================================================

extern "C" {
    #[allow(
        clashing_extern_declarations,
        reason = "C ABI compatibility is fixed during the Rust migration; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
    )]
    fn TFB_InitInput(driver: c_int, flags: c_int);
}

/// Initialize the input subsystem.
///
/// Delegates to C `TFB_InitInput()` which configures the SDL input driver.
#[no_mangle]
pub extern "C" fn rust_tfb_init_input(driver: c_int, flags: c_int) {
    unsafe { TFB_InitInput(driver, flags) };
    tracing::debug!(
        "Input system initialized (driver={}, flags={})",
        driver,
        flags
    );
}

// ===========================================================================
// Tests
// ===========================================================================
