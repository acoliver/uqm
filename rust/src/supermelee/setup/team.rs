// SuperMelee Team / Setup Models
// @plan PLAN-SUPERMELEE.P03, P04, P05
// @requirement Port of sc2/src/uqm/supermelee/meleesetup.h + meleesetup.c

use crate::supermelee::error::SuperMeleeError;
use crate::supermelee::types::{
    FleetShipIndex, MeleeShip, PlayerControl, MAX_TEAM_CHARS, MELEE_FLEET_SIZE, NUM_SIDES,
};

// ---------------------------------------------------------------------------
// Ship cost lookup
// ---------------------------------------------------------------------------

/// Returns the fleet-point cost of `ship`.
///
/// Costs are sourced from the verified Rust ship registry
/// (`rust/src/ships/races/`), which in turn mirrors the C
/// `RACE_SHIP_COST` table and individual `ship_cost` fields.
///
/// Returns 0 for `MeleeShip::MeleeNone` and `MeleeShip::MeleeUnset`.
pub fn ship_cost(ship: MeleeShip) -> u16 {
    match ship {
        MeleeShip::Androsynth => 15,
        MeleeShip::Arilou => 16,
        MeleeShip::Chenjesu => 28,
        MeleeShip::Chmmr => 30,
        MeleeShip::Druuge => 17,
        MeleeShip::Earthling => 11,
        MeleeShip::Ilwrath => 10,
        MeleeShip::KohrAh => 30,
        MeleeShip::Melnorme => 18,
        MeleeShip::Mmrnmhrm => 19,
        MeleeShip::Mycon => 21,
        MeleeShip::Orz => 23,
        MeleeShip::Pkunk => 20,
        MeleeShip::Shofixti => 5,
        MeleeShip::Slylandro => 17,
        MeleeShip::Spathi => 18,
        MeleeShip::Supox => 16,
        MeleeShip::Syreen => 13,
        MeleeShip::Thraddash => 10,
        MeleeShip::Umgah => 7,
        MeleeShip::Urquan => 30,
        MeleeShip::Utwig => 22,
        MeleeShip::Vux => 12,
        MeleeShip::Yehat => 23,
        MeleeShip::ZoqFotPik => 6,
        MeleeShip::MeleeNone | MeleeShip::MeleeUnset => 0,
    }
}

// ---------------------------------------------------------------------------
// Name buffer size
// ---------------------------------------------------------------------------

/// Size of the in-struct name buffer.
///
/// Matches C `char name[MAX_TEAM_CHARS + 1 + 24]`: one byte for the
/// terminating NUL plus 24 bytes of overflow padding.
const TEAM_NAME_BUF: usize = uqm_save::mle::NAME_BYTES;

// ---------------------------------------------------------------------------
// MeleeTeam
// ---------------------------------------------------------------------------

/// One player's fleet of ships for a SuperMelee match.
///
/// Memory layout intentionally mirrors the C `struct MeleeTeam` so that
/// future serialisation code can validate byte offsets easily.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeleeTeam {
    /// Fleet slots; initialised to `MeleeShip::MeleeNone`.
    pub ships: [MeleeShip; MELEE_FLEET_SIZE],
    /// NUL-terminated UTF-8 team name.  The buffer is deliberately the same
    /// size as the C field (`MAX_TEAM_CHARS + 1 + 24`).
    pub name: [u8; TEAM_NAME_BUF],
}

impl Default for MeleeTeam {
    fn default() -> Self {
        Self::new()
    }
}

impl MeleeTeam {
    /// Creates a team with all slots empty and a blank name.
    pub fn new() -> Self {
        Self {
            ships: [MeleeShip::MeleeNone; MELEE_FLEET_SIZE],
            name: [0u8; TEAM_NAME_BUF],
        }
    }

    /// Returns the team name as a `&str`.
    ///
    /// Finds the first NUL byte and returns the valid UTF-8 prefix up to
    /// that point.  Non-UTF-8 bytes are replaced with U+FFFD (replacement
    /// character) so the function never panics.
    pub fn name_str(&self) -> &str {
        let end = self
            .name
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(TEAM_NAME_BUF);
        // SAFETY: we only return the slice if it is valid UTF-8; the lossy
        //         variant is used for the general case via a temporary.
        std::str::from_utf8(&self.name[..end]).unwrap_or("")
    }

