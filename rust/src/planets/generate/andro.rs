//! `genand.c`: the Androsynth system. Planet 1 is a telluric world of ruins
//! whose lander reports form a story: each city ruin visited is remembered in
//! the upper 16 bits of the energy retrieval mask and advances the report.
//!
//! Overridden: `generatePlanets`, `generateOrbital`, `generateEnergy`,
//! `pickupEnergy`. Story flags: `PLANETARY_CHANGE` (set on pickup).

use crate::engine::world::planet::{NodeInfo, PlanetDesc, PlanetFrame, PlanetInfo, SystemInfo};
use crate::math::RandomContext;
use crate::planet_side::report_adapter::DiscoveryReportPort;
use crate::planets::generate::default;
use crate::planets::generate::special::{
    self, Body, Orbit, Story, EARTH_ATMOSPHERE, TELLURIC_WORLD,
};
use crate::planets::generate::surface::ENERGY_SCAN;

const WORLD: Body = Body::Planet(1);

/// The ruins visited are marked in bits 16 to 31 of the energy mask; the
/// lower bits are cleared so the ruin nodes stay on the map.
const VISITED_BITS: std::ops::Range<u32> = 16..32;

/// What `generateOrbital` asks of the host.
pub trait AndrosynthSite {
    /// `LoadStdLanderFont`, the ruins mask and the Androsynth ruins report
    /// table, started at the first report not yet given: after `visits`
    /// reports. When every report was already given (`visits` reaches the
    /// table's count) the table is released and `DiscoveryString` is cleared.
    fn load_ruins_story(&mut self, visits: u32);
}

/// `GenerateAndrosynth_generatePlanets`.
pub fn generate_planets(
    rng: &mut RandomContext,
    plan_data: &[PlanetFrame],
    star_size: u8,
    star_color: u8,
    planets: &mut [PlanetDesc],
) -> u8 {
    special::planets_then(rng, plan_data, star_size, star_color, planets, |planets| {
        let world = &mut planets[1];
        world.data_index = TELLURIC_WORLD;
        special::reposition(world, 204);
    })
}

/// The number of ruins already visited (`is_node_retrieved (.., ENERGY_SCAN, i)`
/// for `i` in 16 to 31).
fn ruins_visited(info: &PlanetInfo) -> u32 {
    VISITED_BITS
        .filter(|&bit| info.scan_retrieve_mask[ENERGY_SCAN] & (1 << bit) != 0)
        .count() as u32
}

/// `GenerateAndrosynth_generateOrbital`: planet 1 starts its report story
/// where the player left off and has a fixed climate.
pub fn generate_orbital(
    orbit: &mut Orbit<'_>,
    body: Body,
    site: &mut impl AndrosynthSite,
    load_planet: impl FnOnce(),
) -> bool {
    if body == WORLD {
        site.load_ruins_story(ruins_visited(&orbit.sys_info.planet_info));
    }

    orbit.default_orbital(load_planet);

    if body == WORLD {
        let info = &mut orbit.sys_info.planet_info;
        info.atmo_density = (EARTH_ATMOSPHERE * 144 / 100) as _;
        info.temperature = 28;
        info.weather = 1;
        info.tectonics = 1;
    }
    true
}

/// `GenerateAndrosynth_generateEnergy`.
pub fn generate_energy(
    rng: &mut RandomContext,
    sys_info: &SystemInfo,
    body: Body,
    which_node: u16,
    info: &mut NodeInfo,
) -> u16 {
    if body == WORLD {
        default::generate_ruins(rng, sys_info, which_node, info)
    } else {
        0
    }
}

/// `GenerateAndrosynth_pickupEnergy`: the first visit to a ruin marks it
/// visited (and tells the solar system state so) and gives the next report
/// of the story. The node is never removed.
pub fn pickup_energy(
    body: Body,
    which_node: u16,
    info: &mut PlanetInfo,
    story: &mut impl Story,
    report: &mut impl DiscoveryReportPort,
) -> bool {
    if body == WORLD {
        let bit = u32::from(which_node) + VISITED_BITS.start;
        assert!(
            VISITED_BITS.contains(&bit),
            "ruin {which_node} has no visited bit"
        );
        if info.scan_retrieve_mask[ENERGY_SCAN] & (1 << bit) != 0 {
            return false;
        }
        info.scan_retrieve_mask[ENERGY_SCAN] |= 1 << bit;
        story.set_flag("PLANETARY_CHANGE", 1);
        default::lander_report_cycle(report);
    }
    false
}

#[cfg(test)]
mod tests;
