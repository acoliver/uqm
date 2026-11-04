//! The C functions and globals the default system generator still needs, in
//! one place. Each goes away when its owner is ported.

use crate::engine::World as WorldOwner;
mod npcs;
pub use npcs::NpcWorld;
mod surface;
pub use surface::{load_planet_surface, CapturedSurface};

use std::ffi::c_void;

use crate::c_bindings::global_state::{
    planetOuterLocation, EncounterGroup as ENCOUNTER_GROUP, EncounterRace as ENCOUNTER_RACE,
    XFormIPLoc, IP_GROUP,
};
use crate::comm::dispatch::{ship, CPoint, ONE_SECOND};
use crate::mainloop::restart_menu::c_extern::{
    uqm_sleep_thread_until, FadeScreen, FADE_ALL_TO_BLACK,
};
use crate::planets::generate::default::{Npcs, PlanetNames};
use crate::planets::generate::special::FleetGroups;
use crate::planets::universe_units::{universe_to_logx, universe_to_logy};
use crate::state::ffi::rust_set_game_state_bits;

#[cfg(not(test))]
use crate::ui::sis::repair_sisborder;
#[cfg(test)]
extern "C" {
    fn repair_sisborder();
}
#[cfg(all(not(test), feature = "debug-process"))]
pub use crate::front_end::uqmdebug::DISABLE_INTERACTIVITY;
use crate::strings::unicode::utf8_string_copy as utf8StringCopy;

pub use crate::log_abi::log_init;

use crate::navigation::groups::live::get_group_info;

use crate::navigation::groups::live as group_storage;

pub use crate::navigation::groups::live::init_group_info;

pub use crate::navigation::groups::live::uninit_group_info;

use crate::navigation::groups::live::build_groups;

use crate::planets::orbit::live::LoadPlanet;

pub use crate::planets::orbit::live::FreePlanet;

pub use crate::planets::plangen::live::GeneratePlanetSurface;

use crate::planets::orbit::live::LoadStdLanderFont;

use crate::planets::orbit::live::FreeLanderFont;

use crate::strings::string_table::GetStringTableCount;

use crate::strings::string_table::SetRelStringTableIndex;

use crate::strings::string_table::ReleaseStringTable;

use crate::strings::string_table::DestroyStringTable;

use crate::graphics::resgfx::LoadGraphicInstance;

pub use crate::strings::sresins::LoadStringTableInstance;

use crate::graphics::pixmap_abi::CaptureDrawable;

use crate::graphics::pixmap_abi::ReleaseDrawable;

use crate::graphics::drawable_abi::DestroyDrawable;

use crate::graphics::drawable_abi::RescaleFrame;

use crate::graphics::tfb_draw_abi::TFB_DrawImage_New;

use crate::graphics::native_canvas::scaling::TFB_DrawCanvas_Rescale_Nearest;

pub use crate::graphics::drawable_abi::ReadFramePixelIndexes;

use crate::graphics::pixmap_abi::GetFrameCount;

use crate::graphics::pixmap_abi::SetAbsFrameIndex;

use crate::graphics::drawable_abi::CreateDrawable;

pub use crate::graphics::dcqueue_abi::Init_DrawCommandQueue;

pub use crate::graphics::dcqueue_abi::Uninit_DrawCommandQueue;

pub use crate::graphics::dcqueue_abi::TFB_FlushGraphicsEx;

pub use crate::strings::string_table::CaptureStringTable;

use crate::strings::string_table::SetAbsStringTableIndex;

use crate::navigation::interplanetary::live::notify_others_abi as NotifyOthers;

use crate::ui::sis::DeltaSISGauges;

use crate::ui::sis::DrawSISMessage;

use crate::ui::sis::DrawHyperCoords;

extern "C" {

    #[link_name = "format_conv_surf"]
    pub static mut FORMAT_CONV_SURF: *mut crate::graphics::ffi::SDL_Surface;
    #[link_name = "SDL_Screen"]
    pub static mut SDL_SCREEN: *mut crate::graphics::ffi::SDL_Surface;
    #[link_name = "star_array"]
    pub static mut STAR_ARRAY: *mut crate::savegame::runtime::star::CStarDesc;

}

