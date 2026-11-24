//! Running child processes and recording how they ended.

use std::process::Command;
use std::time::Duration;
use uqm_rust::automation::child_session::{ChildSession, ChildSessionError, ChildSessionReceipt};

/// Bytes of a child's output kept in memory; the rest is counted and dropped.
const OUTPUT_LIMIT: usize = 16 * 1024 * 1024;

/// What a child did, in the vocabulary the native suite's teardown report uses.
#[derive(Debug, Clone)]
pub struct Captured {
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
    pub timed_out: bool,
    pub launch_error: Option<String>,
    pub supervision_error: Option<String>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub stdout_bytes_seen: u64,
    pub stderr_bytes_seen: u64,
    pub termination_reason: &'static str,
    pub termination_signal: &'static str,
    pub process_group_cleanup: &'static str,
    pub pipe_cleanup: &'static str,
    /// Set when processes were still alive in the child's group after it exited.
    pub descendant_survivors: Option<String>,
}

impl Captured {
    fn not_launched(error: String) -> Self {
        Self {
            exit_code: None,
            signal: None,
            timed_out: false,
            launch_error: Some(error),
            supervision_error: None,
            stdout: Vec::new(),
            stderr: Vec::new(),
            stdout_bytes_seen: 0,
            stderr_bytes_seen: 0,
            termination_reason: "none",
            termination_signal: "none",
            process_group_cleanup: "not-launched",
            pipe_cleanup: "not-launched",
            descendant_survivors: None,
        }
    }

    #[must_use]
    pub fn succeeded(&self) -> bool {
        self.launch_error.is_none()
            && self.supervision_error.is_none()
            && self.exit_code == Some(0)
            && self.signal.is_none()
            && !self.timed_out
            && self.descendant_survivors.is_none()
            && self.process_group_cleanup == "verified-empty"
            && self.pipe_cleanup == "complete"
    }

    #[must_use]
    pub fn failure_detail(&self, label: &str) -> String {
        if let Some(error) = &self.launch_error {
            return format!("{label} did not launch: {error}");
        }
        let primary = self.process_failure_detail(label);
        match &self.supervision_error {
            Some(error) => format!("{primary}; supervision: {error}"),
            None => primary,
        }
    }

    fn process_failure_detail(&self, label: &str) -> String {
        if self.timed_out {
            return format!(
                "{label} timed out; termination signal {}",
                self.termination_signal
            );
        }
        if let Some(survivors) = &self.descendant_survivors {
            return format!("{label} left live descendants at exit: {survivors}");
        }
        if let Some(signal) = self.signal {
            return format!("{label} was killed by signal {signal}");
        }
        let tail = String::from_utf8_lossy(&self.stderr);
        let tail: String = tail
            .chars()
            .rev()
            .take(2000)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        format!("{label} exited with {:?}: {}", self.exit_code, tail.trim())
    }
}

/// Run an outer controller with its existing output-retention and timeout policy.
pub fn run(command: &mut Command, timeout: Option<Duration>) -> Captured {
    run_bounded(command, timeout, OUTPUT_LIMIT)
}

pub(crate) fn run_bounded(
    command: &mut Command,
    timeout: Option<Duration>,
    limit: usize,
) -> Captured {
    let session = match ChildSession::spawn_outer_captured(command, timeout, limit) {
        Ok(session) => session,
        Err(error) => {
            let detail = format!(
                "failed to start '{}' in {}: {error}",
                command.get_program().to_string_lossy(),
                command.get_current_dir().map_or_else(
                    || "<current directory>".into(),
                    |path| path.display().to_string()
                )
            );
            let before_spawn = matches!(
                error,
                ChildSessionError::Spawn(_)
                    | ChildSessionError::ExecutableIdentity { .. }
                    | ChildSessionError::LogOpen { .. }
            );
            let mut captured = Captured::not_launched(detail.clone());
            if !before_spawn {
                captured.launch_error = None;
                captured.supervision_error = Some(detail);
                captured.process_group_cleanup = "unknown";
                captured.pipe_cleanup = "unknown";
            }
            return captured;
        }
    };
    match session.finish() {
        Ok(receipt) => observed(receipt, false, None),
        Err(failure) => {
            let timed_out = matches!(failure.error, ChildSessionError::Timeout { .. });
            let detail = failure.to_string();
            observed(*failure.receipt, timed_out, Some(detail))
        }
    }
}

