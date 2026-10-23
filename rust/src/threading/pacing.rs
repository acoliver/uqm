//! Frame-pacing sleeps: `SleepThread` and `SleepThreadUntil`.
//!
//! With `RUST_OWNS_MAIN` the game runs on a single thread, so every sleep
//! doubles as the point where SDL events are pumped and the draw command
//! queue is flushed. Flushing renders, reads back, presents and (under the
//! automation harness) waits for an acknowledgement, which can take tens of
//! milliseconds. `sleep_until` therefore flushes first and only then reads
//! the clock, so the time spent flushing counts against the frame period
//! instead of being added to it.
//!
//! Every sleep inside `sleep_until` is timed against the monotonic clock.
//! Each window of [`WINDOW_SLEEPS`] sleeps is summarised on stderr as one
//! `pacing:` line, so late wake-ups show up in any log of a run.

/// Game time units per second (`ONE_SECOND` in `timelib.h`).
const ONE_SECOND: u64 = 840;

/// Converts game time units to milliseconds, saturating at `u32::MAX`.
fn ticks_to_ms(ticks: u32) -> u32 {
    u32::try_from(u64::from(ticks) * 1000 / ONE_SECOND).unwrap_or(u32::MAX)
}

/// Sleeps summarised by each `pacing:` report line.
pub const WINDOW_SLEEPS: usize = 240;

/// Timing of one window of sleeps, kept in fixed-size storage so recording
/// never allocates. All durations are microseconds.
pub struct PacingStats {
    sleeps: usize,
    requested_us: u64,
    actual_us: u64,
    flush_us: u64,
    overshoots_us: [u32; WINDOW_SLEEPS],
}

impl PacingStats {
    pub const fn new() -> Self {
        PacingStats {
            sleeps: 0,
            requested_us: 0,
            actual_us: 0,
            flush_us: 0,
            overshoots_us: [0; WINDOW_SLEEPS],
        }
    }

    /// Adds time spent pumping events and flushing graphics.
    pub fn record_flush(&mut self, elapsed_us: u64) {
        self.flush_us += elapsed_us;
    }

    /// Records one sleep. Returns the report line once the window is full
    /// and starts a new window.
    pub fn record_sleep(&mut self, requested_us: u64, actual_us: u64) -> Option<String> {
        let overshoot_us = actual_us.saturating_sub(requested_us);
        self.overshoots_us[self.sleeps] = u32::try_from(overshoot_us).unwrap_or(u32::MAX);
        self.sleeps += 1;
        self.requested_us += requested_us;
        self.actual_us += actual_us;
        if self.sleeps < WINDOW_SLEEPS {
            return None;
        }

        let mut sorted = self.overshoots_us;
        sorted.sort_unstable();
        let report = format!(
            "pacing: sleeps={} requested_ms={} actual_ms={} overshoot_p50_ms={} \
             overshoot_p90_ms={} overshoot_max_ms={} flush_ms={}",
            WINDOW_SLEEPS,
            format_ms(self.requested_us),
            format_ms(self.actual_us),
            format_ms(u64::from(nearest_rank(&sorted, 50))),
            format_ms(u64::from(nearest_rank(&sorted, 90))),
            format_ms(u64::from(sorted[WINDOW_SLEEPS - 1])),
            format_ms(self.flush_us),
        );
        *self = PacingStats::new();
        Some(report)
    }
}

impl Default for PacingStats {
    fn default() -> Self {
        PacingStats::new()
    }
}

/// Nearest-rank percentile of an ascending, full window.
fn nearest_rank(sorted: &[u32; WINDOW_SLEEPS], percent: usize) -> u32 {
    sorted[(WINDOW_SLEEPS * percent).div_ceil(100) - 1]
}

/// Microseconds as milliseconds with one decimal, truncated.
fn format_ms(us: u64) -> String {
    format!("{}.{}", us / 1000, us % 1000 / 100)
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
    /// Monotonic clock in microseconds, independent of the game clock.
    fn monotonic_us(&self) -> u64;
    /// Timing statistics for the current window of sleeps.
    fn stats(&mut self) -> &mut PacingStats;
    /// Emit one `pacing:` report line.
    fn report(&mut self, line: &str);
}

