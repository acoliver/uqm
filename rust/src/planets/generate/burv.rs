//! `genburv.c`: the Burvixese home system. Planet 0 is a redux world with
//! ruins and a selenic moon that holds the broadcasters.
//!
//! Overridden: `generatePlanets`, `generateMoons`, `generateOrbital`,
//! `generateEnergy`, `pickupEnergy`. Story flags: `BURVIXESE_BROADCASTERS`,
//! `BURV_BROADCASTERS_ON_SHIP`.

use crate::engine::math::battle_types::normalize_angle;
use crate::engine::world::planet::{NodeInfo, PlanetDesc, PlanetFrame, SystemInfo};
use crate::engine::StoryKey;
use crate::math::RandomContext;
use crate::planets::generate::default;
use crate::planets::generate::home_contact::LanderTakeoff;
use crate::planets::generate::orbits::{MAX_MOONS, MOON_DELTA};
use crate::planets::generate::special::{
    self, Body, Story, MIN_MOON_RADIUS, REDUX_WORLD, SELENIC_WORLD,
};

const HOME: Body = Body::Planet(0);
const BROADCASTERS_MOON: Body = Body::Moon { planet: 0, moon: 0 };

/// The two scenes `generateOrbital` can load before the planet display.
pub trait BurvScenes {
    /// `LoadStdLanderFont`, the ruins mask and the Burvixese ruins strings.
    fn load_ruins_scene(&mut self, _system: &mut crate::engine::world::generation::SolarSys<'_>);
    /// `LoadStdLanderFont`, the broadcasters mask and report strings.
    fn load_broadcasters_scene(
        &mut self,
        _system: &mut crate::engine::world::generation::SolarSys<'_>,
    );
}

/// `GenerateBurvixese_generatePlanets`.
pub fn generate_planets(
    rng: &mut RandomContext,
    plan_data: &[PlanetFrame],
    star_size: u8,
    star_color: u8,
    planets: &mut [PlanetDesc],
) -> u8 {
    special::planets_then(rng, plan_data, star_size, star_color, planets, |planets| {
        planets[0].data_index = REDUX_WORLD;
        planets[0].num_planets = 1;
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
    moons: &mut [PlanetDesc],
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
    system: &mut SolarSys<'_>,
    body: Body,
    scenes: &mut (impl BurvScenes + Story + super::host::PlanetSite + ?Sized),
) -> bool {
    super::special::with_orbit(system, body, |orbit| orbit.analyse());
    if body == HOME {
        scenes.load_ruins_scene(system);
        system.planet_info().weather = 0;
        system.planet_info().tectonics = 0;
    } else if body == BROADCASTERS_MOON && scenes.flag(StoryKey::BURVIXESE_BROADCASTERS) == 0 {
        scenes.load_broadcasters_scene(system);
    }
    super::host::PlanetSite::load_planet(scenes, system, body);
    true
}

/// `GenerateBurvixese_generateEnergy`: ruins on the home world and, until
/// they are taken, the broadcasters on its first moon.
pub fn generate_energy(
    rng: &mut RandomContext,
    sys_info: &SystemInfo,
    body: Body,
    which_node: u16,
    info: &mut NodeInfo,
    story: &(impl Story + ?Sized),
) -> u16 {
    if body == HOME {
        return default::generate_ruins(rng, sys_info, which_node, info);
    }
    if body == BROADCASTERS_MOON && story.flag(StoryKey::BURVIXESE_BROADCASTERS) == 0 {
        return default::generate_artifact(rng, sys_info, which_node, info);
    }
    0
}

/// `GenerateBurvixese_pickupEnergy`: the ruins give the standard cycled
/// report; the broadcasters are taken, once.
pub fn pickup_energy(
    system: &mut SolarSys<'_>,
    body: Body,
    which_node: u16,
    lander: &mut (impl LanderTakeoff + Story + super::host::ReportSite + ?Sized),
) -> bool {
    if body == HOME {
        lander.report(
            system,
            crate::engine::world::generation::ReportDisposition::Cycle,
        );
        return false;
    }
    if body == BROADCASTERS_MOON {
        assert!(
            lander.flag(StoryKey::BURVIXESE_BROADCASTERS) == 0 && which_node == 0,
            "the broadcasters are picked up once, from node 0"
        );
        lander.report(
            system,
            crate::engine::world::generation::ReportDisposition::Once,
        );
        lander.lander_takeoff();
        lander.set_flag(StoryKey::BURVIXESE_BROADCASTERS, 1);
        lander.set_flag(StoryKey::BURV_BROADCASTERS_ON_SHIP, 1);
        return true;
    }
    false
}

use super::generator::SystemGenerator;
use super::host::SiteHost;
use crate::engine::world::generation::SolarSys;

pub struct Generator;

impl SystemGenerator for Generator {
    fn generate_planets(&self, system: &mut SolarSys<'_>, _host: &mut dyn SiteHost) -> bool {
        system.layout(|rng, table, size, color, planets| {
            generate_planets(rng, table, size, color, planets)
        });
        true
    }
    fn generate_moons(
        &self,
        system: &mut SolarSys<'_>,
        planet: u8,
        _host: &mut dyn SiteHost,
    ) -> bool {
        system.layout_moons(planet, |rng, size, color, parent, moons| {
            generate_moons(
                rng,
                size,
                color,
                parent.num_planets,
                moons,
                Body::Planet(planet),
            )
        })
    }
    fn generate_energy(
        &self,
        system: &mut SolarSys<'_>,
        body: Body,
        node: u16,
        info: &mut NodeInfo,
        host: &dyn SiteHost,
    ) -> u16 {
        system.scan(body, |rng, sys, _frame| {
            generate_energy(rng, sys, body, node, info, host)
        })
    }

    fn generate_orbital(
        &self,
        system: &mut SolarSys<'_>,
        body: Body,
        host: &mut dyn SiteHost,
    ) -> bool {
        generate_orbital(system, body, host)
    }
    fn pickup_energy(
        &self,
        system: &mut SolarSys<'_>,
        body: Body,
        node: u16,
        host: &mut dyn SiteHost,
    ) -> bool {
        pickup_energy(system, body, node, host)
    }
}

#[cfg(test)]
mod tests;
