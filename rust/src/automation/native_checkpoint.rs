//! Action identities and committed-presentation obligations for native autoplay.

use super::script::{Action, ValidatedScript};
use super::trace::{RecordKind, TraceRecord};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// The action family whose successful completion requires an OS checkpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckpointClass {
    Capture,
    Assertion,
    SemanticWait,
    Navigation,
}

/// Classify actions without consulting scenario names or observed results.
#[must_use]
pub fn checkpoint_class(action: &Action) -> Option<CheckpointClass> {
    match action {
        Action::Capture(_) => Some(CheckpointClass::Capture),
        Action::AssertActivity(_)
        | Action::AssertScene(_)
        | Action::AssertMode(_)
        | Action::AssertDispatch(_)
        | Action::AssertGameOptions(_)
        | Action::AssertCommunicationResponses(_)
        | Action::AssertBattleFrames(_)
        | Action::AssertPlanetSideCollisions(_)
        | Action::AssertMainMenuTransition(_) => Some(CheckpointClass::Assertion),
        Action::WaitForMainMenuReady(_)
        | Action::WaitForPlanetSideStart(_)
        | Action::WaitForPlanetSideEnd(_)
        | Action::WaitForDispatch(_)
        | Action::WaitForBattleFrames(_)
        | Action::WaitForCommunicationEnd(_)
        | Action::WaitForCommunicationReplay(_) => Some(CheckpointClass::SemanticWait),
        Action::NavigateToPlanet(_)
        | Action::NavigateToMoon(_)
        | Action::NavigateToOrbit(_)
        | Action::SelectPlanetMenu(_)
        | Action::SelectCommunicationResponse(_)
        | Action::SetupPlanetSideCollisionFixture(_) => Some(CheckpointClass::Navigation),
        Action::WaitInputTicks(_)
        | Action::WaitPresentations(_)
        | Action::SetMenuKey(_)
        | Action::TapMenuKey(_)
        | Action::SetPlayerKey(_)
        | Action::TapPlayerKey(_)
        | Action::Finish => None,
    }
}

/// An immutable obligation derived from the selected script, not from child output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeCheckpoint {
    pub action_index: usize,
    pub class: CheckpointClass,
    pub action: Action,
    pub id: String,
}

/// Complete selected source identity and ordered checkpoint obligations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeCheckpointPlan {
    pub source_sha256: String,
    pub resolved_sha256: String,
    pub checkpoints: Vec<NativeCheckpoint>,
    pub linked_floors: bool,
}

impl NativeCheckpointPlan {
    /// Bind both exact source bytes and the complete resolved behavior, including seed.
    pub fn derive(source: &[u8], script: &ValidatedScript) -> Result<Self, String> {
        let resolved = serde_json::to_vec(&script.resolved()).map_err(|error| error.to_string())?;
        let resolved_sha256 = format!("{:x}", Sha256::digest(resolved));
        let checkpoints = script
            .steps()
            .iter()
            .enumerate()
            .filter_map(|(index, action)| {
                checkpoint_class(action).map(|class| NativeCheckpoint {
                    action_index: index,
                    class,
                    action: action.clone(),
                    id: checkpoint_id(&resolved_sha256, index),
                })
            })
            .collect();
        Ok(Self {
            source_sha256: format!("{:x}", Sha256::digest(source)),
            resolved_sha256,
            checkpoints,
            // This additional proof is an explicit existing scenario obligation.
            // General actions never inherit its battle or presentation floors.
            linked_floors: script.name() == "linked-playable-v1",
        })
    }

    /// The previous script capture, excluding intervening semantic checkpoints.
    pub fn previous_capture_for_change(
        &self,
        action_index: usize,
    ) -> Result<Option<usize>, String> {
        let position = self
            .checkpoints
            .iter()
            .position(|item| item.action_index == action_index)
            .ok_or("capture action is outside the selected checkpoint plan")?;
        if !matches!(&self.checkpoints[position].action, Action::Capture(step) if step.expect_change)
        {
            return Ok(None);
        }
        self.checkpoints[..position]
            .iter()
            .rev()
            .find(|item| matches!(item.action, Action::Capture(_)))
            .map(|item| Some(item.action_index))
            .ok_or_else(|| "expect_change has no previous script capture".to_string())
    }

