//! Solar-system entry, orbit, saving, and menu lifecycle from `solarsys.c`.

use super::drawing::*;
use super::flight::*;
use super::*;
use crate::engine::world::planet::{PlanetDesc, SystemInfo};
use crate::engine::{battle::Cx, observe::Io, Kernel, StoryKey, World, WorldData};

use super::site::dispatch::{body, with_system};
use crate::comm::locdata::CPoint;
use crate::graphics::pixmap_abi::{GetFrameIndex, ReleaseDrawable, SetAbsFrameIndex};
use crate::mainloop::restart_menu::c_extern::PlayMusic;
use crate::math::arctan;
use crate::planets::devices::devices_menu;
use crate::planets::ffi::do_planetary_analysis_c;
use crate::sound::heart_ffi::PLRPlaying;
use crate::strings::string_table::{GetStringAddress, SetAbsStringTableIndex};
use std::ffi::c_char;
use std::mem::MaybeUninit;

#[export_name = "pSolarSysState"]
pub static mut P_SOLAR_SYS_STATE: *mut c::SOLARSYS_STATE = ptr::null_mut();

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

    pub(super) use crate::graphics::drawable_abi::SetContextFGFrame;
    pub(super) use crate::time::counter_abi::GetTimeCounter;
    pub(super) use crate::ui::border::draw_sisframe;
    pub(super) use crate::ui::settings::stop_music;
    pub(super) use crate::ui::sis::repair_sisborder;

    pub(super) use crate::graphics::drawable_abi::DestroyDrawable;

    pub(super) use crate::graphics::context_abi::SetContext;

    pub(super) use crate::graphics::context_abi::SetContextBackGroundColor;

    pub(super) use crate::graphics::gfx_common_abi::SetTransitionSource;

    pub(super) use crate::graphics::gfx_common_abi::ScreenTransition;

    pub(super) use crate::graphics::cmap_abi::FadeScreen;

    pub(super) use crate::ui::sis::SetFlashRect;

    pub(super) use crate::ui::menu::DrawMenuStateStrings;

    pub(super) use crate::ui::sis::DrawSISMessage;

    pub(super) use crate::ui::sis::DrawStatusMessage;

    pub(super) use crate::navigation::interplanetary::live::do_missions as DoMissions;

    pub(super) use crate::state::c_api::planets::get_planet_info;

    pub(super) use crate::state::c_api::planets::put_planet_info;

    pub(super) use crate::planets::orbit::live::planet_orbit_menu;

    pub(super) use crate::planets::orbit::live::FreePlanet;

    pub(super) use crate::mainloop::input_abi::live::SetMenuSounds;

    pub(super) use crate::ui::menu::do_menu_chooser;

    pub(super) use crate::planets::cargo::live::cargo_menu;

    pub(super) use crate::planets::roster::live::roster_menu;

    pub(super) use crate::front_end::gameopt::runtime::game_options;

    pub(super) use crate::planets::pstarmap::star_map;

    pub(super) use crate::graphics::gfx_common_abi::{BatchGraphics, UnbatchGraphics};
    pub(super) use crate::threading::pacing::linked::sleep_thread_until as SleepThreadUntil;
}

