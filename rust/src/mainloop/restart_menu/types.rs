//! Restart menu domain type definitions.
//!
//! Defines the Rust-side representations of the C restart menu's
//! item list, selection outcomes, and per-frame input state.
//!
//! # C Reference
//!
//! The five menu items match the C `RESTART_MENU` array in
//! `sc2/src/uqm/restart.c:45-52`:
//!
//! ```c
//! RESTART_MENU[] = {
//!     NewGame,      // index 0
//!     LoadGame,     // index 1
//!     SuperMelee,   // index 2
//!     Setup,        // index 3
//!     Quit,         // index 4
//! };
//! ```
//!
//! @plan PLAN-20260707-RESTARTMENU.P02
//! @requirement REQ-RM-001

// ===========================================================================
//  RestartMenuItem
// ===========================================================================

/// The five items on the UQM main menu, matching `restart.c:45-52`.
///
/// The `#[repr(u8)]` ensures the discriminant values are stable and
/// match the C-side array indices, allowing safe conversion to/from
/// the raw byte used by the C drawing routines.
///
/// @plan PLAN-20260707-RESTARTMENU.P02
/// @requirement REQ-RM-001
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RestartMenuItem {
    /// Start a new game from scratch.
    NewGame = 0,
    /// Load a previously saved game.
    LoadGame = 1,
    /// Enter Super Melee (ship-vs-ship combat).
    SuperMelee = 2,
    /// Open the settings/setup menu.
    Setup = 3,
    /// Quit the game.
    Quit = 4,
}

impl RestartMenuItem {
    /// The total number of menu items.
    ///
    /// @plan PLAN-20260707-RESTARTMENU.P02
    /// @requirement REQ-RM-001
    pub const COUNT: u8 = 5;

    /// Safely convert a raw `u8` into a [`RestartMenuItem`].
    ///
    /// Returns `None` for values outside the valid range (0–4).
    ///
    /// @plan PLAN-20260707-RESTARTMENU.P02
    /// @requirement REQ-RM-001
    #[inline]
    #[must_use]
    pub const fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::NewGame),
            1 => Some(Self::LoadGame),
            2 => Some(Self::SuperMelee),
            3 => Some(Self::Setup),
            4 => Some(Self::Quit),
            _ => None,
        }
    }

    /// Explicit conversion back to `u8`.
    ///
    /// This is the inverse of [`RestartMenuItem::from_u8`].
    ///
    /// @plan PLAN-20260707-RESTARTMENU.P02
    /// @requirement REQ-RM-001
    #[inline]
    #[must_use]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

// ===========================================================================
//  SelectionResult
// ===========================================================================

/// The outcome of processing a menu selection.
///
/// Produced by `apply_selection` (P03) and consumed by the frame logic
/// (P06) to decide what side effects to perform.
///
/// @plan PLAN-20260707-RESTARTMENU.P02
/// @requirement REQ-RM-001
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SelectionResult {
    /// Player chose to start or load a game — exit the menu and proceed.
    ///
    /// `new_game == true` means a fresh game; `false` means loading a save.
    StartGame { new_game: bool },
    /// Player chose Super Melee — exit the menu and run the Melee module.
    SuperMelee,
    /// Player chose Setup — stay in the menu after setup completes.
    StayInMenu,
    /// Player chose Quit — exit the menu and signal abort.
    Quit,
}

// ===========================================================================
//  MenuInputState
// ===========================================================================

/// Input state snapshot read from C each frame.
///
/// Each field corresponds to a `PulsedInputState` key or mouse button
/// checked in `DoRestart` (`restart.c:183-230`). All fields are `bool`
/// because they represent edge-triggered (pulsed) input, not held state.
///
/// `Default` gives an all-`false` snapshot (no input), which is the
/// correct initial state before any player interaction.
///
/// @plan PLAN-20260707-RESTARTMENU.P02
/// @requirement REQ-RM-001
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct MenuInputState {
    /// Select/confirm key (`KEY_MENU_SELECT`).
    pub select: bool,
    /// Navigate up (`KEY_MENU_UP`).
    pub up: bool,
    /// Navigate down (`KEY_MENU_DOWN`).
    pub down: bool,
    /// Navigate left (`KEY_MENU_LEFT`).
    pub left: bool,
    /// Navigate right (`KEY_MENU_RIGHT`).
    pub right: bool,
    /// Mouse button held down (triggers a "not supported" popup).
    pub mouse_down: bool,
}

