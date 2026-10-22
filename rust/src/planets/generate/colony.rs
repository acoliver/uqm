//! `gencol.c`: the Ur-Quan colony. Planet 0 is a shielded water world that a
//! fixed Ur-Quan group orbits.
//!
//! Overridden: `initNpcs`, `generatePlanets`, `generateOrbital`. Story flags:
//! `COLONY_GRPOFFS0` (32-bit group reference).

use crate::c_bindings::global_state::PLANET_DESC;
use crate::math::RandomContext;
use crate::planets::consts::PLANET_SHIELDED;
use crate::planets::generate::default;
use crate::planets::generate::special::{
    self, Body, FleetGroups, Orbit, Story, EARTH_ATMOSPHERE, IN_ORBIT, URQUAN_SHIP, WATER_WORLD,
};
use crate::planets::orbits::{self, OrbitRequest};

/// `GenerateColony_initNpcs`.
pub fn init_npcs(story: &mut impl Story, host: &mut impl FleetGroups) -> bool {
    special::ensure_fleet(story, host, "COLONY_GRPOFFS0", URQUAN_SHIP);
    default::init_npcs(host);
    if host.battle_group_ref() != 0 {
        if let Some(group) = host.head_ip_group() {
            group.task = IN_ORBIT;
            group.sys_loc = 1;
            group.dest_loc = 1;
            group.loc.x = 0;
            group.loc.y = 0;
            group.group_counter = 0;
        }
    }
    true
}

/// `GenerateColony_generatePlanets`: unlike the other systems it fills the
/// orbits itself, without drawing moon counts. Returns the planet count.
pub fn generate_planets(
    rng: &mut RandomContext,
    star_size: u8,
    star_color: u8,
    planets: &mut [PLANET_DESC],
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
pub fn generate_orbital(orbit: &mut Orbit<'_>, body: Body, load_planet: impl FnOnce()) -> bool {
    if body != Body::Planet(0) {
        return orbit.default_orbital(load_planet);
    }
    crate::planets::calc::do_planetary_analysis(
        orbit.rng,
        orbit.plan_data,
        orbit.star,
        orbit.site,
        orbit.sys_info,
        orbit.world,
    );
    let info = &mut orbit.sys_info.PlanetInfo;
    info.AtmoDensity = (EARTH_ATMOSPHERE * 98 / 100) as _;
    info.Weather = 0;
    info.Tectonics = 0;
    info.SurfaceTemperature = 28;
    load_planet();
    true
}
