//! ChildSession supervision: pure model and production supervisor.
//!
//! ## Pure model
//!
//! [`ChildSessionModel`] is a pure state-machine for session lifecycle. It
//! tracks state transitions, identity, and hang classification without any
//! I/O or process management.
//!
//! ## Production supervisor (`cfg(unix)`)
//!
//! [`ChildSession`] is a real OS-level supervisor: spawns an exact
//! `std::process::Child`, drains piped stdout/stderr concurrently into
//! bounded log files, records PID/start-time identity, owns a single
//! wait/reap path, applies cooperative SIGTERM then SIGKILL to the exact
//! PID on timeout, joins both readers, performs an orphan check, and
//! provides a non-panicking Drop backstop.
//!
//! @plan PLAN-20260723-RUNTIME-AUTOMATION.P08
//! @requirement REQ-PROOF-002

use std::path::PathBuf;

// ===========================================================================
//  Session state machine (REQ-PROOF-002)
// ===========================================================================

/// The state of a ChildSession.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P08
/// @requirement REQ-PROOF-002
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Child spawned, running.
    Running,
    /// Cooperative stop requested.
    StopRequested,
    /// Child reaped (exit status stored).
    Reaped,
    /// Parent pipe handles closed.
    PipesClosed,
    /// Reader threads joined.
    Joined,
    /// Session complete: validated and cleaned up.
    Complete,
}

impl SessionState {
    /// Returns `true` if this state is terminal (no further transitions).
    #[must_use]
    pub fn is_terminal(self) -> bool {
        self == Self::Complete
    }

    /// Returns the expected next state, or `None` if terminal.
    #[must_use]
    pub fn next(self) -> Option<SessionState> {
        match self {
            Self::Running => Some(Self::StopRequested),
            Self::StopRequested => Some(Self::Reaped),
            Self::Reaped => Some(Self::PipesClosed),
            Self::PipesClosed => Some(Self::Joined),
            Self::Joined => Some(Self::Complete),
            Self::Complete => None,
        }
    }
}

// ===========================================================================
//  Process identity (REQ-PROOF-003)
// ===========================================================================

/// Process identity for orphan detection and PID-reuse prevention.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P08
/// @requirement REQ-PROOF-003
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessIdentity {
    /// Process ID.
    pub pid: u32,
    /// Process start time (for PID reuse detection).
    pub start_time: String,
    /// SHA-256 digest of the executable.
    pub executable_digest: String,
}

impl ProcessIdentity {
    /// Check if two identities match (same PID, same start time, same
    /// executable digest). Used to detect PID reuse.
    ///
    /// @plan PLAN-20260723-RUNTIME-AUTOMATION.P08
    /// @requirement REQ-PROOF-003
    #[must_use]
    pub fn matches(&self, other: &ProcessIdentity) -> bool {
        self.pid == other.pid
            && self.start_time == other.start_time
            && self.executable_digest == other.executable_digest
    }
}

// ===========================================================================
//  Hang classification (REQ-WATCH-004)
// ===========================================================================

/// Classification of a child that failed to respond.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P08
/// @requirement REQ-WATCH-004
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HangClassification {
    /// Child reached a watchdog limit cooperatively.
    CooperativeTimeout,
    /// Child never reached any callback; parent observed hard hang.
    ParentHardHang,
}

// ===========================================================================
//  Session result (REQ-PROOF-002)
// ===========================================================================

/// The result of a ChildSession finish attempt.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P08
/// @requirement REQ-PROOF-002
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionResult {
    /// Session completed successfully with an exit status.
    Complete { exit_code: i32 },
    /// Session failed: cooperative timeout.
    CooperativeTimeout,
    /// Session failed: hard hang (no callback).
    HardHang,
    /// Session failed: reader error.
    ReaderError(String),
    /// Session failed: join panic.
    JoinPanic,
    /// Session failed: socket cleanup failure.
    SocketCleanupFailure,
    /// Session failed: spawn partial failure.
    SpawnPartialFailure,
}

// ===========================================================================
//  ChildSession model
// ===========================================================================

/// The pure model for ChildSession supervision.
///
/// This is a pure state-machine: it tracks the session state, identity, and
/// classification without spawning processes, managing pipes, or performing
/// any I/O. For production OS-level supervision see [`ChildSession`].
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P08
/// @requirement REQ-PROOF-002
pub struct ChildSessionModel {
    state: SessionState,
    identity: ProcessIdentity,
    socket_path: Option<PathBuf>,
    manifest_path: Option<PathBuf>,
    exit_code: Option<i32>,
    hang_classification: Option<HangClassification>,
}

impl ChildSessionModel {
    /// Create a new session model.
    #[must_use]
    pub fn new(identity: ProcessIdentity) -> Self {
        Self {
            state: SessionState::Running,
            identity,
            socket_path: None,
            manifest_path: None,
            exit_code: None,
            hang_classification: None,
        }
    }

    /// Get the current state.
    #[must_use]
    pub fn state(&self) -> SessionState {
        self.state
    }

    /// Get the process identity.
    #[must_use]
    pub fn identity(&self) -> &ProcessIdentity {
        &self.identity
    }

    /// Record a successful reap (exit status stored exactly once).
    ///
    /// @plan PLAN-20260723-RUNTIME-AUTOMATION.P08
    /// @requirement REQ-PROOF-002
    pub fn record_reap(&mut self, exit_code: i32) {
        if self.state == SessionState::Running || self.state == SessionState::StopRequested {
            self.exit_code = Some(exit_code);
            self.state = SessionState::Reaped;
        }
        // If already reaped, this is a no-op (do not call wait again).
    }

    /// Transition to pipes closed.
    pub fn close_pipes(&mut self) {
        if self.state == SessionState::Reaped {
            self.state = SessionState::PipesClosed;
        }
    }

    /// Transition to joined.
    pub fn join(&mut self) {
        if self.state == SessionState::PipesClosed {
            self.state = SessionState::Joined;
        }
    }

    /// Transition to complete.
    pub fn complete(&mut self) -> SessionResult {
        if self.state == SessionState::Joined {
            self.state = SessionState::Complete;
            if let Some(code) = self.exit_code {
                return SessionResult::Complete { exit_code: code };
            }
        }
        SessionResult::Complete { exit_code: -1 }
    }

    /// Request cooperative stop.
    pub fn request_stop(&mut self) {
        if self.state == SessionState::Running {
            self.state = SessionState::StopRequested;
        }
    }

    /// Classify hang.
    pub fn classify_hang(&mut self, classification: HangClassification) {
        self.hang_classification = Some(classification);
    }

