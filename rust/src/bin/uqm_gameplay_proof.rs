use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use uqm_rust::automation::{
    ChildSession, ChildSessionConfig, ChildSessionError, ChildSessionReceipt, RecordKind,
    SeedDomain, TeardownReceipt, TerminalClass, TraceRecord, AUTOMATION_SEED,
};

const SCHEMA: &str = "uqm-lcar-v1";
const FAILURE_FILE: &str = "failure-lcar-v1.json";
const PASS_FILE: &str = "lcar-v1.json";
const PRODUCTION_SCHEMA: &str = "uqm-deterministic-artifacts-v4";
const PRODUCTION_FEATURES: [&str; 2] = ["audio_heart", "linked_c_archive"];
const LOG_BUDGET: u64 = 64 * 1024 * 1024;
const TIMEOUT_SECONDS: u64 = 900;

#[derive(Debug, Clone)]
struct ProductionManifest {
    git_head: String,
    target: String,
    profile: String,
    features: Vec<String>,
    executable: ProductionArtifact,
}

#[derive(Debug, Clone)]
struct ProductionArtifact {
    path: String,
    sha256: String,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum FailedContract {
    Timeout,
    Reader,
    Budget,
    NonzeroChild,
    MissingTeardown,
    SemanticEvidence,
    TeardownEvidence,
    ConfigCleanup,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
enum ArtifactRole {
    StdoutLog,
    StderrLog,
    Trace,
    TeardownReceipt,
    Capture,
    ProductionManifestSnapshot,
    ExecutableSnapshot,
    ScriptSnapshot,
    ContentIdentitySnapshot,
    InitialConfigSnapshot,
    FinalConfigSnapshot,
    RetainedConfigFile,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ArtifactEntry {
    role: ArtifactRole,
    path: String,
    sha256: String,
    bytes: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ProcessReceipt {
    pid: u32,
    start_time: String,
    executable_sha256: String,
    exit_code: Option<i32>,
    signal: Option<i32>,
    term_sent: bool,
    kill_sent: bool,
    stdout_bytes: u64,
    stderr_bytes: u64,
    output_drained: bool,
    orphan_check_passed: bool,
}

impl From<ChildSessionReceipt> for ProcessReceipt {
    fn from(receipt: ChildSessionReceipt) -> Self {
        Self {
            pid: receipt.identity.pid,
            start_time: receipt.identity.start_time,
            executable_sha256: receipt.identity.executable_digest,
            exit_code: receipt.exit_code,
            signal: receipt.signal,
            term_sent: receipt.term_sent,
            kill_sent: receipt.kill_sent,
            stdout_bytes: receipt.stdout_bytes,
            stderr_bytes: receipt.stderr_bytes,
            output_drained: receipt.output_drained,
            orphan_check_passed: receipt.orphan_check_passed,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CleanupReceipt {
    exact_child_reaped: bool,
    orphan_check_passed: bool,
    output_drained: bool,
    config_root_removed: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Provenance {
    production_manifest_sha256: String,
    executable_sha256: String,
    script_sha256: String,
    content_tree_sha256: String,
    initial_config_tree_sha256: String,
    final_config_tree_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct LcarManifest {
    schema: String,
    passed: bool,
    first_failed_contract: Option<FailedContract>,
    git_head: String,
    command: Vec<String>,
    environment: BTreeMap<String, String>,
    target: String,
    profile: String,
    features: Vec<String>,
    renderer: String,
    seed: u32,
    provenance: Provenance,
    process: ProcessReceipt,
    cleanup: CleanupReceipt,
    artifacts: Vec<ArtifactEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct TreeSnapshot {
    schema: String,
    root_role: String,
    tree_sha256: String,
    entries: Vec<TreeEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct TreeEntry {
    path: String,
    sha256: String,
    bytes: u64,
}

struct RunEvidence {
    output_root: PathBuf,
    config_root: PathBuf,
    production: ProductionManifest,
    command: Vec<String>,
    environment: BTreeMap<String, String>,
    provenance: Provenance,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("uqm-gameplay-proof: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("run") if args.len() == 6 => run_proof(
            Path::new(&args[2]),
            Path::new(&args[3]),
            Path::new(&args[4]),
            Path::new(&args[5]),
        ),
        Some("validate") if args.len() == 3 => validate_manifest(Path::new(&args[2])),
        Some("validate-negative-fixtures") if args.len() == 2 => {
            run_deterministic_negative_fixtures()
        }
        Some("compare-battle") if args.len() == 4 => {
            compare_battle_proofs(Path::new(&args[2]), Path::new(&args[3]))
        }
        _ => Err("usage: uqm-gameplay-proof run REPO_ROOT PRODUCTION_MANIFEST SCRIPT OUTPUT_ROOT | validate LCAR_MANIFEST | validate-negative-fixtures | compare-battle FIRST_LCAR SECOND_LCAR".into()),
    }
}

fn run_proof(
    repo_root: &Path,
    production_path: &Path,
    script: &Path,
    output_root: &Path,
) -> Result<(), String> {
    let repo_root = fs::canonicalize(repo_root)
        .map_err(|error| format!("canonicalize repository {}: {error}", repo_root.display()))?;
    let production_value = read_json_value(production_path)?;
    let production = parse_production(&production_value)?;
    validate_production(&production, true)?;
    verify_source_binding(&repo_root, &production.git_head)?;
    let executable = repo_root.join(&production.executable.path);
    if hash_file(&executable)? != production.executable.sha256 {
        return Err("production executable hash differs from production manifest".into());
    }
    let script = fs::canonicalize(script)
        .map_err(|error| format!("canonicalize script {}: {error}", script.display()))?;
    let content = repo_root.join("sc2/content");
    let mut evidence = prepare_evidence(
        &repo_root,
        production_path,
        &production,
        &executable,
        &script,
        &content,
        output_root,
    )?;
    let receipt = supervise_child(
        &repo_root,
        &evidence.output_root.join("snapshots/uqm"),
        &content,
        &evidence.output_root.join("snapshots/script.json"),
        &evidence,
    );
    complete_run(&mut evidence, receipt)
}

fn prepare_evidence(
    repo_root: &Path,
    production_path: &Path,
    production: &ProductionManifest,
    executable: &Path,
    script: &Path,
    content: &Path,
    output_root: &Path,
) -> Result<RunEvidence, String> {
    fs::create_dir(output_root)
        .map_err(|error| format!("create fresh output {}: {error}", output_root.display()))?;
    let output_root = fs::canonicalize(output_root)
        .map_err(|error| format!("canonicalize output {}: {error}", output_root.display()))?;
    let snapshots = output_root.join("snapshots");
    let config_root = output_root.join("config");
    fs::create_dir(&snapshots).map_err(|error| format!("create snapshots: {error}"))?;
    fs::create_dir(&config_root).map_err(|error| format!("create config: {error}"))?;

    copy_new(production_path, &snapshots.join("production-manifest.json"))?;
    copy_new(executable, &snapshots.join("uqm"))?;
    copy_new(script, &snapshots.join("script.json"))?;
    let content_snapshot = snapshot_tree(content, "content")?;
    write_new_json(&snapshots.join("content-identity.json"), &content_snapshot)?;
    let initial_config = snapshot_tree(&config_root, "initial_config")?;
    write_new_json(&snapshots.join("config-initial.json"), &initial_config)?;

    let run_root = output_root.join("run");
    let command = vec![
        snapshots.join("uqm").display().to_string(),
        format!("--contentdir={}", content.display()),
        format!("--configdir={}", config_root.display()),
        format!(
            "--automation-script={}",
            snapshots.join("script.json").display()
        ),
        format!("--automation-output={}", run_root.display()),
        "--res=640x480".into(),
        "--windowed".into(),
        "--scroll=pc".into(),
    ];
    let environment = BTreeMap::from([
        ("SDL_AUDIODRIVER".into(), "dummy".into()),
        ("SDL_VIDEODRIVER".into(), "dummy".into()),
    ]);
    let provenance = Provenance {
        production_manifest_sha256: hash_file(&snapshots.join("production-manifest.json"))?,
        executable_sha256: hash_file(&snapshots.join("uqm"))?,
        script_sha256: hash_file(&snapshots.join("script.json"))?,
        content_tree_sha256: content_snapshot.tree_sha256,
        initial_config_tree_sha256: initial_config.tree_sha256,
        final_config_tree_sha256: String::new(),
    };
    let _ = repo_root;
    Ok(RunEvidence {
        output_root,
        config_root,
        production: production.clone(),
        command,
        environment,
        provenance,
    })
}

fn supervise_child(
    repo_root: &Path,
    executable: &Path,
    content: &Path,
    script: &Path,
    evidence: &RunEvidence,
) -> Result<ChildSessionReceipt, (ChildSessionError, Box<ChildSessionReceipt>)> {
    let run_root = evidence.output_root.join("run");
    let mut command = Command::new(executable);
    command
        .arg(format!("--contentdir={}", content.display()))
        .arg(format!("--configdir={}", evidence.config_root.display()))
        .arg(format!("--automation-script={}", script.display()))
        .arg(format!("--automation-output={}", run_root.display()))
        .args(["--res=640x480", "--windowed", "--scroll=pc"])
        .current_dir(repo_root)
        .env("SDL_VIDEODRIVER", "dummy")
        .env("SDL_AUDIODRIVER", "dummy");
    let config = ChildSessionConfig {
        output_root: evidence.output_root.clone(),
        stdout_log: evidence.output_root.join("stdout.log"),
        stderr_log: evidence.output_root.join("stderr.log"),
        stdout_budget: LOG_BUDGET,
        stderr_budget: LOG_BUDGET,
        timeout: Duration::from_secs(TIMEOUT_SECONDS),
        grace: Duration::from_secs(5),
        executable_digest: evidence.provenance.executable_sha256.clone(),
    };
    let session = match ChildSession::spawn(command, config) {
        Ok(session) => session,
        Err(error) => {
            eprintln!("child spawn failed before a trustworthy process receipt existed: {error}");
            return Err((
                error,
                Box::new(unavailable_receipt(&evidence.provenance.executable_sha256)),
            ));
        }
    };
    session
        .finish()
        .map_err(|failure| (failure.error, failure.receipt))
}

fn complete_run(
    evidence: &mut RunEvidence,
    child_result: Result<ChildSessionReceipt, (ChildSessionError, Box<ChildSessionReceipt>)>,
) -> Result<(), String> {
    let (session_contract, process) = match child_result {
        Ok(receipt) => (None, ProcessReceipt::from(receipt)),
        Err((error, receipt)) if receipt.identity.pid != 0 => (
            Some(classify_session_error(&error)),
            ProcessReceipt::from(*receipt),
        ),
        Err((error, _)) => return Err(error.to_string()),
    };
    let cleanup_error = finalize_config_evidence(evidence).err();
    let cleanup = CleanupReceipt {
        exact_child_reaped: process.exit_code.is_some() || process.signal.is_some(),
        orphan_check_passed: process.orphan_check_passed,
        output_drained: process.output_drained,
        config_root_removed: !evidence.config_root.exists(),
    };
    let evidence_contract = inspect_child_evidence(evidence, &process).err();
    let first_failed_contract = session_contract
        .or_else(|| {
            cleanup_error
                .as_ref()
                .map(|_| FailedContract::ConfigCleanup)
        })
        .or(evidence_contract);
    let passed = first_failed_contract.is_none();
    let artifacts = collect_artifacts(&evidence.output_root)?;
    let manifest = LcarManifest {
        schema: SCHEMA.into(),
        passed,
        first_failed_contract,
        git_head: evidence.production.git_head.clone(),
        command: evidence.command.clone(),
        environment: evidence.environment.clone(),
        target: evidence.production.target.clone(),
        profile: evidence.production.profile.clone(),
        features: evidence.production.features.clone(),
        renderer: "sdl2-software-dummy".into(),
        seed: AUTOMATION_SEED,
        provenance: evidence.provenance.clone(),
        process,
        cleanup,
        artifacts,
    };
    let name = if passed { PASS_FILE } else { FAILURE_FILE };
    let manifest_path = evidence.output_root.join(name);
    write_atomic_new_json(&manifest_path, &manifest)?;
    validate_manifest(&manifest_path)?;
    if passed {
        Ok(())
    } else {
        Err(format!(
            "gameplay proof failed at {:?}; evidence: {}{}",
            manifest.first_failed_contract,
            manifest_path.display(),
            cleanup_error
                .as_ref()
                .map(|error| format!("; cleanup detail: {error}"))
                .unwrap_or_default()
        ))
    }
}

fn finalize_config_evidence(evidence: &mut RunEvidence) -> Result<(), String> {
    let final_config = snapshot_tree(&evidence.config_root, "final_config")?;
    evidence.provenance.final_config_tree_sha256 = final_config.tree_sha256.clone();
    write_new_json(
        &evidence.output_root.join("snapshots/config-final.json"),
        &final_config,
    )?;
    fs::remove_dir_all(&evidence.config_root)
        .map_err(|error| format!("remove mutable config root: {error}"))?;
    if evidence.config_root.exists() {
        return Err("mutable config root remains after cleanup".into());
    }
    Ok(())
}

fn inspect_child_evidence(
    evidence: &RunEvidence,
    process: &ProcessReceipt,
) -> Result<(), FailedContract> {
    let teardown_path = evidence.output_root.join("run/teardown-complete.json");
    if !teardown_path.is_file() {
        return Err(FailedContract::MissingTeardown);
    }
    let teardown: TeardownReceipt =
        read_json(&teardown_path).map_err(|_| FailedContract::TeardownEvidence)?;
    if teardown.schema != "uqm-teardown-v1"
        || teardown.process_status != process.exit_code.unwrap_or(1)
        || !teardown.runtime_finalized
        || !teardown.runtime_deactivated
        || !teardown.callbacks_quiescent
        || !teardown.trace_durable
    {
        return Err(FailedContract::TeardownEvidence);
    }
    if process.exit_code != Some(0) || process.signal.is_some() {
        return if teardown
            .terminal
            .is_some_and(|terminal| !terminal.is_success())
        {
            Err(FailedContract::SemanticEvidence)
        } else {
            Err(FailedContract::NonzeroChild)
        };
    }
    if teardown.terminal != Some(TerminalClass::Success)
        || teardown.game_status != 0
        || teardown.process_status != 0
    {
        return Err(FailedContract::SemanticEvidence);
    }
    validate_trace_and_captures(&evidence.output_root, true)
        .map_err(|_| FailedContract::SemanticEvidence)
}

fn classify_session_error(error: &ChildSessionError) -> FailedContract {
    match error {
        ChildSessionError::Timeout { .. } => FailedContract::Timeout,
        ChildSessionError::BudgetExceeded { .. } => FailedContract::Budget,
        ChildSessionError::Reader { .. } | ChildSessionError::JoinPanic { .. } => {
            FailedContract::Reader
        }
        _ => FailedContract::NonzeroChild,
    }
}

fn unavailable_receipt(executable_digest: &str) -> ChildSessionReceipt {
    ChildSessionReceipt {
        exit_code: None,
        signal: None,
        term_sent: false,
        kill_sent: false,
        stdout_bytes: 0,
        stderr_bytes: 0,
        output_drained: false,
        orphan_check_passed: false,
        identity: uqm_rust::automation::ProcessIdentity {
            pid: 0,
            start_time: String::new(),
            executable_digest: executable_digest.into(),
        },
    }
}

fn validate_manifest(path: &Path) -> Result<(), String> {
    let manifest: LcarManifest = read_json(path)?;
    let root = path
        .parent()
        .ok_or_else(|| "LCAR manifest has no parent".to_string())?;
    validate_manifest_identity(path, &manifest)?;
    validate_inventory(root, &manifest)?;
    validate_provenance(root, &manifest)?;
    validate_command(root, &manifest)?;
    validate_result(root, &manifest)
}

fn validate_manifest_identity(path: &Path, manifest: &LcarManifest) -> Result<(), String> {
    let expected_name = if manifest.passed {
        PASS_FILE
    } else {
        FAILURE_FILE
    };
    if path.file_name().and_then(|name| name.to_str()) != Some(expected_name)
        || manifest.schema != SCHEMA
        || !is_hex(&manifest.git_head, 40)
        || manifest.seed != AUTOMATION_SEED
        || manifest.renderer != "sdl2-software-dummy"
        || manifest.profile != "release"
        || manifest.features != PRODUCTION_FEATURES
        || !supported_target(&manifest.target)
        || manifest.process.executable_sha256 != manifest.provenance.executable_sha256
    {
        return Err("LCAR identity or production contract is invalid".into());
    }
    let expected_env = BTreeMap::from([
        ("SDL_AUDIODRIVER".into(), "dummy".into()),
        ("SDL_VIDEODRIVER".into(), "dummy".into()),
    ]);
    if manifest.environment != expected_env {
        return Err("LCAR SDL dummy environment is not exact".into());
    }
    Ok(())
}

fn validate_inventory(root: &Path, manifest: &LcarManifest) -> Result<(), String> {
    if manifest.artifacts.is_empty() {
        return Err("LCAR artifact inventory is empty".into());
    }
    let mut paths = BTreeSet::new();
    let mut roles = BTreeMap::<ArtifactRole, usize>::new();
    for entry in &manifest.artifacts {
        validate_relative_path(&entry.path)?;
        if !paths.insert(entry.path.clone()) {
            return Err(format!("duplicate artifact path: {}", entry.path));
        }
        if !is_hex(&entry.sha256, 64) || entry.bytes == 0 && !allows_empty(entry.role) {
            return Err(format!("invalid artifact identity: {}", entry.path));
        }
        *roles.entry(entry.role).or_default() += 1;
        let artifact = root.join(&entry.path);
        let metadata = fs::metadata(&artifact)
            .map_err(|error| format!("missing artifact {}: {error}", artifact.display()))?;
        if !metadata.is_file()
            || metadata.len() != entry.bytes
            || hash_file(&artifact)? != entry.sha256
        {
            return Err(format!(
                "artifact identity mismatch: {}",
                artifact.display()
            ));
        }
    }
    let actual = collect_relative_files(root)?;
    if actual != paths {
        return Err(format!(
            "artifact inventory is not exact: manifest={paths:?}, actual={actual:?}"
        ));
    }
    for role in mandatory_roles() {
        if roles.get(&role) != Some(&1) {
            return Err(format!("mandatory artifact role {role:?} is not unique"));
        }
    }
    if manifest.passed && roles.get(&ArtifactRole::Capture).copied().unwrap_or(0) == 0 {
        return Err("passing LCAR has no capture artifacts".into());
    }
    if manifest.passed {
        for role in [ArtifactRole::Trace, ArtifactRole::TeardownReceipt] {
            if roles.get(&role) != Some(&1) {
                return Err(format!("passing LCAR lacks {role:?}"));
            }
        }
    }
    Ok(())
}

fn validate_provenance(root: &Path, manifest: &LcarManifest) -> Result<(), String> {
    let fields = [
        &manifest.provenance.production_manifest_sha256,
        &manifest.provenance.executable_sha256,
        &manifest.provenance.script_sha256,
        &manifest.provenance.content_tree_sha256,
        &manifest.provenance.initial_config_tree_sha256,
        &manifest.provenance.final_config_tree_sha256,
    ];
    if fields.iter().any(|digest| !is_hex(digest, 64)) {
        return Err("top-level provenance contains a malformed SHA-256".into());
    }
    revalidate_snapshot_digest(
        root,
        manifest,
        ArtifactRole::ProductionManifestSnapshot,
        &manifest.provenance.production_manifest_sha256,
    )?;
    revalidate_snapshot_digest(
        root,
        manifest,
        ArtifactRole::ExecutableSnapshot,
        &manifest.provenance.executable_sha256,
    )?;
    revalidate_snapshot_digest(
        root,
        manifest,
        ArtifactRole::ScriptSnapshot,
        &manifest.provenance.script_sha256,
    )?;
    let content = read_tree_snapshot(root, manifest, ArtifactRole::ContentIdentitySnapshot)?;
    let initial = read_tree_snapshot(root, manifest, ArtifactRole::InitialConfigSnapshot)?;
    let final_config = read_tree_snapshot(root, manifest, ArtifactRole::FinalConfigSnapshot)?;
    validate_tree_snapshot(
        &content,
        "content",
        &manifest.provenance.content_tree_sha256,
    )?;
    validate_tree_snapshot(
        &initial,
        "initial_config",
        &manifest.provenance.initial_config_tree_sha256,
    )?;
    validate_tree_snapshot(
        &final_config,
        "final_config",
        &manifest.provenance.final_config_tree_sha256,
    )?;
    let production_path = artifact_path(root, manifest, ArtifactRole::ProductionManifestSnapshot)?;
    let production = parse_production(&read_json_value(&production_path)?)?;
    validate_production(&production, false)?;
    if production.git_head != manifest.git_head
        || production.target != manifest.target
        || production.profile != manifest.profile
        || production.features != manifest.features
        || production.executable.sha256 != manifest.provenance.executable_sha256
    {
        return Err("retained production snapshot does not bind the LCAR identity".into());
    }
    Ok(())
}

fn validate_command(root: &Path, manifest: &LcarManifest) -> Result<(), String> {
    let root = fs::canonicalize(root)
        .map_err(|error| format!("canonicalize LCAR root {}: {error}", root.display()))?;
    if manifest.command.len() != 8
        || manifest.command[0]
            != artifact_path(&root, manifest, ArtifactRole::ExecutableSnapshot)?
                .display()
                .to_string()
        || manifest.command[1] != format!("--contentdir={}", command_content(&manifest.command[1])?)
        || manifest.command[2] != format!("--configdir={}", root.join("config").display())
        || !manifest.command[3].starts_with("--automation-script=")
        || manifest.command[4] != format!("--automation-output={}", root.join("run").display())
        || manifest.command[5] != "--res=640x480"
        || manifest.command[6] != "--windowed"
        || manifest.command[7] != "--scroll=pc"
    {
        return Err("recorded gameplay command is not the exact supported command".into());
    }
    let content = command_content(&manifest.command[1])?;
    if !content.ends_with("sc2/content")
        || Path::new(content)
            .components()
            .any(|c| c == Component::ParentDir)
    {
        return Err("recorded content command path is invalid".into());
    }
    let script = manifest.command[3]
        .strip_prefix("--automation-script=")
        .ok_or_else(|| "script command argument is malformed".to_string())?;
    if script
        != artifact_path(&root, manifest, ArtifactRole::ScriptSnapshot)?
            .display()
            .to_string()
    {
        return Err("recorded automation script is not the retained immutable snapshot".into());
    }
    Ok(())
}

fn validate_result(root: &Path, manifest: &LcarManifest) -> Result<(), String> {
    if manifest.cleanup.orphan_check_passed != manifest.process.orphan_check_passed
        || manifest.cleanup.output_drained != manifest.process.output_drained
        || manifest.cleanup.config_root_removed != !root.join("config").exists()
    {
        return Err("parent cleanup facts do not match retained evidence".into());
    }
    if manifest.passed {
        if manifest.first_failed_contract.is_some()
            || manifest.process.exit_code != Some(0)
            || manifest.process.signal.is_some()
            || !manifest.cleanup.exact_child_reaped
            || !manifest.cleanup.orphan_check_passed
            || !manifest.cleanup.output_drained
            || !manifest.cleanup.config_root_removed
        {
            return Err("passing LCAR has a failing process or cleanup receipt".into());
        }
        validate_teardown(root, manifest, true)?;
        validate_trace_and_captures(root, true)
    } else {
        let contract = manifest
            .first_failed_contract
            .ok_or_else(|| "failing LCAR lacks first_failed_contract".to_string())?;
        validate_failure_contract(root, manifest, contract)
    }
}

fn validate_failure_contract(
    root: &Path,
    manifest: &LcarManifest,
    contract: FailedContract,
) -> Result<(), String> {
    match contract {
        FailedContract::Timeout if !(manifest.process.term_sent || manifest.process.kill_sent) => {
            Err("timeout failure lacks stop evidence".into())
        }
        FailedContract::Reader | FailedContract::Budget
            if !manifest.cleanup.exact_child_reaped || !manifest.cleanup.orphan_check_passed =>
        {
            Err("reader/budget failure lacks child cleanup evidence".into())
        }
        FailedContract::NonzeroChild
            if manifest.process.exit_code == Some(0) && manifest.process.signal.is_none() =>
        {
            Err("nonzero-child contract has a successful status".into())
        }
        FailedContract::MissingTeardown
            if manifest
                .artifacts
                .iter()
                .any(|entry| entry.role == ArtifactRole::TeardownReceipt) =>
        {
            Err("missing-teardown contract includes a teardown receipt".into())
        }
        FailedContract::SemanticEvidence => {
            validate_teardown(root, manifest, false)?;
            let receipt: TeardownReceipt = read_json(&artifact_path(
                root,
                manifest,
                ArtifactRole::TeardownReceipt,
            )?)?;
            if receipt.terminal.is_none_or(TerminalClass::is_success) {
                return Err("semantic failure lacks a typed failing terminal outcome".into());
            }
            let trace = artifact_path(root, manifest, ArtifactRole::Trace)?;
            validate_trace_failure(&trace)
        }
        FailedContract::TeardownEvidence => {
            if validate_teardown(root, manifest, false).is_ok() {
                return Err("teardown-evidence contract contains an acceptable receipt".into());
            }
            Ok(())
        }
        FailedContract::ConfigCleanup
            if manifest.cleanup.config_root_removed || !root.join("config").exists() =>
        {
            Err("config-cleanup failure does not retain the failed cleanup state".into())
        }
        FailedContract::ConfigCleanup
            if !manifest
                .artifacts
                .iter()
                .any(|entry| entry.role == ArtifactRole::RetainedConfigFile) =>
        {
            Err("config-cleanup failure lacks retained config evidence".into())
        }
        _ => Ok(()),
    }
}

fn validate_teardown(root: &Path, manifest: &LcarManifest, passing: bool) -> Result<(), String> {
    let path = artifact_path(root, manifest, ArtifactRole::TeardownReceipt)?;
    let receipt: TeardownReceipt = read_json(&path)?;
    if receipt.schema != "uqm-teardown-v1"
        || receipt.process_status != manifest.process.exit_code.unwrap_or(1)
        || !receipt.runtime_finalized
        || !receipt.runtime_deactivated
        || !receipt.callbacks_quiescent
        || !receipt.trace_durable
    {
        return Err("typed teardown receipt facts are invalid".into());
    }
    if passing
        && (receipt.terminal != Some(TerminalClass::Success)
            || receipt.game_status != 0
            || receipt.process_status != 0)
    {
        return Err("passing teardown receipt is not successful".into());
    }
    Ok(())
}

fn validate_trace_and_captures(root: &Path, require_success: bool) -> Result<(), String> {
    let trace_path = root.join("run/trace.jsonl");
    let records = parse_trace(&trace_path)?;
    if records.first().map(|record| &record.kind) != Some(&RecordKind::RunStart)
        || records.last().map(|record| &record.kind) != Some(&RecordKind::RunEnd)
    {
        return Err("trace does not start with run_start and end with run_end".into());
    }
    let mut present_count = 0_usize;
    let mut semantic_count = 0_usize;
    let mut traced_captures = BTreeSet::new();
    let mut ordered_capture_labels: Vec<String> = Vec::new();
    for (sequence, record) in records.iter().enumerate() {
        if record.schema != TraceRecord::SCHEMA
            || record.run != 1
            || record.sequence != sequence as u64
        {
            return Err("trace sequence/schema/run is not monotonic and exact".into());
        }
        if record.kind == RecordKind::Presentation {
            validate_presentation(record)?;
            present_count += 1;
        }
        if record.kind == RecordKind::SemanticAssertion {
            validate_semantic_assertion(record, require_success)?;
            semantic_count += 1;
        }
        if record.kind == RecordKind::Capture {
            validate_presentation(record)?;
            let label = capture_base(record)?;
            traced_captures.insert(format!("run/captures/{label}.png"));
            ordered_capture_labels.push(label);
        }
    }
    if require_success && (present_count == 0 || semantic_count == 0 || traced_captures.is_empty())
    {
        return Err("trace lacks present, semantic assertion, or capture evidence".into());
    }
    let actual_captures = collect_capture_paths(root)?;
    if traced_captures != actual_captures {
        return Err("capture trace records do not correlate exactly with PNG artifacts".into());
    }
    if require_success {
        validate_captures_differ(root, &ordered_capture_labels)?;
    }
    Ok(())
}

/// Reject a passing run where a capture marked `expect_change` is identical to
/// the one before it.
///
/// A frozen screen still completes captures and still records presentations, so
/// without this a proof passes while the player sees the previous frame. Only
/// captures the script marks are checked, because a legitimately static screen
/// sampled twice produces identical pixels.
fn validate_captures_differ(root: &Path, ordered_labels: &[String]) -> Result<(), String> {
    let script: serde_json::Value = read_json_value(&root.join("snapshots/script.json"))?;
    let expecting: BTreeSet<&str> = script
        .get("steps")
        .and_then(serde_json::Value::as_array)
        .map(|steps| {
            steps
                .iter()
                .filter(|step| {
                    step.get("expect_change")
                        .and_then(serde_json::Value::as_bool)
                        == Some(true)
                })
                .filter_map(|step| step.get("label").and_then(serde_json::Value::as_str))
                .collect()
        })
        .unwrap_or_default();
    if expecting.is_empty() {
        return Ok(());
    }

    let mut previous: Option<(String, String)> = None;
    for label in ordered_labels {
        let relative = format!("run/captures/{label}.png");
        let digest = hash_file(&root.join(&relative))?;
        if expecting.contains(label.as_str()) {
            if let Some((previous_label, previous_digest)) = &previous {
                if *previous_digest == digest {
                    return Err(format!(
                        "capture {label} is byte-identical to {previous_label}, so the screen \
                         never changed across a transition the script expected to be visible; \
                         the run presented no new frame"
                    ));
                }
            }
        }
        previous = Some((label.clone(), digest));
    }
    Ok(())
}

fn validate_trace_failure(path: &Path) -> Result<(), String> {
    let records = parse_trace(path)?;
    if records.is_empty() {
        return Err("failure trace is empty".into());
    }
    for (sequence, record) in records.iter().enumerate() {
        if record.sequence != sequence as u64 || record.schema != TraceRecord::SCHEMA {
            return Err("failure trace sequence is malformed".into());
        }
    }
    if records.last().map(|record| &record.kind) != Some(&RecordKind::RunEnd) {
        return Err("semantic failure trace lacks terminal run_end".into());
    }
    Ok(())
}

fn parse_trace(path: &Path) -> Result<Vec<TraceRecord>, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("read trace {}: {error}", path.display()))?;
    if text.is_empty() || !text.ends_with('\n') {
        return Err("trace is empty or lacks a final newline".into());
    }
    text.lines()
        .map(|line| TraceRecord::from_jsonl(line).map_err(|error| error.to_string()))
        .collect()
}

fn validate_presentation(record: &TraceRecord) -> Result<(), String> {
    let presentation = record
        .presentation
        .as_ref()
        .ok_or_else(|| "present/capture record lacks actual presentation evidence".to_string())?;
    if presentation.count == 0
        || presentation.count != record.present_seen
        || presentation.width == 0
        || presentation.height == 0
    {
        return Err("present/capture evidence is inconsistent".into());
    }
    Ok(())
}

fn validate_semantic_assertion(record: &TraceRecord, require_success: bool) -> Result<(), String> {
    if let Some(activity) = &record.activity {
        if activity.word & activity.mask != activity.equals || !activity.passed {
            return Err("activity semantic assertion failed".into());
        }
        return Ok(());
    }
    let label = record
        .label
        .as_deref()
        .ok_or_else(|| "semantic assertion lacks typed evidence or a label".to_string())?;
    if require_success
        && ["failed", "mismatch", "error"]
            .iter()
            .any(|word| label.contains(word))
    {
        return Err("semantic assertion label reports failure".into());
    }
    Ok(())
}

fn capture_base(record: &TraceRecord) -> Result<String, String> {
    let label = record
        .label
        .as_deref()
        .ok_or_else(|| "capture trace lacks label".to_string())?;
    let (base, generation) = label
        .rsplit_once("_gen")
        .ok_or_else(|| "capture trace label lacks generation".to_string())?;
    if base.is_empty()
        || generation
            .parse::<u64>()
            .ok()
            .filter(|value| *value > 0)
            .is_none()
    {
        return Err("capture trace label has invalid generation".into());
    }
    Ok(base.into())
}

fn validate_production(manifest: &ProductionManifest, require_host: bool) -> Result<(), String> {
    if !is_hex(&manifest.git_head, 40)
        || manifest.profile != "release"
        || manifest.features != PRODUCTION_FEATURES
        || !supported_target(&manifest.target)
        || !is_hex(&manifest.executable.sha256, 64)
    {
        return Err("production artifact manifest is not exact canonical production".into());
    }
    validate_relative_path(&manifest.executable.path)?;
    if require_host && manifest.target != host_target()? {
        return Err(format!(
            "production target {} does not match native host {}",
            manifest.target,
            host_target()?
        ));
    }
    Ok(())
}

fn parse_production(value: &serde_json::Value) -> Result<ProductionManifest, String> {
    let object = value
        .as_object()
        .ok_or_else(|| "production manifest root is not an object".to_string())?;
    if object.get("schema").and_then(|value| value.as_str()) != Some(PRODUCTION_SCHEMA)
        || object.get("dirty").and_then(|value| value.as_bool()) != Some(false)
    {
        return Err("production manifest schema/cleanliness is invalid".into());
    }
    let git_head = required_string(object, "git_head")?;
    let target = required_string(object, "target")?;
    let profile = required_string(object, "profile")?;
    let features = object
        .get("features")
        .and_then(|value| value.as_array())
        .ok_or_else(|| "production features are absent".to_string())?
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| "production feature is not a string".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let artifacts = object
        .get("artifacts")
        .and_then(|value| value.as_array())
        .ok_or_else(|| "production artifacts are absent".to_string())?;
    let mut executable = artifacts.iter().filter(|artifact| {
        artifact.get("role").and_then(|value| value.as_str()) == Some("executable")
    });
    let artifact = executable
        .next()
        .ok_or_else(|| "production manifest lacks executable".to_string())?;
    if executable.next().is_some() {
        return Err("production manifest has duplicate executable artifacts".into());
    }
    Ok(ProductionManifest {
        git_head,
        target,
        profile,
        features,
        executable: ProductionArtifact {
            path: artifact
                .get("path")
                .and_then(|value| value.as_str())
                .ok_or_else(|| "production executable path is absent".to_string())?
                .into(),
            sha256: artifact
                .get("sha256")
                .and_then(|value| value.as_str())
                .ok_or_else(|| "production executable digest is absent".to_string())?
                .into(),
        },
    })
}

fn required_string(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &str,
) -> Result<String, String> {
    object
        .get(field)
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .ok_or_else(|| format!("production field {field} is absent or malformed"))
}

fn verify_source_binding(repo_root: &Path, expected_head: &str) -> Result<(), String> {
    let top = git_output(
        repo_root,
        &["rev-parse", "--show-toplevel"],
        "repository root",
    )?;
    let canonical_top =
        fs::canonicalize(top.trim()).map_err(|error| format!("canonicalize git root: {error}"))?;
    if canonical_top != repo_root {
        return Err("repository path is not the canonical git root".into());
    }
    let head = git_output(repo_root, &["rev-parse", "HEAD"], "HEAD")?;
    if head.trim() != expected_head {
        return Err("current repository HEAD differs from production manifest".into());
    }
    let status = git_command(repo_root)
        .args(["status", "--porcelain=v1", "--untracked-files=all", "-z"])
        .output()
        .map_err(|error| format!("run git source cleanliness check: {error}"))?;
    if !status.status.success() || !status.stdout.is_empty() {
        return Err("gameplay proof requires a clean source tree including untracked files".into());
    }
    Ok(())
}

fn git_command(root: &Path) -> Command {
    let mut safe_directory = std::ffi::OsString::from("safe.directory=");
    safe_directory.push(root.as_os_str());
    let mut command = Command::new("git");
    command.arg("-c").arg(safe_directory).current_dir(root);
    command
}

fn git_output(root: &Path, args: &[&str], label: &str) -> Result<String, String> {
    let output = git_command(root)
        .args(args)
        .output()
        .map_err(|error| format!("run git for {label}: {error}"))?;
    if !output.status.success() {
        return Err(format!("git failed to provide {label}"));
    }
    String::from_utf8(output.stdout).map_err(|error| format!("git {label} is not UTF-8: {error}"))
}

fn host_target() -> Result<String, String> {
    let output = Command::new("rustc")
        .arg("-vV")
        .output()
        .map_err(|error| format!("run rustc -vV: {error}"))?;
    String::from_utf8(output.stdout)
        .map_err(|error| format!("rustc output is not UTF-8: {error}"))?
        .lines()
        .find_map(|line| line.strip_prefix("host: ").map(str::to_string))
        .ok_or_else(|| "rustc did not report a host target".into())
}

fn supported_target(target: &str) -> bool {
    matches!(
        target,
        "aarch64-apple-darwin"
            | "x86_64-apple-darwin"
            | "aarch64-unknown-linux-gnu"
            | "x86_64-unknown-linux-gnu"
    )
}

fn snapshot_tree(root: &Path, root_role: &str) -> Result<TreeSnapshot, String> {
    let paths = collect_paths(root)?;
    let mut entries = Vec::with_capacity(paths.len());
    for path in paths {
        let metadata =
            fs::metadata(&path).map_err(|error| format!("metadata {}: {error}", path.display()))?;
        entries.push(TreeEntry {
            path: relative_path(root, &path)?,
            sha256: hash_file(&path)?,
            bytes: metadata.len(),
        });
    }
    let tree_sha256 = tree_digest(&entries);
    Ok(TreeSnapshot {
        schema: "uqm-tree-identity-v1".into(),
        root_role: root_role.into(),
        tree_sha256,
        entries,
    })
}

fn validate_tree_snapshot(
    snapshot: &TreeSnapshot,
    role: &str,
    expected_digest: &str,
) -> Result<(), String> {
    if snapshot.schema != "uqm-tree-identity-v1"
        || snapshot.root_role != role
        || snapshot.tree_sha256 != expected_digest
        || tree_digest(&snapshot.entries) != expected_digest
    {
        return Err(format!("{role} tree snapshot identity is invalid"));
    }
    let mut paths = BTreeSet::new();
    for entry in &snapshot.entries {
        validate_relative_path(&entry.path)?;
        if !paths.insert(&entry.path) || !is_hex(&entry.sha256, 64) {
            return Err(format!(
                "{role} tree snapshot has duplicate/malformed entries"
            ));
        }
    }
    Ok(())
}

fn tree_digest(entries: &[TreeEntry]) -> String {
    let mut hasher = Sha256::new();
    for entry in entries {
        hasher.update(entry.path.as_bytes());
        hasher.update([0]);
        hasher.update(entry.sha256.as_bytes());
        hasher.update([0]);
        hasher.update(entry.bytes.to_string().as_bytes());
        hasher.update(b"\n");
    }
    format!("{:x}", hasher.finalize())
}

fn collect_artifacts(root: &Path) -> Result<Vec<ArtifactEntry>, String> {
    let mut entries = Vec::new();
    for path in collect_paths(root)? {
        let relative = relative_path(root, &path)?;
        if is_lcar_name(&relative) || relative.ends_with(".tmp") {
            continue;
        }
        let role = role_for_path(&relative)?;
        let metadata =
            fs::metadata(&path).map_err(|error| format!("metadata {}: {error}", path.display()))?;
        entries.push(ArtifactEntry {
            role,
            path: relative,
            sha256: hash_file(&path)?,
            bytes: metadata.len(),
        });
    }
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(entries)
}

fn role_for_path(path: &str) -> Result<ArtifactRole, String> {
    let role = match path {
        "stdout.log" => ArtifactRole::StdoutLog,
        "stderr.log" => ArtifactRole::StderrLog,
        "run/trace.jsonl" => ArtifactRole::Trace,
        "run/teardown-complete.json" => ArtifactRole::TeardownReceipt,
        "snapshots/production-manifest.json" => ArtifactRole::ProductionManifestSnapshot,
        "snapshots/uqm" => ArtifactRole::ExecutableSnapshot,
        "snapshots/script.json" => ArtifactRole::ScriptSnapshot,
        "snapshots/content-identity.json" => ArtifactRole::ContentIdentitySnapshot,
        "snapshots/config-initial.json" => ArtifactRole::InitialConfigSnapshot,
        "snapshots/config-final.json" => ArtifactRole::FinalConfigSnapshot,
        _ if path.starts_with("run/captures/") && path.ends_with(".png") => ArtifactRole::Capture,
        _ if path.starts_with("config/") => ArtifactRole::RetainedConfigFile,
        _ => return Err(format!("unexpected evidence artifact: {path}")),
    };
    Ok(role)
}

fn mandatory_roles() -> [ArtifactRole; 8] {
    [
        ArtifactRole::StdoutLog,
        ArtifactRole::StderrLog,
        ArtifactRole::ProductionManifestSnapshot,
        ArtifactRole::ExecutableSnapshot,
        ArtifactRole::ScriptSnapshot,
        ArtifactRole::ContentIdentitySnapshot,
        ArtifactRole::InitialConfigSnapshot,
        ArtifactRole::FinalConfigSnapshot,
    ]
}

fn allows_empty(role: ArtifactRole) -> bool {
    matches!(role, ArtifactRole::StdoutLog | ArtifactRole::StderrLog)
}

fn collect_relative_files(root: &Path) -> Result<BTreeSet<String>, String> {
    collect_paths(root)?
        .into_iter()
        .map(|path| relative_path(root, &path))
        .filter(|result| {
            result
                .as_ref()
                .map(|path| !is_lcar_name(path) && !path.ends_with(".tmp"))
                .unwrap_or(true)
        })
        .collect()
}

fn collect_capture_paths(root: &Path) -> Result<BTreeSet<String>, String> {
    let directory = root.join("run/captures");
    if !directory.exists() {
        return Ok(BTreeSet::new());
    }
    collect_paths(&directory)?
        .into_iter()
        .map(|path| relative_path(root, &path))
        .collect()
}

fn collect_paths(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut paths = Vec::new();
    collect_files(root, root, &mut paths)?;
    paths.sort();
    Ok(paths)
}

fn collect_files(root: &Path, current: &Path, paths: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(current)
        .map_err(|error| format!("read directory {}: {error}", current.display()))?
    {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        if file_type.is_dir() {
            collect_files(root, &path, paths)?;
        } else if file_type.is_file() {
            paths.push(path);
        } else {
            return Err(format!(
                "unsupported filesystem entry under {}",
                root.display()
            ));
        }
    }
    Ok(())
}

fn artifact_path(
    root: &Path,
    manifest: &LcarManifest,
    role: ArtifactRole,
) -> Result<PathBuf, String> {
    let mut entries = manifest.artifacts.iter().filter(|entry| entry.role == role);
    let entry = entries
        .next()
        .ok_or_else(|| format!("artifact role {role:?} is absent"))?;
    if entries.next().is_some() {
        return Err(format!("artifact role {role:?} is duplicated"));
    }
    Ok(root.join(&entry.path))
}

fn revalidate_snapshot_digest(
    root: &Path,
    manifest: &LcarManifest,
    role: ArtifactRole,
    digest: &str,
) -> Result<(), String> {
    if hash_file(&artifact_path(root, manifest, role)?)? != digest {
        return Err(format!("top-level {role:?} provenance does not revalidate"));
    }
    Ok(())
}

fn read_tree_snapshot(
    root: &Path,
    manifest: &LcarManifest,
    role: ArtifactRole,
) -> Result<TreeSnapshot, String> {
    read_json(&artifact_path(root, manifest, role)?)
}

fn validate_relative_path(path: &str) -> Result<(), String> {
    if path.is_empty() || path.contains('\\') || path.starts_with('/') || path.ends_with('/') {
        return Err(format!(
            "artifact path is not normalized relative UTF-8: {path:?}"
        ));
    }
    if Path::new(path)
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(format!(
            "artifact path contains traversal/non-normal components: {path}"
        ));
    }
    let normalized = Path::new(path)
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/");
    if normalized != path {
        return Err(format!("artifact path is not normalized: {path}"));
    }
    Ok(())
}

fn command_content(argument: &str) -> Result<&str, String> {
    argument
        .strip_prefix("--contentdir=")
        .ok_or_else(|| "content command argument is malformed".into())
}

fn is_lcar_name(path: &str) -> bool {
    path == PASS_FILE || path == FAILURE_FILE
}

fn is_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn hash_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn relative_path(root: &Path, path: &Path) -> Result<String, String> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| format!("{} is outside {}", path.display(), root.display()))?;
    let value = relative
        .to_str()
        .ok_or_else(|| format!("path is not UTF-8: {}", relative.display()))?
        .replace(std::path::MAIN_SEPARATOR, "/");
    validate_relative_path(&value)?;
    Ok(value)
}

fn copy_new(source: &Path, destination: &Path) -> Result<(), String> {
    let mut input = fs::File::open(source)
        .map_err(|error| format!("open snapshot source {}: {error}", source.display()))?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(|error| format!("create snapshot {}: {error}", destination.display()))?;
    std::io::copy(&mut input, &mut output)
        .map_err(|error| format!("copy snapshot {}: {error}", destination.display()))?;
    fs::set_permissions(
        destination,
        fs::metadata(source)
            .map_err(|error| format!("read snapshot permissions {}: {error}", source.display()))?
            .permissions(),
    )
    .map_err(|error| {
        format!(
            "preserve snapshot permissions {}: {error}",
            destination.display()
        )
    })?;
    output
        .sync_all()
        .map_err(|error| format!("sync snapshot {}: {error}", destination.display()))
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let bytes = fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("parse {}: {error}", path.display()))
}

fn read_json_value(path: &Path) -> Result<serde_json::Value, String> {
    read_json(path)
}

fn write_new_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    write_new(path, &bytes)
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("create {}: {error}", path.display()))?;
    file.write_all(bytes).map_err(|error| error.to_string())?;
    file.flush().map_err(|error| error.to_string())?;
    file.sync_all().map_err(|error| error.to_string())
}

fn write_atomic_new_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "manifest has no parent".to_string())?;
    let temp = parent.join(format!(
        ".{}.tmp",
        path.file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| "manifest name is not UTF-8".to_string())?
    ));
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    write_new(&temp, &bytes)?;
    if path.exists() {
        let _ = fs::remove_file(&temp);
        return Err(format!("refusing to replace existing {}", path.display()));
    }
    fs::rename(&temp, path).map_err(|error| format!("publish {}: {error}", path.display()))?;
    fs::File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("sync manifest directory {}: {error}", parent.display()))
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
struct BattleEvidenceDigest {
    semantic_trace_sha256: String,
    capture_paths: BTreeSet<String>,
}

