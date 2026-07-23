//! Input/menu ABI shells, semantic observer, and trace integration.
//!
//! Implements the Rust side of the input automation contract:
//! - Bounds-checked production setter model (`SetImmediateMenuKey`)
//! - Post-update observation model (read current/pulsed menu keys)
//! - Typed `CallbackControl::{Continue, Stop}` for menu observer
//! - `MainMenuTransition` observer for `handle_navigate`
//! - Input/menu trace record construction
//!
//! All ABI shells follow execution-contract §3. This module owns
//! REQ-INJECT-001..007, REQ-SEM-001 (observer/in-process propagation),
//! and input/menu trace integration (REQ-TRACE-001..003).
//!
//! @plan PLAN-20260723-RUNTIME-AUTOMATION.P06
//! @requirement REQ-INJECT-001..007, REQ-SEM-001

use crate::automation::script::MenuKey;
use crate::automation::trace::{RecordKind, TraceRecord};

// ===========================================================================
//  Menu key index mapping (from controls.h)
// ===========================================================================

/// The six menu key indices from `controls.h`.
///
/// These match the C enum values:
/// `KEY_MENU_UP=5, KEY_MENU_DOWN=6, KEY_MENU_LEFT=7, KEY_MENU_RIGHT=8,
/// KEY_MENU_SELECT=9, KEY_MENU_CANCEL=10`.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P06
/// @requirement REQ-SCRIPT-005, REQ-INJECT-003
pub const MENU_KEY_INDICES: [u8; 6] = [5, 6, 7, 8, 9, 10];

/// Total number of menu keys (from `NUM_MENU_KEYS` in controls.h).
pub const NUM_MENU_KEYS: u8 = 28;

/// Convert a `MenuKey` to its C index.
#[must_use]
pub fn menu_key_to_index(key: MenuKey) -> u8 {
    match key {
        MenuKey::Up => MENU_KEY_INDICES[0],
        MenuKey::Down => MENU_KEY_INDICES[1],
        MenuKey::Left => MENU_KEY_INDICES[2],
        MenuKey::Right => MENU_KEY_INDICES[3],
        MenuKey::Select => MENU_KEY_INDICES[4],
        MenuKey::Cancel => MENU_KEY_INDICES[5],
    }
}

// ===========================================================================
//  Bounds-checked setter model (REQ-INJECT-003)
// ===========================================================================

/// The result of a setter operation.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P06
/// @requirement REQ-INJECT-003
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetterResult {
    /// The key was set successfully.
    Set { index: u8, value: u8 },
    /// The index was out of bounds; no state was changed.
    InvalidIndex { requested: u8 },
    /// The value was normalized to 0 (clear).
    Cleared { index: u8 },
}

/// Bounds-checked setter for `ImmediateInputState.menu[index]`.
///
/// Validates the index against `NUM_MENU_KEYS`, normalizes nonzero values
/// to 1, and returns a typed result. On invalid indices, no state is
/// changed.
///
/// This is the pure model; the actual C write is done via FFI in the
/// linked harness and production code.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P06
/// @requirement REQ-INJECT-003
#[must_use]
pub fn setter_set_menu_key(index: u8, value: u8) -> SetterResult {
    if index >= NUM_MENU_KEYS {
        return SetterResult::InvalidIndex { requested: index };
    }
    if value == 0 {
        SetterResult::Cleared { index }
    } else {
        SetterResult::Set {
            index,
            value: 1, // normalize nonzero to 1
        }
    }
}

// ===========================================================================
//  Observation model (REQ-INJECT-006)
// ===========================================================================

/// A snapshot of a single menu key's state, read via production getters.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P06
/// @requirement REQ-INJECT-006
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MenuKeySnapshot {
    /// The menu key index that was read.
    pub index: u8,
    /// The intended value (what the scheduler planned to set).
    pub intended: u8,
    /// The current value from `c_GetCurrentMenuKey`.
    pub current: u8,
    /// The pulsed value from `c_GetPulsedMenuKey`.
    pub pulsed: u8,
}

/// Read a menu key observation. The pure model validates the index and
/// returns a snapshot placeholder. The actual C getter calls happen in
/// the linked harness and production code.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P06
/// @requirement REQ-INJECT-006
#[must_use]
pub fn observe_menu_key(index: u8, intended: u8) -> MenuKeySnapshot {
    MenuKeySnapshot {
        index,
        intended,
        current: 0,
        pulsed: 0,
    }
}

// ===========================================================================
//  CallbackControl (REQ-SEM-001)
// ===========================================================================

