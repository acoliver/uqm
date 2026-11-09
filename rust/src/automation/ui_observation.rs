//! Read-only semantic observations for automation-visible UI states.

use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

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

/// UI observations retained by one app-owned automation observer.
pub struct UiObservation {
    response_choices: parking_lot::Mutex<crate::engine::observe::ResponseChoices>,
    response_request: parking_lot::Mutex<Option<usize>>,
    game_options_active: AtomicBool,
    communication_response_count: AtomicUsize,
    communication_response_generation: AtomicU64,
    communication_responses_ready: AtomicBool,
    communication_active: AtomicBool,
    communication_replay_active: AtomicBool,
    communication_replay_generation: AtomicU64,
    starbase_menu_active: AtomicBool,
    planet_menu_phase: AtomicUsize,
    planet_menu_generation: AtomicU64,
    ship_picker_active: AtomicBool,
    ship_picker_generation: AtomicU64,
    ship_escape_generation: AtomicU64,
    communication_completions: AtomicU64,
    encounter_menu_active: AtomicBool,
    ship_picker_cursor: std::sync::Mutex<crate::pickship::Cursor>,
    star_map: std::sync::Mutex<Option<crate::engine::observe::StarMapView>>,
    devices: std::sync::Mutex<crate::engine::observe::DeviceObservation>,
    solar: std::sync::Mutex<Option<crate::engine::world::generation::SolarSnapshot>>,
}

impl Default for UiObservation {
    fn default() -> Self {
        Self {
            response_choices: Default::default(),
            response_request: Default::default(),
            game_options_active: AtomicBool::new(false),
            communication_response_count: AtomicUsize::new(0),
            communication_response_generation: AtomicU64::new(0),
            communication_responses_ready: AtomicBool::new(false),
            communication_active: AtomicBool::new(false),
            communication_replay_active: AtomicBool::new(false),
            communication_replay_generation: AtomicU64::new(0),
            starbase_menu_active: AtomicBool::new(false),
            planet_menu_phase: AtomicUsize::new(0),
            planet_menu_generation: AtomicU64::new(0),
            ship_picker_active: AtomicBool::new(false),
            ship_picker_generation: AtomicU64::new(0),
            ship_escape_generation: AtomicU64::new(0),
            communication_completions: AtomicU64::new(0),
            encounter_menu_active: AtomicBool::new(false),
            ship_picker_cursor: std::sync::Mutex::new(crate::pickship::Cursor::START),
            star_map: std::sync::Mutex::new(None),
            devices: std::sync::Mutex::new(Default::default()),
            solar: std::sync::Mutex::new(None),
        }
    }
}

impl UiObservation {
    pub fn observe_solar(
        &self,
        snapshot: Option<&crate::engine::world::generation::SolarSnapshot>,
    ) {
        *self
            .solar
            .lock()
            .unwrap_or_else(|error| panic!("solar observation lock poisoned: {error}")) =
            snapshot.copied();
    }

    pub fn solar_snapshot(&self) -> Option<crate::engine::world::generation::SolarSnapshot> {
        *self
            .solar
            .lock()
            .unwrap_or_else(|error| panic!("solar observation lock poisoned: {error}"))
    }

    pub fn observe_devices(&self, state: &crate::engine::observe::DeviceObservation) {
        self.devices
            .lock()
            .unwrap_or_else(|error| panic!("device observation lock poisoned: {error}"))
            .clone_from(state);
    }

    pub fn devices(&self) -> crate::engine::observe::DeviceObservation {
        self.devices
            .lock()
            .unwrap_or_else(|error| panic!("device observation lock poisoned: {error}"))
            .clone()
    }

    pub(crate) fn observe_choices(&self, choices: &crate::engine::observe::ResponseChoices) {
        *self.response_choices.lock() = choices.clone();
    }
    pub(crate) fn offered_response_index(&self, phrase: &str) -> Result<usize, String> {
        self.response_choices.lock().index(phrase)
    }
    pub(crate) fn request_response(&self, index: usize) -> i32 {
        assert!(index < self.communication_responses().1);
        *self.response_request.lock() = Some(index);
        1
    }
    pub(crate) fn take_response_request(&self) -> Option<usize> {
        self.response_request.lock().take()
    }

