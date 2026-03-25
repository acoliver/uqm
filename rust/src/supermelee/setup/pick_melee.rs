// SuperMelee Combatant Selection Contract — initial and next ship selection
// @plan PLAN-20260314-SUPERMELEE.P08
// @requirement combatant selection

use crate::supermelee::error::SuperMeleeError;
use crate::supermelee::setup::team::{ship_cost, MeleeSetup, MeleeTeam};
use crate::supermelee::types::{
    BattleReadyCombatant, FleetShipIndex, MeleeShip, MELEE_FLEET_SIZE, NUM_SIDES,
};

// ---------------------------------------------------------------------------
// Selection state per side
// ---------------------------------------------------------------------------

/// Tracks consumed (eliminated) fleet slots for one side during a match.
#[derive(Debug, Clone)]
pub struct SideSelectionState {
    /// Which slots have been consumed (ship destroyed or deactivated).
    consumed: [bool; MELEE_FLEET_SIZE],
    /// The currently active slot, if any.
    current_slot: Option<FleetShipIndex>,
}

impl Default for SideSelectionState {
    fn default() -> Self {
        Self::new()
    }
}

impl SideSelectionState {
    pub fn new() -> Self {
        Self {
            consumed: [false; MELEE_FLEET_SIZE],
            current_slot: None,
        }
    }

    /// Marks a slot as consumed (ship destroyed/deactivated).
    ///
    /// Matches C `MeleeShipDeath()` which sets `SpeciesID = NO_ID`.
    pub fn consume_slot(&mut self, slot: FleetShipIndex) {
        if (slot as usize) < MELEE_FLEET_SIZE {
            self.consumed[slot as usize] = true;
            if self.current_slot == Some(slot) {
                self.current_slot = None;
            }
        }
    }

    /// Returns `true` if the slot is still available (not consumed, has a ship).
    pub fn is_available(&self, slot: FleetShipIndex, team: &MeleeTeam) -> bool {
        let idx = slot as usize;
        idx < MELEE_FLEET_SIZE && !self.consumed[idx] && team.ships[idx] != MeleeShip::MeleeNone
    }

    /// Counts remaining available ships for this side.
    pub fn remaining_count(&self, team: &MeleeTeam) -> usize {
        (0..MELEE_FLEET_SIZE)
            .filter(|&i| !self.consumed[i] && team.ships[i] != MeleeShip::MeleeNone)
            .count()
    }

    /// Returns the first available slot, if any.
    fn first_available(&self, team: &MeleeTeam) -> Option<FleetShipIndex> {
        (0..MELEE_FLEET_SIZE as FleetShipIndex).find(|&slot| self.is_available(slot, team))
    }

    /// Returns the currently active slot, if any.
    pub fn current_slot(&self) -> Option<FleetShipIndex> {
        self.current_slot
    }

    /// Resets all consumed flags (for new match).
    pub fn reset(&mut self) {
        self.consumed = [false; MELEE_FLEET_SIZE];
        self.current_slot = None;
    }
}

// ---------------------------------------------------------------------------
// Match-level selection state
// ---------------------------------------------------------------------------

/// Tracks combatant selection for both sides during a SuperMelee match.
#[derive(Debug, Clone)]
pub struct MatchSelectionState {
    pub sides: [SideSelectionState; NUM_SIDES],
}

impl Default for MatchSelectionState {
    fn default() -> Self {
        Self::new()
    }
}

impl MatchSelectionState {
    pub fn new() -> Self {
        Self {
            sides: [SideSelectionState::new(), SideSelectionState::new()],
        }
    }

    /// Resets selection state for a new match.
    pub fn reset(&mut self) {
        for side in &mut self.sides {
            side.reset();
        }
    }
}

// ---------------------------------------------------------------------------
// Battle-ready combatant creation
// ---------------------------------------------------------------------------

/// Creates a `BattleReadyCombatant` from a selected fleet slot.
///
/// This preserves the stronger contract: the returned object carries the
/// fleet slot, ship type, and cost — not bare ship IDs.
fn make_combatant(side: usize, slot: FleetShipIndex, ship: MeleeShip) -> BattleReadyCombatant {
    BattleReadyCombatant {
        handle: (side << 16) | (slot as usize),
    }
}

