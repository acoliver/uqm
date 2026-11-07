//! `genthrad.c`: the two Thraddash systems. Which one a hook runs for is
//! `CurStarDescPtr->Index`: Aqua Helix (a primordial world holding the helix
//! artifact, defended by Thraddash until the helix is unprotected) or the
//! Thraddash home (a water world with ruins).
//!
//! Overridden: `generatePlanets`, `generateOrbital`, `generateEnergy`,
//! `pickupEnergy`. Story flags: `HELIX_UNPROTECTED`, `THRADD_MISSION`,
//! `AQUA_HELIX`, `AQUA_HELIX_ON_SHIP`, `HELIX_VISITS`.

use crate::engine::world::planet::{NodeInfo, PlanetDesc, PlanetFrame, SystemInfo};
use crate::engine::StoryKey;
use crate::math::RandomContext;
use crate::planets::generate::default;
use crate::planets::generate::home_contact::{self, HomeContactSite, LanderTakeoff};
use crate::planets::generate::identities::{AQUA_HELIX_DEFINED, THRADD_DEFINED};
use crate::planets::generate::special::{self, Body, Story, PRIMORDIAL_WORLD, WATER_WORLD};

const WORLD: Body = Body::Planet(0);

/// `GLOBAL_FLAGS_AND_DATA` during the talk at the Thraddash home (`1 << 7`)
/// and at Aqua Helix (`1 << 6`).
const THRADD_TALK_FLAGS: u8 = 1 << 7;
const HELIX_TALK_FLAGS: u8 = 1 << 6;

