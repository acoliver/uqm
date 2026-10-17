//! Game-facing effects of the Starbase and Outfit screens, and their typed
//! error.
//!
//! [`Platform`] is everything the screens need besides drawing: activity and
//! game-state words, the SIS state, pulsed menu keys, resources, music, the
//! game clock and the input loop. The game implements it over the existing
//! Rust and C-exported services; tests implement it with a recorder.

use super::fuel::{FuelError, GaugeDelta};
use super::gfx::{FrameId, Gfx};
use super::input::MenuPulses;
use super::module_chooser::ModuleChooserError;
use super::sis_modules::{FuelLayoutError, ModuleSlotsError};
use crate::mainloop::types::ActivityValue;

/// Opaque `MUSIC_REF`.
pub type MusicId = usize;

/// Opaque `COLORMAP` handle.
pub type ColorMapId = usize;

/// `FLEET_INFO.allied_state`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlliedState {
    DeadGuy,
    GoodGuy,
    BadGuy,
}

/// An `avail_race_q` entry: the template a built ship is cloned from. The
/// position in [`Platform::race_templates`] is the race number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RaceTemplate {
    pub allied: AlliedState,
    /// `FLEET_INFO.melee_icon`.
    pub melee_icon: FrameId,
    /// `FLEET_INFO.crew_level`: the crew a full ship of this race has.
    pub crew_level: u16,
}

/// A `SHIP_FRAGMENT` of `built_ship_q`: an escort ship.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Escort {
    /// `SHIP_FRAGMENT.index`: the hangar slot.
    pub slot: u8,
    /// `SHIP_FRAGMENT.race_id`: the `avail_race_q` position.
    pub race_id: u8,
    pub crew_level: u16,
    pub max_crew: u16,
    pub melee_icon: FrameId,
}

/// `ONE_SECOND` (`timelib.h`).
pub const ONE_SECOND: u32 = 840;

/// Game-state bits the screens read or write.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StateKey {
    StarbaseVisited,
    StarbaseAvailable,
    ProbeIlwrathEncounter,
    GlobalFlagsAndData,
    MoonbaseOnShip,
    ChmmrBombState,
    LanderShields,
    CrewPurchased0,
    CrewPurchased1,
}

/// A snapshot of the `GLOBAL_SIS` and `GLOBAL` fields the screens read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SisView {
    pub res_units: u32,
    pub fuel_on_board: u32,
    pub crew_enlisted: u16,
    pub total_element_mass: u16,
    pub module_slots: [u8; 16],
    pub drive_slots: [u8; 11],
    pub jet_slots: [u8; 8],
    pub num_landers: u8,
    /// `GLOBAL_SIS (ShipName)`.
    pub ship_name: String,
    /// `GLOBAL (FuelCost)`: RU per displayed fuel unit.
    pub fuel_cost: u8,
    /// `GLOBAL (ModuleCost)`.
    pub module_cost: [u8; 20],
}

/// The RU argument of `DeltaSISGauges (UNDEFINED_DELTA, UNDEFINED_DELTA,
/// resunit_delta)`, which redraws the whole ship status and leaves crew and
/// fuel as they are (`sis.c:1115-1161`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResUnitsUpdate {
    /// `UNDEFINED_DELTA`: redraw the RU gauge without changing it.
    Redraw,
    /// Add this many RU (negative pays), floored at zero, then redraw. The
    /// C `int` argument must not be `UNDEFINED_DELTA` (0x7FFF); module
    /// prices are at most 255 * 50 RU, so they never are.
    Change(i32),
}

/// `StatMsgMode` (`sis.h`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusMessageMode {
    Undefined,
    Date,
    ResUnits,
    Credits,
}

/// Which menu hierarchy and font set the player chose (`optWhichMenu`,
/// `optWhichFonts`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiStyle {
    ThreeDo,
    Pc,
}

/// `SetMenuSounds` masks, in the bit meanings of `sounds.h`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuSounds {
    pub move_mask: u16,
    pub select_mask: u16,
}

impl MenuSounds {
    const UP: u16 = 1 << 0;
    const DOWN: u16 = 1 << 1;
    const LEFT: u16 = 1 << 2;
    const RIGHT: u16 = 1 << 3;
    const SELECT: u16 = 1 << 4;
    const CANCEL: u16 = 1 << 5;
    const PAGE_UP: u16 = 1 << 7;
    const PAGE_DOWN: u16 = 1 << 8;

    /// `SetMenuSounds (MENU_SOUND_ARROWS, MENU_SOUND_SELECT)`.
    pub const ARROWS_SELECT: Self = Self {
        move_mask: Self::UP | Self::DOWN | Self::LEFT | Self::RIGHT,
        select_mask: Self::SELECT,
    };

