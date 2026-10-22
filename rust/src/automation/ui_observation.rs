//! Read-only semantic observations for automation-visible UI states.

use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

static GAME_OPTIONS_ACTIVE: AtomicBool = AtomicBool::new(false);
static COMMUNICATION_RESPONSE_COUNT: AtomicUsize = AtomicUsize::new(0);
static COMMUNICATION_RESPONSE_GENERATION: AtomicU64 = AtomicU64::new(0);
static COMMUNICATION_RESPONSES_READY: AtomicBool = AtomicBool::new(false);
static COMMUNICATION_ACTIVE: AtomicBool = AtomicBool::new(false);
static COMMUNICATION_REPLAY_ACTIVE: AtomicBool = AtomicBool::new(false);
static COMMUNICATION_REPLAY_GENERATION: AtomicU64 = AtomicU64::new(0);
static STARBASE_MENU_ACTIVE: AtomicBool = AtomicBool::new(false);
static PLANET_MENU_PHASE: AtomicUsize = AtomicUsize::new(0);
static PLANET_MENU_GENERATION: AtomicU64 = AtomicU64::new(0);
static SHIP_PICKER_ACTIVE: AtomicBool = AtomicBool::new(false);
static SHIP_PICKER_GENERATION: AtomicU64 = AtomicU64::new(0);

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

/// Record whether the Starbase menu, or the Outfit screen it opens, is the
/// active input loop.
pub fn observe_starbase_menu(active: bool) {
    STARBASE_MENU_ACTIVE.store(active, Ordering::Release);
}

/// Whether the Starbase menu or the Outfit screen is the active input loop.
#[must_use]
pub fn starbase_menu_active() -> bool {
    STARBASE_MENU_ACTIVE.load(Ordering::Acquire)
}

/// Record entry to or exit from the production Game Options input loop.
#[no_mangle]
pub extern "C" fn rust_automation_observe_game_options(active: i32) {
    GAME_OPTIONS_ACTIVE.store(active != 0, Ordering::Release);
}

static ENCOUNTER_MENU_ACTIVE: AtomicBool = AtomicBool::new(false);

pub fn observe_encounter_menu(active: bool) {
    ENCOUNTER_MENU_ACTIVE.store(active, Ordering::Release);
}

pub fn encounter_menu_active() -> bool {
    ENCOUNTER_MENU_ACTIVE.load(Ordering::Acquire)
}

static SHIP_PICKER_CURSOR: std::sync::Mutex<crate::pickship::Cursor> =
    std::sync::Mutex::new(crate::pickship::Cursor::START);

pub fn observe_ship_picker_cursor(cursor: crate::pickship::Cursor) {
    *SHIP_PICKER_CURSOR.lock().expect("picker cursor lock") = cursor;
}

pub fn ship_picker_cursor() -> crate::pickship::Cursor {
    *SHIP_PICKER_CURSOR.lock().expect("picker cursor lock")
}

/// Record entry to or exit from the production battle ship picker.
///
/// Each entry starts a new generation so a pick consumed for one battle cannot
/// satisfy the picker of the next.
pub fn observe_ship_picker(active: bool) {
    let previous = SHIP_PICKER_ACTIVE.swap(active, Ordering::AcqRel);
    if active && !previous {
        SHIP_PICKER_GENERATION.fetch_add(1, Ordering::AcqRel);
    }
}

/// Return the current ship-picker generation and whether the picker is active.
#[must_use]
pub fn ship_picker_observation() -> (u64, bool) {
    (
        SHIP_PICKER_GENERATION.load(Ordering::Acquire),
        SHIP_PICKER_ACTIVE.load(Ordering::Acquire),
    )
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

/// The production communication state a response wait inspects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConversationSnapshot {
    pub active: bool,
    pub completions: u64,
    pub response_generation: u64,
    pub response_count: usize,
}

impl ConversationSnapshot {
    /// Whether the conversation that began after the last consumed completion
    /// has registered a response list, newer than any a selection consumed,
    /// holding at least `minimum` choices.
    ///
    /// The NPC callback registers choices one at a time after the hail is
    /// dispatched, so the count is only meaningful once it reaches the
    /// minimum. A finished conversation clears its list, but the completion
    /// and generation checks keep a stale list from ever satisfying the wait.
    #[must_use]
    pub fn has_fresh_responses(
        &self,
        minimum: usize,
        consumed_completions: u64,
        consumed_generation: u64,
    ) -> bool {
        self.active
            && self.completions == consumed_completions
            && self.response_generation > consumed_generation
            && self.response_count >= minimum
    }
}

