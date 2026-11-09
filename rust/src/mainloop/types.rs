//! Activity type definitions for the UQM main loop.
//!
//! Defines the Rust-side representations of the C `ACTIVITY`, `BOOLEAN`,
//! and related types used by the main loop's activity state machine.
//!
//! # ABI Rules (verified against `libs/compiler.h` and `globdata.h`)
//!
//! - `BOOLEAN` is a C `enum` → ABI is `int` (4 bytes). Rust uses `CBoolean`.
//! - `ACTIVITY` / `UWORD` / `COUNT` are `uint16` → Rust `u16`.
//! - `BYTE` is `uint8` → Rust `u8`.
//! - Activity flags use the `MAKE_WORD(lo, hi)` macro:
//!   `MAKE_WORD(lo, hi) = ((hi << 8) | lo)`.
//!   So `CHECK_LOAD = MAKE_WORD(0, 1<<4) = 0x1000`.
//!
//! @plan PLAN-20260707-MAINLOOP.P03
//! @requirement REQ-ML-003

use std::os::raw::c_int;

/// C `BOOLEAN` type — a C `enum` whose ABI is `int` (4 bytes).
///
/// This MUST NOT be confused with Rust `bool` (1 byte) or C99 `bool`.
/// Use `CBoolean` for any value crossing the FFI boundary that the C
/// side treats as `BOOLEAN`.
///
/// @plan PLAN-20260707-MAINLOOP.P03
pub type CBoolean = c_int;

pub use crate::engine::world::activity::{
    activity_flags, ActivityFlags, ActivityKind, ActivityValue,
};

