//! Replay the existing semantic trace vocabulary against selected action parameters.

use super::{
    script::Action,
    trace::{RecordKind, TraceRecord},
};

fn number(label: &str, key: &str) -> Option<u64> {
    label
        .split(':')
        .find_map(|field| field.strip_prefix(key))?
        .parse()
        .ok()
}

fn at_least(label: &str, key: &str, minimum: u64) -> bool {
    number(label, key).is_some_and(|actual| actual >= minimum)
}

fn shape(label: &str, prefix: &str, keys: &[&str]) -> bool {
    let mut fields = label.split(':');
    fields.next() == Some(prefix)
        && keys.iter().all(|key| {
            fields.next().is_some_and(|field| {
                field
                    .strip_prefix(key)
                    .is_some_and(|value| !value.is_empty())
            })
        })
        && fields.next().is_none()
}

pub(super) fn satisfied(
    action: &Action,
    records: &[TraceRecord],
    frame: &super::trace::PresentationEvidence,
) -> bool {
    records.iter().any(|record| match action {
        Action::Capture(step) => {
            record.kind == RecordKind::Capture
                && record.presentation.as_ref() == Some(frame)
                && record.label.as_deref()
                    == Some(&format!("{}_gen{}", step.label, frame.generation))
        }
        Action::WaitForMainMenuReady(_) => {
            record.kind == RecordKind::Readiness
                && record.readiness.as_ref().is_some_and(|ready| {
                    ready.subject == super::coordinator::MAIN_MENU_READINESS_SUBJECT
                        && ready.observation == super::coordinator::MAIN_MENU_READINESS_OBSERVATION
                })
        }
        Action::AssertActivity(assertion) => {
            record.kind == RecordKind::SemanticAssertion
                && record.activity.as_ref().is_some_and(|actual| {
                    actual.passed
                        && actual.mask == assertion.mask
                        && actual.equals == assertion.equals
                        && actual.word & assertion.mask == assertion.equals
                })
        }
        _ => {
            record.kind == RecordKind::SemanticAssertion
                && record
                    .label
                    .as_deref()
                    .is_some_and(|label| semantic_satisfied(action, label))
        }
    })
}

fn semantic_satisfied(action: &Action, label: &str) -> bool {
    match action {
        Action::AssertScene(value) => super::scenario::scene_plan(
            value.scene,
            super::scenario::SceneActivationBoundary::GameInitialized,
        )
        .is_ok_and(|plan| {
            label
                == format!(
                    "scene_verified:{}:encounter={}:dialogue={}",
                    value.scene.name(),
                    plan.expected_encounter_conversation,
                    plan.expected_dialogue_conversation
                )
        }),
        Action::AssertMode(value) => label == format!("mode_verified:{}", value.mode.name()),
        Action::AssertDispatch(value) => {
            label
                == format!(
                    "dispatch_verified:encounter={}:dialogue={}",
                    value.encounter, value.dialogue
                )
        }
        Action::AssertGameOptions(_) => label == "game_options_active",
        Action::AssertCommunicationResponses(value) => {
            shape(label, "communication_responses_active", &["count="])
                && at_least(label, "count=", value.minimum as u64)
        }
        Action::AssertBattleFrames(value) => {
            shape(label, "battle_frames_verified", &["count="])
                && at_least(label, "count=", value.minimum)
        }
        Action::AssertPlanetSideCollisions(value) => collision_satisfied(label, value),
        Action::AssertMainMenuTransition(value) => menu_transition_satisfied(label, value),
        _ => wait_or_navigation_satisfied(action, label),
    }
}

fn collision_satisfied(label: &str, value: &super::script::PlanetSideCollisionAssertion) -> bool {
    shape(
        label,
        "planet_side_collisions_verified",
        &["mineral=", "creature_hits=", "seam="],
    ) && at_least(label, "mineral=", value.mineral_pickups)
        && at_least(label, "creature_hits=", value.creature_hits)
        && at_least(label, "seam=", value.seam_hits)
}

