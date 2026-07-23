//! Terminal outcome classification and first-wins terminal state model.
//!
//! Implements REQ-STATE-001 (first-wins terminal), REQ-STATE-002 (absorbing
//! transition), and the pure classification model for REQ-WATCH-004
//! (cooperative timeout vs parent hard hang).
//!
//! @plan PLAN-20260723-RUNTIME-AUTOMATION.P04
//! @requirement REQ-STATE-001, REQ-STATE-002, REQ-WATCH-004

use std::sync::atomic::{AtomicU8, Ordering};

// ===========================================================================
//  Terminal outcome classification (REQ-STATE-001)
// ===========================================================================

/// The class of a terminal outcome. First-wins: once a terminal class is
/// stored, later errors are secondary and never replace the first outcome.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P04
/// @requirement REQ-STATE-001
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TerminalClass {
    /// Scheduler completed all steps successfully.
    Success = 0,
    /// Watchdog input timeout.
    InputTimeout = 1,
    /// Watchdog presentation timeout.
    PresentationTimeout = 2,
    /// Watchdog wall-clock timeout.
    WallTimeout = 3,
    /// Clock regression detected.
    ClockRegression = 4,
    /// Counter overflow.
    CounterOverflow = 5,
    /// Capture generation mismatch (stale/duplicate/zero/future).
    CaptureMismatch = 6,
    /// Semantic assertion mismatch.
    SemanticMismatch = 7,
    /// Trace/file sink failure.
    TraceFailure = 8,
    /// State version overflow.
    StateVersionOverflow = 9,
    /// Capture generation overflow.
    CaptureGenerationOverflow = 10,
    /// Panic in an ABI shell (converted to terminal fallback).
    PanicFallback = 11,
    /// Poisoned runtime mutex.
    PoisonedMutex = 12,
    /// Cooperative stop requested (clean shutdown, not a hard hang).
    CooperativeStop = 13,
}

impl TerminalClass {
    /// Convert from the stored u8.
    fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Success),
            1 => Some(Self::InputTimeout),
            2 => Some(Self::PresentationTimeout),
            3 => Some(Self::WallTimeout),
            4 => Some(Self::ClockRegression),
            5 => Some(Self::CounterOverflow),
            6 => Some(Self::CaptureMismatch),
            7 => Some(Self::SemanticMismatch),
            8 => Some(Self::TraceFailure),
            9 => Some(Self::StateVersionOverflow),
            10 => Some(Self::CaptureGenerationOverflow),
            11 => Some(Self::PanicFallback),
            12 => Some(Self::PoisonedMutex),
            13 => Some(Self::CooperativeStop),
            _ => None,
        }
    }

    /// Whether this terminal class represents a successful outcome.
    #[must_use]
    pub fn is_success(self) -> bool {
        matches!(self, Self::Success | Self::CooperativeStop)
    }
}

/// Terminal command output — always includes release-all, OR-abort intent,
/// and stop=true (REQ-STATE-002).
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P04
/// @requirement REQ-STATE-002
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalCommand {
    pub release_all: bool,
    pub or_abort: bool,
    pub stop: bool,
}

impl TerminalCommand {
    /// The canonical terminal command: release all keys, OR-abort intent,
    /// stop=true.
    #[must_use]
    pub fn terminal() -> Self {
        Self {
            release_all: true,
            or_abort: true,
            stop: true,
        }
    }
}

// ===========================================================================
//  Lock-free terminal mirror (REQ-STATE-001)
// ===========================================================================

/// Lock-free terminal status mirror. Uses `AtomicU8` to store the terminal
/// class. First-wins: a CAS from "no terminal" (255 = none) to a terminal
/// class succeeds only once; later attempts are secondary.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P04
/// @requirement REQ-STATE-001
pub struct TerminalMirror {
    class: AtomicU8,
}

/// Sentinel value for "no terminal outcome yet".
const NO_TERMINAL: u8 = 255;

impl TerminalMirror {
    /// Create a new terminal mirror with no terminal outcome.
    #[must_use]
    pub fn new() -> Self {
        Self {
            class: AtomicU8::new(NO_TERMINAL),
        }
    }

