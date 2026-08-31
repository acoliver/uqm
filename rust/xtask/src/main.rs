use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uqm_ownership::{
    apply_toolchain_environment, canonical_build_environment, discover_package_identities,
    read_build_evidence, reject_ambient_build_flags, reject_noncanonical_build_flags,
    resolve_toolchain, NativeBuildEvidence, ProductionArtifacts, ProductionToolPaths,
    ToolchainIdentity, Validator, BUILD_EVIDENCE_FILE, BUILD_EVIDENCE_SCHEMA, DEPENDENCY_FLAGS,
    REPOSITORY_INCLUDE_ROOTS,
};
use uqm_rust::automation::{
    NativeLinkedBuildReceipt, NativeRetainedInput, NATIVE_LINKED_BUILD_RECEIPT_SCHEMA,
};

mod ci;
#[allow(dead_code)]
#[path = "../../src/bin/uqm-native-acceptance.rs"]
mod native_acceptance_runner;

const PRODUCTION_FEATURES: &str = "audio_heart,linked_c_archive";
const NATIVE_INPUTS: &str = "rust/build/native-inputs.json";
const NATIVE_DEPENDENCIES: &str = "rust/build/native-dependencies.json";
const NATIVE_ACCEPTANCE_PRECREATED_ROOT_ENV: &str = "UQM_CI_NATIVE_ACCEPTANCE_PRECREATED_ROOT";
const PROVIDER_MANIFEST: &str = "rust/ownership/native-provider-manifest.json";
const MATRIX: &str = "rust/build/supported-matrix.json";
const TREND: &str = "rust/build/native-input-trend.json";
const PRODUCTION_COMMAND: &str =
    "cargo run --locked --manifest-path rust/xtask/Cargo.toml -- production";
const PROVE_COMMAND: &str = "cargo run --locked --manifest-path rust/xtask/Cargo.toml -- prove";
const ARTIFACT_SCHEMA: &str = "uqm-deterministic-artifacts-v4";
const PROOF_COMPARISON: &str = "byte_length_and_sha256_identical";
const CLEAN_BUILD_COUNT: u8 = 2;
const CARGO_BUILD_COMMAND: &str = "cargo build --locked --manifest-path rust/Cargo.toml --release --no-default-features --features audio_heart,linked_c_archive --bin uqm";
const C_ARCHIVE_COMMAND: &str =
    "canonical ar rcs <OUT_DIR>/libuqm_c.a <exact manifest-selected native objects>";
const SIDECAR_COMMAND: &str = "rust/build.rs archive_sidecar_inputs(rust/build/native-inputs.json, rust/ownership/native-provider-manifest.json)";
const PROVIDER_REPORT_COMMAND: &str = "rust/build.rs uqm_ownership::Validator::generate_report()";
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
    cargo_feature_graph: Vec<CargoFeatureEntry>,
    artifacts: Vec<Artifact>,
    #[serde(skip_serializing_if = "Option::is_none")]
    determinism_proof: Option<DeterminismProof>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
struct CargoFeatureEntry {
    name: String,
    version: String,
    features: Vec<String>,
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

struct LinkedBuildProof {
    directory: tempfile::TempDir,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Preflight {
    /// Full prerequisites plus strict source identity (reject untracked).
    StrictSource,
    /// Full prerequisites and contract validation.
    Full,
    /// Contract validation only (no external prerequisites).
    ContractOnly,
    /// Pure inspection, no validation.
    PureInspection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Profile {
    Debug,
    Release,
}

impl Profile {
    fn flag(&self) -> Option<&'static str> {
        match self {
            Self::Debug => None,
            Self::Release => Some("--release"),
        }
    }
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

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn run_containment_hidden_command(
    command: &str,
    arguments: &[String],
) -> Option<Result<(), String>> {
    (command == ci::exec::CONTAINMENT_ESCAPE_HELPER_COMMAND)
        .then(|| ci::exec::run_containment_escape_helper(arguments))
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let command = args.next().ok_or_else(usage)?;
    #[cfg(unix)]
    if command == ci::exec::CONTAINMENT_MONITOR_COMMAND {
        return ci::exec::run_containment_monitor_from_env();
    }
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    if command == ci::exec::CONTAINMENT_ESCAPE_HELPER_COMMAND {
        return run_containment_hidden_command(&command, &args.collect::<Vec<_>>())
            .expect("matched Darwin hidden command must dispatch");
    }
    if command == "observer-helper" || command == "__ci-native-acceptance" {
        let mut native_arguments = vec!["uqm-native-acceptance".to_string()];
        if command == "observer-helper" {
            native_arguments.push(command);
        }
        native_arguments.extend(args);
        return native_acceptance_runner::entry_with_arguments(&native_arguments);
    }
    let root = repository_root()?;
    let extra: Vec<_> = args.collect();
    if command == ci::mutations::INTERNAL_VALIDATOR_COMMAND {
        return ci::mutations::run_internal_validator(&root, &extra);
    }
    if command == "ci" {
        return ci::run_ci(&root, &extra);
    }
    if !extra.is_empty() {
        return Err(format!(
            "unexpected arguments for '{command}': {}",
            extra.join(" ")
        ));
    }
    run_preflight(&root, preflight_for(&command)?)
        .map_err(|error| format!("{command} preflight: {error}"))?;
    match command.as_str() {
        "__ci-test" => test_all(&root, false).map_err(|error| format!("test command: {error}")),
        "__ci-native-test" => {
            native_test(&root).map_err(|error| format!("native test command: {error}"))
        }
        "__ci-package" => package(&root),
        "__ci-capture-dependencies" => capture_dependencies(&root),
        "__ci-production" => production(&root).map(|_| ()),
        "__ci-verify" => verify_artifact_manifest(&root),
        "__ci-ownership-production" => verify_production_ownership(&root),
        "__ci-ownership-fixture" => verify_fixture_ownership(&root),
        "debug" => build_profile(&root, Profile::Debug),
        "release" => build_profile(&root, Profile::Release),
        "test" => test_all(&root, true).map_err(|error| format!("test command: {error}")),
        "native-test" => {
            native_test(&root).map_err(|error| format!("native test command: {error}"))
        }
        "probe" => run_script(&root, "rust/probes/run_p00_probes.sh"),
        "harness" => run_harnesses(&root),
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
    "usage: cargo run --manifest-path rust/xtask/Cargo.toml -- <debug|release|test|native-test|probe|harness|package|production|prove|verify|capture-dependencies|doctor|matrix|ci>".into()
}

fn preflight_for(command: &str) -> Result<Preflight, String> {
    match command {
        "production" | "prove" | "package" | "__ci-production" | "__ci-package" => {
            Ok(Preflight::StrictSource)
        }
        "debug"
        | "release"
        | "probe"
        | "harness"
        | "capture-dependencies"
        | "__ci-capture-dependencies"
        | "doctor" => Ok(Preflight::Full),
        "verify"
        | "test"
        | "native-test"
        | "__ci-verify"
        | "__ci-test"
        | "__ci-native-test"
        | "__ci-ownership-production"
        | "__ci-ownership-fixture" => Ok(Preflight::ContractOnly),
        "matrix" => Ok(Preflight::PureInspection),
        _ => Err(format!("unknown command '{command}'\n{}", usage())),
    }
}

fn run_preflight(root: &Path, preflight: Preflight) -> Result<(), String> {
    match preflight {
        Preflight::StrictSource => {
            validate_contract(root, true)?;
            reject_dirty_source(root)?;
            Ok(())
        }
        Preflight::Full => validate_contract(root, true).map(|_| ()),
        Preflight::ContractOnly => validate_contract(root, false).map(|_| ()),
        Preflight::PureInspection => Ok(()),
    }
}

fn reject_dirty_source(root: &Path) -> Result<(), String> {
    let output = run_bounded_command(
        git_command(root).args(["status", "--porcelain=v1", "--untracked-files=all", "-z"]),
        "source cleanliness check",
    )?;
    if !output.succeeded() {
        return Err(output.failure_detail("source cleanliness check"));
    }
    let dirty = output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty())
        .map(|entry| String::from_utf8_lossy(entry).into_owned())
        .collect::<Vec<_>>();
    if dirty.is_empty() {
        return Ok(());
    }
    Err(format!(
        "dirty tracked or untracked source blocks proof/production/package: {}",
        dirty.join(", ")
    ))
}

fn repository_root() -> Result<PathBuf, String> {
    if let Some(root) = env::var_os("UQM_CI_SOURCE_ROOT") {
        let root = PathBuf::from(root);
        if !root.is_absolute() {
            return Err("UQM_CI_SOURCE_ROOT must be an absolute path".into());
        }
        ci::bounded_io::validate_directory_nofollow(&root).map_err(|detail| {
            format!(
                "UQM_CI_SOURCE_ROOT must name an accessible component-wise no-follow directory {}: {detail}",
                root.display()
            )
        })?;
        return Ok(root);
    }

    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| "xtask is not inside the repository".into())
}

fn build_profile(root: &Path, profile: Profile) -> Result<(), String> {
    prepare_source_environment(root)?;
    let toolchain = canonical_toolchain(root)?;
    prepare_canonical_build(&toolchain)?;
    let args = build_profile_args(profile);
    let mut command = Command::new(&toolchain.cargo.executable);
    command.current_dir(root).args(&args);
    if let Some(target) = ci_cargo_target_dir()? {
        command.arg("--target-dir").arg(target);
    }
    run_command(&mut command, "Cargo build")
}

fn build_profile_args(profile: Profile) -> Vec<String> {
    let mut args = vec![
        "build".to_string(),
        "--locked".into(),
        "--manifest-path".into(),
        "rust/Cargo.toml".into(),
    ];
    if let Some(flag) = profile.flag() {
        args.push(flag.into());
    }
    args.extend([
        "--no-default-features".into(),
        "--features".into(),
        PRODUCTION_FEATURES.into(),
        "--bin".into(),
        "uqm".into(),
    ]);
    args
}

const PURE_TEST_FEATURES: &str = "audio_heart,debug-process";
const LINKED_TEST_FEATURES: &str = "audio_heart,debug-process,linked_c_archive";

fn ci_cargo_target_dir() -> Result<Option<PathBuf>, String> {
    env::var_os("UQM_CI_CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .map(|path| {
            if path.is_absolute() {
                Ok(path)
            } else {
                Err("UQM_CI_CARGO_TARGET_DIR must be absolute".to_string())
            }
        })
        .transpose()
}

fn cargo_args_with_target<const N: usize>(
    arguments: [&str; N],
    target: Option<&Path>,
) -> Vec<OsString> {
    let mut arguments: Vec<OsString> = arguments.into_iter().map(OsString::from).collect();
    if let Some(target) = target {
        arguments.push("--target-dir".into());
        arguments.push(target.as_os_str().to_owned());
    }
    arguments
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct RequiredTestInventory<'a> {
    schema: &'static str,
    route: &'a str,
    listed_tests: usize,
    listing_stdout_bytes: u64,
    listing_stdout_sha256: String,
}

fn parse_cargo_test_listing(bytes: &[u8]) -> Result<Vec<String>, String> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| format!("Cargo test listing is not UTF-8: {error}"))?;
    let mut tests = Vec::new();
    for line in text.lines() {
        if let Some(name) = line.strip_suffix(": test") {
            if name.is_empty() {
                return Err("Cargo test listing contains an empty test name".to_string());
            }
            tests.push(name.to_string());
        }
    }
    Ok(tests)
}

