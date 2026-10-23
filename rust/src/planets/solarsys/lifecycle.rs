//! Solar-system entry, orbit, saving, and menu lifecycle from `solarsys.c`.

use super::drawing::*;
use super::flight::*;
use super::*;

use crate::battle::gendef::getGenerateFunctions;
use crate::c_bindings::controller_input as input;
use crate::collections::queue::ReinitQueue;
use crate::comm::locdata::{CGlobData, CPoint};
use crate::graphics::pixmap_abi::{GetFrameIndex, ReleaseDrawable, SetAbsFrameIndex};
use crate::mainloop::restart_menu::c_extern::{DoInput, PlayMusic};
use crate::math::arctan;
use crate::planet_side::orbit_music::load_lander_data;
use crate::planets::devices::DevicesMenu;
use crate::planets::ffi::do_planetary_analysis_c;
use crate::planets::starmap::FindStar;
use crate::sound::heart_ffi::PLRPlaying;
use crate::state::game_state_keys::{get_game_state, set_game_state};
use crate::strings::string_table::{GetStringAddress, SetAbsStringTableIndex};
use std::ffi::c_char;
use std::mem::MaybeUninit;

#[export_name = "pSolarSysState"]
pub static mut P_SOLAR_SYS_STATE: *mut c::SOLARSYS_STATE = ptr::null_mut();
#[export_name = "EncounterRace"]
pub static mut ENCOUNTER_RACE: c::SIZE = 0;

// globdata.h activity flags occupy the high byte of ACTIVITY.
const START_ENCOUNTER: u16 = 1 << 10;
const END_INTERPLANETARY: u16 = 1 << 11;
const CHECK_LOAD: u16 = 1 << 12;
const CHECK_RESTART: u16 = 1 << 13;
const CHECK_ABORT: u16 = 1 << 14;
const MAX_ZOOM_RADIUS: c::SIZE = 128 << 6;
const ONE_SECOND: u32 = 840;
const IP_FRAME_RATE: u32 = ONE_SECOND / 30;
const FUSION_THRUSTER: u8 = 1;
const TURNING_JETS: u8 = 2;
const IP_SHIP_THRUST_INCREMENT: u8 = 8;
const IP_SHIP_TURN_WAIT: u8 = 17;
const IP_SHIP_TURN_DECREMENT: u8 = 2;
const STARMAP: u8 = 1;
const EQUIP_DEVICE: u8 = 2;
const CARGO: u8 = 3;
const ROSTER: u8 = 4;
const GAME_MENU: u8 = 5;
const NAVIGATION: u8 = 6;
const PM_SCAN: i16 = 0;
const PM_STARMAP: u8 = 1;
const PM_NAVIGATE: i16 = 6;
const OPT_PC: i32 = 2;
const MENU_SOUND_NONE: u16 = 0;
const MENU_SOUND_ARROWS: u16 = 0x0f;
const MENU_SOUND_SELECT: u16 = 1 << 4;
const KEY_MENU_SELECT: usize = 9;
const KEY_MENU_CANCEL: usize = 10;
const FADE_ALL_TO_COLOR: u32 = 253;
const SOL_DEFINED: u8 = 1;
const SPATHI_DEFINED: u8 = 5;
const SAMATRA_DEFINED: u8 = 29;
const MATCH_PLANET: u8 = 0xff;
const PLANET_NUMBER_BASE: u16 = 375;
const STARBASE_STRING_BASE: u16 = 422;

static mut NEXT_TIME: u32 = 0;

mod ffi {
    use super::*;

