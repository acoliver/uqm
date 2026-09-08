use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use uqm_rust::automation::interrupt;
use uqm_rust::automation::{
    verified_command_digest, ChildSession, ChildSessionConfig, ChildSessionError,
    ChildSessionReceipt, RecordKind, RunLock, SeedDomain, TeardownReceipt, TerminalClass,
    TraceRecord,
};

const USAGE: &str = "usage: uqm-gameplay-proof \
run REPO_ROOT PRODUCTION_MANIFEST SCRIPT OUTPUT_ROOT | \
validate LCAR_MANIFEST | \
validate-negative-fixtures | \
compare-battle FIRST_LCAR SECOND_LCAR | \
list [DOMAIN] | \
select CHANGED_PATH... | \
report BUNDLE_DIR | \
replay REPO_ROOT PRIOR_BUNDLE OUTPUT_ROOT | \
gallery SUITE_ROOT";

const SCHEMA: &str = "uqm-lcar-v2";

/// Every LCAR schema this build can validate.
///
/// Named as a set rather than compared to one constant so a bundle carrying an
/// unknown version is rejected by name, and so adding a version is a visible
/// change here rather than a silent widening.
const SUPPORTED_SCHEMAS: &[&str] = &[SCHEMA];
const FAILURE_FILE: &str = "failure-lcar-v2.json";
const PASS_FILE: &str = "lcar-v2.json";
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
    ConfigRetention,
    ConfigCleanup,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
enum ArtifactRole {
    StdoutLog,
    StderrLog,
    Trace,
    TeardownReceipt,
    ResolvedScenario,
    Capture,
    ProductionManifestSnapshot,
    ExecutableSnapshot,
    ScriptSnapshot,
    ContentIdentitySnapshot,
    ContentSnapshotFile,
    InitialConfigSnapshot,
    FinalConfigSnapshot,
    FinalConfigSnapshotFile,
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
    input_identity: String,
    provenance: Provenance,
    process: ProcessReceipt,
    cleanup: CleanupReceipt,
    artifacts: Vec<ArtifactEntry>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
struct TreeSnapshot {
    schema: String,
    root_role: String,
    tree_sha256: String,
    entries: Vec<TreeEntry>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
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
    seed: u32,
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
        Some("list") if args.len() == 2 => list_scenarios(None),
        Some("list") if args.len() == 3 => list_scenarios(Some(&args[2])),
        Some("select") if args.len() >= 2 => select_scenarios(&args[2..]),
        Some("report") if args.len() == 3 => report_bundle(Path::new(&args[2])),
        Some("gallery") if args.len() == 3 => build_gallery(Path::new(&args[2])),
        Some("replay") if args.len() == 5 => replay_bundle(
            Path::new(&args[2]),
            Path::new(&args[3]),
            Path::new(&args[4]),
        ),
        _ => Err(USAGE.into()),
    }
}

fn run_proof(
    repo_root: &Path,
    production_path: &Path,
    script: &Path,
    output_root: &Path,
) -> Result<(), String> {
    // Record interruptions before the first byte of work. Preparing evidence
    // copies the executable and snapshots the whole content tree, which takes
    // long enough that a signal arriving during it is likely rather than
    // theoretical. Installing after that leaves the longest phase of the run
    // under the default disposition, where a signal kills the process outright
    // and the bundle it leaves behind says nothing about why it stopped.
    interrupt::install().map_err(|error| error.to_string())?;
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
    execute_prepared(&repo_root, &mut evidence)
}

fn execute_prepared(repo_root: &Path, evidence: &mut RunEvidence) -> Result<(), String> {
    // An interruption that arrived while evidence was being prepared stops the
    // run here, before a child exists, and says so. Continuing would spawn the
    // game after the operator already asked for it to stop.
    if let Some(signal) = interrupt::interrupted() {
        let detail = format!("interrupted by signal {signal} while preparing evidence");
        if let Err(failure) = finalize_config_evidence(evidence) {
            return record_config_diagnostic(evidence, None, None, Some(&failure), Some(&detail));
        }
        record_interrupted_preparation(evidence, signal)?;
        return Err(detail);
    }

    // The run owns its output root for as long as it is producing evidence
    // there. Ownership is released when this guard drops, on every path.
    let _ownership = RunLock::acquire(
        &evidence.output_root,
        &evidence.provenance.executable_sha256,
    )
    .map_err(|error| error.to_string())?;
    let receipt = supervise_child(
        repo_root,
        &evidence.output_root.join("snapshots/uqm"),
        &evidence.output_root.join("snapshots/sc2/content"),
        &evidence.output_root.join("snapshots/script.json"),
        evidence,
    );
    complete_run(evidence, receipt)
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
    let content_snapshot = retain_content(content, &snapshots.join("sc2/content"))?;
    let content = snapshots.join("sc2/content");
    let seed = read_validated_script(&snapshots.join("script.json"))?.seed();
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
        seed,
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
        .env_clear()
        .env("SDL_VIDEODRIVER", "dummy")
        .env("SDL_AUDIODRIVER", "dummy");
    // The run must not proceed under a digest nobody checked: the declared
    // provenance is verified against the binary this command would launch.
    if let Err(error) = verified_command_digest(&command, &evidence.provenance.executable_sha256) {
        return Err((
            error,
            Box::new(unavailable_receipt(&evidence.provenance.executable_sha256)),
        ));
    }
    let config = ChildSessionConfig {
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
    // An interruption arriving at the supervisor becomes an observer failure,
    // which routes into the same targeted stop-and-reap path as any other
    // failure. Dying on the signal would leave the game running.
    session
        .finish_observing(|_| match interrupt::interrupted() {
            Some(signal) => Err(interrupt::interruption_error(signal)),
            None => Ok(()),
        })
        .map_err(|failure| (failure.error, failure.receipt))
}

fn complete_run(
    evidence: &mut RunEvidence,
    child_result: Result<ChildSessionReceipt, (ChildSessionError, Box<ChildSessionReceipt>)>,
) -> Result<(), String> {
    complete_run_with(evidence, child_result, finalize_config_evidence)
}

fn complete_run_with(
    evidence: &mut RunEvidence,
    child_result: Result<ChildSessionReceipt, (ChildSessionError, Box<ChildSessionReceipt>)>,
    finalize: impl FnOnce(&mut RunEvidence) -> Result<(), ConfigFinalizationFailure>,
) -> Result<(), String> {
    let (session_contract, process) = match child_result {
        Ok(receipt) => (None, ProcessReceipt::from(receipt)),
        Err((error, receipt)) if receipt.identity.pid != 0 => (
            Some(classify_session_error(&error)),
            ProcessReceipt::from(*receipt),
        ),
        Err((error, _)) => {
            let failure = finalize(evidence).err();
            return record_config_diagnostic(
                evidence,
                Some(classify_session_error(&error)),
                None,
                failure.as_ref(),
                Some(&error.to_string()),
            );
        }
    };
    let first_failed_contract =
        session_contract.or(inspect_child_evidence(evidence, &process).err());
    if let Err(failure) = finalize(evidence) {
        return record_config_diagnostic(
            evidence,
            first_failed_contract,
            Some(&process),
            Some(&failure),
            None,
        );
    }
    let cleanup = CleanupReceipt {
        exact_child_reaped: process.exit_code.is_some() || process.signal.is_some(),
        orphan_check_passed: process.orphan_check_passed,
        output_drained: process.output_drained,
        config_root_removed: config_root_removed(&evidence.config_root)?,
    };
    let passed = first_failed_contract.is_none();
    let artifacts = collect_artifacts(&evidence.output_root)?;
    let mut manifest = LcarManifest {
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
        seed: evidence.seed,
        input_identity: String::new(),
        provenance: evidence.provenance.clone(),
        process,
        cleanup,
        artifacts,
    };
    manifest.input_identity = input_identity(&evidence.output_root, &manifest)?;
    let name = if passed { PASS_FILE } else { FAILURE_FILE };
    let manifest_path = evidence.output_root.join(name);
    write_atomic_new_json(&manifest_path, &manifest)?;
    validate_manifest(&manifest_path)?;
    if passed {
        Ok(())
    } else {
        Err(format!(
            "gameplay proof failed at {:?}; evidence: {}",
            manifest.first_failed_contract,
            manifest_path.display()
        ))
    }
}

fn record_config_diagnostic(
    evidence: &RunEvidence,
    earlier: Option<FailedContract>,
    process: Option<&ProcessReceipt>,
    failure: Option<&ConfigFinalizationFailure>,
    child_error: Option<&str>,
) -> Result<(), String> {
    let removed = config_root_removed(&evidence.config_root);
    let first = earlier.or(failure.map(|failure| failure.contract));
    let bounded = |text: &str| text.chars().take(4096).collect::<String>();
    let document = serde_json::json!({
        "schema": "uqm-config-finalization-failure-v1",
        "passed": false,
        "first_failed_contract": first,
        "process": process,
        "config_root_removed": removed.as_ref().ok(),
        "config_inspection_error": removed.as_ref().err().map(|error| bounded(error)),
        "finalization_failed_contract": failure.map(|failure| failure.contract),
        "finalization_detail": failure.map(|failure| bounded(&failure.detail)),
        "child_error": child_error.map(bounded),
    });
    let detail = format!("proof stopped at {first:?}; finalization: {failure:?}; child: {child_error:?}; config inspection: {removed:?}");
    write_new_json(
        &evidence
            .output_root
            .join("config-finalization-failure.json"),
        &document,
    )
    .map_err(|error| format!("{detail}; diagnostic publication failed: {error}"))?;
    Err(detail)
}

#[derive(Debug, Serialize)]
struct ConfigFinalizationFailure {
    contract: FailedContract,
    detail: String,
}

fn finalize_config_evidence(evidence: &mut RunEvidence) -> Result<(), ConfigFinalizationFailure> {
    finalize_config_with(evidence, write_config_snapshot_file, |path| {
        fs::remove_dir_all(path)
    })
}

fn finalize_config_with(
    evidence: &mut RunEvidence,
    write: impl FnMut(&Path, &[u8]) -> Result<(), String>,
    remove: impl FnOnce(&Path) -> std::io::Result<()>,
) -> Result<(), ConfigFinalizationFailure> {
    let final_config =
        retain_final_config(evidence, write).map_err(|detail| ConfigFinalizationFailure {
            contract: FailedContract::ConfigRetention,
            detail,
        })?;
    evidence.provenance.final_config_tree_sha256 = final_config.tree_sha256;
    remove(&evidence.config_root).map_err(|error| ConfigFinalizationFailure {
        contract: FailedContract::ConfigCleanup,
        detail: format!("remove mutable config root: {error}"),
    })?;
    if !config_root_removed(&evidence.config_root).map_err(|detail| ConfigFinalizationFailure {
        contract: FailedContract::ConfigCleanup,
        detail,
    })? {
        return Err(ConfigFinalizationFailure {
            contract: FailedContract::ConfigCleanup,
            detail: "mutable config root remains after cleanup".into(),
        });
    }
    uqm_rust::automation::artifact::sync_directory(&evidence.output_root).map_err(|error| {
        ConfigFinalizationFailure {
            contract: FailedContract::ConfigCleanup,
            detail: format!("sync mutable config removal: {error}"),
        }
    })?;
    Ok(())
}

fn config_root_removed(path: &Path) -> Result<bool, String> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(false),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(true),
        Err(error) => Err(format!("inspect mutable config root: {error}")),
    }
}

