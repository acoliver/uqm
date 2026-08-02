use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use uqm_rust::automation::{ChildSession, ChildSessionConfig, AUTOMATION_SEED};

const SCHEMA: &str = "uqm-lcar-v1";
const LOG_BUDGET: u64 = 64 * 1024 * 1024;

#[derive(Debug, Deserialize)]
struct ProductionManifest {
    schema: String,
    git_head: String,
    dirty: bool,
    target: String,
    profile: String,
    features: Vec<String>,
    artifacts: Vec<ProductionArtifact>,
}

#[derive(Debug, Deserialize)]
struct ProductionArtifact {
    role: String,
    path: String,
    sha256: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ArtifactEntry {
    path: String,
    sha256: String,
    bytes: u64,
}

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
struct LcarManifest {
    schema: String,
    git_head: String,
    command: Vec<String>,
    target: String,
    profile: String,
    features: Vec<String>,
    renderer: String,
    seed: u32,
    executable_sha256: String,
    content_sha256: String,
    script_sha256: String,
    initial_config_sha256: String,
    final_config_sha256: String,
    process: ProcessReceipt,
    teardown_receipt: String,
    artifacts: Vec<ArtifactEntry>,
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
        _ => Err("usage: uqm-gameplay-proof run REPO_ROOT PRODUCTION_MANIFEST SCRIPT OUTPUT_ROOT | validate LCAR_MANIFEST".into()),
    }
}

fn run_proof(
    repo_root: &Path,
    production_path: &Path,
    script: &Path,
    output_root: &Path,
) -> Result<(), String> {
    fs::create_dir(output_root)
        .map_err(|error| format!("create fresh output {}: {error}", output_root.display()))?;
    let run_root = output_root.join("run");
    let config_root = output_root.join("config");
    fs::create_dir(&config_root).map_err(|error| format!("create config: {error}"))?;

    let production: ProductionManifest = read_json(production_path)?;
    validate_production(&production)?;
    let executable_artifact = production
        .artifacts
        .iter()
        .find(|artifact| artifact.role == "executable")
        .ok_or_else(|| "production manifest has no executable artifact".to_string())?;
    let executable = repo_root.join(&executable_artifact.path);
    let executable_sha256 = hash_file(&executable)?;
    if executable_sha256 != executable_artifact.sha256 {
        return Err("production executable hash differs from artifact manifest".into());
    }

    let content = repo_root.join("sc2/content");
    let content_sha256 = hash_tree(&content)?;
    let script_sha256 = hash_file(script)?;
    let initial_config_sha256 = hash_tree(&config_root)?;
    let stdout_log = output_root.join("stdout.log");
    let stderr_log = output_root.join("stderr.log");

    let command_args = vec![
        format!("--contentdir={}", content.display()),
        format!("--configdir={}", config_root.display()),
        format!("--automation-script={}", script.display()),
        format!("--automation-output={}", run_root.display()),
        "--res=640x480".into(),
        "--windowed".into(),
    ];
    let mut command = Command::new(&executable);
    command
        .args(&command_args)
        .current_dir(repo_root)
        .env("SDL_VIDEODRIVER", "dummy")
        .env("SDL_AUDIODRIVER", "dummy");

    let config = ChildSessionConfig {
        stdout_log: stdout_log.clone(),
        stderr_log: stderr_log.clone(),
        stdout_budget: LOG_BUDGET,
        stderr_budget: LOG_BUDGET,
        timeout: Duration::from_secs(900),
        grace: Duration::from_secs(5),
        executable_digest: executable_sha256.clone(),
    };
    let session = ChildSession::spawn(command, config).map_err(|error| error.to_string())?;
    let receipt = session.finish().map_err(|error| error.to_string())?;

    let final_config_sha256 = hash_tree(&config_root)?;
    let teardown = run_root.join("teardown-complete.json");
    if !teardown.is_file() {
        return Err(format!("missing teardown receipt {}", teardown.display()));
    }
    if receipt.exit_code != Some(0) || receipt.signal.is_some() {
        return Err(format!(
            "game child failed: exit={:?}, signal={:?}",
            receipt.exit_code, receipt.signal
        ));
    }

    let artifacts = collect_artifacts(output_root)?;
    let manifest_path = output_root.join("lcar-v1.json");
    let manifest = LcarManifest {
        schema: SCHEMA.into(),
        git_head: production.git_head,
        command: std::iter::once(executable.display().to_string())
            .chain(command_args)
            .collect(),
        target: production.target,
        profile: production.profile,
        features: production.features,
        renderer: "sdl-dummy".into(),
        seed: AUTOMATION_SEED,
        executable_sha256,
        content_sha256,
        script_sha256,
        initial_config_sha256,
        final_config_sha256,
        process: ProcessReceipt {
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
        },
        teardown_receipt: relative_path(output_root, &teardown)?,
        artifacts,
    };
    write_new_json(&manifest_path, &manifest)?;
    validate_manifest(&manifest_path)
}

