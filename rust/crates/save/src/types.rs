//! Typed model of a UQM Save version 1 ("UMS\x01") file.
//!
//! Every field here is one that `sc2/src/uqm/save.c` writes and
//! `sc2/src/uqm/load.c` reads. Nothing in this module knows about the byte
//! layout; `encode` and `decode` own that.

use super::error::SaveNameError;
/// Number of bytes in the serialized story-state bitfield.
pub const NUM_GAME_STATE_BYTES: usize = 155;

/// Slots offered by the Starbase menu (`MAX_SAVED_GAMES`, gameopt.c:41).
pub const MAX_SAVED_GAMES: usize = 50;

/// `SAVE_NAME_SIZE` (save.h): 63 name bytes plus the terminating NUL.
pub const SAVE_NAME_SIZE: usize = 64;
/// `SIS_NAME_SIZE` (sis.h).
pub const SIS_NAME_SIZE: usize = 16;
/// `NUM_MODULE_SLOTS` (sis.h).
pub const NUM_MODULE_SLOTS: usize = 16;
/// `NUM_DRIVE_SLOTS` (sis.h).
pub const NUM_DRIVE_SLOTS: usize = 11;
/// `NUM_JET_SLOTS` (sis.h).
pub const NUM_JET_SLOTS: usize = 8;
/// `NUM_MODULES` (sis.h): the enum ends at `BOMB_MODULE_5 + 1`.
pub const NUM_MODULES: usize = 20;
/// `NUM_ELEMENT_CATEGORIES` (planets/elemdata.h).
pub const NUM_ELEMENT_CATEGORIES: usize = 8;
/// `MAX_BUILT_SHIPS` (sis.h).
pub const MAX_BUILT_SHIPS: usize = 12;
/// `MAX_EXCLUSIVE_DEVICES` (save.h).
pub const MAX_EXCLUSIVE_DEVICES: usize = 16;
/// `MAX_HYPER_SHIPS` (encount.h).
pub const MAX_HYPER_SHIPS: usize = 7;
/// Size of the serialized game-state bit array.
pub const GAME_STATE_BYTES: usize = NUM_GAME_STATE_BYTES;

/// Uncollected biological canister, keyed independently of surface entity IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CannedNode {
    pub star: u16,
    pub planet: u8,
    pub moon: u8,
    pub node: u8,
    pub value: u8,
    pub x: i16,
    pub y: i16,
}

impl CannedNode {
    pub fn key(self) -> (u16, u8, u8, u8) {
        (self.star, self.planet, self.moon, self.node)
    }
    pub fn is_valid(self) -> bool {
        self.star < 502
            && self.planet < 16
            && self.moon <= 4
            && self.node < 32
            && (1..=15).contains(&self.value)
            && (0..968).contains(&self.x)
            && (0..300).contains(&self.y)
    }
}

/// A signed 16-bit point (`POINT`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Point {
    pub x: i16,
    pub y: i16,
}

/// A signed 16-bit extent (`EXTENT`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Extent {
    pub width: i16,
    pub height: i16,
}

/// The name a player gave a save: at most 63 raw bytes, no NUL.
///
/// The bytes are stored as written. The game treats them as UTF-8 but the
/// loader never validates that, so neither does this type.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SaveName(Vec<u8>);

impl SaveName {
    /// The longest name that fits (`SAVE_NAME_SIZE - 1`).
    pub const MAX_BYTES: usize = SAVE_NAME_SIZE - 1;

    pub fn new(bytes: impl Into<Vec<u8>>) -> Result<Self, SaveNameError> {
        let bytes = bytes.into();
        if bytes.len() > Self::MAX_BYTES {
            return Err(SaveNameError::TooLong { len: bytes.len() });
        }
        if bytes.contains(&0) {
            return Err(SaveNameError::ContainsNul);
        }
        Ok(SaveName(bytes))
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// `SIS_STATE`, the part of the flagship state kept in the summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SisState {
    pub log_x: i32,
    pub log_y: i32,
    pub res_units: u32,
    pub fuel_on_board: u32,
    pub crew_enlisted: u16,
    pub total_element_mass: u16,
    pub total_bio_mass: u16,
    pub module_slots: [u8; NUM_MODULE_SLOTS],
    pub drive_slots: [u8; NUM_DRIVE_SLOTS],
    pub jet_slots: [u8; NUM_JET_SLOTS],
    pub num_landers: u8,
    pub element_amounts: [u16; NUM_ELEMENT_CATEGORIES],
    pub ship_name: [u8; SIS_NAME_SIZE],
    pub commander_name: [u8; SIS_NAME_SIZE],
    pub planet_name: [u8; SIS_NAME_SIZE],
}

/// `SUMMARY_DESC`: the "Summ" chunk, which is all the slot list needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    pub sis: SisState,
    pub activity: u8,
    pub flags: u8,
    pub day_index: u8,
    pub month_index: u8,
    pub year_index: u16,
    pub mcredit_lo: u8,
    pub mcredit_hi: u8,
    pub num_ships: u8,
    pub num_devices: u8,
    pub ship_list: [u8; MAX_BUILT_SHIPS],
    pub device_list: [u8; MAX_EXCLUSIVE_DEVICES],
    pub save_name: SaveName,
}

/// `CLOCK_STATE` fields that the "GlSt" chunk carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockState {
    pub day_index: u8,
    pub month_index: u8,
    pub year_index: u16,
    pub tick_count: i16,
    pub day_in_ticks: i16,
}

