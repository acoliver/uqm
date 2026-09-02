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

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub schema: String,
    pub authority: String,
    pub authority_contract: Option<serde_json::Value>,
    pub tuples: Vec<PlanTuple>,
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
    plan.authority_contract = Some(authority_contract);
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
