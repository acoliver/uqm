//! Bootstrap-proof gate: one fixed S3 profile against the exact packaged
//! executable and manifest, with presented-generation-correlated LCAR, offline
//! validation, and teardown.
//!
//! The S3 `uqm-gameplay-proof` binary supervises the packaged game under SDL
//! dummy drivers and emits `lcar-v1.json`. The internal PNG captures are
//! presented-framebuffer evidence from that session; they are not OS-window
//! screenshots and this gate never claims otherwise.

#[cfg(test)]
use std::fs;
use std::path::Path;

use super::authority::Gate;
use super::cache::CacheEnvironment;
use super::exec::{run_captured_with_limits, Captured};
use super::run::{write_captured, RunSession};
use super::CiError;

pub const PROOF_BIN: &str = "rust/target/debug/uqm-gameplay-proof";
struct ProofStep<'a> {
    id: &'a str,
    cwd: &'a Path,
    command: &'a [String],
    environment: &'a [(String, String)],
    accepted_exit_codes: &'a [i32],
    contract: &'a str,
}

/// Run the bootstrap-proof gate and record evidence.
pub fn run_bootstrap_proof(
    root: &Path,
    session: &mut RunSession,
    cache: &CacheEnvironment,
) -> Result<(), CiError> {
    let gate = session
        .authority
        .gate("bootstrap-proof")
        .cloned()
        .ok_or_else(|| CiError::new("bootstrap-proof.authority", "gate is missing"))?;
    run_bootstrap_steps(root, session, cache, &gate)
}

fn run_bootstrap_steps(
    root: &Path,
    session: &mut RunSession,
    cache: &CacheEnvironment,
    gate: &Gate,
) -> Result<(), CiError> {
    let script = root.join(&session.authority.bootstrap_proof.profile);
    if !script.is_file() {
        return Err(CiError::new(
            "bootstrap-proof.profile",
            format!("fixed S3 profile is absent: {}", script.display()),
        ));
    }
    let target =
        crate::host_target().map_err(|error| CiError::new("bootstrap-proof.target", error))?;
    let package_dir = root.join(&session.authority.bootstrap_proof.packaged_root);
    let packaged_root = package_dir.join(&target);
    let packaged_executable =
        packaged_root.join(&session.authority.bootstrap_proof.packaged_executable);
    let packaged_manifest =
        packaged_root.join(&session.authority.bootstrap_proof.packaged_manifest);
    for required in [&packaged_executable, &packaged_manifest] {
        if !required.is_file() {
            return Err(CiError::new(
                "bootstrap-proof.package",
                format!(
                    "exact packaged executable/manifest is absent: {}; run the package gate first",
                    required.display()
                ),
            ));
        }
    }

    let package_command = vec![
        "cargo".into(),
        "run".into(),
        "--locked".into(),
        "--manifest-path".into(),
        "rust/xtask/Cargo.toml".into(),
        "--".into(),
        "package".into(),
    ];
    session.entry_from_file(
        &packaged_manifest,
        "bootstrap-proof.package-manifest",
        "application/json",
        &gate.id,
        &package_command,
    )?;
    session.entry_from_file(
        &packaged_executable,
        "bootstrap-proof.executable",
        "application/x-executable",
        &gate.id,
        &package_command,
    )?;
    session.entry_from_file(
        &script,
        "bootstrap-proof.profile",
        "application/json",
        &gate.id,
        &package_command,
    )?;

    let build_command = vec![
        "cargo".into(),
        "build".into(),
        "--locked".into(),
        "--manifest-path".into(),
        "rust/Cargo.toml".into(),
        "--bin".into(),
        "uqm-gameplay-proof".into(),
    ];
    run_step(
        session,
        gate,
        ProofStep {
            id: "build-runner",
            cwd: root,
            command: &build_command,
            environment: &cache.resolved().vars,
            accepted_exit_codes: &[0],
            contract: "bootstrap-proof.build",
        },
    )?;
    session.entry_from_file(
        &root.join(PROOF_BIN),
        "bootstrap-proof.runner",
        "application/x-executable",
        &gate.id,
        &build_command,
    )?;

    let output_dir = session.evidence_root.join("bootstrap-proof");
    std::fs::create_dir_all(&output_dir)
        .map_err(|error| CiError::new("bootstrap-proof.output", error.to_string()))?;
    let proof_bin = root.join(PROOF_BIN);

    let run_command = vec![
        proof_bin.display().to_string(),
        "run".into(),
        root.display().to_string(),
        packaged_manifest.display().to_string(),
        script.display().to_string(),
        output_dir.display().to_string(),
    ];
    let run_result = run_step(
        session,
        gate,
        ProofStep {
            id: "run",
            cwd: root,
            command: &run_command,
            environment: &[],
            accepted_exit_codes: &[0],
            contract: "bootstrap-proof.run",
        },
    );
    if let Err(error) = run_result {
        let failure_lcar = output_dir.join("failure-lcar-v1.json");
        if failure_lcar.is_file() {
            let retention = retain_lcar_bundle(
                session,
                gate,
                &run_command,
                &output_dir,
                &failure_lcar,
                "bootstrap-proof.failure-lcar",
            );
            if let Err(retention_error) = retention {
                return Err(CiError::new(
                    "bootstrap-proof.failure-retain",
                    format!(
                        "{}; retaining its failure LCAR failed at {}: {}",
                        error.detail, retention_error.contract, retention_error.detail
                    ),
                ));
            }
        }
        return Err(error);
    }

    let lcar = output_dir.join("lcar-v1.json");
    retain_lcar_bundle(
        session,
        gate,
        &run_command,
        &output_dir,
        &lcar,
        "bootstrap-proof.lcar",
    )?;

    let validate_command = vec![
        proof_bin.display().to_string(),
        "validate".into(),
        lcar.display().to_string(),
    ];
    run_step(
        session,
        gate,
        ProofStep {
            id: "validate",
            cwd: root,
            command: &validate_command,
            environment: &[],
            accepted_exit_codes: &[0],
            contract: "bootstrap-proof.validate",
        },
    )
}

