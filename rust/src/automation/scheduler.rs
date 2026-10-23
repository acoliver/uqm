//! Pure scheduler reducer and capture-generation model.
//!
//! Implements execution-contract §2.3 (scheduler reducer table) and §2.4
//! (capture generation) exactly, as pure typed types with no side effects.
//!
//! @plan PLAN-20260723-RUNTIME-AUTOMATION.P02
//! @requirement REQ-SCHED-001..003, REQ-DET-001

use crate::automation::script::{Action, MainMenuTransition, MenuKey, PlayerKey};

// ===========================================================================
//  Capture generation model (execution-contract §2.4)
// ===========================================================================

/// A nonzero capture generation. `0` means "no capture requested".
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P02
/// @requirement REQ-SCHED-001
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct CaptureGeneration(pub u64);

impl CaptureGeneration {
    /// `true` if this generation is armed (nonzero).
    #[must_use]
    pub const fn is_armed(self) -> bool {
        self.0 != 0
    }

    /// Reserve the next nonzero generation using checked add.
    ///
    /// @plan PLAN-20260723-RUNTIME-AUTOMATION.P02
    /// @requirement REQ-SCHED-001, REQ-SCHED-003
    #[must_use]
    pub fn next(self) -> Option<CaptureGeneration> {
        self.0.checked_add(1).map(CaptureGeneration)
    }
}

/// Result of validating a capture completion against the pending request.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P02
/// @requirement REQ-SCHED-001
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CaptureValidation {
    /// Generation matches — completion allowed.
    Match,
    /// Generation is zero — no capture was armed.
    Zero,
    /// Generation is stale — it is less than the pending request.
    Stale,
    /// Generation is a duplicate — it equals an already-completed request.
    Duplicate,
    /// Generation is in the future — it exceeds the pending request.
    Future,
}

/// Validate a capture completion against pending metadata.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P02
/// @requirement REQ-SCHED-001
#[must_use]
pub fn validate_capture_completion(
    pending: CaptureGeneration,
    completed: CaptureGeneration,
    already_completed: bool,
) -> CaptureValidation {
    if completed.0 == 0 {
        return CaptureValidation::Zero;
    }
    if already_completed {
        return CaptureValidation::Duplicate;
    }
    if !pending.is_armed() {
        // Nothing pending — any nonzero is stale (nothing to complete).
        return CaptureValidation::Stale;
    }
    match completed.0.cmp(&pending.0) {
        std::cmp::Ordering::Equal => CaptureValidation::Match,
        std::cmp::Ordering::Less => CaptureValidation::Stale,
        std::cmp::Ordering::Greater => CaptureValidation::Future,
    }
}

// ===========================================================================
//  Scheduler state
// ===========================================================================

/// Within-action processing phase.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P02
/// @requirement REQ-SCHED-002
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActionPhase {
    /// Waiting for the first admitted input callback.
    WaitingForInput,
    /// Waiting for `remaining` admitted input callbacks (wait_input_ticks).
    WaitCounting { remaining: u64 },
    /// Waiting for `remaining` committed presentation callbacks.
    WaitingPresentations { remaining: u64 },
    /// Tap is holding: planning held value each admitted input.
    TapHolding { remaining: u64 },
    /// Tap just released; needs one more admitted input to begin settling.
    TapReleasePending,
    /// Tap is settling: consuming admitted input callbacks.
    TapSettling { remaining: u64 },
    /// Driving the real flagship toward a planet with a bounded callback budget.
    Navigating { remaining: u64 },
    /// Waiting for a runtime-observed semantic condition with a bounded budget.
    WaitingCondition { remaining: u64 },
    /// Waiting for a committed present with a matching capture generation.
    WaitingCapture { generation: CaptureGeneration },
    /// Waiting for a typed main-menu transition event.
    WaitingSemantic,
}

/// Terminal outcome types.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P02
/// @requirement REQ-DET-001
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TerminalOutcome {
    /// Script completed normally via `finish`.
    FinishComplete,
    /// Semantic assertion mismatch (wrong from/to).
    SemanticMismatch,
    /// Active capture generation mismatch.
    CaptureMismatch,
    /// State version overflow (REQ-SCHED-003 checked arithmetic).
    StateVersionOverflow,
    /// Capture generation overflow (REQ-SCHED-003 checked arithmetic).
    CaptureGenerationOverflow,
    /// The step index or action phase cannot occur in a valid scheduler run.
    InvalidState,
}

/// The scheduler's state: current step index, within-action phase, state
/// version, capture metadata, and terminal outcome (if any).
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P02
/// @requirement REQ-SCHED-001, REQ-DET-001
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SchedulerState {
    pub step_index: usize,
    pub phase: ActionPhase,
    pub state_version: u64,
    pub capture_generation: CaptureGeneration,
    pub terminal: Option<TerminalOutcome>,
}

impl SchedulerState {
    /// `true` if this state is terminal (absorbing).
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        self.terminal.is_some()
    }

    /// Initial state at step 0, waiting for input.
    #[must_use]
    pub const fn initial() -> Self {
        Self {
            step_index: 0,
            phase: ActionPhase::WaitingForInput,
            state_version: 0,
            capture_generation: CaptureGeneration(0),
            terminal: None,
        }
    }
}

// ===========================================================================
//  Scheduler events and effects
// ===========================================================================

/// Events consumed by the scheduler reducer.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P02
/// @requirement REQ-SCHED-001
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SchedulerEvent {
    /// An input callback admitted by the watchdog.
    AdmittedInput,
    /// A committed present carrying a capture generation (0 = no capture).
    CommittedPresent { generation: CaptureGeneration },
    /// Real navigation entered the requested planet's inner system.
    NavigationReached,
    /// A runtime-observed condition required by the current action was reached.
    ConditionReached,
    /// The production restart menu finished initialization and accepts input.
    MainMenuReady,
    /// An observed typed main-menu transition.
    MenuTransition { from: u8, to: u8 },
}

/// Planned effects produced by the scheduler reducer.
///
/// These are declarative: the actual execution (C/SDL/file operations)
/// happens in later phases. The reducer only *plans* them.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P02
/// @requirement REQ-SCHED-001
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct EffectPlan {
    /// Write an owned menu key to `value` (0 or 1).
    pub write_key: Option<(MenuKey, u8)>,
    /// Release an owned menu key (write 0).
    pub release_key: Option<MenuKey>,
    /// A tapped menu key that this update only holds down after the press
    /// that already registered. Its production auto-repeat pulse must not
    /// register as another press.
    pub held_repeat_key: Option<MenuKey>,
    /// Write an owned player-one gameplay key to `value`.
    pub write_player_key: Option<(PlayerKey, u8)>,
    /// Release an owned player-one gameplay key.
    pub release_player_key: Option<PlayerKey>,
    /// Arm capture with this generation.
    pub arm_capture: Option<CaptureGeneration>,
    /// Complete a capture with this generation.
    pub complete_capture: Option<CaptureGeneration>,
    /// Complete a committed-presentation wait with this requested count.
    pub complete_presentation_wait: Option<u64>,
}

impl EffectPlan {
    /// An empty effect plan (no effects).
    #[must_use]
    pub const fn none() -> Self {
        Self {
            write_key: None,
            release_key: None,
            held_repeat_key: None,
            write_player_key: None,
            release_player_key: None,
            arm_capture: None,
            complete_capture: None,
            complete_presentation_wait: None,
        }
    }

    /// `true` if this plan has no effects.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.write_key.is_none()
            && self.release_key.is_none()
            && self.held_repeat_key.is_none()
            && self.write_player_key.is_none()
            && self.release_player_key.is_none()
            && self.arm_capture.is_none()
            && self.complete_capture.is_none()
            && self.complete_presentation_wait.is_none()
    }
}

/// The result of a scheduler reducer transition.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P02
/// @requirement REQ-SCHED-001
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SchedulerTransition {
    /// Proposed new state (commit only if state_version matches).
    pub new_state: SchedulerState,
    /// Planned effects for the external layer to execute.
    pub effects: EffectPlan,
}

// ===========================================================================
//  Scheduler config
// ===========================================================================

/// Immutable scheduler configuration: the validated actions and transitions.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P02
/// @requirement REQ-DET-001
#[derive(Debug, Clone)]
pub struct SchedulerConfig<'a> {
    pub actions: &'a [Action],
    pub transitions: &'a [MainMenuTransition],
}

// ===========================================================================
//  Pure scheduler reducer (execution-contract §2.3 table)
// ===========================================================================

/// Apply the scheduler reducer for one admitted event.
///
/// Terminal state is absorbing: the reducer returns the same state with no
/// effects.
///
fn valid_action_phase(action: &Action, phase: ActionPhase) -> bool {
    match action {
        Action::WaitInputTicks(_) => matches!(
            phase,
            ActionPhase::WaitingForInput | ActionPhase::WaitCounting { .. }
        ),
        Action::WaitPresentations(_) => matches!(
            phase,
            ActionPhase::WaitingForInput | ActionPhase::WaitingPresentations { .. }
        ),
        Action::TapMenuKey(_) | Action::TapPlayerKey(_) => matches!(
            phase,
            ActionPhase::WaitingForInput
                | ActionPhase::TapHolding { .. }
                | ActionPhase::TapReleasePending
                | ActionPhase::TapSettling { .. }
        ),
        Action::NavigateToPlanet(_)
        | Action::NavigateToContact(_)
        | Action::NavigateToMoon(_)
        | Action::NavigateToOrbit(_)
        | Action::UseDevice(_)
        | Action::CollectEnergyNode(_)
        | Action::FlyToStar(_)
        | Action::NavigateToHyperspace(_)
        | Action::TurnFlagship(_)
        | Action::CollectPlanetMinerals(_)
        | Action::FightBattle(_)
        | Action::FightSamatra(_)
        | Action::FightEncounter(_) => {
            matches!(
                phase,
                ActionPhase::WaitingForInput | ActionPhase::Navigating { .. }
            )
        }
        Action::WaitForPlanetSideStart(_)
        | Action::WaitForPlanetSideEnd(_)
        | Action::SelectPlanetMenu(_)
        | Action::PickBattleShip(_)
        | Action::WaitForBattleFrames(_)
        | Action::WaitForBattleShipLoss(_)
        | Action::WaitForDispatch(_)
        | Action::WaitForState(_)
        | Action::SelectResponse(_)
        | Action::SelectCommunicationResponse(_)
        | Action::WaitForCommunicationResponses(_)
        | Action::WaitForCommunicationEnd(_)
        | Action::WaitForCommunicationReplay(_) => matches!(
            phase,
            ActionPhase::WaitingForInput | ActionPhase::WaitingCondition { .. }
        ),
        Action::Capture(_) => matches!(
            phase,
            ActionPhase::WaitingForInput | ActionPhase::WaitingCapture { .. }
        ),
        Action::AssertMainMenuTransition(_) => matches!(
            phase,
            ActionPhase::WaitingForInput | ActionPhase::WaitingSemantic
        ),
        Action::WaitForMainMenuReady(_) => phase == ActionPhase::WaitingForInput,
        _ => phase == ActionPhase::WaitingForInput,
    }
}

/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P02
/// @requirement REQ-SCHED-001, REQ-SCHED-002, REQ-DET-001
#[must_use]
pub fn scheduler_reduce(
    state: &SchedulerState,
    config: &SchedulerConfig<'_>,
    event: SchedulerEvent,
) -> SchedulerTransition {
    // Terminal state is absorbing.
    if state.is_terminal() {
        return SchedulerTransition {
            new_state: *state,
            effects: EffectPlan::none(),
        };
    }

    // REQ-SCHED-003: state_version uses checked arithmetic; overflow is
    // a typed terminal failure.
    let sv = match state.state_version.checked_add(1) {
        Some(v) => v,
        None => {
            return SchedulerTransition {
                new_state: SchedulerState {
                    terminal: Some(TerminalOutcome::StateVersionOverflow),
                    ..*state
                },
                effects: EffectPlan::none(),
            };
        }
    };

    let Some(current_action) = config.actions.get(state.step_index) else {
        return SchedulerTransition {
            new_state: SchedulerState {
                terminal: Some(TerminalOutcome::InvalidState),
                state_version: sv,
                ..*state
            },
            effects: EffectPlan::none(),
        };
    };
    if !valid_action_phase(current_action, state.phase) {
        return SchedulerTransition {
            new_state: SchedulerState {
                terminal: Some(TerminalOutcome::InvalidState),
                state_version: sv,
                ..*state
            },
            effects: EffectPlan::none(),
        };
    }

    match event {
        SchedulerEvent::AdmittedInput => reduce_admitted_input(state, config, current_action, sv),
        SchedulerEvent::CommittedPresent { generation } => {
            reduce_committed_present(state, config, generation, current_action, sv)
        }
        SchedulerEvent::NavigationReached => {
            if matches!(
                current_action,
                Action::NavigateToPlanet(_)
                    | Action::NavigateToContact(_)
                    | Action::NavigateToMoon(_)
                    | Action::NavigateToOrbit(_)
                    | Action::UseDevice(_)
                    | Action::CollectEnergyNode(_)
                    | Action::FlyToStar(_)
                    | Action::NavigateToHyperspace(_)
                    | Action::TurnFlagship(_)
                    | Action::CollectPlanetMinerals(_)
                    | Action::FightBattle(_)
                    | Action::FightSamatra(_)
                    | Action::FightEncounter(_)
            ) {
                match state.phase {
                    ActionPhase::WaitingForInput | ActionPhase::Navigating { remaining: 1.. } => {
                        advance_to_next(state, config, sv, EffectPlan::none())
                    }
                    ActionPhase::Navigating { remaining: 0 } => SchedulerTransition {
                        new_state: SchedulerState {
                            terminal: Some(TerminalOutcome::SemanticMismatch),
                            state_version: sv,
                            ..*state
                        },
                        effects: EffectPlan::none(),
                    },
                    _ => SchedulerTransition {
                        new_state: *state,
                        effects: EffectPlan::none(),
                    },
                }
            } else {
                SchedulerTransition {
                    new_state: *state,
                    effects: EffectPlan::none(),
                }
            }
        }
        SchedulerEvent::ConditionReached => {
            if matches!(
                current_action,
                Action::WaitForCommunicationEnd(_)
                    | Action::WaitForCommunicationReplay(_)
                    | Action::WaitForBattleFrames(_)
                    | Action::WaitForBattleShipLoss(_)
                    | Action::WaitForDispatch(_)
                    | Action::WaitForState(_)
                    | Action::SelectResponse(_)
                    | Action::SelectCommunicationResponse(_)
                    | Action::WaitForCommunicationResponses(_)
                    | Action::WaitForPlanetSideStart(_)
                    | Action::WaitForPlanetSideEnd(_)
                    | Action::SelectPlanetMenu(_)
                    | Action::PickBattleShip(_)
            ) {
                match state.phase {
                    ActionPhase::WaitingForInput
                    | ActionPhase::WaitingCondition { remaining: 1.. } => {
                        advance_to_next(state, config, sv, EffectPlan::none())
                    }
                    ActionPhase::WaitingCondition { remaining: 0 } => SchedulerTransition {
                        new_state: SchedulerState {
                            terminal: Some(TerminalOutcome::SemanticMismatch),
                            state_version: sv,
                            ..*state
                        },
                        effects: EffectPlan::none(),
                    },
                    _ => SchedulerTransition {
                        new_state: *state,
                        effects: EffectPlan::none(),
                    },
                }
            } else {
                SchedulerTransition {
                    new_state: *state,
                    effects: EffectPlan::none(),
                }
            }
        }
        SchedulerEvent::MainMenuReady => {
            if matches!(current_action, Action::WaitForMainMenuReady(_)) {
                advance_to_next(state, config, sv, EffectPlan::none())
            } else {
                SchedulerTransition {
                    new_state: *state,
                    effects: EffectPlan::none(),
                }
            }
        }
        SchedulerEvent::MenuTransition { from, to } => {
            reduce_menu_transition(state, config, from, to, sv)
        }
    }
}

