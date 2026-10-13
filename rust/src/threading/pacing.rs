//! Frame-pacing sleeps: `SleepThread` and `SleepThreadUntil`.
//!
//! With `RUST_OWNS_MAIN` the game runs on a single thread, so every sleep
//! doubles as the point where SDL events are pumped and the draw command
//! queue is flushed. Flushing renders, reads back, presents and (under the
//! automation harness) waits for an acknowledgement, which can take tens of
//! milliseconds. `sleep_until` therefore flushes first and only then reads
//! the clock, so the time spent flushing counts against the frame period
//! instead of being added to it.

/// Game time units per second (`ONE_SECOND` in `timelib.h`).
const ONE_SECOND: u64 = 840;

/// Converts game time units to milliseconds, saturating at `u32::MAX`.
fn ticks_to_ms(ticks: u32) -> u32 {
    u32::try_from(u64::from(ticks) * 1000 / ONE_SECOND).unwrap_or(u32::MAX)
}

/// The outside world a sleep depends on: game clock, async work, graphics
/// flushing and the actual sleeping.
pub trait PacingEnv {
    /// Run pending callbacks and due alarms.
    fn process_async(&mut self);
    /// Pump window events and flush the draw command queue.
    fn pump_and_flush(&mut self);
    /// Current game time in `ONE_SECOND` units.
    fn now_ticks(&self) -> u32;
    /// Milliseconds until the next async work is due; `u32::MAX` for none.
    fn async_wait_ms(&self) -> u32;
    /// Block the calling thread.
    fn sleep_ms(&mut self, ms: u32);
}

/// Sleep until the game clock reaches `wake_time`, servicing async work and
/// the display on every pass. The display is serviced even when `wake_time`
/// has already passed, since nothing else drains the draw command queue.
pub fn sleep_until(env: &mut impl PacingEnv, wake_time: u32) {
    loop {
        env.process_async();
        env.pump_and_flush();

        let now = env.now_ticks();
        if wake_time <= now {
            return;
        }

        let remaining_ms = ticks_to_ms(wake_time - now);
        env.sleep_ms(remaining_ms.min(env.async_wait_ms()));
    }
}

/// Service the display, then block for `duration` game time units.
pub fn sleep_for(env: &mut impl PacingEnv, duration: u32) {
    env.pump_and_flush();
    env.sleep_ms(ticks_to_ms(duration));
}

#[cfg(feature = "linked_c_archive")]
mod linked {
    use super::{sleep_for, sleep_until, PacingEnv};
    use crate::callback::ffi::{Async_process, Async_timeBeforeNextMs};
    use crate::threading::hibernate_thread;
    use std::time::Duration;

    extern "C" {
        fn GetTimeCounter() -> u32;
        fn TFB_ProcessEvents();
        fn TFB_FlushGraphics();
    }

    struct GameThread;

    impl PacingEnv for GameThread {
        fn process_async(&mut self) {
            Async_process();
        }

        fn pump_and_flush(&mut self) {
            // SAFETY: both are C entry points with no arguments, called from
            // the game thread that owns the display.
            unsafe {
                TFB_ProcessEvents();
                TFB_FlushGraphics();
            }
        }

        fn now_ticks(&self) -> u32 {
            // SAFETY: plain read of the SDL tick counter.
            unsafe { GetTimeCounter() }
        }

        fn async_wait_ms(&self) -> u32 {
            Async_timeBeforeNextMs()
        }

        fn sleep_ms(&mut self, ms: u32) {
            hibernate_thread(Duration::from_millis(u64::from(ms)));
        }
    }

    /// Sleep for `sleep_time` game time units after servicing the display.
    #[no_mangle]
    pub extern "C" fn SleepThread(sleep_time: u32) {
        sleep_for(&mut GameThread, sleep_time);
    }

