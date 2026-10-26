//! SDL lifecycle and composition, ported from sdl_common.c (GPL-2.0-or-later).
use crate::graphics::{
    ffi::{self, SDL_Rect, SDL_Surface},
    geometry_ffi::TFB_B_BOX,
    native_abi::RECT,
};
use std::{
    ffi::{c_char, c_int, c_void},
    ptr,
};
const NUM_SCREENS: usize = 3;
#[no_mangle]
pub static mut SDL_Screen: *mut SDL_Surface = ptr::null_mut();
pub static mut TRANSITION_SCREEN: *mut SDL_Surface = ptr::null_mut();
pub static mut SDL_SCREENS: [*mut SDL_Surface; NUM_SCREENS] = [ptr::null_mut(); NUM_SCREENS];
#[no_mangle]
pub static mut format_conv_surf: *mut SDL_Surface = ptr::null_mut();
#[no_mangle]
pub static mut GfxFlags: c_int = 0;
#[no_mangle]
pub static mut QuitPosted: c_int = 0;
#[no_mangle]
pub static mut GameActive: c_int = 1;
#[repr(C)]
pub struct TfbGraphicsBackend {
    preprocess: unsafe extern "C" fn(c_int, c_int, c_int),
    postprocess: unsafe extern "C" fn(),
    upload_transition_screen: unsafe extern "C" fn(),
    screen: unsafe extern "C" fn(c_int, u8, *const SDL_Rect),
    color: unsafe extern "C" fn(u8, u8, u8, u8, *const SDL_Rect),
}
static mut RUST_BACKEND: TfbGraphicsBackend = TfbGraphicsBackend {
    preprocess: ffi::rust_gfx_preprocess,
    postprocess: ffi::rust_gfx_postprocess,
    upload_transition_screen: ffi::rust_gfx_upload_transition_screen,
    screen: ffi::rust_gfx_screen,
    color: ffi::rust_gfx_color,
};
pub static mut GRAPHICS_BACKEND: *mut TfbGraphicsBackend = ptr::null_mut();

use crate::graphics::dcqueue_abi::Init_DrawCommandQueue;

use crate::graphics::dcqueue_abi::Uninit_DrawCommandQueue;

use crate::automation::input_ffi::rust_automation_present_callback;

use crate::automation::input_ffi::rust_automation_presented_frame;

use crate::graphics::cmap_abi::GetFadeAmount;

