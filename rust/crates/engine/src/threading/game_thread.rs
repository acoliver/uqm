//! The game thread: it owns SDL dispatch and the draw queue, and its
//! cooperative yield pumps window events (rust_thrcommon.c).
use super::{init_thread_system, task_switch, uninit_thread_system};
use std::sync::Mutex;
use std::thread::{self, ThreadId};

/// The game thread, which owns SDL dispatch and the draw queue.
static GAME_THREAD: Mutex<Option<ThreadId>> = Mutex::new(None);

fn on_game_thread() -> bool {
    *GAME_THREAD
        .lock()
        .unwrap_or_else(|error| panic!("thread owner lock: {error}"))
        == Some(thread::current().id())
}

/// Declare the calling thread the game thread (C: `InitThreadSystem`).
pub fn init_game_thread() {
    // a second initialization is a broken startup order
    init_thread_system().unwrap_or_else(|error| panic!("thread system: {error:?}"));
    *GAME_THREAD
        .lock()
        .unwrap_or_else(|error| panic!("thread owner lock: {error}")) =
        Some(thread::current().id());
}

/// Retire the thread system once every worker has stopped
/// (C: `UnInitThreadSystem`).
pub fn uninit_game_thread() {
    uninit_thread_system();
}

/// C: `TaskSwitch`: let pending window events run on the game thread, then
/// yield. It does not flush: the draw queue's own waits yield here.
pub fn yield_task() {
    if on_game_thread() {
        crate::graphics::sdl::process_events();
    }
    task_switch();
}

pub fn yield_game_thread() {
    assert!(
        on_game_thread(),
        "event polling requires the initialized game thread"
    );
    yield_task();
}
