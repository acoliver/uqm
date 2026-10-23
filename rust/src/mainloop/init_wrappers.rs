//! Init-sequence wrappers for the UQM startup.
//!
//! SDL startup and tracing initialization for the Rust entry point.

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
pub fn rust_tfb_preinit() -> c_int {
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
pub fn rust_log_init_threads() {
    // tracing::subscriber is already thread-safe; no mutex needed.
    tracing::debug!("log_initThreads: tracing handles thread safety natively");
}
