use super::*;
use crate::engine::{
    battle::{input::InputContext, Battle},
    input::{frames::InputState, Frame, InputControls, InputRuntime},
    observe::{CampaignView, Observer, Unobserved},
    Game,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[derive(Default)]
struct Driver {
    input: InputSession,
    sleeps: RefCell<Vec<u32>>,
    panic_on_run: Cell<bool>,
    yields: Cell<usize>,
}
impl InputRuntime for Driver {
    fn input(&self) -> &InputSession {
        &self.input
    }
    fn observer(&self) -> Box<dyn Observer<Option<CampaignView>>> {
        Box::new(Unobserved)
    }
    fn yield_poll(&self, _: &mut Cx<'_>) {
        panic!("presentation must not poll callbacks while unbatching")
    }
    fn yield_task(&self, _: &mut Cx<'_>) {
        self.yields.set(self.yields.get() + 1);
    }
    fn sleep(&self, _: &mut Cx<'_>, ticks: u32) {
        self.sleeps.borrow_mut().push(ticks);
    }
    fn sleep_until(&self, _: &mut Cx<'_>, deadline: u32) {
        self.sleeps.borrow_mut().push(deadline);
    }
    fn flush(&self) {}
    fn confirm_exit(&self, _: &mut Cx<'_>) -> bool {
        panic!("unexpected exit confirmation")
    }
    fn update(&self, _: &mut Cx<'_>) {
        panic!("unexpected input update")
    }
    fn battle_input(&self, _: &InputContext) -> u8 {
        panic!("unexpected battle input")
    }
    fn run(&self, cx: &mut Cx<'_>, io: &mut Io<'_>, reset: bool, frame: &mut dyn Frame) {
        assert!(reset);
        assert!(!self.panic_on_run.get(), "presentation fixture unwind");
        assert!(!frame.frame(cx, io));
    }
}

fn pulse(input: &InputControls, key: u32) {
    input.advance(InputState::default(), InputState::default(), 0);
    let mut state = InputState::default();
    state.menu[key as usize] = 1;
    input.advance(state, state, 1);
}

#[test]
#[serial_test::serial]
fn commands_draw_retained_frames_and_preserve_wait_call_skip_and_restore_order() {
    if !crate::native_test_process::own_graphics_process("front_end::intro::render_tests::commands_draw_retained_frames_and_preserve_wait_call_skip_and_restore_order") { return; }
    let mut game = Game::new();
    let fixture = crate::campaign_display_test::CampaignDisplay::install(&mut game).unwrap();
    let context = ContextLease::require(game.kernel.status_context);
    game.kernel.screen_context = game.kernel.status_context;
    context.set_frame(ContextFrame::retain(std::ptr::NonNull::new(
        game.kernel.screen,
    )));
    let screen = RECT {
        corner: POINT { x: 0, y: 0 },
        extent: EXTENT {
            width: 320,
            height: 240,
        },
    };
    context.set_clip(screen);
    context.set_font(&crate::graphics::native_font::FontLease::retain(
        std::ptr::NonNull::new(game.kernel.tiny_font).unwrap(),
    ));
    let original_font = context.font_lease().unwrap();
    let driver = Rc::new(Driver::default());
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
        crate::math::random,
    );
    let mut state = PresentationState::new(
        LoadedTextTable::new(vec![
            c"DIMS 320 240".to_owned(),
            c"TC ff0000".to_owned(),
            c"BATCH".to_owned(),
            c"LINE 2 3 7 3".to_owned(),
            c"UNBATCH".to_owned(),
            c"WAIT 20".to_owned(),
            c"CALL missing.txt".to_owned(),
            c"NOOP".to_owned(),
        ]),
        context.clone(),
        false,
    );
    assert!(matches!(
        commands::advance(&mut cx, &mut io, &mut state),
        commands::Step::Input(true)
    ));
    assert_eq!(state.oper_index, 6);
    assert!(state.time_out_on_skip);
    assert_eq!(driver.yields.get(), 0);
    context.flush();
    let pixel = context.target().pixel(POINT { x: 4, y: 3 });
    assert_eq!((pixel.r, pixel.g, pixel.b), (255, 0, 0));
    state.movie_frame = 0;
    state.movie_frame();
    state.movie_frame = -1;
    let movie = LoadedDrawable::create(2, 2, 2, 2).unwrap();
    context.with_frame(movie.lease().context_frame(1), || {
        context.set_background(crate::ui::rgb15(0, 0, 31));
        context.clear();
    });
    state.frame = Some(movie);
    state.movie_frame = 1;
    state.movie_frame();
    context.flush();
    assert_eq!(context.target().pixel(POINT { x: 0, y: 0 }).b, 255);
    state.movie_frame = -1;
    state.text_effect = b'T';
    render::text(
        &state,
        c"A",
        POINT { x: 20, y: 20 },
        crate::ui::rgb15(31, 31, 31),
        BLACK_COLOR,
    );
    context.flush();
    assert!((16..24).any(|y| (16..24).any(|x| {
        let pixel = context.target().pixel(POINT { x, y });
        (pixel.r, pixel.g, pixel.b) == (255, 255, 255)
    })));
    state.time_out = u32::MAX;
    assert_eq!(admit(&mut cx, &mut io, &mut state), Some(true));
    assert_eq!(*driver.sleeps.borrow(), vec![ONE_SECOND / 84]);
    pulse(&driver.input, KEY_MENU_SELECT);
    assert_eq!(admit(&mut cx, &mut io, &mut state), Some(true));
    assert_eq!((state.time_out, state.movie_frame), (0, -1));
    assert!(
        matches!(commands::advance(&mut cx, &mut io, &mut state), commands::Step::Call(path) if path.as_c_str() == c"missing.txt")
    );
    assert!(matches!(
        commands::advance(&mut cx, &mut io, &mut state),
        commands::Step::Input(false)
    ));
    pulse(&driver.input, KEY_MENU_CANCEL);
    assert_eq!(admit(&mut cx, &mut io, &mut state), Some(false));
    driver.input.clear_menu_state();
    cx.world.set_activity(CHECK_ABORT);
    assert_eq!(admit(&mut cx, &mut io, &mut state), Some(false));
    cx.world.set_activity(0);
    assert!(!show_slides(&mut cx, &mut io, None, false));
    assert!(show_slides(
        &mut cx,
        &mut io,
        Some(LoadedTextTable::new(vec![c"DIMS 40 20".to_owned()])),
        false
    ));
    assert_eq!(context.clip_rect().extent.width, 320);
    assert_eq!(
        context.font_lease().unwrap().leading(),
        original_font.leading()
    );
    driver.panic_on_run.set(true);
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        show_slides(&mut cx, &mut io, Some(LoadedTextTable::new(vec![])), false);
    }))
    .is_err());
    assert_eq!(context.clip_rect().extent.width, 320);
    assert_eq!(
        context.font_lease().unwrap().leading(),
        original_font.leading()
    );
    drop(state);
    drop(original_font);
    drop(context);
    drop(driver);
    fixture.finish(game);
}