    pub fn observe_star_map(&self, view: Option<crate::engine::observe::StarMapView>) {
        *self
            .star_map
            .lock()
            .unwrap_or_else(|error| panic!("map observation lock poisoned: {error}")) = view;
    }

    pub fn star_map(&self) -> Option<crate::engine::observe::StarMapView> {
        *self
            .star_map
            .lock()
            .unwrap_or_else(|error| panic!("map observation lock poisoned: {error}"))
    }

    pub fn record_ship_escape(&self) {
        self.ship_escape_generation.fetch_add(1, Ordering::SeqCst);
    }

    pub fn ship_escape_generation(&self) -> u64 {
        self.ship_escape_generation.load(Ordering::SeqCst)
    }

    /// Record the current production planet-menu phase.
    pub fn observe_planet_menu(&self, phase: usize) {
        let previous = self.planet_menu_phase.swap(phase, Ordering::AcqRel);
        if phase != PlanetMenuPhase::Inactive as usize && previous != phase {
            self.planet_menu_generation.fetch_add(1, Ordering::AcqRel);
        }
    }

    /// Return the current production planet-menu generation and phase.
    #[must_use]
    pub fn planet_menu_observation(&self) -> (u64, PlanetMenuPhase) {
        (
            self.planet_menu_generation.load(Ordering::Acquire),
            self.planet_menu_phase(),
        )
    }

    /// Return the current production planet-menu phase.
    #[must_use]
    pub fn planet_menu_phase(&self) -> PlanetMenuPhase {
        match self.planet_menu_phase.load(Ordering::Acquire) {
            1 => PlanetMenuPhase::Orbit,
            2 => PlanetMenuPhase::AutoScan,
            3 => PlanetMenuPhase::Dispatch,
            4 => PlanetMenuPhase::LandingSite,
            _ => PlanetMenuPhase::Inactive,
        }
    }

    /// Record whether the Starbase menu, or the Outfit screen it opens, is the
    /// active input loop.
    pub fn observe_starbase_menu(&self, active: bool) {
        self.starbase_menu_active.store(active, Ordering::Release);
    }

    /// Whether the Starbase menu or the Outfit screen is the active input loop.
    #[must_use]
    pub fn starbase_menu_active(&self) -> bool {
        self.starbase_menu_active.load(Ordering::Acquire)
    }

    /// Record entry to or exit from the production Game Options input loop.
    pub fn observe_game_options(&self, active: bool) {
        self.game_options_active.store(active, Ordering::Release);
    }

    pub fn observe_encounter_menu(&self, active: bool) {
        self.encounter_menu_active.store(active, Ordering::Release);
    }

    pub fn encounter_menu_active(&self) -> bool {
        self.encounter_menu_active.load(Ordering::Acquire)
    }

    pub fn observe_ship_picker_cursor(&self, cursor: crate::pickship::Cursor) {
        *self.ship_picker_cursor.lock().expect("picker cursor lock") = cursor;
    }

    pub fn ship_picker_cursor(&self) -> crate::pickship::Cursor {
        *self.ship_picker_cursor.lock().expect("picker cursor lock")
    }

    /// Record entry to or exit from the production battle ship picker.
    ///
    /// Each entry starts a new generation so a pick consumed for one battle cannot
    /// satisfy the picker of the next.
    pub fn observe_ship_picker(&self, active: bool) {
        let previous = self.ship_picker_active.swap(active, Ordering::AcqRel);
        if active && !previous {
            self.ship_picker_generation.fetch_add(1, Ordering::AcqRel);
        }
    }

    /// Return the current ship-picker generation and whether the picker is active.
    #[must_use]
    pub fn ship_picker_observation(&self) -> (u64, bool) {
        (
            self.ship_picker_generation.load(Ordering::Acquire),
            self.ship_picker_active.load(Ordering::Acquire),
        )
    }

