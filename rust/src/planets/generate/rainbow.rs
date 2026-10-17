//! `genrain.c`: the rainbow worlds. Planet 0 of each is a rainbow world, and
//! visiting it records which of the rainbow stars have been seen.
//!
//! Overridden: `generatePlanets`, `generateOrbital`. Story flags:
//! `RAINBOW_WORLD0` (8 bits), `RAINBOW_WORLD1` (2 bits).

use crate::battle::battle_types::{FULL_CIRCLE, QUADRANT};
use crate::c_bindings::global_state::{PlanetFrame, PLANET_DESC};
use crate::math::RandomContext;
use crate::planets::generate::special::{self, Body, Orbit, Story, RAINBOW_WORLD};

/// `RAINBOW_DEFINED` (`gendef.h`).
pub const RAINBOW_DEFINED: u8 = 42;

/// `GenerateRainbowWorld_generatePlanets`.
pub fn generate_planets(
    rng: &mut RandomContext,
    plan_data: &[PlanetFrame],
    star_size: u8,
    star_color: u8,
    planets: &mut [PLANET_DESC],
) -> u8 {
    special::planets_then(rng, plan_data, star_size, star_color, planets, |planets| {
        let planet = &mut planets[0];
        planet.data_index = RAINBOW_WORLD;
        planet.NumPlanets = 0;
        planet.radius = (i32::from(crate::planets::consts::EARTH_RADIUS) * 50 / 100) as i16;
        let mut angle = crate::battle::battle_types::arctan(
            i32::from(planet.location.x),
            i32::from(planet.location.y),
        );
        if angle <= QUADRANT {
            angle += QUADRANT;
        } else if angle >= FULL_CIRCLE - QUADRANT {
            angle -= QUADRANT;
        }
        special::place(planet, angle);
    })
}

/// How many rainbow stars come before the current one in the star table.
pub fn rainbow_ordinal(star_indexes: &[u8], current: usize) -> u8 {
    star_indexes[..current]
        .iter()
        .filter(|index| **index == RAINBOW_DEFINED)
        .count() as u8
}

/// `GenerateRainbowWorld_generateOrbital`: mark this rainbow world visited.
pub fn generate_orbital(
    orbit: &mut Orbit<'_>,
    body: Body,
    which_rainbow: u8,
    story: &mut impl Story,
    load_planet: impl FnOnce(),
) -> bool {
    if body == Body::Planet(0) {
        let mask = (story.flag("RAINBOW_WORLD0") | story.flag("RAINBOW_WORLD1") << 8) as u16
            | 1 << which_rainbow;
        story.set_flag("RAINBOW_WORLD0", u32::from(mask & 0xFF));
        story.set_flag("RAINBOW_WORLD1", u32::from(mask >> 8));
    }
    orbit.default_orbital(load_planet)
}
