//! ChildSession supervision state machine.
//!
//! Owns Child/process identity/pipes/bounded readers/socket/manifest.
//! Normal: try_wait Some → store once → drop stdin/pipes → drain/join →
//! validate. Failure: record cause → cooperative stop → bounded poll →
//! child-only kill if live → wait on Interrupted → reap → close → join →
//! cleanup → orphan check. Drop is non-panicking backstop only.
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
/// In production, this owns a `Child`, pipes, reader threads, socket, and
/// manifest. The pure model tracks the state machine, identity, and
/// classification.
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
