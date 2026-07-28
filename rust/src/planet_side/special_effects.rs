//! Typed handoff for effects requested by transitional special-system pickups.
//!
//! Selected-system callbacks execute synchronously on the game thread. These
//! exports replace the former `lander.c` globals and turn callback requests
//! into a value that the Rust collision adapter applies to its session.

use std::cell::RefCell;

/// Effects emitted while one selected-system pickup callback is running.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SpecialPickupEffects {
    pub crew_killed: u8,
    pub takeoff_requested: bool,
}

#[derive(Debug, Clone, Copy, Default)]
struct ActiveEffects {
    crew: u8,
    effects: SpecialPickupEffects,
    active: bool,
}

thread_local! {
    static ACTIVE: RefCell<ActiveEffects> = RefCell::new(ActiveEffects::default());
}

pub(crate) fn begin(crew: u8) {
    ACTIVE.with(|active| {
        *active.borrow_mut() = ActiveEffects {
            crew,
            effects: SpecialPickupEffects::default(),
            active: true,
        };
    });
}

#[must_use]
pub(crate) fn finish() -> SpecialPickupEffects {
    ACTIVE.with(|active| {
        let mut active = active.borrow_mut();
        let effects = active.effects;
        *active = ActiveEffects::default();
        effects
    })
}

/// Transitional symbol used by selected-system generators to request takeoff.
#[no_mangle]
pub extern "C" fn SetLanderTakeoff() {
    ACTIVE.with(|active| {
        let mut active = active.borrow_mut();
        if active.active {
            active.effects.takeoff_requested = true;
        }
    });
}

/// Transitional symbol used by special encounters to kill lander crew.
///
/// The period controls presentation in the legacy implementation. Gameplay is
/// synchronous here, so it does not alter the deterministic crew effect.
#[no_mangle]
pub extern "C" fn KillLanderCrewSeq(num_killed: u16, _period: u32) -> bool {
    ACTIVE.with(|active| {
        let mut active = active.borrow_mut();
        if !active.active {
            return false;
        }
        let requested = u8::try_from(num_killed).unwrap_or(u8::MAX);
        let killed = active.crew.min(requested);
        active.crew -= killed;
        active.effects.crew_killed = active.effects.crew_killed.saturating_add(killed);
        active.crew != 0
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn callback_effects_are_scoped_to_one_pickup() {
        begin(5);
        assert!(KillLanderCrewSeq(2, 10));
        SetLanderTakeoff();
        assert_eq!(
            finish(),
            SpecialPickupEffects {
                crew_killed: 2,
                takeoff_requested: true,
            }
        );
        assert_eq!(finish(), SpecialPickupEffects::default());
    }

    #[test]
    fn fatal_crew_sequence_reports_no_survivor() {
        begin(3);
        assert!(!KillLanderCrewSeq(8, 10));
        assert_eq!(finish().crew_killed, 3);
    }

    #[test]
    fn callbacks_outside_pickup_do_not_leak_effects() {
        SetLanderTakeoff();
        assert!(!KillLanderCrewSeq(1, 10));
        assert_eq!(finish(), SpecialPickupEffects::default());
    }
}