    pub(super) use crate::time::counter_abi::GetTimeCounter;
    pub(super) use crate::ui::border::DrawSISFrame;
    pub(super) use crate::ui::settings::StopMusic;
    pub(super) use crate::ui::sis::RepairSISBorder;
    extern "C" {
        pub(super) static mut GlobData: CGlobData;
        pub(super) static mut CurStarDescPtr: *mut c::STAR_DESC;
        pub(super) static mut LastActivity: c::ACTIVITY;
        pub(super) static mut NextActivity: c::ACTIVITY;
        pub(super) static mut SpaceContext: *mut c_void;
        pub(super) static mut Screen: *mut c::FRAME_DESC;
        pub(super) static mut GameStrings: c::STRING;
        pub(super) static optWhichMenu: i32;

        pub(super) fn DestroyDrawable(drawable: *mut c_void) -> c::BOOLEAN;
        pub(super) fn SetContext(context: *mut c_void) -> *mut c_void;
        pub(super) fn SetContextFGFrame(frame: *mut c_void) -> *mut c_void;
        pub(super) fn SetContextBackGroundColor(color: c::Color) -> c::Color;
        pub(super) fn SetTransitionSource(rect: *mut c_void);
        pub(super) fn BatchGraphics();
        pub(super) fn UnbatchGraphics();
        pub(super) fn RedrawQueue(force: c::BOOLEAN);
        pub(super) fn ScreenTransition(kind: i32, rect: *const c_void);
        pub(super) fn FadeScreen(kind: u32, interval: c::SIZE) -> c::DWORD;
        pub(super) fn SetFlashRect(rect: *const c_void);
        pub(super) fn DrawMenuStateStrings(base: c::BYTE, state: c::SWORD);

        pub(super) fn DrawSISMessage(text: *const c_char);
        pub(super) fn DrawStatusMessage(text: *const c_char);

        pub(super) fn InitDisplayList();
        pub(super) fn DoMissions();

        pub(super) fn GetPlanetInfo();
        pub(super) fn PutPlanetInfo();
        pub(super) fn PlanetOrbitMenu();
        pub(super) fn FreePlanet();
        pub(super) fn GetGroupInfo(offset: c::DWORD, group: c::BYTE) -> c::BOOLEAN;
        pub(super) fn SetMenuSounds(sound0: u16, sound1: u16);
        pub(super) fn DoMenuChooser(menu: *mut c::MENU_STATE, base: c::BYTE) -> c::BOOLEAN;
        pub(super) fn CargoMenu();
        pub(super) fn RosterMenu() -> i32;
        pub(super) fn GameOptions() -> u32;
        pub(super) fn StarMap() -> i32;
        pub(super) fn SleepThreadUntil(time: u32);

    }
}

#[inline]
pub(super) unsafe fn glob() -> *mut CGlobData {
    ptr::addr_of_mut!(ffi::GlobData)
}

#[inline]
pub(super) unsafe fn system() -> *mut c::SOLARSYS_STATE {
    crate::c_bindings::solar_system::current()
}

const fn rgb15(r: u8, g: u8, b: u8) -> c::Color {
    c::Color {
        r: (r << 3) | (r >> 2),
        g: (g << 3) | (g >> 2),
        b: (b << 3) | (b >> 2),
        a: 0xff,
    }
}

#[inline]
fn c_point(point: CPoint) -> c::POINT {
    c::POINT {
        x: point.x,
        y: point.y,
    }
}

#[inline]
fn saved_point(point: c::POINT) -> CPoint {
    CPoint {
        x: point.x,
        y: point.y,
    }
}

pub(super) unsafe fn generate_moons(system: *mut c::SOLARSYS_STATE, planet: *mut c::PLANET_DESC) {
    (*SysGenRNG).reseed((*planet).rand_seed);
    ((*(*system).genFuncs).generateName.unwrap())(system, planet);
    ((*(*system).genFuncs).generateMoons.unwrap())(system, planet);

    let angle = arctan((*planet).location.x, (*planet).location.y);
    let _facing = (((u32::from(angle) + 2) >> 2) & 15) as c::COUNT;
    for i in 0..(*system).MoonDesc.len() {
        let moon = (*system).MoonDesc.as_mut_ptr().add(i);
        (*moon).pPrevDesc = planet;
        if i >= usize::from((*planet).NumPlanets) {
            continue;
        }
        (*moon).temp_color = (*planet).temp_color;
    }
}

pub(super) unsafe fn init_solar_sys_sis_characteristics() {
    let state = system();
    let mut num_thrusters = 0_u8;
    for i in 0..11 {
        if (*glob()).sis_state.drive_slots[i] == FUSION_THRUSTER {
            num_thrusters += 1;
        }
    }
    (*state).max_ship_speed = (num_thrusters + 5) * IP_SHIP_THRUST_INCREMENT;
    (*state).turn_wait = IP_SHIP_TURN_WAIT;
    for i in 0..8 {
        if (*glob()).sis_state.jet_slots[i] == TURNING_JETS {
            (*state).turn_wait = (*state).turn_wait.wrapping_sub(IP_SHIP_TURN_DECREMENT);
        }
    }
}

