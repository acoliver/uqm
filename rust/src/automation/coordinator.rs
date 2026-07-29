//! Runtime automation coordinator — wires the scheduler, watchdog, and
//! runtime model to the live game loop.
//!
//! This module is the bridge between the pure model types (scheduler reducer,
//! watchdog reducer, runtime model) and the real C game loop. When
//! automation is active, the FFI hooks in `input_ffi.rs` call into this
//! coordinator to:
//!
//! 1. Feed admitted input callbacks to the scheduler reducer
//! 2. Apply planned effects (write/release menu keys, arm capture)
//! 3. Check terminal/watchdog conditions
//! 4. Write trace records to the ordered commit
//! 5. Signal stop when the script finishes or a terminal condition fires
//!
//! @plan PLAN-20260723-RUNTIME-AUTOMATION.P08
//! @requirement REQ-FFI-001..005, REQ-SCHED-001, REQ-WATCH-001

use crate::automation::input_ffi;
use crate::automation::outcome::TerminalClass;
use crate::automation::runtime::RuntimeModel;
use crate::automation::scenario::{self, PendingStartScene, SceneActivationBoundary};
use crate::automation::scheduler::{
    scheduler_reduce, ActionPhase, CaptureGeneration, EffectPlan, SchedulerConfig, SchedulerEvent,
    SchedulerState, TerminalOutcome,
};
use crate::automation::script::{Action, ValidatedScript};
use crate::automation::trace::{RecordKind, TraceRecord};
use crate::automation::watchdog::{
    watchdog_reduce, CallbackKind, ClockSample, WatchdogEntry, WatchdogLimits, WatchdogOutcome,
};
use parking_lot::Mutex;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

// ===========================================================================
//  Coordinator state (global, single-threaded in RUST_OWNS_MAIN mode)
// ===========================================================================

/// The global automation coordinator. Initialized once when automation is
/// activated, accessed via FFI hooks from the C game loop.
static COORDINATOR: OnceLock<Coordinator> = OnceLock::new();

/// Maximum number of script steps ahead of the current step that semantic
/// communication actions may lookahead. Bounded to keep the search cheap
/// and avoid scanning past unrelated actions.
const SEMANTIC_LOOKAHEAD: usize = 4;
/// Mutable inner state, protected by a Mutex. In RUST_OWNS_MAIN mode this
/// is effectively uncontended (single-threaded), but the Mutex ensures
/// memory safety and Sync-ness.
struct CoordInner {
    sched_state: SchedulerState,
    input_seen: u64,
    present_seen: u64,
    last_observed: Instant,
    trace_seq: u64,
    finalized: bool,
    terminal_class: Option<TerminalClass>,
    /// Queued menu transition events that arrived while the scheduler
    /// was not in WaitingSemantic. Replayed when the scheduler enters
    /// WaitingSemantic.
    pending_transitions: Vec<u8>,
    /// The label of the currently armed capture step, if any.
    /// Used when the capture completes to write a PNG artifact.
    armed_capture_label: Option<String>,
    /// Declarative start scene, consumed once at the game-initialized boundary.
    pending_start_scene: PendingStartScene,
    /// Most recent communication completion consumed by a completion wait.
    consumed_communication_completions: u64,
    /// Most recent dispatch generation consumed by a dispatch wait.
    consumed_dispatch_generation: u64,
    /// Most recent communication response generation selected semantically.
    consumed_response_generation: u64,
    /// Most recent planet-menu generation selected semantically.
    consumed_planet_menu_generation: u64,
    /// Require one callback after orbit navigation before accepting menu ownership.
    orbit_transition_pending: bool,
    /// Release a semantically injected Select key on the next callback.
    release_semantic_select: bool,
}

/// The automation coordinator, holding all live state needed to drive
/// the scheduler/watchdog during the game loop.
pub struct Coordinator {
    /// The validated script actions.
    actions: Vec<Action>,
    /// The typed main-menu transition assertions from the script.
    transitions: Vec<crate::automation::script::MainMenuTransition>,
    /// Watchdog limits from the script budgets.
    watchdog_limits: WatchdogLimits,
    /// Wall-clock start time.
    started_at: Instant,
    /// Output root for artifacts/traces.
    output_root: PathBuf,
    /// The runtime model reference (borrowed from input_ffi's global).
    runtime: &'static RuntimeModel,
    /// Mutable inner state.
    inner: Mutex<CoordInner>,
}

impl Coordinator {
    /// Initialize the global coordinator with a validated script.
    ///
    /// This is called from `main.rs` after `setup_automation` succeeds.
    /// It activates the runtime model and writes the run_start trace.
    pub fn init(script: ValidatedScript, output_root: PathBuf) {
        let budgets = script.budgets();
        let start_scene = script.start_scene();
        let actions = script.steps().to_vec();
        let transitions = script.transitions().to_vec();

        let watchdog_limits = WatchdogLimits {
            max_input_ticks: budgets.max_input_ticks,
            max_presentations: budgets.max_presentations,
            max_wallclock: Duration::from_secs(budgets.max_wallclock_seconds),
        };

        let now = Instant::now();

        // Initialize the runtime model via input_ffi.
        input_ffi::init_automation_runtime();

        let runtime = input_ffi::get_runtime().expect("runtime initialized");

        // Activate the runtime.
        runtime.activate();

        let (_, consumed_communication_completions) =
            crate::automation::ui_observation::communication_lifecycle();

        let coord = Coordinator {
            actions,
            transitions,
            watchdog_limits,
            started_at: now,
            output_root,
            runtime,
            inner: Mutex::new(CoordInner {
                sched_state: SchedulerState::initial(),
                input_seen: 0,
                present_seen: 0,
                last_observed: now,
                trace_seq: 0,
                finalized: false,
                terminal_class: None,
                pending_transitions: Vec::new(),
                armed_capture_label: None,
                pending_start_scene: PendingStartScene::new(start_scene),
                consumed_communication_completions,
                consumed_dispatch_generation: 0,
                consumed_response_generation: 0,
                consumed_planet_menu_generation: 0,
                release_semantic_select: false,
                orbit_transition_pending: false,
            }),
        };

        // Write run_start trace.
        {
            let mut init_inner = coord.inner.lock();
            coord.write_trace(&mut init_inner, RecordKind::RunStart);
        }

        let _ = COORDINATOR.set(coord);
    }

