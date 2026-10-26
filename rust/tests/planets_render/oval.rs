//! Offscreen coverage of the exported Rust oval ABI against original C fixtures.
//! Uses real RAM drawables and C graphics dependencies, never a video driver.

use std::ffi::c_void;
use std::fmt::Write;
use std::io::Read;

use sha2::{Digest, Sha256};
use uqm_rust::c_bindings::global_state as c;
use uqm_rust::planets::c_api::{self, CapturedSurface, FORMAT_CONV_SURF, SDL_SCREEN};
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
}

struct Offscreen {
    context: *mut c_void,
    previous_context: *mut c_void,
    surface: Option<CapturedSurface>,
    frame: *mut c::FRAME_DESC,
    width: i32,
    height: i32,
    previous_screen: (i32, i32),
    screen: (i32, i32),
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
            FORMAT_CONV_SURF = format.raw().cast();
            SDL_SCREEN = screen_format.raw().cast();
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
                screen,
                _format: format,
                _screen_format: screen_format,
            }
        }
    }

    fn assert_context(&self, alpha: u8) {
        unsafe {
            assert_eq!(
                SetContext(self.context),
                self.context,
                "oval changed context"
            );
            assert_eq!(c::GetContextFGFrame(), self.frame);
            let color = c::GetContextForeGroundColor();
            assert_eq!([color.r, color.g, color.b, color.a], [71, 131, 197, alpha]);
            let mut clip: c::RECT = std::mem::zeroed();
            c::GetContextClipRect(&mut clip);
            assert_eq!(
                (
                    clip.corner.x,
                    clip.corner.y,
                    clip.extent.width,
                    clip.extent.height
                ),
                (0, 0, self.width as i16, self.height as i16)
            );
            let mode = c::GetContextDrawMode();
            assert_eq!((mode.kind, mode.factor), (0, 255));
            assert_eq!((ScreenWidth, ScreenHeight), self.screen);
            assert_eq!(((*self.frame).HotSpot.x, (*self.frame).HotSpot.y), (0, 0));
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

pub fn exported_ovals_match_original_c_full_offscreen_buffers() {
    let seed_before = unsafe { (*super::SysGenRNG).seed() };
    // Standard tests require every full-buffer digest. A supplied original-C
    // capture adds a streaming byte-for-byte comparison without checking in or
    // duplicating the large capture.
    let mut raw = std::env::var_os("UQM_OVAL_ORIGINAL_C_PIXELS")
        .map(|path| std::fs::File::open(path).expect("read retained original-C pixels"));
    let mut raw_digest = Sha256::new();
    let mut fixture = String::new();
    let mut compared_bytes = 0usize;
    for screen in [(320, 240), (640, 480)] {
        let canvas = Offscreen::new(screen);
        for (case, [x, y, width, height]) in rects(canvas.width as i16, canvas.height as i16)
            .into_iter()
            .enumerate()
        {
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
                    canvas.assert_context(alpha);
                    unsafe {
                        if let Some(off) = mode {
                            c::DrawOval(&mut c_rect, off);
                        } else {
                            c::DrawFilledOval(&mut c_rect);
                        }
                    }
                    canvas.assert_context(alpha);
                    let actual = canvas.pixels();
                    if (x, y, width, height) == (10, 20, 5, 5) {
                        assert!(
                            actual.iter().any(|p| *p != [11, 23, 37, 255]),
                            "exported Rust oval must draw into the RAM frame"
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
                    let bytes = actual.as_flattened();
                    compared_bytes += bytes.len();
                    writeln!(fixture, "{}x{} case{case} {x},{y},{width},{height} alpha{alpha} {mode:?} bytes{} {:x}", screen.0, screen.1, bytes.len(), Sha256::digest(bytes)).unwrap();
                    if let Some(raw) = raw.as_mut() {
                        let mut expected = vec![0; bytes.len()];
                        raw.read_exact(&mut expected)
                            .expect("complete original-C buffer");
                        raw_digest.update(&expected);
                        assert_eq!(bytes, expected, "exported ABI differs from original C: screen={screen:?} case={case} alpha={alpha} mode={mode:?}");
                    }
                }
            }
        }
    }
    assert_eq!(
        unsafe { (*super::SysGenRNG).seed() },
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
        "exported Rust ABI differs from the retained original-C capture"
    );
    if let Some(mut raw) = raw {
        assert_eq!(raw.read(&mut [0]).unwrap(), 0, "no extra original-C bytes");
        assert_eq!(
            format!("{:x}", raw_digest.finalize()),
            "2ef36aee9679faafb011be5e915bb5634637b6bd8ef7a5d1f6cd5091c34cc310"
        );
        println!("Planet ovals: byte-for-byte comparison with all 533211840 retained original-C bytes passed");
    }
    println!("Planet ovals: 840 exported Rust ABI draws match the immutable original-C fixture; all {compared_bytes} RGBA readback bytes hashed, rectangle inputs, context state and system RNG preserved");
}
