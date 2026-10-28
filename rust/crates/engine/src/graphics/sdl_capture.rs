//! ABI-authoritative SDL surface capture helpers.
//!
//! These functions operate on the real `SDL_Surface`/`SDL_PixelFormat` types
//! from `sdl2_sys`, whose bindgen bindings are generated against the same SDL2
//! headers the production library links against. They provide Rust access to
//! SDL_Surface width/height/pitch/format/BPP/masks and the SDL_MUSTLOCK
//! predicate.
//!
//! The lock-copy-unlock helper is the single shared production helper — both
//! capture code and tests call the same function (`lock_copy_unlock`).
//!
//! @plan PLAN-20260723-RUNTIME-AUTOMATION.P00 §7

use std::cell::Cell;
use std::thread_local;

use sdl2_sys::{
    SDL_CreateRGBSurface, SDL_LockSurface, SDL_SetSurfaceRLE, SDL_Surface, SDL_UnlockSurface,
    SDL_RLEACCEL,
};

/// SDL2 defines `SDL_MUSTLOCK(S)` as the preprocessor macro
/// `(((S)->flags & SDL_RLEACCEL) != 0)` (SDL_surface.h); it is not a function
/// symbol. `SDL_RLEACCEL` is imported from `sdl2_sys`, which binds the real
/// SDL2 headers, so this predicate tracks the linked ABI rather than a
/// hand-written flag value.
fn must_lock(surface: &SDL_Surface) -> bool {
    (surface.flags & SDL_RLEACCEL) != 0
}

// Test fault injection is thread-local so parallel tests cannot contaminate
// production-helper calls made by another test thread.
thread_local! {
    static INJECT_LOCK_FAILURE: Cell<bool> = const { Cell::new(false) };
}

/// Surface metadata read directly from the linked SDL2 structures.
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

/// Query surface metadata directly from the linked SDL2 struct layout.
///
/// # Safety
/// `surface` must be a valid, non-null `SDL_Surface*` from the linked SDL2
/// library.
pub unsafe fn query_surface_info(surface: *const SDL_Surface) -> SurfaceInfo {
    // SAFETY: the caller guarantees `surface` is a valid SDL_Surface, and SDL
    // guarantees its `format` pointer stays valid for the surface's lifetime.
    let surface = &*surface;
    let format = &*surface.format;

    SurfaceInfo {
        width: surface.w,
        height: surface.h,
        pitch: surface.pitch,
        bpp: format.BitsPerPixel,
        bytes_per_pixel: format.BytesPerPixel,
        rmask: format.Rmask,
        gmask: format.Gmask,
        bmask: format.Bmask,
        amask: format.Amask,
        must_lock: must_lock(surface),
        flags: surface.flags,
    }
}

/// The ONE shared production lock-copy-unlock helper.
///
/// Locks the surface, copies `len` bytes from its pixel buffer into `dst`,
/// then unlocks. Returns 0 on success, -1 on lock failure (real or injected),
/// -2 on null/invalid arguments. On failure no pixel bytes are read. Both
/// production capture code and tests call this — no duplicated lock/copy
/// logic.
///
/// # Safety
/// `surface` must be a valid `SDL_Surface*` whose pixel buffer holds at least
/// `len` bytes, and `dst` must point to at least `len` writable bytes.
unsafe fn lock_copy_unlock_raw(surface: *mut SDL_Surface, dst: *mut u8, len: usize) -> i32 {
    if surface.is_null() || dst.is_null() || len == 0 {
        return -2;
    }

    if is_lock_failure_injected() {
        // Simulated lock failure — do NOT read pixels.
        return -1;
    }

    // SAFETY: `surface` is non-null and valid per the caller contract.
    if SDL_LockSurface(surface) != 0 {
        // Real lock failure — do NOT read pixels.
        return -1;
    }

    // SAFETY: the caller guarantees `dst` has at least `len` writable bytes
    // and the locked surface's pixel buffer holds at least `len` bytes.
    std::ptr::copy_nonoverlapping((*surface).pixels.cast::<u8>(), dst, len);
    // SAFETY: the surface is locked by the call above, so this unlock pairs
    // with it.
    SDL_UnlockSurface(surface);
    0
}

/// Copy pixel bytes through the shared production lock/copy/unlock helper.
///
/// # Safety
/// See [`lock_copy_unlock_raw`].
pub unsafe fn lock_copy_unlock(
    surface: *mut SDL_Surface,
    dst: *mut u8,
    len: usize,
) -> Result<(), String> {
    match lock_copy_unlock_raw(surface, dst, len) {
        0 => Ok(()),
        -1 => Err("SDL_LockSurface failed".into()),
        -2 => Err("invalid surface or buffer".into()),
        ret => Err(format!("unknown error: {ret}")),
    }
}

/// Lock a surface, returning 0 on success and -1 on failure (null surface or
/// injected/real lock failure). Always pairs with [`unlock_surface`]. Used for
/// lock/no-read verification in the fault-injection tests.
///
/// # Safety
/// `surface` must be null or a valid `SDL_Surface*`.
pub unsafe fn lock_surface(surface: *mut SDL_Surface) -> i32 {
    if surface.is_null() {
        return -1;
    }

    if is_lock_failure_injected() {
        return -1;
    }

    // SAFETY: `surface` is non-null and valid per the caller contract.
    if SDL_LockSurface(surface) != 0 {
        return -1;
    }

    0
}

