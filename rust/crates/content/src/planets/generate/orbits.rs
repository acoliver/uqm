//! Planet and moon orbit layout, ported from `planets/orbits.c`.

use crate::engine::math::battle_types::{cosine, sine};
use crate::engine::world::planet::PlanetDesc;
use crate::math::RandomContext;
use crate::planets::generate::orbit_tables::{
    BLUE, GREEN, NUMBER_OF_PLANET_TYPES, ORANGE, RED, WHITE, YELLOW,
};

const FIRST_LARGE_ROCKY_WORLD: u8 = 29;
pub const FIRST_GAS_GIANT: u8 = (NUMBER_OF_PLANET_TYPES - 9) as u8;
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

const ORBIT_BUCKET: u16 = 5 << 6;

fn has_available_orbit(prior: &[PlanetDesc], minimum: u16) -> bool {
    (minimum / ORBIT_BUCKET..=(MAX_PLANET_RADIUS - 1) / ORBIT_BUCKET).any(|bucket| {
        prior
            .iter()
            .all(|planet| (planet.radius / ORBIT_BUCKET as i16 - bucket as i16).abs() > 1)
    })
}

fn position_orbit(planet: &mut PlanetDesc, angle: u16) {
    let radius = i32::from(planet.radius);
    let x = cosine(angle, radius) as i16;
    let y = sine(angle, radius) as i16;
    planet.location.x = x;
    planet.location.y = y;
    planet.rand_seed = (u32::from(y as u16) << 16) | u32::from(x as u16);
}

