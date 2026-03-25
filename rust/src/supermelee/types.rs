// SuperMelee Core Types
// @plan PLAN-SUPERMELEE.P03, P04, P05
// @requirement Port of sc2/src/uqm/supermelee/meleeship.h and melee.h

// ---------------------------------------------------------------------------
// Fleet / grid constants  (from melee.h)
// ---------------------------------------------------------------------------

/// Number of rows in a fleet display grid.
pub const NUM_MELEE_ROWS: usize = 2;
/// Number of columns in a fleet display grid.
pub const NUM_MELEE_COLUMNS: usize = 7;
/// Total fleet capacity: rows × columns.
pub const MELEE_FLEET_SIZE: usize = NUM_MELEE_ROWS * NUM_MELEE_COLUMNS;

/// Maximum number of UTF-8/ASCII bytes in a team name (not counting NUL).
pub const MAX_TEAM_CHARS: usize = 30;

/// Number of opposing sides in a melee.
pub const NUM_SIDES: usize = 2;

/// Number of columns in the ship-pick grid.
pub const NUM_PICK_COLS: usize = 5;
/// Number of rows in the ship-pick grid.
pub const NUM_PICK_ROWS: usize = 5;

// ---------------------------------------------------------------------------
// MeleeShip  (from meleeship.h)
// ---------------------------------------------------------------------------

/// Identifies a ship race that can appear in a SuperMelee fleet.
///
/// Discriminant values match the C `enum MeleeShip` exactly so that
/// serialised team data is binary-compatible.  The special sentinels
/// `MELEE_UNSET` (254) and `MELEE_NONE` (255) use `u8` wrap-around
/// identical to the C `((BYTE) ~0) - 1` / `(BYTE) ~0` expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum MeleeShip {
    Androsynth = 0,
    Arilou = 1,
    Chenjesu = 2,
    Chmmr = 3,
    Druuge = 4,
    Earthling = 5,
    Ilwrath = 6,
    KohrAh = 7,
    Melnorme = 8,
    Mmrnmhrm = 9,
    Mycon = 10,
    Orz = 11,
    Pkunk = 12,
    Shofixti = 13,
    Slylandro = 14,
    Spathi = 15,
    Supox = 16,
    Syreen = 17,
    Thraddash = 18,
    Umgah = 19,
    Urquan = 20,
    Utwig = 21,
    Vux = 22,
    Yehat = 23,
    ZoqFotPik = 24,
    /// Sentinel used by the netplay Update protocol to mark unset sent-team
    /// slots.  Not a valid ship for gameplay purposes.
    MeleeUnset = 254,
    /// Empty fleet position — this slot contains no ship.
    MeleeNone = 255,
}

/// Total number of playable ship races (not counting sentinels).
pub const NUM_MELEE_SHIPS: usize = MeleeShip::ZoqFotPik as usize + 1; // 25

impl MeleeShip {
    /// Returns `true` if the ship is a real race (index 0..NUM_MELEE_SHIPS)
    /// or `MELEE_NONE` (an intentionally empty slot).
    ///
    /// `MELEE_UNSET` is **not** considered valid for gameplay.
    pub fn is_valid(self) -> bool {
        (self as u8) < NUM_MELEE_SHIPS as u8 || self == MeleeShip::MeleeNone
    }

    /// Attempt to construct a `MeleeShip` from a raw `u8`.
    ///
    /// Accepts 0–24, 254, and 255.  Everything else returns `None`.
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(MeleeShip::Androsynth),
            1 => Some(MeleeShip::Arilou),
            2 => Some(MeleeShip::Chenjesu),
            3 => Some(MeleeShip::Chmmr),
            4 => Some(MeleeShip::Druuge),
            5 => Some(MeleeShip::Earthling),
            6 => Some(MeleeShip::Ilwrath),
            7 => Some(MeleeShip::KohrAh),
            8 => Some(MeleeShip::Melnorme),
            9 => Some(MeleeShip::Mmrnmhrm),
            10 => Some(MeleeShip::Mycon),
            11 => Some(MeleeShip::Orz),
            12 => Some(MeleeShip::Pkunk),
            13 => Some(MeleeShip::Shofixti),
            14 => Some(MeleeShip::Slylandro),
            15 => Some(MeleeShip::Spathi),
            16 => Some(MeleeShip::Supox),
            17 => Some(MeleeShip::Syreen),
            18 => Some(MeleeShip::Thraddash),
            19 => Some(MeleeShip::Umgah),
            20 => Some(MeleeShip::Urquan),
            21 => Some(MeleeShip::Utwig),
            22 => Some(MeleeShip::Vux),
            23 => Some(MeleeShip::Yehat),
            24 => Some(MeleeShip::ZoqFotPik),
            254 => Some(MeleeShip::MeleeUnset),
            255 => Some(MeleeShip::MeleeNone),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// PlayerControl  (from melee.h / original C control flags)
// ---------------------------------------------------------------------------

/// Bitfield describing how a player's side is controlled.
///
/// Wraps a `u8` to allow arbitrary combinations of the flag constants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlayerControl(pub u8);

impl PlayerControl {
    /// Human player at the keyboard/gamepad.
    pub const HUMAN_CONTROL: Self = Self(1);
    /// Standard computer opponent (Cyborg tier).
    pub const CYBORG_CONTROL: Self = Self(2);
    /// Advanced computer opponent (Psytron tier).
    pub const PSYTRON_CONTROL: Self = Self(4);
    /// Remote player via network.
    pub const NETWORK_CONTROL: Self = Self(8);

