use super::*;

/// Reduce a capture, assertion, fixture, or finish, or decline so the next family may answer.
pub(super) fn reduce_capture_and_assertion(
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

        (action, ActionPhase::WaitingForInput) if callback_assertion(action) => {
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

fn callback_assertion(action: &Action) -> bool {
    matches!(
        action,
        Action::AssertActivity(_)
            | Action::AssertScene(_)
            | Action::AssertMode(_)
            | Action::AssertDispatch(_)
            | Action::AssertGameOptions(_)
            | Action::AssertCommunicationResponses(_)
            | Action::AssertBattleFrames(_)
            | Action::RecordHyperspacePosition(_)
            | Action::AssertHyperspaceDisplacement(_)
            | Action::AssertHyperspaceStationary(_)
            | Action::RecordSavedHyperspacePosition(_)
            | Action::AssertHyperspacePositionRestored(_)
            | Action::AssertShipResources(_)
            | Action::AssertState(_)
            | Action::AssertShipElement(_)
            | Action::AssertPlanetSideCollisions(_)
            | Action::SetupPlanetSideCollisionFixture(_)
            | Action::SetupMeleeMutualLossFixture(_)
            | Action::ConfigureParityMelee(_)
            | Action::RunParityFixture(_)
            | Action::RunParityCorpus(_)
            | Action::SetupPlanetSideMineralGallery(_)
            | Action::AssertParityFixture
    )
}
