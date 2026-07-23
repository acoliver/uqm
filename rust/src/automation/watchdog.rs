//! Pure watchdog reducer — inclusive-limit enforcement.
//!
//! Implements execution-contract §2.1 and §2.2 exactly: applicable counter
//! checked-add and store before comparison; equality is terminal and admits
//! no action work; priority is overflow → input → presentation → wall →
//! clock regression → admit.
//!
//! This is a pure reducer: it takes typed inputs and returns typed outputs.
//! No globals, no locks, no side effects.
//!
//! @plan PLAN-20260723-RUNTIME-AUTOMATION.P02
//! @requirement REQ-WATCH-001..003

use std::time::{Duration, Instant};

// ===========================================================================
//  Types
// ===========================================================================

/// The kind of active callback entering the watchdog.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P02
/// @requirement REQ-WATCH-001
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CallbackKind {
    Input,
    Present,
}

/// Inclusive budget limits. Zero maxima are rejected by script validation;
/// they must be positive.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P02
/// @requirement REQ-WATCH-001
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WatchdogLimits {
    pub max_input_ticks: u64,
    pub max_presentations: u64,
    pub max_wallclock: Duration,
}

/// Monotonic clock sample for regression detection.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P02
/// @requirement REQ-WATCH-003
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockSample {
    /// When the run started.
    pub started_at: Instant,
    /// The previous observation, for regression detection.
    pub last_observed: Instant,
    /// The current observation.
    pub now: Instant,
}

/// The watchdog entry — everything the pure reducer needs to make its
/// decision.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P02
/// @requirement REQ-WATCH-001, REQ-WATCH-002
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WatchdogEntry {
    pub kind: CallbackKind,
    /// Pre-increment input count.
    pub input_seen: u64,
    /// Pre-increment presentation count.
    pub present_seen: u64,
    /// Elapsed time since run start.
    pub elapsed: Duration,
    /// Clock samples for regression detection.
    pub clock: ClockSample,
}

/// The outcome of one watchdog reducer transition.
///
/// Variants are ordered by priority (execution-contract §2.2 table).
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P02
/// @requirement REQ-WATCH-001, REQ-WATCH-002
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WatchdogOutcome {
    /// `input_seen.checked_add(1)` overflowed (order 1 for input callbacks).
    InputCounterOverflow,
    /// `present_seen.checked_add(1)` overflowed (order 1 for present callbacks).
    PresentationCounterOverflow,
    /// Post-increment `input_seen >= max_input_ticks` (order 2).
    InputTimeout,
    /// Post-increment `present_seen >= max_presentations` (order 3).
    PresentationTimeout,
    /// `elapsed >= timeout` (order 4).
    WallTimeout,
    /// Clock moved backwards (order 5).
    ClockRegression,
    /// All limits satisfied — action work is allowed (order 6).
    Admit,
}

impl WatchdogOutcome {
    /// `true` if this outcome permits scheduler action work.
    #[must_use]
    pub const fn admits_work(self) -> bool {
        matches!(self, Self::Admit)
    }

    /// `true` if this outcome is terminal (non-admit).
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        !self.admits_work()
    }
}

/// The result of a watchdog reducer transition.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P02
/// @requirement REQ-WATCH-001
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WatchdogTransition {
    /// Post-increment input count (stored before comparison).
    pub candidate_input_seen: u64,
    /// Post-increment presentation count (stored before comparison).
    pub candidate_present_seen: u64,
    /// The outcome after comparison.
    pub outcome: WatchdogOutcome,
}

// ===========================================================================
//  Pure reducer
// ===========================================================================

