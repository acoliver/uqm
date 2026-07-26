//! Read-only semantic observations for automation-visible UI states.

use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

static GAME_OPTIONS_ACTIVE: AtomicBool = AtomicBool::new(false);
static COMMUNICATION_RESPONSE_COUNT: AtomicUsize = AtomicUsize::new(0);
static COMMUNICATION_ACTIVE: AtomicBool = AtomicBool::new(false);
static COMMUNICATION_COMPLETIONS: AtomicU64 = AtomicU64::new(0);

/// Record entry to or exit from the production Game Options input loop.
#[no_mangle]
pub extern "C" fn rust_automation_observe_game_options(active: i32) {
    GAME_OPTIONS_ACTIVE.store(active != 0, Ordering::Release);
}

/// Record the number of response choices in the current communication frame.
pub fn observe_communication_responses(count: usize) {
    COMMUNICATION_RESPONSE_COUNT.store(count, Ordering::Release);
}

/// Record entry into a production communication input loop.
pub fn begin_communication() {
    COMMUNICATION_RESPONSE_COUNT.store(0, Ordering::Release);
    COMMUNICATION_ACTIVE.store(true, Ordering::Release);
}

/// Record normal return from a production communication input loop.
pub fn complete_communication() {
    COMMUNICATION_ACTIVE.store(false, Ordering::Release);
    COMMUNICATION_COMPLETIONS.fetch_add(1, Ordering::AcqRel);
}

/// Return whether communication is active and how many loops completed.
#[must_use]
pub fn communication_lifecycle() -> (bool, u64) {
    (
        COMMUNICATION_ACTIVE.load(Ordering::Acquire),
        COMMUNICATION_COMPLETIONS.load(Ordering::Acquire),
    )
}

/// C ABI observer used by the production response-list renderer.
#[no_mangle]
pub extern "C" fn rust_automation_observe_communication_responses(count: u32) {
    observe_communication_responses(count as usize);
}

/// Verify that the nested production Game Options loop is currently active.
pub fn verify_game_options_active() -> Result<(), &'static str> {
    if GAME_OPTIONS_ACTIVE.load(Ordering::Acquire) {
        Ok(())
    } else {
        Err("Game Options input loop is not active")
    }
}

/// Verify that a communication response list containing at least `minimum` entries is active.
pub fn verify_communication_responses(minimum: usize) -> Result<usize, String> {
    let actual = COMMUNICATION_RESPONSE_COUNT.load(Ordering::Acquire);
    if actual >= minimum {
        Ok(actual)
    } else {
        Err(format!(
            "expected at least {minimum} communication responses, observed {actual}"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_options_observation_tracks_entry_and_exit() {
        rust_automation_observe_game_options(0);
        assert!(verify_game_options_active().is_err());
        rust_automation_observe_game_options(1);
        assert!(verify_game_options_active().is_ok());
        rust_automation_observe_game_options(0);
        assert!(verify_game_options_active().is_err());
    }

    #[test]
    fn response_observation_requires_the_requested_minimum() {
        observe_communication_responses(3);
        assert_eq!(verify_communication_responses(2), Ok(3));
        assert!(verify_communication_responses(4).is_err());
        observe_communication_responses(0);
    }

    #[test]
    fn communication_lifecycle_counts_completed_loops() {
        let (_, before) = communication_lifecycle();
        begin_communication();
        assert_eq!(communication_lifecycle(), (true, before));
        complete_communication();
        assert_eq!(communication_lifecycle(), (false, before + 1));
    }
}