fn enforce_nonzero_test_inventory(route: &str, tests: &[String]) -> Result<(), String> {
    if tests.is_empty() {
        Err(format!("{route} listed zero executable tests"))
    } else {
        Ok(())
    }
}

fn require_test_inventory(command: &Command, route: &str) -> Result<(), String> {
    let captured = run_bounded_command(command, route)
        .map_err(|error| format!("{route} inventory: {error}"))?;
    if !captured.succeeded() {
        return Err(captured.failure_detail(route));
    }
    let tests = parse_cargo_test_listing(&captured.stdout)?;
    enforce_nonzero_test_inventory(route, &tests)?;
    let receipt = RequiredTestInventory {
        schema: "uqm-required-test-inventory-v1",
        route,
        listed_tests: tests.len(),
        listing_stdout_bytes: captured.stdout_bytes_seen,
        listing_stdout_sha256: format!("{:x}", Sha256::digest(&captured.stdout)),
    };
    println!(
        "{}",
        serde_json::to_string(&receipt)
            .map_err(|error| format!("serialize {route} test inventory: {error}"))?
    );
    Ok(())
}

fn test_all(root: &Path, run_native_acceptance: bool) -> Result<(), String> {
    let cargo_target = ci_cargo_target_dir()?;
    let test_cargo = retained_rust_tool_program("CARGO", "cargo")?;

    // Phase 1: Broad pure all-feature tests (no native linking required).
    // This exercises all Rust code paths including debug-process.
    let pure_arguments = cargo_args_with_target(
        [
            "test",
            "--locked",
            "--manifest-path",
            "rust/Cargo.toml",
            "--workspace",
            "--all-targets",
            "--no-default-features",
            "--features",
            PURE_TEST_FEATURES,
        ],
        cargo_target.as_deref(),
    );
    let mut pure_listing_arguments = pure_arguments.clone();
    pure_listing_arguments.extend([
        OsString::from("--"),
        OsString::from("--list"),
        OsString::from("--format"),
        OsString::from("terse"),
    ]);
    require_test_inventory(
        Command::new(&test_cargo)
            .current_dir(root)
            .args(pure_listing_arguments),
        "Cargo test workspace (pure feature set)",
    )?;
    run_command(
        Command::new(&test_cargo)
            .current_dir(root)
            .args(pure_arguments),
        "Cargo test workspace (pure feature set)",
    )?;

    // Phase 2: Strict production-linked fixtures through canonical orchestration.
    let toolchain = canonical_toolchain(root)
        .map_err(|error| format!("prepare canonical test toolchain: {error}"))?;

    // Both targets pass through the same S1 provider/archive/ownership gates.
    prepare_source_environment(root)?;
    prepare_canonical_build(&toolchain)?;
    let marker = serde_json::to_string(&toolchain)
        .map_err(|error| format!("cannot serialize toolchain: {error}"))?;
    let linked_build_proof = build_linked_test_proof(root, &toolchain, cargo_target.as_deref())?;
    let linked_arguments = cargo_args_with_target(
        [
            "test",
            "--locked",
            "--manifest-path",
            "rust/Cargo.toml",
            "--no-default-features",
            "--features",
            LINKED_TEST_FEATURES,
            "--test",
            "linked_provider_fixture",
        ],
        cargo_target.as_deref(),
    );
    let mut linked_listing_arguments = linked_arguments.clone();
    linked_listing_arguments.extend([
        OsString::from("--"),
        OsString::from("--list"),
        OsString::from("--format"),
        OsString::from("terse"),
    ]);
    let mut linked_listing = Command::new(&toolchain.cargo.executable);
    linked_listing
        .current_dir(root)
        .env("UQM_NATIVE_PROFILE", "linked-test")
        .env("UQM_CANONICAL_TOOLCHAIN", &marker)
        .env("RUSTC_LINKER", &toolchain.linker.executable)
        .args(linked_listing_arguments);
    require_test_inventory(&linked_listing, "Cargo test strict linked provider fixture")?;
    run_command(
        Command::new(&toolchain.cargo.executable)
            .current_dir(root)
            .env("UQM_NATIVE_PROFILE", "linked-test")
            .env("UQM_CANONICAL_TOOLCHAIN", marker)
            .env("RUSTC_LINKER", &toolchain.linker.executable)
            .args(linked_arguments),
        "Cargo test strict linked provider fixture",
    )?;
    if run_native_acceptance {
        run_native_window_acceptance(root, &linked_build_proof)?;
    }
    Ok(())
}

fn native_test(root: &Path) -> Result<(), String> {
    if !cfg!(target_os = "macos") {
        println!("native window acceptance is not required on this platform");
        return Ok(());
    }
    let cargo_target = ci_cargo_target_dir()?;
    let toolchain = canonical_toolchain(root)
        .map_err(|error| format!("prepare canonical native-test toolchain: {error}"))?;
    prepare_source_environment(root)?;
    prepare_canonical_build(&toolchain)?;
    let linked_build_proof = build_linked_test_proof(root, &toolchain, cargo_target.as_deref())?;
    run_native_window_acceptance(root, &linked_build_proof)
}

fn build_linked_test_proof(
    root: &Path,
    toolchain: &ToolchainIdentity,
    cargo_target: Option<&Path>,
) -> Result<LinkedBuildProof, String> {
    let authority = ci::authority::load_authority(root)?;
    ci::authority::validate_authority(&authority)?;
    let environment = linked_build_subprocess_environment(root, toolchain)?;
    let marker = serde_json::to_string(toolchain)
        .map_err(|error| format!("cannot serialize toolchain: {error}"))?;
    let mut arguments = [
        "build",
        "--locked",
        "--manifest-path",
        "rust/Cargo.toml",
        "--release",
        "--no-default-features",
        "--features",
        LINKED_TEST_FEATURES,
        "--bin",
        "uqm",
        "--message-format=json-render-diagnostics",
    ]
    .map(str::to_string)
    .to_vec();
    if let Some(target) = cargo_target {
        arguments.push("--target-dir".to_string());
        arguments.push(
            target
                .to_str()
                .ok_or_else(|| "Cargo target directory is not UTF-8".to_string())?
                .to_string(),
        );
    }
    let output = ci::exec::run_captured_with_bound_environment(
        root,
        &toolchain.cargo.executable,
        &arguments,
        authority.supervision.builtin_limits(),
        true,
        |_| Ok(environment),
    );
    eprint!("{}", String::from_utf8_lossy(&output.stderr));
    if !output.completed_under_supervision()
        || output.exit_code != Some(0)
        || output.signal.is_some()
    {
        return Err(output.failure_detail("linked-test Cargo build"));
    }
    let paths = parse_production_paths(root, &output.stdout)?;
    let directory = tempfile::tempdir()
        .map_err(|error| format!("create linked-build proof directory: {error}"))?;
    ci::exec::permit_containment_directory(directory.path())
        .map_err(|error| format!("permit linked-build proof directory: {error}"))?;
    let member_limit = authority.actions.evidence_snapshot_member_limit_bytes;
    let messages = stage_linked_build_bytes(
        directory.path(),
        "cargo-messages.jsonl",
        "inputs/linked-build/cargo-messages.jsonl",
        &output.stdout,
        member_limit,
    )?;
    let rust_archive = stage_linked_build_file(
        directory.path(),
        "rust-archive.a",
        "inputs/linked-build/rust-archive.a",
        &paths.rust_archive,
        member_limit,
    )?;
    let c_archive = stage_linked_build_file(
        directory.path(),
        "c-archive.a",
        "inputs/linked-build/c-archive.a",
        &paths.c_archive,
        member_limit,
    )?;
    let object_sidecar = stage_linked_build_file(
        directory.path(),
        "object-sidecar.manifest",
        "inputs/linked-build/object-sidecar.manifest",
        &paths.object_sidecar,
        member_limit,
    )?;
    let provider_report = stage_linked_build_file(
        directory.path(),
        "provider-report.json",
        "inputs/linked-build/provider-report.json",
        &paths.provider_report,
        member_limit,
    )?;
    let native_build_evidence = stage_linked_build_file(
        directory.path(),
        "native-build-evidence.json",
        "inputs/linked-build/native-build-evidence.json",
        &paths.build_evidence,
        member_limit,
    )?;
    let executable_bytes = read_regular_file_nofollow_bounded(&paths.executable, member_limit)?;
    let cargo_manifest = stage_linked_build_file(
        directory.path(),
        "Cargo.toml",
        "inputs/linked-build/Cargo.toml",
        &root.join("rust/Cargo.toml"),
        member_limit,
    )?;
    let cargo_lock = stage_linked_build_file(
        directory.path(),
        "Cargo.lock",
        "inputs/linked-build/Cargo.lock",
        &root.join("rust/Cargo.lock"),
        member_limit,
    )?;
    let retained_authority = stage_linked_build_file(
        directory.path(),
        "gates.json",
        "inputs/linked-build/gates.json",
        &root.join(ci::authority::AUTHORITY_RELATIVE),
        member_limit,
    )?;
    let canonical_toolchain = stage_linked_build_bytes(
        directory.path(),
        "canonical-toolchain.json",
        "inputs/linked-build/canonical-toolchain.json",
        marker.as_bytes(),
        member_limit,
    )?;
    let source_sha = git_text(root, &["rev-parse", "HEAD"], "HEAD")?;
    let receipt = NativeLinkedBuildReceipt {
        schema: NATIVE_LINKED_BUILD_RECEIPT_SCHEMA.to_string(),
        source_sha,
        cargo_command: std::iter::once(toolchain.cargo.executable.clone())
            .chain(arguments)
            .collect(),
        native_profile: "linked-test".to_string(),
        feature: LINKED_TEST_FEATURES.to_string(),
        cargo_executable_path: paths
            .executable
            .to_str()
            .ok_or_else(|| "Cargo executable path is not UTF-8".to_string())?
            .to_string(),
        cargo_rust_archive_path: paths
            .rust_archive
            .to_str()
            .ok_or_else(|| "Cargo Rust archive path is not UTF-8".to_string())?
            .to_string(),
        cargo_out_dir: paths
            .build_evidence
            .parent()
            .and_then(Path::to_str)
            .ok_or_else(|| "Cargo OUT_DIR is not UTF-8".to_string())?
            .to_string(),
        executable: retained_identity("inputs/uqm", &executable_bytes),
        cargo_messages: messages,
        rust_archive,
        c_archive,
        object_sidecar,
        provider_report,
        native_build_evidence,
        cargo_manifest,
        cargo_lock,
        authority: retained_authority,
        canonical_toolchain,
    };
    let receipt_bytes = serde_json::to_vec_pretty(&receipt)
        .map_err(|error| format!("serialize linked-build receipt: {error}"))?;
    stage_linked_build_bytes(
        directory.path(),
        "linked-build-receipt.json",
        "inputs/linked-build/linked-build-receipt.json",
        &receipt_bytes,
        member_limit,
    )?;
    Ok(LinkedBuildProof { directory })
}