pub(super) unsafe fn load_solar_sys() -> *mut c::PLANET_DESC {
    const TEMP_COLORS: [c::Color; 5] = [
        rgb15(0x00, 0x00, 0x0e),
        rgb15(0x00, 0x06, 0x08),
        rgb15(0x00, 0x0b, 0x00),
        rgb15(0x0f, 0x00, 0x00),
        rgb15(0x0f, 0x08, 0x00),
    ];

    let state = system();
    let mut orbital = ptr::null_mut();
    (*SysGenRNG).reseed(GetRandomSeedForStar(ffi::CurStarDescPtr));
    SUN_FRAME = SetAbsFrameIndex(
        SUN_FRAME.cast(),
        u16::from(((*ffi::CurStarDescPtr).Type & 0x18) >> 3),
    )
    .cast();

    let sun = (*state).SunDesc.as_mut_ptr();
    (*sun).pPrevDesc = ptr::null_mut();
    (*sun).rand_seed = (*SysGenRNG).next_random();
    (*sun).data_index = ((*ffi::CurStarDescPtr).Type & 0x18) >> 3;
    (*sun).location = c::POINT { x: 0, y: 0 };
    (*sun).image.origin = (*sun).location;
    (*sun).image.frame = SUN_FRAME;

    ((*(*state).genFuncs).generatePlanets.unwrap())(state);
    if get_game_state("PLANETARY_CHANGE") != 0 {
        ffi::PutPlanetInfo();
        set_game_state("PLANETARY_CHANGE", 0);
    }

    for i in 0..(*state).PlanetDesc.len() {
        let planet = (*state).PlanetDesc.as_mut_ptr().add(i);
        (*planet).pPrevDesc = sun;
        (*planet).image.origin = (*planet).location;
        if i >= usize::from((*sun).NumPlanets) {
            (*planet).image.frame = ptr::null_mut();
        } else {
            // The Rust analysis implementation borrows the whole SYSTEM_INFO.
            let mut info: c::SYSTEM_INFO = std::mem::zeroed();
            do_planetary_analysis_c(ptr::addr_of_mut!(info), planet);
            let temperature = info.PlanetInfo.SurfaceTemperature;
            let mut index = ((i32::from(temperature) + 250) / 100) as c::COUNT;
            if usize::from(index) >= TEMP_COLORS.len() {
                index = (TEMP_COLORS.len() - 1) as c::COUNT;
            }
            (*planet).temp_color = TEMP_COLORS[usize::from(index)];
        }
    }
    sort_planet_positions();

    if (*glob()).game_state.ip_planet == 0 {
        (*state).pBaseDesc = (*state).PlanetDesc.as_mut_ptr();
        (*state).pOrbitalDesc = ptr::null_mut();
    } else {
        (*sun).location = c_point((*glob()).game_state.ip_location);
        (*glob()).game_state.ip_location = saved_point(displayToLocation(
            c_point((*glob()).game_state.ship_stamp.origin),
            MAX_ZOOM_RADIUS,
        ));
        let i = usize::from((*glob()).game_state.ip_planet - 1);
        (*state).pOrbitalDesc = (*state).PlanetDesc.as_mut_ptr().add(i);
        generate_moons(state, (*state).pOrbitalDesc);
        (*state).pBaseDesc = (*state).MoonDesc.as_mut_ptr();
        set_game_state("PLANETARY_LANDING", 0);
    }

    init_solar_sys_sis_characteristics();
    if (*glob()).game_state.in_orbit != 0 {
        let i = usize::from((*glob()).game_state.in_orbit - 1);
        orbital = if i == 0 {
            (*(*state).pBaseDesc).pPrevDesc
        } else {
            (*state).MoonDesc.as_mut_ptr().add(i - 1)
        };
        (*glob()).game_state.ip_location = saved_point((*sun).location);
        (*glob()).game_state.in_orbit = 0;
    } else {
        let mut i = (*glob()).game_state.ship_facing;
        if i == 0 {
            i += 1;
        }
        (*glob()).game_state.ship_stamp.frame = SetAbsFrameIndex(SISIPFRAME.cast(), i - 1).cast();
    }
    orbital
}