/// `GROUP_INIT_IP`, `GROUP_SAVE_IP` (`state.h`): `(BYTE)~0`.
const GROUP_INIT_IP: u8 = !0;
const GROUP_SAVE_IP: u8 = !0;
/// `GROUP_LOAD_IP` is `GROUP_LIST` (`state.h`).
const GROUP_LOAD_IP: u8 = 0;

/// Clear the guard encounter so the interplanetary groups do not chase.
///
/// # Safety
/// Game thread; the encounter globals belong to the current visit.
pub unsafe fn reset_encounter() {
    std::ptr::addr_of_mut!(ENCOUNTER_GROUP).write(0);
    std::ptr::addr_of_mut!(ENCOUNTER_RACE).write(-1);
}

/// The group selected by a starbase encounter.
///
/// # Safety
/// Game thread, with no references to the encounter global outstanding.
pub unsafe fn set_encounter_group(value: u8) {
    std::ptr::addr_of_mut!(ENCOUNTER_GROUP).write(value);
}

/// The remaining legacy escort queue's feasibility check for rebel Yehat.
///
/// # Safety
/// Game thread, with the available-race and escort queues initialised.
pub unsafe fn rebel_escort_feasible() -> bool {
    WorldOwner::current().read(|world| {
        crate::battle::build_native::escort_feasibility(world, u16::from(ship::YEHAT_REBEL_SHIP))
            != 0
    })
}

/// The beast report has no mask, but still loads the lander font.
///
/// # Safety
/// `system` is the live system, with no outstanding references into it.
pub unsafe fn load_beast_scene(system: *mut crate::c_bindings::global_state::SOLARSYS_STATE) {
    LoadStdLanderFont(
        (std::ptr::addr_of_mut!((*system).SysInfo.PlanetInfo).cast::<c_void>()).cast(),
    );
    (*system).PlanetSideFrame[1] = std::ptr::null_mut();
    (*system).SysInfo.PlanetInfo.DiscoveryString =
        CaptureStringTable(LoadStringTableInstance(c"text.vuxbeast".as_ptr())).cast();
}

/// `GROUPS_RANDOM`.
const GROUPS_RANDOM: u32 = 0;

/// `LoadPlanet (NULL)`.
///
/// # Safety
/// Game thread, inside a planet orbit.
pub unsafe fn load_planet_default_surface() {
    LoadPlanet(std::ptr::null_mut());
}

/// `CurStarDescPtr->Index`.
///
/// # Safety
/// `CurStarDescPtr` must point at a valid star description.
pub unsafe fn current_star_index() -> u8 {
    // SAFETY: the caller retains a selected entry of the initialized catalog on the game thread.
    unsafe {
        (*(**WorldOwner::current()).star_pointer(crate::planets::starmap::current_catalog())).index
    }
}

/// `GROUPS_ADD_NEW` (`state.h`).
const GROUPS_ADD_NEW: u32 = !0;
/// `SYREEN_CONVERSATION` and `SLYLANDRO_HOME_CONVERSATION` (`commglue.h`).
pub const SYREEN_CONVERSATION: u32 = 20;
pub const SLYLANDRO_HOME_CONVERSATION: u32 = 23;

/// `InitCommunication (which_comm)`.
///
/// # Safety
/// Game thread, inside a planet orbit.
pub unsafe fn init_communication(which_comm: u32) {
    crate::app::bridge::enter(|cx| {
        let mut observer = cx.kernel.input_runtime().observer();
        let mut io = crate::engine::observe::Io {
            observer: &mut *observer,
        };
        crate::comm::dispatch::init_communication(cx, &mut io, which_comm);
    });
}

/// `YEHAT_CONVERSATION` (`commglue.h`) and `YEHAT_SHIP` (`races.h`).
pub const YEHAT_CONVERSATION: u32 = 11;
pub const YEHAT_SHIP: u16 = 11;
/// `IPNL_ALL_CLEAR` (`ipdisp.h`): `(BYTE)-1`.
const IPNL_ALL_CLEAR: u8 = !0;
/// `INFINITE_FLEET` (`races.h`): `(COUNT)~0`.
const INFINITE_FLEET: u16 = !0;

/// `StartSphereTracking (race)` as the truth value the generators test.
///
/// # Safety
/// Game thread, with the fleet queue initialised.
pub unsafe fn start_sphere_tracking(race: u16) -> bool {
    WorldOwner::current()
        .write(|world| crate::battle::build_native::start_sphere_tracking(world, race))
        != 0
}

