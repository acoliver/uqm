//! `gensyr.c`: the Syreen home system. Planet 0 is the shielded home world
//! with a starbase moon.
//!
//! Overridden: `generatePlanets`, `generateMoons`, `generateOrbital`. No story
//! flags.

use crate::engine::math::battle_types::{cosine, sine, QUADRANT};
use crate::engine::world::planet::{PlanetDesc, PlanetFrame};
use crate::math::RandomContext;
use crate::planets::generate::consts::PLANET_SHIELDED;
use crate::planets::generate::default;
use crate::planets::generate::special::{
    self, Body, EARTH_ATMOSPHERE, HIERARCHY_STARBASE, MIN_MOON_RADIUS, WATER_WORLD,
};

/// `GenerateSyreen_generatePlanets`.
pub fn generate_planets(
    rng: &mut RandomContext,
    plan_data: &[PlanetFrame],
    star_size: u8,
    star_color: u8,
    planets: &mut [PlanetDesc],
) -> u8 {
    special::planets_then(rng, plan_data, star_size, star_color, planets, |planets| {
        planets[0].data_index = WATER_WORLD | PLANET_SHIELDED;
        planets[0].num_planets = 1;
    })
}

/// `GenerateSyreen_generateMoons`: `moons` is the whole moon table and
/// `planet_moons` the moon count of the planet being generated.
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
    if body == Body::Planet(0) {
        let base = &mut moons[0];
        base.data_index = HIERARCHY_STARBASE;
        base.radius = MIN_MOON_RADIUS;
        base.location.x = cosine(QUADRANT, i32::from(MIN_MOON_RADIUS)) as i16;
        base.location.y = sine(QUADRANT, i32::from(MIN_MOON_RADIUS)) as i16;
    }
    true
}

/// What the starbase moon does.
pub trait StarbaseSite {
    /// `InitCommunication (SYREEN_CONVERSATION)`.
    fn talk_to_syreen(&mut self);
}

/// `GenerateSyreen_generateOrbital`.
pub fn generate_orbital(
    system: &mut SolarSys<'_>,
    body: Body,
    site: &mut (impl StarbaseSite + super::host::PlanetSite + ?Sized),
) -> bool {
    if body == Body::Planet(0) {
        super::special::default_orbital(system, body, site);
        let info = system.planet_info();
        info.temperature = 19;
        info.tectonics = 0;
        info.weather = 0;
        info.atmo_density = (EARTH_ATMOSPHERE * 9 / 10) as _;
        return true;
    }
    if body == (Body::Moon { planet: 0, moon: 0 }) {
        site.talk_to_syreen();
        return true;
    }
    super::special::default_orbital(system, body, site)
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

    fn generate_orbital(
        &self,
        system: &mut SolarSys<'_>,
        body: Body,
        host: &mut dyn SiteHost,
    ) -> bool {
        generate_orbital(system, body, host)
    }
}