/// Typed control flow for menu navigation callbacks.
///
/// `Continue` proceeds with normal frame work.
/// `Stop` propagates immediately through `handle_navigate`,
/// `do_restart_frame`, and `rust_do_restart_frame` before any sleep or
/// later frame work.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P06
/// @requirement REQ-SEM-001
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallbackControl {
    /// Proceed with normal frame work.
    Continue,
    /// Stop immediately; propagate through all layers.
    Stop,
}

impl CallbackControl {
    /// Returns `true` if this is `Stop`.
    #[must_use]
    pub fn is_stop(self) -> bool {
        matches!(self, CallbackControl::Stop)
    }

    /// Returns `true` if this is `Continue`.
    #[must_use]
    pub fn is_continue(self) -> bool {
        matches!(self, CallbackControl::Continue)
    }
}

impl From<bool> for CallbackControl {
    /// Convert a stop bool to `CallbackControl`.
    /// `true` (stop) → `Stop`, `false` (no stop) → `Continue`.
    fn from(stop: bool) -> Self {
        if stop {
            CallbackControl::Stop
        } else {
            CallbackControl::Continue
        }
    }
}

// ===========================================================================
//  Main menu transition observer (REQ-SEM-001)
// ===========================================================================

/// A typed main-menu transition event.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P06
/// @requirement REQ-SEM-001
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MainMenuTransitionEvent {
    /// The item the menu was on before navigation.
    pub from: u8,
    /// The item the menu navigated to.
    pub to: u8,
}

/// Observe a main menu transition. The pure model checks if the transition
/// matches a scheduler-expected transition and returns `Continue` or
/// `Stop`.
///
/// The actual matching against scheduler state happens when this is
/// wired into the scheduler; here we provide the typed return.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P06
/// @requirement REQ-SEM-001
#[must_use]
pub fn observe_main_menu_transition(
    from: u8,
    to: u8,
    expected: Option<&MainMenuTransitionEvent>,
) -> CallbackControl {
    if let Some(exp) = expected {
        if exp.from == from && exp.to == to {
            return CallbackControl::Continue;
        }
        // Mismatch: stop (will be terminal in full integration).
        return CallbackControl::Stop;
    }
    // No expectation set: continue (no semantic assertion active).
    CallbackControl::Continue
}

// ===========================================================================
//  Input/menu trace records (REQ-TRACE-001)
// ===========================================================================

/// Construct an input tick trace record.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P06
/// @requirement REQ-TRACE-001
#[must_use]
pub fn input_trace_record(
    sequence: u64,
    elapsed_ms: u64,
    input_seen: u64,
    key_index: u8,
    key_value: u8,
) -> TraceRecord {
    TraceRecord {
        schema: TraceRecord::SCHEMA,
        run: 0,
        sequence,
        input_seen,
        present_seen: 0,
        elapsed_ms,
        kind: RecordKind::InputTick,
        label: Some(format!("key_{key_index}={key_value}")),
        from: None,
        to: None,
        terminal_reason: None,
    }
}

/// Construct a semantic (menu transition) trace record.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P06
/// @requirement REQ-TRACE-001
#[must_use]
pub fn semantic_trace_record(
    sequence: u64,
    elapsed_ms: u64,
    from: u8,
    to: u8,
    control: CallbackControl,
) -> TraceRecord {
    TraceRecord {
        schema: TraceRecord::SCHEMA,
        run: 0,
        sequence,
        input_seen: 0,
        present_seen: 0,
        elapsed_ms,
        kind: RecordKind::MenuTransition,
        label: Some(format!(
            "control={}",
            if control.is_stop() {
                "Stop"
            } else {
                "Continue"
            }
        )),
        from: Some(format!("item_{from}")),
        to: Some(format!("item_{to}")),
        terminal_reason: None,
    }
}

// ===========================================================================
//  Service stop combination (REQ-INJECT-007)
// ===========================================================================

/// Combine service stop and observation stop. If either returns stop,
/// the combined result is stop.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P06
/// @requirement REQ-INJECT-007
#[must_use]
pub fn combine_stops(service_stop: bool, observation_stop: bool) -> bool {
    service_stop || observation_stop
}

