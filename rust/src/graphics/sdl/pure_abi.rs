//! SDL2 video configuration over the existing Rust window and surface owner.
use super::lifecycle_abi::{refresh_rust_screen_globals, update_video_globals};
use crate::graphics::ffi;
use std::ffi::{c_char, c_int};

extern "C" {
    fn Scale_Init();
}

/// # Safety
/// Called on the SDL main thread; renderer name must be null or a live C string.
pub unsafe extern "C" fn tfb_pure_init_graphics(
    driver: c_int,
    flags: c_int,
    renderer: *const c_char,
    width: c_int,
    height: c_int,
) -> c_int {
    if ffi::rust_gfx_init(driver, flags, renderer, width, height) != 0 {
        std::process::exit(1);
    }
    if tfb_pure_configure_video(driver, flags, width, height, 0) != 0 {
        std::process::exit(1);
    }
    Scale_Init();
    0
}

/// # Safety
/// Called on the SDL main thread with the Rust graphics owner initialized.
pub unsafe extern "C" fn tfb_pure_configure_video(
    driver: c_int,
    flags: c_int,
    width: c_int,
    height: c_int,
    _togglefullscreen: c_int,
) -> c_int {
    if ffi::reconfigure_video(flags, width, height).is_err() {
        return -1;
    }
    update_video_globals(driver, flags, width, height);
    assert!(
        refresh_rust_screen_globals(),
        "initialized graphics surfaces"
    );
    0
}

/// # Safety
/// Called on the SDL main thread after graphics users have stopped.
pub unsafe extern "C" fn tfb_pure_uninit_graphics() {
    ffi::rust_gfx_uninit();
    super::lifecycle_abi::clear_rust_screen_globals();
}
