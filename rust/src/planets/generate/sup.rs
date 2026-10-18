//! `gensup.c`: the Supox home system. Planet 0 is a water world with ruins;
//! flying into orbit may start the Supox conversation, and the ruins hold the
//! artifact behind `ULTRON_CONDITION`.
//!
//! Overridden: `generatePlanets`, `generateOrbital`, `generateEnergy`,
//! `pickupEnergy`. Story flags: `ULTRON_CONDITION`.

use crate::c_bindings::global_state::{PlanetFrame, NODE_INFO, PLANET_DESC, SYSTEM_INFO};
use crate::math::RandomContext;
use crate::planet_side::report_adapter::DiscoveryReportPort;
use crate::planets::generate::default;
use crate::planets::generate::home_contact::{self, HomeContactSite, LanderTakeoff};
use crate::planets::generate::special::{self, Body, Orbit, Story, WATER_WORLD};

/// `GenerateSupox_generatePlanets`.
pub fn generate_planets(
    rng: &mut RandomContext,
    plan_data: &[PlanetFrame],
    star_size: u8,
    star_color: u8,
    planets: &mut [PLANET_DESC],
) -> u8 {
    special::planets_then(rng, plan_data, star_size, star_color, planets, |planets| {
        planets[0].data_index = WATER_WORLD;
        planets[0].NumPlanets = 2;
        special::reposition(&mut planets[0], 152);
    })
}

/// `GenerateSupox_generateOrbital`: while the Supox sphere of influence can be
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
    if body == Body::Planet(0)
        && home_contact::enter_orbit(site, story.flag("ULTRON_CONDITION") != 0)
    {
        return true;
    }
    orbit.default_orbital(load_planet);
    true
}

/// `GenerateSupox_generateEnergy`.
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

/// `GenerateSupox_pickupEnergy`: the ruins give the standard cycled report and the
/// first one visited hands over the artifact. The node is never removed.
pub fn pickup_energy(
    body: Body,
    story: &mut impl Story,
    report: &mut impl DiscoveryReportPort,
    lander: &mut impl LanderTakeoff,
) -> bool {
    if body == Body::Planet(0) {
        home_contact::pickup_artifact(story, report, lander, "ULTRON_CONDITION", &[]);
    }
    false
}

#[cfg(test)]
mod tests;