fn retain_final_config(
    evidence: &RunEvidence,
    mut write: impl FnMut(&Path, &[u8]) -> Result<(), String>,
) -> Result<TreeSnapshot, String> {
    let destination = evidence.output_root.join("snapshots/config-final");
    let paths = collect_paths(&evidence.config_root)?;
    fs::create_dir(&destination)
        .map_err(|error| format!("create final config snapshot: {error}"))?;
    let mut entries = Vec::new();
    for path in paths {
        let relative = relative_path(&evidence.config_root, &path)?;
        let bytes = uqm_rust::automation::artifact::read_regular_relative_nofollow(
            &evidence.output_root,
            &Path::new("config").join(&relative),
            LOG_BUDGET,
        )
        .map_err(|error| format!("read final config {relative}: {error}"))?;
        let target = destination.join(&relative);
        let parent = target
            .parent()
            .ok_or_else(|| "final config file lacks parent".to_string())?;
        fs::create_dir_all(parent)
            .map_err(|error| format!("create final config directory: {error}"))?;
        write(&target, &bytes)?;
        for directory in parent
            .ancestors()
            .take_while(|path| path.starts_with(&destination))
        {
            uqm_rust::automation::artifact::sync_directory(directory)
                .map_err(|error| format!("sync final config directory: {error}"))?;
        }
        entries.push(TreeEntry {
            path: relative,
            sha256: format!("{:x}", Sha256::digest(&bytes)),
            bytes: bytes.len() as u64,
        });
    }
    let snapshot = TreeSnapshot {
        schema: "uqm-tree-identity-v1".into(),
        root_role: "final_config".into(),
        tree_sha256: tree_digest(&entries),
        entries,
    };
    // Check the stored bytes and the source again before deleting the mutable profile.
    if snapshot_tree(&destination, "final_config")? != snapshot
        || snapshot_tree(&evidence.config_root, "final_config")? != snapshot
    {
        return Err("final config changed while retaining its snapshot".into());
    }
    write_new_json(
        &evidence.output_root.join("snapshots/config-final.json"),
        &snapshot,
    )?;
    uqm_rust::automation::artifact::sync_directory(&destination)
        .map_err(|error| format!("sync final config snapshot: {error}"))?;
    uqm_rust::automation::artifact::sync_directory(&evidence.output_root.join("snapshots"))
        .map_err(|error| format!("sync snapshots: {error}"))?;
    Ok(snapshot)
}

fn write_config_snapshot_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("create final config file: {error}"))?;
    file.write_all(bytes)
        .map_err(|error| format!("write final config file: {error}"))?;
    let mut permissions = file
        .metadata()
        .map_err(|error| format!("stat final config file: {error}"))?
        .permissions();
    permissions.set_readonly(true);
    file.set_permissions(permissions)
        .map_err(|error| format!("seal final config file: {error}"))?;
    file.sync_all()
        .map_err(|error| format!("sync final config file: {error}"))
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
    validate_loaded_manifest(path, &manifest)
}

fn validate_loaded_manifest(path: &Path, manifest: &LcarManifest) -> Result<(), String> {
    let root = path
        .parent()
        .ok_or_else(|| "LCAR manifest has no parent".to_string())?;
    validate_manifest_identity(path, manifest)?;
    validate_inventory(root, manifest)?;
    validate_provenance(root, manifest)?;
    validate_command(manifest)?;
    validate_scenario_binding(root, manifest)?;
    if manifest.input_identity != input_identity(root, manifest)? {
        return Err("LCAR replay input identity does not match retained inputs".into());
    }
    validate_result(root, manifest)
}

fn read_validated_script(
    path: &Path,
) -> Result<uqm_rust::automation::script::ValidatedScript, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("read script {}: {error}", path.display()))?;
    let doc = uqm_rust::automation::script::parse_script(&bytes, path)
        .map_err(|error| error.to_string())?;
    uqm_rust::automation::script::validate_script(doc, path).map_err(|error| error.to_string())
}

