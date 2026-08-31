//! `ci mutations`: the mutation gate.
//!
//! Each mutation injects a deliberate defect or related trust-boundary defect set
//! and proves the responsible gate rejects it. Tool-backed cases use isolated temporary mini fixtures with the
//! registry-free `--offline` mode; contract cases mutate the authority or a
//! fixture in memory. The working tree is never mutated and every case produces a
//! typed machine-readable receipt.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use sha2::{Digest, Sha256};

use super::authority::{self, Gate, MutationTarget, Step};
use super::doctor::ToolExecutableIdentity;
use super::evidence;
use super::exec::{run_captured_with_limits, Captured};
use super::run::{gate_command, subordinate_evidence_environment, RunSession};
use super::CiError;

pub const RECEIPT_SCHEMA: &str = "uqm-s4-mutations-receipt-v3";
const MUTATION_FIXTURE_MEMBER_LIMIT_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MutationCase {
    pub target: String,
    pub contract: String,
    pub defect: String,
    pub baseline_accepted: bool,
    pub rejection_observed: bool,
    pub detail: String,
    pub baseline_executions: Vec<MutationExecution>,
    pub executions: Vec<MutationExecution>,
    pub baseline_files: Vec<MutationFile>,
    pub files: Vec<MutationFile>,
    pub recipe: MutationRecipe,
    pub expected_diagnostic: MutationDiagnostic,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MutationRecipe {
    pub operation: String,
    pub path: String,
    pub baseline_sha256: String,
    pub mutant_sha256: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MutationDiagnostic {
    pub class: String,
    pub path: String,
    pub required_fragments: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MutationExecution {
    pub gate: String,
    pub step: String,
    pub cwd: String,
    pub native_profile: Option<String>,
    pub command: Vec<String>,
    pub executable_identity: Option<ToolExecutableIdentity>,
    pub supervision: MutationSupervision,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
    pub launch_error: Option<String>,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MutationSupervision {
    pub timeout_milliseconds: u128,
    pub termination_grace_milliseconds: u128,
    pub pipe_drain_timeout_milliseconds: u128,
    pub stdout_limit_bytes: usize,
    pub stderr_limit_bytes: usize,
    pub stdout_bytes_seen: u64,
    pub stderr_bytes_seen: u64,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
    pub timed_out: bool,
    pub termination_reason: String,
    pub termination_signal: String,
    pub process_group_cleanup: String,
    pub pipe_cleanup: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MutationFile {
    pub path: String,
    pub byte_length: u64,
    pub sha256: String,
    #[serde(skip)]
    bytes: Vec<u8>,
    #[serde(skip)]
    retained_path: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MutationsReceipt {
    pub schema: String,
    pub source_sha: String,
    pub passed: bool,
    pub first_failed_target: Option<String>,
    pub cases: Vec<MutationCase>,
}

/// Standalone `ci mutations` command.
pub fn run_mutations(root: &Path) -> Result<(), String> {
    let receipt = collect(root)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt).map_err(|error| error.to_string())?
    );
    if receipt.passed {
        Ok(())
    } else {
        Err(format!(
            "mutation gate failed at '{}': {} did not reject its defect",
            receipt.first_failed_target.unwrap_or_default(),
            receipt
                .cases
                .iter()
                .find(|case| !case.rejection_observed)
                .map(|case| case.contract.as_str())
                .unwrap_or("unknown")
        ))
    }
}

/// Mutation gate inside `ci run`.
pub fn mutations_gate(session: &mut RunSession, gate: &Gate) -> Result<(), CiError> {
    let producing_command = gate_command(&gate.id)?;
    let mut receipt = collect(&session.root)?;
    retain_mutation_fixtures(session, &mut receipt, &producing_command)?;
    let relative = format!("{}/mutations-receipt.json", gate.id);
    let mut bytes = serde_json::to_vec_pretty(&receipt)
        .map_err(|error| CiError::new("mutations.evidence", error.to_string()))?;
    bytes.push(b'\n');
    let receipt_path = session.evidence_root.join(&relative);
    let receipt_parent = receipt_path
        .parent()
        .ok_or_else(|| CiError::new("mutations.evidence", receipt_path.display().to_string()))?;
    fs::create_dir_all(receipt_parent).map_err(|error| {
        CiError::new(
            "mutations.evidence",
            format!("cannot create {}: {error}", receipt_parent.display()),
        )
    })?;
    fs::write(&receipt_path, bytes)
        .map_err(|error| CiError::new("mutations.evidence", error.to_string()))?;
    session.entry_from_evidence_path(
        &receipt_path,
        "mutations.receipt",
        "application/json",
        "mutations",
        &producing_command,
    )?;
    if receipt.passed {
        Ok(())
    } else {
        Err(CiError::new(
            "mutations",
            format!(
                "mutation gate failed at '{}'",
                receipt.first_failed_target.unwrap_or_default()
            ),
        ))
    }
}

fn retain_mutation_fixtures(
    session: &mut RunSession,
    receipt: &mut MutationsReceipt,
    producing_command: &[String],
) -> Result<(), CiError> {
    for case in &mut receipt.cases {
        retain_fixture_set(
            session,
            &case.target,
            "baseline",
            &mut case.baseline_files,
            producing_command,
        )?;
        retain_fixture_set(
            session,
            &case.target,
            "mutant",
            &mut case.files,
            producing_command,
        )?;
    }
    Ok(())
}

fn retain_fixture_set(
    session: &mut RunSession,
    target: &str,
    phase: &str,
    files: &mut [MutationFile],
    producing_command: &[String],
) -> Result<(), CiError> {
    for (position, file) in files.iter_mut().enumerate() {
        if !super::evidence::validate_relative_path(&file.retained_path) {
            return Err(CiError::new(
                "mutations.evidence",
                format!("invalid retained fixture path '{}'", file.retained_path),
            ));
        }
        let relative = format!(
            "payloads/mutation.fixture/{target}/{phase}/{position}/{}",
            file.retained_path,
        );
        let destination = session.evidence_root.join(&relative);
        let parent = destination
            .parent()
            .ok_or_else(|| CiError::new("mutations.evidence", destination.display().to_string()))?;
        fs::create_dir_all(parent).map_err(|error| {
            CiError::new(
                "mutations.evidence",
                format!("cannot create {}: {error}", parent.display()),
            )
        })?;
        fs::write(&destination, &file.bytes).map_err(|error| {
            CiError::new(
                "mutations.evidence",
                format!("cannot write {}: {error}", destination.display()),
            )
        })?;
        session.entry_from_evidence_path(
            &destination,
            "mutation.fixture",
            "application/octet-stream",
            "mutations",
            producing_command,
        )?;
        file.path = relative;
    }
    Ok(())
}

/// Collect every mutation case in the order declared by machine authority.
pub fn collect(root: &Path) -> Result<MutationsReceipt, CiError> {
    let authority = authority::load_authority(root)
        .map_err(|error| CiError::new("mutations.authority", error))?;
    authority::validate_authority(&authority)
        .map_err(|error| CiError::new("mutations.authority", error))?;
    let cases = authority
        .mutation_targets
        .iter()
        .map(|target| {
            match MutationTarget::parse(target).ok_or_else(|| {
                CiError::new(
                    "mutations.authority",
                    format!("unsupported mutation target '{target}'"),
                )
            })? {
                MutationTarget::Format => format_mutation(&authority),
                MutationTarget::Check => check_mutation(&authority),
                MutationTarget::Clippy => clippy_mutation(&authority),
                MutationTarget::Test => test_mutation(&authority),
                MutationTarget::Ownership => ownership_mutation(root, &authority),
                MutationTarget::Link => link_mutation(root, &authority),
                MutationTarget::Harness => harness_mutation(root, &authority),
                MutationTarget::Complexity => complexity_mutation(&authority),
                MutationTarget::Security => security_mutation(root, &authority),
                MutationTarget::Coverage => coverage_mutation(root, &authority),
                MutationTarget::Cache => cache_mutation(root, &authority),
                MutationTarget::Workflow => workflow_mutation(root, &authority),
                MutationTarget::Artifact => artifact_mutation(root, &authority),
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    let passed = cases.iter().all(|case| case.rejection_observed);
    let first_failed = cases
        .iter()
        .find(|case| !case.rejection_observed)
        .map(|case| case.target.clone());
    let source_sha = crate::git_text(root, &["rev-parse", "HEAD"], "HEAD")
        .map_err(|error| CiError::new("mutations.source_sha", error))?;
    Ok(MutationsReceipt {
        schema: RECEIPT_SCHEMA.to_string(),
        source_sha,
        passed,
        first_failed_target: first_failed,
        cases,
    })
}

fn fixture_file(temp: &Path, path: &str, content: &str) -> Result<PathBuf, CiError> {
    let file = temp.join(path);
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            CiError::new("mutations.fixture.create", format!("{path}: {error}"))
        })?;
    }
    fs::write(&file, content)
        .map_err(|error| CiError::new("mutations.fixture.write", format!("{path}: {error}")))?;
    Ok(file)
}

fn mutation_file(path: &Path) -> Result<MutationFile, CiError> {
    let retained_path = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            CiError::new(
                "mutations.fixture.path",
                format!("mutation fixture has no UTF-8 basename: {}", path.display()),
            )
        })?;
    mutation_file_at(path, retained_path)
}

fn mutation_file_at(path: &Path, retained_path: &str) -> Result<MutationFile, CiError> {
    let bytes = super::bounded_io::read_regular_nofollow(path, MUTATION_FIXTURE_MEMBER_LIMIT_BYTES)
        .map_err(|error| {
            CiError::new(
                "mutations.fixture.read",
                format!("{}: {error}", path.display()),
            )
        })?;
    Ok(MutationFile {
        path: path.display().to_string(),
        byte_length: bytes.len() as u64,
        sha256: format!("{:x}", Sha256::digest(&bytes)),
        bytes,
        retained_path: retained_path.to_string(),
    })
}
fn mutation_tempdir(context: &str) -> Result<tempfile::TempDir, CiError> {
    let directory =
        tempfile::tempdir().map_err(|error| CiError::new(context, error.to_string()))?;
    super::exec::permit_containment_directory(directory.path())
        .map_err(|detail| CiError::new(context, detail))?;
    Ok(directory)
}

fn utf8_fixture(bytes: &[u8]) -> Result<&str, CiError> {
    std::str::from_utf8(bytes)
        .map_err(|error| CiError::new("mutations.fixture.utf8", error.to_string()))
}

fn write_mutant(path: &Path, bytes: &[u8]) -> Result<(), CiError> {
    fs::write(path, bytes).map_err(|error| {
        CiError::new(
            "mutations.fixture.mutate",
            format!("{}: {error}", path.display()),
        )
    })
}

fn causal_contract(
    path: &str,
    baseline: &[u8],
    mutant: &[u8],
    class: &str,
    required_fragments: &[&str],
) -> (MutationRecipe, MutationDiagnostic) {
    (
        MutationRecipe {
            operation: "replace-exact-file".into(),
            path: path.into(),
            baseline_sha256: format!("{:x}", Sha256::digest(baseline)),
            mutant_sha256: format!("{:x}", Sha256::digest(mutant)),
        },
        MutationDiagnostic {
            class: class.into(),
            path: path.into(),
            required_fragments: required_fragments
                .iter()
                .map(|fragment| (*fragment).to_string())
                .collect(),
        },
    )
}

fn capture_execution(
    gate: &str,
    step: &str,
    cwd: &str,
    native_profile: Option<&str>,
    command: Vec<String>,
    executable_identity: Option<ToolExecutableIdentity>,
    captured: Captured,
) -> MutationExecution {
    let supervision = MutationSupervision {
        timeout_milliseconds: captured.limits.timeout.as_millis(),
        termination_grace_milliseconds: captured.limits.termination_grace.as_millis(),
        pipe_drain_timeout_milliseconds: captured.limits.pipe_drain_timeout.as_millis(),
        stdout_limit_bytes: captured.limits.stdout_bytes,
        stderr_limit_bytes: captured.limits.stderr_bytes,
        stdout_bytes_seen: captured.stdout_bytes_seen,
        stderr_bytes_seen: captured.stderr_bytes_seen,
        stdout_truncated: captured.stdout_truncated,
        stderr_truncated: captured.stderr_truncated,
        timed_out: captured.timed_out,
        termination_reason: captured.termination_reason.to_string(),
        termination_signal: captured.termination_signal.to_string(),
        process_group_cleanup: captured.process_group_cleanup.to_string(),
        pipe_cleanup: captured.pipe_cleanup.to_string(),
        error: captured.supervision_error.clone(),
    };
    let success = captured.succeeded();
    MutationExecution {
        gate: gate.to_string(),
        step: step.to_string(),
        cwd: cwd.to_string(),
        native_profile: native_profile.map(str::to_string),
        command,
        executable_identity,
        supervision,
        stdout: String::from_utf8_lossy(&captured.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&captured.stderr).into_owned(),
        exit_code: captured.exit_code,
        signal: captured.signal,
        launch_error: captured.launch_error,
        success,
    }
}

fn run_bound_captured(
    working_directory: &Path,
    command: &[String],
    environment: &[(String, String)],
    limits: super::exec::Limits,
) -> (Captured, Option<ToolExecutableIdentity>) {
    let captured = run_captured_with_limits(
        working_directory,
        &command[0],
        &command[1..],
        environment,
        limits,
    );
    let identity = captured.executable_identity.clone();
    (captured, identity)
}

fn authoritative_gate<'a>(
    authority: &'a authority::Authority,
    gate: &str,
) -> Result<&'a Gate, CiError> {
    authority
        .gate(gate)
        .filter(|gate| !gate.steps.is_empty())
        .ok_or_else(|| {
            CiError::new(
                "mutations.authority",
                format!("gate '{gate}' has no authoritative process steps"),
            )
        })
}

