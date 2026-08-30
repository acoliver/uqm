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
//! bounded log files, records PID/start-time identity, and owns a single
//! wait/reap path. The exact child owns a new session when standalone or a new
//! process group when nested under a containment monitor. That group receives
//! cooperative SIGTERM then SIGKILL during teardown. The supervisor verifies
//! that the group is empty, bounds reader completion,
//! and provides a non-panicking Drop backstop.
//!
//! @plan PLAN-20260723-RUNTIME-AUTOMATION.P08
//! @requirement REQ-PROOF-002

use std::path::PathBuf;

#[cfg(unix)]
use std::io;
#[cfg(unix)]
use std::os::fd::RawFd;

/// Environment variable carrying the full-duplex nested-group registration descriptor.
#[cfg(unix)]
pub const NESTED_GROUP_REGISTRATION_FD_ENV: &str = "UQM_CI_NESTED_GROUP_FD";
/// Environment variable carrying the serialization-token read descriptor.
#[cfg(unix)]
pub const NESTED_GROUP_TOKEN_READ_FD_ENV: &str = "UQM_CI_NESTED_GROUP_TOKEN_READ_FD";
/// Environment variable carrying the serialization-token write descriptor.
#[cfg(unix)]
pub const NESTED_GROUP_TOKEN_WRITE_FD_ENV: &str = "UQM_CI_NESTED_GROUP_TOKEN_WRITE_FD";

/// Operation in the fixed nested process-group registration protocol.
#[cfg(unix)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum NestedGroupOperation {
    Register = b'+',
    Query = b'?',
    Unregister = b'-',
}

#[cfg(unix)]
impl TryFrom<u8> for NestedGroupOperation {
    type Error = io::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            b'+' => Ok(Self::Register),
            b'?' => Ok(Self::Query),
            b'-' => Ok(Self::Unregister),
            _ => Err(io::Error::from_raw_os_error(libc::EPROTO)),
        }
    }
}

/// A decoded request from the fixed nested process-group protocol.
#[cfg(unix)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NestedGroupRequest {
    pub operation: NestedGroupOperation,
    pub pid: libc::pid_t,
}

/// Inherited descriptors for token-serialized containment-monitor exchanges.
#[cfg(unix)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NestedGroupProtocol {
    registration: RawFd,
    token_read: RawFd,
    token_write: RawFd,
}

#[cfg(unix)]
impl NestedGroupProtocol {
    #[must_use]
    pub const fn new(registration: RawFd, token_read: RawFd, token_write: RawFd) -> Self {
        Self {
            registration,
            token_read,
            token_write,
        }
    }

    /// Parse the descriptor contract. It must be entirely present or absent.
    pub fn inherited() -> io::Result<Option<Self>> {
        let names = [
            NESTED_GROUP_REGISTRATION_FD_ENV,
            NESTED_GROUP_TOKEN_READ_FD_ENV,
            NESTED_GROUP_TOKEN_WRITE_FD_ENV,
        ];
        let values = names.map(std::env::var_os);
        if values.iter().all(Option::is_none) {
            return Ok(None);
        }
        let mut descriptors = [0; 3];
        for (index, value) in values.into_iter().enumerate() {
            let value = value.ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        "incomplete nested-group descriptor contract: missing {}",
                        names[index]
                    ),
                )
            })?;
            let value = value.into_string().map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("{} is not UTF-8", names[index]),
                )
            })?;
            descriptors[index] = value.parse::<RawFd>().map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("{} is not a descriptor", names[index]),
                )
            })?;
            if descriptors[index] < 0 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("{} is negative", names[index]),
                ));
            }
        }
        Ok(Some(Self::new(
            descriptors[0],
            descriptors[1],
            descriptors[2],
        )))
    }

    pub fn apply_environment(self, command: &mut std::process::Command) {
        command
            .env(
                NESTED_GROUP_REGISTRATION_FD_ENV,
                self.registration.to_string(),
            )
            .env(NESTED_GROUP_TOKEN_READ_FD_ENV, self.token_read.to_string())
            .env(
                NESTED_GROUP_TOKEN_WRITE_FD_ENV,
                self.token_write.to_string(),
            );
    }

    #[must_use]
    pub const fn descriptors(self) -> [RawFd; 3] {
        [self.registration, self.token_read, self.token_write]
    }

    /// Clear close-on-exec on every protocol descriptor.
    ///
    /// This performs only `fcntl`, so it is suitable for `pre_exec`.
    pub fn make_inheritable(self) -> io::Result<()> {
        for descriptor in self.descriptors() {
            // SAFETY: each descriptor belongs to the inherited protocol.
            if unsafe { libc::fcntl(descriptor, libc::F_SETFD, 0) } == -1 {
                return Err(io::Error::last_os_error());
            }
        }
        Ok(())
    }

    /// Perform one token-serialized request/acknowledgment exchange.
    ///
    /// The I/O path uses deadline-bounded `clock_gettime`, `poll`, `read`, and
    /// `write` calls, so a stalled monitor cannot block `pre_exec` indefinitely.
    pub fn exchange(
        self,
        operation: NestedGroupOperation,
        pid: libc::pid_t,
    ) -> io::Result<libc::pid_t> {
        self.exchange_with_timeout(operation, pid, std::time::Duration::from_secs(5))
    }

    /// Perform one exchange within a single timeout budget.
    pub fn exchange_with_timeout(
        self,
        operation: NestedGroupOperation,
        pid: libc::pid_t,
        timeout: std::time::Duration,
    ) -> io::Result<libc::pid_t> {
        let started = monotonic_millis()?;
        let budget = duration_millis(timeout);
        let deadline = started.saturating_add(budget);
        let release_reserve = (budget / 4).clamp(1, 100).min(budget);
        let operation_deadline = deadline.saturating_sub(release_reserve);
        let mut token = [0_u8; 1];
        fd_io_until(self.token_read, &mut token, false, operation_deadline)?;
        let result = exchange_locked_until(self.registration, operation, pid, operation_deadline);
        let release = fd_io_until(self.token_write, &mut token, true, deadline);
        match (result, release) {
            (Ok(anchor), Ok(())) => Ok(anchor),
            (Err(error), _) | (Ok(_), Err(error)) => Err(error),
        }
    }
}

#[cfg(unix)]
fn duration_millis(duration: std::time::Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

#[cfg(unix)]
fn monotonic_millis() -> io::Result<u64> {
    let mut value = std::mem::MaybeUninit::<libc::timespec>::uninit();
    // SAFETY: value points to writable timespec storage.
    if unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, value.as_mut_ptr()) } == -1 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: clock_gettime initialized value on success.
    let value = unsafe { value.assume_init() };
    let seconds =
        u64::try_from(value.tv_sec).map_err(|_| io::Error::from_raw_os_error(libc::EOVERFLOW))?;
    let nanos =
        u64::try_from(value.tv_nsec).map_err(|_| io::Error::from_raw_os_error(libc::EOVERFLOW))?;
    seconds
        .checked_mul(1_000)
        .and_then(|millis| millis.checked_add(nanos / 1_000_000))
        .ok_or_else(|| io::Error::from_raw_os_error(libc::EOVERFLOW))
}

#[cfg(unix)]
fn fd_io(fd: RawFd, bytes: &mut [u8], write: bool) -> io::Result<()> {
    let deadline = monotonic_millis()?.saturating_add(5_000);
    fd_io_until(fd, bytes, write, deadline)
}

