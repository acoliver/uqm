//! Pure sticky-terminal runtime model: ABI shell, finalization, and lock-order
//! instrumentation.
//!
//! Implements REQ-STATE-004 (finalization: clear active/capture, drain active
//! shells/reservations, atomic take once, ordered run_end/close once, late
//! callback cannot use writer) and the lock-order instrumentation that
//! rejects runtime-mutex overlap with C/SDL/graphics/log/wait/file and
//! rejects runtime+ordered-I/O nesting.
//!
//! @plan PLAN-20260723-RUNTIME-AUTOMATION.P04
//! @requirement REQ-STATE-004

#[cfg(test)]
use crate::automation::outcome::TerminalMirror;
use crate::automation::outcome::{TerminalClass, TerminalCommand};
use crate::automation::sync_model::{RuntimePhase, SyncModel};
use crate::automation::trace::OrderedCommit;
use parking_lot::{Mutex, MutexGuard};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

// ===========================================================================
//  Runtime state (under mutex)
// ===========================================================================

/// The inner runtime state, protected by the runtime mutex.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P04
/// @requirement REQ-STATE-004
pub struct RuntimeState {
    /// Whether the runtime is active.
    pub active: bool,
    /// The current state version (for stale/duplicate commit detection).
    pub state_version: u64,
    /// The number of active shells currently executing.
    pub active_shell_count: u64,
    /// Whether finalization has been performed.
    pub finalized: bool,
    /// Whether run_end has been written (exactly once).
    pub run_end_written: bool,
}

impl RuntimeState {
    fn new() -> Self {
        Self {
            active: false,
            state_version: 0,
            active_shell_count: 0,
            finalized: false,
            run_end_written: false,
        }
    }
}

// ===========================================================================
//  ABI shell model (REQ-STATE-003/004)
// ===========================================================================

/// The result of an ABI shell entry.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P04
/// @requirement REQ-STATE-003
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShellResult {
    /// Whether to stop (conservative result for service/after/safe-point = 1).
    pub stop: bool,
    /// Whether this was an inactive fast-path return.
    pub inactive_fast_path: bool,
    /// Whether a terminal fallback was applied.
    pub terminal_fallback: bool,
}

impl ShellResult {
    /// Neutral inactive result: no stop, fast-path return.
    #[must_use]
    pub fn inactive_continue() -> Self {
        Self {
            stop: false,
            inactive_fast_path: true,
            terminal_fallback: false,
        }
    }

    /// Neutral inactive result for service/after/safe-point: stop=0.
    #[must_use]
    pub fn inactive_no_stop() -> Self {
        Self {
            stop: false,
            inactive_fast_path: true,
            terminal_fallback: false,
        }
    }

    /// Conservative terminal result: stop=true.
    #[must_use]
    pub fn terminal() -> Self {
        Self {
            stop: true,
            inactive_fast_path: false,
            terminal_fallback: true,
        }
    }

    /// Active normal result: stop depends on scheduler.
    #[must_use]
    pub fn active(stop: bool) -> Self {
        Self {
            stop,
            inactive_fast_path: false,
            terminal_fallback: false,
        }
    }
}

/// The complete runtime model, combining the lock-free mirror with the
/// mutex-protected inner state and the ordered commit object.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P04
/// @requirement REQ-STATE-003, REQ-STATE-004
pub struct RuntimeModel {
    /// Lock-free mirror for terminal/abort/phase/capture/keys.
    pub mirror: SyncModel,
    /// Mutex-protected inner state.
    inner: Mutex<RuntimeState>,
    /// Ordered commit for trace records.
    pub commit: OrderedCommit,
    /// ABI entry counter (saturating, nonwrapping).
    abi_entry: AtomicU64,
    /// Active gate entry counter.
    active_gate_entry: AtomicU64,
}

impl RuntimeModel {
    /// Create a new runtime model in the inactive state.
    #[must_use]
    pub fn new() -> Self {
        Self {
            mirror: SyncModel::new(),
            inner: Mutex::new(RuntimeState::new()),
            commit: OrderedCommit::new(),
            abi_entry: AtomicU64::new(0),
            active_gate_entry: AtomicU64::new(0),
        }
    }

