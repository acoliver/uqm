//! `genwreck.c`: the Ur-Quan wreck on planet 6 that holds the portal key.
//!
//! Overridden: `generateOrbital`, `generateEnergy`, `pickupEnergy`. Story
//! flags: `PORTAL_KEY`, `PORTAL_KEY_ON_SHIP`.

use crate::engine::world::planet::{NodeInfo, SystemInfo};
use crate::engine::StoryKey;
use crate::math::RandomContext;
use crate::planets::generate::default;
use crate::planets::generate::special::{Body, Story};

const WRECK: Body = Body::Planet(6);

/// The wreck's scene and lander.
pub trait WreckSite {
    /// `LoadStdLanderFont`, the wreck mask and the wreck report strings, with
    /// the report started at `first_report`.
    fn load_wreck_scene(
        &mut self,
        _system: &mut crate::engine::world::generation::SolarSys<'_>,
        first_report: u16,
    );
    /// `set_lander_takeoff ()`.
    fn lander_takeoff(&mut self);
}

/// `GenerateWreck_generateOrbital`: once the key is taken the first report is
/// skipped.
pub fn generate_orbital(
    system: &mut SolarSys<'_>,
    body: Body,
    site: &mut (impl WreckSite + Story + super::host::PlanetSite + ?Sized),
) -> bool {
    if body == WRECK {
        site.load_wreck_scene(system, u16::from(site.flag(StoryKey::PORTAL_KEY) != 0));
    }
    super::special::default_orbital(system, body, site)
}

/// `GenerateWreck_generateEnergy`: the wreck is one artifact node.
pub fn generate_energy(
    rng: &mut RandomContext,
    sys_info: &SystemInfo,
    body: Body,
    which_node: u16,
    info: &mut NodeInfo,
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
    system: &mut SolarSys<'_>,
    body: Body,
    which_node: u16,
    site: &mut (impl WreckSite + Story + super::host::ReportSite + ?Sized),
) -> bool {
    if body != WRECK {
        return false;
    }
    assert_eq!(which_node, 0, "the wreck has a single energy node");
    site.report(
        system,
        crate::engine::world::generation::ReportDisposition::Cycle,
    );
    if site.flag(StoryKey::PORTAL_KEY) == 0 {
        site.lander_takeoff();
        site.set_flag(StoryKey::PORTAL_KEY, 1);
        site.set_flag(StoryKey::PORTAL_KEY_ON_SHIP, 1);
    }
    false
}

use super::generator::SystemGenerator;
use super::host::SiteHost;
use crate::engine::world::generation::SolarSys;

pub struct Generator;

impl SystemGenerator for Generator {
    fn generate_energy(
        &self,
        system: &mut SolarSys<'_>,
        body: Body,
        node: u16,
        info: &mut NodeInfo,
        _host: &dyn SiteHost,
    ) -> u16 {
        system.scan(body, |rng, sys, _frame| {
            generate_energy(rng, sys, body, node, info)
        })
    }

    fn generate_orbital(
        &self,
        system: &mut SolarSys<'_>,
        body: Body,
        host: &mut dyn SiteHost,
    ) -> bool {
        generate_orbital(system, body, host)
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
}
