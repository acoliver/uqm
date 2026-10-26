//! Process clock choice, installed before initializing the game.

use std::panic::Location;
use std::sync::{Mutex, OnceLock};

use super::counter_abi::{ticks_to_time_count, ONE_SECOND};

pub enum TimeSource {
    Real,
    Replay(Mutex<ReplayClock>),
}

/// The only clock selection slot during the engine migration (adapter A6).
static TIME_SOURCE: OnceLock<TimeSource> = OnceLock::new();

#[derive(Default)]
pub struct ReplayClock {
    now: u32,
    reads: usize,
    steps: u64,
}

impl ReplayClock {
    #[track_caller]
    pub fn now_ticks(&mut self) -> u32 {
        self.reads += 1;
        assert!(
            self.reads < 10_000,
            "replay clock read 10,000 times without sleeping at {}",
            Location::caller()
        );
        self.now
    }

    pub fn advance_to(&mut self, wake: u32) {
        self.now = self.now.max(wake);
        self.reads = 0;
        self.steps += 1;
    }
}

pub fn install(replay: bool) {
    let source = if replay {
        TimeSource::Replay(Mutex::new(ReplayClock::default()))
    } else {
        TimeSource::Real
    };
    assert!(
        TIME_SOURCE.set(source).is_ok(),
        "time source installed twice"
    );
}

pub fn is_replay() -> bool {
    matches!(TIME_SOURCE.get(), Some(TimeSource::Replay(_)))
}

fn real_ms() -> u32 {
    // SAFETY: the SDL monotonic clock has no preconditions.
    unsafe { sdl2_sys::SDL_GetTicks() }
}

#[track_caller]
pub fn now_ticks() -> u32 {
    match TIME_SOURCE.get() {
        Some(TimeSource::Replay(clock)) => clock.lock().expect("replay clock poisoned").now_ticks(),
        Some(TimeSource::Real) | None => ticks_to_time_count(real_ms()),
    }
}

#[track_caller]
pub fn now_ms() -> u32 {
    if is_replay() {
        (u64::from(now_ticks()) * 1000 / u64::from(ONE_SECOND)) as u32
    } else {
        real_ms()
    }
}

pub fn advance_to(wake: u32) {
    let Some(TimeSource::Replay(clock)) = TIME_SOURCE.get() else {
        panic!("only replay sleeps may advance the clock");
    };
    clock
        .lock()
        .expect("replay clock poisoned")
        .advance_to(wake);
}

pub fn steps() -> u64 {
    match TIME_SOURCE.get() {
        Some(TimeSource::Replay(clock)) => clock.lock().expect("replay clock poisoned").steps,
        _ => 0,
    }
}

#[cfg(test)]
mod replay {
    use super::*;

    #[test]
    fn clock_moves_only_on_sleep_and_never_backwards() {
        let mut clock = ReplayClock::default();
        assert_eq!(clock.now_ticks(), 0);
        std::thread::sleep(std::time::Duration::from_millis(2));
        assert_eq!(clock.now_ticks(), 0);
        clock.advance_to(42);
        assert_eq!(clock.now_ticks(), 42);
        clock.advance_to(14);
        assert_eq!(clock.now_ticks(), 42);
        assert_eq!(clock.steps, 2);
    }

    #[test]
    #[should_panic(expected = "replay clock read 10,000 times without sleeping")]
    fn busy_clock_loop_fails_with_the_call_site() {
        let mut clock = ReplayClock::default();
        for _ in 0..10_000 {
            clock.now_ticks();
        }
    }

    #[test]
    fn sleep_resets_the_read_guard_even_when_already_due() {
        let mut clock = ReplayClock::default();
        for _ in 0..2 {
            for _ in 0..9_999 {
                clock.now_ticks();
            }
            clock.advance_to(0);
        }
    }
}
