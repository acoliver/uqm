//! The save-error popup restores its pixels and context after an observed wait.
use std::{cell::Cell, ptr, rc::Rc};
use uqm_rust::{
    engine::{
        battle::Cx,
        input::{Frame, InputRuntime},
        observe::{CampaignView, CheckpointView, Injected, InputView, Io, Observer, Presented},
        Game,
    },
    graphics::{
        context_abi::*,
        dcqueue_abi::TFB_FlushGraphics,
        drawable_abi::*,
        frame_abi::ClearDrawable,
        native_abi::*,
        pixmap_abi::CaptureDrawable,
        sdl::lifecycle_abi::{GraphicsOptions, TFB_InitGraphics, TFB_UninitGraphics},
    },
    strings::string_table::*,
};

#[derive(Clone)]
struct WaitObserver {
    waits: Rc<Cell<usize>>,
    frame: *mut frame_desc,
}

impl Observer<Option<CampaignView>> for WaitObserver {
    fn input_admitted(&mut self, view: &InputView, campaign: &Option<CampaignView>) -> Injected {
        assert_eq!(*view, InputView::Nested);
        assert!(campaign.is_some());
        // SAFETY: the fixture retains this canvas until the popup finishes.
        let pixel = unsafe { GetFramePixel(self.frame, POINT { x: 118, y: 110 }) };
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

    fn observer(&self) -> Box<dyn Observer<Option<CampaignView>>> {
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
        panic!("the observer must stop the wait before sampling input");
    }
    fn run(&self, _: &mut Cx<'_>, _: &mut Io<'_>, _: bool, _: &mut dyn Frame) {
        panic!("the popup uses the nested wait, not a second input driver");
    }
    fn battle_input(&self, _: &uqm_rust::engine::battle::input::InputContext) -> u8 {
        panic!("popup must not request battle input");
    }
}

#[test]
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
    let host = std::ffi::CString::new(directory.path().to_str().unwrap()).unwrap();
    // SAFETY: this process owns SDL, the game, and every native fixture handle.
    unsafe {
        let options = GraphicsOptions::default();
        assert_eq!(
            TFB_InitGraphics(1, 0, ptr::from_ref(&options).cast(), 640, 480),
            0
        );
        #[cfg(feature = "debug-process")]
        let context = CreateContextAux(c"save-error fixture".as_ptr());
        #[cfg(not(feature = "debug-process"))]
        let context = CreateContextAux();
        #[cfg(feature = "debug-process")]
        let previous = CreateContextAux(c"previous context".as_ptr());
        #[cfg(not(feature = "debug-process"))]
        let previous = CreateContextAux();
        let drawable = CreateDrawable(8, 320, 240, 1);
        let frame = CaptureDrawable(drawable);
        SetContext(context);
        SetContextFGFrame(frame);
        SetContextBackGroundColor(Color {
            r: 9,
            g: 17,
            b: 25,
            a: 255,
        });
        ClearDrawable();
        TFB_FlushGraphics();
        use uqm_rust::io::uio_bridge as uio;
        uio::uio_init();
        let repository = uio::uio_openRepository(0);
        assert!(!uio::uio_mountDir(
            repository,
            c"/".as_ptr(),
            1,
            ptr::null_mut(),
            ptr::null(),
            host.as_ptr(),
            ptr::null_mut(),
            6,
            ptr::null_mut()
        )
        .is_null());
        let content = uio::uio_openDir(repository, c"/".as_ptr(), 0);
        assert!(!content.is_null());
        let previous_content = uqm_rust::mainloop::dir_prep::contentDir;
        uqm_rust::mainloop::dir_prep::contentDir = content.cast();
        let font = uqm_rust::graphics::filegfx::LoadFontFile(c"font".as_ptr());
        assert!(!font.is_null());
        assert!(
            !(*font).fontPages.is_null(),
            "one-glyph font fixture did not load: {:?}",
            std::ffi::CStr::from_ptr(sdl2_sys::SDL_GetError())
        );
        uqm_rust::mainloop::dir_prep::contentDir = previous_content;
        uio::uio_closeDir(content);
        uio::uio_closeRepository(repository);
        uio::uio_unInit();
        (*font).Leading = 2;
        (*font).LeadingWidth = 2;
        let glyph = (*(*font).fontPages).charDesc;
        (*glyph).HotSpot.y = 0;
        assert_eq!(((*glyph).extent.width, (*glyph).extent.height), (1, 1));
        assert_eq!(((*glyph).disp.width, (*glyph).disp.height), (2, 2));
        assert_eq!(*(*glyph).data, 255);
        let strings = AllocStringTable(457, 0);
        let message = (*strings).strings.add(456);
        (*message).data = uqm_rust::memory::rust_hmalloc(2).cast();
        ptr::copy_nonoverlapping(c"A".as_ptr(), (*message).data, 2);
        (*message).length = 1;
        let context_lease = uqm_rust::graphics::context_abi::ContextLease::retain(
            std::ptr::NonNull::new(context).unwrap(),
        );
        let font_lease = uqm_rust::graphics::native_font::FontLease::retain(
            std::ptr::NonNull::new(font).unwrap(),
        );
        let waits = Rc::new(Cell::new(0));
        let mut game = Game::new();
        game.kernel.space_context = context;
        game.kernel.star_con_font = font;
        game.kernel.game_strings = CaptureStringTable(strings).cast();
        game.kernel.install_input_runtime(Rc::new(Runtime(
            WaitObserver {
                waits: waits.clone(),
                frame,
            },
            uqm_rust::engine::input::InputSession::default(),
        )));
        uqm_rust::app::bridge::with_game(&mut game, || {
            {
                use uqm_rust::{callback::ffi as alarms, time::source};
                source::install(true);
                alarms::Alarm_init();
                let region = |x| RECT {
                    corner: POINT { x, y: 1 },
                    extent: EXTENT {
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
                assert_eq!(alarms::Alarm_timeBeforeNextMs(), 61);
                source::advance_to(51);
                assert!(!alarms::Alarm_processOne());
                source::advance_to(52);
                assert!(alarms::Alarm_processOne());
                assert!(!alarms::Alarm_processOne());
                manual.process();
                TFB_FlushGraphics();
                let pixel = |x| {
                    let p = GetFramePixel(frame, POINT { x, y: 2 });
                    (p.r, p.g, p.b, p.a)
                };
                assert_eq!(pixel(2), pixel(11));
                assert_eq!(alarms::Alarm_timeBeforeNextMs(), 62);
                scheduled.pause();
                source::advance_to(104);
                assert!(!alarms::Alarm_processOne());
                scheduled.resume();
                assert!(alarms::Alarm_processOne());
                manual.process();
                TFB_FlushGraphics();
                assert_eq!(pixel(2), pixel(11));
                scheduled.pre_update();
                manual.pre_update();
                TFB_FlushGraphics();
                assert_eq!(pixel(2), (9, 17, 25, 255));
                assert_eq!(pixel(2), pixel(11));
                scheduled.post_update();
                manual.post_update();
                TFB_FlushGraphics();
                assert_eq!(pixel(2), pixel(11));
                drop(scheduled);
                drop(manual);
                source::advance_to(1_000);
                assert!(!alarms::Alarm_processOne());
                TFB_FlushGraphics();
                assert_eq!(pixel(2), (9, 17, 25, 255));
                let clip = RECT {
                    corner: POINT { x: 20, y: 10 },
                    extent: EXTENT {
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
                TFB_FlushGraphics();
                let translated = GetFramePixel(frame, POINT { x: 22, y: 12 });
                assert_eq!((translated.r, translated.g, translated.b), (9, 17, 25));
                assert_eq!(alarms::Alarm_timeBeforeNextMs(), 0);
                assert!(alarms::Alarm_processOne());
                reference.process();
                flash_kernel.sis_highlight.pause_flash();
                source::advance_to(1_052);
                assert!(!alarms::Alarm_processOne());
                flash_kernel.sis_highlight.continue_flash();
                assert!(alarms::Alarm_processOne());
                reference.process();
                TFB_FlushGraphics();
                let translated = GetFramePixel(frame, POINT { x: 22, y: 12 });
                let original = GetFramePixel(frame, POINT { x: 31, y: 2 });
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
                assert!(!alarms::Alarm_processOne());
                TFB_FlushGraphics();
                let restored = GetFramePixel(frame, POINT { x: 22, y: 12 });
                assert_eq!((restored.r, restored.g, restored.b), (9, 17, 25));
                alarms::Alarm_uninit();
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
                        RECT {
                            corner: POINT { x: 0, y: 0 },
                            extent: EXTENT {
                                width: 65,
                                height: 77,
                            },
                        },
                    );
                    uqm_rust::graphics::font_abi::SetContextFont(ptr::null_mut());
                    uqm_rust::ui::shipstat::draw_ship_name_text(
                        &context_lease,
                        &font_lease,
                        c"AA",
                        count,
                        offset,
                    );
                    assert!((*context).Font.is_null(), "title must restore its font");
                    let foreground = (*context).ForeGroundColor;
                    assert_eq!((foreground.r, foreground.g, foreground.b), (0, 0, 0));
                    TFB_FlushGraphics();
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
                            let pixel = GetFramePixel(frame, POINT { x, y });
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
            TFB_FlushGraphics();
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
                    let pixel = GetFramePixel(frame, POINT { x, y });
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
                        RECT {
                            corner: POINT { x: 0, y: 0 },
                            extent: EXTENT {
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
                    TFB_FlushGraphics();
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
                            let pixel = GetFramePixel(frame, POINT { x, y });
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
                RECT {
                    corner: POINT { x: 0, y: 0 },
                    extent: EXTENT {
                        width: 65,
                        height: 77,
                    },
                },
            );
            {
                let target = context_lease.target();
                // Loaded status stencils have alpha or a color key; an opaque canvas cannot mask a fill.
                let labels =
                    uqm_rust::graphics::drawable_abi::LoadedDrawable::create(8, 1, 1, 6).unwrap();
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
                        RECT {
                            corner: POINT { x: 0, y: 0 },
                            extent: EXTENT {
                                width: 1,
                                height: 1,
                            },
                        },
                    );
                }
                context_lease.set_frame(target);
                TFB_FlushGraphics();
                for index in 0..6 {
                    let pixel = GetFramePixel(frames.frame(index).unwrap(), POINT { x: 0, y: 0 });
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
                            RECT {
                                corner: POINT { x: 0, y: 0 },
                                extent: EXTENT {
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
                        TFB_FlushGraphics();
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
                            let pixel = GetFramePixel(frame, POINT { x, y: 62 });
                            assert_eq!(
                                (pixel.r, pixel.g, pixel.b),
                                expected,
                                "labels pc={pc_menu} change={change} x={x}"
                            );
                        }
                        let foreground = (*context).ForeGroundColor;
                        let expected = if change < 0 {
                            (255, 255, 82)
                        } else {
                            (9, 17, 25)
                        };
                        assert_eq!((foreground.r, foreground.g, foreground.b), expected);
                    }
                }
                {
                    use uqm_rust::battle::ContextOps;
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
                        RECT {
                            corner: POINT { x: 0, y: 0 },
                            extent: EXTENT {
                                width: 1,
                                height: 1,
                            },
                        },
                    );
                    context_lease.set_frame(target);
                    uqm_rust::app::bridge::enter(|cx| {
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
                        let selected = uqm_rust::graphics::context_abi::_pCurContext;
                        assert_eq!(selected, previous);
                        let info = &cx.desc(id).ship_info;
                        assert_eq!((info.crew_level, info.energy_level), (2, 3));
                        assert_eq!(
                            (info.icons, info.max_crew, info.max_energy, info.ship_cost),
                            (icon, 3, 4, 7)
                        );
                        TFB_FlushGraphics();
                        let pixel = GetFramePixel(frame, POINT { x: 32, y: 31 });
                        assert_eq!((pixel.r, pixel.g, pixel.b), (73, 91, 109));
                        let active_canvas = LoadedDrawable::create(8, 65, 77, 1).unwrap();
                        let active_frame = active_canvas.lease().frame(0).unwrap();
                        let active_context =
                            ContextLease::retain(std::ptr::NonNull::new(previous).unwrap());
                        active_context.set_frame(active_canvas.lease().context_frame(0));
                        let gauge_pixel = POINT { x: 8, y: 52 };
                        let status_before = GetFramePixel(frame, gauge_pixel);
                        cx.draw_statistics(id, 1, -1);
                        TFB_FlushGraphics();
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
                        let selected = uqm_rust::graphics::context_abi::_pCurContext;
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
                        cx.kernel.status_context = ptr::null_mut();
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
                        RECT {
                            corner: POINT { x: 0, y: 0 },
                            extent: EXTENT {
                                width: 65,
                                height: 77,
                            },
                        },
                    );
                    let mut ship: uqm_rust::navigation::abi::SHIP_INFO = std::mem::zeroed();
                    ship.max_crew = 2;
                    ship.max_energy = 2;
                    ship.crew_level = 2;
                    ship.energy_level = 2;
                    let clip = clip_x.map(|x| RECT {
                        corner: POINT { x, y: 1 },
                        extent: EXTENT {
                            width: 55,
                            height: 52,
                        },
                    });
                    let original_clip = context_lease.clip_rect();
                    let original_origin = (*frame).HotSpot;
                    uqm_rust::graphics::font_abi::SetContextFont(ptr::null_mut());
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
                    let selected = uqm_rust::graphics::context_abi::_pCurContext;
                    assert_eq!(selected, previous);
                    assert!((*context).Font.is_null());
                    assert_eq!((ship.crew_level, ship.energy_level), (2, 2));
                    assert_eq!(
                        ((*frame).HotSpot.x, (*frame).HotSpot.y),
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
                    TFB_FlushGraphics();
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
                        let pixel = GetFramePixel(frame, POINT { x, y });
                        assert_eq!(
                            (pixel.r, pixel.g, pixel.b),
                            expected,
                            "panel clip_x={clip_x:?} ({x},{y})"
                        );
                    }
                }
            }
            let mut stats: uqm_rust::navigation::abi::SHIP_INFO = std::mem::zeroed();
            stats.ship_flags = 1 << 13;
            let update = |stats: &mut uqm_rust::navigation::abi::SHIP_INFO, crew, energy| {
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
                TFB_FlushGraphics();
                let pixel = GetFramePixel(frame, POINT { x, y });
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
            let foreground = (*context).ForeGroundColor;
            assert_eq!((foreground.r, foreground.g, foreground.b), (0, 165, 0));
            update(&mut stats, 0, 0);
            let foreground = (*context).ForeGroundColor;
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
                TFB_FlushGraphics();
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
                        let pixel = GetFramePixel(frame, POINT { x, y });
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
                    TFB_FlushGraphics();
                    for y in 119..129 {
                        for x in 156..165 {
                            let pixel = GetFramePixel(frame, POINT { x, y });
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
                        RECT {
                            corner: POINT { x: 0, y: 0 },
                            extent: EXTENT {
                                width: 320,
                                height: 240,
                            },
                        },
                    );
                }
            }
            for text in [b'A', 0] {
                *(*message).data = text as std::ffi::c_char;
                (*message).length = i32::from(text != 0);
                SetContext(previous);
                uqm_rust::app::bridge::enter(|cx| {
                    let mut observer = cx.kernel_mut().input_runtime().observer();
                    let mut io = Io {
                        observer: &mut *observer,
                    };
                    let text = std::ffi::CStr::from_ptr((*message).data);
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
                assert_eq!((*context).Font, font);
                let foreground = (*context).ForeGroundColor;
                assert_eq!((foreground.r, foreground.g, foreground.b), (165, 165, 165));
                TFB_FlushGraphics();
                for y in 108..120 {
                    for x in 116..128 {
                        let pixel = GetFramePixel(frame, POINT { x, y });
                        assert_eq!(
                            (pixel.r, pixel.g, pixel.b),
                            (9, 17, 25),
                            "popup pixels must be restored"
                        );
                    }
                }
            }
        });
        assert_eq!(waits.get(), 2);
        game.kernel.game_strings = ptr::null_mut();
        game.kernel.star_con_font = ptr::null_mut();
        game.kernel.space_context = ptr::null_mut();
        FreeStringTable(strings);
        SetContext(ptr::null_mut());
        DestroyDrawable(drawable);
        drop(context_lease);
        drop(font_lease);
        DestroyContext(context);
        DestroyContext(previous);
        uqm_rust::graphics::font_abi::DestroyFont(font);
        TFB_UninitGraphics();
    }
}