fn retain_lcar_bundle(
    session: &mut RunSession,
    gate: &Gate,
    run_command: &[String],
    output_dir: &Path,
    lcar: &Path,
    lcar_role: &str,
) -> Result<(), CiError> {
    struct RetainedFile {
        bundle_relative: String,
        role: String,
        mime: &'static str,
        bytes: Vec<u8>,
    }

    let lcar_name = lcar
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| CiError::new("bootstrap-proof.lcar", "LCAR filename is invalid"))?;
    let snapshot = super::evidence::EvidenceSnapshot::open(output_dir).map_err(|error| {
        CiError::new(
            "bootstrap-proof.lcar",
            format!("cannot snapshot {}: {error}", output_dir.display()),
        )
    })?;
    let lcar_bytes = snapshot
        .read(lcar_name)
        .map(Vec::from)
        .map_err(|error| CiError::new("bootstrap-proof.lcar", error.to_string()))?;
    let manifest: serde_json::Value = serde_json::from_slice(&lcar_bytes).map_err(|error| {
        CiError::new(
            "bootstrap-proof.lcar",
            format!("cannot parse {}: {error}", lcar.display()),
        )
    })?;
    let artifacts = manifest
        .get("artifacts")
        .and_then(|value| value.as_array())
        .ok_or_else(|| CiError::new("bootstrap-proof.lcar", "LCAR artifacts are absent"))?;

    let mut retained = vec![RetainedFile {
        bundle_relative: format!("payloads/{lcar_role}/{lcar_name}"),
        role: lcar_role.to_string(),
        mime: "application/json",
        bytes: lcar_bytes,
    }];
    let mut paths = std::collections::BTreeSet::new();
    for artifact in artifacts {
        let relative = artifact
            .get("path")
            .and_then(|value| value.as_str())
            .filter(|path| super::evidence::validate_relative_path(path))
            .ok_or_else(|| CiError::new("bootstrap-proof.lcar", "LCAR artifact path is invalid"))?;
        if !paths.insert(relative.to_string()) {
            return Err(CiError::new(
                "bootstrap-proof.lcar",
                format!("duplicate LCAR artifact path '{relative}'"),
            ));
        }
        let bytes = snapshot.read(relative).map(Vec::from).map_err(|error| {
            CiError::new(
                "bootstrap-proof.lcar",
                format!("cannot read LCAR artifact {relative}: {error}"),
            )
        })?;
        retained.push(RetainedFile {
            bundle_relative: format!("payloads/bootstrap-proof.lcar-artifact/{relative}"),
            role: "bootstrap-proof.lcar-artifact".to_string(),
            mime: "application/octet-stream",
            bytes,
        });
    }

    let publisher =
        super::evidence::EvidencePublisher::open(&session.evidence_root).map_err(|error| {
            CiError::new(
                "bootstrap-proof.lcar",
                format!("cannot open evidence root: {error}"),
            )
        })?;
    let mut written = Vec::new();
    let transaction = (|| {
        let mut entries = Vec::with_capacity(retained.len());
        for file in &retained {
            publisher
                .create(&file.bundle_relative, &file.bytes)
                .map_err(|error| {
                    CiError::new(
                        "evidence.bundle_copy",
                        format!("cannot retain {}: {error}", file.bundle_relative),
                    )
                })?;
            written.push(file.bundle_relative.clone());
            entries.push(super::evidence::entry_from_bytes(
                &file.bundle_relative,
                &file.bytes,
                &file.role,
                file.mime,
                &gate.id,
                run_command,
            )?);
        }
        Ok(entries)
    })();

    match transaction {
        Ok(entries) => {
            session.entries.extend(entries);
            Ok(())
        }
        Err(error) => {
            for relative in written {
                let _ = publisher.remove(&relative);
            }
            Err(error)
        }
    }
}

