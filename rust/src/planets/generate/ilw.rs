//! `genilw.c`: the Ilwrath home system. Planet 0 is a primordial world with
//! ruins; flying into orbit may start the Ilwrath conversation.
//!
//! Overridden: `generatePlanets`, `generateOrbital`, `generateEnergy`,
//! `pickupEnergy`. Story flags: none; the conversation sequence is a host
//! effect.

use crate::c_bindings::global_state::{PlanetFrame, NODE_INFO, PLANET_DESC, SYSTEM_INFO};
use crate::math::RandomContext;
use crate::planet_side::report_adapter::DiscoveryReportPort;
use crate::planets::generate::default;
use crate::planets::generate::home_contact::{self, HomeContactSite};
use crate::planets::generate::special::{self, Body, Orbit, PRIMORDIAL_WORLD};

/// `GenerateIlwrath_generatePlanets`.
pub fn generate_planets(
    rng: &mut RandomContext,
    plan_data: &[PlanetFrame],
    star_size: u8,
    star_color: u8,
    planets: &mut [PLANET_DESC],
) -> u8 {
    special::planets_then(rng, plan_data, star_size, star_color, planets, |planets| {
        planets[0].data_index = PRIMORDIAL_WORLD;
        special::reposition(&mut planets[0], 204);
    })
}

/// `GenerateIlwrath_generateOrbital`: while the Ilwrath sphere of influence
/// can still be tracked, entering orbit at the home world is the Ilwrath
/// conversation and nothing else; otherwise the ruins scene is loaded before
/// the default orbital, and the home world gets its fixed weather.
pub fn generate_orbital(
    orbit: &mut Orbit<'_>,
    body: Body,
    site: &mut impl HomeContactSite,
    load_planet: impl FnOnce(),
) -> bool {
    if body == Body::Planet(0) && home_contact::enter_orbit(site, false) {
        return true;
    }
    orbit.default_orbital(load_planet);
    set_home_climate(body, orbit.sys_info);
    true
}

/// The weather and tectonics the home world has after the default orbital.
fn set_home_climate(body: Body, sys_info: &mut SYSTEM_INFO) {
    if body == Body::Planet(0) {
        sys_info.PlanetInfo.Weather = 2;
        sys_info.PlanetInfo.Tectonics = 3;
    }
}

/// `GenerateIlwrath_generateEnergy`.
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

/// `GenerateIlwrath_pickupEnergy`: the ruins give the standard cycled report
/// and are never taken.
pub fn pickup_energy(body: Body, report: &mut impl DiscoveryReportPort) -> bool {
    if body == Body::Planet(0) {
        default::lander_report_cycle(report);
    }
    false
}

#[cfg(test)]
mod tests;
