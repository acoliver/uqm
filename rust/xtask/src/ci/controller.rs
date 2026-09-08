//! Base-owned admission of candidate policy and event-bound autoplay selection.

use super::authority::{Authority, PinnedScript};
use super::plan::{derive_autoplay, AutoplayPlan};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowEvent {
    PullRequestTarget,
    Push,
    Schedule,
    WorkflowDispatch,
    Local,
}

/// Only a PR diff may narrow the required suite.
pub fn event_selection(event: WorkflowEvent, changed: Option<&[String]>) -> AutoplayPlan {
    derive_autoplay(if event == WorkflowEvent::PullRequestTarget {
        changed
    } else {
        None
    })
}

/// Candidate data cannot grant permission to change controller code or gate limits.
pub fn admit_policy(base: &[u8], candidate: &[u8]) -> Result<(), String> {
    let base: serde_json::Value = serde_json::from_slice(base).map_err(|e| e.to_string())?;
    let candidate: serde_json::Value =
        serde_json::from_slice(candidate).map_err(|e| e.to_string())?;
    let typed: Authority = serde_json::from_value(candidate.clone()).map_err(|e| e.to_string())?;
    super::authority::validate_authority(&typed)?;
    let installed: Authority = serde_json::from_slice(include_bytes!("../../../ci/gates.json"))
        .map_err(|e| e.to_string())?;
    for pointer in ["/mutation_targets", "/native_acceptance/scenario_scripts"] {
        let legacy = vec![serde_json::json!({
            "path": base["native_acceptance"]["script"],
            "sha256": base["native_acceptance"]["script_sha256"],
            "byte_length": base["native_acceptance"]["script_byte_length"],
        })];
        let old = match base.pointer(pointer) {
            Some(value) => value
                .as_array()
                .ok_or("base policy inventory is not an array")?,
            None if pointer == "/native_acceptance/scenario_scripts" => &legacy,
            None => return Err("base mutation inventory missing".into()),
        };
        let new = candidate
            .pointer(pointer)
            .and_then(|v| v.as_array())
            .ok_or("candidate policy inventory missing")?;
        let mut cursor = 0;
        for item in new {
            if old.get(cursor) == Some(item) {
                cursor += 1;
            } else {
                let permitted = if pointer == "/mutation_targets" {
                    item == "autoplay"
                } else {
                    serde_json::from_value::<PinnedScript>(item.clone()).is_ok_and(|pin| {
                        installed.native_acceptance.scenario_scripts.contains(&pin)
                    })
                };
                if !permitted || old.contains(item) {
                    return Err(format!(
                        "candidate policy has an unapproved addition or reordering at {pointer}"
                    ));
                }
            }
        }
        if cursor != old.len() {
            return Err(format!(
                "candidate policy removes or changes a base entry at {pointer}"
            ));
        }
    }
    let mut comparable = candidate;
    comparable["mutation_targets"] = base["mutation_targets"].clone();
    if let Some(inventory) = base.pointer("/native_acceptance/scenario_scripts") {
        comparable["native_acceptance"]["scenario_scripts"] = inventory.clone();
    } else {
        comparable["native_acceptance"]
            .as_object_mut()
            .ok_or("candidate native policy missing")?
            .remove("scenario_scripts");
    }
    if comparable != base {
        return Err("candidate policy changes a field outside base-owned admission rules".into());
    }
    Ok(())
}
pub fn retained_selection(
    root: &Path,
    authority: &Authority,
    source: &str,
) -> Result<Vec<PinnedScript>, String> {
    let policy =
        super::evidence::read_regular_relative(root, "payloads/authority.snapshot/gates.json")
            .map_err(|e| e.to_string())?;
    let receipt = super::evidence::read_regular_relative(
        root,
        "payloads/preflight.source/source-preflight.json",
    )
    .map_err(|e| e.to_string())?;
    let receipt: serde_json::Value = serde_json::from_slice(&receipt).map_err(|e| e.to_string())?;
    let binding: SelectionBinding =
        serde_json::from_value(receipt["selection"].clone()).map_err(|e| e.to_string())?;
    binding.bind_environment()?;
    binding.selected(authority, source, &policy)
}

/// Retained controller input, not a candidate's requested scenario list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectionBinding {
    pub schema: String,
    pub event: WorkflowEvent,
    pub source_sha: String,
    pub controller_sha: String,
    pub base_sha: Option<String>,
    pub merge_base_sha: Option<String>,
    pub changed_paths_z: Vec<u8>,
    pub authority_sha256: String,
    pub autoplay: AutoplayPlan,
}

