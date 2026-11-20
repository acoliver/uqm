//! The game thread, its cooperative yields and pacing.

pub mod affinity;
mod game_thread;
pub mod pacing;
pub use game_thread::{init_game_thread, uninit_game_thread, yield_game_thread, yield_task};

use std::time::Duration;

/// Yield execution to other threads (C: `TaskSwitch` -> `SDL_Delay(1)`).
pub fn task_switch() {
    crate::host::sleep(Duration::from_millis(1));
}

/// Sleep the current thread for `duration`.
pub fn hibernate_thread(duration: Duration) {
    crate::host::sleep(duration);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hibernation_never_returns_early() {
        let start = std::time::Instant::now();
        hibernate_thread(Duration::from_millis(50));
        assert!(start.elapsed() >= Duration::from_millis(40));
        let start = std::time::Instant::now();
        task_switch();
        assert!(start.elapsed() >= Duration::from_millis(1));
    }
}
