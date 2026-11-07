//! Text-entry frames lend their actual context to synchronous callbacks.
use std::{ffi::CStr, ptr};
use uqm_rust::{
    engine::{
        battle::Cx,
        input::{frames::InputState, InputSession},
        observe::{Io, Unobserved},
        Game,
    },
    navigation::abi::TEXTENTRY_STATE,
    ui::getchar::text_entry_frame,
};

#[derive(Default)]
struct Callbacks {
    changed: usize,
    frames: usize,
}

unsafe fn reject_change(
    entry: *mut TEXTENTRY_STATE,
    cx: &mut Cx<'_>,
    _input: &InputSession,
) -> u32 {
    // SAFETY: the fixture retains these records and buffers until this frame returns.
    unsafe {
        let calls = &mut *(*entry).CbParam.cast::<Callbacks>();
        calls.changed += 1;
        assert_eq!((*entry).CursorPos, 1);
        assert_eq!(cx.world.data.game.glob_flags, 0);
        cx.world.data.game.glob_flags = 16;
    }
    0
}

unsafe fn observe_rejection(
    entry: *mut TEXTENTRY_STATE,
    cx: &mut Cx<'_>,
    _io: &mut Io<'_>,
    _input: &InputSession,
) -> u32 {
    // SAFETY: callbacks borrow the fixture's still-live entry and NUL-terminated buffer.
    unsafe {
        let calls = &mut *(*entry).CbParam.cast::<Callbacks>();
        calls.frames += 1;
        assert_eq!(cx.world.data.game.glob_flags, 16);
        assert_eq!((*entry).CursorPos, 0);
        assert_eq!((*entry).InsPt, (*entry).BaseStr);
        assert_eq!(CStr::from_ptr((*entry).BaseStr).to_bytes(), b"ab");
    }
    0
}

fn frame(key: usize, activity: u16) -> (u32, u32, Callbacks) {
    let input = InputSession::default();
    let mut pulse = InputState::default();
    pulse.menu[key] = 1;
    input.advance(pulse, pulse, 0);
    let mut game = Game::new();
    game.world.set_activity(activity);
    let mut calls = Callbacks::default();
    let mut text = [0; 16];
    text[0] = b'a' as std::ffi::c_char;
    text[1] = b'b' as std::ffi::c_char;
    let mut cache = [0; 16];
    // SAFETY: zero represents an absent callback, allocation or integer in this legacy entry.
    let mut entry: TEXTENTRY_STATE = unsafe { std::mem::zeroed() };
    entry.Initialized = 1;
    entry.BaseStr = text.as_mut_ptr();
    entry.InsPt = text.as_mut_ptr();
    entry.CacheStr = cache.as_mut_ptr();
    entry.MaxSize = text.len() as i32;
    entry.CbParam = ptr::from_mut(&mut calls).cast();
    entry.ChangeCallback = Some(reject_change);
    entry.FrameCallback = Some(observe_rejection);
    let mut observer = Unobserved;
    let mut io = Io {
        observer: &mut observer,
    };
    // SAFETY: the fixture lends one game context; enter removes its legacy publication.
    let result = unsafe {
        uqm_rust::app::bridge::with_game(&mut game, || {
            uqm_rust::app::bridge::enter(|cx| text_entry_frame(&mut entry, cx, &mut io, &input))
        })
    };
    (result, entry.Success, calls)
}

#[test]
#[serial_test::serial]
fn rejection_restores_cursor_before_frame_callback_using_the_same_world() {
    let (result, success, calls) = frame(8, 0);
    assert_eq!((result, success), (0, 0));
    assert_eq!((calls.changed, calls.frames), (1, 1));
}

#[test]
#[serial_test::serial]
fn abort_finishes_before_touching_text_or_callbacks() {
    let (result, success, calls) = frame(8, 0x4000);
    assert_eq!((result, success), (0, 0));
    assert_eq!((calls.changed, calls.frames), (0, 0));
}

#[test]
#[serial_test::serial]
fn select_and_cancel_finish_before_callbacks_with_the_original_success_value() {
    for (key, success) in [(9, 1), (20, 0)] {
        let (result, actual, calls) = frame(key, 0);
        assert_eq!((result, actual), (0, success));
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
    // SAFETY: the fixture lends its single game for this synchronous input session.
    let success = unsafe {
        uqm_rust::app::bridge::with_game(&mut game, || {
            uqm_rust::app::bridge::enter(|cx| {
                uqm_rust::ui::getchar::edit_text(cx, &mut io, &input, buffer, 0, &mut callbacks)
            })
        })
    };
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