impl SelectionBinding {
    pub fn selected(
        &self,
        authority: &Authority,
        source: &str,
        policy: &[u8],
    ) -> Result<Vec<PinnedScript>, String> {
        if self.schema != "uqm-s4-selection-v1"
            || self.source_sha != source
            || !revision(&self.source_sha)
            || !revision(&self.controller_sha)
            || self.authority_sha256 != super::evidence::hex_sha256(policy)
        {
            return Err("selection source/controller/policy identity mismatch".into());
        }
        admit_policy(include_bytes!("../../../ci/gates.json"), policy)?;
        let pr = self.event == WorkflowEvent::PullRequestTarget;
        if pr != self.base_sha.is_some()
            || pr != self.merge_base_sha.is_some()
            || self.base_sha.as_deref().is_some_and(|v| !revision(v))
            || self.merge_base_sha.as_deref().is_some_and(|v| !revision(v))
            || (!pr && !self.changed_paths_z.is_empty())
            || self.changed_paths_z.len() > 1024 * 1024
            || (!self.changed_paths_z.is_empty() && self.changed_paths_z.last() != Some(&0))
        {
            return Err("selection event/diff binding mismatch".into());
        }
        let paths = if self.changed_paths_z.is_empty() {
            Some(Vec::new())
        } else {
            self.changed_paths_z[..self.changed_paths_z.len() - 1]
                .split(|b| *b == 0)
                .map(|p| std::str::from_utf8(p).map(str::to_owned))
                .collect::<Result<Vec<_>, _>>()
                .ok()
        };
        let expected = event_selection(self.event, paths.as_deref());
        if self.autoplay != expected {
            return Err("selection does not equal the event-required suite".into());
        }
        expected
            .scenarios
            .iter()
            .map(|name| {
                authority
                    .native_acceptance
                    .scenario_scripts
                    .iter()
                    .find(|pin| pin.path == format!("rust/scripts/{name}.json"))
                    .cloned()
                    .ok_or_else(|| format!("required scenario {name} is not pinned"))
            })
            .collect()
    }

    pub fn bind_environment(&self) -> Result<(), String> {
        if std::env::var_os("UQM_CI_EVENT_NAME").is_some() {
            let event = workflow_event()?;
            let controller = std::env::var("UQM_CI_CONTROLLER_SHA").map_err(|e| e.to_string())?;
            let expected = std::env::var("UQM_CI_EXPECTED_SHA").map_err(|e| e.to_string())?;
            let base = if event == WorkflowEvent::PullRequestTarget {
                Some(std::env::var("UQM_CI_EVENT_BASE_SHA").map_err(|e| e.to_string())?)
            } else {
                None
            };
            self.bind_event(event, &expected, &controller, base.as_deref())?;
        }
        Ok(())
    }

    /// Compare with workflow-owned inputs at gate entry or hosted validation.
    pub fn bind_event(
        &self,
        event: WorkflowEvent,
        source: &str,
        controller: &str,
        base: Option<&str>,
    ) -> Result<(), String> {
        if self.event != event
            || self.source_sha != source
            || self.controller_sha != controller
            || self.base_sha.as_deref() != base
        {
            return Err("retained selection contradicts trusted workflow event".into());
        }
        Ok(())
    }
}

