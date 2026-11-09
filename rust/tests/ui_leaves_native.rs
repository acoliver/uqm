//! Owned modal windows restore their pixels and input policy after cancellation.
#[path = "support/campaign_display.rs"]
mod campaign_display;
#[path = "support/native_process.rs"]
mod native_process;

use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    rc::Rc,
};
use uqm_rust::{
    engine::{
        battle::{Battle, Cx},
        input::{frames::*, Frame, InputRuntime, InputSession},
        observe::{
            CampaignView, CheckpointView, Injected, InputView, Io, Observer, Presented, Unobserved,
        },
        Game,
    },
    graphics::{
        context_abi::{ContextFrame, ContextLease},
        native_abi::{Color, POINT},
    },
};

struct Runtime {
    input: InputSession,
    keys: RefCell<VecDeque<u32>>,
    updates: Cell<usize>,
    sleeps: Cell<usize>,
    flushes: Cell<usize>,
    context: ContextLease,
}

impl InputRuntime for Runtime {
    fn input(&self) -> &InputSession {
        &self.input
    }
    fn observer(&self) -> Box<dyn Observer<Option<CampaignView>>> {
        Box::new(Unobserved)
    }
    fn yield_poll(&self, _: &mut Cx<'_>) {
        panic!("modal fixture does not yield");
    }
    fn sleep(&self, _: &mut Cx<'_>, _: u32) {
        self.sleeps.set(self.sleeps.get() + 1);
    }
    fn sleep_until(&self, _: &mut Cx<'_>, _: u32) {
        panic!("modal fixture does not sleep to a deadline");
    }
    fn flush(&self) {
        self.flushes.set(self.flushes.get() + 1);
        self.input.clear_menu_state();
    }
    fn confirm_exit(&self, _: &mut Cx<'_>) -> bool {
        panic!("modal fixture does not recurse into confirmation");
    }
    fn update(&self, _: &mut Cx<'_>) {
        self.updates.set(self.updates.get() + 1);
        let key = self
            .keys
            .borrow_mut()
            .pop_front()
            .unwrap_or_else(|| panic!("unexpected modal sample"));
        let mut state = InputState::default();
        if key != u32::MAX {
            state.menu[key as usize] = 1;
        }
        self.input.advance(state, state, 0);
    }
    fn run(&self, cx: &mut Cx<'_>, io: &mut Io<'_>, reset: bool, frame: &mut dyn Frame) {
        assert!(reset);
        self.context.flush();
        let painted = self.context.target().pixel(POINT { x: 150, y: 105 });
        assert_ne!((painted.r, painted.g, painted.b), (9, 17, 25));
        self.update(cx);
        assert!(!frame.frame(cx, io));
    }
    fn battle_input(&self, _: &uqm_rust::engine::battle::input::InputContext) -> u8 {
        panic!("modal fixture has no ship input");
    }
}

