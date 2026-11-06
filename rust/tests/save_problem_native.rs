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
