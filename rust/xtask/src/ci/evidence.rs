//! Generic content-addressed evidence index and offline validation.
//!
//! `ci run` emits an index whose entries carry byte counts and SHA-256 digests.
//! `ci validate-evidence <path>` (and `ci run` itself) revalidates the index
//! offline by recomputing sizes and hashes and checking every required field, without
//! executing any producing command. Tuples are validated against the set derived from
//! `rust/build/supported-matrix.json`, never a local list.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::authority::{
    validate_authority, Authority, GateKind, MutationTarget, AUTHORITY_RELATIVE, CACHE_MODES,
};
use super::CiError;

pub const EVIDENCE_SCHEMA: &str = "uqm-s4-evidence-index-v1";
pub const ENTRY_SCHEMA: &str = "uqm-s4-evidence-entry-v1";
pub const INDEX_FILENAME: &str = "evidence-index.json";
pub const PROFILE: &str = "ci";

pub const PRE_SESSION_SCHEMA: &str = "uqm-s4-pre-session-failure-v1";
pub const PRE_SESSION_FILENAME: &str = "pre-session-failure.json";
pub const TRANSPORT_SCHEMA: &str = "uqm-s4-transport-evidence-v1";
const WORKFLOW_SETUP_SCHEMA: &str = "uqm-s4-workflow-setup-results-v1";
const UPLOAD_RECEIPT_SCHEMA: &str = "uqm-s4-upload-receipt-v1";
const UPLOAD_AUTHORITY_UNAVAILABLE_SCHEMA: &str = "uqm-s4-upload-authority-unavailable-v1";
const REQUIRED_RESULT_SCHEMA: &str = "uqm-s4-required-result-v1";

const PACKAGE_PROOF_COMMAND: &str =
    "cargo run --locked --manifest-path rust/xtask/Cargo.toml -- prove";
const PACKAGE_PROOF_COMPARISON: &str = "byte_length_and_sha256_identical";
const PACKAGE_PROOF_CLEAN_BUILDS: u64 = 2;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct TransportIndex {
    schema: String,
    job: String,
    source_sha: String,
    #[serde(default)]
    tuple: Option<String>,
    #[serde(default)]
    exit_code: Option<i32>,
    #[serde(default)]
    job_status: Option<String>,
    files: Vec<TransportFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct TransportFile {
    path: String,
    byte_length: u64,
    sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkflowSetupResults {
    schema: String,
    job: String,
    source_sha: String,
    tuple: Option<String>,
    steps: Vec<WorkflowSetupStep>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkflowSetupStep {
    step: String,
    outcome: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RequiredResult {
    schema: String,
    source_sha: String,
    plan: String,
    gates: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct UploadReceipt {
    schema: String,
    job: String,
    #[serde(default)]
    tuple: Option<String>,
    source_sha: String,
    artifact_name: String,
    artifact_id: Option<u64>,
    artifact_url: Option<String>,
    artifact_digest: Option<String>,
    retention_days: u32,
    size_in_bytes: Option<u64>,
    upload_outcome: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct UploadAuthorityUnavailableReceipt {
    schema: String,
    job: String,
    source_sha: String,
    artifact_name: String,
    artifact_id: Option<u64>,
    artifact_url: Option<String>,
    artifact_digest: Option<String>,
    retention_days: Option<u32>,
    size_in_bytes: Option<u64>,
    upload_outcome: String,
    failure: String,
}

const TRANSPORT_FALLBACK_SCHEMA: &str = "uqm-s4-transport-finalizer-fallback-v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct TransportFinalizerFallback {
    schema: String,
    job: String,
    source_sha: String,
    tuple: Option<String>,
    first_failed_contract: String,
    detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PreSessionFailureEnvelope {
    pub schema: String,
    pub passed: bool,
    pub first_failed_contract: String,
    pub detail: String,
    pub requested_gate: String,
    pub tuple: String,
    pub cache_mode: String,
    pub configured_evidence_root: Option<String>,
    pub authority_snapshot: Option<String>,
    pub controller_command: Vec<String>,
    pub offline_validation: OfflineValidation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OfflineValidation {
    pub passed: bool,
    pub contracts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceEntry {
    pub schema: String,
    pub role: String,
    pub path: String,
    pub mime: String,
    pub byte_length: u64,
    pub sha256: String,
    pub producing_gate: String,
    pub producing_command: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceIndex {
    pub schema: String,
    pub source_sha: String,
    pub clean: bool,
    pub tuple: String,
    pub supported_tuples: Vec<String>,
    pub profile: String,
    pub features: Vec<String>,
    pub cache_mode: String,
    pub first_failed_contract: Option<String>,
    pub offline_validation: OfflineValidation,
    pub entries: Vec<EvidenceEntry>,
}

pub struct EvidenceContext {
    pub source_sha: String,
    pub clean: bool,
    pub tuple: String,
    pub features: Vec<String>,
    pub cache_mode: String,
    pub first_failed_contract: Option<String>,
}

impl PreSessionFailureEnvelope {
    pub fn build(root: &Path, requested_gate: &str, contract: &str, detail: &str) -> Self {
        let authority_snapshot = (contract != "authority.load")
            .then(|| {
                read_regular_relative(root, AUTHORITY_RELATIVE)
                    .ok()
                    .and_then(|bytes| String::from_utf8(bytes).ok())
            })
            .flatten();
        let configured_evidence_root = std::env::var_os("UQM_CI_EVIDENCE_ROOT")
            .map(|path| PathBuf::from(path).display().to_string());
        let cache_mode = std::env::var("UQM_CI_CACHE_MODE").ok().or_else(|| {
            authority_snapshot.as_deref().and_then(|snapshot| {
                serde_json::from_str::<Authority>(snapshot)
                    .ok()
                    .map(|authority| authority.cache.mode)
            })
        });
        let executable = std::env::current_exe()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|_| "uqm-xtask".to_string());
        let mut envelope = Self {
            schema: PRE_SESSION_SCHEMA.to_string(),
            passed: false,
            first_failed_contract: contract.to_string(),
            detail: detail.to_string(),
            requested_gate: requested_gate.to_string(),
            tuple: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
            cache_mode: cache_mode.unwrap_or_else(|| "authority-unavailable".to_string()),
            configured_evidence_root,
            authority_snapshot,
            controller_command: vec![
                executable,
                "ci".to_string(),
                "run".to_string(),
                requested_gate.to_string(),
            ],
            offline_validation: OfflineValidation {
                passed: false,
                contracts: Vec::new(),
            },
        };
        envelope.offline_validation = pre_session_validation(&envelope);
        envelope
    }
}

impl EvidenceIndex {
    /// Build the index and run offline validation against the current repository.
    pub fn build_and_validate(
        root: &Path,
        supported_tuples: &[String],
        context: EvidenceContext,
        entries: Vec<EvidenceEntry>,
    ) -> Result<Self, String> {
        let index = Self {
            schema: EVIDENCE_SCHEMA.to_string(),
            source_sha: context.source_sha,
            clean: context.clean,
            tuple: context.tuple,
            supported_tuples: supported_tuples.to_vec(),
            profile: PROFILE.to_string(),
            features: context.features,
            cache_mode: context.cache_mode,
            first_failed_contract: context.first_failed_contract,
            offline_validation: OfflineValidation {
                passed: false,
                contracts: Vec::new(),
            },
            entries,
        };
        with_snapshot(root, || {
            let mut contracts = validate_index(root, supported_tuples, &index)
                .map_err(|error| format!("evidence: {error}"))?;
            contracts.extend(validate_authority_snapshot(root, &index));
            Ok(index.with_validation(contracts))
        })
        .map_err(|error| format!("cannot snapshot evidence bundle: {error}"))?
    }

    fn with_validation(mut self, contracts: Vec<String>) -> Self {
        self.offline_validation = OfflineValidation {
            passed: contracts.is_empty(),
            contracts,
        };
        self
    }
}

fn pre_session_validation(envelope: &PreSessionFailureEnvelope) -> OfflineValidation {
    let mut contracts = Vec::new();
    if envelope.schema != PRE_SESSION_SCHEMA {
        contracts.push("pre_session.schema".to_string());
    }
    if envelope.passed {
        contracts.push("pre_session.result".to_string());
    }
    let failure = envelope.first_failed_contract.as_str();
    let known_failure = matches!(
        failure,
        "authority.load"
            | "authority.validate"
            | "authority.gate"
            | "plan.derive"
            | "environment.tuple"
            | "evidence.root"
            | "source.head"
            | "source.sha"
            | "source.status"
            | "ownership.delta_measure"
            | "cache.mode"
            | "cache.prepare"
            | "cache.inspect"
            | "cache.receipt"
            | "evidence.finalize"
    );
    if !known_failure {
        contracts.push("pre_session.first_failed_contract".to_string());
    }
    if envelope.detail.trim().is_empty() {
        contracts.push("pre_session.detail".to_string());
    }
    if envelope.requested_gate.is_empty()
        || envelope.requested_gate.len() > 128
        || !envelope
            .requested_gate
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b"-"[0])
    {
        contracts.push("pre_session.requested_gate".to_string());
    }
    if envelope.tuple.is_empty()
        || !envelope
            .tuple
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_".contains(&byte))
    {
        contracts.push("pre_session.tuple".to_string());
    }
    if !matches!(
        envelope.cache_mode.as_str(),
        "isolated-empty" | "ambient-dev"
    ) && failure != "cache.mode"
    {
        contracts.push("pre_session.cache_mode".to_string());
    }
    if envelope
        .configured_evidence_root
        .as_deref()
        .is_some_and(str::is_empty)
    {
        contracts.push("pre_session.configured_evidence_root".to_string());
    }
    if envelope.controller_command.len() != 4
        || envelope.controller_command[0].is_empty()
        || envelope.controller_command[1] != "ci"
        || envelope.controller_command[2] != "run"
        || envelope.controller_command[3] != envelope.requested_gate
    {
        contracts.push("pre_session.controller_command".to_string());
    }
    if failure == "authority.load" {
        if envelope.authority_snapshot.is_some() {
            contracts.push("pre_session.authority.unexpected".to_string());
        }
    } else {
        match envelope
            .authority_snapshot
            .as_deref()
            .map(serde_json::from_str::<Authority>)
        {
            None => contracts.push("pre_session.authority.missing".to_string()),
            Some(Err(_)) if failure != "authority.validate" => {
                contracts.push("pre_session.authority.parse".to_string());
            }
            Some(Ok(authority)) => {
                let authority_result = validate_authority(&authority);
                if failure == "authority.validate" {
                    if authority_result.is_ok() {
                        contracts.push("pre_session.authority.unexpected_valid".to_string());
                    }
                } else if authority_result.is_err() {
                    contracts.push("pre_session.authority.invalid".to_string());
                } else {
                    let requested_exists = envelope.requested_gate == "all"
                        || authority
                            .gates
                            .iter()
                            .any(|gate| gate.id == envelope.requested_gate);
                    if (failure == "authority.gate" && requested_exists)
                        || (failure != "authority.gate" && !requested_exists)
                    {
                        contracts.push("pre_session.authority.gate".to_string());
                    }
                    let tuple_exists = authority
                        .runner_mapping
                        .iter()
                        .any(|mapping| mapping.tuple == envelope.tuple);
                    if (failure == "environment.tuple" && tuple_exists)
                        || (failure != "environment.tuple" && !tuple_exists)
                    {
                        contracts.push("pre_session.authority.tuple".to_string());
                    }
                }
            }
            Some(Err(_)) => {}
        }
    }
    OfflineValidation {
        passed: contracts.is_empty(),
        contracts,
    }
}
fn validate_transport_index(root: &Path, index: &TransportIndex) -> Vec<String> {
    match with_snapshot(root, || validate_transport_index_snapshot(root, index)) {
        Ok(contracts) => contracts,
        Err(error) => vec![format!("transport.files.snapshot ({error})")],
    }
}

fn validate_transport_index_snapshot(root: &Path, index: &TransportIndex) -> Vec<String> {
    let mut contracts = Vec::new();
    if active_rejected_paths(root).is_ok_and(|paths| !paths.is_empty()) {
        contracts.push("transport.files.symlink".to_string());
    }
    if index.schema != TRANSPORT_SCHEMA {
        contracts.push("transport.schema".to_string());
    }
    if !is_hex(&index.source_sha, 40) {
        contracts.push("transport.source_sha".to_string());
    }
    if !matches!(index.job.as_str(), "plan" | "gates" | "required-gates") {
        contracts.push("transport.job".to_string());
    }
    let mut indexed_paths = BTreeSet::new();
    let mut previous_path: Option<&str> = None;
    for entry in &index.files {
        if !validate_relative_path(&entry.path)
            || previous_path.is_some_and(|previous| previous >= entry.path.as_str())
            || !indexed_paths.insert(entry.path.clone())
        {
            contracts.push("transport.files.order_or_path".to_string());
        }
        previous_path = Some(&entry.path);
        match read_bundle_file(root, &entry.path) {
            Ok(bytes) => {
                if entry.byte_length != bytes.len() as u64 {
                    contracts.push(format!("transport.file.{}.byte_length", entry.path));
                }
                if !is_hex(&entry.sha256, 64) || entry.sha256 != hex_sha256(&bytes) {
                    contracts.push(format!("transport.file.{}.sha256", entry.path));
                }
            }
            Err(error) => contracts.push(format!("transport.file.{}.read ({error})", entry.path)),
        }
    }
    let actual_paths = regular_file_inventory(root)
        .map(|files| {
            files
                .into_iter()
                .map(|file| file.relative_path)
                .filter(|path| path != "index.json")
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_else(|error| {
            contracts.push(format!("transport.files.snapshot ({error})"));
            BTreeSet::new()
        });
    if indexed_paths != actual_paths {
        contracts.push("transport.files.completeness".to_string());
    }

    match index.job.as_str() {
        "plan" => validate_workflow_setup(
            root,
            index,
            "plan",
            &["plan-build", "checkout", "plan"],
            &mut contracts,
        ),
        "gates" => {
            let mut expected_steps = vec![
                "xtask-build",
                "checkout",
                "architecture",
                "prerequisites",
                "tools",
                "native-content",
            ];
            expected_steps.push("containment-check");
            expected_steps.push("authoritative-gates");
            expected_steps.push("source-revalidation");
            validate_workflow_setup(root, index, "gates", &expected_steps, &mut contracts);
        }
        "required-gates" => validate_required_result(root, index, &mut contracts),
        _ => {}
    }
    contracts
}

#[cfg(test)]
fn collect_transport_paths(
    root: &Path,
    _directory: &Path,
    paths: &mut Vec<String>,
    contracts: &mut Vec<String>,
) {
    match regular_file_inventory(root) {
        Ok(files) => paths.extend(
            files
                .into_iter()
                .map(|file| file.relative_path)
                .filter(|path| path != "index.json"),
        ),
        Err(error) => contracts.push(format!("transport.files.snapshot ({error})")),
    }
}

fn validate_workflow_setup(
    root: &Path,
    index: &TransportIndex,
    job: &str,
    expected_steps: &[&str],
    contracts: &mut Vec<String>,
) {
    if index.exit_code.is_some()
        || !index
            .job_status
            .as_deref()
            .is_some_and(|status| matches!(status, "success" | "failure" | "cancelled" | "skipped"))
        || (job == "gates" && index.tuple.as_deref().is_none_or(|tuple| tuple.is_empty()))
        || (job == "plan" && index.tuple.is_some())
    {
        contracts.push(format!("transport.{job}.identity"));
    }
    let setup: Option<WorkflowSetupResults> = read_bundle_file(root, "workflow-setup-results.json")
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok());
    let Some(setup) = setup else {
        contracts.push(format!("transport.{job}.setup"));
        return;
    };
    if setup.schema != WORKFLOW_SETUP_SCHEMA
        || setup.job != job
        || setup.source_sha != index.source_sha
        || setup.tuple != index.tuple
        || setup.steps.len() != expected_steps.len()
    {
        contracts.push(format!("transport.{job}.setup_identity"));
    }
    let mut first_non_success = None;
    for (position, expected) in expected_steps.iter().enumerate() {
        let Some(step) = setup.steps.get(position) else {
            continue;
        };
        if step.step != *expected
            || !matches!(
                step.outcome.as_str(),
                "success" | "failure" | "cancelled" | "skipped"
            )
        {
            contracts.push(format!("transport.{job}.setup_step"));
        }
        if step.outcome != "success" && first_non_success.is_none() {
            first_non_success = Some(position);
        } else if first_non_success.is_some()
            && step.outcome != "skipped"
            && !(*expected == "source-revalidation"
                && matches!(step.outcome.as_str(), "success" | "failure"))
        {
            contracts.push(format!("transport.{job}.setup_prefix"));
        }
    }
    let expected_status = first_non_success
        .and_then(|position| setup.steps.get(position))
        .map_or("success", |step| step.outcome.as_str());
    if index.job_status.as_deref() != Some(expected_status) {
        contracts.push(format!("transport.{job}.result"));
    }
    let authority = read_bundle_file(root, "authority-snapshot.json")
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Authority>(&bytes).ok());
    let checkout_succeeded = setup
        .steps
        .iter()
        .any(|step| step.step == "checkout" && step.outcome == "success");
    match authority.as_ref() {
        Some(authority) => {
            if validate_authority(authority).is_err()
                || (job == "gates"
                    && index.tuple.as_deref().is_some_and(|tuple| {
                        !authority
                            .runner_mapping
                            .iter()
                            .any(|item| item.tuple == tuple)
                    }))
            {
                contracts.push(format!("transport.{job}.authority"));
            }
        }
        None if checkout_succeeded => contracts.push(format!("transport.{job}.authority")),
        None => {}
    }
    validate_workflow_subprocess_receipts(root, index, job, &setup, authority.as_ref(), contracts);
    if job == "gates" {
        let gates_outcome = setup
            .steps
            .iter()
            .find(|step| step.step == "authoritative-gates")
            .map(|step| step.outcome.as_str());
        let nested: Vec<_> = index
            .files
            .iter()
            .filter(|entry| {
                entry.path.ends_with(&format!("/{INDEX_FILENAME}"))
                    || entry.path.ends_with(&format!("/{PRE_SESSION_FILENAME}"))
            })
            .collect();
        if gates_outcome == Some("skipped") {
            if !nested.is_empty() {
                contracts.push("transport.gates.unexpected_nested_evidence".to_string());
            }
        } else if gates_outcome == Some("cancelled") && nested.is_empty() {
            // GitHub can terminate the child before it can finalize its own bundle.
        } else if nested.len() != 1 {
            contracts.push("transport.gates.nested_evidence_count".to_string());
        } else if let Err(error) = validate_nested_transport_evidence(
            root,
            &nested[0].path,
            gates_outcome,
            &index.source_sha,
            index.tuple.as_deref(),
        ) {
            contracts.push(format!("transport.gates.nested_evidence ({error})"));
        }
    }
    if job == "plan" {
        validate_plan_payload(root, index, &setup, authority.as_ref(), contracts);
    }
}

fn valid_descendant_start_identity(value: &serde_json::Value, scope: &str) -> bool {
    match scope {
        "child-subreaper-descendant-tree" => value.as_u64().is_some(),
        "observed-descendant-tree" => value.as_array().is_some_and(|parts| {
            parts.len() == 2 && parts.iter().all(|part| part.as_u64().is_some())
        }),
        _ => false,
    }
}

fn valid_workflow_descendant_receipt(
    value: &serde_json::Value,
    launched: bool,
    expected_scope: Option<&str>,
) -> bool {
    const LINUX_CEILING: &str = "the kernel reparents every orphaned descendant to this supervisor, so a detached descendant remains a tracked and reapable child until it exits";
    const DARWIN_CEILING: &str = "darwin has no child subreaper: a descendant that detaches and whose ancestors all exit before any supervisor observation passes is outside this tree; every observed escaped descendant is stopped, re-verified against its kernel start identity while stopped, and only then signaled, so an unrelated reused pid is at worst briefly stopped and resumed, never killed, while descendant discovery itself remains observational";

    let Some(scope) = value
        .get("descendant_tracking_scope")
        .and_then(|item| item.as_str())
    else {
        return false;
    };
    if expected_scope.is_some_and(|expected| expected != scope) {
        return false;
    }
    let expected_ceiling = match scope {
        "child-subreaper-descendant-tree" => LINUX_CEILING,
        "observed-descendant-tree" => DARWIN_CEILING,
        _ => return false,
    };
    if value
        .get("descendant_containment_ceiling")
        .and_then(|item| item.as_str())
        != Some(expected_ceiling)
    {
        return false;
    }
    let Some(observed) = value
        .get("descendants_observed")
        .and_then(|item| item.as_u64())
    else {
        return false;
    };
    let Some(escaped) = value
        .get("escaped_descendants_observed")
        .and_then(|item| item.as_u64())
    else {
        return false;
    };
    let terminated = value
        .get("descendants_terminated")
        .and_then(|item| item.as_bool());
    let Some(signals) = value
        .get("descendant_signals")
        .and_then(|item| item.as_array())
    else {
        return false;
    };
    if escaped > observed
        || (!launched
            && (terminated != Some(false) || observed != 0 || escaped != 0 || !signals.is_empty()))
    {
        return false;
    }
    let mut last = None;
    let mut uncontainable = false;
    for (sequence, signal) in signals.iter().enumerate() {
        let valid_sequence =
            signal.get("sequence").and_then(|item| item.as_u64()) == Some(sequence as u64);
        let valid_pid = signal
            .get("pid")
            .and_then(|item| item.as_u64())
            .is_some_and(|pid| pid > 0);
        let valid_name = signal
            .get("signal")
            .and_then(|item| item.as_str())
            .is_some_and(|name| matches!(name, "SIGTERM" | "SIGCONT" | "SIGKILL"));
        let valid_result = signal
            .get("result")
            .and_then(|item| item.as_str())
            .is_some_and(|result| {
                let allowed = match scope {
                    "child-subreaper-descendant-tree" => matches!(
                        result,
                        "delivered"
                            | "not-found"
                            | "permission-denied"
                            | "identity-changed"
                            | "pidfd-error"
                    ),
                    "observed-descendant-tree" => matches!(
                        result,
                        "delivered"
                            | "not-found"
                            | "permission-denied"
                            | "identity-changed"
                            | "signal-error"
                    ),
                    _ => false,
                };
                let errno_valid = if result == "pidfd-error" || result == "signal-error" {
                    signal
                        .get("errno")
                        .and_then(|item| item.as_i64())
                        .is_some_and(|errno| errno > 0)
                } else {
                    signal.get("errno").is_none()
                };
                uncontainable |=
                    matches!(result, "identity-changed" | "pidfd-error" | "signal-error");
                allowed && errno_valid
            });
        let valid_identity = signal
            .get("start_identity")
            .is_some_and(|identity| valid_descendant_start_identity(identity, scope));
        let Some(at) = signal
            .get("monotonic_milliseconds")
            .and_then(|item| item.as_u64())
        else {
            return false;
        };
        let Some(at_ns) = signal
            .get("monotonic_nanoseconds")
            .and_then(|item| item.as_u64())
        else {
            return false;
        };
        if !valid_sequence
            || !valid_pid
            || !valid_name
            || !valid_result
            || !valid_identity
            || at != at_ns / 1_000_000
            || last.is_some_and(|previous| previous > at_ns)
        {
            return false;
        }
        last = Some(at_ns);
    }
    !launched || terminated == Some(true) || terminated == Some(false) && uncontainable
}

fn valid_workflow_containment_receipt_for_tuple(
    value: &serde_json::Value,
    launched: bool,
    tuple: Option<&str>,
) -> bool {
    let expected_descendant_scope = match tuple {
        Some(tuple) if tuple.starts_with("macos-") => "observed-descendant-tree",
        Some(tuple) if tuple.starts_with("linux-") => "child-subreaper-descendant-tree",
        None => "child-subreaper-descendant-tree",
        Some(_) => return false,
    };
    if value
        .get("containment_scope")
        .and_then(|item| item.as_str())
        != Some("initial-process-group")
        || value
            .get("pgid_pinned_through_last_signal")
            .and_then(|item| item.as_bool())
            != Some(true)
        || !valid_workflow_descendant_receipt(value, launched, Some(expected_descendant_scope))
    {
        return false;
    }
    let Some(signals) = value.get("signals").and_then(|item| item.as_array()) else {
        return false;
    };
    let mut last = None;
    for (sequence, signal) in signals.iter().enumerate() {
        let valid_sequence =
            signal.get("sequence").and_then(|item| item.as_u64()) == Some(sequence as u64);
        let valid_name = signal
            .get("signal")
            .and_then(|item| item.as_str())
            .is_some_and(|name| matches!(name, "SIGTERM" | "SIGCONT" | "SIGKILL"));
        let valid_result = signal
            .get("result")
            .and_then(|item| item.as_str())
            .is_some_and(|result| {
                matches!(result, "delivered" | "not-found" | "permission-denied")
            });
        let Some(at) = signal
            .get("monotonic_milliseconds")
            .and_then(|item| item.as_u64())
        else {
            return false;
        };
        let Some(at_ns) = signal
            .get("monotonic_nanoseconds")
            .and_then(|item| item.as_u64())
        else {
            return false;
        };
        if !valid_sequence
            || !valid_name
            || !valid_result
            || at != at_ns / 1_000_000
            || last.is_some_and(|previous| previous > at_ns)
        {
            return false;
        }
        last = Some(at_ns);
    }
    let retained_last = value
        .get("last_signal_monotonic_nanoseconds")
        .and_then(|item| item.as_u64());
    let retained_last_ms = value
        .get("last_signal_monotonic_milliseconds")
        .and_then(|item| item.as_u64());
    let unpinned = value
        .get("leader_unpinned_monotonic_nanoseconds")
        .and_then(|item| item.as_u64());
    let unpinned_ms = value
        .get("leader_unpinned_monotonic_milliseconds")
        .and_then(|item| item.as_u64());
    retained_last == last
        && retained_last_ms == retained_last.map(|value| value / 1_000_000)
        && unpinned_ms == unpinned.map(|value| value / 1_000_000)
        && if launched {
            unpinned.is_some_and(|unpinned| last.is_none_or(|last| last < unpinned))
        } else {
            unpinned.is_none() && signals.is_empty()
        }
}

#[cfg(test)]
fn valid_workflow_containment_receipt(value: &serde_json::Value, launched: bool) -> bool {
    let scope = value
        .get("descendant_tracking_scope")
        .and_then(|item| item.as_str());
    let tuple = match scope {
        Some("observed-descendant-tree") => Some("macos-aarch64"),
        Some("child-subreaper-descendant-tree") => Some("linux-x86_64"),
        _ => Some("unsupported"),
    };
    valid_workflow_containment_receipt_for_tuple(value, launched, tuple)
}

fn collect_workflow_subprocess_receipts(
    root: &Path,
    index: &TransportIndex,
    job: &str,
    contracts: &mut Vec<String>,
) -> (BTreeSet<String>, BTreeSet<String>) {
    let mut successful = BTreeSet::new();
    let mut failed = BTreeSet::new();
    for entry in index
        .files
        .iter()
        .filter(|entry| entry.path.ends_with(".result.json"))
    {
        let value = read_bundle_file(root, &entry.path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok());
        let Some(value) = value else {
            contracts.push(format!("transport.{job}.subprocess_receipt"));
            continue;
        };
        if value.get("schema").and_then(|item| item.as_str())
            != Some("uqm-s4-workflow-subprocess-v1")
        {
            continue;
        }
        let identity = value.get("executable_identity");
        let valid_identity = identity.is_some_and(valid_workflow_executable_identity);
        let command_valid = value
            .get("command")
            .and_then(|item| item.as_array())
            .is_some_and(|command| {
                !command.is_empty()
                    && command.iter().all(|argument| {
                        argument
                            .as_str()
                            .is_some_and(|argument| !argument.is_empty())
                    })
            });
        let exit_code = value.get("exit_code").and_then(|item| item.as_i64());
        let launch_error = value.get("launch_error").and_then(|item| item.as_str());
        let failure = value.get("failure").and_then(|item| item.as_str());
        let group_empty = value
            .get("process_group_empty")
            .and_then(|item| item.as_bool());
        let success_terminal = exit_code == Some(0)
            && value.get("launch_error").is_some_and(|item| item.is_null())
            && value.get("failure").is_some_and(|item| item.is_null())
            && group_empty == Some(true);
        let failed_terminal = ((exit_code.is_some_and(|code| code != 0)
            && value.get("launch_error").is_some_and(|item| item.is_null()))
            || launch_error.is_some_and(|error| !error.is_empty())
            || failure.is_some_and(|error| !error.is_empty()))
            && (group_empty == Some(true)
                || (launch_error.is_some() && group_empty == Some(false)));
        let identity_valid_for_terminal = valid_identity
            || (launch_error.is_some_and(|error| !error.is_empty())
                && identity.is_some_and(serde_json::Value::is_null));
        let containment_valid = valid_workflow_containment_receipt_for_tuple(
            &value,
            exit_code.is_some(),
            index.tuple.as_deref(),
        );
        if !identity_valid_for_terminal
            || !command_valid
            || !containment_valid
            || (!success_terminal && !failed_terminal)
        {
            contracts.push(format!("transport.{job}.subprocess_receipt"));
        } else if success_terminal {
            successful.insert(entry.path.clone());
        } else {
            failed.insert(entry.path.clone());
        }
    }
    (successful, failed)
}

fn valid_workflow_executable_identity(identity: &serde_json::Value) -> bool {
    identity
        .get("path")
        .and_then(|item| item.as_str())
        .is_some_and(|path| !path.is_empty())
        && identity
            .get("byte_length")
            .and_then(|item| item.as_u64())
            .is_some_and(|length| length > 0)
        && identity
            .get("sha256")
            .and_then(|item| item.as_str())
            .is_some_and(|digest| is_hex(digest, 64))
        && identity
            .get("mode")
            .and_then(|item| item.as_u64())
            .is_some_and(|mode| mode & 0o111 != 0)
}

fn workflow_step_has_outcome(setup: &WorkflowSetupResults, name: &str, outcome: &str) -> bool {
    setup
        .steps
        .iter()
        .any(|step| step.step == name && step.outcome == outcome)
}

fn receipt_set_has_name(receipts: &BTreeSet<String>, name: &str) -> bool {
    receipts
        .iter()
        .any(|path| path.rsplit('/').next() == Some(name))
}

fn receipt_set_has_prefix(receipts: &BTreeSet<String>, prefix: &str) -> bool {
    receipts.iter().any(|path| {
        path.rsplit('/')
            .next()
            .is_some_and(|name| name.starts_with(prefix))
    })
}

fn validate_successful_workflow_subprocess_receipts(
    index: &TransportIndex,
    job: &str,
    setup: &WorkflowSetupResults,
    authority: Option<&Authority>,
    successful: &BTreeSet<String>,
    contracts: &mut Vec<String>,
) {
    let step_succeeded = |name| workflow_step_has_outcome(setup, name, "success");
    let has_success = |name| receipt_set_has_name(successful, name);
    let required: &[&str] = if job == "plan" && step_succeeded("plan") {
        &[
            "bootstrap-apt-update.result.json",
            "bootstrap-apt-install.result.json",
            "bootstrap-rustup.result.json",
            "bootstrap-xtask-build.result.json",
            "ci-plan.result.json",
        ]
    } else if job == "gates" && step_succeeded("authoritative-gates") {
        &["xtask-build.result.json", "ci-run.result.json"]
    } else {
        &[]
    };
    if required.iter().any(|name| !has_success(name)) {
        contracts.push(format!("transport.{job}.subprocess_receipts"));
    }
    if job == "gates"
        && step_succeeded("source-revalidation")
        && !has_success("source-revalidation.result.json")
    {
        contracts.push("transport.gates.source_revalidation_receipt".to_string());
    }
    if job == "gates"
        && step_succeeded("authoritative-gates")
        && !has_success("containment-check.result.json")
    {
        contracts.push("transport.gates.uid_containment_receipt".to_string());
    }
    if job == "gates"
        && step_succeeded("prerequisites")
        && !receipt_set_has_prefix(successful, "prerequisites-")
    {
        contracts.push("transport.gates.prerequisite_receipts".to_string());
    }
    if job == "gates" && step_succeeded("tools") {
        let required_tools = [
            "tools-rustup.result.json",
            "tools-venv.result.json",
            "tools-lizard.result.json",
            "tools-cargo-audit.result.json",
            "tools-cargo-llvm-cov.result.json",
            "tools-actionlint.result.json",
        ];
        if required_tools.iter().any(|name| !has_success(name)) {
            contracts.push("transport.gates.tool_receipts".to_string());
        }
        if authority.is_some_and(|authority| {
            authority.tools.rust.components.iter().any(|component| {
                let name = format!("tools-component-{component}.result.json");
                !receipt_set_has_name(successful, &name)
            })
        }) {
            contracts.push("transport.gates.tool_component_receipts".to_string());
        }
    }
    if job == "gates"
        && step_succeeded("native-content")
        && authority.is_some_and(|authority| {
            index
                .tuple
                .as_deref()
                .is_some_and(|tuple| tuple.starts_with(&authority.native_acceptance.platform))
        })
        && !has_success("native-content.result.json")
    {
        contracts.push("transport.gates.native_content_receipt".to_string());
    }
}

fn validate_failed_workflow_subprocess_receipts(
    index: &TransportIndex,
    job: &str,
    setup: &WorkflowSetupResults,
    authority: Option<&Authority>,
    failed: &BTreeSet<String>,
    contracts: &mut Vec<String>,
) {
    let step_failed = |name| workflow_step_has_outcome(setup, name, "failure");
    let has_failed = |name| receipt_set_has_name(failed, name);
    if job == "plan" && step_failed("plan") && failed.is_empty() {
        contracts.push("transport.plan.failed_subprocess_receipt".to_string());
    }
    if job != "gates" {
        return;
    }
    if step_failed("prerequisites") && !receipt_set_has_prefix(failed, "prerequisites-") {
        contracts.push("transport.gates.failed_prerequisite_receipt".to_string());
    }
    if step_failed("tools") && !receipt_set_has_prefix(failed, "tools-") {
        contracts.push("transport.gates.failed_tool_receipt".to_string());
    }
    if step_failed("native-content")
        && authority.is_some_and(|authority| {
            index
                .tuple
                .as_deref()
                .is_some_and(|tuple| tuple.starts_with(&authority.native_acceptance.platform))
        })
        && !has_failed("native-content.result.json")
    {
        contracts.push("transport.gates.failed_native_content_receipt".to_string());
    }
    if step_failed("containment-check") && !has_failed("containment-check.result.json") {
        contracts.push("transport.gates.failed_uid_containment_receipt".to_string());
    }
    if step_failed("xtask-build") && !has_failed("xtask-build.result.json") {
        contracts.push("transport.gates.failed_xtask_build_receipt".to_string());
    }
    if step_failed("authoritative-gates") && !has_failed("ci-run.result.json") {
        contracts.push("transport.gates.failed_ci_run_receipt".to_string());
    }
    if step_failed("source-revalidation") && !has_failed("source-revalidation.result.json") {
        contracts.push("transport.gates.failed_source_revalidation_receipt".to_string());
    }
}

fn validate_workflow_subprocess_receipts(
    root: &Path,
    index: &TransportIndex,
    job: &str,
    setup: &WorkflowSetupResults,
    authority: Option<&Authority>,
    contracts: &mut Vec<String>,
) {
    let (successful, failed) = collect_workflow_subprocess_receipts(root, index, job, contracts);
    validate_successful_workflow_subprocess_receipts(
        index,
        job,
        setup,
        authority,
        &successful,
        contracts,
    );
    validate_failed_workflow_subprocess_receipts(index, job, setup, authority, &failed, contracts);
}

fn validate_plan_payload(
    root: &Path,
    index: &TransportIndex,
    setup: &WorkflowSetupResults,
    authority: Option<&Authority>,
    contracts: &mut Vec<String>,
) {
    if !setup
        .steps
        .iter()
        .any(|step| step.step == "plan" && step.outcome == "success")
    {
        return;
    }
    let plan: Option<super::plan::Plan> = read_bundle_file(root, "ci-plan.json")
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok());
    let Some(authority) = authority else {
        contracts.push("transport.plan.payload.authority".to_string());
        return;
    };
    let expected_authority = serde_json::to_value(authority).ok();
    let valid = plan.as_ref().is_some_and(|plan| {
        plan.schema == super::plan::PLAN_SCHEMA
            && plan.authority == super::authority::AUTHORITY_RELATIVE
            && plan.authority_contract.as_ref() == expected_authority.as_ref()
            && plan.tuples.len() == authority.runner_mapping.len()
            && plan
                .tuples
                .iter()
                .zip(&authority.runner_mapping)
                .all(|(tuple, mapping)| {
                    let expected = mapping.tuple.split_once('-');
                    tuple.tuple == mapping.tuple
                        && tuple.runner == mapping.runner
                        && tuple.expected_uname == mapping.expected_uname
                        && expected.is_some_and(|(os, architecture)| {
                            tuple.os == os && tuple.architecture == architecture
                        })
                })
    });
    if !valid {
        contracts.push("transport.plan.payload".to_string());
    }
    if !index.files.iter().any(|entry| entry.path == "ci-plan.json")
        || !index
            .files
            .iter()
            .any(|entry| entry.path == "ci-plan.stderr.log")
    {
        contracts.push("transport.plan.payload.files".to_string());
    }
}

fn validate_nested_transport_evidence(
    transport_root: &Path,
    relative: &str,
    workflow_outcome: Option<&str>,
    source_sha: &str,
    tuple: Option<&str>,
) -> Result<(), String> {
    let bytes = read_bundle_file(transport_root, relative)
        .map_err(|error| format!("cannot read nested evidence: {error}"))?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid nested evidence JSON: {error}"))?;
    let nested_root = transport_root
        .join(relative)
        .parent()
        .ok_or_else(|| "nested evidence has no parent".to_string())?
        .to_path_buf();
    match value.get("schema").and_then(|schema| schema.as_str()) {
        Some(EVIDENCE_SCHEMA) => {
            let index: EvidenceIndex = serde_json::from_value(value)
                .map_err(|error| format!("invalid nested evidence index: {error}"))?;
            let mut contracts = validate_index(&nested_root, &index.supported_tuples, &index)
                .map_err(|error| format!("invalid nested evidence: {error}"))?;
            contracts.extend(validate_authority_snapshot(&nested_root, &index));
            if !contracts.is_empty() {
                return Err(contracts[0].clone());
            }
            let controller_runs_all = index.entries.iter().any(|entry| {
                entry.role == "authority.snapshot"
                    && entry.producing_command.get(3).map(String::as_str) == Some("all")
            });
            if !controller_runs_all {
                return Err(
                    "nested evidence did not execute the complete gate authority".to_string(),
                );
            }
            if index.source_sha != source_sha || Some(index.tuple.as_str()) != tuple {
                return Err("nested evidence identity contradicts transport".to_string());
            }
            if (workflow_outcome == Some("success")) != index.first_failed_contract.is_none() {
                return Err("nested evidence result contradicts workflow outcome".to_string());
            }
        }
        Some(PRE_SESSION_SCHEMA) => {
            let mut envelope: PreSessionFailureEnvelope = serde_json::from_value(value)
                .map_err(|error| format!("invalid nested pre-session evidence: {error}"))?;
            envelope.offline_validation = pre_session_validation(&envelope);
            if !envelope.offline_validation.passed || workflow_outcome == Some("success") {
                return Err("nested pre-session evidence contradicts workflow outcome".to_string());
            }
            if envelope.requested_gate != "all" {
                return Err("nested pre-session evidence did not request all gates".to_string());
            }
            if envelope.first_failed_contract != "environment.tuple"
                && Some(envelope.tuple.as_str()) != tuple
            {
                return Err("nested pre-session tuple contradicts transport".to_string());
            }
        }
        _ => return Err("unknown nested evidence schema".to_string()),
    }
    Ok(())
}

fn validate_required_result(root: &Path, index: &TransportIndex, contracts: &mut Vec<String>) {
    if index.tuple.is_some() || index.exit_code.is_some() || index.job_status.is_some() {
        contracts.push("transport.required-gates.identity".to_string());
    }
    let result: Option<RequiredResult> = read_bundle_file(root, "required-result.json")
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok());
    let valid = result.is_some_and(|result| {
        result.schema == REQUIRED_RESULT_SCHEMA
            && result.source_sha == index.source_sha
            && matches!(
                result.plan.as_str(),
                "success" | "failure" | "cancelled" | "skipped"
            )
            && matches!(
                result.gates.as_str(),
                "success" | "failure" | "cancelled" | "skipped"
            )
    });
    if !valid || index.files.len() != 1 || index.files[0].path != "required-result.json" {
        contracts.push("transport.required-gates.result".to_string());
    }
}

pub fn write_pre_session_failure(
    root: &Path,
    destination_root: &Path,
    requested_gate: &str,
    contract: &str,
    detail: &str,
) -> Result<PathBuf, String> {
    fs::create_dir_all(destination_root).map_err(|error| {
        format!(
            "cannot create pre-session evidence root {}: {error}",
            destination_root.display()
        )
    })?;
    let envelope = PreSessionFailureEnvelope::build(root, requested_gate, contract, detail);
    if !envelope.offline_validation.passed {
        return Err(format!(
            "pre-session envelope failed validation: {}",
            envelope.offline_validation.contracts[0]
        ));
    }
    let path = destination_root.join(PRE_SESSION_FILENAME);
    let bytes = serde_json::to_vec_pretty(&envelope)
        .map_err(|error| format!("cannot serialize pre-session evidence: {error}"))?;
    let mut temporary = tempfile::NamedTempFile::new_in(destination_root).map_err(|error| {
        format!(
            "cannot create temporary pre-session evidence in {}: {error}",
            destination_root.display()
        )
    })?;
    temporary.write_all(&bytes).map_err(|error| {
        format!(
            "cannot write temporary pre-session evidence in {}: {error}",
            destination_root.display()
        )
    })?;
    temporary.as_file_mut().sync_all().map_err(|error| {
        format!(
            "cannot synchronize temporary pre-session evidence in {}: {error}",
            destination_root.display()
        )
    })?;
    temporary.persist_noclobber(&path).map_err(|error| {
        format!(
            "cannot publish pre-session evidence {}: {}",
            path.display(),
            error.error
        )
    })?;
    Ok(path)
}

fn safe_transport_identifier(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.".contains(&byte))
}

fn validate_upload_receipt(receipt: &UploadReceipt, expected_retention_days: u32) -> Vec<String> {
    let mut contracts = Vec::new();
    if receipt.schema != UPLOAD_RECEIPT_SCHEMA {
        contracts.push("upload-receipt.schema".to_string());
    }
    if !is_hex(&receipt.source_sha, 40) {
        contracts.push("upload-receipt.source_sha".to_string());
    }
    let expected_name_prefix = match receipt.job.as_str() {
        "plan" if receipt.tuple.is_none() => Some("s4-plan-".to_string()),
        "gates" => receipt
            .tuple
            .as_deref()
            .filter(|tuple| safe_transport_identifier(tuple))
            .map(|tuple| format!("s4-{tuple}-")),
        "required-gates" if receipt.tuple.is_none() => Some("s4-required-".to_string()),
        _ => None,
    };
    if expected_name_prefix
        .as_deref()
        .is_none_or(|prefix| !receipt.artifact_name.starts_with(prefix))
        || !safe_transport_identifier(&receipt.artifact_name)
    {
        contracts.push("upload-receipt.identity".to_string());
    }
    if receipt.retention_days != expected_retention_days {
        contracts.push("upload-receipt.retention".to_string());
    }
    if !matches!(
        receipt.upload_outcome.as_str(),
        "success" | "failure" | "cancelled" | "skipped"
    ) {
        contracts.push("upload-receipt.outcome".to_string());
    }
    if receipt.upload_outcome == "success" {
        let id = receipt.artifact_id.filter(|id| *id > 0);
        let valid_url = id.is_some_and(|id| {
            receipt.artifact_url.as_deref().is_some_and(|url| {
                url.starts_with("https://github.com/") && url.ends_with(&format!("/artifacts/{id}"))
            })
        });
        let valid_digest = receipt.artifact_digest.as_deref().is_some_and(|digest| {
            is_hex(digest, 64)
                || digest
                    .strip_prefix("sha256:")
                    .is_some_and(|value| is_hex(value, 64))
        });
        if id.is_none()
            || !valid_url
            || !valid_digest
            || !receipt.size_in_bytes.is_some_and(|size| size > 0)
        {
            contracts.push("upload-receipt.transport".to_string());
        }
    } else if receipt.artifact_id.is_some()
        || receipt.artifact_url.is_some()
        || receipt.artifact_digest.is_some()
        || receipt.size_in_bytes.is_some()
    {
        contracts.push("upload-receipt.failed_transport".to_string());
    }
    contracts
}

fn validate_upload_authority_unavailable_receipt(
    receipt: &UploadAuthorityUnavailableReceipt,
) -> Vec<String> {
    let mut contracts = Vec::new();
    if receipt.schema != UPLOAD_AUTHORITY_UNAVAILABLE_SCHEMA {
        contracts.push("upload-authority-unavailable.schema".to_string());
    }
    if receipt.job != "plan"
        || !is_hex(&receipt.source_sha, 40)
        || !receipt.artifact_name.starts_with("s4-plan-")
        || !safe_transport_identifier(&receipt.artifact_name)
    {
        contracts.push("upload-authority-unavailable.identity".to_string());
    }
    if receipt.retention_days.is_some() || receipt.size_in_bytes.is_some() {
        contracts.push("upload-authority-unavailable.unknown-authority".to_string());
    }
    if receipt.failure != "exact authority could not be resolved before checkout" {
        contracts.push("upload-authority-unavailable.failure".to_string());
    }
    if !matches!(
        receipt.upload_outcome.as_str(),
        "success" | "failure" | "cancelled" | "skipped"
    ) {
        contracts.push("upload-authority-unavailable.outcome".to_string());
    }
    if receipt.upload_outcome == "success" {
        let id = receipt.artifact_id.filter(|id| *id > 0);
        let valid_url = id.is_some_and(|id| {
            receipt.artifact_url.as_deref().is_some_and(|url| {
                url.starts_with("https://github.com/") && url.ends_with(&format!("/artifacts/{id}"))
            })
        });
        let valid_digest = receipt.artifact_digest.as_deref().is_some_and(|digest| {
            is_hex(digest, 64)
                || digest
                    .strip_prefix("sha256:")
                    .is_some_and(|value| is_hex(value, 64))
        });
        if id.is_none() || !valid_url || !valid_digest {
            contracts.push("upload-authority-unavailable.transport".to_string());
        }
    } else if receipt.artifact_id.is_some()
        || receipt.artifact_url.is_some()
        || receipt.artifact_digest.is_some()
    {
        contracts.push("upload-authority-unavailable.failed-transport".to_string());
    }
    contracts
}

fn validate_transport_finalizer_fallback(fallback: &TransportFinalizerFallback) -> Vec<String> {
    let mut contracts = Vec::new();
    if fallback.schema != TRANSPORT_FALLBACK_SCHEMA {
        contracts.push("transport-fallback.schema".to_string());
    }
    if !is_hex(&fallback.source_sha, 40) {
        contracts.push("transport-fallback.source_sha".to_string());
    }
    let identity_valid = match fallback.job.as_str() {
        "plan" | "required-gates" => fallback.tuple.is_none(),
        "gates" => fallback
            .tuple
            .as_deref()
            .is_some_and(|tuple| !tuple.is_empty() && validate_relative_path(tuple)),
        _ => false,
    };
    if !identity_valid {
        contracts.push("transport-fallback.identity".to_string());
    }
    if fallback.first_failed_contract != "transport.finalize" || fallback.detail.is_empty() {
        contracts.push("transport-fallback.failure".to_string());
    }
    contracts
}

/// `ci validate-evidence <path>` entry point.
pub fn validate_evidence_command(root: &Path, path: &str) -> Result<(), String> {
    let requested = PathBuf::from(path);
    let absolute = if requested.is_absolute() {
        requested
    } else {
        root.join(requested)
    };
    let parent = absolute.parent().ok_or_else(|| {
        format!(
            "evidence index has no containing directory: {}",
            absolute.display()
        )
    })?;
    let snapshot = EvidenceSnapshot::open_validation(parent).map_err(|error| {
        format!(
            "cannot open immutable evidence snapshot {}: {error}",
            parent.display()
        )
    })?;
    snapshot.scoped(|| validate_evidence_command_snapshot(root, path))
}

fn validate_evidence_command_snapshot(root: &Path, path: &str) -> Result<(), String> {
    let path = PathBuf::from(path);
    let absolute = if path.is_absolute() {
        path.clone()
    } else {
        root.join(path)
    };
    let parent = absolute.parent().ok_or_else(|| {
        format!(
            "evidence index has no containing directory: {}",
            absolute.display()
        )
    })?;
    let filename = absolute
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("evidence index name is not UTF-8: {}", absolute.display()))?;
    let bytes = read_bundle_file(parent, filename)
        .map_err(|error| format!("cannot read {}: {error}", absolute.display()))?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid evidence JSON {}: {error}", absolute.display()))?;
    match value.get("schema").and_then(|schema| schema.as_str()) {
        Some(TRANSPORT_FALLBACK_SCHEMA) => validate_transport_fallback_command(value, &absolute),
        Some(UPLOAD_AUTHORITY_UNAVAILABLE_SCHEMA) => {
            validate_authority_unavailable_upload_command(value, &absolute)
        }
        Some(UPLOAD_RECEIPT_SCHEMA) => validate_upload_receipt_command(value, &absolute),
        Some(PRE_SESSION_SCHEMA) => validate_pre_session_command(value, &absolute),
        Some(TRANSPORT_SCHEMA) => validate_transport_command(value, &absolute),
        _ => validate_index_command(value, &absolute),
    }
}

fn print_offline_validation<T: Serialize>(
    value: T,
    validation: OfflineValidation,
    label: &str,
) -> Result<(), String> {
    let mut output = serde_json::to_value(value)
        .map_err(|error| format!("cannot serialize {label}: {error}"))?;
    output
        .as_object_mut()
        .ok_or_else(|| format!("serialized {label} is not an object"))?
        .insert(
            "offline_validation".to_string(),
            serde_json::to_value(&validation)
                .map_err(|error| format!("cannot serialize {label} validation: {error}"))?,
        );
    println!(
        "{}",
        serde_json::to_string_pretty(&output)
            .map_err(|error| format!("cannot format {label}: {error}"))?
    );
    if validation.passed {
        Ok(())
    } else {
        Err(format!(
            "offline {label} validation failed at first contract: {}",
            validation.contracts[0]
        ))
    }
}

fn validate_transport_fallback_command(
    value: serde_json::Value,
    absolute: &Path,
) -> Result<(), String> {
    let fallback: TransportFinalizerFallback = serde_json::from_value(value).map_err(|error| {
        format!(
            "invalid transport finalizer fallback {}: {error}",
            absolute.display()
        )
    })?;
    let contracts = validate_transport_finalizer_fallback(&fallback);
    print_offline_validation(
        fallback,
        OfflineValidation {
            passed: contracts.is_empty(),
            contracts,
        },
        "transport fallback",
    )
}

fn validate_authority_unavailable_upload_command(
    value: serde_json::Value,
    absolute: &Path,
) -> Result<(), String> {
    let receipt: UploadAuthorityUnavailableReceipt =
        serde_json::from_value(value).map_err(|error| {
            format!(
                "invalid authority-unavailable upload receipt {}: {error}",
                absolute.display()
            )
        })?;
    let contracts = validate_upload_authority_unavailable_receipt(&receipt);
    print_offline_validation(
        receipt,
        OfflineValidation {
            passed: contracts.is_empty(),
            contracts,
        },
        "authority-unavailable upload receipt",
    )
}

fn validate_upload_receipt_command(
    value: serde_json::Value,
    absolute: &Path,
) -> Result<(), String> {
    let receipt: UploadReceipt = serde_json::from_value(value)
        .map_err(|error| format!("invalid upload receipt {}: {error}", absolute.display()))?;
    let authority: Authority = serde_json::from_slice(include_bytes!("../../../ci/gates.json"))
        .map_err(|error| format!("cannot parse checked-in upload authority: {error}"))?;
    validate_authority(&authority)
        .map_err(|error| format!("invalid checked-in upload authority: {error}"))?;
    let contracts = validate_upload_receipt(
        &receipt,
        u32::from(authority.actions.artifact_retention_days),
    );
    print_offline_validation(
        receipt,
        OfflineValidation {
            passed: contracts.is_empty(),
            contracts,
        },
        "upload receipt",
    )
}

fn validate_pre_session_command(value: serde_json::Value, absolute: &Path) -> Result<(), String> {
    let mut envelope: PreSessionFailureEnvelope =
        serde_json::from_value(value).map_err(|error| {
            format!(
                "invalid pre-session evidence {}: {error}",
                absolute.display()
            )
        })?;
    envelope.offline_validation = pre_session_validation(&envelope);
    let text = serde_json::to_string_pretty(&envelope)
        .map_err(|error| format!("cannot serialize pre-session evidence: {error}"))?;
    println!("{text}");
    if envelope.offline_validation.passed {
        Ok(())
    } else {
        Err(format!(
            "offline validation failed at first contract: {}",
            envelope.offline_validation.contracts[0]
        ))
    }
}

fn validate_transport_command(value: serde_json::Value, absolute: &Path) -> Result<(), String> {
    let index: TransportIndex = serde_json::from_value(value).map_err(|error| {
        format!(
            "invalid transport evidence index {}: {error}",
            absolute.display()
        )
    })?;
    let bundle_root = absolute
        .parent()
        .ok_or_else(|| format!("transport index has no parent: {}", absolute.display()))?;
    let contracts = validate_transport_index(bundle_root, &index);
    print_offline_validation(
        index,
        OfflineValidation {
            passed: contracts.is_empty(),
            contracts,
        },
        "transport evidence",
    )
}

fn validate_index_contracts(
    bundle_root: &Path,
    index: &EvidenceIndex,
) -> Result<Vec<String>, String> {
    with_snapshot(bundle_root, || {
        let mut contracts = validate_index_snapshot(bundle_root, &index.supported_tuples, index)?;
        adversarial_hook("index-before-authority-validation");
        contracts.extend(validate_authority_snapshot(bundle_root, index));
        Ok::<Vec<String>, String>(contracts)
    })
    .map_err(|error| format!("cannot open immutable evidence snapshot: {error}"))?
    .map_err(|error| format!("evidence: {error}"))
}

fn validate_index_command(value: serde_json::Value, absolute: &Path) -> Result<(), String> {
    let index: EvidenceIndex = serde_json::from_value(value)
        .map_err(|error| format!("invalid evidence index {}: {error}", absolute.display()))?;
    let bundle_root = absolute
        .parent()
        .ok_or_else(|| format!("evidence index has no parent: {}", absolute.display()))?;
    let contracts = validate_index_contracts(bundle_root, &index)?;
    let output = index.with_validation(contracts);
    let text = serde_json::to_string_pretty(&output)
        .map_err(|error| format!("cannot serialize evidence: {error}"))?;
    println!("{text}");
    if output.offline_validation.passed {
        Ok(())
    } else {
        Err(format!(
            "offline validation failed at first contract: {}",
            output.offline_validation.contracts[0]
        ))
    }
}

/// Validate an evidence index against live repository state, returning each
/// violated contract in order. An empty return means the index is valid.
pub fn validate_index(
    root: &Path,
    supported_tuples: &[String],
    index: &EvidenceIndex,
) -> Result<Vec<String>, String> {
    with_snapshot(root, || {
        validate_index_snapshot(root, supported_tuples, index)
    })
    .map_err(|error| format!("cannot open immutable evidence snapshot: {error}"))?
}

fn validate_index_snapshot(
    root: &Path,
    supported_tuples: &[String],
    index: &EvidenceIndex,
) -> Result<Vec<String>, String> {
    let mut contracts = Vec::new();
    let authority = retained_authority(root, index);
    let gate_ids = authority
        .as_ref()
        .map(|authority| {
            authority
                .gates
                .iter()
                .map(|gate| gate.id.as_str())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if index.schema != EVIDENCE_SCHEMA {
        contracts.push(format!("evidence.schema (expected {EVIDENCE_SCHEMA})"));
    }
    if !is_hex(&index.source_sha, 40) {
        contracts.push("evidence.source_sha (must be 40 lowercase hex)".into());
    }
    if !index.clean
        && !index
            .first_failed_contract
            .as_deref()
            .is_some_and(is_setup_failure)
    {
        contracts.push("evidence.clean (must be true unless setup failed first)".into());
    }
    let embedded_tuples = index.supported_tuples.iter().collect::<BTreeSet<_>>();
    if index.supported_tuples.is_empty() || embedded_tuples.len() != index.supported_tuples.len() {
        contracts.push("evidence.supported_tuples (must be non-empty and unique)".into());
    }
    if !supported_tuples.contains(&index.tuple) {
        contracts.push(format!(
            "evidence.tuple (unsupported tuple '{}')",
            index.tuple
        ));
    }
    if index.profile != PROFILE {
        contracts.push("evidence.profile (must be 'ci')".into());
    }
    if index.features.is_empty() || index.features.iter().any(|feature| feature.is_empty()) {
        contracts.push("evidence.features (must be non-empty)".into());
    }
    if !CACHE_MODES.contains(&index.cache_mode.as_str()) {
        contracts.push(format!(
            "evidence.cache_mode (unsupported mode '{}')",
            index.cache_mode
        ));
    }
    if let Some(failed) = &index.first_failed_contract {
        let known_contract = authority.is_none()
            || is_setup_failure(failed)
            || gate_ids.iter().any(|gate| {
                failed == gate
                    || failed
                        .strip_prefix(*gate)
                        .is_some_and(|suffix| suffix.starts_with('.'))
            });
        if !known_contract {
            contracts.push("evidence.first_failed_contract (unknown typed contract)".into());
        }
    }
    if index.entries.is_empty() {
        contracts.push("evidence.entries (empty index)".into());
    }
    let mut paths = BTreeSet::new();
    for entry in &index.entries {
        if entry.schema != ENTRY_SCHEMA {
            contracts.push(format!("evidence.entry.{}.schema", entry.path));
        }
        let valid_role = authority.as_ref().map_or_else(
            || valid_role_mime(&entry.role, &entry.mime),
            |authority| {
                authority
                    .evidence_roles
                    .iter()
                    .any(|role| role.role == entry.role && role.media_type == entry.mime)
            },
        );
        if !valid_role {
            contracts.push(format!("evidence.entry.{}.role_mime_contract", entry.path));
        }
        if authority.is_some() && !gate_ids.contains(&entry.producing_gate.as_str()) {
            contracts.push(format!(
                "evidence.entry.{}.producing_gate (unknown gate '{}')",
                entry.path, entry.producing_gate
            ));
        }
        if entry.producing_command.is_empty()
            || entry.producing_command.iter().any(String::is_empty)
        {
            contracts.push(format!("evidence.entry.{}.producing_command", entry.path));
        }
        if !validate_relative_path(&entry.path) || !paths.insert(entry.path.clone()) {
            contracts.push(format!("evidence.entry.{}.path", entry.path));
        }
        if !is_hex(&entry.sha256, 64) {
            contracts.push(format!("evidence.entry.{}.sha256", entry.path));
        }
        let absolute = root.join(&entry.path);
        match read_bundle_file(root, &entry.path) {
            Ok(bytes) => {
                if validate_bytes(entry, &bytes).is_err() {
                    if bytes.len() as u64 != entry.byte_length {
                        contracts.push(format!(
                            "evidence.entry.{}.byte_length (expected {}, got {})",
                            entry.path,
                            entry.byte_length,
                            bytes.len()
                        ));
                    }
                    if hex_sha256(&bytes) != entry.sha256 {
                        contracts.push(format!("evidence.entry.{}.sha256_mismatch", entry.path));
                    }
                }
            }
            Err(error) => contracts.push(format!(
                "evidence.entry.{}.missing ({}: {error})",
                entry.path,
                absolute.display()
            )),
        }
    }
    Ok(contracts)
}

#[derive(Debug, Clone)]
pub(crate) struct RegularFileSnapshot {
    pub relative_path: String,
    pub bytes: Vec<u8>,
}

#[derive(Clone)]
struct ActiveSnapshot {
    root: PathBuf,
    files: std::sync::Arc<BTreeMap<String, Vec<u8>>>,
    rejected_paths: std::sync::Arc<Vec<String>>,
}

thread_local! {
    static ACTIVE_SNAPSHOTS: std::cell::RefCell<Vec<ActiveSnapshot>> = const {
        std::cell::RefCell::new(Vec::new())
    };
}

pub(crate) struct EvidenceSnapshot {
    active: ActiveSnapshot,
}

impl EvidenceSnapshot {
    pub(crate) fn open(root: &Path) -> std::io::Result<Self> {
        let snapshot = Self::open_validation(root)?;
        if snapshot.active.rejected_paths.is_empty() {
            Ok(snapshot)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "snapshot contains rejected path '{}'",
                    snapshot.active.rejected_paths[0]
                ),
            ))
        }
    }

    fn open_validation(root: &Path) -> std::io::Result<Self> {
        let root = lexical_absolute(root)?;
        let opened = OpenedRoot::open(&root)?;
        let limits = SnapshotLimits::from_checked_in_authority()?;
        let (files, rejected_paths) = opened.snapshot_files(limits)?;
        Ok(Self {
            active: ActiveSnapshot {
                root,
                files: std::sync::Arc::new(files),
                rejected_paths: std::sync::Arc::new(rejected_paths),
            },
        })
    }

    pub(crate) fn read(&self, relative: &str) -> std::io::Result<&[u8]> {
        validate_snapshot_relative(relative)?;
        self.active
            .files
            .get(relative)
            .map(Vec::as_slice)
            .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))
    }

    pub(crate) fn files(&self) -> Vec<RegularFileSnapshot> {
        self.active
            .files
            .iter()
            .map(|(relative_path, bytes)| RegularFileSnapshot {
                relative_path: relative_path.clone(),
                bytes: bytes.clone(),
            })
            .collect()
    }

    fn scoped<T>(&self, action: impl FnOnce() -> T) -> T {
        ACTIVE_SNAPSHOTS.with(|active| active.borrow_mut().push(self.active.clone()));
        let _guard = ActiveSnapshotGuard;
        action()
    }
}

struct ActiveSnapshotGuard;

impl Drop for ActiveSnapshotGuard {
    fn drop(&mut self) {
        ACTIVE_SNAPSHOTS.with(|active| {
            active.borrow_mut().pop();
        });
    }
}

fn lexical_absolute(path: &Path) -> std::io::Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            std::path::Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            std::path::Component::RootDir => normalized.push(Path::new("/")),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if !normalized.pop() {
                    return Err(invalid_path("path escapes its lexical root"));
                }
            }
            std::path::Component::Normal(component) => normalized.push(component),
        }
    }
    Ok(normalized)
}

fn invalid_path(detail: &str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidInput, detail)
}

fn validate_snapshot_relative(relative: &str) -> std::io::Result<()> {
    if validate_relative_path(relative) {
        Ok(())
    } else {
        Err(invalid_path(
            "evidence path is not normalized relative UTF-8",
        ))
    }
}

enum SnapshotRead {
    Inactive,
    Missing,
    Bytes(Vec<u8>),
}

fn active_file(root: &Path, relative: &str) -> std::io::Result<SnapshotRead> {
    validate_snapshot_relative(relative)?;
    let root = lexical_absolute(root)?;
    ACTIVE_SNAPSHOTS.with(|active| {
        for snapshot in active.borrow().iter().rev() {
            let Ok(prefix) = root.strip_prefix(&snapshot.root) else {
                continue;
            };
            let combined = prefix.join(relative);
            let Some(combined) = combined.to_str() else {
                return Err(invalid_path("evidence path is not UTF-8"));
            };
            let combined = combined.trim_start_matches('/');
            return Ok(snapshot
                .files
                .get(combined)
                .cloned()
                .map_or(SnapshotRead::Missing, SnapshotRead::Bytes));
        }
        Ok(SnapshotRead::Inactive)
    })
}

fn active_inventory(root: &Path) -> std::io::Result<Option<Vec<RegularFileSnapshot>>> {
    let root = lexical_absolute(root)?;
    ACTIVE_SNAPSHOTS.with(|active| {
        for snapshot in active.borrow().iter().rev() {
            let Ok(prefix) = root.strip_prefix(&snapshot.root) else {
                continue;
            };
            let prefix = prefix
                .to_str()
                .ok_or_else(|| invalid_path("root is not UTF-8"))?;
            let prefix = prefix.trim_matches('/');
            let prefix_slash = (!prefix.is_empty()).then(|| format!("{prefix}/"));
            let files = snapshot
                .files
                .iter()
                .filter_map(|(path, bytes)| {
                    let relative = match &prefix_slash {
                        Some(prefix) => path.strip_prefix(prefix)?,
                        None => path.as_str(),
                    };
                    Some(RegularFileSnapshot {
                        relative_path: relative.to_string(),
                        bytes: bytes.clone(),
                    })
                })
                .collect();
            return Ok(Some(files));
        }
        Ok(None)
    })
}

fn active_rejected_paths(root: &Path) -> std::io::Result<Vec<String>> {
    let root = lexical_absolute(root)?;
    ACTIVE_SNAPSHOTS.with(|active| {
        for snapshot in active.borrow().iter().rev() {
            let Ok(prefix) = root.strip_prefix(&snapshot.root) else {
                continue;
            };
            let prefix = prefix
                .to_str()
                .ok_or_else(|| invalid_path("root is not UTF-8"))?;
            let prefix = prefix.trim_matches('/');
            let prefix = (!prefix.is_empty()).then(|| format!("{prefix}/"));
            return Ok(snapshot
                .rejected_paths
                .iter()
                .filter_map(|path| match &prefix {
                    Some(prefix) => path.strip_prefix(prefix).map(str::to_string),
                    None => Some(path.clone()),
                })
                .collect());
        }
        Ok(Vec::new())
    })
}
fn with_snapshot<T>(root: &Path, action: impl FnOnce() -> T) -> std::io::Result<T> {
    if active_inventory(root)?.is_some() {
        return Ok(action());
    }
    let snapshot = EvidenceSnapshot::open_validation(root)?;
    Ok(snapshot.scoped(action))
}

fn read_bundle_file(root: &Path, relative: &str) -> std::io::Result<Vec<u8>> {
    match active_file(root, relative)? {
        SnapshotRead::Bytes(bytes) => Ok(bytes),
        SnapshotRead::Missing => Err(std::io::Error::from(std::io::ErrorKind::NotFound)),
        SnapshotRead::Inactive => OpenedRoot::open(&lexical_absolute(root)?)?.read_file(relative),
    }
}

pub(crate) fn read_regular_relative(root: &Path, relative: &str) -> std::io::Result<Vec<u8>> {
    read_bundle_file(root, relative)
}

pub(crate) fn regular_file_inventory(root: &Path) -> std::io::Result<Vec<RegularFileSnapshot>> {
    if let Some(files) = active_inventory(root)? {
        return Ok(files);
    }
    EvidenceSnapshot::open(root).map(|snapshot| snapshot.files())
}

pub(crate) struct EvidencePublisher {
    root: OpenedRoot,
}

impl EvidencePublisher {
    pub(crate) fn open(root: &Path) -> std::io::Result<Self> {
        Ok(Self {
            root: OpenedRoot::open(&lexical_absolute(root)?)?,
        })
    }

    pub(crate) fn replace(&self, relative: &str, bytes: &[u8]) -> std::io::Result<()> {
        self.root.publish(relative, bytes, true)
    }

    pub(crate) fn create(&self, relative: &str, bytes: &[u8]) -> std::io::Result<()> {
        self.root.publish(relative, bytes, false)
    }

    pub(crate) fn remove(&self, relative: &str) -> std::io::Result<()> {
        self.root.remove(relative)
    }
}

#[cfg(test)]
type AdversarialHook = Option<Box<dyn FnMut(&str)>>;

#[cfg(test)]
thread_local! {
    static ADVERSARIAL_HOOK: std::cell::RefCell<AdversarialHook> =
        std::cell::RefCell::new(None);
}

#[cfg(test)]
fn adversarial_hook(event: &str) {
    ADVERSARIAL_HOOK.with(|slot| {
        let hook = slot.borrow_mut().take();
        if let Some(mut hook) = hook {
            hook(event);
            *slot.borrow_mut() = Some(hook);
        }
    });
}

#[cfg(not(test))]
fn adversarial_hook(_event: &str) {}

type FileSnapshot = (BTreeMap<String, Vec<u8>>, Vec<String>);

#[derive(Clone, Copy)]
struct SnapshotLimits {
    member_count: u64,
    member_bytes: u64,
    aggregate_bytes: u64,
    path_bytes: u64,
    aggregate_path_bytes: u64,
}

impl SnapshotLimits {
    fn from_checked_in_authority() -> std::io::Result<Self> {
        let authority: Authority = serde_json::from_slice(include_bytes!("../../../ci/gates.json"))
            .map_err(|error| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("cannot parse snapshot authority: {error}"),
                )
            })?;
        validate_authority(&authority).map_err(|error| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("invalid snapshot authority: {error}"),
            )
        })?;
        Ok(Self {
            member_count: u64::from(authority.actions.evidence_snapshot_member_count_limit),
            member_bytes: authority.actions.evidence_snapshot_member_limit_bytes,
            aggregate_bytes: authority.actions.evidence_snapshot_aggregate_limit_bytes,
            path_bytes: u64::from(authority.actions.evidence_snapshot_path_limit_bytes),
            aggregate_path_bytes: authority
                .actions
                .evidence_snapshot_aggregate_path_limit_bytes,
        })
    }
}

struct SnapshotBudget {
    limits: SnapshotLimits,
    member_count: u64,
    aggregate_bytes: u64,
    aggregate_path_bytes: u64,
}

impl SnapshotBudget {
    fn new(limits: SnapshotLimits) -> Self {
        Self {
            limits,
            member_count: 0,
            aggregate_bytes: 0,
            aggregate_path_bytes: 0,
        }
    }

    fn admit_path(&mut self, relative: &str) -> std::io::Result<()> {
        let path_bytes = relative.len() as u64;
        if path_bytes > self.limits.path_bytes {
            return Err(invalid_path(
                "evidence snapshot path exceeds authority limit",
            ));
        }
        self.aggregate_path_bytes = self
            .aggregate_path_bytes
            .checked_add(path_bytes)
            .ok_or_else(|| invalid_path("evidence snapshot path length overflowed"))?;
        if self.aggregate_path_bytes > self.limits.aggregate_path_bytes {
            return Err(invalid_path(
                "evidence snapshot aggregate path bytes exceed authority limit",
            ));
        }
        Ok(())
    }

    fn admit_file(&mut self, byte_length: u64) -> std::io::Result<()> {
        if byte_length > self.limits.member_bytes {
            return Err(invalid_path(
                "evidence snapshot member exceeds authority byte limit",
            ));
        }
        self.member_count = self
            .member_count
            .checked_add(1)
            .ok_or_else(|| invalid_path("evidence snapshot member count overflowed"))?;
        if self.member_count > self.limits.member_count {
            return Err(invalid_path(
                "evidence snapshot member count exceeds authority limit",
            ));
        }
        self.aggregate_bytes = self
            .aggregate_bytes
            .checked_add(byte_length)
            .ok_or_else(|| invalid_path("evidence snapshot aggregate bytes overflowed"))?;
        if self.aggregate_bytes > self.limits.aggregate_bytes {
            return Err(invalid_path(
                "evidence snapshot aggregate bytes exceed authority limit",
            ));
        }
        Ok(())
    }
}

#[cfg(unix)]
struct OpenedRoot {
    directory: std::os::fd::OwnedFd,
}

#[cfg(unix)]
impl OpenedRoot {
    fn open(root: &Path) -> std::io::Result<Self> {
        use std::ffi::CString;
        use std::os::fd::{FromRawFd as _, OwnedFd};
        use std::os::unix::ffi::OsStrExt as _;

        let name = CString::new(root.as_os_str().as_bytes())
            .map_err(|_| invalid_path("evidence root contains NUL"))?;
        let fd = unsafe {
            libc::open(
                name.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            )
        };
        if fd < 0 {
            return Err(std::io::Error::last_os_error());
        }
        let directory = unsafe { OwnedFd::from_raw_fd(fd) };
        adversarial_hook("root-opened");
        Ok(Self { directory })
    }

    fn read_file(&self, relative: &str) -> std::io::Result<Vec<u8>> {
        let (parent, name) = self.open_parent(relative, false)?;
        let file = openat_regular(&parent, &name)?;
        adversarial_hook(&format!("snapshot-entry:{relative}"));
        read_owned_file_bounded(
            file,
            SnapshotLimits::from_checked_in_authority()?.member_bytes,
        )
    }

    fn snapshot_files(&self, limits: SnapshotLimits) -> std::io::Result<FileSnapshot> {
        let mut files = BTreeMap::new();
        let mut rejected_paths = Vec::new();
        let mut budget = SnapshotBudget::new(limits);
        walk_opened_directory(
            &self.directory,
            "",
            &mut files,
            &mut rejected_paths,
            &mut budget,
        )?;
        Ok((files, rejected_paths))
    }

    fn publish(&self, relative: &str, bytes: &[u8], replace: bool) -> std::io::Result<()> {
        use std::os::fd::AsRawFd as _;

        let (parent, name) = self.open_parent(relative, true)?;
        adversarial_hook(&format!("publish-parent:{relative}"));
        let temporary = temporary_name();
        let mut file = createat_new(&parent, &temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        adversarial_hook(&format!("publish-before-commit:{relative}"));
        let result = unsafe {
            if replace {
                libc::renameat(
                    parent.as_raw_fd(),
                    temporary.as_ptr(),
                    parent.as_raw_fd(),
                    name.as_ptr(),
                )
            } else {
                libc::linkat(
                    parent.as_raw_fd(),
                    temporary.as_ptr(),
                    parent.as_raw_fd(),
                    name.as_ptr(),
                    0,
                )
            }
        };
        if result != 0 {
            let error = std::io::Error::last_os_error();
            unlinkat_file(&parent, &temporary);
            return Err(error);
        }
        if !replace && unsafe { libc::unlinkat(parent.as_raw_fd(), temporary.as_ptr(), 0) } != 0 {
            let error = std::io::Error::last_os_error();
            unlinkat_file(&parent, &name);
            return Err(error);
        }
        if unsafe { libc::fsync(parent.as_raw_fd()) } != 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }

    fn remove(&self, relative: &str) -> std::io::Result<()> {
        use std::os::fd::AsRawFd as _;

        let (parent, name) = self.open_parent(relative, false)?;
        if unsafe { libc::unlinkat(parent.as_raw_fd(), name.as_ptr(), 0) } != 0 {
            let error = std::io::Error::last_os_error();
            if error.kind() != std::io::ErrorKind::NotFound {
                return Err(error);
            }
        }
        Ok(())
    }

    fn open_parent(
        &self,
        relative: &str,
        create: bool,
    ) -> std::io::Result<(std::os::fd::OwnedFd, std::ffi::CString)> {
        use std::os::fd::{AsRawFd as _, FromRawFd as _};
        use std::os::unix::ffi::OsStrExt as _;

        validate_snapshot_relative(relative)?;
        let components: Vec<_> = Path::new(relative).components().collect();
        let mut directory = duplicate_fd(&self.directory)?;
        for component in &components[..components.len() - 1] {
            let std::path::Component::Normal(name) = component else {
                return Err(invalid_path("non-normal evidence component"));
            };
            let name = std::ffi::CString::new(name.as_bytes())
                .map_err(|_| invalid_path("evidence component contains NUL"))?;
            let mut fd = openat_directory(directory.as_raw_fd(), &name);
            if fd < 0
                && create
                && std::io::Error::last_os_error().kind() == std::io::ErrorKind::NotFound
            {
                if unsafe { libc::mkdirat(directory.as_raw_fd(), name.as_ptr(), 0o755) } != 0
                    && std::io::Error::last_os_error().kind() != std::io::ErrorKind::AlreadyExists
                {
                    return Err(std::io::Error::last_os_error());
                }
                fd = openat_directory(directory.as_raw_fd(), &name);
            }
            if fd < 0 {
                return Err(std::io::Error::last_os_error());
            }
            directory = unsafe { std::os::fd::OwnedFd::from_raw_fd(fd) };
        }
        let last = components
            .last()
            .ok_or_else(|| invalid_path("empty evidence path"))?;
        let std::path::Component::Normal(name) = last else {
            return Err(invalid_path("non-normal evidence filename"));
        };
        let name = std::ffi::CString::new(name.as_bytes())
            .map_err(|_| invalid_path("evidence filename contains NUL"))?;
        Ok((directory, name))
    }
}

#[cfg(unix)]
fn duplicate_fd(fd: &std::os::fd::OwnedFd) -> std::io::Result<std::os::fd::OwnedFd> {
    use std::os::fd::{AsRawFd as _, FromRawFd as _};

    let duplicate = unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 0) };
    if duplicate < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(unsafe { std::os::fd::OwnedFd::from_raw_fd(duplicate) })
}

#[cfg(unix)]
fn openat_directory(parent: libc::c_int, name: &std::ffi::CStr) -> libc::c_int {
    unsafe {
        libc::openat(
            parent,
            name.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
        )
    }
}

#[cfg(unix)]
fn openat_regular(
    parent: &std::os::fd::OwnedFd,
    name: &std::ffi::CStr,
) -> std::io::Result<std::os::fd::OwnedFd> {
    use std::os::fd::{AsRawFd as _, FromRawFd as _};

    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
        )
    };
    if fd < 0 {
        return Err(std::io::Error::last_os_error());
    }
    let fd = unsafe { std::os::fd::OwnedFd::from_raw_fd(fd) };
    require_type(&fd, libc::S_IFREG)?;
    Ok(fd)
}

#[cfg(unix)]
fn require_type(fd: &std::os::fd::OwnedFd, expected: libc::mode_t) -> std::io::Result<()> {
    use std::os::fd::AsRawFd as _;

    let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
    if unsafe { libc::fstat(fd.as_raw_fd(), stat.as_mut_ptr()) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    if unsafe { stat.assume_init() }.st_mode & libc::S_IFMT != expected {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "evidence path contains an unexpected file type",
        ));
    }
    Ok(())
}

#[cfg(unix)]
fn read_owned_file_bounded(fd: std::os::fd::OwnedFd, byte_limit: u64) -> std::io::Result<Vec<u8>> {
    use std::os::fd::AsRawFd as _;

    let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
    if unsafe { libc::fstat(fd.as_raw_fd(), stat.as_mut_ptr()) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    let stat = unsafe { stat.assume_init() };
    if stat.st_size < 0 || stat.st_size as u64 > byte_limit {
        return Err(invalid_path(
            "evidence snapshot member exceeds authority byte limit",
        ));
    }
    let mut file = fs::File::from(fd);
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        let next = (bytes.len() as u64)
            .checked_add(read as u64)
            .ok_or_else(|| invalid_path("evidence snapshot member length overflowed"))?;
        if next > byte_limit {
            return Err(invalid_path(
                "evidence snapshot member exceeds authority byte limit",
            ));
        }
        bytes.extend_from_slice(&buffer[..read]);
    }
    Ok(bytes)
}

#[cfg(unix)]
fn directory_entry_names(fd: &std::os::fd::OwnedFd) -> std::io::Result<Vec<String>> {
    use std::ffi::CStr;
    use std::os::fd::AsRawFd as _;

    struct Directory(*mut libc::DIR);
    impl Drop for Directory {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe { libc::closedir(self.0) };
            }
        }
    }

    let dot =
        std::ffi::CString::new(".").map_err(|_| invalid_path("dot directory name contains NUL"))?;
    let duplicate = openat_directory(fd.as_raw_fd(), &dot);
    if duplicate < 0 {
        return Err(std::io::Error::last_os_error());
    }
    let mut directory = Directory(unsafe { libc::fdopendir(duplicate) });
    if directory.0.is_null() {
        unsafe { libc::close(duplicate) };
        return Err(std::io::Error::last_os_error());
    }
    let mut names = Vec::new();
    loop {
        set_errno(0);
        let entry = unsafe { libc::readdir(directory.0) };
        if entry.is_null() {
            let errno = get_errno();
            if errno != 0 {
                return Err(std::io::Error::from_raw_os_error(errno));
            }
            break;
        }
        let bytes = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
        if bytes != b"." && bytes != b".." {
            names.push(
                String::from_utf8(bytes.to_vec())
                    .map_err(|_| invalid_path("directory entry is not UTF-8"))?,
            );
        }
    }
    if unsafe { libc::closedir(directory.0) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    directory.0 = std::ptr::null_mut();
    names.sort();
    Ok(names)
}

#[cfg(all(unix, target_os = "macos"))]
fn errno_pointer() -> *mut libc::c_int {
    unsafe { libc::__error() }
}

#[cfg(all(unix, not(target_os = "macos")))]
fn errno_pointer() -> *mut libc::c_int {
    unsafe { libc::__errno_location() }
}

#[cfg(unix)]
fn set_errno(value: libc::c_int) {
    unsafe { *errno_pointer() = value };
}

#[cfg(unix)]
fn get_errno() -> libc::c_int {
    unsafe { *errno_pointer() }
}

#[cfg(unix)]
fn walk_opened_directory(
    directory: &std::os::fd::OwnedFd,
    prefix: &str,
    files: &mut BTreeMap<String, Vec<u8>>,
    rejected_paths: &mut Vec<String>,
    budget: &mut SnapshotBudget,
) -> std::io::Result<()> {
    use std::ffi::CString;
    use std::os::fd::{AsRawFd as _, FromRawFd as _};

    for name in directory_entry_names(directory)? {
        let relative = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        budget.admit_path(&relative)?;
        let c_name =
            CString::new(name.as_bytes()).map_err(|_| invalid_path("entry contains NUL"))?;
        let fd = unsafe {
            libc::openat(
                directory.as_raw_fd(),
                c_name.as_ptr(),
                libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
            )
        };
        if fd < 0 {
            let error = std::io::Error::last_os_error();
            if matches!(
                error.raw_os_error(),
                Some(libc::ELOOP) | Some(libc::EOPNOTSUPP) | Some(libc::ENXIO)
            ) {
                rejected_paths.push(relative);
                continue;
            }
            return Err(error);
        }
        let child = unsafe { std::os::fd::OwnedFd::from_raw_fd(fd) };
        let kind = file_type(&child)?;
        if kind == libc::S_IFDIR {
            adversarial_hook(&format!("snapshot-directory:{relative}"));
            walk_opened_directory(&child, &relative, files, rejected_paths, budget)?;
        } else if kind == libc::S_IFREG {
            adversarial_hook(&format!("snapshot-entry:{relative}"));
            let bytes = read_owned_file_bounded(child, budget.limits.member_bytes)?;
            budget.admit_file(bytes.len() as u64)?;
            files.insert(relative, bytes);
        } else {
            rejected_paths.push(relative);
        }
    }
    Ok(())
}

#[cfg(unix)]
fn file_type(fd: &std::os::fd::OwnedFd) -> std::io::Result<libc::mode_t> {
    use std::os::fd::AsRawFd as _;

    let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
    if unsafe { libc::fstat(fd.as_raw_fd(), stat.as_mut_ptr()) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(unsafe { stat.assume_init() }.st_mode & libc::S_IFMT)
}

#[cfg(unix)]
fn temporary_name() -> std::ffi::CString {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    use std::sync::atomic::Ordering;

    let ordinal = NEXT.fetch_add(1, Ordering::Relaxed);
    std::ffi::CString::new(format!(
        ".uqm-evidence-{}-{ordinal}.tmp",
        std::process::id()
    ))
    .expect("temporary name has no NUL")
}

#[cfg(unix)]
fn createat_new(parent: &std::os::fd::OwnedFd, name: &std::ffi::CStr) -> std::io::Result<fs::File> {
    use std::os::fd::{AsRawFd as _, FromRawFd as _};
    use std::os::unix::fs::PermissionsExt as _;

    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            0o600,
        )
    };
    if fd < 0 {
        return Err(std::io::Error::last_os_error());
    }
    let file = unsafe { fs::File::from_raw_fd(fd) };
    file.set_permissions(fs::Permissions::from_mode(0o640))?;
    Ok(file)
}

#[cfg(unix)]
fn unlinkat_file(parent: &std::os::fd::OwnedFd, name: &std::ffi::CStr) {
    use std::os::fd::AsRawFd as _;

    unsafe {
        libc::unlinkat(parent.as_raw_fd(), name.as_ptr(), 0);
    }
}

#[cfg(not(unix))]
struct OpenedRoot {
    root: PathBuf,
}

#[cfg(not(unix))]
impl OpenedRoot {
    fn open(root: &Path) -> std::io::Result<Self> {
        let metadata = fs::symlink_metadata(root)?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(invalid_path("evidence root is not a real directory"));
        }
        Ok(Self {
            root: root.to_path_buf(),
        })
    }

    fn read_file(&self, relative: &str) -> std::io::Result<Vec<u8>> {
        validate_snapshot_relative(relative)?;
        let path = checked_portable_path(&self.root, relative, false)?;
        let limit = SnapshotLimits::from_checked_in_authority()?.member_bytes;
        let file = fs::File::open(path)?;
        if file.metadata()?.len() > limit {
            return Err(invalid_path(
                "evidence snapshot member exceeds authority byte limit",
            ));
        }
        let mut bytes = Vec::new();
        file.take(limit + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > limit {
            return Err(invalid_path(
                "evidence snapshot member exceeds authority byte limit",
            ));
        }
        Ok(bytes)
    }

    fn snapshot_files(&self, limits: SnapshotLimits) -> std::io::Result<FileSnapshot> {
        fn walk(
            root: &Path,
            directory: &Path,
            files: &mut BTreeMap<String, Vec<u8>>,
            rejected_paths: &mut Vec<String>,
            budget: &mut SnapshotBudget,
        ) -> std::io::Result<()> {
            let mut entries = fs::read_dir(directory)?.collect::<Result<Vec<_>, _>>()?;
            entries.sort_by_key(std::fs::DirEntry::file_name);
            for entry in entries {
                let path = entry.path();
                let metadata = fs::symlink_metadata(&path)?;
                let relative = path
                    .strip_prefix(root)
                    .map_err(|_| invalid_path("invalid snapshot path"))?
                    .to_str()
                    .ok_or_else(|| invalid_path("snapshot path is not UTF-8"))?
                    .replace('\\', "/");
                budget.admit_path(&relative)?;
                if metadata.file_type().is_symlink() {
                    rejected_paths.push(relative);
                } else if metadata.is_dir() {
                    walk(root, &path, files, rejected_paths, budget)?;
                } else if metadata.is_file() {
                    if metadata.len() > budget.limits.member_bytes {
                        return Err(invalid_path(
                            "evidence snapshot member exceeds authority byte limit",
                        ));
                    }
                    let mut file = fs::File::open(path)?;
                    let mut bytes = Vec::new();
                    file.take(budget.limits.member_bytes + 1)
                        .read_to_end(&mut bytes)?;
                    budget.admit_file(bytes.len() as u64)?;
                    files.insert(relative, bytes);
                } else {
                    rejected_paths.push(relative);
                }
            }
            Ok(())
        }

        let mut files = BTreeMap::new();
        let mut rejected_paths = Vec::new();
        let mut budget = SnapshotBudget::new(limits);
        walk(
            &self.root,
            &self.root,
            &mut files,
            &mut rejected_paths,
            &mut budget,
        )?;
        Ok((files, rejected_paths))
    }

    fn publish(&self, relative: &str, bytes: &[u8], replace: bool) -> std::io::Result<()> {
        validate_snapshot_relative(relative)?;
        let destination = checked_portable_path(&self.root, relative, true)?;
        if !replace && destination.try_exists()? {
            return Err(std::io::Error::from(std::io::ErrorKind::AlreadyExists));
        }
        let parent = destination
            .parent()
            .ok_or_else(|| invalid_path("destination has no parent"))?;
        fs::create_dir_all(parent)?;
        let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
        temporary.write_all(bytes)?;
        temporary.as_file().sync_all()?;
        if replace {
            temporary
                .persist(&destination)
                .map_err(|error| error.error)?;
        } else {
            temporary
                .persist_noclobber(&destination)
                .map_err(|error| error.error)?;
        }
        Ok(())
    }

    fn remove(&self, relative: &str) -> std::io::Result<()> {
        let path = checked_portable_path(&self.root, relative, true)?;
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        }
    }
}

#[cfg(not(unix))]
fn checked_portable_path(
    root: &Path,
    relative: &str,
    allow_missing: bool,
) -> std::io::Result<PathBuf> {
    let mut path = root.to_path_buf();
    let components: Vec<_> = Path::new(relative).components().collect();
    for (position, component) in components.iter().enumerate() {
        let std::path::Component::Normal(component) = component else {
            return Err(invalid_path("non-normal evidence component"));
        };
        path.push(component);
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(invalid_path("evidence path contains a symlink"));
            }
            Ok(metadata) if position + 1 < components.len() && !metadata.is_dir() => {
                return Err(invalid_path("evidence parent is not a directory"));
            }
            Ok(metadata) if position + 1 == components.len() && !metadata.is_file() => {
                return Err(invalid_path("evidence destination is not a regular file"));
            }
            Ok(_) => {}
            Err(error) if allow_missing && error.kind() == std::io::ErrorKind::NotFound => break,
            Err(error) => return Err(error),
        }
    }
    Ok(path)
}

/// Validate a single signed byte payload against an entry (used for receipts).
fn retained_authority(root: &Path, index: &EvidenceIndex) -> Option<Authority> {
    let entry = index
        .entries
        .iter()
        .find(|entry| entry.role == "authority.snapshot")?;
    let bytes = read_bundle_file(root, &entry.path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn validate_authority_snapshot(root: &Path, index: &EvidenceIndex) -> Vec<String> {
    let snapshots: Vec<_> = index
        .entries
        .iter()
        .filter(|entry| entry.role == "authority.snapshot")
        .collect();
    if snapshots.len() != 1 {
        return vec![format!(
            "evidence.authority_snapshot.count (expected 1, got {})",
            snapshots.len()
        )];
    }
    let entry = snapshots[0];
    let bytes = match read_bundle_file(root, &entry.path) {
        Ok(bytes) => bytes,
        Err(error) => {
            return vec![format!(
                "evidence.authority_snapshot.read ({}: {error})",
                entry.path
            )];
        }
    };
    let authority: Authority = match serde_json::from_slice(&bytes) {
        Ok(authority) => authority,
        Err(error) => {
            return vec![format!("evidence.authority_snapshot.json ({error})")];
        }
    };
    let mut contracts = Vec::new();
    if let Err(error) = validate_authority(&authority) {
        contracts.push(format!("evidence.authority_snapshot.contract ({error})"));
    }
    let tuples: Vec<_> = authority
        .runner_mapping
        .iter()
        .map(|mapping| mapping.tuple.clone())
        .collect();
    let mut expected = index.supported_tuples.clone();
    let mut actual = tuples;
    expected.sort();
    actual.sort();
    if actual != expected {
        contracts.push("evidence.authority_snapshot.tuples".to_string());
    }
    contracts.extend(validate_gate_and_step_receipts(
        root, index, &authority, entry,
    ));
    contracts
}

fn validate_singleton_role(index: &EvidenceIndex, role: &str, contracts: &mut Vec<String>) {
    let count = index
        .entries
        .iter()
        .filter(|entry| entry.role == role)
        .count();
    if count != 1 {
        contracts.push(format!(
            "evidence.role.{role}.count (expected 1, got {count})"
        ));
    }
}

fn validate_receipt_identity(
    index: &EvidenceIndex,
    role: &str,
    path: &str,
    producing_gate: &str,
    producing_command: &[String],
    contracts: &mut Vec<String>,
) {
    if let Some(entry) = index.entries.iter().find(|entry| entry.role == role) {
        if entry.path != path
            || entry.producing_gate != producing_gate
            || entry.producing_command != producing_command
        {
            contracts.push(format!("evidence.{role}.identity"));
        }
    }
}

fn valid_executable_identity(identity: &serde_json::Value) -> bool {
    exact_json_fields(identity, &["path", "byte_length", "sha256", "mode"])
        && identity
            .get("path")
            .and_then(|value| value.as_str())
            .is_some_and(validate_absolute_path)
        && identity
            .get("byte_length")
            .and_then(|value| value.as_u64())
            .is_some_and(|length| length > 0)
        && identity
            .get("sha256")
            .and_then(|value| value.as_str())
            .is_some_and(|digest| is_hex(digest, 64))
        && identity
            .get("mode")
            .and_then(|value| value.as_u64())
            .is_some_and(|mode| mode <= 0o7777 && mode & 0o111 != 0)
}

fn valid_step_executable_identity(receipt: &serde_json::Value) -> bool {
    receipt
        .get("executable_identity")
        .is_some_and(valid_executable_identity)
        || (receipt
            .get("executable_identity")
            .is_some_and(serde_json::Value::is_null)
            && receipt
                .get("launch_error")
                .and_then(|value| value.as_str())
                .is_some_and(|error| !error.is_empty()))
}

fn valid_step_execution_provenance(
    receipt: &serde_json::Value,
    expected_command: &[String],
) -> bool {
    let Some(effective_command) = receipt
        .get("effective_command")
        .and_then(serde_json::Value::as_array)
        .and_then(|values| {
            values
                .iter()
                .map(|value| value.as_str().map(str::to_string))
                .collect::<Option<Vec<_>>>()
        })
    else {
        return false;
    };
    if let Some(hidden_command) = super::run::trusted_controller_command(expected_command) {
        return matches!(
            effective_command.as_slice(),
            [program, command]
                if Path::new(program).is_absolute() && command == hidden_command
        ) && receipt
            .get("staged_script_sha256")
            .is_some_and(serde_json::Value::is_null);
    }
    match super::run::trusted_control_plane_script(expected_command) {
        Some((script_name, script_bytes)) => {
            effective_command.len() == expected_command.len()
                && effective_command.first() == expected_command.first()
                && effective_command.get(2..) == expected_command.get(2..)
                && effective_command.get(1).map(Path::new).is_some_and(|path| {
                    path.is_absolute()
                        && path.file_name().and_then(|name| name.to_str()) == Some(script_name)
                })
                && receipt
                    .get("staged_script_sha256")
                    .and_then(serde_json::Value::as_str)
                    == Some(hex_sha256(script_bytes).as_str())
        }
        None => {
            effective_command == expected_command
                && receipt
                    .get("staged_script_sha256")
                    .is_some_and(serde_json::Value::is_null)
        }
    }
}

fn validate_tool_receipt(
    root: &Path,
    index: &EvidenceIndex,
    authority: &Authority,
    contracts: &mut Vec<String>,
) {
    let Some(entry) = index
        .entries
        .iter()
        .find(|entry| entry.role == "preflight.tools")
    else {
        return;
    };
    let report: Option<serde_json::Value> = read_bundle_file(root, &entry.path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok());
    let Some(report) = report else {
        contracts.push("evidence.preflight.tools.json".to_string());
        return;
    };
    if !exact_json_fields(&report, &["schema", "passed", "observations"]) {
        contracts.push("evidence.preflight.tools.fields".to_string());
    }
    let tool_failure = index.first_failed_contract.as_deref() == Some("tools.preflight");
    if report.get("schema").and_then(|value| value.as_str()) != Some("uqm-s4-tool-preflight-v2") {
        contracts.push("evidence.preflight.tools.schema".to_string());
    }
    let observations = report
        .get("observations")
        .and_then(|value| value.as_array());
    type ExpectedTool<'a> = (&'a str, &'a [String], Option<&'a str>, &'a [i32]);
    let mut expected_tools: Vec<ExpectedTool<'_>> = authority
        .tools
        .preflight
        .iter()
        .map(|probe| {
            (
                probe.name.as_str(),
                probe.version_command.as_slice(),
                probe.expected_output_prefix.as_deref(),
                probe.accepted_exit_codes.as_slice(),
            )
        })
        .collect();
    expected_tools.extend(authority.tools.entries().into_iter().map(|(name, tool)| {
        (
            name,
            tool.version_command.as_slice(),
            Some(tool.expected_output_prefix.as_str()),
            &[0][..],
        )
    }));
    if observations.is_none_or(|items| items.len() != expected_tools.len()) {
        contracts.push("evidence.preflight.tools.observation_count".to_string());
    }
    let mut observed_failure = false;
    for (name, command, expected_prefix, accepted_exit_codes) in expected_tools {
        let matching: Vec<_> = observations
            .into_iter()
            .flatten()
            .filter(|item| {
                item.get("name").and_then(|value| value.as_str()) == Some(name)
                    && item.get("command") == Some(&serde_json::json!(command))
                    && item.get("expected_output_prefix")
                        == Some(&serde_json::json!(expected_prefix))
            })
            .collect();
        if matching.len() != 1 {
            contracts.push(format!("evidence.preflight.tools.{name}.identity"));
            continue;
        }
        let item = matching[0];
        if !exact_json_fields(
            item,
            &[
                "name",
                "command",
                "expected_output_prefix",
                "executable_identity",
                "stdout",
                "stderr",
                "exit_code",
                "signal",
                "launch_error",
                "passed",
            ],
        ) {
            contracts.push(format!("evidence.preflight.tools.{name}.fields"));
        }
        let passed = item.get("passed").and_then(|value| value.as_bool());
        let exit_value = item.get("exit_code");
        let exit_code = exit_value.and_then(|value| value.as_i64());
        let executable_identity = item.get("executable_identity");
        let identity_valid = executable_identity.is_some_and(valid_executable_identity);
        let identity_shape_valid =
            identity_valid || executable_identity.is_some_and(|value| value.is_null());
        if !identity_shape_valid {
            contracts.push(format!(
                "evidence.preflight.tools.{name}.executable_identity"
            ));
        }
        let stdout = item.get("stdout").and_then(|value| value.as_str());
        let stderr = item.get("stderr").and_then(|value| value.as_str());
        let signal_value = item.get("signal");
        let signal = signal_value.and_then(|value| value.as_i64());
        let launch_error = item.get("launch_error");
        let observed = match (stdout, stderr) {
            (Some(stdout), Some(stderr)) if stdout.trim().is_empty() => Some(stderr.trim()),
            (Some(stdout), Some(_)) => Some(stdout.trim()),
            _ => None,
        };
        let observed_prefix = expected_prefix
            .is_none_or(|prefix| observed.is_some_and(|value| value.starts_with(prefix)));
        let valid_exit = exit_code.is_some_and(|code| (0..=255).contains(&code));
        let valid_signal = signal.is_some_and(|signal| signal > 0 && signal <= i64::from(i32::MAX));
        let spawned = launch_error.is_some_and(|value| value.is_null())
            && ((valid_exit && signal_value.is_some_and(|value| value.is_null()))
                || (exit_value.is_some_and(|value| value.is_null()) && valid_signal));
        let execution_error = launch_error
            .and_then(|value| value.as_str())
            .is_some_and(|error| !error.is_empty());
        let launch_failed = exit_value.is_some_and(|value| value.is_null())
            && signal_value.is_some_and(|value| value.is_null())
            && execution_error
            && stdout == Some("")
            && stderr == Some("");
        let supervised_execution_failed = launch_error
            .and_then(|value| value.as_str())
            .is_some_and(|error| error.starts_with("supervision: "))
            && ((valid_exit && signal_value.is_some_and(|value| value.is_null()))
                || (exit_value.is_some_and(|value| value.is_null()) && valid_signal));
        let semantic_pass = identity_valid
            && spawned
            && exit_code
                .and_then(|code| i32::try_from(code).ok())
                .is_some_and(|code| accepted_exit_codes.contains(&code))
            && observed_prefix;
        if passed != Some(semantic_pass)
            || observed.is_none()
            || (!spawned && !launch_failed && !supervised_execution_failed)
        {
            contracts.push(format!("evidence.preflight.tools.{name}.result"));
        }
        if passed == Some(false) {
            observed_failure = true;
        }
    }
    if report.get("passed").and_then(|value| value.as_bool()) != Some(!observed_failure) {
        contracts.push("evidence.preflight.tools.result".to_string());
    }
    if tool_failure && !observed_failure {
        contracts.push("evidence.preflight.tools.missing_failure".to_string());
    } else if index.first_failed_contract.is_none() && observed_failure {
        contracts.push("evidence.preflight.tools.unexpected_failure".to_string());
    }
}

fn validate_cache_receipt(
    root: &Path,
    index: &EvidenceIndex,
    authority: &Authority,
    contracts: &mut Vec<String>,
) {
    let Some(entry) = index
        .entries
        .iter()
        .find(|entry| entry.role == "cache.initial-state")
    else {
        return;
    };
    let Ok(bytes) = read_bundle_file(root, &entry.path) else {
        return;
    };
    let Ok(receipt) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        contracts.push("evidence.cache.initial_state.json".to_string());
        return;
    };
    if !exact_json_fields(
        &receipt,
        &[
            "schema",
            "mode",
            "ambient_cargo_home",
            "isolation_cargo_home",
            "execution_target",
            "registry_cache_present",
            "git_cache_present",
            "execution_target_absent",
            "rust_target_present",
            "sc2_obj_present",
            "restore_used",
            "save_used",
            "first_failed_contract",
            "passed",
        ],
    ) {
        contracts.push("evidence.cache.initial_state.fields".to_string());
    }
    if receipt.get("schema").and_then(|value| value.as_str())
        != Some("uqm-s4-cache-initial-state-v1")
        || receipt.get("mode").and_then(|value| value.as_str()) != Some(index.cache_mode.as_str())
    {
        contracts.push("evidence.cache.initial_state.identity".to_string());
    }
    if receipt
        .get("restore_used")
        .and_then(|value| value.as_bool())
        != Some(false)
        || receipt.get("save_used").and_then(|value| value.as_bool()) != Some(false)
    {
        contracts.push("evidence.cache.initial_state.cache_action".to_string());
    }
    let rust_target_present = receipt
        .get("rust_target_present")
        .and_then(|value| value.as_bool());
    let sc2_obj_present = receipt
        .get("sc2_obj_present")
        .and_then(|value| value.as_bool());
    let registry_present = receipt
        .get("registry_cache_present")
        .and_then(|value| value.as_bool());
    let git_present = receipt
        .get("git_cache_present")
        .and_then(|value| value.as_bool());
    let target_absent = receipt
        .get("execution_target_absent")
        .and_then(|value| value.as_bool());
    let expected_failure = if index.cache_mode == "isolated-empty" {
        if authority.cache.require_rust_target_absent && rust_target_present == Some(true) {
            Some("cache.rust_target")
        } else if authority.cache.require_sc2_obj_absent && sc2_obj_present == Some(true) {
            Some("cache.sc2_obj")
        } else if registry_present == Some(true) || git_present == Some(true) {
            Some("cache.cache_present")
        } else {
            None
        }
    } else {
        None
    };
    if [
        rust_target_present,
        sc2_obj_present,
        registry_present,
        git_present,
        target_absent,
    ]
    .contains(&None)
    {
        contracts.push("evidence.cache.initial_state.fields".to_string());
    }
    let ambient_home = receipt
        .get("ambient_cargo_home")
        .and_then(|value| value.as_str())
        .map(Path::new);
    let isolation_home = receipt
        .get("isolation_cargo_home")
        .and_then(|value| value.as_str())
        .map(Path::new);
    let execution_target = receipt
        .get("execution_target")
        .and_then(|value| value.as_str())
        .map(Path::new);
    let paths_valid = match (ambient_home, isolation_home, execution_target) {
        (Some(ambient), Some(isolation), Some(target)) if index.cache_mode == "isolated-empty" => {
            ambient.is_absolute()
                && isolation.is_absolute()
                && target.is_absolute()
                && isolation.starts_with(target)
                && isolation != ambient
                && target.ends_with("rust/target")
        }
        (Some(ambient), Some(isolation), Some(target)) => {
            ambient.is_absolute() && isolation == ambient && target.as_os_str().is_empty()
        }
        _ => false,
    };
    if !paths_valid
        || (index.cache_mode == "isolated-empty"
            && target_absent != rust_target_present.map(|present| !present))
    {
        contracts.push("evidence.cache.initial_state.paths".to_string());
    }
    if receipt
        .get("first_failed_contract")
        .and_then(|value| value.as_str())
        != expected_failure
        || receipt.get("passed").and_then(|value| value.as_bool())
            != Some(expected_failure.is_none())
    {
        contracts.push("evidence.cache.initial_state.result".to_string());
    }
    if index
        .first_failed_contract
        .as_deref()
        .is_some_and(|contract| contract.starts_with("cache."))
        && index.first_failed_contract.as_deref() != expected_failure
    {
        contracts.push("evidence.cache.initial_state.first_failure".to_string());
    }
}

fn validate_delta_receipt(
    root: &Path,
    index: &EvidenceIndex,
    authority: &Authority,
    contracts: &mut Vec<String>,
) {
    let Some(entry) = index
        .entries
        .iter()
        .find(|entry| entry.role == "ownership.zero-native-delta")
    else {
        return;
    };
    let Ok(bytes) = read_bundle_file(root, &entry.path) else {
        return;
    };
    let Ok(report) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        contracts.push("evidence.ownership.zero_native_delta.json".to_string());
        return;
    };
    let source_base_sha = index
        .entries
        .iter()
        .find(|entry| entry.role == "preflight.source")
        .and_then(|entry| read_bundle_file(root, &entry.path).ok())
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .and_then(|receipt| {
            receipt
                .get("base_sha")
                .and_then(|value| value.as_str())
                .map(str::to_string)
        });
    if !exact_json_fields(
        &report,
        &[
            "schema",
            "base_sha",
            "head_sha",
            "categories",
            "transitional_native_inputs",
            "passed",
        ],
    ) {
        contracts.push("evidence.ownership.zero_native_delta.fields".to_string());
    }
    let report_base_sha = report.get("base_sha").and_then(|value| value.as_str());
    if report.get("schema").and_then(|value| value.as_str()) != Some("uqm-s4-zero-native-delta-v1")
        || report.get("head_sha").and_then(|value| value.as_str())
            != Some(index.source_sha.as_str())
        || !report_base_sha.is_some_and(|value| is_hex(value, 40))
        || report_base_sha != source_base_sha.as_deref()
    {
        contracts.push("evidence.ownership.zero_native_delta.identity".to_string());
    }
    let expected = [
        (
            "tracked_sources",
            authority.zero_native_delta.tracked_sources,
        ),
        ("providers", authority.zero_native_delta.providers),
        ("objects", authority.zero_native_delta.objects),
        (
            "internal_symbols",
            authority.zero_native_delta.internal_symbols,
        ),
        ("bridges", authority.zero_native_delta.bridges),
        (
            "generated_bindings",
            authority.zero_native_delta.generated_bindings,
        ),
        (
            "transitional_flags",
            authority.zero_native_delta.transitional_flags,
        ),
    ];
    let categories = report.get("categories").and_then(|value| value.as_object());
    if categories.is_none_or(|values| values.len() != expected.len()) {
        contracts.push("evidence.ownership.zero_native_delta.category_count".to_string());
    }
    let mut passed = true;
    for (name, expected_delta) in expected {
        let category = categories.and_then(|values| values.get(name));
        if category
            .is_some_and(|value| !exact_json_fields(value, &["measured_delta", "changed_paths"]))
        {
            contracts.push(format!(
                "evidence.ownership.zero_native_delta.{name}.fields"
            ));
        }
        let measured = category
            .and_then(|value| value.get("measured_delta"))
            .and_then(|value| value.as_u64());
        let changed_paths = category
            .and_then(|value| value.get("changed_paths"))
            .and_then(|value| value.as_array());
        if measured.is_none() {
            contracts.push(format!(
                "evidence.ownership.zero_native_delta.{name}.measured"
            ));
        }
        match changed_paths {
            Some(paths) => {
                if Some(paths.len() as u64) != measured {
                    contracts.push(format!("evidence.ownership.zero_native_delta.{name}.count"));
                }
                if !paths
                    .iter()
                    .all(|path| path.as_str().is_some_and(validate_relative_path))
                {
                    contracts.push(format!("evidence.ownership.zero_native_delta.{name}.path"));
                }
            }
            None => contracts.push(format!("evidence.ownership.zero_native_delta.{name}.paths")),
        }
        if measured != Some(u64::from(expected_delta)) {
            passed = false;
        }
    }
    let input_counts = report.get("transitional_native_inputs");
    if input_counts.is_none_or(|value| {
        !exact_json_fields(
            value,
            &["base_count", "head_count", "maximum_count", "passed"],
        )
    }) {
        contracts
            .push("evidence.ownership.zero_native_delta.transitional_inputs.fields".to_string());
    }
    let base_count = input_counts
        .and_then(|value| value.get("base_count"))
        .and_then(|value| value.as_u64());
    let head_count = input_counts
        .and_then(|value| value.get("head_count"))
        .and_then(|value| value.as_u64());
    let maximum_count = input_counts
        .and_then(|value| value.get("maximum_count"))
        .and_then(|value| value.as_u64());
    let input_counts_passed = base_count
        .zip(head_count)
        .is_some_and(|(base, head)| head <= base)
        && head_count
            .zip(maximum_count)
            .is_some_and(|(head, maximum)| head <= maximum)
        && maximum_count
            == Some(u64::from(
                authority
                    .zero_native_delta
                    .maximum_transitional_native_inputs,
            ));
    if input_counts
        .and_then(|value| value.get("passed"))
        .and_then(|value| value.as_bool())
        != Some(input_counts_passed)
    {
        contracts
            .push("evidence.ownership.zero_native_delta.transitional_inputs.result".to_string());
    }
    passed &= input_counts_passed;
    let failure_correlated = passed
        || index.first_failed_contract.as_deref() == Some("ownership.zero_native_delta")
        || index
            .first_failed_contract
            .as_deref()
            .is_some_and(|contract| contract.starts_with("cache."));
    if report.get("passed").and_then(|value| value.as_bool()) != Some(passed) || !failure_correlated
    {
        contracts.push("evidence.ownership.zero_native_delta.result".to_string());
    }
}

fn is_source_preflight_failure(contract: &str) -> bool {
    matches!(
        contract,
        "source.detached_head"
            | "source.expected_sha"
            | "source.expected_tuple"
            | "source.clean"
            | "environment.canonical"
    )
}

fn is_preflight_failure(contract: &str) -> bool {
    is_source_preflight_failure(contract) || contract == "tools.preflight"
}

fn is_setup_failure(contract: &str) -> bool {
    is_preflight_failure(contract)
        || contract.starts_with("cache.")
        || contract == "ownership.zero_native_delta"
}
fn valid_detached_git_command(value: &serde_json::Value) -> bool {
    let Some(command) = value.as_array() else {
        return false;
    };
    if command.len() != 6
        || command[0].as_str() != Some("git")
        || command[1].as_str() != Some("-c")
        || command[3].as_str() != Some("symbolic-ref")
        || command[4].as_str() != Some("-q")
        || command[5].as_str() != Some("HEAD")
    {
        return false;
    }
    command[2]
        .as_str()
        .and_then(|argument| argument.strip_prefix("safe.directory="))
        .is_some_and(|path| Path::new(path).is_absolute())
}

fn valid_detached_receipt(value: &serde_json::Value, authority: &Authority) -> bool {
    exact_json_fields(
        value,
        &[
            "schema",
            "command",
            "exit_code",
            "signal",
            "launch_error",
            "success",
            "stdout",
            "stderr",
            "supervision",
        ],
    ) && value.get("schema").and_then(serde_json::Value::as_str) == Some("uqm-s4-detached-state-v1")
        && value.get("command").is_some_and(valid_detached_git_command)
        && value.get("success").and_then(serde_json::Value::as_bool) == Some(false)
        && value
            .get("launch_error")
            .is_some_and(serde_json::Value::is_null)
        && value.get("signal").is_some_and(serde_json::Value::is_null)
        && spawned_exit_code(value) == Some(1)
        && value.get("stdout").and_then(serde_json::Value::as_str) == Some("")
        && value
            .get("stderr")
            .and_then(serde_json::Value::as_str)
            .is_some()
        && strict_mutation_supervision(value, authority.supervision.builtin_timeout_seconds * 1_000)
}

fn valid_detached_replay(
    receipt: &serde_json::Value,
    index: &EvidenceIndex,
    authority: &Authority,
) -> bool {
    let detached = receipt.get("detached_state");
    if index.cache_mode != "isolated-empty" {
        return detached.is_some_and(serde_json::Value::is_null);
    }
    detached.is_some_and(|value| {
        valid_detached_receipt(value, authority)
            || (index.first_failed_contract.as_deref() == Some("source.detached_head")
                && value.get("exit_code").and_then(serde_json::Value::as_i64) == Some(0)
                && value.get("signal").is_some_and(serde_json::Value::is_null)
                && value
                    .get("launch_error")
                    .is_some_and(serde_json::Value::is_null)
                && strict_mutation_supervision(
                    value,
                    authority.supervision.builtin_timeout_seconds * 1_000,
                ))
    })
}

fn validate_source_receipt(
    root: &Path,
    index: &EvidenceIndex,
    authority: &Authority,
    contracts: &mut Vec<String>,
) {
    let Some(entry) = index
        .entries
        .iter()
        .find(|entry| entry.role == "preflight.source")
    else {
        return;
    };
    let receipt: Option<serde_json::Value> = read_bundle_file(root, &entry.path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok());
    let valid = receipt.as_ref().is_some_and(|receipt| {
        let exact_fields = exact_json_fields(
            receipt,
            &[
                "schema",
                "source_sha",
                "detached_state",
                "expected_sha",
                "base_sha",
                "tuple",
                "expected_tuple",
                "cache_mode",
                "clean",
                "canonical_environment",
                "passed",
                "first_failed_contract",
                "detail",
            ],
        );
        let common = exact_fields
            && receipt.get("schema").and_then(|value| value.as_str())
                == Some("uqm-s4-source-preflight-v2")
            && valid_detached_replay(receipt, index, authority)
            && receipt.get("source_sha").and_then(|value| value.as_str())
                == Some(index.source_sha.as_str())
            && receipt.get("tuple").and_then(|value| value.as_str()) == Some(index.tuple.as_str())
            && receipt.get("cache_mode").and_then(|value| value.as_str())
                == Some(index.cache_mode.as_str())
            && (index.cache_mode != "isolated-empty"
                || receipt
                    .get("base_sha")
                    .and_then(|value| value.as_str())
                    .is_some_and(|value| is_hex(value, 40)));
        let index_failure = index.first_failed_contract.as_deref();
        let receipt_failure = receipt
            .get("first_failed_contract")
            .and_then(|value| value.as_str());
        let correlated = !index_failure.is_some_and(is_source_preflight_failure)
            || receipt_failure == index_failure;
        let outcome = if let Some(first_failed) = receipt_failure {
            is_source_preflight_failure(first_failed)
                && receipt.get("passed").and_then(|value| value.as_bool()) == Some(false)
                && receipt
                    .get("detail")
                    .and_then(|value| value.as_str())
                    .is_some_and(|detail| !detail.is_empty())
                && match first_failed {
                    "source.detached_head" => receipt
                        .get("detached_state")
                        .is_some_and(|value| !valid_detached_receipt(value, authority)),
                    "source.expected_sha" => {
                        let expected = receipt.get("expected_sha").and_then(|value| value.as_str());
                        (index.cache_mode == "isolated-empty" && expected.is_none())
                            || expected.is_some_and(|value| {
                                !is_hex(value, 40) || value != index.source_sha
                            })
                    }
                    "source.expected_tuple" => {
                        let expected = receipt
                            .get("expected_tuple")
                            .and_then(|value| value.as_str());
                        (index.cache_mode == "isolated-empty" && expected.is_none())
                            || expected.is_some_and(|value| value != index.tuple)
                    }
                    "source.clean" => {
                        receipt.get("clean").and_then(|value| value.as_bool()) == Some(false)
                    }
                    "environment.canonical" => {
                        receipt
                            .get("canonical_environment")
                            .and_then(|value| value.as_bool())
                            == Some(false)
                    }
                    _ => false,
                }
        } else {
            receipt.get("passed").and_then(|value| value.as_bool()) == Some(true)
                && receipt.get("clean").and_then(|value| value.as_bool()) == Some(true)
                && receipt.get("detail").is_some_and(|value| value.is_null())
                && (index.cache_mode != "isolated-empty"
                    || (receipt.get("expected_sha").and_then(|value| value.as_str())
                        == Some(index.source_sha.as_str())
                        && receipt
                            .get("expected_tuple")
                            .and_then(|value| value.as_str())
                            == Some(index.tuple.as_str())
                        && receipt
                            .get("canonical_environment")
                            .and_then(|value| value.as_bool())
                            == Some(true)))
        };
        let explicit_null = receipt_failure.is_some()
            || receipt
                .get("first_failed_contract")
                .is_some_and(|value| value.is_null());
        common && correlated && explicit_null && outcome
    });
    if !valid {
        contracts.push("evidence.preflight.source.content".to_string());
    }
}

fn validate_gate_result_content(
    root: &Path,
    index: &EvidenceIndex,
    expected: &super::authority::Gate,
    entry: &EvidenceEntry,
    command: &[String],
    contracts: &mut Vec<String>,
) -> Option<bool> {
    if entry.producing_gate != expected.id
        || entry.producing_command != command
        || entry.path != format!("{}/gate.result.json", expected.id)
    {
        contracts.push(format!("evidence.gate_result.{}.identity", expected.id));
    }
    let receipt: serde_json::Value = match read_bundle_file(root, &entry.path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
    {
        Some(receipt) => receipt,
        None => {
            contracts.push(format!("evidence.gate_result.{}.json", expected.id));
            return None;
        }
    };
    if !exact_json_fields(
        &receipt,
        &[
            "schema",
            "gate",
            "owner",
            "kind",
            "passed",
            "first_failed_contract",
            "detail",
            "controller_command",
        ],
    ) || receipt.get("schema").and_then(|value| value.as_str()) != Some("uqm-s4-gate-result-v1")
        || receipt.get("gate").and_then(|value| value.as_str()) != Some(expected.id.as_str())
        || receipt.get("owner").and_then(|value| value.as_str()) != Some(expected.owner.as_str())
        || receipt.get("kind") != Some(&serde_json::json!(expected.kind))
        || receipt.get("controller_command") != Some(&serde_json::json!(command))
    {
        contracts.push(format!("evidence.gate_result.{}.content", expected.id));
    }
    let passed = receipt
        .get("passed")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    let receipt_failure = receipt
        .get("first_failed_contract")
        .and_then(|value| value.as_str());
    let receipt_detail = receipt.get("detail");
    if (passed
        && (receipt_failure.is_some() || !receipt_detail.is_some_and(serde_json::Value::is_null)))
        || (!passed
            && (receipt_failure.is_none()
                || !receipt_detail
                    .and_then(|value| value.as_str())
                    .is_some_and(|detail| !detail.is_empty())))
    {
        contracts.push(format!("evidence.gate_result.{}.result", expected.id));
    }
    if index.first_failed_contract.is_none() && !passed {
        contracts.push(format!(
            "evidence.gate_result.{}.unexpected_failure",
            expected.id
        ));
    }
    Some(passed)
}

fn validate_successful_process_gate(
    root: &Path,
    index: &EvidenceIndex,
    authority: &Authority,
    expected: &super::authority::Gate,
    contracts: &mut Vec<String>,
) {
    let actual_step_entries = index
        .entries
        .iter()
        .filter(|entry| {
            entry.producing_gate == expected.id
                && matches!(
                    entry.role.as_str(),
                    "step.stdout" | "step.stderr" | "step.result"
                )
        })
        .count();
    let expected_step_entries = expected.steps.len() * 3;
    if actual_step_entries != expected_step_entries {
        contracts.push(format!(
            "evidence.gate.{}.step_entry_count (expected {expected_step_entries}, got {actual_step_entries})",
            expected.id
        ));
    }
    for step in &expected.steps {
        let expected_command = &step.command;
        for (suffix, role) in [
            ("stdout.log", "step.stdout"),
            ("stderr.log", "step.stderr"),
            ("result.json", "step.result"),
        ] {
            let ending = format!("{}/{}.{}", expected.id, step.id, suffix);
            let matching: Vec<_> = index
                .entries
                .iter()
                .filter(|candidate| {
                    candidate.role == role
                        && candidate.producing_gate == expected.id
                        && candidate.producing_command == *expected_command
                        && candidate.path == ending
                })
                .collect();
            if matching.len() != 1 {
                contracts.push(format!(
                    "evidence.step.{}.{}.{} (expected 1, got {})",
                    expected.id,
                    step.id,
                    role,
                    matching.len()
                ));
            }
            if role == "step.result" && matching.len() == 1 {
                let result: Option<serde_json::Value> = read_bundle_file(root, &matching[0].path)
                    .ok()
                    .and_then(|bytes| serde_json::from_slice(&bytes).ok());
                let valid = result.as_ref().is_some_and(|receipt| {
                    exact_step_result_fields(receipt)
                        && receipt.get("schema").and_then(|value| value.as_str())
                            == Some("uqm-s4-step-result-v2")
                        && receipt.get("gate").and_then(|value| value.as_str())
                            == Some(expected.id.as_str())
                        && receipt.get("step").and_then(|value| value.as_str())
                            == Some(step.id.as_str())
                        && receipt.get("command") == Some(&serde_json::json!(expected_command))
                        && valid_step_execution_provenance(receipt, expected_command)
                        && valid_step_executable_identity(receipt)
                        && receipt.get("success").and_then(|value| value.as_bool()) == Some(true)
                        && receipt.get("exit_code").and_then(|value| value.as_i64()) == Some(0)
                        && receipt.get("signal").is_some_and(|value| value.is_null())
                        && receipt
                            .get("launch_error")
                            .is_some_and(|value| value.is_null())
                        && step_stream_lengths(index, &expected.id, &step.id).is_some_and(
                            |(stdout_bytes, stderr_bytes)| {
                                valid_step_supervision(
                                    receipt,
                                    stdout_bytes,
                                    stderr_bytes,
                                    Some(step.timeout_seconds * 1_000),
                                )
                            },
                        )
                });
                if !valid {
                    contracts.push(format!(
                        "evidence.step.{}.{}.result_content",
                        expected.id, step.id
                    ));
                }
            }
        }
        if expected.id == "security"
            && step.id == "advisory-db-revision"
            && !security_revision_matches(root, index, authority, step)
        {
            contracts.push("evidence.security.advisory_database_revision".to_string());
        }
    }
    if expected.id == "security" {
        contracts.extend(validate_security_advisory_database(root, index, authority));
    } else if expected.id == "package" {
        contracts.extend(validate_successful_package_gate(
            root, index, authority, expected,
        ));
    } else if expected.id == "tests" {
        contracts.extend(validate_native_acceptance_evidence(
            root, index, authority, expected,
        ));
    }
    contracts.extend(validate_subordinate_outputs(
        index,
        expected,
        expected.steps.len(),
        None,
    ));
    contracts.extend(validate_subordinate_semantics(
        root,
        index,
        expected,
        expected.steps.len(),
        None,
    ));
}

fn validate_failed_process_gate(
    root: &Path,
    index: &EvidenceIndex,
    authority: &Authority,
    failed_gate: &super::authority::Gate,
    first_failed: &str,
    contracts: &mut Vec<String>,
) {
    if failed_gate.id == "probes-harnesses" && first_failed.starts_with("probes-harnesses.pre.") {
        contracts.extend(validate_failed_subordinate_preprocess(
            root,
            index,
            failed_gate,
            first_failed,
        ));
    } else if failed_gate.id == "probes-harnesses"
        && first_failed.starts_with("probes-harnesses.post.")
    {
        contracts.extend(validate_failed_subordinate_postprocess(
            root,
            index,
            failed_gate,
            first_failed,
        ));
    } else if failed_gate.id == "package" && first_failed.starts_with("package.post.") {
        contracts.extend(validate_subordinate_outputs(
            index,
            failed_gate,
            failed_gate.steps.len(),
            None,
        ));
        contracts.extend(validate_failed_package_postprocess(
            root,
            index,
            authority,
            failed_gate,
            first_failed,
        ));
    } else if failed_gate.id == "security" && first_failed.starts_with("security.post.") {
        contracts.extend(validate_subordinate_outputs(
            index,
            failed_gate,
            failed_gate.steps.len(),
            None,
        ));
        contracts.extend(validate_failed_security_postprocess(
            root,
            index,
            authority,
            failed_gate,
            first_failed,
        ));
    } else if failed_gate.id == "tests"
        && first_failed == "tests.post.native-acceptance.native-window-acceptance"
    {
        contracts.extend(validate_failed_native_acceptance_postprocess(
            root,
            index,
            failed_gate,
        ));
    } else {
        contracts.extend(validate_failed_process_receipts(
            root,
            index,
            failed_gate,
            first_failed,
        ));
        if let Some(step_id) = first_failed.strip_prefix(&format!("{}.", failed_gate.id)) {
            if let Some(position) = failed_gate.steps.iter().position(|step| step.id == step_id) {
                contracts.extend(validate_subordinate_outputs(
                    index,
                    failed_gate,
                    position,
                    Some(position),
                ));
                contracts.extend(validate_subordinate_semantics(
                    root,
                    index,
                    failed_gate,
                    position,
                    Some(position),
                ));
            }
        }
        if failed_gate.id == "security"
            && first_failed == "security.cargo-audit"
            && failed_gate
                .steps
                .iter()
                .find(|step| step.id == "advisory-db-revision")
                .is_none_or(|step| !security_revision_matches(root, index, authority, step))
        {
            contracts.push("evidence.security.advisory_database_revision".to_string());
        }
        if failed_gate.id == "tests" && first_failed == "tests.native-acceptance" {
            contracts.extend(validate_failed_native_acceptance_evidence(
                root,
                index,
                authority,
                failed_gate,
            ));
        }
    }
}

fn validate_gate_and_step_receipts(
    root: &Path,
    index: &EvidenceIndex,
    authority: &Authority,
    snapshot: &EvidenceEntry,
) -> Vec<String> {
    let mut contracts = Vec::new();
    let command = &snapshot.producing_command;
    if command.len() != 4 || command[1] != "ci" || command[2] != "run" {
        return vec!["evidence.controller_command".to_string()];
    }
    let requested = &command[3];
    let requested_gates: Vec<_> = if requested == "all" {
        authority.gates.iter().collect()
    } else {
        match authority.gate(requested) {
            Some(gate) => vec![gate],
            None => return vec!["evidence.controller_command.gate".to_string()],
        }
    };
    let setup_gate = requested_gates[0].id.as_str();
    for (role, path) in [
        (
            "authority.snapshot",
            "payloads/authority.snapshot/gates.json",
        ),
        ("preflight.source", "source-preflight.json"),
        ("preflight.tools", "tool-preflight.json"),
        ("cache.initial-state", "cache-initial-state.json"),
    ] {
        validate_receipt_identity(index, role, path, setup_gate, command, &mut contracts);
    }
    validate_receipt_identity(
        index,
        "ownership.zero-native-delta",
        "zero-native-delta.json",
        "ownership-link",
        command,
        &mut contracts,
    );
    validate_singleton_role(index, "preflight.source", &mut contracts);
    validate_source_receipt(root, index, authority, &mut contracts);
    validate_singleton_role(index, "cache.initial-state", &mut contracts);
    validate_cache_receipt(root, index, authority, &mut contracts);
    validate_singleton_role(index, "preflight.tools", &mut contracts);
    let first_failed = index.first_failed_contract.as_deref();
    let preflight_failed = first_failed.is_some_and(is_preflight_failure);
    let setup_failed = first_failed.is_some_and(is_setup_failure);
    if index.cache_mode == "isolated-empty" && !preflight_failed {
        validate_singleton_role(index, "ownership.zero-native-delta", &mut contracts);
    }
    validate_delta_receipt(root, index, authority, &mut contracts);
    validate_tool_receipt(root, index, authority, &mut contracts);

    let subordinate_entries: Vec<_> = index
        .entries
        .iter()
        .filter(|entry| entry.role == "subordinate.output")
        .collect();
    for entry in &subordinate_entries {
        if !requested_gates
            .iter()
            .any(|gate| gate.kind == GateKind::Process && gate.id == entry.producing_gate)
        {
            contracts.push("evidence.subordinate.producing_gate".to_string());
        }
    }
    let gate_entries: Vec<_> = index
        .entries
        .iter()
        .filter(|entry| entry.role == "gate.result")
        .collect();
    if setup_failed {
        if !gate_entries.is_empty() {
            contracts.push("evidence.setup.unexpected_gate_results".to_string());
        }
        if !subordinate_entries.is_empty() {
            contracts.push("evidence.setup.unexpected_subordinate_outputs".to_string());
        }
        return contracts;
    }
    if index.first_failed_contract.is_none() && gate_entries.len() != requested_gates.len() {
        contracts.push(format!(
            "evidence.gate_result.count (expected {}, got {})",
            requested_gates.len(),
            gate_entries.len()
        ));
    }
    for (position, entry) in gate_entries.iter().enumerate() {
        let Some(expected) = requested_gates.get(position) else {
            contracts.push("evidence.gate_result.extra".to_string());
            break;
        };
        let Some(passed) =
            validate_gate_result_content(root, index, expected, entry, command, &mut contracts)
        else {
            continue;
        };

        if passed && expected.kind == GateKind::Process {
            validate_successful_process_gate(root, index, authority, expected, &mut contracts);
        } else if expected.kind == GateKind::Builtin {
            if passed {
                contracts.extend(validate_successful_builtin_gate(
                    root,
                    index,
                    authority,
                    &expected.id,
                ));
            } else if let Some(first_failed) = index.first_failed_contract.as_deref() {
                contracts.extend(validate_failed_builtin_gate(
                    root,
                    index,
                    authority,
                    &expected.id,
                    first_failed,
                ));
            }
        }
    }
    if let Some(first_failed) = index.first_failed_contract.as_deref() {
        if gate_entries.is_empty() {
            contracts.push("evidence.first_failed_contract.missing_gate_result".to_string());
        }
        if gate_entries.len() > requested_gates.len() {
            contracts.push("evidence.gate_result.extra".to_string());
        }
        if let Some(last) = gate_entries.last() {
            if let Some(failed_gate) = requested_gates.get(gate_entries.len() - 1) {
                if failed_gate.kind == GateKind::Process {
                    validate_failed_process_gate(
                        root,
                        index,
                        authority,
                        failed_gate,
                        first_failed,
                        &mut contracts,
                    );
                }
            }
            let receipt: Option<serde_json::Value> = read_bundle_file(root, &last.path)
                .ok()
                .and_then(|bytes| serde_json::from_slice(&bytes).ok());
            let correlated = receipt.as_ref().is_some_and(|receipt| {
                receipt.get("passed").and_then(|value| value.as_bool()) == Some(false)
                    && receipt
                        .get("first_failed_contract")
                        .and_then(|value| value.as_str())
                        == Some(first_failed)
            });
            if !correlated {
                contracts.push("evidence.first_failed_contract.gate_result".to_string());
            }
        }
    }
    contracts
}

fn security_revision_matches(
    root: &Path,
    index: &EvidenceIndex,
    authority: &Authority,
    step: &super::authority::Step,
) -> bool {
    index.entries.iter().any(|entry| {
        entry.role == "step.stdout"
            && entry.producing_gate == "security"
            && entry.producing_command == step.command
            && entry.path == "security/advisory-db-revision.stdout.log"
            && read_bundle_file(root, &entry.path).is_ok_and(|bytes| {
                bytes == format!("{}\n", authority.security.advisory_database_revision).as_bytes()
            })
    })
}

fn validate_security_advisory_database(
    root: &Path,
    index: &EvidenceIndex,
    authority: &Authority,
) -> Vec<String> {
    let mut contracts = Vec::new();
    let controller_command = index
        .entries
        .iter()
        .find(|entry| entry.role == "authority.snapshot")
        .map(|entry| entry.producing_command.as_slice());
    let matching: Vec<_> = index
        .entries
        .iter()
        .filter(|entry| entry.role == "security.advisory-database")
        .collect();
    if matching.len() != 1 {
        return vec![format!(
            "evidence.security.advisory_database.count (expected 1, got {})",
            matching.len()
        )];
    }
    let entry = matching[0];
    if entry.path != "payloads/security.advisory-database/advisory-database.pack"
        || entry.mime != "application/octet-stream"
        || entry.producing_gate != "security"
        || controller_command != Some(entry.producing_command.as_slice())
    {
        contracts.push("evidence.security.advisory_database.identity".to_string());
    }
    let bytes = read_bundle_file(root, &entry.path);
    let hash_matches = match bytes.as_ref() {
        Ok(bytes) => hex_sha256(bytes) == authority.security.advisory_database_pack_sha256,
        Err(_) => false,
    };
    if !hash_matches {
        contracts.push("evidence.security.advisory_database.hash".to_string());
    }
    match bytes {
        Ok(bytes) => match parse_advisory_database_pack(&bytes) {
            Ok(count) if count == authority.security.advisory_database_file_count as usize => {}
            Ok(_) => contracts.push("evidence.security.advisory_database.file_count".to_string()),
            Err(_) => contracts.push("evidence.security.advisory_database.format".to_string()),
        },
        Err(_) => contracts.push("evidence.security.advisory_database.format".to_string()),
    }
    contracts
}

fn parse_advisory_database_pack(bytes: &[u8]) -> Result<usize, &'static str> {
    const HEADER: &[u8] = b"UQM-S4-ADVISORY-DB-V1\0";
    if !bytes.starts_with(HEADER) {
        return Err("header");
    }
    let mut cursor = HEADER.len();
    let mut previous: Option<&str> = None;
    let mut count = 0_usize;
    loop {
        let path_length = read_pack_u32(bytes, &mut cursor)? as usize;
        if path_length == 0 {
            if cursor != bytes.len() {
                return Err("trailing bytes");
            }
            return Ok(count);
        }
        if path_length > 4096 {
            return Err("path length");
        }
        let content_length = read_pack_u64(bytes, &mut cursor)?;
        let content_length = usize::try_from(content_length).map_err(|_| "content length")?;
        let path_end = cursor.checked_add(path_length).ok_or("path overflow")?;
        let path_bytes = bytes.get(cursor..path_end).ok_or("truncated path")?;
        cursor = path_end;
        let path = std::str::from_utf8(path_bytes).map_err(|_| "path encoding")?;
        if !validate_relative_path(path)
            || path.split('/').any(|component| component == ".git")
            || previous.is_some_and(|previous| Path::new(previous) >= Path::new(path))
        {
            return Err("path identity");
        }
        previous = Some(path);
        cursor = cursor
            .checked_add(content_length)
            .filter(|end| *end <= bytes.len())
            .ok_or("content length")?;
        count = count.checked_add(1).ok_or("file count")?;
    }
}

fn read_pack_u32(bytes: &[u8], cursor: &mut usize) -> Result<u32, &'static str> {
    let end = cursor.checked_add(4).ok_or("integer overflow")?;
    let value = bytes.get(*cursor..end).ok_or("truncated integer")?;
    *cursor = end;
    Ok(u32::from_be_bytes(value.try_into().unwrap()))
}

fn read_pack_u64(bytes: &[u8], cursor: &mut usize) -> Result<u64, &'static str> {
    let end = cursor.checked_add(8).ok_or("integer overflow")?;
    let value = bytes.get(*cursor..end).ok_or("truncated integer")?;
    *cursor = end;
    Ok(u64::from_be_bytes(value.try_into().unwrap()))
}

fn validate_failed_security_postprocess(
    root: &Path,
    index: &EvidenceIndex,
    authority: &Authority,
    gate: &super::authority::Gate,
    first_failed: &str,
) -> Vec<String> {
    let mut contracts = Vec::new();
    if !matches!(
        first_failed,
        "security.post.database-identity" | "security.post.database-retain"
    ) {
        contracts.push("evidence.security.post.contract".to_string());
    }
    if builtin_subordinate_count(index, &gate.id) != gate.steps.len() * 3 {
        contracts.push("evidence.security.post.step_entry_count".to_string());
    }
    for step in &gate.steps {
        contracts.extend(validate_builtin_step(
            root,
            index,
            &gate.id,
            &step.id,
            &[0],
            |command| command == step.command,
        ));
        if step.id == "advisory-db-revision"
            && !security_revision_matches(root, index, authority, step)
        {
            contracts.push("evidence.security.advisory_database_revision".to_string());
        }
    }
    if index
        .entries
        .iter()
        .any(|entry| entry.role == "security.advisory-database")
    {
        contracts.push("evidence.security.post.unexpected_database".to_string());
    }
    contracts
}
fn validate_failed_native_acceptance_postprocess(
    root: &Path,
    index: &EvidenceIndex,
    gate: &super::authority::Gate,
) -> Vec<String> {
    let mut contracts = Vec::new();
    let Some(step) = gate
        .steps
        .iter()
        .find(|step| step.id == "native-acceptance")
    else {
        return vec!["evidence.native_window.authority_step".to_string()];
    };
    contracts.extend(validate_builtin_step(
        root,
        index,
        &gate.id,
        &step.id,
        &[0],
        |command| command == step.command,
    ));
    let entries: Vec<_> = index
        .entries
        .iter()
        .filter(|entry| entry.role == "native-window.acceptance")
        .collect();
    for entry in entries {
        if entry.producing_gate != gate.id
            || entry.producing_command != step.command
            || !entry.path.starts_with("payloads/native-window.acceptance/")
        {
            contracts.push("evidence.native_window.failed_entry_identity".to_string());
        }
    }
    contracts
}
#[derive(serde::Deserialize)]
#[serde(untagged)]
enum NativeAcceptanceFailureEnvelope {
    Runtime(Box<uqm_rust::automation::NativeAcceptanceFailureManifest>),
    Setup(Box<uqm_rust::automation::NativeAcceptanceSetupFailureManifest>),
}

fn validate_failed_native_acceptance_evidence(
    root: &Path,
    index: &EvidenceIndex,
    authority: &Authority,
    gate: &super::authority::Gate,
) -> Vec<String> {
    let mut contracts = Vec::new();
    let entries: Vec<_> = index
        .entries
        .iter()
        .filter(|entry| entry.role == "native-window.failure")
        .collect();
    if entries.is_empty() {
        return contracts;
    }
    if !index.tuple.starts_with("macos-") {
        return vec!["evidence.native_window.failure.platform".to_string()];
    }
    let Some(step) = gate
        .steps
        .iter()
        .find(|step| step.id == "native-acceptance")
    else {
        return vec!["evidence.native_window.failure.step".to_string()];
    };
    let acceptance_root = root.join("payloads/native-window.acceptance");
    let mut actual_paths = Vec::new();
    collect_native_acceptance_paths(&acceptance_root, &mut actual_paths, &mut contracts);
    actual_paths.sort();
    let expected_paths: Vec<_> = actual_paths
        .iter()
        .map(|path| format!("payloads/native-window.acceptance/{path}"))
        .collect();
    let mut indexed_paths: Vec<_> = entries.iter().map(|entry| entry.path.clone()).collect();
    indexed_paths.sort();
    if expected_paths != indexed_paths
        || entries.iter().any(|entry| {
            entry.producing_gate != gate.id
                || entry.producing_command != step.command
                || entry.mime != "application/octet-stream"
        })
    {
        contracts.push("evidence.native_window.failure.inventory".to_string());
    }
    let manifest_path = "payloads/native-window.acceptance/native-acceptance-failure.json";
    let manifest: Option<NativeAcceptanceFailureEnvelope> = read_bundle_file(root, manifest_path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok());
    let Some(manifest) = manifest else {
        contracts.push("evidence.native_window.failure.manifest".to_string());
        return contracts;
    };
    let native = &authority.native_acceptance;
    let manifest = match manifest {
        NativeAcceptanceFailureEnvelope::Runtime(manifest) => manifest,
        NativeAcceptanceFailureEnvelope::Setup(manifest) => {
            if manifest.runtime_contract != authority.native_runtime_contract()
                || manifest.acceptance_policy != native.acceptance_policy
            {
                contracts.push("evidence.native_window.failure.authority_contract".to_string());
            }
            if uqm_rust::automation::validate_native_acceptance_setup_failure_bundle(
                &acceptance_root,
                &manifest,
            )
            .is_err()
            {
                contracts.push("evidence.native_window.failure.result".to_string());
            }
            return contracts;
        }
    };
    if manifest.runtime_contract != authority.native_runtime_contract()
        || manifest.acceptance_policy != native.acceptance_policy
    {
        contracts.push("evidence.native_window.failure.authority_contract".to_string());
    }
    if uqm_rust::automation::validate_native_acceptance_failure_bundle(&acceptance_root, &manifest)
        .is_err()
    {
        contracts.push("evidence.native_window.failure.result".to_string());
        return contracts;
    }
    let expected_script = Path::new(&native.script)
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| format!("inputs/{name}"));
    if expected_script.as_deref() != Some(manifest.script.relative_path.as_str())
        || manifest.script.sha256 != native.script_sha256
        || manifest.script.byte_length != native.script_byte_length
    {
        contracts.push("evidence.native_window.failure.script".to_string());
    }
    if manifest.content_package.relative_path
        != format!("inputs/content/packages/{}", native.content_filename)
        || manifest.content_package.sha256 != native.content_sha256
        || manifest.content_package.byte_length != native.content_byte_length
    {
        contracts.push("evidence.native_window.failure.content".to_string());
    }
    let version_bytes = format!("{}\n", native.content_version).into_bytes();
    let version_input = manifest
        .retained_files
        .iter()
        .find(|input| input.relative_path == "inputs/content/version");
    if version_input.is_none_or(|input| {
        input.byte_length != version_bytes.len() as u64
            || input.sha256 != hex_sha256(&version_bytes)
    }) {
        contracts.push("evidence.native_window.failure.content_version".to_string());
    }
    contracts
}

fn validate_native_acceptance_evidence(
    root: &Path,
    index: &EvidenceIndex,
    authority: &Authority,
    gate: &super::authority::Gate,
) -> Vec<String> {
    let entries: Vec<_> = index
        .entries
        .iter()
        .filter(|entry| entry.role == "native-window.acceptance")
        .collect();
    let acceptance_root = root.join("payloads/native-window.acceptance");
    let native_tuple_prefix = format!("{}-", authority.native_acceptance.platform);
    if !index.tuple.starts_with(&native_tuple_prefix) {
        let payload_exists = match fs::symlink_metadata(&acceptance_root) {
            Ok(_) => true,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(_) => true,
        };
        return if entries.is_empty() && !payload_exists {
            Vec::new()
        } else {
            vec!["evidence.native_window.unexpected_tuple".to_string()]
        };
    }
    let Some(step) = gate
        .steps
        .iter()
        .find(|step| step.id == "native-acceptance")
    else {
        return vec!["evidence.native_window.authority_step".to_string()];
    };
    let mut contracts = Vec::new();
    let mut actual_paths = Vec::new();
    collect_native_acceptance_paths(&acceptance_root, &mut actual_paths, &mut contracts);
    if !contracts.is_empty() {
        return contracts;
    }
    actual_paths.sort();
    let mut indexed_paths: Vec<_> = entries.iter().map(|entry| entry.path.clone()).collect();
    indexed_paths.sort();
    let expected_paths: Vec<_> = actual_paths
        .iter()
        .map(|path| format!("payloads/native-window.acceptance/{path}"))
        .collect();
    if indexed_paths != expected_paths {
        contracts.push("evidence.native_window.file_set".to_string());
    }
    for entry in &entries {
        if entry.producing_gate != gate.id
            || entry.producing_command != step.command
            || !entry.path.starts_with("payloads/native-window.acceptance/")
        {
            contracts.push("evidence.native_window.entry_identity".to_string());
        }
    }
    let manifest: Option<uqm_rust::automation::NativeAcceptanceManifest> = read_bundle_file(
        root,
        "payloads/native-window.acceptance/native-acceptance.json",
    )
    .ok()
    .and_then(|bytes| serde_json::from_slice(&bytes).ok());
    match manifest {
        Some(manifest) => {
            if uqm_rust::automation::validate_native_acceptance_bundle(&acceptance_root, &manifest)
                .is_err()
            {
                contracts.push("evidence.native_window.receipt".to_string());
            }
            let linked_receipt = read_bundle_file(
                root,
                "payloads/native-window.acceptance/inputs/linked-build/linked-build-receipt.json",
            );
            let nested_authority = read_bundle_file(
                root,
                "payloads/native-window.acceptance/inputs/linked-build/gates.json",
            );
            let outer_authority = read_bundle_file(root, "payloads/authority.snapshot/gates.json");
            contracts.extend(linked_outer_correlation_contracts(
                linked_receipt.as_deref().ok(),
                nested_authority.as_deref().ok(),
                outer_authority.as_deref().ok(),
                &index.source_sha,
            ));
            let content = &authority.native_acceptance;
            if manifest.runtime_contract != authority.native_runtime_contract()
                || manifest.acceptance_policy != content.acceptance_policy
            {
                contracts.push("evidence.native_window.authority_contract".to_string());
            }
            if manifest.content_package.relative_path
                != format!("inputs/content/packages/{}", content.content_filename)
                || manifest.content_package.byte_length != content.content_byte_length
                || manifest.content_package.sha256 != content.content_sha256
            {
                contracts.push("evidence.native_window.content_package".to_string());
            }
            let version_bytes = format!("{}\n", content.content_version).into_bytes();
            let version_input = manifest
                .retained_files
                .iter()
                .find(|input| input.relative_path == "inputs/content/version");
            if version_input.is_none_or(|input| {
                input.byte_length != version_bytes.len() as u64
                    || input.sha256 != hex_sha256(&version_bytes)
            }) {
                contracts.push("evidence.native_window.content_version".to_string());
            }
            let expected_script = Path::new(&content.script)
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| format!("inputs/{name}"));
            if expected_script.as_deref() != Some(manifest.script.relative_path.as_str())
                || manifest.script.byte_length != content.script_byte_length
                || manifest.script.sha256 != content.script_sha256
            {
                contracts.push("evidence.native_window.script".to_string());
            }
        }
        None => contracts.push("evidence.native_window.manifest".to_string()),
    }
    contracts
}

fn linked_outer_correlation_contracts(
    linked_receipt: Option<&[u8]>,
    nested_authority: Option<&[u8]>,
    outer_authority: Option<&[u8]>,
    source_sha: &str,
) -> Vec<String> {
    let mut contracts = Vec::new();
    let linked_source_matches = linked_receipt
        .and_then(|bytes| {
            serde_json::from_slice::<uqm_rust::automation::NativeLinkedBuildReceipt>(bytes).ok()
        })
        .is_some_and(|receipt| receipt.source_sha == source_sha);
    if !linked_source_matches {
        contracts.push("evidence.native_window.linked_source".to_string());
    }
    if nested_authority
        .zip(outer_authority)
        .is_none_or(|(nested, outer)| nested != outer)
    {
        contracts.push("evidence.native_window.linked_authority".to_string());
    }
    contracts
}
fn collect_native_acceptance_paths(
    root: &Path,
    paths: &mut Vec<String>,
    contracts: &mut Vec<String>,
) {
    match regular_file_inventory(root) {
        Ok(files) => paths.extend(files.into_iter().map(|file| file.relative_path)),
        Err(error)
            if matches!(
                error.raw_os_error(),
                Some(libc::ELOOP) | Some(libc::ENOTDIR)
            ) =>
        {
            contracts.push("evidence.native_window.non_regular".to_string());
        }
        Err(error) => contracts.push(format!("evidence.native_window.walk ({error})")),
    }
}

fn subordinate_output_names(gate: &str, step: &str) -> &'static [&'static str] {
    match (gate, step) {
        ("probes-harnesses", "p00-probes") => &["p00-probe-results.log"],
        ("probes-harnesses", "p00-harness") => &[
            "pkg-config-cflags.txt",
            "pkg-config-libs.txt",
            "link-map.txt",
            "archive-nm.txt",
            "archive-nm.stderr.txt",
            "archive-nm.exit.txt",
            "archive-nm-origins.txt",
            "harness-nm.txt",
            "harness-nm.stderr.txt",
            "harness-nm.exit.txt",
            "object-manifest.txt",
            "harness-output.txt",
            "harness-exit.txt",
        ],
        ("probes-harnesses", "menu-binding-probe") => &[
            "pkg-config-cflags.txt",
            "pkg-config-libs.txt",
            "menu-binding-link-map.txt",
            "c-archive-nm.txt",
            "c-archive-nm.stderr.txt",
            "c-archive-nm.exit.txt",
            "c-archive-nm-origins.txt",
            "rust-archive-nm.txt",
            "rust-archive-nm.stderr.txt",
            "rust-archive-nm.exit.txt",
            "rust-archive-nm-origins.txt",
            "harness-archive-nm.txt",
            "harness-archive-nm.stderr.txt",
            "harness-archive-nm.exit.txt",
            "harness-archive-nm-origins.txt",
            "probe-binary-nm.txt",
            "probe-binary-nm.stderr.txt",
            "probe-binary-nm.exit.txt",
            "probe-output.txt",
            "probe-exit.txt",
        ],
        _ => &[],
    }
}

fn validate_subordinate_outputs(
    index: &EvidenceIndex,
    gate: &super::authority::Gate,
    completed_steps: usize,
    partial_step: Option<usize>,
) -> Vec<String> {
    let mut contracts = Vec::new();
    let entries: Vec<_> = index
        .entries
        .iter()
        .filter(|entry| entry.role == "subordinate.output" && entry.producing_gate == gate.id)
        .collect();
    let mut expected_paths = BTreeSet::new();
    let mut optional_paths = BTreeSet::new();
    for (position, step) in gate.steps.iter().enumerate() {
        let destination = if position < completed_steps {
            &mut expected_paths
        } else if Some(position) == partial_step {
            &mut optional_paths
        } else {
            continue;
        };

        for name in subordinate_output_names(&gate.id, &step.id) {
            destination.insert((
                format!(
                    "payloads/subordinate.output/{}/{}/{}",
                    gate.id, step.id, name
                ),
                step.command.clone(),
            ));
        }
    }
    for (path, command) in &expected_paths {
        let count = entries
            .iter()
            .filter(|entry| entry.path == *path && entry.producing_command == *command)
            .count();
        if count != 1 {
            contracts.push(format!(
                "evidence.subordinate.{}.required (expected 1, got {count})",
                path
            ));
        }
    }
    for entry in &entries {
        let identity = (entry.path.clone(), entry.producing_command.clone());
        if !expected_paths.contains(&identity) && !optional_paths.contains(&identity) {
            contracts.push(format!("evidence.subordinate.{}.unexpected", entry.path));
        }
    }
    contracts
}
fn subordinate_bytes(
    root: &Path,
    index: &EvidenceIndex,
    gate: &super::authority::Gate,
    step: &super::authority::Step,
    name: &str,
) -> Option<Vec<u8>> {
    let path = format!(
        "payloads/subordinate.output/{}/{}/{}",
        gate.id, step.id, name
    );
    let mut matching = index.entries.iter().filter(|entry| {
        entry.role == "subordinate.output"
            && entry.path == path
            && entry.producing_gate == gate.id
            && entry.producing_command == step.command
    });
    let entry = matching.next()?;
    if matching.next().is_some() {
        return None;
    }
    read_bundle_file(root, &entry.path).ok()
}

fn nm_prefixes(step: &str) -> &'static [&'static str] {
    match step {
        "p00-harness" => &["archive", "harness"],
        "menu-binding-probe" => &[
            "c-archive",
            "rust-archive",
            "harness-archive",
            "probe-binary",
        ],
        _ => &[],
    }
}

fn parse_nm_exit(bytes: &[u8]) -> Option<i32> {
    let text = std::str::from_utf8(bytes).ok()?;
    let digits = text.strip_suffix('\n')?;
    let status = digits.parse().ok()?;
    ((0..=255).contains(&status) && text == format!("{status}\n")).then_some(status)
}

fn valid_selected_origins(bytes: &[u8], expected: &[(&str, &str)]) -> bool {
    let Ok(receipt) = std::str::from_utf8(bytes) else {
        return false;
    };
    let lines = receipt.lines().collect::<Vec<_>>();
    lines.len() == expected.len()
        && lines
            .iter()
            .zip(expected)
            .all(|(line, (expected_symbol, expected_hint))| {
                let fields = line.splitn(3, '\t').collect::<Vec<_>>();
                if fields.len() != 3 {
                    return false;
                }
                let hint_matches = if *expected_hint == "*" {
                    !fields[1].is_empty() && fields[2].contains(fields[1])
                } else {
                    fields[1] == *expected_hint
                        && (expected_hint.is_empty() || fields[2].contains(expected_hint))
                };
                if fields[0] != *expected_symbol || !hint_matches || fields[2].is_empty() {
                    return false;
                }
                let words = fields[2].split_whitespace().collect::<Vec<_>>();
                words.len() >= 2
                    && words[words.len() - 2] == "T"
                    && words.last().copied().is_some_and(|symbol| {
                        symbol == *expected_symbol
                            || symbol.strip_prefix('_') == Some(*expected_symbol)
                    })
            })
}

fn valid_menu_probe_output(bytes: &[u8]) -> bool {
    std::str::from_utf8(bytes).ok().is_some_and(|output| {
        output
            .lines()
            .filter_map(|line| line.strip_prefix("key_code="))
            .eq(["1073741905"])
    })
}

fn validate_step_subordinate_semantics(
    root: &Path,
    index: &EvidenceIndex,
    gate: &super::authority::Gate,
    step: &super::authority::Step,
    complete: bool,
) -> Vec<String> {
    let mut contracts = Vec::new();
    if complete && step.id == "p00-probes" {
        const PROBES: &[&str] = &[
            "lock_free_atomics",
            "monotonic_instant",
            "unix_datagram",
            "file_primitives",
            "process_identity",
            "sdl_dummy_hidden",
        ];
        let valid = subordinate_bytes(root, index, gate, step, "p00-probe-results.log")
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .is_some_and(|log| {
                let lines = log.lines().collect::<Vec<_>>();
                PROBES.iter().all(|probe| {
                    let prefix = format!("PASS {probe}:");
                    lines
                        .iter()
                        .filter(|line| line.starts_with(&prefix))
                        .count()
                        == 1
                }) && lines
                    .iter()
                    .filter(|line| **line == "P00 probes: 6 passed, 0 failed")
                    .count()
                    == 1
                    && !lines.iter().any(|line| line.starts_with("FAIL "))
            });
        if !valid {
            contracts.push("evidence.subordinate.p00-probes.results".to_string());
        }
    }

    for prefix in nm_prefixes(&step.id) {
        let stdout = subordinate_bytes(root, index, gate, step, &format!("{prefix}-nm.txt"));
        let stderr = subordinate_bytes(root, index, gate, step, &format!("{prefix}-nm.stderr.txt"));
        let status = subordinate_bytes(root, index, gate, step, &format!("{prefix}-nm.exit.txt"));
        let present = usize::from(stdout.is_some())
            + usize::from(stderr.is_some())
            + usize::from(status.is_some());
        if present == 0 && !complete {
            continue;
        }
        if present != 3 {
            contracts.push(format!(
                "evidence.subordinate.{}.{}.nm_triplet",
                step.id, prefix
            ));
            continue;
        }
        let parsed = status.as_deref().and_then(parse_nm_exit);
        if parsed.is_none() {
            contracts.push(format!(
                "evidence.subordinate.{}.{}.nm_exit",
                step.id, prefix
            ));
        } else if complete && parsed != Some(0) {
            contracts.push(format!(
                "evidence.subordinate.{}.{}.nm_nonzero",
                step.id, prefix
            ));
        }
    }

    if complete && step.id == "p00-harness" {
        const EXPECTED: &[(&str, &str)] = &[
            ("DoInput", "*"),
            ("AnyButtonPress", "*"),
            ("DoConfirmExit", "*"),
            ("TFB_ProcessEvents", "*"),
            ("TFB_SwapBuffers", "*"),
            ("ProcessInputEvent", "*"),
            ("TFB_FlushGraphicsEx", "*"),
        ];
        let valid = subordinate_bytes(root, index, gate, step, "archive-nm-origins.txt")
            .is_some_and(|bytes| valid_selected_origins(&bytes, EXPECTED));
        if !valid {
            contracts.push("evidence.subordinate.p00-harness.nm_origins".to_string());
        }
    } else if complete && step.id == "menu-binding-probe" {
        const C_EXPECTED: &[(&str, &str)] = &[
            ("VControl_ParseGesture", "rust_vcontrol_impl.c.o"),
            ("InstallGraphicResTypes", "resgfx.c.o"),
            ("InstallStringTableResType", "sresins.c.o"),
        ];
        const RUST_EXPECTED: &[(&str, &str)] = &[
            ("InitResourceSystem", ""),
            ("LoadResourceIndex", ""),
            ("res_IsString", ""),
            ("res_GetString", ""),
            ("uio_openRepository", ""),
            ("uio_mountDir", ""),
            ("uio_openDir", ""),
        ];
        const HARNESS_EXPECTED: &[(&str, &str)] =
            &[("uqm_query_menu_binding", "menu_binding_accessor.o")];
        for (name, expected) in [
            ("c-archive-nm-origins.txt", C_EXPECTED),
            ("rust-archive-nm-origins.txt", RUST_EXPECTED),
            ("harness-archive-nm-origins.txt", HARNESS_EXPECTED),
        ] {
            let valid = subordinate_bytes(root, index, gate, step, name)
                .is_some_and(|bytes| valid_selected_origins(&bytes, expected));
            if !valid {
                contracts.push(format!("evidence.subordinate.menu-binding-probe.{name}"));
            }
        }
        let valid_key = subordinate_bytes(root, index, gate, step, "probe-output.txt")
            .is_some_and(|bytes| valid_menu_probe_output(&bytes));
        if !valid_key {
            contracts.push("evidence.subordinate.menu-binding-probe.sdlk_down".to_string());
        }
    }
    contracts
}

fn validate_subordinate_semantics(
    root: &Path,
    index: &EvidenceIndex,
    gate: &super::authority::Gate,
    completed_steps: usize,
    partial_step: Option<usize>,
) -> Vec<String> {
    let mut contracts = Vec::new();
    for step in gate.steps.iter().take(completed_steps) {
        contracts.extend(validate_step_subordinate_semantics(
            root, index, gate, step, true,
        ));
    }
    if let Some(position) = partial_step {
        if let Some(step) = gate.steps.get(position) {
            contracts.extend(validate_step_subordinate_semantics(
                root, index, gate, step, false,
            ));
        }
    }
    contracts
}

fn validate_failed_subordinate_preprocess(
    root: &Path,
    index: &EvidenceIndex,
    gate: &super::authority::Gate,
    first_failed: &str,
) -> Vec<String> {
    let mut contracts = Vec::new();
    let Some(step_id) = first_failed
        .strip_prefix(&format!("{}.pre.", gate.id))
        .and_then(|suffix| suffix.strip_suffix(".subordinate-output"))
    else {
        return vec![format!("evidence.subordinate.{}.failure_contract", gate.id)];
    };
    let Some(position) = gate.steps.iter().position(|step| step.id == step_id) else {
        return vec![format!("evidence.subordinate.{}.failure_step", gate.id)];
    };
    let actual_step_entries = index
        .entries
        .iter()
        .filter(|entry| {
            entry.producing_gate == gate.id
                && matches!(
                    entry.role.as_str(),
                    "step.stdout" | "step.stderr" | "step.result"
                )
        })
        .count();
    let expected_step_entries = position * 3;
    if actual_step_entries != expected_step_entries {
        contracts.push(format!(
            "evidence.subordinate.{}.step_entry_count (expected {expected_step_entries}, got {actual_step_entries})",
            gate.id
        ));
    }
    for step in gate.steps.iter().take(position) {
        contracts.extend(validate_builtin_step(
            root,
            index,
            &gate.id,
            &step.id,
            &[0],
            |command| command == step.command,
        ));
    }
    contracts.extend(validate_subordinate_outputs(index, gate, position, None));
    contracts.extend(validate_subordinate_semantics(
        root, index, gate, position, None,
    ));
    contracts
}

fn validate_failed_subordinate_postprocess(
    root: &Path,
    index: &EvidenceIndex,
    gate: &super::authority::Gate,
    first_failed: &str,
) -> Vec<String> {
    let mut contracts = Vec::new();
    let Some(step_id) = first_failed
        .strip_prefix(&format!("{}.post.", gate.id))
        .and_then(|suffix| suffix.strip_suffix(".subordinate-output"))
    else {
        return vec![format!("evidence.subordinate.{}.failure_contract", gate.id)];
    };
    let Some(position) = gate.steps.iter().position(|step| step.id == step_id) else {
        return vec![format!("evidence.subordinate.{}.failure_step", gate.id)];
    };
    let actual_step_entries = index
        .entries
        .iter()
        .filter(|entry| {
            entry.producing_gate == gate.id
                && matches!(
                    entry.role.as_str(),
                    "step.stdout" | "step.stderr" | "step.result"
                )
        })
        .count();
    let expected_step_entries = (position + 1) * 3;
    if actual_step_entries != expected_step_entries {
        contracts.push(format!(
            "evidence.subordinate.{}.step_entry_count (expected {expected_step_entries}, got {actual_step_entries})",
            gate.id
        ));
    }
    for step in gate.steps.iter().take(position + 1) {
        contracts.extend(validate_builtin_step(
            root,
            index,
            &gate.id,
            &step.id,
            &[0],
            |command| command == step.command,
        ));
    }
    contracts.extend(validate_subordinate_outputs(
        index,
        gate,
        position,
        Some(position),
    ));
    contracts.extend(validate_subordinate_semantics(
        root,
        index,
        gate,
        position,
        Some(position),
    ));
    contracts
}

fn validate_failed_process_receipts(
    root: &Path,
    index: &EvidenceIndex,
    gate: &super::authority::Gate,
    first_failed: &str,
) -> Vec<String> {
    let mut contracts = Vec::new();
    let Some(step_id) = first_failed.strip_prefix(&format!("{}.", gate.id)) else {
        return vec![format!("evidence.gate.{}.failed_step_contract", gate.id)];
    };
    let Some(failed_position) = gate.steps.iter().position(|step| step.id == step_id) else {
        return vec![format!("evidence.gate.{}.failed_step_identity", gate.id)];
    };
    let actual_count = index
        .entries
        .iter()
        .filter(|entry| {
            entry.producing_gate == gate.id
                && matches!(
                    entry.role.as_str(),
                    "step.stdout" | "step.stderr" | "step.result"
                )
        })
        .count();
    let expected_count = (failed_position + 1) * 3;
    if actual_count != expected_count {
        contracts.push(format!(
            "evidence.gate.{}.failed_step_entry_count (expected {expected_count}, got {actual_count})",
            gate.id
        ));
    }
    for (position, step) in gate.steps.iter().take(failed_position + 1).enumerate() {
        for (suffix, role) in [
            ("stdout.log", "step.stdout"),
            ("stderr.log", "step.stderr"),
            ("result.json", "step.result"),
        ] {
            let ending = format!("{}/{}.{}", gate.id, step.id, suffix);
            let matching: Vec<_> = index
                .entries
                .iter()
                .filter(|entry| {
                    entry.role == role
                        && entry.producing_gate == gate.id
                        && entry.producing_command == step.command
                        && entry.path == ending
                })
                .collect();
            if matching.len() != 1 {
                contracts.push(format!(
                    "evidence.step.{}.{}.{} (expected 1, got {})",
                    gate.id,
                    step.id,
                    role,
                    matching.len()
                ));
                continue;
            }
            if role != "step.result" {
                continue;
            }
            let receipt: Option<serde_json::Value> = read_bundle_file(root, &matching[0].path)
                .ok()
                .and_then(|bytes| serde_json::from_slice(&bytes).ok());
            let should_pass = position < failed_position;
            let valid = receipt.as_ref().is_some_and(|receipt| {
                exact_step_result_fields(receipt)
                    && receipt.get("schema").and_then(|value| value.as_str())
                        == Some("uqm-s4-step-result-v2")
                    && receipt.get("gate").and_then(|value| value.as_str())
                        == Some(gate.id.as_str())
                    && receipt.get("step").and_then(|value| value.as_str())
                        == Some(step.id.as_str())
                    && receipt.get("command") == Some(&serde_json::json!(step.command))
                    && valid_step_execution_provenance(receipt, &step.command)
                    && valid_step_executable_identity(receipt)
                    && (if should_pass {
                        receipt.get("success").and_then(|value| value.as_bool()) == Some(true)
                            && spawned_exit_code(receipt) == Some(0)
                    } else {
                        valid_failed_step_terminal(receipt)
                    })
                    && step_stream_lengths(index, &gate.id, &step.id).is_some_and(
                        |(stdout_bytes, stderr_bytes)| {
                            valid_step_supervision(
                                receipt,
                                stdout_bytes,
                                stderr_bytes,
                                Some(step.timeout_seconds * 1_000),
                            )
                        },
                    )
            });
            if !valid {
                contracts.push(format!(
                    "evidence.step.{}.{}.failed_result_content",
                    gate.id, step.id
                ));
            }
        }
    }
    contracts
}

fn validate_failed_package_postprocess(
    root: &Path,
    index: &EvidenceIndex,
    authority: &Authority,
    gate: &super::authority::Gate,
    first_failed: &str,
) -> Vec<String> {
    let mut contracts = Vec::new();
    if builtin_subordinate_count(index, &gate.id) != gate.steps.len() * 3 {
        contracts.push("evidence.package.post.step_entry_count".to_string());
    }
    for step in &gate.steps {
        contracts.extend(validate_builtin_step(
            root,
            index,
            &gate.id,
            &step.id,
            &[0],
            |command| command == step.command,
        ));
    }

    let mut expected_roles = Vec::new();
    let artifact_failure = first_failed.strip_prefix("package.post.artifact.");
    match first_failed {
        "package.post.manifest-read" | "package.post.manifest-retain" => {}
        "package.post.ownership-report"
        | "package.post.dependencies-validate"
        | "package.post.dependencies-retain" => {
            expected_roles.push("package-manifest".to_string());
            expected_roles.extend(
                authority
                    .package
                    .artifacts
                    .iter()
                    .map(|artifact| format!("package-{}", artifact.role)),
            );
            if first_failed != "package.post.ownership-report" {
                expected_roles.push("ownership-production-report".to_string());
            }
            if first_failed == "package.post.dependencies-validate" {
                expected_roles.push("native-dependency-capture".to_string());
            }
        }
        _ if artifact_failure.is_some() => {
            let failed_role = artifact_failure.unwrap_or_default();
            let Some(failed_position) = authority
                .package
                .artifacts
                .iter()
                .position(|artifact| artifact.role == failed_role)
            else {
                return vec!["evidence.package.post.contract".to_string()];
            };
            expected_roles.push("package-manifest".to_string());
            expected_roles.extend(
                authority
                    .package
                    .artifacts
                    .iter()
                    .take(failed_position)
                    .map(|artifact| format!("package-{}", artifact.role)),
            );
        }
        _ => return vec!["evidence.package.post.contract".to_string()],
    }
    expected_roles.sort();
    let mut actual_roles: Vec<_> = index
        .entries
        .iter()
        .filter(|entry| {
            entry.producing_gate == gate.id
                && (entry.role == "package-manifest"
                    || entry.role.starts_with("package-")
                    || entry.role == "ownership-production-report"
                    || entry.role == "native-dependency-capture")
        })
        .map(|entry| entry.role.clone())
        .collect();
    actual_roles.sort();
    if actual_roles != expected_roles {
        contracts.push("evidence.package.post.supplemental_prefix".to_string());
    }

    let Some(package_step) = gate.steps.iter().find(|step| step.id == "xtask-package") else {
        contracts.push("evidence.package.authority".to_string());
        return contracts;
    };
    let manifests: Vec<_> = index
        .entries
        .iter()
        .filter(|entry| entry.role == "package-manifest")
        .collect();
    if expected_roles.iter().any(|role| role == "package-manifest") {
        if manifests.len() != 1
            || manifests[0].producing_gate != gate.id
            || manifests[0].producing_command != package_step.command
            || manifests[0].path != "payloads/package-manifest/production-artifacts.json"
        {
            contracts.push("evidence.package.post.manifest.identity".to_string());
        } else {
            let manifest: Option<serde_json::Value> = read_bundle_file(root, &manifests[0].path)
                .ok()
                .and_then(|bytes| serde_json::from_slice(&bytes).ok());
            let manifest_valid = manifest
                .as_ref()
                .is_some_and(|manifest| valid_package_manifest_content(index, authority, manifest));
            if !manifest_valid {
                contracts.push("evidence.package.post.manifest.content".to_string());
            }
        }
    } else if !manifests.is_empty() {
        contracts.push("evidence.package.post.manifest.unexpected".to_string());
    }
    if let Some(manifest) = manifests.first().and_then(|entry| {
        read_bundle_file(root, &entry.path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
    }) {
        let artifacts = manifest
            .get("artifacts")
            .and_then(serde_json::Value::as_array);
        for expected in &authority.package.artifacts {
            if expected_roles.contains(&format!("package-{}", expected.role))
                && !valid_package_artifact_evidence(index, &gate.id, artifacts, expected)
            {
                contracts.push(format!(
                    "evidence.package.post.artifact.{}.evidence",
                    expected.role
                ));
            }
        }
        if expected_roles
            .iter()
            .any(|role| role == "ownership-production-report")
        {
            let Some(ownership_step) = gate
                .steps
                .iter()
                .find(|step| step.id == "verify-production-ownership")
            else {
                contracts.push("evidence.package.authority".to_string());
                return contracts;
            };
            let ownership: Vec<_> = index
                .entries
                .iter()
                .filter(|entry| entry.role == "ownership-production-report")
                .collect();
            if ownership.len() != 1
                || ownership[0].producing_gate != gate.id
                || ownership[0].producing_command != ownership_step.command
                || ownership[0].path
                    != "payloads/ownership-production-report/ownership-production-report.json"
            {
                contracts.push("evidence.package.post.ownership-report.identity".to_string());
            }
            validate_package_ownership_report(root, index, authority, artifacts, &mut contracts);
        }
    }
    if first_failed == "package.post.dependencies-validate" {
        let capture_step = gate
            .steps
            .iter()
            .find(|step| step.id == "capture-native-dependencies");
        let captures: Vec<_> = index
            .entries
            .iter()
            .filter(|entry| entry.role == "native-dependency-capture")
            .collect();
        let identity_valid = capture_step.is_some_and(|step| {
            captures.len() == 1
                && captures[0].producing_gate == gate.id
                && captures[0].producing_command == step.command
                && captures[0].path
                    == format!(
                        "payloads/native-dependency-capture/native-dependencies-{}.candidate.json",
                        index.tuple
                    )
        });
        if !identity_valid {
            contracts.push("evidence.package.post.dependencies.identity".to_string());
        } else {
            let capture = read_bundle_file(root, &captures[0].path)
                .ok()
                .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok());
            if capture
                .as_ref()
                .is_some_and(|value| valid_native_dependency_content(value, &index.tuple))
            {
                contracts.push("evidence.package.post.dependencies.unexpected_valid".to_string());
            }
        }
    }
    contracts
}

fn step_stream_lengths(index: &EvidenceIndex, gate: &str, step: &str) -> Option<(u64, u64)> {
    let length = |role: &str, suffix: &str| {
        let path = format!("{gate}/{step}.{suffix}");
        let mut matching = index
            .entries
            .iter()
            .filter(|entry| entry.role == role && entry.path == path);
        let entry = matching.next()?;
        matching.next().is_none().then_some(entry.byte_length)
    };
    Some((
        length("step.stdout", "stdout.log")?,
        length("step.stderr", "stderr.log")?,
    ))
}

fn exact_step_result_fields(value: &serde_json::Value) -> bool {
    exact_json_fields(
        value,
        &[
            "schema",
            "gate",
            "step",
            "command",
            "effective_command",
            "staged_script_sha256",
            "executable_identity",
            "exit_code",
            "signal",
            "launch_error",
            "success",
            "supervision",
        ],
    )
}

struct StepSupervision<'a> {
    timeout: u64,
    termination_grace: u64,
    pipe_drain_timeout: u64,
    stdout_limit: u64,
    stderr_limit: u64,
    stdout_seen: u64,
    stderr_seen: u64,
    stdout_truncated: bool,
    stderr_truncated: bool,
    timed_out: bool,
    reason: &'a str,
    termination_signal: &'a str,
    process_group: &'a str,
    pipe_cleanup: &'a str,
    error: &'a serde_json::Value,
}

impl<'a> StepSupervision<'a> {
    fn parse(value: &'a serde_json::Value) -> Option<Self> {
        let supervision = value.get("supervision")?;
        if !exact_json_fields(
            supervision,
            &[
                "timeout_milliseconds",
                "termination_grace_milliseconds",
                "pipe_drain_timeout_milliseconds",
                "stdout_limit_bytes",
                "stderr_limit_bytes",
                "stdout_bytes_seen",
                "stderr_bytes_seen",
                "stdout_truncated",
                "stderr_truncated",
                "timed_out",
                "termination_reason",
                "termination_signal",
                "process_group_cleanup",
                "pipe_cleanup",
                "error",
            ],
        ) {
            return None;
        }
        let unsigned = |field| supervision.get(field)?.as_u64();
        let boolean = |field| supervision.get(field)?.as_bool();
        let string = |field| supervision.get(field)?.as_str();
        Some(Self {
            timeout: unsigned("timeout_milliseconds")?,
            termination_grace: unsigned("termination_grace_milliseconds")?,
            pipe_drain_timeout: unsigned("pipe_drain_timeout_milliseconds")?,
            stdout_limit: unsigned("stdout_limit_bytes")?,
            stderr_limit: unsigned("stderr_limit_bytes")?,
            stdout_seen: unsigned("stdout_bytes_seen")?,
            stderr_seen: unsigned("stderr_bytes_seen")?,
            stdout_truncated: boolean("stdout_truncated")?,
            stderr_truncated: boolean("stderr_truncated")?,
            timed_out: boolean("timed_out")?,
            reason: string("termination_reason")?,
            termination_signal: string("termination_signal")?,
            process_group: string("process_group_cleanup")?,
            pipe_cleanup: string("pipe_cleanup")?,
            error: supervision.get("error")?,
        })
    }

    fn limits_valid(&self, expected_timeout: Option<u64>) -> bool {
        self.timeout > 0
            && expected_timeout.is_none_or(|expected| self.timeout == expected)
            && self.termination_grace > 0
            && self.pipe_drain_timeout > 0
            && self.stdout_limit > 0
            && self.stderr_limit > 0
            && expected_timeout.is_none_or(|_| {
                self.termination_grace == 1_000
                    && self.pipe_drain_timeout == 1_000
                    && self.stdout_limit == 4 * 1024 * 1024
                    && self.stderr_limit == 4 * 1024 * 1024
            })
    }

    fn stream_valid(captured: u64, seen: u64, limit: u64, truncated: bool) -> bool {
        if truncated {
            seen > captured && limit == captured
        } else {
            seen == captured && captured <= limit
        }
    }

    fn streams_valid(&self, stdout_bytes: u64, stderr_bytes: u64) -> bool {
        Self::stream_valid(
            stdout_bytes,
            self.stdout_seen,
            self.stdout_limit,
            self.stdout_truncated,
        ) && Self::stream_valid(
            stderr_bytes,
            self.stderr_seen,
            self.stderr_limit,
            self.stderr_truncated,
        )
    }

    fn launch_valid(&self, stdout_bytes: u64, stderr_bytes: u64, success: bool) -> bool {
        stdout_bytes == 0
            && stderr_bytes == 0
            && !self.timed_out
            && !self.stdout_truncated
            && !self.stderr_truncated
            && self.reason == "none"
            && self.termination_signal == "none"
            && self.process_group == "not-started"
            && self.pipe_cleanup == "not-started"
            && self.error.is_null()
            && !success
    }

    fn causal(&self, success: bool) -> bool {
        let truncated = self.stdout_truncated || self.stderr_truncated;
        let has_error = self
            .error
            .as_str()
            .is_some_and(|message| !message.is_empty());
        match self.reason {
            "none" => {
                !self.timed_out && !truncated && self.termination_signal == "none" && !has_error
            }
            "timeout" => {
                self.timed_out
                    && !truncated
                    && matches!(self.termination_signal, "term" | "kill")
                    && !has_error
            }
            "output-limit" => {
                !self.timed_out
                    && truncated
                    && matches!(self.termination_signal, "term" | "kill")
                    && !has_error
            }
            "descendant-cleanup" => {
                !self.timed_out
                    && !truncated
                    && matches!(self.termination_signal, "term" | "kill")
                    && !has_error
            }
            "supervision-error" => !success && has_error,
            _ => false,
        }
    }

    fn cleanup_valid(&self) -> bool {
        if self.error.is_string() {
            matches!(self.pipe_cleanup, "complete" | "timed-out")
        } else {
            self.pipe_cleanup == "complete"
        }
    }
}

fn valid_step_supervision(
    value: &serde_json::Value,
    stdout_bytes: u64,
    stderr_bytes: u64,
    expected_timeout_milliseconds: Option<u64>,
) -> bool {
    let Some(supervision) = StepSupervision::parse(value) else {
        return false;
    };
    if !supervision.limits_valid(expected_timeout_milliseconds)
        || !supervision.streams_valid(stdout_bytes, stderr_bytes)
    {
        return false;
    }
    let success = value.get("success").and_then(serde_json::Value::as_bool) == Some(true);
    if is_launch_failure(value) {
        return supervision.launch_valid(stdout_bytes, stderr_bytes, success);
    }
    let has_error = supervision.error.is_string();
    let truncated = supervision.stdout_truncated || supervision.stderr_truncated;
    supervision.process_group == "verified-empty"
        && supervision.causal(success)
        && supervision.cleanup_valid()
        && success
            == (spawned_exit_code(value) == Some(0)
                && supervision.reason == "none"
                && !supervision.timed_out
                && !truncated
                && !has_error)
}

fn spawned_exit_code(value: &serde_json::Value) -> Option<i64> {
    let code = value.get("exit_code")?.as_i64()?;
    (value.get("signal").is_some_and(serde_json::Value::is_null)
        && value
            .get("launch_error")
            .is_some_and(serde_json::Value::is_null)
        && (0..=255).contains(&code))
    .then_some(code)
}

fn has_spawned_signal(value: &serde_json::Value) -> bool {
    value
        .get("exit_code")
        .is_some_and(serde_json::Value::is_null)
        && value
            .get("signal")
            .and_then(serde_json::Value::as_i64)
            .is_some_and(|signal| signal > 0 && signal <= i64::from(i32::MAX))
        && value
            .get("launch_error")
            .is_some_and(serde_json::Value::is_null)
}

fn is_launch_failure(value: &serde_json::Value) -> bool {
    value
        .get("exit_code")
        .is_some_and(serde_json::Value::is_null)
        && value.get("signal").is_some_and(serde_json::Value::is_null)
        && value
            .get("launch_error")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|error| !error.is_empty())
}

fn valid_failed_step_terminal(value: &serde_json::Value) -> bool {
    value.get("success").and_then(serde_json::Value::as_bool) == Some(false)
        && (spawned_exit_code(value).is_some_and(|code| code != 0)
            || has_spawned_signal(value)
            || is_launch_failure(value))
}

fn process_exit_code(value: &serde_json::Value) -> Option<i64> {
    let code = value.get("exit_code")?.as_i64()?;
    (value.get("signal").is_some_and(serde_json::Value::is_null) && (0..=255).contains(&code))
        .then_some(code)
}

fn has_process_signal(value: &serde_json::Value) -> bool {
    value
        .get("exit_code")
        .is_some_and(serde_json::Value::is_null)
        && value
            .get("signal")
            .and_then(serde_json::Value::as_i64)
            .is_some_and(|signal| signal > 0 && signal <= i64::from(i32::MAX))
}

fn validate_builtin_step<F>(
    root: &Path,
    index: &EvidenceIndex,
    gate: &str,
    step: &str,
    accepted_exit_codes: &[i64],
    command_valid: F,
) -> Vec<String>
where
    F: Fn(&[String]) -> bool,
{
    let mut contracts = Vec::new();
    let paths = [
        ("step.stdout", format!("{gate}/{step}.stdout.log")),
        ("step.stderr", format!("{gate}/{step}.stderr.log")),
        ("step.result", format!("{gate}/{step}.result.json")),
    ];
    let matching: Vec<_> = paths
        .iter()
        .map(|(role, path)| {
            index
                .entries
                .iter()
                .filter(|entry| {
                    entry.role == *role && entry.producing_gate == gate && entry.path == *path
                })
                .collect::<Vec<_>>()
        })
        .collect();
    if matching.iter().any(|entries| entries.len() != 1) {
        contracts.push(format!("evidence.builtin.{gate}.{step}.triplet"));
        return contracts;
    }
    let command = &matching[0][0].producing_command;
    if !command_valid(command)
        || matching
            .iter()
            .any(|entries| entries[0].producing_command != *command)
    {
        contracts.push(format!("evidence.builtin.{gate}.{step}.command"));
        return contracts;
    }
    let receipt: Option<serde_json::Value> = read_bundle_file(root, &matching[2][0].path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok());
    let valid = receipt.as_ref().is_some_and(|receipt| {
        let result_valid = if accepted_exit_codes.is_empty() {
            valid_failed_step_terminal(receipt)
        } else {
            spawned_exit_code(receipt).is_some_and(|code| accepted_exit_codes.contains(&code))
                && receipt.get("success").and_then(|value| value.as_bool())
                    == spawned_exit_code(receipt).map(|code| code == 0)
        };
        exact_step_result_fields(receipt)
            && receipt.get("schema").and_then(|value| value.as_str())
                == Some("uqm-s4-step-result-v2")
            && receipt.get("gate").and_then(|value| value.as_str()) == Some(gate)
            && receipt.get("step").and_then(|value| value.as_str()) == Some(step)
            && receipt.get("command") == Some(&serde_json::json!(command))
            && valid_step_execution_provenance(receipt, command)
            && valid_step_executable_identity(receipt)
            && result_valid
            && valid_step_supervision(
                receipt,
                matching[0][0].byte_length,
                matching[1][0].byte_length,
                Some(3_600_000),
            )
    });
    if !valid {
        contracts.push(format!("evidence.builtin.{gate}.{step}.result"));
    }
    contracts
}

fn validate_builtin_payload(
    index: &EvidenceIndex,
    role: &str,
    gate: &str,

    path: &str,
    command: &[String],
    contracts: &mut Vec<String>,
) {
    let matching: Vec<_> = index
        .entries
        .iter()
        .filter(|entry| entry.role == role)
        .collect();
    if matching.len() != 1
        || matching[0].producing_gate != gate
        || matching[0].path != path
        || matching[0].producing_command != command
    {
        contracts.push(format!("evidence.builtin.{gate}.{role}.identity"));
    }
}

fn validate_complexity_command(command: &[String], authority: &Authority) -> bool {
    let prefix = std::iter::once("lizard".to_string())
        .chain(authority.complexity.lizard_arguments.iter().cloned())
        .collect::<Vec<_>>();
    let sources = command.get(prefix.len()..).unwrap_or_default();
    command.starts_with(&prefix)
        && !sources.is_empty()
        && sources.windows(2).all(|pair| pair[0] < pair[1])
        && sources.iter().all(|path| {
            validate_relative_path(path)
                && path.ends_with(".rs")
                && authority
                    .complexity
                    .source_roots
                    .iter()
                    .any(|source_root| path.starts_with(source_root))
        })
}

fn validate_bootstrap_profile(
    index: &EvidenceIndex,
    authority: &Authority,
    contracts: &mut Vec<String>,
) {
    let matching: Vec<_> = index
        .entries
        .iter()
        .filter(|entry| entry.role == "bootstrap-proof.profile")
        .collect();
    if matching.len() != 1 || matching[0].sha256 != authority.bootstrap_proof.profile_sha256 {
        contracts.push("evidence.builtin.bootstrap-proof.profile_content".to_string());
    }
}

fn validate_bootstrap_runner(root: &Path, index: &EvidenceIndex, contracts: &mut Vec<String>) {
    let matching: Vec<_> = index
        .entries
        .iter()
        .filter(|entry| entry.role == "bootstrap-proof.runner")
        .collect();
    let valid = matching.len() == 1
        && read_bundle_file(root, &matching[0].path)
            .ok()
            .is_some_and(|bytes| {
                bytes
                    .get(..4)
                    .is_some_and(|magic| match index.tuple.as_str() {
                        tuple if tuple.starts_with("linux-") => magic == b"\x7fELF",
                        tuple if tuple.starts_with("macos-") => matches!(
                            magic,
                            [0xcf, 0xfa, 0xed, 0xfe]
                                | [0xfe, 0xed, 0xfa, 0xcf]
                                | [0xca, 0xfe, 0xba, 0xbe]
                                | [0xbe, 0xba, 0xfe, 0xca]
                        ),
                        _ => false,
                    })
            });
    if !valid {
        contracts.push("evidence.builtin.bootstrap-proof.runner_content".to_string());
    }
}

fn validate_bootstrap_package(
    root: &Path,
    index: &EvidenceIndex,
    authority: &Authority,
    contracts: &mut Vec<String>,
) {
    let manifests: Vec<_> = index
        .entries
        .iter()
        .filter(|entry| entry.role == "bootstrap-proof.package-manifest")
        .collect();
    let executables: Vec<_> = index
        .entries
        .iter()
        .filter(|entry| entry.role == "bootstrap-proof.executable")
        .collect();
    if manifests.len() != 1 || executables.len() != 1 {
        return;
    }
    let manifest: Option<serde_json::Value> = read_bundle_file(root, &manifests[0].path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok());
    let Some(manifest) = manifest else {
        contracts.push("evidence.builtin.bootstrap-proof.package_content".to_string());
        return;
    };
    let artifacts = manifest.get("artifacts").and_then(|value| value.as_array());
    let packaged_executable_path = rust_target_for_tuple(&index.tuple).map(|target| {
        format!(
            "{}/{}/{}",
            authority.bootstrap_proof.packaged_root,
            target,
            authority.bootstrap_proof.packaged_executable
        )
    });
    let manifest_valid = valid_package_manifest_content(index, authority, &manifest)
        && packaged_executable_path.is_some_and(|expected_path| {
            artifacts.is_some_and(|items| {
                items
                    .iter()
                    .filter(|artifact| {
                        artifact.get("role").and_then(serde_json::Value::as_str)
                            == Some("executable")
                            && artifact.get("path").and_then(serde_json::Value::as_str)
                                == Some(expected_path.as_str())
                    })
                    .count()
                    == 1
            })
        });
    if !manifest_valid {
        contracts.push("evidence.builtin.bootstrap-proof.package_content".to_string());
    }
    for expected in &authority.package.artifacts {
        let matching: Vec<_> = artifacts
            .into_iter()
            .flatten()
            .filter(|artifact| {
                artifact.get("role").and_then(|value| value.as_str())
                    == Some(expected.role.as_str())
            })
            .collect();
        let valid = matching.len() == 1
            && matching[0]
                .get("media_type")
                .and_then(|value| value.as_str())
                == Some(expected.media_type.as_str())
            && matching[0]
                .get("producing_command")
                .and_then(|value| value.as_str())
                == Some(expected.producing_command.as_str())
            && matching[0]
                .get("path")
                .and_then(|value| value.as_str())
                .is_some_and(validate_relative_path)
            && matching[0]
                .get("sha256")
                .and_then(|value| value.as_str())
                .is_some_and(|hash| is_hex(hash, 64));
        if !valid {
            contracts.push(format!(
                "evidence.builtin.bootstrap-proof.package_artifact.{}",
                expected.role
            ));
        }
    }
    let executable_hash = artifacts
        .into_iter()
        .flatten()
        .find(|artifact| {
            artifact.get("role").and_then(|value| value.as_str()) == Some("executable")
        })
        .and_then(|artifact| artifact.get("sha256"))
        .and_then(|value| value.as_str());
    if executable_hash != Some(executables[0].sha256.as_str()) || executables[0].byte_length == 0 {
        contracts.push("evidence.builtin.bootstrap-proof.executable_content".to_string());
    }
}

struct LcarInventory {
    valid: bool,
    roles: BTreeMap<String, usize>,
    role_hashes: BTreeMap<String, String>,
    role_paths: BTreeMap<String, String>,
    role_lengths: BTreeMap<String, u64>,
}

fn validate_failure_lcar_inventory(
    root: &Path,
    artifacts: Option<&Vec<serde_json::Value>>,
    retained: &[&EvidenceEntry],
    run_command: &[String],
) -> LcarInventory {
    let mut paths = BTreeSet::new();
    let mut roles = BTreeMap::<String, usize>::new();
    let mut role_hashes = BTreeMap::<String, String>::new();
    let mut role_paths = BTreeMap::<String, String>::new();
    let mut role_lengths = BTreeMap::<String, u64>::new();
    let valid = artifacts.is_some_and(|items| {
        items.len() == retained.len()
            && !items.is_empty()
            && items.windows(2).all(|pair| {
                pair[0].get("path").and_then(|value| value.as_str())
                    < pair[1].get("path").and_then(|value| value.as_str())
            })
            && items.iter().all(|artifact| {
                if !exact_json_fields(artifact, &["role", "path", "sha256", "bytes"]) {
                    return false;
                }
                let Some(path) = artifact.get("path").and_then(|value| value.as_str()) else {
                    return false;
                };
                let Some(role) = artifact.get("role").and_then(|value| value.as_str()) else {
                    return false;
                };
                let Some(hash) = artifact.get("sha256").and_then(|value| value.as_str()) else {
                    return false;
                };
                let Some(bytes) = artifact.get("bytes").and_then(|value| value.as_u64()) else {
                    return false;
                };
                if !validate_relative_path(path)
                    || !valid_lcar_artifact_role(role, path)
                    || !paths.insert(path.to_string())
                {
                    return false;
                }
                *roles.entry(role.to_string()).or_default() += 1;
                role_hashes.insert(role.to_string(), hash.to_string());
                role_paths.insert(role.to_string(), path.to_string());
                role_lengths.insert(role.to_string(), bytes);
                let expected_path = format!("payloads/bootstrap-proof.lcar-artifact/{path}");
                let matching: Vec<_> = retained
                    .iter()
                    .filter(|entry| {
                        entry.path == expected_path
                            && entry.producing_gate == "bootstrap-proof"
                            && entry.producing_command == run_command
                    })
                    .collect();
                matching.len() == 1
                    && matching[0].sha256 == hash
                    && matching[0].byte_length == bytes
                    && (bytes > 0 || matches!(role, "stdout_log" | "stderr_log"))
                    && (role != "capture" || validate_lcar_capture(root, path))
            })
    });
    LcarInventory {
        valid,
        roles,
        role_hashes,
        role_paths,
        role_lengths,
    }
}

fn validate_success_lcar_inventory(
    root: &Path,
    artifacts: Option<&Vec<serde_json::Value>>,
    retained: &[&EvidenceEntry],
    run_command: &[String],
) -> LcarInventory {
    let mut paths = BTreeSet::new();
    let mut roles = BTreeMap::<String, usize>::new();
    let mut role_hashes = BTreeMap::<String, String>::new();
    let mut role_paths = BTreeMap::<String, String>::new();
    let mut role_lengths = BTreeMap::<String, u64>::new();
    let valid = artifacts.is_some_and(|items| {
        items.len() == retained.len()
            && !items.is_empty()
            && items.windows(2).all(|pair| {
                pair[0].get("path").and_then(|value| value.as_str())
                    < pair[1].get("path").and_then(|value| value.as_str())
            })
            && items.iter().all(|artifact| {
                if !exact_json_fields(artifact, &["role", "path", "sha256", "bytes"]) {
                    return false;
                }
                let Some(path) = artifact.get("path").and_then(|value| value.as_str()) else {
                    return false;
                };
                let Some(role) = artifact.get("role").and_then(|value| value.as_str()) else {
                    return false;
                };
                if !validate_relative_path(path)
                    || !valid_lcar_artifact_role(role, path)
                    || !paths.insert(path.to_string())
                {
                    return false;
                }
                *roles.entry(role.to_string()).or_default() += 1;
                let Some(hash) = artifact.get("sha256").and_then(|value| value.as_str()) else {
                    return false;
                };
                role_hashes.insert(role.to_string(), hash.to_string());
                role_paths.insert(role.to_string(), path.to_string());
                let Some(bytes) = artifact.get("bytes").and_then(|value| value.as_u64()) else {
                    return false;
                };
                role_lengths.insert(role.to_string(), bytes);
                let expected_path = format!("payloads/bootstrap-proof.lcar-artifact/{path}");
                let matching: Vec<_> = retained
                    .iter()
                    .filter(|entry| {
                        entry.path == expected_path
                            && entry.producing_gate == "bootstrap-proof"
                            && entry.producing_command == run_command
                    })
                    .collect();
                matching.len() == 1
                    && artifact.get("sha256").and_then(|value| value.as_str())
                        == Some(matching[0].sha256.as_str())
                    && artifact.get("bytes").and_then(|value| value.as_u64())
                        == Some(matching[0].byte_length)
                    && (matching[0].byte_length > 0 || matches!(role, "stdout_log" | "stderr_log"))
                    && (role != "capture" || validate_lcar_capture(root, path))
            })
    });
    LcarInventory {
        valid,
        roles,
        role_hashes,
        role_paths,
        role_lengths,
    }
}

fn valid_lcar_identity(
    manifest: &serde_json::Value,
    index: &EvidenceIndex,

    authority: &Authority,
    expected_failure: Option<&str>,
) -> bool {
    let environment = serde_json::json!({
        "SDL_AUDIODRIVER": "dummy",
        "SDL_VIDEODRIVER": "dummy"
    });
    let failure_matches = match expected_failure {
        Some(failure) => {
            manifest
                .get("first_failed_contract")
                .and_then(|value| value.as_str())
                == Some(failure)
        }
        None => manifest
            .get("first_failed_contract")
            .is_some_and(|value| value.is_null()),
    };
    exact_json_fields(
        manifest,
        &[
            "schema",
            "passed",
            "first_failed_contract",
            "git_head",
            "command",
            "environment",
            "target",
            "profile",
            "features",
            "renderer",
            "seed",
            "provenance",
            "process",
            "cleanup",
            "artifacts",
        ],
    ) && manifest.get("schema").and_then(|value| value.as_str()) == Some("uqm-lcar-v1")
        && manifest.get("passed").and_then(|value| value.as_bool())
            == Some(expected_failure.is_none())
        && failure_matches
        && manifest.get("git_head").and_then(|value| value.as_str())
            == Some(index.source_sha.as_str())
        && manifest.get("target").and_then(|value| value.as_str()) == Some(index.tuple.as_str())
        && manifest.get("profile").and_then(|value| value.as_str())
            == Some(authority.package.profile.as_str())
        && manifest.get("features") == Some(&serde_json::json!(authority.package.features))
        && manifest.get("renderer").and_then(|value| value.as_str()) == Some("sdl2-software-dummy")
        && manifest.get("seed").and_then(|value| value.as_u64()) == Some(0x5eed_c0de)
        && manifest.get("environment") == Some(&environment)
}

fn validate_bootstrap_lcar(
    root: &Path,
    index: &EvidenceIndex,
    authority: &Authority,
    run_command: &[String],
    contracts: &mut Vec<String>,
) {
    let manifests: Vec<_> = index
        .entries
        .iter()
        .filter(|entry| entry.role == "bootstrap-proof.lcar")
        .collect();
    if manifests.len() != 1 {
        return;
    }
    let manifest: Option<serde_json::Value> = read_bundle_file(root, &manifests[0].path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok());
    let Some(manifest) = manifest else {
        contracts.push("evidence.builtin.bootstrap-proof.lcar.content".to_string());
        return;
    };
    let identity_valid = valid_lcar_identity(&manifest, index, authority, None);
    if !identity_valid {
        contracts.push("evidence.builtin.bootstrap-proof.lcar.identity".to_string());
    }

    let package = index
        .entries
        .iter()
        .find(|entry| entry.role == "bootstrap-proof.package-manifest");
    let executable = index
        .entries
        .iter()
        .find(|entry| entry.role == "bootstrap-proof.executable");
    let profile = index
        .entries
        .iter()
        .find(|entry| entry.role == "bootstrap-proof.profile");
    let provenance = manifest.get("provenance");
    let provenance_valid = provenance.is_some_and(|value| {
        exact_json_fields(
            value,
            &[
                "production_manifest_sha256",
                "executable_sha256",
                "script_sha256",
                "content_tree_sha256",
                "initial_config_tree_sha256",
                "final_config_tree_sha256",
            ],
        ) && value
            .get("production_manifest_sha256")
            .and_then(|value| value.as_str())
            == package.map(|entry| entry.sha256.as_str())
            && value
                .get("executable_sha256")
                .and_then(|value| value.as_str())
                == executable.map(|entry| entry.sha256.as_str())
            && value.get("script_sha256").and_then(|value| value.as_str())
                == profile.map(|entry| entry.sha256.as_str())
            && [
                "content_tree_sha256",
                "initial_config_tree_sha256",
                "final_config_tree_sha256",
            ]
            .iter()
            .all(|field| {
                value
                    .get(field)
                    .and_then(|value| value.as_str())
                    .is_some_and(|hash| is_hex(hash, 64))
            })
    });
    if !provenance_valid {
        contracts.push("evidence.builtin.bootstrap-proof.lcar.provenance".to_string());
    }

    let process = manifest.get("process");
    let cleanup = manifest.get("cleanup");
    let result_valid = process.is_some_and(|value| {
        exact_json_fields(
            value,
            &[
                "pid",
                "start_time",
                "executable_sha256",
                "exit_code",
                "signal",
                "term_sent",
                "kill_sent",
                "stdout_bytes",
                "stderr_bytes",
                "output_drained",
                "orphan_check_passed",
            ],
        ) && value
            .get("executable_sha256")
            .and_then(|value| value.as_str())
            == executable.map(|entry| entry.sha256.as_str())
            && value.get("exit_code").and_then(|value| value.as_i64()) == Some(0)
            && value.get("signal").is_some_and(|value| value.is_null())
            && value
                .get("output_drained")
                .and_then(|value| value.as_bool())
                == Some(true)
            && value
                .get("orphan_check_passed")
                .and_then(|value| value.as_bool())
                == Some(true)
    }) && cleanup.is_some_and(|value| {
        exact_json_fields(
            value,
            &[
                "exact_child_reaped",
                "orphan_check_passed",
                "output_drained",
                "config_root_removed",
            ],
        ) && [
            "exact_child_reaped",
            "orphan_check_passed",
            "output_drained",
            "config_root_removed",
        ]
        .iter()
        .all(|field| value.get(field).and_then(|value| value.as_bool()) == Some(true))
    });
    if !result_valid {
        contracts.push("evidence.builtin.bootstrap-proof.lcar.result".to_string());
    }

    let artifacts = manifest.get("artifacts").and_then(|value| value.as_array());
    let retained: Vec<_> = index
        .entries
        .iter()
        .filter(|entry| entry.role == "bootstrap-proof.lcar-artifact")
        .collect();
    let LcarInventory {
        valid: inventory_valid,
        roles,
        role_hashes,
        role_paths,
        role_lengths,
    } = validate_success_lcar_inventory(root, artifacts, &retained, run_command);
    let mandatory = [
        "stdout_log",
        "stderr_log",
        "production_manifest_snapshot",
        "executable_snapshot",
        "script_snapshot",
        "content_identity_snapshot",
        "initial_config_snapshot",
        "final_config_snapshot",
        "trace",
        "teardown_receipt",
    ];
    let roles_valid = mandatory.iter().all(|role| roles.get(*role) == Some(&1))
        && roles.get("capture").is_some_and(|count| *count > 0);
    if !inventory_valid || !roles_valid {
        contracts.push("evidence.builtin.bootstrap-proof.lcar.inventory".to_string());
    }
    let snapshots_valid = role_hashes.get("production_manifest_snapshot")
        == package.map(|entry| &entry.sha256)
        && role_hashes.get("executable_snapshot") == executable.map(|entry| &entry.sha256)
        && role_hashes.get("script_snapshot") == profile.map(|entry| &entry.sha256);
    let trees_valid = [
        (
            "content_identity_snapshot",
            "content",
            "content_tree_sha256",
        ),
        (
            "initial_config_snapshot",
            "initial_config",
            "initial_config_tree_sha256",
        ),
        (
            "final_config_snapshot",
            "final_config",
            "final_config_tree_sha256",
        ),
    ]
    .iter()
    .all(|(role, root_role, field)| {
        role_paths.get(*role).is_some_and(|path| {
            provenance
                .and_then(|value| value.get(*field))
                .and_then(|value| value.as_str())
                .is_some_and(|digest| validate_lcar_tree_snapshot(root, path, root_role, digest))
        })
    });
    let config_valid = role_paths
        .get("final_config_snapshot")
        .is_some_and(|path| validate_lcar_retained_config(root, path, artifacts));
    if !snapshots_valid || !trees_valid || !config_valid {
        contracts.push("evidence.builtin.bootstrap-proof.lcar.snapshots".to_string());
    }
    if !validate_lcar_command(&manifest, &role_paths) {
        contracts.push("evidence.builtin.bootstrap-proof.lcar.command".to_string());
    }
    let process_complete = process.is_some_and(|process| {
        process
            .get("pid")
            .and_then(|value| value.as_u64())
            .is_some_and(|pid| pid > 0)
            && process
                .get("start_time")
                .and_then(|value| value.as_str())
                .is_some_and(|value| !value.is_empty())
            && process.get("term_sent").and_then(|value| value.as_bool()) == Some(false)
            && process.get("kill_sent").and_then(|value| value.as_bool()) == Some(false)
            && process.get("stdout_bytes").and_then(|value| value.as_u64())
                == role_lengths.get("stdout_log").copied()
            && process.get("stderr_bytes").and_then(|value| value.as_u64())
                == role_lengths.get("stderr_log").copied()
    });
    if !process_complete {
        contracts.push("evidence.builtin.bootstrap-proof.lcar.process_receipt".to_string());
    }
    if !role_paths.get("teardown_receipt").is_some_and(|path| {
        validate_lcar_teardown(root, path, process.and_then(|value| value.get("exit_code")))
    }) {
        contracts.push("evidence.builtin.bootstrap-proof.lcar.teardown".to_string());
    }
    if !role_paths.get("trace").is_some_and(|path| {
        validate_lcar_trace(
            root,
            path,
            role_paths.get("script_snapshot").map(String::as_str),
            artifacts,
        )
    }) {
        contracts.push("evidence.builtin.bootstrap-proof.lcar.trace".to_string());
    }
}

struct LcarFailureFacts {
    status_failed: bool,
    teardown_common_valid: bool,
    teardown_terminal_failed: bool,
}

fn valid_lcar_failure_semantics(
    root: &Path,
    failure: Option<&str>,
    process: Option<&serde_json::Value>,
    cleanup: Option<&serde_json::Value>,
    roles: &BTreeMap<String, usize>,
    role_paths: &BTreeMap<String, String>,
    facts: LcarFailureFacts,
) -> bool {
    match failure {
        Some("timeout") => process.is_some_and(|value| {
            value.get("term_sent").and_then(|value| value.as_bool()) == Some(true)
                || value.get("kill_sent").and_then(|value| value.as_bool()) == Some(true)
        }),
        Some("reader" | "budget") => cleanup.is_some_and(|value| {
            value
                .get("exact_child_reaped")
                .and_then(|value| value.as_bool())
                == Some(true)
                && value
                    .get("orphan_check_passed")
                    .and_then(|value| value.as_bool())
                    == Some(true)
        }),
        Some("nonzero_child") => facts.status_failed,
        Some("missing_teardown") => !roles.contains_key("teardown_receipt"),
        Some("semantic_evidence") => {
            roles.get("teardown_receipt") == Some(&1)
                && facts.teardown_common_valid
                && facts.teardown_terminal_failed
                && roles.get("trace") == Some(&1)
                && role_paths
                    .get("trace")
                    .is_some_and(|path| validate_lcar_failure_trace(root, path))
        }
        Some("teardown_evidence") => {
            roles.get("teardown_receipt") == Some(&1) && !facts.teardown_common_valid
        }
        Some("config_cleanup") => {
            cleanup.and_then(|value| value.get("config_root_removed"))
                == Some(&serde_json::Value::Bool(false))
                && roles
                    .get("retained_config_file")
                    .is_some_and(|count| *count > 0)
        }
        _ => false,
    }
}

fn validate_bootstrap_failure_lcar(
    root: &Path,
    index: &EvidenceIndex,
    authority: &Authority,
    run_command: &[String],
    contracts: &mut Vec<String>,
) {
    let Some(entry) = index
        .entries
        .iter()
        .find(|entry| entry.role == "bootstrap-proof.failure-lcar")
    else {
        return;
    };
    let manifest: Option<serde_json::Value> = read_bundle_file(root, &entry.path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok());
    let Some(manifest) = manifest else {
        contracts.push("evidence.builtin.bootstrap-proof.failure_lcar.content".to_string());
        return;
    };
    let failure = manifest
        .get("first_failed_contract")
        .and_then(|value| value.as_str());
    let known_failure = matches!(
        failure,
        Some(
            "timeout"
                | "reader"
                | "budget"
                | "nonzero_child"
                | "missing_teardown"
                | "semantic_evidence"
                | "teardown_evidence"
                | "config_cleanup"
        )
    );
    if !known_failure || !valid_lcar_identity(&manifest, index, authority, failure) {
        contracts.push("evidence.builtin.bootstrap-proof.failure_lcar.identity".to_string());
    }

    let package = index
        .entries
        .iter()
        .find(|entry| entry.role == "bootstrap-proof.package-manifest");
    let executable = index
        .entries
        .iter()
        .find(|entry| entry.role == "bootstrap-proof.executable");
    let profile = index
        .entries
        .iter()
        .find(|entry| entry.role == "bootstrap-proof.profile");
    let provenance = manifest.get("provenance");
    let provenance_valid = provenance.is_some_and(|value| {
        exact_json_fields(
            value,
            &[
                "production_manifest_sha256",
                "executable_sha256",
                "script_sha256",
                "content_tree_sha256",
                "initial_config_tree_sha256",
                "final_config_tree_sha256",
            ],
        ) && value
            .get("production_manifest_sha256")
            .and_then(|value| value.as_str())
            == package.map(|entry| entry.sha256.as_str())
            && value
                .get("executable_sha256")
                .and_then(|value| value.as_str())
                == executable.map(|entry| entry.sha256.as_str())
            && value.get("script_sha256").and_then(|value| value.as_str())
                == profile.map(|entry| entry.sha256.as_str())
            && [
                "content_tree_sha256",
                "initial_config_tree_sha256",
                "final_config_tree_sha256",
            ]
            .iter()
            .all(|field| {
                value
                    .get(field)
                    .and_then(|value| value.as_str())
                    .is_some_and(|hash| is_hex(hash, 64))
            })
    });
    if !provenance_valid {
        contracts.push("evidence.builtin.bootstrap-proof.failure_lcar.provenance".to_string());
    }

    let artifacts = manifest.get("artifacts").and_then(|value| value.as_array());
    let retained: Vec<_> = index
        .entries
        .iter()
        .filter(|entry| entry.role == "bootstrap-proof.lcar-artifact")
        .collect();
    let LcarInventory {
        valid: inventory_valid,
        roles,
        role_hashes,
        role_paths,
        role_lengths,
    } = validate_failure_lcar_inventory(root, artifacts, &retained, run_command);
    let mandatory = [
        "stdout_log",
        "stderr_log",
        "production_manifest_snapshot",
        "executable_snapshot",
        "script_snapshot",
        "content_identity_snapshot",
        "initial_config_snapshot",
        "final_config_snapshot",
    ];
    if !inventory_valid || !mandatory.iter().all(|role| roles.get(*role) == Some(&1)) {
        contracts.push("evidence.builtin.bootstrap-proof.failure_lcar.inventory".to_string());
    }
    let snapshots_valid = role_hashes.get("production_manifest_snapshot")
        == package.map(|entry| &entry.sha256)
        && role_hashes.get("executable_snapshot") == executable.map(|entry| &entry.sha256)
        && role_hashes.get("script_snapshot") == profile.map(|entry| &entry.sha256)
        && [
            (
                "content_identity_snapshot",
                "content",
                "content_tree_sha256",
            ),
            (
                "initial_config_snapshot",
                "initial_config",
                "initial_config_tree_sha256",
            ),
            (
                "final_config_snapshot",
                "final_config",
                "final_config_tree_sha256",
            ),
        ]
        .iter()
        .all(|(role, root_role, field)| {
            role_paths.get(*role).is_some_and(|path| {
                provenance
                    .and_then(|value| value.get(*field))
                    .and_then(|value| value.as_str())
                    .is_some_and(|digest| {
                        validate_lcar_tree_snapshot(root, path, root_role, digest)
                    })
            })
        });
    let config_valid = role_paths
        .get("final_config_snapshot")
        .is_some_and(|path| validate_lcar_retained_config(root, path, artifacts));
    if !snapshots_valid || !config_valid {
        contracts.push("evidence.builtin.bootstrap-proof.failure_lcar.snapshots".to_string());
    }
    if !validate_lcar_command(&manifest, &role_paths) {
        contracts.push("evidence.builtin.bootstrap-proof.failure_lcar.command".to_string());
    }

    let process = manifest.get("process");
    let cleanup = manifest.get("cleanup");
    let process_valid = process.is_some_and(|value| {
        exact_json_fields(
            value,
            &[
                "pid",
                "start_time",
                "executable_sha256",
                "exit_code",
                "signal",
                "term_sent",
                "kill_sent",
                "stdout_bytes",
                "stderr_bytes",
                "output_drained",
                "orphan_check_passed",
            ],
        ) && value
            .get("pid")
            .and_then(|value| value.as_u64())
            .is_some_and(|pid| pid > 0)
            && value
                .get("start_time")
                .and_then(|value| value.as_str())
                .is_some_and(|value| !value.is_empty())
            && value
                .get("executable_sha256")
                .and_then(|value| value.as_str())
                == executable.map(|entry| entry.sha256.as_str())
            && value.get("stdout_bytes").and_then(|value| value.as_u64())
                == role_lengths.get("stdout_log").copied()
            && value.get("stderr_bytes").and_then(|value| value.as_u64())
                == role_lengths.get("stderr_log").copied()
            && [
                "term_sent",
                "kill_sent",
                "output_drained",
                "orphan_check_passed",
            ]
            .iter()
            .all(|field| value.get(field).and_then(|value| value.as_bool()).is_some())
            && (process_exit_code(value).is_some() || has_process_signal(value))
    });
    let cleanup_valid = cleanup.is_some_and(|value| {
        exact_json_fields(
            value,
            &[
                "exact_child_reaped",
                "orphan_check_passed",
                "output_drained",
                "config_root_removed",
            ],
        ) && value.get("orphan_check_passed")
            == process.and_then(|value| value.get("orphan_check_passed"))
            && value.get("output_drained") == process.and_then(|value| value.get("output_drained"))
            && value
                .get("exact_child_reaped")
                .and_then(|value| value.as_bool())
                == process.map(|value| {
                    value.get("exit_code").is_some_and(|value| !value.is_null())
                        || value.get("signal").is_some_and(|value| !value.is_null())
                })
            && value
                .get("config_root_removed")
                .and_then(|value| value.as_bool())
                .is_some()
    });
    let status_failed = process.is_some_and(|value| {
        process_exit_code(value).is_some_and(|code| code != 0) || has_process_signal(value)
    });
    let teardown_receipt = role_paths
        .get("teardown_receipt")
        .and_then(|path| read_lcar_artifact(root, path).ok())
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok());
    let teardown_common_valid = teardown_receipt.as_ref().is_some_and(|receipt| {
        exact_json_fields(
            receipt,
            &[
                "schema",
                "terminal",
                "game_status",
                "process_status",
                "runtime_finalized",
                "runtime_deactivated",
                "callbacks_quiescent",
                "trace_durable",
            ],
        ) && receipt.get("schema").and_then(|value| value.as_str()) == Some("uqm-teardown-v1")
            && receipt.get("terminal").is_some_and(|value| {
                value.is_null()
                    || value.as_str().is_some_and(|terminal| {
                        matches!(
                            terminal,
                            "success"
                                | "input_timeout"
                                | "presentation_timeout"
                                | "wall_timeout"
                                | "clock_regression"
                                | "counter_overflow"
                                | "capture_mismatch"
                                | "semantic_mismatch"
                                | "trace_failure"
                                | "state_version_overflow"
                                | "capture_generation_overflow"
                                | "panic_fallback"
                                | "poisoned_mutex"
                                | "cooperative_stop"
                        )
                    })
            })
            && ["game_status", "process_status"].iter().all(|field| {
                receipt
                    .get(field)
                    .and_then(|value| value.as_i64())
                    .is_some_and(|status| i32::try_from(status).is_ok())
            })
            && receipt
                .get("process_status")
                .and_then(|value| value.as_i64())
                == process
                    .and_then(|value| value.get("exit_code"))
                    .and_then(|value| value.as_i64())
                    .or(Some(1))
            && [
                "runtime_finalized",
                "runtime_deactivated",
                "callbacks_quiescent",
                "trace_durable",
            ]
            .iter()
            .all(|field| receipt.get(field).and_then(|value| value.as_bool()) == Some(true))
    });
    let teardown_terminal_failed = teardown_receipt.as_ref().is_some_and(|receipt| {
        receipt
            .get("terminal")
            .and_then(|value| value.as_str())
            .is_some_and(|terminal| !matches!(terminal, "success" | "cooperative_stop"))
    });
    let failure_semantics = valid_lcar_failure_semantics(
        root,
        failure,
        process,
        cleanup,
        &roles,
        &role_paths,
        LcarFailureFacts {
            status_failed,
            teardown_common_valid,
            teardown_terminal_failed,
        },
    );
    if !process_valid || !cleanup_valid || !failure_semantics {
        contracts.push("evidence.builtin.bootstrap-proof.failure_lcar.result".to_string());
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum OfflineRecordKind {
    RunStart,
    RunEnd,
    InputTick,
    Presentation,
    Capture,
    MenuTransition,
    SemanticAssertion,
    SeedApplication,
    Terminal,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum OfflineSeedDomain {
    SuperMeleeMenu,
    SuperMeleeBattle,
    NewGame,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OfflineSeedApplication {
    #[allow(dead_code)]
    domain: OfflineSeedDomain,
    #[allow(dead_code)]
    seed: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OfflinePresentationEvidence {
    #[allow(dead_code)]
    count: u64,
    #[allow(dead_code)]
    generation: u64,
    #[allow(dead_code)]
    width: u32,
    #[allow(dead_code)]
    height: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OfflineActivityEvidence {
    #[allow(dead_code)]
    word: u16,
    #[allow(dead_code)]
    mask: u16,
    #[allow(dead_code)]
    equals: u16,
    #[allow(dead_code)]
    passed: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OfflineTraceRecord {
    schema: u16,
    #[allow(dead_code)]
    run: u64,
    sequence: u64,
    #[allow(dead_code)]
    input_seen: u64,
    #[allow(dead_code)]
    present_seen: u64,
    #[allow(dead_code)]
    elapsed_ms: u64,
    kind: OfflineRecordKind,
    #[allow(dead_code)]
    label: Option<String>,
    #[allow(dead_code)]
    from: Option<String>,
    #[allow(dead_code)]
    to: Option<String>,
    #[allow(dead_code)]
    terminal_reason: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    seed_application: Option<OfflineSeedApplication>,
    #[serde(default)]
    #[allow(dead_code)]
    presentation: Option<OfflinePresentationEvidence>,
    #[serde(default)]
    #[allow(dead_code)]
    activity: Option<OfflineActivityEvidence>,
}

fn validate_lcar_failure_trace(root: &Path, artifact_path: &str) -> bool {
    let Ok(bytes) = read_lcar_artifact(root, artifact_path) else {
        return false;
    };
    let Ok(text) = std::str::from_utf8(&bytes) else {
        return false;
    };
    let records: Option<Vec<OfflineTraceRecord>> = text
        .lines()
        .map(|line| serde_json::from_str(line.trim()).ok())
        .collect();
    text.ends_with('\n')
        && records.is_some_and(|records| {
            !records.is_empty()
                && records.iter().enumerate().all(|(sequence, record)| {
                    record.schema == 1 && record.sequence == sequence as u64
                })
                && records
                    .last()
                    .is_some_and(|record| matches!(record.kind, OfflineRecordKind::RunEnd))
        })
}

fn read_lcar_artifact(root: &Path, path: &str) -> std::io::Result<Vec<u8>> {
    read_bundle_file(
        root,
        &format!("payloads/bootstrap-proof.lcar-artifact/{path}"),
    )
}

fn validate_lcar_capture(root: &Path, path: &str) -> bool {
    let Ok(bytes) = read_lcar_artifact(root, path) else {
        return false;
    };
    let Ok(mut reader) = png::Decoder::new(std::io::Cursor::new(bytes)).read_info() else {
        return false;
    };
    if reader.info().width == 0
        || reader.info().height == 0
        || reader.info().animation_control.is_some()
    {
        return false;
    }
    let Some(buffer_size) = reader.output_buffer_size() else {
        return false;
    };
    let mut pixels = vec![0; buffer_size];
    reader.next_frame(&mut pixels).is_ok() && reader.finish().is_ok()
}

fn valid_lcar_artifact_role(role: &str, path: &str) -> bool {
    match path {
        "stdout.log" => role == "stdout_log",
        "stderr.log" => role == "stderr_log",
        "run/trace.jsonl" => role == "trace",
        "run/teardown-complete.json" => role == "teardown_receipt",
        "snapshots/production-manifest.json" => role == "production_manifest_snapshot",
        "snapshots/uqm" => role == "executable_snapshot",
        "snapshots/script.json" => role == "script_snapshot",
        "snapshots/content-identity.json" => role == "content_identity_snapshot",
        "snapshots/config-initial.json" => role == "initial_config_snapshot",
        "snapshots/config-final.json" => role == "final_config_snapshot",
        _ if path.starts_with("run/captures/") && path.ends_with(".png") => role == "capture",
        _ if path.starts_with("config/") => role == "retained_config_file",
        _ => false,
    }
}

fn validate_lcar_command(
    manifest: &serde_json::Value,
    role_paths: &BTreeMap<String, String>,
) -> bool {
    let Some(command) = manifest.get("command").and_then(|value| value.as_array()) else {
        return false;
    };
    let command: Option<Vec<&str>> = command.iter().map(|value| value.as_str()).collect();
    let Some(command) = command else {
        return false;
    };
    if command.len() != 8 {
        return false;
    }
    let executable = Path::new(command[0]);
    let Some(output_root) = executable.parent().and_then(Path::parent) else {
        return false;
    };
    let content = command[1].strip_prefix("--contentdir=").map(Path::new);
    executable.is_absolute()
        && role_paths.get("executable_snapshot").map(String::as_str) == Some("snapshots/uqm")
        && command[0] == output_root.join("snapshots/uqm").to_string_lossy()
        && content.is_some_and(|path| {
            path.is_absolute()
                && path.ends_with("sc2/content")
                && !path
                    .components()
                    .any(|component| component == std::path::Component::ParentDir)
        })
        && command[2] == format!("--configdir={}", output_root.join("config").display())
        && command[3]
            == format!(
                "--automation-script={}",
                output_root.join("snapshots/script.json").display()
            )
        && command[4] == format!("--automation-output={}", output_root.join("run").display())
        && command[5..] == ["--res=640x480", "--windowed", "--scroll=pc"]
}

fn validate_lcar_teardown(
    root: &Path,
    artifact_path: &str,
    process_exit: Option<&serde_json::Value>,
) -> bool {
    let receipt: Option<serde_json::Value> = read_lcar_artifact(root, artifact_path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok());
    receipt.is_some_and(|receipt| {
        exact_json_fields(
            &receipt,
            &[
                "schema",
                "terminal",
                "game_status",
                "process_status",
                "runtime_finalized",
                "runtime_deactivated",
                "callbacks_quiescent",
                "trace_durable",
            ],
        ) && receipt.get("schema").and_then(|value| value.as_str()) == Some("uqm-teardown-v1")
            && receipt.get("terminal").and_then(|value| value.as_str()) == Some("success")
            && receipt.get("game_status").and_then(|value| value.as_i64()) == Some(0)
            && receipt
                .get("process_status")
                .and_then(|value| value.as_i64())
                == Some(0)
            && process_exit.and_then(|value| value.as_i64()) == Some(0)
            && [
                "runtime_finalized",
                "runtime_deactivated",
                "callbacks_quiescent",
                "trace_durable",
            ]
            .iter()
            .all(|field| receipt.get(*field).and_then(|value| value.as_bool()) == Some(true))
    })
}

fn validate_lcar_trace(
    root: &Path,
    artifact_path: &str,
    script_path: Option<&str>,
    artifacts: Option<&Vec<serde_json::Value>>,
) -> bool {
    let Ok(bytes) = read_lcar_artifact(root, artifact_path) else {
        return false;
    };
    let Ok(text) = std::str::from_utf8(&bytes) else {
        return false;
    };
    let records: Option<Vec<serde_json::Value>> = text
        .lines()
        .map(|line| serde_json::from_str(line).ok())
        .collect();
    let Some(records) = records else {
        return false;
    };
    if records
        .first()
        .and_then(|value| value.get("kind"))
        .and_then(|value| value.as_str())
        != Some("run_start")
        || records
            .last()
            .and_then(|value| value.get("kind"))
            .and_then(|value| value.as_str())
            != Some("run_end")
    {
        return false;
    }
    let mut presentations = 0_usize;
    let mut semantics = 0_usize;
    let mut traced_captures = BTreeSet::new();
    let mut ordered_capture_paths = Vec::new();
    for (sequence, record) in records.iter().enumerate() {
        if record.get("schema").and_then(|value| value.as_u64()) != Some(1)
            || record.get("run").and_then(|value| value.as_u64()) != Some(1)
            || record.get("sequence").and_then(|value| value.as_u64()) != Some(sequence as u64)
        {
            return false;
        }
        let kind = record.get("kind").and_then(|value| value.as_str());
        if matches!(kind, Some("presentation" | "capture")) {
            let presentation = record.get("presentation");
            let present_seen = record.get("present_seen").and_then(|value| value.as_u64());
            if !presentation.is_some_and(|value| {
                value.get("count").and_then(|value| value.as_u64()) == present_seen
                    && present_seen.is_some_and(|count| count > 0)
                    && value
                        .get("width")
                        .and_then(|value| value.as_u64())
                        .is_some_and(|width| width > 0)
                    && value
                        .get("height")
                        .and_then(|value| value.as_u64())
                        .is_some_and(|height| height > 0)
            }) {
                return false;
            }
            presentations += 1;
        }
        if kind == Some("semantic_assertion") {
            semantics += 1;
            let activity_valid = record.get("activity").is_some_and(|activity| {
                let word = activity.get("word").and_then(|value| value.as_u64());
                let mask = activity.get("mask").and_then(|value| value.as_u64());
                let equals = activity.get("equals").and_then(|value| value.as_u64());
                word.zip(mask)
                    .zip(equals)
                    .is_some_and(|((word, mask), equals)| word & mask == equals)
                    && activity.get("passed").and_then(|value| value.as_bool()) == Some(true)
            });
            let label_valid = record
                .get("label")
                .and_then(|value| value.as_str())
                .is_some_and(|label| {
                    !["failed", "mismatch", "error"]
                        .iter()
                        .any(|word| label.contains(word))
                });
            if !activity_valid && !label_valid {
                return false;
            }
        }
        if kind == Some("capture") {
            let Some(label) = record.get("label").and_then(|value| value.as_str()) else {
                return false;
            };
            let Some((base, generation)) = label.rsplit_once("_gen") else {
                return false;
            };
            let capture_path = format!("run/captures/{base}.png");
            if base.is_empty()
                || generation
                    .parse::<u64>()
                    .ok()
                    .is_none_or(|generation| generation == 0)
                || !traced_captures.insert(capture_path.clone())
            {
                return false;
            }
            ordered_capture_paths.push(capture_path);
        }
    }
    let actual_captures: BTreeSet<String> = artifacts
        .into_iter()
        .flatten()
        .filter(|artifact| artifact.get("role").and_then(|value| value.as_str()) == Some("capture"))
        .filter_map(|artifact| {
            artifact
                .get("path")
                .and_then(|value| value.as_str())
                .map(str::to_string)
        })
        .collect();
    presentations > 0
        && semantics > 0
        && !traced_captures.is_empty()
        && traced_captures == actual_captures
        && validate_lcar_capture_changes(root, script_path, &ordered_capture_paths)
}

fn validate_lcar_capture_changes(
    root: &Path,
    script_path: Option<&str>,
    ordered_capture_paths: &[String],
) -> bool {
    let Some(script_path) = script_path else {
        return false;
    };
    let script: Option<serde_json::Value> = read_lcar_artifact(root, script_path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok());
    let Some(steps) = script
        .as_ref()
        .and_then(|value| value.get("steps"))
        .and_then(|value| value.as_array())
    else {
        return false;
    };
    let expecting: BTreeSet<&str> = steps
        .iter()
        .filter(|step| step.get("expect_change").and_then(|value| value.as_bool()) == Some(true))
        .filter_map(|step| step.get("label").and_then(|value| value.as_str()))
        .collect();
    let mut previous: Option<(&str, String)> = None;
    for path in ordered_capture_paths {
        let Some(label) = path
            .strip_prefix("run/captures/")
            .and_then(|path| path.strip_suffix(".png"))
        else {
            return false;
        };
        let Ok(bytes) = read_lcar_artifact(root, path) else {
            return false;
        };
        let digest = hex_sha256(&bytes);
        if expecting.contains(label)
            && previous
                .as_ref()
                .is_some_and(|(_, previous_digest)| previous_digest == &digest)
        {
            return false;
        }
        previous = Some((label, digest));
    }
    true
}
fn validate_lcar_retained_config(
    root: &Path,
    snapshot_path: &str,
    artifacts: Option<&Vec<serde_json::Value>>,
) -> bool {
    let snapshot: Option<serde_json::Value> = read_lcar_artifact(root, snapshot_path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok());
    let expected: Option<BTreeMap<String, (String, u64)>> = snapshot
        .as_ref()
        .and_then(|value| value.get("entries"))
        .and_then(|value| value.as_array())
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| {
                    Some((
                        format!("config/{}", entry.get("path")?.as_str()?),
                        (
                            entry.get("sha256")?.as_str()?.to_string(),
                            entry.get("bytes")?.as_u64()?,
                        ),
                    ))
                })
                .collect()
        });
    let actual: Option<BTreeMap<String, (String, u64)>> = artifacts.map(|entries| {
        entries
            .iter()
            .filter(|entry| {
                entry.get("role").and_then(|value| value.as_str()) == Some("retained_config_file")
            })
            .filter_map(|entry| {
                Some((
                    entry.get("path")?.as_str()?.to_string(),
                    (
                        entry.get("sha256")?.as_str()?.to_string(),
                        entry.get("bytes")?.as_u64()?,
                    ),
                ))
            })
            .collect()
    });
    expected.is_some() && expected == actual
}

fn validate_lcar_tree_snapshot(
    root: &Path,
    artifact_path: &str,
    expected_role: &str,
    expected_digest: &str,
) -> bool {
    let snapshot: Option<serde_json::Value> = read_lcar_artifact(root, artifact_path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok());
    let Some(snapshot) = snapshot else {
        return false;
    };
    if !exact_json_fields(
        &snapshot,
        &["schema", "root_role", "tree_sha256", "entries"],
    ) {
        return false;
    }
    let Some(entries) = snapshot.get("entries").and_then(|value| value.as_array()) else {
        return false;
    };
    let mut paths = BTreeSet::new();
    let mut previous_path: Option<&str> = None;
    let mut hasher = Sha256::new();
    let entries_valid = entries.iter().all(|entry| {
        if !exact_json_fields(entry, &["path", "sha256", "bytes"]) {
            return false;
        }
        let Some(path) = entry.get("path").and_then(|value| value.as_str()) else {
            return false;
        };
        let Some(hash) = entry.get("sha256").and_then(|value| value.as_str()) else {
            return false;
        };
        let Some(bytes) = entry.get("bytes").and_then(|value| value.as_u64()) else {
            return false;
        };
        if !validate_relative_path(path)
            || !paths.insert(path)
            || previous_path.is_some_and(|previous| previous >= path)
            || !is_hex(hash, 64)
        {
            return false;
        }
        previous_path = Some(path);
        hasher.update(path.as_bytes());
        hasher.update([0]);
        hasher.update(hash.as_bytes());
        hasher.update([0]);
        hasher.update(bytes.to_string().as_bytes());
        hasher.update(b"\n");
        true
    });
    snapshot.get("schema").and_then(|value| value.as_str()) == Some("uqm-tree-identity-v1")
        && snapshot.get("root_role").and_then(|value| value.as_str()) == Some(expected_role)
        && snapshot.get("tree_sha256").and_then(|value| value.as_str()) == Some(expected_digest)
        && entries_valid
        && format!("{:x}", hasher.finalize()) == expected_digest
}

fn validate_coverage_command(command: &[String], authority: &Authority) -> bool {
    let features = authority.profiles.linked_test.join(",");
    command.len() == 14
        && command[0..9]
            == [
                "cargo",
                "llvm-cov",
                "--manifest-path",
                "rust/Cargo.toml",
                "--workspace",
                "--all-targets",
                "--no-default-features",
                "--features",
                features.as_str(),
            ]
        && command[9] == "--lcov"
        && command[10] == "--output-path"
        && Path::new(&command[11]).is_absolute()
        && Path::new(&command[11]).ends_with("coverage.lcov")
        && command[12] == "--ignore-filename-regex"
        && command[13] == authority.coverage.ignore_filename_regex
}

fn validate_failed_bootstrap_payloads(
    root: &Path,
    index: &EvidenceIndex,
    authority: &Authority,
    first_failed: &str,
    contracts: &mut Vec<String>,
) {
    let base_count = usize::from(!matches!(
        first_failed,
        "bootstrap-proof.authority"
            | "bootstrap-proof.profile"
            | "bootstrap-proof.target"
            | "bootstrap-proof.package"
    ));
    let runner_count = usize::from(matches!(
        first_failed,
        "bootstrap-proof.output"
            | "bootstrap-proof.run"
            | "bootstrap-proof.failure-retain"
            | "bootstrap-proof.validate"
    ));
    let lcar_count = usize::from(first_failed == "bootstrap-proof.validate");
    let failure_lcar_count = index
        .entries
        .iter()
        .filter(|entry| entry.role == "bootstrap-proof.failure-lcar")
        .count();
    let manifest_role = if lcar_count == 1 {
        Some("bootstrap-proof.lcar")
    } else if first_failed == "bootstrap-proof.run" && failure_lcar_count == 1 {
        Some("bootstrap-proof.failure-lcar")
    } else {
        None
    };
    let lcar_artifact_count = manifest_role
        .and_then(|role| index.entries.iter().find(|entry| entry.role == role))
        .and_then(|entry| read_bundle_file(root, &entry.path).ok())
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .and_then(|manifest| {
            manifest
                .get("artifacts")
                .and_then(|value| value.as_array())
                .map(Vec::len)
        })
        .unwrap_or(0);
    let expected_count =
        base_count * 3 + runner_count + lcar_count + failure_lcar_count + lcar_artifact_count;
    let roles: Vec<_> = index
        .entries
        .iter()
        .filter(|entry| entry.role.starts_with("bootstrap-proof."))
        .filter(|entry| {
            !matches!(
                entry.role.as_str(),
                "step.stdout" | "step.stderr" | "step.result"
            )
        })
        .collect();
    if roles.len() != expected_count {
        contracts.push("evidence.builtin.bootstrap-proof.failed_payload_count".to_string());
    }
    let package_command: Vec<String> = vec![
        "cargo".into(),
        "run".into(),
        "--locked".into(),
        "--manifest-path".into(),
        "rust/xtask/Cargo.toml".into(),
        "--".into(),
        "package".into(),
    ];
    if base_count == 1 {
        for (role, file) in [
            (
                "bootstrap-proof.package-manifest",
                authority.bootstrap_proof.packaged_manifest.as_str(),
            ),
            (
                "bootstrap-proof.executable",
                authority.bootstrap_proof.packaged_executable.as_str(),
            ),
            (
                "bootstrap-proof.profile",
                Path::new(&authority.bootstrap_proof.profile)
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or_default(),
            ),
        ] {
            validate_builtin_payload(
                index,
                role,
                "bootstrap-proof",
                &format!("payloads/{role}/{file}"),
                &package_command,
                contracts,
            );
        }
        validate_bootstrap_profile(index, authority, contracts);
        validate_bootstrap_package(root, index, authority, contracts);
    }
    if runner_count == 1 {
        let build: Vec<String> = vec![
            "cargo".into(),
            "build".into(),
            "--locked".into(),
            "--manifest-path".into(),
            "rust/Cargo.toml".into(),
            "--bin".into(),
            "uqm-gameplay-proof".into(),
        ];
        validate_builtin_payload(
            index,
            "bootstrap-proof.runner",
            "bootstrap-proof",
            "payloads/bootstrap-proof.runner/uqm-gameplay-proof",
            &build,
            contracts,
        );
    }
    if lcar_count == 1 {
        if let Some(command) = builtin_step_command(index, "bootstrap-proof", "run") {
            validate_builtin_payload(
                index,
                "bootstrap-proof.lcar",
                "bootstrap-proof",
                "payloads/bootstrap-proof.lcar/lcar-v1.json",
                command,
                contracts,
            );
            validate_bootstrap_lcar(root, index, authority, command, contracts);
        } else {
            contracts.push("evidence.builtin.bootstrap-proof.failed_lcar_command".to_string());
        }
    }
    if first_failed == "bootstrap-proof.run" {
        let failure_reported = index
            .entries
            .iter()
            .find(|entry| {
                entry.role == "step.stderr"
                    && entry.producing_gate == "bootstrap-proof"
                    && entry.path == "bootstrap-proof/run.stderr.log"
            })
            .and_then(|entry| read_bundle_file(root, &entry.path).ok())
            .is_some_and(|bytes| String::from_utf8_lossy(&bytes).contains("failure-lcar-v1.json"));
        if failure_reported != (failure_lcar_count == 1) {
            contracts.push("evidence.builtin.bootstrap-proof.failure_lcar.presence".to_string());
        }
        if failure_lcar_count == 1 {
            if let Some(command) = builtin_step_command(index, "bootstrap-proof", "run") {
                validate_builtin_payload(
                    index,
                    "bootstrap-proof.failure-lcar",
                    "bootstrap-proof",
                    "payloads/bootstrap-proof.failure-lcar/failure-lcar-v1.json",
                    command,
                    contracts,
                );
                validate_bootstrap_failure_lcar(root, index, authority, command, contracts);
            } else {
                contracts.push("evidence.builtin.bootstrap-proof.failed_lcar_command".to_string());
            }
        } else if failure_lcar_count > 1 {
            contracts.push("evidence.builtin.bootstrap-proof.failure_lcar.count".to_string());
        }
    } else if failure_lcar_count != 0 {
        contracts.push("evidence.builtin.bootstrap-proof.failure_lcar.unexpected".to_string());
    }
}

fn validate_failed_bootstrap_gate(
    root: &Path,
    index: &EvidenceIndex,
    authority: &Authority,
    first_failed: &str,
) -> Vec<String> {
    let mut contracts = Vec::new();
    validate_failed_bootstrap_payloads(root, index, authority, first_failed, &mut contracts);
    let build: Vec<String> = vec![
        "cargo".into(),
        "build".into(),
        "--locked".into(),
        "--manifest-path".into(),
        "rust/Cargo.toml".into(),
        "--bin".into(),
        "uqm-gameplay-proof".into(),
    ];
    let steps = ["build-runner", "run", "validate"];
    let failed_position = match first_failed {
        "bootstrap-proof.build" => Some(0),
        "bootstrap-proof.run" | "bootstrap-proof.failure-retain" => Some(1),
        "bootstrap-proof.validate" => Some(2),
        _ => None,
    };
    let actual = index
        .entries
        .iter()
        .filter(|entry| {
            entry.producing_gate == "bootstrap-proof"
                && matches!(
                    entry.role.as_str(),
                    "step.stdout" | "step.stderr" | "step.result"
                )
        })
        .count();
    let Some(failed_position) = failed_position else {
        let recorded_positions: &[usize] = match first_failed {
            "bootstrap-proof.authority"
            | "bootstrap-proof.profile"
            | "bootstrap-proof.target"
            | "bootstrap-proof.package" => &[],
            "bootstrap-proof.output" => &[0],
            _ => {
                contracts.push(format!(
                    "evidence.builtin.bootstrap-proof.unsupported_failure.{first_failed}"
                ));
                return contracts;
            }
        };
        if actual != recorded_positions.len() * 3 {
            contracts.push("evidence.builtin.bootstrap-proof.pre_step_receipts".to_string());
        }
        for &position in recorded_positions {
            let step = steps[position];
            contracts.extend(validate_builtin_step(
                root,
                index,
                "bootstrap-proof",
                step,
                &[0],
                |command| match step {
                    "build-runner" => command == build,
                    _ => false,
                },
            ));
        }
        return contracts;
    };
    let recorded_steps = &steps[..=failed_position];
    if actual != recorded_steps.len() * 3 {
        contracts.push("evidence.builtin.bootstrap-proof.failed_step_entry_count".to_string());
    }
    for (position, step) in recorded_steps.iter().enumerate() {
        let accepted: &[i64] = if position == failed_position {
            &[]
        } else {
            &[0]
        };
        let predicate = |command: &[String]| match *step {
            "build-runner" => command == build,
            "run" => validate_bootstrap_run_command(command, authority),
            "validate" => validate_bootstrap_validate_command(command),
            _ => false,
        };
        contracts.extend(validate_builtin_step(
            root,
            index,
            "bootstrap-proof",
            step,
            accepted,
            predicate,
        ));
    }
    contracts
}

fn builtin_subordinate_count(index: &EvidenceIndex, gate: &str) -> usize {
    index
        .entries
        .iter()
        .filter(|entry| {
            entry.producing_gate == gate
                && matches!(
                    entry.role.as_str(),
                    "step.stdout" | "step.stderr" | "step.result"
                )
        })
        .count()
}

fn validate_failed_builtin_gate(
    root: &Path,
    index: &EvidenceIndex,
    authority: &Authority,
    gate: &str,
    first_failed: &str,
) -> Vec<String> {
    let mut contracts = Vec::new();
    let allowed_supplemental: &[&str] = match (gate, first_failed) {
        ("coverage", "coverage.parse" | "coverage.floor") => &["coverage.lcov"],
        ("workflow", "workflow") => &["workflow.validation"],
        ("mutations", "mutations") => &["mutations.receipt", "mutation.fixture"],
        ("bootstrap-proof", _) => &[
            "bootstrap-proof.package-manifest",
            "bootstrap-proof.executable",
            "bootstrap-proof.profile",
            "bootstrap-proof.runner",
            "bootstrap-proof.lcar",
            "bootstrap-proof.failure-lcar",
            "bootstrap-proof.lcar-artifact",
        ],
        _ => &[],
    };
    for entry in index.entries.iter().filter(|entry| {
        entry.producing_gate == gate
            && !matches!(
                entry.role.as_str(),
                "authority.snapshot"
                    | "preflight.source"
                    | "preflight.tools"
                    | "cache.initial-state"
                    | "ownership.zero-native-delta"
                    | "gate.result"
                    | "step.stdout"
                    | "step.stderr"
                    | "step.result"
            )
            && !allowed_supplemental.contains(&entry.role.as_str())
    }) {
        contracts.push(format!(
            "evidence.builtin.{gate}.unexpected_failure_role.{}",
            entry.role
        ));
    }
    match gate {
        "complexity" if first_failed == "complexity.sources" => {
            if builtin_subordinate_count(index, gate) != 0 {
                contracts.push("evidence.builtin.complexity.source_receipts".to_string());
            }
        }
        "complexity" if first_failed == "complexity.exec" => {
            contracts.extend(validate_builtin_step(
                root,
                index,
                gate,
                "lizard",
                &[],
                |command| validate_complexity_command(command, authority),
            ));
        }
        "complexity" if first_failed == "complexity.maximum" => {
            contracts.extend(validate_builtin_step(
                root,
                index,
                gate,
                "lizard",
                &[],
                |command| validate_complexity_command(command, authority),
            ));
        }
        "coverage" if matches!(first_failed, "coverage.write" | "coverage.toolchain") => {
            if builtin_subordinate_count(index, gate) != 0 {
                contracts.push("evidence.builtin.coverage.pre_step_receipts".to_string());
            }
        }
        "coverage" if first_failed == "coverage.exec" => {
            contracts.extend(validate_builtin_step(
                root,
                index,
                gate,
                "llvm-cov",
                &[],
                |command| validate_coverage_command(command, authority),
            ));
        }
        "coverage" if matches!(first_failed, "coverage.read" | "coverage.parse") => {
            contracts.extend(validate_builtin_step(
                root,
                index,
                gate,
                "llvm-cov",
                &[0],
                |command| validate_coverage_command(command, authority),
            ));
            let payload = index
                .entries
                .iter()
                .find(|entry| entry.role == "coverage.lcov");
            if first_failed == "coverage.read" {
                if payload.is_some() {
                    contracts.push("evidence.builtin.coverage.read_payload".to_string());
                }
            } else if let (Some(payload), Some(command)) =
                (payload, builtin_step_command(index, gate, "llvm-cov"))
            {
                validate_builtin_payload(
                    index,
                    "coverage.lcov",
                    gate,
                    "payloads/coverage.lcov/coverage.lcov",
                    command,
                    &mut contracts,
                );
                let parse_failed = read_bundle_file(root, &payload.path)
                    .ok()
                    .is_some_and(|bytes| super::run::lcov_line_coverage(&bytes).is_err());
                if !parse_failed {
                    contracts.push("evidence.builtin.coverage.failed_parse".to_string());
                }
            } else {
                contracts.push("evidence.builtin.coverage.failed_parse_payload".to_string());
            }
        }
        "coverage" if first_failed == "coverage.floor" => {
            contracts.extend(validate_builtin_step(
                root,
                index,
                gate,
                "llvm-cov",
                &[0],
                |command| validate_coverage_command(command, authority),
            ));
            if let Some(command) = builtin_step_command(index, gate, "llvm-cov") {
                validate_builtin_payload(
                    index,
                    "coverage.lcov",
                    gate,
                    "payloads/coverage.lcov/coverage.lcov",
                    command,
                    &mut contracts,
                );
                let below_floor = index
                    .entries
                    .iter()
                    .find(|entry| entry.role == "coverage.lcov")
                    .and_then(|entry| read_bundle_file(root, &entry.path).ok())
                    .and_then(|bytes| super::run::lcov_line_coverage(&bytes).ok())
                    .is_some_and(|percent| percent < authority.coverage.minimum_line_percent);
                if !below_floor {
                    contracts.push("evidence.builtin.coverage.failed_floor".to_string());
                }
            }
        }
        "workflow" if first_failed == "workflow.actionlint" => {
            contracts.extend(validate_builtin_step(
                root,
                index,
                gate,
                "actionlint",
                &[],
                |command| command == ["actionlint"],
            ));
            if index
                .entries
                .iter()
                .any(|entry| entry.role == "workflow.validation")
            {
                contracts.push("evidence.builtin.workflow.spawn_validation".to_string());
            }
        }
        "workflow" if first_failed == "workflow" => {
            let command = vec!["actionlint".to_string()];
            let actionlint_passed = collection_item_result(
                root,
                index,
                "workflow.validation",
                "rules",
                "workflow.actionlint",
                "rule",
                "passed",
            );
            contracts.extend(validate_builtin_step(
                root,
                index,
                gate,
                "actionlint",
                if actionlint_passed == Some(true) {
                    &[0]
                } else {
                    &[]
                },
                |actual| actual == command,
            ));
            let producer = gate_specific_command(index, gate);
            validate_builtin_payload(
                index,
                "workflow.validation",
                gate,
                "workflow/workflow-validation.json",
                &producer,
                &mut contracts,
            );
            contracts.extend(validate_failed_collection_receipt(
                root,
                index,
                authority,
                "workflow.validation",
            ));
        }
        "mutations" if first_failed == "mutations" => {
            let producer = gate_specific_command(index, gate);
            validate_builtin_payload(
                index,
                "mutations.receipt",
                gate,
                "mutations/mutations-receipt.json",
                &producer,
                &mut contracts,
            );
            contracts.extend(validate_failed_collection_receipt(
                root,
                index,
                authority,
                "mutations.receipt",
            ));
            contracts.extend(validate_mutation_fixtures(root, index, authority));
        }
        "bootstrap-proof" => contracts.extend(validate_failed_bootstrap_gate(
            root,
            index,
            authority,
            first_failed,
        )),
        _ => contracts.push(format!(
            "evidence.builtin.{gate}.unsupported_failure.{first_failed}"
        )),
    }
    contracts
}

fn collection_item_result(
    root: &Path,
    index: &EvidenceIndex,
    role: &str,
    collection: &str,
    identity: &str,
    identity_field: &str,
    result_field: &str,
) -> Option<bool> {
    index
        .entries
        .iter()
        .find(|entry| entry.role == role)
        .and_then(|entry| read_bundle_file(root, &entry.path).ok())
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .and_then(|receipt| receipt.get(collection).cloned())
        .and_then(|value| value.as_array().cloned())
        .and_then(|items| {
            items.into_iter().find(|item| {
                item.get(identity_field).and_then(|value| value.as_str()) == Some(identity)
            })
        })
        .and_then(|item| item.get(result_field).and_then(|value| value.as_bool()))
}

fn gate_specific_command(index: &EvidenceIndex, gate: &str) -> Vec<String> {
    vec![
        index
            .entries
            .iter()
            .find(|entry| entry.role == "authority.snapshot")
            .and_then(|entry| entry.producing_command.first())
            .cloned()
            .unwrap_or_default(),
        "ci".into(),
        "run".into(),
        gate.into(),
    ]
}

fn rust_target_for_tuple(tuple: &str) -> Option<&'static str> {
    match tuple {
        "macos-aarch64" => Some("aarch64-apple-darwin"),
        "macos-x86_64" => Some("x86_64-apple-darwin"),
        "linux-aarch64" => Some("aarch64-unknown-linux-gnu"),
        "linux-x86_64" => Some("x86_64-unknown-linux-gnu"),
        _ => None,
    }
}
fn validate_package_tool_identity(value: &serde_json::Value) -> bool {
    exact_json_fields(
        value,
        &["executable", "version", "sha256", "effective_args"],
    ) && value
        .get("executable")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|text| !text.is_empty())
        && value
            .get("version")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|text| !text.is_empty())
        && value
            .get("sha256")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|hash| is_hex(hash, 64))
        && value
            .get("effective_args")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|args| args.iter().all(serde_json::Value::is_string))
}

fn validate_package_toolchain(value: Option<&serde_json::Value>, target: &str) -> bool {
    let Some(value) = value else {
        return false;
    };
    const TOOLS: [&str; 7] = ["rustc", "cargo", "cc", "ar", "nm", "pkg_config", "linker"];
    exact_json_fields(
        value,
        &[
            "target",
            "rustc",
            "cargo",
            "cc",
            "ar",
            "nm",
            "pkg_config",
            "linker",
        ],
    ) && value.get("target").and_then(serde_json::Value::as_str) == Some(target)
        && TOOLS
            .iter()
            .all(|tool| value.get(*tool).is_some_and(validate_package_tool_identity))
}

fn json_string_array(value: Option<&serde_json::Value>) -> bool {
    value
        .and_then(serde_json::Value::as_array)
        .is_some_and(|items| items.iter().all(serde_json::Value::is_string))
}

fn validate_native_build_shape(
    value: Option<&serde_json::Value>,
    tuple: &str,
    rust_target: &str,
    epoch: Option<&serde_json::Value>,
    features: &[String],
) -> bool {
    let Some(native) = value else {
        return false;
    };
    let packages = native.get("packages").and_then(serde_json::Value::as_array);
    let compile = native.get("compile_profile");
    let environment = native
        .get("build_environment")
        .and_then(serde_json::Value::as_object);
    exact_json_fields(
        native,
        &[
            "schema",
            "source_date_epoch",
            "build_date",
            "target",
            "active_features",
            "toolchain",
            "packages",
            "compile_profile",
            "build_environment",
        ],
    ) && native.get("schema").and_then(serde_json::Value::as_str)
        == Some("uqm-native-build-evidence-v1")
        && native.get("source_date_epoch") == epoch
        && native
            .get("build_date")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|text| !text.is_empty())
        && native.get("target").and_then(serde_json::Value::as_str) == Some(tuple)
        && native.get("active_features") == Some(&serde_json::json!(features))
        && validate_package_toolchain(native.get("toolchain"), rust_target)
        && packages.is_some_and(|items| {
            !items.is_empty()
                && items.iter().all(|package| {
                    exact_json_fields(package, &["name", "version", "cflags", "libs"])
                        && package
                            .get("name")
                            .and_then(serde_json::Value::as_str)
                            .is_some_and(|text| !text.is_empty())
                        && package
                            .get("version")
                            .and_then(serde_json::Value::as_str)
                            .is_some_and(|text| !text.is_empty())
                        && json_string_array(package.get("cflags"))
                        && json_string_array(package.get("libs"))
                })
        })
        && compile.is_some_and(|profile| {
            exact_json_fields(
                profile,
                &[
                    "target",
                    "compiler",
                    "ordered_defines",
                    "ordered_include_roots",
                    "ordered_compile_flags",
                    "dependency_flags",
                    "command_template",
                ],
            ) && profile.get("target").and_then(serde_json::Value::as_str) == Some(tuple)
                && profile
                    .get("compiler")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|text| !text.is_empty())
                && [
                    "ordered_defines",
                    "ordered_include_roots",
                    "ordered_compile_flags",
                    "dependency_flags",
                    "command_template",
                ]
                .iter()
                .all(|field| json_string_array(profile.get(*field)))
        })
        && environment.is_some_and(|values| values.values().all(serde_json::Value::is_string))
}

fn validate_package_feature_graph(
    value: Option<&serde_json::Value>,
    expected_root_features: &[String],
) -> bool {
    let Some(packages) = value.and_then(serde_json::Value::as_array) else {
        return false;
    };
    if packages.is_empty() {
        return false;
    }
    let mut previous: Option<(&str, &str)> = None;
    let mut root_package_count = 0;
    for package in packages {
        if !exact_json_fields(package, &["name", "version", "features"]) {
            return false;
        }
        let Some(name) = package.get("name").and_then(serde_json::Value::as_str) else {
            return false;
        };
        let Some(version) = package.get("version").and_then(serde_json::Value::as_str) else {
            return false;
        };
        let Some(features) = package
            .get("features")
            .and_then(serde_json::Value::as_array)
        else {
            return false;
        };
        if name.is_empty()
            || version.is_empty()
            || previous.is_some_and(|prior| prior >= (name, version))
            || features
                .iter()
                .any(|feature| feature.as_str().is_none_or(|feature| feature.is_empty()))
            || features.windows(2).any(|pair| {
                pair[0].as_str().unwrap_or_default() >= pair[1].as_str().unwrap_or_default()
            })
        {
            return false;
        }
        if name == "uqm" {
            root_package_count += 1;
            if features.len() != expected_root_features.len()
                || features
                    .iter()
                    .zip(expected_root_features)
                    .any(|(actual, expected)| actual.as_str() != Some(expected.as_str()))
            {
                return false;
            }
        }
        previous = Some((name, version));
    }
    root_package_count == 1
}

fn valid_package_determinism_proof(manifest: &serde_json::Value) -> bool {
    let Some(artifacts) = manifest
        .get("artifacts")
        .and_then(serde_json::Value::as_array)
    else {
        return false;
    };
    let expected_digests = serde_json::Value::Array(
        artifacts
            .iter()
            .map(|artifact| {
                serde_json::json!({
                    "role": artifact.get("role"),
                    "byte_length": artifact.get("byte_length"),
                    "sha256": artifact.get("sha256"),
                })
            })
            .collect(),
    );
    let expected_identity = serde_json::json!({
        "git_head": manifest.get("git_head"),
        "tracked_worktree": manifest.get("tracked_worktree"),
        "dirty": manifest.get("dirty"),
        "source_date_epoch": manifest.get("source_date_epoch"),
        "toolchain": manifest.get("toolchain"),
        "native_build": manifest.get("native_build"),
    });
    manifest.get("determinism_proof").is_some_and(|proof| {
        exact_json_fields(
            proof,
            &[
                "command",
                "clean_builds",
                "comparison",
                "first_build",
                "second_build",
                "first_identity",
                "second_identity",
            ],
        ) && proof.get("command").and_then(serde_json::Value::as_str) == Some(PACKAGE_PROOF_COMMAND)
            && proof
                .get("clean_builds")
                .and_then(serde_json::Value::as_u64)
                == Some(PACKAGE_PROOF_CLEAN_BUILDS)
            && proof.get("comparison").and_then(serde_json::Value::as_str)
                == Some(PACKAGE_PROOF_COMPARISON)
            && proof.get("first_build") == Some(&expected_digests)
            && proof.get("second_build") == Some(&expected_digests)
            && proof.get("first_identity") == Some(&expected_identity)
            && proof.get("second_identity") == Some(&expected_identity)
    })
}

fn valid_package_manifest_content(
    index: &EvidenceIndex,
    authority: &Authority,
    manifest: &serde_json::Value,
) -> bool {
    exact_json_fields(
        manifest,
        &[
            "schema",
            "git_head",
            "tracked_worktree",
            "dirty",
            "toolchain",
            "source_date_epoch",
            "native_build",
            "command",
            "target",
            "profile",
            "features",
            "cargo_feature_graph",
            "artifacts",
            "determinism_proof",
        ],
    ) && manifest.get("schema").and_then(serde_json::Value::as_str)
        == Some(authority.package.manifest_schema.as_str())
        && manifest.get("command").and_then(serde_json::Value::as_str)
            == Some(PACKAGE_PROOF_COMMAND)
        && manifest.get("git_head").and_then(serde_json::Value::as_str)
            == Some(index.source_sha.as_str())
        && manifest.get("dirty").and_then(serde_json::Value::as_bool) == Some(false)
        && manifest.get("profile").and_then(serde_json::Value::as_str)
            == Some(authority.package.profile.as_str())
        && manifest.get("features") == Some(&serde_json::json!(&authority.package.features))
        && manifest.get("tracked_worktree").is_some_and(|worktree| {
            exact_json_fields(worktree, &["file_count", "sha256"])
                && worktree
                    .get("file_count")
                    .and_then(serde_json::Value::as_u64)
                    .is_some_and(|count| count > 0)
                && worktree
                    .get("sha256")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|hash| is_hex(hash, 64))
        })
        && manifest
            .get("source_date_epoch")
            .and_then(serde_json::Value::as_u64)
            .is_some_and(|epoch| epoch > 0)
        && manifest.get("target").and_then(serde_json::Value::as_str)
            == rust_target_for_tuple(&index.tuple)
        && rust_target_for_tuple(&index.tuple).is_some_and(|target| {
            validate_package_toolchain(manifest.get("toolchain"), target)
                && validate_native_build_shape(
                    manifest.get("native_build"),
                    &index.tuple,
                    target,
                    manifest.get("source_date_epoch"),
                    &authority.package.features,
                )
        })
        && validate_package_feature_graph(
            manifest.get("cargo_feature_graph"),
            &authority.package.features,
        )
        && manifest
            .get("artifacts")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|artifacts| {
                artifacts.len() == authority.package.artifacts.len()
                    && artifacts.iter().zip(&authority.package.artifacts).all(
                        |(artifact, expected)| {
                            artifact.get("role").and_then(serde_json::Value::as_str)
                                == Some(expected.role.as_str())
                        },
                    )
            })
        && valid_package_determinism_proof(manifest)
}

fn valid_package_artifact_evidence(
    index: &EvidenceIndex,
    gate_id: &str,
    artifacts: Option<&Vec<serde_json::Value>>,
    expected: &super::authority::PackageArtifactAuthority,
) -> bool {
    let matching: Vec<_> = artifacts
        .into_iter()
        .flatten()
        .filter(|artifact| {
            artifact.get("role").and_then(serde_json::Value::as_str) == Some(expected.role.as_str())
        })
        .collect();
    if matching.len() != 1 {
        return false;
    }
    let artifact = matching[0];
    let Some(source_path) = artifact.get("path").and_then(serde_json::Value::as_str) else {
        return false;
    };
    let Some(filename) = Path::new(source_path)
        .file_name()
        .and_then(|name| name.to_str())
    else {
        return false;
    };
    let evidence_role = format!("package-{}", expected.role);
    let evidence: Vec<_> = index
        .entries
        .iter()
        .filter(|entry| entry.role == evidence_role)
        .collect();
    let producing_command: Vec<String> = expected
        .producing_command
        .split_whitespace()
        .map(str::to_string)
        .collect();
    exact_json_fields(
        artifact,
        &[
            "role",
            "path",
            "media_type",
            "producing_command",
            "byte_length",
            "sha256",
        ],
    ) && validate_relative_path(source_path)
        && evidence.len() == 1
        && evidence[0].producing_gate == gate_id
        && evidence[0].path == format!("payloads/{evidence_role}/{filename}")
        && artifact
            .get("media_type")
            .and_then(serde_json::Value::as_str)
            == Some(expected.media_type.as_str())
        && artifact
            .get("producing_command")
            .and_then(serde_json::Value::as_str)
            == Some(expected.producing_command.as_str())
        && evidence[0].mime == expected.media_type
        && evidence[0].producing_command == producing_command
        && artifact
            .get("byte_length")
            .and_then(serde_json::Value::as_u64)
            == Some(evidence[0].byte_length)
        && artifact.get("sha256").and_then(serde_json::Value::as_str)
            == Some(evidence[0].sha256.as_str())
}

fn validate_successful_package_gate(
    root: &Path,
    index: &EvidenceIndex,
    authority: &Authority,
    gate: &super::authority::Gate,
) -> Vec<String> {
    let mut contracts = Vec::new();
    let Some(package_step) = gate.steps.iter().find(|step| step.id == "xtask-package") else {
        return vec!["evidence.package.authority".to_string()];
    };
    let Some(ownership_step) = gate
        .steps
        .iter()
        .find(|step| step.id == "verify-production-ownership")
    else {
        return vec!["evidence.package.authority".to_string()];
    };
    let Some(dependency_step) = gate
        .steps
        .iter()
        .find(|step| step.id == "capture-native-dependencies")
    else {
        return vec!["evidence.package.authority".to_string()];
    };
    let manifests: Vec<_> = index
        .entries
        .iter()
        .filter(|entry| entry.role == "package-manifest")
        .collect();
    if manifests.len() != 1
        || manifests[0].producing_gate != gate.id
        || manifests[0].producing_command != package_step.command
        || manifests[0].path != "payloads/package-manifest/production-artifacts.json"
    {
        contracts.push("evidence.package.manifest.identity".to_string());
        return contracts;
    }
    let manifest: serde_json::Value = match read_bundle_file(root, &manifests[0].path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
    {
        Some(manifest) => manifest,
        None => {
            contracts.push("evidence.package.manifest.json".to_string());
            return contracts;
        }
    };
    if !valid_package_manifest_content(index, authority, &manifest) {
        contracts.push("evidence.package.manifest.content".to_string());
    }
    let expected_artifacts = &authority.package.artifacts;
    let artifacts = manifest.get("artifacts").and_then(|value| value.as_array());
    if artifacts.is_none_or(|items| items.len() != expected_artifacts.len()) {
        contracts.push("evidence.package.artifacts.count".to_string());
    }
    for expected_artifact in expected_artifacts {
        let role = &expected_artifact.role;
        let matching: Vec<_> = artifacts
            .into_iter()
            .flatten()
            .filter(|artifact| {
                artifact.get("role").and_then(|value| value.as_str()) == Some(role.as_str())
            })
            .collect();
        if matching.len() != 1 {
            contracts.push(format!("evidence.package.artifact.{role}.manifest"));
            continue;
        }
        let artifact = matching[0];
        let Some(source_path) = artifact.get("path").and_then(|value| value.as_str()) else {
            contracts.push(format!("evidence.package.artifact.{role}.path"));
            continue;
        };
        let Some(filename) = Path::new(source_path)
            .file_name()
            .and_then(|name| name.to_str())
        else {
            contracts.push(format!("evidence.package.artifact.{role}.path"));
            continue;
        };
        if !validate_relative_path(source_path) {
            contracts.push(format!("evidence.package.artifact.{role}.path"));
        }
        let evidence_role = format!("package-{role}");
        let expected_path = format!("payloads/{evidence_role}/{filename}");
        let producing_command: Vec<String> = expected_artifact
            .producing_command
            .split_whitespace()
            .map(str::to_string)
            .collect();
        let evidence: Vec<_> = index
            .entries
            .iter()
            .filter(|entry| entry.role == evidence_role)
            .collect();
        let valid = exact_json_fields(
            artifact,
            &[
                "role",
                "path",
                "media_type",
                "producing_command",
                "byte_length",
                "sha256",
            ],
        ) && evidence.len() == 1
            && evidence[0].producing_gate == gate.id
            && evidence[0].path == expected_path
            && artifact.get("media_type").and_then(|value| value.as_str())
                == Some(expected_artifact.media_type.as_str())
            && artifact
                .get("producing_command")
                .and_then(|value| value.as_str())
                == Some(expected_artifact.producing_command.as_str())
            && evidence[0].mime == expected_artifact.media_type
            && evidence[0].producing_command == producing_command
            && artifact.get("byte_length").and_then(|value| value.as_u64())
                == Some(evidence[0].byte_length)
            && artifact.get("sha256").and_then(|value| value.as_str())
                == Some(evidence[0].sha256.as_str());
        if !valid {
            contracts.push(format!("evidence.package.artifact.{role}.evidence"));
        }
    }
    for (role, filename, command) in [
        (
            "ownership-production-report",
            "ownership-production-report.json".to_string(),
            &ownership_step.command,
        ),
        (
            "native-dependency-capture",
            format!("native-dependencies-{}.candidate.json", index.tuple),
            &dependency_step.command,
        ),
    ] {
        let matching: Vec<_> = index
            .entries
            .iter()
            .filter(|entry| entry.role == role)
            .collect();
        if matching.len() != 1
            || matching[0].producing_gate != gate.id
            || matching[0].producing_command != *command
            || matching[0].path != format!("payloads/{role}/{filename}")
        {
            contracts.push(format!("evidence.package.{role}.identity"));
        }
    }
    validate_package_ownership_report(root, index, authority, artifacts, &mut contracts);
    validate_native_dependency_capture(root, index, &mut contracts);
    let allowed = [
        "authority.snapshot",
        "preflight.source",
        "preflight.tools",
        "cache.initial-state",
        "ownership.zero-native-delta",
        "gate.result",
        "step.stdout",
        "step.stderr",
        "step.result",
        "package-manifest",
        "package-executable",
        "package-rust_static_archive",
        "package-c_static_archive",
        "package-object_sidecar",
        "package-provider_report",
        "ownership-production-report",
        "native-dependency-capture",
    ];
    for entry in index
        .entries
        .iter()
        .filter(|entry| entry.producing_gate == gate.id && !allowed.contains(&entry.role.as_str()))
    {
        contracts.push(format!("evidence.package.unexpected_role.{}", entry.role));
    }
    contracts
}

fn valid_provider_entries(entries: Option<&Vec<serde_json::Value>>) -> bool {
    entries.is_some_and(|items| {
        items.iter().all(|item| {
            exact_json_fields(
                item,
                &["path", "issue", "provider", "archive_decision", "status"],
            ) && item
                .get("path")
                .and_then(|value| value.as_str())
                .is_some_and(validate_relative_path)
                && item
                    .get("issue")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|value| !value.is_empty())
                && item
                    .get("archive_decision")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|decision| {
                        matches!(
                            decision,
                            "include"
                                | "exclude_duplicate_provider"
                                | "exclude_recompiled"
                                | "exclude_replaced"
                        )
                    })
                && item
                    .get("provider")
                    .and_then(serde_json::Value::as_str)
                    .zip(item.get("path").and_then(serde_json::Value::as_str))
                    .zip(
                        item.get("archive_decision")
                            .and_then(serde_json::Value::as_str),
                    )
                    .is_some_and(|((provider, path), decision)| {
                        if decision == "include" {
                            provider == format!("native_object:{path}")
                        } else {
                            provider
                                .strip_prefix("rust_source:")
                                .is_some_and(validate_relative_path)
                        }
                    })
        }) && items.windows(2).all(|pair| {
            pair[0]
                .get("path")
                .and_then(|value| value.as_str())
                .zip(pair[1].get("path").and_then(|value| value.as_str()))
                .is_some_and(|(left, right)| left < right)
        })
    })
}

fn validate_package_ownership_report(
    root: &Path,
    index: &EvidenceIndex,
    authority: &Authority,
    artifacts: Option<&Vec<serde_json::Value>>,
    contracts: &mut Vec<String>,
) {
    let entries: Vec<_> = index
        .entries
        .iter()
        .filter(|entry| entry.role == "ownership-production-report")
        .collect();
    if entries.len() != 1 {
        return;
    }
    let report: Option<serde_json::Value> = read_bundle_file(root, &entries[0].path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok());
    let Some(report) = report else {
        contracts.push("evidence.package.ownership-report.json".to_string());
        return;
    };
    let provider = report.get("provider_report");
    let provider_entries = provider
        .and_then(|value| value.get("entries"))
        .and_then(|value| value.as_array());
    let summary = provider.and_then(|value| value.get("summary"));
    let provider_valid = provider
        .and_then(|value| value.get("schema"))
        .and_then(|value| value.as_str())
        == Some("uqm-provider-report-v1")
        && provider
            .and_then(|value| value.get("tracked_native_file_delta"))
            .and_then(|value| value.as_i64())
            == Some(0)
        && provider_entries.is_some_and(|items| {
            !items.is_empty()
                && items
                    .iter()
                    .all(|item| item.get("status").and_then(|value| value.as_str()) == Some("ok"))
        })
        && summary
            .and_then(|value| value.get("violations"))
            .and_then(|value| value.as_u64())
            == Some(0)
        && summary
            .and_then(|value| value.get("passed"))
            .and_then(|value| value.as_bool())
            == Some(true);
    let provider_artifacts: Vec<_> = index
        .entries
        .iter()
        .filter(|entry| entry.role == "package-provider_report")
        .collect();
    let retained_provider: Option<serde_json::Value> = (provider_artifacts.len() == 1)
        .then(|| read_bundle_file(root, &provider_artifacts[0].path).ok())
        .flatten()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok());
    let entries_exact = valid_provider_entries(provider_entries);
    let symbols = provider
        .and_then(|value| value.get("symbols"))
        .and_then(|value| value.as_array());
    let symbols_exact = symbols.is_some_and(|items| {
        items.iter().all(|item| {
            exact_json_fields(
                item,
                &[
                    "symbol",
                    "canonical_owner",
                    "provider_kind",
                    "provider_path",
                    "excluded_provider_paths",
                ],
            ) && item
                .get("symbol")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|value| !value.is_empty())
                && item
                    .get("canonical_owner")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|value| !value.is_empty())
                && item
                    .get("provider_kind")
                    .and_then(serde_json::Value::as_str)
                    == Some("rust_source")
                && item
                    .get("provider_path")
                    .and_then(|value| value.as_str())
                    .is_some_and(validate_relative_path)
                && item
                    .get("excluded_provider_paths")
                    .and_then(|value| value.as_array())
                    .is_some_and(|paths| {
                        !paths.is_empty()
                            && paths
                                .iter()
                                .all(|path| path.as_str().is_some_and(validate_relative_path))
                            && paths.windows(2).all(|pair| {
                                pair[0]
                                    .as_str()
                                    .zip(pair[1].as_str())
                                    .is_some_and(|(left, right)| left < right)
                            })
                    })
        }) && items.windows(2).all(|pair| {
            pair[0]
                .get("symbol")
                .and_then(|value| value.as_str())
                .zip(pair[1].get("symbol").and_then(|value| value.as_str()))
                .is_some_and(|(left, right)| left < right)
        })
    });
    let summary_exact = summary.is_some_and(|value| {
        exact_json_fields(
            value,
            &[
                "total_objects",
                "included",
                "excluded",
                "duplicate_providers_excluded",
                "recompiled",
                "replaced",
                "violations",
                "passed",
            ],
        ) && value.get("total_objects").and_then(|field| field.as_u64())
            == provider_entries.map(|items| items.len() as u64)
            && provider_entries.is_some_and(|items| {
                let count = |decision: &str| {
                    items
                        .iter()
                        .filter(|item| {
                            item.get("archive_decision")
                                .and_then(serde_json::Value::as_str)
                                == Some(decision)
                        })
                        .count() as u64
                };
                let included = count("include");
                let duplicates = count("exclude_duplicate_provider");
                let recompiled = count("exclude_recompiled");
                let replaced = count("exclude_replaced");
                let excluded = duplicates + recompiled + replaced;
                value.get("included").and_then(serde_json::Value::as_u64) == Some(included)
                    && value.get("excluded").and_then(serde_json::Value::as_u64) == Some(excluded)
                    && value
                        .get("duplicate_providers_excluded")
                        .and_then(serde_json::Value::as_u64)
                        == Some(duplicates)
                    && value.get("recompiled").and_then(serde_json::Value::as_u64)
                        == Some(recompiled)
                    && value.get("replaced").and_then(serde_json::Value::as_u64) == Some(replaced)
            })
    });
    let exact_content = exact_json_fields(
        &report,
        &[
            "schema",
            "provider_report",
            "rust_archive",
            "c_archive",
            "executable",
        ],
    ) && provider.is_some_and(|value| {
        exact_json_fields(
            value,
            &[
                "schema",
                "entries",
                "ledger_sha256",
                "symbols",
                "tracked_native_file_delta",
                "summary",
            ],
        ) && value.get("ledger_sha256").and_then(|field| field.as_str())
            == Some(authority.ledger_identity.sha256.as_str())
    }) && retained_provider.as_ref() == provider
        && entries_exact
        && symbols_exact
        && summary_exact;
    if report.get("schema").and_then(|value| value.as_str())
        != Some("uqm-production-artifact-report-v1")
        || !provider_valid
        || !exact_content
    {
        contracts.push("evidence.package.ownership-report.content".to_string());
    }
    for (report_key, artifact_role) in [
        ("executable", "executable"),
        ("rust_archive", "rust_static_archive"),
        ("c_archive", "c_static_archive"),
    ] {
        let expected_hash = artifacts
            .into_iter()
            .flatten()
            .find(|artifact| {
                artifact.get("role").and_then(|value| value.as_str()) == Some(artifact_role)
            })
            .and_then(|artifact| artifact.get("sha256"))
            .and_then(|value| value.as_str());
        let digest = report.get(report_key);
        let actual_hash = digest
            .and_then(|value| value.get("sha256"))
            .and_then(|value| value.as_str());
        if digest.is_none_or(|value| !exact_json_fields(value, &["path", "sha256"]))
            || actual_hash != expected_hash
            || actual_hash.is_none_or(|hash| !is_hex(hash, 64))
        {
            contracts.push(format!("evidence.package.ownership-report.{artifact_role}"));
        }
    }
}

fn validate_native_dependency_capture(
    root: &Path,
    index: &EvidenceIndex,
    contracts: &mut Vec<String>,
) {
    let entries: Vec<_> = index
        .entries
        .iter()
        .filter(|entry| entry.role == "native-dependency-capture")
        .collect();
    if entries.len() != 1 {
        return;
    }
    let capture: Option<serde_json::Value> = read_bundle_file(root, &entries[0].path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok());
    let Some(capture) = capture else {
        contracts.push("evidence.package.native-dependencies.json".to_string());
        return;
    };
    if !valid_native_dependency_content(&capture, &index.tuple) {
        contracts.push("evidence.package.native-dependencies.content".to_string());
    }
}

fn valid_native_dependency_content(capture: &serde_json::Value, tuple: &str) -> bool {
    let dependencies = capture
        .get("dependencies")
        .and_then(serde_json::Value::as_array);
    let dependencies_valid = dependencies.is_some_and(|items| {
        !items.is_empty()
            && items
                .iter()
                .all(|item| item.as_str().is_some_and(validate_relative_path))
            && items.windows(2).all(|pair| {
                pair[0]
                    .as_str()
                    .is_some_and(|left| pair[1].as_str().is_some_and(|right| left < right))
            })
    });
    exact_json_fields(capture, &["schema", "target", "dependencies"])
        && capture.get("schema").and_then(serde_json::Value::as_str)
            == Some("uqm-native-dependency-capture-v1")
        && capture.get("target").and_then(serde_json::Value::as_str) == Some(tuple)
        && dependencies_valid
}

fn validate_successful_builtin_gate(
    root: &Path,
    index: &EvidenceIndex,
    authority: &Authority,
    gate: &str,
) -> Vec<String> {
    let mut contracts = Vec::new();
    let expected_step_count = match gate {
        "complexity" | "coverage" | "workflow" => 1,
        "bootstrap-proof" => 3,
        "mutations" => 0,
        _ => 0,
    };
    let actual_step_entries = index
        .entries
        .iter()
        .filter(|entry| {
            entry.producing_gate == gate
                && matches!(
                    entry.role.as_str(),
                    "step.stdout" | "step.stderr" | "step.result"
                )
        })
        .count();
    if actual_step_entries != expected_step_count * 3 {
        contracts.push(format!(
            "evidence.builtin.{gate}.step_entry_count (expected {}, got {actual_step_entries})",
            expected_step_count * 3
        ));
    }
    let supplemental_roles: &[&str] = match gate {
        "coverage" => &["coverage.lcov"],
        "workflow" => &["workflow.validation"],
        "mutations" => &["mutations.receipt", "mutation.fixture"],
        "bootstrap-proof" => &[
            "bootstrap-proof.package-manifest",
            "bootstrap-proof.executable",
            "bootstrap-proof.profile",
            "bootstrap-proof.runner",
            "bootstrap-proof.lcar",
            "bootstrap-proof.lcar-artifact",
        ],
        _ => &[],
    };
    let unexpected_roles = index.entries.iter().filter(|entry| {
        entry.producing_gate == gate
            && !matches!(
                entry.role.as_str(),
                "authority.snapshot"
                    | "preflight.source"
                    | "preflight.tools"
                    | "cache.initial-state"
                    | "ownership.zero-native-delta"
                    | "gate.result"
                    | "step.stdout"
                    | "step.stderr"
                    | "step.result"
            )
            && !supplemental_roles.contains(&entry.role.as_str())
    });
    for entry in unexpected_roles {
        contracts.push(format!(
            "evidence.builtin.{gate}.unexpected_role.{}",
            entry.role
        ));
    }
    match gate {
        "complexity" => {
            contracts.extend(validate_builtin_step(
                root,
                index,
                gate,
                "lizard",
                &[0],
                |command| validate_complexity_command(command, authority),
            ));
        }
        "coverage" => {
            contracts.extend(validate_builtin_step(
                root,
                index,
                gate,
                "llvm-cov",
                &[0],
                |command| validate_coverage_command(command, authority),
            ));
            if let Some(command) = index
                .entries
                .iter()
                .find(|entry| {
                    entry.role == "step.stdout" && entry.path == "coverage/llvm-cov.stdout.log"
                })
                .map(|entry| entry.producing_command.as_slice())
            {
                validate_builtin_payload(
                    index,
                    "coverage.lcov",
                    gate,
                    "payloads/coverage.lcov/coverage.lcov",
                    command,
                    &mut contracts,
                );
                if let Some(entry) = index
                    .entries
                    .iter()
                    .find(|entry| entry.role == "coverage.lcov")
                {
                    match read_bundle_file(root, &entry.path).and_then(|bytes| {
                        super::run::lcov_line_coverage(&bytes).map_err(|error| {
                            std::io::Error::new(std::io::ErrorKind::InvalidData, error)
                        })
                    }) {
                        Ok(percent) if percent >= authority.coverage.minimum_line_percent => {}
                        _ => contracts
                            .push("evidence.builtin.coverage.coverage.lcov.content".to_string()),
                    }
                }
            }
        }
        "workflow" => {
            contracts.extend(validate_builtin_step(
                root,
                index,
                gate,
                "actionlint",
                &[0],
                |command| command == ["actionlint"],
            ));
            let gate_command = vec![
                index
                    .entries
                    .iter()
                    .find(|entry| entry.role == "authority.snapshot")
                    .and_then(|entry| entry.producing_command.first())
                    .cloned()
                    .unwrap_or_default(),
                "ci".into(),
                "run".into(),
                gate.into(),
            ];
            validate_builtin_payload(
                index,
                "workflow.validation",
                gate,
                "workflow/workflow-validation.json",
                &gate_command,
                &mut contracts,
            );
            contracts.extend(validate_passed_collection_receipt(
                root,
                index,
                authority,
                "workflow.validation",
            ));
        }
        "mutations" => {
            let gate_command = vec![
                index
                    .entries
                    .iter()
                    .find(|entry| entry.role == "authority.snapshot")
                    .and_then(|entry| entry.producing_command.first())
                    .cloned()
                    .unwrap_or_default(),
                "ci".into(),
                "run".into(),
                gate.into(),
            ];
            validate_builtin_payload(
                index,
                "mutations.receipt",
                gate,
                "mutations/mutations-receipt.json",
                &gate_command,
                &mut contracts,
            );
            contracts.extend(validate_passed_collection_receipt(
                root,
                index,
                authority,
                "mutations.receipt",
            ));
            contracts.extend(validate_mutation_fixtures(root, index, authority));
        }
        "bootstrap-proof" => {
            let build = vec![
                "cargo".into(),
                "build".into(),
                "--locked".into(),
                "--manifest-path".into(),
                "rust/Cargo.toml".into(),
                "--bin".into(),
                "uqm-gameplay-proof".into(),
            ];
            contracts.extend(validate_builtin_step(
                root,
                index,
                gate,
                "build-runner",
                &[0],
                |command| command == build,
            ));
            contracts.extend(validate_builtin_step(
                root,
                index,
                gate,
                "run",
                &[0],
                |command| validate_bootstrap_run_command(command, authority),
            ));
            contracts.extend(validate_builtin_step(
                root,
                index,
                gate,
                "validate",
                &[0],
                validate_bootstrap_validate_command,
            ));
            let package = vec![
                "cargo".into(),
                "run".into(),
                "--locked".into(),
                "--manifest-path".into(),
                "rust/xtask/Cargo.toml".into(),
                "--".into(),
                "package".into(),
            ];
            for (role, filename) in [
                (
                    "bootstrap-proof.package-manifest",
                    authority.bootstrap_proof.packaged_manifest.as_str(),
                ),
                (
                    "bootstrap-proof.executable",
                    authority.bootstrap_proof.packaged_executable.as_str(),
                ),
                (
                    "bootstrap-proof.profile",
                    Path::new(&authority.bootstrap_proof.profile)
                        .file_name()
                        .and_then(|value| value.to_str())
                        .unwrap_or_default(),
                ),
            ] {
                validate_builtin_payload(
                    index,
                    role,
                    gate,
                    &format!("payloads/{role}/{filename}"),
                    &package,
                    &mut contracts,
                );
            }
            validate_bootstrap_profile(index, authority, &mut contracts);
            validate_bootstrap_package(root, index, authority, &mut contracts);
            validate_builtin_payload(
                index,
                "bootstrap-proof.runner",
                gate,
                "payloads/bootstrap-proof.runner/uqm-gameplay-proof",
                &build,
                &mut contracts,
            );
            validate_bootstrap_runner(root, index, &mut contracts);

            if let Some(run_command) = builtin_step_command(index, gate, "run") {
                validate_builtin_payload(
                    index,
                    "bootstrap-proof.lcar",
                    gate,
                    "payloads/bootstrap-proof.lcar/lcar-v1.json",
                    run_command,
                    &mut contracts,
                );
                validate_bootstrap_lcar(root, index, authority, run_command, &mut contracts);
                let correlated = run_command.len() == 6
                    && builtin_step_command(index, gate, "validate").is_some_and(|validate| {
                        validate.len() == 3
                            && validate[0] == run_command[0]
                            && Path::new(&validate[2])
                                == Path::new(&run_command[5]).join("lcar-v1.json")
                    });
                if !correlated {
                    contracts.push("evidence.builtin.bootstrap-proof.command_chain".to_string());
                }
            }
        }
        _ => contracts.push(format!("evidence.builtin.{gate}.unknown")),
    }
    contracts
}

fn builtin_step_command<'a>(
    index: &'a EvidenceIndex,
    gate: &str,
    step: &str,
) -> Option<&'a [String]> {
    let path = format!("{gate}/{step}.stdout.log");
    index
        .entries
        .iter()
        .find(|entry| entry.role == "step.stdout" && entry.path == path)
        .map(|entry| entry.producing_command.as_slice())
}

fn validate_bootstrap_run_command(command: &[String], authority: &Authority) -> bool {
    if command.len() != 6 {
        return false;
    }
    let root = Path::new(&command[2]);
    Path::new(&command[0]).is_absolute()
        && Path::new(&command[0]).ends_with("rust/target/debug/uqm-gameplay-proof")
        && command[1] == "run"
        && root.is_absolute()
        && Path::new(&command[3]).starts_with(root.join(&authority.bootstrap_proof.packaged_root))
        && Path::new(&command[3]).ends_with(&authority.bootstrap_proof.packaged_manifest)
        && Path::new(&command[4]) == root.join(&authority.bootstrap_proof.profile)
        && Path::new(&command[5]).is_absolute()
        && Path::new(&command[5]).ends_with("bootstrap-proof")
}

fn validate_bootstrap_validate_command(command: &[String]) -> bool {
    command.len() == 3
        && Path::new(&command[0]).is_absolute()
        && Path::new(&command[0]).ends_with("rust/target/debug/uqm-gameplay-proof")
        && command[1] == "validate"
        && Path::new(&command[2]).is_absolute()
        && Path::new(&command[2]).ends_with("bootstrap-proof/lcar-v1.json")
}

fn validate_passed_collection_receipt(
    root: &Path,
    index: &EvidenceIndex,
    authority: &Authority,
    role: &str,
) -> Vec<String> {
    let (schema, collection, first_failure, item_result) = collection_contract(role);
    let (identity_field, expected_identities) = expected_collection_identities(role, authority);
    let mut contracts = Vec::new();
    let receipt = index
        .entries
        .iter()
        .find(|entry| entry.role == role)
        .and_then(|entry| read_bundle_file(root, &entry.path).ok())
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok());
    let valid = receipt.as_ref().is_some_and(|receipt| {
        receipt.get("schema").and_then(|value| value.as_str()) == Some(schema)
            && exact_collection_receipt_fields(role, receipt)
            && (role == "workflow.validation"
                || (receipt.get("passed").and_then(|value| value.as_bool()) == Some(true)
                    && receipt.get("source_sha").and_then(|value| value.as_str())
                        == Some(index.source_sha.as_str())))
            && receipt
                .get(first_failure)
                .is_some_and(|value| value.is_null())
            && receipt
                .get(collection)
                .and_then(|value| value.as_array())
                .is_some_and(|items| {
                    items.len() == expected_identities.len()
                        && items
                            .iter()
                            .zip(&expected_identities)
                            .all(|(item, expected)| {
                                item.get(identity_field).and_then(|value| value.as_str())
                                    == Some(*expected)
                                    && item.get(item_result).and_then(|value| value.as_bool())
                                        == Some(true)
                                    && validate_collection_item(role, item, expected, authority)
                            })
                })
    });
    if !valid {
        contracts.push(format!("evidence.builtin.{role}.content"));
    }
    contracts
}

fn collection_contract(role: &str) -> (&'static str, &'static str, &'static str, &'static str) {
    match role {
        "workflow.validation" => (
            "uqm-s4-workflow-validation-v1",
            "rules",
            "first_failed_rule",
            "passed",
        ),
        "mutations.receipt" => (
            "uqm-s4-mutations-receipt-v3",
            "cases",
            "first_failed_target",
            "rejection_observed",
        ),
        _ => ("", "", "", ""),
    }
}

fn exact_collection_receipt_fields(role: &str, receipt: &serde_json::Value) -> bool {
    match role {
        "workflow.validation" => {
            exact_json_fields(receipt, &["schema", "first_failed_rule", "rules"])
        }
        "mutations.receipt" => exact_json_fields(
            receipt,
            &[
                "schema",
                "source_sha",
                "passed",
                "first_failed_target",
                "cases",
            ],
        ),
        _ => false,
    }
}

fn expected_collection_identities<'a>(
    role: &str,
    authority: &'a Authority,
) -> (&'static str, Vec<&'a str>) {
    if role == "workflow.validation" {
        (
            "rule",
            vec![
                "workflow.actionlint",
                "workflow.unrestricted_triggers",
                "workflow.checkout_pr_head",
                "workflow.actions_full_sha",
                "workflow.required_identity_environment",
                "workflow.tool_authority",
                "workflow.least_permissions",
                "workflow.timeouts",
                "workflow.generated_matrix",
                "workflow.no_direct_gate_commands",
                "workflow.no_cache_action",
                "workflow.always_uploaded_failure_evidence",
                "workflow.content_addressed_transport",
            ],
        )
    } else {
        (
            "target",
            authority
                .mutation_targets
                .iter()
                .map(String::as_str)
                .collect(),
        )
    }
}

fn validate_collection_item(
    role: &str,
    item: &serde_json::Value,
    identity: &str,
    authority: &Authority,
) -> bool {
    if role == "workflow.validation" {
        return exact_json_fields(item, &["rule", "passed", "detail"])
            && item
                .get("detail")
                .and_then(|value| value.as_str())
                .is_some_and(|detail| !detail.is_empty());
    }
    let Some(expected_contract) = MutationTarget::parse(identity).map(MutationTarget::contract)
    else {
        return false;
    };
    exact_json_fields(
        item,
        &[
            "target",
            "contract",
            "defect",
            "baseline_accepted",
            "rejection_observed",
            "detail",
            "baseline_executions",
            "executions",
            "baseline_files",
            "files",
            "recipe",
            "expected_diagnostic",
        ],
    ) && item.get("contract").and_then(|value| value.as_str()) == Some(expected_contract)
        && item
            .get("baseline_accepted")
            .and_then(|value| value.as_bool())
            == Some(true)
        && item
            .get("defect")
            .and_then(|value| value.as_str())
            .is_some_and(|value| !value.is_empty())
        && item
            .get("detail")
            .and_then(|value| value.as_str())
            .is_some_and(|value| !value.is_empty())
        && valid_mutation_file_declarations(item.get("baseline_files"))
        && valid_mutation_file_declarations(item.get("files"))
        && valid_mutation_causal_metadata(item, identity, authority)
        && validate_mutation_execution(item, identity, authority)
}

fn artifact_mutation_metadata(
    item: &serde_json::Value,
) -> Option<(
    super::mutations::MutationRecipe,
    super::mutations::MutationDiagnostic,
)> {
    let hash_for = |field: &str, phase: &str| {
        item.get(field)?
            .as_array()?
            .iter()
            .enumerate()
            .find_map(|(position, declaration)| {
                (retained_mutation_name(declaration, "artifact", phase, position)
                    == Some("tool-preflight.json"))
                .then(|| {
                    declaration
                        .get("sha256")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_string)
                })
                .flatten()
            })
    };
    Some((
        super::mutations::MutationRecipe {
            operation: "replace-and-rehash-enclosing-index".to_string(),
            path: "tool-preflight.json".to_string(),
            baseline_sha256: hash_for("baseline_files", "baseline")?,
            mutant_sha256: hash_for("files", "mutant")?,
        },
        super::mutations::MutationDiagnostic {
            class: "artifact-provenance".to_string(),
            path: "tool-preflight.json".to_string(),
            required_fragments: vec![
                "artifact-provenance".to_string(),
                "evidence.preflight.tools.rust.result".to_string(),
            ],
        },
    ))
}

fn valid_mutation_causal_metadata(
    item: &serde_json::Value,
    target: &str,
    authority: &Authority,
) -> bool {
    let metadata = if target == "artifact" {
        artifact_mutation_metadata(item)
    } else {
        super::mutations::expected_causal_contract(target, authority)
            .map(|(recipe, diagnostic, _, _)| (recipe, diagnostic))
    };
    let Some((recipe, diagnostic)) = metadata else {
        return false;
    };
    item.get("recipe") == Some(&serde_json::to_value(recipe).unwrap())
        && item.get("expected_diagnostic") == Some(&serde_json::to_value(diagnostic).unwrap())
}

fn valid_mutation_file_declarations(value: Option<&serde_json::Value>) -> bool {
    value
        .and_then(|value| value.as_array())
        .is_some_and(|files| {
            !files.is_empty()
                && files.iter().all(|file| {
                    exact_json_fields(file, &["path", "byte_length", "sha256"])
                        && file
                            .get("path")
                            .and_then(|value| value.as_str())
                            .is_some_and(|path| !path.is_empty())
                        && file
                            .get("byte_length")
                            .and_then(|value| value.as_u64())
                            .is_some()
                        && file
                            .get("sha256")
                            .and_then(|value| value.as_str())
                            .is_some_and(|hash| is_hex(hash, 64))
                })
        })
}

type MutationRouteIdentity = (String, String, String, Option<String>, Vec<String>, u64);

fn validate_mutation_execution(
    item: &serde_json::Value,
    target: &str,
    authority: &Authority,
) -> bool {
    let expected_route: Option<Vec<MutationRouteIdentity>> = match target {
        "format" | "check" | "clippy" | "test" => {
            let gate_id = if target == "test" { "tests" } else { target };
            authority.gate(gate_id).map(|gate| {
                gate.steps
                    .iter()
                    .map(|step| {
                        (
                            gate.id.clone(),
                            step.id.clone(),
                            step.cwd.clone(),
                            step.native_profile.clone(),
                            step.command.clone(),
                            step.timeout_seconds * 1_000,
                        )
                    })
                    .collect()
            })
        }
        "harness" => authority.gate("probes-harnesses").and_then(|gate| {
            gate.steps
                .iter()
                .find(|step| step.id == "p00-harness")
                .map(|step| {
                    vec![(
                        gate.id.clone(),
                        step.id.clone(),
                        step.cwd.clone(),
                        step.native_profile.clone(),
                        step.command.clone(),
                        step.timeout_seconds * 1_000,
                    )]
                })
        }),
        "complexity" => Some(vec![(
            "complexity".to_string(),
            "lizard".to_string(),
            ".".to_string(),
            None,
            std::iter::once("lizard".to_string())
                .chain(authority.complexity.lizard_arguments.iter().cloned())
                .chain(std::iter::once("runaway.rs".to_string()))
                .collect(),
            authority.supervision.builtin_timeout_seconds * 1_000,
        )]),
        "ownership" | "link" | "security" | "coverage" | "cache" | "workflow" | "artifact" => {
            Some(vec![(
                "mutations".to_string(),
                "internal-validator".to_string(),
                ".".to_string(),
                None,
                vec![
                    "uqm-xtask-internal".to_string(),
                    super::mutations::INTERNAL_VALIDATOR_COMMAND.to_string(),
                    target.to_string(),
                ],
                authority.supervision.builtin_timeout_seconds * 1_000,
            )])
        }
        _ => None,
    };
    let baseline = item
        .get("baseline_executions")
        .and_then(|value| value.as_array());
    let mutant = item.get("executions").and_then(|value| value.as_array());
    let Some(expected_route) = expected_route else {
        return baseline.is_some_and(Vec::is_empty) && mutant.is_some_and(Vec::is_empty);
    };
    let Some(baseline) = baseline else {
        return false;
    };
    let Some(mutant) = mutant else {
        return false;
    };
    baseline.len() == expected_route.len()
        && !mutant.is_empty()
        && mutant.len() <= expected_route.len()
        && baseline
            .iter()
            .zip(&expected_route)
            .all(|(execution, expected)| {
                validate_mutation_process_result(execution, expected, true)
            })
        && mutant.iter().zip(&expected_route).enumerate().all(
            |(position, (execution, expected))| {
                validate_mutation_process_result(execution, expected, position + 1 < mutant.len())
            },
        )
}

fn validate_mutation_process_result(
    execution: &serde_json::Value,
    expected: &MutationRouteIdentity,
    should_succeed: bool,
) -> bool {
    exact_json_fields(
        execution,
        &[
            "gate",
            "step",
            "cwd",
            "native_profile",
            "command",
            "executable_identity",
            "supervision",
            "stdout",
            "stderr",
            "exit_code",
            "signal",
            "launch_error",
            "success",
        ],
    ) && execution.get("gate").and_then(|value| value.as_str()) == Some(expected.0.as_str())
        && execution.get("step").and_then(|value| value.as_str()) == Some(expected.1.as_str())
        && execution.get("cwd").and_then(|value| value.as_str()) == Some(expected.2.as_str())
        && execution.get("native_profile") == Some(&serde_json::json!(expected.3))
        && execution.get("command") == Some(&serde_json::json!(expected.4))
        && execution
            .get("executable_identity")
            .is_some_and(valid_executable_identity)
        && strict_mutation_supervision(execution, expected.5)
        && execution
            .get("stdout")
            .is_some_and(|value| value.is_string())
        && execution
            .get("stderr")
            .is_some_and(|value| value.is_string())
        && execution
            .get("launch_error")
            .is_some_and(|value| value.is_null())
        && if should_succeed {
            execution.get("success").and_then(|value| value.as_bool()) == Some(true)
                && spawned_exit_code(execution) == Some(0)
        } else {
            spawned_exit_code(execution).is_some_and(|code| code != 0)
                && execution
                    .get("success")
                    .and_then(serde_json::Value::as_bool)
                    == Some(false)
        }
}

fn strict_mutation_supervision(execution: &serde_json::Value, expected_timeout: u64) -> bool {
    let Some(receipt) = StepSupervision::parse(execution) else {
        return false;
    };
    let stdout = execution
        .get("stdout")
        .and_then(serde_json::Value::as_str)
        .map_or(0, |value| value.len() as u64);
    let stderr = execution
        .get("stderr")
        .and_then(serde_json::Value::as_str)
        .map_or(0, |value| value.len() as u64);
    receipt.limits_valid(Some(expected_timeout))
        && receipt.streams_valid(stdout, stderr)
        && !receipt.timed_out
        && !receipt.stdout_truncated
        && !receipt.stderr_truncated
        && receipt.reason == "none"
        && receipt.termination_signal == "none"
        && matches!(receipt.process_group, "verified-empty" | "not-supported")
        && receipt.pipe_cleanup == "complete"
        && receipt.error.is_null()
}

fn validate_mutation_fixtures(
    root: &Path,
    index: &EvidenceIndex,
    authority: &Authority,
) -> Vec<String> {
    let mut contracts = Vec::new();
    let receipt = index
        .entries
        .iter()
        .find(|entry| entry.role == "mutations.receipt")
        .and_then(|entry| read_bundle_file(root, &entry.path).ok())
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok());
    let mut declarations = Vec::new();
    for case in receipt
        .as_ref()
        .and_then(|value| value.get("cases"))
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
    {
        let target = case
            .get("target")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        for (field, phase) in [("baseline_files", "baseline"), ("files", "mutant")] {
            for (position, file) in case
                .get(field)
                .and_then(|value| value.as_array())
                .into_iter()
                .flatten()
                .enumerate()
            {
                declarations.push((target, phase, position, file));
            }
        }
    }
    let entries = index
        .entries
        .iter()
        .filter(|entry| entry.role == "mutation.fixture")
        .collect::<Vec<_>>();
    if declarations.len() != entries.len() {
        contracts.push("evidence.builtin.mutations.fixture.count".to_string());
    }
    let producer = gate_specific_command(index, "mutations");
    for (target, phase, position, declaration) in declarations {
        let expected_prefix = format!("payloads/mutation.fixture/{target}/{phase}/{position}/");
        let path = declaration
            .get("path")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        let matching = entries
            .iter()
            .filter(|entry| {
                entry.path.starts_with(&expected_prefix)
                    && entry.path == path
                    && entry.producing_gate == "mutations"
                    && entry.producing_command == producer
            })
            .collect::<Vec<_>>();
        let valid = matching.len() == 1
            && read_bundle_file(root, path).ok().is_some_and(|bytes| {
                declaration
                    .get("byte_length")
                    .and_then(|value| value.as_u64())
                    == Some(bytes.len() as u64)
                    && declaration.get("sha256").and_then(|value| value.as_str())
                        == Some(hex_sha256(&bytes).as_str())
            });
        if !valid {
            contracts.push(format!(
                "evidence.builtin.mutations.fixture.{target}.{phase}.{position}"
            ));
        }
    }
    if let Some(cases) = receipt
        .as_ref()
        .and_then(|value| value.get("cases"))
        .and_then(|value| value.as_array())
    {
        for case in cases {
            let target = case
                .get("target")
                .and_then(|value| value.as_str())
                .unwrap_or_default();
            if !validate_mutation_semantics(root, case, authority, &index.supported_tuples) {
                contracts.push(format!("evidence.builtin.mutations.semantic.{target}"));
            }
        }
    }
    contracts
}
fn retained_mutation_name<'a>(
    declaration: &'a serde_json::Value,
    target: &str,
    phase: &str,
    position: usize,
) -> Option<&'a str> {
    declaration
        .get("path")
        .and_then(|value| value.as_str())
        .and_then(|path| {
            path.strip_prefix(&format!(
                "payloads/mutation.fixture/{target}/{phase}/{position}/"
            ))
        })
}

fn materialize_artifact_mutation_phase(
    root: &Path,
    case: &serde_json::Value,
    field: &str,
    phase: &str,
) -> Option<(tempfile::TempDir, Vec<Vec<u8>>)> {
    const FILES: [&str; 5] = [
        "evidence-index.json",
        "payloads/authority.snapshot/gates.json",
        "source-preflight.json",
        "tool-preflight.json",
        "cache-initial-state.json",
    ];
    let declarations = case.get(field)?.as_array()?;
    if declarations.len() != FILES.len() {
        return None;
    }
    let fixture = tempfile::tempdir().ok()?;
    let mut retained = Vec::with_capacity(FILES.len());
    for (position, (declaration, expected_name)) in declarations.iter().zip(FILES).enumerate() {
        if retained_mutation_name(declaration, "artifact", phase, position) != Some(expected_name) {
            return None;
        }
        let bytes = declaration
            .get("path")
            .and_then(serde_json::Value::as_str)
            .and_then(|path| read_bundle_file(root, path).ok())?;
        let destination = fixture.path().join(expected_name);
        fs::create_dir_all(destination.parent()?).ok()?;
        fs::write(destination, &bytes).ok()?;
        retained.push(bytes);
    }
    Some((fixture, retained))
}

fn validate_materialized_artifact_fixture(root: &Path) -> Option<Vec<String>> {
    let bytes = fs::read(root.join("evidence-index.json")).ok()?;
    let index: EvidenceIndex = serde_json::from_slice(&bytes).ok()?;
    validate_index_contracts(root, &index).ok()
}

fn validate_artifact_causal_mutation(root: &Path, case: &serde_json::Value) -> bool {
    let Some((baseline_root, baseline)) =
        materialize_artifact_mutation_phase(root, case, "baseline_files", "baseline")
    else {
        return false;
    };
    let Some((mutant_root, mutant)) =
        materialize_artifact_mutation_phase(root, case, "files", "mutant")
    else {
        return false;
    };
    if baseline.len() != mutant.len()
        || baseline[1] != mutant[1]
        || baseline[2] != mutant[2]
        || baseline[4] != mutant[4]
        || baseline[0] == mutant[0]
        || baseline[3] == mutant[3]
    {
        return false;
    }
    let Some((recipe, diagnostic)) = artifact_mutation_metadata(case) else {
        return false;
    };
    if recipe.baseline_sha256 != hex_sha256(&baseline[3])
        || recipe.mutant_sha256 != hex_sha256(&mutant[3])
        || validate_materialized_artifact_fixture(baseline_root.path())
            .is_none_or(|contracts| !contracts.is_empty())
    {
        return false;
    }
    let mutant_rejection =
        validate_materialized_artifact_fixture(mutant_root.path()).is_some_and(|contracts| {
            contracts
                .first()
                .is_some_and(|contract| contract == "evidence.preflight.tools.rust.result")
        });
    let output = case
        .get("executions")
        .and_then(serde_json::Value::as_array)
        .and_then(|executions| executions.last())
        .map(|execution| {
            format!(
                "{}\n{}",
                execution
                    .get("stdout")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default(),
                execution
                    .get("stderr")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
            )
        });
    mutant_rejection
        && output.is_some_and(|output| {
            diagnostic
                .required_fragments
                .iter()
                .all(|fragment| output.contains(fragment))
        })
}

fn validate_causal_mutation(
    root: &Path,
    case: &serde_json::Value,
    target: &str,
    authority: &Authority,
) -> bool {
    if target == "artifact" {
        return validate_artifact_causal_mutation(root, case);
    }
    let Some((recipe, diagnostic, expected_baseline, expected_mutant)) =
        super::mutations::expected_causal_contract(target, authority)
    else {
        return false;
    };
    let Some(baseline_files) = case
        .get("baseline_files")
        .and_then(|value| value.as_array())
    else {
        return false;
    };
    let Some(mutant_files) = case.get("files").and_then(|value| value.as_array()) else {
        return false;
    };
    if baseline_files.len() != mutant_files.len() {
        return false;
    }
    let mut changed = 0;
    for (position, (baseline, mutant)) in baseline_files.iter().zip(mutant_files).enumerate() {
        let baseline_name = retained_mutation_name(baseline, target, "baseline", position);
        let mutant_name = retained_mutation_name(mutant, target, "mutant", position);
        if baseline_name.is_none() || baseline_name != mutant_name {
            return false;
        }
        let baseline_bytes = baseline
            .get("path")
            .and_then(|value| value.as_str())
            .and_then(|path| read_bundle_file(root, path).ok());
        let mutant_bytes = mutant
            .get("path")
            .and_then(|value| value.as_str())
            .and_then(|path| read_bundle_file(root, path).ok());
        let Some((baseline_bytes, mutant_bytes)) = baseline_bytes.zip(mutant_bytes) else {
            return false;
        };
        if baseline_name == Some(recipe.path.as_str()) {
            changed += 1;
            if baseline_bytes != expected_baseline || mutant_bytes != expected_mutant {
                return false;
            }
        } else if baseline_bytes != mutant_bytes {
            return false;
        }
    }
    if changed != 1 {
        return false;
    }
    let Some(execution) = case
        .get("executions")
        .and_then(|value| value.as_array())
        .and_then(|executions| executions.last())
    else {
        return false;
    };
    let output = format!(
        "{}\n{}",
        execution
            .get("stdout")
            .and_then(|value| value.as_str())
            .unwrap_or_default(),
        execution
            .get("stderr")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
    );
    execution.get("success").and_then(|value| value.as_bool()) == Some(false)
        && execution
            .get("launch_error")
            .is_some_and(serde_json::Value::is_null)
        && execution
            .get("signal")
            .is_some_and(serde_json::Value::is_null)
        && spawned_exit_code(execution).is_some_and(|code| code != 0)
        && diagnostic
            .required_fragments
            .iter()
            .all(|fragment| output.contains(fragment))
}

fn validate_mutation_semantics(
    root: &Path,
    case: &serde_json::Value,
    authority: &Authority,
    _tuples: &[String],
) -> bool {
    let target = case
        .get("target")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    MutationTarget::parse(target).is_some()
        && validate_mutation_execution(case, target, authority)
        && validate_causal_mutation(root, case, target, authority)
}

fn validate_failed_collection_receipt(
    root: &Path,
    index: &EvidenceIndex,
    authority: &Authority,
    role: &str,
) -> Vec<String> {
    let mut contracts = Vec::new();
    let (schema, collection, first_failure, item_result) = collection_contract(role);
    let (identity_field, expected_identities) = expected_collection_identities(role, authority);
    let receipt = index
        .entries
        .iter()
        .find(|entry| entry.role == role)
        .and_then(|entry| read_bundle_file(root, &entry.path).ok())
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok());
    let valid = receipt.as_ref().is_some_and(|receipt| {
        let items = receipt.get(collection).and_then(|value| value.as_array());
        let first_observed = items.and_then(|items| {
            items.iter().find_map(|item| {
                (item.get(item_result).and_then(|value| value.as_bool()) == Some(false))
                    .then(|| item.get(identity_field).and_then(|value| value.as_str()))
                    .flatten()
            })
        });
        receipt.get("schema").and_then(|value| value.as_str()) == Some(schema)
            && exact_collection_receipt_fields(role, receipt)
            && (role == "workflow.validation"
                || (receipt.get("passed").and_then(|value| value.as_bool()) == Some(false)
                    && receipt.get("source_sha").and_then(|value| value.as_str())
                        == Some(index.source_sha.as_str())))
            && receipt.get(first_failure).and_then(|value| value.as_str()) == first_observed
            && items.is_some_and(|items| {
                items.len() == expected_identities.len()
                    && items
                        .iter()
                        .zip(&expected_identities)
                        .all(|(item, expected)| {
                            item.get(identity_field).and_then(|value| value.as_str())
                                == Some(*expected)
                                && item
                                    .get(item_result)
                                    .and_then(|value| value.as_bool())
                                    .is_some()
                                && validate_collection_item(role, item, expected, authority)
                        })
            })
            && first_observed.is_some()
    });
    if !valid {
        contracts.push(format!("evidence.builtin.{role}.failed_content"));
    }
    contracts
}

pub fn validate_bytes(entry: &EvidenceEntry, bytes: &[u8]) -> Result<(), String> {
    if (bytes.len() as u64) != entry.byte_length || hex_sha256(bytes) != entry.sha256 {
        return Err(format!(
            "evidence entry {} contradicts its declared size/hash",
            entry.path
        ));
    }
    Ok(())
}

/// Build the detached evidence fixture used by the artifact mutation gate.
///
/// The mutant changes a retained tool observation and then updates the enclosing
/// evidence entry's length and digest. Detached replay must therefore reject the
/// semantic authority correlation rather than stale transport metadata.
pub(crate) fn build_artifact_mutation_fixture(
    root: &Path,
    authority: &Authority,
    authority_bytes: &[u8],
    mutant: bool,
) -> Result<PathBuf, String> {
    let controller = vec![
        "uqm-xtask".to_string(),
        "ci".to_string(),
        "run".to_string(),
        "format".to_string(),
    ];
    let source_sha = "a".repeat(40);
    let tuples = authority
        .runner_mapping
        .iter()
        .map(|mapping| mapping.tuple.clone())
        .collect::<Vec<_>>();
    let mut entries = Vec::new();

    write_artifact_fixture_entry(
        root,
        &mut entries,
        authority,
        "payloads/authority.snapshot/gates.json",
        "authority.snapshot",
        &controller,
        authority_bytes,
    )?;
    let source = serde_json::json!({
        "schema": "uqm-s4-source-preflight-v2",
        "source_sha": source_sha,
        "detached_state": null,
        "expected_sha": null,
        "base_sha": null,
        "tuple": "linux-x86_64",
        "expected_tuple": null,
        "cache_mode": "ambient-dev",
        "clean": false,
        "canonical_environment": false,
        "passed": false,
        "first_failed_contract": "source.clean",
        "detail": "preflight rejected"
    });
    write_artifact_fixture_entry(
        root,
        &mut entries,
        authority,
        "source-preflight.json",
        "preflight.source",
        &controller,
        &serde_json::to_vec(&source).map_err(|error| error.to_string())?,
    )?;

    let mut observations = authority
        .tools
        .preflight
        .iter()
        .map(|probe| {
            serde_json::json!({
                "name": probe.name,
                "command": probe.version_command,
                "expected_output_prefix": probe.expected_output_prefix,
                "executable_identity": artifact_fixture_executable(&probe.name),
                "stdout": probe.expected_output_prefix.as_deref().map_or_else(
                    || "available".to_string(),
                    |prefix| format!("{prefix}fixture"),
                ),
                "stderr": "",
                "exit_code": 0,
                "signal": null,
                "launch_error": null,
                "passed": true
            })
        })
        .collect::<Vec<_>>();
    observations.extend(authority.tools.entries().into_iter().map(|(name, tool)| {
        serde_json::json!({
            "name": name,
            "command": tool.version_command,
            "expected_output_prefix": tool.expected_output_prefix,
            "executable_identity": artifact_fixture_executable(name),
            "stdout": if mutant && name == "rust" {
                "coherently rehashed forged tool output".to_string()
            } else {
                format!("{}fixture", tool.expected_output_prefix)
            },
            "stderr": "",
            "exit_code": 0,
            "signal": null,
            "launch_error": null,
            "passed": true
        })
    }));
    let tools = serde_json::json!({
        "schema": "uqm-s4-tool-preflight-v2",
        "passed": true,
        "observations": observations
    });
    write_artifact_fixture_entry(
        root,
        &mut entries,
        authority,
        "tool-preflight.json",
        "preflight.tools",
        &controller,
        &serde_json::to_vec(&tools).map_err(|error| error.to_string())?,
    )?;
    let cache = serde_json::json!({
        "schema": "uqm-s4-cache-initial-state-v1",
        "mode": "ambient-dev",
        "ambient_cargo_home": "/tmp/cargo-home",
        "isolation_cargo_home": "/tmp/cargo-home",
        "execution_target": "",
        "registry_cache_present": false,
        "git_cache_present": false,
        "execution_target_absent": true,
        "rust_target_present": false,
        "sc2_obj_present": false,
        "restore_used": false,
        "save_used": false,
        "first_failed_contract": null,
        "passed": true
    });
    write_artifact_fixture_entry(
        root,
        &mut entries,
        authority,
        "cache-initial-state.json",
        "cache.initial-state",
        &controller,
        &serde_json::to_vec(&cache).map_err(|error| error.to_string())?,
    )?;

    let index = EvidenceIndex::build_and_validate(
        root,
        &tuples,
        EvidenceContext {
            source_sha,
            clean: false,
            tuple: "linux-x86_64".to_string(),
            features: vec!["audio_heart".to_string()],
            cache_mode: "ambient-dev".to_string(),
            first_failed_contract: Some("source.clean".to_string()),
        },
        entries,
    )?;
    let index_path = root.join("evidence-index.json");
    fs::write(
        &index_path,
        serde_json::to_vec_pretty(&index).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("write artifact mutation evidence index: {error}"))?;
    Ok(index_path)
}

fn artifact_fixture_executable(name: &str) -> serde_json::Value {
    serde_json::json!({
        "path": format!("/usr/bin/{name}"),
        "byte_length": 1,
        "sha256": "c".repeat(64),
        "mode": 0o755
    })
}

fn write_artifact_fixture_entry(
    root: &Path,
    entries: &mut Vec<EvidenceEntry>,
    authority: &Authority,
    relative: &str,
    role: &str,
    command: &[String],
    bytes: &[u8],
) -> Result<(), String> {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create artifact mutation fixture: {error}"))?;
    }
    fs::write(&path, bytes)
        .map_err(|error| format!("write artifact mutation fixture {relative}: {error}"))?;
    let media_type = authority
        .evidence_roles
        .iter()
        .find_map(|candidate| (candidate.role == role).then_some(candidate.media_type.as_str()))
        .ok_or_else(|| format!("authority has no media type for artifact fixture role {role}"))?;
    entries.push(
        entry(root, relative, role, media_type, "format", command)
            .map_err(|error| error.to_string())?,
    );
    Ok(())
}
fn entry(
    root: &Path,
    relative: &str,
    role: &str,
    mime: &str,
    producing_gate: &str,
    producing_command: &[String],
) -> Result<EvidenceEntry, CiError> {
    let bytes = read_bundle_file(root, relative).map_err(|error| {
        CiError::new(
            "evidence.read",
            format!("cannot read {}: {error}", root.join(relative).display()),
        )
    })?;
    entry_from_bytes(
        relative,
        &bytes,
        role,
        mime,
        producing_gate,
        producing_command,
    )
}

pub(crate) fn entry_from_bytes(
    relative: &str,
    bytes: &[u8],
    role: &str,
    mime: &str,
    producing_gate: &str,
    producing_command: &[String],
) -> Result<EvidenceEntry, CiError> {
    if !validate_relative_path(relative) {
        return Err(CiError::new(
            "evidence.path",
            format!("invalid relative evidence path '{relative}'"),
        ));
    }
    if !valid_role_mime(role, mime) {
        return Err(CiError::new(
            "evidence.role_mime_contract",
            format!("unsupported evidence role/MIME pair '{role}'/'{mime}'"),
        ));
    }
    Ok(EvidenceEntry {
        schema: ENTRY_SCHEMA.to_string(),
        role: role.to_string(),
        path: relative.to_string(),
        mime: mime.to_string(),
        byte_length: bytes.len() as u64,
        sha256: hex_sha256(bytes),
        producing_gate: producing_gate.to_string(),
        producing_command: producing_command.to_vec(),
    })
}

fn valid_role_mime(role: &str, mime: &str) -> bool {
    !role.is_empty()
        && !role.chars().any(char::is_whitespace)
        && mime
            .split_once('/')
            .is_some_and(|(kind, subtype)| !kind.is_empty() && !subtype.is_empty())
}

/// Validate that a path is normalized relative UTF-8 without traversal.
fn validate_absolute_path(path: &str) -> bool {
    let mut components = Path::new(path).components();
    matches!(components.next(), Some(std::path::Component::RootDir))
        && components.all(|component| matches!(component, std::path::Component::Normal(_)))
}

pub fn validate_relative_path(path: &str) -> bool {
    if path.is_empty()
        || path.contains('\\')
        || path.starts_with('/')
        || path.starts_with("./")
        || path.contains("..")
        || path.ends_with('/')
    {
        return false;
    }
    Path::new(path)
        .components()
        .all(|component| matches!(component, std::path::Component::Normal(_)))
}

pub fn hex_sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn exact_json_fields(value: &serde_json::Value, fields: &[&str]) -> bool {
    value.as_object().is_some_and(|object| {
        object.len() == fields.len() && fields.iter().all(|field| object.contains_key(*field))
    })
}

pub fn is_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_and_authority_validation_share_one_immutable_snapshot() {
        let authority: Authority =
            serde_json::from_str(include_str!("../../../ci/gates.json")).unwrap();
        let authority_bytes = serde_json::to_vec_pretty(&authority).unwrap();
        let fixture = tempfile::tempdir().unwrap();
        let index_path =
            build_artifact_mutation_fixture(fixture.path(), &authority, &authority_bytes, false)
                .unwrap();
        let value = serde_json::from_slice(&fs::read(&index_path).unwrap()).unwrap();
        let authority_path = fixture
            .path()
            .join("payloads/authority.snapshot/gates.json");
        let mut replaced = false;
        ADVERSARIAL_HOOK.with(|slot| {
            *slot.borrow_mut() = Some(Box::new(move |event| {
                if event == "index-before-authority-validation" && !replaced {
                    fs::write(&authority_path, b"{}\n").unwrap();
                    replaced = true;
                }
            }));
        });
        let result = validate_index_command(value, &index_path);
        ADVERSARIAL_HOOK.with(|slot| *slot.borrow_mut() = None);
        assert!(result.is_ok(), "snapshot validation failed: {result:?}");
        assert_eq!(
            fs::read(
                fixture
                    .path()
                    .join("payloads/authority.snapshot/gates.json")
            )
            .unwrap(),
            b"{}\n"
        );
    }

    fn sample_entry(path: &str, role: &str, payload: &[u8], gate: &str) -> EvidenceEntry {
        EvidenceEntry {
            schema: ENTRY_SCHEMA.to_string(),
            role: role.to_string(),
            path: path.to_string(),
            mime: "text/plain".to_string(),
            byte_length: payload.len() as u64,
            sha256: hex_sha256(payload),
            producing_gate: gate.to_string(),
            producing_command: vec!["uqm-xtask".into(), "ci".into(), "run".into(), gate.into()],
        }
    }

    fn snapshot_limits(
        member_count: u64,
        member_bytes: u64,
        aggregate_bytes: u64,
        path_bytes: u64,
        aggregate_path_bytes: u64,
    ) -> SnapshotLimits {
        SnapshotLimits {
            member_count,
            member_bytes,
            aggregate_bytes,
            path_bytes,
            aggregate_path_bytes,
        }
    }

    #[test]
    fn snapshot_budget_rejects_each_authority_owned_resource_limit() {
        let mut member_count = SnapshotBudget::new(snapshot_limits(1, 8, 16, 8, 16));
        member_count.admit_file(1).unwrap();
        assert!(member_count.admit_file(1).is_err());

        let mut path_length = SnapshotBudget::new(snapshot_limits(2, 8, 16, 1, 16));
        assert!(path_length.admit_path("ab").is_err());

        let mut aggregate_paths = SnapshotBudget::new(snapshot_limits(2, 8, 16, 8, 1));
        aggregate_paths.admit_path("a").unwrap();
        assert!(aggregate_paths.admit_path("b").is_err());

        let mut member_length = SnapshotBudget::new(snapshot_limits(2, 1, 16, 8, 16));
        assert!(member_length.admit_file(2).is_err());

        let mut aggregate_length = SnapshotBudget::new(snapshot_limits(2, 8, 3, 8, 16));
        aggregate_length.admit_file(2).unwrap();
        assert!(aggregate_length.admit_file(2).is_err());
    }

    #[test]
    fn snapshot_budget_rejects_counter_overflow() {
        let limits = snapshot_limits(u64::MAX, u64::MAX, u64::MAX, u64::MAX, u64::MAX);

        let mut members = SnapshotBudget::new(limits);
        members.member_count = u64::MAX;
        assert!(members.admit_file(0).is_err());

        let mut contents = SnapshotBudget::new(limits);
        contents.aggregate_bytes = u64::MAX;
        assert!(contents.admit_file(1).is_err());

        let mut paths = SnapshotBudget::new(limits);
        paths.aggregate_path_bytes = u64::MAX;
        assert!(paths.admit_path("a").is_err());
    }

    #[test]
    fn opened_snapshot_enforces_limits_before_retaining_members() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("a"), b"1234").unwrap();
        fs::write(root.path().join("b"), b"56").unwrap();
        let opened = OpenedRoot::open(root.path()).unwrap();

        let snapshot = opened
            .snapshot_files(snapshot_limits(2, 4, 6, 1, 2))
            .unwrap();
        assert_eq!(snapshot.0.len(), 2);
        assert!(opened
            .snapshot_files(snapshot_limits(2, 3, 6, 1, 2))
            .is_err());
        assert!(opened
            .snapshot_files(snapshot_limits(2, 4, 5, 1, 2))
            .is_err());
        assert!(opened
            .snapshot_files(snapshot_limits(1, 4, 6, 1, 2))
            .is_err());
    }

    fn fixture_tuples() -> Vec<String> {
        let bytes = include_bytes!("../../../build/supported-matrix.json");
        let matrix: super::super::authority::Matrix = serde_json::from_slice(bytes).unwrap();
        matrix.derive_contract_tuples().unwrap()
    }

    fn native_runtime_contract() -> uqm_rust::automation::native_window::NativeWindowRuntimeContract
    {
        let authority: Authority =
            serde_json::from_str(include_str!("../../../ci/gates.json")).unwrap();
        authority.native_runtime_contract()
    }

    fn native_acceptance_policy() -> uqm_rust::automation::native_window::NativeAcceptancePolicy {
        let authority: Authority =
            serde_json::from_str(include_str!("../../../ci/gates.json")).unwrap();
        authority.native_acceptance.acceptance_policy
    }
    fn valid_pre_session_envelope(contract: &str) -> PreSessionFailureEnvelope {
        let tuple = if contract == "environment.tuple" {
            "unsupported-x86_64"
        } else {
            "macos-aarch64"
        };
        let authority_snapshot = if contract == "authority.load" {
            None
        } else {
            Some(include_str!("../../../ci/gates.json").to_string())
        };
        let mut envelope = PreSessionFailureEnvelope {
            schema: PRE_SESSION_SCHEMA.to_string(),
            passed: false,
            first_failed_contract: contract.to_string(),
            detail: "fixture pre-session failure".to_string(),
            requested_gate: "format".to_string(),
            tuple: tuple.to_string(),
            cache_mode: "isolated-empty".to_string(),
            configured_evidence_root: Some("/tmp/evidence/bundle".to_string()),
            authority_snapshot,
            controller_command: vec![
                "/tmp/uqm-xtask".to_string(),
                "ci".to_string(),
                "run".to_string(),
                "format".to_string(),
            ],
            offline_validation: OfflineValidation {
                passed: false,
                contracts: Vec::new(),
            },
        };
        envelope.offline_validation = pre_session_validation(&envelope);
        envelope
    }

    #[test]
    fn pre_session_envelope_validates_without_a_repository() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join(PRE_SESSION_FILENAME);
        let envelope = valid_pre_session_envelope("plan.derive");
        assert!(envelope.offline_validation.passed);
        fs::write(&path, serde_json::to_vec_pretty(&envelope).unwrap()).unwrap();
        validate_evidence_command(
            Path::new("/definitely-not-a-repository"),
            path.to_str().unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn pre_session_envelope_rejects_forged_contract_authority_and_command() {
        let mut envelope = valid_pre_session_envelope("plan.derive");
        envelope.first_failed_contract = "unknown.failure".to_string();
        assert!(pre_session_validation(&envelope)
            .contracts
            .contains(&"pre_session.first_failed_contract".to_string()));

        let mut envelope = valid_pre_session_envelope("plan.derive");
        let mut authority: serde_json::Value =
            serde_json::from_str(envelope.authority_snapshot.as_deref().unwrap()).unwrap();
        authority["schema"] = serde_json::json!("forged-authority");
        envelope.authority_snapshot = Some(serde_json::to_string(&authority).unwrap());
        assert!(pre_session_validation(&envelope)
            .contracts
            .contains(&"pre_session.authority.invalid".to_string()));

        let mut envelope = valid_pre_session_envelope("plan.derive");
        envelope.controller_command[3] = "check".to_string();
        assert!(pre_session_validation(&envelope)
            .contracts
            .contains(&"pre_session.controller_command".to_string()));
    }

    #[test]
    fn pre_session_envelope_rejects_unknown_fields() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join(PRE_SESSION_FILENAME);
        let mut value = serde_json::to_value(valid_pre_session_envelope("plan.derive")).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .insert("forged".to_string(), serde_json::json!(true));
        fs::write(&path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
        let error =
            validate_evidence_command(Path::new("/unused"), path.to_str().unwrap()).unwrap_err();
        assert!(error.contains("unknown field"), "{error}");
    }

    #[test]
    fn pre_session_writer_publishes_atomically_without_overwrite() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("pre-session-run-0");
        let path = write_pre_session_failure(
            &root,
            &destination,
            "format",
            "plan.derive",
            "fixture failure",
        )
        .unwrap();
        let original = fs::read(&path).unwrap();
        let error = write_pre_session_failure(
            &root,
            &destination,
            "format",
            "plan.derive",
            "replacement failure",
        )
        .unwrap_err();
        assert!(
            error.contains("cannot publish pre-session evidence"),
            "{error}"
        );
        assert_eq!(fs::read(path).unwrap(), original);
    }

    /// An index that passes every contract check against a temp root.
    fn valid_index() -> EvidenceIndex {
        EvidenceIndex {
            schema: EVIDENCE_SCHEMA.to_string(),
            source_sha: "a".repeat(40),
            clean: true,
            tuple: "linux-x86_64".to_string(),
            supported_tuples: fixture_tuples(),
            profile: PROFILE.to_string(),
            features: vec!["audio_heart".to_string()],
            cache_mode: "isolated-empty".to_string(),

            first_failed_contract: None,
            offline_validation: OfflineValidation {
                passed: false,
                contracts: Vec::new(),
            },
            entries: vec![sample_entry(
                "artifact.log",
                "step.stdout",
                b"hello",
                "format",
            )],
        }
    }

    fn retain_fixture_authority(root: &Path, index: &mut EvidenceIndex) {
        let bytes = include_bytes!("../../../ci/gates.json");
        let relative = "payloads/authority.snapshot/gates.json";
        let path = root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
        index.entries.push(sample_entry(
            relative,
            "authority.snapshot",
            bytes,
            "format",
        ));
    }

    fn write_bundle_entry(
        root: &Path,
        entries: &mut Vec<EvidenceEntry>,
        path: &str,
        role: &str,
        command: &[String],
        value: &[u8],
    ) {
        let full = root.join(path);
        fs::create_dir_all(full.parent().unwrap()).unwrap();
        fs::write(&full, value).unwrap();
        let authority: Authority =
            serde_json::from_str(include_str!("../../../ci/gates.json")).unwrap();
        let mime = authority
            .evidence_roles
            .iter()
            .find_map(|candidate| (candidate.role == role).then_some(candidate.media_type.as_str()))
            .unwrap();
        entries.push(entry(root, path, role, mime, "format", command).unwrap());
    }

    fn subordinate_fixture_bytes(step: &str, name: &str) -> Vec<u8> {
        if name.ends_with("-nm.exit.txt") {
            return b"0\n".to_vec();
        }
        if name.ends_with("-nm.stderr.txt") {
            return Vec::new();
        }
        match (step, name) {
            ("p00-probes", "p00-probe-results.log") => [
                "PASS lock_free_atomics: fixture",
                "PASS monotonic_instant: fixture",
                "PASS unix_datagram: fixture",
                "PASS file_primitives: fixture",
                "PASS process_identity: fixture",
                "PASS sdl_dummy_hidden: fixture",
                "",
                "P00 probes: 6 passed, 0 failed",
            ]
            .join("\n")
            .into_bytes(),
            ("p00-harness", "archive-nm-origins.txt") => [
                "DoInput\tlibuqm_c.a(input.c.o):\tlibuqm_c.a(input.c.o): 000 T DoInput",
                "AnyButtonPress\tlibuqm_c.a(input.c.o):\tlibuqm_c.a(input.c.o): 000 T AnyButtonPress",
                "DoConfirmExit\tlibuqm_c.a(confirm.c.o):\tlibuqm_c.a(confirm.c.o): 000 T DoConfirmExit",
                "TFB_ProcessEvents\tlibuqm_c.a(events.c.o):\tlibuqm_c.a(events.c.o): 000 T TFB_ProcessEvents",
                "TFB_SwapBuffers\tlibuqm_c.a(gfx.c.o):\tlibuqm_c.a(gfx.c.o): 000 T TFB_SwapBuffers",
                "ProcessInputEvent\tlibuqm_c.a(input.c.o):\tlibuqm_c.a(input.c.o): 000 T ProcessInputEvent",
                "TFB_FlushGraphicsEx\tlibuqm_c.a(gfx.c.o):\tlibuqm_c.a(gfx.c.o): 000 T TFB_FlushGraphicsEx",
            ]
            .join("\n")
            .into_bytes(),
            ("menu-binding-probe", "c-archive-nm-origins.txt") => [
                "VControl_ParseGesture\trust_vcontrol_impl.c.o\tlibuqm_c.a(rust_vcontrol_impl.c.o): 000 T VControl_ParseGesture",
                "InstallGraphicResTypes\tresgfx.c.o\tlibuqm_c.a(resgfx.c.o): 000 T InstallGraphicResTypes",
                "InstallStringTableResType\tsresins.c.o\tlibuqm_c.a(sresins.c.o): 000 T InstallStringTableResType",
            ]
            .join("\n")
            .into_bytes(),
            ("menu-binding-probe", "rust-archive-nm-origins.txt") => [
                "InitResourceSystem\t\tlibuqm_rust.a(resource.o): 000 T InitResourceSystem",
                "LoadResourceIndex\t\tlibuqm_rust.a(resource.o): 000 T LoadResourceIndex",
                "res_IsString\t\tlibuqm_rust.a(resource.o): 000 T res_IsString",
                "res_GetString\t\tlibuqm_rust.a(resource.o): 000 T res_GetString",
                "uio_openRepository\t\tlibuqm_rust.a(uio.o): 000 T uio_openRepository",
                "uio_mountDir\t\tlibuqm_rust.a(uio.o): 000 T uio_mountDir",
                "uio_openDir\t\tlibuqm_rust.a(uio.o): 000 T uio_openDir",
            ]
            .join("\n")
            .into_bytes(),
            ("menu-binding-probe", "harness-archive-nm-origins.txt") => b"uqm_query_menu_binding\tmenu_binding_accessor.o\tlibp00_harness_shim.a(menu_binding_accessor.o): 000 T uqm_query_menu_binding\n".to_vec(),
            ("menu-binding-probe", "probe-output.txt") => {
                b"RESULT=PASS\nfound=1\nbinding_type=VCONTROL_KEY\nkey_code=1073741905\nbinding_id=1\nnum_alternates=1\n".to_vec()
            }
            _ => b"retained subordinate output\n".to_vec(),
        }
    }

    fn write_step_subordinate_fixture(
        root: &Path,
        index: &mut EvidenceIndex,
        gate: &super::super::authority::Gate,
        step: &super::super::authority::Step,
    ) {
        for name in subordinate_output_names(&gate.id, &step.id) {
            let path = format!(
                "payloads/subordinate.output/{}/{}/{}",
                gate.id, step.id, name
            );
            let bytes = subordinate_fixture_bytes(&step.id, name);
            write_bundle_entry(
                root,
                &mut index.entries,
                &path,
                "subordinate.output",
                &step.command,
                &bytes,
            );
            index.entries.last_mut().unwrap().producing_gate = gate.id.clone();
        }
    }
    fn production_contracts(root: &Path, index: &EvidenceIndex) -> Vec<String> {
        let mut contracts = validate_index(root, &index.supported_tuples, index).unwrap();
        contracts.extend(validate_authority_snapshot(root, index));
        contracts
    }
    fn ambient_cache_receipt() -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "schema": "uqm-s4-cache-initial-state-v1",
            "mode": "ambient-dev",
            "ambient_cargo_home": "/tmp/cargo-home",
            "isolation_cargo_home": "/tmp/cargo-home",
            "execution_target": "",
            "registry_cache_present": false,
            "git_cache_present": false,
            "execution_target_absent": true,
            "rust_target_present": false,
            "sc2_obj_present": false,
            "restore_used": false,
            "save_used": false,
            "first_failed_contract": null,
            "passed": true
        }))
        .unwrap()
    }

    fn preflight_failure_bundle(root: &Path, contract: &str) -> EvidenceIndex {
        let controller = vec![
            "uqm-xtask".into(),
            "ci".into(),
            "run".into(),
            "format".into(),
        ];
        let source_sha = "a".repeat(40);
        let mut entries = Vec::new();
        let authority_bytes = include_bytes!("../../../ci/gates.json");
        let authority: Authority = serde_json::from_slice(authority_bytes).unwrap();
        write_bundle_entry(
            root,
            &mut entries,
            "payloads/authority.snapshot/gates.json",
            "authority.snapshot",
            &controller,
            authority_bytes,
        );
        let source_failed = is_source_preflight_failure(contract);
        let source = serde_json::json!({
            "schema": "uqm-s4-source-preflight-v2",
            "source_sha": source_sha,
            "detached_state": null,
            "expected_sha": if contract == "source.expected_sha" { Some("b".repeat(40)) } else { None },
            "base_sha": null,
            "tuple": "linux-x86_64",
            "expected_tuple": if contract == "source.expected_tuple" { Some("linux-aarch64") } else { None },
            "cache_mode": "ambient-dev",
            "clean": contract != "source.clean",
            "canonical_environment": false,
            "passed": !source_failed,
            "first_failed_contract": source_failed.then_some(contract),
            "detail": source_failed.then_some("preflight rejected")
        });
        write_bundle_entry(
            root,
            &mut entries,
            "source-preflight.json",
            "preflight.source",
            &controller,
            &serde_json::to_vec(&source).unwrap(),
        );
        let tool_failed = contract == "tools.preflight";
        let mut observations: Vec<_> = authority
            .tools
            .preflight
            .iter()
            .map(|probe| {
                serde_json::json!({
                    "name": probe.name,
                    "command": probe.version_command,
                    "expected_output_prefix": probe.expected_output_prefix,
                    "executable_identity": {
                        "path": format!("/usr/bin/{}", probe.name),
                        "byte_length": 1,
                        "sha256": "c".repeat(64),
                        "mode": 0o755
                    },
                    "stdout": probe.expected_output_prefix.as_deref().map_or_else(|| "available".to_string(), |prefix| format!("{prefix}fixture")),
                    "stderr": "",
                    "exit_code": 0,
                    "signal": null,
                    "launch_error": null,
                    "passed": true
                })
            })
            .collect();
        observations.extend(authority.tools.entries().into_iter().enumerate().map(
            |(position, (name, tool))| {
                let passed = !(tool_failed && position == 0);
                serde_json::json!({
                    "name": name,
                    "command": tool.version_command,
                    "expected_output_prefix": tool.expected_output_prefix,
                    "executable_identity": {
                        "path": format!("/usr/bin/{name}"),
                        "byte_length": 1,
                        "sha256": "c".repeat(64),
                        "mode": 0o755
                    },
                    "stdout": if passed { format!("{}fixture", tool.expected_output_prefix) } else { "wrong version".to_string() },
                    "stderr": "",
                    "exit_code": if passed { 0 } else { 1 },
                    "signal": null,
                    "launch_error": null,
                    "passed": passed
                })
            },
        ));
        let tools = serde_json::json!({
            "schema": "uqm-s4-tool-preflight-v2",
            "passed": !tool_failed,
            "observations": observations
        });
        write_bundle_entry(
            root,
            &mut entries,
            "tool-preflight.json",
            "preflight.tools",
            &controller,
            &serde_json::to_vec(&tools).unwrap(),
        );
        write_bundle_entry(
            root,
            &mut entries,
            "cache-initial-state.json",
            "cache.initial-state",
            &controller,
            &ambient_cache_receipt(),
        );
        EvidenceIndex::build_and_validate(
            root,
            &fixture_tuples(),
            EvidenceContext {
                source_sha,
                clean: contract != "source.clean",
                tuple: "linux-x86_64".into(),
                features: vec!["audio_heart".into()],
                cache_mode: "ambient-dev".into(),
                first_failed_contract: Some(contract.into()),
            },
            entries,
        )
        .unwrap()
    }

    fn all_gates_preflight_failure_bundle(root: &Path, contract: &str) -> EvidenceIndex {
        let mut index = preflight_failure_bundle(root, contract);
        for entry in &mut index.entries {
            entry.producing_command[3] = "all".to_string();
        }
        let mut contracts = validate_index(root, &fixture_tuples(), &index).unwrap();
        contracts.extend(validate_authority_snapshot(root, &index));
        index.with_validation(contracts)
    }

    fn rewrite_bundle_entry(root: &Path, entries: &mut [EvidenceEntry], role: &str, value: &[u8]) {
        let entry = entries.iter_mut().find(|entry| entry.role == role).unwrap();
        fs::write(root.join(&entry.path), value).unwrap();
        entry.byte_length = value.len() as u64;
        entry.sha256 = hex_sha256(value);
    }

    fn rewrite_bundle_path(root: &Path, entries: &mut [EvidenceEntry], path: &str, value: &[u8]) {
        let entry = entries.iter_mut().find(|entry| entry.path == path).unwrap();
        fs::write(root.join(path), value).unwrap();
        entry.byte_length = value.len() as u64;
        entry.sha256 = hex_sha256(value);

        let stream = path
            .strip_suffix(".stdout.log")
            .map(|base| (base, "stdout_bytes_seen"))
            .or_else(|| {
                path.strip_suffix(".stderr.log")
                    .map(|base| (base, "stderr_bytes_seen"))
            });
        let Some((base, field)) = stream else {
            return;
        };
        let result_path = format!("{base}.result.json");
        let Some(result_entry) = entries.iter_mut().find(|entry| entry.path == result_path) else {
            return;
        };
        let mut result: serde_json::Value =
            serde_json::from_slice(&fs::read(root.join(&result_path)).unwrap()).unwrap();
        result["supervision"][field] = serde_json::json!(value.len());
        let bytes = serde_json::to_vec(&result).unwrap();
        fs::write(root.join(&result_path), &bytes).unwrap();
        result_entry.byte_length = bytes.len() as u64;
        result_entry.sha256 = hex_sha256(&bytes);
    }

    fn rewrite_lcar_artifact(
        root: &Path,
        entries: &mut [EvidenceEntry],
        lcar: &mut serde_json::Value,
        path: &str,
        value: &[u8],
    ) {
        let bundle_path = format!("payloads/bootstrap-proof.lcar-artifact/{path}");
        let entry = entries
            .iter_mut()
            .find(|entry| entry.path == bundle_path)
            .unwrap();
        fs::write(root.join(&entry.path), value).unwrap();
        entry.byte_length = value.len() as u64;
        entry.sha256 = hex_sha256(value);
        let artifact = lcar["artifacts"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|artifact| artifact.get("path").and_then(|value| value.as_str()) == Some(path))
            .unwrap();
        artifact["bytes"] = serde_json::json!(value.len());
        artifact["sha256"] = serde_json::json!(hex_sha256(value));
    }
    fn tampered_delta_contracts(mutate: impl FnOnce(&mut serde_json::Value)) -> Vec<String> {
        let bundle = tempfile::tempdir().unwrap();
        let mut index = setup_failure_bundle(bundle.path(), "ownership.zero_native_delta");
        let delta_path = bundle.path().join(
            &index
                .entries
                .iter()
                .find(|entry| entry.role == "ownership.zero-native-delta")
                .unwrap()
                .path,
        );
        let mut report: serde_json::Value =
            serde_json::from_slice(&fs::read(delta_path).unwrap()).unwrap();
        mutate(&mut report);
        rewrite_bundle_entry(
            bundle.path(),
            &mut index.entries,
            "ownership.zero-native-delta",
            &serde_json::to_vec(&report).unwrap(),
        );
        production_contracts(bundle.path(), &index)
    }

    fn fixture_supervision(
        launch_failed: bool,
        stdout_bytes: u64,
        stderr_bytes: u64,
    ) -> serde_json::Value {
        serde_json::json!({
            "timeout_milliseconds": 3_600_000,
            "termination_grace_milliseconds": 1_000,
            "pipe_drain_timeout_milliseconds": 1_000,
            "stdout_limit_bytes": 4_194_304,
            "stderr_limit_bytes": 4_194_304,
            "stdout_bytes_seen": stdout_bytes,
            "stderr_bytes_seen": stderr_bytes,
            "stdout_truncated": false,
            "stderr_truncated": false,
            "timed_out": false,
            "termination_reason": "none",
            "termination_signal": "none",
            "process_group_cleanup": if launch_failed { "not-started" } else { "verified-empty" },
            "pipe_cleanup": if launch_failed { "not-started" } else { "complete" },
            "error": null,
        })
    }

    fn fixture_detached_state() -> serde_json::Value {
        serde_json::json!({
            "schema": "uqm-s4-detached-state-v1",
            "command": ["git", "-c", "safe.directory=/checkout", "symbolic-ref", "-q", "HEAD"],
            "exit_code": 1,
            "signal": null,
            "launch_error": null,
            "success": false,
            "stdout": "",
            "stderr": "",
            "supervision": fixture_supervision(false, 0, 0),
        })
    }

    fn fixture_executable_identity(launch_error: Option<&str>) -> serde_json::Value {
        if launch_error.is_some() {
            serde_json::Value::Null
        } else {
            serde_json::json!({
                "path": "/tmp/uqm-bound-tool",
                "byte_length": 1,
                "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "mode": 0o755,
            })
        }
    }

    fn fixture_execution_provenance(command: &[String]) -> (Vec<String>, Option<String>) {
        if let Some(hidden_command) = super::super::run::trusted_controller_command(command) {
            return (
                vec![
                    "/tmp/trusted-control-plane/uqm-controller".into(),
                    hidden_command.into(),
                ],
                None,
            );
        }
        match super::super::run::trusted_control_plane_script(command) {
            Some((script_name, script_bytes)) => (
                vec![
                    command[0].clone(),
                    format!("/tmp/trusted-control-plane/{script_name}"),
                ],
                Some(hex_sha256(script_bytes)),
            ),
            None => (command.to_vec(), None),
        }
    }
    fn write_builtin_step_fixture(
        root: &Path,
        entries: &mut Vec<EvidenceEntry>,
        gate: &str,
        step: &str,
        command: &[String],
        terminal: (Option<i64>, Option<i64>, Option<&str>),
    ) {
        let (exit_code, signal, launch_error) = terminal;

        let success = exit_code == Some(0) && signal.is_none() && launch_error.is_none();
        let (effective_command, staged_script_sha256) = fixture_execution_provenance(command);
        let result = serde_json::to_vec(&serde_json::json!({
            "schema": "uqm-s4-step-result-v2",
            "gate": gate,
            "step": step,
            "command": command,
            "effective_command": effective_command,
            "staged_script_sha256": staged_script_sha256,
            "executable_identity": fixture_executable_identity(launch_error),
            "success": success,
            "exit_code": exit_code,
            "signal": signal,
            "launch_error": launch_error,
            "supervision": fixture_supervision(launch_error.is_some(), 0, 0),
        }))
        .unwrap();
        for (suffix, role, bytes) in [
            ("stdout.log", "step.stdout", Vec::new()),
            ("stderr.log", "step.stderr", Vec::new()),
            ("result.json", "step.result", result),
        ] {
            write_bundle_entry(
                root,
                entries,
                &format!("{gate}/{step}.{suffix}"),
                role,
                command,
                &bytes,
            );
            entries.last_mut().unwrap().producing_gate = gate.to_string();
        }
    }

    #[test]
    fn step_execution_provenance_binds_effective_argv_and_staged_script_digest() {
        let embedded = vec![
            "bash".to_string(),
            "rust/ownership/verify-fixture.sh".to_string(),
        ];
        let (effective_command, staged_script_sha256) = fixture_execution_provenance(&embedded);
        let valid = serde_json::json!({
            "effective_command": effective_command,
            "staged_script_sha256": staged_script_sha256,
        });
        assert!(valid_step_execution_provenance(&valid, &embedded));

        for mutant in [
            serde_json::json!({
                "effective_command": ["bash", "/tmp/trusted-control-plane/verify-fixture.sh"],
                "staged_script_sha256": null,
            }),
            serde_json::json!({
                "effective_command": ["bash", "/tmp/trusted-control-plane/verify-fixture.sh"],
                "staged_script_sha256": "0".repeat(64),
            }),
            serde_json::json!({
                "effective_command": ["bash", "/tmp/trusted-control-plane/forged.sh"],
                "staged_script_sha256": valid["staged_script_sha256"],
            }),
            serde_json::json!({
                "effective_command": ["bash", "verify-fixture.sh"],
                "staged_script_sha256": valid["staged_script_sha256"],
            }),
        ] {
            assert!(!valid_step_execution_provenance(&mutant, &embedded));
        }

        let controller = [
            "cargo",
            "run",
            "--locked",
            "--manifest-path",
            "rust/xtask/Cargo.toml",
            "--",
            "test",
        ]
        .map(str::to_string);
        let hidden = serde_json::json!({
            "effective_command": ["/tmp/uqm-controller", "__ci-test"],
            "staged_script_sha256": null,
        });
        assert!(valid_step_execution_provenance(&hidden, &controller));
        for mutant in [
            serde_json::json!({
                "effective_command": controller,
                "staged_script_sha256": null,
            }),
            serde_json::json!({
                "effective_command": ["/tmp/uqm-controller", "__ci-native-test"],
                "staged_script_sha256": null,
            }),
            serde_json::json!({
                "effective_command": ["uqm-controller", "__ci-test"],
                "staged_script_sha256": null,
            }),
            serde_json::json!({
                "effective_command": ["/tmp/uqm-controller", "__ci-test"],
                "staged_script_sha256": "0".repeat(64),
            }),
        ] {
            assert!(!valid_step_execution_provenance(&mutant, &controller));
        }

        let ordinary = vec!["cargo".to_string(), "check".to_string()];
        assert!(valid_step_execution_provenance(
            &serde_json::json!({
                "effective_command": ordinary,
                "staged_script_sha256": null,
            }),
            &ordinary,
        ));
        assert!(!valid_step_execution_provenance(
            &serde_json::json!({
                "effective_command": ["cargo", "test"],
                "staged_script_sha256": null,
            }),
            &ordinary,
        ));
        assert!(!valid_step_execution_provenance(
            &serde_json::json!({
                "effective_command": ordinary,
                "staged_script_sha256": "0".repeat(64),
            }),
            &ordinary,
        ));
    }

    fn rewrite_builtin_step_result(
        root: &Path,
        entries: &mut [EvidenceEntry],
        gate: &str,
        step: &str,
        command: &[String],
        terminal: (Option<i64>, Option<i64>, Option<&str>),
    ) {
        let (exit_code, signal, launch_error) = terminal;
        let success = exit_code == Some(0) && signal.is_none() && launch_error.is_none();
        let (effective_command, staged_script_sha256) = fixture_execution_provenance(command);
        let bytes = serde_json::to_vec(&serde_json::json!({
            "schema": "uqm-s4-step-result-v2",
            "gate": gate,
            "step": step,
            "command": command,
            "effective_command": effective_command,
            "staged_script_sha256": staged_script_sha256,
            "executable_identity": fixture_executable_identity(launch_error),
            "success": success,
            "exit_code": exit_code,
            "signal": signal,
            "launch_error": launch_error,
            "supervision": fixture_supervision(launch_error.is_some(), 0, 0),
        }))
        .unwrap();
        let path = format!("{gate}/{step}.result.json");
        fs::write(root.join(&path), &bytes).unwrap();
        let entry = entries.iter_mut().find(|entry| entry.path == path).unwrap();
        entry.byte_length = bytes.len() as u64;
        entry.sha256 = hex_sha256(&bytes);
    }
    fn setup_failure_bundle(root: &Path, contract: &str) -> EvidenceIndex {
        assert!(matches!(
            contract,
            "cache.rust_target" | "ownership.zero_native_delta"
        ));
        let seed = preflight_failure_bundle(root, "source.expected_sha");
        let mut entries = seed.entries;
        let source_sha = "a".repeat(40);
        let source = serde_json::to_vec(&serde_json::json!({
            "schema": "uqm-s4-source-preflight-v2",
            "source_sha": source_sha,
            "detached_state": fixture_detached_state(),
            "expected_sha": source_sha,
            "base_sha": "b".repeat(40),
            "tuple": "linux-x86_64",
            "expected_tuple": "linux-x86_64",
            "cache_mode": "isolated-empty",
            "clean": true,
            "canonical_environment": true,
            "passed": true,
            "first_failed_contract": null,
            "detail": null
        }))
        .unwrap();
        rewrite_bundle_entry(root, &mut entries, "preflight.source", &source);
        let cache_failed = contract == "cache.rust_target";
        let cache = serde_json::to_vec(&serde_json::json!({
            "schema": "uqm-s4-cache-initial-state-v1",
            "mode": "isolated-empty",
            "ambient_cargo_home": "/tmp/ambient-cargo-home",
            "isolation_cargo_home": "/tmp/repository/rust/target/ci-cargo-home",
            "execution_target": "/tmp/repository/rust/target",
            "registry_cache_present": false,
            "git_cache_present": false,
            "execution_target_absent": !cache_failed,
            "rust_target_present": cache_failed,
            "sc2_obj_present": false,
            "restore_used": false,
            "save_used": false,
            "first_failed_contract": cache_failed.then_some("cache.rust_target"),
            "passed": !cache_failed
        }))
        .unwrap();
        rewrite_bundle_entry(root, &mut entries, "cache.initial-state", &cache);
        let delta_failed = contract == "ownership.zero_native_delta";
        let mut categories = serde_json::Map::new();
        for name in [
            "tracked_sources",
            "providers",
            "objects",
            "internal_symbols",
            "bridges",
            "generated_bindings",
            "transitional_flags",
        ] {
            let failed_category = delta_failed && name == "tracked_sources";
            categories.insert(
                name.to_string(),
                serde_json::json!({
                    "measured_delta": usize::from(failed_category),
                    "changed_paths": if failed_category { vec!["rust/src/lib.rs"] } else { Vec::<&str>::new() }
                }),
            );
        }
        let delta = serde_json::to_vec(&serde_json::json!({
            "schema": "uqm-s4-zero-native-delta-v1",
            "base_sha": "b".repeat(40),
            "head_sha": source_sha,
            "categories": categories,
            "transitional_native_inputs": {
                "base_count": 321,
                "head_count": 321,
                "maximum_count": 321,
                "passed": true
            },
            "passed": !delta_failed
        }))
        .unwrap();
        let controller = entries[0].producing_command.clone();
        write_bundle_entry(
            root,
            &mut entries,
            "zero-native-delta.json",
            "ownership.zero-native-delta",
            &controller,
            &delta,
        );
        entries.last_mut().unwrap().producing_gate = "ownership-link".to_string();
        EvidenceIndex::build_and_validate(
            root,
            &fixture_tuples(),
            EvidenceContext {
                source_sha,
                clean: true,
                tuple: "linux-x86_64".into(),
                features: vec!["audio_heart".into()],
                cache_mode: "isolated-empty".into(),
                first_failed_contract: Some(contract.into()),
            },
            entries,
        )
        .unwrap()
    }

    fn write_gate_result_fixture(
        root: &Path,
        entries: &mut Vec<EvidenceEntry>,
        gate: &super::super::authority::Gate,
        controller: &[String],
        failure: Option<&str>,
    ) {
        let bytes = serde_json::to_vec(&serde_json::json!({
            "schema": "uqm-s4-gate-result-v1",
            "gate": gate.id,
            "owner": gate.owner,
            "kind": gate.kind,
            "passed": failure.is_none(),
            "first_failed_contract": failure,
            "detail": failure.map(|_| "fixture reached the security database retention boundary"),
            "controller_command": controller
        }))
        .unwrap();
        write_bundle_entry(
            root,
            entries,
            &format!("{}/gate.result.json", gate.id),
            "gate.result",
            controller,
            &bytes,
        );
        entries.last_mut().unwrap().producing_gate = gate.id.clone();
    }

    fn all_gates_security_post_failure_bundle(root: &Path) -> EvidenceIndex {
        let authority: Authority =
            serde_json::from_slice(include_bytes!("../../../ci/gates.json")).unwrap();
        let seed = setup_failure_bundle(root, "ownership.zero_native_delta");
        let mut entries = seed.entries;
        let delta_entry = entries
            .iter()
            .find(|entry| entry.role == "ownership.zero-native-delta")
            .unwrap();
        let mut delta: serde_json::Value =
            serde_json::from_slice(&fs::read(root.join(&delta_entry.path)).unwrap()).unwrap();
        delta["categories"]["tracked_sources"]["measured_delta"] = serde_json::json!(0);
        delta["categories"]["tracked_sources"]["changed_paths"] = serde_json::json!([]);
        delta["passed"] = serde_json::json!(true);
        rewrite_bundle_entry(
            root,
            &mut entries,
            "ownership.zero-native-delta",
            &serde_json::to_vec(&delta).unwrap(),
        );

        let controller = vec![
            "uqm-xtask".to_string(),
            "ci".to_string(),
            "run".to_string(),
            "all".to_string(),
        ];
        for entry in &mut entries {
            entry.producing_command.clone_from(&controller);
            if matches!(
                entry.role.as_str(),
                "authority.snapshot"
                    | "preflight.source"
                    | "preflight.tools"
                    | "cache.initial-state"
            ) {
                entry.producing_gate = "format".to_string();
            }
        }

        for gate in authority
            .gates
            .iter()
            .take_while(|gate| gate.id != "security")
            .chain(authority.gates.iter().filter(|gate| gate.id == "security"))
        {
            if gate.id == "complexity" {
                let command = std::iter::once("lizard".to_string())
                    .chain(authority.complexity.lizard_arguments.iter().cloned())
                    .chain(std::iter::once("rust/src/lib.rs".to_string()))
                    .collect::<Vec<_>>();
                write_builtin_step_fixture(
                    root,
                    &mut entries,
                    &gate.id,
                    "lizard",
                    &command,
                    (Some(0), None, None),
                );
            } else {
                for step in &gate.steps {
                    write_builtin_step_fixture(
                        root,
                        &mut entries,
                        &gate.id,
                        &step.id,
                        &step.command,
                        (Some(0), None, None),
                    );
                    for name in subordinate_output_names(&gate.id, &step.id) {
                        let bytes = subordinate_fixture_bytes(&step.id, name);
                        write_bundle_entry(
                            root,
                            &mut entries,
                            &format!(
                                "payloads/subordinate.output/{}/{}/{}",
                                gate.id, step.id, name
                            ),
                            "subordinate.output",
                            &step.command,
                            &bytes,
                        );
                        let entry = entries.last_mut().unwrap();
                        entry.producing_gate = gate.id.clone();
                        entry.mime = "application/octet-stream".to_string();
                    }
                }
            }
            if gate.id == "security" {
                let revision_stdout =
                    format!("{}\n", authority.security.advisory_database_revision);
                rewrite_bundle_path(
                    root,
                    &mut entries,
                    "security/advisory-db-revision.stdout.log",
                    revision_stdout.as_bytes(),
                );
                write_gate_result_fixture(
                    root,
                    &mut entries,
                    gate,
                    &controller,
                    Some("security.post.database-retain"),
                );
            } else {
                write_gate_result_fixture(root, &mut entries, gate, &controller, None);
            }
        }

        EvidenceIndex::build_and_validate(
            root,
            &fixture_tuples(),
            EvidenceContext {
                source_sha: "a".repeat(40),
                clean: true,
                tuple: "linux-x86_64".to_string(),
                features: vec!["audio_heart".to_string()],
                cache_mode: "isolated-empty".to_string(),
                first_failed_contract: Some("security.post.database-retain".to_string()),
            },
            entries,
        )
        .unwrap()
    }
    fn successful_workflow_bundle(root: &Path) -> EvidenceIndex {
        let mut index = preflight_failure_bundle(root, "source.expected_sha");
        let controller = vec![
            "uqm-xtask".into(),
            "ci".into(),
            "run".into(),
            "workflow".into(),
        ];
        index.clean = true;
        index.first_failed_contract = None;
        for entry in &mut index.entries {
            entry.producing_gate = "workflow".into();
            entry.producing_command.clone_from(&controller);
        }
        let source = serde_json::json!({
            "schema": "uqm-s4-source-preflight-v2",
            "source_sha": index.source_sha,
            "detached_state": null,
            "expected_sha": null,
            "base_sha": null,
            "tuple": index.tuple,
            "expected_tuple": null,
            "cache_mode": "ambient-dev",
            "clean": true,
            "canonical_environment": false,
            "passed": true,
            "first_failed_contract": null,
            "detail": null
        });
        rewrite_bundle_entry(
            root,
            &mut index.entries,
            "preflight.source",
            &serde_json::to_vec(&source).unwrap(),
        );
        let actionlint = vec!["actionlint".to_string()];
        for (path, role, bytes) in [
            ("workflow/actionlint.stdout.log", "step.stdout", Vec::new()),
            ("workflow/actionlint.stderr.log", "step.stderr", Vec::new()),
            (
                "workflow/actionlint.result.json",
                "step.result",
                serde_json::to_vec(&serde_json::json!({
                    "schema": "uqm-s4-step-result-v2",
                    "gate": "workflow",
                    "step": "actionlint",
                    "command": actionlint,
                    "effective_command": actionlint,
                    "staged_script_sha256": null,
                    "executable_identity": fixture_executable_identity(None),
                    "success": true,
                    "exit_code": 0,
                    "signal": null,
                    "launch_error": null,
                    "supervision": fixture_supervision(false, 0, 0),
                }))
                .unwrap(),
            ),
        ] {
            write_bundle_entry(root, &mut index.entries, path, role, &actionlint, &bytes);
            let entry = index.entries.last_mut().unwrap();
            entry.producing_gate = "workflow".into();
        }
        let rules = [
            "workflow.actionlint",
            "workflow.unrestricted_triggers",
            "workflow.checkout_pr_head",
            "workflow.actions_full_sha",
            "workflow.required_identity_environment",
            "workflow.tool_authority",
            "workflow.least_permissions",
            "workflow.timeouts",
            "workflow.generated_matrix",
            "workflow.no_direct_gate_commands",
            "workflow.no_cache_action",
            "workflow.always_uploaded_failure_evidence",
            "workflow.content_addressed_transport",
        ]
        .into_iter()
        .map(|rule| {
            serde_json::json!({
                "rule": rule,
                "passed": true,
                "detail": "fixture"
            })
        })
        .collect::<Vec<_>>();
        let validation = serde_json::to_vec(&serde_json::json!({
            "schema": "uqm-s4-workflow-validation-v1",
            "first_failed_rule": null,
            "rules": rules
        }))
        .unwrap();
        write_bundle_entry(
            root,
            &mut index.entries,
            "workflow/workflow-validation.json",
            "workflow.validation",
            &controller,
            &validation,
        );
        index.entries.last_mut().unwrap().producing_gate = "workflow".into();
        let authority: Authority =
            serde_json::from_slice(include_bytes!("../../../ci/gates.json")).unwrap();
        let gate = authority.gate("workflow").unwrap();
        let result = serde_json::to_vec(&serde_json::json!({
            "schema": "uqm-s4-gate-result-v1",
            "gate": gate.id,
            "owner": gate.owner,
            "kind": gate.kind,
            "controller_command": controller,

            "passed": true,
            "first_failed_contract": null,
            "detail": null
        }))
        .unwrap();
        write_bundle_entry(
            root,
            &mut index.entries,
            "workflow/gate.result.json",
            "gate.result",
            &controller,
            &result,
        );
        index.entries.last_mut().unwrap().producing_gate = "workflow".into();
        index
    }

    fn mutation_fixture_set(
        target: &str,
        authority: &Authority,
        baseline_phase: bool,
    ) -> Vec<(String, Vec<u8>)> {
        if target == "artifact" {
            let fixture = tempfile::tempdir().unwrap();
            build_artifact_mutation_fixture(
                fixture.path(),
                authority,
                include_bytes!("../../../ci/gates.json"),
                !baseline_phase,
            )
            .unwrap();
            return [
                "evidence-index.json",
                "payloads/authority.snapshot/gates.json",
                "source-preflight.json",
                "tool-preflight.json",
                "cache-initial-state.json",
            ]
            .into_iter()
            .map(|path| {
                (
                    path.to_string(),
                    fs::read(fixture.path().join(path)).unwrap(),
                )
            })
            .collect();
        }
        let (recipe, _, baseline, mutant) =
            super::super::mutations::expected_causal_contract(target, authority).unwrap();
        let recipe_bytes = if baseline_phase { baseline } else { mutant };
        match target {
            "format" | "check" | "clippy" | "test" => vec![
                (
                    "Cargo.toml".into(),
                    b"[package]\nname='fixture'\nversion='0.1.0'\nedition='2021'\n".to_vec(),
                ),
                (recipe.path, recipe_bytes),
            ],
            "harness" => vec![
                (recipe.path, recipe_bytes),
                (
                    "rust/target/production-artifacts.json".into(),
                    b"fixture\n".to_vec(),
                ),
            ],
            _ => vec![(recipe.path, recipe_bytes)],
        }
    }

    fn baseline_mutation_fixture_bytes(
        target: &str,
        authority: &Authority,
    ) -> Vec<(String, Vec<u8>)> {
        mutation_fixture_set(target, authority, true)
    }

    fn mutation_fixture_bytes(target: &str, authority: &Authority) -> Vec<(String, Vec<u8>)> {
        mutation_fixture_set(target, authority, false)
    }

    fn successful_mutations_bundle(root: &Path) -> EvidenceIndex {
        let authority: Authority =
            serde_json::from_slice(include_bytes!("../../../ci/gates.json")).unwrap();
        let gate = authority.gate("mutations").unwrap();
        let controller = vec![
            "uqm-xtask".into(),
            "ci".into(),
            "run".into(),
            "mutations".into(),
        ];
        let mut index = preflight_failure_bundle(root, "source.expected_sha");
        index.clean = true;
        index.first_failed_contract = None;
        for entry in &mut index.entries {
            entry.producing_gate = "mutations".into();
            entry.producing_command.clone_from(&controller);
        }
        let source = serde_json::json!({
            "schema": "uqm-s4-source-preflight-v2",
            "source_sha": index.source_sha,
            "detached_state": null,
            "expected_sha": null,
            "base_sha": null,
            "tuple": index.tuple,
            "expected_tuple": null,
            "cache_mode": "ambient-dev",
            "clean": true,
            "canonical_environment": false,
            "passed": true,
            "first_failed_contract": null,
            "detail": null
        });
        rewrite_bundle_entry(
            root,
            &mut index.entries,
            "preflight.source",
            &serde_json::to_vec(&source).unwrap(),
        );
        let mut cases = Vec::new();
        for target in &authority.mutation_targets {
            let target = target.as_str();
            let baseline_fixture_bytes = baseline_mutation_fixture_bytes(target, &authority);
            let mutant_fixture_bytes = mutation_fixture_bytes(target, &authority);
            let mut baseline_files = Vec::new();
            let mut files = Vec::new();
            for (phase, fixture_bytes, declarations) in [
                ("baseline", &baseline_fixture_bytes, &mut baseline_files),
                ("mutant", &mutant_fixture_bytes, &mut files),
            ] {
                for (position, (name, bytes)) in fixture_bytes.iter().enumerate() {
                    let path =
                        format!("payloads/mutation.fixture/{target}/{phase}/{position}/{name}");
                    write_bundle_entry(
                        root,
                        &mut index.entries,
                        &path,
                        "mutation.fixture",
                        &controller,
                        bytes,
                    );
                    index.entries.last_mut().unwrap().producing_gate = "mutations".into();
                    declarations.push(serde_json::json!({
                        "path": path,
                        "byte_length": bytes.len(),
                        "sha256": hex_sha256(bytes)
                    }));
                }
            }
            let contract = MutationTarget::parse(target).unwrap().contract();
            let (recipe, expected_diagnostic) = if target == "artifact" {
                let hash_for = |files: &[(String, Vec<u8>)]| {
                    files
                        .iter()
                        .find_map(|(path, bytes)| {
                            (path == "tool-preflight.json").then(|| hex_sha256(bytes))
                        })
                        .unwrap()
                };
                (
                    super::super::mutations::MutationRecipe {
                        operation: "replace-and-rehash-enclosing-index".to_string(),
                        path: "tool-preflight.json".to_string(),
                        baseline_sha256: hash_for(&baseline_fixture_bytes),
                        mutant_sha256: hash_for(&mutant_fixture_bytes),
                    },
                    super::super::mutations::MutationDiagnostic {
                        class: "artifact-provenance".to_string(),
                        path: "tool-preflight.json".to_string(),
                        required_fragments: vec![
                            "artifact-provenance".to_string(),
                            "evidence.preflight.tools.rust.result".to_string(),
                        ],
                    },
                )
            } else {
                let (recipe, diagnostic, _, _) =
                    super::super::mutations::expected_causal_contract(target, &authority).unwrap();
                (recipe, diagnostic)
            };
            let failure_output = expected_diagnostic.required_fragments.join("\n");
            let expected_route: Vec<MutationRouteIdentity> = match target {
                "format" | "check" | "clippy" | "test" => {
                    let gate_id = if target == "test" { "tests" } else { target };
                    authority
                        .gate(gate_id)
                        .unwrap()
                        .steps
                        .iter()
                        .map(|step| {
                            (
                                gate_id.to_string(),
                                step.id.clone(),
                                step.cwd.clone(),
                                step.native_profile.clone(),
                                step.command.clone(),
                                step.timeout_seconds * 1_000,
                            )
                        })
                        .collect()
                }
                "harness" => {
                    let gate = authority.gate("probes-harnesses").unwrap();
                    let step = gate
                        .steps
                        .iter()
                        .find(|step| step.id == "p00-harness")
                        .unwrap();
                    vec![(
                        gate.id.clone(),
                        step.id.clone(),
                        step.cwd.clone(),
                        step.native_profile.clone(),
                        step.command.clone(),
                        step.timeout_seconds * 1_000,
                    )]
                }
                "complexity" => vec![(
                    "complexity".to_string(),
                    "lizard".to_string(),
                    ".".to_string(),
                    None,
                    std::iter::once("lizard".to_string())
                        .chain(authority.complexity.lizard_arguments.iter().cloned())
                        .chain(std::iter::once("runaway.rs".to_string()))
                        .collect(),
                    authority.supervision.builtin_timeout_seconds * 1_000,
                )],
                _ => vec![(
                    "mutations".to_string(),
                    "internal-validator".to_string(),
                    ".".to_string(),
                    None,
                    vec![
                        "uqm-xtask-internal".to_string(),
                        super::super::mutations::INTERNAL_VALIDATOR_COMMAND.to_string(),
                        target.to_string(),
                    ],
                    authority.supervision.builtin_timeout_seconds * 1_000,
                )],
            };
            let execution_json = |expected: &MutationRouteIdentity, success: bool| {
                let stdout = if success { "baseline accepted" } else { "" };
                let stderr = if success { "" } else { failure_output.as_str() };
                let mut supervision =
                    fixture_supervision(false, stdout.len() as u64, stderr.len() as u64);
                supervision["timeout_milliseconds"] = serde_json::json!(expected.5);
                serde_json::json!({
                    "gate": expected.0,
                    "step": expected.1,
                    "cwd": expected.2,
                    "native_profile": expected.3,
                    "command": expected.4,
                    "executable_identity": {
                        "path": "/usr/bin/mutation-tool",
                        "byte_length": 64,
                        "sha256": "a".repeat(64),
                        "mode": 0o755
                    },
                    "supervision": supervision,
                    "stdout": stdout,
                    "stderr": stderr,
                    "exit_code": if success { 0 } else { 1 },
                    "signal": null,
                    "launch_error": null,
                    "success": success
                })
            };
            let baseline_executions: Vec<_> = expected_route
                .iter()
                .map(|expected| execution_json(expected, true))
                .collect();
            let executions: Vec<_> = expected_route
                .iter()
                .enumerate()
                .map(|(position, expected)| {
                    execution_json(expected, position + 1 < expected_route.len())
                })
                .collect();
            let recipe = serde_json::to_value(&recipe).unwrap();
            let expected_diagnostic = serde_json::to_value(&expected_diagnostic).unwrap();
            cases.push(serde_json::json!({
                "target": target,
                "contract": contract,
                "defect": "fixture defect",
                "baseline_accepted": true,
                "rejection_observed": true,
                "detail": "production validator rejected the fixture",
                "baseline_executions": baseline_executions,
                "executions": executions,
                "baseline_files": baseline_files,
                "files": files,
                "recipe": recipe,
                "expected_diagnostic": expected_diagnostic
            }));
        }
        let receipt = serde_json::to_vec(&serde_json::json!({
            "schema": "uqm-s4-mutations-receipt-v3",
            "source_sha": index.source_sha,
            "passed": true,
            "first_failed_target": null,
            "cases": cases
        }))
        .unwrap();
        write_bundle_entry(
            root,
            &mut index.entries,
            "mutations/mutations-receipt.json",
            "mutations.receipt",
            &controller,
            &receipt,
        );
        index.entries.last_mut().unwrap().producing_gate = "mutations".into();
        let result = serde_json::to_vec(&serde_json::json!({
            "schema": "uqm-s4-gate-result-v1",
            "gate": gate.id,
            "owner": gate.owner,
            "kind": gate.kind,
            "controller_command": controller,
            "passed": true,
            "first_failed_contract": null,
            "detail": null
        }))
        .unwrap();
        write_bundle_entry(
            root,
            &mut index.entries,
            "mutations/gate.result.json",
            "gate.result",
            &controller,
            &result,
        );
        index.entries.last_mut().unwrap().producing_gate = "mutations".into();
        index
    }
    fn package_manifest_fixture(
        authority: &Authority,
        index: &EvidenceIndex,
        artifacts: Vec<serde_json::Value>,
    ) -> serde_json::Value {
        let tool_identity = serde_json::json!({
            "executable": "/usr/bin/tool",
            "version": "tool 1.0",
            "sha256": "b".repeat(64),
            "effective_args": []
        });
        let toolchain = serde_json::json!({
            "target": rust_target_for_tuple(&index.tuple).unwrap(),
            "rustc": tool_identity,
            "cargo": tool_identity,
            "cc": tool_identity,
            "ar": tool_identity,
            "nm": tool_identity,
            "pkg_config": tool_identity,
            "linker": tool_identity
        });
        let mut manifest = serde_json::json!({
            "schema": authority.package.manifest_schema,
            "git_head": index.source_sha,
            "tracked_worktree": { "file_count": 1, "sha256": "a".repeat(64) },
            "dirty": false,
            "toolchain": toolchain,
            "source_date_epoch": 1,
            "native_build": {
                "schema": "uqm-native-build-evidence-v1",
                "source_date_epoch": 1,
                "build_date": "Jan 01 1970",
                "target": index.tuple,
                "active_features": authority.package.features,
                "toolchain": toolchain,
                "packages": [{
                    "name": "sdl2",
                    "version": "2.0",
                    "cflags": [],
                    "libs": ["-lSDL2"]
                }],
                "compile_profile": {
                    "target": index.tuple,
                    "compiler": "/usr/bin/cc",
                    "ordered_defines": [],
                    "ordered_include_roots": [],
                    "ordered_compile_flags": [],
                    "dependency_flags": ["-MMD", "-MF", "<depfile>"],
                    "command_template": ["cc", "-c", "<source>"]
                },
                "build_environment": {}
            },
            "command": PACKAGE_PROOF_COMMAND,
            "target": rust_target_for_tuple(&index.tuple).unwrap(),
            "profile": authority.package.profile,
            "features": authority.package.features,
            "cargo_feature_graph": [{
                "name": "uqm",
                "version": "0.8.0",
                "features": ["audio_heart", "linked_c_archive"]
            }],
            "artifacts": artifacts
        });
        let artifact_digests = manifest["artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|artifact| {
                serde_json::json!({
                    "role": artifact["role"],
                    "byte_length": artifact["byte_length"],
                    "sha256": artifact["sha256"],
                })
            })
            .collect::<Vec<_>>();
        let build_identity = serde_json::json!({
            "git_head": manifest["git_head"],
            "tracked_worktree": manifest["tracked_worktree"],
            "dirty": manifest["dirty"],
            "source_date_epoch": manifest["source_date_epoch"],
            "toolchain": manifest["toolchain"],
            "native_build": manifest["native_build"],
        });
        manifest.as_object_mut().unwrap().insert(
            "determinism_proof".into(),
            serde_json::json!({
                "command": PACKAGE_PROOF_COMMAND,
                "clean_builds": PACKAGE_PROOF_CLEAN_BUILDS,
                "comparison": PACKAGE_PROOF_COMPARISON,
                "first_build": artifact_digests,
                "second_build": artifact_digests,
                "first_identity": build_identity,
                "second_identity": build_identity,
            }),
        );
        manifest
    }

    fn successful_package_bundle(root: &Path) -> EvidenceIndex {
        let authority: Authority =
            serde_json::from_slice(include_bytes!("../../../ci/gates.json")).unwrap();
        let gate = authority.gate("package").unwrap();
        let controller = vec![
            "uqm-xtask".into(),
            "ci".into(),
            "run".into(),
            "package".into(),
        ];
        let mut index = preflight_failure_bundle(root, "source.expected_sha");
        index.clean = true;
        index.first_failed_contract = None;
        index.features.clone_from(&authority.profiles.linked_test);
        for entry in &mut index.entries {
            entry.producing_gate = "package".into();
            entry.producing_command.clone_from(&controller);
        }
        let source = serde_json::json!({
            "schema": "uqm-s4-source-preflight-v2",
            "source_sha": index.source_sha,
            "detached_state": null,
            "expected_sha": null,
            "base_sha": null,
            "tuple": index.tuple,
            "expected_tuple": null,
            "cache_mode": "ambient-dev",
            "clean": true,
            "canonical_environment": false,
            "passed": true,
            "first_failed_contract": null,
            "detail": null
        });
        rewrite_bundle_entry(
            root,
            &mut index.entries,
            "preflight.source",
            &serde_json::to_vec(&source).unwrap(),
        );
        for step in &gate.steps {
            let (effective_command, staged_script_sha256) =
                fixture_execution_provenance(&step.command);
            for (suffix, role, bytes) in [
                ("stdout.log", "step.stdout", Vec::new()),
                ("stderr.log", "step.stderr", Vec::new()),
                (
                    "result.json",
                    "step.result",
                    serde_json::to_vec(&serde_json::json!({
                        "schema": "uqm-s4-step-result-v2",
                        "gate": gate.id,
                        "step": step.id,
                        "command": step.command,
                        "effective_command": effective_command,
                        "staged_script_sha256": staged_script_sha256,
                        "executable_identity": fixture_executable_identity(None),
                        "success": true,
                        "exit_code": 0,
                        "signal": null,
                        "launch_error": null,
                        "supervision": fixture_supervision(false, 0, 0),
                    }))
                    .unwrap(),
                ),
            ] {
                write_bundle_entry(
                    root,
                    &mut index.entries,
                    &format!("package/{}.{}", step.id, suffix),
                    role,
                    &step.command,
                    &bytes,
                );
                index.entries.last_mut().unwrap().producing_gate = "package".into();
            }
        }
        let provider_report = serde_json::json!({
            "schema": "uqm-provider-report-v1",
            "entries": [{
                "path": "native/example.c.o",
                "issue": "EXAMPLE",
                "provider": "native_object:native/example.c.o",
                "archive_decision": "include",
                "status": "ok"
            }],
            "ledger_sha256": authority.ledger_identity.sha256,
            "symbols": [{
                "symbol": "example_symbol",
                "canonical_owner": "EXAMPLE/#27",
                "provider_kind": "rust_source",
                "provider_path": "rust/src/example.rs",
                "excluded_provider_paths": ["native/example.c.o"]
            }],
            "tracked_native_file_delta": 0,
            "summary": {
                "total_objects": 1,
                "included": 1,
                "excluded": 0,
                "duplicate_providers_excluded": 0,
                "recompiled": 0,
                "replaced": 0,
                "violations": 0,
                "passed": true
            }
        });
        let mut artifacts = Vec::new();
        for (position, artifact) in authority.package.artifacts.iter().enumerate() {
            let filename = if artifact.role == "executable" {
                "uqm".to_string()
            } else {
                format!("artifact-{position}")
            };
            let source_path = if artifact.role == "executable" {
                "rust/target/uqm-package/x86_64-unknown-linux-gnu/uqm".to_string()
            } else {
                format!("rust/target/release/{filename}")
            };
            let bytes = if artifact.role == "provider_report" {
                serde_json::to_vec(&provider_report).unwrap()
            } else {
                format!("{} fixture", artifact.role).into_bytes()
            };
            let evidence_role = format!("package-{}", artifact.role);
            let command = artifact
                .producing_command
                .split_whitespace()
                .map(str::to_string)
                .collect::<Vec<_>>();
            write_bundle_entry(
                root,
                &mut index.entries,
                &format!("payloads/{evidence_role}/{filename}"),
                &evidence_role,
                &command,
                &bytes,
            );
            let entry = index.entries.last_mut().unwrap();
            entry.producing_gate = "package".into();
            entry.mime.clone_from(&artifact.media_type);
            artifacts.push(serde_json::json!({
                "role": artifact.role,
                "path": source_path,
                "media_type": artifact.media_type,
                "producing_command": artifact.producing_command,
                "byte_length": bytes.len(),
                "sha256": hex_sha256(&bytes)
            }));
        }
        let package_step = gate
            .steps
            .iter()
            .find(|step| step.id == "xtask-package")
            .unwrap();
        let manifest = package_manifest_fixture(&authority, &index, artifacts.clone());
        write_bundle_entry(
            root,
            &mut index.entries,
            "payloads/package-manifest/production-artifacts.json",
            "package-manifest",
            &package_step.command,
            &serde_json::to_vec(&manifest).unwrap(),
        );
        index.entries.last_mut().unwrap().producing_gate = "package".into();
        for (role, filename, step_id) in [
            (
                "ownership-production-report",
                "ownership-production-report.json".to_string(),
                "verify-production-ownership",
            ),
            (
                "native-dependency-capture",
                format!("native-dependencies-{}.candidate.json", index.tuple),
                "capture-native-dependencies",
            ),
        ] {
            let step = gate.steps.iter().find(|step| step.id == step_id).unwrap();
            let bytes = if role == "ownership-production-report" {
                let artifact_hash = |wanted: &str| {
                    artifacts
                        .iter()
                        .find(|artifact| {
                            artifact.get("role").and_then(|value| value.as_str()) == Some(wanted)
                        })
                        .and_then(|artifact| artifact.get("sha256"))
                        .and_then(|value| value.as_str())
                        .unwrap()
                };
                serde_json::to_vec(&serde_json::json!({
                    "schema": "uqm-production-artifact-report-v1",
                    "provider_report": provider_report.clone(),
                    "executable": {
                        "path": "/tmp/repository/rust/target/release/uqm",
                        "sha256": artifact_hash("executable")
                    },
                    "rust_archive": {
                        "path": "/tmp/repository/rust/target/release/libuqm_rust.a",
                        "sha256": artifact_hash("rust_static_archive")
                    },
                    "c_archive": {
                        "path": "/tmp/repository/rust/target/release/libuqm_c.a",
                        "sha256": artifact_hash("c_static_archive")
                    }
                }))
                .unwrap()
            } else {
                serde_json::to_vec(&serde_json::json!({
                    "schema": "uqm-native-dependency-capture-v1",
                    "target": index.tuple,
                    "dependencies": ["rust/build.rs", "sc2/src/config.h"]
                }))
                .unwrap()
            };
            write_bundle_entry(
                root,
                &mut index.entries,
                &format!("payloads/{role}/{filename}"),
                role,
                &step.command,
                &bytes,
            );
            index.entries.last_mut().unwrap().producing_gate = "package".into();
        }
        let result = serde_json::json!({
            "schema": "uqm-s4-gate-result-v1",
            "gate": gate.id,
            "owner": gate.owner,
            "kind": gate.kind,
            "controller_command": controller,
            "passed": true,
            "first_failed_contract": null,
            "detail": null
        });
        write_bundle_entry(
            root,
            &mut index.entries,
            "package/gate.result.json",
            "gate.result",
            &controller,
            &serde_json::to_vec(&result).unwrap(),
        );
        index.entries.last_mut().unwrap().producing_gate = "package".into();
        index
    }

    fn tampered_package_manifest_contracts(
        mutate: impl FnOnce(&mut serde_json::Value),
    ) -> Vec<String> {
        let bundle = tempfile::tempdir().unwrap();
        let mut index = successful_package_bundle(bundle.path());
        let manifest_entry = index
            .entries
            .iter()
            .find(|entry| entry.role == "package-manifest")
            .unwrap();
        let mut manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(bundle.path().join(&manifest_entry.path)).unwrap())
                .unwrap();
        mutate(&mut manifest);
        rewrite_bundle_entry(
            bundle.path(),
            &mut index.entries,
            "package-manifest",
            &serde_json::to_vec(&manifest).unwrap(),
        );

        production_contracts(bundle.path(), &index)
    }

    fn tampered_package_payload_contracts(
        role: &str,
        mutate: impl FnOnce(&mut serde_json::Value),
    ) -> Vec<String> {
        let bundle = tempfile::tempdir().unwrap();
        let mut index = successful_package_bundle(bundle.path());
        let payload_entry = index
            .entries
            .iter()
            .find(|entry| entry.role == role)
            .unwrap();
        let mut payload: serde_json::Value =
            serde_json::from_slice(&fs::read(bundle.path().join(&payload_entry.path)).unwrap())
                .unwrap();
        mutate(&mut payload);
        rewrite_bundle_entry(
            bundle.path(),
            &mut index.entries,
            role,
            &serde_json::to_vec(&payload).unwrap(),
        );
        production_contracts(bundle.path(), &index)
    }

    #[test]
    fn successful_package_replay_requires_authoritative_manifest_and_artifacts() {
        let bundle = tempfile::tempdir().unwrap();
        let index = successful_package_bundle(bundle.path());
        assert_eq!(
            production_contracts(bundle.path(), &index),
            Vec::<String>::new()
        );

        let mut forged = index.clone();
        let manifest_entry = forged
            .entries
            .iter()
            .find(|entry| entry.role == "package-manifest")
            .unwrap();
        let mut manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(bundle.path().join(&manifest_entry.path)).unwrap())
                .unwrap();
        manifest["artifacts"][0]["producing_command"] = serde_json::json!("forged producer");
        rewrite_bundle_entry(
            bundle.path(),
            &mut forged.entries,
            "package-manifest",
            &serde_json::to_vec(&manifest).unwrap(),
        );
        assert!(production_contracts(bundle.path(), &forged)
            .iter()
            .any(|contract| contract == "evidence.package.artifact.executable.evidence"));

        for contracts in [
            tampered_package_manifest_contracts(|manifest| {
                manifest["command"] = serde_json::json!("cargo build --release");
            }),
            tampered_package_manifest_contracts(|manifest| {
                manifest
                    .as_object_mut()
                    .unwrap()
                    .remove("determinism_proof");
            }),
            tampered_package_manifest_contracts(|manifest| {
                manifest["determinism_proof"]["first_build"][0]["sha256"] =
                    serde_json::json!("0".repeat(64));
            }),
            tampered_package_manifest_contracts(|manifest| {
                manifest["determinism_proof"]["second_identity"]["git_head"] =
                    serde_json::json!("b".repeat(40));
            }),
            tampered_package_manifest_contracts(|manifest| {
                manifest["git_head"] = serde_json::json!("b".repeat(40));
            }),
            tampered_package_manifest_contracts(|manifest| {
                manifest["features"] = serde_json::json!(["audio_heart"]);
            }),
            tampered_package_manifest_contracts(|manifest| {
                manifest["target"] = serde_json::json!("aarch64-unknown-linux-gnu");
            }),
            tampered_package_manifest_contracts(|manifest| {
                manifest["tracked_worktree"]["sha256"] = serde_json::json!("0".repeat(63));
            }),
            tampered_package_manifest_contracts(|manifest| {
                manifest["native_build"]["source_date_epoch"] = serde_json::json!(2);
            }),
            tampered_package_manifest_contracts(|manifest| {
                manifest["cargo_feature_graph"][0]["unexpected"] = serde_json::json!(true);
            }),
            tampered_package_manifest_contracts(|manifest| {
                manifest["cargo_feature_graph"][0]["features"] = serde_json::json!([]);
            }),
            tampered_package_manifest_contracts(|manifest| {
                manifest["cargo_feature_graph"][0]["name"] = serde_json::json!("forged");
            }),
            tampered_package_manifest_contracts(|manifest| {
                manifest["artifacts"].as_array_mut().unwrap().swap(0, 1);
            }),
        ] {
            assert!(contracts
                .iter()
                .any(|contract| contract == "evidence.package.manifest.content"));
        }
        for contracts in [
            tampered_package_manifest_contracts(|manifest| {
                manifest.as_object_mut().unwrap().remove("toolchain");
            }),
            tampered_package_manifest_contracts(|manifest| {
                manifest["native_build"]
                    .as_object_mut()
                    .unwrap()
                    .remove("toolchain");
            }),
            tampered_package_manifest_contracts(|manifest| {
                manifest["cargo_feature_graph"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove("features");
            }),
            tampered_package_manifest_contracts(|manifest| {
                manifest["toolchain"]["rustc"]["unexpected"] = serde_json::json!(true);
            }),
            tampered_package_manifest_contracts(|manifest| {
                manifest["toolchain"]["rustc"]
                    .as_object_mut()
                    .unwrap()
                    .remove("sha256");
            }),
            tampered_package_manifest_contracts(|manifest| {
                manifest["native_build"]["packages"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove("libs");
            }),
            tampered_package_manifest_contracts(|manifest| {
                manifest["native_build"]["compile_profile"]["dependency_flags"] =
                    serde_json::json!("-MMD");
            }),
            tampered_package_manifest_contracts(|manifest| {
                manifest["native_build"]["build_environment"] = serde_json::json!({"CC": 7});
            }),
        ] {
            assert!(contracts
                .iter()
                .any(|contract| contract == "evidence.package.manifest.content"));
        }
        for contracts in [
            tampered_package_manifest_contracts(|manifest| {
                manifest["artifacts"][0]["sha256"] = serde_json::json!("0".repeat(64));
            }),
            tampered_package_manifest_contracts(|manifest| {
                manifest["artifacts"][0]["path"] = serde_json::json!("../outside");
            }),
        ] {
            assert!(contracts
                .iter()
                .any(|contract| { contract.starts_with("evidence.package.artifact.executable.") }));
        }
        assert!(tampered_package_manifest_contracts(|manifest| {
            manifest["artifacts"][0]["role"] = serde_json::json!("forged");
        })
        .iter()
        .any(|contract| contract == "evidence.package.artifact.executable.manifest"));

        let mut omitted = index.clone();
        for role in ["ownership-production-report", "native-dependency-capture"] {
            let mut missing = index.clone();
            missing.entries.retain(|entry| entry.role != role);
            assert!(production_contracts(bundle.path(), &missing)
                .iter()
                .any(|contract| contract == &format!("evidence.package.{role}.identity")));
        }
        let mut duplicate = index.clone();
        let mut extra = duplicate
            .entries
            .iter()
            .find(|entry| entry.role == "package-executable")
            .unwrap()
            .clone();
        extra.path = "payloads/package-executable/duplicate".to_string();
        fs::write(bundle.path().join(&extra.path), b"executable fixture").unwrap();
        duplicate.entries.push(extra);
        assert!(production_contracts(bundle.path(), &duplicate)
            .iter()
            .any(|contract| contract == "evidence.package.artifact.executable.evidence"));

        omitted
            .entries
            .retain(|entry| entry.role != "package-c_static_archive");
        assert!(production_contracts(bundle.path(), &omitted)
            .iter()
            .any(|contract| contract == "evidence.package.artifact.c_static_archive.evidence"));

        assert!(
            tampered_package_payload_contracts("ownership-production-report", |report| {
                report["provider_report"]["summary"]["passed"] = serde_json::json!(false);
            })
            .iter()
            .any(|contract| contract == "evidence.package.ownership-report.content")
        );
        for contracts in [
            tampered_package_payload_contracts("ownership-production-report", |report| {
                report["unexpected"] = serde_json::json!(true);
            }),
            tampered_package_payload_contracts("ownership-production-report", |report| {
                report["provider_report"]["entries"][0]["unexpected"] = serde_json::json!(true);
            }),
            tampered_package_payload_contracts("ownership-production-report", |report| {
                report["provider_report"]["entries"][0]["issue"] = serde_json::json!("");
            }),
            tampered_package_payload_contracts("ownership-production-report", |report| {
                report["provider_report"]["entries"][0]["provider"] =
                    serde_json::json!("native_object:native/other.c.o");
            }),
            tampered_package_payload_contracts("ownership-production-report", |report| {
                report["provider_report"]["symbols"][0]["provider_kind"] =
                    serde_json::json!("native_object");
            }),
            tampered_package_payload_contracts("ownership-production-report", |report| {
                report["provider_report"]["summary"]["included"] = serde_json::json!(0);
                report["provider_report"]["summary"]["excluded"] = serde_json::json!(1);
                report["provider_report"]["summary"]["replaced"] = serde_json::json!(1);
            }),
            tampered_package_payload_contracts("ownership-production-report", |report| {
                report["provider_report"]["ledger_sha256"] = serde_json::json!("0".repeat(64));
            }),
            tampered_package_payload_contracts("ownership-production-report", |report| {
                report["provider_report"]["symbols"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove("provider_path");
            }),
            tampered_package_payload_contracts("ownership-production-report", |report| {
                report["provider_report"]["summary"]
                    .as_object_mut()
                    .unwrap()
                    .remove("included");
            }),
            tampered_package_payload_contracts("ownership-production-report", |report| {
                report["rust_archive"]
                    .as_object_mut()
                    .unwrap()
                    .remove("path");
            }),
        ] {
            assert!(contracts
                .iter()
                .any(|contract| contract.starts_with("evidence.package.ownership-report.")));
        }
        assert!(
            tampered_package_payload_contracts("ownership-production-report", |report| {
                report["executable"]["sha256"] = serde_json::json!("0".repeat(64));
            })
            .iter()
            .any(|contract| contract == "evidence.package.ownership-report.executable")
        );
        for contracts in [
            tampered_package_payload_contracts("native-dependency-capture", |capture| {
                capture["target"] = serde_json::json!("linux-aarch64");
            }),
            tampered_package_payload_contracts("native-dependency-capture", |capture| {
                capture["dependencies"] = serde_json::json!(["sc2/src/config.h", "rust/build.rs"]);
            }),
            tampered_package_payload_contracts("native-dependency-capture", |capture| {
                capture["unexpected"] = serde_json::json!(true);
            }),
            tampered_package_payload_contracts("native-dependency-capture", |capture| {
                capture.as_object_mut().unwrap().remove("dependencies");
            }),
        ] {
            assert!(contracts
                .iter()
                .any(|contract| contract == "evidence.package.native-dependencies.content"));
        }
    }

    #[test]
    fn failed_package_postprocessing_requires_exact_retained_prefix() {
        let authority: Authority =
            serde_json::from_slice(include_bytes!("../../../ci/gates.json")).unwrap();
        let gate = authority
            .gates
            .iter()
            .find(|gate| gate.id == "package")
            .unwrap();
        let bundle = tempfile::tempdir().unwrap();
        let complete = successful_package_bundle(bundle.path());

        let mut manifest_read = complete.clone();
        manifest_read.entries.retain(|entry| {
            !(entry.role == "package-manifest"
                || entry.role.starts_with("package-")
                || entry.role == "ownership-production-report"
                || entry.role == "native-dependency-capture")
        });
        assert_eq!(
            validate_failed_package_postprocess(
                bundle.path(),
                &manifest_read,
                &authority,
                gate,
                "package.post.manifest-read",
            ),
            Vec::<String>::new()
        );
        manifest_read.first_failed_contract = Some("package.post.manifest-read".to_string());
        let gate_result_entry = manifest_read
            .entries
            .iter()
            .find(|entry| entry.role == "gate.result")
            .unwrap();
        let gate_result: serde_json::Value =
            serde_json::from_slice(&fs::read(bundle.path().join(&gate_result_entry.path)).unwrap())
                .unwrap();
        let assert_production_failure = |index: &mut EvidenceIndex, contract: &str| {
            index.first_failed_contract = Some(contract.to_string());
            let mut failed_result = gate_result.clone();
            failed_result["passed"] = serde_json::json!(false);
            failed_result["first_failed_contract"] = serde_json::json!(contract);
            failed_result["detail"] = serde_json::json!("package postprocessing failed");
            rewrite_bundle_entry(
                bundle.path(),
                &mut index.entries,
                "gate.result",
                &serde_json::to_vec(&failed_result).unwrap(),
            );
            assert_eq!(
                production_contracts(bundle.path(), index),
                Vec::<String>::new(),
                "{contract} must replay from the retained bundle",
            );
        };
        assert_production_failure(&mut manifest_read, "package.post.manifest-read");

        let retain_supplemental = |roles: &[String]| {
            let mut index = complete.clone();
            index.entries.retain(|entry| {
                !entry.role.starts_with("package-")
                    && entry.role != "package-manifest"
                    && entry.role != "ownership-production-report"
                    && entry.role != "native-dependency-capture"
                    || roles.contains(&entry.role)
            });
            index
        };
        assert!(validate_failed_package_postprocess(
            bundle.path(),
            &manifest_read,
            &authority,
            gate,
            "package.post.manifest-retain",
        )
        .is_empty());
        assert_production_failure(&mut manifest_read, "package.post.manifest-retain");
        for (position, failed) in authority.package.artifacts.iter().enumerate() {
            let mut retained_roles = vec!["package-manifest".to_string()];
            retained_roles.extend(
                authority
                    .package
                    .artifacts
                    .iter()
                    .take(position)
                    .map(|artifact| format!("package-{}", artifact.role)),
            );
            let mut artifact_failure = retain_supplemental(&retained_roles);
            let contract = format!("package.post.artifact.{}", failed.role);
            assert_eq!(
                validate_failed_package_postprocess(
                    bundle.path(),
                    &artifact_failure,
                    &authority,
                    gate,
                    &contract,
                ),
                Vec::<String>::new()
            );
            assert_production_failure(&mut artifact_failure, &contract);
        }
        let mut all_artifacts = vec!["package-manifest".to_string()];
        all_artifacts.extend(
            authority
                .package
                .artifacts
                .iter()
                .map(|artifact| format!("package-{}", artifact.role)),
        );
        let mut ownership_failure = retain_supplemental(&all_artifacts);
        assert!(validate_failed_package_postprocess(
            bundle.path(),
            &ownership_failure,
            &authority,
            gate,
            "package.post.ownership-report",
        )
        .is_empty());
        assert_production_failure(&mut ownership_failure, "package.post.ownership-report");
        all_artifacts.push("ownership-production-report".to_string());
        let mut dependency_retention = retain_supplemental(&all_artifacts);
        assert!(validate_failed_package_postprocess(
            bundle.path(),
            &dependency_retention,
            &authority,
            gate,
            "package.post.dependencies-retain",
        )
        .is_empty());
        assert_production_failure(
            &mut dependency_retention,
            "package.post.dependencies-retain",
        );
        all_artifacts.push("native-dependency-capture".to_string());
        let mut dependency_validation = retain_supplemental(&all_artifacts);
        let unexpected_valid_dependency = retain_supplemental(&all_artifacts);
        assert!(validate_failed_package_postprocess(
            bundle.path(),
            &unexpected_valid_dependency,
            &authority,
            gate,
            "package.post.dependencies-validate",
        )
        .iter()
        .any(|contract| contract == "evidence.package.post.dependencies.unexpected_valid"));
        rewrite_bundle_entry(
            bundle.path(),
            &mut dependency_validation.entries,
            "native-dependency-capture",
            br#"{"schema":"uqm-native-dependency-capture-v1","target":"linux-x86_64","dependencies":[]}"#,
        );
        assert!(validate_failed_package_postprocess(
            bundle.path(),
            &dependency_validation,
            &authority,
            gate,
            "package.post.dependencies-validate",
        )
        .is_empty());
        assert_production_failure(
            &mut dependency_validation,
            "package.post.dependencies-validate",
        );

        let failed_role = &authority.package.artifacts[2].role;
        let extra_artifact = retain_supplemental(&[
            "package-manifest".to_string(),
            format!("package-{}", authority.package.artifacts[0].role),
            format!("package-{}", authority.package.artifacts[1].role),
            format!("package-{failed_role}"),
        ]);
        assert!(validate_failed_package_postprocess(
            bundle.path(),
            &extra_artifact,
            &authority,
            gate,
            &format!("package.post.artifact.{failed_role}"),
        )
        .iter()
        .any(|contract| contract == "evidence.package.post.supplemental_prefix"));
    }

    #[test]
    fn successful_builtin_replay_requires_exact_subordinate_receipts() {
        let root = std::env::temp_dir().join(format!(
            "uqm-evidence-builtin-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        fs::create_dir_all(&root).unwrap();
        let index = successful_workflow_bundle(&root);
        assert_eq!(production_contracts(&root, &index), Vec::<String>::new());

        let omitted = index
            .entries
            .iter()
            .position(|entry| entry.role == "step.result")
            .unwrap();
        let mut forged = index.clone();
        forged.entries.remove(omitted);

        assert!(production_contracts(&root, &forged)
            .iter()
            .any(|contract| contract == "evidence.builtin.workflow.actionlint.triplet"));

        let validation_entry = index
            .entries
            .iter()
            .find(|entry| entry.role == "workflow.validation")
            .unwrap();
        let exact_validation: serde_json::Value =
            serde_json::from_slice(&read_bundle_file(&root, &validation_entry.path).unwrap())
                .unwrap();
        let mut forged = index.clone();
        let mut unknown_field = exact_validation.clone();
        unknown_field["unexpected"] = serde_json::json!(true);
        rewrite_bundle_entry(
            &root,
            &mut forged.entries,
            "workflow.validation",
            &serde_json::to_vec(&unknown_field).unwrap(),
        );
        assert!(production_contracts(&root, &forged)
            .iter()
            .any(|contract| contract == "evidence.builtin.workflow.validation.content"));

        let mut forged = index.clone();
        let mut unknown_rule_field = exact_validation;
        unknown_rule_field["rules"][0]["unexpected"] = serde_json::json!(true);
        rewrite_bundle_entry(
            &root,
            &mut forged.entries,
            "workflow.validation",
            &serde_json::to_vec(&unknown_rule_field).unwrap(),
        );
        assert!(production_contracts(&root, &forged)
            .iter()
            .any(|contract| contract == "evidence.builtin.workflow.validation.content"));

        let validation = serde_json::json!({
            "schema": "uqm-s4-workflow-validation-v1",
            "first_failed_rule": null,
            "rules": []
        });
        let mut forged = index;

        rewrite_bundle_entry(
            &root,
            &mut forged.entries,
            "workflow.validation",
            &serde_json::to_vec(&validation).unwrap(),
        );
        assert!(production_contracts(&root, &forged)
            .iter()
            .any(|contract| contract == "evidence.builtin.workflow.validation.content"));
        let _ = fs::remove_dir_all(root);
    }
    #[test]
    fn mutation_replay_order_comes_only_from_embedded_authority() {
        let root = std::env::temp_dir().join(format!(
            "uqm-evidence-mutation-order-{}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        let mut index = successful_mutations_bundle(&root);

        let authority_entry = index
            .entries
            .iter()
            .find(|entry| entry.role == "authority.snapshot")
            .unwrap();
        let mut authority: Authority =
            serde_json::from_slice(&read_bundle_file(&root, &authority_entry.path).unwrap())
                .unwrap();
        authority.mutation_targets.reverse();
        validate_authority(&authority).unwrap();
        rewrite_bundle_entry(
            &root,
            &mut index.entries,
            "authority.snapshot",
            &serde_json::to_vec(&authority).unwrap(),
        );

        let receipt_entry = index
            .entries
            .iter()
            .find(|entry| entry.role == "mutations.receipt")
            .unwrap();
        let mut receipt: serde_json::Value =
            serde_json::from_slice(&read_bundle_file(&root, &receipt_entry.path).unwrap()).unwrap();
        receipt["cases"].as_array_mut().unwrap().reverse();
        rewrite_bundle_entry(
            &root,
            &mut index.entries,
            "mutations.receipt",
            &serde_json::to_vec(&receipt).unwrap(),
        );

        assert_eq!(production_contracts(&root, &index), Vec::<String>::new());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn successful_mutation_replay_requires_retained_fixture_bytes() {
        let root =
            std::env::temp_dir().join(format!("uqm-evidence-mutations-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let index = successful_mutations_bundle(&root);
        assert_eq!(production_contracts(&root, &index), Vec::<String>::new());

        let mut missing = index.clone();
        let fixture = missing
            .entries
            .iter()
            .position(|entry| entry.role == "mutation.fixture")
            .unwrap();
        missing.entries.remove(fixture);
        assert!(production_contracts(&root, &missing)
            .iter()
            .any(|contract| contract == "evidence.builtin.mutations.fixture.count"));

        let receipt_entry = index
            .entries
            .iter()
            .find(|entry| entry.role == "mutations.receipt")
            .unwrap();
        let mut receipt: serde_json::Value =
            serde_json::from_slice(&read_bundle_file(&root, &receipt_entry.path).unwrap()).unwrap();
        let original_receipt = receipt.clone();
        receipt["cases"][0]["files"][0]["sha256"] = serde_json::json!("0".repeat(64));
        let mut forged = index.clone();
        rewrite_bundle_entry(
            &root,
            &mut forged.entries,
            "mutations.receipt",
            &serde_json::to_vec(&receipt).unwrap(),
        );
        assert!(production_contracts(&root, &forged)
            .iter()
            .any(|contract| { contract == "evidence.builtin.mutations.fixture.format.mutant.0" }));

        let mut forged_baseline_receipt = original_receipt.clone();
        forged_baseline_receipt["cases"][0]["baseline_files"][0]["sha256"] =
            serde_json::json!("0".repeat(64));
        let mut forged_baseline = index.clone();
        rewrite_bundle_entry(
            &root,
            &mut forged_baseline.entries,
            "mutations.receipt",
            &serde_json::to_vec(&forged_baseline_receipt).unwrap(),
        );
        assert!(production_contracts(&root, &forged_baseline)
            .iter()
            .any(|contract| {
                contract == "evidence.builtin.mutations.fixture.format.baseline.0"
            }));

        let assert_fixture_declaration_rejected = |mutate: fn(&mut serde_json::Value)| {
            let mut candidate = index.clone();
            let mut candidate_receipt = original_receipt.clone();
            mutate(&mut candidate_receipt);
            rewrite_bundle_entry(
                &root,
                &mut candidate.entries,
                "mutations.receipt",
                &serde_json::to_vec(&candidate_receipt).unwrap(),
            );
            assert!(production_contracts(&root, &candidate)
                .iter()
                .any(|contract| {
                    contract == "evidence.builtin.mutations.fixture.format.mutant.0"
                }));
        };
        assert_fixture_declaration_rejected(|receipt| {
            receipt["cases"][0]["files"][0]["path"] =
                serde_json::json!("payloads/mutation.fixture/format/mutant/0/wrong.rs");
        });
        assert_fixture_declaration_rejected(|receipt| {
            receipt["cases"][0]["files"][0]["byte_length"] = serde_json::json!(0);
        });
        let mut wrong_producer = index.clone();
        wrong_producer
            .entries
            .iter_mut()
            .find(|entry| entry.role == "mutation.fixture")
            .unwrap()
            .producing_gate = "format".into();
        rewrite_bundle_entry(
            &root,
            &mut wrong_producer.entries,
            "mutations.receipt",
            &serde_json::to_vec(&original_receipt).unwrap(),
        );
        assert!(production_contracts(&root, &wrong_producer)
            .iter()
            .any(|contract| {
                contract == "evidence.builtin.mutations.fixture.format.baseline.0"
            }));

        let assert_execution_rejected = |receipt: &serde_json::Value| {
            let mut candidate = index.clone();
            rewrite_bundle_entry(
                &root,
                &mut candidate.entries,
                "mutations.receipt",
                &serde_json::to_vec(receipt).unwrap(),
            );
            assert!(production_contracts(&root, &candidate)
                .iter()
                .any(|contract| contract == "evidence.builtin.mutations.receipt.content"));
        };
        let mut wrong_execution = original_receipt.clone();
        wrong_execution["cases"][0]["executions"][0]["command"] =
            serde_json::json!(["cargo", "fmt"]);
        assert_execution_rejected(&wrong_execution);

        let mut forged_success = original_receipt.clone();
        forged_success["cases"][0]["executions"][0]["success"] = serde_json::json!(true);
        assert_execution_rejected(&forged_success);

        let mut zero_exit = original_receipt.clone();
        zero_exit["cases"][0]["executions"][0]["exit_code"] = serde_json::json!(0);
        assert_execution_rejected(&zero_exit);

        let mut launch_error = original_receipt.clone();
        launch_error["cases"][0]["executions"][0]["exit_code"] = serde_json::Value::Null;
        launch_error["cases"][0]["executions"][0]["launch_error"] =
            serde_json::json!("cargo did not launch");
        assert_execution_rejected(&launch_error);

        let mut contradictory_signal = original_receipt.clone();
        contradictory_signal["cases"][0]["executions"][0]["signal"] = serde_json::json!(9);
        assert_execution_rejected(&contradictory_signal);

        let mut missing_field = original_receipt.clone();
        missing_field["cases"][0]["executions"][0]
            .as_object_mut()
            .unwrap()
            .remove("stderr");
        assert_execution_rejected(&missing_field);

        let mut extra_field = original_receipt.clone();
        extra_field["cases"][0]["executions"][0]["forged"] = serde_json::json!(true);
        assert_execution_rejected(&extra_field);

        for (field, forged) in [
            ("path", serde_json::json!("relative/tool")),
            ("byte_length", serde_json::json!(0)),
            ("sha256", serde_json::json!("0")),
            ("mode", serde_json::json!(0o644)),
        ] {
            let mut forged_identity = original_receipt.clone();
            forged_identity["cases"][0]["executions"][0]["executable_identity"][field] = forged;
            assert_execution_rejected(&forged_identity);
        }
        let mut missing_identity = original_receipt.clone();
        missing_identity["cases"][0]["executions"][0]["executable_identity"] =
            serde_json::Value::Null;
        assert_execution_rejected(&missing_identity);

        for field in ["gate", "step", "cwd", "native_profile"] {
            let mut wrong_route = original_receipt.clone();
            wrong_route["cases"][0]["executions"][0][field] = serde_json::json!("forged");
            assert_execution_rejected(&wrong_route);
        }

        let check_position = original_receipt["cases"]
            .as_array()
            .unwrap()
            .iter()
            .position(|case| case["target"] == "check")
            .unwrap();
        let mut omitted_baseline_step = original_receipt.clone();
        omitted_baseline_step["cases"][check_position]["baseline_executions"]
            .as_array_mut()
            .unwrap()
            .pop();
        assert_execution_rejected(&omitted_baseline_step);

        let mut reordered_baseline = original_receipt.clone();
        reordered_baseline["cases"][check_position]["baseline_executions"]
            .as_array_mut()
            .unwrap()
            .swap(0, 1);
        assert_execution_rejected(&reordered_baseline);

        let mut extra_mutant_step = original_receipt.clone();
        let extra = extra_mutant_step["cases"][check_position]["executions"][0].clone();
        extra_mutant_step["cases"][check_position]["executions"]
            .as_array_mut()
            .unwrap()
            .push(extra);
        assert_execution_rejected(&extra_mutant_step);

        let authority: Authority =
            serde_json::from_slice(include_bytes!("../../../ci/gates.json")).unwrap();
        let assert_semantic_rejected = |target: &str, file_name: &str, replacement: Vec<u8>| {
            let mut candidate = index.clone();
            let mut candidate_receipt = original_receipt.clone();
            let case = candidate_receipt["cases"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|case| case["target"] == target)
                .unwrap();
            let file = case["files"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|file| {
                    file["path"]
                        .as_str()
                        .and_then(|path| Path::new(path).file_name())
                        .and_then(|name| name.to_str())
                        == Some(file_name)
                })
                .unwrap();
            let path = file["path"].as_str().unwrap().to_string();
            fs::write(root.join(&path), &replacement).unwrap();
            let entry = candidate
                .entries
                .iter_mut()
                .find(|entry| entry.path == path)
                .unwrap();
            entry.byte_length = replacement.len() as u64;
            entry.sha256 = hex_sha256(&replacement);
            file["byte_length"] = serde_json::json!(replacement.len());
            file["sha256"] = serde_json::json!(hex_sha256(&replacement));
            rewrite_bundle_entry(
                &root,
                &mut candidate.entries,
                "mutations.receipt",
                &serde_json::to_vec(&candidate_receipt).unwrap(),
            );
            assert!(production_contracts(&root, &candidate).iter().any(
                |contract| contract == &format!("evidence.builtin.mutations.semantic.{target}")
            ));
            let original = mutation_fixture_bytes(target, &authority)
                .into_iter()
                .find(|(name, _)| {
                    Path::new(name).file_name().and_then(|name| name.to_str()) == Some(file_name)
                })
                .unwrap()
                .1;
            fs::write(root.join(path), original).unwrap();
        };
        assert_semantic_rejected(
            "ownership",
            "authority.json",
            serde_json::to_vec_pretty(&authority).unwrap(),
        );
        assert_semantic_rejected(
            "link",
            "provider-manifest.json",
            include_bytes!("../../../ownership/native-provider-manifest.json").to_vec(),
        );
        assert_semantic_rejected(
            "harness",
            "run_p00_harness.sh",
            b"#!/bin/sh\necho RESULT=PASS\n".to_vec(),
        );
        assert_semantic_rejected(
            "security",
            "authority.json",
            serde_json::to_vec_pretty(&authority).unwrap(),
        );
        let mut passing_cache = mutation_fixture_bytes("cache", &authority)[0].1.clone();
        let mut passing_cache_json: serde_json::Value =
            serde_json::from_slice(&passing_cache).unwrap();
        passing_cache_json["registry_cache_present"] = serde_json::json!(false);
        passing_cache = serde_json::to_vec_pretty(&passing_cache_json).unwrap();
        assert_semantic_rejected("cache", "cache-initial-state.json", passing_cache);
        assert_semantic_rejected(
            "workflow",
            "rust-quality.yaml",
            include_bytes!("../../../../.github/workflows/rust-quality.yaml").to_vec(),
        );
        let baseline_tool = baseline_mutation_fixture_bytes("artifact", &authority)
            .into_iter()
            .find_map(|(name, bytes)| (name == "tool-preflight.json").then_some(bytes))
            .unwrap();
        assert_semantic_rejected("artifact", "tool-preflight.json", baseline_tool);

        let mut semantic = index;
        rewrite_bundle_entry(
            &root,
            &mut semantic.entries,
            "mutations.receipt",
            &serde_json::to_vec(&original_receipt).unwrap(),
        );
        let mut semantic_receipt = original_receipt;
        let coverage_case = semantic_receipt["cases"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|case| case["target"] == "coverage")
            .unwrap();
        let coverage_path = coverage_case["files"][0]["path"]
            .as_str()
            .unwrap()
            .to_string();
        let passing_coverage = b"LF:4\nLH:4\n";
        fs::write(root.join(&coverage_path), passing_coverage).unwrap();
        let coverage_entry = semantic
            .entries
            .iter_mut()
            .find(|entry| entry.path == coverage_path)
            .unwrap();
        coverage_entry.byte_length = passing_coverage.len() as u64;
        coverage_entry.sha256 = hex_sha256(passing_coverage);
        coverage_case["files"][0]["byte_length"] = serde_json::json!(passing_coverage.len());
        coverage_case["files"][0]["sha256"] = serde_json::json!(hex_sha256(passing_coverage));
        rewrite_bundle_entry(
            &root,
            &mut semantic.entries,
            "mutations.receipt",
            &serde_json::to_vec(&semantic_receipt).unwrap(),
        );
        assert!(production_contracts(&root, &semantic)
            .iter()
            .any(|contract| contract == "evidence.builtin.mutations.semantic.coverage"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn tool_mutation_replay_rejects_unrelated_failures_and_source_changes() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let index = successful_mutations_bundle(root);
        let receipt_entry = index
            .entries
            .iter()
            .find(|entry| entry.role == "mutations.receipt")
            .unwrap();
        let receipt: serde_json::Value =
            serde_json::from_slice(&read_bundle_file(root, &receipt_entry.path).unwrap()).unwrap();
        let format_position = receipt["cases"]
            .as_array()
            .unwrap()
            .iter()
            .position(|case| case["target"] == "format")
            .unwrap();

        let mut unrelated = receipt.clone();
        let terminal = unrelated["cases"][format_position]["executions"]
            .as_array_mut()
            .unwrap()
            .last_mut()
            .unwrap();
        terminal["stdout"] = serde_json::json!("");
        terminal["stderr"] = serde_json::json!("unrelated command failure");
        let mut unrelated_index = index.clone();
        rewrite_bundle_entry(
            root,
            &mut unrelated_index.entries,
            "mutations.receipt",
            &serde_json::to_vec(&unrelated).unwrap(),
        );
        assert!(production_contracts(root, &unrelated_index)
            .iter()
            .any(|contract| contract == "evidence.builtin.mutations.semantic.format"));

        let mut wrong_recipe = receipt.clone();
        wrong_recipe["cases"][format_position]["recipe"]["path"] =
            serde_json::json!("src/unrelated.rs");
        let mut wrong_recipe_index = index.clone();
        rewrite_bundle_entry(
            root,
            &mut wrong_recipe_index.entries,
            "mutations.receipt",
            &serde_json::to_vec(&wrong_recipe).unwrap(),
        );
        assert!(production_contracts(root, &wrong_recipe_index)
            .iter()
            .any(|contract| contract == "evidence.builtin.mutations.receipt.content"));

        let mut changed_companion = receipt;
        let companion = changed_companion["cases"][format_position]["files"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|file| file["path"].as_str().unwrap().ends_with("/Cargo.toml"))
            .unwrap();
        let companion_path = companion["path"].as_str().unwrap().to_string();
        let replacement = b"[package]\nname='unrelated'\nversion='0.1.0'\n";
        fs::write(root.join(&companion_path), replacement).unwrap();
        companion["byte_length"] = serde_json::json!(replacement.len());
        companion["sha256"] = serde_json::json!(hex_sha256(replacement));
        let mut changed_companion_index = index;
        let companion_entry = changed_companion_index
            .entries
            .iter_mut()
            .find(|entry| entry.path == companion_path)
            .unwrap();
        companion_entry.byte_length = replacement.len() as u64;
        companion_entry.sha256 = hex_sha256(replacement);
        rewrite_bundle_entry(
            root,
            &mut changed_companion_index.entries,
            "mutations.receipt",
            &serde_json::to_vec(&changed_companion).unwrap(),
        );
        let contracts = production_contracts(root, &changed_companion_index);
        assert!(!contracts
            .iter()
            .any(|contract| contract == "evidence.builtin.mutations.fixture.format.mutant.0"));
        assert!(contracts
            .iter()
            .any(|contract| contract == "evidence.builtin.mutations.semantic.format"));
    }

    #[test]
    fn failed_workflow_replay_requires_truthful_retained_rules() {
        let root = std::env::temp_dir().join(format!(
            "uqm-evidence-failed-workflow-{}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        let mut index = successful_workflow_bundle(&root);
        index.first_failed_contract = Some("workflow".into());

        let validation_entry = index
            .entries
            .iter()
            .find(|entry| entry.role == "workflow.validation")
            .unwrap();
        let mut validation: serde_json::Value =
            serde_json::from_slice(&read_bundle_file(&root, &validation_entry.path).unwrap())
                .unwrap();
        validation["first_failed_rule"] = serde_json::json!("workflow.timeouts");
        validation["rules"][7]["passed"] = serde_json::json!(false);
        rewrite_bundle_entry(
            &root,
            &mut index.entries,
            "workflow.validation",
            &serde_json::to_vec(&validation).unwrap(),
        );

        let gate_entry = index
            .entries
            .iter()
            .find(|entry| entry.role == "gate.result")
            .unwrap();
        let mut gate_result: serde_json::Value =
            serde_json::from_slice(&read_bundle_file(&root, &gate_entry.path).unwrap()).unwrap();
        gate_result["passed"] = serde_json::json!(false);
        gate_result["first_failed_contract"] = serde_json::json!("workflow");
        gate_result["detail"] = serde_json::json!("workflow.timeouts rejected the fixture");
        rewrite_bundle_entry(
            &root,
            &mut index.entries,
            "gate.result",
            &serde_json::to_vec(&gate_result).unwrap(),
        );

        assert_eq!(production_contracts(&root, &index), Vec::<String>::new());

        let valid_validation = validation.clone();
        validation["unexpected"] = serde_json::json!(true);
        rewrite_bundle_entry(
            &root,
            &mut index.entries,
            "workflow.validation",
            &serde_json::to_vec(&validation).unwrap(),
        );
        assert!(production_contracts(&root, &index)
            .iter()
            .any(|contract| contract == "evidence.builtin.workflow.validation.failed_content"));

        validation = valid_validation.clone();
        validation["rules"][0]["unexpected"] = serde_json::json!(true);
        rewrite_bundle_entry(
            &root,
            &mut index.entries,
            "workflow.validation",
            &serde_json::to_vec(&validation).unwrap(),
        );
        assert!(production_contracts(&root, &index)
            .iter()
            .any(|contract| contract == "evidence.builtin.workflow.validation.failed_content"));

        validation = valid_validation.clone();
        validation.as_object_mut().unwrap().remove("rules");
        rewrite_bundle_entry(
            &root,
            &mut index.entries,
            "workflow.validation",
            &serde_json::to_vec(&validation).unwrap(),
        );
        assert!(production_contracts(&root, &index)
            .iter()
            .any(|contract| contract == "evidence.builtin.workflow.validation.failed_content"));

        validation = valid_validation.clone();
        validation["rules"][0]
            .as_object_mut()
            .unwrap()
            .remove("detail");
        rewrite_bundle_entry(
            &root,
            &mut index.entries,
            "workflow.validation",
            &serde_json::to_vec(&validation).unwrap(),
        );
        assert!(production_contracts(&root, &index)
            .iter()
            .any(|contract| contract == "evidence.builtin.workflow.validation.failed_content"));

        validation = valid_validation;
        validation["first_failed_rule"] = serde_json::json!("workflow.actionlint");
        rewrite_bundle_entry(
            &root,
            &mut index.entries,
            "workflow.validation",
            &serde_json::to_vec(&validation).unwrap(),
        );
        assert!(production_contracts(&root, &index)
            .iter()
            .any(|contract| contract == "evidence.builtin.workflow.validation.failed_content"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn failed_builtin_steps_require_truthful_terminal_results() {
        let root = std::env::temp_dir().join(format!(
            "uqm-evidence-failed-builtins-{}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        let authority: Authority =
            serde_json::from_slice(include_bytes!("../../../ci/gates.json")).unwrap();

        let mut complexity = valid_index();
        complexity.entries.clear();
        let complexity_command = std::iter::once("lizard".to_string())
            .chain(authority.complexity.lizard_arguments.iter().cloned())
            .chain(std::iter::once("rust/src/lib.rs".to_string()))
            .collect::<Vec<_>>();
        for (path, role, bytes) in [
            ("complexity/lizard.stdout.log", "step.stdout", Vec::new()),
            ("complexity/lizard.stderr.log", "step.stderr", Vec::new()),
            (
                "complexity/lizard.result.json",
                "step.result",
                serde_json::to_vec(&serde_json::json!({
                    "schema": "uqm-s4-step-result-v2",
                    "gate": "complexity",
                    "step": "lizard",
                    "command": complexity_command,
                    "effective_command": complexity_command,
                    "staged_script_sha256": null,
                    "executable_identity": fixture_executable_identity(None),
                    "success": false,
                    "exit_code": 1,
                    "signal": null,
                    "launch_error": null,
                    "supervision": fixture_supervision(false, 0, 0),
                }))
                .unwrap(),
            ),
        ] {
            write_bundle_entry(
                &root,
                &mut complexity.entries,
                path,
                role,
                &complexity_command,
                &bytes,
            );
            complexity.entries.last_mut().unwrap().producing_gate = "complexity".into();
        }
        assert_eq!(
            validate_failed_builtin_gate(
                &root,
                &complexity,
                &authority,
                "complexity",
                "complexity.maximum",
            ),
            Vec::<String>::new()
        );
        let mut unexpected = complexity.clone();
        write_bundle_entry(
            &root,
            &mut unexpected.entries,
            "payloads/coverage.lcov/coverage.lcov",
            "coverage.lcov",
            &complexity_command,
            b"LF:1\nLH:0\n",
        );
        unexpected.entries.last_mut().unwrap().producing_gate = "complexity".into();
        assert!(validate_failed_builtin_gate(
            &root,
            &unexpected,
            &authority,
            "complexity",
            "complexity.maximum",
        )
        .iter()
        .any(|contract| contract
            == "evidence.builtin.complexity.unexpected_failure_role.coverage.lcov"));
        complexity.entries.pop();
        assert!(validate_failed_builtin_gate(
            &root,
            &complexity,
            &authority,
            "complexity",
            "complexity.maximum",
        )
        .iter()
        .any(|contract| contract == "evidence.builtin.complexity.lizard.triplet"));
        let launch_result = serde_json::to_vec(&serde_json::json!({
            "schema": "uqm-s4-step-result-v2",
            "gate": "complexity",
            "step": "lizard",
            "command": complexity_command,
            "effective_command": complexity_command,
            "staged_script_sha256": null,
            "executable_identity": fixture_executable_identity(Some("cannot execute lizard: not found")),
            "success": false,
            "exit_code": null,
            "signal": null,
            "launch_error": "cannot execute lizard: not found",
            "supervision": fixture_supervision(true, 0, 0),
        }))
        .unwrap();
        write_bundle_entry(
            &root,
            &mut complexity.entries,
            "complexity/lizard.result.json",
            "step.result",
            &complexity_command,
            &launch_result,
        );
        complexity.entries.last_mut().unwrap().producing_gate = "complexity".into();
        assert_eq!(
            validate_failed_builtin_gate(
                &root,
                &complexity,
                &authority,
                "complexity",
                "complexity.exec",
            ),
            Vec::<String>::new()
        );
        for (field, forged_value) in [
            ("success", serde_json::json!(true)),
            ("exit_code", serde_json::json!(127)),
            ("signal", serde_json::json!(9)),
            ("launch_error", serde_json::json!("")),
        ] {
            let mut forged: serde_json::Value = serde_json::from_slice(&launch_result).unwrap();
            forged[field] = forged_value;
            rewrite_bundle_entry(
                &root,
                &mut complexity.entries,
                "step.result",
                &serde_json::to_vec(&forged).unwrap(),
            );
            assert!(validate_failed_builtin_gate(
                &root,
                &complexity,
                &authority,
                "complexity",
                "complexity.exec",
            )
            .iter()
            .any(|contract| contract == "evidence.builtin.complexity.lizard.result"));
        }
        for field in ["success", "exit_code", "signal", "launch_error"] {
            let mut forged: serde_json::Value = serde_json::from_slice(&launch_result).unwrap();
            forged.as_object_mut().unwrap().remove(field);
            rewrite_bundle_entry(
                &root,
                &mut complexity.entries,
                "step.result",
                &serde_json::to_vec(&forged).unwrap(),
            );
            assert!(validate_failed_builtin_gate(
                &root,
                &complexity,
                &authority,
                "complexity",
                "complexity.exec",
            )
            .iter()
            .any(|contract| contract == "evidence.builtin.complexity.lizard.result"));
        }
        rewrite_bundle_entry(
            &root,
            &mut complexity.entries,
            "step.result",
            &launch_result,
        );

        let mut bootstrap = valid_index();
        bootstrap.entries.clear();
        let build: Vec<String> = vec![
            "cargo".into(),
            "build".into(),
            "--locked".into(),
            "--manifest-path".into(),
            "rust/Cargo.toml".into(),
            "--bin".into(),
            "uqm-gameplay-proof".into(),
        ];
        for (path, role, bytes) in [
            (
                "bootstrap-proof/build-runner.stdout.log",
                "step.stdout",
                Vec::new(),
            ),
            (
                "bootstrap-proof/build-runner.stderr.log",
                "step.stderr",
                Vec::new(),
            ),
            (
                "bootstrap-proof/build-runner.result.json",
                "step.result",
                serde_json::to_vec(&serde_json::json!({
                    "schema": "uqm-s4-step-result-v2",
                    "gate": "bootstrap-proof",
                    "step": "build-runner",
                    "command": build,
                    "effective_command": build,
                    "staged_script_sha256": null,
                    "executable_identity": fixture_executable_identity(None),
                    "success": false,
                    "exit_code": 101,
                    "signal": null,
                    "launch_error": null,
                    "supervision": fixture_supervision(false, 0, 0),
                }))
                .unwrap(),
            ),
        ] {
            write_bundle_entry(&root, &mut bootstrap.entries, path, role, &build, &bytes);
            bootstrap.entries.last_mut().unwrap().producing_gate = "bootstrap-proof".into();
        }
        let package_command: Vec<String> = vec![
            "cargo".into(),
            "run".into(),
            "--locked".into(),
            "--manifest-path".into(),
            "rust/xtask/Cargo.toml".into(),
            "--".into(),
            "package".into(),
        ];
        let executable_hash = hex_sha256(b"fixture");
        let package_artifacts: Vec<_> = authority
            .package
            .artifacts
            .iter()
            .map(|artifact| {
                serde_json::json!({
                    "role": artifact.role,
                    "path": if artifact.role == "executable" {
                        "rust/target/uqm-package/x86_64-unknown-linux-gnu/uqm".to_string()
                    } else {
                        format!("rust/target/{}", artifact.role)
                    },
                    "media_type": artifact.media_type,
                    "byte_length": if artifact.role == "executable" { 7 } else { 1 },
                    "sha256": if artifact.role == "executable" {
                        executable_hash.clone()
                    } else {
                        "0".repeat(64)
                    },
                    "producing_command": artifact.producing_command
                })
            })
            .collect();
        let package_manifest = serde_json::to_vec(&package_manifest_fixture(
            &authority,
            &bootstrap,
            package_artifacts,
        ))
        .unwrap();
        for (path, role) in [
            (
                "payloads/bootstrap-proof.package-manifest/production-artifacts.json",
                "bootstrap-proof.package-manifest",
            ),
            (
                "payloads/bootstrap-proof.executable/uqm",
                "bootstrap-proof.executable",
            ),
            (
                "payloads/bootstrap-proof.profile/main-menu-v1.json",
                "bootstrap-proof.profile",
            ),
        ] {
            write_bundle_entry(
                &root,
                &mut bootstrap.entries,
                path,
                role,
                &package_command,
                if role == "bootstrap-proof.profile" {
                    include_bytes!("../../../scripts/main-menu-v1.json")
                } else if role == "bootstrap-proof.package-manifest" {
                    &package_manifest
                } else {
                    b"fixture"
                },
            );
            bootstrap.entries.last_mut().unwrap().producing_gate = "bootstrap-proof".into();
        }
        let mut profile_failure = valid_index();
        profile_failure.entries.clear();
        assert_eq!(
            validate_failed_builtin_gate(
                &root,
                &profile_failure,
                &authority,
                "bootstrap-proof",
                "bootstrap-proof.profile",
            ),
            Vec::<String>::new()
        );
        let unexpected_command = vec!["unexpected-cleanup".into()];
        write_builtin_step_fixture(
            &root,
            &mut profile_failure.entries,
            "bootstrap-proof",
            "teardown",
            &unexpected_command,
            (Some(0), None, None),
        );
        let contracts = validate_failed_builtin_gate(
            &root,
            &profile_failure,
            &authority,
            "bootstrap-proof",
            "bootstrap-proof.profile",
        );
        assert!(contracts
            .iter()
            .any(|contract| contract == "evidence.builtin.bootstrap-proof.pre_step_receipts"));
        profile_failure.entries.clear();
        write_bundle_entry(
            &root,
            &mut profile_failure.entries,
            "payloads/bootstrap-proof.profile/unexpected.json",
            "bootstrap-proof.profile",
            &package_command,
            b"{}",
        );
        profile_failure.entries.last_mut().unwrap().producing_gate = "bootstrap-proof".into();
        let contracts = validate_failed_builtin_gate(
            &root,
            &profile_failure,
            &authority,
            "bootstrap-proof",
            "bootstrap-proof.profile",
        );
        assert!(contracts
            .iter()
            .any(|contract| contract == "evidence.builtin.bootstrap-proof.failed_payload_count"));

        assert_eq!(
            validate_failed_builtin_gate(
                &root,
                &bootstrap,
                &authority,
                "bootstrap-proof",
                "bootstrap-proof.build",
            ),
            Vec::<String>::new()
        );
        rewrite_builtin_step_result(
            &root,
            &mut bootstrap.entries,
            "bootstrap-proof",
            "build-runner",
            &build,
            (Some(0), None, None),
        );
        write_bundle_entry(
            &root,
            &mut bootstrap.entries,
            "payloads/bootstrap-proof.runner/uqm-gameplay-proof",
            "bootstrap-proof.runner",
            &build,
            b"runner",
        );
        bootstrap.entries.last_mut().unwrap().producing_gate = "bootstrap-proof".into();
        assert_eq!(
            validate_failed_builtin_gate(
                &root,
                &bootstrap,
                &authority,
                "bootstrap-proof",
                "bootstrap-proof.output",
            ),
            Vec::<String>::new()
        );

        let run_command = vec![
            "/tmp/repository/rust/target/debug/uqm-gameplay-proof".into(),
            "run".into(),
            "/tmp/repository".into(),
            "/tmp/repository/rust/target/uqm-package/production-artifacts.json".into(),
            "/tmp/repository/rust/scripts/main-menu-v1.json".into(),
            "/tmp/evidence/bootstrap-proof".into(),
        ];
        write_builtin_step_fixture(
            &root,
            &mut bootstrap.entries,
            "bootstrap-proof",
            "run",
            &run_command,
            (Some(2), None, None),
        );
        assert_eq!(
            validate_failed_builtin_gate(
                &root,
                &bootstrap,
                &authority,
                "bootstrap-proof",
                "bootstrap-proof.run",
            ),
            Vec::<String>::new()
        );
        assert_eq!(
            validate_failed_builtin_gate(
                &root,
                &bootstrap,
                &authority,
                "bootstrap-proof",
                "bootstrap-proof.failure-retain",
            ),
            Vec::<String>::new()
        );
        rewrite_builtin_step_result(
            &root,
            &mut bootstrap.entries,
            "bootstrap-proof",
            "run",
            &run_command,
            (Some(0), None, None),
        );
        let validate_command = vec![
            "/tmp/repository/rust/target/debug/uqm-gameplay-proof".into(),
            "validate".into(),
            "/tmp/evidence/bootstrap-proof/lcar-v1.json".into(),
        ];
        write_builtin_step_fixture(
            &root,
            &mut bootstrap.entries,
            "bootstrap-proof",
            "validate",
            &validate_command,
            (Some(2), None, None),
        );
        write_bundle_entry(
            &root,
            &mut bootstrap.entries,
            "payloads/bootstrap-proof.lcar/lcar-v1.json",
            "bootstrap-proof.lcar",
            &run_command,
            b"malformed LCAR rejected by validation",
        );
        bootstrap.entries.last_mut().unwrap().producing_gate = "bootstrap-proof".into();
        assert!(validate_failed_builtin_gate(
            &root,
            &bootstrap,
            &authority,
            "bootstrap-proof",
            "bootstrap-proof.validate",
        )
        .iter()
        .any(|contract| contract == "evidence.builtin.bootstrap-proof.lcar.content"));
        let _ = fs::remove_dir_all(root);
    }

    fn successful_bootstrap_bundle(
        root: &Path,
    ) -> (EvidenceIndex, serde_json::Value, Vec<String>, Vec<String>) {
        fs::create_dir_all(root).unwrap();
        let authority: Authority =
            serde_json::from_slice(include_bytes!("../../../ci/gates.json")).unwrap();
        let mut index = valid_index();
        index.entries.clear();
        let package_command: Vec<String> = vec![
            "cargo".into(),
            "run".into(),
            "--locked".into(),
            "--manifest-path".into(),
            "rust/xtask/Cargo.toml".into(),
            "--".into(),
            "package".into(),
        ];
        let run_command: Vec<String> = vec![
            "/tmp/repository/rust/target/debug/uqm-gameplay-proof".into(),
            "run".into(),
            "/tmp/repository".into(),
            "/tmp/repository/rust/target/uqm-package/x86_64-unknown-linux-gnu/production-artifacts.json".into(),
            "/tmp/repository/rust/scripts/main-menu-v1.json".into(),
            "/tmp/evidence/bootstrap-proof".into(),
        ];
        let executable = b"\x7fELF bootstrap executable";
        let profile = include_bytes!("../../../scripts/main-menu-v1.json");
        let package_artifacts: Vec<_> = authority
            .package
            .artifacts
            .iter()
            .map(|artifact| {
                serde_json::json!({
                    "role": artifact.role,
                    "media_type": artifact.media_type,
                    "producing_command": artifact.producing_command,
                    "path": if artifact.role == "executable" {
                        "rust/target/uqm-package/x86_64-unknown-linux-gnu/uqm".to_string()
                    } else {
                        format!("{}.fixture", artifact.role)
                    },
                    "byte_length": if artifact.role == "executable" {
                        executable.len()
                    } else {
                        1
                    },
                    "sha256": if artifact.role == "executable" {
                        hex_sha256(executable)
                    } else {
                        "b".repeat(64)
                    }
                })
            })
            .collect();
        let package_manifest = serde_json::to_vec(&package_manifest_fixture(
            &authority,
            &index,
            package_artifacts,
        ))
        .unwrap();
        for (path, role, bytes) in [
            (
                "payloads/bootstrap-proof.package-manifest/production-artifacts.json",
                "bootstrap-proof.package-manifest",
                package_manifest.as_slice(),
            ),
            (
                "payloads/bootstrap-proof.executable/uqm",
                "bootstrap-proof.executable",
                executable.as_slice(),
            ),
            (
                "payloads/bootstrap-proof.profile/main-menu-v1.json",
                "bootstrap-proof.profile",
                profile.as_slice(),
            ),
        ] {
            write_bundle_entry(
                root,
                &mut index.entries,
                path,
                role,
                &package_command,
                bytes,
            );
            index.entries.last_mut().unwrap().producing_gate = "bootstrap-proof".into();
        }
        let empty_tree_hash = hex_sha256(b"");
        let tree = |root_role: &str| {
            serde_json::to_vec(&serde_json::json!({
                "schema": "uqm-tree-identity-v1",
                "root_role": root_role,
                "tree_sha256": empty_tree_hash,
                "entries": []
            }))
            .unwrap()
        };
        let trace_record = |sequence: u64, kind: &str| {
            serde_json::json!({
                "schema": 1,
                "run": 1,
                "sequence": sequence,
                "input_seen": 0,
                "present_seen": 0,
                "elapsed_ms": sequence,
                "kind": kind
            })
        };
        let mut trace_records = [
            trace_record(0, "run_start"),
            trace_record(1, "presentation"),
            trace_record(2, "semantic_assertion"),
            trace_record(3, "capture"),
            trace_record(4, "capture"),
            trace_record(5, "run_end"),
        ];
        for index in [1_usize, 3, 4] {
            trace_records[index]["present_seen"] = serde_json::json!(1);
            trace_records[index]["presentation"] = serde_json::json!({
                "count": 1,
                "generation": 1,
                "width": 640,
                "height": 480
            });
        }
        trace_records[2]["label"] = serde_json::json!("main_menu_visible");
        trace_records[3]["label"] = serde_json::json!("menu-after-down_gen1");
        trace_records[4]["label"] = serde_json::json!("menu-after-select_gen2");
        let trace = trace_records
            .iter()
            .map(serde_json::Value::to_string)
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        let teardown = serde_json::to_vec(&serde_json::json!({
            "schema": "uqm-teardown-v1",
            "terminal": "success",
            "game_status": 0,
            "process_status": 0,
            "runtime_finalized": true,
            "runtime_deactivated": true,
            "callbacks_quiescent": true,
            "trace_durable": true
        }))
        .unwrap();
        let png = |first_pixel: [u8; 3]| {
            let mut bytes = Vec::new();
            {
                let mut encoder = png::Encoder::new(&mut bytes, 2, 2);
                encoder.set_color(png::ColorType::Rgb);
                encoder.set_depth(png::BitDepth::Eight);
                let mut writer = encoder.write_header().unwrap();
                let pixels = [
                    first_pixel[0],
                    first_pixel[1],
                    first_pixel[2],
                    255,
                    0,
                    0,
                    0,
                    255,
                    0,
                    0,
                    0,
                    255,
                ];
                writer.write_image_data(&pixels).unwrap();
            }
            bytes
        };
        let artifact_specs = vec![
            (
                "capture",
                "run/captures/menu-after-down.png",
                png([0, 0, 0]),
            ),
            (
                "capture",
                "run/captures/menu-after-select.png",
                png([255, 255, 255]),
            ),
            (
                "teardown_receipt",
                "run/teardown-complete.json",
                teardown.clone(),
            ),
            ("trace", "run/trace.jsonl", trace.into_bytes()),
            (
                "final_config_snapshot",
                "snapshots/config-final.json",
                tree("final_config"),
            ),
            (
                "initial_config_snapshot",
                "snapshots/config-initial.json",
                tree("initial_config"),
            ),
            (
                "content_identity_snapshot",
                "snapshots/content-identity.json",
                tree("content"),
            ),
            (
                "production_manifest_snapshot",
                "snapshots/production-manifest.json",
                package_manifest.clone(),
            ),
            ("script_snapshot", "snapshots/script.json", profile.to_vec()),
            ("executable_snapshot", "snapshots/uqm", executable.to_vec()),
            ("stderr_log", "stderr.log", Vec::new()),
            ("stdout_log", "stdout.log", Vec::new()),
        ];
        let mut artifacts = Vec::new();
        for (role, path, bytes) in artifact_specs {
            artifacts.push(serde_json::json!({
                "role": role,
                "path": path,
                "sha256": hex_sha256(&bytes),
                "bytes": bytes.len()
            }));
            write_bundle_entry(
                root,
                &mut index.entries,
                &format!("payloads/bootstrap-proof.lcar-artifact/{path}"),
                "bootstrap-proof.lcar-artifact",
                &run_command,
                &bytes,
            );
            let entry = index.entries.last_mut().unwrap();
            entry.producing_gate = "bootstrap-proof".into();
            entry.mime = "application/octet-stream".into();
        }
        let package_hash = index
            .entries
            .iter()
            .find(|entry| entry.role == "bootstrap-proof.package-manifest")
            .unwrap()
            .sha256
            .clone();
        let executable_hash = index
            .entries
            .iter()
            .find(|entry| entry.role == "bootstrap-proof.executable")
            .unwrap()
            .sha256
            .clone();
        let profile_hash = index
            .entries
            .iter()
            .find(|entry| entry.role == "bootstrap-proof.profile")
            .unwrap()
            .sha256
            .clone();
        let lcar = serde_json::json!({
            "schema": "uqm-lcar-v1",
            "passed": true,
            "first_failed_contract": null,
            "git_head": index.source_sha,
            "command": [
                "/tmp/bootstrap-output/snapshots/uqm",
                "--contentdir=/tmp/repository/sc2/content",
                "--configdir=/tmp/bootstrap-output/config",
                "--automation-script=/tmp/bootstrap-output/snapshots/script.json",
                "--automation-output=/tmp/bootstrap-output/run",
                "--res=640x480",
                "--windowed",
                "--scroll=pc"
            ],
            "environment": {"SDL_AUDIODRIVER": "dummy", "SDL_VIDEODRIVER": "dummy"},
            "target": index.tuple,
            "profile": authority.package.profile,
            "features": authority.package.features,
            "renderer": "sdl2-software-dummy",
            "seed": 0x5eed_c0de_u64,
            "provenance": {
                "production_manifest_sha256": package_hash,
                "executable_sha256": executable_hash,
                "script_sha256": profile_hash,
                "content_tree_sha256": empty_tree_hash,
                "initial_config_tree_sha256": empty_tree_hash,
                "final_config_tree_sha256": empty_tree_hash
            },
            "process": {
                "pid": 1,
                "start_time": "fixture-start",
                "executable_sha256": executable_hash,
                "exit_code": 0,
                "signal": null,
                "term_sent": false,
                "kill_sent": false,
                "stdout_bytes": 0,
                "stderr_bytes": 0,
                "output_drained": true,
                "orphan_check_passed": true
            },
            "cleanup": {
                "exact_child_reaped": true,
                "orphan_check_passed": true,
                "output_drained": true,
                "config_root_removed": true
            },
            "artifacts": artifacts
        });
        write_bundle_entry(
            root,
            &mut index.entries,
            "payloads/bootstrap-proof.lcar/lcar-v1.json",
            "bootstrap-proof.lcar",
            &run_command,
            &serde_json::to_vec(&lcar).unwrap(),
        );
        index.entries.last_mut().unwrap().producing_gate = "bootstrap-proof".into();
        let build = vec![
            "cargo".into(),
            "build".into(),
            "--locked".into(),
            "--manifest-path".into(),
            "rust/Cargo.toml".into(),
            "--bin".into(),
            "uqm-gameplay-proof".into(),
        ];
        write_bundle_entry(
            root,
            &mut index.entries,
            "payloads/bootstrap-proof.runner/uqm-gameplay-proof",
            "bootstrap-proof.runner",
            &build,
            b"\x7fELF bootstrap runner",
        );
        index.entries.last_mut().unwrap().producing_gate = "bootstrap-proof".into();
        let validate_command = vec![
            run_command[0].clone(),
            "validate".into(),
            "/tmp/evidence/bootstrap-proof/lcar-v1.json".into(),
        ];
        for (step, command) in [
            ("build-runner", build.as_slice()),
            ("run", run_command.as_slice()),
            ("validate", validate_command.as_slice()),
        ] {
            write_builtin_step_fixture(
                root,
                &mut index.entries,
                "bootstrap-proof",
                step,
                command,
                (Some(0), None, None),
            );
        }
        (index, lcar, run_command, validate_command)
    }

    fn import_gate_payloads(
        root: &Path,
        source_root: &Path,
        source: &EvidenceIndex,
        gate: &str,
        entries: &mut Vec<EvidenceEntry>,
    ) {
        for entry in source.entries.iter().filter(|entry| {
            entry.producing_gate == gate
                && !matches!(
                    entry.role.as_str(),
                    "gate.result"
                        | "authority.snapshot"
                        | "preflight.source"
                        | "preflight.tools"
                        | "cache.initial-state"
                        | "ownership.zero-native-delta"
                )
        }) {
            let bytes = fs::read(source_root.join(&entry.path)).unwrap();
            let destination = root.join(&entry.path);
            fs::create_dir_all(destination.parent().unwrap()).unwrap();
            fs::write(destination, bytes).unwrap();
            entries.push(entry.clone());
        }
    }

    fn successful_all_gates_bundle(root: &Path, advisory_pack: &[u8]) -> EvidenceIndex {
        let authority: Authority =
            serde_json::from_slice(include_bytes!("../../../ci/gates.json")).unwrap();
        assert_eq!(
            hex_sha256(advisory_pack),
            authority.security.advisory_database_pack_sha256
        );
        let controller = vec!["uqm-xtask".into(), "ci".into(), "run".into(), "all".into()];
        let mut prefix = all_gates_security_post_failure_bundle(root);
        prefix.first_failed_contract = None;
        prefix
            .entries
            .retain(|entry| !(entry.role == "gate.result" && entry.producing_gate == "security"));
        write_bundle_entry(
            root,
            &mut prefix.entries,
            "payloads/security.advisory-database/advisory-database.pack",
            "security.advisory-database",
            &controller,
            advisory_pack,
        );
        prefix.entries.last_mut().unwrap().producing_gate = "security".into();
        write_gate_result_fixture(
            root,
            &mut prefix.entries,
            authority.gate("security").unwrap(),
            &controller,
            None,
        );

        let coverage_command = vec![
            "cargo".into(),
            "llvm-cov".into(),
            "--manifest-path".into(),
            "rust/Cargo.toml".into(),
            "--workspace".into(),
            "--all-targets".into(),
            "--no-default-features".into(),
            "--features".into(),
            authority.profiles.linked_test.join(","),
            "--lcov".into(),
            "--output-path".into(),
            root.join("coverage.lcov").display().to_string(),
            "--ignore-filename-regex".into(),
            authority.coverage.ignore_filename_regex.clone(),
        ];
        write_builtin_step_fixture(
            root,
            &mut prefix.entries,
            "coverage",
            "llvm-cov",
            &coverage_command,
            (Some(0), None, None),
        );
        write_bundle_entry(
            root,
            &mut prefix.entries,
            "payloads/coverage.lcov/coverage.lcov",
            "coverage.lcov",
            &coverage_command,
            b"LF:100\nLH:100\n",
        );
        prefix.entries.last_mut().unwrap().producing_gate = "coverage".into();
        write_gate_result_fixture(
            root,
            &mut prefix.entries,
            authority.gate("coverage").unwrap(),
            &controller,
            None,
        );

        let package_root = tempfile::tempdir().unwrap();
        let package = successful_package_bundle(package_root.path());
        import_gate_payloads(
            root,
            package_root.path(),
            &package,
            "package",
            &mut prefix.entries,
        );
        write_gate_result_fixture(
            root,
            &mut prefix.entries,
            authority.gate("package").unwrap(),
            &controller,
            None,
        );

        let bootstrap_root = tempfile::tempdir().unwrap();
        let (bootstrap, _, _, _) = successful_bootstrap_bundle(bootstrap_root.path());
        import_gate_payloads(
            root,
            bootstrap_root.path(),
            &bootstrap,
            "bootstrap-proof",
            &mut prefix.entries,
        );
        write_gate_result_fixture(
            root,
            &mut prefix.entries,
            authority.gate("bootstrap-proof").unwrap(),
            &controller,
            None,
        );

        let workflow_root = tempfile::tempdir().unwrap();
        let workflow = successful_workflow_bundle(workflow_root.path());
        import_gate_payloads(
            root,
            workflow_root.path(),
            &workflow,
            "workflow",
            &mut prefix.entries,
        );
        write_gate_result_fixture(
            root,
            &mut prefix.entries,
            authority.gate("workflow").unwrap(),
            &controller,
            None,
        );

        let mutations_root = tempfile::tempdir().unwrap();
        let mutations = successful_mutations_bundle(mutations_root.path());
        import_gate_payloads(
            root,
            mutations_root.path(),
            &mutations,
            "mutations",
            &mut prefix.entries,
        );
        write_gate_result_fixture(
            root,
            &mut prefix.entries,
            authority.gate("mutations").unwrap(),
            &controller,
            None,
        );

        EvidenceIndex::build_and_validate(
            root,
            &fixture_tuples(),
            EvidenceContext {
                source_sha: prefix.source_sha,
                clean: true,
                tuple: prefix.tuple,
                features: prefix.features,
                cache_mode: prefix.cache_mode,
                first_failed_contract: None,
            },
            prefix.entries,
        )
        .unwrap()
    }

    /// Synthetic validator-contract fixture; this is not hosted or production evidence.
    #[test]
    #[ignore = "requires UQM_TEST_ADVISORY_DATABASE_PACK with the pinned RustSec pack"]
    fn successful_all_controller_fixture_replays_offline() {
        let pack_path = std::env::var_os("UQM_TEST_ADVISORY_DATABASE_PACK")
            .map(PathBuf::from)
            .expect("UQM_TEST_ADVISORY_DATABASE_PACK must name the pinned pack");
        let advisory_pack = fs::read(pack_path).unwrap();
        let bundle = tempfile::tempdir().unwrap();
        let index = successful_all_gates_bundle(bundle.path(), &advisory_pack);
        let index_path = bundle.path().join("evidence-index.json");
        fs::write(&index_path, serde_json::to_vec_pretty(&index).unwrap()).unwrap();
        validate_evidence_command(
            Path::new("/definitely-not-a-repository"),
            index_path.to_str().unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn bootstrap_lcar_replay_requires_a_complete_embedded_inventory() {
        let root = std::env::temp_dir().join(format!(
            "uqm-evidence-lcar-inventory-{}",
            std::process::id()
        ));
        let authority: Authority =
            serde_json::from_slice(include_bytes!("../../../ci/gates.json")).unwrap();
        let (mut index, lcar, run_command, validate_command) = successful_bootstrap_bundle(&root);
        let teardown = fs::read(
            root.join("payloads/bootstrap-proof.lcar-artifact/run/teardown-complete.json"),
        )
        .unwrap();
        let executable = b"\x7fELF bootstrap executable";
        let profile = include_bytes!("../../../scripts/main-menu-v1.json");
        let trace_records: Vec<serde_json::Value> =
            fs::read_to_string(root.join("payloads/bootstrap-proof.lcar-artifact/run/trace.jsonl"))
                .unwrap()
                .lines()
                .map(|line| serde_json::from_str(line).unwrap())
                .collect();
        let png = || {
            let mut bytes = Vec::new();
            {
                let mut encoder = png::Encoder::new(&mut bytes, 2, 2);
                encoder.set_color(png::ColorType::Rgb);
                encoder.set_depth(png::BitDepth::Eight);
                let mut writer = encoder.write_header().unwrap();
                writer
                    .write_image_data(&[0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255])
                    .unwrap();
            }
            bytes
        };
        assert_eq!(
            validate_successful_builtin_gate(&root, &index, &authority, "bootstrap-proof"),
            Vec::<String>::new()
        );
        let mut failed_validate = index.clone();
        rewrite_builtin_step_result(
            &root,
            &mut failed_validate.entries,
            "bootstrap-proof",
            "validate",
            &validate_command,
            (Some(2), None, None),
        );
        assert_eq!(
            validate_failed_builtin_gate(
                &root,
                &failed_validate,
                &authority,
                "bootstrap-proof",
                "bootstrap-proof.validate",
            ),
            Vec::<String>::new()
        );

        let mut contracts = Vec::new();
        validate_bootstrap_lcar(&root, &index, &authority, &run_command, &mut contracts);
        assert_eq!(contracts, Vec::<String>::new());

        let mut failed_run = index.clone();
        failed_run.entries.retain(|entry| {
            entry.role != "bootstrap-proof.lcar"
                && !entry.path.starts_with("bootstrap-proof/validate.")
        });
        rewrite_builtin_step_result(
            &root,
            &mut failed_run.entries,
            "bootstrap-proof",
            "run",
            &run_command,
            (Some(1), None, None),
        );
        let run_stderr =
            b"proof failed; retained /tmp/evidence/bootstrap-proof/failure-lcar-v1.json\n";
        rewrite_bundle_path(
            &root,
            &mut failed_run.entries,
            "bootstrap-proof/run.stderr.log",
            run_stderr,
        );
        let mut failure_lcar = lcar.clone();
        failure_lcar["passed"] = serde_json::json!(false);
        failure_lcar["first_failed_contract"] = serde_json::json!("nonzero_child");
        failure_lcar["process"]["exit_code"] = serde_json::json!(1);
        write_bundle_entry(
            &root,
            &mut failed_run.entries,
            "payloads/bootstrap-proof.failure-lcar/failure-lcar-v1.json",
            "bootstrap-proof.failure-lcar",
            &run_command,
            &serde_json::to_vec(&failure_lcar).unwrap(),
        );
        failed_run.entries.last_mut().unwrap().producing_gate = "bootstrap-proof".into();
        assert_eq!(
            validate_failed_builtin_gate(
                &root,
                &failed_run,
                &authority,
                "bootstrap-proof",
                "bootstrap-proof.run",
            ),
            Vec::<String>::new()
        );

        let mut missing_failure_lcar = failed_run.clone();
        missing_failure_lcar
            .entries
            .retain(|entry| entry.role != "bootstrap-proof.failure-lcar");
        assert!(validate_failed_builtin_gate(
            &root,
            &missing_failure_lcar,
            &authority,
            "bootstrap-proof",
            "bootstrap-proof.run",
        )
        .iter()
        .any(|contract| contract == "evidence.builtin.bootstrap-proof.failure_lcar.presence"));
        let mut forged_failure = failure_lcar.clone();
        forged_failure["first_failed_contract"] = serde_json::json!("invented");
        rewrite_bundle_entry(
            &root,
            &mut failed_run.entries,
            "bootstrap-proof.failure-lcar",
            &serde_json::to_vec(&forged_failure).unwrap(),
        );
        assert!(validate_failed_builtin_gate(
            &root,
            &failed_run,
            &authority,
            "bootstrap-proof",
            "bootstrap-proof.run",
        )
        .iter()
        .any(|contract| contract == "evidence.builtin.bootstrap-proof.failure_lcar.identity"));
        rewrite_bundle_entry(
            &root,
            &mut failed_run.entries,
            "bootstrap-proof.failure-lcar",
            &serde_json::to_vec(&failure_lcar).unwrap(),
        );

        for (exit_code, signal) in [
            (serde_json::json!(1), serde_json::json!(9)),
            (serde_json::json!(256), serde_json::Value::Null),
            (serde_json::Value::Null, serde_json::json!(0)),
            (
                serde_json::Value::Null,
                serde_json::json!(i64::from(i32::MAX) + 1),
            ),
        ] {
            let mut forged = failure_lcar.clone();
            forged["process"]["exit_code"] = exit_code;
            forged["process"]["signal"] = signal;
            rewrite_bundle_entry(
                &root,
                &mut failed_run.entries,
                "bootstrap-proof.failure-lcar",
                &serde_json::to_vec(&forged).unwrap(),
            );
            assert!(validate_failed_builtin_gate(
                &root,
                &failed_run,
                &authority,
                "bootstrap-proof",
                "bootstrap-proof.run",
            )
            .iter()
            .any(|contract| contract == "evidence.builtin.bootstrap-proof.failure_lcar.result"));
        }

        for (field, value) in [
            ("process", serde_json::json!({})),
            ("cleanup", serde_json::json!({})),
        ] {
            let mut forged = failure_lcar.clone();
            forged[field] = value;
            rewrite_bundle_entry(
                &root,
                &mut failed_run.entries,
                "bootstrap-proof.failure-lcar",
                &serde_json::to_vec(&forged).unwrap(),
            );
            assert!(validate_failed_builtin_gate(
                &root,
                &failed_run,
                &authority,
                "bootstrap-proof",
                "bootstrap-proof.run",
            )
            .iter()
            .any(|contract| contract == "evidence.builtin.bootstrap-proof.failure_lcar.result"));
        }
        for (contract, process_field) in [
            ("timeout", Some("term_sent")),
            ("reader", None),
            ("budget", None),
        ] {
            let mut typed = failure_lcar.clone();
            typed["first_failed_contract"] = serde_json::json!(contract);
            if let Some(field) = process_field {
                typed["process"][field] = serde_json::json!(true);
            }
            rewrite_bundle_entry(
                &root,
                &mut failed_run.entries,
                "bootstrap-proof.failure-lcar",
                &serde_json::to_vec(&typed).unwrap(),
            );
            assert_eq!(
                validate_failed_builtin_gate(
                    &root,
                    &failed_run,
                    &authority,
                    "bootstrap-proof",
                    "bootstrap-proof.run",
                ),
                Vec::<String>::new(),
                "{contract} failure LCAR should replay",
            );
        }

        let failed_teardown_receipt = serde_json::to_vec(&serde_json::json!({
            "schema": "uqm-teardown-v1",
            "terminal": "semantic_mismatch",
            "game_status": 1,
            "process_status": 1,
            "runtime_finalized": true,
            "runtime_deactivated": true,
            "callbacks_quiescent": true,
            "trace_durable": true
        }))
        .unwrap();
        let mut semantic_lcar = failure_lcar.clone();
        semantic_lcar["first_failed_contract"] = serde_json::json!("semantic_evidence");
        rewrite_lcar_artifact(
            &root,
            &mut failed_run.entries,
            &mut semantic_lcar,
            "run/teardown-complete.json",
            &failed_teardown_receipt,
        );
        rewrite_bundle_entry(
            &root,
            &mut failed_run.entries,
            "bootstrap-proof.failure-lcar",
            &serde_json::to_vec(&semantic_lcar).unwrap(),
        );
        assert_eq!(
            validate_failed_builtin_gate(
                &root,
                &failed_run,
                &authority,
                "bootstrap-proof",
                "bootstrap-proof.run",
            ),
            Vec::<String>::new(),
        );

        let mut teardown_failure_lcar = failure_lcar.clone();
        teardown_failure_lcar["first_failed_contract"] = serde_json::json!("teardown_evidence");
        rewrite_lcar_artifact(
            &root,
            &mut failed_run.entries,
            &mut teardown_failure_lcar,
            "run/teardown-complete.json",
            b"{}",
        );
        rewrite_bundle_entry(
            &root,
            &mut failed_run.entries,
            "bootstrap-proof.failure-lcar",
            &serde_json::to_vec(&teardown_failure_lcar).unwrap(),
        );
        assert_eq!(
            validate_failed_builtin_gate(
                &root,
                &failed_run,
                &authority,
                "bootstrap-proof",
                "bootstrap-proof.run",
            ),
            Vec::<String>::new(),
        );

        let mut restored_failure_lcar = failure_lcar.clone();
        rewrite_lcar_artifact(
            &root,
            &mut failed_run.entries,
            &mut restored_failure_lcar,
            "run/teardown-complete.json",
            &teardown,
        );
        rewrite_bundle_entry(
            &root,
            &mut failed_run.entries,
            "bootstrap-proof.failure-lcar",
            &serde_json::to_vec(&restored_failure_lcar).unwrap(),
        );
        let mut missing_teardown_index = failed_run.clone();
        let mut missing_teardown_lcar = failure_lcar.clone();
        missing_teardown_lcar["first_failed_contract"] = serde_json::json!("missing_teardown");
        missing_teardown_lcar["artifacts"]
            .as_array_mut()
            .unwrap()
            .retain(|artifact| {
                artifact.get("role").and_then(|value| value.as_str()) != Some("teardown_receipt")
            });
        missing_teardown_index.entries.retain(|entry| {
            entry.path != "payloads/bootstrap-proof.lcar-artifact/run/teardown-complete.json"
        });
        rewrite_bundle_entry(
            &root,
            &mut missing_teardown_index.entries,
            "bootstrap-proof.failure-lcar",
            &serde_json::to_vec(&missing_teardown_lcar).unwrap(),
        );
        assert_eq!(
            validate_failed_builtin_gate(
                &root,
                &missing_teardown_index,
                &authority,
                "bootstrap-proof",
                "bootstrap-proof.run",
            ),
            Vec::<String>::new(),
        );
        let mut forged_teardown_index = missing_teardown_index.clone();
        missing_teardown_lcar["first_failed_contract"] = serde_json::json!("teardown_evidence");
        rewrite_bundle_entry(
            &root,
            &mut forged_teardown_index.entries,
            "bootstrap-proof.failure-lcar",
            &serde_json::to_vec(&missing_teardown_lcar).unwrap(),
        );
        assert!(validate_failed_builtin_gate(
            &root,
            &forged_teardown_index,
            &authority,
            "bootstrap-proof",
            "bootstrap-proof.run",
        )
        .iter()
        .any(|contract| contract == "evidence.builtin.bootstrap-proof.failure_lcar.result"));
        let config_bytes = b"retained cleanup state\n";
        let config_hash = hex_sha256(config_bytes);
        let mut config_tree_hasher = Sha256::new();
        config_tree_hasher.update(b"settings.cfg");
        config_tree_hasher.update([0]);
        config_tree_hasher.update(config_hash.as_bytes());
        config_tree_hasher.update([0]);
        config_tree_hasher.update(config_bytes.len().to_string().as_bytes());
        config_tree_hasher.update(b"\n");
        let config_tree_hash = config_tree_hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let config_snapshot = serde_json::to_vec(&serde_json::json!({
            "schema": "uqm-tree-identity-v1",
            "root_role": "final_config",
            "tree_sha256": config_tree_hash,
            "entries": [{
                "path": "settings.cfg",
                "sha256": config_hash,
                "bytes": config_bytes.len()
            }]
        }))
        .unwrap();
        let mut config_cleanup_index = failed_run.clone();
        let mut config_cleanup_lcar = failure_lcar.clone();
        config_cleanup_lcar["first_failed_contract"] = serde_json::json!("config_cleanup");
        config_cleanup_lcar["cleanup"]["config_root_removed"] = serde_json::json!(false);
        config_cleanup_lcar["provenance"]["final_config_tree_sha256"] =
            serde_json::json!(config_tree_hash);
        rewrite_lcar_artifact(
            &root,
            &mut config_cleanup_index.entries,
            &mut config_cleanup_lcar,
            "snapshots/config-final.json",
            &config_snapshot,
        );
        config_cleanup_lcar["artifacts"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({
                "role": "retained_config_file",
                "path": "config/settings.cfg",
                "sha256": config_hash,
                "bytes": config_bytes.len()
            }));
        config_cleanup_lcar["artifacts"]
            .as_array_mut()
            .unwrap()
            .sort_by(|left, right| {
                left.get("path")
                    .and_then(|value| value.as_str())
                    .cmp(&right.get("path").and_then(|value| value.as_str()))
            });
        write_bundle_entry(
            &root,
            &mut config_cleanup_index.entries,
            "payloads/bootstrap-proof.lcar-artifact/config/settings.cfg",
            "bootstrap-proof.lcar-artifact",
            &run_command,
            config_bytes,
        );
        let retained_config_entry = config_cleanup_index.entries.last_mut().unwrap();
        retained_config_entry.producing_gate = "bootstrap-proof".into();
        retained_config_entry.mime = "application/octet-stream".into();
        rewrite_bundle_entry(
            &root,
            &mut config_cleanup_index.entries,
            "bootstrap-proof.failure-lcar",
            &serde_json::to_vec(&config_cleanup_lcar).unwrap(),
        );
        assert_eq!(
            validate_failed_builtin_gate(
                &root,
                &config_cleanup_index,
                &authority,
                "bootstrap-proof",
                "bootstrap-proof.run",
            ),
            Vec::<String>::new(),
        );
        let mut mismatched_config_index = config_cleanup_index.clone();
        let mut mismatched_config_lcar = config_cleanup_lcar.clone();
        rewrite_lcar_artifact(
            &root,
            &mut mismatched_config_index.entries,
            &mut mismatched_config_lcar,
            "config/settings.cfg",
            b"different retained cleanup state\n",
        );
        rewrite_bundle_entry(
            &root,
            &mut mismatched_config_index.entries,
            "bootstrap-proof.failure-lcar",
            &serde_json::to_vec(&mismatched_config_lcar).unwrap(),
        );
        assert!(validate_failed_builtin_gate(
            &root,
            &mismatched_config_index,
            &authority,
            "bootstrap-proof",
            "bootstrap-proof.run",
        )
        .iter()
        .any(|contract| contract == "evidence.builtin.bootstrap-proof.failure_lcar.snapshots"));
        let mut forged_inventory = failure_lcar.clone();
        forged_inventory["artifacts"].as_array_mut().unwrap().pop();
        rewrite_bundle_entry(
            &root,
            &mut failed_run.entries,
            "bootstrap-proof.failure-lcar",
            &serde_json::to_vec(&forged_inventory).unwrap(),
        );
        let forged_inventory_contracts = validate_failed_builtin_gate(
            &root,
            &failed_run,
            &authority,
            "bootstrap-proof",
            "bootstrap-proof.run",
        );
        assert!(
            forged_inventory_contracts
                .iter()
                .any(|contract| contract
                    == "evidence.builtin.bootstrap-proof.failure_lcar.inventory"),
            "{forged_inventory_contracts:?}"
        );
        rewrite_bundle_entry(
            &root,
            &mut failed_run.entries,
            "bootstrap-proof.failure-lcar",
            &serde_json::to_vec(&failure_lcar).unwrap(),
        );
        let snapshot_path = "payloads/bootstrap-proof.lcar-artifact/snapshots/uqm";
        let snapshot_entry = failed_run
            .entries
            .iter_mut()
            .find(|entry| entry.path == snapshot_path)
            .unwrap();
        fs::write(root.join(snapshot_path), b"forged snapshot").unwrap();
        snapshot_entry.byte_length = b"forged snapshot".len() as u64;
        snapshot_entry.sha256 = hex_sha256(b"forged snapshot");
        assert!(validate_failed_builtin_gate(
            &root,
            &failed_run,
            &authority,
            "bootstrap-proof",
            "bootstrap-proof.run",
        )
        .iter()
        .any(|contract| contract == "evidence.builtin.bootstrap-proof.failure_lcar.inventory"));
        fs::write(root.join(snapshot_path), executable).unwrap();

        rewrite_bundle_entry(
            &root,
            &mut index.entries,
            "bootstrap-proof.executable",
            b"forged executable",
        );
        let contracts =
            validate_successful_builtin_gate(&root, &index, &authority, "bootstrap-proof");
        assert!(contracts
            .iter()
            .any(|contract| { contract == "evidence.builtin.bootstrap-proof.executable_content" }));
        rewrite_bundle_entry(
            &root,
            &mut index.entries,
            "bootstrap-proof.executable",
            executable,
        );

        let package_manifest_entry = index
            .entries
            .iter()
            .find(|entry| entry.role == "bootstrap-proof.package-manifest")
            .unwrap();
        let original_package_manifest = fs::read(root.join(&package_manifest_entry.path)).unwrap();
        let mut forged_package_manifest: serde_json::Value =
            serde_json::from_slice(&original_package_manifest).unwrap();
        let executable_artifact = forged_package_manifest["artifacts"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|artifact| artifact["role"] == "executable")
            .unwrap();
        executable_artifact["path"] = serde_json::json!("rust/target/release/uqm");
        rewrite_bundle_entry(
            &root,
            &mut index.entries,
            "bootstrap-proof.package-manifest",
            &serde_json::to_vec(&forged_package_manifest).unwrap(),
        );
        let contracts =
            validate_successful_builtin_gate(&root, &index, &authority, "bootstrap-proof");
        assert!(contracts
            .iter()
            .any(|contract| contract == "evidence.builtin.bootstrap-proof.package_content"));
        rewrite_bundle_entry(
            &root,
            &mut index.entries,
            "bootstrap-proof.package-manifest",
            &original_package_manifest,
        );

        rewrite_bundle_entry(&root, &mut index.entries, "bootstrap-proof.profile", b"{}");
        let contracts =
            validate_successful_builtin_gate(&root, &index, &authority, "bootstrap-proof");
        assert!(contracts
            .iter()
            .any(|contract| contract == "evidence.builtin.bootstrap-proof.profile_content"));
        rewrite_bundle_entry(
            &root,
            &mut index.entries,
            "bootstrap-proof.profile",
            profile,
        );

        rewrite_bundle_entry(
            &root,
            &mut index.entries,
            "bootstrap-proof.runner",
            b"not an executable",
        );
        let contracts =
            validate_successful_builtin_gate(&root, &index, &authority, "bootstrap-proof");
        assert!(contracts
            .iter()
            .any(|contract| contract == "evidence.builtin.bootstrap-proof.runner_content"));
        rewrite_bundle_entry(
            &root,
            &mut index.entries,
            "bootstrap-proof.runner",
            b"\x7fELF bootstrap runner",
        );

        let original_lcar = serde_json::to_vec(&lcar).unwrap();
        let mut forged_command = lcar.clone();
        forged_command["command"][5] = serde_json::json!("--res=320x240");
        rewrite_bundle_entry(
            &root,
            &mut index.entries,
            "bootstrap-proof.lcar",
            &serde_json::to_vec(&forged_command).unwrap(),
        );
        let mut contracts = Vec::new();
        validate_bootstrap_lcar(&root, &index, &authority, &run_command, &mut contracts);
        assert!(contracts
            .iter()
            .any(|contract| contract == "evidence.builtin.bootstrap-proof.lcar.command"));

        let mut forged_process = lcar.clone();
        forged_process["process"]["stdout_bytes"] = serde_json::json!(1);
        rewrite_bundle_entry(
            &root,
            &mut index.entries,
            "bootstrap-proof.lcar",
            &serde_json::to_vec(&forged_process).unwrap(),
        );
        let mut contracts = Vec::new();
        validate_bootstrap_lcar(&root, &index, &authority, &run_command, &mut contracts);
        assert!(contracts.iter().any(|contract| {
            contract == "evidence.builtin.bootstrap-proof.lcar.process_receipt"
        }));
        rewrite_bundle_entry(
            &root,
            &mut index.entries,
            "bootstrap-proof.lcar",
            &original_lcar,
        );

        let mut forged_identity = lcar.clone();
        forged_identity["unexpected"] = serde_json::json!(true);
        rewrite_bundle_entry(
            &root,
            &mut index.entries,
            "bootstrap-proof.lcar",
            &serde_json::to_vec(&forged_identity).unwrap(),
        );
        let mut contracts = Vec::new();
        validate_bootstrap_lcar(&root, &index, &authority, &run_command, &mut contracts);
        assert!(contracts
            .iter()
            .any(|contract| contract == "evidence.builtin.bootstrap-proof.lcar.identity"));

        let mut forged_cleanup = lcar.clone();
        forged_cleanup["cleanup"]["config_root_removed"] = serde_json::json!(false);
        rewrite_bundle_entry(
            &root,
            &mut index.entries,
            "bootstrap-proof.lcar",
            &serde_json::to_vec(&forged_cleanup).unwrap(),
        );
        let mut contracts = Vec::new();
        validate_bootstrap_lcar(&root, &index, &authority, &run_command, &mut contracts);
        assert!(contracts
            .iter()
            .any(|contract| contract == "evidence.builtin.bootstrap-proof.lcar.result"));

        let mut forged_order = lcar.clone();
        forged_order["artifacts"].as_array_mut().unwrap().swap(0, 1);
        rewrite_bundle_entry(
            &root,
            &mut index.entries,
            "bootstrap-proof.lcar",
            &serde_json::to_vec(&forged_order).unwrap(),
        );
        let mut contracts = Vec::new();
        validate_bootstrap_lcar(&root, &index, &authority, &run_command, &mut contracts);
        assert!(contracts
            .iter()
            .any(|contract| contract == "evidence.builtin.bootstrap-proof.lcar.inventory"));

        let original_trace = trace_records
            .iter()
            .map(serde_json::Value::to_string)
            .collect::<Vec<_>>()
            .join(
                "
",
            )
            + "
";
        let mut forged_trace = lcar.clone();
        rewrite_lcar_artifact(
            &root,
            &mut index.entries,
            &mut forged_trace,
            "run/trace.jsonl",
            b"{}
",
        );
        rewrite_bundle_entry(
            &root,
            &mut index.entries,
            "bootstrap-proof.lcar",
            &serde_json::to_vec(&forged_trace).unwrap(),
        );
        let mut contracts = Vec::new();
        validate_bootstrap_lcar(&root, &index, &authority, &run_command, &mut contracts);
        assert!(contracts
            .iter()
            .any(|contract| contract == "evidence.builtin.bootstrap-proof.lcar.trace"));
        let mut restored_lcar = lcar.clone();
        rewrite_lcar_artifact(
            &root,
            &mut index.entries,
            &mut restored_lcar,
            "run/trace.jsonl",
            original_trace.as_bytes(),
        );
        rewrite_bundle_entry(
            &root,
            &mut index.entries,
            "bootstrap-proof.lcar",
            &serde_json::to_vec(&restored_lcar).unwrap(),
        );

        let mut forged_teardown = lcar.clone();
        rewrite_lcar_artifact(
            &root,
            &mut index.entries,
            &mut forged_teardown,
            "run/teardown-complete.json",
            br#"{"schema":"uqm-teardown-v1","terminal":"fatal"}"#,
        );
        rewrite_bundle_entry(
            &root,
            &mut index.entries,
            "bootstrap-proof.lcar",
            &serde_json::to_vec(&forged_teardown).unwrap(),
        );
        let mut contracts = Vec::new();
        validate_bootstrap_lcar(&root, &index, &authority, &run_command, &mut contracts);
        assert!(contracts
            .iter()
            .any(|contract| contract == "evidence.builtin.bootstrap-proof.lcar.teardown"));
        let mut restored_teardown = lcar.clone();
        rewrite_lcar_artifact(
            &root,
            &mut index.entries,
            &mut restored_teardown,
            "run/teardown-complete.json",
            &teardown,
        );
        rewrite_bundle_entry(
            &root,
            &mut index.entries,
            "bootstrap-proof.lcar",
            &serde_json::to_vec(&restored_teardown).unwrap(),
        );

        let mut forged_capture = lcar.clone();
        rewrite_lcar_artifact(
            &root,
            &mut index.entries,
            &mut forged_capture,
            "run/captures/menu-after-down.png",
            &png()[..24],
        );
        rewrite_bundle_entry(
            &root,
            &mut index.entries,
            "bootstrap-proof.lcar",
            &serde_json::to_vec(&forged_capture).unwrap(),
        );
        let mut contracts = Vec::new();
        validate_bootstrap_lcar(&root, &index, &authority, &run_command, &mut contracts);
        assert!(contracts
            .iter()
            .any(|contract| contract == "evidence.builtin.bootstrap-proof.lcar.inventory"));
        let mut restored_capture = lcar.clone();
        rewrite_lcar_artifact(
            &root,
            &mut index.entries,
            &mut restored_capture,
            "run/captures/menu-after-down.png",
            &png(),
        );
        rewrite_bundle_entry(
            &root,
            &mut index.entries,
            "bootstrap-proof.lcar",
            &serde_json::to_vec(&restored_capture).unwrap(),
        );

        let mut forged_tree = lcar.clone();

        rewrite_lcar_artifact(
            &root,
            &mut index.entries,
            &mut forged_tree,
            "snapshots/content-identity.json",
            b"{}",
        );
        rewrite_bundle_entry(
            &root,
            &mut index.entries,
            "bootstrap-proof.lcar",
            &serde_json::to_vec(&forged_tree).unwrap(),
        );
        let mut contracts = Vec::new();
        validate_bootstrap_lcar(&root, &index, &authority, &run_command, &mut contracts);
        assert!(contracts
            .iter()
            .any(|contract| contract == "evidence.builtin.bootstrap-proof.lcar.snapshots"));

        rewrite_bundle_entry(
            &root,
            &mut index.entries,
            "bootstrap-proof.lcar-artifact",
            b"forged",
        );
        let mut contracts = Vec::new();
        validate_bootstrap_lcar(&root, &index, &authority, &run_command, &mut contracts);
        assert!(contracts
            .iter()
            .any(|contract| contract == "evidence.builtin.bootstrap-proof.lcar.inventory"));
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn publisher_files_are_readable_by_the_containment_group_but_not_writable() {
        use std::os::unix::fs::PermissionsExt as _;

        let temp = tempfile::tempdir().unwrap();
        let publisher = EvidencePublisher::open(temp.path()).unwrap();
        publisher.create("receipt.json", b"{}\n").unwrap();

        assert_eq!(
            fs::metadata(temp.path().join("receipt.json"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o640
        );
    }

    #[cfg(unix)]
    #[test]
    fn publisher_create_never_replaces_a_destination_created_at_commit() {
        let temp = tempfile::tempdir().unwrap();
        let publisher = EvidencePublisher::open(temp.path()).unwrap();
        let destination = temp.path().join("contested.log");
        let injected_destination = destination.clone();
        ADVERSARIAL_HOOK.with(|slot| {
            *slot.borrow_mut() = Some(Box::new(move |event| {
                if event == "publish-before-commit:contested.log" {
                    fs::write(&injected_destination, b"incumbent").unwrap();
                }
            }));
        });

        let error = publisher.create("contested.log", b"candidate").unwrap_err();
        ADVERSARIAL_HOOK.with(|slot| *slot.borrow_mut() = None);

        assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read(destination).unwrap(), b"incumbent");
    }

    #[test]
    fn tree_snapshot_rejects_a_coherently_rehashed_noncanonical_order() {
        let root =
            std::env::temp_dir().join(format!("uqm-evidence-tree-order-{}", std::process::id()));
        let snapshot_path =
            root.join("payloads/bootstrap-proof.lcar-artifact/snapshots/config-final.json");
        fs::create_dir_all(snapshot_path.parent().unwrap()).unwrap();
        let hash_a = "a".repeat(64);
        let hash_b = "b".repeat(64);
        let digest = |entries: &[(&str, &str, u64)]| {
            let mut hasher = Sha256::new();
            for (path, hash, bytes) in entries {
                hasher.update(path.as_bytes());
                hasher.update([0]);
                hasher.update(hash.as_bytes());
                hasher.update([0]);
                hasher.update(bytes.to_string().as_bytes());
                hasher.update(b"\n");
            }
            format!("{:x}", hasher.finalize())
        };
        let sorted = [("a.cfg", hash_a.as_str(), 1), ("b.cfg", hash_b.as_str(), 2)];
        let sorted_digest = digest(&sorted);
        fs::write(
            &snapshot_path,
            serde_json::to_vec(&serde_json::json!({
                "schema": "uqm-tree-identity-v1",
                "root_role": "final_config",
                "tree_sha256": sorted_digest,
                "entries": sorted.iter().map(|(path, hash, bytes)| serde_json::json!({
                    "path": path,
                    "sha256": hash,
                    "bytes": bytes
                })).collect::<Vec<_>>()
            }))
            .unwrap(),
        )
        .unwrap();
        assert!(validate_lcar_tree_snapshot(
            &root,
            "snapshots/config-final.json",
            "final_config",
            &sorted_digest,
        ));

        let reversed = [("b.cfg", hash_b.as_str(), 2), ("a.cfg", hash_a.as_str(), 1)];
        let reversed_digest = digest(&reversed);
        fs::write(
            &snapshot_path,
            serde_json::to_vec(&serde_json::json!({
                "schema": "uqm-tree-identity-v1",
                "root_role": "final_config",
                "tree_sha256": reversed_digest,
                "entries": reversed.iter().map(|(path, hash, bytes)| serde_json::json!({
                    "path": path,
                    "sha256": hash,
                    "bytes": bytes
                })).collect::<Vec<_>>()
            }))
            .unwrap(),
        )
        .unwrap();
        assert!(!validate_lcar_tree_snapshot(
            &root,
            "snapshots/config-final.json",
            "final_config",
            &reversed_digest,
        ));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn bootstrap_capture_change_contract_rejects_a_frozen_expected_transition() {
        let root = std::env::temp_dir().join(format!(
            "uqm-evidence-capture-change-{}",
            std::process::id()
        ));
        let payload = root.join("payloads/bootstrap-proof.lcar-artifact");
        fs::create_dir_all(payload.join("snapshots")).unwrap();
        fs::create_dir_all(payload.join("run/captures")).unwrap();
        fs::write(
            payload.join("snapshots/script.json"),
            serde_json::to_vec(&serde_json::json!({
                "steps": [
                    {"action": "capture", "label": "before"},
                    {"action": "capture", "label": "after", "expect_change": true}
                ]
            }))
            .unwrap(),
        )
        .unwrap();
        fs::write(payload.join("run/captures/before.png"), b"same").unwrap();
        fs::write(payload.join("run/captures/after.png"), b"same").unwrap();
        let captures = vec![
            "run/captures/before.png".to_string(),
            "run/captures/after.png".to_string(),
        ];
        assert!(!validate_lcar_capture_changes(
            &root,
            Some("snapshots/script.json"),
            &captures,
        ));
        fs::write(payload.join("run/captures/after.png"), b"changed").unwrap();
        assert!(validate_lcar_capture_changes(
            &root,
            Some("snapshots/script.json"),
            &captures,
        ));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn builtin_step_accepts_authorized_nonzero_exit_without_forging_success() {
        let root =
            std::env::temp_dir().join(format!("uqm-evidence-builtin-exit-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let command: Vec<String> = vec!["cleanup-helper".into(), "--exact-child".into()];
        let mut index = valid_index();
        index.entries.clear();
        for (path, role, bytes) in [
            (
                "bootstrap-proof/cleanup-helper.stdout.log",
                "step.stdout",
                Vec::new(),
            ),
            (
                "bootstrap-proof/cleanup-helper.stderr.log",
                "step.stderr",
                Vec::new(),
            ),
            (
                "bootstrap-proof/cleanup-helper.result.json",
                "step.result",
                serde_json::to_vec(&serde_json::json!({
                    "schema": "uqm-s4-step-result-v2",
                    "gate": "bootstrap-proof",
                    "step": "cleanup-helper",
                    "command": command,
                    "effective_command": command,
                    "staged_script_sha256": null,
                    "executable_identity": fixture_executable_identity(None),
                    "success": false,
                    "exit_code": 1,
                    "signal": null,
                    "launch_error": null,
                    "supervision": fixture_supervision(false, 0, 0),
                }))
                .unwrap(),
            ),
        ] {
            write_bundle_entry(&root, &mut index.entries, path, role, &command, &bytes);
            index.entries.last_mut().unwrap().producing_gate = "bootstrap-proof".into();
        }
        assert_eq!(
            validate_builtin_step(
                &root,
                &index,
                "bootstrap-proof",
                "cleanup-helper",
                &[0, 1],
                |actual| actual == command
            ),
            Vec::<String>::new()
        );
        assert!(validate_builtin_step(
            &root,
            &index,
            "bootstrap-proof",
            "cleanup-helper",
            &[0],
            |actual| actual == command
        )
        .iter()
        .any(|contract| contract.ends_with(".result")));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn setup_failures_require_truthful_cache_and_delta_receipts() {
        for contract in ["cache.rust_target", "ownership.zero_native_delta"] {
            let bundle = tempfile::tempdir().unwrap();
            let index = setup_failure_bundle(bundle.path(), contract);
            assert!(index.offline_validation.passed, "{contract}: {index:#?}");
            assert!(!index
                .entries
                .iter()
                .any(|entry| entry.role == "gate.result"));
        }

        let bundle = tempfile::tempdir().unwrap();
        let mut forged = setup_failure_bundle(bundle.path(), "cache.rust_target");
        let cache_entry = forged
            .entries
            .iter_mut()
            .find(|entry| entry.role == "cache.initial-state")
            .unwrap();
        let path = bundle.path().join(&cache_entry.path);
        let mut cache: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        cache["restore_used"] = serde_json::json!(true);
        let bytes = serde_json::to_vec(&cache).unwrap();
        fs::write(&path, &bytes).unwrap();
        cache_entry.byte_length = bytes.len() as u64;
        cache_entry.sha256 = hex_sha256(&bytes);
        assert!(production_contracts(bundle.path(), &forged)
            .iter()
            .any(|contract| contract == "evidence.cache.initial_state.cache_action"));

        let bundle = tempfile::tempdir().unwrap();
        let mut forged = setup_failure_bundle(bundle.path(), "ownership.zero_native_delta");
        let delta_entry = forged
            .entries
            .iter_mut()
            .find(|entry| entry.role == "ownership.zero-native-delta")
            .unwrap();
        let path = bundle.path().join(&delta_entry.path);
        let mut delta: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        delta["base_sha"] = serde_json::json!("c".repeat(40));
        let bytes = serde_json::to_vec(&delta).unwrap();
        fs::write(&path, &bytes).unwrap();
        delta_entry.byte_length = bytes.len() as u64;
        delta_entry.sha256 = hex_sha256(&bytes);
        assert!(production_contracts(bundle.path(), &forged)
            .iter()
            .any(|contract| contract == "evidence.ownership.zero_native_delta.identity"));
    }

    #[test]
    fn delta_replay_rejects_identity_category_count_path_and_result_forgery() {
        assert!(tampered_delta_contracts(|report| {
            report["head_sha"] = serde_json::json!("c".repeat(40));
        })
        .iter()
        .any(|contract| contract == "evidence.ownership.zero_native_delta.identity"));

        assert!(tampered_delta_contracts(|report| {
            report["categories"]
                .as_object_mut()
                .unwrap()
                .remove("bridges");
        })
        .iter()
        .any(|contract| contract == "evidence.ownership.zero_native_delta.category_count"));

        let contracts = tampered_delta_contracts(|report| {
            report["categories"]["tracked_sources"]["changed_paths"] =
                serde_json::json!(["rust/src/lib.rs", "../outside"]);
        });
        assert!(contracts.iter().any(
            |contract| contract == "evidence.ownership.zero_native_delta.tracked_sources.count"
        ));
        assert!(contracts.iter().any(
            |contract| contract == "evidence.ownership.zero_native_delta.tracked_sources.path"
        ));

        assert!(tampered_delta_contracts(|report| {
            report["passed"] = serde_json::json!(true);
        })
        .iter()
        .any(|contract| contract == "evidence.ownership.zero_native_delta.result"));

        let contracts = tampered_delta_contracts(|report| {
            report["transitional_native_inputs"]["head_count"] = serde_json::json!(322);
        });
        assert!(contracts.iter().any(|contract| {
            contract == "evidence.ownership.zero_native_delta.transitional_inputs.result"
        }));
        assert!(!tampered_delta_contracts(|report| {
            report["transitional_native_inputs"]["head_count"] = serde_json::json!(322);
            report["transitional_native_inputs"]["passed"] = serde_json::json!(false);
            report["passed"] = serde_json::json!(false);
        })
        .iter()
        .any(|contract| {
            contract == "evidence.ownership.zero_native_delta.transitional_inputs.result"
        }));

        assert!(tampered_delta_contracts(|report| {
            report
                .as_object_mut()
                .unwrap()
                .remove("transitional_native_inputs");
        })
        .iter()
        .any(|contract| {
            contract == "evidence.ownership.zero_native_delta.transitional_inputs.fields"
        }));

        for mutate in [
            |report: &mut serde_json::Value| {
                report["transitional_native_inputs"]["maximum_count"] = serde_json::json!(322);
            },
            |report: &mut serde_json::Value| {
                report["transitional_native_inputs"]["base_count"] = serde_json::json!("321");
            },
        ] {
            assert!(tampered_delta_contracts(mutate).iter().any(|contract| {
                contract == "evidence.ownership.zero_native_delta.transitional_inputs.result"
            }));
        }
    }

    #[test]
    fn control_receipts_reject_unknown_fields() {
        for (contract, role, expected_contract) in [
            (
                "source.expected_sha",
                "preflight.source",
                "evidence.preflight.source.content",
            ),
            (
                "tools.preflight",
                "preflight.tools",
                "evidence.preflight.tools.fields",
            ),
        ] {
            let bundle = tempfile::tempdir().unwrap();
            let mut index = preflight_failure_bundle(bundle.path(), contract);
            let entry = index
                .entries
                .iter()
                .find(|entry| entry.role == role)
                .unwrap();
            let mut receipt: serde_json::Value =
                serde_json::from_slice(&fs::read(bundle.path().join(&entry.path)).unwrap())
                    .unwrap();
            receipt["unexpected"] = serde_json::json!(true);
            rewrite_bundle_entry(
                bundle.path(),
                &mut index.entries,
                role,
                &serde_json::to_vec(&receipt).unwrap(),
            );
            let contracts = production_contracts(bundle.path(), &index);
            assert!(
                contracts.iter().any(|actual| actual == expected_contract),
                "{contract}: {contracts:?}"
            );
        }

        let bundle = tempfile::tempdir().unwrap();
        let mut index = preflight_failure_bundle(bundle.path(), "tools.preflight");
        let entry = index
            .entries
            .iter()
            .find(|entry| entry.role == "preflight.tools")
            .unwrap();
        let mut report: serde_json::Value =
            serde_json::from_slice(&fs::read(bundle.path().join(&entry.path)).unwrap()).unwrap();
        report["observations"][0]["unexpected"] = serde_json::json!(true);
        rewrite_bundle_entry(
            bundle.path(),
            &mut index.entries,
            "preflight.tools",
            &serde_json::to_vec(&report).unwrap(),
        );
        assert!(production_contracts(bundle.path(), &index)
            .iter()
            .any(|actual| actual.ends_with(".fields")));

        for (contract, role, expected_contract) in [
            (
                "cache.rust_target",
                "cache.initial-state",
                "evidence.cache.initial_state.fields",
            ),
            (
                "ownership.zero_native_delta",
                "ownership.zero-native-delta",
                "evidence.ownership.zero_native_delta.fields",
            ),
        ] {
            let bundle = tempfile::tempdir().unwrap();
            let mut index = setup_failure_bundle(bundle.path(), contract);
            let entry = index
                .entries
                .iter()
                .find(|entry| entry.role == role)
                .unwrap();
            let mut receipt: serde_json::Value =
                serde_json::from_slice(&fs::read(bundle.path().join(&entry.path)).unwrap())
                    .unwrap();
            receipt["unexpected"] = serde_json::json!(true);
            rewrite_bundle_entry(
                bundle.path(),
                &mut index.entries,
                role,
                &serde_json::to_vec(&receipt).unwrap(),
            );
            let contracts = production_contracts(bundle.path(), &index);
            assert!(
                contracts.iter().any(|actual| actual == expected_contract),
                "{contract}: {contracts:?}"
            );
        }
    }

    #[test]
    fn tool_receipt_retains_supervised_execution_failure() {
        let bundle = tempfile::tempdir().unwrap();
        let mut index = preflight_failure_bundle(bundle.path(), "tools.preflight");
        let entry = index
            .entries
            .iter()
            .find(|entry| entry.role == "preflight.tools")
            .unwrap();
        let mut report: serde_json::Value =
            serde_json::from_slice(&fs::read(bundle.path().join(&entry.path)).unwrap()).unwrap();
        let failed = report["observations"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|observation| observation["passed"] == serde_json::json!(false))
            .unwrap();
        let expected_prefix = failed["expected_output_prefix"]
            .as_str()
            .unwrap()
            .to_string();
        failed["stdout"] = serde_json::json!(format!("{expected_prefix}fixture"));
        failed["exit_code"] = serde_json::json!(0);
        failed["launch_error"] = serde_json::json!(
            "supervision: subprocess cargo left descendants in its owned process group"
        );
        rewrite_bundle_entry(
            bundle.path(),
            &mut index.entries,
            "preflight.tools",
            &serde_json::to_vec(&report).unwrap(),
        );

        let contracts = production_contracts(bundle.path(), &index);
        assert!(
            !contracts
                .iter()
                .any(|contract| contract.ends_with(".result")),
            "{contracts:?}"
        );
    }

    #[test]
    fn source_preflight_failures_validate_without_gate_or_delta_receipts() {
        for contract in [
            "source.expected_sha",
            "source.expected_tuple",
            "source.clean",
            "environment.canonical",
        ] {
            let bundle = tempfile::tempdir().unwrap();
            let index = preflight_failure_bundle(bundle.path(), contract);
            assert!(index.offline_validation.passed, "{contract}: {index:#?}");
            assert!(!index.entries.iter().any(|entry| matches!(
                entry.role.as_str(),
                "gate.result" | "ownership.zero-native-delta"
            )));
        }
    }

    fn forge_tool_identity(mutate: impl FnOnce(&mut serde_json::Value)) -> Vec<String> {
        let bundle = tempfile::tempdir().unwrap();
        let mut index = preflight_failure_bundle(bundle.path(), "tools.preflight");
        let report_path = bundle.path().join(
            &index
                .entries
                .iter()
                .find(|entry| entry.role == "preflight.tools")
                .unwrap()
                .path,
        );
        let mut report: serde_json::Value =
            serde_json::from_slice(&fs::read(&report_path).unwrap()).unwrap();
        mutate(&mut report["observations"][0]);
        rewrite_bundle_entry(
            bundle.path(),
            &mut index.entries,
            "preflight.tools",
            &serde_json::to_vec(&report).unwrap(),
        );
        production_contracts(bundle.path(), &index)
    }

    #[test]
    fn tool_preflight_failure_requires_a_failed_authoritative_observation() {
        let bundle = tempfile::tempdir().unwrap();
        let index = preflight_failure_bundle(bundle.path(), "tools.preflight");
        assert!(index.offline_validation.passed, "{index:#?}");
        assert!(!index.entries.iter().any(|entry| matches!(
            entry.role.as_str(),
            "gate.result" | "ownership.zero-native-delta"
        )));

        let mut forged = index.clone();
        let tool_entry = forged
            .entries
            .iter_mut()
            .find(|entry| entry.role == "preflight.tools")
            .unwrap();
        let report_path = bundle.path().join(&tool_entry.path);
        let mut report: serde_json::Value =
            serde_json::from_slice(&fs::read(&report_path).unwrap()).unwrap();
        for observation in report["observations"].as_array_mut().unwrap() {
            observation["passed"] = serde_json::json!(true);
        }
        report["passed"] = serde_json::json!(true);
        let bytes = serde_json::to_vec(&report).unwrap();
        fs::write(&report_path, &bytes).unwrap();
        tool_entry.byte_length = bytes.len() as u64;
        tool_entry.sha256 = hex_sha256(&bytes);
        assert!(production_contracts(bundle.path(), &forged)
            .iter()
            .any(|contract| contract == "evidence.preflight.tools.missing_failure"));

        let bundle = tempfile::tempdir().unwrap();
        let mut duplicated = preflight_failure_bundle(bundle.path(), "tools.preflight");
        let report_path = bundle.path().join(
            &duplicated
                .entries
                .iter()
                .find(|entry| entry.role == "preflight.tools")
                .unwrap()
                .path,
        );
        let mut report: serde_json::Value =
            serde_json::from_slice(&fs::read(&report_path).unwrap()).unwrap();
        let duplicate = report["observations"][0].clone();
        report["observations"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        rewrite_bundle_entry(
            bundle.path(),
            &mut duplicated.entries,
            "preflight.tools",
            &serde_json::to_vec(&report).unwrap(),
        );
        let contracts = production_contracts(bundle.path(), &duplicated);
        assert!(contracts
            .iter()
            .any(|contract| contract == "evidence.preflight.tools.observation_count"));
        assert!(contracts
            .iter()
            .any(|contract| contract.starts_with("evidence.preflight.tools.")
                && contract.ends_with(".identity")));

        let bundle = tempfile::tempdir().unwrap();
        let mut forged = preflight_failure_bundle(bundle.path(), "tools.preflight");
        let report_path = bundle.path().join(
            &forged
                .entries
                .iter()
                .find(|entry| entry.role == "preflight.tools")
                .unwrap()
                .path,
        );
        let mut report: serde_json::Value =
            serde_json::from_slice(&fs::read(&report_path).unwrap()).unwrap();
        let failed = report["observations"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|observation| observation["passed"] == serde_json::json!(false))
            .unwrap();
        failed["exit_code"] = serde_json::json!(0);
        failed["stdout"] = serde_json::json!(format!(
            "{}fixture",
            failed["expected_output_prefix"].as_str().unwrap()
        ));
        rewrite_bundle_entry(
            bundle.path(),
            &mut forged.entries,
            "preflight.tools",
            &serde_json::to_vec(&report).unwrap(),
        );
        assert!(production_contracts(bundle.path(), &forged)
            .iter()
            .any(|contract| contract.ends_with(".result")));
    }

    #[test]
    fn tool_preflight_rejects_forged_executable_identities() {
        let identity_failure = |contracts: Vec<String>| {
            assert!(contracts
                .iter()
                .any(|contract| contract.ends_with(".executable_identity")));
        };
        identity_failure(forge_tool_identity(|observation| {
            observation
                .as_object_mut()
                .unwrap()
                .remove("executable_identity");
        }));
        identity_failure(forge_tool_identity(|observation| {
            observation["executable_identity"]["extra"] = serde_json::json!(true);
        }));
        identity_failure(forge_tool_identity(|observation| {
            observation["executable_identity"]["path"] = serde_json::json!("/usr/bin/../bin/cargo");
        }));
        identity_failure(forge_tool_identity(|observation| {
            observation["executable_identity"]["byte_length"] = serde_json::json!(0);
        }));
        identity_failure(forge_tool_identity(|observation| {
            observation["executable_identity"]["sha256"] = serde_json::json!("not-a-digest");
        }));
        identity_failure(forge_tool_identity(|observation| {
            observation["executable_identity"]["mode"] = serde_json::json!(0o644);
        }));
        identity_failure(forge_tool_identity(|observation| {
            observation["executable_identity"]["mode"] = serde_json::json!(0o10_755);
        }));
        assert!(forge_tool_identity(|observation| {
            observation["executable_identity"] = serde_json::Value::Null;
        })
        .iter()
        .any(|contract| contract.ends_with(".result")));

        let launch_failure = forge_tool_identity(|observation| {
            observation["executable_identity"] = serde_json::Value::Null;
            observation["stdout"] = serde_json::json!("");
            observation["stderr"] = serde_json::json!("");
            observation["exit_code"] = serde_json::Value::Null;
            observation["signal"] = serde_json::Value::Null;
            observation["launch_error"] = serde_json::json!("executable not found");
            observation["passed"] = serde_json::json!(false);
        });
        assert!(!launch_failure.iter().any(|contract| {
            contract.ends_with(".executable_identity") || contract.ends_with(".result")
        }));

        let signal_failure = forge_tool_identity(|observation| {
            observation["exit_code"] = serde_json::Value::Null;
            observation["signal"] = serde_json::json!(9);
            observation["passed"] = serde_json::json!(false);
        });
        assert!(!signal_failure
            .iter()
            .any(|contract| contract.ends_with(".result")));

        for contracts in [
            forge_tool_identity(|observation| {
                observation["signal"] = serde_json::json!(9);
            }),
            forge_tool_identity(|observation| {
                observation["exit_code"] = serde_json::json!(-1);
            }),
            forge_tool_identity(|observation| {
                observation["exit_code"] = serde_json::Value::Null;
                observation["signal"] = serde_json::json!(0);
                observation["passed"] = serde_json::json!(false);
            }),
            forge_tool_identity(|observation| {
                observation["launch_error"] = serde_json::json!("unexpected launch error");
                observation["passed"] = serde_json::json!(false);
            }),
            forge_tool_identity(|observation| {
                observation["exit_code"] = serde_json::Value::Null;
                observation["signal"] = serde_json::Value::Null;
                observation["launch_error"] = serde_json::json!("executable not found");
                observation["passed"] = serde_json::json!(false);
            }),
        ] {
            assert!(contracts
                .iter()
                .any(|contract| contract.ends_with(".result")));
        }
    }

    #[test]
    fn tool_preflight_uses_authorized_exit_codes() {
        let bundle = tempfile::tempdir().unwrap();
        let mut index = preflight_failure_bundle(bundle.path(), "tools.preflight");
        let report_entry = index
            .entries
            .iter()
            .find(|entry| entry.role == "preflight.tools")
            .unwrap();
        let report_path = bundle.path().join(&report_entry.path);
        let mut report: serde_json::Value =
            serde_json::from_slice(&fs::read(&report_path).unwrap()).unwrap();
        let observations = report["observations"].as_array_mut().unwrap();
        let pinned_failure = observations
            .iter_mut()
            .find(|observation| observation["passed"] == serde_json::json!(false))
            .unwrap();
        pinned_failure["exit_code"] = serde_json::json!(0);
        pinned_failure["stdout"] = serde_json::json!(format!(
            "{}fixture",
            pinned_failure["expected_output_prefix"].as_str().unwrap()
        ));
        pinned_failure["passed"] = serde_json::json!(true);
        let cargo = observations
            .iter_mut()
            .find(|observation| observation["name"] == serde_json::json!("cargo"))
            .unwrap();
        cargo["exit_code"] = serde_json::json!(9);
        cargo["passed"] = serde_json::json!(false);
        rewrite_bundle_entry(
            bundle.path(),
            &mut index.entries,
            "preflight.tools",
            &serde_json::to_vec(&report).unwrap(),
        );
        assert!(production_contracts(bundle.path(), &index).is_empty());

        let bundle = tempfile::tempdir().unwrap();
        let mut index = preflight_failure_bundle(bundle.path(), "source.expected_sha");
        let report_entry = index
            .entries
            .iter()
            .find(|entry| entry.role == "preflight.tools")
            .unwrap();
        let report_path = bundle.path().join(&report_entry.path);
        let mut report: serde_json::Value =
            serde_json::from_slice(&fs::read(&report_path).unwrap()).unwrap();
        let ar = report["observations"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|observation| observation["name"] == serde_json::json!("ar"))
            .unwrap();
        ar["exit_code"] = serde_json::json!(1);
        rewrite_bundle_entry(
            bundle.path(),
            &mut index.entries,
            "preflight.tools",
            &serde_json::to_vec(&report).unwrap(),
        );
        assert!(production_contracts(bundle.path(), &index).is_empty());
    }

    #[test]
    fn advisory_database_pack_parser_rejects_malformed_streams() {
        fn pack(entries: &[(&str, &[u8])]) -> Vec<u8> {
            let mut bytes = b"UQM-S4-ADVISORY-DB-V1\0".to_vec();
            for (path, content) in entries {
                bytes.extend_from_slice(&(path.len() as u32).to_be_bytes());
                bytes.extend_from_slice(&(content.len() as u64).to_be_bytes());
                bytes.extend_from_slice(path.as_bytes());
                bytes.extend_from_slice(content);
            }
            bytes.extend_from_slice(&0_u32.to_be_bytes());
            bytes
        }

        let valid = pack(&[("a.md", b"a"), ("nested/b.md", b"beta")]);
        assert_eq!(parse_advisory_database_pack(&valid), Ok(2));

        let mut truncated = valid.clone();
        truncated.pop();
        assert!(parse_advisory_database_pack(&truncated).is_err());
        let mut trailing = valid.clone();
        trailing.push(0);
        assert!(parse_advisory_database_pack(&trailing).is_err());
        assert!(parse_advisory_database_pack(&pack(&[("b", b""), ("a", b"")])).is_err());
        assert!(parse_advisory_database_pack(&pack(&[("a", b""), ("a", b"")])).is_err());
        assert!(parse_advisory_database_pack(&pack(&[("../escape", b"")])).is_err());
        assert!(parse_advisory_database_pack(&pack(&[(".git/config", b"")])).is_err());

        let mut oversized = b"UQM-S4-ADVISORY-DB-V1\0".to_vec();
        oversized.extend_from_slice(&4097_u32.to_be_bytes());
        oversized.extend_from_slice(&0_u64.to_be_bytes());
        assert!(parse_advisory_database_pack(&oversized).is_err());
    }

    #[test]
    #[ignore = "requires UQM_TEST_ADVISORY_DATABASE_PACK pointing at the pinned pack"]
    fn advisory_database_parser_accepts_pinned_pack() {
        let path = std::env::var_os("UQM_TEST_ADVISORY_DATABASE_PACK")
            .map(PathBuf::from)
            .expect("UQM_TEST_ADVISORY_DATABASE_PACK must name the pinned pack");
        let bytes = fs::read(path).unwrap();
        let authority: Authority =
            serde_json::from_slice(include_bytes!("../../../ci/gates.json")).unwrap();
        assert_eq!(
            parse_advisory_database_pack(&bytes).unwrap(),
            authority.security.advisory_database_file_count as usize
        );
        assert_eq!(
            hex_sha256(&bytes),
            authority.security.advisory_database_pack_sha256
        );
    }

    #[test]
    fn security_revision_receipt_must_match_the_authority_pin() {
        let bundle = tempfile::tempdir().unwrap();
        let authority: Authority =
            serde_json::from_slice(include_bytes!("../../../ci/gates.json")).unwrap();
        let revision_step = authority
            .gate("security")
            .unwrap()
            .steps
            .iter()
            .find(|step| step.id == "advisory-db-revision")
            .unwrap();
        let path = "security/advisory-db-revision.stdout.log";
        fs::create_dir_all(bundle.path().join("security")).unwrap();
        fs::write(
            bundle.path().join(path),
            format!("{}\n", authority.security.advisory_database_revision),
        )
        .unwrap();
        let mut index = valid_index();
        index.entries = vec![entry(
            bundle.path(),
            path,
            "step.stdout",
            "text/plain",
            "security",
            &revision_step.command,
        )
        .unwrap()];
        assert!(security_revision_matches(
            bundle.path(),
            &index,
            &authority,
            revision_step,
        ));

        fs::write(bundle.path().join(path), format!("{}\n", "b".repeat(40))).unwrap();
        assert!(!security_revision_matches(
            bundle.path(),
            &index,
            &authority,
            revision_step,
        ));
    }

    #[test]
    fn advisory_database_evidence_requires_exact_identity_hash_and_count() {
        let bundle = tempfile::tempdir().unwrap();
        let command = vec![
            "uqm-xtask".to_string(),
            "ci".to_string(),
            "run".to_string(),
            "security".to_string(),
        ];
        let mut bytes = b"UQM-S4-ADVISORY-DB-V1\0".to_vec();
        bytes.extend_from_slice(&4_u32.to_be_bytes());
        bytes.extend_from_slice(&1_u64.to_be_bytes());
        bytes.extend_from_slice(b"a.md");
        bytes.push(b"a"[0]);
        bytes.extend_from_slice(&0_u32.to_be_bytes());
        let path = "payloads/security.advisory-database/advisory-database.pack";
        fs::create_dir_all(bundle.path().join("payloads/security.advisory-database")).unwrap();
        fs::write(bundle.path().join(path), &bytes).unwrap();

        let mut authority: Authority =
            serde_json::from_slice(include_bytes!("../../../ci/gates.json")).unwrap();
        authority.security.advisory_database_pack_sha256 = hex_sha256(&bytes);
        authority.security.advisory_database_file_count = 1;
        let mut index = valid_index();
        index.entries = vec![
            EvidenceEntry {
                schema: ENTRY_SCHEMA.to_string(),
                role: "authority.snapshot".to_string(),
                path: "payloads/authority.snapshot/gates.json".to_string(),
                mime: "application/json".to_string(),
                byte_length: 0,
                sha256: "0".repeat(64),
                producing_gate: "security".to_string(),
                producing_command: command.clone(),
            },
            EvidenceEntry {
                schema: ENTRY_SCHEMA.to_string(),
                role: "security.advisory-database".to_string(),
                path: path.to_string(),
                mime: "application/octet-stream".to_string(),
                byte_length: bytes.len() as u64,
                sha256: hex_sha256(&bytes),
                producing_gate: "security".to_string(),
                producing_command: command,
            },
        ];
        assert!(validate_security_advisory_database(bundle.path(), &index, &authority).is_empty());

        index.entries[1].path = "advisory-database.pack".to_string();
        assert!(
            validate_security_advisory_database(bundle.path(), &index, &authority)
                .iter()
                .any(|contract| contract == "evidence.security.advisory_database.identity")
        );
        index.entries[1].path = path.to_string();
        index.entries[1].mime = "application/json".to_string();
        assert!(
            validate_security_advisory_database(bundle.path(), &index, &authority)
                .iter()
                .any(|contract| contract == "evidence.security.advisory_database.identity")
        );
        index.entries[1].mime = "application/octet-stream".to_string();
        authority.security.advisory_database_file_count = 2;
        assert!(
            validate_security_advisory_database(bundle.path(), &index, &authority)
                .iter()
                .any(|contract| contract == "evidence.security.advisory_database.file_count")
        );
        authority.security.advisory_database_file_count = 1;
        authority.security.advisory_database_pack_sha256 = "0".repeat(64);
        assert!(
            validate_security_advisory_database(bundle.path(), &index, &authority)
                .iter()
                .any(|contract| contract == "evidence.security.advisory_database.hash")
        );
        index.entries.pop();
        assert!(
            validate_security_advisory_database(bundle.path(), &index, &authority)
                .iter()
                .any(|contract| contract.starts_with("evidence.security.advisory_database.count"))
        );
    }

    #[test]
    fn all_controller_security_retention_failure_replays_offline() {
        let bundle = tempfile::tempdir().unwrap();
        let index = all_gates_security_post_failure_bundle(bundle.path());
        assert!(index.offline_validation.passed);
        assert_eq!(
            index.first_failed_contract.as_deref(),
            Some("security.post.database-retain")
        );
        let index_path = bundle.path().join("evidence-index.json");
        fs::write(&index_path, serde_json::to_vec_pretty(&index).unwrap()).unwrap();
        validate_evidence_command(
            Path::new("/definitely-not-a-repository"),
            index_path.to_str().unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn security_post_failure_requires_all_successful_steps_and_no_database() {
        let bundle = tempfile::tempdir().unwrap();
        let authority: Authority =
            serde_json::from_slice(include_bytes!("../../../ci/gates.json")).unwrap();
        let gate = authority.gate("security").unwrap();
        let mut index = valid_index();
        index.entries.clear();
        for step in &gate.steps {
            write_builtin_step_fixture(
                bundle.path(),
                &mut index.entries,
                "security",
                &step.id,
                &step.command,
                (Some(0), None, None),
            );
        }
        let revision_stdout = format!("{}\n", authority.security.advisory_database_revision);
        rewrite_bundle_path(
            bundle.path(),
            &mut index.entries,
            "security/advisory-db-revision.stdout.log",
            revision_stdout.as_bytes(),
        );
        assert!(validate_failed_security_postprocess(
            bundle.path(),
            &index,
            &authority,
            gate,
            "security.post.database-identity",
        )
        .is_empty());
        assert!(validate_failed_security_postprocess(
            bundle.path(),
            &index,
            &authority,
            gate,
            "security.post.database-retain",
        )
        .is_empty());

        rewrite_bundle_path(
            bundle.path(),
            &mut index.entries,
            "security/advisory-db-revision.stdout.log",
            b"forged-revision\n",
        );
        assert!(validate_failed_security_postprocess(
            bundle.path(),
            &index,
            &authority,
            gate,
            "security.post.database-retain",
        )
        .iter()
        .any(|contract| contract == "evidence.security.advisory_database_revision"));
        rewrite_bundle_path(
            bundle.path(),
            &mut index.entries,
            "security/advisory-db-revision.stdout.log",
            revision_stdout.as_bytes(),
        );

        index.entries.pop();
        assert!(validate_failed_security_postprocess(
            bundle.path(),
            &index,
            &authority,
            gate,
            "security.post.database-retain",
        )
        .iter()
        .any(|contract| contract == "evidence.security.post.step_entry_count"));
        index.entries.push(EvidenceEntry {
            schema: ENTRY_SCHEMA.to_string(),
            role: "security.advisory-database".to_string(),
            path: "payloads/security.advisory-database/advisory-database.pack".to_string(),
            mime: "application/octet-stream".to_string(),
            byte_length: 0,
            sha256: hex_sha256(&[]),
            producing_gate: "security".to_string(),
            producing_command: vec![
                "uqm-xtask".to_string(),
                "ci".to_string(),
                "run".to_string(),
                "security".to_string(),
            ],
        });
        assert!(validate_failed_security_postprocess(
            bundle.path(),
            &index,
            &authority,
            gate,
            "security.post.database-retain",
        )
        .iter()
        .any(|contract| contract == "evidence.security.post.unexpected_database"));
    }

    fn failed_process_fixture(
        root: &Path,
        gate: &super::super::authority::Gate,
        failed_position: usize,
    ) -> EvidenceIndex {
        let mut entries = Vec::new();
        for (position, step) in gate.steps.iter().take(failed_position + 1).enumerate() {
            let (effective_command, staged_script_sha256) =
                fixture_execution_provenance(&step.command);
            for (suffix, role, bytes) in [
                ("stdout.log", "step.stdout", Vec::new()),
                ("stderr.log", "step.stderr", Vec::new()),
                (
                    "result.json",
                    "step.result",
                    serde_json::to_vec(&serde_json::json!({
                        "schema": "uqm-s4-step-result-v2",
                        "gate": gate.id,
                        "step": step.id,
                        "command": step.command,
                        "effective_command": effective_command,
                        "staged_script_sha256": staged_script_sha256,
                        "executable_identity": fixture_executable_identity(None),
                        "exit_code": if position == failed_position { 1 } else { 0 },
                        "signal": null,
                        "launch_error": null,
                        "success": position < failed_position,
                        "supervision": fixture_supervision(false, 0, 0),
                    }))
                    .unwrap(),
                ),
            ] {
                let path = format!("{}/{}.{}", gate.id, step.id, suffix);
                let full = root.join(&path);
                fs::create_dir_all(full.parent().unwrap()).unwrap();
                fs::write(&full, &bytes).unwrap();
                entries.push(
                    entry(
                        root,
                        &path,
                        role,
                        if role == "step.result" {
                            "application/json"
                        } else {
                            "text/plain"
                        },
                        &gate.id,
                        &step.command,
                    )
                    .unwrap(),
                );
            }
        }
        EvidenceIndex {
            schema: EVIDENCE_SCHEMA.into(),
            source_sha: "a".repeat(40),
            clean: true,
            tuple: "linux-x86_64".into(),
            supported_tuples: fixture_tuples(),
            profile: PROFILE.into(),
            features: vec!["audio_heart".into()],
            cache_mode: "ambient-dev".into(),
            first_failed_contract: Some(format!("{}.{}", gate.id, gate.steps[failed_position].id)),
            offline_validation: OfflineValidation {
                passed: false,
                contracts: Vec::new(),
            },
            entries,
        }
    }

    #[test]
    fn failed_process_replay_accepts_exact_trusted_hidden_controller_routes() {
        let authority: Authority =
            serde_json::from_slice(include_bytes!("../../../ci/gates.json")).unwrap();
        let gate = authority.gate("tests").unwrap();
        let bundle = tempfile::tempdir().unwrap();
        let index = failed_process_fixture(bundle.path(), gate, 1);

        assert_eq!(
            validate_failed_process_receipts(
                bundle.path(),
                &index,
                gate,
                index.first_failed_contract.as_deref().unwrap(),
            ),
            Vec::<String>::new()
        );
    }

    #[test]
    fn failed_process_replay_requires_the_exact_executed_step_prefix() {
        let authority: Authority =
            serde_json::from_slice(include_bytes!("../../../ci/gates.json")).unwrap();
        let gate = authority.gate("check").unwrap();
        let bundle = tempfile::tempdir().unwrap();
        let valid = failed_process_fixture(bundle.path(), gate, 1);
        assert_eq!(
            validate_failed_process_receipts(
                bundle.path(),
                &valid,
                gate,
                valid.first_failed_contract.as_deref().unwrap()
            ),
            Vec::<String>::new()
        );

        let failed_result_position = valid
            .entries
            .iter()
            .position(|entry| entry.path.ends_with("check-linked-bin.result.json"))
            .unwrap();
        let failed_result_path = valid.entries[failed_result_position].path.clone();
        let original_result = fs::read(bundle.path().join(&failed_result_path)).unwrap();
        for (exit_code, signal, launch_error) in [
            (
                serde_json::json!(1),
                serde_json::json!(9),
                serde_json::Value::Null,
            ),
            (
                serde_json::json!(-1),
                serde_json::Value::Null,
                serde_json::Value::Null,
            ),
            (
                serde_json::Value::Null,
                serde_json::json!(0),
                serde_json::Value::Null,
            ),
            (
                serde_json::json!(1),
                serde_json::Value::Null,
                serde_json::json!("launch failed"),
            ),
        ] {
            let mut forged: serde_json::Value = serde_json::from_slice(&original_result).unwrap();
            forged["exit_code"] = exit_code;
            forged["signal"] = signal;
            forged["launch_error"] = launch_error;
            let bytes = serde_json::to_vec(&forged).unwrap();
            fs::write(bundle.path().join(&failed_result_path), &bytes).unwrap();
            let mut forged_index = valid.clone();
            forged_index.entries[failed_result_position].byte_length = bytes.len() as u64;
            forged_index.entries[failed_result_position].sha256 = hex_sha256(&bytes);
            assert!(validate_failed_process_receipts(
                bundle.path(),
                &forged_index,
                gate,
                forged_index.first_failed_contract.as_deref().unwrap(),
            )
            .iter()
            .any(
                |contract| contract == "evidence.step.check.check-linked-bin.failed_result_content"
            ));
        }
        fs::write(bundle.path().join(&failed_result_path), &original_result).unwrap();

        let mut omitted_result = valid.clone();
        omitted_result.entries.retain(|entry| {
            !(entry.role == "step.result" && entry.path.ends_with("check-linked-bin.result.json"))
        });
        let contracts = validate_failed_process_receipts(
            bundle.path(),
            &omitted_result,
            gate,
            omitted_result.first_failed_contract.as_deref().unwrap(),
        );
        assert!(contracts.iter().any(|contract| {
            contract.starts_with("evidence.gate.check.failed_step_entry_count")
        }));
        assert!(contracts.iter().any(|contract| {
            contract.starts_with("evidence.step.check.check-linked-bin.step.result")
        }));

        let extra_bundle = tempfile::tempdir().unwrap();
        let mut extra_skipped = failed_process_fixture(extra_bundle.path(), gate, 2);
        extra_skipped.first_failed_contract = valid.first_failed_contract.clone();
        let contracts = validate_failed_process_receipts(
            extra_bundle.path(),
            &extra_skipped,
            gate,
            extra_skipped.first_failed_contract.as_deref().unwrap(),
        );
        assert!(contracts.iter().any(|contract| {
            contract.starts_with("evidence.gate.check.failed_step_entry_count")
        }));

        let contracts = validate_failed_process_receipts(
            bundle.path(),
            &valid,
            gate,
            "check.not-an-authoritative-step",
        );
        assert_eq!(
            contracts,
            vec!["evidence.gate.check.failed_step_identity".to_string()]
        );
    }

    #[test]
    fn extracted_format_bundle_validates_without_a_repository() {
        let bundle = tempfile::tempdir().unwrap();
        let root = bundle.path();
        let controller = vec![
            "uqm-xtask".into(),
            "ci".into(),
            "run".into(),
            "format".into(),
        ];
        let source_sha = "a".repeat(40);
        let mut entries = Vec::new();
        let authority_bytes = include_bytes!("../../../ci/gates.json");
        let authority: Authority = serde_json::from_slice(authority_bytes).unwrap();
        write_bundle_entry(
            root,
            &mut entries,
            "payloads/authority.snapshot/gates.json",
            "authority.snapshot",
            &controller,
            authority_bytes,
        );
        let source = serde_json::json!({
            "schema": "uqm-s4-source-preflight-v2", "source_sha": source_sha,
            "detached_state": null, "expected_sha": null, "base_sha": null, "tuple": "linux-x86_64",
            "expected_tuple": null, "cache_mode": "ambient-dev", "clean": true,
            "canonical_environment": false, "passed": true,
            "first_failed_contract": null, "detail": null
        });
        write_bundle_entry(
            root,
            &mut entries,
            "source-preflight.json",
            "preflight.source",
            &controller,
            &serde_json::to_vec(&source).unwrap(),
        );
        let mut observations: Vec<_> = authority
            .tools
            .preflight
            .iter()
            .map(|probe| {
                serde_json::json!({
                    "name": probe.name, "command": probe.version_command,
                    "expected_output_prefix": probe.expected_output_prefix,
                    "executable_identity": {
                        "path": format!("/usr/bin/{}", probe.name), "byte_length": 1,
                        "sha256": "c".repeat(64), "mode": 0o755
                    },
                    "stdout": probe.expected_output_prefix.as_deref().map_or_else(|| "available".to_string(), |prefix| format!("{prefix}fixture")),
                    "stderr": "", "exit_code": 0, "signal": null,
                    "launch_error": null, "passed": true
                })
            })
            .collect();
        observations.extend(authority.tools.entries().into_iter().map(|(name, tool)| {
            serde_json::json!({
                "name": name, "command": tool.version_command,
                "expected_output_prefix": tool.expected_output_prefix,
                "executable_identity": {
                    "path": format!("/usr/bin/{name}"), "byte_length": 1,
                    "sha256": "c".repeat(64), "mode": 0o755
                },
                "stdout": format!("{}fixture", tool.expected_output_prefix),
                "stderr": "", "exit_code": 0, "signal": null,
                "launch_error": null, "passed": true
            })
        }));
        let tools = serde_json::json!({"schema": "uqm-s4-tool-preflight-v2", "passed": true, "observations": observations});
        write_bundle_entry(
            root,
            &mut entries,
            "tool-preflight.json",
            "preflight.tools",
            &controller,
            &serde_json::to_vec(&tools).unwrap(),
        );
        write_bundle_entry(
            root,
            &mut entries,
            "cache-initial-state.json",
            "cache.initial-state",
            &controller,
            &ambient_cache_receipt(),
        );
        let step = &authority.gate("format").unwrap().steps[0];
        write_bundle_entry(
            root,
            &mut entries,
            "format/fmt-check.stdout.log",
            "step.stdout",
            &step.command,
            b"",
        );
        write_bundle_entry(
            root,
            &mut entries,
            "format/fmt-check.stderr.log",
            "step.stderr",
            &step.command,
            b"",
        );
        let step_result = serde_json::json!({
            "schema": "uqm-s4-step-result-v2", "gate": "format", "step": "fmt-check",
            "command": step.command, "effective_command": step.command,
            "staged_script_sha256": null, "executable_identity": fixture_executable_identity(None),
            "exit_code": 0, "signal": null, "launch_error": null, "success": true,
            "supervision": fixture_supervision(false, 0, 0),
        });
        write_bundle_entry(
            root,
            &mut entries,
            "format/fmt-check.result.json",
            "step.result",
            &step.command,
            &serde_json::to_vec(&step_result).unwrap(),
        );
        let gate_result = serde_json::json!({
            "schema": "uqm-s4-gate-result-v1", "gate": "format", "owner": "S4",
            "kind": "process", "passed": true, "first_failed_contract": null,
            "detail": null, "controller_command": controller
        });
        write_bundle_entry(
            root,
            &mut entries,
            "format/gate.result.json",
            "gate.result",
            &controller,
            &serde_json::to_vec(&gate_result).unwrap(),
        );
        let index = EvidenceIndex::build_and_validate(
            root,
            &fixture_tuples(),
            EvidenceContext {
                source_sha,
                clean: true,
                tuple: "linux-x86_64".into(),
                features: vec!["audio_heart".into()],
                cache_mode: "ambient-dev".into(),
                first_failed_contract: None,
            },
            entries,
        )
        .unwrap();
        assert!(
            index.offline_validation.passed,
            "{:#?}",
            index.offline_validation
        );
        let index_path = root.join(INDEX_FILENAME);
        fs::write(&index_path, serde_json::to_vec_pretty(&index).unwrap()).unwrap();
        validate_evidence_command(
            Path::new("/definitely-not-a-repository"),
            index_path.to_str().unwrap(),
        )
        .unwrap();

        let mut omitted = index.clone();
        omitted.entries.retain(|entry| entry.role != "step.result");
        let contracts = production_contracts(root, &omitted);
        assert!(contracts
            .iter()
            .any(|contract| contract.starts_with("evidence.gate.format.step_entry_count")));

        let mut wrong_command = index.clone();
        wrong_command
            .entries
            .iter_mut()
            .find(|entry| entry.role == "step.stdout")
            .unwrap()
            .producing_command = vec!["cargo".into(), "fmt".into()];
        let contracts = production_contracts(root, &wrong_command);
        assert!(contracts
            .iter()
            .any(|contract| contract.starts_with("evidence.step.format.fmt-check.step.stdout")));

        let forged_bytes = serde_json::to_vec(&serde_json::json!({
            "schema": "uqm-s4-step-result-v2", "gate": "format", "step": "fmt-check",
            "command": step.command, "effective_command": step.command,
            "staged_script_sha256": null, "executable_identity": fixture_executable_identity(None),
            "exit_code": 0, "signal": null, "launch_error": null, "success": false,
            "supervision": fixture_supervision(false, 0, 0),
        }))
        .unwrap();
        fs::write(root.join("format/fmt-check.result.json"), &forged_bytes).unwrap();
        let mut forged = index.clone();
        let forged_entry = forged
            .entries
            .iter_mut()
            .find(|entry| entry.role == "step.result")
            .unwrap();
        forged_entry.byte_length = forged_bytes.len() as u64;
        forged_entry.sha256 = hex_sha256(&forged_bytes);
        let contracts = production_contracts(root, &forged);
        assert!(contracts
            .iter()
            .any(|contract| contract == "evidence.step.format.fmt-check.result_content"));

        let failed_step_bytes = serde_json::to_vec(&serde_json::json!({
            "schema": "uqm-s4-step-result-v2", "gate": "format", "step": "fmt-check",
            "command": step.command, "effective_command": step.command,
            "staged_script_sha256": null, "executable_identity": fixture_executable_identity(None),
            "exit_code": 1, "signal": null, "launch_error": null, "success": false,
            "supervision": fixture_supervision(false, 0, 0),
        }))
        .unwrap();
        fs::write(
            root.join("format/fmt-check.result.json"),
            &failed_step_bytes,
        )
        .unwrap();
        let failed_gate_bytes = serde_json::to_vec(&serde_json::json!({
            "schema": "uqm-s4-gate-result-v1", "gate": "format", "owner": "S4",
            "kind": "process", "passed": false,
            "first_failed_contract": "format.fmt-check", "detail": "failed",
            "controller_command": controller
        }))
        .unwrap();
        fs::write(root.join("format/gate.result.json"), &failed_gate_bytes).unwrap();
        let mut failed = index.clone();
        failed.first_failed_contract = Some("format.fmt-check".into());
        for (role, bytes) in [
            ("step.result", failed_step_bytes.as_slice()),
            ("gate.result", failed_gate_bytes.as_slice()),
        ] {
            let entry = failed
                .entries
                .iter_mut()
                .find(|entry| entry.role == role)
                .unwrap();
            entry.byte_length = bytes.len() as u64;
            entry.sha256 = hex_sha256(bytes);
        }
        assert_eq!(production_contracts(root, &failed), Vec::<String>::new());
        let mut missing_gate_result = failed.clone();
        missing_gate_result
            .entries
            .retain(|entry| entry.role != "gate.result");
        assert!(production_contracts(root, &missing_gate_result)
            .iter()
            .any(|contract| contract == "evidence.first_failed_contract.missing_gate_result"));
    }

    #[test]
    fn production_validation_rejects_a_tampered_authority_snapshot() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("authority.json");
        fs::write(&path, b"{\"schema\":\"forged\"}").unwrap();
        let mut index = valid_index();
        index.entries = vec![EvidenceEntry {
            schema: ENTRY_SCHEMA.to_string(),
            role: "authority.snapshot".to_string(),
            path: "authority.json".to_string(),
            mime: "application/json".to_string(),
            byte_length: fs::metadata(&path).unwrap().len(),
            sha256: hex_sha256(&fs::read(&path).unwrap()),
            producing_gate: "format".to_string(),
            producing_command: vec!["uqm-xtask".into(), "ci".into(), "run".into(), "all".into()],
        }];
        let contracts = validate_authority_snapshot(temp.path(), &index);
        assert!(contracts
            .iter()
            .any(|contract| contract.starts_with("evidence.authority_snapshot.json")));
    }

    #[test]
    fn valid_index_passes_offline_validation() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("artifact.log"), b"hello").unwrap();
        let index = valid_index();
        let contracts = validate_index(temp.path(), &fixture_tuples(), &index).unwrap();
        assert_eq!(contracts, Vec::<String>::new());
    }

    #[cfg(unix)]
    #[test]
    fn indexed_symlink_is_rejected_without_reading_its_target() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("target.log"), b"hello").unwrap();
        symlink("target.log", temp.path().join("artifact.log")).unwrap();
        let contracts = validate_index(temp.path(), &fixture_tuples(), &valid_index()).unwrap();
        assert!(contracts
            .iter()
            .any(|contract| contract.starts_with("evidence.entry.artifact.log.missing")));
    }

    #[cfg(unix)]
    #[test]
    fn bundle_read_never_follows_a_raced_intermediate_symlink() {
        use std::os::unix::fs::symlink;
        use std::sync::{Arc, Barrier};

        let temp = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let member = temp.path().join("member");
        let held = temp.path().join("member-held");
        fs::create_dir(&member).unwrap();
        fs::write(member.join("payload"), b"retained").unwrap();
        fs::write(outside.path().join("payload"), b"outside").unwrap();
        let barrier = Arc::new(Barrier::new(2));
        let writer_barrier = Arc::clone(&barrier);
        let root = temp.path().to_path_buf();
        let outside_root = outside.path().to_path_buf();
        let writer = std::thread::spawn(move || {
            writer_barrier.wait();
            for _ in 0..1_000 {
                fs::rename(root.join("member"), root.join("member-held")).unwrap();
                symlink(&outside_root, root.join("member")).unwrap();
                fs::remove_file(root.join("member")).unwrap();
                fs::rename(root.join("member-held"), root.join("member")).unwrap();
            }
        });
        barrier.wait();
        for _ in 0..10_000 {
            if let Ok(bytes) = read_bundle_file(temp.path(), "member/payload") {
                assert_eq!(bytes, b"retained");
            }
        }
        writer.join().unwrap();
        assert!(!held.exists());
    }

    #[cfg(unix)]
    #[test]
    fn inventory_never_reads_through_a_swapped_directory() {
        use std::os::unix::fs::symlink;
        use std::sync::{Arc, Barrier};

        let temp = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let member = temp.path().join("member");
        fs::create_dir(&member).unwrap();
        fs::write(member.join("payload"), b"retained").unwrap();
        fs::write(outside.path().join("payload"), b"outside").unwrap();
        let barrier = Arc::new(Barrier::new(2));
        let writer_barrier = Arc::clone(&barrier);
        let root = temp.path().to_path_buf();
        let outside_root = outside.path().to_path_buf();
        let writer = std::thread::spawn(move || {
            writer_barrier.wait();
            for _ in 0..1_000 {
                fs::rename(root.join("member"), root.join("member-held")).unwrap();
                symlink(&outside_root, root.join("member")).unwrap();
                fs::remove_file(root.join("member")).unwrap();
                fs::rename(root.join("member-held"), root.join("member")).unwrap();
            }
        });
        barrier.wait();
        for _ in 0..1_000 {
            if let Ok(files) = regular_file_inventory(temp.path()) {
                assert!(files.iter().all(|file| file.bytes != b"outside"));
            }
        }
        writer.join().unwrap();
    }

    #[test]
    fn wrong_tuple_is_a_contract_failure() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("artifact.log"), b"hello").unwrap();
        let mut index = valid_index();
        index.tuple = "windows-amd64".into();
        let contracts = validate_index(temp.path(), &fixture_tuples(), &index).unwrap();
        assert!(contracts
            .iter()
            .any(|item| item.starts_with("evidence.tuple")));
    }

    #[test]
    fn missing_evidence_file_is_a_contract_failure() {
        let temp = tempfile::tempdir().unwrap();
        let index = valid_index();
        let contracts = validate_index(temp.path(), &fixture_tuples(), &index).unwrap();
        assert!(contracts.iter().any(|item| item.contains(".missing")));
    }

    #[test]
    fn tampered_payload_is_detected() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("artifact.log"), b"tampered!").unwrap();
        let index = valid_index();
        let contracts = validate_index(temp.path(), &fixture_tuples(), &index).unwrap();
        assert!(contracts
            .iter()
            .any(|item| item.ends_with(".byte_length") || item.ends_with(".sha256_mismatch")));
    }

    #[test]
    fn badge_schema_and_gate_and_cache_mode_are_checked() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("artifact.log"), b"hello").unwrap();
        let mut index = valid_index();
        retain_fixture_authority(temp.path(), &mut index);
        index.schema = "wrong".into();
        index.cache_mode = "leaky".into();
        index.entries[0].producing_gate = "nope".into();
        let contracts = validate_index(temp.path(), &fixture_tuples(), &index).unwrap();
        assert!(contracts
            .iter()
            .any(|item| item.starts_with("evidence.schema")));
        assert!(contracts
            .iter()
            .any(|item| item.contains("evidence.cache_mode")));
        assert!(contracts
            .iter()
            .any(|item| item.contains(".producing_gate")));
    }

    #[test]
    fn unsupported_role_mime_pair_is_rejected() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("artifact.log"), b"hello").unwrap();
        let mut index = valid_index();
        retain_fixture_authority(temp.path(), &mut index);
        index.entries[0].mime = "application/json".into();
        let contracts = validate_index(temp.path(), &fixture_tuples(), &index).unwrap();
        assert!(contracts
            .iter()
            .any(|item| item.ends_with(".role_mime_contract")));
    }

    #[test]
    fn entry_round_trips_bytes_but_detects_forgery() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("payload.bin");
        fs::write(&path, b"evidence payload").unwrap();
        let made = entry(
            temp.path(),
            "payload.bin",
            "step.stdout",
            "text/plain",
            "format",
            &[
                "uqm-xtask".into(),
                "ci".into(),
                "run".into(),
                "format".into(),
            ],
        )
        .unwrap();
        assert!(validate_bytes(&made, b"evidence payload").is_ok());
        assert!(validate_bytes(&made, b"forged").is_err());
    }

    #[test]
    fn relative_path_rules_reject_traversal_and_absolutes() {
        assert!(validate_relative_path("rust/target/ci/log.txt"));
        assert!(!validate_relative_path("/etc/passwd"));
        assert!(!validate_relative_path("../secret"));
        assert!(!validate_relative_path("a/../../b"));
        assert!(!validate_relative_path(""));
        assert!(!validate_relative_path("a\\b"));
        assert!(!validate_relative_path("dir/"));
    }

    fn transport_file(root: &Path, path: &str) -> TransportFile {
        let bytes = fs::read(root.join(path)).unwrap();
        TransportFile {
            path: path.to_string(),
            byte_length: bytes.len() as u64,
            sha256: hex_sha256(&bytes),
        }
    }

    fn add_successful_workflow_receipts(root: &Path, index: &mut TransportIndex, names: &[&str]) {
        let (descendant_tracking_scope, descendant_containment_ceiling) = if index
            .tuple
            .as_deref()
            .is_some_and(|tuple| tuple.starts_with("macos-"))
        {
            (
                    "observed-descendant-tree",
                    "darwin has no child subreaper: a descendant that detaches and whose ancestors all exit before any supervisor observation passes is outside this tree; every observed escaped descendant is stopped, re-verified against its kernel start identity while stopped, and only then signaled, so an unrelated reused pid is at worst briefly stopped and resumed, never killed, while descendant discovery itself remains observational",
                )
        } else {
            (
                    "child-subreaper-descendant-tree",
                    "the kernel reparents every orphaned descendant to this supervisor, so a detached descendant remains a tracked and reapable child until it exits",
                )
        };
        for name in names {
            fs::write(
                root.join(name),
                serde_json::to_vec_pretty(&serde_json::json!({
                    "schema": "uqm-s4-workflow-subprocess-v1",
                    "command": ["/usr/bin/true"],
                    "executable_identity": {
                        "path": "/usr/bin/true",
                        "byte_length": 1,
                        "sha256": "a".repeat(64),
                        "mode": 0o755
                    },
                    "exit_code": 0,
                    "launch_error": null,
                    "failure": null,
                    "stdout_bytes": 0,
                    "stderr_bytes": 0,
                    "process_group_empty": true,
                    "containment_scope": "initial-process-group",
                    "descendant_tracking_scope": descendant_tracking_scope,
                    "descendant_containment_ceiling": descendant_containment_ceiling,
                    "descendants_observed": 0,
                    "escaped_descendants_observed": 0,
                    "descendants_terminated": true,
                    "descendant_signals": [],
                    "signals": [],
                    "last_signal_monotonic_milliseconds": null,
                    "last_signal_monotonic_nanoseconds": null,
                    "leader_unpinned_monotonic_milliseconds": 1,
                    "leader_unpinned_monotonic_nanoseconds": 1_000_000,
                    "pgid_pinned_through_last_signal": true,
                    "elapsed_milliseconds": 1
                }))
                .unwrap(),
            )
            .unwrap();
            index.files.push(transport_file(root, name));
        }
        index
            .files
            .sort_by(|left, right| left.path.cmp(&right.path));
    }

    fn add_failed_workflow_receipt(root: &Path, index: &mut TransportIndex, name: &str) {
        let (descendant_tracking_scope, descendant_containment_ceiling) = if index
            .tuple
            .as_deref()
            .is_some_and(|tuple| tuple.starts_with("macos-"))
        {
            (
                    "observed-descendant-tree",
                    "darwin has no child subreaper: a descendant that detaches and whose ancestors all exit before any supervisor observation passes is outside this tree; every observed escaped descendant is stopped, re-verified against its kernel start identity while stopped, and only then signaled, so an unrelated reused pid is at worst briefly stopped and resumed, never killed, while descendant discovery itself remains observational",
                )
        } else {
            (
                    "child-subreaper-descendant-tree",
                    "the kernel reparents every orphaned descendant to this supervisor, so a detached descendant remains a tracked and reapable child until it exits",
                )
        };
        fs::write(
            root.join(name),
            serde_json::to_vec_pretty(&serde_json::json!({
                "schema": "uqm-s4-workflow-subprocess-v1",
                "command": ["/usr/bin/false"],
                "executable_identity": {
                    "path": "/usr/bin/false",
                    "byte_length": 1,
                    "sha256": "b".repeat(64),
                    "mode": 0o755
                },
                "exit_code": 1,
                "launch_error": null,
                "failure": null,
                "stdout_bytes": 0,
                "stderr_bytes": 0,
                "process_group_empty": true,
                "containment_scope": "initial-process-group",
                "descendant_tracking_scope": descendant_tracking_scope,
                "descendant_containment_ceiling": descendant_containment_ceiling,
                "descendants_observed": 0,
                "escaped_descendants_observed": 0,
                "descendants_terminated": true,
                "descendant_signals": [],
                "signals": [],
                "last_signal_monotonic_milliseconds": null,
                "last_signal_monotonic_nanoseconds": null,
                "leader_unpinned_monotonic_milliseconds": 1,
                "leader_unpinned_monotonic_nanoseconds": 1_000_000,
                "pgid_pinned_through_last_signal": true,
                "elapsed_milliseconds": 1
            }))
            .unwrap(),
        )
        .unwrap();
        index.files.push(transport_file(root, name));
        index
            .files
            .sort_by(|left, right| left.path.cmp(&right.path));
    }

    #[test]
    fn workflow_containment_receipt_rejects_missing_or_forged_descendant_facts() {
        let valid = serde_json::json!({
            "containment_scope": "initial-process-group",
            "pgid_pinned_through_last_signal": true,
            "signals": [],
            "last_signal_monotonic_milliseconds": null,
            "last_signal_monotonic_nanoseconds": null,
            "leader_unpinned_monotonic_milliseconds": 1,
            "leader_unpinned_monotonic_nanoseconds": 1_000_000,
            "descendant_tracking_scope": "child-subreaper-descendant-tree",
            "descendant_containment_ceiling": "the kernel reparents every orphaned descendant to this supervisor, so a detached descendant remains a tracked and reapable child until it exits",
            "descendants_observed": 1,
            "escaped_descendants_observed": 1,
            "descendants_terminated": true,
            "descendant_signals": [{
                "sequence": 0,
                "pid": 42,
                "signal": "SIGKILL",
                "monotonic_milliseconds": 1,
                "monotonic_nanoseconds": 1_000_000,
                "result": "delivered",
                "start_identity": 17
            }]
        });
        assert!(valid_workflow_containment_receipt(&valid, true));

        assert!(valid_workflow_containment_receipt_for_tuple(
            &valid,
            true,
            Some("linux-x86_64")
        ));
        assert!(!valid_workflow_containment_receipt_for_tuple(
            &valid,
            true,
            Some("macos-aarch64")
        ));
        for field in [
            "descendant_tracking_scope",
            "descendant_containment_ceiling",
            "descendants_observed",
            "escaped_descendants_observed",
            "descendants_terminated",
            "descendant_signals",
        ] {
            let mut mutated = valid.clone();
            mutated.as_object_mut().unwrap().remove(field);
            assert!(!valid_workflow_containment_receipt(&mutated, true));
        }

        let mut escaped = valid.clone();
        escaped["escaped_descendants_observed"] = serde_json::json!(2);
        assert!(!valid_workflow_containment_receipt(&escaped, true));
        let mut identity = valid.clone();
        identity["descendant_signals"][0]["start_identity"] = serde_json::json!([1, 2]);
        assert!(!valid_workflow_containment_receipt(&identity, true));
        let mut sequence = valid.clone();
        sequence["descendant_signals"][0]["sequence"] = serde_json::json!(1);
        assert!(!valid_workflow_containment_receipt(&sequence, true));

        let mut same_tick = valid.clone();
        same_tick["signals"] = serde_json::json!([
            {
                "sequence": 0,
                "signal": "SIGTERM",
                "monotonic_milliseconds": 1,
                "monotonic_nanoseconds": 1_000_000,
                "result": "delivered"
            },
            {
                "sequence": 1,
                "signal": "SIGCONT",
                "monotonic_milliseconds": 1,
                "monotonic_nanoseconds": 1_000_000,
                "result": "delivered"
            }
        ]);
        same_tick["last_signal_monotonic_milliseconds"] = serde_json::json!(1);
        same_tick["last_signal_monotonic_nanoseconds"] = serde_json::json!(1_000_000);
        same_tick["leader_unpinned_monotonic_milliseconds"] = serde_json::json!(1);
        same_tick["leader_unpinned_monotonic_nanoseconds"] = serde_json::json!(1_000_001);
        assert!(valid_workflow_containment_receipt(&same_tick, true));
        same_tick["signals"][1]["sequence"] = serde_json::json!(0);
        assert!(!valid_workflow_containment_receipt(&same_tick, true));
        let mut wrong_platform_result = valid.clone();
        wrong_platform_result["descendant_signals"][0]["result"] =
            serde_json::json!("uncontainable-platform-ceiling");
        assert!(!valid_workflow_containment_receipt(
            &wrong_platform_result,
            true
        ));

        let mut pidfd_without_errno = valid.clone();
        pidfd_without_errno["descendant_signals"][0]["result"] = serde_json::json!("pidfd-error");
        assert!(!valid_workflow_containment_receipt(
            &pidfd_without_errno,
            true
        ));
        pidfd_without_errno["descendant_signals"][0]["errno"] = serde_json::json!(9);
        pidfd_without_errno["descendants_terminated"] = serde_json::json!(false);
        assert!(valid_workflow_containment_receipt(
            &pidfd_without_errno,
            true
        ));

        let mut darwin = valid.clone();
        darwin["descendant_tracking_scope"] = serde_json::json!("observed-descendant-tree");
        darwin["descendant_containment_ceiling"] = serde_json::json!("darwin has no child subreaper: a descendant that detaches and whose ancestors all exit before any supervisor observation passes is outside this tree; every observed escaped descendant is stopped, re-verified against its kernel start identity while stopped, and only then signaled, so an unrelated reused pid is at worst briefly stopped and resumed, never killed, while descendant discovery itself remains observational");
        darwin["descendant_signals"][0]["start_identity"] = serde_json::json!([1, 2]);
        darwin["descendant_signals"][0]["result"] = serde_json::json!("delivered");
        assert!(valid_workflow_containment_receipt(&darwin, true));
        assert!(valid_workflow_containment_receipt_for_tuple(
            &darwin,
            true,
            Some("macos-aarch64")
        ));
        assert!(!valid_workflow_containment_receipt_for_tuple(
            &darwin,
            true,
            Some("linux-x86_64")
        ));
        darwin["descendant_signals"][0]["result"] =
            serde_json::json!("uncontainable-platform-ceiling");
        assert!(!valid_workflow_containment_receipt(&darwin, true));
        darwin["descendant_signals"][0]["result"] = serde_json::json!("pidfd-error");
        assert!(!valid_workflow_containment_receipt(&darwin, true));
        darwin["descendant_signals"][0]["result"] = serde_json::json!("signal-error");
        assert!(!valid_workflow_containment_receipt(&darwin, true));
        darwin["descendant_signals"][0]["errno"] = serde_json::json!(9);
        darwin["descendants_terminated"] = serde_json::json!(false);
        assert!(valid_workflow_containment_receipt(&darwin, true));
        darwin["descendant_signals"][0]["result"] = serde_json::json!("identity-changed");
        darwin["descendant_signals"][0]
            .as_object_mut()
            .unwrap()
            .remove("errno");
        assert!(valid_workflow_containment_receipt(&darwin, true));
        let mut terminated = valid;
        terminated["descendants_terminated"] = serde_json::json!(false);
        assert!(!valid_workflow_containment_receipt(&terminated, true));
    }

    fn add_gate_setup_receipts(root: &Path, index: &mut TransportIndex) {
        add_successful_workflow_receipts(
            root,
            index,
            &[
                "prerequisites-brew.result.json",
                "tools-rustup.result.json",
                "tools-venv.result.json",
                "tools-lizard.result.json",
                "tools-cargo-audit.result.json",
                "tools-cargo-llvm-cov.result.json",
                "tools-actionlint.result.json",
                "tools-component-rustfmt.result.json",
                "tools-component-clippy.result.json",
                "tools-component-llvm-tools-preview.result.json",
                "native-content.result.json",
                "xtask-build.result.json",
                "source-revalidation.result.json",
            ],
        );
        add_successful_workflow_receipts(root, index, &["containment-check.result.json"]);
    }
    #[test]
    fn failed_workflow_prefixes_require_causal_failed_receipts() {
        let authority: Authority =
            serde_json::from_slice(include_bytes!("../../../ci/gates.json")).unwrap();
        for (job, step, receipt, expected) in [
            (
                "plan",
                "plan",
                "ci-plan.result.json",
                "transport.plan.failed_subprocess_receipt",
            ),
            (
                "gates",
                "prerequisites",
                "prerequisites-brew.result.json",
                "transport.gates.failed_prerequisite_receipt",
            ),
            (
                "gates",
                "tools",
                "tools-rustup.result.json",
                "transport.gates.failed_tool_receipt",
            ),
            (
                "gates",
                "native-content",
                "native-content.result.json",
                "transport.gates.failed_native_content_receipt",
            ),
            (
                "gates",
                "containment-check",
                "containment-check.result.json",
                "transport.gates.failed_uid_containment_receipt",
            ),
            (
                "gates",
                "xtask-build",
                "xtask-build.result.json",
                "transport.gates.failed_xtask_build_receipt",
            ),
            (
                "gates",
                "authoritative-gates",
                "ci-run.result.json",
                "transport.gates.failed_ci_run_receipt",
            ),
            (
                "gates",
                "source-revalidation",
                "source-revalidation.result.json",
                "transport.gates.failed_source_revalidation_receipt",
            ),
        ] {
            let temporary = tempfile::tempdir().unwrap();
            let mut index = TransportIndex {
                schema: TRANSPORT_SCHEMA.to_string(),
                job: job.to_string(),
                source_sha: "a".repeat(40),
                tuple: (job == "gates").then(|| "macos-aarch64".to_string()),
                exit_code: None,
                job_status: Some("failure".to_string()),
                files: Vec::new(),
            };
            let setup = WorkflowSetupResults {
                schema: WORKFLOW_SETUP_SCHEMA.to_string(),
                job: job.to_string(),
                source_sha: "a".repeat(40),
                tuple: index.tuple.clone(),
                steps: vec![WorkflowSetupStep {
                    step: step.to_string(),
                    outcome: "failure".to_string(),
                }],
            };
            let mut contracts = Vec::new();
            validate_workflow_subprocess_receipts(
                temporary.path(),
                &index,
                job,
                &setup,
                Some(&authority),
                &mut contracts,
            );
            assert!(
                contracts.contains(&expected.to_string()),
                "{step}: {contracts:?}"
            );

            add_successful_workflow_receipts(temporary.path(), &mut index, &[receipt]);
            let mut forged_contracts = Vec::new();
            validate_workflow_subprocess_receipts(
                temporary.path(),
                &index,
                job,
                &setup,
                Some(&authority),
                &mut forged_contracts,
            );
            assert!(
                forged_contracts.contains(&expected.to_string()),
                "{step}: {forged_contracts:?}"
            );
        }
    }

    #[test]
    fn successful_gates_require_dedicated_uid_containment_receipt() {
        let temporary = tempfile::tempdir().unwrap();
        let authority: Authority =
            serde_json::from_slice(include_bytes!("../../../ci/gates.json")).unwrap();
        let mut index = TransportIndex {
            schema: TRANSPORT_SCHEMA.to_string(),
            job: "gates".to_string(),
            source_sha: "a".repeat(40),
            tuple: Some("linux-x86_64".to_string()),
            exit_code: None,
            job_status: Some("success".to_string()),
            files: Vec::new(),
        };
        let setup = WorkflowSetupResults {
            schema: WORKFLOW_SETUP_SCHEMA.to_string(),
            job: "gates".to_string(),
            source_sha: index.source_sha.clone(),
            tuple: index.tuple.clone(),
            steps: vec![WorkflowSetupStep {
                step: "authoritative-gates".to_string(),
                outcome: "success".to_string(),
            }],
        };
        add_successful_workflow_receipts(
            temporary.path(),
            &mut index,
            &["xtask-build.result.json", "ci-run.result.json"],
        );
        let validate = |index: &TransportIndex| {
            let mut contracts = Vec::new();
            validate_workflow_subprocess_receipts(
                temporary.path(),
                index,
                "gates",
                &setup,
                Some(&authority),
                &mut contracts,
            );
            contracts
        };
        assert!(validate(&index).contains(&"transport.gates.uid_containment_receipt".to_string()));
        add_successful_workflow_receipts(
            temporary.path(),
            &mut index,
            &["containment-check.result.json"],
        );
        assert!(!validate(&index).contains(&"transport.gates.uid_containment_receipt".to_string()));
    }

    #[test]
    fn workflow_transport_validates_without_a_repository_and_rejects_forged_outcomes() {
        let temp = tempfile::tempdir().unwrap();

        fs::write(
            temp.path().join("authority-snapshot.json"),
            include_bytes!("../../../ci/gates.json"),
        )
        .unwrap();
        fs::write(
            temp.path().join("workflow-setup-results.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "schema": WORKFLOW_SETUP_SCHEMA,
                "job": "plan",
                "source_sha": "a".repeat(40),
                "tuple": null,
                "steps": [
                    {"step": "plan-build", "outcome": "success"},
                    {"step": "checkout", "outcome": "success"},
                    {"step": "plan", "outcome": "success"}
                ]
            }))
            .unwrap(),
        )
        .unwrap();
        let authority: Authority =
            serde_json::from_slice(include_bytes!("../../../ci/gates.json")).unwrap();
        let plan = super::super::plan::Plan {
            schema: super::super::plan::PLAN_SCHEMA.to_string(),
            authority: super::super::authority::AUTHORITY_RELATIVE.to_string(),
            authority_contract: Some(serde_json::to_value(&authority).unwrap()),
            tuples: authority
                .runner_mapping
                .iter()
                .map(|mapping| {
                    let (os, architecture) = mapping.tuple.split_once('-').unwrap();
                    super::super::plan::PlanTuple {
                        os: os.to_string(),
                        architecture: architecture.to_string(),
                        tuple: mapping.tuple.clone(),
                        runner: mapping.runner.clone(),
                        expected_uname: mapping.expected_uname.clone(),
                    }
                })
                .collect(),
        };
        fs::write(
            temp.path().join("ci-plan.json"),
            serde_json::to_vec_pretty(&plan).unwrap(),
        )
        .unwrap();
        fs::write(temp.path().join("ci-plan.stderr.log"), b"").unwrap();
        let mut index = TransportIndex {
            schema: TRANSPORT_SCHEMA.to_string(),
            job: "plan".to_string(),
            source_sha: "a".repeat(40),
            tuple: None,
            exit_code: None,
            job_status: Some("success".to_string()),
            files: vec![
                transport_file(temp.path(), "authority-snapshot.json"),
                transport_file(temp.path(), "ci-plan.json"),
                transport_file(temp.path(), "ci-plan.stderr.log"),
                transport_file(temp.path(), "workflow-setup-results.json"),
            ],
        };
        add_successful_workflow_receipts(
            temp.path(),
            &mut index,
            &[
                "bootstrap-apt-update.result.json",
                "bootstrap-apt-install.result.json",
                "bootstrap-rustup.result.json",
                "bootstrap-xtask-build.result.json",
                "ci-plan.result.json",
            ],
        );
        let index_path = temp.path().join("index.json");
        fs::write(&index_path, serde_json::to_vec_pretty(&index).unwrap()).unwrap();
        validate_evidence_command(
            Path::new("/definitely-not-a-repository"),
            index_path.to_str().unwrap(),
        )
        .unwrap();

        let mut bad_hash = index.clone();
        bad_hash.files[0].sha256 = "0".repeat(64);
        assert!(validate_transport_index(temp.path(), &bad_hash)
            .iter()
            .any(|contract| contract.ends_with(".sha256")));

        let mut bad_size = index.clone();
        bad_size.files[0].byte_length += 1;
        assert!(validate_transport_index(temp.path(), &bad_size)
            .iter()
            .any(|contract| contract.ends_with(".byte_length")));

        let mut bad_order = index.clone();
        bad_order.files.swap(0, 1);
        assert!(validate_transport_index(temp.path(), &bad_order)
            .contains(&"transport.files.order_or_path".to_string()));

        let mut bad_source = index.clone();
        bad_source.source_sha = "not-a-sha".to_string();
        assert!(validate_transport_index(temp.path(), &bad_source)
            .contains(&"transport.source_sha".to_string()));

        let mut bad_job = index.clone();
        bad_job.job = "forged".to_string();
        assert!(
            validate_transport_index(temp.path(), &bad_job).contains(&"transport.job".to_string())
        );

        let mut missing_file = index.clone();
        missing_file.files[0].path = "missing.json".to_string();
        assert!(validate_transport_index(temp.path(), &missing_file)
            .iter()
            .any(|contract| contract.starts_with("transport.file.missing.json.read")));
        fs::write(temp.path().join("authority-snapshot.json"), b"{}").unwrap();
        let mut malformed_authority = index.clone();
        malformed_authority.files[0] = transport_file(temp.path(), "authority-snapshot.json");
        assert!(validate_transport_index(temp.path(), &malformed_authority)
            .contains(&"transport.plan.authority".to_string()));
        fs::write(
            temp.path().join("authority-snapshot.json"),
            include_bytes!("../../../ci/gates.json"),
        )
        .unwrap();
        index.files[0] = transport_file(temp.path(), "authority-snapshot.json");

        fs::write(temp.path().join("unindexed.txt"), b"unindexed").unwrap();
        assert!(validate_transport_index(temp.path(), &index)
            .contains(&"transport.files.completeness".to_string()));
        fs::remove_file(temp.path().join("unindexed.txt")).unwrap();

        let mut unknown_field = serde_json::to_value(&index).unwrap();

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(
                temp.path().join("authority-snapshot.json"),
                temp.path().join("symlinked-authority.json"),
            )
            .unwrap();
            assert!(validate_transport_index(temp.path(), &index)
                .contains(&"transport.files.symlink".to_string()));
            fs::remove_file(temp.path().join("symlinked-authority.json")).unwrap();
        }
        unknown_field["forged"] = serde_json::json!(true);
        fs::write(
            &index_path,
            serde_json::to_vec_pretty(&unknown_field).unwrap(),
        )
        .unwrap();
        assert!(
            validate_evidence_command(Path::new("/unused"), index_path.to_str().unwrap())
                .unwrap_err()
                .contains("unknown field")
        );
        fs::write(&index_path, serde_json::to_vec_pretty(&index).unwrap()).unwrap();

        let mut forged_plan = serde_json::to_value(&plan).unwrap();
        forged_plan["tuples"][0]["runner"] = serde_json::json!("ubuntu-latest");
        fs::write(
            temp.path().join("ci-plan.json"),
            serde_json::to_vec_pretty(&forged_plan).unwrap(),
        )
        .unwrap();
        *index
            .files
            .iter_mut()
            .find(|entry| entry.path == "ci-plan.json")
            .unwrap() = transport_file(temp.path(), "ci-plan.json");
        assert!(validate_transport_index(temp.path(), &index)
            .contains(&"transport.plan.payload".to_string()));
        fs::write(
            temp.path().join("ci-plan.json"),
            serde_json::to_vec_pretty(&plan).unwrap(),
        )
        .unwrap();
        *index
            .files
            .iter_mut()
            .find(|entry| entry.path == "ci-plan.json")
            .unwrap() = transport_file(temp.path(), "ci-plan.json");

        fs::write(
            temp.path().join("workflow-setup-results.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "schema": WORKFLOW_SETUP_SCHEMA,
                "job": "plan",
                "source_sha": "a".repeat(40),
                "tuple": null,
                "steps": [
                    {"step": "plan-build", "outcome": "success"},
                    {"step": "checkout", "outcome": "failure"},
                    {"step": "plan", "outcome": "success"}
                ]
            }))
            .unwrap(),
        )
        .unwrap();
        *index
            .files
            .iter_mut()
            .find(|entry| entry.path == "workflow-setup-results.json")
            .unwrap() = transport_file(temp.path(), "workflow-setup-results.json");
        fs::write(&index_path, serde_json::to_vec_pretty(&index).unwrap()).unwrap();
        let error = validate_evidence_command(Path::new("/unused"), index_path.to_str().unwrap())
            .unwrap_err();
        assert!(error.contains("transport.plan.setup_prefix"));
    }

    #[test]
    fn cancelled_gate_transport_validates_without_a_nested_child_bundle() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(
            temp.path().join("authority-snapshot.json"),
            include_bytes!("../../../ci/gates.json"),
        )
        .unwrap();

        fs::write(
            temp.path().join("workflow-setup-results.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "schema": WORKFLOW_SETUP_SCHEMA,
                "job": "gates",
                "source_sha": "a".repeat(40),
                "tuple": "macos-aarch64",
                "steps": [
                    {"step": "xtask-build", "outcome": "success"},
                    {"step": "checkout", "outcome": "success"},
                    {"step": "architecture", "outcome": "success"},
                    {"step": "prerequisites", "outcome": "success"},
                    {"step": "tools", "outcome": "success"},
                    {"step": "native-content", "outcome": "success"},
                    {"step": "containment-check", "outcome": "success"},
                    {"step": "authoritative-gates", "outcome": "cancelled"},
                    {"step": "source-revalidation", "outcome": "success"}
                ]
            }))
            .unwrap(),
        )
        .unwrap();
        let mut index = TransportIndex {
            schema: TRANSPORT_SCHEMA.to_string(),
            job: "gates".to_string(),
            source_sha: "a".repeat(40),
            tuple: Some("macos-aarch64".to_string()),
            exit_code: None,
            job_status: Some("cancelled".to_string()),
            files: vec![
                transport_file(temp.path(), "authority-snapshot.json"),
                transport_file(temp.path(), "workflow-setup-results.json"),
            ],
        };
        add_gate_setup_receipts(temp.path(), &mut index);
        let contracts = validate_transport_index(temp.path(), &index);
        assert!(contracts.is_empty(), "{contracts:?}");

        index
            .files
            .retain(|file| file.path != "source-revalidation.result.json");
        let contracts = validate_transport_index(temp.path(), &index);
        assert!(contracts.contains(&"transport.gates.source_revalidation_receipt".to_string()));
    }

    #[test]
    fn failed_gate_transport_replays_a_nested_normal_all_gates_bundle() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(
            temp.path().join("authority-snapshot.json"),
            include_bytes!("../../../ci/gates.json"),
        )
        .unwrap();
        fs::write(
            temp.path().join("workflow-setup-results.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "schema": WORKFLOW_SETUP_SCHEMA,
                "job": "gates",
                "source_sha": "a".repeat(40),
                "tuple": "linux-x86_64",
                "steps": [
                    {"step": "xtask-build", "outcome": "success"},
                    {"step": "checkout", "outcome": "success"},
                    {"step": "architecture", "outcome": "success"},
                    {"step": "prerequisites", "outcome": "success"},
                    {"step": "tools", "outcome": "success"},
                    {"step": "native-content", "outcome": "success"},
                    {"step": "containment-check", "outcome": "success"},
                    {"step": "authoritative-gates", "outcome": "failure"},
                    {"step": "source-revalidation", "outcome": "success"}
                ]
            }))
            .unwrap(),
        )
        .unwrap();
        let nested_root = temp.path().join("run-0");
        fs::create_dir(&nested_root).unwrap();
        let nested_index = all_gates_preflight_failure_bundle(&nested_root, "source.expected_sha");
        fs::write(
            nested_root.join(INDEX_FILENAME),
            serde_json::to_vec_pretty(&nested_index).unwrap(),
        )
        .unwrap();
        let mut paths = Vec::new();
        let mut walk_contracts = Vec::new();
        collect_transport_paths(temp.path(), temp.path(), &mut paths, &mut walk_contracts);
        assert!(walk_contracts.is_empty());
        paths.sort();
        let mut index = TransportIndex {
            schema: TRANSPORT_SCHEMA.to_string(),
            job: "gates".to_string(),
            source_sha: "a".repeat(40),
            tuple: Some("linux-x86_64".to_string()),
            exit_code: None,
            job_status: Some("failure".to_string()),
            files: paths
                .iter()
                .map(|path| transport_file(temp.path(), path))
                .collect(),
        };
        add_gate_setup_receipts(temp.path(), &mut index);
        add_failed_workflow_receipt(temp.path(), &mut index, "ci-run.result.json");
        let contracts = validate_transport_index(temp.path(), &index);
        assert!(contracts.is_empty(), "{contracts:?}");

        let mut wrong_source = index.clone();
        wrong_source.source_sha = "b".repeat(40);
        let contracts = validate_transport_index(temp.path(), &wrong_source);
        assert!(contracts
            .iter()
            .any(|contract| contract.contains("nested evidence identity contradicts transport")));
    }

    #[test]
    fn failed_gate_transport_replays_and_correlates_nested_pre_session_evidence() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(
            temp.path().join("authority-snapshot.json"),
            include_bytes!("../../../ci/gates.json"),
        )
        .unwrap();
        fs::write(
            temp.path().join("workflow-setup-results.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "schema": WORKFLOW_SETUP_SCHEMA,
                "job": "gates",
                "source_sha": "a".repeat(40),
                "tuple": "macos-aarch64",
                "steps": [
                    {"step": "xtask-build", "outcome": "success"},
                    {"step": "checkout", "outcome": "success"},
                    {"step": "architecture", "outcome": "success"},
                    {"step": "prerequisites", "outcome": "success"},
                    {"step": "tools", "outcome": "success"},
                    {"step": "native-content", "outcome": "success"},
                    {"step": "containment-check", "outcome": "success"},
                    {"step": "authoritative-gates", "outcome": "failure"},
                    {"step": "source-revalidation", "outcome": "success"}
                ]
            }))
            .unwrap(),
        )
        .unwrap();
        let mut envelope = valid_pre_session_envelope("source.head");
        envelope.requested_gate = "all".to_string();
        envelope.controller_command[3] = "all".to_string();
        envelope.tuple = "macos-aarch64".to_string();
        envelope.offline_validation = pre_session_validation(&envelope);
        let nested = "pre-session-run-0/pre-session-failure.json";
        fs::create_dir(temp.path().join("pre-session-run-0")).unwrap();
        fs::write(
            temp.path().join(nested),
            serde_json::to_vec_pretty(&envelope).unwrap(),
        )
        .unwrap();
        let mut index = TransportIndex {
            schema: TRANSPORT_SCHEMA.to_string(),
            job: "gates".to_string(),
            source_sha: "a".repeat(40),
            tuple: Some("macos-aarch64".to_string()),
            exit_code: None,
            job_status: Some("failure".to_string()),
            files: vec![
                transport_file(temp.path(), "authority-snapshot.json"),
                transport_file(temp.path(), nested),
                transport_file(temp.path(), "workflow-setup-results.json"),
            ],
        };
        add_gate_setup_receipts(temp.path(), &mut index);
        add_failed_workflow_receipt(temp.path(), &mut index, "ci-run.result.json");
        let contracts = validate_transport_index(temp.path(), &index);
        assert!(contracts.is_empty(), "{contracts:?}");

        envelope.requested_gate = "format".to_string();
        envelope.controller_command[3] = "format".to_string();
        envelope.offline_validation = pre_session_validation(&envelope);
        fs::write(
            temp.path().join(nested),
            serde_json::to_vec_pretty(&envelope).unwrap(),
        )
        .unwrap();
        *index
            .files
            .iter_mut()
            .find(|entry| entry.path == nested)
            .unwrap() = transport_file(temp.path(), nested);
        assert!(validate_transport_index(temp.path(), &index)
            .iter()
            .any(|contract| contract.contains("did not request all gates")));

        envelope.requested_gate = "all".to_string();
        envelope.controller_command[3] = "all".to_string();
        envelope.tuple = "linux-x86_64".to_string();
        envelope.offline_validation = pre_session_validation(&envelope);
        fs::write(
            temp.path().join(nested),
            serde_json::to_vec_pretty(&envelope).unwrap(),
        )
        .unwrap();
        *index
            .files
            .iter_mut()
            .find(|entry| entry.path == nested)
            .unwrap() = transport_file(temp.path(), nested);
        let contracts = validate_transport_index(temp.path(), &index);
        assert!(contracts
            .iter()
            .any(|contract| contract.contains("nested pre-session tuple contradicts transport")));
    }

    #[test]
    fn upload_receipt_validates_transport_metadata_without_a_repository() {
        let temp = tempfile::tempdir().unwrap();
        let receipt_path = temp.path().join("upload-receipt.json");
        let mut receipt = serde_json::json!({
            "schema": UPLOAD_RECEIPT_SCHEMA,
            "job": "gates",
            "tuple": "macos-aarch64",
            "source_sha": "a".repeat(40),
            "artifact_name": "s4-macos-aarch64-123-1",
            "artifact_id": 42,
            "artifact_url": "https://github.com/owner/repo/actions/runs/123/artifacts/42",
            "artifact_digest": format!("sha256:{}", "b".repeat(64)),
            "retention_days": 30,
            "size_in_bytes": 1024,
            "upload_outcome": "success"
        });
        fs::write(&receipt_path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
        validate_evidence_command(
            Path::new("/definitely-not-a-repository"),
            receipt_path.to_str().unwrap(),
        )
        .unwrap();

        let valid_receipt = receipt.clone();

        receipt["artifact_digest"] = serde_json::json!("forged");
        fs::write(&receipt_path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
        assert!(
            validate_evidence_command(Path::new("/unused"), receipt_path.to_str().unwrap())
                .unwrap_err()
                .contains("upload-receipt.transport")
        );

        receipt = valid_receipt.clone();
        receipt["job"] = serde_json::json!("plan");
        fs::write(&receipt_path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
        assert!(
            validate_evidence_command(Path::new("/unused"), receipt_path.to_str().unwrap())
                .unwrap_err()
                .contains("upload-receipt.identity")
        );

        receipt = valid_receipt.clone();
        receipt["retention_days"] = serde_json::json!(29);
        fs::write(&receipt_path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
        assert!(
            validate_evidence_command(Path::new("/unused"), receipt_path.to_str().unwrap())
                .unwrap_err()
                .contains("upload-receipt.retention")
        );

        receipt = valid_receipt.clone();
        receipt["artifact_url"] =
            serde_json::json!("https://github.com/owner/repo/actions/runs/123/artifacts/43");
        fs::write(&receipt_path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
        assert!(
            validate_evidence_command(Path::new("/unused"), receipt_path.to_str().unwrap())
                .unwrap_err()
                .contains("upload-receipt.transport")
        );

        receipt = valid_receipt.clone();
        receipt["source_sha"] = serde_json::json!("not-a-sha");
        fs::write(&receipt_path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
        assert!(
            validate_evidence_command(Path::new("/unused"), receipt_path.to_str().unwrap())
                .unwrap_err()
                .contains("upload-receipt.source_sha")
        );

        receipt = valid_receipt.clone();
        receipt["size_in_bytes"] = serde_json::json!(0);
        fs::write(&receipt_path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
        assert!(
            validate_evidence_command(Path::new("/unused"), receipt_path.to_str().unwrap())
                .unwrap_err()
                .contains("upload-receipt.transport")
        );

        receipt = valid_receipt.clone();
        receipt["upload_outcome"] = serde_json::json!("unknown");
        fs::write(&receipt_path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
        assert!(
            validate_evidence_command(Path::new("/unused"), receipt_path.to_str().unwrap())
                .unwrap_err()
                .contains("upload-receipt.outcome")
        );

        receipt = valid_receipt.clone();
        receipt["unknown"] = serde_json::json!(true);
        fs::write(&receipt_path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
        assert!(
            validate_evidence_command(Path::new("/unused"), receipt_path.to_str().unwrap())
                .unwrap_err()
                .contains("unknown field")
        );

        receipt = valid_receipt;
        receipt["upload_outcome"] = serde_json::json!("failure");
        receipt["artifact_id"] = serde_json::Value::Null;
        receipt["artifact_url"] = serde_json::Value::Null;
        receipt["artifact_digest"] = serde_json::Value::Null;
        receipt["size_in_bytes"] = serde_json::Value::Null;
        fs::write(&receipt_path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
        validate_evidence_command(Path::new("/unused"), receipt_path.to_str().unwrap()).unwrap();
    }

    #[test]
    fn authority_unavailable_upload_receipt_preserves_unknown_retention() {
        let temp = tempfile::tempdir().unwrap();
        let receipt_path = temp.path().join("upload-receipt.json");
        let mut receipt = serde_json::json!({
            "schema": UPLOAD_AUTHORITY_UNAVAILABLE_SCHEMA,
            "job": "plan",
            "source_sha": "a".repeat(40),
            "artifact_name": "s4-plan-123-1",
            "artifact_id": 42,
            "artifact_url": "https://github.com/owner/repo/actions/runs/123/artifacts/42",
            "artifact_digest": format!("sha256:{}", "b".repeat(64)),
            "retention_days": null,
            "size_in_bytes": null,
            "upload_outcome": "success",
            "failure": "exact authority could not be resolved before checkout"
        });
        fs::write(&receipt_path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
        validate_evidence_command(
            Path::new("/definitely-not-a-repository"),
            receipt_path.to_str().unwrap(),
        )
        .unwrap();

        receipt["retention_days"] = serde_json::json!(30);
        fs::write(&receipt_path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
        assert!(
            validate_evidence_command(Path::new("/unused"), receipt_path.to_str().unwrap())
                .unwrap_err()
                .contains("upload-authority-unavailable.unknown-authority")
        );

        receipt["retention_days"] = serde_json::Value::Null;
        receipt["failure"] = serde_json::json!("forged");
        fs::write(&receipt_path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
        assert!(
            validate_evidence_command(Path::new("/unused"), receipt_path.to_str().unwrap())
                .unwrap_err()
                .contains("upload-authority-unavailable.failure")
        );
    }

    #[test]
    fn required_transport_validates_aggregate_result_without_a_repository() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(
            temp.path().join("required-result.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "schema": REQUIRED_RESULT_SCHEMA,
                "source_sha": "a".repeat(40),
                "plan": "success",
                "gates": "success"
            }))
            .unwrap(),
        )
        .unwrap();
        let mut index = TransportIndex {
            schema: TRANSPORT_SCHEMA.to_string(),
            job: "required-gates".to_string(),
            source_sha: "a".repeat(40),
            tuple: None,
            exit_code: None,
            job_status: None,
            files: vec![transport_file(temp.path(), "required-result.json")],
        };
        let index_path = temp.path().join("index.json");
        fs::write(&index_path, serde_json::to_vec_pretty(&index).unwrap()).unwrap();
        validate_evidence_command(
            Path::new("/definitely-not-a-repository"),
            index_path.to_str().unwrap(),
        )
        .unwrap();

        let mut bad_identity = index.clone();
        bad_identity.tuple = Some("macos-aarch64".to_string());
        assert!(validate_transport_index(temp.path(), &bad_identity)
            .contains(&"transport.required-gates.identity".to_string()));

        let valid_result = fs::read(temp.path().join("required-result.json")).unwrap();
        fs::write(
            temp.path().join("required-result.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "schema": REQUIRED_RESULT_SCHEMA,
                "source_sha": "a".repeat(40),
                "plan": "unknown",
                "gates": "success"
            }))
            .unwrap(),
        )
        .unwrap();
        let mut bad_outcome = index.clone();
        bad_outcome.files[0] = transport_file(temp.path(), "required-result.json");
        assert!(validate_transport_index(temp.path(), &bad_outcome)
            .contains(&"transport.required-gates.result".to_string()));
        fs::write(temp.path().join("required-result.json"), valid_result).unwrap();
        index.files[0] = transport_file(temp.path(), "required-result.json");

        index.source_sha = "b".repeat(40);
        assert!(validate_transport_index(temp.path(), &index)
            .contains(&"transport.required-gates.result".to_string()));
    }

    #[test]
    fn retained_nonzero_nm_fixture_requires_complete_diagnostics_and_failed_step_context() {
        let authority: Authority =
            serde_json::from_slice(include_bytes!("../../../ci/gates.json")).unwrap();
        let gate = authority.gate("probes-harnesses").unwrap();
        let step = gate
            .steps
            .iter()
            .find(|step| step.id == "menu-binding-probe")
            .unwrap();
        let bundle = tempfile::tempdir().unwrap();
        let mut index = valid_index();
        index.entries.clear();
        write_step_subordinate_fixture(bundle.path(), &mut index, gate, step);
        let prefix = format!(
            "payloads/subordinate.output/{}/{}/c-archive-nm",
            gate.id, step.id
        );
        rewrite_bundle_path(
            bundle.path(),
            &mut index.entries,
            &format!("{prefix}.txt"),
            b"partial nm listing\n",
        );
        rewrite_bundle_path(
            bundle.path(),
            &mut index.entries,
            &format!("{prefix}.stderr.txt"),
            b"nm: fixture diagnostic\n",
        );
        rewrite_bundle_path(
            bundle.path(),
            &mut index.entries,
            &format!("{prefix}.exit.txt"),
            b"7\n",
        );

        assert!(
            validate_step_subordinate_semantics(bundle.path(), &index, gate, step, false)
                .is_empty()
        );
        assert!(
            validate_step_subordinate_semantics(bundle.path(), &index, gate, step, true)
                .iter()
                .any(|contract| contract.ends_with("c-archive.nm_nonzero"))
        );

        index
            .entries
            .retain(|entry| entry.path != format!("{prefix}.stderr.txt"));
        assert!(
            validate_step_subordinate_semantics(bundle.path(), &index, gate, step, false)
                .iter()
                .any(|contract| contract.ends_with("c-archive.nm_triplet"))
        );
    }

    #[test]
    fn p00_harness_fixture_requires_a_recorded_archive_origin() {
        let authority: Authority =
            serde_json::from_slice(include_bytes!("../../../ci/gates.json")).unwrap();
        let gate = authority.gate("probes-harnesses").unwrap();
        let step = gate
            .steps
            .iter()
            .find(|step| step.id == "p00-harness")
            .unwrap();
        let bundle = tempfile::tempdir().unwrap();
        let mut index = valid_index();
        index.entries.clear();
        write_step_subordinate_fixture(bundle.path(), &mut index, gate, step);
        assert!(
            validate_step_subordinate_semantics(bundle.path(), &index, gate, step, true).is_empty()
        );

        let path = format!(
            "payloads/subordinate.output/{}/{}/archive-nm-origins.txt",
            gate.id, step.id
        );
        let missing_origin = String::from_utf8(subordinate_fixture_bytes(
            &step.id,
            "archive-nm-origins.txt",
        ))
        .unwrap()
        .replacen("DoInput\tlibuqm_c.a(input.c.o):\t", "DoInput\t\t", 1);
        rewrite_bundle_path(
            bundle.path(),
            &mut index.entries,
            &path,
            missing_origin.as_bytes(),
        );

        assert_eq!(
            validate_step_subordinate_semantics(bundle.path(), &index, gate, step, true),
            vec!["evidence.subordinate.p00-harness.nm_origins"]
        );
    }

    #[test]
    fn selected_nm_origin_fixture_rejects_wrong_archive_member() {
        let authority: Authority =
            serde_json::from_slice(include_bytes!("../../../ci/gates.json")).unwrap();
        let gate = authority.gate("probes-harnesses").unwrap();
        let step = gate
            .steps
            .iter()
            .find(|step| step.id == "menu-binding-probe")
            .unwrap();
        let bundle = tempfile::tempdir().unwrap();
        let mut index = valid_index();
        index.entries.clear();
        write_step_subordinate_fixture(bundle.path(), &mut index, gate, step);
        let path = format!(
            "payloads/subordinate.output/{}/{}/c-archive-nm-origins.txt",
            gate.id, step.id
        );
        let wrong_member = String::from_utf8(subordinate_fixture_bytes(
            &step.id,
            "c-archive-nm-origins.txt",
        ))
        .unwrap()
        .replacen(
            "libuqm_c.a(rust_vcontrol_impl.c.o)",
            "libuqm_c.a(wrong_member.c.o)",
            1,
        );
        rewrite_bundle_path(
            bundle.path(),
            &mut index.entries,
            &path,
            wrong_member.as_bytes(),
        );

        assert!(
            validate_step_subordinate_semantics(bundle.path(), &index, gate, step, true)
                .iter()
                .any(|contract| contract.ends_with("c-archive-nm-origins.txt"))
        );
    }

    #[test]
    fn menu_binding_fixture_rejects_wrong_but_valid_sdl_key() {
        let authority: Authority =
            serde_json::from_slice(include_bytes!("../../../ci/gates.json")).unwrap();
        let gate = authority.gate("probes-harnesses").unwrap();
        let step = gate
            .steps
            .iter()
            .find(|step| step.id == "menu-binding-probe")
            .unwrap();
        let bundle = tempfile::tempdir().unwrap();
        let mut index = valid_index();
        index.entries.clear();
        write_step_subordinate_fixture(bundle.path(), &mut index, gate, step);
        let path = format!(
            "payloads/subordinate.output/{}/{}/probe-output.txt",
            gate.id, step.id
        );
        let wrong_key = String::from_utf8(subordinate_fixture_bytes(&step.id, "probe-output.txt"))
            .unwrap()
            .replace("key_code=1073741905", "key_code=1073741906");
        rewrite_bundle_path(
            bundle.path(),
            &mut index.entries,
            &path,
            wrong_key.as_bytes(),
        );

        assert!(
            validate_step_subordinate_semantics(bundle.path(), &index, gate, step, true)
                .contains(&"evidence.subordinate.menu-binding-probe.sdlk_down".to_string())
        );
    }

    #[test]
    fn p00_probe_evidence_requires_every_named_success_and_summary() {
        let authority: Authority =
            serde_json::from_slice(include_bytes!("../../../ci/gates.json")).unwrap();
        let gate = authority.gate("probes-harnesses").unwrap();
        let step = gate
            .steps
            .iter()
            .find(|step| step.id == "p00-probes")
            .unwrap();
        let bundle = tempfile::tempdir().unwrap();
        let mut index = valid_index();
        index.entries.clear();
        write_step_subordinate_fixture(bundle.path(), &mut index, gate, step);
        assert!(
            validate_step_subordinate_semantics(bundle.path(), &index, gate, step, true).is_empty()
        );

        let path = format!(
            "payloads/subordinate.output/{}/{}/p00-probe-results.log",
            gate.id, step.id
        );
        let forged =
            String::from_utf8(subordinate_fixture_bytes(&step.id, "p00-probe-results.log"))
                .unwrap()
                .replace("PASS process_identity:", "FAIL process_identity:");
        rewrite_bundle_path(bundle.path(), &mut index.entries, &path, forged.as_bytes());
        assert_eq!(
            validate_step_subordinate_semantics(bundle.path(), &index, gate, step, true),
            vec!["evidence.subordinate.p00-probes.results"]
        );
    }

    #[test]
    fn subordinate_outputs_require_exact_success_sets_and_allow_failed_step_prefixes() {
        let authority: Authority =
            serde_json::from_slice(include_bytes!("../../../ci/gates.json")).unwrap();
        let gate = authority.gate("probes-harnesses").unwrap();
        let mut index = valid_index();
        index.entries.clear();
        for step in &gate.steps {
            for name in subordinate_output_names(&gate.id, &step.id) {
                index.entries.push(EvidenceEntry {
                    schema: ENTRY_SCHEMA.to_string(),
                    role: "subordinate.output".to_string(),
                    path: format!(
                        "payloads/subordinate.output/{}/{}/{}",
                        gate.id, step.id, name
                    ),
                    mime: "application/octet-stream".to_string(),
                    byte_length: 1,
                    sha256: "a".repeat(64),
                    producing_gate: gate.id.clone(),
                    producing_command: step.command.clone(),
                });
            }
        }
        assert!(validate_subordinate_outputs(&index, gate, gate.steps.len(), None).is_empty());

        let missing = index.entries.remove(0);
        assert!(
            validate_subordinate_outputs(&index, gate, gate.steps.len(), None)
                .iter()
                .any(|contract| contract.contains(".required"))
        );
        index.entries.push(missing);

        index.entries.last_mut().unwrap().producing_command = vec!["forged".to_string()];
        let forged = validate_subordinate_outputs(&index, gate, gate.steps.len(), None);
        assert!(forged.iter().any(|contract| contract.contains(".required")));
        assert!(forged
            .iter()
            .any(|contract| contract.contains(".unexpected")));

        index.entries.retain(|entry| {
            entry.path.contains("/p00-probes/") || entry.path.contains("/p00-harness/")
        });
        for entry in &mut index.entries {
            if entry.path.contains("/p00-probes/") {
                entry.producing_command = gate.steps[0].command.clone();
            } else if entry.path.contains("/p00-harness/") {
                entry.producing_command = gate.steps[1].command.clone();
            }
        }
        assert!(validate_subordinate_outputs(&index, gate, 1, Some(1)).is_empty());
        assert!(!validate_subordinate_outputs(&index, gate, 0, Some(0)).is_empty());
    }

    #[test]
    fn subordinate_post_failure_requires_successful_step_prefix_and_prior_outputs() {
        let authority: Authority =
            serde_json::from_slice(include_bytes!("../../../ci/gates.json")).unwrap();
        let gate = authority.gate("probes-harnesses").unwrap();
        let bundle = tempfile::tempdir().unwrap();
        let mut index = valid_index();
        index.entries.clear();
        for step in gate.steps.iter().take(2) {
            write_builtin_step_fixture(
                bundle.path(),
                &mut index.entries,
                &gate.id,
                &step.id,
                &step.command,
                (Some(0), None, None),
            );
        }
        let first = &gate.steps[0];
        write_step_subordinate_fixture(bundle.path(), &mut index, gate, first);
        let contract = "probes-harnesses.post.p00-harness.subordinate-output";
        assert!(
            validate_failed_subordinate_postprocess(bundle.path(), &index, gate, contract)
                .is_empty()
        );

        let mut pre_index = index.clone();
        pre_index.entries.retain(|entry| {
            entry.path.contains("/p00-probes/")
                || entry
                    .path
                    .starts_with(&format!("{}/{}.", gate.id, first.id))
        });
        let pre_contract = "probes-harnesses.pre.p00-harness.subordinate-output";
        assert!(validate_failed_subordinate_preprocess(
            bundle.path(),
            &pre_index,
            gate,
            pre_contract
        )
        .is_empty());
        pre_index.entries.push(EvidenceEntry {
            schema: ENTRY_SCHEMA.to_string(),
            role: "subordinate.output".to_string(),
            path: format!(
                "payloads/subordinate.output/{}/{}/{}",
                gate.id,
                gate.steps[1].id,
                subordinate_output_names(&gate.id, &gate.steps[1].id)[0]
            ),
            mime: "application/octet-stream".to_string(),
            byte_length: 1,
            sha256: "a".repeat(64),
            producing_gate: gate.id.clone(),
            producing_command: gate.steps[1].command.clone(),
        });
        assert!(validate_failed_subordinate_preprocess(
            bundle.path(),
            &pre_index,
            gate,
            pre_contract
        )
        .iter()
        .any(|failure| failure.contains(".unexpected")));

        index
            .entries
            .retain(|entry| entry.role != "subordinate.output");
        assert!(
            validate_failed_subordinate_postprocess(bundle.path(), &index, gate, contract)
                .iter()
                .any(|failure| failure.contains(".required"))
        );

        let later = &gate.steps[2];
        index.entries.push(EvidenceEntry {
            schema: ENTRY_SCHEMA.to_string(),
            role: "subordinate.output".to_string(),
            path: format!(
                "payloads/subordinate.output/{}/{}/{}",
                gate.id,
                later.id,
                subordinate_output_names(&gate.id, &later.id)[0]
            ),
            mime: "application/octet-stream".to_string(),
            byte_length: 1,
            sha256: "a".repeat(64),
            producing_gate: gate.id.clone(),
            producing_command: later.command.clone(),
        });
        assert!(
            validate_failed_subordinate_postprocess(bundle.path(), &index, gate, contract)
                .iter()
                .any(|failure| failure.contains(".unexpected"))
        );
    }

    #[test]
    fn subordinate_pre_and_post_failures_replay_through_full_production_validation() {
        let authority: Authority =
            serde_json::from_slice(include_bytes!("../../../ci/gates.json")).unwrap();
        let gate = authority.gate("probes-harnesses").unwrap();
        let bundle = tempfile::tempdir().unwrap();
        let seed = setup_failure_bundle(bundle.path(), "ownership.zero_native_delta");
        let mut entries = seed.entries;
        let delta_entry = entries
            .iter()
            .find(|entry| entry.role == "ownership.zero-native-delta")
            .unwrap();
        let mut delta: serde_json::Value =
            serde_json::from_slice(&fs::read(bundle.path().join(&delta_entry.path)).unwrap())
                .unwrap();
        delta["categories"]["tracked_sources"]["measured_delta"] = serde_json::json!(0);
        delta["categories"]["tracked_sources"]["changed_paths"] = serde_json::json!([]);
        delta["passed"] = serde_json::json!(true);
        rewrite_bundle_entry(
            bundle.path(),
            &mut entries,
            "ownership.zero-native-delta",
            &serde_json::to_vec(&delta).unwrap(),
        );
        let controller = vec![
            "uqm-xtask".to_string(),
            "ci".to_string(),
            "run".to_string(),
            gate.id.clone(),
        ];
        for entry in &mut entries {
            entry.producing_command = controller.clone();
            if matches!(
                entry.role.as_str(),
                "authority.snapshot"
                    | "preflight.source"
                    | "preflight.tools"
                    | "cache.initial-state"
            ) {
                entry.producing_gate = gate.id.clone();
            }
        }
        for step in gate.steps.iter().take(2) {
            write_builtin_step_fixture(
                bundle.path(),
                &mut entries,
                &gate.id,
                &step.id,
                &step.command,
                (Some(0), None, None),
            );
        }
        let first = &gate.steps[0];
        for name in subordinate_output_names(&gate.id, &first.id) {
            let path = format!(
                "payloads/subordinate.output/{}/{}/{}",
                gate.id, first.id, name
            );
            let bytes = subordinate_fixture_bytes(&first.id, name);
            write_bundle_entry(
                bundle.path(),
                &mut entries,
                &path,
                "subordinate.output",
                &first.command,
                &bytes,
            );
            entries.last_mut().unwrap().producing_gate = gate.id.clone();
        }
        let failed_contract = "probes-harnesses.post.p00-harness.subordinate-output";
        let result = serde_json::to_vec(&serde_json::json!({
            "schema": "uqm-s4-gate-result-v1",
            "gate": gate.id,
            "owner": gate.owner,
            "kind": gate.kind,
            "passed": false,
            "first_failed_contract": failed_contract,
            "detail": "subordinate output set differs",
            "controller_command": controller
        }))
        .unwrap();
        write_bundle_entry(
            bundle.path(),
            &mut entries,
            &format!("{}/gate.result.json", gate.id),
            "gate.result",
            &controller,
            &result,
        );
        entries.last_mut().unwrap().producing_gate = gate.id.clone();
        let mut pre_entries = entries.clone();
        let index = EvidenceIndex::build_and_validate(
            bundle.path(),
            &fixture_tuples(),
            EvidenceContext {
                source_sha: "a".repeat(40),
                clean: true,
                tuple: "linux-x86_64".to_string(),
                features: vec!["audio_heart".to_string()],
                cache_mode: "isolated-empty".to_string(),
                first_failed_contract: Some(failed_contract.to_string()),
            },
            entries,
        )
        .unwrap();
        assert!(
            index.offline_validation.passed,
            "{:?}",
            index.offline_validation.contracts
        );

        pre_entries.retain(|entry| {
            entry.role != "gate.result"
                && !entry
                    .path
                    .starts_with(&format!("{}/{}.", gate.id, gate.steps[1].id))
        });
        let pre_contract = "probes-harnesses.pre.p00-harness.subordinate-output";
        let pre_result = serde_json::to_vec(&serde_json::json!({
            "schema": "uqm-s4-gate-result-v1",
            "gate": gate.id,
            "owner": gate.owner,
            "kind": gate.kind,
            "passed": false,
            "first_failed_contract": pre_contract,
            "detail": "cannot create subordinate evidence directory",
            "controller_command": controller
        }))
        .unwrap();
        write_bundle_entry(
            bundle.path(),
            &mut pre_entries,
            &format!("{}/gate.result.json", gate.id),
            "gate.result",
            &controller,
            &pre_result,
        );
        pre_entries.last_mut().unwrap().producing_gate = gate.id.clone();
        let pre_index = EvidenceIndex::build_and_validate(
            bundle.path(),
            &fixture_tuples(),
            EvidenceContext {
                source_sha: "a".repeat(40),
                clean: true,
                tuple: "linux-x86_64".to_string(),
                features: vec!["audio_heart".to_string()],
                cache_mode: "isolated-empty".to_string(),
                first_failed_contract: Some(pre_contract.to_string()),
            },
            pre_entries,
        )
        .unwrap();
        assert!(
            pre_index.offline_validation.passed,
            "{:?}",
            pre_index.offline_validation.contracts
        );
    }

    #[test]
    fn native_acceptance_postprocess_failure_replays_after_successful_test_step() {
        let authority: Authority =
            serde_json::from_slice(include_bytes!("../../../ci/gates.json")).unwrap();
        let gate = authority.gate("tests").unwrap();
        let step = gate
            .steps
            .iter()
            .find(|step| step.id == "native-acceptance")
            .unwrap();
        let unrelated_step = gate
            .steps
            .iter()
            .find(|step| step.id == "xtask-test")
            .unwrap();
        let bundle = tempfile::tempdir().unwrap();
        let seed = setup_failure_bundle(bundle.path(), "ownership.zero_native_delta");
        let mut entries = seed.entries;
        let delta_entry = entries
            .iter()
            .find(|entry| entry.role == "ownership.zero-native-delta")
            .unwrap();
        let mut delta: serde_json::Value =
            serde_json::from_slice(&fs::read(bundle.path().join(&delta_entry.path)).unwrap())
                .unwrap();
        delta["categories"]["tracked_sources"]["measured_delta"] = serde_json::json!(0);
        delta["categories"]["tracked_sources"]["changed_paths"] = serde_json::json!([]);
        delta["passed"] = serde_json::json!(true);
        rewrite_bundle_entry(
            bundle.path(),
            &mut entries,
            "ownership.zero-native-delta",
            &serde_json::to_vec(&delta).unwrap(),
        );
        let source_entry = entries
            .iter()
            .find(|entry| entry.role == "preflight.source")
            .unwrap();
        let mut source: serde_json::Value =
            serde_json::from_slice(&fs::read(bundle.path().join(&source_entry.path)).unwrap())
                .unwrap();
        source["tuple"] = serde_json::json!("macos-aarch64");
        source["expected_tuple"] = serde_json::json!("macos-aarch64");
        rewrite_bundle_entry(
            bundle.path(),
            &mut entries,
            "preflight.source",
            &serde_json::to_vec(&source).unwrap(),
        );
        let controller = vec![
            "uqm-xtask".to_string(),
            "ci".to_string(),
            "run".to_string(),
            gate.id.clone(),
        ];
        for entry in &mut entries {
            entry.producing_command = controller.clone();
            if matches!(
                entry.role.as_str(),
                "authority.snapshot"
                    | "preflight.source"
                    | "preflight.tools"
                    | "cache.initial-state"
            ) {
                entry.producing_gate = gate.id.clone();
            }
        }
        write_builtin_step_fixture(
            bundle.path(),
            &mut entries,
            &gate.id,
            &step.id,
            &step.command,
            (Some(0), None, None),
        );
        let contract = "tests.post.native-acceptance.native-window-acceptance";
        let result = serde_json::to_vec(&serde_json::json!({
            "schema": "uqm-s4-gate-result-v1",
            "gate": gate.id,
            "owner": gate.owner,
            "kind": gate.kind,
            "passed": false,
            "first_failed_contract": contract,
            "detail": "native acceptance retention failed",
            "controller_command": controller
        }))
        .unwrap();
        write_bundle_entry(
            bundle.path(),
            &mut entries,
            &format!("{}/gate.result.json", gate.id),
            "gate.result",
            &controller,
            &result,
        );
        entries.last_mut().unwrap().producing_gate = gate.id.clone();
        let index = EvidenceIndex::build_and_validate(
            bundle.path(),
            &fixture_tuples(),
            EvidenceContext {
                source_sha: "a".repeat(40),
                clean: true,
                tuple: "macos-aarch64".to_string(),
                features: vec!["audio_heart".to_string()],
                cache_mode: "isolated-empty".to_string(),
                first_failed_contract: Some(contract.to_string()),
            },
            entries,
        )
        .unwrap();
        assert!(
            index.offline_validation.passed,
            "{:?}",
            index.offline_validation.contracts
        );
        let mut unrelated_test_failure = index.clone();
        unrelated_test_failure.first_failed_contract = Some("tests.xtask-test".to_string());
        unrelated_test_failure.entries.retain(|entry| {
            entry.producing_gate != gate.id || !entry.path.contains("tests/native-acceptance.")
        });
        write_builtin_step_fixture(
            bundle.path(),
            &mut unrelated_test_failure.entries,
            &gate.id,
            &unrelated_step.id,
            &unrelated_step.command,
            (Some(0), None, None),
        );
        let step_result_entry = unrelated_test_failure
            .entries
            .iter()
            .find(|entry| {
                entry.role == "step.result"
                    && entry.producing_gate == gate.id
                    && entry.path.ends_with("tests/xtask-test.result.json")
            })
            .unwrap();
        let step_result_path = step_result_entry.path.clone();
        let successful_step_result = fs::read(bundle.path().join(&step_result_path)).unwrap();
        let mut step_result: serde_json::Value =
            serde_json::from_slice(&successful_step_result).unwrap();
        step_result["exit_code"] = serde_json::json!(1);
        step_result["success"] = serde_json::json!(false);
        rewrite_bundle_entry(
            bundle.path(),
            &mut unrelated_test_failure.entries,
            "step.result",
            &serde_json::to_vec(&step_result).unwrap(),
        );
        let failed_gate_result = serde_json::to_vec(&serde_json::json!({
            "schema": "uqm-s4-gate-result-v1",
            "gate": gate.id,
            "owner": gate.owner,
            "kind": gate.kind,
            "passed": false,
            "first_failed_contract": "tests.xtask-test",
            "detail": "unrelated test failure",
            "controller_command": controller
        }))
        .unwrap();
        rewrite_bundle_entry(
            bundle.path(),
            &mut unrelated_test_failure.entries,
            "gate.result",
            &failed_gate_result,
        );
        assert_eq!(
            production_contracts(bundle.path(), &unrelated_test_failure),
            Vec::<String>::new()
        );
        fs::write(
            bundle.path().join(&step_result_path),
            successful_step_result,
        )
        .unwrap();
        fs::write(
            bundle.path().join(format!("{}/gate.result.json", gate.id)),
            &result,
        )
        .unwrap();
        let diagnostic_path =
            "payloads/native-window.acceptance/automation/native-window-state.json";
        let diagnostic_bytes = br#"{"schema":"diagnostic-partial"}"#;
        let mut partial = index.clone();
        write_bundle_entry(
            bundle.path(),
            &mut partial.entries,
            diagnostic_path,
            "native-window.acceptance",
            &step.command,
            diagnostic_bytes,
        );
        let partial_entry = partial.entries.last_mut().unwrap();
        partial_entry.mime = "application/octet-stream".to_string();
        partial_entry.producing_gate = gate.id.clone();
        assert_eq!(
            production_contracts(bundle.path(), &partial),
            Vec::<String>::new()
        );

        let mut wrong_producer = partial;
        wrong_producer.entries.last_mut().unwrap().producing_command = vec!["forged".to_string()];
        assert!(production_contracts(bundle.path(), &wrong_producer)
            .iter()
            .any(|contract| contract == "evidence.native_window.failed_entry_identity"));
    }

    #[test]
    #[ignore = "requires UQM_TEST_NATIVE_CONTENT_PACKAGE with the authority-pinned content package"]
    fn authority_valid_native_failure_bundle_replays_without_repository_state() {
        use uqm_rust::automation::{
            native_acceptance_failure_inventory, NativeAcceptanceFailureManifest,
            NativeChildCleanupReceipt, NativeProcessIdentity, NativeRetainedInput,
            NativeWindowBounds, NativeWindowConfigFile, NATIVE_ACCEPTANCE_FAILURE_SCHEMA,
            NATIVE_WINDOW_CONFIG_SCHEMA,
        };

        let content_source = std::env::var_os("UQM_TEST_NATIVE_CONTENT_PACKAGE")
            .map(PathBuf::from)
            .expect("UQM_TEST_NATIVE_CONTENT_PACKAGE must name the pinned content package");
        let authority: Authority =
            serde_json::from_slice(include_bytes!("../../../ci/gates.json")).unwrap();
        let gate = authority.gate("tests").unwrap();
        let step = gate
            .steps
            .iter()
            .find(|step| step.id == "native-acceptance")
            .unwrap();
        let bundle = tempfile::tempdir().unwrap();
        let acceptance = bundle.path().join("payloads/native-window.acceptance");
        fs::create_dir_all(acceptance.join("inputs")).unwrap();
        let executable_bytes = b"retained linked executable";
        let script_bytes = include_bytes!("../../../scripts/linked-playable-v1.json");
        fs::write(acceptance.join("inputs/uqm"), executable_bytes).unwrap();
        fs::write(
            acceptance.join("inputs/linked-playable-v1.json"),
            script_bytes,
        )
        .unwrap();
        let content_relative = format!(
            "inputs/content/packages/{}",
            authority.native_acceptance.content_filename
        );
        fs::create_dir_all(acceptance.join("inputs/content/packages")).unwrap();
        fs::write(
            acceptance.join("inputs/content/version"),
            format!("{}\n", authority.native_acceptance.content_version),
        )
        .unwrap();
        fs::write(
            acceptance.join("native-window-proof.json"),
            serde_json::to_vec(&NativeWindowConfigFile {
                schema: NATIVE_WINDOW_CONFIG_SCHEMA.to_string(),
                nonce: "a".repeat(64),
                client_bounds: NativeWindowBounds {
                    x: 80,
                    y: 80,
                    width: 1280,
                    height: 960,
                },
                runtime_contract: authority.native_runtime_contract(),
                acceptance_policy: authority.native_acceptance.acceptance_policy,
            })
            .unwrap(),
        )
        .unwrap();
        fs::copy(&content_source, acceptance.join(&content_relative)).unwrap();
        let content_bytes = fs::read(acceptance.join(&content_relative)).unwrap();
        assert_eq!(
            content_bytes.len() as u64,
            authority.native_acceptance.content_byte_length
        );
        assert_eq!(
            hex_sha256(&content_bytes),
            authority.native_acceptance.content_sha256
        );
        let retained = |relative_path: &str, bytes: &[u8]| NativeRetainedInput {
            relative_path: relative_path.to_string(),
            byte_length: bytes.len() as u64,
            sha256: hex_sha256(bytes),
        };
        let collection = acceptance.to_string_lossy();
        let manifest = NativeAcceptanceFailureManifest {
            schema: NATIVE_ACCEPTANCE_FAILURE_SCHEMA.to_string(),
            command: vec![
                acceptance.join("inputs/uqm").to_string_lossy().into_owned(),
                format!("--configdir={collection}/config"),
                format!("--contentdir={collection}/inputs/content"),
                format!("--automation-script={collection}/inputs/linked-playable-v1.json"),
                format!("--automation-output={collection}/automation"),
                format!("--native-window-proof={collection}/native-window-proof.json"),
            ],
            environment: std::collections::BTreeMap::from([(
                "SDL_AUDIODRIVER".to_string(),
                "dummy".to_string(),
            )]),
            executable: retained("inputs/uqm", executable_bytes),
            script: retained("inputs/linked-playable-v1.json", script_bytes),
            content_package: retained(&content_relative, &content_bytes),
            runtime_contract: authority.native_runtime_contract(),
            acceptance_policy: authority.native_acceptance.acceptance_policy,
            retained_files: native_acceptance_failure_inventory(
                &acceptance,
                authority.native_runtime_contract().inventory_limits,
            )
            .unwrap(),
            child: NativeChildCleanupReceipt {
                process: NativeProcessIdentity {
                    pid: 42,
                    start_time: "1234".to_string(),
                    executable_sha256: hex_sha256(executable_bytes),
                    nonce: "a".repeat(64),
                },
                exit_code: Some(0),
                signal: None,
                term_sent: false,
                kill_sent: false,
                stdout_bytes: 0,
                stderr_bytes: 1,
                output_drained: true,
                initial_process_group_empty: true,
                config_root_removed: true,
                materialized_content_removed: true,
            },
            failure_contract: serde_json::from_value(serde_json::json!("semantic")).unwrap(),
            error: "semantic acceptance failed".to_string(),
            passed: false,
        };
        assert_eq!(
            uqm_rust::automation::validate_native_acceptance_failure_bundle(&acceptance, &manifest,),
            Ok(())
        );
        fs::write(
            acceptance.join("native-acceptance-failure.json"),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let seed = setup_failure_bundle(bundle.path(), "ownership.zero_native_delta");
        let mut entries = seed.entries;
        let delta_entry = entries
            .iter()
            .find(|entry| entry.role == "ownership.zero-native-delta")
            .unwrap();
        let mut delta: serde_json::Value =
            serde_json::from_slice(&fs::read(bundle.path().join(&delta_entry.path)).unwrap())
                .unwrap();
        delta["categories"]["tracked_sources"]["measured_delta"] = serde_json::json!(0);
        delta["categories"]["tracked_sources"]["changed_paths"] = serde_json::json!([]);
        delta["passed"] = serde_json::json!(true);
        rewrite_bundle_entry(
            bundle.path(),
            &mut entries,
            "ownership.zero-native-delta",
            &serde_json::to_vec(&delta).unwrap(),
        );
        let source_entry = entries
            .iter()
            .find(|entry| entry.role == "preflight.source")
            .unwrap();
        let mut source: serde_json::Value =
            serde_json::from_slice(&fs::read(bundle.path().join(&source_entry.path)).unwrap())
                .unwrap();
        source["tuple"] = serde_json::json!("macos-aarch64");
        source["expected_tuple"] = serde_json::json!("macos-aarch64");
        rewrite_bundle_entry(
            bundle.path(),
            &mut entries,
            "preflight.source",
            &serde_json::to_vec(&source).unwrap(),
        );
        let controller = vec![
            "uqm-xtask".to_string(),
            "ci".to_string(),
            "run".to_string(),
            gate.id.clone(),
        ];
        for entry in &mut entries {
            entry.producing_command = controller.clone();
            if matches!(
                entry.role.as_str(),
                "authority.snapshot"
                    | "preflight.source"
                    | "preflight.tools"
                    | "cache.initial-state"
            ) {
                entry.producing_gate = gate.id.clone();
            }
        }
        write_builtin_step_fixture(
            bundle.path(),
            &mut entries,
            &gate.id,
            &step.id,
            &step.command,
            (Some(1), None, None),
        );
        for relative in manifest
            .retained_files
            .iter()
            .map(|file| file.relative_path.as_str())
            .chain(std::iter::once("native-acceptance-failure.json"))
        {
            let bundle_relative = format!("payloads/native-window.acceptance/{relative}");
            entries.push(
                entry(
                    bundle.path(),
                    &bundle_relative,
                    "native-window.failure",
                    "application/octet-stream",
                    &gate.id,
                    &step.command,
                )
                .unwrap(),
            );
        }
        let contract = "tests.native-acceptance";
        let result = serde_json::to_vec(&serde_json::json!({
            "schema": "uqm-s4-gate-result-v1",
            "gate": gate.id,
            "owner": gate.owner,
            "kind": gate.kind,
            "passed": false,
            "first_failed_contract": contract,
            "detail": "native acceptance reported a semantic failure",
            "controller_command": controller
        }))
        .unwrap();
        write_bundle_entry(
            bundle.path(),
            &mut entries,
            &format!("{}/gate.result.json", gate.id),
            "gate.result",
            &controller,
            &result,
        );
        entries.last_mut().unwrap().producing_gate = gate.id.clone();
        let index = EvidenceIndex::build_and_validate(
            bundle.path(),
            &fixture_tuples(),
            EvidenceContext {
                source_sha: "a".repeat(40),
                clean: true,
                tuple: "macos-aarch64".to_string(),
                features: vec!["audio_heart".to_string()],
                cache_mode: "isolated-empty".to_string(),
                first_failed_contract: Some(contract.to_string()),
            },
            entries,
        )
        .unwrap();
        assert!(
            index.offline_validation.passed,
            "{:?}",
            index.offline_validation.contracts
        );
        fs::write(
            bundle.path().join("evidence-index.json"),
            serde_json::to_vec_pretty(&index).unwrap(),
        )
        .unwrap();
        assert_eq!(index.first_failed_contract.as_deref(), Some(contract));
        validate_evidence_command(
            Path::new("/definitely-not-a-repository"),
            bundle.path().join("evidence-index.json").to_str().unwrap(),
        )
        .unwrap();

        let manifest_path = "payloads/native-window.acceptance/native-acceptance-failure.json";
        let script_path = "payloads/native-window.acceptance/inputs/linked-playable-v1.json";
        let content_path = format!("payloads/native-window.acceptance/{content_relative}");
        let assert_rejects = |forged: &EvidenceIndex, expected: &str| {
            let contracts = production_contracts(bundle.path(), forged);
            assert!(
                contracts.iter().any(|contract| contract == expected),
                "expected {expected}, got {contracts:?}"
            );
        };

        let mut forged = index.clone();
        let mut forged_script = script_bytes.to_vec();
        forged_script.push(b" "[0]);
        rewrite_bundle_path(
            bundle.path(),
            &mut forged.entries,
            script_path,
            &forged_script,
        );
        let mut forged_manifest = manifest.clone();
        forged_manifest.script.sha256 = hex_sha256(&forged_script);
        forged_manifest.script.byte_length = forged_script.len() as u64;
        let retained_script = forged_manifest
            .retained_files
            .iter_mut()
            .find(|file| file.relative_path == "inputs/linked-playable-v1.json")
            .unwrap();
        retained_script.sha256 = hex_sha256(&forged_script);
        retained_script.byte_length = forged_script.len() as u64;
        rewrite_bundle_path(
            bundle.path(),
            &mut forged.entries,
            manifest_path,
            &serde_json::to_vec_pretty(&forged_manifest).unwrap(),
        );
        assert_rejects(&forged, "evidence.native_window.failure.script");
        rewrite_bundle_path(
            bundle.path(),
            &mut forged.entries,
            script_path,
            script_bytes,
        );
        rewrite_bundle_path(
            bundle.path(),
            &mut forged.entries,
            manifest_path,
            &serde_json::to_vec_pretty(&manifest).unwrap(),
        );

        let mut forged_content = content_bytes.clone();
        forged_content.push(0);
        rewrite_bundle_path(
            bundle.path(),
            &mut forged.entries,
            &content_path,
            &forged_content,
        );
        let mut forged_manifest = manifest.clone();
        forged_manifest.content_package.sha256 = hex_sha256(&forged_content);
        forged_manifest.content_package.byte_length = forged_content.len() as u64;
        let retained_content = forged_manifest
            .retained_files
            .iter_mut()
            .find(|file| file.relative_path == content_relative)
            .unwrap();
        retained_content.sha256 = hex_sha256(&forged_content);
        retained_content.byte_length = forged_content.len() as u64;
        rewrite_bundle_path(
            bundle.path(),
            &mut forged.entries,
            manifest_path,
            &serde_json::to_vec_pretty(&forged_manifest).unwrap(),
        );
        assert_rejects(&forged, "evidence.native_window.failure.content");
        rewrite_bundle_path(
            bundle.path(),
            &mut forged.entries,
            &content_path,
            &content_bytes,
        );
        rewrite_bundle_path(
            bundle.path(),
            &mut forged.entries,
            manifest_path,
            &serde_json::to_vec_pretty(&manifest).unwrap(),
        );

        let mut forged_manifest = manifest.clone();
        forged_manifest.child.exit_code = Some(0);
        forged_manifest.child.signal = Some(9);
        rewrite_bundle_path(
            bundle.path(),
            &mut forged.entries,
            manifest_path,
            &serde_json::to_vec_pretty(&forged_manifest).unwrap(),
        );
        assert_rejects(&forged, "evidence.native_window.failure.result");
        rewrite_bundle_path(
            bundle.path(),
            &mut forged.entries,
            manifest_path,
            &serde_json::to_vec_pretty(&manifest).unwrap(),
        );

        let mut forged_manifest = manifest.clone();
        forged_manifest.acceptance_policy.stable_presentation_floor -= 1;
        rewrite_bundle_path(
            bundle.path(),
            &mut forged.entries,
            manifest_path,
            &serde_json::to_vec_pretty(&forged_manifest).unwrap(),
        );
        assert_rejects(&forged, "evidence.native_window.failure.authority_contract");
        rewrite_bundle_path(
            bundle.path(),
            &mut forged.entries,
            manifest_path,
            &serde_json::to_vec_pretty(&manifest).unwrap(),
        );

        let mut forged_manifest = manifest.clone();
        forged_manifest.runtime_contract.capture_timeout_ms -= 1;
        rewrite_bundle_path(
            bundle.path(),
            &mut forged.entries,
            manifest_path,
            &serde_json::to_vec_pretty(&forged_manifest).unwrap(),
        );
        assert_rejects(&forged, "evidence.native_window.failure.authority_contract");
        rewrite_bundle_path(
            bundle.path(),
            &mut forged.entries,
            manifest_path,
            &serde_json::to_vec_pretty(&manifest).unwrap(),
        );

        let mut forged_identity = index.clone();
        let native_entry = forged_identity
            .entries
            .iter_mut()
            .find(|entry| entry.role == "native-window.failure")
            .unwrap();
        native_entry.producing_command = vec!["forged".to_string()];
        assert_rejects(&forged_identity, "evidence.native_window.failure.inventory");
        let mut forged_role = index.clone();
        forged_role
            .entries
            .iter_mut()
            .find(|entry| entry.role == "native-window.failure")
            .unwrap()
            .role = "native-window.acceptance".to_string();
        assert_rejects(&forged_role, "evidence.native_window.failure.inventory");
        let mut forged_path = index.clone();
        forged_path
            .entries
            .iter_mut()
            .find(|entry| entry.role == "native-window.failure")
            .unwrap()
            .path = manifest_path.to_string();
        assert_rejects(&forged_path, "evidence.native_window.failure.inventory");
    }

    #[test]
    fn non_macos_native_acceptance_rejects_unindexed_payloads() {
        let root = tempfile::tempdir().unwrap();
        let authority: Authority =
            serde_json::from_slice(include_bytes!("../../../ci/gates.json")).unwrap();
        let gate = authority.gate("tests").unwrap();
        let mut index = valid_index();
        index.tuple = "linux-x86_64".to_string();
        index.entries.clear();

        assert!(
            validate_native_acceptance_evidence(root.path(), &index, &authority, gate).is_empty()
        );

        let native_root = root.path().join("payloads/native-window.acceptance");
        fs::create_dir_all(&native_root).unwrap();
        fs::write(native_root.join("unindexed.json"), b"{}").unwrap();
        assert_eq!(
            validate_native_acceptance_evidence(root.path(), &index, &authority, gate),
            vec!["evidence.native_window.unexpected_tuple"]
        );
    }

    #[cfg(unix)]
    #[test]
    fn native_acceptance_rejects_a_symlinked_payload_root_without_following_it() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("native-acceptance.json"), b"{}").unwrap();
        fs::create_dir_all(root.path().join("payloads")).unwrap();
        symlink(
            outside.path(),
            root.path().join("payloads/native-window.acceptance"),
        )
        .unwrap();
        let authority: Authority =
            serde_json::from_slice(include_bytes!("../../../ci/gates.json")).unwrap();
        let gate = authority.gate("tests").unwrap();
        let mut index = valid_index();
        index.tuple = "macos-aarch64".to_string();
        index.entries.clear();

        assert_eq!(
            validate_native_acceptance_evidence(root.path(), &index, &authority, gate),
            vec!["evidence.native_window.non_regular"]
        );
    }

    #[test]
    fn native_failure_manifest_requires_truthful_cleanup_provenance_and_inventory() {
        use uqm_rust::automation::{
            native_acceptance_failure_inventory, validate_native_acceptance_failure_bundle,
            NativeAcceptanceFailureManifest, NativeChildCleanupReceipt, NativeProcessIdentity,
            NativeRetainedInput, NativeWindowBounds, NativeWindowConfigFile,
            NATIVE_ACCEPTANCE_FAILURE_SCHEMA, NATIVE_WINDOW_CONFIG_SCHEMA,
        };

        let root = tempfile::tempdir().unwrap();
        let executable_bytes = b"retained linked executable";
        let script_bytes = include_bytes!("../../../scripts/linked-playable-v1.json");
        let content_bytes = b"retained native content";
        for (relative, bytes) in [
            ("inputs/uqm", executable_bytes.as_slice()),
            ("inputs/linked-playable-v1.json", script_bytes.as_slice()),
            (
                "inputs/content/packages/uqm-0.8.0-content.uqm",
                content_bytes.as_slice(),
            ),
            ("inputs/content/version", b"0.8.0\n".as_slice()),
        ] {
            let path = root.path().join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, bytes).unwrap();
        }
        fs::write(
            root.path().join("native-window-proof.json"),
            serde_json::to_vec(&NativeWindowConfigFile {
                schema: NATIVE_WINDOW_CONFIG_SCHEMA.to_string(),
                nonce: "a".repeat(64),
                client_bounds: NativeWindowBounds {
                    x: 80,
                    y: 80,
                    width: 1280,
                    height: 960,
                },
                runtime_contract: native_runtime_contract(),
                acceptance_policy: native_acceptance_policy(),
            })
            .unwrap(),
        )
        .unwrap();
        let retained = |relative_path: &str, bytes: &[u8]| NativeRetainedInput {
            relative_path: relative_path.to_string(),
            byte_length: bytes.len() as u64,
            sha256: hex_sha256(bytes),
        };
        let executable = retained("inputs/uqm", executable_bytes);
        let script = retained("inputs/linked-playable-v1.json", script_bytes);
        let content = retained(
            "inputs/content/packages/uqm-0.8.0-content.uqm",
            content_bytes,
        );
        let manifest = NativeAcceptanceFailureManifest {
            schema: NATIVE_ACCEPTANCE_FAILURE_SCHEMA.to_string(),
            command: vec![
                "/collection/inputs/uqm".to_string(),
                "--configdir=/collection/config".to_string(),
                "--contentdir=/collection/inputs/content".to_string(),
                "--automation-script=/collection/inputs/linked-playable-v1.json".to_string(),
                "--automation-output=/collection/automation".to_string(),
                "--native-window-proof=/collection/native-window-proof.json".to_string(),
            ],
            environment: std::collections::BTreeMap::from([(
                "SDL_AUDIODRIVER".to_string(),
                "dummy".to_string(),
            )]),
            executable,
            script,
            content_package: content,
            runtime_contract: native_runtime_contract(),
            acceptance_policy: native_acceptance_policy(),
            retained_files: native_acceptance_failure_inventory(
                root.path(),
                native_runtime_contract().inventory_limits,
            )
            .unwrap(),
            child: NativeChildCleanupReceipt {
                process: NativeProcessIdentity {
                    pid: 42,
                    start_time: "1234".to_string(),
                    executable_sha256: hex_sha256(executable_bytes),
                    nonce: "a".repeat(64),
                },
                exit_code: None,
                signal: Some(15),
                term_sent: true,
                kill_sent: false,
                stdout_bytes: 12,
                stderr_bytes: 3,
                output_drained: true,
                initial_process_group_empty: true,
                config_root_removed: true,
                materialized_content_removed: true,
            },
            failure_contract: serde_json::from_value(serde_json::json!("child-supervision"))
                .unwrap(),
            error: "child supervision failed".to_string(),
            passed: false,
        };
        for contract in ["child-supervision", "child-exit"] {
            let mut typed = manifest.clone();
            typed.failure_contract = serde_json::from_value(serde_json::json!(contract)).unwrap();
            assert!(validate_native_acceptance_failure_bundle(root.path(), &typed).is_ok());
        }
        for contract in ["observer", "config-cleanup", "semantic"] {
            let mut typed = manifest.clone();
            typed.failure_contract = serde_json::from_value(serde_json::json!(contract)).unwrap();
            typed.child.exit_code = Some(0);
            typed.child.signal = None;
            typed.child.term_sent = false;
            assert!(validate_native_acceptance_failure_bundle(root.path(), &typed).is_ok());
        }

        for exit_code in [0, 1, 255] {
            let mut exited = manifest.clone();
            exited.child.exit_code = Some(exit_code);
            exited.child.signal = None;
            exited.child.term_sent = false;
            assert!(validate_native_acceptance_failure_bundle(root.path(), &exited).is_ok());
        }
        for (exit_code, signal) in [
            (None, None),
            (Some(-1), None),
            (Some(256), None),
            (None, Some(0)),
            (None, Some(-1)),
            (Some(1), Some(15)),
        ] {
            let mut invalid = manifest.clone();
            invalid.child.exit_code = exit_code;
            invalid.child.signal = signal;
            assert!(validate_native_acceptance_failure_bundle(root.path(), &invalid).is_err());
        }
        let mut undrained = manifest.clone();
        undrained.child.output_drained = false;
        assert!(validate_native_acceptance_failure_bundle(root.path(), &undrained).is_err());
        let mut orphaned = manifest.clone();
        orphaned.child.initial_process_group_empty = false;
        assert!(validate_native_acceptance_failure_bundle(root.path(), &orphaned).is_err());
        let mut missing_cleanup = manifest.clone();
        missing_cleanup.child.config_root_removed = false;
        assert!(validate_native_acceptance_failure_bundle(root.path(), &missing_cleanup).is_err());

        let config_file = root.path().join("config/uqm.cfg");
        fs::create_dir_all(config_file.parent().unwrap()).unwrap();
        fs::write(&config_file, b"retained config after failed cleanup").unwrap();
        let mut config_failure = manifest.clone();
        config_failure.failure_contract =
            serde_json::from_value(serde_json::json!("config-cleanup")).unwrap();
        config_failure.child.exit_code = Some(0);
        config_failure.child.signal = None;
        config_failure.child.term_sent = false;
        config_failure.child.config_root_removed = false;
        config_failure.retained_files = native_acceptance_failure_inventory(
            root.path(),
            native_runtime_contract().inventory_limits,
        )
        .unwrap();
        assert!(validate_native_acceptance_failure_bundle(root.path(), &config_failure).is_ok());
        fs::remove_file(config_file).unwrap();
        fs::remove_dir(root.path().join("config")).unwrap();

        let mut contradictory_terminal = manifest.clone();
        contradictory_terminal.child.exit_code = Some(1);
        assert!(
            validate_native_acceptance_failure_bundle(root.path(), &contradictory_terminal)
                .is_err()
        );
        let mut forged_command = manifest.clone();
        forged_command.command[2] = "--contentdir=/outside".to_string();
        assert!(validate_native_acceptance_failure_bundle(root.path(), &forged_command).is_err());
        let mut forged_digest = manifest.clone();
        forged_digest.executable.sha256 = "b".repeat(64);
        assert!(validate_native_acceptance_failure_bundle(root.path(), &forged_digest).is_err());
        let mut omitted_inventory = manifest;
        omitted_inventory.retained_files.pop();
        assert!(
            validate_native_acceptance_failure_bundle(root.path(), &omitted_inventory).is_err()
        );
    }

    #[test]
    fn playable_capture_requires_semantic_evidence_at_the_capture_presentation() {
        use uqm_rust::automation::native_window::{
            NativeProcessIdentity, NativeScreenshot, NativeScreenshotStage, NativeWindowBinding,
            NativeWindowBounds, NativeWindowObservation, NativeWindowProof, NativeWindowProofError,
            NativeWindowSemanticSnapshot,
        };

        let process = NativeProcessIdentity {
            pid: 42,
            start_time: "1234".to_string(),
            executable_sha256: "a".repeat(64),
            nonce: "b".repeat(64),
        };
        let client_bounds = NativeWindowBounds {
            x: 100,
            y: 120,
            width: 640,
            height: 480,
        };
        let binding = NativeWindowBinding {
            process: process.clone(),
            window_id: 99,
            client_bounds,
            os_bounds: NativeWindowBounds {
                x: 94,
                y: 92,
                width: 652,
                height: 514,
            },
        };
        let policy = native_acceptance_policy();
        let playable_presentation_floor = policy.playable_presentation_floor;
        let mut proof = NativeWindowProof::new(process, client_bounds, policy);
        for committed_presentation in 0..=policy.stable_presentation_floor {
            proof
                .observe_visible(NativeWindowObservation {
                    binding: binding.clone(),
                    committed_presentation,
                    visible: true,
                    minimized: false,
                    semantic: NativeWindowSemanticSnapshot {
                        trace_record_count: committed_presentation + 1,
                        accepted_player_inputs: 0,
                        verified_battle_frames: 0,
                    },
                })
                .unwrap();
        }
        proof
            .record_screenshot(NativeScreenshot {
                stage: NativeScreenshotStage::Stable,
                binding: binding.clone(),
                post_capture_observation: NativeWindowObservation {
                    binding: binding.clone(),
                    committed_presentation: policy.stable_presentation_floor,
                    visible: true,
                    minimized: false,
                    semantic: NativeWindowSemanticSnapshot {
                        trace_record_count: policy.stable_presentation_floor + 1,
                        accepted_player_inputs: 0,
                        verified_battle_frames: 0,
                    },
                },
                committed_presentation: policy.stable_presentation_floor,
                input_events: 0,
                trace_record_count: policy.stable_presentation_floor + 1,
                battle_frames: 0,
                relative_path: "screenshots/stable.png".to_string(),
                byte_length: 128,
                sha256: "d".repeat(64),
            })
            .unwrap();
        for committed_presentation in
            (policy.stable_presentation_floor + 1)..=playable_presentation_floor
        {
            proof
                .observe_visible(NativeWindowObservation {
                    binding: binding.clone(),
                    committed_presentation,
                    visible: true,
                    minimized: false,
                    semantic: NativeWindowSemanticSnapshot {
                        trace_record_count: committed_presentation + 1,
                        accepted_player_inputs: 0,
                        verified_battle_frames: 0,
                    },
                })
                .unwrap();
        }
        let screenshot = |committed_presentation, input_events, battle_frames| NativeScreenshot {
            stage: NativeScreenshotStage::Playable,
            binding: binding.clone(),
            post_capture_observation: NativeWindowObservation {
                binding: binding.clone(),
                committed_presentation,
                visible: true,
                minimized: false,
                semantic: NativeWindowSemanticSnapshot {
                    trace_record_count: committed_presentation + 1,
                    accepted_player_inputs: input_events,
                    verified_battle_frames: battle_frames,
                },
            },
            committed_presentation,
            input_events,
            trace_record_count: committed_presentation + 1,
            battle_frames,
            relative_path: "screenshots/playable.png".to_string(),
            byte_length: 128,
            sha256: "c".repeat(64),
        };
        assert_eq!(
            proof.record_screenshot(screenshot(playable_presentation_floor, 0, 0)),
            Err(NativeWindowProofError::ScreenshotStage)
        );
        let capture_presentation = playable_presentation_floor + 1;
        proof
            .observe_visible(NativeWindowObservation {
                binding: binding.clone(),
                committed_presentation: capture_presentation,
                visible: true,
                minimized: false,
                semantic: NativeWindowSemanticSnapshot {
                    trace_record_count: capture_presentation + 1,
                    accepted_player_inputs: 1,
                    verified_battle_frames: policy.battle_frame_floor,
                },
            })
            .unwrap();
        proof
            .record_screenshot(screenshot(
                capture_presentation,
                1,
                policy.battle_frame_floor,
            ))
            .unwrap();
    }
    #[test]
    fn detached_source_replay_requires_expected_supervised_nonzero_exit() {
        let authority: Authority =
            serde_json::from_slice(include_bytes!("../../../ci/gates.json")).unwrap();
        let valid = fixture_detached_state();
        assert!(valid_detached_receipt(&valid, &authority));

        let mut attached = valid.clone();
        let mut unsafe_checkout = valid.clone();
        unsafe_checkout["command"] = serde_json::json!(["git", "symbolic-ref", "-q", "HEAD"]);
        assert!(!valid_detached_receipt(&unsafe_checkout, &authority));

        attached["exit_code"] = serde_json::json!(0);
        attached["success"] = serde_json::json!(true);
        assert!(!valid_detached_receipt(&attached, &authority));

        let mut signaled = valid.clone();
        signaled["exit_code"] = serde_json::Value::Null;
        signaled["signal"] = serde_json::json!(9);
        assert!(!valid_detached_receipt(&signaled, &authority));

        let mut forged_fatal = valid.clone();
        forged_fatal["exit_code"] = serde_json::json!(128);
        forged_fatal["stderr"] = serde_json::json!("fatal: forged detached result");
        forged_fatal["supervision"]["stderr_bytes_seen"] = serde_json::json!(29);
        assert!(!valid_detached_receipt(&forged_fatal, &authority));

        let mut forged_stdout = valid.clone();
        forged_stdout["stdout"] = serde_json::json!(
            "refs/heads/main
"
        );
        forged_stdout["supervision"]["stdout_bytes_seen"] = serde_json::json!(16);
        assert!(!valid_detached_receipt(&forged_stdout, &authority));

        let mut truncated = valid;
        truncated["supervision"]["stdout_truncated"] = serde_json::json!(true);
        truncated["supervision"]["stdout_bytes_seen"] = serde_json::json!(1);
        assert!(!valid_detached_receipt(&truncated, &authority));
    }

    #[test]
    fn detached_step_validation_requires_causal_supervision_receipts() {
        let base = |exit_code: serde_json::Value, signal: serde_json::Value| {
            serde_json::json!({
                "schema": "uqm-s4-step-result-v2",
                "gate": "format",
                "step": "fmt-check",
                "command": ["fixture"],
                "effective_command": ["fixture"],
                "staged_script_sha256": null,
                "executable_identity": fixture_executable_identity(None),
                "exit_code": exit_code,
                "signal": signal,
                "launch_error": null,
                "success": false,
                "supervision": fixture_supervision(false, 0, 0),
            })
        };

        let mut timeout = base(serde_json::Value::Null, serde_json::json!(9));
        timeout["supervision"]["timed_out"] = serde_json::json!(true);
        timeout["supervision"]["termination_reason"] = serde_json::json!("timeout");
        timeout["supervision"]["termination_signal"] = serde_json::json!("kill");
        assert!(valid_step_supervision(&timeout, 0, 0, Some(3_600_000)));
        timeout["supervision"]["timed_out"] = serde_json::json!(false);
        assert!(!valid_step_supervision(&timeout, 0, 0, Some(3_600_000)));

        let mut flood = base(serde_json::Value::Null, serde_json::json!(15));
        flood["supervision"]["stdout_limit_bytes"] = serde_json::json!(64);
        flood["supervision"]["stdout_bytes_seen"] = serde_json::json!(128);
        flood["supervision"]["stdout_truncated"] = serde_json::json!(true);
        flood["supervision"]["termination_reason"] = serde_json::json!("output-limit");
        flood["supervision"]["termination_signal"] = serde_json::json!("term");
        assert!(valid_step_supervision(&flood, 64, 0, None));
        flood["supervision"]["stdout_bytes_seen"] = serde_json::json!(64);
        assert!(!valid_step_supervision(&flood, 64, 0, None));

        let mut descendant = base(serde_json::json!(0), serde_json::Value::Null);
        descendant["supervision"]["termination_reason"] = serde_json::json!("descendant-cleanup");
        descendant["supervision"]["termination_signal"] = serde_json::json!("kill");
        assert!(valid_step_supervision(&descendant, 0, 0, Some(3_600_000)));
        descendant["supervision"]["process_group_cleanup"] = serde_json::json!("failed");
        assert!(!valid_step_supervision(&descendant, 0, 0, Some(3_600_000)));
    }

    #[test]
    fn native_linked_outer_correlation_rejects_rehashed_source_and_authority_forgery() {
        let retained = serde_json::json!({
            "relative_path": "fixture",
            "byte_length": 0,
            "sha256": "0".repeat(64),
        });
        let receipt = serde_json::to_vec(&serde_json::json!({
            "schema": uqm_rust::automation::NATIVE_LINKED_BUILD_RECEIPT_SCHEMA,
            "source_sha": "a".repeat(40),
            "cargo_command": ["cargo", "test"],
            "native_profile": "debug",
            "feature": "linked_c_archive",
            "cargo_executable_path": "fixture",
            "cargo_rust_archive_path": "fixture",
            "cargo_out_dir": "fixture",
            "executable": retained.clone(),
            "cargo_messages": retained.clone(),
            "rust_archive": retained.clone(),
            "c_archive": retained.clone(),
            "object_sidecar": retained.clone(),
            "provider_report": retained.clone(),
            "native_build_evidence": retained.clone(),
            "cargo_manifest": retained.clone(),
            "cargo_lock": retained.clone(),
            "authority": retained.clone(),
            "canonical_toolchain": retained,
        }))
        .unwrap();
        let authority = br#"{"schema":"fixture"}"#;

        assert!(linked_outer_correlation_contracts(
            Some(&receipt),
            Some(authority),
            Some(authority),
            &"a".repeat(40),
        )
        .is_empty());
        assert_eq!(
            linked_outer_correlation_contracts(
                Some(&receipt),
                Some(authority),
                Some(authority),
                &"b".repeat(40),
            ),
            vec!["evidence.native_window.linked_source"]
        );
        assert_eq!(
            linked_outer_correlation_contracts(
                Some(&receipt),
                Some(br#"{"schema":"forged"}"#),
                Some(authority),
                &"a".repeat(40),
            ),
            vec!["evidence.native_window.linked_authority"]
        );
    }

    #[test]
    fn transport_finalizer_fallback_validates_without_repository_state() {
        let fallback = TransportFinalizerFallback {
            schema: TRANSPORT_FALLBACK_SCHEMA.to_string(),
            job: "gates".to_string(),
            source_sha: "a".repeat(40),
            tuple: Some("linux-x86_64".to_string()),
            first_failed_contract: "transport.finalize".to_string(),
            detail: "transport finalizer did not replace the pre-seeded fallback index".to_string(),
        };

        assert!(validate_transport_finalizer_fallback(&fallback).is_empty());

        let mut forged = fallback.clone();
        forged.first_failed_contract = "transport.finalize.guessed-setup".to_string();
        assert!(validate_transport_finalizer_fallback(&forged)
            .iter()
            .any(|contract| contract == "transport-fallback.failure"));

        let value = serde_json::json!({
            "schema": TRANSPORT_FALLBACK_SCHEMA,
            "job": "plan",
            "source_sha": "a".repeat(40),
            "tuple": null,
            "first_failed_contract": "transport.finalize",
            "detail": "retained fallback",
            "guessed_checkout_outcome": "failure"
        });
        assert!(serde_json::from_value::<TransportFinalizerFallback>(value).is_err());
    }
}
