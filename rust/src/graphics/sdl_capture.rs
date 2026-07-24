//! ABI-authoritative SDL surface capture helpers.
//!
//! These functions wrap the C accessors compiled against the real linked SDL2
//! headers. They provide safe Rust access to SDL_Surface width/height/pitch/
//! pixels/format/BPP/masks and the SDL_MUSTLOCK macro.
//!
//! The lock-copy-unlock helper is the single shared production helper — both
//! capture code and tests call the same C function (`uqm_sdl_lock_copy_unlock`).
//!
//! @plan PLAN-20260723-RUNTIME-AUTOMATION.P00 §7

use std::ffi::c_void;

// These FFI declarations link against the C harness accessors compiled in build.rs
// via cc::Build::compile("p00_sdl_accessors"). The cc crate auto-links the resulting
// static library into all targets (lib, bin, test).
extern "C" {
    fn uqm_sdl_surface_w(surf: *const c_void) -> i32;
    fn uqm_sdl_surface_h(surf: *const c_void) -> i32;
    fn uqm_sdl_surface_pitch(surf: *const c_void) -> i32;
    fn uqm_sdl_surface_flags(surf: *const c_void) -> u32;
    fn uqm_sdl_surface_format(surf: *const c_void) -> *const c_void;
    fn uqm_sdl_must_lock(surf: *const c_void) -> u8; // SDL_bool

    fn uqm_sdl_format_bpp(fmt: *const c_void) -> u8;
    fn uqm_sdl_format_bytesPerPixel(fmt: *const c_void) -> u8;
    fn uqm_sdl_format_Rmask(fmt: *const c_void) -> u32;
    fn uqm_sdl_format_Gmask(fmt: *const c_void) -> u32;
    fn uqm_sdl_format_Bmask(fmt: *const c_void) -> u32;
    fn uqm_sdl_format_Amask(fmt: *const c_void) -> u32;

    /// The ONE shared production lock-copy-unlock helper.
    /// Returns 0 on success, -1 on lock failure, -2 on null/invalid.
    fn uqm_sdl_lock_copy_unlock(surf: *mut c_void, dst: *mut c_void, len: usize) -> i32;

    fn uqm_sdl_create_mustlock_surface(width: i32, height: i32) -> *mut c_void;
    fn uqm_sdl_inject_lock_failure(enable: i32);
    fn uqm_sdl_is_lock_failure_injected() -> i32;
}

/// Surface metadata obtained via ABI-authoritative accessors.
#[derive(Debug, Clone, Copy)]
pub struct SurfaceInfo {
    pub width: i32,
    pub height: i32,
    pub pitch: i32,
    pub bpp: u8,
    pub bytes_per_pixel: u8,
    pub rmask: u32,
    pub gmask: u32,
    pub bmask: u32,
    pub amask: u32,
    pub must_lock: bool,
    pub flags: u32,
}

/// Query surface metadata via ABI-authoritative C accessors.
///
/// # Safety
/// `surface` must be a valid `SDL_Surface*` from the linked SDL2 library.
pub unsafe fn query_surface_info(surface: *const c_void) -> SurfaceInfo {
    let format_ptr = uqm_sdl_surface_format(surface);

    SurfaceInfo {
        width: uqm_sdl_surface_w(surface),
        height: uqm_sdl_surface_h(surface),
        pitch: uqm_sdl_surface_pitch(surface),
        bpp: uqm_sdl_format_bpp(format_ptr),
        bytes_per_pixel: uqm_sdl_format_bytesPerPixel(format_ptr),
        rmask: uqm_sdl_format_Rmask(format_ptr),
        gmask: uqm_sdl_format_Gmask(format_ptr),
        bmask: uqm_sdl_format_Bmask(format_ptr),
        amask: uqm_sdl_format_Amask(format_ptr),
        must_lock: uqm_sdl_must_lock(surface) != 0,
        flags: uqm_sdl_surface_flags(surface),
    }
}

/// Copy pixel bytes through the shared production lock/copy/unlock helper.
///
/// # Safety
/// `surface` must be a valid `SDL_Surface*`. `dst` must have at least `len` bytes.
pub unsafe fn lock_copy_unlock(
    surface: *mut c_void,
    dst: *mut u8,
    len: usize,
) -> Result<(), String> {
    let ret = uqm_sdl_lock_copy_unlock(surface, dst.cast(), len);
    match ret {
        0 => Ok(()),
        -1 => Err("SDL_LockSurface failed".into()),
        -2 => Err("invalid surface or buffer".into()),
        _ => Err(format!("unknown error: {ret}")),
    }
}

