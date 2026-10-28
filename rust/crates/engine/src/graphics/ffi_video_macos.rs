//! Read the Cocoa frame rather than SDL's cached window origin.
use std::ffi::{c_char, c_void};

#[repr(C)]
#[derive(Clone, Copy)]
struct Point {
    x: f64,
    y: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Size {
    width: f64,
    height: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Rect {
    origin: Point,
    size: Size,
}

#[link(name = "objc")]
unsafe extern "C" {
    fn sel_registerName(name: *const c_char) -> *mut c_void;
    #[cfg(target_arch = "aarch64")]
    fn objc_msgSend();
    #[cfg(target_arch = "x86_64")]
    fn objc_msgSend_stret();
}

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGMainDisplayID() -> u32;
    fn CGDisplayBounds(display: u32) -> Rect;
}

pub(super) fn top_left_y(primary_height: f64, bottom: f64, height: f64) -> i32 {
    (primary_height - bottom - height).round() as i32
}

pub(super) fn position(window: &sdl2::video::Window) -> Result<(i32, i32), String> {
    // SAFETY: SDL initializes the tagged WM union for this live window. The
    // Cocoa variant starts with an NSWindow pointer; sdl2-sys's pregenerated
    // bindings expose the platform-neutral 64-byte union storage instead.
    let native_window = unsafe {
        let mut info: sdl2_sys::SDL_SysWMinfo = std::mem::zeroed();
        sdl2_sys::SDL_GetVersion(&mut info.version);
        if sdl2_sys::SDL_GetWindowWMInfo(window.raw(), &mut info) != sdl2_sys::SDL_bool::SDL_TRUE {
            return Err(sdl2::get_error());
        }
        if info.subsystem != sdl2_sys::SDL_SYSWM_TYPE::SDL_SYSWM_COCOA {
            return Err("native proof window is not a Cocoa window".into());
        }
        std::ptr::read(std::ptr::addr_of!(info.info).cast::<*mut c_void>())
    };
    if native_window.is_null() {
        return Err("SDL returned a null Cocoa window".into());
    }
    // SAFETY: SDL owns this NSWindow on the graphics thread. `frame` returns
    // an NSRect in Cocoa points. The typed call follows each platform's
    // Objective-C structure-return ABI.
    let frame = unsafe {
        let selector = sel_registerName(c"frame".as_ptr());
        #[cfg(target_arch = "aarch64")]
        {
            let send: unsafe extern "C" fn(*mut c_void, *mut c_void) -> Rect =
                std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
            send(native_window, selector)
        }
        #[cfg(target_arch = "x86_64")]
        {
            let send: unsafe extern "C" fn(*mut Rect, *mut c_void, *mut c_void) =
                std::mem::transmute(objc_msgSend_stret as unsafe extern "C" fn());
            let mut frame = std::mem::MaybeUninit::uninit();
            send(frame.as_mut_ptr(), native_window, selector);
            frame.assume_init()
        }
    };
    // SAFETY: these CoreGraphics queries return the live primary desktop in
    // points, the same top-left coordinate space used by the OS observer.
    let primary = unsafe { CGDisplayBounds(CGMainDisplayID()) };
    Ok((
        frame.origin.x.round() as i32,
        top_left_y(primary.size.height, frame.origin.y, frame.size.height),
    ))
}
