//! `ci run <gate|all>`: fail-fast gate execution with a content-addressed
//! evidence index.
//!
//! Gates run in authority order. The first internal contract failure stops the
//! run, preserves bounded stdout/stderr, records a typed first-failure
//! identity, and still emits the evidence index after offline validation.
//! Evidence entries are normalized paths relative to the bundle root. Offline
//! replay resolves only those embedded payloads and does not access the repository.

use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use super::authority::{Authority, Gate, GateKind, Step, AUTHORITY_RELATIVE};
use super::cache::{self, CacheEnvironment};
use super::delta;
use super::doctor;
use super::evidence::{self, EvidenceEntry, EvidenceIndex};
use super::exec::{run_captured_with_bound_environment, run_captured_with_limits as run_captured};
use super::{gate_by_id, CiError};
use crate::git_text;

/// State accumulated across one `ci run` invocation.
pub struct RunSession {
    pub root: PathBuf,
    pub authority: Authority,
    pub evidence_root: PathBuf,
    pub tuple: String,
    pub cache_mode: String,
    pub source_sha: String,
    pub clean: bool,
    pub features: Vec<String>,
    pub entries: Vec<EvidenceEntry>,
}

impl RunSession {
    /// Copy a produced file into the self-contained bundle and index that copy.
    pub fn entry_from_file(
        &mut self,
        path: &Path,
        role: &str,
        mime: &str,
        producing_gate: &str,
        producing_command: &[String],
    ) -> Result<(), CiError> {
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| CiError::new("evidence.bundle_path", "source filename is not UTF-8"))?;
        let source_root = path
            .parent()
            .ok_or_else(|| CiError::new("evidence.bundle_path", path.display().to_string()))?;
        let bytes = evidence::read_regular_relative(source_root, file_name).map_err(|error| {
            CiError::new(
                "evidence.bundle_copy",
                format!(
                    "cannot read {} without following links: {error}",
                    path.display()
                ),
            )
        })?;
        let relative = format!("payloads/{role}/{file_name}");
        self.publish_entry_bytes(
            &relative,
            &bytes,
            role,
            mime,
            producing_gate,
            producing_command,
        )
    }

    fn entry_from_bytes(
        &mut self,
        relative: &str,
        bytes: &[u8],
        role: &str,
        mime: &str,
        producing_gate: &str,
        producing_command: &[String],
    ) -> Result<(), CiError> {
        let entry = evidence::entry_from_bytes(
            relative,
            bytes,
            role,
            mime,
            producing_gate,
            producing_command,
        )?;
        self.entries.push(entry);
        Ok(())
    }

    fn publish_entry_bytes(
        &mut self,
        relative: &str,
        bytes: &[u8],
        role: &str,
        mime: &str,
        producing_gate: &str,
        producing_command: &[String],
    ) -> Result<(), CiError> {
        let publisher =
            evidence::EvidencePublisher::open(&self.evidence_root).map_err(|error| {
                CiError::new(
                    "evidence.bundle_copy",
                    format!("cannot open evidence root: {error}"),
                )
            })?;
        publisher.replace(relative, bytes).map_err(|error| {
            CiError::new(
                "evidence.bundle_copy",
                format!("cannot publish {relative}: {error}"),
            )
        })?;
        self.entry_from_bytes(
            relative,
            bytes,
            role,
            mime,
            producing_gate,
            producing_command,
        )
    }

    fn publish_snapshot_entries(
        &mut self,
        files: &[evidence::RegularFileSnapshot],
        destination_prefix: &str,
        role: &str,
        mime: &str,
        producing_gate: &str,
        producing_command: &[String],
    ) -> Result<(), CiError> {
        let publisher =
            evidence::EvidencePublisher::open(&self.evidence_root).map_err(|error| {
                CiError::new(
                    "evidence.bundle_copy",
                    format!("cannot open evidence root: {error}"),
                )
            })?;
        let mut entries = Vec::with_capacity(files.len());
        for file in files {
            let relative = format!("{destination_prefix}/{}", file.relative_path);
            publisher.replace(&relative, &file.bytes).map_err(|error| {
                CiError::new(
                    "evidence.bundle_copy",
                    format!("cannot publish {relative}: {error}"),
                )
            })?;
            entries.push(evidence::entry_from_bytes(
                &relative,
                &file.bytes,
                role,
                mime,
                producing_gate,
                producing_command,
            )?);
        }
        self.entries.extend(entries);
        Ok(())
    }
    /// Record a payload that is already inside the self-contained bundle.
    pub fn entry_from_evidence_path(
        &mut self,
        evidence_path: &Path,
        role: &str,
        mime: &str,
        producing_gate: &str,
        producing_command: &[String],
    ) -> Result<(), CiError> {
        let relative = relative_evidence_path(&self.evidence_root, evidence_path)?;
        let bytes =
            evidence::read_regular_relative(&self.evidence_root, &relative).map_err(|error| {
                CiError::new(
                    "evidence.read",
                    format!("cannot read {}: {error}", evidence_path.display()),
                )
            })?;
        self.publish_entry_bytes(
            &relative,
            &bytes,
            role,
            mime,
            producing_gate,
            producing_command,
        )
    }
}

/// Execute `ci run <gate|all>`.
pub fn run_gate(root: &Path, gate_arg: &str) -> Result<(), String> {
    match run_gate_with_session(root, gate_arg) {
        Ok(()) => Ok(()),
        Err(error) => Err(retain_run_failure(
            root,
            gate_arg,
            error,
            pre_session_evidence_root(root),
            &env::temp_dir().join("uqm-s4-pre-session-evidence-fallback"),
        )),
    }
}

fn retain_run_failure(
    root: &Path,
    gate_arg: &str,
    error: String,
    preferred: PathBuf,
    fallback_base: &Path,
) -> String {
    if error.starts_with("ci run evidence failed offline validation:")
        || error.starts_with("ci run failed at first contract '")
    {
        return error;
    }
    let (contract, detail) = classify_pre_session_failure(&error);
    let written = evidence::write_pre_session_failure(
        root, &preferred, gate_arg, contract, &detail,
    )
    .or_else(|preferred_error| {
        let fallback = fresh_pre_session_evidence_root(fallback_base);
        evidence::write_pre_session_failure(root, &fallback, gate_arg, contract, &detail).map_err(
            |fallback_error| {
                format!("{preferred_error}; fallback pre-session evidence failed: {fallback_error}")
            },
        )
    });
    match written {
        Ok(path) => format!(
            "{error}; pre-session contract '{contract}'; evidence: {}",
            path.display()
        ),
        Err(evidence_error) => {
            let envelope =
                evidence::PreSessionFailureEnvelope::build(root, gate_arg, contract, &detail);
            if let Ok(serialized) = serde_json::to_string(&envelope) {
                eprintln!("uqm-pre-session-evidence: {serialized}");
            }
            format!(
                "{error}; pre-session contract '{contract}'; evidence unavailable: {evidence_error}"
            )
        }
    }
}

fn detached_state_receipt(root: &Path, captured: &super::exec::Captured) -> serde_json::Value {
    serde_json::json!({
        "schema": "uqm-s4-detached-state-v1",
        "command": ["git", "-c", format!("safe.directory={}", root.display()), "symbolic-ref", "-q", "HEAD"],
        "exit_code": captured.exit_code,
        "signal": captured.signal,
        "launch_error": captured.launch_error,
        "success": captured.succeeded(),
        "stdout": String::from_utf8_lossy(&captured.stdout),
        "stderr": String::from_utf8_lossy(&captured.stderr),
        "supervision": {
            "timeout_milliseconds": captured.limits.timeout.as_millis(),
            "termination_grace_milliseconds": captured.limits.termination_grace.as_millis(),
            "pipe_drain_timeout_milliseconds": captured.limits.pipe_drain_timeout.as_millis(),
            "stdout_limit_bytes": captured.limits.stdout_bytes,
            "stderr_limit_bytes": captured.limits.stderr_bytes,
            "stdout_bytes_seen": captured.stdout_bytes_seen,
            "stderr_bytes_seen": captured.stderr_bytes_seen,
            "stdout_truncated": captured.stdout_truncated,
            "stderr_truncated": captured.stderr_truncated,
            "timed_out": captured.timed_out,
            "termination_reason": captured.termination_reason,
            "termination_signal": captured.termination_signal,
            "process_group_cleanup": captured.process_group_cleanup,
            "pipe_cleanup": captured.pipe_cleanup,
            "error": captured.supervision_error,
        },
    })
}

fn validate_detached_state(captured: &super::exec::Captured) -> Result<(), (String, String)> {
    let valid_terminal = captured.completed_under_supervision()
        && captured.exit_code == Some(1)
        && captured.signal.is_none()
        && captured.stdout.is_empty()
        && std::str::from_utf8(&captured.stderr).is_ok();
    if valid_terminal {
        Ok(())
    } else {
        Err((
            "source.detached_head".to_string(),
            captured.failure_detail("git symbolic-ref -q HEAD must exit 1 with empty stdout"),
        ))
    }
}

fn run_git_captured(
    root: &Path,
    arguments: &[String],
    limits: super::exec::Limits,
) -> super::exec::Captured {
    run_captured_with_bound_environment(root, "git", arguments, limits, true, |_| Ok(Vec::new()))
}

fn inspect_detached_state(
    root: &Path,
    authority: &Authority,
    required_mode: bool,
) -> (Option<super::exec::Captured>, Option<(String, String)>) {
    if !required_mode {
        return (None, None);
    }
    let captured = run_git_captured(
        root,
        &[
            "-c".into(),
            format!("safe.directory={}", root.display()),
            "symbolic-ref".into(),
            "-q".into(),
            "HEAD".into(),
        ],
        authority.supervision.builtin_limits(),
    );
    let failure = validate_detached_state(&captured).err();
    (Some(captured), failure)
}

fn evaluate_source_preflight(
    required_mode: bool,
    source_sha: &str,
    tuple: &str,
    expected: Option<&str>,
    expected_tuple: Option<&str>,
) -> (Option<(String, String)>, bool) {
    let mut failure = if required_mode && expected.is_none() {
        Some((
            "source.expected_sha".to_string(),
            "UQM_CI_EXPECTED_SHA is required in isolated-empty mode".to_string(),
        ))
    } else {
        validate_expected_source_sha(source_sha, expected)
            .err()
            .map(|error| split_contract_detail(&error))
    };
    if failure.is_none() && required_mode && expected_tuple.is_none() {
        failure = Some((
            "source.expected_tuple".to_string(),
            "UQM_CI_EXPECTED_TUPLE is required in isolated-empty mode".to_string(),
        ));
    } else if failure.is_none() && expected_tuple.is_some_and(|expected| expected != tuple) {
        failure = Some((
            "source.expected_tuple".to_string(),
            format!(
                "expected {}, running on {tuple}",
                expected_tuple.unwrap_or_default()
            ),
        ));
    }
    let canonical_environment = if required_mode {
        match crate::reject_ambient_build_flags() {
            Ok(()) => true,
            Err(error) => {
                if failure.is_none() {
                    failure = Some(("environment.canonical".to_string(), error));
                }
                false
            }
        }
    } else {
        false
    };
    (failure, canonical_environment)
}

fn source_receipt_failure(
    preflight_failure: &Option<(String, String)>,
    clean: bool,
) -> Option<(String, String)> {
    preflight_failure
        .as_ref()
        .filter(|(contract, _)| contract != "tools.preflight")
        .cloned()
        .or_else(|| {
            (!clean).then_some((
                "source.clean".to_string(),
                "tracked or untracked source state is not clean".to_string(),
            ))
        })
}

fn execute_gates(
    session: &mut RunSession,
    cache: Option<&CacheEnvironment>,
    gates: Vec<Gate>,
    controller_command: &[String],
    mut first_failed: Option<String>,
) -> Result<Option<String>, String> {
    for gate in gates {
        if first_failed.is_some() {
            break;
        }
        let result = execute_gate(
            session,
            cache.ok_or_else(|| "cache.prepare: execution cache was not prepared".to_string())?,
            &gate,
            controller_command,
        );
        record_gate_result(session, &gate, controller_command, &result)?;
        if let Err(error) = result {
            eprintln!("{error}");
            first_failed = Some(error.contract);
        }
    }
    Ok(first_failed)
}

fn finalize_session(
    session: &RunSession,
    supported_tuples: &[String],
    first_failed: Option<String>,
) -> Result<(), String> {
    let index = EvidenceIndex::build_and_validate(
        &session.evidence_root,
        supported_tuples,
        evidence::EvidenceContext {
            source_sha: session.source_sha.clone(),
            clean: session.clean,
            tuple: session.tuple.clone(),
            features: session.features.clone(),
            cache_mode: session.cache_mode.clone(),
            first_failed_contract: first_failed.clone(),
        },
        session.entries.clone(),
    )?;
    let index_path = session.evidence_root.join(evidence::INDEX_FILENAME);
    write_index(&index_path, &index)?;
    if !index.offline_validation.passed {
        return Err(format!(
            "ci run evidence failed offline validation: {}; evidence: {}",
            index.offline_validation.contracts[0],
            index_path.display()
        ));
    }
    if let Some(first) = first_failed {
        return Err(format!(
            "ci run failed at first contract '{}'; evidence: {}",
            first,
            index_path.display()
        ));
    }
    println!(
        "ci run complete; content-addressed evidence index: {}",
        index_path.display()
    );
    Ok(())
}

