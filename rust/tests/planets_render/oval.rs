//! Offscreen coverage of retained-context oval drawing against original C fixtures.
//! Uses real RAM drawables and C graphics dependencies, never a video driver.

use sha2::{Digest, Sha256};
use std::fmt::Write;
use std::io::Read;
use uqm_engine::graphics::context::LoadedContext;
use uqm_engine::graphics::drawable::{ContextFrame, FrameId};
use uqm_engine::graphics::menu;
use uqm_engine::graphics::surface::CapturedSurface;
use uqm_engine::test_support::native_probe;
use uqm_modes::starbase::ids::RadarLayout;

struct Offscreen {
    context: Option<LoadedContext>,
    previous_context: Option<uqm_engine::graphics::context::ContextId>,
    surface: Option<CapturedSurface>,
    frame: FrameId,
    width: i32,
    height: i32,
    _graphics: native_probe::DrawQueue,
}

impl Offscreen {
    fn new(screen: (i32, i32)) -> Self {
        let layout = RadarLayout::for_screen(screen.0, screen.1);
        let graphics = native_probe::DrawQueue::init();
        let context = LoadedContext::create(c"oval parity");
        let previous_context = menu::select(Some(context.context()));
        let surface = CapturedSurface::pixmap(
            layout.sis_screen_width as i16,
            layout.sis_screen_height as i16,
            1,
        );
        let frame = surface.first_frame();
        assert!(!frame.is_null());
        menu::set_frame(frame);
        Self {
            context: Some(context),
            previous_context,
            surface: Some(surface),
            frame,
            width: layout.sis_screen_width,
            height: layout.sis_screen_height,
            _graphics: graphics,
        }
    }

    fn context(&self) -> uqm_engine::graphics::context::ContextId {
        self.context
            .as_ref()
            .unwrap_or_else(|| panic!("oval fixture context"))
            .context()
    }

    fn assert_context(&self, alpha: u8) {
        assert_eq!(
            menu::select(Some(self.context())),
            Some(self.context()),
            "oval changed context"
        );
        let target = ContextFrame::retain(self.frame);
        assert_eq!(
            uqm_engine::graphics::context::ContextLease::retain(self.context())
                .target()
                .frame(),
            target.frame()
        );
        let color = native_probe::context_foreground(self.context());
        assert_eq!([color.r, color.g, color.b, color.a], [71, 131, 197, alpha]);
        let (clip, _) = native_probe::selected_clip();
        assert_eq!(
            (
                clip.corner.x,
                clip.corner.y,
                clip.extent.width,
                clip.extent.height
            ),
            (0, 0, self.width as i16, self.height as i16)
        );
        let mode = native_probe::selected_draw_mode();
        assert_eq!((mode.kind, mode.factor), (0, 255));
        let hot = self.frame.hot_spot();
        assert_eq!((hot.x, hot.y), (0, 0));
    }

    fn reset(&self) {
        let color = uqm_engine::graphics::geometry::Color {
            r: 11,
            g: 23,
            b: 37,
            a: 255,
        };
        let pixels = vec![color; (self.width * self.height) as usize];
        assert!(native_probe::write_pixels(
            self.frame,
            &pixels,
            self.width,
            self.height
        ));
    }

    fn pixels(&self) -> Vec<[u8; 4]> {
        native_probe::read_pixels(self.frame, self.width, self.height)
            .unwrap_or_else(|| panic!("oval fixture frame"))
            .into_iter()
            .map(|p| [p.r, p.g, p.b, p.a])
            .collect()
    }
}

impl Drop for Offscreen {
    fn drop(&mut self) {
        menu::set_frame(FrameId::NULL);
        drop(self.surface.take());
        uqm_engine::graphics::state::flush_ex(true);
        menu::select(self.previous_context);
        drop(self.context.take());
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

pub fn retained_ovals_match_original_c_full_offscreen_buffers(
    system_rng: &uqm_engine::math::RandomContext,
) {
    let seed_before = system_rng.seed();
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
                menu::foreground(uqm_engine::graphics::geometry::Color {
                    r: 71,
                    g: 131,
                    b: 197,
                    a: alpha,
                });
                for mode in [Some(0), Some(1), Some(7), Some(255), None] {
                    let c_rect = uqm_engine::graphics::geometry::Rect {
                        corner: uqm_engine::graphics::geometry::Point { x, y },
                        extent: uqm_engine::graphics::geometry::Extent { width, height },
                    };
                    canvas.reset();
                    canvas.assert_context(alpha);
                    let context =
                        uqm_engine::graphics::context::ContextLease::retain(canvas.context());
                    let rectangle = uqm_modes::comm::locdata::CRect {
                        corner: uqm_modes::comm::locdata::CPoint { x, y },
                        width,
                        height,
                    };
                    let mut renderer = uqm_modes::planets::render::c_api::NativeOvalGfx::new(
                        &context, screen.0, screen.1,
                    );
                    if let Some(off) = mode {
                        uqm_modes::planets::render::oval::draw_oval(&rectangle, off, &mut renderer);
                    } else {
                        uqm_modes::planets::render::oval::draw_filled_oval(
                            &rectangle,
                            &mut renderer,
                        );
                    }
                    canvas.assert_context(alpha);
                    let actual = canvas.pixels();
                    if (x, y, width, height) == (10, 20, 5, 5) {
                        assert!(
                            actual.iter().any(|p| *p != [11, 23, 37, 255]),
                            "retained Rust oval must draw into the RAM frame"
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
                        assert_eq!(bytes, expected, "retained renderer differs from original C: screen={screen:?} case={case} alpha={alpha} mode={mode:?}");
                    }
                }
            }
        }
    }
    assert_eq!(
        system_rng.seed(),
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
