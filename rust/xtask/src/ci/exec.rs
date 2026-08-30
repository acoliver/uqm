//! Bounded, deadline-enforced subprocess execution shared by every CI gate step.

use std::io::Read;
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread;
use std::time::{Duration, Instant};
#[cfg(unix)]
use std::{collections::BTreeMap, os::fd::RawFd};

#[cfg(all(test, unix))]
use uqm_rust::automation::child_session::NESTED_GROUP_REGISTRATION_FD_ENV;
#[cfg(unix)]
use uqm_rust::automation::child_session::{
    read_nested_group_request, write_nested_group_response, NestedGroupOperation,
    NestedGroupProtocol,
};

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub(super) const DEDICATED_CONTAINMENT_UID_ENV: &str = "UQM_CI_DEDICATED_CONTAINMENT_UID";
#[cfg(any(target_os = "macos", target_os = "linux"))]
const DEDICATED_CONTAINMENT_HOME_ENV: &str = "UQM_CI_DEDICATED_CONTAINMENT_HOME";
#[cfg(any(target_os = "macos", target_os = "linux"))]
const DEDICATED_CONTAINMENT_USER_ENV: &str = "UQM_CI_DEDICATED_CONTAINMENT_USER";
#[cfg(any(target_os = "macos", target_os = "linux"))]
pub const CONTAINMENT_ESCAPE_HELPER_COMMAND: &str = "__ci-containment-escape-helper";
#[cfg(any(target_os = "macos", target_os = "linux"))]
const DEDICATED_UID_WRAPPER: &str = include_str!("dedicated_uid_wrapper.sh");
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub timeout: Duration,
    pub termination_grace: Duration,
    pub pipe_drain_timeout: Duration,
    pub stdout_bytes: usize,
    pub stderr_bytes: usize,
    pub executable_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct Captured {
    pub limits: Limits,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub stdout_bytes_seen: u64,
    pub stderr_bytes_seen: u64,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
    pub executable_identity: Option<super::doctor::ToolExecutableIdentity>,
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
    pub launch_error: Option<String>,
    pub timed_out: bool,
    pub termination_reason: &'static str,
    pub termination_signal: &'static str,
    pub process_group_cleanup: &'static str,
    pub pipe_cleanup: &'static str,
    pub supervision_error: Option<String>,
    pub descendant_survivors: Option<String>,
}

impl Captured {
    pub fn completed_under_supervision(&self) -> bool {
        self.exit_code.is_some()
            && self.signal.is_none()
            && self.launch_error.is_none()
            && !self.timed_out
            && !self.stdout_truncated
            && !self.stderr_truncated
            && self.termination_reason == "none"
            && self.pipe_cleanup == "complete"
            && self.supervision_error.is_none()
            && matches!(
                self.process_group_cleanup,
                "verified-empty" | "not-supported"
            )
    }

    pub fn succeeded(&self) -> bool {
        self.exit_code == Some(0) && self.completed_under_supervision()
    }

    pub fn failure_detail(&self, program: &str) -> String {
        if let Some(error) = &self.launch_error {
            return error.clone();
        }
        if let Some(error) = &self.supervision_error {
            return format!("subprocess supervision failed for {program}: {error}");
        }
        match self.termination_reason {
            "timeout" => format!("subprocess {program} exceeded its authorized timeout"),
            "output-limit" => format!("subprocess {program} exceeded an authorized output limit"),
            "descendant-cleanup" => match &self.descendant_survivors {
                Some(survivors) => format!(
                    "subprocess {program} left descendants in its owned process group: {survivors}"
                ),
                None => format!("subprocess {program} left descendants in its owned process group"),
            },
            _ => format!(
                "subprocess {program} failed with exit code {:?} and signal {:?}",
                self.exit_code, self.signal
            ),
        }
    }
}

#[derive(Clone, Copy)]
enum Stream {
    Stdout,
    Stderr,
}

enum StreamEvent {
    Bytes(Stream, Vec<u8>),
    Finished(Stream, Option<String>),
}

fn pump<R: Read + Send + 'static>(mut reader: R, stream: Stream, sender: SyncSender<StreamEvent>) {
    let mut buffer = vec![0_u8; 16 * 1024];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => {
                let _ = sender.send(StreamEvent::Finished(stream, None));
                return;
            }
            Ok(length) => {
                if sender
                    .send(StreamEvent::Bytes(stream, buffer[..length].to_vec()))
                    .is_err()
                {
                    return;
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(error) => {
                let _ = sender.send(StreamEvent::Finished(stream, Some(error.to_string())));
                return;
            }
        }
    }
}

struct CaptureState {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    stdout_bytes_seen: u64,
    stderr_bytes_seen: u64,
    stdout_finished: bool,
    stderr_finished: bool,
    stdout_truncated: bool,
    stderr_truncated: bool,
    error: Option<String>,
}

impl CaptureState {
    fn new(limits: Limits) -> Self {
        Self {
            stdout: Vec::with_capacity(limits.stdout_bytes.min(64 * 1024)),
            stderr: Vec::with_capacity(limits.stderr_bytes.min(64 * 1024)),
            stdout_bytes_seen: 0,
            stderr_bytes_seen: 0,
            stdout_finished: false,
            stderr_finished: false,
            stdout_truncated: false,
            stderr_truncated: false,
            error: None,
        }
    }

    fn accept(&mut self, event: StreamEvent, limits: Limits) {
        match event {
            StreamEvent::Bytes(stream, bytes) => {
                let (captured, seen, truncated, limit) = match stream {
                    Stream::Stdout => (
                        &mut self.stdout,
                        &mut self.stdout_bytes_seen,
                        &mut self.stdout_truncated,
                        limits.stdout_bytes,
                    ),
                    Stream::Stderr => (
                        &mut self.stderr,
                        &mut self.stderr_bytes_seen,
                        &mut self.stderr_truncated,
                        limits.stderr_bytes,
                    ),
                };
                match u64::try_from(bytes.len())
                    .ok()
                    .and_then(|length| seen.checked_add(length))
                {
                    Some(total) => *seen = total,
                    None => {
                        self.error.get_or_insert_with(|| {
                            "captured byte count overflowed u64".to_string()
                        });
                        return;
                    }
                }
                let remaining = limit.saturating_sub(captured.len());
                captured.extend_from_slice(&bytes[..bytes.len().min(remaining)]);
                *truncated = *seen > limit as u64;
            }
            StreamEvent::Finished(stream, error) => {
                match stream {
                    Stream::Stdout => self.stdout_finished = true,
                    Stream::Stderr => self.stderr_finished = true,
                }
                if let Some(error) = error {
                    self.error
                        .get_or_insert_with(|| format!("cannot read captured pipe: {error}"));
                }
            }
        }
    }

    fn drain(&mut self, receiver: &Receiver<StreamEvent>, limits: Limits) {
        while let Ok(event) = receiver.try_recv() {
            self.accept(event, limits);
        }
    }

    fn pipes_finished(&self) -> bool {
        self.stdout_finished && self.stderr_finished
    }

    fn output_limited(&self) -> bool {
        self.stdout_truncated || self.stderr_truncated
    }
}

#[cfg(unix)]
const MONITOR_REGISTRATION_FD_ENV: &str = "UQM_CI_MONITOR_REGISTRATION_FD";
#[cfg(unix)]
const MONITOR_LIFELINE_FD_ENV: &str = "UQM_CI_MONITOR_LIFELINE_FD";
#[cfg(unix)]
const MONITOR_GRACE_MS_ENV: &str = "UQM_CI_MONITOR_GRACE_MS";
#[cfg(any(target_os = "macos", target_os = "linux"))]
const MONITOR_DEDICATED_UID_ENV: &str = "UQM_CI_MONITOR_DEDICATED_UID";
#[cfg(any(target_os = "macos", target_os = "linux"))]
const PRIVILEGED_CLEANUP_TIMEOUT: Duration = Duration::from_secs(5);
#[cfg(any(target_os = "macos", target_os = "linux"))]
const CONTAINMENT_ESCAPE_PROBE_TIMEOUT: Duration = Duration::from_secs(120);
#[cfg(unix)]
pub const CONTAINMENT_MONITOR_COMMAND: &str = "__ci-containment-monitor";

#[cfg(unix)]
#[derive(Debug)]
struct LeaderAnchor {
    pid: libc::pid_t,
    monitor_anchor_pid: libc::pid_t,
    observed: bool,
    observed_at: Option<Instant>,
    reaped: bool,
    dedicated_uid: Option<String>,
}

#[cfg(unix)]
impl LeaderAnchor {
    fn new(
        pid: u32,
        monitor_anchor_pid: libc::pid_t,
        dedicated_uid: Option<String>,
    ) -> Result<Self, String> {
        if monitor_anchor_pid <= 0 {
            return Err("monitor anchor PID must be positive".to_string());
        }
        Ok(Self {
            pid: libc::pid_t::try_from(pid)
                .map_err(|_| "child process identifier does not fit pid_t".to_string())?,
            monitor_anchor_pid,
            observed: false,
            observed_at: None,
            reaped: false,
            dedicated_uid,
        })
    }

    fn observe(&mut self) -> Result<bool, String> {
        if self.reaped {
            return Err("cannot observe a reaped leader anchor".to_string());
        }
        if self.observed {
            return Ok(true);
        }
        let mut info = std::mem::MaybeUninit::<libc::siginfo_t>::zeroed();
        // SAFETY: info is writable siginfo_t storage. WNOWAIT keeps the exact
        // child waitable and prevents PID/process-group reuse through cleanup.
        let result = unsafe {
            libc::waitid(
                libc::P_PID,
                self.pid as libc::id_t,
                info.as_mut_ptr(),
                libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
            )
        };
        if result == -1 {
            return Err(format!(
                "cannot inspect child status: {}",
                std::io::Error::last_os_error()
            ));
        }
        // SAFETY: waitid initialized info on success.
        if unsafe { info.assume_init().si_pid() } != 0 {
            self.observed = true;
            self.observed_at = Some(Instant::now());
        }
        Ok(self.observed)
    }

    fn mark_reaped(&mut self) -> Result<(), String> {
        if !self.observed {
            return Err("leader must be observed before final reap".to_string());
        }
        if self.reaped {
            return Err("leader may be reaped only once".to_string());
        }
        self.reaped = true;
        Ok(())
    }

    fn descendant_cleanup_required(
        &self,
        now: Instant,
        group_clean: bool,
        settle_grace: Duration,
    ) -> bool {
        !group_clean
            && self.observed_at.is_some_and(|observed_at| {
                now.saturating_duration_since(observed_at) >= settle_grace
            })
    }
}

#[cfg(unix)]
fn fd_io(fd: RawFd, bytes: &mut [u8], write: bool) -> Result<(), std::io::Error> {
    let deadline = Instant::now()
        .checked_add(Duration::from_secs(5))
        .ok_or_else(|| std::io::Error::from_raw_os_error(libc::EOVERFLOW))?;
    let mut offset = 0;
    while offset < bytes.len() {
        let now = Instant::now();
        if now >= deadline {
            return Err(std::io::Error::from_raw_os_error(libc::ETIMEDOUT));
        }
        let remaining = deadline.saturating_duration_since(now).as_millis();
        let remaining = i32::try_from(remaining).unwrap_or(i32::MAX).max(1);
        let mut descriptor = libc::pollfd {
            fd,
            events: if write { libc::POLLOUT } else { libc::POLLIN },
            revents: 0,
        };
        // SAFETY: descriptor points to one initialized pollfd.
        let ready = unsafe { libc::poll(&mut descriptor, 1, remaining) };
        if ready == 0 {
            return Err(std::io::Error::from_raw_os_error(libc::ETIMEDOUT));
        }
        if ready == -1 {
            let error = std::io::Error::last_os_error();
            if error.kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            return Err(error);
        }
        if descriptor.revents & (libc::POLLERR | libc::POLLNVAL) != 0
            || descriptor.revents & descriptor.events == 0
        {
            return Err(std::io::Error::from_raw_os_error(libc::EPIPE));
        }
        // SAFETY: bytes is valid for the requested operation and fd is owned by
        // the containment protocol participant.
        let result = unsafe {
            if write {
                libc::write(fd, bytes[offset..].as_ptr().cast(), bytes.len() - offset)
            } else {
                libc::read(
                    fd,
                    bytes[offset..].as_mut_ptr().cast(),
                    bytes.len() - offset,
                )
            }
        };
        if result > 0 {
            offset += usize::try_from(result)
                .map_err(|_| std::io::Error::from_raw_os_error(libc::EOVERFLOW))?;
        } else if result == -1
            && std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted
        {
            continue;
        } else if result == 0 {
            return Err(std::io::Error::from_raw_os_error(libc::EPIPE));
        } else {
            return Err(std::io::Error::last_os_error());
        }
    }
    Ok(())
}

#[cfg(unix)]
fn configure_process_group(
    command: &mut Command,
    protocol: NestedGroupProtocol,
    registration_timeout: Duration,
) {
    use std::os::unix::process::CommandExt as _;
    command.process_group(0);
    protocol.apply_environment(command);
    // SAFETY: fcntl, poll, read, write, clock_gettime, and getpid are async-signal-safe.
    unsafe {
        command.pre_exec(move || {
            protocol.make_inheritable()?;
            protocol.exchange_with_timeout(
                NestedGroupOperation::Register,
                libc::getpid(),
                registration_timeout,
            )?;
            Ok(())
        });
    }
}

#[cfg(not(unix))]
fn configure_process_group(_command: &mut Command, _registration_timeout: Duration) {}

#[cfg(target_os = "macos")]
fn macos_process_is_terminal(pid: i32) -> Result<bool, String> {
    // A process that has exited but has not been reaped is terminal. The kernel
    // process record reports that state; proc_pidinfo refuses such a process,
    // which is why membership is decided from the record instead.
    match macos_process_record(pid) {
        Some(record) => Ok(record.terminal),
        None => Ok(true),
    }
}

/// One process as the kernel reports it.
#[cfg(target_os = "macos")]
struct MacosProcessRecord {
    group: i32,
    terminal: bool,
    start_micros: u64,
}

/// Read a process record from the kernel.
///
/// `proc_listpids` filters by group without reporting each process's own group,
/// and `proc_pidinfo` refuses a process that has exited but has not been reaped,
/// which is exactly the process a supervision check must classify. This record
/// answers both questions at once and carries the pid, so a record that does not
/// describe the requested process is rejected rather than trusted.
#[cfg(target_os = "macos")]
fn macos_process_record(pid: libc::pid_t) -> Option<MacosProcessRecord> {
    // Offsets within the KERN_PROC_PID record: kp_proc.p_stat, kp_proc.p_pid,
    // and kp_eproc.e_pgid.
    const START_SECONDS_OFFSET: usize = 0;
    const START_MICROSECONDS_OFFSET: usize = 8;
    const STATE_OFFSET: usize = 36;
    const PID_OFFSET: usize = 40;
    const GROUP_OFFSET: usize = 564;
    const MINIMUM_RECORD: usize = GROUP_OFFSET + std::mem::size_of::<i32>();
    const ZOMBIE_STATE: u8 = 5;

    let mut mib: [libc::c_int; 4] = [libc::CTL_KERN, libc::KERN_PROC, libc::KERN_PROC_PID, pid];
    let mut required: usize = 0;
    // SAFETY: mib names KERN_PROC_PID for one PID, and a null buffer with a
    // writable length requests the record size without writing any bytes.
    let sized = unsafe {
        libc::sysctl(
            mib.as_mut_ptr(),
            mib.len() as u32,
            std::ptr::null_mut(),
            &raw mut required,
            std::ptr::null_mut(),
            0,
        )
    };
    if sized != 0 || required < MINIMUM_RECORD {
        return None;
    }
    let mut record = vec![0_u8; required];
    let mut written = required;
    // SAFETY: record is writable for written bytes, and the kernel reports the
    // byte count it produced through written.
    let read = unsafe {
        libc::sysctl(
            mib.as_mut_ptr(),
            mib.len() as u32,
            record.as_mut_ptr().cast::<libc::c_void>(),
            &raw mut written,
            std::ptr::null_mut(),
            0,
        )
    };
    if read != 0 || written < MINIMUM_RECORD {
        return None;
    }
    let field = |offset: usize| -> i32 {
        let mut bytes = [0_u8; 4];
        bytes.copy_from_slice(&record[offset..offset + 4]);
        i32::from_ne_bytes(bytes)
    };
    if field(PID_OFFSET) != pid {
        return None;
    }
    let unsigned_field = |offset: usize| -> u64 {
        let mut bytes = [0_u8; 8];
        bytes.copy_from_slice(&record[offset..offset + 8]);
        u64::from_ne_bytes(bytes)
    };
    let start_seconds = unsigned_field(START_SECONDS_OFFSET);
    let start_microseconds = u64::from(field(START_MICROSECONDS_OFFSET).unsigned_abs());
    Some(MacosProcessRecord {
        group: field(GROUP_OFFSET),
        terminal: record[STATE_OFFSET] == ZOMBIE_STATE,
        start_micros: start_seconds
            .saturating_mul(1_000_000)
            .saturating_add(start_microseconds),
    })
}