    /// Attempt to set the terminal class. First-wins: returns `true` if this
    /// call set the class, `false` if a terminal was already set (secondary).
    ///
    /// @plan PLAN-20260723-RUNTIME-AUTOMATION.P04
    /// @requirement REQ-STATE-001
    pub fn try_set(&self, class: TerminalClass) -> bool {
        self.class
            .compare_exchange(
                NO_TERMINAL,
                class as u8,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
    }

    /// Load the current terminal class, or `None` if no terminal has been set.
    #[must_use]
    pub fn load(&self) -> Option<TerminalClass> {
        let v = self.class.load(Ordering::Acquire);
        if v == NO_TERMINAL {
            None
        } else {
            TerminalClass::from_u8(v)
        }
    }

    /// Whether a terminal outcome has been set.
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        self.class.load(Ordering::Acquire) != NO_TERMINAL
    }
}

impl Default for TerminalMirror {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
//  Cooperative timeout vs parent hard hang (REQ-WATCH-004)
// ===========================================================================

/// Classification of a timeout or hang condition.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P04
/// @requirement REQ-WATCH-004
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HangClassification {
    /// Cooperative timeout: the child/process responded to a stop request
    /// within the deadline. This is a clean shutdown.
    CooperativeTimeout,
    /// Parent hard hang: the child/process did not respond to a stop request
    /// and the parent had to force termination.
    ParentHardHang,
}

// ===========================================================================
//  Unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- First-wins terminal (REQ-STATE-001) ---

    #[test]
    fn first_wins_sets_once() {
        let mirror = TerminalMirror::new();
        assert!(!mirror.is_terminal());
        assert!(mirror.try_set(TerminalClass::InputTimeout));
        assert!(mirror.is_terminal());
        assert!(!mirror.try_set(TerminalClass::Success));
        assert_eq!(mirror.load(), Some(TerminalClass::InputTimeout));
    }

    #[test]
    fn success_is_first_wins_too() {
        let mirror = TerminalMirror::new();
        assert!(mirror.try_set(TerminalClass::Success));
        assert!(!mirror.try_set(TerminalClass::InputTimeout));
        assert_eq!(mirror.load(), Some(TerminalClass::Success));
    }

    #[test]
    fn later_errors_are_secondary() {
        let mirror = TerminalMirror::new();
        mirror.try_set(TerminalClass::CaptureMismatch);
        // Multiple later attempts all fail.
        assert!(!mirror.try_set(TerminalClass::TraceFailure));
        assert!(!mirror.try_set(TerminalClass::PanicFallback));
        assert!(!mirror.try_set(TerminalClass::Success));
        assert_eq!(mirror.load(), Some(TerminalClass::CaptureMismatch));
    }

    // --- Absorbing transition (REQ-STATE-002) ---

    #[test]
    fn terminal_command_always_release_all_or_abort_stop() {
        let cmd = TerminalCommand::terminal();
        assert!(cmd.release_all);
        assert!(cmd.or_abort);
        assert!(cmd.stop);
    }

    // --- is_success ---

    #[test]
    fn success_and_cooperative_stop_are_success() {
        assert!(TerminalClass::Success.is_success());
        assert!(TerminalClass::CooperativeStop.is_success());
        assert!(!TerminalClass::InputTimeout.is_success());
        assert!(!TerminalClass::PanicFallback.is_success());
    }

    // --- Hang classification (REQ-WATCH-004) ---

    #[test]
    fn cooperative_timeout_distinct_from_hard_hang() {
        assert_ne!(
            HangClassification::CooperativeTimeout,
            HangClassification::ParentHardHang
        );
    }

    // --- Lock-free property (verified by P00 probes at runtime) ---

    #[test]
    fn terminal_mirror_uses_atomic() {
        let mirror = TerminalMirror::new();
        mirror.try_set(TerminalClass::Success);
        assert_eq!(mirror.load(), Some(TerminalClass::Success));
        // First-wins: second attempt fails.
        assert!(!mirror.try_set(TerminalClass::InputTimeout));
    }
}
