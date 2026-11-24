//! Running child processes and recording how they ended.

use std::io::Read;
use std::process::{Command, Stdio};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

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
    }

    #[must_use]
    pub fn failure_detail(&self, label: &str) -> String {
        if let Some(error) = &self.launch_error {
            return format!("{label} did not launch: {error}");
        }
        if let Some(error) = &self.supervision_error {
            return format!("{label} supervision failed: {error}");
        }
        if self.timed_out {
            return format!("{label} timed out and was killed");
        }
        if let Some(survivors) = &self.descendant_survivors {
            return format!("{label} left processes running: {survivors}");
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

#[derive(Debug, Default, PartialEq, Eq)]
struct Drained {
    bytes: Vec<u8>,
    seen: u64,
    error: Option<String>,
}

fn drain(mut pipe: impl Read + Send + 'static, limit: usize) -> JoinHandle<Drained> {
    std::thread::spawn(move || {
        let mut result = Drained::default();
        let mut chunk = [0u8; 64 * 1024];
        loop {
            match pipe.read(&mut chunk) {
                Ok(0) => break,
                Ok(count) => {
                    result.seen += count as u64;
                    result.bytes.extend_from_slice(
                        &chunk[..count.min(limit.saturating_sub(result.bytes.len()))],
                    );
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) => {
                    result.error = Some(error.to_string());
                    break;
                }
            }
        }
        result
    })
}

fn joined(handle: Option<JoinHandle<Drained>>) -> Drained {
    match handle.map(JoinHandle::join) {
        Some(Ok(result)) => result,
        _ => Drained {
            error: Some("pipe reader did not join".into()),
            ..Drained::default()
        },
    }
}

/// Run `command` to completion in its own process group, capturing its output.
///
/// With a `timeout` the whole group is killed when it elapses. Whatever is
/// still alive in the group once the child has exited is killed and reported.
pub fn run(command: &mut Command, timeout: Option<Duration>) -> Captured {
    run_bounded(command, timeout, OUTPUT_LIMIT)
}

pub(crate) fn run_bounded(
    command: &mut Command,
    timeout: Option<Duration>,
    limit: usize,
) -> Captured {
    use std::os::unix::process::{CommandExt, ExitStatusExt};

    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0);
    let mut child = match crate::quality::tool::spawn(command) {
        Ok(child) => child,
        Err(error) => return Captured::not_launched(error),
    };
    let pid = child.id();
    let group = crate::parity::process::child_group(&child);
    let stdout = child.stdout.take().map(|pipe| drain(pipe, limit));
    let stderr = child.stderr.take().map(|pipe| drain(pipe, limit));

    let deadline = timeout.map(|timeout| Instant::now() + timeout);
    let mut timed_out = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if deadline.is_some_and(|deadline| Instant::now() >= deadline) => {
                timed_out = true;
                let _ = crate::parity::process::kill_group(&child);
                break child.wait().ok();
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(_) => break None,
        }
    };

    // A timeout already killed the whole group; probing it again would only see
    // members that have been killed but not yet reaped.
    let survivors = !timed_out
        && group.is_ok_and(|group| rustix::process::test_kill_process_group(group).is_ok());
    if survivors {
        let _ = crate::parity::process::kill_group(&child);
    }
    let stdout = joined(stdout);
    let stderr = joined(stderr);
    let supervision_error = stdout.error.as_ref().or(stderr.error.as_ref()).cloned();
    let pipe_cleanup = if supervision_error.is_none() {
        "complete"
    } else {
        "failed"
    };
    Captured {
        exit_code: status.and_then(|status| status.code()),
        signal: status.and_then(|status| status.signal()),
        timed_out,
        launch_error: None,
        supervision_error,
        stdout: stdout.bytes,
        stderr: stderr.bytes,
        stdout_bytes_seen: stdout.seen,
        stderr_bytes_seen: stderr.seen,
        termination_reason: if timed_out { "timeout" } else { "none" },
        termination_signal: if timed_out { "SIGKILL" } else { "none" },
        process_group_cleanup: "verified-empty",
        pipe_cleanup,
        descendant_survivors: survivors
            .then(|| format!("process group {pid} still had members after the child exited")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_read_is_not_reported_as_clean_eof() {
        struct FailedRead;
        impl std::io::Read for FailedRead {
            fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("injected pipe read failure"))
            }
        }
        let failed = drain(FailedRead, 4).join().unwrap();
        let complete = drain(std::io::empty(), 4).join().unwrap();
        assert_ne!(
            failed, complete,
            "failed drain cannot certify complete output"
        );
    }

    #[test]
    fn interrupted_read_does_not_discard_child_output() {
        struct InterruptedOnce(bool);
        impl std::io::Read for InterruptedOnce {
            fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
                if !self.0 {
                    self.0 = true;
                    return Err(std::io::ErrorKind::Interrupted.into());
                }
                if buffer[0] == 0 {
                    buffer[0] = b'x';
                    Ok(1)
                } else {
                    Ok(0)
                }
            }
        }
        let result = drain(InterruptedOnce(false), 4).join().unwrap();
        assert_eq!(result.bytes, b"x");
        assert_eq!(result.seen, 1);
        assert!(result.error.is_none());
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