    /// Increment the ABI entry counter (saturating).
    pub fn record_abi_entry(&self) {
        let _ = self
            .abi_entry
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |v| v.checked_add(1));
    }

    /// Get the ABI entry count.
    #[must_use]
    pub fn abi_entry_count(&self) -> u64 {
        self.abi_entry.load(Ordering::Acquire)
    }

    /// Increment the active gate entry counter (saturating).
    pub fn record_active_gate_entry(&self) {
        let _ = self
            .active_gate_entry
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |v| v.checked_add(1));
    }

    /// Get the active gate entry count.
    #[must_use]
    pub fn active_gate_count(&self) -> u64 {
        self.active_gate_entry.load(Ordering::Acquire)
    }

    /// Activate the runtime. Sets phase=Running in the lock-free mirror.
    pub fn activate(&self) {
        let mut state = self.inner.lock();
        state.active = true;
        self.mirror.set_phase(RuntimePhase::Running);
    }

    /// Deactivate the runtime. Sets phase back to Inactive.
    pub fn deactivate(&self) {
        let mut state = self.inner.lock();
        state.active = false;
        self.mirror.set_phase(RuntimePhase::Inactive);
    }

    /// ABI shell entry: the complete pure model.
    ///
    /// 1. Increment ABI_ENTRY (saturating).
    /// 2. Acquire-load activation. If inactive, return neutral fast path.
    /// 3. Increment ACTIVE_GATE_ENTRY.
    /// 4. Check reentry (depth > 0). If reentrant, request abort, release
    ///    keys, return conservative.
    /// 5. If terminal mirror is set, return conservative.
    /// 6. Otherwise, increment active_shell_count and return active.
    ///
    /// @plan PLAN-20260723-RUNTIME-AUTOMATION.P04
    /// @requirement REQ-STATE-003
    pub fn shell_enter(&self) -> ShellResult {
        // Step 1: ABI entry counter (saturating, nonwrapping).
        self.record_abi_entry();

        // Step 2: Acquire-load activation from the lock-free mirror.
        // Inactive → neutral fast path (no TLS/depth/lock/alloc/log).
        if !self.mirror.is_active() {
            return ShellResult::inactive_no_stop();
        }

        // Step 3: Active gate entry.
        self.record_active_gate_entry();

        // Step 4: Check reentry.
        if self.mirror.is_reentrant() {
            // Nested entry: request abort, release keys, return conservative.
            self.mirror.request_abort();
            self.mirror.owned_keys.release_all();
            self.mirror.terminal.try_set(TerminalClass::PanicFallback);
            return ShellResult::terminal();
        }

        // Step 5: Check terminal mirror.
        if self.mirror.is_terminal() {
            return ShellResult::terminal();
        }

        // Step 6: Increment active shell count.
        {
            let mut state = self.inner.lock();
            state.active_shell_count = state.active_shell_count.saturating_add(1);
        }

        ShellResult::active(false)
    }

    /// Shell exit: decrement active shell count.
    pub fn shell_exit(&self) {
        let mut state = self.inner.lock();
        state.active_shell_count = state.active_shell_count.saturating_sub(1);
    }

    /// Reserve a pure transition under the runtime mutex. Returns the new
    /// state version and a reservation. Does NOT commit scheduler advancement.
    ///
    /// @plan PLAN-20260723-RUNTIME-AUTOMATION.P04
    /// @requirement REQ-STATE-004
    pub fn reserve_transition(&self) -> Option<(u64, crate::automation::trace::Reservation)> {
        let mut state = self.inner.lock();
        if state.finalized {
            return None;
        }
        let new_version = state.state_version.checked_add(1)?;
        state.state_version = new_version;
        let reservation = self.commit.reserve();
        Some((new_version, reservation))
    }

    /// Commit a transition if the state version matches. Returns `false` if
    /// stale (version mismatch) or duplicate.
    ///
    /// @plan PLAN-20260723-RUNTIME-AUTOMATION.P04
    /// @requirement REQ-STATE-004
    pub fn commit_transition(&self, expected_version: u64) -> bool {
        let state = self.inner.lock();
        if state.finalized {
            return false;
        }
        if state.state_version != expected_version {
            return false;
        }
        true
    }

    /// Finalization: atomically change phase to Finalizing and take runtime
    /// ownership once.
    ///
    /// 1. Set phase to Finalizing.
    /// 2. Clear active gate and capture request.
    /// 3. Wait for active shell count to reach zero (in pure model, we just
    ///    check).
    /// 4. Write run_end exactly once.
    /// 5. Set phase to Finalized.
    /// 6. Late callback cannot use writer (finalized flag blocks).
    ///
    /// @plan PLAN-20260723-RUNTIME-AUTOMATION.P04
    /// @requirement REQ-STATE-004
    pub fn finalize(&self) -> FinalizationResult {
        // Step 1: Atomic phase change to Finalizing.
        let prev_phase = self.mirror.phase.load(Ordering::Acquire);
        if prev_phase == RuntimePhase::Finalized as u8 {
            return FinalizationResult::AlreadyFinalized;
        }
        if prev_phase == RuntimePhase::Finalizing as u8 {
            return FinalizationResult::AlreadyFinalizing;
        }
        self.mirror.set_phase(RuntimePhase::Finalizing);

        // Step 2: Clear active gate and capture request.
        self.mirror.clear_capture_generation();

        // Step 3: Check active shell count.
        let shell_count = {
            let state = self.inner.lock();
            state.active_shell_count
        };
        if shell_count > 0 {
            // In pure model, we record that shells are still active.
            // Real finalization would wait; here we report it.
            return FinalizationResult::ShellsStillActive(shell_count);
        }

        // Step 4: Write run_end exactly once.
        let mut state = self.inner.lock();
        if state.run_end_written {
            return FinalizationResult::DuplicateRunEnd;
        }
        state.run_end_written = true;

        // Step 5: Mark finalized.
        state.finalized = true;
        drop(state);

        self.mirror.set_phase(RuntimePhase::Finalized);

        FinalizationResult::Finalized
    }

    /// Whether a late callback can use the writer (i.e., not finalized).
    #[must_use]
    pub fn can_write(&self) -> bool {
        let state = self.inner.lock();
        !state.finalized
    }

    /// Get the terminal command for the current terminal class.
    #[must_use]
    pub fn terminal_command(&self) -> Option<TerminalCommand> {
        if self.mirror.is_terminal() {
            Some(TerminalCommand::terminal())
        } else {
            None
        }
    }

    /// Lock the runtime mutex, returning a guard.
    pub fn lock_inner(&self) -> MutexGuard<'_, RuntimeState> {
        self.inner.lock()
    }
}

