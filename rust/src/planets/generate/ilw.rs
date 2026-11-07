//! `genilw.c`: the Ilwrath home system. Planet 0 is a primordial world with
//! ruins; flying into orbit may start the Ilwrath conversation.
//!
//! Overridden: `generatePlanets`, `generateOrbital`, `generateEnergy`,
//! `pickupEnergy`. Story flags: none; the conversation sequence is a host
//! effect.

use crate::engine::world::planet::{NodeInfo, PlanetDesc, PlanetFrame, SystemInfo};
use crate::math::RandomContext;
use crate::planets::generate::default;
use crate::planets::generate::home_contact::{self, HomeContactSite};
use crate::planets::generate::special::{self, Body, PRIMORDIAL_WORLD};

/// `GenerateIlwrath_generatePlanets`.
pub fn generate_planets(
    rng: &mut RandomContext,
    plan_data: &[PlanetFrame],
    star_size: u8,
    star_color: u8,
    planets: &mut [PlanetDesc],
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
    system: &mut SolarSys<'_>,
    body: Body,
    site: &mut (impl HomeContactSite + super::host::PlanetSite + ?Sized),
) -> bool {
    if body == Body::Planet(0) && home_contact::enter_orbit(system, site, false) {
        return true;
    }
    super::special::default_orbital(system, body, site);
    set_home_climate(body, system.system_info());
    true
}

/// The weather and tectonics the home world has after the default orbital.
fn set_home_climate(body: Body, sys_info: &mut SystemInfo) {
    if body == Body::Planet(0) {
        sys_info.planet_info.weather = 2;
        sys_info.planet_info.tectonics = 3;
    }
}

/// `GenerateIlwrath_generateEnergy`.
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

/// `GenerateIlwrath_pickupEnergy`: the ruins give the standard cycled report
/// and are never taken.
pub fn pickup_energy(
    system: &mut SolarSys<'_>,
    body: Body,
    site: &mut (impl super::host::ReportSite + ?Sized),
) -> bool {
    if body == Body::Planet(0) {
        site.report(
            system,
            crate::engine::world::generation::ReportDisposition::Cycle,
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