/// Process an admitted input callback.
///
/// Each family below answers only for the actions it owns, and the families
/// are disjoint, so the order they are consulted in does not change behaviour.
fn reduce_admitted_input(
    state: &SchedulerState,
    config: &SchedulerConfig<'_>,
    action: &Action,
    sv: u64,
) -> SchedulerTransition {
    if let Some(transition) = reduce_input_timing(state, config, action, sv) {
        return transition;
    }
    if let Some(transition) = reduce_menu_key(state, config, action, sv) {
        return transition;
    }
    if let Some(transition) = reduce_player_key(state, config, action, sv) {
        return transition;
    }
    if let Some(transition) = reduce_navigation(state, action, sv) {
        return transition;
    }
    if let Some(transition) = reduce_condition_wait(state, action, sv) {
        return transition;
    }
    if let Some(transition) = reduce_capture_and_assertion(state, config, action, sv) {
        return transition;
    }

    // Any event/action pair not handled above is impossible for this phase.
    SchedulerTransition {
        new_state: SchedulerState {
            terminal: Some(TerminalOutcome::InvalidState),
            state_version: sv,
            ..*state
        },
        effects: EffectPlan::none(),
    }
}

/// Reduce a timing wait, or decline so the next family may answer.
fn reduce_input_timing(
    state: &SchedulerState,
    config: &SchedulerConfig<'_>,
    action: &Action,
    sv: u64,
) -> Option<SchedulerTransition> {
    Some(match (action, state.phase) {
        // ExpectQuit holds like a readiness wait: the quit the game reports at
        // finalization ends the run, and a game that never quits times out.
        (Action::WaitForMainMenuReady(_) | Action::ExpectQuit(_), ActionPhase::WaitingForInput) => {
            SchedulerTransition {
                new_state: SchedulerState {
                    state_version: sv,
                    ..*state
                },
                effects: EffectPlan::none(),
            }
        }
        (Action::WaitInputTicks(w), ActionPhase::WaitingForInput) if w.count == 0 => {
            advance_to_next(state, config, sv, EffectPlan::none())
        }

        // Ready wait_input_ticks(n>0): enter WaitCounting(n-1), consuming this callback.
        (Action::WaitInputTicks(w), ActionPhase::WaitingForInput) => {
            if w.count <= 1 {
                advance_to_next(state, config, sv, EffectPlan::none())
            } else {
                SchedulerTransition {
                    new_state: SchedulerState {
                        phase: ActionPhase::WaitCounting {
                            remaining: w.count - 1,
                        },
                        state_version: sv,
                        ..*state
                    },
                    effects: EffectPlan::none(),
                }
            }
        }

        // WaitCounting(n>1): consume one, decrement.
        (Action::WaitInputTicks(_w), ActionPhase::WaitCounting { remaining: r }) if r > 1 => {
            SchedulerTransition {
                new_state: SchedulerState {
                    phase: ActionPhase::WaitCounting { remaining: r - 1 },
                    state_version: sv,
                    ..*state
                },
                effects: EffectPlan::none(),
            }
        }

        // WaitCounting(1): consume last, advance.
        (Action::WaitInputTicks(_w), ActionPhase::WaitCounting { remaining: 1 }) => {
            advance_to_next(state, config, sv, EffectPlan::none())
        }

        // A presentation wait is armed by one input callback and advances only
        // on committed presentations.
        (Action::WaitPresentations(wait), ActionPhase::WaitingForInput) => SchedulerTransition {
            new_state: SchedulerState {
                phase: ActionPhase::WaitingPresentations {
                    remaining: wait.count,
                },
                state_version: sv,
                ..*state
            },
            effects: EffectPlan::none(),
        },
        (Action::WaitPresentations(_), ActionPhase::WaitingPresentations { .. }) => {
            SchedulerTransition {
                new_state: SchedulerState {
                    state_version: sv,
                    ..*state
                },
                effects: EffectPlan::none(),
            }
        }

        // set_menu_key: plan one owned-key write, advance on commit.
        _ => return None,
    })
}

/// Reduce a menu key, or decline so the next family may answer.
fn reduce_menu_key(
    state: &SchedulerState,
    config: &SchedulerConfig<'_>,
    action: &Action,
    sv: u64,
) -> Option<SchedulerTransition> {
    Some(match (action, state.phase) {
        (Action::SetMenuKey(s), ActionPhase::WaitingForInput) => {
            let effects = EffectPlan {
                write_key: Some((s.key, if s.value != 0 { 1 } else { 0 })),
                ..EffectPlan::none()
            };
            advance_to_next(state, config, sv, effects)
        }

        // tap Hold(n>1): plan held value for this update; commit to Hold(n-1).
        (Action::TapMenuKey(t), ActionPhase::TapHolding { remaining }) if remaining > 1 => {
            let effects = EffectPlan {
                write_key: Some((t.key, if t.value != 0 { 1 } else { 0 })),
                held_repeat_key: Some(t.key),
                ..EffectPlan::none()
            };
            SchedulerTransition {
                new_state: SchedulerState {
                    phase: ActionPhase::TapHolding {
                        remaining: remaining.saturating_sub(1),
                    },
                    state_version: sv,
                    ..*state
                },
                effects,
            }
        }

        // tap Hold(1): plan held value for this update; commit to ReleasePending.
        (Action::TapMenuKey(t), ActionPhase::TapHolding { remaining: 1 }) => {
            let effects = EffectPlan {
                write_key: Some((t.key, if t.value != 0 { 1 } else { 0 })),
                held_repeat_key: Some(t.key),
                ..EffectPlan::none()
            };
            SchedulerTransition {
                new_state: SchedulerState {
                    phase: ActionPhase::TapReleasePending,
                    state_version: sv,
                    ..*state
                },
                effects,
            }
        }

        // Initial tap entry from WaitingForInput.
        (Action::TapMenuKey(t), ActionPhase::WaitingForInput) => {
            if t.hold == 1 {
                // Hold(1) → ReleasePending in the same callback.
                let effects = EffectPlan {
                    write_key: Some((t.key, if t.value != 0 { 1 } else { 0 })),
                    ..EffectPlan::none()
                };
                SchedulerTransition {
                    new_state: SchedulerState {
                        phase: ActionPhase::TapReleasePending,
                        state_version: sv,
                        ..*state
                    },
                    effects,
                }
            } else {
                // hold > 1 → TapHolding(hold-1)
                let effects = EffectPlan {
                    write_key: Some((t.key, if t.value != 0 { 1 } else { 0 })),
                    ..EffectPlan::none()
                };
                SchedulerTransition {
                    new_state: SchedulerState {
                        phase: ActionPhase::TapHolding {
                            remaining: t.hold.saturating_sub(1),
                        },
                        state_version: sv,
                        ..*state
                    },
                    effects,
                }
            }
        }

        // ReleasePending: plan release before this update; commit to Settle(m) or next action.
        (Action::TapMenuKey(t), ActionPhase::TapReleasePending) => {
            let effects = EffectPlan {
                release_key: Some(t.key),
                ..EffectPlan::none()
            };
            if t.settle == 0 {
                advance_to_next(state, config, sv, effects)
            } else {
                SchedulerTransition {
                    new_state: SchedulerState {
                        phase: ActionPhase::TapSettling {
                            remaining: t.settle,
                        },
                        state_version: sv,
                        ..*state
                    },
                    effects,
                }
            }
        }

        // Settle(1): consume last, advance.
        (Action::TapMenuKey(_t), ActionPhase::TapSettling { remaining: 1 }) => {
            advance_to_next(state, config, sv, EffectPlan::none())
        }
        // Settle(n>1): consume one, decrement.
        (Action::TapMenuKey(_t), ActionPhase::TapSettling { remaining }) => SchedulerTransition {
            new_state: SchedulerState {
                phase: ActionPhase::TapSettling {
                    remaining: remaining.saturating_sub(1),
                },
                state_version: sv,
                ..*state
            },
            effects: EffectPlan::none(),
        },
        _ => return None,
    })
}

/// Reduce a player key, or decline so the next family may answer.
fn reduce_player_key(
    state: &SchedulerState,
    config: &SchedulerConfig<'_>,
    action: &Action,
    sv: u64,
) -> Option<SchedulerTransition> {
    Some(match (action, state.phase) {
        (Action::SetPlayerKey(s), ActionPhase::WaitingForInput) => {
            let effects = EffectPlan {
                write_player_key: Some((s.key, u8::from(s.value != 0))),
                ..EffectPlan::none()
            };
            advance_to_next(state, config, sv, effects)
        }
        (Action::TapPlayerKey(t), ActionPhase::WaitingForInput) => SchedulerTransition {
            new_state: SchedulerState {
                phase: if t.hold == 1 {
                    ActionPhase::TapReleasePending
                } else {
                    ActionPhase::TapHolding {
                        remaining: t.hold.saturating_sub(1),
                    }
                },
                state_version: sv,
                ..*state
            },
            effects: EffectPlan {
                write_player_key: Some((t.key, u8::from(t.value != 0))),
                ..EffectPlan::none()
            },
        },
        (Action::TapPlayerKey(t), ActionPhase::TapHolding { remaining }) => SchedulerTransition {
            new_state: SchedulerState {
                phase: if remaining == 1 {
                    ActionPhase::TapReleasePending
                } else {
                    ActionPhase::TapHolding {
                        remaining: remaining.saturating_sub(1),
                    }
                },
                state_version: sv,
                ..*state
            },
            effects: EffectPlan {
                write_player_key: Some((t.key, u8::from(t.value != 0))),
                ..EffectPlan::none()
            },
        },
        (Action::TapPlayerKey(t), ActionPhase::TapReleasePending) => {
            let effects = EffectPlan {
                release_player_key: Some(t.key),
                ..EffectPlan::none()
            };
            if t.settle == 0 {
                advance_to_next(state, config, sv, effects)
            } else {
                SchedulerTransition {
                    new_state: SchedulerState {
                        phase: ActionPhase::TapSettling {
                            remaining: t.settle,
                        },
                        state_version: sv,
                        ..*state
                    },
                    effects,
                }
            }
        }
        (Action::TapPlayerKey(_), ActionPhase::TapSettling { remaining: 1 }) => {
            advance_to_next(state, config, sv, EffectPlan::none())
        }
        (Action::TapPlayerKey(_), ActionPhase::TapSettling { remaining }) => SchedulerTransition {
            new_state: SchedulerState {
                phase: ActionPhase::TapSettling {
                    remaining: remaining.saturating_sub(1),
                },
                state_version: sv,
                ..*state
            },
            effects: EffectPlan::none(),
        },
        _ => return None,
    })
}

/// Reduce a navigation request, or decline so the next family may answer.
fn reduce_navigation(
    state: &SchedulerState,
    action: &Action,
    sv: u64,
) -> Option<SchedulerTransition> {
    Some(match (action, state.phase) {
        (Action::NavigateToContact(navigation), ActionPhase::WaitingForInput) => {
            SchedulerTransition {
                new_state: SchedulerState {
                    phase: ActionPhase::Navigating {
                        remaining: navigation.max_ticks.saturating_sub(1),
                    },
                    state_version: sv,
                    ..*state
                },
                effects: EffectPlan::none(),
            }
        }
        (Action::NavigateToPlanet(navigation), ActionPhase::WaitingForInput) => {
            SchedulerTransition {
                new_state: SchedulerState {
                    phase: ActionPhase::Navigating {
                        remaining: navigation.max_ticks.saturating_sub(1),
                    },
                    state_version: sv,
                    ..*state
                },
                effects: EffectPlan::none(),
            }
        }
        (Action::NavigateToPlanet(_), ActionPhase::Navigating { remaining: 0 })
        | (Action::NavigateToContact(_), ActionPhase::Navigating { remaining: 0 })
        | (Action::NavigateToMoon(_), ActionPhase::Navigating { remaining: 0 })
        | (Action::NavigateToOrbit(_), ActionPhase::Navigating { remaining: 0 })
        | (Action::UseDevice(_), ActionPhase::Navigating { remaining: 0 })
        | (Action::CollectEnergyNode(_), ActionPhase::Navigating { remaining: 0 })
        | (Action::FlyToStar(_), ActionPhase::Navigating { remaining: 0 })
        | (Action::NavigateToHyperspace(_), ActionPhase::Navigating { remaining: 0 })
        | (Action::TurnFlagship(_), ActionPhase::Navigating { remaining: 0 })
        | (Action::CollectPlanetMinerals(_), ActionPhase::Navigating { remaining: 0 })
        | (Action::FightSamatra(_), ActionPhase::Navigating { remaining: 0 })
        | (Action::FightEncounter(_), ActionPhase::Navigating { remaining: 0 })
        | (Action::FightBattle(_), ActionPhase::Navigating { remaining: 0 }) => {
            SchedulerTransition {
                new_state: SchedulerState {
                    terminal: Some(TerminalOutcome::SemanticMismatch),
                    state_version: sv,
                    ..*state
                },
                effects: EffectPlan::none(),
            }
        }
        (Action::NavigateToPlanet(_), ActionPhase::Navigating { remaining })
        | (Action::NavigateToContact(_), ActionPhase::Navigating { remaining })
        | (Action::NavigateToMoon(_), ActionPhase::Navigating { remaining })
        | (Action::NavigateToOrbit(_), ActionPhase::Navigating { remaining })
        | (Action::UseDevice(_), ActionPhase::Navigating { remaining })
        | (Action::CollectEnergyNode(_), ActionPhase::Navigating { remaining })
        | (Action::FlyToStar(_), ActionPhase::Navigating { remaining })
        | (Action::NavigateToHyperspace(_), ActionPhase::Navigating { remaining })
        | (Action::TurnFlagship(_), ActionPhase::Navigating { remaining })
        | (Action::CollectPlanetMinerals(_), ActionPhase::Navigating { remaining })
        | (Action::FightSamatra(_), ActionPhase::Navigating { remaining })
        | (Action::FightEncounter(_), ActionPhase::Navigating { remaining })
        | (Action::FightBattle(_), ActionPhase::Navigating { remaining }) => SchedulerTransition {
            new_state: SchedulerState {
                phase: ActionPhase::Navigating {
                    remaining: remaining - 1,
                },
                state_version: sv,
                ..*state
            },
            effects: EffectPlan::none(),
        },
        (Action::NavigateToOrbit(navigation), ActionPhase::WaitingForInput) => {
            SchedulerTransition {
                new_state: SchedulerState {
                    phase: ActionPhase::Navigating {
                        remaining: navigation.max_ticks.saturating_sub(1),
                    },
                    state_version: sv,
                    ..*state
                },
                effects: EffectPlan::none(),
            }
        }
        (Action::TurnFlagship(turn), ActionPhase::WaitingForInput) => SchedulerTransition {
            new_state: SchedulerState {
                phase: ActionPhase::Navigating {
                    remaining: turn.max_ticks.saturating_sub(1),
                },
                state_version: sv,
                ..*state
            },
            effects: EffectPlan::none(),
        },
        (Action::CollectPlanetMinerals(collect), ActionPhase::WaitingForInput) => {
            SchedulerTransition {
                new_state: SchedulerState {
                    phase: ActionPhase::Navigating {
                        remaining: collect.max_ticks.saturating_sub(1),
                    },
                    state_version: sv,
                    ..*state
                },
                effects: EffectPlan::none(),
            }
        }
        (Action::FightSamatra(fight), ActionPhase::WaitingForInput) => SchedulerTransition {
            new_state: SchedulerState {
                phase: ActionPhase::Navigating {
                    remaining: fight.max_ticks.saturating_sub(1),
                },
                state_version: sv,
                ..*state
            },
            effects: EffectPlan::none(),
        },
        (
            Action::FightBattle(fight) | Action::FightEncounter(fight),
            ActionPhase::WaitingForInput,
        ) => SchedulerTransition {
            new_state: SchedulerState {
                phase: ActionPhase::Navigating {
                    remaining: fight.max_ticks.saturating_sub(1),
                },
                state_version: sv,
                ..*state
            },
            effects: EffectPlan::none(),
        },
        (Action::UseDevice(step), ActionPhase::WaitingForInput) => SchedulerTransition {
            new_state: SchedulerState {
                phase: ActionPhase::Navigating {
                    remaining: step.max_ticks.saturating_sub(1),
                },
                state_version: sv,
                ..*state
            },
            effects: EffectPlan::none(),
        },
        (Action::CollectEnergyNode(step), ActionPhase::WaitingForInput) => SchedulerTransition {
            new_state: SchedulerState {
                phase: ActionPhase::Navigating {
                    remaining: step.max_ticks.saturating_sub(1),
                },
                state_version: sv,
                ..*state
            },
            effects: EffectPlan::none(),
        },
        (Action::FlyToStar(flight), ActionPhase::WaitingForInput) => SchedulerTransition {
            new_state: SchedulerState {
                phase: ActionPhase::Navigating {
                    remaining: flight.max_ticks.saturating_sub(1),
                },
                state_version: sv,
                ..*state
            },
            effects: EffectPlan::none(),
        },
        (Action::NavigateToHyperspace(navigation), ActionPhase::WaitingForInput) => {
            SchedulerTransition {
                new_state: SchedulerState {
                    phase: ActionPhase::Navigating {
                        remaining: navigation.max_ticks.saturating_sub(1),
                    },
                    state_version: sv,
                    ..*state
                },
                effects: EffectPlan::none(),
            }
        }
        (Action::NavigateToMoon(navigation), ActionPhase::WaitingForInput) => SchedulerTransition {
            new_state: SchedulerState {
                phase: ActionPhase::Navigating {
                    remaining: navigation.max_ticks.saturating_sub(1),
                },
                state_version: sv,
                ..*state
            },
            effects: EffectPlan::none(),
        },
        _ => return None,
    })
}

