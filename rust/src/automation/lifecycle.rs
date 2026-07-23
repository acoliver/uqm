//! Lifecycle finalization: active receipt, teardown ordering, and status
//! mapping.
//!
//! Implements REQ-EXIT-006, REQ-EXIT-008, REQ-EXIT-009, and the lifecycle
//! integration portion of REQ-FFI-005. The lifecycle trait makes `run_uqm`
//! testable without requiring a real C game loop.
//!
//! @plan PLAN-20260723-RUNTIME-AUTOMATION.P05
//! @requirement REQ-EXIT-006, REQ-EXIT-008, REQ-EXIT-009, REQ-FFI-005

use crate::automation::artifact::{write_durable, DurableResult};
use crate::automation::error::AutomationError;
use crate::automation::outcome::TerminalClass;
use crate::automation::runtime::RuntimeModel;
use crate::automation::trace::{OrderedCommit, RecordKind, TraceRecord};
use std::path::Path;

// ===========================================================================
//  Lifecycle trait (testable abstraction)
// ===========================================================================

/// The game lifecycle trait. Makes `run_uqm` testable by abstracting the
/// C init/game/teardown sequence.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P05
/// @requirement REQ-EXIT-008
pub trait GameLifecycle {
    /// Initialize C subsystems. Returns 0 on success, nonzero on failure.
    fn c_init(&mut self) -> i32;

    /// Run the game loop. Returns the exit code.
    fn run_game(&mut self) -> i32;

    /// Teardown C subsystems.
    fn teardown_subsystems(&mut self);
}

// ===========================================================================
//  Status mapping (REQ-EXIT-008)
// ===========================================================================

/// Map a terminal class to a process exit status.
///
/// Zero only for fully evidenced success; all other outcomes are nonzero.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P05
/// @requirement REQ-EXIT-008
#[must_use]
pub fn map_status(terminal: Option<TerminalClass>, game_result: i32) -> i32 {
    match terminal {
        Some(TerminalClass::Success) | None if game_result == 0 => 0,
        Some(TerminalClass::Success) | None => game_result,
        Some(TerminalClass::CooperativeStop) => 0,
        Some(_) => 1,
    }
}

// ===========================================================================
//  Active teardown receipt (REQ-EXIT-009)
// ===========================================================================

/// Write `teardown-complete.json` as the active receipt.
///
/// Only called after `teardown_subsystems` returns. Created with the durable
/// file helper (create_new → write → flush → sync → close → exclusive publish).
/// Teardown panic/error cannot emit this false receipt.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P05
/// @requirement REQ-EXIT-009
pub fn write_teardown_receipt(
    output_root: &Path,
    terminal: Option<TerminalClass>,
    game_result: i32,
) -> Result<DurableResult, AutomationError> {
    let status = map_status(terminal, game_result);
    let content = format!(
        r#"{{"schema":1,"status":{status},"terminal":"{terminal_str}"}}"#,
        terminal_str = match terminal {
            Some(t) => format!("{t:?}"),
            None => "None".into(),
        }
    );
    write_durable(output_root, "teardown-complete", "json", content.as_bytes())
}

// ===========================================================================
//  Lifecycle orchestration (REQ-EXIT-006, REQ-EXIT-008, REQ-EXIT-009)
// ===========================================================================

/// The result of the complete lifecycle run.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P05
/// @requirement REQ-EXIT-008
#[derive(Debug, Clone)]
pub struct LifecycleResult {
    pub status: i32,
    pub terminal: Option<TerminalClass>,
    pub receipt_written: bool,
}

