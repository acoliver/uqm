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
use crate::automation::runtime::{FinalizationResult, RuntimeModel};
use crate::automation::scenario::{self, PendingStartScene, SceneActivationBoundary};
use crate::automation::scheduler::{
    scheduler_reduce, ActionPhase, CaptureGeneration, EffectPlan, SchedulerConfig, SchedulerEvent,
    SchedulerState, TerminalOutcome,
};
use crate::automation::script::{Action, ActivityAssertion, ValidatedScript};
use crate::automation::trace::{
    ActivityEvidence, RecordKind, SeedApplication, SeedDomain, TraceRecord,
};
use crate::automation::watchdog::{
    watchdog_reduce, CallbackKind, ClockSample, WatchdogEntry, WatchdogLimits, WatchdogOutcome,
};
use parking_lot::Mutex;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// Fixed RNG seed applied once when active automation enters gameplay.
pub const AUTOMATION_SEED: u32 = 0x55AA_2317;

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
    /// Most recent communication replay generation consumed semantically.
    consumed_replay_generation: u64,
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
                consumed_replay_generation: 0,
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
        if coord.halt_on_rejected_injection() {
            return true;
        }
        coord.process_input_inner()
    }

    /// Stop the run if the native owner ever refused an input write.
    ///
    /// The script's action never reached the game, so continuing would assert
    /// against state the automation never actually produced.
    fn halt_on_rejected_injection(&self) -> bool {
        if !crate::automation::input_ffi::injection_rejected() {
            return false;
        }
        let mut inner = self.inner.lock();
        if inner.terminal_class.is_none() {
            self.set_terminal(&mut inner, TerminalClass::SemanticMismatch);
        }
        true
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
        matches!(
            coord.actions.get(inner.sched_state.step_index),
            Some(Action::WaitForCommunicationEnd(_))
        )
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
            crate::automation::input_ffi::inject_menu_key(
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

        if self.verify_runtime_assertions(&mut inner) || self.verify_activity_assertion(&mut inner)
        {
            return true;
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
        } else if matches!(
            self.actions.get(inner.sched_state.step_index),
            Some(Action::WaitForCommunicationReplay(_))
        ) {
            let (generation, _) =
                crate::automation::ui_observation::communication_replay_observation();
            if consume_new_generation(&mut inner.consumed_replay_generation, generation) {
                scheduler_event = SchedulerEvent::ConditionReached;
                self.write_trace_labeled(
                    &mut inner,
                    RecordKind::SemanticAssertion,
                    "communication_replay_active".to_owned(),
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
                    crate::automation::input_ffi::inject_menu_key(
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
                crate::automation::input_ffi::inject_menu_key(
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
        self.apply_effects(&mut inner, &transition.effects, None);

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
                let label = if inner.sched_state.terminal == Some(TerminalOutcome::SemanticMismatch)
                {
                    format!("menu_transition_failed:to={to}")
                } else {
                    format!("menu_transition_passed:to={to}")
                };
                self.write_trace_labeled(&mut inner, RecordKind::SemanticAssertion, label);
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

    fn verify_runtime_assertions(&self, inner: &mut CoordInner) -> bool {
        // Runtime semantic assertions are checked before allowing the pure
        // scheduler to advance their action.
        if let Some(Action::AssertScene(assertion)) = self.actions.get(inner.sched_state.step_index)
        {
            match scenario::verify(assertion.scene) {
                Ok(plan) => self.write_trace_labeled(
                    inner,
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
                        inner,
                        RecordKind::SemanticAssertion,
                        error.to_string(),
                    );
                    self.set_terminal(inner, TerminalClass::SemanticMismatch);
                    return true;
                }
            }
        }

        if let Some(Action::AssertDispatch(assertion)) =
            self.actions.get(inner.sched_state.step_index)
        {
            match scenario::verify_dispatch(assertion.encounter, assertion.dialogue) {
                Ok(()) => self.write_trace_labeled(
                    inner,
                    RecordKind::SemanticAssertion,
                    format!(
                        "dispatch_verified:encounter={}:dialogue={}",
                        assertion.encounter, assertion.dialogue
                    ),
                ),
                Err(error) => {
                    self.write_trace_labeled(
                        inner,
                        RecordKind::SemanticAssertion,
                        error.to_string(),
                    );
                    self.set_terminal(inner, TerminalClass::SemanticMismatch);
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
                    inner,
                    RecordKind::SemanticAssertion,
                    "game_options_active".to_string(),
                ),
                Err(error) => {
                    self.write_trace_labeled(
                        inner,
                        RecordKind::SemanticAssertion,
                        error.to_string(),
                    );
                    self.set_terminal(inner, TerminalClass::SemanticMismatch);
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
                    inner,
                    RecordKind::SemanticAssertion,
                    format!("communication_responses_active:count={actual}"),
                ),
                Err(error) => {
                    self.write_trace_labeled(inner, RecordKind::SemanticAssertion, error);
                    self.set_terminal(inner, TerminalClass::SemanticMismatch);
                    return true;
                }
            }
        }
        if let Some(Action::AssertBattleFrames(assertion)) =
            self.actions.get(inner.sched_state.step_index)
        {
            match crate::automation::battle_observer::assert_progress(assertion.minimum) {
                Ok(actual) => self.write_trace_labeled(
                    inner,
                    RecordKind::SemanticAssertion,
                    format!("battle_frames_verified:count={actual}"),
                ),
                Err(error) => {
                    self.write_trace_labeled(inner, RecordKind::SemanticAssertion, error);
                    self.set_terminal(inner, TerminalClass::SemanticMismatch);
                    return true;
                }
            }
        }

        false
    }

    fn verify_activity_assertion(&self, inner: &mut CoordInner) -> bool {
        let Some(Action::AssertActivity(assertion)) =
            self.actions.get(inner.sched_state.step_index)
        else {
            return false;
        };
        let word = crate::mainloop::ffi::get_current_activity().0;
        let evidence = activity_evidence(assertion, word);
        let passed = evidence.passed;
        let seq = inner.trace_seq;
        inner.trace_seq = inner.trace_seq.saturating_add(1);
        let record = TraceRecord {
            schema: TraceRecord::SCHEMA,
            run: 1,
            sequence: seq,
            input_seen: inner.input_seen,
            present_seen: inner.present_seen,
            elapsed_ms: self.started_at.elapsed().as_millis() as u64,
            kind: RecordKind::SemanticAssertion,
            label: Some(if passed {
                "activity_assertion_passed".into()
            } else {
                "activity_assertion_failed".into()
            }),
            from: None,
            to: None,
            terminal_reason: None,
            seed_application: None,
            presentation: None,
            activity: Some(evidence),
        };
        let reservation = self.runtime.commit.reserve_sequence(seq);
        match record.to_jsonl() {
            Ok(jsonl) => reservation.commit_record(jsonl),
            Err(_) => {
                reservation.cancel();
                self.set_terminal(inner, TerminalClass::TraceFailure);
                return true;
            }
        }
        if !passed {
            self.set_terminal(inner, TerminalClass::SemanticMismatch);
        }
        !passed
    }

    // -----------------------------------------------------------------------
    //  Present callback processing (called from present observation hook)
    // -----------------------------------------------------------------------

    /// Process a committed present callback. Returns true if the game loop
    /// should stop.
    pub fn process_present(frame: Option<crate::automation::capture::PresentedFrame>) -> bool {
        let Some(coord) = Self::get() else {
            return false;
        };
        coord.process_present_inner(frame)
    }

    fn process_present_inner(
        &self,
        frame: Option<crate::automation::capture::PresentedFrame>,
    ) -> bool {
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

        let Some(frame) = frame else {
            self.capture_failure(&mut inner, "presented-frame evidence is unavailable");
            return true;
        };
        if let Err(error) = frame.validate() {
            self.capture_failure(&mut inner, error);
            return true;
        }

        let config = SchedulerConfig {
            actions: &self.actions,
            transitions: &self.transitions,
        };
        let transition = scheduler_reduce(
            &inner.sched_state,
            &config,
            SchedulerEvent::CommittedPresent {
                generation: CaptureGeneration(frame.generation),
            },
        );
        inner.sched_state = transition.new_state;

        if self.write_presentation_trace(&mut inner, &frame) {
            return true;
        }
        self.apply_effects(&mut inner, &transition.effects, Some(&frame));

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

            let label = if inner.sched_state.terminal == Some(TerminalOutcome::SemanticMismatch) {
                format!("menu_transition_failed:to={to}")
            } else {
                format!("menu_transition_passed:to={to}")
            };
            self.write_trace_labeled(&mut inner, RecordKind::SemanticAssertion, label);

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

    /// Finalize the automation run after production subsystem teardown.
    ///
    /// In inactive mode this preserves the game result. Active mode durably
    /// publishes the ordered trace and teardown receipt before returning the
    /// terminal-aware process status. Evidence failures are fatal.
    pub fn finalize(game_result: i32) -> Result<i32, String> {
        let Some(coord) = Self::get() else {
            return Ok(game_result);
        };
        coord.finalize_inner(game_result)
    }

    fn finalize_inner(&self, game_result: i32) -> Result<i32, String> {
        let terminal = {
            let mut inner = self.inner.lock();
            if inner.finalized {
                return Err("automation coordinator finalized more than once".into());
            }
            inner.finalized = true;
            self.write_trace(&mut inner, RecordKind::RunEnd);
            inner.terminal_class
        };

        validate_runtime_finalization(self.runtime.finalize())?;
        let status = active_automation_status(terminal, game_result)?;

        let mut trace = Vec::new();
        self.runtime
            .commit
            .publish_all(&mut trace)
            .map_err(|error| format!("cannot publish ordered automation trace: {error}"))?;
        crate::automation::artifact::write_durable(&self.output_root, "trace", "jsonl", &trace)
            .map_err(|error| format!("cannot durably publish automation trace: {error}"))?;

        let teardown = crate::automation::lifecycle::TeardownReceipt {
            schema: "uqm-teardown-v1".into(),
            terminal,
            game_status: game_result,
            process_status: status,
            runtime_finalized: true,
            runtime_deactivated: !self.runtime.mirror.is_active(),
            callbacks_quiescent: !self.runtime.can_write(),
            trace_durable: true,
        };
        crate::automation::lifecycle::write_teardown_receipt(&self.output_root, &teardown)
            .map_err(|error| format!("cannot durably publish teardown receipt: {error}"))?;

        Ok(status)
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
            crate::automation::input_ffi::inject_player_key(
                i32::from(key.index()),
                i32::from(active),
            );
        }
    }

    /// Apply planned effects from the scheduler reducer.
    fn apply_effects(
        &self,
        inner: &mut CoordInner,
        effects: &EffectPlan,
        frame: Option<&crate::automation::capture::PresentedFrame>,
    ) {
        // Note: `inner` is `&mut` so callers can pass it as mutable.
        if let Some((key, value)) = effects.write_key {
            let index = crate::automation::input::menu_key_to_index(key);
            crate::automation::input_ffi::inject_menu_key(i32::from(index), i32::from(value));
        }
        if let Some(key) = effects.release_key {
            let index = crate::automation::input::menu_key_to_index(key);
            crate::automation::input_ffi::inject_menu_key(i32::from(index), 0);
        }
        if let Some((key, value)) = effects.write_player_key {
            crate::automation::input_ffi::inject_player_key(
                i32::from(key.index()),
                i32::from(value),
            );
        }
        if let Some(key) = effects.release_player_key {
            crate::automation::input_ffi::inject_player_key(i32::from(key.index()), 0);
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
            let label = inner
                .armed_capture_label
                .take()
                .unwrap_or_else(|| format!("capture_gen{}", gen.0));
            match frame {
                Some(frame) => self.capture_presented_frame(inner, &label, gen, frame),
                None => self.capture_failure(inner, "capture completed without a presented frame"),
            }
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
            seed_application: None,
            presentation: None,
            activity: None,
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
            seed_application: None,
            presentation: None,
            activity: None,
        };

        if let Ok(jsonl) = record.to_jsonl() {
            let res = self.runtime.commit.reserve_sequence(seq);
            res.commit_record(jsonl);
        }
    }

    fn write_presentation_trace(
        &self,
        inner: &mut CoordInner,
        frame: &crate::automation::capture::PresentedFrame,
    ) -> bool {
        let seq = inner.trace_seq;
        inner.trace_seq = inner.trace_seq.saturating_add(1);
        let record = TraceRecord {
            schema: TraceRecord::SCHEMA,
            run: 1,
            sequence: seq,
            input_seen: inner.input_seen,
            present_seen: inner.present_seen,
            elapsed_ms: self.started_at.elapsed().as_millis() as u64,
            kind: RecordKind::Presentation,
            label: None,
            from: None,
            to: None,
            terminal_reason: None,
            seed_application: None,
            presentation: Some(frame.presentation_evidence()),
            activity: None,
        };
        let reservation = self.runtime.commit.reserve_sequence(seq);
        match record.to_jsonl() {
            Ok(jsonl) => {
                reservation.commit_record(jsonl);
                false
            }
            Err(error) => {
                reservation.cancel();
                self.capture_failure(inner, format!("cannot serialize presentation: {error}"));
                true
            }
        }
    }

    fn capture_presented_frame(
        &self,
        inner: &mut CoordInner,
        label: &str,
        generation: CaptureGeneration,
        frame: &crate::automation::capture::PresentedFrame,
    ) {
        if frame.generation != generation.0 {
            return self.capture_failure(inner, "presented-frame generation mismatch");
        }
        let png_data = match frame.encode_png() {
            Ok(png) => png,
            Err(error) => return self.capture_failure(inner, error),
        };
        let capture_dir = self.output_root.join("captures");
        if let Err(error) =
            crate::automation::artifact::write_durable(&capture_dir, label, "png", &png_data)
        {
            return self.capture_failure(inner, format!("durable PNG publication failed: {error}"));
        }

        let mut record = crate::automation::capture::capture_trace_record(
            inner.trace_seq,
            self.started_at.elapsed().as_millis() as u64,
            generation,
            label,
        );
        record.run = 1;
        record.input_seen = inner.input_seen;
        record.present_seen = inner.present_seen;
        record.presentation = Some(frame.presentation_evidence());
        inner.trace_seq = inner.trace_seq.saturating_add(1);
        let reservation = self.runtime.commit.reserve_sequence(record.sequence);
        match record.to_jsonl() {
            Ok(jsonl) => reservation.commit_record(jsonl),
            Err(error) => {
                reservation.cancel();
                self.capture_failure(
                    inner,
                    format!("cannot serialize capture trace record: {error}"),
                );
            }
        }
    }

    fn capture_failure(&self, inner: &mut CoordInner, error: impl std::fmt::Display) {
        eprintln!("[automation] capture failed: {error}");
        self.set_terminal(inner, TerminalClass::TraceFailure);
    }
}

fn activity_evidence(assertion: &ActivityAssertion, word: u16) -> ActivityEvidence {
    ActivityEvidence {
        word,
        mask: assertion.mask,
        equals: assertion.equals,
        passed: word & assertion.mask == assertion.equals,
    }
}

/// Return the deterministic seed for an automation-owned RNG boundary, while
/// preserving the caller's wall-clock seed when automation is inactive.
#[no_mangle]
pub extern "C" fn rust_automation_seed_value(domain: u32, fallback: u32) -> u32 {
    let Some(domain) = SeedDomain::from_ffi(domain) else {
        return fallback;
    };
    let Some(coord) = Coordinator::get() else {
        return fallback;
    };
    let mut inner = coord.inner.lock();
    let seq = inner.trace_seq;
    inner.trace_seq = inner.trace_seq.saturating_add(1);
    let record = TraceRecord {
        schema: TraceRecord::SCHEMA,
        run: 1,
        sequence: seq,
        input_seen: inner.input_seen,
        present_seen: inner.present_seen,
        elapsed_ms: coord.started_at.elapsed().as_millis() as u64,
        kind: RecordKind::SeedApplication,
        label: None,
        from: None,
        to: None,
        terminal_reason: None,
        seed_application: Some(SeedApplication {
            domain,
            seed: AUTOMATION_SEED,
        }),
        presentation: None,
        activity: None,
    };
    let reservation = coord.runtime.commit.reserve_sequence(seq);
    match record.to_jsonl() {
        Ok(jsonl) => reservation.commit_record(jsonl),
        Err(_) => {
            reservation.cancel();
            coord.set_terminal(&mut inner, TerminalClass::TraceFailure);
        }
    }
    AUTOMATION_SEED
}

fn validate_runtime_finalization(result: FinalizationResult) -> Result<(), String> {
    match result {
        FinalizationResult::Finalized => Ok(()),
        other => Err(format!(
            "automation runtime finalization did not complete: {other:?}"
        )),
    }
}

fn active_automation_status(
    terminal: Option<TerminalClass>,
    game_result: i32,
) -> Result<i32, String> {
    terminal
        .map(|class| crate::automation::lifecycle::map_status(Some(class), game_result))
        .ok_or_else(|| "automation run ended without a terminal outcome".into())
}

fn consume_new_generation(consumed: &mut u64, observed: u64) -> bool {
    if observed <= *consumed {
        return false;
    }
    *consumed = observed;
    true
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
    fn replay_generation_is_consumed_once_and_retains_early_events() {
        let mut consumed = 0;
        assert!(consume_new_generation(&mut consumed, 1));
        assert_eq!(consumed, 1);
        assert!(!consume_new_generation(&mut consumed, 1));
        assert!(consume_new_generation(&mut consumed, 2));
        assert_eq!(consumed, 2);
    }

    #[test]
    fn coordinator_not_active_by_default() {
        assert!(!Coordinator::is_active());
    }

    #[test]
    fn activity_evidence_uses_the_complete_activity_word() {
        let assertion = ActivityAssertion {
            mask: 0x02ff,
            equals: 0x0200,
        };
        let passed = activity_evidence(&assertion, 0x1200);
        assert!(passed.passed);
        assert_eq!(passed.word, 0x1200);

        let failed = activity_evidence(&assertion, 0x0000);
        assert!(!failed.passed);
        assert_eq!(failed.mask, 0x02ff);
        assert_eq!(failed.equals, 0x0200);
    }

    #[test]
    fn capture_trace_is_bound_to_the_active_run_and_callback_counts() {
        let generation = CaptureGeneration(3);
        let mut record =
            crate::automation::capture::capture_trace_record(11, 25, generation, "capture");
        record.run = 1;
        record.input_seen = 7;
        record.present_seen = 9;

        assert_eq!(record.run, 1);
        assert_eq!(record.sequence, 11);
        assert_eq!(record.input_seen, 7);
        assert_eq!(record.present_seen, 9);
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

    #[test]
    fn active_status_requires_a_terminal_outcome() {
        assert!(active_automation_status(None, 0).is_err());
    }

    #[test]
    fn active_status_preserves_game_failure_after_success() {
        assert_eq!(
            active_automation_status(Some(TerminalClass::Success), 7),
            Ok(7)
        );
    }

    #[test]
    fn active_status_maps_automation_failure_nonzero() {
        assert_eq!(
            active_automation_status(Some(TerminalClass::SemanticMismatch), 0),
            Ok(1)
        );
    }

    #[test]
    fn runtime_finalization_rejects_every_nonfinalized_result() {
        assert!(validate_runtime_finalization(FinalizationResult::Finalized).is_ok());
        for result in [
            FinalizationResult::AlreadyFinalized,
            FinalizationResult::AlreadyFinalizing,
            FinalizationResult::ShellsStillActive(1),
            FinalizationResult::DuplicateRunEnd,
        ] {
            assert!(validate_runtime_finalization(result).is_err());
        }
    }
}