#[cfg(target_os = "macos")]
fn group_member_other_than(
    process_group: i32,
    ignored: &[i32],
) -> Result<Option<libc::pid_t>, String> {
    const PROC_PGRP_ONLY: u32 = 2;
    for _ in 0..3 {
        // SAFETY: a null buffer requests the required size.
        let required = unsafe {
            libc::proc_listpids(
                PROC_PGRP_ONLY,
                u32::try_from(process_group).map_err(|_| "negative process group".to_string())?,
                std::ptr::null_mut(),
                0,
            )
        };
        if required < 0 {
            return Err(format!(
                "cannot inspect process group {process_group}: {}",
                std::io::Error::last_os_error()
            ));
        }
        let slots = usize::try_from(required)
            .ok()
            .and_then(|bytes| bytes.checked_div(std::mem::size_of::<libc::pid_t>()))
            .and_then(|count| count.checked_add(16))
            .ok_or_else(|| "process-group member count overflow".to_string())?;
        let mut pids = vec![0 as libc::pid_t; slots];
        let bytes = i32::try_from(pids.len() * std::mem::size_of::<libc::pid_t>())
            .map_err(|_| "process-group buffer does not fit c_int".to_string())?;
        // SAFETY: pids is writable for bytes bytes.
        let written = unsafe {
            libc::proc_listpids(
                PROC_PGRP_ONLY,
                u32::try_from(process_group).map_err(|_| "negative process group".to_string())?,
                pids.as_mut_ptr().cast(),
                bytes,
            )
        };
        if written < 0 {
            return Err(format!(
                "cannot inspect process group {process_group}: {}",
                std::io::Error::last_os_error()
            ));
        }
        if written < bytes {
            let count = usize::try_from(written)
                .ok()
                .and_then(|value| value.checked_div(std::mem::size_of::<libc::pid_t>()))
                .ok_or_else(|| "invalid process-group byte count".to_string())?;
            for pid in pids[..count]
                .iter()
                .copied()
                .filter(|pid| *pid > 0 && !ignored.contains(pid))
            {
                // The filter can name a process that has left the group, whose
                // PID was reused, or that has exited without being reaped.
                let Some(record) = macos_process_record(pid) else {
                    continue;
                };
                if record.group != process_group || record.terminal {
                    continue;
                }
                return Ok(Some(pid));
            }
            return Ok(None);
        }
    }
    Err("process-group membership changed during every inspection".to_string())
}
/// Describe one process for a supervision failure message.
#[cfg(target_os = "macos")]
fn describe_process(pid: libc::pid_t) -> String {
    let mut path = [0_u8; libc::PROC_PIDPATHINFO_MAXSIZE as usize];
    // SAFETY: path is writable for its full length.
    let written = unsafe { libc::proc_pidpath(pid, path.as_mut_ptr().cast(), path.len() as u32) };
    let name = usize::try_from(written)
        .ok()
        .filter(|length| *length > 0)
        .and_then(|length| String::from_utf8(path[..length].to_vec()).ok())
        .unwrap_or_else(|| "no executable path".to_string());
    // proc_pidpath can name the image a departing process was spawned from
    // rather than the one it ran, so report the kernel's own view beside it.
    match macos_process_record(pid) {
        Some(record) => format!(
            "{pid} ({name}, group {}, {})",
            record.group,
            if record.terminal { "exited" } else { "live" }
        ),
        None => format!("{pid} ({name}, absent from the process table)"),
    }
}

#[cfg(target_os = "linux")]
fn linux_process_state_and_group(stat: &str, path: &Path) -> Result<(char, i32), String> {
    let after_comm = stat
        .rfind(')')
        .ok_or_else(|| format!("malformed process stat: {}", path.display()))?;
    let mut fields = stat[after_comm + 1..].split_whitespace();
    let state = fields
        .next()
        .and_then(|value| value.chars().next())
        .ok_or_else(|| format!("missing process state in {}", path.display()))?;
    let group = fields
        .nth(1)
        .ok_or_else(|| format!("missing process group in {}", path.display()))?
        .parse::<i32>()
        .map_err(|error| format!("invalid process group in {}: {error}", path.display()))?;
    Ok((state, group))
}

#[cfg(target_os = "linux")]
fn group_member_other_than(
    process_group: i32,
    ignored: &[i32],
) -> Result<Option<libc::pid_t>, String> {
    for entry in std::fs::read_dir("/proc").map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<i32>().ok())
        else {
            continue;
        };
        if ignored.contains(&pid) {
            continue;
        }
        let stat_path = entry.path().join("stat");
        let stat = match std::fs::read_to_string(&stat_path) {
            Ok(stat) => stat,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(format!("cannot inspect {}: {error}", stat_path.display()));
            }
        };
        let (state, group) = linux_process_state_and_group(&stat, &stat_path)?;
        if state != 'Z' && group == process_group {
            return Ok(Some(pid));
        }
    }
    Ok(None)
}

/// Describe one process for a supervision failure message.
#[cfg(target_os = "linux")]
fn describe_process(pid: libc::pid_t) -> String {
    let name = std::fs::read_to_string(format!("/proc/{pid}/comm"))
        .map(|comm| comm.trim().to_string())
        .unwrap_or_else(|_| "no executable name".to_string());
    // Report the kernel's own view beside the name, matching what the macOS
    // path reports, so a failure message carries the same evidence either way.
    let stat_path = PathBuf::from(format!("/proc/{pid}/stat"));
    match std::fs::read_to_string(&stat_path)
        .map_err(|error| error.to_string())
        .and_then(|stat| linux_process_state_and_group(&stat, &stat_path))
    {
        Ok((state, group)) => format!(
            "{pid} ({name}, group {group}, {})",
            if state == 'Z' { "exited" } else { "live" }
        ),
        Err(_) => format!("{pid} ({name}, absent from the process table)"),
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn privileged_uid_pkill_command(uid: &str, signal: i32, selector: &str) -> Result<Command, String> {
    let signal = match signal {
        libc::SIGTERM => "-TERM",
        libc::SIGKILL => "-KILL",
        _ => {
            return Err(format!(
                "unsupported privileged UID cleanup signal {signal}"
            ))
        }
    };
    if !matches!(selector, "-U" | "-u") {
        return Err(format!("unsupported privileged UID selector {selector}"));
    }
    let mut command = Command::new("/usr/bin/sudo");
    command
        .args(["-n", "/usr/bin/pkill", signal, selector, uid])
        .env_clear()
        .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    Ok(command)
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn run_uid_process_command(
    mut command: Command,
    operation: &str,
    uid: &str,
    selector: &str,
) -> Result<bool, String> {
    let mut child = command
        .spawn()
        .map_err(|error| format!("cannot launch {operation} for {selector} {uid}: {error}"))?;
    let deadline = Instant::now()
        .checked_add(PRIVILEGED_CLEANUP_TIMEOUT)
        .ok_or_else(|| format!("{operation} deadline overflow"))?;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => match status.code() {
                Some(0) => return Ok(true),
                Some(1) => return Ok(false),
                _ => {
                    return Err(format!(
                        "{operation} for {selector} {uid} failed with {status}"
                    ));
                }
            },
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(5)),
            Ok(None) => {
                let kill_error = child.kill().err();
                let wait_error = child.wait().err();
                return Err(format!(
                    "{operation} for {selector} {uid} exceeded its deadline{}{}",
                    kill_error
                        .map(|error| format!("; cannot terminate helper: {error}"))
                        .unwrap_or_default(),
                    wait_error
                        .map(|error| format!("; cannot reap helper: {error}"))
                        .unwrap_or_default()
                ));
            }
            Err(error) => {
                return Err(format!(
                    "cannot inspect {operation} for {selector} {uid}: {error}"
                ));
            }
        }
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn run_privileged_uid_pkill(uid: &str, signal: i32, selector: &str) -> Result<bool, String> {
    run_uid_process_command(
        privileged_uid_pkill_command(uid, signal, selector)?,
        "privileged UID cleanup",
        uid,
        selector,
    )
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn privileged_uid_cleanup(uid: &str, signal: i32) -> Result<(), String> {
    let mut first_error = None;
    for selector in ["-U", "-u"] {
        if let Err(error) = run_privileged_uid_pkill(uid, signal, selector) {
            first_error.get_or_insert(error);
        }
    }
    first_error.map_or(Ok(()), Err)
}

#[cfg(target_os = "macos")]
fn macos_process_matches_uid(info: &libc::proc_bsdinfo, uid: u32) -> bool {
    info.pbi_status != libc::SZOMB && (info.pbi_uid == uid || info.pbi_ruid == uid)
}

#[cfg(target_os = "macos")]
fn privileged_uid_is_empty(uid: &str) -> Result<bool, String> {
    const PROC_UID_ONLY: u32 = 4;
    const PROC_RUID_ONLY: u32 = 5;

    let uid = uid
        .parse::<u32>()
        .map_err(|error| format!("invalid dedicated containment uid: {error}"))?;
    for selector in [PROC_UID_ONLY, PROC_RUID_ONLY] {
        let mut inspected = false;
        for _ in 0..3 {
            // SAFETY: a null buffer requests the required size.
            let required = unsafe { libc::proc_listpids(selector, uid, std::ptr::null_mut(), 0) };
            if required < 0 {
                return Err(format!(
                    "cannot inspect processes for dedicated uid {uid}: {}",
                    std::io::Error::last_os_error()
                ));
            }
            let slots = usize::try_from(required)
                .ok()
                .and_then(|bytes| bytes.checked_div(std::mem::size_of::<libc::pid_t>()))
                .and_then(|count| count.checked_add(16))
                .ok_or_else(|| "process count overflow".to_string())?;
            let mut pids = vec![0 as libc::pid_t; slots];
            let bytes = i32::try_from(pids.len() * std::mem::size_of::<libc::pid_t>())
                .map_err(|_| "process buffer does not fit c_int".to_string())?;
            // SAFETY: pids is writable for bytes bytes.
            let written =
                unsafe { libc::proc_listpids(selector, uid, pids.as_mut_ptr().cast(), bytes) };
            if written < 0 {
                return Err(format!(
                    "cannot inspect processes for dedicated uid {uid}: {}",
                    std::io::Error::last_os_error()
                ));
            }
            if written < bytes {
                let count = usize::try_from(written)
                    .ok()
                    .and_then(|value| value.checked_div(std::mem::size_of::<libc::pid_t>()))
                    .ok_or_else(|| "invalid process-list byte count".to_string())?;
                for pid in pids[..count].iter().copied().filter(|pid| *pid > 0) {
                    let mut info = std::mem::MaybeUninit::<libc::proc_bsdinfo>::zeroed();
                    let expected = i32::try_from(std::mem::size_of::<libc::proc_bsdinfo>())
                        .map_err(|_| "macOS process-info size does not fit c_int".to_string())?;
                    // SAFETY: info is writable for expected bytes and pid came from proc_listpids.
                    let observed = unsafe {
                        libc::proc_pidinfo(
                            pid,
                            libc::PROC_PIDTBSDINFO,
                            0,
                            info.as_mut_ptr().cast(),
                            expected,
                        )
                    };
                    if observed == 0 {
                        match macos_process_is_terminal(pid) {
                            Ok(true) => continue,
                            Ok(false) => return Ok(false),
                            Err(error) => {
                                return Err(format!(
                                    "cannot inspect process {pid} for dedicated uid {uid}: {error}"
                                ));
                            }
                        }
                    }
                    if observed != expected {
                        return Err(format!(
                            "cannot inspect process {pid} for dedicated uid {uid}: proc_pidinfo returned {observed} bytes, expected {expected}"
                        ));
                    }
                    // SAFETY: proc_pidinfo initialized the complete structure.
                    if macos_process_matches_uid(unsafe { &info.assume_init() }, uid) {
                        return Ok(false);
                    }
                }
                inspected = true;
                break;
            }
        }
        if !inspected {
            return Err(format!(
                "process membership changed during every dedicated uid {uid} selector {selector} inspection"
            ));
        }
    }
    Ok(true)
}

#[cfg(target_os = "linux")]
fn linux_process_real_and_effective_uids(status: &str, path: &Path) -> Result<(u32, u32), String> {
    let line = status
        .lines()
        .find_map(|line| line.strip_prefix("Uid:"))
        .ok_or_else(|| format!("missing process UIDs in {}", path.display()))?;
    let mut fields = line.split_whitespace();
    let real = fields
        .next()
        .ok_or_else(|| format!("missing real process UID in {}", path.display()))?
        .parse::<u32>()
        .map_err(|error| format!("invalid real process UID in {}: {error}", path.display()))?;
    let effective = fields
        .next()
        .ok_or_else(|| format!("missing effective process UID in {}", path.display()))?
        .parse::<u32>()
        .map_err(|error| {
            format!(
                "invalid effective process UID in {}: {error}",
                path.display()
            )
        })?;
    Ok((real, effective))
}

#[cfg(target_os = "linux")]
fn privileged_uid_is_empty(uid: &str) -> Result<bool, String> {
    let uid = uid
        .parse::<u32>()
        .map_err(|error| format!("invalid dedicated containment uid: {error}"))?;
    for entry in std::fs::read_dir("/proc").map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        if entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<i32>().ok())
            .is_none()
        {
            continue;
        }
        let stat_path = entry.path().join("stat");
        let stat = match std::fs::read_to_string(&stat_path) {
            Ok(stat) => stat,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(format!("cannot inspect {}: {error}", stat_path.display()));
            }
        };
        let (state, _) = linux_process_state_and_group(&stat, &stat_path)?;
        if state == 'Z' {
            continue;
        }
        let status_path = entry.path().join("status");
        let status = match std::fs::read_to_string(&status_path) {
            Ok(status) => status,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(format!("cannot inspect {}: {error}", status_path.display()));
            }
        };
        let (real, effective) = linux_process_real_and_effective_uids(&status, &status_path)?;
        if real == uid || effective == uid {
            return Ok(false);
        }
    }
    Ok(true)
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn wait_for_privileged_uid_empty(uid: &str, timeout: Duration) -> Result<(), String> {
    let deadline = Instant::now()
        .checked_add(timeout)
        .ok_or_else(|| "privileged UID cleanup verification deadline overflow".to_string())?;
    loop {
        if privileged_uid_is_empty(uid)? {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "dedicated containment uid {uid} still owns processes"
            ));
        }
        thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(unix)]
fn signal_group(anchor: &LeaderAnchor, signal: i32) -> Result<(), String> {
    if anchor.reaped {
        return Err("group signal requires an unreaped leader anchor".to_string());
    }
    // SAFETY: the unreaped exact leader prevents process-group ID reuse.
    if unsafe { libc::kill(-anchor.pid, signal) } == 0 {
        return Ok(());
    }
    let error = std::io::Error::last_os_error();
    match error.raw_os_error() {
        Some(libc::ESRCH) => Ok(()),
        // A kernel can refuse a group signal once every member has released
        // its credentials while exiting. The survivor contract covers live
        // members, so confirm emptiness instead of trusting the errno alone.
        Some(libc::EPERM) if group_occupant(anchor)?.is_none() => Ok(()),
        _ => Err(format!(
            "cannot signal process group {}: {error}",
            anchor.pid
        )),
    }
}

