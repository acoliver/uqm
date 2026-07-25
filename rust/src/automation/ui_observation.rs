//! Read-only semantic observations for automation-visible UI states.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

static GAME_OPTIONS_ACTIVE: AtomicBool = AtomicBool::new(false);
static COMMUNICATION_RESPONSE_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Record entry to or exit from the production Game Options input loop.
#[no_mangle]
pub extern "C" fn rust_automation_observe_game_options(active: i32) {
    GAME_OPTIONS_ACTIVE.store(active != 0, Ordering::Release);
}

/// Record the number of response choices in the current communication frame.
pub fn observe_communication_responses(count: usize) {
    COMMUNICATION_RESPONSE_COUNT.store(count, Ordering::Release);
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
}
