//! Developer tasks for the UQM Rust port: building the linked game and
//! running the native-window scenarios against it.
//!
//! Usage: `cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- <command>`
//! with `debug`, `release`, `test` or `native`.

#[cfg(feature = "native")]
use std::collections::BTreeSet;
use std::env;
#[cfg(feature = "native")]
use std::fs;
#[cfg(feature = "native")]
use std::io::Write;
use std::path::{Path, PathBuf};
#[cfg(feature = "native")]
use std::process::Command;
use std::process::ExitCode;
#[cfg(feature = "native")]
use std::time::Duration;

#[cfg(feature = "native")]
use sha2::{Digest, Sha256};
#[cfg(feature = "native")]
use uqm_rust::automation::native_runner as native_acceptance_runner;
#[cfg(feature = "native")]
use uqm_rust::automation::{
    NativeLinkedBuildReceipt, NativeRetainedInput, NATIVE_LINKED_BUILD_RECEIPT_SCHEMA,
};

mod abi_check;
mod cli;
mod clippy;
mod coverage;
mod deny;
#[cfg(feature = "native")]
mod display_awake;
#[cfg(feature = "native")]
mod evidence;
#[cfg(feature = "native")]
pub mod native_cli;
#[cfg(feature = "native")]
mod native_config;
#[cfg(feature = "native")]
mod native_frames;
#[cfg(feature = "native")]
mod native_gallery;
#[cfg(feature = "native")]
mod native_save;
#[cfg(feature = "native")]
mod native_smoke;
#[cfg(feature = "native")]
mod native_suite;
mod parity;
#[cfg(feature = "native")]
mod process;
pub mod quality;

/// What the game is built with: Rust audio and the normal Cargo game target.
#[cfg(feature = "native")]
const BUILD_FEATURES: &str = "audio_heart";
#[cfg(feature = "native")]
const LINKED_TEST_FEATURES: &str = "audio_heart,debug-process";

/// Subcommand the native-window controller re-invokes this binary with to run
/// one scenario's game child.
#[cfg(feature = "native")]
const NATIVE_ACCEPTANCE_COMMAND: &str = "native-acceptance";

#[derive(Debug)]
#[cfg(feature = "native")]
struct ProductionPaths {
    executable: PathBuf,
    rust_archive: PathBuf,
    out_dir: PathBuf,
}

#[cfg(feature = "native")]
struct LinkedBuildProof {
    directory: tempfile::TempDir,
}

