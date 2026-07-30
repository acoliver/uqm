use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uqm_ownership::{
    apply_toolchain_environment, canonical_build_environment, discover_package_identities,
    read_build_evidence, reject_ambient_build_flags, resolve_toolchain, NativeBuildEvidence,
    ToolchainIdentity, BUILD_EVIDENCE_FILE, BUILD_EVIDENCE_SCHEMA, DEPENDENCY_FLAGS,
    REPOSITORY_INCLUDE_ROOTS,
};

const PRODUCTION_FEATURES: &str = "audio_heart,linked_c_archive";
const NATIVE_INPUTS: &str = "rust/build/native-inputs.json";
const NATIVE_DEPENDENCIES: &str = "rust/build/native-dependencies.json";
const PROVIDER_MANIFEST: &str = "rust/ownership/native-provider-manifest.json";
const MATRIX: &str = "rust/build/supported-matrix.json";
const TREND: &str = "rust/build/native-input-trend.json";
const PRODUCTION_COMMAND: &str =
    "cargo run --locked --manifest-path rust/xtask/Cargo.toml -- production";
const PROVE_COMMAND: &str = "cargo run --locked --manifest-path rust/xtask/Cargo.toml -- prove";
const ARTIFACT_SCHEMA: &str = "uqm-deterministic-artifacts-v3";
const PROOF_COMPARISON: &str = "byte_length_and_sha256_identical";
const CLEAN_BUILD_COUNT: u8 = 2;
const CARGO_BUILD_COMMAND: &str = "cargo build --locked --manifest-path rust/Cargo.toml --release --no-default-features --features audio_heart,linked_c_archive --bin uqm";
const C_ARCHIVE_COMMAND: &str =
    "canonical ar rcs <OUT_DIR>/libuqm_c.a <exact manifest-selected native objects>";
const SIDECAR_COMMAND: &str = "rust/build.rs archive_sidecar_inputs(rust/build/native-inputs.json, rust/ownership/native-provider-manifest.json)";
const PROVIDER_REPORT_COMMAND: &str = "rust/build.rs uqm_ownership::Validator::generate_report()";
const LEDGER_SCHEMA: &str = "uqm-native-ownership-ledger-v5";
const LEDGER_ASSESSMENT_COMMIT: &str = "54e1dba5f56e9f20a3aa773d5f151470a8cf0662";
const LEDGER_RAW_REVISION: &str = "9b0d2a1cced5d0ac3eb73432f765a008053eb81b";
const LEDGER_RAW_URL: &str = "https://gist.githubusercontent.com/acoliver/03378acffcc0d62e7cfd094fc77c223c/raw/9b0d2a1cced5d0ac3eb73432f765a008053eb81b/uqm-native-ownership-ledger.json";
const LEDGER_GIST_REVISION: &str = "519aea3f1f27ba6ac6022dfe08e1520e979cbe1c";
const LEDGER_SHA256: &str = "9fb0c1458aa7364324a294af4afecb8875e103a4e53abd6297418321a167b0b5";
const HASH_TABLE_PROVIDER: &str = "rust/src/collections/hash_table.rs";
const CHAR_HASH_OBJECT: &str = "native/charhashtable.c.o";
const STRING_HASH_OBJECT: &str = "native/stringhashtable.c.o";
const REMOVED_PROVIDERS: [&str; 3] = ["native/heap.c.o", CHAR_HASH_OBJECT, STRING_HASH_OBJECT];
const HASH_TABLE_CUTOVERS: [(&str, &str, &str, &str); 2] = [
    (
        CHAR_HASH_OBJECT,
        HASH_TABLE_PROVIDER,
        "RESOURCE/#22",
        "sc2/src/libs/uio/charhashtable.c",
    ),
    (
        STRING_HASH_OBJECT,
        HASH_TABLE_PROVIDER,
        "CORE_NATIVE/#22",
        "sc2/src/libs/strings/stringhashtable.c",
    ),
];

#[derive(Debug, Deserialize)]
struct Trend {
    schema: String,
    ownership_ledger: TrendLedger,
    current_transitional_inputs: usize,
    maximum_transitional_inputs: usize,
    tracked_native_file_delta: i32,
    infrastructure_delta: InfrastructureDelta,
    removed_providers: Vec<String>,
    provider_cutovers: Vec<ProviderCutover>,
}

#[derive(Debug, Deserialize)]
struct TrendLedger {
    schema: String,
    assessment_commit: String,
    raw_revision: String,
    raw_url: String,
    gist_revision: String,
    sha256: String,
}

#[derive(Debug, Deserialize)]
struct InfrastructureDelta {
    rust_hash_table_provider_cutovers: usize,
}

#[derive(Debug, Deserialize)]
struct ProviderCutover {
    native_provider: String,
    rust_provider: String,
    canonical_owner: String,
    retained_source: String,
}

#[derive(Debug, Deserialize)]
struct Matrix {
    schema: String,
    supported: Vec<SupportedTarget>,
}

