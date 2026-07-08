//! Activity state machine for the UQM main game loop.
//!
//! Implements the activity dispatch and loop-control logic from
//! `Starcon2Main` (`starcon.c:255-322`) as pure, testable Rust
//! functions.  Every function takes its inputs as value parameters
//! (never reads C globals directly), making each one trivially
//! unit-testable without FFI.
//!
//! The game-loop driver (P06) reads C globals via FFI and passes them
//! into these functions, then performs the side effects they prescribe.
//!
//! @plan PLAN-20260707-MAINLOOP.P05
//! @requirement REQ-ML-004

use super::types::{activity_flags, ActivityKind, ActivityValue};

// ===========================================================================
//  Types
// ===========================================================================

/// What activity the game loop should execute this iteration.
///
/// Determined by [`evaluate`] from `CurrentActivity` and game state.
///
/// @plan PLAN-20260707-MAINLOOP.P05
/// @requirement REQ-ML-004
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum ActivityDecision {
    /// `InstallBombAtEarth()` — bomb ready, starbase not yet available.
    InstallBombAtEarth,
    /// `VisitStarBase()` — starbase ending or bomb-with-starbase.
    VisitStarBase,
    /// `RaceCommunication()` — talk to an alien race.
    RaceCommunication,
    /// `ExploreSolarSys()` — interplanetary exploration.
    ExploreSolarSystem,
    /// `Battle(&on_battle_frame)` — hyperspace / quasispace combat loop.
    Battle,
}

/// Side effect requested when the break condition fires.
///
/// `starcon.c:311-320`:
/// ```c
/// if (!(CurrentActivity & (CHECK_ABORT | CHECK_LOAD))
///         && (LOBYTE(CurrentActivity) == WON_LAST_BATTLE
///                 || CrewEnlisted == (COUNT)~0))
/// {
///     if (KOHR_AH_KILLED_ALL)
///         InitCommunication(BLACKURQ_CONVERSATION);
///     else if (CurrentActivity & CHECK_RESTART)
///         CurrentActivity &= ~CHECK_RESTART;
///     break;
/// }
/// ```
///
/// @plan PLAN-20260707-MAINLOOP.P05
/// @requirement REQ-ML-004
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum BreakAction {
    /// No break — keep looping.
    Continue,
    /// Break; caller must call `InitCommunication(BLACKURQ_CONVERSATION)`.
    InitBlackUrqCommunication,
    /// Break; caller must clear `CHECK_RESTART` from `CurrentActivity`.
    ClearRestart,
    /// Break; no additional side effects.
    JustBreak,
}

/// Read-only snapshot of the C globals the state machine consults.
///
/// @plan PLAN-20260707-MAINLOOP.P05
/// @requirement REQ-ML-004
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Default)]
pub struct GameStateInfo {
    /// `GET_GAME_STATE(CHMMR_BOMB_STATE)` — 0, 1, or 2.
    pub chmmr_bomb_state: u8,
    /// `GET_GAME_STATE(STARBASE_AVAILABLE)` — 0 or 1.
    pub starbase_available: u8,
    /// `GET_GAME_STATE(GLOBAL_FLAGS_AND_DATA)` — `(BYTE)~0` = 255 means all set.
    pub global_flags_and_data: u8,
    /// `GET_GAME_STATE(KOHR_AH_KILLED_ALL)` — 0 or 1.
    pub kohr_ah_killed_all: u8,
    /// `GLOBAL_SIS(CrewEnlisted)` — `0xFFFF` (`(COUNT)~0`) means dead.
    pub crew_enlisted: u16,
}

// ===========================================================================
//  Load-path predicates  (starcon.c:258-263)
// ===========================================================================

/// Whether velocity components should be zeroed before dispatching.
///
/// `starcon.c:260-261`:
/// ```c
/// if (!((CurrentActivity | NextActivity) & CHECK_LOAD))
///     ZeroVelocityComponents(&velocity);
/// ```
///
/// Returns **true** when neither `current` nor `next` carries
/// `CHECK_LOAD` — i.e. the velocity should be zeroed.
///
/// @plan PLAN-20260707-MAINLOOP.P05
/// @requirement REQ-ML-004
#[must_use]
pub fn should_zero_velocity(current: ActivityValue, next: ActivityValue) -> bool {
    !current.has_flag(activity_flags::CHECK_LOAD)
        && !next.has_flag(activity_flags::CHECK_LOAD)
}