fn run_authoritative_step(
    root: &Path,
    cargo_home: &Path,
    authority: &authority::Authority,
    gate: &str,
    step: &Step,
) -> MutationExecution {
    let environment = vec![("CARGO_HOME".to_string(), cargo_home.display().to_string())];
    let (captured, executable_identity) = run_bound_captured(
        &root.join(&step.cwd),
        &step.command,
        &environment,
        authority.supervision.limits(step.timeout_seconds),
    );
    capture_execution(
        gate,
        &step.id,
        &step.cwd,
        step.native_profile.as_deref(),
        step.command.clone(),
        executable_identity,
        captured,
    )
}

fn run_mutation_command(
    root: &Path,
    cargo_home: &Path,
    gate: &str,
    step: &str,
    cwd: &str,
    command: &[String],
    authority: &authority::Authority,
) -> MutationExecution {
    let environment = vec![("CARGO_HOME".to_string(), cargo_home.display().to_string())];
    let (captured, executable_identity) = run_bound_captured(
        &root.join(cwd),
        command,
        &environment,
        authority.supervision.builtin_limits(),
    );
    capture_execution(
        gate,
        step,
        cwd,
        None,
        command.to_vec(),
        executable_identity,
        captured,
    )
}

fn run_authoritative_route(
    root: &Path,
    cargo_home: &Path,
    authority: &authority::Authority,
    gate: &Gate,
) -> Vec<MutationExecution> {
    let mut executions = Vec::new();
    for step in &gate.steps {
        let execution = run_authoritative_step(root, cargo_home, authority, &gate.id, step);
        let accepted = execution_accepted(&execution);
        executions.push(execution);
        if !accepted {
            break;
        }
    }
    executions
}

fn route_accepted(executions: &[MutationExecution], gate: &Gate) -> bool {
    executions.len() == gate.steps.len() && executions.iter().all(execution_accepted)
}

fn execution_accepted(execution: &MutationExecution) -> bool {
    execution.executable_identity.is_some()
        && execution.success
        && execution.exit_code == Some(0)
        && valid_supervision(&execution.supervision)
}

fn execution_rejected(execution: &MutationExecution) -> bool {
    execution.executable_identity.is_some()
        && execution.launch_error.is_none()
        && !execution.success
        && execution.exit_code.is_some_and(|code| code != 0)
        && execution.signal.is_none()
        && valid_supervision(&execution.supervision)
}

fn valid_supervision(receipt: &MutationSupervision) -> bool {
    !receipt.timed_out
        && !receipt.stdout_truncated
        && !receipt.stderr_truncated
        && receipt.termination_reason == "none"
        && matches!(
            receipt.process_group_cleanup.as_str(),
            "verified-empty" | "not-supported"
        )
        && receipt.pipe_cleanup == "complete"
        && receipt.error.is_none()
}

fn route_rejected_with_diagnostic(
    executions: &[MutationExecution],
    required_fragments: &[&str],
) -> bool {
    let Some(terminal) = executions.last() else {
        return false;
    };
    if !execution_rejected(terminal) {
        return false;
    }
    let output = format!("{}\n{}", terminal.stdout, terminal.stderr);
    required_fragments
        .iter()
        .all(|fragment| output.contains(fragment))
}

fn coverage_fixture(hit: u32) -> Vec<u8> {
    let mut lcov = String::new();
    for _ in 0..40 {
        lcov.push_str("LF:40\n");
        lcov.push_str(&format!("LH:{hit}\n"));
    }
    lcov.into_bytes()
}

fn cache_fixture(registry_cache_present: bool) -> Option<Vec<u8>> {
    serde_json::to_vec_pretty(&super::cache::InitialStateReceipt {
        schema: super::cache::INITIAL_SCHEMA.to_string(),
        mode: "isolated-empty".into(),
        ambient_cargo_home: "/tmp/cargo-home".into(),
        isolation_cargo_home: "/tmp/repository/rust/target/ci-cargo-home".into(),
        execution_target: "/tmp/repository/rust/target".into(),
        registry_cache_present,
        git_cache_present: false,
        execution_target_absent: true,
        rust_target_present: false,
        sc2_obj_present: false,
        restore_used: false,
        save_used: false,
        first_failed_contract: None,
        passed: true,
    })
    .ok()
}

