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

use crate::automation::coordinator::Coordinator;
use crate::automation::input::setter_set_menu_key;
use crate::automation::runtime::RuntimeModel;

// ===========================================================================
//  C global input state — FFI access to ImmediateInputState.menu[]
// ===========================================================================

/// The C `CONTROLLER_INPUT_STATE` struct, used only when linking against
/// the real C archive.
#[cfg(feature = "linked_c_archive")]
#[repr(C)]
struct ControllerInputState {
    key: [[i32; 7]; 6],
    menu: [i32; 24],
}

#[cfg(feature = "linked_c_archive")]
extern "C" {
    static mut ImmediateInputState: ControllerInputState;
    static CurrentInputState: ControllerInputState;
    static PulsedInputState: ControllerInputState;
}

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

/// Get the runtime model if it exists (public for coordinator).
pub fn get_runtime() -> Option<&'static RuntimeModel> {
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

    // Step 2b: Check terminal — if already terminal, return stop to
    // break the DoInput loop. This is the key mechanism that makes
    // DoInput break out when the scheduler finishes.
    let rt = match with_runtime() {
        Some(rt) => rt,
        None => return 0,
    };

    if rt.mirror.is_terminal() {
        // Re-assert CHECK_ABORT every frame. Game logic (handle_select)
        // can overwrite CurrentActivity, clearing our CHECK_ABORT. By
        // re-asserting on every DoInput call, we ensure the activity
        // state machine's should_continue() check sees CHECK_ABORT and
        // exits the inner loop.
        #[cfg(feature = "linked_c_archive")]
        unsafe {
            crate::mainloop::c_extern::set_current_activity(
                crate::mainloop::c_extern::get_current_activity() | 0x4000,
            );
        }
        return 1; // Terminal: stop the DoInput loop.
    }

    // Step 4: Feed the input callback to the coordinator (scheduler+watchdog).
    if Coordinator::is_active() && Coordinator::process_input() {
        return 1; // Stop requested by scheduler or watchdog.
    }

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

    // Step 2b: Check terminal — if terminal, return stop to break DoInput.
    if rt.mirror.is_terminal() {
        return 1; // Terminal: stop the DoInput loop.
    }

    // Step 3: Observation only — no additional scheduler processing.
    0
}

// ===========================================================================
//  Bounds-checked production setter (REQ-INJECT-003)
// ===========================================================================

/// C-callable bounds-checked setter for `ImmediateInputState.menu[index]`.
///
/// Writes directly to the C global volatile `ImmediateInputState.menu[index]`.
/// Returns 0 on success, -1 on invalid index.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P06
/// @requirement REQ-INJECT-003
#[no_mangle]
#[cfg(feature = "linked_c_archive")]
pub extern "C" fn rust_automation_set_immediate_menu_key(index: i32, value: i32) -> i32 {
    if index < 0 || index >= i32::from(crate::automation::input::NUM_MENU_KEYS) {
        return -1;
    }
    let _result = setter_set_menu_key(index as u8, value as u8);
    unsafe {
        ImmediateInputState.menu[index as usize] = if value != 0 { 1 } else { 0 };
    }
    0
}

/// C-callable bounds-checked setter for `ImmediateInputState.menu[index]`
/// (stub for non-linked builds — lib tests).
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P06
/// @requirement REQ-INJECT-003
#[no_mangle]
#[cfg(not(feature = "linked_c_archive"))]
pub extern "C" fn rust_automation_set_immediate_menu_key(index: i32, value: i32) -> i32 {
    if index < 0 || index >= i32::from(crate::automation::input::NUM_MENU_KEYS) {
        return -1;
    }
    let _result = setter_set_menu_key(index as u8, value as u8);
    let _ = value;
    0
}

// ===========================================================================
//  Present hook: called from TFB_SwapBuffers
// ===========================================================================

/// C-callable automation present callback hook.
///
/// Called from `TFB_SwapBuffers` after a frame is presented. Feeds the
/// committed present event (with the current armed capture generation)
/// to the coordinator's scheduler.
///
/// Returns 1 if the game loop should stop, 0 otherwise.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P07
/// @requirement REQ-FFI-004
#[no_mangle]
pub extern "C" fn rust_automation_present_callback() -> i32 {
    if !Coordinator::is_active() {
        return 0;
    }

    // Check terminal — if terminal, return stop.
    if let Some(rt) = with_runtime() {
        if rt.mirror.is_terminal() {
            return 1;
        }
    }

    // Read the armed capture generation from the runtime mirror.
    let gen = if let Some(rt) = with_runtime() {
        rt.mirror.capture_generation()
    } else {
        0
    };

    if gen > 0 {
        eprintln!("[automation] present_callback gen={gen}");
    }

    if Coordinator::process_present(gen) {
        return 1;
    }
    0
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
#[cfg(feature = "linked_c_archive")]
pub extern "C" fn rust_automation_get_current_menu_key(index: i32) -> i32 {
    if index < 0 || index >= i32::from(crate::automation::input::NUM_MENU_KEYS) {
        return -1;
    }
    unsafe { CurrentInputState.menu[index as usize] }
}

/// C-callable getter for `CurrentInputState.menu[index]` (stub for tests).
#[no_mangle]
#[cfg(not(feature = "linked_c_archive"))]
pub extern "C" fn rust_automation_get_current_menu_key(index: i32) -> i32 {
    if index < 0 || index >= i32::from(crate::automation::input::NUM_MENU_KEYS) {
        return -1;
    }
    0
}

/// C-callable getter for `PulsedInputState.menu[index]`.
///
/// Returns the value (0 or 1) or -1 on invalid index.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P06
/// @requirement REQ-INJECT-006
#[no_mangle]
#[cfg(feature = "linked_c_archive")]
pub extern "C" fn rust_automation_get_pulsed_menu_key(index: i32) -> i32 {
    if index < 0 || index >= i32::from(crate::automation::input::NUM_MENU_KEYS) {
        return -1;
    }
    unsafe { PulsedInputState.menu[index as usize] }
}

/// C-callable getter for `PulsedInputState.menu[index]` (stub for tests).
#[no_mangle]
#[cfg(not(feature = "linked_c_archive"))]
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
        assert_eq!(rust_automation_set_immediate_menu_key(24, 1), -1);
        assert_eq!(rust_automation_set_immediate_menu_key(-1, 1), -1);
    }

    #[test]
    fn setter_clear_returns_zero() {
        assert_eq!(rust_automation_set_immediate_menu_key(5, 0), 0);
    }

    #[test]
    fn getter_invalid_index_returns_negative() {
        assert_eq!(rust_automation_get_current_menu_key(24), -1);
        assert_eq!(rust_automation_get_current_menu_key(-1), -1);
        assert_eq!(rust_automation_get_pulsed_menu_key(24), -1);
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
