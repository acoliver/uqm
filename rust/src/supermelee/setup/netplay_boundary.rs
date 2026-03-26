// SuperMelee Netplay Boundary — sync events and remote validation
// @plan PLAN-20260314-SUPERMELEE.P09
// @requirement netplay boundary

use crate::supermelee::error::SuperMeleeError;
use crate::supermelee::setup::pick_melee::{commit_selection, MatchSelectionState};
use crate::supermelee::setup::team::MeleeSetup;
use crate::supermelee::types::{
    BattleReadyCombatant, FleetShipIndex, MeleeShip, MELEE_FLEET_SIZE, NUM_SIDES,
};

// ---------------------------------------------------------------------------
// Sync events — emitted by setup changes for netplay transport
// ---------------------------------------------------------------------------

/// Events emitted when the local player modifies their setup.
///
/// These are consumed by the netplay transport layer (owned by the netplay
/// subsystem, not SuperMelee) to synchronize with the remote peer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetupSyncEvent {
    /// A single ship slot was changed.
    ShipSlotChanged {
        side: usize,
        slot: FleetShipIndex,
        ship: MeleeShip,
    },
    /// The team name was changed.
    TeamNameChanged { side: usize, name: String },
    /// A whole team was replaced (bootstrap or load).
    WholeTeamSync { side: usize },
    /// Local combatant selection result for the boundary.
    CombatantSelected {
        side: usize,
        slot: FleetShipIndex,
        ship: MeleeShip,
    },
}

// ---------------------------------------------------------------------------
// Netplay connection state (boundary-owned view)
// ---------------------------------------------------------------------------

/// Connection and readiness state as seen from the SuperMelee boundary.
///
/// This is NOT the full netplay connection — it's the minimal state that
/// SuperMelee needs for start gating and validation.
#[derive(Debug, Clone)]
pub struct NetplayBoundaryState {
    /// Whether a netplay connection is active.
    pub connected: bool,
    /// Whether each side is ready (confirmed for start).
    pub side_ready: [bool; NUM_SIDES],
    /// Whether both sides have confirmed start.
    pub start_confirmed: bool,
}

impl Default for NetplayBoundaryState {
    fn default() -> Self {
        Self::new()
    }
}