#[cfg(unix)]
fn group_occupant(anchor: &LeaderAnchor) -> Result<Option<libc::pid_t>, String> {
    group_member_other_than(anchor.pid, &[anchor.pid, anchor.monitor_anchor_pid])
}

#[cfg(unix)]
fn process_group_clean(anchor: &LeaderAnchor) -> Result<bool, String> {
    if !anchor.observed || group_occupant(anchor)?.is_some() {
        return Ok(false);
    }
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    if let Some(uid) = &anchor.dedicated_uid {
        return privileged_uid_is_empty(uid);
    }
    Ok(true)
}

#[cfg(not(unix))]
fn process_group_clean(status: &Option<ExitStatus>) -> Result<bool, String> {
    Ok(status.is_some())
}

#[cfg(unix)]
fn terminate_owned(anchor: &LeaderAnchor, force: bool) -> Result<(), String> {
    if process_group_clean(anchor)? {
        return Ok(());
    }
    let signal = if force { libc::SIGKILL } else { libc::SIGTERM };
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    if let Some(uid) = &anchor.dedicated_uid {
        privileged_uid_cleanup(uid, signal)?;
    }
    signal_group(anchor, signal)
}

#[cfg(not(unix))]
fn terminate_owned(
    child: &mut std::process::Child,
    force: bool,
    status: &Option<ExitStatus>,
) -> Result<(), String> {
    let _ = force;
    if status.is_some() {
        Ok(())
    } else {
        child
            .kill()
            .map_err(|error| format!("cannot terminate child: {error}"))
    }
}

#[cfg(unix)]
struct Containment {
    registration: RawFd,
    token_read: RawFd,
    token_write: RawFd,
    lifeline_write: RawFd,
    monitor: std::process::Child,
    dedicated_uid: Option<String>,
}

#[cfg(unix)]
impl Containment {
    fn start(grace: Duration) -> Result<Self, std::io::Error> {
        use std::os::unix::process::CommandExt as _;

        #[cfg(any(target_os = "macos", target_os = "linux"))]
        let dedicated_uid = dedicated_containment_config()?.map(|config| config.uid);
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        let dedicated_uid: Option<String> = None;
        let (monitor_registration, registration) = socket_pair_cloexec()?;
        let (token_read, token_write) = match pipe_cloexec() {
            Ok(pipe) => pipe,
            Err(error) => {
                close_fds(&[monitor_registration, registration]);
                return Err(error);
            }
        };
        let (lifeline_read, lifeline_write) = match pipe_cloexec() {
            Ok(pipe) => pipe,
            Err(error) => {
                close_fds(&[monitor_registration, registration, token_read, token_write]);
                return Err(error);
            }
        };
        if let Err(error) = fd_io(token_write, &mut [1], true) {
            close_fds(&[
                monitor_registration,
                registration,
                token_read,
                token_write,
                lifeline_read,
                lifeline_write,
            ]);
            return Err(error);
        }
        let executable = match std::env::current_exe() {
            Ok(path) => path,
            Err(error) => {
                close_fds(&[
                    monitor_registration,
                    registration,
                    token_read,
                    token_write,
                    lifeline_read,
                    lifeline_write,
                ]);
                return Err(error);
            }
        };
        let mut command = Command::new(executable);
        #[cfg(test)]
        command.args([
            "--exact",
            "ci::exec::tests::containment_monitor_process",
            "--nocapture",
        ]);
        #[cfg(not(test))]
        command.arg(CONTAINMENT_MONITOR_COMMAND);
        command
            .process_group(0)
            .env(
                MONITOR_REGISTRATION_FD_ENV,
                monitor_registration.to_string(),
            )
            .env(MONITOR_LIFELINE_FD_ENV, lifeline_read.to_string())
            .env(
                MONITOR_GRACE_MS_ENV,
                grace.as_millis().min(u128::from(u64::MAX)).to_string(),
            )
            .stdin(Stdio::null())
            .stdout(Stdio::null());
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            command
                .env_remove(DEDICATED_CONTAINMENT_UID_ENV)
                .env_remove(DEDICATED_CONTAINMENT_HOME_ENV)
                .env_remove(DEDICATED_CONTAINMENT_USER_ENV);
            match dedicated_uid.as_deref() {
                Some(uid) => command.env(MONITOR_DEDICATED_UID_ENV, uid),
                None => command.env_remove(MONITOR_DEDICATED_UID_ENV),
            };
        }
        #[cfg(not(test))]
        command.stderr(Stdio::null());
        // SAFETY: fcntl and close are async-signal-safe. A separate process
        // group keeps the monitor outside every group it is authorized to kill.
        unsafe {
            command.pre_exec(move || {
                for fd in [monitor_registration, lifeline_read] {
                    if libc::fcntl(fd, libc::F_SETFD, 0) == -1 {
                        return Err(std::io::Error::last_os_error());
                    }
                }
                for fd in [registration, token_read, token_write, lifeline_write] {
                    libc::close(fd);
                }
                Ok(())
            });
        }
        let monitor = command.spawn();
        close_fd(monitor_registration);
        close_fd(lifeline_read);
        let monitor = match monitor {
            Ok(child) => child,
            Err(error) => {
                for fd in [registration, token_read, token_write, lifeline_write] {
                    close_fd(fd);
                }
                return Err(error);
            }
        };
        Ok(Self {
            registration,
            token_read,
            token_write,
            lifeline_write,
            monitor,
            dedicated_uid,
        })
    }

    fn protocol(&self) -> NestedGroupProtocol {
        NestedGroupProtocol::new(self.registration, self.token_read, self.token_write)
    }

    fn registered_anchor(&self, pid: u32, timeout: Duration) -> Result<libc::pid_t, String> {
        if timeout.is_zero() {
            return Err("command deadline elapsed before process-group anchor query".into());
        }
        let pid = libc::pid_t::try_from(pid).map_err(|_| "child PID does not fit pid_t")?;
        self.protocol()
            .exchange_with_timeout(NestedGroupOperation::Query, pid, timeout)
            .map_err(|error| format!("cannot obtain process-group anchor for {pid}: {error}"))
    }

    fn unregister(&self, pid: u32) -> Result<(), String> {
        let pid = libc::pid_t::try_from(pid).map_err(|_| "child PID does not fit pid_t")?;
        self.protocol()
            .exchange(NestedGroupOperation::Unregister, pid)
            .map(|_| ())
            .map_err(|error| format!("cannot unregister supervised process group {pid}: {error}"))
    }

    fn finish(mut self, timeout: Duration) -> Result<(), String> {
        for fd in [
            self.lifeline_write,
            self.registration,
            self.token_read,
            self.token_write,
        ] {
            close_fd(fd);
        }
        let deadline = Instant::now()
            .checked_add(timeout)
            .ok_or_else(|| "containment monitor deadline overflow".to_string())?;
        loop {
            match self.monitor.try_wait() {
                Ok(Some(status)) if status.success() => return Ok(()),
                Ok(Some(status)) => {
                    return Err(format!("nested process-group monitor failed with {status}"));
                }
                Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(5)),
                Ok(None) => {
                    return Err(
                        "nested process-group monitor exceeded its cleanup bound; monitor retains cleanup ownership"
                            .into(),
                    );
                }
                Err(error) => {
                    return Err(format!(
                        "cannot wait for containment monitor; monitor retains cleanup ownership: {error}"
                    ));
                }
            }
        }
    }
}

#[cfg(unix)]
fn pipe_cloexec() -> Result<(RawFd, RawFd), std::io::Error> {
    let mut fds = [-1; 2];
    // SAFETY: fds points to two writable file-descriptor slots.
    if unsafe { libc::pipe(fds.as_mut_ptr()) } == -1 {
        return Err(std::io::Error::last_os_error());
    }
    for fd in fds {
        // SAFETY: fd was returned by pipe.
        if unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) } == -1 {
            let error = std::io::Error::last_os_error();
            close_fd(fds[0]);
            close_fd(fds[1]);
            return Err(error);
        }
    }
    Ok((fds[0], fds[1]))
}
#[cfg(unix)]
fn socket_pair_cloexec() -> Result<(RawFd, RawFd), std::io::Error> {
    let mut fds = [-1; 2];
    // SAFETY: fds points to two writable descriptor slots.
    if unsafe { libc::socketpair(libc::AF_UNIX, libc::SOCK_STREAM, 0, fds.as_mut_ptr()) } == -1 {
        return Err(std::io::Error::last_os_error());
    }
    for fd in fds {
        // SAFETY: fd was returned by socketpair.
        if unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) } == -1 {
            let error = std::io::Error::last_os_error();
            close_fd(fds[0]);
            close_fd(fds[1]);
            return Err(error);
        }
    }
    Ok((fds[0], fds[1]))
}

#[cfg(unix)]
fn close_fd(fd: RawFd) {
    // SAFETY: callers transfer ownership of each raw descriptor exactly once.
    unsafe {
        libc::close(fd);
    }
}

#[cfg(unix)]
fn close_fds(fds: &[RawFd]) {
    for &fd in fds {
        close_fd(fd);
    }
}

#[cfg(target_os = "macos")]
fn monitor_process_start(pid: i32) -> Option<u64> {
    // A registered leader can exit before the monitor answers its registration.
    // proc_pidinfo refuses such a process, which would leave the monitor unable
    // to capture an identity it already has; the kernel record still reports it.
    macos_process_record(pid).map(|record| record.start_micros)
}

#[cfg(target_os = "linux")]
fn monitor_process_start(pid: i32) -> Option<u64> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let after_comm = stat.rfind(')')?;
    stat[after_comm + 1..]
        .split_whitespace()
        .nth(19)?
        .parse()
        .ok()
}

#[cfg(unix)]
#[derive(Debug)]
struct MonitorAnchor {
    pid: libc::pid_t,
    command_write: RawFd,
    result_read: RawFd,
}

#[cfg(unix)]
fn wait_pid(pid: libc::pid_t) -> Result<(), String> {
    loop {
        // SAFETY: pid names a direct child owned by the monitor.
        let result = unsafe { libc::waitpid(pid, std::ptr::null_mut(), 0) };
        if result == pid {
            return Ok(());
        }
        if result == -1 && std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted
        {
            continue;
        }
        return Err(format!(
            "cannot reap containment anchor {pid}: {}",
            std::io::Error::last_os_error()
        ));
    }
}

#[cfg(unix)]
fn try_reap_pid(pid: libc::pid_t) -> Result<bool, String> {
    loop {
        // SAFETY: pid names a direct child owned by the monitor.
        let result = unsafe { libc::waitpid(pid, std::ptr::null_mut(), libc::WNOHANG) };
        match result {
            value if value == pid => return Ok(true),
            0 => return Ok(false),
            -1 if std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted => {}
            _ => {
                return Err(format!(
                    "cannot inspect containment anchor {pid}: {}",
                    std::io::Error::last_os_error()
                ));
            }
        }
    }
}

#[cfg(unix)]
fn anchor_child(
    leader: libc::pid_t,
    command_read: RawFd,
    result_write: RawFd,
    inherited: &[RawFd],
) -> ! {
    for &fd in inherited {
        close_fd(fd);
    }
    // SAFETY: the child runs only async-signal-safe operations after fork.
    unsafe {
        libc::signal(libc::SIGTERM, libc::SIG_IGN);
    }
    let ready = if unsafe { libc::setpgid(0, leader) } == 0 {
        0_i32
    } else {
        std::io::Error::last_os_error()
            .raw_os_error()
            .unwrap_or(libc::EINVAL)
    };
    let _ = fd_io(result_write, &mut ready.to_ne_bytes(), true);
    if ready != 0 {
        unsafe { libc::_exit(125) }
    }
    loop {
        let mut bytes = [0_u8; 4];
        if fd_io(command_read, &mut bytes, false).is_err() {
            unsafe { libc::_exit(126) }
        }
        let signal = i32::from_ne_bytes(bytes);
        if signal == 0 {
            unsafe { libc::_exit(0) }
        }
        let result = if unsafe { libc::kill(-libc::getpgrp(), signal) } == 0 {
            0_i32
        } else {
            std::io::Error::last_os_error()
                .raw_os_error()
                .unwrap_or(libc::EINVAL)
        };
        if signal == libc::SIGKILL {
            unsafe { libc::_exit(127) }
        }
        let _ = fd_io(result_write, &mut result.to_ne_bytes(), true);
    }
}

#[cfg(unix)]
impl MonitorAnchor {
    fn spawn(
        leader: libc::pid_t,
        leader_start: u64,
        registration_fd: RawFd,
        lifeline_fd: RawFd,
    ) -> Result<Self, String> {
        Self::spawn_checked(leader, leader_start, registration_fd, lifeline_fd, || {
            monitor_process_start(leader)
        })
    }

    fn spawn_checked<F>(
        leader: libc::pid_t,
        leader_start: u64,
        registration_fd: RawFd,
        lifeline_fd: RawFd,
        recheck: F,
    ) -> Result<Self, String>
    where
        F: FnOnce() -> Option<u64>,
    {
        let (command_read, command_write) = pipe_cloexec().map_err(|error| error.to_string())?;
        let (result_read, result_write) = match pipe_cloexec() {
            Ok(pipe) => pipe,
            Err(error) => {
                close_fds(&[command_read, command_write]);
                return Err(error.to_string());
            }
        };
        // SAFETY: the monitor is single-threaded; the child immediately enters
        // anchor_child and uses only async-signal-safe operations.
        let pid = unsafe { libc::fork() };
        if pid == 0 {
            close_fd(command_write);
            close_fd(result_read);
            anchor_child(
                leader,
                command_read,
                result_write,
                &[registration_fd, lifeline_fd],
            );
        }
        close_fd(command_read);
        close_fd(result_write);
        if pid == -1 {
            close_fd(command_write);
            close_fd(result_read);
            return Err(format!(
                "cannot fork containment anchor: {}",
                std::io::Error::last_os_error()
            ));
        }
        let mut ready = [0_u8; 4];
        if let Err(error) = fd_io(result_read, &mut ready, false) {
            close_fd(command_write);
            close_fd(result_read);
            let _ = wait_pid(pid);
            return Err(format!("containment anchor did not initialize: {error}"));
        }
        let ready_errno = i32::from_ne_bytes(ready);
        if ready_errno != 0 {
            close_fd(command_write);
            close_fd(result_read);
            wait_pid(pid)?;
            return Err(format!(
                "containment anchor could not join process group {leader}: {}",
                std::io::Error::from_raw_os_error(ready_errno)
            ));
        }
        if recheck() != Some(leader_start) {
            let _ = fd_io(command_write, &mut 0_i32.to_ne_bytes(), true);
            close_fd(command_write);
            close_fd(result_read);
            wait_pid(pid)?;
            return Err(format!(
                "registered process-group leader {leader} changed identity during anchoring"
            ));
        }
        Ok(Self {
            pid,
            command_write,
            result_read,
        })
    }

    fn signal_group(&self, signal: i32) -> Result<(), String> {
        fd_io(self.command_write, &mut signal.to_ne_bytes(), true)
            .map_err(|error| format!("cannot command containment anchor {}: {error}", self.pid))?;
        if signal == libc::SIGKILL {
            return Ok(());
        }
        let mut result = [0_u8; 4];
        fd_io(self.result_read, &mut result, false)
            .map_err(|error| format!("containment anchor {} did not reply: {error}", self.pid))?;
        let errno = i32::from_ne_bytes(result);
        if errno == 0 {
            Ok(())
        } else {
            Err(format!(
                "containment anchor {} could not signal its group: {}",
                self.pid,
                std::io::Error::from_raw_os_error(errno)
            ))
        }
    }

    fn release(self) -> Result<(), String> {
        let result = if try_reap_pid(self.pid)? {
            Ok(())
        } else {
            let _ = fd_io(self.command_write, &mut 0_i32.to_ne_bytes(), true);
            wait_pid(self.pid)
        };
        close_fd(self.command_write);
        close_fd(self.result_read);
        result
    }

