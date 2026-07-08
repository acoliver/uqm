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

/// A raw activity value as stored in C `ACTIVITY` (`UWORD` = `uint16`).
///
/// The low byte holds the *kind* (which top-level mode the game is in),
/// while the high byte holds *flags* (transient signals like `CHECK_LOAD`,
/// `START_ENCOUNTER`, etc.).
///
/// The `#[repr(transparent)]` ensures this has the exact same layout
/// as `u16`, so it can be passed across the FFI boundary directly.
///
/// @plan PLAN-20260707-MAINLOOP.P03
/// @requirement REQ-ML-003
#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct ActivityValue(pub u16);

impl ActivityValue {
    /// Create a new `ActivityValue` from a raw `u16`.
    #[inline]
    #[must_use]
    pub const fn new(raw: u16) -> Self {
        Self(raw)
    }

    /// Create an `ActivityValue` from a kind and a set of flags.
    #[inline]
    #[must_use]
    pub const fn from_kind_and_flags(kind: ActivityKind, flags: u16) -> Self {
        Self::new((kind as u16) | (flags & KIND_MASK_HIGH_BYTE))
    }

    /// Returns `true` if the given flag (high-byte mask) is set.
    ///
    /// `flag` should be one of the `ActivityFlags::*` constants (which
    /// live in the high byte).
    #[inline]
    #[must_use]
    pub const fn has_flag(self, flag: u16) -> bool {
        (self.0 & flag) == flag
    }

    /// Returns a copy of this value with the given flag bits set.
    #[inline]
    #[must_use]
    pub const fn set_flag(self, flag: u16) -> Self {
        Self(self.0 | flag)
    }

    /// Returns a copy of this value with the given flag bits cleared.
    #[inline]
    #[must_use]
    pub const fn clear_flag(self, flag: u16) -> Self {
        Self(self.0 & !flag)
    }

    /// Extracts the *kind* portion (low byte) of this activity value.
    #[inline]
    #[must_use]
    pub const fn kind_raw(self) -> u8 {
        (self.0 & KIND_MASK_LOW_BYTE) as u8
    }

    /// Extracts the *kind* as a typed [`ActivityKind`] enum.
    ///
    /// If the low byte does not correspond to a known kind, returns
    /// [`ActivityKind::Unknown`].
    #[inline]
    #[must_use]
    pub fn kind(self) -> ActivityKind {
        ActivityKind::from_raw(self.kind_raw())
    }

    /// Extracts the *flags* portion (high byte) of this activity value.
    #[inline]
    #[must_use]
    pub const fn flags(self) -> u16 {
        self.0 & KIND_MASK_HIGH_BYTE
    }
}

impl From<u16> for ActivityValue {
    #[inline]
    fn from(raw: u16) -> Self {
        Self(raw)
    }
}

impl From<ActivityValue> for u16 {
    #[inline]
    fn from(val: ActivityValue) -> Self {
        val.0
    }
}

/// Mask selecting the low byte (kind) of an [`ActivityValue`].
const KIND_MASK_LOW_BYTE: u16 = 0x00FF;

/// Mask selecting the high byte (flags) of an [`ActivityValue`].
const KIND_MASK_HIGH_BYTE: u16 = 0xFF00;

/// The *kind* portion of an activity value (low byte).
///
/// These correspond to the C `enum` values in `globdata.h`:
/// ```c
/// enum {
///     SUPER_MELEE = 0,
///     IN_LAST_BATTLE,
///     IN_ENCOUNTER,
///     IN_HYPERSPACE,
///     IN_INTERPLANETARY,
///     WON_LAST_BATTLE,
///     IN_QUASISPACE,
///     IN_PLANET_ORBIT,
///     IN_STARBASE,
/// };
/// ```
///
/// @plan PLAN-20260707-MAINLOOP.P03
/// @requirement REQ-ML-003
#[repr(u8)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum ActivityKind {
    /// Also used while in the main menu.
    SuperMelee = 0,
    InLastBattle = 1,
    InEncounter = 2,
    /// In HyperSpace or QuasiSpace.
    InHyperspace = 3,
    InInterplanetary = 4,
    WonLastBattle = 5,
    /// Only used for save game summaries.
    InQuasispace = 6,
    /// Only used for save game summaries.
    InPlanetOrbit = 7,
    /// Only used for save game summaries.
    InStarbase = 8,
    /// Sentinel for unrecognized kind bytes.
    Unknown = 255,
}

