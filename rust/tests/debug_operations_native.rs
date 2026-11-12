#![cfg(feature = "debug-process")]

#[path = "support/campaign_display.rs"]
mod campaign_display;

use std::{cell::Cell, io, rc::Rc};
use uqm_rust::{
    engine::{
        battle::{input::InputContext, Battle, Cx},
        input::{frames::InputState, Frame, InputControls, InputRuntime, InputSession},
        observe::{CampaignView, Io, Observer, Unobserved},
        Game,
    },
    front_end::uqmdebug::debug_contexts,
    graphics::{
        context_abi::{ContextFrame, ContextLease, LoadedContext},
        native_abi::{Color, EXTENT, POINT, RECT},
    },
};

struct Driver {
    input: InputSession,
    screen: ContextLease,
    background: Color,
    calls: Cell<usize>,
    panic_on_run: Cell<bool>,
}

impl InputRuntime for Driver {
    fn input(&self) -> &InputSession {
        &self.input
    }
    fn observer(&self) -> Box<dyn Observer<Option<CampaignView>>> {
        Box::new(Unobserved)
    }
    fn yield_poll(&self, _: &mut Cx<'_>) {
        panic!("unexpected diagnostic yield")
    }
    fn sleep(&self, _: &mut Cx<'_>, _: u32) {
        panic!("select must end the diagnostic wait")
    }
    fn sleep_until(&self, _: &mut Cx<'_>, _: u32) {
        panic!("unexpected diagnostic deadline")
    }
    fn flush(&self) {}
    fn confirm_exit(&self, _: &mut Cx<'_>) -> bool {
        panic!("unexpected exit confirmation")
    }
    fn update(&self, _: &mut Cx<'_>) {}
    fn battle_input(&self, _: &InputContext) -> u8 {
        panic!("unexpected battle sampling")
    }
    fn run(&self, cx: &mut Cx<'_>, io: &mut Io<'_>, reset: bool, frame: &mut dyn Frame) {
        assert!(reset);
        self.calls.set(self.calls.get() + 1);
        let pixel = self.screen.with_selected(|| {
            self.screen.flush();
            self.screen.target().pixel(POINT { x: 10, y: 10 })
        });
        assert_ne!(
            pixel, self.background,
            "the real overlay must reach its target"
        );
        let mut nested = Vec::new();
        debug_contexts(cx, io, &mut nested).unwrap();
        assert!(nested.is_empty(), "nested inspection must not recurse");
        assert!(!self.panic_on_run.get(), "injected diagnostic unwind");
        let mut pressed = InputState::default();
        pressed.menu[uqm_rust::navigation::abi::KEY_MENU_SELECT as usize] = 1;
        self.input.advance(InputState::default(), pressed, 1000);
        assert!(!frame.frame(cx, io));
    }
}

#[test]
#[serial_test::serial]
fn contexts_draw_actual_pixels_and_restore_display_selection_and_owned_resources() {
    let mut game = Game::new();
    let display = campaign_display::CampaignDisplay::install(&mut game).unwrap();
    let screen = ContextLease::require(game.kernel.status_context);
    game.kernel.screen_context = game.kernel.status_context;
    screen.set_frame(ContextFrame::retain(std::ptr::NonNull::new(
        game.kernel.screen,
    )));
    screen.set_clip(RECT {
        corner: POINT { x: 0, y: 0 },
        extent: EXTENT {
            width: 320,
            height: 240,
        },
    });
    let background = Color {
        r: 7,
        g: 9,
        b: 13,
        a: 255,
    };
    screen.fill_rectangle(background, screen.clip_rect());
    screen.flush();
    let original = screen.target();
    assert_eq!(original.pixel(POINT { x: 10, y: 10 }), background);
    let other = LoadedContext::create(c"non-screen selection");
    let other_context = other.lease();
    other_context.select();
    let selected = other_context.diagnostic_name();
    let contexts_before = screen.diagnostic_contexts().len();
    let driver = Rc::new(Driver {
        input: InputSession::new(InputControls::default(), game.kernel.configuration.clone()),
        screen: screen.clone(),
        background,
        calls: Cell::new(0),
        panic_on_run: Cell::new(false),
    });
    game.kernel.install_input_runtime(driver.clone());
    let mut battle = Battle::new();
    let mut observer = Unobserved;
    let mut io = Io {
        observer: &mut observer,
    };
    let mut cx = Cx::new(
        &mut battle,
        &mut game.world,
        &mut game.kernel,
        uqm_rust::math::random,
    );
    let mut output = Vec::new();
    debug_contexts(&mut cx, &mut io, &mut output).unwrap();
    assert_eq!(driver.calls.get(), 1);
    assert!(output
        .windows(b"campaign callback fixture".len())
        .any(|bytes| bytes == b"campaign callback fixture"));
    assert!(output
        .windows(b"(320 x 240)".len())
        .any(|bytes| bytes == b"(320 x 240)"));
    assert_eq!(screen.diagnostic_contexts().len(), contexts_before);
    assert_eq!(
        other_context.selected_context().unwrap().diagnostic_name(),
        selected
    );
    screen.with_selected(|| screen.flush());
    assert_eq!(original.pixel(POINT { x: 10, y: 10 }), background);

    struct Broken;
    impl io::Write for Broken {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::ErrorKind::BrokenPipe.into())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    assert_eq!(
        debug_contexts(&mut cx, &mut io, &mut Broken)
            .unwrap_err()
            .kind(),
        io::ErrorKind::BrokenPipe
    );
    assert_eq!(driver.calls.get(), 1);
    assert_eq!(screen.diagnostic_contexts().len(), contexts_before);
    assert_eq!(
        other_context.selected_context().unwrap().diagnostic_name(),
        selected
    );

    driver.panic_on_run.set(true);
    let unwind = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        debug_contexts(&mut cx, &mut io, &mut Vec::new()).unwrap();
    }));
    assert!(unwind.is_err());
    screen.with_selected(|| screen.flush());
    assert_eq!(original.pixel(POINT { x: 10, y: 10 }), background);
    assert_eq!(screen.diagnostic_contexts().len(), contexts_before);
    assert_eq!(
        other_context.selected_context().unwrap().diagnostic_name(),
        selected
    );
    driver.panic_on_run.set(false);
    debug_contexts(&mut cx, &mut io, &mut Vec::new()).unwrap();
    assert_eq!(driver.calls.get(), 3);
    drop(driver);
    drop(other_context);
    drop(other);
    drop(screen);
    drop(original);
    display.finish(game);
}
