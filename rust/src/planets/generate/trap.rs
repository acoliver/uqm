//! `gentrap.c`: the Mycon trap world, a hot telluric planet 0.
//!
//! Overridden: `generatePlanets`, `generateOrbital`. No story flags.

use crate::c_bindings::global_state::{PlanetFrame, PLANET_DESC};
use crate::math::RandomContext;
use crate::planets::generate::special::{self, Body, Orbit, EARTH_ATMOSPHERE, TELLURIC_WORLD};

/// `GenerateTrap_generatePlanets`.
pub fn generate_planets(
    rng: &mut RandomContext,
    plan_data: &[PlanetFrame],
    star_size: u8,
    star_color: u8,
    planets: &mut [PLANET_DESC],
) -> u8 {
    special::planets_then(rng, plan_data, star_size, star_color, planets, |planets| {
        planets[0].data_index = TELLURIC_WORLD;
        planets[0].NumPlanets = 1;
        special::reposition(&mut planets[0], 203);
    })
}

/// `GenerateTrap_generateOrbital`: analyse as usual, then fix the conditions.
pub fn generate_orbital(orbit: &mut Orbit<'_>, body: Body, load_planet: impl FnOnce()) -> bool {
    orbit.default_orbital(load_planet);
    if body == Body::Planet(0) {
        let info = &mut orbit.sys_info.PlanetInfo;
        info.AtmoDensity = (EARTH_ATMOSPHERE * 2) as _;
        info.SurfaceTemperature = 35;
        info.Weather = 3;
        info.Tectonics = 1;
    }
    true
}
