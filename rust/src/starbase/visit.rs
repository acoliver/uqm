//! Arrival time passage and post-visit cleanup.
//!
//! Ports the pure decisions of `VisitStarBase` (`starbase.c:430-515`),
//! `DoTimePassage` (`starbase.c:422-428`) and `CleanupAfterStarBase`
//! (`starbase.c:517-529`). Nothing here reads or advances a clock. The
//! functions state what the reference game does and leave the doing to the
//! caller.

use crate::mainloop::types::{activity_flags, ActivityKind, ActivityValue};

/// `LOST_DAYS` (`starbase.c:425`): game days the arrival transport takes.
pub const LOST_DAYS: u32 = 14;
/// `DoTimePassage` fades to black over `ONE_SECOND * 2` (`starbase.c:426`).
pub const TIME_PASSAGE_FADE_SECONDS: u32 = 2;
/// `SET_GAME_STATE (GLOBAL_FLAGS_AND_DATA, (BYTE)~0)`: the value that marks
/// the player as inside the Starbase (`starbase.c:363`, `443`, `483`, `503`).
pub const IN_STARBASE_MARK: u8 = 0xFF;
/// `CHMMR_BOMB_STATE` value after the Chmmr transported the ship to the
/// Starbase (`starbase.c:438`, `489`).
pub const CHMMR_BOMB_STATE_TRANSPORTED: u8 = 2;

/// Time lost on arrival: a fade to black, then `days` on the game clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimePassage {
    pub fade_seconds: u32,
    pub days: u32,
}

/// The arrival time passage, if this visit has one.
///
/// The Starbase conversation starts at once, after `DoTimePassage`, when the
/// moonbase just became available or the Chmmr just transported the ship
/// (`starbase.c:488-496`). In a normal game this costs 14 days. A caller
/// that skips it changes the game, so there is no switch to turn it off.
#[must_use]
pub fn entry_time_passage(moonbase_on_ship: bool, chmmr_bomb_state: u8) -> Option<TimePassage> {
    (moonbase_on_ship || chmmr_bomb_state == CHMMR_BOMB_STATE_TRANSPORTED).then_some(TimePassage {
        fade_seconds: TIME_PASSAGE_FADE_SECONDS,
        days: LOST_DAYS,
    })
}

/// What `CleanupAfterStarBase` does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupOutcome {
    /// A load or abort is pending. Leave everything for it.
    Unchanged,
    /// Leave the Starbase for interplanetary space.
    Apply {
        /// New `GLOBAL_FLAGS_AND_DATA`: no longer in the Starbase.
        global_flags_and_data: u8,
        /// New `GLOBAL (CurrentActivity)`: a faked `CHECK_LOAD`.
        current_activity: ActivityValue,
        /// New `NextActivity`: `IN_INTERPLANETARY | START_INTERPLANETARY`.
        next_activity: ActivityValue,
    },
}

/// `CleanupAfterStarBase`. The main loop turns the faked `CHECK_LOAD` into
/// `CurrentActivity = NextActivity` (`mainloop::state_machine::
/// resolve_load_activity`), which lands in the solar system.
#[must_use]
pub fn cleanup_after_starbase(current_activity: ActivityValue) -> CleanupOutcome {
    let pending = current_activity.0 & (activity_flags::CHECK_LOAD | activity_flags::CHECK_ABORT);
    if pending != 0 {
        return CleanupOutcome::Unchanged;
    }
    CleanupOutcome::Apply {
        global_flags_and_data: 0,
        current_activity: ActivityValue::new(activity_flags::CHECK_LOAD),
        next_activity: ActivityValue::from_kind_and_flags(
            ActivityKind::InInterplanetary,
            activity_flags::START_INTERPLANETARY,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mainloop::state_machine::resolve_load_activity;

    #[test]
    fn moonbase_arrival_loses_fourteen_days_after_a_two_second_fade() {
        assert_eq!(
            entry_time_passage(true, 0),
            Some(TimePassage {
                fade_seconds: 2,
                days: 14
            })
        );
    }

    #[test]
    fn chmmr_transport_loses_fourteen_days_too() {
        assert_eq!(entry_time_passage(false, 2).map(|t| t.days), Some(14));
    }

    #[test]
    fn an_ordinary_visit_has_no_time_passage() {
        for state in [0, 1, 3, 255] {
            assert_eq!(entry_time_passage(false, state), None, "state {state}");
        }
    }

    #[test]
    fn cleanup_fakes_a_load_into_interplanetary_space() {
        assert_eq!(
            cleanup_after_starbase(ActivityValue::new(0x0004)),
            CleanupOutcome::Apply {
                global_flags_and_data: 0,
                current_activity: ActivityValue::new(0x1000),
                next_activity: ActivityValue::new(0x0804),
            }
        );
    }

    #[test]
    fn cleanup_leaves_a_pending_load_or_abort_alone() {
        for flags in [
            activity_flags::CHECK_LOAD,
            activity_flags::CHECK_ABORT,
            activity_flags::CHECK_LOAD | activity_flags::CHECK_ABORT,
        ] {
            assert_eq!(
                cleanup_after_starbase(ActivityValue::new(flags | 0x0004)),
                CleanupOutcome::Unchanged
            );
        }
    }

    #[test]
    fn the_faked_load_resolves_to_the_interplanetary_start() {
        let CleanupOutcome::Apply {
            current_activity,
            next_activity,
            ..
        } = cleanup_after_starbase(ActivityValue::new(0))
        else {
            panic!("a clean activity word must apply the cleanup");
        };
        let resolved = resolve_load_activity(current_activity, next_activity);
        assert_eq!(resolved.kind(), ActivityKind::InInterplanetary);
        assert!(resolved.has_flag(activity_flags::START_INTERPLANETARY));
    }

    #[test]
    fn starbase_mark_is_the_byte_all_ones() {
        assert_eq!(IN_STARBASE_MARK, !0u8);
    }
}
