//! Capture of the "Summ" section from the running game.
//!
//! This is `PrepareSummary` (save.c) reading the same owners the C code reads:
//!
//! * `GlobData.flagship`: the flagship fields and the three name strings,
//!   copied as they stand;
//! * the borrowed story state: the Melnorme credit, lander flags, device list,
//!   quasispace side and the "global flags and data" byte;
//! * `GLOBAL (GameClock)`: the date;
//! * `GLOBAL (built_ship_q)`: the escorts' races.
//!
//! The activity of a summary is the low byte of `CurrentActivity`, refined the
//! way `PrepareSummary` does. Solar names and the orbit status are prepared
//! at the active mode boundary and supplied as [`SessionSummary`]. The
//! engine derives the summary from those values and the supplied owners.
//!
//! The device list is `InventoryDevices` (planets/devices.c) over the same
//! game-state bits. The save name is cut the way `strncpy (SaveName, name,
//! SAVE_NAME_SIZE-1)` cuts it: at the first NUL or at 63 bytes.
//!
//! Where this differs from the C code on purpose: the unused tail of
//! `ShipList` and `DeviceList` is zero, where C writes whatever the caller's
//! `SUMMARY_DESC` held. The fixed escort pool cannot exceed `ShipList`.

use super::super::battle::link_queue::LinkQueue;
use super::super::world::queues::Fragment;
use std::fmt;

use super::super::battle::schema::GAME_STATE as CGameState;
use super::super::world::story::GameState;
use super::super::world::story::GameStateOwnerError;
use super::super::world::SisState as CSisState;
use super::super::StoryKey;
use super::activity::{IN_HYPERSPACE, IN_INTERPLANETARY, IN_LAST_BATTLE};
use uqm_save::{
    SaveName, SisState, Summary, MAX_BUILT_SHIPS, MAX_EXCLUSIVE_DEVICES, SIS_NAME_SIZE,
};

/// Why a solar-system question could not be answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolarError {
    /// The flagship is in an inner system but `pOrbitalDesc` is NULL, which
    /// C asserts against.
    OrbitalWorldMissing,
    /// `pOrbitalDesc` is not an element of `PlanetDesc` or `MoonDesc`, or
    /// does not sit on an element boundary.
    WorldNotInSystem,
    /// An entry of `PlanetDesc` does not orbit the sun, or an entry of
    /// `MoonDesc` orbits the sun or something that is not in `PlanetDesc`.
    WorldDisagreesWithItsTable,
}

impl fmt::Display for SolarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for SolarError {}

/// `IN_QUASISPACE` (globdata.h): only ever stored in a summary.
pub const IN_QUASISPACE: u8 = 6;
/// `IN_PLANET_ORBIT` (globdata.h): only ever stored in a summary.
pub const IN_PLANET_ORBIT: u8 = 7;
/// `IN_STARBASE` (globdata.h): only ever stored in a summary.
pub const IN_STARBASE: u8 = 8;

/// `GET_GAME_STATE (GLOBAL_FLAGS_AND_DATA) == (BYTE)~0` marks a starbase visit.
const IN_STARBASE_FLAGS: u8 = u8::MAX;
pub const PLANET_NUMBER_BASE: u32 = 375;
pub const STARBASE_STRING_BASE: u32 = 422;
pub const SOL_DEFINED: u32 = 1;
pub const SAMATRA_DEFINED: u32 = 29;
/// `GAME_STRING (PLANET_NUMBER_BASE + 32)`: Sa-Matra's name.
pub const SA_MATRA_STRING: u16 = PLANET_NUMBER_BASE as u16 + 32;

/// `NUM_DEVICES` (globdata.h).
pub const NUM_DEVICES: usize = 28;

/// How a device shows in `InventoryDevices`.
#[derive(Clone, Copy)]
enum OnBoard {
    /// The device is aboard when this one-bit state is set.
    Flag(StoryKey),
    /// The device is aboard when `ULTRON_CONDITION` holds this value.
    Ultron(u8),
    /// `InventoryDevices` never lists it (the data plates, and the destruct
    /// code whose test is compiled out under `#ifdef NEVER`).
    Never,
}

