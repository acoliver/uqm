//! Solar-system entry, orbit, saving, and menu lifecycle from `solarsys.c`.

use super::drawing::*;
use super::flight::*;
use super::site::dispatch::invoke;
use super::*;
use crate::comm::locdata::CPoint;
use crate::engine::world::generation::Body;
use crate::engine::{battle::Cx, observe::Io, Kernel, StoryKey, World};
use crate::graphics::{context::BatchGuard, context::ContextLease, drawable::ContextFrame};
use crate::planets::devices::{devices_menu, SolarVisit};
use crate::planets::generate::calc::preview_body;
use crate::planets::{
    solar_coords::star_seed,
    solar_system::{SolarSystem, SystemLocation},
};

// globdata.h activity flags occupy the high byte of ACTIVITY.
const START_ENCOUNTER: u16 = 1 << 10;
const END_INTERPLANETARY: u16 = 1 << 11;
const CHECK_LOAD: u16 = 1 << 12;
const CHECK_RESTART: u16 = 1 << 13;
const CHECK_ABORT: u16 = 1 << 14;
const MAX_ZOOM_RADIUS: i16 = 128 << 6;
const ONE_SECOND: u32 = 840;
const IP_FRAME_RATE: u32 = ONE_SECOND / 30;
const FUSION_THRUSTER: u8 = 1;
const TURNING_JETS: u8 = 2;
use crate::starbase::ids::{IP_SHIP_THRUST_INCREMENT, IP_SHIP_TURN_DECREMENT, IP_SHIP_TURN_WAIT};
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

use super::planet_info::{get_planet_info, put_planet_info};
use crate::engine::time::counter_abi::GetTimeCounter;
use crate::front_end::gameopt::runtime::game_options;
use crate::navigation::interplanetary::do_missions;
use crate::planets::cargo::live::cargo_menu;
use crate::planets::orbit::live::{free_planet, planet_orbit_menu};
use crate::planets::pstarmap::star_map;
use crate::planets::roster::live::roster_menu;
use crate::ui::settings::stop_music;

#[inline]
const fn rgb15(r: u8, g: u8, b: u8) -> crate::graphics::geometry::Color {
    crate::graphics::geometry::Color {
        r: (r << 3) | (r >> 2),
        g: (g << 3) | (g >> 2),
        b: (b << 3) | (b >> 2),
        a: 0xff,
    }
}

#[inline]
fn c_point(point: CPoint) -> crate::graphics::geometry::Point {
    crate::graphics::geometry::Point {
        x: point.x,
        y: point.y,
    }
}