pub(crate) fn expected_causal_contract(
    target: &str,
    authority: &authority::Authority,
) -> Option<(MutationRecipe, MutationDiagnostic, Vec<u8>, Vec<u8>)> {
    let mut contract_authority = authority.clone();
    contract_authority.mutation_targets.sort();
    let (path, baseline, mutant, class, fragments): (&str, Vec<u8>, Vec<u8>, &str, &[&str]) =
        match target {
            "format" => (
                "src/lib.rs",
                b"pub fn formatted(x: u32) -> u32 {\n    x + 1\n}\n".to_vec(),
                b"pub fn  unformatted( x : u32)->u32{ x+1 }\n".to_vec(),
                "format-diff",
                &["src/lib.rs"],
            ),
            "check" => (
                "src/lib.rs",
                b"pub fn valid() -> u32 { 1 }\n".to_vec(),
                b"pub fn broken() -> u32 {\n    let value: u32 = \"wrong\";\n    value\n}\n"
                    .to_vec(),
                "compiler-type-error",
                &["src/lib.rs", "mismatched types"],
            ),
            "clippy" => (
                "src/lib.rs",
                b"pub fn valid() -> u32 { 1 }\n".to_vec(),
                b"pub fn dead_weight() {\n    let unused = 5;\n}\n".to_vec(),
                "deny-warnings-lint",
                &["src/lib.rs", "unused variable"],
            ),
            "test" => (
                "fixture/src/lib.rs",
                b"#[test]\nfn must_pass() {\n    assert!(true);\n}\n".to_vec(),
                b"#[test]\nfn must_pass() {\n    assert!(false);\n}\n".to_vec(),
                "failing-test",
                &["must_pass", "FAILED"],
            ),
            "ownership" => {
                let baseline = serde_json::to_vec_pretty(&contract_authority).ok()?;
                let mut mutated = contract_authority.clone();
                mutated.gates[0].owner.clear();
                (
                    "authority.json",
                    baseline,
                    serde_json::to_vec_pretty(&mutated).ok()?,
                    "authority-owner",
                    &["authority-owner", "gate owners"],
                )
            }
            "link" => {
                let baseline =
                    include_bytes!("../../../ownership/native-provider-manifest.json").to_vec();
                let mut manifest: uqm_ownership::Manifest =
                    serde_json::from_slice(&baseline).ok()?;
                manifest
                    .symbol_contracts
                    .push(manifest.symbol_contracts.first()?.clone());
                (
                    "provider-manifest.json",
                    baseline,
                    serde_json::to_vec_pretty(&manifest).ok()?,
                    "duplicate-provider",
                    &["duplicate-provider", "duplicate symbol provider"],
                )
            }
            "harness" => {
                let baseline = include_bytes!("../../../harness/run_p00_harness.sh").to_vec();
                let text = std::str::from_utf8(&baseline).ok()?;
                let mutant =
                    text.replacen("for sym in DoInput ", "for sym in MissingInputProvider ", 1);
                (
                    "rust/harness/run_p00_harness.sh",
                    baseline,
                    mutant.into_bytes(),
                    "missing-symbol",
                    &["MissingInputProvider", "not found"],
                )
            }
            "complexity" => {
                let mut mutant = String::from("fn runaway() -> u32 {\n    let mut value = 0;\n");
                for _ in 0..45 {
                    mutant.push_str("    if value < 1000 { value += 1; }\n");
                }
                mutant.push_str("    value\n}\n");
                (
                    "runaway.rs",
                    b"fn bounded() -> u32 { 1 }\n".to_vec(),
                    mutant.into_bytes(),
                    "cyclomatic-complexity",
                    &["runaway"],
                )
            }
            "security" => {
                let baseline = serde_json::to_vec_pretty(&contract_authority).ok()?;
                let mut mutated = contract_authority.clone();
                mutated
                    .gates
                    .iter_mut()
                    .find(|gate| gate.id == "security")?
                    .steps
                    .last_mut()?
                    .command = vec!["cargo".into(), "audit".into()];
                (
                    "authority.json",
                    baseline,
                    serde_json::to_vec_pretty(&mutated).ok()?,
                    "security-command",
                    &["security-command", "--deny warnings"],
                )
            }
            "coverage" => (
                "coverage.lcov",
                coverage_fixture(40),
                coverage_fixture(26),
                "coverage-floor",
                &["coverage-floor", "65.00%"],
            ),
            "cache" => (
                "cache-initial-state.json",
                cache_fixture(false)?,
                cache_fixture(true)?,
                "cache-registry",
                &["cache-registry", "registry cache"],
            ),
            "workflow" => {
                let baseline =
                    include_bytes!("../../../../.github/workflows/rust-quality.yaml").to_vec();
                let mutant = workflow_trust_boundary_mutant(&baseline, authority)?;
                (
                    "rust-quality.yaml",
                    baseline,
                    mutant,
                    "workflow-trust-boundaries",
                    &[
                        "workflow-trust-boundaries",
                        "workflow.actions_full_sha",
                        "workflow.tool_authority",
                        "workflow.uid_containment",
                        "workflow.bootstrap_failure_receipts",
                        "workflow.content_addressed_transport",
                    ],
                )
            }
            _ => return None,
        };
    let (recipe, diagnostic) = causal_contract(path, &baseline, &mutant, class, fragments);
    Some((recipe, diagnostic, baseline, mutant))
}

fn workflow_trust_boundary_mutant(
    baseline: &[u8],
    authority: &authority::Authority,
) -> Option<Vec<u8>> {
    let mut mutant = std::str::from_utf8(baseline).ok()?.to_string();
    for (from, to) in [
        (authority.actions.checkout.as_str(), "actions/checkout@v4"),
        (
            "actionlint_checksums}\" | shasum -a 256 -c -",
            "actionlint_checksums}\" | shasum -a 256",
        ),
        (
            "/usr/sbin/useradd --uid",
            "/usr/sbin/useradd-disabled --uid",
        ),
        (
            "${EVIDENCE_DIR}/xtask-build.result.json",
            "${EVIDENCE_DIR}/xtask-build-unretained.result.json",
        ),
        (
            "uqm-s4-transport-evidence-v1",
            "uqm-s4-transport-evidence-disabled",
        ),
    ] {
        if !mutant.contains(from) {
            return None;
        }
        mutant = mutant.replacen(from, to, 1);
    }
    Some(mutant.into_bytes())
}

pub const INTERNAL_VALIDATOR_COMMAND: &str = "__uqm-ci-mutation-validator";
const INTERNAL_EXECUTABLE: &str = "uqm-xtask-internal";

fn validate_internal_fixture(
    root: &Path,
    fixture: &Path,
    target: &str,
    authority: &authority::Authority,
) -> Result<(), String> {
    match target {
        "ownership" => validate_ownership_mutant(fixture, authority),
        "security" => validate_security_mutant(fixture, authority),
        "link" => validate_link_mutant(fixture, authority),
        "coverage" => validate_coverage_mutant(fixture, authority),
        "cache" => validate_cache_mutant(fixture, authority),
        "workflow" => validate_workflow_mutant(root, fixture, authority),
        "artifact" => validate_artifact_mutant(),
        _ => Err(format!("unsupported internal mutation target '{target}'")),
    }
}

/// Validate the retained ownership mutation fixture.
fn validate_ownership_mutant(
    fixture: &Path,
    authority: &authority::Authority,
) -> Result<(), String> {
    let read = |path: &str| {
        super::bounded_io::read_regular_nofollow(
            &fixture.join(path),
            authority.actions.evidence_snapshot_member_limit_bytes,
        )
        .map_err(|error| format!("cannot read internal mutation fixture {path}: {error}"))
    };
    let value: authority::Authority = serde_json::from_slice(&read("authority.json")?)
        .map_err(|error| format!("invalid authority mutation fixture: {error}"))?;
    match authority::validate_authority(&value) {
        Ok(()) => Ok(()),
        Err(error) if value.gates.iter().any(|gate| gate.owner.is_empty()) => Err(format!(
            "authority-owner: gate owners must be nonempty: {error}"
        )),
        Err(error) => Err(error),
    }
}

/// Validate the retained security mutation fixture.
fn validate_security_mutant(
    fixture: &Path,
    authority: &authority::Authority,
) -> Result<(), String> {
    let read = |path: &str| {
        super::bounded_io::read_regular_nofollow(
            &fixture.join(path),
            authority.actions.evidence_snapshot_member_limit_bytes,
        )
        .map_err(|error| format!("cannot read internal mutation fixture {path}: {error}"))
    };
    let value: authority::Authority = serde_json::from_slice(&read("authority.json")?)
        .map_err(|error| format!("invalid authority mutation fixture: {error}"))?;
    authority::validate_authority(&value)?;
    let expected = authority
        .gates
        .iter()
        .find(|gate| gate.id == "security")
        .ok_or_else(|| "retained authority has no security gate".to_string())?;
    let observed = value
        .gates
        .iter()
        .find(|gate| gate.id == "security")
        .ok_or_else(|| "mutant authority has no security gate".to_string())?;
    let expected = serde_json::to_vec(&expected.steps)
        .map_err(|error| format!("serialize retained security gate: {error}"))?;
    let observed = serde_json::to_vec(&observed.steps)
        .map_err(|error| format!("serialize mutant security gate: {error}"))?;
    if observed != expected {
        return Err(
                "security-command: security gate differs from retained authority and no longer retains --deny warnings"
                    .into(),
            );
    }
    Ok(())
}

/// Validate the retained link mutation fixture.
fn validate_link_mutant(fixture: &Path, authority: &authority::Authority) -> Result<(), String> {
    let read = |path: &str| {
        super::bounded_io::read_regular_nofollow(
            &fixture.join(path),
            authority.actions.evidence_snapshot_member_limit_bytes,
        )
        .map_err(|error| format!("cannot read internal mutation fixture {path}: {error}"))
    };
    let value: uqm_ownership::Manifest =
        serde_json::from_slice(&read("provider-manifest.json")?)
            .map_err(|error| format!("invalid provider mutation fixture: {error}"))?;
    match value.validate_self() {
        Ok(()) => Ok(()),
        Err(error) => {
            let detail = error.to_string();
            if detail.contains("DUPLICATE_PROVIDER")
                && detail.contains("symbol contract must be non-empty and unique")
            {
                Err(format!(
                    "duplicate-provider: duplicate symbol provider: {detail}"
                ))
            } else {
                Err(detail)
            }
        }
    }
}

/// Validate the retained coverage mutation fixture.
fn validate_coverage_mutant(
    fixture: &Path,
    authority: &authority::Authority,
) -> Result<(), String> {
    let read = |path: &str| {
        super::bounded_io::read_regular_nofollow(
            &fixture.join(path),
            authority.actions.evidence_snapshot_member_limit_bytes,
        )
        .map_err(|error| format!("cannot read internal mutation fixture {path}: {error}"))
    };
    let percent = super::run::lcov_line_coverage(&read("coverage.lcov")?)?;
    if percent >= authority.coverage.minimum_line_percent {
        Ok(())
    } else {
        Err(format!(
            "coverage-floor: line coverage is {percent:.2}%, below {:.2}%",
            authority.coverage.minimum_line_percent
        ))
    }
}

