//! The game thread: it owns SDL dispatch and the draw queue, and its
//! cooperative yield pumps window events (rust_thrcommon.c).
use super::task_switch;
use crate::host::thread_number;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// The game thread, which owns SDL dispatch and the draw queue.
static GAME_THREAD: Mutex<Option<u64>> = Mutex::new(None);
/// Between `init_game_thread` and `uninit_game_thread`.
static INITIALIZED: AtomicBool = AtomicBool::new(false);

fn on_game_thread() -> bool {
    *GAME_THREAD
        .lock()
        .unwrap_or_else(|error| panic!("thread owner lock: {error}"))
        == Some(thread_number())
}

/// Declare the calling thread the game thread (C: `InitThreadSystem`).
pub fn init_game_thread() {
    assert!(
        !INITIALIZED.swap(true, Ordering::SeqCst),
        "thread system already initialized"
    );
    *GAME_THREAD
        .lock()
        .unwrap_or_else(|error| panic!("thread owner lock: {error}")) = Some(thread_number());
}

/// Retire the thread system once every worker has stopped
/// (C: `UnInitThreadSystem`).
pub fn uninit_game_thread() {
    INITIALIZED.store(false, Ordering::SeqCst);
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