#[cfg(unix)]
fn fd_io_until(fd: RawFd, bytes: &mut [u8], write: bool, deadline: u64) -> io::Result<()> {
    let mut offset = 0;
    while offset < bytes.len() {
        let now = monotonic_millis()?;
        if now >= deadline {
            return Err(io::Error::from_raw_os_error(libc::ETIMEDOUT));
        }
        let remaining = i32::try_from(deadline - now).unwrap_or(i32::MAX);
        let mut descriptor = libc::pollfd {
            fd,
            events: if write { libc::POLLOUT } else { libc::POLLIN },
            revents: 0,
        };
        // SAFETY: descriptor points to one initialized pollfd.
        let ready = unsafe { libc::poll(&mut descriptor, 1, remaining) };
        if ready == 0 {
            return Err(io::Error::from_raw_os_error(libc::ETIMEDOUT));
        }
        if ready == -1 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(error);
        }
        if descriptor.revents & (libc::POLLERR | libc::POLLNVAL) != 0
            || descriptor.revents & descriptor.events == 0
        {
            return Err(io::Error::from_raw_os_error(libc::EPIPE));
        }
        // SAFETY: bytes is valid for the operation and fd is its protocol endpoint.
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
            offset += result as usize;
        } else if result == -1 && io::Error::last_os_error().kind() == io::ErrorKind::Interrupted {
            continue;
        } else if result == 0 {
            return Err(io::Error::from_raw_os_error(libc::EPIPE));
        } else {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}

#[cfg(unix)]
fn exchange_locked_until(
    registration_fd: RawFd,
    operation: NestedGroupOperation,
    pid: libc::pid_t,
    deadline: u64,
) -> io::Result<libc::pid_t> {
    let mut request = [0_u8; 8];
    request[0] = operation as u8;
    request[4..8].copy_from_slice(&pid.to_ne_bytes());
    fd_io_until(registration_fd, &mut request, true, deadline)?;
    let mut response = [0_u8; 12];
    fd_io_until(registration_fd, &mut response, false, deadline)?;
    let status = i32::from_ne_bytes([response[0], response[1], response[2], response[3]]);
    let response_pid = i32::from_ne_bytes([response[4], response[5], response[6], response[7]]);
    let anchor_pid = i32::from_ne_bytes([response[8], response[9], response[10], response[11]]);
    if response_pid != pid
        || status != 0
        || (operation != NestedGroupOperation::Unregister && anchor_pid <= 0)
    {
        return Err(io::Error::from_raw_os_error(libc::EPROTO));
    }
    Ok(anchor_pid)
}

/// Read and validate one fixed-size registration request.
#[cfg(unix)]
pub fn read_nested_group_request(fd: RawFd) -> io::Result<NestedGroupRequest> {
    let mut request = [0_u8; 8];
    fd_io(fd, &mut request, false)?;
    if request[1..4] != [0; 3] {
        return Err(io::Error::from_raw_os_error(libc::EPROTO));
    }
    Ok(NestedGroupRequest {
        operation: NestedGroupOperation::try_from(request[0])?,
        pid: i32::from_ne_bytes([request[4], request[5], request[6], request[7]]),
    })
}

/// Write one fixed-size registration acknowledgment.
#[cfg(unix)]
pub fn write_nested_group_response(
    fd: RawFd,
    status: i32,
    pid: libc::pid_t,
    anchor: libc::pid_t,
) -> io::Result<()> {
    let mut response = [0_u8; 12];
    response[..4].copy_from_slice(&status.to_ne_bytes());
    response[4..8].copy_from_slice(&pid.to_ne_bytes());
    response[8..12].copy_from_slice(&anchor.to_ne_bytes());
    fd_io(fd, &mut response, true)
}
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
// identity, owns a single wait/reap path, and supervises the new process group
// rooted at that child through reader completion and orphan verification.

#[cfg(unix)]
mod os {
    #[cfg(target_os = "macos")]
    use std::ffi::{c_int, c_void};
    use std::fs::{File, OpenOptions};
    use std::io::{self, ErrorKind, Read, Write};
    use std::os::unix::process::CommandExt;
    use std::path::{Path, PathBuf};
    use std::process::{Child, Command, ExitStatus, Stdio};
    use std::thread::{self, JoinHandle};
    use std::time::{Duration, Instant};

