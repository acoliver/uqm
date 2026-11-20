use std::{cell::Cell, rc::Rc};
use uqm_rust::engine::battle::input::InputContext;
use uqm_rust::engine::battle::Battle;
use uqm_rust::engine::battle::Cx;
use uqm_rust::engine::input::frames::InputState;
use uqm_rust::engine::input::frames::KEY_MENU_RIGHT;
use uqm_rust::engine::input::frames::KEY_MENU_SELECT;
use uqm_rust::engine::input::Frame;
use uqm_rust::engine::input::InputControls;
use uqm_rust::engine::input::InputRuntime;
use uqm_rust::engine::input::InputSession;
use uqm_rust::engine::observe::Io;
use uqm_rust::engine::observe::Observer;
use uqm_rust::engine::observe::Unobserved;
use uqm_rust::engine::test_support::native_probe::frame_pixel as GetFramePixel;
use uqm_rust::engine::world::activity::activity_flags::IN_BATTLE;
use uqm_rust::engine::Game;
use uqm_rust::graphics::context::{ContextLease, LoadedContext};
use uqm_rust::graphics::{
    drawable::LoadedDrawable, geometry::Color, geometry::Extent, geometry::Point, geometry::Rect,
    menu::select as SetContext, present::GraphicsOptions,
};

struct MenuDriver {
    input: InputSession,
    frames: Cell<usize>,
}

impl InputRuntime for MenuDriver {
    fn input(&self) -> &InputSession {
        &self.input
    }
    fn observer(&self) -> Box<dyn Observer> {
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
    {
        uqm_rust::graphics::present::init_graphics(1, 0, GraphicsOptions::default(), 320, 240);
        let contexts: [_; 3] = std::array::from_fn(|_| {
            Some(LoadedContext::create(c"hyperspace menu fixture").into_id())
        });
        let [space, status, screen_context] = contexts;
        let drawable = LoadedDrawable::create(8, 320, 240, 1).unwrap();
        let screen = drawable.lease().frame(0).unwrap();
        for context in contexts {
            ContextLease::require(context).set_frame(drawable.lease().context_frame(0));
        }
        ContextLease::require(status).set_clip(Rect {
            corner: Point { x: 256, y: 0 },
            extent: Extent {
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
        ContextLease::require(space).set_background(background);
        SetContext(status);
        let mut game = Game::new();
        uqm_rust::battle::services::install(&mut game.kernel);
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
        let restored = uqm_rust::graphics::menu::background(background);
        assert_eq!((restored.r, restored.g, restored.b), (9, 17, 25));
        uqm_rust::graphics::state::flush();
        let radar = GetFramePixel(screen, Point { x: 259, y: 173 });
        assert_eq!((radar.r, radar.g, radar.b), (82, 82, 82));
        assert_eq!(cx.world.activity(), IN_BATTLE);
        SetContext(None);
        for context in contexts {
            drop(context.map(LoadedContext::from_id));
        }
        drop(drawable);
        uqm_rust::graphics::present::uninit_graphics();
    }
    uqm_rust::engine::host::dispatch().clear();
}
