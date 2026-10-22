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

use crate::automation::battle_fight::{
    BattleFightTracker, BattleSnapshot, FightControl, FightProgress,
};
use crate::automation::battle_outcome::{self, BattleLossTracker};
use crate::automation::campaign_state;
use crate::automation::flagship_turn::{self, FlagshipTurnTracker, TurnProgress};
use crate::automation::flight_position::{self, FlightPositionTracker};
use crate::automation::input_ffi;
use crate::automation::mode;
use crate::automation::outcome::TerminalClass;
use crate::automation::planet_minerals::{self, CollectProgress, PlanetMineralsTracker};
use crate::automation::runtime::{FinalizationResult, RuntimeModel};
use crate::automation::scenario::{self, PendingStartScene, SceneActivationBoundary};
use crate::automation::scheduler::{
    scheduler_reduce, ActionPhase, CaptureGeneration, EffectPlan, SchedulerConfig, SchedulerEvent,
    SchedulerState, TerminalOutcome,
};
use crate::automation::script::{Action, ActivityAssertion, ValidatedScript};
use crate::automation::ship_element;
use crate::automation::ship_resources;
use crate::automation::trace::{
    ActivityEvidence, ReadinessEvidence, RecordKind, SeedApplication, SeedDomain, TraceRecord,
};
use crate::automation::watchdog::{
    watchdog_reduce, CallbackKind, ClockSample, WatchdogEntry, WatchdogLimits, WatchdogOutcome,
};
use parking_lot::Mutex;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

#[cfg(feature = "linked_c_archive")]
use crate::c_bindings::controller_input::{
    CurrentInputState, PulsedInputState, CONTROL_TEMPLATE_NUM_TEMPLATES, NUM_KEYS,
};
#[cfg(feature = "linked_c_archive")]
use crate::c_bindings::player_control_template;

#[cfg(feature = "linked_c_archive")]
const NUM_PLAYER_KEYS: usize = NUM_KEYS as usize;
#[cfg(feature = "linked_c_archive")]
const NUM_INPUT_TEMPLATES: usize = CONTROL_TEMPLATE_NUM_TEMPLATES as usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PendingPlayerInput {
    index: i32,
    value: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlayerInputObservation {
    Matched { current: i32, pulsed: i32 },
    Missing,
    Contradictory { current: i32, pulsed: i32 },
}

fn inject_menu_key(index: i32, value: i32) -> bool {
    input_ffi::inject_menu_key(index, value);
    !input_ffi::injection_rejected()
}

/// Discard the repeat pulse of a menu key that a scripted tap only holds.
///
/// Returns `false` when the native owner has no such key.
fn discard_held_repeat(
    held: Option<crate::automation::script::MenuKey>,
    discard: impl FnOnce(i32) -> bool,
) -> bool {
    held.is_none_or(|key| discard(i32::from(crate::automation::input::menu_key_to_index(key))))
}

fn prepare_player_input(index: i32, value: i32) -> Option<PendingPlayerInput> {
    input_ffi::inject_player_key(index, value);
    (!input_ffi::injection_rejected()).then_some(PendingPlayerInput {
        index,
        value: i32::from(value != 0),
    })
}

fn inject_player_key(index: i32, value: i32) -> bool {
    input_ffi::inject_player_key(index, value);
    !input_ffi::injection_rejected()
}

#[cfg(feature = "linked_c_archive")]
fn read_player_input_state(index: i32) -> Option<(i32, i32)> {
    let index = usize::try_from(index).ok()?;
    if index >= NUM_PLAYER_KEYS {
        return None;
    }
    unsafe {
        let template = usize::try_from(player_control_template(0)?).ok()?;
        if template >= NUM_INPUT_TEMPLATES {
            return None;
        }
        Some((
            CurrentInputState.key[template][index],
            PulsedInputState.key[template][index],
        ))
    }
}

#[cfg(not(feature = "linked_c_archive"))]
fn read_player_input_state(_index: i32) -> Option<(i32, i32)> {
    None
}

fn observe_player_input_with(
    pending: PendingPlayerInput,
    observer: impl FnOnce(i32) -> Option<(i32, i32)>,
) -> PlayerInputObservation {
    let Some((current, pulsed)) = observer(pending.index) else {
        return PlayerInputObservation::Missing;
    };
    let matches_write = match pending.value {
        0 => current == 0 && pulsed == 0,
        1 => current == 1 && matches!(pulsed, 0 | 1),
        _ => false,
    };
    if matches_write {
        PlayerInputObservation::Matched { current, pulsed }
    } else {
        PlayerInputObservation::Contradictory { current, pulsed }
    }
}

fn observe_player_input(pending: PendingPlayerInput) -> PlayerInputObservation {
    observe_player_input_with(pending, read_player_input_state)
}

/// Outcome of servicing a planet-side wait for one input callback.
#[derive(Debug, Default, PartialEq, Eq)]
struct PlanetSideWaitResult {
    /// Whether the wait's condition is now satisfied.
    reached: bool,
    /// Trace labels to record for this callback.
    labels: Vec<String>,
}

impl PlanetSideWaitResult {
    fn pending() -> Self {
        Self::default()
    }

    fn reached(label: String) -> Self {
        Self {
            reached: true,
            labels: vec![label],
        }
    }
}

/// A scripted player input awaiting production-state observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PendingAcceptedPlayerInput {
    key: crate::automation::script::PlayerKey,
    injection: PendingPlayerInput,
}

/// Whether a settled trip satisfies a wait bound to `awaited`.
///
/// The wait must be bound to a started trip, that exact trip must be the one
/// being observed, it must have settled, and its terminal code must match.
fn planet_side_wait_satisfied(
    awaited: Option<u64>,
    observation: &crate::planet_side::telemetry::PlanetSideObservation,
    outcome: crate::automation::script::PlanetSideOutcomeName,
) -> bool {
    awaited == Some(observation.generation)
        && !observation.active
        && observation.terminal == outcome.terminal_code()
}

/// Describe the trip state a `wait_for_planet_side_end` last observed before
/// it exhausted its ticks.
fn planet_side_wait_mismatch_label(
    awaited: Option<u64>,
    observation: &crate::planet_side::telemetry::PlanetSideObservation,
    outcome: crate::automation::script::PlanetSideOutcomeName,
) -> String {
    format!(
        "planet_side_wait_unsatisfied:awaited={awaited:?}:expected_terminal={}:\
         generation={}:active={}:terminal={}:phase={}:crew={}",
        outcome.terminal_code(),
        observation.generation,
        observation.active,
        observation.terminal,
        crate::planet_side::telemetry::phase_name(observation.phase),
        observation.current_crew
    )
}

/// Rotating phase for the orbital-exit key sequence.
static ORBIT_EXIT_PHASE: AtomicU64 = AtomicU64::new(0);

/// Callbacks in one repetition of the orbital-exit key sequence.
const ORBIT_EXIT_PERIOD: u64 = 6;

/// Menu keys `(cancel, up, select)` that leave orbit, one admitted callback
/// at a time.
///
/// Orbit may be left from two menus. `DoScan` (scan.c) is open right after a
/// lander returns: Cancel exits it back to `DoPlanetOrbit`, whereas Up and
/// Select there would move within the scan items and scan or dispatch again.
/// Cancel is ignored by the `DoPlanetOrbit` menu itself (`DoMenuChooser`
/// only reacts to it in the alternate cargo menu), so it is safe to press
/// first from either state. `PlanetOrbitMenu` always opens on the first item
/// (SCAN), and NAVIGATION is the last, so one Up wraps the cursor straight
/// onto it and Select confirms. `DoMenuChooser` honours Up in both the PC and
/// 3DO menu layouts. Every key is pulsed, so each press needs an intervening
/// release to register as a new press.
fn orbit_exit_menu_keys(phase: u64) -> (bool, bool, bool) {
    match phase % ORBIT_EXIT_PERIOD {
        0 => (true, false, false),
        2 => (false, true, false),
        4 => (false, false, true),
        _ => (false, false, false),
    }
}

/// Default RNG seed for scripts that predate explicit seed selection.
pub const AUTOMATION_SEED: u32 = 0x55AA_2317;

/// What the main-menu readiness record reports as having become ready.
pub const MAIN_MENU_READINESS_SUBJECT: &str = "main_menu";

/// The trace label that proves a scripted quit: the game set `CHECK_ABORT`
/// from the restart menu's Quit entry and the script declared `expect_quit`.
pub const QUIT_VERIFIED_LABEL: &str = "quit_verified:activity=0x4000";

/// The production observation that proves main-menu readiness: the restart
/// menu finished initialization and accepts input.
pub const MAIN_MENU_READINESS_OBSERVATION: &str = "restart_menu_first_frame_initialized";

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
    /// Whether readiness has been observed, which starts the idle watchdog and
    /// stops the startup one.
    ready: bool,
    /// When progress was last admitted, for the idle watchdog.
    last_progress: Instant,
    sched_state: SchedulerState,
    checkpoint_step: usize,
    input_seen: u64,
    present_seen: u64,
    last_observed: Instant,
    accepted_player_inputs: u64,
    pending_player_input: Option<PendingAcceptedPlayerInput>,
    /// Menu key a scripted tap held down this update after its press.
    held_repeat_key: Option<crate::automation::script::MenuKey>,
    verified_battle_frames: u64,
    /// Hyperspace position snapshot awaiting its displacement assertion.
    flight_position: FlightPositionTracker,
    /// Fleet sizes at the start of the battle a ship-loss wait is watching.
    battle_loss: BattleLossTracker,
    /// Facing at which the current `turn_flagship` step began.
    flagship_turn: FlagshipTurnTracker,
    /// Ticks and trip of the current `collect_planet_minerals` step.
    planet_minerals: PlanetMineralsTracker,
    /// Ticks and starting ship counts of the current `fight_battle` step.
    battle_fight: BattleFightTracker,
    finalized: bool,
    terminal_class: Option<TerminalClass>,
    /// Queued menu transition events that arrived while the scheduler
    /// was not in WaitingSemantic. Replayed when the scheduler enters
    /// WaitingSemantic.
    pending_transitions: Vec<(u8, u8)>,
    /// The restart menu reported that Quit was selected (activity set to
    /// `CHECK_ABORT`). The game exits right after, before another input
    /// callback, so finalization settles whether the script expected it.
    quit_selected: bool,
    pending_main_menu_ready: bool,
    /// The label of the currently armed capture step, if any.
    /// Used when the capture completes to write a PNG artifact.
    armed_capture_label: Option<String>,
    /// Declarative start scene, consumed once at the game-initialized boundary.
    pending_start_scene: PendingStartScene,
    /// Most recent communication completion consumed by a completion wait.
    consumed_communication_completions: u64,
    /// Most recent dispatch generation consumed by a dispatch wait.
    consumed_dispatch_generation: u64,
    /// Last observed planet-side (generation, phase), so transitions are traced
    /// once each without swallowing a repeated phase in a later trip.
    observed_planet_side_phase: Option<(u64, u32)>,
    /// Generation latched by `wait_for_planet_side_start`, which
    /// `wait_for_planet_side_end` must match so a settled earlier trip cannot
    /// satisfy a later wait.
    awaited_planet_side_generation: Option<u64>,
    /// Most recent communication response generation selected semantically.
    consumed_response_generation: u64,
    /// Most recent communication replay generation consumed semantically.
    consumed_replay_generation: u64,
    /// Most recent planet-menu generation selected semantically.
    consumed_planet_menu_generation: u64,
    /// Most recent battle ship picker generation confirmed semantically.
    consumed_ship_picker_generation: u64,
    /// Require one callback after orbit navigation before accepting menu ownership.
    orbit_transition_pending: bool,
    /// Whether the current `navigate_to_hyperspace` step has sampled live
    /// interplanetary flight, which arrival requires.
    hyperspace_flight_observed: bool,
    /// Release a semantically injected Select key on the next callback.
    release_semantic_select: bool,
}

