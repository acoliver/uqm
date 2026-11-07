//! `gencol.c`: the Ur-Quan colony. Planet 0 is a shielded water world that a
//! fixed Ur-Quan group orbits.
//!
//! Overridden: `initNpcs`, `generatePlanets`, `generateOrbital`. Story flags:
//! `COLONY_GRPOFFS0` (32-bit group reference).

use crate::engine::world::planet::PlanetDesc;
use crate::engine::StoryKey;
use crate::math::RandomContext;
use crate::planets::generate::consts::PLANET_SHIELDED;
use crate::planets::generate::default;
use crate::planets::generate::orbits::{self, OrbitRequest};
use crate::planets::generate::special::{
    self, Body, FleetGroups, Story, EARTH_ATMOSPHERE, IN_ORBIT, URQUAN_SHIP, WATER_WORLD,
};

/// `GenerateColony_initNpcs`.
pub fn init_npcs(host: &mut (impl FleetGroups + Story + ?Sized)) -> bool {
    special::ensure_fleet(host, StoryKey::COLONY_GRPOFFS0, URQUAN_SHIP);
    default::init_npcs(host);
    if host.battle_group_ref() != 0 {
        host.head_ip_group(&mut |group| {
            group.task = IN_ORBIT;
            group.sys_loc = 1;
            group.dest_loc = 1;
            group.loc.x = 0;
            group.loc.y = 0;
            group.group_counter = 0;
        });
    }
    true
}

/// `GenerateColony_generatePlanets`: unlike the other systems it fills the
/// orbits itself, without drawing moon counts. Returns the planet count.
pub fn generate_planets(
    rng: &mut RandomContext,
    star_size: u8,
    star_color: u8,
    planets: &mut [PlanetDesc],
) -> u8 {
    let request = OrbitRequest {
        star_size,
        star_color,
        num_planets: !0,
        generating_moons: false,
        types_defined: false,
    };
    let drawn = orbits::fill_orbits(rng, request, planets)
        .expect("FillOrbits draws the planet count when asked for ~0");
    special::reposition(&mut planets[0], 115);
    planets[0].data_index = WATER_WORLD | PLANET_SHIELDED;
    drawn
}

/// `GenerateColony_generateOrbital`: the colony world has fixed conditions.
pub fn generate_orbital(
    system: &mut SolarSys<'_>,
    body: Body,
    site: &mut (impl super::host::PlanetSite + ?Sized),
) -> bool {
    if body != Body::Planet(0) {
        return super::special::default_orbital(system, body, site);
    }
    super::special::with_orbit(system, body, |orbit| {
        crate::planets::generate::calc::do_planetary_analysis(
            orbit.rng,
            orbit.plan_data,
            orbit.star,
            orbit.site,
            orbit.sys_info,
            orbit.world,
        );
        let info = &mut orbit.sys_info.planet_info;
        info.atmo_density = (EARTH_ATMOSPHERE * 98 / 100) as _;
        info.weather = 0;
        info.tectonics = 0;
        info.temperature = 28;
    });
    super::host::PlanetSite::load_planet(site, system);
    true
}

use super::generator::SystemGenerator;
use super::host::SiteHost;
use crate::engine::world::generation::SolarSys;

pub struct Generator;

impl SystemGenerator for Generator {
    fn init_npcs(&self, _system: &mut SolarSys<'_>, host: &mut dyn SiteHost) -> bool {
        init_npcs(host)
    }
    fn generate_planets(&self, system: &mut SolarSys<'_>, _host: &mut dyn SiteHost) -> bool {
        system.layout(|rng, _table, size, color, planets| {
            generate_planets(rng, size, color, planets)
        });
        true
    }

    fn generate_orbital(
        &self,
        system: &mut SolarSys<'_>,
        body: Body,
        host: &mut dyn SiteHost,
    ) -> bool {
        generate_orbital(system, body, host)
    }
}