fn cache_receipt_for_session(
    root: &Path,
    authority: &super::authority::CacheAuthority,
    cache: Option<&cache::CacheEnvironment>,
) -> Result<cache::InitialStateReceipt, String> {
    match cache {
        Some(cache) => Ok(cache.receipt.clone()),
        None => cache::inspect(root, authority).map_err(|error| format!("cache.inspect: {error}")),
    }
}

fn selected_gates(authority: &Authority, gate_arg: &str) -> Result<Vec<Gate>, String> {
    if gate_arg == "all" {
        Ok(authority.gates.clone())
    } else {
        let gate = gate_by_id(authority, gate_arg).map_err(|error| error.to_string())?;
        Ok(vec![gate.clone()])
    }
}

fn run_gate_with_session(root: &Path, gate_arg: &str) -> Result<(), String> {
    let authority =
        super::load_authority(root).map_err(|error| format!("authority.load: {error}"))?;
    super::authority::validate_authority(&authority)
        .map_err(|error| format!("authority.validate: {error}"))?;
    let supported_tuples = super::plan::derive_plan(root)
        .map_err(|error| format!("plan.derive: {error}"))?
        .tuple_names();
    let gates = selected_gates(&authority, gate_arg)?;

    let tuple = format!("{}-{}", env::consts::OS, env::consts::ARCH);
    if !supported_tuples.contains(&tuple) {
        return Err(format!(
            "environment.tuple: host tuple '{tuple}' is absent from the validated CI plan"
        ));
    }
    let required_mode = super::cache::effective_mode(&authority.cache)
        .map_err(|error| error.to_string())?
        == authority.cache.mode;
    if required_mode && env::var_os("UQM_CI_EVIDENCE_ROOT").is_none() {
        return Err(
            "evidence.root: UQM_CI_EVIDENCE_ROOT is required in isolated-empty mode".into(),
        );
    }
    let source_sha = git_text(root, &["rev-parse", "HEAD"], "HEAD")
        .map_err(|error| format!("source.head: {error}"))?;
    validate_source_identity(&source_sha)?;
    let (detached_state, detached_failure) =
        inspect_detached_state(root, &authority, required_mode);
    let expected = env::var("UQM_CI_EXPECTED_SHA").ok();
    let expected_tuple = env::var("UQM_CI_EXPECTED_TUPLE").ok();
    let (source_failure, canonical_environment) = evaluate_source_preflight(
        required_mode,
        &source_sha,
        &tuple,
        expected.as_deref(),
        expected_tuple.as_deref(),
    );
    let mut preflight_failure = detached_failure.or(source_failure);
    let tool_report = doctor::inspect_tools(root, &authority);
    if preflight_failure.is_none() && !tool_report.passed {
        preflight_failure = Some((
            "tools.preflight".to_string(),
            "authoritative tool identity validation failed".to_string(),
        ));
    }
    let clean = git_status_empty(root, &authority.supervision)
        .map_err(|error| format!("source.status: {error}"))?;
    if preflight_failure.is_none() && !clean {
        preflight_failure = Some((
            "source.clean".to_string(),
            "tracked or untracked source state is not clean".to_string(),
        ));
    }
    let delta_report = if required_mode && preflight_failure.is_none() {
        Some(
            delta::measure(root, &authority, &source_sha)
                .map_err(|error| format!("ownership.delta_measure: {error}"))?,
        )
    } else {
        None
    };

    let cache = if preflight_failure.is_none() {
        Some(
            cache::prepare(root, &authority.cache)
                .map_err(|error| format!("cache.prepare: {error}"))?,
        )
    } else {
        None
    };
    let cache_receipt = cache_receipt_for_session(root, &authority.cache, cache.as_ref())?;
    let receipt_failures = cache::validate_receipt(&cache_receipt, &authority.cache)
        .map_err(|error| format!("cache.receipt: {error}"))?;
    let cache_failure = cache_receipt
        .first_failed_contract
        .clone()
        .or_else(|| receipt_failures.iter().next().cloned());
    let evidence_root =
        fresh_evidence_root(root).map_err(|error| format!("evidence.root: {error}"))?;
    super::exec::permit_containment_directory(&evidence_root)
        .map_err(|error| format!("evidence.dedicated_containment: {error}"))?;
    let features = {
        let set: BTreeSet<String> = authority
            .profiles
            .pure_test
            .iter()
            .chain(authority.profiles.linked_test.iter())
            .cloned()
            .collect();
        set.into_iter().collect()
    };
    let mut session = RunSession {
        root: root.to_path_buf(),
        authority,
        evidence_root,
        tuple,
        cache_mode: cache_receipt.mode.clone(),
        source_sha,
        clean,
        features,
        entries: Vec::new(),
    };

    let controller_command = gate_command(gate_arg).map_err(String::from)?;
    record_cache_receipt(
        &mut session,
        &cache_receipt,
        &gates[0].id,
        &controller_command,
    )?;
    session.entry_from_file(
        &root.join(AUTHORITY_RELATIVE),
        "authority.snapshot",
        "application/json",
        &gates[0].id,
        &controller_command,
    )?;
    let source_receipt_path = session.evidence_root.join("source-preflight.json");
    let source_failure = source_receipt_failure(&preflight_failure, clean);
    let source_receipt = serde_json::json!({
        "schema": "uqm-s4-source-preflight-v2",
        "source_sha": session.source_sha,
        "detached_state": detached_state
            .as_ref()
            .map(|captured| detached_state_receipt(root, captured)),
        "expected_sha": expected,
        "base_sha": env::var("UQM_CI_BASE_SHA").ok(),
        "tuple": session.tuple,
        "expected_tuple": expected_tuple,
        "cache_mode": session.cache_mode,
        "clean": session.clean,
        "canonical_environment": canonical_environment,
        "passed": source_failure.is_none(),
        "first_failed_contract": source_failure.as_ref().map(|(contract, _)| contract),
        "detail": source_failure.as_ref().map(|(_, detail)| detail),
    });
    fs::write(
        &source_receipt_path,
        serde_json::to_vec_pretty(&source_receipt)
            .map_err(|error| format!("cannot serialize source preflight: {error}"))?,
    )
    .map_err(|error| format!("cannot write {}: {error}", source_receipt_path.display()))?;
    session.entry_from_evidence_path(
        &source_receipt_path,
        "preflight.source",
        "application/json",
        &gates[0].id,
        &controller_command,
    )?;
    let tool_report_path = session.evidence_root.join("tool-preflight.json");
    fs::write(
        &tool_report_path,
        serde_json::to_vec_pretty(&tool_report)
            .map_err(|error| format!("cannot serialize tool preflight: {error}"))?,
    )
    .map_err(|error| format!("cannot write {}: {error}", tool_report_path.display()))?;
    session.entry_from_evidence_path(
        &tool_report_path,
        "preflight.tools",
        "application/json",
        &gates[0].id,
        &controller_command,
    )?;

    let delta_failure = if let Some(report) = &delta_report {
        let path = session.evidence_root.join("zero-native-delta.json");
        fs::write(
            &path,
            serde_json::to_vec_pretty(report)
                .map_err(|error| format!("cannot serialize zero-native delta: {error}"))?,
        )
        .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
        session.entry_from_evidence_path(
            &path,
            "ownership.zero-native-delta",
            "application/json",
            "ownership-link",
            &controller_command,
        )?;
        (!report.passed).then_some("ownership.zero_native_delta".to_string())
    } else {
        None
    };

    let first_failed = preflight_failure
        .as_ref()
        .map(|(contract, _)| contract.clone())
        .or(cache_failure)
        .or(delta_failure);
    let first_failed = execute_gates(
        &mut session,
        cache.as_ref(),
        gates,
        &controller_command,
        first_failed,
    )?;
    finalize_session(&session, &supported_tuples, first_failed)
}

fn classify_pre_session_failure(error: &str) -> (&'static str, String) {
    for contract in [
        "authority.load",
        "authority.validate",
        "plan.derive",
        "environment.tuple",
        "evidence.root",
        "source.head",
        "source.sha",
        "source.status",
        "ownership.delta_measure",
        "cache.prepare",
        "cache.inspect",
        "cache.receipt",
    ] {
        if let Some(detail) = error.strip_prefix(&format!("{contract}: ")) {
            return (contract, detail.to_string());
        }
    }
    if let Some(remainder) = error.strip_prefix("contract '") {
        if let Some((contract, detail)) = remainder.split_once("': ") {
            if contract == "authority.gate" {
                return ("authority.gate", detail.to_string());
            }
            if contract == "cache.mode" {
                return ("cache.mode", detail.to_string());
            }
        }
    }
    ("evidence.finalize", error.to_string())
}

fn pre_session_evidence_root(_root: &Path) -> PathBuf {
    let configured = env::var_os("UQM_CI_EVIDENCE_ROOT").map(PathBuf::from);
    fresh_pre_session_evidence_root(&pre_session_evidence_base(configured))
}

fn pre_session_evidence_base(configured: Option<PathBuf>) -> PathBuf {
    configured
        .map(|configured| {
            if configured.file_name().is_some_and(|name| name == "bundle") {
                configured
                    .parent()
                    .map(Path::to_path_buf)
                    .unwrap_or(configured)
            } else {
                configured
            }
        })
        .unwrap_or_else(|| env::temp_dir().join("uqm-s4-pre-session-evidence"))
}

