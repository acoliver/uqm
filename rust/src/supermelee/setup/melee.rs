// SuperMelee Entry Point — setup/menu orchestration
// @plan PLAN-20260314-SUPERMELEE.P07
// @requirement setup menu flow, battle handoff

use crate::supermelee::error::SuperMeleeError;
use crate::supermelee::setup::build_pick::{PickResult, ShipPicker};
use crate::supermelee::setup::config::{load_melee_config, save_melee_config, ConfigLoadResult};
use crate::supermelee::setup::persistence::builtin_teams;
use crate::supermelee::setup::team::{MeleeSetup, MeleeTeam};
use crate::supermelee::types::{FleetShipIndex, MeleeShip, PlayerControl, NUM_SIDES};
use std::path::Path;

// ---------------------------------------------------------------------------
// Melee menu option (matches C MELEE_OPTIONS enum)
// ---------------------------------------------------------------------------

/// Active menu option on the melee screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeleeOption {
    /// Currently in the fleet editing area.
    EditFleet,
    /// Hovering over the "Load" button.
    Load,
    /// Hovering over the "Save" button.
    Save,
    /// Hovering over the "Start" button.
    Start,
    /// Hovering over the control-mode toggle.
    Controls,
}

// ---------------------------------------------------------------------------
// MeleeState — runtime state for the SuperMelee menu
// ---------------------------------------------------------------------------

/// Runtime state for the SuperMelee setup screen.
///
/// This is the Rust equivalent of the C `MELEE_STATE` struct.
/// It owns the `MeleeSetup` and transient UI state but does NOT own
/// graphics resources (those remain in C or future Rust graphics).
#[derive(Debug)]
pub struct MeleeState {
    /// The editable two-sided setup (teams + fleet values + controls).
    pub setup: MeleeSetup,
    /// Whether the state has been fully initialized.
    pub initialized: bool,
    /// Currently active side (0 or 1).
    pub side: usize,
    /// Current cursor row within the fleet grid.
    pub row: usize,
    /// Current cursor column within the fleet grid.
    pub col: usize,
    /// Active menu option.
    pub option: MeleeOption,
    /// Current ship in the picker (used during fleet edit).
    pub current_ship: MeleeShip,
    /// Whether the melee session has started (at least one battle).
    pub melee_started: bool,
    /// Whether buildpick confirmation is active for each side.
    pub build_pick_confirmed: [bool; NUM_SIDES],
}

impl Default for MeleeState {
    fn default() -> Self {
        Self::new()
    }
}

impl MeleeState {
    /// Creates a new zeroed melee state (matches C `memset(0)`).
    pub fn new() -> Self {
        Self {
            setup: MeleeSetup::new(),
            initialized: false,
            side: 0,
            row: 0,
            col: 0,
            option: MeleeOption::EditFleet,
            current_ship: MeleeShip::MeleeNone,
            melee_started: false,
            build_pick_confirmed: [false; NUM_SIDES],
        }
    }

    // -----------------------------------------------------------------------
    // Initialization
    // -----------------------------------------------------------------------

    /// Initializes the melee state from persisted config or built-in fallback.
    ///
    /// Matches C `Melee()` entry: tries `LoadMeleeConfig`, falls back to
    /// built-in teams 0 and 1 with HUMAN/COMPUTER control.
    pub fn init(&mut self, config_dir: &Path) -> Result<(), SuperMeleeError> {
        let result = load_melee_config(config_dir, &mut self.setup);

        match result {
            ConfigLoadResult::Ok => {}
            ConfigLoadResult::Missing | ConfigLoadResult::Invalid(_) => {
                self.apply_builtin_fallback()?;
            }
        }

        self.side = 0;
        self.current_ship = MeleeShip::MeleeNone;
        self.initialized = true;
        Ok(())
    }