#[inline]
#[track_caller]
pub(super) fn glob() -> crate::engine::Projection<crate::engine::WorldData> {
    WorldData::current()
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

pub(super) unsafe fn generate_moons(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    system: *mut c::SOLARSYS_STATE,
    planet: *mut PlanetDesc,
) {
    (*SysGenRNG).reseed((*planet).rand_seed);
    let target = body(system, planet);
    with_system(cx, io, system, |generator, system, host| {
        generator.generate_name(system, target, host);
        generator.generate_moons(system, target.planet() as u8, host);
    });

    let angle = arctan((*planet).location.x, (*planet).location.y);
    let _facing = (((u32::from(angle) + 2) >> 2) & 15) as c::COUNT;
    for i in 0..(*system).MoonDesc.len() {
        let moon = (*system).MoonDesc.as_mut_ptr().add(i);
        (*moon).prev_desc = planet;
        if i >= usize::from((*planet).num_planets) {
            continue;
        }
        (*moon).temp_color = (*planet).temp_color;
    }
}

pub(super) unsafe fn init_solar_sys_sis_characteristics() {
    let state = system();
    let mut num_thrusters = 0_u8;
    for i in 0..11 {
        if glob().read(|owner| owner.flagship.drive_slots[i]) == FUSION_THRUSTER {
            num_thrusters += 1;
        }
    }
    (*state).max_ship_speed = (num_thrusters + 5) * IP_SHIP_THRUST_INCREMENT;
    (*state).turn_wait = IP_SHIP_TURN_WAIT;
    for i in 0..8 {
        if glob().read(|owner| owner.flagship.jet_slots[i]) == TURNING_JETS {
            (*state).turn_wait = (*state).turn_wait.wrapping_sub(IP_SHIP_TURN_DECREMENT);
        }
    }
}

fn temperature_color(temperature: i16) -> c::Color {
    const TEMP_COLORS: [c::Color; 5] = [
        rgb15(0x00, 0x00, 0x0e),
        rgb15(0x00, 0x06, 0x08),
        rgb15(0x00, 0x0b, 0x00),
        rgb15(0x0f, 0x00, 0x00),
        rgb15(0x0f, 0x08, 0x00),
    ];
    let mut index = ((i32::from(temperature) + 250) / 100) as c::COUNT;
    if usize::from(index) >= TEMP_COLORS.len() {
        index = (TEMP_COLORS.len() - 1) as c::COUNT;
    }
    TEMP_COLORS[usize::from(index)]
}

pub(super) unsafe fn load_solar_sys(cx: &mut Cx<'_>, io: &mut Io<'_>) -> *mut PlanetDesc {
    let state = system();
    let mut orbital = ptr::null_mut();
    let star = *cx.world.star_pointer();
    (*SysGenRNG).reseed(GetRandomSeedForStar(ptr::from_ref(&star).cast()));
    let sun_type = (star.star_type & 0x18) >> 3;
    SUN_FRAME = SetAbsFrameIndex(SUN_FRAME.cast(), u16::from(sun_type)).cast();

    let sun = (*state).SunDesc.as_mut_ptr();
    (*sun).prev_desc = ptr::null_mut();
    (*sun).rand_seed = (*SysGenRNG).next_random();
    (*sun).data_index = sun_type;
    (*sun).location = c::POINT { x: 0, y: 0 };
    (*sun).image.origin = (*sun).location;
    (*sun).image.frame = SUN_FRAME;

    with_system(cx, io, state, |generator, system, host| {
        generator.generate_planets(system, host)
    });
    if cx.world.story_state(StoryKey::PLANETARY_CHANGE) != 0 {
        ffi::put_planet_info(cx.world);
        cx.world.set_story_state(StoryKey::PLANETARY_CHANGE, 0);
    }

    for i in 0..(*state).PlanetDesc.len() {
        let planet = (*state).PlanetDesc.as_mut_ptr().add(i);
        (*planet).prev_desc = sun;
        (*planet).image.origin = (*planet).location;
        if i >= usize::from((*sun).num_planets) {
            (*planet).image.frame = ptr::null_mut();
        } else {
            // The Rust analysis implementation borrows the whole SystemInfo.
            let mut info: SystemInfo = std::mem::zeroed();
            cx.with_legacy(|| do_planetary_analysis_c(ptr::addr_of_mut!(info), planet));
            (*planet).temp_color = temperature_color(info.planet_info.temperature);
        }
    }
    cx.with_legacy(|| sort_planet_positions());

    if cx.world.data.game.ip_planet == 0 {
        (*state).pBaseDesc = (*state).PlanetDesc.as_mut_ptr();
        (*state).pOrbitalDesc = ptr::null_mut();
    } else {
        (*sun).location = c_point(cx.world.data.game.ip_location);
        cx.world.data.game.ip_location = saved_point(displayToLocation(
            c_point(cx.world.data.game.ship_stamp.origin),
            MAX_ZOOM_RADIUS,
        ));
        let i = usize::from(cx.world.data.game.ip_planet - 1);
        (*state).pOrbitalDesc = (*state).PlanetDesc.as_mut_ptr().add(i);
        generate_moons(cx, io, state, (*state).pOrbitalDesc);
        (*state).pBaseDesc = (*state).MoonDesc.as_mut_ptr();
        cx.world.set_story_state(StoryKey::PLANETARY_LANDING, 0);
    }

    cx.with_legacy(|| init_solar_sys_sis_characteristics());
    if cx.world.data.game.in_orbit != 0 {
        let i = usize::from(cx.world.data.game.in_orbit - 1);
        orbital = if i == 0 {
            (*(*state).pBaseDesc).prev_desc
        } else {
            (*state).MoonDesc.as_mut_ptr().add(i - 1)
        };
        cx.world.data.game.ip_location = saved_point((*sun).location);
        cx.world.data.game.in_orbit = 0;
    } else {
        let mut i = cx.world.data.game.ship_facing;
        if i == 0 {
            i += 1;
        }
        cx.world.data.game.ship_stamp.frame = SetAbsFrameIndex(SISIPFRAME.cast(), i - 1).cast();
    }
    orbital
}

pub(super) unsafe fn save_non_orbital_location() {
    (**glob()).game.ship_facing =
        GetFrameIndex(glob().read(|owner| owner.game.ship_stamp.frame).cast()).wrapping_add(1);
    (**glob()).game.in_orbit = 0;
    if !playerInInnerSystem() {
        (**glob()).game.ip_planet = 0;
    } else {
        let state = system();
        (**glob()).game.ip_planet = (1 + planetIndex(state, (*state).pOrbitalDesc)) as u8;
        (**glob()).game.ip_location = saved_point((*state).SunDesc[0].location);
    }
}

pub(super) unsafe fn free_solar_sys() {
    let state = system();
    if (*state).InIpFlight != 0 {
        (*state).InIpFlight = 0;
        if World::current().read(|world| world.activity()) & (CHECK_ABORT | CHECK_LOAD) == 0 {
            save_non_orbital_location();
        }
    }
    ffi::DestroyDrawable(ReleaseDrawable(SOLAR_SYS_FRAME.cast()).cast());
    SOLAR_SYS_FRAME = ptr::null_mut();
    ffi::stop_music();
}

pub(super) unsafe fn reset_solar_sys(next_activity: u16) {
    let state = system();
    assert!((*state).InIpFlight == 0);
    ffi::DrawMenuStateStrings(PM_STARMAP, -(PM_NAVIGATE - PM_SCAN));
    crate::app::bridge::enter(|cx| {
        crate::battle::process_native::init_display_list(
            cx,
            cx.kernel.configuration.preferences().melee_scale == 0,
        );
    });
    ffi::DoMissions();
    check_intersect();
    (*state).InIpFlight = 1;
    if PLRPlaying(usize::MAX as *mut c_void) == 0
        && (Kernel::current().read(|owner| owner.last_activity) != CHECK_LOAD || next_activity != 0)
    {
        PlayMusic((SpaceMusic).cast(), 1, 1);
    }
}

enum OrbitAdmission {
    Interrupted,
    Ready(Option<crate::planets::orbit::presentation::OrbitPresentation>),
}

unsafe fn run_planet_orbit(cx: &mut Cx<'_>, io: &mut Io<'_>) {
    let OrbitAdmission::Ready(presentation) = prepare_planet_orbit(cx, io) else {
        return;
    };
    if let Some(presentation) = presentation {
        ffi::planet_orbit_menu(cx, io, &presentation);
        cx.with_legacy(|| ffi::FreePlanet());
    }
    finish_planet_orbit(cx, io);
}

unsafe fn prepare_planet_orbit(cx: &mut Cx<'_>, io: &mut Io<'_>) -> OrbitAdmission {
    let state = system();
    cx.with_legacy(|| {
        if (*state).InIpFlight != 0 {
            free_solar_sys();
            if worldIsMoon(state, (*state).pOrbitalDesc) {
                (**glob()).game.ship_stamp.origin =
                    saved_point((*(*state).pOrbitalDesc).image.origin);
            } else {
                let (width, height) = sis_dimensions();
                (**glob()).game.ship_stamp.origin.x = (width >> 1) as _;
                (**glob()).game.ship_stamp.origin.y = (height >> 1) as _;
            }
        }

        World::current().write(|world| ffi::get_planet_info(world));
    });
    let target = body(state, (*state).pOrbitalDesc);
    let (_, presentation) = with_system(cx, io, state, |generator, system, host| {
        generator.generate_orbital(system, target, host)
    });
    cx.with_legacy(|| {
        (**Kernel::current()).last_activity &= !(CHECK_LOAD | CHECK_RESTART);
        if World::current().read(|world| world.activity())
            & (CHECK_ABORT | CHECK_LOAD | START_ENCOUNTER)
            != 0
            || glob().read(|owner| owner.flagship.crew_enlisted) == c::COUNT::MAX
            || World::current().read(|world| world.story_state(StoryKey::CHMMR_BOMB_STATE)) == 2
        {
            return OrbitAdmission::Interrupted;
        }
        OrbitAdmission::Ready(presentation)
    })
}

unsafe fn finish_planet_orbit(cx: &mut Cx<'_>, io: &mut Io<'_>) {
    if cx.world.activity() & (START_ENCOUNTER | CHECK_ABORT | CHECK_LOAD) == 0
        && cx.world.data.flagship.crew_enlisted != c::COUNT::MAX
    {
        let space_context = cx.kernel.space_context;
        let next_activity = cx.world.next_activity;
        let orbital = load_solar_sys(cx, io);
        assert!(orbital.is_null());
        let inner = cx.with_legacy(|| {
            check_zoom_level();
            validate_orbits();
            validate_inner_orbits();
            reset_solar_sys(next_activity);
            ffi::repair_sisborder();
            ffi::SetContext(space_context);
            playerInInnerSystem()
        });
        draw_system_transition(cx, inner);
    }
}

enum SolarSystemStart {
    Orbit,
    Fade,
    ResumeLoad,
    Transition,
}

unsafe fn start_solar_sys(cx: &mut Cx<'_>, io: &mut Io<'_>) {
    let next_activity = cx.world.next_activity;
    let start = prepare_solar_sys(cx, io, next_activity);
    match start {
        SolarSystemStart::Orbit => {}
        SolarSystemStart::ResumeLoad => cx.with_legacy(|| ffi::UnbatchGraphics()),
        SolarSystemStart::Fade => {
            crate::battle::process_native::engine::redraw_queue(cx, false);
            cx.with_legacy(|| {
                ffi::UnbatchGraphics();
                ffi::FadeScreen(FADE_ALL_TO_COLOR, (ONE_SECOND / 2) as c::SIZE);
            });
            cx.kernel.last_activity = 0;
        }
        SolarSystemStart::Transition => {
            crate::battle::process_native::engine::redraw_queue(cx, false);
            cx.with_legacy(|| {
                ffi::ScreenTransition(3, ptr::null());
                ffi::UnbatchGraphics();
            });
            cx.kernel.last_activity &= !CHECK_LOAD;
        }
    }
}

unsafe fn prepare_solar_sys(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    next_activity: u16,
) -> SolarSystemStart {
    cx.with_legacy(|| super::resources::LoadIPData());
    if cx.kernel.lander_resources.is_none() {
        cx.kernel.lander_resources = Some(
            crate::engine::kernel::lander_resources::LanderResources::load()
                .unwrap_or_else(|key| panic!("missing lander resource {key}")),
        );
    }
    let (state, reentry) = cx.with_legacy(|| {
        let state = system();
        let reentry = glob().read(|owner| owner.game.ship_facing) != 0;
        if !reentry {
            (**glob()).game.autopilot = CPoint { x: -1, y: -1 };
            let (width, height) = sis_dimensions();
            (**glob()).game.ship_stamp.origin.x = (width >> 1) as _;
            (**glob()).game.ship_stamp.origin.y = (height - 2) as _;
            (**glob()).game.ip_location = saved_point(displayToLocation(
                c_point(glob().read(|owner| owner.game.ship_stamp.origin)),
                MAX_ZOOM_RADIUS,
            ));
        }

        STARS_FRAME = create_star_background();
        ffi::SetContext(Kernel::current().read(|owner| owner.space_context));
        ffi::SetContextFGFrame(Kernel::current().read(|owner| owner.screen).cast());
        ffi::SetContextBackGroundColor(rgb15(0, 0, 0));

        (state, reentry)
    });
    let orbital = load_solar_sys(cx, io);
    let inner_system = cx.with_legacy(|| {
        let inner_system = check_zoom_level() != 0;
        validate_orbits();
        if inner_system {
            validate_inner_orbits();
        }
        inner_system
    });
    if !reentry {
        let world = &mut *cx.world;
        world.encounter_race = -1;
        EncounterGroup = 0;
        world.data.game.battle_group_ref = 0;
        world.ip_groups().reinit();
        world.npc_built_ships().reinit();
    }
    with_system(cx, io, state, |generator, system, host| {
        if reentry {
            generator.reinit_npcs(system, host);
        } else {
            generator.init_npcs(system, host);
        }
    });
    cx.with_legacy(|| {
        if !orbital.is_null() {
            enter_orbital(orbital);
            SolarSystemStart::Orbit
        } else {
            ffi::SetContext(Kernel::current().read(|owner| owner.space_context));
            ffi::SetTransitionSource(ptr::null_mut());
            ffi::BatchGraphics();
            ffi::draw_sisframe();
            ffi::DrawSISMessage(ptr::null());
            reset_solar_sys(next_activity);
            if Kernel::current().read(|owner| owner.last_activity) == CHECK_LOAD | CHECK_RESTART {
                draw_outer_system();
                SolarSystemStart::Fade
            } else if Kernel::current().read(|owner| owner.last_activity) == CHECK_LOAD
                && next_activity == 0
            {
                SolarSystemStart::ResumeLoad
            } else {
                if inner_system {
                    draw_inner_system();
                } else {
                    draw_outer_system();
                }
                SolarSystemStart::Transition
            }
        }
    })
}

pub(super) unsafe fn end_interplanetary(cx: &mut Cx<'_>, io: &mut Io<'_>) {
    cx.world
        .set_activity(cx.world.activity() & !END_INTERPLANETARY);
    if cx.world.activity() & (CHECK_ABORT | CHECK_LOAD) == 0 {
        with_system(cx, io, system(), |generator, system, host| {
            generator.uninit_npcs(system, host)
        });
        cx.world.set_story_state(StoryKey::USED_BROADCASTER, 0);
    }
}

pub(super) unsafe fn closest_planet_interplanetary(point: *const c::POINT) -> *mut PlanetDesc {
    let state = system();
    assert!(!state.is_null());
    let num_planets = (*state).SunDesc[0].num_planets;
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

pub(super) unsafe fn uninit_solar_sys(cx: &mut Cx<'_>, io: &mut Io<'_>) {
    cx.with_legacy(|| {
        free_solar_sys();
        ffi::DestroyDrawable(ReleaseDrawable(STARS_FRAME.cast()).cast());
        STARS_FRAME = ptr::null_mut();
    });
    if cx.world.activity() & END_INTERPLANETARY != 0 {
        end_interplanetary(cx, io);
        return;
    }
    if cx.world.activity() & START_ENCOUNTER != 0 && EncounterGroup != 0 {
        {
            let world = &mut *cx.world;
            crate::navigation::groups::live::get_group_info(
                world,
                world.data.game.battle_group_ref,
                EncounterGroup,
            )
        };
        if cx.world.data.game.ip_planet == 0 {
            let point = c_point(cx.world.data.game.ship_stamp.origin);
            let planet = cx.with_legacy(|| closest_planet_interplanetary(&point));
            let state = system();
            let target = body(state, planet);
            with_system(cx, io, state, |generator, system, host| {
                generator.generate_name(system, target, host)
            });
        }
    }
}

#[inline]
pub(super) unsafe fn hyperspace_unit_factors() -> (i32, i32) {
    let (width, height) = dimensions();
    ((((width - 64) << 5) >> 4) * 195, ((height << 5) >> 4) * 25)
}

/// # Safety
/// Call on the game thread with campaign and solar-system globals initialized;
/// their pointers must remain valid through any input or generator callbacks.
pub unsafe fn run_explore_solar_sys(cx: &mut Cx<'_>, io: &mut Io<'_>) {
    let runtime = cx.kernel.input_runtime();
    let input = runtime.input();
    let mut solar_sys_state = MaybeUninit::<c::SOLARSYS_STATE>::uninit();
    cx.with_legacy(|| prepare_explore_solar_sys(solar_sys_state.as_mut_ptr()));
    start_solar_sys(cx, io);
    cx.with_legacy(|| ffi::SetMenuSounds(input, MENU_SOUND_NONE, MENU_SOUND_NONE));
    cx.run_input(io, false, |cx, io| {
        do_ip_flight(cx, io, solar_sys_state.as_mut_ptr()) != 0
    });
    uninit_solar_sys(cx, io);
    cx.with_legacy(|| {
        P_SOLAR_SYS_STATE = ptr::null_mut();
    });
}

unsafe fn prepare_explore_solar_sys(state: *mut c::SOLARSYS_STATE) {
    if (**World::current()).star_pointer().is_null() {
        let (log_units_x, log_units_y) = hyperspace_unit_factors();
        let universe = c::POINT {
            x: ((glob().read(|owner| owner.flagship.log_x) * 6250 + (log_units_x >> 1))
                / log_units_x) as _,
            y: (9999
                - (glob().read(|owner| owner.flagship.log_y) * 625 + (log_units_y >> 1))
                    / log_units_y) as _,
        };
        World::current().write(|world| {
            world.current_star = crate::planets::starmap::find_star_id(
                world,
                world.catalog(),
                None,
                crate::graphics::native_abi::POINT {
                    x: universe.x,
                    y: universe.y,
                },
                1,
                1,
            );
        });
        if (**World::current()).star_pointer().is_null() {
            crate::log_abi::log_message(1, "ExploreSolarSys(): do not know where you are!");
            #[cfg(feature = "debug-process")]
            libc::abort();
            #[cfg(not(feature = "debug-process"))]
            libc::exit(libc::EXIT_FAILURE);
        }
    }
    let (log_units_x, log_units_y) = hyperspace_unit_factors();
    World::current().write(|world| {
        let star = &*world.star_pointer();
        world.data.flagship.log_x = (i32::from(star.star_pt.x) * log_units_x + (6250 >> 1)) / 6250;
        world.data.flagship.log_y =
            ((9999 - i32::from(star.star_pt.y)) * log_units_y + (625 >> 1)) / 625;
    });

    P_SOLAR_SYS_STATE = state;
    ptr::write_bytes(P_SOLAR_SYS_STATE, 0, 1);
    let index = World::current().read(|world| (*world.star_pointer()).index);
    (*P_SOLAR_SYS_STATE).generator_index = index;
}

pub(super) unsafe fn game_string(index: c::COUNT) -> *mut c_char {
    GetStringAddress(SetAbsStringTableIndex(
        Kernel::current().read(|owner| owner.game_strings).cast(),
        index,
    ))
}

#[no_mangle]
pub unsafe extern "C" fn GetNamedPlanetaryBody() -> *mut c::UNICODE {
    if (**World::current()).star_pointer().is_null()
        || !playerInSolarSystem()
        || !playerInInnerSystem()
    {
        return ptr::null_mut();
    }
    let state = system();
    let orbital = (*state).pOrbitalDesc;
    assert!(!orbital.is_null());
    match (*(**World::current()).star_pointer()).index {
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
/// # Safety
/// Call on the game thread with campaign and solar-system globals initialized;
/// their pointers must remain valid through any input or generator callbacks.
pub unsafe extern "C" fn SaveSolarSysLocation() {
    assert!(playerInSolarSystem());
    if !playerInPlanetOrbit() {
        save_non_orbital_location();
    } else {
        if World::current().read(|world| world.story_state(StoryKey::PLANETARY_CHANGE)) != 0 {
            World::current().write(|world| ffi::put_planet_info(world));
            World::current().write(|world| world.set_story_state(StoryKey::PLANETARY_CHANGE, 0));
        }
        assert!((**glob()).game.ip_planet != 0);
        let mut moon = 1_u8;
        let state = system();
        if worldIsMoon(state, (*state).pOrbitalDesc) {
            moon = (u16::from(moon) + moonIndex(state, (*state).pOrbitalDesc)) as u8;
            moon = moon.wrapping_add(1);
        }
        (**glob()).game.in_orbit = moon;
    }
}

unsafe fn do_solar_sys_menu(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    menu: &mut c::MENU_STATE,
) -> c::BOOLEAN {
    let runtime = cx.kernel.input_runtime();
    let input = runtime.input();
    let mut select = input.pulsed().menu[KEY_MENU_SELECT] as c::BOOLEAN;
    if cx.world.activity() & (CHECK_ABORT | CHECK_LOAD) != 0
        || cx.world.data.flagship.crew_enlisted == c::COUNT::MAX
    {
        return 0;
    }
    if cx.with_legacy(|| ffi::do_menu_chooser(input, menu, PM_STARMAP)) != 0 {
        return 1;
    }
    if cx.kernel.last_activity == CHECK_LOAD {
        select = 1;
    }
    if select == 0 {
        return 1;
    }
    cx.with_legacy(|| ffi::SetFlashRect(ptr::null()));
    match menu.CurState {
        EQUIP_DEVICE => {
            select = devices_menu(cx, io) as c::BOOLEAN;
            if cx.world.activity() & START_ENCOUNTER != 0 {
                return 0;
            }
        }
        CARGO => ffi::cargo_menu(cx, io),
        ROSTER => select = ffi::roster_menu(cx, io) as c::BOOLEAN,
        GAME_MENU => {
            if ffi::game_options(cx, io) == 0 {
                return 0;
            }
        }
        STARMAP => {
            ffi::star_map(cx, io);
            if cx.world.activity() & CHECK_ABORT != 0 {
                return 0;
            }
            let space_context = cx.kernel.space_context;
            let inner = cx.with_legacy(|| {
                ffi::SetContext(space_context);
                playerInInnerSystem()
            });
            draw_system_transition(cx, inner);
            return 0;
        }
        NAVIGATION => return 0,
        _ => {}
    }
    if cx.world.activity() & CHECK_ABORT == 0 {
        if select != 0 {
            if input.preferences().menu != OPT_PC {
                menu.CurState = NAVIGATION;
            }
            cx.with_legacy(|| ffi::DrawMenuStateStrings(PM_STARMAP, i16::from(menu.CurState)));
        }
        cx.with_legacy(|| ffi::SetFlashRect((usize::MAX as *const c_void).cast()));
    }
    1
}

unsafe fn solar_sys_menu(cx: &mut Cx<'_>, io: &mut Io<'_>) {
    let runtime = cx.kernel.input_runtime();
    let input = runtime.input();
    let mut menu: c::MENU_STATE = std::mem::zeroed();
    cx.with_legacy(|| {
        if Kernel::current().read(|owner| owner.last_activity) == CHECK_LOAD {
            menu.CurState = GAME_MENU;
        } else {
            ffi::DrawMenuStateStrings(PM_STARMAP, i16::from(STARMAP));
            menu.CurState = STARMAP;
        }
        ffi::DrawStatusMessage(ptr::null());
        ffi::SetFlashRect((usize::MAX as *const c_void).cast());
        ffi::SetMenuSounds(input, MENU_SOUND_ARROWS, MENU_SOUND_SELECT);
    });
    cx.run_input(io, true, |cx, io| do_solar_sys_menu(cx, io, &mut menu) != 0);
    cx.with_legacy(|| ffi::DrawMenuStateStrings(PM_STARMAP, -i16::from(NAVIGATION)));
}

unsafe fn do_ip_flight(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    state: *mut c::SOLARSYS_STATE,
) -> c::BOOLEAN {
    let runtime = cx.kernel.input_runtime();
    let input = runtime.input();
    let cancel = input.pulsed().menu[KEY_MENU_CANCEL];
    if (*state).InOrbit != 0 {
        run_planet_orbit(cx, io);
        cx.with_legacy(|| ffi::SetMenuSounds(input, MENU_SOUND_NONE, MENU_SOUND_NONE));
        (*state).InOrbit = 0;
    } else if cancel != 0 || cx.kernel.last_activity == CHECK_LOAD {
        solar_sys_menu(cx, io);
        cx.with_legacy(|| ffi::SetMenuSounds(input, MENU_SOUND_NONE, MENU_SOUND_NONE));
    } else {
        assert!((*state).InIpFlight != 0);
        ip_frame(cx, io);
        cx.with_legacy(|| {
            ffi::SleepThreadUntil(NEXT_TIME);
            NEXT_TIME = ffi::GetTimeCounter().wrapping_add(IP_FRAME_RATE);
        });
    }
    (cx.world.activity() & (START_ENCOUNTER | END_INTERPLANETARY | CHECK_ABORT | CHECK_LOAD) == 0
        && cx.world.data.flagship.crew_enlisted != c::COUNT::MAX) as c::BOOLEAN
}