/// Apply the watchdog reducer (execution-contract §2.2 table).
///
/// Steps:
/// 1. Checked-add the applicable counter; overflow is the first-priority
///    typed failure.
/// 2. Compare post-increment counters and elapsed time in priority order:
///    input ≥ max → presentation ≥ max → wall ≥ timeout → clock regression.
/// 3. If no limit is exceeded, admit.
///
/// The applicable counter is incremented and stored before any comparison.
/// Equality (`candidate == max`) is terminal and admits no action work.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P02
/// @requirement REQ-WATCH-001, REQ-WATCH-002
#[must_use]
pub fn watchdog_reduce(entry: &WatchdogEntry, limits: &WatchdogLimits) -> WatchdogTransition {
    // Order 1: checked-add the applicable counter.
    let (candidate_input_seen, candidate_present_seen) = match entry.kind {
        CallbackKind::Input => {
            let candidate = match entry.input_seen.checked_add(1) {
                Some(v) => v,
                None => {
                    return WatchdogTransition {
                        candidate_input_seen: entry.input_seen,
                        candidate_present_seen: entry.present_seen,
                        outcome: WatchdogOutcome::InputCounterOverflow,
                    };
                }
            };
            (candidate, entry.present_seen)
        }
        CallbackKind::Present => {
            let candidate = match entry.present_seen.checked_add(1) {
                Some(v) => v,
                None => {
                    return WatchdogTransition {
                        candidate_input_seen: entry.input_seen,
                        candidate_present_seen: entry.present_seen,
                        outcome: WatchdogOutcome::PresentationCounterOverflow,
                    };
                }
            };
            (entry.input_seen, candidate)
        }
    };

    // Order 2: input_seen >= max_input_ticks
    if candidate_input_seen >= limits.max_input_ticks {
        return WatchdogTransition {
            candidate_input_seen,
            candidate_present_seen,
            outcome: WatchdogOutcome::InputTimeout,
        };
    }

    // Order 3: present_seen >= max_presentations
    if candidate_present_seen >= limits.max_presentations {
        return WatchdogTransition {
            candidate_input_seen,
            candidate_present_seen,
            outcome: WatchdogOutcome::PresentationTimeout,
        };
    }

    // Order 4: elapsed >= timeout
    if entry.elapsed >= limits.max_wallclock {
        return WatchdogTransition {
            candidate_input_seen,
            candidate_present_seen,
            outcome: WatchdogOutcome::WallTimeout,
        };
    }

    // Order 5: clock regression
    if entry.clock.now < entry.clock.started_at || entry.clock.now < entry.clock.last_observed {
        return WatchdogTransition {
            candidate_input_seen,
            candidate_present_seen,
            outcome: WatchdogOutcome::ClockRegression,
        };
    }

    // Order 6: admit
    WatchdogTransition {
        candidate_input_seen,
        candidate_present_seen,
        outcome: WatchdogOutcome::Admit,
    }
}

