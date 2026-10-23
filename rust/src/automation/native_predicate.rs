//! Replay the existing semantic trace vocabulary against selected action parameters.

use super::{
    navigation::HYPERSPACE_REACHED_LABEL,
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

fn has_category(label: &str, category: crate::planet_side::cargo::ElementCategory) -> bool {
    label
        .split(':')
        .any(|field| field.strip_prefix("category=") == Some(category.name()))
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
        Action::AssertHyperspaceDisplacement(value) => {
            shape(
                label,
                "hyperspace_displacement_verified",
                &["from=", "to=", "moved=", "minimum="],
            ) && number(label, "minimum=") == Some(u64::from(value.minimum_logical_units))
                && at_least(label, "moved=", u64::from(value.minimum_logical_units))
        }
        Action::AssertHyperspaceStationary(value) => {
            shape(
                label,
                "hyperspace_stationary_verified",
                &["from=", "to=", "moved=", "maximum="],
            ) && number(label, "maximum=") == Some(u64::from(value.maximum_logical_units))
                && number(label, "moved=")
                    .is_some_and(|moved| moved <= u64::from(value.maximum_logical_units))
        }
        Action::AssertHyperspacePositionRestored(_) => {
            shape(
                label,
                "hyperspace_position_restored",
                &["x=", "y=", "saved_x=", "saved_y="],
            ) && number(label, "x=").is_some()
                && number(label, "x=") == number(label, "saved_x=")
                && number(label, "y=").is_some()
                && number(label, "y=") == number(label, "saved_y=")
        }
        Action::AssertShipResources(value) => {
            super::ship_resources::verified_label_satisfies(label, *value)
        }
        Action::AssertState(predicate) => {
            super::campaign_state::label_satisfies(label, "state_verified", predicate)
        }
        Action::AssertShipElement(value) => {
            shape(
                label,
                "ship_element_verified",
                &["category=", "amount=", "minimum="],
            ) && has_category(label, value.category)
                && number(label, "minimum=") == Some(u64::from(value.minimum))
                && at_least(label, "amount=", u64::from(value.minimum))
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
        Action::WaitForBattleShipLoss(value) => {
            shape(
                label,
                "battle_ship_loss_reached",
                &["lost=", "minimum=", "start=", "left="],
            ) && number(label, "minimum=") == Some(u64::from(value.minimum_ships))
                && at_least(label, "lost=", u64::from(value.minimum_ships))
        }
        Action::WaitForState(value) => {
            super::campaign_state::label_satisfies(label, "state_reached", &value.predicate)
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
        Action::WaitForCommunicationResponses(value) => {
            shape(
                label,
                "communication_responses_reached",
                &["generation=", "count=", "minimum="],
            ) && at_least(label, "generation=", 1)
                && at_least(label, "count=", value.minimum as u64)
                && number(label, "minimum=") == Some(value.minimum as u64)
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
        Action::NavigateToContact(value) => label == format!("contact_reached:race={}", value.race),
        Action::NavigateToPlanet(value) => {
            label == format!("navigation_reached:planet={}", value.planet)
        }
        Action::NavigateToOrbit(value) => {
            label == format!("orbit_reached:planet={}", value.planet)
                || (value.stop_at_encounter
                    && label == format!("contact_reached:planet={}", value.planet))
        }
        Action::UseDevice(step) => {
            shape(
                label,
                "device_used",
                &["device=", "generation=", "verified=", "state="],
            ) && label.contains(&format!(":device={}:generation=", step.device.name()))
                && at_least(label, "generation=", 1)
                && number(label, "verified=") == Some(1)
                && label.split_once(":state=").is_some_and(|(_, state)| {
                    super::story_actions::device_evidence_holds(step.device, state)
                })
        }
        Action::CollectEnergyNode(_) => {
            if label.starts_with("energy_vault_unlocked:") {
                return shape(
                    label,
                    "energy_vault_unlocked",
                    &[
                        "generation=",
                        "node=",
                        "vault_before=",
                        "vault_after=",
                        "shuttle_before=",
                        "shuttle_after=",
                    ],
                ) && at_least(label, "generation=", 1)
                    && number(label, "node=") == Some(0)
                    && number(label, "vault_before=") == Some(0)
                    && number(label, "vault_after=") == Some(1)
                    && number(label, "shuttle_before=") == Some(1)
                    && number(label, "shuttle_after=") == Some(0);
            }
            shape(
                label,
                "energy_node_collected",
                &["generation=", "node=", "retrieved=", "flag=", "value="],
            ) && at_least(label, "generation=", 1)
                && number(label, "node=")
                    .zip(number(label, "retrieved="))
                    .is_some_and(|(node, retrieved)| node < 32 && retrieved & (1 << node) != 0)
                && crate::automation::story_actions::ARTIFACT_FLAGS
                    .iter()
                    .any(|flag| label.contains(&format!(":flag={flag}:")))
                && number(label, "value=") == Some(1)
        }
        Action::FlyToStar(flight) => super::star_flight::arrival_satisfies(label, flight.target),
        Action::NavigateToHyperspace(_) => label == HYPERSPACE_REACHED_LABEL,
        Action::TurnFlagship(value) => {
            shape(
                label,
                "flagship_turn_verified",
                &["direction=", "facings=", "from=", "to="],
            ) && label.contains(&format!(":direction={}:", value.direction.name()))
                && number(label, "facings=") == Some(u64::from(value.facings))
        }
        Action::CollectPlanetMinerals(value) => {
            shape(
                label,
                "planet_minerals_collected",
                &["category=", "units=", "minimum="],
            ) && has_category(label, value.category)
                && number(label, "minimum=") == Some(u64::from(value.minimum_units))
                && at_least(label, "units=", u64::from(value.minimum_units))
        }
        Action::FightSamatra(value) if value.until == super::samatra::Until::Ready => {
            shape(label, "samatra_ready", &["generators=", "generation="])
                && number(label, "generators=") == Some(8)
                && at_least(label, "generation=", 1)
        }
        Action::FightSamatra(value) if value.until == super::samatra::Until::GateOpen => {
            shape(
                label,
                "samatra_gate_open",
                &["generators_destroyed=", "gate_open=", "generation="],
            ) && number(label, "generators_destroyed=") == Some(8)
                && number(label, "gate_open=") == Some(1)
                && at_least(label, "generation=", 1)
        }
        Action::FightSamatra(value) => {
            shape(
                label,
                "samatra_won",
                &[
                    "generators_destroyed=",
                    "gate_entered=",
                    "escort=",
                    "bomb=",
                    "generation=",
                    "ticks=",
                ],
            ) && number(label, "generators_destroyed=") == Some(8)
                && number(label, "gate_entered=") == Some(1)
                && number(label, "escort=") == Some(1)
                && number(label, "bomb=") == Some(1)
                && at_least(label, "generation=", 1)
                && number(label, "ticks=")
                    .is_some_and(|ticks| ticks > 0 && ticks <= value.max_ticks)
        }
        Action::FightEncounter(value) => {
            shape(
                label,
                "fleet_won",
                &[
                    "player_left=",
                    "enemy_left=",
                    "ticks=",
                    "flagship_crew=",
                    "enemy_initial=",
                    "picks=",
                    "speed=",
                ],
            ) && at_least(label, "player_left=", 1)
                && number(label, "enemy_left=") == Some(0)
                && number(label, "flagship_crew=")
                    .is_some_and(|crew| crew > 0 && crew < u64::from(u16::MAX))
                && at_least(label, "enemy_initial=", 1)
                && at_least(label, "ticks=", 1)
                && number(label, "ticks=").is_some_and(|ticks| ticks <= value.max_ticks)
                && number(label, "speed=") == Some(1)
        }
        Action::FightBattle(value) => {
            shape(
                label,
                "battle_won",
                &["player_left=", "enemy_left=", "ticks=", "player_crew="],
            ) && at_least(label, "player_left=", 1)
                && at_least(label, "player_crew=", 1)
                && at_least(label, "ticks=", 1)
                && number(label, "ticks=").is_some_and(|ticks| ticks <= value.max_ticks)
        }
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
        Action::PickBattleShip(_) => {
            shape(label, "battle_ship_picked", &["generation="])
                && at_least(label, "generation=", 1)
        }
        Action::SelectResponse(value) => {
            shape(
                label,
                "response_selected",
                &["generation=", "count=", "index=", "phrase="],
            ) && at_least(label, "generation=", 1)
                && number(label, "index=")
                    .zip(number(label, "count="))
                    .is_some_and(|(index, count)| index < count)
                && label.ends_with(&format!(":phrase={}", value.phrase))
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
#[derive(Clone, Debug, Default)]
pub(super) struct PredicateHistory {
    dispatch: u64,
    response: u64,
    replay: u64,
    menu: u64,
    ship_pick: u64,
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
            Action::SelectCommunicationResponse(_) | Action::SelectResponse(_) => {
                advance(&mut self.response, number(label, "generation="))
            }
            // Waiting observes a list but consumes nothing: only a list newer
            // than the last one a selection consumed belongs to this conversation.
            Action::WaitForCommunicationResponses(_) => {
                number(label, "generation=").is_some_and(|observed| observed > self.response)
            }
            Action::WaitForCommunicationReplay(_) => {
                advance(&mut self.replay, number(label, "generation="))
            }
            Action::SelectPlanetMenu(_) => advance(&mut self.menu, number(label, "generation=")),
            Action::PickBattleShip(_) => advance(&mut self.ship_pick, number(label, "generation=")),
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
    fn contact_is_not_orbit_and_requires_explicit_interruption_permission() {
        use crate::automation::script::NavigateToOrbitStep;
        let action = |stop_at_encounter| {
            Action::NavigateToOrbit(NavigateToOrbitStep {
                planet: 0,
                max_ticks: 100,
                stop_at_encounter,
                encounter_response: None,
                patrol_policy: crate::automation::script::PatrolPolicy::Stop,
            })
        };
        assert!(!wait_or_navigation_satisfied(
            &action(false),
            "contact_reached:planet=0"
        ));
        assert!(wait_or_navigation_satisfied(
            &action(true),
            "contact_reached:planet=0"
        ));
        assert!(!wait_or_navigation_satisfied(
            &action(true),
            "contact_reached:planet=1"
        ));
        assert!(wait_or_navigation_satisfied(
            &action(false),
            "orbit_reached:planet=0"
        ));
    }

    #[test]
    fn phrase_checkpoints_require_the_requested_name_and_a_fresh_response_generation() {
        let action: Action = serde_json::from_value(
            json!({"action":"select_response","phrase":"we_will_transfer_now","max_ticks":100}),
        )
        .unwrap();
        assert_eq!(
            super::super::native_checkpoint::checkpoint_class(&action),
            Some(super::super::native_checkpoint::CheckpointClass::Navigation)
        );
        let good =
            record("response_selected:generation=2:count=3:index=1:phrase=we_will_transfer_now");
        let mut history = PredicateHistory::default();
        assert!(history.verify(&action, std::slice::from_ref(&good), &frame()));
        assert!(!history.verify(&action, std::slice::from_ref(&good), &frame()));
        for label in [
            "response_selected:generation=3:count=3:index=1:phrase=what_will_you_give_us",
            "response_selected:generation=3:count=1:index=1:phrase=we_will_transfer_now",
            "response_selected:generation=0:count=3:index=1:phrase=we_will_transfer_now",
        ] {
            assert!(!satisfied(&action, &[record(label)], &frame()));
        }
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
                json!({"action":"wait_for_communication_responses","minimum":2,"max_ticks":100}),
                "communication_responses_reached:generation=1:count=2:minimum=2",
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
                json!({"action":"navigate_to_hyperspace","max_ticks":100}),
                "navigation_reached:hyperspace",
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
                json!({"action":"pick_battle_ship","max_ticks":100}),
                "battle_ship_picked:generation=1",
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
                json!({"action":"pick_battle_ship","max_ticks":100}),
                "battle_ship_picked:generation=1",
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
    fn a_response_wait_is_not_satisfied_by_a_list_a_selection_already_consumed() {
        let select: Action = serde_json::from_value(
            json!({"action":"select_communication_response","index":0,"max_ticks":100}),
        )
        .unwrap();
        let wait: Action = serde_json::from_value(
            json!({"action":"wait_for_communication_responses","minimum":2,"max_ticks":100}),
        )
        .unwrap();
        let mut history = PredicateHistory::default();
        assert!(history.verify(
            &select,
            &[record(
                "communication_response_selected:generation=2:count=3:index=0"
            )],
            &frame()
        ));
        let reached = |generation: u64, count: u64| {
            record(&format!(
                "communication_responses_reached:generation={generation}:count={count}:minimum=2"
            ))
        };
        assert!(!history.verify(&wait, &[reached(2, 3)], &frame()));
        assert!(!history.verify(&wait, &[reached(1, 3)], &frame()));
        assert!(history.verify(&wait, &[reached(3, 2)], &frame()));
        // Waiting consumes nothing, so the selection of that list still counts.
        assert!(history.verify(
            &select,
            &[record(
                "communication_response_selected:generation=3:count=2:index=0"
            )],
            &frame()
        ));
        // A count below the declared minimum never satisfies the wait.
        assert!(!history.verify(&wait, &[reached(4, 1)], &frame()));
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
    #[test]
    fn only_a_verified_displacement_label_that_meets_the_minimum_satisfies_the_action() {
        let satisfies = |minimum: u32, label: &str| {
            semantic_satisfied(
                &Action::AssertHyperspaceDisplacement(
                    crate::automation::script::HyperspaceDisplacementAssertion {
                        minimum_logical_units: minimum,
                    },
                ),
                label,
            )
        };
        let verified = "hyperspace_displacement_verified:from=0,0:to=0,40:moved=40:minimum=16";
        assert!(satisfies(16, verified));
        assert!(
            !satisfies(32, verified),
            "a different minimum is another assertion"
        );
        assert!(!satisfies(
            16,
            "hyperspace_displacement_verified:from=0,0:to=0,4:moved=4:minimum=16"
        ));
        assert!(!satisfies(
            16,
            "hyperspace_displacement_failed:from=0,0:to=0,0:moved=0:minimum=16"
        ));
        assert!(!satisfies(16, "hyperspace_position_recorded:x=0:y=0"));
    }

    #[test]
    fn only_a_restored_label_with_equal_coordinates_satisfies_the_restore_assertion() {
        let action = Action::AssertHyperspacePositionRestored(
            crate::automation::script::HyperspacePositionRestoredAssertion {},
        );
        assert!(semantic_satisfied(
            &action,
            "hyperspace_position_restored:x=1752:y=1450:saved_x=1752:saved_y=1450"
        ));
        for label in [
            "hyperspace_position_restored:x=1752:y=1520:saved_x=1752:saved_y=1450",
            "hyperspace_position_restored:x=1753:y=1450:saved_x=1752:saved_y=1450",
            "hyperspace_position_restore_failed:saved=1752,1450:observed=1752,1520",
            "hyperspace_position_restored:x=1752:y=1450",
            "hyperspace_position_saved:x=1752:y=1450",
        ] {
            assert!(!semantic_satisfied(&action, label), "{label}");
        }
    }

    #[test]
    fn only_a_ship_loss_label_that_meets_the_requested_minimum_satisfies_the_wait() {
        let action = |minimum_ships: u32| {
            Action::WaitForBattleShipLoss(crate::automation::script::WaitForBattleShipLossStep {
                minimum_ships,
                max_ticks: 100,
            })
        };
        let label = "battle_ship_loss_reached:lost=2:minimum=2:start=14,14:left=13,13";
        assert!(semantic_satisfied(&action(2), label));
        assert!(!semantic_satisfied(&action(3), label));
        assert!(!semantic_satisfied(
            &action(1),
            "battle_ship_loss_reached:lost=0:minimum=1:start=14,14:left=14,14"
        ));
        assert!(!semantic_satisfied(
            &action(1),
            "battle_frames_reached:count=40:minimum=1"
        ));
    }

    #[test]
    fn only_the_verified_label_of_the_same_resource_expectation_satisfies_the_action() {
        use crate::automation::script::ShipResourcesAssertion;
        let action = Action::AssertShipResources(ShipResourcesAssertion {
            fuel_on_board: Some(6000),
            res_units: Some(1500),
        });
        assert!(semantic_satisfied(
            &action,
            "ship_resources_verified:fuel=6000:ru=1500:expected_fuel=6000:expected_ru=1500"
        ));
        for label in [
            "ship_resources_failed:fuel=5900:ru=1500:expected_fuel=6000:expected_ru=1500",
            "ship_resources_verified:fuel=5900:ru=1500:expected_fuel=6000:expected_ru=1500",
            "ship_resources_verified:fuel=6000:ru=1500:expected_fuel=6000:expected_ru=any",
            "ship_resources_verified:fuel=1000:ru=2500:expected_fuel=1000:expected_ru=2500",
            "ship_resources_unavailable",
        ] {
            assert!(!semantic_satisfied(&action, label), "{label}");
        }
    }

    #[test]
    fn only_a_verified_turn_label_of_the_same_direction_and_facings_satisfies_the_action() {
        use crate::automation::script::{TurnDirection, TurnFlagshipStep};
        let satisfies = |direction: TurnDirection, facings: u8, label: &str| {
            semantic_satisfied(
                &Action::TurnFlagship(TurnFlagshipStep {
                    direction,
                    facings,
                    max_ticks: 600,
                }),
                label,
            )
        };
        let verified = "flagship_turn_verified:direction=right:facings=4:from=2:to=6";
        assert!(satisfies(TurnDirection::Right, 4, verified));
        assert!(!satisfies(TurnDirection::Left, 4, verified));
        assert!(!satisfies(TurnDirection::Right, 5, verified));
        assert!(!satisfies(
            TurnDirection::Right,
            4,
            "flagship_turn_overshot:direction=right:facings=4:turned=5"
        ));
    }

    #[test]
    fn vault_proof_requires_fresh_unlock_consumption_and_the_vault_node() {
        let action =
            Action::CollectEnergyNode(super::super::story_actions::CollectEnergyNodeStep {
                max_ticks: 50,
            });
        let good = "energy_vault_unlocked:generation=3:node=0:vault_before=0:vault_after=1:shuttle_before=1:shuttle_after=0";
        assert!(semantic_satisfied(&action, good));
        for bad in [
            good.replace("generation=3", "generation=0"),
            good.replace("node=0", "node=1"),
            good.replace("vault_before=0", "vault_before=1"),
            good.replace("vault_after=1", "vault_after=0"),
            good.replace("shuttle_before=1", "shuttle_before=0"),
            good.replace("shuttle_after=0", "shuttle_after=1"),
        ] {
            assert!(!semantic_satisfied(&action, &bad), "{bad}");
        }
    }

    #[test]
    fn story_actions_require_identity_retrieval_and_verified_effects() {
        use super::super::story_actions::{CollectEnergyNodeStep, Device, UseDeviceStep};
        let device = Action::UseDevice(UseDeviceStep {
            device: Device::ClearSpindle,
            max_ticks: 50,
        });
        let state = super::super::story_actions::DEVICE_STORY_KEYS
            .iter()
            .copied()
            .chain(["fuel", "activity"])
            .map(|key| {
                let (before, after) = match key {
                    "ULTRON_CONDITION" => (1, 2),
                    "CLEAR_SPINDLE_ON_SHIP" => (1, 0),
                    _ => (0, 0),
                };
                format!("before_{key}={before},after_{key}={after}")
            })
            .collect::<Vec<_>>()
            .join(",");
        let good =
            format!("device_used:device=clear_spindle:generation=2:verified=1:state={state}");
        assert!(semantic_satisfied(&device, &good));
        assert!(!semantic_satisfied(
            &device,
            &good.replace("after_ULTRON_CONDITION=2", "after_ULTRON_CONDITION=1")
        ));
        for bad in [
            good.replace("verified=1", "verified=0"),
            good.replace("clear_spindle", "aqua_helix"),
            good.replace("generation=2", "generation=0"),
        ] {
            assert!(!semantic_satisfied(&device, &bad));
        }
        let node = Action::CollectEnergyNode(CollectEnergyNodeStep { max_ticks: 50 });
        let good =
            "energy_node_collected:generation=3:node=1:retrieved=2:flag=MOONBASE_ON_SHIP:value=1";
        assert!(semantic_satisfied(&node, good));
        for bad in [
            good.replace("retrieved=2", "retrieved=1"),
            good.replace("MOONBASE_ON_SHIP", "STARBASE_AVAILABLE"),
            good.replace("value=1", "value=0"),
        ] {
            assert!(!semantic_satisfied(&node, &bad));
        }
    }

    #[test]
    fn boss_checkpoints_require_their_own_milestone_and_complete_terminal_proof() {
        use super::super::samatra::{FightSamatraStep, Until};
        let action = |until| {
            Action::FightSamatra(FightSamatraStep {
                max_ticks: 900,
                until,
            })
        };
        let won = "samatra_won:generators_destroyed=8:gate_entered=1:escort=1:bomb=1:generation=1:ticks=800";
        assert!(wait_or_navigation_satisfied(
            &action(Until::GateEntered),
            won
        ));
        for bad in [
            won.replace("destroyed=8", "destroyed=7"),
            won.replace("bomb=1", "bomb=0"),
            won.replace("ticks=800", "ticks=901"),
            won.replace("generation=1", "generation=0"),
        ] {
            assert!(!wait_or_navigation_satisfied(
                &action(Until::GateEntered),
                &bad
            ));
        }
        assert!(!wait_or_navigation_satisfied(&action(Until::Ready), won));
        assert!(wait_or_navigation_satisfied(
            &action(Until::Ready),
            "samatra_ready:generators=8:generation=1"
        ));
        assert!(wait_or_navigation_satisfied(
            &action(Until::GateOpen),
            "samatra_gate_open:generators_destroyed=8:gate_open=1:generation=1"
        ));
        assert!(!wait_or_navigation_satisfied(
            &action(Until::GateEntered),
            "samatra_gate_open:generators_destroyed=8:gate_open=1:generation=1"
        ));
    }

    #[test]
    fn only_a_collected_label_of_the_same_category_and_minimum_satisfies_the_action() {
        use crate::automation::script::CollectPlanetMineralsStep;
        use crate::planet_side::cargo::ElementCategory;
        let action = Action::CollectPlanetMinerals(CollectPlanetMineralsStep {
            category: ElementCategory::Radioactive,
            minimum_units: 2,
            max_ticks: 900,
        });
        assert!(semantic_satisfied(
            &action,
            "planet_minerals_collected:category=radioactive:units=3:minimum=2"
        ));
        for label in [
            "planet_minerals_collected:category=common:units=3:minimum=2",
            "planet_minerals_collected:category=radioactive:units=1:minimum=2",
            "planet_minerals_collected:category=radioactive:units=3:minimum=1",
            "planet_minerals_collected:category=radioactive:units=3",
            "planet_minerals_collect_failed:category=radioactive:minimum=2:reason=timeout:units=0",
        ] {
            assert!(!semantic_satisfied(&action, label), "{label}");
        }
    }

    #[test]
    fn whole_fleet_proof_requires_empty_enemy_fleet_normal_speed_and_living_flagship() {
        let action =
            Action::FightEncounter(crate::automation::script::FightBattleStep { max_ticks: 900 });
        let good = "fleet_won:player_left=3:enemy_left=0:ticks=600:flagship_crew=46:enemy_initial=5:picks=3:speed=1";
        assert!(semantic_satisfied(&action, good));
        for bad in [
            good.replace("enemy_left=0", "enemy_left=1"),
            good.replace("speed=1", "speed=3"),
            good.replace("flagship_crew=46", "flagship_crew=65535"),
            good.replace("ticks=600", "ticks=901"),
            good.replace("enemy_initial=5", "enemy_initial=0"),
            good.replace("player_left=3", "player_left=0"),
        ] {
            assert!(!semantic_satisfied(&action, &bad), "{bad}");
        }
    }

    #[test]
    fn only_a_won_label_within_the_budget_with_a_surviving_player_satisfies_fight_battle() {
        use crate::automation::script::FightBattleStep;
        let action = Action::FightBattle(FightBattleStep { max_ticks: 900 });
        assert!(semantic_satisfied(
            &action,
            "battle_won:player_left=1:enemy_left=0:ticks=412:player_crew=14"
        ));
        assert!(semantic_satisfied(
            &action,
            "battle_won:player_left=2:enemy_left=0:ticks=900:player_crew=3"
        ));
        for label in [
            "battle_won:player_left=0:enemy_left=0:ticks=412:player_crew=14",
            "battle_won:player_left=1:enemy_left=0:ticks=412:player_crew=0",
            "battle_won:player_left=1:enemy_left=0:ticks=412",
            "battle_won:player_left=1:enemy_left=0:ticks=901:player_crew=14",
            "battle_won:player_left=1:enemy_left=0:ticks=0:player_crew=14",
            "battle_won:player_left=1:enemy_left=0",
            "battle_won:player_left=1:enemy_left=0:ticks=4:player_crew=14:extra=1",
            "battle_fight_failed:reason=timeout",
            "battle_fight_failed:reason=player_ship_lost",
            "battle_fight_failed:reason=not_in_battle",
        ] {
            assert!(!semantic_satisfied(&action, label), "{label}");
        }
    }

    #[test]
    fn only_a_verified_element_label_of_the_same_category_and_minimum_satisfies_the_action() {
        use crate::automation::script::ShipElementAssertion;
        use crate::planet_side::cargo::ElementCategory;
        let action = Action::AssertShipElement(ShipElementAssertion {
            category: ElementCategory::Radioactive,
            minimum: 1,
        });
        assert!(semantic_satisfied(
            &action,
            "ship_element_verified:category=radioactive:amount=4:minimum=1"
        ));
        for label in [
            "ship_element_verified:category=exotic:amount=4:minimum=1",
            "ship_element_verified:category=radioactive:amount=0:minimum=1",
            "ship_element_verified:category=radioactive:amount=4:minimum=2",
            "ship_element_failed:category=radioactive:amount=0:minimum=1",
            "ship_element_unavailable",
        ] {
            assert!(!semantic_satisfied(&action, label), "{label}");
        }
    }
}

#[cfg(test)]
mod contact_navigation_tests {
    #[test]
    fn contact_proof_requires_the_requested_race_not_a_planet_or_wrong_ship() {
        let action =
            super::Action::NavigateToContact(crate::automation::script::NavigateToContactStep {
                race: 12,
                max_ticks: 100,
            });
        assert!(super::semantic_satisfied(
            &action,
            "contact_reached:race=12"
        ));
        assert!(!super::semantic_satisfied(
            &action,
            "contact_reached:race=8"
        ));
        assert!(!super::semantic_satisfied(
            &action,
            "orbit_reached:planet=0"
        ));
    }
}
