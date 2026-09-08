use super::*;

pub(super) fn binding(event: WorkflowEvent) -> SelectionBinding {
    let pr = event == WorkflowEvent::PullRequestTarget;
    let paths = ["rust/src/battle/x.rs".to_string()];
    SelectionBinding {
        schema: "uqm-s4-selection-v1".into(),
        event,
        source_sha: "a".repeat(40),
        controller_sha: "b".repeat(40),
        base_sha: pr.then(|| "c".repeat(40)),
        merge_base_sha: pr.then(|| "d".repeat(40)),
        changed_paths_z: if pr {
            b"rust/src/battle/x.rs\0".to_vec()
        } else {
            Vec::new()
        },
        authority_sha256: super::super::evidence::hex_sha256(include_bytes!(
            "../../../ci/gates.json"
        )),
        autoplay: event_selection(event, pr.then_some(paths.as_slice())),
    }
}

#[test]
fn successful_full_and_pr_selection_require_exact_ordered_original_pins() {
    let bytes = include_bytes!("../../../ci/gates.json");
    let authority: Authority = serde_json::from_slice(bytes).unwrap();
    for event in [
        WorkflowEvent::PullRequestTarget,
        WorkflowEvent::Push,
        WorkflowEvent::Schedule,
        WorkflowEvent::WorkflowDispatch,
    ] {
        let binding = binding(event);
        let selected = binding
            .selected(&authority, &"a".repeat(40), bytes)
            .unwrap();
        assert_eq!(
            selected.len(),
            if event == WorkflowEvent::PullRequestTarget {
                2
            } else {
                32
            }
        );
        assert!(selected
            .iter()
            .all(|pin| authority.native_acceptance.scenario_scripts.contains(pin)));
        for mutation in 0..9 {
            let mut forged = binding.clone();
            match mutation {
                0 => {
                    forged.autoplay.scenarios.pop();
                }
                1 => {
                    forged
                        .autoplay
                        .scenarios
                        .push(forged.autoplay.scenarios[0].clone());
                }
                2 => forged.autoplay.scenarios.swap(0, 1),
                3 => forged.autoplay.scenarios[0] = "unknown".into(),
                4 => forged.source_sha = "e".repeat(40),
                5 => forged.controller_sha = "not-a-revision".into(),
                6 => forged.authority_sha256 = "0".repeat(64),
                7 => forged.autoplay.policy = "untrusted".into(),
                8 => forged.changed_paths_z = b"narrowed".to_vec(),
                _ => unreachable!(),
            }
            assert!(
                forged.selected(&authority, &"a".repeat(40), bytes).is_err(),
                "{event:?} mutation {mutation}"
            );
        }
    }
}

#[test]
fn rehashed_event_spoof_is_rejected_by_workflow_binding() {
    let genuine = binding(WorkflowEvent::Schedule);
    let candidate = binding(WorkflowEvent::PullRequestTarget);
    let policy = include_bytes!("../../../ci/gates.json");
    let authority: Authority = serde_json::from_slice(policy).unwrap();
    // Internally valid PR proof still cannot answer a scheduled workflow.
    assert!(candidate
        .selected(&authority, &genuine.source_sha, policy)
        .is_ok());
    assert!(candidate
        .bind_event(
            genuine.event,
            &genuine.source_sha,
            &genuine.controller_sha,
            None
        )
        .is_err());
    assert!(genuine
        .bind_event(
            genuine.event,
            &genuine.source_sha,
            &genuine.controller_sha,
            None
        )
        .is_ok());
    assert!(genuine
        .bind_event(genuine.event, &genuine.source_sha, &"e".repeat(40), None)
        .is_err());
    assert!(candidate
        .bind_event(
            candidate.event,
            &candidate.source_sha,
            &candidate.controller_sha,
            Some(&"e".repeat(40))
        )
        .is_err());
}