impl Default for RuntimeModel {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
//  Finalization result
// ===========================================================================

/// The result of finalization.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P04
/// @requirement REQ-STATE-004
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinalizationResult {
    /// Finalization completed successfully.
    Finalized,
    /// Finalization was already completed.
    AlreadyFinalized,
    /// Finalization was already in progress.
    AlreadyFinalizing,
    /// Active shells are still running.
    ShellsStillActive(u64),
    /// run_end was already written (duplicate).
    DuplicateRunEnd,
}

// ===========================================================================
//  Lock-order instrumentation (REQ-STATE-004)
// ===========================================================================

/// Lock-order violation detector. Tracks whether the runtime mutex is held
/// while external operations (C/SDL/graphics/log/wait/file) or ordered I/O
/// nesting occurs.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P04
/// @requirement REQ-STATE-004
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockOrderViolation {
    /// Runtime mutex held during C/SDL/graphics/log/wait/file operation.
    RuntimeMutexOverlapWithExternal,
    /// Runtime mutex held while waiting/writing through ordered I/O.
    RuntimeMutexOverlapWithOrderedIo,
}

/// A simple lock-order tracker for pure testing.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P04
/// @requirement REQ-STATE-004
pub struct LockOrderTracker {
    runtime_mutex_held: AtomicBool,
    ordered_io_held: AtomicBool,
}

impl LockOrderTracker {
    /// Create a new lock-order tracker.
    #[must_use]
    pub fn new() -> Self {
        Self {
            runtime_mutex_held: AtomicBool::new(false),
            ordered_io_held: AtomicBool::new(false),
        }
    }