/// `RepairSISBorder ()`.
///
/// # Safety
/// Game thread, inside a planet orbit.
pub unsafe fn repair_sis_border() {
    repair_sisborder();
}

/// `NotifyOthers (race, IPNL_ALL_CLEAR)`.
///
/// # Safety
/// Game thread, inside a planet orbit.
pub unsafe fn notify_others_all_clear(race: u16) {
    NotifyOthers(race, IPNL_ALL_CLEAR);
}

unsafe fn clone_npc_ship(
    ship: u8,
    crew: u16,
    captain: Option<u8>,
) -> Option<crate::engine::battle::link_queue::LinkId<crate::engine::world::queues::Fragment>> {
    WorldOwner::current().write(|world| {
        let fleets = world.avail_races_ref();
        let template = *fleets.get(fleets.at(usize::from(ship))?);
        let mut ships = world.npc_built_ships();
        let id = ships.clone_fragment(
            &template,
            ship,
            crew,
            crate::comm::dispatch::production_captain_name,
        )?;
        if let Some(captain) = captain {
            ships.get_mut(id).captains_name_index = captain;
        }
        Some(id)
    })
}

/// `CloneShipFragment (ship, &GLOBAL (npc_built_ship_q), INFINITE_FLEET)`.
///
/// # Safety
/// Game thread, with the fleet queue initialised.
pub unsafe fn clone_infinite_ship_into_built_queue(ship: u16) {
    clone_npc_ship(
        u8::try_from(ship).expect("ship index fits a byte"),
        INFINITE_FLEET,
        None,
    );
}

/// `CountLinks (&GLOBAL (npc_built_ship_q))`.
///
/// # Safety
/// Game thread; `GlobData` is initialised.
pub unsafe fn built_ship_count() -> u16 {
    glob().read(|world| world.npc_built_ships_ref().count() as u16)
}

/// The `star_index` byte of every catalog entry before `CurStarDescPtr`:
/// `for (SDPtr = &star_array[0]; SDPtr != CurStarDescPtr; ++SDPtr)`.
///
/// # Safety
/// `CurStarDescPtr` must be an entry of `star_array`.
pub unsafe fn star_indexes_before_current() -> Vec<u8> {
    let first = std::ptr::addr_of!(STAR_ARRAY).read();
    let current =
        (**WorldOwner::current()).star_pointer(crate::planets::starmap::current_catalog());
    let before = usize::try_from(current.offset_from(first))
        .expect("(**WorldOwner::current()).star_pointer(crate::planets::starmap::current_catalog()) is an entry of star_array");
    std::slice::from_raw_parts(first, before)
        .iter()
        .map(|star| star.index)
        .collect()
}

/// `LoadStdLanderFont (info)`, the mask `graphics` frame and the report
/// `strtab`, with the report started at `first_report` (the string-table
/// index call only when it is not the first).
///
/// # Safety
/// `system` must be the live solar system state, on the game thread; both
/// resource names must be NUL terminated.
pub unsafe fn load_lander_scene(
    system: *mut crate::c_bindings::global_state::SOLARSYS_STATE,
    graphics: &std::ffi::CStr,
    strtab: &std::ffi::CStr,
    first_report: u16,
) {
    LoadStdLanderFont(
        (std::ptr::addr_of_mut!((*system).SysInfo.PlanetInfo).cast::<c_void>()).cast(),
    );
    (*system).PlanetSideFrame[1] =
        CaptureDrawable((LoadGraphicInstance(graphics.as_ptr())).cast()).cast();
    let table = CaptureStringTable(LoadStringTableInstance(strtab.as_ptr()));
    (*system).SysInfo.PlanetInfo.DiscoveryString = if first_report == 0 {
        table.cast()
    } else {
        SetAbsStringTableIndex(table, first_report).cast()
    };
}

