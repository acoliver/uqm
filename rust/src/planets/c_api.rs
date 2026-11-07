//! The C functions and globals the default system generator still needs, in
//! one place. Each goes away when its owner is ported.

use crate::engine::World as WorldOwner;
mod surface;
pub use surface::{load_planet_surface, CapturedSurface};

use std::ffi::c_void;

use crate::c_bindings::global_state::EncounterGroup as ENCOUNTER_GROUP;
use crate::comm::dispatch::{ship, CPoint, ONE_SECOND};
use crate::mainloop::restart_menu::c_extern::{
    uqm_sleep_thread_until, FadeScreen, FADE_ALL_TO_BLACK,
};
use crate::planets::universe_units::{universe_to_logx, universe_to_logy};

#[cfg(not(test))]
use crate::ui::sis::repair_sisborder;
#[cfg(test)]
extern "C" {
    fn repair_sisborder();
}
#[cfg(all(not(test), feature = "debug-process"))]
pub use crate::front_end::uqmdebug::DISABLE_INTERACTIVITY;

pub use crate::log_abi::log_init;

pub use crate::navigation::groups::live::init_group_info;

pub use crate::navigation::groups::live::uninit_group_info;

pub use crate::planets::orbit::live::FreePlanet;

pub use crate::planets::plangen::live::generate_planet_surface;

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

}

/// `GROUP_INIT_IP`, `GROUP_SAVE_IP` (`state.h`): `(BYTE)~0`.
pub(super) const GROUP_INIT_IP: u8 = !0;
pub(super) const GROUP_SAVE_IP: u8 = !0;
/// `GROUP_LOAD_IP` is `GROUP_LIST` (`state.h`).
pub(super) const GROUP_LOAD_IP: u8 = 0;

/// Clear the guard encounter so the interplanetary groups do not chase.
///
/// # Safety
/// Game thread; the encounter globals belong to the current visit.
pub unsafe fn reset_encounter(world: &mut crate::engine::World) {
    std::ptr::addr_of_mut!(ENCOUNTER_GROUP).write(0);
    world.encounter_race = -1;
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
pub fn rebel_escort_feasible(world: &crate::engine::World) -> bool {
    {
        crate::engine::world::fleet::escort_feasibility(world, u16::from(ship::YEHAT_REBEL_SHIP))
            != 0
    }
}

/// The beast report has no mask, but still loads the lander font.
///
/// # Safety
/// `system` is the live system, with no outstanding references into it.
pub unsafe fn load_beast_scene(system: *mut crate::c_bindings::global_state::SOLARSYS_STATE) {
    LoadStdLanderFont(
        (std::ptr::addr_of_mut!((*system).SysInfo.planet_info).cast::<c_void>()).cast(),
    );
    (*system).PlanetSideFrame[1] = std::ptr::null_mut();
    (*system).SysInfo.planet_info.discovery_string =
        CaptureStringTable(LoadStringTableInstance(c"text.vuxbeast".as_ptr())).cast();
}

/// `GROUPS_RANDOM`.
pub(super) const GROUPS_RANDOM: u32 = 0;

/// `GROUPS_ADD_NEW` (`state.h`).
pub(super) const GROUPS_ADD_NEW: u32 = !0;
/// `SYREEN_CONVERSATION` and `SLYLANDRO_HOME_CONVERSATION` (`commglue.h`).
pub const SYREEN_CONVERSATION: u32 = 20;
pub const SLYLANDRO_HOME_CONVERSATION: u32 = 23;

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
pub fn start_sphere_tracking(world: &mut crate::engine::World, race: u16) -> bool {
    crate::engine::world::fleet::start_sphere_tracking(world, race) != 0
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
    world: &mut crate::engine::World,
    ship: u8,
    crew: u16,
    captain: Option<u8>,
) -> Option<crate::engine::battle::link_queue::LinkId<crate::engine::world::queues::Fragment>> {
    {
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
    }
}

/// `CloneShipFragment (ship, &GLOBAL (npc_built_ship_q), INFINITE_FLEET)`.
///
/// # Safety
/// Game thread, with the fleet queue initialised.
pub unsafe fn clone_infinite_ship_into_built_queue(world: &mut crate::engine::World, ship: u16) {
    clone_npc_ship(
        world,
        u8::try_from(ship).expect("ship index fits a byte"),
        INFINITE_FLEET,
        None,
    );
}

/// `CountLinks (&GLOBAL (npc_built_ship_q))`.
///
/// # Safety
/// Game thread; `GlobData` is initialised.
pub fn built_ship_count(world: &crate::engine::World) -> u16 {
    world.npc_built_ships_ref().count() as u16
}

/// The generator index of every catalog entry before the selected star.
pub fn star_indexes_before_current(world: &WorldOwner) -> Vec<u8> {
    let before = world.current_star.expect("current star is selected").0;
    world.catalog()[..before]
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
        (std::ptr::addr_of_mut!((*system).SysInfo.planet_info).cast::<c_void>()).cast(),
    );
    (*system).PlanetSideFrame[1] =
        CaptureDrawable((LoadGraphicInstance(graphics.as_ptr())).cast()).cast();
    let table = CaptureStringTable(LoadStringTableInstance(strtab.as_ptr()));
    (*system).SysInfo.planet_info.discovery_string = if first_report == 0 {
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
        (std::ptr::addr_of_mut!((*system).SysInfo.planet_info).cast::<c_void>()).cast(),
    );
    (*system).PlanetSideFrame[1] =
        CaptureDrawable((LoadGraphicInstance(c"graphics.ruins".as_ptr())).cast()).cast();
    let table = CaptureStringTable(LoadStringTableInstance(c"text.androsynthruins".as_ptr()));
    (*system).SysInfo.planet_info.discovery_string = table.cast();
    if visits >= u32::from(GetStringTableCount(table)) {
        DestroyStringTable(ReleaseStringTable(table));
        (*system).SysInfo.planet_info.discovery_string = std::ptr::null_mut();
    } else {
        let offset = i16::try_from(visits).expect("a visit count is at most sixteen");
        (*system).SysInfo.planet_info.discovery_string =
            SetRelStringTableIndex(table, offset).cast();
    }
}

