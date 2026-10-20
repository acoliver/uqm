//! Original C == sealed fixture == Rust, using actual RAM drawables and DrawBatch.
//! Synthetic resource records are inputs, not fake rendering/resource effects.

use sha2::{Digest, Sha256};
use std::cell::RefCell;
use std::ffi::{c_void, CString};
use uqm_rust::c_bindings::global_state as c;
use uqm_rust::comm::locdata::{CPoint, CRect};
use uqm_rust::planets::c_api::{self, CapturedSurface, FORMAT_CONV_SURF, SDL_SCREEN};
use uqm_rust::planets::render::c_api::render_system_topography;
use uqm_rust::planets::render::topography::*;
use uqm_rust::planets::tables::PLANET_ARRAY;

extern "C" {
    fn CreateContextAux(name: *const std::ffi::c_char) -> *mut c_void;
    fn SetContext(context: *mut c_void) -> *mut c_void;
    fn DestroyContext(context: *mut c_void) -> c::BOOLEAN;
    fn SetContextFGFrame(frame: c::FRAME) -> c::FRAME;
    fn SetContextOrigin(point: c::POINT) -> c::POINT;
    fn SetContextClipRect(rect: *mut c::RECT) -> c::BOOLEAN;
    fn SetContextDrawMode(mode: c::DrawMode) -> c::DrawMode;
    fn SetContextForeGroundColor(color: c::Color) -> c::Color;
    fn GetColorMapAddress(map: c::COLORMAP) -> *mut u8;
    fn TFB_DrawCanvas_Initialize();
    fn ReadFramePixelColors(frame: c::FRAME, pixels: *mut c::Color, w: i32, h: i32) -> c::BOOLEAN;
    fn WriteFramePixelColors(
        frame: c::FRAME,
        pixels: *const c::Color,
        w: i32,
        h: i32,
    ) -> c::BOOLEAN;
}

#[repr(C)]
struct Operations {
    frame: unsafe extern "C" fn(c::FRAME) -> c::FRAME,
    origin: unsafe extern "C" fn(c::POINT) -> c::POINT,
    get_clip: unsafe extern "C" fn(*mut c::RECT) -> c::BOOLEAN,
    set_clip: unsafe extern "C" fn(*mut c::RECT) -> c::BOOLEAN,
    colors: unsafe extern "C" fn(c::COLORMAP) -> *mut u8,
    batch: unsafe extern "C" fn(*mut c::PRIMITIVE, u32, u8),
    table: *const c::PlanetFrame,
    system: *mut c::SOLARSYS_STATE,
}

type Original = unsafe extern "C" fn(*const Operations, c::FRAME, *mut i8, i32, i32);

struct Oracle {
    handle: *mut c_void,
    run: Original,
}
impl Oracle {
    fn open() -> Option<Self> {
        std::env::var_os("UQM_RENDER_TOPOGRAPHY_ORACLE").map(|path| unsafe {
            let path = CString::new(path.as_encoded_bytes()).unwrap();
            let handle = libc::dlopen(path.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL);
            assert!(
                !handle.is_null(),
                "load compiled original C oracle: {:?}",
                std::ffi::CStr::from_ptr(libc::dlerror())
            );
            let symbol = libc::dlsym(handle, c"original_render_topography".as_ptr());
            assert!(!symbol.is_null());
            Self {
                handle,
                run: std::mem::transmute::<*mut c_void, Original>(symbol),
            }
        })
    }
}
impl Drop for Oracle {
    fn drop(&mut self) {
        assert_eq!(unsafe { libc::dlclose(self.handle) }, 0);
    }
}