/// Reduce a wait on an observed condition, or decline so the next family may answer.
fn reduce_condition_wait(
    state: &SchedulerState,
    action: &Action,
    sv: u64,
) -> Option<SchedulerTransition> {
    Some(match (action, state.phase) {
        (Action::SelectPlanetMenu(select), ActionPhase::WaitingForInput) => SchedulerTransition {
            new_state: SchedulerState {
                phase: ActionPhase::WaitingCondition {
                    remaining: select.max_ticks.saturating_sub(1),
                },
                state_version: sv,
                ..*state
            },
            effects: EffectPlan::none(),
        },
        (Action::PickBattleShip(pick), ActionPhase::WaitingForInput) => SchedulerTransition {
            new_state: SchedulerState {
                phase: ActionPhase::WaitingCondition {
                    remaining: pick.max_ticks.saturating_sub(1),
                },
                state_version: sv,
                ..*state
            },
            effects: EffectPlan::none(),
        },
        (Action::PickBattleShip(_), ActionPhase::WaitingCondition { remaining: 0 }) => {
            SchedulerTransition {
                new_state: SchedulerState {
                    terminal: Some(TerminalOutcome::SemanticMismatch),
                    state_version: sv,
                    ..*state
                },
                effects: EffectPlan::none(),
            }
        }
        (Action::PickBattleShip(_), ActionPhase::WaitingCondition { remaining }) => {
            SchedulerTransition {
                new_state: SchedulerState {
                    phase: ActionPhase::WaitingCondition {
                        remaining: remaining - 1,
                    },
                    state_version: sv,
                    ..*state
                },
                effects: EffectPlan::none(),
            }
        }
        (Action::SelectPlanetMenu(_), ActionPhase::WaitingCondition { remaining: 0 }) => {
            SchedulerTransition {
                new_state: SchedulerState {
                    terminal: Some(TerminalOutcome::SemanticMismatch),
                    state_version: sv,
                    ..*state
                },
                effects: EffectPlan::none(),
            }
        }
        (Action::SelectPlanetMenu(_), ActionPhase::WaitingCondition { remaining }) => {
            SchedulerTransition {
                new_state: SchedulerState {
                    phase: ActionPhase::WaitingCondition {
                        remaining: remaining - 1,
                    },
                    state_version: sv,
                    ..*state
                },
                effects: EffectPlan::none(),
            }
        }
        (Action::WaitForPlanetSideStart(wait), ActionPhase::WaitingForInput) => {
            SchedulerTransition {
                new_state: SchedulerState {
                    phase: ActionPhase::WaitingCondition {
                        remaining: wait.max_ticks.saturating_sub(1),
                    },
                    state_version: sv,
                    ..*state
                },
                effects: EffectPlan::none(),
            }
        }
        (Action::WaitForPlanetSideStart(_), ActionPhase::WaitingCondition { remaining: 0 }) => {
            SchedulerTransition {
                new_state: SchedulerState {
                    terminal: Some(TerminalOutcome::SemanticMismatch),
                    state_version: sv,
                    ..*state
                },
                effects: EffectPlan::none(),
            }
        }
        (Action::WaitForPlanetSideStart(_), ActionPhase::WaitingCondition { remaining }) => {
            SchedulerTransition {
                new_state: SchedulerState {
                    phase: ActionPhase::WaitingCondition {
                        remaining: remaining - 1,
                    },
                    state_version: sv,
                    ..*state
                },
                effects: EffectPlan::none(),
            }
        }
        (Action::WaitForPlanetSideEnd(wait), ActionPhase::WaitingForInput) => SchedulerTransition {
            new_state: SchedulerState {
                phase: ActionPhase::WaitingCondition {
                    remaining: wait.max_ticks.saturating_sub(1),
                },
                state_version: sv,
                ..*state
            },
            effects: EffectPlan::none(),
        },
        (Action::WaitForPlanetSideEnd(_), ActionPhase::WaitingCondition { remaining: 0 }) => {
            SchedulerTransition {
                new_state: SchedulerState {
                    terminal: Some(TerminalOutcome::SemanticMismatch),
                    state_version: sv,
                    ..*state
                },
                effects: EffectPlan::none(),
            }
        }
        (Action::WaitForPlanetSideEnd(_), ActionPhase::WaitingCondition { remaining }) => {
            SchedulerTransition {
                new_state: SchedulerState {
                    phase: ActionPhase::WaitingCondition {
                        remaining: remaining - 1,
                    },
                    state_version: sv,
                    ..*state
                },
                effects: EffectPlan::none(),
            }
        }
        (Action::WaitForBattleFrames(wait), ActionPhase::WaitingForInput) => SchedulerTransition {
            new_state: SchedulerState {
                phase: ActionPhase::WaitingCondition {
                    remaining: wait.max_ticks.saturating_sub(1),
                },
                state_version: sv,
                ..*state
            },
            effects: EffectPlan::none(),
        },
        (Action::WaitForBattleFrames(_), ActionPhase::WaitingCondition { remaining: 0 }) => {
            SchedulerTransition {
                new_state: SchedulerState {
                    terminal: Some(TerminalOutcome::SemanticMismatch),
                    state_version: sv,
                    ..*state
                },
                effects: EffectPlan::none(),
            }
        }
        (Action::WaitForBattleFrames(_), ActionPhase::WaitingCondition { remaining }) => {
            SchedulerTransition {
                new_state: SchedulerState {
                    phase: ActionPhase::WaitingCondition {
                        remaining: remaining.saturating_sub(1),
                    },
                    state_version: sv,
                    ..*state
                },
                effects: EffectPlan::none(),
            }
        }
        (Action::WaitForBattleShipLoss(wait), ActionPhase::WaitingForInput) => {
            SchedulerTransition {
                new_state: SchedulerState {
                    phase: ActionPhase::WaitingCondition {
                        remaining: wait.max_ticks.saturating_sub(1),
                    },
                    state_version: sv,
                    ..*state
                },
                effects: EffectPlan::none(),
            }
        }
        (Action::WaitForBattleShipLoss(_), ActionPhase::WaitingCondition { remaining: 0 }) => {
            SchedulerTransition {
                new_state: SchedulerState {
                    terminal: Some(TerminalOutcome::SemanticMismatch),
                    state_version: sv,
                    ..*state
                },
                effects: EffectPlan::none(),
            }
        }
        (Action::WaitForBattleShipLoss(_), ActionPhase::WaitingCondition { remaining }) => {
            SchedulerTransition {
                new_state: SchedulerState {
                    phase: ActionPhase::WaitingCondition {
                        remaining: remaining.saturating_sub(1),
                    },
                    state_version: sv,
                    ..*state
                },
                effects: EffectPlan::none(),
            }
        }
        (Action::WaitForDispatch(wait), ActionPhase::WaitingForInput) => SchedulerTransition {
            new_state: SchedulerState {
                phase: ActionPhase::WaitingCondition {
                    remaining: wait.max_ticks.saturating_sub(1),
                },
                state_version: sv,
                ..*state
            },
            effects: EffectPlan::none(),
        },
        (Action::WaitForDispatch(_), ActionPhase::WaitingCondition { remaining: 0 }) => {
            SchedulerTransition {
                new_state: SchedulerState {
                    terminal: Some(TerminalOutcome::SemanticMismatch),
                    state_version: sv,
                    ..*state
                },
                effects: EffectPlan::none(),
            }
        }
        (Action::WaitForDispatch(_), ActionPhase::WaitingCondition { remaining }) => {
            SchedulerTransition {
                new_state: SchedulerState {
                    phase: ActionPhase::WaitingCondition {
                        remaining: remaining.saturating_sub(1),
                    },
                    state_version: sv,
                    ..*state
                },
                effects: EffectPlan::none(),
            }
        }
        (Action::WaitForState(wait), ActionPhase::WaitingForInput) => SchedulerTransition {
            new_state: SchedulerState {
                phase: ActionPhase::WaitingCondition {
                    remaining: wait.max_ticks.saturating_sub(1),
                },
                state_version: sv,
                ..*state
            },
            effects: EffectPlan::none(),
        },
        (Action::WaitForState(_), ActionPhase::WaitingCondition { remaining: 0 }) => {
            SchedulerTransition {
                new_state: SchedulerState {
                    terminal: Some(TerminalOutcome::SemanticMismatch),
                    state_version: sv,
                    ..*state
                },
                effects: EffectPlan::none(),
            }
        }
        (Action::WaitForState(_), ActionPhase::WaitingCondition { remaining }) => {
            SchedulerTransition {
                new_state: SchedulerState {
                    phase: ActionPhase::WaitingCondition {
                        remaining: remaining.saturating_sub(1),
                    },
                    state_version: sv,
                    ..*state
                },
                effects: EffectPlan::none(),
            }
        }
        (
            Action::SelectCommunicationResponse(
                crate::automation::script::SelectCommunicationResponseStep { max_ticks, .. },
            )
            | Action::SelectResponse(crate::automation::script::SelectResponseStep {
                max_ticks, ..
            }),
            ActionPhase::WaitingForInput,
        ) => SchedulerTransition {
            new_state: SchedulerState {
                phase: ActionPhase::WaitingCondition {
                    remaining: max_ticks.saturating_sub(1),
                },
                state_version: sv,
                ..*state
            },
            effects: EffectPlan::none(),
        },
        (
            Action::SelectCommunicationResponse(_) | Action::SelectResponse(_),
            ActionPhase::WaitingCondition { remaining: 0 },
        ) => SchedulerTransition {
            new_state: SchedulerState {
                terminal: Some(TerminalOutcome::SemanticMismatch),
                state_version: sv,
                ..*state
            },
            effects: EffectPlan::none(),
        },
        (
            Action::SelectCommunicationResponse(_) | Action::SelectResponse(_),
            ActionPhase::WaitingCondition { remaining },
        ) => SchedulerTransition {
            new_state: SchedulerState {
                phase: ActionPhase::WaitingCondition {
                    remaining: remaining.saturating_sub(1),
                },
                state_version: sv,
                ..*state
            },
            effects: EffectPlan::none(),
        },
        (Action::WaitForCommunicationResponses(wait), ActionPhase::WaitingForInput) => {
            SchedulerTransition {
                new_state: SchedulerState {
                    phase: ActionPhase::WaitingCondition {
                        remaining: wait.max_ticks.saturating_sub(1),
                    },
                    state_version: sv,
                    ..*state
                },
                effects: EffectPlan::none(),
            }
        }
        (
            Action::WaitForCommunicationResponses(_),
            ActionPhase::WaitingCondition { remaining: 0 },
        ) => SchedulerTransition {
            new_state: SchedulerState {
                terminal: Some(TerminalOutcome::SemanticMismatch),
                state_version: sv,
                ..*state
            },
            effects: EffectPlan::none(),
        },
        (Action::WaitForCommunicationResponses(_), ActionPhase::WaitingCondition { remaining }) => {
            SchedulerTransition {
                new_state: SchedulerState {
                    phase: ActionPhase::WaitingCondition {
                        remaining: remaining.saturating_sub(1),
                    },
                    state_version: sv,
                    ..*state
                },
                effects: EffectPlan::none(),
            }
        }
        (Action::WaitForCommunicationEnd(wait), ActionPhase::WaitingForInput) => {
            SchedulerTransition {
                new_state: SchedulerState {
                    phase: ActionPhase::WaitingCondition {
                        remaining: wait.max_ticks.saturating_sub(1),
                    },
                    state_version: sv,
                    ..*state
                },
                effects: EffectPlan::none(),
            }
        }
        (Action::WaitForCommunicationEnd(_), ActionPhase::WaitingCondition { remaining: 0 }) => {
            SchedulerTransition {
                new_state: SchedulerState {
                    terminal: Some(TerminalOutcome::SemanticMismatch),
                    state_version: sv,
                    ..*state
                },
                effects: EffectPlan::none(),
            }
        }
        (Action::WaitForCommunicationEnd(_), ActionPhase::WaitingCondition { remaining }) => {
            SchedulerTransition {
                new_state: SchedulerState {
                    phase: ActionPhase::WaitingCondition {
                        remaining: remaining.saturating_sub(1),
                    },
                    state_version: sv,
                    ..*state
                },
                effects: EffectPlan::none(),
            }
        }
        (Action::WaitForCommunicationReplay(wait), ActionPhase::WaitingForInput) => {
            SchedulerTransition {
                new_state: SchedulerState {
                    phase: ActionPhase::WaitingCondition {
                        remaining: wait.max_ticks.saturating_sub(1),
                    },
                    state_version: sv,
                    ..*state
                },
                effects: EffectPlan::none(),
            }
        }
        (Action::WaitForCommunicationReplay(_), ActionPhase::WaitingCondition { remaining: 0 }) => {
            SchedulerTransition {
                new_state: SchedulerState {
                    terminal: Some(TerminalOutcome::SemanticMismatch),
                    state_version: sv,
                    ..*state
                },
                effects: EffectPlan::none(),
            }
        }
        (Action::WaitForCommunicationReplay(_), ActionPhase::WaitingCondition { remaining }) => {
            SchedulerTransition {
                new_state: SchedulerState {
                    phase: ActionPhase::WaitingCondition {
                        remaining: remaining.saturating_sub(1),
                    },
                    state_version: sv,
                    ..*state
                },
                effects: EffectPlan::none(),
            }
        }
        // capture: arm once, commit to WaitingCapture.
        _ => return None,
    })
}