    /// Record the number of response choices in the current communication frame.
    pub fn observe_communication_responses(&self, count: usize) {
        let previous = self
            .communication_response_count
            .swap(count, Ordering::AcqRel);
        if count > 0 && previous == 0 {
            self.communication_response_generation
                .fetch_add(1, Ordering::AcqRel);
        }
    }

    /// Current response-list generation and number of choices.
    #[must_use]
    pub fn communication_responses(&self) -> (u64, usize, bool) {
        (
            self.communication_response_generation
                .load(Ordering::Acquire),
            self.communication_response_count.load(Ordering::Acquire),
            self.communication_responses_ready.load(Ordering::Acquire),
        )
    }

    /// Record whether the production response phase can accept Select input.
    pub fn set_communication_responses_ready(&self, ready: bool) {
        self.communication_responses_ready
            .store(ready, Ordering::Release);
    }

    /// Record entry into a production communication input loop.
    pub fn begin_communication(&self) {
        self.observe_choices(&Default::default());
        self.take_response_request();
        self.communication_response_count
            .store(0, Ordering::Release);
        self.communication_responses_ready
            .store(false, Ordering::Release);
        self.communication_active.store(true, Ordering::Release);
    }

    /// Record normal return from a production communication input loop.
    pub fn complete_communication(&self) {
        self.observe_choices(&Default::default());
        self.take_response_request();
        self.communication_responses_ready
            .store(false, Ordering::Release);
        self.communication_active.store(false, Ordering::Release);
        self.communication_completions
            .fetch_add(1, Ordering::AcqRel);
    }

    /// Return whether communication is active and how many loops completed.
    #[must_use]
    pub fn communication_lifecycle(&self) -> (bool, u64) {
        (
            self.communication_active.load(Ordering::Acquire),
            self.communication_completions.load(Ordering::Acquire),
        )
    }

    /// Record entry to or exit from replaying the most recent alien phrase.
    pub fn observe_communication_replay(&self, active: bool) {
        let previous = self
            .communication_replay_active
            .swap(active, Ordering::AcqRel);
        if active && !previous {
            self.communication_replay_generation
                .fetch_add(1, Ordering::AcqRel);
        }
    }

    /// Return the replay generation and whether phrase playback remains active.
    #[must_use]
    pub fn communication_replay_observation(&self) -> (u64, bool) {
        (
            self.communication_replay_generation.load(Ordering::Acquire),
            self.communication_replay_active.load(Ordering::Acquire),
        )
    }

    /// Read the communication loop's current lifecycle and response list.
    #[must_use]
    pub fn conversation_snapshot(&self) -> ConversationSnapshot {
        let (active, completions) = self.communication_lifecycle();
        let (response_generation, response_count, _) = self.communication_responses();
        ConversationSnapshot {
            active,
            completions,
            response_generation,
            response_count,
        }
    }

    /// Verify that the nested production Game Options loop is currently active.
    pub fn verify_game_options_active(&self) -> Result<(), &'static str> {
        if self.game_options_active.load(Ordering::Acquire) {
            Ok(())
        } else {
            Err("Game Options input loop is not active")
        }
    }

    /// Verify that a communication response list containing at least `minimum` entries is active.
    pub fn verify_communication_responses(&self, minimum: usize) -> Result<usize, String> {
        let actual = self.communication_response_count.load(Ordering::Acquire);
        if actual >= minimum {
            Ok(actual)
        } else {
            Err(format!(
                "expected at least {minimum} communication responses, observed {actual}"
            ))
        }
    }
}