    /// Get the global coordinator if active.
    fn get() -> Option<&'static Coordinator> {
        COORDINATOR.get()
    }

    /// Whether automation is active and the coordinator is initialized.
    pub fn is_active() -> bool {
        Self::get().is_some()
    }

    /// Activate the script's start scene after game structures and initial
    /// events are initialized, before the first activity dispatch.
    pub fn activate_start_scene() {
        let Some(coord) = Self::get() else {
            return;
        };

        let scene = {
            let mut inner = coord.inner.lock();
            match inner
                .pending_start_scene
                .take(SceneActivationBoundary::GameInitialized)
            {
                Ok(scene) => scene,
                Err(error) => {
                    coord.write_trace_labeled(
                        &mut inner,
                        RecordKind::SemanticAssertion,
                        error.to_string(),
                    );
                    coord.set_terminal(&mut inner, TerminalClass::SemanticMismatch);
                    return;
                }
            }
        };

        let Some(scene) = scene else {
            return;
        };

        match scenario::activate(scene, SceneActivationBoundary::GameInitialized) {
            Ok(plan) => {
                let mut inner = coord.inner.lock();
                coord.write_trace_labeled(
                    &mut inner,
                    RecordKind::SemanticAssertion,
                    format!("scene_activated:{}", plan.scene.name()),
                );
            }
            Err(error) => {
                let mut inner = coord.inner.lock();
                coord.write_trace_labeled(
                    &mut inner,
                    RecordKind::SemanticAssertion,
                    error.to_string(),
                );
                coord.set_terminal(&mut inner, TerminalClass::SemanticMismatch);
            }
        }
    }

    // -----------------------------------------------------------------------
    //  Input callback processing (called from rust_automation_service_do_input)
    // -----------------------------------------------------------------------

    /// Process an admitted input callback. Returns true if the game loop
    /// should stop (terminal condition or script finished).
    pub fn process_input() -> bool {
        let Some(coord) = Self::get() else {
            return false;
        };
        coord.process_input_inner()
    }

    /// Whether the current semantic response action is waiting for NPC speech
    /// to finish before its pending response list can become selectable.
    #[must_use]
    pub fn should_skip_pending_communication_speech() -> bool {
        let Some(coord) = Self::get() else {
            return false;
        };
        let inner = coord.inner.lock();
        let Some(Action::SelectCommunicationResponse(select)) =
            coord.actions.get(inner.sched_state.step_index)
        else {
            return false;
        };
        let (generation, count, ready) =
            crate::automation::ui_observation::communication_responses();
        generation > inner.consumed_response_generation && count > select.index && !ready
    }

    /// Report whether a response action later in the current synchronous
    /// prerequisite chain targets the pending communication list.
    #[must_use]
    pub fn pending_communication_response_action(response_count: usize) -> bool {
        let Some(coord) = Self::get() else {
            return false;
        };
        if response_count == 0 {
            return false;
        }
        let inner = coord.inner.lock();
        coord
            .actions
            .get(inner.sched_state.step_index..)
            .unwrap_or_default()
            .iter()
            .take(SEMANTIC_LOOKAHEAD)
            .any(|action| {
                matches!(action, Action::SelectCommunicationResponse(select) if response_count > select.index)
            })
    }

    /// Whether automation is waiting for this communication to close and can
    /// therefore skip its final NPC speech segment.
    #[must_use]
    pub fn should_skip_communication_closing_speech() -> bool {
        let Some(coord) = Self::get() else {
            return false;
        };
        let inner = coord.inner.lock();
        coord
            .actions
            .get(inner.sched_state.step_index..)
            .unwrap_or_default()
            .iter()
            .take(SEMANTIC_LOOKAHEAD)
            .any(|action| matches!(action, Action::WaitForCommunicationEnd(_)))
    }

    /// Advance one scheduler callback at a synchronous boundary and report
    /// whether a pending semantic response action now requires speech skipping.
    #[must_use]
    pub fn service_and_should_skip_pending_communication_speech() -> bool {
        for _ in 0..4 {
            if Self::should_skip_pending_communication_speech() {
                return true;
            }

            let Some(coord) = Self::get() else {
                return false;
            };
            let before = coord.inner.lock().sched_state.step_index;
            if coord.process_input_inner() {
                return false;
            }
            if Self::should_skip_pending_communication_speech() {
                return true;
            }
            let after = coord.inner.lock().sched_state.step_index;
            if after == before {
                return false;
            }
        }
        false
    }

    /// Consume an already-observed communication completion after hail teardown
    /// and expose the next action before returning to the outer game loop.
    pub fn service_communication_completion_boundary() -> bool {
        for _ in 0..2 {
            let Some(coord) = Self::get() else {
                return false;
            };
            let before = coord.inner.lock().sched_state.step_index;
            if coord.process_input_inner() {
                return true;
            }
            let after = coord.inner.lock().sched_state.step_index;
            if after == before {
                break;
            }
        }
        false
    }

    fn process_input_inner(&self) -> bool {
        let mut inner = self.inner.lock();

        if inner.terminal_class.is_some() {
            return true;
        }

        let released_semantic_select = inner.release_semantic_select;
        if released_semantic_select {
            crate::automation::input_ffi::rust_automation_set_immediate_menu_key(
                i32::from(crate::automation::script::MenuKey::Select.index()),
                0,
            );
            inner.release_semantic_select = false;
        }
        let orbit_transition_pending = inner.orbit_transition_pending;
        inner.orbit_transition_pending = false;

        let now = Instant::now();
        let elapsed = now.duration_since(self.started_at);

        // Step 1: Watchdog check.
        let entry = WatchdogEntry {
            kind: CallbackKind::Input,
            input_seen: inner.input_seen,
            present_seen: inner.present_seen,
            elapsed,
            clock: ClockSample {
                started_at: self.started_at,
                last_observed: inner.last_observed,
                now,
            },
        };

        let wd_result = watchdog_reduce(&entry, &self.watchdog_limits);
        inner.last_observed = now;

        // Update counters from watchdog result (post-increment values).
        inner.input_seen = wd_result.candidate_input_seen;
        inner.present_seen = wd_result.candidate_present_seen;

        match wd_result.outcome {
            WatchdogOutcome::Admit => {}
            WatchdogOutcome::InputCounterOverflow
            | WatchdogOutcome::PresentationCounterOverflow => {
                self.set_terminal(&mut inner, TerminalClass::CounterOverflow);
                return true;
            }
            WatchdogOutcome::InputTimeout => {
                self.set_terminal(&mut inner, TerminalClass::InputTimeout);
                return true;
            }
            WatchdogOutcome::PresentationTimeout => {
                self.set_terminal(&mut inner, TerminalClass::PresentationTimeout);
                return true;
            }
            WatchdogOutcome::WallTimeout => {
                self.set_terminal(&mut inner, TerminalClass::WallTimeout);
                return true;
            }
            WatchdogOutcome::ClockRegression => {
                self.set_terminal(&mut inner, TerminalClass::ClockRegression);
                return true;
            }
        }

        // Runtime semantic assertions are checked before allowing the pure
        // scheduler to advance their action.
        if let Some(Action::AssertScene(assertion)) = self.actions.get(inner.sched_state.step_index)
        {
            match scenario::verify(assertion.scene) {
                Ok(plan) => self.write_trace_labeled(
                    &mut inner,
                    RecordKind::SemanticAssertion,
                    format!(
                        "scene_verified:{}:encounter={}:dialogue={}",
                        plan.scene.name(),
                        plan.expected_encounter_conversation,
                        plan.expected_dialogue_conversation
                    ),
                ),
                Err(error) => {
                    self.write_trace_labeled(
                        &mut inner,
                        RecordKind::SemanticAssertion,
                        error.to_string(),
                    );
                    self.set_terminal(&mut inner, TerminalClass::SemanticMismatch);
                    return true;
                }
            }
        }

        if let Some(Action::AssertDispatch(assertion)) =
            self.actions.get(inner.sched_state.step_index)
        {
            match scenario::verify_dispatch(assertion.encounter, assertion.dialogue) {
                Ok(()) => self.write_trace_labeled(
                    &mut inner,
                    RecordKind::SemanticAssertion,
                    format!(
                        "dispatch_verified:encounter={}:dialogue={}",
                        assertion.encounter, assertion.dialogue
                    ),
                ),
                Err(error) => {
                    self.write_trace_labeled(
                        &mut inner,
                        RecordKind::SemanticAssertion,
                        error.to_string(),
                    );
                    self.set_terminal(&mut inner, TerminalClass::SemanticMismatch);
                    return true;
                }
            }
        }

        if matches!(
            self.actions.get(inner.sched_state.step_index),
            Some(Action::AssertGameOptions(_))
        ) {
            match crate::automation::ui_observation::verify_game_options_active() {
                Ok(()) => self.write_trace_labeled(
                    &mut inner,
                    RecordKind::SemanticAssertion,
                    "game_options_active".to_string(),
                ),
                Err(error) => {
                    self.write_trace_labeled(
                        &mut inner,
                        RecordKind::SemanticAssertion,
                        error.to_string(),
                    );
                    self.set_terminal(&mut inner, TerminalClass::SemanticMismatch);
                    return true;
                }
            }
        }

        if let Some(Action::AssertCommunicationResponses(assertion)) =
            self.actions.get(inner.sched_state.step_index)
        {
            match crate::automation::ui_observation::verify_communication_responses(
                assertion.minimum,
            ) {
                Ok(actual) => self.write_trace_labeled(
                    &mut inner,
                    RecordKind::SemanticAssertion,
                    format!("communication_responses_active:count={actual}"),
                ),
                Err(error) => {
                    self.write_trace_labeled(&mut inner, RecordKind::SemanticAssertion, error);
                    self.set_terminal(&mut inner, TerminalClass::SemanticMismatch);
                    return true;
                }
            }
        }

        // Step 2: Feed to scheduler. Runtime waits are satisfied only by
        // observed production state, never by injected input.
        let mut scheduler_event = SchedulerEvent::AdmittedInput;
        if let Some(Action::WaitForCommunicationEnd(wait)) =
            self.actions.get(inner.sched_state.step_index)
        {
            let (active, completions) =
                crate::automation::ui_observation::communication_lifecycle();
            let target = inner
                .consumed_communication_completions
                .saturating_add(wait.minimum_completions);
            if !active && completions >= target {
                inner.consumed_communication_completions = completions;
                scheduler_event = SchedulerEvent::ConditionReached;
                self.write_trace_labeled(
                    &mut inner,
                    RecordKind::SemanticAssertion,
                    format!(
                        "communication_completed:count={completions}:minimum={}",
                        wait.minimum_completions
                    ),
                );
            }
        } else if let Some(Action::SelectCommunicationResponse(select)) =
            self.actions.get(inner.sched_state.step_index)
        {
            let (generation, count, ready) =
                crate::automation::ui_observation::communication_responses();
            if !released_semantic_select
                && generation > inner.consumed_response_generation
                && count > select.index
            {
                if ready {
                    crate::comm::ffi::rust_SelectResponseIndex(select.index as i32);
                    crate::automation::input_ffi::rust_automation_set_immediate_menu_key(
                        i32::from(crate::automation::script::MenuKey::Select.index()),
                        1,
                    );
                    inner.release_semantic_select = true;
                    inner.consumed_response_generation = generation;
                    scheduler_event = SchedulerEvent::ConditionReached;
                    self.write_trace_labeled(
                        &mut inner,
                        RecordKind::SemanticAssertion,
                        format!(
                            "communication_response_selected:generation={generation}:count={count}:index={}",
                            select.index
                        ),
                    );
                } else {
                    let _ = crate::sound::trackplayer::jump_track(0);
                }
            }
        } else if let Some(Action::SelectPlanetMenu(select)) =
            self.actions.get(inner.sched_state.step_index)
        {
            use crate::automation::script::PlanetMenuPhaseName;
            use crate::automation::ui_observation::PlanetMenuPhase;
            let expected = match select.phase {
                PlanetMenuPhaseName::Orbit => PlanetMenuPhase::Orbit,
                PlanetMenuPhaseName::AutoScan => PlanetMenuPhase::AutoScan,
                PlanetMenuPhaseName::Dispatch => PlanetMenuPhase::Dispatch,
                PlanetMenuPhaseName::LandingSite => PlanetMenuPhase::LandingSite,
            };
            let (generation, phase) = crate::automation::ui_observation::planet_menu_observation();
            if !released_semantic_select
                && !orbit_transition_pending
                && generation > inner.consumed_planet_menu_generation
                && phase == expected
            {
                crate::automation::input_ffi::rust_automation_set_immediate_menu_key(
                    i32::from(crate::automation::script::MenuKey::Select.index()),
                    1,
                );
                inner.release_semantic_select = true;
                inner.consumed_planet_menu_generation = generation;
                scheduler_event = SchedulerEvent::ConditionReached;
                self.write_trace_labeled(
                    &mut inner,
                    RecordKind::SemanticAssertion,
                    format!(
                        "planet_menu_selected:generation={generation}:phase={:?}",
                        select.phase
                    ),
                );
            }
        } else if matches!(
            self.actions.get(inner.sched_state.step_index),
            Some(Action::WaitForPlanetSideStart(_))
        ) {
            let observation = crate::planet_side::telemetry::observation();
            if observation.active {
                scheduler_event = SchedulerEvent::ConditionReached;
                self.write_trace_labeled(
                    &mut inner,
                    RecordKind::SemanticAssertion,
                    format!(
                        "planet_side_started:generation={}:crew={}:position={},{}",
                        observation.generation,
                        observation.start_crew,
                        observation.start_x,
                        observation.start_y
                    ),
                );
            }
        } else if let Some(Action::WaitForDispatch(wait)) =
            self.actions.get(inner.sched_state.step_index)
        {
            let (generation, encounter, dialogue) = scenario::dispatch_observation();
            if generation > inner.consumed_dispatch_generation
                && encounter == Some(wait.encounter)
                && dialogue == Some(wait.dialogue)
            {
                inner.consumed_dispatch_generation = generation;
                scheduler_event = SchedulerEvent::ConditionReached;
                self.write_trace_labeled(
                    &mut inner,
                    RecordKind::SemanticAssertion,
                    format!(
                        "dispatch_observed:generation={generation}:encounter={}:dialogue={}",
                        wait.encounter, wait.dialogue
                    ),
                );
            }
        }

        // Navigation actions derive real player controls from the live
        // solar-system state before UpdateInputState.
        if let Some(Action::NavigateToPlanet(navigation)) =
            self.actions.get(inner.sched_state.step_index)
        {
            let snapshot = crate::automation::input_ffi::navigation_snapshot(
                i32::from(navigation.planet),
                None,
            );
            let reached =
                snapshot.active != 0 && snapshot.inner_planet == i32::from(navigation.planet);
            let control = crate::automation::navigation::steer_toward_target(
                crate::automation::navigation::NavigationObservation {
                    active: snapshot.active != 0,
                    inner_planet: u8::try_from(snapshot.inner_planet).ok(),
                    in_orbit: snapshot.in_orbit != 0,
                    ship_x: snapshot.ship_x,
                    ship_y: snapshot.ship_y,
                    ship_facing: u8::try_from(snapshot.ship_facing).unwrap_or(0),
                    target_x: snapshot.target_x,
                    target_y: snapshot.target_y,
                    velocity_x: snapshot.velocity_x,
                    velocity_y: snapshot.velocity_y,
                    view_center_x: snapshot.view_center_x,
                    view_center_y: snapshot.view_center_y,
                },
            );
            if reached {
                Self::set_navigation_controls(
                    crate::automation::navigation::NavigationControl::default(),
                );
                scheduler_event = SchedulerEvent::NavigationReached;
                self.write_trace_labeled(
                    &mut inner,
                    RecordKind::SemanticAssertion,
                    format!("navigation_reached:planet={}", navigation.planet),
                );
            } else {
                Self::set_navigation_controls(control);
            }
        } else if let Some(Action::NavigateToOrbit(navigation)) =
            self.actions.get(inner.sched_state.step_index)
        {
            let snapshot = crate::automation::input_ffi::navigation_snapshot(
                i32::from(navigation.planet),
                None,
            );
            let reached = snapshot.active != 0
                && snapshot.in_orbit != 0
                && snapshot.inner_planet == i32::from(navigation.planet)
                && snapshot.orbital_moon < 0;
            let control = crate::automation::navigation::steer_toward_target(
                crate::automation::navigation::NavigationObservation {
                    active: snapshot.active != 0,
                    inner_planet: None,
                    in_orbit: false,
                    ship_x: snapshot.ship_x,
                    ship_y: snapshot.ship_y,
                    ship_facing: u8::try_from(snapshot.ship_facing).unwrap_or(0),
                    target_x: snapshot.target_x,
                    target_y: snapshot.target_y,
                    velocity_x: snapshot.velocity_x,
                    velocity_y: snapshot.velocity_y,
                    view_center_x: snapshot.view_center_x,
                    view_center_y: snapshot.view_center_y,
                },
            );
            if reached {
                Self::set_navigation_controls(
                    crate::automation::navigation::NavigationControl::default(),
                );
                inner.orbit_transition_pending = true;
                scheduler_event = SchedulerEvent::NavigationReached;
                self.write_trace_labeled(
                    &mut inner,
                    RecordKind::SemanticAssertion,
                    format!("orbit_reached:planet={}", navigation.planet),
                );
            } else {
                Self::set_navigation_controls(control);
            }
        } else if let Some(Action::NavigateToMoon(navigation)) =
            self.actions.get(inner.sched_state.step_index)
        {
            let snapshot = crate::automation::input_ffi::navigation_snapshot(
                i32::from(navigation.planet),
                Some(i32::from(navigation.moon)),
            );
            // `enterOrbital` is the single production commitment point for a
            // moon: it sets InOrbit and repoints pOrbitalDesc at the moon.
            // The automation hook runs at the top of the next DoInput
            // iteration, before DoIpFlight consumes InOrbit, so this state is
            // observed exactly once per commitment. WaitIntersect must not be
            // consulted here: CheckIntersect assigns it the target's own
            // MAKE_WORD immediately before returning the descriptor that
            // triggers enterOrbital, so it always equals the target at this
            // instant.
            let reached_orbit = snapshot.active != 0
                && snapshot.in_orbit != 0
                && snapshot.inner_planet == i32::from(navigation.planet)
                && snapshot.orbital_moon == i32::from(navigation.moon)
                && snapshot.orbital_data_index == snapshot.target_data_index;
            let control = crate::automation::navigation::steer_moon_navigation(
                crate::automation::navigation::NavigationObservation {
                    active: snapshot.active != 0,
                    inner_planet: None,
                    in_orbit: snapshot.in_orbit != 0,
                    ship_x: snapshot.ship_x,
                    ship_y: snapshot.ship_y,
                    ship_facing: u8::try_from(snapshot.ship_facing).unwrap_or(0),
                    target_x: snapshot.target_x,
                    target_y: snapshot.target_y,
                    velocity_x: snapshot.velocity_x,
                    velocity_y: snapshot.velocity_y,
                    view_center_x: snapshot.view_center_x,
                    view_center_y: snapshot.view_center_y,
                },
            );
            if let ActionPhase::Navigating { remaining } = inner.sched_state.phase {
                if remaining % 250 == 0 {
                    self.write_trace_labeled(
                        &mut inner,
                        RecordKind::SemanticAssertion,
                        format!(
                            "moon_navigation_state:remaining={remaining}:active={}:in_ip_flight={}:in_orbit={}:wait_intersect={}:inner={}:moon={}:ship={},{}:target={},{}:orbital_data={}:target_data={}",
                            snapshot.active,
                            snapshot.in_ip_flight,
                            snapshot.in_orbit,
                            snapshot.wait_intersect,
                            snapshot.inner_planet,
                            snapshot.orbital_moon,
                            snapshot.ship_x,
                            snapshot.ship_y,
                            snapshot.target_x,
                            snapshot.target_y,
                            snapshot.orbital_data_index,
                            snapshot.target_data_index
                        ),
                    );
                }
            }
            if reached_orbit {
                Self::set_navigation_controls(
                    crate::automation::navigation::NavigationControl::default(),
                );
                scheduler_event = SchedulerEvent::NavigationReached;
                self.write_trace_labeled(
                    &mut inner,
                    RecordKind::SemanticAssertion,
                    format!(
                        "navigation_reached:planet={}:moon={}:orbital_data={}:target_data={}",
                        navigation.planet,
                        navigation.moon,
                        snapshot.orbital_data_index,
                        snapshot.target_data_index
                    ),
                );
            } else {
                Self::set_navigation_controls(control);
            }
        }

        let config = SchedulerConfig {
            actions: &self.actions,
            transitions: &self.transitions,
        };
        let transition = scheduler_reduce(&inner.sched_state, &config, scheduler_event);
        inner.sched_state = transition.new_state;

        // Step 3: Apply effects.
        self.apply_effects(&mut inner, &transition.effects);

        // Step 4: Write trace.
        self.write_trace(&mut inner, RecordKind::InputTick);

        // Step 4b: If the scheduler just entered WaitingSemantic, replay
        // any pending menu transitions that arrived before the scheduler
        // was ready.
        if inner.sched_state.phase == crate::automation::scheduler::ActionPhase::WaitingSemantic
            && !inner.pending_transitions.is_empty()
        {
            let config2 = SchedulerConfig {
                actions: &self.actions,
                transitions: &self.transitions,
            };
            let pending: Vec<u8> = inner.pending_transitions.drain(..).collect();
            for to in pending {
                eprintln!("[automation] replaying pending menu_transition to={}", to);
                let t2 = scheduler_reduce(
                    &inner.sched_state,
                    &config2,
                    SchedulerEvent::MenuTransition { to },
                );
                inner.sched_state = t2.new_state;
                self.write_trace(&mut inner, RecordKind::MenuTransition);
                if inner.sched_state.is_terminal() {
                    break;
                }
            }
        }

        // Step 5: Check terminal.
        if inner.sched_state.is_terminal() {
            let class = map_scheduler_terminal(inner.sched_state.terminal);
            eprintln!(
                "[automation] scheduler terminal: {:?} -> class={:?}",
                inner.sched_state.terminal, class
            );
            self.set_terminal(&mut inner, class);
            return true;
        }

        false
    }

    // -----------------------------------------------------------------------
    //  Present callback processing (called from present observation hook)
    // -----------------------------------------------------------------------

    /// Process a committed present callback. Returns true if the game loop
    /// should stop.
    pub fn process_present(generation: u64) -> bool {
        let Some(coord) = Self::get() else {
            return false;
        };
        coord.process_present_inner(generation)
    }

    fn process_present_inner(&self, generation: u64) -> bool {
        let mut inner = self.inner.lock();

        if inner.terminal_class.is_some() {
            return true;
        }

        let now = Instant::now();
        let elapsed = now.duration_since(self.started_at);

        // Watchdog check for present callback.
        let entry = WatchdogEntry {
            kind: CallbackKind::Present,
            input_seen: inner.input_seen,
            present_seen: inner.present_seen,
            elapsed,
            clock: ClockSample {
                started_at: self.started_at,
                last_observed: inner.last_observed,
                now,
            },
        };

        let wd_result = watchdog_reduce(&entry, &self.watchdog_limits);
        inner.last_observed = now;

        inner.input_seen = wd_result.candidate_input_seen;
        inner.present_seen = wd_result.candidate_present_seen;

        match wd_result.outcome {
            WatchdogOutcome::Admit => {}
            WatchdogOutcome::InputCounterOverflow
            | WatchdogOutcome::PresentationCounterOverflow => {
                self.set_terminal(&mut inner, TerminalClass::CounterOverflow);
                return true;
            }
            WatchdogOutcome::InputTimeout => {
                self.set_terminal(&mut inner, TerminalClass::InputTimeout);
                return true;
            }
            WatchdogOutcome::PresentationTimeout => {
                self.set_terminal(&mut inner, TerminalClass::PresentationTimeout);
                return true;
            }
            WatchdogOutcome::WallTimeout => {
                self.set_terminal(&mut inner, TerminalClass::WallTimeout);
                return true;
            }
            WatchdogOutcome::ClockRegression => {
                self.set_terminal(&mut inner, TerminalClass::ClockRegression);
                return true;
            }
        }

        let config = SchedulerConfig {
            actions: &self.actions,
            transitions: &self.transitions,
        };
        let transition = scheduler_reduce(
            &inner.sched_state,
            &config,
            SchedulerEvent::CommittedPresent {
                generation: CaptureGeneration(generation),
            },
        );
        inner.sched_state = transition.new_state;

        self.apply_effects(&mut inner, &transition.effects);

        self.write_trace(&mut inner, RecordKind::Presentation);

        if inner.sched_state.is_terminal() {
            let class = map_scheduler_terminal(inner.sched_state.terminal);
            self.set_terminal(&mut inner, class);
            return true;
        }

        false
    }

    // -----------------------------------------------------------------------
    //  Menu transition observation (called from handle_navigate)
    // -----------------------------------------------------------------------

    /// Process an observed main-menu transition. Returns true if the game
    /// loop should stop (e.g., semantic assertion mismatch).
    pub fn process_menu_transition(to_index: u8) -> bool {
        let Some(coord) = Self::get() else {
            return false;
        };
        coord.process_menu_transition_inner(to_index)
    }

    fn process_menu_transition_inner(&self, to_index: u8) -> bool {
        let mut inner = self.inner.lock();

        if inner.terminal_class.is_some() {
            return true;
        }

        let config = SchedulerConfig {
            actions: &self.actions,
            transitions: &self.transitions,
        };

        // If the scheduler is not in WaitingSemantic, queue the transition.
        // It will be replayed when the scheduler enters WaitingSemantic.
        if inner.sched_state.phase != crate::automation::scheduler::ActionPhase::WaitingSemantic {
            eprintln!(
                "[automation] menu_transition to={} queued (phase={:?})",
                to_index, inner.sched_state.phase
            );
            inner.pending_transitions.push(to_index);
            return false;
        }

        // Process pending transitions first.
        let mut to_process: Vec<u8> = inner.pending_transitions.drain(..).collect();
        to_process.push(to_index);

        for to in to_process {
            eprintln!("[automation] menu_transition to={} processing", to);
            let transition = scheduler_reduce(
                &inner.sched_state,
                &config,
                SchedulerEvent::MenuTransition { to },
            );
            inner.sched_state = transition.new_state;

            self.write_trace(&mut inner, RecordKind::MenuTransition);

            if inner.sched_state.is_terminal() {
                let class = map_scheduler_terminal(inner.sched_state.terminal);
                self.set_terminal(&mut inner, class);
                return true;
            }
        }

        false
    }

    // -----------------------------------------------------------------------
    //  Finalization
    // -----------------------------------------------------------------------

    /// Finalize the automation run. Writes run_end trace, finalizes the
    /// runtime model, and writes the teardown receipt.
    pub fn finalize() {
        let Some(coord) = Self::get() else {
            return;
        };
        coord.finalize_inner();
    }

    fn finalize_inner(&self) {
        eprintln!("[automation] finalize_inner: starting");
        let mut inner = self.inner.lock();

        if inner.finalized {
            eprintln!("[automation] finalize_inner: already finalized, returning");
            return;
        }

        inner.finalized = true;

        // Write run_end trace.
        self.write_trace(&mut inner, RecordKind::RunEnd);
        eprintln!(
            "[automation] finalize_inner: wrote run_end trace, seq={}",
            inner.trace_seq
        );

        // Finalize the runtime model.
        let _ = self.runtime.finalize();

        // Deactivate.
        self.runtime.deactivate();

        // Flush trace records to file.
        let trace_path = self.output_root.join("trace.jsonl");
        drop(inner);

        eprintln!(
            "[automation] finalize_inner: flushing trace to {}",
            trace_path.display()
        );
        match std::fs::File::create(&trace_path) {
            Ok(mut file) => {
                use std::io::Write;
                let _ = file.write_all(b"");
                if let Err(e) = self.runtime.commit.publish_all(&mut file) {
                    eprintln!("[automation] finalize_inner: trace flush error: {e}");
                }
                let _ = file.flush();
            }
            Err(e) => {
                eprintln!("[automation] finalize_inner: FAILED to create trace file: {e}");
            }
        }

        // Write teardown receipt.
        let terminal = self.inner.lock().terminal_class;

        eprintln!(
            "[automation] finalize_inner: writing teardown receipt to {}",
            self.output_root.display()
        );
        match crate::automation::lifecycle::write_teardown_receipt(&self.output_root, terminal, 0) {
            Ok(result) => {
                eprintln!(
                    "[automation] finalize_inner: receipt written to {}",
                    result.final_path.display()
                );
            }
            Err(e) => {
                eprintln!("[automation] finalize_inner: FAILED to write receipt: {e}");
            }
        }
    }

    // -----------------------------------------------------------------------
    //  Internal helpers
    // -----------------------------------------------------------------------
    /// Set a terminal outcome on both the coordinator and the runtime mirror.
    /// Also propagates the stop to the game loop by setting CHECK_ABORT
    /// and MainExited to force the game loop to exit.
    fn set_terminal(&self, inner: &mut CoordInner, class: TerminalClass) {
        inner.terminal_class = Some(class);
        self.runtime.mirror.terminal.try_set(class);

        // Propagate stop to the C game loop: set CHECK_ABORT so the
        // activity state machine exits, and set MainExited so the
        // outer game loop stops requesting new games.
        // CHECK_ABORT = 0x4000 (setup.h).
        #[cfg(feature = "linked_c_archive")]
        unsafe {
            crate::mainloop::c_extern::set_current_activity(
                crate::mainloop::c_extern::get_current_activity() | 0x4000,
            );
            crate::mainloop::ffi::set_main_exited(true);
        }
    }

    fn set_navigation_controls(control: crate::automation::navigation::NavigationControl) {
        use crate::automation::script::PlayerKey;

        for (key, active) in [
            (PlayerKey::Thrust, control.thrust),
            (PlayerKey::Left, control.left),
            (PlayerKey::Right, control.right),
            (PlayerKey::Escape, control.escape),
        ] {
            crate::automation::input_ffi::rust_automation_set_immediate_player_key(
                i32::from(key.index()),
                i32::from(active),
            );
        }
    }

    /// Apply planned effects from the scheduler reducer.
    fn apply_effects(&self, inner: &mut CoordInner, effects: &EffectPlan) {
        // Note: `inner` is `&mut` so callers can pass it as mutable.
        if let Some((key, value)) = effects.write_key {
            let index = crate::automation::input::menu_key_to_index(key);
            crate::automation::input_ffi::rust_automation_set_immediate_menu_key(
                i32::from(index),
                i32::from(value),
            );
        }
        if let Some(key) = effects.release_key {
            let index = crate::automation::input::menu_key_to_index(key);
            crate::automation::input_ffi::rust_automation_set_immediate_menu_key(
                i32::from(index),
                0,
            );
        }
        if let Some((key, value)) = effects.write_player_key {
            crate::automation::input_ffi::rust_automation_set_immediate_player_key(
                i32::from(key.index()),
                i32::from(value),
            );
        }
        if let Some(key) = effects.release_player_key {
            crate::automation::input_ffi::rust_automation_set_immediate_player_key(
                i32::from(key.index()),
                0,
            );
        }
        if let Some(gen) = effects.arm_capture {
            self.runtime.mirror.set_capture_generation(gen.0);
            // Store the capture label from the current action so we can
            // write the PNG when the capture completes.
            if let Some(Action::Capture(step)) = self.actions.get(inner.sched_state.step_index) {
                inner.armed_capture_label = Some(step.label.clone());
            }
        }
        if let Some(gen) = effects.complete_capture {
            self.runtime.mirror.clear_capture_generation();
            // Capture generation matched — read the SDL surface and write
            // a PNG artifact.
            let label = inner
                .armed_capture_label
                .take()
                .unwrap_or_else(|| format!("capture_gen{}", gen.0));
            self.capture_surface(inner, &label, gen);
        }
    }

    /// Write a trace record through the ordered commit.
    fn write_trace(&self, inner: &mut CoordInner, kind: RecordKind) {
        let seq = inner.trace_seq;
        inner.trace_seq = inner.trace_seq.saturating_add(1);

        let record = TraceRecord {
            schema: TraceRecord::SCHEMA,
            run: 1,
            sequence: seq,
            input_seen: inner.input_seen,
            present_seen: inner.present_seen,
            elapsed_ms: self.started_at.elapsed().as_millis() as u64,
            kind,
            label: None,
            from: None,
            to: None,
            terminal_reason: None,
        };

        if let Ok(jsonl) = record.to_jsonl() {
            let res = self.runtime.commit.reserve_sequence(seq);
            res.commit_record(jsonl);
        }
    }

    /// Write a trace record with a semantic/evidence label.
    fn write_trace_labeled(&self, inner: &mut CoordInner, kind: RecordKind, label: String) {
        let seq = inner.trace_seq;
        inner.trace_seq = inner.trace_seq.saturating_add(1);

        let record = TraceRecord {
            schema: TraceRecord::SCHEMA,
            run: 1,
            sequence: seq,
            input_seen: inner.input_seen,
            present_seen: inner.present_seen,
            elapsed_ms: self.started_at.elapsed().as_millis() as u64,
            kind,
            label: Some(label),
            from: None,
            to: None,
            terminal_reason: None,
        };

        if let Ok(jsonl) = record.to_jsonl() {
            let res = self.runtime.commit.reserve_sequence(seq);
            res.commit_record(jsonl);
        }
    }

    /// Capture the logical main SDL surface (screen 0) and write a PNG
    /// artifact via the durable file helper.
    ///
    /// This is called when the scheduler reports `complete_capture` —
    /// the present callback has fired with the correct generation,
    /// meaning the surface is in a consistent state for reading.
    ///
    /// The capture uses the ABI-authoritative SDL surface accessors
    /// (REQ-SHOT-002), the shared lock-copy-unlock helper (REQ-SHOT-003),
    /// and writes a durable PNG file (REQ-SHOT-004/005).
    fn capture_surface(&self, inner: &mut CoordInner, label: &str, gen: CaptureGeneration) {
        // Write a capture trace record.
        let capture_rec = crate::automation::capture::capture_trace_record(
            inner.trace_seq,
            self.started_at.elapsed().as_millis() as u64,
            gen,
            label,
        );
        inner.trace_seq = inner.trace_seq.saturating_add(1);
        if let Ok(jsonl) = capture_rec.to_jsonl() {
            let res = self.runtime.commit.reserve_sequence(capture_rec.sequence);
            res.commit_record(jsonl);
        }

        // Get the logical main surface (screen 0) via the graphics FFI.
        #[cfg(feature = "linked_c_archive")]
        {
            use crate::automation::capture::{validate_surface, SurfaceMetadata};
            use std::ffi::c_void;

            // Get the SDL surface pointer from the Rust graphics subsystem.
            let surface: *mut c_void =
                unsafe { crate::graphics::ffi::rust_gfx_get_sdl_screen() as *mut c_void };
            if surface.is_null() {
                eprintln!("[automation] capture: surface is null, skipping PNG");
                return;
            }

            // Query surface metadata via ABI-authoritative accessors.
            let info = unsafe { crate::graphics::sdl_capture::query_surface_info(surface) };

            let meta = SurfaceMetadata {
                width: info.width,
                height: info.height,
                pitch: info.pitch,
                bpp: info.bpp,
                bytes_per_pixel: info.bytes_per_pixel,
                r_mask: info.rmask,
                g_mask: info.gmask,
                b_mask: info.bmask,
                a_mask: info.amask,
            };

            if let Err(e) = validate_surface(&meta) {
                eprintln!("[automation] capture: surface validation failed: {e:?}");
                return;
            }

            // Compute safe copy length per row.
            let row_copy = match crate::automation::capture::safe_row_copy(
                info.width,
                info.pitch,
                info.bytes_per_pixel,
            ) {
                Some(r) => r,
                None => {
                    eprintln!("[automation] capture: safe_row_copy failed");
                    return;
                }
            };

            let height_u = match u64::try_from(info.height) {
                Ok(h) => h,
                Err(_) => {
                    eprintln!("[automation] capture: height overflow");
                    return;
                }
            };

            let buf_size = match u64::from(row_copy).checked_mul(height_u) {
                Some(s) => s,
                None => {
                    eprintln!("[automation] capture: buffer size overflow");
                    return;
                }
            };

            let buf_size_usize = match usize::try_from(buf_size) {
                Ok(s) => s,
                Err(_) => {
                    eprintln!("[automation] capture: buffer size too large");
                    return;
                }
            };

            // Allocate pixel buffer and copy via shared lock-copy-unlock.
            let mut pixel_buf = vec![0u8; buf_size_usize];
            let copy_result = unsafe {
                crate::graphics::sdl_capture::lock_copy_unlock(
                    surface,
                    pixel_buf.as_mut_ptr(),
                    buf_size_usize,
                )
            };

            match copy_result {
                Ok(()) => {}
                Err(e) => {
                    eprintln!("[automation] capture: lock_copy_unlock failed: {e}");
                    return;
                }
            }

            // Encode PNG using the `image` crate.
            let width_u32 = match u32::try_from(info.width) {
                Ok(w) => w,
                Err(_) => return,
            };
            let height_u32 = match u32::try_from(info.height) {
                Ok(h) => h,
                Err(_) => return,
            };

            // Swizzle pixel data from SDL surface format to RGBA8.
            // SDL on macOS typically uses [pad, R, G, B] (ARGB) or [B, G, R, pad]
            // depending on the surface format. Use the masks to determine the
            // actual layout and convert to standard [R, G, B, A] for PNG encoding.
            let r_mask = info.rmask;
            let g_mask = info.gmask;
            let b_mask = info.bmask;
            let a_mask = info.amask;

            // Determine byte positions from masks (for 32-bit surfaces).
            let r_shift = mask_to_shift(r_mask);
            let g_shift = mask_to_shift(g_mask);
            let b_shift = mask_to_shift(b_mask);
            let a_shift = mask_to_shift(a_mask);

            for chunk in pixel_buf.chunks_exact_mut(4) {
                let pixel = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                let r = if r_mask != 0 {
                    ((pixel & r_mask) >> r_shift) as u8
                } else {
                    0
                };
                let g = if g_mask != 0 {
                    ((pixel & g_mask) >> g_shift) as u8
                } else {
                    0
                };
                let b = if b_mask != 0 {
                    ((pixel & b_mask) >> b_shift) as u8
                } else {
                    0
                };
                let a = if a_mask != 0 {
                    ((pixel & a_mask) >> a_shift) as u8
                } else {
                    255
                };
                chunk[0] = r;
                chunk[1] = g;
                chunk[2] = b;
                chunk[3] = a;
            }

            let rgba_image = match image::RgbaImage::from_raw(width_u32, height_u32, pixel_buf) {
                Some(img) => img,
                None => {
                    eprintln!("[automation] capture: RgbaImage::from_raw failed");
                    return;
                }
            };

            let mut png_data = Vec::new();
            let encoder = image::codecs::png::PngEncoder::new(&mut png_data);
            if let Err(e) = image::ImageEncoder::write_image(
                encoder,
                rgba_image.as_raw(),
                width_u32,
                height_u32,
                image::ExtendedColorType::Rgba8,
            ) {
                eprintln!("[automation] capture: PNG encode failed: {e}");
                return;
            }

            // Write the PNG via the durable file helper.
            let capture_dir = self.output_root.join("captures");
            match crate::automation::artifact::write_durable(&capture_dir, label, "png", &png_data)
            {
                Ok(result) => {
                    eprintln!(
                        "[automation] capture: wrote {} ({} bytes)",
                        result.final_path.display(),
                        png_data.len()
                    );
                }
                Err(e) => {
                    eprintln!("[automation] capture: write_durable failed: {e}");
                }
            }
        }

        #[cfg(not(feature = "linked_c_archive"))]
        {
            let _ = (inner, label, gen);
            eprintln!("[automation] capture: linked_c_archive feature not enabled, skipping PNG");
        }
    }
}

