//! `genyeh.c`: the Yehat home system. Planet 0 is a water world with the
//! ruins of the Yehat capital; flying into orbit may start the Yehat
//! conversation.
//!
//! Overridden: `generatePlanets`, `generateOrbital`, `generateEnergy`,
//! `pickupEnergy`. Everything else is the default. Story flags: none; the
//! conversation sequence is a host effect.

use crate::c_bindings::global_state::{NODE_INFO, PLANET_DESC, SYSTEM_INFO};
use crate::math::RandomContext;
use crate::planet_side::report_adapter::DiscoveryReportPort;
use crate::planets::generate::default;
use crate::planets::generate::special::{self, Body, Orbit, WATER_WORLD};

/// What entering orbit at the Yehat capital does to the rest of the game.
pub trait YehatSite {
    /// `StartSphereTracking (YEHAT_SHIP)`.
    fn start_sphere_tracking(&mut self) -> bool;
    /// The whole contact sequence of `GenerateYehat_generateOrbital`: notify
    /// the other ships, save the groups, set the Yehat flagship up and run
    /// `InitCommunication (YEHAT_CONVERSATION)`.
    fn meet_yehat(&mut self);
    /// `LoadStdLanderFont`, the ruins mask and the ruins report strings.
    fn load_ruins_scene(&mut self);
}

/// `GenerateYehat_generatePlanets`.
pub fn generate_planets(
    rng: &mut RandomContext,
    plan_data: &[crate::c_bindings::global_state::PlanetFrame],
    star_size: u8,
    star_color: u8,
    planets: &mut [PLANET_DESC],
) -> u8 {
    special::planets_then(rng, plan_data, star_size, star_color, planets, |planets| {
        planets[0].data_index = WATER_WORLD;
        planets[0].NumPlanets = 1;
        special::reposition(&mut planets[0], 106);
    })
}

/// `GenerateYehat_generateOrbital`.
pub fn generate_orbital(
    orbit: &mut Orbit<'_>,
    body: Body,
    site: &mut impl YehatSite,
    load_planet: impl FnOnce(),
) -> bool {
    if body == Body::Planet(0) {
        if site.start_sphere_tracking() {
            site.meet_yehat();
            return true;
        }
        site.load_ruins_scene();
    }
    orbit.default_orbital(load_planet)
}

/// `GenerateYehat_generateEnergy`.
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

/// `GenerateYehat_pickupEnergy`: the ruins give the standard cycled report and
/// are never taken.
pub fn pickup_energy(body: Body, report: &mut impl DiscoveryReportPort) -> bool {
    if body == Body::Planet(0) {
        default::lander_report_cycle(report);
    }
    false
}