#[derive(Debug, Deserialize)]
struct SupportedTarget {
    os: String,
    architectures: Vec<String>,
    renderer: String,
    input: String,
    content: String,
    audio: String,
    network: String,
    package: String,
    prerequisites: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
struct ArtifactManifest {
    schema: String,
    git_head: String,
    tracked_worktree: TrackedWorktree,
    dirty: bool,
    toolchain: ToolchainIdentity,
    source_date_epoch: u64,
    native_build: NativeBuildEvidence,
    command: String,
    target: String,
    profile: String,
    features: Vec<String>,
    artifacts: Vec<Artifact>,
    #[serde(skip_serializing_if = "Option::is_none")]
    determinism_proof: Option<DeterminismProof>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
struct TrackedWorktree {
    file_count: usize,
    sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
struct Artifact {
    role: String,
    path: String,
    media_type: String,
    producing_command: String,
    byte_length: u64,
    sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
struct DeterminismProof {
    command: String,
    clean_builds: u8,
    comparison: String,
    first_build: Vec<ArtifactDigest>,
    second_build: Vec<ArtifactDigest>,
    first_identity: BuildIdentity,
    second_identity: BuildIdentity,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
struct BuildIdentity {
    git_head: String,
    tracked_worktree: TrackedWorktree,
    dirty: bool,
    source_date_epoch: u64,
    toolchain: ToolchainIdentity,
    native_build: NativeBuildEvidence,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
struct ArtifactDigest {
    role: String,
    byte_length: u64,
    sha256: String,
}

#[derive(Debug)]
struct ProductionPaths {
    executable: PathBuf,
    rust_archive: PathBuf,
    c_archive: PathBuf,
    object_sidecar: PathBuf,
    provider_report: PathBuf,
    build_evidence: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Preflight {
    Full,
    ContractOnly,
    PureInspection,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("uqm: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let root = repository_root()?;
    let mut args = env::args().skip(1);
    let command = args.next().ok_or_else(usage)?;
    let extra: Vec<_> = args.collect();
    if !extra.is_empty() {
        return Err(format!(
            "unexpected arguments for '{command}': {}",
            extra.join(" ")
        ));
    }
    run_preflight(&root, preflight_for(&command)?)?;
    match command.as_str() {
        "debug" => cargo(&root, &["build"]),
        "release" => cargo(&root, &["build", "--release"]),
        "test" => test_all(&root),
        "probe" => run_script(&root, "rust/probes/run_p00_probes.sh"),
        "harness" => run_script(&root, "rust/harness/run_p00_harness.sh"),
        "production" => production(&root).map(|_| ()),
        "prove" => prove_determinism(&root),
        "verify" => verify_artifact_manifest(&root),
        "capture-dependencies" => capture_dependencies(&root),
        "package" => package(&root),
        "doctor" => Ok(()),
        "matrix" => print_matrix(&root),
        _ => Err(format!("unknown command after preflight: {command}")),
    }
}

fn usage() -> String {
    "usage: cargo run --manifest-path rust/xtask/Cargo.toml -- <debug|release|test|probe|harness|package|production|prove|verify|capture-dependencies|doctor|matrix>".into()
}

fn preflight_for(command: &str) -> Result<Preflight, String> {
    match command {
        "debug"
        | "release"
        | "probe"
        | "harness"
        | "package"
        | "production"
        | "prove"
        | "capture-dependencies"
        | "doctor" => Ok(Preflight::Full),
        "verify" => Ok(Preflight::ContractOnly),
        "test" => Ok(Preflight::ContractOnly),
        "matrix" => Ok(Preflight::PureInspection),
        _ => Err(format!("unknown command '{command}'\n{}", usage())),
    }
}

fn run_preflight(root: &Path, preflight: Preflight) -> Result<(), String> {
    match preflight {
        Preflight::Full => validate_contract(root, true).map(|_| ()),
        Preflight::ContractOnly => validate_contract(root, false).map(|_| ()),
        Preflight::PureInspection => Ok(()),
    }
}

fn repository_root() -> Result<PathBuf, String> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| "xtask is not inside the repository".into())
}

fn cargo(root: &Path, arguments: &[&str]) -> Result<(), String> {
    let (subcommand, options) = arguments
        .split_first()
        .ok_or_else(|| "Cargo subcommand is missing".to_string())?;
    run_command(
        Command::new("cargo")
            .current_dir(root)
            .arg(subcommand)
            .args(["--locked", "--manifest-path", "rust/Cargo.toml"])
            .args(options),
        "Cargo",
    )
}

fn test_all(root: &Path) -> Result<(), String> {
    cargo(root, &["test", "--workspace", "--all-targets"])
}

fn production(root: &Path) -> Result<ArtifactManifest, String> {
    let epoch = source_date_epoch(root)?;
    env::set_var("SOURCE_DATE_EPOCH", epoch.to_string());
    env::set_var("UQM_BUILD_DATE", source_date(root)?);
    let toolchain = canonical_toolchain(root)?;
    apply_toolchain_environment(&toolchain);
    env::set_var(
        "UQM_CANONICAL_TOOLCHAIN",
        serde_json::to_string(&toolchain)
            .map_err(|error| format!("cannot serialize canonical toolchain: {error}"))?,
    );
    reject_ambient_build_flags()?;
    let paths = cargo_production(root, &toolchain)?;
    let manifest = artifact_manifest(root, paths, None)?;
    write_artifact_manifest(root, &manifest)?;
    Ok(manifest)
}

fn prove_determinism(root: &Path) -> Result<(), String> {
    clean(root)?;
    let first = production(root)?;
    clean(root)?;
    let mut second = production(root)?;
    if first.artifacts != second.artifacts {
        return Err(describe_artifact_difference(
            &first.artifacts,
            &second.artifacts,
        ));
    }
    second.command = PROVE_COMMAND.to_string();
    second.determinism_proof = Some(DeterminismProof {
        command: PROVE_COMMAND.to_string(),
        clean_builds: CLEAN_BUILD_COUNT,
        comparison: PROOF_COMPARISON.to_string(),
        first_build: artifact_digests(&first.artifacts),
        second_build: artifact_digests(&second.artifacts),
        first_identity: build_identity(&first),
        second_identity: build_identity(&second),
    });
    write_artifact_manifest(root, &second)
}

fn clean(root: &Path) -> Result<(), String> {
    run_command(
        Command::new("cargo").current_dir(root).args([
            "clean",
            "--manifest-path",
            "rust/Cargo.toml",
            "--release",
            "-p",
            "uqm",
        ]),
        "Cargo clean",
    )
}

fn cargo_production(root: &Path, toolchain: &ToolchainIdentity) -> Result<ProductionPaths, String> {
    let output = Command::new(&toolchain.cargo.executable)
        .current_dir(root)
        .args([
            "build",
            "--locked",
            "--manifest-path",
            "rust/Cargo.toml",
            "--release",
            "--no-default-features",
            "--features",
            PRODUCTION_FEATURES,
            "--bin",
            "uqm",
            "--message-format=json-render-diagnostics",
        ])
        .output()
        .map_err(|error| format!("cannot execute production Cargo build: {error}"))?;
    eprint!("{}", String::from_utf8_lossy(&output.stderr));
    if !output.status.success() {
        return Err(format!(
            "production Cargo build failed with {}",
            output.status
        ));
    }
    parse_production_paths(&output.stdout)
}

fn parse_production_paths(messages: &[u8]) -> Result<ProductionPaths, String> {
    let text = std::str::from_utf8(messages)
        .map_err(|error| format!("Cargo JSON messages are not UTF-8: {error}"))?;
    let mut out_dirs = BTreeSet::new();
    let mut executables = BTreeSet::new();
    let mut rust_archives = BTreeSet::new();
    for line in text.lines().filter(|line| !line.is_empty()) {
        let message: serde_json::Value = serde_json::from_str(line)
            .map_err(|error| format!("invalid Cargo JSON message: {error}"))?;
        match message["reason"].as_str() {
            Some("build-script-executed") if package_is_uqm(&message) => {
                if let Some(path) = message["out_dir"].as_str() {
                    out_dirs.insert(PathBuf::from(path));
                }
            }
            Some("compiler-artifact") if message["target"]["name"] == "uqm" => {
                if let Some(path) = message["executable"].as_str() {
                    executables.insert(PathBuf::from(path));
                }
            }
            Some("compiler-artifact") if message["target"]["name"] == "uqm_rust" => {
                for path in message["filenames"].as_array().into_iter().flatten() {
                    let Some(path) = path.as_str() else { continue };
                    let filename = Path::new(path)
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or_default();
                    if filename.starts_with("libuqm_rust-") && filename.ends_with(".a") {
                        rust_archives.insert(PathBuf::from(path));
                    }
                }
            }
            _ => {}
        }
    }
    let out_dir = exactly_one(out_dirs, "uqm OUT_DIR")?;
    Ok(ProductionPaths {
        executable: exactly_one(executables, "uqm executable")?,
        rust_archive: exactly_one(rust_archives, "uqm Rust archive")?,
        c_archive: out_dir.join("libuqm_c.a"),
        object_sidecar: out_dir.join("uqm-c-objects.manifest"),
        provider_report: out_dir.join("provider-report.json"),
        build_evidence: out_dir.join(BUILD_EVIDENCE_FILE),
    })
}

fn package_is_uqm(message: &serde_json::Value) -> bool {
    message["package_id"]
        .as_str()
        .is_some_and(|package| package.contains("#uqm@"))
}

fn exactly_one(paths: BTreeSet<PathBuf>, label: &str) -> Result<PathBuf, String> {
    if paths.len() != 1 {
        return Err(format!(
            "current Cargo invocation identified {} candidates for {label}: {paths:?}",
            paths.len()
        ));
    }
    paths
        .into_iter()
        .next()
        .ok_or_else(|| format!("current Cargo invocation did not identify {label}"))
}

fn package(root: &Path) -> Result<(), String> {
    let production = production(root)?;
    let executable = production
        .artifacts
        .iter()
        .find(|item| item.role == "executable")
        .ok_or_else(|| "production invocation did not return an executable artifact".to_string())?;
    let target = host_target()?;
    let parent = root.join("rust/target/uqm-package");
    fs::create_dir_all(&parent)
        .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    let package = parent.join(&target);
    let staging = parent.join(format!(".{target}.staging"));
    if staging.exists() {
        fs::remove_dir_all(&staging)
            .map_err(|error| format!("cannot reset {}: {error}", staging.display()))?;
    }
    fs::create_dir(&staging)
        .map_err(|error| format!("cannot create {}: {error}", staging.display()))?;
    if let Err(error) = populate_package(root, executable, &staging) {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }
    if package.exists() {
        fs::remove_dir_all(&package)
            .map_err(|error| format!("cannot replace {}: {error}", package.display()))?;
    }
    fs::rename(&staging, &package).map_err(|error| {
        let _ = fs::remove_dir_all(&staging);
        format!("cannot atomically install {}: {error}", package.display())
    })
}

fn populate_package(root: &Path, executable: &Artifact, staging: &Path) -> Result<(), String> {
    let source = root.join(&executable.path);
    let destination = staging.join("uqm");
    fs::copy(&source, &destination)
        .map_err(|error| format!("cannot package executable {}: {error}", source.display()))?;
    let packaged = fs::read(&destination).map_err(|error| {
        format!(
            "cannot verify packaged executable {}: {error}",
            destination.display()
        )
    })?;
    if hex_sha256(&packaged) != executable.sha256 {
        return Err("packaged executable digest differs from production invocation".into());
    }
    fs::copy(
        root.join("rust/target/production-artifacts.json"),
        staging.join("production-artifacts.json"),
    )
    .map_err(|error| format!("cannot package artifact manifest: {error}"))?;
    Ok(())
}

fn validate_contract(root: &Path, prerequisites: bool) -> Result<SupportedTarget, String> {
    validate_native_inputs(root)?;
    let target = select_host_target(root)?;
    if prerequisites {
        validate_prerequisites(&target)?;
    }
    Ok(target)
}

fn validate_native_inputs(root: &Path) -> Result<(), String> {
    let manifest = uqm_ownership::load_native_inputs(&root.join(NATIVE_INPUTS))?;
    let dependencies = uqm_ownership::load_native_dependencies(&root.join(NATIVE_DEPENDENCIES))?;
    let providers = uqm_ownership::Manifest::from_file(&root.join(PROVIDER_MANIFEST))
        .map_err(|error| error.to_string())?;
    uqm_ownership::validate_native_authority(root, &manifest, &dependencies, &providers)?;
    let trend: Trend = read_json(&root.join(TREND))?;
    if trend.schema != "uqm-native-input-trend-v1" {
        return Err("unsupported native input trend schema".into());
    }
    validate_trend_authority(&trend)?;
    if trend.tracked_native_file_delta != 0 {
        return Err(format!(
            "contradictory zero-native declaration: tracked delta is {}",
            trend.tracked_native_file_delta
        ));
    }
    if manifest.inputs.len() != trend.current_transitional_inputs
        || manifest.inputs.len() > trend.maximum_transitional_inputs
    {
        return Err(format!(
            "native trend violation: manifest={}, current={}, maximum={}",
            manifest.inputs.len(),
            trend.current_transitional_inputs,
            trend.maximum_transitional_inputs
        ));
    }
    Ok(())
}

fn validate_trend_authority(trend: &Trend) -> Result<(), String> {
    let ledger = &trend.ownership_ledger;
    let actual_ledger = (
        ledger.schema.as_str(),
        ledger.assessment_commit.as_str(),
        ledger.raw_revision.as_str(),
        ledger.raw_url.as_str(),
        ledger.gist_revision.as_str(),
        ledger.sha256.as_str(),
    );
    let expected_ledger = (
        LEDGER_SCHEMA,
        LEDGER_ASSESSMENT_COMMIT,
        LEDGER_RAW_REVISION,
        LEDGER_RAW_URL,
        LEDGER_GIST_REVISION,
        LEDGER_SHA256,
    );
    if actual_ledger != expected_ledger {
        return Err("native input trend is not pinned to authoritative ownership ledger v5".into());
    }
    if trend.infrastructure_delta.rust_hash_table_provider_cutovers != HASH_TABLE_CUTOVERS.len()
        || trend.removed_providers != REMOVED_PROVIDERS
    {
        return Err("native input trend does not record the authorized provider removals".into());
    }
    let actual_cutovers: Vec<_> = trend
        .provider_cutovers
        .iter()
        .map(|cutover| {
            (
                cutover.native_provider.as_str(),
                cutover.rust_provider.as_str(),
                cutover.canonical_owner.as_str(),
                cutover.retained_source.as_str(),
            )
        })
        .collect();
    if actual_cutovers != HASH_TABLE_CUTOVERS {
        return Err("native input trend hash-table cutovers differ from ledger v5".into());
    }
    Ok(())
}

fn select_host_target(root: &Path) -> Result<SupportedTarget, String> {
    let matrix: Matrix = read_json(&root.join(MATRIX))?;
    if matrix.schema != "uqm-supported-matrix-v1" {
        return Err(format!("unsupported matrix schema '{}'", matrix.schema));
    }
    select_target(matrix, env::consts::OS, env::consts::ARCH)
}

fn select_target(matrix: Matrix, os: &str, architecture: &str) -> Result<SupportedTarget, String> {
    matrix
        .supported
        .into_iter()
        .find(|target| target.os == os && target.architectures.iter().any(|item| item == architecture))
        .ok_or_else(|| {
            format!(
                "unsupported target tuple: os={os}, architecture={architecture}, renderer=sdl2-software, input=sdl2, content=uqm-content-v0.8, audio=cpal, network=full, package=directory-manifest"
            )
        })
}

fn validate_prerequisites(target: &SupportedTarget) -> Result<(), String> {
    let root = repository_root()?;
    let toolchain = canonical_toolchain(&root)?;
    if target.renderer != "sdl2-software"
        || target.input != "sdl2"
        || target.content != "uqm-content-v0.8"
        || !matches!(target.audio.as_str(), "cpal" | "cpal-alsa")
        || target.network != "full"
        || target.package != "directory-manifest"
    {
        return Err(format!(
            "contradictory supported matrix row for {}",
            target.os
        ));
    }
    let packages: Vec<_> = target
        .prerequisites
        .iter()
        .filter(|name| !matches!(name.as_str(), "cc" | "ar" | "nm" | "pkg-config"))
        .map(String::as_str)
        .collect();
    run_command(
        Command::new(&toolchain.pkg_config.executable)
            .arg("--exists")
            .args(&packages),
        &format!("pkg-config prerequisites [{}]", packages.join(", ")),
    )
}

fn print_matrix(root: &Path) -> Result<(), String> {
    let matrix = fs::read_to_string(root.join(MATRIX))
        .map_err(|error| format!("cannot read {MATRIX}: {error}"))?;
    print!("{matrix}");
    Ok(())
}

fn capture_dependencies(root: &Path) -> Result<(), String> {
    let epoch = source_date_epoch(root)?;
    env::set_var("SOURCE_DATE_EPOCH", epoch.to_string());
    env::set_var("UQM_BUILD_DATE", source_date(root)?);
    let toolchain = canonical_toolchain(root)?;
    apply_toolchain_environment(&toolchain);
    env::set_var(
        "UQM_CANONICAL_TOOLCHAIN",
        serde_json::to_string(&toolchain)
            .map_err(|error| format!("cannot serialize canonical toolchain: {error}"))?,
    );
    let target = uqm_ownership::target_key(env::consts::OS, env::consts::ARCH)?;
    let path = root
        .join("rust/target")
        .join(format!("native-dependencies-{target}.candidate.json"));
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    env::set_var("UQM_DEPENDENCY_CAPTURE", &path);
    let result = cargo_production(root, &toolchain).map(|_| ());
    env::remove_var("UQM_DEPENDENCY_CAPTURE");
    result?;
    if !path.is_file() {
        return Err(format!(
            "dependency capture was not produced: {}",
            path.display()
        ));
    }
    println!(
        "review target-scoped dependency candidate: {}",
        path.display()
    );
    Ok(())
}

fn artifact_manifest(
    root: &Path,
    paths: ProductionPaths,
    determinism_proof: Option<DeterminismProof>,
) -> Result<ArtifactManifest, String> {
    let artifacts = vec![
        artifact(
            root,
            "executable",
            &paths.executable,
            "application/x-executable",
            CARGO_BUILD_COMMAND,
        )?,
        artifact(
            root,
            "rust_static_archive",
            &paths.rust_archive,
            "application/x-archive",
            CARGO_BUILD_COMMAND,
        )?,
        artifact(
            root,
            "c_static_archive",
            &paths.c_archive,
            "application/x-archive",
            C_ARCHIVE_COMMAND,
        )?,
        artifact(
            root,
            "object_sidecar",
            &paths.object_sidecar,
            "text/plain",
            SIDECAR_COMMAND,
        )?,
        artifact(
            root,
            "provider_report",
            &paths.provider_report,
            "application/json",
            PROVIDER_REPORT_COMMAND,
        )?,
    ];
    let native_build = read_build_evidence(&paths.build_evidence)?;
    Ok(ArtifactManifest {
        schema: ARTIFACT_SCHEMA.to_string(),
        git_head: git_text(root, &["rev-parse", "HEAD"], "full HEAD")?,
        tracked_worktree: tracked_worktree_identity(root)?,
        dirty: tracked_worktree_dirty(root)?,
        toolchain: native_build.toolchain.clone(),
        source_date_epoch: source_date_epoch(root)?,
        native_build,
        command: PRODUCTION_COMMAND.to_string(),
        target: host_target()?,
        profile: "release".to_string(),
        features: vec!["audio_heart".to_string(), "linked_c_archive".to_string()],
        artifacts,
        determinism_proof,
    })
}

fn artifact(
    root: &Path,
    role: &str,
    path: &Path,
    media_type: &str,
    producing_command: &str,
) -> Result<Artifact, String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("production artifact {} is absent: {error}", path.display()))?;
    let canonical_root = root
        .canonicalize()
        .map_err(|error| format!("cannot canonicalize repository root: {error}"))?;
    let canonical_path = path
        .canonicalize()
        .map_err(|error| format!("cannot canonicalize artifact {}: {error}", path.display()))?;
    let relative = canonical_path.strip_prefix(&canonical_root).map_err(|_| {
        format!(
            "production artifact is outside the repository: {}",
            path.display()
        )
    })?;
    let relative = relative
        .to_str()
        .ok_or_else(|| format!("production artifact path is not UTF-8: {}", path.display()))?;
    Ok(Artifact {
        role: role.to_string(),
        path: relative.to_string(),
        media_type: media_type.to_string(),
        producing_command: producing_command.to_string(),
        byte_length: bytes.len() as u64,
        sha256: hex_sha256(&bytes),
    })
}

fn write_artifact_manifest(root: &Path, manifest: &ArtifactManifest) -> Result<(), String> {
    let path = root.join("rust/target/production-artifacts.json");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    let bytes = serde_json::to_vec_pretty(manifest)
        .map_err(|error| format!("cannot serialize artifact manifest: {error}"))?;
    fs::write(&path, [bytes.as_slice(), b"\n"].concat())
        .map_err(|error| format!("cannot write {}: {error}", path.display()))
}

fn verify_artifact_manifest(root: &Path) -> Result<(), String> {
    let path = root.join("rust/target/production-artifacts.json");
    let manifest: ArtifactManifest = read_json(&path)?;
    if manifest.schema != ARTIFACT_SCHEMA
        || manifest.command != PROVE_COMMAND
        || manifest.profile != "release"
        || manifest.features != ["audio_heart", "linked_c_archive"]
    {
        return Err("artifact proof schema or fixed production constants differ".into());
    }
    let toolchain = canonical_toolchain(root)?;
    let epoch = source_date_epoch(root)?;
    validate_live_native_evidence(root, &manifest.native_build, &toolchain, epoch)?;
    if manifest.git_head != git_text(root, &["rev-parse", "HEAD"], "full HEAD")?
        || manifest.tracked_worktree != tracked_worktree_identity(root)?
        || manifest.dirty != tracked_worktree_dirty(root)?
        || manifest.toolchain != toolchain
        || manifest.source_date_epoch != epoch
        || manifest.target != host_target()?
    {
        return Err(
            "production proof is stale for live source, toolchain, target, or SOURCE_DATE_EPOCH"
                .into(),
        );
    }
    let expected_tuples = [
        (
            "executable",
            "application/x-executable",
            CARGO_BUILD_COMMAND,
        ),
        (
            "rust_static_archive",
            "application/x-archive",
            CARGO_BUILD_COMMAND,
        ),
        (
            "c_static_archive",
            "application/x-archive",
            C_ARCHIVE_COMMAND,
        ),
        ("object_sidecar", "text/plain", SIDECAR_COMMAND),
        (
            "provider_report",
            "application/json",
            PROVIDER_REPORT_COMMAND,
        ),
    ];
    if manifest.artifacts.len() != expected_tuples.len() {
        return Err("artifact manifest must contain exactly five entries".into());
    }
    let mut roles = BTreeSet::new();
    let mut paths = BTreeSet::new();
    for (item, (role, media, producer)) in manifest.artifacts.iter().zip(expected_tuples) {
        if (
            item.role.as_str(),
            item.media_type.as_str(),
            item.producing_command.as_str(),
        ) != (role, media, producer)
            || !roles.insert(item.role.as_str())
            || !paths.insert(item.path.as_str())
        {
            return Err(
                "artifact role/path/media/producer tuples are not the exact unique schema".into(),
            );
        }
        uqm_ownership::validate_repo_relative_path(&item.path)?;
        let artifact_path = root.join(&item.path);
        let bytes = fs::read(&artifact_path).map_err(|error| {
            format!(
                "cannot verify artifact {}: {error}",
                artifact_path.display()
            )
        })?;
        if bytes.len() as u64 != item.byte_length || hex_sha256(&bytes) != item.sha256 {
            return Err(format!(
                "live production artifact differs: {}",
                artifact_path.display()
            ));
        }
    }
    let proof = manifest
        .determinism_proof
        .as_ref()
        .ok_or_else(|| "artifact manifest lacks mandatory determinism proof".to_string())?;
    let vector = artifact_digests(&manifest.artifacts);
    let identity = build_identity(&manifest);
    if proof.command != PROVE_COMMAND
        || proof.clean_builds != CLEAN_BUILD_COUNT
        || proof.comparison != PROOF_COMPARISON
        || proof.first_build != vector
        || proof.second_build != vector
        || proof.first_identity != identity
        || proof.second_identity != identity
    {
        return Err(
            "determinism proof constants, vectors, or build identities differ from live manifest"
                .into(),
        );
    }
    Ok(())
}

fn validate_live_native_evidence(
    root: &Path,
    evidence: &NativeBuildEvidence,
    toolchain: &ToolchainIdentity,
    epoch: u64,
) -> Result<(), String> {
    let inputs = uqm_ownership::load_native_inputs(&root.join(NATIVE_INPUTS))?;
    let packages = discover_package_identities(
        root,
        &toolchain.pkg_config,
        &uqm_ownership::PRODUCTION_PACKAGES,
    )?;
    let expected_environment = canonical_build_environment(toolchain, epoch);
    let expected_defines: Vec<_> = [
        format!("-DUQM_BUILD_DATE=\"{}\"", source_date(root)?),
        "-D__DATE__=UQM_BUILD_DATE".into(),
        "-D__TIME__=\"00:00:00\"".into(),
    ]
    .into_iter()
    .chain(
        inputs
            .production_profile
            .defines
            .iter()
            .map(uqm_ownership::PreprocessorDefine::compiler_argument),
    )
    .collect();
    let mut package_include_roots = BTreeSet::new();
    for package in &packages {
        for flag in &package.cflags {
            if let Some(path) = flag.strip_prefix("-I") {
                if path.is_empty() {
                    return Err(format!(
                        "pkg-config emitted an unsupported split include flag for {}",
                        package.name
                    ));
                }
                let include = Path::new(path);
                if !include.is_dir() {
                    return Err(format!(
                        "pkg-config package include is not a directory: {path}"
                    ));
                }
                package_include_roots.insert(path.to_string());
            }
        }
    }
    let mut expected_include_roots: Vec<_> = package_include_roots.into_iter().collect();
    let canonical_root = root
        .canonicalize()
        .map_err(|error| format!("cannot canonicalize repository root: {error}"))?;
    expected_include_roots.extend(
        REPOSITORY_INCLUDE_ROOTS
            .iter()
            .map(|path| canonical_root.join(path).to_string_lossy().into_owned()),
    );
    let expected_template = evidence.compile_profile.compiler_argv(
        Path::new("<canonical-source>"),
        Path::new("<object-output>"),
        Path::new("<depfile>"),
    );
    if evidence.compile_profile.ordered_include_roots != expected_include_roots {
        return Err(format!(
            "native include roots differ: expected {expected_include_roots:?}, got {:?}",
            evidence.compile_profile.ordered_include_roots
        ));
    }
    if evidence.compile_profile.command_template != expected_template {
        return Err(format!(
            "native command template differs: expected {expected_template:?}, got {:?}",
            evidence.compile_profile.command_template
        ));
    }
    if evidence.schema != BUILD_EVIDENCE_SCHEMA
        || evidence.source_date_epoch != epoch
        || evidence.build_date != source_date(root)?
        || evidence.target != uqm_ownership::target_key(env::consts::OS, env::consts::ARCH)?
        || evidence.active_features != inputs.production_profile.cargo_features
        || &evidence.toolchain != toolchain
        || evidence.packages != packages
        || evidence.build_environment != expected_environment
        || evidence.compile_profile.target != evidence.target
        || evidence.compile_profile.compiler != toolchain.cc.executable
        || evidence.compile_profile.ordered_defines != expected_defines
        || evidence.compile_profile.ordered_compile_flags != inputs.production_profile.compile_flags
        || evidence.compile_profile.dependency_flags
            != DEPENDENCY_FLAGS
                .iter()
                .map(|flag| (*flag).to_string())
                .collect::<Vec<_>>()
    {
        return Err(
            "native build evidence differs from live source/toolchain/configuration identity"
                .into(),
        );
    }
    Ok(())
}

fn build_identity(manifest: &ArtifactManifest) -> BuildIdentity {
    BuildIdentity {
        git_head: manifest.git_head.clone(),
        tracked_worktree: manifest.tracked_worktree.clone(),
        dirty: manifest.dirty,
        source_date_epoch: manifest.source_date_epoch,
        toolchain: manifest.toolchain.clone(),
        native_build: manifest.native_build.clone(),
    }
}

fn tracked_worktree_identity(root: &Path) -> Result<TrackedWorktree, String> {
    let files = git_bytes(root, &["ls-files", "-z"], "tracked file inventory")?;
    let file_count = files
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .count();
    if file_count == 0 {
        return Err("tracked worktree identity cannot be empty".into());
    }
    let head = git_bytes(root, &["rev-parse", "HEAD"], "HEAD identity")?;
    let diff = git_bytes(
        root,
        &[
            "diff",
            "--binary",
            "--full-index",
            "--no-ext-diff",
            "HEAD",
            "--",
        ],
        "tracked worktree patch identity",
    )?;
    let mut digest = Sha256::new();
    digest.update((head.len() as u64).to_le_bytes());
    digest.update(head);
    digest.update((diff.len() as u64).to_le_bytes());
    digest.update(diff);
    Ok(TrackedWorktree {
        file_count,
        sha256: digest
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
    })
}

fn git_bytes(root: &Path, arguments: &[&str], label: &str) -> Result<Vec<u8>, String> {
    let output = Command::new("git")
        .current_dir(root)
        .args(arguments)
        .output()
        .map_err(|error| format!("cannot obtain {label} from git: {error}"))?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(format!("git could not provide {label}"))
    }
}

fn tracked_worktree_dirty(root: &Path) -> Result<bool, String> {
    let output = Command::new("git")
        .current_dir(root)
        .args(["status", "--porcelain", "--untracked-files=no"])
        .output()
        .map_err(|error| format!("cannot inspect tracked worktree status: {error}"))?;
    if !output.status.success() {
        return Err("git could not inspect tracked worktree status".into());
    }
    Ok(!output.stdout.is_empty())
}

fn canonical_toolchain(root: &Path) -> Result<ToolchainIdentity, String> {
    resolve_toolchain(root, &host_target()?)
}

fn artifact_digests(artifacts: &[Artifact]) -> Vec<ArtifactDigest> {
    artifacts
        .iter()
        .map(|artifact| ArtifactDigest {
            role: artifact.role.clone(),
            byte_length: artifact.byte_length,
            sha256: artifact.sha256.clone(),
        })
        .collect()
}

fn describe_artifact_difference(first: &[Artifact], second: &[Artifact]) -> String {
    let changed: Vec<_> = first
        .iter()
        .zip(second)
        .filter(|(left, right)| left != right)
        .map(|(left, right)| {
            format!(
                "{}: first={} bytes {}, second={} bytes {}",
                left.role, left.byte_length, left.sha256, right.byte_length, right.sha256
            )
        })
        .collect();
    format!(
        "production artifacts differ across two clean builds: {}",
        changed.join("; ")
    )
}

fn source_date_epoch(root: &Path) -> Result<u64, String> {
    if let Ok(value) = env::var("SOURCE_DATE_EPOCH") {
        return value
            .parse()
            .map_err(|error| format!("invalid SOURCE_DATE_EPOCH '{value}': {error}"));
    }
    git_text(
        root,
        &["show", "-s", "--format=%ct", "HEAD"],
        "source epoch",
    )?
    .parse()
    .map_err(|error| format!("git source epoch is invalid: {error}"))
}

fn source_date(root: &Path) -> Result<String, String> {
    git_text(
        root,
        &[
            "show",
            "-s",
            "--date=format:%b %e %Y",
            "--format=%cd",
            "HEAD",
        ],
        "source date",
    )
}

fn git_text(root: &Path, arguments: &[&str], label: &str) -> Result<String, String> {
    let output = Command::new("git")
        .current_dir(root)
        .args(arguments)
        .output()
        .map_err(|error| format!("cannot obtain {label} from git: {error}"))?;
    if !output.status.success() {
        return Err(format!("git could not provide {label}"));
    }
    String::from_utf8(output.stdout)
        .map(|value| value.trim().to_string())
        .map_err(|error| format!("git {label} is not UTF-8: {error}"))
}

fn host_target() -> Result<String, String> {
    let output = Command::new("rustc")
        .arg("-vV")
        .output()
        .map_err(|error| format!("cannot execute rustc: {error}"))?;
    String::from_utf8(output.stdout)
        .map_err(|error| format!("rustc output is not UTF-8: {error}"))?
        .lines()
        .find_map(|line| line.strip_prefix("host: ").map(str::to_string))
        .ok_or_else(|| "rustc did not report a host target".into())
}

fn run_script(root: &Path, script: &str) -> Result<(), String> {
    run_command(Command::new("bash").current_dir(root).arg(script), script)
}

fn run_command(command: &mut Command, label: &str) -> Result<(), String> {
    let status = command
        .status()
        .map_err(|error| format!("cannot execute {label}: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{label} failed with {status}"))
    }
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    serde_json::from_slice(
        &fs::read(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?,
    )
    .map_err(|error| format!("invalid {}: {error}", path.display()))
}

fn hex_sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_in_trend_pins_ledger_v5_and_both_hash_table_cutovers() {
        let bytes = include_bytes!("../../build/native-input-trend.json");
        let mut trend: Trend = serde_json::from_slice(bytes).unwrap();
        validate_trend_authority(&trend).unwrap();

        trend.provider_cutovers[0].canonical_owner = "S2".into();
        assert!(validate_trend_authority(&trend)
            .unwrap_err()
            .contains("differ from ledger v5"));
    }

    #[test]
    fn every_work_command_has_preflight_and_pure_exceptions_are_explicit() {
        for command in [
            "debug",
            "release",
            "probe",
            "harness",
            "package",
            "production",
            "prove",
            "capture-dependencies",
            "doctor",
        ] {
            assert_eq!(preflight_for(command).unwrap(), Preflight::Full);
        }
        assert_eq!(preflight_for("test").unwrap(), Preflight::ContractOnly);
        assert_eq!(preflight_for("verify").unwrap(), Preflight::ContractOnly);
        assert_eq!(preflight_for("matrix").unwrap(), Preflight::PureInspection);
    }

    #[test]
    fn unsupported_matrix_tuple_reports_every_dimension() {
        let matrix: Matrix =
            serde_json::from_slice(include_bytes!("../../build/supported-matrix.json")).unwrap();
        let message = select_target(matrix, "windows", "x86_64").unwrap_err();
        for dimension in [
            "os=",
            "architecture=",
            "renderer=",
            "input=",
            "content=",
            "audio=",
            "network=",
            "package=",
        ] {
            assert!(message.contains(dimension));
        }
    }
}