impl super::coordinator::AutomationObserver {
    pub(crate) fn observe_ui_checkpoint(&self, view: crate::engine::observe::CheckpointView<'_>) {
        use crate::engine::observe::CheckpointView;
        match view {
            CheckpointView::ShipEscaped => self.ui.record_ship_escape(),
            CheckpointView::EncounterDispatch(id) => self.scene.lock().observe_encounter(id),
            CheckpointView::DialogueDispatch(id) => self.scene.lock().observe_dialogue(id),
            CheckpointView::EncounterMenu(active) => self.ui.observe_encounter_menu(active),
            CheckpointView::CommunicationResponses { count, choices } => {
                self.ui.observe_choices(choices);
                self.ui.observe_communication_responses(count)
            }
            CheckpointView::ResponsesReady(ready) => {
                self.ui.set_communication_responses_ready(ready)
            }
            CheckpointView::CommunicationStarted => self.ui.begin_communication(),
            CheckpointView::CommunicationEnded => self.ui.complete_communication(),
            CheckpointView::CommunicationReplay(active) => {
                self.ui.observe_communication_replay(active)
            }
            CheckpointView::GameOptions(active) => self.observe_game_options(active),
            CheckpointView::PlanetMenu(phase) => self.observe_planet_menu(phase),
            CheckpointView::ShipPicker(active) => self.ui.observe_ship_picker(active),
            CheckpointView::ShipPickerCursor { x, y } => self
                .ui
                .observe_ship_picker_cursor(crate::pickship::Cursor { x, y }),
            CheckpointView::StarbaseMenu(active) => self.ui.observe_starbase_menu(active),
            CheckpointView::StarMap(view) => self.ui.observe_star_map(view),
            CheckpointView::Devices(view) => self.ui.observe_devices(view),
            CheckpointView::Solar(view) => self.ui.observe_solar(view),
            view => self.ending.lock().observe_checkpoint(view),
        }
    }

    pub(crate) fn observe_planet_menu(&self, phase: usize) {
        self.ui.observe_planet_menu(phase);
    }