thread_local! { static TRACE: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) }; }
fn record(bytes: &[u8]) {
    TRACE.with(|t| t.borrow_mut().extend_from_slice(bytes));
}
fn take_trace() -> Vec<u8> {
    TRACE.with(|t| std::mem::take(&mut *t.borrow_mut()))
}
unsafe extern "C" fn frame(f: c::FRAME) -> c::FRAME {
    record(&[1]);
    SetContextFGFrame(f)
}
unsafe extern "C" fn origin(p: c::POINT) -> c::POINT {
    record(&[2]);
    record(&p.x.to_le_bytes());
    record(&p.y.to_le_bytes());
    SetContextOrigin(p)
}
unsafe extern "C" fn get_clip(r: *mut c::RECT) -> c::BOOLEAN {
    record(&[3]);
    c::GetContextClipRect(r)
}
unsafe extern "C" fn set_clip(r: *mut c::RECT) -> c::BOOLEAN {
    record(&[4, u8::from(!r.is_null())]);
    if !r.is_null() {
        for v in [
            (*r).corner.x,
            (*r).corner.y,
            (*r).extent.width,
            (*r).extent.height,
        ] {
            record(&v.to_le_bytes());
        }
    }
    SetContextClipRect(r)
}
unsafe extern "C" fn batch(prims: *mut c::PRIMITIVE, links: u32, flags: u8) {
    record(&[5, flags]);
    record(&links.to_le_bytes());
    let mut index = links as u16;
    while index != u16::MAX {
        let p = &*prims.add(usize::from(index));
        record(&index.to_le_bytes());
        record(&p.Links.to_le_bytes());
        record(&[p.Type]);
        record(&p.Object.Point.x.to_le_bytes());
        record(&p.Object.Point.y.to_le_bytes());
        record(&[p.color.r, p.color.g, p.color.b, p.color.a]);
        index = (p.Links >> 16) as u16;
    }
    record(&u16::MAX.to_le_bytes());
    c::DrawBatch(prims, links, flags);
}

// For the core comparison these callbacks observe and forward every effect to
// the real context renderer, the same operations used by the compiled C body.
struct Observed;
impl TopographyGfx for Observed {
    type Frame = c::FRAME;
    fn set_frame(&mut self, f: c::FRAME) -> c::FRAME {
        unsafe { frame(f) }
    }
    fn set_origin(&mut self, p: CPoint) -> CPoint {
        let p = unsafe { origin(c::POINT { x: p.x, y: p.y }) };
        CPoint { x: p.x, y: p.y }
    }
    fn clip(&mut self) -> CRect {
        let mut r: c::RECT = unsafe { std::mem::zeroed() };
        unsafe { get_clip(&mut r) };
        CRect {
            corner: CPoint {
                x: r.corner.x,
                y: r.corner.y,
            },
            width: r.extent.width,
            height: r.extent.height,
        }
    }
    fn set_clip(&mut self, r: Option<CRect>) {
        let mut r = r.map(|r| c::RECT {
            corner: c::POINT {
                x: r.corner.x,
                y: r.corner.y,
            },
            extent: c::EXTENT {
                width: r.width,
                height: r.height,
            },
        });
        unsafe { set_clip(r.as_mut().map_or(std::ptr::null_mut(), |r| r)) };
    }
    fn batch(&mut self, points: &[TopographyPoint; 64], first: usize) {
        let mut prims = uqm_rust::planets::render::c_api::topography_primitives(points);
        unsafe { batch(prims.as_mut_ptr(), first as u32, 0) };
    }
}

// strintrn.h's owned-by-caller resource entry; the real getters read it.
#[repr(C)]
struct Entry {
    data: *mut u8,
    length: i32,
    index: i32,
    parent: *mut c_void,
}
impl Entry {
    fn new(data: &mut [u8]) -> Self {
        Self {
            data: data.as_mut_ptr(),
            length: data.len() as i32,
            index: 0,
            parent: std::ptr::null_mut(),
        }
    }
    fn handle(&mut self) -> c::STRING {
        (self as *mut Self).cast()
    }
}