fn wait_or_navigation_satisfied(action: &Action, label: &str) -> bool {
    match action {
        Action::WaitForBattleFrames(value) => {
            shape(label, "battle_frames_reached", &["count=", "minimum="])
                && at_least(label, "count=", value.minimum)
                && number(label, "minimum=") == Some(value.minimum)
        }
        Action::WaitForDispatch(value) => {
            shape(
                label,
                "dispatch_observed",
                &["generation=", "encounter=", "dialogue="],
            ) && at_least(label, "generation=", 1)
                && number(label, "encounter=") == Some(u64::from(value.encounter))
                && number(label, "dialogue=") == Some(u64::from(value.dialogue))
        }
        Action::WaitForCommunicationEnd(value) => {
            shape(label, "communication_completed", &["count=", "minimum="])
                && at_least(label, "count=", value.minimum_completions)
                && number(label, "minimum=") == Some(value.minimum_completions)
        }
        Action::WaitForCommunicationReplay(_) => {
            shape(label, "communication_replay_active", &["generation="])
                && at_least(label, "generation=", 1)
        }
        Action::WaitForPlanetSideStart(_) => {
            shape(
                label,
                "planet_side_started",
                &["generation=", "crew=", "position="],
            ) && at_least(label, "generation=", 1)
                && number(label, "crew=").is_some()
        }
        Action::WaitForPlanetSideEnd(value) => {
            shape(
                label,
                "planet_side_completed",
                &["generation=", "outcome=", "crew=", "minerals="],
            ) && at_least(label, "generation=", 1)
                && number(label, "crew=").is_some()
                && number(label, "minerals=").is_some()
                && label.contains(&format!(":outcome={:?}:", value.outcome))
        }
        Action::NavigateToPlanet(value) => {
            label == format!("navigation_reached:planet={}", value.planet)
        }
        Action::NavigateToOrbit(value) => label == format!("orbit_reached:planet={}", value.planet),
        Action::NavigateToMoon(value) => {
            shape(
                label,
                "navigation_reached",
                &["planet=", "moon=", "orbital_data=", "target_data="],
            ) && number(label, "planet=") == Some(u64::from(value.planet))
                && number(label, "moon=") == Some(u64::from(value.moon))
                && number(label, "orbital_data=").is_some()
                && number(label, "orbital_data=") == number(label, "target_data=")
        }
        Action::SelectPlanetMenu(value) => {
            shape(label, "planet_menu_selected", &["generation=", "phase="])
                && at_least(label, "generation=", 1)
                && label.ends_with(&format!(":phase={:?}", value.phase))
        }
        Action::SelectCommunicationResponse(value) => {
            shape(
                label,
                "communication_response_selected",
                &["generation=", "count=", "index="],
            ) && at_least(label, "generation=", 1)
                && number(label, "index=") == Some(value.index as u64)
                && at_least(label, "count=", value.index as u64 + 1)
        }
        Action::SetupPlanetSideCollisionFixture(_) => {
            label == "planet_side_collision_fixture_queued"
        }
        _ => false,
    }
}

fn menu_transition_satisfied(label: &str, value: &super::script::MainMenuTransitionDto) -> bool {
    let item = |key| {
        number(label, key)
            .and_then(|value| u8::try_from(value).ok())
            .and_then(crate::mainloop::restart_menu::types::RestartMenuItem::from_u8)
            .map(|value| format!("{value:?}"))
    };
    shape(label, "menu_transition_passed", &["from=", "to="])
        && item("from=").as_deref() == Some(&value.from)
        && item("to=").as_deref() == Some(&value.to)
}

/// Consumption history mirrors the one-shot production observation contracts.
#[derive(Default)]
pub(super) struct PredicateHistory {
    dispatch: u64,
    response: u64,
    replay: u64,
    menu: u64,
    completions: u64,
    last_planet_side: u64,
    awaited_planet_side: Option<u64>,
}

