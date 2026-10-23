//! SDL reconfiguration must keep the three live screen surfaces and real window.
use sdl2_sys as sys;
use std::{ffi::c_void, ptr};
use uqm_rust::graphics::{
    ffi,
    sdl::{lifecycle_abi::*, platform_abi::TFB_ReInitGraphics},
};
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
        widget_label_uses_the_shared_context_font_and_pixels();
        TFB_UninitGraphics();
        assert!(ffi::rust_gfx_get_screen_surface(0).is_null());
    }
}

unsafe fn widget_label_uses_the_shared_context_font_and_pixels() {
    use uqm_rust::graphics::{
        context_abi::{CreateContextAux, DestroyContext, SetContext, SetContextForeGroundColor},
        dcqueue_abi::TFB_FlushGraphics,
        drawable_abi::{CreateDrawable, DestroyDrawable, GetFramePixel, SetContextFGFrame},
        font_abi::SetContextFont,
        native_abi::{font_desc, Color, FontPage, TFB_Char, EXTENT, HOT_SPOT, POINT, WIDGET_LABEL},
        pixmap_abi::CaptureDrawable,
        widgets_abi::{Widget_DrawLabel, Widget_SetFont},
    };
    #[cfg(feature = "debug-process")]
    let context = CreateContextAux(c"widget native fixture".as_ptr());
    #[cfg(not(feature = "debug-process"))]
    let context = CreateContextAux();
    assert!(!context.is_null());
    let previous_context = SetContext(context);
    let drawable = CreateDrawable(8, 320, 32, 1);
    assert!(!drawable.is_null());
    let frame = CaptureDrawable(drawable);
    SetContextFGFrame(frame);
    let mut alpha = 255;
    let mut glyph = TFB_Char {
        extent: EXTENT {
            width: 1,
            height: 1,
        },
        disp: EXTENT {
            width: 2,
            height: 2,
        },
        HotSpot: HOT_SPOT { x: 0, y: 0 },
        data: &mut alpha,
        pitch: 1,
    };
    let mut page = FontPage {
        next: ptr::null_mut(),
        pageStart: 0,
        firstChar: 65,
        numChars: 1,
        charDesc: &mut glyph,
    };
    let mut font = font_desc {
        Leading: 2,
        LeadingWidth: 2,
        fontPages: &mut page,
    };
    SetContextFont(&mut font);
    let previous_widget_font = Widget_SetFont(&mut font);
    let original_color = Color {
        r: 9,
        g: 8,
        b: 7,
        a: 255,
    };
    SetContextForeGroundColor(original_color);
    let mut label: WIDGET_LABEL = std::mem::zeroed();
    let mut lines = [c"A".as_ptr(), c"A".as_ptr()];
    label.lines = lines.as_mut_ptr();
    label.line_count = 2;
    Widget_DrawLabel(ptr::addr_of_mut!(label).cast(), 0, 8);
    for y in [8, 16] {
        let pixel = GetFramePixel(frame, POINT { x: 160, y });
        assert_eq!((pixel.r, pixel.g, pixel.b, pixel.a), (255, 255, 255, 255));
    }
    let untouched = GetFramePixel(frame, POINT { x: 159, y: 8 });
    assert_eq!(untouched.a, 0);
    let restored = SetContextForeGroundColor(original_color);
    assert_eq!((restored.r, restored.g, restored.b), (9, 8, 7));
    Widget_SetFont(previous_widget_font);
    SetContextFont(ptr::null_mut());
    DestroyDrawable(drawable);
    DestroyContext(context);
    SetContext(previous_context);
    TFB_FlushGraphics();
}
