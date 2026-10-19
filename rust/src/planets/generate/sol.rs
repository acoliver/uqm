//! Sol layout, naming, scans and lander pickups from `gensol.c`.
//!
//! This core is not installed in the live vtable. Orbital entry and the probe
//! lifecycle still belong to the linked C generator.

use crate::c_bindings::global_state::{NODE_INFO, PLANET_DESC, PLANET_NUMBER_BASE, SYSTEM_INFO};
use crate::math::RandomContext;
use crate::planets::consts::{EARTH_RADIUS, MAP_HEIGHT, MAP_WIDTH, PLANET_SHIELDED};
use crate::planets::generate::default::{self, PlanetNames};
use crate::planets::generate::special::{self, Body, Story};
use crate::planets::orbits::{MAX_MOONS, MOON_DELTA};
use crate::planets::surface::{self, BIOLOGICAL_SCAN};

const SOL_SEED: u32 = 334_241_042;
const PLUTO: Body = Body::Planet(8);
const LUNA: Body = Body::Moon { planet: 2, moon: 1 };
// `plandata.h`: frame, orbit as percent of Earth, moon count.
const PLANETS: [(u8, i32, u8); 9] = [
    (6, 39, 0),
    (special::PRIMORDIAL_WORLD, 72, 0),
    (special::WATER_WORLD | PLANET_SHIELDED, 100, 2),
    (special::DUST_WORLD, 152, 0),
    (special::RED_GAS_GIANT, 500, 4),
    (54, 750, 1),
    (52, 1000, 0),
    (50, 1250, 1),
    (22, 1550, 0),
];

/// `GenerateSol_generatePlanets`: Sol replaces the incoming RNG seed.
/// Slots after Pluto and fields not written by C remain untouched.
pub fn generate_planets(rng: &mut RandomContext, planets: &mut [PLANET_DESC; 16]) -> u8 {
    rng.reseed(SOL_SEED);
    for (index, &(frame, percent, moons)) in PLANETS.iter().enumerate() {
        let desc = &mut planets[index];
        desc.rand_seed = rng.next_random();
        let mut angle = ((desc.rand_seed as u16) >> 8) & 63;
        if index == 1 {
            angle = (64 - angle) & 63;
        } else if index == 8 {
            angle = 64 - 8;
        }
        desc.data_index = frame;
        desc.radius = (i32::from(EARTH_RADIUS) * percent / 100) as i16;
        desc.NumPlanets = moons;
        special::place(desc, angle);
    }
    9
}

/// `GenerateSol_generateMoons`: the default draw happens even for the
/// starbase and Luna, before their frame, radius and location overrides.
pub fn generate_moons(
    rng: &mut RandomContext,
    size: u8,
    color: u8,
    planet: u8,
    count: u8,
    moons: &mut [PLANET_DESC; 4],
) {
    default::generate_moons(rng, size, color, count, &mut moons[..usize::from(count)]);
    match planet {
        2 => {
            moons[0].data_index = special::HIERARCHY_STARBASE;
            moons[0].radius = special::MIN_MOON_RADIUS;
            special::place(&mut moons[0], 32 + 16);
            moons[1].data_index = special::SELENIC_WORLD;
            moons[1].radius = special::MIN_MOON_RADIUS + ((MAX_MOONS - 1) * MOON_DELTA) as i16;
            let angle = (rng.next_random() as u16) & 63;
            special::place(&mut moons[1], angle);
        }
        4 => {
            // Io, Europa, Ganymede, Callisto (`plandata.h`).
            for (moon, frame) in moons.iter_mut().zip([7, 12, 9, 22]) {
                moon.data_index = frame;
            }
        }
        5 => moons[0].data_index = 11, // Titan: ALKALI_WORLD.
        7 => moons[0].data_index = 46, // Triton: VINYLOGOUS_WORLD.
        _ => {}
    }
}

/// `planet` is the parent planet for a moon; the frame is that planet's
/// `data_index`, never the moon's. Sol's strings precede the default names.
pub fn generate_name(host: &mut impl PlanetNames, planet: u16, frame: u8) -> bool {
    host.set_planet_name(PLANET_NUMBER_BASE as u16 + planet);
    host.set_battle_planet(frame);
    true
}

/// `GenerateSol_generateEnergy`. C ignores `whichNode` and changes only the
/// location, leaving the caller's density and type intact.
pub fn generate_energy(body: Body, node: Option<&mut NODE_INFO>, story: &impl Story) -> u16 {
    let location = if body == PLUTO && story.flag("FOUND_PLUTO_SPATHI") == 0 {
        Some((20, MAP_HEIGHT - 8))
    } else if body == LUNA && story.flag("MOONBASE_DESTROYED") == 0 {
        Some((MAP_WIDTH * 3 / 4, MAP_HEIGHT / 4))
    } else {
        None
    };
    if let Some((x, y)) = location {
        if let Some(node) = node {
            node.loc_pt.x = x as i16;
            node.loc_pt.y = y as i16;
        }
        1
    } else {
        0
    }
}

/// `GenerateSol_generateLife`: Luna's ten mobile tractors are present even
/// after the base is destroyed (`NUM_CREATURE_TYPES + 1` in `lifeform.h`).
pub fn generate_life(
    rng: &mut RandomContext,
    info: &SYSTEM_INFO,
    body: Body,
    which: u16,
    node: &mut NODE_INFO,
) -> u16 {
    if body == LUNA {
        surface::generate_random_nodes(rng, info, BIOLOGICAL_SCAN, 10, 24, which, node)
    } else {
        0
    }
}

/// The lander effects of Sol's energy pickup, ordered with the story writes.
/// Report ownership and takeoff are the same operations as the default host.
pub trait Pickup: Story {
    /// `KillLanderCrewSeq (8, ONE_SECOND / 20)`; false stops the pickup.
    fn kill_lander_crew(&mut self, count: u16, interval_ticks: u32) -> bool;
    /// `GenerateDefault_landerReport (solarSys)`.
    fn lander_report(&mut self);
    /// `SetLanderTakeoff ()`.
    fn lander_takeoff(&mut self);
}

pub fn pickup_energy(body: Body, which: u16, host: &mut impl Pickup) -> bool {
    if body == PLUTO {
        assert!(host.flag("FOUND_PLUTO_SPATHI") == 0 && which == 0);
        // `timelib.h`: ONE_SECOND is 840, not milliseconds.
        if !host.kill_lander_crew(8, 840 / 20) {
            return false;
        }
        host.set_flag("FOUND_PLUTO_SPATHI", 1);
        host.lander_report();
        host.lander_takeoff();
        // Keep Fwiffo's node visible during takeoff; the flag hides it later.
        return false;
    }
    if body == LUNA {
        assert!(host.flag("MOONBASE_DESTROYED") == 0 && which == 0);
        host.lander_report();
        host.lander_takeoff();
        host.set_flag("MOONBASE_DESTROYED", 1);
        host.set_flag("MOONBASE_ON_SHIP", 1);
        return true;
    }
    false
}

#[cfg(test)]
mod tests;
