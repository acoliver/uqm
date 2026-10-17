//! `genwreck.c`: the Ur-Quan wreck on planet 6 that holds the portal key.
//!
//! Overridden: `generateOrbital`, `generateEnergy`, `pickupEnergy`. Story
//! flags: `PORTAL_KEY`, `PORTAL_KEY_ON_SHIP`.

use crate::c_bindings::global_state::{NODE_INFO, SYSTEM_INFO};
use crate::math::RandomContext;
use crate::planet_side::report_adapter::DiscoveryReportPort;
use crate::planets::generate::default;
use crate::planets::generate::special::{Body, Orbit, Story};

const WRECK: Body = Body::Planet(6);

/// The wreck's scene and lander.
pub trait WreckSite {
    /// `LoadStdLanderFont`, the wreck mask and the wreck report strings, with
    /// the report started at `first_report`.
    fn load_wreck_scene(&mut self, first_report: u16);
    /// `SetLanderTakeoff ()`.
    fn lander_takeoff(&mut self);
}

/// `GenerateWreck_generateOrbital`: once the key is taken the first report is
/// skipped.
pub fn generate_orbital(
    orbit: &mut Orbit<'_>,
    body: Body,
    story: &impl Story,
    site: &mut impl WreckSite,
    load_planet: impl FnOnce(),
) -> bool {
    if body == WRECK {
        site.load_wreck_scene(u16::from(story.flag("PORTAL_KEY") != 0));
    }
    orbit.default_orbital(load_planet)
}

/// `GenerateWreck_generateEnergy`: the wreck is one artifact node.
pub fn generate_energy(
    rng: &mut RandomContext,
    sys_info: &SYSTEM_INFO,
    body: Body,
    which_node: u16,
    info: &mut NODE_INFO,
) -> u16 {
    if body == WRECK {
        default::generate_artifact(rng, sys_info, which_node, info)
    } else {
        0
    }
}

/// `GenerateWreck_pickupEnergy`: the wreck is never picked up; the first
/// visit hands over the portal key.
pub fn pickup_energy(
    body: Body,
    which_node: u16,
    story: &mut impl Story,
    site: &mut impl WreckSite,
    report: &mut impl DiscoveryReportPort,
) -> bool {
    if body != WRECK {
        return false;
    }
    assert_eq!(which_node, 0, "the wreck has a single energy node");
    default::lander_report_cycle(report);
    if story.flag("PORTAL_KEY") == 0 {
        site.lander_takeoff();
        story.set_flag("PORTAL_KEY", 1);
        story.set_flag("PORTAL_KEY_ON_SHIP", 1);
    }
    false
}