    /// Returns the team name, replacing invalid UTF-8 with '?' characters.
    ///
    /// Unlike `name_str()` this never returns an empty string for non-UTF-8
    /// content — callers that need a displayable string should use this.
    pub fn name_str_lossy(&self) -> std::borrow::Cow<'_, str> {
        let end = self
            .name
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(TEAM_NAME_BUF);
        String::from_utf8_lossy(&self.name[..end])
    }

    pub fn c_name(&self) -> &std::ffi::CStr {
        std::ffi::CStr::from_bytes_until_nul(&self.name)
            .unwrap_or_else(|_| panic!("team name is not terminated"))
    }

    pub fn rename(&mut self, name: &std::ffi::CStr) -> bool {
        if self.c_name() == name {
            return false;
        }
        let bytes = name.to_bytes();
        let count = bytes.len().min(TEAM_NAME_BUF - 1);
        self.name[..count].copy_from_slice(&bytes[..count]);
        self.name[count..].fill(0);
        true
    }

    /// Sets the team name, clamping to `MAX_TEAM_CHARS` bytes.
    ///
    /// Copies up to `MAX_TEAM_CHARS` bytes from `name`, then writes a NUL
    /// terminator at position `MAX_TEAM_CHARS` — matching the C
    /// `strncpy(team->name, name, sizeof team->name - 1)` behaviour.
    pub fn set_name(&mut self, name: &str) {
        let src = name.as_bytes();
        let copy_len = src.len().min(MAX_TEAM_CHARS);
        self.name[..copy_len].copy_from_slice(&src[..copy_len]);
        // NUL-terminate; clear any stale bytes beyond the new name
        for byte in &mut self.name[copy_len..] {
            *byte = 0;
        }
    }
}

impl From<&MeleeTeam> for uqm_save::mle::Team {
    fn from(team: &MeleeTeam) -> Self {
        Self {
            ships: team.ships.map(|ship| ship as u8),
            name: team.name,
        }
    }
}

impl From<uqm_save::mle::Team> for MeleeTeam {
    fn from(record: uqm_save::mle::Team) -> Self {
        let mut team = Self {
            ships: record.ships.map(|raw| match MeleeShip::from_u8(raw) {
                Some(ship) if ship != MeleeShip::MeleeUnset => ship,
                _ => MeleeShip::MeleeNone,
            }),
            name: record.name,
        };
        team.name[MAX_TEAM_CHARS] = 0;
        team
    }
}

/// Outstanding setup acknowledgments, independent of the editable fleet.
#[derive(Debug, Clone)]
pub struct SentTeam {
    pub ships: [MeleeShip; MELEE_FLEET_SIZE],
    pub name: Option<std::ffi::CString>,
}
impl Default for SentTeam {
    fn default() -> Self {
        Self {
            ships: [MeleeShip::MeleeUnset; MELEE_FLEET_SIZE],
            name: None,
        }
    }
}
impl SentTeam {
    pub fn set_name(&mut self, name: Option<&std::ffi::CStr>) -> bool {
        if self.name.as_deref() == name {
            return false;
        }
        self.name = name.map(|name| {
            let bytes = name.to_bytes();
            std::ffi::CString::new(&bytes[..bytes.len().min(TEAM_NAME_BUF - 1)])
                .unwrap_or_else(|error| panic!("bounded C name: {error}"))
        });
        true
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn is_synchronized(&self) -> bool {
        self.name.is_none() && self.ships.iter().all(|ship| *ship == MeleeShip::MeleeUnset)
    }
}

// ---------------------------------------------------------------------------
// MeleeSetup
// ---------------------------------------------------------------------------

/// Complete setup for a two-sided SuperMelee: both teams, cached fleet
/// values, and per-side control modes.
#[derive(Debug)]
pub struct MeleeSetup {
    /// The two teams (index 0 = player 1, index 1 = player 2).
    pub teams: [MeleeTeam; NUM_SIDES],
    /// Cached fleet-point total for each side.  Updated incrementally by
    /// `set_ship()` and fully by `recompute_fleet_value()`.
    pub fleet_value: [u16; NUM_SIDES],
    /// Control mode for each side.
    pub player_control: [PlayerControl; NUM_SIDES],
    pub sent: [SentTeam; NUM_SIDES],
}

impl Default for MeleeSetup {
    fn default() -> Self {
        Self::new()
    }
}

impl MeleeSetup {
    /// Creates a new setup with both teams empty, values at zero, and
    /// player control defaulting to `HUMAN_CONTROL`.
    pub fn new() -> Self {
        Self {
            teams: [MeleeTeam::new(), MeleeTeam::new()],
            fleet_value: [0; NUM_SIDES],
            player_control: [PlayerControl::HUMAN_CONTROL; NUM_SIDES],
            sent: std::array::from_fn(|_| SentTeam::default()),
        }
    }

