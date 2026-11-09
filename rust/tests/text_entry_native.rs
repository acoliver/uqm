//! Text-entry frames lend their actual context to synchronous callbacks.
use uqm_rust::{
    engine::{
        battle::Cx,
        input::{frames::InputState, InputSession},
        observe::{Io, Unobserved},
        Game,
    },
    ui::getchar::{Alphabet, Editor},
};

#[derive(Default)]
struct Callbacks {
    changed: usize,
    frames: usize,
}

fn frame(key: usize, activity: u16) -> (bool, bool, Callbacks) {
    let input = InputSession::default();
    let mut pulse = InputState::default();
    pulse.menu[key] = 1;
    input.advance(pulse, pulse, 0);
    let mut game = Game::new();
    game.world.set_activity(activity);
    let mut battle = uqm_rust::engine::battle::Battle::new();
    let mut cx = Cx::new(
        &mut battle,
        &mut game.world,
        &mut game.kernel,
        uqm_rust::math::random,
    );
    let mut callbacks = BorrowedCallbacks(Callbacks::default());
    let mut text = *b"ab\0\0\0\0\0\0";
    let mut editor = Editor::new(&mut text, 8, 0);
    let mut observer = Unobserved;
    let mut io = Io {
        observer: &mut observer,
    };
    let result = editor.frame(
        &mut cx,
        &mut io,
        &input,
        &Alphabet::default(),
        &mut callbacks,
    );
    (result, editor.success(), callbacks.0)
}

#[test]
#[serial_test::serial]
fn rejection_restores_cursor_before_frame_callback_using_the_same_world() {
    let (result, success, calls) = frame(8, 0);
    assert_eq!((result, success), (false, false));
    assert_eq!((calls.changed, calls.frames), (1, 1));
}

#[test]
#[serial_test::serial]
fn abort_finishes_before_editing_or_callbacks() {
    let (result, success, calls) = frame(8, 0x4000);
    assert_eq!((result, success), (false, false));
    assert_eq!((calls.changed, calls.frames), (0, 0));
}

#[test]
#[serial_test::serial]
fn select_and_cancel_finish_before_callbacks_with_the_original_success_value() {
    for (key, success) in [(9, true), (20, false)] {
        let (result, actual, calls) = frame(key, 0);
        assert_eq!((result, actual), (false, success));
        assert_eq!((calls.changed, calls.frames), (0, 0));
    }
}

struct BorrowedCallbacks(Callbacks);

impl uqm_rust::ui::getchar::TextEntryCallbacks for BorrowedCallbacks {
    fn changed(
        &mut self,
        entry: uqm_rust::ui::getchar::TextEntryView<'_>,
        cx: &mut Cx<'_>,
        _input: &InputSession,
    ) -> bool {
        self.0.changed += 1;
        assert_eq!(entry.cursor, 1);
        assert_eq!(entry.text, c"ab");
        cx.world.data.game.glob_flags = 16;
        false
    }

    fn frame(
        &mut self,
        entry: uqm_rust::ui::getchar::TextEntryView<'_>,
        cx: &mut Cx<'_>,
        _io: &mut Io<'_>,
        _input: &InputSession,
    ) -> bool {
        self.0.frames += 1;
        assert_eq!(cx.world.data.game.glob_flags, 16);
        assert_eq!(entry.cursor, 0);
        assert_eq!(entry.text, c"ab");
        false
    }
}

struct OneFrame(InputSession);

impl uqm_rust::engine::input::InputRuntime for OneFrame {
    fn input(&self) -> &InputSession {
        &self.0
    }

    fn observer(
        &self,
    ) -> Box<dyn uqm_rust::engine::observe::Observer<Option<uqm_rust::engine::observe::CampaignView>>>
    {
        Box::new(Unobserved)
    }

    fn sleep(&self, _: &mut uqm_engine::battle::Cx<'_>, _: u32) {
        panic!("unexpected pacing");
    }
    fn sleep_until(&self, _: &mut uqm_engine::battle::Cx<'_>, _: u32) {
        panic!("unexpected pacing");
    }
    fn yield_poll(&self, _: &mut Cx<'_>) {
        panic!("unexpected yield");
    }
    fn flush(&self) {
        self.input().clear_menu_state();
    }
    fn confirm_exit(&self, _: &mut uqm_engine::battle::Cx<'_>) -> bool {
        panic!("unexpected confirmation");
    }

    fn update(&self, _cx: &mut Cx<'_>) {}

    fn run(
        &self,
        cx: &mut Cx<'_>,
        io: &mut Io<'_>,
        reset: bool,
        frame: &mut dyn uqm_rust::engine::input::Frame,
    ) {
        assert!(reset);
        assert!(!frame.frame(cx, io));
    }

    fn battle_input(&self, _input: &uqm_rust::engine::battle::input::InputContext) -> u8 {
        unreachable!("text entry does not poll a battle ship")
    }
}

fn borrowed_frame(key: usize, activity: u16, buffer: &mut [u8]) -> (bool, Callbacks) {
    let input = InputSession::default();
    let mut pulse = InputState::default();
    pulse.menu[key] = 1;
    input.advance(pulse, pulse, 0);
    let mut game = Game::new();
    game.world.set_activity(activity);
    game.kernel
        .install_input_runtime(std::rc::Rc::new(OneFrame(input.clone())));
    let mut callbacks = BorrowedCallbacks(Callbacks::default());
    let mut observer = Unobserved;
    let mut io = Io {
        observer: &mut observer,
    };
    let mut battle = uqm_rust::engine::battle::Battle::new();
    let mut cx = Cx::new(
        &mut battle,
        &mut game.world,
        &mut game.kernel,
        uqm_rust::math::random,
    );
    let success = uqm_rust::ui::getchar::edit_text(
        &mut cx,
        &mut io,
        &input,
        buffer.len(),
        buffer,
        0,
        &mut callbacks,
    );
    (success, callbacks.0)
}

#[test]
#[serial_test::serial]
fn borrowed_callbacks_keep_the_buffer_and_menu_borrow_through_rollback() {
    let mut buffer = *b"ab\0\0\0\0\0\0";
    let (success, calls) = borrowed_frame(8, 0, &mut buffer);
    assert!(!success);
    assert_eq!((calls.changed, calls.frames), (1, 1));
    assert_eq!(buffer, *b"ab\0\0\0\0\0\0");
}

#[test]
#[serial_test::serial]
fn borrowed_editor_finishes_select_and_cancel_before_callbacks() {
    for (key, expected) in [(9, true), (20, false)] {
        let mut buffer = *b"ab\0\0\0\0\0\0";
        let (success, calls) = borrowed_frame(key, 0, &mut buffer);
        assert_eq!(success, expected);
        assert_eq!((calls.changed, calls.frames), (0, 0));
    }
}

#[test]
#[serial_test::serial]
fn borrowed_editor_abort_does_not_inspect_the_buffer_or_run_callbacks() {
    let (success, calls) = borrowed_frame(8, 0x4000, &mut []);
    assert!(!success);
    assert_eq!((calls.changed, calls.frames), (0, 0));
}