/// When `current` carries `CHECK_LOAD`, the C code replaces it with
/// `next` (`starcon.c:262-263`):
/// ```c
/// else if (CurrentActivity & CHECK_LOAD)
///     CurrentActivity = NextActivity;
/// ```
///
/// @plan PLAN-20260707-MAINLOOP.P05
/// @requirement REQ-ML-004
#[must_use]
pub fn resolve_load_activity(
    current: ActivityValue,
    next: ActivityValue,
) -> ActivityValue {
    if current.has_flag(activity_flags::CHECK_LOAD) {
        next
    } else {
        current
    }
}

// ===========================================================================
//  Activity dispatch  (starcon.c:265-306)
// ===========================================================================

/// Decide which activity to execute based on `CurrentActivity` and game
/// state.
///
/// `starcon.c:265-306`:
/// ```c
/// if ((CurrentActivity & START_ENCOUNTER)
///         || GET_GAME_STATE(CHMMR_BOMB_STATE) == 2)
/// {
///     if (CHMMR_BOMB_STATE == 2 && !STARBASE_AVAILABLE)
///         InstallBombAtEarth();
///     else if (GLOBAL_FLAGS_AND_DATA == (BYTE)~0 || CHMMR_BOMB_STATE == 2)
///         // sets START_ENCOUNTER
///         VisitStarBase();
///     else
///         // sets START_ENCOUNTER
///         RaceCommunication();
/// }
/// else if (CurrentActivity & START_INTERPLANETARY)
///     ExploreSolarSys();
/// else
///     Battle();
/// ```
///
/// @plan PLAN-20260707-MAINLOOP.P05
/// @requirement REQ-ML-004
#[must_use]
pub fn evaluate(activity: ActivityValue, state: &GameStateInfo) -> ActivityDecision {
    // Outer gate: encounter block is entered when START_ENCOUNTER is set
    // OR the Chmmr bomb is ready (state 2).
    let in_encounter_block =
        activity.has_flag(activity_flags::START_ENCOUNTER) || state.chmmr_bomb_state == 2;

    if in_encounter_block {
        if state.chmmr_bomb_state == 2 && state.starbase_available == 0 {
            // BGD mode — bomb ready, starbase not yet available.
            ActivityDecision::InstallBombAtEarth
        } else if state.global_flags_and_data == 0xFF || state.chmmr_bomb_state == 2 {
            // Starbase ending — all flags set, or bomb ready WITH starbase.
            ActivityDecision::VisitStarBase
        } else {
            // Normal encounter — communicate with the alien race.
            ActivityDecision::RaceCommunication
        }
    } else if activity.has_flag(activity_flags::START_INTERPLANETARY) {
        ActivityDecision::ExploreSolarSystem
    } else {
        ActivityDecision::Battle
    }
}

/// Mutation applied to `CurrentActivity` **before** dispatching, based on
/// the chosen [`ActivityDecision`].
///
/// `starcon.c:276, 280, 294, 301`:
/// ```c
/// VisitStarBase:  CurrentActivity |= START_ENCOUNTER;
/// RaceCommunication: CurrentActivity |= START_ENCOUNTER;
/// ExploreSolarSys: CurrentActivity = MAKE_WORD(IN_INTERPLANETARY, 0);
/// Battle:         CurrentActivity = MAKE_WORD(IN_HYPERSPACE, 0);
/// ```
///
/// @plan PLAN-20260707-MAINLOOP.P05
/// @requirement REQ-ML-004
#[must_use]
pub fn pre_dispatch_mutate(
    activity: ActivityValue,
    decision: ActivityDecision,
) -> ActivityValue {
    match decision {
        ActivityDecision::VisitStarBase | ActivityDecision::RaceCommunication => {
            activity.set_flag(activity_flags::START_ENCOUNTER)
        }
        ActivityDecision::ExploreSolarSystem => {
            ActivityValue::from_kind_and_flags(ActivityKind::InInterplanetary, 0)
        }
        ActivityDecision::Battle => {
            ActivityValue::from_kind_and_flags(ActivityKind::InHyperspace, 0)
        }
        ActivityDecision::InstallBombAtEarth => activity,
    }
}