fn validate_production(manifest: &ProductionManifest) -> Result<(), String> {
    if manifest.schema != "uqm-deterministic-artifacts-v4"
        || manifest.dirty
        || manifest.git_head.len() != 40
        || !manifest
            .git_head
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
        || manifest.profile != "release"
        || manifest.features != ["audio_heart", "linked_c_archive"]
    {
        return Err(
            "production artifact manifest is not a clean canonical production build".into(),
        );
    }
    Ok(())
}

fn validate_manifest(path: &Path) -> Result<(), String> {
    let manifest: LcarManifest = read_json(path)?;
    if manifest.schema != SCHEMA
        || manifest.git_head.len() != 40
        || manifest.seed != AUTOMATION_SEED
        || manifest.process.exit_code != Some(0)
        || manifest.process.signal.is_some()
        || !manifest.process.output_drained
        || !manifest.process.orphan_check_passed
        || manifest.process.executable_sha256 != manifest.executable_sha256
    {
        return Err("LCAR identity, process, or teardown fields are invalid".into());
    }
    let root = path
        .parent()
        .ok_or_else(|| "manifest has no parent".to_string())?;
    let expected: BTreeMap<&str, (&str, u64)> = manifest
        .artifacts
        .iter()
        .map(|entry| (entry.path.as_str(), (entry.sha256.as_str(), entry.bytes)))
        .collect();
    for (relative, (digest, bytes)) in expected {
        let artifact = root.join(relative);
        let metadata = fs::metadata(&artifact)
            .map_err(|error| format!("missing artifact {}: {error}", artifact.display()))?;
        if metadata.len() != bytes || hash_file(&artifact)? != digest {
            return Err(format!(
                "artifact identity mismatch: {}",
                artifact.display()
            ));
        }
    }
    if !root.join(&manifest.teardown_receipt).is_file() {
        return Err("teardown receipt is absent".into());
    }
    Ok(())
}

fn collect_artifacts(root: &Path) -> Result<Vec<ArtifactEntry>, String> {
    let mut paths = Vec::new();
    collect_files(root, root, &mut paths)?;
    paths.retain(|path| path.file_name().and_then(|name| name.to_str()) != Some("lcar-v1.json"));
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            let metadata = fs::metadata(&path).map_err(|error| error.to_string())?;
            Ok(ArtifactEntry {
                path: relative_path(root, &path)?,
                sha256: hash_file(&path)?,
                bytes: metadata.len(),
            })
        })
        .collect()
}

fn hash_tree(root: &Path) -> Result<String, String> {
    let mut paths = Vec::new();
    collect_files(root, root, &mut paths)?;
    paths.sort();
    let mut hasher = Sha256::new();
    for path in paths {
        let relative = relative_path(root, &path)?;
        hasher.update(relative.as_bytes());
        hasher.update([0]);
        hasher.update(hash_file(&path)?.as_bytes());
        hasher.update(b"\n");
    }
    Ok(format!("{:x}", hasher.finalize()))
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

fn hash_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn relative_path(root: &Path, path: &Path) -> Result<String, String> {
    path.strip_prefix(root)
        .map_err(|_| format!("{} is outside {}", path.display(), root.display()))
        .map(|relative| relative.to_string_lossy().into_owned())
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let bytes = fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("parse {}: {error}", path.display()))
}

fn write_new_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("create {}: {error}", path.display()))?;
    file.write_all(&bytes).map_err(|error| error.to_string())?;
    file.write_all(b"\n").map_err(|error| error.to_string())?;
    file.flush().map_err(|error| error.to_string())?;
    file.sync_all().map_err(|error| error.to_string())
}