    fn kill_and_reap(self) -> Result<(), String> {
        let signal_result = self.signal_group(libc::SIGKILL);
        let wait_result = wait_pid(self.pid);
        close_fd(self.command_write);
        close_fd(self.result_read);
        signal_result.and(wait_result)
    }
}

#[cfg(unix)]
fn monitor_reply(
    fd: RawFd,
    status: i32,
    pid: libc::pid_t,
    anchor: libc::pid_t,
) -> Result<(), String> {
    write_nested_group_response(fd, status, pid, anchor)
        .map_err(|error| format!("cannot reply to registration: {error}"))
}

#[cfg(unix)]
fn monitor_request(
    fd: RawFd,
    lifeline_fd: RawFd,
    registrations: &mut BTreeMap<i32, MonitorAnchor>,
) -> Result<(), String> {
    let request = read_nested_group_request(fd).map_err(|error| error.to_string())?;
    let operation = request.operation;
    let pid = request.pid;
    if pid <= 0 {
        monitor_reply(fd, -1, pid, 0)?;
        return Err("nested process-group registration has nonpositive PID".into());
    }
    let result = match operation {
        NestedGroupOperation::Register => {
            let start = monitor_process_start(pid).ok_or_else(|| {
                format!("cannot capture registered process-group leader identity for {pid}")
            })?;
            match registrations.entry(pid) {
                std::collections::btree_map::Entry::Vacant(entry) => {
                    MonitorAnchor::spawn(pid, start, fd, lifeline_fd).map(|anchor| {
                        let anchor_pid = anchor.pid;
                        entry.insert(anchor);
                        anchor_pid
                    })
                }
                std::collections::btree_map::Entry::Occupied(_) => Err(format!(
                    "duplicate nested process-group registration for {pid}"
                )),
            }
        }
        NestedGroupOperation::Query => registrations
            .get(&pid)
            .map(|anchor| anchor.pid)
            .ok_or_else(|| format!("unknown nested process-group query for {pid}")),
        NestedGroupOperation::Unregister => registrations
            .remove(&pid)
            .ok_or_else(|| format!("unknown nested process-group unregister for {pid}"))
            .and_then(|anchor| anchor.release().map(|()| 0)),
    };
    match result {
        Ok(anchor_pid) => monitor_reply(fd, 0, pid, anchor_pid),
        Err(error) => {
            monitor_reply(fd, -1, pid, 0)?;
            Err(error)
        }
    }
}

#[cfg(unix)]
fn monitor_cleanup_registered(
    registrations: BTreeMap<i32, MonitorAnchor>,
    grace: Duration,
    dedicated_uid: Option<&str>,
) -> Result<(), String> {
    let mut first_error = None;
    for anchor in registrations.values() {
        if let Err(error) = anchor.signal_group(libc::SIGTERM) {
            first_error.get_or_insert(error);
        }
    }
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    if let Some(uid) = dedicated_uid {
        if let Err(error) = privileged_uid_cleanup(uid, libc::SIGTERM) {
            first_error.get_or_insert(error);
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    let _ = dedicated_uid;
    thread::sleep(grace);
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    if let Some(uid) = dedicated_uid {
        if let Err(error) = privileged_uid_cleanup(uid, libc::SIGKILL) {
            first_error.get_or_insert(error);
        }
    }
    for (_, anchor) in registrations {
        if let Err(error) = anchor.kill_and_reap() {
            first_error.get_or_insert(error);
        }
    }
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    if let Some(uid) = dedicated_uid {
        if let Err(error) = wait_for_privileged_uid_empty(uid, PRIVILEGED_CLEANUP_TIMEOUT) {
            first_error.get_or_insert(error);
        }
    }
    match first_error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn monitor_dedicated_uid_from_env() -> Result<Option<String>, String> {
    let Some(value) = std::env::var_os(MONITOR_DEDICATED_UID_ENV) else {
        return Ok(None);
    };
    let uid = value
        .into_string()
        .map_err(|_| "containment monitor dedicated uid is not UTF-8".to_string())?;
    let numeric = uid
        .parse::<u32>()
        .map_err(|_| "containment monitor dedicated uid must be numeric".to_string())?;
    if !(501..=60_000).contains(&numeric) || numeric == unsafe { libc::geteuid() } {
        return Err("containment monitor dedicated uid is outside the authorized range".into());
    }
    Ok(Some(uid))
}

#[cfg(unix)]
pub fn run_containment_monitor_from_env() -> Result<(), String> {
    // SAFETY: the monitor handles closed anchor command pipes as I/O errors.
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_IGN);
    }
    let descriptor = |name: &str| {
        std::env::var(name)
            .map_err(|_| format!("containment monitor lacks {name}"))?
            .parse::<RawFd>()
            .map_err(|_| format!("containment monitor has invalid {name}"))
    };
    let registration_fd = descriptor(MONITOR_REGISTRATION_FD_ENV)?;
    let lifeline_fd = descriptor(MONITOR_LIFELINE_FD_ENV)?;
    let grace = Duration::from_millis(
        std::env::var(MONITOR_GRACE_MS_ENV)
            .map_err(|_| "containment monitor lacks grace period".to_string())?
            .parse::<u64>()
            .map_err(|_| "containment monitor has invalid grace period".to_string())?,
    );
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    let dedicated_uid = monitor_dedicated_uid_from_env()?;
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    let dedicated_uid: Option<String> = None;
    let mut registrations = BTreeMap::new();
    let monitor_result = loop {
        let mut descriptors = [
            libc::pollfd {
                fd: registration_fd,
                events: libc::POLLIN | libc::POLLHUP,
                revents: 0,
            },
            libc::pollfd {
                fd: lifeline_fd,
                events: libc::POLLIN | libc::POLLHUP,
                revents: 0,
            },
        ];
        // SAFETY: descriptors is valid writable pollfd storage.
        if unsafe { libc::poll(descriptors.as_mut_ptr(), 2, -1) } == -1 {
            let error = std::io::Error::last_os_error();
            if error.kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            break Err(format!("containment monitor poll failed: {error}"));
        }
        if descriptors[1].revents & (libc::POLLIN | libc::POLLHUP) != 0 {
            let mut byte = [0_u8; 1];
            // SAFETY: byte is writable and lifeline_fd is the monitor pipe.
            let read = unsafe { libc::read(lifeline_fd, byte.as_mut_ptr().cast(), 1) };
            if read == 0 {
                break Ok(());
            }
            if read == -1 {
                let error = std::io::Error::last_os_error();
                if error.kind() != std::io::ErrorKind::Interrupted {
                    break Err(format!("containment lifeline read failed: {error}"));
                }
            }
        }
        if descriptors[0].revents & libc::POLLIN != 0 {
            if let Err(error) = monitor_request(registration_fd, lifeline_fd, &mut registrations) {
                break Err(error);
            }
        }
    };
    let cleanup_result = monitor_cleanup_registered(registrations, grace, dedicated_uid.as_deref());
    match (monitor_result, cleanup_result) {
        (Err(error), _) => Err(error),
        (Ok(()), result) => result,
    }
}

fn select_termination_reason(
    current: &'static str,
    child_running: bool,
    deadline_reached: bool,
    output_limited: bool,
    has_error: bool,
    descendants: bool,
) -> (&'static str, bool) {
    if current != "none" {
        (current, false)
    } else if child_running && deadline_reached {
        ("timeout", true)
    } else if output_limited {
        ("output-limit", false)
    } else if has_error {
        ("supervision-error", false)
    } else if descendants {
        ("descendant-cleanup", false)
    } else {
        ("none", false)
    }
}

fn terminal_identity(status: Option<ExitStatus>) -> (Option<i32>, Option<i32>) {
    let Some(status) = status else {
        return (None, None);
    };
    #[cfg(unix)]
    let signal = {
        use std::os::unix::process::ExitStatusExt as _;
        status.signal()
    };
    #[cfg(not(unix))]
    let signal = None;
    (status.code(), signal)
}

fn launch_failure(program: &str, error: &std::io::Error, limits: Limits) -> Captured {
    Captured {
        limits,
        stdout: Vec::new(),
        stderr: Vec::new(),
        stdout_bytes_seen: 0,
        stderr_bytes_seen: 0,
        stdout_truncated: false,
        stderr_truncated: false,
        executable_identity: None,
        exit_code: None,
        signal: None,
        launch_error: Some(format!("cannot execute {program}: {error}")),
        timed_out: false,
        termination_reason: "none",
        termination_signal: "none",
        process_group_cleanup: "not-started",
        pipe_cleanup: "not-started",
        supervision_error: None,
        descendant_survivors: None,
    }
}

struct LaunchError {
    label: String,
    error: std::io::Error,
}

struct Launch {
    child: std::process::Child,
    receiver: Receiver<StreamEvent>,
    #[cfg(unix)]
    anchor: LeaderAnchor,
    #[cfg(unix)]
    containment: Containment,
}

struct Probe {
    deadline: Instant,
    capture: CaptureState,
    status: Option<ExitStatus>,
    timed_out: bool,
    termination_reason: &'static str,
    termination_signal: &'static str,
    termination_started: Option<Instant>,
    pipe_deadline: Option<Instant>,
    supervision_error: Option<String>,
    descendant_survivors: Option<String>,
}

impl Probe {
    #[cfg(test)]
    fn new(limits: Limits, timeout: Duration) -> Self {
        let now = Instant::now();
        match now.checked_add(timeout) {
            Some(deadline) => Self::with_deadline(limits, deadline),
            None => {
                let mut probe = Self::with_deadline(limits, now);
                probe.supervision_error =
                    Some("subprocess supervision deadline overflowed".to_string());
                probe
            }
        }
    }

    fn with_deadline(limits: Limits, deadline: Instant) -> Self {
        Self {
            deadline,
            capture: CaptureState::new(limits),
            status: None,
            timed_out: false,
            termination_reason: "none",
            termination_signal: "none",
            termination_started: None,
            pipe_deadline: None,
            supervision_error: None,
            descendant_survivors: None,
        }
    }
}

struct SupervisionOutcome {
    probe: Probe,
    process_group_cleanup: &'static str,
    pipe_cleanup: &'static str,
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn permit_containment_directory_if(path: &Path, configured: bool) -> Result<(), String> {
    if !configured {
        return Ok(());
    }
    use std::os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _};
    let directory = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
        .map_err(|error| {
            format!(
                "cannot open dedicated-UID containment directory {} without following links: {error}",
                path.display()
            )
        })?;
    directory
        .set_permissions(std::fs::Permissions::from_mode(0o770))
        .map_err(|error| {
            format!(
                "cannot make dedicated-UID containment directory {} group-writable: {error}",
                path.display()
            )
        })
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub fn permit_containment_directory(path: &Path) -> Result<(), String> {
    permit_containment_directory_if(
        path,
        std::env::var_os(DEDICATED_CONTAINMENT_UID_ENV).is_some(),
    )
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn permit_containment_directory(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[derive(Debug, PartialEq, Eq)]
struct UidContainmentConfig {
    uid: String,
    home: String,
    user: String,
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn containment_environment_allows(name: &str) -> bool {
    matches!(
        name,
        "PATH"
            | "TMPDIR"
            | "LANG"
            | "TERM"
            | "HOME"
            | "USER"
            | "LOGNAME"
            | "SDKROOT"
            | "DEVELOPER_DIR"
            | "MACOSX_DEPLOYMENT_TARGET"
            | "PKG_CONFIG_PATH"
            | "PKG_CONFIG_LIBDIR"
            | "PKG_CONFIG_SYSROOT_DIR"
            | "LIBRARY_PATH"
            | "CPATH"
            | "C_INCLUDE_PATH"
            | "CPLUS_INCLUDE_PATH"
            | "CC"
            | "CXX"
            | "AR"
            | "NM"
            | "HTTP_PROXY"
            | "HTTPS_PROXY"
            | "ALL_PROXY"
            | "NO_PROXY"
            | "http_proxy"
            | "https_proxy"
            | "all_proxy"
            | "no_proxy"
            | "SSL_CERT_FILE"
            | "SSL_CERT_DIR"
            | "CARGO_HOME"
            | "CARGO_TARGET_DIR"
            | "RUSTUP_HOME"
            | "RUSTUP_TOOLCHAIN"
    ) || name.starts_with("LC_")
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn filter_inherited_environment(
    variables: impl IntoIterator<Item = (std::ffi::OsString, std::ffi::OsString)>,
    allows: impl Fn(&str) -> bool,
) -> Result<BTreeMap<String, String>, std::io::Error> {
    let mut environment = BTreeMap::new();
    for (name, value) in variables {
        let name = name.into_string().map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "cannot transfer a non-UTF-8 environment name into a bounded child",
            )
        })?;
        let value = value.into_string().map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("cannot transfer non-UTF-8 environment variable {name}"),
            )
        })?;
        if allows(&name) {
            environment.insert(name, value);
        }
    }
    Ok(environment)
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn bind_dedicated_containment_environment(
    environment: &mut BTreeMap<String, String>,
    extra_env: &[(String, String)],
    config: &UidContainmentConfig,
) {
    for (name, value) in extra_env {
        environment.insert(name.clone(), value.clone());
    }
    environment.remove(DEDICATED_CONTAINMENT_UID_ENV);
    environment.remove(DEDICATED_CONTAINMENT_HOME_ENV);
    environment.remove(DEDICATED_CONTAINMENT_USER_ENV);
    environment.insert("HOME".into(), config.home.clone());
    environment.insert("TMPDIR".into(), config.home.clone());
    environment.insert("USER".into(), config.user.clone());
    environment.insert("LOGNAME".into(), config.user.clone());
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn build_dedicated_containment_command(
    config: &UidContainmentConfig,
    environment: BTreeMap<String, String>,
    program: &str,
    arguments: &[String],
) -> Command {
    let mut command = Command::new("/usr/bin/sudo");
    command
        .env_clear()
        .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin");
    command.args([
        "-n",
        "/bin/bash",
        "-c",
        DEDICATED_UID_WRAPPER,
        "uqm-dedicated-containment",
        &config.uid,
        &environment.len().to_string(),
    ]);
    for (name, value) in environment {
        command.arg(format!("{name}={value}"));
    }
    command.arg(program).args(arguments);
    command
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn dedicated_containment_config() -> Result<Option<UidContainmentConfig>, std::io::Error> {
    parse_dedicated_containment_config(
        std::env::var_os(DEDICATED_CONTAINMENT_UID_ENV),
        std::env::var(DEDICATED_CONTAINMENT_HOME_ENV),
        std::env::var(DEDICATED_CONTAINMENT_USER_ENV),
        unsafe { libc::geteuid() },
    )
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn dedicated_contained_command(
    program: &str,
    arguments: &[String],
    extra_env: &[(String, String)],
) -> Result<Command, std::io::Error> {
    let config = dedicated_containment_config()?;
    let Some(config) = config else {
        let mut command = Command::new(program);
        command.args(arguments);
        return Ok(command);
    };

    let mut environment =
        filter_inherited_environment(std::env::vars_os(), containment_environment_allows)?;
    bind_dedicated_containment_environment(&mut environment, extra_env, &config);
    Ok(build_dedicated_containment_command(
        &config,
        environment,
        program,
        arguments,
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LaunchContext {
    DedicatedContainment,
    CurrentAquaSession,
}

#[cfg(target_os = "macos")]
fn launchd_manager_value(subcommand: &str) -> Result<String, std::io::Error> {
    let output = Command::new("/bin/launchctl")
        .arg(subcommand)
        .env_clear()
        .output()?;
    if !output.status.success() {
        return Err(std::io::Error::other(format!(
            "launchctl {subcommand} failed with status {}",
            output.status
        )));
    }
    std::str::from_utf8(&output.stdout)
        .map(str::trim)
        .map(str::to_string)
        .map_err(std::io::Error::other)
}

#[cfg(target_os = "macos")]
fn current_aqua_session() -> Result<(), std::io::Error> {
    // SAFETY: getuid and geteuid have no preconditions.
    let real_uid = unsafe { libc::getuid() };
    let effective_uid = unsafe { libc::geteuid() };
    let manager_uid = launchd_manager_value("manageruid")?
        .parse::<u32>()
        .map_err(std::io::Error::other)?;
    let manager_name = launchd_manager_value("managername")?;
    if real_uid == 0
        || real_uid != effective_uid
        || manager_uid != real_uid
        || manager_name != "Aqua"
    {
        return Err(std::io::Error::other(format!(
            "native acceptance requires matching non-root real/effective/Aqua manager UIDs; real={real_uid}, effective={effective_uid}, manager={manager_uid}, manager_name={manager_name:?}"
        )));
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn current_aqua_environment_allows(name: &str) -> bool {
    containment_environment_allows(name)
        || matches!(
            name,
            DEDICATED_CONTAINMENT_UID_ENV
                | DEDICATED_CONTAINMENT_HOME_ENV
                | DEDICATED_CONTAINMENT_USER_ENV
        )
}

#[cfg(target_os = "macos")]
fn current_aqua_command(
    program: &str,
    arguments: &[String],
    extra_env: &[(String, String)],
) -> Result<Command, std::io::Error> {
    let mut environment =
        filter_inherited_environment(std::env::vars_os(), current_aqua_environment_allows)?;
    for (name, value) in extra_env {
        if matches!(
            name.as_str(),
            DEDICATED_CONTAINMENT_UID_ENV
                | DEDICATED_CONTAINMENT_HOME_ENV
                | DEDICATED_CONTAINMENT_USER_ENV
        ) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("current-Aqua child cannot override trusted containment binding {name}"),
            ));
        }
        environment.insert(name.clone(), value.clone());
    }
    let mut command = Command::new(program);
    command.args(arguments).env_clear().envs(environment);
    Ok(command)
}

#[cfg(unix)]
fn launch(
    working_directory: &Path,
    program: &str,
    arguments: &[String],
    extra_env: &[(String, String)],
    limits: Limits,
    deadline: Instant,
    context: LaunchContext,
) -> Result<Launch, LaunchError> {
    let mut command = match context {
        LaunchContext::DedicatedContainment => {
            #[cfg(any(target_os = "macos", target_os = "linux"))]
            {
                dedicated_contained_command(program, arguments, extra_env).map_err(|error| {
                    LaunchError {
                        label: "dedicated-uid containment".to_string(),
                        error,
                    }
                })?
            }
            #[cfg(not(any(target_os = "macos", target_os = "linux")))]
            {
                let mut command = Command::new(program);
                command.args(arguments);
                command
            }
        }
        LaunchContext::CurrentAquaSession => {
            #[cfg(target_os = "macos")]
            {
                current_aqua_session().map_err(|error| LaunchError {
                    label: "native Aqua session".to_string(),
                    error,
                })?;
                current_aqua_command(program, arguments, extra_env).map_err(|error| {
                    LaunchError {
                        label: "native Aqua environment".to_string(),
                        error,
                    }
                })?
            }
            #[cfg(not(target_os = "macos"))]
            return Err(LaunchError {
                label: "native Aqua session".to_string(),
                error: std::io::Error::other("native Aqua launch is supported only on macOS"),
            });
        }
    };
    let containment =
        Containment::start(limits.termination_grace).map_err(|error| LaunchError {
            label: "nested process-group monitor".to_string(),
            error,
        })?;
    command
        .current_dir(working_directory)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let registration_timeout = deadline.saturating_duration_since(Instant::now());
    if registration_timeout.is_zero() {
        let _ = containment.finish(limits.pipe_drain_timeout);
        return Err(LaunchError {
            label: program.to_string(),
            error: std::io::Error::from_raw_os_error(libc::ETIMEDOUT),
        });
    }
    configure_process_group(&mut command, containment.protocol(), registration_timeout);
    if context == LaunchContext::DedicatedContainment {
        // The dedicated-uid wrapper reconstructs this exact environment for the
        // unprivileged target. Setting it here also preserves ordinary launches.
        for (name, value) in extra_env {
            command.env(name, value);
        }
    }
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            let _ = containment.finish(limits.pipe_drain_timeout);
            return Err(LaunchError {
                label: program.to_string(),
                error,
            });
        }
    };
    let anchor = containment
        .registered_anchor(
            child.id(),
            deadline.saturating_duration_since(Instant::now()),
        )
        .and_then(|monitor_anchor_pid| {
            LeaderAnchor::new(
                child.id(),
                monitor_anchor_pid,
                containment.dedicated_uid.clone(),
            )
        });
    let anchor = match anchor {
        Ok(anchor) => anchor,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            let _ = containment.finish(limits.pipe_drain_timeout);
            return Err(LaunchError {
                label: program.to_string(),
                error: std::io::Error::other(error),
            });
        }
    };
    let stdout = match child.stdout.take() {
        Some(stdout) => stdout,
        None => {
            let _ = child.kill();
            let _ = child.wait();
            let _ = containment.finish(limits.pipe_drain_timeout);
            return Err(LaunchError {
                label: program.to_string(),
                error: std::io::Error::other("piped stdout is unavailable after launch"),
            });
        }
    };
    let stderr = match child.stderr.take() {
        Some(stderr) => stderr,
        None => {
            let _ = child.kill();
            let _ = child.wait();
            let _ = containment.finish(limits.pipe_drain_timeout);
            return Err(LaunchError {
                label: program.to_string(),
                error: std::io::Error::other("piped stderr is unavailable after launch"),
            });
        }
    };
    let (sender, receiver) = mpsc::sync_channel(8);
    let stdout_sender = sender.clone();
    thread::spawn(move || pump(stdout, Stream::Stdout, stdout_sender));
    thread::spawn(move || pump(stderr, Stream::Stderr, sender));
    Ok(Launch {
        child,
        receiver,
        anchor,
        containment,
    })
}

#[cfg(not(unix))]
fn launch(
    working_directory: &Path,
    program: &str,
    arguments: &[String],
    extra_env: &[(String, String)],
    limits: Limits,
    deadline: Instant,
    context: LaunchContext,
) -> Result<Launch, LaunchError> {
    let _ = (limits, context);
    let mut command = Command::new(program);
    command
        .current_dir(working_directory)
        .args(arguments)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    configure_process_group(
        &mut command,
        deadline.saturating_duration_since(Instant::now()),
    );
    for (name, value) in extra_env {
        command.env(name, value);
    }
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            return Err(LaunchError {
                label: program.to_string(),
                error,
            });
        }
    };
    let stdout = match child.stdout.take() {
        Some(stdout) => stdout,
        None => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(LaunchError {
                label: program.to_string(),
                error: std::io::Error::other("piped stdout is unavailable after launch"),
            });
        }
    };
    let stderr = match child.stderr.take() {
        Some(stderr) => stderr,
        None => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(LaunchError {
                label: program.to_string(),
                error: std::io::Error::other("piped stderr is unavailable after launch"),
            });
        }
    };
    let (sender, receiver) = mpsc::sync_channel(8);
    let stdout_sender = sender.clone();
    thread::spawn(move || pump(stdout, Stream::Stdout, stdout_sender));
    thread::spawn(move || pump(stderr, Stream::Stderr, sender));
    Ok(Launch { child, receiver })
}

