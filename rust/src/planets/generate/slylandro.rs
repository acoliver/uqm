//! `gensly.c`: the Slylandro home system. Planet 3 is the red gas giant the
//! Slylandro live in.
//!
//! Overridden: `generatePlanets`, `generateOrbital`. No story flags.

use crate::c_bindings::global_state::{PlanetFrame, PLANET_DESC};
use crate::math::RandomContext;
use crate::planets::generate::special::{self, Body, Orbit, RED_GAS_GIANT};

/// `GenerateSlylandro_generatePlanets`.
pub fn generate_planets(
    rng: &mut RandomContext,
    plan_data: &[PlanetFrame],
    star_size: u8,
    star_color: u8,
    planets: &mut [PLANET_DESC],
) -> u8 {
    special::planets_then(rng, plan_data, star_size, star_color, planets, |planets| {
        planets[3].data_index = RED_GAS_GIANT;
        planets[3].NumPlanets = 1;
    })
}

/// What the Slylandro home planet does.
pub trait HomeSite {
    /// `InitCommunication (SLYLANDRO_HOME_CONVERSATION)`.
    fn talk_to_slylandro(&mut self);
}

/// `GenerateSlylandro_generateOrbital`.
pub fn generate_orbital(
    orbit: &mut Orbit<'_>,
    body: Body,
    site: &mut impl HomeSite,
    load_planet: impl FnOnce(),
) -> bool {
    if body == Body::Planet(3) {
        site.talk_to_slylandro();
        return true;
    }
    orbit.default_orbital(load_planet)
}