// ===========================================================================
//  Unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const MAX_INPUT: u64 = 3;
    const MAX_PRESENT: u64 = 3;
    const TIMEOUT: Duration = Duration::from_secs(60);

    fn limits() -> WatchdogLimits {
        WatchdogLimits {
            max_input_ticks: MAX_INPUT,
            max_presentations: MAX_PRESENT,
            max_wallclock: TIMEOUT,
        }
    }

    fn base() -> Instant {
        // Use a fixed base to avoid time-inversion across helper calls.
        Instant::now()
    }

    fn clock_at(start: Instant, now: Instant) -> ClockSample {
        ClockSample {
            started_at: start,
            last_observed: now,
            now,
        }
    }

    fn input_entry(input_seen: u64, elapsed: Duration, clock: ClockSample) -> WatchdogEntry {
        WatchdogEntry {
            kind: CallbackKind::Input,
            input_seen,
            present_seen: 0,
            elapsed,
            clock,
        }
    }

    fn present_entry(present_seen: u64, elapsed: Duration, clock: ClockSample) -> WatchdogEntry {
        WatchdogEntry {
            kind: CallbackKind::Present,
            input_seen: 0,
            present_seen,
            elapsed,
            clock,
        }
    }

    fn ok_clock() -> ClockSample {
        let s = base();
        clock_at(s, s + Duration::from_secs(1))
    }

    // --- Exact timeline: max_input_ticks=3 (execution-contract §2.1) ---

    #[test]
    fn max3_timeline_callback1_admit() {
        let entry = input_entry(0, Duration::ZERO, ok_clock());
        let t = watchdog_reduce(&entry, &limits());
        assert_eq!(t.candidate_input_seen, 1);
        assert_eq!(t.outcome, WatchdogOutcome::Admit);
    }

    #[test]
    fn max3_timeline_callback2_admit() {
        let entry = input_entry(1, Duration::from_secs(1), ok_clock());
        let t = watchdog_reduce(&entry, &limits());
        assert_eq!(t.candidate_input_seen, 2);
        assert_eq!(t.outcome, WatchdogOutcome::Admit);
    }

    #[test]
    fn max3_timeline_callback3_timeout() {
        let entry = input_entry(2, Duration::from_secs(2), ok_clock());
        let t = watchdog_reduce(&entry, &limits());
        assert_eq!(t.candidate_input_seen, 3);
        assert_eq!(t.outcome, WatchdogOutcome::InputTimeout);
        assert!(t.outcome.is_terminal());
        assert!(!t.outcome.admits_work());
    }

    // --- Same for presentations ---

    #[test]
    fn max3_present_timeline() {
        let lim = limits();
        assert_eq!(
            watchdog_reduce(&present_entry(0, Duration::ZERO, ok_clock()), &lim).outcome,
            WatchdogOutcome::Admit
        );
        assert_eq!(
            watchdog_reduce(&present_entry(1, Duration::ZERO, ok_clock()), &lim).outcome,
            WatchdogOutcome::Admit
        );
        assert_eq!(
            watchdog_reduce(&present_entry(2, Duration::ZERO, ok_clock()), &lim).outcome,
            WatchdogOutcome::PresentationTimeout
        );
    }

    // --- Priority: input before presentation ---

    #[test]
    fn input_priority_over_presentation() {
        // Both at max simultaneously for an input callback: input wins.
        let entry = WatchdogEntry {
            kind: CallbackKind::Input,
            input_seen: MAX_INPUT - 1,
            present_seen: MAX_PRESENT - 1,
            elapsed: Duration::ZERO,
            clock: ok_clock(),
        };
        let t = watchdog_reduce(&entry, &limits());
        assert_eq!(t.outcome, WatchdogOutcome::InputTimeout);
    }

    // --- Priority: presentation before wall ---

    #[test]
    fn presentation_priority_over_wall() {
        // present at max AND wall at max for a present callback: presentation wins.
        let entry = WatchdogEntry {
            kind: CallbackKind::Present,
            input_seen: 0,
            present_seen: MAX_PRESENT - 1,
            elapsed: TIMEOUT,
            clock: ok_clock(),
        };
        let t = watchdog_reduce(&entry, &limits());
        assert_eq!(t.outcome, WatchdogOutcome::PresentationTimeout);
    }

    // --- Priority: wall before clock ---

    #[test]
    fn wall_priority_over_clock() {
        let s = base();
        let entry = WatchdogEntry {
            kind: CallbackKind::Input,
            input_seen: 0,
            present_seen: 0,
            elapsed: TIMEOUT,
            clock: ClockSample {
                started_at: s,
                last_observed: s + Duration::from_secs(10),
                now: s, // regressed
            },
        };
        let t = watchdog_reduce(&entry, &limits());
        assert_eq!(t.outcome, WatchdogOutcome::WallTimeout);
    }

    // --- Clock regression (REQ-WATCH-003) ---

    #[test]
    fn clock_regression_before_started_at() {
        let s = base();
        let entry = input_entry(
            0,
            Duration::ZERO,
            ClockSample {
                started_at: s + Duration::from_secs(10),
                last_observed: s,
                now: s, // now < started_at
            },
        );
        let t = watchdog_reduce(&entry, &limits());
        assert_eq!(t.outcome, WatchdogOutcome::ClockRegression);
    }

    #[test]
    fn clock_regression_before_last_observed() {
        let s = base();
        let entry = input_entry(
            0,
            Duration::ZERO,
            ClockSample {
                started_at: s,
                last_observed: s + Duration::from_secs(5),
                now: s, // now < last_observed
            },
        );
        let t = watchdog_reduce(&entry, &limits());
        assert_eq!(t.outcome, WatchdogOutcome::ClockRegression);
    }

    #[test]
    fn input_counter_overflow() {
        let entry = input_entry(u64::MAX, Duration::ZERO, ok_clock());
        let t = watchdog_reduce(&entry, &limits());
        assert_eq!(t.outcome, WatchdogOutcome::InputCounterOverflow);
        // Candidate is unchanged (overflow didn't happen)
        assert_eq!(t.candidate_input_seen, u64::MAX);
    }

    #[test]
    fn present_counter_overflow() {
        let entry = present_entry(u64::MAX, Duration::ZERO, ok_clock());
        let t = watchdog_reduce(&entry, &limits());
        assert_eq!(t.outcome, WatchdogOutcome::PresentationCounterOverflow);
    }

    // --- One-below boundary: still admitted ---

    #[test]
    fn one_below_max_admits() {
        // max=3, so input_seen=1 -> candidate=2 -> 2 < 3 -> admit
        let entry = input_entry(1, Duration::ZERO, ok_clock());
        let t = watchdog_reduce(&entry, &limits());
        assert_eq!(t.outcome, WatchdogOutcome::Admit);
    }

    // --- Terminal callback does not increment other counter ---

    #[test]
    fn input_terminal_leaves_present_unchanged() {
        let entry = input_entry(MAX_INPUT - 1, Duration::ZERO, ok_clock());
        let t = watchdog_reduce(&entry, &limits());
        assert_eq!(t.candidate_present_seen, 0);
    }

    #[test]
    fn present_does_not_increment_input() {
        let entry = present_entry(0, Duration::ZERO, ok_clock());
        let t = watchdog_reduce(&entry, &limits());
        assert_eq!(t.candidate_input_seen, 0);
        assert_eq!(t.candidate_present_seen, 1);
    }
}