/// Post-dispatch clearing **for the encounter branch only**.
///
/// `starcon.c:285-290`:
/// ```c
/// if (!(CurrentActivity & (CHECK_ABORT | CHECK_LOAD)))
/// {
///     CurrentActivity &= ~START_ENCOUNTER;
///     if (LOBYTE(CurrentActivity) == IN_INTERPLANETARY)
///         CurrentActivity |= START_INTERPLANETARY;
/// }
/// ```
///
/// Returns the (possibly mutated) activity.  When `CHECK_ABORT` or
/// `CHECK_LOAD` is set the activity is returned unchanged — the flags
/// are preserved for the outer loop condition.
///
/// @plan PLAN-20260707-MAINLOOP.P05
/// @requirement REQ-ML-004
#[must_use]
pub fn post_encounter_clear(activity: ActivityValue) -> ActivityValue {
    if activity.has_flag(activity_flags::CHECK_ABORT)
        || activity.has_flag(activity_flags::CHECK_LOAD)
    {
        return activity;
    }

    let cleared = activity.clear_flag(activity_flags::START_ENCOUNTER);
    if cleared.kind() == ActivityKind::InInterplanetary {
        cleared.set_flag(activity_flags::START_INTERPLANETARY)
    } else {
        cleared
    }
}

// ===========================================================================
//  Loop-control predicates  (starcon.c:311-322)
// ===========================================================================

/// Check the break condition inside the inner `do { … } while` loop.
///
/// `starcon.c:311-320`:
/// ```c
/// if (!(CurrentActivity & (CHECK_ABORT | CHECK_LOAD))
///         && (LOBYTE(CurrentActivity) == WON_LAST_BATTLE
///                 || CrewEnlisted == (COUNT)~0))
/// {
///     if (KOHR_AH_KILLED_ALL)
///         InitCommunication(BLACKURQ_CONVERSATION);
///     else if (CurrentActivity & CHECK_RESTART)
///         CurrentActivity &= ~CHECK_RESTART;
///     break;
/// }
/// ```
///
/// @plan PLAN-20260707-MAINLOOP.P05
/// @requirement REQ-ML-004
#[must_use]
pub fn check_break(activity: ActivityValue, state: &GameStateInfo) -> BreakAction {
    // Guard: skip when CHECK_ABORT or CHECK_LOAD is set.
    if activity.has_flag(activity_flags::CHECK_ABORT)
        || activity.has_flag(activity_flags::CHECK_LOAD)
    {
        return BreakAction::Continue;
    }

    let won = activity.kind() == ActivityKind::WonLastBattle;
    let dead = state.crew_enlisted == 0xFFFF;

    if won || dead {
        if state.kohr_ah_killed_all != 0 {
            BreakAction::InitBlackUrqCommunication
        } else if activity.has_flag(activity_flags::CHECK_RESTART) {
            BreakAction::ClearRestart
        } else {
            BreakAction::JustBreak
        }
    } else {
        BreakAction::Continue
    }
}

/// Inner `do-while` continuation condition.
///
/// `starcon.c:322`:
/// ```c
/// } while (!(CurrentActivity & CHECK_ABORT));
/// ```
///
/// Returns **true** when the loop should continue (no `CHECK_ABORT`).
///
/// @plan PLAN-20260707-MAINLOOP.P05
/// @requirement REQ-ML-004
#[must_use]
pub fn should_continue(activity: ActivityValue) -> bool {
    !activity.has_flag(activity_flags::CHECK_ABORT)
}