#[inline]
fn saved_point(point: crate::graphics::geometry::Point) -> CPoint {
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

fn temperature_color(temperature: i16) -> crate::graphics::geometry::Color {
    const TEMP_COLORS: [crate::graphics::geometry::Color; 5] = [
        rgb15(0x00, 0x00, 0x0e),
        rgb15(0x00, 0x06, 0x08),
        rgb15(0x00, 0x0b, 0x00),
        rgb15(0x0f, 0x00, 0x00),
        rgb15(0x0f, 0x08, 0x00),
    ];
    let mut index = ((i32::from(temperature) + 250) / 100) as u16;
    if usize::from(index) >= TEMP_COLORS.len() {
        index = (TEMP_COLORS.len() - 1) as u16;
    }
    TEMP_COLORS[usize::from(index)]
}

pub(super) fn load_solar_sys(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    solar: &mut SolarSystem,
) -> Option<Body> {
    let star = cx.world.catalog()[solar.star().0];
    solar.sys.rng.reseed(star_seed(star.star_pt));
    let sun_type = (star.star_type & 0x18) >> 3;
    solar.sys.sun.rand_seed = solar.sys.rng.next_random();
    solar.sys.sun.data_index = sun_type;
    solar.sys.sun.location = crate::graphics::geometry::Point { x: 0, y: 0 };
    solar.sys.sun.image.origin = solar.sys.sun.location;
    solar.sys.sun.image.frame =
        resources::frame(&resources::retained(cx.kernel).sun, u16::from(sun_type));

    invoke(cx, io, solar, |generator, system, host| {
        generator.generate_planets(system, host)
    });
    if cx.world.story_state(StoryKey::PLANETARY_CHANGE) != 0 {
        put_planet_info(cx.world, solar);
        cx.world.set_story_state(StoryKey::PLANETARY_CHANGE, 0);
    }
    for i in 0..solar.sys.planets.len() {
        solar.sys.planets[i].image.origin = solar.sys.planets[i].location;
        if i >= usize::from(solar.sys.sun.num_planets) {
            solar.sys.planets[i].image.frame = crate::graphics::drawable::FrameId::NULL;
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
        cx.world.data.game.ip_location = saved_point(solar.coordinates().to_location(
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
        cx.world.data.game.ship_stamp.frame =
            resources::frame(&resources::retained(cx.kernel).flagship, i - 1);
        None
    }
}

pub(super) fn save_non_orbital_location(world: &mut World, kernel: &Kernel, solar: &SolarSystem) {
    let game = &mut world.data.game;
    game.ship_facing = resources::ship_facing(kernel, game).wrapping_add(1);
    game.in_orbit = 0;
    match solar.location() {
        SystemLocation::Outer => game.ip_planet = 0,
        SystemLocation::Inner(planet) => {
            game.ip_planet = planet + 1;
            game.ip_location = saved_point(solar.sys.sun.location);
        }
    }
}

pub(super) fn free_solar_sys(cx: &mut Cx<'_>, solar: &mut SolarSystem) {
    if solar.in_ip_flight {
        solar.in_ip_flight = false;
        if cx.world.activity() & (CHECK_ABORT | CHECK_LOAD) == 0 {
            save_non_orbital_location(cx.world, cx.kernel, solar);
        }
    }
    solar.clear_system_image();
    stop_music();
}

pub(super) fn reset_solar_sys(cx: &mut Cx<'_>, solar: &mut SolarSystem, next_activity: u16) {
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
    do_missions(cx);
    check_intersect(cx.world, cx.kernel, solar);
    solar.in_ip_flight = true;
    if !crate::sound::music::plr_playing_current()
        && (cx.kernel.last_activity != CHECK_LOAD || next_activity != 0)
    {
        crate::sound::music::remember_music(
            resources::retained(cx.kernel).music_request(),
            true,
            1,
            music_enabled,
        );
    }
}

enum OrbitAdmission {
    Interrupted,
    Ready(Option<crate::planets::orbit::presentation::OrbitPresentation>),
}

fn run_planet_orbit(cx: &mut Cx<'_>, io: &mut Io<'_>, solar: &mut SolarSystem) {
    let OrbitAdmission::Ready(presentation) = prepare_planet_orbit(cx, io, solar) else {
        return;
    };
    if let Some(presentation) = presentation {
        let services = crate::planets::orbit::OrbitServices {
            submenu: |cx, io, solar, selection| match selection {
                crate::planets::orbit::OrbitSubmenu::Devices => {
                    devices_menu(cx, io, Some(solar_visit(solar))) != 0
                }
                crate::planets::orbit::OrbitSubmenu::Cargo => {
                    cargo_menu(cx, io);
                    true
                }
                crate::planets::orbit::OrbitSubmenu::Roster => roster_menu(cx, io) != 0,
                crate::planets::orbit::OrbitSubmenu::Options => {
                    game_options(cx, io, Some(solar)) != 0
                }
            },
            save_location: save_solar_location,
            store_groups: |world| {
                crate::navigation::groups::put_group_info(world, 0, u8::MAX);
            },
        };
        planet_orbit_menu(cx, io, solar, &presentation, &services);
        free_planet(cx, solar);
    }
    finish_planet_orbit(cx, io, solar);
}

fn prepare_planet_orbit(
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
            let (width, height) = solar.coordinates().dimensions();
            cx.world.data.game.ship_stamp.origin = CPoint {
                x: (width >> 1) as _,
                y: (height >> 1) as _,
            };
        }
    }
    get_planet_info(cx.world, solar);
    let (_, presentation) = invoke(cx, io, solar, |generator, system, host| {
        generator.generate_orbital(system, target, host)
    });
    cx.kernel.last_activity &= !(CHECK_LOAD | CHECK_RESTART);
    if cx.world.activity() & (CHECK_ABORT | CHECK_LOAD | START_ENCOUNTER) != 0
        || cx.world.data.flagship.crew_enlisted == u16::MAX
        || cx.world.story_state(StoryKey::CHMMR_BOMB_STATE) == 2
    {
        OrbitAdmission::Interrupted
    } else {
        OrbitAdmission::Ready(presentation)
    }
}

fn finish_planet_orbit(cx: &mut Cx<'_>, io: &mut Io<'_>, solar: &mut SolarSystem) {
    if cx.world.activity() & (START_ENCOUNTER | CHECK_ABORT | CHECK_LOAD) == 0
        && cx.world.data.flagship.crew_enlisted != u16::MAX
    {
        let space_context = ContextLease::require(cx.kernel.space_context);
        let next_activity = cx.world.next_activity;
        let orbital = load_solar_sys(cx, io, solar);
        assert!(orbital.is_none());
        check_zoom_level(&mut cx.world.data.game, solar);
        validate_orbits(resources::retained(cx.kernel), solar);
        validate_inner_orbits(resources::retained(cx.kernel), solar);
        reset_solar_sys(cx, solar, next_activity);
        let dimensions = cx.screen_dimensions();
        crate::ui::sis::repair_sis_border(cx.kernel, dimensions);
        space_context.select();
        let inner = matches!(solar.location(), SystemLocation::Inner(_));
        draw_system_transition(cx, solar, inner);
    }
}

enum SolarSystemStart {
    Orbit,
    Fade(BatchGuard),
    ResumeLoad(BatchGuard),
    Transition(BatchGuard),
}

fn start_solar_sys(cx: &mut Cx<'_>, io: &mut Io<'_>, solar: &mut SolarSystem) {
    let next_activity = cx.world.next_activity;
    let start = prepare_solar_sys(cx, io, solar, next_activity);
    solar.refresh_frame(cx);
    let context = ContextLease::require(cx.kernel.space_context);
    match start {
        SolarSystemStart::Orbit => {}
        SolarSystemStart::ResumeLoad(batch) => drop(batch),
        SolarSystemStart::Fade(batch) => {
            crate::battle::process_native::engine::redraw_queue(cx, false);
            drop(batch);
            context.fade_screen(FADE_ALL_TO_COLOR, (ONE_SECOND / 2) as i16);
            cx.kernel.last_activity = 0;
        }
        SolarSystemStart::Transition(batch) => {
            crate::battle::process_native::engine::redraw_queue(cx, false);
            context.transition(None);
            drop(batch);
            cx.kernel.last_activity &= !CHECK_LOAD;
        }
    }
}

fn prepare_solar_sys(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    solar: &mut SolarSystem,
    next_activity: u16,
) -> SolarSystemStart {
    crate::engine::kernel::solar_resources::SolarResources::retain(
        &mut cx.kernel.solar_presentation.assets,
    );
    if cx.kernel.lander_resources.is_none() {
        cx.kernel.lander_resources = Some(
            crate::engine::kernel::lander_resources::LanderResources::load()
                .unwrap_or_else(|key| panic!("missing lander resource {key}")),
        );
    }
    let reentry = cx.world.data.game.ship_facing != 0;
    if !reentry {
        let game = &mut cx.world.data.game;
        game.autopilot = CPoint { x: -1, y: -1 };
        let (width, height) = solar.coordinates().dimensions();
        game.ship_stamp.origin = CPoint {
            x: (width >> 1) as _,
            y: (height - 2) as _,
        };
        game.ip_location = saved_point(
            solar
                .coordinates()
                .to_location(c_point(game.ship_stamp.origin), MAX_ZOOM_RADIUS),
        );
    }
    solar.stars = Some(create_star_background(cx.world, cx.kernel, solar));
    let context = ContextLease::require(cx.kernel.space_context);
    context.select();
    context.set_frame(ContextFrame::retain(cx.kernel.screen));
    context.set_background(rgb15(0, 0, 0));
    let orbital = load_solar_sys(cx, io, solar);
    let inner_system = check_zoom_level(&mut cx.world.data.game, solar) != 0;
    validate_orbits(resources::retained(cx.kernel), solar);
    if inner_system {
        validate_inner_orbits(resources::retained(cx.kernel), solar);
    }
    if !reentry {
        let world = &mut *cx.world;
        world.encounter_race = -1;
        world.encounter_group = 0;
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
        context.select();
        context.transition_source(None);
        let batch = context.batch();
        let dimensions = cx.screen_dimensions();
        crate::ui::border::draw_sis_frame(cx.kernel, cx.world, dimensions);
        crate::ui::sis::draw_campaign_message(cx.kernel, cx.world, dimensions.0, None);
        reset_solar_sys(cx, solar, next_activity);
        if cx.kernel.last_activity == CHECK_LOAD | CHECK_RESTART {
            draw_outer_system(cx, solar);
            SolarSystemStart::Fade(batch)
        } else if cx.kernel.last_activity == CHECK_LOAD && next_activity == 0 {
            SolarSystemStart::ResumeLoad(batch)
        } else {
            if inner_system {
                draw_inner_system(cx, solar);
            } else {
                draw_outer_system(cx, solar);
            }
            SolarSystemStart::Transition(batch)
        }
    }
}

pub(super) fn end_interplanetary(cx: &mut Cx<'_>, io: &mut Io<'_>, solar: &mut SolarSystem) {
    cx.world
        .set_activity(cx.world.activity() & !END_INTERPLANETARY);
    if cx.world.activity() & (CHECK_ABORT | CHECK_LOAD) == 0 {
        invoke(cx, io, solar, |generator, system, host| {
            generator.uninit_npcs(system, host)
        });
        cx.world.set_story_state(StoryKey::USED_BROADCASTER, 0);
    }
}

pub(super) fn closest_planet_interplanetary(
    solar: &SolarSystem,
    point: crate::graphics::geometry::Point,
) -> Body {
    let mut best_distance_squared = u32::MAX;
    let mut best_planet = None;
    for (i, planet) in solar.sys.planets[..usize::from(solar.sys.sun.num_planets)]
        .iter()
        .enumerate()
    {
        let dx = (i32::from(point.x) - i32::from(planet.image.origin.x)) as i16;
        let dy = (i32::from(point.y) - i32::from(planet.image.origin.y)) as i16;
        let distance_squared =
            (i64::from(dx) * i64::from(dx) + i64::from(dy) * i64::from(dy)) as u32;
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

pub(super) fn uninit_solar_sys(cx: &mut Cx<'_>, io: &mut Io<'_>, solar: &mut SolarSystem) {
    free_solar_sys(cx, solar);
    solar.stars = None;
    if cx.world.activity() & END_INTERPLANETARY != 0 {
        end_interplanetary(cx, io, solar);
        return;
    }
    if cx.world.activity() & START_ENCOUNTER != 0 && cx.world.encounter_group != 0 {
        {
            let world = &mut *cx.world;
            crate::navigation::groups::get_group_info(
                world,
                world.data.game.battle_group_ref,
                world.encounter_group,
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
pub(super) fn hyperspace_unit_factors((width, height): (i32, i32)) -> (i32, i32) {
    ((((width - 64) << 5) >> 4) * 195, ((height << 5) >> 4) * 25)
}

pub fn run_explore_solar_sys(cx: &mut Cx<'_>, io: &mut Io<'_>) {
    let runtime = cx.kernel.input_runtime();
    let input = runtime.input();
    let star = prepare_explore_solar_sys(cx.world, cx.screen_dimensions());
    let mut solar = SolarSystem::new(cx.world, star);
    solar.coordinates = Some(crate::planets::solar_coords::SolarCoordinates::new(
        cx.screen_dimensions(),
    ));
    start_solar_sys(cx, io, &mut solar);
    input
        .controls()
        .set_menu_sounds(MENU_SOUND_NONE, MENU_SOUND_NONE);
    solar.run_input(cx, io, false, |cx, io, solar| {
        do_ip_flight(cx, io, solar) != 0
    });
    uninit_solar_sys(cx, io, &mut solar);
    cx.solar_frame = None;
    io.observer
        .checkpoint(&crate::engine::observe::CheckpointView::Solar(None));
}

fn prepare_explore_solar_sys(
    world: &mut World,
    screen: (i32, i32),
) -> crate::engine::world::StarId {
    let (log_units_x, log_units_y) = hyperspace_unit_factors(screen);
    if world.current_star.is_none() {
        let universe = crate::graphics::geometry::Point {
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

fn solar_visit(system: &SolarSystem) -> SolarVisit<'_> {
    SolarVisit {
        system,
        save_location: save_solar_location,
    }
}

pub fn save_solar_location(world: &mut World, kernel: &Kernel, solar: &SolarSystem) {
    if !solar.in_orbit() {
        save_non_orbital_location(world, kernel, solar);
    } else {
        if world.story_state(StoryKey::PLANETARY_CHANGE) != 0 {
            put_planet_info(world, solar);
            world.set_story_state(StoryKey::PLANETARY_CHANGE, 0);
        }
        assert_ne!(world.data.game.ip_planet, 0);
        world.data.game.in_orbit = solar.require_body().saved_orbit();
    }
}

struct SolarMenu {
    current: u8,
}

fn do_solar_sys_menu(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    solar: &mut SolarSystem,
    menu: &mut SolarMenu,
) -> u32 {
    let runtime = cx.kernel.input_runtime();
    let input = runtime.input();
    let mut select = input.pulsed().menu[KEY_MENU_SELECT] as u32;
    if cx.world.activity() & (CHECK_ABORT | CHECK_LOAD) != 0
        || cx.world.data.flagship.crew_enlisted == u16::MAX
    {
        return 0;
    }
    if crate::ui::menu::choose_menu_state(cx, input, &mut menu.current, PM_STARMAP) != 0 {
        return 1;
    }
    if cx.kernel.last_activity == CHECK_LOAD {
        select = 1;
    }
    if select == 0 {
        return 1;
    }
    crate::ui::sis::clear_flash(cx.kernel);
    match menu.current {
        EQUIP_DEVICE => {
            select = devices_menu(cx, io, Some(solar_visit(solar))) as u32;
            if cx.world.activity() & START_ENCOUNTER != 0 {
                return 0;
            }
        }
        CARGO => cargo_menu(cx, io),
        ROSTER => select = roster_menu(cx, io) as u32,
        GAME_MENU => {
            if game_options(cx, io, Some(solar)) == 0 {
                return 0;
            }
        }
        STARMAP => {
            star_map(cx, io);
            if cx.world.activity() & CHECK_ABORT != 0 {
                return 0;
            }
            let space_context = ContextLease::require(cx.kernel.space_context);
            space_context.select();
            let inner = matches!(solar.location(), SystemLocation::Inner(_));
            draw_system_transition(cx, solar, inner);
            return 0;
        }
        NAVIGATION => return 0,
        _ => {}
    }
    if cx.world.activity() & CHECK_ABORT == 0 {
        if select != 0 {
            if input.preferences().menu != OPT_PC {
                menu.current = NAVIGATION;
            }
            crate::ui::menu::draw_menu_state(
                cx.kernel,
                cx.world,
                (cx.screen_dimensions().0, cx.screen_dimensions().1),
                PM_STARMAP,
                i16::from(menu.current),
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

fn solar_sys_menu(cx: &mut Cx<'_>, io: &mut Io<'_>, solar: &mut SolarSystem) {
    let runtime = cx.kernel.input_runtime();
    let input = runtime.input();
    let mut menu = SolarMenu { current: 0 };
    let dimensions = cx.screen_dimensions();
    if cx.kernel.last_activity == CHECK_LOAD {
        menu.current = GAME_MENU;
    } else {
        crate::ui::menu::draw_menu_state(
            cx.kernel,
            cx.world,
            dimensions,
            PM_STARMAP,
            i16::from(STARMAP),
        );
        menu.current = STARMAP;
    }
    crate::ui::sis::draw_status_message(cx.kernel, cx.world, None);
    crate::ui::sis::set_menu_flash(cx.kernel, dimensions, false);
    input
        .controls()
        .set_menu_sounds(MENU_SOUND_ARROWS, MENU_SOUND_SELECT);
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

fn do_ip_flight(cx: &mut Cx<'_>, io: &mut Io<'_>, solar: &mut SolarSystem) -> u32 {
    let runtime = cx.kernel.input_runtime();
    let input = runtime.input();
    let cancel = input.pulsed().menu[KEY_MENU_CANCEL];
    if solar.in_orbit() {
        run_planet_orbit(cx, io, solar);
        input
            .controls()
            .set_menu_sounds(MENU_SOUND_NONE, MENU_SOUND_NONE);
        solar.finish_orbit();
    } else if cancel != 0 || cx.kernel.last_activity == CHECK_LOAD {
        solar_sys_menu(cx, io, solar);
        input
            .controls()
            .set_menu_sounds(MENU_SOUND_NONE, MENU_SOUND_NONE);
    } else {
        assert!(solar.in_ip_flight);
        ip_frame(cx, io, solar);
        let deadline = cx.kernel.solar_presentation.frame_deadline;
        runtime.sleep_until(cx, deadline);
        cx.kernel.solar_presentation.frame_deadline = GetTimeCounter().wrapping_add(IP_FRAME_RATE);
    }
    (cx.world.activity() & (START_ENCOUNTER | END_INTERPLANETARY | CHECK_ABORT | CHECK_LOAD) == 0
        && cx.world.data.flagship.crew_enlisted != u16::MAX) as u32
}