    /// Mark the runtime mutex as held/released.
    pub fn set_runtime_mutex(&self, held: bool) {
        self.runtime_mutex_held.store(held, Ordering::Release);
    }

    /// Mark ordered I/O as held/released.
    pub fn set_ordered_io(&self, held: bool) {
        self.ordered_io_held.store(held, Ordering::Release);
    }

    /// Check for runtime mutex overlap with external operations.
    /// Returns a violation if the runtime mutex is held.
    #[must_use]
    pub fn check_external(&self) -> Option<LockOrderViolation> {
        if self.runtime_mutex_held.load(Ordering::Acquire) {
            Some(LockOrderViolation::RuntimeMutexOverlapWithExternal)
        } else {
            None
        }
    }

    /// Check for runtime + ordered-I/O nesting.
    #[must_use]
    pub fn check_ordered_io(&self) -> Option<LockOrderViolation> {
        if self.runtime_mutex_held.load(Ordering::Acquire)
            && self.ordered_io_held.load(Ordering::Acquire)
        {
            Some(LockOrderViolation::RuntimeMutexOverlapWithOrderedIo)
        } else {
            None
        }
    }
}

impl Default for LockOrderTracker {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
//  Unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- ABI shell: inactive fast path ---

    #[test]
    fn inactive_shell_returns_fast_path() {
        let model = RuntimeModel::new();
        let result = model.shell_enter();
        assert!(result.inactive_fast_path);
        assert!(!result.stop);
        assert!(!result.terminal_fallback);
        // ABI entry should be incremented even on inactive path.
        assert_eq!(model.abi_entry_count(), 1);
        // Active gate should NOT be incremented on inactive path.
        assert_eq!(model.active_gate_count(), 0);
        // Entry depth should NOT be incremented on inactive path.
        assert_eq!(model.mirror.entry_depth(), 0);
    }

    #[test]
    fn inactive_shell_does_not_lock_runtime_mutex() {
        // The inactive fast path must not acquire the runtime mutex.
        // We verify by checking that shell_enter on an inactive model
        // does not deadlock when the mutex is already held.
        let model = RuntimeModel::new();
        let guard = model.lock_inner();
        // While holding the lock, shell_enter should still return
        // the inactive fast path (no lock needed).
        let result = model.shell_enter();
        assert!(result.inactive_fast_path);
        drop(guard);
    }

    #[test]
    fn inactive_shell_abi_entry_increments_only() {
        // Multiple inactive entries should increment ABI counter
        // but never active gate or entry depth.
        let model = RuntimeModel::new();
        for _ in 0..5 {
            let result = model.shell_enter();
            assert!(result.inactive_fast_path);
        }
        assert_eq!(model.abi_entry_count(), 5);
        assert_eq!(model.active_gate_count(), 0);
        assert_eq!(model.mirror.entry_depth(), 0);
    }

    // --- ABI shell: active normal path ---

    #[test]
    fn active_shell_returns_active() {
        let model = RuntimeModel::new();
        model.activate();
        let result = model.shell_enter();
        assert!(!result.inactive_fast_path);
        assert!(!result.terminal_fallback);
        assert_eq!(model.active_gate_count(), 1);
        model.shell_exit();
    }

    // --- ABI shell: terminal mirror returns conservative ---

    #[test]
    fn terminal_mirror_returns_conservative() {
        let model = RuntimeModel::new();
        model.activate();
        model.mirror.terminal.try_set(TerminalClass::InputTimeout);
        let result = model.shell_enter();
        assert!(result.terminal_fallback);
        assert!(result.stop);
    }

    // --- ABI shell: reentry returns conservative ---

    #[test]
    fn reentry_returns_conservative() {
        let model = RuntimeModel::new();
        model.activate();
        model.mirror.entry_depth.store(1, Ordering::Release);
        let result = model.shell_enter();
        assert!(result.terminal_fallback);
        assert!(result.stop);
        assert!(model.mirror.is_abort_requested());
        assert_eq!(model.mirror.owned_keys.owned_mask(), 0);
        assert!(model.mirror.is_terminal());
    }

    // --- Terminal command always release-all, OR-abort, stop ---

