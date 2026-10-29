//! The C functions and globals the default system generator still needs, in
//! one place. Each goes away when its owner is ported.

mod surface;
pub use surface::{load_planet_surface, CapturedSurface};

use std::ffi::c_void;

use crate::c_bindings::global_state::{
    planetOuterLocation, EncounterGroup as ENCOUNTER_GROUP, EncounterRace as ENCOUNTER_RACE,
    EscortFeasibilityStudy, XFormIPLoc, IP_GROUP,
};
use crate::collections::queue::{
    AllocLink, CountLinks, HLink, InsertQueue, Queue, ReinitQueue, RemoveQueue,
};
use crate::comm::dispatch::{
    get_star_ship_from_index, ship, BriefShipInfo, CEncounter, CFleetInfo, CPoint, ONE_SECOND,
};
use crate::mainloop::restart_menu::c_extern::{
    uqm_sleep_thread_until, FadeScreen, FADE_ALL_TO_BLACK,
};
use crate::planets::generate::default::{Npcs, PlanetNames};
use crate::planets::generate::special::FleetGroups;
use crate::planets::universe_units::{universe_to_logx, universe_to_logy};
use crate::state::ffi::{rust_get_game_state_bits, rust_set_game_state_bits};

#[cfg(not(test))]
use crate::ui::sis::RepairSISBorder;
#[cfg(test)]
extern "C" {
    fn RepairSISBorder();
}
#[cfg(all(not(test), feature = "debug-process"))]
pub use crate::front_end::uqmdebug::DISABLE_INTERACTIVITY;
use crate::strings::unicode::utf8_string_copy as utf8StringCopy;

pub use crate::log_abi::log_init;

use crate::navigation::groups::live::GetGroupInfo;

use crate::navigation::groups::live::PutGroupInfo;

pub use crate::navigation::groups::live::InitGroupInfo;

pub use crate::navigation::groups::live::UninitGroupInfo;

use crate::navigation::groups::live::BuildGroups;

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

use crate::battle::build_native::engine::StartSphereTracking;

use crate::navigation::interplanetary::live::notify_others_abi as NotifyOthers;

use crate::ui::sis::DeltaSISGauges;

use crate::ui::sis::DrawSISMessage;

use crate::ui::sis::DrawHyperCoords;

extern "C" {
    #[link_name = "GlobData"]
    pub static mut GLOB_DATA: crate::ships::native::abi::GLOBDATA;
    #[link_name = "GameStrings"]
    pub static mut GAME_STRINGS: *mut crate::strings::string_table::StringTableEntry;
    #[link_name = "format_conv_surf"]
    pub static mut FORMAT_CONV_SURF: *mut crate::graphics::ffi::SDL_Surface;
    #[link_name = "SDL_Screen"]
    pub static mut SDL_SCREEN: *mut crate::graphics::ffi::SDL_Surface;
    #[link_name = "star_array"]
    pub static mut STAR_ARRAY: *mut crate::savegame::runtime::star::CStarDesc;
    #[link_name = "CurStarDescPtr"]
    static mut CUR_STAR_DESC_PTR: *mut crate::savegame::runtime::star::CStarDesc;
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
    EscortFeasibilityStudy(u16::from(ship::YEHAT_REBEL_SHIP)) != 0
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
    (*std::ptr::addr_of!(CUR_STAR_DESC_PTR).read()).index
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
    crate::comm::dispatch::init_communication(which_comm);
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
    StartSphereTracking(race) != 0
}

/// `RepairSISBorder ()`.
///
/// # Safety
/// Game thread, inside a planet orbit.
pub unsafe fn repair_sis_border() {
    RepairSISBorder();
}

/// `NotifyOthers (race, IPNL_ALL_CLEAR)`.
///
/// # Safety
/// Game thread, inside a planet orbit.
pub unsafe fn notify_others_all_clear(race: u16) {
    NotifyOthers(race, IPNL_ALL_CLEAR);
}