/// Sleep until the game clock reaches `wake_time`, servicing async work and
/// the display on every pass. The display is serviced even when `wake_time`
/// has already passed, since nothing else drains the draw command queue.
pub fn sleep_until(env: &mut impl PacingEnv, wake_time: u32) {
    loop {
        env.process_async();
        let flush_start_us = env.monotonic_us();
        env.pump_and_flush();
        let flush_us = env.monotonic_us().saturating_sub(flush_start_us);
        env.stats().record_flush(flush_us);

        let now = env.now_ticks();
        if wake_time <= now {
            return;
        }

        let remaining_ms = ticks_to_ms(wake_time - now);
        let sleep_ms = remaining_ms.min(env.async_wait_ms());
        let sleep_start_us = env.monotonic_us();
        env.sleep_ms(sleep_ms);
        let actual_us = env.monotonic_us().saturating_sub(sleep_start_us);
        if let Some(line) = env
            .stats()
            .record_sleep(u64::from(sleep_ms) * 1000, actual_us)
        {
            env.report(&line);
        }
    }
}

/// Service the display, then block for `duration` game time units.
pub fn sleep_for(env: &mut impl PacingEnv, duration: u32) {
    env.pump_and_flush();
    env.sleep_ms(ticks_to_ms(duration));
}

#[cfg(any(not(test), feature = "linked_c_archive"))]
pub(crate) mod linked {
    use super::{sleep_for, sleep_until, PacingEnv, PacingStats};
    use crate::callback::ffi::{Async_process, Async_timeBeforeNextMs};
    #[cfg(target_os = "macos")]
    use crate::threading::critical_timer::CriticalTimer;
    #[cfg(not(target_os = "macos"))]
    use crate::threading::hibernate_thread;
    use std::sync::{Mutex, MutexGuard, OnceLock};
    #[cfg(not(target_os = "macos"))]
    use std::time::Duration;
    use std::time::Instant;

    /// Only the game thread sleeps, so the lock is never contended.
    static STATS: Mutex<PacingStats> = Mutex::new(PacingStats::new());
    static CLOCK_ORIGIN: OnceLock<Instant> = OnceLock::new();
    /// macOS coalesces plain sleeps into ~16 ms steps on loaded hosts; see
    /// `threading::critical_timer`. One queue lives as long as the process.
    #[cfg(target_os = "macos")]
    static SLEEP_TIMER: OnceLock<CriticalTimer> = OnceLock::new();

    extern "C" {
        fn GetTimeCounter() -> u32;
        fn TFB_ProcessEvents();
        fn TFB_FlushGraphics();
    }

    struct GameThread {
        stats: MutexGuard<'static, PacingStats>,
    }

