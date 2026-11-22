//! Sealed original-C fixture == Rust, using actual RAM drawables and point batches.
//! Synthetic resource records are inputs, not fake rendering/resource effects.

use sha2::{Digest, Sha256};
use std::cell::RefCell;
use uqm_engine::graphics::context::{ContextLease, LoadedContext};
use uqm_engine::graphics::drawable::{ContextFrame, FrameId};
use uqm_engine::graphics::geometry::{Color, DrawMode, Extent, Point, Rect};
use uqm_engine::graphics::menu;
use uqm_engine::graphics::surface::CapturedSurface;
use uqm_engine::test_support::native_probe;
use uqm_modes::comm::locdata::{CPoint, CRect};
use uqm_modes::planets::render::c_api::render_native_topography;
use uqm_modes::planets::render::topography::*;
use uqm_modes::planets::sphere::palette::SurfacePalette;
use uqm_modes::planets::tables::PLANET_ARRAY;

thread_local! { static TRACE: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) }; }
fn record(bytes: &[u8]) {
    TRACE.with(|t| t.borrow_mut().extend_from_slice(bytes));
}
fn take_trace() -> Vec<u8> {
    TRACE.with(|t| std::mem::take(&mut *t.borrow_mut()))
}
fn frame(f: FrameId) -> FrameId {
    record(&[1]);
    menu::set_frame_returning(f)
}
fn origin(p: Point) -> Point {
    record(&[2]);
    record(&p.x.to_le_bytes());
    record(&p.y.to_le_bytes());
    menu::set_origin(p)
}
fn get_clip() -> (Rect, bool) {
    record(&[3]);
    native_probe::selected_clip()
}
fn set_clip(r: Option<Rect>) {
    record(&[4, u8::from(r.is_some())]);
    if let Some(r) = r {
        for v in [r.corner.x, r.corner.y, r.extent.width, r.extent.height] {
            record(&v.to_le_bytes());
        }
    }
    menu::set_clip_or_clear(r);
}
/// Record the C `PRIMITIVE` chain the original batch walked: from `first`
/// forward, each point linking to the next and the last to the sentinel.
fn batch(points: &[TopographyPoint; 64], first: usize) {
    record(&[5, 0]);
    record(&(first as u32).to_le_bytes());
    for (index, source) in points.iter().enumerate().skip(first) {
        let next = if index + 1 < points.len() {
            index as u16 + 1
        } else {
            u16::MAX
        };
        let links = u32::from(u16::MAX) | (u32::from(next) << 16);
        record(&(index as u16).to_le_bytes());
        record(&links.to_le_bytes());
        record(&[0]);
        record(&source.point.x.to_le_bytes());
        record(&source.point.y.to_le_bytes());
        let c = source.color;
        record(&[c.r, c.g, c.b, c.a]);
    }
    record(&u16::MAX.to_le_bytes());
    let selected = menu::selected();
    let context = ContextLease::require(selected);
    context.draw_preserving_scale(&mut uqm_modes::planets::render::c_api::topography_batch(
        points, first,
    ));
}

struct Observed;
impl TopographyGfx for Observed {
    type Frame = FrameId;
    fn set_frame(&mut self, f: FrameId) -> FrameId {
        frame(f)
    }
    fn set_origin(&mut self, p: CPoint) -> CPoint {
        let p = origin(Point { x: p.x, y: p.y });
        CPoint { x: p.x, y: p.y }
    }
    fn clip(&mut self) -> CRect {
        let (r, _) = get_clip();
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
        set_clip(r.map(|r| Rect {
            corner: Point {
                x: r.corner.x,
                y: r.corner.y,
            },
            extent: Extent {
                width: r.width,
                height: r.height,
            },
        }));
    }
    fn batch(&mut self, points: &[TopographyPoint; 64], first: usize) {
        batch(points, first);
    }
}