    /// Get the hang classification.
    #[must_use]
    pub fn hang_classification(&self) -> Option<HangClassification> {
        self.hang_classification
    }

    /// Set the socket path.
    pub fn set_socket_path(&mut self, path: PathBuf) {
        self.socket_path = Some(path);
    }

    /// Set the manifest path.
    pub fn set_manifest_path(&mut self, path: PathBuf) {
        self.manifest_path = Some(path);
    }

    /// Returns `true` if the session is in a state where kill is appropriate.
    #[must_use]
    pub fn should_kill(&self) -> bool {
        matches!(self.state, SessionState::StopRequested)
    }

    /// Returns `true` if the session has been reaped.
    #[must_use]
    pub fn is_reaped(&self) -> bool {
        matches!(
            self.state,
            SessionState::Reaped
                | SessionState::PipesClosed
                | SessionState::Joined
                | SessionState::Complete
        )
    }
}

// ===========================================================================
//  Proof run types (REQ-PROOF-001..008)
// ===========================================================================

/// The type of proof run.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P08
/// @requirement REQ-PROOF-001..008
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProofType {
    /// Main menu navigation proof (NewGame → LoadGame).
    MainMenu,
    /// Watchdog cooperative timeout proof.
    Watchdog,
    /// Inactive smoke transport proof.
    InactiveSmoke,
    /// Controlled hard hang proof.
    HardHang,
}

/// The result of a proof run.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P08
/// @requirement REQ-PROOF-007
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofResult {
    /// The proof type.
    pub proof_type: ProofType,
    /// Whether the proof passed.
    pub passed: bool,
    /// The exit code of the child.
    pub exit_code: Option<i32>,
    /// Hang classification if applicable.
    pub hang_classification: Option<HangClassification>,
    /// Whether the teardown receipt was created.
    pub teardown_receipt_created: bool,
    /// Whether the proof report was created.
    pub proof_report_created: bool,
    /// Whether orphan check passed.
    pub orphan_check_passed: bool,
}

impl ProofResult {
    /// Create a passing proof result.
    #[must_use]
    pub fn passed(proof_type: ProofType, exit_code: i32) -> Self {
        Self {
            proof_type,
            passed: true,
            exit_code: Some(exit_code),
            hang_classification: None,
            teardown_receipt_created: true,
            proof_report_created: true,
            orphan_check_passed: true,
        }
    }

    /// Create a failing proof result.
    #[must_use]
    pub fn failed(proof_type: ProofType, classification: HangClassification) -> Self {
        Self {
            proof_type,
            passed: false,
            exit_code: None,
            hang_classification: Some(classification),
            teardown_receipt_created: false,
            proof_report_created: false,
            orphan_check_passed: false,
        }
    }
}

// ===========================================================================
//  Unit tests
// ===========================================================================
//  Production ChildSession supervisor (cfg(unix))
// ===========================================================================
//
// Real OS-level supervisor: spawns an exact std::process::Child, drains
// stdout/stderr concurrently into bounded log files, records PID/start-time
// identity, owns a single wait/reap path, applies SIGTERM then SIGKILL to
// the exact PID, joins readers, checks for orphans, and provides a
// non-panicking Drop backstop.

#[cfg(unix)]
mod os {
    use std::ffi::c_int;
    use std::ffi::c_void;
    use std::fs::{File, OpenOptions};
    use std::io::{self, ErrorKind, Read, Write};
    use std::path::{Path, PathBuf};
    use std::process::{Child, Command, ExitStatus, Stdio};
    use std::thread::{self, JoinHandle};
    use std::time::{Duration, Instant};

    use super::ProcessIdentity;

    // --- Errors -----------------------------------------------------------