#[test]
#[serial_test::serial]
fn cancellation_restores_confirmation_popup_pixels_and_input_policy() {
    if !native_process::own_graphics_process(
        "cancellation_restores_confirmation_popup_pixels_and_input_policy",
    ) {
        return;
    }
    let mut game = Game::new();
    let display = campaign_display::CampaignDisplay::install(&mut game).unwrap();
    game.kernel.screen_context = game.kernel.status_context;
    game.kernel.star_con_font = game.kernel.tiny_font;
    let context = ContextLease::require(game.kernel.screen_context);
    context.set_frame(ContextFrame::retain(std::ptr::NonNull::new(
        game.kernel.screen,
    )));
    context.set_background(Color {
        r: 9,
        g: 17,
        b: 25,
        a: 255,
    });
    context.clear();
    context.flush();
    let runtime = Rc::new(Runtime {
        input: InputSession::default(),
        keys: RefCell::new(VecDeque::new()),
        updates: Cell::new(0),
        sleeps: Cell::new(0),
        flushes: Cell::new(0),
        context: context.clone(),
    });
    game.kernel.install_input_runtime(runtime.clone());
    let mut battle = Battle::new();
    {
        let mut cx = Cx::new(
            &mut battle,
            &mut game.world,
            &mut game.kernel,
            uqm_rust::math::random,
        );
        let mut observer = Unobserved;
        let mut io = Io {
            observer: &mut observer,
        };
        for (keys, expected) in [
            (vec![KEY_MENU_CANCEL], 0),
            (vec![KEY_MENU_RIGHT, KEY_MENU_SELECT], 1),
        ] {
            runtime.keys.borrow_mut().extend(keys);
            assert_eq!(
                uqm_rust::ui::confirm::do_confirm_exit(&mut cx, &runtime.input),
                expected
            );
            assert_eq!(
                cx.world.activity() & 0x4000,
                if expected == 1 { 0x4000 } else { 0 }
            );
            assert!(!runtime.input.paused());
            assert!(!runtime.input.exit_requested());
            context.flush();
            let restored = context.target().pixel(POINT { x: 140, y: 110 });
            assert_eq!((restored.r, restored.g, restored.b), (9, 17, 25));
            cx.world.set_activity(0);
        }
        runtime.input.set_menu_sounds(7, 11);
        runtime.keys.borrow_mut().push_back(KEY_MENU_CANCEL);
        uqm_rust::ui::confirm::do_popup_window(&mut cx, &mut io, c"A\nA");
        assert_eq!(runtime.input.menu_sounds(), (7, 11));
        assert!(runtime.input.callback().is_none());
        context.flush();
        let restored = context.target().pixel(POINT { x: 150, y: 105 });
        assert_eq!((restored.r, restored.g, restored.b), (9, 17, 25));
    }
    assert_eq!(runtime.updates.get(), 4);
    assert_eq!(runtime.sleeps.get(), 4);
    assert_eq!(runtime.flushes.get(), 4);
    drop(runtime);
    drop(context);
    display.finish(game);
}

#[derive(Default)]
struct WaitObserver(usize);
impl Observer<Option<CampaignView>> for WaitObserver {
    fn input_admitted(&mut self, view: &InputView, _: &Option<CampaignView>) -> Injected {
        assert_eq!(*view, InputView::Nested);
        self.0 += 1;
        Injected {
            stop: false,
            seed: None,
            planet_side: None,
        }
    }
    fn checkpoint(&mut self, _: &CheckpointView<'_>) {}
    fn presented(&mut self, _: &mut Presented) {}
}

#[test]
#[serial_test::serial]
fn wait_samples_initial_state_then_admits_each_nested_sample_and_flushes_once() {
    if !native_process::own_graphics_process(
        "wait_samples_initial_state_then_admits_each_nested_sample_and_flushes_once",
    ) {
        return;
    }
    let mut game = Game::new();
    let display = campaign_display::CampaignDisplay::install(&mut game).unwrap();
    let runtime = Rc::new(Runtime {
        input: InputSession::default(),
        keys: RefCell::new([KEY_MENU_SELECT, u32::MAX, KEY_MENU_SELECT].into()),
        updates: Cell::new(0),
        sleeps: Cell::new(0),
        flushes: Cell::new(0),
        context: ContextLease::require(game.kernel.status_context),
    });
    game.kernel.install_input_runtime(runtime.clone());
    let mut battle = Battle::new();
    let mut observer = WaitObserver::default();
    {
        let mut io = Io {
            observer: &mut observer,
        };
        let mut cx = Cx::new(
            &mut battle,
            &mut game.world,
            &mut game.kernel,
            uqm_rust::math::random,
        );
        assert_eq!(
            uqm_rust::ui::util::wait_for_any_button(&mut cx, &mut io, 1, u32::MAX, 1),
            1
        );
    }
    assert_eq!(runtime.updates.get(), 3);
    assert_eq!(runtime.sleeps.get(), 1);
    assert_eq!(runtime.flushes.get(), 1);
    assert_eq!(observer.0, 1);
    assert_eq!(
        runtime.input.current().menu[KEY_MENU_SELECT as usize],
        1,
        "flush does not rewrite the already sampled frame"
    );
    drop(runtime);
    display.finish(game);
}
