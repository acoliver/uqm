//! `genvux.c`: VUX home, Zex' maidens and Zex' beast. C remains linked.

use crate::battle::gendef::{MAIDENS_DEFINED, VUX_BEAST_DEFINED, VUX_DEFINED};
use crate::c_bindings::global_state::{PlanetFrame, NODE_INFO, PLANET_DESC, SYSTEM_INFO};
use crate::math::RandomContext;
use crate::planet_side::report_adapter::DiscoveryReportPort;
use crate::planets::consts::EARTH_RADIUS;
use crate::planets::generate::default;
use crate::planets::generate::home_contact::{HomeContactSite, LanderTakeoff};
use crate::planets::generate::special::{self, Body, Orbit, Story, REDUX_WORLD, WATER_WORLD};
use crate::planets::generate::yehat::{CHECK_ABORT, CHECK_LOAD, START_INTERPLANETARY};
use crate::planets::surface;

const WORLD: Body = Body::Planet(0);
// `units.h`: FULL_CIRCLE=64, HALF_CIRCLE=32, OCTANT=8.
const HOME_ANGLE: u16 = 32 + 8;
const BEAST_ANGLE: u16 = 32 - 8;
// `lifeform.h`: NUM_CREATURE_TYPES=23; lists omit C's -1 terminator.
const MAIDENS_LIFE: [i8; 12] = [9, 9, 9, 9, 14, 14, 14, 14, 18, 18, 18, 18];
const BEAST_LIFE: [i8; 11] = [23 + 2, 3, 3, 3, 3, 3, 8, 8, 8, 8, 8];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scene {
    Maidens,
    Beast,
    Ruins,
}

pub trait VuxSite: HomeContactSite {
    /// Read after the conversation, which can kill Zex.
    fn zex_is_dead(&self) -> bool;
    fn repair_sis_border(&mut self);
    /// Beast loads the font and strings and clears PlanetSideFrame[1].
    fn load_scene(&mut self, scene: Scene);
}

pub fn generate_planets(
    rng: &mut RandomContext,
    table: &[PlanetFrame],
    star_size: u8,
    star_color: u8,
    star: u8,
    planets: &mut [PLANET_DESC],
) -> u8 {
    let mut count = default::generate_planets(rng, table, star_size, star_color, planets);
    if star == MAIDENS_DEFINED {
        // C deliberately consumes a second complete layout, including moon counts.
        count = default::generate_planets(rng, table, star_size, star_color, planets);
        planets[0].data_index = REDUX_WORLD;
        special::reposition(&mut planets[0], 212);
    } else {
        let angle = if star == VUX_DEFINED {
            planets[0].data_index = REDUX_WORLD;
            planets[0].NumPlanets = 1;
            planets[0].radius = (i32::from(EARTH_RADIUS) * 42 / 100) as i16;
            HOME_ANGLE
        } else {
            planets.copy_within(0..usize::from(count), 1);
            count += 1;
            planets[0].data_index = WATER_WORLD;
            planets[0].radius = (i32::from(EARTH_RADIUS) * 110 / 100) as i16;
            planets[0].NumPlanets = 0;
            BEAST_ANGLE
        };
        special::place(&mut planets[0], angle);
        planets[0].rand_seed = u32::from(planets[0].location.x as u16)
            | (u32::from(planets[0].location.y as u16) << 16);
    }
    count
}

