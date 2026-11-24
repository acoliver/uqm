use super::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use uqm_engine::graphics::{geometry::Extent, geometry::Point, geometry::Rect};
use uqm_engine::{
    battle::{input::InputContext, Battle},
    input::{frames::InputState, Frame, InputControls, InputRuntime},
    observe::{Observer, Unobserved},
    Game,
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
    fn observer(&self) -> Box<dyn Observer> {
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
    if !uqm_engine::test_support::native_process::own_graphics_process("front_end::intro::render_tests::commands_draw_retained_frames_and_preserve_wait_call_skip_and_restore_order") { return; }
    let mut game = Game::new();
    let fixture =
        uqm_engine::test_support::campaign_display::CampaignDisplay::install(&mut game).unwrap();
    let context = ContextLease::require(game.kernel.status_context);
    game.kernel.screen_context = game.kernel.status_context;
    context.set_frame(ContextFrame::retain(game.kernel.screen));
    let screen = Rect {
        corner: Point { x: 0, y: 0 },
        extent: Extent {
            width: 320,
            height: 240,
        },
    };
    context.set_clip(screen);
    context.set_font(&uqm_engine::graphics::font::FontLease::retain(
        game.kernel.tiny_font.unwrap(),
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
        uqm_engine::math::random,
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
    let pixel = context.target().pixel(Point { x: 4, y: 3 });
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
    assert_eq!(context.target().pixel(Point { x: 0, y: 0 }).b, 255);
    state.movie_frame = -1;
    state.text_effect = b'T';
    render::text(
        &state,
        c"A",
        Point { x: 20, y: 20 },
        crate::ui::rgb15(31, 31, 31),
        BLACK_COLOR,
    );
    context.flush();
    assert!((16..24).any(|y| (16..24).any(|x| {
        let pixel = context.target().pixel(Point { x, y });
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

#[test]
#[serial_test::serial]
fn command_matrix_renders_text_transitions_and_retains_generated_sis_frames() {
    if !uqm_engine::test_support::native_process::own_graphics_process("front_end::intro::render_tests::command_matrix_renders_text_transitions_and_retains_generated_sis_frames") { return; }
    let mut game = Game::new();
    let fixture = uqm_engine::test_support::campaign_display::CampaignDisplay::install_prepared(
        &mut game,
        |_| {},
        |game| {
            let context = ContextLease::require(game.kernel.status_context);
            game.kernel.screen_context = game.kernel.status_context;
            context.set_frame(ContextFrame::retain(game.kernel.screen));
            context.set_clip(Rect {
                corner: Point { x: 0, y: 0 },
                extent: Extent {
                    width: 320,
                    height: 240,
                },
            });
            let offscreen =
                uqm_engine::graphics::context::LoadedContext::create(c"intro SIS fixture");
            game.kernel.off_screen_context = Some(offscreen.context());
            let driver = Rc::new(Driver::default());
            game.kernel.install_input_runtime(driver.clone());
            game.world.data.game.glob_flags = 8;
            game.world.data.flagship.drive_slots[0] = 0;
            game.world.data.flagship.jet_slots[0] = 0;
            game.world.data.flagship.module_slots[0] = 0;
            let mut battle = Battle::new();
            let mut observer = Unobserved;
            let mut io = Io {
                observer: &mut observer,
            };
            let mut cx = Cx::new(
                &mut battle,
                &mut game.world,
                &mut game.kernel,
                uqm_engine::math::random,
            );
            let mut state = PresentationState::new(
                LoadedTextTable::new(vec![
                    c"TVA C".to_owned(),
                    c"TE N".to_owned(),
                    c"TC 00ff00".to_owned(),
                    c"TBC 000000".to_owned(),
                    c"TFC ff0000".to_owned(),
                    c"FONT 0".to_owned(),
                    c"TEXT 10 10 A".to_owned(),
                    c"TFI A".to_owned(),
                    c"TFO".to_owned(),
                    c"SAVEBG".to_owned(),
                    c"CLS".to_owned(),
                    c"RESTBG".to_owned(),
                    c"FONT 9".to_owned(),
                    c"LINE 1".to_owned(),
                    c"MOVIE 0 2 1".to_owned(),
                    c"MOVIE 24 0 1".to_owned(),
                    c"SYNC 20".to_owned(),
                    c"DSYNC 30".to_owned(),
                    c"RESYNC".to_owned(),
                    c"FTW 0".to_owned(),
                    c"FTB 0".to_owned(),
                    c"FTC 0".to_owned(),
                    c"WAIT 20".to_owned(),
                    c"ANI missing.ani".to_owned(),
                    c"DRAW SIS 40 40".to_owned(),
                    c"DRAW".to_owned(),
                    c"MUSIC missing.ogg".to_owned(),
                    c"FONT 0 missing-font".to_owned(),
                    c"TEXT 10 10 A".to_owned(),
                    c"".to_owned(),
                ]),
                context.clone(),
                false,
            );
            state.fonts[0] =
                uqm_engine::graphics::font::LoadedFont::load(&cx.kernel.content(), c"fixture.tiny");
            assert!(matches!(
                commands::advance(&mut cx, &mut io, &mut state),
                commands::Step::Input(true)
            ));
            assert_eq!(
                (
                    state.movie_frame,
                    state.movie_end_frame,
                    state.interframe_delay
                ),
                (0, 1, 35)
            );
            assert!(state.time_out_on_skip);
            assert!(matches!(
                commands::advance(&mut cx, &mut io, &mut state),
                commands::Step::Input(true)
            ));
            assert_eq!(state.last_sync_time, state.start_time.wrapping_add(16));
            assert!(!state.time_out_on_skip);
            assert!(matches!(
                commands::advance(&mut cx, &mut io, &mut state),
                commands::Step::Input(true)
            ));
            assert_eq!(state.time_out, state.last_sync_time.wrapping_add(25));
            for _ in 0..3 {
                assert!(matches!(
                    commands::advance(&mut cx, &mut io, &mut state),
                    commands::Step::Input(true)
                ));
                assert!(!state.time_out_on_skip);
            }
            assert!(matches!(
                commands::advance(&mut cx, &mut io, &mut state),
                commands::Step::Input(true)
            ));
            assert!(state.time_out_on_skip);
            assert!(matches!(
                commands::advance(&mut cx, &mut io, &mut state),
                commands::Step::Input(false)
            ));
            assert!(state.context.font_lease().is_none());
            assert!(state.sis.is_some());
            context.flush();
            let pixel = context.target().pixel(Point { x: 40, y: 40 });
            assert_eq!((pixel.r, pixel.g, pixel.b), (255, 255, 255));
            render::draw(&cx, &mut state, b"SIS 50 50 256 90");
            context.flush();
            assert_eq!(context.target().pixel(Point { x: 50, y: 50 }).r, 255);
            render::draw(&cx, &mut state, b"SIS 50 50 256 90");
            state.frame = Some(LoadedDrawable::create(2, 2, 2, 1).unwrap());
            render::draw(&cx, &mut state, b"0 60 60 256 90");
            drop(state);
            drop(context);
            drop(driver);
            game.kernel.off_screen_context = game.kernel.status_context;
        },
    )
    .unwrap();
    fixture.finish(game);
}