    /// Which output stream a value relates to.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum StreamKind {
        /// Standard output.
        Stdout,
        /// Standard error.
        Stderr,
    }

    /// Errors produced by [`ChildSession`].
    ///
    /// @plan PLAN-20260723-RUNTIME-AUTOMATION.P08
    /// @requirement REQ-PROOF-002
    #[derive(Debug)]
    pub enum ChildSessionError {
        /// `spawn` failed before any child existed.
        Spawn(io::Error),
        /// Creating or writing a log file failed.
        LogOpen {
            stream: StreamKind,
            error: io::Error,
        },
        /// A requested child pipe was unexpectedly unavailable.
        PipeUnavailable { stream: StreamKind },
        /// Starting a named reader thread failed.
        ReaderStart {
            stream: StreamKind,
            error: io::Error,
        },
        /// A reader thread encountered an I/O error.
        Reader {
            stream: StreamKind,
            error: io::Error,
        },
        /// A reader thread panicked and could not be joined cleanly.
        JoinPanic { stream: StreamKind },
        /// The child produced more output than the byte budget allows.
        BudgetExceeded { stream: StreamKind },
        /// The child was still live after the timeout and grace period.
        Timeout {
            /// Whether SIGTERM was sent.
            term_sent: bool,
            /// Whether SIGKILL was sent.
            kill_sent: bool,
        },
        /// Exact process start identity could not be captured.
        IdentityUnavailable { pid: u32 },
        /// Signaling the exact child failed for a reason other than it already exiting.
        Signal {
            pid: u32,
            signal: i32,
            error: io::Error,
        },
        /// Waiting for the exact child failed.
        Wait(io::Error),
        /// The recorded process identity was still live after reaping.
        Orphan { pid: u32 },
    }

    impl std::fmt::Display for ChildSessionError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::Spawn(e) => write!(f, "spawn failed: {e}"),
                Self::LogOpen { stream, error } => {
                    write!(f, "log open failed for {stream:?}: {error}")
                }
                Self::PipeUnavailable { stream } => {
                    write!(f, "piped {stream:?} was unavailable after spawn")
                }
                Self::ReaderStart { stream, error } => {
                    write!(f, "reader thread start failed for {stream:?}: {error}")
                }
                Self::Reader { stream, error } => {
                    write!(f, "reader error on {stream:?}: {error}")
                }
                Self::JoinPanic { stream } => {
                    write!(f, "reader thread panic on {stream:?}")
                }
                Self::BudgetExceeded { stream } => {
                    write!(f, "byte budget exceeded on {stream:?}")
                }
                Self::Timeout {
                    term_sent,
                    kill_sent,
                } => {
                    write!(f, "child timed out (term={term_sent}, kill={kill_sent})")
                }
                Self::IdentityUnavailable { pid } => {
                    write!(f, "process start identity unavailable for pid {pid}")
                }
                Self::Signal { pid, signal, error } => {
                    write!(f, "signal {signal} failed for pid {pid}: {error}")
                }
                Self::Wait(error) => write!(f, "wait failed: {error}"),
                Self::Orphan { pid } => write!(f, "orphan process still live: pid {pid}"),
            }
        }
    }

    impl std::error::Error for ChildSessionError {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                Self::Spawn(e)
                | Self::LogOpen { error: e, .. }
                | Self::ReaderStart { error: e, .. }
                | Self::Reader { error: e, .. }
                | Self::Signal { error: e, .. }
                | Self::Wait(e) => Some(e),
                _ => None,
            }
        }
    }

    // --- Process identity / start-time -----------------------------------

    /// Capture the process start time (wall-clock microseconds since epoch)
    /// for a PID on macOS via `proc_pidinfo` + `PROC_PIDTBSDINFO`.
    ///
    /// Returns `None` if the information is unavailable (e.g. insufficient
    /// privileges or the process has already exited).
    ///
    /// @plan PLAN-20260723-RUNTIME-AUTOMATION.P08
    /// @requirement REQ-PROOF-003
    #[cfg(target_os = "macos")]
    #[must_use]
    pub fn process_start_micros(pid: u32) -> Option<u64> {
        // SAFETY: we provide a correctly-sized buffer for the flavor
        // PROC_PIDTBSDINFO and pass it to proc_pidinfo. The function fills
        // the buffer or returns an error value; we validate the returned
        // byte count before reading.
        unsafe {
            let mut info: libc::proc_bsdinfo = std::mem::zeroed();
            const PROC_PIDTBSDINFO: c_int = 3;
            let n = libc::proc_pidinfo(
                pid as c_int,
                PROC_PIDTBSDINFO,
                0,
                &mut info as *mut _ as *mut c_void,
                std::mem::size_of::<libc::proc_bsdinfo>() as c_int,
            );
            if n == std::mem::size_of::<libc::proc_bsdinfo>() as c_int {
                Some(info.pbi_start_tvsec * 1_000_000 + info.pbi_start_tvusec)
            } else {
                None
            }
        }
    }

    /// Capture the process start time on Linux via `/proc/<pid>/stat`.
    ///
    /// Returns `None` if the information is unavailable.
    ///
    /// @plan PLAN-20260723-RUNTIME-AUTOMATION.P08
    /// @requirement REQ-PROOF-003
    #[cfg(target_os = "linux")]
    #[must_use]
    pub fn process_start_micros(pid: u32) -> Option<u64> {
        let contents = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
        // SAFETY: sysconf is a read-only configuration query.
        let hz = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
        let hz = u64::try_from(hz).ok().filter(|value| *value > 0)?;
        parse_linux_proc_start_micros(&contents, hz)
    }

    /// Parse Linux `/proc/<pid>/stat` and convert field 22 (`starttime`) from
    /// ticks to microseconds. The process name may contain spaces and `)`.
    #[must_use]
    pub fn parse_linux_proc_start_micros(stat: &str, hz: u64) -> Option<u64> {
        if hz == 0 {
            return None;
        }
        let after_comm = stat.rfind(')')?;
        let fields: Vec<&str> = stat
            .get(after_comm.checked_add(1)?..)?
            .split_whitespace()
            .collect();
        let ticks = fields.get(19)?.parse::<u64>().ok()?;
        ticks.checked_mul(1_000_000)?.checked_div(hz)
    }

    /// Build a [`ProcessIdentity`] for a PID, capturing start time and the
    /// provided executable digest.
    ///
    /// @plan PLAN-20260723-RUNTIME-AUTOMATION.P08
    /// @requirement REQ-PROOF-003
    pub fn capture_identity(
        pid: u32,
        executable_digest: &str,
    ) -> Result<ProcessIdentity, ChildSessionError> {
        let start_time = process_start_micros(pid)
            .ok_or(ChildSessionError::IdentityUnavailable { pid })?
            .to_string();
        Ok(ProcessIdentity {
            pid,
            start_time,
            executable_digest: executable_digest.to_string(),
        })
    }

    /// Returns `true` if `pid` is still alive (`kill(pid, 0)` succeeds or
    /// returns `EPERM`).
    fn pid_exists(pid: u32) -> bool {
        // SAFETY: kill(pid, 0) is a standard signal-existence check that
        // does not deliver a signal.
        let rc = unsafe { libc::kill(pid as libc::pid_t, 0) };
        if rc == 0 {
            true
        } else {
            std::io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH)
        }
    }

    fn same_process_exists(identity: &ProcessIdentity) -> bool {
        if !pid_exists(identity.pid) {
            return false;
        }
        process_start_micros(identity.pid)
            .map(|start| start.to_string() == identity.start_time)
            .unwrap_or(true)
    }
    /// Send `signal` to `pid`.
    fn signal_pid(pid: u32, signal: libc::c_int) -> io::Result<()> {
        // SAFETY: kill() with a valid signal number is safe.
        let rc = unsafe { libc::kill(pid as libc::pid_t, signal) };
        if rc == 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        }
    }

    // --- Transactional readers and receipt-bearing supervision ------------

    #[derive(Debug, Clone)]
    enum ReaderFaultKind {
        BudgetExceeded,
        Io { message: String },
    }

    #[derive(Debug, Clone)]
    struct ReaderFault {
        stream: StreamKind,
        kind: ReaderFaultKind,
    }

    #[derive(Debug, Default)]
    struct DrainResult {
        bytes_written: u64,
        reached_eof: bool,
        fault: Option<ReaderFaultKind>,
    }

    fn notify_fault(
        sender: &std::sync::mpsc::Sender<ReaderFault>,
        stream: StreamKind,
        fault: &ReaderFaultKind,
    ) {
        let _ = sender.send(ReaderFault {
            stream,
            kind: fault.clone(),
        });
    }

    fn drain_stream(
        mut reader: Box<dyn Read + Send>,
        mut file: File,
        budget: u64,
        stream: StreamKind,
        sender: std::sync::mpsc::Sender<ReaderFault>,
    ) -> DrainResult {
        let mut result = DrainResult::default();
        let mut discard = false;
        let mut buffer = [0_u8; 8192];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => {
                    result.reached_eof = true;
                    break;
                }
                Ok(read) if discard => {
                    let _ = read;
                }
                Ok(read) => {
                    let remaining = budget.saturating_sub(result.bytes_written);
                    let writable = read.min(usize::try_from(remaining).unwrap_or(usize::MAX));
                    if writable > 0 {
                        if let Err(error) = file.write_all(&buffer[..writable]) {
                            let fault = ReaderFaultKind::Io {
                                message: error.to_string(),
                            };
                            notify_fault(&sender, stream, &fault);
                            result.fault = Some(fault);
                            discard = true;
                            continue;
                        }
                        result.bytes_written = result
                            .bytes_written
                            .saturating_add(u64::try_from(writable).unwrap_or(u64::MAX));
                    }
                    if writable < read {
                        let fault = ReaderFaultKind::BudgetExceeded;
                        notify_fault(&sender, stream, &fault);
                        result.fault = Some(fault);
                        discard = true;
                    }
                }
                Err(error) if error.kind() == ErrorKind::Interrupted => {}
                Err(error) => {
                    let fault = ReaderFaultKind::Io {
                        message: error.to_string(),
                    };
                    notify_fault(&sender, stream, &fault);
                    result.fault = Some(fault);
                    break;
                }
            }
        }
        if let Err(error) = file.flush() {
            if result.fault.is_none() {
                let fault = ReaderFaultKind::Io {
                    message: error.to_string(),
                };
                notify_fault(&sender, stream, &fault);
                result.fault = Some(fault);
            }
        }
        result
    }

    fn spawn_reader(
        name: &str,
        reader: Box<dyn Read + Send>,
        file: File,
        budget: u64,
        stream: StreamKind,
        sender: std::sync::mpsc::Sender<ReaderFault>,
    ) -> io::Result<JoinHandle<DrainResult>> {
        thread::Builder::new()
            .name(name.to_string())
            .spawn(move || drain_stream(reader, file, budget, stream, sender))
    }

    /// Detailed receipt from a completed [`ChildSession`] run, including
    /// failure paths where the exact child was still reaped and its output
    /// pipes were drained.
    #[derive(Debug)]
    pub struct ChildSessionReceipt {
        pub exit_code: Option<i32>,
        pub signal: Option<i32>,
        pub term_sent: bool,
        pub kill_sent: bool,
        pub stdout_bytes: u64,
        pub stderr_bytes: u64,
        pub output_drained: bool,
        pub orphan_check_passed: bool,
        pub identity: ProcessIdentity,
    }

    /// A post-spawn failure coupled to the trustworthy cleanup receipt.
    #[derive(Debug)]
    pub struct ChildSessionFailure {
        pub error: ChildSessionError,
        pub receipt: ChildSessionReceipt,
    }

    impl std::fmt::Display for ChildSessionFailure {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.error.fmt(f)
        }
    }

    impl std::error::Error for ChildSessionFailure {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            Some(&self.error)
        }
    }

    #[derive(Debug, Clone)]
    pub struct ChildSessionConfig {
        pub stdout_log: PathBuf,
        pub stderr_log: PathBuf,
        pub stdout_budget: u64,
        pub stderr_budget: u64,
        pub timeout: Duration,
        pub grace: Duration,
        pub executable_digest: String,
    }

    #[derive(Debug)]
    enum SupervisorEvent {
        Exited(ExitStatus),
        Reader(ReaderFault),
        Deadline,
    }

    #[derive(Debug)]
    struct WaitOutcome {
        status: ExitStatus,
        term_sent: bool,
        kill_sent: bool,
        failure: Option<ChildSessionError>,
    }

    pub struct ChildSession {
        child: Child,
        identity: ProcessIdentity,
        config: ChildSessionConfig,
        reader_faults: std::sync::mpsc::Receiver<ReaderFault>,
        stdout_handle: Option<JoinHandle<DrainResult>>,
        stderr_handle: Option<JoinHandle<DrainResult>>,
    }

    impl ChildSession {
        pub fn spawn(
            mut command: Command,
            config: ChildSessionConfig,
        ) -> Result<Self, ChildSessionError> {
            let stdout_file = create_log(&config.stdout_log, StreamKind::Stdout)?;
            let stderr_file = match create_log(&config.stderr_log, StreamKind::Stderr) {
                Ok(file) => file,
                Err(error) => {
                    drop(stdout_file);
                    let _ = std::fs::remove_file(&config.stdout_log);
                    return Err(error);
                }
            };
            command
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .stdin(Stdio::null());
            let mut child = command.spawn().map_err(ChildSessionError::Spawn)?;
            let pid = child.id();
            let identity = match capture_identity(pid, &config.executable_digest) {
                Ok(identity) => identity,
                Err(error) => {
                    cleanup_partial_spawn(&mut child, None, None);
                    return Err(error);
                }
            };
            let stdout = match child.stdout.take() {
                Some(stdout) => stdout,
                None => {
                    cleanup_partial_spawn(&mut child, None, None);
                    return Err(ChildSessionError::PipeUnavailable {
                        stream: StreamKind::Stdout,
                    });
                }
            };
            let stderr = match child.stderr.take() {
                Some(stderr) => stderr,
                None => {
                    cleanup_partial_spawn(&mut child, None, None);
                    return Err(ChildSessionError::PipeUnavailable {
                        stream: StreamKind::Stderr,
                    });
                }
            };
            let (sender, receiver) = std::sync::mpsc::channel();
            let stdout_handle = spawn_reader(
                "uqm-child-stdout",
                Box::new(stdout),
                stdout_file,
                config.stdout_budget,
                StreamKind::Stdout,
                sender.clone(),
            )
            .map_err(|error| {
                cleanup_partial_spawn(&mut child, None, None);
                ChildSessionError::ReaderStart {
                    stream: StreamKind::Stdout,
                    error,
                }
            })?;
            let stderr_handle = match spawn_reader(
                "uqm-child-stderr",
                Box::new(stderr),
                stderr_file,
                config.stderr_budget,
                StreamKind::Stderr,
                sender,
            ) {
                Ok(handle) => handle,
                Err(error) => {
                    cleanup_partial_spawn(&mut child, Some(stdout_handle), None);
                    return Err(ChildSessionError::ReaderStart {
                        stream: StreamKind::Stderr,
                        error,
                    });
                }
            };
            Ok(Self {
                child,
                identity,
                config,
                reader_faults: receiver,
                stdout_handle: Some(stdout_handle),
                stderr_handle: Some(stderr_handle),
            })
        }

        #[must_use]
        pub fn identity(&self) -> &ProcessIdentity {
            &self.identity
        }

        #[must_use]
        pub fn pid(&self) -> u32 {
            self.identity.pid
        }

        pub fn finish(mut self) -> Result<ChildSessionReceipt, ChildSessionFailure> {
            let outcome = match self.wait_for_event(self.config.timeout) {
                Ok(SupervisorEvent::Exited(status)) => WaitOutcome {
                    status,
                    term_sent: false,
                    kill_sent: false,
                    failure: None,
                },
                Ok(SupervisorEvent::Reader(fault)) => self.stop_and_reap(Some(fault_error(fault))),
                Ok(SupervisorEvent::Deadline) => {
                    self.stop_and_reap(Some(ChildSessionError::Timeout {
                        term_sent: true,
                        kill_sent: false,
                    }))
                }
                Err(error) => self.stop_and_reap(Some(error)),
            };
            let stdout = join_reader(self.stdout_handle.take(), StreamKind::Stdout);
            let stderr = join_reader(self.stderr_handle.take(), StreamKind::Stderr);
            let orphan_ok = !same_process_exists(&self.identity);
            let receipt = receipt_from(&self.identity, &outcome, &stdout, &stderr, orphan_ok);
            let failure = outcome
                .failure
                .or_else(|| reader_error(&stdout, StreamKind::Stdout))
                .or_else(|| reader_error(&stderr, StreamKind::Stderr))
                .or_else(|| {
                    (!orphan_ok).then_some(ChildSessionError::Orphan {
                        pid: self.identity.pid,
                    })
                });
            match failure {
                Some(error) => Err(ChildSessionFailure { error, receipt }),
                None => Ok(receipt),
            }
        }

        fn wait_for_event(
            &mut self,
            duration: Duration,
        ) -> Result<SupervisorEvent, ChildSessionError> {
            let deadline = Instant::now()
                .checked_add(duration)
                .ok_or_else(|| ChildSessionError::Wait(io::Error::other("deadline overflow")))?;
            loop {
                match self.reader_faults.try_recv() {
                    Ok(fault) => return Ok(SupervisorEvent::Reader(fault)),
                    Err(std::sync::mpsc::TryRecvError::Disconnected)
                    | Err(std::sync::mpsc::TryRecvError::Empty) => {}
                }
                match self.child.try_wait() {
                    Ok(Some(status)) => return Ok(SupervisorEvent::Exited(status)),
                    Ok(None) if Instant::now() >= deadline => return Ok(SupervisorEvent::Deadline),
                    Ok(None) => thread::sleep(Duration::from_millis(5)),
                    Err(error) if error.kind() == ErrorKind::Interrupted => {}
                    Err(error) => return Err(ChildSessionError::Wait(error)),
                }
            }
        }

        fn stop_and_reap(&mut self, mut failure: Option<ChildSessionError>) -> WaitOutcome {
            let mut term_sent = false;
            let mut kill_sent = false;
            if self.child.try_wait().ok().flatten().is_none() {
                match self.signal_exact(libc::SIGTERM) {
                    Ok(()) => term_sent = true,
                    Err(error) if failure.is_none() => failure = Some(error),
                    Err(_) => {}
                }
            }
            let status = match self.wait_for_event(self.config.grace) {
                Ok(SupervisorEvent::Exited(status)) => status,
                _ => {
                    if self.child.try_wait().ok().flatten().is_none() {
                        match self.signal_exact(libc::SIGKILL) {
                            Ok(()) => kill_sent = true,
                            Err(error) if failure.is_none() => failure = Some(error),
                            Err(_) => {}
                        }
                    }
                    match blocking_reap(&mut self.child) {
                        Ok(status) => status,
                        Err(error) => {
                            if failure.is_none() {
                                failure = Some(ChildSessionError::Wait(error));
                            }
                            synthetic_failure_status()
                        }
                    }
                }
            };
            if matches!(failure, Some(ChildSessionError::Timeout { .. })) {
                failure = Some(ChildSessionError::Timeout {
                    term_sent,
                    kill_sent,
                });
            }
            WaitOutcome {
                status,
                term_sent,
                kill_sent,
                failure,
            }
        }

        fn signal_exact(&self, signal: i32) -> Result<(), ChildSessionError> {
            match signal_pid(self.identity.pid, signal) {
                Ok(()) => Ok(()),
                Err(error) if error.raw_os_error() == Some(libc::ESRCH) => Ok(()),
                Err(error) => Err(ChildSessionError::Signal {
                    pid: self.identity.pid,
                    signal,
                    error,
                }),
            }
        }
    }

    fn create_log(path: &Path, stream: StreamKind) -> Result<File, ChildSessionError> {
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|error| ChildSessionError::LogOpen { stream, error })
    }

    fn cleanup_partial_spawn(
        child: &mut Child,
        stdout: Option<JoinHandle<DrainResult>>,
        stderr: Option<JoinHandle<DrainResult>>,
    ) {
        let _ = child.kill();
        let _ = blocking_reap(child);
        if let Some(handle) = stdout {
            let _ = handle.join();
        }
        if let Some(handle) = stderr {
            let _ = handle.join();
        }
    }

    fn blocking_reap(child: &mut Child) -> io::Result<ExitStatus> {
        loop {
            match child.wait() {
                Ok(status) => return Ok(status),
                Err(error) if error.kind() == ErrorKind::Interrupted => {}
                Err(error) => return Err(error),
            }
        }
    }

    #[cfg(unix)]
    fn synthetic_failure_status() -> ExitStatus {
        use std::os::unix::process::ExitStatusExt;
        ExitStatus::from_raw(1 << 8)
    }

    fn fault_error(fault: ReaderFault) -> ChildSessionError {
        match fault.kind {
            ReaderFaultKind::BudgetExceeded => ChildSessionError::BudgetExceeded {
                stream: fault.stream,
            },
            ReaderFaultKind::Io { message } => ChildSessionError::Reader {
                stream: fault.stream,
                error: io::Error::other(message),
            },
        }
    }

    fn join_reader(
        handle: Option<JoinHandle<DrainResult>>,
        stream: StreamKind,
    ) -> Result<DrainResult, ChildSessionError> {
        match handle {
            Some(handle) => handle
                .join()
                .map_err(|_| ChildSessionError::JoinPanic { stream }),
            None => Err(ChildSessionError::JoinPanic { stream }),
        }
    }

    fn reader_error(
        result: &Result<DrainResult, ChildSessionError>,
        stream: StreamKind,
    ) -> Option<ChildSessionError> {
        match result {
            Err(_) => Some(ChildSessionError::JoinPanic { stream }),
            Ok(result) => result
                .fault
                .clone()
                .map(|kind| fault_error(ReaderFault { stream, kind })),
        }
    }

    fn receipt_from(
        identity: &ProcessIdentity,
        outcome: &WaitOutcome,
        stdout: &Result<DrainResult, ChildSessionError>,
        stderr: &Result<DrainResult, ChildSessionError>,
        orphan_ok: bool,
    ) -> ChildSessionReceipt {
        use std::os::unix::process::ExitStatusExt;
        ChildSessionReceipt {
            exit_code: outcome.status.code(),
            signal: outcome.status.signal(),
            term_sent: outcome.term_sent,
            kill_sent: outcome.kill_sent,
            stdout_bytes: stdout.as_ref().map_or(0, |result| result.bytes_written),
            stderr_bytes: stderr.as_ref().map_or(0, |result| result.bytes_written),
            output_drained: stdout.as_ref().is_ok_and(|result| result.reached_eof)
                && stderr.as_ref().is_ok_and(|result| result.reached_eof),
            orphan_check_passed: orphan_ok,
            identity: identity.clone(),
        }
    }

    impl Drop for ChildSession {
        fn drop(&mut self) {
            if self.child.try_wait().ok().flatten().is_none() {
                let _ = signal_pid(self.identity.pid, libc::SIGTERM);
                let deadline = Instant::now()
                    .checked_add(self.config.grace)
                    .unwrap_or_else(Instant::now);
                while Instant::now() < deadline {
                    if self.child.try_wait().ok().flatten().is_some() {
                        break;
                    }
                    thread::sleep(Duration::from_millis(5));
                }
                if self.child.try_wait().ok().flatten().is_none() {
                    let _ = signal_pid(self.identity.pid, libc::SIGKILL);
                }
                let _ = blocking_reap(&mut self.child);
            }
            if let Some(handle) = self.stdout_handle.take() {
                let _ = handle.join();
            }
            if let Some(handle) = self.stderr_handle.take() {
                let _ = handle.join();
            }
        }
    }
}
#[cfg(unix)]
pub use os::parse_linux_proc_start_micros;
#[cfg(unix)]
pub use os::process_start_micros;
#[cfg(unix)]
pub use os::{
    capture_identity, ChildSession, ChildSessionConfig, ChildSessionError, ChildSessionFailure,
    ChildSessionReceipt, StreamKind,
};