fn retained_identity(relative_path: &str, bytes: &[u8]) -> NativeRetainedInput {
    NativeRetainedInput {
        relative_path: relative_path.to_string(),
        byte_length: bytes.len() as u64,
        sha256: hex_sha256(bytes),
    }
}

fn stage_linked_build_file(
    directory: &Path,
    filename: &str,
    retained_path: &str,
    source: &Path,
    limit: u64,
) -> Result<NativeRetainedInput, String> {
    let bytes = read_regular_file_nofollow_bounded(source, limit)?;
    stage_linked_build_bytes(directory, filename, retained_path, &bytes, limit)
}

fn stage_linked_build_bytes(
    directory: &Path,
    filename: &str,
    retained_path: &str,
    bytes: &[u8],
    limit: u64,
) -> Result<NativeRetainedInput, String> {
    if bytes.len() as u64 > limit {
        return Err(format!(
            "linked-build proof member {filename} exceeds authority limit"
        ));
    }
    let path = directory.join(filename);
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|error| format!("create linked-build proof member {filename}: {error}"))?;
    file.write_all(bytes)
        .map_err(|error| format!("write linked-build proof member {filename}: {error}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        file.set_permissions(fs::Permissions::from_mode(0o640))
            .map_err(|error| format!("publish linked-build proof member {filename}: {error}"))?;
    }
    Ok(retained_identity(retained_path, bytes))
}

fn run_native_window_acceptance(
    root: &Path,
    linked_build_proof: &LinkedBuildProof,
) -> Result<(), String> {
    let Some(evidence_root) = env::var_os("UQM_CI_NATIVE_ACCEPTANCE_EVIDENCE_ROOT") else {
        return Ok(());
    };
    let content_root = env::var_os("UQM_CI_NATIVE_CONTENT_ROOT").ok_or_else(|| {
        "UQM_CI_NATIVE_CONTENT_ROOT is required for native acceptance".to_string()
    })?;
    let authority = ci::authority::load_authority(root)?;
    ci::authority::validate_authority(&authority)?;
    let script = root.join(&authority.native_acceptance.script);
    let script_bytes = read_regular_file_nofollow_bounded(
        &script,
        authority.native_acceptance.script_byte_length,
    )?;
    if script_bytes.len() as u64 != authority.native_acceptance.script_byte_length
        || hex_sha256(&script_bytes) != authority.native_acceptance.script_sha256
    {
        return Err("native acceptance script differs from machine authority".to_string());
    }
    let script_value: serde_json::Value = serde_json::from_slice(&script_bytes)
        .map_err(|error| format!("parse native acceptance script budget: {error}"))?;
    let script_wallclock_ms = script_value
        .pointer("/budgets/max_wallclock_seconds")
        .and_then(serde_json::Value::as_u64)
        .and_then(|seconds| seconds.checked_mul(1_000))
        .ok_or_else(|| "native acceptance script has no valid wallclock budget".to_string())?;
    let required_outer_ms = script_wallclock_ms
        .checked_add(
            authority
                .native_acceptance
                .runtime_contract
                .observer_timeout_ms,
        )
        .and_then(|value| {
            value.checked_add(
                authority
                    .native_acceptance
                    .runtime_contract
                    .outer_child_kill_grace_ms,
            )
        })
        .ok_or_else(|| "native acceptance outer deadline calculation overflowed".to_string())?;
    if authority
        .native_acceptance
        .runtime_contract
        .outer_child_timeout_ms
        <= required_outer_ms
    {
        return Err(
            "native acceptance outer timeout does not cover script, startup, and cleanup"
                .to_string(),
        );
    }
    let target = ci_cargo_target_dir()?.unwrap_or_else(|| root.join("rust/target"));
    let (controller, linked_executable) = native_acceptance_executables(&target)?;
    let linked_bytes = read_regular_file_nofollow_bounded(
        &linked_executable,
        authority.supervision.executable_member_limit_bytes,
    )?;
    let linked_length = linked_bytes.len().to_string();
    let linked_sha256 = hex_sha256(&linked_bytes);
    let runtime_contract = serde_json::to_string(&authority.native_runtime_contract())
        .map_err(|error| format!("serialize native acceptance runtime contract: {error}"))?;
    let acceptance_policy =
        serde_json::to_string(&authority.native_acceptance.acceptance_policy)
            .map_err(|error| format!("serialize native acceptance policy: {error}"))?;
    let mut command = Command::new(controller);
    command.current_dir(root).args([
        "__ci-native-acceptance",
        "run",
        &linked_executable.display().to_string(),
        &PathBuf::from(content_root).display().to_string(),
        &script.display().to_string(),
        &PathBuf::from(evidence_root).display().to_string(),
        &linked_length,
        &linked_sha256,
        &runtime_contract,
        &acceptance_policy,
        &linked_build_proof.directory.path().display().to_string(),
        &authority
            .actions
            .evidence_snapshot_member_limit_bytes
            .to_string(),
    ]);
    // The Aqua child inherits only an allowlisted environment, so the trusted
    // controller's precreated-root binding must be forwarded explicitly.
    if let Ok(precreated) = env::var(NATIVE_ACCEPTANCE_PRECREATED_ROOT_ENV) {
        command.env(NATIVE_ACCEPTANCE_PRECREATED_ROOT_ENV, precreated);
    }
    run_aqua_command(&mut command, "Direct linked native-window acceptance")
}

fn native_acceptance_executables(target: &Path) -> Result<(PathBuf, PathBuf), String> {
    let controller = env::current_exe()
        .map_err(|error| format!("resolve base-owned native-acceptance controller: {error}"))?;
    Ok((controller, target.join("release/uqm")))
}

fn prepare_source_environment(root: &Path) -> Result<(), String> {
    env::set_var("SOURCE_DATE_EPOCH", source_date_epoch(root)?.to_string());
    env::set_var("UQM_BUILD_DATE", source_date(root)?);
    Ok(())
}

fn prepare_canonical_build(toolchain: &ToolchainIdentity) -> Result<(), String> {
    reject_noncanonical_build_flags(toolchain)?;
    apply_canonical_toolchain_environment(toolchain);
    env::set_var(
        "UQM_CANONICAL_TOOLCHAIN",
        serde_json::to_string(toolchain)
            .map_err(|error| format!("cannot serialize canonical toolchain: {error}"))?,
    );
    Ok(())
}

fn production(root: &Path) -> Result<ArtifactManifest, String> {
    prepare_source_environment(root)?;
    let toolchain = canonical_toolchain(root)?;
    prepare_canonical_build(&toolchain)?;
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
    let first_identity = build_identity(&first);
    let second_identity = build_identity(&second);
    if first_identity != second_identity {
        return Err(
            "source/toolchain identity differs across two clean builds before artifact comparison"
                .into(),
        );
    }
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
        first_identity,
        second_identity,
    });
    write_artifact_manifest(root, &second)
}

fn clean(root: &Path) -> Result<(), String> {
    let release_dir = root.join("rust/target/release");
    let manifest_path = manifest_path_str(root)?;
    let toolchain = canonical_toolchain(root)?;
    clean_release_dir(&release_dir)?;
    run_command(
        Command::new(&toolchain.cargo.executable)
            .current_dir(root)
            .args(["clean", "--manifest-path", &manifest_path, "--release"]),
        "Cargo clean release",
    )
}

fn manifest_path_str(root: &Path) -> Result<String, String> {
    root.join("rust/Cargo.toml")
        .to_str()
        .map(str::to_string)
        .ok_or_else(|| "rust/Cargo.toml manifest path is not UTF-8".to_string())
}

fn clean_release_dir(release_dir: &Path) -> Result<(), String> {
    if release_dir.is_dir() {
        fs::remove_dir_all(release_dir).map_err(|error| {
            format!(
                "cannot remove release directory {}: {error}",
                release_dir.display()
            )
        })?;
    }
    Ok(())
}

fn canonical_toolchain_subprocess_environment(
    toolchain: &ToolchainIdentity,
) -> Result<Vec<(String, String)>, String> {
    let marker = serde_json::to_string(toolchain)
        .map_err(|error| format!("cannot serialize canonical toolchain: {error}"))?;
    let target = toolchain.target.replace('-', "_").to_ascii_uppercase();
    Ok(vec![
        ("UQM_CANONICAL_TOOLCHAIN".into(), marker),
        ("CC".into(), toolchain.cc.executable.clone()),
        ("AR".into(), toolchain.ar.executable.clone()),
        ("NM".into(), toolchain.nm.executable.clone()),
        ("PKG_CONFIG".into(), toolchain.pkg_config.executable.clone()),
        ("RUSTC".into(), toolchain.rustc.executable.clone()),
        ("CARGO".into(), toolchain.cargo.executable.clone()),
        ("RUSTC_LINKER".into(), toolchain.linker.executable.clone()),
        (
            format!("CARGO_TARGET_{target}_LINKER"),
            toolchain.linker.executable.clone(),
        ),
    ])
}

fn linked_build_subprocess_environment(
    root: &Path,
    toolchain: &ToolchainIdentity,
) -> Result<Vec<(String, String)>, String> {
    let mut environment = source_subprocess_environment(root)?;
    environment.extend(canonical_toolchain_subprocess_environment(toolchain)?);
    environment.push(("UQM_NATIVE_PROFILE".into(), "linked-test".into()));
    Ok(environment)
}

fn production_subprocess_environment(
    root: &Path,
    toolchain: &ToolchainIdentity,
) -> Result<Vec<(String, String)>, String> {
    let mut environment = source_subprocess_environment(root)?;
    environment.push(("CARGO_BUILD_JOBS".into(), "1".into()));
    environment.extend(canonical_toolchain_subprocess_environment(toolchain)?);
    environment.push(("UQM_NATIVE_PROFILE".into(), "production".into()));
    Ok(environment)
}

fn source_subprocess_environment(root: &Path) -> Result<Vec<(String, String)>, String> {
    Ok(vec![
        (
            "SOURCE_DATE_EPOCH".into(),
            source_date_epoch(root)?.to_string(),
        ),
        ("UQM_BUILD_DATE".into(), source_date(root)?),
    ])
}

fn cargo_production(root: &Path, toolchain: &ToolchainIdentity) -> Result<ProductionPaths, String> {
    let authority = ci::authority::load_authority(root)?;
    ci::authority::validate_authority(&authority)?;
    let environment = production_subprocess_environment(root, toolchain)?;
    let arguments = [
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
    ]
    .map(str::to_string);
    let output = ci::exec::run_captured_with_bound_environment(
        root,
        &toolchain.cargo.executable,
        &arguments,
        authority.supervision.builtin_limits(),
        true,
        |_| Ok(environment),
    );
    eprint!("{}", String::from_utf8_lossy(&output.stderr));
    if !output.completed_under_supervision()
        || output.exit_code != Some(0)
        || output.signal.is_some()
    {
        return Err(output.failure_detail("production Cargo build"));
    }
    parse_production_paths(root, &output.stdout)
}