#[cfg(test)]
fn read_regular_output_file(output_dir: &Path, relative: &str) -> Result<Vec<u8>, CiError> {
    super::evidence::read_regular_relative(output_dir, relative).map_err(|error| {
        let detail = if error.raw_os_error() == Some(libc::ELOOP) {
            format!("LCAR artifact path contains a symlink: {relative}")
        } else {
            format!("cannot read LCAR artifact {relative}: {error}")
        };
        CiError::new("bootstrap-proof.lcar", detail)
    })
}

fn validate_step_capture(captured: &Captured, step: ProofStep<'_>) -> Result<(), CiError> {
    if !captured.completed_under_supervision() {
        return Err(CiError::new(
            step.contract,
            captured.failure_detail(&step.command[0]),
        ));
    }
    let code = captured.exit_code.ok_or_else(|| {
        CiError::new(
            step.contract,
            "supervision marked the command complete without an exit code",
        )
    })?;
    if !step.accepted_exit_codes.contains(&code) {
        return Err(CiError::new(
            step.contract,
            format!(
                "command '{}' failed with exit code {code}",
                step.command.join(" ")
            ),
        ));
    }
    Ok(())
}

fn run_step(session: &mut RunSession, gate: &Gate, step: ProofStep<'_>) -> Result<(), CiError> {
    let captured = run_captured_with_limits(
        step.cwd,
        &step.command[0],
        &step.command[1..],
        step.environment,
        session.authority.supervision.builtin_limits(),
    );
    write_captured(
        session,
        gate,
        step.id,
        step.command,
        step.command,
        None,
        &captured,
    )?;
    validate_step_capture(&captured, step)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(project_root: &Path, evidence_root: &Path) -> (RunSession, Gate) {
        let authority = super::super::authority::load_authority(project_root).unwrap();
        let gate = authority
            .gates
            .iter()
            .find(|gate| gate.id == "bootstrap-proof")
            .unwrap()
            .clone();
        (
            RunSession {
                root: project_root.to_path_buf(),
                authority,
                evidence_root: evidence_root.to_path_buf(),
                tuple: "macos-aarch64".to_string(),
                cache_mode: "ambient-dev".to_string(),
                source_sha: "a".repeat(40),
                clean: true,
                features: Vec::new(),
                entries: Vec::new(),
            },
            gate,
        )
    }

    fn completed_capture() -> Captured {
        Captured {
            limits: super::super::exec::Limits {
                timeout: std::time::Duration::from_secs(1),
                termination_grace: std::time::Duration::from_secs(1),
                pipe_drain_timeout: std::time::Duration::from_secs(1),
                stdout_bytes: 1024,
                stderr_bytes: 1024,
                executable_bytes: 1024,
            },
            stdout: Vec::new(),
            stderr: Vec::new(),
            stdout_bytes_seen: 0,
            stderr_bytes_seen: 0,
            stdout_truncated: false,
            stderr_truncated: false,
            executable_identity: None,
            exit_code: Some(0),
            signal: None,
            launch_error: None,
            timed_out: false,
            termination_reason: "none",
            termination_signal: "none",
            process_group_cleanup: "verified-empty",
            pipe_cleanup: "complete",
            supervision_error: None,
            descendant_survivors: None,
        }
    }

    #[test]
    fn zero_exit_does_not_override_supervision_failures() {
        let root = Path::new(".");
        let command = vec!["proof".to_string()];
        let accepted = [0];
        let cases = [
            {
                let mut capture = completed_capture();
                capture.timed_out = true;
                capture.termination_reason = "timeout";
                capture
            },
            {
                let mut capture = completed_capture();
                capture.stdout_truncated = true;
                capture.termination_reason = "output-limit";
                capture
            },
            {
                let mut capture = completed_capture();
                capture.pipe_cleanup = "timed-out";
                capture
            },
            {
                let mut capture = completed_capture();
                capture.termination_reason = "descendant-cleanup";
                capture.process_group_cleanup = "failed";
                capture
            },
        ];

        for capture in cases {
            let step = ProofStep {
                id: "proof",
                cwd: root,
                command: &command,
                environment: &[],
                accepted_exit_codes: &accepted,
                contract: "bootstrap-proof.supervision",
            };
            assert!(validate_step_capture(&capture, step).is_err());
        }
    }

    #[test]
    fn lcar_retention_does_not_publish_a_partial_bundle() {
        let project_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let output = tempfile::tempdir().unwrap();
        let evidence = tempfile::tempdir().unwrap();
        fs::write(output.path().join("present.log"), b"present").unwrap();
        let lcar = output.path().join("failure-lcar-v1.json");
        fs::write(
            &lcar,
            serde_json::to_vec(&serde_json::json!({
                "artifacts": [
                    {"path": "present.log"},
                    {"path": "missing.log"}
                ]
            }))
            .unwrap(),
        )
        .unwrap();
        let (mut session, gate) = session(project_root, evidence.path());
        let command = vec!["proof".to_string(), "run".to_string()];

        assert!(retain_lcar_bundle(
            &mut session,
            &gate,
            &command,
            output.path(),
            &lcar,
            "bootstrap-proof.failure-lcar",
        )
        .is_err());
        assert!(session.entries.is_empty());
        assert!(!evidence.path().join("payloads").exists());
    }

    #[test]
    fn lcar_retention_rolls_back_files_after_a_destination_failure() {
        let project_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let output = tempfile::tempdir().unwrap();
        let evidence = tempfile::tempdir().unwrap();
        fs::write(output.path().join("present.log"), b"present").unwrap();
        fs::create_dir(output.path().join("blocked")).unwrap();
        fs::write(output.path().join("blocked/artifact.log"), b"blocked").unwrap();
        let lcar = output.path().join("failure-lcar-v1.json");
        fs::write(
            &lcar,
            serde_json::to_vec(&serde_json::json!({
                "artifacts": [
                    {"path": "present.log"},
                    {"path": "blocked/artifact.log"}
                ]
            }))
            .unwrap(),
        )
        .unwrap();
        let artifact_root = evidence
            .path()
            .join("payloads/bootstrap-proof.lcar-artifact");
        fs::create_dir_all(&artifact_root).unwrap();
        fs::write(artifact_root.join("blocked"), b"directory blocker").unwrap();
        let (mut session, gate) = session(project_root, evidence.path());
        let command = vec!["proof".to_string(), "run".to_string()];

        assert!(retain_lcar_bundle(
            &mut session,
            &gate,
            &command,
            output.path(),
            &lcar,
            "bootstrap-proof.failure-lcar",
        )
        .is_err());
        assert!(session.entries.is_empty());
        assert!(!evidence
            .path()
            .join("payloads/bootstrap-proof.failure-lcar/failure-lcar-v1.json")
            .exists());
        assert!(!artifact_root.join("present.log").exists());
        assert!(artifact_root.join("blocked").is_file());
    }

    #[cfg(unix)]
    #[test]
    fn lcar_retention_rejects_symlinked_output_without_following_it() {
        use std::os::unix::fs::symlink;

        let output = tempfile::tempdir().unwrap();
        let outside = tempfile::NamedTempFile::new().unwrap();
        symlink(outside.path(), output.path().join("artifact.log")).unwrap();
        let error = read_regular_output_file(output.path(), "artifact.log").unwrap_err();
        assert_eq!(error.contract, "bootstrap-proof.lcar");
        assert!(error.detail.contains("symlink"));
    }
}