    /// Return obligations newly committed in this exact trace prefix. Unknown,
    /// duplicated or reordered markers fail before any acknowledgement is sent.
    pub fn committed<'a>(
        &'a self,
        records: &[TraceRecord],
    ) -> Result<Vec<&'a NativeCheckpoint>, String> {
        if records.iter().enumerate().any(|(index, record)| {
            record.sequence != index as u64
                || record.schema != TraceRecord::SCHEMA
                || records.first().is_some_and(|first| first.run != record.run)
        }) {
            return Err("native trace sequence, schema or run identity differs".into());
        }
        let mut committed = Vec::new();
        let mut previous_marker = 0;
        let mut previous_presentation = 0;
        let mut history = super::native_predicate::PredicateHistory::default();
        for (position, record) in records
            .iter()
            .enumerate()
            .filter(|(_, record)| record.kind == RecordKind::Checkpoint)
        {
            let Some(evidence) = &record.checkpoint else {
                return Err("native checkpoint has no typed identity".into());
            };
            let expected = self
                .checkpoints
                .get(committed.len())
                .ok_or("native checkpoint exceeds selected obligations")?;
            if evidence.id != expected.id || record.presentation.is_none() {
                return Err("native checkpoint action identity or committed frame differs".into());
            }
            let frame = record
                .presentation
                .as_ref()
                .ok_or("native checkpoint lacks frame")?;
            let (present_position, presented) = records[..position]
                .iter()
                .enumerate()
                .rev()
                .find(|(_, item)| item.kind == RecordKind::Presentation)
                .ok_or("native checkpoint has no committed presentation")?;
            if frame.count <= previous_presentation
                || frame.width == 0
                || frame.height == 0
                || present_position < previous_marker
                || presented.presentation.as_ref() != Some(frame)
            {
                return Err(
                    "native checkpoint reused or misidentified a committed presentation".into(),
                );
            }
            let evidence_range = if expected.class == CheckpointClass::Capture {
                present_position + 1..position
            } else {
                previous_marker..present_position
            };
            if !history.verify(&expected.action, &records[evidence_range], frame) {
                return Err(
                    "native checkpoint lacks its action's successful semantic/frame evidence"
                        .into(),
                );
            }
            previous_presentation = frame.count;
            previous_marker = position + 1;
            committed.push(expected);
        }
        Ok(committed)
    }
}

/// Stable identity shared by the scheduler barrier and the external controller.
#[must_use]
pub fn checkpoint_id(resolved_sha256: &str, index: usize) -> String {
    format!("native:{resolved_sha256}:{index}")
}