/// The automation coordinator, holding all live state needed to drive
/// the scheduler/watchdog during the game loop.
pub struct Coordinator {
    /// Validated seed returned at each automation-owned RNG initialization.
    seed: u32,
    /// The validated script actions.
    actions: Vec<Action>,
    native_checkpoint_identity: Option<String>,
    native_trace: Option<Mutex<super::native_trace::NativeTraceJournal>>,
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
        #[cfg(feature = "debug-process")]
        let native_limit = super::native_window::active_native_window_config()
            .map(|config| config.runtime_contract.inventory_limits.member_bytes);
        #[cfg(not(feature = "debug-process"))]
        let native_limit = None;
        Self::init_with_native_checkpoints(script, output_root, native_limit);
    }

    fn init_with_native_checkpoints(
        script: ValidatedScript,
        output_root: PathBuf,
        native_limit: Option<u64>,
    ) {
        let native_trace = native_limit
            .map(|limit| {
                super::native_trace::NativeTraceJournal::create(&output_root, limit).map(Mutex::new)
            })
            .transpose();
        let checkpoint_plan = native_limit
            .is_some()
            .then(|| super::native_checkpoint::NativeCheckpointPlan::derive(&[], &script))
            .transpose();
        let native_checkpoint_identity = checkpoint_plan
            .as_ref()
            .ok()
            .and_then(|plan| plan.as_ref())
            .map(|plan| plan.resolved_sha256.clone());
        // Missing input identity must stop automation before any step executes.
        let resolved_result =
            crate::automation::lifecycle::write_resolved_scenario(&output_root, &script.resolved())
                .map_err(|error| error.to_string());
        if let Err(error) = &resolved_result {
            eprintln!("automation: cannot record the resolved scenario: {error}");
        }

        let budgets = script.budgets();
        let start_scene = script.start_scene();
        let actions = script.steps().to_vec();
        let transitions = script.transitions().to_vec();

        let watchdog_limits = WatchdogLimits {
            max_input_ticks: budgets.max_input_ticks,
            max_presentations: budgets.max_presentations,
            max_wallclock: Duration::from_secs(budgets.max_wallclock_seconds),
            max_startup: Duration::from_secs(budgets.startup_seconds()),
            max_idle: Duration::from_secs(budgets.idle_seconds()),
        };

        let now = Instant::now();

        // Initialize the runtime model via input_ffi.
        input_ffi::init_automation_runtime();

        let runtime = input_ffi::get_runtime().expect("runtime initialized");

        // Activate the runtime.
        runtime.activate();

        let (_, consumed_communication_completions) =
            crate::automation::ui_observation::communication_lifecycle();

        let native_trace_failed = native_trace.is_err();
        if let Err(error) = &native_trace {
            eprintln!("automation: cannot create native trace journal: {error}");
        }
        let coord = Coordinator {
            seed: script.seed(),
            actions,
            native_checkpoint_identity,
            native_trace: native_trace.ok().flatten(),
            transitions,
            watchdog_limits,
            started_at: now,
            output_root,
            runtime,
            inner: Mutex::new(CoordInner {
                sched_state: SchedulerState::initial(),
                checkpoint_step: 0,
                ready: false,
                last_progress: now,
                input_seen: 0,
                present_seen: 0,
                last_observed: now,
                accepted_player_inputs: 0,
                pending_player_input: None,
                held_repeat_key: None,
                verified_battle_frames: 0,
                flight_position: FlightPositionTracker::default(),
                battle_loss: BattleLossTracker::default(),
                flagship_turn: FlagshipTurnTracker::default(),
                planet_minerals: PlanetMineralsTracker::default(),
                battle_fight: BattleFightTracker::default(),
                finalized: false,
                terminal_class: None,
                pending_transitions: Vec::new(),
                quit_selected: false,
                pending_main_menu_ready: false,
                armed_capture_label: None,
                pending_start_scene: PendingStartScene::new(start_scene),
                consumed_communication_completions,
                consumed_dispatch_generation: 0,
                observed_planet_side_phase: None,
                awaited_planet_side_generation: None,
                consumed_response_generation: 0,
                consumed_replay_generation: 0,
                consumed_planet_menu_generation: 0,
                consumed_ship_picker_generation: 0,
                release_semantic_select: false,
                orbit_transition_pending: false,
                hyperspace_flight_observed: false,
            }),
        };

        // Write run_start trace.
        {
            let mut init_inner = coord.inner.lock();
            coord.write_trace(&mut init_inner, RecordKind::RunStart);
            if resolved_result.is_err() || checkpoint_plan.is_err() || native_trace_failed {
                coord.set_terminal(&mut init_inner, TerminalClass::TraceFailure);
            }
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
    /// Return one atomic semantic snapshot for the backend publication.
    ///
    /// The backend calls this after presenting pixels but before the corresponding coordinator
    /// presentation callback. The ordered commit's next sequence is the length of the exact
    /// reserved trace prefix; both semantic counters are protected by the coordinator lock.
    pub fn native_window_semantic_snapshot() -> Option<(u64, u64, u64)> {
        let coord = Self::get()?;
        let inner = coord.inner.lock();
        Some((
            coord.runtime.commit.next_sequence(),
            inner.accepted_player_inputs,
            inner.verified_battle_frames,
        ))
    }

    /// Terminate active automation when an external evidence publisher fails.
    pub fn external_trace_failure(detail: impl Into<String>) {
        let Some(coord) = Self::get() else {
            return;
        };
        let mut inner = coord.inner.lock();
        coord.write_trace_labeled(&mut inner, RecordKind::SemanticAssertion, detail.into());
        coord.set_terminal(&mut inner, TerminalClass::TraceFailure);
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

    /// Observe a scripted player input after production updates current and pulsed state.
    pub fn process_player_input_observation() -> bool {
        let Some(coord) = Self::get() else {
            return false;
        };
        coord.process_player_input_observation_inner()
    }

    fn process_player_input_observation_inner(&self) -> bool {
        let mut inner = self.inner.lock();
        let held = inner.held_repeat_key.take();
        if inner.terminal_class.is_some() {
            return true;
        }
        if !discard_held_repeat(held, input_ffi::discard_menu_pulse) {
            self.write_trace_labeled(
                &mut inner,
                RecordKind::SemanticAssertion,
                "held_menu_repeat_discard_rejected".to_string(),
            );
            self.set_terminal(&mut inner, TerminalClass::SemanticMismatch);
            return true;
        }
        let Some(pending) = inner.pending_player_input.take() else {
            return false;
        };

        match observe_player_input(pending.injection) {
            PlayerInputObservation::Matched { current, pulsed } => {
                self.write_trace_labeled(
                    &mut inner,
                    RecordKind::SemanticAssertion,
                    player_input_observation_trace_label(
                        pending.key,
                        pending.injection.value,
                        current,
                        pulsed,
                    ),
                );
                let Some(next) = accepted_player_input_count(inner.accepted_player_inputs, pulsed)
                else {
                    self.write_trace_labeled(
                        &mut inner,
                        RecordKind::SemanticAssertion,
                        "accepted_player_inputs_overflow".to_string(),
                    );
                    self.set_terminal(&mut inner, TerminalClass::CounterOverflow);
                    return true;
                };
                inner.accepted_player_inputs = next;
                self.write_trace_labeled(
                    &mut inner,
                    RecordKind::SemanticAssertion,
                    player_input_trace_label(pending.key, pending.injection.value),
                );
                if inner.sched_state.is_terminal() {
                    let class = map_scheduler_terminal(inner.sched_state.terminal);
                    self.set_terminal(&mut inner, class);
                    return true;
                }
                false
            }
            PlayerInputObservation::Missing => {
                self.reject_player_input_observation(&mut inner, pending, None);
                true
            }
            PlayerInputObservation::Contradictory { current, pulsed } => {
                self.reject_player_input_observation(&mut inner, pending, Some((current, pulsed)));
                true
            }
        }
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

    /// Service whichever semantic wait the current step is blocked on.
    ///
    /// Each wait observes real production state and only then reports that its
    /// condition was reached; injected input never satisfies a wait.
    fn service_semantic_waits(
        &self,
        inner: &mut CoordInner,
        released_semantic_select: bool,
        orbit_transition_pending: bool,
    ) -> SchedulerEvent {
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
                    inner,
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
                    inner,
                    RecordKind::SemanticAssertion,
                    format!("communication_replay_active:generation={generation}"),
                );
            }
        } else if let Some(Action::WaitForCommunicationResponses(wait)) =
            self.actions.get(inner.sched_state.step_index)
        {
            let snapshot = crate::automation::ui_observation::conversation_snapshot();
            if snapshot.has_fresh_responses(
                wait.minimum,
                inner.consumed_communication_completions,
                inner.consumed_response_generation,
            ) {
                scheduler_event = SchedulerEvent::ConditionReached;
                self.write_trace_labeled(
                    inner,
                    RecordKind::SemanticAssertion,
                    format!(
                        "communication_responses_reached:generation={}:count={}:minimum={}",
                        snapshot.response_generation, snapshot.response_count, wait.minimum
                    ),
                );
            } else if inner.sched_state.phase == (ActionPhase::WaitingCondition { remaining: 0 }) {
                // The scheduler ends the proof on this tick; say what the
                // production state was instead of leaving a bare mismatch.
                self.write_trace_labeled(
                    inner,
                    RecordKind::SemanticAssertion,
                    format!(
                        "communication_responses_wait_expired:minimum={}:active={}:completions={}:consumed_completions={}:response_generation={}:consumed_generation={}:count={}",
                        wait.minimum,
                        snapshot.active,
                        snapshot.completions,
                        inner.consumed_communication_completions,
                        snapshot.response_generation,
                        inner.consumed_response_generation,
                        snapshot.response_count
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
                    let key_index = i32::from(crate::automation::script::MenuKey::Select.index());
                    if !inject_menu_key(key_index, 1) {
                        self.reject_input(inner, "menu", key_index, 1);
                        return scheduler_event;
                    }
                    inner.release_semantic_select = true;
                    inner.consumed_response_generation = generation;
                    scheduler_event = SchedulerEvent::ConditionReached;
                    self.write_trace_labeled(
                        inner,
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
                let key_index = i32::from(crate::automation::script::MenuKey::Select.index());
                if !inject_menu_key(key_index, 1) {
                    self.reject_input(inner, "menu", key_index, 1);
                    return scheduler_event;
                }
                inner.release_semantic_select = true;
                inner.consumed_planet_menu_generation = generation;
                scheduler_event = SchedulerEvent::ConditionReached;
                self.write_trace_labeled(
                    inner,
                    RecordKind::SemanticAssertion,
                    format!(
                        "planet_menu_selected:generation={generation}:phase={:?}",
                        select.phase
                    ),
                );
            }
        } else if matches!(
            self.actions.get(inner.sched_state.step_index),
            Some(Action::PickBattleShip(_))
        ) {
            let (generation, active) = crate::automation::ui_observation::ship_picker_observation();
            if !released_semantic_select
                && active
                && generation > inner.consumed_ship_picker_generation
            {
                let key_index = i32::from(crate::automation::script::MenuKey::Select.index());
                if !inject_menu_key(key_index, 1) {
                    self.reject_input(inner, "menu", key_index, 1);
                    return scheduler_event;
                }
                inner.release_semantic_select = true;
                inner.consumed_ship_picker_generation = generation;
                scheduler_event = SchedulerEvent::ConditionReached;
                self.write_trace_labeled(
                    inner,
                    RecordKind::SemanticAssertion,
                    format!("battle_ship_picked:generation={generation}"),
                );
            }
        } else if let Some(result) = self.service_planet_side_wait(inner) {
            scheduler_event = self.record_planet_side_result(inner, result, scheduler_event);
        } else if let Some(Action::WaitForBattleFrames(wait)) =
            self.actions.get(inner.sched_state.step_index)
        {
            let actual = crate::automation::battle_observer::current_frame();
            if battle_frame_floor_reached(actual, wait.minimum) {
                inner.verified_battle_frames = inner.verified_battle_frames.max(actual);
                scheduler_event = SchedulerEvent::ConditionReached;
                self.write_trace_labeled(
                    inner,
                    RecordKind::SemanticAssertion,
                    format!(
                        "battle_frames_reached:count={actual}:minimum={}",
                        wait.minimum
                    ),
                );
            }
        } else if let Some(Action::WaitForBattleShipLoss(wait)) =
            self.actions.get(inner.sched_state.step_index)
        {
            match inner.battle_loss.observe(
                mode::require_live(),
                crate::automation::battle_observer::current_frame(),
                battle_outcome::live_fleets(),
                wait.minimum_ships,
            ) {
                Ok(Some(proof)) => {
                    scheduler_event = SchedulerEvent::ConditionReached;
                    self.write_trace_labeled(
                        inner,
                        RecordKind::SemanticAssertion,
                        proof.trace_label(),
                    );
                }
                Ok(None) => {}
                Err(error) => {
                    self.write_trace_labeled(
                        inner,
                        RecordKind::SemanticAssertion,
                        error.trace_label(),
                    );
                    self.set_terminal(inner, TerminalClass::SemanticMismatch);
                }
            }
        } else if let Some(Action::WaitForState(wait)) =
            self.actions.get(inner.sched_state.step_index)
        {
            let expired =
                inner.sched_state.phase == (ActionPhase::WaitingCondition { remaining: 0 });
            let evaluation =
                campaign_state::evaluate(&wait.predicate, &campaign_state::LiveCampaign);
            if expired {
                // The scheduler ends the proof on this tick; say what the
                // game state was instead of leaving a bare mismatch.
                self.write_trace_labeled(
                    inner,
                    RecordKind::SemanticAssertion,
                    evaluation.failure_label("state_wait_expired", &wait.predicate),
                );
            } else if evaluation.held {
                scheduler_event = SchedulerEvent::ConditionReached;
                self.write_trace_labeled(
                    inner,
                    RecordKind::SemanticAssertion,
                    evaluation.label("state_reached"),
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
                    inner,
                    RecordKind::SemanticAssertion,
                    format!(
                        "dispatch_observed:generation={generation}:encounter={}:dialogue={}",
                        wait.encounter, wait.dialogue
                    ),
                );
            }
        }
        scheduler_event
    }

    /// Fly the flagship out of the live solar system with player controls and
    /// report arrival once production has dispatched hyperspace.
    /// Hold the turn key of a `turn_flagship` step until the live flagship
    /// facing has changed by exactly the requested facings, then release it.
    fn service_flagship_turn(
        &self,
        inner: &mut CoordInner,
        turn: crate::automation::script::TurnFlagshipStep,
        scheduler_event: SchedulerEvent,
    ) -> SchedulerEvent {
        use crate::automation::navigation::NavigationControl;
        use crate::automation::script::TurnDirection;

        let progress = inner.flagship_turn.observe(
            mode::require_live(),
            flagship_turn::live_facing(),
            turn.direction,
            turn.facings,
        );
        let holding = NavigationControl {
            left: turn.direction == TurnDirection::Left,
            right: turn.direction == TurnDirection::Right,
            ..NavigationControl::default()
        };
        match progress {
            Ok(TurnProgress::Turning { .. }) => {
                if !Self::set_navigation_controls(holding) {
                    self.reject_input(inner, "turn_flagship", -1, -1);
                }
                scheduler_event
            }
            Ok(TurnProgress::Complete(proof)) => {
                if !Self::set_navigation_controls(NavigationControl::default()) {
                    self.reject_input(inner, "turn_flagship", -1, -1);
                    return scheduler_event;
                }
                self.write_trace_labeled(inner, RecordKind::SemanticAssertion, proof.trace_label());
                SchedulerEvent::NavigationReached
            }
            Err(error) => {
                self.write_trace_labeled(inner, RecordKind::SemanticAssertion, error.trace_label());
                self.set_terminal(inner, TerminalClass::SemanticMismatch);
                scheduler_event
            }
        }
    }

    /// Drive the planet-side lander toward the nearest deposit of the
    /// requested category until its hold carries the requested units.
    fn service_planet_minerals(
        &self,
        inner: &mut CoordInner,
        collect: crate::automation::script::CollectPlanetMineralsStep,
        scheduler_event: SchedulerEvent,
    ) -> SchedulerEvent {
        use crate::automation::navigation::NavigationControl;

        let progress = inner
            .planet_minerals
            .observe(&planet_minerals::PlanetSnapshot::live(), &collect);
        match progress {
            Ok(CollectProgress::Steering(control)) => {
                if !Self::set_navigation_controls(control) {
                    self.reject_input(inner, "collect_planet_minerals", -1, -1);
                }
                scheduler_event
            }
            Ok(CollectProgress::Complete(proof)) => {
                if !Self::set_navigation_controls(NavigationControl::default()) {
                    self.reject_input(inner, "collect_planet_minerals", -1, -1);
                    return scheduler_event;
                }
                self.write_trace_labeled(inner, RecordKind::SemanticAssertion, proof.trace_label());
                SchedulerEvent::NavigationReached
            }
            Err(error) => {
                if !Self::set_navigation_controls(NavigationControl::default()) {
                    self.reject_input(inner, "collect_planet_minerals", -1, -1);
                    return scheduler_event;
                }
                self.write_trace_labeled(
                    inner,
                    RecordKind::SemanticAssertion,
                    error.trace_label(&collect),
                );
                self.set_terminal(inner, TerminalClass::SemanticMismatch);
                scheduler_event
            }
        }
    }

    /// Aim the flagship at the opposing ship and fire until the battle ends
    /// with the opposing side a ship down, holding only the keys a player
    /// would press.
    fn service_battle_fight(
        &self,
        inner: &mut CoordInner,
        fight: crate::automation::script::FightBattleStep,
        scheduler_event: SchedulerEvent,
    ) -> SchedulerEvent {
        let progress = inner.battle_fight.observe(&BattleSnapshot::live(), &fight);
        match progress {
            Ok(FightProgress::Fighting { control, report }) => {
                if !Self::set_fight_controls(control) {
                    self.reject_input(inner, "fight_battle", -1, -1);
                }
                if let Some(report) = report {
                    self.write_trace_labeled(
                        inner,
                        RecordKind::SemanticAssertion,
                        report.trace_label(),
                    );
                }
                scheduler_event
            }
            Ok(FightProgress::Won(proof)) => {
                if !Self::set_fight_controls(FightControl::default()) {
                    self.reject_input(inner, "fight_battle", -1, -1);
                    return scheduler_event;
                }
                self.write_trace_labeled(inner, RecordKind::SemanticAssertion, proof.trace_label());
                SchedulerEvent::NavigationReached
            }
            Err(error) => {
                if !Self::set_fight_controls(FightControl::default()) {
                    self.reject_input(inner, "fight_battle", -1, -1);
                    return scheduler_event;
                }
                self.write_trace_labeled(inner, RecordKind::SemanticAssertion, error.trace_label());
                self.set_terminal(inner, TerminalClass::SemanticMismatch);
                scheduler_event
            }
        }
    }

    fn service_hyperspace_departure(
        &self,
        inner: &mut CoordInner,
        mut scheduler_event: SchedulerEvent,
    ) -> SchedulerEvent {
        let snapshot = crate::automation::input_ffi::departure_snapshot();
        let solar_system_active = snapshot.active != 0;
        if solar_system_active && snapshot.in_ip_flight != 0 && !inner.hyperspace_flight_observed {
            inner.hyperspace_flight_observed = true;
            self.write_trace_labeled(
                inner,
                RecordKind::SemanticAssertion,
                format!(
                    "hyperspace_departure_started:ship={},{}:facing={}",
                    snapshot.ship_x, snapshot.ship_y, snapshot.ship_facing
                ),
            );
        }
        let activity = crate::mainloop::ffi::get_current_activity().0;
        if crate::automation::navigation::hyperspace_reached(
            inner.hyperspace_flight_observed,
            solar_system_active,
            activity,
        ) {
            if !Self::set_navigation_controls(
                crate::automation::navigation::NavigationControl::default(),
            ) {
                self.reject_input(inner, "navigation", -1, -1);
                return scheduler_event;
            }
            inner.hyperspace_flight_observed = false;
            scheduler_event = SchedulerEvent::NavigationReached;
            self.write_trace_labeled(
                inner,
                RecordKind::SemanticAssertion,
                crate::automation::navigation::HYPERSPACE_REACHED_LABEL.to_string(),
            );
            return scheduler_event;
        }
        let control = crate::automation::navigation::steer_out_of_system(
            crate::automation::navigation::NavigationObservation {
                active: solar_system_active,
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
                target_guard_active: false,
            },
        );
        if !Self::set_navigation_controls(control) {
            self.reject_input(inner, "navigation", -1, -1);
        }
        scheduler_event
    }

    /// Derive real player controls for whichever navigation the current step
    /// is running, and report arrival.
    fn service_navigation(
        &self,
        inner: &mut CoordInner,
        mut scheduler_event: SchedulerEvent,
    ) -> SchedulerEvent {
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
                    target_guard_active: false,
                },
            );
            if reached {
                if !Self::set_navigation_controls(
                    crate::automation::navigation::NavigationControl::default(),
                ) {
                    self.reject_input(inner, "navigation", -1, -1);
                    return scheduler_event;
                }
                scheduler_event = SchedulerEvent::NavigationReached;
                self.write_trace_labeled(
                    inner,
                    RecordKind::SemanticAssertion,
                    format!("navigation_reached:planet={}", navigation.planet),
                );
            } else if !Self::set_navigation_controls(control) {
                self.reject_input(inner, "navigation", -1, -1);
                return scheduler_event;
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
            // Inside the planet's inner system the flagship is inertial, so
            // aiming the nose alone accumulates drift and circles the planet
            // without touching it. Drive the velocity toward the planet, which
            // the snapshot reports at the view centre.
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
                    target_guard_active: false,
                },
            );
            if reached {
                if !Self::set_navigation_controls(
                    crate::automation::navigation::NavigationControl::default(),
                ) {
                    self.reject_input(inner, "navigation", -1, -1);
                    return scheduler_event;
                }
                inner.orbit_transition_pending = true;
                scheduler_event = SchedulerEvent::NavigationReached;
                self.write_trace_labeled(
                    inner,
                    RecordKind::SemanticAssertion,
                    format!("orbit_reached:planet={}", navigation.planet),
                );
            } else if !Self::set_navigation_controls(control) {
                self.reject_input(inner, "navigation", -1, -1);
                return scheduler_event;
            }
        } else if let Some(Action::NavigateToHyperspace(_)) =
            self.actions.get(inner.sched_state.step_index)
        {
            scheduler_event = self.service_hyperspace_departure(inner, scheduler_event);
        } else if let Some(Action::TurnFlagship(turn)) =
            self.actions.get(inner.sched_state.step_index)
        {
            scheduler_event = self.service_flagship_turn(inner, *turn, scheduler_event);
        } else if let Some(Action::CollectPlanetMinerals(collect)) =
            self.actions.get(inner.sched_state.step_index)
        {
            scheduler_event = self.service_planet_minerals(inner, *collect, scheduler_event);
        } else if let Some(Action::FightBattle(fight)) =
            self.actions.get(inner.sched_state.step_index)
        {
            scheduler_event = self.service_battle_fight(inner, *fight, scheduler_event);
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
            let control = crate::automation::navigation::steer_moon_route(
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
                    // The collision guard still holds this moon, so the ship is sitting in
                    // it without having entered orbit.
                    target_guard_active: snapshot.wait_intersect
                        == crate::automation::navigation::moon_wait_intersect(
                            navigation.planet,
                            navigation.moon,
                        ),
                },
                navigation.planet,
            );
            if let ActionPhase::Navigating { remaining } = inner.sched_state.phase {
                if remaining % 250 == 0 {
                    self.write_trace_labeled(
                        inner,
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
                if !Self::set_navigation_controls(
                    crate::automation::navigation::NavigationControl::default(),
                ) {
                    self.reject_input(inner, "navigation", -1, -1);
                    return scheduler_event;
                }
                scheduler_event = SchedulerEvent::NavigationReached;
                self.write_trace_labeled(
                    inner,
                    RecordKind::SemanticAssertion,
                    format!(
                        "navigation_reached:planet={}:moon={}:orbital_data={}:target_data={}",
                        navigation.planet,
                        navigation.moon,
                        snapshot.orbital_data_index,
                        snapshot.target_data_index
                    ),
                );
            } else if !Self::set_navigation_controls(control) {
                self.reject_input(inner, "navigation", -1, -1);
                return scheduler_event;
            }
        }
        scheduler_event
    }

    /// Replay menu transitions that arrived before the scheduler was ready.
    fn replay_pending_transitions(&self, inner: &mut CoordInner) {
        while inner.sched_state.phase == ActionPhase::WaitingSemantic
            && !inner.pending_transitions.is_empty()
            && !inner.sched_state.is_terminal()
            && !self.native_checkpoint_pending(inner)
        {
            let (from, to) = inner.pending_transitions.remove(0);
            let config = SchedulerConfig {
                actions: &self.actions,
                transitions: &self.transitions,
            };
            let transition = scheduler_reduce(
                &inner.sched_state,
                &config,
                SchedulerEvent::MenuTransition { from, to },
            );
            inner.sched_state = transition.new_state;
            let label = if inner.sched_state.terminal == Some(TerminalOutcome::SemanticMismatch) {
                format!("menu_transition_failed:from={from}:to={to}")
            } else {
                format!("menu_transition_passed:from={from}:to={to}")
            };
            self.write_trace_labeled(inner, RecordKind::SemanticAssertion, label);
        }
    }

    fn process_input_inner(&self) -> bool {
        let mut inner = self.inner.lock();

        if inner.terminal_class.is_some() {
            return true;
        }
        if self.reject_unobserved_player_input(&mut inner) {
            return true;
        }

        let released_semantic_select = inner.release_semantic_select;
        if released_semantic_select {
            let index = i32::from(crate::automation::script::MenuKey::Select.index());
            if !inject_menu_key(index, 0) {
                self.reject_input(&mut inner, "menu", index, 0);
                return true;
            }
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
            ready: inner.ready,
            idle: now.saturating_duration_since(inner.last_progress),
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

        if let Some(class) = watchdog_terminal_class(wd_result.outcome) {
            self.set_terminal(&mut inner, class);
            return true;
        }
        // Admitted: this callback is progress for the idle watchdog.
        inner.last_progress = now;

        // Keep the next action behind the committed-presentation barrier.
        if self.native_checkpoint_pending(&inner) {
            return false;
        }
        if inner.pending_main_menu_ready {
            inner.pending_main_menu_ready = false;
            return self.apply_main_menu_ready(&mut inner);
        }
        if self.verify_runtime_assertions(&mut inner) || self.verify_activity_assertion(&mut inner)
        {
            return true;
        }

        // Step 2: Feed to scheduler. Runtime waits are satisfied only by
        // observed production state, never by injected input.
        let mut scheduler_event = self.service_semantic_waits(
            &mut inner,
            released_semantic_select,
            orbit_transition_pending,
        );
        if inner.terminal_class.is_some() {
            return true;
        }

        scheduler_event = self.service_navigation(&mut inner, scheduler_event);

        if inner.terminal_class.is_some() {
            return true;
        }
        // The setup_planet_side_collision_fixture action is explicit and
        // one-shot: only the current action queues, only once.  A duplicate
        // while a request is still pending is a fail-fast semantic
        // mismatch instead of an idempotent success.
        if matches!(
            self.actions.get(inner.sched_state.step_index),
            Some(Action::SetupPlanetSideCollisionFixture(_))
        ) && inner.sched_state.phase == ActionPhase::WaitingForInput
            && scheduler_event == SchedulerEvent::AdmittedInput
        {
            match crate::planet_side::automation_fixture::coordinator_queues_fixture_request() {
                Ok(()) => {
                    self.write_trace_labeled(
                        &mut inner,
                        RecordKind::SemanticAssertion,
                        "planet_side_collision_fixture_queued".to_string(),
                    );
                }
                Err(error) => {
                    self.write_trace_labeled(
                        &mut inner,
                        RecordKind::SemanticAssertion,
                        format!(
                            "planet_side_collision_fixture_duplicate:{}",
                            error.operation
                        ),
                    );
                    self.set_terminal(&mut inner, TerminalClass::SemanticMismatch);
                    return true;
                }
            }
        }

        let config = SchedulerConfig {
            actions: &self.actions,
            transitions: &self.transitions,
        };
        let transition = scheduler_reduce(&inner.sched_state, &config, scheduler_event);
        let previous_state = inner.sched_state;
        inner.sched_state = transition.new_state;

        // Step 3: Apply effects.
        if !self.apply_effects(&mut inner, &transition.effects, None) {
            inner.sched_state =
                scheduler_state_after_effects(previous_state, transition.new_state, false);
            return true;
        }

        // Step 4: Write trace.
        self.write_trace(&mut inner, RecordKind::InputTick);

        // Step 4b: Replay menu transitions that arrived before the scheduler
        // was ready for them.
        self.replay_pending_transitions(&mut inner);

        // Step 5: Check terminal.
        if inner.sched_state.is_terminal() {
            if terminal_waits_for_player_input_observation(
                inner.sched_state,
                inner.pending_player_input.is_some(),
            ) {
                return false;
            }
            let class = map_scheduler_terminal(inner.sched_state.terminal);
            eprintln!(
                "[automation] scheduler terminal: {:?} -> class={:?}",
                inner.sched_state.terminal, class
            );
            if class == TerminalClass::SemanticMismatch {
                self.trace_unsatisfied_planet_side_wait(&mut inner);
            }
            self.set_terminal(&mut inner, class);
            return true;
        }

        false
    }

    /// When a `wait_for_planet_side_end` ran out of ticks, record the last
    /// observation it saw so the failure names the state the trip stalled in.
    fn trace_unsatisfied_planet_side_wait(&self, inner: &mut CoordInner) {
        let Some(Action::WaitForPlanetSideEnd(wait)) =
            self.actions.get(inner.sched_state.step_index)
        else {
            return;
        };
        let label = planet_side_wait_mismatch_label(
            inner.awaited_planet_side_generation,
            &crate::planet_side::telemetry::observation(),
            wait.outcome,
        );
        self.write_trace_labeled(inner, RecordKind::SemanticAssertion, label);
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

        if let Some(Action::AssertMode(assertion)) = self.actions.get(inner.sched_state.step_index)
        {
            match crate::automation::mode::require_live() {
                Ok(observed) if observed == assertion.mode => self.write_trace_labeled(
                    inner,
                    RecordKind::SemanticAssertion,
                    format!("mode_verified:{}", observed.name()),
                ),
                Ok(observed) => {
                    self.write_trace_labeled(
                        inner,
                        RecordKind::SemanticAssertion,
                        format!(
                            "mode_mismatch:expected={}:observed={}",
                            assertion.mode.name(),
                            observed.name()
                        ),
                    );
                    self.set_terminal(inner, TerminalClass::SemanticMismatch);
                    return true;
                }
                Err(error) => {
                    // An observation that names no mode, or more than one, is
                    // never resolved by picking a nearest match.
                    self.write_trace_labeled(
                        inner,
                        RecordKind::SemanticAssertion,
                        format!("mode_ambiguous:{error}"),
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
                Ok(actual) => {
                    inner.verified_battle_frames = inner.verified_battle_frames.max(actual);
                    self.write_trace_labeled(
                        inner,
                        RecordKind::SemanticAssertion,
                        format!("battle_frames_verified:count={actual}"),
                    );
                }
                Err(error) => {
                    self.write_trace_labeled(inner, RecordKind::SemanticAssertion, error);
                    self.set_terminal(inner, TerminalClass::SemanticMismatch);
                    return true;
                }
            }
        }
        if let Some(Action::RecordHyperspacePosition(_)) =
            self.actions.get(inner.sched_state.step_index)
        {
            match inner
                .flight_position
                .record(mode::require_live(), flight_position::live_position())
            {
                Ok(position) => self.write_trace_labeled(
                    inner,
                    RecordKind::SemanticAssertion,
                    flight_position::recorded_trace_label(position),
                ),
                Err(error) => {
                    self.write_trace_labeled(
                        inner,
                        RecordKind::SemanticAssertion,
                        error.trace_label(),
                    );
                    self.set_terminal(inner, TerminalClass::SemanticMismatch);
                    return true;
                }
            }
        }
        if let Some(Action::AssertHyperspaceDisplacement(assertion)) =
            self.actions.get(inner.sched_state.step_index)
        {
            match inner.flight_position.assert_displacement(
                mode::require_live(),
                flight_position::live_position(),
                assertion.minimum_logical_units,
            ) {
                Ok(proof) => self.write_trace_labeled(
                    inner,
                    RecordKind::SemanticAssertion,
                    proof.trace_label(),
                ),
                Err(error) => {
                    self.write_trace_labeled(
                        inner,
                        RecordKind::SemanticAssertion,
                        error.trace_label(),
                    );
                    self.set_terminal(inner, TerminalClass::SemanticMismatch);
                    return true;
                }
            }
        }
        if let Some(Action::AssertHyperspaceStationary(assertion)) =
            self.actions.get(inner.sched_state.step_index)
        {
            match inner.flight_position.assert_stationary(
                mode::require_live(),
                flight_position::live_position(),
                assertion.maximum_logical_units,
            ) {
                Ok(proof) => self.write_trace_labeled(
                    inner,
                    RecordKind::SemanticAssertion,
                    proof.trace_label(),
                ),
                Err(error) => {
                    self.write_trace_labeled(
                        inner,
                        RecordKind::SemanticAssertion,
                        error.trace_label(),
                    );
                    self.set_terminal(inner, TerminalClass::SemanticMismatch);
                    return true;
                }
            }
        }
        if let Some(Action::RecordSavedHyperspacePosition(_)) =
            self.actions.get(inner.sched_state.step_index)
        {
            match inner
                .flight_position
                .record_saved(mode::require_live(), flight_position::live_position())
            {
                Ok(position) => self.write_trace_labeled(
                    inner,
                    RecordKind::SemanticAssertion,
                    flight_position::saved_trace_label(position),
                ),
                Err(error) => {
                    self.write_trace_labeled(
                        inner,
                        RecordKind::SemanticAssertion,
                        error.trace_label(),
                    );
                    self.set_terminal(inner, TerminalClass::SemanticMismatch);
                    return true;
                }
            }
        }
        if let Some(Action::AssertHyperspacePositionRestored(_)) =
            self.actions.get(inner.sched_state.step_index)
        {
            match inner
                .flight_position
                .assert_restored(mode::require_live(), flight_position::live_position())
            {
                Ok((saved, observed)) => self.write_trace_labeled(
                    inner,
                    RecordKind::SemanticAssertion,
                    flight_position::restored_trace_label(saved, observed),
                ),
                Err(error) => {
                    self.write_trace_labeled(
                        inner,
                        RecordKind::SemanticAssertion,
                        error.trace_label(),
                    );
                    self.set_terminal(inner, TerminalClass::SemanticMismatch);
                    return true;
                }
            }
        }
        if let Some(Action::AssertShipResources(assertion)) =
            self.actions.get(inner.sched_state.step_index)
        {
            match ship_resources::assert_ship_resources(
                mode::require_live(),
                ship_resources::live_resources(),
                *assertion,
            ) {
                Ok(proof) => self.write_trace_labeled(
                    inner,
                    RecordKind::SemanticAssertion,
                    proof.trace_label(),
                ),
                Err(error) => {
                    self.write_trace_labeled(
                        inner,
                        RecordKind::SemanticAssertion,
                        error.trace_label(),
                    );
                    self.set_terminal(inner, TerminalClass::SemanticMismatch);
                    return true;
                }
            }
        }
        if let Some(Action::AssertState(predicate)) = self.actions.get(inner.sched_state.step_index)
        {
            let evaluation = campaign_state::evaluate(predicate, &campaign_state::LiveCampaign);
            if evaluation.held {
                self.write_trace_labeled(
                    inner,
                    RecordKind::SemanticAssertion,
                    evaluation.label("state_verified"),
                );
            } else {
                self.write_trace_labeled(
                    inner,
                    RecordKind::SemanticAssertion,
                    evaluation.failure_label("state_mismatch", predicate),
                );
                self.set_terminal(inner, TerminalClass::SemanticMismatch);
                return true;
            }
        }
        if let Some(Action::AssertShipElement(assertion)) =
            self.actions.get(inner.sched_state.step_index)
        {
            match ship_element::assert_ship_element(
                mode::require_live(),
                ship_element::live_amounts(),
                *assertion,
            ) {
                Ok(proof) => self.write_trace_labeled(
                    inner,
                    RecordKind::SemanticAssertion,
                    proof.trace_label(),
                ),
                Err(error) => {
                    self.write_trace_labeled(
                        inner,
                        RecordKind::SemanticAssertion,
                        error.trace_label(),
                    );
                    self.set_terminal(inner, TerminalClass::SemanticMismatch);
                    return true;
                }
            }
        }
        if let Some(Action::AssertPlanetSideCollisions(assertion)) =
            self.actions.get(inner.sched_state.step_index)
        {
            let observation = crate::planet_side::telemetry::observation();
            let actual = (
                observation.mineral_pickups,
                observation.creature_hits,
                observation.seam_hits,
            );
            let floors = (
                assertion.mineral_pickups,
                assertion.creature_hits,
                assertion.seam_hits,
            );
            if actual.0 >= floors.0 && actual.1 >= floors.1 && actual.2 >= floors.2 {
                self.write_trace_labeled(
                    inner,
                    RecordKind::SemanticAssertion,
                    format!(
                        "planet_side_collisions_verified:mineral={}:creature_hits={}:seam={}",
                        actual.0, actual.1, actual.2
                    ),
                );
            } else {
                self.write_trace_labeled(
                    inner,
                    RecordKind::SemanticAssertion,
                    format!(
                        "planet_side_collisions_failed:mineral={}≥{}:creature_hits={}≥{}:seam={}≥{}",
                        actual.0,
                        assertion.mineral_pickups,
                        actual.1,
                        assertion.creature_hits,
                        actual.2,
                        assertion.seam_hits
                    ),
                );
                self.set_terminal(inner, TerminalClass::SemanticMismatch);
                return true;
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
        let reservation = self.runtime.commit.reserve();
        let seq = reservation.sequence();
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
            readiness: None,
            command_acknowledgement: None,
            checkpoint: None,
            failure: None,
        };
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

    fn flush_native_trace(&self) -> std::io::Result<()> {
        if let Some(journal) = &self.native_trace {
            journal.lock().publish(&self.runtime.commit)?;
        }
        Ok(())
    }

    fn native_checkpoint_pending(&self, inner: &CoordInner) -> bool {
        self.native_checkpoint_identity.is_some()
            && self.actions[inner.checkpoint_step..inner.sched_state.step_index]
                .iter()
                .any(|action| super::native_checkpoint::checkpoint_class(action).is_some())
    }

    fn commit_native_checkpoints(
        &self,
        inner: &mut CoordInner,
        frame: &crate::automation::capture::PresentedFrame,
    ) -> bool {
        let Some(identity) = &self.native_checkpoint_identity else {
            return false;
        };
        for index in inner.checkpoint_step..inner.sched_state.step_index {
            if super::native_checkpoint::checkpoint_class(&self.actions[index]).is_none() {
                continue;
            }
            let id = super::native_checkpoint::checkpoint_id(identity, index);
            let reservation = self.runtime.commit.reserve();
            let mut record = super::capture::capture_trace_record(
                reservation.sequence(),
                self.started_at.elapsed().as_millis() as u64,
                CaptureGeneration(frame.generation),
                &id,
            );
            record.run = 1;
            record.kind = RecordKind::Checkpoint;
            record.input_seen = inner.input_seen;
            record.present_seen = inner.present_seen;
            record.presentation = Some(frame.presentation_evidence());
            record.checkpoint = Some(super::trace::CheckpointEvidence { id });
            match record.to_jsonl() {
                Ok(line) => reservation.commit_record(line),
                Err(error) => {
                    reservation.cancel();
                    self.capture_failure(inner, error);
                    return true;
                }
            }
        }
        inner.checkpoint_step = inner.sched_state.step_index;
        false
    }

    /// Process a committed present callback. Returns true if the game loop
    /// should stop.
    pub fn process_present(frame: Option<crate::automation::capture::PresentedFrame>) -> bool {
        let Some(coord) = Self::get() else {
            return false;
        };
        #[cfg(feature = "debug-process")]
        let committed_presentation = frame.as_ref().map(|frame| frame.count);
        let stop = coord.process_present_inner(frame);
        if let Err(error) = coord.flush_native_trace() {
            Self::external_trace_failure(format!(
                "native trace prefix publication failed: {error}"
            ));
            return true;
        }
        #[cfg(feature = "debug-process")]
        if let Some(committed_presentation) = committed_presentation {
            if let Err(error) = crate::automation::native_window::publish_native_window_presentation(
                committed_presentation,
            ) {
                Self::external_trace_failure(format!(
                    "native-window state publication failed: {error}"
                ));
                return true;
            }
        }
        stop
    }

    fn process_present_inner(
        &self,
        frame: Option<crate::automation::capture::PresentedFrame>,
    ) -> bool {
        let mut inner = self.inner.lock();

        if inner.terminal_class.is_some() {
            return true;
        }
        if self.reject_unobserved_player_input(&mut inner) {
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
            ready: inner.ready,
            idle: now.saturating_duration_since(inner.last_progress),
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
            WatchdogOutcome::Admit => inner.last_progress = now,
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
            WatchdogOutcome::StartupTimeout => {
                self.set_terminal(&mut inner, TerminalClass::StartupTimeout);
                return true;
            }
            WatchdogOutcome::IdleTimeout => {
                self.set_terminal(&mut inner, TerminalClass::IdleTimeout);
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

        if self.native_checkpoint_pending(&inner) {
            if self.write_presentation_trace(&mut inner, &frame) {
                return true;
            }
            return self.commit_native_checkpoints(&mut inner, &frame);
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
        let previous_state = inner.sched_state;
        inner.sched_state = transition.new_state;

        if self.write_presentation_trace(&mut inner, &frame) {
            return true;
        }
        if !self.apply_effects(&mut inner, &transition.effects, Some(&frame)) {
            inner.sched_state =
                scheduler_state_after_effects(previous_state, transition.new_state, false);
            return true;
        }
        if self.commit_native_checkpoints(&mut inner, &frame) {
            return true;
        }

        if inner.sched_state.is_terminal() {
            if terminal_waits_for_player_input_observation(
                inner.sched_state,
                inner.pending_player_input.is_some(),
            ) {
                return false;
            }
            let class = map_scheduler_terminal(inner.sched_state.terminal);
            self.set_terminal(&mut inner, class);
            return true;
        }

        false
    }

    // -----------------------------------------------------------------------
    //  Menu transition observation (called from handle_navigate)
    // -----------------------------------------------------------------------

    /// Advance a script waiting for the initialized production restart menu.
    pub fn process_main_menu_ready() -> bool {
        let Some(coord) = Self::get() else {
            return false;
        };
        let mut inner = coord.inner.lock();
        if inner.terminal_class.is_some() {
            return true;
        }
        if coord.native_checkpoint_pending(&inner) {
            inner.pending_main_menu_ready = true;
            inner.ready = true;
            inner.last_progress = Instant::now();
            return false;
        }
        coord.apply_main_menu_ready(&mut inner)
    }

    fn apply_main_menu_ready(&self, inner: &mut CoordInner) -> bool {
        let config = SchedulerConfig {
            actions: &self.actions,
            transitions: &self.transitions,
        };
        let transition =
            scheduler_reduce(&inner.sched_state, &config, SchedulerEvent::MainMenuReady);
        inner.sched_state = transition.new_state;
        // Readiness ends the startup budget and starts the idle one.
        inner.ready = true;
        inner.last_progress = Instant::now();
        self.write_readiness_trace(
            inner,
            MAIN_MENU_READINESS_SUBJECT,
            MAIN_MENU_READINESS_OBSERVATION,
        );
        if inner.sched_state.is_terminal() {
            let class = map_scheduler_terminal(inner.sched_state.terminal);
            self.set_terminal(inner, class);
            return true;
        }
        false
    }

    /// The restart menu set `CHECK_ABORT` for the Quit entry. Records the
    /// observation; whether the script declared it is decided at finalization.
    pub fn observe_quit_selected() {
        let Some(coord) = Self::get() else {
            return;
        };
        let mut inner = coord.inner.lock();
        if inner.terminal_class.is_some() {
            return;
        }
        inner.quit_selected = true;
        coord.write_trace_labeled(
            &mut inner,
            RecordKind::SemanticAssertion,
            "quit_selected:activity=0x4000".to_string(),
        );
    }

    /// Settle a run that ended with no terminal outcome. A quit is a success
    /// only when the restart menu reported it and the script declares
    /// `expect_quit` at the step the run stopped on (the select tap or the
    /// expectation itself). Any other early exit is a semantic mismatch.
    fn settle_exit_without_terminal(&self, inner: &mut CoordInner) {
        let step = inner.sched_state.step_index;
        let declared = match self.actions.get(step) {
            Some(Action::ExpectQuit(_)) => true,
            Some(Action::TapMenuKey(_)) => {
                matches!(self.actions.get(step + 1), Some(Action::ExpectQuit(_)))
            }
            _ => false,
        };
        let label = match (inner.quit_selected, declared) {
            (true, true) => {
                self.write_trace_labeled(
                    inner,
                    RecordKind::SemanticAssertion,
                    QUIT_VERIFIED_LABEL.to_string(),
                );
                self.set_terminal(inner, TerminalClass::Success);
                return;
            }
            (true, false) => "quit_unexpected:script_did_not_declare_expect_quit",
            (false, true) => "quit_expected:game_exited_without_quit_selection",
            (false, false) => {
                self.set_terminal(inner, TerminalClass::SemanticMismatch);
                return;
            }
        };
        self.write_trace_labeled(inner, RecordKind::SemanticAssertion, label.to_string());
        self.set_terminal(inner, TerminalClass::SemanticMismatch);
    }

    /// Process an observed main-menu transition. Returns true if the game
    /// loop should stop (e.g., semantic assertion mismatch).
    pub fn process_menu_transition(from_index: u8, to_index: u8) -> bool {
        let Some(coord) = Self::get() else {
            return false;
        };
        coord.process_menu_transition_inner(from_index, to_index)
    }

    fn process_menu_transition_inner(&self, from_index: u8, to_index: u8) -> bool {
        let mut inner = self.inner.lock();
        if inner.terminal_class.is_some() {
            return true;
        }
        inner.pending_transitions.push((from_index, to_index));
        self.replay_pending_transitions(&mut inner);
        if inner.sched_state.is_terminal() {
            let class = map_scheduler_terminal(inner.sched_state.terminal);
            self.set_terminal(&mut inner, class);
            return true;
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
        // Clear any pending fixture request before runtime finalization:
        // the single fixture is one-shot per script, so an unconsumed queue
        // must never leak into another script.
        crate::planet_side::automation_fixture::clear_pending_fixture_request();

        let terminal = {
            let mut inner = self.inner.lock();
            if inner.finalized {
                return Err("automation coordinator finalized more than once".into());
            }
            self.reject_unobserved_player_input(&mut inner);
            if inner.terminal_class.is_none() {
                self.settle_exit_without_terminal(&mut inner);
            }
            inner.finalized = true;
            self.write_terminal_trace(&mut inner);
            inner.terminal_class
        };

        validate_runtime_finalization(self.runtime.finalize())?;
        let status = active_automation_status(terminal, game_result)?;

        if self.native_trace.is_some() {
            self.flush_native_trace()
                .map_err(|error| format!("cannot finalize native trace: {error}"))?;
        } else {
            let mut trace = Vec::new();
            self.runtime
                .commit
                .publish_all(&mut trace)
                .map_err(|error| format!("cannot publish ordered automation trace: {error}"))?;
            crate::automation::artifact::write_durable(&self.output_root, "trace", "jsonl", &trace)
                .map_err(|error| format!("cannot durably publish automation trace: {error}"))?;
        }

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
        if !record_first_terminal(&mut inner.terminal_class, class) {
            return;
        }
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

    /// Service whichever planet-side wait is the current action.
    ///
    /// Returns `None` when the current action is not a planet-side wait, so the
    /// caller can fall through to the remaining action handlers.
    fn service_planet_side_wait(&self, inner: &mut CoordInner) -> Option<PlanetSideWaitResult> {
        match self.actions.get(inner.sched_state.step_index) {
            Some(Action::WaitForPlanetSideStart(_)) => Some(self.observe_planet_side_start(inner)),
            Some(Action::WaitForPlanetSideEnd(wait)) => {
                Some(self.observe_planet_side_end(inner, wait.outcome))
            }
            _ => None,
        }
    }

    /// Record a serviced planet-side wait, returning the scheduler event to use.
    fn record_planet_side_result(
        &self,
        inner: &mut CoordInner,
        result: PlanetSideWaitResult,
        current: SchedulerEvent,
    ) -> SchedulerEvent {
        for label in result.labels {
            self.write_trace_labeled(inner, RecordKind::SemanticAssertion, label);
        }
        if result.reached {
            SchedulerEvent::ConditionReached
        } else {
            current
        }
    }

    /// Latch the generation of a newly started planet-side trip.
    fn observe_planet_side_start(&self, inner: &mut CoordInner) -> PlanetSideWaitResult {
        let observation = crate::planet_side::telemetry::observation();
        if !observation.active {
            return PlanetSideWaitResult::pending();
        }
        inner.awaited_planet_side_generation = Some(observation.generation);
        inner.observed_planet_side_phase = None;
        PlanetSideWaitResult::reached(format!(
            "planet_side_started:generation={}:crew={}:position={},{}",
            observation.generation,
            observation.start_crew,
            observation.start_x,
            observation.start_y
        ))
    }

    /// Accept the settled outcome of the trip this wait is bound to, tracing
    /// every lifecycle transition seen along the way.
    ///
    /// A trip publishes its terminal code only once it has settled, and that
    /// code stays resident afterwards. Matching on the latched generation is
    /// therefore required: without it a later wait would immediately consume an
    /// earlier trip's result. Transitions are rare, so recording each one shows
    /// whether a trip that never ends is stuck before takeoff or stalled in the
    /// takeoff animation. Deduplication is keyed by generation as well as phase
    /// so the same phase in a later trip is not swallowed.
    fn observe_planet_side_end(
        &self,
        inner: &mut CoordInner,
        outcome: crate::automation::script::PlanetSideOutcomeName,
    ) -> PlanetSideWaitResult {
        let observation = crate::planet_side::telemetry::observation();
        let mut result = PlanetSideWaitResult::pending();

        let seen = (observation.generation, observation.phase);
        if inner.observed_planet_side_phase != Some(seen) {
            inner.observed_planet_side_phase = Some(seen);
            result.labels.push(format!(
                "planet_side_phase:generation={}:phase={}",
                observation.generation,
                crate::planet_side::telemetry::phase_name(observation.phase)
            ));
        }

        if !planet_side_wait_satisfied(inner.awaited_planet_side_generation, &observation, outcome)
        {
            return result;
        }
        inner.awaited_planet_side_generation = None;
        result.reached = true;
        result.labels.push(format!(
            "planet_side_completed:generation={}:outcome={outcome:?}:crew={}:minerals={}",
            observation.generation, observation.returned_crew, observation.returned_minerals
        ));
        result
    }

    /// Hold exactly the thrust, turn and fire keys of `control`.
    fn set_fight_controls(control: FightControl) -> bool {
        use crate::automation::script::PlayerKey;

        [
            (PlayerKey::Thrust, control.thrust),
            (PlayerKey::Left, control.left),
            (PlayerKey::Right, control.right),
            (PlayerKey::Weapon, control.weapon),
        ]
        .into_iter()
        .all(|(key, active)| inject_player_key(i32::from(key.index()), i32::from(active)))
    }

    fn set_navigation_controls(control: crate::automation::navigation::NavigationControl) -> bool {
        use crate::automation::script::{MenuKey, PlayerKey};

        for (key, active) in [
            (PlayerKey::Thrust, control.thrust),
            (PlayerKey::Left, control.left),
            (PlayerKey::Right, control.right),
        ] {
            if !inject_player_key(i32::from(key.index()), i32::from(active)) {
                return false;
            }
        }

        let (cancel, up, select) = if control.leave_orbit {
            orbit_exit_menu_keys(ORBIT_EXIT_PHASE.fetch_add(1, Ordering::AcqRel))
        } else {
            ORBIT_EXIT_PHASE.store(0, Ordering::Release);
            (false, false, false)
        };
        for (key, active) in [
            (MenuKey::Cancel, cancel),
            (MenuKey::Up, up),
            (MenuKey::Select, select),
        ] {
            if !inject_menu_key(i32::from(key.index()), i32::from(active)) {
                return false;
            }
        }
        true
    }

    /// Apply planned effects from the scheduler reducer.
    fn apply_effects(
        &self,
        inner: &mut CoordInner,
        effects: &EffectPlan,
        frame: Option<&crate::automation::capture::PresentedFrame>,
    ) -> bool {
        if let Some((key, value)) = effects.write_key {
            let index = crate::automation::input::menu_key_to_index(key);
            if !inject_menu_key(i32::from(index), i32::from(value)) {
                self.reject_input(inner, "menu", i32::from(index), i32::from(value));
                return false;
            }
        }
        inner.held_repeat_key = effects.held_repeat_key;
        if let Some(key) = effects.release_key {
            let index = crate::automation::input::menu_key_to_index(key);
            if !inject_menu_key(i32::from(index), 0) {
                self.reject_input(inner, "menu", i32::from(index), 0);
                return false;
            }
        }
        if let Some((key, value)) = effects.write_player_key {
            if !self.queue_player_input_observation(inner, key, i32::from(value)) {
                return false;
            }
        }
        if let Some(key) = effects.release_player_key {
            if !self.queue_player_input_observation(inner, key, 0) {
                return false;
            }
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
        if let Some(count) = effects.complete_presentation_wait {
            self.write_trace_labeled(
                inner,
                RecordKind::SemanticAssertion,
                presentation_wait_trace_label(count),
            );
        }
        true
    }

    fn queue_player_input_observation(
        &self,
        inner: &mut CoordInner,
        key: crate::automation::script::PlayerKey,
        value: i32,
    ) -> bool {
        if self.reject_unobserved_player_input(inner) {
            return false;
        }
        let Some(injection) = prepare_player_input(i32::from(key.index()), value) else {
            self.reject_input(inner, "player", i32::from(key.index()), value);
            return false;
        };
        inner.pending_player_input = Some(PendingAcceptedPlayerInput { key, injection });
        true
    }

    fn reject_player_input_observation(
        &self,
        inner: &mut CoordInner,
        pending: PendingAcceptedPlayerInput,
        observed: Option<(i32, i32)>,
    ) {
        let label = match observed {
            Some((current, pulsed)) => format!(
                "player_input_observation_contradictory:key={:?}:intended={}:current={current}:pulsed={pulsed}",
                pending.key, pending.injection.value
            ),
            None => format!(
                "player_input_observation_missing:key={:?}:intended={}",
                pending.key, pending.injection.value
            ),
        };
        self.write_trace_labeled(inner, RecordKind::SemanticAssertion, label);
        self.set_terminal(inner, TerminalClass::SemanticMismatch);
    }

    fn reject_unobserved_player_input(&self, inner: &mut CoordInner) -> bool {
        let Some(pending) = inner.pending_player_input.take() else {
            return false;
        };
        self.write_trace_labeled(
            inner,
            RecordKind::SemanticAssertion,
            format!(
                "player_input_observation_missing:key={:?}:intended={}:reason=next_callback",
                pending.key, pending.injection.value
            ),
        );
        self.set_terminal(inner, TerminalClass::SemanticMismatch);
        true
    }

    fn reject_input(&self, inner: &mut CoordInner, domain: &str, index: i32, value: i32) {
        self.write_trace_labeled(
            inner,
            RecordKind::SemanticAssertion,
            format!("input_rejected:domain={domain}:index={index}:value={value}"),
        );
        self.set_terminal(inner, TerminalClass::SemanticMismatch);
    }

    /// Write a trace record through the ordered commit.
    fn write_trace(&self, inner: &mut CoordInner, kind: RecordKind) {
        let reservation = self.runtime.commit.reserve();
        let seq = reservation.sequence();

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
            readiness: None,
            command_acknowledgement: None,
            checkpoint: None,
            failure: None,
        };

        if let Ok(jsonl) = record.to_jsonl() {
            reservation.commit_record(jsonl);
        }
    }

    fn write_terminal_trace(&self, inner: &mut CoordInner) {
        let reservation = self.runtime.commit.reserve();
        let seq = reservation.sequence();
        let terminal_reason = inner
            .terminal_class
            .and_then(|class| serde_json::to_value(class).ok())
            .and_then(|value| value.as_str().map(str::to_owned));
        let record = TraceRecord {
            schema: TraceRecord::SCHEMA,
            run: 1,
            sequence: seq,
            input_seen: inner.input_seen,
            present_seen: inner.present_seen,
            elapsed_ms: self.started_at.elapsed().as_millis() as u64,
            kind: RecordKind::RunEnd,
            label: None,
            from: None,
            to: None,
            terminal_reason,
            seed_application: None,
            presentation: None,
            activity: None,
            readiness: None,
            command_acknowledgement: None,
            checkpoint: None,
            failure: None,
        };
        if let Ok(jsonl) = record.to_jsonl() {
            reservation.commit_record(jsonl);
        }
    }

    /// Write a trace record with a semantic/evidence label.
    fn write_trace_labeled(&self, inner: &mut CoordInner, kind: RecordKind, label: String) {
        let reservation = self.runtime.commit.reserve();
        let seq = reservation.sequence();

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
            readiness: None,
            command_acknowledgement: None,
            checkpoint: None,
            failure: None,
        };

        if let Ok(jsonl) = record.to_jsonl() {
            reservation.commit_record(jsonl);
        }
    }

    /// Write a readiness trace record carrying what became ready and the
    /// observation that proved it.
    fn write_readiness_trace(&self, inner: &mut CoordInner, subject: &str, observation: &str) {
        let reservation = self.runtime.commit.reserve();
        let seq = reservation.sequence();

        let record = TraceRecord {
            schema: TraceRecord::SCHEMA,
            run: 1,
            sequence: seq,
            input_seen: inner.input_seen,
            present_seen: inner.present_seen,
            elapsed_ms: self.started_at.elapsed().as_millis() as u64,
            kind: RecordKind::Readiness,
            label: None,
            from: None,
            to: None,
            terminal_reason: None,
            seed_application: None,
            presentation: None,
            activity: None,
            readiness: Some(ReadinessEvidence {
                subject: subject.to_string(),
                observation: observation.to_string(),
            }),
            command_acknowledgement: None,
            checkpoint: None,
            failure: None,
        };

        if let Ok(jsonl) = record.to_jsonl() {
            reservation.commit_record(jsonl);
        }
    }

    fn write_presentation_trace(
        &self,
        inner: &mut CoordInner,
        frame: &crate::automation::capture::PresentedFrame,
    ) -> bool {
        let reservation = self.runtime.commit.reserve();
        let seq = reservation.sequence();
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
            readiness: None,
            command_acknowledgement: None,
            checkpoint: None,
            failure: None,
        };
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

        let reservation = self.runtime.commit.reserve();
        let mut record = crate::automation::capture::capture_trace_record(
            reservation.sequence(),
            self.started_at.elapsed().as_millis() as u64,
            generation,
            label,
        );
        record.run = 1;
        record.input_seen = inner.input_seen;
        record.present_seen = inner.present_seen;
        record.presentation = Some(frame.presentation_evidence());
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

fn record_first_terminal(slot: &mut Option<TerminalClass>, class: TerminalClass) -> bool {
    if slot.is_some() {
        return false;
    }
    *slot = Some(class);
    true
}
fn scheduler_state_after_effects(
    previous: SchedulerState,
    transitioned: SchedulerState,
    accepted: bool,
) -> SchedulerState {
    if accepted {
        transitioned
    } else {
        previous
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
    let reservation = coord.runtime.commit.reserve();
    let seq = reservation.sequence();
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
            seed: coord.seed,
        }),
        presentation: None,
        activity: None,
        readiness: None,
        command_acknowledgement: None,
        checkpoint: None,
        failure: None,
    };
    match record.to_jsonl() {
        Ok(jsonl) => reservation.commit_record(jsonl),
        Err(_) => {
            reservation.cancel();
            coord.set_terminal(&mut inner, TerminalClass::TraceFailure);
        }
    }
    coord.seed
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
/// Map a watchdog outcome to the terminal class it ends the run with.
///
/// `None` means the callback is admitted and the run continues.
fn watchdog_terminal_class(outcome: WatchdogOutcome) -> Option<TerminalClass> {
    match outcome {
        WatchdogOutcome::Admit => None,
        WatchdogOutcome::InputCounterOverflow | WatchdogOutcome::PresentationCounterOverflow => {
            Some(TerminalClass::CounterOverflow)
        }
        WatchdogOutcome::InputTimeout => Some(TerminalClass::InputTimeout),
        WatchdogOutcome::PresentationTimeout => Some(TerminalClass::PresentationTimeout),
        WatchdogOutcome::WallTimeout => Some(TerminalClass::WallTimeout),
        WatchdogOutcome::ClockRegression => Some(TerminalClass::ClockRegression),
        WatchdogOutcome::StartupTimeout => Some(TerminalClass::StartupTimeout),
        WatchdogOutcome::IdleTimeout => Some(TerminalClass::IdleTimeout),
    }
}

fn map_scheduler_terminal(terminal: Option<TerminalOutcome>) -> TerminalClass {
    match terminal {
        Some(TerminalOutcome::FinishComplete) => TerminalClass::Success,
        Some(TerminalOutcome::SemanticMismatch) => TerminalClass::SemanticMismatch,
        Some(TerminalOutcome::CaptureMismatch) => TerminalClass::CaptureMismatch,
        Some(TerminalOutcome::StateVersionOverflow) => TerminalClass::StateVersionOverflow,
        Some(TerminalOutcome::CaptureGenerationOverflow) => {
            TerminalClass::CaptureGenerationOverflow
        }
        Some(TerminalOutcome::InvalidState) => TerminalClass::SemanticMismatch,
        None => TerminalClass::CooperativeStop,
    }
}

fn battle_frame_floor_reached(actual: u64, minimum: u64) -> bool {
    actual >= minimum
}

// ===========================================================================
//  Unit tests
// ===========================================================================

fn terminal_waits_for_player_input_observation(
    scheduler_state: SchedulerState,
    observation_pending: bool,
) -> bool {
    scheduler_state.is_terminal() && observation_pending
}

fn accepted_player_input_count(current: u64, observed_value: i32) -> Option<u64> {
    if observed_value == 0 {
        Some(current)
    } else {
        current.checked_add(1)
    }
}

fn player_input_trace_label(key: crate::automation::script::PlayerKey, value: i32) -> String {
    format!("player_input:key={key:?}:value={value}")
}

fn player_input_observation_trace_label(
    key: crate::automation::script::PlayerKey,
    intended: i32,
    current: i32,
    pulsed: i32,
) -> String {
    format!(
        "player_input_observed:key={key:?}:intended={intended}:current={current}:pulsed={pulsed}"
    )
}

fn presentation_wait_trace_label(count: u64) -> String {
    format!("wait_presentations:{count}")
}

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
    fn semantic_trace_labels_bind_observed_player_input_and_presentation_waits() {
        assert_eq!(
            player_input_trace_label(crate::automation::script::PlayerKey::Thrust, 1),
            "player_input:key=Thrust:value=1"
        );
        assert_eq!(
            player_input_observation_trace_label(
                crate::automation::script::PlayerKey::Weapon,
                1,
                1,
                1,
            ),
            "player_input_observed:key=Weapon:intended=1:current=1:pulsed=1"
        );
        assert_eq!(presentation_wait_trace_label(300), "wait_presentations:300");
    }

    #[test]
    fn battle_frame_wait_requires_the_observed_floor() {
        assert!(!battle_frame_floor_reached(0, 1));
        assert!(!battle_frame_floor_reached(299, 300));
        assert!(battle_frame_floor_reached(1, 1));
        assert!(battle_frame_floor_reached(334, 300));
    }

    #[test]
    fn post_update_observation_accepts_and_rejects_without_a_presentation() {
        let press = PendingPlayerInput { index: 4, value: 1 };
        assert_eq!(
            observe_player_input_with(press, |_| Some((1, 1))),
            PlayerInputObservation::Matched {
                current: 1,
                pulsed: 1,
            }
        );
        assert_eq!(
            observe_player_input_with(press, |_| Some((1, 0))),
            PlayerInputObservation::Matched {
                current: 1,
                pulsed: 0,
            }
        );
        let release = PendingPlayerInput { index: 4, value: 0 };
        assert_eq!(
            observe_player_input_with(release, |_| Some((0, 0))),
            PlayerInputObservation::Matched {
                current: 0,
                pulsed: 0,
            }
        );
        assert_eq!(
            observe_player_input_with(release, |_| Some((1, 0))),
            PlayerInputObservation::Contradictory {
                current: 1,
                pulsed: 0,
            }
        );
        assert_eq!(
            observe_player_input_with(press, |_| None),
            PlayerInputObservation::Missing
        );
    }

    #[test]
    fn held_tap_discards_the_repeat_pulse_of_exactly_its_key() {
        use crate::automation::script::MenuKey;

        let mut discarded = Vec::new();
        assert!(discard_held_repeat(Some(MenuKey::Down), |index| {
            discarded.push(index);
            true
        }));
        assert_eq!(discarded, [6]);

        assert!(discard_held_repeat(None, |index| {
            discarded.push(index);
            true
        }));
        assert_eq!(discarded, [6], "a press or an idle update discards nothing");

        assert!(!discard_held_repeat(Some(MenuKey::Select), |_| false));
    }

    #[test]
    fn accepted_player_input_count_changes_only_for_observed_press() {
        assert_eq!(accepted_player_input_count(7, 0), Some(7));
        assert_eq!(accepted_player_input_count(7, 1), Some(8));
        assert_eq!(accepted_player_input_count(u64::MAX, 1), None);
    }

    #[test]
    fn terminal_success_waits_for_pending_player_input_observation() {
        let mut terminal = SchedulerState::initial();
        terminal.terminal = Some(TerminalOutcome::FinishComplete);

        assert!(terminal_waits_for_player_input_observation(terminal, true));
        assert!(!terminal_waits_for_player_input_observation(
            terminal, false
        ));
        assert!(!terminal_waits_for_player_input_observation(
            SchedulerState::initial(),
            true,
        ));
    }

    /// A game that exits right after the restart menu sets `CHECK_ABORT` never
    /// reaches another input callback. Only a script that declares
    /// `expect_quit` and a quit the menu reported settle as a success.
    #[test]
    fn a_quit_is_a_success_only_when_declared_and_reported() {
        const CHILD: &str = "UQM_QUIT_SETTLEMENT_TEST";
        const CASES: [&str; 3] = [
            "declared_and_reported",
            "undeclared",
            "declared_not_reported",
        ];
        let Some(case) = std::env::var_os(CHILD) else {
            for case in CASES {
                let output = std::process::Command::new(std::env::current_exe().unwrap())
                    .args(["--exact", "automation::coordinator::tests::a_quit_is_a_success_only_when_declared_and_reported", "--nocapture"])
                    .env(CHILD, case)
                    .output()
                    .unwrap();
                assert!(
                    output.status.success(),
                    "{case}\n{}\n{}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            return;
        };
        let case = case.to_string_lossy().into_owned();
        let tap = r#"{"action":"tap_menu_key","key":"select","value":1,"hold":1,"settle":1}"#;
        let steps = if case == "undeclared" {
            format!("[{tap},{{\"action\":\"finish\"}}]")
        } else {
            format!("[{tap},{{\"action\":\"expect_quit\"}},{{\"action\":\"finish\"}}]")
        };
        let source = format!(
            r#"{{"version":1,"name":"quit-settlement","budgets":{{"max_input_ticks":100,"max_presentations":100,"max_wallclock_seconds":60}},"steps":{steps}}}"#
        );
        let temp = tempfile::tempdir().unwrap();
        let script = crate::automation::validate_script(
            crate::automation::parse_script(source.as_bytes(), "quit.json").unwrap(),
            "quit.json",
        )
        .unwrap();
        Coordinator::init(script, temp.path().to_path_buf());
        if case != "declared_not_reported" {
            Coordinator::observe_quit_selected();
        }
        let status = Coordinator::finalize(0).unwrap();
        let records: Vec<_> = std::fs::read_to_string(temp.path().join("trace.jsonl"))
            .unwrap()
            .lines()
            .map(|line| TraceRecord::from_jsonl(line).unwrap())
            .collect();
        let labels: Vec<&str> = records
            .iter()
            .filter_map(|record| record.label.as_deref())
            .collect();
        let terminal = records.last().unwrap().terminal_reason.as_deref();
        if case == "declared_and_reported" {
            assert_eq!(status, 0);
            assert_eq!(terminal, Some("success"));
            assert!(labels.contains(&QUIT_VERIFIED_LABEL), "{labels:?}");
        } else {
            assert_ne!(status, 0, "{case}: an unproven exit must fail");
            assert_eq!(terminal, Some("semantic_mismatch"));
            assert!(!labels.contains(&QUIT_VERIFIED_LABEL), "{labels:?}");
            let expected = if case == "undeclared" {
                "quit_unexpected:script_did_not_declare_expect_quit"
            } else {
                "quit_expected:game_exited_without_quit_selection"
            };
            assert!(labels.contains(&expected), "{labels:?}");
        }
    }

    #[test]
    fn native_capture_then_activity_requires_verified_distinct_commit() {
        const CHILD: &str = "UQM_CAPTURE_ACTIVITY_COMMIT_TEST";
        if std::env::var_os(CHILD).is_none() {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "automation::coordinator::tests::native_capture_then_activity_requires_verified_distinct_commit", "--nocapture"])
                .env(CHILD, "1").output().unwrap();
            assert!(
                output.status.success(),
                "{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }
        let temp = tempfile::tempdir().unwrap();
        let source = br#"{"version":1,"name":"capture-activity","budgets":{"max_input_ticks":100,"max_presentations":100,"max_wallclock_seconds":60},"steps":[{"action":"capture","label":"before-activity"},{"action":"assert_activity","mask":0,"equals":0},{"action":"capture","label":"after-activity"},{"action":"finish"}]}"#;
        let script = crate::automation::validate_script(
            crate::automation::parse_script(source, "test.json").unwrap(),
            "test.json",
        )
        .unwrap();
        let plan =
            super::super::native_checkpoint::NativeCheckpointPlan::derive(source, &script).unwrap();
        Coordinator::init_with_native_checkpoints(script, temp.path().into(), Some(65536));
        let coord = Coordinator::get().unwrap();
        let frame = |count, generation| super::super::capture::PresentedFrame {
            count,
            generation,
            width: 1,
            height: 1,
            rgba: vec![1, 2, 3, 255],
        };
        let records = || {
            std::fs::read_to_string(temp.path().join("trace.jsonl"))
                .unwrap()
                .lines()
                .map(|line| TraceRecord::from_jsonl(line).unwrap())
                .collect::<Vec<_>>()
        };
        assert!(!coord.process_input_inner());
        assert!(!Coordinator::process_present(Some(frame(1, 1))));
        assert_eq!(
            coord.inner.lock().sched_state.step_index,
            1,
            "capture must leave activity awaiting its runtime verification"
        );
        assert_eq!(plan.committed(&records()).unwrap().len(), 1);
        assert!(!coord.process_input_inner());
        assert_eq!(coord.inner.lock().sched_state.step_index, 2);
        assert!(!coord.process_input_inner());
        assert!(coord.inner.lock().armed_capture_label.is_none());
        assert!(!Coordinator::process_present(Some(frame(2, 1))));
        assert_eq!(plan.committed(&records()).unwrap().len(), 2);
        assert!(!coord.process_input_inner());
        assert!(!Coordinator::process_present(Some(frame(3, 2))));
        assert_eq!(plan.committed(&records()).unwrap().len(), 3);
        assert!(coord.process_input_inner());
        assert_eq!(Coordinator::finalize(0).unwrap(), 0);
        let trace = records();
        let counts: Vec<_> = trace
            .iter()
            .filter(|record| record.kind == RecordKind::Checkpoint)
            .map(|record| record.presentation.as_ref().unwrap().count)
            .collect();
        assert_eq!(counts, [1, 2, 3]);
        assert_eq!(
            trace
                .iter()
                .filter(|record| record
                    .activity
                    .as_ref()
                    .is_some_and(|evidence| evidence.passed))
                .count(),
            1
        );
    }

    #[test]
    fn native_checkpoint_barrier_queues_readiness_until_prior_commit() {
        const CHILD: &str = "UQM_NATIVE_READY_BARRIER_TEST";
        if std::env::var_os(CHILD).is_none() {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "automation::coordinator::tests::native_checkpoint_barrier_queues_readiness_until_prior_commit", "--nocapture"])
                .env(CHILD, "1").output().unwrap();
            assert!(
                output.status.success(),
                "{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }
        let temp = tempfile::tempdir().unwrap();
        let source = br#"{"version":1,"name":"readiness-barrier","budgets":{"max_input_ticks":300,"max_presentations":100,"max_wallclock_seconds":60},"steps":[{"action":"wait_for_main_menu_ready"},{"action":"wait_for_main_menu_ready"},{"action":"finish"}]}"#;
        let script = crate::automation::validate_script(
            crate::automation::parse_script(source, "test.json").unwrap(),
            "test.json",
        )
        .unwrap();
        let plan =
            super::super::native_checkpoint::NativeCheckpointPlan::derive(source, &script).unwrap();
        Coordinator::init_with_native_checkpoints(script, temp.path().into(), Some(65536));
        let coord = Coordinator::get().unwrap();
        assert!(!Coordinator::process_main_menu_ready());
        assert!(!Coordinator::process_main_menu_ready());
        assert_eq!(coord.inner.lock().sched_state.step_index, 1);
        assert!(coord.inner.lock().pending_main_menu_ready);
        for count in 1..=2 {
            assert!(!Coordinator::process_present(Some(
                super::super::capture::PresentedFrame {
                    count,
                    generation: 0,
                    width: 1,
                    height: 1,
                    rgba: vec![1, 2, 3, 255]
                }
            )));
            let records: Vec<_> = std::fs::read_to_string(temp.path().join("trace.jsonl"))
                .unwrap()
                .lines()
                .map(|line| TraceRecord::from_jsonl(line).unwrap())
                .collect();
            assert_eq!(plan.committed(&records).unwrap().len() as u64, count);
            assert_eq!(coord.process_input_inner(), count == 2);
        }
        assert!(!coord.inner.lock().pending_main_menu_ready);
        assert_eq!(Coordinator::finalize(0).unwrap(), 0);
    }

    #[test]
    fn native_checkpoint_barrier_holds_next_action_until_committed_presentation() {
        const CHILD: &str = "UQM_NATIVE_CHECKPOINT_BARRIER_TEST";
        if std::env::var_os(CHILD).is_none() {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "automation::coordinator::tests::native_checkpoint_barrier_holds_next_action_until_committed_presentation", "--nocapture"])
                .env(CHILD, "1").output().unwrap();
            assert!(
                output.status.success(),
                "{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }
        let temp = tempfile::tempdir().unwrap();
        let source = br#"{"version":1,"name":"barrier-test","budgets":{"max_input_ticks":200,"max_presentations":100,"max_wallclock_seconds":60},"steps":[{"action":"wait_for_main_menu_ready"},{"action":"assert_main_menu_transition","from":"NewGame","to":"LoadGame"},{"action":"assert_main_menu_transition","from":"LoadGame","to":"NewGame"},{"action":"capture","label":"menu"},{"action":"finish"}]}"#;
        let script = crate::automation::validate_script(
            crate::automation::parse_script(source, "test.json").unwrap(),
            "test.json",
        )
        .unwrap();
        let plan =
            super::super::native_checkpoint::NativeCheckpointPlan::derive(source, &script).unwrap();
        Coordinator::init_with_native_checkpoints(script, temp.path().into(), Some(65536));
        let coord = Coordinator::get().unwrap();
        assert!(!Coordinator::process_main_menu_ready());
        for _ in 0..3 {
            assert!(!coord.process_input_inner());
            assert_eq!(coord.inner.lock().sched_state.step_index, 1);
            assert!(coord.inner.lock().armed_capture_label.is_none());
        }
        let frame = |count, generation| super::super::capture::PresentedFrame {
            count,
            generation,
            width: 1,
            height: 1,
            rgba: vec![1, 2, 3, 255],
        };
        assert!(!Coordinator::process_menu_transition(0, 1));
        assert!(!Coordinator::process_menu_transition(1, 0));
        assert!(!Coordinator::process_present(Some(frame(1, 0))));
        let live_trace = std::fs::read_to_string(temp.path().join("trace.jsonl")).unwrap();
        let live_records: Vec<_> = live_trace
            .lines()
            .map(|line| TraceRecord::from_jsonl(line).unwrap())
            .collect();
        assert_eq!(
            live_records.len() as u64,
            Coordinator::native_window_semantic_snapshot().unwrap().0
        );
        assert_eq!(plan.committed(&live_records).unwrap().len(), 1);
        assert!(!coord.process_input_inner());
        assert_eq!(coord.inner.lock().sched_state.step_index, 2);
        assert_eq!(coord.inner.lock().pending_transitions.len(), 1);
        assert!(!coord.process_input_inner());
        assert_eq!(coord.inner.lock().sched_state.step_index, 2);
        assert!(!Coordinator::process_present(Some(frame(2, 0))));
        assert!(!coord.process_input_inner());
        assert_eq!(coord.inner.lock().sched_state.step_index, 3);
        assert!(coord.inner.lock().pending_transitions.is_empty());
        assert!(!Coordinator::process_present(Some(frame(3, 0))));
        assert!(!coord.process_input_inner());
        assert_eq!(
            coord.inner.lock().armed_capture_label.as_deref(),
            Some("menu")
        );
        assert!(!Coordinator::process_present(Some(frame(4, 1))));
        assert!(coord.process_input_inner());
        assert_eq!(
            coord.inner.lock().terminal_class,
            Some(TerminalClass::Success)
        );
        assert_eq!(Coordinator::finalize(0).unwrap(), 0);
        let trace = std::fs::read(temp.path().join("trace.jsonl")).unwrap();
        let records: Vec<_> = std::str::from_utf8(&trace)
            .unwrap()
            .lines()
            .map(|line| TraceRecord::from_jsonl(line).unwrap())
            .collect();
        assert_eq!(plan.committed(&records).unwrap().len(), 4);
        let generations: Vec<_> = records
            .iter()
            .filter(|record| record.kind == RecordKind::Checkpoint)
            .map(|record| record.presentation.as_ref().unwrap().count)
            .collect();
        assert_eq!(generations, [1, 2, 3, 4]);
    }

    #[test]
    fn replay_seed_is_applied_at_each_rng_boundary() {
        const CHILD: &str = "UQM_REPLAY_SEED_TEST_CHILD";
        if std::env::var_os(CHILD).is_none() {
            for seed in [0, 42, u32::MAX] {
                let output = std::process::Command::new(std::env::current_exe().unwrap())
                    .args(["--exact", "automation::coordinator::tests::replay_seed_is_applied_at_each_rng_boundary", "--nocapture"])
                    .env(CHILD, seed.to_string())
                    .output().unwrap();
                assert!(
                    output.status.success(),
                    "{}\n{}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            return;
        }
        let seed: u32 = std::env::var(CHILD).unwrap().parse().unwrap();
        let applied_seed = seed.max(1);
        let temp = tempfile::tempdir().unwrap();
        let bytes = br#"{"version":2,"name":"seed-test","seed":42,"fixture":"seed-test","budgets":{"max_input_ticks":2,"max_presentations":2,"max_wallclock_seconds":1},"steps":[{"action":"finish"}]}"#;
        let mut doc = crate::automation::script::parse_script(bytes, "seed-test.json").unwrap();
        doc.seed = Some(u64::from(seed));
        let script = crate::automation::script::validate_script(doc, "seed-test.json").unwrap();
        assert_eq!(
            rust_automation_seed_value(SeedDomain::NEW_GAME_ID, 999),
            999
        );
        Coordinator::init(script, temp.path().to_path_buf());
        let mut expected = Vec::new();
        for domain in [
            SeedDomain::SUPER_MELEE_MENU_ID,
            SeedDomain::SUPER_MELEE_BATTLE_ID,
            SeedDomain::NEW_GAME_ID,
        ] {
            assert_eq!(rust_automation_seed_value(domain, 999), applied_seed);
            expected.push(SeedApplication {
                domain: SeedDomain::from_ffi(domain).unwrap(),
                seed: applied_seed,
            });
        }
        let mut trace = Vec::new();
        Coordinator::get()
            .unwrap()
            .runtime
            .commit
            .publish_all(&mut trace)
            .unwrap();
        let applications: Vec<_> = std::str::from_utf8(&trace)
            .unwrap()
            .lines()
            .map(|line| TraceRecord::from_jsonl(line).unwrap())
            .filter_map(|record| record.seed_application)
            .collect();
        assert_eq!(applications, expected);
        let record: crate::automation::lifecycle::ResolvedScenarioRecord = serde_json::from_slice(
            &std::fs::read(temp.path().join("resolved-scenario.json")).unwrap(),
        )
        .unwrap();
        record.validate().unwrap();
        assert_eq!(record.scenario.seed, applied_seed);
        assert_eq!(record.scenario.requested_seed, seed);
    }

    /// Automation runs the Rust restart menu, so the new-campaign seed
    /// must be applied there, not only in the retired C `RestartMenu`.
    #[test]
    fn rust_restart_path_applies_and_records_the_new_game_seed() {
        const CHILD: &str = "UQM_RESTART_SEED_TEST_CHILD";
        if std::env::var_os(CHILD).is_none() {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "automation::coordinator::tests::rust_restart_path_applies_and_records_the_new_game_seed",
                    "--nocapture",
                ])
                .env(CHILD, "1")
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }
        let temp = tempfile::tempdir().unwrap();
        let bytes = br#"{"version":2,"name":"restart-seed","seed":1437213463,"fixture":"restart-seed","budgets":{"max_input_ticks":2,"max_presentations":2,"max_wallclock_seconds":1},"steps":[{"action":"finish"}]}"#;
        let doc = crate::automation::script::parse_script(bytes, "restart-seed.json").unwrap();
        let script = crate::automation::script::validate_script(doc, "restart-seed.json").unwrap();

        // Without automation the clock seeder runs and nothing is recorded.
        let mut clock_used = false;
        crate::mainloop::restart_menu::restart_ops::seed_new_campaign(|| clock_used = true);
        assert!(clock_used, "inactive automation must keep the clock seed");

        Coordinator::init(script, temp.path().to_path_buf());
        crate::mainloop::restart_menu::restart_ops::seed_new_campaign(|| {
            panic!("the clock seed must not run while automation supplies a seed")
        });
        let after_restart = crate::math::TFB_Random();
        crate::math::TFB_SeedRandom(1437213463);
        assert_eq!(
            after_restart,
            crate::math::TFB_Random(),
            "the generator must be seeded from the scenario seed"
        );

        let mut trace = Vec::new();
        Coordinator::get()
            .unwrap()
            .runtime
            .commit
            .publish_all(&mut trace)
            .unwrap();
        let applications: Vec<_> = std::str::from_utf8(&trace)
            .unwrap()
            .lines()
            .map(|line| TraceRecord::from_jsonl(line).unwrap())
            .filter_map(|record| record.seed_application)
            .collect();
        assert_eq!(
            applications,
            [SeedApplication {
                domain: SeedDomain::NewGame,
                seed: 1437213463,
            }]
        );
    }

    #[test]
    fn replay_stops_if_resolved_input_receipt_cannot_be_published() {
        const CHILD: &str = "UQM_REPLAY_RECEIPT_TEST_CHILD";
        if std::env::var_os(CHILD).is_none() {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "automation::coordinator::tests::replay_stops_if_resolved_input_receipt_cannot_be_published", "--nocapture"])
                .env(CHILD, "1").output().unwrap();
            assert!(
                output.status.success(),
                "{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join("resolved-scenario.json"), b"existing").unwrap();
        let doc = crate::automation::script::parse_script(
            br#"{"version":1,"name":"receipt","budgets":{"max_input_ticks":2,"max_presentations":2,"max_wallclock_seconds":1},"steps":[{"action":"finish"}]}"#,
            "receipt.json",
        ).unwrap();
        Coordinator::init(
            crate::automation::script::validate_script(doc, "receipt.json").unwrap(),
            temp.path().to_path_buf(),
        );
        assert_eq!(
            Coordinator::get().unwrap().inner.lock().terminal_class,
            Some(TerminalClass::TraceFailure)
        );
        assert_eq!(
            std::fs::read(temp.path().join("resolved-scenario.json")).unwrap(),
            b"existing"
        );
    }

    #[test]
    fn terminal_class_is_first_writer_wins() {
        let mut terminal = None;
        assert!(record_first_terminal(
            &mut terminal,
            TerminalClass::SemanticMismatch,
        ));
        assert!(!record_first_terminal(
            &mut terminal,
            TerminalClass::TraceFailure,
        ));
        assert_eq!(terminal, Some(TerminalClass::SemanticMismatch));
    }
    #[test]
    fn rejected_effects_do_not_advance_scheduler_state() {
        let previous = SchedulerState::initial();
        let mut transitioned = previous;
        transitioned.step_index = 1;
        transitioned.state_version = 1;

        assert_eq!(
            scheduler_state_after_effects(previous, transitioned, false),
            previous
        );
        assert_eq!(
            scheduler_state_after_effects(previous, transitioned, true),
            transitioned
        );
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

    fn settled(
        generation: u64,
        terminal: u32,
    ) -> crate::planet_side::telemetry::PlanetSideObservation {
        crate::planet_side::telemetry::PlanetSideObservation {
            generation,
            active: false,
            terminal,
            ..Default::default()
        }
    }

    #[test]
    fn planet_side_wait_accepts_only_the_trip_it_is_bound_to() {
        let outcome = crate::automation::script::PlanetSideOutcomeName::Returned;
        let code = outcome.terminal_code();
        assert!(planet_side_wait_satisfied(
            Some(2),
            &settled(2, code),
            outcome
        ));
    }

    #[test]
    fn exhausted_planet_side_wait_names_the_state_it_last_observed() {
        let outcome = crate::automation::script::PlanetSideOutcomeName::Returned;
        let mut observation = settled(1, crate::planet_side::telemetry::terminal::DESTROYED);
        observation.active = true;
        observation.phase = 8;
        observation.current_crew = 0;
        assert_eq!(
            planet_side_wait_mismatch_label(Some(1), &observation, outcome),
            "planet_side_wait_unsatisfied:awaited=Some(1):expected_terminal=1:\
             generation=1:active=true:terminal=2:phase=Complete:crew=0"
        );
    }

    #[test]
    fn planet_side_wait_rejects_a_trip_that_is_still_running() {
        let outcome = crate::automation::script::PlanetSideOutcomeName::Returned;
        let mut observation = settled(2, outcome.terminal_code());
        observation.active = true;
        assert!(!planet_side_wait_satisfied(Some(2), &observation, outcome));
    }

    #[test]
    fn planet_side_wait_rejects_an_earlier_settled_trip() {
        // The terminal code stays resident after a trip settles, so a wait must
        // not be satisfied by the previous trip's result.
        let outcome = crate::automation::script::PlanetSideOutcomeName::Returned;
        let code = outcome.terminal_code();
        assert!(!planet_side_wait_satisfied(
            Some(3),
            &settled(2, code),
            outcome
        ));
    }

    #[test]
    fn planet_side_wait_requires_a_started_trip() {
        let outcome = crate::automation::script::PlanetSideOutcomeName::Returned;
        let code = outcome.terminal_code();
        assert!(!planet_side_wait_satisfied(
            None,
            &settled(1, code),
            outcome
        ));
    }

    #[test]
    fn planet_side_wait_rejects_a_different_outcome() {
        use crate::automation::script::PlanetSideOutcomeName;
        let destroyed = PlanetSideOutcomeName::Destroyed.terminal_code();
        assert!(!planet_side_wait_satisfied(
            Some(1),
            &settled(1, destroyed),
            PlanetSideOutcomeName::Returned
        ));
        assert!(planet_side_wait_satisfied(
            Some(1),
            &settled(1, destroyed),
            PlanetSideOutcomeName::Destroyed
        ));
    }

    #[test]
    fn planet_side_wait_is_consumed_once() {
        let outcome = crate::automation::script::PlanetSideOutcomeName::Returned;
        let code = outcome.terminal_code();
        let mut awaited = Some(1);
        assert!(planet_side_wait_satisfied(
            awaited,
            &settled(1, code),
            outcome
        ));
        awaited = None; // the coordinator clears the latch on acceptance
        assert!(!planet_side_wait_satisfied(
            awaited,
            &settled(1, code),
            outcome
        ));
    }

    #[test]
    fn orbit_exit_cancels_then_presses_up_then_select_with_releases_between() {
        // `DoMenuChooser` only sees a new press when the pulsed key was
        // released on an intervening callback, so the sequence must be
        // press-Cancel, release, press-Up, release, press-Select, release.
        // Cancel comes first because the ship can be inside the scan menu
        // (for example right after a lander returns), where Up and Select
        // would scan instead of leaving orbit.
        assert_eq!(orbit_exit_menu_keys(0), (true, false, false));
        assert_eq!(orbit_exit_menu_keys(1), (false, false, false));
        assert_eq!(orbit_exit_menu_keys(2), (false, true, false));
        assert_eq!(orbit_exit_menu_keys(3), (false, false, false));
        assert_eq!(orbit_exit_menu_keys(4), (false, false, true));
        assert_eq!(orbit_exit_menu_keys(5), (false, false, false));
    }

    #[test]
    fn orbit_exit_never_presses_two_keys_on_one_callback() {
        for phase in 0..64 {
            let (cancel, up, select) = orbit_exit_menu_keys(phase);
            assert!(
                u8::from(cancel) + u8::from(up) + u8::from(select) <= 1,
                "phase {phase} pressed more than one key together"
            );
        }
    }

    #[test]
    fn orbit_exit_never_selects_before_cancel_and_up_have_been_pressed() {
        let first_press = |wanted: fn((bool, bool, bool)) -> bool| {
            (0..ORBIT_EXIT_PERIOD)
                .find(|phase| wanted(orbit_exit_menu_keys(*phase)))
                .expect("every key is pressed once per period")
        };
        let cancel = first_press(|keys| keys.0);
        let up = first_press(|keys| keys.1);
        let select = first_press(|keys| keys.2);
        assert!(cancel < up && up < select);
    }

    #[test]
    fn orbit_exit_sequence_repeats_so_a_missed_press_is_retried() {
        for phase in 0..64 {
            assert_eq!(
                orbit_exit_menu_keys(phase),
                orbit_exit_menu_keys(phase + ORBIT_EXIT_PERIOD),
                "phase {phase} did not repeat after one period"
            );
        }
    }

    #[test]
    fn orbit_exit_phase_restarts_at_the_cancel_press_after_a_reset() {
        // `set_navigation_controls` stores zero whenever `leave_orbit` is
        // false, so the next orbit exit must begin with the Cancel press
        // again rather than resuming mid-sequence.
        let phase = ORBIT_EXIT_PHASE.load(Ordering::Acquire);
        assert_eq!(
            orbit_exit_menu_keys(phase.wrapping_sub(phase)),
            (true, false, false)
        );
        assert_eq!(orbit_exit_menu_keys(0), (true, false, false));
    }

    #[test]
    fn every_watchdog_outcome_maps_to_its_terminal_class() {
        use super::watchdog_terminal_class;
        assert_eq!(watchdog_terminal_class(WatchdogOutcome::Admit), None);
        assert_eq!(
            watchdog_terminal_class(WatchdogOutcome::InputCounterOverflow),
            Some(TerminalClass::CounterOverflow)
        );
        assert_eq!(
            watchdog_terminal_class(WatchdogOutcome::PresentationCounterOverflow),
            Some(TerminalClass::CounterOverflow)
        );
        assert_eq!(
            watchdog_terminal_class(WatchdogOutcome::InputTimeout),
            Some(TerminalClass::InputTimeout)
        );
        assert_eq!(
            watchdog_terminal_class(WatchdogOutcome::PresentationTimeout),
            Some(TerminalClass::PresentationTimeout)
        );
        assert_eq!(
            watchdog_terminal_class(WatchdogOutcome::WallTimeout),
            Some(TerminalClass::WallTimeout)
        );
        assert_eq!(
            watchdog_terminal_class(WatchdogOutcome::ClockRegression),
            Some(TerminalClass::ClockRegression)
        );
    }
}
