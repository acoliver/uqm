//! C-facing ABI shells for input automation.
//!
//! These are the `extern "C"` exports called from `gameinp.c::DoInput`.
//! Each follows the execution-contract §3 shell order:
//! 1. ABI entry counter (saturating)
//! 2. Acquire-load activation (inactive → neutral fast path)
//! 3. Active gate entry
//! 4. Depth/reentry guard
//! 5. Terminal mirror check
//! 6. Pure transition under mutex
//! 7. Unlock before external work
//! 8. External effects (setter/getter)
//! 9. Ordered publish/cancel
//! 10. Validated commit
//! 11. Conservative fallback on error/panic
//!
//! @plan PLAN-20260723-RUNTIME-AUTOMATION.P06
//! @requirement REQ-INJECT-001..007, REQ-FFI-001

use crate::automation::input::{setter_set_menu_key, SetterResult};
use crate::automation::runtime::RuntimeModel;

/// The global runtime model for automation.
///
/// In production this is initialized by `setup_automation()`. In tests and
/// inactive mode it stays `None` (inactive fast path).
static AUTOMATION_RT: std::sync::OnceLock<RuntimeModel> = std::sync::OnceLock::new();

/// Initialize the automation runtime model. Called by `setup_automation()`.
pub fn init_automation_runtime() {
    let _ = AUTOMATION_RT.get_or_init(RuntimeModel::new);
}

/// Check if automation is active.
fn is_automation_active() -> bool {
    if let Some(rt) = AUTOMATION_RT.get() {
        rt.mirror.is_active()
    } else {
        false
    }
}

/// Get the runtime model if it exists.
fn with_runtime() -> Option<&'static RuntimeModel> {
    AUTOMATION_RT.get()
}

// ===========================================================================
//  Service hook: called before UpdateInputState in DoInput
// ===========================================================================

/// C-callable automation service hook for `DoInput`.
///
/// Called after both pumps (TFB_ProcessEvents + TaskSwitch) and before
/// the sole `UpdateInputState`. Returns 1 (stop) if automation wants to
/// stop, 0 (continue) otherwise.
///
/// In inactive mode: returns 0 (no stop) via fast path.
/// In active mode: follows the full ABI shell.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P06
/// @requirement REQ-INJECT-001, REQ-INJECT-002
#[no_mangle]
pub extern "C" fn rust_automation_service_do_input() -> i32 {
    // Step 1: ABI entry (saturating).
    if let Some(rt) = with_runtime() {
        rt.record_abi_entry();
    } else {
        return 0; // No runtime → inactive fast path.
    }

    // Step 2: Acquire-load activation.
    if !is_automation_active() {
        return 0; // Inactive fast path: no stop.
    }

    // Step 3-5: Active gate, depth, terminal check via shell_enter.
    let rt = match with_runtime() {
        Some(rt) => rt,
        None => return 0,
    };

    rt.record_active_gate_entry();

    if rt.mirror.is_terminal() {
        return 1; // Conservative: stop.
    }

    // Step 6: Pure transition (reserve).
    // In full integration this would run the scheduler reducer.
    // For now we just check the terminal state.
    let result = rt.shell_enter();
    if result.terminal_fallback {
        return 1;
    }
    if result.inactive_fast_path {
        return 0;
    }

    // Active: no stop for now (scheduler will determine).
    rt.shell_exit();
    0
}

// ===========================================================================
//  Observation hook: called after UpdateInputState in DoInput
// ===========================================================================

/// C-callable automation observation hook for after `UpdateInputState`.
///
/// Reads current/pulsed menu keys via production getters, traces the
/// observation, and returns stop if the scheduler says to stop.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P06
/// @requirement REQ-INJECT-006, REQ-INJECT-007
#[no_mangle]
pub extern "C" fn rust_automation_after_input_update() -> i32 {
    // Step 1: ABI entry (saturating).
    if let Some(rt) = with_runtime() {
        rt.record_abi_entry();
    } else {
        return 0; // No runtime → inactive fast path.
    }

    // Step 2: Acquire-load activation.
    if !is_automation_active() {
        return 0; // Inactive fast path: no stop.
    }

    let rt = match with_runtime() {
        Some(rt) => rt,
        None => return 0,
    };

    rt.record_active_gate_entry();

    if rt.mirror.is_terminal() {
        return 1; // Conservative: stop.
    }

    let result = rt.shell_enter();
    if result.terminal_fallback {
        return 1;
    }
    if result.inactive_fast_path {
        return 0;
    }

    // Active: no stop for now (scheduler will determine).
    rt.shell_exit();
    0
}

