//! The game clock port and its two implementations.
//!
//! `OsClock` reads the monotonic OS clock. `ReplayClock` advances only when a
//! replayed sleep reaches its wake time, so a replay never depends on how fast
//! the host runs.

use std::panic::Location;
use std::sync::Mutex;
use std::time::Instant;

use crate::time::counter_abi::{ticks_to_time_count, ONE_SECOND};

pub trait Clock: Send + Sync {
    /// Game time in `ONE_SECOND` units.
    fn now_ticks(&self, caller: &'static Location<'static>) -> u32;
    fn now_ms(&self, caller: &'static Location<'static>) -> u32;
    /// Move a replay clock to `wake`; the OS clock cannot be moved.
    fn advance_to(&self, wake: u32);
    fn is_replay(&self) -> bool;
    /// Replayed sleeps taken so far.
    fn steps(&self) -> u64;
}

/// Milliseconds since the clock was created, wrapping at `u32` like the SDL tick counter.
pub struct OsClock {
    epoch: Instant,
}

impl OsClock {
    pub fn new() -> Self {
        Self {
            epoch: Instant::now(),
        }
    }
}

impl Default for OsClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for OsClock {
    fn now_ticks(&self, caller: &'static Location<'static>) -> u32 {
        ticks_to_time_count(self.now_ms(caller))
    }

    fn now_ms(&self, _caller: &'static Location<'static>) -> u32 {
        self.epoch.elapsed().as_millis() as u32
    }

    fn advance_to(&self, _wake: u32) {
        panic!("only replay sleeps may advance the clock");
    }

    fn is_replay(&self) -> bool {
        false
    }

    fn steps(&self) -> u64 {
        0
    }
}

#[derive(Default)]
struct ReplayState {
    now: u32,
    reads: usize,
    steps: u64,
}

#[derive(Default)]
pub struct ReplayClock(Mutex<ReplayState>);

impl ReplayClock {
    fn state(&self) -> std::sync::MutexGuard<'_, ReplayState> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl Clock for ReplayClock {
    fn now_ticks(&self, caller: &'static Location<'static>) -> u32 {
        let mut state = self.state();
        state.reads += 1;
        assert!(
            state.reads < 10_000,
            "replay clock read 10,000 times without sleeping at {caller}"
        );
        state.now
    }

    fn now_ms(&self, caller: &'static Location<'static>) -> u32 {
        (u64::from(self.now_ticks(caller)) * 1000 / u64::from(ONE_SECOND)) as u32
    }

    fn advance_to(&self, wake: u32) {
        let mut state = self.state();
        state.now = state.now.max(wake);
        state.reads = 0;
        state.steps += 1;
    }

    fn is_replay(&self) -> bool {
        true
    }

    fn steps(&self) -> u64 {
        self.state().steps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_moves_only_on_sleep_and_never_backwards() {
        let clock = ReplayClock::default();
        assert_eq!(clock.now_ticks(Location::caller()), 0);
        std::thread::sleep(std::time::Duration::from_millis(2));
        assert_eq!(clock.now_ticks(Location::caller()), 0);
        clock.advance_to(42);
        assert_eq!(clock.now_ticks(Location::caller()), 42);
        clock.advance_to(14);
        assert_eq!(clock.now_ticks(Location::caller()), 42);
        assert_eq!(clock.steps(), 2);
    }

    #[test]
    #[should_panic(expected = "replay clock read 10,000 times without sleeping")]
    fn busy_clock_loop_fails_with_the_call_site() {
        let clock = ReplayClock::default();
        for _ in 0..10_000 {
            clock.now_ticks(Location::caller());
        }
    }

    #[test]
    fn sleep_resets_the_read_guard_even_when_already_due() {
        let clock = ReplayClock::default();
        for _ in 0..2 {
            for _ in 0..9_999 {
                clock.now_ticks(Location::caller());
            }
            clock.advance_to(0);
        }
    }

    #[test]
    #[should_panic(expected = "only replay sleeps may advance the clock")]
    fn the_os_clock_cannot_be_advanced() {
        OsClock::new().advance_to(1);
    }
}