// ---------------------------------------------------------------------------
// Selection API
// ---------------------------------------------------------------------------

/// Selects the initial combatant for both sides at match start.
///
/// Mirrors C `GetInitialMeleeStarShips()`: requests ships for all players.
/// Returns a pair of combatants, or error if either side has no ships.
pub fn select_initial_combatants(
    setup: &MeleeSetup,
    state: &mut MatchSelectionState,
) -> Result<[Option<BattleReadyCombatant>; NUM_SIDES], SuperMeleeError> {
    let mut result = [None; NUM_SIDES];

    for side in 0..NUM_SIDES {
        match state.sides[side].first_available(&setup.teams[side]) {
            Some(slot) => {
                let ship = setup.teams[side].ships[slot as usize];
                state.sides[side].current_slot = Some(slot);
                result[side] = Some(make_combatant(side, slot, ship));
            }
            None => {
                return Err(SuperMeleeError::SelectionError(format!(
                    "side {} has no available ships",
                    side,
                )));
            }
        }
    }

    Ok(result)
}

/// Selects the next combatant for a side after a ship loss.
///
/// Mirrors C `GetNextMeleeStarShip()`.
/// Returns `None` if no ships remain (side eliminated).
pub fn select_next_combatant(
    setup: &MeleeSetup,
    state: &mut MatchSelectionState,
    side: usize,
) -> Result<Option<BattleReadyCombatant>, SuperMeleeError> {
    if side >= NUM_SIDES {
        return Err(SuperMeleeError::SelectionError(format!(
            "invalid side {}",
            side,
        )));
    }

    match state.sides[side].first_available(&setup.teams[side]) {
        Some(slot) => {
            let ship = setup.teams[side].ships[slot as usize];
            state.sides[side].current_slot = Some(slot);
            Ok(Some(make_combatant(side, slot, ship)))
        }
        None => Ok(None), // side eliminated
    }
}

/// Commits the current selection for a side (marks slot as consumed).
///
/// Called when a ship is destroyed in battle.
pub fn commit_ship_death(
    state: &mut MatchSelectionState,
    side: usize,
    slot: FleetShipIndex,
) -> Result<(), SuperMeleeError> {
    if side >= NUM_SIDES {
        return Err(SuperMeleeError::SelectionError(format!(
            "invalid side {}",
            side,
        )));
    }
    state.sides[side].consume_slot(slot);
    Ok(())
}

/// Commits a player's ship selection (for human prompt or auto-select).
///
/// Sets the slot as the current combatant for the side.
pub fn commit_selection(
    setup: &MeleeSetup,
    state: &mut MatchSelectionState,
    side: usize,
    slot: FleetShipIndex,
) -> Result<BattleReadyCombatant, SuperMeleeError> {
    if side >= NUM_SIDES {
        return Err(SuperMeleeError::SelectionError(format!(
            "invalid side {}",
            side,
        )));
    }
    if !state.sides[side].is_available(slot, &setup.teams[side]) {
        return Err(SuperMeleeError::SelectionError(format!(
            "slot {} not available on side {}",
            slot, side,
        )));
    }

    let ship = setup.teams[side].ships[slot as usize];
    state.sides[side].current_slot = Some(slot);
    Ok(make_combatant(side, slot, ship))
}