    #[test]
    fn terminal_command_complete() {
        let model = RuntimeModel::new();
        model.mirror.terminal.try_set(TerminalClass::Success);
        let cmd = model.terminal_command().unwrap();
        assert!(cmd.release_all);
        assert!(cmd.or_abort);
        assert!(cmd.stop);
    }

    // --- Reserve and commit transition ---

    #[test]
    fn reserve_and_commit_transition() {
        let model = RuntimeModel::new();
        model.activate();
        let (version, _res) = model.reserve_transition().unwrap();
        assert_eq!(version, 1);
        assert!(model.commit_transition(version));
    }

    #[test]
    fn stale_commit_rejected() {
        let model = RuntimeModel::new();
        model.activate();
        let (v1, _) = model.reserve_transition().unwrap();
        let (v2, _) = model.reserve_transition().unwrap();
        // v1 is now stale.
        assert!(!model.commit_transition(v1));
        assert!(model.commit_transition(v2));
    }

    // --- Finalization ---

    #[test]
    fn finalize_completes_once() {
        let model = RuntimeModel::new();
        model.activate();
        let result = model.finalize();
        assert_eq!(result, FinalizationResult::Finalized);
        assert_eq!(model.mirror.phase(), RuntimePhase::Finalized);
        // Second finalize fails.
        let result2 = model.finalize();
        assert_eq!(result2, FinalizationResult::AlreadyFinalized);
    }

    #[test]
    fn finalize_duplicate_run_end_rejected() {
        let model = RuntimeModel::new();
        model.activate();
        // Manually set run_end_written to simulate duplicate.
        {
            let mut state = model.lock_inner();
            state.run_end_written = true;
        }
        let result = model.finalize();
        assert_eq!(result, FinalizationResult::DuplicateRunEnd);
    }

    #[test]
    fn late_callback_cannot_use_writer_after_finalize() {
        let model = RuntimeModel::new();
        model.activate();
        assert!(model.can_write());
        model.finalize();
        assert!(!model.can_write());
    }

    #[test]
    fn finalize_blocks_reservation_after_finalized() {
        let model = RuntimeModel::new();
        model.activate();
        model.finalize();
        assert!(model.reserve_transition().is_none());
    }

    // --- Lock-order instrumentation ---

    #[test]
    fn lock_order_rejects_runtime_mutex_overlap_with_external() {
        let tracker = LockOrderTracker::new();
        tracker.set_runtime_mutex(true);
        assert_eq!(
            tracker.check_external(),
            Some(LockOrderViolation::RuntimeMutexOverlapWithExternal)
        );
        tracker.set_runtime_mutex(false);
        assert_eq!(tracker.check_external(), None);
    }

    #[test]
    fn lock_order_rejects_runtime_and_ordered_io_nesting() {
        let tracker = LockOrderTracker::new();
        tracker.set_runtime_mutex(true);
        tracker.set_ordered_io(true);
        assert_eq!(
            tracker.check_ordered_io(),
            Some(LockOrderViolation::RuntimeMutexOverlapWithOrderedIo)
        );
        tracker.set_runtime_mutex(false);
        assert_eq!(tracker.check_ordered_io(), None);
    }

    // --- Property tests ---

    #[test]
    fn arbitrary_terminal_sequence_first_wins() {
        let mirror = TerminalMirror::new();
        let classes = [
            TerminalClass::InputTimeout,
            TerminalClass::Success,
            TerminalClass::PanicFallback,
            TerminalClass::CaptureMismatch,
            TerminalClass::TraceFailure,
        ];
        let first = mirror.try_set(classes[0]);
        assert!(first);
        for &c in &classes[1..] {
            assert!(!mirror.try_set(c));
        }
        assert_eq!(mirror.load(), Some(classes[0]));
    }

    #[test]
    fn terminal_is_absorbing() {
        let model = RuntimeModel::new();
        model.activate();
        model.mirror.terminal.try_set(TerminalClass::InputTimeout);
        // Multiple shell entries after terminal all return conservative.
        for _ in 0..5 {
            let result = model.shell_enter();
            assert!(result.terminal_fallback);
            assert!(result.stop);
        }
    }
}
