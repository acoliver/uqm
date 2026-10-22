//! Planet and moon orbit layout, ported from `planets/orbits.c`.

use crate::battle::battle_types::{cosine, sine};
use crate::c_bindings::global_state::PLANET_DESC;
use crate::math::RandomContext;
use crate::planets::orbit_tables::{
    BLUE, GREEN, NUMBER_OF_PLANET_TYPES, ORANGE, RED, WHITE, YELLOW,
};

const FIRST_LARGE_ROCKY_WORLD: u8 = 29;
const FIRST_GAS_GIANT: u8 = (NUMBER_OF_PLANET_TYPES - 9) as u8;
const MAX_GENERATED_PLANETS: u16 = 9;

const MIN_PLANET_RADIUS: u16 = 4 << 6;
const MAX_PLANET_RADIUS: u16 = 124 << 6;
const MIN_MOON_RADIUS: u16 = 35;
/// `MOON_DELTA` and `MAX_MOONS` (`planets.h`).
pub const MOON_DELTA: u16 = 20;
pub const MAX_MOONS: u16 = 4;

/// Closest rocky and gas giant orbits for dwarf, giant and supergiant suns.
const SUN_MIN_DISTANCES: [(u16, u16); 3] = [
    (MIN_PLANET_RADIUS, 12 << 6),
    (8 << 6, 13 << 6),
    (16 << 6, 33 << 6),
];

fn distribution(star_color: u8) -> Option<&'static [u8; NUMBER_OF_PLANET_TYPES]> {
    match star_color {
        0 => Some(&BLUE),
        1 => Some(&GREEN),
        2 => Some(&ORANGE),
        3 => Some(&RED),
        4 => Some(&WHITE),
        5 => Some(&YELLOW),
        _ => None,
    }
}

/// How `FillOrbits` is being used.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OrbitRequest {
    /// `SunDesc[0].data_index`.
    pub star_size: u8,
    /// `STAR_COLOR (CurStarDescPtr->Type)`.
    pub star_color: u8,
    /// Number of worlds, or `!0` to draw it from the RNG.
    pub num_planets: u8,
    /// True when filling `MoonDesc` rather than `PlanetDesc`.
    pub generating_moons: bool,
    /// True when the caller already set every `data_index`.
    pub types_defined: bool,
}

/// `FillOrbits`: lay out `descs` and return the world count when it was drawn
/// from the RNG (the caller stores it in `SunDesc[0].NumPlanets`).
pub fn fill_orbits(
    rng: &mut RandomContext,
    request: OrbitRequest,
    descs: &mut [PLANET_DESC],
) -> Option<u8> {
    let sun = SUN_MIN_DISTANCES[usize::from(request.star_size)];
    let mut num_planets = request.num_planets;
    let mut generated_count = None;

    if num_planets == !0 {
        // The C original spins until the result is non-zero instead of
        // drawing 1 + Random % 9; that draw count is part of the universe.
        loop {
            num_planets = ((rng.next_random() as u16) % (MAX_GENERATED_PLANETS + 1)) as u8;
            if num_planets != 0 {
                break;
            }
        }
        generated_count = Some(num_planets);
    }

    let max_planet = if request.generating_moons {
        FIRST_LARGE_ROCKY_WORLD
    } else {
        NUMBER_OF_PLANET_TYPES as u8
    };
    let planet_count = num_planets;

    for index in 0..usize::from(planet_count) {
        loop {
            let mut rand_val = rng.next_random();
            if request.types_defined {
                rand_val = 0;
            } else {
                descs[index].data_index = (((rand_val as u16) >> 8) as u8) % max_planet;
            }

            let chance = distribution(request.star_color)
                .map_or(0, |table| table[usize::from(descs[index].data_index)]);
            if (rand_val as u8) < chance {
                break;
            }
        }

        let min_radius = if descs[index].data_index < FIRST_GAS_GIANT {
            sun.0
        } else {
            sun.1
        };

        loop {
            let rand_val = rng.next_random() as u16;
            if request.generating_moons {
                let radius = (MIN_MOON_RADIUS + (rand_val % MAX_MOONS) * MOON_DELTA) as i16;
                descs[index].radius = radius;
                if descs[..index].iter().any(|prior| prior.radius == radius) {
                    continue;
                }
                descs[index].NumPlanets = 0;
            } else {
                let radius = ((rand_val % (MAX_PLANET_RADIUS - min_radius)) + min_radius) as i16;
                descs[index].radius = radius;
                let crowded = descs[..index].iter().any(|prior| {
                    let delta = (prior.radius >> 6) / 5 - (radius >> 6) / 5;
                    delta.abs() <= 1
                });
                if crowded {
                    continue;
                }
            }
            break;
        }

        let angle = (rng.next_random() as u16) & 63;
        let radius = i32::from(descs[index].radius);
        let x = cosine(angle, radius) as i16;
        let y = sine(angle, radius) as i16;
        descs[index].location.x = x;
        descs[index].location.y = y;
        descs[index].rand_seed = (u32::from(y as u16) << 16) | u32::from(x as u16);
    }

    let count = usize::from(planet_count);
    for i in 0..count {
        for j in ((i + 1)..count).rev() {
            if descs[i].radius > descs[j].radius {
                descs.swap(i, j);
            }
        }
    }

    generated_count
}