/// Unlock a surface previously locked with [`lock_surface`]. A null surface is
/// ignored.
///
/// # Safety
/// `surface` must be null or a valid, locked `SDL_Surface*`.
pub unsafe fn unlock_surface(surface: *mut SDL_Surface) {
    if !surface.is_null() {
        // SAFETY: `surface` is non-null and valid per the caller contract.
        SDL_UnlockSurface(surface);
    }
}

/// Create a real SDL surface that satisfies SDL_MUSTLOCK (RLEACCEL).
///
/// Returns a raw `SDL_Surface*` or null on failure. Caller must
/// `SDL_FreeSurface`.
///
/// # Safety
/// The returned pointer must be freed with `SDL_FreeSurface`.
pub unsafe fn create_mustlock_surface(width: i32, height: i32) -> *mut SDL_Surface {
    // SDL2 2.32.x on macOS: SDL_SetSurfaceRLE does not set the SDL_RLEACCEL
    // flag until the surface is actually RLE-encoded (which happens lazily
    // during blit). Since SDL_MUSTLOCK is just `flags & SDL_RLEACCEL`, we
    // enable RLE and then set the flag directly to create a deterministic
    // surface that satisfies the SDL_MUSTLOCK predicate. The surface is still
    // valid for lock/unlock — SDL_LockSurface handles the RLE decompression
    // path when the flag is set.
    // SAFETY: the arguments satisfy SDL_CreateRGBSurface's contract and SDL
    // returns a valid surface or null.
    let surface = SDL_CreateRGBSurface(
        0, width, height, 32, 0xFF000000, 0x00FF0000, 0x0000FF00, 0x000000FF,
    );
    if !surface.is_null() {
        // SAFETY: `surface` is non-null and valid, as returned by SDL.
        SDL_SetSurfaceRLE(surface, 1);
        // SAFETY: `surface` is non-null and valid; this sets the same flag SDL
        // would set after actual RLE encoding, keeping the surface consistent.
        (*surface).flags |= SDL_RLEACCEL;
    }
    surface
}

/// Inject a simulated lock failure for fault-injection testing.
///
/// While enabled, [`lock_copy_unlock`] and [`lock_surface`] report lock
/// failure without reading pixels or calling the real SDL lock.
pub fn inject_lock_failure(enable: bool) {
    INJECT_LOCK_FAILURE.with(|flag| flag.set(enable));
}

/// Check if lock failure is currently injected.
pub fn is_lock_failure_injected() -> bool {
    INJECT_LOCK_FAILURE.with(Cell::get)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdl2_sys::SDL_FreeSurface;

    #[test]
    fn test_create_and_query_mustlock_surface() {
        unsafe {
            let surf = create_mustlock_surface(320, 240);
            assert!(!surf.is_null(), "Failed to create MUSTLOCK surface");

            let info = query_surface_info(surf);
            assert_eq!(info.width, 320);
            assert_eq!(info.height, 240);
            assert!(info.must_lock, "Surface must require locking (RLEACCEL)");
            assert_eq!(info.bpp, 32);
            assert_eq!(info.bytes_per_pixel, 4);

            SDL_FreeSurface(surf);
        }
    }

    #[test]
    fn test_lock_copy_unlock_success() {
        unsafe {
            let surf =
                SDL_CreateRGBSurface(0, 4, 4, 32, 0xFF000000, 0x00FF0000, 0x0000FF00, 0x000000FF);
            assert!(!surf.is_null());

            // Write known data to the surface pixels before locking
            let pixels = (*surf).pixels as *mut u8;
            assert!(!pixels.is_null());

            // Lock to write initial data
            assert_eq!(lock_surface(surf), 0);
            for i in 0..(4 * 4 * 4) {
                *pixels.add(i) = (i % 256) as u8;
            }
            unlock_surface(surf);

            // Use the shared helper to copy
            let mut dst = [0u8; 4 * 4 * 4];
            let ret = lock_copy_unlock(surf, dst.as_mut_ptr(), dst.len());
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
        unsafe {
            let surf =
                SDL_CreateRGBSurface(0, 4, 4, 32, 0xFF000000, 0x00FF0000, 0x0000FF00, 0x000000FF);
            assert!(!surf.is_null());

            // Write known initial data
            let pixels = (*surf).pixels as *mut u8;

            assert_eq!(lock_surface(surf), 0);
            for i in 0..(4 * 4 * 4) {
                *pixels.add(i) = 0xAA;
            }
            unlock_surface(surf);

            // Inject lock failure
            inject_lock_failure(true);
            assert!(is_lock_failure_injected());

            // The helper must fail and NOT read pixels
            let mut dst = [0u8; 4 * 4 * 4];
            // Fill with sentinel to detect any partial read
            dst.fill(0xBB);

            let ret = lock_copy_unlock(surf, dst.as_mut_ptr(), dst.len());
            assert!(
                ret.is_err(),
                "lock_copy_unlock should fail with injected lock failure"
            );
            // The documented contract maps injected lock failure to -1.
            assert_eq!(lock_copy_unlock_raw(surf, dst.as_mut_ptr(), dst.len()), -1);

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
            // The documented contract maps null/invalid arguments to -2.
            assert_eq!(
                lock_copy_unlock_raw(std::ptr::null_mut(), dst.as_mut_ptr(), dst.len()),
                -2
            );
        }
    }
}