/// `LoadStdLanderFont (info)`, the `graphics.ruins` mask and the Androsynth
/// ruins report table started at report `visits`. When every report was
/// already given the table is destroyed and `DiscoveryString` cleared.
///
/// # Safety
/// `system` must be the live solar system state, on the game thread.
pub unsafe fn load_ruins_story(
    system: *mut crate::c_bindings::global_state::SOLARSYS_STATE,
    visits: u32,
) {
    LoadStdLanderFont(
        (std::ptr::addr_of_mut!((*system).SysInfo.PlanetInfo).cast::<c_void>()).cast(),
    );
    (*system).PlanetSideFrame[1] =
        CaptureDrawable((LoadGraphicInstance(c"graphics.ruins".as_ptr())).cast()).cast();
    let table = CaptureStringTable(LoadStringTableInstance(c"text.androsynthruins".as_ptr()));
    (*system).SysInfo.PlanetInfo.DiscoveryString = table.cast();
    if visits >= u32::from(GetStringTableCount(table)) {
        DestroyStringTable(ReleaseStringTable(table));
        (*system).SysInfo.PlanetInfo.DiscoveryString = std::ptr::null_mut();
    } else {
        let offset = i16::try_from(visits).expect("a visit count is at most sixteen");
        (*system).SysInfo.PlanetInfo.DiscoveryString = SetRelStringTableIndex(table, offset).cast();
    }
}

/// `LoadStdLanderFont (info)` and the `text.chmmrbase` report table, as the
/// `DiscoveryString` the starbase report presents.
///
/// # Safety
/// `system` must be the live solar system state, on the game thread.
pub unsafe fn load_starbase_report(system: *mut crate::c_bindings::global_state::SOLARSYS_STATE) {
    LoadStdLanderFont(
        (std::ptr::addr_of_mut!((*system).SysInfo.PlanetInfo).cast::<c_void>()).cast(),
    );
    (*system).SysInfo.PlanetInfo.DiscoveryString =
        CaptureStringTable(LoadStringTableInstance(c"text.chmmrbase".as_ptr())).cast();
}

/// `FreeLanderFont (info)`.
///
/// # Safety
/// `system` must be the live solar system state, on the game thread.
pub unsafe fn free_lander_font(system: *mut crate::c_bindings::global_state::SOLARSYS_STATE) {
    FreeLanderFont((std::ptr::addr_of_mut!((*system).SysInfo.PlanetInfo).cast::<c_void>()).cast());
}

/// `GetHeadLink (&GLOBAL (npc_built_ship_q)) != 0`.
///
/// # Safety
/// Game thread; `GlobData` is initialised.
pub unsafe fn built_ship_queue_has_ships() -> bool {
    glob().read(|world| world.npc_built_ships_ref().head().is_some())
}

/// `ONE_SHOT_ENCOUNTER` (`encount.h`).
const ONE_SHOT_ENCOUNTER: u8 = 1 << 7;

/// The one-shot Ur-Quan encounter `ZapToUrquanEncounter` builds at the
/// universe point `loc` from the `URQUAN_SHIP` fleet template, with the SIS
/// logical position set to match. False when there is no encounter to use.
///
/// # Safety
/// Game thread; `GlobData` and the fleet queue are initialised.
pub unsafe fn begin_urquan_encounter(loc: (i16, i16)) -> bool {
    glob().write(|world| {
        let mut encounters = world.encounters();
        let Some(id) = encounters.alloc().or_else(|| encounters.head()) else {
            return false;
        };
        if Some(id) == encounters.head() {
            encounters.remove(id);
        }
        *encounters.get_mut(id) = crate::navigation::abi::ENCOUNTER::default();
        encounters.insert_before(id, encounters.head());
        let fleets = world.avail_races_ref();
        let template = *fleets.get(
            fleets
                .at(usize::from(ship::URQUAN_SHIP))
                .expect("Ur-Quan fleet"),
        );
        let mut encounters = world.encounters();
        let encounter = encounters.get_mut(id);
        encounter.origin = template.loc;
        encounter.radius = template.actual_strength;
        encounter.race_id = ship::URQUAN_SHIP;
        encounter.num_ships = 1;
        encounter.flags = ONE_SHOT_ENCOUNTER;
        encounter.ship_list[0] = crate::navigation::abi::brief_ship_info {
            race_id: ship::URQUAN_SHIP,
            crew_level: template.crew_level,
            max_crew: template.max_crew,
            max_energy: template.max_energy,
        };
        encounter.loc_pt = CPoint { x: loc.0, y: loc.1 };
        encounter.log_x = universe_to_logx(loc.0);
        encounter.log_y = universe_to_logy(loc.1);
        let position = (encounter.log_x, encounter.log_y);
        world.data.flagship.log_x = position.0;
        world.data.flagship.log_y = position.1;
        true
    })
}