/// Reduce a capture, assertion, fixture, or finish, or decline so the next family may answer.
fn reduce_capture_and_assertion(
    state: &SchedulerState,
    config: &SchedulerConfig<'_>,
    action: &Action,
    sv: u64,
) -> Option<SchedulerTransition> {
    Some(match (action, state.phase) {
        (Action::Capture(_), ActionPhase::WaitingForInput) => {
            let new_gen = match state.capture_generation.next() {
                Some(g) => g,
                None => {
                    return Some(SchedulerTransition {
                        new_state: SchedulerState {
                            terminal: Some(TerminalOutcome::CaptureGenerationOverflow),
                            ..*state
                        },
                        effects: EffectPlan::none(),
                    });
                }
            };
            let effects = EffectPlan {
                arm_capture: Some(new_gen),
                ..EffectPlan::none()
            };
            SchedulerTransition {
                new_state: SchedulerState {
                    phase: ActionPhase::WaitingCapture {
                        generation: new_gen,
                    },
                    state_version: sv,
                    capture_generation: new_gen,
                    ..*state
                },
                effects,
            }
        }

        // WaitingCapture: input does not advance.
        (Action::Capture(_), ActionPhase::WaitingCapture { .. }) => SchedulerTransition {
            new_state: SchedulerState {
                state_version: sv,
                ..*state
            },
            effects: EffectPlan::none(),
        },

        // WaitingSemantic: input does not advance.
        (Action::AssertMainMenuTransition(_), ActionPhase::WaitingSemantic) => {
            SchedulerTransition {
                new_state: SchedulerState {
                    state_version: sv,
                    ..*state
                },
                effects: EffectPlan::none(),
            }
        }

        // AssertMainMenuTransition: enter WaitingSemantic on first input callback.
        (Action::AssertMainMenuTransition(_), ActionPhase::WaitingForInput) => {
            // Consume this input callback to enter WaitingSemantic.
            // Do NOT advance step_index — we wait for a MenuTransition event.
            SchedulerTransition {
                new_state: SchedulerState {
                    phase: ActionPhase::WaitingSemantic,
                    state_version: sv,
                    ..*state
                },
                effects: EffectPlan::none(),
            }
        }

        // AssertActivity: immediate advance (no callback needed).
        (Action::AssertActivity(_), ActionPhase::WaitingForInput) => {
            advance_to_next(state, config, sv, EffectPlan::none())
        }

        // AssertScene is validated by the runtime coordinator before the
        // reducer advances it. Keeping it callback-bound prevents an earlier
        // action from chaining past the semantic check.
        (Action::AssertScene(_), ActionPhase::WaitingForInput)
        | (Action::AssertMode(_), ActionPhase::WaitingForInput)
        | (Action::AssertDispatch(_), ActionPhase::WaitingForInput)
        | (Action::AssertGameOptions(_), ActionPhase::WaitingForInput)
        | (Action::AssertCommunicationResponses(_), ActionPhase::WaitingForInput)
        | (Action::AssertBattleFrames(_), ActionPhase::WaitingForInput)
        | (Action::RecordHyperspacePosition(_), ActionPhase::WaitingForInput)
        | (Action::AssertHyperspaceDisplacement(_), ActionPhase::WaitingForInput)
        | (Action::AssertHyperspaceStationary(_), ActionPhase::WaitingForInput)
        | (Action::RecordSavedHyperspacePosition(_), ActionPhase::WaitingForInput)
        | (Action::AssertHyperspacePositionRestored(_), ActionPhase::WaitingForInput)
        | (Action::AssertShipResources(_), ActionPhase::WaitingForInput)
        | (Action::AssertState(_), ActionPhase::WaitingForInput)
        | (Action::AssertShipElement(_), ActionPhase::WaitingForInput)
        | (Action::AssertPlanetSideCollisions(_), ActionPhase::WaitingForInput) => {
            advance_to_next(state, config, sv, EffectPlan::none())
        }

        // SetupPlanetSideCollisionFixture: the coordinator queues the
        // one-shot fixture before the reducer advances it.
        (Action::SetupPlanetSideCollisionFixture(_), ActionPhase::WaitingForInput) => {
            advance_to_next(state, config, sv, EffectPlan::none())
        }

        // Finish: terminal success. Consumes the input callback.
        (Action::Finish, ActionPhase::WaitingForInput) => SchedulerTransition {
            new_state: SchedulerState {
                terminal: Some(TerminalOutcome::FinishComplete),
                state_version: sv,
                ..*state
            },
            effects: EffectPlan::none(),
        },

        // Any event/action pair not handled above is impossible for this phase.
        _ => return None,
    })
}

/// Process a committed-present callback.
fn reduce_committed_present(
    state: &SchedulerState,
    config: &SchedulerConfig<'_>,
    generation: CaptureGeneration,
    action: &Action,
    sv: u64,
) -> SchedulerTransition {
    if let (Action::WaitPresentations(wait), ActionPhase::WaitingPresentations { remaining }) =
        (action, state.phase)
    {
        return match remaining {
            1 => advance_to_next(
                state,
                config,
                sv,
                EffectPlan {
                    complete_presentation_wait: Some(wait.count),
                    ..EffectPlan::none()
                },
            ),
            2.. => SchedulerTransition {
                new_state: SchedulerState {
                    phase: ActionPhase::WaitingPresentations {
                        remaining: remaining - 1,
                    },
                    state_version: sv,
                    ..*state
                },
                effects: EffectPlan::none(),
            },
            0 => SchedulerTransition {
                new_state: SchedulerState {
                    terminal: Some(TerminalOutcome::SemanticMismatch),
                    state_version: sv,
                    ..*state
                },
                effects: EffectPlan::none(),
            },
        };
    }

    // Only WaitingCapture consumes other presents.
    if let ActionPhase::WaitingCapture {
        generation: pending,
    } = state.phase
    {
        if let Action::Capture(_) = action {
            match validate_capture_completion(pending, generation, false) {
                CaptureValidation::Match => {
                    let effects = EffectPlan {
                        complete_capture: Some(generation),
                        ..EffectPlan::none()
                    };
                    // Advance to next action.
                    advance_to_next(state, config, sv, effects)
                }
                CaptureValidation::Zero
                | CaptureValidation::Stale
                | CaptureValidation::Duplicate
                | CaptureValidation::Future => {
                    // Active mismatch is terminal.
                    SchedulerTransition {
                        new_state: SchedulerState {
                            terminal: Some(TerminalOutcome::CaptureMismatch),
                            state_version: sv,
                            ..*state
                        },
                        effects: EffectPlan::none(),
                    }
                }
            }
        } else {
            SchedulerTransition {
                new_state: SchedulerState {
                    state_version: sv,
                    ..*state
                },
                effects: EffectPlan::none(),
            }
        }
    } else {
        // Present in non-capture state: no effect.
        SchedulerTransition {
            new_state: SchedulerState {
                state_version: sv,
                ..*state
            },
            effects: EffectPlan::none(),
        }
    }
}

/// Process a typed menu-transition event.
fn reduce_menu_transition(
    state: &SchedulerState,
    config: &SchedulerConfig<'_>,
    from: u8,
    to: u8,
    sv: u64,
) -> SchedulerTransition {
    if state.phase != ActionPhase::WaitingSemantic {
        return SchedulerTransition {
            new_state: SchedulerState {
                state_version: sv,
                ..*state
            },
            effects: EffectPlan::none(),
        };
    }

    // Look up the expected transition for this step.
    // Transitions are indexed by their position in the script, not by step_index.
    // We need to find the transition that corresponds to the current action.
    // Since AssertMainMenuTransition actions map to transitions in order,
    // we count how many transitions precede this step.
    let mut transition_idx = 0;
    for (i, action) in config.actions.iter().enumerate() {
        if i >= state.step_index {
            break;
        }
        if matches!(action, Action::AssertMainMenuTransition(_)) {
            transition_idx += 1;
        }
    }

    if let Some(expected) = config.transitions.get(transition_idx) {
        if expected.from.as_u8() == from && expected.to.as_u8() == to {
            advance_to_next(state, config, sv, EffectPlan::none())
        } else {
            SchedulerTransition {
                new_state: SchedulerState {
                    terminal: Some(TerminalOutcome::SemanticMismatch),
                    state_version: sv,
                    ..*state
                },
                effects: EffectPlan::none(),
            }
        }
    } else {
        SchedulerTransition {
            new_state: SchedulerState {
                terminal: Some(TerminalOutcome::SemanticMismatch),
                state_version: sv,
                ..*state
            },
            effects: EffectPlan::none(),
        }
    }
}

/// Advance to the next action in the script, handling zero-callback chaining.
fn advance_to_next(
    state: &SchedulerState,
    config: &SchedulerConfig<'_>,
    sv: u64,
    initial_effects: EffectPlan,
) -> SchedulerTransition {
    let next_index = state.step_index + 1;

    // Only zero-tick waits may chain. Assertions must remain current until
    // the coordinator verifies runtime state before their input callback.
    if let Some(next_action) = config.actions.get(next_index) {
        match next_action {
            Action::WaitInputTicks(w) if w.count == 0 => {
                // Chain: emit initial_effects, advance past this step too.
                let combined = initial_effects;
                let chained_state = SchedulerState {
                    step_index: next_index,
                    phase: ActionPhase::WaitingForInput,
                    state_version: sv,
                    ..*state
                };
                return advance_to_next(&chained_state, config, sv, combined);
            }
            _ => {}
        }
    }

    SchedulerTransition {
        new_state: SchedulerState {
            step_index: next_index,
            phase: ActionPhase::WaitingForInput,
            state_version: sv,
            ..*state
        },
        effects: initial_effects,
    }
}