fn observed(
    receipt: ChildSessionReceipt,
    timed_out: bool,
    supervision_error: Option<String>,
) -> Captured {
    let supervision_error = supervision_error.or_else(|| {
        (!receipt.cleanup_diagnostics.is_empty()).then(|| receipt.cleanup_diagnostics.join("; "))
    });
    Captured {
        exit_code: receipt.exit_code,
        signal: receipt.signal,
        timed_out,
        launch_error: None,
        supervision_error,
        stdout: receipt.captured.stdout,
        stderr: receipt.captured.stderr,
        stdout_bytes_seen: receipt.captured.stdout_bytes_seen,
        stderr_bytes_seen: receipt.captured.stderr_bytes_seen,
        termination_reason: if timed_out {
            "timeout"
        } else if receipt.descendants_observed {
            "descendants"
        } else {
            "none"
        },
        termination_signal: if receipt.kill_sent {
            "SIGKILL"
        } else if receipt.term_sent {
            "SIGTERM"
        } else {
            "none"
        },
        process_group_cleanup: if receipt.orphan_check_passed && receipt.reaped_status.is_some() {
            "verified-empty"
        } else {
            "unknown"
        },
        pipe_cleanup: if receipt.output_drained {
            "complete"
        } else {
            "failed"
        },
        descendant_survivors: (!timed_out && receipt.descendants_observed).then(|| {
            format!(
                "process group {} still had members after the child exited",
                receipt.identity.pid
            )
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn successful_nonzero_and_signalled_children_keep_their_observed_disposition() {
        for (script, exit, signal) in [
            ("printf stdout; printf stderr >&2", Some(0), None),
            ("exit 7", Some(7), None),
            ("kill -TERM $$", None, Some(libc::SIGTERM)),
        ] {
            let captured = run(
                Command::new("sh").args(["-c", script]),
                Some(Duration::from_secs(5)),
            );
            assert_eq!(captured.exit_code, exit);
            assert_eq!(captured.signal, signal);
            assert_eq!(captured.succeeded(), exit == Some(0));
            assert_eq!(captured.process_group_cleanup, "verified-empty");
            assert_eq!(captured.pipe_cleanup, "complete");
            if exit == Some(0) {
                assert_eq!(captured.stdout, b"stdout");
                assert_eq!(captured.stderr, b"stderr");
            }
        }
    }

    #[test]
    fn missing_cleanup_observations_cannot_certify_a_zero_exit() {
        let session = ChildSession::spawn_outer_captured(
            Command::new("sh").args(["-c", "exit 0"]),
            Some(Duration::from_secs(5)),
            4,
        )
        .unwrap();
        let mut receipt = session.finish().unwrap();
        receipt.orphan_check_passed = false;
        receipt.output_drained = false;
        receipt.reaped_status = None;
        receipt.exit_code = None;
        let captured = observed(
            receipt,
            false,
            Some("injected reap and reader failure".into()),
        );
        assert!(!captured.succeeded());
        assert_eq!(captured.process_group_cleanup, "unknown");
        assert_eq!(captured.pipe_cleanup, "failed");
        assert!(captured.exit_code.is_none() && captured.signal.is_none());
        assert!(captured
            .failure_detail("child")
            .contains("injected reap and reader failure"));
    }

    #[test]
    fn a_command_that_cannot_start_names_its_program_and_directory() {
        let directory = tempfile::tempdir().unwrap();
        let program = "uqm-xtask-no-such-program";
        let expected = directory.path().display().to_string();

        let captured = run(Command::new(program).current_dir(directory.path()), None);
        let launch_error = captured.launch_error.expect("the command must not launch");
        assert!(launch_error.contains(program), "{launch_error}");
        assert!(launch_error.contains(&expected), "{launch_error}");
    }
}