struct Ram {
    context: Option<LoadedContext>,
    old_context: Option<uqm_engine::graphics::context::ContextId>,
    previous: FrameId,
    target: Option<CapturedSurface>,
    old_frame: Option<CapturedSurface>,
    width: i32,
    height: i32,
    _graphics: native_probe::DrawQueue,
}
impl Ram {
    fn new(w: i32, h: i32) -> Self {
        let graphics = native_probe::DrawQueue::init();
        let context = LoadedContext::create(c"topography parity");
        let old_context = menu::select(Some(context.context()));
        let target = CapturedSurface::pixmap(w as i16, h as i16, 1);
        let old_frame = CapturedSurface::pixmap(11, 9, 1);
        assert!(!target.first_frame().is_null() && !old_frame.first_frame().is_null());
        let previous = old_frame.first_frame();
        menu::set_frame(previous);
        Self {
            context: Some(context),
            old_context,
            previous,
            target: Some(target),
            old_frame: Some(old_frame),
            width: w,
            height: h,
            _graphics: graphics,
        }
    }
    fn context(&self) -> ContextLease {
        self.context
            .as_ref()
            .unwrap_or_else(|| panic!("context"))
            .lease()
    }
    fn target(&self) -> FrameId {
        self.target
            .as_ref()
            .unwrap_or_else(|| panic!("target"))
            .first_frame()
    }
    fn reset(&self, mode: DrawMode, explicit_clip: bool) {
        for (frame, w, h) in [
            (self.target(), self.width, self.height),
            (self.previous, 11, 9),
        ] {
            let pixels = vec![Color::rgba(11, 23, 37, 255); (w * h) as usize];
            assert!(native_probe::write_pixels(frame, &pixels, w, h));
        }
        self.target().set_hot_spot(Point { x: 7, y: -9 });
        self.previous.set_hot_spot(Point { x: -3, y: 4 });
        menu::set_frame(self.previous);
        let clip = Rect {
            corner: Point { x: 2, y: 1 },
            extent: Extent {
                width: 3,
                height: 2,
            },
        };
        menu::set_clip_or_clear(explicit_clip.then_some(clip));
        menu::draw_mode(mode);
        menu::foreground(Color::rgba(31, 61, 91, 101));
        take_trace();
    }
    fn result(&self, mode: DrawMode) -> Vec<u8> {
        let context = self
            .context
            .as_ref()
            .unwrap_or_else(|| panic!("context"))
            .context();
        assert_eq!(self.context().target().frame(), self.previous);
        assert_eq!(menu::select(Some(context)), Some(context));
        let hot = self.target().hot_spot();
        assert_eq!((hot.x, hot.y), (7, -9));
        let hot = self.previous.hot_spot();
        assert_eq!((hot.x, hot.y), (-3, 4));
        let actual_mode = native_probe::selected_draw_mode();
        assert_eq!(
            (actual_mode.kind, actual_mode.factor),
            (mode.kind, mode.factor)
        );
        let color = native_probe::context_foreground(context);
        assert_eq!([color.r, color.g, color.b, color.a], [31, 61, 91, 101]);
        let (clip, defined) = native_probe::selected_clip();
        let mut result = u32::from(defined).to_le_bytes().to_vec();
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
            for p in native_probe::read_pixels(frame, w, h).unwrap_or_else(|| panic!("frame")) {
                result.extend_from_slice(&[p.r, p.g, p.b, p.a]);
            }
        }
        result
    }
}
impl Drop for Ram {
    fn drop(&mut self) {
        menu::set_frame(FrameId::NULL);
        drop(self.target.take());
        drop(self.old_frame.take());
        uqm_engine::graphics::state::flush_ex(true);
        menu::select(self.old_context);
        drop(self.context.take());
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

pub fn full_c_fixture_core_and_host_parity(system_rng: &uqm_engine::math::RandomContext) {
    host_errors_preserve_real_context_and_pixels();
    let fixture = include_bytes!("../fixtures/planets/render-topography.bin");
    let mut offset = 0;
    assert_eq!(
        format!("{:x}", Sha256::digest(fixture)),
        "8ca6a84e393a64f0b390a81e43d6b992f619908841549ae1822766c6e7529496"
    );
    let cases = cases();
    assert_eq!(cases.len(), 108);
    let system_seed = system_rng.seed();
    uqm_engine::math::seed_random(90210);
    let mut independent = uqm_engine::math::RandomContext::with_seed(90210);
    for (index, case) in cases.iter().enumerate() {
        {
            let ram = if case.cropped {
                Ram::new(7, 3)
            } else {
                Ram::new(case.width.max(1) + 3, case.height.max(1) + 2)
            };
            let mut table = PLANET_ARRAY;
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
            let desc = uqm_engine::world::planet::PlanetDesc {
                data_index: case.planet as u8 | if case.shielded { 0x80 } else { 0 },
                ..Default::default()
            };
            let count = (case.width.max(0) * case.height.max(0)) as usize;
            let depth: Vec<i8> = (0..count + 32)
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
            let mode = uqm_engine::graphics::geometry::DrawMode {
                kind: case.mode,
                factor: case.factor,
            };
            let palette = (!case.missing).then_some(TopographyPalette {
                translation: translation[6..]
                    .try_into()
                    .unwrap_or_else(|error| panic!("translation table holds the palette: {error}")),
                colors: &colors,
                planet_type: planet.type_,
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
            .unwrap_or_else(|error| panic!("render topography: {error:?}"));
            let actual = ram.result(mode);
            let trace = take_trace();
            let mut record = (actual.len() as u32).to_le_bytes().to_vec();
            record.extend_from_slice(&actual);
            record.extend_from_slice(&(trace.len() as u32).to_le_bytes());
            record.extend_from_slice(&trace);
            assert_eq!(
                &fixture[offset..offset + record.len()],
                record,
                "sealed original C case {index}: {case:?}"
            );
            offset += record.len();
            if case.base.is_none() {
                ram.reset(mode, case.clip);
                let owned = if case.missing {
                    None
                } else {
                    Some(
                        SurfacePalette::from_bytes(
                            desc.data_index & !128,
                            &translation,
                            colors.clone(),
                        )
                        .unwrap_or_else(|error| panic!("valid fixture palette: {error:?}")),
                    )
                };
                render_native_topography(
                    &ram.context(),
                    ContextFrame::retain(ram.target()),
                    &depth,
                    case.width,
                    case.height,
                    owned.as_ref().map(SurfacePalette::view),
                )
                .unwrap_or_else(|error| panic!("render native topography: {error:?}"));
                assert_eq!(
                    ram.result(mode),
                    actual,
                    "host adapter case {index}: {case:?}"
                );
            }
            assert_eq!(depth, before);
            assert_eq!(colors, color_before);
            assert_eq!(translation, xlat_before);
            assert_eq!(system_rng.seed(), system_seed);
            assert_eq!(uqm_engine::math::random(), independent.next_random());
        }
    }
    assert_eq!(offset, fixture.len());
    println!(
        "RenderTopography: {} full RGBA/state/primitive records, {} bytes, original fixture {:x}",
        cases.len(),
        offset,
        Sha256::digest(fixture)
    );
}

fn host_errors_preserve_real_context_and_pixels() {
    {
        let ram = Ram::new(9, 7);
        let mode = uqm_engine::graphics::geometry::DrawMode {
            kind: 2,
            factor: 101,
        };
        ram.reset(mode, true);
        let before = ram.result(mode);
        let xlat = vec![0; 262];
        let colors = vec![0, 0, 61, 62, 63];
        for error in [
            TopographyError::PlanetIndex,
            TopographyError::TranslationLength,
            TopographyError::DepthLength,
        ] {
            let index = if error == TopographyError::PlanetIndex {
                127
            } else {
                0
            };
            let length = if error == TopographyError::TranslationLength {
                261
            } else {
                262
            };
            let result = SurfacePalette::from_bytes(index, &xlat[..length], colors.clone())
                .and_then(|palette| {
                    render_native_topography(
                        &ram.context(),
                        ContextFrame::retain(ram.target()),
                        &[],
                        1,
                        1,
                        Some(palette.view()),
                    )
                });
            assert_eq!(result, Err(error));
            assert_eq!(ram.result(mode), before);
        }
    }
}