    pub(crate) fn observe_game_options(&self, active: bool) {
        self.ui.observe_game_options(active);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_options_observation_tracks_entry_and_exit() {
        let ui = UiObservation::default();
        ui.observe_game_options(false);
        assert!(ui.verify_game_options_active().is_err());
        ui.observe_game_options(true);
        assert!(ui.verify_game_options_active().is_ok());
        ui.observe_game_options(false);
        assert!(ui.verify_game_options_active().is_err());
    }

    #[test]
    fn ship_picker_observation_starts_a_generation_per_entry() {
        let ui = UiObservation::default();
        ui.observe_ship_picker(false);
        let (before, active) = ui.ship_picker_observation();
        assert!(!active);
        ui.observe_ship_picker(true);
        assert_eq!(ui.ship_picker_observation(), (before + 1, true));
        ui.observe_ship_picker(true);
        assert_eq!(ui.ship_picker_observation(), (before + 1, true));
        ui.observe_ship_picker(false);
        assert_eq!(ui.ship_picker_observation(), (before + 1, false));
        ui.observe_ship_picker(true);
        assert_eq!(ui.ship_picker_observation(), (before + 2, true));
        ui.observe_ship_picker(false);
    }

    #[test]
    fn starbase_menu_observation_tracks_entry_and_exit() {
        let ui = UiObservation::default();
        ui.observe_starbase_menu(false);
        assert!(!ui.starbase_menu_active());
        ui.observe_starbase_menu(true);
        assert!(ui.starbase_menu_active());
        ui.observe_starbase_menu(false);
        assert!(!ui.starbase_menu_active());
    }

    #[test]
    fn response_observation_requires_the_requested_minimum() {
        let ui = UiObservation::default();
        ui.observe_communication_responses(0);
        let (before, _, _) = ui.communication_responses();
        ui.observe_communication_responses(3);
        assert_eq!(ui.communication_responses(), (before + 1, 3, false));
        assert_eq!(ui.verify_communication_responses(2), Ok(3));
        assert!(ui.verify_communication_responses(4).is_err());
        ui.observe_communication_responses(2);
        assert_eq!(ui.communication_responses(), (before + 1, 2, false));
        ui.set_communication_responses_ready(true);
        assert_eq!(ui.communication_responses(), (before + 1, 2, true));
        ui.observe_communication_responses(0);
        ui.set_communication_responses_ready(false);
    }

    #[test]
    fn planet_menu_observation_requires_a_new_active_phase() {
        let ui = UiObservation::default();
        ui.observe_planet_menu(PlanetMenuPhase::Inactive as usize);
        let (before, _) = ui.planet_menu_observation();
        ui.observe_planet_menu(PlanetMenuPhase::Orbit as usize);
        assert_eq!(
            ui.planet_menu_observation(),
            (before + 1, PlanetMenuPhase::Orbit)
        );
        ui.observe_planet_menu(PlanetMenuPhase::Orbit as usize);
        assert_eq!(
            ui.planet_menu_observation(),
            (before + 1, PlanetMenuPhase::Orbit)
        );
        ui.observe_planet_menu(PlanetMenuPhase::Inactive as usize);
        assert_eq!(
            ui.planet_menu_observation(),
            (before + 1, PlanetMenuPhase::Inactive)
        );
    }

    #[test]
    fn communication_lifecycle_counts_completed_loops() {
        let ui = UiObservation::default();
        let (_, before) = ui.communication_lifecycle();
        ui.begin_communication();
        assert_eq!(ui.communication_lifecycle(), (true, before));
        ui.complete_communication();
        assert_eq!(ui.communication_lifecycle(), (false, before + 1));
    }

    #[test]
    fn communication_replay_observation_tracks_generations_and_activity() {
        let ui = UiObservation::default();
        ui.observe_communication_replay(false);
        let (before, active) = ui.communication_replay_observation();
        assert!(!active);

        ui.observe_communication_replay(true);
        assert_eq!(ui.communication_replay_observation(), (before + 1, true));
        ui.observe_communication_replay(true);
        assert_eq!(ui.communication_replay_observation(), (before + 1, true));
        ui.observe_communication_replay(false);
        assert_eq!(ui.communication_replay_observation(), (before + 1, false));
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
        let ui = UiObservation::default();
        // The previous conversation ended with its list selected and cleared.
        ui.observe_communication_responses(0);
        ui.set_communication_responses_ready(false);
        let (_, completions) = ui.communication_lifecycle();
        let (selected_generation, _, _) = ui.communication_responses();

        // The new hail is dispatched and speech plays: the loop is live but
        // the NPC callback has not registered any choice yet.
        ui.begin_communication();
        let reached = || {
            ui.conversation_snapshot()
                .has_fresh_responses(2, completions, selected_generation)
        };
        assert!(ui.verify_communication_responses(2).is_err());
        assert!(!reached());

        // The callback registers its choices one at a time.
        ui.observe_communication_responses(1);
        assert!(!reached());
        ui.observe_communication_responses(2);
        assert!(reached());
        assert_eq!(ui.verify_communication_responses(2), Ok(2));

        // Once the conversation closes nothing from it can satisfy a wait.
        ui.observe_communication_responses(0);
        ui.complete_communication();
        assert!(!reached());
    }
}

#[cfg(test)]
mod device_tests {
    use super::*;

    #[test]
    fn device_events_are_copied_and_isolated_between_observers() {
        let first = UiObservation::default();
        let second = UiObservation::default();
        let mut observation = crate::engine::observe::DeviceObservation {
            list: Some(vec![1, 3, 12]),
            selected: 2,
            activation: Some((7, 12, "effect".into(), true)),
            ..Default::default()
        };
        first.observe_devices(&observation);
        observation.list = None;
        assert_eq!(first.devices().list, Some(vec![1, 3, 12]));
        assert_eq!(first.devices().selected, 2);
        assert_eq!(first.devices().activation.as_ref().unwrap().0, 7);
        assert_eq!(second.devices(), Default::default());
        first.observe_devices(&observation);
        assert!(first.devices().list.is_none());
        assert_eq!(first.devices().activation.as_ref().unwrap().0, 7);
    }
}