/// Controller executable entry, shared by the thin xtask binary.
pub fn entry() -> ExitCode {
    match cli::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("uqm: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Build the linked game binary the way a developer does.
#[cfg(feature = "native")]
fn build(root: &Path, release: bool) -> Result<(), String> {
    let mut command = Command::new(cargo_program());
    command
        .current_dir(root)
        .args(["build", "--locked", "--manifest-path", "rust/Cargo.toml"]);
    if release {
        command.arg("--release");
    }
    command.args([
        "--no-default-features",
        "--features",
        BUILD_FEATURES,
        "--bin",
        "uqm",
    ]);
    run_command(&mut command, "Cargo build")
}

/// Integration tests against the production Rust providers.
#[cfg(feature = "native")]
fn test_linked(root: &Path) -> Result<(), String> {
    run_command(
        Command::new(cargo_program()).current_dir(root).args([
            "test",
            "--locked",
            "--manifest-path",
            "rust/Cargo.toml",
            "--no-default-features",
            "--features",
            LINKED_TEST_FEATURES,
            "--test",
            "linked_provider_fixture",
            "--test",
            "planets_parity",
            "--test",
            "canvas_native",
            "--test",
            "queue_native",
            "--test",
            "video_native",
        ]),
        "Cargo test linked fixtures",
    )
}

// ---------------------------------------------------------------------------
// Native-window scenarios
// ---------------------------------------------------------------------------

/// Where one native acceptance run gets its selection, evidence and content.
#[derive(Debug, Clone, Default)]
#[cfg(feature = "native")]
pub(crate) struct NativeRunRequest {
    pub scenarios: Option<String>,
    pub evidence_root: Option<PathBuf>,
    pub content_root: Option<PathBuf>,
}

#[cfg(feature = "native")]
pub(crate) fn native_test_with(root: &Path, request: &NativeRunRequest) -> Result<(), String> {
    require_native_platform(cfg!(target_os = "macos"))?;
    let selected = native_config::select(root, request.scenarios.as_deref())?;
    let mut accounting = native_suite::from_request(request.evidence_root.as_deref(), &selected)?;
    let mut phase = native_suite::SuitePhase::Preflight;
    let result = (|| {
        let admission = native_suite::preflight(root, &selected)?;
        println!(
            "native suite source admission (not gameplay proof): {}",
            serde_json::to_string(&admission)
                .map_err(|error| format!("serialize native admission: {error}"))?
        );
        phase = native_suite::SuitePhase::Build;
        let linked_build_proof = build_linked_test_proof(root)?;
        phase = native_suite::SuitePhase::Execute;
        match accounting.as_mut() {
            Some(suite) => {
                run_native_window_acceptance(root, &linked_build_proof, request, &selected, suite)
            }
            None => {
                println!("native window acceptance skipped: no evidence root was bound; native gameplay was not run");
                Ok(())
            }
        }
    })();
    match accounting.as_mut() {
        Some(suite) => native_suite::conclude(suite, result, phase),
        None => result,
    }
}

#[cfg(feature = "native")]
pub(crate) fn require_native_platform(is_macos: bool) -> Result<(), String> {
    if is_macos {
        Ok(())
    } else {
        Err("native window acceptance requires macOS".into())
    }
}

/// Build the release linked binary and stage everything the runner needs to
/// check that the game it launches is the one this build produced.
#[cfg(feature = "native")]
fn build_linked_test_proof(root: &Path) -> Result<LinkedBuildProof, String> {
    let cargo = cargo_program();
    let (output, arguments, paths) = cargo_json_build(root, &cargo)?;
    let directory = tempfile::tempdir_in(&paths.out_dir)
        .map_err(|error| format!("create linked-build proof directory: {error}"))?;
    let member_limit = native_config::runtime_contract()
        .inventory_limits
        .member_bytes;
    let stage_file = |filename: &str, source: &Path| {
        let bytes = read_regular_file_bounded(source, member_limit)?;
        stage_linked_build_bytes(directory.path(), filename, &bytes, member_limit)
    };
    let messages = stage_linked_build_bytes(
        directory.path(),
        "cargo-messages.jsonl",
        &output,
        member_limit,
    )?;
    let rust_archive = stage_file("rust-archive.rlib", &paths.rust_archive)?;
    let executable_bytes = read_regular_file_bounded(&paths.executable, member_limit)?;
    let cargo_manifest = stage_file("Cargo.toml", &root.join("rust/Cargo.toml"))?;
    let cargo_lock = stage_file("Cargo.lock", &root.join("rust/Cargo.lock"))?;
    let receipt = NativeLinkedBuildReceipt {
        schema: NATIVE_LINKED_BUILD_RECEIPT_SCHEMA.to_string(),
        source_sha: git_text(root, &["rev-parse", "HEAD"], "HEAD")?,
        cargo_command: std::iter::once(cargo).chain(arguments).collect(),
        feature: LINKED_TEST_FEATURES.to_string(),
        cargo_executable_path: path_text(&paths.executable, "Cargo executable")?,
        cargo_rust_archive_path: path_text(&paths.rust_archive, "Cargo Rust archive")?,
        cargo_out_dir: path_text(&paths.out_dir, "Cargo OUT_DIR")?,
        executable: retained_identity("inputs/uqm", &executable_bytes),
        cargo_messages: messages,
        rust_archive,
        cargo_manifest,
        cargo_lock,
    };
    let receipt_bytes = serde_json::to_vec_pretty(&receipt)
        .map_err(|error| format!("serialize linked-build receipt: {error}"))?;
    stage_linked_build_bytes(
        directory.path(),
        "linked-build-receipt.json",
        &receipt_bytes,
        member_limit,
    )?;
    Ok(LinkedBuildProof { directory })
}

#[cfg(feature = "native")]
fn path_text(path: &Path, label: &str) -> Result<String, String> {
    path.to_str()
        .map(str::to_string)
        .ok_or_else(|| format!("{label} path is not UTF-8"))
}

/// The file's name inside the linked-build proof, and where the runner finds
/// it relative to the bundle.
#[cfg(feature = "native")]
fn retained_linked_path(filename: &str) -> String {
    format!("inputs/linked-build/{filename}")
}

#[cfg(feature = "native")]
fn retained_identity(relative_path: &str, bytes: &[u8]) -> NativeRetainedInput {
    NativeRetainedInput {
        relative_path: relative_path.to_string(),
        byte_length: bytes.len() as u64,
        sha256: hex_sha256(bytes),
    }
}

#[cfg(feature = "native")]
fn stage_linked_build_bytes(
    directory: &Path,
    filename: &str,
    bytes: &[u8],
    limit: u64,
) -> Result<NativeRetainedInput, String> {
    if bytes.len() as u64 > limit {
        return Err(format!(
            "linked-build proof member {filename} exceeds the {limit} byte limit"
        ));
    }
    fs::write(directory.join(filename), bytes)
        .map_err(|error| format!("write linked-build proof member {filename}: {error}"))?;
    Ok(retained_identity(&retained_linked_path(filename), bytes))
}

#[cfg(feature = "native")]
fn run_native_window_acceptance(
    root: &Path,
    linked_build_proof: &LinkedBuildProof,
    request: &NativeRunRequest,
    selected: &[native_config::Scenario],
    suite: &mut native_suite::SuiteAccounting,
) -> Result<(), String> {
    let evidence_root = request
        .evidence_root
        .as_deref()
        .ok_or("native acceptance needs an evidence root")?;
    let content_root = request.content_root.as_deref().ok_or_else(|| {
        "a content root is required for native acceptance ('native run --content')".to_string()
    })?;
    let controller = env::current_exe()
        .map_err(|error| format!("resolve native-acceptance controller: {error}"))?;
    let linked_executable = root.join("rust/target/release/uqm");

    // A suite that shrank to its default is otherwise indistinguishable in the
    // log from one that was meant to be a single scenario.
    println!(
        "native window acceptance: {} scenario(s): {}",
        selected.len(),
        selected
            .iter()
            .map(|entry| entry.path.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    );
    execute_native_scenarios(
        &NativeScenarioLaunch {
            source: root,
            controller: &controller,
            linked_executable: &linked_executable,
            content_root,
            evidence_root,
            linked_build_proof,
            requested: request.scenarios.as_deref(),
        },
        selected,
        suite,
    )
}

/// Everything one serial native suite execution launches against.
#[cfg(feature = "native")]
pub(crate) struct NativeScenarioLaunch<'a> {
    /// Root whose scenario paths hold the script bytes.
    pub source: &'a Path,
    /// This executable, which is also the observer.
    pub controller: &'a Path,
    pub linked_executable: &'a Path,
    pub content_root: &'a Path,
    pub evidence_root: &'a Path,
    pub linked_build_proof: &'a LinkedBuildProof,
    /// The exact scenario list the caller asked for, forwarded to the child.
    pub requested: Option<&'a str>,
}

#[cfg(feature = "native")]
pub(crate) fn execute_native_scenarios(
    launch: &NativeScenarioLaunch<'_>,
    selected: &[native_config::Scenario],
    suite: &mut native_suite::SuiteAccounting,
) -> Result<(), String> {
    let linked_bytes = read_regular_file_bounded(
        launch.linked_executable,
        native_config::EXECUTABLE_LIMIT_BYTES,
    )?;
    let linked_length = linked_bytes.len().to_string();
    let linked_sha256 = hex_sha256(&linked_bytes);
    let acceptance_policy = serde_json::to_string(&native_config::ACCEPTANCE_POLICY)
        .map_err(|error| format!("serialize native acceptance policy: {error}"))?;
    let member_bytes = native_config::runtime_contract()
        .inventory_limits
        .member_bytes;

    let shared = prepare_native_shared_inputs(
        launch.evidence_root,
        NativeSharedSources {
            source: launch.source,
            executable: launch.linked_executable,
            content: launch.content_root,
            linked: launch.linked_build_proof,
        },
        selected,
        suite.shared_budget(),
    )?;
    for (index, scenario) in selected.iter().enumerate() {
        suite.begin(index)?;
        let label = format!("Direct linked native-window acceptance: {}", scenario.path);
        // Everything up to the spawn either produces the command or fails
        // without a child ever existing, which the receipt has to distinguish
        // from a child that ran and died.
        let prepared = (|| -> Result<(Command, Duration), String> {
            let script = launch.source.join(&scenario.path);
            native_suite::admit_script(launch.source, scenario)?;
            let contract = scenario.runtime_contract();
            let runtime_contract = serde_json::to_string(&contract).map_err(|error| {
                format!("serialize native acceptance runtime contract: {error}")
            })?;
            let scenario_evidence = suite.directory(index)?;
            shared.verify()?;
            let names = native_shared_member_names(scenario)?;
            let references = shared.references(&names)?;
            let allowance = suite.scenario_allowance(index)?;
            let mut command = Command::new(launch.controller);
            command.env(
                "UQM_NATIVE_EVIDENCE_ALLOWANCE",
                serde_json::to_string(&allowance).map_err(|error| error.to_string())?,
            );
            command.env(
                "UQM_NATIVE_SHARED_INPUTS",
                serde_json::to_string(&references).map_err(|error| error.to_string())?,
            );
            command.current_dir(launch.source).args([
                NATIVE_ACCEPTANCE_COMMAND,
                "run",
                &launch.linked_executable.display().to_string(),
                &launch.content_root.display().to_string(),
                &script.display().to_string(),
                &scenario_evidence.display().to_string(),
                &linked_length,
                &linked_sha256,
                &runtime_contract,
                &acceptance_policy,
                &launch
                    .linked_build_proof
                    .directory
                    .path()
                    .display()
                    .to_string(),
                &member_bytes.to_string(),
            ]);
            if let Some(requested) = launch.requested {
                command.env("UQM_NATIVE_SCENARIOS", requested);
            }
            // The controller bounds its game child; this is the last resort.
            let timeout =
                Duration::from_millis(contract.outer_child_timeout_ms) + Duration::from_secs(120);
            Ok((command, timeout))
        })();
        let (mut command, timeout) = match prepared {
            Ok(prepared) => prepared,
            Err(error) => {
                let receipt = native_suite::ControllerSupervision::not_launched(error.clone());
                suite.record_supervision(index, receipt, &[], &[])?;
                return Err(error);
            }
        };
        let captured = process::run(&mut command, Some(timeout));
        echo_output(&captured);
        let supervision = classify_supervision(&captured, &label);
        let outcome = if captured.succeeded() {
            Ok(())
        } else {
            Err(supervision.detail.clone())
        };
        let retention =
            suite.record_supervision(index, supervision, &captured.stdout, &captured.stderr);
        let protection = shared.verify();
        let accounting = suite.account_scenario(index);
        outcome.and(retention).and(protection).and(accounting)?;

        // Each scenario adds evidence, so check the running total as the suite
        // grows rather than after the upload fails, and name the scenario that
        // crossed the line.
        let (bytes, files) = measure_tree(launch.evidence_root)?;
        check_native_suite_evidence(
            bytes,
            files,
            &scenario.path,
            native_config::runtime_contract().inventory_limits,
        )?;
        suite.complete(index)?;
    }
    Ok(())
}

#[cfg(feature = "native")]
fn echo_output(captured: &process::Captured) {
    let _ = std::io::stdout().write_all(&captured.stdout);
    let _ = std::io::stderr().write_all(&captured.stderr);
}

/// The running total of a native suite's retained evidence, against the native
/// inventory limits.
#[cfg(feature = "native")]
fn check_native_suite_evidence(
    bytes: u64,
    files: u64,
    after: &str,
    limits: uqm_rust::automation::native_window::NativeInventoryLimits,
) -> Result<(), String> {
    if bytes > limits.aggregate_bytes {
        return Err(format!(
            "native suite evidence reached {bytes} bytes after {after}, over the {} byte aggregate limit",
            limits.aggregate_bytes
        ));
    }
    if files > u64::from(limits.member_count) {
        return Err(format!(
            "native suite evidence reached {files} files after {after}, over the {} member limit",
            limits.member_count
        ));
    }
    Ok(())
}

/// Classify what the controller observed of a scenario child.
///
/// The order matters: a descendant left behind outranks the exit status of the
/// process that left it.
#[cfg(feature = "native")]
fn classify_supervision(
    captured: &process::Captured,
    label: &str,
) -> native_suite::ControllerSupervision {
    use native_suite::FaultClass;
    let class = if captured.launch_error.is_some() {
        FaultClass::LaunchFailed
    } else if captured.descendant_survivors.is_some() {
        FaultClass::EscapedDescendants
    } else if captured.timed_out {
        FaultClass::Timeout
    } else if captured.signal.is_some() {
        FaultClass::Signal
    } else if captured.succeeded() {
        FaultClass::Completed
    } else {
        FaultClass::Exit
    };
    let detail = if class == FaultClass::Completed {
        format!("{label} completed and exited zero")
    } else {
        captured.failure_detail(label)
    };
    native_suite::ControllerSupervision {
        class,
        detail,
        exit_code: captured.exit_code,
        signal: captured.signal,
        timed_out: captured.timed_out,
        termination_reason: captured.termination_reason.to_string(),
        termination_signal: captured.termination_signal.to_string(),
        process_group_cleanup: captured.process_group_cleanup.to_string(),
        pipe_cleanup: captured.pipe_cleanup.to_string(),
        descendant_survivors: captured.descendant_survivors.clone(),
        stdout_bytes_seen: captured.stdout_bytes_seen,
        stderr_bytes_seen: captured.stderr_bytes_seen,
        stdout_log: None,
        stderr_log: None,
    }
}

#[cfg(feature = "native")]
fn native_shared_member_names(scenario: &native_config::Scenario) -> Result<Vec<String>, String> {
    let filename = Path::new(&scenario.path)
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("script filename is not UTF-8")?;
    let mut names = vec![
        "inputs/uqm".into(),
        format!("inputs/{filename}"),
        format!(
            "inputs/content/packages/{}",
            native_config::CONTENT_FILENAME
        ),
        "inputs/content/version".into(),
    ];
    names.extend(
        [
            "linked-build-receipt.json",
            "cargo-messages.jsonl",
            "rust-archive.rlib",
            "Cargo.toml",
            "Cargo.lock",
        ]
        .map(retained_linked_path),
    );
    Ok(names)
}

#[cfg(feature = "native")]
struct NativeSharedSources<'a> {
    source: &'a Path,
    executable: &'a Path,
    content: &'a Path,
    linked: &'a LinkedBuildProof,
}

#[cfg(feature = "native")]
fn prepare_native_shared_inputs(
    evidence: &Path,
    sources: NativeSharedSources<'_>,
    selected: &[native_config::Scenario],
    budget: uqm_rust::automation::native_artifacts::ArtifactBudget,
) -> Result<uqm_rust::automation::native_artifacts::SharedSnapshot, String> {
    use std::os::unix::fs::PermissionsExt;
    use uqm_rust::automation::native_artifacts::SharedSnapshot;
    let NativeSharedSources {
        source,
        executable,
        content,
        linked,
    } = sources;
    fs::set_permissions(evidence, fs::Permissions::from_mode(0o700))
        .map_err(|error| error.to_string())?;
    let limit = native_config::runtime_contract()
        .inventory_limits
        .member_bytes;
    let mut snapshot = SharedSnapshot::create(&evidence.join("shared"), budget)?;
    snapshot.insert("inputs/uqm", &read_regular_file_bounded(executable, limit)?)?;
    let bytes = read_regular_file_bounded(&content.join(native_config::CONTENT_FILENAME), limit)?;
    if bytes.len() as u64 != native_config::CONTENT_BYTE_LENGTH
        || hex_sha256(&bytes) != native_config::CONTENT_SHA256
    {
        return Err("the content package is not the 0.8.0 content this runner expects".into());
    }
    snapshot.insert(
        &format!(
            "inputs/content/packages/{}",
            native_config::CONTENT_FILENAME
        ),
        &bytes,
    )?;
    snapshot.insert(
        "inputs/content/version",
        format!("{}\n", native_config::CONTENT_VERSION).as_bytes(),
    )?;
    for scenario in selected {
        native_suite::admit_script(source, scenario)?;
        let name = Path::new(&scenario.path)
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or("invalid script filename")?;
        let bytes = read_regular_file_bounded(
            &source.join(&scenario.path),
            native_config::SCRIPT_LIMIT_BYTES,
        )?;
        snapshot.insert(&format!("inputs/{name}"), &bytes)?;
    }
    for entry in fs::read_dir(linked.directory.path()).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "invalid linked member filename")?;
        snapshot.insert(
            &retained_linked_path(&name),
            &read_regular_file_bounded(&entry.path(), limit)?,
        )?;
    }
    snapshot.seal()?;
    Ok(snapshot)
}

