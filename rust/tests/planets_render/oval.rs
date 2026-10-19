//! Offscreen differential coverage of the original linked `oval.c` and Rust.
//! Uses real RAM drawables and C graphics dependencies, never a video driver.

use std::ffi::c_void;
use std::fmt::Write;

use sha2::{Digest, Sha256};
use uqm_rust::c_bindings::global_state as c;
use uqm_rust::comm::locdata::{CPoint, CRect};
use uqm_rust::planets::c_api::{self, CapturedSurface, FORMAT_CONV_SURF, SDL_SCREEN};
use uqm_rust::planets::render::c_api::CffiOvalGfx;
use uqm_rust::planets::render::oval::{draw_filled_oval, draw_oval};
use uqm_rust::starbase::ids::RadarLayout;

extern "C" {
    static mut ScreenWidth: i32;
    static mut ScreenHeight: i32;
    // The linked-test compile profile defines DEBUG, so gfxlib.h takes a name.
    fn CreateContextAux(name: *const std::ffi::c_char) -> *mut c_void;
    fn SetContext(context: *mut c_void) -> *mut c_void;
    fn DestroyContext(context: *mut c_void) -> c::BOOLEAN;
    fn SetContextFGFrame(frame: *mut c::FRAME_DESC) -> *mut c::FRAME_DESC;
    fn ReadFramePixelColors(
        frame: *mut c::FRAME_DESC,
        pixels: *mut c::Color,
        width: i32,
        height: i32,
    ) -> c::BOOLEAN;
    fn WriteFramePixelColors(
        frame: *mut c::FRAME_DESC,
        pixels: *const c::Color,
        width: i32,
        height: i32,
    ) -> c::BOOLEAN;
    fn SetContextForeGroundColor(color: c::Color) -> c::Color;
    fn TFB_DrawCanvas_Initialize();
    fn DrawOval(rect: *mut c::RECT, off: u8);
    fn DrawFilledOval(rect: *mut c::RECT);
}

struct Offscreen {
    context: *mut c_void,
    previous_context: *mut c_void,
    surface: Option<CapturedSurface>,
    frame: *mut c::FRAME_DESC,
    width: i32,
    height: i32,
    previous_screen: (i32, i32),
    _format: sdl2::surface::Surface<'static>,
    _screen_format: sdl2::surface::Surface<'static>,
}

impl Offscreen {
    fn new(screen: (i32, i32)) -> Self {
        let layout = RadarLayout::for_screen(screen.0, screen.1);
        let format =
            sdl2::surface::Surface::new(1, 1, sdl2::pixels::PixelFormatEnum::RGBA8888).unwrap();
        let screen_format =
            sdl2::surface::Surface::new(1, 1, sdl2::pixels::PixelFormatEnum::RGBX8888).unwrap();
        unsafe {
            assert!(
                FORMAT_CONV_SURF.is_null() && SDL_SCREEN.is_null(),
                "test must not replace active graphics"
            );
            FORMAT_CONV_SURF = format.raw();
            SDL_SCREEN = screen_format.raw();
            let previous_screen = (ScreenWidth, ScreenHeight);
            ScreenWidth = screen.0;
            ScreenHeight = screen.1;
            TFB_DrawCanvas_Initialize();
            c_api::Init_DrawCommandQueue();
            let context = CreateContextAux(c"oval parity".as_ptr());
            assert!(!context.is_null());
            let previous_context = SetContext(context);
            let surface = CapturedSurface::pixmap(
                layout.sis_screen_width as i16,
                layout.sis_screen_height as i16,
                1,
            );
            let frame = surface.as_frame();
            assert!(!frame.is_null());
            SetContextFGFrame(frame);
            Self {
                context,
                previous_context,
                surface: Some(surface),
                frame,
                width: layout.sis_screen_width,
                height: layout.sis_screen_height,
                previous_screen,
                _format: format,
                _screen_format: screen_format,
            }
        }
    }

    fn reset(&self) {
        let color = c::Color {
            r: 11,
            g: 23,
            b: 37,
            a: 255,
        };
        let pixels = vec![color; (self.width * self.height) as usize];
        assert_ne!(
            unsafe { WriteFramePixelColors(self.frame, pixels.as_ptr(), self.width, self.height) },
            0
        );
    }

    fn pixels(&self) -> Vec<[u8; 4]> {
        let mut pixels = vec![
            c::Color {
                r: 0,
                g: 0,
                b: 0,
                a: 0
            };
            (self.width * self.height) as usize
        ];
        assert_ne!(
            unsafe {
                ReadFramePixelColors(self.frame, pixels.as_mut_ptr(), self.width, self.height)
            },
            0
        );
        pixels.into_iter().map(|p| [p.r, p.g, p.b, p.a]).collect()
    }
}

impl Drop for Offscreen {
    fn drop(&mut self) {
        unsafe {
            SetContextFGFrame(std::ptr::null_mut());
            drop(self.surface.take());
            c_api::TFB_FlushGraphicsEx(1);
            SetContext(self.previous_context);
            assert_ne!(DestroyContext(self.context), 0);
            c_api::Uninit_DrawCommandQueue();
            FORMAT_CONV_SURF = std::ptr::null_mut();
            SDL_SCREEN = std::ptr::null_mut();
            ScreenWidth = self.previous_screen.0;
            ScreenHeight = self.previous_screen.1;
        }
    }
}