extern "C" {
    static mut TransitionAmount: i32;
    static mut TransitionClipRect: crate::graphics::native_abi::rect;
    static mut ScreenWidth: c_int;
    static mut ScreenHeight: c_int;
    static mut ScreenWidthActual: c_int;
    static mut ScreenHeightActual: c_int;
    static mut GraphicsDriver: c_int;
}
pub(crate) unsafe fn refresh_rust_screen_globals() -> bool {
    let mut screens = [ptr::null_mut(); NUM_SCREENS];
    for (i, screen) in screens.iter_mut().enumerate() {
        *screen = ffi::rust_gfx_get_screen_surface(i as c_int);
        if screen.is_null() {
            return false;
        }
    }
    SDL_SCREENS = screens;
    SDL_Screen = screens[0];
    TRANSITION_SCREEN = screens[2];
    format_conv_surf = ffi::rust_gfx_get_format_conv_surf();
    !format_conv_surf.is_null()
}
pub(crate) unsafe fn update_video_globals(
    driver: c_int,
    flags: c_int,
    width: c_int,
    height: c_int,
) {
    GraphicsDriver = driver;
    GfxFlags = flags;
    ScreenWidth = 320;
    ScreenHeight = 240;
    ScreenWidthActual = width;
    ScreenHeightActual = height;
}
pub(crate) unsafe fn clear_rust_screen_globals() {
    SDL_SCREENS = [ptr::null_mut(); NUM_SCREENS];
    SDL_Screen = ptr::null_mut();
    TRANSITION_SCREEN = ptr::null_mut();
    format_conv_surf = ptr::null_mut();
}
/// # Safety
/// Called once on the SDL main thread; renderer must be null or a readable C string.
#[no_mangle]
pub unsafe extern "C" fn TFB_InitGraphics(
    driver: c_int,
    flags: c_int,
    renderer: *const c_char,
    width: c_int,
    height: c_int,
) -> c_int {
    SDL_SCREENS = [ptr::null_mut(); NUM_SCREENS];
    update_video_globals(driver, flags, width, height);
    if ffi::rust_gfx_init(driver, flags, renderer, width, height) != 0 {
        std::process::exit(1);
    }
    assert!(
        refresh_rust_screen_globals(),
        "initialized graphics surfaces"
    );
    GRAPHICS_BACKEND = ptr::addr_of_mut!(RUST_BACKEND);
    if flags & 1 != 0 {
        sdl2_sys::SDL_ShowCursor(0);
    }
    Init_DrawCommandQueue();
    crate::graphics::native_canvas::lifecycle::TFB_DrawCanvas_Initialize();
    0
}
/// # Safety
/// Called on the SDL main thread after all graphics producers have stopped.
#[no_mangle]
pub unsafe extern "C" fn TFB_UninitGraphics() {
    Uninit_DrawCommandQueue();
    ffi::rust_gfx_uninit();
    clear_rust_screen_globals();
}
/// # Safety
/// Called on the SDL main thread with input initialized.
#[no_mangle]
pub unsafe extern "C" fn TFB_ProcessEvents() {
    let mut event = std::mem::MaybeUninit::uninit();
    while sdl2_sys::SDL_PollEvent(event.as_mut_ptr()) > 0 {
        let event = event.assume_init();
        crate::input::process_input_event(&event);
        if event.type_ == sdl2_sys::SDL_EventType::SDL_QUIT as u32 {
            ptr::write_volatile(ptr::addr_of_mut!(QuitPosted), 1);
        } else if event.type_ == sdl2_sys::SDL_EventType::SDL_WINDOWEVENT as u32
            && event.window.event == sdl2_sys::SDL_WindowEventID::SDL_WINDOWEVENT_EXPOSED as u8
        {
            TFB_SwapBuffers(2);
        }
    }
}
static mut SYSTEM_BOX_ACTIVE: bool = false;
static mut SYSTEM_BOX: SDL_Rect = SDL_Rect {
    x: 0,
    y: 0,
    w: 0,
    h: 0,
};
fn sdl_rect(rect: RECT) -> SDL_Rect {
    SDL_Rect {
        x: i32::from(rect.corner.x),
        y: i32::from(rect.corner.y),
        w: i32::from(rect.extent.width),
        h: i32::from(rect.extent.height),
    }
}
/// # Safety
/// Rectangle must be readable on the serialized graphics thread.
#[no_mangle]
pub unsafe extern "C" fn SetSystemRect(rect: *const RECT) {
    SYSTEM_BOX = sdl_rect(*rect);
    SYSTEM_BOX_ACTIVE = true;
}
/// # Safety
/// Called on the serialized graphics thread.
#[no_mangle]
pub unsafe extern "C" fn ClearSystemRect() {
    SYSTEM_BOX_ACTIVE = false;
}
static mut LAST_FADE: c_int = 255;
static mut LAST_TRANSITION: c_int = 255;
/// # Safety
/// Called on the SDL main thread with graphics initialized.
#[no_mangle]
pub unsafe extern "C" fn TFB_SwapBuffers(mut redraw: c_int) {
    if rust_automation_present_callback() != 0 {
        return;
    }
    let fade = GetFadeAmount();
    let transition = ptr::read_volatile(ptr::addr_of!(TransitionAmount));
    if redraw == 0
        && TFB_B_BOX.valid == 0
        && fade == 255
        && transition == 255
        && LAST_FADE == 255
        && LAST_TRANSITION == 255
    {
        return;
    }
    if redraw == 0
        && (fade != 255 || transition != 255 || LAST_FADE != 255 || LAST_TRANSITION != 255)
    {
        redraw = 1;
    }
    LAST_FADE = fade;
    LAST_TRANSITION = transition;
    let backend = &*GRAPHICS_BACKEND;
    (backend.preprocess)(redraw, transition, fade);
    (backend.screen)(0, 255, ptr::null_mut());
    if transition != 255 {
        let mut rect = sdl_rect(TransitionClipRect);
        (backend.screen)(2, (255 - transition) as u8, &mut rect);
    }
    if fade != 255 {
        if fade < 255 {
            (backend.color)(0, 0, 0, (255 - fade) as u8, ptr::null_mut());
        } else {
            (backend.color)(255, 255, 255, (fade - 255) as u8, ptr::null_mut());
        }
    }
    if SYSTEM_BOX_ACTIVE {
        (backend.screen)(0, 255, ptr::addr_of_mut!(SYSTEM_BOX));
    }
    (backend.postprocess)();
    rust_automation_presented_frame();
}
/// # Safety
/// Screen must be a valid screen ID; called on graphics thread.
#[no_mangle]
pub unsafe extern "C" fn TFB_GetScreenCanvas(screen: u32) -> *mut c_void {
    SDL_SCREENS[screen as usize].cast()
}
/// # Safety
/// Called on the SDL main thread with graphics initialized.
#[no_mangle]
pub unsafe extern "C" fn TFB_UploadTransitionScreen() {
    ((*GRAPHICS_BACKEND).upload_transition_screen)();
}
/// # Safety
/// Pointer must be writable and refer to null or an owned SDL surface.
pub unsafe extern "C" fn un_init_screen(screen: *mut *mut SDL_Surface) {
    if !(*screen).is_null() {
        sdl2_sys::SDL_FreeSurface((*screen).cast());
        *screen = ptr::null_mut();
    }
}