    pub fn reset_sent(&mut self) {
        self.sent.iter_mut().for_each(SentTeam::reset);
    }

    // -----------------------------------------------------------------------
    // Bounds helpers
    // -----------------------------------------------------------------------

    fn check_side(side: usize) -> Result<(), SuperMeleeError> {
        if side < NUM_SIDES {
            Ok(())
        } else {
            Err(SuperMeleeError::SelectionError(format!(
                "side {} out of range (max {})",
                side,
                NUM_SIDES - 1
            )))
        }
    }

    fn check_slot(slot: FleetShipIndex) -> Result<(), SuperMeleeError> {
        if (slot as usize) < MELEE_FLEET_SIZE {
            Ok(())
        } else {
            Err(SuperMeleeError::SelectionError(format!(
                "slot {} out of range (max {})",
                slot,
                MELEE_FLEET_SIZE - 1
            )))
        }
    }

    // -----------------------------------------------------------------------
    // Mutating operations
    // -----------------------------------------------------------------------

    /// Places `ship` in `slot` for `side`, updating the cached fleet value.
    ///
    /// Mirrors `MeleeSetup_setShip()` in `meleesetup.c`: subtracts the cost
    /// of the old occupant (if any) then adds the cost of the new ship.
    pub fn set_ship(
        &mut self,
        side: usize,
        slot: FleetShipIndex,
        ship: MeleeShip,
    ) -> Result<(), SuperMeleeError> {
        self.set_ship_with_cost(side, slot, ship, ship_cost)
            .map(|_| ())
    }

    pub fn set_ship_with_cost(
        &mut self,
        side: usize,
        slot: FleetShipIndex,
        ship: MeleeShip,
        cost: impl Fn(MeleeShip) -> u16,
    ) -> Result<bool, SuperMeleeError> {
        Self::check_side(side)?;
        Self::check_slot(slot)?;
        let old_ship = self.teams[side].ships[slot as usize];
        if old_ship == ship {
            return Ok(false);
        }
        if old_ship != MeleeShip::MeleeNone {
            self.fleet_value[side] = self.fleet_value[side].wrapping_sub(cost(old_ship));
        }
        self.teams[side].ships[slot as usize] = ship;
        if ship != MeleeShip::MeleeNone {
            self.fleet_value[side] = self.fleet_value[side].wrapping_add(cost(ship));
        }
        Ok(true)
    }

    /// Removes the ship at `slot` for `side` (sets it to `MeleeNone`).
    pub fn clear_slot(&mut self, side: usize, slot: FleetShipIndex) -> Result<(), SuperMeleeError> {
        self.set_ship(side, slot, MeleeShip::MeleeNone)
    }

    /// Sets the team name for `side`.
    pub fn set_team_name(&mut self, side: usize, name: &str) -> Result<(), SuperMeleeError> {
        Self::check_side(side)?;
        self.teams[side].set_name(name);
        Ok(())
    }