    use super::{NestedGroupOperation, NestedGroupProtocol, ProcessIdentity};

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
        /// A reader did not complete within the bounded teardown interval.
        ReaderCompletionTimeout { stream: StreamKind },
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
        /// Signaling the owned process group failed for a reason other than it exiting.
        Signal {
            pid: u32,
            signal: i32,
            error: io::Error,
        },
        /// Parent-side observation of the exact child failed.
        Observer(io::Error),
        /// Registering or unregistering the nested process group failed.
        Registration(io::Error),
        /// Checking the owned process group failed.
        ProcessGroup(io::Error),
        /// Waiting for the exact child failed.
        Wait(io::Error),
        /// The exact child did not become waitable before the reap deadline.
        ReapTimeout { pid: u32 },
        /// The process anchor entered an impossible cleanup transition.
        CleanupState { detail: &'static str },
        /// The owned process group still had members after teardown.
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
                Self::ReaderCompletionTimeout { stream } => {
                    write!(f, "reader completion timed out on {stream:?}")
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
                    write!(f, "signal {signal} failed for process group {pid}: {error}")
                }
                Self::Observer(error) => write!(f, "child observation failed: {error}"),
                Self::Registration(error) => {
                    write!(f, "nested process-group registration failed: {error}")
                }
                Self::ProcessGroup(error) => {
                    write!(f, "process group observation failed: {error}")
                }
                Self::Wait(error) => write!(f, "wait failed: {error}"),
                Self::ReapTimeout { pid } => {
                    write!(f, "child reap deadline expired for pid {pid}")
                }
                Self::CleanupState { detail } => {
                    write!(f, "invalid child cleanup state: {detail}")
                }
                Self::Orphan { pid } => write!(f, "process group still has members: pgid {pid}"),
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
                | Self::Observer(e)
                | Self::Registration(e)
                | Self::ProcessGroup(e)
                | Self::Wait(e) => Some(e),
                _ => None,
            }
        }
    }

    // --- Process identity / start-time -----------------------------------

    /// Capture the process start time (wall-clock microseconds since epoch)
    /// for a PID on macOS from the `KERN_PROC_PID` process record.
    ///
    /// `proc_pidinfo` answers `ESRCH` the moment a child exits, even while its
    /// parent still holds it unreaped, so it cannot identify a short-lived
    /// child. The kernel process record still reports the exact start time of
    /// an exited but unreaped process, and it begins with that `timeval`.
    ///
    /// Returns `None` if the information is unavailable (e.g. insufficient
    /// privileges or the process has been reaped).
    ///
    /// @plan PLAN-20260723-RUNTIME-AUTOMATION.P08
    /// @requirement REQ-PROOF-003
    #[cfg(target_os = "macos")]
    #[must_use]
    pub fn process_start_micros(pid: u32) -> Option<u64> {
        #[repr(C)]
        #[derive(Clone, Copy)]
        struct StartTimeval {
            seconds: i64,
            microseconds: i32,
            _padding: i32,
        }

        let mut mib: [c_int; 4] = [
            libc::CTL_KERN,
            libc::KERN_PROC,
            libc::KERN_PROC_PID,
            c_int::try_from(pid).ok()?,
        ];
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
        if sized != 0 || required < std::mem::size_of::<StartTimeval>() {
            return None;
        }
        let mut record = vec![0_u8; required];
        let mut written = required;
        // SAFETY: record is writable for written bytes, and the kernel reports
        // the byte count it produced through written.
        let read = unsafe {
            libc::sysctl(
                mib.as_mut_ptr(),
                mib.len() as u32,
                record.as_mut_ptr().cast::<c_void>(),
                &raw mut written,
                std::ptr::null_mut(),
                0,
            )
        };
        if read != 0 || written < std::mem::size_of::<StartTimeval>() {
            return None;
        }
        let mut start = StartTimeval {
            seconds: 0,
            microseconds: 0,
            _padding: 0,
        };
        // SAFETY: the record begins with the process start timeval and holds at
        // least that many initialized bytes, and both operands are plain data.
        unsafe {
            std::ptr::copy_nonoverlapping(
                record.as_ptr(),
                (&raw mut start).cast::<u8>(),
                std::mem::size_of::<StartTimeval>(),
            );
        }
        u64::try_from(start.seconds)
            .ok()?
            .checked_mul(1_000_000)?
            .checked_add(u64::try_from(start.microseconds).ok()?)
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

    #[derive(Debug)]
    pub(super) struct LeaderAnchor {
        pid: libc::pid_t,
        monitor_anchor_pid: Option<libc::pid_t>,
        observed: bool,
        reaped: bool,
    }

    impl LeaderAnchor {
        pub(super) fn new(
            pid: u32,
            monitor_anchor_pid: Option<libc::pid_t>,
        ) -> Result<Self, ChildSessionError> {
            Ok(Self {
                pid: libc::pid_t::try_from(pid).map_err(|_| {
                    ChildSessionError::Wait(io::Error::other("child PID does not fit pid_t"))
                })?,
                monitor_anchor_pid,
                observed: false,
                reaped: false,
            })
        }

        pub(super) fn observe(&mut self) -> io::Result<bool> {
            if self.reaped {
                return Err(io::Error::other("cannot observe a reaped leader anchor"));
            }
            if self.observed {
                return Ok(true);
            }
            let mut info = std::mem::MaybeUninit::<libc::siginfo_t>::zeroed();
            // SAFETY: info points to writable siginfo_t storage. WNOWAIT leaves
            // the exact child waitable, so its PID and process-group identity
            // cannot be reused before group cleanup is complete.
            let result = unsafe {
                libc::waitid(
                    libc::P_PID,
                    self.pid as libc::id_t,
                    info.as_mut_ptr(),
                    libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
                )
            };
            if result == -1 {
                return Err(io::Error::last_os_error());
            }
            // SAFETY: waitid initialized info on success.
            let info = unsafe { info.assume_init() };
            if unsafe { info.si_pid() } != 0 {
                self.observed = true;
            }
            Ok(self.observed)
        }

        fn permits_group_operation(&self) -> bool {
            !self.reaped
        }

        fn mark_reaped(&mut self) -> Result<(), ChildSessionError> {
            if self.reaped {
                return Err(ChildSessionError::CleanupState {
                    detail: "leader may be reaped only once",
                });
            }
            self.reaped = true;
            Ok(())
        }

        fn ignored_group_members(&self) -> [libc::pid_t; 2] {
            [self.pid, self.monitor_anchor_pid.unwrap_or(0)]
        }
    }

    #[cfg(target_os = "macos")]
    fn macos_process_is_terminal(pid: libc::pid_t) -> io::Result<bool> {
        let mut info = std::mem::MaybeUninit::<libc::proc_bsdinfo>::zeroed();
        let expected = i32::try_from(std::mem::size_of::<libc::proc_bsdinfo>())
            .map_err(|_| io::Error::other("process-info size does not fit c_int"))?;
        // SAFETY: info is writable for expected bytes and pid came from proc_listpids.
        let written = unsafe {
            libc::proc_pidinfo(
                pid,
                libc::PROC_PIDTBSDINFO,
                0,
                info.as_mut_ptr().cast(),
                expected,
            )
        };
        if written == expected {
            // SAFETY: proc_pidinfo initialized the complete structure.
            return Ok(unsafe { info.assume_init() }.pbi_status == libc::SZOMB);
        }
        if written == 0 {
            let output = Command::new("/bin/ps")
                .args(["-o", "state=", "-p", &pid.to_string()])
                .output()
                .map_err(|error| {
                    io::Error::other(format!(
                        "cannot inspect process {pid} state with ps: {error}"
                    ))
                })?;
            if output.status.success() {
                let state = std::str::from_utf8(&output.stdout)
                    .map_err(|error| {
                        io::Error::other(format!("process {pid} state is not UTF-8: {error}"))
                    })?
                    .trim();
                if let Some(state) = state.as_bytes().first() {
                    return Ok(*state == b'Z');
                }
            }
            // SAFETY: signal zero checks whether the listed PID still exists.
            if unsafe { libc::kill(pid, 0) } == -1
                && io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
            {
                return Ok(true);
            }
        }
        Err(io::Error::other(format!(
            "cannot inspect process {pid}: proc_pidinfo returned {written} bytes, expected {expected}"
        )))
    }

    #[cfg(target_os = "macos")]
    fn process_group_has_other_members(
        process_group: u32,
        ignored: &[libc::pid_t],
    ) -> io::Result<bool> {
        const PROC_PGRP_ONLY: u32 = 2;
        // Query and retry if the process table grows between calls.
        for _ in 0..3 {
            // SAFETY: a null buffer requests the required byte count.
            let required = unsafe {
                libc::proc_listpids(PROC_PGRP_ONLY, process_group, std::ptr::null_mut(), 0)
            };
            if required < 0 {
                return Err(io::Error::last_os_error());
            }
            let slots = usize::try_from(required)
                .ok()
                .and_then(|bytes| bytes.checked_div(std::mem::size_of::<libc::pid_t>()))
                .and_then(|count| count.checked_add(16))
                .ok_or_else(|| io::Error::other("process-group member count overflow"))?;
            let mut pids = vec![0 as libc::pid_t; slots];
            let bytes = i32::try_from(pids.len() * std::mem::size_of::<libc::pid_t>())
                .map_err(|_| io::Error::other("process-group buffer does not fit c_int"))?;
            // SAFETY: pids is writable for exactly bytes bytes.
            let written = unsafe {
                libc::proc_listpids(
                    PROC_PGRP_ONLY,
                    process_group,
                    pids.as_mut_ptr().cast(),
                    bytes,
                )
            };
            if written < 0 {
                return Err(io::Error::last_os_error());
            }
            if written < bytes {
                let count = usize::try_from(written)
                    .ok()
                    .and_then(|value| value.checked_div(std::mem::size_of::<libc::pid_t>()))
                    .ok_or_else(|| io::Error::other("invalid process-group byte count"))?;
                for pid in pids[..count]
                    .iter()
                    .copied()
                    .filter(|pid| *pid > 0 && !ignored.contains(pid))
                {
                    if !macos_process_is_terminal(pid)? {
                        return Ok(true);
                    }
                }
                return Ok(false);
            }
        }
        Err(io::Error::other(
            "process-group membership changed during every inspection",
        ))
    }

    #[cfg(target_os = "linux")]
    pub(super) fn linux_stat_is_live_group_member(stat: &str, process_group: u32) -> bool {
        let Some(after_comm) = stat.rfind(')') else {
            return false;
        };
        let mut fields = stat[after_comm + 1..].split_whitespace();
        let state = fields.next();
        let pgrp = fields.nth(1).and_then(|field| field.parse::<u32>().ok());
        state != Some("Z") && pgrp == Some(process_group)
    }

    #[cfg(target_os = "linux")]
    fn process_group_has_other_members(
        process_group: u32,
        ignored: &[libc::pid_t],
    ) -> io::Result<bool> {
        for entry in std::fs::read_dir("/proc")? {
            let entry = entry?;
            let Some(pid) = entry
                .file_name()
                .to_str()
                .and_then(|name| name.parse::<u32>().ok())
            else {
                continue;
            };
            if ignored
                .iter()
                .any(|ignored| u32::try_from(*ignored).ok() == Some(pid))
            {
                continue;
            }
            let Ok(stat) = std::fs::read_to_string(entry.path().join("stat")) else {
                continue;
            };
            if linux_stat_is_live_group_member(&stat, process_group) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn signal_process_group(anchor: &LeaderAnchor, signal: libc::c_int) -> io::Result<()> {
        assert!(
            anchor.permits_group_operation(),
            "process-group signal requires an unreaped leader anchor"
        );
        // SAFETY: the unreaped exact leader prevents reuse of this process-group ID.
        let rc = unsafe { libc::kill(-anchor.pid, signal) };
        if rc == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
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
    pub(super) struct DrainResult {
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
        Exited,
        Reader(ReaderFault),
        Deadline,
    }

    #[derive(Debug)]
    struct WaitOutcome {
        status: ExitStatus,
        term_sent: bool,
        kill_sent: bool,
        group_empty: bool,
        failure: Option<ChildSessionError>,
    }

    pub struct ChildSession {
        child: Child,
        anchor: LeaderAnchor,
        identity: ProcessIdentity,
        config: ChildSessionConfig,
        protocol: Option<NestedGroupProtocol>,
        registration_active: bool,
        cleanup_complete: bool,
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
            let protocol =
                NestedGroupProtocol::inherited().map_err(ChildSessionError::Registration)?;
            // SAFETY: setsid, setpgid, fcntl, read, write, and getpid are
            // async-signal-safe. A nested group stays in the monitor's session
            // so its stable anchor can join; an unmonitored child owns a session.
            unsafe {
                command.pre_exec(move || {
                    if let Some(protocol) = protocol {
                        if libc::setpgid(0, 0) == -1 {
                            return Err(io::Error::last_os_error());
                        }
                        protocol.make_inheritable()?;
                        protocol.exchange(NestedGroupOperation::Register, libc::getpid())?;
                    } else if libc::setsid() == -1 {
                        return Err(io::Error::last_os_error());
                    }
                    Ok(())
                });
            }
            let mut child = command.spawn().map_err(ChildSessionError::Spawn)?;
            let pid = child.id();
            let monitor_anchor_pid = match protocol {
                Some(protocol) => match libc::pid_t::try_from(pid)
                    .map_err(|_| io::Error::other("child PID does not fit pid_t"))
                    .and_then(|pid| protocol.exchange(NestedGroupOperation::Query, pid))
                {
                    Ok(anchor_pid) => Some(anchor_pid),
                    Err(error) => {
                        let mut anchor = LeaderAnchor::new(pid, None)?;
                        let cleanup = cleanup_partial_spawn(
                            &mut child,
                            &mut anchor,
                            Some(protocol),
                            config.grace,
                            None,
                            None,
                        );
                        return Err(cleanup
                            .err()
                            .unwrap_or(ChildSessionError::Registration(error)));
                    }
                },
                None => None,
            };
            let mut anchor = LeaderAnchor::new(pid, monitor_anchor_pid)?;
            let identity = match capture_identity(pid, &config.executable_digest) {
                Ok(identity) => identity,
                Err(error) => {
                    let cleanup = cleanup_partial_spawn(
                        &mut child,
                        &mut anchor,
                        protocol,
                        config.grace,
                        None,
                        None,
                    );
                    return Err(cleanup.err().unwrap_or(error));
                }
            };
            let stdout = match child.stdout.take() {
                Some(stdout) => stdout,
                None => {
                    let cleanup = cleanup_partial_spawn(
                        &mut child,
                        &mut anchor,
                        protocol,
                        config.grace,
                        None,
                        None,
                    );
                    return Err(cleanup.err().unwrap_or(ChildSessionError::PipeUnavailable {
                        stream: StreamKind::Stdout,
                    }));
                }
            };
            let stderr = match child.stderr.take() {
                Some(stderr) => stderr,
                None => {
                    let cleanup = cleanup_partial_spawn(
                        &mut child,
                        &mut anchor,
                        protocol,
                        config.grace,
                        None,
                        None,
                    );
                    return Err(cleanup.err().unwrap_or(ChildSessionError::PipeUnavailable {
                        stream: StreamKind::Stderr,
                    }));
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
                let original = ChildSessionError::ReaderStart {
                    stream: StreamKind::Stdout,
                    error,
                };
                cleanup_partial_spawn(&mut child, &mut anchor, protocol, config.grace, None, None)
                    .err()
                    .unwrap_or(original)
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
                    let original = ChildSessionError::ReaderStart {
                        stream: StreamKind::Stderr,
                        error,
                    };
                    let cleanup = cleanup_partial_spawn(
                        &mut child,
                        &mut anchor,
                        protocol,
                        config.grace,
                        Some(stdout_handle),
                        None,
                    );
                    return Err(cleanup.err().unwrap_or(original));
                }
            };
            Ok(Self {
                child,
                anchor,
                identity,
                config,
                protocol,
                registration_active: protocol.is_some(),
                cleanup_complete: false,
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

        pub fn finish(self) -> Result<ChildSessionReceipt, ChildSessionFailure> {
            self.finish_observing(|_| Ok(()))
        }

        /// Wait for the exact child while invoking a parent-side observer.
        ///
        /// Observer failure triggers the same targeted stop-and-reap path as
        /// reader and wait failures. The callback never owns child cleanup.
        pub fn finish_observing<F>(
            mut self,
            mut observer: F,
        ) -> Result<ChildSessionReceipt, ChildSessionFailure>
        where
            F: FnMut(&ProcessIdentity) -> io::Result<()>,
        {
            let event = self.wait_for_event(self.config.timeout, &mut observer);
            let outcome = match event {
                Ok(SupervisorEvent::Exited) => self.teardown_and_reap(None),
                Ok(SupervisorEvent::Reader(fault)) => {
                    self.teardown_and_reap(Some(fault_error(fault)))
                }
                Ok(SupervisorEvent::Deadline) => {
                    self.teardown_and_reap(Some(ChildSessionError::Timeout {
                        term_sent: false,
                        kill_sent: false,
                    }))
                }
                Err(error) => self.teardown_and_reap(Some(error)),
            };
            let reader_deadline = Instant::now()
                .checked_add(self.config.grace)
                .unwrap_or_else(Instant::now);
            let stdout = join_reader_until(
                self.stdout_handle.take(),
                StreamKind::Stdout,
                reader_deadline,
            );
            let stderr = join_reader_until(
                self.stderr_handle.take(),
                StreamKind::Stderr,
                reader_deadline,
            );
            let orphan_ok = outcome.group_empty;
            let receipt = receipt_from(&self.identity, &outcome, &stdout, &stderr, orphan_ok);
            let failure = outcome
                .failure
                .or_else(|| reader_error(stdout, StreamKind::Stdout))
                .or_else(|| reader_error(stderr, StreamKind::Stderr))
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
            observer: &mut impl FnMut(&ProcessIdentity) -> io::Result<()>,
        ) -> Result<SupervisorEvent, ChildSessionError> {
            let deadline = Instant::now()
                .checked_add(duration)
                .ok_or_else(|| ChildSessionError::Wait(io::Error::other("deadline overflow")))?;
            loop {
                if Instant::now() >= deadline {
                    return Ok(SupervisorEvent::Deadline);
                }
                observer(&self.identity).map_err(ChildSessionError::Observer)?;
                if Instant::now() >= deadline {
                    return Ok(SupervisorEvent::Deadline);
                }
                match self.reader_faults.try_recv() {
                    Ok(fault) => return Ok(SupervisorEvent::Reader(fault)),
                    Err(std::sync::mpsc::TryRecvError::Disconnected)
                    | Err(std::sync::mpsc::TryRecvError::Empty) => {}
                }
                match self.anchor.observe() {
                    Ok(true) => return Ok(SupervisorEvent::Exited),
                    Ok(false) if Instant::now() >= deadline => {
                        return Ok(SupervisorEvent::Deadline)
                    }
                    Ok(false) => thread::sleep(
                        Duration::from_millis(5)
                            .min(deadline.saturating_duration_since(Instant::now())),
                    ),
                    Err(error) if error.kind() == ErrorKind::Interrupted => {}
                    Err(error) => return Err(ChildSessionError::Wait(error)),
                }
            }
        }

        fn teardown_and_reap(&mut self, mut failure: Option<ChildSessionError>) -> WaitOutcome {
            if let Err(error) = self.anchor.observe() {
                remember_first_failure(&mut failure, ChildSessionError::Wait(error));
            }

            let mut term_sent = false;
            self.record_group_signal(libc::SIGTERM, &mut term_sent, &mut failure);

            let mut group_empty = self.record_group_wait(&mut failure);
            let mut kill_sent = false;
            if !group_empty {
                self.record_group_signal(libc::SIGKILL, &mut kill_sent, &mut failure);
                group_empty = self.record_group_wait(&mut failure);
            }

            if group_empty && self.registration_active {
                if let Some(protocol) = self.protocol {
                    let pid = self.anchor.pid;
                    if let Err(error) = protocol.exchange(NestedGroupOperation::Unregister, pid) {
                        remember_first_failure(
                            &mut failure,
                            ChildSessionError::Registration(error),
                        );
                    } else {
                        self.registration_active = false;
                    }
                }
            }

            let status =
                match reap_until(&mut self.child, self.anchor.pid as u32, self.config.grace) {
                    Ok(status) => {
                        if let Err(error) = self.anchor.mark_reaped() {
                            remember_first_failure(&mut failure, error);
                        } else {
                            self.cleanup_complete = true;
                        }
                        status
                    }
                    Err(error) => {
                        remember_first_failure(&mut failure, error);
                        synthetic_failure_status()
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
                group_empty,
                failure,
            }
        }

        fn record_group_signal(
            &self,
            signal: i32,
            sent: &mut bool,
            failure: &mut Option<ChildSessionError>,
        ) {
            match self.signal_group_if_present(signal) {
                Ok(was_sent) => *sent = was_sent,
                Err(error) => remember_first_failure(failure, error),
            }
        }

        fn record_group_wait(&mut self, failure: &mut Option<ChildSessionError>) -> bool {
            match self.wait_for_group(self.config.grace) {
                Ok(stopped) => stopped,
                Err(error) => {
                    remember_first_failure(failure, error);
                    false
                }
            }
        }

        fn wait_for_group(&mut self, duration: Duration) -> Result<bool, ChildSessionError> {
            let deadline = Instant::now()
                .checked_add(duration)
                .ok_or_else(|| ChildSessionError::Wait(io::Error::other("deadline overflow")))?;
            loop {
                match self.anchor.observe() {
                    Ok(_) => {}
                    Err(error) if error.kind() == ErrorKind::Interrupted => continue,
                    Err(error) => return Err(ChildSessionError::Wait(error)),
                }
                if self.anchor.observed
                    && !process_group_has_other_members(
                        self.identity.pid,
                        &self.anchor.ignored_group_members(),
                    )
                    .map_err(ChildSessionError::ProcessGroup)?
                {
                    return Ok(true);
                }
                if Instant::now() >= deadline {
                    return Ok(false);
                }
                thread::sleep(
                    Duration::from_millis(5)
                        .min(deadline.saturating_duration_since(Instant::now())),
                );
            }
        }

        fn signal_group_if_present(&self, signal: i32) -> Result<bool, ChildSessionError> {
            if self.anchor.observed
                && !process_group_has_other_members(
                    self.identity.pid,
                    &self.anchor.ignored_group_members(),
                )
                .map_err(ChildSessionError::ProcessGroup)?
            {
                return Ok(false);
            }
            match signal_process_group(&self.anchor, signal) {
                Ok(()) => Ok(true),
                Err(error) if error.raw_os_error() == Some(libc::ESRCH) => Ok(false),
                Err(error) => Err(ChildSessionError::Signal {
                    pid: self.identity.pid,
                    signal,
                    error,
                }),
            }
        }
    }

    fn remember_first_failure(failure: &mut Option<ChildSessionError>, error: ChildSessionError) {
        if failure.is_none() {
            *failure = Some(error);
        }
    }

    fn create_log(path: &Path, stream: StreamKind) -> Result<File, ChildSessionError> {
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|error| ChildSessionError::LogOpen { stream, error })
    }

    pub(super) fn cleanup_partial_spawn(
        child: &mut Child,
        anchor: &mut LeaderAnchor,
        protocol: Option<NestedGroupProtocol>,
        grace: Duration,
        stdout: Option<JoinHandle<DrainResult>>,
        stderr: Option<JoinHandle<DrainResult>>,
    ) -> Result<(), ChildSessionError> {
        cleanup_partial_spawn_with_inspector(
            child,
            anchor,
            protocol,
            grace,
            stdout,
            stderr,
            process_group_has_other_members,
        )
    }

    pub(super) fn cleanup_partial_spawn_with_inspector<F>(
        child: &mut Child,
        anchor: &mut LeaderAnchor,
        protocol: Option<NestedGroupProtocol>,
        grace: Duration,
        stdout: Option<JoinHandle<DrainResult>>,
        stderr: Option<JoinHandle<DrainResult>>,
        mut inspect_group: F,
    ) -> Result<(), ChildSessionError>
    where
        F: FnMut(u32, &[libc::pid_t]) -> io::Result<bool>,
    {
        let process_group = u32::try_from(anchor.pid).unwrap_or(0);
        let ignored = anchor.ignored_group_members();
        let mut failure = None;
        let mut group_empty = false;
        match anchor.observe() {
            Ok(_) if anchor.observed => match inspect_group(process_group, &ignored) {
                Ok(has_others) => group_empty = !has_others,
                Err(error) if error.raw_os_error() == Some(libc::ESRCH) => group_empty = true,
                Err(error) => {
                    remember_first_failure(&mut failure, ChildSessionError::ProcessGroup(error));
                }
            },
            Ok(_) => {}
            Err(error) => {
                remember_first_failure(&mut failure, ChildSessionError::Observer(error));
            }
        }
        if !group_empty {
            if let Err(error) = signal_process_group(anchor, libc::SIGTERM) {
                if error.raw_os_error() != Some(libc::ESRCH) {
                    remember_first_failure(
                        &mut failure,
                        ChildSessionError::Signal {
                            pid: process_group,
                            signal: libc::SIGTERM,
                            error,
                        },
                    );
                }
            }
            group_empty = match wait_for_partial_group(
                anchor,
                process_group,
                &ignored,
                grace,
                &mut inspect_group,
            ) {
                Ok(empty) => empty,
                Err(error) => {
                    remember_first_failure(&mut failure, ChildSessionError::ProcessGroup(error));
                    false
                }
            };
        }
        if !group_empty {
            if let Err(error) = signal_process_group(anchor, libc::SIGKILL) {
                if error.raw_os_error() != Some(libc::ESRCH) {
                    remember_first_failure(
                        &mut failure,
                        ChildSessionError::Signal {
                            pid: process_group,
                            signal: libc::SIGKILL,
                            error,
                        },
                    );
                }
            }
            group_empty = match wait_for_partial_group(
                anchor,
                process_group,
                &ignored,
                grace,
                &mut inspect_group,
            ) {
                Ok(empty) => empty,
                Err(error) => {
                    remember_first_failure(&mut failure, ChildSessionError::ProcessGroup(error));
                    false
                }
            };
        }
        let reader_deadline = Instant::now()
            .checked_add(grace)
            .unwrap_or_else(Instant::now);
        if let Some(stdout) = stdout {
            if let Err(error) = join_reader_until(Some(stdout), StreamKind::Stdout, reader_deadline)
            {
                remember_first_failure(&mut failure, error);
            }
        }
        if let Some(stderr) = stderr {
            if let Err(error) = join_reader_until(Some(stderr), StreamKind::Stderr, reader_deadline)
            {
                remember_first_failure(&mut failure, error);
            }
        }
        if group_empty {
            if let Some(protocol) = protocol {
                if let Err(error) = protocol.exchange(NestedGroupOperation::Unregister, anchor.pid)
                {
                    remember_first_failure(&mut failure, ChildSessionError::Registration(error));
                }
            }
        } else {
            remember_first_failure(
                &mut failure,
                ChildSessionError::Orphan { pid: process_group },
            );
        }
        match reap_until(child, anchor.pid as u32, grace) {
            Ok(_) => {
                if let Err(error) = anchor.mark_reaped() {
                    remember_first_failure(&mut failure, error);
                }
            }
            Err(error) => remember_first_failure(&mut failure, error),
        }
        failure.map_or(Ok(()), Err)
    }

    fn wait_for_partial_group<F>(
        anchor: &mut LeaderAnchor,
        process_group: u32,
        ignored: &[libc::pid_t],
        duration: Duration,
        inspect_group: &mut F,
    ) -> io::Result<bool>
    where
        F: FnMut(u32, &[libc::pid_t]) -> io::Result<bool>,
    {
        let deadline = Instant::now()
            .checked_add(duration)
            .unwrap_or_else(Instant::now);
        while Instant::now() < deadline {
            anchor.observe()?;
            if anchor.observed {
                match inspect_group(process_group, ignored) {
                    Ok(false) => return Ok(true),
                    Ok(true) => {}
                    Err(error) if error.raw_os_error() == Some(libc::ESRCH) => return Ok(true),
                    Err(error) => return Err(error),
                }
            }
            thread::sleep(Duration::from_millis(5));
        }
        Ok(false)
    }

    fn reap_until(
        child: &mut Child,
        pid: u32,
        duration: Duration,
    ) -> Result<ExitStatus, ChildSessionError> {
        let deadline = Instant::now()
            .checked_add(duration)
            .unwrap_or_else(Instant::now);
        match reap_until_with(deadline, || child.try_wait()).map_err(ChildSessionError::Wait)? {
            Some(status) => Ok(status),
            None => Err(ChildSessionError::ReapTimeout { pid }),
        }
    }

    fn reap_until_with<T, F>(deadline: Instant, mut try_wait: F) -> io::Result<Option<T>>
    where
        F: FnMut() -> io::Result<Option<T>>,
    {
        loop {
            match try_wait() {
                Ok(Some(status)) => return Ok(Some(status)),
                Ok(None) if Instant::now() >= deadline => return Ok(None),
                Ok(None) => thread::sleep(
                    Duration::from_millis(5)
                        .min(deadline.saturating_duration_since(Instant::now())),
                ),
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

    fn join_reader_until(
        handle: Option<JoinHandle<DrainResult>>,
        stream: StreamKind,
        deadline: Instant,
    ) -> Result<DrainResult, ChildSessionError> {
        let handle = handle.ok_or(ChildSessionError::JoinPanic { stream })?;
        while !handle.is_finished() {
            if Instant::now() >= deadline {
                return Err(ChildSessionError::ReaderCompletionTimeout { stream });
            }
            thread::sleep(
                Duration::from_millis(5).min(deadline.saturating_duration_since(Instant::now())),
            );
        }
        handle
            .join()
            .map_err(|_| ChildSessionError::JoinPanic { stream })
    }

    fn reader_error(
        result: Result<DrainResult, ChildSessionError>,
        stream: StreamKind,
    ) -> Option<ChildSessionError> {
        match result {
            Err(error) => Some(error),
            Ok(result) => result
                .fault
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
            if !self.cleanup_complete {
                let _ = self.teardown_and_reap(None);
            }
            let reader_deadline = Instant::now()
                .checked_add(self.config.grace)
                .unwrap_or_else(Instant::now);
            let _ = join_reader_until(
                self.stdout_handle.take(),
                StreamKind::Stdout,
                reader_deadline,
            );
            let _ = join_reader_until(
                self.stderr_handle.take(),
                StreamKind::Stderr,
                reader_deadline,
            );
        }
    }

    #[cfg(test)]
    mod anchor_tests {
        use super::*;

        #[test]
        fn leader_anchor_remains_owned_from_observation_through_signal_boundary() {
            let mut anchor = LeaderAnchor {
                pid: 123,
                monitor_anchor_pid: None,
                observed: true,
                reaped: false,
            };
            assert!(anchor.permits_group_operation());
            anchor.mark_reaped().unwrap();
            assert!(!anchor.permits_group_operation());
            assert!(matches!(
                anchor.mark_reaped(),
                Err(ChildSessionError::CleanupState { .. })
            ));
        }

        #[test]
        fn bounded_reap_reports_deadline_without_blocking_wait() {
            let mut attempts = 0;
            let result = reap_until_with(Instant::now(), || {
                attempts += 1;
                Ok::<Option<()>, io::Error>(None)
            })
            .unwrap();
            assert_eq!(result, None);
            assert_eq!(attempts, 1);
        }

        #[test]
        fn bounded_reap_propagates_observation_failure() {
            let error = reap_until_with::<(), _>(Instant::now() + Duration::from_secs(1), || {
                Err(io::Error::other("injected wait failure"))
            })
            .unwrap_err();
            assert_eq!(error.to_string(), "injected wait failure");
        }

        #[test]
        fn reaped_anchor_observation_returns_io_failure() {
            let mut anchor = LeaderAnchor {
                pid: 123,
                monitor_anchor_pid: None,
                observed: true,
                reaped: true,
            };
            assert_eq!(
                anchor.observe().unwrap_err().to_string(),
                "cannot observe a reaped leader anchor"
            );
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

    #[cfg(unix)]
    #[test]
    fn nested_group_exchange_times_out_when_token_peer_stalls() {
        let mut token = [0; 2];
        // SAFETY: token contains storage for both descriptors.
        assert_eq!(unsafe { libc::pipe(token.as_mut_ptr()) }, 0);
        let protocol = NestedGroupProtocol::new(token[0], token[1], token[1]);
        let error = protocol
            .exchange_with_timeout(
                NestedGroupOperation::Register,
                123,
                std::time::Duration::from_millis(1),
            )
            .unwrap_err();
        assert_eq!(error.raw_os_error(), Some(libc::ETIMEDOUT));
        // SAFETY: both descriptors were returned by pipe and are closed once.
        unsafe {
            libc::close(token[0]);
            libc::close(token[1]);
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

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_group_membership_ignores_terminal_processes() {
        assert!(super::os::linux_stat_is_live_group_member(
            "42 (live worker) S 1 599 0",
            599
        ));
        assert!(!super::os::linux_stat_is_live_group_member(
            "42 (terminal worker) Z 1 599 0",
            599
        ));
        assert!(!super::os::linux_stat_is_live_group_member(
            "42 (other group) S 1 600 0",
            599
        ));
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

    fn delayed_command(script: &str) -> Command {
        let mut command = Command::new("sh");
        command.args(["-c", &format!("sleep 1; {script}")]);
        command
    }

    #[test]
    fn normal_completion_exit_zero() {
        let dir = TempDir::new().expect("tempdir");
        let config = make_config(&dir, Duration::from_secs(5), Duration::from_secs(2));
        let cmd = delayed_command("exit 0");
        let session = ChildSession::spawn(cmd, config).expect("spawn");
        let receipt = session.finish().expect("finish");
        assert_eq!(receipt.exit_code, Some(0));
        assert!(!receipt.term_sent);
        assert!(!receipt.kill_sent);
        assert!(receipt.output_drained);
        assert!(receipt.orphan_check_passed);
    }

    #[test]
    fn observer_receives_only_the_exact_child_identity() {
        let dir = TempDir::new().expect("tempdir");
        let config = make_config(&dir, Duration::from_secs(5), Duration::from_secs(2));
        let mut command = Command::new("sh");
        command.args(["-c", "sleep 0.05; exit 0"]);
        let session = ChildSession::spawn(command, config).expect("spawn");
        let expected = session.identity().clone();
        let mut observations = 0;
        let receipt = session
            .finish_observing(|identity| {
                assert_eq!(identity, &expected);
                observations += 1;
                Ok(())
            })
            .expect("finish");
        assert!(observations > 0);
        assert_eq!(receipt.identity, expected);
        assert!(receipt.orphan_check_passed);
    }

    #[test]
    fn observer_failure_stops_and_reaps_the_exact_child() {
        let dir = TempDir::new().expect("tempdir");
        let config = make_config(&dir, Duration::from_secs(5), Duration::from_secs(2));
        let mut command = Command::new("sh");
        command.args(["-c", "sleep 10"]);
        let session = ChildSession::spawn(command, config).expect("spawn");
        let failure = session
            .finish_observing(|_| Err(std::io::Error::other("observation failed")))
            .expect_err("observer failure");
        assert!(matches!(failure.error, ChildSessionError::Observer(_)));
        assert!(failure.receipt.term_sent);
        assert!(failure.receipt.orphan_check_passed);
    }

    #[test]
    fn observer_returning_after_deadline_cannot_turn_timeout_into_success() {
        let dir = TempDir::new().expect("tempdir");
        let config = make_config(&dir, Duration::from_millis(10), Duration::from_secs(1));
        let mut command = Command::new("sh");
        command.args(["-c", "sleep 10"]);
        let session = ChildSession::spawn(command, config).expect("spawn");
        let failure = session
            .finish_observing(|_| {
                std::thread::sleep(Duration::from_millis(50));
                Ok(())
            })
            .expect_err("observer overrun must preserve timeout");
        assert!(matches!(
            failure.error,
            ChildSessionError::Timeout {
                term_sent: true,
                ..
            }
        ));
        assert!(failure.receipt.term_sent);
        assert!(failure.receipt.orphan_check_passed);
    }

    #[test]
    fn normal_completion_exit_nonzero() {
        let dir = TempDir::new().expect("tempdir");
        let config = make_config(&dir, Duration::from_secs(5), Duration::from_secs(2));
        let cmd = delayed_command("exit 1");
        let session = ChildSession::spawn(cmd, config).expect("spawn");
        let receipt = session.finish().expect("finish");
        assert_eq!(receipt.exit_code, Some(1));
    }

    #[test]
    fn stdout_drained_to_file() {
        let dir = TempDir::new().expect("tempdir");
        let config = make_config(&dir, Duration::from_secs(5), Duration::from_secs(2));
        let cmd = delayed_command("printf 'hello-stdout'");
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
        let cmd = delayed_command("printf 'hello-stderr' >&2");
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
        let cmd = delayed_command("printf 'OUT'; printf 'ERR' >&2");
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
        let cmd = delayed_command("printf 'AAAAAAAAAA'"); // 10 bytes > 4
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
        let cmd = delayed_command("printf 'BBBBBBBBBB' >&2");
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
        let cmd = delayed_command("exit 0");
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
    fn identity_survives_a_child_that_exits_before_it_is_captured() {
        let child = Command::new("/usr/bin/true")
            .spawn()
            .expect("spawn immediate-exit child");
        let pid = child.id();
        let mut status = 0;
        let mut info = std::mem::MaybeUninit::<libc::siginfo_t>::zeroed();
        // Leave the child waitable so its PID stays pinned while unreaped.
        let observed = unsafe {
            libc::waitid(
                libc::P_PID,
                pid,
                info.as_mut_ptr(),
                libc::WEXITED | libc::WNOWAIT,
            )
        };
        assert_eq!(observed, 0, "child must reach a terminal state");

        let identity = capture_identity(pid, "deadbeef").expect("identity of an exited child");
        assert_eq!(identity.pid, pid);
        assert!(
            identity
                .start_time
                .parse::<u64>()
                .is_ok_and(|start| start > 0),
            "start_time should be numeric: {}",
            identity.start_time
        );

        assert_eq!(
            unsafe { libc::waitpid(pid as libc::pid_t, &raw mut status, 0) },
            pid as libc::pid_t
        );
        drop(child);
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
        let mut cmd = Command::new("sleep");
        cmd.arg("0.05");
        let session = ChildSession::spawn(cmd, config).expect("spawn");
        let expected_pid = session.pid();
        let receipt = session.finish().expect("finish");
        assert_eq!(receipt.identity.pid, expected_pid);
        assert_eq!(receipt.identity.executable_digest, "deadbeef");
    }

    fn command_with_pipe_holding_descendant(
        pid_file: &std::path::Path,
        ignore_term: bool,
    ) -> Command {
        let mut command = Command::new("sh");
        command
            .env("DESCENDANT_PID_FILE", pid_file)
            .env(
                "DESCENDANT_IGNORE_TERM",
                if ignore_term { "1" } else { "0" },
            )
            .args([
                "-c",
                "sh -c 'trap \"\" HUP; if [ \"$DESCENDANT_IGNORE_TERM\" = 1 ]; then trap \"\" TERM; fi; printf descendant-stdout; printf descendant-stderr >&2; printf %s \"$$\" > \"$DESCENDANT_PID_FILE\"; while :; do sleep 1; done' & while [ ! -s \"$DESCENDANT_PID_FILE\" ]; do sleep 0.01; done; sleep 0.05; exit 0",
            ]);
        command
    }

    fn read_pid(path: &std::path::Path) -> u32 {
        std::fs::read_to_string(path)
            .expect("read descendant pid")
            .parse()
            .expect("parse descendant pid")
    }

    fn process_is_live(pid: u32) -> bool {
        #[cfg(target_os = "linux")]
        {
            let stat = match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
                Ok(stat) => stat,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => return false,
                Err(_) => return true,
            };
            let Some(after_comm) = stat.rfind(')') else {
                return true;
            };
            stat[after_comm + 1..].split_whitespace().next() != Some("Z")
        }

        #[cfg(not(target_os = "linux"))]
        {
            // SAFETY: signal zero only checks whether this exact PID exists.
            let result = unsafe { libc::kill(pid as libc::pid_t, 0) };
            result == 0 || std::io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH)
        }
    }

    #[test]
    fn direct_exit_terminates_descendant_holding_both_output_pipes() {
        let dir = TempDir::new().expect("tempdir");
        let pid_file = dir.path().join("descendant.pid");
        let config = make_config(&dir, Duration::from_secs(5), Duration::from_millis(200));
        let command = command_with_pipe_holding_descendant(&pid_file, false);
        let session = ChildSession::spawn(command, config).expect("spawn");

        let receipt = session.finish().expect("finish");
        let descendant_pid = read_pid(&pid_file);

        assert_eq!(receipt.exit_code, Some(0));
        assert!(receipt.term_sent);
        assert!(!receipt.kill_sent);
        assert!(receipt.output_drained);
        assert!(receipt.orphan_check_passed);
        assert!(!process_is_live(descendant_pid));
        assert_eq!(
            std::fs::read_to_string(dir.path().join("out.log")).expect("read stdout log"),
            "descendant-stdout"
        );
        assert_eq!(
            std::fs::read_to_string(dir.path().join("err.log")).expect("read stderr log"),
            "descendant-stderr"
        );
    }

    #[test]
    fn direct_exit_kills_term_ignoring_descendant_without_blocking_readers() {
        let dir = TempDir::new().expect("tempdir");
        let pid_file = dir.path().join("descendant.pid");
        let config = make_config(&dir, Duration::from_secs(5), Duration::from_millis(100));
        let command = command_with_pipe_holding_descendant(&pid_file, true);
        let started = std::time::Instant::now();
        let session = ChildSession::spawn(command, config).expect("spawn");

        let receipt = session.finish().expect("finish");
        let descendant_pid = read_pid(&pid_file);

        assert!(started.elapsed() < Duration::from_secs(2));
        assert_eq!(receipt.exit_code, Some(0));
        assert!(receipt.term_sent);
        assert!(receipt.kill_sent);
        assert!(receipt.output_drained);
        assert!(receipt.orphan_check_passed);
        assert!(!process_is_live(descendant_pid));
    }

    fn partial_cleanup_child(seconds: &str) -> (std::process::Child, super::os::LeaderAnchor) {
        use std::os::unix::process::CommandExt as _;

        let mut command = Command::new("/bin/sleep");
        command.arg(seconds);
        unsafe {
            command.pre_exec(|| {
                if libc::setsid() == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let child = command.spawn().expect("spawn isolated cleanup child");
        let anchor = super::os::LeaderAnchor::new(child.id(), None).expect("create leader anchor");
        (child, anchor)
    }

    fn assert_exact_child_reaped(pid: u32) {
        let mut status = 0;
        let result = unsafe { libc::waitpid(pid as libc::pid_t, &mut status, libc::WNOHANG) };
        assert_eq!(result, -1);
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ECHILD)
        );
    }

    #[test]
    fn partial_cleanup_accepts_an_already_exited_leader_and_reaps_it() {
        let (mut child, mut anchor) = partial_cleanup_child("0.01");
        let pid = child.id();
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while !anchor.observe().expect("observe cleanup child") {
            assert!(
                std::time::Instant::now() < deadline,
                "cleanup child did not exit"
            );
            std::thread::sleep(Duration::from_millis(5));
        }

        super::os::cleanup_partial_spawn(
            &mut child,
            &mut anchor,
            None,
            Duration::from_millis(100),
            None,
            None,
        )
        .expect("clean already-exited child");
        assert_exact_child_reaped(pid);
    }

    #[test]
    fn partial_cleanup_reaps_after_reader_join_failure_without_touching_unrelated_group() {
        let (mut child, mut anchor) = partial_cleanup_child("10");
        let pid = child.id();
        let (mut unrelated, _) = partial_cleanup_child("10");
        let reader =
            std::thread::spawn(|| -> super::os::DrainResult { panic!("injected reader failure") });

        let error = super::os::cleanup_partial_spawn(
            &mut child,
            &mut anchor,
            None,
            Duration::from_millis(100),
            Some(reader),
            None,
        )
        .expect_err("reader failure must be reported after cleanup");
        assert!(matches!(
            error,
            super::os::ChildSessionError::JoinPanic {
                stream: super::os::StreamKind::Stdout
            }
        ));
        assert_exact_child_reaped(pid);
        assert!(unrelated.try_wait().expect("inspect unrelated").is_none());
        unrelated.kill().expect("kill unrelated");
        unrelated.wait().expect("reap unrelated");
    }

    #[test]
    fn partial_cleanup_reaps_after_group_inspection_failure_and_retains_registration() {
        let (mut child, mut anchor) = partial_cleanup_child("10");
        let pid = child.id();
        let invalid_protocol = NestedGroupProtocol::new(-1, -1, -1);
        let error = super::os::cleanup_partial_spawn_with_inspector(
            &mut child,
            &mut anchor,
            Some(invalid_protocol),
            Duration::from_millis(50),
            None,
            None,
            |_, _| Err(std::io::Error::from_raw_os_error(libc::EIO)),
        )
        .expect_err("inspection failure must be retained");
        assert!(matches!(
            error,
            super::os::ChildSessionError::ProcessGroup(_)
        ));
        assert_exact_child_reaped(pid);
    }

    #[test]
    fn partial_cleanup_reaps_after_unregister_failure() {
        let (mut child, mut anchor) = partial_cleanup_child("10");
        let pid = child.id();
        let invalid_protocol = NestedGroupProtocol::new(-1, -1, -1);
        let error = super::os::cleanup_partial_spawn(
            &mut child,
            &mut anchor,
            Some(invalid_protocol),
            Duration::from_millis(100),
            None,
            None,
        )
        .expect_err("unregister failure must be reported after cleanup");
        assert!(matches!(
            error,
            super::os::ChildSessionError::Registration(_)
        ));
        assert_exact_child_reaped(pid);
    }
}