impl PredicateHistory {
    pub(super) fn verify(
        &mut self,
        action: &Action,
        records: &[TraceRecord],
        frame: &super::trace::PresentationEvidence,
    ) -> bool {
        let Some(record) = records
            .iter()
            .find(|record| satisfied(action, std::slice::from_ref(*record), frame))
        else {
            return false;
        };
        let label = record.label.as_deref().unwrap_or_default();
        match action {
            Action::WaitForDispatch(_) => advance(&mut self.dispatch, number(label, "generation=")),
            Action::SelectCommunicationResponse(_) => {
                advance(&mut self.response, number(label, "generation="))
            }
            Action::WaitForCommunicationReplay(_) => {
                advance(&mut self.replay, number(label, "generation="))
            }
            Action::SelectPlanetMenu(_) => advance(&mut self.menu, number(label, "generation=")),
            Action::WaitForCommunicationEnd(value) => {
                let (Some(target), Some(actual)) = (
                    self.completions.checked_add(value.minimum_completions),
                    number(label, "count="),
                ) else {
                    return false;
                };
                if actual < target {
                    return false;
                }
                self.completions = actual;
                true
            }
            Action::WaitForPlanetSideStart(_) => {
                if self.awaited_planet_side.is_some()
                    || !advance(&mut self.last_planet_side, number(label, "generation="))
                {
                    return false;
                }
                self.awaited_planet_side = Some(self.last_planet_side);
                true
            }
            Action::WaitForPlanetSideEnd(_) => {
                if self.awaited_planet_side.is_none()
                    || self.awaited_planet_side != number(label, "generation=")
                {
                    return false;
                }
                self.awaited_planet_side = None;
                true
            }
            _ => true,
        }
    }
}