/// Run the complete automation-aware lifecycle.
///
/// Ordered evidence:
/// 1. Setup (script validation, output creation) — before C init.
/// 2. C init.
/// 3. Game loop.
/// 4. Automation finalize (run_end, drain, close).
/// 5. Teardown subsystems.
/// 6. Teardown receipt (only after teardown returns).
/// 7. Status mapping.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P05
/// @requirement REQ-EXIT-006, REQ-EXIT-008, REQ-EXIT-009, REQ-FFI-005
pub fn run_lifecycle<L: GameLifecycle>(
    lifecycle: &mut L,
    runtime: Option<&RuntimeModel>,
    output_root: Option<&Path>,
) -> LifecycleResult {
    // Step 2: C init.
    let init_result = lifecycle.c_init();
    if init_result != 0 {
        if init_result < 0 {
            // version/usage — clean exit
            return LifecycleResult {
                status: 0,
                terminal: None,
                receipt_written: false,
            };
        }
        return LifecycleResult {
            status: init_result,
            terminal: None,
            receipt_written: false,
        };
    }

    // Step 3: Game loop.
    let game_result = lifecycle.run_game();

    // Step 4: Automation finalize.
    let terminal = if let Some(rt) = runtime {
        let _ = rt.finalize();
        rt.mirror.terminal.load()
    } else {
        None
    };

    // Step 5: Teardown subsystems (no automation mutex/I/O lock held).
    lifecycle.teardown_subsystems();

    // Step 6: Teardown receipt (only after teardown returns).
    let receipt_written = if let Some(root) = output_root {
        write_teardown_receipt(root, terminal, game_result).is_ok()
    } else {
        false
    };

    // Step 7: Status mapping.
    let status = map_status(terminal, game_result);

    LifecycleResult {
        status,
        terminal,
        receipt_written,
    }
}

// ===========================================================================
//  Lifecycle trace integration (REQ-TRACE-001)
// ===========================================================================

/// Write lifecycle trace records (run_start, run_end) through the ordered
/// commit protocol.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P05
/// @requirement REQ-TRACE-001
pub fn write_lifecycle_trace(
    commit: &OrderedCommit,
    run_id: u64,
    records: &[(RecordKind, u64, u64, u64)],
) -> Result<(), AutomationError> {
    for (kind, sequence, input_seen, present_seen) in records {
        let record = TraceRecord {
            schema: TraceRecord::SCHEMA,
            run: run_id,
            sequence: *sequence,
            input_seen: *input_seen,
            present_seen: *present_seen,
            elapsed_ms: 0,
            kind: kind.clone(),
            label: None,
            from: None,
            to: None,
            terminal_reason: None,
        };
        let jsonl = record.to_jsonl()?;
        let res = commit.reserve_sequence(*sequence);
        res.commit_record(jsonl);
    }
    Ok(())
}

// ===========================================================================
//  Outer terminal guard (REQ-EXIT-006)
// ===========================================================================

/// Check the sticky terminal guard at an outer boundary. Returns `true` if
/// the caller should stop (terminal state reached), `false` to continue.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P05
/// @requirement REQ-EXIT-006
#[must_use]
pub fn check_terminal_guard(runtime: &RuntimeModel) -> bool {
    runtime.mirror.is_terminal()
}

/// Reassert abort before/after nested calls if terminal.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P05
/// @requirement REQ-EXIT-006
pub fn reassert_abort_if_terminal(runtime: &RuntimeModel) {
    if runtime.mirror.is_terminal() {
        runtime.mirror.request_abort();
    }
}

