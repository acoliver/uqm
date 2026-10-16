//! `ci plan`: derive the four-tuple execution plan from the CI authority.
//!
//! `rust/ci/gates.json` owns tuple and runner identity. The supported-matrix file is
//! accepted only when its compatibility rows derive exactly that authority set.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::authority::{
    derive_supported_tuples, load_authority_contract, Matrix, RunnerMapping, AUTHORITY_RELATIVE,
};
use super::CiError;

pub const PLAN_SCHEMA: &str = "uqm-s4-plan-v1";
pub const PLAN_RELATIVE: &str = "rust/target/ci-plan.json";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PlanTuple {
    pub os: String,
    pub architecture: String,
    pub tuple: String,
    pub runner: String,
    pub expected_uname: String,
}

/// Which autoplay scenarios this run must prove, and why that set.
///
/// The plan job derives this once so every gate tuple proves the same set. A
/// run that cannot establish its changed paths asks for everything, because
/// the alternative is a narrower suite chosen on no evidence.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AutoplayPlan {
    /// `changed-paths` when a diff was available, `full` otherwise.
    pub policy: String,
    /// Why this policy applied, in terms a reviewer can check.
    pub reason: String,
    /// The scenarios to run, sorted, never empty.
    pub scenarios: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub schema: String,
    pub authority: String,
    pub authority_contract: Option<serde_json::Value>,
    pub tuples: Vec<PlanTuple>,
    pub autoplay: AutoplayPlan,
    pub selection: Option<super::controller::SelectionBinding>,
}

/// Derive the autoplay suite for a set of changed paths.
///
/// `changed` is `None` when the run could not determine its own diff, which
/// includes scheduled and manual runs. That is not treated as "nothing
/// changed"; it is treated as "unknown", and unknown means the full suite.
#[must_use]
pub fn derive_autoplay(changed: Option<&[String]>) -> AutoplayPlan {
    use uqm_rust::automation::suite::{path_is_mapped, required_suite, select_for_changed_paths};

    let Some(changed) = changed else {
        return AutoplayPlan {
            policy: "full".into(),
            reason: "the run did not establish its changed paths".into(),
            scenarios: to_owned(required_suite()),
        };
    };
    if changed.is_empty() {
        return AutoplayPlan {
            policy: "full".into(),
            reason: "the run reported no changed paths".into(),
            scenarios: to_owned(required_suite()),
        };
    }
    let unmapped: Vec<&String> = changed
        .iter()
        .filter(|path| !path_is_mapped(path))
        .collect();
    if let Some(first) = unmapped.first() {
        return AutoplayPlan {
            policy: "full".into(),
            reason: format!(
                "{} of {} changed paths map to no domain, beginning with {first}",
                unmapped.len(),
                changed.len()
            ),
            scenarios: to_owned(required_suite()),
        };
    }
    AutoplayPlan {
        policy: "changed-paths".into(),
        reason: format!("all {} changed paths map to known domains", changed.len()),
        scenarios: to_owned(select_for_changed_paths(changed)),
    }
}

fn to_owned(scenarios: Vec<&'static str>) -> Vec<String> {
    scenarios.into_iter().map(str::to_owned).collect()
}

impl Plan {
    pub fn tuple_names(&self) -> Vec<String> {
        self.tuples
            .iter()
            .map(|tuple| tuple.tuple.clone())
            .collect()
    }
}

pub fn plan(root: &Path) -> Result<Plan, CiError> {
    let source_root = super::controller::runtime_source_root(root)
        .map_err(|error| CiError::new("ci.plan.source", error))?;
    let plan = derive_plan(&source_root)?;
    write_plan(&source_root, &plan)?;
    let text = serde_json::to_string_pretty(&plan)
        .map_err(|error| CiError::new("ci.plan.serialize", error.to_string()))?;
    println!("{text}");
    Ok(plan)
}

pub fn derive_plan(root: &Path) -> Result<Plan, CiError> {
    derive_plan_with(root, &super::controller::WorkflowInputs::from_env())
}

/// Tuple identity only: it needs no workflow event, so it never consults one.
pub fn derive_tuples(root: &Path) -> Result<Vec<String>, CiError> {
    derive_unselected_plan(root).map(|unselected| unselected.plan.tuple_names())
}

struct UnselectedPlan {
    plan: Plan,
    source_root: std::path::PathBuf,
    authority: super::authority::Authority,
    authority_contract: serde_json::Value,
    policy: Vec<u8>,
}

