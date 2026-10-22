//! `gendru.c`: the Druuge home system. Planet 0 is a dust world with ruins;
//! flying into orbit may start the Druuge conversation, and the ruins hold the
//! artifact behind `ROSY_SPHERE`.
//!
//! Overridden: `generatePlanets`, `generateOrbital`, `generateEnergy`,
//! `pickupEnergy`. Story flags: `ROSY_SPHERE`, `ROSY_SPHERE_ON_SHIP`.

use crate::battle::battle_types::{HALF_CIRCLE, OCTANT};
use crate::c_bindings::global_state::{PlanetFrame, NODE_INFO, PLANET_DESC, SYSTEM_INFO};
use crate::math::RandomContext;
use crate::planet_side::report_adapter::DiscoveryReportPort;
use crate::planets::consts::EARTH_RADIUS;
use crate::planets::generate::default;
use crate::planets::generate::home_contact::{self, HomeContactSite, LanderTakeoff};
use crate::planets::generate::special::{self, Body, Orbit, Story, DUST_WORLD};

/// `GenerateDruuge_generatePlanets`.
pub fn generate_planets(
    rng: &mut RandomContext,
    plan_data: &[PlanetFrame],
    star_size: u8,
    star_color: u8,
    planets: &mut [PLANET_DESC],
) -> u8 {
    let drawn = default::generate_planets(rng, plan_data, star_size, star_color, planets);
    planets.copy_within(0..usize::from(drawn), 1);
    let home = &mut planets[0];
    home.data_index = DUST_WORLD;
    home.radius = (i32::from(EARTH_RADIUS) * 50 / 100) as i16;
    home.NumPlanets = 0;
    special::place(home, HALF_CIRCLE - OCTANT);
    home.rand_seed = (u32::from(home.location.y as u16) << 16) | u32::from(home.location.x as u16);
    drawn + 1
}

/// `GenerateDruuge_generateOrbital`: while the Druuge sphere of influence can be
/// tracked, entering orbit at the home world is the conversation and nothing
/// else; otherwise the ruins scene is loaded (its report skipped past the
/// first entry once the artifact is taken) before the default orbital.
pub fn generate_orbital(
    orbit: &mut Orbit<'_>,
    body: Body,
    story: &impl Story,
    site: &mut impl HomeContactSite,
    load_planet: impl FnOnce(),
) -> bool {
    if body == Body::Planet(0) && home_contact::enter_orbit(site, story.flag("ROSY_SPHERE") != 0) {
        return true;
    }
    orbit.default_orbital(load_planet);
    true
}

/// `GenerateDruuge_generateEnergy`.
pub fn generate_energy(
    rng: &mut RandomContext,
    sys_info: &SYSTEM_INFO,
    body: Body,
    which_node: u16,
    info: &mut NODE_INFO,
) -> u16 {
    if body == Body::Planet(0) {
        default::generate_ruins(rng, sys_info, which_node, info)
    } else {
        0
    }
}

/// `GenerateDruuge_pickupEnergy`: the ruins give the standard cycled report and the
/// first one visited hands over the artifact. The node is never removed.
pub fn pickup_energy(
    body: Body,
    story: &mut impl Story,
    report: &mut impl DiscoveryReportPort,
    lander: &mut impl LanderTakeoff,
) -> bool {
    if body == Body::Planet(0) {
        home_contact::pickup_artifact(
            story,
            report,
            lander,
            "ROSY_SPHERE",
            &["ROSY_SPHERE_ON_SHIP"],
        );
    }
    false
}

#[cfg(test)]
mod tests;
