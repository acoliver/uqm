//! Capture of the "Summ" section from the running game.
//!
//! This is `PrepareSummary` (save.c) reading the same owners the C code reads:
//!
//! * `GlobData.SIS_state`: the flagship fields and the three name strings,
//!   copied as they stand;
//! * the Rust game-state singleton: the Melnorme credit, the lander flags, the
//!   device list, the quasispace side and the "global flags and data" byte
//!   (`snapshot_game_state`, one lock, so every game-state field of the summary
//!   comes from the same snapshot);
//! * `GLOBAL (GameClock)`: the date;
//! * `GLOBAL (built_ship_q)`: the escorts' races.
//!
//! The activity of a summary is the low byte of `CurrentActivity`, refined the
//! way `PrepareSummary` does. Two of its branches read the solar-system
//! owner, which is still C (`GetPlanetOrMoonName` and `playerInPlanetOrbit`
//! depend on `pSolarSysState`) and, for the final battle, the game string
//! table. They reach this module through [`SolarSystemOwner`]; `live` supplies
//! the C-backed implementation, and nothing here guesses a name when that owner
//! cannot answer.
//!
//! The device list is `InventoryDevices` (planets/devices.c) over the same
//! game-state bits. The save name is cut the way `strncpy (SaveName, name,
//! SAVE_NAME_SIZE-1)` cuts it: at the first NUL or at 63 bytes.
//!
//! Where this differs from the C code on purpose: the unused tail of
//! `ShipList` and `DeviceList` is zero, where C writes whatever the caller's
//! `SUMMARY_DESC` held, and more escorts than a summary lists is an error where
//! C would write past `ShipList`.

use std::fmt;

use super::queues::{walk, QueueError, QueueKind};
use crate::comm::dispatch::{CShipFragment, IN_HYPERSPACE, IN_INTERPLANETARY, IN_LAST_BATTLE};
use crate::comm::locdata::{CGameState, CSisState};
use crate::savegame::types::{
    SaveName, SisState, Summary, MAX_BUILT_SHIPS, MAX_EXCLUSIVE_DEVICES, SIS_NAME_SIZE,
};
use crate::state::ffi::{snapshot_game_state, GameStateOwnerError};
use crate::state::game_state::GameState;

/// `IN_QUASISPACE` (globdata.h): only ever stored in a summary.
pub const IN_QUASISPACE: u8 = 6;
/// `IN_PLANET_ORBIT` (globdata.h): only ever stored in a summary.
pub const IN_PLANET_ORBIT: u8 = 7;
/// `IN_STARBASE` (globdata.h): only ever stored in a summary.
pub const IN_STARBASE: u8 = 8;

/// `GET_GAME_STATE (GLOBAL_FLAGS_AND_DATA) == (BYTE)~0` marks a starbase visit.
const IN_STARBASE_FLAGS: u8 = u8::MAX;
/// `GAME_STRING (PLANET_NUMBER_BASE + 32)`: Sa-Matra's name.
pub const SA_MATRA_STRING: u16 = 407;

/// `NUM_DEVICES` (globdata.h).
pub const NUM_DEVICES: usize = 28;