fn fresh_pre_session_evidence_root(base: &Path) -> PathBuf {
    for ordinal in 0_u32..1_000 {
        let candidate = base.join(format!("pre-session-run-{ordinal}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    base.join(format!("pre-session-overflow-{}", std::process::id()))
}

fn split_contract_detail(message: &str) -> (String, String) {
    message
        .split_once(": ")
        .map(|(contract, detail)| (contract.to_string(), detail.to_string()))
        .unwrap_or_else(|| ("preflight.unknown".to_string(), message.to_string()))
}

fn validate_source_identity(source_sha: &str) -> Result<(), String> {
    if source_sha.len() != 40
        || !source_sha
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!(
            "source.sha: expected 40 lowercase hexadecimal characters, got '{source_sha}'"
        ));
    }
    Ok(())
}

fn validate_expected_source_sha(source_sha: &str, expected: Option<&str>) -> Result<(), String> {
    let Some(expected) = expected else {
        return Ok(());
    };
    validate_source_identity(expected)
        .map_err(|error| format!("source.expected_sha: invalid UQM_CI_EXPECTED_SHA: {error}"))?;
    if source_sha != expected {
        return Err(format!(
            "source.expected_sha: checked-out HEAD {source_sha} differs from UQM_CI_EXPECTED_SHA {expected}"
        ));
    }
    Ok(())
}
pub fn gate_command(gate_id: &str) -> Result<Vec<String>, CiError> {
    let executable = env::current_exe().map_err(|error| {
        CiError::new(
            "evidence.producing_command",
            format!("cannot identify the executing xtask binary: {error}"),
        )
    })?;
    Ok(vec![
        executable.display().to_string(),
        "ci".into(),
        "run".into(),
        gate_id.into(),
    ])
}

fn record_cache_receipt(
    session: &mut RunSession,
    receipt: &cache::InitialStateReceipt,
    producing_gate: &str,
    producing_command: &[String],
) -> Result<(), String> {
    let path = session.evidence_root.join("cache-initial-state.json");
    let bytes = serde_json::to_vec_pretty(receipt)
        .map_err(|error| format!("cannot serialize cache initial-state receipt: {error}"))?;
    fs::write(&path, bytes).map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    session
        .entry_from_evidence_path(
            &path,
            "cache.initial-state",
            "application/json",
            producing_gate,
            producing_command,
        )
        .map_err(|error| error.to_string())
}

fn record_gate_result(
    session: &mut RunSession,
    gate: &Gate,
    controller_command: &[String],
    result: &Result<(), CiError>,
) -> Result<(), String> {
    let path = session
        .evidence_root
        .join(&gate.id)
        .join("gate.result.json");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    let (contract, detail) = match result {
        Ok(()) => (None, None),
        Err(error) => (Some(error.contract.as_str()), Some(error.detail.as_str())),
    };
    let receipt = serde_json::json!({
        "schema": "uqm-s4-gate-result-v1",
        "gate": gate.id,
        "owner": gate.owner,
        "kind": gate.kind,
        "passed": result.is_ok(),
        "first_failed_contract": contract,
        "detail": detail,
        "controller_command": controller_command,
    });
    fs::write(
        &path,
        serde_json::to_vec_pretty(&receipt)
            .map_err(|error| format!("cannot serialize gate receipt: {error}"))?,
    )
    .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    session
        .entry_from_evidence_path(
            &path,
            "gate.result",
            "application/json",
            &gate.id,
            controller_command,
        )
        .map_err(String::from)
}

fn execute_gate(
    session: &mut RunSession,
    cache: &CacheEnvironment,
    gate: &Gate,
    controller_command: &[String],
) -> Result<(), CiError> {
    let gate_dir = session.evidence_root.join(&gate.id);
    fs::create_dir_all(&gate_dir)
        .map_err(|error| CiError::new(format!("{}.evidence", gate.id), error.to_string()))?;
    match gate.kind {
        GateKind::Process => execute_process_gate(session, cache, gate, controller_command),
        GateKind::Builtin => execute_builtin_gate(session, cache, gate),
    }
}

fn subordinate_output_names(gate: &str, step: &str) -> &'static [&'static str] {
    match (gate, step) {
        ("probes-harnesses", "p00-probes") => &["p00-probe-results.log"],
        ("probes-harnesses", "p00-harness") => &[
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

pub(crate) fn subordinate_evidence_environment(
    authority: &Authority,
    root: &Path,
) -> Vec<(String, String)> {
    vec![
        (
            "UQM_CI_SUBORDINATE_EVIDENCE_ROOT".into(),
            root.display().to_string(),
        ),
        (
            "UQM_CI_EVIDENCE_MEMBER_LIMIT_BYTES".into(),
            authority
                .actions
                .evidence_snapshot_member_limit_bytes
                .to_string(),
        ),
    ]
}

pub(super) fn trusted_control_plane_script(
    command: &[String],
) -> Option<(&'static str, &'static [u8])> {
    match command {
        [shell, path] if shell == "bash" && path == "rust/probes/run_p00_probes.sh" => Some((
            "run_p00_probes.sh",
            include_bytes!("../../../probes/run_p00_probes.sh"),
        )),
        [shell, path] if shell == "bash" && path == "rust/harness/run_p00_harness.sh" => Some((
            "run_p00_harness.sh",
            include_bytes!("../../../harness/run_p00_harness.sh"),
        )),
        [shell, path] if shell == "bash" && path == "rust/harness/run_menu_binding_probe.sh" => {
            Some((
                "run_menu_binding_probe.sh",
                include_bytes!("../../../harness/run_menu_binding_probe.sh"),
            ))
        }
        [shell, path] if shell == "bash" && path == "rust/ownership/verify-fixture.sh" => Some((
            "verify-fixture.sh",
            include_bytes!("../../../ownership/verify-fixture.sh"),
        )),
        [shell, path] if shell == "bash" && path == "rust/ownership/verify-production.sh" => {
            Some((
                "verify-production.sh",
                include_bytes!("../../../ownership/verify-production.sh"),
            ))
        }
        _ => None,
    }
}

fn trusted_controller_command(command: &[String]) -> Option<&'static str> {
    match command {
        [cargo, run, locked, manifest, path, separator, command]
            if cargo == "cargo"
                && run == "run"
                && locked == "--locked"
                && manifest == "--manifest-path"
                && path == "rust/xtask/Cargo.toml"
                && separator == "--" =>
        {
            match command.as_str() {
                "test" => Some("__ci-test"),
                "package" => Some("__ci-package"),
                "capture-dependencies" => Some("__ci-capture-dependencies"),
                _ => None,
            }
        }
        _ => None,
    }
}

fn process_command_requires_retained_source(command: &[String]) -> bool {
    command.first().is_some_and(|program| program == "cargo")
        && trusted_controller_command(command).is_none()
}

fn trusted_staging_parent_from(
    evidence_root: &Path,
    configured: Option<std::ffi::OsString>,
    dedicated_containment: bool,
) -> Result<(PathBuf, bool), CiError> {
    let is_configured = configured.is_some();
    if !is_configured && dedicated_containment {
        return Err(CiError::new(
            "trusted-control-plane-root",
            "UQM_CI_TRUSTED_STAGING_ROOT is required under dedicated-UID containment",
        ));
    }
    let parent = configured.map(PathBuf::from).unwrap_or_else(env::temp_dir);
    if !parent.is_absolute() || parent.starts_with(evidence_root) {
        return Err(CiError::new(
            "trusted-control-plane-root",
            "trusted staging root must be absolute and outside the writable evidence tree",
        ));
    }
    Ok((parent, is_configured))
}

fn trusted_staging_parent(evidence_root: &Path) -> Result<PathBuf, CiError> {
    use std::os::unix::fs::{MetadataExt as _, OpenOptionsExt as _, PermissionsExt as _};

    let (parent, is_configured) = trusted_staging_parent_from(
        evidence_root,
        env::var_os("UQM_CI_TRUSTED_STAGING_ROOT"),
        env::var_os(super::exec::DEDICATED_CONTAINMENT_UID_ENV).is_some(),
    )?;
    let directory = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(&parent)
        .map_err(|error| CiError::new("trusted-control-plane-root", error.to_string()))?;
    if is_configured {
        let metadata = directory
            .metadata()
            .map_err(|error| CiError::new("trusted-control-plane-root", error.to_string()))?;
        if metadata.uid() != unsafe { libc::geteuid() }
            || metadata.permissions().mode() & 0o7777 != 0o750
        {
            return Err(CiError::new(
                "trusted-control-plane-root",
                "configured trusted staging root must be controller-owned with exact mode 0750",
            ));
        }
    }
    Ok(parent)
}

fn runtime_binding_environment_from(
    source_root: &Path,
    source_bound: bool,
    configured_authority: Option<std::ffi::OsString>,
) -> Result<Vec<(String, String)>, CiError> {
    let authority = match configured_authority {
        Some(authority) => PathBuf::from(authority),
        None if !source_bound => source_root.join(super::authority::AUTHORITY_RELATIVE),
        None => {
            return Err(CiError::new(
                "authority.runtime_path",
                "UQM_CI_AUTHORITY_PATH is required with UQM_CI_SOURCE_ROOT",
            ));
        }
    };
    if !authority.is_absolute() {
        return Err(CiError::new(
            "authority.runtime_path",
            "UQM_CI_AUTHORITY_PATH must be absolute",
        ));
    }
    let source = source_root.display().to_string();
    Ok(vec![
        ("UQM_CI_SOURCE_ROOT".into(), source.clone()),
        (
            "UQM_CI_AUTHORITY_PATH".into(),
            authority.display().to_string(),
        ),
        ("GIT_CONFIG_COUNT".into(), "1".into()),
        ("GIT_CONFIG_KEY_0".into(), "safe.directory".into()),
        ("GIT_CONFIG_VALUE_0".into(), source),
    ])
}

fn runtime_binding_environment(source_root: &Path) -> Result<Vec<(String, String)>, CiError> {
    runtime_binding_environment_from(
        source_root,
        env::var_os("UQM_CI_SOURCE_ROOT").is_some(),
        env::var_os("UQM_CI_AUTHORITY_PATH"),
    )
}

#[cfg(unix)]
fn verify_trusted_control_plane_directory(
    directory: &Path,
    script: &Path,
    expected_script_sha256: &str,
) -> Result<(), CiError> {
    use std::os::unix::fs::{MetadataExt as _, OpenOptionsExt as _, PermissionsExt as _};

    let verify = |path: &Path, directory_kind: bool, mode: u32| -> Result<(), CiError> {
        let mut options = fs::OpenOptions::new();
        options
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
        if directory_kind {
            options.custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC);
        }
        let handle = options
            .open(path)
            .map_err(|error| CiError::new("trusted-control-plane-integrity", error.to_string()))?;
        let metadata = handle
            .metadata()
            .map_err(|error| CiError::new("trusted-control-plane-integrity", error.to_string()))?;
        let kind_matches = if directory_kind {
            metadata.file_type().is_dir()
        } else {
            metadata.file_type().is_file()
        };
        if !kind_matches
            || metadata.uid() != unsafe { libc::geteuid() }
            || metadata.permissions().mode() & 0o7777 != mode
        {
            return Err(CiError::new(
                "trusted-control-plane-integrity",
                format!(
                    "{} must be a controller-owned {} with exact mode {mode:04o}",
                    path.display(),
                    if directory_kind {
                        "directory"
                    } else {
                        "regular file"
                    }
                ),
            ));
        }
        Ok(())
    };

    verify(directory, true, 0o750)?;
    let controller = directory.join("uqm-base-controller");
    if script.parent() != Some(directory) || script == controller {
        return Err(CiError::new(
            "trusted-control-plane-integrity",
            "trusted script must be the selected direct staging-directory member",
        ));
    }
    let members = fs::read_dir(directory)
        .map_err(|error| CiError::new("trusted-control-plane-integrity", error.to_string()))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<std::collections::BTreeSet<_>, _>>()
        .map_err(|error| CiError::new("trusted-control-plane-integrity", error.to_string()))?;
    let expected = [controller.clone(), script.to_path_buf()]
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>();
    if members != expected {
        return Err(CiError::new(
            "trusted-control-plane-integrity",
            "trusted control-plane staging directory members differ from the selected controller and script",
        ));
    }
    verify(&controller, false, 0o550)?;
    verify(script, false, 0o440)?;
    let script_bytes = super::bounded_io::read_regular_nofollow(script, 1024 * 1024)
        .map_err(|detail| CiError::new("trusted-control-plane-integrity", detail))?;
    if super::evidence::hex_sha256(&script_bytes) != expected_script_sha256 {
        return Err(CiError::new(
            "trusted-control-plane-integrity",
            "trusted script bytes changed after staging",
        ));
    }
    Ok(())
}

#[cfg(not(unix))]
fn verify_trusted_control_plane_directory(
    _directory: &Path,
    _script: &Path,
    _expected_script_sha256: &str,
) -> Result<(), CiError> {
    Ok(())
}

type StagedControlPlaneScript = (Vec<String>, Option<tempfile::TempDir>, Option<String>);

fn stage_trusted_control_plane_script(
    command: &[String],
    trusted_staging_parent: &Path,
    source_root: &Path,
    controller_limit: u64,
    environment: &mut Vec<(String, String)>,
) -> Result<StagedControlPlaneScript, CiError> {
    let mut effective_command = command.to_vec();
    if let Some(hidden_command) = trusted_controller_command(command) {
        let controller = std::env::current_exe().map_err(|error| {
            CiError::new(
                "trusted-control-plane-controller",
                format!("cannot resolve the base-owned controller executable: {error}"),
            )
        })?;
        return Ok((
            vec![
                controller.to_string_lossy().into_owned(),
                hidden_command.into(),
            ],
            None,
            None,
        ));
    }
    if command
        .iter()
        .any(|argument| argument == "rust/xtask/Cargo.toml")
    {
        return Err(CiError::new(
            "trusted-controller-command",
            format!("unrecognized head-xtask command vector: {command:?}"),
        ));
    }
    let Some((script_name, script_bytes)) = trusted_control_plane_script(command) else {
        if matches!(command, [shell, path] if shell == "bash" && path.ends_with(".sh")) {
            return Err(CiError::new(
                "trusted-script",
                "authority shell script is not embedded in the trusted controller",
            ));
        }
        return Ok((effective_command, None, None));
    };
    let directory = tempfile::Builder::new()
        .prefix("trusted-control-plane-")
        .tempdir_in(trusted_staging_parent)
        .map_err(|error| CiError::new("trusted-control-plane-script", error.to_string()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o750))
            .map_err(|error| CiError::new("trusted-control-plane-script", error.to_string()))?;
    }
    let staged = directory.path().join(script_name);
    std::fs::write(&staged, script_bytes)
        .map_err(|error| CiError::new("trusted-control-plane-script", error.to_string()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o440))
            .map_err(|error| CiError::new("trusted-control-plane-script", error.to_string()))?;
    }
    effective_command[1] = staged.to_string_lossy().into_owned();
    let controller_source = std::env::current_exe().map_err(|error| {
        CiError::new(
            "trusted-control-plane-controller",
            format!("cannot resolve the base-owned controller executable: {error}"),
        )
    })?;
    let controller = directory.path().join("uqm-base-controller");
    super::bounded_io::copy_regular_nofollow(&controller_source, &controller, controller_limit)
        .map_err(|detail| CiError::new("trusted-control-plane-controller", detail))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&controller, std::fs::Permissions::from_mode(0o550))
            .map_err(|error| CiError::new("trusted-control-plane-controller", error.to_string()))?;
    }
    environment.extend([
        (
            "UQM_CI_SOURCE_ROOT".to_string(),
            source_root.to_string_lossy().into_owned(),
        ),
        (
            "UQM_CI_CONTROLLER_EXECUTABLE".to_string(),
            controller.to_string_lossy().into_owned(),
        ),
    ]);
    Ok((
        effective_command,
        Some(directory),
        Some(super::evidence::hex_sha256(script_bytes)),
    ))
}

