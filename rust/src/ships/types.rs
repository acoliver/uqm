// Ship subsystem core types and enums.
// @plan PLAN-20260314-SHIPS.P03
// @requirement REQ-SHIP-IDENTITY, REQ-SHIP-DESCRIPTOR, REQ-CAPABILITY-FLAGS
//
// All types match C definitions in `sc2/src/uqm/races.h`.
// Bit positions, enum discriminants, and field widths are FFI-critical
// and must not be changed without updating the C side.

/// Number of resolution variants stored per ship (Big, Med, Sml).
pub const NUM_VIEWS: usize = 3;

/// Sentinel for "infinite sphere of influence radius" (C: `(COUNT) ~0`).
pub const INFINITE_RADIUS: u16 = !0;

/// Sentinel for "infinite fleet" crew in SHIP_FRAGMENT (C: `(COUNT) ~0`).
pub const INFINITE_FLEET: u16 = !0;

// ---------------------------------------------------------------------------
// SpeciesId
// ---------------------------------------------------------------------------

/// Unique species identity for every ship in the game.
///
/// Discriminant values match C `SPECIES_ID` in `races.h` exactly.
/// Melee-eligible species are `Arilou` (1) through `Mmrnmhrm` (25).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum SpeciesId {
    NoId = 0,
    Arilou = 1,
    Chmmr = 2,
    Earthling = 3,
    Orz = 4,
    Pkunk = 5,
    Shofixti = 6,
    Spathi = 7,
    Supox = 8,
    Thraddash = 9,
    Utwig = 10,
    Vux = 11,
    Yehat = 12,
    Melnorme = 13,
    Druuge = 14,
    Ilwrath = 15,
    Mycon = 16,
    Slylandro = 17,
    Umgah = 18,
    UrQuan = 19,
    Zoqfotpik = 20,
    Syreen = 21,
    KohrAh = 22,
    Androsynth = 23,
    Chenjesu = 24,
    Mmrnmhrm = 25,
    SisShip = 26,
    SaMatra = 27,
    UrQuanProbe = 28,
}

impl SpeciesId {
    /// Highest melee-eligible species discriminant (C: `LAST_MELEE_ID`).
    pub const LAST_MELEE_ID: i32 = SpeciesId::Mmrnmhrm as i32;

    /// Total number of species entries including `NoId` (C: `NUM_SPECIES_ID`).
    pub const NUM_SPECIES: i32 = 29;

    /// Whether this species can appear in Super Melee ship selection.
    pub const fn is_melee_eligible(self) -> bool {
        let v = self as i32;
        v >= SpeciesId::Arilou as i32 && v <= Self::LAST_MELEE_ID
    }