fn capture_drive(probe: &mut Probe, receiver: &Receiver<StreamEvent>, limits: Limits) {
    match receiver.recv_timeout(Duration::from_millis(5)) {
        Ok(event) => probe.capture.accept(event, limits),
        Err(mpsc::RecvTimeoutError::Timeout) => {}
        Err(mpsc::RecvTimeoutError::Disconnected) if !probe.capture.pipes_finished() => {
            probe
                .supervision_error
                .get_or_insert_with(|| "capture workers disconnected before EOF".to_string());
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {}
    }
    probe.capture.drain(receiver, limits);
    if probe.supervision_error.is_none() {
        probe.supervision_error = probe.capture.error.clone();
    }
}

#[cfg(unix)]
fn observe_leader(probe: &mut Probe, anchor: &mut LeaderAnchor) {
    if let Err(error) = anchor.observe() {
        probe.supervision_error.get_or_insert(error);
    }
}

#[cfg(not(unix))]
fn observe_child(probe: &mut Probe, child: &mut std::process::Child) {
    if probe.status.is_none() {
        match child.try_wait() {
            Ok(observed) => probe.status = observed,
            Err(error) => {
                probe
                    .supervision_error
                    .get_or_insert_with(|| format!("cannot inspect child status: {error}"));
            }
        }
    }
}

#[cfg(unix)]
fn observed_group_clean(probe: &mut Probe, anchor: &LeaderAnchor) -> bool {
    match process_group_clean(anchor) {
        Ok(true) => true,
        Ok(false) => {
            // Name the exact process whose membership produced the verdict, so
            // a descendant that exits moments later is still identified.
            if probe.descendant_survivors.is_none() {
                if let Ok(Some(pid)) = group_occupant(anchor) {
                    probe.descendant_survivors = Some(describe_process(pid));
                }
            }
            false
        }
        Err(error) => {
            probe.supervision_error.get_or_insert(error);
            false
        }
    }
}

#[cfg(not(unix))]
fn observed_group_clean(probe: &mut Probe) -> bool {
    let status = probe.status;
    match process_group_clean(&status) {
        Ok(clean) => clean,
        Err(error) => {
            probe.supervision_error.get_or_insert(error);
            false
        }
    }
}

#[cfg(unix)]
fn apply_terminations(
    probe: &mut Probe,
    anchor: &mut LeaderAnchor,
    now: Instant,
    limits: Limits,
    child_running: bool,
    group_clean: bool,
) {
    if probe.termination_reason != "none" && probe.termination_started.is_none() {
        probe.termination_signal = "term";
        probe.termination_started = Some(now);
        if let Err(error) = terminate_owned(anchor, false) {
            probe.supervision_error.get_or_insert(error);
        }
    }
    let grace_expired = probe
        .termination_started
        .is_some_and(|instant| now.saturating_duration_since(instant) >= limits.termination_grace);
    if grace_expired && probe.termination_signal == "term" && (child_running || !group_clean) {
        probe.termination_signal = "kill";
        if let Err(error) = terminate_owned(anchor, true) {
            probe.supervision_error.get_or_insert(error);
        }
        probe.termination_started = Some(now);
    }
}

#[cfg(not(unix))]
fn apply_terminations(
    probe: &mut Probe,
    child: &mut std::process::Child,
    now: Instant,
    limits: Limits,
    child_running: bool,
    group_clean: bool,
) {
    if probe.termination_reason != "none" && probe.termination_started.is_none() {
        probe.termination_signal = "term";
        probe.termination_started = Some(now);
        if let Err(error) = terminate_owned(child, false, &probe.status) {
            probe.supervision_error.get_or_insert(error);
        }
    }
    let grace_expired = probe
        .termination_started
        .is_some_and(|instant| now.saturating_duration_since(instant) >= limits.termination_grace);
    if grace_expired && probe.termination_signal == "term" && (child_running || !group_clean) {
        probe.termination_signal = "kill";
        if let Err(error) = terminate_owned(child, true, &probe.status) {
            probe.supervision_error.get_or_insert(error);
        }
        probe.termination_started = Some(now);
    }
}

enum LoopStep {
    Continue,
    Finish(bool),
}

fn next_loop_step(
    probe: &mut Probe,
    now: Instant,
    child_running: bool,
    group_clean: bool,
    limits: Limits,
) -> LoopStep {
    if !child_running && group_clean && probe.pipe_deadline.is_none() {
        probe.pipe_deadline = now.checked_add(limits.pipe_drain_timeout);
        if probe.pipe_deadline.is_none() {
            probe
                .supervision_error
                .get_or_insert_with(|| "subprocess pipe-drain deadline overflowed".to_string());
        }
    }
    if !child_running && group_clean && probe.capture.pipes_finished() {
        return LoopStep::Finish(group_clean);
    }
    if probe.pipe_deadline.is_some_and(|deadline| now >= deadline) {
        probe.supervision_error.get_or_insert_with(|| {
            "captured pipes did not close after process-group cleanup".to_string()
        });
        return LoopStep::Finish(group_clean);
    }
    if probe.termination_signal == "kill"
        && probe.termination_started.is_some_and(|instant| {
            now.saturating_duration_since(instant) >= limits.pipe_drain_timeout
        })
    {
        probe.supervision_error.get_or_insert_with(|| {
            "owned process group remained after SIGKILL cleanup".to_string()
        });
        return LoopStep::Finish(group_clean);
    }
    LoopStep::Continue
}

#[cfg(unix)]
fn supervise(launch: Launch, limits: Limits, deadline: Instant) -> SupervisionOutcome {
    let Launch {
        child,
        mut anchor,
        containment,
        receiver,
    } = launch;
    let mut probe = Probe::with_deadline(limits, deadline);
    let final_group_clean = loop {
        capture_drive(&mut probe, &receiver, limits);
        observe_leader(&mut probe, &mut anchor);
        let now = Instant::now();
        let child_running = !anchor.observed;
        let group_clean = observed_group_clean(&mut probe, &anchor);
        let descendant_cleanup_required =
            anchor.descendant_cleanup_required(now, group_clean, limits.termination_grace);
        let (reason, timeout_triggered) = select_termination_reason(
            probe.termination_reason,
            child_running,
            now >= probe.deadline,
            probe.capture.output_limited(),
            probe.supervision_error.is_some(),
            descendant_cleanup_required,
        );
        probe.termination_reason = reason;
        probe.timed_out |= timeout_triggered;
        apply_terminations(
            &mut probe,
            &mut anchor,
            now,
            limits,
            child_running,
            group_clean,
        );
        match next_loop_step(&mut probe, now, child_running, group_clean, limits) {
            LoopStep::Continue => {}
            LoopStep::Finish(group_clean) => break group_clean,
        }
    };
    finalize_unix(
        child,
        anchor,
        containment,
        &receiver,
        probe,
        final_group_clean,
        limits,
    )
}

#[cfg(not(unix))]
fn supervise(launch: Launch, limits: Limits, deadline: Instant) -> SupervisionOutcome {
    let Launch {
        mut child,
        receiver,
    } = launch;
    let mut probe = Probe::with_deadline(limits, deadline);
    loop {
        capture_drive(&mut probe, &receiver, limits);
        observe_child(&mut probe, &mut child);
        let now = Instant::now();
        let child_running = probe.status.is_none();
        let group_clean = observed_group_clean(&mut probe);
        let (reason, timeout_triggered) = select_termination_reason(
            probe.termination_reason,
            child_running,
            now >= probe.deadline,
            probe.capture.output_limited(),
            probe.supervision_error.is_some(),
            !child_running && !group_clean,
        );
        probe.termination_reason = reason;
        probe.timed_out |= timeout_triggered;
        apply_terminations(
            &mut probe,
            &mut child,
            now,
            limits,
            child_running,
            group_clean,
        );
        match next_loop_step(&mut probe, now, child_running, group_clean, limits) {
            LoopStep::Continue => {}
            LoopStep::Finish(_) => break,
        }
    }
    finalize_non_unix(child, &receiver, probe, limits)
}

#[cfg(unix)]
fn finalize_unix(
    mut child: std::process::Child,
    mut anchor: LeaderAnchor,
    containment: Containment,
    receiver: &Receiver<StreamEvent>,
    mut probe: Probe,
    final_group_clean: bool,
    limits: Limits,
) -> SupervisionOutcome {
    if !anchor.observed {
        probe.supervision_error.get_or_insert_with(|| {
            "exact child was not observed before final reap boundary".to_string()
        });
    } else {
        if final_group_clean {
            if let Err(error) = containment.unregister(child.id()) {
                probe.supervision_error.get_or_insert(error);
            }
        }
        match child.wait() {
            Ok(observed) => {
                probe.status = Some(observed);
                if let Err(error) = anchor.mark_reaped() {
                    probe.supervision_error.get_or_insert(error);
                }
            }
            Err(error) => {
                probe
                    .supervision_error
                    .get_or_insert_with(|| format!("cannot reap child: {error}"));
            }
        }
    }
    probe.capture.drain(receiver, limits);
    // The monitor has already finished its own grace-bounded cleanup by this
    // point, so this bound only covers its exit. A loaded host can delay that
    // exit well past one drain interval, so allow several before declaring the
    // monitor hung.
    if let Err(error) = containment.finish(
        limits
            .termination_grace
            .saturating_mul(2)
            .saturating_add(limits.pipe_drain_timeout.saturating_mul(10)),
    ) {
        probe.supervision_error.get_or_insert(error);
    }
    let process_group_cleanup = if final_group_clean {
        "verified-empty"
    } else {
        let occupant = group_occupant(&anchor)
            .ok()
            .flatten()
            .map(describe_process)
            .or_else(|| probe.descendant_survivors.clone());
        probe
            .supervision_error
            .get_or_insert_with(|| match occupant {
                Some(occupant) => format!(
                    "owned process group was not empty before the leader reap boundary: {occupant}"
                ),
                None => {
                    "owned process group was not empty before the leader reap boundary".to_string()
                }
            });
        "failed"
    };
    let pipe_cleanup = if probe.capture.pipes_finished() {
        "complete"
    } else {
        "timed-out"
    };
    SupervisionOutcome {
        probe,
        process_group_cleanup,
        pipe_cleanup,
    }
}

#[cfg(not(unix))]
fn finalize_non_unix(
    mut child: std::process::Child,
    receiver: &Receiver<StreamEvent>,
    mut probe: Probe,
    limits: Limits,
) -> SupervisionOutcome {
    if probe.status.is_none() {
        match child.wait() {
            Ok(observed) => probe.status = Some(observed),
            Err(error) => {
                probe
                    .supervision_error
                    .get_or_insert_with(|| format!("cannot reap child: {error}"));
            }
        }
    }
    probe.capture.drain(receiver, limits);
    let pipe_cleanup = if probe.capture.pipes_finished() {
        "complete"
    } else {
        "timed-out"
    };
    SupervisionOutcome {
        probe,
        process_group_cleanup: "not-supported",
        pipe_cleanup,
    }
}

fn receipt(limits: Limits, outcome: SupervisionOutcome) -> Captured {
    let probe = outcome.probe;
    let (exit_code, signal) = terminal_identity(probe.status);
    Captured {
        limits,
        stdout: probe.capture.stdout,
        stderr: probe.capture.stderr,
        stdout_bytes_seen: probe.capture.stdout_bytes_seen,
        stderr_bytes_seen: probe.capture.stderr_bytes_seen,
        stdout_truncated: probe.capture.stdout_truncated,
        stderr_truncated: probe.capture.stderr_truncated,
        executable_identity: None,
        exit_code,
        signal,
        launch_error: None,
        timed_out: probe.timed_out,
        termination_reason: probe.termination_reason,
        termination_signal: probe.termination_signal,
        process_group_cleanup: outcome.process_group_cleanup,
        pipe_cleanup: outcome.pipe_cleanup,
        supervision_error: probe.supervision_error,
        descendant_survivors: probe.descendant_survivors,
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub fn run_containment_escape_helper(arguments: &[String]) -> Result<(), String> {
    let [sentinel] = arguments else {
        return Err("containment escape helper requires one sentinel path".into());
    };
    // SAFETY: this hidden helper is single-threaded at this point. Each forked
    // branch performs only async-signal-safe operations before exec-independent
    // Rust work in the final grandchild.
    let child = unsafe { libc::fork() };
    if child < 0 {
        return Err(format!(
            "first fork failed: {}",
            std::io::Error::last_os_error()
        ));
    }
    if child > 0 {
        let mut status = 0;
        // SAFETY: child is the exact positive PID returned by fork above.
        if unsafe { libc::waitpid(child, &mut status, 0) } != child || !libc::WIFEXITED(status) {
            return Err("cannot reap containment escape helper child".into());
        }
        return Ok(());
    }
    // SAFETY: this is the first single-threaded child and immediate errors use
    // _exit, avoiding inherited Rust teardown after fork.
    if unsafe { libc::setsid() } < 0 {
        unsafe { libc::_exit(2) };
    }
    let grandchild = unsafe { libc::fork() };
    if grandchild < 0 {
        unsafe { libc::_exit(3) };
    }
    if grandchild > 0 {
        unsafe { libc::_exit(0) };
    }
    thread::sleep(Duration::from_millis(500));
    let status = if std::fs::write(sentinel, b"escaped\n").is_ok() {
        0
    } else {
        4
    };
    unsafe { libc::_exit(status) };
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn stage_containment_check_executable(
    home: &Path,
    executable: &Path,
) -> Result<(std::path::PathBuf, std::path::PathBuf), String> {
    use std::os::unix::fs::PermissionsExt as _;

    let directory = home.join("containment-check-helper");
    if directory.exists() {
        std::fs::remove_dir_all(&directory)
            .map_err(|error| format!("cannot clear {}: {error}", directory.display()))?;
    }
    std::fs::create_dir(&directory)
        .map_err(|error| format!("cannot create {}: {error}", directory.display()))?;
    let staged = directory.join("uqm-xtask");
    std::fs::copy(executable, &staged).map_err(|error| {
        format!(
            "cannot stage containment-check executable at {}: {error}",
            staged.display()
        )
    })?;
    std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o550)).map_err(|error| {
        format!(
            "cannot authorize containment-check executable {}: {error}",
            staged.display()
        )
    })?;
    std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o750)).map_err(
        |error| {
            format!(
                "cannot lock containment-check executable directory {}: {error}",
                directory.display()
            )
        },
    )?;
    Ok((directory, staged))
}