/// Validate the retained cache mutation fixture.
fn validate_cache_mutant(fixture: &Path, authority: &authority::Authority) -> Result<(), String> {
    let read = |path: &str| {
        super::bounded_io::read_regular_nofollow(
            &fixture.join(path),
            authority.actions.evidence_snapshot_member_limit_bytes,
        )
        .map_err(|error| format!("cannot read internal mutation fixture {path}: {error}"))
    };
    let receipt = serde_json::from_slice(&read("cache-initial-state.json")?)
        .map_err(|error| format!("invalid cache mutation fixture: {error}"))?;
    let failures = super::cache::validate_receipt(&receipt, &authority.cache)?;
    if failures.is_empty() {
        Ok(())
    } else if receipt.registry_cache_present
        && failures.contains("cache.initial_state.cache_present")
    {
        Err(format!(
            "cache-registry: isolated cache receipt declares a registry cache: {failures:?}"
        ))
    } else {
        Err(format!(
            "cache receipt failed unrelated contracts: {failures:?}"
        ))
    }
}

/// Validate the retained workflow mutation fixture.
fn validate_workflow_mutant(
    root: &Path,
    fixture: &Path,
    authority: &authority::Authority,
) -> Result<(), String> {
    let read = |path: &str| {
        super::bounded_io::read_regular_nofollow(
            &fixture.join(path),
            authority.actions.evidence_snapshot_member_limit_bytes,
        )
        .map_err(|error| format!("cannot read internal mutation fixture {path}: {error}"))
    };
    let yaml = String::from_utf8(read("rust-quality.yaml")?)
        .map_err(|error| format!("workflow mutation fixture is not UTF-8: {error}"))?;
    let document = super::workflow::parse_yaml(&yaml)?;
    let tuples = super::plan::derive_plan(root)?.tuple_names();
    let failed = super::workflow::validate_semantics(&document, &tuples, authority)
        .into_iter()
        .filter(|rule| !rule.passed)
        .map(|rule| rule.rule)
        .collect::<Vec<_>>();
    let required = [
        "workflow.actions_full_sha",
        "workflow.tool_authority",
        "workflow.uid_containment",
        "workflow.bootstrap_failure_receipts",
        "workflow.content_addressed_transport",
    ];
    if required
        .iter()
        .all(|rule| failed.iter().any(|failed| failed == rule))
    {
        Err(format!(
            "workflow-trust-boundaries rejected {}",
            required.join(", ")
        ))
    } else if failed.is_empty() {
        Ok(())
    } else {
        Err(format!(
                "workflow-trust-boundaries mutation did not causally reject every required rule; failed rules: {failed:?}"
            ))
    }
}

/// Validate the retained artifact mutation fixture.
fn validate_artifact_mutant() -> Result<(), String> {
    Err("artifact mutation requires detached top-level replay".to_string())
}

fn validate_artifact_mutation_fixture(fixture: &Path, phase: &str) -> Result<(), String> {
    match (
        phase,
        evidence::validate_evidence_command(fixture, "evidence-index.json"),
    ) {
        ("baseline", Ok(())) => Ok(()),
        ("baseline", Err(error)) => Err(format!(
            "artifact-provenance baseline detached replay failed: {error}"
        )),
        ("mutant", Ok(())) => {
            Err("artifact-provenance mutant survived detached replay".to_string())
        }
        ("mutant", Err(error)) => {
            let detail = error.to_string();
            if !detail.contains("evidence.preflight.tools.rust.result") {
                return Err(format!(
                    "artifact-provenance mutant failed for the wrong contract: {detail}"
                ));
            }
            Err(format!(
                "artifact-provenance detached replay rejected evidence.preflight.tools.rust.result: {detail}"
            ))
        }
        _ => Err("mutation validation phase must be baseline or mutant".to_string()),
    }
}

pub fn run_internal_validator(root: &Path, arguments: &[String]) -> Result<(), String> {
    let [target, phase, fixture] = arguments else {
        return Err("internal mutation validator requires TARGET PHASE FIXTURE".into());
    };
    if !matches!(phase.as_str(), "baseline" | "mutant") {
        return Err(format!("unsupported internal mutation phase '{phase}'"));
    }
    let authority = authority::load_authority(root)?;
    authority::validate_authority(&authority)?;
    if target == "artifact" {
        return validate_artifact_mutation_fixture(Path::new(fixture), phase);
    }
    let (recipe, _diagnostic, baseline, mutant) = expected_causal_contract(target, &authority)
        .ok_or_else(|| format!("target '{target}' has no internal mutation contract"))?;
    let fixture = Path::new(fixture);
    let actual = super::bounded_io::read_regular_nofollow(
        &fixture.join(&recipe.path),
        authority.actions.evidence_snapshot_member_limit_bytes,
    )
    .map_err(|error| {
        format!(
            "cannot read mutation recipe path '{}': {error}",
            recipe.path
        )
    })?;
    let expected = if phase == "baseline" {
        baseline
    } else {
        mutant
    };
    if actual != expected {
        return Err(format!(
            "internal mutation fixture does not match the exact {phase} recipe for {target}"
        ));
    }
    let validation = validate_internal_fixture(root, fixture, target, &authority);
    match (phase.as_str(), validation) {
        ("baseline", Ok(())) => Ok(()),
        ("mutant", Err(error)) => Err(error),
        ("baseline", Err(error)) => Err(format!("baseline validator rejection: {error}")),
        ("mutant", Ok(())) => Err("mutant was accepted by its authoritative validator".into()),
        _ => unreachable!(),
    }
}

fn run_internal_mutation_validator(
    root: &Path,
    fixture: &Path,
    target: &str,
    phase: &str,
    authority: &authority::Authority,
) -> Result<MutationExecution, CiError> {
    let executable = env::current_exe()
        .map_err(|error| CiError::new("mutations.internal_runner", error.to_string()))?;
    let executable = executable
        .to_str()
        .ok_or_else(|| CiError::new("mutations.internal_runner", "xtask path is not UTF-8"))?;
    let arguments = vec![
        INTERNAL_VALIDATOR_COMMAND.to_string(),
        target.to_string(),
        phase.to_string(),
        fixture.display().to_string(),
    ];
    let command: Vec<String> = std::iter::once(executable.to_string())
        .chain(arguments)
        .collect();
    let (captured, executable_identity) =
        run_bound_captured(root, &command, &[], authority.supervision.builtin_limits());
    let mut execution = capture_execution(
        "mutations",
        "internal-validator",
        ".",
        None,
        command,
        executable_identity,
        captured,
    );
    execution.command = vec![
        INTERNAL_EXECUTABLE.into(),
        INTERNAL_VALIDATOR_COMMAND.into(),
        target.into(),
    ];
    Ok(execution)
}

fn internal_mutation_case(
    root: &Path,
    authority: &authority::Authority,
    target: MutationTarget,
    defect: &str,
    companions: &[(&str, &[u8])],
) -> Result<MutationCase, CiError> {
    let target_name = authority
        .mutation_targets
        .iter()
        .find(|name| MutationTarget::parse(name) == Some(target))
        .ok_or_else(|| CiError::new("mutations.authority", "mutation target is absent"))?;
    let (recipe, expected_diagnostic, baseline, mutant) =
        expected_causal_contract(target_name, authority).ok_or_else(|| {
            CiError::new(
                "mutations.contract",
                format!("missing contract for {target_name}"),
            )
        })?;
    let baseline_root = mutation_tempdir("mutations.fixture")?;
    let mutant_root = mutation_tempdir("mutations.fixture")?;
    let write_fixture = |fixture: &Path, bytes: &[u8]| -> Result<Vec<MutationFile>, CiError> {
        let path = fixture.join(&recipe.path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| CiError::new("mutations.fixture", error.to_string()))?;
        }
        fs::write(&path, bytes)
            .map_err(|error| CiError::new("mutations.fixture", error.to_string()))?;
        let mut files = vec![mutation_file_at(&path, &recipe.path)?];
        for (companion_path, companion_bytes) in companions {
            let path = fixture.join(companion_path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|error| CiError::new("mutations.fixture", error.to_string()))?;
            }
            fs::write(&path, companion_bytes)
                .map_err(|error| CiError::new("mutations.fixture", error.to_string()))?;
            files.push(mutation_file_at(&path, companion_path)?);
        }
        Ok(files)
    };
    let baseline_files = write_fixture(baseline_root.path(), &baseline)?;
    let files = write_fixture(mutant_root.path(), &mutant)?;
    let baseline_execution = run_internal_mutation_validator(
        root,
        baseline_root.path(),
        target_name,
        "baseline",
        authority,
    )?;
    let mutant_execution = run_internal_mutation_validator(
        root,
        mutant_root.path(),
        target_name,
        "mutant",
        authority,
    )?;
    let baseline_accepted = execution_accepted(&baseline_execution);
    let fragments = expected_diagnostic
        .required_fragments
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let rejection_observed = baseline_accepted
        && route_rejected_with_diagnostic(std::slice::from_ref(&mutant_execution), &fragments);
    Ok(MutationCase {
        target: target_name.clone(),
        contract: target.contract().into(),
        defect: defect.into(),
        baseline_accepted,
        rejection_observed,
        detail: format!(
            "supervised internal validator rejected {target_name} recipe = {rejection_observed}"
        ),
        baseline_executions: vec![baseline_execution],
        executions: vec![mutant_execution],
        baseline_files,
        files,
        recipe,
        expected_diagnostic,
    })
}

