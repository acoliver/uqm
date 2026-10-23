//! SDL reconfiguration must keep the three live screen surfaces and real window.
use sdl2_sys as sys;
use std::{ffi::c_void, ptr};
use uqm_rust::graphics::{
    ffi,
    sdl::{lifecycle_abi::*, platform_abi::TFB_ReInitGraphics},
};
#[cfg_attr(
    target_os = "macos",
    link(name = "uqm_c", kind = "static", modifiers = "+whole-archive")
)]
extern "C" {
    static ScreenWidthActual: i32;
    static ScreenHeightActual: i32;
}
#[test]
fn fullscreen_and_resize_keep_the_window_and_screen_owners() {
    std::env::set_var("SDL_VIDEODRIVER", "dummy");
    std::env::set_var("SDL_AUDIODRIVER", "dummy");
    unsafe {
        assert_eq!(TFB_InitGraphics(1, 0, ptr::null(), 640, 480), 0);
        let window = sys::SDL_GetWindowFromID(1);
        assert!(!window.is_null());
        let surfaces: [*mut c_void; 3] =
            std::array::from_fn(|i| ffi::rust_gfx_get_screen_surface(i as i32).cast());
        assert_eq!(TFB_ReInitGraphics(1, 1, 640, 480), 0);
        let fullscreen = sys::SDL_WindowFlags::SDL_WINDOW_FULLSCREEN_DESKTOP as u32;
        assert_eq!(sys::SDL_GetWindowFlags(window) & fullscreen, fullscreen);
        assert_eq!(sys::SDL_GetWindowFromID(1), window);
        assert_eq!(TFB_ReInitGraphics(1, 0, 800, 600), 0);
        assert_eq!(sys::SDL_GetWindowFlags(window) & fullscreen, 0);
        let (mut width, mut height) = (0, 0);
        sys::SDL_GetWindowSize(window, &mut width, &mut height);
        assert_eq!((width, height), (800, 600));
        assert_eq!((ScreenWidthActual, ScreenHeightActual), (800, 600));
        for (i, &surface) in surfaces.iter().enumerate() {
            assert_eq!(
                ffi::rust_gfx_get_screen_surface(i as i32).cast::<c_void>(),
                surface
            );
        }
        TFB_UninitGraphics();
        assert!(ffi::rust_gfx_get_screen_surface(0).is_null());
    }
}
