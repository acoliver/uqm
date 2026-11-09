//! Solar-system entry, orbit, saving, and menu lifecycle from `solarsys.c`.

use super::drawing::*;
use super::flight::*;
use super::*;
use crate::engine::{battle::Cx, observe::Io, Kernel, StoryKey, World, WorldData};

use super::site::dispatch::invoke;
use crate::comm::locdata::CPoint;
use crate::engine::world::generation::Body;
use crate::graphics::pixmap_abi::{GetFrameIndex, ReleaseDrawable, SetAbsFrameIndex};
use crate::planets::devices::devices_menu;
use crate::planets::generate::calc::preview_body;
use crate::planets::solar_system::{SolarSystem, SystemLocation};
use crate::sound::heart_ffi::PLRPlaying;

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
const SAMATRA_DEFINED: u8 = 29;
const PLANET_NUMBER_BASE: u16 = 375;
const STARBASE_STRING_BASE: u16 = 422;

static mut NEXT_TIME: u32 = 0;

mod ffi {

    pub(super) use crate::graphics::drawable_abi::SetContextFGFrame;
    pub(super) use crate::time::counter_abi::GetTimeCounter;
    pub(super) use crate::ui::settings::stop_music;

    pub(super) use crate::graphics::drawable_abi::DestroyDrawable;

    pub(super) use crate::graphics::context_abi::SetContext;

    pub(super) use crate::graphics::context_abi::SetContextBackGroundColor;

    pub(super) use crate::graphics::gfx_common_abi::SetTransitionSource;

    pub(super) use crate::graphics::gfx_common_abi::ScreenTransition;

    pub(super) use crate::graphics::cmap_abi::FadeScreen;

    pub(super) use crate::navigation::interplanetary::live::do_missions as DoMissions;

    pub(super) use crate::state::c_api::planets::get_planet_info;

    pub(super) use crate::state::c_api::planets::put_planet_info;

    pub(super) use crate::planets::orbit::live::planet_orbit_menu;

    pub(super) use crate::planets::orbit::live::free_planet;