    /// Sleep until the game clock reaches `wake_time`.
    #[no_mangle]
    pub extern "C" fn SleepThreadUntil(wake_time: u32) {
        sleep_until(&mut GameThread, wake_time);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Scripted clock. Flushes cost the scripted number of ticks (zero once
    /// the script runs out), sleeping advances the clock by the milliseconds
    /// requested, and a pending async wait is cleared once a sleep happened.
    struct FakeEnv {
        now: u32,
        flush_costs: Vec<u32>,
        async_wait_ms: u32,
        async_processed: u32,
        flushes: u32,
        sleeps: Vec<u32>,
    }

    impl FakeEnv {
        fn at(now: u32) -> Self {
            FakeEnv {
                now,
                flush_costs: Vec::new(),
                async_wait_ms: u32::MAX,
                async_processed: 0,
                flushes: 0,
                sleeps: Vec::new(),
            }
        }
    }

    impl PacingEnv for FakeEnv {
        fn process_async(&mut self) {
            self.async_processed += 1;
            // A due alarm fires during processing; none are left afterwards.
            if self.async_wait_ms != u32::MAX && !self.sleeps.is_empty() {
                self.async_wait_ms = u32::MAX;
            }
        }

        fn pump_and_flush(&mut self) {
            self.flushes += 1;
            if !self.flush_costs.is_empty() {
                self.now += self.flush_costs.remove(0);
            }
        }

        fn now_ticks(&self) -> u32 {
            self.now
        }

        fn async_wait_ms(&self) -> u32 {
            self.async_wait_ms
        }

        fn sleep_ms(&mut self, ms: u32) {
            self.sleeps.push(ms);
            // Round up so the fake clock never wakes early.
            self.now += (ms * 840).div_ceil(1000);
        }
    }

    #[test]
    fn ticks_convert_to_whole_milliseconds() {
        assert_eq!(ticks_to_ms(0), 0);
        assert_eq!(ticks_to_ms(1), 1);
        assert_eq!(ticks_to_ms(35), 41);
        assert_eq!(ticks_to_ms(840), 1000);
        assert_eq!(ticks_to_ms(u32::MAX), u32::MAX);
    }

    #[test]
    fn sleep_is_measured_from_the_clock_after_the_flush() {
        let mut env = FakeEnv::at(1000);
        env.flush_costs = vec![17];

        // One 35-tick (41 ms) frame period, of which the flush used 17 ticks.
        sleep_until(&mut env, 1035);

        assert_eq!(env.sleeps[0], 21, "18 ticks remain after the flush");
        assert!(env.now >= 1035);
    }

    #[test]
    fn a_wake_time_already_passed_still_flushes_once_and_returns() {
        let mut env = FakeEnv::at(1000);

        sleep_until(&mut env, 500);

        assert_eq!(env.flushes, 1);
        assert_eq!(env.async_processed, 1);
        assert!(env.sleeps.is_empty());
    }

    #[test]
    fn a_flush_that_overruns_the_frame_returns_without_sleeping() {
        let mut env = FakeEnv::at(1000);
        env.flush_costs = vec![40];

        sleep_until(&mut env, 1035);

        assert_eq!(env.flushes, 1);
        assert!(env.sleeps.is_empty());
    }

    #[test]
    fn a_pending_alarm_shortens_the_sleep() {
        let mut env = FakeEnv::at(1000);
        env.async_wait_ms = 5;

        sleep_until(&mut env, 1035);

        // 5 ms for the alarm (5 ticks of clock), leaving 30 ticks (35 ms).
        assert_eq!(env.sleeps, vec![5, 35]);
        assert_eq!(env.async_processed, 3);
    }

    #[test]
    fn an_alarm_later_than_the_wake_time_does_not_extend_the_sleep() {
        let mut env = FakeEnv::at(1000);
        env.async_wait_ms = 500;

        sleep_until(&mut env, 1035);

        assert_eq!(env.sleeps, vec![41]);
    }

    #[test]
    fn an_overdue_alarm_wakes_the_loop_without_sleeping() {
        let mut env = FakeEnv::at(1000);
        env.async_wait_ms = 0;

        sleep_until(&mut env, 1035);

        assert_eq!(env.sleeps, vec![0, 41]);
    }

    #[test]
    fn sleep_for_flushes_then_sleeps_the_whole_duration() {
        let mut env = FakeEnv::at(1000);
        env.flush_costs = vec![17];

        sleep_for(&mut env, 420);

        assert_eq!(env.flushes, 1);
        assert_eq!(env.sleeps, vec![500]);
    }
}