// ===========================================================================
//  Unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_identity() -> ProcessIdentity {
        ProcessIdentity {
            pid: 12345,
            start_time: "2026-07-23T12:00:00Z".to_string(),
            executable_digest: "abc123".to_string(),
        }
    }

    // --- State machine (REQ-PROOF-002) ---

    #[test]
    fn state_machine_normal_flow() {
        let mut session = ChildSessionModel::new(test_identity());
        assert_eq!(session.state(), SessionState::Running);

        session.request_stop();
        assert_eq!(session.state(), SessionState::StopRequested);

        session.record_reap(0);
        assert_eq!(session.state(), SessionState::Reaped);
        assert!(session.is_reaped());

        session.close_pipes();
        assert_eq!(session.state(), SessionState::PipesClosed);

        session.join();
        assert_eq!(session.state(), SessionState::Joined);

        let result = session.complete();
        assert_eq!(session.state(), SessionState::Complete);
        assert!(session.state().is_terminal());
        assert_eq!(result, SessionResult::Complete { exit_code: 0 });
    }

    #[test]
    fn record_reap_only_once() {
        let mut session = ChildSessionModel::new(test_identity());
        session.request_stop();
        session.record_reap(0);
        assert_eq!(session.state(), SessionState::Reaped);

        // Second reap is a no-op.
        session.record_reap(1);
        assert_eq!(session.state(), SessionState::Reaped);
        assert_eq!(session.exit_code, Some(0)); // original value preserved
    }

    #[test]
    fn should_kill_only_when_stop_requested() {
        let mut session = ChildSessionModel::new(test_identity());
        assert!(!session.should_kill());

        session.request_stop();
        assert!(session.should_kill());

        session.record_reap(0);
        assert!(!session.should_kill());
    }

    #[test]
    fn state_next_transitions() {
        assert_eq!(
            SessionState::Running.next(),
            Some(SessionState::StopRequested)
        );
        assert_eq!(SessionState::Complete.next(), None);
    }

    // --- Process identity (REQ-PROOF-003) ---

    #[test]
    fn identity_matches_same() {
        let id = test_identity();
        assert!(id.matches(&id));
    }

    #[test]
    fn identity_no_match_different_pid() {
        let id1 = test_identity();
        let id2 = ProcessIdentity {
            pid: 99999,
            ..id1.clone()
        };
        assert!(!id1.matches(&id2));
    }

    #[test]
    fn identity_no_match_different_start() {
        let id1 = test_identity();
        let id2 = ProcessIdentity {
            start_time: "different".to_string(),
            ..id1.clone()
        };
        assert!(!id1.matches(&id2));
    }

    #[test]
    fn identity_no_match_different_digest() {
        let id1 = test_identity();
        let id2 = ProcessIdentity {
            executable_digest: "different".to_string(),
            ..id1.clone()
        };
        assert!(!id1.matches(&id2));
    }

    // --- Hang classification (REQ-WATCH-004) ---

    #[test]
    fn cooperative_timeout_distinct_from_hard_hang() {
        let mut session = ChildSessionModel::new(test_identity());
        session.classify_hang(HangClassification::CooperativeTimeout);
        assert_eq!(
            session.hang_classification(),
            Some(HangClassification::CooperativeTimeout)
        );
        assert_ne!(
            session.hang_classification(),
            Some(HangClassification::ParentHardHang)
        );
    }

    #[test]
    fn hard_hang_classification() {
        let mut session = ChildSessionModel::new(test_identity());
        session.classify_hang(HangClassification::ParentHardHang);
        assert_eq!(
            session.hang_classification(),
            Some(HangClassification::ParentHardHang)
        );
    }

    // --- Proof results (REQ-PROOF-001..008) ---

    #[test]
    fn passed_proof_result() {
        let result = ProofResult::passed(ProofType::MainMenu, 0);
        assert!(result.passed);
        assert_eq!(result.exit_code, Some(0));
        assert!(result.teardown_receipt_created);
        assert!(result.proof_report_created);
        assert!(result.orphan_check_passed);
    }

    #[test]
    fn failed_proof_result() {
        let result = ProofResult::failed(ProofType::HardHang, HangClassification::ParentHardHang);
        assert!(!result.passed);
        assert_eq!(
            result.hang_classification,
            Some(HangClassification::ParentHardHang)
        );
        assert!(!result.teardown_receipt_created);
    }

    #[test]
    fn watchdog_proof_type() {
        let result = ProofResult::passed(ProofType::Watchdog, 1);
        assert_eq!(result.proof_type, ProofType::Watchdog);
        assert!(result.passed);
    }

    #[test]
    fn inactive_smoke_proof_type() {
        let result = ProofResult::passed(ProofType::InactiveSmoke, 0);
        assert_eq!(result.proof_type, ProofType::InactiveSmoke);
        assert!(result.passed);
    }

    // --- REQ-PROOF-002: Kill/reap order ---

    #[test]
    fn kill_before_reap_in_failure_path() {
        let mut session = ChildSessionModel::new(test_identity());
        session.request_stop();
        assert!(session.should_kill());
        // Kill would happen here in production.
        session.record_reap(9); // killed child exits with signal-like code
        assert!(session.is_reaped());
    }

    // --- REQ-PROOF-007: Report after teardown ---

    #[test]
    fn proof_report_only_after_complete() {
        let mut session = ChildSessionModel::new(test_identity());
        session.record_reap(0);
        session.close_pipes();
        session.join();

        // Before complete(), proof report should not be created.
        assert_ne!(session.state(), SessionState::Complete);

        session.complete();
        assert_eq!(session.state(), SessionState::Complete);
        // Only now can the proof report be written.
    }

    // --- Drop is backstop only ---

    #[test]
    fn drop_is_not_explicit_finish() {
        // Explicit finish must reach Complete; Drop is only a backstop.
        let mut session = ChildSessionModel::new(test_identity());
        session.request_stop();
        session.record_reap(0);
        session.close_pipes();
        session.join();
        let result = session.complete();
        assert_eq!(result, SessionResult::Complete { exit_code: 0 });
        // In production, Drop would kill/wait/close if not Complete.
    }
}