fn compare_battle_proofs(first: &Path, second: &Path) -> Result<(), String> {
    validate_manifest(first)?;
    validate_manifest(second)?;
    let first_manifest: LcarManifest = read_json(first)?;
    let second_manifest: LcarManifest = read_json(second)?;
    if !first_manifest.passed || !second_manifest.passed {
        return Err("battle comparison requires two passing LCAR manifests".into());
    }
    let first_digest = battle_evidence_digest(first, &first_manifest)?;
    let second_digest = battle_evidence_digest(second, &second_manifest)?;
    if first_digest != second_digest {
        return Err(format!(
            "battle semantic/capture evidence differs: first={first_digest:?}, second={second_digest:?}"
        ));
    }
    Ok(())
}

fn battle_evidence_digest(
    manifest_path: &Path,
    manifest: &LcarManifest,
) -> Result<BattleEvidenceDigest, String> {
    let root = manifest_path
        .parent()
        .ok_or_else(|| "battle manifest has no parent".to_string())?;
    let script = read_json_value(&artifact_path(
        root,
        manifest,
        ArtifactRole::ScriptSnapshot,
    )?)?;
    if script.get("name").and_then(|value| value.as_str()) != Some("battle-v1") {
        return Err("battle comparison LCAR does not retain battle-v1".into());
    }
    let trace = parse_trace(&artifact_path(root, manifest, ArtifactRole::Trace)?)?;
    let mut normalized = Vec::new();
    let mut menu_seed_seen = false;
    let mut battle_seed_seen = false;
    for mut record in trace {
        if matches!(
            record.kind,
            RecordKind::SeedApplication | RecordKind::SemanticAssertion | RecordKind::Capture
        ) {
            record.elapsed_ms = 0;
            record.sequence = normalized.len() as u64;
            record.input_seen = 0;
            record.present_seen = 0;
            if let Some(presentation) = &mut record.presentation {
                presentation.count = 0;
            }
            if let Some(seed) = &record.seed_application {
                if seed.seed != AUTOMATION_SEED {
                    return Err("battle trace contains a noncanonical RNG seed".into());
                }
                match seed.domain {
                    SeedDomain::SuperMeleeMenu => menu_seed_seen = true,
                    SeedDomain::SuperMeleeBattle => battle_seed_seen = true,
                    // Campaign seeding, not part of the battle boundary
                    // evidence this comparison is about.
                    SeedDomain::NewGame => {}
                }
            }
            normalized.push(record);
        }
    }
    if !menu_seed_seen || !battle_seed_seen {
        return Err("battle trace lacks both menu and battle RNG boundary evidence".into());
    }
    let semantic_bytes = serde_json::to_vec(&normalized).map_err(|error| error.to_string())?;
    let capture_paths = manifest
        .artifacts
        .iter()
        .filter(|entry| entry.role == ArtifactRole::Capture)
        .map(|entry| entry.path.clone())
        .collect();
    Ok(BattleEvidenceDigest {
        semantic_trace_sha256: format!("{:x}", Sha256::digest(semantic_bytes)),
        capture_paths,
    })
}