pub fn verify_uid_containment(root: &Path) -> Result<(), String> {
    // SAFETY: geteuid has no preconditions.
    let controller_uid = unsafe { libc::geteuid() };
    let config = parse_dedicated_containment_config(
        std::env::var_os(DEDICATED_CONTAINMENT_UID_ENV),
        std::env::var(DEDICATED_CONTAINMENT_HOME_ENV),
        std::env::var(DEDICATED_CONTAINMENT_USER_ENV),
        controller_uid,
    )
    .map_err(|error| error.to_string())?
    .ok_or_else(|| {
        format!(
            "{DEDICATED_CONTAINMENT_UID_ENV} is required for merge-deciding macOS or Linux execution"
        )
    })?;
    use std::os::unix::fs::PermissionsExt as _;
    let directory = Path::new(&config.home).join("darwin-containment-check");
    if directory.exists() {
        std::fs::remove_dir_all(&directory)
            .map_err(|error| format!("cannot clear {}: {error}", directory.display()))?;
    }
    let timeout_probe = run_captured_with_limits(
        root,
        "/bin/sleep",
        &["30".to_string()],
        &[],
        Limits {
            timeout: Duration::from_millis(250),
            termination_grace: Duration::from_millis(250),
            pipe_drain_timeout: Duration::from_secs(2),
            stdout_bytes: 16 * 1024,
            stderr_bytes: 16 * 1024,
            executable_bytes: 268_435_456,
        },
    );
    if timeout_probe.termination_reason != "timeout"
        || timeout_probe.process_group_cleanup != "verified-empty"
        || timeout_probe.supervision_error.is_some()
    {
        return Err(format!(
            "dedicated-uid timeout cleanup was not causal and complete: reason={}, group_cleanup={}, supervision_error={:?}",
            timeout_probe.termination_reason,
            timeout_probe.process_group_cleanup,
            timeout_probe.supervision_error
        ));
    }

    std::fs::create_dir_all(&directory)
        .map_err(|error| format!("cannot create {}: {error}", directory.display()))?;
    std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o770))
        .map_err(|error| format!("cannot authorize {}: {error}", directory.display()))?;
    let sentinel = directory.join("escaped-write.txt");
    let executable = std::env::current_exe()
        .map_err(|error| format!("cannot locate containment-check executable: {error}"))?;
    let (helper_directory, helper_executable) =
        stage_containment_check_executable(Path::new(&config.home), &executable)?;
    let containment_result = (|| -> Result<(), String> {
        let arguments = vec![
            CONTAINMENT_ESCAPE_HELPER_COMMAND.to_string(),
            sentinel.to_string_lossy().into_owned(),
        ];
        let captured = run_captured_with_bound_environment(
            root,
            &helper_executable.to_string_lossy(),
            &arguments,
            Limits {
                timeout: CONTAINMENT_ESCAPE_PROBE_TIMEOUT,
                termination_grace: Duration::from_secs(1),
                pipe_drain_timeout: Duration::from_secs(2),
                stdout_bytes: 16 * 1024,
                stderr_bytes: 16 * 1024,
                executable_bytes: 268_435_456,
            },
            true,
            |_| Ok(Vec::new()),
        );
        if !captured.succeeded() {
            return Err(captured.failure_detail("dedicated-uid containment check"));
        }
        thread::sleep(Duration::from_millis(700));
        if sentinel.exists() {
            return Err(
                "a pre-observation detached grandchild escaped dedicated-uid cleanup".into(),
            );
        }

        Ok(())
    })();

    let helper_cleanup = std::fs::remove_dir_all(&helper_directory)
        .map_err(|error| format!("cannot remove {}: {error}", helper_directory.display()));
    let directory_cleanup = std::fs::remove_dir_all(&directory)
        .map_err(|error| format!("cannot remove {}: {error}", directory.display()));
    containment_result?;
    helper_cleanup?;
    directory_cleanup?;
    Ok(())
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn verify_uid_containment(_root: &Path) -> Result<(), String> {
    Ok(())
}
/// Run a command with bounded streaming capture and an authority-selected deadline.
pub fn run_captured_with_limits(
    working_directory: &Path,
    program: &str,
    arguments: &[String],
    extra_env: &[(String, String)],
    limits: Limits,
) -> Captured {
    run_captured_with_bound_environment(
        working_directory,
        program,
        arguments,
        limits,
        false,
        |_| Ok(extra_env.to_vec()),
    )
}

pub fn run_captured_with_bound_environment<F>(
    working_directory: &Path,
    program: &str,
    arguments: &[String],
    limits: Limits,
    execute_retained_source: bool,
    environment: F,
) -> Captured
where
    F: FnOnce(&str) -> Result<Vec<(String, String)>, String>,
{
    run_captured_with_launch_context(
        working_directory,
        program,
        arguments,
        limits,
        execute_retained_source,
        environment,
        LaunchContext::DedicatedContainment,
    )
}

pub fn run_captured_in_current_aqua_session<F>(
    working_directory: &Path,
    program: &str,
    arguments: &[String],
    limits: Limits,
    execute_retained_source: bool,
    environment: F,
) -> Captured
where
    F: FnOnce(&str) -> Result<Vec<(String, String)>, String>,
{
    run_captured_with_launch_context(
        working_directory,
        program,
        arguments,
        limits,
        execute_retained_source,
        environment,
        LaunchContext::CurrentAquaSession,
    )
}