pub(super) unsafe fn save_non_orbital_location() {
    (*glob()).game_state.ship_facing =
        GetFrameIndex((*glob()).game_state.ship_stamp.frame.cast()).wrapping_add(1);
    (*glob()).game_state.in_orbit = 0;
    if !playerInInnerSystem() {
        (*glob()).game_state.ip_planet = 0;
    } else {
        let state = system();
        (*glob()).game_state.ip_planet = (1 + planetIndex(state, (*state).pOrbitalDesc)) as u8;
        (*glob()).game_state.ip_location = saved_point((*state).SunDesc[0].location);
    }
}

pub(super) unsafe fn free_solar_sys() {
    let state = system();
    if (*state).InIpFlight != 0 {
        (*state).InIpFlight = 0;
        if (*glob()).game_state.current_activity & (CHECK_ABORT | CHECK_LOAD) == 0 {
            save_non_orbital_location();
        }
    }
    ffi::DestroyDrawable(ReleaseDrawable(SOLAR_SYS_FRAME.cast()).cast());
    SOLAR_SYS_FRAME = ptr::null_mut();
    ffi::StopMusic();
}

pub(super) unsafe fn reset_solar_sys() {
    let state = system();
    assert!((*state).InIpFlight == 0);
    ffi::DrawMenuStateStrings(PM_STARMAP, -(PM_NAVIGATE - PM_SCAN));
    ffi::InitDisplayList();
    ffi::DoMissions();
    check_intersect();
    (*state).InIpFlight = 1;
    if PLRPlaying(usize::MAX as *mut c_void) == 0
        && (ffi::LastActivity != CHECK_LOAD || ffi::NextActivity != 0)
    {
        PlayMusic(SpaceMusic, 1, 1);
    }
}

pub(super) unsafe fn enter_planet_orbit() {
    let state = system();
    if (*state).InIpFlight != 0 {
        free_solar_sys();
        if worldIsMoon(state, (*state).pOrbitalDesc) {
            (*glob()).game_state.ship_stamp.origin =
                saved_point((*(*state).pOrbitalDesc).image.origin);
        } else {
            let (width, height) = sis_dimensions();
            (*glob()).game_state.ship_stamp.origin.x = (width >> 1) as _;
            (*glob()).game_state.ship_stamp.origin.y = (height >> 1) as _;
        }
    }

    ffi::GetPlanetInfo();
    ((*(*state).genFuncs).generateOrbital.unwrap())(state, (*state).pOrbitalDesc);
    ffi::LastActivity &= !(CHECK_LOAD | CHECK_RESTART);
    if (*glob()).game_state.current_activity & (CHECK_ABORT | CHECK_LOAD | START_ENCOUNTER) != 0
        || (*glob()).sis_state.crew_enlisted == c::COUNT::MAX
        || get_game_state("CHMMR_BOMB_STATE") == 2
    {
        return;
    }
    if !(*state).TopoFrame.is_null() {
        ffi::PlanetOrbitMenu();
        ffi::FreePlanet();
    }
    if (*glob()).game_state.current_activity & (START_ENCOUNTER | CHECK_ABORT | CHECK_LOAD) == 0
        && (*glob()).sis_state.crew_enlisted != c::COUNT::MAX
    {
        let orbital = load_solar_sys();
        assert!(orbital.is_null());
        check_zoom_level();
        validate_orbits();
        validate_inner_orbits();
        reset_solar_sys();
        ffi::RepairSISBorder();
        ffi::SetContext(ffi::SpaceContext);
        draw_system_transition(playerInInnerSystem());
    }
}

