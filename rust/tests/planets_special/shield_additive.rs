//! Direct supported-native parity for the isolated additive core. No window,
//! graphics registry, provider switch or replacement resource is involved.

use sdl2::pixels::PixelFormatEnum;
use sdl2::surface::Surface;
use uqm_rust::graphics::additive::{AdditiveError, Rgb32CanvasMut, Rgb32Format};
use uqm_rust::graphics::dcqueue::{Color, Extent, Point, Rect};

#[repr(C)]
struct NativeColor {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

#[repr(C)]
struct NativeMode {
    kind: u8,
    factor: i16,
}

unsafe extern "C" {
    fn TFB_DrawCanvas_Rect(
        rect: *mut [i16; 4],
        color: NativeColor,
        mode: NativeMode,
        surface: *mut sdl2::sys::SDL_Surface,
    );
    fn renderpixel_for(surface: *mut sdl2::sys::SDL_Surface, kind: i32) -> *const std::ffi::c_void;
}

fn rect(x: i32, y: i32, w: i32, h: i32) -> Rect {
    Rect::new(Point::new(x, y), Extent::new(w, h))
}

pub fn check() {
    let mut compared = 0;
    for layout in [
        PixelFormatEnum::RGBX8888,
        PixelFormatEnum::BGRX8888,
        PixelFormatEnum::RGB888,
        PixelFormatEnum::BGR888,
    ] {
        let mut surface = Surface::new(5, 3, layout).unwrap();
        let raw = surface.raw();
        let format = unsafe { &*(*raw).format };
        let masks = [format.Rmask, format.Gmask, format.Bmask, format.Amask];
        let core_format = Rgb32Format::new(format.BytesPerPixel, masks).unwrap();
        let pitch = surface.pitch() as usize;
        for factor in [-32767, -255, -1, 0, 1, 128, 254, 255, 256, 32767] {
            for color in [Color::new(255, 0, 0, 255), Color::new(17, 129, 250, 0)] {
                surface.with_lock_mut(|pixels| {
                    for (index, byte) in pixels.iter_mut().enumerate() {
                        *byte = index.wrapping_mul(73).wrapping_add(29) as u8;
                    }
                });
                let before = surface.with_lock(|pixels| pixels.to_vec());
                let mut expected = before.clone();
                let clip = sdl2::rect::Rect::new(1, 0, 3, 2);
                surface.set_clip_rect(clip);
                Rgb32CanvasMut::new(&mut expected, 5, 3, pitch, core_format)
                    .unwrap()
                    .fill(rect(-1, -1, 5, 4), rect(1, 0, 3, 2), color, factor)
                    .unwrap();
                unsafe {
                    TFB_DrawCanvas_Rect(
                        &mut [-1, -1, 5, 4],
                        NativeColor {
                            r: color.r,
                            g: color.g,
                            b: color.b,
                            a: color.a,
                        },
                        NativeMode { kind: 1, factor },
                        raw,
                    );
                }
                let actual = surface.with_lock(|pixels| pixels.to_vec());
                assert_eq!(
                    actual, expected,
                    "layout {layout:?}, factor {factor}, color {color:?}"
                );
                compared += 1;
            }
        }
        println!(
            "additive native/core parity: layout={layout:?}, masks={masks:08x?}, pitch={pitch}"
        );
    }
    for layout in [
        PixelFormatEnum::RGBA8888,
        PixelFormatEnum::RGB24,
        PixelFormatEnum::RGB565,
        PixelFormatEnum::Index8,
    ] {
        let mut surface = Surface::new(3, 2, layout).unwrap();
        surface.with_lock_mut(|pixels| pixels.fill(73));
        let raw = surface.raw();
        let f = unsafe { &*(*raw).format };
        assert_eq!(
            Rgb32Format::new(f.BytesPerPixel, [f.Rmask, f.Gmask, f.Bmask, f.Amask]).unwrap_err(),
            AdditiveError::UnsupportedFormat
        );
        assert!(
            unsafe { renderpixel_for(raw, 1) }.is_null(),
            "native rejects {layout:?}"
        );
        if layout == PixelFormatEnum::RGBA8888 {
            println!(
                "intentional rejection probe: additive to RGBA must warn and leave bytes unchanged"
            );
            let before = surface.with_lock(|pixels| pixels.to_vec());
            unsafe {
                TFB_DrawCanvas_Rect(
                    &mut [0, 0, 3, 2],
                    NativeColor {
                        r: 255,
                        g: 0,
                        b: 0,
                        a: 255,
                    },
                    NativeMode {
                        kind: 1,
                        factor: 255,
                    },
                    raw,
                );
            }
            assert_eq!(
                surface.with_lock(|pixels| pixels.to_vec()),
                before,
                "original fixture's RGBA destination is rejected without drawing"
            );
        }
    }
    println!("additive parity: {compared} native/core rectangles; RGBA/24-bit/16-bit/indexed remain explicitly unsupported");
}

pub unsafe fn frame_bytes(frame: *mut uqm_rust::c_bindings::global_state::FRAME_DESC) -> Vec<u8> {
    let surface = (*frame).image.cast::<*mut sdl2::sys::SDL_Surface>().read();
    let mut bytes = vec![0; ((*surface).pitch * (*surface).h) as usize];
    uqm_rust::graphics::sdl_capture::lock_copy_unlock(surface, bytes.as_mut_ptr(), bytes.len())
        .unwrap();
    bytes
}

pub unsafe fn earth_expected(
    frame: *mut uqm_rust::c_bindings::global_state::FRAME_DESC,
) -> Vec<u8> {
    let surface = (*frame).image.cast::<*mut sdl2::sys::SDL_Surface>().read();
    let f = &*(*surface).format;
    assert!(f.palette.is_null());
    assert_eq!(
        (f.BytesPerPixel, f.Rmask, f.Gmask, f.Bmask, f.Amask),
        (4, 0xff000000, 0xff0000, 0xff00, 0),
        "real Earth must use the production non-alpha screen format"
    );
    let format = Rgb32Format::new(f.BytesPerPixel, [f.Rmask, f.Gmask, f.Bmask, f.Amask]).unwrap();
    let mut bytes = frame_bytes(frame);
    let area = rect(0, 0, (*surface).w, (*surface).h);
    Rgb32CanvasMut::new(
        &mut bytes,
        (*surface).w,
        (*surface).h,
        (*surface).pitch as usize,
        format,
    )
    .unwrap()
    .fill(area, area, Color::rgb(255, 0, 0), 255)
    .unwrap();
    bytes
}