fn pack_orbits(planets: &mut [PlanetDesc], angles: &[u16], sun: (u16, u16)) {
    let minimum = |planet: &PlanetDesc| {
        if planet.data_index < FIRST_GAS_GIANT {
            sun.0
        } else {
            sun.1
        }
    };
    let mut order: Vec<_> = (0..planets.len()).collect();
    order.sort_by_key(|&i| (minimum(&planets[i]), planets[i].radius));
    let mut next_bucket = 0;
    for i in order {
        let min_radius = minimum(&planets[i]);
        let bucket = next_bucket.max(min_radius / ORBIT_BUCKET);
        let radius = min_radius.max(bucket * ORBIT_BUCKET);
        assert!(
            radius < MAX_PLANET_RADIUS,
            "too many planets for legal orbits"
        );
        planets[i].radius = radius as i16;
        if let Some(&angle) = angles.get(i) {
            position_orbit(&mut planets[i], angle);
        }
        next_bucket = bucket + 2;
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
    descs: &mut [PlanetDesc],
) -> Option<u8> {
    if request.num_planets == !0 {
        Some(fill_drawn_orbits(rng, request, descs))
    } else {
        place_orbits(rng, &request, request.num_planets, descs);
        None
    }
}

/// `FillOrbits` for a system whose world count comes from the RNG: draw the
/// count, lay out `descs` and return the count.
pub fn fill_drawn_orbits(
    rng: &mut RandomContext,
    request: OrbitRequest,
    descs: &mut [PlanetDesc],
) -> u8 {
    // The C original spins until the result is non-zero instead of
    // drawing 1 + Random % 9; that draw count is part of the universe.
    let num_planets = loop {
        let drawn = ((rng.next_random() as u16) % (MAX_GENERATED_PLANETS + 1)) as u8;
        if drawn != 0 {
            break drawn;
        }
    };
    place_orbits(rng, &request, num_planets, descs);
    num_planets
}

fn place_orbits(
    rng: &mut RandomContext,
    request: &OrbitRequest,
    num_planets: u8,
    descs: &mut [PlanetDesc],
) {
    let sun = SUN_MIN_DISTANCES[usize::from(request.star_size)];
    let count = usize::from(num_planets);
    let mut angles = vec![0; count];

    for index in 0..count {
        draw_world_type(rng, request, &mut descs[index]);

        let min_radius = if descs[index].data_index < FIRST_GAS_GIANT {
            sun.0
        } else {
            sun.1
        };

        // C retries forever if earlier placements cover every legal bucket.
        // Leave its draw sequence untouched unless no draw can succeed.
        let repacked =
            !request.generating_moons && !has_available_orbit(&descs[..index], min_radius);
        if repacked {
            pack_orbits(&mut descs[..=index], &angles[..index], sun);
        }
        draw_radius(rng, request, descs, index, min_radius, repacked);

        let angle = (rng.next_random() as u16) & 63;
        angles[index] = angle;
        position_orbit(&mut descs[index], angle);
    }

    for i in 0..count {
        for j in ((i + 1)..count).rev() {
            if descs[i].radius > descs[j].radius {
                descs.swap(i, j);
            }
        }
    }
}

/// Draw until the world type passes the star colour's distribution table.
fn draw_world_type(rng: &mut RandomContext, request: &OrbitRequest, desc: &mut PlanetDesc) {
    let max_planet = if request.generating_moons {
        FIRST_LARGE_ROCKY_WORLD
    } else {
        NUMBER_OF_PLANET_TYPES as u8
    };
    loop {
        let mut rand_val = rng.next_random();
        if request.types_defined {
            rand_val = 0;
        } else {
            desc.data_index = (((rand_val as u16) >> 8) as u8) % max_planet;
        }

        let chance =
            distribution(request.star_color).map_or(0, |table| table[usize::from(desc.data_index)]);
        if (rand_val as u8) < chance {
            break;
        }
    }
}

/// Draw `descs[index]`'s radius until it clears every earlier orbit; a repack
/// already placed it, so that case takes one draw.
fn draw_radius(
    rng: &mut RandomContext,
    request: &OrbitRequest,
    descs: &mut [PlanetDesc],
    index: usize,
    min_radius: u16,
    repacked: bool,
) {
    loop {
        let rand_val = rng.next_random() as u16;
        if repacked {
            break;
        }
        if request.generating_moons {
            let radius = (MIN_MOON_RADIUS + (rand_val % MAX_MOONS) * MOON_DELTA) as i16;
            descs[index].radius = radius;
            if descs[..index].iter().any(|prior| prior.radius == radius) {
                continue;
            }
            descs[index].num_planets = 0;
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supergiant_drawn_planets_complete_with_legal_orbits() {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            for color in 0..6 {
                for seed in 0..60 {
                    let seed = seed * 7919 + 1;
                    let mut rng = RandomContext::with_seed(seed);
                    let mut planets: [PlanetDesc; 9] = Default::default();
                    let count = fill_orbits(
                        &mut rng,
                        OrbitRequest {
                            star_size: 2,
                            star_color: color,
                            num_planets: !0,
                            generating_moons: false,
                            types_defined: false,
                        },
                        &mut planets,
                    )
                    .unwrap();
                    for (i, planet) in planets[..usize::from(count)].iter().enumerate() {
                        let minimum = if planet.data_index < FIRST_GAS_GIANT {
                            SUN_MIN_DISTANCES[2].0
                        } else {
                            SUN_MIN_DISTANCES[2].1
                        };
                        assert!((minimum..MAX_PLANET_RADIUS).contains(&(planet.radius as u16)));
                        for prior in &planets[..i] {
                            assert!((prior.radius / 320 - planet.radius / 320).abs() > 1);
                        }
                        assert_eq!(
                            planet.rand_seed,
                            (u32::from(planet.location.y as u16) << 16)
                                | u32::from(planet.location.x as u16)
                        );
                    }
                    tx.send((color, seed)).unwrap();
                }
            }
        });
        for _ in 0..360 {
            // A test deadline detects the inherited infinite loop; production
            // generation must terminate by proving that a legal radius exists.
            rx.recv_timeout(std::time::Duration::from_secs(2))
                .expect("supergiant orbit generation did not finish");
        }
    }
}
