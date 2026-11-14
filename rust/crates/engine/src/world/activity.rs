//! One live activity word with explicitly supplied observation and flag signaling.

use std::sync::{
    atomic::{AtomicU16, Ordering},
    Arc,
};

pub struct ActivityState {
    value: Arc<AtomicU16>,
}

impl ActivityState {
    pub fn new(value: u16) -> Self {
        Self {
            value: Arc::new(AtomicU16::new(value)),
        }
    }

    pub fn activity(&self) -> u16 {
        self.value.load(Ordering::Relaxed)
    }

    pub fn set_activity(&mut self, value: u16) {
        self.value.store(value, Ordering::Relaxed);
    }

    pub fn control(&self) -> ActivityControl {
        ActivityControl {
            value: self.value.clone(),
        }
    }
}

/// Reads the owner's current word and raises flags without retaining a World borrow.
#[derive(Clone)]
pub struct ActivityControl {
    value: Arc<AtomicU16>,
}

impl ActivityControl {
    pub fn activity(&self) -> u16 {
        self.value.load(Ordering::Relaxed)
    }

    /// Return the activity before raising flags, for terminal-outcome recording.
    pub fn raise_flags(&self, flags: u16) -> u16 {
        self.value.fetch_or(flags, Ordering::Relaxed)
    }
}

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

#[cfg(test)]
mod tests {
    use super::ActivityState;

    #[test]
    fn controls_read_fresh_owner_values_without_a_publication() {
        let mut state = ActivityState::new(3);
        let control = state.control();
        let other_control = control.clone();
        state.set_activity(0x1004);
        assert_eq!(control.activity(), 0x1004);
        assert_eq!(other_control.activity(), 0x1004);
        assert_eq!(state.activity(), 0x1004);
    }

    #[test]
    fn raising_flags_returns_the_preimage_and_preserves_the_activity() {
        let state = ActivityState::new(0x0403);
        let control = state.control();
        assert_eq!(control.raise_flags(0x4000), 0x0403);
        assert_eq!(state.activity(), 0x4403);
        assert_eq!(control.raise_flags(0x4000), 0x4403);
        assert_eq!(state.activity(), 0x4403);
    }

    #[test]
    fn reset_uses_the_same_word_and_does_not_affect_another_owner() {
        let mut first = ActivityState::new(0xffff);
        let second = ActivityState::new(7);
        let control = first.control();
        first.set_activity(0);
        assert_eq!(control.activity(), 0);
        assert_eq!(second.activity(), 7);
        control.raise_flags(0x4000);
        assert_eq!(first.activity(), 0x4000);
        assert_eq!(second.activity(), 7);
    }

    #[test]
    fn flag_signal_keeps_the_observer_send_and_sync_without_sharing_a_world() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<super::ActivityControl>();
        let state = ActivityState::new(3);
        let control = state.control();
        std::thread::scope(|scope| {
            scope.spawn(move || assert_eq!(control.raise_flags(0x4000), 3));
        });
        assert_eq!(state.activity(), 0x4003);
    }
}