fn run_captured_with_launch_context<F>(
    working_directory: &Path,
    program: &str,
    arguments: &[String],
    limits: Limits,
    execute_retained_source: bool,
    environment: F,
    context: LaunchContext,
) -> Captured
where
    F: FnOnce(&str) -> Result<Vec<(String, String)>, String>,
{
    let started = Instant::now();
    let Some(deadline) = started.checked_add(limits.timeout) else {
        return launch_failure(
            program,
            &std::io::Error::other("subprocess supervision deadline overflowed"),
            limits,
        );
    };
    let mut executable = match super::doctor::resolve_executable(program, limits.executable_bytes) {
        Ok(executable) => executable,
        Err(error) => {
            return launch_failure(program, &std::io::Error::other(error), limits);
        }
    };
    if execute_retained_source {
        executable.execute_retained_source();
    }
    let identity = executable.identity().clone();
    let extra_env = match environment(executable.execution_path()) {
        Ok(extra_env) => extra_env,
        Err(error) => {
            let mut captured = launch_failure(program, &std::io::Error::other(error), limits);
            captured.executable_identity = Some(identity);
            return captured;
        }
    };
    let launch = match launch(
        working_directory,
        executable.execution_path(),
        arguments,
        &extra_env,
        limits,
        deadline,
        context,
    ) {
        Ok(launch) => launch,
        Err(LaunchError { label, error }) => {
            let mut captured = launch_failure(&label, &error, limits);
            captured.executable_identity = Some(identity);
            return captured;
        }
    };
    let mut captured = receipt(limits, supervise(launch, limits, deadline));
    if let Err(error) = executable.verify_unchanged() {
        captured.supervision_error.get_or_insert(error);
    }
    captured.executable_identity = Some(identity);
    captured
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn parse_dedicated_containment_config(
    uid: Option<std::ffi::OsString>,
    home: Result<String, std::env::VarError>,
    user: Result<String, std::env::VarError>,
    controller_uid: u32,
) -> Result<Option<UidContainmentConfig>, std::io::Error> {
    let uid = match uid {
        Some(uid) => uid,
        None => return Ok(None),
    };
    let uid = uid.into_string().map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("{DEDICATED_CONTAINMENT_UID_ENV} is not UTF-8"),
        )
    })?;
    let numeric_uid = uid.parse::<u32>().map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("{DEDICATED_CONTAINMENT_UID_ENV} must be a numeric uid"),
        )
    })?;
    if !(501..=60_000).contains(&numeric_uid) || numeric_uid == controller_uid {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("{DEDICATED_CONTAINMENT_UID_ENV} must name a distinct uid in 501..=60000"),
        ));
    }
    let home = home.map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "{DEDICATED_CONTAINMENT_HOME_ENV} is required with {DEDICATED_CONTAINMENT_UID_ENV}"
            ),
        )
    })?;
    if !Path::new(&home).is_absolute() || home.bytes().any(|byte| byte == 0 || byte == b'=') {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("invalid {DEDICATED_CONTAINMENT_HOME_ENV}"),
        ));
    }
    let user = user.map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "{DEDICATED_CONTAINMENT_USER_ENV} is required with {DEDICATED_CONTAINMENT_UID_ENV}"
            ),
        )
    })?;
    if user.is_empty()
        || !user
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("invalid {DEDICATED_CONTAINMENT_USER_ENV}"),
        ));
    }
    Ok(Some(UidContainmentConfig { uid, home, user }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use uqm_rust::automation::child_session::{
        ChildSession, ChildSessionConfig, ChildSessionError,
    };

    fn limits() -> Limits {
        Limits {
            timeout: Duration::from_millis(250),
            termination_grace: Duration::from_millis(50),
            pipe_drain_timeout: Duration::from_millis(250),
            stdout_bytes: 1_024,
            stderr_bytes: 1_024,
            executable_bytes: 64 * 1024 * 1024,
        }
    }

    #[cfg(unix)]
    fn limits_for_current_executable() -> Limits {
        let mut test_limits = limits();
        test_limits.executable_bytes = std::env::current_exe()
            .expect("test executable")
            .metadata()
            .expect("test executable metadata")
            .len();
        test_limits
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn containment_fixture_permissioning_is_causal() {
        use std::os::unix::fs::PermissionsExt as _;

        let directory = tempfile::tempdir().unwrap();
        assert_ne!(
            directory.path().metadata().unwrap().permissions().mode() & 0o777,
            0o770
        );
        permit_containment_directory_if(directory.path(), true).unwrap();
        assert_eq!(
            directory.path().metadata().unwrap().permissions().mode() & 0o777,
            0o770
        );
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn containment_helper_is_staged_where_the_dedicated_identity_can_execute_it() {
        use std::os::unix::fs::PermissionsExt as _;

        let home = tempfile::tempdir().unwrap();
        let source = home.path().join("source-xtask");
        std::fs::write(&source, b"exact executable bytes").unwrap();
        let (directory, staged) = stage_containment_check_executable(home.path(), &source).unwrap();
        assert_eq!(std::fs::read(&staged).unwrap(), b"exact executable bytes");
        assert_eq!(
            directory.metadata().unwrap().permissions().mode() & 0o777,
            0o750
        );
        assert_eq!(
            staged.metadata().unwrap().permissions().mode() & 0o777,
            0o550
        );
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_process_inspection_distinguishes_live_terminal_and_vanished() {
        assert!(!macos_process_is_terminal(std::process::id() as i32).unwrap());

        // SAFETY: the child exits immediately without touching shared process state.
        let child = unsafe { libc::fork() };
        assert!(
            child >= 0,
            "fork failed: {}",
            std::io::Error::last_os_error()
        );
        if child == 0 {
            // SAFETY: _exit terminates the forked child without running Rust destructors.
            unsafe { libc::_exit(0) };
        }

        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let terminal = loop {
            match macos_process_is_terminal(child) {
                Ok(true) => break true,
                Ok(false) if std::time::Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Ok(false) => break false,
                Err(error) => {
                    // SAFETY: child is this process's direct child.
                    unsafe { libc::waitpid(child, std::ptr::null_mut(), 0) };
                    panic!("{error}");
                }
            }
        };
        // SAFETY: child is this process's direct child.
        assert_eq!(
            unsafe { libc::waitpid(child, std::ptr::null_mut(), 0) },
            child
        );
        assert!(terminal, "exited child was not classified as terminal");
        assert!(macos_process_is_terminal(i32::MAX).unwrap());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_record_reports_this_process_group_and_rejects_absent_processes() {
        let pid = std::process::id() as libc::pid_t;
        let record = macos_process_record(pid).expect("record for this process");
        // SAFETY: getpgrp has no preconditions.
        assert_eq!(record.group, unsafe { libc::getpgrp() });
        assert!(!record.terminal);
        assert!(macos_process_record(i32::MAX).is_none());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_start_identity_survives_a_leader_that_exits_before_registration_is_answered() {
        // SAFETY: fork has no preconditions here; the child exits immediately.
        let child = unsafe { libc::fork() };
        assert!(child >= 0, "fork failed");
        if child == 0 {
            // SAFETY: the child does nothing but exit.
            unsafe { libc::_exit(0) };
        }

        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            let record = macos_process_record(child).expect("record for exited child");
            if record.terminal {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "child never reached a terminal state"
            );
            std::thread::sleep(Duration::from_millis(10));
        }

        let start = monitor_process_start(child);
        assert!(
            start.is_some_and(|start| start > 0),
            "an exited leader must still yield a start identity"
        );

        // SAFETY: child is this process's direct child.
        assert_eq!(
            unsafe { libc::waitpid(child, std::ptr::null_mut(), 0) },
            child
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_membership_excludes_an_exited_child_of_this_group() {
        // SAFETY: fork has no preconditions here; the child exits immediately.
        let child = unsafe { libc::fork() };
        assert!(child >= 0, "fork failed");
        if child == 0 {
            // SAFETY: the child does nothing but exit.
            unsafe { libc::_exit(0) };
        }

        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            let record = macos_process_record(child).expect("record for exited child");
            if record.terminal {
                // The unreaped child still reports this group, so membership
                // must exclude it on state rather than on group alone.
                // SAFETY: getpgrp has no preconditions.
                assert_eq!(record.group, unsafe { libc::getpgrp() });
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "child never reached a terminal state"
            );
            std::thread::sleep(Duration::from_millis(10));
        }

        // SAFETY: getpgrp has no preconditions.
        let group = unsafe { libc::getpgrp() };
        let occupant = group_member_other_than(group, &[std::process::id() as libc::pid_t])
            .expect("membership inspection");
        assert_ne!(occupant, Some(child));

        // SAFETY: child is this process's direct child.
        assert_eq!(
            unsafe { libc::waitpid(child, std::ptr::null_mut(), 0) },
            child
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_uid_inspection_ignores_terminal_processes() {
        // SAFETY: every byte pattern is valid for this C process-info structure.
        let mut info = unsafe { std::mem::zeroed::<libc::proc_bsdinfo>() };
        info.pbi_uid = 48001;
        info.pbi_ruid = 48002;

        assert!(macos_process_matches_uid(&info, 48001));
        assert!(macos_process_matches_uid(&info, 48002));
        assert!(!macos_process_matches_uid(&info, 48003));

        info.pbi_status = libc::SZOMB;
        assert!(!macos_process_matches_uid(&info, 48001));
        assert!(!macos_process_matches_uid(&info, 48002));

        // SAFETY: getuid has no preconditions.
        let current_uid = unsafe { libc::getuid() };
        assert!(!privileged_uid_is_empty(&current_uid.to_string()).unwrap());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_process_inspection_distinguishes_terminal_processes_and_uids() {
        let path = Path::new("/proc/42/stat");
        assert_eq!(
            linux_process_state_and_group("42 (name with ) marker) Z 1 599 0", path).unwrap(),
            ('Z', 599)
        );
        assert_eq!(
            linux_process_state_and_group("42 (name) S 1 600 0", path).unwrap(),
            ('S', 600)
        );
        assert!(linux_process_state_and_group("malformed", path).is_err());

        let status_path = Path::new("/proc/42/status");
        assert_eq!(
            linux_process_real_and_effective_uids(
                "Name:\tprobe\nUid:\t59999\t60000\t59999\t60000\n",
                status_path,
            )
            .unwrap(),
            (59_999, 60_000)
        );
        assert!(linux_process_real_and_effective_uids("Name:\tprobe\n", status_path).is_err());
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn dedicated_containment_config_fails_closed_without_process_environment_mutation() {
        let missing = Err(std::env::VarError::NotPresent);
        assert_eq!(
            parse_dedicated_containment_config(None, missing.clone(), missing.clone(), 501)
                .unwrap(),
            None
        );
        for (uid, home, user, controller_uid) in [
            ("", Ok("/tmp/home".into()), Ok("worker".into()), 501),
            ("500", Ok("/tmp/home".into()), Ok("worker".into()), 501),
            (
                "59999",
                Err(std::env::VarError::NotPresent),
                Ok("worker".into()),
                501,
            ),
            ("59999", Ok("relative".into()), Ok("worker".into()), 501),
            (
                "59999",
                Ok("/tmp/home".into()),
                Err(std::env::VarError::NotPresent),
                501,
            ),
            ("59999", Ok("/tmp/home".into()), Ok("bad-name".into()), 501),
            ("59999", Ok("/tmp/home".into()), Ok("worker".into()), 59999),
        ] {
            assert!(
                parse_dedicated_containment_config(Some(uid.into()), home, user, controller_uid,)
                    .is_err(),
                "accepted invalid containment config for uid {uid}"
            );
        }
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn containment_environment_excludes_runner_command_and_credential_channels() {
        for denied in [
            "GITHUB_ENV",
            "GITHUB_PATH",
            "GITHUB_TOKEN",
            "ACTIONS_RUNTIME_TOKEN",
            "ACTIONS_ID_TOKEN_REQUEST_TOKEN",
            "RUNNER_TEMP",
            "BASH_ENV",
            "LD_PRELOAD",
            "DYLD_INSERT_LIBRARIES",
            "CARGO_REGISTRIES_CRATES_IO_TOKEN",
            "RUSTC_WRAPPER",
            "UQM_CI_BASE_SHA",
            "UQM_SECRET",
        ] {
            assert!(!containment_environment_allows(denied), "allowed {denied}");
        }
        for allowed in [
            "PATH",
            "CARGO_HOME",
            "CARGO_TARGET_DIR",
            "RUSTUP_HOME",
            "RUSTUP_TOOLCHAIN",
            "PKG_CONFIG_PATH",
            "HTTPS_PROXY",
        ] {
            assert!(
                containment_environment_allows(allowed),
                "rejected {allowed}"
            );
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn current_aqua_environment_is_clean_and_preserves_nested_containment_bindings() {
        let inherited = [
            ("PATH", "/usr/bin:/bin"),
            ("GITHUB_TOKEN", "secret"),
            ("UQM_SECRET", "secret"),
            (DEDICATED_CONTAINMENT_UID_ENV, "59999"),
            (DEDICATED_CONTAINMENT_HOME_ENV, "/tmp/containment"),
            (DEDICATED_CONTAINMENT_USER_ENV, "uqm_s4_containment"),
        ]
        .into_iter()
        .map(|(name, value)| (name.into(), value.into()));
        let environment =
            filter_inherited_environment(inherited, current_aqua_environment_allows).unwrap();

        assert_eq!(environment.get("PATH").unwrap(), "/usr/bin:/bin");
        assert_eq!(
            environment.get(DEDICATED_CONTAINMENT_UID_ENV).unwrap(),
            "59999"
        );
        assert_eq!(
            environment.get(DEDICATED_CONTAINMENT_HOME_ENV).unwrap(),
            "/tmp/containment"
        );
        assert_eq!(
            environment.get(DEDICATED_CONTAINMENT_USER_ENV).unwrap(),
            "uqm_s4_containment"
        );
        assert!(!environment.contains_key("GITHUB_TOKEN"));
        assert!(!environment.contains_key("UQM_SECRET"));

        assert!(current_aqua_command(
            "/usr/bin/true",
            &[],
            &[(DEDICATED_CONTAINMENT_UID_ENV.into(), "60000".into())],
        )
        .is_err());
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn dedicated_containment_command_strips_controls_and_binds_child_identity() {
        let config = parse_dedicated_containment_config(
            Some("59999".into()),
            Ok("/tmp/uqm-containment".into()),
            Ok("uqm_s4_containment".into()),
            501,
        )
        .unwrap()
        .unwrap();
        let mut environment = BTreeMap::from([
            ("PATH".into(), "/usr/bin:/bin".into()),
            (DEDICATED_CONTAINMENT_UID_ENV.into(), "59999".into()),
            (DEDICATED_CONTAINMENT_HOME_ENV.into(), "/wrong".into()),
            (DEDICATED_CONTAINMENT_USER_ENV.into(), "wrong".into()),
        ]);
        bind_dedicated_containment_environment(
            &mut environment,
            &[
                ("HOME".into(), "/attacker".into()),
                (DEDICATED_CONTAINMENT_UID_ENV.into(), "60000".into()),
            ],
            &config,
        );
        assert!(!environment.contains_key(DEDICATED_CONTAINMENT_UID_ENV));
        assert!(!environment.contains_key(DEDICATED_CONTAINMENT_HOME_ENV));
        assert!(!environment.contains_key(DEDICATED_CONTAINMENT_USER_ENV));
        assert_eq!(environment.get("HOME").unwrap(), "/tmp/uqm-containment");
        assert_eq!(environment.get("TMPDIR").unwrap(), "/tmp/uqm-containment");
        assert_eq!(environment.get("USER").unwrap(), "uqm_s4_containment");
        assert_eq!(environment.get("LOGNAME").unwrap(), "uqm_s4_containment");
        let command = build_dedicated_containment_command(
            &config,
            environment,
            "/usr/bin/true",
            &["argument".into()],
        );
        assert_eq!(command.get_program(), "/usr/bin/sudo");
        let arguments = command
            .get_args()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert_eq!(&arguments[..2], &["-n", "/bin/bash"]);
        assert!(arguments.iter().any(|argument| argument == "59999"));
        assert!(arguments.iter().any(|argument| argument == "/usr/bin/true"));
        assert!(arguments.iter().any(|argument| argument == "argument"));
        assert!(DEDICATED_UID_WRAPPER.contains("/usr/bin/pkill -KILL -U \"$uid\""));
        assert!(DEDICATED_UID_WRAPPER.contains("/usr/bin/pkill -KILL -u \"$uid\""));
        assert!(DEDICATED_UID_WRAPPER.contains("/bin/ps -o stat= -p \"$pid\""));
        assert!(DEDICATED_UID_WRAPPER.contains("/bin/kill -0 \"$pid\""));
        assert!(DEDICATED_UID_WRAPPER.contains("Z*|\"\""));
        assert!(DEDICATED_UID_WRAPPER.contains("/usr/bin/env -i"));
        let umask = DEDICATED_UID_WRAPPER.find("umask 0027").unwrap();
        let initial_cleanup = DEDICATED_UID_WRAPPER.find("if ! cleanup; then").unwrap();
        let launch = DEDICATED_UID_WRAPPER
            .find("/usr/bin/sudo -n -u \"#$uid\"")
            .unwrap();
        assert!(umask < initial_cleanup);
        assert!(initial_cleanup < launch);
        assert!(DEDICATED_UID_WRAPPER
            .contains("dedicated containment uid $uid still owns processes before launch"));
        assert!(!DEDICATED_UID_WRAPPER.contains("was already in use"));
        assert!(!DEDICATED_UID_WRAPPER.contains("launchctl"));
        assert!(DEDICATED_UID_WRAPPER.contains(
            "/usr/bin/sudo -n -u \"#$uid\" -- /usr/bin/env -i \"${env_args[@]}\" \"$@\""
        ));
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn privileged_uid_cleanup_commands_bind_signal_selector_and_uid() {
        for (signal, signal_argument) in [(libc::SIGTERM, "-TERM"), (libc::SIGKILL, "-KILL")] {
            for selector in ["-U", "-u"] {
                let command = privileged_uid_pkill_command("59999", signal, selector).unwrap();
                assert_eq!(command.get_program(), "/usr/bin/sudo");
                assert_eq!(
                    command
                        .get_args()
                        .map(|argument| argument.to_string_lossy().into_owned())
                        .collect::<Vec<_>>(),
                    ["-n", "/usr/bin/pkill", signal_argument, selector, "59999"]
                );
            }
        }
        assert!(privileged_uid_pkill_command("59999", 0, "-U").is_err());
        assert!(privileged_uid_pkill_command("59999", libc::SIGINT, "-U").is_err());
        assert!(privileged_uid_pkill_command("59999", libc::SIGTERM, "--uid").is_err());
    }

    #[test]
    fn supervision_deadline_overflow_is_a_typed_failure() {
        let probe = Probe::new(limits(), Duration::MAX);
        assert_eq!(
            probe.supervision_error.as_deref(),
            Some("subprocess supervision deadline overflowed")
        );
    }

    #[test]
    fn pipe_drain_deadline_overflow_is_a_typed_failure() {
        let mut overflow_limits = limits();
        overflow_limits.pipe_drain_timeout = Duration::MAX;
        let mut probe = Probe::new(overflow_limits, overflow_limits.timeout);
        let _ = next_loop_step(&mut probe, Instant::now(), false, true, overflow_limits);
        assert_eq!(
            probe.supervision_error.as_deref(),
            Some("subprocess pipe-drain deadline overflowed")
        );
    }

    #[cfg(unix)]
    fn process_exists(pid: i32) -> bool {
        // SAFETY: signal zero only observes the exact numeric PID.
        (unsafe { libc::kill(pid, 0) }) == 0
            || std::io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH)
    }

    #[cfg(unix)]
    fn process_is_live(pid: i32) -> bool {
        if !process_exists(pid) {
            return false;
        }
        Command::new("/bin/ps")
            .args(["-p", &pid.to_string(), "-o", "stat="])
            .output()
            .ok()
            .filter(|output| output.status.success())
            .and_then(|output| {
                output
                    .stdout
                    .into_iter()
                    .find(|byte| !byte.is_ascii_whitespace())
            })
            .is_none_or(|state| state != b'Z')
    }

    #[cfg(unix)]
    #[test]
    fn containment_monitor_process() {
        if std::env::var_os(MONITOR_REGISTRATION_FD_ENV).is_none() {
            return;
        }
        match run_containment_monitor_from_env() {
            Ok(()) => std::process::exit(0),
            Err(error) => {
                eprintln!("containment monitor failed: {error}");
                std::process::exit(1);
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn nested_group_helper_process() {
        if std::env::var_os("UQM_TEST_NESTED_HELPER").is_none() {
            return;
        }
        let pid = unsafe { libc::getpid() };
        std::fs::write(
            std::env::var("UQM_TEST_NESTED_PID_FILE").expect("PID file"),
            pid.to_string(),
        )
        .expect("write nested PID after registration acknowledgment");
        unsafe {
            libc::signal(libc::SIGTERM, libc::SIG_IGN);
        }
        loop {
            thread::sleep(Duration::from_secs(1));
        }
    }

    #[cfg(unix)]
    #[test]
    fn outer_controller_helper_process() {
        if std::env::var_os("UQM_TEST_OUTER_CONTROLLER").is_none() {
            return;
        }
        unsafe {
            libc::signal(libc::SIGTERM, libc::SIG_IGN);
        }
        let executable = std::env::current_exe().expect("test executable");
        let pid_file =
            std::path::PathBuf::from(std::env::var("UQM_TEST_NESTED_PID_FILE").expect("PID file"));
        let mut command = Command::new(executable);
        command
            .args([
                "--exact",
                "ci::exec::tests::nested_group_helper_process",
                "--nocapture",
            ])
            .env("UQM_TEST_NESTED_HELPER", "1")
            .env("UQM_TEST_NESTED_PID_FILE", &pid_file);
        let config = ChildSessionConfig {
            stdout_log: pid_file.with_extension("stdout.log"),
            stderr_log: pid_file.with_extension("stderr.log"),
            stdout_budget: 1_024,
            stderr_budget: 1_024,
            timeout: Duration::from_secs(30),
            grace: Duration::from_millis(100),
            executable_digest: "nested-test".into(),
        };
        let _nested = ChildSession::spawn(command, config).expect("spawn nested ChildSession");
        let deadline = Instant::now() + Duration::from_secs(5);
        while !pid_file.is_file() {
            assert!(Instant::now() < deadline, "nested helper did not register");
            thread::sleep(Duration::from_millis(5));
        }
        std::fs::write(
            std::env::var("UQM_TEST_OUTER_PID_FILE").expect("outer PID file"),
            std::process::id().to_string(),
        )
        .expect("write outer PID");
        loop {
            thread::sleep(Duration::from_secs(1));
        }
    }

    #[cfg(unix)]
    #[test]
    fn outer_supervisor_helper_process() {
        if std::env::var_os("UQM_TEST_OUTER_SUPERVISOR").is_none() {
            return;
        }
        let executable = std::env::current_exe().expect("test executable");
        let mut test_limits = limits_for_current_executable();
        test_limits.timeout = Duration::from_secs(30);
        test_limits.termination_grace = Duration::from_millis(100);
        test_limits.pipe_drain_timeout = Duration::from_secs(2);
        let captured = run_captured_with_limits(
            Path::new("."),
            executable.to_str().expect("UTF-8 test executable"),
            &[
                "--exact".into(),
                "ci::exec::tests::outer_controller_helper_process".into(),
                "--nocapture".into(),
            ],
            &[
                ("UQM_TEST_OUTER_CONTROLLER".into(), "1".into()),
                (
                    "UQM_TEST_NESTED_PID_FILE".into(),
                    std::env::var("UQM_TEST_NESTED_PID_FILE").expect("nested PID file"),
                ),
                (
                    "UQM_TEST_OUTER_PID_FILE".into(),
                    std::env::var("UQM_TEST_OUTER_PID_FILE").expect("outer PID file"),
                ),
            ],
            test_limits,
        );
        panic!("outer supervisor was not terminated: {captured:?}");
    }

    #[cfg(unix)]
    #[test]
    fn nested_child_session_lifecycle_process() {
        let Some(directory) = std::env::var_os("UQM_TEST_CHILD_SESSION_LIFECYCLE") else {
            return;
        };
        assert!(
            std::env::var_os(NESTED_GROUP_REGISTRATION_FD_ENV).is_some(),
            "run_captured must expose the shared descriptor contract"
        );
        let directory = std::path::PathBuf::from(directory);
        let mut command = Command::new("sh");
        command.args(["-c", "trap '' TERM; while :; do sleep 1; done"]);
        let config = ChildSessionConfig {
            stdout_log: directory.join("lifecycle.stdout.log"),
            stderr_log: directory.join("lifecycle.stderr.log"),
            stdout_budget: 1_024,
            stderr_budget: 1_024,
            timeout: Duration::from_millis(100),
            grace: Duration::from_millis(50),
            executable_digest: "nested-lifecycle-test".into(),
        };
        let session = ChildSession::spawn(command, config).expect("register nested ChildSession");
        let pid = session.pid() as libc::pid_t;
        let failure = session.finish().expect_err("nested child must time out");
        assert!(matches!(
            failure.error,
            ChildSessionError::Timeout {
                term_sent: true,
                kill_sent: true,
            }
        ));
        assert_eq!(failure.receipt.signal, Some(libc::SIGKILL));
        assert!(failure.receipt.output_drained);
        assert!(failure.receipt.orphan_check_passed);
        assert!(!process_exists(pid));
    }

    #[cfg(unix)]
    #[test]
    fn anchor_target_helper_process() {
        let Some(mode) = std::env::var_os("UQM_TEST_ANCHOR_TARGET") else {
            return;
        };
        if mode == "exit" {
            thread::sleep(Duration::from_millis(100));
            return;
        }
        unsafe {
            libc::signal(libc::SIGTERM, libc::SIG_IGN);
        }
        loop {
            thread::sleep(Duration::from_secs(1));
        }
    }

    #[cfg(unix)]
    fn spawn_test_group(mode: &str) -> std::process::Child {
        use std::os::unix::process::CommandExt as _;
        let executable = std::env::current_exe().expect("test executable");
        let mut command = Command::new(executable);
        command
            .args([
                "--exact",
                "ci::exec::tests::anchor_target_helper_process",
                "--nocapture",
            ])
            .env("UQM_TEST_ANCHOR_TARGET", mode)
            .process_group(0)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn process-group leader")
    }

    #[cfg(unix)]
    #[test]
    fn monitor_anchor_survives_leader_exit_until_explicit_reap() {
        let mut leader = spawn_test_group("exit");
        let leader_pid = leader.id() as libc::pid_t;
        let start = monitor_process_start(leader_pid).expect("leader start identity");
        let anchor = MonitorAnchor::spawn(leader_pid, start, -1, -1).expect("stable anchor");
        leader.wait().expect("reap leader");
        assert_eq!(monitor_process_start(leader_pid), None);
        assert_eq!(unsafe { libc::getpgid(anchor.pid) }, leader_pid);
        anchor
            .signal_group(libc::SIGTERM)
            .expect("signal anchored group");
        anchor.release().expect("reap monitor-owned anchor");
    }

    #[cfg(unix)]
    #[test]
    fn identity_mismatch_never_signals_an_unrelated_reused_group() {
        let mut candidate = spawn_test_group("wait");
        let mut unrelated = spawn_test_group("wait");
        let candidate_pid = candidate.id() as libc::pid_t;
        let start = monitor_process_start(candidate_pid).expect("candidate start identity");
        let error = MonitorAnchor::spawn_checked(candidate_pid, start, -1, -1, || {
            Some(start.wrapping_add(1))
        })
        .expect_err("changed identity must reject registration");
        assert!(error.contains("changed identity"));
        assert!(process_exists(candidate_pid));
        assert!(process_exists(unrelated.id() as libc::pid_t));
        for child in [&mut candidate, &mut unrelated] {
            child.kill().expect("kill test process");
            child.wait().expect("reap test process");
        }
    }

    #[cfg(unix)]
    #[test]
    fn leader_anchor_blocks_group_operations_after_reap_boundary() {
        let mut anchor = LeaderAnchor {
            pid: 123,
            monitor_anchor_pid: 124,
            observed: true,
            observed_at: Some(Instant::now()),
            reaped: false,
            dedicated_uid: None,
        };
        anchor.mark_reaped().unwrap();
        assert!(signal_group(&anchor, 0).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn descendant_cleanup_waits_for_natural_process_group_settlement() {
        let observed_at = Instant::now();
        let anchor = LeaderAnchor {
            pid: 123,
            monitor_anchor_pid: 124,
            observed: true,
            observed_at: Some(observed_at),
            reaped: false,
            dedicated_uid: None,
        };
        let grace = Duration::from_millis(100);
        assert!(!anchor.descendant_cleanup_required(observed_at, false, grace));
        assert!(anchor.descendant_cleanup_required(observed_at + grace, false, grace));
        assert!(!anchor.descendant_cleanup_required(observed_at + grace, true, grace));
    }

    #[cfg(unix)]
    #[test]
    fn child_session_nested_run_registers_terminates_kills_reaps_and_unregisters() {
        let directory = std::env::temp_dir().join(format!(
            "uqm-nested-lifecycle-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        ));
        std::fs::create_dir(&directory).expect("create test directory");
        let mut unrelated = spawn_test_group("wait");
        let executable = std::env::current_exe().expect("test executable");
        let mut test_limits = limits_for_current_executable();
        test_limits.timeout = Duration::from_secs(120);
        test_limits.pipe_drain_timeout = Duration::from_secs(1);
        let captured = run_captured_with_limits(
            Path::new("."),
            executable.to_str().expect("UTF-8 test executable"),
            &[
                "--exact".into(),
                "ci::exec::tests::nested_child_session_lifecycle_process".into(),
                "--nocapture".into(),
            ],
            &[(
                "UQM_TEST_CHILD_SESSION_LIFECYCLE".into(),
                directory.to_string_lossy().into_owned(),
            )],
            test_limits,
        );
        assert!(
            captured.succeeded(),
            "nested lifecycle failed: {captured:?}"
        );
        assert!(
            process_exists(unrelated.id() as libc::pid_t),
            "nested cleanup signaled an unrelated process group"
        );
        unrelated.kill().expect("kill unrelated test process");
        unrelated.wait().expect("reap unrelated test process");
        std::fs::remove_dir_all(directory).expect("remove test directory");
    }
    #[cfg(unix)]
    #[test]
    fn terminating_outer_controller_cleans_registered_nested_session() {
        let directory = std::env::temp_dir().join(format!(
            "uqm-containment-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        ));
        std::fs::create_dir(&directory).expect("create test directory");
        let nested_pid_file = directory.join("nested.pid");
        let outer_pid_file = directory.join("outer.pid");
        let mut unrelated = spawn_test_group("wait");
        let executable = std::env::current_exe().expect("test executable");
        let mut supervisor = Command::new(executable)
            .args([
                "--exact",
                "ci::exec::tests::outer_supervisor_helper_process",
                "--nocapture",
            ])
            .env("UQM_TEST_OUTER_SUPERVISOR", "1")
            .env("UQM_TEST_NESTED_PID_FILE", &nested_pid_file)
            .env("UQM_TEST_OUTER_PID_FILE", &outer_pid_file)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn outer supervisor");
        let deadline = Instant::now() + Duration::from_secs(30);
        while !nested_pid_file.is_file() || !outer_pid_file.is_file() {
            assert!(Instant::now() < deadline, "nested group did not register");
            thread::sleep(Duration::from_millis(5));
        }
        let nested_pid = std::fs::read_to_string(&nested_pid_file)
            .expect("read nested PID")
            .parse::<i32>()
            .expect("parse nested PID");
        let outer_pid = std::fs::read_to_string(&outer_pid_file)
            .expect("read outer PID")
            .parse::<i32>()
            .expect("parse outer PID");
        assert_eq!(
            unsafe { libc::kill(supervisor.id() as i32, libc::SIGKILL) },
            0
        );
        supervisor.wait().expect("reap outer supervisor");

        let cleanup_deadline = Instant::now() + Duration::from_secs(30);
        for pid in [outer_pid, nested_pid] {
            while process_is_live(pid) && Instant::now() < cleanup_deadline {
                thread::sleep(Duration::from_millis(5));
            }
            assert!(
                !process_is_live(pid),
                "registered process {pid} survived outer supervisor termination"
            );
        }
        assert!(
            process_exists(unrelated.id() as libc::pid_t),
            "outer-controller cleanup signaled an unrelated process group"
        );
        unrelated.kill().expect("kill unrelated test process");
        unrelated.wait().expect("reap unrelated test process");
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn launch_failure_is_a_captured_result() {
        let captured = run_captured_with_limits(
            Path::new("."),
            "uqm-ci-command-that-does-not-exist",
            &[],
            &[],
            limits(),
        );
        assert!(captured.stdout.is_empty());
        assert!(captured.stderr.is_empty());
        assert_eq!(captured.exit_code, None);
        assert_eq!(captured.signal, None);
        assert!(captured
            .launch_error
            .as_deref()
            .is_some_and(|error| error.contains("cannot execute")));
        assert_eq!(captured.process_group_cleanup, "not-started");
        assert!(!captured.succeeded());
    }

    #[test]
    fn command_deadline_is_active_before_launch() {
        let mut expired = limits();
        expired.timeout = Duration::ZERO;
        let captured = run_captured_with_limits(
            Path::new("."),
            "sh",
            &["-c".into(), "exit 0".into()],
            &[],
            expired,
        );
        assert_eq!(captured.process_group_cleanup, "not-started");
        assert!(
            captured
                .launch_error
                .as_deref()
                .is_some_and(|error| error.to_ascii_lowercase().contains("timed out")),
            "{:?}",
            captured.launch_error
        );
    }

    #[cfg(unix)]
    #[test]
    fn hanging_process_is_terminated_reaped_and_typed() {
        // The budget must outlast containment startup on a loaded host, or the
        // command never launches and the deadline proves nothing.
        let mut hanging_limits = limits();
        hanging_limits.timeout = Duration::from_secs(10);
        let captured = run_captured_with_limits(
            Path::new("."),
            "sh",
            &[
                "-c".into(),
                "trap '' TERM; while :; do sleep 1; done".into(),
            ],
            &[],
            hanging_limits,
        );
        assert!(captured.timed_out);
        assert_eq!(captured.termination_reason, "timeout");
        assert_eq!(captured.termination_signal, "kill");
        assert_eq!(captured.process_group_cleanup, "verified-empty");
        assert_eq!(captured.pipe_cleanup, "complete");
        assert!(
            captured.supervision_error.is_none(),
            "{:?}",
            captured.supervision_error
        );
        assert!(!captured.succeeded());
    }

    #[cfg(unix)]
    #[test]
    fn output_flood_is_bounded_and_terminates_the_group() {
        let mut flood_limits = limits();
        flood_limits.stdout_bytes = 64;
        let captured = run_captured_with_limits(
            Path::new("."),
            "sh",
            &[
                "-c".into(),
                "while :; do printf 0123456789abcdef; done".into(),
            ],
            &[],
            flood_limits,
        );
        assert_eq!(captured.stdout.len(), 64);
        assert!(captured.stdout_bytes_seen > 64);
        assert!(captured.stdout_truncated);
        assert_eq!(captured.termination_reason, "output-limit");
        assert_eq!(captured.process_group_cleanup, "verified-empty");
        assert!(captured.supervision_error.is_none());
    }

    #[cfg(unix)]
    #[test]
    fn short_lived_descendant_settles_without_false_cleanup_failure() {
        let mut settlement_limits = limits();
        settlement_limits.timeout = Duration::from_secs(1);
        settlement_limits.termination_grace = Duration::from_millis(250);
        let captured = run_captured_with_limits(
            Path::new("."),
            "sh",
            &["-c".into(), "(sleep 0.05) &".into()],
            &[],
            settlement_limits,
        );
        assert!(captured.succeeded(), "{captured:?}");
        assert_eq!(captured.termination_reason, "none");
        assert_eq!(captured.process_group_cleanup, "verified-empty");
    }

    #[cfg(unix)]
    #[test]
    fn inherited_pipe_descendant_is_cleaned_without_waiting_for_eof() {
        let root = tempfile::tempdir().unwrap();
        let captured = run_captured_with_limits(
            root.path(),
            "sh",
            &[
                "-c".into(),
                "(trap '' TERM; mkdir ready; exec sleep 30) & while [ ! -d ready ]; do :; done; rmdir ready; printf parent-exited".into(),
            ],
            &[],
            limits(),
        );
        assert_eq!(captured.stdout, b"parent-exited");
        assert_eq!(captured.exit_code, Some(0));
        assert_eq!(captured.termination_reason, "descendant-cleanup");
        assert_eq!(captured.termination_signal, "kill");
        assert_eq!(captured.process_group_cleanup, "verified-empty");
        assert_eq!(captured.pipe_cleanup, "complete");
        assert!(
            captured.supervision_error.is_none(),
            "{:?}",
            captured.supervision_error
        );
        assert!(!captured.succeeded());
    }
}