const MINI_MANIFEST: &str = "[package]\nname = \"uqm-ci-mutation-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[features]\naudio_heart = []\ndebug-process = []\nlinked_c_archive = []\n";
const MINI_LOCK: &str = "# This file is automatically @generated by Cargo.\n# It is not intended for manual editing.\nversion = 4\n\n[[package]]\nname = \"uqm-ci-mutation-fixture\"\nversion = \"0.1.0\"\n";

const MINI_XTASK_MANIFEST: &str =
    "[package]\nname = \"uqm-xtask\"\nversion = \"0.1.0\"\nedition = \"2021\"\n";
const MINI_XTASK_MAIN: &str = r#"use std::process::{exit, Command};

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("native-test") => return,
        Some("test") => {}
        _ => exit(2),
    }
    let code = match Command::new("cargo")
        .args([
            "test",
            "--locked",
            "--manifest-path",
            "fixture/Cargo.toml",
        ])
        .status()
    {
        Ok(status) => status.code().unwrap_or(1),
        Err(error) => {
            eprintln!("fixture launch failed: {error}");
            111
        }
    };
    exit(code);
}
"#;
const MINI_XTASK_LOCK: &str = "# This file is automatically @generated by Cargo.\n# It is not intended for manual editing.\nversion = 4\n\n[[package]]\nname = \"uqm-xtask\"\nversion = \"0.1.0\"\n";
fn format_mutation(authority: &authority::Authority) -> Result<MutationCase, CiError> {
    let temp = mutation_tempdir("mutations.fixture.tempdir")?;
    let manifest = fixture_file(temp.path(), "rust/Cargo.toml", MINI_MANIFEST)?;
    let baseline_source = b"pub fn formatted(x: u32) -> u32 {\n    x + 1\n}\n";
    let mutant_source = b"pub fn  unformatted( x : u32)->u32{ x+1 }\n";
    let source = fixture_file(
        temp.path(),
        "rust/src/lib.rs",
        utf8_fixture(baseline_source)?,
    )?;
    let cargo_home = mutation_tempdir("mutations.fixture.cargo_home")?;
    let gate = authoritative_gate(authority, "format")?;
    let baseline_executions =
        run_authoritative_route(temp.path(), cargo_home.path(), authority, gate);
    let baseline_files = vec![
        mutation_file(&manifest)?,
        mutation_file_at(&source, "src/lib.rs")?,
    ];
    write_mutant(&source, mutant_source)?;
    let executions = run_authoritative_route(temp.path(), cargo_home.path(), authority, gate);
    let baseline_accepted = route_accepted(&baseline_executions, gate);
    let rejection_observed =
        baseline_accepted && route_rejected_with_diagnostic(&executions, &["src/lib.rs"]);
    let terminal = executions.last().ok_or_else(|| {
        CiError::new(
            "mutations.format.execution",
            "format mutation route produced no terminal execution",
        )
    })?;
    let detail = if let Some(error) = &terminal.launch_error {
        format!("cannot run cargo fmt for the fixture: {error}")
    } else {
        format!(
            "cargo fmt --all --check on unformatted fixture exited {:?}",
            terminal.exit_code
        )
    };
    let (recipe, expected_diagnostic) = causal_contract(
        "src/lib.rs",
        baseline_source,
        mutant_source,
        "format-diff",
        &["src/lib.rs"],
    );
    Ok(MutationCase {
        target: "format".into(),
        contract: "mutations.format.rejects_unformatted".into(),
        defect: "unformatted first-party Rust source".into(),
        baseline_accepted,
        rejection_observed,
        detail,
        baseline_executions,
        executions,
        baseline_files,
        files: vec![
            mutation_file(&manifest)?,
            mutation_file_at(&source, "src/lib.rs")?,
        ],
        recipe,
        expected_diagnostic,
    })
}

fn check_mutation(authority: &authority::Authority) -> Result<MutationCase, CiError> {
    let temp = mutation_tempdir("mutations.fixture.tempdir")?;
    let manifest = fixture_file(temp.path(), "rust/Cargo.toml", MINI_MANIFEST)?;
    let lock = fixture_file(temp.path(), "rust/Cargo.lock", MINI_LOCK)?;
    let baseline_source = b"pub fn valid() -> u32 { 1 }\n";
    let mutant_source =
        b"pub fn broken() -> u32 {\n    let value: u32 = \"wrong\";\n    value\n}\n";
    let source = fixture_file(
        temp.path(),
        "rust/src/lib.rs",
        utf8_fixture(baseline_source)?,
    )?;
    let binary = fixture_file(temp.path(), "rust/src/bin/uqm.rs", "fn main() {}\n")?;
    let linked_test = fixture_file(
        temp.path(),
        "rust/tests/linked_provider_fixture.rs",
        "#[test]\nfn linked_provider_fixture() {}\n",
    )?;
    let cargo_home = mutation_tempdir("mutations.fixture.cargo_home")?;
    let gate = authoritative_gate(authority, "check")?;
    let baseline_executions =
        run_authoritative_route(temp.path(), cargo_home.path(), authority, gate);
    let baseline_files = vec![
        mutation_file(&manifest)?,
        mutation_file(&lock)?,
        mutation_file_at(&source, "src/lib.rs")?,
        mutation_file_at(&binary, "src/bin/uqm.rs")?,
        mutation_file_at(&linked_test, "tests/linked_provider_fixture.rs")?,
    ];
    write_mutant(&source, mutant_source)?;
    let executions = run_authoritative_route(temp.path(), cargo_home.path(), authority, gate);
    let baseline_accepted = route_accepted(&baseline_executions, gate);
    let rejection_observed = baseline_accepted
        && route_rejected_with_diagnostic(&executions, &["src/lib.rs", "mismatched types"]);
    let (recipe, expected_diagnostic) = causal_contract(
        "src/lib.rs",
        baseline_source,
        mutant_source,
        "compiler-type-error",
        &["src/lib.rs", "mismatched types"],
    );
    Ok(MutationCase {
        target: "check".into(),
        contract: "mutations.check.rejects_compile_error".into(),
        defect: "first-party Rust source with a type error".into(),
        baseline_accepted,
        rejection_observed,
        detail: format!(
            "authoritative cargo check route rejected at step {:?}",
            executions.last().map(|execution| execution.step.as_str())
        ),
        baseline_executions,
        executions,
        baseline_files,
        files: vec![
            mutation_file(&manifest)?,
            mutation_file(&lock)?,
            mutation_file_at(&source, "src/lib.rs")?,
            mutation_file_at(&binary, "src/bin/uqm.rs")?,
            mutation_file_at(&linked_test, "tests/linked_provider_fixture.rs")?,
        ],
        recipe,
        expected_diagnostic,
    })
}

fn clippy_mutation(authority: &authority::Authority) -> Result<MutationCase, CiError> {
    let temp = mutation_tempdir("mutations.fixture.tempdir")?;
    let manifest = fixture_file(temp.path(), "rust/Cargo.toml", MINI_MANIFEST)?;
    let lock = fixture_file(temp.path(), "rust/Cargo.lock", MINI_LOCK)?;
    let baseline_source = b"pub fn valid() -> u32 { 1 }\n";
    let mutant_source = b"pub fn dead_weight() {\n    let unused = 5;\n}\n";
    let source = fixture_file(
        temp.path(),
        "rust/src/lib.rs",
        utf8_fixture(baseline_source)?,
    )?;
    let binary = fixture_file(temp.path(), "rust/src/bin/uqm.rs", "fn main() {}\n")?;
    let linked_test = fixture_file(
        temp.path(),
        "rust/tests/linked_provider_fixture.rs",
        "#[test]\nfn linked_provider_fixture() {}\n",
    )?;
    let cargo_home = mutation_tempdir("mutations.fixture.cargo_home")?;
    let gate = authoritative_gate(authority, "clippy")?;
    let baseline_executions =
        run_authoritative_route(temp.path(), cargo_home.path(), authority, gate);
    let baseline_files = vec![
        mutation_file(&manifest)?,
        mutation_file(&lock)?,
        mutation_file_at(&source, "src/lib.rs")?,
        mutation_file_at(&binary, "src/bin/uqm.rs")?,
        mutation_file_at(&linked_test, "tests/linked_provider_fixture.rs")?,
    ];
    write_mutant(&source, mutant_source)?;
    let executions = run_authoritative_route(temp.path(), cargo_home.path(), authority, gate);
    let baseline_accepted = route_accepted(&baseline_executions, gate);
    let rejection_observed = baseline_accepted
        && route_rejected_with_diagnostic(&executions, &["src/lib.rs", "unused variable"]);
    let (recipe, expected_diagnostic) = causal_contract(
        "src/lib.rs",
        baseline_source,
        mutant_source,
        "deny-warnings-lint",
        &["src/lib.rs", "unused variable"],
    );
    Ok(MutationCase {
        target: "clippy".into(),
        contract: "mutations.clippy.rejects_warning".into(),
        defect: "first-party Rust source with a deny-warnings lint".into(),
        baseline_accepted,
        rejection_observed,
        detail: format!(
            "authoritative cargo clippy route rejected at step {:?}",
            executions.last().map(|execution| execution.step.as_str())
        ),
        baseline_executions,
        executions,
        baseline_files,
        files: vec![
            mutation_file(&manifest)?,
            mutation_file(&lock)?,
            mutation_file_at(&source, "src/lib.rs")?,
            mutation_file_at(&binary, "src/bin/uqm.rs")?,
            mutation_file_at(&linked_test, "tests/linked_provider_fixture.rs")?,
        ],
        recipe,
        expected_diagnostic,
    })
}