pub fn derive_plan_with(
    root: &Path,
    inputs: &super::controller::WorkflowInputs,
) -> Result<Plan, CiError> {
    let UnselectedPlan {
        mut plan,
        source_root,
        authority,
        authority_contract,
        policy,
    } = derive_unselected_plan(root)?;
    let binding = super::controller::derive_binding(&source_root, &authority, &policy, inputs)
        .map_err(|e| CiError::new("ci.plan.selection", e))?;
    plan.authority_contract = Some(authority_contract);
    plan.autoplay = binding.autoplay.clone();
    plan.selection = Some(binding);
    Ok(plan)
}

fn derive_unselected_plan(root: &Path) -> Result<UnselectedPlan, CiError> {
    let source_root = super::controller::runtime_source_root(root)
        .map_err(|error| CiError::new("ci.plan.source", error))?;
    if !source_root.is_absolute() {
        return Err(CiError::new(
            "ci.plan.source",
            "UQM_CI_SOURCE_ROOT must be absolute",
        ));
    }
    let (authority, authority_contract) = load_authority_contract(&source_root)
        .map_err(|error| CiError::new("ci.plan.authority", error))?;
    derive_supported_tuples(&source_root, &authority)
        .map_err(|error| CiError::new("ci.plan.matrix", error))?;
    let matrix_path = source_root.join(&authority.matrix_file);
    let bytes = super::bounded_io::read_regular_nofollow(
        &matrix_path,
        authority.actions.evidence_snapshot_member_limit_bytes,
    )
    .map_err(|error| CiError::new("ci.plan.matrix", error))?;
    let matrix: Matrix = serde_json::from_slice(&bytes)
        .map_err(|error| CiError::new("ci.plan.matrix", error.to_string()))?;
    let plan = build_plan(&matrix, &authority.runner_mapping, AUTHORITY_RELATIVE)
        .map_err(|error| CiError::new("ci.plan.matrix", error))?;
    let policy = super::bounded_io::read_regular_nofollow(
        &source_root.join(AUTHORITY_RELATIVE),
        super::bounded_io::AUTHORITY_BOOTSTRAP_LIMIT_BYTES,
    )
    .map_err(|e| CiError::new("ci.plan.policy", e))?;
    let source_policy: serde_json::Value = serde_json::from_slice(&policy)
        .map_err(|e| CiError::new("ci.plan.policy", e.to_string()))?;
    if source_policy != authority_contract {
        return Err(CiError::new(
            "ci.plan.policy",
            "source policy differs from admitted controller policy",
        ));
    }
    Ok(UnselectedPlan {
        plan,
        source_root,
        authority,
        authority_contract,
        policy,
    })
}

fn write_plan(root: &Path, plan: &Plan) -> Result<(), CiError> {
    let path = root.join(PLAN_RELATIVE);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            CiError::new(
                "ci.plan.write",
                format!("cannot create {}: {error}", parent.display()),
            )
        })?;
    }
    let mut bytes = serde_json::to_vec_pretty(plan)
        .map_err(|error| CiError::new("ci.plan.serialize", error.to_string()))?;
    bytes.push(b'\n');
    fs::write(&path, bytes).map_err(|error| {
        CiError::new(
            "ci.plan.write",
            format!("cannot write {}: {error}", path.display()),
        )
    })
}

