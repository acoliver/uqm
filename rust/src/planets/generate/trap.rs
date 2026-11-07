//! `gentrap.c`: the Mycon trap world, a hot telluric planet 0.
//!
//! Overridden: `generatePlanets`, `generateOrbital`. No story flags.

use crate::engine::world::planet::{PlanetDesc, PlanetFrame};
use crate::math::RandomContext;
use crate::planets::generate::special::{self, Body, EARTH_ATMOSPHERE, TELLURIC_WORLD};

/// `GenerateTrap_generatePlanets`.
pub fn generate_planets(
    rng: &mut RandomContext,
    plan_data: &[PlanetFrame],
    star_size: u8,
    star_color: u8,
    planets: &mut [PlanetDesc],
) -> u8 {
    special::planets_then(rng, plan_data, star_size, star_color, planets, |planets| {
        planets[0].data_index = TELLURIC_WORLD;
        planets[0].num_planets = 1;
        special::reposition(&mut planets[0], 203);
    })
}

/// `GenerateTrap_generateOrbital`: analyse as usual, then fix the conditions.
pub fn generate_orbital(
    system: &mut SolarSys<'_>,
    body: Body,
    site: &mut (impl super::host::PlanetSite + ?Sized),
) -> bool {
    super::special::default_orbital(system, body, site);
    if body == Body::Planet(0) {
        let info = system.planet_info();
        info.atmo_density = (EARTH_ATMOSPHERE * 2) as _;
        info.temperature = 35;
        info.weather = 3;
        info.tectonics = 1;
    }
    true
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

    fn generate_orbital(
        &self,
        system: &mut SolarSys<'_>,
        body: Body,
        host: &mut dyn SiteHost,
    ) -> bool {
        generate_orbital(system, body, host)
    }
}