// ===========================================================================
//  Unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automation::script::{
        ActivityAssertion, CaptureStep, MainMenuTransitionDto, NavigateToHyperspaceStep,
        NavigateToMoonStep, SetMenuKeyStep, TapMenuKeyStep, TapPlayerKeyStep,
        WaitForBattleFramesStep, WaitForCommunicationEndStep, WaitForCommunicationReplayStep,
        WaitForDispatchStep, WaitForMainMenuReadyStep, WaitInputTicksStep, WaitPresentationsStep,
    };
    use crate::mainloop::restart_menu::types::RestartMenuItem;

    fn explore_script() -> crate::automation::ValidatedScript {
        let path = std::path::Path::new("explore-planet-v1.json");
        crate::automation::validate_script(
            crate::automation::parse_script(
                include_bytes!("../../scripts/explore-planet-v1.json"),
                path,
            )
            .unwrap(),
            path,
        )
        .unwrap()
    }

    /// #213: the explore scenario must establish hyperspace through a real new
    /// game and real flight, never through Down (which focuses Load Game on
    /// the restart menu) or a start scene.
    #[test]
    fn explore_selects_new_game_without_moving_menu_focus() {
        let script = explore_script();
        assert_eq!(script.start_scene(), None);
        let config = cfg(script.steps());
        let mut state = SchedulerState::initial();
        let mut observed = Vec::new();
        for tick in 1..=60 {
            if tick == 31 {
                state = scheduler_reduce(&state, &config, SchedulerEvent::MainMenuReady).new_state;
            }
            let transition = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
            if let Some(key) = transition.effects.write_key {
                observed.push((tick, key.0, key.1));
            }
            if let Some(key) = transition.effects.release_key {
                observed.push((tick, key, 0));
            }
            state = transition.new_state;
        }
        assert_eq!(
            observed,
            vec![
                (31, MenuKey::Select, 1),
                (32, MenuKey::Select, 1),
                (33, MenuKey::Select, 1),
                (34, MenuKey::Select, 0),
                (45, MenuKey::Cancel, 1),
                (46, MenuKey::Cancel, 1),
                (47, MenuKey::Cancel, 1),
                (48, MenuKey::Cancel, 0),
            ],
            "new-game selection and intro-skip input must follow restart readiness"
        );
    }

    /// The explore scenario claims navigation, orbit and scan, so it must fly
    /// from the opening interplanetary state to Mars, enter orbit, run the
    /// auto scan from the planet menu and assert orbit mode afterwards. It
    /// never leaves the solar system, so it asserts no hyperspace.
    #[test]
    fn explore_flies_to_mars_orbit_and_scans_with_real_menu_input() {
        use crate::automation::mode::GameMode;
        use crate::automation::script::{
            ModeAssertion, NavigateToOrbitStep, NavigateToPlanetStep, PlanetMenuPhaseName,
            SelectPlanetMenuStep,
        };
        let script = explore_script();
        let steps = script.steps();
        let position = |predicate: &dyn Fn(&Action) -> bool| {
            steps.iter().position(predicate).expect("step present")
        };
        let phase = |wanted: PlanetMenuPhaseName| {
            position(&|step| {
                matches!(step, Action::SelectPlanetMenu(SelectPlanetMenuStep { phase, .. })
                    if *phase == wanted)
            })
        };
        let interplanetary = position(&|step| {
            matches!(
                step,
                Action::AssertActivity(ActivityAssertion {
                    mask: 255,
                    equals: 4
                })
            )
        });
        let to_mars = position(&|step| {
            matches!(
                step,
                Action::NavigateToPlanet(NavigateToPlanetStep { planet: 3, .. })
            )
        });
        let to_orbit = position(&|step| {
            matches!(
                step,
                Action::NavigateToOrbit(NavigateToOrbitStep { planet: 3, .. })
            )
        });
        let orbit_menu = phase(PlanetMenuPhaseName::Orbit);
        let auto_scan = phase(PlanetMenuPhaseName::AutoScan);
        let dispatch = phase(PlanetMenuPhaseName::Dispatch);
        let orbit_modes: Vec<usize> = steps
            .iter()
            .enumerate()
            .filter_map(|(index, step)| {
                matches!(
                    step,
                    Action::AssertMode(ModeAssertion {
                        mode: GameMode::Orbit
                    })
                )
                .then_some(index)
            })
            .collect();
        assert!(interplanetary < to_mars);
        assert!(to_mars < to_orbit);
        assert!(to_orbit < orbit_menu);
        assert!(orbit_menu < auto_scan);
        assert!(auto_scan < dispatch);
        assert_eq!(
            orbit_modes.len(),
            2,
            "orbit asserted at the menu and after the scan"
        );
        assert!(orbit_menu < orbit_modes[0] && orbit_modes[0] < auto_scan);
        assert!(dispatch < orbit_modes[1]);
        let captures: Vec<&str> = steps
            .iter()
            .filter_map(|step| match step {
                Action::Capture(capture) => Some(capture.label.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(
            captures,
            ["mars-reached", "mars-orbit-menu", "mars-orbit-scanned"]
        );
        assert!(!steps.iter().any(|step| matches!(
            step,
            Action::NavigateToHyperspace(_)
                | Action::TapMenuKey(TapMenuKeyStep {
                    key: MenuKey::Down,
                    ..
                })
                | Action::SetMenuKey(SetMenuKeyStep {
                    key: MenuKey::Down,
                    ..
                })
        )));
    }

    #[test]
    fn hyperspace_departure_counts_down_and_fails_when_exhausted() {
        let actions = [
            Action::NavigateToHyperspace(NavigateToHyperspaceStep {
                max_ticks: 2,
                encounter_response: None,
            }),
            Action::Finish,
        ];
        let config = cfg(&actions);
        let started = scheduler_reduce(
            &SchedulerState::initial(),
            &config,
            SchedulerEvent::AdmittedInput,
        )
        .new_state;
        assert_eq!(started.phase, ActionPhase::Navigating { remaining: 1 });
        let reached = scheduler_reduce(&started, &config, SchedulerEvent::NavigationReached);
        assert_eq!(reached.new_state.step_index, 1);
        let last = scheduler_reduce(&started, &config, SchedulerEvent::AdmittedInput).new_state;
        assert_eq!(last.phase, ActionPhase::Navigating { remaining: 0 });
        let exhausted = scheduler_reduce(&last, &config, SchedulerEvent::AdmittedInput);
        assert_eq!(
            exhausted.new_state.terminal,
            Some(TerminalOutcome::SemanticMismatch)
        );
    }

    fn cfg(actions: &[Action]) -> SchedulerConfig<'_> {
        SchedulerConfig {
            actions,
            transitions: &[],
        }
    }

    fn cfg_with_trans<'a>(
        actions: &'a [Action],
        transitions: &'a [MainMenuTransition],
    ) -> SchedulerConfig<'a> {
        SchedulerConfig {
            actions,
            transitions,
        }
    }

    // --- Zero-wait chaining ---

    #[test]
    fn wait_zero_advances_immediately() {
        let actions = [
            Action::WaitInputTicks(WaitInputTicksStep { count: 0 }),
            Action::Finish,
        ];
        let config = cfg(&actions);
        let state = SchedulerState::initial();
        let t = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
        assert_eq!(t.new_state.step_index, 1);
        assert!(t.effects.is_empty());
    }

    // --- wait_input_ticks(n>0) consumes exactly n callbacks ---

    #[test]
    fn wait_n_consumes_n_callbacks() {
        let actions = [
            Action::WaitInputTicks(WaitInputTicksStep { count: 2 }),
            Action::Finish,
        ];
        let config = cfg(&actions);
        let mut state = SchedulerState::initial();

        // First input: consume one, count becomes 1
        let t = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
        assert_eq!(t.new_state.step_index, 0); // still on step 0
        state = t.new_state;

        // Second input: count reaches 0, advance
        let t = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
        assert_eq!(t.new_state.step_index, 1); // advanced
    }
    #[test]
    fn wait_presentations_counts_only_committed_presents() {
        let actions = [
            Action::WaitPresentations(WaitPresentationsStep { count: 2 }),
            Action::Finish,
        ];
        let config = cfg(&actions);
        let mut state = SchedulerState::initial();

        let armed = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
        assert_eq!(
            armed.new_state.phase,
            ActionPhase::WaitingPresentations { remaining: 2 }
        );
        state = armed.new_state;

        let unrelated_input = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
        assert_eq!(
            unrelated_input.new_state.phase,
            ActionPhase::WaitingPresentations { remaining: 2 }
        );
        state = unrelated_input.new_state;

        let first = scheduler_reduce(
            &state,
            &config,
            SchedulerEvent::CommittedPresent {
                generation: CaptureGeneration(0),
            },
        );
        assert_eq!(
            first.new_state.phase,
            ActionPhase::WaitingPresentations { remaining: 1 }
        );
        assert!(first.effects.is_empty());
        state = first.new_state;

        let second = scheduler_reduce(
            &state,
            &config,
            SchedulerEvent::CommittedPresent {
                generation: CaptureGeneration(0),
            },
        );
        assert_eq!(second.new_state.step_index, 1);
        assert_eq!(second.effects.complete_presentation_wait, Some(2));
    }

    #[test]
    fn wait_presentations_honors_native_acceptance_boundaries() {
        for count in [1, 120, 300] {
            let actions = [
                Action::WaitPresentations(WaitPresentationsStep { count }),
                Action::Finish,
            ];
            let config = cfg(&actions);
            let mut state = scheduler_reduce(
                &SchedulerState::initial(),
                &config,
                SchedulerEvent::AdmittedInput,
            )
            .new_state;

            for presentation in 1..count {
                let transition = scheduler_reduce(
                    &state,
                    &config,
                    SchedulerEvent::CommittedPresent {
                        generation: CaptureGeneration(presentation),
                    },
                );
                assert_eq!(transition.new_state.step_index, 0);
                assert_eq!(
                    transition.new_state.phase,
                    ActionPhase::WaitingPresentations {
                        remaining: count - presentation,
                    }
                );
                assert_eq!(transition.effects.complete_presentation_wait, None);
                state = transition.new_state;
            }

            let final_presentation = scheduler_reduce(
                &state,
                &config,
                SchedulerEvent::CommittedPresent {
                    generation: CaptureGeneration(count),
                },
            );
            assert_eq!(final_presentation.new_state.step_index, 1);
            assert_eq!(
                final_presentation.effects.complete_presentation_wait,
                Some(count)
            );
        }
    }

    // --- set_menu_key ---

    #[test]
    fn set_menu_key_writes_and_advances() {
        let actions = [
            Action::SetMenuKey(SetMenuKeyStep {
                key: MenuKey::Down,
                value: 1,
            }),
            Action::Finish,
        ];
        let config = cfg(&actions);
        let state = SchedulerState::initial();
        let t = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
        assert_eq!(t.new_state.step_index, 1);
        assert_eq!(t.effects.write_key, Some((MenuKey::Down, 1)));
    }

    #[test]
    fn set_menu_key_normalizes_nonzero() {
        let actions = [
            Action::SetMenuKey(SetMenuKeyStep {
                key: MenuKey::Up,
                value: 5,
            }),
            Action::Finish,
        ];
        let config = cfg(&actions);
        let state = SchedulerState::initial();
        let t = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
        assert_eq!(t.effects.write_key, Some((MenuKey::Up, 1)));
    }

    // --- Tap edge: hold=1, settle=0 ---

    #[test]
    fn tap_hold1_settle0() {
        let actions = [
            Action::TapMenuKey(TapMenuKeyStep {
                key: MenuKey::Down,
                value: 1,
                hold: 1,

                settle: 0,
            }),
            Action::Finish,
        ];
        let config = cfg(&actions);
        let mut state = SchedulerState::initial();

        // Input 1: Hold(1) → plan held value, commit to ReleasePending
        let t = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
        assert_eq!(t.effects.write_key, Some((MenuKey::Down, 1)));
        assert_eq!(t.new_state.phase, ActionPhase::TapReleasePending);
        state = t.new_state;

        // Input 2: ReleasePending → plan release, advance (settle=0)
        let t = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
        assert_eq!(t.effects.release_key, Some(MenuKey::Down));
        assert_eq!(t.new_state.step_index, 1);
    }

    #[test]
    fn tap_player_key_holds_and_releases_gameplay_control() {
        let actions = [
            Action::TapPlayerKey(TapPlayerKeyStep {
                key: PlayerKey::Thrust,
                value: 1,
                hold: 2,
                settle: 0,
            }),
            Action::Finish,
        ];
        let config = cfg(&actions);
        let mut state = SchedulerState::initial();

        let first = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
        assert_eq!(first.effects.write_player_key, Some((PlayerKey::Thrust, 1)));
        assert_eq!(
            first.new_state.phase,
            ActionPhase::TapHolding { remaining: 1 }
        );
        state = first.new_state;

        let second = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
        assert_eq!(
            second.effects.write_player_key,
            Some((PlayerKey::Thrust, 1))
        );
        assert_eq!(second.new_state.phase, ActionPhase::TapReleasePending);
        state = second.new_state;

        let release = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
        assert_eq!(release.effects.release_player_key, Some(PlayerKey::Thrust));
        assert_eq!(release.new_state.step_index, 1);
    }
    /// Issue #214: a menu tap reaches `menu[]` and a player tap reaches the
    /// ship's `key[]`. Neither routes through the other's effect for the
    /// whole hold, release and settle.
    #[test]
    fn menu_and_player_taps_never_cross_input_classes() {
        let drive = |action: Action| {
            let actions = [action, Action::Finish];
            let config = cfg(&actions);
            let mut state = SchedulerState::initial();
            let mut effects = Vec::new();
            while state.step_index == 0 {
                let t = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
                effects.push(t.effects);
                state = t.new_state;
            }
            effects
        };
        let menu = drive(Action::TapMenuKey(TapMenuKeyStep {
            key: MenuKey::Up,
            value: 1,
            hold: 3,
            settle: 2,
        }));
        assert!(menu.iter().any(|e| e.write_key == Some((MenuKey::Up, 1))));
        assert!(menu
            .iter()
            .all(|e| e.write_player_key.is_none() && e.release_player_key.is_none()));

        let player = drive(Action::TapPlayerKey(TapPlayerKeyStep {
            key: PlayerKey::Thrust,
            value: 1,
            hold: 3,
            settle: 2,
        }));
        let writes = player
            .iter()
            .filter(|e| e.write_player_key == Some((PlayerKey::Thrust, 1)))
            .count();
        assert_eq!(writes, 3, "thrust is written on every held update");
        assert!(player
            .iter()
            .any(|e| e.release_player_key == Some(PlayerKey::Thrust)));
        assert!(player
            .iter()
            .all(|e| e.write_key.is_none() && e.release_key.is_none()));
    }

    #[test]
    fn ship_resources_assertion_advances_without_input_effects() {
        use crate::automation::script::ShipResourcesAssertion;
        let actions = [
            Action::AssertShipResources(ShipResourcesAssertion {
                fuel_on_board: Some(1000),
                res_units: Some(0),
            }),
            Action::Finish,
        ];
        let config = cfg(&actions);
        let state = SchedulerState::initial();
        let t = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
        assert_eq!(t.new_state.step_index, 1);
        assert_eq!(t.effects, EffectPlan::none());
    }

    fn state_predicate() -> crate::automation::campaign_state::StatePredicate {
        serde_json::from_str(r#"{"all":[{"key":"sis:fuel","op":"eq","value":896}]}"#).unwrap()
    }

    #[test]
    fn state_assertion_advances_without_input_effects() {
        let actions = [Action::AssertState(state_predicate()), Action::Finish];
        let config = cfg(&actions);
        let t = scheduler_reduce(
            &SchedulerState::initial(),
            &config,
            SchedulerEvent::AdmittedInput,
        );
        assert_eq!(t.new_state.step_index, 1);
        assert_eq!(t.effects, EffectPlan::none());
    }

    #[test]
    fn state_wait_advances_when_reached_and_expires_at_its_bound() {
        use crate::automation::script::WaitForStateStep;
        let actions = [
            Action::WaitForState(WaitForStateStep {
                predicate: state_predicate(),
                max_ticks: 2,
            }),
            Action::Finish,
        ];
        let config = cfg(&actions);
        let first = scheduler_reduce(
            &SchedulerState::initial(),
            &config,
            SchedulerEvent::AdmittedInput,
        );
        assert_eq!(
            first.new_state.phase,
            ActionPhase::WaitingCondition { remaining: 1 }
        );
        let reached = scheduler_reduce(&first.new_state, &config, SchedulerEvent::ConditionReached);
        assert_eq!(reached.new_state.step_index, 1);

        let second = scheduler_reduce(&first.new_state, &config, SchedulerEvent::AdmittedInput);
        assert_eq!(
            second.new_state.phase,
            ActionPhase::WaitingCondition { remaining: 0 }
        );
        let expired = scheduler_reduce(&second.new_state, &config, SchedulerEvent::AdmittedInput);
        assert_eq!(
            expired.new_state.terminal,
            Some(TerminalOutcome::SemanticMismatch)
        );
    }

    #[test]
    fn ship_element_assertion_advances_without_input_effects() {
        use crate::automation::script::ShipElementAssertion;
        use crate::planet_side::cargo::ElementCategory;
        let actions = [
            Action::AssertShipElement(ShipElementAssertion {
                category: ElementCategory::Radioactive,
                minimum: 1,
            }),
            Action::Finish,
        ];
        let config = cfg(&actions);
        let state = SchedulerState::initial();
        let t = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
        assert_eq!(t.new_state.step_index, 1);
        assert_eq!(t.effects, EffectPlan::none());
    }

    #[test]
    fn position_record_and_displacement_assertion_advance_without_input_effects() {
        use crate::automation::script::{
            HyperspaceDisplacementAssertion, RecordHyperspacePositionStep,
        };
        let actions = [
            Action::RecordHyperspacePosition(RecordHyperspacePositionStep {}),
            Action::AssertHyperspaceDisplacement(HyperspaceDisplacementAssertion {
                minimum_logical_units: 16,
            }),
            Action::Finish,
        ];
        let config = cfg(&actions);
        let mut state = SchedulerState::initial();
        for expected in [1, 2] {
            let t = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
            assert_eq!(t.new_state.step_index, expected);
            assert_eq!(t.effects, EffectPlan::none());
            state = t.new_state;
        }
    }

    #[test]
    fn saved_position_steps_advance_without_input_effects() {
        use crate::automation::script::{
            HyperspacePositionRestoredAssertion, RecordSavedHyperspacePositionStep,
        };
        let actions = [
            Action::RecordSavedHyperspacePosition(RecordSavedHyperspacePositionStep {}),
            Action::AssertHyperspacePositionRestored(HyperspacePositionRestoredAssertion {}),
            Action::Finish,
        ];
        let config = cfg(&actions);
        let mut state = SchedulerState::initial();
        for expected in [1, 2] {
            let t = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
            assert_eq!(t.new_state.step_index, expected);
            assert_eq!(t.effects, EffectPlan::none());
            state = t.new_state;
        }
    }

    /// The production input loop turns a key that stays down into a repeat
    /// pulse once half a second of wall-clock time has passed. A scripted tap
    /// is one press, so every update after the first one only holds the key
    /// and must say so, otherwise a slow host turns one tap into several.
    #[test]
    fn tap_marks_only_the_updates_after_the_press_as_held() {
        let actions = [
            Action::TapMenuKey(TapMenuKeyStep {
                key: MenuKey::Down,
                value: 1,
                hold: 3,
                settle: 0,
            }),
            Action::Finish,
        ];
        let config = cfg(&actions);
        let mut state = SchedulerState::initial();
        let mut held = Vec::new();
        for _ in 0..4 {
            let t = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
            held.push((t.effects.write_key.is_some(), t.effects.held_repeat_key));
            state = t.new_state;
        }
        assert_eq!(
            held,
            [
                (true, None),
                (true, Some(MenuKey::Down)),
                (true, Some(MenuKey::Down)),
                (false, None),
            ]
        );
    }

    #[test]
    fn single_update_tap_never_marks_a_held_repeat() {
        let actions = [
            Action::TapMenuKey(TapMenuKeyStep {
                key: MenuKey::Select,
                value: 1,
                hold: 1,
                settle: 0,
            }),
            Action::Finish,
        ];
        let config = cfg(&actions);
        let press = scheduler_reduce(
            &SchedulerState::initial(),
            &config,
            SchedulerEvent::AdmittedInput,
        );
        assert_eq!(press.effects.held_repeat_key, None);
    }

    // --- Tap edge: hold=1, settle=1 (no infinite loop) ---

    #[test]
    fn tap_hold1_settle1() {
        let actions = [
            Action::TapMenuKey(TapMenuKeyStep {
                key: MenuKey::Down,
                value: 1,
                hold: 1,
                settle: 1,
            }),
            Action::Finish,
        ];
        let config = cfg(&actions);
        let mut state = SchedulerState::initial();

        // Input 1: Hold(1) → ReleasePending
        let t = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
        assert_eq!(t.new_state.phase, ActionPhase::TapReleasePending);
        state = t.new_state;

        // Input 2: ReleasePending → Settle(1), release
        let t = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
        assert_eq!(t.effects.release_key, Some(MenuKey::Down));
        assert_eq!(t.new_state.phase, ActionPhase::TapSettling { remaining: 1 });
        state = t.new_state;

        // Input 3: Settle(1) → advance
        let t = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
        assert_eq!(t.new_state.step_index, 1);
    }

    // --- Tap settle consumes exactly M callbacks ---

    #[test]
    fn tap_settle_consumes_exactly_m_callbacks() {
        let actions = [
            Action::TapMenuKey(TapMenuKeyStep {
                key: MenuKey::Down,
                value: 1,
                hold: 1,
                settle: 3,
            }),
            Action::Finish,
        ];
        let config = cfg(&actions);
        let mut state = SchedulerState::initial();

        // Input 1: Hold(1) → ReleasePending
        state = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput).new_state;

        // Input 2: ReleasePending → Settle(3)
        state = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput).new_state;
        assert_eq!(state.phase, ActionPhase::TapSettling { remaining: 3 });

        // Inputs 3-5: Settle(3) → Settle(2) → Settle(1) → advance
        assert_eq!(state.step_index, 0);
        state = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput).new_state;
        assert_eq!(state.phase, ActionPhase::TapSettling { remaining: 2 });
        state = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput).new_state;
        assert_eq!(state.phase, ActionPhase::TapSettling { remaining: 1 });
        state = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput).new_state;
        // After consuming exactly 3 settle callbacks, advance
        assert_eq!(state.step_index, 1);
    }

    // --- Tap edge: hold=3, settle=2 ---

    #[test]
    fn tap_hold3_settle2() {
        let actions = [
            Action::TapMenuKey(TapMenuKeyStep {
                key: MenuKey::Down,
                value: 1,
                hold: 3,
                settle: 2,
            }),
            Action::Finish,
        ];
        let config = cfg(&actions);
        let mut state = SchedulerState::initial();

        // Input 1: enter TapHolding(2), plan held
        let t = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
        assert_eq!(t.effects.write_key, Some((MenuKey::Down, 1)));
        assert_eq!(t.new_state.phase, ActionPhase::TapHolding { remaining: 2 });
        state = t.new_state;

        // Input 2: Hold(2) → Hold(1), plan held
        let t = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
        assert_eq!(t.effects.write_key, Some((MenuKey::Down, 1)));
        assert_eq!(t.new_state.phase, ActionPhase::TapHolding { remaining: 1 });
        state = t.new_state;

        // Input 3: Hold(1) → ReleasePending, plan held
        let t = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
        assert_eq!(t.effects.write_key, Some((MenuKey::Down, 1)));
        assert_eq!(t.new_state.phase, ActionPhase::TapReleasePending);
        state = t.new_state;

        // Input 4: ReleasePending → Settle(2), plan release
        let t = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
        assert_eq!(t.effects.release_key, Some(MenuKey::Down));
        assert_eq!(t.new_state.phase, ActionPhase::TapSettling { remaining: 2 });
        state = t.new_state;

        // Input 5: Settle(2) → Settle(1)
        let t = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
        assert!(t.effects.is_empty());
        assert_eq!(t.new_state.phase, ActionPhase::TapSettling { remaining: 1 });
        state = t.new_state;

        // Input 6: Settle(1) → advance
        let t = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
        assert!(t.effects.is_empty());
        assert_eq!(t.new_state.step_index, 1);
    }

    // --- Tap preserves unowned controls ---

    #[test]
    fn tap_only_writes_owned_key() {
        let actions = [
            Action::TapMenuKey(TapMenuKeyStep {
                key: MenuKey::Down,
                value: 1,
                hold: 1,
                settle: 0,
            }),
            Action::Finish,
        ];
        let config = cfg(&actions);
        let state = SchedulerState::initial();
        let t = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
        // Only Down is written, not Up/Left/Right/etc.
        assert_eq!(t.effects.write_key, Some((MenuKey::Down, 1)));
        assert!(t.effects.write_key != Some((MenuKey::Up, 1)));
    }

    // --- Capture arm + completion ---

    #[test]
    fn capture_arms_then_completes_on_matching_present() {
        let actions = [
            Action::Capture(CaptureStep {
                label: "shot".into(),
                expect_change: false,
            }),
            Action::Finish,
        ];
        let config = cfg(&actions);
        let mut state = SchedulerState::initial();

        // Input: arm capture, enter WaitingCapture
        let t = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
        assert!(t.effects.arm_capture.is_some());
        let gen = t.effects.arm_capture.unwrap();
        assert!(gen.is_armed());
        assert_eq!(
            t.new_state.phase,
            ActionPhase::WaitingCapture { generation: gen }
        );
        state = t.new_state;

        // Present with matching generation: complete and advance
        let t = scheduler_reduce(
            &state,
            &config,
            SchedulerEvent::CommittedPresent { generation: gen },
        );
        assert_eq!(t.effects.complete_capture, Some(gen));
        assert_eq!(t.new_state.step_index, 1);
    }

    #[test]
    fn capture_rejects_stale_generation() {
        // Arm two captures to get generation 2, then try to complete with
        // a nonzero stale generation (1 < 2) — exercises the Stale branch.
        let actions = [
            Action::Capture(CaptureStep {
                label: "shot1".into(),
                expect_change: false,
            }),
            Action::Capture(CaptureStep {
                label: "shot2".into(),
                expect_change: false,
            }),
            Action::Finish,
        ];
        let config = cfg(&actions);
        let mut state = SchedulerState::initial();

        // First capture arms gen=1, then complete it.
        let t = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
        let gen1 = t.effects.arm_capture.unwrap();
        state = t.new_state;

        // Complete first capture.
        let t = scheduler_reduce(
            &state,
            &config,
            SchedulerEvent::CommittedPresent { generation: gen1 },
        );
        state = t.new_state;

        // Second capture arms gen=2.
        let t = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
        let gen2 = t.effects.arm_capture.unwrap();
        assert_eq!(gen2.0, 2);
        state = t.new_state;

        // Present with nonzero stale generation (1 < 2): terminal mismatch.
        let stale = CaptureGeneration(1);
        let t = scheduler_reduce(
            &state,
            &config,
            SchedulerEvent::CommittedPresent { generation: stale },
        );
        assert_eq!(t.new_state.terminal, Some(TerminalOutcome::CaptureMismatch));
    }

    #[test]
    fn capture_rejects_duplicate_generation() {
        // validate_capture_completion: already_completed=true → Duplicate.
        assert_eq!(
            validate_capture_completion(CaptureGeneration(5), CaptureGeneration(5), true),
            CaptureValidation::Duplicate
        );
        // Match when not already completed.
        assert_eq!(
            validate_capture_completion(CaptureGeneration(5), CaptureGeneration(5), false),
            CaptureValidation::Match
        );
    }

    #[test]
    fn capture_rejects_zero_generation() {
        let actions = [
            Action::Capture(CaptureStep {
                label: "shot".into(),
                expect_change: false,
            }),
            Action::Finish,
        ];
        let config = cfg(&actions);
        let mut state = SchedulerState::initial();

        let t = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
        state = t.new_state;

        let t = scheduler_reduce(
            &state,
            &config,
            SchedulerEvent::CommittedPresent {
                generation: CaptureGeneration(0),
            },
        );
        assert_eq!(t.new_state.terminal, Some(TerminalOutcome::CaptureMismatch));
    }

    #[test]
    fn capture_rejects_future_generation() {
        let actions = [
            Action::Capture(CaptureStep {
                label: "shot".into(),
                expect_change: false,
            }),
            Action::Finish,
        ];
        let config = cfg(&actions);
        let mut state = SchedulerState::initial();

        let t = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
        let gen = t.effects.arm_capture.unwrap();
        state = t.new_state;

        let future = CaptureGeneration(gen.0 + 1);
        let t = scheduler_reduce(
            &state,
            &config,
            SchedulerEvent::CommittedPresent { generation: future },
        );
        assert_eq!(t.new_state.terminal, Some(TerminalOutcome::CaptureMismatch));
    }

    #[test]
    fn waiting_capture_input_does_not_advance() {
        let actions = [
            Action::Capture(CaptureStep {
                label: "shot".into(),
                expect_change: false,
            }),
            Action::Finish,
        ];
        let config = cfg(&actions);
        let mut state = SchedulerState::initial();

        let t = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
        state = t.new_state;

        // Extra input while waiting: no advance
        let t = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
        assert_eq!(t.new_state.step_index, 0);
        assert!(t.effects.is_empty());
    }

    #[test]
    fn main_menu_readiness_ignores_input_until_production_ready_event() {
        let actions = [
            Action::WaitForMainMenuReady(WaitForMainMenuReadyStep {}),
            Action::Finish,
        ];
        let config = cfg(&actions);
        let state = SchedulerState::initial();

        let input = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
        assert_eq!(input.new_state.step_index, 0);
        assert_eq!(input.new_state.phase, ActionPhase::WaitingForInput);
        assert_eq!(input.new_state.terminal, None);

        let ready = scheduler_reduce(&input.new_state, &config, SchedulerEvent::MainMenuReady);
        assert_eq!(ready.new_state.step_index, 1);
        assert_eq!(ready.new_state.terminal, None);
    }

    // --- Semantic transition ---

    #[test]
    fn semantic_match_advances() {
        let trans = MainMenuTransition::new(RestartMenuItem::NewGame, RestartMenuItem::LoadGame);
        let actions = [
            Action::AssertMainMenuTransition(MainMenuTransitionDto {
                from: "NewGame".into(),
                to: "LoadGame".into(),
            }),
            Action::Finish,
        ];
        let config = cfg_with_trans(&actions, std::slice::from_ref(&trans));

        // Enter WaitingSemantic
        let mut state = SchedulerState::initial();
        state.phase = ActionPhase::WaitingSemantic;

        let t = scheduler_reduce(
            &state,
            &config,
            SchedulerEvent::MenuTransition {
                from: RestartMenuItem::NewGame.as_u8(),
                to: RestartMenuItem::LoadGame.as_u8(),
            },
        );
        assert_eq!(t.new_state.step_index, 1);
    }

    #[test]
    fn semantic_same_destination_wrong_source_is_terminal() {
        let trans = MainMenuTransition::new(RestartMenuItem::NewGame, RestartMenuItem::LoadGame);
        let actions = [
            Action::AssertMainMenuTransition(MainMenuTransitionDto {
                from: "NewGame".into(),
                to: "LoadGame".into(),
            }),
            Action::Finish,
        ];
        let config = cfg_with_trans(&actions, std::slice::from_ref(&trans));
        let mut state = SchedulerState::initial();
        state.phase = ActionPhase::WaitingSemantic;

        let t = scheduler_reduce(
            &state,
            &config,
            SchedulerEvent::MenuTransition {
                from: RestartMenuItem::SuperMelee.as_u8(),
                to: RestartMenuItem::LoadGame.as_u8(),
            },
        );
        assert_eq!(
            t.new_state.terminal,
            Some(TerminalOutcome::SemanticMismatch)
        );
    }

    #[test]
    fn semantic_mismatch_is_terminal() {
        let trans = MainMenuTransition::new(RestartMenuItem::NewGame, RestartMenuItem::LoadGame);
        let actions = [
            Action::AssertMainMenuTransition(MainMenuTransitionDto {
                from: "NewGame".into(),
                to: "LoadGame".into(),
            }),
            Action::Finish,
        ];
        let config = cfg_with_trans(&actions, std::slice::from_ref(&trans));

        let mut state = SchedulerState::initial();
        state.phase = ActionPhase::WaitingSemantic;

        let t = scheduler_reduce(
            &state,
            &config,
            SchedulerEvent::MenuTransition {
                from: RestartMenuItem::NewGame.as_u8(),
                to: RestartMenuItem::Quit.as_u8(),
            },
        );
        assert_eq!(
            t.new_state.terminal,
            Some(TerminalOutcome::SemanticMismatch)
        );
    }

    // --- Terminal absorbing ---

    #[test]
    fn terminal_is_absorbing() {
        let actions = [Action::Finish];
        let config = cfg(&actions);
        let mut state = SchedulerState::initial();
        state.terminal = Some(TerminalOutcome::FinishComplete);

        let t = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
        assert_eq!(t.new_state, state);
        assert!(t.effects.is_empty());
    }

    // --- Finish is terminal success ---

    #[test]
    fn finish_is_terminal_success() {
        let actions = [Action::Finish];
        let config = cfg(&actions);
        let state = SchedulerState::initial();
        let t = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
        assert_eq!(t.new_state.terminal, Some(TerminalOutcome::FinishComplete));
    }

    #[test]
    fn out_of_range_step_is_invalid_state() {
        let actions = [Action::Finish];
        let config = cfg(&actions);
        let mut state = SchedulerState::initial();
        state.step_index = actions.len();
        let transition = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
        assert_eq!(
            transition.new_state.terminal,
            Some(TerminalOutcome::InvalidState)
        );
        assert!(transition.effects.is_empty());
    }

    #[test]
    fn finish_outside_waiting_for_input_is_invalid_state() {
        let actions = [Action::Finish];
        let config = cfg(&actions);
        let mut state = SchedulerState::initial();
        state.phase = ActionPhase::TapHolding { remaining: 1 };
        let transition = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
        assert_eq!(
            transition.new_state.terminal,
            Some(TerminalOutcome::InvalidState)
        );
    }

    #[test]
    fn action_phase_mismatch_is_invalid_and_absorbing() {
        let actions = [Action::WaitInputTicks(WaitInputTicksStep { count: 1 })];
        let config = cfg(&actions);
        let mut state = SchedulerState::initial();
        state.phase = ActionPhase::WaitingCapture {
            generation: CaptureGeneration(1),
        };
        let transition = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
        assert_eq!(
            transition.new_state.terminal,
            Some(TerminalOutcome::InvalidState)
        );
        let absorbed = scheduler_reduce(
            &transition.new_state,
            &config,
            SchedulerEvent::CommittedPresent {
                generation: CaptureGeneration(0),
            },
        );
        assert_eq!(absorbed.new_state, transition.new_state);
        assert!(absorbed.effects.is_empty());
    }
    // --- Multiple inputs without presents ---

    #[test]
    fn multiple_inputs_without_presents() {
        let actions = [
            Action::WaitInputTicks(WaitInputTicksStep { count: 3 }),
            Action::Finish,
        ];
        let config = cfg(&actions);
        let mut state = SchedulerState::initial();

        for i in 0..3 {
            let t = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
            state = t.new_state;
            if i < 2 {
                assert_eq!(state.step_index, 0, "should stay on step 0 for input {i}");
            } else {
                assert_eq!(state.step_index, 1, "should advance on input {i}");
            }
        }
    }

    // --- assert_activity advances immediately ---

    #[test]
    fn capture_completion_leaves_activity_assertion_for_runtime_verification() {
        let actions = [
            Action::Capture(crate::automation::script::CaptureStep {
                label: "before-activity".into(),
                expect_change: false,
            }),
            Action::AssertActivity(ActivityAssertion { mask: 1, equals: 0 }),
            Action::Finish,
        ];
        let config = cfg(&actions);
        let armed = scheduler_reduce(
            &SchedulerState::initial(),
            &config,
            SchedulerEvent::AdmittedInput,
        );
        let completed = scheduler_reduce(
            &armed.new_state,
            &config,
            SchedulerEvent::CommittedPresent {
                generation: armed.new_state.capture_generation,
            },
        );
        assert_eq!(completed.new_state.step_index, 1);
        assert_eq!(completed.new_state.phase, ActionPhase::WaitingForInput);
        assert_eq!(
            completed.effects.complete_capture,
            Some(CaptureGeneration(1))
        );
    }

    #[test]
    fn assert_activity_advances() {
        let actions = [
            Action::AssertActivity(ActivityAssertion { mask: 1, equals: 0 }),
            Action::Finish,
        ];
        let config = cfg(&actions);
        let state = SchedulerState::initial();
        let t = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
        assert_eq!(t.new_state.step_index, 1);
    }

    // --- setup_planet_side_collision_fixture: one callback, one step ---

    #[test]
    fn setup_planet_side_collision_fixture_advances_exactly_one_step_from_waiting_for_input() {
        // The fixture action is callback-bound and one-shot: a single admitted
        // input moves it to the next action; a second input naturally falls
        // through to the following action, never re-queues the fixture.
        let actions = [
            Action::SetupPlanetSideCollisionFixture(
                crate::automation::script::SetupPlanetSideCollisionFixtureStep {},
            ),
            Action::Finish,
        ];
        let config = cfg(&actions);
        let state = SchedulerState::initial();
        assert_eq!(state.phase, ActionPhase::WaitingForInput);
        let t = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
        assert_eq!(
            t.new_state.step_index, 1,
            "exactly one callback advances the fixture step"
        );
        assert_eq!(t.new_state.phase, ActionPhase::WaitingForInput);
        assert_eq!(t.new_state.terminal, None);
        assert!(t.effects.is_empty());
    }

    // --- State version increments ---

    #[test]
    fn state_version_increments() {
        let actions = [Action::Finish];
        let config = cfg(&actions);
        let state = SchedulerState::initial();
        assert_eq!(state.state_version, 0);
        let t = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
        assert_eq!(t.new_state.state_version, 1);
    }

    #[test]
    fn state_version_overflow_is_terminal() {
        let actions = [Action::Finish];
        let config = cfg(&actions);
        let state = SchedulerState {
            state_version: u64::MAX,
            ..SchedulerState::initial()
        };
        let t = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
        assert_eq!(
            t.new_state.terminal,
            Some(TerminalOutcome::StateVersionOverflow)
        );
        assert!(t.effects.is_empty());
    }

    #[test]
    fn capture_generation_overflow_is_terminal() {
        let actions = [
            Action::Capture(CaptureStep {
                label: "shot".into(),
                expect_change: false,
            }),
            Action::Finish,
        ];
        let config = cfg(&actions);
        let state = SchedulerState {
            capture_generation: CaptureGeneration(u64::MAX),
            ..SchedulerState::initial()
        };
        let t = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
        assert_eq!(
            t.new_state.terminal,
            Some(TerminalOutcome::CaptureGenerationOverflow)
        );
        assert!(t.effects.is_empty());
    }

    // --- Capture generation model ---

    #[test]
    fn capture_generation_next() {
        assert_eq!(CaptureGeneration(0).next(), Some(CaptureGeneration(1)));
        assert_eq!(CaptureGeneration(5).next(), Some(CaptureGeneration(6)));
        assert_eq!(CaptureGeneration(u64::MAX).next(), None);
    }

    #[test]
    fn capture_generation_is_armed() {
        assert!(!CaptureGeneration(0).is_armed());
        assert!(CaptureGeneration(1).is_armed());
    }

    // --- Deterministic replay (REQ-DET-001) ---

    #[test]
    fn deterministic_replay_same_events_same_states() {
        let actions = [
            Action::TapMenuKey(TapMenuKeyStep {
                key: MenuKey::Down,
                value: 1,
                hold: 2,
                settle: 1,
            }),
            Action::Finish,
        ];
        let config = cfg(&actions);

        let run1: Vec<_> = {
            let mut s = SchedulerState::initial();
            let mut results = vec![s];
            for _ in 0..4 {
                let t = scheduler_reduce(&s, &config, SchedulerEvent::AdmittedInput);
                s = t.new_state;
                results.push(s);
            }
            results
        };

        let run2: Vec<_> = {
            let mut s = SchedulerState::initial();
            let mut results = vec![s];
            for _ in 0..4 {
                let t = scheduler_reduce(&s, &config, SchedulerEvent::AdmittedInput);
                s = t.new_state;
                results.push(s);
            }
            results
        };

        assert_eq!(run1, run2);
    }

    #[test]
    fn planet_side_end_advances_only_when_the_session_settles() {
        use crate::automation::script::{PlanetSideOutcomeName, WaitForPlanetSideEndStep};
        let actions = [
            Action::WaitForPlanetSideEnd(WaitForPlanetSideEndStep {
                outcome: PlanetSideOutcomeName::Returned,
                max_ticks: 3,
            }),
            Action::Finish,
        ];
        let config = cfg(&actions);

        let started = scheduler_reduce(
            &SchedulerState::initial(),
            &config,
            SchedulerEvent::AdmittedInput,
        );
        assert_eq!(
            started.new_state.phase,
            ActionPhase::WaitingCondition { remaining: 2 }
        );

        // Ticking while the session is still running must not advance the step.
        let waiting = scheduler_reduce(&started.new_state, &config, SchedulerEvent::AdmittedInput);
        assert_eq!(waiting.new_state.step_index, 0);
        assert_eq!(
            waiting.new_state.phase,
            ActionPhase::WaitingCondition { remaining: 1 }
        );

        // The coordinator reports the settled session as a reached condition.
        let reached = scheduler_reduce(
            &waiting.new_state,
            &config,
            SchedulerEvent::ConditionReached,
        );
        assert_eq!(reached.new_state.step_index, 1);
        assert_eq!(reached.new_state.terminal, None);
    }

    #[test]
    fn planet_side_end_budget_exhaustion_is_terminal() {
        use crate::automation::script::{PlanetSideOutcomeName, WaitForPlanetSideEndStep};
        let actions = [Action::WaitForPlanetSideEnd(WaitForPlanetSideEndStep {
            outcome: PlanetSideOutcomeName::Returned,
            max_ticks: 1,
        })];
        let config = cfg(&actions);

        let started = scheduler_reduce(
            &SchedulerState::initial(),
            &config,
            SchedulerEvent::AdmittedInput,
        );
        assert_eq!(
            started.new_state.phase,
            ActionPhase::WaitingCondition { remaining: 0 }
        );

        let expired = scheduler_reduce(&started.new_state, &config, SchedulerEvent::AdmittedInput);
        assert_eq!(
            expired.new_state.terminal,
            Some(TerminalOutcome::SemanticMismatch),
            "a trip that never settles must fail the proof"
        );
    }

    #[test]
    fn moon_navigation_budget_exhaustion_is_terminal() {
        let actions = [Action::NavigateToMoon(NavigateToMoonStep {
            planet: 2,
            moon: 0,
            max_ticks: 1,
        })];
        let config = cfg(&actions);
        let started = scheduler_reduce(
            &SchedulerState::initial(),
            &config,
            SchedulerEvent::AdmittedInput,
        );
        assert_eq!(
            started.new_state.phase,
            ActionPhase::Navigating { remaining: 0 }
        );

        let expired = scheduler_reduce(&started.new_state, &config, SchedulerEvent::AdmittedInput);
        assert_eq!(
            expired.new_state.terminal,
            Some(TerminalOutcome::SemanticMismatch)
        );
    }

    #[test]
    fn communication_wait_advances_only_on_condition() {
        let actions = [
            Action::WaitForCommunicationEnd(WaitForCommunicationEndStep {
                minimum_completions: 1,
                max_ticks: 3,
            }),
            Action::Finish,
        ];
        let config = cfg(&actions);
        let waiting = scheduler_reduce(
            &SchedulerState::initial(),
            &config,
            SchedulerEvent::AdmittedInput,
        );
        assert_eq!(
            waiting.new_state.phase,
            ActionPhase::WaitingCondition { remaining: 2 }
        );
        let reached = scheduler_reduce(
            &waiting.new_state,
            &config,
            SchedulerEvent::ConditionReached,
        );

        assert_eq!(reached.new_state.step_index, 1);
        assert_eq!(reached.new_state.terminal, None);
        let finished = scheduler_reduce(&reached.new_state, &config, SchedulerEvent::AdmittedInput);
        assert_eq!(
            finished.new_state.terminal,
            Some(TerminalOutcome::FinishComplete)
        );
    }

    #[test]
    fn communication_replay_wait_advances_only_on_condition() {
        let actions = [
            Action::WaitForCommunicationReplay(WaitForCommunicationReplayStep { max_ticks: 3 }),
            Action::Finish,
        ];
        let config = cfg(&actions);
        let waiting = scheduler_reduce(
            &SchedulerState::initial(),
            &config,
            SchedulerEvent::AdmittedInput,
        );
        assert_eq!(
            waiting.new_state.phase,
            ActionPhase::WaitingCondition { remaining: 2 }
        );
        let reached = scheduler_reduce(
            &waiting.new_state,
            &config,
            SchedulerEvent::ConditionReached,
        );

        assert_eq!(reached.new_state.step_index, 1);
        assert_eq!(reached.new_state.terminal, None);
    }

    #[test]
    fn communication_response_wait_holds_until_condition_and_expires_as_a_mismatch() {
        use crate::automation::script::WaitForCommunicationResponsesStep;
        let actions = [
            Action::WaitForCommunicationResponses(WaitForCommunicationResponsesStep {
                minimum: 2,
                max_ticks: 3,
            }),
            Action::Finish,
        ];
        let config = cfg(&actions);
        let mut state = scheduler_reduce(
            &SchedulerState::initial(),
            &config,
            SchedulerEvent::AdmittedInput,
        )
        .new_state;
        assert_eq!(state.phase, ActionPhase::WaitingCondition { remaining: 2 });
        for remaining in [1, 0] {
            state = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput).new_state;
            assert_eq!(state.step_index, 0, "ticks alone never satisfy the wait");
            assert_eq!(state.terminal, None);
            assert_eq!(state.phase, ActionPhase::WaitingCondition { remaining });
        }
        let expired = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
        assert_eq!(
            expired.new_state.terminal,
            Some(TerminalOutcome::SemanticMismatch)
        );

        let waiting = scheduler_reduce(
            &SchedulerState::initial(),
            &config,
            SchedulerEvent::AdmittedInput,
        );
        let reached = scheduler_reduce(
            &waiting.new_state,
            &config,
            SchedulerEvent::ConditionReached,
        );
        assert_eq!(reached.new_state.step_index, 1);
        assert_eq!(reached.new_state.terminal, None);
    }

    #[test]
    fn phrase_selection_waits_for_fresh_choices_and_expires() {
        let actions = [
            Action::SelectResponse(crate::automation::script::SelectResponseStep {
                phrase: "we_will_transfer_now".into(),
                max_ticks: 2,
            }),
            Action::Finish,
        ];
        let config = cfg(&actions);
        let waiting = scheduler_reduce(
            &SchedulerState::initial(),
            &config,
            SchedulerEvent::AdmittedInput,
        );
        assert_eq!(
            waiting.new_state.phase,
            ActionPhase::WaitingCondition { remaining: 1 }
        );
        let reached = scheduler_reduce(
            &waiting.new_state,
            &config,
            SchedulerEvent::ConditionReached,
        );
        assert_eq!(reached.new_state.step_index, 1);
        let exhausted =
            scheduler_reduce(&waiting.new_state, &config, SchedulerEvent::AdmittedInput);
        let expired =
            scheduler_reduce(&exhausted.new_state, &config, SchedulerEvent::AdmittedInput);
        assert_eq!(
            expired.new_state.terminal,
            Some(TerminalOutcome::SemanticMismatch)
        );
    }

    #[test]
    fn semantic_response_selection_advances_on_condition() {
        let actions = [
            Action::SelectCommunicationResponse(
                crate::automation::script::SelectCommunicationResponseStep {
                    index: 0,
                    max_ticks: 3,
                },
            ),
            Action::Finish,
        ];
        let config = cfg(&actions);
        let waiting = scheduler_reduce(
            &SchedulerState::initial(),
            &config,
            SchedulerEvent::AdmittedInput,
        );
        assert_eq!(
            waiting.new_state.phase,
            ActionPhase::WaitingCondition { remaining: 2 }
        );

        let reached = scheduler_reduce(
            &waiting.new_state,
            &config,
            SchedulerEvent::ConditionReached,
        );
        assert_eq!(reached.new_state.step_index, 1);
        assert_eq!(reached.new_state.terminal, None);
    }

    #[test]
    fn communication_replay_wait_expires_with_semantic_mismatch() {
        let actions = [Action::WaitForCommunicationReplay(
            WaitForCommunicationReplayStep { max_ticks: 1 },
        )];
        let config = cfg(&actions);
        let waiting = scheduler_reduce(
            &SchedulerState::initial(),
            &config,
            SchedulerEvent::AdmittedInput,
        );
        assert_eq!(
            waiting.new_state.phase,
            ActionPhase::WaitingCondition { remaining: 0 }
        );

        for event in [
            SchedulerEvent::AdmittedInput,
            SchedulerEvent::ConditionReached,
        ] {
            let expired = scheduler_reduce(&waiting.new_state, &config, event);
            assert_eq!(
                expired.new_state.terminal,
                Some(TerminalOutcome::SemanticMismatch)
            );
        }
    }

    #[test]
    fn battle_frame_wait_advances_only_when_observation_reaches_its_floor() {
        let actions = [
            Action::WaitForBattleFrames(WaitForBattleFramesStep {
                minimum: 1,
                max_ticks: 3,
            }),
            Action::Finish,
        ];
        let config = cfg(&actions);
        let waiting = scheduler_reduce(
            &SchedulerState::initial(),
            &config,
            SchedulerEvent::AdmittedInput,
        );
        assert_eq!(
            waiting.new_state.phase,
            ActionPhase::WaitingCondition { remaining: 2 }
        );

        let still_waiting =
            scheduler_reduce(&waiting.new_state, &config, SchedulerEvent::AdmittedInput);
        assert_eq!(
            still_waiting.new_state.phase,
            ActionPhase::WaitingCondition { remaining: 1 }
        );

        let reached = scheduler_reduce(
            &still_waiting.new_state,
            &config,
            SchedulerEvent::ConditionReached,
        );
        assert_eq!(reached.new_state.step_index, 1);
        assert_eq!(reached.new_state.phase, ActionPhase::WaitingForInput);
        assert_eq!(reached.new_state.terminal, None);

        let last_callback = scheduler_reduce(
            &still_waiting.new_state,
            &config,
            SchedulerEvent::AdmittedInput,
        );
        assert_eq!(
            last_callback.new_state.phase,
            ActionPhase::WaitingCondition { remaining: 0 }
        );
        let expired = scheduler_reduce(
            &last_callback.new_state,
            &config,
            SchedulerEvent::AdmittedInput,
        );
        assert_eq!(
            expired.new_state.terminal,
            Some(TerminalOutcome::SemanticMismatch)
        );
    }

    #[test]
    fn dispatch_wait_expires_with_semantic_mismatch() {
        let actions = [Action::WaitForDispatch(WaitForDispatchStep {
            encounter: 2,
            dialogue: 2,
            max_ticks: 1,
        })];
        let config = cfg(&actions);
        let waiting = scheduler_reduce(
            &SchedulerState::initial(),
            &config,
            SchedulerEvent::AdmittedInput,
        );
        assert_eq!(
            waiting.new_state.phase,
            ActionPhase::WaitingCondition { remaining: 0 }
        );
        let expired = scheduler_reduce(&waiting.new_state, &config, SchedulerEvent::AdmittedInput);
        assert_eq!(
            expired.new_state.terminal,
            Some(TerminalOutcome::SemanticMismatch)
        );
    }

    #[test]
    fn reached_events_cannot_bypass_exhausted_budgets() {
        let navigation = [Action::NavigateToMoon(NavigateToMoonStep {
            planet: 2,
            moon: 0,
            max_ticks: 1,
        })];
        let navigation_config = cfg(&navigation);
        let navigation_waiting = scheduler_reduce(
            &SchedulerState::initial(),
            &navigation_config,
            SchedulerEvent::AdmittedInput,
        );
        let late_navigation = scheduler_reduce(
            &navigation_waiting.new_state,
            &navigation_config,
            SchedulerEvent::NavigationReached,
        );
        assert_eq!(
            late_navigation.new_state.terminal,
            Some(TerminalOutcome::SemanticMismatch)
        );

        let condition = [Action::WaitForDispatch(WaitForDispatchStep {
            encounter: 2,
            dialogue: 2,
            max_ticks: 1,
        })];
        let condition_config = cfg(&condition);
        let condition_waiting = scheduler_reduce(
            &SchedulerState::initial(),
            &condition_config,
            SchedulerEvent::AdmittedInput,
        );
        let late_condition = scheduler_reduce(
            &condition_waiting.new_state,
            &condition_config,
            SchedulerEvent::ConditionReached,
        );
        assert_eq!(
            late_condition.new_state.terminal,
            Some(TerminalOutcome::SemanticMismatch)
        );
    }
}