/// `CloneShipFragment (ship, &GLOBAL (npc_built_ship_q), INFINITE_FLEET)`.
///
/// # Safety
/// Game thread, with the fleet queue initialised.
pub unsafe fn clone_infinite_ship_into_built_queue(ship: u16) {
    crate::comm::dispatch::clone_ship_fragment(
        u8::try_from(ship).expect("ship index fits a byte"),
        std::ptr::addr_of_mut!((*glob()).Game_state.npc_built_ship_q).cast::<Queue>(),
        INFINITE_FLEET,
    );
}

/// `CountLinks (&GLOBAL (npc_built_ship_q))`.
///
/// # Safety
/// Game thread; `GlobData` is initialised.
pub unsafe fn built_ship_count() -> u16 {
    CountLinks(std::ptr::addr_of!((*glob()).Game_state.npc_built_ship_q).cast::<Queue>())
}

/// The `star_index` byte of every catalog entry before `CurStarDescPtr`:
/// `for (SDPtr = &star_array[0]; SDPtr != CurStarDescPtr; ++SDPtr)`.
///
/// # Safety
/// `CurStarDescPtr` must be an entry of `star_array`.
pub unsafe fn star_indexes_before_current() -> Vec<u8> {
    let first = std::ptr::addr_of!(STAR_ARRAY).read();
    let current = std::ptr::addr_of!(CUR_STAR_DESC_PTR).read();
    let before = usize::try_from(current.offset_from(first))
        .expect("CurStarDescPtr is an entry of star_array");
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

/// `CloneShipFragment (ship, &GLOBAL (npc_built_ship_q), 1)` and, when a
/// fragment was made, `captains_name_index = name_index`.
///
/// # Safety
/// Game thread, with the fleet queue initialised.
pub unsafe fn clone_named_ship_into_built_queue(ship: u16, name_index: u8) {
    let fragment = crate::comm::dispatch::clone_ship_fragment(
        u8::try_from(ship).expect("ship index fits a byte"),
        std::ptr::addr_of_mut!((*glob()).Game_state.npc_built_ship_q).cast::<Queue>(),
        1,
    );
    if !fragment.is_null() {
        // The queue is a table queue: `LockShipFrag` is a cast of the link.
        (*fragment.cast::<crate::comm::dispatch::CShipFragment>()).captains_name_index = name_index;
    }
}

/// `PutGroupInfo (offset, 1)`.
///
/// # Safety
/// Game thread.
pub unsafe fn put_group_info(offset: u32) -> u32 {
    PutGroupInfo(offset, 1)
}

/// `GetHeadLink (&GLOBAL (npc_built_ship_q)) != 0`.
///
/// # Safety
/// Game thread; `GlobData` is initialised.
pub unsafe fn built_ship_queue_has_ships() -> bool {
    !(*std::ptr::addr_of!((*glob()).Game_state.npc_built_ship_q))
        .head
        .is_null()
}

/// `ONE_SHOT_ENCOUNTER` (`encount.h`).
const ONE_SHOT_ENCOUNTER: u8 = 1 << 7;

/// The head of the encounter queue, or the first encounter free to use:
/// `AllocEncounter ()` and failing that `GetHeadEncounter ()`. A head
/// encounter is taken off the queue first.
///
/// # Safety
/// Game thread; `GlobData` is initialised.
unsafe fn take_encounter() -> HLink {
    let queue = std::ptr::addr_of_mut!((*glob()).Game_state.encounter_q).cast::<Queue>();
    let mut handle = AllocLink(queue);
    if handle.is_null() {
        handle = (*queue).head;
    }
    if !handle.is_null() && handle == (*queue).head {
        RemoveQueue(queue, handle);
    }
    handle
}

/// The one-shot Ur-Quan encounter `ZapToUrquanEncounter` builds at the
/// universe point `loc` from the `URQUAN_SHIP` fleet template, with the SIS
/// logical position set to match. False when there is no encounter to use.
///
/// # Safety
/// Game thread; `GlobData` and the fleet queue are initialised.
pub unsafe fn begin_urquan_encounter(loc: (i16, i16)) -> bool {
    let handle = take_encounter();
    if handle.is_null() {
        return false;
    }
    let queue = std::ptr::addr_of_mut!((*glob()).Game_state.encounter_q).cast::<Queue>();
    // `LockEncounter` on a table queue is a cast.
    let encounter = handle.cast::<CEncounter>();
    std::ptr::write_bytes(encounter, 0, 1);
    InsertQueue(queue, handle, (*queue).head);

    let fleets = std::ptr::addr_of!((*glob()).Game_state.avail_race_q).cast::<Queue>();
    let template = get_star_ship_from_index(fleets, ship::URQUAN_SHIP).cast::<CFleetInfo>();
    assert!(
        !template.is_null(),
        "the available races include the Ur-Quan fleet"
    );
    let template = &*template;

    (*encounter).origin = template.loc;
    (*encounter).radius = template.actual_strength;
    (*encounter).race_id = ship::URQUAN_SHIP;
    (*encounter).num_ships = 1;
    (*encounter).flags = ONE_SHOT_ENCOUNTER;
    (*encounter).ship_list[0] = BriefShipInfo {
        race_id: ship::URQUAN_SHIP,
        crew_level: template.crew_level,
        max_crew: template.max_crew,
        max_energy: template.max_energy,
    };
    (*encounter).loc_pt = CPoint { x: loc.0, y: loc.1 };
    (*encounter).log_x = universe_to_logx(loc.0);
    (*encounter).log_y = universe_to_logy(loc.1);
    (*glob()).SIS_state.log_x = (*encounter).log_x;
    (*glob()).SIS_state.log_y = (*encounter).log_y;
    true
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
    let point = (*std::ptr::addr_of!(CUR_STAR_DESC_PTR).read()).star_pt;
    (point.x, point.y)
}

/// `GLOBAL_SIS (FuelOnBoard)`.
///
/// # Safety
/// Game thread; `GlobData` is initialised.
pub unsafe fn fuel_on_board() -> u32 {
    (*glob()).SIS_state.FuelOnBoard
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

fn glob() -> *mut crate::ships::native::abi::GLOBDATA {
    std::ptr::addr_of_mut!(GLOB_DATA)
}

impl Npcs for LiveGame {
    fn init_ip_groups(&mut self) -> bool {
        // SAFETY: game thread; `GlobData` is initialised.
        unsafe { GetGroupInfo((*glob()).Game_state.BattleGroupRef, GROUP_INIT_IP) != 0 }
    }
    fn clear_battle_group_ref(&mut self) {
        // SAFETY: as above.
        unsafe { (*glob()).Game_state.BattleGroupRef = 0 };
    }
    fn build_groups(&mut self) {
        // SAFETY: game thread.
        unsafe { BuildGroups() };
    }
    fn load_ip_groups(&mut self) {
        // SAFETY: game thread.
        unsafe { GetGroupInfo(GROUPS_RANDOM, GROUP_LOAD_IP) };
    }
    fn save_ip_groups(&mut self) {
        // SAFETY: game thread.
        unsafe { PutGroupInfo(GROUPS_RANDOM, GROUP_SAVE_IP) };
    }
    fn reinit_npc_built_ship_q(&mut self) {
        // SAFETY: the queue is the live `npc_built_ship_q`.
        unsafe {
            ReinitQueue(
                std::ptr::addr_of_mut!((*glob()).Game_state.npc_built_ship_q).cast::<Queue>(),
            )
        };
    }
    fn reinit_ip_group_q(&mut self) {
        // SAFETY: the queue is the live `ip_group_q`.
        unsafe {
            ReinitQueue(std::ptr::addr_of_mut!((*glob()).Game_state.ip_group_q).cast::<Queue>())
        };
    }
    fn yehat_civil_war(&self) -> bool {
        // SAFETY: reads the Rust-owned game state.
        unsafe { rust_get_game_state_bits(YEHAT_CIVIL_WAR.0, YEHAT_CIVIL_WAR.1) != 0 }
    }
    fn encounter_race(&self) -> i16 {
        // SAFETY: plain global read.
        unsafe { std::ptr::addr_of!(ENCOUNTER_RACE).read() }
    }
    fn for_each_ip_group(&mut self, visit: &mut dyn FnMut(&mut IP_GROUP)) {
        // SAFETY: walks the live `ip_group_q`; the successor is read before
        // the visit, as C does. `LockLink` on a table queue is a cast: the
        // links are the elements themselves.
        unsafe {
            let queue = std::ptr::addr_of!((*glob()).Game_state.ip_group_q);
            let mut handle = (*queue).head;
            while !handle.is_null() {
                let group = handle.cast::<IP_GROUP>();
                let next = (*group).succ;
                visit(&mut *group);
                handle = next;
            }
        }
    }
}

/// `YEHAT_CIVIL_WAR` start and end bit (`globdata.h`).
const YEHAT_CIVIL_WAR: (i32, i32) = (285, 285);
/// `BATTLE_PLANET` start and end bit.
const BATTLE_PLANET: (i32, i32) = (751, 758);

impl PlanetNames for LiveGame {
    fn set_planet_name(&mut self, string_index: u16) {
        // SAFETY: `PlanetName` is the 16-byte name field of the SIS state;
        // `GAME_STRING` text is NUL terminated.
        unsafe {
            let text = crate::comm::hail::game_string(i32::from(string_index));
            let name = &mut (*glob()).SIS_state.PlanetName;
            utf8StringCopy(name.as_mut_ptr().cast(), name.len(), (text).cast());
        }
    }
    fn set_battle_planet(&mut self, value: u8) {
        // SAFETY: writes the Rust-owned game state.
        unsafe { rust_set_game_state_bits(BATTLE_PLANET.0, BATTLE_PLANET.1, value) };
    }
}

impl FleetGroups for LiveGame {
    fn battle_group_ref(&self) -> u32 {
        // SAFETY: game thread; `GlobData` is initialised.
        unsafe { (*glob()).Game_state.BattleGroupRef }
    }
    fn set_battle_group_ref(&mut self, value: u32) {
        // SAFETY: as above.
        unsafe { (*glob()).Game_state.BattleGroupRef = value };
    }
    fn clone_ship_into_built_queue(&mut self, ship: u8) {
        // SAFETY: the queue is the live `npc_built_ship_q`.
        unsafe {
            crate::comm::dispatch::clone_ship_fragment(
                ship,
                std::ptr::addr_of_mut!((*glob()).Game_state.npc_built_ship_q).cast::<Queue>(),
                0,
            )
        };
    }
    fn add_new_group(&mut self) -> u32 {
        // SAFETY: game thread.
        unsafe { PutGroupInfo(GROUPS_ADD_NEW, 1) }
    }
    fn reinit_built_queue(&mut self) {
        self.reinit_npc_built_ship_q();
    }
    fn head_ip_group(&mut self) -> Option<&mut IP_GROUP> {
        // SAFETY: the head of the live `ip_group_q`; its links are the
        // elements themselves, as in `for_each_ip_group`.
        unsafe {
            let head = (*std::ptr::addr_of!((*glob()).Game_state.ip_group_q)).head;
            if head.is_null() {
                None
            } else {
                let queue = std::ptr::addr_of!((*glob()).Game_state.ip_group_q).cast::<Queue>();
                assert_eq!(
                    usize::from((*queue).object_size),
                    std::mem::size_of::<IP_GROUP>()
                );
                assert_ip_group_link(queue, head);
                Some(&mut *head.cast::<IP_GROUP>())
            }
        }
    }
}

unsafe fn assert_ip_group_link(queue: *const Queue, handle: HLink) {
    let start = (*queue).pq_tab as usize;
    let end = start + usize::from((*queue).object_size) * usize::from((*queue).num_objects);
    assert!(!(*queue).pq_tab.is_null() && handle as usize >= start && (handle as usize) < end);
}

impl crate::planets::generate::sam::GuardSite for LiveGame {
    fn save_ip_groups(&mut self) {
        Npcs::save_ip_groups(self);
    }
    fn load_ip_groups(&mut self) {
        Npcs::load_ip_groups(self);
    }
    fn reset_encounter(&mut self) {
        // SAFETY: game-thread encounter globals.
        unsafe { reset_encounter() };
    }
    fn built_ship_count(&self) -> u16 {
        // SAFETY: the live built queue on the game thread.
        unsafe { built_ship_count() }
    }
    fn set_battle_group_ref(&mut self, value: u32) {
        FleetGroups::set_battle_group_ref(self, value);
    }
    fn battle_group_ref(&self) -> u32 {
        FleetGroups::battle_group_ref(self)
    }
    fn clone_ship(&mut self, race: u8) {
        self.clone_ship_into_built_queue(race);
    }
    fn add_new_group(&mut self) -> u32 {
        FleetGroups::add_new_group(self)
    }
    fn store_group(&mut self, reference: u32, group: u8) {
        // SAFETY: the guard group and built queue belong to this visit.
        unsafe { PutGroupInfo(reference, group) };
    }
    fn reinit_built_queue(&mut self) {
        self.reinit_npc_built_ship_q();
    }
    fn init_ip_groups(&mut self, reference: u32) {
        // SAFETY: the reference was stored by PutGroupInfo.
        unsafe { GetGroupInfo(reference, GROUP_INIT_IP) };
    }
    fn planet_outer_location(&self, planet: u16) -> crate::c_bindings::global_state::POINT {
        // SAFETY: the planet index belongs to the current system.
        unsafe { planetOuterLocation(planet) }
    }
    fn for_each_ip_group(&mut self, visit: &mut dyn FnMut(&mut IP_GROUP)) {
        // SAFETY: the live table queue on the game thread. Preserve both
        // LockIpGroup's size assertion and Lock/UnlockLink's bounds checks.
        unsafe {
            let queue = std::ptr::addr_of!((*glob()).Game_state.ip_group_q).cast::<Queue>();
            let mut handle = (*queue).head;
            while !handle.is_null() {
                assert_eq!(
                    usize::from((*queue).object_size),
                    std::mem::size_of::<IP_GROUP>()
                );
                assert_ip_group_link(queue, handle);
                let group = handle.cast::<IP_GROUP>();
                let next = (*group).succ;
                visit(&mut *group);
                assert_ip_group_link(queue, handle);
                handle = next;
            }
        }
    }
    fn ip_location(&self) -> crate::c_bindings::global_state::POINT {
        // SAFETY: game-thread global, represented by the asserted GLOBDATA layout.
        unsafe {
            let point = (*glob()).Game_state.ip_location;
            crate::c_bindings::global_state::POINT {
                x: point.x,
                y: point.y,
            }
        }
    }
    fn set_and_transform_ip_location(&mut self, location: crate::c_bindings::global_state::POINT) {
        // SAFETY: disjoint global fields, with no Rust references held across
        // the legacy coordinate transform. CPoint's layout is bindgen-asserted.
        unsafe {
            let input = std::ptr::addr_of_mut!((*glob()).Game_state.ip_location);
            input.write(crate::comm::locdata::CPoint {
                x: location.x,
                y: location.y,
            });
            let output = std::ptr::addr_of_mut!((*glob()).Game_state.ShipStamp.origin);
            XFormIPLoc(input.cast(), output.cast(), 1);
        }
    }
}
