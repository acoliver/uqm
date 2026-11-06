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
        let strings = AllocStringTable(457, 0);
        let message = (*strings).strings.add(456);
        (*message).data = uqm_rust::memory::rust_hmalloc(2).cast();
        ptr::copy_nonoverlapping(c"A".as_ptr(), (*message).data, 2);
        (*message).length = 1;
        let waits = Rc::new(Cell::new(0));
        let mut game = Game::new();
        game.kernel.space_context = context;
        game.kernel.star_con_font = &mut font;
        game.kernel.game_strings = CaptureStringTable(strings).cast();
        game.kernel.install_input_runtime(Rc::new(Runtime(
            WaitObserver {
                waits: waits.clone(),
                frame,
            },
            uqm_rust::engine::input::InputSession::default(),
        )));
        uqm_rust::app::bridge::with_game(&mut game, || {
            for text in [b'A', 0] {
                *(*message).data = text as std::ffi::c_char;
                (*message).length = i32::from(text != 0);
                SetContext(previous);
                uqm_rust::navigation::abi::SaveProblem();
                assert_eq!(
                    SetContext(previous),
                    previous,
                    "previous context must be restored"
                );
                TFB_FlushGraphics();
                for (x, y) in [(118, 110), (120, 112), (124, 115)] {
                    let pixel = GetFramePixel(frame, POINT { x, y });
                    assert_eq!(
                        (pixel.r, pixel.g, pixel.b),
                        (9, 17, 25),
                        "popup pixels must be restored"
                    );
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
        DestroyContext(context);
        DestroyContext(previous);
        TFB_UninitGraphics();
    }
}
