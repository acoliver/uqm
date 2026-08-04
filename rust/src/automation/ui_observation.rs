//! Read-only semantic observations for automation-visible UI states.

use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

static GAME_OPTIONS_ACTIVE: AtomicBool = AtomicBool::new(false);
static COMMUNICATION_RESPONSE_COUNT: AtomicUsize = AtomicUsize::new(0);
static COMMUNICATION_RESPONSE_GENERATION: AtomicU64 = AtomicU64::new(0);
static COMMUNICATION_RESPONSES_READY: AtomicBool = AtomicBool::new(false);
static COMMUNICATION_ACTIVE: AtomicBool = AtomicBool::new(false);
static COMMUNICATION_REPLAY_ACTIVE: AtomicBool = AtomicBool::new(false);
static COMMUNICATION_REPLAY_GENERATION: AtomicU64 = AtomicU64::new(0);
static PLANET_MENU_PHASE: AtomicUsize = AtomicUsize::new(0);
static PLANET_MENU_GENERATION: AtomicU64 = AtomicU64::new(0);

/// Production planet-menu phases exposed to semantic automation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum PlanetMenuPhase {
    Inactive = 0,
    Orbit = 1,
    AutoScan = 2,
    Dispatch = 3,
    LandingSite = 4,
}

/// Record the current production planet-menu phase.
#[no_mangle]
pub extern "C" fn rust_automation_observe_planet_menu(phase: usize) {
    let previous = PLANET_MENU_PHASE.swap(phase, Ordering::AcqRel);
    if phase != PlanetMenuPhase::Inactive as usize && previous != phase {
        PLANET_MENU_GENERATION.fetch_add(1, Ordering::AcqRel);
    }
}

/// Return the current production planet-menu generation and phase.
#[must_use]
pub fn planet_menu_observation() -> (u64, PlanetMenuPhase) {
    (
        PLANET_MENU_GENERATION.load(Ordering::Acquire),
        planet_menu_phase(),
    )
}

/// Return the current production planet-menu phase.
#[must_use]
pub fn planet_menu_phase() -> PlanetMenuPhase {
    match PLANET_MENU_PHASE.load(Ordering::Acquire) {
        1 => PlanetMenuPhase::Orbit,
        2 => PlanetMenuPhase::AutoScan,
        3 => PlanetMenuPhase::Dispatch,
        4 => PlanetMenuPhase::LandingSite,
        _ => PlanetMenuPhase::Inactive,
    }
}

static COMMUNICATION_COMPLETIONS: AtomicU64 = AtomicU64::new(0);

/// Record entry to or exit from the production Game Options input loop.
#[no_mangle]
pub extern "C" fn rust_automation_observe_game_options(active: i32) {
    GAME_OPTIONS_ACTIVE.store(active != 0, Ordering::Release);
}

/// Record the number of response choices in the current communication frame.
pub fn observe_communication_responses(count: usize) {
    let previous = COMMUNICATION_RESPONSE_COUNT.swap(count, Ordering::AcqRel);
    if count > 0 && previous == 0 {
        COMMUNICATION_RESPONSE_GENERATION.fetch_add(1, Ordering::AcqRel);
    }
}

/// Current response-list generation and number of choices.
#[must_use]
pub fn communication_responses() -> (u64, usize, bool) {
    (
        COMMUNICATION_RESPONSE_GENERATION.load(Ordering::Acquire),
        COMMUNICATION_RESPONSE_COUNT.load(Ordering::Acquire),
        COMMUNICATION_RESPONSES_READY.load(Ordering::Acquire),
    )
}

/// Record whether the production response phase can accept Select input.
pub fn set_communication_responses_ready(ready: bool) {
    COMMUNICATION_RESPONSES_READY.store(ready, Ordering::Release);
}

/// Record entry into a production communication input loop.
pub fn begin_communication() {
    COMMUNICATION_RESPONSE_COUNT.store(0, Ordering::Release);
    COMMUNICATION_RESPONSES_READY.store(false, Ordering::Release);
    COMMUNICATION_ACTIVE.store(true, Ordering::Release);
}

/// Record normal return from a production communication input loop.
pub fn complete_communication() {
    COMMUNICATION_RESPONSES_READY.store(false, Ordering::Release);
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

/// Record entry to or exit from replaying the most recent alien phrase.
pub fn observe_communication_replay(active: bool) {
    let previous = COMMUNICATION_REPLAY_ACTIVE.swap(active, Ordering::AcqRel);
    if active && !previous {
        COMMUNICATION_REPLAY_GENERATION.fetch_add(1, Ordering::AcqRel);
    }
}

/// Return the replay generation and whether phrase playback remains active.
#[must_use]
pub fn communication_replay_observation() -> (u64, bool) {
    (
        COMMUNICATION_REPLAY_GENERATION.load(Ordering::Acquire),
        COMMUNICATION_REPLAY_ACTIVE.load(Ordering::Acquire),
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
        observe_communication_responses(0);
        let (before, _, _) = communication_responses();
        observe_communication_responses(3);
        assert_eq!(communication_responses(), (before + 1, 3, false));
        assert_eq!(verify_communication_responses(2), Ok(3));
        assert!(verify_communication_responses(4).is_err());
        observe_communication_responses(2);
        assert_eq!(communication_responses(), (before + 1, 2, false));
        set_communication_responses_ready(true);
        assert_eq!(communication_responses(), (before + 1, 2, true));
        observe_communication_responses(0);
        set_communication_responses_ready(false);
    }

    #[test]
    fn planet_menu_observation_requires_a_new_active_phase() {
        rust_automation_observe_planet_menu(PlanetMenuPhase::Inactive as usize);
        let (before, _) = planet_menu_observation();
        rust_automation_observe_planet_menu(PlanetMenuPhase::Orbit as usize);
        assert_eq!(
            planet_menu_observation(),
            (before + 1, PlanetMenuPhase::Orbit)
        );
        rust_automation_observe_planet_menu(PlanetMenuPhase::Orbit as usize);
        assert_eq!(
            planet_menu_observation(),
            (before + 1, PlanetMenuPhase::Orbit)
        );
        rust_automation_observe_planet_menu(PlanetMenuPhase::Inactive as usize);
        assert_eq!(
            planet_menu_observation(),
            (before + 1, PlanetMenuPhase::Inactive)
        );
    }

    #[test]
    fn communication_lifecycle_counts_completed_loops() {
        let (_, before) = communication_lifecycle();
        begin_communication();
        assert_eq!(communication_lifecycle(), (true, before));
        complete_communication();
        assert_eq!(communication_lifecycle(), (false, before + 1));
    }

    #[test]
    fn communication_replay_observation_tracks_generations_and_activity() {
        observe_communication_replay(false);
        let (before, active) = communication_replay_observation();
        assert!(!active);

        observe_communication_replay(true);
        assert_eq!(communication_replay_observation(), (before + 1, true));
        observe_communication_replay(true);
        assert_eq!(communication_replay_observation(), (before + 1, true));
        observe_communication_replay(false);
        assert_eq!(communication_replay_observation(), (before + 1, false));
    }
}