fn rects(width: i16, height: i16) -> Vec<[i16; 4]> {
    let mut cases = vec![
        [5, 7, 0, 0],
        [5, 7, -8, 9],
        [5, 7, 9, -8],
        [5, 7, 1, 9],
        [5, 7, 9, 1],
        [5, 7, 2, 2],
        [10, 20, 5, 5],
        [10, 20, 6, 6],
        [10, 20, 32, 17],
        [10, 20, 17, 32],
        [0, 0, width, height],
        [-10, 20, 31, 21],
        [width - 10, 20, 31, 21],
        [20, -10, 31, 21],
        [20, height - 10, 31, 21],
        [-20, -20, 41, 41],
        [width - 20, -20, 41, 41],
        [-20, height - 20, 41, 41],
        [width - 20, height - 20, 41, 41],
        [-100, -100, 20, 20],
        [width + 10, height + 10, 20, 20],
        [-width, -height, width * 3, height * 3],
        [i16::MAX - 1, 7, 8, 5],
        [i16::MIN, 7, -1, 5],
        [7, i16::MAX - 1, 5, 8],
        [7, i16::MIN, 5, -1],
    ];
    for size in 2..=17 {
        cases.push([20, 20, size, 19 - size]);
    }
    cases
}

pub fn original_c_and_rust_ovals_match_full_offscreen_buffers() {
    let seed_before = unsafe { uqm_rust::math::RandomContext_GetSeed(super::SysGenRNG) };
    let mut fixture = String::new();
    let mut compared_bytes = 0usize;
    for screen in [(320, 240), (640, 480)] {
        let canvas = Offscreen::new(screen);
        for (case, [x, y, width, height]) in rects(canvas.width as i16, canvas.height as i16)
            .into_iter()
            .enumerate()
        {
            let rect = CRect {
                corner: CPoint { x, y },
                width,
                height,
            };
            for alpha in [255, 101] {
                unsafe {
                    SetContextForeGroundColor(c::Color {
                        r: 71,
                        g: 131,
                        b: 197,
                        a: alpha,
                    });
                }
                for mode in [Some(0), Some(1), Some(7), Some(255), None] {
                    let mut c_rect = c::RECT {
                        corner: c::POINT { x, y },
                        extent: c::EXTENT { width, height },
                    };
                    canvas.reset();
                    unsafe {
                        if let Some(off) = mode {
                            DrawOval(&mut c_rect, off);
                        } else {
                            DrawFilledOval(&mut c_rect);
                        }
                    }
                    let expected = canvas.pixels();
                    if (x, y, width, height) == (10, 20, 5, 5) {
                        assert!(
                            expected.iter().any(|p| *p != [11, 23, 37, 255]),
                            "original C must draw into the RAM frame"
                        );
                    }
                    assert_eq!(
                        (
                            c_rect.corner.x,
                            c_rect.corner.y,
                            c_rect.extent.width,
                            c_rect.extent.height
                        ),
                        (x, y, width, height)
                    );
                    let bytes = expected.as_flattened();
                    compared_bytes += bytes.len();
                    writeln!(fixture, "{}x{} case{case} {x},{y},{width},{height} alpha{alpha} {mode:?} bytes{} {:x}", screen.0, screen.1, bytes.len(), Sha256::digest(bytes)).unwrap();
                    canvas.reset();
                    let mut gfx = unsafe { CffiOvalGfx::current(screen.0, screen.1) };
                    if let Some(off) = mode {
                        draw_oval(&rect, off, &mut gfx);
                    } else {
                        draw_filled_oval(&rect, &mut gfx);
                    }
                    let actual = canvas.pixels();
                    if let Some(index) = actual.iter().zip(&expected).position(|(a, b)| a != b) {
                        panic!("screen={screen:?} case={case} rect={x},{y},{width},{height} alpha={alpha} mode={mode:?} pixel={index} C={:?} Rust={:?}", expected[index], actual[index]);
                    }
                }
            }
        }
    }
    assert_eq!(
        unsafe { uqm_rust::math::RandomContext_GetSeed(super::SysGenRNG) },
        seed_before,
        "ovals do not consume system RNG"
    );
    let golden = include_str!("../fixtures/planets/oval.txt");
    assert_eq!(
        format!("{:x}", Sha256::digest(golden.as_bytes())),
        "bc270ce794ea6cc7fc48df48f96df2500969fcd07b0f0e7b3b5f1d51307fb212",
        "immutable original-C oval fixture"
    );
    assert_eq!(fixture.lines().count(), 840);
    assert_eq!(compared_bytes, 533_211_840);
    assert_eq!(
        fixture, golden,
        "original linked C differs from the retained capture"
    );
    println!("Planet ovals: 840 original-C draws match the immutable fixture and Rust; all {compared_bytes} RGBA readback bytes compared, rectangle inputs and system RNG preserved");
}