    /// The fuel adjust sounds of `DoOutfit` (`outfit.c:775-778`).
    /// `SetMenuSounds (MENU_SOUND_UP | MENU_SOUND_DOWN, MENU_SOUND_SELECT |
    /// MENU_SOUND_CANCEL)` while the Shipyard edits crew.
    pub const EDIT_CREW: Self = Self {
        move_mask: Self::UP | Self::DOWN,
        select_mask: Self::SELECT | Self::CANCEL,
    };

    pub const FUEL_ADJUST: Self = Self {
        move_mask: Self::UP | Self::DOWN | Self::PAGE_UP | Self::PAGE_DOWN,
        select_mask: Self::SELECT | Self::CANCEL,
    };
}

/// A route this port cannot serve because the Rust game has no
/// implementation of it. Selecting it ends the game loop with
/// [`StarbaseError::Unported`] rather than faking success.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnportedRoute {
    /// `DoShipSpin` (`fmv.c`): the slide-show presentation of a ship, started
    /// by the SPECIAL key in the Shipyard. It needs `ShowPresentation`.
    ShipSpin,
    /// `GameOptions` Save/Load from the Outfit menu (`gameopt.c`).
    SaveLoad,
}

impl std::fmt::Display for UnportedRoute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            UnportedRoute::ShipSpin => "the Shipyard ship spin presentation",
            UnportedRoute::SaveLoad => "Save/Load from the Outfit menu",
        })
    }
}

/// Why a Starbase or Outfit screen could not continue.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StarbaseError {
    #[error("{0} is not ported to Rust")]
    Unported(UnportedRoute),
    #[error("fuel transaction: {0}")]
    Fuel(#[from] FuelError),
    #[error("ship module slots: {0}")]
    ModuleSlots(#[from] ModuleSlotsError),
    #[error("fuel gauge layout: {0}")]
    FuelLayout(#[from] FuelLayoutError),
    #[error("resource `{0}` did not load")]
    ResourceLoad(String),
    #[error(
        "gauge update disagrees with the transaction: expected fuel {expected_fuel} and {expected_res_units} RU, found fuel {actual_fuel} and {actual_res_units} RU"
    )]
    GaugeMismatch {
        expected_fuel: u32,
        expected_res_units: u32,
        actual_fuel: u32,
        actual_res_units: u32,
    },
    #[error("ship name is not valid UTF-8")]
    ShipName,
    #[error("module chooser: {0}")]
    ModuleChooser(#[from] ModuleChooserError),
    #[error(
        "RU gauge update disagrees with the transaction: expected {expected} RU, found {actual} RU"
    )]
    ResUnitsMismatch { expected: u32, actual: u32 },
    #[error("fleet queue reports allied state {0}")]
    UnknownAlliedState(u16),
    #[error("race {0} is not in the fleet queue")]
    UnknownRace(u8),
    #[error("no ship can be built: no race is allied")]
    NoShipsAvailable,
    #[error("hangar slot {0} holds no escort ship")]
    EscortSlot(u8),
    #[error("{crew} crew do not fit the crew pods")]
    CrewExceedsPods { crew: u16 },
    #[error("escort ship queue: {0}")]
    EscortQueue(&'static str),
    #[error(
        "crew update disagrees with the transaction: expected {expected_crew} crew and {expected_res_units} RU, found {actual_crew} crew and {actual_res_units} RU"
    )]
    CrewMismatch {
        expected_crew: u16,
        expected_res_units: u32,
        actual_crew: u16,
        actual_res_units: u32,
    },
}

/// Everything besides drawing that the screens do.
pub trait Platform: Gfx {
    // --- Activity words and game state ---
    fn current_activity(&self) -> ActivityValue;
    fn set_current_activity(&self, value: ActivityValue);
    fn last_activity(&self) -> ActivityValue;
    fn set_last_activity(&self, value: ActivityValue);
    fn set_next_activity(&self, value: ActivityValue);
    fn game_state(&self, key: StateKey) -> u32;
    fn set_game_state(&self, key: StateKey, value: u32);
    fn set_status_message_mode(&self, mode: StatusMessageMode) -> StatusMessageMode;