fn execute_process_gate(
    session: &mut RunSession,
    cache: &CacheEnvironment,
    gate: &Gate,
    controller_command: &[String],
) -> Result<(), CiError> {
    let gate_dir = session.evidence_root.join(&gate.id);
    fs::create_dir_all(&gate_dir)
        .map_err(|error| CiError::new(format!("{}.evidence", gate.id), error.to_string()))?;
    super::exec::permit_containment_directory(&gate_dir)
        .map_err(|detail| CiError::new(format!("{}.dedicated_containment", gate.id), detail))?;
    let mut cache_environment = cache.resolved().vars;
    cache_environment.extend(runtime_binding_environment(&session.root)?);
    let trusted_staging_parent = trusted_staging_parent(&session.evidence_root)?;
    let linked_environment = gate
        .steps
        .iter()
        .any(|step| step.native_profile.as_deref() == Some("linked-test"))
        .then(|| session.authority.linked_step_env())
        .transpose()
        .map_err(|error| CiError::new(format!("{}.exec", gate.id), error))?;
    for step in &gate.steps {
        let mut env_overrides = cache_environment.clone();
        match step.native_profile.as_deref() {
            Some("linked-test") => {
                env_overrides.extend(linked_environment.iter().flatten().cloned());
            }
            Some("production") => {
                env_overrides.push(("UQM_NATIVE_PROFILE".into(), "production".into()));
            }
            Some(profile) => {
                return Err(CiError::new(
                    format!("{}.exec", gate.id),
                    format!("unsupported native profile '{profile}'"),
                ));
            }
            None => {}
        }
        let subordinate_names = subordinate_output_names(&gate.id, &step.id);
        let subordinate_root = session
            .evidence_root
            .join("payloads/subordinate.output")
            .join(&gate.id)
            .join(&step.id);
        if !subordinate_names.is_empty() {
            fs::create_dir_all(&subordinate_root).map_err(|error| {
                CiError::new(
                    format!("{}.pre.{}.subordinate-output", gate.id, step.id),
                    format!("cannot create {}: {error}", subordinate_root.display()),
                )
            })?;
            super::exec::permit_containment_directory(&subordinate_root).map_err(|detail| {
                CiError::new(
                    format!("{}.pre.{}.dedicated_containment", gate.id, step.id),
                    detail,
                )
            })?;
            env_overrides.extend(subordinate_evidence_environment(
                &session.authority,
                &subordinate_root,
            ));
        }
        let native_platform_prefix = format!("{}-", session.authority.native_acceptance.platform);
        let native_acceptance_root = (gate.id == "tests"
            && step.id == "xtask-test"
            && session.tuple.starts_with(&native_platform_prefix))
        .then(|| {
            session
                .evidence_root
                .join("payloads/native-window.acceptance")
        });
        if let Some(root) = &native_acceptance_root {
            fs::create_dir_all(root).map_err(|error| {
                CiError::new(
                    format!("{}.pre.{}.native-acceptance-root", gate.id, step.id),
                    format!("cannot create {}: {error}", root.display()),
                )
            })?;
            super::exec::permit_containment_directory(root).map_err(|detail| {
                CiError::new(
                    format!("{}.pre.{}.native-acceptance-root", gate.id, step.id),
                    detail,
                )
            })?;
            env_overrides.extend([
                (
                    "UQM_CI_NATIVE_ACCEPTANCE_EVIDENCE_ROOT".into(),
                    root.display().to_string(),
                ),
                (
                    "UQM_CI_NATIVE_ACCEPTANCE_PRECREATED_ROOT".into(),
                    "1".into(),
                ),
            ]);
            if let Ok(content_root) = env::var("UQM_CI_NATIVE_CONTENT_ROOT") {
                env_overrides.push(("UQM_CI_NATIVE_CONTENT_ROOT".into(), content_root));
            }
        }
        let execute_retained_source = process_command_requires_retained_source(&step.command);
        let (effective_command, trusted_script_directory, staged_script_sha256) =
            stage_trusted_control_plane_script(
                &step.command,
                &trusted_staging_parent,
                &session.root,
                session.authority.supervision.executable_member_limit_bytes,
                &mut env_overrides,
            )
            .map_err(|error| {
                CiError::new(
                    format!("{}.pre.{}.{}", gate.id, step.id, error.contract),
                    error.detail,
                )
            })?;
        if gate.id == "security" && step.id == "cargo-audit" {
            record_security_advisory_database(session, controller_command)?;
        }
        if let Some(directory) = trusted_script_directory.as_ref() {
            let staged_sha256 = staged_script_sha256.as_deref().ok_or_else(|| {
                CiError::new(
                    format!(
                        "{}.pre.{}.trusted-control-plane-integrity",
                        gate.id, step.id
                    ),
                    "staged trusted script is missing its expected digest",
                )
            })?;
            verify_trusted_control_plane_directory(
                directory.path(),
                Path::new(&effective_command[1]),
                staged_sha256,
            )
            .map_err(|error| {
                CiError::new(
                    format!("{}.pre.{}.{}", gate.id, step.id, error.contract),
                    error.detail,
                )
            })?;
        }
        let captured = super::exec::run_captured_with_bound_environment(
            &session.root.join(&step.cwd),
            &effective_command[0],
            &effective_command[1..],
            session.authority.supervision.limits(step.timeout_seconds),
            execute_retained_source,
            |_| Ok(env_overrides),
        );
        write_captured(
            session,
            gate,
            &step.id,
            &step.command,
            &effective_command,
            staged_script_sha256.as_deref(),
            &captured,
        )?;
        drop(trusted_script_directory);
        let subordinate_result = retain_subordinate_outputs(
            session,
            gate,
            step,
            &subordinate_root,
            subordinate_names,
            captured.succeeded(),
        );
        if !captured.succeeded() {
            let contract = format!("{}.{}", gate.id, step.id);
            let mut detail = captured.failure_detail(&step.command[0]);
            if let Some(root) = native_acceptance_root.as_deref() {
                let manifest = root.join("native-acceptance-failure.json");
                match fs::symlink_metadata(&manifest) {
                    Ok(_) => {
                        if let Err(error) =
                            retain_native_acceptance_failure(session, gate, step, root)
                        {
                            detail.push_str(&format!(
                                "; retaining native-acceptance failure evidence failed at {}: {}",
                                error.contract, error.detail
                            ));
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => detail.push_str(&format!(
                        "; cannot inspect optional native-acceptance failure manifest {}: {error}",
                        manifest.display()
                    )),
                }
            }
            return Err(CiError::new(contract, detail));
        }
        subordinate_result?;
        if let Some(root) = native_acceptance_root.as_deref() {
            retain_native_acceptance(session, gate, step, root)?;
        }
    }
    if gate.id == "package" {
        validate_and_record_package(session, gate)?;
    }
    Ok(())
}

fn retain_native_acceptance(
    session: &mut RunSession,
    gate: &Gate,
    step: &Step,
    root: &Path,
) -> Result<(), CiError> {
    let manifest_path = root.join("native-acceptance.json");
    let snapshot = evidence::EvidenceSnapshot::open(root).map_err(|error| {
        CiError::new(
            "tests.post.xtask-test.native-window-acceptance",
            format!("cannot snapshot {}: {error}", root.display()),
        )
    })?;
    let manifest_bytes = snapshot.read("native-acceptance.json").map_err(|error| {
        CiError::new(
            "tests.post.xtask-test.native-window-acceptance",
            format!("cannot read {}: {error}", manifest_path.display()),
        )
    })?;
    let manifest: uqm_rust::automation::NativeAcceptanceManifest =
        serde_json::from_slice(manifest_bytes).map_err(|error| {
            CiError::new(
                "tests.post.xtask-test.native-window-acceptance",
                format!("cannot parse {}: {error}", manifest_path.display()),
            )
        })?;
    let validation = materialize_native_snapshot(session, &snapshot)?;
    uqm_rust::automation::validate_native_acceptance_bundle(validation.path(), &manifest).map_err(
        |error| {
            CiError::new(
                "tests.post.xtask-test.native-window-acceptance",
                format!("native acceptance bundle failed validation: {error:?}"),
            )
        },
    )?;
    retain_native_acceptance_files(
        session,
        &snapshot,
        "native-window.acceptance",
        gate,
        step,
        "tests.post.xtask-test.native-window-acceptance",
    )
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum NativeAcceptanceFailureEnvelope {
    Runtime(Box<uqm_rust::automation::NativeAcceptanceFailureManifest>),
    Setup(Box<uqm_rust::automation::NativeAcceptanceSetupFailureManifest>),
}

fn retain_native_acceptance_failure(
    session: &mut RunSession,
    gate: &Gate,
    step: &Step,
    root: &Path,
) -> Result<(), CiError> {
    let manifest_path = root.join("native-acceptance-failure.json");
    let snapshot = evidence::EvidenceSnapshot::open(root).map_err(|error| {
        CiError::new(
            format!("{}.{}", gate.id, step.id),
            format!("cannot snapshot {}: {error}", root.display()),
        )
    })?;
    let manifest_bytes = match snapshot.read("native-acceptance-failure.json") {
        Ok(bytes) => bytes,
        Err(error) => {
            return Err(CiError::new(
                format!("{}.{}", gate.id, step.id),
                format!("cannot read {}: {error}", manifest_path.display()),
            ));
        }
    };
    let manifest: NativeAcceptanceFailureEnvelope = serde_json::from_slice(manifest_bytes)
        .map_err(|error| {
            CiError::new(
                format!("{}.{}", gate.id, step.id),
                format!("cannot parse {}: {error}", manifest_path.display()),
            )
        })?;
    let validation = materialize_native_snapshot(session, &snapshot)?;
    let validation_result = match &manifest {
        NativeAcceptanceFailureEnvelope::Runtime(manifest) => {
            uqm_rust::automation::validate_native_acceptance_failure_bundle(
                validation.path(),
                manifest,
            )
        }
        NativeAcceptanceFailureEnvelope::Setup(manifest) => {
            uqm_rust::automation::validate_native_acceptance_setup_failure_bundle(
                validation.path(),
                manifest,
            )
        }
    };
    validation_result.map_err(|error| {
        CiError::new(
            format!("{}.{}", gate.id, step.id),
            format!("native acceptance failure bundle failed validation: {error:?}"),
        )
    })?;
    let retention_contract = format!("{}.{}", gate.id, step.id);
    retain_native_acceptance_files(
        session,
        &snapshot,
        "native-window.failure",
        gate,
        step,
        &retention_contract,
    )
}

fn materialize_native_snapshot(
    session: &RunSession,
    snapshot: &evidence::EvidenceSnapshot,
) -> Result<tempfile::TempDir, CiError> {
    let directory = tempfile::tempdir_in(&session.evidence_root).map_err(|error| {
        CiError::new(
            "tests.post.xtask-test.native-window-acceptance",
            format!("cannot create native validation snapshot: {error}"),
        )
    })?;
    let publisher = evidence::EvidencePublisher::open(directory.path()).map_err(|error| {
        CiError::new(
            "tests.post.xtask-test.native-window-acceptance",
            format!("cannot open native validation snapshot: {error}"),
        )
    })?;
    for file in snapshot.files() {
        publisher
            .replace(&file.relative_path, &file.bytes)
            .map_err(|error| {
                CiError::new(
                    "tests.post.xtask-test.native-window-acceptance",
                    format!("cannot materialize {}: {error}", file.relative_path),
                )
            })?;
    }
    Ok(directory)
}

fn retain_native_acceptance_files(
    session: &mut RunSession,
    snapshot: &evidence::EvidenceSnapshot,
    role: &str,
    gate: &Gate,
    step: &Step,
    contract: &str,
) -> Result<(), CiError> {
    session
        .publish_snapshot_entries(
            &snapshot.files(),
            "payloads/native-window.acceptance",
            role,
            "application/octet-stream",
            &gate.id,
            &step.command,
        )
        .map_err(|error| CiError::new(contract, error.detail))
}

fn retain_subordinate_outputs(
    session: &mut RunSession,

    gate: &Gate,
    step: &Step,
    output_root: &Path,
    expected_names: &[&str],
    step_succeeded: bool,
) -> Result<(), CiError> {
    if expected_names.is_empty() {
        return Ok(());
    }
    let failure_contract = format!("{}.post.{}.subordinate-output", gate.id, step.id);
    let snapshot = evidence::EvidenceSnapshot::open(output_root).map_err(|error| {
        CiError::new(
            failure_contract.clone(),
            format!("cannot snapshot {}: {error}", output_root.display()),
        )
    })?;
    let files = snapshot.files();
    let actual_names: BTreeSet<_> = files
        .iter()
        .map(|file| file.relative_path.clone())
        .collect();
    let expected: BTreeSet<String> = expected_names
        .iter()
        .map(|name| (*name).to_string())
        .collect();
    if !actual_names.is_subset(&expected) || (step_succeeded && actual_names != expected) {
        return Err(CiError::new(
            failure_contract.clone(),
            format!("subordinate output set differs: expected {expected:?}, got {actual_names:?}"),
        ));
    }
    session
        .publish_snapshot_entries(
            &files,
            &format!("payloads/subordinate.output/{}/{}", gate.id, step.id),
            "subordinate.output",
            "application/octet-stream",
            &gate.id,
            &step.command,
        )
        .map_err(|error| CiError::new(failure_contract, error.detail))
}

fn record_security_advisory_database(
    session: &mut RunSession,
    controller_command: &[String],
) -> Result<(), CiError> {
    let source = session
        .root
        .join("rust")
        .join(&session.authority.security.advisory_database_path);
    let (pack, file_count) = build_advisory_database_pack(
        &source,
        session
            .authority
            .actions
            .evidence_snapshot_member_limit_bytes,
        session
            .authority
            .actions
            .evidence_snapshot_member_limit_bytes,
        session.authority.security.advisory_database_file_count,
    )?;
    if file_count != session.authority.security.advisory_database_file_count
        || evidence::hex_sha256(&pack) != session.authority.security.advisory_database_pack_sha256
    {
        return Err(CiError::new(
            "security.post.database-identity",
            "retained advisory database differs from the authority-pinned content",
        ));
    }
    let destination = session
        .evidence_root
        .join("payloads/security.advisory-database/advisory-database.pack");
    fs::create_dir_all(destination.parent().unwrap()).map_err(|error| {
        CiError::new(
            "security.post.database-retain",
            format!("cannot create advisory database payload directory: {error}"),
        )
    })?;
    publish_advisory_database_pack(&destination, &pack)?;
    let indexed = session.entry_from_evidence_path(
        &destination,
        "security.advisory-database",
        "application/octet-stream",
        "security",
        controller_command,
    );
    if indexed.is_err() {
        let _ = fs::remove_file(&destination);
    }
    indexed
}

fn publish_advisory_database_pack(destination: &Path, pack: &[u8]) -> Result<(), CiError> {
    let parent = destination.parent().ok_or_else(|| {
        CiError::new(
            "security.post.database-retain",
            format!(
                "advisory database destination has no parent: {}",
                destination.display()
            ),
        )
    })?;
    let mut staged = tempfile::NamedTempFile::new_in(parent).map_err(|error| {
        CiError::new(
            "security.post.database-retain",
            format!("cannot stage advisory database: {error}"),
        )
    })?;
    staged.write_all(pack).map_err(|error| {
        CiError::new(
            "security.post.database-retain",
            format!("cannot write staged advisory database: {error}"),
        )
    })?;
    staged.as_file().sync_all().map_err(|error| {
        CiError::new(
            "security.post.database-retain",
            format!("cannot sync staged advisory database: {error}"),
        )
    })?;
    staged.persist_noclobber(destination).map_err(|error| {
        CiError::new(
            "security.post.database-retain",
            format!("cannot publish advisory database: {}", error.error),
        )
    })?;
    Ok(())
}

fn build_advisory_database_pack(
    source: &Path,
    member_limit: u64,
    aggregate_limit: u64,
    file_limit: u32,
) -> Result<(Vec<u8>, u32), CiError> {
    let mut files = Vec::new();
    collect_advisory_database_files(source, source, &mut files)?;
    files.sort();
    let file_count = u32::try_from(files.len()).map_err(|_| {
        CiError::new(
            "security.post.database-retain",
            "advisory database contains too many files",
        )
    })?;
    if file_count > file_limit {
        return Err(CiError::new(
            "security.post.database-retain",
            "advisory database exceeds its file-count limit",
        ));
    }
    let mut pack = b"UQM-S4-ADVISORY-DB-V1\0".to_vec();
    for relative in &files {
        let relative_text = relative.to_str().ok_or_else(|| {
            CiError::new(
                "security.post.database-retain",
                format!("advisory path is not UTF-8: {}", relative.display()),
            )
        })?;
        if !evidence::validate_relative_path(relative_text) {
            return Err(CiError::new(
                "security.post.database-retain",
                format!("invalid advisory path '{relative_text}'"),
            ));
        }
        let path_bytes = relative_text.as_bytes();
        let path_length = u32::try_from(path_bytes.len()).map_err(|_| {
            CiError::new(
                "security.post.database-retain",
                format!("advisory path is too long: {relative_text}"),
            )
        })?;
        let bytes = super::bounded_io::read_regular_nofollow(&source.join(relative), member_limit)
            .map_err(|error| CiError::new("security.post.database-retain", error))?;
        let record_length = 12_u64
            .checked_add(u64::from(path_length))
            .and_then(|length| length.checked_add(bytes.len() as u64))
            .ok_or_else(|| {
                CiError::new(
                    "security.post.database-retain",
                    "advisory database pack length overflow",
                )
            })?;
        if (pack.len() as u64)
            .checked_add(record_length)
            .is_none_or(|length| length > aggregate_limit)
        {
            return Err(CiError::new(
                "security.post.database-retain",
                "advisory database exceeds its aggregate byte limit",
            ));
        }
        pack.extend_from_slice(&path_length.to_be_bytes());
        pack.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
        pack.extend_from_slice(path_bytes);
        pack.extend_from_slice(&bytes);
    }
    pack.extend_from_slice(&0_u32.to_be_bytes());
    Ok((pack, file_count))
}

fn collect_advisory_database_files(
    root: &Path,
    directory: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), CiError> {
    let mut entries = fs::read_dir(directory)
        .map_err(|error| {
            CiError::new(
                "security.post.database-retain",
                format!("cannot read {}: {error}", directory.display()),
            )
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| CiError::new("security.post.database-retain", error.to_string()))?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).map_err(|error| {
            CiError::new(
                "security.post.database-retain",
                format!("cannot inspect {}: {error}", path.display()),
            )
        })?;
        if entry.file_name() == ".git" {
            if !metadata.is_dir() || metadata.file_type().is_symlink() {
                return Err(CiError::new(
                    "security.post.database-retain",
                    "advisory database .git metadata is not a real directory",
                ));
            }
            continue;
        }
        if metadata.file_type().is_symlink() {
            return Err(CiError::new(
                "security.post.database-retain",
                format!("advisory database contains symlink {}", path.display()),
            ));
        }
        if metadata.is_dir() {
            collect_advisory_database_files(root, &path, files)?;
        } else if metadata.is_file() {
            files.push(path.strip_prefix(root).unwrap().to_path_buf());
        } else {
            return Err(CiError::new(
                "security.post.database-retain",
                format!(
                    "advisory database contains non-regular input {}",
                    path.display()
                ),
            ));
        }
    }
    Ok(())
}

fn validate_and_record_package(session: &mut RunSession, gate: &Gate) -> Result<(), CiError> {
    let package_step = gate
        .steps
        .iter()
        .find(|step| step.id == "xtask-package")
        .ok_or_else(|| CiError::new("package.authority", "package step is missing"))?;
    let manifest_path = session.root.join("rust/target/production-artifacts.json");
    crate::verify_artifact_manifest_with_authority(&session.root, &session.authority)
        .map_err(|error| CiError::new("package.post.manifest-verify", error))?;
    let manifest: crate::ArtifactManifest = read_json_contract(
        &manifest_path,
        "package.post.manifest-read",
        session
            .authority
            .actions
            .evidence_snapshot_member_limit_bytes,
    )?;
    session
        .entry_from_file(
            &manifest_path,
            "package-manifest",
            "application/json",
            &gate.id,
            &package_step.command,
        )
        .map_err(|error| CiError::new("package.post.manifest-retain", error))?;

    for artifact in manifest.artifacts {
        let path = session.root.join(&artifact.path);
        let producing_command: Vec<String> = artifact
            .producing_command
            .split_whitespace()
            .map(str::to_string)
            .collect();
        let contract = format!("package.post.artifact.{}", artifact.role);
        session
            .entry_from_file(
                &path,
                &format!("package-{}", artifact.role),
                &artifact.media_type,
                &gate.id,
                &producing_command,
            )
            .map_err(|error| CiError::new(contract, error))?;
    }

    let ownership_step = gate
        .steps
        .iter()
        .find(|step| step.id == "verify-production-ownership")
        .ok_or_else(|| {
            CiError::new(
                "package.authority",
                "ownership verification step is missing",
            )
        })?;
    session
        .entry_from_file(
            &session
                .root
                .join("rust/target/ownership-production-report.json"),
            "ownership-production-report",
            "application/json",
            &gate.id,
            &ownership_step.command,
        )
        .map_err(|error| CiError::new("package.post.ownership-report", error))?;

    let capture_step = gate
        .steps
        .iter()
        .find(|step| step.id == "capture-native-dependencies")
        .ok_or_else(|| CiError::new("package.authority", "dependency capture step is missing"))?;
    let candidate_path = session.root.join(format!(
        "rust/target/native-dependencies-{}.candidate.json",
        session.tuple
    ));
    if let Err(error) = validate_dependency_capture(
        &session.root,
        &candidate_path,
        &session.tuple,
        session
            .authority
            .actions
            .evidence_snapshot_member_limit_bytes,
    ) {
        session
            .entry_from_file(
                &candidate_path,
                "native-dependency-capture",
                "application/json",
                &gate.id,
                &capture_step.command,
            )
            .map_err(|retain_error| {
                CiError::new("package.post.dependencies-retain", retain_error)
            })?;
        return Err(CiError::new(
            "package.post.dependencies-validate",
            error.detail,
        ));
    }
    session
        .entry_from_file(
            &candidate_path,
            "native-dependency-capture",
            "application/json",
            &gate.id,
            &capture_step.command,
        )
        .map_err(|error| CiError::new("package.post.dependencies-retain", error))?;
    Ok(())
}

fn read_json_contract<T: serde::de::DeserializeOwned>(
    path: &Path,
    contract: &str,
    limit: u64,
) -> Result<T, CiError> {
    let bytes = super::bounded_io::read_regular_nofollow(path, limit)
        .map_err(|error| CiError::new(contract, error))?;
    serde_json::from_slice(&bytes).map_err(|error| {
        CiError::new(
            contract,
            format!("invalid JSON {}: {error}", path.display()),
        )
    })
}

fn validate_dependency_capture(
    root: &Path,
    candidate_path: &Path,
    tuple: &str,
    limit: u64,
) -> Result<(), CiError> {
    let authority: serde_json::Value = read_json_contract(
        &root.join("rust/build/native-dependencies.json"),
        "package.dependencies",
        limit,
    )?;
    let candidate: serde_json::Value =
        read_json_contract(candidate_path, "package.dependencies", limit)?;
    if candidate.get("target").and_then(serde_json::Value::as_str) != Some(tuple) {
        return Err(CiError::new(
            "package.dependencies",
            format!("dependency capture target must be '{tuple}'"),
        ));
    }

    let expected: BTreeSet<_> = authority
        .get("dependencies")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| CiError::new("package.dependencies", "authority dependencies are missing"))?
        .iter()
        .filter(|entry| {
            entry
                .get("targets")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|targets| targets.iter().any(|target| target.as_str() == Some(tuple)))
        })
        .filter_map(|entry| entry.get("path").and_then(serde_json::Value::as_str))
        .collect();
    let observed: BTreeSet<_> = candidate
        .get("dependencies")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| CiError::new("package.dependencies", "capture dependencies are missing"))?
        .iter()
        .filter_map(serde_json::Value::as_str)
        .collect();
    if expected != observed {
        return Err(CiError::new(
            "package.dependencies",
            format!(
                "captured native dependency inventory differs for {tuple}: expected {}, observed {}",
                expected.len(),
                observed.len()
            ),
        ));
    }
    Ok(())
}