/// Every action that spends a bounded callback budget waiting for the real game
/// to reach a state shares one contract: the budget counts down one admitted
/// input at a time, the matching "reached" event advances the script, and an
/// exhausted budget fails the proof instead of waiting forever.
#[cfg(test)]
mod budgeted_wait_tests {
    use super::*;
    use crate::automation::script::{
        CollectPlanetMineralsStep, FightBattleStep, NavigateToHyperspaceStep, NavigateToMoonStep,
        NavigateToOrbitStep, NavigateToPlanetStep, PickBattleShipStep, PlanetMenuPhaseName,
        PlanetSideOutcomeName, SelectCommunicationResponseStep, SelectPlanetMenuStep,
        TurnDirection, TurnFlagshipStep, WaitForBattleFramesStep, WaitForBattleShipLossStep,
        WaitForCommunicationEndStep, WaitForCommunicationReplayStep, WaitForDispatchStep,
        WaitForPlanetSideEndStep, WaitForPlanetSideStartStep,
    };

    use crate::planet_side::cargo::ElementCategory;

    fn navigation_actions(max_ticks: u64) -> Vec<Action> {
        vec![
            Action::NavigateToContact(crate::automation::script::NavigateToContactStep {
                race: 12,
                max_ticks,
            }),
            Action::NavigateToPlanet(NavigateToPlanetStep {
                planet: 1,
                max_ticks,
                encounter_response: None,
                patrol_policy: crate::automation::script::PatrolPolicy::Stop,
            }),
            Action::NavigateToMoon(NavigateToMoonStep {
                planet: 1,
                moon: 0,
                max_ticks,
            }),
            Action::NavigateToOrbit(NavigateToOrbitStep {
                planet: 1,
                max_ticks,
                stop_at_encounter: false,
                encounter_response: None,
                patrol_policy: crate::automation::script::PatrolPolicy::Stop,
            }),
            Action::NavigateToHyperspace(NavigateToHyperspaceStep {
                max_ticks,
                encounter_response: None,
            }),
            Action::FlyToStar(crate::automation::star_flight::FlyToStarStep {
                retreat_encounters: false,
                stop_at_encounter: false,
                encounter_response: None,
                target: crate::automation::star_flight::StarTarget::resolve("Alpha Centauri")
                    .unwrap(),
                max_ticks,
            }),
            Action::FlyToStar(crate::automation::star_flight::FlyToStarStep {
                retreat_encounters: true,
                stop_at_encounter: false,
                encounter_response: None,
                target: crate::automation::star_flight::StarTarget::resolve("Alpha Centauri")
                    .unwrap(),
                max_ticks,
            }),
            Action::TurnFlagship(TurnFlagshipStep {
                direction: TurnDirection::Right,
                facings: 4,
                max_ticks,
            }),
            Action::CollectPlanetMinerals(CollectPlanetMineralsStep {
                category: ElementCategory::Radioactive,
                minimum_units: 1,
                max_ticks,
            }),
            Action::UseDevice(crate::automation::story_actions::UseDeviceStep {
                device: crate::automation::story_actions::Device::ClearSpindle,
                max_ticks,
            }),
            Action::CollectEnergyNode(crate::automation::story_actions::CollectEnergyNodeStep {
                max_ticks,
            }),
            Action::FightBattle(FightBattleStep { max_ticks }),
            Action::FightEncounter(FightBattleStep { max_ticks }),
        ]
    }

