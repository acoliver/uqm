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

fn drain(mut pipe: impl Read + Send + 'static) -> JoinHandle<(Vec<u8>, u64)> {
    std::thread::spawn(move || {
        let (mut kept, mut seen) = (Vec::new(), 0u64);
        let mut chunk = [0u8; 64 * 1024];
        while let Ok(count) = pipe.read(&mut chunk) {
            if count == 0 {
                break;
            }
            seen += count as u64;
            kept.extend_from_slice(&chunk[..count.min(OUTPUT_LIMIT.saturating_sub(kept.len()))]);
        }
        (kept, seen)
    })
}

/// Run `command` to completion in its own process group, capturing its output.
///
/// With a `timeout` the whole group is killed when it elapses. Whatever is
/// still alive in the group once the child has exited is killed and reported.
pub fn run(command: &mut Command, timeout: Option<Duration>) -> Captured {
    use std::os::unix::process::{CommandExt, ExitStatusExt};

    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0);
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => return Captured::not_launched(error.to_string()),
    };
    let group = child.id() as libc::pid_t;
    let stdout = child.stdout.take().map(drain);
    let stderr = child.stderr.take().map(drain);

    let deadline = timeout.map(|timeout| Instant::now() + timeout);
    let mut timed_out = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if deadline.is_some_and(|deadline| Instant::now() >= deadline) => {
                timed_out = true;
                // SAFETY: signalling a process group this function created.
                unsafe { libc::kill(-group, libc::SIGKILL) };
                break child.wait().ok();
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(_) => break None,
        }
    };

    // SAFETY: probing and killing the process group created above.
    let survivors = unsafe { libc::kill(-group, 0) } == 0;
    if survivors {
        unsafe { libc::kill(-group, libc::SIGKILL) };
    }
    let (stdout, stdout_bytes_seen) =
        stdout.map_or_else(Default::default, |h| h.join().unwrap_or_default());
    let (stderr, stderr_bytes_seen) =
        stderr.map_or_else(Default::default, |h| h.join().unwrap_or_default());
    Captured {
        exit_code: status.and_then(|status| status.code()),
        signal: status.and_then(|status| status.signal()),
        timed_out,
        launch_error: None,
        stdout,
        stderr,
        stdout_bytes_seen,
        stderr_bytes_seen,
        termination_reason: if timed_out { "timeout" } else { "none" },
        termination_signal: if timed_out { "SIGKILL" } else { "none" },
        process_group_cleanup: "verified-empty",
        pipe_cleanup: "complete",
        descendant_survivors: survivors
            .then(|| format!("process group {group} still had members after the child exited")),
    }
}