// ---------------------------------------------------------------------------
// Unit tests — Tier 1 (pure Rust, no C linkage)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// @plan PLAN-20260707-MAINLOOP.P03
    /// @requirement REQ-ML-003
    #[test]
    fn test_cboolean_is_four_bytes() {
        // BOOLEAN is a C enum = int, which is 4 bytes on all supported platforms.
        assert_eq!(std::mem::size_of::<CBoolean>(), 4);
    }

    /// @plan PLAN-20260707-MAINLOOP.P03
    /// @requirement REQ-ML-003
    #[test]
    fn test_activity_value_is_two_bytes() {
        // ACTIVITY = UWORD = uint16_t → 2 bytes.
        assert_eq!(std::mem::size_of::<ActivityValue>(), 2);
    }

    /// @plan PLAN-20260707-MAINLOOP.P03
    /// @requirement REQ-ML-003
    #[test]
    fn test_check_load_value() {
        // MAKE_WORD(0, 1<<4) = (16 << 8) | 0 = 0x1000.
        assert_eq!(activity_flags::CHECK_LOAD, 0x1000);
    }

    /// @plan PLAN-20260707-MAINLOOP.P03
    /// @requirement REQ-ML-003
    #[test]
    fn test_all_flag_values() {
        assert_eq!(activity_flags::CHECK_PAUSE, 0x0100);
        assert_eq!(activity_flags::IN_BATTLE, 0x0200);
        assert_eq!(activity_flags::START_ENCOUNTER, 0x0400);
        assert_eq!(activity_flags::START_INTERPLANETARY, 0x0800);
        assert_eq!(activity_flags::CHECK_LOAD, 0x1000);
        assert_eq!(activity_flags::CHECK_RESTART, 0x2000);
        assert_eq!(activity_flags::CHECK_ABORT, 0x4000);
    }

    /// @plan PLAN-20260707-MAINLOOP.P03
    /// @requirement REQ-ML-003
    #[test]
    fn test_activity_value_has_flag() {
        let av = ActivityValue(0x0403); // IN_ENCOUNTER | START_ENCOUNTER
        assert!(av.has_flag(activity_flags::START_ENCOUNTER));
        assert!(!av.has_flag(activity_flags::CHECK_ABORT));
        assert!(!av.has_flag(activity_flags::CHECK_LOAD));
    }

    /// @plan PLAN-20260707-MAINLOOP.P03
    /// @requirement REQ-ML-003
    #[test]
    fn test_activity_value_set_flag() {
        let av = ActivityValue(0x0002); // IN_ENCOUNTER only
        assert!(!av.has_flag(activity_flags::CHECK_LOAD));
        let av2 = av.set_flag(activity_flags::CHECK_LOAD);
        assert!(av2.has_flag(activity_flags::CHECK_LOAD));
        // Original is unchanged (Copy semantics).
        assert!(!av.has_flag(activity_flags::CHECK_LOAD));
    }

    /// @plan PLAN-20260707-MAINLOOP.P03
    /// @requirement REQ-ML-003
    #[test]
    fn test_activity_value_clear_flag() {
        let av = ActivityValue(0x1002); // IN_ENCOUNTER | CHECK_LOAD
        assert!(av.has_flag(activity_flags::CHECK_LOAD));
        let av2 = av.clear_flag(activity_flags::CHECK_LOAD);
        assert!(!av2.has_flag(activity_flags::CHECK_LOAD));
        // Kind is preserved.
        assert_eq!(av2.kind(), ActivityKind::InEncounter);
    }

    /// @plan PLAN-20260707-MAINLOOP.P03
    /// @requirement REQ-ML-003
    #[test]
    fn test_activity_value_kind_extraction() {
        assert_eq!(ActivityValue(0x0000).kind(), ActivityKind::SuperMelee);
        assert_eq!(ActivityValue(0x0001).kind(), ActivityKind::InLastBattle);
        assert_eq!(ActivityValue(0x0002).kind(), ActivityKind::InEncounter);
        assert_eq!(ActivityValue(0x0003).kind(), ActivityKind::InHyperspace);
        assert_eq!(ActivityValue(0x0004).kind(), ActivityKind::InInterplanetary);
        assert_eq!(ActivityValue(0x0005).kind(), ActivityKind::WonLastBattle);
        assert_eq!(ActivityValue(0x0006).kind(), ActivityKind::InQuasispace);
        assert_eq!(ActivityValue(0x0007).kind(), ActivityKind::InPlanetOrbit);
        assert_eq!(ActivityValue(0x0008).kind(), ActivityKind::InStarbase);
        // Unknown kind byte.
        assert_eq!(ActivityValue(0x0009).kind(), ActivityKind::Unknown);
    }

    /// @plan PLAN-20260707-MAINLOOP.P03
    /// @requirement REQ-ML-003
    #[test]
    fn test_activity_value_kind_ignores_flags() {
        // High byte (flags) should not affect kind extraction.
        let av = ActivityValue(0x1003); // IN_HYPERSPACE | CHECK_LOAD
        assert_eq!(av.kind(), ActivityKind::InHyperspace);
        assert_eq!(av.kind_raw(), 3);
    }

    /// @plan PLAN-20260707-MAINLOOP.P03
    /// @requirement REQ-ML-003
    #[test]
    fn test_activity_value_flags_extraction() {
        let av = ActivityValue(0x1403); // IN_HYPERSPACE | CHECK_LOAD | START_ENCOUNTER
        let flags = av.flags();
        assert!(flags & activity_flags::CHECK_LOAD != 0);
        assert!(flags & activity_flags::START_ENCOUNTER != 0);
    }

    /// @plan PLAN-20260707-MAINLOOP.P03
    /// @requirement REQ-ML-003
    #[test]
    fn test_activity_value_from_kind_and_flags() {
        let av = ActivityValue::from_kind_and_flags(
            ActivityKind::InEncounter,
            activity_flags::START_ENCOUNTER,
        );
        assert_eq!(av.0, 0x0402);
        assert_eq!(av.kind(), ActivityKind::InEncounter);
        assert!(av.has_flag(activity_flags::START_ENCOUNTER));
    }

    /// @plan PLAN-20260707-MAINLOOP.P03
    /// @requirement REQ-ML-003
    #[test]
    fn test_activity_value_conversions() {
        let av = ActivityValue::from(0x0403u16);
        assert_eq!(u16::from(av), 0x0403);
        let av2 = ActivityValue::new(0x1000);
        assert_eq!(u16::from(av2), 0x1000);
    }

    /// @plan PLAN-20260707-MAINLOOP.P03
    /// @requirement REQ-ML-003
    #[test]
    fn test_activity_kind_from_raw() {
        assert_eq!(ActivityKind::from_raw(0), ActivityKind::SuperMelee);
        assert_eq!(ActivityKind::from_raw(5), ActivityKind::WonLastBattle);
        assert_eq!(ActivityKind::from_raw(9), ActivityKind::Unknown);
        assert_eq!(ActivityKind::from_raw(255), ActivityKind::Unknown);
    }

    /// @plan PLAN-20260707-MAINLOOP.P03
    /// @requirement REQ-ML-003
    #[test]
    fn test_flag_arithmetic_idempotency() {
        // Setting a flag twice is the same as setting it once.
        let av = ActivityValue(0);
        let once = av.set_flag(activity_flags::CHECK_LOAD);
        let twice = once.set_flag(activity_flags::CHECK_LOAD);
        assert_eq!(once, twice);
    }

    /// @plan PLAN-20260707-MAINLOOP.P03
    /// @requirement REQ-ML-003
    #[test]
    fn test_clear_flag_on_unset_is_noop() {
        let av = ActivityValue(0x0002);
        let cleared = av.clear_flag(activity_flags::CHECK_LOAD);
        assert_eq!(av, cleared);
    }
}