fn run_deterministic_negative_fixtures() -> Result<(), String> {
    #[cfg(test)]
    {
        // Recursing into cargo from inside the very tests this spawns would
        // never terminate.
        Ok(())
    }
    #[cfg(not(test))]
    {
        let status = Command::new("cargo")
            .args([
                "test",
                "--locked",
                "--manifest-path",
                "rust/Cargo.toml",
                "--bin",
                "uqm-gameplay-proof",
                "adversarial_",
            ])
            .status()
            .map_err(|error| format!("run deterministic LCAR mutation tests: {error}"))?;
        if status.success() {
            Ok(())
        } else {
            Err(format!(
                "deterministic LCAR mutation tests failed with {status}"
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct Fixture {
        _temp: tempfile::TempDir,
        path: PathBuf,
    }

    fn record(sequence: u64, kind: RecordKind) -> TraceRecord {
        TraceRecord {
            schema: TraceRecord::SCHEMA,
            run: 1,
            sequence,
            input_seen: sequence,
            present_seen: 1,
            elapsed_ms: sequence,
            kind,
            label: None,
            from: None,
            to: None,
            terminal_reason: None,
            seed_application: None,
            presentation: None,
            activity: None,
            readiness: None,
            command_acknowledgement: None,
            checkpoint: None,
            failure: None,
        }
    }

    fn fixture() -> Fixture {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        fs::create_dir_all(root.join("run/captures")).unwrap();
        fs::create_dir_all(root.join("snapshots")).unwrap();
        fs::write(root.join("stdout.log"), b"").unwrap();
        fs::write(root.join("stderr.log"), b"").unwrap();
        fs::write(root.join("snapshots/uqm"), b"executable").unwrap();
        fs::write(root.join("snapshots/script.json"), b"{}\n").unwrap();
        let target = if cfg!(target_arch = "aarch64") {
            if cfg!(target_os = "macos") {
                "aarch64-apple-darwin"
            } else {
                "aarch64-unknown-linux-gnu"
            }
        } else if cfg!(target_os = "macos") {
            "x86_64-apple-darwin"
        } else {
            "x86_64-unknown-linux-gnu"
        };
        let executable_digest = hash_file(&root.join("snapshots/uqm")).unwrap();
        let production = json!({
            "schema": PRODUCTION_SCHEMA,
            "git_head": "a".repeat(40),
            "dirty": false,
            "target": target,
            "profile": "release",
            "features": PRODUCTION_FEATURES,
            "artifacts": [{"role":"executable","path":"rust/target/release/uqm","sha256":executable_digest}]
        });
        write_new_json(
            &root.join("snapshots/production-manifest.json"),
            &production,
        )
        .unwrap();
        let content = TreeSnapshot {
            schema: "uqm-tree-identity-v1".into(),
            root_role: "content".into(),
            tree_sha256: format!("{:x}", Sha256::digest([])),
            entries: vec![],
        };
        let initial = TreeSnapshot {
            root_role: "initial_config".into(),
            ..content.clone()
        };
        let final_config = TreeSnapshot {
            root_role: "final_config".into(),
            ..content.clone()
        };
        write_new_json(&root.join("snapshots/content-identity.json"), &content).unwrap();
        write_new_json(&root.join("snapshots/config-initial.json"), &initial).unwrap();
        write_new_json(&root.join("snapshots/config-final.json"), &final_config).unwrap();
        fs::write(root.join("run/captures/frame.png"), b"png").unwrap();
        let mut records = vec![record(0, RecordKind::RunStart)];
        let mut present = record(1, RecordKind::Presentation);
        present.presentation = Some(uqm_rust::automation::PresentationEvidence {
            count: 1,
            generation: 1,
            width: 1,
            height: 1,
        });
        records.push(present);
        let mut semantic = record(2, RecordKind::SemanticAssertion);
        semantic.label = Some("battle_progress_passed".into());
        records.push(semantic);
        let mut capture = record(3, RecordKind::Capture);
        capture.label = Some("frame_gen1".into());
        capture.presentation = Some(uqm_rust::automation::PresentationEvidence {
            count: 1,
            generation: 1,
            width: 1,
            height: 1,
        });
        records.push(capture);
        records.push(record(4, RecordKind::RunEnd));
        let trace = records
            .iter()
            .map(|record| record.to_jsonl().unwrap())
            .collect::<String>();
        fs::write(root.join("run/trace.jsonl"), trace).unwrap();
        let teardown = TeardownReceipt {
            schema: "uqm-teardown-v1".into(),
            terminal: Some(TerminalClass::Success),
            game_status: 0,
            process_status: 0,
            runtime_finalized: true,
            runtime_deactivated: true,
            callbacks_quiescent: true,
            trace_durable: true,
        };
        write_new_json(&root.join("run/teardown-complete.json"), &teardown).unwrap();
        let artifacts = collect_artifacts(root).unwrap();
        let canonical_root = fs::canonicalize(root).unwrap();
        let manifest = LcarManifest {
            schema: SCHEMA.into(),
            passed: true,
            first_failed_contract: None,
            git_head: "a".repeat(40),
            command: vec![
                canonical_root.join("snapshots/uqm").display().to_string(),
                "--contentdir=/repo/sc2/content".into(),
                format!("--configdir={}", canonical_root.join("config").display()),
                format!(
                    "--automation-script={}",
                    canonical_root.join("snapshots/script.json").display()
                ),
                format!(
                    "--automation-output={}",
                    canonical_root.join("run").display()
                ),
                "--res=640x480".into(),
                "--windowed".into(),
                "--scroll=pc".into(),
            ],
            environment: BTreeMap::from([
                ("SDL_AUDIODRIVER".into(), "dummy".into()),
                ("SDL_VIDEODRIVER".into(), "dummy".into()),
            ]),
            target: target.into(),
            profile: "release".into(),
            features: PRODUCTION_FEATURES
                .iter()
                .map(|feature| (*feature).into())
                .collect(),
            renderer: "sdl2-software-dummy".into(),
            seed: AUTOMATION_SEED,
            provenance: Provenance {
                production_manifest_sha256: hash_file(
                    &root.join("snapshots/production-manifest.json"),
                )
                .unwrap(),
                executable_sha256: executable_digest,
                script_sha256: hash_file(&root.join("snapshots/script.json")).unwrap(),
                content_tree_sha256: content.tree_sha256,
                initial_config_tree_sha256: initial.tree_sha256,
                final_config_tree_sha256: final_config.tree_sha256,
            },
            process: ProcessReceipt {
                pid: 42,
                start_time: "1".into(),
                executable_sha256: hash_file(&root.join("snapshots/uqm")).unwrap(),
                exit_code: Some(0),
                signal: None,
                term_sent: false,
                kill_sent: false,
                stdout_bytes: 0,
                stderr_bytes: 0,
                output_drained: true,
                orphan_check_passed: true,
            },
            cleanup: CleanupReceipt {
                exact_child_reaped: true,
                orphan_check_passed: true,
                output_drained: true,
                config_root_removed: true,
            },
            artifacts,
        };
        let path = root.join(PASS_FILE);
        write_new_json(&path, &manifest).unwrap();
        Fixture { _temp: temp, path }
    }

    fn mutate_manifest(fixture: &Fixture, mutation: impl FnOnce(&mut serde_json::Value)) {
        let mut value = read_json_value(&fixture.path).unwrap();
        mutation(&mut value);
        fs::write(&fixture.path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    }

    #[test]
    fn valid_fixture_passes_offline_validation() {
        let fixture = fixture();
        validate_manifest(&fixture.path).unwrap();
    }

    #[test]
    fn adversarial_empty_artifact_inventory_fails() {
        let fixture = fixture();
        mutate_manifest(&fixture, |value| value["artifacts"] = json!([]));
        assert!(validate_manifest(&fixture.path).is_err());
    }

    #[test]
    fn adversarial_traversal_path_fails() {
        let fixture = fixture();
        mutate_manifest(&fixture, |value| {
            value["artifacts"][0]["path"] = json!("../escape")
        });
        assert!(validate_manifest(&fixture.path).is_err());
    }

    #[test]
    fn adversarial_duplicate_path_fails() {
        let fixture = fixture();
        mutate_manifest(&fixture, |value| {
            let duplicate = value["artifacts"][0].clone();
            value["artifacts"].as_array_mut().unwrap().push(duplicate);
        });
        assert!(validate_manifest(&fixture.path).is_err());
    }

    #[test]
    fn adversarial_malformed_provenance_fails() {
        let fixture = fixture();
        mutate_manifest(&fixture, |value| {
            value["provenance"]["script_sha256"] = json!("bad")
        });
        assert!(validate_manifest(&fixture.path).is_err());
    }

    #[test]
    fn adversarial_mutated_artifact_fails() {
        let fixture = fixture();
        fs::write(
            fixture
                .path
                .parent()
                .unwrap()
                .join("run/captures/frame.png"),
            b"mutated",
        )
        .unwrap();
        assert!(validate_manifest(&fixture.path).is_err());
    }

    #[test]
    fn adversarial_unknown_manifest_field_fails() {
        let fixture = fixture();
        mutate_manifest(&fixture, |value| value["forged"] = json!(true));
        assert!(validate_manifest(&fixture.path).is_err());
    }

    #[test]
    fn adversarial_forged_failure_contract_fails() {
        let fixture = fixture();
        let failure = fixture.path.parent().unwrap().join(FAILURE_FILE);
        mutate_manifest(&fixture, |value| {
            value["passed"] = json!(false);
            value["first_failed_contract"] = json!("nonzero_child");
        });
        fs::rename(&fixture.path, &failure).unwrap();
        assert!(validate_manifest(&failure).is_err());
    }

    #[test]
    fn adversarial_mutated_trace_sequence_fails() {
        let fixture = fixture();
        let trace = fixture.path.parent().unwrap().join("run/trace.jsonl");
        let text =
            fs::read_to_string(&trace)
                .unwrap()
                .replacen("\"sequence\":1", "\"sequence\":9", 1);
        fs::write(trace, text).unwrap();
        assert!(validate_manifest(&fixture.path).is_err());
    }
}