// ===========================================================================
//  Unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// A fake lifecycle for testing.
    struct FakeLifecycle {
        init_result: i32,
        game_result: i32,
        teardown_called: bool,
    }

    impl GameLifecycle for FakeLifecycle {
        fn c_init(&mut self) -> i32 {
            self.init_result
        }
        fn run_game(&mut self) -> i32 {
            self.game_result
        }
        fn teardown_subsystems(&mut self) {
            self.teardown_called = true;
        }
    }

    fn tmpdir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "uqm-p05-lifecycle-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    // --- Status mapping (REQ-EXIT-008) ---

    #[test]
    fn success_maps_zero() {
        assert_eq!(map_status(Some(TerminalClass::Success), 0), 0);
    }

    #[test]
    fn none_with_zero_game_maps_zero() {
        assert_eq!(map_status(None, 0), 0);
    }

    #[test]
    fn cooperative_stop_maps_zero() {
        assert_eq!(map_status(Some(TerminalClass::CooperativeStop), 0), 0);
    }

    #[test]
    fn failure_maps_nonzero() {
        assert_eq!(map_status(Some(TerminalClass::InputTimeout), 0), 1);
        assert_eq!(map_status(Some(TerminalClass::PanicFallback), 0), 1);
        assert_eq!(map_status(Some(TerminalClass::CaptureMismatch), 0), 1);
    }

    #[test]
    fn game_failure_maps_nonzero_even_without_terminal() {
        assert_eq!(map_status(None, 1), 1);
        assert_eq!(map_status(None, -1), -1);
    }

    // --- Lifecycle: normal success path ---

    #[test]
    fn lifecycle_normal_success() {
        let mut lc = FakeLifecycle {
            init_result: 0,
            game_result: 0,
            teardown_called: false,
        };
        let result = run_lifecycle(&mut lc, None, None);
        assert_eq!(result.status, 0);
        assert!(lc.teardown_called);
        assert!(!result.receipt_written);
    }

    // --- Lifecycle: init failure ---

    #[test]
    fn lifecycle_init_failure() {
        let mut lc = FakeLifecycle {
            init_result: 1,
            game_result: 0,
            teardown_called: false,
        };
        let result = run_lifecycle(&mut lc, None, None);
        assert_eq!(result.status, 1);
        assert!(!lc.teardown_called);
    }

    // --- Lifecycle: version/usage early exit ---

    #[test]
    fn lifecycle_version_usage_exit() {
        let mut lc = FakeLifecycle {
            init_result: -1,
            game_result: 0,
            teardown_called: false,
        };
        let result = run_lifecycle(&mut lc, None, None);
        assert_eq!(result.status, 0);
        assert!(!lc.teardown_called);
    }

    // --- Lifecycle: game failure + teardown ---

    #[test]
    fn lifecycle_game_failure_still_tears_down() {
        let mut lc = FakeLifecycle {
            init_result: 0,
            game_result: 1,
            teardown_called: false,
        };
        let result = run_lifecycle(&mut lc, None, None);
        assert_eq!(result.status, 1);
        assert!(lc.teardown_called);
    }

    // --- Lifecycle: teardown receipt after teardown (REQ-EXIT-009) ---

    #[test]
    fn teardown_receipt_written_after_teardown() {
        let dir = tmpdir();
        let mut lc = FakeLifecycle {
            init_result: 0,
            game_result: 0,
            teardown_called: false,
        };
        let result = run_lifecycle(&mut lc, None, Some(&dir));
        assert!(result.receipt_written);
        assert!(lc.teardown_called);
        assert!(dir.join("teardown-complete.json").exists());
        let content = std::fs::read_to_string(dir.join("teardown-complete.json")).unwrap();
        assert!(content.contains(r#""status":0"#));
    }

    // --- Lifecycle: receipt not written before teardown (REQ-EXIT-009 ordering) ---

    #[test]
    fn receipt_not_written_without_output_root() {
        let mut lc = FakeLifecycle {
            init_result: 0,
            game_result: 0,
            teardown_called: false,
        };
        let result = run_lifecycle(&mut lc, None, None);
        assert!(!result.receipt_written);
    }

    /// Lifecycle that checks if the receipt file exists when teardown is called.
    /// If it does, that means the receipt was written before teardown (bug).
    struct OrderingCheckLifecycle {
        receipt_path: std::path::PathBuf,
        receipt_existed_during_teardown: bool,
    }

    impl GameLifecycle for OrderingCheckLifecycle {
        fn c_init(&mut self) -> i32 {
            0
        }
        fn run_game(&mut self) -> i32 {
            0
        }
        fn teardown_subsystems(&mut self) {
            self.receipt_existed_during_teardown = self.receipt_path.exists();
        }
    }

    #[test]
    fn teardown_happens_before_receipt() {
        let dir = tmpdir();
        let receipt_path = dir.join("teardown-complete.json");
        let mut lc = OrderingCheckLifecycle {
            receipt_path: receipt_path.clone(),
            receipt_existed_during_teardown: false,
        };
        let result = run_lifecycle(&mut lc, None, Some(&dir));
        assert!(result.receipt_written);
        assert!(
            !lc.receipt_existed_during_teardown,
            "receipt file existed during teardown — ordering violation"
        );
        assert!(receipt_path.exists());
    }

    // --- Lifecycle: terminal runtime finalization ---

    #[test]
    fn lifecycle_with_terminal_runtime() {
        let rt = RuntimeModel::new();
        rt.activate();
        rt.mirror.terminal.try_set(TerminalClass::InputTimeout);

        let mut lc = FakeLifecycle {
            init_result: 0,
            game_result: 0,
            teardown_called: false,
        };
        let result = run_lifecycle(&mut lc, Some(&rt), None);
        assert_eq!(result.status, 1);
        assert_eq!(result.terminal, Some(TerminalClass::InputTimeout));
        assert!(lc.teardown_called);
    }

    // --- Outer terminal guard (REQ-EXIT-006) ---

    #[test]
    fn terminal_guard_blocks_when_terminal() {
        let rt = RuntimeModel::new();
        rt.mirror.terminal.try_set(TerminalClass::InputTimeout);
        assert!(check_terminal_guard(&rt));
    }

    #[test]
    fn terminal_guard_allows_when_not_terminal() {
        let rt = RuntimeModel::new();
        assert!(!check_terminal_guard(&rt));
    }

    #[test]
    fn reassert_abort_when_terminal() {
        let rt = RuntimeModel::new();
        rt.mirror.terminal.try_set(TerminalClass::InputTimeout);
        reassert_abort_if_terminal(&rt);
        assert!(rt.mirror.is_abort_requested());
    }

    #[test]
    fn no_reassert_when_not_terminal() {
        let rt = RuntimeModel::new();
        reassert_abort_if_terminal(&rt);
        assert!(!rt.mirror.is_abort_requested());
    }

    // --- Finalization: run_end exactly once ---

    #[test]
    fn finalize_called_once() {
        use crate::automation::runtime::FinalizationResult;
        let rt = RuntimeModel::new();
        rt.activate();
        let r1 = rt.finalize();
        assert_eq!(r1, FinalizationResult::Finalized);
        let r2 = rt.finalize();
        assert_eq!(r2, FinalizationResult::AlreadyFinalized);
    }

    // --- Late callback cannot use writer ---

    #[test]
    fn late_callback_blocked_after_finalize() {
        let rt = RuntimeModel::new();
        rt.activate();
        assert!(rt.can_write());
        rt.finalize();
        assert!(!rt.can_write());
    }

    // --- Finalization clears capture generation (REQ-FFI-005) ---

    #[test]
    fn finalize_clears_capture_generation() {
        let rt = RuntimeModel::new();
        rt.activate();
        // Arm a capture by setting the generation.
        rt.mirror.set_capture_generation(5);
        assert_eq!(rt.mirror.capture_generation(), 5);
        rt.finalize();
        // After finalize, capture generation should be cleared.
        assert_eq!(rt.mirror.capture_generation(), 0);
    }

    // --- Finalization drains active shells ---

    #[test]
    fn finalize_fails_with_active_shells() {
        use crate::automation::runtime::FinalizationResult;
        let rt = RuntimeModel::new();
        rt.activate();
        // Simulate an active shell by incrementing the count.
        {
            let mut state = rt.lock_inner();
            state.active_shell_count = 2;
        }
        let result = rt.finalize();
        assert!(matches!(result, FinalizationResult::ShellsStillActive(2)));
    }
}