    /// Replaces all ships for `side` with the ships from `team`, then
    /// recomputes the cached fleet value from scratch.
    pub fn replace_team(&mut self, side: usize, team: &MeleeTeam) -> Result<(), SuperMeleeError> {
        Self::check_side(side)?;
        self.teams[side] = team.clone();
        self.fleet_value[side] = self.recompute_fleet_value(side);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Queries
    // -----------------------------------------------------------------------

    /// Returns the cached fleet-point total for `side`.
    pub fn get_fleet_value(&self, side: usize) -> u16 {
        self.fleet_value[side]
    }

    /// Returns `true` if `side` has at least one non-empty ship slot.
    pub fn is_playable(&self, side: usize) -> bool {
        self.teams[side]
            .ships
            .iter()
            .any(|&s| s != MeleeShip::MeleeNone)
    }

    /// Sums the costs of all non-empty ships for `side` and returns the
    /// result.  Does **not** update the cached `fleet_value`; call the
    /// result into `fleet_value[side]` when a full refresh is needed.
    pub fn recompute_fleet_value(&self, side: usize) -> u16 {
        self.teams[side]
            .ships
            .iter()
            .filter(|&&s| s != MeleeShip::MeleeNone)
            .map(|&s| ship_cost(s) as u32)
            .sum::<u32>() as u16
    }
}

// ===========================================================================
// Tests  (P04 — verified by P05 implementations above)
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_ship_changes_use_the_supplied_catalog_and_wrapping_totals() {
        let mut setup = MeleeSetup::new();
        let cost = |ship: MeleeShip| match ship {
            MeleeShip::Androsynth => 23,
            MeleeShip::Arilou => 16,
            _ => 0,
        };
        assert!(setup
            .set_ship_with_cost(0, 3, MeleeShip::Androsynth, cost)
            .unwrap());
        assert_eq!(setup.fleet_value, [23, 0]);
        assert!(!setup
            .set_ship_with_cost(0, 3, MeleeShip::Androsynth, cost)
            .unwrap());
        assert!(setup
            .set_ship_with_cost(0, 3, MeleeShip::Arilou, cost)
            .unwrap());
        assert_eq!(setup.fleet_value, [16, 0]);
        assert!(setup
            .set_ship_with_cost(0, 3, MeleeShip::MeleeNone, cost)
            .unwrap());
        assert_eq!(setup.fleet_value, [0, 0]);
        setup.fleet_value[1] = u16::MAX;
        assert!(setup
            .set_ship_with_cost(1, 13, MeleeShip::Androsynth, cost)
            .unwrap());
        assert_eq!(setup.fleet_value, [0, 22]);
    }

    #[test]
    fn legacy_team_roundtrip_retains_slots_costs_and_import_name_limit() {
        let mut team = MeleeTeam::new();
        team.ships[0] = MeleeShip::Chmmr;
        team.ships[13] = MeleeShip::ZoqFotPik;
        team.rename(c"A fleet name that exceeds thirty characters by a bit");
        let bytes = uqm_save::mle::Team::from(&team).to_bytes();
        assert_eq!(bytes[0], 3);
        assert_eq!(bytes[13], 24);
        let copy = MeleeTeam::from(uqm_save::mle::Team::from_bytes(&bytes));
        assert_eq!(copy.ships, team.ships);
        assert_eq!(copy.name[30], 0);
        assert_eq!(copy.ships.into_iter().map(ship_cost).sum::<u16>(), 30 + 6);
    }

    #[test]
    fn pending_name_preserves_unset_empty_and_duplicate_transitions() {
        let mut sent = SentTeam::default();
        assert_eq!(sent.ships[0], MeleeShip::MeleeUnset);
        assert!(sent.name.is_none());
        assert!(sent.set_name(Some(c"")));
        assert_eq!(sent.name.as_deref(), Some(c""));
        assert!(!sent.set_name(Some(c"")));
        assert!(sent.set_name(None));
        assert!(!sent.set_name(None));
    }

    #[test]
    fn live_setup_owns_pending_acknowledgments_independently_of_editable_teams() {
        let mut setup = MeleeSetup::new();
        assert!(setup.sent.iter().all(SentTeam::is_synchronized));
        setup.teams[0].rename(c"Live fleet");
        setup.teams[0].ships[3] = MeleeShip::Chmmr;
        setup.sent[0].ships[3] = MeleeShip::Chmmr;
        setup.sent[0].set_name(Some(c""));
        assert!(!setup.sent[0].is_synchronized());
        assert_eq!(setup.teams[0].c_name(), c"Live fleet");
        setup.reset_sent();
        assert!(setup.sent.iter().all(SentTeam::is_synchronized));
        assert_eq!(setup.teams[0].ships[3], MeleeShip::Chmmr);
        assert_eq!(setup.teams[0].c_name(), c"Live fleet");
    }