    fn condition_actions(max_ticks: u64) -> Vec<Action> {
        vec![
            Action::WaitForPlanetSideStart(WaitForPlanetSideStartStep { max_ticks }),
            Action::WaitForPlanetSideEnd(WaitForPlanetSideEndStep {
                outcome: PlanetSideOutcomeName::Returned,
                max_ticks,
            }),
            Action::SelectPlanetMenu(SelectPlanetMenuStep {
                phase: PlanetMenuPhaseName::Orbit,
                max_ticks,
            }),
            Action::PickBattleShip(PickBattleShipStep { max_ticks }),
            Action::WaitForBattleShipLoss(WaitForBattleShipLossStep {
                minimum_ships: 1,
                max_ticks,
            }),
            Action::WaitForBattleFrames(WaitForBattleFramesStep {
                minimum: 1,
                max_ticks,
            }),
            Action::WaitForDispatch(WaitForDispatchStep {
                encounter: 2,
                dialogue: 2,
                max_ticks,
            }),
            Action::SelectCommunicationResponse(SelectCommunicationResponseStep {
                index: 0,
                max_ticks,
            }),
            Action::WaitForCommunicationEnd(WaitForCommunicationEndStep {
                minimum_completions: 1,
                max_ticks,
            }),
            Action::WaitForCommunicationReplay(WaitForCommunicationReplayStep { max_ticks }),
        ]
    }