pub(super) unsafe fn init_solar_sys() {
    super::resources::LoadIPData();
    load_lander_data();

    let state = system();
    let reentry = (*glob()).game_state.ship_facing != 0;
    if !reentry {
        (*glob()).game_state.autopilot = CPoint { x: -1, y: -1 };
        let (width, height) = sis_dimensions();
        (*glob()).game_state.ship_stamp.origin.x = (width >> 1) as _;
        (*glob()).game_state.ship_stamp.origin.y = (height - 2) as _;
        (*glob()).game_state.ip_location = saved_point(displayToLocation(
            c_point((*glob()).game_state.ship_stamp.origin),
            MAX_ZOOM_RADIUS,
        ));
    }

    STARS_FRAME = create_star_background();
    ffi::SetContext(ffi::SpaceContext);
    ffi::SetContextFGFrame(ffi::Screen.cast());
    ffi::SetContextBackGroundColor(rgb15(0, 0, 0));

    let orbital = load_solar_sys();
    let inner_system = check_zoom_level() != 0;
    validate_orbits();
    if inner_system {
        validate_inner_orbits();
    }
    if reentry {
        ((*(*state).genFuncs).reinitNpcs.unwrap())(state);
    } else {
        ENCOUNTER_RACE = -1;
        EncounterGroup = 0;
        (*glob()).game_state.battle_group_ref = 0;
        ReinitQueue(ptr::addr_of_mut!((*glob()).game_state.ip_group_q).cast());
        ReinitQueue(ptr::addr_of_mut!((*glob()).game_state.npc_built_ship_q).cast());
        ((*(*state).genFuncs).initNpcs.unwrap())(state);
    }

    if !orbital.is_null() {
        enter_orbital(orbital);
    } else {
        ffi::SetContext(ffi::SpaceContext);
        ffi::SetTransitionSource(ptr::null_mut());
        ffi::BatchGraphics();
        ffi::DrawSISFrame();
        ffi::DrawSISMessage(ptr::null());
        reset_solar_sys();
        if ffi::LastActivity == CHECK_LOAD | CHECK_RESTART {
            draw_outer_system();
            ffi::RedrawQueue(0);
            ffi::UnbatchGraphics();
            ffi::FadeScreen(FADE_ALL_TO_COLOR, (ONE_SECOND / 2) as c::SIZE);
            ffi::LastActivity = 0;
        } else if ffi::LastActivity == CHECK_LOAD && ffi::NextActivity == 0 {
            ffi::UnbatchGraphics();
        } else {
            if inner_system {
                draw_inner_system();
            } else {
                draw_outer_system();
            }
            ffi::RedrawQueue(0);
            ffi::ScreenTransition(3, ptr::null());
            ffi::UnbatchGraphics();
            ffi::LastActivity &= !CHECK_LOAD;
        }
    }
}

pub(super) unsafe fn end_interplanetary() {
    (*glob()).game_state.current_activity &= !END_INTERPLANETARY;
    if (*glob()).game_state.current_activity & (CHECK_ABORT | CHECK_LOAD) == 0 {
        let state = system();
        ((*(*state).genFuncs).uninitNpcs.unwrap())(state);
        set_game_state("USED_BROADCASTER", 0);
    }
}

pub(super) unsafe fn closest_planet_interplanetary(point: *const c::POINT) -> *mut c::PLANET_DESC {
    let state = system();
    assert!(!state.is_null());
    let num_planets = (*state).SunDesc[0].NumPlanets;
    let mut best_distance_squared = u32::MAX;
    let mut best_planet = ptr::null_mut();
    for i in 0..usize::from(num_planets) {
        let planet = (*state).PlanetDesc.as_mut_ptr().add(i);
        let dx = (i32::from((*point).x) - i32::from((*planet).image.origin.x)) as c::SIZE;
        let dy = (i32::from((*point).y) - i32::from((*planet).image.origin.y)) as c::SIZE;
        let distance_squared =
            (i64::from(dx) * i64::from(dx) + i64::from(dy) * i64::from(dy)) as c::DWORD;
        if distance_squared < best_distance_squared {
            best_distance_squared = distance_squared;
            best_planet = planet;
        }
    }
    best_planet
}

pub(super) unsafe fn uninit_solar_sys() {
    free_solar_sys();
    ffi::DestroyDrawable(ReleaseDrawable(STARS_FRAME.cast()).cast());
    STARS_FRAME = ptr::null_mut();
    if (*glob()).game_state.current_activity & END_INTERPLANETARY != 0 {
        end_interplanetary();
        return;
    }
    if (*glob()).game_state.current_activity & START_ENCOUNTER != 0 && EncounterGroup != 0 {
        ffi::GetGroupInfo((*glob()).game_state.battle_group_ref, EncounterGroup);
        if (*glob()).game_state.ip_planet == 0 {
            let point = c_point((*glob()).game_state.ship_stamp.origin);
            let planet = closest_planet_interplanetary(&point);
            let state = system();
            ((*(*state).genFuncs).generateName.unwrap())(state, planet);
        }
    }
}

#[inline]
pub(super) unsafe fn hyperspace_unit_factors() -> (i32, i32) {
    let (width, height) = dimensions();
    ((((width - 64) << 5) >> 4) * 195, ((height << 5) >> 4) * 25)
}