impl NetplayBoundaryState {
    pub fn new() -> Self {
        Self {
            connected: false,
            side_ready: [false; NUM_SIDES],
            start_confirmed: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Local-only mode (no netplay)
// ---------------------------------------------------------------------------

/// Returns `true` if netplay is not active (local-only SuperMelee).
///
/// When netplay is disabled, all sync events are no-ops and start gating
/// only checks playable fleets (no connection/readiness requirements).
pub fn is_local_mode(netplay: &Option<NetplayBoundaryState>) -> bool {
    netplay.is_none()
}

// ---------------------------------------------------------------------------
// Start gating
// ---------------------------------------------------------------------------

/// Checks whether a netplay match can start.
///
/// Requirements: connected + both sides ready + start confirmed.
pub fn can_start_netplay_match(state: &NetplayBoundaryState) -> bool {
    state.connected && state.side_ready[0] && state.side_ready[1] && state.start_confirmed
}

// ---------------------------------------------------------------------------
// Remote selection validation
// ---------------------------------------------------------------------------

/// Validates and commits a remote player's ship selection.
///
/// Checks:
/// 1. Side is in range
/// 2. Slot is in range
/// 3. Ship at slot matches the claimed ship (fleet consistency)
/// 4. Slot is not already consumed
///
/// On success, commits the selection and returns the combatant.
/// On failure, returns an error WITHOUT modifying any state.
pub fn validate_remote_selection(
    setup: &MeleeSetup,
    selection_state: &mut MatchSelectionState,
    side: usize,
    slot: FleetShipIndex,
    claimed_ship: MeleeShip,
) -> Result<BattleReadyCombatant, SuperMeleeError> {
    if side >= NUM_SIDES {
        return Err(SuperMeleeError::NetplayValidationError(format!(
            "invalid side {}",
            side,
        )));
    }
    if (slot as usize) >= MELEE_FLEET_SIZE {
        return Err(SuperMeleeError::NetplayValidationError(format!(
            "invalid slot {}",
            slot,
        )));
    }

    // Check fleet consistency
    let actual_ship = setup.teams[side].ships[slot as usize];
    if actual_ship != claimed_ship {
        return Err(SuperMeleeError::NetplayValidationError(format!(
            "fleet mismatch: slot {} has {:?} but remote claims {:?}",
            slot, actual_ship, claimed_ship,
        )));
    }

    // Check not consumed
    if !selection_state.sides[side].is_available(slot, &setup.teams[side]) {
        return Err(SuperMeleeError::NetplayValidationError(format!(
            "slot {} on side {} is consumed or empty",
            slot, side,
        )));
    }

    // Commit
    commit_selection(setup, selection_state, side, slot)
        .map_err(|e| SuperMeleeError::NetplayValidationError(e.to_string()))
}

// ---------------------------------------------------------------------------
// Sync event generation helpers
// ---------------------------------------------------------------------------

/// Creates a sync event for a ship slot change.
pub fn emit_ship_slot_changed(
    side: usize,
    slot: FleetShipIndex,
    ship: MeleeShip,
) -> SetupSyncEvent {
    SetupSyncEvent::ShipSlotChanged { side, slot, ship }
}

/// Creates a sync event for a team name change.
pub fn emit_team_name_changed(side: usize, name: &str) -> SetupSyncEvent {
    SetupSyncEvent::TeamNameChanged {
        side,
        name: name.to_string(),
    }
}

/// Creates a sync event for a whole team sync/bootstrap.
pub fn emit_whole_team_sync(side: usize) -> SetupSyncEvent {
    SetupSyncEvent::WholeTeamSync { side }
}

/// Creates a sync event exposing local combatant selection.
pub fn emit_combatant_selected(
    side: usize,
    slot: FleetShipIndex,
    ship: MeleeShip,
) -> SetupSyncEvent {
    SetupSyncEvent::CombatantSelected { side, slot, ship }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::supermelee::setup::pick_melee::{commit_ship_death, MatchSelectionState};
    use crate::supermelee::setup::team::MeleeSetup;
    use crate::supermelee::types::MeleeShip;

    fn setup_with_ships() -> MeleeSetup {
        let mut setup = MeleeSetup::new();
        setup.set_ship(0, 0, MeleeShip::Chmmr).unwrap();
        setup.set_ship(0, 1, MeleeShip::Shofixti).unwrap();
        setup.set_ship(1, 0, MeleeShip::Urquan).unwrap();
        setup.set_ship(1, 1, MeleeShip::Pkunk).unwrap();
        setup
    }

    #[test]
    fn local_mode_requires_no_network_state() {
        unsafe {
            assert!(is_local_mode(&None));
            assert!(!is_local_mode(&Some(NetplayBoundaryState::new())));
        }
    }

    #[test]
    fn ship_slot_change_emits_sync_event() {
        unsafe {
            let evt = emit_ship_slot_changed(0, 3, MeleeShip::Orz);
            assert_eq!(
                evt,
                SetupSyncEvent::ShipSlotChanged {
                    side: 0,
                    slot: 3,
                    ship: MeleeShip::Orz,
                }
            );
        }
    }

    #[test]
    fn team_name_change_emits_sync_event() {
        unsafe {
            let evt = emit_team_name_changed(1, "New Name");
            assert_eq!(
                evt,
                SetupSyncEvent::TeamNameChanged {
                    side: 1,
                    name: "New Name".to_string(),
                }
            );
        }
    }

    #[test]
    fn whole_team_sync_emits_event() {
        unsafe {
            let evt = emit_whole_team_sync(0);
            assert_eq!(evt, SetupSyncEvent::WholeTeamSync { side: 0 });
        }
    }

    #[test]
    fn start_blocked_without_connection() {
        unsafe {
            let state = NetplayBoundaryState::new();
            assert!(!can_start_netplay_match(&state));
        }
    }

    #[test]
    fn start_blocked_without_readiness() {
        unsafe {
            let mut state = NetplayBoundaryState::new();
            state.connected = true;
            assert!(!can_start_netplay_match(&state));
        }
    }

    #[test]
    fn start_blocked_without_confirmation() {
        unsafe {
            let mut state = NetplayBoundaryState::new();
            state.connected = true;
            state.side_ready = [true; NUM_SIDES];
            assert!(!can_start_netplay_match(&state));
        }
    }

    #[test]
    fn start_allowed_when_all_conditions_met() {
        unsafe {
            let mut state = NetplayBoundaryState::new();
            state.connected = true;
            state.side_ready = [true; NUM_SIDES];
            state.start_confirmed = true;
            assert!(can_start_netplay_match(&state));
        }
    }

    #[test]
    fn local_selection_outcome_exposed_to_boundary() {
        unsafe {
            let evt = emit_combatant_selected(0, 2, MeleeShip::Vux);
            assert_eq!(
                evt,
                SetupSyncEvent::CombatantSelected {
                    side: 0,
                    slot: 2,
                    ship: MeleeShip::Vux,
                }
            );
        }
    }

    #[test]
    fn valid_remote_selection_accepted_and_committed() {
        unsafe {
            let setup = setup_with_ships();
            let mut sel_state = MatchSelectionState::new();

            let result = validate_remote_selection(&setup, &mut sel_state, 1, 0, MeleeShip::Urquan);
            assert!(result.is_ok());
            assert_eq!(sel_state.sides[1].current_slot(), Some(0));
        }
    }

    #[test]
    fn invalid_remote_selection_rejected_fleet_mismatch() {
        unsafe {
            let setup = setup_with_ships();
            let mut sel_state = MatchSelectionState::new();

            // Claim slot 0 has Pkunk, but it actually has Urquan
            let result = validate_remote_selection(&setup, &mut sel_state, 1, 0, MeleeShip::Pkunk);
            assert!(result.is_err());
        }
    }

    #[test]
    fn remote_selection_for_consumed_ship_rejected() {
        unsafe {
            let setup = setup_with_ships();
            let mut sel_state = MatchSelectionState::new();

            commit_ship_death(&mut sel_state, 1, 0).unwrap();

            let result = validate_remote_selection(&setup, &mut sel_state, 1, 0, MeleeShip::Urquan);
            assert!(result.is_err());
        }
    }

    #[test]
    fn accepted_remote_selection_not_rejected_after_commit() {
        unsafe {
            let setup = setup_with_ships();
            let mut sel_state = MatchSelectionState::new();

            let combatant =
                validate_remote_selection(&setup, &mut sel_state, 0, 0, MeleeShip::Chmmr).unwrap();
            // The combatant was committed — verify state is consistent
            assert_eq!(sel_state.sides[0].current_slot(), Some(0));
            // A second valid selection on a different slot should also work
            let _combatant2 =
                validate_remote_selection(&setup, &mut sel_state, 0, 1, MeleeShip::Shofixti)
                    .unwrap();
            assert_eq!(sel_state.sides[0].current_slot(), Some(1));
        }
    }
}
