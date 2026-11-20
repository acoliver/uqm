//! Sol generation and probe lifecycle from `gensol.c`.
//!
//! Live effects are provided by the solar-system mode.

use crate::engine::world::planet::{NodeInfo, PlanetDesc, SystemInfo};
use crate::engine::world::queues::IpGroup as IP_GROUP;
use crate::engine::StoryKey;

use super::generator::SystemGenerator;
use super::host::SiteHost;
use crate::engine::world::generation::SolarSys;

pub struct Generator;

impl SystemGenerator for Generator {
    fn init_npcs(&self, _system: &mut SolarSys<'_>, host: &mut dyn SiteHost) -> bool {
        init_npcs(host)
    }
    fn reinit_npcs(&self, _system: &mut SolarSys<'_>, host: &mut dyn SiteHost) -> bool {
        reinit_npcs(host)
    }
    fn generate_planets(&self, system: &mut SolarSys<'_>, _host: &mut dyn SiteHost) -> bool {
        system.layout(|rng, _table, _size, _color, planets| generate_planets(rng, planets));
        true
    }
    fn generate_moons(
        &self,
        system: &mut SolarSys<'_>,
        planet: u8,
        _host: &mut dyn SiteHost,
    ) -> bool {
        system.layout_moons(planet, |rng, size, color, parent, moons| {
            generate_moons(rng, size, color, planet, parent.num_planets, moons)
        });
        true
    }
    fn generate_energy(
        &self,
        _system: &mut SolarSys<'_>,
        body: Body,
        _node: u16,
        info: &mut NodeInfo,
        host: &dyn SiteHost,
    ) -> u16 {
        generate_energy(body, Some(info), host)
    }
    fn generate_life(
        &self,
        system: &mut SolarSys<'_>,
        body: Body,
        node: u16,
        info: &mut NodeInfo,
        _host: &dyn SiteHost,
    ) -> u16 {
        system.scan(body, |rng, sys, _frame| {
            generate_life(rng, sys, body, node, info)
        })
    }

