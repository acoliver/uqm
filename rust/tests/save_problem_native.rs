//! The save-error popup restores its pixels and context after an observed wait.
use std::{cell::Cell, rc::Rc};
use uqm_rust::engine::battle::Cx;
use uqm_rust::engine::input::Frame;
use uqm_rust::engine::input::InputRuntime;
use uqm_rust::engine::observe::CampaignView;
use uqm_rust::engine::observe::CheckpointView;
use uqm_rust::engine::observe::Injected;
use uqm_rust::engine::observe::InputView;
use uqm_rust::engine::observe::Io;
use uqm_rust::engine::observe::Observer;
use uqm_rust::engine::observe::Presented;
use uqm_rust::engine::test_support::native_probe;
use uqm_rust::engine::test_support::native_probe::frame_pixel as GetFramePixel;
use uqm_rust::engine::Game;
use uqm_rust::graphics::context::{ContextLease, LoadedContext};
use uqm_rust::graphics::drawable::{FrameId, LoadedDrawable};
use uqm_rust::graphics::{
    geometry::Color, geometry::Extent, geometry::Point, geometry::Rect,
    menu::clear as ClearDrawable, menu::select as SetContext, present::GraphicsOptions,
};

#[derive(Clone)]
struct WaitObserver {
    waits: Rc<Cell<usize>>,
    polls: Rc<Cell<usize>>,
    frame: FrameId,
}

impl Observer for WaitObserver {
    fn input_admitted(
        &mut self,
        view: &InputView,
        cx: &mut uqm_rust::engine::battle::Cx<'_>,
    ) -> Injected {
        let campaign = CampaignView::for_input(*view, || CampaignView::capture(cx));
        assert_eq!(*view, InputView::Nested);
        assert_eq!(
            self.polls.get(),
            2 * (self.waits.get() + 1),
            "release and press waits each poll before admission"
        );
        assert!(campaign.is_some());
        // SAFETY: the fixture retains this canvas until the popup finishes.
        let pixel = GetFramePixel(self.frame, Point { x: 118, y: 110 });
        assert_ne!(
            (pixel.r, pixel.g, pixel.b),
            (9, 17, 25),
            "popup must be drawn before its input wait"
        );
        self.waits.set(self.waits.get() + 1);
        Injected {
            stop: true,
            seed: None,
            planet_side: None,
            comm_response: None,
        }
    }
    fn checkpoint(&mut self, _: &CheckpointView<'_>) {}
    fn presented(&mut self, _: &mut Presented) {}
}

struct Runtime(WaitObserver, uqm_rust::engine::input::InputSession);

impl InputRuntime for Runtime {
    fn input(&self) -> &uqm_rust::engine::input::InputSession {
        &self.1
    }

    fn observer(&self) -> Box<dyn Observer> {
        Box::new(self.0.clone())
    }
    fn yield_poll(&self, _: &mut uqm_engine::battle::Cx<'_>) {
        panic!("unexpected unthrottled pacing");
    }
    fn sleep(&self, _: &mut uqm_engine::battle::Cx<'_>, _: u32) {
        panic!("unexpected pacing");
    }
    fn sleep_until(&self, _: &mut uqm_engine::battle::Cx<'_>, _: u32) {
        panic!("unexpected pacing");
    }
    fn flush(&self) {
        self.input().clear_menu_state();
    }
    fn confirm_exit(&self, _: &mut uqm_engine::battle::Cx<'_>) -> bool {
        panic!("unexpected confirmation");
    }