fn parse_production_paths(root: &Path, messages: &[u8]) -> Result<ProductionPaths, String> {
    let text = std::str::from_utf8(messages)
        .map_err(|error| format!("Cargo JSON messages are not UTF-8: {error}"))?;
    let messages: Vec<serde_json::Value> = text
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            serde_json::from_str(line)
                .map_err(|error| format!("invalid Cargo JSON message: {error}"))
        })
        .collect::<Result<_, _>>()?;
    let expected_main = fs::canonicalize(root.join("rust/src/main.rs"))
        .map_err(|error| format!("resolve UQM main source: {error}"))?;
    let expected_lib = fs::canonicalize(root.join("rust/src/lib.rs"))
        .map_err(|error| format!("resolve UQM library source: {error}"))?;
    let package_ids: BTreeSet<_> = messages
        .iter()
        .filter(|message| {
            message["reason"] == "compiler-artifact"
                && message["target"]["name"] == "uqm"
                && cargo_message_has_target_kind(message, "bin")
                && message["executable"].is_string()
                && message_source_is(message, &expected_main)
        })
        .filter_map(|message| message["package_id"].as_str().map(str::to_string))
        .collect();
    let package_id = exactly_one(package_ids, "UQM Cargo package identity")?;
    let mut out_dirs = BTreeSet::new();
    let mut executables = BTreeSet::new();
    let mut rust_archives = BTreeSet::new();
    for message in &messages {
        if message["package_id"].as_str() != Some(package_id.as_str()) {
            continue;
        }
        match message["reason"].as_str() {
            Some("build-script-executed") => {
                if let Some(path) = message["out_dir"].as_str() {
                    out_dirs.insert(PathBuf::from(path));
                }
            }
            Some("compiler-artifact")
                if message["target"]["name"] == "uqm"
                    && cargo_message_has_target_kind(message, "bin")
                    && message_source_is(message, &expected_main) =>
            {
                if let Some(path) = message["executable"].as_str() {
                    executables.insert(PathBuf::from(path));
                }
            }
            Some("compiler-artifact")
                if message["target"]["name"] == "uqm_rust"
                    && cargo_message_has_target_kind(message, "staticlib")
                    && message_source_is(message, &expected_lib) =>
            {
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

fn message_source_is(message: &serde_json::Value, expected: &Path) -> bool {
    let Some(expected) = fs::canonicalize(expected).ok() else {
        return false;
    };
    message["target"]["src_path"]
        .as_str()
        .and_then(|path| fs::canonicalize(path).ok())
        .is_some_and(|path| path == expected)
}

fn cargo_message_has_target_kind(message: &serde_json::Value, expected: &str) -> bool {
    message["target"]["kind"]
        .as_array()
        .is_some_and(|kinds| kinds.iter().any(|kind| kind == expected))
}

fn exactly_one<T: std::fmt::Debug + Ord>(paths: BTreeSet<T>, label: &str) -> Result<T, String> {
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
    prove_determinism(root)?;
    verify_artifact_manifest(root)?;
    let authority = ci::load_authority(root)?;
    let manifest: ArtifactManifest = read_json(
        &root.join("rust/target/production-artifacts.json"),
        authority.actions.evidence_snapshot_member_limit_bytes,
    )?;
    let executable = manifest
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
    if let Err(error) = populate_package(root, executable, &staging, &target, &authority) {
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

fn populate_package(
    root: &Path,
    executable: &Artifact,
    staging: &Path,
    target: &str,
    authority: &ci::authority::Authority,
) -> Result<(), String> {
    let executable_limit = authority.supervision.executable_member_limit_bytes;
    let evidence_limit = authority.actions.evidence_snapshot_member_limit_bytes;
    let source = root.join(&executable.path);
    let destination = staging.join("uqm");
    ci::bounded_io::copy_executable_nofollow(&source, &destination, executable_limit)
        .map_err(|error| format!("cannot package executable {}: {error}", source.display()))?;
    let packaged = ci::bounded_io::read_regular_nofollow(&destination, executable_limit)?;
    if hex_sha256(&packaged) != executable.sha256 {
        return Err("packaged executable digest differs from production invocation".into());
    }

    let manifest_source = root.join("rust/target/production-artifacts.json");
    let source_bytes = ci::bounded_io::read_regular_nofollow(&manifest_source, evidence_limit)?;
    let mut staged: ArtifactManifest = serde_json::from_slice(&source_bytes)
        .map_err(|error| format!("production artifact manifest is invalid: {error}"))?;
    if staged.determinism_proof.is_none() {
        return Err("staged manifest lacks mandatory determinism proof".into());
    }
    rewrite_packaged_executable(&mut staged.artifacts, executable, target)?;
    let mut staged_bytes = serde_json::to_vec_pretty(&staged)
        .map_err(|error| format!("cannot serialize packaged artifact manifest: {error}"))?;
    staged_bytes.push(b'\n');
    let manifest_staged = staging.join("production-artifacts.json");
    ci::bounded_io::write_regular_nofollow(&manifest_staged, &staged_bytes, evidence_limit)
        .map_err(|error| format!("cannot package artifact manifest: {error}"))?;
    let retained = ci::bounded_io::read_regular_nofollow(&manifest_staged, evidence_limit)?;
    if retained != staged_bytes {
        return Err("retained package manifest differs from its generated bytes".into());
    }
    Ok(())
}

fn rewrite_packaged_executable(
    artifacts: &mut [Artifact],
    executable: &Artifact,
    target: &str,
) -> Result<(), String> {
    let staged_executable = artifacts
        .iter_mut()
        .find(|artifact| artifact.role == "executable")
        .ok_or_else(|| "staged manifest lacks its executable artifact".to_string())?;
    if staged_executable.path != executable.path || staged_executable.sha256 != executable.sha256 {
        return Err("staged manifest executable contradicts the production invocation".into());
    }
    staged_executable.path = format!("rust/target/uqm-package/{target}/uqm");
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
    let authority = ci::load_authority(root)?;
    let limit = authority.actions.evidence_snapshot_member_limit_bytes;
    let manifest = serde_json::from_slice(&ci::bounded_io::read_regular_nofollow(
        &root.join(NATIVE_INPUTS),
        limit,
    )?)
    .map_err(|error| format!("invalid {NATIVE_INPUTS}: {error}"))?;
    let dependencies = serde_json::from_slice(&ci::bounded_io::read_regular_nofollow(
        &root.join(NATIVE_DEPENDENCIES),
        limit,
    )?)
    .map_err(|error| format!("invalid {NATIVE_DEPENDENCIES}: {error}"))?;
    let providers = uqm_ownership::Manifest::from_json(&ci::bounded_io::read_regular_nofollow(
        &root.join(PROVIDER_MANIFEST),
        limit,
    )?)
    .map_err(|error| error.to_string())?;
    uqm_ownership::validate_native_authority(root, &manifest, &dependencies, &providers)?;
    let trend: Trend = read_json(&root.join(TREND), limit)?;
    if trend.schema != "uqm-native-input-trend-v1" {
        return Err("unsupported native input trend schema".into());
    }
    validate_trend_authority(root, &trend)?;
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

fn validate_trend_authority(root: &Path, trend: &Trend) -> Result<(), String> {
    let authority = ci::load_authority(root)?;
    let ledger = &trend.ownership_ledger;
    let expected = &authority.ledger_identity;
    if ledger.schema != expected.schema
        || ledger.assessment_commit != expected.assessment_commit
        || ledger.raw_revision != expected.raw_revision
        || ledger.raw_url != expected.url
        || ledger.gist_revision != expected.history_revision
        || ledger.sha256 != expected.sha256
    {
        return Err("native input trend does not match machine-authority ledger identity".into());
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
        return Err("native input trend hash-table cutovers differ from ledger v7".into());
    }
    Ok(())
}

fn select_host_target(root: &Path) -> Result<SupportedTarget, String> {
    let authority = ci::load_authority(root)?;
    let matrix: Matrix = read_json(
        &root.join(MATRIX),
        authority.actions.evidence_snapshot_member_limit_bytes,
    )?;
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
        .filter(|name| {
            !matches!(name.as_str(), "cc" | "ar" | "nm" | "pkg-config")
                && !(target.os == "linux" && name.as_str() == "bzip2")
        })
        .map(String::as_str)
        .collect();
    run_command(
        Command::new(&toolchain.pkg_config.executable)
            .arg("--exists")
            .args(&packages),
        &format!("pkg-config prerequisites [{}]", packages.join(", ")),
    )?;
    if target.os == "linux" {
        validate_linux_bzip2(&toolchain.cc.executable)?;
    }
    Ok(())
}

fn validate_linux_bzip2(cc: &str) -> Result<(), String> {
    let directory = tempfile::tempdir()
        .map_err(|error| format!("cannot create bzip2 prerequisite workspace: {error}"))?;
    let source = directory.path().join("bzip2-probe.c");
    fs::write(
        &source,
        b"#include <bzlib.h>\nint main(void) { bz_stream stream = {0}; return BZ2_bzCompressInit(&stream, 1, 0, 0); }\n",
    )
    .map_err(|error| format!("cannot write bzip2 prerequisite probe: {error}"))?;
    let source = source
        .to_str()
        .ok_or_else(|| "bzip2 prerequisite path is not UTF-8".to_string())?;
    let captured = run_bounded_command(
        Command::new(cc).args(["-x", "c", source, "-lbz2", "-o", "/dev/null"]),
        "bzip2 prerequisite probe",
    )?;
    if captured.succeeded() {
        Ok(())
    } else {
        Err(format!(
            "{}; install libbz2-dev",
            captured.failure_detail("bzip2 header/library prerequisite")
        ))
    }
}

fn print_matrix(root: &Path) -> Result<(), String> {
    let authority = ci::load_authority(root)?;
    let bytes = ci::bounded_io::read_regular_nofollow(
        &root.join(MATRIX),
        authority.actions.evidence_snapshot_member_limit_bytes,
    )?;
    let matrix =
        std::str::from_utf8(&bytes).map_err(|error| format!("{MATRIX} is not UTF-8: {error}"))?;
    print!("{matrix}");
    Ok(())
}

fn capture_dependencies(root: &Path) -> Result<(), String> {
    let epoch = source_date_epoch(root)?;
    env::set_var("SOURCE_DATE_EPOCH", epoch.to_string());
    env::set_var("UQM_BUILD_DATE", source_date(root)?);
    let toolchain = canonical_toolchain(root)?;
    // The gate hands this step the canonical toolchain environment, which sets
    // CARGO_TARGET_<TRIPLE>_LINKER. Demanding no build flags at all would refuse
    // the values the harness just supplied, so require the canonical ones and
    // reject everything else.
    reject_noncanonical_build_flags(&toolchain)?;
    apply_canonical_toolchain_environment(&toolchain);
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
    let artifact_limit = ci::load_authority(root)?
        .supervision
        .executable_member_limit_bytes;
    let artifacts = vec![
        artifact(
            root,
            "executable",
            &paths.executable,
            "application/x-executable",
            CARGO_BUILD_COMMAND,
            artifact_limit,
        )?,
        artifact(
            root,
            "rust_static_archive",
            &paths.rust_archive,
            "application/x-archive",
            CARGO_BUILD_COMMAND,
            artifact_limit,
        )?,
        artifact(
            root,
            "c_static_archive",
            &paths.c_archive,
            "application/x-archive",
            C_ARCHIVE_COMMAND,
            artifact_limit,
        )?,
        artifact(
            root,
            "object_sidecar",
            &paths.object_sidecar,
            "text/plain",
            SIDECAR_COMMAND,
            artifact_limit,
        )?,
        artifact(
            root,
            "provider_report",
            &paths.provider_report,
            "application/json",
            PROVIDER_REPORT_COMMAND,
            artifact_limit,
        )?,
    ];
    let native_build = read_build_evidence(&paths.build_evidence)?;
    let cargo_feature_graph = resolve_cargo_feature_graph(root, &native_build.toolchain)?;
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
        cargo_feature_graph,
        artifacts,
        determinism_proof,
    })
}

fn resolve_cargo_feature_graph(
    root: &Path,
    toolchain: &ToolchainIdentity,
) -> Result<Vec<CargoFeatureEntry>, String> {
    let output = run_bounded_command(
        Command::new(&toolchain.cargo.executable)
            .current_dir(root.join("rust"))
            .args([
                "tree",
                "--locked",
                "--manifest-path",
                "Cargo.toml",
                "--no-default-features",
                "--features",
                PRODUCTION_FEATURES,
                "--edges",
                "normal,build",
                "--prefix",
                "none",
                "--no-dedupe",
                "--format",
                "{p}|{f}",
            ]),
        "cargo tree feature graph",
    )?;
    if !output.succeeded() {
        return Err(output.failure_detail("cargo tree feature graph"));
    }
    let text = String::from_utf8(output.stdout)
        .map_err(|error| format!("cargo tree output is not UTF-8: {error}"))?;
    let mut entries = BTreeMap::<(String, String), BTreeSet<String>>::new();
    for line in text.lines().filter(|line| !line.is_empty()) {
        let entry = parse_cargo_tree_line(line)?;
        entries
            .entry((entry.name, entry.version))
            .or_default()
            .extend(entry.features);
    }
    Ok(entries
        .into_iter()
        .map(|((name, version), features)| CargoFeatureEntry {
            name,
            version,
            features: features.into_iter().collect(),
        })
        .collect())
}

fn parse_cargo_tree_line(line: &str) -> Result<CargoFeatureEntry, String> {
    let (package, features) = line
        .split_once('|')
        .ok_or_else(|| format!("cargo tree emitted malformed feature row: {line}"))?;
    let mut words = package.split_whitespace();
    let name = words
        .next()
        .ok_or_else(|| format!("cargo tree feature row lacks package name: {line}"))?;
    let version = words
        .next()
        .and_then(|word| word.strip_prefix('v'))
        .ok_or_else(|| format!("cargo tree feature row lacks package version: {line}"))?;
    let features = features
        .split(',')
        .filter(|feature| !feature.is_empty())
        .map(str::to_string)
        .collect();
    Ok(CargoFeatureEntry {
        name: name.to_string(),
        version: version.to_string(),
        features,
    })
}

fn artifact(
    root: &Path,
    role: &str,
    path: &Path,
    media_type: &str,
    producing_command: &str,
    limit: u64,
) -> Result<Artifact, String> {
    let bytes = ci::bounded_io::read_regular_nofollow(path, limit)
        .map_err(|error| format!("production artifact {} is invalid: {error}", path.display()))?;
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
fn resolve_production_ownership_artifacts(
    root: &Path,
    artifacts: &[Artifact],
) -> Result<ProductionArtifacts, String> {
    let artifact_path = |role: &str| {
        let matches = artifacts
            .iter()
            .filter(|artifact| artifact.role == role)
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            return Err(format!("production role must occur exactly once: {role}"));
        }
        Ok(root.join(&matches[0].path))
    };
    Ok(ProductionArtifacts {
        rust_archive: artifact_path("rust_static_archive")?,
        c_archive: artifact_path("c_static_archive")?,
        executable: artifact_path("executable")?,
    })
}

fn verify_production_ownership(root: &Path) -> Result<(), String> {
    verify_artifact_manifest(root)?;
    let authority = ci::load_authority(root)?;
    let manifest_path = root.join("rust/target/production-artifacts.json");
    let manifest: ArtifactManifest = read_json(
        &manifest_path,
        authority.actions.evidence_snapshot_member_limit_bytes,
    )?;
    let artifacts = resolve_production_ownership_artifacts(root, &manifest.artifacts)?;
    let validator =
        Validator::from_manifest_file(&root.join("rust/ownership/native-provider-manifest.json"))
            .map_err(|error| error.to_string())?;
    let report = validator
        .validate_production_artifacts(
            &artifacts,
            &ProductionToolPaths {
                ar: manifest.native_build.toolchain.ar.executable.clone().into(),
                nm: manifest.native_build.toolchain.nm.executable.clone().into(),
            },
        )
        .map_err(|error| error.to_string())?;
    let report_path = root.join("rust/target/ownership-production-report.json");
    fs::write(
        &report_path,
        [
            report
                .to_json()
                .map_err(|error| error.to_string())?
                .as_bytes(),
            b"\n",
        ]
        .concat(),
    )
    .map_err(|error| format!("cannot write {}: {error}", report_path.display()))?;
    println!(
        "strict production ownership verified: report={}",
        report_path.display()
    );
    Ok(())
}

fn verify_fixture_ownership(root: &Path) -> Result<(), String> {
    let authority = ci::load_authority(root)?;
    let executable_limit = authority.supervision.executable_member_limit_bytes;
    let ar = ci::doctor::resolve_executable("ar", executable_limit)?;
    let nm = ci::doctor::resolve_executable("nm", executable_limit)?;
    let output = root.join("rust/target/ownership-strict-link-fixture");
    let validator =
        Validator::from_manifest_file(&root.join("rust/ownership/native-provider-manifest.json"))
            .map_err(|error| error.to_string())?;
    let report = validator
        .validate_symbol_artifacts(
            &ProductionArtifacts {
                rust_archive: output.join("libfixture_rust.a"),
                c_archive: output.join("libfixture_c.a"),
                executable: output.join("uqm-fixture"),
            },
            &ProductionToolPaths {
                ar: ar.execution_path().into(),
                nm: nm.execution_path().into(),
            },
        )
        .map_err(|error| error.to_string())?;
    let report_path = output.join("ownership-fixture-report.json");
    fs::write(
        &report_path,
        [
            report
                .to_json()
                .map_err(|error| error.to_string())?
                .as_bytes(),
            b"\n",
        ]
        .concat(),
    )
    .map_err(|error| format!("cannot write {}: {error}", report_path.display()))?;
    println!("focused strict-link fixture verified: {}", output.display());
    Ok(())
}

fn verify_artifact_manifest(root: &Path) -> Result<(), String> {
    let authority = ci::load_authority(root)?;
    verify_artifact_manifest_with_authority(root, &authority)
}

fn verify_artifact_manifest_with_authority(
    root: &Path,
    authority: &ci::authority::Authority,
) -> Result<(), String> {
    let evidence_limit = authority.actions.evidence_snapshot_member_limit_bytes;
    let artifact_limit = authority.supervision.executable_member_limit_bytes;
    let path = root.join("rust/target/production-artifacts.json");
    let manifest: ArtifactManifest = read_json(&path, evidence_limit)?;
    if manifest.schema != ARTIFACT_SCHEMA
        || manifest.command != PROVE_COMMAND
        || manifest.profile != "release"
        || manifest.features != ["audio_heart", "linked_c_archive"]
    {
        return Err("artifact proof schema or fixed production constants differ".into());
    }
    let toolchain = canonical_toolchain(root)?;
    apply_canonical_toolchain_environment(&toolchain);
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
    let live_feature_graph = resolve_cargo_feature_graph(root, &toolchain)?;
    if manifest.cargo_feature_graph != live_feature_graph {
        return Err("locked Cargo feature graph differs from live Cargo.lock".into());
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
        let bytes = read_manifest_artifact(root, item, artifact_limit)?;
        let artifact_path = root.join(&item.path);
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
fn read_manifest_artifact(root: &Path, item: &Artifact, limit: u64) -> Result<Vec<u8>, String> {
    uqm_ownership::validate_repo_relative_path(&item.path)?;
    let relative = Path::new(&item.path);
    if !relative.starts_with(Path::new("rust/target")) {
        return Err(format!(
            "production artifact is outside rust/target: {}",
            item.path
        ));
    }
    let artifact_path = root.join(relative);
    ci::bounded_io::read_regular_nofollow(&artifact_path, limit).map_err(|error| {
        format!(
            "cannot verify artifact {}: {error}",
            artifact_path.display()
        )
    })
}

/// Describe which tool in a recorded toolchain differs from the live one.
///
/// Whole-struct equality reports only that the toolchain moved, which is the
/// least useful thing to know when a build and its verification disagree.
fn describe_toolchain_difference(recorded: &ToolchainIdentity, live: &ToolchainIdentity) -> String {
    if recorded.target != live.target {
        return format!(
            "target recorded {} but resolved {}",
            recorded.target, live.target
        );
    }
    let tools = [
        ("rustc", &recorded.rustc, &live.rustc),
        ("cargo", &recorded.cargo, &live.cargo),
        ("cc", &recorded.cc, &live.cc),
        ("ar", &recorded.ar, &live.ar),
        ("nm", &recorded.nm, &live.nm),
        ("pkg_config", &recorded.pkg_config, &live.pkg_config),
        ("linker", &recorded.linker, &live.linker),
    ];
    for (name, was, now) in tools {
        if was.executable != now.executable {
            return format!(
                "{name} recorded {} but resolved {}",
                was.executable, now.executable
            );
        }
        if was.version != now.version {
            return format!(
                "{name} at {} recorded a different version than it now reports",
                was.executable
            );
        }
    }
    "no individual tool differs, which contradicts the comparison".to_string()
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
        uqm_ownership::production_packages(&toolchain.target),
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
    let expected_target = uqm_ownership::target_key(env::consts::OS, env::consts::ARCH)?;
    let expected_dependency_flags = DEPENDENCY_FLAGS
        .iter()
        .map(|flag| (*flag).to_string())
        .collect::<Vec<_>>();
    let checks = [
        ("schema", evidence.schema == BUILD_EVIDENCE_SCHEMA),
        ("source_date_epoch", evidence.source_date_epoch == epoch),
        ("build_date", evidence.build_date == source_date(root)?),
        ("target", evidence.target == expected_target),
        (
            "active_features",
            evidence.active_features == inputs.production_profile.cargo_features,
        ),
        ("toolchain", &evidence.toolchain == toolchain),
        ("packages", evidence.packages == packages),
        (
            "build_environment",
            evidence.build_environment == expected_environment,
        ),
        (
            "compile_profile.target",
            evidence.compile_profile.target == evidence.target,
        ),
        (
            "compile_profile.compiler",
            evidence.compile_profile.compiler == toolchain.cc.executable,
        ),
        (
            "compile_profile.ordered_defines",
            evidence.compile_profile.ordered_defines == expected_defines,
        ),
        (
            "compile_profile.ordered_compile_flags",
            evidence.compile_profile.ordered_compile_flags
                == inputs.production_profile.compile_flags,
        ),
        (
            "compile_profile.dependency_flags",
            evidence.compile_profile.dependency_flags == expected_dependency_flags,
        ),
    ];
    if let Some((field, _)) = checks.iter().find(|(_, matches)| !matches) {
        let detail = if *field == "toolchain" {
            format!(
                ": {}",
                describe_toolchain_difference(&evidence.toolchain, toolchain)
            )
        } else {
            String::new()
        };
        return Err(format!(
            "native build evidence field differs from live identity: {field}{detail}"
        ));
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
    let output = run_bounded_command(git_command(root).args(arguments), &format!("git {label}"))?;
    if output.succeeded() {
        Ok(output.stdout)
    } else {
        Err(output.failure_detail(&format!("git {label}")))
    }
}

fn tracked_worktree_dirty(root: &Path) -> Result<bool, String> {
    let output = run_bounded_command(
        git_command(root).args(["status", "--porcelain", "--untracked-files=no"]),
        "git tracked worktree status",
    )?;
    if !output.succeeded() {
        return Err(output.failure_detail("git tracked worktree status"));
    }
    Ok(!output.stdout.is_empty())
}

fn retained_rust_tool_program(variable: &str, tool: &str) -> Result<String, String> {
    if let Ok(program) = env::var(variable) {
        let path = fs::canonicalize(&program)
            .map_err(|error| format!("cannot resolve {variable} executable {program}: {error}"))?;
        if path.file_name().and_then(OsStr::to_str) == Some(tool) {
            return Ok(path.to_string_lossy().into_owned());
        }
    }
    let output = run_bounded_command(
        Command::new("rustup").args(["which", tool]),
        &format!("rustup {tool} resolution"),
    )?;
    if !output.succeeded() {
        return Err(output.failure_detail(&format!("rustup {tool} resolution")));
    }
    let program = String::from_utf8(output.stdout)
        .map_err(|error| format!("rustup {tool} path is not UTF-8: {error}"))?;
    let program = program.trim();
    if program.is_empty() {
        return Err(format!("rustup did not report a {tool} executable"));
    }
    Ok(program.to_string())
}

fn normalize_path_dependent_tool_versions(
    expected: &mut ToolchainIdentity,
    observed: &ToolchainIdentity,
) {
    expected.rustc.version.clone_from(&observed.rustc.version);
    expected.cargo.version.clone_from(&observed.cargo.version);
    expected.cc.version.clone_from(&observed.cc.version);
    expected.ar.version.clone_from(&observed.ar.version);
    expected.nm.version.clone_from(&observed.nm.version);
    expected
        .pkg_config
        .version
        .clone_from(&observed.pkg_config.version);
    expected.linker.version.clone_from(&observed.linker.version);
}

fn canonical_toolchain(root: &Path) -> Result<ToolchainIdentity, String> {
    let executable_limit = ci::authority::load_authority(root)?
        .supervision
        .executable_member_limit_bytes;
    let cargo_program = retained_rust_tool_program("CARGO", "cargo")?;
    let rustc_program = retained_rust_tool_program("RUSTC", "rustc")?;
    let tool_programs = [
        ("Cargo", cargo_program),
        ("rustc", rustc_program),
        ("C compiler", env::var("CC").unwrap_or_else(|_| "cc".into())),
        ("archiver", env::var("AR").unwrap_or_else(|_| "ar".into())),
        ("nm", env::var("NM").unwrap_or_else(|_| "nm".into())),
        (
            "pkg-config",
            env::var("PKG_CONFIG").unwrap_or_else(|_| "pkg-config".into()),
        ),
    ];
    let mut tools = tool_programs
        .iter()
        .map(|(label, program)| {
            ci::doctor::resolve_executable(program, executable_limit)
                .map_err(|error| format!("resolve retained {label} source: {error}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let host = host_target().map_err(|error| format!("resolve host target: {error}"))?;
    let mut toolchain = resolve_toolchain(root, &host)
        .map_err(|error| format!("resolve canonical toolchain: {error}"))?;
    for tool in &mut tools {
        tool.execute_retained_source();
    }
    toolchain.cargo.executable = tools[0].identity().path.clone();
    toolchain.rustc.executable = tools[1].identity().path.clone();
    toolchain.cc.executable = tools[2].identity().path.clone();
    toolchain.ar.executable = tools[3].identity().path.clone();
    toolchain.nm.executable = tools[4].identity().path.clone();
    toolchain.pkg_config.executable = tools[5].identity().path.clone();
    toolchain.cargo.sha256 = tools[0].identity().sha256.clone();
    toolchain.rustc.sha256 = tools[1].identity().sha256.clone();
    toolchain.cc.sha256 = tools[2].identity().sha256.clone();
    toolchain.ar.sha256 = tools[3].identity().sha256.clone();
    toolchain.nm.sha256 = tools[4].identity().sha256.clone();
    toolchain.pkg_config.sha256 = tools[5].identity().sha256.clone();
    apply_canonical_toolchain_environment(&toolchain);
    let observed = resolve_toolchain(root, &host)
        .map_err(|error| format!("re-resolve canonical toolchain: {error}"))?;
    let mut expected = toolchain;
    normalize_path_dependent_tool_versions(&mut expected, &observed);
    if observed != expected {
        return Err(format!(
            "canonical toolchain does not reproduce from its environment: expected {}; observed {}",
            serde_json::to_string(&expected).unwrap_or_else(|_| "<unserializable>".into()),
            serde_json::to_string(&observed).unwrap_or_else(|_| "<unserializable>".into())
        ));
    }
    for tool in &mut tools {
        tool.verify_unchanged()?;
    }
    let marker = serde_json::to_string(&observed)
        .map_err(|error| format!("cannot serialize canonical toolchain: {error}"))?;
    env::set_var("UQM_CANONICAL_TOOLCHAIN", marker);
    Ok(observed)
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

fn git_command(root: &Path) -> Command {
    let mut command = Command::new("git");
    command
        .current_dir(root)
        .arg("-c")
        .arg(format!("safe.directory={}", root.display()));
    command
}

fn git_text(root: &Path, arguments: &[&str], label: &str) -> Result<String, String> {
    let output = run_bounded_command(git_command(root).args(arguments), &format!("git {label}"))?;
    if !output.succeeded() {
        return Err(output.failure_detail(&format!("git {label}")));
    }
    String::from_utf8(output.stdout)
        .map(|value| value.trim().to_string())
        .map_err(|error| format!("git {label} is not UTF-8: {error}"))
}

fn host_target() -> Result<String, String> {
    let output = run_bounded_command(Command::new("rustc").arg("-vV"), "rustc host query")?;
    if !output.succeeded() {
        return Err(output.failure_detail("rustc host query"));
    }
    String::from_utf8(output.stdout)
        .map_err(|error| format!("rustc output is not UTF-8: {error}"))?
        .lines()
        .find_map(|line| line.strip_prefix("host: ").map(str::to_string))
        .ok_or_else(|| "rustc did not report a host target".into())
}

fn run_harnesses(root: &Path) -> Result<(), String> {
    run_script(root, "rust/harness/run_p00_harness.sh")?;
    run_script(root, "rust/harness/run_menu_binding_probe.sh")
}

fn run_script(root: &Path, script: &str) -> Result<(), String> {
    let authority = ci::load_authority(root)?;
    let member_limit = authority.actions.evidence_snapshot_member_limit_bytes;
    let evidence_parent = root.join("rust/target/xtask-script-evidence");
    fs::create_dir_all(&evidence_parent).map_err(|error| {
        format!(
            "cannot create script evidence parent {}: {error}",
            evidence_parent.display()
        )
    })?;
    let evidence_root = tempfile::Builder::new()
        .prefix("run-")
        .tempdir_in(&evidence_parent)
        .map_err(|error| format!("cannot create script evidence directory: {error}"))?
        .keep();
    let result = run_command(
        Command::new("bash")
            .current_dir(root)
            .env(
                "UQM_CI_EVIDENCE_MEMBER_LIMIT_BYTES",
                member_limit.to_string(),
            )
            .env("UQM_CI_SUBORDINATE_EVIDENCE_ROOT", &evidence_root)
            .arg(script),
        script,
    );
    eprintln!("{script} evidence: {}", evidence_root.display());
    result
}

fn apply_canonical_toolchain_environment(toolchain: &ToolchainIdentity) {
    apply_toolchain_environment(toolchain);
    env::set_var("RUSTC_LINKER", &toolchain.linker.executable);
}

fn run_command(command: &mut Command, label: &str) -> Result<(), String> {
    let captured =
        run_bounded_command(command, label).map_err(|error| format!("{label}: {error}"))?;
    std::io::stdout()
        .write_all(&captured.stdout)
        .map_err(|error| format!("cannot write {label} stdout: {error}"))?;
    std::io::stderr()
        .write_all(&captured.stderr)
        .map_err(|error| format!("cannot write {label} stderr: {error}"))?;
    if captured.succeeded() {
        Ok(())
    } else {
        Err(captured.failure_detail(label))
    }
}
fn run_aqua_command(command: &mut Command, label: &str) -> Result<(), String> {
    let captured = run_bounded_command_in_session(command, label, CommandSession::CurrentAqua)
        .map_err(|error| format!("{label}: {error}"))?;
    std::io::stdout()
        .write_all(&captured.stdout)
        .map_err(|error| format!("cannot write {label} stdout: {error}"))?;
    std::io::stderr()
        .write_all(&captured.stderr)
        .map_err(|error| format!("cannot write {label} stderr: {error}"))?;
    if captured.succeeded() {
        Ok(())
    } else {
        Err(captured.failure_detail(label))
    }
}

#[derive(Clone, Copy)]
enum CommandSession {
    Dedicated,
    CurrentAqua,
}

fn retained_source_mode(
    program: &str,
    environment: &[(String, String)],
) -> Result<(bool, bool), String> {
    let canonical_cargo = environment
        .iter()
        .find(|(name, _)| name == "UQM_CANONICAL_TOOLCHAIN")
        .map(|(_, marker)| {
            serde_json::from_str::<ToolchainIdentity>(marker)
                .map(|toolchain| toolchain.cargo.executable == program)
                .map_err(|error| format!("invalid canonical toolchain marker: {error}"))
        })
        .transpose()?
        .unwrap_or(false);
    Ok((program == "git" || canonical_cargo, canonical_cargo))
}

fn run_bounded_command(command: &Command, label: &str) -> Result<ci::exec::Captured, String> {
    run_bounded_command_in_session(command, label, CommandSession::Dedicated)
}

fn run_bounded_command_in_session(
    command: &Command,
    label: &str,
    session: CommandSession,
) -> Result<ci::exec::Captured, String> {
    let root = repository_root()?;
    let authority = ci::load_authority(&root)?;
    let working_directory = command
        .get_current_dir()
        .map(Path::to_path_buf)
        .unwrap_or(env::current_dir().map_err(|error| format!("cannot determine CWD: {error}"))?);
    let program = command
        .get_program()
        .to_str()
        .ok_or_else(|| format!("{label} executable path is not UTF-8"))?;
    let arguments = command
        .get_args()
        .map(|argument| {
            argument
                .to_str()
                .map(str::to_string)
                .ok_or_else(|| format!("{label} argument is not UTF-8"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut environment = command
        .get_envs()
        .map(|(name, value)| {
            let name = name
                .to_str()
                .ok_or_else(|| format!("{label} environment name is not UTF-8"))?;
            let value = value
                .ok_or_else(|| format!("{label} cannot remove inherited environment variables"))?
                .to_str()
                .ok_or_else(|| format!("{label} environment value is not UTF-8"))?;
            Ok((name.to_string(), value.to_string()))
        })
        .collect::<Result<Vec<_>, String>>()?;
    if !environment
        .iter()
        .any(|(name, _)| name == "UQM_CANONICAL_TOOLCHAIN")
    {
        if let Ok(marker) = env::var("UQM_CANONICAL_TOOLCHAIN") {
            environment.push(("UQM_CANONICAL_TOOLCHAIN".into(), marker));
        }
    }
    let (execute_retained_source, is_canonical_cargo) =
        retained_source_mode(program, &environment)?;
    let bind_environment = |execution_path: &str| {
        let mut environment = environment;
        if is_canonical_cargo {
            let position = environment
                .iter()
                .position(|(name, _)| name == "UQM_CANONICAL_TOOLCHAIN")
                .ok_or_else(|| {
                    "canonical toolchain marker disappeared before launch".to_string()
                })?;
            let mut toolchain: ToolchainIdentity =
                serde_json::from_str(&environment[position].1)
                    .map_err(|error| format!("invalid canonical toolchain marker: {error}"))?;
            toolchain.cargo.executable = execution_path.to_string();
            environment[position].1 = serde_json::to_string(&toolchain).map_err(|error| {
                format!("cannot bind canonical toolchain marker to executable: {error}")
            })?;
            if let Some((_, cargo)) = environment.iter_mut().find(|(name, _)| name == "CARGO") {
                *cargo = execution_path.to_string();
            } else {
                environment.push(("CARGO".into(), execution_path.to_string()));
            }
        }
        Ok(environment)
    };
    let captured = match session {
        CommandSession::Dedicated => ci::exec::run_captured_with_bound_environment(
            &working_directory,
            program,
            &arguments,
            authority.supervision.builtin_limits(),
            execute_retained_source,
            bind_environment,
        ),
        CommandSession::CurrentAqua => ci::exec::run_captured_in_current_aqua_session(
            &working_directory,
            program,
            &arguments,
            authority.supervision.builtin_limits(),
            execute_retained_source,
            bind_environment,
        ),
    };
    Ok(captured)
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path, limit: u64) -> Result<T, String> {
    serde_json::from_slice(&ci::bounded_io::read_regular_nofollow(path, limit)?)
        .map_err(|error| format!("invalid {}: {error}", path.display()))
}

fn read_regular_file_nofollow_bounded(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    ci::bounded_io::read_regular_nofollow(path, limit)
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

    #[cfg(unix)]
    #[test]
    fn linked_build_proof_members_are_group_readable_and_immutable_to_the_group() {
        use std::os::unix::fs::PermissionsExt as _;

        let directory = tempfile::tempdir().unwrap();
        let retained = stage_linked_build_bytes(
            directory.path(),
            "member.json",
            "inputs/linked-build/member.json",
            b"{}\n",
            1024,
        )
        .unwrap();

        assert_eq!(retained.byte_length, 3);
        assert_eq!(
            fs::metadata(directory.path().join("member.json"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o640
        );
    }

    #[test]
    fn packaged_manifest_binds_executable_to_the_package_path() {
        let executable = Artifact {
            role: "executable".into(),
            path: "rust/target/release/uqm".into(),
            media_type: "application/x-executable".into(),
            producing_command: CARGO_BUILD_COMMAND.into(),
            byte_length: 4,
            sha256: hex_sha256(b"game"),
        };
        let mut artifacts = vec![executable.clone()];

        rewrite_packaged_executable(&mut artifacts, &executable, "test-target").unwrap();

        assert_eq!(artifacts[0].path, "rust/target/uqm-package/test-target/uqm");
        assert_eq!(artifacts[0].sha256, executable.sha256);
    }

    #[cfg(unix)]
    #[test]
    fn package_artifact_reads_reject_escape_and_symlink_paths() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().unwrap();
        let target = root.path().join("rust/target");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("safe.bin"), b"safe").unwrap();
        let artifact = |path: &str| Artifact {
            role: "executable".into(),
            path: path.into(),
            media_type: "application/x-executable".into(),
            producing_command: CARGO_BUILD_COMMAND.into(),
            byte_length: 4,
            sha256: hex_sha256(b"safe"),
        };
        assert_eq!(
            read_manifest_artifact(root.path(), &artifact("rust/target/safe.bin"), 64).unwrap(),
            b"safe"
        );
        for path in [
            "../outside",
            "/etc/passwd",
            "rust/target/../../../etc/passwd",
            "rust/Cargo.toml",
        ] {
            assert!(read_manifest_artifact(root.path(), &artifact(path), 64).is_err());
        }

        let outside = root.path().join("outside");
        std::fs::create_dir(&outside).unwrap();
        std::fs::write(outside.join("payload"), b"safe").unwrap();
        symlink(&outside, target.join("linked-parent")).unwrap();
        symlink(outside.join("payload"), target.join("linked-leaf")).unwrap();
        assert!(read_manifest_artifact(
            root.path(),
            &artifact("rust/target/linked-parent/payload"),
            64,
        )
        .is_err());
        assert!(
            read_manifest_artifact(root.path(), &artifact("rust/target/linked-leaf"), 64,).is_err()
        );
    }

    #[test]
    fn production_subprocess_receives_exact_toolchain_environment() {
        fn tool(executable: &str) -> uqm_ownership::ToolIdentity {
            uqm_ownership::ToolIdentity {
                executable: executable.into(),
                version: "test-version".into(),
                sha256: "0".repeat(64),
                effective_args: Vec::new(),
            }
        }

        let toolchain = ToolchainIdentity {
            target: "aarch64-unknown-linux-gnu".into(),
            rustc: tool("/tools/rustc"),
            cargo: tool("/tools/cargo"),
            cc: tool("/tools/cc"),
            ar: tool("/tools/ar"),
            nm: tool("/tools/nm"),
            pkg_config: tool("/tools/pkg-config"),
            linker: tool("/tools/linker"),
        };
        let root = repository_root().unwrap();
        let environment = production_subprocess_environment(&root, &toolchain).unwrap();
        let environment = BTreeMap::from_iter(environment);

        assert_eq!(environment.len(), 13);
        assert_eq!(
            environment["SOURCE_DATE_EPOCH"],
            source_date_epoch(&root).unwrap().to_string()
        );
        assert_eq!(environment["UQM_BUILD_DATE"], source_date(&root).unwrap());
        assert_eq!(environment["UQM_NATIVE_PROFILE"], "production");
        assert_eq!(environment["CARGO_BUILD_JOBS"], "1");
        assert_eq!(environment["CC"], "/tools/cc");
        assert_eq!(environment["AR"], "/tools/ar");
        assert_eq!(environment["NM"], "/tools/nm");
        assert_eq!(environment["PKG_CONFIG"], "/tools/pkg-config");
        assert_eq!(environment["RUSTC"], "/tools/rustc");
        assert_eq!(environment["CARGO"], "/tools/cargo");
        assert_eq!(environment["RUSTC_LINKER"], "/tools/linker");
        assert_eq!(
            environment["CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER"],
            "/tools/linker"
        );
        assert_eq!(
            serde_json::from_str::<ToolchainIdentity>(&environment["UQM_CANONICAL_TOOLCHAIN"])
                .unwrap(),
            toolchain
        );

        let linked_environment =
            BTreeMap::from_iter(linked_build_subprocess_environment(&root, &toolchain).unwrap());
        let mut expected_linked_environment = environment;
        expected_linked_environment.remove("CARGO_BUILD_JOBS");
        expected_linked_environment.insert("UQM_NATIVE_PROFILE".into(), "linked-test".into());
        assert_eq!(linked_environment, expected_linked_environment);
    }

    #[test]
    fn canonical_tool_versions_follow_the_final_executable_identity_only() {
        fn tool(executable: &str, version: &str) -> uqm_ownership::ToolIdentity {
            uqm_ownership::ToolIdentity {
                executable: executable.into(),
                version: version.into(),
                sha256: "0".repeat(64),
                effective_args: Vec::new(),
            }
        }

        let expected = ToolchainIdentity {
            target: "x86_64-unknown-linux-gnu".into(),
            rustc: tool("/tools/rustc", "rustc via alias"),
            cargo: tool("/tools/cargo", "cargo via alias"),
            cc: tool("/tools/gcc-13", "cc via alias"),
            ar: tool("/tools/ar", "ar via alias"),
            nm: tool("/tools/nm", "nm via alias"),
            pkg_config: tool("/tools/pkg-config", "pkg-config via alias"),
            linker: tool("/tools/linker", "linker via alias"),
        };
        let mut observed = expected.clone();
        for identity in [
            &mut observed.rustc,
            &mut observed.cargo,
            &mut observed.cc,
            &mut observed.ar,
            &mut observed.nm,
            &mut observed.pkg_config,
            &mut observed.linker,
        ] {
            identity.version = format!("{} directly", identity.executable);
        }
        let mut normalized = expected.clone();
        normalize_path_dependent_tool_versions(&mut normalized, &observed);
        assert_eq!(normalized, observed);

        observed.cc.executable = "/tools/different-gcc".into();
        normalize_path_dependent_tool_versions(&mut normalized, &observed);
        assert_ne!(normalized, observed);
    }

    #[test]
    fn package_authority_matches_live_manifest_and_recorded_build_command() {
        let authority: ci::authority::Authority =
            serde_json::from_str(include_str!("../../ci/gates.json")).unwrap();
        assert_eq!(
            authority.package.cargo_manifest_sha256,
            hex_sha256(include_bytes!("../../Cargo.toml"))
        );
        for artifact in &authority.package.artifacts {
            if matches!(artifact.role.as_str(), "executable" | "rust_static_archive") {
                assert_eq!(artifact.producing_command, CARGO_BUILD_COMMAND);
            }
        }
    }

    #[test]
    fn production_ownership_resolves_exact_artifact_roles() {
        let root = Path::new("/source");
        let artifact = |role: &str, path: &str| Artifact {
            role: role.into(),
            path: path.into(),
            media_type: "test/type".into(),
            producing_command: "test command".into(),
            byte_length: 1,
            sha256: "0".repeat(64),
        };
        let artifacts = vec![
            artifact("c_static_archive", "rust/target/native/libuqm_c.a"),
            artifact("executable", "rust/target/release/uqm"),
            artifact("provider_report", "rust/target/providers.json"),
            artifact("rust_static_archive", "rust/target/release/libuqm_rust.a"),
        ];

        let resolved = resolve_production_ownership_artifacts(root, &artifacts).unwrap();
        assert_eq!(
            resolved.rust_archive,
            root.join("rust/target/release/libuqm_rust.a")
        );
        assert_eq!(
            resolved.c_archive,
            root.join("rust/target/native/libuqm_c.a")
        );
        assert_eq!(resolved.executable, root.join("rust/target/release/uqm"));

        for missing_role in ["rust_static_archive", "c_static_archive", "executable"] {
            let missing = artifacts
                .iter()
                .filter(|artifact| artifact.role != missing_role)
                .cloned()
                .collect::<Vec<_>>();
            assert_eq!(
                resolve_production_ownership_artifacts(root, &missing).unwrap_err(),
                format!("production role must occur exactly once: {missing_role}")
            );
        }

        let mut duplicate = artifacts.clone();
        duplicate.push(artifact(
            "rust_static_archive",
            "rust/target/release/libduplicate.a",
        ));
        assert_eq!(
            resolve_production_ownership_artifacts(root, &duplicate).unwrap_err(),
            "production role must occur exactly once: rust_static_archive"
        );
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn containment_escape_helper_dispatch_precedes_repository_resolution() {
        assert!(run_containment_hidden_command("not-hidden", &[]).is_none());
        assert_eq!(
            run_containment_hidden_command(ci::exec::CONTAINMENT_ESCAPE_HELPER_COMMAND, &[])
                .unwrap()
                .unwrap_err(),
            "containment escape helper requires one sentinel path"
        );
    }

    #[test]
    fn bounded_command_preserves_rustup_proxy_name() {
        let captured =
            run_bounded_command(Command::new("rustc").arg("-vV"), "rustc proxy").unwrap();
        assert!(
            captured.succeeded(),
            "{}",
            captured.failure_detail("rustc proxy")
        );
        assert!(String::from_utf8(captured.stdout)
            .unwrap()
            .lines()
            .any(|line| line.starts_with("host: ")));
    }

    #[test]
    fn git_commands_mark_the_repository_as_safe_for_containment() {
        let root = Path::new("/checkout");
        let command = git_command(root);
        let arguments: Vec<_> = command.get_args().collect();
        assert_eq!(
            arguments,
            [
                std::ffi::OsStr::new("-c"),
                std::ffi::OsStr::new("safe.directory=/checkout"),
            ]
        );
    }

    #[test]
    fn git_commands_retain_path_dependent_executable_semantics() {
        assert_eq!(retained_source_mode("git", &[]).unwrap(), (true, false));
        assert_eq!(retained_source_mode("rustc", &[]).unwrap(), (false, false));
    }

    #[test]
    fn checked_in_trend_pins_ledger_v7_and_both_hash_table_cutovers() {
        let bytes = include_bytes!("../../build/native-input-trend.json");
        let mut trend: Trend = serde_json::from_slice(bytes).unwrap();
        let root = repository_root().unwrap();
        validate_trend_authority(&root, &trend).unwrap();

        let expected_sha256 = std::mem::replace(&mut trend.ownership_ledger.sha256, "0".repeat(64));
        assert!(validate_trend_authority(&root, &trend)
            .unwrap_err()
            .contains("machine-authority ledger identity"));
        trend.ownership_ledger.sha256 = expected_sha256;

        trend.provider_cutovers[0].canonical_owner = "S2".into();
        assert!(validate_trend_authority(&root, &trend)
            .unwrap_err()
            .contains("differ from ledger v7"));
    }

    #[test]
    fn every_work_command_has_preflight_and_pure_exceptions_are_explicit() {
        for command in ["production", "prove", "package"] {
            assert_eq!(
                preflight_for(command).unwrap(),
                Preflight::StrictSource,
                "{command} should have strict source preflight"
            );
        }
        for command in [
            "debug",
            "release",
            "probe",
            "harness",
            "capture-dependencies",
            "doctor",
        ] {
            assert_eq!(
                preflight_for(command).unwrap(),
                Preflight::Full,
                "{command} should have full preflight"
            );
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

    #[test]
    fn clean_release_dir_removes_existing_directory() {
        let temp = tempfile::tempdir().unwrap();
        let release = temp.path().join("release");
        fs::create_dir_all(&release).unwrap();
        fs::write(release.join("artifact.o"), b"data").unwrap();
        clean_release_dir(&release).unwrap();
        assert!(!release.exists());
    }

    #[test]
    fn clean_release_dir_succeeds_when_directory_absent() {
        let temp = tempfile::tempdir().unwrap();
        let release = temp.path().join("nonexistent");
        clean_release_dir(&release).unwrap();
    }

    #[test]
    fn profile_flag_returns_release_only_for_release() {
        assert_eq!(Profile::Debug.flag(), None);
        assert_eq!(Profile::Release.flag(), Some("--release"));
    }

    #[test]
    fn build_profile_args_includes_release_flag_for_release() {
        let args = build_profile_args(Profile::Release);
        assert!(args.contains(&"--release".to_string()));
        assert!(args.contains(&PRODUCTION_FEATURES.to_string()));
        assert!(args.contains(&"uqm".to_string()));
    }

    #[test]
    fn build_profile_args_omits_release_flag_for_debug() {
        let args = build_profile_args(Profile::Debug);
        assert!(!args.contains(&"--release".to_string()));
        assert!(args.contains(&"build".to_string()));
    }

    #[test]
    fn native_acceptance_uses_the_base_controller_and_release_subject() {
        let target = Path::new("/target");
        let (controller, linked) = native_acceptance_executables(target).unwrap();
        assert_eq!(controller, std::env::current_exe().unwrap());
        assert_ne!(controller, target.join("debug/uqm-native-acceptance"));
        assert_eq!(linked, target.join("release/uqm"));
    }

    #[test]
    fn parse_cargo_tree_line_extracts_real_cargo_features() {
        let entry = parse_cargo_tree_line("fast_image_resize v5.5.0|only_u8x4").unwrap();
        assert_eq!(entry.name, "fast_image_resize");
        assert_eq!(entry.version, "5.5.0");
        assert_eq!(entry.features, vec!["only_u8x4"]);
    }

    #[test]
    fn parse_cargo_tree_line_ignores_package_annotations() {
        let entry = parse_cargo_tree_line("clap_derive v4.6.4 (proc-macro)|default").unwrap();
        assert_eq!(entry.name, "clap_derive");
        assert_eq!(entry.version, "4.6.4");
        assert_eq!(entry.features, vec!["default"]);
    }

    #[test]
    fn parse_cargo_tree_line_handles_workspace_paths_and_empty_features() {
        let entry = parse_cargo_tree_line("uqm v0.8.0 (/checkout/rust)|").unwrap();
        assert_eq!(entry.name, "uqm");
        assert_eq!(entry.version, "0.8.0");
        assert!(entry.features.is_empty());
    }

    #[test]
    fn parse_cargo_tree_line_rejects_malformed_rows() {
        assert!(parse_cargo_tree_line("libc v0.2.0").is_err());
        assert!(parse_cargo_tree_line("libc|default").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn cargo_message_source_accepts_a_canonical_directory_alias() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("main.rs"), b"fn main() {}\n").unwrap();
        let alias = root.path().join("alias");
        std::os::unix::fs::symlink(&source, &alias).unwrap();
        let message = serde_json::json!({
            "target": {"src_path": alias.join("main.rs")}
        });
        assert!(message_source_is(&message, &source.join("main.rs")));
    }

    #[test]
    fn cargo_message_paths_require_one_source_bound_package_identity() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("rust/src")).unwrap();
        fs::write(root.path().join("rust/src/main.rs"), b"fn main() {}\n").unwrap();
        fs::write(root.path().join("rust/src/lib.rs"), b"pub fn marker() {}\n").unwrap();
        let main = fs::canonicalize(root.path().join("rust/src/main.rs")).unwrap();
        let lib = fs::canonicalize(root.path().join("rust/src/lib.rs")).unwrap();
        let package = "path+file:///checkout/rust#uqm@0.8.0";
        let values = [
            serde_json::json!({
                "reason": "compiler-artifact",
                "package_id": package,
                "target": {"name": "uqm", "src_path": main, "kind": ["bin"]},
                "executable": "/target/release/uqm",
                "filenames": []
            }),
            serde_json::json!({
                "reason": "compiler-artifact",
                "package_id": package,
                "target": {"name": "uqm_rust", "src_path": lib, "kind": ["staticlib"]},
                "executable": null,
                "filenames": ["/target/release/deps/libuqm_rust-proof.a"]
            }),
            serde_json::json!({
                "reason": "build-script-executed",
                "package_id": package,
                "out_dir": "/target/release/build/uqm-proof/out"
            }),
        ];
        let mut bytes = values
            .iter()
            .map(serde_json::Value::to_string)
            .collect::<Vec<_>>()
            .join("\n")
            .into_bytes();
        bytes.push(b'\n');

        let paths = parse_production_paths(root.path(), &bytes).unwrap();
        assert_eq!(paths.executable, Path::new("/target/release/uqm"));
        assert_eq!(
            paths.rust_archive,
            Path::new("/target/release/deps/libuqm_rust-proof.a")
        );
        assert_eq!(
            paths.build_evidence,
            Path::new("/target/release/build/uqm-proof/out/native-build-evidence.json")
        );

        let mut forged_values = values;
        forged_values[2]["package_id"] = serde_json::json!("path+file:///other#uqm@0.8.0");
        let forged = forged_values
            .iter()
            .map(serde_json::Value::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(parse_production_paths(root.path(), forged.as_bytes()).is_err());
    }

    #[test]
    fn artifact_schema_is_v4() {
        assert_eq!(ARTIFACT_SCHEMA, "uqm-deterministic-artifacts-v4");
    }

    #[test]
    fn prove_command_constant_includes_full_release_clean() {
        assert!(PROVE_COMMAND.contains("prove"));
        assert_eq!(CLEAN_BUILD_COUNT, 2);
    }

    #[test]
    fn feature_graph_entries_are_sortable() {
        let mut entries = [
            CargoFeatureEntry {
                name: "zlib".into(),
                version: "1.0".into(),
                features: vec![],
            },
            CargoFeatureEntry {
                name: "abc".into(),
                version: "2.0".into(),
                features: vec!["only_u8x4".into()],
            },
        ];
        entries.sort_by(|a, b| a.name.cmp(&b.name).then(a.version.cmp(&b.version)));
        assert_eq!(entries[0].name, "abc");
        assert_eq!(entries[1].name, "zlib");
    }
    #[test]
    fn cargo_test_listing_counts_only_executable_tests() {
        let raw = b"alpha::works: test\nbeta::works: test\nsetup: benchmark\n";
        assert_eq!(
            parse_cargo_test_listing(raw).unwrap(),
            vec!["alpha::works", "beta::works"]
        );
        assert!(parse_cargo_test_listing(&[0xff]).is_err());
        assert!(parse_cargo_test_listing(b": test\n").is_err());
    }

    #[test]
    fn executable_zero_test_listing_is_rejected() {
        let executable = std::env::current_exe().unwrap();
        let output = Command::new(executable)
            .args([
                "definitely_no_such_uqm_test_name",
                "--list",
                "--format",
                "terse",
            ])
            .output()
            .unwrap();
        assert!(output.status.success());
        let tests = parse_cargo_test_listing(&output.stdout).unwrap();
        assert!(tests.is_empty());
        assert_eq!(
            enforce_nonzero_test_inventory("required route", &tests).unwrap_err(),
            "required route listed zero executable tests"
        );
    }
}