    fn generate_orbital(
        &self,
        system: &mut SolarSys<'_>,
        body: Body,
        host: &mut dyn SiteHost,
    ) -> bool {
        orbital::generate_orbital(system, body, host)
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
    fn generate_name(
        &self,
        system: &mut SolarSys<'_>,
        body: Body,
        host: &mut dyn SiteHost,
    ) -> bool {
        generate_name(
            host,
            body.planet() as u16,
            system.planet_data_index(body.planet()),
        )
    }
}

#[cfg(test)]
mod lifecycle_tests;
pub mod orbital;
#[cfg(test)]
mod orbital_tests;
pub use orbital::generate_orbital;

use crate::engine::save::summary::PLANET_NUMBER_BASE;
use crate::math::RandomContext;
use crate::planets::generate::consts::{EARTH_RADIUS, MAP_HEIGHT, MAP_WIDTH, PLANET_SHIELDED};
use crate::planets::generate::default::{self, PlanetNames};
use crate::planets::generate::orbits::{MAX_MOONS, MOON_DELTA};
use crate::planets::generate::special::{self, Body, FleetGroups, Story};
use crate::planets::generate::surface::{self, BIOLOGICAL_SCAN};

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

const STARBASE: Body = Body::Moon { planet: 2, moon: 0 };
const URQUAN_DRONE_SHIP: u8 = 23;

/// Persist the single probe group even after its message has been delivered.
pub fn init_npcs(host: &mut (impl FleetGroups + Story + ?Sized)) -> bool {
    special::ensure_fleet(host, StoryKey::URQUAN_PROBE_GRPOFFS0, URQUAN_DRONE_SHIP);
    if host.flag(StoryKey::PROBE_MESSAGE_DELIVERED) == 0
        && host.init_ip_groups()
        && host.head_ip_group(&mut |group| {
            group.task = IP_GROUP::IN_ORBIT;
            group.sys_loc = 3;
            group.dest_loc = 3;
            group.loc.x = 0;
            group.loc.y = 0;
            group.group_counter = 0;
        })
    {
        return true;
    }
    default::init_npcs(host);
    true
}

/// Restore the queue before checking the collision flag, except when the
/// completed bomb empties Sol. Only the head probe may be changed.
pub fn reinit_npcs<H: FleetGroups + orbital::Queues + Story + ?Sized>(host: &mut H) -> bool {
    if host.flag(StoryKey::CHMMR_BOMB_STATE) == 3 {
        host.clear_battle_group_ref();
        default::Npcs::reinit_ip_group_q(host);
        assert_eq!(host.built_ship_count(), 0);
    } else {
        default::reinit_npcs(host);
        if host.battle_group_ref() != 0 {
            host.head_ip_group(&mut |group| {
                if group.race_id == URQUAN_DRONE_SHIP && group.task & IP_GROUP::REFORM_GROUP != 0 {
                    group.task = IP_GROUP::FLEE | IP_GROUP::IGNORE_FLAGSHIP;
                    group.dest_loc = 0;
                }
            });
        }
    }
    true
}

/// `GenerateSol_generatePlanets`: Sol replaces the incoming RNG seed.
/// Slots after Pluto and fields not written by C remain untouched.
pub fn generate_planets(rng: &mut RandomContext, planets: &mut [PlanetDesc; 16]) -> u8 {
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
        desc.num_planets = moons;
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
    moons: &mut [PlanetDesc; 4],
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
pub fn generate_name(host: &mut (impl PlanetNames + ?Sized), planet: u16, frame: u8) -> bool {
    host.set_planet_name(PLANET_NUMBER_BASE as u16 + planet);
    host.set_battle_planet(frame);
    true
}

/// `GenerateSol_generateEnergy`. C ignores `whichNode` and changes only the
/// location, leaving the caller's density and type intact.
pub fn generate_energy(
    body: Body,
    node: Option<&mut NodeInfo>,
    story: &(impl Story + ?Sized),
) -> u16 {
    let location = if body == PLUTO && story.flag(StoryKey::FOUND_PLUTO_SPATHI) == 0 {
        Some((20, MAP_HEIGHT - 8))
    } else if body == LUNA && story.flag(StoryKey::MOONBASE_DESTROYED) == 0 {
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
    info: &SystemInfo,
    body: Body,
    which: u16,
    node: &mut NodeInfo,
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
    /// `kill_lander_crew_seq (8, ONE_SECOND / 20)`; false stops the pickup.
    fn kill_lander_crew(&mut self, count: u16, interval_ticks: u32) -> bool;
    /// `GenerateDefault_landerReport (solarSys)`.
    fn lander_report(&mut self, system: &mut SolarSys<'_>);
    /// `set_lander_takeoff ()`.
    fn lander_takeoff(&mut self);
}

pub fn pickup_energy(
    system: &mut SolarSys<'_>,
    body: Body,
    which: u16,
    host: &mut (impl Pickup + ?Sized),
) -> bool {
    if body == PLUTO {
        assert!(host.flag(StoryKey::FOUND_PLUTO_SPATHI) == 0 && which == 0);
        // `timelib.h`: ONE_SECOND is 840, not milliseconds.
        if !host.kill_lander_crew(8, 840 / 20) {
            return false;
        }
        host.set_flag(StoryKey::FOUND_PLUTO_SPATHI, 1);
        host.lander_report(system);
        host.lander_takeoff();
        // Keep Fwiffo's node visible during takeoff; the flag hides it later.
        return false;
    }
    if body == LUNA {
        assert!(host.flag(StoryKey::MOONBASE_DESTROYED) == 0 && which == 0);
        host.lander_report(system);
        host.lander_takeoff();
        host.set_flag(StoryKey::MOONBASE_DESTROYED, 1);
        host.set_flag(StoryKey::MOONBASE_ON_SHIP, 1);
        return true;
    }
    false
}

#[cfg(test)]
mod tests;