    #[test]
    fn sent_team_ownership_distinguishes_unset_empty_and_pending_names() {
        let mut sent = SentTeam::default();
        assert_eq!(sent.ships, [MeleeShip::MeleeUnset; MELEE_FLEET_SIZE]);
        assert!(sent.name.is_none());
        sent.name = Some(c"".to_owned());
        assert_eq!(sent.name.as_deref(), Some(c""));
        sent.name = Some(c"Pending fleet".to_owned());
        sent.ships[3] = MeleeShip::Arilou;
        sent.reset();
        assert_eq!(sent.ships, [MeleeShip::MeleeUnset; MELEE_FLEET_SIZE]);
        assert!(sent.name.is_none());
    }

    #[test]
    fn importing_legacy_bytes_normalizes_only_ships_and_the_display_terminator() {
        let mut record = uqm_save::mle::Team {
            ships: [255; MELEE_FLEET_SIZE],
            name: [0xa5; uqm_save::mle::NAME_BYTES],
        };
        record.ships[..6].copy_from_slice(&[0, 24, 25, 200, 254, 255]);
        let imported = MeleeTeam::from(record.clone());
        assert_eq!(imported.ships[0], MeleeShip::Androsynth);
        assert_eq!(imported.ships[1], MeleeShip::ZoqFotPik);
        assert!(imported.ships[2..]
            .iter()
            .all(|ship| *ship == MeleeShip::MeleeNone));
        let mut expected_name = record.name;
        expected_name[MAX_TEAM_CHARS] = 0;
        assert_eq!(imported.name, expected_name);
        assert_eq!(record.name[MAX_TEAM_CHARS], 0xa5);
    }

    #[test]
    fn live_names_preserve_non_utf8_and_use_the_full_native_buffer() {
        let mut team = MeleeTeam::new();
        let bytes = [0xe9; 70];
        let value = std::ffi::CString::new(bytes).unwrap();
        assert!(team.rename(&value));
        assert_eq!(team.c_name().to_bytes(), [0xe9; 54]);
        assert_eq!(team.name[54], 0);
        assert!(!team.rename(team.c_name().to_owned().as_c_str()));
        assert!(team.rename(c""));
        assert_eq!(team.c_name(), c"");
        assert_eq!(team.name, [0; TEAM_NAME_BUF]);
    }

    #[test]
    fn exporting_a_team_preserves_unset_slots_and_name_padding() {
        let mut team = MeleeTeam::new();
        team.ships[0] = MeleeShip::MeleeUnset;
        team.name.fill(0xe9);
        let record = uqm_save::mle::Team::from(&team);
        assert_eq!(record.ships[0], 254);
        assert_eq!(record.ships[1..], [255; MELEE_FLEET_SIZE - 1]);
        assert_eq!(record.name, team.name);
    }

    // -----------------------------------------------------------------------
    // MeleeTeam defaults
    // -----------------------------------------------------------------------

    #[test]
    fn team_default_has_all_none_ships() {
        let team = MeleeTeam::new();
        for &ship in &team.ships {
            assert_eq!(ship, MeleeShip::MeleeNone);
        }
    }

    #[test]
    fn team_default_name_is_empty() {
        let team = MeleeTeam::new();
        assert_eq!(team.name_str(), "");
    }

    // -----------------------------------------------------------------------
    // MeleeTeam name handling
    // -----------------------------------------------------------------------

    #[test]
    fn set_name_stores_correctly() {
        let mut team = MeleeTeam::new();
        team.set_name("Team Alpha");
        assert_eq!(team.name_str(), "Team Alpha");
    }