/// Total byte length and file count beneath a directory.
///
/// Symbolic links are counted but not followed, so a link cannot inflate the
/// measurement or walk the run out of its own evidence tree.
#[cfg(feature = "native")]
fn measure_tree(root: &Path) -> Result<(u64, u64), String> {
    let mut bytes = 0u64;
    let mut files = 0u64;
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let entries = fs::read_dir(&directory)
            .map_err(|error| format!("read {}: {error}", directory.display()))?;
        for entry in entries {
            let entry = entry.map_err(|error| format!("read {}: {error}", directory.display()))?;
            let metadata = entry
                .path()
                .symlink_metadata()
                .map_err(|error| format!("stat {}: {error}", entry.path().display()))?;
            if metadata.is_dir() {
                pending.push(entry.path());
            } else {
                bytes = bytes.saturating_add(metadata.len());
                files = files.saturating_add(1);
            }
        }
    }
    Ok((bytes, files))
}

// ---------------------------------------------------------------------------
// Toolchain and build plumbing
// ---------------------------------------------------------------------------

/// The cargo that launched this task, so builds use the same toolchain.
#[cfg(feature = "native")]
fn cargo_program() -> String {
    env::var("CARGO").unwrap_or_else(|_| "cargo".into())
}

/// Build `uqm` with `--message-format=json` and locate what it produced.
#[cfg(feature = "native")]
fn cargo_json_build(
    root: &Path,
    cargo: &str,
) -> Result<(Vec<u8>, Vec<String>, ProductionPaths), String> {
    let arguments: Vec<String> = uqm_rust::automation::native_window::NATIVE_LINKED_BUILD_ARGUMENTS
        .into_iter()
        .map(str::to_string)
        .collect();
    let output = process::run(Command::new(cargo).current_dir(root).args(&arguments), None);
    eprint!("{}", String::from_utf8_lossy(&output.stderr));
    if !output.succeeded() {
        return Err(output.failure_detail("Cargo build"));
    }
    let paths = parse_production_paths(root, &output.stdout)?;
    Ok((output.stdout, arguments, paths))
}

