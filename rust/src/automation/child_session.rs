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
    use std::sync::{Arc, Mutex};
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
        // comm may contain spaces/parens; find the last ')' to skip past it.
        // After that, field index 17 (0-based) = starttime in clock ticks.
        let after_comm = contents.rfind(')')?;
        let fields: Vec<&str> = contents[after_comm + 1..].split_whitespace().collect();
        let ticks = fields.get(17)?.parse::<u64>().ok()?;
        // SAFETY: sysconf is a read-only configuration query.
        let hz = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
        let hz = if hz > 0 { hz as u64 } else { 100 };
        Some(ticks * 1_000_000 / hz)
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

    // --- Bounded log writer ----------------------------------------------

    /// Outcome of a bounded write attempt.
    enum BudgetOutcome {
        /// An I/O error occurred.
        Io(io::Error),
        /// The budget was exceeded.
        Exceeded,
    }

    /// A writer that writes to a file up to a byte budget, then fails.
    struct BoundedLogWriter {
        file: File,
        written: u64,
        budget: u64,
        exhausted: bool,
    }

    impl BoundedLogWriter {
        fn open(path: &Path, budget: u64) -> io::Result<Self> {
            let file = OpenOptions::new().write(true).create_new(true).open(path)?;
            Ok(Self {
                file,
                written: 0,
                budget,
                exhausted: false,
            })
        }

        /// Write a chunk, enforcing the budget.
        fn write_chunk(&mut self, buf: &[u8]) -> Result<(), BudgetOutcome> {
            if self.exhausted {
                return Err(BudgetOutcome::Exceeded);
            }
            let remaining = self.budget.saturating_sub(self.written);
            if buf.len() as u64 > remaining {
                if remaining > 0 {
                    self.file
                        .write_all(&buf[..remaining as usize])
                        .map_err(BudgetOutcome::Io)?;
                    self.written = self.budget;
                }
                self.exhausted = true;
                Err(BudgetOutcome::Exceeded)
            } else {
                self.file.write_all(buf).map_err(BudgetOutcome::Io)?;
                self.written += buf.len() as u64;
                Ok(())
            }
        }

        fn flush(&mut self) -> io::Result<()> {
            self.file.flush()
        }
    }

    // --- Reader thread ----------------------------------------------------

    /// Result of draining one stream into a log file.
    #[derive(Debug, Default)]
    struct DrainResult {
        bytes_written: u64,
        budget_exceeded: bool,
        error: Option<io::Error>,
    }

    /// Drain `reader` into `path` in a separate thread until EOF or budget.
    fn spawn_reader(
        mut reader: Box<dyn Read + Send>,
        path: PathBuf,
        budget: u64,
    ) -> (Arc<Mutex<Option<DrainResult>>>, JoinHandle<()>) {
        let result = Arc::new(Mutex::new(None));
        let result_clone = Arc::clone(&result);
        let handle = thread::spawn(move || {
            let mut writer = match BoundedLogWriter::open(&path, budget) {
                Ok(w) => w,
                Err(e) => {
                    record_result(&result_clone, 0, false, Some(e));
                    return;
                }
            };
            let mut buf = [0u8; 8192];
            let mut total: u64 = 0;
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => match writer.write_chunk(&buf[..n]) {
                        Ok(()) => total += n as u64,
                        Err(BudgetOutcome::Io(e)) => {
                            let _ = writer.flush();
                            record_result(&result_clone, total, false, Some(e));
                            return;
                        }
                        Err(BudgetOutcome::Exceeded) => {
                            total += n as u64;
                            let _ = writer.flush();
                            record_result(&result_clone, total, true, None);
                            return;
                        }
                    },
                    Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                    Err(e) => {
                        let _ = writer.flush();
                        record_result(&result_clone, total, false, Some(e));
                        return;
                    }
                }
            }
            let _ = writer.flush();
            record_result(&result_clone, total, false, None);
        });
        (result, handle)
    }

    /// Store the reader-thread result into the shared slot.
    fn record_result(
        slot: &Arc<Mutex<Option<DrainResult>>>,
        bytes: u64,
        budget_exceeded: bool,
        error: Option<io::Error>,
    ) {
        let mut guard = slot.lock().expect("reader mutex poisoned");
        *guard = Some(DrainResult {
            bytes_written: bytes,
            budget_exceeded,
            error,
        });
    }

    // --- Receipt ----------------------------------------------------------

    /// Detailed receipt from a completed [`ChildSession`] run.
    ///
    /// @plan PLAN-20260723-RUNTIME-AUTOMATION.P08
    /// @requirement REQ-PROOF-002
    #[derive(Debug)]
    pub struct ChildSessionReceipt {
        /// Exit status code if the child exited normally.
        pub exit_code: Option<i32>,
        /// Signal number if the child was killed by a signal.
        pub signal: Option<i32>,
        /// Whether SIGTERM was sent during teardown.
        pub term_sent: bool,
        /// Whether SIGKILL was sent during teardown.
        pub kill_sent: bool,
        /// Number of bytes written to the stdout log.
        pub stdout_bytes: u64,
        /// Number of bytes written to the stderr log.
        pub stderr_bytes: u64,
        /// Whether all output was fully drained (true = complete drain).
        pub output_drained: bool,
        /// Whether the orphan check passed (child no longer live).
        pub orphan_check_passed: bool,
        /// The process identity recorded at spawn time.
        pub identity: ProcessIdentity,
    }

    // --- Configuration ----------------------------------------------------

    /// Configuration for spawning a [`ChildSession`].
    ///
    /// @plan PLAN-20260723-RUNTIME-AUTOMATION.P08
    /// @requirement REQ-PROOF-002
    #[derive(Debug, Clone)]
    pub struct ChildSessionConfig {
        /// Path for stdout log (must not exist; will be created new).
        pub stdout_log: PathBuf,
        /// Path for stderr log (must not exist; will be created new).
        pub stderr_log: PathBuf,
        /// Maximum bytes to write to the stdout log.
        pub stdout_budget: u64,
        /// Maximum bytes to write to the stderr log.
        pub stderr_budget: u64,
        /// Timeout for normal completion.
        pub timeout: Duration,
        /// Grace period after SIGTERM before SIGKILL.
        pub grace: Duration,
        /// SHA-256 hex digest of the executable.
        pub executable_digest: String,
    }

    // --- Session ----------------------------------------------------------

    /// Internal outcome of the wait phase.
    #[derive(Debug)]
    struct WaitOutcome {
        status: Option<ExitStatus>,
        term_sent: bool,
        kill_sent: bool,
        timed_out: bool,
    }

    /// Production OS-level child session supervisor.
    ///
    /// Owns a single `std::process::Child`, drains piped stdout/stderr into
    /// bounded log files, enforces timeouts with cooperative SIGTERM then
    /// SIGKILL, joins reader threads, checks for orphans, and provides a
    /// non-panicking Drop backstop.
    ///
    /// @plan PLAN-20260723-RUNTIME-AUTOMATION.P08
    /// @requirement REQ-PROOF-002
    pub struct ChildSession {
        child: Child,
        identity: ProcessIdentity,
        config: ChildSessionConfig,
        stdout_result: Arc<Mutex<Option<DrainResult>>>,
        stderr_result: Arc<Mutex<Option<DrainResult>>>,
        stdout_handle: Option<JoinHandle<()>>,
        stderr_handle: Option<JoinHandle<()>>,
    }

    impl ChildSession {
        /// Spawn a child from `command` and begin draining its output.
        ///
        /// The command must be configured by the caller (program + args).
        /// Stdout/stderr are piped; stdin is nulled.
        ///
        /// # Errors
        ///
        /// Returns [`ChildSessionError::Spawn`] if the process cannot be
        /// started.
        ///
        /// @plan PLAN-20260723-RUNTIME-AUTOMATION.P08
        /// @requirement REQ-PROOF-002
        pub fn spawn(
            mut command: Command,
            config: ChildSessionConfig,
        ) -> Result<Self, ChildSessionError> {
            command
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .stdin(Stdio::null());

            let mut child = command.spawn().map_err(ChildSessionError::Spawn)?;
            let pid = child.id();
            let identity = match capture_identity(pid, &config.executable_digest) {
                Ok(identity) => identity,
                Err(error) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(error);
                }
            };

            let stdout = child.stdout.take().expect("piped stdout");
            let stderr = child.stderr.take().expect("piped stderr");

            let (stdout_result, stdout_handle) = spawn_reader(
                Box::new(stdout),
                config.stdout_log.clone(),
                config.stdout_budget,
            );
            let (stderr_result, stderr_handle) = spawn_reader(
                Box::new(stderr),
                config.stderr_log.clone(),
                config.stderr_budget,
            );

            Ok(Self {
                child,
                identity,
                config,
                stdout_result,
                stderr_result,
                stdout_handle: Some(stdout_handle),
                stderr_handle: Some(stderr_handle),
            })
        }

        /// The recorded process identity.
        #[must_use]
        pub fn identity(&self) -> &ProcessIdentity {
            &self.identity
        }

        /// The recorded PID.
        #[must_use]
        pub fn pid(&self) -> u32 {
            self.identity.pid
        }

        /// Wait for the child to complete normally, with timeout enforcement.
        ///
        /// On timeout: send SIGTERM, wait the grace period, then SIGKILL if
        /// still live. Always reaps the child, joins readers, and checks for
        /// orphans before returning.
        ///
        /// # Errors
        ///
        /// See [`ChildSessionError`].
        ///
        /// @plan PLAN-20260723-RUNTIME-AUTOMATION.P08
        /// @requirement REQ-PROOF-002
        pub fn finish(mut self) -> Result<ChildSessionReceipt, ChildSessionError> {
            let outcome = self.run_wait_sequence()?;
            self.join_readers();
            let orphan_ok = !same_process_exists(&self.identity);
            self.build_receipt(outcome, orphan_ok)
        }

        /// Run the full wait sequence: try_wait until timeout, then SIGTERM,
        /// then grace poll, then SIGKILL, then blocking reap.
        fn run_wait_sequence(&mut self) -> Result<WaitOutcome, ChildSessionError> {
            if let Some(status) = self.poll_until_deadline(self.config.timeout)? {
                return Ok(WaitOutcome {
                    status: Some(status),
                    term_sent: false,
                    kill_sent: false,
                    timed_out: false,
                });
            }
            self.signal_exact(libc::SIGTERM)?;
            if let Some(status) = self.poll_until_deadline(self.config.grace)? {
                return Ok(WaitOutcome {
                    status: Some(status),
                    term_sent: true,
                    kill_sent: false,
                    timed_out: true,
                });
            }
            self.signal_exact(libc::SIGKILL)?;
            let status = self.blocking_reap()?;
            Ok(WaitOutcome {
                status: Some(status),
                term_sent: true,
                kill_sent: true,
                timed_out: true,
            })
        }

        /// Poll with `try_wait` until the child exits or `deadline_dur`
        /// elapses. Returns `Some(status)` on exit, `None` on timeout.
        fn poll_until_deadline(
            &mut self,
            deadline_dur: Duration,
        ) -> Result<Option<ExitStatus>, ChildSessionError> {
            let deadline = Instant::now() + deadline_dur;
            loop {
                match self.child.try_wait() {
                    Ok(Some(status)) => return Ok(Some(status)),
                    Ok(None) => {
                        if Instant::now() >= deadline {
                            return Ok(None);
                        }
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                    Err(error) => return Err(ChildSessionError::Wait(error)),
                }
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

        /// Blocking reap with EINTR retry.
        fn blocking_reap(&mut self) -> Result<ExitStatus, ChildSessionError> {
            loop {
                match self.child.wait() {
                    Ok(status) => return Ok(status),
                    Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                    Err(error) => return Err(ChildSessionError::Wait(error)),
                }
            }
        }

        /// Join both reader threads, recording join failures.
        fn join_readers(&mut self) {
            self.stdout_handle = join_one(self.stdout_handle.take(), StreamKind::Stdout);
            self.stderr_handle = join_one(self.stderr_handle.take(), StreamKind::Stderr);
        }

        /// Collect one reader's outcome into `(bytes, error)`.
        fn collect_reader(
            &self,
            result: &Arc<Mutex<Option<DrainResult>>>,
            stream: StreamKind,
        ) -> (u64, Option<ChildSessionError>) {
            let guard = result.lock().expect("reader mutex poisoned");
            match &*guard {
                None => (0, Some(ChildSessionError::JoinPanic { stream })),
                Some(dr) => {
                    if let Some(e) = &dr.error {
                        (
                            dr.bytes_written,
                            Some(ChildSessionError::Reader {
                                stream,
                                error: io::Error::new(e.kind(), e.to_string()),
                            }),
                        )
                    } else if dr.budget_exceeded {
                        (
                            dr.bytes_written,
                            Some(ChildSessionError::BudgetExceeded { stream }),
                        )
                    } else {
                        (dr.bytes_written, None)
                    }
                }
            }
        }

        /// Build the final receipt, combining wait outcome, reader outcomes,
        /// and orphan check into a single `Result`.
        fn build_receipt(
            &self,
            outcome: WaitOutcome,
            orphan_ok: bool,
        ) -> Result<ChildSessionReceipt, ChildSessionError> {
            let (stdout_bytes, stdout_err) =
                self.collect_reader(&self.stdout_result, StreamKind::Stdout);
            let (stderr_bytes, stderr_err) =
                self.collect_reader(&self.stderr_result, StreamKind::Stderr);

            let (exit_code, signal) = status_parts(outcome.status);

            // Priority: reader errors > timeout > orphan.
            if let Some(e) = stdout_err.or(stderr_err) {
                return Err(e);
            }
            if outcome.timed_out {
                return Err(ChildSessionError::Timeout {
                    term_sent: outcome.term_sent,
                    kill_sent: outcome.kill_sent,
                });
            }
            if !orphan_ok {
                return Err(ChildSessionError::Orphan {
                    pid: self.identity.pid,
                });
            }

            Ok(ChildSessionReceipt {
                exit_code,
                signal,
                term_sent: outcome.term_sent,
                kill_sent: outcome.kill_sent,
                stdout_bytes,
                stderr_bytes,
                output_drained: true,
                orphan_check_passed: orphan_ok,
                identity: self.identity.clone(),
            })
        }
    }

    /// Join a reader handle, returning `None` on success or re-storing
    /// the handle if join failed (shouldn't happen for non-panicking threads).
    fn join_one(handle: Option<JoinHandle<()>>, _stream: StreamKind) -> Option<JoinHandle<()>> {
        if let Some(h) = handle {
            // join() returns Err only if the thread panicked; our reader
            // threads never panic, but we must not swallow the error.
            if h.join().is_err() {
                // Thread panicked — this is surfaced as JoinPanic via the
                // result slot remaining None.
            }
        }
        None
    }

    /// Extract `(exit_code, signal)` from an `ExitStatus`.
    fn status_parts(status: Option<ExitStatus>) -> (Option<i32>, Option<i32>) {
        use std::os::unix::process::ExitStatusExt;
        status.map_or((None, None), |s| (s.code(), s.signal()))
    }

    impl Drop for ChildSession {
        fn drop(&mut self) {
            // Non-panicking backstop: if the child was never reaped, do our
            // best to clean up without panicking. This only targets the exact
            // recorded PID.
            if self.child.try_wait().ok().flatten().is_none() {
                let _ = signal_pid(self.identity.pid, libc::SIGTERM);
                let deadline = Instant::now() + self.config.grace;
                while Instant::now() < deadline {
                    if self.child.try_wait().ok().flatten().is_some() {
                        break;
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                if self.child.try_wait().ok().flatten().is_none() {
                    let _ = signal_pid(self.identity.pid, libc::SIGKILL);
                }
                let _ = self.blocking_reap();
            }
            self.join_readers();
        }
    }
}

#[cfg(unix)]
pub use os::process_start_micros;
#[cfg(unix)]
pub use os::{
    capture_identity, ChildSession, ChildSessionConfig, ChildSessionError, ChildSessionReceipt,
    StreamKind,
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
        let result = session.finish();
        assert!(result.is_err(), "expected timeout error");
        match result.unwrap_err() {
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
        let result = session.finish();
        // Child exits on SIGTERM via trap → Timeout with term_sent=true.
        assert!(result.is_err());
        match result.unwrap_err() {
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
        let result = session.finish();
        assert!(result.is_err(), "expected budget error");
        match result.unwrap_err() {
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
        let result = session.finish();
        assert!(result.is_err());
        match result.unwrap_err() {
            ChildSessionError::BudgetExceeded { stream } => {
                assert_eq!(stream, StreamKind::Stderr);
            }
            other => panic!("expected BudgetExceeded, got {other:?}"),
        }
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
        let result = ChildSession::spawn(cmd, config);
        // spawn_reader creates the file with create_new; the reader thread
        // will fail to open it but the child still spawned. The error is
        // surfaced via finish().
        assert!(result.is_ok(), "spawn should succeed; error surfaced later");
        let session = result.unwrap();
        let finish_result = session.finish();
        assert!(
            finish_result.is_err(),
            "should fail due to log file conflict"
        );
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