struct Ram {
    context: *mut c_void,
    old_context: *mut c_void,
    previous: c::FRAME,
    target: Option<CapturedSurface>,
    old_frame: Option<CapturedSurface>,
    width: i32,
    height: i32,
    _format: sdl2::surface::Surface<'static>,
    _screen: sdl2::surface::Surface<'static>,
}
impl Ram {
    unsafe fn new(w: i32, h: i32) -> Self {
        assert!(FORMAT_CONV_SURF.is_null() && SDL_SCREEN.is_null());
        let format =
            sdl2::surface::Surface::new(1, 1, sdl2::pixels::PixelFormatEnum::RGBA8888).unwrap();
        let screen =
            sdl2::surface::Surface::new(1, 1, sdl2::pixels::PixelFormatEnum::RGBX8888).unwrap();
        FORMAT_CONV_SURF = format.raw();
        SDL_SCREEN = screen.raw();
        TFB_DrawCanvas_Initialize();
        c_api::Init_DrawCommandQueue();
        let context = CreateContextAux(c"topography parity".as_ptr());
        assert!(!context.is_null());
        let old_context = SetContext(context);
        let target = CapturedSurface::pixmap(w as i16, h as i16, 1);
        let old_frame = CapturedSurface::pixmap(11, 9, 1);
        assert!(!target.as_frame().is_null() && !old_frame.as_frame().is_null());
        let previous = old_frame.as_frame();
        SetContextFGFrame(previous);
        Self {
            context,
            old_context,
            previous,
            target: Some(target),
            old_frame: Some(old_frame),
            width: w,
            height: h,
            _format: format,
            _screen: screen,
        }
    }
    fn target(&self) -> c::FRAME {
        self.target.as_ref().unwrap().as_frame()
    }
    unsafe fn reset(&self, mode: c::DrawMode, explicit_clip: bool) {
        for (frame, w, h) in [
            (self.target(), self.width, self.height),
            (self.previous, 11, 9),
        ] {
            let pixels = vec![
                c::Color {
                    r: 11,
                    g: 23,
                    b: 37,
                    a: 255
                };
                (w * h) as usize
            ];
            assert_ne!(WriteFramePixelColors(frame, pixels.as_ptr(), w, h), 0);
        }
        (*self.target()).HotSpot = c::POINT { x: 7, y: -9 };
        (*self.previous).HotSpot = c::POINT { x: -3, y: 4 };
        SetContextFGFrame(self.previous);
        let mut clip = c::RECT {
            corner: c::POINT { x: 2, y: 1 },
            extent: c::EXTENT {
                width: 3,
                height: 2,
            },
        };
        SetContextClipRect(if explicit_clip {
            &mut clip
        } else {
            std::ptr::null_mut()
        });
        SetContextDrawMode(mode);
        SetContextForeGroundColor(c::Color {
            r: 31,
            g: 61,
            b: 91,
            a: 101,
        });
        take_trace();
    }
    unsafe fn result(&self, mode: c::DrawMode) -> Vec<u8> {
        assert_eq!(c::GetContextFGFrame(), self.previous);
        assert_eq!(SetContext(self.context), self.context);
        assert_eq!(
            ((*self.target()).HotSpot.x, (*self.target()).HotSpot.y),
            (7, -9)
        );
        assert_eq!(
            ((*self.previous).HotSpot.x, (*self.previous).HotSpot.y),
            (-3, 4)
        );
        let actual_mode = c::GetContextDrawMode();
        assert_eq!(
            (actual_mode.kind, actual_mode.factor),
            (mode.kind, mode.factor)
        );
        let color = c::GetContextForeGroundColor();
        assert_eq!([color.r, color.g, color.b, color.a], [31, 61, 91, 101]);
        let mut clip: c::RECT = std::mem::zeroed();
        let defined = c::GetContextClipRect(&mut clip);
        let mut result = defined.to_le_bytes().to_vec();
        for v in [
            clip.corner.x,
            clip.corner.y,
            clip.extent.width,
            clip.extent.height,
        ] {
            result.extend_from_slice(&v.to_le_bytes());
        }
        for (frame, w, h) in [
            (self.target(), self.width, self.height),
            (self.previous, 11, 9),
        ] {
            let mut pixels = vec![
                c::Color {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 0
                };
                (w * h) as usize
            ];
            assert_ne!(ReadFramePixelColors(frame, pixels.as_mut_ptr(), w, h), 0);
            for p in pixels {
                result.extend_from_slice(&[p.r, p.g, p.b, p.a]);
            }
        }
        result
    }
}
impl Drop for Ram {
    fn drop(&mut self) {
        unsafe {
            SetContextFGFrame(std::ptr::null_mut());
            drop(self.target.take());
            drop(self.old_frame.take());
            c_api::TFB_FlushGraphicsEx(1);
            SetContext(self.old_context);
            assert_ne!(DestroyContext(self.context), 0);
            c_api::Uninit_DrawCommandQueue();
            FORMAT_CONV_SURF = std::ptr::null_mut();
            SDL_SCREEN = std::ptr::null_mut();
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Case {
    planet: usize,
    width: i32,
    height: i32,
    palette: u8,
    mode: u8,
    factor: i16,
    clip: bool,
    missing: bool,
    shielded: bool,
    base: Option<i16>,
    cropped: bool,
}
fn cases() -> Vec<Case> {
    let base = Case {
        planet: 0,
        width: 17,
        height: 5,
        palette: 0,
        mode: 0,
        factor: 255,
        clip: true,
        missing: false,
        shielded: false,
        base: None,
        cropped: false,
    };
    let mut cases: Vec<_> = (0..59)
        .map(|planet| Case {
            planet,
            width: 256,
            height: 1,
            palette: (planet % 3) as u8,
            shielded: planet % 2 == 0,
            ..base
        })
        .collect();
    for (w, h) in [
        (0, 0),
        (-1, 3),
        (3, -1),
        (1, 1),
        (63, 1),
        (64, 1),
        (65, 1),
        (127, 1),
        (128, 1),
        (129, 1),
        (7, 19),
        (242, 75),
        (968, 300),
    ] {
        cases.push(Case {
            width: w,
            height: h,
            ..base
        });
    }
    for palette in 0..3 {
        for (mode, factor) in [(0, 0), (1, 127), (1, -127), (2, 101)] {
            for clip in [false, true] {
                cases.push(Case {
                    width: 256,
                    height: 1,
                    palette,
                    mode,
                    factor,
                    clip,
                    ..base
                });
            }
        }
    }
    cases.push(Case {
        cropped: true,
        ..base
    });
    for missing in [false, true] {
        for clip in [false, true] {
            cases.push(Case {
                missing,
                clip,
                ..base
            });
        }
    }
    for elevation in [i16::MIN, -129, -1, 0, 128, 255, i16::MAX] {
        cases.push(Case {
            width: 256,
            height: 1,
            base: Some(elevation),
            ..base
        });
    }
    cases
}

pub fn full_c_fixture_core_and_host_parity() {
    host_errors_preserve_real_context_and_pixels();
    let oracle = Oracle::open();
    let fixture = include_bytes!("../fixtures/planets/render-topography.bin");
    let mut offset = 0;
    assert_eq!(
        format!("{:x}", Sha256::digest(fixture)),
        "8ca6a84e393a64f0b390a81e43d6b992f619908841549ae1822766c6e7529496"
    );
    let cases = cases();
    assert_eq!(cases.len(), 108);
    let system_rng = unsafe { uqm_rust::math::RandomContext_GetSeed(super::SysGenRNG) };
    uqm_rust::math::TFB_SeedRandom(90210);
    let mut independent = uqm_rust::math::RandomContext::with_seed(90210);
    for (index, case) in cases.iter().enumerate() {
        unsafe {
            let ram = if case.cropped {
                Ram::new(7, 3)
            } else {
                Ram::new(case.width.max(1) + 3, case.height.max(1) + 2)
            };
            let mut table = PLANET_ARRAY.0;
            if let Some(base) = case.base {
                table[case.planet].base_elevation = base;
            }
            let planet = table[case.planet];
            let mut colors = vec![0, 255];
            for i in 0..256 {
                colors.extend_from_slice(&[
                    (i * 73) as u8,
                    (i * 151 + 63) as u8,
                    (i * 37 + 64) as u8,
                ]);
            }
            let mut translation = vec![0; 262];
            for (i, byte) in translation[6..].iter_mut().enumerate() {
                *byte = match case.palette {
                    0 => i as u8,
                    1 => (255 - i) as u8,
                    _ => (i % 32 + 16) as u8,
                };
            }
            if case.palette == 2 {
                colors[0] = 16;
                colors[1] = 47;
            }
            let mut xlat_entry = Entry::new(&mut translation);
            let mut color_entry = Entry::new(&mut colors);
            let mut system: c::SOLARSYS_STATE = std::mem::zeroed();
            let mut desc: c::PLANET_DESC = std::mem::zeroed();
            desc.data_index = case.planet as u8 | if case.shielded { 0x80 } else { 0 };
            system.pOrbitalDesc = &mut desc;
            system.XlatRef = if case.missing {
                std::ptr::null_mut()
            } else {
                xlat_entry.handle()
            };
            system.XlatPtr = translation.as_ptr().cast();
            system.OrbitalCMap = color_entry.handle();
            let count = (case.width.max(0) * case.height.max(0)) as usize;
            let mut depth: Vec<i8> = (0..count + 32)
                .map(|i| {
                    if case.width == 256 && case.height == 1 {
                        i as i8
                    } else {
                        (i * 73 + i / 17 * 29) as i8
                    }
                })
                .collect();
            let before = depth.clone();
            let color_before = colors.clone();
            let xlat_before = translation.clone();
            let mode = c::DrawMode {
                kind: case.mode,
                factor: case.factor,
            };
            let ops = Operations {
                frame,
                origin,
                get_clip,
                set_clip,
                colors: GetColorMapAddress,
                batch,
                table: table.as_ptr(),
                system: &mut system,
            };
            let palette = (!case.missing).then_some(TopographyPalette {
                translation: translation[6..].try_into().unwrap(),
                colors: &colors,
                planet_type: planet.Type,
                base_elevation: planet.base_elevation,
            });
            ram.reset(mode, case.clip);
            render_topography(
                &mut Observed,
                ram.target(),
                &depth,
                case.width,
                case.height,
                palette,
            )
            .unwrap();
            let actual = ram.result(mode);
            let trace = take_trace();
            let mut record = (actual.len() as u32).to_le_bytes().to_vec();
            record.extend_from_slice(&actual);
            record.extend_from_slice(&(trace.len() as u32).to_le_bytes());
            record.extend_from_slice(&trace);
            if let Some(oracle) = &oracle {
                ram.reset(mode, case.clip);
                (oracle.run)(
                    &ops,
                    ram.target(),
                    depth.as_mut_ptr(),
                    case.width,
                    case.height,
                );
                let expected = ram.result(mode);
                let original_trace = take_trace();
                assert_eq!(expected, actual, "C pixels/state case {index}: {case:?}");
                assert_eq!(
                    original_trace, trace,
                    "C primitive/effect order case {index}: {case:?}"
                );
                let mut original = (expected.len() as u32).to_le_bytes().to_vec();
                original.extend_from_slice(&expected);
                original.extend_from_slice(&(original_trace.len() as u32).to_le_bytes());
                original.extend_from_slice(&original_trace);
                assert_eq!(original, record);
            }
            assert_eq!(
                &fixture[offset..offset + record.len()],
                record,
                "sealed original C case {index}: {case:?}"
            );
            offset += record.len();
            if case.base.is_none() {
                ram.reset(mode, case.clip);
                render_system_topography(&system, ram.target(), &depth, case.width, case.height)
                    .unwrap();
                assert_eq!(
                    ram.result(mode),
                    actual,
                    "host adapter case {index}: {case:?}"
                );
            }
            assert_eq!(depth, before);
            assert_eq!(colors, color_before);
            assert_eq!(translation, xlat_before);
            assert_eq!(
                uqm_rust::math::RandomContext_GetSeed(super::SysGenRNG),
                system_rng
            );
            assert_eq!(uqm_rust::math::TFB_Random(), independent.next_random());
        }
    }
    assert_eq!(offset, fixture.len());
    println!("RenderTopography: {} full RGBA/state/primitive records, {} bytes, original fixture {:x}; live C={}", cases.len(), offset, Sha256::digest(fixture), oracle.is_some());
}

fn host_errors_preserve_real_context_and_pixels() {
    unsafe {
        let ram = Ram::new(9, 7);
        let mode = c::DrawMode {
            kind: 2,
            factor: 101,
        };
        ram.reset(mode, true);
        let before = ram.result(mode);
        let mut xlat = vec![0; 262];
        let mut colors = vec![0, 0, 61, 62, 63];
        let mut xlat_entry = Entry::new(&mut xlat);
        let mut color_entry = Entry::new(&mut colors);
        let mut system: c::SOLARSYS_STATE = std::mem::zeroed();
        let mut desc: c::PLANET_DESC = std::mem::zeroed();
        system.pOrbitalDesc = &mut desc;
        system.XlatRef = xlat_entry.handle();
        system.XlatPtr = xlat.as_ptr().cast();
        system.OrbitalCMap = color_entry.handle();
        for error in [
            TopographyError::PlanetIndex,
            TopographyError::TranslationLength,
            TopographyError::DepthLength,
        ] {
            desc.data_index = if error == TopographyError::PlanetIndex {
                127
            } else {
                0
            };
            xlat_entry.length = if error == TopographyError::TranslationLength {
                261
            } else {
                262
            };
            system.pOrbitalDesc = &mut desc;
            system.XlatRef = xlat_entry.handle();
            assert_eq!(
                render_system_topography(&system, ram.target(), &[], 1, 1),
                Err(error)
            );
            assert_eq!(ram.result(mode), before);
        }
    }
}