#[cfg(feature = "native")]
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
    let build_package = build_script_package(root, &messages, &package_id)?;
    let mut out_dirs = BTreeSet::new();
    let mut executables = BTreeSet::new();
    let mut rust_archives = BTreeSet::new();
    for message in &messages {
        if message["reason"] == "build-script-executed"
            && message["package_id"].as_str() == Some(build_package.as_str())
        {
            if let Some(path) = message["out_dir"].as_str() {
                out_dirs.insert(PathBuf::from(path));
            }
        }
        if message["package_id"].as_str() != Some(package_id.as_str()) {
            continue;
        }
        match message["reason"].as_str() {
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
                    && cargo_message_has_target_kind(message, "rlib")
                    && message_source_is(message, &expected_lib) =>
            {
                for path in message["filenames"].as_array().into_iter().flatten() {
                    let Some(path) = path.as_str() else { continue };
                    let filename = Path::new(path)
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or_default();
                    if filename.starts_with("libuqm_rust-") && filename.ends_with(".rlib") {
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
        out_dir,
    })
}

#[cfg(feature = "native")]
fn build_script_package(
    root: &Path,
    messages: &[serde_json::Value],
    package_id: &str,
) -> Result<String, String> {
    let engine_source = root.join("rust/crates/engine/src/lib.rs");
    if engine_source.exists() {
        exactly_one(
            messages
                .iter()
                .filter(|message| {
                    message["reason"] == "compiler-artifact"
                        && message["target"]["name"] == "uqm_engine"
                        && cargo_message_has_target_kind(message, "rlib")
                        && message_source_is(message, &engine_source)
                })
                .filter_map(|message| message["package_id"].as_str().map(str::to_owned))
                .collect(),
            "engine Cargo package identity",
        )
    } else {
        Ok(package_id.to_owned())
    }
}

#[cfg(feature = "native")]
fn message_source_is(message: &serde_json::Value, expected: &Path) -> bool {
    let Some(expected) = fs::canonicalize(expected).ok() else {
        return false;
    };
    message["target"]["src_path"]
        .as_str()
        .and_then(|path| fs::canonicalize(path).ok())
        .is_some_and(|path| path == expected)
}

#[cfg(feature = "native")]
fn cargo_message_has_target_kind(message: &serde_json::Value, expected: &str) -> bool {
    message["target"]["kind"]
        .as_array()
        .is_some_and(|kinds| kinds.iter().any(|kind| kind == expected))
}

#[cfg(feature = "native")]
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

#[cfg(feature = "native")]
fn git_text(root: &Path, arguments: &[&str], label: &str) -> Result<String, String> {
    let output = process::run(
        Command::new("git")
            .current_dir(root)
            .arg("-c")
            .arg(format!("safe.directory={}", root.display()))
            .args(arguments),
        None,
    );
    if !output.succeeded() {
        return Err(output.failure_detail(&format!("git {label}")));
    }
    String::from_utf8(output.stdout)
        .map(|value| value.trim().to_string())
        .map_err(|error| format!("git {label} is not UTF-8: {error}"))
}

/// Run a command with its output going straight to this terminal.
#[cfg(feature = "native")]
fn run_command(command: &mut Command, label: &str) -> Result<(), String> {
    let status = command
        .status()
        .map_err(|error| format!("{label} did not launch: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{label} failed: {status}"))
    }
}

#[cfg(feature = "native")]
fn read_regular_file_bounded(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    let metadata =
        fs::symlink_metadata(path).map_err(|error| format!("stat {}: {error}", path.display()))?;
    if !metadata.is_file() {
        return Err(format!("{} is not a regular file", path.display()));
    }
    if metadata.len() > limit {
        return Err(format!(
            "{} is {} bytes, over the {limit} byte limit",
            path.display(),
            metadata.len()
        ));
    }
    fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))
}