pub(super) fn write_captured(
    session: &mut RunSession,
    gate: &Gate,
    step_id: &str,
    command: &[String],
    effective_command: &[String],
    staged_script_sha256: Option<&str>,
    captured: &super::exec::Captured,
) -> Result<(), CiError> {
    let base = format!("{}/{}", gate.id, step_id);
    let stdout_abs = session.evidence_root.join(format!("{base}.stdout.log"));
    let stderr_abs = session.evidence_root.join(format!("{base}.stderr.log"));
    let result_abs = session.evidence_root.join(format!("{base}.result.json"));
    fs::write(&stdout_abs, &captured.stdout)
        .map_err(|error| CiError::new(format!("{}.evidence", gate.id), error.to_string()))?;
    fs::write(&stderr_abs, &captured.stderr)
        .map_err(|error| CiError::new(format!("{}.evidence", gate.id), error.to_string()))?;
    let result = serde_json::json!({
        "schema": "uqm-s4-step-result-v2",
        "gate": gate.id,
        "step": step_id,
        "command": command,
        "effective_command": effective_command,
        "staged_script_sha256": staged_script_sha256,
        "executable_identity": captured.executable_identity,
        "exit_code": captured.exit_code,
        "signal": captured.signal,
        "launch_error": captured.launch_error,
        "success": captured.succeeded(),
        "supervision": {
            "timeout_milliseconds": captured.limits.timeout.as_millis(),
            "termination_grace_milliseconds": captured.limits.termination_grace.as_millis(),
            "pipe_drain_timeout_milliseconds": captured.limits.pipe_drain_timeout.as_millis(),
            "stdout_limit_bytes": captured.limits.stdout_bytes,
            "stderr_limit_bytes": captured.limits.stderr_bytes,
            "stdout_bytes_seen": captured.stdout_bytes_seen,
            "stderr_bytes_seen": captured.stderr_bytes_seen,
            "stdout_truncated": captured.stdout_truncated,
            "stderr_truncated": captured.stderr_truncated,
            "timed_out": captured.timed_out,
            "termination_reason": captured.termination_reason,
            "termination_signal": captured.termination_signal,
            "process_group_cleanup": captured.process_group_cleanup,
            "pipe_cleanup": captured.pipe_cleanup,
            "error": captured.supervision_error,
        },
    });
    fs::write(
        &result_abs,
        serde_json::to_vec_pretty(&result)
            .map_err(|error| CiError::new(format!("{}.evidence", gate.id), error.to_string()))?,
    )
    .map_err(|error| CiError::new(format!("{}.evidence", gate.id), error.to_string()))?;
    session.entry_from_evidence_path(
        &stdout_abs,
        "step.stdout",
        "text/plain",
        &gate.id,
        command,
    )?;
    session.entry_from_evidence_path(
        &stderr_abs,
        "step.stderr",
        "text/plain",
        &gate.id,
        command,
    )?;
    session.entry_from_evidence_path(
        &result_abs,
        "step.result",
        "application/json",
        &gate.id,
        command,
    )?;
    Ok(())
}

fn execute_builtin_gate(
    session: &mut RunSession,
    cache: &CacheEnvironment,
    gate: &Gate,
) -> Result<(), CiError> {
    match gate.id.as_str() {
        "complexity" => complexity_gate(session, gate),
        "coverage" => coverage_gate(session, cache, gate),
        "workflow" => super::workflow::workflow_gate(session, gate),
        "mutations" => super::mutations::mutations_gate(session, gate),
        "bootstrap-proof" => {
            let root = session.root.clone();
            super::proof::run_bootstrap_proof(&root, session, cache)
        }
        other => Err(CiError::new(
            "authority.gate",
            format!("builtin gate '{other}' has no executor"),
        )),
    }
}