// ===========================================================================
//  Production supervisor tests (cfg(unix))
// ===========================================================================

#[cfg(all(test, unix))]
mod os_tests {
    use super::*;
    use std::process::Command;
    use std::time::Duration;
    use tempfile::TempDir;

    #[test]
    fn linux_stat_parser_handles_spaces_parentheses_and_field_22() {
        let stat =
            "42 (worker name) with ) chars) R 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 250 20";
        assert_eq!(parse_linux_proc_start_micros(stat, 100), Some(2_500_000));
    }

    #[test]
    fn linux_stat_parser_rejects_short_invalid_and_overflowing_values() {
        assert_eq!(parse_linux_proc_start_micros("42 (x) R 1 2", 100), None);
        assert_eq!(
            parse_linux_proc_start_micros(
                "42 (x) R 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 nope",
                100
            ),
            None
        );
        let overflow = format!(
            "42 (x) R 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 {}",
            u64::MAX
        );
        assert_eq!(parse_linux_proc_start_micros(&overflow, 100), None);
        assert_eq!(parse_linux_proc_start_micros("42 (x) R", 0), None);
    }

    fn make_config(dir: &TempDir, timeout: Duration, grace: Duration) -> ChildSessionConfig {
        ChildSessionConfig {
            stdout_log: dir.path().join("out.log"),
            stderr_log: dir.path().join("err.log"),
            stdout_budget: 1 << 20,
            stderr_budget: 1 << 20,
            timeout,
            grace,
            executable_digest: "deadbeef".to_string(),
        }
    }