/// What `generateOrbital` does besides the home contact flow.
pub trait ThraddashSite: HomeContactSite {
    /// `RepairSISBorder ()`.
    fn repair_sis_border(&mut self);
    /// `LoadStdLanderFont`, the Aqua Helix mask and report strings.
    fn load_aqua_scene(&mut self, _system: &mut crate::engine::world::generation::SolarSys<'_>);
}

/// `GenerateThraddash_generatePlanets` for the system at `star_index`.
pub fn generate_planets(
    rng: &mut RandomContext,
    plan_data: &[PlanetFrame],
    star_size: u8,
    star_color: u8,
    planets: &mut [PlanetDesc],
    star_index: u8,
) -> u8 {
    special::planets_then(rng, plan_data, star_size, star_color, planets, |planets| {
        let world = &mut planets[0];
        if star_index == AQUA_HELIX_DEFINED {
            world.data_index = PRIMORDIAL_WORLD;
            special::reposition(world, 65);
        } else {
            world.data_index = WATER_WORLD;
            world.num_planets = 0;
            special::reposition(world, 98);
        }
    })
}

/// The Thraddash defend the system: always at their home, at Aqua Helix until
/// the helix is unprotected or the Thraddash mission is under way (values 1 to
/// 3).
fn thraddash_defend(star_index: u8, story: &(impl Story + ?Sized)) -> bool {
    star_index == THRADD_DEFINED
        || (story.flag(StoryKey::HELIX_UNPROTECTED) == 0
            && (story.flag(StoryKey::THRADD_MISSION).wrapping_sub(1) as u8) >= 3)
}

/// `GenerateThraddash_generateOrbital`: at planet 0, a defended system whose
/// sphere of influence can be tracked starts the conversation, and when the
/// player survives it with the world intact the visit goes on only where the
/// Thraddash no longer defend it; then the ruins or helix scene is loaded
/// before the default orbital.
pub fn generate_orbital(
    system: &mut SolarSys<'_>,
    body: Body,
    star_index: u8,
    site: &mut (impl ThraddashSite + Story + super::host::PlanetSite + ?Sized),
) -> bool {
    if body == WORLD {
        if site.start_sphere_tracking() && thraddash_defend(star_index, site) {
            let flags = if star_index == THRADD_DEFINED {
                THRADD_TALK_FLAGS
            } else {
                HELIX_TALK_FLAGS
            };
            if !home_contact::meet(site, flags) {
                return true;
            }
            if thraddash_defend(star_index, site) {
                return true;
            }
            site.repair_sis_border();
        }

        if star_index == AQUA_HELIX_DEFINED && site.flag(StoryKey::AQUA_HELIX) == 0 {
            site.load_aqua_scene(system);
        } else if star_index == THRADD_DEFINED {
            site.load_ruins_scene(system, false);
        }
    }
    super::special::default_orbital(system, body, site);
    true
}

/// `GenerateThraddash_generateEnergy`: ruins at the Thraddash home and the
/// helix artifact at Aqua Helix until it is taken.
pub fn generate_energy(
    rng: &mut RandomContext,
    sys_info: &SystemInfo,
    star_index: u8,
    body: Body,
    which_node: u16,
    info: &mut NodeInfo,
    story: &(impl Story + ?Sized),
) -> u16 {
    if star_index == THRADD_DEFINED && body == WORLD {
        return default::generate_ruins(rng, sys_info, which_node, info);
    }
    if star_index == AQUA_HELIX_DEFINED && body == WORLD {
        if story.flag(StoryKey::AQUA_HELIX) != 0 {
            return 0;
        }
        return default::generate_artifact(rng, sys_info, which_node, info);
    }
    0
}

/// `GenerateThraddash_pickupEnergy`: the home ruins give the standard cycled
/// report; the helix is taken once, from node 0.
pub fn pickup_energy(
    system: &mut SolarSys<'_>,
    star_index: u8,
    body: Body,
    which_node: u16,
    lander: &mut (impl LanderTakeoff + Story + super::host::ReportSite + ?Sized),
) -> bool {
    if star_index == THRADD_DEFINED && body == WORLD {
        lander.report(
            system,
            crate::engine::world::generation::ReportDisposition::Cycle,
        );
        return false;
    }
    if star_index == AQUA_HELIX_DEFINED && body == WORLD {
        assert!(
            lander.flag(StoryKey::AQUA_HELIX) == 0 && which_node == 0,
            "the helix is picked up once, from node 0"
        );
        lander.report(
            system,
            crate::engine::world::generation::ReportDisposition::Once,
        );
        lander.lander_takeoff();
        lander.set_flag(StoryKey::HELIX_VISITS, 0);
        lander.set_flag(StoryKey::AQUA_HELIX, 1);
        lander.set_flag(StoryKey::AQUA_HELIX_ON_SHIP, 1);
        lander.set_flag(StoryKey::HELIX_UNPROTECTED, 1);
        return true;
    }
    false
}

use super::generator::SystemGenerator;
use super::host::SiteHost;
use crate::engine::world::generation::SolarSys;

pub struct Generator;

impl SystemGenerator for Generator {
    fn generate_planets(&self, system: &mut SolarSys<'_>, _host: &mut dyn SiteHost) -> bool {
        let star = system.star_index();
        system.layout(|rng, table, size, color, planets| {
            generate_planets(rng, table, size, color, planets, star)
        });
        true
    }
    fn generate_energy(
        &self,
        system: &mut SolarSys<'_>,
        body: Body,
        node: u16,
        info: &mut NodeInfo,
        host: &dyn SiteHost,
    ) -> u16 {
        let star = system.star_index();
        system.scan(body, |rng, sys, _frame| {
            generate_energy(rng, sys, star, body, node, info, host)
        })
    }

    fn generate_orbital(
        &self,
        system: &mut SolarSys<'_>,
        body: Body,
        host: &mut dyn SiteHost,
    ) -> bool {
        generate_orbital(system, body, system.star_index(), host)
    }
    fn pickup_energy(
        &self,
        system: &mut SolarSys<'_>,
        body: Body,
        node: u16,
        host: &mut dyn SiteHost,
    ) -> bool {
        pickup_energy(system, system.star_index(), body, node, host)
    }
}

#[cfg(test)]
mod tests;