#[no_mangle]
pub unsafe extern "C" fn ExploreSolarSys() {
    let mut solar_sys_state = MaybeUninit::<c::SOLARSYS_STATE>::uninit();
    if ffi::CurStarDescPtr.is_null() {
        let (log_units_x, log_units_y) = hyperspace_unit_factors();
        let mut universe = c::POINT {
            x: (((*glob()).sis_state.log_x * 6250 + (log_units_x >> 1)) / log_units_x) as _,
            y: (9999 - ((*glob()).sis_state.log_y * 625 + (log_units_y >> 1)) / log_units_y) as _,
        };
        ffi::CurStarDescPtr =
            FindStar(ptr::null_mut(), ptr::addr_of_mut!(universe).cast(), 1, 1).cast();
        if ffi::CurStarDescPtr.is_null() {
            crate::log_abi::log_message(1, "ExploreSolarSys(): do not know where you are!");
            #[cfg(feature = "debug-process")]
            libc::abort();
            #[cfg(not(feature = "debug-process"))]
            libc::exit(libc::EXIT_FAILURE);
        }
    }
    let (log_units_x, log_units_y) = hyperspace_unit_factors();
    (*glob()).sis_state.log_x =
        (i32::from((*ffi::CurStarDescPtr).star_pt.x) * log_units_x + (6250 >> 1)) / 6250;
    (*glob()).sis_state.log_y =
        ((9999 - i32::from((*ffi::CurStarDescPtr).star_pt.y)) * log_units_y + (625 >> 1)) / 625;

    // DoInput runs callbacks synchronously; this storage stays live through teardown.
    P_SOLAR_SYS_STATE = solar_sys_state.as_mut_ptr();
    ptr::write_bytes(P_SOLAR_SYS_STATE, 0, 1);
    (*P_SOLAR_SYS_STATE).genFuncs = getGenerateFunctions((*ffi::CurStarDescPtr).Index).cast();
    init_solar_sys();
    ffi::SetMenuSounds(MENU_SOUND_NONE, MENU_SOUND_NONE);
    (*P_SOLAR_SYS_STATE).InputFunc = Some(do_ip_flight);
    DoInput(P_SOLAR_SYS_STATE.cast(), 0);
    uninit_solar_sys();
    P_SOLAR_SYS_STATE = ptr::null_mut();
}

pub(super) unsafe fn game_string(index: c::COUNT) -> *mut c_char {
    GetStringAddress(SetAbsStringTableIndex(ffi::GameStrings.cast(), index))
}