/// Build the plan from authority mappings after checking matrix compatibility.
pub fn build_plan(
    matrix: &Matrix,
    runner_mapping: &[RunnerMapping],
    authority: &str,
) -> Result<Plan, String> {
    let compatibility = matrix.derive_contract_tuples()?;
    let mut authority_tuples: Vec<String> = runner_mapping
        .iter()
        .map(|mapping| mapping.tuple.clone())
        .collect();
    authority_tuples.sort();
    if compatibility != authority_tuples {
        return Err(format!(
            "compatibility matrix tuple set differs from authority: {compatibility:?} vs {authority_tuples:?}"
        ));
    }
    let plan_tuples = runner_mapping
        .iter()
        .map(|mapping| PlanTuple {
            os: mapping.os.clone(),
            architecture: mapping.architecture.clone(),
            tuple: mapping.tuple.clone(),
            runner: mapping.runner.clone(),
            expected_uname: mapping.expected_uname.clone(),
        })
        .collect();
    Ok(Plan {
        schema: PLAN_SCHEMA.to_string(),
        authority: authority.to_string(),
        authority_contract: None,
        tuples: plan_tuples,
        // build_plan derives tuple identity only; derive_plan fills the
        // autoplay suite once it knows the run's changed paths.
        autoplay: derive_autoplay(None),
        selection: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_matrix() -> Matrix {
        serde_json::from_slice(include_bytes!("../../../build/supported-matrix.json")).unwrap()
    }

    fn fixture_runner_mapping() -> Vec<RunnerMapping> {
        let authority: super::super::authority::Authority =
            serde_json::from_slice(include_bytes!("../../../ci/gates.json")).unwrap();
        authority.runner_mapping
    }

    #[test]
    fn derived_plan_preserves_the_exact_authority_json_shape() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap();
        let plan =
            derive_plan_with(&root, &super::super::controller::WorkflowInputs::local()).unwrap();
        let raw: serde_json::Value =
            serde_json::from_slice(include_bytes!("../../../ci/gates.json")).unwrap();
        assert_eq!(plan.authority_contract, Some(raw));
    }
    #[test]
    fn a_full_policy_must_carry_the_whole_suite() {
        // This is the shape the autoplay mutation gate plants: a plan that
        // keeps the reassuring label while the suite behind it shrank.
        let full = derive_autoplay(None);
        let mut trimmed = full.clone();
        trimmed.scenarios.truncate(1);
        assert_eq!(trimmed.policy, "full");
        assert_ne!(
            trimmed.scenarios, full.scenarios,
            "the mutation must actually change the suite, or the gate proves nothing"
        );
    }

    #[test]
    fn unknown_changed_paths_ask_for_everything() {
        // Three ways of not knowing, one answer.
        let full = derive_autoplay(None);
        assert_eq!(full.policy, "full");
        let empty = derive_autoplay(Some(&[]));
        assert_eq!(empty.policy, "full");
        let unmapped = derive_autoplay(Some(&["docs/readme.md".to_string()]));
        assert_eq!(unmapped.policy, "full");
        assert_eq!(full.scenarios, unmapped.scenarios);
        assert!(!full.scenarios.is_empty());
    }

    #[test]
    fn one_unmapped_path_widens_an_otherwise_mapped_change() {
        let plan = derive_autoplay(Some(&[
            "rust/src/battle/x.rs".to_string(),
            "docs/readme.md".to_string(),
        ]));
        assert_eq!(plan.policy, "full");
        assert!(
            plan.reason.contains("docs/readme.md"),
            "the reason must name the path that widened the suite: {}",
            plan.reason
        );
    }

    #[test]
    fn fully_mapped_changes_select_a_subset() {
        let plan = derive_autoplay(Some(&["rust/src/battle/x.rs".to_string()]));
        assert_eq!(plan.policy, "changed-paths");
        assert!(!plan.scenarios.is_empty());
        assert!(plan.scenarios.len() < derive_autoplay(None).scenarios.len());
    }

    #[test]
    fn every_selected_scenario_is_a_real_scenario() {
        let all = derive_autoplay(None).scenarios;
        let subset = derive_autoplay(Some(&["rust/src/battle/x.rs".to_string()])).scenarios;
        for scenario in subset {
            assert!(all.contains(&scenario), "{scenario} is not in the suite");
        }
    }

    #[test]
    fn plan_uses_all_four_authority_tuples() {
        let plan = build_plan(
            &fixture_matrix(),
            &fixture_runner_mapping(),
            "rust/ci/gates.json",
        )
        .unwrap();
        let mut actual: Vec<_> = plan
            .tuples
            .iter()
            .map(|tuple| tuple.tuple.as_str())
            .collect();
        actual.sort();
        assert_eq!(
            actual,
            vec![
                "linux-aarch64",
                "linux-x86_64",
                "macos-aarch64",
                "macos-x86_64"
            ]
        );
        assert_eq!(plan.schema, PLAN_SCHEMA);
    }

    #[test]
    fn plan_rejects_authority_tuple_drift_from_compatibility_matrix() {
        let mut mapping = fixture_runner_mapping();
        mapping[0].tuple = "freebsd-riscv64".into();
        assert!(build_plan(&fixture_matrix(), &mapping, "rust/ci/gates.json").is_err());
    }

    #[test]
    fn plan_rejects_an_invalid_compatibility_matrix() {
        let mut matrix = fixture_matrix();
        matrix.supported[0].architectures.clear();
        assert!(matrix.derive_contract_tuples().is_err());
        assert!(build_plan(&matrix, &fixture_runner_mapping(), "rust/ci/gates.json").is_err());
    }
}

#[cfg(test)]
mod external_controller_regressions {
    use super::*;
    use std::process::Command;

    #[test]
    fn controller_a_plans_candidate_b_and_refuses_missing_bindings_before_candidate_code() {
        const MARKER: &str = "UQM_PLAN_BINDING_FIXTURE";
        if let Ok(controller) = std::env::var(MARKER) {
            let mode = std::env::var("UQM_PLAN_FIXTURE_MODE").unwrap();
            let result = derive_plan(Path::new(&controller));
            if mode == "valid" {
                let plan = result.unwrap();
                let binding = plan.selection.unwrap();
                assert_eq!(
                    binding.source_sha,
                    std::env::var("UQM_CI_EXPECTED_SHA").unwrap()
                );
                assert_ne!(binding.source_sha, binding.controller_sha);
                assert_eq!(plan.autoplay.scenarios.len(), 33);
            } else {
                let error = result.unwrap_err().to_string();
                assert!(
                    error.contains(if mode == "missing-source" {
                        "UQM_CI_SOURCE_ROOT"
                    } else {
                        "UQM_CI_AUTHORITY_PATH"
                    }),
                    "{error}"
                );
            }
            return;
        }
        let temp = tempfile::tempdir().unwrap();
        let policy = include_bytes!("../../../ci/gates.json");
        let mut base: serde_json::Value = serde_json::from_slice(policy).unwrap();
        base["native_acceptance"]["scenario_scripts"]
            .as_array_mut()
            .unwrap()
            .truncate(1);
        let controller = temp.path().join("controller-a");
        let candidate = temp.path().join("candidate-b");
        let commit = |root: &Path, bytes: &[u8]| {
            fs::create_dir_all(root.join("rust/ci")).unwrap();
            fs::create_dir_all(root.join("rust/build")).unwrap();
            fs::write(root.join("rust/ci/gates.json"), bytes).unwrap();
            fs::write(
                root.join("rust/build/supported-matrix.json"),
                include_bytes!("../../../build/supported-matrix.json"),
            )
            .unwrap();
            fs::write(
                root.join("build.rs"),
                b"compile_error!(\"candidate code must not execute during plan\");",
            )
            .unwrap();
            for args in [
                vec!["init", "-q"],
                vec!["config", "user.email", "fixture@example.org"],
                vec!["config", "user.name", "fixture"],
                vec!["add", "."],
                vec!["commit", "-qm", "fixture"],
            ] {
                assert!(Command::new("git")
                    .current_dir(root)
                    .args(args)
                    .status()
                    .unwrap()
                    .success());
            }
            String::from_utf8(
                Command::new("git")
                    .current_dir(root)
                    .args(["rev-parse", "HEAD"])
                    .output()
                    .unwrap()
                    .stdout,
            )
            .unwrap()
            .trim()
            .to_string()
        };
        let a = commit(&controller, &serde_json::to_vec(&base).unwrap());
        let b = commit(&candidate, policy);
        let admitted = temp.path().join("admitted.json");
        super::super::controller::admit_command(
            &controller.join("rust/ci/gates.json"),
            &candidate.join("rust/ci/gates.json"),
            &admitted,
        )
        .unwrap();
        for mode in ["valid", "missing-source", "missing-policy"] {
            let mut child = Command::new(std::env::current_exe().unwrap());
            child.args(["--exact", "ci::plan::external_controller_regressions::controller_a_plans_candidate_b_and_refuses_missing_bindings_before_candidate_code", "--nocapture"])
                .env(MARKER, &controller).env("UQM_PLAN_FIXTURE_MODE", mode)
                .env("UQM_CI_EVENT_NAME", "push").env("UQM_CI_EXPECTED_SHA", &b).env("UQM_CI_CONTROLLER_SHA", &a)
                .env_remove("UQM_CI_SOURCE_ROOT").env_remove("UQM_CI_AUTHORITY_PATH");
            if mode != "missing-source" {
                child.env("UQM_CI_SOURCE_ROOT", &candidate);
            }
            if mode != "missing-policy" {
                child.env("UQM_CI_AUTHORITY_PATH", &admitted);
            }
            let output = child.output().unwrap();
            assert!(
                output.status.success(),
                "{mode}: {} {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
}