/// Complexity gate: tracked first-party Rust sources via `git ls-files`,
/// including `rust/build.rs` and `rast/src/io/uio_bridge.rs`, maximum 40,
/// fail fast.
fn complexity_gate(session: &mut RunSession, gate: &Gate) -> Result<(), CiError> {
    let root = session.root.clone();
    let files = tracked_first_party_sources(&root, &session.authority)?;
    for required in ["rust/build.rs", "rast/src/io/uio_bridge.rs"] {
        if !files.contains(required) {
            return Err(CiError::new(
                "complexity.sources",
                format!("required first-party source '{required}' is not tracked"),
            ));
        }
    }
    let arguments = session
        .authority
        .complexity
        .lizard_arguments
        .iter()
        .cloned()
        .chain(files)
        .collect::<Vec<_>>();
    let producing_command = std::iter::once("lizard".to_string())
        .chain(arguments.iter().cloned())
        .collect::<Vec<_>>();
    let captured = run_captured(
        &root,
        "lizard",
        &arguments,
        &[],
        session.authority.supervision.builtin_limits(),
    );
    write_captured(
        session,
        gate,
        "lizard",
        &producing_command,
        &producing_command,
        None,
        &captured,
    )?;
    if let Some(error) = captured.launch_error {
        return Err(CiError::new("complexity.exec", error));
    }
    if !captured.succeeded() {
        return Err(CiError::new(
            "complexity.maximum",
            format!(
                "lizard rejected tracked Rust source at configured maximum {} with {:?}",
                session.authority.complexity.maximum, captured.exit_code
            ),
        ));
    }
    Ok(())
}

fn tracked_first_party_sources(
    root: &Path,
    authority: &Authority,
) -> Result<BTreeSet<String>, CiError> {
    let arguments = vec![
        "-c".to_string(),
        format!("safe.directory={}", root.display()),
        "ls-files".to_string(),
    ];
    let captured = run_captured(
        root,
        "git",
        &arguments,
        &[],
        authority.supervision.builtin_limits(),
    );
    if !captured.succeeded() {
        return Err(CiError::new(
            "complexity.sources",
            captured.failure_detail("git ls-files"),
        ));
    }
    let text = String::from_utf8(captured.stdout)
        .map_err(|error| CiError::new("complexity.sources", error.to_string()))?;
    let roots: BTreeSet<&String> = authority.complexity.source_roots.iter().collect();
    Ok(text
        .lines()
        .filter(|path| path.ends_with(".rs"))
        .filter(|path| roots.iter().any(|root| path.starts_with(root.as_str())))
        .map(str::to_string)
        .collect())
}

/// Coverage gate: 80% line floor through cargo llvm-cov, report bytes written.
fn coverage_gate(
    session: &mut RunSession,
    cache: &CacheEnvironment,
    gate: &Gate,
) -> Result<(), CiError> {
    let root = session.root.clone();
    let coverage = session.authority.coverage.clone();
    let output_path = session.evidence_root.join("coverage.lcov");
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| CiError::new("coverage.write", error.to_string()))?;
    }
    let arguments = vec![
        "llvm-cov".to_string(),
        "--manifest-path".into(),
        "rust/Cargo.toml".into(),
        "--workspace".into(),
        "--all-targets".into(),
        "--no-default-features".into(),
        "--features".into(),
        session.authority.profiles.linked_test.join(","),
        "--lcov".into(),
        "--output-path".into(),
        output_path.display().to_string(),
        "--ignore-filename-regex".into(),
        coverage.ignore_filename_regex.clone(),
    ];
    let producing_command = std::iter::once("cargo".to_string())
        .chain(arguments.iter().cloned())
        .collect::<Vec<_>>();
    let mut environment = cache.resolved().vars;
    environment.extend(
        session
            .authority
            .linked_step_env()
            .map_err(|error| CiError::new("coverage.toolchain", error))?,
    );
    let captured = run_captured(
        &root,
        "cargo",
        &arguments,
        &environment,
        session.authority.supervision.builtin_limits(),
    );
    write_captured(
        session,
        gate,
        "llvm-cov",
        &producing_command,
        &producing_command,
        None,
        &captured,
    )?;
    if let Some(error) = captured.launch_error {
        return Err(CiError::new("coverage.exec", error));
    }
    if !captured.succeeded() {
        return Err(CiError::new(
            "coverage.exec",
            format!("cargo llvm-cov failed with {:?}", captured.exit_code),
        ));
    }
    let bytes = super::bounded_io::read_regular_nofollow(
        &output_path,
        session
            .authority
            .actions
            .evidence_snapshot_member_limit_bytes,
    )
    .map_err(|error| CiError::new("coverage.read", error))?;
    session.publish_entry_bytes(
        "payloads/coverage.lcov/coverage.lcov",
        &bytes,
        "coverage.lcov",
        "text/plain",
        "coverage",
        &producing_command,
    )?;
    let percent =
        lcov_line_coverage(&bytes).map_err(|error| CiError::new("coverage.parse", error))?;
    if percent < coverage.minimum_line_percent {
        return Err(CiError::new(
            "coverage.floor",
            format!(
                "line coverage {percent:.2}% is below the {:.0}% floor",
                coverage.minimum_line_percent
            ),
        ));
    }
    Ok(())
}

/// Compute overall line coverage percentage from lcov `LF`/`LH` records.
pub fn lcov_line_coverage(bytes: &[u8]) -> Result<f64, String> {
    let text = std::str::from_utf8(bytes).map_err(|error| format!("lcov is not UTF-8: {error}"))?;
    let mut found = 0_u64;
    let mut hit = 0_u64;
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("LF:") {
            found += value
                .parse::<u64>()
                .map_err(|error| format!("malformed lcov LF: {error}"))?;
        } else if let Some(value) = line.strip_prefix("LH:") {
            hit += value
                .parse::<u64>()
                .map_err(|error| format!("malformed lcov LH: {error}"))?;
        }
    }
    if found == 0 {
        return Err("lcov report contains no line records".into());
    }
    Ok(hit as f64 * 100.0 / found as f64)
}

fn write_index(path: &Path, index: &EvidenceIndex) -> Result<(), String> {
    let mut bytes = serde_json::to_vec_pretty(index)
        .map_err(|error| format!("cannot serialize evidence index: {error}"))?;
    bytes.push(b'\n');
    let root = path
        .parent()
        .ok_or_else(|| format!("evidence index has no parent: {}", path.display()))?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("evidence index name is not UTF-8: {}", path.display()))?;
    let publisher = evidence::EvidencePublisher::open(root)
        .map_err(|error| format!("cannot open {}: {error}", root.display()))?;
    publisher
        .create(name, &bytes)
        .map_err(|error| format!("cannot write {}: {error}", path.display()))
}

/// Lexically relative path; descriptor-based reads enforce containment and file type.
pub fn relative_evidence_path(root: &Path, path: &Path) -> Result<String, CiError> {
    let relative = path.strip_prefix(root).map_err(|_| {
        CiError::new(
            "evidence.path",
            format!("evidence is outside the bundle: {}", path.display()),
        )
    })?;
    let relative = relative.to_str().ok_or_else(|| {
        CiError::new(
            "evidence.path",
            format!("evidence path is not UTF-8: {}", path.display()),
        )
    })?;
    if !evidence::validate_relative_path(relative) {
        return Err(CiError::new(
            "evidence.path",
            format!("evidence path is not normalized: {}", path.display()),
        ));
    }
    Ok(relative.to_string())
}