/// Create a real SDL surface that satisfies SDL_MUSTLOCK (RLEACCEL).
///
/// Returns a raw `SDL_Surface*` or null on failure. Caller must `SDL_FreeSurface`.
///
/// # Safety
/// The returned pointer must be freed with `SDL_FreeSurface`.
pub unsafe fn create_mustlock_surface(width: i32, height: i32) -> *mut c_void {
    uqm_sdl_create_mustlock_surface(width, height)
}

/// Inject a simulated lock failure for fault-injection testing.
pub fn inject_lock_failure(enable: bool) {
    unsafe { uqm_sdl_inject_lock_failure(if enable { 1 } else { 0 }) };
}

/// Check if lock failure is currently injected.
pub fn is_lock_failure_injected() -> bool {
    unsafe { uqm_sdl_is_lock_failure_injected() != 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_query_mustlock_surface() {
        use sdl2::sys::SDL_FreeSurface;

        unsafe {
            let surf = create_mustlock_surface(320, 240);
            assert!(!surf.is_null(), "Failed to create MUSTLOCK surface");

            let info = query_surface_info(surf);
            assert_eq!(info.width, 320);
            assert_eq!(info.height, 240);
            assert!(info.must_lock, "Surface must require locking (RLEACCEL)");
            assert_eq!(info.bpp, 32);
            assert_eq!(info.bytes_per_pixel, 4);

            SDL_FreeSurface(surf as *mut sdl2::sys::SDL_Surface);
        }
    }

    #[test]
    fn test_lock_copy_unlock_success() {
        use sdl2::sys::{SDL_CreateRGBSurface, SDL_FreeSurface};

        unsafe {
            let surf =
                SDL_CreateRGBSurface(0, 4, 4, 32, 0xFF000000, 0x00FF0000, 0x0000FF00, 0x000000FF);
            assert!(!surf.is_null());

            // Write known data to the surface pixels before locking
            let pixels = (*surf).pixels as *mut u8;
            assert!(!pixels.is_null());

            // Lock to write initial data via the C accessors
            extern "C" {
                fn uqm_sdl_lock(surf: *mut c_void) -> i32;
                fn uqm_sdl_unlock(surf: *mut c_void);
            }
            assert_eq!(uqm_sdl_lock(surf as *mut c_void), 0);
            for i in 0..(4 * 4 * 4) {
                *pixels.add(i) = (i % 256) as u8;
            }
            uqm_sdl_unlock(surf as *mut c_void);

            // Use the shared helper to copy
            let mut dst = [0u8; 4 * 4 * 4];
            let ret = lock_copy_unlock(surf as *mut c_void, dst.as_mut_ptr(), dst.len());
            assert!(ret.is_ok(), "lock_copy_unlock should succeed");

            // Verify the data
            for (i, byte) in dst.iter().enumerate() {
                assert_eq!(*byte, (i % 256) as u8, "byte mismatch at {}", i);
            }

            SDL_FreeSurface(surf);
        }
    }

    #[test]
    fn test_injected_lock_failure_no_read() {
        use sdl2::sys::{SDL_CreateRGBSurface, SDL_FreeSurface};

        unsafe {
            let surf =
                SDL_CreateRGBSurface(0, 4, 4, 32, 0xFF000000, 0x00FF0000, 0x0000FF00, 0x000000FF);
            assert!(!surf.is_null());

            // Write known initial data
            let pixels = (*surf).pixels as *mut u8;

            extern "C" {
                fn uqm_sdl_lock(surf: *mut c_void) -> i32;
                fn uqm_sdl_unlock(surf: *mut c_void);
            }
            assert_eq!(uqm_sdl_lock(surf as *mut c_void), 0);
            for i in 0..(4 * 4 * 4) {
                *pixels.add(i) = 0xAA;
            }
            uqm_sdl_unlock(surf as *mut c_void);

            // Inject lock failure
            inject_lock_failure(true);
            assert!(is_lock_failure_injected());

            // The helper must fail and NOT read pixels
            let mut dst = [0u8; 4 * 4 * 4];
            // Fill with sentinel to detect any partial read
            dst.fill(0xBB);

            let ret = lock_copy_unlock(surf as *mut c_void, dst.as_mut_ptr(), dst.len());
            assert!(
                ret.is_err(),
                "lock_copy_unlock should fail with injected lock failure"
            );

            // Verify NO data was read — all bytes must still be the sentinel
            for (i, byte) in dst.iter().enumerate() {
                assert_eq!(
                    *byte, 0xBB,
                    "pixel data was read despite lock failure at {}",
                    i
                );
            }

            // Clear injection
            inject_lock_failure(false);
            assert!(!is_lock_failure_injected());

            SDL_FreeSurface(surf);
        }
    }

    #[test]
    fn test_null_surface_returns_error() {
        unsafe {
            let mut dst = [0u8; 16];
            let ret = lock_copy_unlock(std::ptr::null_mut(), dst.as_mut_ptr(), dst.len());
            assert!(ret.is_err(), "null surface should return error");
        }
    }
}
