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
        Action::NavigateToPlanet(_) | Action::NavigateToMoon(_) | Action::NavigateToOrbit(_) => {
            matches!(
                phase,
                ActionPhase::WaitingForInput | ActionPhase::Navigating { .. }
            )
        }
        Action::WaitForPlanetSideStart(_)
        | Action::WaitForPlanetSideEnd(_)
        | Action::SelectPlanetMenu(_)
        | Action::WaitForBattleFrames(_)
        | Action::WaitForDispatch(_)
        | Action::SelectCommunicationResponse(_)
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
                    | Action::NavigateToMoon(_)
                    | Action::NavigateToOrbit(_)
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
                    | Action::WaitForDispatch(_)
                    | Action::SelectCommunicationResponse(_)
                    | Action::WaitForPlanetSideStart(_)
                    | Action::WaitForPlanetSideEnd(_)
                    | Action::SelectPlanetMenu(_)
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
        (Action::WaitForMainMenuReady(_), ActionPhase::WaitingForInput) => SchedulerTransition {
            new_state: SchedulerState {
                state_version: sv,
                ..*state
            },
            effects: EffectPlan::none(),
        },
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
        | (Action::NavigateToMoon(_), ActionPhase::Navigating { remaining: 0 })
        | (Action::NavigateToOrbit(_), ActionPhase::Navigating { remaining: 0 }) => {
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
        | (Action::NavigateToMoon(_), ActionPhase::Navigating { remaining })
        | (Action::NavigateToOrbit(_), ActionPhase::Navigating { remaining }) => {
            SchedulerTransition {
                new_state: SchedulerState {
                    phase: ActionPhase::Navigating {
                        remaining: remaining - 1,
                    },
                    state_version: sv,
                    ..*state
                },
                effects: EffectPlan::none(),
            }
        }
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
        (Action::SelectCommunicationResponse(select), ActionPhase::WaitingForInput) => {
            SchedulerTransition {
                new_state: SchedulerState {
                    phase: ActionPhase::WaitingCondition {
                        remaining: select.max_ticks.saturating_sub(1),
                    },
                    state_version: sv,
                    ..*state
                },
                effects: EffectPlan::none(),
            }
        }
        (
            Action::SelectCommunicationResponse(_),
            ActionPhase::WaitingCondition { remaining: 0 },
        ) => SchedulerTransition {
            new_state: SchedulerState {
                terminal: Some(TerminalOutcome::SemanticMismatch),
                state_version: sv,
                ..*state
            },
            effects: EffectPlan::none(),
        },
        (Action::SelectCommunicationResponse(_), ActionPhase::WaitingCondition { remaining }) => {
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
        | (Action::AssertDispatch(_), ActionPhase::WaitingForInput)
        | (Action::AssertGameOptions(_), ActionPhase::WaitingForInput)
        | (Action::AssertCommunicationResponses(_), ActionPhase::WaitingForInput)
        | (Action::AssertBattleFrames(_), ActionPhase::WaitingForInput)
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

    // Zero-callback chaining: if the next action requires no input callbacks
    // (wait_input_ticks(0) or AssertActivity), chain through it immediately.
    // AssertMainMenuTransition is handled by the AdmittedInput match arm
    // (enters WaitingSemantic on first input callback), NOT chained here.
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
            Action::AssertActivity(_) => {
                // AssertActivity is immediate — no callback needed.
                let chained_state = SchedulerState {
                    step_index: next_index,
                    phase: ActionPhase::WaitingForInput,
                    state_version: sv,
                    ..*state
                };
                return advance_to_next(&chained_state, config, sv, initial_effects);
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
        ActivityAssertion, CaptureStep, MainMenuTransitionDto, NavigateToMoonStep, SetMenuKeyStep,
        TapMenuKeyStep, TapPlayerKeyStep, WaitForBattleFramesStep, WaitForCommunicationEndStep,
        WaitForCommunicationReplayStep, WaitForDispatchStep, WaitForMainMenuReadyStep,
        WaitInputTicksStep, WaitPresentationsStep,
    };
    use crate::mainloop::restart_menu::types::RestartMenuItem;

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