/// Bind every selected obligation to its first committed publication and OS image.
#[cfg(feature = "debug-process")]
pub fn validate_correlations(
    manifest: &super::native_window::NativeAcceptanceManifest,
    records: &[TraceRecord],
) -> Result<(), String> {
    use super::native_window::NativeScreenshotStage;
    let plan = manifest
        .window
        .checkpoint_plan
        .as_ref()
        .ok_or("native checkpoint plan is missing")?;
    let committed = plan.committed(records)?;
    if committed.len() != plan.checkpoints.len() {
        return Err("native run omitted selected checkpoint obligations".into());
    }
    let markers = records
        .iter()
        .filter(|record| record.kind == RecordKind::Checkpoint);
    for (obligation, marker) in committed.iter().zip(markers) {
        let first = manifest
            .publications
            .iter()
            .find(|publication| publication.state.semantic.trace_record_count > marker.sequence)
            .ok_or("checkpoint has no acknowledged publication")?;
        let stage = NativeScreenshotStage::Checkpoint {
            action_index: obligation.action_index,
        };
        let shots: Vec<_> = manifest
            .window
            .screenshots
            .iter()
            .filter(|shot| shot.stage == stage)
            .collect();
        if shots.len() != 1
            || shots[0].committed_presentation != first.state.committed_presentation
            || marker.presentation.as_ref().map(|frame| frame.count)
                != Some(shots[0].committed_presentation)
            || shots[0].trace_record_count != first.state.semantic.trace_record_count
        {
            return Err(
                "checkpoint image does not bind the first committed action publication".into(),
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn script(actions: serde_json::Value) -> (Vec<u8>, ValidatedScript) {
        let source = serde_json::to_vec(&serde_json::json!({
            "version": 1, "name": "checkpoint-test", "budgets": {
                "max_input_ticks": 1000, "max_presentations": 1000, "max_wallclock_seconds": 60
            }, "steps": actions
        }))
        .unwrap();
        let doc = super::super::script::parse_script(&source, "test.json").unwrap();
        let script = super::super::script::validate_script(doc, "test.json").unwrap();
        (source, script)
    }

    fn record(kind: RecordKind, label: &str, sequence: u64) -> TraceRecord {
        let mut record = super::super::capture::capture_trace_record(
            sequence,
            0,
            super::super::scheduler::CaptureGeneration(9),
            label,
        );
        record.kind = kind;
        record.label = Some(label.into());
        record
    }

    fn committed_trace(plan: &NativeCheckpointPlan, label: &str) -> Vec<TraceRecord> {
        let frame = super::super::trace::PresentationEvidence {
            count: 7,
            generation: 9,
            width: 320,
            height: 240,
        };
        let obligation = &plan.checkpoints[0];
        let mut evidence = record(RecordKind::SemanticAssertion, label, 0);
        if let Action::Capture(step) = &obligation.action {
            evidence.kind = RecordKind::Capture;
            evidence.label = Some(format!("{}_gen9", step.label));
            evidence.presentation = Some(frame.clone());
        }
        let mut present = record(RecordKind::Presentation, "", 1);
        present.presentation = Some(frame.clone());
        let mut checkpoint = record(RecordKind::Checkpoint, "", 2);
        checkpoint.presentation = Some(frame);
        checkpoint.checkpoint = Some(super::super::trace::CheckpointEvidence {
            id: obligation.id.clone(),
        });
        if obligation.class == CheckpointClass::Capture {
            present.sequence = 0;
            evidence.sequence = 1;
            vec![present, evidence, checkpoint]
        } else {
            vec![evidence, present, checkpoint]
        }
    }

    #[test]
    fn native_checkpoints_reject_reused_commits_and_post_present_assertions() {
        let (source, script) = script(serde_json::json!([
            {"action":"assert_battle_frames","minimum":4},
            {"action":"assert_battle_frames","minimum":4},
            {"action":"finish"}
        ]));
        let plan = NativeCheckpointPlan::derive(&source, &script).unwrap();
        let mut trace = committed_trace(&plan, "battle_frames_verified:count=4");
        assert!(plan.committed(&trace).is_ok());
        let mut second = trace[2].clone();
        second.sequence = 4;
        second.checkpoint.as_mut().unwrap().id = plan.checkpoints[1].id.clone();
        trace.push(record(
            RecordKind::SemanticAssertion,
            "battle_frames_verified:count=4",
            3,
        ));
        trace.push(second);
        assert!(
            plan.committed(&trace).is_err(),
            "two obligations reused one committed frame"
        );
        let mut trace = committed_trace(&plan, "battle_frames_verified:count=4");
        trace.swap(0, 1);
        for (index, record) in trace.iter_mut().enumerate() {
            record.sequence = index as u64;
        }
        assert!(
            plan.committed(&trace).is_err(),
            "assertion happened after its purported commit"
        );
    }

    #[test]
    fn checkpoint_classes_require_successful_predicates_at_the_committed_frame() {
        for (action, label, class) in [
            (
                serde_json::json!({"action":"capture", "label":"menu"}),
                "",
                CheckpointClass::Capture,
            ),
            (
                serde_json::json!({"action":"assert_battle_frames", "minimum":30}),
                "battle_frames_verified:count=30",
                CheckpointClass::Assertion,
            ),
            (
                serde_json::json!({"action":"wait_for_battle_frames", "minimum":30, "max_ticks":40}),
                "battle_frames_reached:count=30:minimum=30",
                CheckpointClass::SemanticWait,
            ),
            (
                serde_json::json!({"action":"navigate_to_planet", "planet":2, "max_ticks":40}),
                "navigation_reached:planet=2",
                CheckpointClass::Navigation,
            ),
        ] {
            let (source, script) = script(serde_json::json!([action, {"action":"finish"}]));
            let plan = NativeCheckpointPlan::derive(&source, &script).unwrap();
            assert_eq!(plan.checkpoints[0].class, class);
            assert!(!plan.linked_floors);
            let trace = committed_trace(&plan, label);
            assert_eq!(plan.committed(&trace).unwrap().len(), 1);
            let mut changed = trace.clone();
            let evidence_index = usize::from(class == CheckpointClass::Capture);
            changed[evidence_index].label = Some("unrelated".into());
            assert!(plan.committed(&changed).is_err());
            let mut changed = trace.clone();
            changed.swap(0, 1);
            for (index, record) in changed.iter_mut().enumerate() {
                record.sequence = index as u64;
            }
            assert!(plan.committed(&changed).is_err());
            let mut changed = trace.clone();
            changed[2].presentation.as_mut().unwrap().count += 1;
            assert!(plan.committed(&changed).is_err());
            let mut changed = trace.clone();
            changed[2].checkpoint.as_mut().unwrap().id.push('0');
            assert!(plan.committed(&changed).is_err());
            let mut duplicate = trace.clone();
            duplicate.push(trace[2].clone());
            assert!(plan.committed(&duplicate).is_err());
        }
    }

    #[test]
    fn checkpoint_identity_binds_source_seed_and_action_order() {
        let (source, script) = script(serde_json::json!([
            {"action":"capture", "label":"one"}, {"action":"capture", "label":"two"}, {"action":"finish"}
        ]));
        let plan = NativeCheckpointPlan::derive(&source, &script).unwrap();
        let mut whitespace = source.clone();
        whitespace.push(b' ');
        let reformatted = NativeCheckpointPlan::derive(&whitespace, &script).unwrap();
        assert_ne!(plan.source_sha256, reformatted.source_sha256);
        assert_eq!(plan.checkpoints, reformatted.checkpoints);
        let mut seeded = script.clone();
        seeded.seed += 1;
        assert_ne!(
            plan.checkpoints[0].id,
            NativeCheckpointPlan::derive(&source, &seeded)
                .unwrap()
                .checkpoints[0]
                .id
        );
        let mut reordered = script.clone();
        reordered.steps.swap(0, 1);
        assert_ne!(
            plan.checkpoints,
            NativeCheckpointPlan::derive(&source, &reordered)
                .unwrap()
                .checkpoints
        );
        let mut trace = committed_trace(&plan, "");
        trace[2].checkpoint.as_mut().unwrap().id = plan.checkpoints[1].id.clone();
        assert!(plan.committed(&trace).is_err());
    }

    #[test]
    fn every_existing_capture_and_semantic_family_is_derived_without_rewriting_scripts() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts");
        let mut captures = 0;
        let mut classes = std::collections::BTreeSet::new();
        for entry in std::fs::read_dir(root).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            let source = std::fs::read(&path).unwrap();
            let document = super::super::script::parse_script(&source, &path).unwrap();
            let script = super::super::script::validate_script(document, &path).unwrap();
            let plan = NativeCheckpointPlan::derive(&source, &script).unwrap();
            let expected_captures = script
                .steps()
                .iter()
                .filter(|action| matches!(action, Action::Capture(_)))
                .count();
            assert_eq!(
                plan.checkpoints
                    .iter()
                    .filter(|item| item.class == CheckpointClass::Capture)
                    .count(),
                expected_captures,
                "{}",
                path.display()
            );
            for item in &plan.checkpoints {
                assert_eq!(&item.action, &script.steps()[item.action_index]);
                classes.insert(format!("{:?}", item.class));
            }
            if super::super::suite::required_suite()
                .contains(&path.file_stem().unwrap().to_str().unwrap())
            {
                captures += expected_captures;
            }
        }
        assert_eq!(captures, 132);
        assert_eq!(classes.len(), 4);
    }
}