// ===========================================================================
//  Unit tests — Tier 1 (pure Rust, no C linkage)
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn gs() -> GameStateInfo {
        GameStateInfo::default()
    }

    fn act(kind: ActivityKind, flags: u16) -> ActivityValue {
        ActivityValue::from_kind_and_flags(kind, flags)
    }

    // -----------------------------------------------------------------------
    //  should_zero_velocity
    // -----------------------------------------------------------------------

    /// @plan PLAN-20260707-MAINLOOP.P05
    /// @requirement REQ-ML-004
    #[test]
    fn test_zero_velocity_when_no_load_anywhere() {
        let cur = act(ActivityKind::InHyperspace, 0);
        let next = act(ActivityKind::InHyperspace, 0);
        assert!(should_zero_velocity(cur, next));
    }

    /// @plan PLAN-20260707-MAINLOOP.P05
    /// @requirement REQ-ML-004
    #[test]
    fn test_zero_velocity_suppressed_when_current_has_load() {
        let cur = act(ActivityKind::InHyperspace, activity_flags::CHECK_LOAD);
        let next = act(ActivityKind::InHyperspace, 0);
        assert!(!should_zero_velocity(cur, next));
    }

    /// @plan PLAN-20260707-MAINLOOP.P05
    /// @requirement REQ-ML-004
    #[test]
    fn test_zero_velocity_suppressed_when_next_has_load() {
        let cur = act(ActivityKind::InHyperspace, 0);
        let next = act(ActivityKind::InHyperspace, activity_flags::CHECK_LOAD);
        assert!(!should_zero_velocity(cur, next));
    }

    // -----------------------------------------------------------------------
    //  resolve_load_activity
    // -----------------------------------------------------------------------

    /// @plan PLAN-20260707-MAINLOOP.P05
    /// @requirement REQ-ML-004
    #[test]
    fn test_resolve_load_replaces_with_next() {
        let cur = act(ActivityKind::InHyperspace, activity_flags::CHECK_LOAD);
        let next = act(ActivityKind::InInterplanetary, 0);
        assert_eq!(resolve_load_activity(cur, next), next);
    }

    /// @plan PLAN-20260707-MAINLOOP.P05
    /// @requirement REQ-ML-004
    #[test]
    fn test_resolve_load_returns_current_when_no_load() {
        let cur = act(ActivityKind::InHyperspace, 0);
        let next = act(ActivityKind::InInterplanetary, 0);
        assert_eq!(resolve_load_activity(cur, next), cur);
    }

    // -----------------------------------------------------------------------
    //  evaluate
    // -----------------------------------------------------------------------

    /// @plan PLAN-20260707-MAINLOOP.P05
    /// @requirement REQ-ML-004
    #[test]
    fn test_evaluate_bomb_no_starbase() {
        let state = GameStateInfo {
            chmmr_bomb_state: 2,
            starbase_available: 0,
            ..gs()
        };
        // Even without START_ENCOUNTER, bomb==2 enters the encounter block.
        let activity = act(ActivityKind::InHyperspace, 0);
        assert_eq!(
            evaluate(activity, &state),
            ActivityDecision::InstallBombAtEarth
        );
    }

    /// @plan PLAN-20260707-MAINLOOP.P05
    /// @requirement REQ-ML-004
    #[test]
    fn test_evaluate_bomb_with_starbase_visits_starbase() {
        let state = GameStateInfo {
            chmmr_bomb_state: 2,
            starbase_available: 1,
            ..gs()
        };
        let activity = act(ActivityKind::InHyperspace, 0);
        assert_eq!(
            evaluate(activity, &state),
            ActivityDecision::VisitStarBase
        );
    }

    /// @plan PLAN-20260707-MAINLOOP.P05
    /// @requirement REQ-ML-004
    #[test]
    fn test_evaluate_global_flags_all_set_visits_starbase() {
        let state = GameStateInfo {
            global_flags_and_data: 0xFF,
            ..gs()
        };
        let activity = act(ActivityKind::InHyperspace, activity_flags::START_ENCOUNTER);
        assert_eq!(
            evaluate(activity, &state),
            ActivityDecision::VisitStarBase
        );
    }

    /// @plan PLAN-20260707-MAINLOOP.P05
    /// @requirement REQ-ML-004
    #[test]
    fn test_evaluate_encounter_race_communication() {
        let activity = act(ActivityKind::InHyperspace, activity_flags::START_ENCOUNTER);
        assert_eq!(
            evaluate(activity, &gs()),
            ActivityDecision::RaceCommunication
        );
    }

    /// @plan PLAN-20260707-MAINLOOP.P05
    /// @requirement REQ-ML-004
    #[test]
    fn test_evaluate_interplanetary() {
        let activity = act(ActivityKind::InHyperspace, activity_flags::START_INTERPLANETARY);
        assert_eq!(
            evaluate(activity, &gs()),
            ActivityDecision::ExploreSolarSystem
        );
    }

    /// @plan PLAN-20260707-MAINLOOP.P05
    /// @requirement REQ-ML-004
    #[test]
    fn test_evaluate_default_battle() {
        let activity = act(ActivityKind::InHyperspace, 0);
        assert_eq!(evaluate(activity, &gs()), ActivityDecision::Battle);
    }

    /// @plan PLAN-20260707-MAINLOOP.P05
    /// @requirement REQ-ML-004
    #[test]
    fn test_evaluate_bomb_takes_priority_over_starbase_flags() {
        let state = GameStateInfo {
            chmmr_bomb_state: 2,
            starbase_available: 0,
            global_flags_and_data: 0xFF,
            ..gs()
        };
        let activity = act(ActivityKind::InHyperspace, 0);
        // bomb==2 + !starbase → InstallBomb, even though global flags are all set.
        assert_eq!(
            evaluate(activity, &state),
            ActivityDecision::InstallBombAtEarth
        );
    }

    // -----------------------------------------------------------------------
    //  pre_dispatch_mutate
    // -----------------------------------------------------------------------

    /// @plan PLAN-20260707-MAINLOOP.P05
    /// @requirement REQ-ML-004
    #[test]
    fn test_pre_dispatch_sets_start_encounter_for_visit_starbase() {
        let activity = act(ActivityKind::InHyperspace, 0);
        let mutated = pre_dispatch_mutate(activity, ActivityDecision::VisitStarBase);
        assert!(mutated.has_flag(activity_flags::START_ENCOUNTER));
    }

    /// @plan PLAN-20260707-MAINLOOP.P05
    /// @requirement REQ-ML-004
    #[test]
    fn test_pre_dispatch_sets_start_encounter_for_race_communication() {
        let activity = act(ActivityKind::InHyperspace, 0);
        let mutated = pre_dispatch_mutate(activity, ActivityDecision::RaceCommunication);
        assert!(mutated.has_flag(activity_flags::START_ENCOUNTER));
    }

    /// @plan PLAN-20260707-MAINLOOP.P05
    /// @requirement REQ-ML-004
    #[test]
    fn test_pre_dispatch_sets_interplanetary_kind_for_explore() {
        let activity = act(ActivityKind::InHyperspace, activity_flags::START_ENCOUNTER);
        let mutated = pre_dispatch_mutate(activity, ActivityDecision::ExploreSolarSystem);
        assert_eq!(mutated.kind(), ActivityKind::InInterplanetary);
        assert_eq!(mutated.flags(), 0);
    }

    /// @plan PLAN-20260707-MAINLOOP.P05
    /// @requirement REQ-ML-004
    #[test]
    fn test_pre_dispatch_sets_hyperspace_kind_for_battle() {
        let activity = act(ActivityKind::InInterplanetary, activity_flags::START_INTERPLANETARY);
        let mutated = pre_dispatch_mutate(activity, ActivityDecision::Battle);
        assert_eq!(mutated.kind(), ActivityKind::InHyperspace);
        assert_eq!(mutated.flags(), 0);
    }

    // -----------------------------------------------------------------------
    //  post_encounter_clear
    // -----------------------------------------------------------------------

    /// @plan PLAN-20260707-MAINLOOP.P05
    /// @requirement REQ-ML-004
    #[test]
    fn test_post_clear_removes_start_encounter() {
        let activity = act(ActivityKind::InHyperspace, activity_flags::START_ENCOUNTER);
        let result = post_encounter_clear(activity);
        assert!(!result.has_flag(activity_flags::START_ENCOUNTER));
    }

    /// @plan PLAN-20260707-MAINLOOP.P05
    /// @requirement REQ-ML-004
    #[test]
    fn test_post_clear_sets_interplanetary_flag_when_kind_matches() {
        // After clearing START_ENCOUNTER, if kind is IN_INTERPLANETARY,
        // START_INTERPLANETARY must be set.
        let activity = act(ActivityKind::InInterplanetary, activity_flags::START_ENCOUNTER);
        let result = post_encounter_clear(activity);
        assert!(!result.has_flag(activity_flags::START_ENCOUNTER));
        assert!(result.has_flag(activity_flags::START_INTERPLANETARY));
    }

    /// @plan PLAN-20260707-MAINLOOP.P05
    /// @requirement REQ-ML-004
    #[test]
    fn test_post_clear_preserves_activity_on_check_abort() {
        let activity = act(
            ActivityKind::InHyperspace,
            activity_flags::START_ENCOUNTER | activity_flags::CHECK_ABORT,
        );
        let result = post_encounter_clear(activity);
        // CHECK_ABORT set → no mutation
        assert_eq!(result, activity);
    }

    /// @plan PLAN-20260707-MAINLOOP.P05
    /// @requirement REQ-ML-004
    #[test]
    fn test_post_clear_preserves_activity_on_check_load() {
        let activity = act(
            ActivityKind::InHyperspace,
            activity_flags::START_ENCOUNTER | activity_flags::CHECK_LOAD,
        );
        let result = post_encounter_clear(activity);
        // CHECK_LOAD set → no mutation
        assert_eq!(result, activity);
    }

    // -----------------------------------------------------------------------
    //  check_break
    // -----------------------------------------------------------------------

    /// @plan PLAN-20260707-MAINLOOP.P05
    /// @requirement REQ-ML-004
    #[test]
    fn test_break_continue_normal() {
        let activity = act(ActivityKind::InHyperspace, 0);
        assert_eq!(check_break(activity, &gs()), BreakAction::Continue);
    }

    /// @plan PLAN-20260707-MAINLOOP.P05
    /// @requirement REQ-ML-004
    #[test]
    fn test_break_won_last_battle() {
        let activity = act(ActivityKind::WonLastBattle, 0);
        assert_eq!(check_break(activity, &gs()), BreakAction::JustBreak);
    }

    /// @plan PLAN-20260707-MAINLOOP.P05
    /// @requirement REQ-ML-004
    #[test]
    fn test_break_dead_crew() {
        let state = GameStateInfo {
            crew_enlisted: 0xFFFF,
            ..gs()
        };
        let activity = act(ActivityKind::InHyperspace, 0);
        assert_eq!(check_break(activity, &state), BreakAction::JustBreak);
    }

    /// @plan PLAN-20260707-MAINLOOP.P05
    /// @requirement REQ-ML-004
    #[test]
    fn test_break_won_kohr_ah_black_urq() {
        let state = GameStateInfo {
            kohr_ah_killed_all: 1,
            ..gs()
        };
        let activity = act(ActivityKind::WonLastBattle, 0);
        assert_eq!(
            check_break(activity, &state),
            BreakAction::InitBlackUrqCommunication
        );
    }

    /// @plan PLAN-20260707-MAINLOOP.P05
    /// @requirement REQ-ML-004
    #[test]
    fn test_break_won_check_restart_clears_restart() {
        let activity = act(ActivityKind::WonLastBattle, activity_flags::CHECK_RESTART);
        assert_eq!(check_break(activity, &gs()), BreakAction::ClearRestart);
    }

    /// @plan PLAN-20260707-MAINLOOP.P05
    /// @requirement REQ-ML-004
    #[test]
    fn test_break_skipped_on_check_abort() {
        // Won but CHECK_ABORT set → break condition NOT evaluated.
        let activity = act(ActivityKind::WonLastBattle, activity_flags::CHECK_ABORT);
        assert_eq!(check_break(activity, &gs()), BreakAction::Continue);
    }

    /// @plan PLAN-20260707-MAINLOOP.P05
    /// @requirement REQ-ML-004
    #[test]
    fn test_break_skipped_on_check_load() {
        // Dead but CHECK_LOAD set → break condition NOT evaluated.
        let state = GameStateInfo {
            crew_enlisted: 0xFFFF,
            ..gs()
        };
        let activity = act(ActivityKind::InHyperspace, activity_flags::CHECK_LOAD);
        assert_eq!(check_break(activity, &state), BreakAction::Continue);
    }

    // -----------------------------------------------------------------------
    //  should_continue
    // -----------------------------------------------------------------------

    /// @plan PLAN-20260707-MAINLOOP.P05
    /// @requirement REQ-ML-004
    #[test]
    fn test_should_continue_no_abort() {
        let activity = act(ActivityKind::InHyperspace, 0);
        assert!(should_continue(activity));
    }

    /// @plan PLAN-20260707-MAINLOOP.P05
    /// @requirement REQ-ML-004
    #[test]
    fn test_should_continue_stops_on_abort() {
        let activity = act(ActivityKind::InHyperspace, activity_flags::CHECK_ABORT);
        assert!(!should_continue(activity));
    }
}