    #[test]
    fn normal_completion_exit_zero() {
        let dir = TempDir::new().expect("tempdir");
        let config = make_config(&dir, Duration::from_secs(5), Duration::from_secs(2));
        let cmd = Command::new("true");
        let session = ChildSession::spawn(cmd, config).expect("spawn");
        let receipt = session.finish().expect("finish");
        assert_eq!(receipt.exit_code, Some(0));
        assert!(!receipt.term_sent);
        assert!(!receipt.kill_sent);
        assert!(receipt.output_drained);
        assert!(receipt.orphan_check_passed);
    }

    #[test]
    fn normal_completion_exit_nonzero() {
        let dir = TempDir::new().expect("tempdir");
        let config = make_config(&dir, Duration::from_secs(5), Duration::from_secs(2));
        let cmd = Command::new("false");
        let session = ChildSession::spawn(cmd, config).expect("spawn");
        let receipt = session.finish().expect("finish");
        assert_eq!(receipt.exit_code, Some(1));
    }

    #[test]
    fn stdout_drained_to_file() {
        let dir = TempDir::new().expect("tempdir");
        let config = make_config(&dir, Duration::from_secs(5), Duration::from_secs(2));
        let mut cmd = Command::new("sh");
        cmd.args(["-c", "printf 'hello-stdout'"]);
        let session = ChildSession::spawn(cmd, config).expect("spawn");
        let receipt = session.finish().expect("finish");
        assert_eq!(receipt.exit_code, Some(0));
        assert_eq!(receipt.stdout_bytes, 12);
        let content = std::fs::read_to_string(dir.path().join("out.log")).expect("read log");
        assert_eq!(content, "hello-stdout");
    }