fn test_mutation(authority: &authority::Authority) -> Result<MutationCase, CiError> {
    let temp = mutation_tempdir("mutations.fixture.tempdir")?;
    let manifest = fixture_file(temp.path(), "fixture/Cargo.toml", MINI_MANIFEST)?;
    let lock = fixture_file(temp.path(), "fixture/Cargo.lock", MINI_LOCK)?;
    let baseline_source = b"#[test]\nfn must_pass() {\n    assert!(true);\n}\n";
    let mutant_source = b"#[test]\nfn must_pass() {\n    assert!(false);\n}\n";
    let source = fixture_file(
        temp.path(),
        "fixture/src/lib.rs",
        utf8_fixture(baseline_source)?,
    )?;
    let xtask_manifest = fixture_file(temp.path(), "rust/xtask/Cargo.toml", MINI_XTASK_MANIFEST)?;
    let xtask_lock = fixture_file(temp.path(), "rust/xtask/Cargo.lock", MINI_XTASK_LOCK)?;
    let xtask_main = fixture_file(temp.path(), "rust/xtask/src/main.rs", MINI_XTASK_MAIN)?;
    let cargo_home = mutation_tempdir("mutations.fixture.cargo_home")?;
    let gate = authoritative_gate(authority, "tests")?;
    let baseline_executions =
        run_authoritative_route(temp.path(), cargo_home.path(), authority, gate);
    let baseline_files = vec![
        mutation_file_at(&manifest, "fixture/Cargo.toml")?,
        mutation_file_at(&lock, "fixture/Cargo.lock")?,
        mutation_file_at(&source, "fixture/src/lib.rs")?,
        mutation_file_at(&xtask_manifest, "rust/xtask/Cargo.toml")?,
        mutation_file_at(&xtask_lock, "rust/xtask/Cargo.lock")?,
        mutation_file_at(&xtask_main, "rust/xtask/src/main.rs")?,
    ];
    write_mutant(&source, mutant_source)?;
    let executions = run_authoritative_route(temp.path(), cargo_home.path(), authority, gate);
    let baseline_accepted = route_accepted(&baseline_executions, gate);
    let rejection_observed =
        baseline_accepted && route_rejected_with_diagnostic(&executions, &["must_pass", "FAILED"]);
    let (recipe, expected_diagnostic) = causal_contract(
        "fixture/src/lib.rs",
        baseline_source,
        mutant_source,
        "failing-test",
        &["must_pass", "FAILED"],
    );
    Ok(MutationCase {
        target: "test".into(),
        contract: "mutations.test.rejects_failing_test".into(),
        defect: "a first-party test that fails".into(),
        baseline_accepted,
        rejection_observed,
        detail: format!(
            "authoritative xtask test route rejected at step {:?}",
            executions.last().map(|execution| execution.step.as_str())
        ),
        baseline_executions,
        executions,
        baseline_files,
        files: vec![
            mutation_file_at(&manifest, "fixture/Cargo.toml")?,
            mutation_file_at(&lock, "fixture/Cargo.lock")?,
            mutation_file_at(&source, "fixture/src/lib.rs")?,
            mutation_file_at(&xtask_manifest, "rust/xtask/Cargo.toml")?,
            mutation_file_at(&xtask_lock, "rust/xtask/Cargo.lock")?,
            mutation_file_at(&xtask_main, "rust/xtask/src/main.rs")?,
        ],
        recipe,
        expected_diagnostic,
    })
}

fn ownership_mutation(
    root: &Path,
    authority: &authority::Authority,
) -> Result<MutationCase, CiError> {
    internal_mutation_case(
        root,
        authority,
        MutationTarget::Ownership,
        "ownership identity altered in the provider authority",
        &[],
    )
}

fn link_mutation(root: &Path, authority: &authority::Authority) -> Result<MutationCase, CiError> {
    internal_mutation_case(
        root,
        authority,
        MutationTarget::Link,
        "duplicate provider assignment for a required symbol",
        &[],
    )
}

fn write_executable(path: &Path, contents: &str) -> Result<(), CiError> {
    use std::os::unix::fs::PermissionsExt;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| CiError::new("mutations.harness", error.to_string()))?;
    }
    fs::write(path, contents)
        .map_err(|error| CiError::new("mutations.harness", error.to_string()))?;
    let mut permissions = fs::metadata(path)
        .map_err(|error| CiError::new("mutations.harness", error.to_string()))?
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)
        .map_err(|error| CiError::new("mutations.harness", error.to_string()))
}

fn build_harness_fixture(root: &Path, script: &str) -> Result<Vec<PathBuf>, CiError> {
    let script_path = root.join("rust/harness/run_p00_harness.sh");
    write_executable(&script_path, script)?;
    let target = root.join("rust/target/fake");
    let tools = root.join("tools");
    fs::create_dir_all(&target)
        .map_err(|error| CiError::new("mutations.harness", error.to_string()))?;
    let nm = tools.join("nm");
    write_executable(
        &nm,
        "#!/bin/sh\nif [ \"${1:-}\" = -A ]; then\n  for symbol in DoInput AnyButtonPress DoConfirmExit TFB_ProcessEvents TFB_SwapBuffers ProcessInputEvent TFB_FlushGraphicsEx; do echo \"archive(member): 000 T $symbol\"; done\nelse\n  echo '000 T main'\nfi\n",
    )?;
    let pkg_config = tools.join("pkg-config");
    write_executable(&pkg_config, "#!/bin/sh\nexit 0\n")?;
    let cargo = tools.join("cargo");
    write_executable(&cargo, "#!/bin/sh\nexit 0\n")?;
    let cc = tools.join("cc");
    write_executable(
        &cc,
        "#!/bin/sh\nout=\nmap=\nwhile [ $# -gt 0 ]; do\n  case \"$1\" in\n    -o) shift; out=$1 ;;\n    -Wl,-map,*) map=${1#-Wl,-map,} ;;\n    -Wl,-Map,*) map=${1#-Wl,-Map,} ;;\n  esac\n  shift\ndone\n[ -n \"$map\" ] && printf 'fixture map\\n' > \"$map\"\ncat > \"$out\" <<'EOF'\n#!/bin/sh\necho harness_symbol_count=7\necho RESULT=PASS\nEOF\nchmod +x \"$out\"\n",
    )?;
    let support = [
        target.join("libuqm_c.a"),
        target.join("libp00_harness_shim.a"),
        target.join("libuqm_rust.a"),
    ];
    for path in &support {
        fs::write(path, b"fixture archive\n")
            .map_err(|error| CiError::new("mutations.harness", error.to_string()))?;
    }
    let object_manifest = target.join("object-manifest.txt");
    fs::write(&object_manifest, b"a.o\nb.o\n")
        .map_err(|error| CiError::new("mutations.harness", error.to_string()))?;
    let artifact =
        |role: &str, relative: &str, path: &Path| -> Result<serde_json::Value, CiError> {
            let bytes = fs::read(path)
                .map_err(|error| CiError::new("mutations.harness", error.to_string()))?;
            Ok(serde_json::json!({
                "role": role,
                "path": relative,
                "byte_length": bytes.len(),
                "sha256": super::evidence::hex_sha256(&bytes)
            }))
        };
    let tool = |path: &Path| -> Result<serde_json::Value, CiError> {
        let bytes =
            fs::read(path).map_err(|error| CiError::new("mutations.harness", error.to_string()))?;
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| CiError::new("mutations.harness", "fixture tool name is not UTF-8"))?;
        Ok(serde_json::json!({
            "executable": format!("../tools/{name}"),
            "sha256": super::evidence::hex_sha256(&bytes)
        }))
    };
    let manifest_path = root.join("rust/target/production-artifacts.json");
    let manifest = serde_json::json!({
        "artifacts": [
            artifact("c_static_archive", "rust/target/fake/libuqm_c.a", &support[0])?,
            artifact("object_sidecar", "rust/target/fake/object-manifest.txt", &object_manifest)?,
            artifact("rust_static_archive", "rust/target/fake/libuqm_rust.a", &support[2])?
        ],
        "native_build": {"toolchain": {
            "cc": tool(&cc)?,
            "nm": tool(&nm)?,
            "pkg_config": tool(&pkg_config)?
        }}
    });
    let manifest_bytes = serde_json::to_vec_pretty(&manifest)
        .map_err(|error| CiError::new("mutations.harness.serialize", error.to_string()))?;
    write_mutant(&manifest_path, &manifest_bytes)?;
    Ok(vec![
        script_path,
        manifest_path,
        support[0].clone(),
        support[1].clone(),
        support[2].clone(),
        object_manifest,
        cargo,
        cc,
        nm,
        pkg_config,
    ])
}

fn isolated_harness_change(baseline: &[MutationFile], mutant: &[MutationFile]) -> bool {
    if baseline.len() != mutant.len() || baseline.len() < 2 {
        return false;
    }
    let changed: Vec<_> = baseline
        .iter()
        .zip(mutant)
        .filter_map(|(baseline, mutant)| {
            (baseline.retained_path == mutant.retained_path && baseline.sha256 != mutant.sha256)
                .then_some(baseline.retained_path.as_str())
        })
        .collect();
    baseline
        .iter()
        .zip(mutant)
        .all(|(baseline, mutant)| baseline.retained_path == mutant.retained_path)
        && changed == ["rust/harness/run_p00_harness.sh"]
}