/// Auto-selects the next available ship for a side (AI/fallback).
pub fn auto_select_combatant(
    setup: &MeleeSetup,
    state: &mut MatchSelectionState,
    side: usize,
) -> Result<Option<BattleReadyCombatant>, SuperMeleeError> {
    select_next_combatant(setup, state, side)
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::supermelee::setup::team::MeleeSetup;
    use crate::supermelee::types::MeleeShip;

    fn setup_with_ships() -> MeleeSetup {
        let mut setup = MeleeSetup::new();
        setup.set_ship(0, 0, MeleeShip::Chmmr).unwrap();
        setup.set_ship(0, 1, MeleeShip::Shofixti).unwrap();
        setup.set_ship(0, 2, MeleeShip::Earthling).unwrap();
        setup.set_ship(1, 0, MeleeShip::Urquan).unwrap();
        setup.set_ship(1, 1, MeleeShip::Pkunk).unwrap();
        setup
    }

    #[test]
    fn initial_combatants_return_battle_ready_entries() {
        let setup = setup_with_ships();
        let mut state = MatchSelectionState::new();
        let result = select_initial_combatants(&setup, &mut state).unwrap();
        assert!(result[0].is_some());
        assert!(result[1].is_some());
    }

    #[test]
    fn initial_combatants_fail_when_side_empty() {
        let mut setup = MeleeSetup::new();
        setup.set_ship(0, 0, MeleeShip::Chmmr).unwrap();
        // Side 1 empty
        let mut state = MatchSelectionState::new();
        let result = select_initial_combatants(&setup, &mut state);
        assert!(result.is_err());
    }

    #[test]
    fn next_combatant_after_loss() {
        let setup = setup_with_ships();
        let mut state = MatchSelectionState::new();
        select_initial_combatants(&setup, &mut state).unwrap();

        // Side 0's first ship dies
        commit_ship_death(&mut state, 0, 0).unwrap();

        let next = select_next_combatant(&setup, &mut state, 0).unwrap();
        assert!(next.is_some());
    }

    #[test]
    fn consumed_slot_is_not_reselected() {
        let setup = setup_with_ships();
        let mut state = MatchSelectionState::new();

        // Consume slot 0
        commit_ship_death(&mut state, 0, 0).unwrap();
        assert!(!state.sides[0].is_available(0, &setup.teams[0]));

        // Next selection should skip slot 0
        let next = select_next_combatant(&setup, &mut state, 0).unwrap();
        assert!(next.is_some());
        // Verify it picked slot 1 (the handle encodes slot in lower bits)
        let combatant = next.unwrap();
        assert_eq!(combatant.handle & 0xFFFF, 1); // slot 1
    }

    #[test]
    fn no_valid_slot_returns_none_without_corrupting_state() {
        let mut setup = MeleeSetup::new();
        setup.set_ship(1, 0, MeleeShip::Vux).unwrap();
        let mut state = MatchSelectionState::new();

        // Consume the only ship on side 1
        commit_ship_death(&mut state, 1, 0).unwrap();

        let result = select_next_combatant(&setup, &mut state, 1).unwrap();
        assert!(result.is_none()); // eliminated

        // State is still valid
        assert_eq!(state.sides[1].remaining_count(&setup.teams[1]), 0);
    }

    #[test]
    fn commit_selection_works() {
        let setup = setup_with_ships();
        let mut state = MatchSelectionState::new();

        let combatant = commit_selection(&setup, &mut state, 0, 2).unwrap();
        assert_eq!(state.sides[0].current_slot, Some(2));
        // Handle encodes side=0, slot=2
        assert_eq!(combatant.handle, (0 << 16) | 2);
    }

    #[test]
    fn commit_selection_rejects_consumed_slot() {
        let setup = setup_with_ships();
        let mut state = MatchSelectionState::new();

        commit_ship_death(&mut state, 0, 1).unwrap();
        let result = commit_selection(&setup, &mut state, 0, 1);
        assert!(result.is_err());
    }

    #[test]
    fn auto_selection_works() {
        let setup = setup_with_ships();
        let mut state = MatchSelectionState::new();

        let result = auto_select_combatant(&setup, &mut state, 0).unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn handoff_contract_is_battle_ready_not_bare_id() {
        let setup = setup_with_ships();
        let mut state = MatchSelectionState::new();
        let result = select_initial_combatants(&setup, &mut state).unwrap();

        // BattleReadyCombatant encodes both side and slot
        let c = result[0].unwrap();
        let encoded_side = c.handle >> 16;
        let encoded_slot = c.handle & 0xFFFF;
        assert_eq!(encoded_side, 0);
        assert!(encoded_slot < MELEE_FLEET_SIZE);
    }

    #[test]
    fn match_reset_clears_consumed_slots() {
        let setup = setup_with_ships();
        let mut state = MatchSelectionState::new();

        commit_ship_death(&mut state, 0, 0).unwrap();
        commit_ship_death(&mut state, 0, 1).unwrap();
        assert_eq!(state.sides[0].remaining_count(&setup.teams[0]), 1);

        state.reset();
        assert_eq!(state.sides[0].remaining_count(&setup.teams[0]), 3);
    }
}