    /// Any computer-controlled variant (Cyborg | Psytron).
    pub const COMPUTER_CONTROL: Self = Self(Self::CYBORG_CONTROL.0 | Self::PSYTRON_CONTROL.0);
    /// Mask of all control flags (Human | Computer | Network).
    pub const CONTROL_MASK: Self =
        Self(Self::HUMAN_CONTROL.0 | Self::COMPUTER_CONTROL.0 | Self::NETWORK_CONTROL.0);

    /// Returns `true` if all bits in `flag` are set in `self`.
    pub fn contains(self, flag: Self) -> bool {
        self.0 & flag.0 == flag.0
    }

    /// Returns the raw `u8` value.
    pub fn bits(self) -> u8 {
        self.0
    }
}

impl std::ops::BitOr for PlayerControl {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitAnd for PlayerControl {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

// ---------------------------------------------------------------------------
// FleetShipIndex  (from meleesetup.h — typedef COUNT FleetShipIndex)
// ---------------------------------------------------------------------------

/// Index into a fleet's ship array.  Matches C `typedef COUNT FleetShipIndex`.
pub type FleetShipIndex = u16;

// ---------------------------------------------------------------------------
// BattleReadyCombatant  (placeholder — to be refined in P08)
// ---------------------------------------------------------------------------

/// Placeholder for a ship that has been locked and is ready to enter battle.
///
/// The `handle` field will be replaced with a typed handle once the
/// full battle-handoff protocol is implemented in P08.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattleReadyCombatant {
    pub handle: usize,
}

// ---------------------------------------------------------------------------
// SelectionCommit
// ---------------------------------------------------------------------------

/// Records a finalised ship selection: which side placed which ship into
/// which fleet slot, plus the resolved battle combatant entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectionCommit {
    /// The side (0 or 1) that made the selection.
    pub side: usize,
    /// Fleet slot index within that side's fleet.
    pub slot: FleetShipIndex,
    /// The ship race selected.
    pub ship: MeleeShip,
    /// The resolved battle-ready combatant for this entry.
    pub battle_entry: BattleReadyCombatant,
}

// ===========================================================================
// Tests  (P04 — written before implementation, verified by P05)
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // MeleeShip discriminant ordering must match C enum exactly
    // -----------------------------------------------------------------------

    #[test]
    fn melee_ship_androsynth_is_zero() {
        assert_eq!(MeleeShip::Androsynth as u8, 0);
    }

    #[test]
    fn melee_ship_zoqfotpik_is_24() {
        assert_eq!(MeleeShip::ZoqFotPik as u8, 24);
    }

    #[test]
    fn melee_ship_none_is_255() {
        assert_eq!(MeleeShip::MeleeNone as u8, 255);
    }

    #[test]
    fn melee_ship_unset_is_254() {
        assert_eq!(MeleeShip::MeleeUnset as u8, 254);
    }

    #[test]
    fn melee_ship_full_ordering() {
        // Verify every race in C enum order (0–24)
        let ships: &[(MeleeShip, u8)] = &[
            (MeleeShip::Androsynth, 0),
            (MeleeShip::Arilou, 1),
            (MeleeShip::Chenjesu, 2),
            (MeleeShip::Chmmr, 3),
            (MeleeShip::Druuge, 4),
            (MeleeShip::Earthling, 5),
            (MeleeShip::Ilwrath, 6),
            (MeleeShip::KohrAh, 7),
            (MeleeShip::Melnorme, 8),
            (MeleeShip::Mmrnmhrm, 9),
            (MeleeShip::Mycon, 10),
            (MeleeShip::Orz, 11),
            (MeleeShip::Pkunk, 12),
            (MeleeShip::Shofixti, 13),
            (MeleeShip::Slylandro, 14),
            (MeleeShip::Spathi, 15),
            (MeleeShip::Supox, 16),
            (MeleeShip::Syreen, 17),
            (MeleeShip::Thraddash, 18),
            (MeleeShip::Umgah, 19),
            (MeleeShip::Urquan, 20),
            (MeleeShip::Utwig, 21),
            (MeleeShip::Vux, 22),
            (MeleeShip::Yehat, 23),
            (MeleeShip::ZoqFotPik, 24),
        ];
        for &(ship, expected) in ships {
            assert_eq!(ship as u8, expected, "{:?} discriminant mismatch", ship);
        }
    }

