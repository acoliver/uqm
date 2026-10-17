//! `gensyr.c`: the Syreen home system. Planet 0 is the shielded home world
//! with a starbase moon.
//!
//! Overridden: `generatePlanets`, `generateMoons`, `generateOrbital`. No story
//! flags.

use crate::battle::battle_types::{cosine, sine, QUADRANT};
use crate::c_bindings::global_state::{PlanetFrame, PLANET_DESC};
use crate::math::RandomContext;
use crate::planets::consts::PLANET_SHIELDED;
use crate::planets::generate::default;
use crate::planets::generate::special::{
    self, Body, Orbit, EARTH_ATMOSPHERE, HIERARCHY_STARBASE, MIN_MOON_RADIUS, WATER_WORLD,
};

/// `GenerateSyreen_generatePlanets`.
pub fn generate_planets(
    rng: &mut RandomContext,
    plan_data: &[PlanetFrame],
    star_size: u8,
    star_color: u8,
    planets: &mut [PLANET_DESC],
) -> u8 {
    special::planets_then(rng, plan_data, star_size, star_color, planets, |planets| {
        planets[0].data_index = WATER_WORLD | PLANET_SHIELDED;
        planets[0].NumPlanets = 1;
    })
}

/// `GenerateSyreen_generateMoons`: `moons` is the whole moon table and
/// `planet_moons` the moon count of the planet being generated.
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
    orbit: &mut Orbit<'_>,
    body: Body,
    site: &mut impl StarbaseSite,
    load_planet: impl FnOnce(),
) -> bool {
    if body == Body::Planet(0) {
        orbit.default_orbital(load_planet);
        let info = &mut orbit.sys_info.PlanetInfo;
        info.SurfaceTemperature = 19;
        info.Tectonics = 0;
        info.Weather = 0;
        info.AtmoDensity = (EARTH_ATMOSPHERE * 9 / 10) as _;
        return true;
    }
    if body == (Body::Moon { planet: 0, moon: 0 }) {
        site.talk_to_syreen();
        return true;
    }
    orbit.default_orbital(load_planet)
}