impl ActivityKind {
    /// Convert a raw `u8` into an [`ActivityKind`].
    ///
    /// Returns [`ActivityKind::Unknown`] for values outside the known range.
    #[inline]
    #[must_use]
    pub fn from_raw(raw: u8) -> Self {
        match raw {
            0 => Self::SuperMelee,
            1 => Self::InLastBattle,
            2 => Self::InEncounter,
            3 => Self::InHyperspace,
            4 => Self::InInterplanetary,
            5 => Self::WonLastBattle,
            6 => Self::InQuasispace,
            7 => Self::InPlanetOrbit,
            8 => Self::InStarbase,
            _ => Self::Unknown,
        }
    }
}

/// Activity *flag* constants (high byte of an [`ActivityValue`]).
///
/// These are defined in `globdata.h` using `MAKE_WORD(0, bit)`:
/// ```c
/// CHECK_PAUSE         = MAKE_WORD(0, 1 << 0)  -> 0x0100
/// IN_BATTLE           = MAKE_WORD(0, 1 << 1)  -> 0x0200
/// START_ENCOUNTER     = MAKE_WORD(0, 1 << 2)  -> 0x0400
/// START_INTERPLANETARY= MAKE_WORD(0, 1 << 3)  -> 0x0800
/// CHECK_LOAD          = MAKE_WORD(0, 1 << 4)  -> 0x1000
/// CHECK_RESTART       = MAKE_WORD(0, 1 << 5)  -> 0x2000
/// CHECK_ABORT         = MAKE_WORD(0, 1 << 6)  -> 0x4000
/// ```
///
/// `MAKE_WORD(lo, hi)` = `((hi << 8) | lo)`, so each flag places its
/// bit in the high byte.
///
/// @plan PLAN-20260707-MAINLOOP.P03
/// @requirement REQ-ML-003
pub mod activity_flags {
    /// Pause requested flag (`MAKE_WORD(0, 1<<0)` = `0x0100`).
    pub const CHECK_PAUSE: u16 = 0x0100;
    /// In-battle flag (`MAKE_WORD(0, 1<<1)` = `0x0200`).
    pub const IN_BATTLE: u16 = 0x0200;
    /// Start encounter flag (`MAKE_WORD(0, 1<<2)` = `0x0400`).
    pub const START_ENCOUNTER: u16 = 0x0400;
    /// Start interplanetary flag (`MAKE_WORD(0, 1<<3)` = `0x0800`).
    pub const START_INTERPLANETARY: u16 = 0x0800;
    /// Load requested flag (`MAKE_WORD(0, 1<<4)` = `0x1000`).
    pub const CHECK_LOAD: u16 = 0x1000;
    /// Restart requested flag (`MAKE_WORD(0, 1<<5)` = `0x2000`).
    pub const CHECK_RESTART: u16 = 0x2000;
    /// Abort requested flag (`MAKE_WORD(0, 1<<6)` = `0x4000`).
    pub const CHECK_ABORT: u16 = 0x4000;

    /// Convenience: mask covering all known activity flags.
    pub const ALL_FLAGS: u16 = CHECK_PAUSE
        | IN_BATTLE
        | START_ENCOUNTER
        | START_INTERPLANETARY
        | CHECK_LOAD
        | CHECK_RESTART
        | CHECK_ABORT;
}

/// Re-exports the activity flag constants at the module root for convenience.
///
/// @plan PLAN-20260707-MAINLOOP.P03
pub use activity_flags as ActivityFlags;

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