/// `VELOCITY_DESC`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Velocity {
    pub travel_angle: u16,
    pub vector: Extent,
    pub fract: Extent,
    pub error: Extent,
    pub incr: Extent,
}

/// The fixed 75-byte "GlSt" chunk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalState {
    pub glob_flags: u8,
    pub crew_cost: u8,
    pub fuel_cost: u8,
    pub module_cost: [u8; NUM_MODULES],
    pub element_worth: [u8; NUM_ELEMENT_CATEGORIES],
    pub current_activity: u16,
    pub clock: ClockState,
    pub autopilot: Point,
    pub ip_location: Point,
    pub ship_stamp_origin: Point,
    pub ship_facing: u16,
    pub ip_planet: u8,
    pub in_orbit: u8,
    pub velocity: Velocity,
}

/// One "RacQ" entry: `FLEET_INFO` plus its queue position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaceEntry {
    pub index: u16,
    pub allied_state: u16,
    pub days_left: u8,
    pub growth_fract: u8,
    pub crew_level: u16,
    pub max_crew: u16,
    pub growth: u8,
    pub max_energy: u8,
    pub loc: Point,
    pub actual_strength: u16,
    pub known_strength: u16,
    pub known_loc: Point,
    pub growth_err_term: u8,
    pub func_index: u8,
    pub dest_loc: Point,
}

/// One "IGpQ" entry: an `IP_GROUP` in the interplanetary group queue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpGroup {
    pub group_counter: u16,
    pub race_id: u8,
    pub sys_loc: u8,
    pub task: u8,
    pub in_system: u8,
    pub dest_loc: u8,
    pub orbit_pos: u8,
    pub group_id: u8,
    pub loc: Point,
}

/// One "ShpQ" or "NpcQ" entry: the race to clone plus a `SHIP_FRAGMENT`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShipFragmentEntry {
    pub race_index: u16,
    pub captains_name_index: u8,
    pub race_id: u8,
    pub index: u8,
    pub crew_level: u16,
    pub max_crew: u16,
    pub energy_level: u8,
    pub max_energy: u8,
}

/// One "Evts" entry: a scheduled game-clock event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventEntry {
    pub day_index: u8,
    pub month_index: u8,
    pub year_index: u16,
    pub func_index: u8,
}

/// `BRIEF_SHIP_INFO`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BriefShipInfo {
    pub race_id: u8,
    pub crew_level: u16,
    pub max_crew: u16,
    pub max_energy: u8,
}

/// One "Enct" entry: an `ENCOUNTER` (the black globes in HyperSpace).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncounterEntry {
    pub transition_state: i16,
    pub origin: Point,
    pub radius: u16,
    pub loc_pt: Point,
    pub race_id: u8,
    pub num_ships: u8,
    pub flags: u8,
    pub ship_list: [BriefShipInfo; MAX_HYPER_SHIPS],
    pub log_x: i32,
    pub log_y: i32,
}

/// The "Star" chunk: `STAR_DESC`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StarDesc {
    pub star_pt: Point,
    pub star_type: u8,
    pub index: u8,
    pub prefix: u8,
    pub postfix: u8,
}

/// One entry of the "Grps" chunk: the stored race byte and an `IP_GROUP`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupListEntry {
    pub race_outer: u8,
    pub group: IpGroup,
}

/// The "Grps" chunk: the random group list from RANDGRPINFO.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupList {
    pub last_encounter: u8,
    pub entries: Vec<GroupListEntry>,
}

/// One ship of a battle group: the stored race byte and a `SHIP_FRAGMENT`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattleShip {
    pub race_outer: u8,
    pub captains_name_index: u8,
    pub race_id: u8,
    pub index: u8,
    pub crew_level: u16,
    pub max_crew: u16,
    pub energy_level: u8,
    pub max_energy: u8,
}

/// One fleet inside a battle group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattleFleet {
    pub icon: u8,
    pub ships: Vec<BattleShip>,
}

/// A "BtGp" chunk. `encounter_id` 0 is the random group of RANDGRPINFO;
/// otherwise it is the 1-based index of the defined encounter group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattleGroup {
    pub encounter_id: u32,
    pub current: bool,
    pub star_index: u16,
    pub day_index: u8,
    pub month_index: u8,
    pub year_index: u16,
    pub fleets: Vec<BattleFleet>,
}

/// A whole save file.
///
/// Queue chunks the game does not write when the queue is empty are empty
/// vectors here, and chunks are written in the order `SaveGame` uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveFile {
    pub summary: Summary,
    pub global: GlobalState,
    /// The "GmSt" chunk, normalised the way `LoadGameState` does: zero
    /// padded when the file holds fewer bytes, cut when it holds more.
    pub game_state: [u8; GAME_STATE_BYTES],
    pub race_queue: Vec<RaceEntry>,
    pub ip_group_queue: Vec<IpGroup>,
    pub npc_ship_queue: Vec<ShipFragmentEntry>,
    pub ship_queue: Vec<ShipFragmentEntry>,
    pub events: Vec<EventEntry>,
    pub encounters: Vec<EncounterEntry>,
    /// The "Scan" chunk: STARINFO as 32-bit words. `Some(vec![])` is a
    /// present, empty chunk.
    pub scan: Option<Vec<u32>>,
    /// Optional "Cans" records. Absent in older saves.
    pub canned_nodes: Vec<CannedNode>,
    pub group_list: Option<GroupList>,
    pub battle_groups: Vec<BattleGroup>,
    pub star: StarDesc,
}
