use std::{cell::Cell, ptr, rc::Rc};
use uqm_rust::{
    battle::RaceOps,
    engine::input::frames::{KEY_MENU_RIGHT, KEY_MENU_SELECT},
    engine::world::activity::activity_flags::IN_BATTLE,
    engine::{
        battle::{input::InputContext, Battle, Cx},
        input::{frames::InputState, Frame, InputControls, InputRuntime, InputSession},
        observe::{CampaignView, Io, Observer, Unobserved},
        Game,
    },
    graphics::{
        context_abi::*,
        dcqueue_abi::TFB_FlushGraphics,
        drawable_abi::{GetFramePixel, LoadedDrawable},
        native_abi::*,
        sdl::lifecycle_abi::{GraphicsOptions, TFB_InitGraphics, TFB_UninitGraphics},
    },
};

struct MenuDriver {
    input: InputSession,
    frames: Cell<usize>,
}

impl InputRuntime for MenuDriver {
    fn input(&self) -> &InputSession {
        &self.input
    }
    fn observer(&self) -> Box<dyn Observer<Option<CampaignView>>> {
        Box::new(Unobserved)
    }
    fn yield_poll(&self, _: &mut Cx<'_>) {
        panic!("unexpected menu yield")
    }
    fn sleep(&self, _: &mut Cx<'_>, _: u32) {}
    fn sleep_until(&self, _: &mut Cx<'_>, _: u32) {}
    fn flush(&self) {
        self.input.clear_menu_state();
    }
    fn confirm_exit(&self, _: &mut Cx<'_>) -> bool {
        panic!("unexpected exit confirmation")
    }
    fn update(&self, _: &mut Cx<'_>) {
        self.input
            .advance(InputState::default(), InputState::default(), 10000);
    }
    fn battle_input(&self, _: &InputContext) -> u8 {
        panic!("unexpected battle input")
    }
    fn run(&self, cx: &mut Cx<'_>, io: &mut Io<'_>, reset: bool, frame: &mut dyn Frame) {
        assert!(reset);
        let keys = [
            KEY_MENU_RIGHT,
            KEY_MENU_SELECT,
            KEY_MENU_RIGHT,
            KEY_MENU_RIGHT,
            KEY_MENU_RIGHT,
            KEY_MENU_RIGHT,
            KEY_MENU_SELECT,
        ];
        for (index, key) in keys.into_iter().enumerate() {
            let mut pressed = InputState::default();
            pressed.menu[key as usize] = 1;
            self.input
                .advance(pressed, pressed, 1000 * (index as u32 + 1));
            assert_eq!(frame.frame(cx, io), index + 1 != keys.len());
            self.frames.set(index + 1);
        }
        self.update(cx);
    }
}

#[test]
#[serial_test::serial]
fn native_menu_navigates_empty_devices_and_restores_the_retained_display() {
    std::env::set_var("SDL_VIDEODRIVER", "dummy");
    std::env::set_var("SDL_AUDIODRIVER", "dummy");
    uqm_rust::callback::ffi::Alarm_init();
    // SAFETY: this serialized test owns the graphics runtime, contexts and drawable until teardown.
    unsafe {
        assert_eq!(
            TFB_InitGraphics(
                1,
                0,
                ptr::from_ref(&GraphicsOptions::default()).cast(),
                320,
                240,
            ),
            0
        );
        let contexts: [_; 3] = std::array::from_fn(|_| {
            #[cfg(feature = "debug-process")]
            {
                CreateContextAux(c"hyperspace menu fixture".as_ptr())
            }
            #[cfg(not(feature = "debug-process"))]
            {
                CreateContextAux()
            }
        });
        let [space, status, screen_context] = contexts;
        let drawable = LoadedDrawable::create(8, 320, 240, 1).unwrap();
        let screen = drawable.lease().frame(0).unwrap();
        for context in contexts {
            ContextLease::retain(ptr::NonNull::new(context).unwrap())
                .set_frame(drawable.lease().context_frame(0));
        }
        ContextLease::retain(ptr::NonNull::new(status).unwrap()).set_clip(RECT {
            corner: POINT { x: 256, y: 0 },
            extent: EXTENT {
                width: 64,
                height: 240,
            },
        });
        let background = Color {
            r: 9,
            g: 17,
            b: 25,
            a: 255,
        };
        ContextLease::retain(ptr::NonNull::new(space).unwrap()).set_background(background);
        SetContext(status);
        let mut game = Game::new();
        game.world.story = Some(uqm_rust::engine::world::story::StoryState::new());
        game.world.set_activity(IN_BATTLE);
        game.kernel
            .configuration
            .update_preferences(|preferences| preferences.menu = 0);
        game.kernel.space_context = space;
        game.kernel.status_context = status;
        game.kernel.screen_context = screen_context;
        game.kernel.screen = screen;
        let runtime = Rc::new(MenuDriver {
            input: InputSession::new(InputControls::default(), game.kernel.configuration.clone()),
            frames: Cell::new(0),
        });
        game.kernel.install_input_runtime(runtime.clone());
        let mut battle = Battle::new();
        let mut cx = Cx::new(&mut battle, &mut game.world, &mut game.kernel, || {
            panic!("unexpected menu RNG")
        });
        cx.hyperspace_menu();
        assert_eq!(runtime.frames.get(), 7);
        assert_eq!(SetContext(status), status);
        SetContext(space);
        let restored = SetContextBackGroundColor(background);
        assert_eq!((restored.r, restored.g, restored.b), (9, 17, 25));
        TFB_FlushGraphics();
        let radar = GetFramePixel(screen, POINT { x: 259, y: 173 });
        assert_eq!((radar.r, radar.g, radar.b), (82, 82, 82));
        assert_eq!(cx.world.activity(), IN_BATTLE);
        SetContext(ptr::null_mut());
        for context in contexts {
            DestroyContext(context);
        }
        drop(drawable);
        TFB_UninitGraphics();
    }
    uqm_rust::callback::ffi::Alarm_uninit();
}