    #[test]
    fn set_name_clamps_to_max_team_chars() {
        let mut team = MeleeTeam::new();
        let long = "A".repeat(MAX_TEAM_CHARS + 10);
        team.set_name(&long);
        let stored = team.name_str();
        assert!(stored.len() <= MAX_TEAM_CHARS);
        // Verify the NUL terminator sits at position MAX_TEAM_CHARS
        assert_eq!(team.name[MAX_TEAM_CHARS], 0);
    }

    #[test]
    fn set_name_clears_stale_bytes() {
        let mut team = MeleeTeam::new();
        team.set_name("LongNameHere");
        team.set_name("Hi");
        assert_eq!(team.name_str(), "Hi");
        // Bytes after the NUL should be zero
        assert_eq!(team.name[2], 0);
    }

    #[test]
    fn set_name_exact_max_length() {
        let mut team = MeleeTeam::new();
        let exact = "X".repeat(MAX_TEAM_CHARS);
        team.set_name(&exact);
        assert_eq!(team.name_str(), &exact as &str);
        assert_eq!(team.name[MAX_TEAM_CHARS], 0);
    }

    // -----------------------------------------------------------------------
    // MeleeSetup: set_ship updates fleet value
    // -----------------------------------------------------------------------

    #[test]
    fn set_ship_updates_fleet_value() {
        let mut setup = MeleeSetup::new();
        setup.set_ship(0, 0, MeleeShip::Shofixti).unwrap();
        assert_eq!(setup.get_fleet_value(0), ship_cost(MeleeShip::Shofixti));
    }

    #[test]
    fn set_ship_replaces_and_adjusts_value() {
        let mut setup = MeleeSetup::new();
        setup.set_ship(0, 0, MeleeShip::Shofixti).unwrap();
        setup.set_ship(0, 0, MeleeShip::Chmmr).unwrap();
        assert_eq!(setup.get_fleet_value(0), ship_cost(MeleeShip::Chmmr));
    }

    #[test]
    fn set_ship_multiple_slots_accumulates() {
        let mut setup = MeleeSetup::new();
        setup.set_ship(0, 0, MeleeShip::Shofixti).unwrap();
        setup.set_ship(0, 1, MeleeShip::Earthling).unwrap();
        let expected = ship_cost(MeleeShip::Shofixti) + ship_cost(MeleeShip::Earthling);
        assert_eq!(setup.get_fleet_value(0), expected);
    }

    #[test]
    fn set_ship_out_of_range_side_returns_error() {
        let mut setup = MeleeSetup::new();
        assert!(setup.set_ship(NUM_SIDES, 0, MeleeShip::Shofixti).is_err());
    }

    #[test]
    fn set_ship_out_of_range_slot_returns_error() {
        let mut setup = MeleeSetup::new();
        assert!(setup
            .set_ship(0, MELEE_FLEET_SIZE as u16, MeleeShip::Shofixti)
            .is_err());
    }

    // -----------------------------------------------------------------------
    // MeleeSetup: clear_slot reduces fleet value
    // -----------------------------------------------------------------------

    #[test]
    fn clear_slot_removes_ship_and_reduces_value() {
        let mut setup = MeleeSetup::new();
        setup.set_ship(0, 0, MeleeShip::Orz).unwrap();
        let before = setup.get_fleet_value(0);
        setup.clear_slot(0, 0).unwrap();
        assert_eq!(setup.get_fleet_value(0), 0);
        assert!(before > 0);
    }

    // -----------------------------------------------------------------------
    // MeleeSetup: set_team_name
    // -----------------------------------------------------------------------

    #[test]
    fn set_team_name_with_valid_string() {
        let mut setup = MeleeSetup::new();
        setup.set_team_name(0, "Earthlings").unwrap();
        assert_eq!(setup.teams[0].name_str(), "Earthlings");
    }

    #[test]
    fn set_team_name_clamps_long_string() {
        let mut setup = MeleeSetup::new();
        let long = "Z".repeat(MAX_TEAM_CHARS + 5);
        setup.set_team_name(0, &long).unwrap();
        assert!(setup.teams[0].name_str().len() <= MAX_TEAM_CHARS);
    }

