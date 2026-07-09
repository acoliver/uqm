//! Restart menu pure navigation logic.
//!
//! This module ports the C restart menu's pure decision functions:
//! `navigate_up`, `navigate_down`, `apply_selection`, and
//! `check_timeout`. Every function here is side-effect free, FFI free,
//! and free of global state, so they are exhaustively unit-testable.
//!
//! # C Reference
//!
//! The navigation wrap-around matches the index arithmetic in
//! `sc2/src/uqm/restart.c:189-202`. The selection mapping matches the
//! switch-case at `restart.c:144-177`. The timeout comparison matches
//! `restart.c:236`.
//!
//! @plan PLAN-20260707-RESTARTMENU.P03
//! @requirement REQ-RM-001

use super::types::{RestartMenuItem, SelectionResult};

/// Navigate menu selection up (decrement with wrap-around).
///
/// Matches `restart.c:189-195`.
///
/// @plan PLAN-20260707-RESTARTMENU.P03
/// @requirement REQ-RM-001
pub fn navigate_up(current: RestartMenuItem) -> RestartMenuItem {
    match current {
        RestartMenuItem::NewGame => RestartMenuItem::Quit,
        RestartMenuItem::LoadGame => RestartMenuItem::NewGame,
        RestartMenuItem::SuperMelee => RestartMenuItem::LoadGame,
        RestartMenuItem::Setup => RestartMenuItem::SuperMelee,
        RestartMenuItem::Quit => RestartMenuItem::Setup,
    }
}

/// Navigate menu selection down (increment with wrap-around).
///
/// Matches `restart.c:196-202`.
///
/// @plan PLAN-20260707-RESTARTMENU.P03
/// @requirement REQ-RM-001
pub fn navigate_down(current: RestartMenuItem) -> RestartMenuItem {
    match current {
        RestartMenuItem::NewGame => RestartMenuItem::LoadGame,
        RestartMenuItem::LoadGame => RestartMenuItem::SuperMelee,
        RestartMenuItem::SuperMelee => RestartMenuItem::Setup,
        RestartMenuItem::Setup => RestartMenuItem::Quit,
        RestartMenuItem::Quit => RestartMenuItem::NewGame,
    }
}

/// Map a menu selection to its [`SelectionResult`].
///
/// Matches `restart.c:144-177`.
///
/// @plan PLAN-20260707-RESTARTMENU.P03
/// @requirement REQ-RM-001
pub fn apply_selection(item: RestartMenuItem) -> SelectionResult {
    match item {
        RestartMenuItem::NewGame => SelectionResult::StartGame { new_game: true },
        RestartMenuItem::LoadGame => SelectionResult::StartGame { new_game: false },
        RestartMenuItem::SuperMelee => SelectionResult::SuperMelee,
        RestartMenuItem::Setup => SelectionResult::StayInMenu,
        RestartMenuItem::Quit => SelectionResult::Quit,
    }
}

/// Check if the inactivity timeout has been exceeded.
///
/// Uses wrapping subtraction to match the C `TimeCount` unsigned
/// arithmetic; `now.wrapping_sub(last_input)` produces the elapsed
/// tick count even across counter wrap-around. Matches `restart.c:236`.
///
/// @plan PLAN-20260707-RESTARTMENU.P03
/// @requirement REQ-RM-001
pub fn check_timeout(now: u32, last_input: u32, timeout: u32) -> bool {
    now.wrapping_sub(last_input) > timeout
}