pub fn generate_orbital(
    orbit: &mut Orbit<'_>,
    body: Body,
    star: u8,
    story: &impl Story,
    site: &mut impl VuxSite,
    load_planet: impl FnOnce(),
) -> bool {
    if body == WORLD
        && (star == VUX_DEFINED || (star == MAIDENS_DEFINED && !site.zex_is_dead()))
        && site.start_sphere_tracking()
    {
        site.notify_others_all_clear();
        site.save_ip_groups();
        site.reinit_ip_group_q();
        assert_eq!(site.built_ship_count(), 0);
        site.clone_fleet_into_built_queue();
        site.set_global_flags_and_data(if star == VUX_DEFINED { 1 << 7 } else { 1 << 6 });
        site.set_current_activity(site.current_activity() | START_INTERPLANETARY);
        site.talk();
        let activity = site.current_activity();
        if activity & (CHECK_ABORT | CHECK_LOAD) != 0 {
            return true;
        }
        site.set_current_activity(activity & !START_INTERPLANETARY);
        site.reinit_built_ship_q();
        site.load_ip_groups();
        if star == VUX_DEFINED || !site.zex_is_dead() {
            return true;
        }
        site.repair_sis_border();
    }
    if body == WORLD {
        match star {
            MAIDENS_DEFINED if story.flag("SHOFIXTI_MAIDENS") == 0 => {
                site.load_scene(Scene::Maidens)
            }
            VUX_BEAST_DEFINED if story.flag("VUX_BEAST") == 0 => site.load_scene(Scene::Beast),
            MAIDENS_DEFINED | VUX_BEAST_DEFINED => {}
            _ => site.load_scene(Scene::Ruins),
        }
    }
    orbit.default_orbital(load_planet);
    if body == WORLD {
        // C changes these after LoadPlanet, not before it.
        orbit.sys_info.PlanetInfo.Weather = 2;
        orbit.sys_info.PlanetInfo.Tectonics = 0;
    }
    true
}

pub fn generate_energy(
    rng: &mut RandomContext,
    info: &SYSTEM_INFO,
    star: u8,
    body: Body,
    which: u16,
    node: &mut NODE_INFO,
    story: &impl Story,
) -> u16 {
    if body == WORLD {
        if star == MAIDENS_DEFINED {
            if story.flag("SHOFIXTI_MAIDENS") != 0 {
                return 0;
            }
            node.loc_pt.x = (crate::planets::consts::MAP_WIDTH / 3) as i16;
            node.loc_pt.y = (crate::planets::consts::MAP_HEIGHT * 5 / 8) as i16;
            return 1;
        }
        if star == VUX_DEFINED {
            return default::generate_ruins(rng, info, which, node);
        }
    }
    0
}

pub fn generate_life(
    rng: &mut RandomContext,
    info: &SYSTEM_INFO,
    star: u8,
    body: Body,
    which: u16,
    node: &mut NODE_INFO,
) -> u16 {
    if body == WORLD {
        if star == MAIDENS_DEFINED {
            return surface::generate_preset_life(rng, info, &MAIDENS_LIFE, which, node);
        }
        if star == VUX_BEAST_DEFINED {
            return surface::generate_preset_life(rng, info, &BEAST_LIFE, which, node);
        }
    }
    default::generate_life(rng, info, which, node)
}

pub fn pickup_energy(
    star: u8,
    body: Body,
    which: u16,
    story: &mut impl Story,
    report: &mut impl DiscoveryReportPort,
    lander: &mut impl LanderTakeoff,
) -> bool {
    if star == MAIDENS_DEFINED && body == WORLD {
        assert!(story.flag("SHOFIXTI_MAIDENS") == 0 && which == 0);
        default::lander_report(report);
        lander.lander_takeoff();
        story.set_flag("SHOFIXTI_MAIDENS", 1);
        story.set_flag("MAIDENS_ON_SHIP", 1);
        return true;
    }
    if star == VUX_DEFINED && body == WORLD {
        default::lander_report_cycle(report);
    }
    false
}

pub fn pickup_life(
    star: u8,
    body: Body,
    which: u16,
    story: &mut impl Story,
    report: &mut impl DiscoveryReportPort,
    lander: &mut impl LanderTakeoff,
) -> bool {
    if star == VUX_BEAST_DEFINED && body == WORLD && which == 0 {
        assert_eq!(story.flag("VUX_BEAST"), 0);
        default::lander_report(report);
        lander.lander_takeoff();
        story.set_flag("VUX_BEAST", 1);
        story.set_flag("VUX_BEAST_ON_SHIP", 1);
    }
    true
}

#[cfg(test)]
pub(crate) mod tests;