    // --- SIS state ---
    fn sis(&self) -> Result<SisView, StarbaseError>;
    /// Write `GLOBAL_SIS (FuelOnBoard)` directly (entry rounding).
    fn set_fuel_on_board(&self, fuel: u32);
    /// `DeltaSISGauges (0, delta.fuel, delta.res_units)`: change the SIS
    /// state and redraw the status gauges.
    fn apply_gauge_delta(&self, delta: GaugeDelta);
    /// `DeltaSISGauges (crew, 0, res_units)`: change the crew and RU, floored
    /// at zero, and redraw the status gauges.
    fn apply_crew_gauge_delta(&self, crew: i16, res_units: i32);
    /// `GLOBAL (CrewCost)`.
    fn crew_cost(&self) -> u8;
    fn set_crew_cost(&self, cost: u8);
    /// `GLOBAL_SIS (DriveSlots[index]) = piece`.
    fn set_drive_slot(&self, index: usize, piece: u8);
    /// `GLOBAL_SIS (JetSlots[index]) = piece`.
    fn set_jet_slot(&self, index: usize, piece: u8);
    /// `GLOBAL_SIS (ModuleSlots[index]) = piece`.
    fn set_module_slot(&self, index: usize, piece: u8);
    /// `GLOBAL_SIS (NumLanders) = count`.
    fn set_num_landers(&self, count: u8);
    /// `DeltaSISGauges (UNDEFINED_DELTA, UNDEFINED_DELTA, resunit_delta)`:
    /// redraw the ship status panel from the SIS state and apply the RU
    /// change.
    fn redraw_ship_status(&self, res_units: ResUnitsUpdate);

    // --- Fleet ---
    /// The `avail_race_q` entries in queue order.
    fn race_templates(&self) -> Result<Vec<RaceTemplate>, StarbaseError>;
    /// The `built_ship_q` entries in queue order.
    fn escorts(&self) -> Result<Vec<Escort>, StarbaseError>;
    /// `CloneShipFragment (race, &built_ship_q, 1)`, then place it in the
    /// hangar slot (`ShowCombatShip`). `Ok(false)` when the queue is full.
    fn add_escort(&self, race: u8, slot: u8) -> Result<bool, StarbaseError>;
    /// `SHIP_FRAGMENT.crew_level` of the escort in the slot.
    fn set_escort_crew(&self, slot: u8, crew_level: u16) -> Result<(), StarbaseError>;
    /// `RemoveQueue` and `FreeShipFrag` of the escort in the slot.
    fn remove_escort(&self, slot: u8) -> Result<(), StarbaseError>;

    // --- SIS frame ---
    fn draw_sis_frame(&self);
    fn draw_sis_message(&self, text: &str);
    fn draw_sis_title(&self, text: &str);

    // --- Input and options ---
    fn pulses(&self) -> MenuPulses;
    fn set_menu_sounds(&self, sounds: MenuSounds);
    fn play_failure_sound(&self);
    fn flush_input(&self);
    /// `AnyButtonPress (FALSE)`.
    fn any_button_press(&self) -> bool;
    fn menu_style(&self) -> UiStyle;
    fn font_style(&self) -> UiStyle;

    // --- Resources and music ---
    /// `CaptureDrawable (LoadGraphic (resource))`.
    fn load_graphic(&self, resource: &str) -> Result<FrameId, StarbaseError>;
    /// `DestroyDrawable (ReleaseDrawable (frame))`.
    fn destroy_drawable(&self, frame: FrameId);
    /// `CaptureColorMap (LoadColorMap (resource))`.
    fn load_color_map(&self, resource: &str) -> Result<ColorMapId, StarbaseError>;
    /// `DestroyColorMap (ReleaseColorMap (map))`.
    fn destroy_color_map(&self, map: ColorMapId);
    /// `SetAbsColorMapIndex`.
    fn abs_color_map(&self, map: ColorMapId, index: u16) -> ColorMapId;
    /// `SetRelColorMapIndex`.
    fn rel_color_map(&self, map: ColorMapId, offset: i16) -> ColorMapId;
    /// `SetColorMap (GetColorMapAddress (map))`.
    fn apply_color_map(&self, map: ColorMapId);
    fn load_music(&self, resource: &str) -> Result<MusicId, StarbaseError>;
    fn play_music(&self, music: MusicId);
    fn stop_music(&self);
    fn destroy_music(&self, music: MusicId);

    // --- Time ---
    fn now(&self) -> u32;
    fn sleep_thread(&self, ticks: u32);
    fn sleep_thread_until(&self, wake_time: u32);
    /// `FadeScreen (FadeAllToBlack, ticks)`; returns the wake time.
    fn fade_to_black(&self, ticks: u32) -> u32;
    fn advance_clock_days(&self, days: u32);

    // --- Observation for automation ---
    /// Tell the automation observer whether the Starbase menu or the Outfit
    /// screen is the active input loop.
    fn observe_starbase_menu(&self, active: bool);

    // --- Conversation and the input loop ---
    /// `InitCommunication (COMMANDER_CONVERSATION)`.
    fn run_commander_conversation(&self);
    /// `DoInput (state, TRUE)`: call `frame` once per input frame until it
    /// returns false.
    fn run_input_loop(&self, frame: &mut dyn FnMut() -> bool);
}