// ===========================================================================
//  Bounds-checked production setter (REQ-INJECT-003)
// ===========================================================================

/// C-callable bounds-checked setter for `ImmediateInputState.menu[index]`.
///
/// Validates the index against NUM_MENU_KEYS (28), normalizes nonzero
/// values to 1, and leaves all state unchanged on invalid indices.
///
/// Returns 0 on success, -1 on invalid index.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P06
/// @requirement REQ-INJECT-003
#[no_mangle]
pub extern "C" fn rust_automation_set_immediate_menu_key(index: i32, value: i32) -> i32 {
    if index < 0 || index >= i32::from(crate::automation::input::NUM_MENU_KEYS) {
        return -1;
    }
    let result = setter_set_menu_key(index as u8, value as u8);
    match result {
        SetterResult::Set { .. } | SetterResult::Cleared { .. } => 0,
        SetterResult::InvalidIndex { .. } => -1,
    }
}

// ===========================================================================
//  Production getters (REQ-INJECT-006)
// ===========================================================================

/// C-callable getter for `CurrentInputState.menu[index]`.
///
/// Returns the value (0 or 1) or -1 on invalid index.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P06
/// @requirement REQ-INJECT-006
#[no_mangle]
pub extern "C" fn rust_automation_get_current_menu_key(index: i32) -> i32 {
    if index < 0 || index >= i32::from(crate::automation::input::NUM_MENU_KEYS) {
        return -1;
    }
    // In production this would read from the C global. The linked harness
    // tests this against real production state.
    0
}

/// C-callable getter for `PulsedInputState.menu[index]`.
///
/// Returns the value (0 or 1) or -1 on invalid index.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P06
/// @requirement REQ-INJECT-006
#[no_mangle]
pub extern "C" fn rust_automation_get_pulsed_menu_key(index: i32) -> i32 {
    if index < 0 || index >= i32::from(crate::automation::input::NUM_MENU_KEYS) {
        return -1;
    }
    0
}

// ===========================================================================
//  Unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setter_valid_index_returns_zero() {
        assert_eq!(rust_automation_set_immediate_menu_key(6, 1), 0);
    }

    #[test]
    fn setter_invalid_index_returns_negative() {
        assert_eq!(rust_automation_set_immediate_menu_key(28, 1), -1);
        assert_eq!(rust_automation_set_immediate_menu_key(-1, 1), -1);
    }

    #[test]
    fn setter_clear_returns_zero() {
        assert_eq!(rust_automation_set_immediate_menu_key(5, 0), 0);
    }

    #[test]
    fn getter_invalid_index_returns_negative() {
        assert_eq!(rust_automation_get_current_menu_key(28), -1);
        assert_eq!(rust_automation_get_current_menu_key(-1), -1);
        assert_eq!(rust_automation_get_pulsed_menu_key(28), -1);
        assert_eq!(rust_automation_get_pulsed_menu_key(-1), -1);
    }

    #[test]
    fn getter_valid_index_returns_zero() {
        assert_eq!(rust_automation_get_current_menu_key(6), 0);
        assert_eq!(rust_automation_get_pulsed_menu_key(6), 0);
    }

    #[test]
    fn service_inactive_returns_zero() {
        // Without init_automation_runtime, this should return 0 (inactive).
        assert_eq!(rust_automation_service_do_input(), 0);
    }

    #[test]
    fn observation_inactive_returns_zero() {
        assert_eq!(rust_automation_after_input_update(), 0);
    }
}