/// `LoadStdLanderFont (info)` and the `text.chmmrbase` report table, as the
/// `DiscoveryString` the starbase report presents.
///
/// # Safety
/// `system` must be the live solar system state, on the game thread.
pub unsafe fn load_starbase_report(system: *mut crate::c_bindings::global_state::SOLARSYS_STATE) {
    LoadStdLanderFont(
        (std::ptr::addr_of_mut!((*system).SysInfo.planet_info).cast::<c_void>()).cast(),
    );
    (*system).SysInfo.planet_info.discovery_string =
        CaptureStringTable(LoadStringTableInstance(c"text.chmmrbase".as_ptr())).cast();
}

/// `FreeLanderFont (info)`.
///
/// # Safety
/// `system` must be the live solar system state, on the game thread.
pub unsafe fn free_lander_font(system: *mut crate::c_bindings::global_state::SOLARSYS_STATE) {
    FreeLanderFont((std::ptr::addr_of_mut!((*system).SysInfo.planet_info).cast::<c_void>()).cast());
}

/// `GetHeadLink (&GLOBAL (npc_built_ship_q)) != 0`.
///
/// # Safety
/// Game thread; `GlobData` is initialised.
pub fn built_ship_queue_has_ships(world: &crate::engine::World) -> bool {
    world.npc_built_ships_ref().head().is_some()
}

/// `ONE_SHOT_ENCOUNTER` (`encount.h`).
const ONE_SHOT_ENCOUNTER: u8 = 1 << 7;

/// The one-shot Ur-Quan encounter `ZapToUrquanEncounter` builds at the
/// universe point `loc` from the `URQUAN_SHIP` fleet template, with the SIS
/// logical position set to match. False when there is no encounter to use.
///
/// # Safety
/// Game thread; `GlobData` and the fleet queue are initialised.
pub unsafe fn begin_urquan_encounter(world: &mut crate::engine::World, loc: (i16, i16)) -> bool {
    {
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
    }
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
    let point = (*(**WorldOwner::current()).star_pointer()).star_pt;
    (point.x, point.y)
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