/// Compute the bit shift for a color mask (number of trailing zeros).
#[cfg(feature = "linked_c_archive")]
fn mask_to_shift(mask: u32) -> u32 {
    if mask == 0 {
        0
    } else {
        mask.trailing_zeros()
    }
}

/// Map a scheduler TerminalOutcome to a TerminalClass for the runtime mirror.
fn map_scheduler_terminal(terminal: Option<TerminalOutcome>) -> TerminalClass {
    match terminal {
        Some(TerminalOutcome::FinishComplete) => TerminalClass::Success,
        Some(TerminalOutcome::SemanticMismatch) => TerminalClass::SemanticMismatch,
        Some(TerminalOutcome::CaptureMismatch) => TerminalClass::CaptureMismatch,
        Some(TerminalOutcome::StateVersionOverflow) => TerminalClass::StateVersionOverflow,
        Some(TerminalOutcome::CaptureGenerationOverflow) => {
            TerminalClass::CaptureGenerationOverflow
        }
        None => TerminalClass::CooperativeStop,
    }
}

// ===========================================================================
//  Unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinator_not_active_by_default() {
        assert!(!Coordinator::is_active());
    }

    #[test]
    fn map_finish_complete_to_success() {
        assert_eq!(
            map_scheduler_terminal(Some(TerminalOutcome::FinishComplete)),
            TerminalClass::Success
        );
    }

    #[test]
    fn map_semantic_mismatch() {
        assert_eq!(
            map_scheduler_terminal(Some(TerminalOutcome::SemanticMismatch)),
            TerminalClass::SemanticMismatch
        );
    }

    #[test]
    fn map_capture_mismatch() {
        assert_eq!(
            map_scheduler_terminal(Some(TerminalOutcome::CaptureMismatch)),
            TerminalClass::CaptureMismatch
        );
    }

    #[test]
    fn map_none_to_cooperative_stop() {
        assert_eq!(map_scheduler_terminal(None), TerminalClass::CooperativeStop);
    }
}
