//! The complete system-generation contract.

use super::{default, host::SiteHost, special};
use crate::engine::world::generation::{Body, SolarSys};
use crate::engine::world::planet::NodeInfo;

pub trait SystemGenerator: Sync {
    fn init_npcs(&self, _system: &mut SolarSys<'_>, host: &mut dyn SiteHost) -> bool {
        default::init_npcs(host)
    }

    fn reinit_npcs(&self, _system: &mut SolarSys<'_>, host: &mut dyn SiteHost) -> bool {
        default::reinit_npcs(host)
    }

    fn uninit_npcs(&self, _system: &mut SolarSys<'_>, host: &mut dyn SiteHost) -> bool {
        default::uninit_npcs(host)
    }

    fn generate_planets(&self, system: &mut SolarSys<'_>, _host: &mut dyn SiteHost) -> bool {
        system.layout(|rng, table, size, color, planets| {
            default::generate_planets(rng, table, size, color, planets)
        });
        true
    }

    fn generate_moons(
        &self,
        system: &mut SolarSys<'_>,
        planet: u8,
        _host: &mut dyn SiteHost,
    ) -> bool {
        system.layout_moons(planet, |rng, size, color, parent, moons| {
            default::generate_moons(
                rng,
                size,
                color,
                parent.num_planets,
                &mut moons[..usize::from(parent.num_planets)],
            );
        });
        true
    }

    fn generate_name(
        &self,
        system: &mut SolarSys<'_>,
        body: Body,
        host: &mut dyn SiteHost,
    ) -> bool {
        default::generate_name(host, body.planet() as u16, system.body_data_index(body))
    }

    fn generate_orbital(
        &self,
        system: &mut SolarSys<'_>,
        body: Body,
        host: &mut dyn SiteHost,
    ) -> bool {
        special::with_orbit(system, body, |orbit| orbit.analyse());
        super::host::PlanetSite::load_planet(host, system);
        true
    }

    fn generate_minerals(
        &self,
        system: &mut SolarSys<'_>,
        body: Body,
        node: u16,
        info: &mut NodeInfo,
        _host: &dyn SiteHost,
    ) -> u16 {
        system.scan(body, |rng, sys, frame| {
            default::generate_minerals(rng, sys, frame, node, info)
        })
    }

    fn generate_energy(
        &self,
        _system: &mut SolarSys<'_>,
        _body: Body,
        _node: u16,
        _info: &mut NodeInfo,
        _host: &dyn SiteHost,
    ) -> u16 {
        default::generate_energy()
    }

    fn generate_life(
        &self,
        system: &mut SolarSys<'_>,
        body: Body,
        node: u16,
        info: &mut NodeInfo,
        _host: &dyn SiteHost,
    ) -> u16 {
        system.scan(body, |rng, sys, frame| {
            default::generate_life(rng, sys, frame, node, info)
        })
    }

    fn pickup_minerals(
        &self,
        _system: &mut SolarSys<'_>,
        _body: Body,
        _node: u16,
        _host: &mut dyn SiteHost,
    ) -> bool {
        default::pickup_minerals()
    }

    fn pickup_energy(
        &self,
        _system: &mut SolarSys<'_>,
        _body: Body,
        _node: u16,
        _host: &mut dyn SiteHost,
    ) -> bool {
        default::pickup_energy()
    }

    fn pickup_life(
        &self,
        _system: &mut SolarSys<'_>,
        _body: Body,
        _node: u16,
        _host: &mut dyn SiteHost,
    ) -> bool {
        default::pickup_life()
    }
}