#[no_mangle]
pub unsafe extern "C" fn GetNamedPlanetaryBody() -> *mut c::UNICODE {
    if ffi::CurStarDescPtr.is_null() || !playerInSolarSystem() || !playerInInnerSystem() {
        return ptr::null_mut();
    }
    let state = system();
    let orbital = (*state).pOrbitalDesc;
    assert!(!orbital.is_null());
    match (*ffi::CurStarDescPtr).Index {
        SOL_DEFINED => {
            let planet = planetIndex(state, orbital);
            if worldIsPlanet(state, orbital) {
                return game_string(PLANET_NUMBER_BASE + planet);
            }
            let moon = moonIndex(state, orbital);
            match (planet, moon) {
                (2, 0) => return game_string(STARBASE_STRING_BASE),
                (2, 1) => return game_string(PLANET_NUMBER_BASE + 9),
                (4, 0..=3) => return game_string(PLANET_NUMBER_BASE + 10 + moon),
                (5, 0) => return game_string(PLANET_NUMBER_BASE + 14),
                (7, 0) => return game_string(PLANET_NUMBER_BASE + 15),
                _ => {}
            }
        }
        SPATHI_DEFINED => {
            // The source's Spathiwa name is inside an inactive NOTYET block.
            let _ = matchWorld(state, orbital, 0, MATCH_PLANET);
        }
        SAMATRA_DEFINED if matchWorld(state, orbital, 4, 0) => {
            return game_string(PLANET_NUMBER_BASE + 32);
        }
        _ => {}
    }
    ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn SaveSolarSysLocation() {
    assert!(playerInSolarSystem());
    if !playerInPlanetOrbit() {
        save_non_orbital_location();
    } else {
        if get_game_state("PLANETARY_CHANGE") != 0 {
            ffi::PutPlanetInfo();
            set_game_state("PLANETARY_CHANGE", 0);
        }
        assert!((*glob()).game_state.ip_planet != 0);
        let mut moon = 1_u8;
        let state = system();
        if worldIsMoon(state, (*state).pOrbitalDesc) {
            moon = (u16::from(moon) + moonIndex(state, (*state).pOrbitalDesc)) as u8;
            moon = moon.wrapping_add(1);
        }
        (*glob()).game_state.in_orbit = moon;
    }
}

pub(super) unsafe extern "C" fn do_solar_sys_menu(menu: *mut c::MENU_STATE) -> c::BOOLEAN {
    let mut select = input::PulsedInputState.menu[KEY_MENU_SELECT] as c::BOOLEAN;
    if (*glob()).game_state.current_activity & (CHECK_ABORT | CHECK_LOAD) != 0
        || (*glob()).sis_state.crew_enlisted == c::COUNT::MAX
    {
        return 0;
    }
    if ffi::DoMenuChooser(menu, PM_STARMAP) != 0 {
        return 1;
    }
    if ffi::LastActivity == CHECK_LOAD {
        select = 1;
    }
    if select == 0 {
        return 1;
    }
    ffi::SetFlashRect(ptr::null());
    match (*menu).CurState {
        EQUIP_DEVICE => {
            select = DevicesMenu() as c::BOOLEAN;
            if (*glob()).game_state.current_activity & START_ENCOUNTER != 0 {
                return 0;
            }
        }
        CARGO => ffi::CargoMenu(),
        ROSTER => select = ffi::RosterMenu() as c::BOOLEAN,
        GAME_MENU => {
            if ffi::GameOptions() == 0 {
                return 0;
            }
        }
        STARMAP => {
            ffi::StarMap();
            if (*glob()).game_state.current_activity & CHECK_ABORT != 0 {
                return 0;
            }
            ffi::SetContext(ffi::SpaceContext);
            draw_system_transition(playerInInnerSystem());
            return 0;
        }
        NAVIGATION => return 0,
        _ => {}
    }
    if (*glob()).game_state.current_activity & CHECK_ABORT == 0 {
        if select != 0 {
            if ffi::optWhichMenu != OPT_PC {
                (*menu).CurState = NAVIGATION;
            }
            ffi::DrawMenuStateStrings(PM_STARMAP, i16::from((*menu).CurState));
        }
        ffi::SetFlashRect(usize::MAX as *const c_void);
    }
    1
}

pub(super) unsafe fn solar_sys_menu() {
    let mut menu: c::MENU_STATE = std::mem::zeroed();
    if ffi::LastActivity == CHECK_LOAD {
        menu.CurState = GAME_MENU;
    } else {
        ffi::DrawMenuStateStrings(PM_STARMAP, i16::from(STARMAP));
        menu.CurState = STARMAP;
    }
    ffi::DrawStatusMessage(ptr::null());
    ffi::SetFlashRect(usize::MAX as *const c_void);
    ffi::SetMenuSounds(MENU_SOUND_ARROWS, MENU_SOUND_SELECT);
    menu.InputFunc = Some(do_solar_sys_menu);
    DoInput(ptr::addr_of_mut!(menu).cast(), 1);
    ffi::DrawMenuStateStrings(PM_STARMAP, -i16::from(NAVIGATION));
}

pub(super) unsafe extern "C" fn do_ip_flight(state: *mut c::SOLARSYS_STATE) -> c::BOOLEAN {
    let cancel = input::PulsedInputState.menu[KEY_MENU_CANCEL];
    if (*state).InOrbit != 0 {
        enter_planet_orbit();
        ffi::SetMenuSounds(MENU_SOUND_NONE, MENU_SOUND_NONE);
        (*state).InOrbit = 0;
    } else if cancel != 0 || ffi::LastActivity == CHECK_LOAD {
        solar_sys_menu();
        ffi::SetMenuSounds(MENU_SOUND_NONE, MENU_SOUND_NONE);
    } else {
        assert!((*state).InIpFlight != 0);
        ip_frame();
        ffi::SleepThreadUntil(NEXT_TIME);
        NEXT_TIME = ffi::GetTimeCounter().wrapping_add(IP_FRAME_RATE);
    }
    ((*glob()).game_state.current_activity
        & (START_ENCOUNTER | END_INTERPLANETARY | CHECK_ABORT | CHECK_LOAD)
        == 0
        && (*glob()).sis_state.crew_enlisted != c::COUNT::MAX) as c::BOOLEAN
}