fn advance(previous: &mut u64, observed: Option<u64>) -> bool {
    let Some(observed) = observed else {
        return false;
    };
    if observed <= *previous {
        return false;
    }
    *previous = observed;
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn frame() -> super::super::trace::PresentationEvidence {
        super::super::trace::PresentationEvidence {
            count: 1,
            generation: 0,
            width: 320,
            height: 240,
        }
    }

    fn record(label: &str) -> TraceRecord {
        let mut record = super::super::capture::capture_trace_record(
            0,
            0,
            super::super::scheduler::CaptureGeneration(0),
            label,
        );
        record.kind = RecordKind::SemanticAssertion;
        record.label = Some(label.into());
        record
    }

    #[test]
    fn semantic_parameters_reject_partial_duplicate_and_wrong_kind_evidence() {
        for (value, label) in [
            (
                json!({"action":"assert_mode","mode":"main_menu"}),
                "mode_verified:main_menu",
            ),
            (
                json!({"action":"assert_dispatch","encounter":1,"dialogue":2}),
                "dispatch_verified:encounter=1:dialogue=2",
            ),
            (
                json!({"action":"assert_game_options"}),
                "game_options_active",
            ),
            (
                json!({"action":"assert_communication_responses","minimum":3}),
                "communication_responses_active:count=3",
            ),
            (
                json!({"action":"assert_battle_frames","minimum":30}),
                "battle_frames_verified:count=30",
            ),
            (
                json!({"action":"assert_planet_side_collisions","mineral_pickups":1,"creature_hits":2,"seam_hits":3}),
                "planet_side_collisions_verified:mineral=1:creature_hits=2:seam=3",
            ),
            (
                json!({"action":"assert_main_menu_transition","from":"NewGame","to":"LoadGame"}),
                "menu_transition_passed:from=0:to=1",
            ),
            (
                json!({"action":"wait_for_battle_frames","minimum":30,"max_ticks":100}),
                "battle_frames_reached:count=30:minimum=30",
            ),
            (
                json!({"action":"wait_for_dispatch","encounter":1,"dialogue":2,"max_ticks":100}),
                "dispatch_observed:generation=1:encounter=1:dialogue=2",
            ),
            (
                json!({"action":"wait_for_communication_end","minimum_completions":2,"max_ticks":100}),
                "communication_completed:count=2:minimum=2",
            ),
            (
                json!({"action":"wait_for_communication_replay","max_ticks":100}),
                "communication_replay_active:generation=1",
            ),
            (
                json!({"action":"wait_for_planet_side_start","max_ticks":100}),
                "planet_side_started:generation=1:crew=12:position=1,2",
            ),
            (
                json!({"action":"wait_for_planet_side_end","outcome":"returned","max_ticks":100}),
                "planet_side_completed:generation=1:outcome=Returned:crew=12:minerals=2",
            ),
            (
                json!({"action":"navigate_to_planet","planet":2,"max_ticks":100}),
                "navigation_reached:planet=2",
            ),
            (
                json!({"action":"navigate_to_orbit","planet":2,"max_ticks":100}),
                "orbit_reached:planet=2",
            ),
            (
                json!({"action":"navigate_to_moon","planet":2,"moon":0,"max_ticks":100}),
                "navigation_reached:planet=2:moon=0:orbital_data=1:target_data=1",
            ),
            (
                json!({"action":"select_planet_menu","phase":"dispatch","max_ticks":100}),
                "planet_menu_selected:generation=1:phase=Dispatch",
            ),
            (
                json!({"action":"select_communication_response","index":1,"max_ticks":100}),
                "communication_response_selected:generation=1:count=3:index=1",
            ),
            (
                json!({"action":"setup_planet_side_collision_fixture"}),
                "planet_side_collision_fixture_queued",
            ),
        ] {
            let action: Action = serde_json::from_value(value).unwrap();
            assert!(satisfied(&action, &[record(label)], &frame()), "{action:?}");
            assert!(
                !satisfied(&action, &[record(&format!("{label}:count=999"))], &frame()),
                "{action:?}"
            );
            assert!(
                !satisfied(&action, &[record("unrelated")], &frame()),
                "{action:?}"
            );
            let mut wrong = record(label);
            wrong.kind = RecordKind::Failure;
            assert!(!satisfied(&action, &[wrong], &frame()));
        }
    }

    #[test]
    fn generation_obligations_cannot_reuse_an_earlier_completion() {
        for (value, label) in [
            (
                json!({"action":"wait_for_dispatch","encounter":1,"dialogue":2,"max_ticks":100}),
                "dispatch_observed:generation=1:encounter=1:dialogue=2",
            ),
            (
                json!({"action":"wait_for_communication_replay","max_ticks":100}),
                "communication_replay_active:generation=1",
            ),
            (
                json!({"action":"select_planet_menu","phase":"dispatch","max_ticks":100}),
                "planet_menu_selected:generation=1:phase=Dispatch",
            ),
            (
                json!({"action":"select_communication_response","index":1,"max_ticks":100}),
                "communication_response_selected:generation=1:count=3:index=1",
            ),
        ] {
            let action: Action = serde_json::from_value(value).unwrap();
            let mut history = PredicateHistory::default();
            assert!(history.verify(&action, &[record(label)], &frame()));
            assert!(!history.verify(&action, &[record(label)], &frame()));
            assert!(!history.verify(
                &action,
                &[record(&label.replace("generation=1", "generation=0"))],
                &frame()
            ));
            assert!(history.verify(
                &action,
                &[record(&label.replace("generation=1", "generation=2"))],
                &frame()
            ));
        }
    }

    #[test]
    fn planet_trip_and_communication_waits_consume_their_own_observations() {
        let start: Action =
            serde_json::from_value(json!({"action":"wait_for_planet_side_start","max_ticks":100}))
                .unwrap();
        let end: Action = serde_json::from_value(
            json!({"action":"wait_for_planet_side_end","outcome":"returned","max_ticks":100}),
        )
        .unwrap();
        let mut history = PredicateHistory::default();
        let started = record("planet_side_started:generation=1:crew=12:position=1,2");
        let ended =
            record("planet_side_completed:generation=1:outcome=Returned:crew=12:minerals=2");
        assert!(!history.verify(&end, std::slice::from_ref(&ended), &frame()));
        assert!(history.verify(&start, std::slice::from_ref(&started), &frame()));
        assert!(!history.verify(
            &end,
            &[record(
                "planet_side_completed:generation=2:outcome=Returned:crew=12:minerals=2"
            )],
            &frame()
        ));
        assert!(history.verify(&end, std::slice::from_ref(&ended), &frame()));
        assert!(!history.verify(&end, &[ended], &frame()));
        assert!(!history.verify(&start, &[started], &frame()));
        let wait: Action = serde_json::from_value(
            json!({"action":"wait_for_communication_end","minimum_completions":2,"max_ticks":100}),
        )
        .unwrap();
        assert!(history.verify(
            &wait,
            &[record("communication_completed:count=2:minimum=2")],
            &frame()
        ));
        assert!(!history.verify(
            &wait,
            &[record("communication_completed:count=3:minimum=2")],
            &frame()
        ));
        assert!(history.verify(
            &wait,
            &[record("communication_completed:count=4:minimum=2")],
            &frame()
        ));
    }
}