    fn update(&self, _cx: &mut Cx<'_>) {
        self.0.polls.set(self.0.polls.get() + 1);
    }
    fn run(&self, _: &mut Cx<'_>, _: &mut Io<'_>, _: bool, _: &mut dyn Frame) {
        panic!("the popup uses the nested wait, not a second input driver");
    }
    fn battle_input(&self, _: &uqm_rust::engine::battle::input::InputContext) -> u8 {
        panic!("popup must not request battle input");
    }
}

#[test]
#[serial_test::serial]
fn save_failure_restores_the_drawable_after_text_and_empty_resource_messages() {
    std::env::set_var("SDL_VIDEODRIVER", "dummy");
    std::env::set_var("SDL_AUDIODRIVER", "dummy");
    let fixture_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("tmp");
    std::fs::create_dir_all(&fixture_root).unwrap();
    let directory = tempfile::tempdir_in(fixture_root).unwrap();
    std::fs::create_dir(directory.path().join("font")).unwrap();
    let mut png = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png, 1, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&[255; 4])
            .unwrap();
    }
    std::fs::write(directory.path().join("font/0041.png"), png).unwrap();
    // SAFETY: this process owns SDL, the game, and every native fixture handle.
    unsafe {
        let options = GraphicsOptions::default();
        uqm_rust::graphics::present::init_graphics(1, 0, options, 640, 480);
        let context_id = LoadedContext::create(c"save-error fixture").into_id();
        let context = Some(context_id);
        let previous = Some(LoadedContext::create(c"previous context").into_id());
        let drawable = LoadedDrawable::create(8, 320, 240, 1).unwrap();
        let frame = drawable.first_frame();
        SetContext(context);
        uqm_rust::graphics::menu::set_frame(frame);
        uqm_rust::graphics::menu::background(Color {
            r: 9,
            g: 17,
            b: 25,
            a: 255,
        });
        ClearDrawable();
        uqm_rust::graphics::state::flush();
        let content = uqm_engine::test_support::native_session::ContentMount::new(directory.path());
        let font_owner = uqm_rust::graphics::font::LoadedFont::load_file(c"font")
            .unwrap_or_else(|| panic!("one-glyph font fixture did not load"));
        let font = Some(font_owner.font());
        drop(content);
        native_probe::with_font_mut(font_owner.font(), |font| {
            font.leading = 2;
            font.leading_width = 2;
            let glyph = font.glyph_mut(65).unwrap_or_else(|| panic!("glyph A"));
            glyph.hot_spot.y = 0;
            assert_eq!((glyph.extent.width, glyph.extent.height), (1, 1));
            assert_eq!((glyph.disp.width, glyph.disp.height), (2, 2));
            assert_eq!(glyph.data[0], 255);
        });
        let mut strings = vec![Vec::new(); 457];
        strings[456] = c"A".to_bytes_with_nul().to_vec();
        let context_lease = uqm_rust::graphics::context::ContextLease::require(context);
        let font_lease = uqm_rust::graphics::font::FontLease::retain(font_owner.font());
        let waits = Rc::new(Cell::new(0));
        let polls = Rc::new(Cell::new(0));
        let mut game = Game::new();
        uqm_rust::battle::services::install(&mut game.kernel);
        game.kernel.space_context = context;
        game.kernel.star_con_font = font;
        game.kernel
            .install_game_text(uqm_rust::strings::owned_table::LoadedTextTable::fixture(
                strings,
            ));
        game.kernel.install_input_runtime(Rc::new(Runtime(
            WaitObserver {
                waits: waits.clone(),
                polls: polls.clone(),
                frame,
            },
            uqm_rust::engine::input::InputSession::default(),
        )));
        {
            {
                use uqm_rust::engine::host as source;
                let process_one = || source::dispatch().process_one_alarm(source::now_ms());
                let wait = || source::dispatch().time_before_next_alarm_ms(source::now_ms());
                source::install(true);
                let region = |x| Rect {
                    corner: Point { x, y: 1 },
                    extent: Extent {
                        width: 4,
                        height: 4,
                    },
                };
                let configure = |x| {
                    let mut highlight = context_lease.highlight(region(x));
                    highlight.set_merge_factors(3, 2, 2);
                    highlight.set_speed(0, 840 / 16, 0, 840 / 16);
                    highlight.set_frame_time(840 / 16);
                    highlight
                };
                let mut manual = configure(10);
                manual.start();
                let mut scheduled = configure(1).schedule();
                assert_eq!(wait(), 61);
                source::advance_to(51);
                assert!(!process_one());
                source::advance_to(52);
                assert!(process_one());
                assert!(!process_one());
                manual.process();
                uqm_rust::graphics::state::flush();
                let pixel = |x| {
                    let p = GetFramePixel(frame, Point { x, y: 2 });
                    (p.r, p.g, p.b, p.a)
                };
                assert_eq!(pixel(2), pixel(11));
                assert_eq!(wait(), 62);
                scheduled.pause();
                source::advance_to(104);
                assert!(!process_one());
                scheduled.resume();
                assert!(process_one());
                manual.process();
                uqm_rust::graphics::state::flush();
                assert_eq!(pixel(2), pixel(11));
                scheduled.pre_update();
                manual.pre_update();
                uqm_rust::graphics::state::flush();
                assert_eq!(pixel(2), (9, 17, 25, 255));
                assert_eq!(pixel(2), pixel(11));
                scheduled.post_update();
                manual.post_update();
                uqm_rust::graphics::state::flush();
                assert_eq!(pixel(2), pixel(11));
                drop(scheduled);
                drop(manual);
                source::advance_to(1_000);
                assert!(!process_one());
                uqm_rust::graphics::state::flush();
                assert_eq!(pixel(2), (9, 17, 25, 255));
                let clip = Rect {
                    corner: Point { x: 20, y: 10 },
                    extent: Extent {
                        width: 100,
                        height: 100,
                    },
                };
                let flash_kernel = uqm_rust::engine::Kernel::new();
                let other_kernel = uqm_rust::engine::Kernel::new();
                let mut reference = configure(30);
                reference.start();
                uqm_rust::ui::sis::set_flash_rect(
                    &flash_kernel.sis_highlight,
                    &context_lease,
                    clip,
                    Some(region(1)),
                );
                uqm_rust::ui::sis::set_flash_rect(
                    &other_kernel.sis_highlight,
                    &context_lease,
                    clip,
                    None,
                );
                uqm_rust::graphics::state::flush();
                let translated = GetFramePixel(frame, Point { x: 22, y: 12 });
                assert_eq!((translated.r, translated.g, translated.b), (9, 17, 25));
                assert_eq!(wait(), 0);
                assert!(process_one());
                reference.process();
                flash_kernel.sis_highlight.pause_flash();
                source::advance_to(1_052);
                assert!(!process_one());
                flash_kernel.sis_highlight.continue_flash();
                assert!(process_one());
                reference.process();
                uqm_rust::graphics::state::flush();
                let translated = GetFramePixel(frame, Point { x: 22, y: 12 });
                let original = GetFramePixel(frame, Point { x: 31, y: 2 });
                assert_eq!((translated.r, translated.g, translated.b), (13, 25, 37));
                assert_eq!(
                    (translated.r, translated.g, translated.b),
                    (original.r, original.g, original.b)
                );
                uqm_rust::ui::sis::set_flash_rect(
                    &flash_kernel.sis_highlight,
                    &context_lease,
                    clip,
                    None,
                );
                drop(reference);
                source::advance_to(2_000);
                assert!(!process_one());
                uqm_rust::graphics::state::flush();
                let restored = GetFramePixel(frame, Point { x: 22, y: 12 });
                assert_eq!((restored.r, restored.g, restored.b), (9, 17, 25));
                source::dispatch().clear();
            }
            for count in [0, 1, u16::MAX] {
                for offset in [0, 10] {
                    context_lease.fill_rectangle(
                        Color {
                            r: 9,
                            g: 17,
                            b: 25,
                            a: 255,
                        },
                        Rect {
                            corner: Point { x: 0, y: 0 },
                            extent: Extent {
                                width: 65,
                                height: 77,
                            },
                        },
                    );
                    uqm_rust::graphics::menu::select_font(None);
                    uqm_rust::ui::shipstat::draw_ship_name_text(
                        &context_lease,
                        &font_lease,
                        c"AA",
                        count,
                        offset,
                    );
                    assert!(
                        native_probe::context_font(context_id).is_none(),
                        "title must restore its font"
                    );
                    let foreground = native_probe::context_foreground(context_id);
                    assert_eq!((foreground.r, foreground.g, foreground.b), (0, 0, 0));
                    uqm_rust::graphics::state::flush();
                    for y in 0..77 {
                        for x in 0..65 {
                            let ink = match count {
                                0 => false,
                                1 => x == 32,
                                _ => x == 31 || x == 33,
                            };
                            let expected = if ink && y == offset + 9 {
                                (0, 0, 0)
                            } else if ink && y == offset + 10 {
                                (132, 132, 132)
                            } else {
                                (9, 17, 25)
                            };
                            let pixel = GetFramePixel(frame, Point { x, y });
                            assert_eq!(
                                (pixel.r, pixel.g, pixel.b),
                                expected,
                                "title count={count} offset={offset} ({x},{y})"
                            );
                        }
                    }
                }
            }
            uqm_rust::ui::shipstat::outline_ship_status(&context_lease, 0);
            uqm_rust::ui::shipstat::clear_ship_status(&context_lease, 0);
            uqm_rust::graphics::state::flush();
            for y in 0..66 {
                for x in 0..65 {
                    let shade = if x >= 64 || y == 65 {
                        None
                    } else if y == 0 {
                        Some(0)
                    } else if y == 1 || x < 2 {
                        Some(66)
                    } else if x == 63 || (x == 62 && y >= 3) {
                        Some(132)
                    } else if y == 2 {
                        Some(66)
                    } else {
                        Some(82)
                    };
                    let expected = shade.map_or((9, 17, 25), |shade| (shade, shade, shade));
                    let pixel = GetFramePixel(frame, Point { x, y });
                    assert_eq!((pixel.r, pixel.g, pixel.b), expected, "status ({x},{y})");
                }
            }
            for (max_crew, max_energy, crew_height, energy_height) in [
                (0, 0, 1, 1),
                (1, 1, 3, 3),
                (42, 42, 43, 43),
                (u16::MAX, 254, 43, 255),
            ] {
                for offset in [0, 10] {
                    context_lease.fill_rectangle(
                        Color {
                            r: 9,
                            g: 17,
                            b: 25,
                            a: 255,
                        },
                        Rect {
                            corner: Point { x: 0, y: 0 },
                            extent: Extent {
                                width: 65,
                                height: 77,
                            },
                        },
                    );
                    uqm_rust::ui::shipstat::draw_gauge_frames(
                        &context_lease,
                        max_crew,
                        max_energy,
                        offset,
                    );
                    uqm_rust::graphics::state::flush();
                    let bottom = 55 + offset;
                    let rectangles = [
                        (66, 3, bottom + 1, 9, 1),
                        (66, 51, bottom + 1, 9, 1),
                        (66, 59, bottom + 1 - energy_height, 1, energy_height),
                        (66, 11, bottom + 1 - crew_height, 1, crew_height),
                        (132, 3, bottom - crew_height, 9, 1),
                        (132, 51, bottom - energy_height, 9, 1),
                        (132, 51, bottom - energy_height, 1, energy_height + 1),
                        (132, 3, bottom - crew_height, 1, crew_height + 1),
                        (0, 4, bottom + 1 - crew_height, 7, crew_height),
                        (0, 52, bottom + 1 - energy_height, 7, energy_height),
                    ];
                    for y in 0..77 {
                        for x in 0..65 {
                            let mut expected = (9, 17, 25);
                            for (shade, left, top, width, height) in rectangles {
                                if (left..left + width).contains(&x)
                                    && (top..top + height).contains(&y)
                                {
                                    expected = (shade, shade, shade);
                                }
                            }
                            let pixel = GetFramePixel(frame, Point { x, y });
                            assert_eq!(
                                (pixel.r, pixel.g, pixel.b), expected,
                                "gauge crew={max_crew} energy={max_energy} offset={offset} ({x},{y})"
                            );
                        }
                    }
                }
            }
            context_lease.fill_rectangle(
                Color {
                    r: 9,
                    g: 17,
                    b: 25,
                    a: 255,
                },
                Rect {
                    corner: Point { x: 0, y: 0 },
                    extent: Extent {
                        width: 65,
                        height: 77,
                    },
                },
            );
            {
                let target = context_lease.target();
                // Loaded status stencils have alpha or a color key; an opaque canvas cannot mask a fill.
                let labels =
                    uqm_rust::graphics::drawable::LoadedDrawable::create(8, 1, 1, 6).unwrap();
                let frames = labels.lease();
                for index in 0..6 {
                    context_lease.set_frame(frames.context_frame(index));
                    context_lease.fill_rectangle(
                        Color {
                            r: 30 + index as u8,
                            g: 40 + index as u8,
                            b: 50 + index as u8,
                            a: 255,
                        },
                        Rect {
                            corner: Point { x: 0, y: 0 },
                            extent: Extent {
                                width: 1,
                                height: 1,
                            },
                        },
                    );
                }
                context_lease.set_frame(target);
                uqm_rust::graphics::state::flush();
                for index in 0..6 {
                    let pixel = GetFramePixel(frames.frame(index).unwrap(), Point { x: 0, y: 0 });
                    assert_eq!(
                        (pixel.r, pixel.g, pixel.b, pixel.a),
                        (30 + index as u8, 40 + index as u8, 50 + index as u8, 255),
                        "label source {index}"
                    );
                }
                for pc_menu in [false, true] {
                    for change in [-1, 0, 1] {
                        context_lease.fill_rectangle(
                            Color {
                                r: 9,
                                g: 17,
                                b: 25,
                                a: 255,
                            },
                            Rect {
                                corner: Point { x: 0, y: 0 },
                                extent: Extent {
                                    width: 65,
                                    height: 77,
                                },
                            },
                        );
                        uqm_rust::ui::shipstat::draw_crew_energy_labels(
                            &context_lease,
                            &frames,
                            pc_menu,
                            0,
                            change,
                        );
                        uqm_rust::graphics::state::flush();
                        let index = if pc_menu { 4 } else { 0 };
                        for (x, expected) in [
                            (
                                13,
                                if change == 0 {
                                    (30 + index, 40 + index, 50 + index)
                                } else {
                                    (9, 17, 25)
                                },
                            ),
                            (
                                50,
                                if change >= 0 {
                                    (31 + index, 41 + index, 51 + index)
                                } else {
                                    (255, 255, 82)
                                },
                            ),
                        ] {
                            let pixel = GetFramePixel(frame, Point { x, y: 62 });
                            assert_eq!(
                                (pixel.r, pixel.g, pixel.b),
                                expected,
                                "labels pc={pc_menu} change={change} x={x}"
                            );
                        }
                        let foreground = native_probe::context_foreground(context_id);
                        let expected = if change < 0 {
                            (255, 255, 82)
                        } else {
                            (9, 17, 25)
                        };
                        assert_eq!((foreground.r, foreground.g, foreground.b), expected);
                    }
                }
                {
                    let portrait = LoadedDrawable::create(8, 1, 1, 1).unwrap();
                    let icon = portrait.lease().frame(0).unwrap();
                    let target = context_lease.target();
                    context_lease.set_frame(portrait.lease().context_frame(0));
                    context_lease.fill_rectangle(
                        Color {
                            r: 73,
                            g: 91,
                            b: 109,
                            a: 255,
                        },
                        Rect {
                            corner: Point { x: 0, y: 0 },
                            extent: Extent {
                                width: 1,
                                height: 1,
                            },
                        },
                    );
                    context_lease.set_frame(target);
                    game.with_context(|cx| {
                        let old = (
                            cx.kernel.status_context,
                            cx.kernel.status_frame,
                            cx.kernel.tiny_font,
                        );
                        cx.kernel.status_context = context;
                        cx.kernel.status_frame = frames.frame(0).unwrap();
                        cx.kernel.tiny_font = font;
                        cx.kernel.reset_ship_capacity(0, 1);
                        let id = {
                            let mut queue = cx.kernel.ships(0);
                            let link = queue.alloc().unwrap();
                            let record = &mut *queue
                                .raw(link)
                                .cast::<uqm_engine::battle::record::ShipRecord>();
                            let mut descriptor: Box<uqm_engine::battle::record::RaceDescriptor> =
                                Box::new(std::mem::zeroed());
                            descriptor.ship_info.icons = icon;
                            descriptor.ship_info.max_crew = 3;
                            descriptor.ship_info.max_energy = 4;
                            descriptor.ship_info.crew_level = 2;
                            descriptor.ship_info.energy_level = 3;
                            descriptor.ship_info.ship_cost = 7;
                            record.descriptor = Some(descriptor);
                            record.playerNr = 0;
                            queue.ship_id(link)
                        };
                        SetContext(previous);
                        uqm_rust::ui::shipstat::draw_battle_status(cx, id, None);
                        let selected = native_probe::selected_context();
                        assert_eq!(selected, previous);
                        let info = &cx.desc(id).ship_info;
                        assert_eq!((info.crew_level, info.energy_level), (2, 3));
                        assert_eq!(
                            (info.icons, info.max_crew, info.max_energy, info.ship_cost),
                            (icon, 3, 4, 7)
                        );
                        uqm_rust::graphics::state::flush();
                        let pixel = GetFramePixel(frame, Point { x: 32, y: 31 });
                        assert_eq!((pixel.r, pixel.g, pixel.b), (73, 91, 109));
                        let active_canvas = LoadedDrawable::create(8, 65, 77, 1).unwrap();
                        let active_frame = active_canvas.lease().frame(0).unwrap();
                        let active_context = ContextLease::require(previous);
                        active_context.set_frame(active_canvas.lease().context_frame(0));
                        let gauge_pixel = Point { x: 8, y: 52 };
                        let status_before = GetFramePixel(frame, gauge_pixel);
                        cx.draw_statistics(id, 1, -1);
                        uqm_rust::graphics::state::flush();
                        let active_pixel = GetFramePixel(active_frame, gauge_pixel);
                        assert_eq!(
                            (active_pixel.r, active_pixel.g, active_pixel.b),
                            (0, 165, 0),
                            "statistics must draw to the caller's selected context"
                        );
                        let status_after = GetFramePixel(frame, gauge_pixel);
                        assert_eq!(
                            (status_after.r, status_after.g, status_after.b),
                            (status_before.r, status_before.g, status_before.b)
                        );
                        let selected = native_probe::selected_context();
                        assert_eq!(selected, previous);
                        let info = &cx.desc(id).ship_info;
                        assert_eq!((info.crew_level, info.energy_level), (3, 2));
                        assert_eq!(
                            (info.icons, info.max_crew, info.max_energy, info.ship_cost),
                            (icon, 3, 4, 7)
                        );
                        cx.draw_statistics(id, 0, 0);
                        assert_eq!(
                            (
                                cx.desc(id).ship_info.crew_level,
                                cx.desc(id).ship_info.energy_level
                            ),
                            (3, 2)
                        );
                        cx.kernel.status_context = None;
                        cx.draw_statistics(id, -1, 1);
                        assert_eq!(
                            (
                                cx.desc(id).ship_info.crew_level,
                                cx.desc(id).ship_info.energy_level
                            ),
                            (2, 3)
                        );
                        uqm_rust::ui::shipstat::draw_battle_status(cx, id, None);
                        cx.kernel.reset_ship_capacity(0, 0);
                        (
                            cx.kernel.status_context,
                            cx.kernel.status_frame,
                            cx.kernel.tiny_font,
                        ) = old;
                    });
                }
                for clip_x in [None, Some(3), Some(500)] {
                    context_lease.fill_rectangle(
                        Color {
                            r: 9,
                            g: 17,
                            b: 25,
                            a: 255,
                        },
                        Rect {
                            corner: Point { x: 0, y: 0 },
                            extent: Extent {
                                width: 65,
                                height: 77,
                            },
                        },
                    );
                    let mut ship: uqm_rust::engine::battle::record::SHIP_INFO = std::mem::zeroed();
                    ship.max_crew = 2;
                    ship.max_energy = 2;
                    ship.crew_level = 2;
                    ship.energy_level = 2;
                    let clip = clip_x.map(|x| Rect {
                        corner: Point { x, y: 1 },
                        extent: Extent {
                            width: 55,
                            height: 52,
                        },
                    });
                    let original_clip = context_lease.clip_rect();
                    let original_origin = frame.hot_spot();
                    uqm_rust::graphics::menu::select_font(None);
                    SetContext(previous);
                    uqm_rust::ui::shipstat::StatusGraphics {
                        context: context_lease.clone(),
                        frames: frames.clone(),
                        font: font_lease.clone(),
                        tiny_font: font_lease.clone(),
                        crew_font: Some(font_lease.clone()),
                        pc_menu: false,
                    }
                    .draw(
                        &mut ship,
                        &frames.context_frame(2),
                        0,
                        clip,
                        Some(uqm_rust::ui::shipstat::StatusTitle {
                            name: c"A",
                            name_count: 1,
                            caption: Some((c"A", 1)),
                        }),
                    );
                    let selected = native_probe::selected_context();
                    assert_eq!(selected, previous);
                    assert!(native_probe::context_font(context_id).is_none());
                    assert_eq!((ship.crew_level, ship.energy_level), (2, 2));
                    assert_eq!(
                        (frame.hot_spot().x, frame.hot_spot().y),
                        (original_origin.x, original_origin.y)
                    );
                    let restored = context_lease.clip_rect();
                    assert_eq!(
                        (
                            restored.corner.x,
                            restored.corner.y,
                            restored.extent.width,
                            restored.extent.height
                        ),
                        (
                            original_clip.corner.x,
                            original_clip.corner.y,
                            original_clip.extent.width,
                            original_clip.extent.height
                        )
                    );
                    uqm_rust::graphics::state::flush();
                    // Offscreen native primitives use the clip for origin and intersection,
                    // not a per-pixel scissor; an entirely excluded region draws nothing.
                    for (x, y, color) in [
                        (32, 9, (0, 0, 0)),
                        (32, 10, (132, 132, 132)),
                        (32, 31, (32, 42, 52)),
                        (0, 1, (66, 66, 66)),
                        (32, 58, (0, 0, 0)),
                        (13, 62, (30, 40, 50)),
                        (50, 62, (31, 41, 51)),
                        (8, 54, (0, 165, 0)),
                        (56, 54, (165, 0, 0)),
                    ] {
                        let expected = if clip_x == Some(500) {
                            (9, 17, 25)
                        } else {
                            color
                        };
                        let pixel = GetFramePixel(frame, Point { x, y });
                        assert_eq!(
                            (pixel.r, pixel.g, pixel.b),
                            expected,
                            "panel clip_x={clip_x:?} ({x},{y})"
                        );
                    }
                }
            }
            let mut stats: uqm_rust::engine::battle::record::SHIP_INFO = std::mem::zeroed();
            stats.ship_flags = 1 << 13;
            let update = |stats: &mut uqm_rust::engine::battle::record::SHIP_INFO, crew, energy| {
                uqm_rust::ui::shipstat::update_statistics(
                    Some(&context_lease),
                    false,
                    false,
                    None,
                    stats,
                    0,
                    (crew, energy),
                );
            };
            let assert_pixel = |x, y, expected| {
                uqm_rust::graphics::state::flush();
                let pixel = GetFramePixel(frame, Point { x, y });
                assert_eq!(
                    (pixel.r, pixel.g, pixel.b),
                    expected,
                    "statistics ({x},{y})"
                );
            };
            update(&mut stats, 3, 3);
            assert_eq!((stats.crew_level, stats.energy_level), (3, 3));
            assert_pixel(8, 54, (82, 82, 255));
            assert_pixel(5, 54, (0, 165, 0));
            assert_pixel(8, 52, (0, 165, 0));
            assert_pixel(56, 54, (165, 0, 0));
            assert_pixel(53, 54, (165, 0, 0));
            assert_pixel(56, 52, (165, 0, 0));
            update(&mut stats, -2, -2);
            assert_eq!((stats.crew_level, stats.energy_level), (1, 1));
            for (x, y) in [(5, 54), (8, 52), (53, 54), (56, 52)] {
                assert_pixel(x, y, (0, 0, 0));
            }
            assert_pixel(8, 54, (82, 82, 255));
            assert_pixel(56, 54, (165, 0, 0));
            update(&mut stats, 42, 0);
            assert_eq!(stats.crew_level, 43);
            assert_pixel(8, 54, (0, 165, 0));
            update(&mut stats, -1, 0);
            assert_eq!(stats.crew_level, 42);
            assert_pixel(8, 54, (82, 82, 255));
            update(&mut stats, 1, 0);
            assert_pixel(8, 54, (0, 165, 0));
            stats.ship_flags = 1 << 6;
            stats.crew_level = 0;
            update(&mut stats, 1, 0);
            assert_pixel(8, 54, (82, 82, 82));
            stats.ship_flags = 0;
            stats.crew_level = 60;
            update(&mut stats, 1, 0);
            assert_eq!(stats.crew_level, 61);
            let foreground = native_probe::context_foreground(context_id);
            assert_eq!((foreground.r, foreground.g, foreground.b), (0, 165, 0));
            update(&mut stats, 0, 0);
            let foreground = native_probe::context_foreground(context_id);
            assert_eq!((foreground.r, foreground.g, foreground.b), (0, 165, 0));
            for saving in [true, false] {
                SetContext(previous);
                let saved = uqm_rust::ui::save_problem::render_save_confirmation(
                    &context_lease,
                    &font_lease,
                    c"A",
                    saving,
                );
                assert_eq!(saved.is_some(), saving);
                assert_eq!(SetContext(previous), previous);
                uqm_rust::graphics::state::flush();
                // Recorded from the original ConfirmSaveLoad with the same one-glyph font.
                let rows = [
                    "lllllllll",
                    "lllllllld",
                    "lliiiiidd",
                    "lliiiiidd",
                    "lliiTiidd",
                    "lliiiiidd",
                    "lliiiiidd",
                    "lliiiiidd",
                    "ldddddddd",
                    "ddddddddd",
                ];
                for y in 115..131 {
                    for x in 150..171 {
                        let expected = if (156..165).contains(&x) && (119..129).contains(&y) {
                            let shade =
                                match rows[(y - 119) as usize].as_bytes()[(x - 156) as usize] {
                                    b'l' => 132,
                                    b'd' => 66,
                                    b'i' => 82,
                                    b'T' => 165,
                                    _ => unreachable!(),
                                };
                            (shade, shade, shade)
                        } else {
                            (9, 17, 25)
                        };
                        let pixel = GetFramePixel(frame, Point { x, y });
                        assert_eq!(
                            (pixel.r, pixel.g, pixel.b),
                            expected,
                            "saving={saving}, ({x},{y})"
                        );
                    }
                }
                if let Some(saved) = saved {
                    saved.restore();
                    assert_eq!(SetContext(previous), previous);
                    uqm_rust::graphics::state::flush();
                    for y in 119..129 {
                        for x in 156..165 {
                            let pixel = GetFramePixel(frame, Point { x, y });
                            assert_eq!((pixel.r, pixel.g, pixel.b), (9, 17, 25));
                        }
                    }
                } else {
                    context_lease.fill_rectangle(
                        Color {
                            r: 9,
                            g: 17,
                            b: 25,
                            a: 255,
                        },
                        Rect {
                            corner: Point { x: 0, y: 0 },
                            extent: Extent {
                                width: 320,
                                height: 240,
                            },
                        },
                    );
                }
            }
            for text in [c"A", c""] {
                SetContext(previous);
                game.with_context(|cx| {
                    let mut observer = cx.kernel_mut().input_runtime().observer();
                    let mut io = Io {
                        observer: &mut *observer,
                    };
                    uqm_rust::ui::save_problem::render_save_problem(
                        cx,
                        &mut io,
                        &context_lease,
                        &font_lease,
                        text,
                    );
                });
                assert_eq!(
                    SetContext(previous),
                    previous,
                    "previous context must be restored"
                );
                assert_eq!(native_probe::context_font(context_id), font);
                let foreground = native_probe::context_foreground(context_id);
                assert_eq!((foreground.r, foreground.g, foreground.b), (165, 165, 165));
                uqm_rust::graphics::state::flush();
                for y in 108..120 {
                    for x in 116..128 {
                        let pixel = GetFramePixel(frame, Point { x, y });
                        assert_eq!(
                            (pixel.r, pixel.g, pixel.b),
                            (9, 17, 25),
                            "popup pixels must be restored"
                        );
                    }
                }
            }
        };
        assert_eq!(waits.get(), 2);
        assert_eq!(polls.get(), 4);
        game.kernel.star_con_font = None;
        game.kernel.space_context = None;
        SetContext(None);
        drop(drawable);
        drop(context_lease);
        drop(font_lease);
        drop(context.map(LoadedContext::from_id));
        drop(previous.map(LoadedContext::from_id));
        drop(font_owner);
        uqm_rust::graphics::present::uninit_graphics();
    }
}
