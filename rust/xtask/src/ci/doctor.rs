//! `ci doctor`: tool, source-identity, cleanliness, tuple, and cache checks.
//!
//! Each check is a named contract; failures surface the first contract id.

use std::env;
use std::fs::{self, File};

use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use serde::Serialize;
use sha2::{Digest, Sha256};

use super::authority::Authority;
use super::cache;
use super::load_authority;
use super::plan::derive_plan;

pub const DOCTOR_SCHEMA: &str = "uqm-s4-doctor-v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct ContractCheck {
    contract: String,
    passed: bool,
    detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct DoctorReport {
    schema: String,
    checks: Vec<ContractCheck>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ToolExecutableIdentity {
    pub path: String,
    pub byte_length: u64,
    pub sha256: String,
    pub mode: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct ToolObservation {
    name: String,
    command: Vec<String>,
    expected_output_prefix: Option<String>,
    executable_identity: Option<ToolExecutableIdentity>,
    stdout: String,
    stderr: String,
    exit_code: Option<i32>,
    signal: Option<i32>,
    launch_error: Option<String>,
    passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct ToolReport {
    schema: String,
    pub passed: bool,
    observations: Vec<ToolObservation>,
}

pub fn doctor(root: &Path) -> Result<(), String> {
    let authority = load_authority(root).map_err(|error| format!("authority.load: {error}"))?;
    super::authority::validate_authority(&authority)
        .map_err(|error| format!("authority.validate: {error}"))?;
    let checks = vec![
        check_tools(root, &authority),
        check_source_sha(root, &authority),
        check_clean(root, &authority),
        check_tuple(root, &authority)?,
        check_cache_initial(root, &authority)?,
    ];
    let report = DoctorReport {
        schema: DOCTOR_SCHEMA.to_string(),
        checks,
    };
    let first = report.checks.iter().find(|check| !check.passed);
    println!(
        "{}",
        serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?
    );
    match first {
        None => Ok(()),
        Some(failed) => Err(format!(
            "ci doctor failed at first contract '{}': {}",
            failed.contract, failed.detail
        )),
    }
}

pub(super) fn inspect_tools(root: &Path, authority: &Authority) -> ToolReport {
    let mut observations = Vec::new();
    let limits = authority.supervision.builtin_limits();
    for probe in &authority.tools.preflight {
        observations.push(observe_tool(
            root,
            &probe.name,
            probe.version_command.clone(),
            probe.expected_output_prefix.clone(),
            &probe.accepted_exit_codes,
            limits,
        ));
    }
    for (name, identity) in authority.tools.entries() {
        observations.push(observe_tool(
            root,
            name,
            identity.version_command.clone(),
            Some(identity.expected_output_prefix.clone()),
            &[0],
            limits,
        ));
    }
    ToolReport {
        schema: "uqm-s4-tool-preflight-v2".into(),
        passed: observations.iter().all(|observation| observation.passed),
        observations,
    }
}

pub(crate) struct ResolvedExecutable {
    file: File,
    identity: ToolExecutableIdentity,
    execution_path: String,
    _staging: Option<tempfile::TempDir>,
    source: Option<(File, ToolExecutableIdentity, String)>,
}

impl ResolvedExecutable {
    pub(crate) fn identity(&self) -> &ToolExecutableIdentity {
        &self.identity
    }

    pub(crate) fn execution_path(&self) -> &str {
        &self.execution_path
    }

    // Some tools depend on their installation path. Cargo exports it to build scripts,
    // while packaged macOS tools may resolve adjacent runtime support from that path.
    pub(crate) fn execute_retained_source(&mut self) {
        let Some((source, source_identity, invocation_path)) = self.source.take() else {
            return;
        };
        self.file = source;
        self.execution_path = invocation_path;
        self.identity = source_identity;
        self._staging = None;
    }

    pub(crate) fn verify_unchanged(&mut self) -> Result<(), String> {
        let observed = identity_from_file(
            &mut self.file,
            Path::new(&self.identity.path),
            self.identity.byte_length,
        )?;
        if observed != self.identity {
            return Err("resolved executable changed while it was running".into());
        }
        if self.execution_path != self.identity.path {
            let rebound = fs::canonicalize(&self.execution_path).map_err(|error| {
                format!(
                    "cannot re-resolve executable {}: {error}",
                    self.execution_path
                )
            })?;
            if rebound != Path::new(&self.identity.path) {
                return Err("resolved executable path changed while it was running".into());
            }
        }
        Ok(())
    }
}

fn staged_executable_mode(source_mode: u32) -> u32 {
    let owner_read_execute = source_mode & 0o500;
    owner_read_execute | (owner_read_execute >> 3) | (owner_read_execute >> 6)
}

pub(crate) fn resolve_executable(
    program: &str,
    executable_limit: u64,
) -> Result<ResolvedExecutable, String> {
    let requested = Path::new(program);
    let candidate = if requested.components().count() > 1 {
        PathBuf::from(requested)
    } else {
        env::split_paths(&env::var_os("PATH").unwrap_or_default())
            .map(|directory| directory.join(requested))
            .find(|path| path.is_file())
            .ok_or_else(|| format!("cannot resolve executable '{program}' from PATH"))?
    };
    let invocation_parent = candidate
        .parent()
        .ok_or_else(|| format!("executable path has no parent: {}", candidate.display()))?
        .canonicalize()
        .map_err(|error| format!("cannot resolve {}: {error}", candidate.display()))?;
    let execution_name = candidate
        .file_name()
        .ok_or_else(|| format!("executable path has no filename: {}", candidate.display()))?;
    let invocation_path = invocation_parent.join(execution_name);
    let path = fs::canonicalize(&invocation_path)
        .map_err(|error| format!("cannot resolve {}: {error}", invocation_path.display()))?;
    let (mut source, _) = super::bounded_io::open_regular_nofollow(&path, executable_limit)?;
    let source_identity = identity_from_file(&mut source, &path, executable_limit)?;
    if executable_requires_original_path(&path, &source)? {
        return Ok(ResolvedExecutable {
            execution_path: path.to_string_lossy().into_owned(),
            file: source,
            identity: source_identity,
            _staging: None,
            source: None,
        });
    }
    let staging = executable_staging_directory()?;
    let staging_path = staging
        .path()
        .canonicalize()
        .map_err(|error| format!("cannot resolve executable staging directory: {error}"))?;
    let execution_path = staging_path.join(execution_name);
    let staged_mode = staged_executable_mode(source_identity.mode);
    let mut output = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .mode(staged_mode)
        .open(&execution_path)
        .map_err(|error| format!("cannot stage {}: {error}", path.display()))?;
    let source_bytes = super::bounded_io::read_open_regular(
        &mut source,
        &path,
        source_identity.byte_length,
        executable_limit,
    )?;
    use std::io::Write as _;
    output
        .write_all(&source_bytes)
        .and_then(|()| output.set_permissions(fs::Permissions::from_mode(staged_mode)))
        .and_then(|()| output.sync_all())
        .map_err(|error| format!("cannot sync staged {}: {error}", path.display()))?;
    drop(output);
    let (mut file, _) =
        super::bounded_io::open_regular_nofollow(&execution_path, executable_limit)?;
    let identity = identity_from_file(&mut file, &execution_path, executable_limit)?;
    if identity.byte_length != source_identity.byte_length
        || identity.sha256 != source_identity.sha256
        || identity.mode != staged_mode
    {
        return Err(format!("staged executable differs from {}", path.display()));
    }
    Ok(ResolvedExecutable {
        execution_path: execution_path.to_string_lossy().into_owned(),
        file,
        identity,
        _staging: Some(staging),
        source: Some((
            source,
            source_identity,
            invocation_path.to_string_lossy().into_owned(),
        )),
    })
}

fn executable_staging_directory() -> Result<tempfile::TempDir, String> {
    let root = Path::new("/var/tmp")
        .canonicalize()
        .map_err(|error| format!("cannot resolve executable staging root: {error}"))?;
    let metadata = root
        .metadata()
        .map_err(|error| format!("cannot inspect executable staging root: {error}"))?;
    let mode = metadata.mode();
    if !metadata.is_dir() || metadata.uid() != 0 || mode & 0o1000 == 0 {
        return Err(format!(
            "executable staging root {} is not a root-owned sticky directory",
            root.display()
        ));
    }
    let staging = tempfile::Builder::new()
        .prefix("uqm-executable-")
        .tempdir_in(&root)
        .map_err(|error| format!("cannot create executable staging directory: {error}"))?;
    fs::set_permissions(staging.path(), fs::Permissions::from_mode(0o755))
        .map_err(|error| format!("cannot permit executable staging directory: {error}"))?;
    Ok(staging)
}

fn executable_requires_original_path(path: &Path, file: &File) -> Result<bool, String> {
    let metadata = file
        .metadata()
        .map_err(|error| format!("cannot inspect executable {}: {error}", path.display()))?;
    let mode = metadata.mode();
    let root_protected = metadata.uid() == 0 && mode & 0o022 == 0;
    let system_path = ["/bin", "/sbin", "/usr/bin", "/usr/sbin"]
        .iter()
        .any(|directory| path.parent() == Some(Path::new(directory)));
    let privileged = metadata.uid() == 0 && mode & 0o6000 != 0;
    let name = path.file_name().and_then(|name| name.to_str());
    let homebrew = name == Some("brew")
        && (path.starts_with("/opt/homebrew/") || path.starts_with("/usr/local/Homebrew/"));
    let hosted_python = name.is_some_and(|name| name.starts_with("python"))
        && (path.starts_with("/opt/hostedtoolcache/Python/")
            || path.starts_with("/Library/Frameworks/Python.framework/"));
    Ok((system_path && root_protected) || privileged || homebrew || hosted_python)
}

fn identity_from_file(
    file: &mut File,
    path: &Path,
    executable_limit: u64,
) -> Result<ToolExecutableIdentity, String> {
    let metadata = file
        .metadata()
        .map_err(|error| format!("cannot inspect {}: {error}", path.display()))?;
    if !metadata.is_file() || metadata.len() > executable_limit {
        return Err(format!("{} is not a bounded regular file", path.display()));
    }
    let bytes = super::bounded_io::read_open_regular(file, path, metadata.len(), executable_limit)?;
    Ok(ToolExecutableIdentity {
        path: path.to_string_lossy().into_owned(),
        byte_length: metadata.len(),
        sha256: format!("{:x}", Sha256::digest(&bytes)),
        mode: metadata.permissions().mode() & 0o7777,
    })
}

fn tool_requires_retained_source(program: &str) -> bool {
    matches!(program, "cargo" | "git")
}

fn observe_tool(
    root: &Path,
    name: &str,
    command: Vec<String>,
    expected: Option<String>,
    accepted_exit_codes: &[i32],
    limits: super::exec::Limits,
) -> ToolObservation {
    let Some((program, arguments)) = command.split_first() else {
        return failed_tool_observation(name, command, expected, "empty version command".into());
    };
    let output = super::exec::run_captured_with_bound_environment(
        root,
        program,
        arguments,
        limits,
        tool_requires_retained_source(program),
        |_| Ok(Vec::new()),
    );
    let executable_identity = output.executable_identity.clone();
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let observed = if stdout.trim().is_empty() {
        stderr.trim()
    } else {
        stdout.trim()
    };
    let completed_under_supervision = output.completed_under_supervision();
    let execution_error = (!completed_under_supervision)
        .then(|| format!("supervision: {}", output.failure_detail(program)));
    let version_passed = completed_under_supervision
        && output
            .exit_code
            .is_some_and(|code| accepted_exit_codes.contains(&code))
        && expected
            .as_deref()
            .is_none_or(|prefix| observed.starts_with(prefix));
    ToolObservation {
        name: name.into(),
        command,
        expected_output_prefix: expected,
        passed: version_passed,
        executable_identity,
        stdout,
        stderr,
        exit_code: output.exit_code,
        signal: output.signal,
        launch_error: execution_error,
    }
}

fn failed_tool_observation(
    name: &str,
    command: Vec<String>,
    expected: Option<String>,
    error: String,
) -> ToolObservation {
    ToolObservation {
        name: name.into(),
        command,
        expected_output_prefix: expected,
        executable_identity: None,
        stdout: String::new(),
        stderr: String::new(),
        exit_code: None,
        signal: None,
        launch_error: Some(error),
        passed: false,
    }
}

fn check_tools(root: &Path, authority: &Authority) -> ContractCheck {
    let report = inspect_tools(root, authority);
    if let Some(failed) = report
        .observations
        .iter()
        .find(|observation| !observation.passed)
    {
        let observed = if failed.stdout.trim().is_empty() {
            failed.stderr.trim()
        } else {
            failed.stdout.trim()
        };
        return tool_failure(&failed.name, format!("observed {observed:?}"));
    }
    ContractCheck {
        contract: "doctor.tools".into(),
        passed: true,
        detail: authority
            .tools
            .entries()
            .into_iter()
            .map(|(name, identity)| format!("{name}={}", identity.version))
            .collect::<Vec<_>>()
            .join(", "),
    }
}

fn tool_failure(tool: &str, detail: String) -> ContractCheck {
    ContractCheck {
        contract: "doctor.tools".into(),
        passed: false,
        detail: format!("tool '{tool}' {detail}"),
    }
}

fn check_source_sha(root: &Path, authority: &Authority) -> ContractCheck {
    let contract = "doctor.source_sha";
    match git_text(root, &["rev-parse", "HEAD"], authority) {
        Ok(head) if is_hex_lower(&head, 40) => ContractCheck {
            contract: contract.into(),
            passed: true,
            detail: head,
        },
        Ok(head) => ContractCheck {
            contract: contract.into(),
            passed: false,
            detail: format!("HEAD '{head}' is not the exact 40-hex source SHA"),
        },
        Err(error) => ContractCheck {
            contract: contract.into(),
            passed: false,
            detail: error,
        },
    }
}

fn check_clean(root: &Path, authority: &Authority) -> ContractCheck {
    let contract = "doctor.clean";
    let arguments = ["status", "--porcelain=v1", "--untracked-files=all", "-z"];
    match run_git(root, &arguments, authority) {
        Ok(output) if output.succeeded() && output.stdout.is_empty() => ContractCheck {
            contract: contract.into(),
            passed: true,
            detail: "clean tracked and untracked state".into(),
        },
        Ok(output) if output.succeeded() => {
            let dirty: Vec<String> = output
                .stdout
                .split(|byte| *byte == 0)
                .filter(|entry| !entry.is_empty())
                .map(|entry| String::from_utf8_lossy(entry).into_owned())
                .collect();
            ContractCheck {
                contract: contract.into(),
                passed: false,
                detail: format!("dirty entries: {}", dirty.join(", ")),
            }
        }
        Ok(output) => ContractCheck {
            contract: contract.into(),
            passed: false,
            detail: output.failure_detail("git status"),
        },
        Err(error) => ContractCheck {
            contract: contract.into(),
            passed: false,
            detail: error,
        },
    }
}

fn check_tuple(root: &Path, authority: &Authority) -> Result<ContractCheck, String> {
    let plan = derive_plan(root).map_err(|error| error.to_string())?;
    if authority.matrix_file != super::authority::MATRIX_RELATIVE {
        return Err("authority matrix path differs from the plan matrix path".into());
    }
    let tuple = format!("{}-{}", env::consts::OS, env::consts::ARCH);
    let supported = plan.tuples.iter().any(|item| item.tuple == tuple);
    Ok(ContractCheck {
        contract: "doctor.tuple".into(),
        passed: supported,
        detail: format!("host tuple '{tuple}' supported = {supported}"),
    })
}

fn check_cache_initial(root: &Path, authority: &Authority) -> Result<ContractCheck, String> {
    let contract = "doctor.cache_initial";
    let receipt = cache::inspect(root, &authority.cache)?;
    let passed = receipt.passed;
    let detail = if receipt.mode == authority.cache.mode {
        format!(
            "mode={} cargo_home={} registry={} git={} target_absent={} rust_target_present={} sc2_obj_present={}",
            receipt.mode,
            receipt.isolation_cargo_home,
            receipt.registry_cache_present,
            receipt.git_cache_present,
            receipt.execution_target_absent,
            receipt.rust_target_present,
            receipt.sc2_obj_present,
        )
    } else {
        format!("mode={} (explicit development/test mode)", receipt.mode)
    };
    Ok(ContractCheck {
        contract: contract.into(),
        passed,
        detail,
    })
}

fn git_text(root: &Path, arguments: &[&str], authority: &Authority) -> Result<String, String> {
    let output = run_git(root, arguments, authority)?;
    if !output.succeeded() {
        return Err(output.failure_detail("git"));
    }
    String::from_utf8(output.stdout)
        .map(|value| value.trim().to_string())
        .map_err(|error| format!("git output is not UTF-8: {error}"))
}

fn run_git(
    root: &Path,
    arguments: &[&str],
    authority: &Authority,
) -> Result<super::exec::Captured, String> {
    let arguments: Vec<String> = [
        "-c".to_string(),
        format!("safe.directory={}", root.display()),
    ]
    .into_iter()
    .chain(arguments.iter().map(|argument| (*argument).to_string()))
    .collect();
    Ok(super::exec::run_captured_with_bound_environment(
        root,
        "git",
        &arguments,
        authority.supervision.builtin_limits(),
        true,
        |_| Ok(Vec::new()),
    ))
}

fn is_hex_lower(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn path_dependent_tools_retain_the_resolved_path() {
        assert!(tool_requires_retained_source("cargo"));
        assert!(tool_requires_retained_source("git"));
        assert!(!tool_requires_retained_source("rustc"));
    }

    #[test]
    fn retained_source_preserves_a_final_symlink_invocation() {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("rustup");
        let proxy = temp.path().join("cargo");
        fs::write(
            &target,
            b"#!/bin/sh\n[ \"${0##*/}\" = cargo ] || exit 97\nprintf cargo-proxy",
        )
        .unwrap();
        fs::set_permissions(&target, fs::Permissions::from_mode(0o700)).unwrap();
        std::os::unix::fs::symlink(&target, &proxy).unwrap();

        let captured = super::super::exec::run_captured_with_bound_environment(
            temp.path(),
            proxy.to_str().unwrap(),
            &[],
            super::super::exec::Limits {
                timeout: Duration::from_secs(5),
                termination_grace: Duration::from_secs(1),
                pipe_drain_timeout: Duration::from_secs(1),
                stdout_bytes: 1024,
                stderr_bytes: 1024,
                executable_bytes: 1024,
            },
            true,
            |_| Ok(Vec::new()),
        );

        assert!(captured.succeeded(), "{}", captured.failure_detail("cargo"));
        assert_eq!(captured.stdout, b"cargo-proxy");
        assert_eq!(
            captured.executable_identity.unwrap().path,
            target.canonicalize().unwrap().to_string_lossy()
        );
    }

    #[test]
    fn retained_source_rejects_final_symlink_rebinding() {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("rustup");
        let replacement = temp.path().join("replacement");
        let proxy = temp.path().join("cargo");
        fs::write(&target, b"#!/bin/sh\nexit 0").unwrap();
        fs::write(&replacement, b"#!/bin/sh\nexit 0").unwrap();
        for path in [&target, &replacement] {
            fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
        }
        std::os::unix::fs::symlink(&target, &proxy).unwrap();

        let mut resolved = resolve_executable(proxy.to_str().unwrap(), 1024).unwrap();
        resolved.execute_retained_source();
        fs::remove_file(&proxy).unwrap();
        std::os::unix::fs::symlink(&replacement, &proxy).unwrap();

        assert_eq!(
            resolved.verify_unchanged().unwrap_err(),
            "resolved executable path changed while it was running"
        );
    }

    #[test]
    fn staged_executable_mode_is_cross_uid_and_immutable() {
        assert_eq!(staged_executable_mode(0o755), 0o555);
        assert_eq!(staged_executable_mode(0o750), 0o555);
        assert_eq!(staged_executable_mode(0o550), 0o555);
        assert_eq!(staged_executable_mode(0o700), 0o555);
        assert_eq!(staged_executable_mode(0o6755), 0o555);
    }

    #[test]
    fn resolved_executable_launches_the_opened_file_after_path_replacement() {
        let temp = tempfile::tempdir().unwrap();
        let program = temp.path().join("tool");
        fs::write(
            &program,
            b"#!/bin/sh\n[ \"${0##*/}\" = tool ] || exit 97\nprintf retained",
        )
        .unwrap();
        fs::set_permissions(&program, fs::Permissions::from_mode(0o700)).unwrap();
        let mut resolved = resolve_executable(program.to_str().unwrap(), 1024).unwrap();
        let staging = resolved._staging.as_ref().unwrap();
        assert_eq!(staging.path().metadata().unwrap().mode() & 0o777, 0o755);
        assert_eq!(
            staging.path().parent().unwrap(),
            Path::new("/var/tmp").canonicalize().unwrap()
        );
        fs::rename(&program, temp.path().join("opened-tool")).unwrap();
        fs::write(&program, b"#!/bin/sh\nprintf replacement").unwrap();
        fs::set_permissions(&program, fs::Permissions::from_mode(0o700)).unwrap();

        assert_eq!(
            Path::new(resolved.execution_path())
                .metadata()
                .unwrap()
                .mode()
                & 0o777,
            0o555
        );

        // The kernel reports ETXTBSY while any process still holds a write
        // descriptor for the image being executed. A concurrently forked
        // descendant of this multi-threaded test binary can hold an inherited
        // one for the newly staged file, and releases it at its own exec, so
        // the condition is transient rather than a property of the staging.
        let deadline = std::time::Instant::now() + Duration::from_secs(30);
        let captured = loop {
            let captured = super::super::exec::run_captured_with_limits(
                temp.path(),
                resolved.execution_path(),
                &[],
                &[],
                super::super::exec::Limits {
                    timeout: Duration::from_secs(5),
                    termination_grace: Duration::from_secs(1),
                    pipe_drain_timeout: Duration::from_secs(1),
                    stdout_bytes: 1024,
                    stderr_bytes: 1024,
                    executable_bytes: 1024,
                },
            );
            let busy = captured.failure_detail("tool").contains("Text file busy");
            if !busy || std::time::Instant::now() >= deadline {
                break captured;
            }
            std::thread::sleep(Duration::from_millis(10));
        };

        assert!(captured.succeeded(), "{}", captured.failure_detail("tool"));
        assert_eq!(captured.stdout, b"retained");
        resolved.verify_unchanged().unwrap();
    }

    #[test]
    fn cleanliness_reports_untracked_python_cache_files() {
        let temp = tempfile::tempdir().unwrap();
        let git = |arguments: &[&str]| {
            let status = std::process::Command::new("git")
                .current_dir(temp.path())
                .args(arguments)
                .status()
                .unwrap();
            assert!(status.success(), "git {arguments:?} failed");
        };
        git(&["init", "-q"]);
        fs::write(
            temp.path().join(".gitignore"),
            include_str!("../../../../.gitignore"),
        )
        .unwrap();
        fs::write(temp.path().join("tracked.txt"), b"tracked\n").unwrap();
        git(&["add", ".gitignore", "tracked.txt"]);
        git(&[
            "-c",
            "user.name=UQM CI",
            "-c",
            "user.email=ci@example.invalid",
            "commit",
            "-q",
            "-m",
            "fixture",
        ]);
        let authority: Authority =
            serde_json::from_slice(include_bytes!("../../../ci/gates.json")).unwrap();
        assert!(check_clean(temp.path(), &authority).passed);

        let cache = temp.path().join("src/__pycache__");
        fs::create_dir_all(&cache).unwrap();
        fs::write(cache.join("workflow_supervisor.pyc"), b"generated").unwrap();
        let check = check_clean(temp.path(), &authority);
        assert!(!check.passed);
        assert!(check.detail.contains("workflow_supervisor.pyc"));
    }
}
