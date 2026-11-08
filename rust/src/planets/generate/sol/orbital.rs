//! Orbital entry, with system borrows confined to host callbacks.

use super::{PLUTO, STARBASE};
use crate::engine::save::START_ENCOUNTER;
use crate::engine::world::planet::{PlanetInfo, SystemInfo};
use crate::engine::StoryKey;
use crate::planets::generate::calc;
use crate::planets::generate::consts::EARTH_RADIUS;
use crate::planets::generate::default;
use crate::planets::generate::special::{Body, Orbit, Story, EARTH_ATMOSPHERE};
use crate::planets::generate::surface::{self, BIOLOGICAL_SCAN, MINERAL_SCAN};

/// Queue operations shared by Sol's bomb reset and starbase entry.
pub trait Queues {
    fn save_ip_groups(&mut self);
    fn reinit_ip_group_q(&mut self);
    fn built_ship_count(&self) -> u16;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scene {
    Pluto,
    Moonbase,
}

impl Scene {
    pub fn graphics(self) -> &'static std::ffi::CStr {
        match self {
            Self::Pluto => c"graphics.fwiffo",
            Self::Moonbase => c"graphics.moonbase",
        }
    }

    pub fn strings(self) -> &'static std::ffi::CStr {
        match self {
            Self::Pluto => c"text.fwiffo",
            Self::Moonbase => c"text.moonbase",
        }
    }
}

/// Effects of `generateOrbital`. The host releases all system/RNG borrows
/// before returning from `with_orbit`, and before resource or display calls.
pub trait Orbital: Queues {
    fn set_encounter_group(&mut self, value: u8);
    fn current_activity(&self) -> u16;
    fn set_current_activity(&mut self, value: u16);
    fn set_global_flags_and_data(&mut self, value: u8);
    /// Load the standard lander font, capture the scene graphic into
    /// `PlanetSideFrame[1]`, then capture its discovery strings. These remain
    /// owned by the system until the existing planet/lander cleanup.
    fn load_scene(
        &mut self,
        _system: &mut crate::engine::world::generation::SolarSys<'_>,
        scene: Scene,
    );
}

/// `DoPlanetaryAnalysis`, mineral seed and counting, followed by Luna's bio
/// seed and the common moon defaults. No life draw or energy seed write occurs.
pub fn analyse_orbital(orbit: &mut Orbit<'_>, body: Body) {
    calc::do_planetary_analysis(
        orbit.rng,
        orbit.plan_data,
        orbit.star,
        orbit.site,
        orbit.sys_info,
        orbit.world,
    );
    orbit.sys_info.planet_info.scan_seed[MINERAL_SCAN] = orbit.rng.seed();
    let mut spare = default::blank_node();
    surface::generate_mineral_deposits(
        orbit.rng,
        orbit.sys_info,
        &orbit.plan_data[usize::from(
            orbit.world.data_index & !crate::planets::generate::consts::PLANET_SHIELDED,
        )],
        default::GENERATE_ALL,
        &mut spare,
    );
    if let Body::Moon { planet, .. } = body {
        let info = &mut orbit.sys_info.planet_info;
        info.axial_tilt = 0;
        info.atmo_density = 0;
        info.weather = 0;
        if planet == 2 {
            info.scan_seed[BIOLOGICAL_SCAN] = orbit.rng.seed();
        }
    }
}

/// Physical values applied after the optional discovery scene, preserving
/// every analysis field C leaves alone, including life chance and scan masks.
pub fn physical_overrides(sys: &mut SystemInfo, body: Body) {
    override_info(&mut sys.planet_info, body);
}

