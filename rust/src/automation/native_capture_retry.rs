//! Bounded recovery from a hung `screencapture` inside a held presentation
//! barrier.
//!
//! The game publishes a presentation and then waits for the controller's
//! acknowledgement, so a second capture after a timed-out first capture sees
//! the same presentation. Only the capture timeout is retried, at most once,
//! and only while the existing acknowledgement deadline still has room for one
//! more bounded helper operation.

use super::native_window::NativeWindowObserverError;
use serde::Serialize;
use std::time::Duration;

/// One initial attempt and a single retry.
pub(super) const MAX_CAPTURE_ATTEMPTS: u32 = 2;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum CaptureAttemptOutcome {
    Captured,
    ScreencaptureTimedOut,
    Failed,
}

/// Evidence of one attempt, written when a retry happened.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct CaptureAttemptRecord {
    pub attempt: u32,
    pub outcome: CaptureAttemptOutcome,
    pub detail: Option<String>,
    /// Acknowledgement budget left when the attempt finished.
    pub remaining_budget_ms: u64,
    /// Whether another attempt followed this one.
    pub retried: bool,
}

/// Runs one helper capture and reports the barrier budget that remains.
pub(super) trait CaptureAttemptSource {
    /// Captures once into a fresh output path. An implementation must discard
    /// every file of a failed attempt before returning its error.
    fn capture_attempt(&mut self, window_id: u64) -> Result<Vec<u8>, NativeWindowObserverError>;
    fn remaining_budget(&self) -> Duration;
}

/// Only a capture timeout is retried, once, with a full helper operation left.
pub(super) fn should_retry_capture(
    error: &NativeWindowObserverError,
    attempts_made: u32,
    remaining: Duration,
    retry_reserve: Duration,
) -> bool {
    matches!(error, NativeWindowObserverError::CaptureTimedOut(_))
        && attempts_made < MAX_CAPTURE_ATTEMPTS
        && remaining >= retry_reserve
}

pub(super) struct CaptureReport {
    pub result: Result<Vec<u8>, NativeWindowObserverError>,
    pub attempts: Vec<CaptureAttemptRecord>,
}

pub(super) fn capture_with_bounded_retry(
    source: &mut impl CaptureAttemptSource,
    window_id: u64,
    retry_reserve: Duration,
) -> CaptureReport {
    let mut attempts = Vec::new();
    loop {
        let attempt = u32::try_from(attempts.len() + 1).unwrap_or(u32::MAX);
        let result = source.capture_attempt(window_id);
        let remaining = source.remaining_budget();
        let (outcome, detail) = match &result {
            Ok(_) => (CaptureAttemptOutcome::Captured, None),
            Err(error @ NativeWindowObserverError::CaptureTimedOut(_)) => (
                CaptureAttemptOutcome::ScreencaptureTimedOut,
                Some(error.to_string()),
            ),
            Err(error) => (CaptureAttemptOutcome::Failed, Some(error.to_string())),
        };
        let retried = result
            .as_ref()
            .err()
            .is_some_and(|error| should_retry_capture(error, attempt, remaining, retry_reserve));
        attempts.push(CaptureAttemptRecord {
            attempt,
            outcome,
            detail,
            remaining_budget_ms: u64::try_from(remaining.as_millis()).unwrap_or(u64::MAX),
            retried,
        });
        if !retried {
            return CaptureReport { result, attempts };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    struct ScriptedSource {
        results: VecDeque<Result<Vec<u8>, NativeWindowObserverError>>,
        remaining: Duration,
        calls: u32,
    }

    impl CaptureAttemptSource for ScriptedSource {
        fn capture_attempt(
            &mut self,
            _window_id: u64,
        ) -> Result<Vec<u8>, NativeWindowObserverError> {
            self.calls += 1;
            self.results.pop_front().expect("scripted attempt")
        }

        fn remaining_budget(&self) -> Duration {
            self.remaining
        }
    }

    fn timed_out() -> NativeWindowObserverError {
        NativeWindowObserverError::CaptureTimedOut("child timed out".to_string())
    }

    fn run(
        results: Vec<Result<Vec<u8>, NativeWindowObserverError>>,
        remaining: Duration,
    ) -> (CaptureReport, u32) {
        let mut source = ScriptedSource {
            results: results.into(),
            remaining,
            calls: 0,
        };
        let report = capture_with_bounded_retry(&mut source, 7, Duration::from_secs(45));
        (report, source.calls)
    }

    #[test]
    fn a_timeout_then_success_retries_once_and_records_both_attempts() {
        let (report, calls) = run(
            vec![Err(timed_out()), Ok(vec![1, 2])],
            Duration::from_secs(59),
        );
        assert_eq!(calls, 2);
        assert_eq!(report.result, Ok(vec![1, 2]));
        assert_eq!(report.attempts.len(), 2);
        assert_eq!(
            report.attempts[0].outcome,
            CaptureAttemptOutcome::ScreencaptureTimedOut
        );
        assert!(report.attempts[0].retried);
        assert_eq!(report.attempts[1].outcome, CaptureAttemptOutcome::Captured);
        assert!(!report.attempts[1].retried);
    }

    #[test]
    fn two_timeouts_fail_after_exactly_one_retry() {
        let (report, calls) = run(
            vec![Err(timed_out()), Err(timed_out())],
            Duration::from_secs(59),
        );
        assert_eq!(calls, 2);
        assert!(matches!(
            report.result,
            Err(NativeWindowObserverError::CaptureTimedOut(_))
        ));
        assert_eq!(report.attempts.len(), 2);
        assert!(report.attempts[0].retried);
        assert!(!report.attempts[1].retried);
    }

    #[test]
    fn errors_other_than_the_capture_timeout_are_not_retried() {
        for error in [
            NativeWindowObserverError::Os("screencapture exited with code Some(1)".to_string()),
            NativeWindowObserverError::OutputLimit {
                stream: "stdout".to_string(),
                limit_bytes: 1,
            },
            NativeWindowObserverError::NotFound { pid: 3 },
        ] {
            let (report, calls) = run(vec![Err(error.clone())], Duration::from_secs(59));
            assert_eq!(calls, 1);
            assert_eq!(report.result, Err(error));
            assert_eq!(report.attempts.len(), 1);
            assert_eq!(report.attempts[0].outcome, CaptureAttemptOutcome::Failed);
            assert!(!report.attempts[0].retried);
        }
    }

    #[test]
    fn a_timeout_without_room_for_another_helper_operation_is_not_retried() {
        let (report, calls) = run(vec![Err(timed_out())], Duration::from_millis(44_999));
        assert_eq!(calls, 1);
        assert!(report.result.is_err());
        assert!(!report.attempts[0].retried);
        assert_eq!(report.attempts[0].remaining_budget_ms, 44_999);

        let (report, calls) = run(vec![Err(timed_out()), Ok(vec![9])], Duration::from_secs(45));
        assert_eq!(calls, 2);
        assert_eq!(report.result, Ok(vec![9]));
    }

    #[test]
    fn the_retry_policy_allows_no_third_attempt() {
        let error = timed_out();
        assert!(should_retry_capture(
            &error,
            1,
            Duration::from_secs(60),
            Duration::from_secs(45)
        ));
        assert!(!should_retry_capture(
            &error,
            2,
            Duration::from_secs(60),
            Duration::from_secs(45)
        ));
    }
}