#[test]
fn non_utf8_empty_and_unmapped_diffs_preserve_full_coverage() {
    let policy = include_bytes!("../../../ci/gates.json");
    let authority: Authority = serde_json::from_slice(policy).unwrap();
    for paths in [
        vec![255, 0],
        Vec::new(),
        b"rust/src/battle/x.rs\0unknown\0".to_vec(),
        b" rust/src/battle/x.rs\0".to_vec(),
    ] {
        let mut bound = binding(WorkflowEvent::PullRequestTarget);
        bound.changed_paths_z = paths;
        let decoded = if bound.changed_paths_z.is_empty() {
            Some(Vec::new())
        } else {
            bound.changed_paths_z[..bound.changed_paths_z.len() - 1]
                .split(|b| *b == 0)
                .map(|p| std::str::from_utf8(p).map(str::to_owned))
                .collect::<Result<Vec<_>, _>>()
                .ok()
        };
        bound.autoplay = event_selection(bound.event, decoded.as_deref());
        assert_eq!(
            bound
                .selected(&authority, &bound.source_sha, policy)
                .unwrap()
                .len(),
            32
        );
    }
}

#[test]
fn policy_admission_rejects_removal_reordering_pin_changes_and_capability_changes() {
    let policy = include_bytes!("../../../ci/gates.json");
    let original: serde_json::Value = serde_json::from_slice(policy).unwrap();
    for mutation in 0..7 {
        let mut candidate = original.clone();
        match mutation {
            0 => {
                candidate["native_acceptance"]["scenario_scripts"]
                    .as_array_mut()
                    .unwrap()
                    .pop();
            }
            1 => candidate["native_acceptance"]["scenario_scripts"]
                .as_array_mut()
                .unwrap()
                .swap(0, 1),
            2 => {
                candidate["native_acceptance"]["scenario_scripts"][0]["sha256"] =
                    serde_json::json!("0".repeat(64))
            }
            3 => {
                let pin = candidate["native_acceptance"]["scenario_scripts"][0].clone();
                candidate["native_acceptance"]["scenario_scripts"]
                    .as_array_mut()
                    .unwrap()
                    .push(pin);
            }
            4 => candidate["gates"][0]["steps"][0]["command"] = serde_json::json!(["true"]),
            5 => candidate["tools"]["rust"]["version"] = serde_json::json!("untrusted"),
            6 => candidate["control_plane_paths"] = serde_json::json!([]),
            _ => unreachable!(),
        }
        assert!(
            admit_policy(policy, &serde_json::to_vec(&candidate).unwrap()).is_err(),
            "mutation {mutation}"
        );
    }
    admit_policy(policy, &serde_json::to_vec_pretty(&original).unwrap()).unwrap();
}

#[test]
fn rejected_migration_never_replaces_the_installed_policy() {
    let temporary = tempfile::tempdir().unwrap();
    let base = temporary.path().join("base.json");
    let candidate = temporary.path().join("candidate.json");
    let output = temporary.path().join("authority.json");
    let original = include_bytes!("../../../ci/gates.json");
    std::fs::write(&base, original).unwrap();
    std::fs::write(&candidate, b"{}").unwrap();
    std::fs::write(&output, original).unwrap();
    assert!(admit_command(&base, &candidate, &output).is_err());
    assert_eq!(std::fs::read(&output).unwrap(), original);
    std::fs::write(&candidate, original).unwrap();
    admit_command(&base, &candidate, &output).unwrap();
    assert_eq!(std::fs::read(&output).unwrap(), original);
}

#[test]
fn singleton_base_policy_migrates_without_replacing_its_original_script_pin() {
    let candidate = include_bytes!("../../../ci/gates.json");
    let mut base: serde_json::Value = serde_json::from_slice(candidate).unwrap();
    base["native_acceptance"]
        .as_object_mut()
        .unwrap()
        .remove("scenario_scripts");
    base["mutation_targets"]
        .as_array_mut()
        .unwrap()
        .retain(|v| v != "autoplay");
    let base_bytes = serde_json::to_vec(&base).unwrap();
    admit_policy(&base_bytes, candidate).unwrap();
    let mut forged: serde_json::Value = serde_json::from_slice(candidate).unwrap();
    let legacy = forged["native_acceptance"]["script"].clone();
    forged["native_acceptance"]["scenario_scripts"]
        .as_array_mut()
        .unwrap()
        .retain(|v| v["path"] != legacy);
    assert!(admit_policy(&base_bytes, &serde_json::to_vec(&forged).unwrap()).is_err());
}