fn override_info(info: &mut PlanetInfo, body: Body) {
    // atmo, density, radius, tilt, weather, tectonics, rotation, temperature.
    let planet = match body {
        Body::Planet(0) => Some((0, 98, 38, 3, 0, 2, 59 * 240, 165)),
        Body::Planet(1) => Some((90 * EARTH_ATMOSPHERE, 95, 95, 177, 7, 1, 243 * 240, 457)),
        Body::Planet(2) => Some((EARTH_ATMOSPHERE, 100, 100, 23, 1, 1, 240, 22)),
        Body::Planet(3) => Some((1, 72, 53, 24, 1, 1, 246, -53)),
        Body::Planet(4) => Some((u16::MAX, 24, 1120, 3, 7, 0, 98, -143)),
        Body::Planet(5) => Some((u16::MAX, 13, 945, 27, 7, 0, 102, -197)),
        Body::Planet(6) => Some((u16::MAX, 21, 411, 98, 7, 0, 172, -217)),
        Body::Planet(7) => Some((u16::MAX, 28, 396, 30, 7, 0, 182, -229)),
        Body::Planet(8) => Some((0, 33, 18, 119, 0, 0, 1533, -235)),
        _ => None,
    };
    if let Some((atmo, density, radius, tilt, weather, tectonics, rotation, temp)) = planet {
        info.atmo_density = atmo;
        info.planet_density = density;
        info.planet_radius = radius;
        info.axial_tilt = tilt;
        info.weather = weather;
        info.tectonics = tectonics;
        info.rotation_period = rotation;
        info.temperature = temp;
    }
    if let Body::Moon { planet, moon } = body {
        let physical = match (planet, moon) {
            (2, _) => Some((60, 25, 0, 240 * 29, -18)),
            (4, 0) => Some((69, 25, 3, 390, -163)),
            (4, 1) => Some((54, 25, 1, 840, -161)),
            (4, 2) => Some((35, 41, 0, 1728, -164)),
            (4, 3) => Some((35, 38, 1, 4008, -167)),
            (5, _) => {
                info.atmo_density = 160;
                info.weather = 2;
                Some((34, 40, 1, 3816, -178))
            }
            (7, _) => {
                info.atmo_density = 10;
                info.weather = 1;
                Some((95, 27, 0, 4300, -216))
            }
            _ => None,
        };
        if let Some((density, radius, tectonics, rotation, temp)) = physical {
            info.planet_density = density;
            info.planet_radius = radius;
            info.tectonics = tectonics;
            info.rotation_period = rotation;
            info.temperature = temp;
        }
    }
    let parent = match body {
        Body::Planet(p) | Body::Moon { planet: p, .. } => p,
    };
    let percent = match parent {
        4 => Some(520),
        5 => Some(952),
        6 if matches!(body, Body::Planet(_)) => Some(1916),
        7 => Some(2999),
        8 if matches!(body, Body::Planet(_)) => Some(3937),
        _ => None,
    };
    if let Some(percent) = percent {
        info.planet_to_sun_dist = (i32::from(EARTH_RADIUS) * percent / 100) as u16;
    }
    info.gravity = calc::calc_gravity(info);
}

pub fn generate_orbital(
    system: &mut crate::engine::world::generation::SolarSys<'_>,
    body: Body,
    host: &mut (impl Orbital + Story + super::super::host::PlanetSite + ?Sized),
) -> bool {
    if body == STARBASE {
        host.save_ip_groups();
        host.reinit_ip_group_q();
        assert_eq!(host.built_ship_count(), 0);
        host.set_encounter_group(0);
        host.set_current_activity(host.current_activity() | START_ENCOUNTER);
        host.set_global_flags_and_data(u8::MAX);
        return true;
    }
    crate::planets::generate::special::with_orbit(system, body, |orbit| {
        analyse_orbital(orbit, body)
    });
    if body == PLUTO && host.flag(StoryKey::FOUND_PLUTO_SPATHI) == 0 {
        host.load_scene(system, Scene::Pluto);
    } else if matches!(body, Body::Moon { planet: 2, .. })
        && host.flag(StoryKey::MOONBASE_DESTROYED) == 0
    {
        host.load_scene(system, Scene::Moonbase);
    }
    override_info(system.planet_info(), body);
    if body == Body::Planet(2) {
        host.load_earth_planet(system, body);
    } else {
        host.load_planet(system, body);
    }
    true
}
