//! Scene assertion handling on the automation owner.

use super::*;

impl AutomationObserver {
    pub(super) fn verify_scene_assertion(&self, inner: &mut CoordInner) -> bool {
        if let Some(Action::AssertScene(assertion)) = self.actions.get(inner.sched_state.step_index)
        {
            match self.scene.lock().verify(assertion.scene) {
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

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automation::observer::Handle;
    use crate::automation::scenario::{AutomationScene, SceneError};
    use crate::engine::observe::{CheckpointView, Observer};

    fn observer() -> (tempfile::TempDir, Handle) {
        let source = br#"{"version":1,"name":"scene-assertion","steps":[{"action":"assert_scene","scene":"sol_probe_encounter"},{"action":"finish"}],"budgets":{"max_input_ticks":10,"max_presentations":10,"max_wallclock_seconds":60}}"#;
        let script = crate::automation::validate_script(
            crate::automation::parse_script(source, "test.json").unwrap(),
            "test.json",
        )
        .unwrap();
        let output = tempfile::tempdir().unwrap();
        let owner = AutomationObserver::new(
            script,
            output.path().into(),
            crate::engine::World::new().activity_control(),
            Default::default(),
            Default::default(),
        );
        (output, Handle(owner))
    }

    fn labels(owner: &AutomationObserver) -> Vec<String> {
        let mut bytes = Vec::new();
        owner.runtime.commit.publish_all(&mut bytes).unwrap();
        String::from_utf8(bytes)
            .unwrap()
            .lines()
            .filter_map(|line| {
                let record: serde_json::Value = serde_json::from_str(line).unwrap();
                record["label"].as_str().map(str::to_owned)
            })
            .collect()
    }

    #[test]
    fn scene_assertion_records_the_owned_dispatch_and_ignores_other_actions() {
        let (_output, mut handle) = observer();
        handle
            .0
            .scene
            .lock()
            .begin(AutomationScene::SolProbeEncounter);
        handle.checkpoint(&CheckpointView::EncounterDispatch(24));
        handle.checkpoint(&CheckpointView::DialogueDispatch(18));
        let mut inner = handle.0.inner.lock();
        assert!(!handle.0.verify_scene_assertion(&mut inner));
        assert_eq!(inner.terminal_class, None);
        assert_eq!(
            labels(&handle.0),
            ["scene_verified:sol_probe_encounter:encounter=24:dialogue=18"]
        );
        inner.sched_state.step_index = 1;
        assert!(!handle.0.verify_scene_assertion(&mut inner));
        inner.sched_state.step_index = 2;
        assert!(!handle.0.verify_scene_assertion(&mut inner));
        assert!(labels(&handle.0).is_empty());
        assert_eq!(
            handle.0.scene.lock().dispatch_observation(),
            (1, Some(24), Some(18))
        );
    }

    #[test]
    fn scene_assertion_stops_on_each_missing_or_mismatched_observation() {
        use AutomationScene::{SolProbeEncounter, StarbaseCommander};
        use CheckpointView::{DialogueDispatch, EncounterDispatch};
        let cases: &[(Option<AutomationScene>, &[CheckpointView<'_>], SceneError)] = &[
            (None, &[], SceneError::NoActiveScene),
            (
                Some(StarbaseCommander),
                &[],
                SceneError::WrongActiveScene {
                    expected: SolProbeEncounter,
                    actual: StarbaseCommander,
                },
            ),
            (
                Some(SolProbeEncounter),
                &[],
                SceneError::EncounterNotDispatched,
            ),
            (
                Some(SolProbeEncounter),
                &[EncounterDispatch(0)],
                SceneError::WrongEncounterConversation {
                    expected: 24,
                    actual: 0,
                },
            ),
            (
                Some(SolProbeEncounter),
                &[EncounterDispatch(24)],
                SceneError::DialogueNotDispatched,
            ),
            (
                Some(SolProbeEncounter),
                &[EncounterDispatch(24), DialogueDispatch(0)],
                SceneError::WrongDialogueConversation {
                    expected: 18,
                    actual: 0,
                },
            ),
        ];
        for (scene, events, expected) in cases {
            let (_output, mut handle) = observer();
            if let Some(scene) = scene {
                handle.0.scene.lock().begin(*scene);
            }
            for event in *events {
                handle.checkpoint(event);
            }
            let mut inner = handle.0.inner.lock();
            assert!(handle.0.verify_scene_assertion(&mut inner), "{expected}");
            assert_eq!(inner.terminal_class, Some(TerminalClass::SemanticMismatch));
            assert_eq!(labels(&handle.0), [expected.to_string()]);
        }
    }
}
