//! Orbital entry, with system borrows confined to host callbacks.

use super::{PLUTO, STARBASE};
use crate::c_bindings::global_state::{PLANET_INFO, SYSTEM_INFO};
use crate::comm::dispatch::START_ENCOUNTER;
use crate::planets::calc;
use crate::planets::consts::EARTH_RADIUS;
use crate::planets::generate::default::{self, Npcs};
use crate::planets::generate::special::{Body, Orbit, Story, EARTH_ATMOSPHERE};
use crate::planets::surface::{self, BIOLOGICAL_SCAN, MINERAL_SCAN};

/// Queue operations shared by Sol's bomb reset and starbase entry.
pub trait Queues: Npcs {
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
    /// Owned surface definition transferred to `LoadPlanet`, never a forged
    /// Rust handle. Its owner must retain or destroy it as surface generation
    /// requires (including rescaling), and free the retained surface on exit.
    type EarthSurface;

    fn set_encounter_group(&mut self, value: u8);
    fn current_activity(&self) -> u16;
    fn set_current_activity(&mut self, value: u16);
    fn set_global_flags_and_data(&mut self, value: u8);
    fn with_orbit(&mut self, run: impl FnOnce(&mut Orbit<'_>));
    fn planet_info(&mut self) -> &mut PLANET_INFO;
    /// Load the standard lander font, capture the scene graphic into
    /// `PlanetSideFrame[1]`, then capture its discovery strings. These remain
    /// owned by the system until the existing planet/lander cleanup.
    fn load_scene(&mut self, scene: Scene);
    /// Capture `graphics.earthmask`; the result is consumed by `load_planet`.
    fn load_earth_surface(&mut self) -> Self::EarthSurface;
    fn load_planet(&mut self, surface: Option<Self::EarthSurface>);
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
    orbit.sys_info.PlanetInfo.ScanSeed[MINERAL_SCAN] = orbit.rng.seed();
    let mut spare = unsafe { std::mem::zeroed() };
    surface::generate_mineral_deposits(
        orbit.rng,
        orbit.sys_info,
        default::GENERATE_ALL,
        &mut spare,
    );
    if let Body::Moon { planet, .. } = body {
        let info = &mut orbit.sys_info.PlanetInfo;
        info.AxialTilt = 0;
        info.AtmoDensity = 0;
        info.Weather = 0;
        if planet == 2 {
            info.ScanSeed[BIOLOGICAL_SCAN] = orbit.rng.seed();
        }
    }
}

/// Physical values applied after the optional discovery scene, preserving
/// every analysis field C leaves alone, including life chance and scan masks.
pub fn physical_overrides(sys: &mut SYSTEM_INFO, body: Body) {
    override_info(&mut sys.PlanetInfo, body);
}

fn override_info(info: &mut PLANET_INFO, body: Body) {
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
        info.AtmoDensity = atmo;
        info.PlanetDensity = density;
        info.PlanetRadius = radius;
        info.AxialTilt = tilt;
        info.Weather = weather;
        info.Tectonics = tectonics;
        info.RotationPeriod = rotation;
        info.SurfaceTemperature = temp;
    }
    if let Body::Moon { planet, moon } = body {
        let physical = match (planet, moon) {
            (2, _) => Some((60, 25, 0, 240 * 29, -18)),
            (4, 0) => Some((69, 25, 3, 390, -163)),
            (4, 1) => Some((54, 25, 1, 840, -161)),
            (4, 2) => Some((35, 41, 0, 1728, -164)),
            (4, 3) => Some((35, 38, 1, 4008, -167)),
            (5, _) => {
                info.AtmoDensity = 160;
                info.Weather = 2;
                Some((34, 40, 1, 3816, -178))
            }
            (7, _) => {
                info.AtmoDensity = 10;
                info.Weather = 1;
                Some((95, 27, 0, 4300, -216))
            }
            _ => None,
        };
        if let Some((density, radius, tectonics, rotation, temp)) = physical {
            info.PlanetDensity = density;
            info.PlanetRadius = radius;
            info.Tectonics = tectonics;
            info.RotationPeriod = rotation;
            info.SurfaceTemperature = temp;
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
        info.PlanetToSunDist = (i32::from(EARTH_RADIUS) * percent / 100) as u16;
    }
    info.SurfaceGravity = calc::calc_gravity(info);
}

pub fn generate_orbital(body: Body, story: &impl Story, host: &mut impl Orbital) -> bool {
    if body == STARBASE {
        host.save_ip_groups();
        host.reinit_ip_group_q();
        assert_eq!(host.built_ship_count(), 0);
        host.set_encounter_group(0);
        host.set_current_activity(host.current_activity() | START_ENCOUNTER);
        host.set_global_flags_and_data(u8::MAX);
        return true;
    }

    host.with_orbit(|orbit| analyse_orbital(orbit, body));
    if body == PLUTO && story.flag("FOUND_PLUTO_SPATHI") == 0 {
        host.load_scene(Scene::Pluto);
    } else if matches!(body, Body::Moon { planet: 2, .. }) && story.flag("MOONBASE_DESTROYED") == 0
    {
        host.load_scene(Scene::Moonbase);
    }
    override_info(host.planet_info(), body);
    let surface = if body == Body::Planet(2) {
        Some(host.load_earth_surface())
    } else {
        None
    };
    host.load_planet(surface);
    true
}
