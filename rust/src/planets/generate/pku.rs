//! `genpku.c`: the Pkunk home system. Planet 0 is a water world with ruins;
//! flying into orbit may start the Pkunk conversation, and the ruins hold the
//! artifact behind `CLEAR_SPINDLE`.
//!
//! Overridden: `generatePlanets`, `generateOrbital`, `generateEnergy`,
//! `pickupEnergy`. Story flags: `CLEAR_SPINDLE`, `CLEAR_SPINDLE_ON_SHIP`.

use crate::engine::world::planet::{NodeInfo, PlanetDesc, PlanetFrame, SystemInfo};
use crate::engine::StoryKey;
use crate::math::RandomContext;
use crate::planets::generate::default;
use crate::planets::generate::home_contact::{self, HomeContactSite, LanderTakeoff};
use crate::planets::generate::special::{self, Body, Story, WATER_WORLD};

/// `GeneratePkunk_generatePlanets`.
pub fn generate_planets(
    rng: &mut RandomContext,
    plan_data: &[PlanetFrame],
    star_size: u8,
    star_color: u8,
    planets: &mut [PlanetDesc],
) -> u8 {
    special::planets_then(rng, plan_data, star_size, star_color, planets, |planets| {
        planets[0].data_index = WATER_WORLD;
        planets[0].num_planets = 1;
        special::reposition(&mut planets[0], 104);
    })
}

/// `GeneratePkunk_generateOrbital`: while the Pkunk sphere of influence can be
/// tracked, entering orbit at the home world is the conversation and nothing
/// else; otherwise the ruins scene is loaded (its report skipped past the
/// first entry once the artifact is taken) before the default orbital.
pub fn generate_orbital(
    system: &mut SolarSys<'_>,
    body: Body,
    site: &mut (impl HomeContactSite + Story + super::host::PlanetSite + ?Sized),
) -> bool {
    if body == Body::Planet(0)
        && home_contact::enter_orbit(system, site, site.flag(StoryKey::CLEAR_SPINDLE) != 0)
    {
        return true;
    }
    super::special::default_orbital(system, body, site);
    true
}

/// `GeneratePkunk_generateEnergy`.
pub fn generate_energy(
    rng: &mut RandomContext,
    sys_info: &SystemInfo,
    body: Body,
    which_node: u16,
    info: &mut NodeInfo,
) -> u16 {
    if body == Body::Planet(0) {
        default::generate_ruins(rng, sys_info, which_node, info)
    } else {
        0
    }
}

/// `GeneratePkunk_pickupEnergy`: the ruins give the standard cycled report and the
/// first one visited hands over the artifact. The node is never removed.
pub fn pickup_energy(
    system: &mut SolarSys<'_>,
    body: Body,
    lander: &mut (impl LanderTakeoff + Story + super::host::ReportSite + ?Sized),
) -> bool {
    if body == Body::Planet(0) {
        home_contact::pickup_artifact(
            system,
            lander,
            StoryKey::CLEAR_SPINDLE,
            &[StoryKey::CLEAR_SPINDLE_ON_SHIP],
        );
    }
    false
}

use super::generator::SystemGenerator;
use super::host::SiteHost;
use crate::engine::world::generation::SolarSys;

pub struct Generator;

impl SystemGenerator for Generator {
    fn generate_planets(&self, system: &mut SolarSys<'_>, _host: &mut dyn SiteHost) -> bool {
        system.layout(|rng, table, size, color, planets| {
            generate_planets(rng, table, size, color, planets)
        });
        true
    }
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
        _node: u16,
        host: &mut dyn SiteHost,
    ) -> bool {
        pickup_energy(system, body, host)
    }
}

#[cfg(test)]
mod tests;