// ===========================================================================
//  Unit tests — Tier 1 (pure Rust, no C linkage)
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// @plan PLAN-20260707-RESTARTMENU.P02
    /// @requirement REQ-RM-001
    #[test]
    fn test_restart_menu_item_has_five_variants() {
        assert_eq!(RestartMenuItem::NewGame as u8, 0);
        assert_eq!(RestartMenuItem::LoadGame as u8, 1);
        assert_eq!(RestartMenuItem::SuperMelee as u8, 2);
        assert_eq!(RestartMenuItem::Setup as u8, 3);
        assert_eq!(RestartMenuItem::Quit as u8, 4);
    }

    /// @plan PLAN-20260707-RESTARTMENU.P02
    /// @requirement REQ-RM-001
    #[test]
    fn test_restart_menu_item_count() {
        assert_eq!(RestartMenuItem::COUNT, 5);
    }

    /// @plan PLAN-20260707-RESTARTMENU.P02
    /// @requirement REQ-RM-001
    #[test]
    fn test_selection_result_new_game() {
        let result = SelectionResult::StartGame { new_game: true };
        assert_eq!(result, SelectionResult::StartGame { new_game: true });
    }

    /// @plan PLAN-20260707-RESTARTMENU.P02
    /// @requirement REQ-RM-001
    #[test]
    fn test_selection_result_load_game() {
        let result = SelectionResult::StartGame { new_game: false };
        assert_eq!(result, SelectionResult::StartGame { new_game: false });
    }

    /// @plan PLAN-20260707-RESTARTMENU.P02
    /// @requirement REQ-RM-001
    #[test]
    fn test_selection_result_super_melee() {
        assert_eq!(SelectionResult::SuperMelee, SelectionResult::SuperMelee);
    }

    /// @plan PLAN-20260707-RESTARTMENU.P02
    /// @requirement REQ-RM-001
    #[test]
    fn test_selection_result_stay_in_menu() {
        assert_eq!(SelectionResult::StayInMenu, SelectionResult::StayInMenu);
    }

    /// @plan PLAN-20260707-RESTARTMENU.P02
    /// @requirement REQ-RM-001
    #[test]
    fn test_selection_result_quit() {
        assert_eq!(SelectionResult::Quit, SelectionResult::Quit);
    }

    /// @plan PLAN-20260707-RESTARTMENU.P02
    /// @requirement REQ-RM-001
    #[test]
    fn test_menu_input_state_default_all_false() {
        let state = MenuInputState::default();
        assert!(!state.select);
        assert!(!state.up);
        assert!(!state.down);
        assert!(!state.left);
        assert!(!state.right);
        assert!(!state.mouse_down);
    }

    /// @plan PLAN-20260707-RESTARTMENU.P02
    /// @requirement REQ-RM-001
    #[test]
    fn test_restart_menu_item_from_u8_valid() {
        assert_eq!(RestartMenuItem::from_u8(0), Some(RestartMenuItem::NewGame));
        assert_eq!(RestartMenuItem::from_u8(1), Some(RestartMenuItem::LoadGame));
        assert_eq!(
            RestartMenuItem::from_u8(2),
            Some(RestartMenuItem::SuperMelee)
        );
        assert_eq!(RestartMenuItem::from_u8(3), Some(RestartMenuItem::Setup));
        assert_eq!(RestartMenuItem::from_u8(4), Some(RestartMenuItem::Quit));
    }

    /// @plan PLAN-20260707-RESTARTMENU.P02
    /// @requirement REQ-RM-001
    #[test]
    fn test_restart_menu_item_from_u8_out_of_range() {
        assert_eq!(RestartMenuItem::from_u8(5), None);
        assert_eq!(RestartMenuItem::from_u8(255), None);
    }

    /// @plan PLAN-20260707-RESTARTMENU.P02
    /// @requirement REQ-RM-001
    #[test]
    fn test_restart_menu_item_as_u8_roundtrip() {
        let all_variants = [
            RestartMenuItem::NewGame,
            RestartMenuItem::LoadGame,
            RestartMenuItem::SuperMelee,
            RestartMenuItem::Setup,
            RestartMenuItem::Quit,
        ];
        for original in all_variants {
            let raw = original.as_u8();
            assert_eq!(RestartMenuItem::from_u8(raw), Some(original));
        }
    }

    /// @plan PLAN-20260707-RESTARTMENU.P02
    /// @requirement REQ-RM-001
    #[test]
    fn test_menu_input_state_individual_fields() {
        let state = MenuInputState {
            select: true,
            up: false,
            down: true,
            left: false,
            right: false,
            mouse_down: true,
        };
        assert!(state.select);
        assert!(!state.up);
        assert!(state.down);
        assert!(!state.left);
        assert!(!state.right);
        assert!(state.mouse_down);
    }

    /// @plan PLAN-20260707-RESTARTMENU.P02
    /// @requirement REQ-RM-001
    #[test]
    fn test_selection_result_variants_distinct() {
        // Ensure all SelectionResult variants are distinguishable.
        let ng = SelectionResult::StartGame { new_game: true };
        let lg = SelectionResult::StartGame { new_game: false };
        assert_ne!(ng, lg);
        assert_ne!(ng, SelectionResult::SuperMelee);
        assert_ne!(ng, SelectionResult::StayInMenu);
        assert_ne!(ng, SelectionResult::Quit);
        assert_ne!(lg, SelectionResult::SuperMelee);
        assert_ne!(SelectionResult::SuperMelee, SelectionResult::StayInMenu);
        assert_ne!(SelectionResult::StayInMenu, SelectionResult::Quit);
    }
}