/// Read the communication loop's current lifecycle and response list.
#[must_use]
pub fn conversation_snapshot() -> ConversationSnapshot {
    let (active, completions) = communication_lifecycle();
    let (response_generation, response_count, _) = communication_responses();
    ConversationSnapshot {
        active,
        completions,
        response_generation,
        response_count,
    }
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
    use std::sync::Mutex;

    /// The observations are process-wide; tests that drive them take turns.
    static OBSERVATIONS: Mutex<()> = Mutex::new(());

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
    fn ship_picker_observation_starts_a_generation_per_entry() {
        let _turn = OBSERVATIONS.lock().unwrap_or_else(|e| e.into_inner());
        observe_ship_picker(false);
        let (before, active) = ship_picker_observation();
        assert!(!active);
        observe_ship_picker(true);
        assert_eq!(ship_picker_observation(), (before + 1, true));
        observe_ship_picker(true);
        assert_eq!(ship_picker_observation(), (before + 1, true));
        observe_ship_picker(false);
        assert_eq!(ship_picker_observation(), (before + 1, false));
        observe_ship_picker(true);
        assert_eq!(ship_picker_observation(), (before + 2, true));
        observe_ship_picker(false);
    }

    #[test]
    fn starbase_menu_observation_tracks_entry_and_exit() {
        observe_starbase_menu(false);
        assert!(!starbase_menu_active());
        observe_starbase_menu(true);
        assert!(starbase_menu_active());
        observe_starbase_menu(false);
        assert!(!starbase_menu_active());
    }

    #[test]
    fn response_observation_requires_the_requested_minimum() {
        let _turn = OBSERVATIONS.lock().unwrap_or_else(|e| e.into_inner());
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
        let _turn = OBSERVATIONS.lock().unwrap_or_else(|e| e.into_inner());
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

    fn snapshot(
        active: bool,
        completions: u64,
        generation: u64,
        count: usize,
    ) -> ConversationSnapshot {
        ConversationSnapshot {
            active,
            completions,
            response_generation: generation,
            response_count: count,
        }
    }

    #[test]
    fn fresh_responses_need_the_current_conversation_and_the_minimum() {
        // Two conversations finished and were consumed; two lists were selected.
        let (done, consumed) = (2, 2);
        assert!(!snapshot(true, done, 2, 0).has_fresh_responses(2, done, consumed));
        assert!(!snapshot(true, done, 3, 1).has_fresh_responses(2, done, consumed));
        assert!(snapshot(true, done, 3, 2).has_fresh_responses(2, done, consumed));
        assert!(snapshot(true, done, 3, 3).has_fresh_responses(2, done, consumed));
        // A list a selection already consumed is stale, whatever its size.
        assert!(!snapshot(true, done, 2, 3).has_fresh_responses(2, done, consumed));
        // A conversation whose completion nobody consumed is another conversation.
        assert!(!snapshot(true, done + 1, 3, 3).has_fresh_responses(2, done, consumed));
        // No live communication loop, no menu.
        assert!(!snapshot(false, done, 3, 3).has_fresh_responses(2, done, consumed));
    }

    #[test]
    fn responses_registered_after_the_hail_satisfy_the_wait_only_when_they_arrive() {
        let _turn = OBSERVATIONS.lock().unwrap_or_else(|e| e.into_inner());
        // The previous conversation ended with its list selected and cleared.
        observe_communication_responses(0);
        set_communication_responses_ready(false);
        let (_, completions) = communication_lifecycle();
        let (selected_generation, _, _) = communication_responses();

        // The new hail is dispatched and speech plays: the loop is live but
        // the NPC callback has not registered any choice yet.
        begin_communication();
        let reached =
            || conversation_snapshot().has_fresh_responses(2, completions, selected_generation);
        assert!(verify_communication_responses(2).is_err());
        assert!(!reached());

        // The callback registers its choices one at a time.
        observe_communication_responses(1);
        assert!(!reached());
        observe_communication_responses(2);
        assert!(reached());
        assert_eq!(verify_communication_responses(2), Ok(2));

        // Once the conversation closes nothing from it can satisfy a wait.
        observe_communication_responses(0);
        complete_communication();
        assert!(!reached());
    }
}