    #[test]
    fn stderr_drained_to_file() {
        let dir = TempDir::new().expect("tempdir");
        let config = make_config(&dir, Duration::from_secs(5), Duration::from_secs(2));
        let mut cmd = Command::new("sh");
        cmd.args(["-c", "printf 'hello-stderr' >&2"]);
        let session = ChildSession::spawn(cmd, config).expect("spawn");
        let receipt = session.finish().expect("finish");
        assert_eq!(receipt.exit_code, Some(0));
        assert_eq!(receipt.stderr_bytes, 12);
        let content = std::fs::read_to_string(dir.path().join("err.log")).expect("read log");
        assert_eq!(content, "hello-stderr");
    }

    #[test]
    fn stdout_and_stderr_both_drained() {
        let dir = TempDir::new().expect("tempdir");
        let config = make_config(&dir, Duration::from_secs(5), Duration::from_secs(2));
        let mut cmd = Command::new("sh");
        cmd.args(["-c", "printf 'OUT'; printf 'ERR' >&2"]);
        let session = ChildSession::spawn(cmd, config).expect("spawn");
        let receipt = session.finish().expect("finish");
        assert_eq!(receipt.stdout_bytes, 3);
        assert_eq!(receipt.stderr_bytes, 3);
    }

    #[test]
    fn timeout_sends_sigterm_then_sigkill() {
        let dir = TempDir::new().expect("tempdir");
        // Short timeout, short grace, child ignores SIGTERM.
        let config = make_config(&dir, Duration::from_millis(200), Duration::from_millis(200));
        let mut cmd = Command::new("sh");
        cmd.args(["-c", "trap '' TERM; while :; do :; done"]);
        let session = ChildSession::spawn(cmd, config).expect("spawn");
        let failure = session.finish().expect_err("expected timeout error");
        assert!(failure.receipt.output_drained);
        assert!(failure.receipt.orphan_check_passed);
        match failure.error {
            ChildSessionError::Timeout {
                term_sent,
                kill_sent,
            } => {
                assert!(term_sent, "SIGTERM should have been sent");
                assert!(kill_sent, "SIGKILL should have been sent");
            }
            other => panic!("expected Timeout, got {other:?}"),
        }
    }