fn harness_mutation(
    root: &Path,
    authority: &authority::Authority,
) -> Result<MutationCase, CiError> {
    let gate = authoritative_gate(authority, "probes-harnesses")?;
    let step = gate
        .steps
        .iter()
        .find(|step| step.id == "p00-harness")
        .ok_or_else(|| CiError::new("mutations.authority", "p00-harness step is absent"))?;
    let harness_path = root.join("rust/harness/run_p00_harness.sh");
    let content = String::from_utf8(
        super::bounded_io::read_regular_nofollow(
            &harness_path,
            MUTATION_FIXTURE_MEMBER_LIMIT_BYTES,
        )
        .map_err(|error| CiError::new("mutations.harness", error))?,
    )
    .map_err(|error| CiError::new("mutations.harness", error.to_string()))?;
    let mutant = content.replacen("for sym in DoInput ", "for sym in MissingInputProvider ", 1);
    if mutant == content {
        return Err(CiError::new(
            "mutations.harness",
            "the authoritative harness symbol contract was not found",
        ));
    }
    let baseline = mutation_tempdir("mutations.harness")?;
    let defective = mutation_tempdir("mutations.harness")?;
    let baseline_paths = build_harness_fixture(baseline.path(), &content)?;
    let mutant_paths = build_harness_fixture(defective.path(), &mutant)?;
    let execute = |fixture: &Path, phase: &str| {
        let subordinate_root = fixture.join(format!("{phase}-subordinate"));
        let mut environment = vec![(
            "CARGO_HOME".to_string(),
            fixture.join("cargo-home").display().to_string(),
        )];
        environment.extend(subordinate_evidence_environment(
            authority,
            &subordinate_root,
        ));
        let tools = fixture.join("tools");
        environment.extend([
            ("CARGO".to_string(), "../tools/cargo".to_string()),
            ("CC".to_string(), "../tools/cc".to_string()),
            ("NM".to_string(), "../tools/nm".to_string()),
            ("PKG_CONFIG".to_string(), "../tools/pkg-config".to_string()),
            (
                "UQM_CI_CONTROLLER_EXECUTABLE".to_string(),
                tools.join("cargo").display().to_string(),
            ),
            (
                "UQM_CI_SOURCE_ROOT".to_string(),
                fixture.display().to_string(),
            ),
        ]);
        environment.push((
            "PATH".to_string(),
            format!(
                "{}:{}",
                tools.display(),
                std::env::var("PATH").unwrap_or_default()
            ),
        ));
        let (captured, executable_identity) = run_bound_captured(
            &fixture.join(&step.cwd),
            &step.command,
            &environment,
            authority.supervision.limits(step.timeout_seconds),
        );
        capture_execution(
            &gate.id,
            &step.id,
            &step.cwd,
            step.native_profile.as_deref(),
            step.command.clone(),
            executable_identity,
            captured,
        )
    };
    let baseline_execution = execute(baseline.path(), "baseline");
    let mutant_execution = execute(defective.path(), "mutant");
    let baseline_accepted = execution_accepted(&baseline_execution);
    let diagnostic_fragments = ["MissingInputProvider", "not found"];
    let diagnostic_rejection = route_rejected_with_diagnostic(
        std::slice::from_ref(&mutant_execution),
        &diagnostic_fragments,
    );
    let retain = |fixture: &Path, paths: Vec<PathBuf>| -> Result<Vec<MutationFile>, CiError> {
        paths
            .into_iter()
            .map(|path| {
                let relative = path
                    .strip_prefix(fixture)
                    .map_err(|error| CiError::new("mutations.harness.path", error.to_string()))?;
                let relative = relative.to_str().ok_or_else(|| {
                    CiError::new(
                        "mutations.harness.path",
                        format!("fixture path is not UTF-8: {}", relative.display()),
                    )
                })?;
                mutation_file_at(&path, relative)
            })
            .collect()
    };
    let baseline_files = retain(baseline.path(), baseline_paths)?;
    let files = retain(defective.path(), mutant_paths)?;
    let rejection_observed = baseline_accepted
        && diagnostic_rejection
        && isolated_harness_change(&baseline_files, &files);
    let (recipe, expected_diagnostic) = causal_contract(
        "rust/harness/run_p00_harness.sh",
        content.as_bytes(),
        mutant.as_bytes(),
        "missing-symbol",
        &diagnostic_fragments,
    );
    Ok(MutationCase {
        target: "harness".into(),
        contract: "mutations.harness.rejects_missing_marker".into(),
        defect: "required native-provider symbol changed in the authoritative harness script"
            .into(),
        baseline_accepted,
        rejection_observed,
        detail: if rejection_observed {
            "the exact p00-harness command accepted the isolated baseline and rejected the missing provider".into()
        } else {
            "the exact p00-harness route did not causally reject the missing provider".into()
        },
        baseline_executions: vec![baseline_execution],
        executions: vec![mutant_execution],
        baseline_files,
        files,
        recipe,
        expected_diagnostic,
    })
}

fn complexity_mutation(authority: &authority::Authority) -> Result<MutationCase, CiError> {
    let temp = mutation_tempdir("mutations.fixture.tempdir")?;
    let baseline_source = b"fn bounded() -> u32 { 1 }\n";
    let file = fixture_file(temp.path(), "runaway.rs", utf8_fixture(baseline_source)?)?;
    let mut command = vec!["lizard".to_string()];
    command.extend(authority.complexity.lizard_arguments.clone());
    command.push("runaway.rs".to_string());
    let tool_home = mutation_tempdir("mutations.fixture.tool_home")?;
    let baseline_executions = vec![run_mutation_command(
        temp.path(),
        tool_home.path(),
        "complexity",
        "lizard",
        ".",
        &command,
        authority,
    )];
    let baseline_files = vec![mutation_file(&file)?];
    let mut source = String::from("fn runaway() -> u32 {\n    let mut value = 0;\n");
    for _ in 0..45 {
        source.push_str("    if value < 1000 { value += 1; }\n");
    }
    source.push_str("    value\n}\n");
    write_mutant(&file, source.as_bytes())?;
    let executions = vec![run_mutation_command(
        temp.path(),
        tool_home.path(),
        "complexity",
        "lizard",
        ".",
        &command,
        authority,
    )];
    let baseline_accepted = baseline_executions.iter().all(execution_accepted);
    let rejection_observed =
        baseline_accepted && route_rejected_with_diagnostic(&executions, &["runaway"]);
    let (recipe, expected_diagnostic) = causal_contract(
        "runaway.rs",
        baseline_source,
        source.as_bytes(),
        "cyclomatic-complexity",
        &["runaway"],
    );
    Ok(MutationCase {
        target: "complexity".into(),
        contract: "mutations.complexity.rejects_over_limit".into(),
        defect: "a function with cyclomatic complexity above the 40 maximum".into(),
        baseline_accepted,
        rejection_observed,
        detail: format!(
            "authoritative lizard route on a 45-branch function exited {:?}",
            executions.last().and_then(|execution| execution.exit_code)
        ),
        baseline_executions,
        executions,
        baseline_files,
        files: vec![mutation_file(&file)?],
        recipe,
        expected_diagnostic,
    })
}

fn security_mutation(
    root: &Path,
    authority: &authority::Authority,
) -> Result<MutationCase, CiError> {
    internal_mutation_case(
        root,
        authority,
        MutationTarget::Security,
        "security gate without --deny warnings",
        &[],
    )
}

fn coverage_mutation(
    root: &Path,
    authority: &authority::Authority,
) -> Result<MutationCase, CiError> {
    internal_mutation_case(
        root,
        authority,
        MutationTarget::Coverage,
        "75% line coverage computed from a synthetic lcov report",
        &[],
    )
}

fn cache_mutation(root: &Path, authority: &authority::Authority) -> Result<MutationCase, CiError> {
    internal_mutation_case(
        root,
        authority,
        MutationTarget::Cache,
        "isolated cache receipt with a pre-populated registry",
        &[],
    )
}

fn workflow_mutation(
    root: &Path,
    authority: &authority::Authority,
) -> Result<MutationCase, CiError> {
    internal_mutation_case(
        root,
        authority,
        MutationTarget::Workflow,
        "workflow trust boundaries weakened across action pinning, downloaded tools, UID containment, bootstrap receipts, and source revalidation",
        &[],
    )
}

fn artifact_mutation(
    root: &Path,
    authority: &authority::Authority,
) -> Result<MutationCase, CiError> {
    let authority_bytes = super::bounded_io::read_regular_nofollow(
        &root.join(authority::AUTHORITY_RELATIVE),
        super::bounded_io::AUTHORITY_BOOTSTRAP_LIMIT_BYTES,
    )
    .map_err(|error| CiError::new("mutations.artifact.authority", error))?;
    let baseline_root = mutation_tempdir("mutations.artifact.baseline")?;
    let mutant_root = mutation_tempdir("mutations.artifact.mutant")?;
    evidence::build_artifact_mutation_fixture(
        baseline_root.path(),
        authority,
        &authority_bytes,
        false,
    )
    .map_err(|error| CiError::new("mutations.artifact.fixture", error))?;
    evidence::build_artifact_mutation_fixture(
        mutant_root.path(),
        authority,
        &authority_bytes,
        true,
    )
    .map_err(|error| CiError::new("mutations.artifact.fixture", error))?;

    let baseline_files = artifact_mutation_files(baseline_root.path(), authority)?;
    let files = artifact_mutation_files(mutant_root.path(), authority)?;
    let baseline_execution = run_internal_mutation_validator(
        root,
        baseline_root.path(),
        "artifact",
        "baseline",
        authority,
    )?;
    let mutant_execution =
        run_internal_mutation_validator(root, mutant_root.path(), "artifact", "mutant", authority)?;
    let baseline_accepted = execution_accepted(&baseline_execution);
    let expected_diagnostic = MutationDiagnostic {
        class: "artifact-provenance".to_string(),
        path: "tool-preflight.json".to_string(),
        required_fragments: vec![
            "artifact-provenance".to_string(),
            "evidence.preflight.tools.rust.result".to_string(),
        ],
    };
    let fragments = expected_diagnostic
        .required_fragments
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let rejection_observed = baseline_accepted
        && route_rejected_with_diagnostic(std::slice::from_ref(&mutant_execution), &fragments);
    let baseline_tool = baseline_files
        .iter()
        .find(|file| file.retained_path == "tool-preflight.json")
        .ok_or_else(|| {
            CiError::new(
                "mutations.artifact.fixture",
                "baseline tool evidence is missing",
            )
        })?;
    let mutant_tool = files
        .iter()
        .find(|file| file.retained_path == "tool-preflight.json")
        .ok_or_else(|| {
            CiError::new(
                "mutations.artifact.fixture",
                "mutant tool evidence is missing",
            )
        })?;
    let recipe = MutationRecipe {
        operation: "replace-and-rehash-enclosing-index".to_string(),
        path: "tool-preflight.json".to_string(),
        baseline_sha256: baseline_tool.sha256.clone(),
        mutant_sha256: mutant_tool.sha256.clone(),
    };
    Ok(MutationCase {
        target: "artifact".to_string(),
        contract: MutationTarget::Artifact.contract().to_string(),
        defect: "coherently rehashed tool observation contradicts authority provenance".to_string(),
        baseline_accepted,
        rejection_observed,
        detail: format!(
            "detached replay accepted baseline and rejected provenance forgery = {rejection_observed}"
        ),
        baseline_executions: vec![baseline_execution],
        executions: vec![mutant_execution],
        baseline_files,
        files,
        recipe,
        expected_diagnostic,
    })
}