// ===========================================================================
//  Unit tests — Tier 1 (pure Rust, no C linkage)
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mainloop::restart_menu::types::{RestartMenuItem as M, SelectionResult as R};

    // ---- navigate_up -----------------------------------------------------

    /// @plan PLAN-20260707-RESTARTMENU.P03
    /// @requirement REQ-RM-001
    #[test]
    fn navigate_up_from_new_game_wraps_to_quit() {
        assert_eq!(navigate_up(M::NewGame), M::Quit);
    }

    /// @plan PLAN-20260707-RESTARTMENU.P03
    /// @requirement REQ-RM-001
    #[test]
    fn navigate_up_from_load_game_goes_to_new_game() {
        assert_eq!(navigate_up(M::LoadGame), M::NewGame);
    }

    /// @plan PLAN-20260707-RESTARTMENU.P03
    /// @requirement REQ-RM-001
    #[test]
    fn navigate_up_from_super_melee_goes_to_load_game() {
        assert_eq!(navigate_up(M::SuperMelee), M::LoadGame);
    }

    /// @plan PLAN-20260707-RESTARTMENU.P03
    /// @requirement REQ-RM-001
    #[test]
    fn navigate_up_from_setup_goes_to_super_melee() {
        assert_eq!(navigate_up(M::Setup), M::SuperMelee);
    }

    /// @plan PLAN-20260707-RESTARTMENU.P03
    /// @requirement REQ-RM-001
    #[test]
    fn navigate_up_from_quit_goes_to_setup() {
        assert_eq!(navigate_up(M::Quit), M::Setup);
    }

    // ---- navigate_down ---------------------------------------------------

    /// @plan PLAN-20260707-RESTARTMENU.P03
    /// @requirement REQ-RM-001
    #[test]
    fn navigate_down_from_quit_wraps_to_new_game() {
        assert_eq!(navigate_down(M::Quit), M::NewGame);
    }

    /// @plan PLAN-20260707-RESTARTMENU.P03
    /// @requirement REQ-RM-001
    #[test]
    fn navigate_down_from_new_game_goes_to_load_game() {
        assert_eq!(navigate_down(M::NewGame), M::LoadGame);
    }

    /// @plan PLAN-20260707-RESTARTMENU.P03
    /// @requirement REQ-RM-001
    #[test]
    fn navigate_down_from_load_game_goes_to_super_melee() {
        assert_eq!(navigate_down(M::LoadGame), M::SuperMelee);
    }

    /// @plan PLAN-20260707-RESTARTMENU.P03
    /// @requirement REQ-RM-001
    #[test]
    fn navigate_down_from_super_melee_goes_to_setup() {
        assert_eq!(navigate_down(M::SuperMelee), M::Setup);
    }

    /// @plan PLAN-20260707-RESTARTMENU.P03
    /// @requirement REQ-RM-001
    #[test]
    fn navigate_down_from_setup_goes_to_quit() {
        assert_eq!(navigate_down(M::Setup), M::Quit);
    }

    // ---- apply_selection -------------------------------------------------

    /// @plan PLAN-20260707-RESTARTMENU.P03
    /// @requirement REQ-RM-001
    #[test]
    fn apply_selection_new_game_starts_new_game() {
        assert_eq!(
            apply_selection(M::NewGame),
            R::StartGame { new_game: true }
        );
    }

    /// @plan PLAN-20260707-RESTARTMENU.P03
    /// @requirement REQ-RM-001
    #[test]
    fn apply_selection_load_game_starts_loaded_game() {
        assert_eq!(
            apply_selection(M::LoadGame),
            R::StartGame { new_game: false }
        );
    }

    /// @plan PLAN-20260707-RESTARTMENU.P03
    /// @requirement REQ-RM-001
    #[test]
    fn apply_selection_super_melee_enters_melee() {
        assert_eq!(apply_selection(M::SuperMelee), R::SuperMelee);
    }

    /// @plan PLAN-20260707-RESTARTMENU.P03
    /// @requirement REQ-RM-001
    #[test]
    fn apply_selection_setup_stays_in_menu() {
        assert_eq!(apply_selection(M::Setup), R::StayInMenu);
    }

    /// @plan PLAN-20260707-RESTARTMENU.P03
    /// @requirement REQ-RM-001
    #[test]
    fn apply_selection_quit_quits() {
        assert_eq!(apply_selection(M::Quit), R::Quit);
    }

    // ---- check_timeout (wrapping arithmetic) -----------------------------

    /// @plan PLAN-20260707-RESTARTMENU.P03
    /// @requirement REQ-RM-001
    #[test]
    fn check_timeout_clearly_over_returns_true() {
        // 100 - 0 = 100 > 50
        assert!(check_timeout(100, 0, 50));
    }

    /// @plan PLAN-20260707-RESTARTMENU.P03
    /// @requirement REQ-RM-001
    #[test]
    fn check_timeout_clearly_under_returns_false() {
        // 30 - 0 = 30 < 50
        assert!(!check_timeout(30, 0, 50));
    }

    /// @plan PLAN-20260707-RESTARTMENU.P03
    /// @requirement REQ-RM-001
    #[test]
    fn check_timeout_at_exact_boundary_returns_false() {
        // Exactly 50 == 50, NOT > 50
        assert!(!check_timeout(50, 0, 50));
    }

    /// @plan PLAN-20260707-RESTARTMENU.P03
    /// @requirement REQ-RM-001
    #[test]
    fn check_timeout_just_over_boundary_returns_true() {
        // 51 > 50
        assert!(check_timeout(51, 0, 50));
    }

    /// @plan PLAN-20260707-RESTARTMENU.P03
    /// @requirement REQ-RM-001
    #[test]
    fn check_timeout_wraparound_just_after_counter_returns_true() {
        // 100.wrapping_sub(0xFFFFFFF0) = 116, 116 > 50 → true
        assert!(check_timeout(100, 0xFFFFFFF0, 50));
    }

    /// @plan PLAN-20260707-RESTARTMENU.P03
    /// @requirement REQ-RM-001
    #[test]
    fn check_timeout_wraparound_small_gap_returns_false() {
        // 5.wrapping_sub(0xFFFFFFF0) = 21, 21 < 50 → false
        assert!(!check_timeout(5, 0xFFFFFFF0, 50));
    }

    // ---- purity / properties --------------------------------------------

    /// @plan PLAN-20260707-RESTARTMENU.P03
    /// @requirement REQ-RM-001
    #[test]
    fn navigate_up_then_down_is_identity() {
        for item in [
            M::NewGame,
            M::LoadGame,
            M::SuperMelee,
            M::Setup,
            M::Quit,
        ] {
            assert_eq!(navigate_down(navigate_up(item)), item);
            assert_eq!(navigate_up(navigate_down(item)), item);
        }
    }

    /// @plan PLAN-20260707-RESTARTMENU.P03
    /// @requirement REQ-RM-001
    #[test]
    fn navigate_up_is_involution_after_five_steps() {
        // Five up-steps return to origin.
        let mut item = M::NewGame;
        for _ in 0..5 {
            item = navigate_up(item);
        }
        assert_eq!(item, M::NewGame);
    }

    /// @plan PLAN-20260707-RESTARTMENU.P03
    /// @requirement REQ-RM-001
    #[test]
    fn navigate_down_visits_all_items_in_order() {
        let mut item = M::NewGame;
        assert_eq!(navigate_down(item), M::LoadGame);
        item = M::LoadGame;
        assert_eq!(navigate_down(item), M::SuperMelee);
        item = M::SuperMelee;
        assert_eq!(navigate_down(item), M::Setup);
        item = M::Setup;
        assert_eq!(navigate_down(item), M::Quit);
        item = M::Quit;
        assert_eq!(navigate_down(item), M::NewGame);
    }

    /// @plan PLAN-20260707-RESTARTMENU.P03
    /// @requirement REQ-RM-001
    #[test]
    fn check_timeout_now_equals_last_input_returns_false() {
        // 0 - 0 = 0, not > 0
        assert!(!check_timeout(42, 42, 0));
    }

    /// @plan PLAN-20260707-RESTARTMENU.P03
    /// @requirement REQ-RM-001
    #[test]
    fn check_timeout_zero_timeout_one_tick_returns_true() {
        // now = 1, last = 0, timeout = 0 → 1 > 0 → true
        assert!(check_timeout(1, 0, 0));
    }
}
