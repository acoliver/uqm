//! SDL reconfiguration must keep the screen canvases and the real window.
use sdl2_sys as sys;
use uqm_rust::graphics::dcq::Screen;
use uqm_rust::graphics::present::{init_graphics, uninit_graphics, GraphicsOptions};
use uqm_rust::graphics::state::{self, VideoMode};

#[test]
fn fullscreen_and_resize_keep_the_window_and_screen_owners() {
    std::env::set_var("SDL_VIDEODRIVER", "dummy");
    std::env::set_var("SDL_AUDIODRIVER", "dummy");
    init_graphics(1, 0, GraphicsOptions::default(), 640, 480);
    // SAFETY: the window id comes from the SDL video subsystem this test initialized.
    let window = unsafe { sys::SDL_GetWindowFromID(1) };
    assert!(!window.is_null());
    state::draw_on_screen(Screen::Main, |canvas| canvas.set_pixel(3, 4, 0x1234_5600));

    let reinit = |flags, width, height| {
        state::reinit_video(VideoMode {
            driver: 1,
            flags,
            width,
            height,
        });
        state::flush();
    };
    reinit(1, 640, 480);
    let fullscreen = sys::SDL_WindowFlags::SDL_WINDOW_FULLSCREEN_DESKTOP as u32;
    // SAFETY: the window stays owned by the live display.
    unsafe {
        assert_eq!(sys::SDL_GetWindowFlags(window) & fullscreen, fullscreen);
        assert_eq!(sys::SDL_GetWindowFromID(1), window);
    }
    reinit(0, 800, 600);
    // SAFETY: as above.
    unsafe {
        assert_eq!(sys::SDL_GetWindowFlags(window) & fullscreen, 0);
        let (mut width, mut height) = (0, 0);
        sys::SDL_GetWindowSize(window, &mut width, &mut height);
        assert_eq!((width, height), (800, 600));
    }
    let mode = state::video_mode();
    assert_eq!((mode.width, mode.height, mode.flags), (800, 600, 0));
    assert_eq!(
        state::draw_on_screen(Screen::Main, |canvas| canvas.pixel(3, 4)),
        0x1234_5600,
        "reconfiguration keeps the screen canvases"
    );
    uninit_graphics();
    assert!(!state::initialized());
}