fn artifact_mutation_files(
    fixture: &Path,
    authority: &authority::Authority,
) -> Result<Vec<MutationFile>, CiError> {
    [
        "evidence-index.json",
        "payloads/authority.snapshot/gates.json",
        "source-preflight.json",
        "tool-preflight.json",
        "cache-initial-state.json",
    ]
    .into_iter()
    .map(|relative| {
        let path = fixture.join(relative);
        let bytes = super::bounded_io::read_regular_nofollow(
            &path,
            authority.actions.evidence_snapshot_member_limit_bytes,
        )
        .map_err(|error| CiError::new("mutations.artifact.fixture", error))?;
        Ok(MutationFile {
            path: path.display().to_string(),
            byte_length: bytes.len() as u64,
            sha256: format!("{:x}", Sha256::digest(&bytes)),
            bytes,
            retained_path: relative.to_string(),
        })
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repository_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("xtask belongs to the repository")
            .to_path_buf()
    }

    fn authority() -> authority::Authority {
        let root = repository_root();
        let authority = authority::load_authority(&root).expect("load checked-in authority");
        authority::validate_authority(&authority).expect("validate checked-in authority");
        authority
    }

    fn assert_causal_rejection(case: MutationCase) {
        assert!(
            case.baseline_accepted,
            "baseline failed: {}; executions: {:#?}",
            case.detail, case.baseline_executions
        );
        assert!(
            case.rejection_observed,
            "mutant survived: {}; executions: {:#?}",
            case.detail, case.executions
        );
        assert!(case.baseline_executions.iter().all(execution_accepted));
        assert!(case
            .executions
            .iter()
            .any(|execution| !execution_accepted(execution)));
        assert!(isolated_harness_change(&case.baseline_files, &case.files));
    }

    fn assert_internal_causal_rejection(target: &str) {
        let root = repository_root();
        let authority = authority();
        let (recipe, diagnostic, baseline, mutant) =
            expected_causal_contract(target, &authority).unwrap();
        for (phase, bytes, accepted) in [("baseline", baseline, true), ("mutant", mutant, false)] {
            let fixture = tempfile::tempdir().unwrap();
            fixture_file(
                fixture.path(),
                &recipe.path,
                std::str::from_utf8(&bytes).unwrap(),
            )
            .unwrap();
            let result = run_internal_validator(
                &root,
                &[
                    target.into(),
                    phase.into(),
                    fixture.path().display().to_string(),
                ],
            );
            assert_eq!(result.is_ok(), accepted, "{target} {phase}: {result:?}");
            if !accepted {
                let detail = result.unwrap_err();
                assert!(
                    diagnostic
                        .required_fragments
                        .iter()
                        .all(|fragment| detail.contains(fragment)),
                    "{target} emitted the wrong diagnostic: {detail}"
                );
            }
        }
    }

    fn rejected_execution(stderr: String) -> MutationExecution {
        MutationExecution {
            gate: "mutations".into(),
            step: "internal-validator".into(),
            cwd: ".".into(),
            native_profile: None,
            command: vec![
                INTERNAL_EXECUTABLE.into(),
                INTERNAL_VALIDATOR_COMMAND.into(),
            ],
            executable_identity: Some(ToolExecutableIdentity {
                path: "/fixture/uqm-xtask".into(),
                byte_length: 1,
                sha256: "a".repeat(64),
                mode: 0o755,
            }),
            supervision: MutationSupervision {
                timeout_milliseconds: 1,
                termination_grace_milliseconds: 1,
                pipe_drain_timeout_milliseconds: 1,
                stdout_limit_bytes: 1024,
                stderr_limit_bytes: 1024,
                stdout_bytes_seen: 0,
                stderr_bytes_seen: stderr.len() as u64,
                stdout_truncated: false,
                stderr_truncated: false,
                timed_out: false,
                termination_reason: "none".into(),
                termination_signal: "none".into(),
                process_group_cleanup: "verified-empty".into(),
                pipe_cleanup: "complete".into(),
                error: None,
            },
            stdout: String::new(),
            stderr,
            exit_code: Some(1),
            signal: None,
            launch_error: None,
            success: false,
        }
    }

    #[test]
    fn internal_validators_causally_reject_their_mutants() {
        for target in [
            "ownership",
            "security",
            "link",
            "coverage",
            "cache",
            "workflow",
        ] {
            assert_internal_causal_rejection(target);
        }
    }

    #[test]
    fn wrong_cause_and_partial_workflow_rejections_do_not_match_diagnostics() {
        let root = repository_root();
        let authority = authority();

        let (_, ownership_diagnostic, _, _) =
            expected_causal_contract("ownership", &authority).unwrap();
        let wrong_cause = rejected_execution(
            "invalid authority mutation fixture: expected value at line 1 column 1".into(),
        );
        let ownership_fragments = ownership_diagnostic
            .required_fragments
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        assert!(!route_rejected_with_diagnostic(
            &[wrong_cause],
            &ownership_fragments
        ));

        let fixture = tempfile::tempdir().unwrap();
        let baseline = include_str!("../../../../.github/workflows/rust-quality.yaml");
        let partial = baseline.replacen(
            authority.actions.checkout.as_str(),
            "actions/checkout@v4",
            1,
        );
        fixture_file(fixture.path(), "rust-quality.yaml", &partial).unwrap();
        let partial_error =
            validate_internal_fixture(&root, fixture.path(), "workflow", &authority).unwrap_err();
        let (_, workflow_diagnostic, _, _) =
            expected_causal_contract("workflow", &authority).unwrap();
        let partial_execution = rejected_execution(partial_error);
        let workflow_fragments = workflow_diagnostic
            .required_fragments
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        assert!(!route_rejected_with_diagnostic(
            &[partial_execution],
            &workflow_fragments
        ));
    }

    #[test]
    fn harness_route_accepts_baseline_and_causally_rejects_mutant() {
        let root = repository_root();
        let record = harness_mutation(&root, &authority()).unwrap();
        assert!(record.baseline_executions.iter().any(|execution| {
            execution
                .stdout
                .contains("=== P00 Harness Probe: ALL CHECKS PASSED ===")
        }));
        assert!(record
            .baseline_executions
            .iter()
            .all(|execution| execution.stderr.is_empty()));
        assert_causal_rejection(record);
    }

    #[test]
    fn artifact_route_replays_complete_coherently_rehashed_bundles() {
        let authority = authority();
        let authority_bytes = serde_json::to_vec_pretty(&authority).unwrap();
        let baseline = tempfile::tempdir().unwrap();
        let mutant = tempfile::tempdir().unwrap();
        evidence::build_artifact_mutation_fixture(
            baseline.path(),
            &authority,
            &authority_bytes,
            false,
        )
        .unwrap();
        evidence::build_artifact_mutation_fixture(
            mutant.path(),
            &authority,
            &authority_bytes,
            true,
        )
        .unwrap();
        assert!(validate_artifact_mutation_fixture(baseline.path(), "baseline").is_ok());
        let rejection = validate_artifact_mutation_fixture(mutant.path(), "mutant").unwrap_err();
        assert!(rejection.contains("evidence.preflight.tools.rust.result"));

        let baseline_files = artifact_mutation_files(baseline.path(), &authority).unwrap();
        let mutant_files = artifact_mutation_files(mutant.path(), &authority).unwrap();
        let changed = baseline_files
            .iter()
            .zip(&mutant_files)
            .filter_map(|(baseline, mutant)| {
                (baseline.sha256 != mutant.sha256).then_some(baseline.retained_path.as_str())
            })
            .collect::<Vec<_>>();
        assert_eq!(changed, ["evidence-index.json", "tool-preflight.json"]);

        let index_file = mutant_files
            .iter()
            .find(|file| file.retained_path == "evidence-index.json")
            .unwrap();
        let tool_file = mutant_files
            .iter()
            .find(|file| file.retained_path == "tool-preflight.json")
            .unwrap();
        let index: evidence::EvidenceIndex = serde_json::from_slice(&index_file.bytes).unwrap();
        let entry = index
            .entries
            .iter()
            .find(|entry| entry.path == "tool-preflight.json")
            .unwrap();
        assert_eq!(entry.byte_length, tool_file.byte_length);
        assert_eq!(entry.sha256, tool_file.sha256);
    }
}