    pub(super) use crate::mainloop::input_abi::live::SetMenuSounds;

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

pub(super) fn generate_moons(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    solar: &mut SolarSystem,
    planet: u8,
) {
    solar
        .sys
        .rng
        .reseed(solar.sys.planets[usize::from(planet)].rand_seed);
    invoke(cx, io, solar, |generator, system, host| {
        generator.generate_name(system, Body::Planet(planet), host);
        generator.generate_moons(system, planet, host);
    });
    let parent = &solar.sys.planets[usize::from(planet)];
    for moon in &mut solar.sys.moons[..usize::from(parent.num_planets)] {
        moon.temp_color = parent.temp_color;
    }
}

pub(super) fn init_solar_sys_sis_characteristics(world: &World, solar: &mut SolarSystem) {
    let mut num_thrusters = 0_u8;
    for i in 0..11 {
        if world.data.flagship.drive_slots[i] == FUSION_THRUSTER {
            num_thrusters += 1;
        }
    }
    solar.max_ship_speed = (num_thrusters + 5) * IP_SHIP_THRUST_INCREMENT;
    solar.turn_wait = IP_SHIP_TURN_WAIT;
    for i in 0..8 {
        if world.data.flagship.jet_slots[i] == TURNING_JETS {
            solar.turn_wait = solar.turn_wait.wrapping_sub(IP_SHIP_TURN_DECREMENT);
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

pub(super) unsafe fn load_solar_sys(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    solar: &mut SolarSystem,
) -> Option<Body> {
    let star = cx.world.catalog()[solar.star().0];
    solar
        .sys
        .rng
        .reseed(GetRandomSeedForStar(ptr::from_ref(&star).cast()));
    let sun_type = (star.star_type & 0x18) >> 3;
    SUN_FRAME = SetAbsFrameIndex(SUN_FRAME.cast(), u16::from(sun_type)).cast();
    solar.sys.sun.rand_seed = solar.sys.rng.next_random();
    solar.sys.sun.data_index = sun_type;
    solar.sys.sun.location = c::POINT { x: 0, y: 0 };
    solar.sys.sun.image.origin = solar.sys.sun.location;
    solar.sys.sun.image.frame = SUN_FRAME;

    invoke(cx, io, solar, |generator, system, host| {
        generator.generate_planets(system, host)
    });
    if cx.world.story_state(StoryKey::PLANETARY_CHANGE) != 0 {
        ffi::put_planet_info(cx.world, solar);
        cx.world.set_story_state(StoryKey::PLANETARY_CHANGE, 0);
    }
    for i in 0..solar.sys.planets.len() {
        solar.sys.planets[i].image.origin = solar.sys.planets[i].location;
        if i >= usize::from(solar.sys.sun.num_planets) {
            solar.sys.planets[i].image.frame = ptr::null_mut();
        } else {
            let info = invoke(cx, io, solar, |_, system, _| {
                preview_body(system, Body::Planet(i as u8))
            })
            .0;
            solar.sys.planets[i].temp_color = temperature_color(info.planet_info.temperature);
        }
    }
    sort_planet_positions(solar);

    if cx.world.data.game.ip_planet == 0 {
        solar.leave_inner();
    } else {
        solar.sys.sun.location = c_point(cx.world.data.game.ip_location);
        cx.world.data.game.ip_location = saved_point(displayToLocation(
            c_point(cx.world.data.game.ship_stamp.origin),
            MAX_ZOOM_RADIUS,
        ));
        let planet = cx.world.data.game.ip_planet - 1;
        solar.enter_inner(planet);
        generate_moons(cx, io, solar, planet);
        cx.world.set_story_state(StoryKey::PLANETARY_LANDING, 0);
    }
    init_solar_sys_sis_characteristics(cx.world, solar);
    if cx.world.data.game.in_orbit != 0 {
        let SystemLocation::Inner(planet) = solar.location() else {
            panic!("saved orbit requires an inner system");
        };
        let code = cx.world.data.game.in_orbit;
        let body = if code == 1 {
            Body::Planet(planet)
        } else {
            Body::Moon {
                planet,
                moon: code - 2,
            }
        };
        cx.world.data.game.ip_location = saved_point(solar.sys.sun.location);
        cx.world.data.game.in_orbit = 0;
        Some(body)
    } else {
        let mut i = cx.world.data.game.ship_facing;
        if i == 0 {
            i += 1;
        }
        cx.world.data.game.ship_stamp.frame = SetAbsFrameIndex(SISIPFRAME.cast(), i - 1).cast();
        None
    }
}

pub(super) unsafe fn save_non_orbital_location(world: &mut World, solar: &SolarSystem) {
    let game = &mut world.data.game;
    game.ship_facing = GetFrameIndex(game.ship_stamp.frame.cast()).wrapping_add(1);
    game.in_orbit = 0;
    match solar.location() {
        SystemLocation::Outer => game.ip_planet = 0,
        SystemLocation::Inner(planet) => {
            game.ip_planet = planet + 1;
            game.ip_location = saved_point(solar.sys.sun.location);
        }
    }
}

pub(super) unsafe fn free_solar_sys(cx: &mut Cx<'_>, solar: &mut SolarSystem) {
    if solar.in_ip_flight {
        solar.in_ip_flight = false;
        if cx.world.activity() & (CHECK_ABORT | CHECK_LOAD) == 0 {
            save_non_orbital_location(cx.world, solar);
        }
    }
    cx.with_legacy(|| {
        ffi::DestroyDrawable(ReleaseDrawable(SOLAR_SYS_FRAME.cast()).cast());
        SOLAR_SYS_FRAME = ptr::null_mut();
        ffi::stop_music();
    });
}

pub(super) unsafe fn reset_solar_sys(cx: &mut Cx<'_>, solar: &mut SolarSystem, next_activity: u16) {
    assert!(!solar.in_ip_flight);
    solar.refresh_frame(cx);
    let dimensions = cx.screen_dimensions();
    crate::ui::menu::draw_menu_state(
        cx.kernel,
        cx.world,
        (dimensions.0, dimensions.1),
        PM_STARMAP,
        -(PM_NAVIGATE - PM_SCAN),
    );
    let music_enabled = cx.world.data.game.glob_flags & 8 == 0;
    crate::battle::process_native::init_display_list(
        cx,
        cx.kernel.configuration.preferences().melee_scale == 0,
    );
    ffi::DoMissions(cx);
    cx.with_legacy(|| {
        check_intersect(solar);
        solar.in_ip_flight = true;
        if PLRPlaying(usize::MAX as *mut c_void) == 0
            && (Kernel::current().read(|owner| owner.last_activity) != CHECK_LOAD
                || next_activity != 0)
        {
            crate::sound::music::remember_native_music(
                std::ptr::NonNull::new(SpaceMusic.cast()),
                true,
                1,
                music_enabled,
            );
        }
    });
}

enum OrbitAdmission {
    Interrupted,
    Ready(Option<crate::planets::orbit::presentation::OrbitPresentation>),
}

unsafe fn run_planet_orbit(cx: &mut Cx<'_>, io: &mut Io<'_>, solar: &mut SolarSystem) {
    let OrbitAdmission::Ready(presentation) = prepare_planet_orbit(cx, io, solar) else {
        return;
    };
    if let Some(presentation) = presentation {
        ffi::planet_orbit_menu(cx, io, solar, &presentation);
        ffi::free_planet(cx, solar);
    }
    finish_planet_orbit(cx, io, solar);
}

unsafe fn prepare_planet_orbit(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    solar: &mut SolarSystem,
) -> OrbitAdmission {
    let target = solar.require_body();
    if solar.in_ip_flight {
        free_solar_sys(cx, solar);
        if matches!(target, Body::Moon { .. }) {
            cx.world.data.game.ship_stamp.origin =
                saved_point(solar.sys.descriptor(target).image.origin);
        } else {
            let (width, height) = sis_dimensions();
            cx.world.data.game.ship_stamp.origin = CPoint {
                x: (width >> 1) as _,
                y: (height >> 1) as _,
            };
        }
    }
    ffi::get_planet_info(cx.world, solar);
    let (_, presentation) = invoke(cx, io, solar, |generator, system, host| {
        generator.generate_orbital(system, target, host)
    });
    cx.kernel.last_activity &= !(CHECK_LOAD | CHECK_RESTART);
    if cx.world.activity() & (CHECK_ABORT | CHECK_LOAD | START_ENCOUNTER) != 0
        || cx.world.data.flagship.crew_enlisted == c::COUNT::MAX
        || cx.world.story_state(StoryKey::CHMMR_BOMB_STATE) == 2
    {
        OrbitAdmission::Interrupted
    } else {
        OrbitAdmission::Ready(presentation)
    }
}

unsafe fn finish_planet_orbit(cx: &mut Cx<'_>, io: &mut Io<'_>, solar: &mut SolarSystem) {
    if cx.world.activity() & (START_ENCOUNTER | CHECK_ABORT | CHECK_LOAD) == 0
        && cx.world.data.flagship.crew_enlisted != c::COUNT::MAX
    {
        let space_context = cx.kernel.space_context;
        let next_activity = cx.world.next_activity;
        let orbital = load_solar_sys(cx, io, solar);
        assert!(orbital.is_none());
        cx.with_legacy(|| {
            check_zoom_level(solar);
            validate_orbits(solar);
            validate_inner_orbits(solar);
        });
        reset_solar_sys(cx, solar, next_activity);
        let dimensions = cx.screen_dimensions();
        crate::ui::sis::repair_sis_border(cx.kernel, dimensions);
        let inner = cx.with_legacy(|| {
            ffi::SetContext(space_context);
            matches!(solar.location(), SystemLocation::Inner(_))
        });
        draw_system_transition(cx, solar, inner);
    }
}

enum SolarSystemStart {
    Orbit,
    Fade,
    ResumeLoad,
    Transition,
}

unsafe fn start_solar_sys(cx: &mut Cx<'_>, io: &mut Io<'_>, solar: &mut SolarSystem) {
    let next_activity = cx.world.next_activity;
    let start = prepare_solar_sys(cx, io, solar, next_activity);
    solar.refresh_frame(cx);
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
    solar: &mut SolarSystem,
    next_activity: u16,
) -> SolarSystemStart {
    cx.with_legacy(|| super::resources::LoadIPData());
    if cx.kernel.lander_resources.is_none() {
        cx.kernel.lander_resources = Some(
            crate::engine::kernel::lander_resources::LanderResources::load()
                .unwrap_or_else(|key| panic!("missing lander resource {key}")),
        );
    }
    let reentry = cx.with_legacy(|| {
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

        STARS_FRAME = create_star_background(solar);
        ffi::SetContext(Kernel::current().read(|owner| owner.space_context));
        ffi::SetContextFGFrame(Kernel::current().read(|owner| owner.screen).cast());
        ffi::SetContextBackGroundColor(rgb15(0, 0, 0));

        reentry
    });
    let orbital = load_solar_sys(cx, io, solar);
    let inner_system = cx.with_legacy(|| {
        let inner_system = check_zoom_level(solar) != 0;
        validate_orbits(solar);
        if inner_system {
            validate_inner_orbits(solar);
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
    invoke(cx, io, solar, |generator, system, host| {
        if reentry {
            generator.reinit_npcs(system, host);
        } else {
            generator.init_npcs(system, host);
        }
    });
    if let Some(body) = orbital {
        enter_orbital(cx, solar, body);
        SolarSystemStart::Orbit
    } else {
        cx.with_legacy(|| {
            ffi::SetContext(Kernel::current().read(|owner| owner.space_context));
            ffi::SetTransitionSource(ptr::null_mut());
            ffi::BatchGraphics();
        });
        let dimensions = cx.screen_dimensions();
        crate::ui::border::draw_sis_frame(cx.kernel, cx.world, dimensions);
        crate::ui::sis::draw_campaign_message(cx.kernel, cx.world, dimensions.0, None);
        reset_solar_sys(cx, solar, next_activity);
        if cx.kernel.last_activity == CHECK_LOAD | CHECK_RESTART {
            draw_outer_system(cx, solar);
            SolarSystemStart::Fade
        } else if cx.kernel.last_activity == CHECK_LOAD && next_activity == 0 {
            SolarSystemStart::ResumeLoad
        } else {
            if inner_system {
                draw_inner_system(cx, solar);
            } else {
                draw_outer_system(cx, solar);
            }
            SolarSystemStart::Transition
        }
    }
}

pub(super) unsafe fn end_interplanetary(cx: &mut Cx<'_>, io: &mut Io<'_>, solar: &mut SolarSystem) {
    cx.world
        .set_activity(cx.world.activity() & !END_INTERPLANETARY);
    if cx.world.activity() & (CHECK_ABORT | CHECK_LOAD) == 0 {
        invoke(cx, io, solar, |generator, system, host| {
            generator.uninit_npcs(system, host)
        });
        cx.world.set_story_state(StoryKey::USED_BROADCASTER, 0);
    }
}

pub(super) fn closest_planet_interplanetary(solar: &SolarSystem, point: c::POINT) -> Body {
    let mut best_distance_squared = u32::MAX;
    let mut best_planet = None;
    for (i, planet) in solar.sys.planets[..usize::from(solar.sys.sun.num_planets)]
        .iter()
        .enumerate()
    {
        let dx = (i32::from(point.x) - i32::from(planet.image.origin.x)) as c::SIZE;
        let dy = (i32::from(point.y) - i32::from(planet.image.origin.y)) as c::SIZE;
        let distance_squared =
            (i64::from(dx) * i64::from(dx) + i64::from(dy) * i64::from(dy)) as c::DWORD;
        if distance_squared < best_distance_squared {
            best_distance_squared = distance_squared;
            best_planet = Some(Body::Planet(i as u8));
        }
    }
    match best_planet {
        Some(planet) => planet,
        None => panic!("interplanetary encounter requires a planet"),
    }
}

pub(super) unsafe fn uninit_solar_sys(cx: &mut Cx<'_>, io: &mut Io<'_>, solar: &mut SolarSystem) {
    free_solar_sys(cx, solar);
    cx.with_legacy(|| {
        ffi::DestroyDrawable(ReleaseDrawable(STARS_FRAME.cast()).cast());
        STARS_FRAME = ptr::null_mut();
    });
    if cx.world.activity() & END_INTERPLANETARY != 0 {
        end_interplanetary(cx, io, solar);
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
            let target = closest_planet_interplanetary(solar, point);
            invoke(cx, io, solar, |generator, system, host| {
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
    let star = prepare_explore_solar_sys(cx.world);
    let mut solar = SolarSystem::new(cx.world, star);
    start_solar_sys(cx, io, &mut solar);
    cx.with_legacy(|| ffi::SetMenuSounds(input, MENU_SOUND_NONE, MENU_SOUND_NONE));
    solar.run_input(cx, io, false, |cx, io, solar| {
        do_ip_flight(cx, io, solar) != 0
    });
    uninit_solar_sys(cx, io, &mut solar);
    cx.solar_frame = None;
    io.observer
        .checkpoint(&crate::engine::observe::CheckpointView::Solar(None));
}

unsafe fn prepare_explore_solar_sys(world: &mut World) -> crate::engine::world::StarId {
    let (log_units_x, log_units_y) = hyperspace_unit_factors();
    if world.current_star.is_none() {
        let universe = c::POINT {
            x: ((world.data.flagship.log_x * 6250 + (log_units_x >> 1)) / log_units_x) as _,
            y: (9999 - (world.data.flagship.log_y * 625 + (log_units_y >> 1)) / log_units_y) as _,
        };
        world.current_star =
            crate::planets::starmap::find_star_id(world, world.catalog(), None, universe, 1, 1);
    }
    let Some(id) = world.current_star else {
        panic!("ExploreSolarSys(): do not know where you are");
    };
    let star = world.catalog()[id.0];
    world.data.flagship.log_x = (i32::from(star.star_pt.x) * log_units_x + (6250 >> 1)) / 6250;
    world.data.flagship.log_y =
        ((9999 - i32::from(star.star_pt.y)) * log_units_y + (625 >> 1)) / 625;
    id
}

pub fn named_body_index(solar: &SolarSystem) -> Option<u16> {
    let body = solar.selected_body()?;
    let planet = body.planet() as u16;
    match (solar.sys.star_index(), body) {
        (SOL_DEFINED, Body::Planet(_)) => Some(PLANET_NUMBER_BASE + planet),
        (SOL_DEFINED, Body::Moon { moon, .. }) => match (planet, moon) {
            (2, 0) => Some(STARBASE_STRING_BASE),
            (2, 1) => Some(PLANET_NUMBER_BASE + 9),
            (4, 0..=3) => Some(PLANET_NUMBER_BASE + 10 + u16::from(moon)),
            (5, 0) => Some(PLANET_NUMBER_BASE + 14),
            (7, 0) => Some(PLANET_NUMBER_BASE + 15),
            _ => None,
        },
        (SAMATRA_DEFINED, Body::Moon { planet: 4, moon: 0 }) => Some(PLANET_NUMBER_BASE + 32),
        _ => None,
    }
}

pub unsafe fn save_solar_location(world: &mut World, solar: &SolarSystem) {
    if !solar.in_orbit() {
        save_non_orbital_location(world, solar);
    } else {
        if world.story_state(StoryKey::PLANETARY_CHANGE) != 0 {
            ffi::put_planet_info(world, solar);
            world.set_story_state(StoryKey::PLANETARY_CHANGE, 0);
        }
        assert_ne!(world.data.game.ip_planet, 0);
        world.data.game.in_orbit = solar.require_body().saved_orbit();
    }
}

unsafe fn do_solar_sys_menu(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    solar: &mut SolarSystem,
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
    if crate::ui::menu::choose_menu_state(cx, input, &mut menu.CurState, PM_STARMAP) != 0 {
        return 1;
    }
    if cx.kernel.last_activity == CHECK_LOAD {
        select = 1;
    }
    if select == 0 {
        return 1;
    }
    crate::ui::sis::clear_flash(cx.kernel);
    match menu.CurState {
        EQUIP_DEVICE => {
            select = devices_menu(cx, io, Some(solar)) as c::BOOLEAN;
            if cx.world.activity() & START_ENCOUNTER != 0 {
                return 0;
            }
        }
        CARGO => ffi::cargo_menu(cx, io),
        ROSTER => select = ffi::roster_menu(cx, io) as c::BOOLEAN,
        GAME_MENU => {
            if ffi::game_options(cx, io, Some(solar)) == 0 {
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
                matches!(solar.location(), SystemLocation::Inner(_))
            });
            draw_system_transition(cx, solar, inner);
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
            crate::ui::menu::draw_menu_state(
                cx.kernel,
                cx.world,
                (cx.screen_dimensions().0, cx.screen_dimensions().1),
                PM_STARMAP,
                i16::from(menu.CurState),
            );
        }
        crate::ui::sis::set_menu_flash(
            cx.kernel,
            (cx.screen_dimensions().0, cx.screen_dimensions().1),
            false,
        );
    }
    1
}

unsafe fn solar_sys_menu(cx: &mut Cx<'_>, io: &mut Io<'_>, solar: &mut SolarSystem) {
    let runtime = cx.kernel.input_runtime();
    let input = runtime.input();
    let mut menu: c::MENU_STATE = std::mem::zeroed();
    let dimensions = cx.screen_dimensions();
    if cx.kernel.last_activity == CHECK_LOAD {
        menu.CurState = GAME_MENU;
    } else {
        crate::ui::menu::draw_menu_state(
            cx.kernel,
            cx.world,
            dimensions,
            PM_STARMAP,
            i16::from(STARMAP),
        );
        menu.CurState = STARMAP;
    }
    crate::ui::sis::draw_status_message(cx.kernel, cx.world, None);
    crate::ui::sis::set_menu_flash(cx.kernel, dimensions, false);
    ffi::SetMenuSounds(input, MENU_SOUND_ARROWS, MENU_SOUND_SELECT);
    solar.run_input(cx, io, true, |cx, io, solar| {
        do_solar_sys_menu(cx, io, solar, &mut menu) != 0
    });
    crate::ui::menu::draw_menu_state(
        cx.kernel,
        cx.world,
        (cx.screen_dimensions().0, cx.screen_dimensions().1),
        PM_STARMAP,
        -i16::from(NAVIGATION),
    );
}

unsafe fn do_ip_flight(cx: &mut Cx<'_>, io: &mut Io<'_>, solar: &mut SolarSystem) -> c::BOOLEAN {
    let runtime = cx.kernel.input_runtime();
    let input = runtime.input();
    let cancel = input.pulsed().menu[KEY_MENU_CANCEL];
    if solar.in_orbit() {
        run_planet_orbit(cx, io, solar);
        cx.with_legacy(|| ffi::SetMenuSounds(input, MENU_SOUND_NONE, MENU_SOUND_NONE));
        solar.finish_orbit();
    } else if cancel != 0 || cx.kernel.last_activity == CHECK_LOAD {
        solar_sys_menu(cx, io, solar);
        cx.with_legacy(|| ffi::SetMenuSounds(input, MENU_SOUND_NONE, MENU_SOUND_NONE));
    } else {
        assert!(solar.in_ip_flight);
        ip_frame(cx, io, solar);
        cx.with_legacy(|| {
            ffi::SleepThreadUntil(NEXT_TIME);
            NEXT_TIME = ffi::GetTimeCounter().wrapping_add(IP_FRAME_RATE);
        });
    }
    (cx.world.activity() & (START_ENCOUNTER | END_INTERPLANETARY | CHECK_ABORT | CHECK_LOAD) == 0
        && cx.world.data.flagship.crew_enlisted != c::COUNT::MAX) as c::BOOLEAN
}