    /// Safe conversion from an `i32` discriminant.
    pub fn from_i32(val: i32) -> Option<SpeciesId> {
        match val {
            0 => Some(SpeciesId::NoId),
            1 => Some(SpeciesId::Arilou),
            2 => Some(SpeciesId::Chmmr),
            3 => Some(SpeciesId::Earthling),
            4 => Some(SpeciesId::Orz),
            5 => Some(SpeciesId::Pkunk),
            6 => Some(SpeciesId::Shofixti),
            7 => Some(SpeciesId::Spathi),
            8 => Some(SpeciesId::Supox),
            9 => Some(SpeciesId::Thraddash),
            10 => Some(SpeciesId::Utwig),
            11 => Some(SpeciesId::Vux),
            12 => Some(SpeciesId::Yehat),
            13 => Some(SpeciesId::Melnorme),
            14 => Some(SpeciesId::Druuge),
            15 => Some(SpeciesId::Ilwrath),
            16 => Some(SpeciesId::Mycon),
            17 => Some(SpeciesId::Slylandro),
            18 => Some(SpeciesId::Umgah),
            19 => Some(SpeciesId::UrQuan),
            20 => Some(SpeciesId::Zoqfotpik),
            21 => Some(SpeciesId::Syreen),
            22 => Some(SpeciesId::KohrAh),
            23 => Some(SpeciesId::Androsynth),
            24 => Some(SpeciesId::Chenjesu),
            25 => Some(SpeciesId::Mmrnmhrm),
            26 => Some(SpeciesId::SisShip),
            27 => Some(SpeciesId::SaMatra),
            28 => Some(SpeciesId::UrQuanProbe),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// ShipFlags  (capability flags -- C: UWORD ship_flags in SHIP_INFO)
// ---------------------------------------------------------------------------

/// Ship capability flags characterising externally observable combat properties.
///
/// Bit positions match C `races.h` defines exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ShipFlags(pub u16);

impl ShipFlags {
    pub const SEEKING_WEAPON: Self = Self(1 << 2);
    pub const SEEKING_SPECIAL: Self = Self(1 << 3);
    pub const POINT_DEFENSE: Self = Self(1 << 4);
    pub const IMMEDIATE_WEAPON: Self = Self(1 << 5);
    pub const CREW_IMMUNE: Self = Self(1 << 6);
    pub const FIRES_FORE: Self = Self(1 << 7);
    pub const FIRES_RIGHT: Self = Self(1 << 8);
    pub const FIRES_AFT: Self = Self(1 << 9);
    pub const FIRES_LEFT: Self = Self(1 << 10);
    pub const SHIELD_DEFENSE: Self = Self(1 << 11);
    pub const DONT_CHASE: Self = Self(1 << 12);
    pub const PLAYER_CAPTAIN: Self = Self(1 << 13);

    pub const fn empty() -> Self {
        Self(0)
    }
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl std::ops::BitOr for ShipFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitAnd for ShipFlags {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

impl std::ops::BitOrAssign for ShipFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl std::ops::BitAndAssign for ShipFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl std::ops::Not for ShipFlags {
    type Output = Self;
    fn not(self) -> Self {
        Self(!self.0)
    }
}

// ---------------------------------------------------------------------------
// StatusFlags  (runtime ship status -- C: STATUS_FLAGS in STARSHIP)
// ---------------------------------------------------------------------------

/// Runtime status flags for a ship during battle.
///
/// Bit positions match C `races.h` defines exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StatusFlags(pub u16);

impl StatusFlags {
    pub const LEFT: Self = Self(1 << 0);
    pub const RIGHT: Self = Self(1 << 1);
    pub const THRUST: Self = Self(1 << 2);
    pub const WEAPON: Self = Self(1 << 3);
    pub const SPECIAL: Self = Self(1 << 4);
    pub const LOW_ON_ENERGY: Self = Self(1 << 5);
    pub const SHIP_BEYOND_MAX_SPEED: Self = Self(1 << 6);
    pub const SHIP_AT_MAX_SPEED: Self = Self(1 << 7);
    pub const SHIP_IN_GRAVITY_WELL: Self = Self(1 << 8);
    pub const PLAY_VICTORY_DITTY: Self = Self(1 << 9);

    pub const fn empty() -> Self {
        Self(0)
    }
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl std::ops::BitOr for StatusFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitAnd for StatusFlags {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

impl std::ops::BitOrAssign for StatusFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl std::ops::BitAndAssign for StatusFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl std::ops::Not for StatusFlags {
    type Output = Self;
    fn not(self) -> Self {
        Self(!self.0)
    }
}

// ---------------------------------------------------------------------------
// AlliedState
// ---------------------------------------------------------------------------

/// Alliance status for a fleet (C: `allied_state` values in `FLEET_INFO`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum AlliedState {
    DeadGuy = 0,
    GoodGuy = 1,
    BadGuy = 2,
}

// ---------------------------------------------------------------------------
// ShipInfo  (C: SHIP_INFO)
// ---------------------------------------------------------------------------

/// Per-species ship metadata: costs, crew/energy caps, resource references.
#[derive(Debug, Clone)]
pub struct ShipInfo {
    pub ship_flags: ShipFlags,
    /// Super Melee point cost (C: BYTE ship_cost).
    pub ship_cost: u8,
    pub crew_level: u16,
    pub max_crew: u16,
    pub energy_level: u8,
    pub max_energy: u8,
    /// Resource ID for race-specific string table.
    pub race_strings_res: u32,
    /// Resource ID for ship selection icons.
    pub icons_res: u32,
    /// Resource ID for melee icon strip.
    pub melee_icon_res: u32,
    /// Loaded string table handle (0 = not loaded).
    pub race_strings: usize,
    /// Loaded icons frame handle (0 = not loaded).
    pub icons: usize,
    /// Loaded melee icon frame handle (0 = not loaded).
    pub melee_icon: usize,
}

impl Default for ShipInfo {
    fn default() -> Self {
        Self {
            ship_flags: ShipFlags::empty(),
            ship_cost: 0,
            crew_level: 0,
            max_crew: 0,
            energy_level: 0,
            max_energy: 0,
            race_strings_res: 0,
            icons_res: 0,
            melee_icon_res: 0,
            race_strings: 0,
            icons: 0,
            melee_icon: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// FleetStuff  (C: FLEET_STUFF)
// ---------------------------------------------------------------------------

/// Sphere-of-influence data for a species.
#[derive(Debug, Clone, Default)]
pub struct FleetStuff {
    /// Sphere of influence radius (C: COUNT strength).
    pub strength: u16,
    /// Last-known location (C: POINT known_loc).
    pub known_loc: (i16, i16),
}

// ---------------------------------------------------------------------------
// Characteristics  (C: CHARACTERISTIC_STUFF)
// ---------------------------------------------------------------------------

/// Movement, energy, and combat timing parameters.
#[derive(Debug, Clone, Default)]
pub struct Characteristics {
    pub max_thrust: u16,
    pub thrust_increment: u16,
    pub energy_regeneration: u8,
    pub weapon_energy_cost: u8,
    pub special_energy_cost: u8,
    pub energy_wait: u8,
    pub turn_wait: u8,
    pub thrust_wait: u8,
    pub weapon_wait: u8,
    pub special_wait: u8,
    pub ship_mass: u8,
}

// ---------------------------------------------------------------------------
// CaptainStuff  (C: CAPTAIN_STUFF)
// ---------------------------------------------------------------------------

/// Captain portrait animation frames.
#[derive(Debug, Clone, Default)]
pub struct CaptainStuff {
    pub captain_res: u32,
    pub background: usize,
    pub turn: usize,
    pub thrust: usize,
    pub weapon: usize,
    pub special: usize,
}

// ---------------------------------------------------------------------------
// ShipData  (C: DATA_STUFF)
// ---------------------------------------------------------------------------

/// Battle asset references and loaded handles for a ship.
#[derive(Debug, Clone)]
pub struct ShipData {
    pub ship_res: [u32; NUM_VIEWS],
    pub weapon_res: [u32; NUM_VIEWS],
    pub special_res: [u32; NUM_VIEWS],
    pub captain: CaptainStuff,
    pub victory_ditty_res: u32,
    pub ship_sounds_res: u32,
    /// Loaded ship graphic frames (one per resolution level).
    pub ship: [usize; NUM_VIEWS],
    /// Loaded weapon graphic frames.
    pub weapon: [usize; NUM_VIEWS],
    /// Loaded special graphic frames.
    pub special: [usize; NUM_VIEWS],
    /// Loaded victory music handle.
    pub victory_ditty: usize,
    /// Loaded ship sound effects handle.
    pub ship_sounds: usize,
}

impl Default for ShipData {
    fn default() -> Self {
        Self {
            ship_res: [0; NUM_VIEWS],
            weapon_res: [0; NUM_VIEWS],
            special_res: [0; NUM_VIEWS],
            captain: CaptainStuff::default(),
            victory_ditty_res: 0,
            ship_sounds_res: 0,
            ship: [0; NUM_VIEWS],
            weapon: [0; NUM_VIEWS],
            special: [0; NUM_VIEWS],
            victory_ditty: 0,
            ship_sounds: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// IntelStuff  (C: INTEL_STUFF -- without the intelligence_func pointer)
// ---------------------------------------------------------------------------

/// AI parameters for cyborg/computer-controlled ships.
#[derive(Debug, Clone, Default)]
pub struct IntelStuff {
    pub maneuverability_index: u16,
    pub weapon_range: u16,
}

// ---------------------------------------------------------------------------
// RaceDescTemplate  (static template -- no loaded assets or behavior)
// ---------------------------------------------------------------------------

/// Static template data returned by each ship behavior's `descriptor_template()`.
///
/// Contains resource IDs but no loaded handles (all handles zero).
/// The loader populates handles after receiving the template.
#[derive(Debug, Clone)]
pub struct RaceDescTemplate {
    pub ship_info: ShipInfo,
    pub fleet: FleetStuff,
    pub characteristics: Characteristics,
    pub ship_data: ShipData,
    pub intel: IntelStuff,
}

// ---------------------------------------------------------------------------
// MasterShipInfo  (C: master catalog entry)
// ---------------------------------------------------------------------------

/// Entry in the master ship catalog (metadata-only loaded).
#[derive(Debug, Clone)]
pub struct MasterShipInfo {
    pub species_id: SpeciesId,
    pub ship_info: ShipInfo,
    pub fleet: FleetStuff,
}

// ---------------------------------------------------------------------------
// ShipBehavior trait  (re-exported from traits module)
// ---------------------------------------------------------------------------

pub use super::traits::ShipBehavior;

// ---------------------------------------------------------------------------
// RaceDesc  (C: RACE_DESC — full runtime descriptor with behavior)
// ---------------------------------------------------------------------------

/// Full runtime ship descriptor aggregating all sub-structures.
///
/// Unlike `RaceDescTemplate`, this carries loaded asset handles and a
/// boxed behavior trait object. Created by the loader (P05) from a
/// template provided by the ship's `ShipBehavior` implementation.
pub struct RaceDesc {
    pub ship_info: ShipInfo,
    pub fleet: FleetStuff,
    pub characteristics: Characteristics,
    pub ship_data: ShipData,
    pub intel: IntelStuff,
    pub behavior: Box<dyn ShipBehavior>,
    /// Opaque private data owned by the race-specific behavior code.
    /// Matches C `void* data` in `RACE_DESC`.
    pub data: Option<Box<dyn std::any::Any + Send>>,
}

impl std::fmt::Debug for RaceDesc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RaceDesc")
            .field("ship_info", &self.ship_info)
            .field("fleet", &self.fleet)
            .field("characteristics", &self.characteristics)
            .field("intel", &self.intel)
            .field("behavior", &self.behavior)
            .finish_non_exhaustive()
    }
}

// ---------------------------------------------------------------------------
// Starship  (C: struct STARSHIP — battle queue entry)
// ---------------------------------------------------------------------------

/// A ship entry in the battle queue.
///
/// Tracks per-instance combat state: crew, energy, counters, facing.
/// The `race_desc` is loaded when the ship enters battle (spawn) and
/// freed when it exits (death/transition).
#[derive(Debug)]
pub struct Starship {
    pub species_id: SpeciesId,
    /// Loaded descriptor — `Some` only while ship is in battle.
    pub race_desc: Option<Box<RaceDesc>>,
    pub crew_level: u16,
    pub max_crew: u16,
    /// Super Melee point cost (C: BYTE ship_cost).
    pub ship_cost: u8,
    /// Original queue index (C: COUNT index).
    pub index: u16,
    /// Race strings handle (copied from ShipInfo for quick access).
    pub race_strings: usize,
    /// Icons handle (copied from ShipInfo for quick access).
    pub icons: usize,
    /// Frames until primary weapon can fire again.
    pub weapon_counter: u8,
    /// Frames until special can activate again.
    pub special_counter: u8,
    /// Frames until next energy regeneration tick.
    pub energy_counter: u8,
    /// Raw input state byte (C: BYTE ship_input_state).
    pub ship_input_state: u8,
    pub cur_status_flags: StatusFlags,
    pub old_status_flags: StatusFlags,
    /// Opaque element handle in the display list (C: HELEMENT hShip).
    pub h_ship: usize,
    /// Current ship facing (rotation index).
    pub ship_facing: u16,
    /// Player index: 0 = bottom/human, 1 = top/NPC, -1 = neutral.
    pub player_nr: i16,
    /// Control mode: human, computer, or network (C: BYTE control).
    pub control: u8,
    /// Captain name index; 0 means flagship in full-game.
    pub captains_name_index: u8,
    /// Audio stop flag — set by new_ship_transition() to signal C-side
    /// StopDitty/StopMusic/StopSound calls during ship replacement (P14 bridge).
    pub audio_stopped: bool,
}

impl Default for Starship {
    fn default() -> Self {
        Self {
            species_id: SpeciesId::NoId,
            race_desc: None,
            crew_level: 0,
            max_crew: 0,
            ship_cost: 0,
            index: 0,
            race_strings: 0,
            icons: 0,
            weapon_counter: 0,
            special_counter: 0,
            energy_counter: 0,
            ship_input_state: 0,
            cur_status_flags: StatusFlags::empty(),
            old_status_flags: StatusFlags::empty(),
            h_ship: 0,
            ship_facing: 0,
            player_nr: 0,
            control: 0,
            captains_name_index: 0,
            audio_stopped: false,
        }
    }
}

// ---------------------------------------------------------------------------
// ShipFragment  (C: SHIP_FRAGMENT — persistent fleet entry)
// ---------------------------------------------------------------------------

/// A ship entry in the fleet/fragment queue.
///
/// Persists across battles — tracks surviving crew and carries
/// resource handles for the ship selection UI.
#[derive(Debug, Clone)]
pub struct ShipFragment {
    pub species_id: SpeciesId,
    /// Captain name index (from SHIP_BASE_COMMON).
    pub captains_name_index: u8,
    /// Also stored as `race_id` (BYTE) in C for save-file compat.
    pub race_id: u8,
    /// Original queue index.
    pub index: u8,
    pub crew_level: u16,
    pub max_crew: u16,
    pub energy_level: u8,
    pub max_energy: u8,
    pub race_strings: usize,
    pub icons: usize,
    /// Only used by Shipyard UI.
    pub melee_icon: usize,
}

impl Default for ShipFragment {
    fn default() -> Self {
        Self {
            species_id: SpeciesId::NoId,
            captains_name_index: 0,
            race_id: 0,
            index: 0,
            crew_level: 0,
            max_crew: 0,
            energy_level: 0,
            max_energy: 0,
            race_strings: 0,
            icons: 0,
            melee_icon: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// FleetInfo  (C: FLEET_INFO — campaign fleet tracking)
// ---------------------------------------------------------------------------

/// Campaign-mode fleet tracking for each species.
///
/// Tracks alliance status, sphere of influence, fleet strength,
/// growth, and movement through hyperspace.
#[derive(Debug, Clone)]
pub struct FleetInfo {
    pub species_id: SpeciesId,
    pub allied_state: AlliedState,
    /// Days remaining before fleet reaches `dest_loc`.
    pub days_left: u8,
    pub growth_fract: u8,
    pub crew_level: u16,
    pub max_crew: u16,
    pub growth: u8,
    pub max_energy: u8,
    /// Current fleet center location.
    pub loc: (i16, i16),
    pub race_strings: usize,
    pub icons: usize,
    pub melee_icon: usize,
    /// Sphere of influence strength (0 = none, `!0` = handled separately).
    pub actual_strength: u16,
    /// Last-known SoI strength (0 = unknown).
    pub known_strength: u16,
    /// Last-known SoI center.
    pub known_loc: (i16, i16),
    pub growth_err_term: u8,
    /// Event function index; `0xFF` means no function.
    pub func_index: u8,
    /// Destination location the fleet is moving toward.
    pub dest_loc: (i16, i16),
}

impl Default for FleetInfo {
    fn default() -> Self {
        Self {
            species_id: SpeciesId::NoId,
            allied_state: AlliedState::DeadGuy,
            days_left: 0,
            growth_fract: 0,
            crew_level: 0,
            max_crew: 0,
            growth: 0,
            max_energy: 0,
            loc: (0, 0),
            race_strings: 0,
            icons: 0,
            melee_icon: 0,
            actual_strength: 0,
            known_strength: 0,
            known_loc: (0, 0),
            growth_err_term: 0,
            func_index: 0xFF,
            dest_loc: (0, 0),
        }
    }
}

// ---------------------------------------------------------------------------
// ShipsError
// ---------------------------------------------------------------------------

/// Errors produced by the ship subsystem.
#[derive(Debug, thiserror::Error)]
pub enum ShipsError {
    #[error("Unknown species ID: {0}")]
    UnknownSpecies(i32),
    #[error("Species not yet implemented for live battle: {0:?}")]
    UnimplementedSpecies(SpeciesId),
    #[error("Ship load failed: {0}")]
    LoadFailed(String),
    #[error("Ship spawn failed: {0}")]
    SpawnFailed(String),
    #[error("Ship subsystem not initialized")]
    NotInitialized,
    #[error("Ship subsystem already initialized")]
    AlreadyInitialized,
    #[error("Invalid ship state: {0}")]
    InvalidState(String),
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -- SpeciesId -----------------------------------------------------------

    #[test]
    fn species_id_from_i32_round_trip() {
        for val in 0..=28 {
            let species = SpeciesId::from_i32(val).unwrap();
            assert_eq!(species as i32, val);
        }
    }

    #[test]
    fn species_id_from_i32_invalid() {
        assert!(SpeciesId::from_i32(-1).is_none());
        assert!(SpeciesId::from_i32(29).is_none());
        assert!(SpeciesId::from_i32(100).is_none());
        assert!(SpeciesId::from_i32(i32::MAX).is_none());
        assert!(SpeciesId::from_i32(i32::MIN).is_none());
    }

    #[test]
    fn species_id_melee_eligible_boundaries() {
        assert!(!SpeciesId::NoId.is_melee_eligible());
        assert!(SpeciesId::Arilou.is_melee_eligible());
        assert!(SpeciesId::Mmrnmhrm.is_melee_eligible());
        assert!(!SpeciesId::SisShip.is_melee_eligible());
        assert!(!SpeciesId::SaMatra.is_melee_eligible());
        assert!(!SpeciesId::UrQuanProbe.is_melee_eligible());
    }

    #[test]
    fn species_id_all_melee_eligible() {
        let melee_species = [
            SpeciesId::Arilou,
            SpeciesId::Chmmr,
            SpeciesId::Earthling,
            SpeciesId::Orz,
            SpeciesId::Pkunk,
            SpeciesId::Shofixti,
            SpeciesId::Spathi,
            SpeciesId::Supox,
            SpeciesId::Thraddash,
            SpeciesId::Utwig,
            SpeciesId::Vux,
            SpeciesId::Yehat,
            SpeciesId::Melnorme,
            SpeciesId::Druuge,
            SpeciesId::Ilwrath,
            SpeciesId::Mycon,
            SpeciesId::Slylandro,
            SpeciesId::Umgah,
            SpeciesId::UrQuan,
            SpeciesId::Zoqfotpik,
            SpeciesId::Syreen,
            SpeciesId::KohrAh,
            SpeciesId::Androsynth,
            SpeciesId::Chenjesu,
            SpeciesId::Mmrnmhrm,
        ];
        assert_eq!(melee_species.len(), 25);
        for species in &melee_species {
            assert!(
                species.is_melee_eligible(),
                "{:?} should be melee eligible",
                species
            );
        }
    }

    #[test]
    fn species_id_constants() {
        assert_eq!(SpeciesId::LAST_MELEE_ID, 25);
        assert_eq!(SpeciesId::NUM_SPECIES, 29);
    }

    // -- ShipFlags -----------------------------------------------------------

    #[test]
    fn ship_flags_bit_values_match_c() {
        assert_eq!(ShipFlags::SEEKING_WEAPON.0, 0x04);
        assert_eq!(ShipFlags::SEEKING_SPECIAL.0, 0x08);
        assert_eq!(ShipFlags::POINT_DEFENSE.0, 0x10);
        assert_eq!(ShipFlags::IMMEDIATE_WEAPON.0, 0x20);
        assert_eq!(ShipFlags::CREW_IMMUNE.0, 0x40);
        assert_eq!(ShipFlags::FIRES_FORE.0, 0x80);
        assert_eq!(ShipFlags::FIRES_RIGHT.0, 0x100);
        assert_eq!(ShipFlags::FIRES_AFT.0, 0x200);
        assert_eq!(ShipFlags::FIRES_LEFT.0, 0x400);
        assert_eq!(ShipFlags::SHIELD_DEFENSE.0, 0x800);
        assert_eq!(ShipFlags::DONT_CHASE.0, 0x1000);
        assert_eq!(ShipFlags::PLAYER_CAPTAIN.0, 0x2000);
    }

    #[test]
    fn ship_flags_empty() {
        let f = ShipFlags::empty();
        assert!(f.is_empty());
        assert!(!f.contains(ShipFlags::FIRES_FORE));
    }

    #[test]
    fn ship_flags_bitor() {
        let f = ShipFlags::FIRES_FORE | ShipFlags::SEEKING_WEAPON;
        assert!(f.contains(ShipFlags::FIRES_FORE));
        assert!(f.contains(ShipFlags::SEEKING_WEAPON));
        assert!(!f.contains(ShipFlags::SHIELD_DEFENSE));
    }

    #[test]
    fn ship_flags_bitand() {
        let f = ShipFlags::FIRES_FORE | ShipFlags::SEEKING_WEAPON;
        let masked = f & ShipFlags::FIRES_FORE;
        assert!(masked.contains(ShipFlags::FIRES_FORE));
        assert!(!masked.contains(ShipFlags::SEEKING_WEAPON));
    }

    #[test]
    fn ship_flags_bitor_assign() {
        let mut f = ShipFlags::empty();
        f |= ShipFlags::PLAYER_CAPTAIN;
        assert!(f.contains(ShipFlags::PLAYER_CAPTAIN));
    }

    #[test]
    fn ship_flags_not() {
        let f = !ShipFlags::empty();
        assert!(f.contains(ShipFlags::FIRES_FORE));
        assert!(f.contains(ShipFlags::PLAYER_CAPTAIN));
    }

    // -- StatusFlags ---------------------------------------------------------

    #[test]
    fn status_flags_bit_values_match_c() {
        assert_eq!(StatusFlags::LEFT.0, 0x01);
        assert_eq!(StatusFlags::RIGHT.0, 0x02);
        assert_eq!(StatusFlags::THRUST.0, 0x04);
        assert_eq!(StatusFlags::WEAPON.0, 0x08);
        assert_eq!(StatusFlags::SPECIAL.0, 0x10);
        assert_eq!(StatusFlags::LOW_ON_ENERGY.0, 0x20);
        assert_eq!(StatusFlags::SHIP_BEYOND_MAX_SPEED.0, 0x40);
        assert_eq!(StatusFlags::SHIP_AT_MAX_SPEED.0, 0x80);
        assert_eq!(StatusFlags::SHIP_IN_GRAVITY_WELL.0, 0x100);
        assert_eq!(StatusFlags::PLAY_VICTORY_DITTY.0, 0x200);
    }

    #[test]
    fn status_flags_empty() {
        let s = StatusFlags::empty();
        assert!(s.is_empty());
    }

    #[test]
    fn status_flags_bitor() {
        let s = StatusFlags::LEFT | StatusFlags::THRUST | StatusFlags::WEAPON;
        assert!(s.contains(StatusFlags::LEFT));
        assert!(s.contains(StatusFlags::THRUST));
        assert!(s.contains(StatusFlags::WEAPON));
        assert!(!s.contains(StatusFlags::SPECIAL));
    }

    #[test]
    fn status_flags_bitand_assign() {
        let mut s = StatusFlags::LEFT | StatusFlags::RIGHT | StatusFlags::THRUST;
        s &= StatusFlags::LEFT | StatusFlags::THRUST;
        assert!(s.contains(StatusFlags::LEFT));
        assert!(s.contains(StatusFlags::THRUST));
        assert!(!s.contains(StatusFlags::RIGHT));
    }

    // -- AlliedState ---------------------------------------------------------

    #[test]
    fn allied_state_discriminants() {
        assert_eq!(AlliedState::DeadGuy as u16, 0);
        assert_eq!(AlliedState::GoodGuy as u16, 1);
        assert_eq!(AlliedState::BadGuy as u16, 2);
    }

    // -- Struct defaults and clone -------------------------------------------

    #[test]
    fn ship_info_default() {
        let info = ShipInfo::default();
        assert!(info.ship_flags.is_empty());
        assert_eq!(info.ship_cost, 0);
        assert_eq!(info.crew_level, 0);
        assert_eq!(info.race_strings, 0);
    }

    #[test]
    fn ship_info_clone() {
        let info = ShipInfo {
            ship_cost: 15,
            max_crew: 42,
            ..ShipInfo::default()
        };
        let cloned = info.clone();
        assert_eq!(cloned.ship_cost, 15);
        assert_eq!(cloned.max_crew, 42);
    }

    #[test]
    fn fleet_stuff_default() {
        let f = FleetStuff::default();
        assert_eq!(f.strength, 0);
        assert_eq!(f.known_loc, (0, 0));
    }

    #[test]
    fn characteristics_default() {
        let c = Characteristics::default();
        assert_eq!(c.max_thrust, 0);
        assert_eq!(c.ship_mass, 0);
    }

    #[test]
    fn captain_stuff_default() {
        let c = CaptainStuff::default();
        assert_eq!(c.captain_res, 0);
        assert_eq!(c.background, 0);
    }

    #[test]
    fn ship_data_default() {
        let d = ShipData::default();
        assert_eq!(d.ship_res, [0; NUM_VIEWS]);
        assert_eq!(d.ship, [0; NUM_VIEWS]);
        assert_eq!(d.victory_ditty, 0);
    }

    #[test]
    fn intel_stuff_default() {
        let i = IntelStuff::default();
        assert_eq!(i.maneuverability_index, 0);
        assert_eq!(i.weapon_range, 0);
    }

    #[test]
    fn master_ship_info_clone() {
        let m = MasterShipInfo {
            species_id: SpeciesId::Arilou,
            ship_info: ShipInfo::default(),
            fleet: FleetStuff::default(),
        };
        let cloned = m.clone();
        assert_eq!(cloned.species_id, SpeciesId::Arilou);
    }

    #[test]
    fn race_desc_template_clone() {
        let t = RaceDescTemplate {
            ship_info: ShipInfo::default(),
            fleet: FleetStuff::default(),
            characteristics: Characteristics::default(),
            ship_data: ShipData::default(),
            intel: IntelStuff::default(),
        };
        let cloned = t.clone();
        assert_eq!(cloned.ship_info.ship_cost, 0);
    }

    // -- Constants -----------------------------------------------------------

    #[test]
    fn infinite_constants() {
        assert_eq!(INFINITE_RADIUS, 0xFFFF);
        assert_eq!(INFINITE_FLEET, 0xFFFF);
    }

    #[test]
    fn num_views_constant() {
        assert_eq!(NUM_VIEWS, 3);
    }

    // -- Starship ------------------------------------------------------------

    #[test]
    fn starship_default() {
        let s = Starship::default();
        assert_eq!(s.species_id, SpeciesId::NoId);
        assert!(s.race_desc.is_none());
        assert_eq!(s.crew_level, 0);
        assert_eq!(s.weapon_counter, 0);
        assert!(s.cur_status_flags.is_empty());
        assert_eq!(s.player_nr, 0);
        assert_eq!(s.control, 0);
    }

    // -- ShipFragment --------------------------------------------------------

    #[test]
    fn ship_fragment_default() {
        let f = ShipFragment::default();
        assert_eq!(f.species_id, SpeciesId::NoId);
        assert_eq!(f.crew_level, 0);
        assert_eq!(f.melee_icon, 0);
    }

    #[test]
    fn ship_fragment_clone() {
        let f = ShipFragment {
            species_id: SpeciesId::Spathi,
            crew_level: 30,
            max_crew: 30,
            ..ShipFragment::default()
        };
        let cloned = f.clone();
        assert_eq!(cloned.species_id, SpeciesId::Spathi);
        assert_eq!(cloned.crew_level, 30);
    }

    // -- FleetInfo -----------------------------------------------------------

    #[test]
    fn fleet_info_default() {
        let fi = FleetInfo::default();
        assert_eq!(fi.species_id, SpeciesId::NoId);
        assert_eq!(fi.allied_state, AlliedState::DeadGuy);
        assert_eq!(fi.func_index, 0xFF);
        assert_eq!(fi.loc, (0, 0));
        assert_eq!(fi.dest_loc, (0, 0));
    }

    #[test]
    fn fleet_info_clone() {
        let fi = FleetInfo {
            species_id: SpeciesId::UrQuan,
            allied_state: AlliedState::BadGuy,
            actual_strength: 500,
            ..FleetInfo::default()
        };
        let cloned = fi.clone();
        assert_eq!(cloned.species_id, SpeciesId::UrQuan);
        assert_eq!(cloned.allied_state, AlliedState::BadGuy);
        assert_eq!(cloned.actual_strength, 500);
    }

    // -- RaceDesc ------------------------------------------------------------

    #[derive(Debug)]
    struct DummyBehavior;

    impl ShipBehavior for DummyBehavior {
        fn descriptor_template(&self) -> RaceDescTemplate {
            RaceDescTemplate {
                ship_info: ShipInfo::default(),
                fleet: FleetStuff::default(),
                characteristics: Characteristics::default(),
                ship_data: ShipData::default(),
                intel: IntelStuff::default(),
            }
        }
    }

    #[test]
    fn race_desc_aggregates_all_fields() {
        let desc = RaceDesc {
            ship_info: ShipInfo {
                ship_cost: 20,
                ..ShipInfo::default()
            },
            fleet: FleetStuff::default(),
            characteristics: Characteristics {
                max_thrust: 40,
                ..Characteristics::default()
            },
            ship_data: ShipData::default(),
            intel: IntelStuff {
                weapon_range: 100,
                ..IntelStuff::default()
            },
            behavior: Box::new(DummyBehavior),
            data: None,
        };
        assert_eq!(desc.ship_info.ship_cost, 20);
        assert_eq!(desc.characteristics.max_thrust, 40);
        assert_eq!(desc.intel.weapon_range, 100);
    }

    #[test]
    fn race_desc_debug_format() {
        let desc = RaceDesc {
            ship_info: ShipInfo::default(),
            fleet: FleetStuff::default(),
            characteristics: Characteristics::default(),
            ship_data: ShipData::default(),
            intel: IntelStuff::default(),
            behavior: Box::new(DummyBehavior),
            data: None,
        };
        let dbg = format!("{:?}", desc);
        assert!(dbg.contains("RaceDesc"));
    }

    // -- ShipBehavior trait ---------------------------------------------------

    #[test]
    fn ship_behavior_trait_object() {
        let b: Box<dyn ShipBehavior> = Box::new(DummyBehavior);
        let template = b.descriptor_template();
        assert_eq!(template.ship_info.ship_cost, 0);
    }

    // -- ShipsError ----------------------------------------------------------

    #[test]
    fn ships_error_display() {
        let e = ShipsError::UnknownSpecies(99);
        assert!(e.to_string().contains("99"));

        let e = ShipsError::NotInitialized;
        assert!(e.to_string().contains("not initialized"));
    }
}