    impl GameThread {
        fn enter() -> Self {
            CLOCK_ORIGIN.get_or_init(Instant::now);
            GameThread {
                stats: STATS.lock().expect("pacing stats lock poisoned"),
            }
        }
    }

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
            #[cfg(target_os = "macos")]
            SLEEP_TIMER
                .get_or_init(|| CriticalTimer::new().expect("create pacing critical timer queue"))
                .sleep(std::time::Duration::from_millis(u64::from(ms)))
                .expect("wait on pacing critical timer");
            #[cfg(not(target_os = "macos"))]
            hibernate_thread(Duration::from_millis(u64::from(ms)));
        }

        fn monotonic_us(&self) -> u64 {
            let origin = CLOCK_ORIGIN.get_or_init(Instant::now);
            u64::try_from(origin.elapsed().as_micros()).unwrap_or(u64::MAX)
        }

        fn stats(&mut self) -> &mut PacingStats {
            &mut self.stats
        }

        fn report(&mut self, line: &str) {
            eprintln!("{line}");
        }
    }

    /// Sleep for `sleep_time` game time units after servicing the display.
    #[no_mangle]
    pub extern "C" fn SleepThread(sleep_time: u32) {
        sleep_for(&mut GameThread::enter(), sleep_time);
    }

    /// Sleep until the game clock reaches `wake_time`.
    #[no_mangle]
    pub extern "C" fn SleepThreadUntil(wake_time: u32) {
        sleep_until(&mut GameThread::enter(), wake_time);
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
        /// Monotonic clock; advances with every flush and sleep.
        mono_us: u64,
        /// Extra microseconds each successive sleep oversleeps by.
        late_us: Vec<u64>,
        stats: PacingStats,
        reports: Vec<String>,
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
                mono_us: 0,
                late_us: Vec::new(),
                stats: PacingStats::new(),
                reports: Vec::new(),
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
                let cost = self.flush_costs.remove(0);
                self.now += cost;
                self.mono_us += u64::from(cost) * 1000;
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
            let late_us = if self.late_us.is_empty() {
                0
            } else {
                self.late_us.remove(0)
            };
            self.mono_us += u64::from(ms) * 1000 + late_us;
        }

        fn monotonic_us(&self) -> u64 {
            self.mono_us
        }

        fn stats(&mut self) -> &mut PacingStats {
            &mut self.stats
        }

        fn report(&mut self, line: &str) {
            self.reports.push(line.to_string());
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

    #[test]
    fn a_full_window_reports_literal_percentiles() {
        let mut stats = PacingStats::new();
        let mut report = None;
        // Sleep n (1..=240) requests 10 ms and wakes n ms late.
        for late_ms in 1..=WINDOW_SLEEPS as u64 {
            assert!(report.is_none(), "reported before the window was full");
            report = stats.record_sleep(10_000, 10_000 + late_ms * 1000);
        }

        assert_eq!(
            report.as_deref(),
            Some(
                "pacing: sleeps=240 requested_ms=2400.0 actual_ms=31320.0 \
                 overshoot_p50_ms=120.0 overshoot_p90_ms=216.0 \
                 overshoot_max_ms=240.0 flush_ms=0.0"
            )
        );
    }

    #[test]
    fn early_wakeups_count_as_zero_overshoot_and_the_window_restarts() {
        let mut stats = PacingStats::new();
        stats.record_flush(1_500);
        stats.record_flush(2_250);
        let mut first = None;
        for _ in 0..WINDOW_SLEEPS {
            first = stats.record_sleep(5_000, 4_000);
        }

        assert_eq!(
            first.as_deref(),
            Some(
                "pacing: sleeps=240 requested_ms=1200.0 actual_ms=960.0 \
                 overshoot_p50_ms=0.0 overshoot_p90_ms=0.0 \
                 overshoot_max_ms=0.0 flush_ms=3.7"
            )
        );
        assert!(stats.record_sleep(1_000, 1_000).is_none(), "new window");
    }

    #[test]
    fn sleep_until_measures_each_sleep_and_the_flush() {
        let mut env = FakeEnv::at(1000);
        env.flush_costs = vec![2];
        env.late_us = vec![7_000];

        sleep_until(&mut env, 1035);

        // The 2-tick flush leaves 33 ticks (39 ms), which woke 7 ms late.
        assert_eq!(env.sleeps, vec![39]);
        assert_eq!(env.stats.sleeps, 1);
        assert_eq!(env.stats.requested_us, 39_000);
        assert_eq!(env.stats.actual_us, 46_000);
        assert_eq!(env.stats.flush_us, 2_000);
        assert_eq!(env.stats.overshoots_us[0], 7_000);
        assert!(env.reports.is_empty());
    }

    #[test]
    fn sleep_until_emits_one_report_per_full_window() {
        let mut env = FakeEnv::at(1000);
        let mut wake = 1000;
        for _ in 0..WINDOW_SLEEPS {
            wake += 35;
            env.late_us.push(3_000);
            sleep_until(&mut env, wake);
        }

        assert_eq!(env.reports.len(), 1);
        assert!(env.reports[0].starts_with("pacing: sleeps=240 "));
        assert!(env.reports[0].contains(" overshoot_p50_ms=3.0 "));
        assert_eq!(env.stats.sleeps, 0, "window restarted after the report");
    }
}