    /// Applies built-in fallback teams (matches C fallback in `Melee()`).
    fn apply_builtin_fallback(&mut self) -> Result<(), SuperMeleeError> {
        let builtins = builtin_teams();
        if builtins.len() >= 2 {
            self.setup.replace_team(0, &builtins[0])?;
            self.setup.replace_team(1, &builtins[1])?;
        }
        // STANDARD_RATING = 1 << 4 = 16
        const STANDARD_RATING: u8 = 1 << 4;
        self.setup.player_control[0] =
            PlayerControl(PlayerControl::HUMAN_CONTROL.0 | STANDARD_RATING);
        self.setup.player_control[1] =
            PlayerControl(PlayerControl::COMPUTER_CONTROL.0 | STANDARD_RATING);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Teardown
    // -----------------------------------------------------------------------

    /// Persists setup state and cleans up (matches C `Melee()` exit).
    pub fn teardown(&self, config_dir: &Path) -> Result<(), SuperMeleeError> {
        save_melee_config(config_dir, &self.setup)?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Start validation
    // -----------------------------------------------------------------------

    /// Returns `true` if a battle can start (both sides playable).
    pub fn can_start_battle(&self) -> bool {
        self.setup.is_playable(0) && self.setup.is_playable(1)
    }

    /// Attempts to start a battle. Returns error if either side is not playable.
    pub fn start_battle(&mut self) -> Result<(), SuperMeleeError> {
        if !self.can_start_battle() {
            return Err(SuperMeleeError::BattleHandoffError(
                "Cannot start: one or both sides have no ships".to_string(),
            ));
        }
        self.melee_started = true;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Post-battle restoration
    // -----------------------------------------------------------------------

    /// Restores the menu state after a battle completes.
    ///
    /// Resets transient state while preserving the setup (teams/controls).
    pub fn restore_after_battle(&mut self) {
        self.option = MeleeOption::EditFleet;
        self.row = 0;
        self.col = 0;
        self.current_ship = MeleeShip::MeleeNone;
        self.build_pick_confirmed = [false; NUM_SIDES];
    }

    // -----------------------------------------------------------------------
    // Fleet edit helpers
    // -----------------------------------------------------------------------

    /// Applies a confirmed picker result to the active slot.
    pub fn apply_pick(
        &mut self,
        side: usize,
        slot: FleetShipIndex,
        result: &PickResult,
    ) -> Result<(), SuperMeleeError> {
        match result {
            PickResult::Confirmed(ship) => {
                self.setup.set_ship(side, slot, *ship)?;
            }
            PickResult::Cancelled => {
                // No change
            }
        }
        Ok(())
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::supermelee::setup::config::save_melee_config;
    use crate::supermelee::types::MeleeShip;

    #[test]
    fn melee_entry_initializes_runtime_state() {
        let dir = tempfile::tempdir().unwrap();
        let mut state = MeleeState::new();
        state.init(dir.path()).unwrap();
        assert!(state.initialized);
        assert_eq!(state.side, 0);
        assert_eq!(state.current_ship, MeleeShip::MeleeNone);
    }

    #[test]
    fn invalid_or_missing_config_uses_builtin_fallback() {
        let dir = tempfile::tempdir().unwrap();
        let mut state = MeleeState::new();
        state.init(dir.path()).unwrap(); // no melee.cfg → fallback

        // Should have teams from built-in catalog
        assert!(state.setup.is_playable(0));
        assert!(state.setup.is_playable(1));
        // Side 0 should be HUMAN, side 1 COMPUTER
        assert!(state.setup.player_control[0].contains(PlayerControl::HUMAN_CONTROL));
        assert!(state.setup.player_control[1].contains(PlayerControl::COMPUTER_CONTROL));
    }

    #[test]
    fn valid_config_restores_state() {
        let dir = tempfile::tempdir().unwrap();

        // Save a config
        let mut setup = MeleeSetup::new();
        setup.set_ship(0, 0, MeleeShip::Urquan).unwrap();
        setup.set_ship(1, 0, MeleeShip::Pkunk).unwrap();
        setup.set_team_name(0, "Saved A").unwrap();
        save_melee_config(dir.path(), &setup).unwrap();

        // Init should restore it
        let mut state = MeleeState::new();
        state.init(dir.path()).unwrap();
        assert_eq!(state.setup.teams[0].ships[0], MeleeShip::Urquan);
        assert_eq!(state.setup.teams[1].ships[0], MeleeShip::Pkunk);
        assert_eq!(state.setup.teams[0].name_str(), "Saved A");
    }

    #[test]
    fn match_start_blocked_when_either_side_unplayable() {
        let mut state = MeleeState::new();
        state.initialized = true;
        // Both sides empty
        assert!(!state.can_start_battle());
        assert!(state.start_battle().is_err());
    }

    #[test]
    fn match_start_allowed_when_both_sides_playable() {
        let mut state = MeleeState::new();
        state.initialized = true;
        state.setup.set_ship(0, 0, MeleeShip::Chmmr).unwrap();
        state.setup.set_ship(1, 0, MeleeShip::Shofixti).unwrap();
        assert!(state.can_start_battle());
        assert!(state.start_battle().is_ok());
        assert!(state.melee_started);
    }

    #[test]
    fn battle_return_restores_valid_post_battle_state() {
        let mut state = MeleeState::new();
        state.initialized = true;
        state.setup.set_ship(0, 0, MeleeShip::Chmmr).unwrap();
        state.setup.set_ship(1, 0, MeleeShip::Shofixti).unwrap();
        state.start_battle().unwrap();

        state.restore_after_battle();
        assert_eq!(state.option, MeleeOption::EditFleet);
        assert_eq!(state.current_ship, MeleeShip::MeleeNone);
        // Teams should still be present
        assert_eq!(state.setup.teams[0].ships[0], MeleeShip::Chmmr);
    }

    #[test]
    fn exit_path_persists_state() {
        let dir = tempfile::tempdir().unwrap();
        let mut state = MeleeState::new();
        state.init(dir.path()).unwrap();
        state.setup.set_ship(0, 0, MeleeShip::Vux).unwrap();
        state.teardown(dir.path()).unwrap();

        // Reload and verify
        let mut state2 = MeleeState::new();
        state2.init(dir.path()).unwrap();
        assert_eq!(state2.setup.teams[0].ships[0], MeleeShip::Vux);
    }

    #[test]
    fn picker_confirm_applies_to_slot() {
        let mut state = MeleeState::new();
        state.initialized = true;
        let result = PickResult::Confirmed(MeleeShip::Orz);
        state.apply_pick(0, 3, &result).unwrap();
        assert_eq!(state.setup.teams[0].ships[3], MeleeShip::Orz);
    }

    #[test]
    fn picker_cancel_leaves_state_unchanged() {
        let mut state = MeleeState::new();
        state.initialized = true;
        state.setup.set_ship(0, 3, MeleeShip::Druuge).unwrap();

        let result = PickResult::Cancelled;
        state.apply_pick(0, 3, &result).unwrap();
        assert_eq!(state.setup.teams[0].ships[3], MeleeShip::Druuge);
    }
}