/// How a device shows in `InventoryDevices`.
#[derive(Clone, Copy)]
enum OnBoard {
    /// The device is aboard when this one-bit state is set.
    Flag(&'static str),
    /// The device is aboard when `ULTRON_CONDITION` holds this value.
    Ultron(u8),
    /// `InventoryDevices` never lists it (the data plates, and the destruct
    /// code whose test is compiled out under `#ifdef NEVER`).
    Never,
}

/// The `InventoryDevices` switch, indexed by the device enum of globdata.h.
const DEVICES: [OnBoard; NUM_DEVICES] = [
    OnBoard::Flag("PORTAL_SPAWNER_ON_SHIP"), // PORTAL_SPAWNER_DEVICE
    OnBoard::Flag("TALKING_PET_ON_SHIP"),    // TALKING_PET_DEVICE
    OnBoard::Flag("UTWIG_BOMB_ON_SHIP"),     // UTWIG_BOMB_DEVICE
    OnBoard::Flag("SUN_DEVICE_ON_SHIP"),     // SUN_EFFICIENCY_DEVICE
    OnBoard::Flag("ROSY_SPHERE_ON_SHIP"),    // ROSY_SPHERE_DEVICE
    OnBoard::Flag("AQUA_HELIX_ON_SHIP"),     // AQUA_HELIX_DEVICE
    OnBoard::Flag("CLEAR_SPINDLE_ON_SHIP"),  // CLEAR_SPINDLE_DEVICE
    OnBoard::Ultron(1),                      // ULTRON_0_DEVICE
    OnBoard::Ultron(2),                      // ULTRON_1_DEVICE
    OnBoard::Ultron(3),                      // ULTRON_2_DEVICE
    OnBoard::Ultron(4),                      // ULTRON_3_DEVICE
    OnBoard::Flag("MAIDENS_ON_SHIP"),        // MAIDENS_DEVICE
    OnBoard::Flag("UMGAH_BROADCASTERS_ON_SHIP"), // UMGAH_HYPERWAVE_DEVICE
    OnBoard::Flag("BURV_BROADCASTERS_ON_SHIP"), // BURVIX_HYPERWAVE_DEVICE
    OnBoard::Never,                          // DATA_PLATE_1_DEVICE
    OnBoard::Never,                          // DATA_PLATE_2_DEVICE
    OnBoard::Never,                          // DATA_PLATE_3_DEVICE
    OnBoard::Flag("TAALO_PROTECTOR_ON_SHIP"), // TAALO_PROTECTOR_DEVICE
    OnBoard::Flag("EGG_CASE0_ON_SHIP"),      // EGG_CASING0_DEVICE
    OnBoard::Flag("EGG_CASE1_ON_SHIP"),      // EGG_CASING1_DEVICE
    OnBoard::Flag("EGG_CASE2_ON_SHIP"),      // EGG_CASING2_DEVICE
    OnBoard::Flag("SYREEN_SHUTTLE_ON_SHIP"), // SYREEN_SHUTTLE_DEVICE
    OnBoard::Flag("VUX_BEAST_ON_SHIP"),      // VUX_BEAST_DEVICE
    OnBoard::Never,                          // DESTRUCT_CODE_DEVICE
    OnBoard::Flag("PORTAL_KEY_ON_SHIP"),     // URQUAN_WARP_DEVICE
    OnBoard::Flag("ARTIFACT_2_ON_SHIP"),     // ARTIFACT_2_DEVICE
    OnBoard::Flag("ARTIFACT_3_ON_SHIP"),     // ARTIFACT_3_DEVICE
    OnBoard::Flag("MOONBASE_ON_SHIP"),       // LUNAR_BASE_DEVICE
];

/// What the solar-system and string owners, still in C, say about the game.
///
/// Each method is one call `PrepareSummary` makes. An implementation reads the
/// live owner; it does not invent an answer when the owner has none.
pub trait SolarSystemOwner {
    /// `GetPlanetOrMoonName` into a `PlanetName`-sized buffer.
    fn planet_or_moon_name(&self) -> [u8; SIS_NAME_SIZE];
    /// `playerInPlanetOrbit ()`.
    fn player_in_planet_orbit(&self) -> bool;
    /// `GAME_STRING (PLANET_NUMBER_BASE + 32)` copied into a `PlanetName`
    /// buffer.
    fn last_battle_planet_name(&self) -> [u8; SIS_NAME_SIZE];
}

/// Why a summary could not be captured.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SummaryError {
    GameState(GameStateOwnerError),
    /// The built-ship queue is corrupt.
    Queue(QueueError),
    /// The built-ship queue holds more escorts than `ShipList` has places.
    TooManyShips {
        count: usize,
    },
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

impl From<QueueError> for SummaryError {
    fn from(error: QueueError) -> Self {
        Self::Queue(error)
    }
}

/// `GET_GAME_STATE (key)` on a snapshot of the singleton. An unknown key is a
/// bug in this module's tables, not a runtime condition.
pub(super) fn state_value(state: &GameState, key: &str) -> u8 {
    let (start, end) = GameState::lookup_bits(key)
        .unwrap_or_else(|| panic!("game-state key {key} is not in the state table"));
    state.get_state(start, end)
}

/// `InventoryDevices (list, MAX_EXCLUSIVE_DEVICES)`.
fn inventory_devices(state: &GameState) -> Vec<u8> {
    let mut devices = Vec::new();
    for (id, on_board) in DEVICES.iter().enumerate() {
        if devices.len() == MAX_EXCLUSIVE_DEVICES {
            break;
        }
        let aboard = match *on_board {
            OnBoard::Flag(key) => state_value(state, key) != 0,
            OnBoard::Ultron(value) => state_value(state, "ULTRON_CONDITION") == value,
            OnBoard::Never => false,
        };
        if aboard {
            devices.push(id as u8);
        }
    }
    devices
}

/// The flagship state as `SummPtr->SS = GlobData.SIS_state` copies it.
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
/// `sis` and `game` are `GlobData.SIS_state` and `GlobData.Game_state`;
/// `solar` is the solar-system owner; `name` is what the player typed.
pub fn capture_summary(
    sis: &CSisState,
    game: &CGameState,
    solar: &dyn SolarSystemOwner,
    name: &[u8],
) -> Result<Summary, SummaryError> {
    let state = GameState::from_bytes(&snapshot_game_state()?);

    let mut sis_copy = sis_state(sis);
    let mut activity = (game.current_activity & 0x00ff) as u8;
    match u16::from(activity) {
        IN_HYPERSPACE => {
            if state_value(&state, "ARILOU_SPACE_SIDE") > 1 {
                activity = IN_QUASISPACE;
            }
        }
        IN_INTERPLANETARY => {
            sis_copy.planet_name = solar.planet_or_moon_name();
            if state_value(&state, "GLOBAL_FLAGS_AND_DATA") == IN_STARBASE_FLAGS {
                activity = IN_STARBASE;
            } else if solar.player_in_planet_orbit() {
                activity = IN_PLANET_ORBIT;
            }
        }
        IN_LAST_BATTLE => sis_copy.planet_name = solar.last_battle_planet_name(),
        _ => {}
    }

    let escorts = walk::<CShipFragment>(&game.built_ship_q, QueueKind::BuiltShips)?;
    if escorts.len() > MAX_BUILT_SHIPS {
        return Err(SummaryError::TooManyShips {
            count: escorts.len(),
        });
    }
    let mut ship_list = [0u8; MAX_BUILT_SHIPS];
    for (slot, &link) in ship_list.iter_mut().zip(&escorts) {
        // SAFETY: `walk` returned slots of the pool, which holds `SHIP_FRAGMENT`s.
        *slot = unsafe { (*link).race_id };
    }

    let devices = inventory_devices(&state);
    let mut device_list = [0u8; MAX_EXCLUSIVE_DEVICES];
    device_list[..devices.len()].copy_from_slice(&devices);

    let flags = state_value(&state, "LANDER_SHIELDS")
        | (state_value(&state, "IMPROVED_LANDER_SPEED") << 4)
        | (state_value(&state, "IMPROVED_LANDER_CARGO") << 5)
        | (state_value(&state, "IMPROVED_LANDER_SHOT") << 6)
        | (u8::from(state_value(&state, "CHMMR_BOMB_STATE") >= 2) << 7);

    Ok(Summary {
        sis: sis_copy,
        activity,
        flags,
        day_index: game.game_clock.day_index,
        month_index: game.game_clock.month_index,
        year_index: game.game_clock.year_index,
        mcredit_lo: state_value(&state, "MELNORME_CREDIT0"),
        mcredit_hi: state_value(&state, "MELNORME_CREDIT1"),
        // At most `MAX_BUILT_SHIPS` and `MAX_EXCLUSIVE_DEVICES`: checked above.
        num_ships: escorts.len() as u8,
        num_devices: devices.len() as u8,
        ship_list,
        device_list,
        save_name: save_name(name),
    })
}