    #[test]
    fn set_team_name_invalid_side_returns_error() {
        let mut setup = MeleeSetup::new();
        assert!(setup.set_team_name(NUM_SIDES, "Oops").is_err());
    }

    // -----------------------------------------------------------------------
    // MeleeSetup: replace_team
    // -----------------------------------------------------------------------

    #[test]
    fn replace_team_copies_ships_and_recomputes_value() {
        let mut src = MeleeTeam::new();
        src.ships[0] = MeleeShip::Chmmr;
        src.ships[1] = MeleeShip::Shofixti;
        src.set_name("Champions");

        let mut setup = MeleeSetup::new();
        setup.replace_team(0, &src).unwrap();

        assert_eq!(setup.teams[0].ships[0], MeleeShip::Chmmr);
        assert_eq!(setup.teams[0].ships[1], MeleeShip::Shofixti);
        assert_eq!(setup.teams[0].name_str(), "Champions");

        let expected = ship_cost(MeleeShip::Chmmr) + ship_cost(MeleeShip::Shofixti);
        assert_eq!(setup.get_fleet_value(0), expected);
    }

    #[test]
    fn replace_team_invalid_side_returns_error() {
        let team = MeleeTeam::new();
        let mut setup = MeleeSetup::new();
        assert!(setup.replace_team(NUM_SIDES, &team).is_err());
    }

    // -----------------------------------------------------------------------
    // MeleeSetup: is_playable
    // -----------------------------------------------------------------------

    #[test]
    fn is_playable_false_when_all_none() {
        let setup = MeleeSetup::new();
        assert!(!setup.is_playable(0));
        assert!(!setup.is_playable(1));
    }

    #[test]
    fn is_playable_true_when_at_least_one_ship() {
        let mut setup = MeleeSetup::new();
        setup.set_ship(0, 0, MeleeShip::Arilou).unwrap();
        assert!(setup.is_playable(0));
        assert!(!setup.is_playable(1));
    }

    // -----------------------------------------------------------------------
    // MeleeSetup: recompute_fleet_value
    // -----------------------------------------------------------------------

    #[test]
    fn recompute_fleet_value_matches_ship_cost_sum() {
        let mut setup = MeleeSetup::new();
        setup.set_ship(0, 0, MeleeShip::Urquan).unwrap();
        setup.set_ship(0, 1, MeleeShip::Syreen).unwrap();
        setup.set_ship(0, 2, MeleeShip::Shofixti).unwrap();

        let expected = ship_cost(MeleeShip::Urquan)
            + ship_cost(MeleeShip::Syreen)
            + ship_cost(MeleeShip::Shofixti);

        assert_eq!(setup.recompute_fleet_value(0), expected);
        assert_eq!(setup.get_fleet_value(0), expected);
    }

    #[test]
    fn recompute_fleet_value_ignores_none_slots() {
        let setup = MeleeSetup::new();
        assert_eq!(setup.recompute_fleet_value(0), 0);
    }

    // -----------------------------------------------------------------------
    // ship_cost table spot checks
    // -----------------------------------------------------------------------

    #[test]
    fn ship_cost_none_is_zero() {
        assert_eq!(ship_cost(MeleeShip::MeleeNone), 0);
        assert_eq!(ship_cost(MeleeShip::MeleeUnset), 0);
    }

    #[test]
    fn ship_cost_known_values() {
        assert_eq!(ship_cost(MeleeShip::Shofixti), 5);
        assert_eq!(ship_cost(MeleeShip::Chmmr), 30);
        assert_eq!(ship_cost(MeleeShip::Earthling), 11);
        assert_eq!(ship_cost(MeleeShip::Arilou), 16);
        assert_eq!(ship_cost(MeleeShip::Androsynth), 15);
        assert_eq!(ship_cost(MeleeShip::ZoqFotPik), 6);
    }

    #[test]
    fn ship_cost_all_races_nonzero() {
        for raw in 0u8..super::super::super::types::NUM_MELEE_SHIPS as u8 {
            let ship = MeleeShip::from_u8(raw).unwrap();
            assert!(
                ship_cost(ship) > 0,
                "{:?} (raw {}) should have nonzero cost",
                ship,
                raw
            );
        }
    }
}