fn fresh_evidence_root(root: &Path) -> Result<PathBuf, String> {
    let base = env::var_os("UQM_CI_EVIDENCE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("rust/target/ci-evidence"));
    fs::create_dir_all(&base)
        .map_err(|error| format!("cannot create {}: {error}", base.display()))?;
    for index in 0..10_000u32 {
        let candidate = base.join(format!("run-{index}"));
        match fs::create_dir(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(format!("cannot create {}: {error}", candidate.display())),
        }
    }
    Err(format!("no free evidence slot under {}", base.display()))
}

fn git_status_empty(
    root: &Path,
    supervision: &super::authority::SupervisionAuthority,
) -> Result<bool, String> {
    let arguments = [
        "-c".to_string(),
        format!("safe.directory={}", root.display()),
        "status".to_string(),
        "--porcelain=v1".to_string(),
        "--untracked-files=all".to_string(),
        "-z".to_string(),
    ];
    let captured = run_git_captured(root, &arguments, supervision.builtin_limits());
    if !captured.succeeded() {
        return Err(captured.failure_detail("git status"));
    }
    Ok(captured.stdout.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cargo_commands_are_not_rewritten_with_unsupported_target_arguments() {
        for command in [
            vec!["cargo", "fmt", "--all", "--", "--check"],
            vec!["cargo", "audit", "--locked", "--no-fetch"],
            vec!["cargo", "clippy", "--locked", "--", "-D", "warnings"],
        ] {
            assert!(!command.contains(&"--target-dir"));
        }
    }

    #[test]
    fn only_unreplaced_cargo_processes_retain_the_resolved_path() {
        let cargo = ["cargo".to_string(), "check".to_string()];
        let controller = [
            "cargo".to_string(),
            "run".to_string(),
            "--locked".to_string(),
            "--manifest-path".to_string(),
            "rust/xtask/Cargo.toml".to_string(),
            "--".to_string(),
            "test".to_string(),
        ];
        let shell = ["bash".to_string(), "script.sh".to_string()];

        assert!(process_command_requires_retained_source(&cargo));
        assert_eq!(trusted_controller_command(&controller), Some("__ci-test"));
        assert!(!process_command_requires_retained_source(&controller));
        assert!(!process_command_requires_retained_source(&shell));
    }

    #[test]
    fn staged_python_snippets_disable_current_directory_imports() {
        for command in [
            [
                "bash".to_string(),
                "rust/probes/run_p00_probes.sh".to_string(),
            ],
            [
                "bash".to_string(),
                "rust/harness/run_p00_harness.sh".to_string(),
            ],
            [
                "bash".to_string(),
                "rust/harness/run_menu_binding_probe.sh".to_string(),
            ],
        ] {
            let (name, bytes) = trusted_control_plane_script(&command).unwrap();
            let script = std::str::from_utf8(bytes).unwrap();
            assert!(
                script
                    .lines()
                    .all(|line| !line.contains("python3 -") || line.contains("python3 -P")),
                "{name} permits current-directory Python imports"
            );
        }
    }

    #[test]
    fn trusted_staging_selection_rejects_the_writable_evidence_tree() {
        let evidence = Path::new("/runner-temp/evidence");
        let selected =
            trusted_staging_parent_from(evidence, Some("/runner-temp".into()), true).unwrap();
        assert_eq!(selected, (PathBuf::from("/runner-temp"), true));
        assert!(trusted_staging_parent_from(
            evidence,
            Some("/runner-temp/evidence/staging".into()),
            true,
        )
        .is_err());
        assert!(trusted_staging_parent_from(evidence, None, true).is_err());
    }

    #[test]
    fn runtime_bindings_keep_source_authority_and_git_trust_distinct() {
        let source = Path::new("/exact-head");
        let bindings =
            runtime_binding_environment_from(source, true, Some("/base-owned/gates.json".into()))
                .unwrap()
                .into_iter()
                .collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(bindings["UQM_CI_SOURCE_ROOT"], "/exact-head");
        assert_eq!(bindings["UQM_CI_AUTHORITY_PATH"], "/base-owned/gates.json");
        assert_eq!(bindings["GIT_CONFIG_COUNT"], "1");
        assert_eq!(bindings["GIT_CONFIG_KEY_0"], "safe.directory");
        assert_eq!(bindings["GIT_CONFIG_VALUE_0"], "/exact-head");
        assert!(runtime_binding_environment_from(source, true, None).is_err());
    }

    #[test]
    fn subordinate_evidence_environment_binds_root_and_authority_member_limit() {
        let authority: Authority =
            serde_json::from_str(include_str!("../../../ci/gates.json")).unwrap();
        let root = Path::new("/tmp/uqm-subordinate-evidence-test");
        let environment = subordinate_evidence_environment(&authority, root)
            .into_iter()
            .collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(environment.len(), 2);
        assert_eq!(
            environment.get("UQM_CI_SUBORDINATE_EVIDENCE_ROOT"),
            Some(&root.display().to_string())
        );
        assert_eq!(
            environment.get("UQM_CI_EVIDENCE_MEMBER_LIMIT_BYTES"),
            Some(
                &authority
                    .actions
                    .evidence_snapshot_member_limit_bytes
                    .to_string()
            )
        );
    }

    #[test]
    fn subordinate_gate_scripts_are_compiled_into_the_trusted_controller() {
        for (path, marker) in [
            ("rust/probes/run_p00_probes.sh", "P00 Preflight Probes"),
            (
                "rust/harness/run_p00_harness.sh",
                "P00 Linked Harness Probe",
            ),
            (
                "rust/harness/run_menu_binding_probe.sh",
                "Menu Binding Probe",
            ),
            (
                "rust/ownership/verify-fixture.sh",
                "ownership-strict-link-fixture",
            ),
            (
                "rust/ownership/verify-production.sh",
                "__ci-ownership-production",
            ),
        ] {
            let command = vec!["bash".to_string(), path.to_string()];
            let (_, bytes) = trusted_control_plane_script(&command).unwrap();
            let script = std::str::from_utf8(bytes).unwrap();

            assert!(script.contains(marker));
            assert!(script.contains("UQM_CI_SOURCE_ROOT"));
        }
        assert!(
            trusted_control_plane_script(&["bash".into(), "rust/probes/untrusted.sh".into(),])
                .is_none()
        );
    }
    #[test]
    fn head_xtask_gate_vectors_are_rewritten_only_when_exact() {
        for (command, hidden) in [
            ("test", "__ci-test"),
            ("package", "__ci-package"),
            ("capture-dependencies", "__ci-capture-dependencies"),
        ] {
            let vector = vec![
                "cargo".into(),
                "run".into(),
                "--locked".into(),
                "--manifest-path".into(),
                "rust/xtask/Cargo.toml".into(),
                "--".into(),
                command.into(),
            ];
            assert_eq!(trusted_controller_command(&vector), Some(hidden));
            let gate = tempfile::tempdir().unwrap();
            let source = tempfile::tempdir().unwrap();
            let head_xtask = source.path().join("rust/xtask/src");
            std::fs::create_dir_all(&head_xtask).unwrap();
            std::fs::write(
                head_xtask.join("main.rs"),
                "fn main() { std::fs::write(\"forged\", \"success\").unwrap(); }\n",
            )
            .unwrap();
            let (effective, directory, digest) = stage_trusted_control_plane_script(
                &vector,
                gate.path(),
                source.path(),
                1024 * 1024,
                &mut Vec::new(),
            )
            .unwrap();
            assert_eq!(
                effective,
                vec![
                    std::env::current_exe()
                        .unwrap()
                        .to_string_lossy()
                        .into_owned(),
                    hidden.to_string(),
                ]
            );
            assert!(directory.is_none());
            assert!(digest.is_none());
            let mut near_miss = vector.clone();
            near_miss.push("forged".into());
            assert_eq!(trusted_controller_command(&near_miss), None);
            let gate = tempfile::tempdir().unwrap();
            let source = tempfile::tempdir().unwrap();
            let error = stage_trusted_control_plane_script(
                &near_miss,
                gate.path(),
                source.path(),
                1024 * 1024,
                &mut Vec::new(),
            )
            .unwrap_err();
            assert_eq!(error.contract, "trusted-controller-command");
        }
    }

    #[test]
    fn trusted_script_staging_ignores_source_control_plane_bytes() {
        let root = tempfile::tempdir().unwrap();
        let gate = tempfile::tempdir().unwrap();
        for path in [
            "rust/probes/run_p00_probes.sh",
            "rust/harness/run_p00_harness.sh",
            "rust/harness/run_menu_binding_probe.sh",
            "rust/ownership/verify-fixture.sh",
            "rust/ownership/verify-production.sh",
        ] {
            let source_path = root.path().join(path);
            fs::create_dir_all(source_path.parent().unwrap()).unwrap();
            fs::write(&source_path, b"#!/bin/sh\necho head-owned-mutant\n").unwrap();
            let command = vec!["bash".to_string(), path.to_string()];
            let mut environment = Vec::new();
            let (effective, directory, staged_script_sha256) = stage_trusted_control_plane_script(
                &command,
                gate.path(),
                root.path(),
                512 * 1024 * 1024,
                &mut environment,
            )
            .unwrap();
            let staged = fs::read(&effective[1]).unwrap();
            let (_, embedded) = trusted_control_plane_script(&command).unwrap();
            assert_eq!(
                staged_script_sha256,
                Some(super::evidence::hex_sha256(embedded))
            );
            assert_eq!(staged, embedded);
            assert_ne!(staged, fs::read(&source_path).unwrap());
            assert_eq!(environment.len(), 2);
            assert_eq!(
                environment
                    .iter()
                    .find(|(name, _)| name == "UQM_CI_SOURCE_ROOT")
                    .map(|(_, value)| value.as_str()),
                Some(root.path().to_string_lossy().as_ref())
            );
            let controller = environment
                .iter()
                .find(|(name, _)| name == "UQM_CI_CONTROLLER_EXECUTABLE")
                .map(|(_, value)| PathBuf::from(value))
                .unwrap();
            assert_eq!(
                fs::read(&controller).unwrap(),
                fs::read(std::env::current_exe().unwrap()).unwrap()
            );
            assert_eq!(
                std::os::unix::fs::PermissionsExt::mode(
                    &fs::metadata(&controller).unwrap().permissions(),
                ) & 0o777,
                0o550
            );
            use std::os::unix::fs::PermissionsExt as _;
            let directory = directory.unwrap();
            let expected_sha256 = staged_script_sha256.as_deref().unwrap();
            verify_trusted_control_plane_directory(
                directory.path(),
                Path::new(&effective[1]),
                expected_sha256,
            )
            .unwrap();
            std::fs::set_permissions(&effective[1], std::fs::Permissions::from_mode(0o640))
                .unwrap();
            fs::write(&effective[1], b"#!/bin/sh\necho substituted\n").unwrap();
            std::fs::set_permissions(&effective[1], std::fs::Permissions::from_mode(0o440))
                .unwrap();
            let error = verify_trusted_control_plane_directory(
                directory.path(),
                Path::new(&effective[1]),
                expected_sha256,
            )
            .unwrap_err();
            assert_eq!(error.contract, "trusted-control-plane-integrity");
            std::fs::set_permissions(&effective[1], std::fs::Permissions::from_mode(0o640))
                .unwrap();
            let error = verify_trusted_control_plane_directory(
                directory.path(),
                Path::new(&effective[1]),
                expected_sha256,
            )
            .unwrap_err();
            assert_eq!(error.contract, "trusted-control-plane-integrity");
            drop(directory);
        }
        let mut environment = Vec::new();
        let error = stage_trusted_control_plane_script(
            &["bash".into(), "rust/ownership/untrusted.sh".into()],
            gate.path(),
            root.path(),
            512 * 1024 * 1024,
            &mut environment,
        )
        .unwrap_err();
        assert_eq!(error.contract, "trusted-script");
    }

    #[test]
    fn pre_session_failure_classification_is_exact_and_bounded() {
        assert_eq!(
            classify_pre_session_failure("source.head: cannot read HEAD"),
            ("source.head", "cannot read HEAD".to_string())
        );

        assert_eq!(
            classify_pre_session_failure("contract 'authority.gate': unknown gate"),
            ("authority.gate", "unknown gate".to_string())
        );
        assert_eq!(
            classify_pre_session_failure("contract 'cache.mode': unsupported"),
            ("cache.mode", "unsupported".to_string())
        );
        assert_eq!(
            classify_pre_session_failure("unclassified operational failure"),
            (
                "evidence.finalize",
                "unclassified operational failure".to_string()
            )
        );
    }

    #[test]
    fn pre_session_fallback_selection_never_reuses_existing_runs() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir(temp.path().join("pre-session-run-0")).unwrap();
        fs::create_dir(temp.path().join("pre-session-run-1")).unwrap();
        assert_eq!(
            fresh_pre_session_evidence_root(temp.path()),
            temp.path().join("pre-session-run-2")
        );
    }

    #[test]
    fn configured_bundle_root_places_pre_session_runs_beside_the_bundle() {
        let configured = PathBuf::from("/tmp/uqm-evidence/controller/bundle");
        assert_eq!(
            pre_session_evidence_base(Some(configured)),
            PathBuf::from("/tmp/uqm-evidence/controller")
        );
        let configured = PathBuf::from("/tmp/uqm-evidence/controller");
        assert_eq!(
            pre_session_evidence_base(Some(configured.clone())),
            configured
        );
    }

    #[test]
    fn run_failure_retention_uses_fallback_and_does_not_wrap_existing_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let blocker = temp.path().join("not-a-directory");
        fs::write(&blocker, b"blocker").unwrap();
        let fallback = temp.path().join("fallback");
        let retained = retain_run_failure(
            repository,
            "unknown-gate",
            "contract 'authority.gate': unknown gate".to_string(),
            blocker.join("bundle"),
            &fallback,
        );
        assert!(retained.contains("pre-session contract 'authority.gate'"));
        let evidence_path = retained.split("; evidence: ").nth(1).unwrap();
        assert!(Path::new(evidence_path).is_file());
        assert!(evidence_path.starts_with(fallback.to_str().unwrap()));

        let existing =
            "ci run failed at first contract 'gate'; evidence: /tmp/already-retained.json"
                .to_string();
        assert_eq!(
            retain_run_failure(
                repository,
                "all",
                existing.clone(),
                temp.path().join("unused"),
                temp.path(),
            ),
            existing
        );

        let spoofed = "gate detail; evidence: /tmp/not-retained.json".to_string();
        let wrapped = retain_run_failure(
            repository,
            "all",
            spoofed.clone(),
            temp.path().join("spoofed"),
            temp.path(),
        );
        assert_ne!(wrapped, spoofed);
        assert!(wrapped.contains("pre-session contract"));
    }

    #[test]
    fn source_identity_requires_full_lowercase_sha() {
        let valid = "0123456789abcdef0123456789abcdef01234567";
        assert!(validate_source_identity(valid).is_ok());
        assert!(validate_source_identity(&valid[..39]).is_err());
        assert!(validate_source_identity("0123456789ABCDEF0123456789ABCDEF01234567").is_err());
        assert!(validate_source_identity("zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz").is_err());
    }

    #[test]
    fn expected_sha_must_be_valid_and_exact() {
        let head = "0123456789abcdef0123456789abcdef01234567";
        assert!(validate_expected_source_sha(head, None).is_ok());
        assert!(validate_expected_source_sha(head, Some(head)).is_ok());
        assert!(validate_expected_source_sha(
            head,
            Some("1123456789abcdef0123456789abcdef01234567")
        )
        .unwrap_err()
        .contains("differs from UQM_CI_EXPECTED_SHA"));
        assert!(validate_expected_source_sha(head, Some("short"))
            .unwrap_err()
            .contains("invalid UQM_CI_EXPECTED_SHA"));
    }

    #[test]
    fn lcov_coverage_computes_ratio() {
        let mut lcov = String::new();
        for _ in 0..10 {
            lcov.push_str("LF:40\n");
            lcov.push_str("LH:30\n");
        }
        let percent = lcov_line_coverage(lcov.as_bytes()).unwrap();
        assert_eq!(percent, 75.0);
    }

    #[test]
    fn lcov_with_no_records_is_an_error() {
        assert!(lcov_line_coverage(b"SF:foo\n").is_err());
    }

    #[test]
    fn lcov_below_floor_is_rejected_and_above_accepted() {
        let low = lcov_line_coverage(b"LF:100\nLH:30\n").unwrap();
        assert!(low < 80.0);
        let high = lcov_line_coverage(b"LF:100\nLH:90\n").unwrap();
        assert!(high >= 80.0);
    }

    #[test]
    fn relative_evidence_path_yields_a_repository_relative_path() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        let file = root.join("rust/ci/gates.json");
        let relative = relative_evidence_path(&root, &file).unwrap();
        assert_eq!(relative, "rust/ci/gates.json");
    }

    #[test]
    fn advisory_database_pack_is_deterministic_and_excludes_git_metadata() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir_all(temp.path().join(".git")).unwrap();
        fs::create_dir_all(temp.path().join("nested/.git")).unwrap();
        fs::write(temp.path().join("z.md"), b"z").unwrap();
        fs::write(temp.path().join("nested/a.toml"), b"alpha").unwrap();
        fs::write(temp.path().join(".git/config"), b"excluded").unwrap();
        fs::write(temp.path().join("nested/.git/config"), b"also excluded").unwrap();

        let (actual, count) = build_advisory_database_pack(temp.path(), 1024, 4096, 100).unwrap();
        let mut expected = b"UQM-S4-ADVISORY-DB-V1\0".to_vec();
        for (path, content) in [
            ("nested/a.toml", b"alpha".as_slice()),
            ("z.md", b"z".as_slice()),
        ] {
            expected.extend_from_slice(&(path.len() as u32).to_be_bytes());
            expected.extend_from_slice(&(content.len() as u64).to_be_bytes());
            expected.extend_from_slice(path.as_bytes());
            expected.extend_from_slice(content);
        }
        expected.extend_from_slice(&0_u32.to_be_bytes());
        assert_eq!(count, 2);
        assert_eq!(actual, expected);
        assert_eq!(
            build_advisory_database_pack(temp.path(), 1024, 4096, 100)
                .unwrap()
                .0,
            actual
        );
    }

    #[test]
    fn advisory_database_pack_publication_does_not_overwrite() {
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("advisory-database.pack");
        fs::write(&destination, b"existing").unwrap();
        let error = publish_advisory_database_pack(&destination, b"replacement").unwrap_err();
        assert_eq!(error.contract, "security.post.database-retain");

        assert_eq!(fs::read(&destination).unwrap(), b"existing");
    }

    #[test]
    #[ignore = "requires UQM_TEST_ADVISORY_DATABASE pointing at the pinned checkout"]
    fn advisory_database_pack_matches_pinned_authority() {
        let source = env::var_os("UQM_TEST_ADVISORY_DATABASE")
            .map(PathBuf::from)
            .expect("UQM_TEST_ADVISORY_DATABASE must name the pinned checkout");
        let authority: Authority =
            serde_json::from_slice(include_bytes!("../../../ci/gates.json")).unwrap();
        let (pack, count) = build_advisory_database_pack(
            &source,
            authority.actions.evidence_snapshot_member_limit_bytes,
            authority.actions.evidence_snapshot_member_limit_bytes,
            authority.security.advisory_database_file_count,
        )
        .unwrap();
        assert_eq!(count, authority.security.advisory_database_file_count);
        assert_eq!(
            evidence::hex_sha256(&pack),
            authority.security.advisory_database_pack_sha256
        );
    }

    #[cfg(unix)]
    #[test]
    fn advisory_database_pack_rejects_symlinks() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().unwrap();
        fs::create_dir(temp.path().join(".git")).unwrap();
        fs::write(temp.path().join("real.md"), b"real").unwrap();
        symlink(temp.path().join("real.md"), temp.path().join("linked.md")).unwrap();
        assert_eq!(
            build_advisory_database_pack(temp.path(), 1024, 4096, 100)
                .unwrap_err()
                .contract,
            "security.post.database-retain"
        );
    }

    #[test]
    fn wrong_advisory_database_prevents_audit_command_execution() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let database = root.join("rust/advisory-db");
        fs::create_dir_all(&database).unwrap();
        fs::write(database.join("advisory.md"), b"untrusted advisory bytes\n").unwrap();
        let marker = root.join("audit-executed");
        let mut authority: Authority =
            serde_json::from_slice(include_bytes!("../../../ci/gates.json")).unwrap();
        authority.security.advisory_database_path = "advisory-db".into();
        authority.security.advisory_database_file_count = 1;
        authority.security.advisory_database_pack_sha256 = "0".repeat(64);
        let mut gate = authority.gate("security").unwrap().clone();
        gate.steps.retain(|step| step.id == "cargo-audit");
        gate.steps[0].command = vec![
            "/bin/sh".into(),
            "-c".into(),
            format!("touch {}", marker.display()),
        ];
        let evidence_root = root.join("evidence");
        fs::create_dir(&evidence_root).unwrap();
        let mut session = RunSession {
            root: root.to_path_buf(),
            authority,
            evidence_root,
            tuple: "macos-aarch64".into(),
            cache_mode: cache::AMBIENT_MODE.into(),
            source_sha: "a".repeat(40),
            clean: true,
            features: Vec::new(),
            entries: Vec::new(),
        };
        let cache = CacheEnvironment {
            mode: cache::AMBIENT_MODE.into(),
            cargo_home: root.join("cargo-home"),
            execution_target: root.join("target"),
            receipt: cache::InitialStateReceipt {
                schema: cache::INITIAL_SCHEMA.into(),
                mode: cache::AMBIENT_MODE.into(),
                ambient_cargo_home: root.join("cargo-home").display().to_string(),
                isolation_cargo_home: root.join("cargo-home").display().to_string(),
                execution_target: root.join("target").display().to_string(),
                registry_cache_present: false,
                git_cache_present: false,
                execution_target_absent: true,
                rust_target_present: false,
                sc2_obj_present: false,
                restore_used: false,
                save_used: false,
                first_failed_contract: None,
                passed: true,
            },
        };
        let error =
            execute_process_gate(&mut session, &cache, &gate, &["controller".into()]).unwrap_err();
        assert_eq!(error.contract, "security.post.database-identity");
        assert!(
            !marker.exists(),
            "audit command ran before pack verification"
        );
    }

    #[test]
    fn subordinate_retention_reports_postprocess_failures_without_masking_failed_subsets() {
        let temp = tempfile::tempdir().unwrap();
        let authority: Authority =
            serde_json::from_slice(include_bytes!("../../../ci/gates.json")).unwrap();
        let gate = authority.gate("probes-harnesses").unwrap().clone();
        let step = gate.steps[0].clone();
        let output = temp.path().join("subordinate");
        let evidence_root = temp.path().join("evidence");
        fs::create_dir_all(&output).unwrap();
        fs::create_dir_all(&evidence_root).unwrap();
        let mut session = RunSession {
            root: temp.path().to_path_buf(),
            authority,
            evidence_root,
            tuple: "macos-aarch64".to_string(),
            cache_mode: "ambient-dev".to_string(),
            source_sha: "a".repeat(40),
            clean: true,
            features: Vec::new(),
            entries: Vec::new(),
        };
        let expected = subordinate_output_names(&gate.id, &step.id);
        let expected_contract = format!("{}.post.{}.subordinate-output", gate.id, step.id);

        let missing =
            retain_subordinate_outputs(&mut session, &gate, &step, &output, expected, true)
                .unwrap_err();
        assert_eq!(missing.contract, expected_contract);

        fs::write(output.join("unexpected"), b"unexpected").unwrap();
        let unexpected =
            retain_subordinate_outputs(&mut session, &gate, &step, &output, expected, true)
                .unwrap_err();
        assert_eq!(unexpected.contract, expected_contract);
        fs::remove_file(output.join("unexpected")).unwrap();

        assert!(
            retain_subordinate_outputs(&mut session, &gate, &step, &output, expected, false,)
                .is_ok()
        );

        fs::create_dir(output.join(expected[0])).unwrap();
        let non_regular =
            retain_subordinate_outputs(&mut session, &gate, &step, &output, expected, true)
                .unwrap_err();
        assert_eq!(non_regular.contract, expected_contract);
    }

    #[cfg(unix)]
    #[test]
    fn native_failure_retention_rejects_partial_diagnostics_and_manifest_symlinks() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().unwrap();
        let authority: Authority =
            serde_json::from_slice(include_bytes!("../../../ci/gates.json")).unwrap();
        let gate = authority.gate("tests").unwrap().clone();
        let step = gate
            .steps
            .iter()
            .find(|step| step.id == "xtask-test")
            .unwrap()
            .clone();
        let evidence_root = temp.path().join("evidence");
        let native_root = temp.path().join("native");
        fs::create_dir_all(&evidence_root).unwrap();
        fs::create_dir_all(native_root.join("automation")).unwrap();
        fs::write(
            native_root.join("automation/partial.json"),
            b"partial diagnostics",
        )
        .unwrap();
        let mut session = RunSession {
            root: temp.path().to_path_buf(),
            authority,
            evidence_root,
            tuple: "macos-aarch64".to_string(),
            cache_mode: "ambient-dev".to_string(),
            source_sha: "a".repeat(40),
            clean: true,
            features: Vec::new(),
            entries: Vec::new(),
        };
        let missing_manifest =
            retain_native_acceptance_failure(&mut session, &gate, &step, &native_root).unwrap_err();
        assert_eq!(missing_manifest.contract, "tests.xtask-test");
        assert!(missing_manifest
            .detail
            .contains("native-acceptance-failure.json"));
        assert!(session.entries.is_empty());

        let manifest_path = native_root.join("native-acceptance-failure.json");
        fs::write(&manifest_path, b"not JSON").unwrap();
        let malformed =
            retain_native_acceptance_failure(&mut session, &gate, &step, &native_root).unwrap_err();
        assert_eq!(malformed.contract, "tests.xtask-test");
        assert!(session.entries.is_empty());
        fs::remove_file(&manifest_path).unwrap();

        let target = native_root.join("diagnostic-manifest.json");
        fs::write(&target, b"{}").unwrap();
        symlink(&target, native_root.join("native-acceptance-failure.json")).unwrap();
        let error =
            retain_native_acceptance_failure(&mut session, &gate, &step, &native_root).unwrap_err();
        assert_eq!(error.contract, "tests.xtask-test");
        assert!(session.entries.is_empty());
    }
    #[test]
    fn complete_native_failure_bundle_is_indexed_with_distinct_role_and_identity() {
        use uqm_rust::automation::{
            native_acceptance_failure_inventory, NativeAcceptanceFailureManifest,
            NativeChildCleanupReceipt, NativeProcessIdentity, NativeRetainedInput,
            NativeWindowBounds, NativeWindowConfigFile, NATIVE_ACCEPTANCE_FAILURE_SCHEMA,
            NATIVE_WINDOW_CONFIG_SCHEMA,
        };

        let temp = tempfile::tempdir().unwrap();
        let authority: Authority =
            serde_json::from_slice(include_bytes!("../../../ci/gates.json")).unwrap();
        let gate = authority.gate("tests").unwrap().clone();
        let step = gate
            .steps
            .iter()
            .find(|step| step.id == "xtask-test")
            .unwrap()
            .clone();
        let evidence_root = temp.path().join("evidence");
        let native_root = evidence_root.join("payloads/native-window.acceptance");
        fs::create_dir_all(&native_root).unwrap();
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
            let path = native_root.join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, bytes).unwrap();
        }
        fs::write(
            native_root.join("native-window-proof.json"),
            serde_json::to_vec_pretty(&NativeWindowConfigFile {
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
        let retained = |relative_path: &str, bytes: &[u8]| NativeRetainedInput {
            relative_path: relative_path.to_string(),
            byte_length: bytes.len() as u64,
            sha256: evidence::hex_sha256(bytes),
        };
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
            executable: retained("inputs/uqm", executable_bytes),
            script: retained("inputs/linked-playable-v1.json", script_bytes),
            content_package: retained(
                "inputs/content/packages/uqm-0.8.0-content.uqm",
                content_bytes,
            ),
            runtime_contract: authority.native_runtime_contract(),
            acceptance_policy: authority.native_acceptance.acceptance_policy,
            retained_files: native_acceptance_failure_inventory(
                &native_root,
                authority.native_runtime_contract().inventory_limits,
            )
            .unwrap(),
            child: NativeChildCleanupReceipt {
                process: NativeProcessIdentity {
                    pid: 42,
                    start_time: "1234".to_string(),
                    executable_sha256: evidence::hex_sha256(executable_bytes),
                    nonce: "a".repeat(64),
                },
                exit_code: Some(1),
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
            failure_contract: serde_json::from_value(serde_json::json!("child-exit")).unwrap(),
            error: "native child exited unsuccessfully".to_string(),
            passed: false,
        };
        fs::write(
            native_root.join("native-acceptance-failure.json"),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
        let mut session = RunSession {
            root: temp.path().to_path_buf(),
            authority,
            evidence_root,
            tuple: "macos-aarch64".to_string(),
            cache_mode: "ambient-dev".to_string(),
            source_sha: "a".repeat(40),
            clean: true,
            features: Vec::new(),
            entries: Vec::new(),
        };

        retain_native_acceptance_failure(&mut session, &gate, &step, &native_root).unwrap();
        assert_eq!(session.entries.len(), 6);
        assert!(session.entries.iter().all(|entry| {
            entry.role == "native-window.failure"
                && entry.producing_gate == "tests"
                && entry.producing_command == step.command
                && entry.path.starts_with("payloads/native-window.acceptance/")
        }));
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;

            let outside = temp.path().join("outside-diagnostic");
            fs::write(&outside, b"must not be read").unwrap();
            fs::create_dir_all(native_root.join("automation")).unwrap();
            symlink(&outside, native_root.join("automation/unsafe-link")).unwrap();
            let error = retain_native_acceptance_failure(&mut session, &gate, &step, &native_root)
                .unwrap_err();
            assert_eq!(error.contract, "tests.xtask-test");
            assert_eq!(session.entries.len(), 6);
        }
        assert!(session.entries.iter().any(|entry| {
            entry.path == "payloads/native-window.acceptance/native-acceptance-failure.json"
        }));
    }

    #[test]
    fn complete_native_setup_failure_is_retained() {
        use uqm_rust::automation::{
            NativeAcceptanceSetupFailureContract, NativeAcceptanceSetupFailureManifest,
            NATIVE_ACCEPTANCE_SETUP_FAILURE_SCHEMA,
        };

        let temp = tempfile::tempdir().unwrap();
        let authority: Authority =
            serde_json::from_slice(include_bytes!("../../../ci/gates.json")).unwrap();
        let gate = authority.gate("tests").unwrap().clone();
        let step = gate
            .steps
            .iter()
            .find(|step| step.id == "xtask-test")
            .unwrap()
            .clone();
        let evidence_root = temp.path().join("evidence");
        let native_root = evidence_root.join("payloads/native-window.acceptance");
        fs::create_dir_all(&native_root).unwrap();
        let manifest = NativeAcceptanceSetupFailureManifest {
            schema: NATIVE_ACCEPTANCE_SETUP_FAILURE_SCHEMA.to_string(),
            command: vec!["uqm-native-acceptance".to_string()],
            expected_executable_byte_length: 1,
            expected_executable_sha256: "a".repeat(64),
            runtime_contract: authority.native_runtime_contract(),
            acceptance_policy: authority.native_acceptance.acceptance_policy,
            retained_files: Vec::new(),
            failure_contract: NativeAcceptanceSetupFailureContract::Preparation,
            error: "invalid setup input".to_string(),
            passed: false,
        };
        fs::write(
            native_root.join("native-acceptance-failure.json"),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
        let mut session = RunSession {
            root: temp.path().to_path_buf(),
            authority,
            evidence_root,
            tuple: "macos-aarch64".to_string(),
            cache_mode: "ambient-dev".to_string(),
            source_sha: "a".repeat(40),
            clean: true,
            features: Vec::new(),
            entries: Vec::new(),
        };

        retain_native_acceptance_failure(&mut session, &gate, &step, &native_root).unwrap();
        assert_eq!(session.entries.len(), 1);
        assert_eq!(session.entries[0].role, "native-window.failure");
    }
}