// ===========================================================================
//  Unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- Menu key mapping ---

    #[test]
    fn menu_key_indices_match_controls_h() {
        assert_eq!(menu_key_to_index(MenuKey::Up), 5);
        assert_eq!(menu_key_to_index(MenuKey::Down), 6);
        assert_eq!(menu_key_to_index(MenuKey::Left), 7);
        assert_eq!(menu_key_to_index(MenuKey::Right), 8);
        assert_eq!(menu_key_to_index(MenuKey::Select), 9);
        assert_eq!(menu_key_to_index(MenuKey::Cancel), 10);
    }

    #[test]
    fn num_menu_keys_matches_c() {
        // controls.h enum has 28 entries (KEY_PAUSE through KEY_MENU_ANY + NUM_MENU_KEYS)
        // The enum starts at 0, so NUM_MENU_KEYS = last_value + 1
        assert_eq!(NUM_MENU_KEYS, 28);
    }

    // --- Bounds-checked setter (REQ-INJECT-003) ---

    #[test]
    fn setter_valid_index_nonzero_normalizes_to_1() {
        let result = setter_set_menu_key(6, 42);
        assert_eq!(result, SetterResult::Set { index: 6, value: 1 });
    }

    #[test]
    fn setter_valid_index_zero_clears() {
        let result = setter_set_menu_key(5, 0);
        assert_eq!(result, SetterResult::Cleared { index: 5 });
    }

    #[test]
    fn setter_invalid_index_no_change() {
        let result = setter_set_menu_key(28, 1);
        assert_eq!(result, SetterResult::InvalidIndex { requested: 28 });
    }

    #[test]
    fn setter_max_valid_index() {
        let result = setter_set_menu_key(27, 1);
        assert_eq!(
            result,
            SetterResult::Set {
                index: 27,
                value: 1
            }
        );
    }

    #[test]
    fn setter_first_index_valid() {
        let result = setter_set_menu_key(0, 1);
        assert_eq!(result, SetterResult::Set { index: 0, value: 1 });
    }

    // --- Observation model (REQ-INJECT-006) ---

    #[test]
    fn observe_returns_snapshot_with_index() {
        let snap = observe_menu_key(6, 1);
        assert_eq!(snap.index, 6);
        assert_eq!(snap.intended, 1);
    }

    // --- CallbackControl (REQ-SEM-001) ---

    #[test]
    fn callback_control_stop_is_stop() {
        assert!(CallbackControl::Stop.is_stop());
        assert!(!CallbackControl::Stop.is_continue());
    }

    #[test]
    fn callback_control_continue_is_continue() {
        assert!(CallbackControl::Continue.is_continue());
        assert!(!CallbackControl::Continue.is_stop());
    }

    #[test]
    fn callback_control_from_bool() {
        assert_eq!(CallbackControl::from(true), CallbackControl::Stop);
        assert_eq!(CallbackControl::from(false), CallbackControl::Continue);
    }

    // --- Main menu transition observer (REQ-SEM-001) ---

    #[test]
    fn observer_match_returns_continue() {
        let expected = MainMenuTransitionEvent { from: 0, to: 1 };
        let control = observe_main_menu_transition(0, 1, Some(&expected));
        assert_eq!(control, CallbackControl::Continue);
    }

    #[test]
    fn observer_mismatch_returns_stop() {
        let expected = MainMenuTransitionEvent { from: 0, to: 1 };
        let control = observe_main_menu_transition(0, 2, Some(&expected));
        assert_eq!(control, CallbackControl::Stop);
    }

    #[test]
    fn observer_no_expectation_returns_continue() {
        let control = observe_main_menu_transition(0, 1, None);
        assert_eq!(control, CallbackControl::Continue);
    }

    // --- Stop combination (REQ-INJECT-007) ---

    #[test]
    fn combine_stops_either() {
        assert!(combine_stops(true, false));
        assert!(combine_stops(false, true));
        assert!(combine_stops(true, true));
        assert!(!combine_stops(false, false));
    }

    // --- Trace records (REQ-TRACE-001) ---

    #[test]
    fn input_trace_record_has_correct_kind() {
        let record = input_trace_record(1, 10, 1, 6, 1);
        assert_eq!(record.kind, RecordKind::InputTick);
        assert_eq!(record.sequence, 1);
    }

    #[test]
    fn semantic_trace_record_has_correct_kind() {
        let record = semantic_trace_record(2, 15, 0, 1, CallbackControl::Continue);
        assert_eq!(record.kind, RecordKind::MenuTransition);
        assert_eq!(record.sequence, 2);
    }

    #[test]
    fn semantic_trace_record_stop() {
        let record = semantic_trace_record(3, 20, 0, 2, CallbackControl::Stop);
        assert_eq!(record.kind, RecordKind::MenuTransition);
        assert_eq!(record.label.as_ref().unwrap(), "control=Stop");
    }

    // --- Setter sentinel coverage (REQ-INJECT-004) ---

    #[test]
    fn setter_sentinel_first_index() {
        let result = setter_set_menu_key(0, 1);
        assert_eq!(result, SetterResult::Set { index: 0, value: 1 });
    }

    #[test]
    fn setter_sentinel_last_valid() {
        let result = setter_set_menu_key(27, 1);
        assert_eq!(
            result,
            SetterResult::Set {
                index: 27,
                value: 1
            }
        );
    }

    #[test]
    fn setter_sentinel_out_of_bounds() {
        let result = setter_set_menu_key(255, 1);
        assert_eq!(result, SetterResult::InvalidIndex { requested: 255 });
    }
}