    #[test]
    fn timeout_sends_sigterm_child_exits() {
        let dir = TempDir::new().expect("tempdir");
        let config = make_config(&dir, Duration::from_millis(200), Duration::from_secs(2));
        let mut cmd = Command::new("sh");
        cmd.args(["-c", "trap 'exit 42' TERM; while :; do :; done"]);
        let session = ChildSession::spawn(cmd, config).expect("spawn");
        let failure = session.finish().expect_err("expected timeout error");
        assert!(failure.receipt.output_drained);
        assert!(failure.receipt.orphan_check_passed);
        // Child exits on SIGTERM via trap → Timeout with term_sent=true.
        match failure.error {
            ChildSessionError::Timeout {
                term_sent,
                kill_sent,
            } => {
                assert!(term_sent);
                assert!(!kill_sent, "SIGKILL should not have been needed");
            }
            other => panic!("expected Timeout, got {other:?}"),
        }
    }

    #[test]
    fn byte_budget_exceeded_on_stdout() {
        let dir = TempDir::new().expect("tempdir");
        let mut config = make_config(&dir, Duration::from_secs(5), Duration::from_secs(2));
        config.stdout_budget = 4; // tiny budget
        let mut cmd = Command::new("sh");
        cmd.args(["-c", "printf 'AAAAAAAAAA'"]); // 10 bytes > 4
        let session = ChildSession::spawn(cmd, config).expect("spawn");
        let failure = session.finish().expect_err("expected budget error");
        assert!(failure.receipt.output_drained);
        assert!(failure.receipt.orphan_check_passed);
        assert_eq!(failure.receipt.stdout_bytes, 4);
        match failure.error {
            ChildSessionError::BudgetExceeded { stream } => {
                assert_eq!(stream, StreamKind::Stdout);
            }
            other => panic!("expected BudgetExceeded, got {other:?}"),
        }
    }

    #[test]
    fn byte_budget_exceeded_on_stderr() {
        let dir = TempDir::new().expect("tempdir");
        let mut config = make_config(&dir, Duration::from_secs(5), Duration::from_secs(2));
        config.stderr_budget = 4;
        let mut cmd = Command::new("sh");
        cmd.args(["-c", "printf 'BBBBBBBBBB' >&2"]);
        let session = ChildSession::spawn(cmd, config).expect("spawn");
        let failure = session.finish().expect_err("expected budget error");
        assert!(failure.receipt.output_drained);
        assert!(failure.receipt.orphan_check_passed);
        assert_eq!(failure.receipt.stderr_bytes, 4);
        match failure.error {
            ChildSessionError::BudgetExceeded { stream } => {
                assert_eq!(stream, StreamKind::Stderr);
            }
            other => panic!("expected BudgetExceeded, got {other:?}"),
        }
    }
    #[test]
    fn unbounded_writer_tiny_budget_is_stopped_reaped_and_drained_promptly() {
        let dir = TempDir::new().expect("tempdir");
        let mut config = make_config(&dir, Duration::from_secs(10), Duration::from_millis(100));
        config.stdout_budget = 16;
        let mut cmd = Command::new("sh");
        cmd.args(["-c", "while :; do printf '1234567890'; done"]);
        let started = std::time::Instant::now();
        let session = ChildSession::spawn(cmd, config).expect("spawn");
        let failure = session.finish().expect_err("budget must terminate writer");
        assert!(started.elapsed() < Duration::from_secs(2));
        assert!(matches!(
            failure.error,
            ChildSessionError::BudgetExceeded {
                stream: StreamKind::Stdout
            }
        ));
        assert_eq!(failure.receipt.stdout_bytes, 16);
        assert!(failure.receipt.term_sent || failure.receipt.kill_sent);
        assert!(failure.receipt.output_drained);
        assert!(failure.receipt.orphan_check_passed);
    }

    #[test]
    fn identity_captures_pid_and_start_time() {
        let dir = TempDir::new().expect("tempdir");
        let config = make_config(&dir, Duration::from_secs(5), Duration::from_secs(2));
        let cmd = Command::new("true");
        let session = ChildSession::spawn(cmd, config).expect("spawn");
        let id = session.identity();
        assert!(id.pid > 0);
        // start_time should be a numeric string (microseconds), not "unknown".
        assert_ne!(id.start_time, "unknown");
        assert!(
            id.start_time.parse::<u64>().is_ok(),
            "start_time should be numeric"
        );
        assert_eq!(id.executable_digest, "deadbeef");
        let _ = session.finish();
    }

    #[test]
    fn drop_backstop_kills_orphan() {
        let dir = TempDir::new().expect("tempdir");
        let config = make_config(&dir, Duration::from_secs(10), Duration::from_millis(100));
        let mut cmd = Command::new("sh");
        cmd.args(["-c", "while :; do :; done"]);
        // Intentionally do NOT call finish(); let Drop clean up.
        let session = ChildSession::spawn(cmd, config).expect("spawn");
        let pid = session.pid();
        drop(session);
        // After Drop, the child should be dead.
        // SAFETY: kill(pid, 0) existence check.
        let alive = unsafe { libc::kill(pid as libc::pid_t, 0) == 0 };
        assert!(!alive, "child should be killed by Drop backstop");
    }

    #[test]
    fn log_file_must_not_exist() {
        let dir = TempDir::new().expect("tempdir");
        let log_path = dir.path().join("out.log");
        std::fs::write(&log_path, "pre-existing").expect("write");
        let config = ChildSessionConfig {
            stdout_log: log_path,
            stderr_log: dir.path().join("err.log"),
            stdout_budget: 1 << 20,
            stderr_budget: 1 << 20,
            timeout: Duration::from_secs(5),
            grace: Duration::from_secs(2),
            executable_digest: "deadbeef".to_string(),
        };
        let cmd = Command::new("true");
        let error = match ChildSession::spawn(cmd, config) {
            Ok(session) => panic!("log setup unexpectedly spawned pid {}", session.pid()),
            Err(error) => error,
        };
        assert!(matches!(
            error,
            ChildSessionError::LogOpen {
                stream: StreamKind::Stdout,
                ..
            }
        ));
        assert!(!dir.path().join("err.log").exists());
    }

    #[test]
    fn receipt_contains_identity() {
        let dir = TempDir::new().expect("tempdir");
        let config = make_config(&dir, Duration::from_secs(5), Duration::from_secs(2));
        let cmd = Command::new("true");
        let session = ChildSession::spawn(cmd, config).expect("spawn");
        let expected_pid = session.pid();
        let receipt = session.finish().expect("finish");
        assert_eq!(receipt.identity.pid, expected_pid);
        assert_eq!(receipt.identity.executable_digest, "deadbeef");
    }
}
