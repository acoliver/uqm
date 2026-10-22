//! `genburv.c`: the Burvixese home system. Planet 0 is a redux world with
//! ruins and a selenic moon that holds the broadcasters.
//!
//! Overridden: `generatePlanets`, `generateMoons`, `generateOrbital`,
//! `generateEnergy`, `pickupEnergy`. Story flags: `BURVIXESE_BROADCASTERS`,
//! `BURV_BROADCASTERS_ON_SHIP`.

use crate::battle::battle_types::normalize_angle;
use crate::c_bindings::global_state::{PlanetFrame, NODE_INFO, PLANET_DESC, SYSTEM_INFO};
use crate::math::RandomContext;
use crate::planet_side::report_adapter::DiscoveryReportPort;
use crate::planets::generate::default;
use crate::planets::generate::home_contact::LanderTakeoff;
use crate::planets::generate::special::{
    self, Body, Orbit, Story, MIN_MOON_RADIUS, REDUX_WORLD, SELENIC_WORLD,
};
use crate::planets::orbits::{MAX_MOONS, MOON_DELTA};

const HOME: Body = Body::Planet(0);
const BROADCASTERS_MOON: Body = Body::Moon { planet: 0, moon: 0 };

/// The two scenes `generateOrbital` can load before the planet display.
pub trait BurvScenes {
    /// `LoadStdLanderFont`, the ruins mask and the Burvixese ruins strings.
    fn load_ruins_scene(&mut self);
    /// `LoadStdLanderFont`, the broadcasters mask and report strings.
    fn load_broadcasters_scene(&mut self);
}

/// `GenerateBurvixese_generatePlanets`.
pub fn generate_planets(
    rng: &mut RandomContext,
    plan_data: &[PlanetFrame],
    star_size: u8,
    star_color: u8,
    planets: &mut [PLANET_DESC],
) -> u8 {
    special::planets_then(rng, plan_data, star_size, star_color, planets, |planets| {
        planets[0].data_index = REDUX_WORLD;
        planets[0].NumPlanets = 1;
        special::reposition(&mut planets[0], 39);
    })
}

/// `GenerateBurvixese_generateMoons`: `moons` is the whole moon table and
/// `planet_moons` the moon count of the planet being generated. The home
/// world's first moon is a selenic world on the outermost moon orbit at a
/// random angle, whatever the default drew.
pub fn generate_moons(
    rng: &mut RandomContext,
    star_size: u8,
    star_color: u8,
    planet_moons: u8,
    moons: &mut [PLANET_DESC],
    body: Body,
) -> bool {
    default::generate_moons(
        rng,
        star_size,
        star_color,
        planet_moons,
        special::moon_slice(moons, planet_moons),
    );
    if body == HOME {
        let moon = &mut moons[0];
        moon.data_index = SELENIC_WORLD;
        moon.radius = MIN_MOON_RADIUS + ((MAX_MOONS - 1) * MOON_DELTA) as i16;
        let angle = normalize_angle(rng.next_random() as u16);
        special::place(moon, angle);
    }
    true
}

/// `GenerateBurvixese_generateOrbital`: the default analysis, then the home
/// world's ruins scene and fixed weather, or the broadcasters moon's scene
/// until the broadcasters are taken, then the planet display.
pub fn generate_orbital(
    orbit: &mut Orbit<'_>,
    body: Body,
    story: &impl Story,
    scenes: &mut impl BurvScenes,
    load_planet: impl FnOnce(),
) -> bool {
    orbit.analyse();
    if body == HOME {
        scenes.load_ruins_scene();
        orbit.sys_info.PlanetInfo.Weather = 0;
        orbit.sys_info.PlanetInfo.Tectonics = 0;
    } else if body == BROADCASTERS_MOON && story.flag("BURVIXESE_BROADCASTERS") == 0 {
        scenes.load_broadcasters_scene();
    }
    load_planet();
    true
}

/// `GenerateBurvixese_generateEnergy`: ruins on the home world and, until
/// they are taken, the broadcasters on its first moon.
pub fn generate_energy(
    rng: &mut RandomContext,
    sys_info: &SYSTEM_INFO,
    body: Body,
    which_node: u16,
    info: &mut NODE_INFO,
    story: &impl Story,
) -> u16 {
    if body == HOME {
        return default::generate_ruins(rng, sys_info, which_node, info);
    }
    if body == BROADCASTERS_MOON && story.flag("BURVIXESE_BROADCASTERS") == 0 {
        return default::generate_artifact(rng, sys_info, which_node, info);
    }
    0
}

/// `GenerateBurvixese_pickupEnergy`: the ruins give the standard cycled
/// report; the broadcasters are taken, once.
pub fn pickup_energy(
    body: Body,
    which_node: u16,
    story: &mut impl Story,
    report: &mut impl DiscoveryReportPort,
    lander: &mut impl LanderTakeoff,
) -> bool {
    if body == HOME {
        default::lander_report_cycle(report);
        return false;
    }
    if body == BROADCASTERS_MOON {
        assert!(
            story.flag("BURVIXESE_BROADCASTERS") == 0 && which_node == 0,
            "the broadcasters are picked up once, from node 0"
        );
        default::lander_report(report);
        lander.lander_takeoff();
        story.set_flag("BURVIXESE_BROADCASTERS", 1);
        story.set_flag("BURV_BROADCASTERS_ON_SHIP", 1);
        return true;
    }
    false
}

#[cfg(test)]
mod tests;