/// The `InventoryDevices` switch, indexed by the device enum of globdata.h.
const DEVICES: [OnBoard; NUM_DEVICES] = [
    OnBoard::Flag(StoryKey::PORTAL_SPAWNER_ON_SHIP), // PORTAL_SPAWNER_DEVICE
    OnBoard::Flag(StoryKey::TALKING_PET_ON_SHIP),    // TALKING_PET_DEVICE
    OnBoard::Flag(StoryKey::UTWIG_BOMB_ON_SHIP),     // UTWIG_BOMB_DEVICE
    OnBoard::Flag(StoryKey::SUN_DEVICE_ON_SHIP),     // SUN_EFFICIENCY_DEVICE
    OnBoard::Flag(StoryKey::ROSY_SPHERE_ON_SHIP),    // ROSY_SPHERE_DEVICE
    OnBoard::Flag(StoryKey::AQUA_HELIX_ON_SHIP),     // AQUA_HELIX_DEVICE
    OnBoard::Flag(StoryKey::CLEAR_SPINDLE_ON_SHIP),  // CLEAR_SPINDLE_DEVICE
    OnBoard::Ultron(1),                              // ULTRON_0_DEVICE
    OnBoard::Ultron(2),                              // ULTRON_1_DEVICE
    OnBoard::Ultron(3),                              // ULTRON_2_DEVICE
    OnBoard::Ultron(4),                              // ULTRON_3_DEVICE
    OnBoard::Flag(StoryKey::MAIDENS_ON_SHIP),        // MAIDENS_DEVICE
    OnBoard::Flag(StoryKey::UMGAH_BROADCASTERS_ON_SHIP), // UMGAH_HYPERWAVE_DEVICE
    OnBoard::Flag(StoryKey::BURV_BROADCASTERS_ON_SHIP), // BURVIX_HYPERWAVE_DEVICE
    OnBoard::Never,                                  // DATA_PLATE_1_DEVICE
    OnBoard::Never,                                  // DATA_PLATE_2_DEVICE
    OnBoard::Never,                                  // DATA_PLATE_3_DEVICE
    OnBoard::Flag(StoryKey::TAALO_PROTECTOR_ON_SHIP), // TAALO_PROTECTOR_DEVICE
    OnBoard::Flag(StoryKey::EGG_CASE0_ON_SHIP),      // EGG_CASING0_DEVICE
    OnBoard::Flag(StoryKey::EGG_CASE1_ON_SHIP),      // EGG_CASING1_DEVICE
    OnBoard::Flag(StoryKey::EGG_CASE2_ON_SHIP),      // EGG_CASING2_DEVICE
    OnBoard::Flag(StoryKey::SYREEN_SHUTTLE_ON_SHIP), // SYREEN_SHUTTLE_DEVICE
    OnBoard::Flag(StoryKey::VUX_BEAST_ON_SHIP),      // VUX_BEAST_DEVICE
    OnBoard::Never,                                  // DESTRUCT_CODE_DEVICE
    OnBoard::Flag(StoryKey::PORTAL_KEY_ON_SHIP),     // URQUAN_WARP_DEVICE
    OnBoard::Flag(StoryKey::ARTIFACT_2_ON_SHIP),     // ARTIFACT_2_DEVICE
    OnBoard::Flag(StoryKey::ARTIFACT_3_ON_SHIP),     // ARTIFACT_3_DEVICE
    OnBoard::Flag(StoryKey::MOONBASE_ON_SHIP),       // LUNAR_BASE_DEVICE
];

/// Summary fields prepared from the active mode's solar state.
#[derive(Clone, Copy)]
pub enum SessionSummary {
    None,
    Planet {
        name: [u8; SIS_NAME_SIZE],
        in_orbit: bool,
    },
    FinalBattle {
        name: [u8; SIS_NAME_SIZE],
    },
}

/// Why a summary could not be captured.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SummaryError {
    GameState(GameStateOwnerError),
    /// The solar-system state is not the shape `ExploreSolarSys` leaves.
    Solar(SolarError),
}

impl fmt::Display for SummaryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for SummaryError {}

impl From<GameStateOwnerError> for SummaryError {
    fn from(error: GameStateOwnerError) -> Self {
        Self::GameState(error)
    }
}

impl From<SolarError> for SummaryError {
    fn from(error: SolarError) -> Self {
        Self::Solar(error)
    }
}

fn state_value(state: &GameState, key: StoryKey) -> u8 {
    state.get_state(key.start(), key.end())
}

/// `InventoryDevices (list, MAX_EXCLUSIVE_DEVICES)`.
pub fn inventory_devices(state: &GameState, limit: usize) -> Vec<u8> {
    let mut devices = Vec::new();
    for (id, on_board) in DEVICES.iter().enumerate() {
        if devices.len() == limit {
            break;
        }
        let aboard = match *on_board {
            OnBoard::Flag(key) => state_value(state, key) != 0,
            OnBoard::Ultron(value) => state_value(state, StoryKey::ULTRON_CONDITION) == value,
            OnBoard::Never => false,
        };
        if aboard {
            devices.push(id as u8);
        }
    }
    devices
}