/// `SleepThreadUntil (FadeScreen (FadeAllToBlack, ONE_SECOND * 2))`.
///
/// # Safety
/// Game thread.
pub unsafe fn fade_to_black_for_two_seconds() {
    let interval = i16::try_from(ONE_SECOND * 2).expect("two seconds fit a SIZE");
    uqm_sleep_thread_until(FadeScreen(FADE_ALL_TO_BLACK, interval));
}

/// `CurStarDescPtr->star_pt`.
///
/// # Safety
/// `CurStarDescPtr` must point at a valid star description.
pub unsafe fn current_star_point() -> (i16, i16) {
    let point = (*(**WorldOwner::current())
        .star_pointer(crate::planets::starmap::current_catalog()))
    .star_pt;
    (point.x, point.y)
}

/// `GLOBAL_SIS (FuelOnBoard)`.
///
/// # Safety
/// Game thread; `GlobData` is initialised.
pub fn fuel_on_board() -> u32 {
    glob().read(|owner| owner.data.flagship.fuel_on_board)
}

/// `DeltaSISGauges (0, delta, 0)`.
///
/// # Safety
/// Game thread.
pub unsafe fn delta_fuel(delta: i16) {
    DeltaSISGauges(0, delta, 0);
}

/// `DrawSISMessage (NULL)`.
///
/// # Safety
/// Game thread.
pub unsafe fn clear_sis_message() {
    DrawSISMessage(std::ptr::null());
}

/// `DrawHyperCoords (loc)`.
///
/// # Safety
/// Game thread.
pub unsafe fn draw_hyper_coords(loc: (i16, i16)) {
    DrawHyperCoords(CPoint { x: loc.0, y: loc.1 });
}

/// The live game, through `GlobData` and the C group functions.
pub struct LiveGame;

fn glob() -> crate::engine::Projection<crate::engine::World> {
    {
        WorldOwner::current()
    }
}

/// `BATTLE_PLANET` start and end bit.
const BATTLE_PLANET: (i32, i32) = (751, 758);

impl PlanetNames for LiveGame {
    fn set_planet_name(&mut self, string_index: u16) {
        // SAFETY: `PlanetName` is the 16-byte name field of the SIS state;
        // `GAME_STRING` text is NUL terminated.
        unsafe {
            let text = crate::comm::hail::game_string(i32::from(string_index));
            let name = &mut (**glob()).data.flagship.planet_name;
            utf8StringCopy(name.as_mut_ptr().cast(), name.len(), (text).cast());
        }
    }
    fn set_battle_planet(&mut self, value: u8) {
        // SAFETY: writes the Rust-owned game state.
        WorldOwner::current().write(|world| {
            rust_set_game_state_bits(world, BATTLE_PLANET.0, BATTLE_PLANET.1, value)
        });
    }
}

impl LiveGame {
    pub(crate) fn load_ip_groups(&mut self) {
        // SAFETY: this orbital entry temporarily takes the published context; no borrow escapes.
        unsafe {
            crate::app::bridge::enter(|cx| get_group_info(cx.world, GROUPS_RANDOM, GROUP_LOAD_IP))
        };
    }
    pub(crate) fn save_ip_groups(&mut self) {
        // SAFETY: this orbital entry temporarily takes the published context; no borrow escapes.
        unsafe {
            crate::app::bridge::enter(|cx| {
                group_storage::put_group_info(cx.world, GROUPS_RANDOM, GROUP_SAVE_IP)
            })
        };
    }
    pub(crate) fn reinit_npc_built_ship_q(&mut self) {
        glob().write(|world| world.npc_built_ships().reinit());
    }
    pub(crate) fn reinit_ip_group_q(&mut self) {
        glob().write(|world| world.ip_groups().reinit());
    }
    pub(crate) fn clone_ship_into_built_queue(&mut self, ship: u8) {
        // SAFETY: the orbital entry owns the initialized NPC fragment queue on the game thread.
        unsafe { clone_npc_ship(ship, 0, None) };
    }
}