fn revision(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn git(root: &Path, authority: &Authority, arguments: &[&str]) -> Result<Vec<u8>, String> {
    let mut args = vec!["-c".into(), format!("safe.directory={}", root.display())];
    args.extend(arguments.iter().map(|v| (*v).to_owned()));
    let captured = super::exec::run_captured_with_limits(
        root,
        "git",
        &args,
        &[],
        authority.supervision.builtin_limits(),
    );
    if !captured.completed_under_supervision() || captured.exit_code != Some(0) {
        return Err(format!(
            "controller selection git failed: {}",
            captured.failure_detail("git")
        ));
    }
    Ok(captured.stdout)
}

pub fn workflow_event() -> Result<WorkflowEvent, String> {
    match std::env::var("UQM_CI_EVENT_NAME").as_deref() {
        Ok("pull_request_target") => Ok(WorkflowEvent::PullRequestTarget),
        Ok("push") => Ok(WorkflowEvent::Push),
        Ok("schedule") => Ok(WorkflowEvent::Schedule),
        Ok("workflow_dispatch") => Ok(WorkflowEvent::WorkflowDispatch),
        Err(std::env::VarError::NotPresent) if std::env::var_os("UQM_CI_SOURCE_ROOT").is_none() => {
            Ok(WorkflowEvent::Local)
        }
        _ => Err("missing or unsupported trusted workflow event".into()),
    }
}

pub fn derive_binding(
    root: &Path,
    authority: &Authority,
    policy: &[u8],
) -> Result<SelectionBinding, String> {
    let event = workflow_event()?;
    let source = String::from_utf8(git(root, authority, &["rev-parse", "HEAD"])?)
        .map_err(|e| e.to_string())?
        .trim()
        .to_owned();
    let controller = if event == WorkflowEvent::Local {
        source.clone()
    } else {
        std::env::var("UQM_CI_CONTROLLER_SHA").map_err(|e| e.to_string())?
    };
    if event != WorkflowEvent::Local
        && std::env::var("UQM_CI_EXPECTED_SHA").ok().as_deref() != Some(&source)
    {
        return Err("workflow source differs from checkout".into());
    }
    let mut binding = SelectionBinding {
        schema: "uqm-s4-selection-v1".into(),
        event,
        source_sha: source,
        controller_sha: controller,
        base_sha: None,
        merge_base_sha: None,
        changed_paths_z: Vec::new(),
        authority_sha256: super::evidence::hex_sha256(policy),
        autoplay: event_selection(event, None),
    };
    if event == WorkflowEvent::PullRequestTarget {
        let base = std::env::var("UQM_CI_EVENT_BASE_SHA").map_err(|e| e.to_string())?;
        if !revision(&base) {
            return Err("invalid workflow PR base revision".into());
        }
        let merge = String::from_utf8(git(
            root,
            authority,
            &["merge-base", &base, &binding.source_sha],
        )?)
        .map_err(|e| e.to_string())?
        .trim()
        .to_owned();
        binding.changed_paths_z = git(
            root,
            authority,
            &[
                "diff",
                "--no-ext-diff",
                "--no-textconv",
                "--name-only",
                "-z",
                "--no-renames",
                &merge,
                &binding.source_sha,
                "--",
            ],
        )?;
        binding.base_sha = Some(base);
        binding.merge_base_sha = Some(merge);
        let paths = binding
            .changed_paths_z
            .split(|b| *b == 0)
            .filter(|p| !p.is_empty())
            .map(|p| std::str::from_utf8(p).map(str::to_owned))
            .collect::<Result<Vec<_>, _>>()
            .ok();
        binding.autoplay = event_selection(event, paths.as_deref());
    }
    binding.selected(authority, &binding.source_sha, policy)?;
    Ok(binding)
}

pub fn admit_command(base: &Path, candidate: &Path, output: &Path) -> Result<(), String> {
    let limit = super::bounded_io::AUTHORITY_BOOTSTRAP_LIMIT_BYTES;
    let base_bytes = super::bounded_io::read_regular_nofollow(base, limit)?;
    let candidate_bytes = super::bounded_io::read_regular_nofollow(candidate, limit)?;
    admit_policy(&base_bytes, &candidate_bytes)?;
    let parent = output.parent().ok_or("admitted policy has no parent")?;
    let filename = output
        .file_name()
        .and_then(|v| v.to_str())
        .ok_or("invalid admitted policy filename")?;
    super::evidence::EvidencePublisher::open(parent)
        .map_err(|e| e.to_string())?
        .replace(filename, &candidate_bytes)
        .map_err(|e| e.to_string())?;
    println!(
        "{}",
        serde_json::to_string(&serde_json::json!({
            "schema": "uqm-s4-policy-admission-v1",
            "base_policy_sha256": super::evidence::hex_sha256(&base_bytes),
            "candidate_policy_sha256": super::evidence::hex_sha256(&candidate_bytes),
            "controller_sha": std::env::var("UQM_CI_CONTROLLER_SHA").ok(),
            "source_sha": std::env::var("UQM_CI_EXPECTED_SHA").ok(),
            "decision": "admitted",
            "rules": "preserve-base-plus-installed-script-pins-and-autoplay-mutation",
        }))
        .map_err(|e| e.to_string())?
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schedule_push_and_dispatch_cannot_spoof_a_pr_subset() {
        let paths = ["rust/src/battle/x.rs".into()];
        let full = derive_autoplay(None).scenarios;
        for event in [
            WorkflowEvent::Schedule,
            WorkflowEvent::Push,
            WorkflowEvent::WorkflowDispatch,
            WorkflowEvent::Local,
        ] {
            assert_eq!(event_selection(event, Some(&paths)).scenarios, full);
        }
        let pr = event_selection(WorkflowEvent::PullRequestTarget, Some(&paths));
        assert!(pr.scenarios.len() < full.len());
        assert_eq!(
            event_selection(WorkflowEvent::PullRequestTarget, Some(&["unmapped".into()])).scenarios,
            full
        );
    }

    #[test]
    fn base_rules_admit_known_additions_without_authorizing_other_changes() {
        let candidate = include_bytes!("../../../ci/gates.json");
        let mut base: serde_json::Value = serde_json::from_slice(candidate).unwrap();
        base["mutation_targets"]
            .as_array_mut()
            .unwrap()
            .retain(|v| v != "autoplay");
        base["native_acceptance"]["scenario_scripts"]
            .as_array_mut()
            .unwrap()
            .truncate(1);
        let base = serde_json::to_vec(&base).unwrap();
        admit_policy(&base, candidate).unwrap();
        let mut forged: serde_json::Value = serde_json::from_slice(candidate).unwrap();
        forged["actions"]["evidence_snapshot_aggregate_limit_bytes"] =
            serde_json::json!(9999999999u64);
        assert!(admit_policy(&base, &serde_json::to_vec(&forged).unwrap()).is_err());
    }
}

#[cfg(test)]
#[path = "controller_tests.rs"]
mod regressions;