/// The flagship state as `SummPtr->SS = GlobData.flagship` copies it.
fn sis_state(sis: &CSisState) -> SisState {
    SisState {
        log_x: sis.log_x,
        log_y: sis.log_y,
        res_units: sis.res_units,
        fuel_on_board: sis.fuel_on_board,
        crew_enlisted: sis.crew_enlisted,
        total_element_mass: sis.total_element_mass,
        total_bio_mass: sis.total_bio_mass,
        module_slots: sis.module_slots,
        drive_slots: sis.drive_slots,
        jet_slots: sis.jet_slots,
        num_landers: sis.num_landers,
        element_amounts: sis.element_amounts,
        ship_name: sis.ship_name,
        commander_name: sis.commander_name,
        planet_name: sis.planet_name,
    }
}

/// `strncpy (SaveName, name, SAVE_NAME_SIZE-1)`.
fn save_name(name: &[u8]) -> SaveName {
    let end = name
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(name.len())
        .min(SaveName::MAX_BYTES);
    SaveName::new(&name[..end]).expect("a name cut at the first NUL and 63 bytes is valid")
}

/// Port of the `strncpy` plus terminator that `utf8StringCopy` is, into a
/// `PlanetName`-sized buffer.
pub fn utf8_string_copy(source: &[u8]) -> [u8; SIS_NAME_SIZE] {
    let mut out = [0u8; SIS_NAME_SIZE];
    for (slot, &byte) in out
        .iter_mut()
        .zip(source.iter().take_while(|&&byte| byte != 0))
        .take(SIS_NAME_SIZE - 1)
    {
        *slot = byte;
    }
    out
}

/// `PrepareSummary`, reading the live owners.
///
/// `sis` and `game` are `GlobData.flagship` and `GlobData.game`;
/// `solar` is the solar-system owner; `name` is what the player typed.
pub fn capture_summary(
    activity: u16,
    sis: &CSisState,
    game: &CGameState,
    escorts: LinkQueue<'_, Fragment, 12>,
    story: &Option<GameState>,
    solar: SessionSummary,
    name: &[u8],
) -> Result<Summary, SummaryError> {
    let state = story.as_ref().ok_or(GameStateOwnerError::NotInitialized)?;

    let mut sis_copy = sis_state(sis);
    let mut activity = (activity & 0x00ff) as u8;
    match u16::from(activity) {
        IN_HYPERSPACE => {
            if state_value(state, StoryKey::ARILOU_SPACE_SIDE) > 1 {
                activity = IN_QUASISPACE;
            }
        }
        IN_INTERPLANETARY => {
            let SessionSummary::Planet { name, in_orbit } = solar else {
                panic!("interplanetary capture requires prepared solar summary");
            };
            sis_copy.planet_name = name;
            if state_value(state, StoryKey::GLOBAL_FLAGS_AND_DATA) == IN_STARBASE_FLAGS {
                activity = IN_STARBASE;
            } else if in_orbit {
                activity = IN_PLANET_ORBIT;
            }
        }
        IN_LAST_BATTLE => {
            let SessionSummary::FinalBattle { name } = solar else {
                panic!("final-battle capture requires its prepared name");
            };
            sis_copy.planet_name = name;
        }
        _ => {}
    }

    let mut ship_list = [0u8; MAX_BUILT_SHIPS];
    let mut next = escorts.head();
    for slot in ship_list.iter_mut().take(escorts.count()) {
        let id = next.unwrap_or_else(|| panic!("escort count matches visit order"));
        *slot = escorts.get(id).race_id;
        next = escorts.succ(id);
    }

    let devices = inventory_devices(state, MAX_EXCLUSIVE_DEVICES);
    let mut device_list = [0u8; MAX_EXCLUSIVE_DEVICES];
    device_list[..devices.len()].copy_from_slice(&devices);

    let flags = state_value(state, StoryKey::LANDER_SHIELDS)
        | (state_value(state, StoryKey::IMPROVED_LANDER_SPEED) << 4)
        | (state_value(state, StoryKey::IMPROVED_LANDER_CARGO) << 5)
        | (state_value(state, StoryKey::IMPROVED_LANDER_SHOT) << 6)
        | (u8::from(state_value(state, StoryKey::CHMMR_BOMB_STATE) >= 2) << 7);

    Ok(Summary {
        sis: sis_copy,
        activity,
        flags,
        day_index: game.game_clock.day_index,
        month_index: game.game_clock.month_index,
        year_index: game.game_clock.year_index,
        mcredit_lo: state_value(state, StoryKey::MELNORME_CREDIT0),
        mcredit_hi: state_value(state, StoryKey::MELNORME_CREDIT1),
        // Both lists are bounded by their fixed storage capacities.
        num_ships: escorts.count() as u8,
        num_devices: devices.len() as u8,
        ship_list,
        device_list,
        save_name: save_name(name),
    })
}