    fn running_phase(action: &Action, remaining: u64) -> ActionPhase {
        if matches!(
            action,
            Action::NavigateToPlanet(_)
                | Action::NavigateToContact(_)
                | Action::NavigateToMoon(_)
                | Action::NavigateToOrbit(_)
                | Action::UseDevice(_)
                | Action::CollectEnergyNode(_)
                | Action::FlyToStar(_)
                | Action::NavigateToHyperspace(_)
                | Action::TurnFlagship(_)
                | Action::CollectPlanetMinerals(_)
                | Action::FightBattle(_)
                | Action::FightSamatra(_)
                | Action::FightEncounter(_)
        ) {
            ActionPhase::Navigating { remaining }
        } else {
            ActionPhase::WaitingCondition { remaining }
        }
    }

    fn reached_event(action: &Action) -> SchedulerEvent {
        if matches!(running_phase(action, 0), ActionPhase::Navigating { .. }) {
            SchedulerEvent::NavigationReached
        } else {
            SchedulerEvent::ConditionReached
        }
    }

    fn other_family_event(action: &Action) -> SchedulerEvent {
        if matches!(running_phase(action, 0), ActionPhase::Navigating { .. }) {
            SchedulerEvent::ConditionReached
        } else {
            SchedulerEvent::NavigationReached
        }
    }

    fn all_budgeted_actions(max_ticks: u64) -> Vec<Action> {
        let mut actions = navigation_actions(max_ticks);
        actions.extend(condition_actions(max_ticks));
        actions
    }

    fn step(
        state: &SchedulerState,
        actions: &[Action],
        event: SchedulerEvent,
    ) -> SchedulerTransition {
        let config = SchedulerConfig {
            actions,
            transitions: &[],
        };
        scheduler_reduce(state, &config, event)
    }

    #[test]
    fn budget_counts_down_one_admitted_input_at_a_time_then_fails_the_proof() {
        for action in all_budgeted_actions(3) {
            let script = [action, Action::Finish];
            let mut state = SchedulerState::initial();
            for (tick, remaining) in [(1_u64, 2_u64), (2, 1), (3, 0)] {
                let transition = step(&state, &script, SchedulerEvent::AdmittedInput);
                assert_eq!(
                    transition.new_state.phase,
                    running_phase(&script[0], remaining),
                    "{:?} after input {tick}",
                    script[0]
                );
                assert_eq!(transition.new_state.state_version, tick, "{:?}", script[0]);
                assert_eq!(transition.new_state.step_index, 0, "{:?}", script[0]);
                assert_eq!(transition.new_state.terminal, None, "{:?}", script[0]);
                assert!(transition.effects.is_empty(), "{:?}", script[0]);
                state = transition.new_state;
            }
            let expired = step(&state, &script, SchedulerEvent::AdmittedInput);
            assert_eq!(
                expired.new_state.terminal,
                Some(TerminalOutcome::SemanticMismatch),
                "{:?} must fail once its budget is spent",
                script[0]
            );
            assert_eq!(expired.new_state.step_index, 0, "{:?}", script[0]);
            assert_eq!(expired.new_state.state_version, 4, "{:?}", script[0]);
            assert!(expired.effects.is_empty(), "{:?}", script[0]);
        }
    }

    #[test]
    fn a_zero_budget_admits_one_input_and_then_fails() {
        for action in all_budgeted_actions(0) {
            let script = [action, Action::Finish];
            let started = step(
                &SchedulerState::initial(),
                &script,
                SchedulerEvent::AdmittedInput,
            );
            assert_eq!(
                started.new_state.phase,
                running_phase(&script[0], 0),
                "{:?}",
                script[0]
            );
            let expired = step(&started.new_state, &script, SchedulerEvent::AdmittedInput);
            assert_eq!(
                expired.new_state.terminal,
                Some(TerminalOutcome::SemanticMismatch),
                "{:?}",
                script[0]
            );
        }
    }

    #[test]
    fn the_matching_reached_event_advances_before_and_during_the_budget() {
        for action in all_budgeted_actions(4) {
            let script = [action, Action::Finish];
            let event = reached_event(&script[0]);

            let immediately = step(&SchedulerState::initial(), &script, event);
            assert_eq!(immediately.new_state.step_index, 1, "{:?}", script[0]);
            assert_eq!(
                immediately.new_state.phase,
                ActionPhase::WaitingForInput,
                "{:?}",
                script[0]
            );
            assert_eq!(immediately.new_state.terminal, None, "{:?}", script[0]);
            assert_eq!(immediately.new_state.state_version, 1, "{:?}", script[0]);

            let counting = step(
                &SchedulerState::initial(),
                &script,
                SchedulerEvent::AdmittedInput,
            );
            let during = step(&counting.new_state, &script, event);
            assert_eq!(during.new_state.step_index, 1, "{:?}", script[0]);
            assert_eq!(
                during.new_state.phase,
                ActionPhase::WaitingForInput,
                "{:?}",
                script[0]
            );
            assert_eq!(during.new_state.state_version, 2, "{:?}", script[0]);
            assert!(during.effects.is_empty(), "{:?}", script[0]);
        }
    }

    #[test]
    fn a_reached_event_after_the_budget_is_spent_is_a_mismatch_not_a_pass() {
        for action in all_budgeted_actions(1) {
            let script = [action, Action::Finish];
            let spent = step(
                &SchedulerState::initial(),
                &script,
                SchedulerEvent::AdmittedInput,
            );
            assert_eq!(
                spent.new_state.phase,
                running_phase(&script[0], 0),
                "{:?}",
                script[0]
            );
            let late = step(&spent.new_state, &script, reached_event(&script[0]));
            assert_eq!(
                late.new_state.terminal,
                Some(TerminalOutcome::SemanticMismatch),
                "{:?}",
                script[0]
            );
            assert_eq!(late.new_state.step_index, 0, "{:?}", script[0]);
        }
    }

    #[test]
    fn a_reached_event_from_the_other_family_changes_nothing() {
        for action in all_budgeted_actions(4) {
            let script = [action, Action::Finish];
            let counting = step(
                &SchedulerState::initial(),
                &script,
                SchedulerEvent::AdmittedInput,
            );
            for event in [
                other_family_event(&script[0]),
                SchedulerEvent::MainMenuReady,
            ] {
                let ignored = step(&counting.new_state, &script, event);
                assert_eq!(
                    ignored.new_state, counting.new_state,
                    "{:?} must ignore {event:?}",
                    script[0]
                );
                assert!(ignored.effects.is_empty(), "{:?}", script[0]);
            }
        }
    }

    #[test]
    fn a_budget_phase_that_belongs_to_the_other_family_is_an_invalid_state() {
        for action in all_budgeted_actions(4) {
            let wrong = match running_phase(&action, 2) {
                ActionPhase::Navigating { remaining } => {
                    ActionPhase::WaitingCondition { remaining }
                }
                _ => ActionPhase::Navigating { remaining: 2 },
            };
            let script = [action, Action::Finish];
            let state = SchedulerState {
                phase: wrong,
                ..SchedulerState::initial()
            };
            let transition = step(&state, &script, SchedulerEvent::AdmittedInput);
            assert_eq!(
                transition.new_state.terminal,
                Some(TerminalOutcome::InvalidState),
                "{:?} cannot run in {wrong:?}",
                script[0]
            );
        }
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use crate::automation::script::{
        ActivityAssertion, SetMenuKeyStep, TapMenuKeyStep, WaitInputTicksStep,
    };
    use proptest::prelude::*;

    fn small_actions() -> impl Strategy<Value = Vec<Action>> {
        prop::collection::vec(
            prop_oneof![
                (0u64..5).prop_map(|c| Action::WaitInputTicks(WaitInputTicksStep { count: c })),
                (0u64..6u64).prop_map(|i| {
                    Action::SetMenuKey(SetMenuKeyStep {
                        key: MenuKey::from_index(5 + i as u8).unwrap_or(MenuKey::Down),
                        value: 1,
                    })
                }),
                (0u64..4, 0u64..3).prop_map(|(h, s)| {
                    Action::TapMenuKey(TapMenuKeyStep {
                        key: MenuKey::Down,
                        value: 1,
                        hold: h.max(1),
                        settle: s,
                    })
                }),
                Just(Action::AssertActivity(ActivityAssertion {
                    mask: 0,
                    equals: 0
                })),
            ],
            1..5,
        )
        .prop_map(|mut actions| {
            actions.push(Action::Finish);
            actions
        })
    }

    proptest! {
        /// No counter wrap: state_version and capture_generation use checked
        /// arithmetic (REQ-SCHED-003); overflow is a typed terminal failure.
        #[test]
        fn state_never_panics(actions in small_actions()) {
            let config = SchedulerConfig { actions: &actions, transitions: &[] };
            let mut state = SchedulerState::initial();
            for _ in 0..100 {
                let t = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
                state = t.new_state;
                if state.is_terminal() {
                    break;
                }
            }
        }

        /// Terminal state is absorbing: once terminal, all events return the same state.
        #[test]
        fn terminal_absorbing(actions in small_actions()) {
            let config = SchedulerConfig { actions: &actions, transitions: &[] };
            let mut state = SchedulerState::initial();
            // Run until terminal
            for _ in 0..200 {
                let t = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
                state = t.new_state;
                if state.is_terminal() {
                    break;
                }
            }
            if state.is_terminal() {
                let t1 = scheduler_reduce(&state, &config, SchedulerEvent::AdmittedInput);
                let t2 = scheduler_reduce(
                    &state,
                    &config,
                    SchedulerEvent::CommittedPresent { generation: CaptureGeneration(0) },
                );
                prop_assert_eq!(t1.new_state, state);
                prop_assert_eq!(t2.new_state, state);
                prop_assert!(t1.effects.is_empty());
                prop_assert!(t2.effects.is_empty());
            }
        }

        /// Deterministic replay: the same event sequence produces the same state sequence.
        #[test]
        fn deterministic_replay(actions in small_actions()) {
            let config = SchedulerConfig { actions: &actions, transitions: &[] };

            let run1: Vec<SchedulerState> = {
                let mut s = SchedulerState::initial();
                let mut results = vec![s];
                for _ in 0..50 {
                    let t = scheduler_reduce(&s, &config, SchedulerEvent::AdmittedInput);
                    s = t.new_state;
                    results.push(s);
                    if s.is_terminal() { break; }
                }
                results
            };

            let run2: Vec<SchedulerState> = {
                let mut s = SchedulerState::initial();
                let mut results = vec![s];
                for _ in 0..50 {
                    let t = scheduler_reduce(&s, &config, SchedulerEvent::AdmittedInput);
                    s = t.new_state;
                    results.push(s);
                    if s.is_terminal() { break; }
                }
                results
            };

            prop_assert_eq!(run1, run2);

        }
    }
}
