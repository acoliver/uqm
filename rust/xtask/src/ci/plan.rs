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
    let plan = derive_plan(root)?;
    write_plan(root, &plan)?;
    let text = serde_json::to_string_pretty(&plan)
        .map_err(|error| CiError::new("ci.plan.serialize", error.to_string()))?;
    println!("{text}");
    Ok(plan)
}

pub fn derive_plan(root: &Path) -> Result<Plan, CiError> {
    let (authority, authority_contract) =
        load_authority_contract(root).map_err(|error| CiError::new("ci.plan.authority", error))?;
    derive_supported_tuples(root, &authority)
        .map_err(|error| CiError::new("ci.plan.matrix", error))?;
    let matrix_path = root.join(&authority.matrix_file);
    let bytes = super::bounded_io::read_regular_nofollow(
        &matrix_path,
        authority.actions.evidence_snapshot_member_limit_bytes,
    )
    .map_err(|error| CiError::new("ci.plan.matrix", error))?;
    let matrix: Matrix = serde_json::from_slice(&bytes)
        .map_err(|error| CiError::new("ci.plan.matrix", error.to_string()))?;
    let mut plan = build_plan(&matrix, &authority.runner_mapping, AUTHORITY_RELATIVE)
        .map_err(|error| CiError::new("ci.plan.matrix", error))?;
    let policy = super::bounded_io::read_regular_nofollow(
        &root.join(AUTHORITY_RELATIVE),
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
    let binding = super::controller::derive_binding(root, &authority, &policy)
        .map_err(|e| CiError::new("ci.plan.selection", e))?;
    plan.authority_contract = Some(authority_contract);
    plan.autoplay = binding.autoplay.clone();
    plan.selection = Some(binding);
    Ok(plan)
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
        let plan = derive_plan(&root).unwrap();
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