fn input_identity(root: &Path, manifest: &LcarManifest) -> Result<String, String> {
    let script = read_validated_script(&artifact_path(
        root,
        manifest,
        ArtifactRole::ScriptSnapshot,
    )?)?;
    let material = serde_json::json!({
        "schema": "uqm-replay-input-v1",
        "scenario_identity": script.resolved().replay_identity().map_err(|error| error.to_string())?,
        "seed": manifest.seed,
        "git_head": manifest.git_head,
        "target": manifest.target,
        "profile": manifest.profile,
        "features": manifest.features,
        "renderer": manifest.renderer,
        "environment": manifest.environment,
        "production_manifest_sha256": manifest.provenance.production_manifest_sha256,
        "executable_sha256": manifest.provenance.executable_sha256,
        "script_sha256": manifest.provenance.script_sha256,
        "content_tree_sha256": manifest.provenance.content_tree_sha256,
        "initial_config_tree_sha256": manifest.provenance.initial_config_tree_sha256,
    });
    let bytes = serde_json::to_vec(&material).map_err(|error| error.to_string())?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn validate_scenario_binding(root: &Path, manifest: &LcarManifest) -> Result<(), String> {
    let script = read_validated_script(&artifact_path(
        root,
        manifest,
        ArtifactRole::ScriptSnapshot,
    )?)?;
    let record: uqm_rust::automation::lifecycle::ResolvedScenarioRecord = read_json(
        &artifact_path(root, manifest, ArtifactRole::ResolvedScenario)?,
    )?;
    record.validate().map_err(|error| error.to_string())?;
    if record.scenario != script.resolved() {
        return Err("resolved scenario differs from the retained script inputs".into());
    }
    if manifest.seed != script.seed() {
        return Err("LCAR seed differs from the resolved script seed".into());
    }
    if manifest
        .artifacts
        .iter()
        .any(|entry| entry.role == ArtifactRole::Trace)
    {
        for record in parse_trace(&artifact_path(root, manifest, ArtifactRole::Trace)?)? {
            match (&record.kind, &record.seed_application) {
                (RecordKind::SeedApplication, Some(application))
                    if application.seed == script.seed() => {}
                (RecordKind::SeedApplication, _) | (_, Some(_)) => {
                    return Err(
                        "trace RNG seed application differs from the resolved script seed".into(),
                    );
                }
                _ => {}
            }
        }
    }
    Ok(())
}

fn retain_content(source: &Path, destination: &Path) -> Result<TreeSnapshot, String> {
    let expected = snapshot_tree(source, "content")?;
    fs::create_dir_all(destination).map_err(|error| format!("create retained content: {error}"))?;
    for entry in &expected.entries {
        let target = destination.join(&entry.path);
        let parent = target
            .parent()
            .ok_or_else(|| "content entry has no parent".to_string())?;
        fs::create_dir_all(parent).map_err(|error| format!("create content directory: {error}"))?;
        copy_new(&source.join(&entry.path), &target)?;
    }
    let copied = snapshot_tree(destination, "content")?;
    if copied.tree_sha256 != expected.tree_sha256 {
        return Err("content changed while retaining replay inputs".into());
    }
    Ok(copied)
}

/// Print the published matrix, optionally for one domain.
fn list_scenarios(domain: Option<&str>) -> Result<(), String> {
    use uqm_rust::automation::suite::{Domain, MATRIX};

    if let Some(name) = domain {
        let selected = Domain::ALL
            .iter()
            .find(|candidate| candidate.name() == name)
            .ok_or_else(|| {
                format!(
                    "unknown domain {name:?}; known domains: {}",
                    Domain::ALL
                        .iter()
                        .map(|domain| domain.name())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })?;
        for scenario in uqm_rust::automation::suite::scenarios_for(*selected) {
            println!("{scenario}");
        }
        return Ok(());
    }

    for row in MATRIX {
        let domains = row
            .domains
            .iter()
            .map(|domain| domain.name())
            .collect::<Vec<_>>()
            .join(",");
        let tag = match row.fixture {
            Some(kind) => format!("fixture:{kind:?}"),
            None if row.composed_journey => format!("journey:{domains}"),
            None => domains,
        };
        println!("{}\t{tag}", row.scenario);
    }
    Ok(())
}

/// Print the scenarios that must run for the given changed paths.
fn select_scenarios(paths: &[String]) -> Result<(), String> {
    for scenario in uqm_rust::automation::suite::select_for_changed_paths(paths) {
        println!("{scenario}");
    }
    Ok(())
}

/// Index every capture a suite produced, with the scenario that produced it.
///
/// A suite that runs thirty-two scenarios produces captures nobody will open
/// one directory at a time. The index gives each image a scenario, a size and
/// a digest, so a reviewer can see what was actually presented and a later run
/// can be compared against it rather than described.
///
/// Writes `gallery.json` at the suite root and prints a readable summary.
fn build_gallery(suite: &Path) -> Result<(), String> {
    // A single-scenario run is its own bundle. Descending into it as well
    // would find its own run/ directory and count the same captures twice.
    let mut bundles: Vec<PathBuf> = Vec::new();
    if run_dir(suite).join("captures").is_dir() {
        bundles.push(suite.to_path_buf());
    } else {
        let entries =
            fs::read_dir(suite).map_err(|error| format!("read {}: {error}", suite.display()))?;
        for entry in entries {
            let path = entry
                .map_err(|error| format!("read {}: {error}", suite.display()))?
                .path();
            if path.is_dir() && run_dir(&path).join("captures").is_dir() {
                bundles.push(path);
            }
        }
        bundles.sort();
    }
    if bundles.is_empty() {
        return Err(format!(
            "{} contains no scenario bundle with captures",
            suite.display()
        ));
    }

    let mut images = Vec::new();
    for bundle in &bundles {
        let run = run_dir(bundle);
        let scenario = scenario_name(&run).unwrap_or_else(|| {
            bundle
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("unknown")
                .to_string()
        });
        let captures = run.join("captures");
        let mut files: Vec<PathBuf> = fs::read_dir(&captures)
            .map_err(|error| format!("read {}: {error}", captures.display()))?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_file())
            .collect();
        files.sort();
        for file in files {
            let bytes =
                fs::read(&file).map_err(|error| format!("read {}: {error}", file.display()))?;
            let name = file
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .to_string();
            images.push(serde_json::json!({
                "scenario": scenario,
                "checkpoint": name.trim_end_matches(".png"),
                "path": file.strip_prefix(suite).unwrap_or(&file).display().to_string(),
                "byte_length": bytes.len(),
                "sha256": format!("{:x}", Sha256::digest(&bytes)),
            }));
        }
    }

    let gallery = serde_json::json!({
        "schema": "uqm-autoplay-gallery-v1",
        "scenarios": bundles.len(),
        "captures": images.len(),
        "images": images,
    });
    let path = suite.join("gallery.json");
    fs::write(
        &path,
        serde_json::to_vec_pretty(&gallery)
            .map_err(|error| format!("serialize gallery: {error}"))?,
    )
    .map_err(|error| format!("write {}: {error}", path.display()))?;

    println!("scenarios\t{}", bundles.len());
    println!("captures\t{}", images.len());
    for image in &images {
        println!(
            "{}\t{}\t{}",
            image["scenario"].as_str().unwrap_or("?"),
            image["checkpoint"].as_str().unwrap_or("?"),
            image["sha256"].as_str().unwrap_or("?")
        );
    }
    Ok(())
}

/// The scenario a run recorded, if it got far enough to record one.
fn scenario_name(run: &Path) -> Option<String> {
    let text = fs::read_to_string(run.join("resolved-scenario.json")).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    value["scenario"]["name"].as_str().map(str::to_owned)
}

/// Replay only verified retained inputs. Outcome comparison is separate from
/// input identity and excludes pixels and callback/wall-clock timing.
fn replay_bundle(repo_root: &Path, prior: &Path, output_root: &Path) -> Result<(), String> {
    with_verified_replay(prior, |manifest| {
        interrupt::install().map_err(|error| error.to_string())?;
        let repo_root = fs::canonicalize(repo_root)
            .map_err(|error| format!("canonicalize replay repository: {error}"))?;
        let production_path =
            artifact_path(prior, manifest, ArtifactRole::ProductionManifestSnapshot)?;
        let production = parse_production(&read_json_value(&production_path)?)?;
        validate_production(&production, true)?;
        verify_source_binding(&repo_root, &production.git_head)?;
        let prior_outcome = replay_outcome(prior)?;
        let mut evidence = prepare_evidence(
            &repo_root,
            &production_path,
            &production,
            &artifact_path(prior, manifest, ArtifactRole::ExecutableSnapshot)?,
            &artifact_path(prior, manifest, ArtifactRole::ScriptSnapshot)?,
            &prior.join("snapshots/sc2/content"),
            output_root,
        )?;
        // Check the newly copied bytes too, before starting a child. The source
        // bundle may have been changed during preparation by an external writer.
        verify_replay_copy(&evidence, manifest)?;
        execute_prepared(&repo_root, &mut evidence)?;
        let produced: LcarManifest = read_json(&output_root.join(PASS_FILE))?;
        if produced.input_identity != manifest.input_identity {
            return Err("replay input identity differs from the verified prior bundle".into());
        }
        if replay_outcome(output_root)? != prior_outcome {
            return Err("replay inputs match but semantic outcomes differ".into());
        }
        println!("input_identity\t{}", produced.input_identity);
        println!("semantic_outcome\tmatched (timing and pixels not compared)");
        println!("replays\t{}", prior.display());
        Ok(())
    })
}

/// The execution callback cannot be reached with an invalid prior inventory.
fn with_verified_replay<T>(
    prior: &Path,
    execute: impl FnOnce(&LcarManifest) -> Result<T, String>,
) -> Result<T, String> {
    let path = prior.join(PASS_FILE);
    let manifest: LcarManifest = read_json(&path)?;
    validate_loaded_manifest(&path, &manifest)?;
    if !manifest.passed {
        return Err("replay requires a passing prior bundle".into());
    }
    execute(&manifest)
}

fn verify_replay_copy(evidence: &RunEvidence, prior: &LcarManifest) -> Result<(), String> {
    let copied = &evidence.provenance;
    let original = &prior.provenance;
    if copied.production_manifest_sha256 != original.production_manifest_sha256
        || copied.executable_sha256 != original.executable_sha256
        || copied.script_sha256 != original.script_sha256
        || copied.content_tree_sha256 != original.content_tree_sha256
        || copied.initial_config_tree_sha256 != original.initial_config_tree_sha256
        || evidence.seed != prior.seed
    {
        return Err("replay snapshot changed while preparing verified inputs".into());
    }
    Ok(())
}

fn replay_outcome(root: &Path) -> Result<Vec<TraceRecord>, String> {
    let mut outcome = Vec::new();
    for mut record in parse_trace(&root.join("run/trace.jsonl"))? {
        if matches!(
            record.kind,
            RecordKind::SeedApplication
                | RecordKind::SemanticAssertion
                | RecordKind::MenuTransition
                | RecordKind::Checkpoint
                | RecordKind::Terminal
        ) {
            record.elapsed_ms = 0;
            record.sequence = outcome.len() as u64;
            record.input_seen = 0;
            record.present_seen = 0;
            record.presentation = None;
            outcome.push(record);
        }
    }
    Ok(outcome)
}

/// Record that a run stopped, before any child existed, because of a signal.
///
/// A bundle with no explanation is indistinguishable from a machine that
/// vanished, so an interrupted run leaves a document naming the signal rather
/// than a directory of half-copied snapshots.
fn record_interrupted_preparation(evidence: &RunEvidence, signal: i32) -> Result<(), String> {
    let run = evidence.output_root.join("run");
    fs::create_dir_all(&run).map_err(|error| format!("create {}: {error}", run.display()))?;
    let document = serde_json::json!({
        "schema": "uqm-interrupted-preparation-v1",
        "terminal": "interrupted",
        "signal": signal,
        "phase": "prepare-evidence",
        "detail": "the run was interrupted before the game was started, so no child process existed to tear down",
    });
    let path = run.join("teardown-complete.json");
    fs::write(
        &path,
        serde_json::to_vec_pretty(&document)
            .map_err(|error| format!("serialize interruption receipt: {error}"))?,
    )
    .map_err(|error| format!("write {}: {error}", path.display()))
}

/// Where a bundle keeps its run documents.
///
/// A full LCAR bundle nests them under `run/` beside its snapshots; a bare
/// scenario bundle writes them at the top level. Both are produced by this
/// repository, so a reader that understands only one of them silently fails
/// on half the evidence.
fn run_dir(bundle: &Path) -> PathBuf {
    let nested = bundle.join("run");
    if nested.is_dir() {
        nested
    } else {
        bundle.to_path_buf()
    }
}

/// Summarise a produced bundle, including the scenario it actually replayed.
fn report_bundle(bundle: &Path) -> Result<(), String> {
    let run = run_dir(bundle);
    let resolved_path = run.join("resolved-scenario.json");
    let resolved: Option<uqm_rust::automation::lifecycle::ResolvedScenarioRecord> =
        match std::fs::read_to_string(&resolved_path) {
            Ok(text) => Some(
                serde_json::from_str(&text)
                    .map_err(|error| format!("parse {}: {error}", resolved_path.display()))?,
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(format!("read {}: {error}", resolved_path.display())),
        };
    let Some(resolved) = resolved else {
        // The run stopped before it resolved a scenario. Say so plainly and
        // report whatever terminal state it did record, because a bundle from
        // a run that died is the one worth reading.
        println!("scenario\tunresolved: the run stopped before resolving a scenario");
        return report_terminal(&run);
    };

    resolved.validate().map_err(|error| error.to_string())?;
    let scenario = &resolved.scenario;
    println!("scenario\t{}", scenario.name);
    println!("fixture\t{}", scenario.fixture);
    println!("schema\t{}", scenario.schema);
    println!("scenario_input_identity\t{}", resolved.replay_identity);
    println!("scenario_version\t{}", scenario.scenario_version);
    println!("seed\t{}", scenario.seed);
    println!("requested_seed\t{}", scenario.requested_seed);
    println!("steps\t{}", scenario.step_count);

    report_terminal(&run)
}

/// Report whatever terminal state a run recorded, including none.
fn report_terminal(run: &Path) -> Result<(), String> {
    let teardown_path = run.join("teardown-complete.json");
    match std::fs::read_to_string(&teardown_path) {
        Ok(text) => {
            let teardown: serde_json::Value = serde_json::from_str(&text)
                .map_err(|error| format!("parse {}: {error}", teardown_path.display()))?;
            println!("terminal\t{}", teardown["terminal"].as_str().unwrap_or("?"));
            // An interrupted run records why it stopped; a completed one does
            // not carry these, so they are printed when present rather than
            // demanded.
            for field in ["signal", "phase", "detail"] {
                match teardown.get(field) {
                    Some(serde_json::Value::String(text)) => println!("{field}\t{text}"),
                    Some(serde_json::Value::Number(number)) => println!("{field}\t{number}"),
                    _ => {}
                }
            }
        }
        // A bundle without teardown is the signature of a run that died, which
        // is worth reporting plainly rather than failing to summarise.
        Err(_) => println!("terminal\tabsent: the run did not reach teardown"),
    }
    Ok(())
}

fn validate_manifest_identity(path: &Path, manifest: &LcarManifest) -> Result<(), String> {
    let expected_name = if manifest.passed {
        PASS_FILE
    } else {
        FAILURE_FILE
    };
    // One reason per rejection. A single message covering every check cannot
    // tell a stale bundle from a wrong binary from a corrupt field, which is
    // the distinction a reviewer needs first.
    if path.file_name().and_then(|name| name.to_str()) != Some(expected_name) {
        return Err(format!(
            "LCAR result file is named {:?} but a manifest with passed={} must be {expected_name}",
            path.file_name().unwrap_or_default(),
            manifest.passed
        ));
    }
    if !SUPPORTED_SCHEMAS.contains(&manifest.schema.as_str()) {
        return Err(format!(
            "unsupported LCAR schema {:?}; this build validates: {}",
            manifest.schema,
            SUPPORTED_SCHEMAS.join(", ")
        ));
    }
    if !is_hex(&manifest.git_head, 40) {
        return Err(format!(
            "LCAR git head {:?} is not a full 40-hex commit",
            manifest.git_head
        ));
    }
    if manifest.renderer != "sdl2-software-dummy" {
        return Err(format!(
            "LCAR renderer {:?} is not the accepted renderer sdl2-software-dummy",
            manifest.renderer
        ));
    }
    if manifest.profile != "release" {
        return Err(format!(
            "LCAR profile {:?} is not release",
            manifest.profile
        ));
    }
    if manifest.features != PRODUCTION_FEATURES {
        return Err(format!(
            "LCAR features {:?} are not the production features {PRODUCTION_FEATURES:?}",
            manifest.features
        ));
    }
    if !supported_target(&manifest.target) {
        return Err(format!(
            "LCAR target {:?} is not in the supported matrix",
            manifest.target
        ));
    }
    if manifest.process.executable_sha256 != manifest.provenance.executable_sha256 {
        return Err(format!(
            "LCAR was produced by a different binary than it claims: process {} but provenance {}",
            manifest.process.executable_sha256, manifest.provenance.executable_sha256
        ));
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
    let conflicting_result = if manifest.passed {
        FAILURE_FILE
    } else {
        PASS_FILE
    };
    if root
        .join(conflicting_result)
        .try_exists()
        .map_err(|error| format!("inspect conflicting result: {error}"))?
    {
        return Err("LCAR bundle contains conflicting result manifests".into());
    }
    if manifest.artifacts.is_empty() {
        return Err("LCAR artifact inventory is empty".into());
    }
    let mut paths = BTreeSet::new();
    let mut roles = BTreeMap::<ArtifactRole, usize>::new();
    for entry in &manifest.artifacts {
        validate_relative_path(&entry.path)?;
        if role_for_path(&entry.path)? != entry.role {
            return Err(format!("artifact role does not match path: {}", entry.path));
        }
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
    if !initial.entries.is_empty() {
        return Err("initial config must be the fresh empty profile this runner executes".into());
    }
    let retained_content = snapshot_tree(&root.join("snapshots/sc2/content"), "content")?;
    if retained_content.tree_sha256 != content.tree_sha256 {
        return Err("retained content does not match content identity snapshot".into());
    }
    let production_path = artifact_path(root, manifest, ArtifactRole::ProductionManifestSnapshot)?;
    validate_final_config_files(root, manifest, &final_config)?;
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

fn validate_final_config_files(
    root: &Path,
    manifest: &LcarManifest,
    snapshot: &TreeSnapshot,
) -> Result<(), String> {
    let expected: BTreeMap<_, _> = snapshot
        .entries
        .iter()
        .map(|entry| {
            (
                format!("snapshots/config-final/{}", entry.path),
                (entry.sha256.clone(), entry.bytes),
            )
        })
        .collect();
    let actual: BTreeMap<_, _> = manifest
        .artifacts
        .iter()
        .filter(|entry| entry.role == ArtifactRole::FinalConfigSnapshotFile)
        .map(|entry| (entry.path.clone(), (entry.sha256.clone(), entry.bytes)))
        .collect();
    if expected != actual {
        return Err("retained final config does not match final config identity snapshot".into());
    }
    for entry in manifest
        .artifacts
        .iter()
        .filter(|entry| entry.role == ArtifactRole::RetainedConfigFile)
    {
        let relative = entry
            .path
            .strip_prefix("config/")
            .ok_or_else(|| "retained config path is invalid".to_string())?;
        if manifest.cleanup.config_root_removed
            || expected.get(&format!("snapshots/config-final/{relative}"))
                != Some(&(entry.sha256.clone(), entry.bytes))
        {
            return Err("mutable config leftovers differ from retained final config".into());
        }
    }
    for (path, (digest, length)) in actual {
        let bytes = uqm_rust::automation::artifact::read_regular_relative_nofollow(
            root,
            Path::new(&path),
            length,
        )
        .map_err(|error| format!("read retained final config: {error}"))?;
        if bytes.len() as u64 != length || format!("{:x}", Sha256::digest(bytes)) != digest {
            return Err("retained final config bytes differ from identity snapshot".into());
        }
    }
    Ok(())
}

fn validate_command(manifest: &LcarManifest) -> Result<(), String> {
    let executable = manifest
        .command
        .first()
        .ok_or_else(|| "recorded command is empty".to_string())?;
    let executable = Path::new(executable);
    if !executable.is_absolute()
        || executable
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::CurDir))
    {
        return Err("recorded executable path is not absolute and normalized".into());
    }
    // Commands describe the original run location. Revalidate that all operands
    // refer to that one bundle, without opening those old absolute paths when a
    // transported bundle is validated elsewhere.
    let recorded_root = executable
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| "recorded executable lacks a bundle root".to_string())?;
    let expected = vec![
        recorded_root.join("snapshots/uqm").display().to_string(),
        format!(
            "--contentdir={}",
            recorded_root.join("snapshots/sc2/content").display()
        ),
        format!("--configdir={}", recorded_root.join("config").display()),
        format!(
            "--automation-script={}",
            recorded_root.join("snapshots/script.json").display()
        ),
        format!(
            "--automation-output={}",
            recorded_root.join("run").display()
        ),
        "--res=640x480".into(),
        "--windowed".into(),
        "--scroll=pc".into(),
    ];
    if manifest.command != expected {
        return Err("recorded gameplay command is not the exact supported snapshot command".into());
    }
    Ok(())
}

fn validate_result(root: &Path, manifest: &LcarManifest) -> Result<(), String> {
    if manifest.cleanup.orphan_check_passed != manifest.process.orphan_check_passed
        || manifest.cleanup.output_drained != manifest.process.output_drained
        || manifest.cleanup.config_root_removed != config_root_removed(&root.join("config"))?
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
        FailedContract::ConfigRetention => Err(
            "incomplete config retention cannot form an LCAR proof; inspect its diagnostic".into(),
        ),
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
        let relative = relative_path(root, &path)?;
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| format!("metadata {}: {error}", path.display()))?;
        let bytes = uqm_rust::automation::artifact::read_regular_relative_nofollow(
            root,
            Path::new(&relative),
            metadata.len(),
        )
        .map_err(|error| format!("read snapshot {}: {error}", path.display()))?;
        entries.push(TreeEntry {
            path: relative,
            sha256: format!("{:x}", Sha256::digest(&bytes)),
            bytes: bytes.len() as u64,
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
    let mut previous: Option<&str> = None;
    for entry in &snapshot.entries {
        validate_relative_path(&entry.path)?;
        if !paths.insert(&entry.path)
            || !is_hex(&entry.sha256, 64)
            || previous.is_some_and(|path| path >= entry.path.as_str())
        {
            return Err(format!(
                "{role} tree snapshot has duplicate/malformed entries"
            ));
        }
        previous = Some(&entry.path);
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
        if is_lcar_name(&relative) {
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
        "run/resolved-scenario.json" => ArtifactRole::ResolvedScenario,
        "snapshots/production-manifest.json" => ArtifactRole::ProductionManifestSnapshot,
        "snapshots/uqm" => ArtifactRole::ExecutableSnapshot,
        "snapshots/script.json" => ArtifactRole::ScriptSnapshot,
        "snapshots/content-identity.json" => ArtifactRole::ContentIdentitySnapshot,
        "snapshots/config-initial.json" => ArtifactRole::InitialConfigSnapshot,
        "snapshots/config-final.json" => ArtifactRole::FinalConfigSnapshot,
        _ if path.starts_with("snapshots/config-final/") => ArtifactRole::FinalConfigSnapshotFile,
        _ if path.starts_with("run/captures/") && path.ends_with(".png") => ArtifactRole::Capture,
        _ if path.starts_with("config/") => ArtifactRole::RetainedConfigFile,
        _ if path.starts_with("snapshots/sc2/content/") => ArtifactRole::ContentSnapshotFile,
        _ => return Err(format!("unexpected evidence artifact: {path}")),
    };
    Ok(role)
}

fn mandatory_roles() -> [ArtifactRole; 9] {
    [
        // Every bundle must say what the run was attempting, otherwise a
        // proof cannot be tied to the scenario it claims to prove.
        ArtifactRole::ResolvedScenario,
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
    matches!(
        role,
        ArtifactRole::StdoutLog
            | ArtifactRole::StderrLog
            | ArtifactRole::ContentSnapshotFile
            | ArtifactRole::FinalConfigSnapshotFile
            | ArtifactRole::RetainedConfigFile
    )
}

fn collect_relative_files(root: &Path) -> Result<BTreeSet<String>, String> {
    collect_paths(root)?
        .into_iter()
        .map(|path| relative_path(root, &path))
        .filter(|result| {
            result
                .as_ref()
                .map(|path| !is_lcar_name(path))
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
    if !fs::symlink_metadata(current)
        .map_err(|error| format!("inspect directory: {error}"))?
        .is_dir()
    {
        return Err(format!(
            "snapshot directory is not a real directory: {}",
            current.display()
        ));
    }
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
    if first_manifest.input_identity != second_manifest.input_identity {
        return Err("battle comparison requires identical verified replay inputs".into());
    }
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
                if seed.seed != manifest.seed {
                    return Err("battle trace seed differs from its resolved input".into());
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

    #[test]
    fn the_resolved_scenario_is_a_known_and_required_artifact() {
        // The proof refuses artifacts it cannot classify, which is how this
        // file first surfaced. Classify it, and require it: a bundle that
        // cannot say what it was attempting proves nothing in particular.
        assert!(matches!(
            role_for_path("run/resolved-scenario.json"),
            Ok(ArtifactRole::ResolvedScenario)
        ));
        assert!(mandatory_roles().contains(&ArtifactRole::ResolvedScenario));
    }

    #[test]
    fn an_unclassifiable_artifact_is_still_refused() {
        assert!(role_for_path("run/whatever.json").is_err());
    }

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
        write_new_json(&root.join("snapshots/script.json"), &json!({
            "version": 2, "name": "fixture", "fixture": "fixture", "seed": 42,
            "budgets": {"max_input_ticks": 10, "max_presentations": 10, "max_wallclock_seconds": 10},
            "steps": [{"action":"wait_presentations","count":1},
                {"action":"assert_battle_frames","minimum":1},
                {"action":"capture","label":"frame"}, {"action":"finish"}]
        })).unwrap();
        fs::create_dir_all(root.join("snapshots/sc2/content")).unwrap();
        fs::write(root.join("snapshots/sc2/content/test-data"), b"content").unwrap();
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
        let content = snapshot_tree(&root.join("snapshots/sc2/content"), "content").unwrap();
        let initial = TreeSnapshot {
            schema: "uqm-tree-identity-v1".into(),
            root_role: "initial_config".into(),
            tree_sha256: format!("{:x}", Sha256::digest([])),
            entries: vec![],
        };
        fs::create_dir_all(root.join("snapshots/config-final/nested")).unwrap();
        write_config_snapshot_file(
            &root.join("snapshots/config-final/nested/settings.cfg"),
            b"final profile",
        )
        .unwrap();
        write_config_snapshot_file(&root.join("snapshots/config-final/empty"), b"").unwrap();
        let final_config =
            snapshot_tree(&root.join("snapshots/config-final"), "final_config").unwrap();
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
        let mut seed = record(4, RecordKind::SeedApplication);
        seed.seed_application = Some(uqm_rust::automation::trace::SeedApplication {
            domain: SeedDomain::SuperMeleeBattle,
            seed: 42,
        });
        records.push(seed);
        records.push(record(5, RecordKind::RunEnd));
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
        uqm_rust::automation::lifecycle::write_resolved_scenario(
            &root.join("run"),
            &read_validated_script(&root.join("snapshots/script.json"))
                .unwrap()
                .resolved(),
        )
        .unwrap();
        let artifacts = collect_artifacts(root).unwrap();
        let canonical_root = fs::canonicalize(root).unwrap();
        let mut manifest = LcarManifest {
            schema: SCHEMA.into(),
            passed: true,
            first_failed_contract: None,
            git_head: "a".repeat(40),
            command: vec![
                canonical_root.join("snapshots/uqm").display().to_string(),
                format!(
                    "--contentdir={}",
                    canonical_root.join("snapshots/sc2/content").display()
                ),
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
            seed: 42,
            input_identity: String::new(),
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
        manifest.input_identity = input_identity(root, &manifest).unwrap();
        let path = root.join(PASS_FILE);
        write_new_json(&path, &manifest).unwrap();
        Fixture { _temp: temp, path }
    }

    fn pending_config(fixture: &Fixture) -> RunEvidence {
        let root = fixture.path.parent().unwrap();
        let manifest: LcarManifest = read_json(&fixture.path).unwrap();
        fs::remove_file(&fixture.path).unwrap();
        fs::remove_file(root.join("snapshots/config-final.json")).unwrap();
        fs::remove_dir_all(root.join("snapshots/config-final")).unwrap();
        fs::create_dir(root.join("config")).unwrap();
        RunEvidence {
            output_root: fs::canonicalize(root).unwrap(),
            config_root: root.join("config"),
            production: parse_production(
                &read_json_value(&root.join("snapshots/production-manifest.json")).unwrap(),
            )
            .unwrap(),
            seed: manifest.seed,
            command: manifest.command,
            environment: manifest.environment,
            provenance: manifest.provenance,
        }
    }

    #[test]
    fn final_config_bytes_survive_successful_cleanup() {
        let fixture = fixture();
        let mut evidence = pending_config(&fixture);
        fs::create_dir(evidence.config_root.join("nested")).unwrap();
        fs::write(
            evidence.config_root.join("nested/settings.cfg"),
            b"final profile",
        )
        .unwrap();
        fs::write(evidence.config_root.join("empty"), b"").unwrap();
        let receipt = config_fixture_receipt(&evidence);
        complete_run(&mut evidence, Ok(receipt)).unwrap();
        validate_manifest(&fixture.path).unwrap();
        assert!(!evidence.config_root.exists());
        assert_eq!(
            fs::read(
                evidence
                    .output_root
                    .join("snapshots/config-final/nested/settings.cfg")
            )
            .unwrap(),
            b"final profile"
        );
        assert_eq!(
            fs::read(evidence.output_root.join("snapshots/config-final/empty")).unwrap(),
            b""
        );
    }

    fn config_fixture_receipt(evidence: &RunEvidence) -> ChildSessionReceipt {
        let mut receipt = unavailable_receipt(&evidence.provenance.executable_sha256);
        receipt.identity.pid = 42;
        receipt.identity.start_time = "fixture-start".into();
        receipt.exit_code = Some(0);
        receipt.output_drained = true;
        receipt.orphan_check_passed = true;
        receipt
    }

    #[test]
    fn failed_child_final_config_is_retained_and_validates_after_cleanup() {
        for cleanup_fails in [false, true] {
            let fixture = fixture();
            let mut evidence = pending_config(&fixture);
            fs::write(
                evidence.config_root.join("settings.cfg"),
                b"failed run profile",
            )
            .unwrap();
            let mut receipt = config_fixture_receipt(&evidence);
            receipt.exit_code = Some(1);
            receipt.term_sent = true;
            let child_result = Err((
                ChildSessionError::Timeout {
                    term_sent: true,
                    kill_sent: false,
                },
                Box::new(receipt),
            ));
            let result = complete_run_with(&mut evidence, child_result, |evidence| {
                finalize_config_with(evidence, write_config_snapshot_file, |path| {
                    if cleanup_fails {
                        Err(std::io::Error::from_raw_os_error(libc::EACCES))
                    } else {
                        fs::remove_dir_all(path)
                    }
                })
            });
            assert!(result.is_err());
            assert!(!fixture.path.exists());
            assert_eq!(
                fs::read(
                    evidence
                        .output_root
                        .join("snapshots/config-final/settings.cfg")
                )
                .unwrap(),
                b"failed run profile"
            );
            if cleanup_fails {
                let diagnostic = read_json_value(
                    &evidence
                        .output_root
                        .join("config-finalization-failure.json"),
                )
                .unwrap();
                assert_eq!(diagnostic["first_failed_contract"], "timeout");
                assert_eq!(diagnostic["finalization_failed_contract"], "config_cleanup");
                assert_eq!(diagnostic["config_root_removed"], false);
            } else {
                assert!(!evidence.config_root.exists());
                validate_manifest(&evidence.output_root.join(FAILURE_FILE)).unwrap();
            }
        }
    }

    #[test]
    fn empty_final_config_validates_without_a_retained_directory() {
        let fixture = fixture();
        let mut evidence = pending_config(&fixture);
        let receipt = config_fixture_receipt(&evidence);
        complete_run(&mut evidence, Ok(receipt)).unwrap();
        fs::remove_dir(evidence.output_root.join("snapshots/config-final")).unwrap();
        validate_manifest(&fixture.path).unwrap();
    }

    #[test]
    fn final_config_rehashed_inconsistency_missing_and_extra_files_are_rejected() {
        for mutation in [
            "tamper",
            "rehash",
            "missing",
            "missing_reindexed",
            "extra",
            "extra_reindexed",
            "tree_hash",
            "length",
            "order",
        ] {
            let fixture = fixture();
            let root = fixture.path.parent().unwrap();
            let path = root.join("snapshots/config-final/nested/settings.cfg");
            let mut manifest: LcarManifest = read_json(&fixture.path).unwrap();
            match mutation {
                "tamper" | "rehash" => {
                    fs::remove_file(&path).unwrap();
                    fs::write(&path, b"changed profile").unwrap();
                }
                "missing" | "missing_reindexed" => {
                    fs::remove_file(root.join("snapshots/config-final/empty")).unwrap();
                }
                "extra" | "extra_reindexed" => {
                    fs::write(root.join("snapshots/config-final/extra"), b"").unwrap();
                }
                _ => {
                    let tree_path = root.join("snapshots/config-final.json");
                    let mut snapshot: TreeSnapshot = read_json(&tree_path).unwrap();
                    match mutation {
                        "tree_hash" => snapshot.entries[1].sha256 = "a".repeat(64),
                        "length" => snapshot.entries[1].bytes += 1,
                        "order" => snapshot.entries.reverse(),
                        _ => unreachable!(),
                    }
                    snapshot.tree_sha256 = tree_digest(&snapshot.entries);
                    manifest.provenance.final_config_tree_sha256 = snapshot.tree_sha256.clone();
                    fs::write(tree_path, serde_json::to_vec(&snapshot).unwrap()).unwrap();
                }
            }
            if !matches!(mutation, "tamper" | "missing" | "extra") {
                manifest.artifacts = collect_artifacts(root).unwrap();
            }
            assert!(
                validate_loaded_manifest(&fixture.path, &manifest).is_err(),
                "{mutation}"
            );
        }
    }

    #[test]
    fn final_config_disk_and_cleanup_failures_do_not_publish_proof() {
        for boundary in ["write", "cleanup", "earlier_child", "diagnostic"] {
            let fixture = fixture();
            let mut evidence = pending_config(&fixture);
            fs::write(evidence.config_root.join("settings.cfg"), b"retain me").unwrap();
            if boundary == "diagnostic" {
                fs::create_dir(
                    evidence
                        .output_root
                        .join("config-finalization-failure.json"),
                )
                .unwrap();
            }
            let mut receipt = config_fixture_receipt(&evidence);
            if boundary == "earlier_child" {
                receipt.exit_code = Some(3);
            }
            let result = complete_run_with(&mut evidence, Ok(receipt), |evidence| {
                finalize_config_with(
                    evidence,
                    |path, bytes| {
                        if matches!(boundary, "write" | "diagnostic") {
                            let mut file = OpenOptions::new()
                                .create_new(true)
                                .write(true)
                                .open(path)
                                .unwrap();
                            file.write_all(&bytes[..1]).unwrap();
                            return Err(std::io::Error::from_raw_os_error(libc::ENOSPC).to_string());
                        }
                        write_config_snapshot_file(path, bytes)
                    },
                    |_| Err(std::io::Error::from_raw_os_error(libc::EACCES)),
                )
            });
            let error = result.unwrap_err();
            assert!(evidence.config_root.is_dir());
            assert!(!fixture.path.exists());
            assert!(!evidence.output_root.join(FAILURE_FILE).exists());
            if boundary == "diagnostic" {
                assert!(error.contains("diagnostic publication failed"), "{error}");
                assert!(error.contains("ConfigRetention"), "{error}");
                continue;
            }
            let diagnostic = read_json_value(
                &evidence
                    .output_root
                    .join("config-finalization-failure.json"),
            )
            .unwrap();
            assert_eq!(diagnostic["passed"], false);
            assert_eq!(diagnostic["config_root_removed"], false);
            let first = match boundary {
                "write" => "config_retention",
                "earlier_child" => "teardown_evidence",
                _ => "config_cleanup",
            };
            assert_eq!(diagnostic["first_failed_contract"], first);
            assert!(
                fs::metadata(
                    evidence
                        .output_root
                        .join("config-finalization-failure.json")
                )
                .unwrap()
                .len()
                    < 16384
            );
        }
    }

    #[test]
    fn final_config_failure_diagnostics_bound_external_text() {
        let fixture = fixture();
        let evidence = pending_config(&fixture);
        let failure = ConfigFinalizationFailure {
            contract: FailedContract::ConfigRetention,
            detail: "x".repeat(20000),
        };
        assert!(record_config_diagnostic(
            &evidence,
            Some(FailedContract::Timeout),
            None,
            Some(&failure),
            None
        )
        .is_err());
        let diagnostic = read_json_value(
            &evidence
                .output_root
                .join("config-finalization-failure.json"),
        )
        .unwrap();
        assert_eq!(
            diagnostic["finalization_detail"].as_str().unwrap().len(),
            4096
        );
        assert_eq!(diagnostic["first_failed_contract"], "timeout");
        assert_eq!(
            diagnostic["finalization_failed_contract"],
            "config_retention"
        );
        assert!(!fixture.path.exists());
    }

    #[cfg(unix)]
    #[test]
    fn final_config_source_symlinks_and_read_failures_preserve_runtime_state() {
        use std::os::unix::fs::symlink;
        for boundary in ["file", "directory", "root", "missing", "destination"] {
            let fixture = fixture();
            let mut evidence = pending_config(&fixture);
            let outside = tempfile::tempdir().unwrap();
            fs::write(outside.path().join("secret"), b"must not copy").unwrap();
            match boundary {
                "file" => symlink(
                    outside.path().join("secret"),
                    evidence.config_root.join("secret"),
                )
                .unwrap(),
                "directory" => {
                    symlink(outside.path(), evidence.config_root.join("external")).unwrap()
                }
                "root" => {
                    fs::remove_dir(&evidence.config_root).unwrap();
                    symlink(outside.path(), &evidence.config_root).unwrap();
                }
                "missing" => fs::remove_dir(&evidence.config_root).unwrap(),
                "destination" => fs::write(
                    evidence.output_root.join("snapshots/config-final"),
                    b"collision",
                )
                .unwrap(),
                _ => unreachable!(),
            }
            let receipt = config_fixture_receipt(&evidence);
            assert!(
                complete_run(&mut evidence, Ok(receipt)).is_err(),
                "{boundary}"
            );
            assert!(!fixture.path.exists());
            let diagnostic = read_json_value(
                &evidence
                    .output_root
                    .join("config-finalization-failure.json"),
            )
            .unwrap();
            assert_eq!(diagnostic["first_failed_contract"], "config_retention");
            assert_eq!(diagnostic["config_root_removed"], boundary == "missing");
            assert_eq!(
                fs::read(outside.path().join("secret")).unwrap(),
                b"must not copy"
            );
        }
    }

    #[test]
    fn final_config_source_changes_during_copy_stop_before_cleanup() {
        for boundary in ["read", "change", "oversize"] {
            let fixture = fixture();
            let mut evidence = pending_config(&fixture);
            let source = evidence.config_root.clone();
            fs::write(source.join("a"), b"first").unwrap();
            fs::write(source.join("b"), b"second").unwrap();
            if boundary == "oversize" {
                OpenOptions::new()
                    .write(true)
                    .open(source.join("b"))
                    .unwrap()
                    .set_len(LOG_BUDGET + 1)
                    .unwrap();
            }
            let receipt = config_fixture_receipt(&evidence);
            let error = complete_run_with(&mut evidence, Ok(receipt), |evidence| {
                finalize_config_with(
                    evidence,
                    |path, bytes| {
                        write_config_snapshot_file(path, bytes)?;
                        if path.file_name().unwrap() == "a" {
                            match boundary {
                                "read" => fs::remove_file(source.join("b")).unwrap(),
                                "change" => fs::write(source.join("a"), b"changed source").unwrap(),
                                _ => {}
                            }
                        }
                        Ok(())
                    },
                    |path| fs::remove_dir_all(path),
                )
            })
            .unwrap_err();
            assert!(error.contains("ConfigRetention"), "{error}");
            assert!(source.is_dir());
            assert!(!fixture.path.exists());
            let diagnostic = read_json_value(
                &evidence
                    .output_root
                    .join("config-finalization-failure.json"),
            )
            .unwrap();
            assert_eq!(diagnostic["first_failed_contract"], "config_retention");
            assert_eq!(diagnostic["config_root_removed"], false);
        }
    }

    #[test]
    fn spawn_failure_still_retains_and_cleans_config_without_claiming_a_child() {
        let fixture = fixture();
        let mut evidence = pending_config(&fixture);
        fs::write(evidence.config_root.join("settings.cfg"), b"retain me").unwrap();
        let receipt = unavailable_receipt(&evidence.provenance.executable_sha256);
        let error = ChildSessionError::Spawn(std::io::Error::other("spawn fixture"));
        assert!(complete_run(&mut evidence, Err((error, Box::new(receipt)))).is_err());
        assert!(!evidence.config_root.exists());
        assert_eq!(
            fs::read(
                evidence
                    .output_root
                    .join("snapshots/config-final/settings.cfg")
            )
            .unwrap(),
            b"retain me"
        );
        let diagnostic = read_json_value(
            &evidence
                .output_root
                .join("config-finalization-failure.json"),
        )
        .unwrap();
        assert!(diagnostic["process"].is_null());
        assert_eq!(diagnostic["config_root_removed"], true);
        assert!(!fixture.path.exists());
    }

    fn mutate_manifest(fixture: &Fixture, mutation: impl FnOnce(&mut serde_json::Value)) {
        let mut value = read_json_value(&fixture.path).unwrap();
        mutation(&mut value);
        fs::write(&fixture.path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    }

    #[test]
    fn replay_rejects_tampered_prior_before_source_check_or_spawn() {
        let fixture = fixture();
        let prior = fixture.path.parent().unwrap();
        fs::write(prior.join("snapshots/script.json"), b"tampered").unwrap();
        let output = prior.join("must-not-be-created");
        let error =
            replay_bundle(Path::new("/nonexistent-replay-repo"), prior, &output).unwrap_err();
        assert!(error.contains("artifact identity mismatch"), "{error}");
        assert!(!output.exists());
    }

    #[test]
    fn replay_untouched_inputs_reach_preparation_using_only_retained_bytes() {
        let fixture = fixture();
        let original = fixture.path.parent().unwrap();
        let relocated = tempfile::tempdir().unwrap();
        let prior = relocated.path().join("prior");
        fs::rename(original, &prior).unwrap();
        assert!(!original.exists());
        let prior = prior.as_path();
        let destination = tempfile::tempdir().unwrap();
        with_verified_replay(prior, |manifest| {
            let production_path =
                artifact_path(prior, manifest, ArtifactRole::ProductionManifestSnapshot)?;
            let production = parse_production(&read_json_value(&production_path)?)?;
            let evidence = prepare_evidence(
                Path::new("/unused-repository"),
                &production_path,
                &production,
                &artifact_path(prior, manifest, ArtifactRole::ExecutableSnapshot)?,
                &artifact_path(prior, manifest, ArtifactRole::ScriptSnapshot)?,
                &prior.join("snapshots/sc2/content"),
                &destination.path().join("replay"),
            )?;
            verify_replay_copy(&evidence, manifest)?;
            assert!(collect_paths(&evidence.config_root).unwrap().is_empty());
            assert!(!evidence.output_root.join("snapshots/config-final").exists());
            assert!(
                !read_tree_snapshot(prior, manifest, ArtifactRole::FinalConfigSnapshot)?
                    .entries
                    .is_empty()
            );
            assert_eq!(
                fs::read(evidence.output_root.join("snapshots/uqm")).unwrap(),
                b"executable"
            );
            assert_eq!(
                fs::read(evidence.output_root.join("snapshots/sc2/content/test-data")).unwrap(),
                b"content"
            );
            assert_eq!(evidence.seed, 42);
            assert_eq!(
                evidence.command[1],
                format!(
                    "--contentdir={}",
                    evidence.output_root.join("snapshots/sc2/content").display()
                )
            );
            let mut changed = manifest.clone();
            changed.seed = 43;
            assert!(verify_replay_copy(&evidence, &changed).is_err());
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn replay_untouched_relocated_bundle_remains_verifiable() {
        let fixture = fixture();
        let destination = tempfile::tempdir().unwrap();
        let original = fixture.path.parent().unwrap();
        for source in collect_paths(original).unwrap() {
            let target = destination
                .path()
                .join(source.strip_prefix(original).unwrap());
            fs::create_dir_all(target.parent().unwrap()).unwrap();
            fs::copy(source, target).unwrap();
        }
        with_verified_replay(destination.path(), |_| Ok(())).unwrap();
    }

    #[test]
    fn replay_uninventoried_temporary_files_and_conflicting_results_are_rejected() {
        for path in ["unrecorded.tmp", FAILURE_FILE] {
            let fixture = fixture();
            let root = fixture.path.parent().unwrap();
            fs::write(root.join(path), b"unexpected evidence").unwrap();
            let mut executed = false;
            assert!(
                with_verified_replay(root, |_| {
                    executed = true;
                    Ok(())
                })
                .is_err(),
                "{path}"
            );
            assert!(!executed);
        }
    }

    #[test]
    fn replay_tampered_inventory_never_reaches_execution_callback() {
        for path in [
            "snapshots/uqm",
            "snapshots/script.json",
            "snapshots/sc2/content/test-data",
            "snapshots/config-initial.json",
            "run/resolved-scenario.json",
            "run/trace.jsonl",
        ] {
            let fixture = fixture();
            let prior = fixture.path.parent().unwrap();
            fs::write(prior.join(path), b"tampered input").unwrap();
            let mut executed = false;
            let result = with_verified_replay(prior, |_| {
                executed = true;
                Ok(())
            });
            assert!(result.is_err(), "{path}");
            assert!(!executed, "{path}");
        }
    }

    fn refresh_inventory(fixture: &Fixture) -> LcarManifest {
        let mut manifest: LcarManifest = read_json(&fixture.path).unwrap();
        manifest.artifacts = collect_artifacts(fixture.path.parent().unwrap()).unwrap();
        manifest
    }

    #[test]
    fn replay_rehashed_same_count_script_mutation_is_not_the_recorded_scenario() {
        let fixture = fixture();
        let root = fixture.path.parent().unwrap();
        let path = root.join("snapshots/script.json");
        let mut script = read_json_value(&path).unwrap();
        script["steps"][1]["minimum"] = json!(2);
        fs::write(&path, serde_json::to_vec(&script).unwrap()).unwrap();
        let mut manifest = refresh_inventory(&fixture);
        manifest.provenance.script_sha256 = hash_file(&path).unwrap();
        manifest.input_identity = input_identity(root, &manifest).unwrap();
        fs::write(&fixture.path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        let error = with_verified_replay(root, |_| -> Result<(), String> {
            panic!("must not execute")
        })
        .unwrap_err();
        assert!(error.contains("resolved scenario differs"), "{error}");
    }

    #[test]
    fn replay_zero_request_binds_the_nonzero_seed_that_is_actually_applied() {
        let fixture = fixture();
        let root = fixture.path.parent().unwrap();
        let old: LcarManifest = read_json(&fixture.path).unwrap();
        let script_path = root.join("snapshots/script.json");
        let mut source = read_json_value(&script_path).unwrap();
        source["seed"] = json!(0);
        fs::write(&script_path, serde_json::to_vec(&source).unwrap()).unwrap();
        let scenario = read_validated_script(&script_path).unwrap().resolved();
        assert_eq!(scenario.requested_seed, 0);
        assert_eq!(scenario.seed, 1);
        let record = uqm_rust::automation::lifecycle::ResolvedScenarioRecord {
            replay_identity: scenario.replay_identity().unwrap(),
            scenario,
        };
        fs::write(
            root.join("run/resolved-scenario.json"),
            serde_json::to_vec(&record).unwrap(),
        )
        .unwrap();
        let trace_path = root.join("run/trace.jsonl");
        let mut records = parse_trace(&trace_path).unwrap();
        records[4].seed_application.as_mut().unwrap().seed = 1;
        fs::write(
            &trace_path,
            records
                .iter()
                .map(|record| record.to_jsonl().unwrap())
                .collect::<String>(),
        )
        .unwrap();
        let mut manifest = refresh_inventory(&fixture);
        manifest.seed = 1;
        manifest.provenance.script_sha256 = hash_file(&script_path).unwrap();
        manifest.input_identity = input_identity(root, &manifest).unwrap();
        assert_ne!(manifest.input_identity, old.input_identity);
        fs::write(&fixture.path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        with_verified_replay(root, |_| Ok(())).unwrap();
    }

    #[test]
    fn replay_rehashed_wrong_applied_seed_is_rejected() {
        let fixture = fixture();
        let root = fixture.path.parent().unwrap();
        let path = root.join("run/trace.jsonl");
        let mut records = parse_trace(&path).unwrap();
        records[4].seed_application.as_mut().unwrap().seed = 43;
        fs::write(
            &path,
            records
                .iter()
                .map(|record| record.to_jsonl().unwrap())
                .collect::<String>(),
        )
        .unwrap();
        let manifest = refresh_inventory(&fixture);
        fs::write(&fixture.path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        let error = with_verified_replay(root, |_| -> Result<(), String> {
            panic!("must not execute")
        })
        .unwrap_err();
        assert!(
            error.contains("trace RNG seed application differs"),
            "{error}"
        );
    }

    #[test]
    fn replay_input_identity_binds_provenance_but_not_outcomes() {
        let fixture = fixture();
        let root = fixture.path.parent().unwrap();
        let manifest: LcarManifest = read_json(&fixture.path).unwrap();
        let expected = input_identity(root, &manifest).unwrap();
        for mutate in [
            (|m: &mut LcarManifest| m.seed += 1) as fn(&mut LcarManifest),
            |m| m.provenance.executable_sha256 = "0".repeat(64),
            |m| m.provenance.script_sha256 = "0".repeat(64),
            |m| m.provenance.content_tree_sha256 = "0".repeat(64),
            |m| m.provenance.initial_config_tree_sha256 = "0".repeat(64),
            |m| m.provenance.production_manifest_sha256 = "0".repeat(64),
        ] {
            let mut changed = manifest.clone();
            mutate(&mut changed);
            assert_ne!(expected, input_identity(root, &changed).unwrap());
        }
        let mut changed_outcome = manifest;
        changed_outcome.provenance.final_config_tree_sha256 = "0".repeat(64);
        changed_outcome.process.exit_code = Some(1);
        assert_eq!(expected, input_identity(root, &changed_outcome).unwrap());
    }

    #[test]
    fn replay_outcomes_compare_semantics_without_claiming_pixel_or_timing_equality() {
        let fixture = fixture();
        let root = fixture.path.parent().unwrap();
        let expected = replay_outcome(root).unwrap();
        let trace_path = root.join("run/trace.jsonl");
        let mut records = parse_trace(&trace_path).unwrap();
        for record in &mut records {
            record.elapsed_ms += 100;
            record.present_seen += 10;
            record.input_seen += 20;
        }
        fs::write(
            &trace_path,
            records
                .iter()
                .map(|record| record.to_jsonl().unwrap())
                .collect::<String>(),
        )
        .unwrap();
        fs::write(root.join("run/captures/frame.png"), b"different pixels").unwrap();
        assert_eq!(expected, replay_outcome(root).unwrap());
        records[2].label = Some("different_semantic_outcome".into());
        fs::write(
            &trace_path,
            records
                .iter()
                .map(|record| record.to_jsonl().unwrap())
                .collect::<String>(),
        )
        .unwrap();
        assert_ne!(expected, replay_outcome(root).unwrap());
    }

    #[test]
    fn valid_fixture_passes_offline_validation() {
        let fixture = fixture();
        validate_manifest(&fixture.path).unwrap();
    }

    /// Each rejection category #32 names must be distinguishable by its message.
    ///
    /// Asserting only that validation failed would pass even if every cause
    /// collapsed back into one opaque string, which is what this replaced.
    /// A named rejection case: what to break, and the phrase that must name it.
    type RejectionCase<'a> = (&'a str, &'a dyn Fn(&mut serde_json::Value), &'a str);

    #[test]
    fn each_rejection_names_its_own_cause() {
        let cases: [RejectionCase<'_>; 6] = [
            (
                "unknown schema",
                &|value| value["schema"] = json!("uqm-lcar-v99"),
                "unsupported LCAR schema",
            ),
            (
                "corrupt commit",
                &|value| value["git_head"] = json!("not-a-commit"),
                "not a full 40-hex commit",
            ),
            (
                "wrong seed",
                &|value| value["seed"] = json!(1234),
                "LCAR seed differs from the resolved script seed",
            ),
            (
                "wrong renderer",
                &|value| value["renderer"] = json!("opengl"),
                "not the accepted renderer",
            ),
            (
                "wrong profile",
                &|value| value["profile"] = json!("debug"),
                "is not release",
            ),
            (
                "wrong binary",
                &|value| {
                    value["process"]["executable_sha256"] = json!("0".repeat(64));
                },
                "produced by a different binary",
            ),
        ];

        for (name, mutate, expected) in cases {
            let fixture = fixture();
            mutate_manifest(&fixture, mutate);
            let error =
                validate_manifest(&fixture.path).expect_err(&format!("{name} must be rejected"));
            assert!(
                error.contains(expected),
                "{name}: expected a message containing {expected:?}, got {error:?}"
            );
        }
    }

    #[test]
    fn the_supported_schema_set_is_what_the_writer_emits() {
        // A validator that accepts a version nothing writes, or rejects the
        // one it does, is worse than no version check.
        assert!(SUPPORTED_SCHEMAS.contains(&SCHEMA));
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