#[cfg(feature = "native")]
fn hex_sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
#[cfg(feature = "native")]
mod tests {
    use super::*;

    #[test]
    fn native_game_build_reuses_workspace_features_without_building_other_binaries() {
        assert_eq!(
            uqm_rust::automation::native_window::NATIVE_LINKED_BUILD_ARGUMENTS,
            [
                "build",
                "--locked",
                "--manifest-path",
                "rust/Cargo.toml",
                "--workspace",
                "--release",
                "--no-default-features",
                "--features",
                "audio_heart,debug-process",
                "--bin",
                "uqm",
                "--message-format=json-render-diagnostics",
            ]
        );
    }

    #[test]
    fn rust_library_artifacts_use_rlib_instead_of_staticlib() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("rust/src");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("main.rs"), "fn main() {}\n").unwrap();
        fs::write(source.join("lib.rs"), "").unwrap();
        let main = fs::canonicalize(source.join("main.rs")).unwrap();
        let lib = fs::canonicalize(source.join("lib.rs")).unwrap();
        let capture = |kind: &str, archive: &str| {
            [
                serde_json::json!({
                    "reason": "compiler-artifact", "package_id": "uqm-package",
                    "target": {"name": "uqm", "kind": ["bin"], "src_path": main},
                    "executable": "/target/release/uqm"
                }),
                serde_json::json!({
                    "reason": "compiler-artifact", "package_id": "uqm-package",
                    "target": {"name": "uqm_rust", "kind": [kind], "src_path": lib},
                    "filenames": [archive]
                }),
                serde_json::json!({
                    "reason": "build-script-executed", "package_id": "uqm-package",
                    "out_dir": "/target/release/build/uqm/out"
                }),
            ]
            .into_iter()
            .map(|message| format!("{message}\n"))
            .collect::<String>()
        };
        let messages = capture("rlib", "/target/release/deps/libuqm_rust-test.rlib");
        let paths = parse_production_paths(root.path(), messages.as_bytes()).unwrap();
        assert_eq!(paths.executable, Path::new("/target/release/uqm"));
        assert_eq!(
            paths.rust_archive,
            Path::new("/target/release/deps/libuqm_rust-test.rlib")
        );
        let engine_source = root.path().join("rust/crates/engine/src/lib.rs");
        fs::create_dir_all(engine_source.parent().unwrap()).unwrap();
        fs::write(&engine_source, "").unwrap();
        let engine_messages = messages.replace(
            "\"package_id\":\"uqm-package\",\"reason\":\"build-script-executed\"",
            "\"package_id\":\"engine-package\",\"reason\":\"build-script-executed\"",
        ) + &format!(
            "{}\n",
            serde_json::json!({
                "reason": "compiler-artifact", "package_id": "engine-package",
                "target": {"name": "uqm_engine", "kind": ["rlib"], "src_path": engine_source},
                "filenames": ["/target/release/deps/libuqm_engine-test.rlib"]
            })
        );
        assert_eq!(
            parse_production_paths(root.path(), engine_messages.as_bytes())
                .unwrap()
                .out_dir,
            Path::new("/target/release/build/uqm/out")
        );
        let legacy = capture("staticlib", "/target/release/deps/libuqm_rust-test.a");
        assert!(parse_production_paths(root.path(), legacy.as_bytes()).is_err());
        let mismatched = capture("rlib", "/target/release/deps/libuqm_rust-test.a");
        assert!(parse_production_paths(root.path(), mismatched.as_bytes()).is_err());
    }

    #[test]
    fn native_suite_total_is_checked_against_the_native_inventory_limit() {
        let limits = native_config::runtime_contract().inventory_limits;
        let script = "rust/scripts/probe-encounter-v6.json";
        check_native_suite_evidence(700_000_000, 3_000, script, limits).unwrap();
        check_native_suite_evidence(limits.aggregate_bytes, 3_000, script, limits).unwrap();
        let error = check_native_suite_evidence(limits.aggregate_bytes + 1, 3_000, script, limits)
            .unwrap_err();
        assert!(error.contains(script), "{error}");
        assert!(
            check_native_suite_evidence(1, u64::from(limits.member_count) + 1, script, limits)
                .is_err()
        );
    }

    #[test]
    fn each_scenario_child_is_handed_its_own_outer_deadline() {
        let root = cli::repository_root().unwrap();
        let all = native_config::scenarios(&root).unwrap();
        let long = all
            .iter()
            .find(|scenario| scenario.path.ends_with("/comm-encounter-v1.json"))
            .unwrap();
        let short = all
            .iter()
            .find(|scenario| scenario.path.ends_with("/battle-v1.json"))
            .unwrap();
        assert!(
            long.runtime_contract().outer_child_timeout_ms
                > short.runtime_contract().outer_child_timeout_ms
        );
    }

    #[test]
    fn a_child_that_leaves_a_descendant_is_not_a_success() {
        let captured = process::run(
            Command::new("sh").args(["-c", "sleep 30 & exit 0"]),
            Some(Duration::from_secs(20)),
        );
        let supervision = classify_supervision(&captured, "leaky child");
        assert_eq!(
            supervision.class,
            native_suite::FaultClass::EscapedDescendants
        );
        assert!(!captured.succeeded());
    }

    #[test]
    fn a_child_that_outlives_its_deadline_is_killed_and_reported() {
        let captured = process::run(
            Command::new("sh").args(["-c", "sleep 30"]),
            Some(Duration::from_millis(100)),
        );
        assert!(captured.timed_out);
        assert_eq!(
            classify_supervision(&captured, "slow child").class,
            native_suite::FaultClass::Timeout
        );
    }
}