    // -----------------------------------------------------------------------
    // MeleeShip::is_valid
    // -----------------------------------------------------------------------

    #[test]
    fn is_valid_for_all_races() {
        for raw in 0u8..NUM_MELEE_SHIPS as u8 {
            let ship = MeleeShip::from_u8(raw).unwrap();
            assert!(ship.is_valid(), "{:?} should be valid", ship);
        }
    }

    #[test]
    fn melee_none_is_valid() {
        assert!(MeleeShip::MeleeNone.is_valid());
    }

    #[test]
    fn melee_unset_is_not_valid_for_gameplay() {
        // MELEE_UNSET is a netplay protocol sentinel, not a valid gameplay slot
        assert!(!MeleeShip::MeleeUnset.is_valid());
    }

    #[test]
    fn from_u8_out_of_range_returns_none() {
        // Raw values 25–253 have no corresponding variant
        for raw in 25u8..=253u8 {
            assert!(
                MeleeShip::from_u8(raw).is_none(),
                "from_u8({}) should be None",
                raw
            );
        }
    }

    // -----------------------------------------------------------------------
    // NUM_MELEE_SHIPS constant
    // -----------------------------------------------------------------------

    #[test]
    fn num_melee_ships_is_25() {
        assert_eq!(NUM_MELEE_SHIPS, 25);
    }

    // -----------------------------------------------------------------------
    // MELEE_FLEET_SIZE = NUM_MELEE_ROWS * NUM_MELEE_COLUMNS
    // -----------------------------------------------------------------------

    #[test]
    fn fleet_size_is_rows_times_columns() {
        assert_eq!(MELEE_FLEET_SIZE, NUM_MELEE_ROWS * NUM_MELEE_COLUMNS);
        assert_eq!(MELEE_FLEET_SIZE, 14);
    }

    // -----------------------------------------------------------------------
    // PlayerControl flag combinations
    // -----------------------------------------------------------------------

    #[test]
    fn player_control_human_bit() {
        assert_eq!(PlayerControl::HUMAN_CONTROL.bits(), 1);
    }

    #[test]
    fn player_control_cyborg_bit() {
        assert_eq!(PlayerControl::CYBORG_CONTROL.bits(), 2);
    }

    #[test]
    fn player_control_psytron_bit() {
        assert_eq!(PlayerControl::PSYTRON_CONTROL.bits(), 4);
    }

    #[test]
    fn player_control_network_bit() {
        assert_eq!(PlayerControl::NETWORK_CONTROL.bits(), 8);
    }

    #[test]
    fn player_control_computer_is_cyborg_or_psytron() {
        assert_eq!(
            PlayerControl::COMPUTER_CONTROL.bits(),
            PlayerControl::CYBORG_CONTROL.bits() | PlayerControl::PSYTRON_CONTROL.bits()
        );
        assert_eq!(PlayerControl::COMPUTER_CONTROL.bits(), 6);
    }

    #[test]
    fn player_control_mask_covers_all_flags() {
        assert_eq!(PlayerControl::CONTROL_MASK.bits(), 15);
        assert!(PlayerControl::CONTROL_MASK.contains(PlayerControl::HUMAN_CONTROL));
        assert!(PlayerControl::CONTROL_MASK.contains(PlayerControl::CYBORG_CONTROL));
        assert!(PlayerControl::CONTROL_MASK.contains(PlayerControl::PSYTRON_CONTROL));
        assert!(PlayerControl::CONTROL_MASK.contains(PlayerControl::NETWORK_CONTROL));
    }

    #[test]
    fn player_control_bitor() {
        let combined = PlayerControl::HUMAN_CONTROL | PlayerControl::NETWORK_CONTROL;
        assert_eq!(combined.bits(), 9);
    }

    #[test]
    fn player_control_contains() {
        let ctrl = PlayerControl::CYBORG_CONTROL | PlayerControl::PSYTRON_CONTROL;
        assert!(ctrl.contains(PlayerControl::COMPUTER_CONTROL));
        assert!(!ctrl.contains(PlayerControl::HUMAN_CONTROL));
    }
}
